use crate::telegram::config::TelegramConfig;
use serde::Deserialize;

const API_BASE: &str = "https://api.telegram.org/bot";

/// Max message length for Telegram API (UTF-8 chars)
const MAX_MESSAGE_LEN: usize = 4096;

/// Lightweight Telegram bot client using ureq (sync HTTP)
pub struct TelegramBot {
    token: String,
    chat_id: i64,
    /// Last update_id we processed (for long polling offset)
    last_update_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct TelegramResponse<T> {
    ok: bool,
    result: Option<T>,
    description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Update {
    pub update_id: i64,
    pub message: Option<Message>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Message {
    pub message_id: i64,
    pub chat: Chat,
    pub text: Option<String>,
    pub caption: Option<String>,
    pub from: Option<User>,
    pub photo: Option<Vec<PhotoSize>>,
    pub document: Option<Document>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Chat {
    pub id: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct User {
    pub id: i64,
    pub first_name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PhotoSize {
    pub file_id: String,
    pub file_size: Option<u64>,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Document {
    pub file_id: String,
    pub file_name: Option<String>,
    pub mime_type: Option<String>,
    pub file_size: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct TelegramFile {
    file_path: Option<String>,
}

impl TelegramBot {
    /// Create a new bot from config. Returns None if not configured.
    pub fn from_config(config: &TelegramConfig) -> Option<Self> {
        if !config.is_configured() {
            return None;
        }
        Some(Self {
            token: config.bot_token.clone()?,
            chat_id: config.chat_id?,
            last_update_id: None,
        })
    }

    /// Create a bot with explicit token and chat_id (for testing setup)
    pub fn new(token: String, chat_id: i64) -> Self {
        Self {
            token,
            chat_id,
            last_update_id: None,
        }
    }

    fn api_url(&self, method: &str) -> String {
        format!("{}{}/{}", API_BASE, self.token, method)
    }

    /// Send a text message to the configured chat.
    /// Long messages are split at 4096 chars (Telegram limit).
    pub fn send_message(&self, text: &str) -> Result<(), String> {
        if text.is_empty() {
            return Ok(());
        }

        // Split long messages
        let chunks = split_message(text);
        for chunk in chunks {
            self.send_chunk(&chunk)?;
        }
        Ok(())
    }

    fn send_chunk(&self, text: &str) -> Result<(), String> {
        let body = serde_json::json!({
            "chat_id": self.chat_id,
            "text": text,
            "parse_mode": "Markdown",
            "disable_web_page_preview": true,
        });

        let resp = ureq::post(&self.api_url("sendMessage"))
            .set("Content-Type", "application/json")
            .timeout(std::time::Duration::from_secs(10))
            .send_json(body)
            .map_err(|e| format!("Telegram send failed: {}", e))?;

        let result: TelegramResponse<serde_json::Value> = resp
            .into_json()
            .map_err(|e| format!("Telegram parse error: {}", e))?;

        if !result.ok {
            // If markdown parsing fails, retry without parse_mode
            let desc = result.description.unwrap_or_default();
            if desc.contains("parse") || desc.contains("entities") {
                return self.send_chunk_plain(text);
            }
            return Err(format!("Telegram API error: {}", desc));
        }
        Ok(())
    }

    /// Fallback: send without Markdown parsing
    fn send_chunk_plain(&self, text: &str) -> Result<(), String> {
        let body = serde_json::json!({
            "chat_id": self.chat_id,
            "text": text,
            "disable_web_page_preview": true,
        });

        let resp = ureq::post(&self.api_url("sendMessage"))
            .set("Content-Type", "application/json")
            .timeout(std::time::Duration::from_secs(10))
            .send_json(body)
            .map_err(|e| format!("Telegram send failed: {}", e))?;

        let result: TelegramResponse<serde_json::Value> = resp
            .into_json()
            .map_err(|e| format!("Telegram parse error: {}", e))?;

        if !result.ok {
            return Err(format!(
                "Telegram API error: {}",
                result.description.unwrap_or_default()
            ));
        }
        Ok(())
    }

    /// Poll for new messages (short poll with timeout).
    /// Returns messages from our chat_id only. Includes attachment content when present.
    /// timeout_secs: how long the server should wait for updates (long polling)
    pub fn get_updates(&mut self, timeout_secs: u64) -> Result<Vec<String>, String> {
        let mut params = serde_json::json!({
            "timeout": timeout_secs,
            "allowed_updates": ["message"],
        });

        if let Some(offset) = self.last_update_id {
            params["offset"] = serde_json::json!(offset + 1);
        }

        let resp = ureq::post(&self.api_url("getUpdates"))
            .set("Content-Type", "application/json")
            .timeout(std::time::Duration::from_secs(timeout_secs + 5))
            .send_json(params)
            .map_err(|e| format!("Telegram poll failed: {}", e))?;

        let result: TelegramResponse<Vec<Update>> = resp
            .into_json()
            .map_err(|e| format!("Telegram parse error: {}", e))?;

        if !result.ok {
            return Err(format!(
                "Telegram API error: {}",
                result.description.unwrap_or_default()
            ));
        }

        let updates = result.result.unwrap_or_default();
        let mut messages = Vec::new();

        for update in updates {
            // Track the latest update_id for offset
            self.last_update_id = Some(update.update_id);

            // Only process messages from our configured chat
            if let Some(msg) = update.message {
                if msg.chat.id != self.chat_id {
                    continue;
                }

                let caption = msg.caption.clone().unwrap_or_default();

                // Handle photo attachments
                if let Some(photos) = &msg.photo {
                    // Take the highest-resolution version (last element)
                    if let Some(photo) = photos.last() {
                        if let Some((b64, mime)) = self.download_file(&photo.file_id, "image/jpeg")
                        {
                            let text_part = if !caption.is_empty() {
                                format!("[Photo] {}\n[data:{}:{}]", caption, mime, b64)
                            } else {
                                format!("[Photo attached]\n[data:{}:{}]", mime, b64)
                            };
                            messages.push(text_part);
                        } else if !caption.is_empty() {
                            messages.push(format!("[Photo] {}", caption));
                        }
                        continue;
                    }
                }

                // Handle document attachments
                if let Some(doc) = &msg.document {
                    let fallback_mime = doc
                        .mime_type
                        .clone()
                        .unwrap_or_else(|| "application/octet-stream".to_string());
                    let filename = doc
                        .file_name
                        .clone()
                        .unwrap_or_else(|| "document".to_string());

                    if let Some((b64, mime)) = self.download_file(&doc.file_id, &fallback_mime) {
                        let text_part = if !caption.is_empty() {
                            format!(
                                "[Document: {}] {}\n[data:{}:{}]",
                                filename, caption, mime, b64
                            )
                        } else {
                            format!("[Document: {}]\n[data:{}:{}]", filename, mime, b64)
                        };
                        messages.push(text_part);
                    } else if !caption.is_empty() {
                        messages.push(format!("[Document: {}] {}", filename, caption));
                    }
                    continue;
                }

                // Plain text message
                if let Some(text) = msg.text {
                    if !text.is_empty() {
                        messages.push(text);
                    }
                }
            }
        }

        Ok(messages)
    }

    /// Verify the bot token works by calling getMe
    pub fn verify(&self) -> Result<String, String> {
        let resp = ureq::get(&self.api_url("getMe"))
            .timeout(std::time::Duration::from_secs(10))
            .call()
            .map_err(|e| format!("Telegram verify failed: {}", e))?;

        let result: TelegramResponse<User> = resp
            .into_json()
            .map_err(|e| format!("Telegram parse error: {}", e))?;

        if !result.ok {
            return Err(format!(
                "Invalid bot token: {}",
                result.description.unwrap_or_default()
            ));
        }

        let user = result.result.ok_or("No bot info returned")?;
        Ok(format!("@{}", user.first_name))
    }

    pub fn chat_id(&self) -> i64 {
        self.chat_id
    }

    /// Get a file's download path from its file_id (max 20MB per Telegram limits)
    fn get_file_path(&self, file_id: &str) -> Option<String> {
        let resp = ureq::post(&self.api_url("getFile"))
            .set("Content-Type", "application/json")
            .timeout(std::time::Duration::from_secs(10))
            .send_json(serde_json::json!({"file_id": file_id}))
            .ok()?;

        let result: TelegramResponse<TelegramFile> = resp.into_json().ok()?;
        if result.ok {
            result.result.and_then(|f| f.file_path)
        } else {
            None
        }
    }

    /// Download a file by file_id, returns base64-encoded content and mime type
    fn download_file(&self, file_id: &str, fallback_mime: &str) -> Option<(String, String)> {
        let file_path = self.get_file_path(file_id)?;
        let url = format!(
            "https://api.telegram.org/file/bot{}/{}",
            self.token, file_path
        );

        let resp = ureq::get(&url)
            .timeout(std::time::Duration::from_secs(30))
            .call()
            .ok()?;

        // Detect MIME type from extension or fallback
        let mime = if file_path.ends_with(".jpg") || file_path.ends_with(".jpeg") {
            "image/jpeg"
        } else if file_path.ends_with(".png") {
            "image/png"
        } else if file_path.ends_with(".gif") {
            "image/gif"
        } else if file_path.ends_with(".webp") {
            "image/webp"
        } else if file_path.ends_with(".pdf") {
            "application/pdf"
        } else {
            fallback_mime
        };

        let mut bytes = Vec::new();
        std::io::Read::read_to_end(&mut resp.into_reader(), &mut bytes).ok()?;

        // Limit to 2MB to avoid memory pressure on RPi3
        if bytes.len() > 2 * 1024 * 1024 {
            return None;
        }

        use base64::Engine as _;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
        Some((b64, mime.to_string()))
    }

    /// Send a file (document) to the configured chat
    pub fn send_document(&self, file_path: &str, caption: Option<&str>) -> Result<(), String> {
        use std::fs;
        let data =
            fs::read(file_path).map_err(|e| format!("Failed to read file {}: {}", file_path, e))?;

        // For files >50MB Telegram requires upload via URL — too large for free bots anyway
        if data.len() > 50 * 1024 * 1024 {
            return Err("File too large for Telegram (max 50MB)".to_string());
        }

        let filename = std::path::Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file");

        // Use multipart form for file upload
        let boundary = "----TelegramBoundary";
        let mut body = Vec::new();
        let chat_id_str = self.chat_id.to_string();

        // chat_id field
        body.extend_from_slice(
            format!(
                "--{}\r\nContent-Disposition: form-data; name=\"chat_id\"\r\n\r\n{}\r\n",
                boundary, chat_id_str
            )
            .as_bytes(),
        );

        // caption field
        if let Some(cap) = caption {
            body.extend_from_slice(
                format!(
                    "--{}\r\nContent-Disposition: form-data; name=\"caption\"\r\n\r\n{}\r\n",
                    boundary, cap
                )
                .as_bytes(),
            );
        }

        // document field
        body.extend_from_slice(
            format!(
                "--{}\r\nContent-Disposition: form-data; name=\"document\"; filename=\"{}\"\r\nContent-Type: application/octet-stream\r\n\r\n",
                boundary, filename
            ).as_bytes()
        );
        body.extend_from_slice(&data);
        body.extend_from_slice(format!("\r\n--{}--\r\n", boundary).as_bytes());

        let content_type = format!("multipart/form-data; boundary={}", boundary);
        let resp = ureq::post(&self.api_url("sendDocument"))
            .set("Content-Type", &content_type)
            .timeout(std::time::Duration::from_secs(60))
            .send_bytes(&body)
            .map_err(|e| format!("Telegram file send failed: {}", e))?;

        let result: TelegramResponse<serde_json::Value> = resp
            .into_json()
            .map_err(|e| format!("Telegram parse error: {}", e))?;

        if !result.ok {
            return Err(format!(
                "Telegram API error: {}",
                result.description.unwrap_or_default()
            ));
        }
        Ok(())
    }
}

/// Split a message into chunks of MAX_MESSAGE_LEN, breaking at newlines when possible
fn split_message(text: &str) -> Vec<String> {
    if text.len() <= MAX_MESSAGE_LEN {
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let mut remaining = text;

    while !remaining.is_empty() {
        if remaining.len() <= MAX_MESSAGE_LEN {
            chunks.push(remaining.to_string());
            break;
        }

        // Find a good split point (newline) within the limit
        let split_at = remaining[..MAX_MESSAGE_LEN]
            .rfind('\n')
            .unwrap_or(MAX_MESSAGE_LEN);

        // Ensure we're at a char boundary
        let mut end = split_at;
        while end > 0 && !remaining.is_char_boundary(end) {
            end -= 1;
        }
        if end == 0 {
            end = MAX_MESSAGE_LEN.min(remaining.len());
            while end < remaining.len() && !remaining.is_char_boundary(end) {
                end += 1;
            }
        }

        chunks.push(remaining[..end].to_string());
        remaining = &remaining[end..];
        // Skip the newline if we split there
        if remaining.starts_with('\n') {
            remaining = &remaining[1..];
        }
    }

    chunks
}

/// Send a message via Telegram as a tool call from the agent.
/// This is the function called by the tool registry.
pub fn send_telegram_tool(args: &serde_json::Value) -> serde_json::Value {
    let message = args.get("message").and_then(|v| v.as_str()).unwrap_or("");

    if message.is_empty() {
        return serde_json::json!({"error": "message is required"});
    }

    let config = TelegramConfig::load();
    let bot = match TelegramBot::from_config(&config) {
        Some(b) => b,
        None => {
            return serde_json::json!({
                "error": "Telegram not configured. Use /telegram setup <bot_token> <chat_id> to configure."
            })
        }
    };

    match bot.send_message(message) {
        Ok(()) => serde_json::json!({
            "success": true,
            "message": "Message sent to Telegram"
        }),
        Err(e) => serde_json::json!({"error": e}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_message_short() {
        let chunks = split_message("hello");
        assert_eq!(chunks, vec!["hello"]);
    }

    #[test]
    fn test_split_message_long() {
        let text = "a".repeat(5000);
        let chunks = split_message(&text);
        assert!(chunks.len() >= 2);
        for chunk in &chunks {
            assert!(chunk.len() <= MAX_MESSAGE_LEN);
        }
        // All chars preserved
        let total: usize = chunks.iter().map(|c| c.len()).sum();
        assert_eq!(total, 5000);
    }

    #[test]
    fn test_split_message_at_newlines() {
        let mut text = String::new();
        for i in 0..200 {
            text.push_str(&format!("Line {}: some content here\n", i));
        }
        let chunks = split_message(&text);
        for chunk in &chunks {
            assert!(chunk.len() <= MAX_MESSAGE_LEN);
        }
    }

    #[test]
    fn test_split_message_empty() {
        let chunks = split_message("");
        assert_eq!(chunks, vec![""]);
    }

    #[test]
    fn test_bot_new() {
        let bot = TelegramBot::new("token".to_string(), 123);
        assert_eq!(bot.chat_id(), 123);
        assert!(bot.last_update_id.is_none());
    }

    #[test]
    fn test_bot_from_config_unconfigured() {
        let config = TelegramConfig::default();
        assert!(TelegramBot::from_config(&config).is_none());
    }

    #[test]
    fn test_bot_from_config_configured() {
        let config = TelegramConfig {
            bot_token: Some("tok".to_string()),
            chat_id: Some(42),
            enabled: true,
        };
        let bot = TelegramBot::from_config(&config).unwrap();
        assert_eq!(bot.chat_id(), 42);
    }

    #[test]
    fn test_api_url() {
        let bot = TelegramBot::new("123:ABC".to_string(), 1);
        let url = bot.api_url("sendMessage");
        assert_eq!(url, "https://api.telegram.org/bot123:ABC/sendMessage");
    }

    #[test]
    fn test_send_telegram_tool_no_message() {
        let result = send_telegram_tool(&serde_json::json!({}));
        assert!(result.get("error").is_some());
    }

    #[test]
    fn test_send_telegram_tool_empty_message() {
        let result = send_telegram_tool(&serde_json::json!({"message": ""}));
        assert!(result.get("error").is_some());
    }

    #[test]
    fn test_send_telegram_tool_not_configured() {
        // This will fail because Telegram isn't configured in test env
        let result = send_telegram_tool(&serde_json::json!({"message": "test"}));
        assert!(result.get("error").is_some());
    }
}
