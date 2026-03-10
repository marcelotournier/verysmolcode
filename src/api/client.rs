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

        // Check for API errors
        if let Some(ref err) = response.error {
            let msg = err.message.as_deref().unwrap_or("Unknown API error");
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
            Err(e) if e.contains("429") || e.contains("rate") || e.contains("quota") => {
                // Rate limited, try next model
                let fallback = match model {
                    ModelId::Gemini25Pro => {
                        if self.router.flash.can_request() {
                            Some(ModelId::Gemini25Flash)
                        } else if self.router.flash_lite.can_request() {
                            Some(ModelId::Gemini25FlashLite)
                        } else {
                            None
                        }
                    }
                    ModelId::Gemini25Flash => {
                        if self.router.flash_lite.can_request() {
                            Some(ModelId::Gemini25FlashLite)
                        } else {
                            None
                        }
                    }
                    ModelId::Gemini25FlashLite => None,
                };

                if let Some(fallback_model) = fallback {
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
        Some(ThinkingConfig {
            thinking_budget: 2048,
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
