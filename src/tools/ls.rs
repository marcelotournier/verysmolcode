//! `ls` tool — port of pi's ls.ts.
//!
//! Returns directory entries sorted case-insensitively, with `/` suffix on
//! subdirectories. Output is a single newline-joined string (matches pi's
//! result shape; the TUI summarizer accepts both this and the legacy entry
//! array format).

use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

use crate::tools::path_utils::resolve_to_cwd;
use crate::tools::truncate::{
    format_size, truncate_head, TruncationOptions, DEFAULT_MAX_BYTES,
};

const DEFAULT_LIMIT: usize = 500;

pub fn ls(args: &Value) -> Value {
    let path_arg = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
    let limit = args
        .get("limit")
        .and_then(|v| v.as_u64())
        .map(|n| n as usize)
        .unwrap_or(DEFAULT_LIMIT);

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let absolute = resolve_to_cwd(path_arg, &cwd);

    if !absolute.exists() {
        return json!({"error": format!("Path not found: {}", absolute.display())});
    }
    if !absolute.is_dir() {
        return json!({"error": format!("Not a directory: {}", absolute.display())});
    }

    let mut entries: Vec<String> = match fs::read_dir(&absolute) {
        Ok(it) => it
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect(),
        Err(e) => {
            return json!({"error": format!("Cannot read directory: {}", e)})
        }
    };
    entries.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));

    let mut entry_limit_reached = false;
    let mut results: Vec<String> = Vec::new();
    let mut entries_payload: Vec<Value> = Vec::new();
    for entry in &entries {
        if results.len() >= limit {
            entry_limit_reached = true;
            break;
        }
        let full = absolute.join(entry);
        let is_dir = full.is_dir();
        let size = full.metadata().map(|m| m.len()).unwrap_or(0);
        let suffix = if is_dir { "/" } else { "" };
        results.push(format!("{}{}", entry, suffix));
        entries_payload.push(json!({
            "name": entry,
            "is_dir": is_dir,
            "size": size,
        }));
    }

    if results.is_empty() {
        return json!({
            "path": absolute.display().to_string(),
            "content": "(empty directory)",
            "entries": entries_payload
        });
    }

    let raw_output = results.join("\n");
    let truncation = truncate_head(
        &raw_output,
        TruncationOptions {
            max_lines: usize::MAX,
            max_bytes: DEFAULT_MAX_BYTES,
        },
    );

    let mut output = truncation.content.clone();
    let mut notices = Vec::new();
    if entry_limit_reached {
        notices.push(format!(
            "{} entries limit reached. Use limit={} for more",
            limit,
            limit * 2
        ));
    }
    if truncation.truncated {
        notices.push(format!("{} limit reached", format_size(DEFAULT_MAX_BYTES)));
    }
    if !notices.is_empty() {
        output.push_str(&format!("\n\n[{}]", notices.join(". ")));
    }

    json!({
        "path": absolute.display().to_string(),
        "content": output,
        "entries": entries_payload
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("vsc_ls_{}", name));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_ls_nonexistent() {
        let r = ls(&json!({"path": "/nonexistent/vsc/path/abc"}));
        assert!(r.get("error").is_some());
    }

    #[test]
    fn test_ls_lists_and_marks_dirs() {
        let d = tmp("basic");
        fs::create_dir(d.join("sub")).unwrap();
        fs::write(d.join("a.txt"), "x").unwrap();
        let r = ls(&json!({"path": d.to_str().unwrap()}));
        let content = r["content"].as_str().unwrap();
        assert!(content.contains("a.txt"));
        assert!(content.contains("sub/"));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn test_ls_empty_dir() {
        let d = tmp("empty");
        let r = ls(&json!({"path": d.to_str().unwrap()}));
        assert_eq!(r["content"], "(empty directory)");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn test_ls_limit() {
        let d = tmp("limit");
        for i in 0..10 {
            fs::write(d.join(format!("f{}.txt", i)), "x").unwrap();
        }
        let r = ls(&json!({"path": d.to_str().unwrap(), "limit": 3}));
        let content = r["content"].as_str().unwrap();
        assert!(content.contains("limit reached"));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn test_ls_case_insensitive_sort() {
        let d = tmp("sort");
        fs::write(d.join("Bear.txt"), "b").unwrap();
        fs::write(d.join("apple.txt"), "a").unwrap();
        let r = ls(&json!({"path": d.to_str().unwrap()}));
        let content = r["content"].as_str().unwrap();
        let apple = content.find("apple").unwrap();
        let bear = content.find("Bear").unwrap();
        assert!(apple < bear);
        let _ = fs::remove_dir_all(&d);
    }
}
