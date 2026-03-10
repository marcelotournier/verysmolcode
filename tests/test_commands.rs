use verysmolcode::tui::commands::{autocomplete, handle_command};

// Helper to check if a command returns a message (not quit/clear/etc.)
fn is_message(input: &str) -> bool {
    matches!(
        handle_command(input),
        verysmolcode::tui::app::CommandResponse::Message(_)
    )
}

fn get_message(input: &str) -> String {
    match handle_command(input) {
        verysmolcode::tui::app::CommandResponse::Message(m) => m,
        _ => panic!("Expected Message response"),
    }
}

#[test]
fn test_help_command() {
    let msg = get_message("/help");
    assert!(msg.contains("Available commands"));
    assert!(msg.contains("/quit"));
    assert!(msg.contains("/clear"));
    assert!(msg.contains("Ctrl+C"));
}

#[test]
fn test_help_alias() {
    let msg = get_message("/h");
    assert!(msg.contains("Available commands"));
}

#[test]
fn test_quit_command() {
    assert!(matches!(
        handle_command("/quit"),
        verysmolcode::tui::app::CommandResponse::Quit
    ));
}

#[test]
fn test_quit_aliases() {
    assert!(matches!(
        handle_command("/q"),
        verysmolcode::tui::app::CommandResponse::Quit
    ));
    assert!(matches!(
        handle_command("/exit"),
        verysmolcode::tui::app::CommandResponse::Quit
    ));
}

#[test]
fn test_clear_command() {
    assert!(matches!(
        handle_command("/clear"),
        verysmolcode::tui::app::CommandResponse::Clear
    ));
}

#[test]
fn test_plan_command() {
    assert!(matches!(
        handle_command("/plan"),
        verysmolcode::tui::app::CommandResponse::TogglePlan
    ));
}

#[test]
fn test_config_command() {
    let msg = get_message("/config");
    assert!(msg.contains("Max tokens"));
    assert!(msg.contains("Temperature"));
    assert!(msg.contains("Safety checks"));
}

#[test]
fn test_model_command() {
    let msg = get_message("/model");
    assert!(msg.contains("3.1 Pro"));
    assert!(msg.contains("3 Flash"));
    assert!(msg.contains("2.5 Pro"));
    assert!(msg.contains("2.5 Flash"));
}

#[test]
fn test_version_command() {
    let msg = get_message("/version");
    assert!(msg.contains("VerySmolCode"));
}

#[test]
fn test_compact_command() {
    let msg = get_message("/compact");
    assert!(msg.contains("compacted"));
}

#[test]
fn test_unknown_command() {
    let msg = get_message("/nonexistent");
    assert!(msg.contains("Unknown command"));
}

#[test]
fn test_mcp_empty() {
    let msg = get_message("/mcp");
    // Either shows "No MCP servers" or lists them
    assert!(msg.contains("MCP") || msg.contains("mcp"));
}

#[test]
fn test_mcp_add_no_args() {
    let msg = get_message("/mcp-add");
    assert!(msg.contains("Usage"));
}

#[test]
fn test_mcp_add_one_arg() {
    let msg = get_message("/mcp-add test");
    assert!(msg.contains("Usage"));
}

#[test]
fn test_mcp_rm_no_args() {
    let msg = get_message("/mcp-rm");
    assert!(msg.contains("Usage"));
}

#[test]
fn test_mcp_rm_nonexistent() {
    let msg = get_message("/mcp-rm nonexistent_server_xyz");
    assert!(msg.contains("not found"));
}

#[test]
fn test_status_shows_tokens() {
    assert!(matches!(
        handle_command("/status"),
        verysmolcode::tui::app::CommandResponse::ShowTokens
    ));
}

#[test]
fn test_tokens_shows_tokens() {
    assert!(matches!(
        handle_command("/tokens"),
        verysmolcode::tui::app::CommandResponse::ShowTokens
    ));
}

#[test]
fn test_autocomplete_empty() {
    let results = autocomplete("/");
    assert!(results.len() > 5); // Should have many commands
}

#[test]
fn test_autocomplete_partial() {
    let results = autocomplete("/he");
    assert!(results.contains(&"/help".to_string()));
}

#[test]
fn test_autocomplete_exact() {
    let results = autocomplete("/help");
    assert!(results.contains(&"/help".to_string()));
}

#[test]
fn test_autocomplete_no_match() {
    let results = autocomplete("/zzz");
    assert!(results.is_empty());
}

#[test]
fn test_autocomplete_mcp() {
    let results = autocomplete("/mcp");
    assert!(results.contains(&"/mcp".to_string()));
    assert!(results.contains(&"/mcp-add".to_string()));
    assert!(results.contains(&"/mcp-rm".to_string()));
}

#[test]
fn test_fast_command() {
    assert!(matches!(
        handle_command("/fast"),
        verysmolcode::tui::app::CommandResponse::SetModelOverride(_)
    ));
    assert!(matches!(
        handle_command("/f"),
        verysmolcode::tui::app::CommandResponse::SetModelOverride(_)
    ));
}

#[test]
fn test_smart_command() {
    assert!(matches!(
        handle_command("/smart"),
        verysmolcode::tui::app::CommandResponse::SetModelOverride(_)
    ));
    assert!(matches!(
        handle_command("/s"),
        verysmolcode::tui::app::CommandResponse::SetModelOverride(_)
    ));
}

#[test]
fn test_save_command() {
    assert!(matches!(
        handle_command("/save"),
        verysmolcode::tui::app::CommandResponse::Save(None)
    ));
    match handle_command("/save output.md") {
        verysmolcode::tui::app::CommandResponse::Save(Some(name)) => {
            assert_eq!(name, "output.md");
        }
        _ => panic!("Expected Save with filename"),
    }
}

#[test]
fn test_undo_command() {
    assert!(matches!(
        handle_command("/undo"),
        verysmolcode::tui::app::CommandResponse::Undo
    ));
    assert!(matches!(
        handle_command("/u"),
        verysmolcode::tui::app::CommandResponse::Undo
    ));
}

#[test]
fn test_config_set_temperature() {
    let msg = get_message("/config set temperature 0.5");
    assert!(msg.contains("Temperature set to"));
}

#[test]
fn test_config_set_invalid() {
    let msg = get_message("/config set temperature abc");
    assert!(msg.contains("Error"));
}

#[test]
fn test_config_set_unknown_key() {
    let msg = get_message("/config set badkey 123");
    assert!(msg.contains("Unknown key"));
}

#[test]
fn test_config_set_no_value() {
    let msg = get_message("/config set temperature");
    assert!(msg.contains("Usage"));
}

#[test]
fn test_case_insensitive_commands() {
    // Commands should be case-insensitive
    assert!(matches!(
        handle_command("/QUIT"),
        verysmolcode::tui::app::CommandResponse::Quit
    ));
    assert!(is_message("/HELP"));
}
