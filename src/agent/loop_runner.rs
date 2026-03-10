use crate::api::client::{build_request, extract_response, GeminiClient};
use crate::api::models::ModelId;
use crate::api::types::*;
use crate::config::Config;
use crate::tools::registry::ToolRegistry;

/// Represents a message in the conversation
#[derive(Debug, Clone)]
pub struct AgentMessage {
    pub role: String,
    pub content: String,
    pub model: Option<String>,
    pub tool_calls: Vec<(String, serde_json::Value)>,
    pub is_thinking: bool,
}

/// The main agent loop that processes user input through the Gemini API
pub struct AgentLoop {
    client: GeminiClient,
    config: Config,
    conversation: Vec<Content>,
    total_conversation_tokens: u32,
    planning_mode: bool,
}

const PLANNING_SYSTEM_PROMPT: &str = r#"You are in PLANNING MODE. Your job is to analyze the task and create a detailed, step-by-step implementation plan.

Rules for planning mode:
1. First, understand the codebase by reading relevant files
2. Break the task into clear, numbered steps
3. Identify files that need to be created or modified
4. Consider edge cases and potential issues
5. Estimate complexity of each step
6. DO NOT make any changes - only read files and create a plan
7. Output a structured plan with clear sections

Format your plan as:
## Analysis
[Brief analysis of the task]

## Steps
1. [Step description] - [files involved]
2. ...

## Risks
- [Potential issues to watch for]

## Estimated Complexity
[Low/Medium/High] - [brief justification]"#;

impl AgentLoop {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            client: GeminiClient::new()?,
            config: Config::load(),
            conversation: Vec::new(),
            total_conversation_tokens: 0,
            planning_mode: false,
        })
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
        // Add user message to conversation
        self.conversation.push(Content {
            role: Some("user".to_string()),
            parts: vec![Part::text(user_input)],
        });

        // In planning mode, always prefer Pro models
        let prefer_smart = self.planning_mode || self.is_complex_task(user_input);

        // Main agent loop - keeps going until no more tool calls
        // Planning mode gets fewer iterations (just reading + planning)
        let max_iterations = if self.planning_mode { 8 } else { 15 };
        let mut had_tool_calls = false;
        for iteration in 0..max_iterations {
            // Check if we need to compact conversation
            if self.total_conversation_tokens > self.config.auto_compact_threshold {
                on_event(AgentEvent::Status(
                    "Compacting conversation to save tokens...".to_string(),
                ));
                self.compact_conversation();
            }

            // Pick model
            let model = self
                .client
                .router
                .pick_model(prefer_smart && iteration == 0)
                .ok_or_else(|| "All models exhausted for today".to_string())?;

            on_event(AgentEvent::ModelSwitch(model.display_name().to_string()));

            // Build request - use planning prompt in planning mode
            let system_prompt = if self.planning_mode {
                format!(
                    "{}\n\n{}",
                    PLANNING_SYSTEM_PROMPT, &self.config.system_prompt
                )
            } else {
                self.config.system_prompt.clone()
            };

            // In planning mode, only provide read-only tools
            let tools = if self.planning_mode {
                ToolRegistry::read_only_declarations()
            } else {
                ToolRegistry::declarations()
            };
            let request = build_request(
                &system_prompt,
                self.conversation.clone(),
                Some(tools),
                model,
                self.config.temperature,
                self.config.max_tokens_per_response,
            );

            // Make API call
            on_event(AgentEvent::Status("Thinking...".to_string()));
            let response = self.client.generate(model, &request)?;

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
                    });
                    function_responses.push(Part::function_response(&call.name, result));
                    continue;
                }

                let result = ToolRegistry::execute(&call.name, &call.args);
                on_event(AgentEvent::ToolResult {
                    name: call.name.clone(),
                    result: result.clone(),
                });
                function_responses.push(Part::function_response(&call.name, result));
            }

            // Add tool responses to conversation
            self.conversation.push(Content {
                role: Some("user".to_string()),
                parts: function_responses,
            });
        }

        // Run critic check if we used tools (not in planning mode)
        if !self.planning_mode && had_tool_calls {
            self.run_critic(user_input, &mut on_event)?;
        }

        Ok(())
    }

    /// Critic step: verify the work was actually done correctly
    fn run_critic<F>(&mut self, original_task: &str, on_event: &mut F) -> Result<(), String>
    where
        F: FnMut(AgentEvent),
    {
        // Use Flash-Lite for the critic to save tokens
        if !self.client.router.flash_lite.can_request() {
            return Ok(()); // Skip critic if no budget
        }

        on_event(AgentEvent::Status("Verifying work...".to_string()));

        let critic_prompt = format!(
            "Review if the following task was completed correctly:\n\nTask: {}\n\n\
             Check the recent conversation and tool results. Was the task fully completed? \
             If yes, say 'VERIFIED: [brief summary]'. \
             If not, say 'INCOMPLETE: [what's missing]' and suggest next steps.",
            original_task
        );

        self.conversation.push(Content {
            role: Some("user".to_string()),
            parts: vec![Part::text(&critic_prompt)],
        });

        let request = build_request(
            "You are a code review critic. Verify work was done correctly. Be concise.",
            self.conversation.clone(),
            None, // No tools needed for critic
            ModelId::Gemini25FlashLite,
            0.3, // Low temperature for consistent evaluation
            512, // Short response
        );

        match self.client.generate(ModelId::Gemini25FlashLite, &request) {
            Ok(response) => {
                if let Some(ref usage) = response.usage_metadata {
                    self.total_conversation_tokens = usage.total_token_count;
                }

                let (texts, _) = extract_response(&response);
                for text in &texts {
                    if !text.trim().is_empty() {
                        on_event(AgentEvent::Status(format!("Critic: {}", text)));
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
                // Critic failure is non-fatal
                on_event(AgentEvent::Status(
                    "Critic check skipped (rate limit)".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Determine if a task is complex enough to warrant Pro model
    fn is_complex_task(&self, input: &str) -> bool {
        let complex_keywords = [
            "refactor",
            "architect",
            "design",
            "complex",
            "debug",
            "fix bug",
            "optimize",
            "review",
            "analyze",
            "explain",
            "why",
            "implement",
            "create",
            "build",
            "full",
            "entire",
            "complete",
        ];
        let input_lower = input.to_lowercase();
        let has_complex_keyword = complex_keywords.iter().any(|k| input_lower.contains(k));
        let is_long = input.len() > 200;
        has_complex_keyword || is_long
    }

    /// Compact the conversation to reduce token usage
    fn compact_conversation(&mut self) {
        if self.conversation.len() <= 4 {
            return;
        }

        // Keep first message (context) and last 4 messages
        let keep_start = 1;
        let keep_end = 4;

        if self.conversation.len() > keep_start + keep_end {
            let summary = Content {
                role: Some("user".to_string()),
                parts: vec![Part::text(
                    "[Previous conversation was compacted to save tokens. Continue from the recent context below.]"
                )],
            };

            let mut new_conv = vec![self.conversation[0].clone()];
            new_conv.push(summary);
            let start = self.conversation.len() - keep_end;
            new_conv.extend_from_slice(&self.conversation[start..]);
            self.conversation = new_conv;
            self.total_conversation_tokens = 0; // Reset, will be recalculated
        }
    }

    pub fn rate_limit_status(&mut self) -> String {
        self.client.router.status_line()
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

    pub fn clear_conversation(&mut self) {
        self.conversation.clear();
        self.total_conversation_tokens = 0;
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

/// Check if a tool call looks dangerous
fn is_dangerous_tool_call(name: &str, args: &serde_json::Value) -> bool {
    match name {
        "run_command" => {
            if let Some(cmd) = args.get("command").and_then(|v| v.as_str()) {
                let dangerous = [
                    "rm -rf",
                    "rm -r /",
                    "dd if=",
                    "mkfs",
                    "format",
                    "shutdown",
                    "reboot",
                    "init 0",
                    "init 6",
                    "chmod 777",
                    "chmod -R 777",
                    "> /dev/",
                    "curl | sh",
                    "wget | sh",
                    "curl | bash",
                ];
                return dangerous.iter().any(|d| cmd.contains(d));
            }
            false
        }
        "write_file" => {
            if let Some(path) = args.get("path").and_then(|v| v.as_str()) {
                let dangerous_paths = [
                    "/etc/", "/boot/", "/usr/", "/bin/", "/sbin/", "/lib/", "/proc/", "/sys/",
                    "/dev/",
                ];
                return dangerous_paths.iter().any(|d| path.starts_with(d));
            }
            false
        }
        _ => false,
    }
}
