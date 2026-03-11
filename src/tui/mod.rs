pub mod app;
pub mod commands;
pub mod input;
pub mod session;
pub mod ui;

use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::prelude::*;
use std::io;

use app::App;

pub fn run() -> Result<(), String> {
    // Setup terminal
    enable_raw_mode().map_err(|e| format!("Failed to enable raw mode: {}", e))?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)
        .map_err(|e| format!("Failed to enter alternate screen: {}", e))?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal =
        Terminal::new(backend).map_err(|e| format!("Failed to create terminal: {}", e))?;

    // Create app
    let mut app = App::new()?;

    // Main loop
    let result = run_app(&mut terminal, &mut app);

    // Auto-save session on exit (only if there are messages)
    if !app.messages.is_empty() {
        let session = session::Session::new(
            &app.messages,
            &app.input_history,
            app.total_input_tokens,
            app.total_output_tokens,
            app.total_thinking_tokens,
        );
        if let Err(e) = session.save() {
            eprintln!("Warning: failed to save session: {}", e);
        }
    }

    // Restore terminal
    disable_raw_mode().ok();
    execute!(terminal.backend_mut(), LeaveAlternateScreen).ok();
    terminal.show_cursor().ok();

    result
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<(), String> {
    loop {
        // Only redraw when state has changed
        if app.dirty {
            terminal
                .draw(|f| ui::draw(f, app))
                .map_err(|e| format!("Draw error: {}", e))?;
            app.dirty = false;
        }

        // Poll for events with a timeout so we can update the UI
        if event::poll(std::time::Duration::from_millis(100))
            .map_err(|e| format!("Event poll error: {}", e))?
        {
            match event::read() {
                Ok(Event::Key(key)) => {
                    app.dirty = true;
                    match (key.modifiers, key.code) {
                        (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                            if app.is_processing {
                                app.cancel_processing();
                            } else {
                                return Ok(());
                            }
                        }
                        (KeyModifiers::CONTROL, KeyCode::Char('l')) => {
                            app.clear_screen();
                        }
                        _ => {
                            input::handle_key(app, key);
                        }
                    }
                }
                Ok(Event::Resize(_, _)) => {
                    app.dirty = true;
                }
                _ => {}
            }
        }

        // Process any pending agent work
        app.tick();

        // Force periodic redraw when processing (for spinner animation)
        if app.is_processing {
            app.dirty = true;
        }

        if app.should_quit {
            return Ok(());
        }
    }
}
