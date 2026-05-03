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
            auto_compact_threshold: 160000,
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
        r#"You are VerySmolCode (vsc), a friendly Rust-based coding harness powered by Gemini.
Be concise. Use tools; don't just describe what to do.

Working directory: {cwd}
{git_context}
## Available tools
- read: Read a file. Supports offset/limit for paging through large files.
- write: Create or overwrite a file (max 5MB). Auto-creates parent dirs.
- edit: Replace exact text. Use edits:[{{oldText,newText}}] for multiple changes
  in one call. Fuzzy match handles smart quotes / unicode dashes / trailing
  whitespace automatically. Each oldText must be unique in the file.
- ls: List a directory (alphabetical, dirs marked with '/').
- grep: Search file contents for a literal pattern. Pass glob to filter files,
  ignore_case for CI search, context for surrounding lines.
- find: Find files by glob (*, **, ?). Skips .git/node_modules/target.
- bash: Run a shell command (default {timeout}s timeout). Output tail-truncated
  to 2000 lines / 50KB. Pass timeout to override.
- task: Spawn a focused subagent for a self-contained subtask (saves your
  tokens). Default read_only:true. Use for codebase exploration, parallel
  investigation, or cleanup work that doesn't need your full context.
- vsc_help: Get vsc's own CLI help and a pointer to the README. CONSULT THIS
  before guessing about vsc features (loop, telegram, MCP, models, slash
  commands, configuration).
- git_status, git_diff, git_log, git_commit, git_add, git_branch, git_checkout,
  git_push, git_pull: First-class git tools. Prefer these over bash for git.
- web_fetch: Fetch a URL as plain text.
- todo_update: Track multi-step work (action: add/start/done/remove/list).
- send_telegram: Message the user via Telegram (only for blocking questions
  or final answers — not status updates).

## Rules
- Read files before editing. Use grep/find/ls to explore the codebase.
- Prefer edit (multi-edit when applicable) over write for changes to existing files.
- For ANY multi-step task, call todo_update first to plan; then start/done as you go.
- Ask before ambiguous or destructive actions.
- After completing changes, give a brief summary of what was done.

## Models
vsc routes between 6 Gemini models on the same API key (Gemini 3.1/3 Pro/Flash/
Flash-Lite + Gemini 2.5 Pro/Flash/Flash-Lite). Pro for complex reasoning, Flash
for follow-ups, Flash-Lite for cheap critic/repair. The router picks per-request
and falls back automatically on rate-limit. Don't worry about it.

## Slash commands you can emit
Format: `CMD:/command` on its own line. Hidden from the user.
- `CMD:/compact` — compact the conversation when context is getting large
- `CMD:/loop <prompt>` / `CMD:/loop off` — recurring iteration"#,
        cwd = cwd,
        git_context = git_context,
        timeout = super::tools::bash::command_timeout_secs()
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
        assert_eq!(config.auto_compact_threshold, 160000);
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
    fn test_config_load_returns_default_on_missing() {
        // Config::load() should never fail — returns something (defaults or saved config)
        let config = Config::load();
        // At minimum, check it has non-zero token limit and threshold
        assert!(config.max_tokens_per_response > 0);
        assert!(config.auto_compact_threshold > 0);
        // temperature can be 0.0 if user set it that way, just check it's in range
        assert!(config.temperature >= 0.0 && config.temperature <= 2.0);
    }

    #[test]
    fn test_config_save_and_reload() {
        // Save a custom config, then verify it can be loaded
        let mut config = Config::default();
        config.temperature = 0.42;
        config.max_tokens_per_response = 2048;
        config.command_timeout = 120;

        // Save should succeed (creates config dir if needed)
        let result = config.save();
        assert!(result.is_ok(), "Config save failed: {:?}", result);

        // Reload and verify
        let loaded = Config::load();
        assert_eq!(loaded.temperature, 0.42);
        assert_eq!(loaded.max_tokens_per_response, 2048);
        assert_eq!(loaded.command_timeout, 120);

        // Restore defaults
        Config::default().save().unwrap();
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
