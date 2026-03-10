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
    ("/version", "Show version information"),
];

pub fn handle_command(input: &str) -> CommandResponse {
    let parts: Vec<&str> = input.splitn(2, ' ').collect();
    let cmd = parts[0].to_lowercase();
    let _args = parts.get(1).unwrap_or(&"");

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
                       Gemini 2.5 Pro      - 5 RPM, 25 RPD  (complex tasks)\n\
                       Gemini 2.5 Flash    - 10 RPM, 250 RPD (general tasks)\n\
                       Gemini 2.0 Flash-Lite - 15 RPM, 1000 RPD (simple tasks)\n\n\
                       Model is automatically selected based on task complexity.\n\
                       Falls back to simpler models when rate limits are hit.";
            CommandResponse::Message(msg.to_string())
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
