#![allow(dead_code)]

mod agent;
mod api;
mod config;
mod mcp;
mod tools;
mod tui;

use std::env;
use std::io::{self, Read};

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
        let prompt = if args.len() >= 3 {
            // Prompt from command line args
            args[2..].join(" ")
        } else {
            // Read from stdin (pipe mode)
            let mut input = String::new();
            io::stdin().read_to_string(&mut input).unwrap_or_default();
            input.trim().to_string()
        };

        if prompt.is_empty() {
            eprintln!("Usage: vsc -p \"your prompt here\"");
            eprintln!("       echo \"your prompt\" | vsc -p");
            std::process::exit(1);
        }

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

fn run_prompt_mode(prompt: &str) {
    use agent::AgentLoop;

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
                                serde_json::Value::String(s) if s.len() > 40 => {
                                    format!("{}...", &s[..37])
                                }
                                serde_json::Value::String(s) => s.clone(),
                                other => {
                                    let s = other.to_string();
                                    if s.len() > 40 {
                                        format!("{}...", &s[..37])
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
                eprintln!("[tool] {}({})", name, args_brief);
            }
            AgentEvent::ToolResult { name, .. } => {
                eprintln!("[done] {}", name);
            }
            AgentEvent::Status(s) => {
                if !s.starts_with("RATE:") {
                    eprintln!("[status] {}", s);
                }
            }
            AgentEvent::ModelSwitch(m) => {
                eprintln!("[model] {}", m);
            }
            _ => {}
        }
    });

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
