//! `write` tool — port of pi's write.ts.
//!
//! Creates or overwrites a file. Auto-creates parent directories. Serialized
//! per-file via the file mutation queue. Enforces the same VSC safe-path
//! checks (no /usr, /etc, /sys, etc.) and a 5MB write cap.

use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

use crate::tools::file_mutation_queue::with_file_mutation_queue;
use crate::tools::file_ops::check_safe_path;
use crate::tools::path_utils::resolve_to_cwd;

const MAX_FILE_WRITE_BYTES: usize = 5_000_000;

pub fn write(args: &Value) -> Value {
    let path = match args.get("path").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => return json!({"error": "Missing 'path' argument"}),
    };
    let content = match args.get("content").and_then(|v| v.as_str()) {
        Some(c) => c,
        None => return json!({"error": "Missing 'content' argument"}),
    };

    if content.len() > MAX_FILE_WRITE_BYTES {
        return json!({"error": format!(
            "Content too large ({} bytes). Maximum write size is {} bytes (5MB).",
            content.len(), MAX_FILE_WRITE_BYTES
        )});
    }

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let absolute = resolve_to_cwd(path, &cwd);

    if let Err(e) = check_safe_path(&absolute) {
        return json!({"error": e});
    }

    with_file_mutation_queue(&absolute, || {
        if let Some(parent) = absolute.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                return json!({"error": format!("Failed to create directories: {}", e)});
            }
        }
        match fs::write(&absolute, content) {
            Ok(()) => json!({
                "success": true,
                "path": absolute.display().to_string(),
                "bytes_written": content.len(),
                "content": format!("Successfully wrote {} bytes to {}", content.len(), path)
            }),
            Err(e) => json!({"error": format!("Failed to write {}: {}", absolute.display(), e)}),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_missing_args() {
        assert!(write(&json!({})).get("error").is_some());
        assert!(write(&json!({"path": "/tmp/x"})).get("error").is_some());
    }

    #[test]
    fn test_write_creates_file() {
        let p = std::env::temp_dir().join("vsc_write_create.txt");
        let _ = fs::remove_file(&p);
        let r = write(&json!({"path": p.to_str().unwrap(), "content": "hello"}));
        assert_eq!(r["success"], true);
        assert_eq!(fs::read_to_string(&p).unwrap(), "hello");
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn test_write_creates_parent_dirs() {
        let dir = std::env::temp_dir().join("vsc_write_nested/a/b");
        let _ = fs::remove_dir_all(std::env::temp_dir().join("vsc_write_nested"));
        let p = dir.join("file.txt");
        let r = write(&json!({"path": p.to_str().unwrap(), "content": "x"}));
        assert_eq!(r["success"], true);
        let _ = fs::remove_dir_all(std::env::temp_dir().join("vsc_write_nested"));
    }

    #[test]
    fn test_write_blocks_system_paths() {
        let r = write(&json!({"path": "/usr/local/x", "content": "x"}));
        assert!(r.get("error").is_some());
    }

    #[test]
    fn test_write_size_cap() {
        let big = "x".repeat(5_000_001);
        let r = write(&json!({"path": "/tmp/vsc_write_huge.txt", "content": big}));
        assert!(r["error"].as_str().unwrap().contains("too large"));
    }
}
