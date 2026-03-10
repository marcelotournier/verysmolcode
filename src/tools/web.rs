use serde_json::{json, Value};

/// Fetch a URL and return its content (text only, truncated for token savings)
pub fn web_fetch(args: &Value) -> Value {
    let url = match args.get("url").and_then(|v| v.as_str()) {
        Some(u) => u,
        None => return json!({"error": "Missing 'url' argument"}),
    };

    // Validate URL
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return json!({"error": "URL must start with http:// or https://"});
    }

    // Block potentially dangerous URLs
    if url.contains("localhost") || url.contains("127.0.0.1") || url.contains("0.0.0.0") {
        return json!({"error": "Cannot fetch localhost URLs"});
    }

    match ureq::get(url)
        .set(
            "User-Agent",
            &format!(
                "VerySmolCode/{} (coding-assistant)",
                env!("CARGO_PKG_VERSION")
            ),
        )
        .call()
    {
        Ok(resp) => {
            let content_type = resp
                .header("Content-Type")
                .unwrap_or("text/plain")
                .to_string();

            // Only handle text content
            if !content_type.contains("text") && !content_type.contains("json") {
                return json!({
                    "error": format!("Cannot process content type: {}", content_type),
                    "url": url
                });
            }

            match resp.into_string() {
                Ok(body) => {
                    // Strip HTML tags for cleaner output (simple approach)
                    let clean = if content_type.contains("html") {
                        strip_html_tags(&body)
                    } else {
                        body.clone()
                    };

                    // Truncate to save tokens (at a safe char boundary)
                    let max_chars = 20_000;
                    let (content, truncated) = if clean.len() > max_chars {
                        let mut end = max_chars;
                        while end > 0 && !clean.is_char_boundary(end) {
                            end -= 1;
                        }
                        (clean[..end].to_string(), true)
                    } else {
                        (clean, false)
                    };

                    json!({
                        "url": url,
                        "content": content,
                        "truncated": truncated,
                        "content_type": content_type
                    })
                }
                Err(e) => json!({"error": format!("Failed to read response: {}", e)}),
            }
        }
        Err(e) => json!({"error": format!("Failed to fetch {}: {}", url, e)}),
    }
}

/// Simple HTML tag stripper - removes tags and collapses whitespace
fn strip_html_tags(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut in_script = false;
    let mut last_was_space = false;

    let lower = html.to_lowercase();
    let chars: Vec<char> = html.chars().collect();
    let lower_chars: Vec<char> = lower.chars().collect();

    let mut i = 0;
    while i < chars.len() {
        if !in_tag && i + 7 < lower_chars.len() {
            let slice: String = lower_chars[i..i + 7].iter().collect();
            if slice == "<script" {
                in_script = true;
            }
        }
        if in_script && i + 9 < lower_chars.len() {
            let slice: String = lower_chars[i..i + 9].iter().collect();
            if slice == "</script>" {
                in_script = false;
                i += 9;
                continue;
            }
        }

        if in_script {
            i += 1;
            continue;
        }

        match chars[i] {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                if !last_was_space {
                    result.push(' ');
                    last_was_space = true;
                }
            }
            _ if !in_tag => {
                let c = chars[i];
                if c.is_whitespace() {
                    if !last_was_space {
                        result.push(' ');
                        last_was_space = true;
                    }
                } else {
                    result.push(c);
                    last_was_space = false;
                }
            }
            _ => {}
        }
        i += 1;
    }

    result.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_html_tags() {
        let html = "<html><body><h1>Hello</h1><p>World</p></body></html>";
        let result = strip_html_tags(html);
        assert!(result.contains("Hello"));
        assert!(result.contains("World"));
        assert!(!result.contains("<"));
    }

    #[test]
    fn test_strip_html_script() {
        let html = "<p>Before</p><script>alert('xss')</script><p>After</p>";
        let result = strip_html_tags(html);
        assert!(result.contains("Before"));
        assert!(result.contains("After"));
        assert!(!result.contains("alert"));
    }

    #[test]
    fn test_web_fetch_missing_url() {
        let result = web_fetch(&json!({}));
        assert!(result.get("error").is_some());
    }

    #[test]
    fn test_web_fetch_invalid_url() {
        let result = web_fetch(&json!({"url": "not-a-url"}));
        assert!(result.get("error").is_some());
    }

    #[test]
    fn test_web_fetch_localhost_blocked() {
        let result = web_fetch(&json!({"url": "http://localhost:8080"}));
        assert!(result.get("error").is_some());
    }
}
