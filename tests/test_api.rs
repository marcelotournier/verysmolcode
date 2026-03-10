use verysmolcode::api::models::*;
use verysmolcode::api::types::*;

#[test]
fn test_model_id_api_names() {
    assert!(ModelId::Gemini25Pro.api_name().contains("pro"));
    assert!(ModelId::Gemini25Flash.api_name().contains("flash"));
    assert!(ModelId::Gemini25FlashLite.api_name().contains("flash"));
}

#[test]
fn test_model_tiers() {
    assert_eq!(ModelId::Gemini25Pro.tier(), ModelTier::Pro);
    assert_eq!(ModelId::Gemini25Flash.tier(), ModelTier::Flash);
    assert_eq!(ModelId::Gemini25FlashLite.tier(), ModelTier::FlashLite);
}

#[test]
fn test_model_display_names() {
    assert!(ModelId::Gemini25Pro.display_name().contains("Pro"));
    assert!(ModelId::Gemini25Flash.display_name().contains("Flash"));
}

#[test]
fn test_thinking_support() {
    assert!(!ModelId::Gemini25Pro.supports_thinking());
    assert!(ModelId::Gemini25Flash.supports_thinking());
    assert!(ModelId::Gemini25FlashLite.supports_thinking());
}

#[test]
fn test_rate_limits() {
    let pro = RateLimit::for_model(ModelId::Gemini25Pro);
    let flash = RateLimit::for_model(ModelId::Gemini25Flash);
    let lite = RateLimit::for_model(ModelId::Gemini25FlashLite);

    // Pro has the lowest RPM/RPD
    assert!(pro.rpm < flash.rpm);
    assert!(pro.rpd < flash.rpd);
    assert!(flash.rpm < lite.rpm);
}

#[test]
fn test_rate_limiter_basic() {
    let mut limiter = RateLimiter::new(ModelId::Gemini25Pro);
    assert!(limiter.can_request());

    // Record some requests
    for _ in 0..5 {
        limiter.record_request();
    }

    // Pro has 5 RPM, so we should be at the limit
    assert!(!limiter.can_request());
}

#[test]
fn test_rate_limiter_remaining() {
    let mut limiter = RateLimiter::new(ModelId::Gemini25Flash);
    assert_eq!(limiter.remaining_today(), 250);

    limiter.record_request();
    assert_eq!(limiter.remaining_today(), 249);
}

#[test]
fn test_model_router_picks_correct_model() {
    let mut router = ModelRouter::new();

    // For smart tasks, should pick Pro first
    let model = router.pick_model(true);
    assert_eq!(model, Some(ModelId::Gemini25Pro));

    // For simple tasks, should pick Flash first
    let model = router.pick_model(false);
    assert_eq!(model, Some(ModelId::Gemini25Flash));
}

#[test]
fn test_model_router_fallback() {
    let mut router = ModelRouter::new();

    // Exhaust Pro RPM
    for _ in 0..5 {
        router.pro.record_request();
    }

    // Should fall back to Flash
    let model = router.pick_model(true);
    assert_eq!(model, Some(ModelId::Gemini25Flash));
}

#[test]
fn test_model_router_status_line() {
    let mut router = ModelRouter::new();
    let status = router.status_line();
    assert!(status.contains("Pro:"));
    assert!(status.contains("Flash:"));
    assert!(status.contains("Lite:"));
}

#[test]
fn test_part_text() {
    let part = Part::text("hello");
    match part {
        Part::Text { text } => assert_eq!(text, "hello"),
        _ => panic!("Expected Text part"),
    }
}

#[test]
fn test_part_function_response() {
    let part = Part::function_response("test_fn", serde_json::json!({"result": "ok"}));
    match part {
        Part::FunctionResponse { function_response } => {
            assert_eq!(function_response.name, "test_fn");
        }
        _ => panic!("Expected FunctionResponse part"),
    }
}

#[test]
fn test_generate_request_serialization() {
    let request = GenerateRequest {
        system_instruction: Some(Content {
            role: None,
            parts: vec![Part::text("You are a helper")],
        }),
        contents: vec![Content {
            role: Some("user".to_string()),
            parts: vec![Part::text("Hello")],
        }],
        tools: None,
        generation_config: Some(GenerationConfig {
            temperature: Some(0.7),
            max_output_tokens: Some(1024),
            thinking_config: None,
        }),
    };

    let json = serde_json::to_value(&request).unwrap();
    assert!(json.get("system_instruction").is_some());
    assert!(json.get("contents").is_some());
    assert!(json.get("generationConfig").is_some());
    // tools should be excluded when None
    assert!(json.get("tools").is_none());
}

#[test]
fn test_generate_response_deserialization() {
    let json = serde_json::json!({
        "candidates": [{
            "content": {
                "role": "model",
                "parts": [{"text": "Hello back!"}]
            },
            "finishReason": "STOP"
        }],
        "usageMetadata": {
            "promptTokenCount": 10,
            "candidatesTokenCount": 5,
            "totalTokenCount": 15,
            "thoughtsTokenCount": 0
        }
    });

    let response: GenerateResponse = serde_json::from_value(json).unwrap();
    assert_eq!(response.candidates.len(), 1);
    assert!(response.error.is_none());
    let usage = response.usage_metadata.unwrap();
    assert_eq!(usage.prompt_token_count, 10);
    assert_eq!(usage.total_token_count, 15);
}

#[test]
fn test_function_call_response_deserialization() {
    let json = serde_json::json!({
        "candidates": [{
            "content": {
                "role": "model",
                "parts": [{
                    "functionCall": {
                        "name": "read_file",
                        "args": {"path": "/tmp/test.txt"}
                    }
                }]
            }
        }]
    });

    let response: GenerateResponse = serde_json::from_value(json).unwrap();
    let content = response.candidates[0].content.as_ref().unwrap();
    match &content.parts[0] {
        Part::FunctionCall { function_call } => {
            assert_eq!(function_call.name, "read_file");
        }
        _ => panic!("Expected FunctionCall"),
    }
}
