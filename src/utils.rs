/// Truncate a string at a safe UTF-8 char boundary, appending a suffix if truncated.
pub fn safe_truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        return s.to_string();
    }
    let mut end = max_len;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}...(truncated)", &s[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_string_unchanged() {
        assert_eq!(safe_truncate("hello", 10), "hello");
    }

    #[test]
    fn test_exact_length_unchanged() {
        assert_eq!(safe_truncate("hello", 5), "hello");
    }

    #[test]
    fn test_truncates_long_string() {
        assert_eq!(safe_truncate("hello world", 5), "hello...(truncated)");
    }

    #[test]
    fn test_multibyte_boundary() {
        // "a😀b" = 1 + 4 + 1 = 6 bytes; truncate at 3 backs up to byte 1
        let s = "a\u{1F600}b";
        let result = safe_truncate(s, 3);
        assert_eq!(result, "a...(truncated)");
    }

    #[test]
    fn test_empty_string() {
        assert_eq!(safe_truncate("", 10), "");
    }

    #[test]
    fn test_zero_max_len() {
        assert_eq!(safe_truncate("hello", 0), "...(truncated)");
    }
}
