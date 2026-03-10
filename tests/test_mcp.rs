use serde_json::json;
use verysmolcode::mcp::types::*;

// -- JsonRpcRequest tests --

#[test]
fn test_json_rpc_request_new() {
    let req = JsonRpcRequest::new(1, "initialize", None);
    assert_eq!(req.jsonrpc, "2.0");
    assert_eq!(req.id, 1);
    assert_eq!(req.method, "initialize");
    assert!(req.params.is_none());
}

#[test]
fn test_json_rpc_request_with_params() {
    let params = json!({"name": "test", "version": "1.0"});
    let req = JsonRpcRequest::new(42, "tools/call", Some(params.clone()));
    assert_eq!(req.id, 42);
    assert_eq!(req.method, "tools/call");
    assert_eq!(req.params.unwrap(), params);
}

#[test]
fn test_json_rpc_request_serialization() {
    let req = JsonRpcRequest::new(1, "initialize", None);
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json.get("jsonrpc").unwrap().as_str().unwrap(), "2.0");
    assert_eq!(json.get("id").unwrap().as_u64().unwrap(), 1);
    assert_eq!(json.get("method").unwrap().as_str().unwrap(), "initialize");
    // params should be omitted when None
    assert!(json.get("params").is_none());
}

#[test]
fn test_json_rpc_request_serialization_with_params() {
    let req = JsonRpcRequest::new(5, "tools/list", Some(json!({"cursor": null})));
    let json = serde_json::to_value(&req).unwrap();
    assert!(json.get("params").is_some());
}

// -- JsonRpcResponse tests --

#[test]
fn test_json_rpc_response_success() {
    let json = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {"tools": []}
    });
    let resp: JsonRpcResponse = serde_json::from_value(json).unwrap();
    assert_eq!(resp.jsonrpc, "2.0");
    assert_eq!(resp.id, Some(1));
    assert!(resp.result.is_some());
    assert!(resp.error.is_none());
}

#[test]
fn test_json_rpc_response_error() {
    let json = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "error": {
            "code": -32600,
            "message": "Invalid Request"
        }
    });
    let resp: JsonRpcResponse = serde_json::from_value(json).unwrap();
    assert!(resp.result.is_none());
    let err = resp.error.unwrap();
    assert_eq!(err.code, -32600);
    assert_eq!(err.message, "Invalid Request");
    assert!(err.data.is_none());
}

#[test]
fn test_json_rpc_response_error_with_data() {
    let json = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "error": {
            "code": -32000,
            "message": "Server error",
            "data": {"details": "out of memory"}
        }
    });
    let resp: JsonRpcResponse = serde_json::from_value(json).unwrap();
    let err = resp.error.unwrap();
    assert_eq!(err.code, -32000);
    assert!(err.data.is_some());
}

// -- McpTool tests --

#[test]
fn test_mcp_tool_deserialization() {
    let json = json!({
        "name": "read_file",
        "description": "Read a file",
        "inputSchema": {
            "type": "object",
            "properties": {
                "path": {"type": "string"}
            }
        }
    });
    let tool: McpTool = serde_json::from_value(json).unwrap();
    assert_eq!(tool.name, "read_file");
    assert_eq!(tool.description.as_deref(), Some("Read a file"));
    assert!(tool.input_schema.is_some());
}

#[test]
fn test_mcp_tool_minimal() {
    let json = json!({"name": "ping"});
    let tool: McpTool = serde_json::from_value(json).unwrap();
    assert_eq!(tool.name, "ping");
    assert!(tool.description.is_none());
    assert!(tool.input_schema.is_none());
}

#[test]
fn test_mcp_tool_serialization_roundtrip() {
    let tool = McpTool {
        name: "test_tool".to_string(),
        description: Some("A test".to_string()),
        input_schema: Some(json!({"type": "object"})),
    };
    let json = serde_json::to_value(&tool).unwrap();
    let deserialized: McpTool = serde_json::from_value(json).unwrap();
    assert_eq!(deserialized.name, "test_tool");
    assert_eq!(deserialized.description, Some("A test".to_string()));
}

// -- McpServerConfig tests --

#[test]
fn test_mcp_server_config_deserialization() {
    let json = json!({
        "name": "context7",
        "command": "npx",
        "args": ["@context7/mcp-server"],
        "env": {"NODE_ENV": "production"}
    });
    let config: McpServerConfig = serde_json::from_value(json).unwrap();
    assert_eq!(config.name, "context7");
    assert_eq!(config.command, "npx");
    assert_eq!(config.args, vec!["@context7/mcp-server"]);
    assert_eq!(config.env.get("NODE_ENV").unwrap(), "production");
}

#[test]
fn test_mcp_server_config_default_env() {
    let json = json!({
        "name": "test",
        "command": "test-cmd",
        "args": []
    });
    let config: McpServerConfig = serde_json::from_value(json).unwrap();
    assert!(config.env.is_empty());
}

#[test]
fn test_mcp_server_config_serialization_roundtrip() {
    let config = McpServerConfig {
        name: "playwright".to_string(),
        command: "npx".to_string(),
        args: vec!["@playwright/mcp-server".to_string()],
        env: std::collections::HashMap::new(),
    };
    let json = serde_json::to_value(&config).unwrap();
    let deserialized: McpServerConfig = serde_json::from_value(json).unwrap();
    assert_eq!(deserialized.name, "playwright");
    assert_eq!(deserialized.args.len(), 1);
}
