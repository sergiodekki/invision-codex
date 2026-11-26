//! Invision: cost simulation of `mask_old_tool_outputs` (not a quality measure).
//!
//! Builds an agent run where every step reads one real file (tool output capped at 40k chars,
//! like the engine's output truncation) and measures, per step, the characters sent and the
//! characters that reuse the previous prompt's cached prefix. Usage:
//!   cargo run -p codex-invision-prompt-mask --example simulate -- <dir> [steps]

use codex_invision_prompt_mask::MaskPolicy;
use codex_invision_prompt_mask::mask_old_tool_outputs;
use codex_protocol::models::ContentItem;
use codex_protocol::models::FunctionCallOutputBody;
use codex_protocol::models::FunctionCallOutputPayload;
use codex_protocol::models::ResponseItem;
use std::path::Path;
use std::path::PathBuf;

const OUTPUT_CAP: usize = 40_000;
/// Cached input billed at 10% of regular input (common provider ratio).
const CACHED_PRICE: f64 = 0.1;

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    entries.sort();
    for path in entries {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.starts_with('.') || name == "node_modules" || name == "target" {
            continue;
        }
        if path.is_dir() {
            collect(&path, out);
        } else if ["ts", "tsx", "rs", "md", "css"]
            .iter()
            .any(|ext| name.ends_with(&format!(".{ext}")))
        {
            out.push(path);
        }
    }
}

fn message(role: &str, text: String) -> ResponseItem {
    ResponseItem::Message {
        encrypted_content: None,
        status: None,
        id: None,
        role: role.to_string(),
        content: vec![if role == "user" {
            ContentItem::InputText { text }
        } else {
            ContentItem::OutputText {
                text,
                annotations: None,
                logprobs: None,
            }
        }],
        phase: None,
        internal_chat_message_metadata_passthrough: None,
    }
}

fn render(items: &[ResponseItem]) -> String {
    items
        .iter()
        .map(|item| serde_json::to_string(item).unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\n")
}

fn common_prefix(a: &str, b: &str) -> usize {
    a.bytes().zip(b.bytes()).take_while(|(x, y)| x == y).count()
}

struct Totals {
    sent: usize,
    uncached: usize,
    largest: usize,
}

fn run(files: &[PathBuf], policy: Option<&MaskPolicy>) -> Totals {
    let mut history = vec![message(
        "user",
        "Revise o módulo e corrija o bug.".repeat(20),
    )];
    let mut previous = String::new();
    let mut totals = Totals {
        sent: 0,
        uncached: 0,
        largest: 0,
    };
    for (step, path) in files.iter().enumerate() {
        let text = std::fs::read_to_string(path).unwrap_or_default();
        let text: String = text.chars().take(OUTPUT_CAP).collect();
        history.push(ResponseItem::FunctionCall {
            encrypted_content: None,
            status: None,
            id: None,
            name: "read_file".to_string(),
            namespace: None,
            arguments: format!("{{\"path\":\"{}\"}}", path.display()),
            encrypted_function_args: None,
            call_id: format!("call_{step}"),
            internal_chat_message_metadata_passthrough: None,
        });
        history.push(ResponseItem::FunctionCallOutput {
            id: None,
            call_id: Some(format!("call_{step}")),
            name: None,
            namespace: None,
            output: FunctionCallOutputPayload {
                body: FunctionCallOutputBody::Text(text),
                success: Some(true),
            },
            internal_chat_message_metadata_passthrough: None,
        });
        let mut prompt = history.clone();
        if let Some(policy) = policy {
            mask_old_tool_outputs(&mut prompt, policy);
        }
        let rendered = render(&prompt);
        let reused = common_prefix(&previous, &rendered);
        totals.sent += rendered.len();
        totals.uncached += rendered.len() - reused;
        totals.largest = totals.largest.max(rendered.len());
        previous = rendered;
        history.push(message(
            "assistant",
            format!("Li o arquivo {step}; seguindo."),
        ));
    }
    totals
}

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(dir) = args.next().map(PathBuf::from) else {
        eprintln!("usage: simulate <dir> [steps]");
        return;
    };
    let steps: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(60);
    let mut files = Vec::new();
    collect(&dir, &mut files);
    files.retain(|p| std::fs::metadata(p).map(|m| m.len() > 0).unwrap_or(false));
    files.truncate(steps);
    let before = run(&files, None);
    let after = run(&files, Some(&MaskPolicy::default()));
    let cost = |t: &Totals| t.uncached as f64 + (t.sent - t.uncached) as f64 * CACHED_PRICE;
    println!("passos: {}", files.len());
    for (label, t) in [("sem máscara", &before), ("com máscara", &after)] {
        println!(
            "{label}: enviado {} mil caracteres; fora do cache {} mil; maior prompt {} mil; custo relativo {:.0}",
            t.sent / 1000,
            t.uncached / 1000,
            t.largest / 1000,
            cost(t) / 1000.0
        );
    }
    println!(
        "redução: enviado {:.0}%, custo com cache {:.0}%, maior prompt {:.0}%",
        100.0 * (1.0 - after.sent as f64 / before.sent as f64),
        100.0 * (1.0 - cost(&after) / cost(&before)),
        100.0 * (1.0 - after.largest as f64 / before.largest as f64)
    );
}
