use serde::{Deserialize, Serialize};

/// JSON-RPC 2.0 request
#[derive(Debug, Serialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

impl JsonRpcRequest {
    pub fn new(id: u64, method: &str, params: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.to_string(),
            params,
        }
    }
}

/// JSON-RPC 2.0 response
#[derive(Debug, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<u64>,
    pub result: Option<serde_json::Value>,
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

/// MCP tool definition from server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "inputSchema")]
    pub input_schema: Option<serde_json::Value>,
}

/// MCP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_jsonrpc_request_new() {
        let req = JsonRpcRequest::new(1, "initialize", None);
        assert_eq!(req.jsonrpc, "2.0");
        assert_eq!(req.id, 1);
        assert_eq!(req.method, "initialize");
        assert!(req.params.is_none());
    }

    #[test]
    fn test_jsonrpc_request_with_params() {
        let params = json!({"key": "value"});
        let req = JsonRpcRequest::new(42, "tools/call", Some(params.clone()));
        assert_eq!(req.id, 42);
        assert_eq!(req.params, Some(params));
    }

    #[test]
    fn test_jsonrpc_request_serialize() {
        let req = JsonRpcRequest::new(1, "test", None);
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"method\":\"test\""));
        // params should be omitted when None
        assert!(!json.contains("\"params\""));
    }

    #[test]
    fn test_jsonrpc_request_serialize_with_params() {
        let req = JsonRpcRequest::new(1, "test", Some(json!({"foo": "bar"})));
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"params\""));
    }

    #[test]
    fn test_jsonrpc_response_deserialize() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":{"tools":[]}}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.jsonrpc, "2.0");
        assert_eq!(resp.id, Some(1));
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_jsonrpc_response_error() {
        let json =
            r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"Method not found"}}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
        assert!(resp.result.is_none());
        let err = resp.error.unwrap();
        assert_eq!(err.code, -32601);
        assert_eq!(err.message, "Method not found");
    }

    #[test]
    fn test_mcp_tool_deserialize() {
        let json = r#"{"name":"resolve","description":"Resolve a library","inputSchema":{"type":"object"}}"#;
        let tool: McpTool = serde_json::from_str(json).unwrap();
        assert_eq!(tool.name, "resolve");
        assert_eq!(tool.description, Some("Resolve a library".to_string()));
        assert!(tool.input_schema.is_some());
    }

    #[test]
    fn test_mcp_tool_optional_fields() {
        let json = r#"{"name":"simple"}"#;
        let tool: McpTool = serde_json::from_str(json).unwrap();
        assert_eq!(tool.name, "simple");
        assert!(tool.description.is_none());
        assert!(tool.input_schema.is_none());
    }

    #[test]
    fn test_mcp_server_config_serialize() {
        let config = McpServerConfig {
            name: "test".to_string(),
            command: "npx".to_string(),
            args: vec!["-y".to_string(), "@test/mcp".to_string()],
            env: std::collections::HashMap::new(),
        };
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"name\":\"test\""));
        assert!(json.contains("\"command\":\"npx\""));
    }

    #[test]
    fn test_mcp_server_config_env_default() {
        let json = r#"{"name":"test","command":"npx","args":[]}"#;
        let config: McpServerConfig = serde_json::from_str(json).unwrap();
        assert!(config.env.is_empty());
    }
}
