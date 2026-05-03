//! Integration tests routed through the public registry.
//!
//! After the v0.16.0 pi-style refactor, the canonical tool names are
//! `read`, `write`, `edit`, `ls`, `grep`, `find`, `bash`, but the
//! registry preserves legacy aliases (`read_file`, `write_file`,
//! `edit_file`, `list_directory`, `grep_search`, `find_files`,
//! `run_command`, `read_image`). Going through the registry keeps these
//! tests stable across future module reorganizations.

use serde_json::json;

use verysmolcode::tools::registry;

fn exec(name: &str, args: serde_json::Value) -> serde_json::Value {
    registry::execute_tool(name, &args)
}

#[test]
fn test_read_file_missing_path() {
    let r = exec("read", json!({}));
    assert!(r.get("error").is_some());
}

#[test]
fn test_read_file_nonexistent() {
    let r = exec(
        "read",
        json!({"path": "/tmp/nonexistent_vsc_test_file_12345"}),
    );
    assert!(r.get("error").is_some());
}

#[test]
fn test_read_file_success() {
    let path = "/tmp/vsc_test_read.txt";
    std::fs::write(path, "hello world").unwrap();
    let r = exec("read", json!({"path": path}));
    assert_eq!(r["content"].as_str().unwrap(), "hello world");
    std::fs::remove_file(path).ok();
}

#[test]
fn test_write_file_missing_args() {
    assert!(exec("write", json!({})).get("error").is_some());
    assert!(exec("write", json!({"path": "/tmp/test"}))
        .get("error")
        .is_some());
}

#[test]
fn test_write_file_success() {
    let path = "/tmp/vsc_test_write.txt";
    let r = exec("write", json!({"path": path, "content": "test content"}));
    assert!(r["success"].as_bool().unwrap());
    assert_eq!(std::fs::read_to_string(path).unwrap(), "test content");
    std::fs::remove_file(path).ok();
}

#[test]
fn test_write_file_blocked_path() {
    let r = exec("write", json!({"path": "/etc/passwd", "content": "bad"}));
    assert!(r.get("error").is_some());
}

#[test]
fn test_write_file_blocked_usr() {
    let r = exec("write", json!({"path": "/usr/bin/evil", "content": "bad"}));
    assert!(r.get("error").is_some());
}

#[test]
fn test_write_file_blocked_bin() {
    let r = exec("write", json!({"path": "/bin/evil", "content": "bad"}));
    assert!(r.get("error").is_some());
}

#[test]
fn test_write_file_blocked_sbin() {
    let r = exec("write", json!({"path": "/sbin/evil", "content": "bad"}));
    assert!(r.get("error").is_some());
}

#[test]
fn test_edit_file_success() {
    let path = "/tmp/vsc_test_edit.txt";
    std::fs::write(path, "hello world").unwrap();
    let r = exec(
        "edit",
        json!({
            "path": path,
            "old_string": "hello",
            "new_string": "goodbye"
        }),
    );
    assert!(r["success"].as_bool().unwrap());
    assert_eq!(std::fs::read_to_string(path).unwrap(), "goodbye world");
    std::fs::remove_file(path).ok();
}

#[test]
fn test_edit_file_not_found() {
    // Use a fresh file so the test doesn't fight with test_edit_file_success
    let path = "/tmp/vsc_test_edit_notfound.txt";
    std::fs::write(path, "abc def").unwrap();
    let r = exec(
        "edit",
        json!({
            "path": path,
            "old_string": "nonexistent",
            "new_string": "replacement"
        }),
    );
    assert!(r.get("error").is_some());
    std::fs::remove_file(path).ok();
}

#[test]
fn test_edit_file_ambiguous() {
    let path = "/tmp/vsc_test_edit_dup.txt";
    std::fs::write(path, "hello hello hello").unwrap();
    let r = exec(
        "edit",
        json!({
            "path": path,
            "old_string": "hello",
            "new_string": "goodbye"
        }),
    );
    assert!(r.get("error").is_some());
    let err = r["error"].as_str().unwrap();
    // Pi-style edit reports "Found N occurrences"; legacy reports "found N times".
    assert!(err.contains("3 occurrences") || err.contains("3 times"));
    std::fs::remove_file(path).ok();
}

#[test]
fn test_list_dir() {
    // ls returns a single newline-joined `content` string in the new shape.
    let r = exec("ls", json!({"path": "/tmp"}));
    assert!(r.get("content").is_some() || r.get("entries").is_some());
}

#[test]
fn test_grep_search() {
    let dir = "/tmp/vsc_grep_test";
    std::fs::create_dir_all(dir).ok();
    std::fs::write(
        format!("{}/test.txt", dir),
        "hello world\nfoo bar\nhello again",
    )
    .unwrap();

    let r = exec("grep", json!({"pattern": "hello", "path": dir}));
    assert_eq!(r["total_matches"].as_u64().unwrap(), 2);

    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn test_find_files() {
    let dir = "/tmp/vsc_find_test";
    std::fs::create_dir_all(dir).ok();
    std::fs::write(format!("{}/test.rs", dir), "fn main() {}").unwrap();
    std::fs::write(format!("{}/test.py", dir), "print('hi')").unwrap();

    let r = exec("find", json!({"pattern": "*.rs", "path": dir}));
    assert_eq!(r["total"].as_u64().unwrap(), 1);

    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn test_git_status() {
    let r = exec("git_status", json!({}));
    assert!(r.get("output").is_some() || r.get("error").is_some());
}

#[test]
fn test_git_log() {
    let r = exec("git_log", json!({"count": 5}));
    assert!(r.get("output").is_some() || r.get("error").is_some());
}

#[test]
fn test_run_shell_basic() {
    let r = exec("bash", json!({"command": "echo hello"}));
    assert!(r["success"].as_bool().unwrap());
    assert!(r["output"].as_str().unwrap().contains("hello"));
}

#[test]
fn test_run_shell_blocked() {
    let r = exec("bash", json!({"command": "rm -rf /"}));
    assert!(r.get("error").is_some());
}

#[test]
fn test_tool_registry_execute() {
    let r = exec("read", json!({"path": "/tmp/nonexistent_12345"}));
    assert!(r.get("error").is_some());
}

#[test]
fn test_tool_registry_unknown() {
    let r = exec("unknown_tool", json!({}));
    assert!(r["error"].as_str().unwrap().contains("Unknown tool"));
}

#[test]
fn test_tool_declarations() {
    let decls = registry::ToolRegistry::declarations();
    assert!(!decls.is_empty());
    let funcs = &decls[0].function_declarations;
    assert!(funcs.len() >= 10);

    for f in funcs {
        assert!(!f.name.is_empty());
        assert!(!f.description.is_empty());
    }

    // Pi-style canonical names must all be advertised.
    for n in &["read", "write", "edit", "ls", "grep", "find", "bash"] {
        assert!(funcs.iter().any(|f| f.name == *n), "missing tool: {}", n);
    }
    // VSC custom tools.
    for n in &[
        "task",
        "vsc_help",
        "git_status",
        "todo_update",
        "send_telegram",
    ] {
        assert!(funcs.iter().any(|f| f.name == *n), "missing tool: {}", n);
    }
}

#[test]
fn test_read_image_missing_path() {
    let r = exec("read_image", json!({}));
    assert!(r.get("error").is_some());
}

#[test]
fn test_read_image_nonexistent() {
    let r = exec(
        "read_image",
        json!({"path": "/tmp/nonexistent_vsc_test.png"}),
    );
    assert!(r.get("error").is_some());
}

#[test]
fn test_read_image_unsupported_format() {
    // Create a real file with an unsupported extension; otherwise the read
    // tool short-circuits on "file not found" before mime inference.
    let path = "/tmp/vsc_test_unsupported_image.xyz";
    std::fs::write(path, b"not an image").unwrap();
    let r = exec("read_image", json!({"path": path}));
    // Pi-style read just falls back to text; either way the result must NOT
    // contain inline_data, since .xyz is not a recognized image extension.
    assert!(r.get("inline_data").is_none());
    std::fs::remove_file(path).ok();
}

#[test]
fn test_read_image_success() {
    // Tiny valid PNG (1x1 pixel)
    let png_data: Vec<u8> = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90,
        0x77, 0x53, 0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8,
        0xCF, 0xC0, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01, 0xE2, 0x21, 0xBC, 0x33, 0x00, 0x00, 0x00,
        0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];
    let path = "/tmp/vsc_test_image.png";
    std::fs::write(path, &png_data).unwrap();

    let r = exec("read_image", json!({"path": path}));
    assert!(r.get("inline_data").is_some());
    let inline = r.get("inline_data").unwrap();
    assert_eq!(
        inline.get("mime_type").unwrap().as_str().unwrap(),
        "image/png"
    );
    assert!(inline.get("data").unwrap().as_str().unwrap().len() > 10);
    assert!(r.get("size_bytes").is_some());

    std::fs::remove_file(path).unwrap();
}
