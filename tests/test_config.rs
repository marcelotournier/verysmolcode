use verysmolcode::config::Config;

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
