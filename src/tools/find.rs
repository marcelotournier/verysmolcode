//! `find` tool — Rust port of pi's find.ts.
//!
//! Pi shells out to `fd`. VSC stays self-contained (no shell deps) and uses an
//! in-process glob matcher (`*`, `**`, `?`). It still respects the same
//! ignored directories as the previous VSC implementation (`.git`,
//! `node_modules`, `target`, etc.) and uses rayon to walk in parallel.

use rayon::prelude::*;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

use crate::tools::path_utils::resolve_to_cwd;
use crate::tools::truncate::{
    format_size, truncate_head, TruncationOptions, DEFAULT_MAX_BYTES,
};

const DEFAULT_LIMIT: usize = 1000;

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

pub fn find(args: &Value) -> Value {
    let pattern = match args.get("pattern").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => return json!({"error": "Missing 'pattern' argument"}),
    };
    let path_arg = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
    let limit = args
        .get("limit")
        .and_then(|v| v.as_u64())
        .map(|n| n as usize)
        .unwrap_or(DEFAULT_LIMIT);

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let search_path = resolve_to_cwd(path_arg, &cwd);
    if !search_path.exists() {
        return json!({"error": format!("Path not found: {}", search_path.display())});
    }

    let mut all_files = Vec::new();
    walk_collect(&search_path, &mut all_files);

    // Match either against basename (pattern without `/`) or full relative path
    let pattern_has_slash = pattern.contains('/');
    let matches: Vec<String> = all_files
        .par_iter()
        .filter_map(|p| {
            let rel = p
                .strip_prefix(&search_path)
                .unwrap_or(p)
                .to_string_lossy()
                .to_string()
                .replace('\\', "/");
            let target = if pattern_has_slash {
                rel.clone()
            } else {
                p.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default()
            };
            if glob_match(pattern, &target) {
                Some(rel)
            } else {
                None
            }
        })
        .collect();

    let limit_reached = matches.len() >= limit;
    let trimmed: Vec<String> = matches.into_iter().take(limit).collect();
    let total = trimmed.len();

    if total == 0 {
        return json!({
            "pattern": pattern,
            "files": Vec::<String>::new(),
            "total": 0,
            "content": "No files found matching pattern"
        });
    }

    let raw_output = trimmed.join("\n");
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
            "{} results limit reached. Use limit={} for more, or refine pattern",
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
        "pattern": pattern,
        "files": trimmed,
        "total": total,
        "content": output
    })
}

fn walk_collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if SKIP_DIRS.contains(&name.as_str()) {
            continue;
        }
        if path.is_dir() {
            walk_collect(&path, out);
        } else {
            out.push(path);
        }
    }
}

/// Tiny glob matcher supporting:
/// - `*`  matches any sequence of characters except `/`
/// - `**` matches any sequence including `/`
/// - `?`  matches a single character (not `/`)
/// - Any other character matches literally
///
/// Implementation: recursive descent. Patterns in this codebase are short,
/// so the simple approach is plenty fast and avoids backtracking bugs that
/// crop up when nesting `**/...*`.
pub fn glob_match(pattern: &str, text: &str) -> bool {
    let pat: Vec<char> = pattern.chars().collect();
    let txt: Vec<char> = text.chars().collect();
    g(&pat, 0, &txt, 0)
}

fn g(p: &[char], pi: usize, t: &[char], ti: usize) -> bool {
    if pi >= p.len() {
        return ti >= t.len();
    }
    // ** — match zero or more characters including '/'
    if p[pi] == '*' && pi + 1 < p.len() && p[pi + 1] == '*' {
        let mut next_pi = pi + 2;
        if next_pi < p.len() && p[next_pi] == '/' {
            next_pi += 1;
        }
        // Try consuming 0..=t.len()-ti chars
        for k in ti..=t.len() {
            if g(p, next_pi, t, k) {
                return true;
            }
        }
        return false;
    }
    // Single * — match zero or more characters but not '/'
    if p[pi] == '*' {
        let mut k = ti;
        loop {
            if g(p, pi + 1, t, k) {
                return true;
            }
            if k >= t.len() || t[k] == '/' {
                return false;
            }
            k += 1;
        }
    }
    if ti >= t.len() {
        return false;
    }
    // ? — single non-slash character
    if p[pi] == '?' {
        return t[ti] != '/' && g(p, pi + 1, t, ti + 1);
    }
    if p[pi] == t[ti] {
        return g(p, pi + 1, t, ti + 1);
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("vsc_find_{}", name));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn test_find_missing_pattern() {
        let r = find(&json!({}));
        assert!(r.get("error").is_some());
    }

    #[test]
    fn test_find_basename_glob() {
        let d = tmp("basename");
        fs::write(d.join("a.rs"), "x").unwrap();
        fs::write(d.join("b.txt"), "x").unwrap();
        let r = find(&json!({"pattern": "*.rs", "path": d.to_str().unwrap()}));
        let files = r["files"].as_array().unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].as_str().unwrap().ends_with("a.rs"));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn test_find_path_glob() {
        let d = tmp("pathglob");
        fs::create_dir(d.join("src")).unwrap();
        fs::write(d.join("src/main.rs"), "x").unwrap();
        fs::write(d.join("README.md"), "x").unwrap();
        let r = find(&json!({"pattern": "src/*.rs", "path": d.to_str().unwrap()}));
        let files = r["files"].as_array().unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].as_str().unwrap().contains("src/main.rs"));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn test_find_no_match() {
        let d = tmp("nomatch");
        fs::write(d.join("only.txt"), "x").unwrap();
        let r = find(&json!({"pattern": "*.py", "path": d.to_str().unwrap()}));
        assert_eq!(r["total"], 0);
        assert_eq!(r["content"], "No files found matching pattern");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn test_glob_match_star() {
        assert!(glob_match("*.rs", "main.rs"));
        assert!(!glob_match("*.rs", "main.py"));
    }

    #[test]
    fn test_glob_match_question() {
        assert!(glob_match("a?c", "abc"));
        assert!(!glob_match("a?c", "ac"));
    }

    #[test]
    fn test_glob_match_doublestar() {
        assert!(glob_match("**/*.rs", "src/lib.rs"));
        assert!(glob_match("**/*.rs", "deep/nested/path/file.rs"));
        assert!(!glob_match("**/*.rs", "file.py"));
    }

    #[test]
    fn test_glob_match_literal() {
        assert!(glob_match("Cargo.toml", "Cargo.toml"));
        assert!(!glob_match("Cargo.toml", "Cargo.lock"));
    }

    #[test]
    fn test_walk_skips_target_and_node_modules() {
        let d = tmp("skip");
        fs::create_dir(d.join("target")).unwrap();
        fs::write(d.join("target/inside.rs"), "skip").unwrap();
        fs::write(d.join("real.rs"), "keep").unwrap();
        let mut all = Vec::new();
        walk_collect(&d, &mut all);
        assert_eq!(all.len(), 1);
        let _ = fs::remove_dir_all(&d);
    }
}
