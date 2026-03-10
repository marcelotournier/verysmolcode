use crate::tui::app::App;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Find the byte position of the previous character boundary
fn prev_char_boundary(s: &str, pos: usize) -> usize {
    let mut p = pos;
    while p > 0 {
        p -= 1;
        if s.is_char_boundary(p) {
            return p;
        }
    }
    0
}

/// Find the byte position of the next character boundary
fn next_char_boundary(s: &str, pos: usize) -> usize {
    let mut p = pos + 1;
    while p < s.len() {
        if s.is_char_boundary(p) {
            return p;
        }
        p += 1;
    }
    s.len()
}

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
                        // Delete word backward (char-boundary aware)
                        let mut pos = app.cursor_pos;
                        while pos > 0 {
                            let prev = prev_char_boundary(&app.input, pos);
                            if app.input[prev..pos].trim().is_empty() {
                                pos = prev;
                            } else {
                                break;
                            }
                        }
                        while pos > 0 {
                            let prev = prev_char_boundary(&app.input, pos);
                            if !app.input[prev..pos].trim().is_empty() {
                                pos = prev;
                            } else {
                                break;
                            }
                        }
                        app.input.drain(pos..app.cursor_pos);
                        app.cursor_pos = pos;
                    }
                    _ => {}
                }
            } else {
                app.input.insert(app.cursor_pos, c);
                app.cursor_pos += c.len_utf8();
            }
        }
        KeyCode::Backspace => {
            if app.cursor_pos > 0 {
                let prev = prev_char_boundary(&app.input, app.cursor_pos);
                app.input.drain(prev..app.cursor_pos);
                app.cursor_pos = prev;
            }
        }
        KeyCode::Delete => {
            if app.cursor_pos < app.input.len() {
                let next = next_char_boundary(&app.input, app.cursor_pos);
                app.input.drain(app.cursor_pos..next);
            }
        }
        KeyCode::Left => {
            if app.cursor_pos > 0 {
                app.cursor_pos = prev_char_boundary(&app.input, app.cursor_pos);
            }
        }
        KeyCode::Right => {
            if app.cursor_pos < app.input.len() {
                app.cursor_pos = next_char_boundary(&app.input, app.cursor_pos);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prev_char_boundary_ascii() {
        let s = "hello";
        assert_eq!(prev_char_boundary(s, 3), 2);
        assert_eq!(prev_char_boundary(s, 1), 0);
        assert_eq!(prev_char_boundary(s, 0), 0);
    }

    #[test]
    fn test_next_char_boundary_ascii() {
        let s = "hello";
        assert_eq!(next_char_boundary(s, 0), 1);
        assert_eq!(next_char_boundary(s, 3), 4);
        assert_eq!(next_char_boundary(s, 4), 5);
    }

    #[test]
    fn test_prev_char_boundary_multibyte() {
        let s = "a\u{1F600}b"; // a + 4-byte emoji + b
                               // Byte positions: a=0, emoji=1..5, b=5
        assert_eq!(prev_char_boundary(s, 5), 1); // before b -> start of emoji
        assert_eq!(prev_char_boundary(s, 1), 0); // before emoji -> a
                                                 // From middle of emoji (invalid position), backs up to emoji start
        assert_eq!(prev_char_boundary(s, 3), 1);
    }

    #[test]
    fn test_next_char_boundary_multibyte() {
        let s = "a\u{1F600}b"; // a + 4-byte emoji + b
        assert_eq!(next_char_boundary(s, 0), 1); // after a -> emoji start
        assert_eq!(next_char_boundary(s, 1), 5); // after emoji -> b
    }
}
