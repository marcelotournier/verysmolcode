//! `read` tool — port of pi's read.ts.
//!
//! Reads a text file (or image) with optional `offset` (1-indexed) / `limit`.
//! Truncation follows pi's two-limit rule: 2000 lines OR 50KB, whichever first.
//! Returns a continuation hint when truncation occurs so the model knows how to
//! page through large files.

use base64::Engine;
use serde_json::{json, Value};
use std::fs;

use crate::tools::path_utils::resolve_read_path;
use crate::tools::truncate::{
    format_size, truncate_head, TruncationOptions, DEFAULT_MAX_BYTES, DEFAULT_MAX_LINES,
};

const IMAGE_EXTS: &[(&str, &str)] = &[
    ("png", "image/png"),
    ("jpg", "image/jpeg"),
    ("jpeg", "image/jpeg"),
    ("gif", "image/gif"),
    ("webp", "image/webp"),
    ("bmp", "image/bmp"),
];

fn detect_image_mime(path: &std::path::Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?.to_lowercase();
    IMAGE_EXTS
        .iter()
        .find_map(|(e, m)| if *e == ext { Some(*m) } else { None })
}

pub fn read(args: &Value) -> Value {
    let path_arg = match args.get("path").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => return json!({"error": "Missing 'path' argument"}),
    };
    let offset = args
        .get("offset")
        .and_then(|v| v.as_u64())
        .map(|n| n as usize);
    let limit = args
        .get("limit")
        .and_then(|v| v.as_u64())
        .map(|n| n as usize);

    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let absolute = resolve_read_path(path_arg, &cwd);

    if let Some(mime) = detect_image_mime(&absolute) {
        return read_image_inline(&absolute, mime);
    }

    let raw = match fs::read_to_string(&absolute) {
        Ok(s) => s,
        Err(e) => {
            return json!({
                "error": format!("Failed to read {}: {}", absolute.display(), e)
            })
        }
    };

    let all_lines: Vec<&str> = raw.split('\n').collect();
    let total_file_lines = all_lines.len();

    let start_line = match offset {
        Some(o) if o >= 1 => o - 1,
        _ => 0,
    };
    if start_line >= total_file_lines {
        return json!({
            "error": format!(
                "Offset {} is beyond end of file ({} lines total)",
                offset.unwrap_or(0),
                total_file_lines
            )
        });
    }

    let (selected_text, user_limited_lines) = match limit {
        Some(l) => {
            let end = (start_line + l).min(total_file_lines);
            (all_lines[start_line..end].join("\n"), Some(end - start_line))
        }
        None => (all_lines[start_line..].join("\n"), None),
    };

    let truncation = truncate_head(&selected_text, TruncationOptions::default());
    let mut output_text;
    let mut details = serde_json::Map::new();

    if truncation.first_line_exceeds_limit {
        let first_line_size = format_size(all_lines[start_line].len());
        output_text = format!(
            "[Line {} is {}, exceeds {} limit. Use bash: sed -n '{}p' {} | head -c {}]",
            start_line + 1,
            first_line_size,
            format_size(DEFAULT_MAX_BYTES),
            start_line + 1,
            path_arg,
            DEFAULT_MAX_BYTES,
        );
        details.insert("first_line_exceeds_limit".into(), json!(true));
    } else if truncation.truncated {
        let end_line_display = start_line + truncation.output_lines;
        let next_offset = end_line_display + 1;
        output_text = truncation.content.clone();
        let suffix = match truncation.truncated_by {
            Some(crate::tools::truncate::TruncatedBy::Lines) => format!(
                "\n\n[Showing lines {}-{} of {}. Use offset={} to continue.]",
                start_line + 1,
                end_line_display,
                total_file_lines,
                next_offset
            ),
            _ => format!(
                "\n\n[Showing lines {}-{} of {} ({} limit). Use offset={} to continue.]",
                start_line + 1,
                end_line_display,
                total_file_lines,
                format_size(DEFAULT_MAX_BYTES),
                next_offset
            ),
        };
        output_text.push_str(&suffix);
        details.insert("truncated".into(), json!(true));
        details.insert("max_lines".into(), json!(DEFAULT_MAX_LINES));
        details.insert("max_bytes".into(), json!(DEFAULT_MAX_BYTES));
    } else if let Some(lim) = user_limited_lines {
        if start_line + lim < total_file_lines {
            let remaining = total_file_lines - (start_line + lim);
            let next_offset = start_line + lim + 1;
            output_text = format!(
                "{}\n\n[{} more lines in file. Use offset={} to continue.]",
                truncation.content, remaining, next_offset
            );
        } else {
            output_text = truncation.content;
        }
    } else {
        output_text = truncation.content;
    }

    let mut out = serde_json::Map::new();
    out.insert("content".into(), json!(output_text));
    out.insert("path".into(), json!(absolute.display().to_string()));
    if !details.is_empty() {
        out.insert("details".into(), Value::Object(details));
        // Backward-compat flag for the TUI summarizer:
        out.insert("truncated".into(), json!(true));
        out.insert("total_bytes".into(), json!(raw.len()));
    }
    Value::Object(out)
}

fn read_image_inline(path: &std::path::Path, mime: &str) -> Value {
    match fs::metadata(path) {
        Ok(meta) if meta.len() > 10_000_000 => {
            return json!({"error": "Image too large (max 10MB)"})
        }
        Err(e) => return json!({"error": format!("Cannot read {}: {}", path.display(), e)}),
        _ => {}
    }
    match fs::read(path) {
        Ok(data) => {
            let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
            json!({
                "inline_data": {
                    "mime_type": mime,
                    "data": b64
                },
                "path": path.display().to_string(),
                "size_bytes": data.len()
            })
        }
        Err(e) => json!({"error": format!("Failed to read {}: {}", path.display(), e)}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("vsc_read_{}", name))
    }

    #[test]
    fn test_read_missing_path() {
        let r = read(&json!({}));
        assert!(r.get("error").is_some());
    }

    #[test]
    fn test_read_simple_file() {
        let p = tmp_path("simple.txt");
        fs::write(&p, "hello world").unwrap();
        let r = read(&json!({"path": p.to_str().unwrap()}));
        assert_eq!(r["content"], "hello world");
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn test_read_offset_limit() {
        let p = tmp_path("range.txt");
        fs::write(&p, "a\nb\nc\nd\ne").unwrap();
        let r = read(&json!({"path": p.to_str().unwrap(), "offset": 2, "limit": 2}));
        let content = r["content"].as_str().unwrap();
        assert!(content.starts_with("b\nc"));
        assert!(content.contains("more lines in file"));
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn test_read_offset_beyond_eof() {
        let p = tmp_path("beyond.txt");
        fs::write(&p, "one line").unwrap();
        let r = read(&json!({"path": p.to_str().unwrap(), "offset": 100}));
        assert!(r["error"].as_str().unwrap().contains("beyond"));
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn test_read_truncates_long_file() {
        let p = tmp_path("long.txt");
        let body = (0..3000).map(|i| format!("line {}", i)).collect::<Vec<_>>().join("\n");
        fs::write(&p, &body).unwrap();
        let r = read(&json!({"path": p.to_str().unwrap()}));
        let content = r["content"].as_str().unwrap();
        assert!(content.contains("Use offset="));
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn test_read_image_returns_inline_data() {
        let p = tmp_path("pic.png");
        fs::write(&p, b"\x89PNG\r\n\x1a\n").unwrap();
        let r = read(&json!({"path": p.to_str().unwrap()}));
        assert_eq!(r["inline_data"]["mime_type"], "image/png");
        let _ = fs::remove_file(&p);
    }
}
