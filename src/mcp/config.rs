use crate::mcp::types::McpServerConfig;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct McpConfig {
    pub servers: Vec<McpServerConfig>,
}

impl McpConfig {
    pub fn config_path() -> PathBuf {
        crate::config::Config::config_dir().join("mcp_servers.json")
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

    pub fn add_server(&mut self, config: McpServerConfig) {
        // Remove existing server with same name
        self.servers.retain(|s| s.name != config.name);
        self.servers.push(config);
    }

    pub fn remove_server(&mut self, name: &str) -> bool {
        let before = self.servers.len();
        self.servers.retain(|s| s.name != name);
        self.servers.len() < before
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn test_server(name: &str) -> McpServerConfig {
        McpServerConfig {
            name: name.to_string(),
            command: "npx".to_string(),
            args: vec!["-y".to_string(), format!("@test/{}", name)],
            env: HashMap::new(),
        }
    }

    #[test]
    fn test_default_empty() {
        let config = McpConfig::default();
        assert!(config.servers.is_empty());
    }

    #[test]
    fn test_add_server() {
        let mut config = McpConfig::default();
        config.add_server(test_server("context7"));
        assert_eq!(config.servers.len(), 1);
        assert_eq!(config.servers[0].name, "context7");
    }

    #[test]
    fn test_add_server_replaces_existing() {
        let mut config = McpConfig::default();
        config.add_server(test_server("context7"));
        config.add_server(McpServerConfig {
            name: "context7".to_string(),
            command: "node".to_string(), // different command
            args: vec![],
            env: HashMap::new(),
        });
        assert_eq!(config.servers.len(), 1);
        assert_eq!(config.servers[0].command, "node");
    }

    #[test]
    fn test_add_multiple_servers() {
        let mut config = McpConfig::default();
        config.add_server(test_server("context7"));
        config.add_server(test_server("playwright"));
        assert_eq!(config.servers.len(), 2);
    }

    #[test]
    fn test_remove_server() {
        let mut config = McpConfig::default();
        config.add_server(test_server("context7"));
        config.add_server(test_server("playwright"));
        assert!(config.remove_server("context7"));
        assert_eq!(config.servers.len(), 1);
        assert_eq!(config.servers[0].name, "playwright");
    }

    #[test]
    fn test_remove_nonexistent() {
        let mut config = McpConfig::default();
        config.add_server(test_server("context7"));
        assert!(!config.remove_server("nonexistent"));
        assert_eq!(config.servers.len(), 1);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let mut config = McpConfig::default();
        config.add_server(test_server("context7"));
        let json = serde_json::to_string(&config).unwrap();
        let parsed: McpConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.servers.len(), 1);
        assert_eq!(parsed.servers[0].name, "context7");
        assert_eq!(parsed.servers[0].command, "npx");
    }

    #[test]
    fn test_config_path() {
        let path = McpConfig::config_path();
        assert!(path.to_string_lossy().contains("mcp_servers.json"));
    }
}
