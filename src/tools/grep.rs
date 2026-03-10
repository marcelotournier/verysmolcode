use rayon::prelude::*;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Search for a pattern in files (parallelized with rayon)
pub fn grep_search(args: &Value) -> Value {
    let pattern = match args.get("pattern").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => return json!({"error": "Missing 'pattern' argument"}),
    };
    let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
    let include = args.get("include").and_then(|v| v.as_str());
    let max_results: usize = args
        .get("max_results")
        .and_then(|v| v.as_u64())
        .unwrap_or(50) as usize;

    let search_path = if Path::new(path).is_absolute() {
        PathBuf::from(path)
    } else {
        std::env::current_dir().unwrap_or_default().join(path)
    };

    // Phase 1: Collect all searchable files (fast, sequential dir walk)
    let mut files = Vec::new();
    collect_files(&search_path, include, &mut files);

    // Phase 2: Search files in parallel with rayon
    let count = AtomicUsize::new(0);
    let results: Vec<Value> = files
        .par_iter()
        .flat_map(|file_path| {
            if count.load(Ordering::Relaxed) >= max_results {
                return Vec::new();
            }

            let pattern_lower = pattern.to_lowercase();
            let mut matches = Vec::new();

            if let Ok(content) = fs::read_to_string(file_path) {
                for (line_num, line) in content.lines().enumerate() {
                    if count.load(Ordering::Relaxed) >= max_results {
                        break;
                    }
                    if line.to_lowercase().contains(&pattern_lower) {
                        count.fetch_add(1, Ordering::Relaxed);
                        matches.push(json!({
                            "file": file_path.display().to_string(),
                            "line": line_num + 1,
                            "content": line.trim()
                        }));
                    }
                }
            }
            matches
        })
        .collect();

    // Trim to max_results (rayon may slightly overshoot due to parallelism)
    let trimmed: Vec<Value> = results.into_iter().take(max_results).collect();

    json!({
        "pattern": pattern,
        "path": search_path.display().to_string(),
        "matches": trimmed,
        "total_matches": trimmed.len()
    })
}

/// Collect all searchable files from a directory tree
fn collect_files(dir: &Path, include: Option<&str>, files: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        // Skip hidden dirs and common non-code directories
        if name.starts_with('.')
            || name == "node_modules"
            || name == "target"
            || name == "__pycache__"
            || name == "venv"
            || name == ".git"
        {
            continue;
        }

        if path.is_dir() {
            collect_files(&path, include, files);
        } else if path.is_file() {
            // Check include pattern (simple glob)
            if let Some(inc) = include {
                let inc = inc.trim_start_matches('*');
                if !name.ends_with(inc) {
                    continue;
                }
            }

            // Skip binary files
            if !is_likely_binary(&path) {
                files.push(path);
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

    // Check first few bytes for null bytes (read only 512 bytes, not the whole file)
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

/// Find files matching a glob pattern (parallelized with rayon)
pub fn find_files(args: &Value) -> Value {
    let pattern = match args.get("pattern").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => return json!({"error": "Missing 'pattern' argument"}),
    };
    let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
    let max_results: usize = args
        .get("max_results")
        .and_then(|v| v.as_u64())
        .unwrap_or(100) as usize;

    let search_path = if Path::new(path).is_absolute() {
        PathBuf::from(path)
    } else {
        std::env::current_dir().unwrap_or_default().join(path)
    };

    // Phase 1: Collect all files
    let mut all_files = Vec::new();
    collect_all_files(&search_path, &mut all_files);

    // Phase 2: Filter in parallel
    let pat = pattern.trim_start_matches('*');
    let results: Vec<String> = all_files
        .par_iter()
        .filter_map(|path| {
            let name = path.file_name()?.to_string_lossy();
            if name.contains(pat) || name.ends_with(pat) {
                Some(path.display().to_string())
            } else {
                None
            }
        })
        .take_any(max_results)
        .collect();

    json!({
        "pattern": pattern,
        "files": results,
        "total": results.len()
    })
}

fn collect_all_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if name.starts_with('.') || name == "node_modules" || name == "target" {
            continue;
        }

        if path.is_dir() {
            collect_all_files(&path, files);
        } else {
            files.push(path);
        }
    }
}
