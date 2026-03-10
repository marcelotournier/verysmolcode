use base64::Engine;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

/// Read a file's contents
pub fn read_file(args: &Value) -> Value {
    let path = match args.get("path").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => return json!({"error": "Missing 'path' argument"}),
    };

    let path = resolve_path(path);
    match fs::read_to_string(&path) {
        Ok(content) => {
            // Truncate very large files to save tokens (at a safe char boundary)
            let max_chars = 50_000;
            if content.len() > max_chars {
                let mut end = max_chars;
                while end > 0 && !content.is_char_boundary(end) {
                    end -= 1;
                }
                let truncated = &content[..end];
                json!({
                    "content": truncated,
                    "truncated": true,
                    "total_bytes": content.len(),
                    "path": path.display().to_string()
                })
            } else {
                json!({
                    "content": content,
                    "path": path.display().to_string()
                })
            }
        }
        Err(e) => json!({"error": format!("Failed to read {}: {}", path.display(), e)}),
    }
}

/// Maximum file size the agent is allowed to write (5MB).
/// Prevents disk exhaustion on constrained devices like RPi3.
const MAX_FILE_WRITE_BYTES: usize = 5_000_000;

/// Write content to a file (creates directories if needed)
pub fn write_file(args: &Value) -> Value {
    let path = match args.get("path").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => return json!({"error": "Missing 'path' argument"}),
    };
    let content = match args.get("content").and_then(|v| v.as_str()) {
        Some(c) => c,
        None => return json!({"error": "Missing 'content' argument"}),
    };

    // Reject oversized writes to protect disk space
    if content.len() > MAX_FILE_WRITE_BYTES {
        return json!({"error": format!(
            "Content too large ({} bytes). Maximum write size is {} bytes (5MB).",
            content.len(), MAX_FILE_WRITE_BYTES
        )});
    }

    let path = resolve_path(path);

    // Safety check - don't write outside the working directory
    if let Err(e) = check_safe_path(&path) {
        return json!({"error": e});
    }

    // Create parent directories
    if let Some(parent) = path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            return json!({"error": format!("Failed to create directories: {}", e)});
        }
    }

    match fs::write(&path, content) {
        Ok(()) => json!({
            "success": true,
            "path": path.display().to_string(),
            "bytes_written": content.len()
        }),
        Err(e) => json!({"error": format!("Failed to write {}: {}", path.display(), e)}),
    }
}

/// Edit a file by replacing old_string with new_string
pub fn edit_file(args: &Value) -> Value {
    let path = match args.get("path").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => return json!({"error": "Missing 'path' argument"}),
    };
    let old_string = match args.get("old_string").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return json!({"error": "Missing 'old_string' argument"}),
    };
    let new_string = match args.get("new_string").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return json!({"error": "Missing 'new_string' argument"}),
    };

    let path = resolve_path(path);
    if let Err(e) = check_safe_path(&path) {
        return json!({"error": e});
    }

    match fs::read_to_string(&path) {
        Ok(content) => {
            let count = content.matches(old_string).count();
            if count == 0 {
                return json!({"error": "old_string not found in file"});
            }
            if count > 1 {
                // Show line numbers of each match so the model can provide more context
                let match_lines: Vec<usize> = content
                    .lines()
                    .enumerate()
                    .filter(|(_, line)| line.contains(old_string))
                    .map(|(i, _)| i + 1)
                    .collect();
                return json!({
                    "error": format!("old_string found {} times - must be unique. Provide more surrounding context.", count),
                    "match_lines": match_lines
                });
            }

            let new_content = content.replacen(old_string, new_string, 1);
            match fs::write(&path, &new_content) {
                Ok(()) => json!({
                    "success": true,
                    "path": path.display().to_string(),
                    "replacements": 1
                }),
                Err(e) => json!({"error": format!("Failed to write: {}", e)}),
            }
        }
        Err(e) => json!({"error": format!("Failed to read {}: {}", path.display(), e)}),
    }
}

/// List directory contents
pub fn list_dir(args: &Value) -> Value {
    let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
    let path = resolve_path(path);

    match fs::read_dir(&path) {
        Ok(entries) => {
            let mut items: Vec<Value> = Vec::new();
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                items.push(json!({
                    "name": name,
                    "is_dir": is_dir,
                    "size": size
                }));
            }
            items.sort_by(|a, b| {
                let a_dir = a["is_dir"].as_bool().unwrap_or(false);
                let b_dir = b["is_dir"].as_bool().unwrap_or(false);
                b_dir.cmp(&a_dir).then_with(|| {
                    a["name"]
                        .as_str()
                        .unwrap_or("")
                        .cmp(b["name"].as_str().unwrap_or(""))
                })
            });
            json!({
                "path": path.display().to_string(),
                "entries": items
            })
        }
        Err(e) => json!({"error": format!("Failed to list {}: {}", path.display(), e)}),
    }
}

/// Read an image file and return base64-encoded data for Gemini
pub fn read_image(args: &Value) -> Value {
    let path = match args.get("path").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => return json!({"error": "Missing 'path' argument"}),
    };

    let path = resolve_path(path);

    // Determine MIME type from extension
    let mime_type = match path.extension().and_then(|e| e.to_str()) {
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("bmp") => "image/bmp",
        Some(ext) => {
            return json!({"error": format!("Unsupported image format: {}", ext)});
        }
        None => {
            return json!({"error": "Cannot determine image format (no extension)"});
        }
    };

    // Limit to 10MB
    match fs::metadata(&path) {
        Ok(meta) if meta.len() > 10_000_000 => {
            return json!({"error": "Image too large (max 10MB)"});
        }
        Err(e) => {
            return json!({"error": format!("Cannot read {}: {}", path.display(), e)});
        }
        _ => {}
    }

    match fs::read(&path) {
        Ok(data) => {
            let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
            json!({
                "inline_data": {
                    "mime_type": mime_type,
                    "data": b64
                },
                "path": path.display().to_string(),
                "size_bytes": data.len()
            })
        }
        Err(e) => json!({"error": format!("Failed to read {}: {}", path.display(), e)}),
    }
}

fn resolve_path(path: &str) -> PathBuf {
    let p = Path::new(path);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_default().join(p)
    }
}

/// System paths that should never be written to by the coding assistant.
/// Single source of truth — reused by is_dangerous_tool_call() in agent/loop_runner.rs.
pub const BLOCKED_PATH_PREFIXES: &[&str] = &[
    "/etc/", "/boot/", "/usr/", "/bin/", "/sbin/", "/lib/", "/proc/", "/sys/", "/dev/",
];

fn check_safe_path(path: &Path) -> Result<(), String> {
    let path_str = path.to_string_lossy();

    // Block dangerous system paths
    for b in BLOCKED_PATH_PREFIXES {
        if path_str.starts_with(b) {
            return Err(format!("Access denied: {} is a protected path", b));
        }
    }

    // Block home directory dotfiles that could be dangerous
    if let Some(home) = dirs::home_dir() {
        let dangerous_dotfiles = [".bashrc", ".profile", ".bash_profile", ".ssh", ".gnupg"];
        for df in &dangerous_dotfiles {
            if path == home.join(df) {
                return Err(format!("Access denied: modifying ~/{} is not allowed", df));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- check_safe_path tests --

    #[test]
    fn test_safe_path_normal() {
        assert!(check_safe_path(Path::new("/tmp/test.txt")).is_ok());
        assert!(check_safe_path(Path::new("/home/user/project/src/main.rs")).is_ok());
    }

    #[test]
    fn test_safe_path_blocks_etc() {
        assert!(check_safe_path(Path::new("/etc/passwd")).is_err());
    }

    #[test]
    fn test_safe_path_blocks_usr() {
        assert!(check_safe_path(Path::new("/usr/local/bin/app")).is_err());
    }

    #[test]
    fn test_safe_path_blocks_proc() {
        assert!(check_safe_path(Path::new("/proc/1/status")).is_err());
    }

    #[test]
    fn test_safe_path_blocks_sys() {
        assert!(check_safe_path(Path::new("/sys/class/net")).is_err());
    }

    #[test]
    fn test_safe_path_blocks_home_dotfiles() {
        if let Some(home) = dirs::home_dir() {
            assert!(check_safe_path(&home.join(".bashrc")).is_err());
            assert!(check_safe_path(&home.join(".ssh")).is_err());
            assert!(check_safe_path(&home.join(".gnupg")).is_err());
        }
    }

    // -- read_file tests --

    #[test]
    fn test_read_file_missing_path() {
        let result = read_file(&json!({}));
        assert!(result.get("error").is_some());
    }

    #[test]
    fn test_read_file_nonexistent() {
        let result = read_file(&json!({"path": "/tmp/vsc_nonexistent_file_test.txt"}));
        assert!(result.get("error").is_some());
    }

    #[test]
    fn test_read_file_success() {
        let path = "/tmp/vsc_test_read_file.txt";
        fs::write(path, "hello world").unwrap();
        let result = read_file(&json!({"path": path}));
        assert_eq!(result["content"].as_str().unwrap(), "hello world");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_read_file_truncation() {
        let path = "/tmp/vsc_test_large_file.txt";
        let content = "x".repeat(60_000);
        fs::write(path, &content).unwrap();
        let result = read_file(&json!({"path": path}));
        assert_eq!(result["truncated"].as_bool(), Some(true));
        assert!(result["content"].as_str().unwrap().len() <= 50_000);
        let _ = fs::remove_file(path);
    }

    // -- edit_file tests --

    #[test]
    fn test_edit_file_missing_path() {
        let result = edit_file(&json!({"old_string": "a", "new_string": "b"}));
        assert!(result.get("error").is_some());
    }

    #[test]
    fn test_edit_file_missing_old_string() {
        let result = edit_file(&json!({"path": "/tmp/x.txt", "new_string": "b"}));
        assert!(result.get("error").is_some());
    }

    #[test]
    fn test_edit_file_missing_new_string() {
        let result = edit_file(&json!({"path": "/tmp/x.txt", "old_string": "a"}));
        assert!(result.get("error").is_some());
    }

    #[test]
    fn test_edit_file_success() {
        let path = "/tmp/vsc_test_edit_file.txt";
        fs::write(path, "hello world").unwrap();
        let result = edit_file(&json!({"path": path, "old_string": "world", "new_string": "rust"}));
        assert_eq!(result["success"].as_bool(), Some(true));
        let content = fs::read_to_string(path).unwrap();
        assert_eq!(content, "hello rust");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_edit_file_not_found_in_content() {
        let path = "/tmp/vsc_test_edit_notfound.txt";
        fs::write(path, "hello world").unwrap();
        let result = edit_file(&json!({"path": path, "old_string": "xyz", "new_string": "abc"}));
        assert!(result.get("error").is_some());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_edit_file_ambiguous_match() {
        let path = "/tmp/vsc_test_edit_ambiguous.txt";
        fs::write(path, "hello hello world").unwrap();
        let result = edit_file(&json!({"path": path, "old_string": "hello", "new_string": "hi"}));
        assert!(result["error"].as_str().unwrap().contains("found 2 times"));
        // Should include line numbers of matches
        assert!(result.get("match_lines").is_some());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_edit_file_ambiguous_multiline() {
        let path = "/tmp/vsc_test_edit_ambiguous_ml.txt";
        fs::write(path, "foo\nbar\nfoo\nbaz").unwrap();
        let result = edit_file(&json!({"path": path, "old_string": "foo", "new_string": "qux"}));
        let lines = result["match_lines"].as_array().unwrap();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].as_u64().unwrap(), 1);
        assert_eq!(lines[1].as_u64().unwrap(), 3);
        let _ = fs::remove_file(path);
    }

    // -- list_dir tests --

    #[test]
    fn test_list_dir_nonexistent() {
        let result = list_dir(&json!({"path": "/tmp/vsc_nonexistent_dir_12345"}));
        assert!(result.get("error").is_some());
    }

    #[test]
    fn test_list_dir_sorting() {
        let dir = "/tmp/vsc_test_list_dir";
        let _ = fs::remove_dir_all(dir);
        fs::create_dir_all(format!("{}/zdir", dir)).unwrap();
        fs::create_dir_all(format!("{}/adir", dir)).unwrap();
        fs::write(format!("{}/zfile.txt", dir), "z").unwrap();
        fs::write(format!("{}/afile.txt", dir), "a").unwrap();

        let result = list_dir(&json!({"path": dir}));
        let entries = result["entries"].as_array().unwrap();
        // Dirs should come first, then files, both alphabetical
        assert!(entries[0]["is_dir"].as_bool().unwrap());
        assert!(entries[1]["is_dir"].as_bool().unwrap());
        assert!(!entries[2]["is_dir"].as_bool().unwrap());
        assert!(!entries[3]["is_dir"].as_bool().unwrap());
        let _ = fs::remove_dir_all(dir);
    }

    // -- read_image tests --

    #[test]
    fn test_read_image_missing_path() {
        let result = read_image(&json!({}));
        assert!(result.get("error").is_some());
    }

    #[test]
    fn test_read_image_no_extension() {
        let path = "/tmp/vsc_test_img_noext";
        fs::write(path, "data").unwrap();
        let result = read_image(&json!({"path": path}));
        assert!(result["error"].as_str().unwrap().contains("no extension"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_read_image_unsupported_format() {
        let result = read_image(&json!({"path": "/tmp/test.tiff"}));
        assert!(result["error"]
            .as_str()
            .unwrap()
            .contains("Unsupported image format"));
    }

    #[test]
    fn test_read_image_png_success() {
        let path = "/tmp/vsc_test_fake.png";
        fs::write(path, b"\x89PNG\r\n\x1a\n").unwrap(); // minimal PNG header
        let result = read_image(&json!({"path": path}));
        assert!(result.get("inline_data").is_some());
        assert_eq!(
            result["inline_data"]["mime_type"].as_str().unwrap(),
            "image/png"
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_read_image_jpg_mime() {
        let path = "/tmp/vsc_test_fake.jpg";
        fs::write(path, b"\xFF\xD8\xFF").unwrap(); // JPEG magic
        let result = read_image(&json!({"path": path}));
        assert_eq!(
            result["inline_data"]["mime_type"].as_str().unwrap(),
            "image/jpeg"
        );
        let _ = fs::remove_file(path);
    }

    // -- write_file safety tests --

    #[test]
    fn test_write_file_blocked_lib() {
        let result = write_file(&json!({"path": "/lib/evil.so", "content": "bad"}));
        assert!(result.get("error").is_some());
    }

    #[test]
    fn test_write_file_missing_content() {
        let result = write_file(&json!({"path": "/tmp/test.txt"}));
        assert!(result.get("error").is_some());
    }

    #[test]
    fn test_write_file_size_limit() {
        // Content over 5MB should be rejected
        let big = "x".repeat(5_000_001);
        let result = write_file(&json!({"path": "/tmp/vsc_too_big.txt", "content": big}));
        assert!(result["error"].as_str().unwrap().contains("too large"));
    }

    #[test]
    fn test_write_file_under_size_limit() {
        let path = "/tmp/vsc_test_size_ok.txt";
        let content = "x".repeat(1000);
        let result = write_file(&json!({"path": path, "content": content}));
        assert_eq!(result["success"].as_bool(), Some(true));
        let _ = fs::remove_file(path);
    }
}
