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
            // Truncate very large files to save tokens
            let max_chars = 50_000;
            if content.len() > max_chars {
                let truncated = &content[..max_chars];
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
                return json!({"error": format!("old_string found {} times - must be unique. Provide more context.", count)});
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

fn resolve_path(path: &str) -> PathBuf {
    let p = Path::new(path);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_default().join(p)
    }
}

fn check_safe_path(path: &Path) -> Result<(), String> {
    let path_str = path.to_string_lossy();

    // Block dangerous system paths
    let blocked = [
        "/etc/passwd",
        "/etc/shadow",
        "/etc/sudoers",
        "/boot/",
        "/proc/",
        "/sys/",
        "/dev/",
    ];
    for b in &blocked {
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
