use crate::utils::safe_truncate;
use serde_json::{json, Value};
use std::process::Command;
use std::time::Duration;

/// Default timeout for shell commands (seconds)
const COMMAND_TIMEOUT_SECS: u64 = 60;

/// Run a command with a timeout, returning stdout/stderr or a timeout error
fn run_command_with_timeout(
    mut child: std::process::Child,
    timeout: Duration,
) -> Result<std::process::Output, String> {
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                // Process finished — collect output
                let mut stdout = Vec::new();
                let mut stderr = Vec::new();
                if let Some(mut out) = child.stdout.take() {
                    use std::io::Read;
                    let _ = out.read_to_end(&mut stdout);
                }
                if let Some(mut err) = child.stderr.take() {
                    use std::io::Read;
                    let _ = err.read_to_end(&mut stderr);
                }
                return Ok(std::process::Output {
                    status,
                    stdout,
                    stderr,
                });
            }
            Ok(None) => {
                // Still running
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait(); // reap zombie
                    return Err(format!("Command timed out after {}s", timeout.as_secs()));
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                return Err(format!("Error waiting for process: {}", e));
            }
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
            match run_command_with_timeout(child, Duration::from_secs(COMMAND_TIMEOUT_SECS)) {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    let max_output = 10_000;
                    if output.status.success() {
                        json!({
                            "success": true,
                            "output": safe_truncate(stdout.trim(), max_output)
                        })
                    } else {
                        json!({
                            "success": false,
                            "error": safe_truncate(stderr.trim(), max_output),
                            "output": safe_truncate(stdout.trim(), max_output)
                        })
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

    // Stage all changes first if requested
    let add_all = args
        .get("add_all")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if add_all {
        run_git(&["add", "-A"]);
    }

    run_git(&["commit", "-m", message])
}

pub fn git_add(args: &Value) -> Value {
    let files = match args.get("files").and_then(|v| v.as_str()) {
        Some(f) => f,
        None => return json!({"error": "Missing 'files' argument"}),
    };

    let file_list: Vec<&str> = files.split_whitespace().collect();
    let mut git_args = vec!["add"];
    git_args.extend(file_list);
    run_git(&git_args)
}

pub fn git_branch(args: &Value) -> Value {
    match args.get("name").and_then(|v| v.as_str()) {
        Some(name) => run_git(&["checkout", "-b", name]),
        None => run_git(&["branch", "-a"]),
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
    let branch = args.get("branch").and_then(|v| v.as_str());

    match branch {
        Some(b) => run_git(&["push", remote, b]),
        None => run_git(&["push", remote]),
    }
}

pub fn git_pull(args: &Value) -> Value {
    let remote = args
        .get("remote")
        .and_then(|v| v.as_str())
        .unwrap_or("origin");
    run_git(&["pull", remote])
}

/// Visible for testing
pub fn command_timeout_secs() -> u64 {
    COMMAND_TIMEOUT_SECS
}

pub fn run_shell(args: &Value) -> Value {
    let command = match args.get("command").and_then(|v| v.as_str()) {
        Some(c) => c,
        None => return json!({"error": "Missing 'command' argument"}),
    };

    // Safety: block dangerous commands
    let blocked = [
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
    for b in &blocked {
        if command.contains(b) {
            return json!({"error": format!("Blocked dangerous command: {}", b)});
        }
    }

    let timeout_secs = args
        .get("timeout")
        .and_then(|v| v.as_u64())
        .unwrap_or(COMMAND_TIMEOUT_SECS);

    let child = if cfg!(target_os = "windows") {
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

    match child {
        Ok(child) => match run_command_with_timeout(child, Duration::from_secs(timeout_secs)) {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let max_len = 10_000;

                json!({
                    "success": output.status.success(),
                    "exit_code": output.status.code(),
                    "stdout": safe_truncate(stdout.trim(), max_len),
                    "stderr": safe_truncate(stderr.trim(), max_len)
                })
            }
            Err(e) => json!({"error": e}),
        },
        Err(e) => json!({"error": format!("Failed to run command: {}", e)}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_timeout_default() {
        assert_eq!(command_timeout_secs(), 60);
    }

    #[test]
    fn test_git_status_returns_json() {
        let result = git_status(&json!({}));
        // Should have either "success" or "error" key
        assert!(result.get("success").is_some() || result.get("error").is_some());
    }

    #[test]
    fn test_git_log_default_count() {
        let result = git_log(&json!({}));
        assert!(result.get("success").is_some() || result.get("error").is_some());
    }

    #[test]
    fn test_git_log_custom_count() {
        let result = git_log(&json!({"count": 3}));
        assert!(result.get("success").is_some() || result.get("error").is_some());
    }

    #[test]
    fn test_git_diff_unstaged() {
        let result = git_diff(&json!({}));
        assert!(result.get("success").is_some() || result.get("error").is_some());
    }

    #[test]
    fn test_git_diff_staged() {
        let result = git_diff(&json!({"staged": true}));
        assert!(result.get("success").is_some() || result.get("error").is_some());
    }

    #[test]
    fn test_git_commit_no_message() {
        let result = git_commit(&json!({}));
        assert!(result.get("error").is_some());
        assert!(result["error"].as_str().unwrap().contains("Missing"));
    }

    #[test]
    fn test_git_add_no_files() {
        let result = git_add(&json!({}));
        assert!(result.get("error").is_some());
        assert!(result["error"].as_str().unwrap().contains("Missing"));
    }

    #[test]
    fn test_git_checkout_no_branch() {
        let result = git_checkout(&json!({}));
        assert!(result.get("error").is_some());
        assert!(result["error"].as_str().unwrap().contains("Missing"));
    }

    #[test]
    fn test_run_shell_no_command() {
        let result = run_shell(&json!({}));
        assert!(result.get("error").is_some());
        assert!(result["error"].as_str().unwrap().contains("Missing"));
    }

    #[test]
    fn test_run_shell_echo() {
        let result = run_shell(&json!({"command": "echo hello"}));
        assert_eq!(result["success"], true);
        assert_eq!(result["stdout"], "hello");
    }

    #[test]
    fn test_run_shell_blocked_rm_rf() {
        let result = run_shell(&json!({"command": "rm -rf /"}));
        assert!(result["error"].as_str().unwrap().contains("Blocked"));
    }

    #[test]
    fn test_run_shell_blocked_fork_bomb() {
        let result = run_shell(&json!({"command": ":(){ :|:& };:"}));
        assert!(result["error"].as_str().unwrap().contains("Blocked"));
    }

    #[test]
    fn test_run_shell_blocked_dd() {
        let result = run_shell(&json!({"command": "dd if=/dev/zero of=/dev/sda"}));
        assert!(result["error"].as_str().unwrap().contains("Blocked"));
    }

    #[test]
    fn test_run_shell_timeout() {
        let result = run_shell(&json!({"command": "sleep 10", "timeout": 1}));
        assert!(result.get("error").is_some());
        assert!(result["error"].as_str().unwrap().contains("timed out"));
    }

    #[test]
    fn test_run_shell_custom_timeout() {
        let result = run_shell(&json!({"command": "echo fast", "timeout": 5}));
        assert_eq!(result["success"], true);
    }

    #[test]
    fn test_run_shell_exit_code() {
        let result = run_shell(&json!({"command": "exit 42"}));
        assert_eq!(result["success"], false);
        assert_eq!(result["exit_code"], 42);
    }

    #[test]
    fn test_run_shell_stderr() {
        let result = run_shell(&json!({"command": "echo err >&2"}));
        assert_eq!(result["stderr"], "err");
    }

    #[test]
    fn test_run_command_with_timeout_success() {
        let child = Command::new("echo")
            .arg("test")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let result = run_command_with_timeout(child, Duration::from_secs(5));
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.status.success());
    }

    #[test]
    fn test_run_command_with_timeout_kills() {
        let child = Command::new("sleep")
            .arg("30")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let result = run_command_with_timeout(child, Duration::from_secs(1));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("timed out"));
    }

    #[test]
    fn test_git_branch_list() {
        let result = git_branch(&json!({}));
        assert!(result.get("success").is_some() || result.get("error").is_some());
    }

    #[test]
    fn test_git_push_default_remote() {
        // Will fail since no remote configured in test env, but should not panic
        let result = git_push(&json!({}));
        assert!(result.get("success").is_some() || result.get("error").is_some());
    }

    #[test]
    fn test_run_shell_blocked_sudo_rm() {
        let result = run_shell(&json!({"command": "sudo rm -rf /home"}));
        assert!(result["error"].as_str().unwrap().contains("Blocked"));
    }

    #[test]
    fn test_run_shell_blocked_chmod_777() {
        let result = run_shell(&json!({"command": "chmod -R 777 /"}));
        assert!(result["error"].as_str().unwrap().contains("Blocked"));
    }

    #[test]
    fn test_run_shell_blocked_dev_sda() {
        let result = run_shell(&json!({"command": "> /dev/sda"}));
        assert!(result["error"].as_str().unwrap().contains("Blocked"));
    }

    #[test]
    fn test_run_shell_blocked_mkfs() {
        let result = run_shell(&json!({"command": "mkfs.ext4 /dev/sdb1"}));
        assert!(result["error"].as_str().unwrap().contains("Blocked"));
    }

    #[test]
    fn test_run_shell_blocked_rm_rf_home() {
        let result = run_shell(&json!({"command": "rm -rf ~"}));
        assert!(result["error"].as_str().unwrap().contains("Blocked"));
    }

    #[test]
    fn test_git_pull() {
        // Will likely error in test env but should not panic
        let result = git_pull(&json!({}));
        assert!(result.get("success").is_some() || result.get("error").is_some());
    }

    #[test]
    fn test_git_push_with_remote_and_branch() {
        let result = git_push(&json!({"remote": "origin", "branch": "test-branch"}));
        assert!(result.get("success").is_some() || result.get("error").is_some());
    }

    #[test]
    fn test_safe_truncate_short() {
        assert_eq!(safe_truncate("hello", 10), "hello");
    }

    #[test]
    fn test_safe_truncate_exact() {
        assert_eq!(safe_truncate("hello", 5), "hello");
    }

    #[test]
    fn test_safe_truncate_truncates() {
        let result = safe_truncate("hello world", 5);
        assert_eq!(result, "hello...(truncated)");
    }

    #[test]
    fn test_safe_truncate_multibyte() {
        // "a😀b" = 1 + 4 + 1 = 6 bytes; truncate at 3 should back up to byte 1
        let s = "a\u{1F600}b";
        let result = safe_truncate(s, 3);
        assert!(result.starts_with("a"));
        assert!(result.ends_with("...(truncated)"));
    }

    #[test]
    fn test_safe_truncate_empty() {
        assert_eq!(safe_truncate("", 10), "");
    }

    #[test]
    fn test_run_shell_blocked_dd_of() {
        let result = run_shell(&json!({"command": "dd of=/dev/sda bs=512"}));
        assert!(result["error"].as_str().unwrap().contains("Blocked"));
    }

    #[test]
    fn test_run_shell_blocked_chown_recursive() {
        let result = run_shell(&json!({"command": "chown -R / root:root"}));
        assert!(result["error"].as_str().unwrap().contains("Blocked"));
    }

    #[test]
    fn test_run_shell_blocked_redirect_dev() {
        let result = run_shell(&json!({"command": "echo x > /dev/null"}));
        assert!(result["error"].as_str().unwrap().contains("Blocked"));
    }

    #[test]
    fn test_run_shell_blocked_redirect_etc() {
        let result = run_shell(&json!({"command": "echo x > /etc/hosts"}));
        assert!(result["error"].as_str().unwrap().contains("Blocked"));
    }

    #[test]
    fn test_run_shell_blocked_redirect_boot() {
        let result = run_shell(&json!({"command": "echo x > /boot/grub.cfg"}));
        assert!(result["error"].as_str().unwrap().contains("Blocked"));
    }
}
