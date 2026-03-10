mod api;
mod agent;
mod config;
mod tools;
mod tui;

use std::env;

fn main() {
    // Check for API key early
    if env::var("GEMINI_API_KEY").is_err() {
        eprintln!("Error: GEMINI_API_KEY environment variable not set.");
        eprintln!("Get your free API key at: https://aistudio.google.com/apikey");
        std::process::exit(1);
    }

    if let Err(e) = tui::run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
