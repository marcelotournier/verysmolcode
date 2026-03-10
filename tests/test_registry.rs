use serde_json::json;
use verysmolcode::tools::registry::{execute_tool, get_tool_declarations, ToolRegistry};

// -- Tool declarations tests --

#[test]
fn test_declarations_count() {
    let decls = ToolRegistry::declarations();
    assert_eq!(decls.len(), 1); // One ToolDeclaration with all function decls
    assert_eq!(decls[0].function_declarations.len(), 18); // 18 tools
}

#[test]
fn test_declarations_tool_names() {
    let decls = get_tool_declarations();
    let names: Vec<&str> = decls[0]
        .function_declarations
        .iter()
        .map(|f| f.name.as_str())
        .collect();

    assert!(names.contains(&"read_file"));
    assert!(names.contains(&"write_file"));
    assert!(names.contains(&"edit_file"));
    assert!(names.contains(&"list_directory"));
    assert!(names.contains(&"grep_search"));
    assert!(names.contains(&"find_files"));
    assert!(names.contains(&"git_status"));
    assert!(names.contains(&"git_diff"));
    assert!(names.contains(&"git_log"));
    assert!(names.contains(&"git_commit"));
    assert!(names.contains(&"git_add"));
    assert!(names.contains(&"git_branch"));
    assert!(names.contains(&"git_checkout"));
    assert!(names.contains(&"git_push"));
    assert!(names.contains(&"git_pull"));
    assert!(names.contains(&"run_command"));
    assert!(names.contains(&"web_fetch"));
    assert!(names.contains(&"read_image"));
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

// -- Read-only declarations tests --

#[test]
fn test_read_only_declarations() {
    let decls = ToolRegistry::read_only_declarations();
    assert_eq!(decls.len(), 1);

    let names: Vec<&str> = decls[0]
        .function_declarations
        .iter()
        .map(|f| f.name.as_str())
        .collect();

    // Should include read-only tools
    assert!(names.contains(&"read_file"));
    assert!(names.contains(&"list_directory"));
    assert!(names.contains(&"grep_search"));
    assert!(names.contains(&"find_files"));
    assert!(names.contains(&"git_status"));
    assert!(names.contains(&"git_diff"));
    assert!(names.contains(&"git_log"));
    assert!(names.contains(&"web_fetch"));
    assert!(names.contains(&"read_image"));

    // Should NOT include write/mutation tools
    assert!(!names.contains(&"write_file"));
    assert!(!names.contains(&"edit_file"));
    assert!(!names.contains(&"git_commit"));
    assert!(!names.contains(&"git_add"));
    assert!(!names.contains(&"git_push"));
    assert!(!names.contains(&"git_pull"));
    assert!(!names.contains(&"git_checkout"));
    assert!(!names.contains(&"run_command"));
}

#[test]
fn test_read_only_count() {
    let decls = ToolRegistry::read_only_declarations();
    assert_eq!(decls[0].function_declarations.len(), 9); // 9 read-only tools
}

// -- execute_tool tests --

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
    // ToolRegistry::execute should work the same as execute_tool
    let result = ToolRegistry::execute("nonexistent_tool", &json!({}));
    assert!(result.get("error").is_some());
}

#[test]
fn test_execute_list_directory() {
    let result = execute_tool("list_directory", &json!({"path": "/tmp"}));
    // /tmp should exist and list successfully
    assert!(result.get("error").is_none() || result.get("entries").is_some());
}
