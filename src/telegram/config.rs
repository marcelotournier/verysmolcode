use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TelegramConfig {
    /// Bot token from @BotFather
    pub bot_token: Option<String>,
    /// Chat ID to send/receive messages
    pub chat_id: Option<i64>,
    /// Whether Telegram integration is enabled
    #[serde(default)]
    pub enabled: bool,
}

impl TelegramConfig {
    pub fn config_path() -> PathBuf {
        crate::config::Config::config_dir().join("telegram.json")
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
        let dir = crate::config::Config::config_dir();
        std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create config dir: {}", e))?;
        let data =
            serde_json::to_string_pretty(self).map_err(|e| format!("Serialize error: {}", e))?;
        std::fs::write(Self::config_path(), data).map_err(|e| format!("Write error: {}", e))?;
        Ok(())
    }

    pub fn is_configured(&self) -> bool {
        self.enabled && self.bot_token.is_some() && self.chat_id.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = TelegramConfig::default();
        assert!(config.bot_token.is_none());
        assert!(config.chat_id.is_none());
        assert!(!config.enabled);
        assert!(!config.is_configured());
    }

    #[test]
    fn test_is_configured() {
        let mut config = TelegramConfig::default();
        assert!(!config.is_configured());

        config.enabled = true;
        assert!(!config.is_configured());

        config.bot_token = Some("token".to_string());
        assert!(!config.is_configured());

        config.chat_id = Some(12345);
        assert!(config.is_configured());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let config = TelegramConfig {
            bot_token: Some("123:ABC".to_string()),
            chat_id: Some(-100123456),
            enabled: true,
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: TelegramConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.bot_token, config.bot_token);
        assert_eq!(parsed.chat_id, config.chat_id);
        assert_eq!(parsed.enabled, config.enabled);
    }

    #[test]
    fn test_deserialization_missing_enabled() {
        let json = r#"{"bot_token": "tok", "chat_id": 123}"#;
        let config: TelegramConfig = serde_json::from_str(json).unwrap();
        assert!(!config.enabled); // default false
    }

    #[test]
    fn test_config_path() {
        let path = TelegramConfig::config_path();
        assert!(path.to_string_lossy().contains("telegram.json"));
    }
}
