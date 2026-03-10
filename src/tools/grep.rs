use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

/// Search for a pattern in files
pub fn grep_search(args: &Value) -> Value {
    let pattern = match args.get("pattern").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => return json!({"error": "Missing 'pattern' argument"}),
    };
    let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
    let include = args.get("include").and_then(|v| v.as_str());
    let max_results: usize = args.get("max_results").and_then(|v| v.as_u64()).unwrap_or(50) as usize;

    let search_path = if Path::new(path).is_absolute() {
        PathBuf::from(path)
    } else {
        std::env::current_dir().unwrap_or_default().join(path)
    };

    let mut results: Vec<Value> = Vec::new();
    search_recursive(&search_path, pattern, include, max_results, &mut results);

    json!({
        "pattern": pattern,
        "path": search_path.display().to_string(),
        "matches": results,
        "total_matches": results.len()
    })
}

fn search_recursive(
    dir: &Path,
    pattern: &str,
    include: Option<&str>,
    max_results: usize,
    results: &mut Vec<Value>,
) {
    if results.len() >= max_results {
        return;
    }

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        if results.len() >= max_results {
            return;
        }

        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        // Skip hidden dirs and common non-code directories
        if name.starts_with('.') || name == "node_modules" || name == "target"
            || name == "__pycache__" || name == "venv" || name == ".git"
        {
            continue;
        }

        if path.is_dir() {
            search_recursive(&path, pattern, include, max_results, results);
        } else if path.is_file() {
            // Check include pattern (simple glob)
            if let Some(inc) = include {
                let inc = inc.trim_start_matches('*');
                if !name.ends_with(inc) {
                    continue;
                }
            }

            // Skip binary files (check first bytes)
            if is_likely_binary(&path) {
                continue;
            }

            if let Ok(content) = fs::read_to_string(&path) {
                let pattern_lower = pattern.to_lowercase();
                for (line_num, line) in content.lines().enumerate() {
                    if results.len() >= max_results {
                        return;
                    }
                    if line.to_lowercase().contains(&pattern_lower) {
                        results.push(json!({
                            "file": path.display().to_string(),
                            "line": line_num + 1,
                            "content": line.trim()
                        }));
                    }
                }
            }
        }
    }
}

fn is_likely_binary(path: &Path) -> bool {
    let binary_exts = ["png", "jpg", "jpeg", "gif", "bmp", "ico", "pdf",
                       "zip", "tar", "gz", "bz2", "xz", "7z",
                       "exe", "dll", "so", "dylib", "o", "a",
                       "wasm", "class", "pyc", "pyo"];

    if let Some(ext) = path.extension() {
        let ext = ext.to_string_lossy().to_lowercase();
        if binary_exts.contains(&ext.as_str()) {
            return true;
        }
    }

    // Check first few bytes for null bytes
    if let Ok(bytes) = fs::read(path) {
        let check_len = bytes.len().min(512);
        return bytes[..check_len].contains(&0);
    }

    false
}

/// Find files matching a glob pattern
pub fn find_files(args: &Value) -> Value {
    let pattern = match args.get("pattern").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => return json!({"error": "Missing 'pattern' argument"}),
    };
    let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
    let max_results: usize = args.get("max_results").and_then(|v| v.as_u64()).unwrap_or(100) as usize;

    let search_path = if Path::new(path).is_absolute() {
        PathBuf::from(path)
    } else {
        std::env::current_dir().unwrap_or_default().join(path)
    };

    let mut results: Vec<String> = Vec::new();
    find_recursive(&search_path, pattern, max_results, &mut results);

    json!({
        "pattern": pattern,
        "files": results,
        "total": results.len()
    })
}

fn find_recursive(dir: &Path, pattern: &str, max: usize, results: &mut Vec<String>) {
    if results.len() >= max {
        return;
    }

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        if results.len() >= max {
            return;
        }

        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if name.starts_with('.') || name == "node_modules" || name == "target" {
            continue;
        }

        if path.is_dir() {
            find_recursive(&path, pattern, max, results);
        } else {
            // Simple glob matching
            let pat = pattern.trim_start_matches('*');
            if name.contains(pat) || name.ends_with(pat) {
                results.push(path.display().to_string());
            }
        }
    }
}
