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

/// Find the start position of the previous word (for Ctrl+W delete-word-backward).
/// Skips trailing whitespace, then backs up through non-whitespace chars.
fn word_start(input: &str, cursor: usize) -> usize {
    let mut pos = cursor;
    // Skip trailing whitespace
    while pos > 0 {
        let prev = prev_char_boundary(input, pos);
        if input[prev..pos].trim().is_empty() {
            pos = prev;
        } else {
            break;
        }
    }
    // Back up through the word
    while pos > 0 {
        let prev = prev_char_boundary(input, pos);
        if !input[prev..pos].trim().is_empty() {
            pos = prev;
        } else {
            break;
        }
    }
    pos
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
            // If suggestion popup is open and one is selected, fill it in first
            if !app.command_suggestions.is_empty() && app.suggestion_index.is_some() {
                app.select_suggestion();
            } else {
                app.submit_input();
            }
        }
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                match c {
                    'a' => app.cursor_pos = 0,
                    'e' => app.cursor_pos = app.input.len(),
                    'u' => {
                        app.input.drain(..app.cursor_pos);
                        app.cursor_pos = 0;
                        app.update_suggestions();
                    }
                    'k' => {
                        app.input.truncate(app.cursor_pos);
                        app.update_suggestions();
                    }
                    'w' => {
                        // Delete word backward (char-boundary aware)
                        let pos = word_start(&app.input, app.cursor_pos);
                        app.input.drain(pos..app.cursor_pos);
                        app.cursor_pos = pos;
                    }
                    _ => {}
                }
            } else {
                app.input.insert(app.cursor_pos, c);
                app.cursor_pos += c.len_utf8();
                app.update_suggestions();
            }
        }
        KeyCode::Backspace => {
            if app.cursor_pos > 0 {
                let prev = prev_char_boundary(&app.input, app.cursor_pos);
                app.input.drain(prev..app.cursor_pos);
                app.cursor_pos = prev;
                app.update_suggestions();
            }
        }
        KeyCode::Delete => {
            if app.cursor_pos < app.input.len() {
                let next = next_char_boundary(&app.input, app.cursor_pos);
                app.input.drain(app.cursor_pos..next);
                app.update_suggestions();
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
            if !app.command_suggestions.is_empty() {
                // Navigate suggestions up
                let len = app.command_suggestions.len();
                app.suggestion_index = Some(match app.suggestion_index {
                    Some(i) if i > 0 => i - 1,
                    Some(_) => len - 1,
                    None => len - 1,
                });
            } else {
                app.history_up();
            }
        }
        KeyCode::Down => {
            if !app.command_suggestions.is_empty() {
                // Navigate suggestions down
                let len = app.command_suggestions.len();
                app.suggestion_index = Some(match app.suggestion_index {
                    Some(i) if i + 1 < len => i + 1,
                    Some(_) => 0,
                    None => 0,
                });
            } else {
                app.history_down();
            }
        }
        KeyCode::PageUp => {
            app.scroll_up();
        }
        KeyCode::PageDown => {
            app.scroll_down();
        }
        KeyCode::Tab => {
            // Select from suggestion popup, or autocomplete
            if !app.command_suggestions.is_empty() {
                app.select_suggestion();
            } else if app.input.starts_with('/') {
                let completions = crate::tui::commands::autocomplete(&app.input);
                if completions.len() == 1 {
                    app.input = completions[0].clone();
                    app.cursor_pos = app.input.len();
                }
            }
        }
        KeyCode::Esc => {
            // Dismiss suggestion popup
            app.command_suggestions.clear();
            app.suggestion_index = None;
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

    // -- word_start tests (Ctrl+W behavior) --

    #[test]
    fn test_word_start_basic() {
        let s = "hello world";
        // Cursor at end: should delete "world"
        assert_eq!(word_start(s, 11), 6);
    }

    #[test]
    fn test_word_start_middle_of_word() {
        let s = "hello world";
        // Cursor in middle of "world": should delete to start of "world"
        assert_eq!(word_start(s, 8), 6);
    }

    #[test]
    fn test_word_start_at_space() {
        let s = "hello world";
        // Cursor right after space: should skip space and delete "hello"
        assert_eq!(word_start(s, 6), 0);
    }

    #[test]
    fn test_word_start_multiple_spaces() {
        let s = "one   two";
        // Cursor at end: should skip to start of "two"
        assert_eq!(word_start(s, 9), 6);
    }

    #[test]
    fn test_word_start_beginning() {
        let s = "hello";
        // Cursor at beginning: no change
        assert_eq!(word_start(s, 0), 0);
    }

    #[test]
    fn test_word_start_single_word() {
        let s = "hello";
        // Cursor at end: delete entire word
        assert_eq!(word_start(s, 5), 0);
    }

    #[test]
    fn test_word_start_three_words() {
        let s = "one two three";
        assert_eq!(word_start(s, 13), 8); // delete "three"
        assert_eq!(word_start(s, 7), 4); // delete "two"
        assert_eq!(word_start(s, 3), 0); // delete "one"
    }

    #[test]
    fn test_word_start_slash_command() {
        let s = "/help something";
        assert_eq!(word_start(s, 15), 6); // delete "something"
        assert_eq!(word_start(s, 5), 0); // delete "/help"
    }
}
