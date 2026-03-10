use serde_json::{json, Value};
use std::process::Command;

fn run_git(args: &[&str]) -> Value {
    match Command::new("git").args(args).output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            if output.status.success() {
                json!({
                    "success": true,
                    "output": stdout.trim()
                })
            } else {
                json!({
                    "success": false,
                    "error": stderr.trim(),
                    "output": stdout.trim()
                })
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
        ":(){ :|:& };:",
        "chmod -R 777 /",
        "sudo rm",
        "> /dev/sda",
    ];
    for b in &blocked {
        if command.contains(b) {
            return json!({"error": format!("Blocked dangerous command: {}", b)});
        }
    }

    match Command::new("sh").arg("-c").arg(command).output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Truncate long output
            let max_len = 10_000;
            let stdout_str = if stdout.len() > max_len {
                format!("{}...(truncated)", &stdout[..max_len])
            } else {
                stdout.to_string()
            };

            json!({
                "success": output.status.success(),
                "exit_code": output.status.code(),
                "stdout": stdout_str.trim(),
                "stderr": stderr.trim()
            })
        }
        Err(e) => json!({"error": format!("Failed to run command: {}", e)}),
    }
}
