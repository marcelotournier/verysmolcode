use verysmolcode::api::client::{build_request, extract_response};
use verysmolcode::api::models::*;
use verysmolcode::api::types::*;

#[test]
fn test_model_id_api_names() {
    assert!(ModelId::Gemini25Pro.api_name().contains("pro"));
    assert!(ModelId::Gemini25Flash.api_name().contains("flash"));
    assert!(ModelId::Gemini25FlashLite.api_name().contains("flash"));
    assert_eq!(ModelId::Gemini31Pro.api_name(), "gemini-3.1-pro-preview");
    assert_eq!(ModelId::Gemini3Flash.api_name(), "gemini-3-flash-preview");
    assert_eq!(
        ModelId::Gemini31FlashLite.api_name(),
        "gemini-3.1-flash-lite-preview"
    );
}

#[test]
fn test_model_tiers() {
    assert_eq!(ModelId::Gemini25Pro.tier(), ModelTier::Pro);
    assert_eq!(ModelId::Gemini25Flash.tier(), ModelTier::Flash);
    assert_eq!(ModelId::Gemini25FlashLite.tier(), ModelTier::FlashLite);
    assert_eq!(ModelId::Gemini31Pro.tier(), ModelTier::Pro);
    assert_eq!(ModelId::Gemini3Flash.tier(), ModelTier::Flash);
    assert_eq!(ModelId::Gemini31FlashLite.tier(), ModelTier::FlashLite);
}

#[test]
fn test_model_display_names() {
    assert!(ModelId::Gemini25Pro.display_name().contains("Pro"));
    assert!(ModelId::Gemini25Flash.display_name().contains("Flash"));
    assert!(ModelId::Gemini31Pro.display_name().contains("3.1 Pro"));
    assert!(ModelId::Gemini3Flash.display_name().contains("3 Flash"));
}

#[test]
fn test_thinking_support() {
    assert!(ModelId::Gemini25Pro.supports_thinking()); // 2.5 Pro supports thinking
    assert!(ModelId::Gemini25Flash.supports_thinking());
    assert!(ModelId::Gemini25FlashLite.supports_thinking());
    assert!(ModelId::Gemini31Pro.supports_thinking()); // 3.1 Pro does
    assert!(ModelId::Gemini3Flash.supports_thinking());
    assert!(ModelId::Gemini31FlashLite.supports_thinking());
}

#[test]
fn test_model_all() {
    let all = ModelId::all();
    assert_eq!(all.len(), 6);
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
fn test_rate_limiter_wait_duration() {
    let mut limiter = RateLimiter::new(ModelId::Gemini25Pro);

    // Should have zero wait when under RPM limit
    assert_eq!(limiter.wait_duration(), Some(std::time::Duration::ZERO));

    // Exhaust RPM
    for _ in 0..5 {
        limiter.record_request();
    }

    // Should return Some with a positive wait (up to 60s)
    let wait = limiter.wait_duration();
    assert!(wait.is_some());
    assert!(wait.unwrap().as_secs() > 0);
    assert!(wait.unwrap().as_secs() <= 60);
}

#[test]
fn test_rate_limiter_daily_exhausted() {
    let mut limiter = RateLimiter::new(ModelId::Gemini25Pro);

    // Exhaust daily limit (25 for Pro)
    for _ in 0..25 {
        limiter.record_request();
    }

    // Should return None (daily limit, cannot wait)
    assert!(limiter.wait_duration().is_none());
    assert_eq!(limiter.remaining_today(), 0);
}

#[test]
fn test_model_router_picks_correct_model() {
    let mut router = ModelRouter::new();

    // For smart tasks, should pick Gemini 3.1 Pro first
    let model = router.pick_model(true);
    assert_eq!(model, Some(ModelId::Gemini31Pro));

    // For simple tasks, should pick Gemini 3 Flash first
    let model = router.pick_model(false);
    assert_eq!(model, Some(ModelId::Gemini3Flash));
}

#[test]
fn test_model_router_fallback() {
    let mut router = ModelRouter::new();

    // Exhaust both Pro models
    for _ in 0..5 {
        router.g3_pro.record_request();
        router.pro.record_request();
    }

    // Should fall back to Flash (Gemini 3 Flash first)
    let model = router.pick_model(true);
    assert_eq!(model, Some(ModelId::Gemini3Flash));
}

#[test]
fn test_model_router_fallback_for() {
    let mut router = ModelRouter::new();

    // Fallback from 3.1 Pro should go to 3 Flash (new order)
    let fb = router.fallback_for(ModelId::Gemini31Pro);
    assert_eq!(fb, Some(ModelId::Gemini3Flash));

    // Exhaust 3 Flash, fallback should be 3.1 Flash-Lite
    for _ in 0..10 {
        router.g3_flash.record_request();
    }
    let fb = router.fallback_for(ModelId::Gemini31Pro);
    assert_eq!(fb, Some(ModelId::Gemini31FlashLite));
}

#[test]
fn test_model_router_status_line() {
    let mut router = ModelRouter::new();
    let status = router.status_line();
    assert!(status.contains("3Pro:"));
    assert!(status.contains("3F:"));
    assert!(status.contains("Pro:"));
    assert!(status.contains("L:"));
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

#[test]
fn test_model_router_wait_for_model() {
    let mut router = ModelRouter::new();

    // Should return zero wait initially
    let wait = router.wait_for_model(ModelId::Gemini3Flash);
    assert_eq!(wait, Some(std::time::Duration::ZERO));

    // Exhaust daily limit (250 for Flash)
    for _ in 0..250 {
        router.g3_flash.record_request();
    }

    // Should return None (daily exhausted)
    let wait = router.wait_for_model(ModelId::Gemini3Flash);
    assert!(wait.is_none());
}

#[test]
fn test_model_router_exhaust_all_pro() {
    let mut router = ModelRouter::new();

    // Exhaust both Pro models
    for _ in 0..25 {
        router.g3_pro.record_request();
        router.pro.record_request();
    }

    // Should not pick Pro for smart tasks, should fall back to Flash
    let model = router.pick_model(true);
    assert!(model.is_some());
    let picked = model.unwrap();
    assert!(
        picked == ModelId::Gemini3Flash
            || picked == ModelId::Gemini25Flash
            || picked == ModelId::Gemini31FlashLite
            || picked == ModelId::Gemini25FlashLite
    );
}

// -- build_request tests --

#[test]
fn test_build_request_with_thinking() {
    let request = build_request(
        "Be helpful",
        vec![Content {
            role: Some("user".to_string()),
            parts: vec![Part::text("Hello")],
        }],
        None,
        ModelId::Gemini25Flash, // Flash supports thinking
        0.7,
        1024,
    );

    let config = request.generation_config.unwrap();
    assert!(config.thinking_config.is_some());
    // Flash tier gets 1024 thinking budget (scaled by tier to save tokens)
    assert_eq!(config.thinking_config.unwrap().thinking_budget, 1024);
}

#[test]
fn test_build_request_25pro_thinking() {
    let request = build_request(
        "Be helpful",
        vec![],
        None,
        ModelId::Gemini25Pro, // 2.5 Pro supports thinking (Pro tier budget)
        0.5,
        2048,
    );

    let config = request.generation_config.unwrap();
    assert!(config.thinking_config.is_some());
    // Pro tier gets 2048 thinking budget
    assert_eq!(config.thinking_config.unwrap().thinking_budget, 2048);
    assert_eq!(config.max_output_tokens, Some(2048));
}

#[test]
fn test_build_request_with_tools() {
    let tools = vec![ToolDeclaration {
        google_search: None,
        function_declarations: vec![FunctionDecl {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            parameters: serde_json::json!({"type": "object"}),
        }],
    }];

    let request = build_request(
        "sys",
        vec![],
        Some(tools),
        ModelId::Gemini25Flash,
        0.7,
        1024,
    );
    assert!(request.tools.is_some());
    assert_eq!(
        request.tools.unwrap()[0].function_declarations[0].name,
        "test_tool"
    );
}

#[test]
fn test_build_request_system_prompt() {
    let request = build_request(
        "Custom system prompt",
        vec![],
        None,
        ModelId::Gemini25Pro,
        0.7,
        1024,
    );
    let sys = request.system_instruction.unwrap();
    match &sys.parts[0] {
        Part::Text { text } => assert_eq!(text, "Custom system prompt"),
        _ => panic!("Expected text part"),
    }
}

// -- extract_response tests --

#[test]
fn test_extract_response_text() {
    let response = GenerateResponse {
        candidates: vec![Candidate {
            content: Some(Content {
                role: Some("model".to_string()),
                parts: vec![Part::text("Hello world")],
            }),
            finish_reason: Some("STOP".to_string()),
        }],
        usage_metadata: None,
        error: None,
    };

    let (texts, calls) = extract_response(&response);
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0], "Hello world");
    assert!(calls.is_empty());
}

#[test]
fn test_extract_response_function_call() {
    let response = GenerateResponse {
        candidates: vec![Candidate {
            content: Some(Content {
                role: Some("model".to_string()),
                parts: vec![Part::FunctionCall {
                    function_call: FunctionCall {
                        name: "write_file".to_string(),
                        args: serde_json::json!({"path": "/tmp/test", "content": "hello"}),
                    },
                }],
            }),
            finish_reason: None,
        }],
        usage_metadata: None,
        error: None,
    };

    let (texts, calls) = extract_response(&response);
    assert!(texts.is_empty());
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "write_file");
}

#[test]
fn test_extract_response_mixed() {
    let response = GenerateResponse {
        candidates: vec![Candidate {
            content: Some(Content {
                role: Some("model".to_string()),
                parts: vec![
                    Part::text("Let me create that file"),
                    Part::FunctionCall {
                        function_call: FunctionCall {
                            name: "write_file".to_string(),
                            args: serde_json::json!({}),
                        },
                    },
                ],
            }),
            finish_reason: None,
        }],
        usage_metadata: None,
        error: None,
    };

    let (texts, calls) = extract_response(&response);
    assert_eq!(texts.len(), 1);
    assert_eq!(calls.len(), 1);
}

#[test]
fn test_extract_response_empty_candidates() {
    let response = GenerateResponse {
        candidates: vec![],
        usage_metadata: None,
        error: None,
    };

    let (texts, calls) = extract_response(&response);
    assert!(texts.is_empty());
    assert!(calls.is_empty());
}

#[test]
fn test_extract_response_no_content() {
    let response = GenerateResponse {
        candidates: vec![Candidate {
            content: None,
            finish_reason: Some("SAFETY".to_string()),
        }],
        usage_metadata: None,
        error: None,
    };

    let (texts, calls) = extract_response(&response);
    assert!(texts.is_empty());
    assert!(calls.is_empty());
}

#[test]
fn test_google_search_tool_declaration() {
    let tool = ToolDeclaration::google_search();
    assert!(tool.function_declarations.is_empty());
    assert!(tool.google_search.is_some());

    // Should serialize with googleSearch key
    let json = serde_json::to_value(&tool).unwrap();
    assert!(json.get("googleSearch").is_some());
    // functionDeclarations should be omitted when empty
    assert!(json.get("functionDeclarations").is_none());
}

// -- build_request thinking budget per tier --

#[test]
fn test_build_request_pro_thinking_budget() {
    let request = build_request(
        "sys",
        vec![],
        None,
        ModelId::Gemini31Pro, // 3.1 Pro supports thinking
        0.7,
        1024,
    );
    let config = request.generation_config.unwrap();
    assert!(config.thinking_config.is_some());
    // Pro tier gets 2048 thinking budget
    assert_eq!(config.thinking_config.unwrap().thinking_budget, 2048);
}

#[test]
fn test_build_request_flash_lite_thinking_budget() {
    let request = build_request("sys", vec![], None, ModelId::Gemini31FlashLite, 0.7, 1024);
    let config = request.generation_config.unwrap();
    assert!(config.thinking_config.is_some());
    // FlashLite tier gets 512 thinking budget
    assert_eq!(config.thinking_config.unwrap().thinking_budget, 512);
}

// -- ModelRouter fallback chain end --

#[test]
fn test_fallback_for_flash_lite_returns_none_at_end() {
    let mut router = ModelRouter::new();
    // Exhaust all FlashLite models
    for _ in 0..1000 {
        router.g3_flash_lite.record_request();
        router.flash_lite.record_request();
    }
    let fb = router.fallback_for(ModelId::Gemini25FlashLite);
    assert!(fb.is_none());
}

// -- ModelRouter record_request --

#[test]
fn test_model_router_record_request() {
    let mut router = ModelRouter::new();
    let remaining_before = router.g3_flash.remaining_today();
    router.record_request(ModelId::Gemini3Flash);
    let remaining_after = router.g3_flash.remaining_today();
    assert_eq!(remaining_before - remaining_after, 1);
}

// -- Error response deserialization --

#[test]
fn test_error_response_deserialization() {
    let json = serde_json::json!({
        "error": {
            "code": 429,
            "message": "Rate limit exceeded",
            "status": "RESOURCE_EXHAUSTED"
        }
    });

    let response: GenerateResponse = serde_json::from_value(json).unwrap();
    assert!(response.error.is_some());
    assert!(response.candidates.is_empty());
}
