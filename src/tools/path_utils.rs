//! Path resolution helpers (Rust port of pi's path-utils.ts).
//!
//! - `expand_path` handles `~` expansion and Unicode-space normalization.
//! - `resolve_to_cwd` joins relative paths to a given working directory.
//! - `resolve_read_path` falls back to NFD / smart-quote variants used by
//!   macOS for screenshots, mirroring pi's behavior so models can refer to
//!   files using straight quotes / NFC even when the filesystem stores NFD.

use std::path::{Path, PathBuf};

const UNICODE_SPACES: &[char] = &[
    '\u{00A0}', '\u{2000}', '\u{2001}', '\u{2002}', '\u{2003}', '\u{2004}', '\u{2005}', '\u{2006}',
    '\u{2007}', '\u{2008}', '\u{2009}', '\u{200A}', '\u{202F}', '\u{205F}', '\u{3000}',
];

const NARROW_NO_BREAK_SPACE: char = '\u{202F}';

fn normalize_unicode_spaces(s: &str) -> String {
    s.chars()
        .map(|c| if UNICODE_SPACES.contains(&c) { ' ' } else { c })
        .collect()
}

fn normalize_at_prefix(s: &str) -> &str {
    s.strip_prefix('@').unwrap_or(s)
}

/// Expand `~` and `~/` to the user's home directory; normalize unicode spaces.
pub fn expand_path(path: &str) -> PathBuf {
    let stripped = normalize_at_prefix(path);
    let normalized = normalize_unicode_spaces(stripped);
    if normalized == "~" {
        return dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"));
    }
    if let Some(rest) = normalized.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(normalized)
}

/// Resolve `path` relative to `cwd` (after `~` expansion).
pub fn resolve_to_cwd(path: &str, cwd: &Path) -> PathBuf {
    let expanded = expand_path(path);
    if expanded.is_absolute() {
        expanded
    } else {
        cwd.join(expanded)
    }
}

/// Like `resolve_to_cwd`, but also tries macOS-friendly variants if the file
/// does not exist (NFD form, narrow no-break space before AM/PM, U+2019 curly
/// apostrophe). These mirror pi's resolveReadPath().
pub fn resolve_read_path(path: &str, cwd: &Path) -> PathBuf {
    let resolved = resolve_to_cwd(path, cwd);
    if resolved.exists() {
        return resolved;
    }

    let s = resolved.to_string_lossy().to_string();

    // AM/PM narrow no-break space variant
    let am_pm = try_macos_screenshot_path(&s);
    if am_pm != s && Path::new(&am_pm).exists() {
        return PathBuf::from(am_pm);
    }

    // NFC / NFD: we don't pull in unicode-normalization to keep RPi3 binary
    // small. Skip that variant; cover the common curly-quote / NBSP cases.
    let curly = try_curly_quote_variant(&s);
    if curly != s && Path::new(&curly).exists() {
        return PathBuf::from(curly);
    }

    resolved
}

fn try_macos_screenshot_path(s: &str) -> String {
    // Replace " AM." / " PM." with NARROW_NO_BREAK_SPACE + "AM." / "PM."
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    let bytes = s.as_bytes();
    while i < bytes.len() {
        if i + 4 <= bytes.len() {
            let slice = &s[i..i + 4];
            let upper = slice.to_uppercase();
            if upper == " AM." || upper == " PM." {
                out.push(NARROW_NO_BREAK_SPACE);
                out.push_str(&slice[1..]);
                i += 4;
                continue;
            }
        }
        // Walk a single char
        let ch = s[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn try_curly_quote_variant(s: &str) -> String {
    s.replace('\'', "\u{2019}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_expand_tilde() {
        let home = dirs::home_dir().unwrap();
        assert_eq!(expand_path("~"), home);
        assert_eq!(expand_path("~/foo"), home.join("foo"));
    }

    #[test]
    fn test_expand_at_prefix() {
        let home = dirs::home_dir().unwrap();
        assert_eq!(expand_path("@~/foo"), home.join("foo"));
    }

    #[test]
    fn test_expand_absolute() {
        assert_eq!(expand_path("/etc/hosts"), PathBuf::from("/etc/hosts"));
    }

    #[test]
    fn test_expand_relative() {
        assert_eq!(expand_path("foo/bar"), PathBuf::from("foo/bar"));
    }

    #[test]
    fn test_resolve_to_cwd_absolute() {
        let cwd = PathBuf::from("/tmp");
        assert_eq!(
            resolve_to_cwd("/etc/hosts", &cwd),
            PathBuf::from("/etc/hosts")
        );
    }

    #[test]
    fn test_resolve_to_cwd_relative() {
        let cwd = PathBuf::from("/tmp");
        assert_eq!(resolve_to_cwd("foo.txt", &cwd), PathBuf::from("/tmp/foo.txt"));
    }

    #[test]
    fn test_resolve_read_path_existing() {
        let tmp = env::temp_dir().join("vsc_path_utils_test.txt");
        std::fs::write(&tmp, "data").unwrap();
        assert_eq!(
            resolve_read_path(tmp.to_str().unwrap(), &env::current_dir().unwrap()),
            tmp
        );
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_curly_quote_variant() {
        assert_eq!(try_curly_quote_variant("don't"), "don\u{2019}t");
    }

    #[test]
    fn test_unicode_space_normalization() {
        let nbsp_path = "foo\u{00A0}bar";
        let result = expand_path(nbsp_path);
        assert_eq!(result.to_string_lossy(), "foo bar");
    }
}
