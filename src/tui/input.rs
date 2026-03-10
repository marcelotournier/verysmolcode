use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crate::tui::app::App;

pub fn handle_key(app: &mut App, key: KeyEvent) {
    if app.is_processing {
        // Only allow scrolling while processing
        match key.code {
            KeyCode::PageUp => app.scroll_up(),
            KeyCode::PageDown => app.scroll_down(),
            _ => {}
        }
        return;
    }

    match key.code {
        KeyCode::Enter => {
            app.submit_input();
        }
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                match c {
                    'a' => app.cursor_pos = 0,
                    'e' => app.cursor_pos = app.input.len(),
                    'u' => {
                        app.input.drain(..app.cursor_pos);
                        app.cursor_pos = 0;
                    }
                    'k' => {
                        app.input.truncate(app.cursor_pos);
                    }
                    'w' => {
                        // Delete word backward
                        let mut pos = app.cursor_pos;
                        while pos > 0 && app.input.as_bytes().get(pos - 1) == Some(&b' ') {
                            pos -= 1;
                        }
                        while pos > 0 && app.input.as_bytes().get(pos - 1) != Some(&b' ') {
                            pos -= 1;
                        }
                        app.input.drain(pos..app.cursor_pos);
                        app.cursor_pos = pos;
                    }
                    _ => {}
                }
            } else {
                app.input.insert(app.cursor_pos, c);
                app.cursor_pos += 1;
            }
        }
        KeyCode::Backspace => {
            if app.cursor_pos > 0 {
                app.cursor_pos -= 1;
                app.input.remove(app.cursor_pos);
            }
        }
        KeyCode::Delete => {
            if app.cursor_pos < app.input.len() {
                app.input.remove(app.cursor_pos);
            }
        }
        KeyCode::Left => {
            app.cursor_pos = app.cursor_pos.saturating_sub(1);
        }
        KeyCode::Right => {
            if app.cursor_pos < app.input.len() {
                app.cursor_pos += 1;
            }
        }
        KeyCode::Home => {
            app.cursor_pos = 0;
        }
        KeyCode::End => {
            app.cursor_pos = app.input.len();
        }
        KeyCode::Up => {
            app.history_up();
        }
        KeyCode::Down => {
            app.history_down();
        }
        KeyCode::PageUp => {
            app.scroll_up();
        }
        KeyCode::PageDown => {
            app.scroll_down();
        }
        KeyCode::Tab => {
            // Auto-complete slash commands
            if app.input.starts_with('/') {
                let completions = crate::tui::commands::autocomplete(&app.input);
                if completions.len() == 1 {
                    app.input = completions[0].clone();
                    app.cursor_pos = app.input.len();
                }
            }
        }
        _ => {}
    }
}
