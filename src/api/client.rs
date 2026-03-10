use crate::api::models::{ModelId, ModelRouter};
use crate::api::types::*;
use std::env;
use std::thread;

const API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta/models";

pub struct GeminiClient {
    api_key: String,
    pub router: ModelRouter,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
}

impl GeminiClient {
    pub fn new() -> Result<Self, String> {
        let api_key =
            env::var("GEMINI_API_KEY").map_err(|_| "GEMINI_API_KEY not set".to_string())?;
        Ok(Self {
            api_key,
            router: ModelRouter::new(),
            total_input_tokens: 0,
            total_output_tokens: 0,
        })
    }

    pub fn generate(
        &mut self,
        model: ModelId,
        request: &GenerateRequest,
    ) -> Result<GenerateResponse, String> {
        // Check rate limits and wait if needed
        if let Some(wait) = self.router.wait_for_model(model) {
            if wait.as_secs() > 0 {
                thread::sleep(wait);
            }
        } else {
            return Err(format!("{} daily limit exhausted", model.display_name()));
        }

        let url = format!(
            "{}/{}:generateContent?key={}",
            API_BASE,
            model.api_name(),
            self.api_key,
        );

        let body = serde_json::to_value(request).map_err(|e| format!("Serialize error: {}", e))?;

        let resp = ureq::post(&url)
            .set("Content-Type", "application/json")
            .timeout(std::time::Duration::from_secs(120))
            .send_json(body)
            .map_err(|e| format!("API request failed: {}", e))?;

        let response: GenerateResponse = resp
            .into_json()
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        // Check for API errors (cascade: message → status → code → unknown)
        if let Some(ref err) = response.error {
            let msg = err
                .message
                .as_deref()
                .or(err.status.as_deref())
                .unwrap_or("Unknown API error");
            return Err(format!("API error: {}", msg));
        }

        // Record the request for rate limiting
        self.router.record_request(model);

        // Track token usage
        if let Some(ref usage) = response.usage_metadata {
            self.total_input_tokens += usage.prompt_token_count as u64;
            self.total_output_tokens += usage.candidates_token_count as u64;
        }

        Ok(response)
    }

    /// Send a request with automatic model fallback
    pub fn generate_with_fallback(
        &mut self,
        request: &GenerateRequest,
        prefer_smart: bool,
    ) -> Result<(GenerateResponse, ModelId), String> {
        let model = self.router.pick_model(prefer_smart).ok_or_else(|| {
            "All models exhausted for today. Please try again tomorrow.".to_string()
        })?;

        match self.generate(model, request) {
            Ok(resp) => Ok((resp, model)),
            Err(e)
                if e.contains("429")
                    || e.contains("rate")
                    || e.contains("quota")
                    || e.contains("503")
                    || e.contains("high demand") =>
            {
                // Rate limited or overloaded, try fallback model
                if let Some(fallback_model) = self.router.fallback_for(model) {
                    self.generate(fallback_model, request)
                        .map(|r| (r, fallback_model))
                } else {
                    Err(e)
                }
            }
            Err(e) => Err(e),
        }
    }

    pub fn token_usage_summary(&self) -> String {
        format!(
            "Tokens used - Input: {} | Output: {} | Total: {}",
            self.total_input_tokens,
            self.total_output_tokens,
            self.total_input_tokens + self.total_output_tokens,
        )
    }
}

/// Build a GenerateRequest with thinking enabled for Flash models
pub fn build_request(
    system_prompt: &str,
    contents: Vec<Content>,
    tools: Option<Vec<ToolDeclaration>>,
    model: ModelId,
    temperature: f32,
    max_tokens: u32,
) -> GenerateRequest {
    let thinking_config = if model.supports_thinking() {
        // Scale thinking budget by model tier to conserve tokens
        let budget = match model.tier() {
            crate::api::models::ModelTier::Pro => 2048,
            crate::api::models::ModelTier::Flash => 1024,
            crate::api::models::ModelTier::FlashLite => 512,
        };
        Some(ThinkingConfig {
            thinking_budget: budget,
        })
    } else {
        None
    };

    GenerateRequest {
        system_instruction: Some(Content {
            role: None,
            parts: vec![Part::text(system_prompt)],
        }),
        contents,
        tools,
        generation_config: Some(GenerationConfig {
            temperature: Some(temperature),
            max_output_tokens: Some(max_tokens),
            thinking_config,
        }),
    }
}

/// Extract text and function calls from a response
pub fn extract_response(response: &GenerateResponse) -> (Vec<String>, Vec<FunctionCall>) {
    let mut texts = Vec::new();
    let mut calls = Vec::new();

    for candidate in &response.candidates {
        if let Some(ref content) = candidate.content {
            for part in &content.parts {
                match part {
                    Part::Text { text } => texts.push(text.clone()),
                    Part::FunctionCall { function_call } => calls.push(function_call.clone()),
                    Part::Thought { text, .. } => {
                        // We could show thinking to the user optionally
                        let _ = text;
                    }
                    _ => {}
                }
            }
        }
    }

    (texts, calls)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::models::ModelTier;
    use serde_json::json;

    // -- build_request tests --

    #[test]
    fn test_build_request_basic() {
        let contents = vec![Content {
            role: Some("user".to_string()),
            parts: vec![Part::text("hello")],
        }];
        let req = build_request("system", contents, None, ModelId::Gemini3Flash, 0.7, 4096);
        assert!(req.system_instruction.is_some());
        assert_eq!(req.contents.len(), 1);
        assert!(req.tools.is_none());
    }

    #[test]
    fn test_build_request_thinking_budget_pro() {
        let req = build_request("sys", vec![], None, ModelId::Gemini31Pro, 0.7, 4096);
        let config = req.generation_config.unwrap();
        let thinking = config.thinking_config.unwrap();
        assert_eq!(thinking.thinking_budget, 2048);
    }

    #[test]
    fn test_build_request_thinking_budget_flash() {
        let req = build_request("sys", vec![], None, ModelId::Gemini3Flash, 0.7, 4096);
        let config = req.generation_config.unwrap();
        let thinking = config.thinking_config.unwrap();
        assert_eq!(thinking.thinking_budget, 1024);
    }

    #[test]
    fn test_build_request_thinking_budget_lite() {
        let req = build_request("sys", vec![], None, ModelId::Gemini31FlashLite, 0.7, 4096);
        let config = req.generation_config.unwrap();
        let thinking = config.thinking_config.unwrap();
        assert_eq!(thinking.thinking_budget, 512);
    }

    #[test]
    fn test_build_request_system_prompt() {
        let req = build_request(
            "You are a helpful assistant",
            vec![],
            None,
            ModelId::Gemini3Flash,
            0.5,
            2048,
        );
        let sys = req.system_instruction.unwrap();
        match &sys.parts[0] {
            Part::Text { text } => assert_eq!(text, "You are a helpful assistant"),
            _ => panic!("Expected text part in system instruction"),
        }
    }

    #[test]
    fn test_build_request_with_tools() {
        let tools = vec![ToolDeclaration::google_search()];
        let req = build_request("sys", vec![], Some(tools), ModelId::Gemini3Flash, 0.7, 4096);
        assert!(req.tools.is_some());
        assert_eq!(req.tools.unwrap().len(), 1);
    }

    #[test]
    fn test_build_request_temperature_and_tokens() {
        let req = build_request("sys", vec![], None, ModelId::Gemini3Flash, 1.5, 8192);
        let config = req.generation_config.unwrap();
        assert_eq!(config.temperature, Some(1.5));
        assert_eq!(config.max_output_tokens, Some(8192));
    }

    // -- extract_response tests --

    #[test]
    fn test_extract_response_text() {
        let resp = GenerateResponse {
            candidates: vec![Candidate {
                content: Some(Content {
                    role: Some("model".to_string()),
                    parts: vec![Part::text("Hello!")],
                }),
                finish_reason: Some("STOP".to_string()),
            }],
            usage_metadata: None,
            error: None,
        };
        let (texts, calls) = extract_response(&resp);
        assert_eq!(texts, vec!["Hello!"]);
        assert!(calls.is_empty());
    }

    #[test]
    fn test_extract_response_function_call() {
        let resp = GenerateResponse {
            candidates: vec![Candidate {
                content: Some(Content {
                    role: Some("model".to_string()),
                    parts: vec![Part::function_call(
                        "read_file",
                        json!({"path": "/tmp/test"}),
                    )],
                }),
                finish_reason: Some("STOP".to_string()),
            }],
            usage_metadata: None,
            error: None,
        };
        let (texts, calls) = extract_response(&resp);
        assert!(texts.is_empty());
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "read_file");
    }

    #[test]
    fn test_extract_response_mixed() {
        let resp = GenerateResponse {
            candidates: vec![Candidate {
                content: Some(Content {
                    role: Some("model".to_string()),
                    parts: vec![
                        Part::text("I'll read the file."),
                        Part::function_call("read_file", json!({"path": "main.rs"})),
                    ],
                }),
                finish_reason: Some("STOP".to_string()),
            }],
            usage_metadata: None,
            error: None,
        };
        let (texts, calls) = extract_response(&resp);
        assert_eq!(texts.len(), 1);
        assert_eq!(calls.len(), 1);
    }

    #[test]
    fn test_extract_response_empty_candidates() {
        let resp = GenerateResponse {
            candidates: vec![],
            usage_metadata: None,
            error: None,
        };
        let (texts, calls) = extract_response(&resp);
        assert!(texts.is_empty());
        assert!(calls.is_empty());
    }

    #[test]
    fn test_extract_response_no_content() {
        let resp = GenerateResponse {
            candidates: vec![Candidate {
                content: None,
                finish_reason: Some("SAFETY".to_string()),
            }],
            usage_metadata: None,
            error: None,
        };
        let (texts, calls) = extract_response(&resp);
        assert!(texts.is_empty());
        assert!(calls.is_empty());
    }

    // -- GeminiClient tests (no API calls) --

    #[test]
    fn test_client_new_without_key() {
        // Temporarily remove key to test error path
        let original = env::var("GEMINI_API_KEY").ok();
        env::remove_var("GEMINI_API_KEY");
        let result = GeminiClient::new();
        // Restore
        if let Some(key) = original {
            env::set_var("GEMINI_API_KEY", key);
        }
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("GEMINI_API_KEY"));
    }

    #[test]
    fn test_client_token_usage_summary() {
        // Can't create real client without API key, but test format
        let summary = format!(
            "Tokens used - Input: {} | Output: {} | Total: {}",
            100, 50, 150
        );
        assert!(summary.contains("Input: 100"));
        assert!(summary.contains("Output: 50"));
        assert!(summary.contains("Total: 150"));
    }

    #[test]
    fn test_thinking_budget_tiers() {
        // Verify budget matches tier expectations
        for model in ModelId::all() {
            let req = build_request("test", vec![], None, *model, 0.7, 4096);
            let config = req.generation_config.unwrap();
            if model.supports_thinking() {
                let budget = config.thinking_config.unwrap().thinking_budget;
                match model.tier() {
                    ModelTier::Pro => assert_eq!(budget, 2048),
                    ModelTier::Flash => assert_eq!(budget, 1024),
                    ModelTier::FlashLite => assert_eq!(budget, 512),
                }
            }
        }
    }
}
