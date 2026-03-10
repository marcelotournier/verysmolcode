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
    let agents_instructions = load_agents_instructions();

    let mut prompt = format!(
        r#"You are VerySmolCode, a friendly coding assistant. Be concise, use emojis, celebrate wins.

Working directory: {cwd}
{git_context}
## Rules
- ALWAYS use tools — don't just describe what to do. Read files before editing.
- Use edit_file for changes (not write_file). Use grep_search/find_files to explore.
- run_command has a {timeout}s timeout. Use MCP tools (e.g. context7) for library docs.
- CRITICAL: Before ANY file creation or modification, ALWAYS plan first:
  1. Call todo_update(action:"add") for each step of work
  2. Call todo_update(action:"start", id:N) when starting a step
  3. Call todo_update(action:"done", id:N) when a step is complete
  The user sees the todo list in their status bar. Never skip this.
- Ask before ambiguous or destructive actions.
- After completing all changes, give a brief summary of what was done."#,
        cwd = cwd,
        git_context = git_context,
        timeout = super::tools::git::command_timeout_secs()
    );

    if !agents_instructions.is_empty() {
        prompt.push_str("\n\n## Project Instructions\n");
        prompt.push_str(&agents_instructions);
    }

    prompt
}

/// Load AGENTS.md / CLAUDE.md instructions from user-level and project-level.
/// User-level: ~/.config/verysmolcode/AGENTS.md
/// Project-level: AGENTS.md or CLAUDE.md in git root or cwd
fn load_agents_instructions() -> String {
    let mut sections = Vec::new();
    let max_size = 8000; // Cap at 8K chars to save tokens

    // 1. User-level AGENTS.md
    let user_path = Config::config_dir().join("AGENTS.md");
    if let Ok(content) = std::fs::read_to_string(&user_path) {
        if !content.trim().is_empty() {
            let truncated = safe_truncate(&content, max_size);
            sections.push(format!(
                "### User Instructions ({})\n{}",
                user_path.display(),
                truncated
            ));
        }
    }

    // 2. Project-level: find git root, then check for AGENTS.md and CLAUDE.md
    let project_root = find_project_root();
    if let Some(root) = &project_root {
        for filename in &["AGENTS.md", "CLAUDE.md"] {
            let path = root.join(filename);
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if !content.trim().is_empty() {
                        let truncated = safe_truncate(&content, max_size);
                        sections.push(format!(
                            "### {} ({})\n{}",
                            filename,
                            path.display(),
                            truncated
                        ));
                    }
                }
            }
        }
    }

    sections.join("\n\n")
}

fn find_project_root() -> Option<PathBuf> {
    // Try git root first
    if let Ok(output) = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
    {
        if output.status.success() {
            let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !root.is_empty() {
                return Some(PathBuf::from(root));
            }
        }
    }
    // Fall back to cwd
    std::env::current_dir().ok()
}

fn safe_truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}...\n(truncated, {} bytes total)", &s[..end], s.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.max_tokens_per_response, 4096);
        assert_eq!(config.max_conversation_tokens, 32000);
        assert_eq!(config.temperature, 0.7);
        assert_eq!(config.auto_compact_threshold, 24000);
        assert!(config.safety_enabled);
        assert_eq!(config.command_timeout, 60);
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let json = serde_json::to_string(&config).unwrap();
        let parsed: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(
            parsed.max_tokens_per_response,
            config.max_tokens_per_response
        );
        assert_eq!(parsed.temperature, config.temperature);
        assert_eq!(parsed.safety_enabled, config.safety_enabled);
    }

    #[test]
    fn test_config_deserialization_missing_timeout() {
        // command_timeout should default to 60 when missing
        let json = r#"{
            "max_tokens_per_response": 4096,
            "max_conversation_tokens": 32000,
            "temperature": 0.7,
            "auto_compact_threshold": 24000,
            "system_prompt": "test",
            "safety_enabled": true
        }"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.command_timeout, 60);
    }

    #[test]
    fn test_config_dir() {
        let dir = Config::config_dir();
        assert!(dir.to_string_lossy().contains("verysmolcode"));
    }

    #[test]
    fn test_config_path() {
        let path = Config::config_path();
        assert!(path.to_string_lossy().contains("config.json"));
    }

    #[test]
    fn test_safe_truncate_short() {
        let s = "hello";
        assert_eq!(safe_truncate(s, 100), "hello");
    }

    #[test]
    fn test_safe_truncate_long() {
        let s = "hello world, this is a long string";
        let result = safe_truncate(s, 10);
        assert!(result.contains("..."));
        assert!(result.contains("truncated"));
    }

    #[test]
    fn test_safe_truncate_utf8_boundary() {
        let s = "Hello \u{1F600} World"; // emoji is multi-byte
        let result = safe_truncate(s, 8);
        // Should not panic on multi-byte boundary
        assert!(result.contains("..."));
    }

    #[test]
    fn test_default_system_prompt() {
        let prompt = default_system_prompt();
        assert!(prompt.contains("VerySmolCode"));
        assert!(prompt.contains("todo_update"));
    }

    #[test]
    fn test_git_context_summary() {
        // Just verify it doesn't panic — output depends on git state
        let _context = git_context_summary();
    }
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
