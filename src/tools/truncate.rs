//! Shared truncation utilities for tool outputs (Rust port of pi's truncate.ts).
//!
//! Truncation is based on two independent limits — whichever is hit first wins:
//! - Line limit (default: 2000)
//! - Byte limit (default: 50KB)
//!
//! Never returns partial lines (except in `truncate_tail`'s edge case when a
//! single line is larger than `max_bytes`).

pub const DEFAULT_MAX_LINES: usize = 2000;
pub const DEFAULT_MAX_BYTES: usize = 50 * 1024;
pub const GREP_MAX_LINE_LENGTH: usize = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TruncatedBy {
    Lines,
    Bytes,
}

#[derive(Debug, Clone)]
pub struct TruncationResult {
    pub content: String,
    pub truncated: bool,
    pub truncated_by: Option<TruncatedBy>,
    pub total_lines: usize,
    pub total_bytes: usize,
    pub output_lines: usize,
    pub output_bytes: usize,
    pub last_line_partial: bool,
    pub first_line_exceeds_limit: bool,
    pub max_lines: usize,
    pub max_bytes: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct TruncationOptions {
    pub max_lines: usize,
    pub max_bytes: usize,
}

impl Default for TruncationOptions {
    fn default() -> Self {
        Self {
            max_lines: DEFAULT_MAX_LINES,
            max_bytes: DEFAULT_MAX_BYTES,
        }
    }
}

pub fn format_size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{}B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1}KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1}MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

fn split_lines(content: &str) -> Vec<&str> {
    content.split('\n').collect()
}

/// Truncate content from the head (keep first N lines/bytes).
/// Suitable for file reads where you want to see the beginning.
pub fn truncate_head(content: &str, opts: TruncationOptions) -> TruncationResult {
    let max_lines = opts.max_lines;
    let max_bytes = opts.max_bytes;
    let total_bytes = content.len();
    let lines = split_lines(content);
    let total_lines = lines.len();

    if total_lines <= max_lines && total_bytes <= max_bytes {
        return TruncationResult {
            content: content.to_string(),
            truncated: false,
            truncated_by: None,
            total_lines,
            total_bytes,
            output_lines: total_lines,
            output_bytes: total_bytes,
            last_line_partial: false,
            first_line_exceeds_limit: false,
            max_lines,
            max_bytes,
        };
    }

    let first_line_bytes = lines.first().map(|l| l.len()).unwrap_or(0);
    if first_line_bytes > max_bytes {
        return TruncationResult {
            content: String::new(),
            truncated: true,
            truncated_by: Some(TruncatedBy::Bytes),
            total_lines,
            total_bytes,
            output_lines: 0,
            output_bytes: 0,
            last_line_partial: false,
            first_line_exceeds_limit: true,
            max_lines,
            max_bytes,
        };
    }

    let mut output_lines_arr: Vec<&str> = Vec::new();
    let mut output_bytes_count: usize = 0;
    let mut truncated_by = TruncatedBy::Lines;

    for (i, line) in lines.iter().take(max_lines).enumerate() {
        let line_bytes = line.len() + if i > 0 { 1 } else { 0 };
        if output_bytes_count + line_bytes > max_bytes {
            truncated_by = TruncatedBy::Bytes;
            break;
        }
        output_lines_arr.push(line);
        output_bytes_count += line_bytes;
    }

    if output_lines_arr.len() >= max_lines && output_bytes_count <= max_bytes {
        truncated_by = TruncatedBy::Lines;
    }

    let output_content = output_lines_arr.join("\n");
    let final_output_bytes = output_content.len();

    TruncationResult {
        content: output_content,
        truncated: true,
        truncated_by: Some(truncated_by),
        total_lines,
        total_bytes,
        output_lines: output_lines_arr.len(),
        output_bytes: final_output_bytes,
        last_line_partial: false,
        first_line_exceeds_limit: false,
        max_lines,
        max_bytes,
    }
}

/// Truncate content from the tail (keep last N lines/bytes).
/// Suitable for bash output where the tail (errors, results) matters most.
pub fn truncate_tail(content: &str, opts: TruncationOptions) -> TruncationResult {
    let max_lines = opts.max_lines;
    let max_bytes = opts.max_bytes;
    let total_bytes = content.len();
    let lines = split_lines(content);
    let total_lines = lines.len();

    if total_lines <= max_lines && total_bytes <= max_bytes {
        return TruncationResult {
            content: content.to_string(),
            truncated: false,
            truncated_by: None,
            total_lines,
            total_bytes,
            output_lines: total_lines,
            output_bytes: total_bytes,
            last_line_partial: false,
            first_line_exceeds_limit: false,
            max_lines,
            max_bytes,
        };
    }

    let mut output_lines_arr: Vec<String> = Vec::new();
    let mut output_bytes_count: usize = 0;
    let mut truncated_by = TruncatedBy::Lines;
    let mut last_line_partial = false;

    for (count_so_far, line) in lines.iter().rev().enumerate() {
        if output_lines_arr.len() >= max_lines {
            break;
        }
        let line_bytes = line.len() + if !output_lines_arr.is_empty() { 1 } else { 0 };
        if output_bytes_count + line_bytes > max_bytes {
            truncated_by = TruncatedBy::Bytes;
            if output_lines_arr.is_empty() {
                let truncated_line = truncate_string_to_bytes_from_end(line, max_bytes);
                let trim_len = truncated_line.len();
                output_lines_arr.push(truncated_line);
                output_bytes_count = trim_len;
                last_line_partial = true;
            }
            let _ = count_so_far;
            break;
        }
        output_lines_arr.push((*line).to_string());
        output_bytes_count += line_bytes;
    }

    output_lines_arr.reverse();
    if output_lines_arr.len() >= max_lines && output_bytes_count <= max_bytes {
        truncated_by = TruncatedBy::Lines;
    }

    let output_content = output_lines_arr.join("\n");
    let final_output_bytes = output_content.len();

    TruncationResult {
        content: output_content,
        truncated: true,
        truncated_by: Some(truncated_by),
        total_lines,
        total_bytes,
        output_lines: output_lines_arr.len(),
        output_bytes: final_output_bytes,
        last_line_partial,
        first_line_exceeds_limit: false,
        max_lines,
        max_bytes,
    }
}

/// Truncate a UTF-8 string from the end so it fits within `max_bytes`.
/// Returns the suffix; never breaks a multi-byte character.
fn truncate_string_to_bytes_from_end(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_string();
    }
    let mut start = s.len().saturating_sub(max_bytes);
    while start < s.len() && !s.is_char_boundary(start) {
        start += 1;
    }
    s[start..].to_string()
}

/// Truncate a single line to a max char count, appending a `... [truncated]` suffix.
pub fn truncate_line(line: &str, max_chars: usize) -> (String, bool) {
    if line.chars().count() <= max_chars {
        return (line.to_string(), false);
    }
    let cut: String = line.chars().take(max_chars).collect();
    (format!("{}... [truncated]", cut), true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts(max_lines: usize, max_bytes: usize) -> TruncationOptions {
        TruncationOptions {
            max_lines,
            max_bytes,
        }
    }

    #[test]
    fn test_truncate_head_no_truncation() {
        let r = truncate_head("hello\nworld", TruncationOptions::default());
        assert!(!r.truncated);
        assert_eq!(r.content, "hello\nworld");
        assert_eq!(r.total_lines, 2);
    }

    #[test]
    fn test_truncate_head_lines() {
        let content = "a\nb\nc\nd\ne";
        let r = truncate_head(content, opts(3, 1000));
        assert!(r.truncated);
        assert_eq!(r.truncated_by, Some(TruncatedBy::Lines));
        assert_eq!(r.content, "a\nb\nc");
    }

    #[test]
    fn test_truncate_head_bytes() {
        let content = "aaaa\nbbbb\ncccc";
        let r = truncate_head(content, opts(100, 6));
        assert!(r.truncated);
        assert_eq!(r.truncated_by, Some(TruncatedBy::Bytes));
        assert_eq!(r.content, "aaaa");
    }

    #[test]
    fn test_truncate_head_first_line_exceeds() {
        let content = "this is a very long single line";
        let r = truncate_head(content, opts(100, 5));
        assert!(r.truncated);
        assert!(r.first_line_exceeds_limit);
        assert_eq!(r.content, "");
    }

    #[test]
    fn test_truncate_tail_no_truncation() {
        let r = truncate_tail("a\nb", TruncationOptions::default());
        assert!(!r.truncated);
    }

    #[test]
    fn test_truncate_tail_lines() {
        let content = "a\nb\nc\nd\ne";
        let r = truncate_tail(content, opts(2, 1000));
        assert!(r.truncated);
        assert_eq!(r.content, "d\ne");
    }

    #[test]
    fn test_truncate_tail_bytes_partial_last() {
        let content = "this is one big line";
        let r = truncate_tail(content, opts(100, 5));
        assert!(r.truncated);
        assert!(r.last_line_partial);
        assert_eq!(r.content.len(), 5);
    }

    #[test]
    fn test_truncate_tail_utf8_boundary() {
        // 😀 = 4 bytes, ensure we don't slice mid-char from the end
        let content = "abc😀def";
        let r = truncate_tail(content, opts(100, 4));
        assert!(r.truncated);
        // Either drops the emoji or starts on a valid boundary
        assert!(r.content.is_char_boundary(0));
    }

    #[test]
    fn test_truncate_line_short() {
        let (s, t) = truncate_line("hello", 10);
        assert_eq!(s, "hello");
        assert!(!t);
    }

    #[test]
    fn test_truncate_line_long() {
        let (s, t) = truncate_line("aaaaaaaaaaaaaaa", 5);
        assert!(s.starts_with("aaaaa"));
        assert!(s.contains("[truncated]"));
        assert!(t);
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(100), "100B");
        assert_eq!(format_size(2048), "2.0KB");
        assert_eq!(format_size(2_097_152), "2.0MB");
    }
}
