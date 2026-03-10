#![allow(dead_code)]

mod agent;
mod api;
mod config;
mod mcp;
mod tools;
mod tui;
mod utils;

use std::env;
use std::io::{self, IsTerminal, Read};

fn main() {
    // Check for API key early
    if env::var("GEMINI_API_KEY").is_err() {
        eprintln!("Error: GEMINI_API_KEY environment variable not set.");
        eprintln!("Get your free API key at: https://aistudio.google.com/apikey");
        std::process::exit(1);
    }

    let args: Vec<String> = env::args().collect();

    // Handle -p/--prompt mode (like claude -p)
    if args.len() >= 2 && (args[1] == "-p" || args[1] == "--prompt") {
        let cli_prompt = if args.len() >= 3 {
            args[2..].join(" ")
        } else {
            String::new()
        };

        // Read stdin if it's piped (not a terminal)
        let stdin_content = if !io::stdin().is_terminal() {
            let mut input = String::new();
            io::stdin().read_to_string(&mut input).unwrap_or_else(|e| {
                eprintln!("Warning: failed to read stdin: {}", e);
                0
            });
            input.trim().to_string()
        } else {
            String::new()
        };

        // Combine: "prompt\n\n<stdin content>" or just one of them
        let prompt = match (cli_prompt.is_empty(), stdin_content.is_empty()) {
            (false, false) => format!("{}\n\n{}", cli_prompt, stdin_content),
            (false, true) => cli_prompt,
            (true, false) => stdin_content,
            (true, true) => {
                eprintln!("Usage: vsc -p \"your prompt here\"");
                eprintln!("       echo \"your prompt\" | vsc -p");
                eprintln!("       cat file.py | vsc -p \"review this code\"");
                std::process::exit(1);
            }
        };

        run_prompt_mode(&prompt);
        return;
    }

    // Handle --version / -v
    if args.len() >= 2 && (args[1] == "--version" || args[1] == "-v") {
        println!("VerySmolCode v{}", env!("CARGO_PKG_VERSION"));
        return;
    }

    // Handle --help
    if args.len() >= 2 && (args[1] == "--help" || args[1] == "-h") {
        println!("VerySmolCode v{}", env!("CARGO_PKG_VERSION"));
        println!("A lightweight TUI coding assistant powered by Gemini API free tier.");
        println!();
        println!("Usage:");
        println!("  vsc              Launch interactive TUI");
        println!("  vsc -p \"prompt\"  Run a single prompt and exit (like claude -p)");
        println!("  echo \"..\" | vsc -p  Pipe input as prompt");
        println!("  vsc -v           Show version");
        println!("  vsc -h           Show this help");
        return;
    }

    if let Err(e) = tui::run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

// ANSI color helpers for prompt mode (only when stderr is a terminal)
struct PromptColors {
    use_color: bool,
}

impl PromptColors {
    fn new() -> Self {
        Self {
            use_color: io::stderr().is_terminal(),
        }
    }

    fn dim(&self, s: &str) -> String {
        if self.use_color {
            format!("\x1b[2m{}\x1b[0m", s)
        } else {
            s.to_string()
        }
    }

    fn green(&self, s: &str) -> String {
        if self.use_color {
            format!("\x1b[32m{}\x1b[0m", s)
        } else {
            s.to_string()
        }
    }

    fn cyan(&self, s: &str) -> String {
        if self.use_color {
            format!("\x1b[36m{}\x1b[0m", s)
        } else {
            s.to_string()
        }
    }

    fn yellow(&self, s: &str) -> String {
        if self.use_color {
            format!("\x1b[33m{}\x1b[0m", s)
        } else {
            s.to_string()
        }
    }
}

fn run_prompt_mode(prompt: &str) {
    use agent::AgentLoop;

    let colors = PromptColors::new();

    let mut agent = match AgentLoop::new() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    let result = agent.process_message(prompt, |event| {
        use agent::loop_runner::AgentEvent;
        match event {
            AgentEvent::Text(text) => {
                println!("{}", text);
            }
            AgentEvent::ToolCall { name, args } => {
                let args_brief = if let Some(obj) = args.as_object() {
                    obj.iter()
                        .map(|(k, v)| {
                            let val = match v {
                                serde_json::Value::String(s) if s.chars().count() > 40 => {
                                    let t: String = s.chars().take(37).collect();
                                    format!("{}...", t)
                                }
                                serde_json::Value::String(s) => s.clone(),
                                other => {
                                    let s = other.to_string();
                                    if s.chars().count() > 40 {
                                        let t: String = s.chars().take(37).collect();
                                        format!("{}...", t)
                                    } else {
                                        s
                                    }
                                }
                            };
                            format!("{}={}", k, val)
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                } else {
                    args.to_string()
                };
                eprintln!(
                    "{}",
                    colors.cyan(&format!("\u{1F529} {}({})", name, args_brief))
                );
            }
            AgentEvent::ToolResult {
                name, duration_ms, ..
            } => {
                if duration_ms > 0 {
                    eprintln!(
                        "{}",
                        colors.green(&format!("\u{2705} {} ({}ms)", name, duration_ms))
                    );
                } else {
                    eprintln!("{}", colors.green(&format!("\u{2705} {}", name)));
                }
            }
            AgentEvent::Status(s) => {
                if !s.starts_with("RATE:") && !s.starts_with("WARN:") {
                    eprintln!("{}", colors.dim(&format!("\u{1F4AC} {}", s)));
                }
            }
            AgentEvent::ModelSwitch(m) => {
                eprintln!("{}", colors.yellow(&format!("\u{2699}\u{FE0F}  {}", m)));
            }
            _ => {}
        }
    });

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
