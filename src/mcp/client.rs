use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};

use crate::mcp::types::*;

/// MCP client that communicates with a server via stdio
pub struct McpClient {
    process: Child,
    name: String,
    request_id: u64,
    pub tools: Vec<McpTool>,
    /// Last stderr output captured from the server (for diagnostics)
    last_stderr: std::sync::Arc<std::sync::Mutex<String>>,
    _stderr_thread: Option<std::thread::JoinHandle<()>>,
}

impl McpClient {
    /// Start an MCP server and initialize the connection
    pub fn start(config: &McpServerConfig) -> Result<Self, String> {
        let mut cmd = Command::new(&config.command);
        cmd.args(&config.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Set environment variables
        for (key, value) in &config.env {
            cmd.env(key, value);
        }

        let mut process = cmd
            .spawn()
            .map_err(|e| format!("Failed to start MCP server '{}': {}", config.name, e))?;

        // Drain stderr in a background thread to prevent pipe buffer deadlock
        let last_stderr = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        let stderr_buf = last_stderr.clone();
        let stderr_thread = process.stderr.take().map(|stderr| {
            std::thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines().map_while(Result::ok) {
                    if let Ok(mut buf) = stderr_buf.lock() {
                        // Keep only last 2KB of stderr to avoid memory growth
                        if buf.len() > 2048 {
                            buf.clear();
                        }
                        buf.push_str(&line);
                        buf.push('\n');
                    }
                }
            })
        });

        let mut client = Self {
            process,
            name: config.name.clone(),
            request_id: 0,
            tools: Vec::new(),
            last_stderr,
            _stderr_thread: stderr_thread,
        };

        // Initialize the connection
        client.initialize()?;

        // List available tools
        client.list_tools()?;

        Ok(client)
    }

    fn next_id(&mut self) -> u64 {
        self.request_id += 1;
        self.request_id
    }

    fn send_request(&mut self, request: &JsonRpcRequest) -> Result<JsonRpcResponse, String> {
        let stdin = self
            .process
            .stdin
            .as_mut()
            .ok_or("Failed to access stdin")?;

        let json = serde_json::to_string(request).map_err(|e| format!("Serialize error: {}", e))?;

        stdin
            .write_all(json.as_bytes())
            .map_err(|e| format!("Write error: {}", e))?;
        stdin
            .write_all(b"\n")
            .map_err(|e| format!("Write newline error: {}", e))?;
        stdin.flush().map_err(|e| format!("Flush error: {}", e))?;

        // Read response
        let stdout = self
            .process
            .stdout
            .as_mut()
            .ok_or("Failed to access stdout")?;
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();

        // Read lines until we get a valid JSON-RPC response
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => {
                    let stderr_info = self
                        .last_stderr
                        .lock()
                        .ok()
                        .filter(|s| !s.is_empty())
                        .map(|s| format!(" (stderr: {})", s.trim()))
                        .unwrap_or_default();
                    return Err(format!("MCP server closed connection{}", stderr_info));
                }
                Ok(_) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    match serde_json::from_str::<JsonRpcResponse>(trimmed) {
                        Ok(resp) => return Ok(resp),
                        Err(_) => continue, // Skip non-JSON lines (notifications, etc.)
                    }
                }
                Err(e) => return Err(format!("Read error: {}", e)),
            }
        }
    }

    fn initialize(&mut self) -> Result<(), String> {
        let id = self.next_id();
        let request = JsonRpcRequest::new(
            id,
            "initialize",
            Some(serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "verysmolcode",
                    "version": env!("CARGO_PKG_VERSION")
                }
            })),
        );

        let response = self.send_request(&request)?;
        if let Some(err) = response.error {
            return Err(format!("MCP init error: {}", err.message));
        }

        // Send initialized notification
        let notif = JsonRpcRequest::new(self.next_id(), "notifications/initialized", None);
        // Send but don't wait for response (it's a notification)
        if let Some(stdin) = self.process.stdin.as_mut() {
            let json = serde_json::to_string(&notif).unwrap_or_default();
            let _ = stdin.write_all(json.as_bytes());
            let _ = stdin.write_all(b"\n");
            let _ = stdin.flush();
        }

        Ok(())
    }

    fn list_tools(&mut self) -> Result<(), String> {
        let id = self.next_id();
        let request = JsonRpcRequest::new(id, "tools/list", None);

        let response = self.send_request(&request)?;
        if let Some(err) = response.error {
            return Err(format!("MCP list tools error: {}", err.message));
        }

        if let Some(result) = response.result {
            if let Some(tools) = result.get("tools").and_then(|t| t.as_array()) {
                self.tools = tools
                    .iter()
                    .filter_map(|t| serde_json::from_value::<McpTool>(t.clone()).ok())
                    .collect();
            }
        }

        Ok(())
    }

    /// Call a tool on the MCP server
    pub fn call_tool(
        &mut self,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let id = self.next_id();
        let request = JsonRpcRequest::new(
            id,
            "tools/call",
            Some(serde_json::json!({
                "name": tool_name,
                "arguments": arguments
            })),
        );

        let response = self.send_request(&request)?;
        if let Some(err) = response.error {
            return Err(format!("MCP tool call error: {}", err.message));
        }

        response
            .result
            .ok_or_else(|| "No result from MCP tool call".to_string())
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        // Try to gracefully shut down the server
        let _ = self.process.kill();
    }
}
