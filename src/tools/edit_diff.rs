//! Edit-diff helpers (Rust port of pi's edit-diff.ts).
//!
//! Provides:
//! - BOM stripping
//! - CRLF / CR line-ending detection + restoration
//! - Fuzzy matching (smart quotes, dashes, NBSP-style spaces, trailing whitespace)
//! - Multi-edit application with overlap detection
//! - Unified diff string generation with line numbers + context windows

#[derive(Debug, Clone)]
pub struct Edit {
    pub old_text: String,
    pub new_text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEnding {
    Lf,
    Crlf,
}

pub fn detect_line_ending(content: &str) -> LineEnding {
    let crlf_idx = content.find("\r\n");
    let lf_idx = content.find('\n');
    match (crlf_idx, lf_idx) {
        (Some(crlf), Some(lf)) if crlf <= lf => LineEnding::Crlf,
        (Some(_), None) => LineEnding::Crlf,
        _ => LineEnding::Lf,
    }
}

pub fn normalize_to_lf(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

pub fn restore_line_endings(text: &str, ending: LineEnding) -> String {
    match ending {
        LineEnding::Crlf => text.replace('\n', "\r\n"),
        LineEnding::Lf => text.to_string(),
    }
}

pub fn strip_bom(content: &str) -> (&str, &str) {
    if let Some(rest) = content.strip_prefix('\u{FEFF}') {
        ("\u{FEFF}", rest)
    } else {
        ("", content)
    }
}

/// Normalize text for fuzzy matching:
/// - strip trailing whitespace from each line
/// - smart quotes → ASCII quotes
/// - Unicode dashes → ASCII '-'
/// - Unicode spaces → ' '
pub fn normalize_for_fuzzy_match(text: &str) -> String {
    // Trim trailing whitespace per line.
    let trimmed: String = text
        .split('\n')
        .map(|line| line.trim_end().to_string())
        .collect::<Vec<_>>()
        .join("\n");

    let mut out = String::with_capacity(trimmed.len());
    for ch in trimmed.chars() {
        let mapped = match ch {
            '\u{2018}' | '\u{2019}' | '\u{201A}' | '\u{201B}' => '\'',
            '\u{201C}' | '\u{201D}' | '\u{201E}' | '\u{201F}' => '"',
            '\u{2010}' | '\u{2011}' | '\u{2012}' | '\u{2013}' | '\u{2014}' | '\u{2015}'
            | '\u{2212}' => '-',
            '\u{00A0}' | '\u{2002}' | '\u{2003}' | '\u{2004}' | '\u{2005}' | '\u{2006}'
            | '\u{2007}' | '\u{2008}' | '\u{2009}' | '\u{200A}' | '\u{202F}' | '\u{205F}'
            | '\u{3000}' => ' ',
            other => other,
        };
        out.push(mapped);
    }
    out
}

#[derive(Debug, Clone)]
pub struct FuzzyMatchResult {
    pub found: bool,
    pub index: usize,
    pub match_length: usize,
    pub used_fuzzy_match: bool,
    pub content_for_replacement: String,
}

/// Find `old_text` in `content`, exact match first, then fuzzy.
pub fn fuzzy_find_text(content: &str, old_text: &str) -> FuzzyMatchResult {
    if let Some(idx) = content.find(old_text) {
        return FuzzyMatchResult {
            found: true,
            index: idx,
            match_length: old_text.len(),
            used_fuzzy_match: false,
            content_for_replacement: content.to_string(),
        };
    }

    let fuzzy_content = normalize_for_fuzzy_match(content);
    let fuzzy_old = normalize_for_fuzzy_match(old_text);
    if let Some(idx) = fuzzy_content.find(&fuzzy_old) {
        return FuzzyMatchResult {
            found: true,
            index: idx,
            match_length: fuzzy_old.len(),
            used_fuzzy_match: true,
            content_for_replacement: fuzzy_content,
        };
    }

    FuzzyMatchResult {
        found: false,
        index: 0,
        match_length: 0,
        used_fuzzy_match: false,
        content_for_replacement: content.to_string(),
    }
}

fn count_occurrences(content: &str, old_text: &str) -> usize {
    let fc = normalize_for_fuzzy_match(content);
    let fo = normalize_for_fuzzy_match(old_text);
    if fo.is_empty() {
        return 0;
    }
    fc.matches(&fo).count()
}

#[derive(Debug)]
pub struct AppliedEdits {
    pub base_content: String,
    pub new_content: String,
}

/// Apply many exact-text replacements to LF-normalized content.
/// All edits match against the original; replacements are applied in
/// reverse order so byte offsets stay stable. Overlapping edits are
/// rejected with a clear error message.
pub fn apply_edits_to_normalized_content(
    normalized_content: &str,
    edits: &[Edit],
    path: &str,
) -> Result<AppliedEdits, String> {
    let total = edits.len();
    let normalized: Vec<Edit> = edits
        .iter()
        .map(|e| Edit {
            old_text: normalize_to_lf(&e.old_text),
            new_text: normalize_to_lf(&e.new_text),
        })
        .collect();

    for (i, e) in normalized.iter().enumerate() {
        if e.old_text.is_empty() {
            return Err(empty_old_text_error(path, i, total));
        }
    }

    let initial: Vec<FuzzyMatchResult> = normalized
        .iter()
        .map(|e| fuzzy_find_text(normalized_content, &e.old_text))
        .collect();
    let any_fuzzy = initial.iter().any(|m| m.used_fuzzy_match);
    let base_content = if any_fuzzy {
        normalize_for_fuzzy_match(normalized_content)
    } else {
        normalized_content.to_string()
    };

    let mut matched: Vec<(usize, usize, usize, String)> = Vec::with_capacity(normalized.len());
    for (i, e) in normalized.iter().enumerate() {
        let m = fuzzy_find_text(&base_content, &e.old_text);
        if !m.found {
            return Err(not_found_error(path, i, total));
        }
        let occ = count_occurrences(&base_content, &e.old_text);
        if occ > 1 {
            return Err(duplicate_error(path, i, total, occ));
        }
        matched.push((i, m.index, m.match_length, e.new_text.clone()));
    }

    matched.sort_by_key(|t| t.1);
    for w in matched.windows(2) {
        let (pi, pidx, plen, _) = &w[0];
        let (ci, cidx, _, _) = &w[1];
        if pidx + plen > *cidx {
            return Err(format!(
                "edits[{}] and edits[{}] overlap in {}. Merge them into one edit or target disjoint regions.",
                pi, ci, path
            ));
        }
    }

    let mut new_content = base_content.clone();
    for (_i, idx, len, new_text) in matched.iter().rev() {
        let mut s = String::with_capacity(new_content.len() + new_text.len());
        s.push_str(&new_content[..*idx]);
        s.push_str(new_text);
        s.push_str(&new_content[idx + len..]);
        new_content = s;
    }

    if base_content == new_content {
        return Err(no_change_error(path, total));
    }

    Ok(AppliedEdits {
        base_content,
        new_content,
    })
}

fn empty_old_text_error(path: &str, idx: usize, total: usize) -> String {
    if total == 1 {
        format!("oldText must not be empty in {}.", path)
    } else {
        format!("edits[{}].oldText must not be empty in {}.", idx, path)
    }
}

fn not_found_error(path: &str, idx: usize, total: usize) -> String {
    if total == 1 {
        format!(
            "Could not find the exact text in {}. The old text must match exactly including all whitespace and newlines.",
            path
        )
    } else {
        format!(
            "Could not find edits[{}] in {}. The oldText must match exactly including all whitespace and newlines.",
            idx, path
        )
    }
}

fn duplicate_error(path: &str, idx: usize, total: usize, occ: usize) -> String {
    if total == 1 {
        format!(
            "Found {} occurrences of the text in {}. The text must be unique. Please provide more context to make it unique.",
            occ, path
        )
    } else {
        format!(
            "Found {} occurrences of edits[{}] in {}. Each oldText must be unique. Please provide more context to make it unique.",
            occ, idx, path
        )
    }
}

fn no_change_error(path: &str, total: usize) -> String {
    if total == 1 {
        format!(
            "No changes made to {}. The replacement produced identical content. This might indicate an issue with special characters or the text not existing as expected.",
            path
        )
    } else {
        format!(
            "No changes made to {}. The replacements produced identical content.",
            path
        )
    }
}

/// Generate a compact unified diff with line numbers and ±4 lines of context.
/// Implements a very small LCS-free diff sufficient for tool output review.
pub fn generate_diff_string(old_content: &str, new_content: &str, context_lines: usize) -> String {
    let old_lines: Vec<&str> = old_content.split('\n').collect();
    let new_lines: Vec<&str> = new_content.split('\n').collect();

    // Compute edit script via classic LCS table — small files only, so quadratic is fine.
    let n = old_lines.len();
    let m = new_lines.len();
    let mut dp = vec![vec![0usize; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            dp[i][j] = if old_lines[i] == new_lines[j] {
                dp[i + 1][j + 1] + 1
            } else {
                dp[i + 1][j].max(dp[i][j + 1])
            };
        }
    }

    enum Op<'a> {
        Same(&'a str),
        Add(&'a str),
        Del(&'a str),
    }
    let mut ops: Vec<Op> = Vec::new();
    let mut i = 0;
    let mut j = 0;
    while i < n && j < m {
        if old_lines[i] == new_lines[j] {
            ops.push(Op::Same(old_lines[i]));
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            ops.push(Op::Del(old_lines[i]));
            i += 1;
        } else {
            ops.push(Op::Add(new_lines[j]));
            j += 1;
        }
    }
    while i < n {
        ops.push(Op::Del(old_lines[i]));
        i += 1;
    }
    while j < m {
        ops.push(Op::Add(new_lines[j]));
        j += 1;
    }

    let max_line_num = old_lines.len().max(new_lines.len()).max(1);
    let line_num_width = max_line_num.to_string().len();

    let mut out: Vec<String> = Vec::new();
    let mut old_line_num = 1usize;
    let mut new_line_num = 1usize;

    // Walk ops and emit only context windows around changes, like pi.
    let n_ops = ops.len();
    let mut idx = 0;
    let mut last_was_change = false;
    while idx < n_ops {
        match &ops[idx] {
            Op::Same(line) => {
                // Find the run of Same lines.
                let mut end = idx + 1;
                while end < n_ops {
                    if let Op::Same(_) = &ops[end] {
                        end += 1;
                    } else {
                        break;
                    }
                }
                let same_len = end - idx;
                let next_is_change = end < n_ops;
                let leading = last_was_change;
                let trailing = next_is_change;

                if leading && trailing {
                    if same_len <= context_lines * 2 {
                        for k in 0..same_len {
                            if let Op::Same(s) = &ops[idx + k] {
                                let l = format!(
                                    " {:>w$} {}",
                                    old_line_num,
                                    s,
                                    w = line_num_width
                                );
                                let _ = line;
                                out.push(l);
                                old_line_num += 1;
                                new_line_num += 1;
                            }
                        }
                    } else {
                        for k in 0..context_lines {
                            if let Op::Same(s) = &ops[idx + k] {
                                let l = format!(
                                    " {:>w$} {}",
                                    old_line_num,
                                    s,
                                    w = line_num_width
                                );
                                out.push(l);
                                old_line_num += 1;
                                new_line_num += 1;
                            }
                        }
                        out.push(format!(" {:>w$} ...", "", w = line_num_width));
                        let skipped = same_len - 2 * context_lines;
                        old_line_num += skipped;
                        new_line_num += skipped;
                        for k in (same_len - context_lines)..same_len {
                            if let Op::Same(s) = &ops[idx + k] {
                                let l = format!(
                                    " {:>w$} {}",
                                    old_line_num,
                                    s,
                                    w = line_num_width
                                );
                                out.push(l);
                                old_line_num += 1;
                                new_line_num += 1;
                            }
                        }
                    }
                } else if leading {
                    let shown = same_len.min(context_lines);
                    for k in 0..shown {
                        if let Op::Same(s) = &ops[idx + k] {
                            let l = format!(
                                " {:>w$} {}",
                                old_line_num,
                                s,
                                w = line_num_width
                            );
                            out.push(l);
                            old_line_num += 1;
                            new_line_num += 1;
                        }
                    }
                    if same_len > shown {
                        out.push(format!(" {:>w$} ...", "", w = line_num_width));
                        let skipped = same_len - shown;
                        old_line_num += skipped;
                        new_line_num += skipped;
                    }
                } else if trailing {
                    let shown = same_len.min(context_lines);
                    let skipped = same_len - shown;
                    if skipped > 0 {
                        out.push(format!(" {:>w$} ...", "", w = line_num_width));
                        old_line_num += skipped;
                        new_line_num += skipped;
                    }
                    for k in (same_len - shown)..same_len {
                        if let Op::Same(s) = &ops[idx + k] {
                            let l = format!(
                                " {:>w$} {}",
                                old_line_num,
                                s,
                                w = line_num_width
                            );
                            out.push(l);
                            old_line_num += 1;
                            new_line_num += 1;
                        }
                    }
                } else {
                    old_line_num += same_len;
                    new_line_num += same_len;
                }
                last_was_change = false;
                idx = end;
            }
            Op::Add(line) => {
                out.push(format!("+{:>w$} {}", new_line_num, line, w = line_num_width));
                new_line_num += 1;
                last_was_change = true;
                idx += 1;
            }
            Op::Del(line) => {
                out.push(format!("-{:>w$} {}", old_line_num, line, w = line_num_width));
                old_line_num += 1;
                last_was_change = true;
                idx += 1;
            }
        }
    }

    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_bom_present() {
        let (b, t) = strip_bom("\u{FEFF}hello");
        assert_eq!(b, "\u{FEFF}");
        assert_eq!(t, "hello");
    }

    #[test]
    fn test_strip_bom_absent() {
        let (b, t) = strip_bom("hello");
        assert_eq!(b, "");
        assert_eq!(t, "hello");
    }

    #[test]
    fn test_detect_line_ending_lf() {
        assert_eq!(detect_line_ending("a\nb\nc"), LineEnding::Lf);
    }

    #[test]
    fn test_detect_line_ending_crlf() {
        assert_eq!(detect_line_ending("a\r\nb\r\n"), LineEnding::Crlf);
    }

    #[test]
    fn test_normalize_to_lf() {
        assert_eq!(normalize_to_lf("a\r\nb\rc"), "a\nb\nc");
    }

    #[test]
    fn test_restore_crlf() {
        assert_eq!(restore_line_endings("a\nb", LineEnding::Crlf), "a\r\nb");
    }

    #[test]
    fn test_fuzzy_match_smart_quotes() {
        let content = "name = \u{2018}foo\u{2019}";
        let r = fuzzy_find_text(content, "name = 'foo'");
        assert!(r.found);
        assert!(r.used_fuzzy_match);
    }

    #[test]
    fn test_fuzzy_match_dashes() {
        let content = "use a\u{2014}b connector";
        let r = fuzzy_find_text(content, "use a-b connector");
        assert!(r.found);
        assert!(r.used_fuzzy_match);
    }

    #[test]
    fn test_fuzzy_match_trailing_whitespace() {
        let content = "line one  \nline two\n";
        let r = fuzzy_find_text(content, "line one\nline two");
        assert!(r.found);
        assert!(r.used_fuzzy_match);
    }

    #[test]
    fn test_apply_single_edit() {
        let r = apply_edits_to_normalized_content(
            "hello world",
            &[Edit {
                old_text: "world".into(),
                new_text: "rust".into(),
            }],
            "f.txt",
        )
        .unwrap();
        assert_eq!(r.new_content, "hello rust");
    }

    #[test]
    fn test_apply_multi_edit() {
        let content = "alpha\nbeta\ngamma\n";
        let r = apply_edits_to_normalized_content(
            content,
            &[
                Edit {
                    old_text: "alpha".into(),
                    new_text: "ALPHA".into(),
                },
                Edit {
                    old_text: "gamma".into(),
                    new_text: "GAMMA".into(),
                },
            ],
            "f.txt",
        )
        .unwrap();
        assert_eq!(r.new_content, "ALPHA\nbeta\nGAMMA\n");
    }

    #[test]
    fn test_apply_edit_duplicate_error() {
        let r = apply_edits_to_normalized_content(
            "hi hi",
            &[Edit {
                old_text: "hi".into(),
                new_text: "ho".into(),
            }],
            "f.txt",
        );
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("occurrences"));
    }

    #[test]
    fn test_apply_edit_not_found() {
        let r = apply_edits_to_normalized_content(
            "abc",
            &[Edit {
                old_text: "xyz".into(),
                new_text: "qrs".into(),
            }],
            "f.txt",
        );
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("Could not find"));
    }

    #[test]
    fn test_apply_overlap_error() {
        let r = apply_edits_to_normalized_content(
            "hello world",
            &[
                Edit {
                    old_text: "hello world".into(),
                    new_text: "x".into(),
                },
                Edit {
                    old_text: "world".into(),
                    new_text: "y".into(),
                },
            ],
            "f.txt",
        );
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("overlap"));
    }

    #[test]
    fn test_apply_no_change_error() {
        // Same old/new is rejected because it produces identical content.
        let r = apply_edits_to_normalized_content(
            "abc",
            &[Edit {
                old_text: "abc".into(),
                new_text: "abc".into(),
            }],
            "f.txt",
        );
        assert!(r.is_err());
    }

    #[test]
    fn test_diff_string_simple() {
        let d = generate_diff_string("hello\nworld\n", "hello\nrust\n", 4);
        assert!(d.contains("-"));
        assert!(d.contains("+"));
    }
}
