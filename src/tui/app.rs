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

    // Command autocomplete popup
    pub command_suggestions: Vec<(String, String)>,
    pub suggestion_index: Option<usize>,
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
            command_suggestions: Vec::new(),
            suggestion_index: None,
        };

        // Welcome message (rendered as styled widget in ui.rs when messages are empty)
        // No need for ASCII art in messages — the welcome screen handles it

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
                Ok(mut a) => {
                    // Report any MCP startup warnings to the user
                    for warning in a.take_startup_warnings() {
                        let _ = event_tx.send(AgentEvent::Status(warning));
                    }
                    a
                }
                Err(e) => {
                    let hint = if e.contains("GEMINI_API_KEY") {
                        format!("Error: {}. Set it with: export GEMINI_API_KEY=your_key", e)
                    } else {
                        format!("Error: {}", e)
                    };
                    let _ = event_tx.send(AgentEvent::Status(format!("WARN:{}", hint)));
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
                if user_input == "/_compact" {
                    agent.compact_now();
                    let _ = done_tx.send(());
                    continue;
                }
                if user_input == "/_todo" {
                    let display = agent.todo.to_display();
                    let _ = event_tx.send(AgentEvent::Text(display));
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
                CommandResponse::Save(filename) => {
                    self.messages.push(DisplayMessage::User(input.clone()));
                    let result = self.save_conversation(filename.as_deref());
                    self.messages.push(match result {
                        Ok(path) => {
                            DisplayMessage::Assistant(format!("Conversation saved to {}", path))
                        }
                        Err(e) => DisplayMessage::Error(format!("Save failed: {}", e)),
                    });
                }
                CommandResponse::Undo => {
                    self.messages.push(DisplayMessage::User(input.clone()));
                    if let Some(tx) = &self.agent_tx {
                        let _ = tx.send("/_undo".to_string());
                    }
                    self.is_processing = true;
                }
                CommandResponse::ShowTodo => {
                    if let Some(tx) = &self.agent_tx {
                        let _ = tx.send("/_todo".to_string());
                    }
                    self.is_processing = true;
                }
                CommandResponse::Compact => {
                    if let Some(tx) = &self.agent_tx {
                        let _ = tx.send("/_compact".to_string());
                    }
                    self.messages.push(DisplayMessage::Status(
                        "Conversation compacted to save tokens.".to_string(),
                    ));
                }
                CommandResponse::Retry => {
                    // Resend the last non-command message
                    if let Some(last) = self.last_user_message() {
                        self.messages.push(DisplayMessage::Status(
                            "Retrying last message...".to_string(),
                        ));
                        self.send_to_agent(&last);
                    } else {
                        self.messages.push(DisplayMessage::Status(
                            "No previous message to retry.".to_string(),
                        ));
                    }
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
            if tx.send(input.to_string()).is_ok() {
                self.is_processing = true;
                self.model_name = "Connecting...".to_string();
            } else {
                self.messages.push(DisplayMessage::Error(
                    "Agent is not running. Check GEMINI_API_KEY and restart.".to_string(),
                ));
            }
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
                                        if s.chars().count() > 60 {
                                            let t: String = s.chars().take(57).collect();
                                            format!("{}...", t)
                                        } else {
                                            s.clone()
                                        }
                                    }
                                    other => {
                                        let s = other.to_string();
                                        if s.chars().count() > 60 {
                                            let t: String = s.chars().take(57).collect();
                                            format!("{}...", t)
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
                AgentEvent::ToolResult {
                    name,
                    result,
                    duration_ms,
                } => {
                    let summary = if duration_ms > 0 {
                        format!(
                            "{} ({}ms)",
                            summarize_tool_result(&name, &result),
                            duration_ms
                        )
                    } else {
                        summarize_tool_result(&name, &result)
                    };
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
    /// Find the last user message that was sent to the agent (not a command)
    fn last_user_message(&self) -> Option<String> {
        self.input_history
            .iter()
            .rev()
            .find(|m| !m.starts_with('/'))
            .cloned()
    }

    pub fn update_suggestions(&mut self) {
        if self.input.starts_with('/') && !self.input.contains(' ') {
            let input = self.input.to_lowercase();
            self.command_suggestions = crate::tui::commands::COMMANDS
                .iter()
                .filter(|(cmd, _)| cmd.starts_with(&input))
                .map(|(cmd, desc)| (cmd.to_string(), desc.to_string()))
                .collect();
            // Reset selection if out of bounds
            if let Some(idx) = self.suggestion_index {
                if idx >= self.command_suggestions.len() {
                    self.suggestion_index = None;
                }
            }
        } else {
            self.command_suggestions.clear();
            self.suggestion_index = None;
        }
    }

    pub fn select_suggestion(&mut self) -> bool {
        if self.command_suggestions.is_empty() {
            return false;
        }
        let idx = self.suggestion_index.unwrap_or(0);
        if idx < self.command_suggestions.len() {
            self.input = self.command_suggestions[idx].0.clone();
            self.cursor_pos = self.input.len();
            self.command_suggestions.clear();
            self.suggestion_index = None;
            true
        } else {
            false
        }
    }

    fn save_conversation(&self, filename: Option<&str>) -> Result<String, String> {
        let name = match filename {
            Some(f) => {
                // Block path traversal
                if f.contains('/') || f.contains('\\') || f.contains("..") {
                    return Err("Filename cannot contain path separators or '..'".to_string());
                }
                f.to_string()
            }
            None => {
                let now = chrono::Local::now();
                format!("vsc-conversation-{}.md", now.format("%Y%m%d-%H%M%S"))
            }
        };

        let cwd = std::env::current_dir().unwrap_or_default();
        let path = cwd.join(&name);

        let mut output = String::from("# VerySmolCode Conversation\n\n");

        for msg in &self.messages {
            match msg {
                DisplayMessage::User(text) => {
                    output.push_str(&format!("## User\n{}\n\n", text));
                }
                DisplayMessage::Assistant(text) => {
                    output.push_str(&format!("## Assistant\n{}\n\n", text));
                }
                DisplayMessage::ToolCall(text) => {
                    output.push_str(&format!("**Tool Call:** `{}`\n\n", text));
                }
                DisplayMessage::ToolResult(text) => {
                    output.push_str(&format!("**Tool Result:** {}\n\n", text));
                }
                DisplayMessage::Status(text) => {
                    output.push_str(&format!("*Status: {}*\n\n", text));
                }
                DisplayMessage::Error(text) => {
                    output.push_str(&format!("**Error:** {}\n\n", text));
                }
                DisplayMessage::ModelInfo(text) => {
                    output.push_str(&format!("*Model: {}*\n\n", text));
                }
            }
        }

        std::fs::write(&path, &output)
            .map_err(|e| format!("Failed to write {}: {}", path.display(), e))?;
        Ok(path.display().to_string())
    }

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
    Save(Option<String>), // Optional filename
    Retry,
    Compact,
    ShowTodo,
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
            let summary = if output.chars().count() > 100 {
                let t: String = output.chars().take(97).collect();
                format!("{}...", t)
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
                let summary = if stderr.chars().count() > 80 {
                    let t: String = stderr.chars().take(77).collect();
                    format!("{}...", t)
                } else {
                    stderr.to_string()
                };
                format!("[cmd] Failed ({}): {}", exit_code, summary)
            }
        }
        "todo_update" => {
            let action = result
                .get("action")
                .and_then(|v| v.as_str())
                .unwrap_or("updated");
            let id = result.get("id").and_then(|v| v.as_u64());
            match (action, id) {
                (action, Some(id)) => format!("[todo] #{} {}", id, action),
                (_, None) => "[todo] Updated".to_string(),
            }
        }
        "find_files" => {
            let files = result
                .get("files")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            format!("[find] {} files found", files)
        }
        "list_directory" => {
            let path = result.get("path").and_then(|v| v.as_str()).unwrap_or(".");
            let entries = result
                .get("entries")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            format!("[ls] {} ({} entries)", path, entries)
        }
        "web_fetch" => {
            let url = result.get("url").and_then(|v| v.as_str()).unwrap_or("?");
            let truncated = result
                .get("truncated")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if truncated {
                format!("[web] {} (truncated)", url)
            } else {
                format!("[web] {}", url)
            }
        }
        "read_image" => {
            let path = result.get("path").and_then(|v| v.as_str()).unwrap_or("?");
            let size = result
                .get("size_bytes")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            format!("[image] {} ({} bytes)", path, size)
        }
        _ => {
            format!("[{}] Done", name)
        }
    }
}

#[cfg(test)]
impl App {
    /// Create an App without starting the agent (for unit tests)
    pub(crate) fn test_new() -> Self {
        Self {
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
            command_suggestions: Vec::new(),
            suggestion_index: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // -- App method tests --

    #[test]
    fn test_scroll_up() {
        let mut app = App::test_new();
        assert_eq!(app.scroll_offset, 0);
        app.scroll_up();
        assert_eq!(app.scroll_offset, 3);
        app.scroll_up();
        assert_eq!(app.scroll_offset, 6);
    }

    #[test]
    fn test_scroll_down() {
        let mut app = App::test_new();
        app.scroll_offset = 5;
        app.scroll_down();
        assert_eq!(app.scroll_offset, 2);
        app.scroll_down();
        assert_eq!(app.scroll_offset, 0);
        // Should not underflow
        app.scroll_down();
        assert_eq!(app.scroll_offset, 0);
    }

    #[test]
    fn test_clear_screen() {
        let mut app = App::test_new();
        app.messages.push(DisplayMessage::User("hello".to_string()));
        app.messages
            .push(DisplayMessage::Assistant("hi".to_string()));
        app.scroll_offset = 10;
        app.clear_screen();
        assert!(app.messages.is_empty());
        assert_eq!(app.scroll_offset, 0);
    }

    #[test]
    fn test_cancel_processing() {
        let mut app = App::test_new();
        app.is_processing = true;
        app.model_name = "Gemini 3 Flash".to_string();
        app.cancel_processing();
        assert!(!app.is_processing);
        assert_eq!(app.model_name, "Ready");
        assert!(matches!(
            app.messages.last(),
            Some(DisplayMessage::Status(_))
        ));
    }

    #[test]
    fn test_history_up_empty() {
        let mut app = App::test_new();
        app.history_up();
        assert!(app.history_index.is_none());
        assert!(app.input.is_empty());
    }

    #[test]
    fn test_history_up_single() {
        let mut app = App::test_new();
        app.input_history.push("first".to_string());
        app.history_up();
        assert_eq!(app.history_index, Some(0));
        assert_eq!(app.input, "first");
        assert_eq!(app.cursor_pos, 5);
    }

    #[test]
    fn test_history_up_multiple() {
        let mut app = App::test_new();
        app.input_history.push("first".to_string());
        app.input_history.push("second".to_string());
        app.input_history.push("third".to_string());

        app.history_up(); // Should go to last (index 2)
        assert_eq!(app.input, "third");
        app.history_up(); // index 1
        assert_eq!(app.input, "second");
        app.history_up(); // index 0
        assert_eq!(app.input, "first");
        app.history_up(); // saturates at 0
        assert_eq!(app.input, "first");
    }

    #[test]
    fn test_history_down_no_history() {
        let mut app = App::test_new();
        app.history_down();
        assert!(app.history_index.is_none());
    }

    #[test]
    fn test_history_down_clears_input() {
        let mut app = App::test_new();
        app.input_history.push("first".to_string());
        app.input_history.push("second".to_string());

        app.history_up(); // "second"
        app.history_down(); // past end → clears input
        assert!(app.history_index.is_none());
        assert!(app.input.is_empty());
        assert_eq!(app.cursor_pos, 0);
    }

    #[test]
    fn test_history_up_down_cycle() {
        let mut app = App::test_new();
        app.input_history.push("a".to_string());
        app.input_history.push("b".to_string());

        app.history_up(); // "b"
        assert_eq!(app.input, "b");
        app.history_up(); // "a"
        assert_eq!(app.input, "a");
        app.history_down(); // "b"
        assert_eq!(app.input, "b");
        app.history_down(); // clears
        assert!(app.input.is_empty());
    }

    #[test]
    fn test_update_suggestions_slash() {
        let mut app = App::test_new();
        app.input = "/h".to_string();
        app.update_suggestions();
        assert_eq!(app.command_suggestions.len(), 1);
        assert_eq!(app.command_suggestions[0].0, "/help");
    }

    #[test]
    fn test_update_suggestions_clears_on_non_slash() {
        let mut app = App::test_new();
        app.input = "/h".to_string();
        app.update_suggestions();
        assert!(!app.command_suggestions.is_empty());

        app.input = "hello".to_string();
        app.update_suggestions();
        assert!(app.command_suggestions.is_empty());
    }

    #[test]
    fn test_update_suggestions_clears_on_space() {
        let mut app = App::test_new();
        app.input = "/help something".to_string();
        app.update_suggestions();
        assert!(app.command_suggestions.is_empty());
    }

    #[test]
    fn test_select_suggestion_empty() {
        let mut app = App::test_new();
        assert!(!app.select_suggestion());
    }

    #[test]
    fn test_select_suggestion_picks_first() {
        let mut app = App::test_new();
        app.input = "/h".to_string();
        app.update_suggestions();
        assert!(app.select_suggestion());
        assert_eq!(app.input, "/help");
        assert_eq!(app.cursor_pos, 5);
        assert!(app.command_suggestions.is_empty());
    }

    #[test]
    fn test_select_suggestion_picks_selected() {
        let mut app = App::test_new();
        app.input = "/mc".to_string();
        app.update_suggestions();
        assert!(app.command_suggestions.len() >= 2);
        app.suggestion_index = Some(1);
        let expected = app.command_suggestions[1].0.clone();
        assert!(app.select_suggestion());
        assert_eq!(app.input, expected);
    }

    #[test]
    fn test_last_user_message() {
        let mut app = App::test_new();
        app.input_history.push("/help".to_string());
        app.input_history.push("fix the bug".to_string());
        app.input_history.push("/status".to_string());
        assert_eq!(app.last_user_message(), Some("fix the bug".to_string()));
    }

    #[test]
    fn test_last_user_message_none() {
        let mut app = App::test_new();
        app.input_history.push("/help".to_string());
        app.input_history.push("/status".to_string());
        assert_eq!(app.last_user_message(), None);
    }

    #[test]
    fn test_token_summary() {
        let mut app = App::test_new();
        app.total_input_tokens = 1000;
        app.total_output_tokens = 500;
        app.total_thinking_tokens = 200;
        app.conversation_tokens = 3000;
        let summary = app.token_summary();
        assert!(summary.contains("1000"));
        assert!(summary.contains("500"));
    }

    #[test]
    fn test_save_conversation_path_traversal_slash() {
        let app = App::test_new();
        let result = app.save_conversation(Some("../evil.md"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("path separators"));
    }

    #[test]
    fn test_save_conversation_path_traversal_backslash() {
        let app = App::test_new();
        let result = app.save_conversation(Some("..\\evil.md"));
        assert!(result.is_err());
    }

    #[test]
    fn test_save_conversation_path_traversal_dotdot() {
        let app = App::test_new();
        let result = app.save_conversation(Some("..test.md"));
        assert!(result.is_err());
    }

    #[test]
    fn test_save_conversation_writes_file() {
        let mut app = App::test_new();
        app.messages.push(DisplayMessage::User("hello".to_string()));
        app.messages
            .push(DisplayMessage::Assistant("hi there".to_string()));

        let result = app.save_conversation(Some("vsc-test-save.md"));
        assert!(result.is_ok());
        let path = result.unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("# VerySmolCode Conversation"));
        assert!(content.contains("hello"));
        assert!(content.contains("hi there"));
        // Clean up
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_save_conversation_default_filename() {
        let app = App::test_new();
        let result = app.save_conversation(None);
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.contains("vsc-conversation-"));
        assert!(path.ends_with(".md"));
        // Clean up
        let _ = std::fs::remove_file(&path);
    }

    // -- summarize_tool_result tests --

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
        let _save = CommandResponse::Save(None);
        let _save_file = CommandResponse::Save(Some("test.md".to_string()));
        let _retry = CommandResponse::Retry;
        let _compact = CommandResponse::Compact;
        let _todo = CommandResponse::ShowTodo;
    }

    #[test]
    fn test_suggestion_updates_on_slash() {
        // Create a minimal app-like state to test update_suggestions
        // We can't call App::new() (needs GEMINI_API_KEY), so test the logic directly
        let suggestions: Vec<(String, String)> = crate::tui::commands::COMMANDS
            .iter()
            .filter(|(cmd, _)| cmd.starts_with("/h"))
            .map(|(cmd, desc)| (cmd.to_string(), desc.to_string()))
            .collect();
        assert_eq!(suggestions.len(), 1); // /help
        assert_eq!(suggestions[0].0, "/help");
    }

    #[test]
    fn test_suggestion_all_commands_on_slash() {
        let suggestions: Vec<(String, String)> = crate::tui::commands::COMMANDS
            .iter()
            .filter(|(cmd, _)| cmd.starts_with("/"))
            .map(|(cmd, desc)| (cmd.to_string(), desc.to_string()))
            .collect();
        assert!(suggestions.len() >= 15); // All commands
    }

    #[test]
    fn test_suggestion_filter_prefix() {
        let suggestions: Vec<(String, String)> = crate::tui::commands::COMMANDS
            .iter()
            .filter(|(cmd, _)| cmd.starts_with("/mc"))
            .map(|(cmd, desc)| (cmd.to_string(), desc.to_string()))
            .collect();
        assert_eq!(suggestions.len(), 3); // /mcp, /mcp-add, /mcp-rm
    }

    #[test]
    fn test_suggestion_no_match() {
        let suggestions: Vec<(String, String)> = crate::tui::commands::COMMANDS
            .iter()
            .filter(|(cmd, _)| cmd.starts_with("/zzz"))
            .map(|(cmd, desc)| (cmd.to_string(), desc.to_string()))
            .collect();
        assert!(suggestions.is_empty());
    }

    #[test]
    fn test_summarize_todo_update() {
        let result = json!({"ok": true, "id": 3, "action": "added"});
        let summary = summarize_tool_result("todo_update", &result);
        assert_eq!(summary, "[todo] #3 added");
    }

    #[test]
    fn test_summarize_todo_update_no_id() {
        let result = json!({"tasks": "list of tasks"});
        let summary = summarize_tool_result("todo_update", &result);
        assert_eq!(summary, "[todo] Updated");
    }

    #[test]
    fn test_summarize_find_files() {
        let result = json!({"files": ["a.rs", "b.rs", "c.rs"]});
        let summary = summarize_tool_result("find_files", &result);
        assert_eq!(summary, "[find] 3 files found");
    }

    #[test]
    fn test_summarize_list_directory() {
        let result = json!({"path": "/src", "entries": [{"name": "main.rs"}, {"name": "lib.rs"}]});
        let summary = summarize_tool_result("list_directory", &result);
        assert_eq!(summary, "[ls] /src (2 entries)");
    }

    #[test]
    fn test_summarize_web_fetch() {
        let result = json!({"url": "https://example.com", "content": "Hello"});
        let summary = summarize_tool_result("web_fetch", &result);
        assert_eq!(summary, "[web] https://example.com");
    }

    #[test]
    fn test_summarize_web_fetch_truncated() {
        let result = json!({"url": "https://big.com", "truncated": true});
        let summary = summarize_tool_result("web_fetch", &result);
        assert!(summary.contains("truncated"));
    }

    #[test]
    fn test_summarize_read_image() {
        let result = json!({"path": "/img/photo.png", "size_bytes": 1024});
        let summary = summarize_tool_result("read_image", &result);
        assert_eq!(summary, "[image] /img/photo.png (1024 bytes)");
    }
}
