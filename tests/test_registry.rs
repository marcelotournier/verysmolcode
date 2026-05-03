//! Registry-level integration tests.
//!
//! After v0.16.0 the canonical tool names exposed to the model are the
//! pi-style short names (`read`, `write`, `edit`, `ls`, `grep`, `find`,
//! `bash`, `task`, `vsc_help`). The legacy long names (`read_file`,
//! `write_file`, …) are still accepted by `execute_tool()` so existing
//! sessions keep working, but they are no longer in the declarations
//! advertised to the model.

use serde_json::json;
use verysmolcode::tools::registry::{execute_tool, get_tool_declarations, ToolRegistry};

const PI_TOOLS: &[&str] = &["read", "write", "edit", "ls", "grep", "find", "bash"];
const VSC_EXTRA: &[&str] = &[
    "task",
    "vsc_help",
    "git_status",
    "git_diff",
    "git_log",
    "git_commit",
    "git_add",
    "git_branch",
    "git_checkout",
    "git_push",
    "git_pull",
    "web_fetch",
    "todo_update",
    "send_telegram",
];

#[test]
fn test_declarations_count() {
    let decls = ToolRegistry::declarations();
    assert_eq!(decls.len(), 1);
    let n = decls[0].function_declarations.len();
    // 7 pi-style + 14 VSC extras = 21 total advertised tools
    assert!(n >= 18, "expected at least 18 declared tools, got {}", n);
}

#[test]
fn test_declarations_tool_names() {
    let decls = get_tool_declarations();
    let names: Vec<&str> = decls[0]
        .function_declarations
        .iter()
        .map(|f| f.name.as_str())
        .collect();
    for n in PI_TOOLS {
        assert!(names.contains(n), "missing pi tool: {}", n);
    }
    for n in VSC_EXTRA {
        assert!(names.contains(n), "missing vsc tool: {}", n);
    }
}

#[test]
fn test_legacy_names_routed_via_execute_tool() {
    // Legacy names are not declared, but the dispatcher must still accept them
    // so saved sessions and older transcripts keep working.
    for legacy in &[
        "read_file",
        "write_file",
        "edit_file",
        "list_directory",
        "grep_search",
        "find_files",
        "run_command",
        "read_image",
    ] {
        let r = execute_tool(legacy, &json!({"path": "/tmp/__nope__"}));
        // We don't care what error comes back — only that it isn't "Unknown tool"
        let err = r.get("error").and_then(|v| v.as_str()).unwrap_or("");
        assert!(
            !err.contains("Unknown tool"),
            "legacy alias '{}' is no longer routed",
            legacy
        );
    }
}

#[test]
fn test_declarations_have_descriptions() {
    let decls = get_tool_declarations();
    for func in &decls[0].function_declarations {
        assert!(
            !func.description.is_empty(),
            "Tool {} has empty description",
            func.name
        );
    }
}

#[test]
fn test_declarations_have_parameters() {
    let decls = get_tool_declarations();
    for func in &decls[0].function_declarations {
        assert!(
            func.parameters.is_object(),
            "Tool {} parameters is not an object",
            func.name
        );
        assert_eq!(
            func.parameters.get("type").unwrap().as_str().unwrap(),
            "object",
            "Tool {} parameters type is not 'object'",
            func.name
        );
    }
}

#[test]
fn test_read_only_declarations() {
    let decls = ToolRegistry::read_only_declarations();
    assert_eq!(decls.len(), 1);
    let names: Vec<&str> = decls[0]
        .function_declarations
        .iter()
        .map(|f| f.name.as_str())
        .collect();

    // Read-only set should expose pi-style read/grep/find/ls + read-only VSC extras
    for n in &[
        "read",
        "ls",
        "grep",
        "find",
        "git_status",
        "git_diff",
        "git_log",
        "web_fetch",
        "todo_update",
        "vsc_help",
    ] {
        assert!(names.contains(n), "missing read-only tool: {}", n);
    }

    // Mutation tools must not be in the read-only set
    for n in &[
        "write",
        "edit",
        "bash",
        "git_commit",
        "git_add",
        "git_push",
        "git_pull",
        "git_checkout",
    ] {
        assert!(!names.contains(n), "{} should not be in read-only set", n);
    }
}

#[test]
fn test_read_only_count() {
    let decls = ToolRegistry::read_only_declarations();
    let n = decls[0].function_declarations.len();
    assert!(
        n >= 8,
        "expected at least 8 read-only declarations, got {}",
        n
    );
}

#[test]
fn test_execute_unknown_tool() {
    let result = execute_tool("nonexistent_tool", &json!({}));
    assert!(result.get("error").is_some());
    let err = result.get("error").unwrap().as_str().unwrap();
    assert!(err.contains("Unknown tool"));
    assert!(err.contains("nonexistent_tool"));
}

#[test]
fn test_execute_read_file_nonexistent() {
    let result = execute_tool("read_file", &json!({"path": "/nonexistent/path/file.txt"}));
    assert!(result.get("error").is_some());
}

#[test]
fn test_execute_via_registry() {
    let result = ToolRegistry::execute("nonexistent_tool", &json!({}));
    assert!(result.get("error").is_some());
}

#[test]
fn test_execute_list_directory() {
    // legacy alias still works
    let r = execute_tool("list_directory", &json!({"path": "/tmp"}));
    assert!(r.get("error").is_none() || r.get("content").is_some() || r.get("entries").is_some());
}
