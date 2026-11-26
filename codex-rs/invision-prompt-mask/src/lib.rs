//! Invision: mask old tool outputs in the prompt sent to the model.
//!
//! "The Complexity Trap" (NeurIPS 2025) measured that replacing old tool observations with a short
//! placeholder halves the cost of agent runs with the same solve rate as LLM summarization. This
//! crate applies that to the prompt projection only (the stored history is untouched), in whole
//! steps so the prompt prefix stays byte-stable between steps and prompt caching keeps working.

use codex_protocol::models::FunctionCallOutputBody;
use codex_protocol::models::FunctionCallOutputContentItem;
use codex_protocol::models::FunctionCallOutputPayload;
use codex_protocol::models::ResponseItem;

/// Text every masked output starts with, so masking is idempotent and easy to spot.
pub const MASK_PREFIX: &str = "[Older tool output omitted";

/// How aggressively old tool outputs are masked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaskPolicy {
    /// The most recent tool outputs that are always sent in full.
    pub keep_recent: usize,
    /// Older outputs are masked in multiples of this many, so the prefix changes only once per step.
    pub step: usize,
    /// Outputs at or under this many characters are never masked.
    pub min_chars: usize,
}

impl Default for MaskPolicy {
    fn default() -> Self {
        Self {
            keep_recent: 8,
            step: 8,
            min_chars: 1500,
        }
    }
}

/// What a masking pass did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MaskStats {
    /// Outputs that are masked in the returned prompt (including ones masked before).
    pub masked: usize,
    /// Characters removed by this pass.
    pub chars_saved: usize,
}

/// Replaces old, large, text-only tool outputs with a placeholder. See the crate docs.
///
/// - Only `function_call_output` and `custom_tool_call_output` are considered; the call id and the
///   success flag stay, so every call keeps its paired output.
/// - The newest `keep_recent` tool outputs are always sent in full.
/// - Among the older ones, only text outputs over `min_chars` (or already masked) are candidates,
///   and they are masked oldest first in whole multiples of `step`: appending outputs between two
///   step boundaries never rewrites anything already sent.
/// - Outputs with images, audio or encrypted content are never touched.
pub fn mask_old_tool_outputs(items: &mut [ResponseItem], policy: &MaskPolicy) -> MaskStats {
    let mut stats = MaskStats::default();
    if policy.step == 0 {
        return stats;
    }
    let tool_outputs: Vec<usize> = items
        .iter()
        .enumerate()
        .filter(|(_, item)| payload(item).is_some())
        .map(|(index, _)| index)
        .collect();
    let old = tool_outputs.len().saturating_sub(policy.keep_recent);
    let candidates: Vec<usize> = tool_outputs[..old]
        .iter()
        .copied()
        .filter(|&index| {
            payload(&items[index])
                .and_then(text_only)
                .is_some_and(|text| {
                    text.starts_with(MASK_PREFIX) || char_len(&text) > policy.min_chars
                })
        })
        .collect();
    let to_mask = candidates.len() / policy.step * policy.step;
    for &index in &candidates[..to_mask] {
        let Some(payload) = payload_mut(&mut items[index]) else {
            continue;
        };
        let Some(text) = text_only(payload) else {
            continue;
        };
        stats.masked += 1;
        if text.starts_with(MASK_PREFIX) {
            continue;
        }
        let chars = char_len(&text);
        let placeholder = format!(
            "{MASK_PREFIX} to save context ({chars} characters). Run the tool again if you need it.]"
        );
        stats.chars_saved += chars.saturating_sub(char_len(&placeholder));
        payload.body = FunctionCallOutputBody::Text(placeholder);
    }
    stats
}

fn payload(item: &ResponseItem) -> Option<&FunctionCallOutputPayload> {
    match item {
        ResponseItem::FunctionCallOutput { output, .. }
        | ResponseItem::CustomToolCallOutput { output, .. } => Some(output),
        _ => None,
    }
}

fn payload_mut(item: &mut ResponseItem) -> Option<&mut FunctionCallOutputPayload> {
    match item {
        ResponseItem::FunctionCallOutput { output, .. }
        | ResponseItem::CustomToolCallOutput { output, .. } => Some(output),
        _ => None,
    }
}

/// The whole text of an output, or `None` when it carries anything that is not text.
fn text_only(payload: &FunctionCallOutputPayload) -> Option<String> {
    match &payload.body {
        FunctionCallOutputBody::Text(text) => Some(text.clone()),
        FunctionCallOutputBody::ContentItems(items) => {
            let mut text = String::new();
            for item in items {
                match item {
                    FunctionCallOutputContentItem::InputText { text: part } => text.push_str(part),
                    _ => return None,
                }
            }
            Some(text)
        }
    }
}

fn char_len(text: &str) -> usize {
    text.chars().count()
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
