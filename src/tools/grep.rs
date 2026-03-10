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
    // Use SeqCst ordering for tighter result count control on multi-core
    let count = AtomicUsize::new(0);
    let pattern_lower = pattern.to_lowercase();
    let results: Vec<Value> = files
        .par_iter()
        .flat_map(|file_path| {
            if count.load(Ordering::SeqCst) >= max_results {
                return Vec::new();
            }

            let mut matches = Vec::new();

            if let Ok(content) = fs::read_to_string(file_path) {
                for (line_num, line) in content.lines().enumerate() {
                    if count.load(Ordering::SeqCst) >= max_results {
                        break;
                    }
                    if line.to_lowercase().contains(&pattern_lower) {
                        count.fetch_add(1, Ordering::SeqCst);
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

    // Trim to exact max_results (rayon may overshoot slightly due to parallelism)
    let trimmed: Vec<Value> = results.into_iter().take(max_results).collect();
    let total = trimmed.len();

    json!({
        "pattern": pattern,
        "path": search_path.display().to_string(),
        "matches": trimmed,
        "total_matches": total
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn make_temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vsc_test_grep_{}", name));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_is_likely_binary_by_extension() {
        assert!(is_likely_binary(Path::new("image.png")));
        assert!(is_likely_binary(Path::new("archive.zip")));
        assert!(is_likely_binary(Path::new("lib.so")));
        assert!(is_likely_binary(Path::new("code.pyc")));
        assert!(is_likely_binary(Path::new("app.exe")));
        assert!(is_likely_binary(Path::new("module.wasm")));
    }

    #[test]
    fn test_is_likely_binary_text_extension() {
        assert!(!is_likely_binary(Path::new("code.rs")));
        assert!(!is_likely_binary(Path::new("readme.md")));
        assert!(!is_likely_binary(Path::new("config.toml")));
    }

    #[test]
    fn test_is_likely_binary_null_bytes() {
        let dir = make_temp_dir("null_bytes");
        let path = dir.join("test.dat");
        let mut f = fs::File::create(&path).unwrap();
        f.write_all(&[0x48, 0x65, 0x6c, 0x00, 0x6f]).unwrap();
        assert!(is_likely_binary(&path));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_is_likely_binary_clean_text_file() {
        let dir = make_temp_dir("clean_text");
        let path = dir.join("clean.txt");
        fs::write(&path, "Hello, this is text!").unwrap();
        assert!(!is_likely_binary(&path));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_is_likely_binary_nonexistent_file() {
        assert!(!is_likely_binary(Path::new("/nonexistent/file.xyz")));
    }

    #[test]
    fn test_grep_search_missing_pattern() {
        let result = grep_search(&json!({}));
        assert!(result.get("error").is_some());
    }

    #[test]
    fn test_grep_search_nonexistent_path() {
        let result = grep_search(&json!({"pattern": "hello", "path": "/nonexistent/dir"}));
        let matches = result.get("matches").and_then(|v| v.as_array());
        assert!(matches.is_some());
        assert_eq!(matches.unwrap().len(), 0);
    }

    #[test]
    fn test_grep_search_with_include_filter() {
        let dir = make_temp_dir("include_filter");
        fs::write(dir.join("code.rs"), "fn hello() {}").unwrap();
        fs::write(dir.join("data.txt"), "hello world").unwrap();

        let result = grep_search(&json!({
            "pattern": "hello",
            "path": dir.to_str().unwrap(),
            "include": "*.rs"
        }));
        let matches = result.get("matches").and_then(|v| v.as_array()).unwrap();
        assert_eq!(matches.len(), 1);
        assert!(matches[0]["file"].as_str().unwrap().ends_with("code.rs"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_grep_search_max_results() {
        let dir = make_temp_dir("max_results");
        let mut content = String::new();
        for i in 0..20 {
            content.push_str(&format!("hello line {}\n", i));
        }
        fs::write(dir.join("many.txt"), &content).unwrap();

        let result = grep_search(&json!({
            "pattern": "hello",
            "path": dir.to_str().unwrap(),
            "max_results": 3
        }));
        let matches = result.get("matches").and_then(|v| v.as_array()).unwrap();
        assert!(matches.len() <= 3);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_find_files_missing_pattern() {
        let result = find_files(&json!({}));
        assert!(result.get("error").is_some());
    }

    #[test]
    fn test_find_files_no_match() {
        let dir = make_temp_dir("no_match");
        fs::write(dir.join("code.rs"), "fn main() {}").unwrap();

        let result = find_files(&json!({
            "pattern": "*.py",
            "path": dir.to_str().unwrap()
        }));
        let files = result.get("files").and_then(|v| v.as_array()).unwrap();
        assert_eq!(files.len(), 0);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_collect_files_skips_hidden_dirs() {
        let dir = make_temp_dir("hidden_dirs");
        let hidden = dir.join(".hidden");
        fs::create_dir(&hidden).unwrap();
        fs::write(hidden.join("secret.rs"), "secret").unwrap();
        fs::write(dir.join("visible.rs"), "visible").unwrap();

        let mut files = Vec::new();
        collect_files(&dir, None, &mut files);
        assert_eq!(files.len(), 1);
        assert!(files[0].to_string_lossy().contains("visible.rs"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_grep_search_case_insensitive() {
        let dir = make_temp_dir("case_insensitive");
        fs::write(dir.join("mixed.txt"), "Hello WORLD hElLo").unwrap();

        let result = grep_search(&json!({
            "pattern": "hello",
            "path": dir.to_str().unwrap()
        }));
        let matches = result.get("matches").and_then(|v| v.as_array()).unwrap();
        assert_eq!(matches.len(), 1);
        // The line contains both "Hello" and "hElLo" — case-insensitive match found it
        assert!(matches[0]["content"].as_str().unwrap().contains("Hello"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_grep_search_case_insensitive_no_match() {
        let dir = make_temp_dir("case_no_match");
        fs::write(dir.join("nope.txt"), "Goodbye WORLD").unwrap();

        let result = grep_search(&json!({
            "pattern": "hello",
            "path": dir.to_str().unwrap()
        }));
        let matches = result.get("matches").and_then(|v| v.as_array()).unwrap();
        assert_eq!(matches.len(), 0);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_collect_files_skips_node_modules() {
        let dir = make_temp_dir("node_modules_skip");
        let nm = dir.join("node_modules");
        fs::create_dir(&nm).unwrap();
        fs::write(nm.join("pkg.js"), "module").unwrap();
        fs::write(dir.join("app.js"), "app").unwrap();

        let mut files = Vec::new();
        collect_files(&dir, None, &mut files);
        assert_eq!(files.len(), 1);
        let _ = fs::remove_dir_all(&dir);
    }
}
