use super::*;
use codex_protocol::models::FunctionCallOutputBody;
use codex_protocol::models::FunctionCallOutputContentItem;
use codex_protocol::models::FunctionCallOutputPayload;
use codex_protocol::models::ResponseItem;
use pretty_assertions::assert_eq;

fn output(call_id: &str, text: &str) -> ResponseItem {
    ResponseItem::FunctionCallOutput {
        id: None,
        call_id: Some(call_id.to_string()),
        name: None,
        namespace: None,
        output: FunctionCallOutputPayload {
            body: FunctionCallOutputBody::Text(text.to_string()),
            success: Some(true),
        },
        internal_chat_message_metadata_passthrough: None,
    }
}

fn custom_output(call_id: &str, text: &str) -> ResponseItem {
    ResponseItem::CustomToolCallOutput {
        id: None,
        call_id: call_id.to_string(),
        name: None,
        output: FunctionCallOutputPayload {
            body: FunctionCallOutputBody::Text(text.to_string()),
            success: None,
        },
        internal_chat_message_metadata_passthrough: None,
    }
}

fn big(n: usize) -> String {
    "x".repeat(n)
}

fn text_of(item: &ResponseItem) -> String {
    match item {
        ResponseItem::FunctionCallOutput { output, .. }
        | ResponseItem::CustomToolCallOutput { output, .. } => match &output.body {
            FunctionCallOutputBody::Text(text) => text.clone(),
            FunctionCallOutputBody::ContentItems(items) => format!("{} items", items.len()),
        },
        _ => String::new(),
    }
}

fn is_masked(item: &ResponseItem) -> bool {
    text_of(item).starts_with(MASK_PREFIX)
}

fn policy(keep_recent: usize, step: usize, min_chars: usize) -> MaskPolicy {
    MaskPolicy {
        keep_recent,
        step,
        min_chars,
    }
}

fn history(n: usize, size: usize) -> Vec<ResponseItem> {
    (0..n)
        .map(|i| output(&format!("call_{i}"), &big(size)))
        .collect()
}

#[test]
fn nothing_is_masked_until_a_full_step_is_old() {
    let p = policy(2, 3, 100);
    for n in 0..=4 {
        let mut items = history(n, 500);
        let stats = mask_old_tool_outputs(&mut items, &p);
        assert_eq!(stats.masked, 0, "n={n}");
        assert!(items.iter().all(|item| !is_masked(item)));
    }
}

#[test]
fn masks_the_oldest_outputs_in_whole_steps_and_keeps_the_recent_ones() {
    let p = policy(2, 3, 100);
    let mut items = history(10, 500);
    let stats = mask_old_tool_outputs(&mut items, &p);
    // 10 outputs, 2 recent kept, 8 candidates, masked in whole steps of 3 -> 6.
    assert_eq!(stats.masked, 6);
    assert_eq!(stats.chars_saved > 0, true);
    let masked: Vec<bool> = items.iter().map(is_masked).collect();
    assert_eq!(
        masked,
        vec![
            true, true, true, true, true, true, false, false, false, false
        ]
    );
}

#[test]
fn the_prompt_prefix_only_changes_when_a_new_step_is_crossed() {
    // Prompt caching needs a byte-stable prefix: between step boundaries, appending a new tool
    // output must not rewrite anything that was already sent.
    let p = policy(2, 3, 100);
    let render = |n: usize| {
        let mut items = history(n, 500);
        mask_old_tool_outputs(&mut items, &p);
        items
            .iter()
            .map(|item| serde_json::to_string(item).expect("serialize"))
            .collect::<Vec<_>>()
    };
    let at_11 = render(11);
    let at_12 = render(12);
    let at_13 = render(13);
    assert_eq!(&at_12[..11], &at_11[..]);
    assert_eq!(&at_13[..12], &at_12[..]);
    let at_14 = render(14);
    assert!(at_14[..13] != at_13[..], "a new step was crossed at 14");
}

#[test]
fn small_outputs_are_never_masked() {
    let p = policy(1, 1, 1000);
    let mut items = history(6, 999);
    let stats = mask_old_tool_outputs(&mut items, &p);
    assert_eq!(stats.masked, 0);
    assert!(items.iter().all(|item| !is_masked(item)));
}

#[test]
fn outputs_with_images_are_never_masked() {
    let p = policy(0, 1, 10);
    let mut items = vec![ResponseItem::FunctionCallOutput {
        id: None,
        call_id: Some("img".to_string()),
        name: None,
        namespace: None,
        output: FunctionCallOutputPayload {
            body: FunctionCallOutputBody::ContentItems(vec![
                FunctionCallOutputContentItem::InputText { text: big(500) },
                FunctionCallOutputContentItem::InputImage {
                    image: codex_protocol::models::ImageReference::Inline {
                        image_url: "data:image/png;base64,AAAA".to_string(),
                    },
                    detail: None,
                },
            ]),
            success: Some(true),
        },
        internal_chat_message_metadata_passthrough: None,
    }];
    let before = items.clone();
    let stats = mask_old_tool_outputs(&mut items, &p);
    assert_eq!(stats.masked, 0);
    assert_eq!(items, before);
}

#[test]
fn masking_keeps_the_call_id_and_success_and_says_how_much_was_omitted() {
    let p = policy(0, 1, 10);
    let mut items = vec![
        custom_output("patch_1", &big(1234)),
        output("read_2", &big(50)),
    ];
    let stats = mask_old_tool_outputs(&mut items, &p);
    assert_eq!(stats.masked, 2);
    match &items[0] {
        ResponseItem::CustomToolCallOutput {
            call_id, output, ..
        } => {
            assert_eq!(call_id, "patch_1");
            assert_eq!(output.success, None);
        }
        other => panic!("unexpected item {other:?}"),
    }
    match &items[1] {
        ResponseItem::FunctionCallOutput {
            call_id, output, ..
        } => {
            assert_eq!(call_id.as_deref(), Some("read_2"));
            assert_eq!(output.success, Some(true));
        }
        other => panic!("unexpected item {other:?}"),
    }
    assert!(text_of(&items[0]).contains("1234"));
    assert!(text_of(&items[0]).starts_with(MASK_PREFIX));
}

#[test]
fn non_tool_items_are_untouched_and_recency_counts_only_tool_outputs() {
    let p = policy(1, 1, 10);
    let user = ResponseItem::Message {
        encrypted_content: None,
        status: None,
        id: None,
        role: "user".to_string(),
        content: vec![codex_protocol::models::ContentItem::InputText { text: big(5000) }],
        phase: None,
        internal_chat_message_metadata_passthrough: None,
    };
    let mut items = vec![
        user.clone(),
        output("a", &big(500)),
        user.clone(),
        output("b", &big(500)),
        user.clone(),
    ];
    let stats = mask_old_tool_outputs(&mut items, &p);
    assert_eq!(stats.masked, 1);
    assert_eq!(items[0], user);
    assert!(is_masked(&items[1]));
    assert!(!is_masked(&items[3]));
    assert_eq!(items[4], user);
}

#[test]
fn masking_twice_changes_nothing() {
    let p = policy(2, 3, 100);
    let mut once = history(10, 500);
    mask_old_tool_outputs(&mut once, &p);
    let mut twice = once.clone();
    let stats = mask_old_tool_outputs(&mut twice, &p);
    assert_eq!(twice, once);
    assert_eq!(stats.masked, 6);
}

#[test]
fn a_zero_step_never_masks() {
    let p = policy(0, 0, 1);
    let mut items = history(5, 500);
    let stats = mask_old_tool_outputs(&mut items, &p);
    assert_eq!(stats.masked, 0);
}
