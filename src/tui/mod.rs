pub mod app;
pub mod commands;
pub mod input;
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
        terminal
            .draw(|f| ui::draw(f, app))
            .map_err(|e| format!("Draw error: {}", e))?;

        // Poll for events with a timeout so we can update the UI
        if event::poll(std::time::Duration::from_millis(100))
            .map_err(|e| format!("Event poll error: {}", e))?
        {
            if let Ok(Event::Key(key)) = event::read() {
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
        }

        // Process any pending agent work
        app.tick();

        if app.should_quit {
            return Ok(());
        }
    }
}
