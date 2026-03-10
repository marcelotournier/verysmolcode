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
                    Span::styled("> ", Style::default().fg(USER_COLOR).bold()),
                    Span::styled(text.as_str(), Style::default().fg(USER_COLOR)),
                ]));
            }
            DisplayMessage::Assistant(text) => {
                // Word-wrap long responses
                for line in wrap_text(text, width.saturating_sub(2)) {
                    lines.push(Line::from(Span::styled(
                        line,
                        Style::default().fg(ASSISTANT_COLOR),
                    )));
                }
            }
            DisplayMessage::ToolCall(text) => {
                lines.push(Line::from(vec![
                    Span::styled("  > ", Style::default().fg(TOOL_COLOR)),
                    Span::styled(text.as_str(), Style::default().fg(TOOL_COLOR)),
                ]));
            }
            DisplayMessage::ToolResult(text) => {
                lines.push(Line::from(vec![
                    Span::styled("  < ", Style::default().fg(TOOL_RESULT_COLOR)),
                    Span::styled(text.as_str(), Style::default().fg(TOOL_RESULT_COLOR)),
                ]));
            }
            DisplayMessage::Status(text) => {
                lines.push(Line::from(Span::styled(
                    format!("  {}", text),
                    Style::default().fg(STATUS_COLOR).italic(),
                )));
            }
            DisplayMessage::Error(text) => {
                lines.push(Line::from(Span::styled(
                    format!("Error: {}", text),
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
        let desc_max = inner.width as usize - 15;
        let desc_display = if desc.len() > desc_max {
            format!("{}...", &desc[..desc_max.saturating_sub(3)])
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
        if line.len() <= width {
            lines.push(line.to_string());
        } else {
            // Simple word wrapping
            let mut current = String::new();
            for word in line.split_whitespace() {
                if current.is_empty() {
                    current = word.to_string();
                } else if current.len() + 1 + word.len() <= width {
                    current.push(' ');
                    current.push_str(word);
                } else {
                    lines.push(current);
                    current = word.to_string();
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
