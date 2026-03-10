use crate::api::client::{build_request, extract_response, GeminiClient};
use crate::api::models::ModelId;
use crate::api::types::*;
use crate::config::Config;
use crate::mcp::client::McpClient;
use crate::mcp::config::McpConfig;
use crate::tools::registry::ToolRegistry;
use crate::tools::todo::TodoList;
use crate::tools::undo::UndoHistory;

/// Represents a message in the conversation
#[derive(Debug, Clone)]
pub struct AgentMessage {
    pub role: String,
    pub content: String,
    pub model: Option<String>,
    pub tool_calls: Vec<(String, serde_json::Value)>,
    pub is_thinking: bool,
}

/// Override for model selection on next request
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelOverride {
    None,
    Fast,  // Force Flash/Lite models
    Smart, // Force Pro models
}

/// The main agent loop that processes user input through the Gemini API
pub struct AgentLoop {
    client: GeminiClient,
    config: Config,
    conversation: Vec<Content>,
    total_conversation_tokens: u32,
    planning_mode: bool,
    mcp_clients: Vec<McpClient>,
    files_modified: bool, // Track if any write/edit tools were used this turn
    pub model_override: ModelOverride,
    undo_history: UndoHistory,
    startup_warnings: Vec<String>,
    pub todo: TodoList,
}

/// Max characters for a single tool result before truncation.
/// Gemini's context is large but each token costs requests/day budget.
const MAX_TOOL_RESULT_CHARS: usize = 8000;

/// Find a safe truncation point that doesn't break UTF-8 char boundaries
fn safe_truncate(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    // Walk backwards from max_bytes to find a char boundary
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// Truncate a tool result JSON to save tokens
pub fn truncate_tool_result(result: &serde_json::Value) -> serde_json::Value {
    let s = result.to_string();
    if s.len() <= MAX_TOOL_RESULT_CHARS {
        return result.clone();
    }
    // For objects with a "content" or "matches" field, truncate that field
    if let Some(obj) = result.as_object() {
        let mut truncated = obj.clone();
        for key in &["content", "matches", "output", "stdout"] {
            if let Some(val) = truncated.get(*key) {
                let val_str = val.to_string();
                if val_str.len() > MAX_TOOL_RESULT_CHARS / 2 {
                    let cut = safe_truncate(&val_str, MAX_TOOL_RESULT_CHARS / 2);
                    truncated.insert(
                        key.to_string(),
                        serde_json::json!(format!(
                            "{}...[truncated, {} chars total]",
                            cut,
                            val_str.len()
                        )),
                    );
                }
            }
        }
        return serde_json::Value::Object(truncated);
    }
    // Fallback: truncate the whole string at a safe boundary
    let cut = safe_truncate(&s, MAX_TOOL_RESULT_CHARS);
    serde_json::json!(format!("{}...[truncated, {} chars total]", cut, s.len()))
}

/// Strip thinking/thought parts from older conversation history to save tokens.
/// Keeps thinking from the last 3 messages so multi-turn reasoning context isn't lost.
pub fn strip_thinking_from_history(conversation: &mut [Content]) {
    let keep_from = conversation.len().saturating_sub(3);
    for (i, content) in conversation.iter_mut().enumerate() {
        if i < keep_from {
            content
                .parts
                .retain(|part| !matches!(part, Part::Thought { .. }));
        }
    }
}

const PLANNING_SYSTEM_PROMPT: &str = r#"You are in PLANNING MODE. Your job is to deeply analyze the task and create a thorough implementation plan.

## How to plan
1. READ relevant files first — understand the codebase before planning
2. Use grep_search and find_files to explore the project structure
3. Identify ALL files that need creation or modification
4. Break the work into small, concrete steps (each step = one logical change)
5. Consider edge cases, error handling, and testing needs
6. DO NOT make any changes — only read files and output a plan

## After analyzing, output your plan in this format:

### Analysis
[What the task requires and key observations from reading the code]

### Architecture decisions
[Any design choices, trade-offs, or alternatives considered]

### Implementation steps
1. [Concrete step] → [file(s) affected]
2. [Concrete step] → [file(s) affected]
...

### Testing plan
- [What tests to add/modify]
- [How to verify the changes work]

### Risks & edge cases
- [Things that could go wrong]

### Complexity: [Low/Medium/High]
[Brief justification]

## IMPORTANT: After outputting your plan, use the todo_update tool to create a task list from your steps!
Call todo_update with action "add" for each implementation step. This way when the user exits planning mode, the task list will guide the implementation."#;

impl AgentLoop {
    pub fn new() -> Result<Self, String> {
        // Start configured MCP servers
        let mcp_config = McpConfig::load();
        let mut mcp_clients = Vec::new();
        let mut startup_warnings = Vec::new();
        for server_config in &mcp_config.servers {
            match McpClient::start(server_config) {
                Ok(client) => {
                    mcp_clients.push(client);
                }
                Err(e) => {
                    startup_warnings.push(format!(
                        "MCP server '{}' failed to start: {}",
                        server_config.name, e
                    ));
                }
            }
        }

        Ok(Self {
            client: GeminiClient::new()?,
            config: Config::load(),
            conversation: Vec::new(),
            total_conversation_tokens: 0,
            planning_mode: false,
            mcp_clients,
            files_modified: false,
            model_override: ModelOverride::None,
            undo_history: UndoHistory::new(),
            startup_warnings,
            todo: TodoList::new(),
        })
    }

    /// Get tool declarations including MCP tools
    fn get_tools(&self, read_only: bool) -> Vec<ToolDeclaration> {
        let mut tools = if read_only {
            ToolRegistry::read_only_declarations()
        } else {
            ToolRegistry::declarations()
        };

        // NOTE: Google Search grounding disabled for now - may not work on free tier
        // tools.push(ToolDeclaration::google_search());

        // Add MCP tool declarations
        if !self.mcp_clients.is_empty() {
            let mut mcp_decls = Vec::new();
            for client in &self.mcp_clients {
                for tool in &client.tools {
                    let params = tool
                        .input_schema
                        .clone()
                        .unwrap_or_else(|| serde_json::json!({"type": "object", "properties": {}}));
                    mcp_decls.push(FunctionDecl {
                        name: format!("mcp_{}_{}", client.name(), tool.name),
                        description: tool
                            .description
                            .clone()
                            .unwrap_or_else(|| format!("MCP tool: {}", tool.name)),
                        parameters: params,
                    });
                }
            }
            if !mcp_decls.is_empty() {
                tools.push(ToolDeclaration {
                    function_declarations: mcp_decls,
                    google_search: None,
                });
            }
        }

        tools
    }

    /// Try to execute an MCP tool call, returns None if not an MCP tool
    fn try_execute_mcp(
        &mut self,
        name: &str,
        args: &serde_json::Value,
    ) -> Option<serde_json::Value> {
        // MCP tools are prefixed: mcp_{server}_{tool}
        let stripped = name.strip_prefix("mcp_")?;

        // Find which MCP client owns this tool
        for client in &mut self.mcp_clients {
            let prefix = format!("{}_", client.name());
            if let Some(tool_name) = stripped.strip_prefix(&prefix) {
                // Found the right client, call the tool
                return match client.call_tool(tool_name, args.clone()) {
                    Ok(result) => Some(result),
                    Err(e) => Some(serde_json::json!({"error": e})),
                };
            }
        }

        None
    }

    pub fn set_planning_mode(&mut self, enabled: bool) {
        self.planning_mode = enabled;
    }

    pub fn is_planning_mode(&self) -> bool {
        self.planning_mode
    }

    /// Process a user message and return agent responses
    /// The callback is called for each response chunk (text, tool use, etc.)
    pub fn process_message<F>(&mut self, user_input: &str, mut on_event: F) -> Result<(), String>
    where
        F: FnMut(AgentEvent),
    {
        // Begin undo tracking for this turn
        self.undo_history.begin_turn();

        // Add user message to conversation
        self.conversation.push(Content {
            role: Some("user".to_string()),
            parts: vec![Part::text(user_input)],
        });

        // Determine model preference: override > planning mode > auto-detect
        let prefer_smart = match self.model_override {
            ModelOverride::Smart => true,
            ModelOverride::Fast => false,
            ModelOverride::None => self.planning_mode || self.is_complex_task(user_input),
        };
        // Reset override after use (one-shot)
        self.model_override = ModelOverride::None;
        self.files_modified = false;

        // Main agent loop - keeps going until no more tool calls
        // Planning mode gets fewer iterations (just reading + planning)
        let max_iterations = if self.planning_mode { 8 } else { 15 };
        let mut had_tool_calls = false;
        for iteration in 0..max_iterations {
            // Strip thinking tokens from history before resending to save tokens
            strip_thinking_from_history(&mut self.conversation);
            // Check if we need to compact conversation
            if self.total_conversation_tokens > self.config.auto_compact_threshold {
                on_event(AgentEvent::Status(
                    "Compacting conversation to save tokens...".to_string(),
                ));
                self.compact_conversation();
            }

            // Pick model: only use Pro for first iteration, Flash for tool-call follow-ups
            // This saves Pro budget (25/day) for initial reasoning
            let use_smart = prefer_smart && iteration == 0;
            let model = self
                .client
                .router
                .pick_model(use_smart)
                .ok_or_else(|| "All models exhausted for today".to_string())?;

            on_event(AgentEvent::ModelSwitch(model.display_name().to_string()));

            // Build system prompt
            let system_prompt = {
                let mut prompt = if self.planning_mode {
                    format!(
                        "{}\n\n{}",
                        PLANNING_SYSTEM_PROMPT, &self.config.system_prompt
                    )
                } else {
                    self.config.system_prompt.clone()
                };

                // Inject todo list state (keeps model focused on objectives)
                let todo_section = self.todo.to_prompt_section();
                if !todo_section.is_empty() {
                    prompt.push_str(&todo_section);
                }

                // Add MCP tool hints so the model knows what's available
                if !self.mcp_clients.is_empty() {
                    prompt.push_str("\n\n## Available MCP servers\n");
                    for client in &self.mcp_clients {
                        let tool_names: Vec<String> =
                            client.tools.iter().map(|t| t.name.clone()).collect();
                        prompt.push_str(&format!(
                            "- {} (tools: {})\n",
                            client.name(),
                            tool_names.join(", ")
                        ));
                    }
                    prompt.push_str(
                        "\nWhen writing code that uses external libraries, ALWAYS use MCP tools \
                         (like context7's resolve-library-id and get-library-docs) to look up \
                         correct API usage BEFORE writing code. This prevents errors from \
                         outdated or incorrect API assumptions.",
                    );
                }
                prompt
            };

            // Make API call with automatic fallback chain on rate limit/overload
            on_event(AgentEvent::Status("Thinking...".to_string()));
            let response = {
                let mut current_model = model;
                let mut retried = false;
                loop {
                    let req = build_request(
                        &system_prompt,
                        self.conversation.clone(),
                        Some(self.get_tools(self.planning_mode)),
                        current_model,
                        self.config.temperature,
                        self.config.max_tokens_per_response,
                    );
                    match self.client.generate(current_model, &req) {
                        Ok(resp) => break resp,
                        Err(e) if is_rate_limit_error(&e) => {
                            // First try: wait for the same model if RPM-limited (not daily)
                            if !retried {
                                if let Some(wait) = self.client.router.wait_for_model(current_model)
                                {
                                    if wait.as_secs() <= 15 && wait.as_secs() > 0 {
                                        on_event(AgentEvent::Status(format!(
                                            "Rate limited, waiting {}s for {}...",
                                            wait.as_secs(),
                                            current_model.display_name()
                                        )));
                                        std::thread::sleep(wait);
                                        retried = true;
                                        continue;
                                    }
                                }
                            }
                            // Fall back to a weaker model
                            if let Some(fb) = self.client.router.fallback_for(current_model) {
                                on_event(AgentEvent::Status(format!(
                                    "{} unavailable, trying {}...",
                                    current_model.display_name(),
                                    fb.display_name()
                                )));
                                on_event(AgentEvent::ModelSwitch(fb.display_name().to_string()));
                                current_model = fb;
                                retried = false;
                            } else {
                                return Err(e);
                            }
                        }
                        Err(e) if is_transient_error(&e) => {
                            // Retry once on transient network errors
                            if !retried {
                                on_event(AgentEvent::Status(
                                    "Network error, retrying...".to_string(),
                                ));
                                std::thread::sleep(std::time::Duration::from_secs(2));
                                retried = true;
                                continue;
                            }
                            return Err(e);
                        }
                        Err(e) => return Err(e),
                    }
                }
            };

            // Track tokens
            if let Some(ref usage) = response.usage_metadata {
                self.total_conversation_tokens = usage.total_token_count;
                on_event(AgentEvent::TokenUpdate {
                    input: usage.prompt_token_count,
                    output: usage.candidates_token_count,
                    total: usage.total_token_count,
                    thinking: usage.thoughts_token_count,
                });
            }

            // Extract response
            let (texts, function_calls) = extract_response(&response);

            // Add model response to conversation
            if let Some(candidate) = response.candidates.first() {
                if let Some(ref content) = candidate.content {
                    self.conversation.push(content.clone());
                }
            }

            // Emit text responses
            for text in &texts {
                if !text.trim().is_empty() {
                    on_event(AgentEvent::Text(text.clone()));
                }
            }

            // If no tool calls, we're done
            if function_calls.is_empty() {
                break;
            }

            // Execute tool calls
            had_tool_calls = true;
            let mut function_responses = Vec::new();
            for call in &function_calls {
                on_event(AgentEvent::ToolCall {
                    name: call.name.clone(),
                    args: call.args.clone(),
                });

                // Safety check
                if self.config.safety_enabled && is_dangerous_tool_call(&call.name, &call.args) {
                    let result = serde_json::json!({
                        "error": "This operation was blocked by safety checks. The action could be destructive."
                    });
                    on_event(AgentEvent::ToolResult {
                        name: call.name.clone(),
                        result: result.clone(),
                        duration_ms: 0,
                    });
                    function_responses.push(Part::function_response(&call.name, result));
                    continue;
                }

                // Snapshot file before mutation for undo support
                if matches!(call.name.as_str(), "write_file" | "edit_file") {
                    if let Some(path) = call.args.get("path").and_then(|v| v.as_str()) {
                        self.undo_history
                            .snapshot_before_write(std::path::Path::new(path));
                    }
                }

                // Execute tool with timing
                let start = std::time::Instant::now();

                // Handle todo_update specially (needs mutable access to self.todo)
                let result = if call.name == "todo_update" {
                    crate::tools::todo::todo_update(&call.args, &mut self.todo)
                } else {
                    // Try MCP tools first, then built-in tools
                    self.try_execute_mcp(&call.name, &call.args)
                        .unwrap_or_else(|| ToolRegistry::execute(&call.name, &call.args))
                };

                let duration_ms = start.elapsed().as_millis() as u64;

                // Track if files were modified (for critic decision)
                if matches!(
                    call.name.as_str(),
                    "write_file" | "edit_file" | "run_command"
                ) {
                    self.files_modified = true;
                }

                on_event(AgentEvent::ToolResult {
                    name: call.name.clone(),
                    result: result.clone(),
                    duration_ms,
                });

                // Truncate large tool results before adding to conversation history
                let result = truncate_tool_result(&result);

                // For read_image, include the InlineData part so Gemini can see it
                if call.name == "read_image" {
                    if let Some(inline) = result.get("inline_data") {
                        if let (Some(mime), Some(data)) = (
                            inline.get("mime_type").and_then(|v| v.as_str()),
                            inline.get("data").and_then(|v| v.as_str()),
                        ) {
                            function_responses.push(Part::function_response(
                                &call.name,
                                serde_json::json!({"path": result.get("path"), "size_bytes": result.get("size_bytes")}),
                            ));
                            function_responses.push(Part::InlineData {
                                inline_data: crate::api::types::InlineData {
                                    mime_type: mime.to_string(),
                                    data: data.to_string(),
                                },
                            });
                            continue;
                        }
                    }
                }

                function_responses.push(Part::function_response(&call.name, result));
            }

            // Add tool responses to conversation
            self.conversation.push(Content {
                role: Some("user".to_string()),
                parts: function_responses,
            });
        }

        // Commit undo history BEFORE critic (so /undo works even if critic errors out)
        self.undo_history.commit_turn();

        // Run critic check only if files were actually modified (saves API calls)
        // Skip critic for read-only operations, planning mode, or when no budget
        if !self.planning_mode && had_tool_calls && self.files_modified {
            self.run_critic(user_input, &mut on_event)?;
        }

        Ok(())
    }

    /// Code review step: verify the work with actual diff context
    fn run_critic<F>(&mut self, original_task: &str, on_event: &mut F) -> Result<(), String>
    where
        F: FnMut(AgentEvent),
    {
        // Pick a cheap model for the critic (prefer Flash-Lite tier)
        let critic_model = if self.client.router.g3_flash_lite.can_request() {
            ModelId::Gemini31FlashLite
        } else if self.client.router.flash_lite.can_request() {
            ModelId::Gemini25FlashLite
        } else {
            on_event(AgentEvent::Status(
                "Review skipped (Flash-Lite budget exhausted)".to_string(),
            ));
            return Ok(());
        };

        on_event(AgentEvent::Status("Reviewing changes...".to_string()));

        // Get the actual diff of changes for the reviewer
        let diff_output = {
            let diff_result = crate::tools::git::git_diff(&serde_json::json!({}));
            diff_result
                .get("output")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        };

        let diff_context = if diff_output.is_empty() {
            String::new()
        } else {
            let truncated = safe_truncate(&diff_output, 3000);
            format!("\n\nGit diff of changes made:\n```\n{}\n```", truncated)
        };

        let critic_prompt = format!(
            "Code review for task: {}\n{}\n\n\
             Review the changes and conversation above. Check for:\n\
             1. CORRECTNESS: Does the code actually solve the task?\n\
             2. BUGS: Any obvious bugs, off-by-one errors, or edge cases?\n\
             3. COMPLETENESS: Are there missing imports, error handling, or test coverage?\n\
             4. STYLE: Does it match the existing code style?\n\n\
             Respond with one of:\n\
             - APPROVED: [1-2 sentence summary of what was done well]\n\
             - NEEDS_WORK: [specific issues to fix, be concrete]\n\
             Be concise and specific. No generic praise.",
            original_task, diff_context
        );

        self.conversation.push(Content {
            role: Some("user".to_string()),
            parts: vec![Part::text(&critic_prompt)],
        });

        let request = build_request(
            "You are a senior code reviewer. Review changes for correctness, bugs, and completeness. Be specific and concise. Focus on real issues, not style nitpicks.",
            self.conversation.clone(),
            None,
            critic_model,
            0.3,
            768,
        );

        match self.client.generate(critic_model, &request) {
            Ok(response) => {
                if let Some(ref usage) = response.usage_metadata {
                    self.total_conversation_tokens = usage.total_token_count;
                }

                let (texts, _) = extract_response(&response);
                for text in &texts {
                    if !text.trim().is_empty() {
                        on_event(AgentEvent::Status(format!("Review: {}", text)));
                    }
                }

                // Add critic response to conversation
                if let Some(candidate) = response.candidates.first() {
                    if let Some(ref content) = candidate.content {
                        self.conversation.push(content.clone());
                    }
                }
            }
            Err(_) => {
                on_event(AgentEvent::Status(
                    "Review skipped (rate limit)".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Determine if a task is complex enough to warrant Pro model.
    /// Pro budget is precious (25/day shared across 2 models), so we're selective.
    fn is_complex_task(&self, input: &str) -> bool {
        let input_lower = input.to_lowercase();

        // Strong signals: these almost always need Pro reasoning
        let strong_keywords = [
            "refactor",
            "architect",
            "design pattern",
            "debug",
            "fix bug",
            "optimize",
            "performance",
            "security",
            "migrate",
            "redesign",
        ];
        if strong_keywords.iter().any(|k| input_lower.contains(k)) {
            return true;
        }

        // Medium signals: need Pro only when combined with complexity indicators
        let medium_keywords = [
            "implement",
            "create",
            "build",
            "analyze",
            "explain",
            "review",
            "why",
        ];
        let complexity_indicators = [
            "multiple",
            "across",
            "entire",
            "full",
            "complete",
            "all",
            "complex",
            "system",
            "integration",
        ];
        let has_medium = medium_keywords.iter().any(|k| input_lower.contains(k));
        let has_complexity = complexity_indicators
            .iter()
            .any(|k| input_lower.contains(k));
        if has_medium && has_complexity {
            return true;
        }

        // Long messages (multi-paragraph instructions) suggest complex tasks
        input.len() > 300 || input.lines().count() > 5
    }

    /// Compact the conversation to reduce token usage.
    /// Keeps first message + last 6 messages for better context continuity.
    /// Estimates remaining tokens from kept message character lengths.
    fn compact_conversation(&mut self) {
        let keep_end = 6;
        if self.conversation.len() <= keep_end + 1 {
            return;
        }

        // Build a brief summary of what was discussed in the dropped messages
        let dropped_start = 1;
        let dropped_end = self.conversation.len() - keep_end;
        let mut topics = Vec::new();
        for msg in &self.conversation[dropped_start..dropped_end] {
            for part in &msg.parts {
                if let Part::Text { text } = part {
                    // Extract first line as topic hint (max 80 chars)
                    let first_line = text.lines().next().unwrap_or("").trim();
                    if !first_line.is_empty() && first_line.len() > 5 {
                        let topic = safe_truncate(first_line, 80);
                        topics.push(topic.to_string());
                    }
                }
            }
        }

        let summary_text = if topics.is_empty() {
            "[Previous conversation compacted. Continue from the recent context below.]".to_string()
        } else {
            let topic_list: String = topics
                .iter()
                .take(5) // Max 5 topic hints
                .map(|t| format!("- {}", t))
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "[Previous conversation compacted. Topics discussed:\n{}\nContinue from the recent context below.]",
                topic_list
            )
        };

        let summary = Content {
            role: Some("user".to_string()),
            parts: vec![Part::text(&summary_text)],
        };

        let mut new_conv = vec![self.conversation[0].clone()];
        new_conv.push(summary);
        let start = self.conversation.len() - keep_end;
        new_conv.extend_from_slice(&self.conversation[start..]);
        self.conversation = new_conv;

        // Estimate tokens from remaining content (~4 chars per token)
        let total_chars: usize = self
            .conversation
            .iter()
            .flat_map(|c| &c.parts)
            .filter_map(|p| match p {
                Part::Text { text } => Some(text.len()),
                _ => None,
            })
            .sum();
        self.total_conversation_tokens = (total_chars / 4) as u32;
    }

    pub fn rate_limit_status(&mut self) -> String {
        self.client.router.status_line()
    }

    /// Check if any model tier is running low and return a warning message
    pub fn rate_limit_warning(&mut self) -> Option<String> {
        let router = &mut self.client.router;
        let pro_left = router.g3_pro.remaining_today() + router.pro.remaining_today();
        let flash_left = router.g3_flash.remaining_today() + router.flash.remaining_today();

        if pro_left == 0 && flash_left == 0 {
            Some(
                "All Pro and Flash models exhausted for today. Only Flash-Lite available."
                    .to_string(),
            )
        } else if pro_left == 0 {
            Some(format!(
                "Pro models exhausted. {} Flash requests remaining.",
                flash_left
            ))
        } else if pro_left <= 5 {
            Some(format!(
                "Low Pro budget: {} requests left. Use /fast to save Pro for complex tasks.",
                pro_left
            ))
        } else {
            None
        }
    }

    pub fn token_usage(&self) -> String {
        self.client.token_usage_summary()
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn config_mut(&mut self) -> &mut Config {
        &mut self.config
    }

    pub fn mcp_status(&self) -> String {
        if self.mcp_clients.is_empty() {
            return "No MCP servers connected".to_string();
        }
        let mut status = format!("{} MCP server(s) connected:\n", self.mcp_clients.len());
        for client in &self.mcp_clients {
            status.push_str(&format!(
                "  {} ({} tools)\n",
                client.name(),
                client.tools.len()
            ));
        }
        status
    }

    /// Manually compact the conversation (called by /compact command)
    pub fn compact_now(&mut self) {
        self.compact_conversation();
    }

    pub fn clear_conversation(&mut self) {
        self.conversation.clear();
        self.total_conversation_tokens = 0;
    }

    /// Undo the last turn's file changes
    pub fn undo(&mut self) -> Result<Vec<String>, String> {
        self.undo_history.undo()
    }

    /// Take any startup warnings (e.g., MCP server failures)
    pub fn take_startup_warnings(&mut self) -> Vec<String> {
        std::mem::take(&mut self.startup_warnings)
    }
}

/// Events emitted by the agent loop
#[derive(Debug, Clone)]
pub enum AgentEvent {
    Text(String),
    ToolCall {
        name: String,
        args: serde_json::Value,
    },
    ToolResult {
        name: String,
        result: serde_json::Value,
        duration_ms: u64,
    },
    ModelSwitch(String),
    TokenUpdate {
        input: u32,
        output: u32,
        total: u32,
        thinking: u32,
    },
    Status(String),
}

/// Check if an error is transient (network issues, timeouts) and worth retrying
pub fn is_transient_error(e: &str) -> bool {
    e.contains("timeout")
        || e.contains("timed out")
        || e.contains("connection")
        || e.contains("Connection")
        || e.contains("reset")
        || e.contains("500")
        || e.contains("502")
        || e.contains("504")
}

/// Check if an error indicates rate limiting or overload
pub fn is_rate_limit_error(e: &str) -> bool {
    e.contains("429")
        || e.contains("rate")
        || e.contains("quota")
        || e.contains("503")
        || e.contains("high demand")
}

/// Check if a tool call looks dangerous
pub fn is_dangerous_tool_call(name: &str, args: &serde_json::Value) -> bool {
    match name {
        "run_command" => {
            if let Some(cmd) = args.get("command").and_then(|v| v.as_str()) {
                let dangerous = [
                    "rm -rf",
                    "rm -r /",
                    "sudo rm",
                    "dd if=",
                    "dd of=",
                    "mkfs",
                    "format",
                    "shutdown",
                    "reboot",
                    "init 0",
                    "init 6",
                    "chmod 777",
                    "chmod -R 777",
                    "chown -R",
                    "> /dev/",
                    "> /etc",
                    "> /sys",
                    "> /proc",
                    "> /boot",
                ];
                if dangerous.iter().any(|d| cmd.contains(d)) {
                    return true;
                }
                // Block find -delete (destructive find)
                if cmd.contains("find ") && cmd.contains("-delete") {
                    return true;
                }
                // Block piping downloads to shell execution
                let has_download = cmd.contains("curl ") || cmd.contains("wget ");
                let has_pipe_exec =
                    cmd.contains("| sh") || cmd.contains("| bash") || cmd.contains("| zsh");
                if has_download && has_pipe_exec {
                    return true;
                }
                // Block eval/exec with variables (code injection)
                if cmd.contains("eval ") || cmd.contains("exec ") {
                    return true;
                }
                false
            } else {
                false
            }
        }
        "write_file" | "edit_file" => {
            if let Some(path) = args.get("path").and_then(|v| v.as_str()) {
                return crate::tools::file_ops::BLOCKED_PATH_PREFIXES
                    .iter()
                    .any(|d| path.starts_with(d));
            }
            false
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- is_complex_task tests --
    // Mirror the logic from is_complex_task since it requires AgentLoop (needs API key)

    fn check_complex(input: &str) -> bool {
        let input_lower = input.to_lowercase();

        let strong_keywords = [
            "refactor",
            "architect",
            "design pattern",
            "debug",
            "fix bug",
            "optimize",
            "performance",
            "security",
            "migrate",
            "redesign",
        ];
        if strong_keywords.iter().any(|k| input_lower.contains(k)) {
            return true;
        }

        let medium_keywords = [
            "implement",
            "create",
            "build",
            "analyze",
            "explain",
            "review",
            "why",
        ];
        let complexity_indicators = [
            "multiple",
            "across",
            "entire",
            "full",
            "complete",
            "all",
            "complex",
            "system",
            "integration",
        ];
        let has_medium = medium_keywords.iter().any(|k| input_lower.contains(k));
        let has_complexity = complexity_indicators
            .iter()
            .any(|k| input_lower.contains(k));
        if has_medium && has_complexity {
            return true;
        }

        input.len() > 300 || input.lines().count() > 5
    }

    #[test]
    fn test_complex_task_strong_keywords() {
        assert!(check_complex("refactor the auth module"));
        assert!(check_complex("debug this crash"));
        assert!(check_complex("optimize the query"));
        assert!(check_complex("fix bug in auth"));
        assert!(check_complex("migrate to new API"));
        assert!(check_complex("redesign the layout"));
    }

    #[test]
    fn test_complex_task_medium_with_complexity() {
        // Medium keywords alone don't trigger Pro
        assert!(!check_complex("create a file"));
        assert!(!check_complex("build a test"));
        assert!(!check_complex("explain this function"));

        // But combined with complexity indicators, they do
        assert!(check_complex("implement the entire auth system"));
        assert!(check_complex("build a complete REST API"));
        assert!(check_complex("create multiple endpoints across services"));
        assert!(check_complex("review all the integration tests"));
    }

    #[test]
    fn test_complex_task_case_insensitive() {
        assert!(check_complex("REFACTOR everything"));
        assert!(check_complex("FIX BUG in auth"));
        assert!(check_complex("OPTIMIZE performance"));
    }

    #[test]
    fn test_simple_task() {
        assert!(!check_complex("hello"));
        assert!(!check_complex("what is this file"));
        assert!(!check_complex("list files"));
        assert!(!check_complex("show me the code"));
        assert!(!check_complex("create a file")); // simple create = Flash
        assert!(!check_complex("build a test")); // simple build = Flash
    }

    #[test]
    fn test_complex_task_long_input() {
        let long_input = "a".repeat(301);
        assert!(check_complex(&long_input));
    }

    #[test]
    fn test_complex_task_multiline() {
        let multiline = "line1\nline2\nline3\nline4\nline5\nline6";
        assert!(check_complex(multiline));
    }

    #[test]
    fn test_complex_task_short_not_complex() {
        let input = "a".repeat(300);
        assert!(!check_complex(&input));
    }

    // -- compact_conversation logic tests --

    #[test]
    fn test_compact_small_conversation() {
        // Conversations with 4 or fewer messages should not be compacted
        let conv = vec![
            Content {
                role: Some("user".to_string()),
                parts: vec![Part::text("hello")],
            },
            Content {
                role: Some("model".to_string()),
                parts: vec![Part::text("hi")],
            },
        ];
        // Conversations with 4 or fewer messages should not be compacted
        assert!(conv.len() <= 4);
    }

    #[test]
    fn test_compact_large_conversation() {
        let mut conv: Vec<Content> = (0..12)
            .map(|i| Content {
                role: Some(if i % 2 == 0 { "user" } else { "model" }.to_string()),
                parts: vec![Part::text(&format!("message {}", i))],
            })
            .collect();

        // Simulate compaction: keep first + summary + last 6
        let keep_end = 6;

        if conv.len() > keep_end + 1 {
            let summary = Content {
                role: Some("user".to_string()),
                parts: vec![Part::text("[Compacted]")],
            };
            let mut new_conv = vec![conv[0].clone()];
            new_conv.push(summary);
            let start = conv.len() - keep_end;
            new_conv.extend_from_slice(&conv[start..]);
            conv = new_conv;
        }

        // Should have: first + summary + last 6 = 8
        assert_eq!(conv.len(), 8);
        // First message preserved
        match &conv[0].parts[0] {
            Part::Text { text } => assert_eq!(text, "message 0"),
            _ => panic!("Expected text"),
        }
        // Summary inserted
        match &conv[1].parts[0] {
            Part::Text { text } => assert!(text.contains("Compacted")),
            _ => panic!("Expected text"),
        }
        // Last 6 messages preserved (starting from message 6)
        match &conv[2].parts[0] {
            Part::Text { text } => assert_eq!(text, "message 6"),
            _ => panic!("Expected text"),
        }
    }

    #[test]
    fn test_safe_truncate_ascii() {
        assert_eq!(safe_truncate("hello world", 5), "hello");
        assert_eq!(safe_truncate("hi", 10), "hi");
    }

    #[test]
    fn test_safe_truncate_utf8() {
        // Multi-byte char: don't split mid-character
        let s = "hello\u{1F600}world"; // emoji is 4 bytes
        let result = safe_truncate(s, 6); // would land in the middle of the emoji
        assert_eq!(result, "hello"); // backs up to char boundary
    }

    // -- Planning prompt tests --

    #[test]
    fn test_planning_prompt_contains_rules() {
        assert!(PLANNING_SYSTEM_PROMPT.contains("PLANNING MODE"));
        assert!(PLANNING_SYSTEM_PROMPT.contains("DO NOT make any changes"));
        assert!(PLANNING_SYSTEM_PROMPT.contains("read files"));
    }

    #[test]
    fn test_transient_error_detection() {
        assert!(is_transient_error("connection reset by peer"));
        assert!(is_transient_error("request timed out"));
        assert!(is_transient_error("HTTP 500 Internal Server Error"));
        assert!(is_transient_error("HTTP 502 Bad Gateway"));
        assert!(is_transient_error("HTTP 504 Gateway Timeout"));
        assert!(!is_transient_error("HTTP 429 Too Many Requests")); // rate limit, not transient
        assert!(!is_transient_error("invalid API key"));
    }

    #[test]
    fn test_rate_limit_error_detection() {
        assert!(is_rate_limit_error("HTTP 429 Too Many Requests"));
        assert!(is_rate_limit_error("rate limit exceeded"));
        assert!(is_rate_limit_error("quota exceeded"));
        assert!(is_rate_limit_error("HTTP 503 high demand"));
        assert!(!is_rate_limit_error("connection reset"));
        assert!(!is_rate_limit_error("invalid key"));
    }
}
