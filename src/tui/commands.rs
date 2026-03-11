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
    (
        "/search",
        "Toggle Google Search grounding (web-aware responses)",
    ),
    ("/version", "Show version information"),
    ("/retry", "Retry the last message"),
    ("/todo", "Show current task list"),
    (
        "/resume",
        "Resume last session or list recent: /resume [id]",
    ),
    ("/copy", "Copy last response to clipboard"),
    ("/diff", "Show git diff (unstaged changes)"),
    ("/new", "Start a new conversation (saves current session)"),
    (
        "/agents",
        "Show loaded AGENTS.md / CLAUDE.md instruction files",
    ),
    (
        "/telegram",
        "Show Telegram bot status or setup: /telegram setup <token> <chat_id>",
    ),
    (
        "/telegram-test",
        "Send a test message to verify Telegram connection",
    ),
    ("/telegram-off", "Disable Telegram integration"),
    (
        "/loop",
        "Loop a prompt repeatedly: /loop [5m] [--max N] <prompt>",
    ),
    ("/loop-cancel", "Cancel the active loop"),
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
            help.push_str("  /copy       Copy last response to clipboard\n");
            help.push_str("  /todo       Show task list\n");
            help.push_str("  /retry      Retry last message\n");
            help.push_str("  /resume     Resume previous session\n");
            help.push_str("  /new        Start new conversation\n");
            help.push_str("  /search     Toggle web search grounding\n");
            help.push_str("  /config     Show/edit configuration\n");
            help.push_str("\n\u{1F50C} MCP Servers\n");
            help.push_str("  /mcp        List MCP servers\n");
            help.push_str("  /mcp-add    Add MCP server\n");
            help.push_str("  /mcp-rm     Remove MCP server\n");
            help.push_str("\n\u{1F4F1} Telegram\n");
            help.push_str("  /telegram      Setup & status\n");
            help.push_str("  /telegram-test Send test message\n");
            help.push_str("  /telegram-off  Disable integration\n");
            help.push_str("\n\u{2328}\u{FE0F}  Keybindings\n");
            help.push_str("  Ctrl+C     Cancel/Quit\n  Ctrl+D     Quit (on empty input)\n");
            help.push_str("  Ctrl+L     Clear screen\n");
            help.push_str("  Ctrl+A/E   Jump to start/end of line\n");
            help.push_str("  Ctrl+T     Toggle task list popup\n");
            help.push_str("  Ctrl+P     Command palette\n");
            help.push_str("  Ctrl+R     Search history (reverse)\n");
            help.push_str("  Ctrl+U/K   Delete before/after cursor\n");
            help.push_str("  Ctrl+W     Delete word backward\n");
            help.push_str("  Up/Down    History / Navigate commands\n");
            help.push_str("  PgUp/PgDn  Scroll output\n");
            help.push_str("  Tab        Select command from popup\n");
            help.push_str("  Esc        Stop agent / Dismiss popup\n");
            help.push_str("  \\ + Enter  Multi-line input\n");
            help.push_str("\n\u{1F504} Loop Mode\n");
            help.push_str("  /loop <prompt>        Loop prompt after each completion\n");
            help.push_str("  /loop 5m <prompt>     Loop every 5 minutes\n");
            help.push_str("  /loop --max N <prompt> Loop N times then stop\n");
            help.push_str("  /loop-cancel          Cancel active loop\n");
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
        "/search" => CommandResponse::ToggleSearch,
        "/copy" | "/cp" => CommandResponse::CopyLast,
        "/version" => CommandResponse::Message(format!(
            "VerySmolCode v{}\nA lightweight coding assistant for constrained devices",
            env!("CARGO_PKG_VERSION")
        )),
        "/new" | "/n" => CommandResponse::NewSession,
        "/agents" => {
            let config_dir = crate::config::Config::config_dir();
            let user_path = config_dir.join("AGENTS.md");
            let mut msg = String::from("AGENTS.md / CLAUDE.md instruction files:\n\n");

            // User-level
            if user_path.exists() {
                let size = std::fs::metadata(&user_path).map(|m| m.len()).unwrap_or(0);
                msg.push_str(&format!(
                    "  [loaded] {} ({} bytes)\n",
                    user_path.display(),
                    size
                ));
            } else {
                msg.push_str(&format!(
                    "  [not found] {} (create for user-level instructions)\n",
                    user_path.display()
                ));
            }

            // Project-level
            let root = std::env::current_dir().unwrap_or_default();
            for filename in &["AGENTS.md", "CLAUDE.md"] {
                let path = root.join(filename);
                if path.exists() {
                    let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                    msg.push_str(&format!("  [loaded] {} ({} bytes)\n", path.display(), size));
                }
            }

            msg.push_str(
                "\nThese files are automatically injected into the AI's system prompt.\n\
                 Create AGENTS.md in your project root for project-specific instructions.",
            );
            CommandResponse::Message(msg)
        }
        "/telegram" => {
            if let Some(setup_rest) = args.strip_prefix("setup ") {
                let setup_parts: Vec<&str> = setup_rest.splitn(2, ' ').collect();
                if setup_parts.len() < 2 {
                    return CommandResponse::Message(
                        "Usage: /telegram setup <bot_token> <chat_id>\n\n\
                         1. Open Telegram and chat with @BotFather\n\
                         2. Send /newbot and follow instructions to get a token\n\
                         3. Send a message to your bot, then visit:\n\
                            https://api.telegram.org/bot<TOKEN>/getUpdates\n\
                         4. Find your chat_id in the response\n\
                         5. Run: /telegram setup <token> <chat_id>"
                            .to_string(),
                    );
                }
                let token = setup_parts[0].trim().to_string();
                let chat_id =
                    match setup_parts[1].trim().parse::<i64>() {
                        Ok(id) => id,
                        Err(_) => return CommandResponse::Message(
                            "Invalid chat_id. It should be a number (can be negative for groups)."
                                .to_string(),
                        ),
                    };

                // Verify the token works
                let bot = crate::telegram::bot::TelegramBot::new(token.clone(), chat_id);
                match bot.verify() {
                    Ok(bot_name) => {
                        let config = crate::telegram::config::TelegramConfig {
                            bot_token: Some(token),
                            chat_id: Some(chat_id),
                            enabled: true,
                        };
                        match config.save() {
                            Ok(()) => CommandResponse::Message(format!(
                                "Telegram connected! Bot: {}\n\
                                 Chat ID: {}\n\
                                 Use /telegram-test to send a test message.\n\
                                 The agent can now send you messages via Telegram.",
                                bot_name, chat_id
                            )),
                            Err(e) => {
                                CommandResponse::Message(format!("Failed to save config: {}", e))
                            }
                        }
                    }
                    Err(e) => CommandResponse::Message(format!(
                        "Bot verification failed: {}\nPlease check your token.",
                        e
                    )),
                }
            } else {
                let config = crate::telegram::config::TelegramConfig::load();
                if config.is_configured() {
                    CommandResponse::Message(format!(
                        "Telegram: enabled\n\
                         Chat ID: {}\n\
                         Token: {}...{}\n\n\
                         Commands:\n\
                         /telegram-test  Send a test message\n\
                         /telegram-off   Disable Telegram\n\n\
                         The agent has a send_telegram tool to message you.",
                        config.chat_id.unwrap(),
                        &config.bot_token.as_ref().unwrap()
                            [..8.min(config.bot_token.as_ref().unwrap().len())],
                        &config.bot_token.as_ref().unwrap()
                            [config.bot_token.as_ref().unwrap().len().saturating_sub(4)..],
                    ))
                } else {
                    CommandResponse::Message(
                        "Telegram: not configured\n\n\
                         Setup instructions:\n\
                         1. Open Telegram and chat with @BotFather\n\
                         2. Send /newbot and follow instructions to get a token\n\
                         3. Send a message to your bot, then visit:\n\
                            https://api.telegram.org/bot<TOKEN>/getUpdates\n\
                         4. Find your chat_id in the response JSON\n\
                         5. Run: /telegram setup <token> <chat_id>"
                            .to_string(),
                    )
                }
            }
        }
        "/telegram-test" => {
            let config = crate::telegram::config::TelegramConfig::load();
            match crate::telegram::bot::TelegramBot::from_config(&config) {
                Some(bot) => match bot
                    .send_message("VerySmolCode connected! The agent can now send you messages.")
                {
                    Ok(()) => CommandResponse::Message(
                        "Test message sent! Check your Telegram.".to_string(),
                    ),
                    Err(e) => CommandResponse::Message(format!("Failed to send: {}", e)),
                },
                None => CommandResponse::Message(
                    "Telegram not configured. Use /telegram setup <token> <chat_id>".to_string(),
                ),
            }
        }
        "/telegram-off" => {
            let mut config = crate::telegram::config::TelegramConfig::load();
            config.enabled = false;
            match config.save() {
                Ok(()) => CommandResponse::Message("Telegram integration disabled.".to_string()),
                Err(e) => CommandResponse::Message(format!("Failed to save config: {}", e)),
            }
        }
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
        "/loop" => {
            let trimmed = args.trim();
            if trimmed.is_empty() {
                return CommandResponse::LoopStatus;
            }
            if trimmed == "off" || trimmed == "cancel" || trimmed == "stop" {
                return CommandResponse::LoopCancel;
            }
            let (interval_secs, max_iterations, prompt) = parse_loop_args(trimmed);
            if prompt.is_empty() {
                CommandResponse::Message(
                    "Usage: /loop [interval] [--max N] <prompt>\n\
                     Examples:\n\
                     /loop check the build          (runs after each completion)\n\
                     /loop 5m run tests             (runs every 5 minutes)\n\
                     /loop --max 3 optimize code    (3 iterations max)\n\
                     /loop 10m --max 5 check status (every 10m, max 5 times)\n\
                     /loop off                      (cancel active loop)"
                        .to_string(),
                )
            } else {
                CommandResponse::StartLoop {
                    prompt,
                    interval_secs,
                    max_iterations,
                }
            }
        }
        "/loop-cancel" | "/loop-stop" => CommandResponse::LoopCancel,
        _ => CommandResponse::Message(format!(
            "Unknown command: {}. Type /help for available commands.",
            cmd
        )),
    }
}

/// Parse interval string: "5m" -> 300, "30s" -> 30, "1h" -> 3600
pub fn parse_interval(s: &str) -> Option<u64> {
    if let Some(n) = s.strip_suffix('m').and_then(|n| n.parse::<u64>().ok()) {
        if n > 0 {
            Some(n * 60)
        } else {
            None
        }
    } else if let Some(n) = s.strip_suffix('s').and_then(|n| n.parse::<u64>().ok()) {
        if n > 0 {
            Some(n)
        } else {
            None
        }
    } else if let Some(n) = s.strip_suffix('h').and_then(|n| n.parse::<u64>().ok()) {
        if n > 0 {
            Some(n * 3600)
        } else {
            None
        }
    } else {
        None
    }
}

/// Parse loop args: "[interval] [--max N] <prompt>"
/// Returns (interval_secs, max_iterations, prompt)
pub fn parse_loop_args(args: &str) -> (u64, u32, String) {
    let parts: Vec<&str> = args.split_whitespace().collect();
    let mut interval = 0u64;
    let mut max_iter = 0u32;
    let mut i = 0;

    // Optional first arg: interval like "5m", "30s", "1h"
    if let Some(&first) = parts.first() {
        if let Some(secs) = parse_interval(first) {
            interval = secs;
            i = 1;
        }
    }

    let mut prompt_parts: Vec<&str> = Vec::new();
    while i < parts.len() {
        if (parts[i] == "--max" || parts[i] == "-n") && i + 1 < parts.len() {
            if let Ok(n) = parts[i + 1].parse::<u32>() {
                max_iter = n;
                i += 2;
                continue;
            }
        }
        prompt_parts.push(parts[i]);
        i += 1;
    }

    (interval, max_iter, prompt_parts.join(" "))
}

pub fn autocomplete(input: &str) -> Vec<String> {
    COMMANDS
        .iter()
        .filter(|(cmd, _)| cmd.starts_with(input))
        .map(|(cmd, _)| cmd.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_help_command() {
        let resp = handle_command("/help");
        assert!(
            matches!(resp, CommandResponse::Message(ref s) if s.contains("Available Commands"))
        );
    }

    #[test]
    fn test_help_alias() {
        let resp = handle_command("/h");
        assert!(
            matches!(resp, CommandResponse::Message(ref s) if s.contains("Available Commands"))
        );
    }

    #[test]
    fn test_quit_command() {
        assert!(matches!(handle_command("/quit"), CommandResponse::Quit));
        assert!(matches!(handle_command("/q"), CommandResponse::Quit));
        assert!(matches!(handle_command("/exit"), CommandResponse::Quit));
    }

    #[test]
    fn test_clear_command() {
        assert!(matches!(handle_command("/clear"), CommandResponse::Clear));
    }

    #[test]
    fn test_fast_command() {
        let resp = handle_command("/fast");
        assert!(matches!(resp, CommandResponse::SetModelOverride(ref s) if s == "fast"));
        let resp = handle_command("/f");
        assert!(matches!(resp, CommandResponse::SetModelOverride(ref s) if s == "fast"));
    }

    #[test]
    fn test_smart_command() {
        let resp = handle_command("/smart");
        assert!(matches!(resp, CommandResponse::SetModelOverride(ref s) if s == "smart"));
        let resp = handle_command("/s");
        assert!(matches!(resp, CommandResponse::SetModelOverride(ref s) if s == "smart"));
    }

    #[test]
    fn test_plan_command() {
        assert!(matches!(
            handle_command("/plan"),
            CommandResponse::TogglePlan
        ));
    }

    #[test]
    fn test_tokens_command() {
        assert!(matches!(
            handle_command("/tokens"),
            CommandResponse::ShowTokens
        ));
        assert!(matches!(
            handle_command("/status"),
            CommandResponse::ShowTokens
        ));
    }

    #[test]
    fn test_undo_command() {
        assert!(matches!(handle_command("/undo"), CommandResponse::Undo));
        assert!(matches!(handle_command("/u"), CommandResponse::Undo));
    }

    #[test]
    fn test_save_no_args() {
        let resp = handle_command("/save");
        assert!(matches!(resp, CommandResponse::Save(None)));
    }

    #[test]
    fn test_save_with_filename() {
        let resp = handle_command("/save myfile.md");
        assert!(matches!(resp, CommandResponse::Save(Some(ref s)) if s == "myfile.md"));
    }

    #[test]
    fn test_compact_command() {
        assert!(matches!(
            handle_command("/compact"),
            CommandResponse::Compact
        ));
    }

    #[test]
    fn test_model_command() {
        let resp = handle_command("/model");
        assert!(matches!(resp, CommandResponse::Message(ref s) if s.contains("Gemini")));
    }

    #[test]
    fn test_search_command() {
        assert!(matches!(
            handle_command("/search"),
            CommandResponse::ToggleSearch
        ));
    }

    #[test]
    fn test_copy_command() {
        assert!(matches!(handle_command("/copy"), CommandResponse::CopyLast));
        assert!(matches!(handle_command("/cp"), CommandResponse::CopyLast));
    }

    #[test]
    fn test_version_command() {
        let resp = handle_command("/version");
        assert!(matches!(resp, CommandResponse::Message(ref s) if s.contains("VerySmolCode")));
    }

    #[test]
    fn test_new_command() {
        assert!(matches!(
            handle_command("/new"),
            CommandResponse::NewSession
        ));
        assert!(matches!(handle_command("/n"), CommandResponse::NewSession));
    }

    #[test]
    fn test_retry_command() {
        assert!(matches!(handle_command("/retry"), CommandResponse::Retry));
        assert!(matches!(handle_command("/r"), CommandResponse::Retry));
    }

    #[test]
    fn test_todo_command() {
        assert!(matches!(handle_command("/todo"), CommandResponse::ShowTodo));
        assert!(matches!(handle_command("/t"), CommandResponse::ShowTodo));
    }

    #[test]
    fn test_resume_no_args() {
        let resp = handle_command("/resume");
        assert!(matches!(resp, CommandResponse::Resume(None)));
    }

    #[test]
    fn test_resume_with_id() {
        let resp = handle_command("/resume abc123");
        assert!(matches!(resp, CommandResponse::Resume(Some(ref s)) if s == "abc123"));
    }

    #[test]
    fn test_unknown_command() {
        let resp = handle_command("/nonexistent");
        assert!(matches!(resp, CommandResponse::Message(ref s) if s.contains("Unknown command")));
    }

    #[test]
    fn test_case_insensitive() {
        assert!(matches!(handle_command("/QUIT"), CommandResponse::Quit));
        assert!(matches!(
            handle_command("/Help"),
            CommandResponse::Message(_)
        ));
    }

    #[test]
    fn test_config_show() {
        let resp = handle_command("/config");
        assert!(
            matches!(resp, CommandResponse::Message(ref s) if s.contains("Current configuration"))
        );
    }

    #[test]
    fn test_config_set_missing_value() {
        let resp = handle_command("/config set temperature");
        assert!(matches!(resp, CommandResponse::Message(ref s) if s.contains("Usage")));
    }

    #[test]
    fn test_config_set_unknown_key() {
        let resp = handle_command("/config set foobar 42");
        assert!(matches!(resp, CommandResponse::Message(ref s) if s.contains("Unknown key")));
    }

    #[test]
    fn test_mcp_add_missing_args() {
        let resp = handle_command("/mcp-add");
        assert!(matches!(resp, CommandResponse::Message(ref s) if s.contains("Usage")));
    }

    #[test]
    fn test_mcp_rm_missing_name() {
        let resp = handle_command("/mcp-rm");
        assert!(matches!(resp, CommandResponse::Message(ref s) if s.contains("Usage")));
    }

    #[test]
    fn test_diff_command() {
        let resp = handle_command("/diff");
        assert!(matches!(resp, CommandResponse::Message(_)));
    }

    #[test]
    fn test_autocomplete_slash() {
        let results = autocomplete("/");
        assert!(results.len() > 10); // There are 20+ commands
        assert!(results.contains(&"/help".to_string()));
    }

    #[test]
    fn test_autocomplete_partial() {
        let results = autocomplete("/he");
        assert_eq!(results, vec!["/help"]);
    }

    #[test]
    fn test_autocomplete_no_match() {
        let results = autocomplete("/zzz");
        assert!(results.is_empty());
    }

    #[test]
    fn test_autocomplete_multiple_matches() {
        let results = autocomplete("/mc");
        assert!(results.len() >= 3); // /mcp, /mcp-add, /mcp-rm
    }

    #[test]
    fn test_commands_list_not_empty() {
        assert!(COMMANDS.len() > 20);
    }

    #[test]
    fn test_commands_all_start_with_slash() {
        for (cmd, _) in COMMANDS {
            assert!(cmd.starts_with('/'), "Command {} doesn't start with /", cmd);
        }
    }

    #[test]
    fn test_commands_no_empty_descriptions() {
        for (cmd, desc) in COMMANDS {
            assert!(!desc.is_empty(), "Command {} has empty description", cmd);
        }
    }

    #[test]
    fn test_loop_no_args_returns_status() {
        let resp = handle_command("/loop");
        assert!(matches!(resp, CommandResponse::LoopStatus));
    }

    #[test]
    fn test_loop_off_returns_cancel() {
        assert!(matches!(
            handle_command("/loop off"),
            CommandResponse::LoopCancel
        ));
        assert!(matches!(
            handle_command("/loop cancel"),
            CommandResponse::LoopCancel
        ));
        assert!(matches!(
            handle_command("/loop stop"),
            CommandResponse::LoopCancel
        ));
    }

    #[test]
    fn test_loop_cancel_command() {
        assert!(matches!(
            handle_command("/loop-cancel"),
            CommandResponse::LoopCancel
        ));
        assert!(matches!(
            handle_command("/loop-stop"),
            CommandResponse::LoopCancel
        ));
    }

    #[test]
    fn test_loop_no_prompt_returns_usage() {
        let resp = handle_command("/loop 5m");
        // "5m" parses as interval, no prompt remains → Message
        assert!(matches!(resp, CommandResponse::Message(_)));
    }

    #[test]
    fn test_loop_immediate_prompt() {
        let resp = handle_command("/loop check the build");
        assert!(
            matches!(resp, CommandResponse::StartLoop { ref prompt, interval_secs: 0, max_iterations: 0 } if prompt == "check the build")
        );
    }

    #[test]
    fn test_loop_with_interval_minutes() {
        let resp = handle_command("/loop 5m run tests");
        assert!(
            matches!(resp, CommandResponse::StartLoop { ref prompt, interval_secs: 300, max_iterations: 0 } if prompt == "run tests")
        );
    }

    #[test]
    fn test_loop_with_interval_seconds() {
        let resp = handle_command("/loop 30s do something");
        assert!(
            matches!(resp, CommandResponse::StartLoop { ref prompt, interval_secs: 30, max_iterations: 0 } if prompt == "do something")
        );
    }

    #[test]
    fn test_loop_with_interval_hours() {
        let resp = handle_command("/loop 1h check status");
        assert!(
            matches!(resp, CommandResponse::StartLoop { ref prompt, interval_secs: 3600, max_iterations: 0 } if prompt == "check status")
        );
    }

    #[test]
    fn test_loop_with_max_iterations() {
        let resp = handle_command("/loop --max 5 optimize code");
        assert!(
            matches!(resp, CommandResponse::StartLoop { ref prompt, interval_secs: 0, max_iterations: 5 } if prompt == "optimize code")
        );
    }

    #[test]
    fn test_loop_interval_and_max() {
        let resp = handle_command("/loop 10m --max 3 check build");
        assert!(
            matches!(resp, CommandResponse::StartLoop { ref prompt, interval_secs: 600, max_iterations: 3 } if prompt == "check build")
        );
    }

    #[test]
    fn test_parse_interval() {
        assert_eq!(parse_interval("5m"), Some(300));
        assert_eq!(parse_interval("30s"), Some(30));
        assert_eq!(parse_interval("1h"), Some(3600));
        assert_eq!(parse_interval("2h"), Some(7200));
        assert_eq!(parse_interval("hello"), None);
        assert_eq!(parse_interval("0m"), None); // 0 not allowed
        assert_eq!(parse_interval(""), None);
    }

    #[test]
    fn test_parse_loop_args_immediate() {
        let (interval, max, prompt) = parse_loop_args("check the build");
        assert_eq!(interval, 0);
        assert_eq!(max, 0);
        assert_eq!(prompt, "check the build");
    }

    #[test]
    fn test_parse_loop_args_interval() {
        let (interval, max, prompt) = parse_loop_args("5m run tests");
        assert_eq!(interval, 300);
        assert_eq!(max, 0);
        assert_eq!(prompt, "run tests");
    }

    #[test]
    fn test_parse_loop_args_max_only() {
        let (interval, max, prompt) = parse_loop_args("--max 3 do work");
        assert_eq!(interval, 0);
        assert_eq!(max, 3);
        assert_eq!(prompt, "do work");
    }

    #[test]
    fn test_parse_loop_args_interval_and_max() {
        let (interval, max, prompt) = parse_loop_args("10m --max 5 check status");
        assert_eq!(interval, 600);
        assert_eq!(max, 5);
        assert_eq!(prompt, "check status");
    }

    #[test]
    fn test_parse_loop_args_empty() {
        let (interval, max, prompt) = parse_loop_args("");
        assert_eq!(interval, 0);
        assert_eq!(max, 0);
        assert_eq!(prompt, "");
    }
}
