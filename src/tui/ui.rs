use crate::tui::app::{App, DisplayMessage};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use std::sync::Mutex;
use unicode_width::UnicodeWidthStr;

/// Cache for rendered message lines to avoid re-rendering markdown on every frame.
/// Only re-renders when message count or terminal width changes.
struct RenderCache {
    lines: Vec<Line<'static>>,
    msg_count: usize,
    width: u16,
}

static RENDER_CACHE: Mutex<Option<RenderCache>> = Mutex::new(None);

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

    // Draw suggestion popups (overlay, drawn last)
    if !app.command_suggestions.is_empty() && !app.is_processing {
        draw_suggestions(f, chunks[2], app);
    } else if !app.file_suggestions.is_empty() && !app.is_processing {
        draw_file_suggestions(f, chunks[2], app);
    }

    // Draw todo popup (Ctrl+T overlay)
    if app.todo_visible {
        draw_todo_popup(f, size, app);
    }
}

fn draw_header(f: &mut Frame, area: Rect, app: &App) {
    let mut badges = String::new();
    if app.planning_mode {
        badges.push_str(" [PLAN]");
    }
    if app.search_grounding {
        badges.push_str(" [WEB]");
    }
    let title = if app.is_processing {
        format!(" \u{1FAD0} VerySmolCode{}  [{}] ", badges, app.model_name)
    } else {
        format!(" \u{1FAD0} VerySmolCode{} ", badges)
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
        draw_welcome(f, inner);
        return;
    }

    let width = inner.width as usize;

    // Use cached lines if message count and width haven't changed
    let lines = {
        let mut cache = RENDER_CACHE.lock().unwrap();
        let needs_rebuild = match cache.as_ref() {
            Some(c) => c.msg_count != app.messages.len() || c.width != inner.width,
            None => true,
        };

        if needs_rebuild {
            let rendered = build_message_lines(&app.messages, width);
            *cache = Some(RenderCache {
                lines: rendered.clone(),
                msg_count: app.messages.len(),
                width: inner.width,
            });
            rendered
        } else {
            cache.as_ref().unwrap().lines.clone()
        }
    };

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

    // Show scroll indicator when not at bottom
    if app.scroll_offset > 0 && total_lines > visible_lines {
        let indicator = format!(" [{}/{} lines] ", scroll + visible_lines, total_lines);
        let x = inner.x + inner.width.saturating_sub(indicator.len() as u16 + 1);
        let y = inner.y;
        if x > inner.x {
            f.render_widget(
                Paragraph::new(indicator).style(
                    Style::default()
                        .fg(Color::Rgb(200, 180, 100))
                        .bg(Color::Rgb(40, 45, 60)),
                ),
                Rect::new(x, y, inner.width.saturating_sub(x - inner.x), 1),
            );
        }
    }
}

/// Build all rendered lines from messages (expensive, cached)
fn build_message_lines(messages: &[DisplayMessage], width: usize) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();

    for msg in messages {
        match msg {
            DisplayMessage::User(text) => {
                lines.push(Line::from(vec![
                    Span::styled(
                        "\u{1F464} ".to_string(),
                        Style::default().fg(USER_COLOR).bold(),
                    ),
                    Span::styled(text.clone(), Style::default().fg(USER_COLOR)),
                ]));
            }
            DisplayMessage::Assistant(text) => {
                let md_lines = render_markdown(text, width.saturating_sub(2));
                for (i, line) in md_lines.iter().enumerate() {
                    if i == 0 {
                        let mut spans = vec![Span::styled(
                            "\u{1FAD0} ".to_string(),
                            Style::default().fg(ASSISTANT_COLOR),
                        )];
                        spans.extend(line.spans.iter().cloned());
                        lines.push(Line::from(spans));
                    } else {
                        let mut spans = vec![Span::raw("  ".to_string())];
                        spans.extend(line.spans.iter().cloned());
                        lines.push(Line::from(spans));
                    }
                }
            }
            DisplayMessage::ToolCall(text) => {
                lines.push(Line::from(vec![
                    Span::styled("  \u{1F529} ".to_string(), Style::default().fg(TOOL_COLOR)),
                    Span::styled(text.clone(), Style::default().fg(TOOL_COLOR)),
                ]));
            }
            DisplayMessage::ToolResult(text) => {
                let wrapped = wrap_text(text, width.saturating_sub(6));
                for (i, line) in wrapped.iter().enumerate() {
                    if i == 0 {
                        lines.push(Line::from(vec![
                            Span::styled(
                                "  \u{2705} ".to_string(),
                                Style::default().fg(TOOL_RESULT_COLOR),
                            ),
                            Span::styled(line.clone(), Style::default().fg(TOOL_RESULT_COLOR)),
                        ]));
                    } else {
                        lines.push(Line::from(Span::styled(
                            format!("      {}", line),
                            Style::default().fg(TOOL_RESULT_COLOR),
                        )));
                    }
                }
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
                    text.clone(),
                    Style::default().fg(ACCENT_COLOR),
                )));
            }
        }
        lines.push(Line::from("".to_string()));
    }

    lines
}

fn draw_input(f: &mut Frame, area: Rect, app: &App) {
    // Reverse search mode
    if app.search_mode {
        let block = Block::default()
            .title(" (reverse-i-search) ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(200, 150, 100)))
            .style(Style::default().bg(INPUT_BG));
        let inner = block.inner(area);
        f.render_widget(block, area);

        let display = if let Some(ref matched) = app.search_match {
            format!("{}: {}", app.search_query, matched)
        } else if app.search_query.is_empty() {
            "Type to search history...".to_string()
        } else {
            format!("{}: (no match)", app.search_query)
        };
        let style = if app.search_match.is_some() {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let input = Paragraph::new(display).style(style);
        f.render_widget(input, inner);

        let cursor_x = inner.x + UnicodeWidthStr::width(app.search_query.as_str()) as u16;
        let cursor_y = inner.y;
        f.set_cursor_position((cursor_x, cursor_y));
        return;
    }

    let is_multiline = app.input.contains('\n');
    let label = if app.is_processing {
        " ... "
    } else if is_multiline {
        " > (multi-line, \\ + Enter) "
    } else {
        " > "
    };

    let block = Block::default()
        .title(label)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if app.is_processing {
            STATUS_COLOR
        } else if is_multiline {
            Color::Rgb(130, 180, 120) // green tint for multi-line
        } else {
            ACCENT_COLOR
        }))
        .style(Style::default().bg(INPUT_BG));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // For multi-line input, show only the last line with a line count prefix
    let (display_text, cursor_offset) = if app.input.is_empty() && !app.is_processing {
        (
            "Ask me anything or type / for commands...".to_string(),
            0usize,
        )
    } else if is_multiline {
        let line_count = app.input.lines().count();
        // Show last line with prefix
        let last_line = app.input.lines().last().unwrap_or("");
        let prefix = format!("[{}L] ", line_count);
        (format!("{}{}", prefix, last_line), prefix.len())
    } else {
        (app.input.clone(), 0)
    };

    let style = if app.input.is_empty() && !app.is_processing {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default().fg(Color::White)
    };

    let input = Paragraph::new(display_text).style(style);
    f.render_widget(input, inner);

    // Show cursor
    if !app.is_processing {
        let cursor_text = if is_multiline {
            // For multi-line, cursor pos is relative to last line
            let last_line_start = app.input.rfind('\n').map(|p| p + 1).unwrap_or(0);
            let pos_in_line = app.cursor_pos.saturating_sub(last_line_start);
            &app.input[last_line_start
                ..last_line_start + pos_in_line.min(app.input.len() - last_line_start)]
        } else {
            &app.input[..app.cursor_pos]
        };
        let visual_pos = cursor_text.chars().count() + cursor_offset;
        f.set_cursor_position(Position::new(inner.x + visual_pos as u16, inner.y));
    }
}

fn draw_welcome(f: &mut Frame, area: Rect) {
    let logo_color = Color::Rgb(100, 160, 255);
    let dim_color = Color::Rgb(80, 90, 120);
    let hint_color = Color::Rgb(140, 150, 180);
    let version = env!("CARGO_PKG_VERSION");

    let lines = vec![
        Line::from(""),
        Line::from(""),
        Line::from(vec![
            Span::styled(" v", Style::default().fg(logo_color).bold()),
            Span::styled("s", Style::default().fg(Color::Rgb(120, 175, 255)).bold()),
            Span::styled("c", Style::default().fg(Color::Rgb(140, 190, 255)).bold()),
        ]),
        Line::from(Span::styled(
            format!("VerySmolCode v{}", version),
            Style::default().fg(logo_color).bold(),
        )),
        Line::from(Span::styled(
            "Your tiny coding buddy",
            Style::default().fg(dim_color).italic(),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Type a message  ", Style::default().fg(hint_color)),
            Span::styled("|", Style::default().fg(dim_color)),
            Span::styled("  /help  ", Style::default().fg(hint_color)),
            Span::styled("|", Style::default().fg(dim_color)),
            Span::styled("  /  for commands", Style::default().fg(hint_color)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Ctrl+C/D ", Style::default().fg(dim_color)),
            Span::styled("quit", Style::default().fg(hint_color)),
            Span::styled("  |  ", Style::default().fg(dim_color)),
            Span::styled("Ctrl+L ", Style::default().fg(dim_color)),
            Span::styled("clear", Style::default().fg(hint_color)),
            Span::styled("  |  ", Style::default().fg(dim_color)),
            Span::raw(""),
            Span::styled("up/down ", Style::default().fg(dim_color)),
            Span::styled("history", Style::default().fg(hint_color)),
        ]),
    ];

    let welcome = Paragraph::new(lines).alignment(Alignment::Center);
    f.render_widget(welcome, area);
}

fn draw_status_bar(f: &mut Frame, area: Rect, app: &App) {
    let left = if !app.todo_summary.is_empty() {
        // Show current task in status bar
        let max_len = (area.width as usize).saturating_sub(40);
        if app.todo_summary.len() > max_len && max_len > 3 {
            format!("{}...", &app.todo_summary[..max_len - 3])
        } else {
            app.todo_summary.clone()
        }
    } else if !app.rate_status.is_empty() {
        app.rate_status.clone()
    } else {
        format!("VerySmolCode v{}", env!("CARGO_PKG_VERSION"))
    };

    let total_tokens = app.total_input_tokens + app.total_output_tokens;
    let right = if total_tokens > 0 {
        // Show context bar: [||||      ] 12.5K/24K
        let threshold = app.conversation_tokens.max(1) as f64;
        let auto_compact = app.auto_compact_threshold;
        let ratio = (app.conversation_tokens as f64) / (auto_compact as f64);
        let bar_width = 8;
        let filled = ((ratio * bar_width as f64).round() as usize).min(bar_width);
        let bar_char = if ratio > 0.8 { '!' } else { '|' };
        let bar = format!(
            "[{}{}]",
            std::iter::repeat_n(bar_char, filled).collect::<String>(),
            std::iter::repeat_n(' ', bar_width - filled).collect::<String>(),
        );
        let color_hint = if ratio > 0.8 { " \u{26A0}" } else { "" };
        format!(
            "{} {:.1}K/{:.0}K ctx{}  In:{} Out:{}",
            bar,
            threshold / 1000.0,
            auto_compact as f64 / 1000.0,
            color_hint,
            format_token_count(app.total_input_tokens),
            format_token_count(app.total_output_tokens),
        )
    } else if !app.status_line.is_empty() {
        format!("\u{1F4CA} {}", app.status_line)
    } else {
        "\u{1F4A1} Ctrl+C/D: quit | /help".to_string()
    };

    let width = area.width as usize;
    let left_width = UnicodeWidthStr::width(left.as_str());
    let right_width = UnicodeWidthStr::width(right.as_str());
    let padding = width.saturating_sub(left_width + right_width);
    let status = format!("{}{:>pad$}", left, right, pad = padding + right.len());

    let bar = Paragraph::new(status).style(
        Style::default()
            .fg(Color::Rgb(160, 170, 190))
            .bg(Color::Rgb(30, 35, 50)),
    );

    f.render_widget(bar, area);
}

fn format_token_count(count: u64) -> String {
    if count >= 1_000_000 {
        format!("{:.1}M", count as f64 / 1_000_000.0)
    } else if count >= 1_000 {
        format!("{:.1}K", count as f64 / 1_000.0)
    } else {
        format!("{}", count)
    }
}

fn draw_suggestions(f: &mut Frame, input_area: Rect, app: &App) {
    let total = app.command_suggestions.len();
    if total == 0 {
        return;
    }

    let max_visible = 10;
    let visible = total.min(max_visible);
    let height = visible as u16 + 2; // +2 for borders
    let width = 50u16.min(input_area.width);

    // Position popup above the input box
    let x = input_area.x + 1;
    let y = input_area.y.saturating_sub(height);

    let popup_area = Rect::new(x, y, width, height);

    // Clear background
    f.render_widget(Clear, popup_area);

    // Compute scroll offset so selected item is always visible
    let scroll_offset = if let Some(idx) = app.suggestion_index {
        if idx >= max_visible {
            idx - max_visible + 1
        } else {
            0
        }
    } else {
        0
    };

    let title = if total > max_visible {
        format!(" Commands ({}/{}) ", scroll_offset + 1, total)
    } else {
        " Commands ".to_string()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT_COLOR))
        .style(Style::default().bg(SUGGESTION_BG))
        .title(title)
        .title_alignment(Alignment::Left);

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    // Render each suggestion with scroll offset
    let mut lines: Vec<Line> = Vec::new();
    for (i, (cmd, desc)) in app
        .command_suggestions
        .iter()
        .enumerate()
        .skip(scroll_offset)
        .take(max_visible)
    {
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

fn draw_file_suggestions(f: &mut Frame, input_area: Rect, app: &App) {
    let count = app.file_suggestions.len().min(10);
    if count == 0 {
        return;
    }

    let height = count as u16 + 2;
    let width = 60u16.min(input_area.width);
    let x = input_area.x + 1;
    let y = input_area.y.saturating_sub(height);

    let popup_area = Rect::new(x, y, width, height);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(100, 200, 150)))
        .style(Style::default().bg(SUGGESTION_BG))
        .title(" @ Files ")
        .title_alignment(Alignment::Left);

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let mut lines: Vec<Line> = Vec::new();
    for (i, path) in app.file_suggestions.iter().take(10).enumerate() {
        let is_selected = app.file_suggestion_index == Some(i);
        let bg = if is_selected {
            SUGGESTION_HIGHLIGHT
        } else {
            SUGGESTION_BG
        };

        let max_len = inner.width as usize;
        let display = if path.len() > max_len {
            format!("...{}", &path[path.len() - (max_len - 3)..])
        } else {
            path.to_string()
        };

        lines.push(Line::from(Span::styled(
            display,
            Style::default().fg(Color::Rgb(130, 200, 170)).bg(bg).bold(),
        )));
    }

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, inner);
}

fn draw_todo_popup(f: &mut Frame, area: Rect, app: &App) {
    let display = if app.todo_display.is_empty() {
        "No tasks yet. The agent creates tasks when working on complex requests."
    } else {
        &app.todo_display
    };

    let lines: Vec<&str> = display.lines().collect();
    let height = (lines.len() as u16 + 2).min(area.height.saturating_sub(4)); // +2 for borders
    let width = 60u16.min(area.width.saturating_sub(4));

    // Center the popup
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;

    let popup_area = Rect::new(x, y, width, height);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(180, 160, 100)))
        .style(Style::default().bg(Color::Rgb(25, 30, 45)))
        .title(" Tasks (Ctrl+T to close) ")
        .title_alignment(Alignment::Center);

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let text_lines: Vec<Line> = lines
        .iter()
        .take(inner.height as usize)
        .map(|line| {
            Line::from(Span::styled(
                *line,
                Style::default().fg(Color::Rgb(200, 210, 230)),
            ))
        })
        .collect();

    let paragraph = Paragraph::new(text_lines);
    f.render_widget(paragraph, inner);
}

const CODE_BG: Color = Color::Rgb(30, 35, 55);
const CODE_FG: Color = Color::Rgb(180, 200, 160);
const HEADER_FG: Color = Color::Rgb(140, 190, 255);
const BOLD_FG: Color = Color::Rgb(220, 225, 240);

/// Basic markdown rendering: code blocks, headers, bold, bullets
fn render_markdown<'a>(text: &str, width: usize) -> Vec<Line<'a>> {
    let mut result: Vec<Line> = Vec::new();
    let mut in_code_block = false;

    for raw_line in text.lines() {
        // Code block toggle
        if raw_line.trim_start().starts_with("```") {
            in_code_block = !in_code_block;
            if in_code_block {
                // Show language hint if present
                let lang = raw_line.trim_start().strip_prefix("```").unwrap_or("");
                if !lang.is_empty() {
                    result.push(Line::from(Span::styled(
                        format!("  --- {} ---", lang),
                        Style::default().fg(CODE_FG).italic(),
                    )));
                }
            }
            continue;
        }

        if in_code_block {
            // Code lines: different colors, no wrapping
            let display = if raw_line.len() > width {
                format!("{}...", &raw_line[..width.saturating_sub(3)])
            } else {
                raw_line.to_string()
            };
            result.push(Line::from(Span::styled(
                display,
                Style::default().fg(CODE_FG).bg(CODE_BG),
            )));
            continue;
        }

        // Headers (check ### before ## before #)
        let header_text = raw_line
            .strip_prefix("### ")
            .or_else(|| raw_line.strip_prefix("## "))
            .or_else(|| raw_line.strip_prefix("# "));
        if let Some(header_text) = header_text {
            let wrapped = wrap_text(header_text, width);
            for line in wrapped {
                result.push(Line::from(Span::styled(
                    line,
                    Style::default().fg(HEADER_FG).bold(),
                )));
            }
            continue;
        }

        // Bullet points
        let trimmed = raw_line.trim_start();
        if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            let indent = raw_line.len() - trimmed.len();
            let bullet_indent = " ".repeat(indent);
            let bullet_text = &trimmed[2..];
            let wrapped = wrap_text(bullet_text, width.saturating_sub(indent + 4));
            for (i, line) in wrapped.iter().enumerate() {
                if i == 0 {
                    result.push(Line::from(Span::styled(
                        format!("{}\u{2022} {}", bullet_indent, line),
                        Style::default().fg(ASSISTANT_COLOR),
                    )));
                } else {
                    result.push(Line::from(Span::styled(
                        format!("{}  {}", bullet_indent, line),
                        Style::default().fg(ASSISTANT_COLOR),
                    )));
                }
            }
            continue;
        }

        // Horizontal rule
        if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            let rule: String = "\u{2500}".repeat(width.min(60));
            result.push(Line::from(Span::styled(
                rule,
                Style::default().fg(Color::Rgb(60, 70, 90)),
            )));
            continue;
        }

        // Numbered lists (e.g., "1. item", "2. item")
        if let Some(rest) = trimmed
            .split_once(". ")
            .filter(|(num, _)| num.len() <= 3 && num.chars().all(|c| c.is_ascii_digit()))
        {
            let indent = raw_line.len() - trimmed.len();
            let list_indent = " ".repeat(indent);
            let wrapped = wrap_text(rest.1, width.saturating_sub(indent + 4));
            for (i, line) in wrapped.iter().enumerate() {
                if i == 0 {
                    result.push(Line::from(Span::styled(
                        format!("{}{}. {}", list_indent, rest.0, line),
                        Style::default().fg(ASSISTANT_COLOR),
                    )));
                } else {
                    result.push(Line::from(Span::styled(
                        format!("{}   {}", list_indent, line),
                        Style::default().fg(ASSISTANT_COLOR),
                    )));
                }
            }
            continue;
        }

        // Regular text with inline code and bold
        let wrapped = wrap_text(raw_line, width);
        for line in wrapped {
            if line.contains('`') || line.contains("**") {
                result.push(render_inline_markdown(&line));
            } else {
                result.push(Line::from(Span::styled(
                    line,
                    Style::default().fg(ASSISTANT_COLOR),
                )));
            }
        }
    }

    if result.is_empty() {
        result.push(Line::from(Span::styled(
            text.to_string(),
            Style::default().fg(ASSISTANT_COLOR),
        )));
    }

    result
}

/// Render inline markdown: `code` and **bold**
fn render_inline_markdown<'a>(text: &str) -> Line<'a> {
    let mut spans: Vec<Span> = Vec::new();
    let mut chars = text.chars().peekable();
    let mut current = String::new();

    while let Some(c) = chars.next() {
        if c == '`' {
            // Flush current text
            if !current.is_empty() {
                spans.push(Span::styled(
                    std::mem::take(&mut current),
                    Style::default().fg(ASSISTANT_COLOR),
                ));
            }
            // Collect code content
            let mut code = String::new();
            for ch in chars.by_ref() {
                if ch == '`' {
                    break;
                }
                code.push(ch);
            }
            spans.push(Span::styled(code, Style::default().fg(CODE_FG).bg(CODE_BG)));
        } else if c == '*' && chars.peek() == Some(&'*') {
            chars.next(); // consume second *
                          // Flush current text
            if !current.is_empty() {
                spans.push(Span::styled(
                    std::mem::take(&mut current),
                    Style::default().fg(ASSISTANT_COLOR),
                ));
            }
            // Collect bold content
            let mut bold = String::new();
            while let Some(ch) = chars.next() {
                if ch == '*' && chars.peek() == Some(&'*') {
                    chars.next(); // consume closing **
                    break;
                }
                bold.push(ch);
            }
            spans.push(Span::styled(bold, Style::default().fg(BOLD_FG).bold()));
        } else {
            current.push(c);
        }
    }

    // Flush remaining
    if !current.is_empty() {
        spans.push(Span::styled(current, Style::default().fg(ASSISTANT_COLOR)));
    }

    Line::from(spans)
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
