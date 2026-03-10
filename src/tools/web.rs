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
        .timeout(std::time::Duration::from_secs(60))
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

/// Check if bytes at position match a tag name (case-insensitive ASCII)
fn matches_tag(bytes: &[u8], pos: usize, tag: &[u8]) -> bool {
    pos + 1 + tag.len() <= bytes.len()
        && bytes[pos] == b'<'
        && bytes[pos + 1..pos + 1 + tag.len()]
            .iter()
            .zip(tag)
            .all(|(a, b)| a.to_ascii_lowercase() == *b)
}

/// Check if bytes at position match a closing tag (case-insensitive ASCII)
fn matches_close_tag(bytes: &[u8], pos: usize, tag: &[u8]) -> bool {
    pos + 2 + tag.len() < bytes.len()
        && bytes[pos] == b'<'
        && bytes[pos + 1] == b'/'
        && bytes[pos + 2..pos + 2 + tag.len()]
            .iter()
            .zip(tag)
            .all(|(a, b)| a.to_ascii_lowercase() == *b)
        && bytes[pos + 2 + tag.len()] == b'>'
}

/// Simple HTML tag stripper - removes tags and collapses whitespace.
/// Uses byte-level scanning for ASCII tags to avoid allocating Vec<char>.
/// Strips <script> and <style> block content entirely.
fn strip_html_tags(html: &str) -> String {
    let mut result = String::with_capacity(html.len() / 2);
    let bytes = html.as_bytes();
    let len = bytes.len();
    let mut in_tag = false;
    let mut in_hidden: Option<&[u8]> = None; // which tag we're inside (script or style)
    let mut last_was_space = false;
    let mut i = 0;

    // Tags whose content should be stripped entirely
    const HIDDEN_TAGS: &[&[u8]] = &[b"script", b"style"];

    while i < len {
        // Check for opening hidden tags (<script, <style)
        if !in_tag && in_hidden.is_none() {
            for tag in HIDDEN_TAGS {
                if matches_tag(bytes, i, tag) {
                    in_hidden = Some(tag);
                    in_tag = true;
                    i += 1;
                    break;
                }
            }
            if in_hidden.is_some() {
                continue;
            }
        }

        // Check for closing hidden tags (</script>, </style>)
        if let Some(tag) = in_hidden {
            if matches_close_tag(bytes, i, tag) {
                let skip = 2 + tag.len() + 1; // </tag>
                in_hidden = None;
                in_tag = false;
                i += skip;
                continue;
            }
            i += 1;
            continue;
        }

        match bytes[i] {
            b'<' => in_tag = true,
            b'>' => {
                in_tag = false;
                if !last_was_space {
                    result.push(' ');
                    last_was_space = true;
                }
            }
            _ if !in_tag => {
                // Safe: we're checking single bytes against ASCII, but for
                // multi-byte UTF-8 chars we just push them through as-is
                let c = if bytes[i].is_ascii() {
                    bytes[i] as char
                } else {
                    // Decode the full UTF-8 char
                    let s = &html[i..];
                    let ch = s.chars().next().unwrap_or(' ');
                    let ch_len = ch.len_utf8();
                    if ch.is_whitespace() {
                        if !last_was_space {
                            result.push(' ');
                            last_was_space = true;
                        }
                        i += ch_len;
                        continue;
                    }
                    result.push(ch);
                    last_was_space = false;
                    i += ch_len;
                    continue;
                };
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

    #[test]
    fn test_web_fetch_127_blocked() {
        let result = web_fetch(&json!({"url": "http://127.0.0.1:9090/admin"}));
        assert!(result.get("error").is_some());
    }

    #[test]
    fn test_web_fetch_0000_blocked() {
        let result = web_fetch(&json!({"url": "http://0.0.0.0/secret"}));
        assert!(result.get("error").is_some());
    }

    #[test]
    fn test_strip_html_empty() {
        assert_eq!(strip_html_tags(""), "");
    }

    #[test]
    fn test_strip_html_plain_text() {
        assert_eq!(strip_html_tags("just plain text"), "just plain text");
    }

    #[test]
    fn test_strip_html_collapses_whitespace() {
        let result = strip_html_tags("hello     world");
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_strip_html_nested_tags() {
        let html = "<div><span><b>Bold</b></span></div>";
        let result = strip_html_tags(html);
        assert!(result.contains("Bold"));
        assert!(!result.contains("<"));
    }

    #[test]
    fn test_web_fetch_ftp_scheme_blocked() {
        let result = web_fetch(&json!({"url": "ftp://files.example.com/data.txt"}));
        assert!(result.get("error").is_some());
    }

    #[test]
    fn test_strip_html_multibyte() {
        let html = "<p>\u{1F600} Hello \u{1F389} World</p>";
        let result = strip_html_tags(html);
        assert!(result.contains("\u{1F600}"));
        assert!(result.contains("\u{1F389}"));
        assert!(result.contains("Hello"));
        assert!(!result.contains("<"));
    }

    #[test]
    fn test_strip_html_script_case_insensitive() {
        let html = "<p>Before</p><SCRIPT>bad();</SCRIPT><p>After</p>";
        let result = strip_html_tags(html);
        assert!(result.contains("Before"));
        assert!(result.contains("After"));
        assert!(!result.contains("bad"));
    }

    #[test]
    fn test_strip_html_style_removed() {
        let html =
            "<p>Hello</p><style>.foo { color: red; } body { margin: 0; }</style><p>World</p>";
        let result = strip_html_tags(html);
        assert!(result.contains("Hello"));
        assert!(result.contains("World"));
        assert!(!result.contains("color"));
        assert!(!result.contains("margin"));
    }

    #[test]
    fn test_truncation_at_char_boundary() {
        // Create a string with multi-byte chars that straddle the truncation point
        // Each emoji is 4 bytes. Place one so it spans across byte 20000.
        let mut s = "A".repeat(19_997); // 19997 ASCII chars
        s.push_str("\u{1F600}\u{1F600}");
        // Total: 20005 bytes. Byte 20000 is mid-emoji (starts at 19997).
        assert!(!s.is_char_boundary(20_000));
        let max_chars = 20_000;
        let mut end = max_chars;
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        let content = &s[..end];
        // Should back up to byte 19997 (start of the emoji)
        assert_eq!(content.len(), 19_997);
        // Verify it's valid UTF-8
        assert!(std::str::from_utf8(content.as_bytes()).is_ok());
    }

    #[test]
    fn test_strip_html_style_case_insensitive() {
        let html = "<p>A</p><STYLE>h1{font-size:2em}</STYLE><p>B</p>";
        let result = strip_html_tags(html);
        assert!(result.contains("A"));
        assert!(result.contains("B"));
        assert!(!result.contains("font-size"));
    }
}
