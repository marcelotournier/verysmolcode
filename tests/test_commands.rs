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
    assert!(msg.contains("Available Commands"));
    assert!(msg.contains("/quit"));
    assert!(msg.contains("/clear"));
    assert!(msg.contains("Ctrl+C"));
}

#[test]
fn test_help_alias() {
    let msg = get_message("/h");
    assert!(msg.contains("Available Commands"));
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
    let response = handle_command("/compact");
    assert!(matches!(
        response,
        verysmolcode::tui::app::CommandResponse::Compact
    ));
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

#[test]
fn test_config_set_max_tokens() {
    let msg = get_message("/config set max_tokens 512");
    assert!(msg.contains("Max tokens"));
}

#[test]
fn test_config_set_max_tokens_invalid() {
    let msg = get_message("/config set max_tokens abc");
    assert!(msg.contains("Error"));
}

#[test]
fn test_config_set_compact_threshold() {
    let msg = get_message("/config set compact_threshold 8000");
    assert!(msg.contains("Auto-compact threshold"));
}

#[test]
fn test_config_set_safety_on() {
    let msg = get_message("/config set safety on");
    assert!(msg.contains("Safety checks enabled"));
}

#[test]
fn test_config_set_safety_off() {
    let msg = get_message("/config set safety off");
    assert!(msg.contains("Safety checks disabled"));
    // Re-enable to not affect other tests
    let _ = get_message("/config set safety on");
}

#[test]
fn test_config_set_safety_invalid() {
    let msg = get_message("/config set safety maybe");
    assert!(msg.contains("Error"));
}

#[test]
fn test_retry_command() {
    assert!(matches!(
        handle_command("/retry"),
        verysmolcode::tui::app::CommandResponse::Retry
    ));
    assert!(matches!(
        handle_command("/r"),
        verysmolcode::tui::app::CommandResponse::Retry
    ));
}

#[test]
fn test_todo_command() {
    assert!(matches!(
        handle_command("/todo"),
        verysmolcode::tui::app::CommandResponse::ShowTodo
    ));
    assert!(matches!(
        handle_command("/t"),
        verysmolcode::tui::app::CommandResponse::ShowTodo
    ));
}

// -- Config clamping tests --

#[test]
fn test_config_set_temp_alias() {
    let msg = get_message("/config set temp 0.8");
    assert!(msg.contains("Temperature set to"));
}

#[test]
fn test_config_set_temperature_clamp_high() {
    let msg = get_message("/config set temperature 5.0");
    assert!(msg.contains("Temperature set to 2"));
}

#[test]
fn test_config_set_temperature_clamp_low() {
    let msg = get_message("/config set temperature -1.0");
    assert!(msg.contains("Temperature set to 0"));
}

#[test]
fn test_config_set_max_tokens_clamp_low() {
    let msg = get_message("/config set max_tokens 10");
    assert!(msg.contains("Max tokens/response set to 256"));
}

#[test]
fn test_config_set_max_tokens_clamp_high() {
    let msg = get_message("/config set max_tokens 999999");
    assert!(msg.contains("Max tokens/response set to 65536"));
}

#[test]
fn test_config_set_compact_threshold_clamp_low() {
    let msg = get_message("/config set compact_threshold 100");
    assert!(msg.contains("Auto-compact threshold set to 4000"));
}

#[test]
fn test_config_set_compact_threshold_clamp_high() {
    let msg = get_message("/config set compact_threshold 999999");
    assert!(msg.contains("Auto-compact threshold set to 128000"));
}

#[test]
fn test_config_set_compact_threshold_invalid() {
    let msg = get_message("/config set compact_threshold abc");
    assert!(msg.contains("Error"));
}

// -- Safety alias tests --

#[test]
fn test_config_set_safety_true() {
    let msg = get_message("/config set safety true");
    assert!(msg.contains("Safety checks enabled"));
}

#[test]
fn test_config_set_safety_false() {
    let msg = get_message("/config set safety false");
    assert!(msg.contains("Safety checks disabled"));
    let _ = get_message("/config set safety on");
}

#[test]
fn test_config_set_safety_enabled() {
    let msg = get_message("/config set safety enabled");
    assert!(msg.contains("Safety checks enabled"));
}

#[test]
fn test_config_set_safety_disabled() {
    let msg = get_message("/config set safety disabled");
    assert!(msg.contains("Safety checks disabled"));
    let _ = get_message("/config set safety on");
}

// -- MCP add/rm success tests --

#[test]
fn test_mcp_add_success() {
    let name = format!("test_add_{}", std::process::id());
    let msg = get_message(&format!("/mcp-add {} echo hello world", name));
    assert!(msg.contains("added"));
    let _ = get_message(&format!("/mcp-rm {}", name));
}

#[test]
fn test_mcp_add_no_extra_args() {
    let name = format!("test_noargs_{}", std::process::id());
    let msg = get_message(&format!("/mcp-add {} echo", name));
    assert!(msg.contains("added"));
    let _ = get_message(&format!("/mcp-rm {}", name));
}

#[test]
fn test_mcp_rm_success() {
    let name = format!("test_rm_{}", std::process::id());
    let _ = get_message(&format!("/mcp-add {} echo hi", name));
    let msg = get_message(&format!("/mcp-rm {}", name));
    assert!(msg.contains("removed"));
}

#[test]
fn test_mcp_list_with_server() {
    // Use a unique name to avoid race conditions with parallel tests
    let name = format!("test_srv_list_{}", std::process::id());
    let _ = get_message(&format!("/mcp-add {} echo hi", name));
    let msg = get_message("/mcp");
    assert!(
        msg.contains(&name),
        "Expected '{}' in MCP list, got: {}",
        name,
        msg
    );
    let _ = get_message(&format!("/mcp-rm {}", name));
}
