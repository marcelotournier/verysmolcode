use crate::tui::app::{App, DisplayMessage};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

// Color scheme - comfortable blue tones for tmux
const BG_COLOR: Color = Color::Rgb(15, 17, 26);
const HEADER_BG: Color = Color::Rgb(25, 35, 60);
const INPUT_BG: Color = Color::Rgb(20, 25, 40);
const USER_COLOR: Color = Color::Rgb(130, 170, 255);
const ASSISTANT_COLOR: Color = Color::Rgb(200, 210, 230);
const TOOL_COLOR: Color = Color::Rgb(100, 180, 140);
const TOOL_RESULT_COLOR: Color = Color::Rgb(140, 150, 170);
const STATUS_COLOR: Color = Color::Rgb(180, 160, 100);
const ERROR_COLOR: Color = Color::Rgb(255, 120, 120);
const BORDER_COLOR: Color = Color::Rgb(60, 80, 120);
const ACCENT_COLOR: Color = Color::Rgb(80, 140, 255);

const SUGGESTION_BG: Color = Color::Rgb(30, 40, 65);
const SUGGESTION_HIGHLIGHT: Color = Color::Rgb(50, 70, 110);
const SUGGESTION_CMD_COLOR: Color = Color::Rgb(140, 190, 255);
const SUGGESTION_DESC_COLOR: Color = Color::Rgb(130, 140, 160);

pub fn draw(f: &mut Frame, app: &App) {
    let size = f.area();

    // Main layout: header, messages, input, status bar
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(1),    // Messages
            Constraint::Length(3), // Input
            Constraint::Length(1), // Status bar
        ])
        .split(size);

    draw_header(f, chunks[0], app);
    draw_messages(f, chunks[1], app);
    draw_input(f, chunks[2], app);
    draw_status_bar(f, chunks[3], app);

    // Draw command suggestion popup (overlay, drawn last)
    if !app.command_suggestions.is_empty() && !app.is_processing {
        draw_suggestions(f, chunks[2], app);
    }
}

fn draw_header(f: &mut Frame, area: Rect, app: &App) {
    let mode = if app.planning_mode { " [PLAN] " } else { "" };
    let title = if app.is_processing {
        format!(" \u{1FAD0} VerySmolCode{}  [{}] ", mode, app.model_name)
    } else {
        format!(" \u{1FAD0} VerySmolCode{} ", mode)
    };

    let header = Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT_COLOR))
        .style(Style::default().bg(HEADER_BG));

    let spinner = if app.is_processing {
        let frames = [
            "\u{2699}\u{FE0F} Working.",
            "\u{2699}\u{FE0F} Working..",
            "\u{2699}\u{FE0F} Working...",
            "\u{1F527} Working....",
            "\u{1F527} Working.....",
        ];
        let idx = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            / 300) as usize
            % frames.len();
        frames[idx].to_string()
    } else {
        "\u{2728} Ready".to_string()
    };

    let inner = header.inner(area);
    f.render_widget(header, area);

    let status_text = Paragraph::new(spinner)
        .style(Style::default().fg(ACCENT_COLOR))
        .alignment(Alignment::Center);
    f.render_widget(status_text, inner);
}

fn draw_messages(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::LEFT | Borders::RIGHT)
        .border_style(Style::default().fg(BORDER_COLOR))
        .style(Style::default().bg(BG_COLOR));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.messages.is_empty() {
        let welcome = Paragraph::new("\u{1F44B} Hey there! Type a message or /help to get started")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        f.render_widget(welcome, inner);
        return;
    }

    // Build text lines from messages
    let mut lines: Vec<Line> = Vec::new();
    let width = inner.width as usize;

    for msg in &app.messages {
        match msg {
            DisplayMessage::User(text) => {
                lines.push(Line::from(vec![
                    Span::styled("\u{1F464} ", Style::default().fg(USER_COLOR).bold()),
                    Span::styled(text.as_str(), Style::default().fg(USER_COLOR)),
                ]));
            }
            DisplayMessage::Assistant(text) => {
                // Word-wrap long responses
                let wrapped = wrap_text(text, width.saturating_sub(2));
                for (i, line) in wrapped.iter().enumerate() {
                    if i == 0 {
                        lines.push(Line::from(vec![
                            Span::styled("\u{1FAD0} ", Style::default().fg(ASSISTANT_COLOR)),
                            Span::styled(line.clone(), Style::default().fg(ASSISTANT_COLOR)),
                        ]));
                    } else {
                        lines.push(Line::from(Span::styled(
                            format!("  {}", line),
                            Style::default().fg(ASSISTANT_COLOR),
                        )));
                    }
                }
            }
            DisplayMessage::ToolCall(text) => {
                lines.push(Line::from(vec![
                    Span::styled("  \u{1F529} ", Style::default().fg(TOOL_COLOR)),
                    Span::styled(text.as_str(), Style::default().fg(TOOL_COLOR)),
                ]));
            }
            DisplayMessage::ToolResult(text) => {
                lines.push(Line::from(vec![
                    Span::styled("  \u{2705} ", Style::default().fg(TOOL_RESULT_COLOR)),
                    Span::styled(text.as_str(), Style::default().fg(TOOL_RESULT_COLOR)),
                ]));
            }
            DisplayMessage::Status(text) => {
                lines.push(Line::from(Span::styled(
                    format!("  \u{1F4AC} {}", text),
                    Style::default().fg(STATUS_COLOR).italic(),
                )));
            }
            DisplayMessage::Error(text) => {
                lines.push(Line::from(Span::styled(
                    format!("\u{26A0}\u{FE0F}  {}", text),
                    Style::default().fg(ERROR_COLOR).bold(),
                )));
            }
            DisplayMessage::ModelInfo(text) => {
                lines.push(Line::from(Span::styled(
                    text.as_str(),
                    Style::default().fg(ACCENT_COLOR),
                )));
            }
        }
        // Add spacing between messages
        lines.push(Line::from(""));
    }

    // Calculate scroll
    let total_lines = lines.len() as u16;
    let visible_lines = inner.height;
    let max_scroll = total_lines.saturating_sub(visible_lines);
    let scroll = if app.scroll_offset > 0 {
        max_scroll.saturating_sub(app.scroll_offset)
    } else {
        max_scroll
    };

    let paragraph = Paragraph::new(lines).scroll((scroll, 0));

    f.render_widget(paragraph, inner);
}

fn draw_input(f: &mut Frame, area: Rect, app: &App) {
    let label = if app.is_processing { " ... " } else { " > " };

    let block = Block::default()
        .title(label)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if app.is_processing {
            STATUS_COLOR
        } else {
            ACCENT_COLOR
        }))
        .style(Style::default().bg(INPUT_BG));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let display_text = if app.input.is_empty() && !app.is_processing {
        "Ask me anything or type / for commands...".to_string()
    } else {
        app.input.clone()
    };

    let style = if app.input.is_empty() && !app.is_processing {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default().fg(Color::White)
    };

    let input = Paragraph::new(display_text).style(style);
    f.render_widget(input, inner);

    // Show cursor (count chars before cursor_pos, not bytes)
    if !app.is_processing {
        let visual_pos = app.input[..app.cursor_pos].chars().count();
        f.set_cursor_position(Position::new(inner.x + visual_pos as u16, inner.y));
    }
}

fn draw_status_bar(f: &mut Frame, area: Rect, app: &App) {
    let left = if !app.rate_status.is_empty() {
        app.rate_status.clone()
    } else {
        format!("VerySmolCode v{}", env!("CARGO_PKG_VERSION"))
    };

    let right = if !app.status_line.is_empty() {
        format!("\u{1F4CA} {}", app.status_line)
    } else {
        "\u{1F4A1} Ctrl+C: quit | /help".to_string()
    };

    let width = area.width as usize;
    let padding = width.saturating_sub(left.len() + right.len());
    let status = format!("{}{:>pad$}", left, right, pad = padding + right.len());

    let bar = Paragraph::new(status).style(
        Style::default()
            .fg(Color::Rgb(160, 170, 190))
            .bg(Color::Rgb(30, 35, 50)),
    );

    f.render_widget(bar, area);
}

fn draw_suggestions(f: &mut Frame, input_area: Rect, app: &App) {
    let count = app.command_suggestions.len().min(10); // Max 10 visible
    if count == 0 {
        return;
    }

    let height = count as u16 + 2; // +2 for borders
    let width = 50u16.min(input_area.width);

    // Position popup above the input box
    let x = input_area.x + 1;
    let y = input_area.y.saturating_sub(height);

    let popup_area = Rect::new(x, y, width, height);

    // Clear background
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT_COLOR))
        .style(Style::default().bg(SUGGESTION_BG))
        .title(" Commands ")
        .title_alignment(Alignment::Left);

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    // Render each suggestion
    let mut lines: Vec<Line> = Vec::new();
    for (i, (cmd, desc)) in app.command_suggestions.iter().take(10).enumerate() {
        let is_selected = app.suggestion_index == Some(i);
        let bg = if is_selected {
            SUGGESTION_HIGHLIGHT
        } else {
            SUGGESTION_BG
        };

        let cmd_display = format!("{:<14}", cmd);
        let desc_max = (inner.width as usize).saturating_sub(15);
        let desc_display = if desc_max < 4 {
            String::new()
        } else if desc.chars().count() > desc_max {
            let truncated: String = desc.chars().take(desc_max - 3).collect();
            format!("{}...", truncated)
        } else {
            desc.to_string()
        };

        lines.push(Line::from(vec![
            Span::styled(
                cmd_display,
                Style::default().fg(SUGGESTION_CMD_COLOR).bg(bg).bold(),
            ),
            Span::styled(
                desc_display,
                Style::default().fg(SUGGESTION_DESC_COLOR).bg(bg),
            ),
        ]));
    }

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, inner);
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let width = width.max(20);
    let mut lines = Vec::new();
    for line in text.lines() {
        if line.chars().count() <= width {
            lines.push(line.to_string());
        } else {
            // Simple word wrapping using char counts for correct multi-byte handling
            let mut current = String::new();
            let mut current_chars = 0usize;
            for word in line.split_whitespace() {
                let word_chars = word.chars().count();
                if current.is_empty() {
                    current = word.to_string();
                    current_chars = word_chars;
                } else if current_chars + 1 + word_chars <= width {
                    current.push(' ');
                    current.push_str(word);
                    current_chars += 1 + word_chars;
                } else {
                    lines.push(current);
                    current = word.to_string();
                    current_chars = word_chars;
                }
            }
            if !current.is_empty() {
                lines.push(current);
            }
        }
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_text_short_line() {
        let result = wrap_text("hello world", 80);
        assert_eq!(result, vec!["hello world"]);
    }

    #[test]
    fn test_wrap_text_exact_width() {
        let result = wrap_text("hello", 5);
        // width.max(20) clamps to 20, so "hello" (5 chars) fits
        assert_eq!(result, vec!["hello"]);
    }

    #[test]
    fn test_wrap_text_wraps_long_line() {
        let result = wrap_text("the quick brown fox jumps over the lazy dog", 20);
        assert!(result.len() > 1);
        for line in &result {
            assert!(line.len() <= 20, "Line too long: '{}'", line);
        }
    }

    #[test]
    fn test_wrap_text_preserves_newlines() {
        let result = wrap_text("line one\nline two\nline three", 80);
        assert_eq!(result, vec!["line one", "line two", "line three"]);
    }

    #[test]
    fn test_wrap_text_empty_input() {
        let result = wrap_text("", 80);
        assert_eq!(result, vec![""]);
    }

    #[test]
    fn test_wrap_text_long_word_no_spaces() {
        // A single long word with no break points — should still appear (not be lost)
        let long = "a".repeat(50);
        let result = wrap_text(&long, 30);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], long);
    }

    #[test]
    fn test_wrap_text_min_width_clamped() {
        // width=5 gets clamped to 20
        let result = wrap_text("short text here", 5);
        assert_eq!(result, vec!["short text here"]);
    }

    #[test]
    fn test_wrap_text_multiple_spaces_short() {
        // Short line (len <= width) is preserved as-is, including spaces
        let result = wrap_text("hello    world", 80);
        assert_eq!(result, vec!["hello    world"]);
    }

    #[test]
    fn test_wrap_text_multiple_spaces_wrapped() {
        // When wrapping occurs, split_whitespace collapses spaces
        let result = wrap_text(
            "hello    world    this    is    long    enough    to    wrap",
            20,
        );
        assert!(result.len() > 1);
        // No double spaces in wrapped output
        for line in &result {
            assert!(!line.contains("  "), "Double space in: '{}'", line);
        }
    }

    #[test]
    fn test_wrap_text_multiline_with_wrapping() {
        let result = wrap_text("short\nthis is a longer line that should wrap around", 25);
        assert_eq!(result[0], "short");
        assert!(result.len() >= 3);
    }

    #[test]
    fn test_wrap_text_multibyte_chars() {
        // Emojis are multi-byte but each counts as 1 char for wrapping
        let input = "\u{1F600} hello \u{1F600} world \u{1F600} test \u{1F600} more \u{1F600} words \u{1F600} here";
        let result = wrap_text(input, 20);
        // Should wrap based on char count, not byte count
        for line in &result {
            assert!(
                line.chars().count() <= 20,
                "Line too long by char count: '{}' = {} chars",
                line,
                line.chars().count()
            );
        }
    }
}
