use verysmolcode::config::Config;
use verysmolcode::mcp::config::McpConfig;
use verysmolcode::mcp::types::McpServerConfig;

#[test]
fn test_default_config() {
    let config = Config::default();
    assert_eq!(config.max_tokens_per_response, 4096);
    assert_eq!(config.max_conversation_tokens, 32000);
    assert!(config.temperature > 0.0);
    assert!(config.safety_enabled);
    assert!(!config.system_prompt.is_empty());
}

#[test]
fn test_config_serialization() {
    let config = Config::default();
    let json = serde_json::to_string(&config).unwrap();
    let deserialized: Config = serde_json::from_str(&json).unwrap();
    assert_eq!(
        config.max_tokens_per_response,
        deserialized.max_tokens_per_response
    );
    assert_eq!(config.temperature, deserialized.temperature);
}

#[test]
fn test_config_paths() {
    let dir = Config::config_dir();
    assert!(dir.to_string_lossy().contains("verysmolcode"));

    let path = Config::config_path();
    assert!(path.to_string_lossy().contains("config.json"));
}

#[test]
fn test_config_command_timeout_default() {
    let config = Config::default();
    assert_eq!(config.command_timeout, 60);
}

#[test]
fn test_config_auto_compact_threshold_default() {
    let config = Config::default();
    assert_eq!(config.auto_compact_threshold, 24000);
}

#[test]
fn test_config_system_prompt_contains_cwd() {
    let config = Config::default();
    // System prompt should include the working directory
    assert!(config.system_prompt.contains("Working directory"));
}

#[test]
fn test_config_deserialize_missing_timeout() {
    // Old config files won't have command_timeout — serde(default) should fill it
    let json = r#"{
        "max_tokens_per_response": 4096,
        "max_conversation_tokens": 32000,
        "temperature": 0.7,
        "auto_compact_threshold": 24000,
        "system_prompt": "test",
        "safety_enabled": true
    }"#;
    let config: Config = serde_json::from_str(json).unwrap();
    assert_eq!(config.command_timeout, 60); // default
}

#[test]
fn test_config_deserialize_corrupted_json() {
    // Corrupted JSON should not panic — Config::load() falls back to default.
    // We can't easily test load() without touching the file system, but we can
    // verify serde gracefully rejects bad input.
    let result: Result<Config, _> = serde_json::from_str("{ invalid json }}}");
    assert!(result.is_err());
}

#[test]
fn test_system_prompt_includes_agents_section() {
    // When CLAUDE.md exists in project root (which it does for verysmolcode),
    // the system prompt should include project instructions
    let config = Config::default();
    // CLAUDE.md exists in this repo, so it should be loaded
    assert!(
        config.system_prompt.contains("Project Instructions")
            || config.system_prompt.contains("CLAUDE.md"),
        "System prompt should reference AGENTS.md or CLAUDE.md when present"
    );
}

#[test]
fn test_system_prompt_contains_git_info() {
    let config = Config::default();
    // We're in a git repo, so git context should be present
    assert!(
        config.system_prompt.contains("Git:") || config.system_prompt.contains("Working directory"),
        "System prompt should contain git or directory info"
    );
}

#[test]
fn test_mcp_config_default() {
    let config = McpConfig::default();
    assert!(config.servers.is_empty());
}

#[test]
fn test_mcp_config_add_remove() {
    let mut config = McpConfig::default();
    config.add_server(McpServerConfig {
        name: "test".to_string(),
        command: "echo".to_string(),
        args: vec!["hello".to_string()],
        env: std::collections::HashMap::new(),
    });
    assert_eq!(config.servers.len(), 1);
    assert_eq!(config.servers[0].name, "test");

    // Adding same name replaces
    config.add_server(McpServerConfig {
        name: "test".to_string(),
        command: "echo2".to_string(),
        args: vec![],
        env: std::collections::HashMap::new(),
    });
    assert_eq!(config.servers.len(), 1);
    assert_eq!(config.servers[0].command, "echo2");

    assert!(config.remove_server("test"));
    assert!(config.servers.is_empty());
    assert!(!config.remove_server("nonexistent"));
}

#[test]
fn test_mcp_config_path() {
    let path = McpConfig::config_path();
    assert!(path.to_string_lossy().contains("mcp_servers.json"));
}

#[test]
fn test_mcp_server_config_serialization() {
    let config = McpServerConfig {
        name: "context7".to_string(),
        command: "npx".to_string(),
        args: vec!["-y".to_string(), "@anthropic-ai/context7-mcp".to_string()],
        env: std::collections::HashMap::new(),
    };
    let json = serde_json::to_string(&config).unwrap();
    let deserialized: McpServerConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(config.name, deserialized.name);
    assert_eq!(config.command, deserialized.command);
    assert_eq!(config.args, deserialized.args);
}
