//! `bash` tool — port of pi's bash.ts (Unix sh / Windows cmd).
//!
//! - Streams stdout+stderr together
//! - Tail-truncates output to last 2000 lines / 50KB (matches pi)
//! - Optional `timeout` in seconds; falls back to VSC's configured default
//! - Same blocked-command list as the previous run_shell tool
//!
//! VSC keeps a default timeout (60s, configurable via `/config set
//! command_timeout`) so RPi3 sessions never deadlock on a runaway process.
//! Pi's no-default-timeout would be unsafe on a 1GB device.

use serde_json::{json, Value};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use crate::tools::truncate::{format_size, truncate_tail, TruncationOptions, DEFAULT_MAX_BYTES};

const DEFAULT_COMMAND_TIMEOUT_SECS: u64 = 60;
static COMMAND_TIMEOUT: AtomicU64 = AtomicU64::new(DEFAULT_COMMAND_TIMEOUT_SECS);

pub fn set_command_timeout_secs(secs: u64) {
    COMMAND_TIMEOUT.store(secs.clamp(5, 600), Ordering::Relaxed);
}

pub fn command_timeout_secs() -> u64 {
    COMMAND_TIMEOUT.load(Ordering::Relaxed)
}

const BLOCKED: &[&str] = &[
    "rm -rf /",
    "rm -rf ~",
    "mkfs",
    "dd if=",
    "dd of=",
    ":(){ :|:& };:",
    "chmod -R 777 /",
    "chown -R /",
    "sudo rm",
    "> /dev/sda",
    "> /dev/",
    "> /etc/",
    "> /boot/",
];

fn wait_for_output(
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

pub fn bash(args: &Value) -> Value {
    let command = match args.get("command").and_then(|v| v.as_str()) {
        Some(c) => c,
        None => return json!({"error": "Missing 'command' argument"}),
    };

    for b in BLOCKED {
        if command.contains(b) {
            return json!({"error": format!("Blocked dangerous command: {}", b)});
        }
    }

    let timeout_secs = args
        .get("timeout")
        .and_then(|v| v.as_u64())
        .unwrap_or_else(command_timeout_secs);

    let child_res = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .arg("/C")
            .arg(command)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
    } else {
        Command::new("sh")
            .arg("-c")
            .arg(command)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
    };

    let child = match child_res {
        Ok(c) => c,
        Err(e) => return json!({"error": format!("Failed to spawn: {}", e)}),
    };

    match wait_for_output(child, Duration::from_secs(timeout_secs)) {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            // Concatenate stdout + stderr for tail truncation (pi merges them).
            let combined = if stderr.is_empty() {
                stdout.clone()
            } else if stdout.is_empty() {
                stderr.clone()
            } else {
                format!("{}\n{}", stdout, stderr)
            };
            let truncation = truncate_tail(&combined, TruncationOptions::default());
            let mut out_text = if truncation.content.is_empty() {
                "(no output)".to_string()
            } else {
                truncation.content.clone()
            };
            if truncation.truncated {
                let start_line = truncation.total_lines - truncation.output_lines + 1;
                let end_line = truncation.total_lines;
                out_text.push_str(&format!(
                    "\n\n[Showing lines {}-{} of {} ({} limit)]",
                    start_line,
                    end_line,
                    truncation.total_lines,
                    format_size(DEFAULT_MAX_BYTES)
                ));
            }
            let mut payload = json!({
                "success": output.status.success(),
                "exit_code": output.status.code(),
                "stdout": stdout,
                "stderr": stderr,
                "output": out_text,
                "content": out_text,
            });
            if truncation.truncated {
                payload["truncated"] = json!(true);
                payload["total_bytes"] = json!(combined.len());
            }
            payload
        }
        Err(e) => json!({"error": e}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bash_missing_command() {
        let r = bash(&json!({}));
        assert!(r.get("error").is_some());
    }

    #[test]
    fn test_bash_echo() {
        let r = bash(&json!({"command": "echo hi"}));
        assert_eq!(r["success"], true);
        assert!(r["output"].as_str().unwrap().contains("hi"));
    }

    #[test]
    fn test_bash_blocked_rm_rf() {
        let r = bash(&json!({"command": "rm -rf /"}));
        assert!(r.get("error").is_some());
    }

    #[test]
    fn test_bash_blocked_dd() {
        let r = bash(&json!({"command": "dd if=/dev/zero of=/dev/sda"}));
        assert!(r.get("error").is_some());
    }

    #[test]
    fn test_bash_timeout() {
        let r = bash(&json!({"command": "sleep 5", "timeout": 1}));
        assert!(r.get("error").is_some());
    }

    #[test]
    fn test_bash_exit_code() {
        let r = bash(&json!({"command": "exit 42"}));
        assert_eq!(r["exit_code"], 42);
    }

    #[test]
    fn test_bash_stderr_captured() {
        let r = bash(&json!({"command": "echo err >&2"}));
        assert!(r["output"].as_str().unwrap().contains("err"));
    }

    #[test]
    fn test_set_timeout_clamped() {
        set_command_timeout_secs(1);
        assert_eq!(command_timeout_secs(), 5);
        set_command_timeout_secs(700);
        assert_eq!(command_timeout_secs(), 600);
        set_command_timeout_secs(60);
    }
}
