//! `edit` tool — port of pi's edit.ts.
//!
//! Supports two argument shapes:
//! - Pi style: `{path, edits: [{oldText, newText}, ...]}` — multi-replacement
//! - Legacy VSC style: `{path, old_string, new_string, replace_all?}`
//!
//! Both flow through the same fuzzy-aware matching and per-file mutation queue.

use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

use crate::tools::edit_diff::{
    apply_edits_to_normalized_content, detect_line_ending, generate_diff_string, normalize_to_lf,
    restore_line_endings, strip_bom, Edit,
};
use crate::tools::file_mutation_queue::with_file_mutation_queue;
use crate::tools::file_ops::check_safe_path;
use crate::tools::path_utils::resolve_to_cwd;

fn extract_edits(args: &Value) -> Result<Vec<Edit>, String> {
    if let Some(arr) = args.get("edits").and_then(|v| v.as_array()) {
        let mut edits = Vec::with_capacity(arr.len());
        for (i, item) in arr.iter().enumerate() {
            // Accept both {oldText,newText} (pi) and {old_text,new_text} (snake)
            let old = item
                .get("oldText")
                .or_else(|| item.get("old_text"))
                .or_else(|| item.get("old_string"))
                .and_then(|v| v.as_str());
            let new = item
                .get("newText")
                .or_else(|| item.get("new_text"))
                .or_else(|| item.get("new_string"))
                .and_then(|v| v.as_str());
            match (old, new) {
                (Some(o), Some(n)) => edits.push(Edit {
                    old_text: o.to_string(),
                    new_text: n.to_string(),
                }),
                _ => {
                    return Err(format!(
                        "edits[{}] must have both oldText and newText (or old_text/new_text)",
                        i
                    ))
                }
            }
        }
        if edits.is_empty() {
            return Err("edits[] must contain at least one replacement".to_string());
        }
        return Ok(edits);
    }

    // Legacy single-edit flat shape (preserved for back-compat)
    let old = args
        .get("old_string")
        .or_else(|| args.get("oldText"))
        .and_then(|v| v.as_str());
    let new = args
        .get("new_string")
        .or_else(|| args.get("newText"))
        .and_then(|v| v.as_str());
    match (old, new) {
        (Some(o), Some(n)) => Ok(vec![Edit {
            old_text: o.to_string(),
            new_text: n.to_string(),
        }]),
        _ => Err("Missing edits[] (or old_string/new_string) arguments".to_string()),
    }
}

pub fn edit(args: &Value) -> Value {
    let path = match args.get("path").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => return json!({"error": "Missing 'path' argument"}),
    };
    let replace_all = args
        .get("replace_all")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let edits = match extract_edits(args) {
        Ok(e) => e,
        Err(msg) => return json!({"error": msg}),
    };

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let absolute = resolve_to_cwd(path, &cwd);

    if let Err(e) = check_safe_path(&absolute) {
        return json!({"error": e});
    }

    with_file_mutation_queue(&absolute, || {
        let raw = match fs::read_to_string(&absolute) {
            Ok(s) => s,
            Err(e) => return json!({"error": format!("Could not edit file: {}. {}", path, e)}),
        };

        let (bom, content_no_bom) = strip_bom(&raw);
        let original_ending = detect_line_ending(content_no_bom);
        let normalized = normalize_to_lf(content_no_bom);

        // replace_all is a legacy single-edit affordance: if the model asks to
        // replace every occurrence, we expand the single edit into N matches
        // BEFORE handing off to apply_edits, by walking the content. Multi-edit
        // pi-style requests are processed as-is.
        let result = if replace_all && edits.len() == 1 {
            let edit = &edits[0];
            let count = normalized.matches(&edit.old_text).count();
            if count == 0 {
                return json!({"error": format!(
                    "Could not find the exact text in {}. The old text must match exactly including all whitespace and newlines.",
                    path
                )});
            }
            let new_content = normalized.replace(&edit.old_text, &edit.new_text);
            Ok((normalized.clone(), new_content, count))
        } else {
            apply_edits_to_normalized_content(&normalized, &edits, path)
                .map(|r| (r.base_content, r.new_content, edits.len()))
        };

        let (base_content, new_content, applied) = match result {
            Ok(t) => t,
            Err(msg) => {
                let mut payload = json!({"error": msg});
                // Helpful hints (kept from VSC)
                let normalized_content = normalized.replace('\t', "    ");
                let mut hint: Option<&str> = None;
                if let Some(first_edit) = edits.first() {
                    let normalized_old = first_edit.old_text.replace('\t', "    ");
                    if normalized_content.contains(&normalized_old) {
                        hint = Some("Hint: found match with different whitespace (tabs vs spaces)");
                    } else if normalized
                        .to_lowercase()
                        .contains(&first_edit.old_text.to_lowercase())
                    {
                        hint = Some("Hint: found case-insensitive match — check exact casing");
                    } else {
                        hint = Some("Read the file first to see its current content");
                    }
                }
                if let Some(h) = hint {
                    payload["hint"] = json!(h);
                }
                return payload;
            }
        };

        let final_content = format!(
            "{}{}",
            bom,
            restore_line_endings(&new_content, original_ending)
        );
        if let Err(e) = fs::write(&absolute, &final_content) {
            return json!({"error": format!("Failed to write: {}", e)});
        }

        let diff = generate_diff_string(&base_content, &new_content, 4);
        json!({
            "success": true,
            "path": absolute.display().to_string(),
            "replacements": applied,
            "diff": diff,
            "content": format!("Successfully replaced {} block(s) in {}.", applied, path)
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("vsc_edit_{}", name))
    }

    #[test]
    fn test_edit_legacy_single() {
        let p = tmp("legacy.txt");
        fs::write(&p, "hello world").unwrap();
        let r = edit(&json!({
            "path": p.to_str().unwrap(),
            "old_string": "world",
            "new_string": "rust"
        }));
        assert_eq!(r["success"], true);
        assert_eq!(fs::read_to_string(&p).unwrap(), "hello rust");
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn test_edit_multi_pi_style() {
        let p = tmp("multi.txt");
        fs::write(&p, "alpha\nbeta\ngamma\n").unwrap();
        let r = edit(&json!({
            "path": p.to_str().unwrap(),
            "edits": [
                {"oldText": "alpha", "newText": "A"},
                {"oldText": "gamma", "newText": "G"}
            ]
        }));
        assert_eq!(r["success"], true);
        assert_eq!(r["replacements"], 2);
        assert_eq!(fs::read_to_string(&p).unwrap(), "A\nbeta\nG\n");
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn test_edit_fuzzy_smart_quotes() {
        let p = tmp("fuzzy.txt");
        fs::write(&p, "name = \u{2018}foo\u{2019}").unwrap();
        let r = edit(&json!({
            "path": p.to_str().unwrap(),
            "old_string": "name = 'foo'",
            "new_string": "name = 'bar'"
        }));
        assert_eq!(r["success"], true);
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn test_edit_replace_all_legacy() {
        let p = tmp("repl_all.txt");
        fs::write(&p, "x x x x").unwrap();
        let r = edit(&json!({
            "path": p.to_str().unwrap(),
            "old_string": "x",
            "new_string": "y",
            "replace_all": true
        }));
        assert_eq!(r["success"], true);
        assert_eq!(fs::read_to_string(&p).unwrap(), "y y y y");
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn test_edit_ambiguous_returns_hint() {
        let p = tmp("ambig.txt");
        fs::write(&p, "foo foo").unwrap();
        let r = edit(&json!({
            "path": p.to_str().unwrap(),
            "old_string": "foo",
            "new_string": "bar"
        }));
        assert!(r.get("error").is_some());
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn test_edit_preserves_crlf() {
        let p = tmp("crlf.txt");
        fs::write(&p, "alpha\r\nbeta\r\n").unwrap();
        let r = edit(&json!({
            "path": p.to_str().unwrap(),
            "old_string": "alpha",
            "new_string": "ALPHA"
        }));
        assert_eq!(r["success"], true);
        let after = fs::read_to_string(&p).unwrap();
        assert!(after.contains("\r\n"));
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn test_edit_blocks_system_path() {
        let r = edit(&json!({
            "path": "/etc/passwd",
            "old_string": "x",
            "new_string": "y"
        }));
        assert!(r.get("error").is_some());
    }
}
