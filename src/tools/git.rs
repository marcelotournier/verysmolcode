//! Git tools — kept as a VSC extension on top of pi's coding-agent core.
//!
//! Pi has no git tools; the model uses `bash` plus the host's git CLI. VSC
//! ships first-class `git_*` tools because the rate-limited Gemini free tier
//! benefits from compact, structured outputs (smaller context cost than raw
//! `git status`/`git diff` shelling out through `bash`).
//!
//! `run_shell` / `run_command` were removed — use the `bash` tool instead.

use crate::utils::safe_truncate;
use serde_json::{json, Value};
use std::process::Command;
use std::sync::atomic::Ordering;
use std::time::Duration;

// Re-exported so existing callers (config bootstrap, /config command) keep
// working after the run_shell move.
pub use crate::tools::bash::{command_timeout_secs as _command_timeout_secs, set_command_timeout_secs};

/// Re-export under the original name for `config.rs` and other callers.
pub fn command_timeout_secs() -> u64 {
    _command_timeout_secs()
}

fn run_command_with_timeout(
    mut child: std::process::Child,
    timeout: Duration,
) -> Result<std::process::Output, String> {
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                use std::io::Read;
                let mut stdout = Vec::new();
                let mut stderr = Vec::new();
                if let Some(mut o) = child.stdout.take() {
                    let _ = o.read_to_end(&mut stdout);
                }
                if let Some(mut e) = child.stderr.take() {
                    let _ = e.read_to_end(&mut stderr);
                }
                return Ok(std::process::Output {
                    status,
                    stdout,
                    stderr,
                });
            }
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("Command timed out after {}s", timeout.as_secs()));
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => return Err(format!("Error waiting for process: {}", e)),
        }
    }
}

fn run_git(args: &[&str]) -> Value {
    let child = Command::new("git")
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn();
    match child {
        Ok(child) => {
            let timeout = Duration::from_secs(
                crate::tools::bash::command_timeout_secs().clamp(5, 600),
            );
            // Atomic load avoids extra mut borrow.
            let _ = Ordering::Relaxed;
            match run_command_with_timeout(child, timeout) {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    let max_output = 10_000;
                    let truncated = stdout.len() > max_output || stderr.len() > max_output;
                    if output.status.success() {
                        let mut result = json!({
                            "success": true,
                            "output": safe_truncate(stdout.trim(), max_output),
                            "content": safe_truncate(stdout.trim(), max_output)
                        });
                        if truncated {
                            result["truncated"] = json!(true);
                            result["total_bytes"] = json!(stdout.len());
                        }
                        result
                    } else {
                        let mut result = json!({
                            "success": false,
                            "error": safe_truncate(stderr.trim(), max_output),
                            "output": safe_truncate(stdout.trim(), max_output)
                        });
                        if truncated {
                            result["truncated"] = json!(true);
                        }
                        result
                    }
                }
                Err(e) => json!({"error": e}),
            }
        }
        Err(e) => json!({"error": format!("Failed to run git: {}", e)}),
    }
}

pub fn git_status(_args: &Value) -> Value {
    run_git(&["status", "--short"])
}

pub fn git_diff(args: &Value) -> Value {
    let staged = args
        .get("staged")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if staged {
        run_git(&["diff", "--cached"])
    } else {
        run_git(&["diff"])
    }
}

pub fn git_log(args: &Value) -> Value {
    let count = args.get("count").and_then(|v| v.as_u64()).unwrap_or(10);
    let count_str = format!("-{}", count);
    run_git(&["log", "--oneline", &count_str])
}

pub fn git_commit(args: &Value) -> Value {
    let message = match args.get("message").and_then(|v| v.as_str()) {
        Some(m) => m,
        None => return json!({"error": "Missing 'message' argument"}),
    };
    let add_all = args
        .get("add_all")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if add_all {
        let _ = run_git(&["add", "-A"]);
    }
    run_git(&["commit", "-m", message])
}

pub fn git_add(args: &Value) -> Value {
    let files = match args.get("files").and_then(|v| v.as_str()) {
        Some(f) => f,
        None => return json!({"error": "Missing 'files' argument"}),
    };
    let parts: Vec<&str> = files.split_whitespace().collect();
    let mut argv = vec!["add"];
    argv.extend(parts.iter().copied());
    run_git(&argv)
}

pub fn git_branch(args: &Value) -> Value {
    if let Some(name) = args.get("name").and_then(|v| v.as_str()) {
        run_git(&["checkout", "-b", name])
    } else {
        run_git(&["branch", "--list"])
    }
}

pub fn git_checkout(args: &Value) -> Value {
    let branch = match args.get("branch").and_then(|v| v.as_str()) {
        Some(b) => b,
        None => return json!({"error": "Missing 'branch' argument"}),
    };
    run_git(&["checkout", branch])
}

pub fn git_push(args: &Value) -> Value {
    let remote = args
        .get("remote")
        .and_then(|v| v.as_str())
        .unwrap_or("origin");
    if let Some(branch) = args.get("branch").and_then(|v| v.as_str()) {
        run_git(&["push", remote, branch])
    } else {
        run_git(&["push", remote])
    }
}

pub fn git_pull(args: &Value) -> Value {
    let remote = args
        .get("remote")
        .and_then(|v| v.as_str())
        .unwrap_or("origin");
    run_git(&["pull", remote])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_timeout_in_range() {
        let t = command_timeout_secs();
        assert!((5..=600).contains(&t));
    }

    #[test]
    fn test_git_status_returns_json() {
        let result = git_status(&json!({}));
        assert!(result.get("success").is_some() || result.get("error").is_some());
    }

    #[test]
    fn test_git_log_count_arg() {
        // Should not panic on missing count
        let result = git_log(&json!({}));
        assert!(result.get("success").is_some() || result.get("error").is_some());
    }

    #[test]
    fn test_git_commit_missing_message() {
        let r = git_commit(&json!({}));
        assert!(r.get("error").is_some());
    }

    #[test]
    fn test_git_add_missing_files() {
        let r = git_add(&json!({}));
        assert!(r.get("error").is_some());
    }

    #[test]
    fn test_git_checkout_missing_branch() {
        let r = git_checkout(&json!({}));
        assert!(r.get("error").is_some());
    }
}
