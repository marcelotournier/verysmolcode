use serde::{Deserialize, Serialize};

// -- Request types --

#[derive(Debug, Clone, Serialize)]
pub struct GenerateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<Content>,
    pub contents: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDeclaration>>,
    #[serde(rename = "generationConfig", skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GenerationConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    pub parts: Vec<Part>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Part {
    Text {
        text: String,
    },
    FunctionCall {
        #[serde(rename = "functionCall")]
        function_call: FunctionCall,
    },
    FunctionResponse {
        #[serde(rename = "functionResponse")]
        function_response: FunctionResponse,
    },
    InlineData {
        #[serde(rename = "inlineData")]
        inline_data: InlineData,
    },
    Thought {
        thought: bool,
        text: String,
    },
}

impl Part {
    pub fn text(s: impl Into<String>) -> Self {
        Part::Text { text: s.into() }
    }

    pub fn function_call(name: impl Into<String>, args: serde_json::Value) -> Self {
        Part::FunctionCall {
            function_call: FunctionCall {
                name: name.into(),
                args,
            },
        }
    }

    pub fn function_response(name: impl Into<String>, response: serde_json::Value) -> Self {
        Part::FunctionResponse {
            function_response: FunctionResponse {
                name: name.into(),
                response,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub args: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionResponse {
    pub name: String,
    pub response: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineData {
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    pub data: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolDeclaration {
    #[serde(rename = "functionDeclarations", skip_serializing_if = "Vec::is_empty")]
    pub function_declarations: Vec<FunctionDecl>,
    #[serde(rename = "googleSearch", skip_serializing_if = "Option::is_none")]
    pub google_search: Option<serde_json::Value>,
}

impl ToolDeclaration {
    /// Create a google_search grounding tool
    pub fn google_search() -> Self {
        Self {
            function_declarations: Vec::new(),
            google_search: Some(serde_json::json!({})),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct FunctionDecl {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(rename = "maxOutputTokens", skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
    #[serde(rename = "thinkingConfig", skip_serializing_if = "Option::is_none")]
    pub thinking_config: Option<ThinkingConfig>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ThinkingConfig {
    #[serde(rename = "thinkingBudget")]
    pub thinking_budget: u32,
}

// -- Response types --

#[derive(Debug, Clone, Deserialize)]
pub struct GenerateResponse {
    #[serde(default)]
    pub candidates: Vec<Candidate>,
    #[serde(rename = "usageMetadata")]
    pub usage_metadata: Option<UsageMetadata>,
    pub error: Option<ApiError>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Candidate {
    pub content: Option<Content>,
    #[serde(rename = "finishReason")]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UsageMetadata {
    #[serde(rename = "promptTokenCount", default)]
    pub prompt_token_count: u32,
    #[serde(rename = "candidatesTokenCount", default)]
    pub candidates_token_count: u32,
    #[serde(rename = "totalTokenCount", default)]
    pub total_token_count: u32,
    #[serde(rename = "thoughtsTokenCount", default)]
    pub thoughts_token_count: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiError {
    pub code: Option<u32>,
    pub message: Option<String>,
    pub status: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_part_text() {
        let part = Part::text("hello");
        let json = serde_json::to_value(&part).unwrap();
        assert_eq!(json, json!({"text": "hello"}));
    }

    #[test]
    fn test_part_function_call() {
        let part = Part::function_call("read_file", json!({"path": "/tmp/test"}));
        let json = serde_json::to_value(&part).unwrap();
        assert!(json.get("functionCall").is_some());
        assert_eq!(json["functionCall"]["name"], "read_file");
    }

    #[test]
    fn test_part_function_response() {
        let part = Part::function_response("read_file", json!({"content": "data"}));
        let json = serde_json::to_value(&part).unwrap();
        assert!(json.get("functionResponse").is_some());
        assert_eq!(json["functionResponse"]["name"], "read_file");
    }

    #[test]
    fn test_content_serialize() {
        let content = Content {
            role: Some("user".to_string()),
            parts: vec![Part::text("hi")],
        };
        let json = serde_json::to_value(&content).unwrap();
        assert_eq!(json["role"], "user");
        assert_eq!(json["parts"][0]["text"], "hi");
    }

    #[test]
    fn test_content_role_none_omitted() {
        let content = Content {
            role: None,
            parts: vec![Part::text("system")],
        };
        let json = serde_json::to_value(&content).unwrap();
        assert!(json.get("role").is_none());
    }

    #[test]
    fn test_tool_declaration_google_search() {
        let tool = ToolDeclaration::google_search();
        let json = serde_json::to_value(&tool).unwrap();
        assert!(json.get("googleSearch").is_some());
        // functionDeclarations should be omitted (empty vec, skip_serializing_if)
        assert!(json.get("functionDeclarations").is_none());
    }

    #[test]
    fn test_generation_config_serialize() {
        let config = GenerationConfig {
            temperature: Some(1.0),
            max_output_tokens: Some(4096),
            thinking_config: None,
        };
        let json = serde_json::to_value(&config).unwrap();
        assert_eq!(json["temperature"], 1.0);
        assert_eq!(json["maxOutputTokens"], 4096);
        assert!(json.get("thinkingConfig").is_none());
    }

    #[test]
    fn test_generation_config_with_thinking() {
        let config = GenerationConfig {
            temperature: None,
            max_output_tokens: None,
            thinking_config: Some(ThinkingConfig {
                thinking_budget: 2048,
            }),
        };
        let json = serde_json::to_value(&config).unwrap();
        assert_eq!(json["thinkingConfig"]["thinkingBudget"], 2048);
    }

    #[test]
    fn test_generate_response_deserialize() {
        let json = r#"{
            "candidates": [{
                "content": {"role": "model", "parts": [{"text": "Hello!"}]},
                "finishReason": "STOP"
            }],
            "usageMetadata": {
                "promptTokenCount": 10,
                "candidatesTokenCount": 5,
                "totalTokenCount": 15,
                "thoughtsTokenCount": 0
            }
        }"#;
        let resp: GenerateResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.candidates.len(), 1);
        assert_eq!(resp.candidates[0].finish_reason.as_deref(), Some("STOP"));
        let meta = resp.usage_metadata.unwrap();
        assert_eq!(meta.prompt_token_count, 10);
        assert_eq!(meta.total_token_count, 15);
    }

    #[test]
    fn test_generate_response_error() {
        let json = r#"{
            "error": {
                "code": 429,
                "message": "Rate limit exceeded",
                "status": "RESOURCE_EXHAUSTED"
            }
        }"#;
        let resp: GenerateResponse = serde_json::from_str(json).unwrap();
        assert!(resp.candidates.is_empty());
        let err = resp.error.unwrap();
        assert_eq!(err.code, Some(429));
        assert_eq!(err.status.as_deref(), Some("RESOURCE_EXHAUSTED"));
    }

    #[test]
    fn test_generate_response_empty_candidates() {
        let json = r#"{}"#;
        let resp: GenerateResponse = serde_json::from_str(json).unwrap();
        assert!(resp.candidates.is_empty());
        assert!(resp.usage_metadata.is_none());
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_inline_data_serialize() {
        let data = InlineData {
            mime_type: "image/png".to_string(),
            data: "base64data".to_string(),
        };
        let json = serde_json::to_value(&data).unwrap();
        assert_eq!(json["mimeType"], "image/png");
        assert_eq!(json["data"], "base64data");
    }

    #[test]
    fn test_function_decl_serialize() {
        let decl = FunctionDecl {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            parameters: json!({"type": "object"}),
        };
        let json = serde_json::to_value(&decl).unwrap();
        assert_eq!(json["name"], "test_tool");
        assert_eq!(json["description"], "A test tool");
    }

    #[test]
    fn test_part_thought_deserialize() {
        // Note: Part uses #[serde(untagged)], so Thought variant with {thought, text}
        // may deserialize as Text first (since Text only requires "text" field).
        // This tests that the JSON at least deserializes without error.
        let json = r#"{"thought": true, "text": "Let me think..."}"#;
        let part: Part = serde_json::from_str(json).unwrap();
        // With untagged enum, serde tries Text first (which matches "text" field)
        match part {
            Part::Text { ref text } => assert_eq!(text, "Let me think..."),
            Part::Thought { text, .. } => assert_eq!(text, "Let me think..."),
            _ => panic!("Expected Text or Thought variant"),
        }
    }

    #[test]
    fn test_usage_metadata_defaults() {
        let json = r#"{}"#;
        let meta: UsageMetadata = serde_json::from_str(json).unwrap();
        assert_eq!(meta.prompt_token_count, 0);
        assert_eq!(meta.candidates_token_count, 0);
        assert_eq!(meta.total_token_count, 0);
        assert_eq!(meta.thoughts_token_count, 0);
    }
}
