use crate::tui::app::CommandResponse;

const COMMANDS: &[(&str, &str)] = &[
    ("/help", "Show available commands"),
    ("/clear", "Clear conversation and screen"),
    ("/quit", "Exit VerySmolCode"),
    ("/plan", "Toggle planning mode (read-only, uses Pro model)"),
    ("/status", "Show rate limits and token usage"),
    ("/config", "Show current configuration"),
    ("/compact", "Manually compact conversation to save tokens"),
    ("/model", "Show available models and current selection"),
    ("/mcp", "List configured MCP servers"),
    (
        "/mcp-add",
        "Add an MCP server: /mcp-add <name> <command> [args...]",
    ),
    ("/mcp-rm", "Remove an MCP server: /mcp-rm <name>"),
    ("/version", "Show version information"),
];

pub fn handle_command(input: &str) -> CommandResponse {
    let parts: Vec<&str> = input.splitn(2, ' ').collect();
    let cmd = parts[0].to_lowercase();
    let args = parts.get(1).unwrap_or(&"");

    match cmd.as_str() {
        "/help" | "/h" => {
            let mut help = String::from("Available commands:\n\n");
            for (cmd, desc) in COMMANDS {
                help.push_str(&format!("  {:12} {}\n", cmd, desc));
            }
            help.push_str("\nKeybindings:\n");
            help.push_str("  Ctrl+C     Cancel/Quit\n");
            help.push_str("  Ctrl+L     Clear screen\n");
            help.push_str("  Up/Down    Input history\n");
            help.push_str("  PgUp/PgDn  Scroll output\n");
            help.push_str("  Tab        Auto-complete commands\n");
            CommandResponse::Message(help)
        }
        "/quit" | "/q" | "/exit" => CommandResponse::Quit,
        "/clear" => CommandResponse::Clear,
        "/plan" => CommandResponse::TogglePlan,
        "/status" => CommandResponse::SendToAgent(
            "Show me the current rate limit status and token usage.".to_string(),
        ),
        "/config" => {
            let config = crate::config::Config::load();
            let msg = format!(
                "Current configuration:\n\
                 Max tokens/response: {}\n\
                 Max conversation tokens: {}\n\
                 Temperature: {}\n\
                 Auto-compact threshold: {}\n\
                 Safety checks: {}",
                config.max_tokens_per_response,
                config.max_conversation_tokens,
                config.temperature,
                config.auto_compact_threshold,
                if config.safety_enabled {
                    "enabled"
                } else {
                    "disabled"
                },
            );
            CommandResponse::Message(msg)
        }
        "/compact" => CommandResponse::Message("Conversation compacted.".to_string()),
        "/model" => {
            let msg = "Available models (Gemini Free Tier):\n\n\
                       Gemini 3.1 Pro        - 5 RPM,  25 RPD  (complex tasks)\n\
                       Gemini 3 Flash        - 10 RPM, 250 RPD (general tasks)\n\
                       Gemini 3.1 Flash-Lite - 15 RPM, 1000 RPD (simple tasks)\n\
                       Gemini 2.5 Pro        - 5 RPM,  25 RPD  (fallback complex)\n\
                       Gemini 2.5 Flash      - 10 RPM, 250 RPD (fallback general)\n\
                       Gemini 2.5 Flash-Lite - 15 RPM, 1000 RPD (fallback simple)\n\n\
                       Gemini 3 models preferred. Falls back to 2.5 when rate-limited.\n\
                       Each model has independent limits, doubling effective daily quota.";
            CommandResponse::Message(msg.to_string())
        }
        "/mcp" => {
            let config = crate::mcp::config::McpConfig::load();
            if config.servers.is_empty() {
                CommandResponse::Message(
                    "No MCP servers configured.\n\
                     Use /mcp-add <name> <command> [args...] to add one.\n\n\
                     Examples:\n\
                     /mcp-add context7 npx -y @anthropic-ai/context7-mcp\n\
                     /mcp-add playwright npx -y @anthropic-ai/playwright-mcp"
                        .to_string(),
                )
            } else {
                let mut msg = String::from("Configured MCP servers:\n\n");
                for server in &config.servers {
                    msg.push_str(&format!(
                        "  {} - {} {}\n",
                        server.name,
                        server.command,
                        server.args.join(" ")
                    ));
                }
                CommandResponse::Message(msg)
            }
        }
        "/mcp-add" => {
            let parts: Vec<&str> = args.splitn(3, ' ').collect();
            if parts.len() < 2 {
                CommandResponse::Message(
                    "Usage: /mcp-add <name> <command> [args...]\n\
                     Example: /mcp-add context7 npx -y @anthropic-ai/context7-mcp"
                        .to_string(),
                )
            } else {
                let name = parts[0].to_string();
                let command = parts[1].to_string();
                let args: Vec<String> = if parts.len() > 2 {
                    parts[2].split_whitespace().map(|s| s.to_string()).collect()
                } else {
                    Vec::new()
                };

                let server_config = crate::mcp::types::McpServerConfig {
                    name: name.clone(),
                    command,
                    args,
                    env: std::collections::HashMap::new(),
                };

                let mut mcp_config = crate::mcp::config::McpConfig::load();
                mcp_config.add_server(server_config);
                match mcp_config.save() {
                    Ok(()) => CommandResponse::Message(format!(
                        "MCP server '{}' added. It will be available on next restart.",
                        name
                    )),
                    Err(e) => CommandResponse::Message(format!("Failed to save config: {}", e)),
                }
            }
        }
        "/mcp-rm" => {
            let name = args.trim();
            if name.is_empty() {
                CommandResponse::Message("Usage: /mcp-rm <name>".to_string())
            } else {
                let mut mcp_config = crate::mcp::config::McpConfig::load();
                if mcp_config.remove_server(name) {
                    match mcp_config.save() {
                        Ok(()) => {
                            CommandResponse::Message(format!("MCP server '{}' removed.", name))
                        }
                        Err(e) => CommandResponse::Message(format!("Failed to save config: {}", e)),
                    }
                } else {
                    CommandResponse::Message(format!("MCP server '{}' not found.", name))
                }
            }
        }
        "/version" => CommandResponse::Message(format!(
            "VerySmolCode v{}\nA lightweight coding assistant for constrained devices",
            env!("CARGO_PKG_VERSION")
        )),
        _ => CommandResponse::Message(format!(
            "Unknown command: {}. Type /help for available commands.",
            cmd
        )),
    }
}

pub fn autocomplete(input: &str) -> Vec<String> {
    COMMANDS
        .iter()
        .filter(|(cmd, _)| cmd.starts_with(input))
        .map(|(cmd, _)| cmd.to_string())
        .collect()
}
