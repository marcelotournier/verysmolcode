//! `grep` tool — Rust port of pi's grep.ts.
//!
//! Pi shells out to `ripgrep` (with auto-download). To keep the VSC binary
//! deployable on RPi3 with no extra binary fetches, we keep the in-process
//! parallel walker but expose pi's option surface:
//! - `pattern`, `path`, `glob`, `ignore_case`, `literal`, `context`, `limit`
//!
//! Output format mirrors pi's: `path:line: text`, with optional `-N-` context
//! lines. Long lines (> 500 chars) are truncated to keep results readable.

use rayon::prelude::*;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::tools::find::glob_match;
use crate::tools::path_utils::resolve_to_cwd;
use crate::tools::truncate::{
    format_size, truncate_head, truncate_line, TruncationOptions, DEFAULT_MAX_BYTES,
    GREP_MAX_LINE_LENGTH,
};

const DEFAULT_LIMIT: usize = 100;

const SKIP_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "__pycache__",
    "venv",
    ".venv",
    ".tox",
    ".mypy_cache",
    ".pytest_cache",
];

pub fn grep(args: &Value) -> Value {
    let pattern = match args.get("pattern").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => return json!({"error": "Missing 'pattern' argument"}),
    };
    let path_arg = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
    let glob = args
        .get("glob")
        .or_else(|| args.get("include"))
        .and_then(|v| v.as_str());
    let ignore_case = args
        .get("ignore_case")
        .or_else(|| args.get("ignoreCase"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let literal = args
        .get("literal")
        .and_then(|v| v.as_bool())
        .unwrap_or(true); // VSC default keeps backwards-compat behavior
    let context = args
        .get("context")
        .and_then(|v| v.as_u64())
        .map(|n| n as usize)
        .unwrap_or(0);
    let limit = args
        .get("limit")
        .or_else(|| args.get("max_results"))
        .and_then(|v| v.as_u64())
        .map(|n| n as usize)
        .unwrap_or(DEFAULT_LIMIT);

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let search_path = resolve_to_cwd(path_arg, &cwd);

    let mut files = Vec::new();
    walk_collect(&search_path, glob, &mut files);

    let count = AtomicUsize::new(0);
    let pattern_owned = pattern.to_string();

    // Collect all matches (file_rel, line_no, line_text) in parallel
    let raw_matches: Vec<(String, usize, String)> = files
        .par_iter()
        .flat_map(|file| {
            if count.load(Ordering::SeqCst) >= limit {
                return Vec::new();
            }
            let mut found = Vec::new();
            let content = match fs::read_to_string(file) {
                Ok(s) => s,
                Err(_) => return found,
            };
            let rel = file
                .strip_prefix(&search_path)
                .unwrap_or(file)
                .to_string_lossy()
                .replace('\\', "/");
            for (i, line) in content.lines().enumerate() {
                if count.load(Ordering::SeqCst) >= limit {
                    break;
                }
                if line_matches(line, &pattern_owned, ignore_case, literal) {
                    count.fetch_add(1, Ordering::SeqCst);
                    found.push((rel.clone(), i + 1, line.to_string()));
                }
            }
            found
        })
        .collect();

    let trimmed: Vec<(String, usize, String)> = raw_matches.into_iter().take(limit).collect();
    let limit_reached = trimmed.len() >= limit;

    if trimmed.is_empty() {
        return json!({
            "pattern": pattern,
            "path": search_path.display().to_string(),
            "matches": [],
            "total_matches": 0,
            "content": "No matches found"
        });
    }

    // Build pi-style output and matches[] array (kept for back-compat with TUI)
    let mut output_lines: Vec<String> = Vec::new();
    let mut matches_arr: Vec<Value> = Vec::with_capacity(trimmed.len());
    let mut lines_were_truncated = false;
    let file_cache_key: Option<&str> = None;
    let _ = file_cache_key;

    for (rel, line_no, line_text) in &trimmed {
        if context > 0 {
            let abs = search_path.join(rel);
            if let Ok(content) = fs::read_to_string(&abs) {
                let lines: Vec<&str> = content.lines().collect();
                let start = line_no.saturating_sub(context).max(1);
                let end = (line_no + context).min(lines.len());
                for cur in start..=end {
                    let lt = lines.get(cur - 1).copied().unwrap_or("");
                    let (truncated, was) = truncate_line(lt, GREP_MAX_LINE_LENGTH);
                    if was {
                        lines_were_truncated = true;
                    }
                    if cur == *line_no {
                        output_lines.push(format!("{}:{}: {}", rel, cur, truncated));
                    } else {
                        output_lines.push(format!("{}-{}- {}", rel, cur, truncated));
                    }
                }
            }
        } else {
            let (truncated, was) = truncate_line(line_text, GREP_MAX_LINE_LENGTH);
            if was {
                lines_were_truncated = true;
            }
            output_lines.push(format!("{}:{}: {}", rel, line_no, truncated));
        }
        matches_arr.push(json!({
            "file": rel,
            "line": line_no,
            "content": line_text.trim()
        }));
    }

    let raw_output = output_lines.join("\n");
    let truncation = truncate_head(
        &raw_output,
        TruncationOptions {
            max_lines: usize::MAX,
            max_bytes: DEFAULT_MAX_BYTES,
        },
    );
    let mut output = truncation.content.clone();
    let mut notices = Vec::new();
    if limit_reached {
        notices.push(format!(
            "{} matches limit reached. Use limit={} for more, or refine pattern",
            limit,
            limit * 2
        ));
    }
    if truncation.truncated {
        notices.push(format!("{} limit reached", format_size(DEFAULT_MAX_BYTES)));
    }
    if lines_were_truncated {
        notices.push(format!(
            "Some lines truncated to {} chars. Use read tool to see full lines",
            GREP_MAX_LINE_LENGTH
        ));
    }
    if !notices.is_empty() {
        output.push_str(&format!("\n\n[{}]", notices.join(". ")));
    }

    json!({
        "pattern": pattern,
        "path": search_path.display().to_string(),
        "matches": matches_arr,
        "total_matches": matches_arr.len(),
        "content": output
    })
}

fn line_matches(line: &str, pattern: &str, ignore_case: bool, literal: bool) -> bool {
    // No regex crate — pi uses ripgrep (regex by default). To keep VSC's
    // dependency footprint flat, we offer literal substring matching plus a
    // tiny case-insensitive switch. Documented as "literal-only" in the schema.
    let _ = literal;
    if ignore_case {
        line.to_lowercase().contains(&pattern.to_lowercase())
    } else {
        line.contains(pattern)
    }
}

fn walk_collect(dir: &Path, glob: Option<&str>, out: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if SKIP_DIRS.contains(&name.as_str()) || name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            walk_collect(&path, glob, out);
        } else if path.is_file() {
            if let Some(g) = glob {
                let target = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                if !glob_match(g, &target) {
                    continue;
                }
            }
            if !is_likely_binary(&path) {
                out.push(path);
            }
        }
    }
}

fn is_likely_binary(path: &Path) -> bool {
    let binary_exts = [
        "png", "jpg", "jpeg", "gif", "bmp", "ico", "pdf", "zip", "tar", "gz", "bz2", "xz", "7z",
        "exe", "dll", "so", "dylib", "o", "a", "wasm", "class", "pyc", "pyo",
    ];
    if let Some(ext) = path.extension() {
        let ext = ext.to_string_lossy().to_lowercase();
        if binary_exts.contains(&ext.as_str()) {
            return true;
        }
    }
    if let Ok(file) = fs::File::open(path) {
        use std::io::Read;
        let mut buf = [0u8; 512];
        let mut reader = std::io::BufReader::new(file);
        if let Ok(n) = reader.read(&mut buf) {
            return buf[..n].contains(&0);
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("vsc_grep_{}", name));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn test_grep_missing_pattern() {
        let r = grep(&json!({}));
        assert!(r.get("error").is_some());
    }

    #[test]
    fn test_grep_simple() {
        let d = tmp("simple");
        fs::write(d.join("a.txt"), "hello world\nbye world\n").unwrap();
        let r = grep(&json!({"pattern": "hello", "path": d.to_str().unwrap()}));
        let m = r["matches"].as_array().unwrap();
        assert_eq!(m.len(), 1);
        assert!(r["content"].as_str().unwrap().contains("a.txt:1:"));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn test_grep_glob_filter() {
        let d = tmp("globflt");
        fs::write(d.join("a.rs"), "marker").unwrap();
        fs::write(d.join("b.txt"), "marker").unwrap();
        let r = grep(&json!({
            "pattern": "marker",
            "path": d.to_str().unwrap(),
            "glob": "*.rs"
        }));
        assert_eq!(r["matches"].as_array().unwrap().len(), 1);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn test_grep_case_insensitive() {
        let d = tmp("ci");
        fs::write(d.join("a.txt"), "Hello").unwrap();
        let r = grep(&json!({
            "pattern": "hello",
            "path": d.to_str().unwrap(),
            "ignore_case": true
        }));
        assert_eq!(r["matches"].as_array().unwrap().len(), 1);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn test_grep_context() {
        let d = tmp("ctx");
        fs::write(d.join("a.txt"), "before\nMATCH\nafter\n").unwrap();
        let r = grep(&json!({
            "pattern": "MATCH",
            "path": d.to_str().unwrap(),
            "context": 1
        }));
        let content = r["content"].as_str().unwrap();
        assert!(content.contains("a.txt-1- before"));
        assert!(content.contains("a.txt:2: MATCH"));
        assert!(content.contains("a.txt-3- after"));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn test_grep_limit() {
        let d = tmp("limit");
        let body = (0..50)
            .map(|i| format!("hello {}", i))
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(d.join("many.txt"), body).unwrap();
        let r = grep(&json!({"pattern": "hello", "path": d.to_str().unwrap(), "limit": 5}));
        let m = r["matches"].as_array().unwrap();
        assert!(m.len() <= 5);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn test_grep_no_match() {
        let d = tmp("nomatch");
        fs::write(d.join("a.txt"), "abc").unwrap();
        let r = grep(&json!({"pattern": "xyz", "path": d.to_str().unwrap()}));
        assert_eq!(r["total_matches"], 0);
        assert_eq!(r["content"], "No matches found");
        let _ = fs::remove_dir_all(&d);
    }
}
