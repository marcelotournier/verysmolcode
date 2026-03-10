use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub max_tokens_per_response: u32,
    pub max_conversation_tokens: u32,
    pub temperature: f32,
    pub auto_compact_threshold: u32,
    pub system_prompt: String,
    pub safety_enabled: bool,
    #[serde(default = "default_command_timeout")]
    pub command_timeout: u64,
}

fn default_command_timeout() -> u64 {
    60
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_tokens_per_response: 4096,
            max_conversation_tokens: 32000,
            temperature: 0.7,
            auto_compact_threshold: 24000,
            system_prompt: default_system_prompt(),
            safety_enabled: true,
            command_timeout: default_command_timeout(),
        }
    }
}

impl Config {
    pub fn config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("verysmolcode")
    }

    pub fn config_path() -> PathBuf {
        Self::config_dir().join("config.json")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(data) = std::fs::read_to_string(&path) {
                if let Ok(config) = serde_json::from_str(&data) {
                    return config;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> Result<(), String> {
        let dir = Self::config_dir();
        std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create config dir: {}", e))?;
        let data =
            serde_json::to_string_pretty(self).map_err(|e| format!("Serialize error: {}", e))?;
        std::fs::write(Self::config_path(), data).map_err(|e| format!("Write error: {}", e))?;
        Ok(())
    }
}

fn default_system_prompt() -> String {
    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| ".".to_string());

    let git_context = git_context_summary();

    format!(
        r#"You are VerySmolCode, a friendly coding assistant. Be concise, use emojis, celebrate wins.

Working directory: {cwd}
{git_context}
## Rules
- ALWAYS use tools — don't just describe what to do. Read files before editing.
- Use edit_file for changes (not write_file). Use grep_search/find_files to explore.
- run_command has a {timeout}s timeout. Use MCP tools (e.g. context7) for library docs.
- For complex tasks, use todo_update to track steps (mark start/done as you go).
- Ask before ambiguous or destructive actions. Summarize when done."#,
        cwd = cwd,
        git_context = git_context,
        timeout = super::tools::git::command_timeout_secs()
    )
}

fn git_context_summary() -> String {
    let branch = std::process::Command::new("git")
        .args(["branch", "--show-current"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

    let status = std::process::Command::new("git")
        .args(["status", "--porcelain", "--short"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| {
            let s = String::from_utf8_lossy(&o.stdout);
            let lines: Vec<&str> = s.lines().take(10).collect();
            if lines.is_empty() {
                "clean".to_string()
            } else {
                let count = s.lines().count();
                let shown: String = lines.join(", ");
                if count > 10 {
                    format!("{} (+{} more)", shown, count - 10)
                } else {
                    shown
                }
            }
        });

    match (branch, status) {
        (Some(b), Some(s)) => format!("Git: {} | {}\n", b, s),
        (Some(b), None) => format!("Git: {}\n", b),
        _ => String::new(),
    }
}
