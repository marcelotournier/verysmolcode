//! `task` — pi-style subagent.
//!
//! Spawns an isolated, short-running AgentLoop for a focused subtask. The
//! subagent gets its own conversation history (so the parent's tokens stay
//! free) and returns only the final text answer to the caller.
//!
//! Defaults are conservative for the Gemini free tier:
//! - `read_only`: true (subagent can only read/grep/find/ls/web_fetch/vsc_help)
//! - `max_iterations`: 4 (caps tool round-trips per subagent)
//! - `model_pref`: "fast" (Flash/Lite, saves Pro budget for the parent)
//!
//! Subagents cannot spawn further subagents (`task` is not in their tool set),
//! preventing fan-out blow-ups.

use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

const MAX_RESULT_CHARS: usize = 8_000;

/// Tool names exposed to the subagent.
pub const READ_ONLY_TOOLS: &[&str] = &[
    "read",
    "read_file",
    "ls",
    "list_directory",
    "grep",
    "grep_search",
    "find",
    "find_files",
    "web_fetch",
    "vsc_help",
];

pub const WRITE_TOOLS: &[&str] = &["write", "write_file", "edit", "edit_file", "bash"];

/// Run a subagent task and return a JSON tool result with the final text.
///
/// We construct a fresh `AgentLoop`, attach the task as a single user message,
/// run the loop, and collect text events into a string. Errors are swallowed
/// into the returned result so the parent agent can continue gracefully.
pub fn task(args: &Value) -> Value {
    let task_text = match args.get("task").and_then(|v| v.as_str()) {
        Some(s) if !s.trim().is_empty() => s.to_string(),
        _ => return json!({"error": "Missing or empty 'task' argument"}),
    };
    let read_only = args
        .get("read_only")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let model_pref = args
        .get("model_pref")
        .and_then(|v| v.as_str())
        .unwrap_or("fast")
        .to_string();
    let _ = model_pref;

    let allowed: Vec<&'static str> = if read_only {
        READ_ONLY_TOOLS.to_vec()
    } else {
        let mut v = READ_ONLY_TOOLS.to_vec();
        v.extend_from_slice(WRITE_TOOLS);
        v
    };

    let mut agent = match crate::agent::loop_runner::AgentLoop::new() {
        Ok(a) => a,
        Err(e) => return json!({"error": format!("Subagent init failed: {}", e)}),
    };
    agent.set_subagent_mode(allowed);
    if read_only {
        agent.set_planning_mode(true);
    }

    // Force fast model for subagents to preserve Pro budget on the parent.
    agent.model_override = crate::agent::loop_runner::ModelOverride::Fast;

    let collected: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let collected_cb = collected.clone();
    let result = agent.process_message(&task_text, move |evt| {
        if let crate::agent::loop_runner::AgentEvent::Text(t) = evt {
            if let Ok(mut v) = collected_cb.lock() {
                v.push(t);
            }
        }
    });

    let answer = collected.lock().map(|v| v.join("\n")).unwrap_or_default();
    let truncated = if answer.len() > MAX_RESULT_CHARS {
        let mut end = MAX_RESULT_CHARS;
        while end > 0 && !answer.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}...[subagent answer truncated at {} chars]", &answer[..end], answer.len())
    } else {
        answer
    };

    let summary = if truncated.trim().is_empty() {
        "(subagent produced no text response)".to_string()
    } else {
        truncated
    };

    let mut out = json!({
        "success": result.is_ok(),
        "content": summary,
        "read_only": read_only,
    });
    if let Err(e) = result {
        out["error"] = json!(e);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_missing_arg() {
        let r = task(&json!({}));
        assert!(r.get("error").is_some());
    }

    #[test]
    fn test_task_empty_arg() {
        let r = task(&json!({"task": "  "}));
        assert!(r.get("error").is_some());
    }

    // Network-dependent — only run when GEMINI_API_KEY is set.
    #[test]
    #[ignore]
    fn test_task_simple_run() {
        if std::env::var("GEMINI_API_KEY").is_err() {
            return;
        }
        let r = task(&json!({"task": "Reply with the literal word PONG and nothing else."}));
        let content = r["content"].as_str().unwrap_or("");
        assert!(content.to_uppercase().contains("PONG"));
    }
}
