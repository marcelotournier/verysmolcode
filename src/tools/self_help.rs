//! `vsc_help` — self-knowledge tool for VSC.
//!
//! When the model is asked "what can vsc do?" / "how do I configure X?" we
//! want it to consult VSC's own help instead of guessing. This tool returns:
//!   1. The output of `vsc -h` (if the binary is on PATH or in target/),
//!   2. A pointer to the project README and GitHub repo for deeper docs.
//!
//! That mirrors how pi's prompt directs the agent to read pi's README/docs/
//! examples before improvising.

use serde_json::{json, Value};
use std::process::Command;

const REPO_URL: &str = "https://github.com/marcelotournier/verysmolcode";

fn try_run_vsc_help() -> Option<String> {
    // First try the binary that's calling us (built artifact) — fall back to
    // PATH lookup so a globally-installed `vsc` works too.
    let candidates: Vec<String> = std::env::current_exe()
        .ok()
        .map(|p| p.to_string_lossy().to_string())
        .into_iter()
        .chain(std::iter::once("vsc".to_string()))
        .collect();

    for bin in candidates {
        if let Ok(output) = Command::new(&bin).arg("-h").output() {
            if output.status.success() {
                let s = String::from_utf8_lossy(&output.stdout).to_string();
                if !s.trim().is_empty() {
                    return Some(s);
                }
            }
        }
    }
    None
}

pub fn vsc_help(args: &Value) -> Value {
    let topic = args
        .get("topic")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let cli_help = try_run_vsc_help().unwrap_or_else(|| {
        "vsc -h could not be invoked. Falling back to README pointers.".to_string()
    });

    let body = format!(
        "# VSC self-help\n\n## CLI usage (vsc -h)\n```\n{}\n```\n\n\
         For deeper questions (architecture, slash commands, config, MCP setup, \
         Telegram, /loop, subagents, model routing), consult:\n\
         - Project README in this repo (CLAUDE.md and README.md)\n\
         - Source: {}\n\n\
         Topic asked: {}\n\
         If the question is not answered above, READ the local README/CLAUDE.md \
         with the read tool BEFORE guessing.",
        cli_help.trim(),
        REPO_URL,
        if topic.is_empty() { "(none)" } else { &topic }
    );

    json!({
        "success": true,
        "content": body,
        "repo": REPO_URL
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_returns_help_string() {
        let r = vsc_help(&json!({"topic": "model routing"}));
        let body = r["content"].as_str().unwrap();
        assert!(body.contains("VSC self-help"));
        assert!(body.contains("github.com"));
    }

    #[test]
    fn test_topic_optional() {
        let r = vsc_help(&json!({}));
        assert!(r["content"].as_str().unwrap().contains("(none)"));
    }
}
