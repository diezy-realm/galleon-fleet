//! History pruning and orphaned tool message sanitization.

use clawcrew_api::model_provider::ChatMessage;

/// Outcome of a single `remove_orphaned_tool_messages` pass. The caller
/// is responsible for logging — that's where the agent/channel/session
/// context lives.
#[derive(Debug, Default, Clone)]
pub struct PrunedOrphans {
    /// Total tool / assistant messages removed across both passes.
    pub removed: usize,
    /// `tool_call_id`s that lost their pairing.
    pub orphan_tool_call_ids: Vec<String>,
}

fn assistant_is_unresolved_dispatch(
    messages: &[ChatMessage],
    prev_idx: usize,
    next_idx: usize,
) -> bool {
    match extract_assistant_tool_call_ids(&messages[prev_idx].content) {
        Some(ids) if !ids.is_empty() => {
            let between = &messages[prev_idx + 1..next_idx];
            !ids.iter().all(|id| {
                between.iter().any(|m| {
                    m.role == "tool" && extract_tool_call_id(&m.content).as_ref() == Some(id)
                })
            })
        }
        _ => false,
    }
}

impl PrunedOrphans {
    pub fn is_empty(&self) -> bool {
        self.removed == 0
    }
}

pub fn remove_orphaned_tool_messages(messages: &mut Vec<ChatMessage>) -> PrunedOrphans {
    let mut outcome = PrunedOrphans::default();
    let mut i = 0;
    while i < messages.len() {
        let assistant_tool_call_ids = if messages[i].role == "assistant" {
            extract_assistant_tool_call_ids(&messages[i].content)
        } else {
            None
        };
        if let Some(doomed_ids) = assistant_tool_call_ids
            && i > 0
            && messages[i - 1].role == "assistant"
            && assistant_is_unresolved_dispatch(messages, i - 1, i)
        {
            outcome
                .orphan_tool_call_ids
                .extend(doomed_ids.iter().cloned());
            messages.remove(i);
            outcome.removed += 1;
            while i < messages.len() && messages[i].role == "tool" {
                let dominated = match extract_tool_call_id(&messages[i].content) {
                    Some(id) => doomed_ids.iter().any(|d| d == &id),
                    None => true,
                };
                if dominated {
                    messages.remove(i);
                    outcome.removed += 1;
                } else {
                    break;
                }
            }
        } else {
            i += 1;
        }
    }

    i = 0;
    while i < messages.len() {
        if messages[i].role != "tool" {
            i += 1;
            continue;
        }

        let assistant_idx = (0..i)
            .rev()
            .take_while(|&j| messages[j].role == "assistant" || messages[j].role == "tool")
            .find(|&j| messages[j].role == "assistant");

        let is_orphan = match assistant_idx {
            None => true,
            Some(idx) => match extract_assistant_tool_call_ids(&messages[idx].content) {
                None => true,
                Some(ids) => match extract_tool_call_id(&messages[i].content) {
                    Some(tool_call_id) => !ids.iter().any(|id| id == &tool_call_id),
                    None => false,
                },
            },
        };

        if is_orphan {
            if let Some(id) = extract_tool_call_id(&messages[i].content) {
                outcome.orphan_tool_call_ids.push(id);
            }
            messages.remove(i);
            outcome.removed += 1;
        } else {
            i += 1;
        }
    }
    outcome
}

/// Try to extract a `tool_call_id` from a tool-role message's JSON content.
/// Tool messages are stored as JSON like:
/// `{"content": "...", "tool_call_id": "toolu_01Abc..."}`
fn extract_tool_call_id(content: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(content).ok()?;
    value
        .get("tool_call_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Extract the list of structured tool-call IDs an assistant message
/// is claiming to have invoked, if any. Returns `None` when the content
/// does not parse as a JSON object with a `tool_calls` array — meaning the
/// assistant has no native tool_use blocks backing any tool_results.
fn extract_assistant_tool_call_ids(content: &str) -> Option<Vec<String>> {
    let value: serde_json::Value = serde_json::from_str(content).ok()?;
    let arr = value.get("tool_calls")?.as_array()?;
    let ids: Vec<String> = arr
        .iter()
        .filter_map(|call| call.get("id").and_then(|v| v.as_str()).map(str::to_owned))
        .collect();
    if ids.is_empty() { None } else { Some(ids) }
}

pub fn strip_orphaned_tool_calls_from_assistants(messages: &mut Vec<ChatMessage>) -> usize {
    let mut seen_tool_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut stripped = 0usize;

    let mut idx = messages.len();
    while idx > 0 {
        idx -= 1;

        if messages[idx].role == "tool" {
            if let Some(id) = extract_tool_call_id(&messages[idx].content) {
                seen_tool_ids.insert(id);
            }
            continue;
        }
        if messages[idx].role != "assistant" || !messages[idx].content.contains("tool_calls") {
            continue;
        }
        let Ok(mut value) = serde_json::from_str::<serde_json::Value>(&messages[idx].content)
        else {
            continue;
        };
        let Some(calls) = value.get("tool_calls").and_then(|v| v.as_array()) else {
            continue;
        };

        let paired_calls: Vec<serde_json::Value> = calls
            .iter()
            .filter(|call| {
                call.get("id")
                    .and_then(|v| v.as_str())
                    .is_some_and(|id| seen_tool_ids.contains(id))
            })
            .cloned()
            .collect();

        if paired_calls.len() == calls.len() {
            continue; // every tool_call is paired — nothing to do
        }

        let orphan_ids: Vec<String> = calls
            .iter()
            .filter_map(|call| call.get("id").and_then(|v| v.as_str()).map(str::to_owned))
            .filter(|id| !seen_tool_ids.contains(id))
            .collect();

        if paired_calls.is_empty() {
            let salvaged_text = value
                .get("content")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|text| !text.is_empty())
                .map(ToString::to_string);
            match salvaged_text {
                Some(text) => messages[idx].content = text,
                None => {
                    messages.remove(idx);
                }
            }
        } else {
            if let serde_json::Value::Object(ref mut map) = value {
                map.insert(
                    "tool_calls".to_string(),
                    serde_json::Value::Array(paired_calls),
                );
            }
            messages[idx].content = value.to_string();
        }
        stripped += 1;

        ::galleon_log::record!(
            WARN,
            ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Note)
                .with_outcome(::galleon_log::EventOutcome::Unknown)
                .with_attrs(::serde_json::json!({ "orphan_ids": orphan_ids })),
            "Stripped unpaired tool_calls from assistant history message — likely a \
             max_tool_iterations early exit"
        );
    }
    stripped
}
