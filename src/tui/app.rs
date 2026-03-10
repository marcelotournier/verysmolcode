use crate::agent::loop_runner::AgentEvent;
use crate::agent::AgentLoop;
use std::sync::mpsc;
use std::thread;

#[derive(Debug, Clone)]
pub enum DisplayMessage {
    User(String),
    Assistant(String),
    ToolCall(String),
    ToolResult(String),
    Status(String),
    Error(String),
    ModelInfo(String),
}

pub struct App {
    pub input: String,
    pub cursor_pos: usize,
    pub messages: Vec<DisplayMessage>,
    pub scroll_offset: u16,
    pub should_quit: bool,
    pub is_processing: bool,
    pub status_line: String,
    pub model_name: String,
    pub rate_status: String,

    // Token tracking (cached from last TokenUpdate)
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_thinking_tokens: u64,
    pub conversation_tokens: u32,

    // Agent communication
    agent_tx: Option<mpsc::Sender<String>>,
    event_rx: Option<mpsc::Receiver<AgentEvent>>,
    done_rx: Option<mpsc::Receiver<()>>,

    // Input history
    pub input_history: Vec<String>,
    pub history_index: Option<usize>,
    pub planning_mode: bool,
}

impl App {
    pub fn new() -> Result<Self, String> {
        let mut app = Self {
            input: String::new(),
            cursor_pos: 0,
            messages: Vec::new(),
            scroll_offset: 0,
            should_quit: false,
            is_processing: false,
            status_line: String::new(),
            model_name: "Ready".to_string(),
            rate_status: String::new(),
            total_input_tokens: 0,
            total_output_tokens: 0,
            total_thinking_tokens: 0,
            conversation_tokens: 0,
            agent_tx: None,
            event_rx: None,
            done_rx: None,
            input_history: Vec::new(),
            history_index: None,
            planning_mode: false,
        };

        // Welcome message
        app.messages.push(DisplayMessage::Assistant(
            "Welcome to VerySmolCode! I'm your lightweight coding assistant powered by Gemini.\n\
             Type /help for available commands. Start typing to ask me anything!"
                .to_string(),
        ));

        // Start the agent thread
        app.start_agent()?;

        Ok(app)
    }

    fn start_agent(&mut self) -> Result<(), String> {
        let (input_tx, input_rx) = mpsc::channel::<String>();
        let (event_tx, event_rx) = mpsc::channel::<AgentEvent>();
        let (done_tx, done_rx) = mpsc::channel::<()>();

        self.agent_tx = Some(input_tx);
        self.event_rx = Some(event_rx);
        self.done_rx = Some(done_rx);

        thread::spawn(move || {
            let mut agent = match AgentLoop::new() {
                Ok(a) => a,
                Err(e) => {
                    let _ = event_tx.send(AgentEvent::Status(format!("Error: {}", e)));
                    let _ = done_tx.send(());
                    return;
                }
            };

            while let Ok(user_input) = input_rx.recv() {
                // Handle internal commands
                if user_input == "/clear" {
                    agent.clear_conversation();
                    let _ = done_tx.send(());
                    continue;
                }
                if user_input == "/plan_on" {
                    agent.set_planning_mode(true);
                    let _ = done_tx.send(());
                    continue;
                }
                if user_input == "/plan_off" {
                    agent.set_planning_mode(false);
                    let _ = done_tx.send(());
                    continue;
                }
                if user_input == "/_undo" {
                    match agent.undo() {
                        Ok(paths) => {
                            if paths.is_empty() {
                                let _ = event_tx
                                    .send(AgentEvent::Status("Nothing to undo.".to_string()));
                            } else {
                                let msg = format!(
                                    "Reverted {} file(s):\n{}",
                                    paths.len(),
                                    paths
                                        .iter()
                                        .map(|p| format!("  - {}", p))
                                        .collect::<Vec<_>>()
                                        .join("\n")
                                );
                                let _ = event_tx.send(AgentEvent::Text(msg));
                            }
                        }
                        Err(e) => {
                            let _ = event_tx.send(AgentEvent::Status(e));
                        }
                    }
                    let _ = done_tx.send(());
                    continue;
                }
                if user_input == "/_override_fast" {
                    agent.model_override = crate::agent::loop_runner::ModelOverride::Fast;
                    let _ = done_tx.send(());
                    continue;
                }
                if user_input == "/_override_smart" {
                    agent.model_override = crate::agent::loop_runner::ModelOverride::Smart;
                    let _ = done_tx.send(());
                    continue;
                }

                let event_tx_clone = event_tx.clone();
                let result = agent.process_message(&user_input, move |event| {
                    let _ = event_tx_clone.send(event);
                });

                if let Err(e) = result {
                    let _ = event_tx.send(AgentEvent::Status(format!("Error: {}", e)));
                }

                // Send rate limit status
                let _ = event_tx.send(AgentEvent::Status(format!(
                    "RATE:{}",
                    agent.rate_limit_status()
                )));

                // Warn if approaching limits
                if let Some(warning) = agent.rate_limit_warning() {
                    let _ = event_tx.send(AgentEvent::Status(format!("WARN:{}", warning)));
                }

                let _ = done_tx.send(());
            }
        });

        Ok(())
    }

    pub fn submit_input(&mut self) {
        let input = self.input.trim().to_string();
        if input.is_empty() {
            return;
        }

        // Save to history
        self.input_history.push(input.clone());
        self.history_index = None;

        // Check for slash commands
        if input.starts_with('/') {
            let response = crate::tui::commands::handle_command(&input);
            match response {
                CommandResponse::Message(msg) => {
                    self.messages.push(DisplayMessage::User(input.clone()));
                    self.messages.push(DisplayMessage::Assistant(msg));
                }
                CommandResponse::Quit => {
                    self.should_quit = true;
                }
                CommandResponse::Clear => {
                    self.messages.clear();
                    if let Some(tx) = &self.agent_tx {
                        let _ = tx.send("/clear".to_string());
                    }
                }
                CommandResponse::TogglePlan => {
                    self.planning_mode = !self.planning_mode;
                    let status = if self.planning_mode {
                        "Planning mode ON - Pro models prioritized, read-only tools"
                    } else {
                        "Planning mode OFF - normal operation resumed"
                    };
                    self.messages
                        .push(DisplayMessage::Status(status.to_string()));
                    // Notify agent thread
                    if let Some(tx) = &self.agent_tx {
                        let cmd = if self.planning_mode {
                            "/plan_on"
                        } else {
                            "/plan_off"
                        };
                        let _ = tx.send(cmd.to_string());
                    }
                }
                CommandResponse::SendToAgent(msg) => {
                    self.messages.push(DisplayMessage::User(input.clone()));
                    self.send_to_agent(&msg);
                }
                CommandResponse::ShowTokens => {
                    self.messages.push(DisplayMessage::User(input.clone()));
                    self.messages
                        .push(DisplayMessage::Assistant(self.token_summary()));
                }
                CommandResponse::Undo => {
                    self.messages.push(DisplayMessage::User(input.clone()));
                    if let Some(tx) = &self.agent_tx {
                        let _ = tx.send("/_undo".to_string());
                    }
                    self.is_processing = true;
                }
                CommandResponse::SetModelOverride(mode) => {
                    // Send override command to agent thread
                    if let Some(tx) = &self.agent_tx {
                        let _ = tx.send(format!("/_override_{}", mode));
                    }
                    let label = if mode == "fast" { "Flash/Lite" } else { "Pro" };
                    self.messages.push(DisplayMessage::Status(format!(
                        "Next message will use {} models",
                        label
                    )));
                }
            }
        } else {
            self.messages.push(DisplayMessage::User(input.clone()));
            self.send_to_agent(&input);
        }

        self.input.clear();
        self.cursor_pos = 0;
        self.scroll_to_bottom();
    }

    fn send_to_agent(&mut self, input: &str) {
        if let Some(tx) = &self.agent_tx {
            self.is_processing = true;
            self.model_name = "Connecting...".to_string();
            let _ = tx.send(input.to_string());
        }
    }

    pub fn tick(&mut self) {
        // Collect events first to avoid borrow issues
        let events: Vec<AgentEvent> = if let Some(rx) = &self.event_rx {
            let mut evts = Vec::new();
            while let Ok(event) = rx.try_recv() {
                evts.push(event);
            }
            evts
        } else {
            Vec::new()
        };

        let mut needs_scroll = false;
        for event in events {
            match event {
                AgentEvent::Text(text) => {
                    self.messages.push(DisplayMessage::Assistant(text));
                    needs_scroll = true;
                }
                AgentEvent::ToolCall { name, args } => {
                    let args_str = if let Some(obj) = args.as_object() {
                        obj.iter()
                            .map(|(k, v)| {
                                let val = match v {
                                    serde_json::Value::String(s) => {
                                        if s.len() > 60 {
                                            format!("{}...", &s[..57])
                                        } else {
                                            s.clone()
                                        }
                                    }
                                    other => {
                                        let s = other.to_string();
                                        if s.len() > 60 {
                                            format!("{}...", &s[..57])
                                        } else {
                                            s
                                        }
                                    }
                                };
                                format!("{}={}", k, val)
                            })
                            .collect::<Vec<_>>()
                            .join(", ")
                    } else {
                        args.to_string()
                    };
                    self.messages
                        .push(DisplayMessage::ToolCall(format!("{}({})", name, args_str)));
                    needs_scroll = true;
                }
                AgentEvent::ToolResult { name, result } => {
                    let summary = summarize_tool_result(&name, &result);
                    self.messages.push(DisplayMessage::ToolResult(summary));
                    needs_scroll = true;
                }
                AgentEvent::ModelSwitch(name) => {
                    self.model_name = name;
                }
                AgentEvent::TokenUpdate {
                    input,
                    output,
                    total,
                    thinking,
                } => {
                    self.total_input_tokens += input as u64;
                    self.total_output_tokens += output as u64;
                    self.total_thinking_tokens += thinking as u64;
                    self.conversation_tokens = total;
                    self.status_line = format!(
                        "In:{} Out:{} Ctx:{}",
                        self.total_input_tokens, self.total_output_tokens, total
                    );
                }
                AgentEvent::Status(s) => {
                    if let Some(rate) = s.strip_prefix("RATE:") {
                        self.rate_status = rate.to_string();
                    } else if let Some(warning) = s.strip_prefix("WARN:") {
                        self.messages
                            .push(DisplayMessage::Error(warning.to_string()));
                        needs_scroll = true;
                    } else {
                        self.messages.push(DisplayMessage::Status(s));
                        needs_scroll = true;
                    }
                }
            }
        }

        if needs_scroll {
            self.scroll_to_bottom();
        }

        // Check if agent is done
        let is_done = if let Some(rx) = &self.done_rx {
            rx.try_recv().is_ok()
        } else {
            false
        };
        if is_done {
            self.is_processing = false;
            self.model_name = "Ready".to_string();
        }
    }

    pub fn cancel_processing(&mut self) {
        self.is_processing = false;
        self.model_name = "Ready".to_string();
        self.messages
            .push(DisplayMessage::Status("Cancelled.".to_string()));
    }

    pub fn clear_screen(&mut self) {
        self.messages.clear();
        self.scroll_offset = 0;
    }

    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(3);
    }

    pub fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(3);
    }

    fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
    }

    pub fn history_up(&mut self) {
        if self.input_history.is_empty() {
            return;
        }
        let idx = match self.history_index {
            Some(i) => i.saturating_sub(1),
            None => self.input_history.len() - 1,
        };
        self.history_index = Some(idx);
        self.input = self.input_history[idx].clone();
        self.cursor_pos = self.input.len();
    }

    pub fn history_down(&mut self) {
        if let Some(idx) = self.history_index {
            if idx + 1 < self.input_history.len() {
                let new_idx = idx + 1;
                self.history_index = Some(new_idx);
                self.input = self.input_history[new_idx].clone();
                self.cursor_pos = self.input.len();
            } else {
                self.history_index = None;
                self.input.clear();
                self.cursor_pos = 0;
            }
        }
    }
}

impl App {
    pub fn token_summary(&self) -> String {
        let total = self.total_input_tokens + self.total_output_tokens;
        format!(
            "Token Usage:\n\
             \n\
             Session totals:\n\
             Input tokens:    {}\n\
             Output tokens:   {}\n\
             Thinking tokens: {}\n\
             Total tokens:    {}\n\
             \n\
             Current context: {} tokens\n\
             \n\
             Rate limits remaining:\n\
             {}",
            self.total_input_tokens,
            self.total_output_tokens,
            self.total_thinking_tokens,
            total,
            self.conversation_tokens,
            if self.rate_status.is_empty() {
                "No requests made yet".to_string()
            } else {
                self.rate_status.clone()
            }
        )
    }
}

pub enum CommandResponse {
    Message(String),
    Quit,
    Clear,
    SendToAgent(String),
    TogglePlan,
    ShowTokens,
    SetModelOverride(String), // "fast" or "smart"
    Undo,
}

fn summarize_tool_result(name: &str, result: &serde_json::Value) -> String {
    if let Some(err) = result.get("error").and_then(|v| v.as_str()) {
        return format!("[{}] Error: {}", name, err);
    }

    match name {
        "read_file" => {
            let path = result.get("path").and_then(|v| v.as_str()).unwrap_or("?");
            let truncated = result
                .get("truncated")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if truncated {
                let bytes = result
                    .get("total_bytes")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                format!("[read_file] {} ({} bytes, truncated)", path, bytes)
            } else {
                format!("[read_file] {}", path)
            }
        }
        "write_file" => {
            let path = result.get("path").and_then(|v| v.as_str()).unwrap_or("?");
            let bytes = result
                .get("bytes_written")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            format!("[write_file] {} ({} bytes)", path, bytes)
        }
        "edit_file" => {
            let path = result.get("path").and_then(|v| v.as_str()).unwrap_or("?");
            format!("[edit_file] {}", path)
        }
        "grep_search" => {
            let matches = result
                .get("total_matches")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            format!("[grep] {} matches found", matches)
        }
        "git_status" | "git_diff" | "git_log" | "git_commit" | "git_push" | "git_pull" => {
            let output = result.get("output").and_then(|v| v.as_str()).unwrap_or("");
            let summary = if output.len() > 100 {
                format!("{}...", &output[..97])
            } else {
                output.to_string()
            };
            format!("[{}] {}", name, summary)
        }
        "run_command" => {
            let success = result
                .get("success")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let exit_code = result
                .get("exit_code")
                .and_then(|v| v.as_i64())
                .unwrap_or(-1);
            if success {
                format!("[cmd] Exit code: {}", exit_code)
            } else {
                let stderr = result.get("stderr").and_then(|v| v.as_str()).unwrap_or("");
                let summary = if stderr.len() > 80 {
                    &stderr[..77]
                } else {
                    stderr
                };
                format!("[cmd] Failed ({}): {}", exit_code, summary)
            }
        }
        _ => {
            format!("[{}] Done", name)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_summarize_read_file() {
        let result = json!({"path": "/src/main.rs"});
        let summary = summarize_tool_result("read_file", &result);
        assert_eq!(summary, "[read_file] /src/main.rs");
    }

    #[test]
    fn test_summarize_read_file_truncated() {
        let result = json!({"path": "/big.rs", "truncated": true, "total_bytes": 50000});
        let summary = summarize_tool_result("read_file", &result);
        assert!(summary.contains("50000 bytes"));
        assert!(summary.contains("truncated"));
    }

    #[test]
    fn test_summarize_write_file() {
        let result = json!({"path": "/tmp/out.txt", "bytes_written": 128});
        let summary = summarize_tool_result("write_file", &result);
        assert!(summary.contains("128 bytes"));
        assert!(summary.contains("/tmp/out.txt"));
    }

    #[test]
    fn test_summarize_edit_file() {
        let result = json!({"path": "/src/lib.rs"});
        let summary = summarize_tool_result("edit_file", &result);
        assert_eq!(summary, "[edit_file] /src/lib.rs");
    }

    #[test]
    fn test_summarize_grep_search() {
        let result = json!({"total_matches": 42});
        let summary = summarize_tool_result("grep_search", &result);
        assert_eq!(summary, "[grep] 42 matches found");
    }

    #[test]
    fn test_summarize_git_status() {
        let result = json!({"output": "On branch main\nnothing to commit"});
        let summary = summarize_tool_result("git_status", &result);
        assert!(summary.contains("[git_status]"));
        assert!(summary.contains("On branch main"));
    }

    #[test]
    fn test_summarize_git_long_output() {
        let long = "x".repeat(200);
        let result = json!({"output": long});
        let summary = summarize_tool_result("git_diff", &result);
        assert!(summary.contains("..."));
        assert!(summary.len() < 200);
    }

    #[test]
    fn test_summarize_run_command_success() {
        let result = json!({"success": true, "exit_code": 0});
        let summary = summarize_tool_result("run_command", &result);
        assert_eq!(summary, "[cmd] Exit code: 0");
    }

    #[test]
    fn test_summarize_run_command_failure() {
        let result = json!({"success": false, "exit_code": 1, "stderr": "compilation error"});
        let summary = summarize_tool_result("run_command", &result);
        assert!(summary.contains("Failed"));
        assert!(summary.contains("compilation error"));
    }

    #[test]
    fn test_summarize_unknown_tool() {
        let result = json!({"anything": "here"});
        let summary = summarize_tool_result("some_mcp_tool", &result);
        assert_eq!(summary, "[some_mcp_tool] Done");
    }

    #[test]
    fn test_summarize_error() {
        let result = json!({"error": "file not found"});
        let summary = summarize_tool_result("read_file", &result);
        assert!(summary.contains("Error"));
        assert!(summary.contains("file not found"));
    }

    #[test]
    fn test_display_message_variants() {
        let msgs = vec![
            DisplayMessage::User("hello".to_string()),
            DisplayMessage::Assistant("hi".to_string()),
            DisplayMessage::ToolCall("read_file(path=/tmp)".to_string()),
            DisplayMessage::ToolResult("[read_file] /tmp".to_string()),
            DisplayMessage::Status("Ready".to_string()),
            DisplayMessage::Error("oops".to_string()),
            DisplayMessage::ModelInfo("Gemini 3 Flash".to_string()),
        ];
        assert_eq!(msgs.len(), 7);
    }

    #[test]
    fn test_command_response_variants() {
        // Just ensure all variants exist and can be constructed
        let _msg = CommandResponse::Message("hello".to_string());
        let _quit = CommandResponse::Quit;
        let _clear = CommandResponse::Clear;
        let _send = CommandResponse::SendToAgent("test".to_string());
        let _plan = CommandResponse::TogglePlan;
        let _tokens = CommandResponse::ShowTokens;
        let _override = CommandResponse::SetModelOverride("fast".to_string());
        let _undo = CommandResponse::Undo;
    }
}
