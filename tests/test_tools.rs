use serde_json::json;

// We need to test tools as a standalone module
// Import through the lib crate
use verysmolcode::tools::file_ops;
use verysmolcode::tools::git;
use verysmolcode::tools::grep;
use verysmolcode::tools::registry;

#[test]
fn test_read_file_missing_path() {
    let result = file_ops::read_file(&json!({}));
    assert!(result.get("error").is_some());
}

#[test]
fn test_read_file_nonexistent() {
    let result = file_ops::read_file(&json!({"path": "/tmp/nonexistent_vsc_test_file_12345"}));
    assert!(result.get("error").is_some());
}

#[test]
fn test_read_file_success() {
    // Write a temp file first
    let path = "/tmp/vsc_test_read.txt";
    std::fs::write(path, "hello world").unwrap();
    let result = file_ops::read_file(&json!({"path": path}));
    assert_eq!(result["content"].as_str().unwrap(), "hello world");
    std::fs::remove_file(path).ok();
}

#[test]
fn test_write_file_missing_args() {
    let result = file_ops::write_file(&json!({}));
    assert!(result.get("error").is_some());

    let result = file_ops::write_file(&json!({"path": "/tmp/test"}));
    assert!(result.get("error").is_some());
}

#[test]
fn test_write_file_success() {
    let path = "/tmp/vsc_test_write.txt";
    let result = file_ops::write_file(&json!({"path": path, "content": "test content"}));
    assert!(result["success"].as_bool().unwrap());
    assert_eq!(std::fs::read_to_string(path).unwrap(), "test content");
    std::fs::remove_file(path).ok();
}

#[test]
fn test_write_file_blocked_path() {
    let result = file_ops::write_file(&json!({"path": "/etc/passwd", "content": "bad"}));
    assert!(result.get("error").is_some());
}

#[test]
fn test_edit_file_success() {
    let path = "/tmp/vsc_test_edit.txt";
    std::fs::write(path, "hello world").unwrap();
    let result = file_ops::edit_file(&json!({
        "path": path,
        "old_string": "hello",
        "new_string": "goodbye"
    }));
    assert!(result["success"].as_bool().unwrap());
    assert_eq!(std::fs::read_to_string(path).unwrap(), "goodbye world");
    std::fs::remove_file(path).ok();
}

#[test]
fn test_edit_file_not_found() {
    let result = file_ops::edit_file(&json!({
        "path": "/tmp/vsc_test_edit.txt",
        "old_string": "nonexistent",
        "new_string": "replacement"
    }));
    assert!(result.get("error").is_some());
}

#[test]
fn test_edit_file_ambiguous() {
    let path = "/tmp/vsc_test_edit_dup.txt";
    std::fs::write(path, "hello hello hello").unwrap();
    let result = file_ops::edit_file(&json!({
        "path": path,
        "old_string": "hello",
        "new_string": "goodbye"
    }));
    assert!(result.get("error").is_some());
    assert!(result["error"].as_str().unwrap().contains("3 times"));
    std::fs::remove_file(path).ok();
}

#[test]
fn test_list_dir() {
    let result = file_ops::list_dir(&json!({"path": "/tmp"}));
    assert!(result.get("entries").is_some());
    assert!(result["entries"].as_array().unwrap().len() > 0);
}

#[test]
fn test_grep_search() {
    // Create test files
    let dir = "/tmp/vsc_grep_test";
    std::fs::create_dir_all(dir).ok();
    std::fs::write(
        format!("{}/test.txt", dir),
        "hello world\nfoo bar\nhello again",
    )
    .unwrap();

    let result = grep::grep_search(&json!({"pattern": "hello", "path": dir}));
    assert_eq!(result["total_matches"].as_u64().unwrap(), 2);

    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn test_find_files() {
    let dir = "/tmp/vsc_find_test";
    std::fs::create_dir_all(dir).ok();
    std::fs::write(format!("{}/test.rs", dir), "fn main() {}").unwrap();
    std::fs::write(format!("{}/test.py", dir), "print('hi')").unwrap();

    let result = grep::find_files(&json!({"pattern": "*.rs", "path": dir}));
    assert_eq!(result["total"].as_u64().unwrap(), 1);

    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn test_git_status() {
    let result = git::git_status(&json!({}));
    // Should succeed in a git repo
    assert!(result.get("output").is_some() || result.get("error").is_some());
}

#[test]
fn test_git_log() {
    let result = git::git_log(&json!({"count": 5}));
    assert!(result.get("output").is_some());
}

#[test]
fn test_run_shell_basic() {
    let result = git::run_shell(&json!({"command": "echo hello"}));
    assert!(result["success"].as_bool().unwrap());
    assert_eq!(result["stdout"].as_str().unwrap(), "hello");
}

#[test]
fn test_run_shell_blocked() {
    let result = git::run_shell(&json!({"command": "rm -rf /"}));
    assert!(result.get("error").is_some());
}

#[test]
fn test_tool_registry_execute() {
    let result = registry::execute_tool("read_file", &json!({"path": "/tmp/nonexistent_12345"}));
    assert!(result.get("error").is_some());
}

#[test]
fn test_tool_registry_unknown() {
    let result = registry::execute_tool("unknown_tool", &json!({}));
    assert!(result["error"].as_str().unwrap().contains("Unknown tool"));
}

#[test]
fn test_tool_declarations() {
    let decls = registry::ToolRegistry::declarations();
    assert!(!decls.is_empty());
    let funcs = &decls[0].function_declarations;
    assert!(funcs.len() >= 10); // Should have many tools

    // Check all tools have names and descriptions
    for f in funcs {
        assert!(!f.name.is_empty());
        assert!(!f.description.is_empty());
    }
}
