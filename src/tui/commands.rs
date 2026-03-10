use crate::tui::app::CommandResponse;

pub const COMMANDS: &[(&str, &str)] = &[
    ("/help", "Show available commands"),
    ("/clear", "Clear conversation and screen"),
    ("/quit", "Exit VerySmolCode"),
    (
        "/fast",
        "Use Flash models for next message (saves Pro budget)",
    ),
    ("/smart", "Use Pro models for next message (best quality)"),
    ("/plan", "Toggle planning mode (read-only, uses Pro model)"),
    ("/tokens", "Show detailed token usage and rate limits"),
    ("/status", "Show rate limits and token usage"),
    ("/config", "Show/set config: /config set <key> <value>"),
    ("/undo", "Undo the last batch of file changes"),
    ("/save", "Save conversation to file: /save [filename]"),
    ("/compact", "Manually compact conversation to save tokens"),
    ("/model", "Show available models and current selection"),
    ("/mcp", "List configured MCP servers"),
    (
        "/mcp-add",
        "Add an MCP server: /mcp-add <name> <command> [args...]",
    ),
    ("/mcp-rm", "Remove an MCP server: /mcp-rm <name>"),
    ("/version", "Show version information"),
    ("/retry", "Retry the last message"),
    ("/todo", "Show current task list"),
    (
        "/resume",
        "Resume last session or list recent: /resume [id]",
    ),
    ("/diff", "Show git diff (unstaged changes)"),
];

pub fn handle_command(input: &str) -> CommandResponse {
    let parts: Vec<&str> = input.splitn(2, ' ').collect();
    let cmd = parts[0].to_lowercase();
    let args = parts.get(1).unwrap_or(&"");

    match cmd.as_str() {
        "/help" | "/h" => {
            let mut help = String::from("\u{1F4D6} Available Commands\n\n");
            help.push_str("\u{2728} Basics\n");
            help.push_str("  /help       Show this help\n");
            help.push_str("  /clear      Clear conversation\n");
            help.push_str("  /quit       Exit VerySmolCode\n");
            help.push_str("\n\u{1F3AF} Model Control\n");
            help.push_str("  /fast       Use Flash (saves Pro budget)\n");
            help.push_str("  /smart      Use Pro (best quality)\n");
            help.push_str("  /plan       Toggle planning mode\n");
            help.push_str("  /model      Show available models\n");
            help.push_str("\n\u{1F4CA} Status & Tokens\n");
            help.push_str("  /tokens     Token usage dashboard\n");
            help.push_str("  /status     Rate limits & usage\n");
            help.push_str("  /compact    Compact conversation\n");
            help.push_str("\n\u{1F527} Tools\n");
            help.push_str("  /undo       Revert last file changes\n");
            help.push_str("  /diff       Show git diff\n");
            help.push_str("  /save       Save conversation to file\n");
            help.push_str("  /todo       Show task list\n");
            help.push_str("  /retry      Retry last message\n");
            help.push_str("  /resume     Resume previous session\n");
            help.push_str("  /config     Show/edit configuration\n");
            help.push_str("\n\u{1F50C} MCP Servers\n");
            help.push_str("  /mcp        List MCP servers\n");
            help.push_str("  /mcp-add    Add MCP server\n");
            help.push_str("  /mcp-rm     Remove MCP server\n");
            help.push_str("\n\u{2328}\u{FE0F}  Keybindings\n");
            help.push_str("  Ctrl+C     Cancel/Quit\n  Ctrl+D     Quit (on empty input)\n");
            help.push_str("  Ctrl+L     Clear screen\n");
            help.push_str("  Ctrl+A/E   Jump to start/end of line\n");
            help.push_str("  Ctrl+R     Search history (reverse)\n");
            help.push_str("  Ctrl+U/K   Delete before/after cursor\n");
            help.push_str("  Ctrl+W     Delete word backward\n");
            help.push_str("  Up/Down    History / Navigate commands\n");
            help.push_str("  PgUp/PgDn  Scroll output\n");
            help.push_str("  Tab        Select command from popup\n");
            help.push_str("  Esc        Dismiss command popup\n");
            help.push_str("  \\ + Enter  Multi-line input\n");
            help.push_str("\n\u{1F4BB} Shell Mode\n");
            help.push_str("  !<command>  Run shell command directly (e.g. !ls -la)\n");
            help.push_str("\n\u{1F4A1} Tip: Type / to see command suggestions!");
            CommandResponse::Message(help)
        }
        "/quit" | "/q" | "/exit" => CommandResponse::Quit,
        "/clear" => CommandResponse::Clear,
        "/fast" | "/f" => CommandResponse::SetModelOverride("fast".to_string()),
        "/smart" | "/s" => CommandResponse::SetModelOverride("smart".to_string()),
        "/plan" => CommandResponse::TogglePlan,
        "/tokens" | "/status" => CommandResponse::ShowTokens,
        "/undo" | "/u" => CommandResponse::Undo,
        "/save" => {
            let filename = if args.is_empty() {
                None
            } else {
                Some(args.to_string())
            };
            CommandResponse::Save(filename)
        }
        "/config" => {
            if let Some(set_rest) = args.strip_prefix("set ") {
                let set_args: Vec<&str> = set_rest.splitn(2, ' ').collect();
                if set_args.len() < 2 {
                    return CommandResponse::Message(
                        "Usage: /config set <key> <value>\n\
                         Keys: temperature, max_tokens, compact_threshold, command_timeout, safety"
                            .to_string(),
                    );
                }
                let key = set_args[0];
                let val = set_args[1].trim();
                let mut config = crate::config::Config::load();
                let result = match key {
                    "temperature" | "temp" => val
                        .parse::<f32>()
                        .map(|v| {
                            config.temperature = v.clamp(0.0, 2.0);
                            format!("Temperature set to {}", config.temperature)
                        })
                        .map_err(|_| "Invalid number".to_string()),
                    "max_tokens" => val
                        .parse::<u32>()
                        .map(|v| {
                            config.max_tokens_per_response = v.clamp(256, 65536);
                            format!(
                                "Max tokens/response set to {}",
                                config.max_tokens_per_response
                            )
                        })
                        .map_err(|_| "Invalid number".to_string()),
                    "compact_threshold" => val
                        .parse::<u32>()
                        .map(|v| {
                            config.auto_compact_threshold = v.clamp(4000, 128000);
                            format!(
                                "Auto-compact threshold set to {}",
                                config.auto_compact_threshold
                            )
                        })
                        .map_err(|_| "Invalid number".to_string()),
                    "command_timeout" | "timeout" => val
                        .parse::<u64>()
                        .map(|v| {
                            config.command_timeout = v.clamp(5, 600);
                            crate::tools::git::set_command_timeout_secs(config.command_timeout);
                            format!("Command timeout set to {}s", config.command_timeout)
                        })
                        .map_err(|_| "Invalid number".to_string()),
                    "safety" => match val {
                        "on" | "true" | "enabled" => {
                            config.safety_enabled = true;
                            Ok("Safety checks enabled".to_string())
                        }
                        "off" | "false" | "disabled" => {
                            config.safety_enabled = false;
                            Ok("Safety checks disabled".to_string())
                        }
                        _ => Err("Use: on/off".to_string()),
                    },
                    _ => Err(format!(
                        "Unknown key: {}. Valid: temperature, max_tokens, compact_threshold, safety",
                        key
                    )),
                };
                match result {
                    Ok(msg) => {
                        let _ = config.save();
                        CommandResponse::Message(msg)
                    }
                    Err(e) => CommandResponse::Message(format!("Error: {}", e)),
                }
            } else {
                let config = crate::config::Config::load();
                let msg = format!(
                    "Current configuration:\n\
                     Max tokens/response: {}\n\
                     Max conversation tokens: {}\n\
                     Temperature: {}\n\
                     Auto-compact threshold: {}\n\
                     Command timeout: {}s\n\
                     Safety checks: {}\n\
                     \n\
                     Use /config set <key> <value> to change.\n\
                     Keys: temperature, max_tokens, compact_threshold, command_timeout, safety",
                    config.max_tokens_per_response,
                    config.max_conversation_tokens,
                    config.temperature,
                    config.auto_compact_threshold,
                    config.command_timeout,
                    if config.safety_enabled {
                        "enabled"
                    } else {
                        "disabled"
                    },
                );
                CommandResponse::Message(msg)
            }
        }
        "/compact" => CommandResponse::Compact,
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
                     /mcp-add context7 npx -y @upstash/context7-mcp\n\
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
                     Example: /mcp-add context7 npx -y @upstash/context7-mcp"
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
        "/retry" | "/r" => CommandResponse::Retry,
        "/diff" | "/d" => {
            let diff_args = if args.is_empty() { "" } else { args };
            let output = std::process::Command::new("git")
                .args(["diff"])
                .args(diff_args.split_whitespace())
                .output();
            match output {
                Ok(out) => {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    if stdout.is_empty() && stderr.is_empty() {
                        CommandResponse::Message("No unstaged changes.".to_string())
                    } else if !stderr.is_empty() && stdout.is_empty() {
                        CommandResponse::Message(format!("git diff error: {}", stderr.trim()))
                    } else {
                        // Truncate large diffs
                        let text = if stdout.len() > 8000 {
                            format!(
                                "{}...\n\n(truncated, {} bytes total)",
                                &stdout[..8000],
                                stdout.len()
                            )
                        } else {
                            stdout.to_string()
                        };
                        CommandResponse::Message(text)
                    }
                }
                Err(e) => CommandResponse::Message(format!("Failed to run git diff: {}", e)),
            }
        }
        "/todo" | "/t" => CommandResponse::ShowTodo,
        "/resume" => {
            if args.is_empty() {
                CommandResponse::Resume(None)
            } else {
                CommandResponse::Resume(Some(args.to_string()))
            }
        }
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
