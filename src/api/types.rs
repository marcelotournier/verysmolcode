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
    #[serde(rename = "functionDeclarations")]
    pub function_declarations: Vec<FunctionDecl>,
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
