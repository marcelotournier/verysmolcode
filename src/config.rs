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
        let data = serde_json::to_string_pretty(self).map_err(|e| format!("Serialize error: {}", e))?;
        std::fs::write(Self::config_path(), data).map_err(|e| format!("Write error: {}", e))?;
        Ok(())
    }
}

fn default_system_prompt() -> String {
    r#"You are VerySmolCode, a lightweight coding assistant. You help users with software engineering tasks by reading, writing, and editing code files, searching codebases, and running git operations.

Rules:
- Be concise and direct in responses
- Use tools to accomplish tasks - don't just describe what to do
- Read files before editing them
- Never delete important system files or run destructive commands
- When editing code, preserve existing style and formatting
- If unsure, ask the user before making changes
- After completing a task, briefly summarize what was done"#.to_string()
}
