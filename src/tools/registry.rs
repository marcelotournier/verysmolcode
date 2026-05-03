//! Tool registry — pi-style names are canonical (`read`, `write`, `edit`,
//! `bash`, `grep`, `find`, `ls`). Legacy VSC names (`read_file`, `write_file`,
//! `edit_file`, `list_directory`, `grep_search`, `find_files`, `run_command`,
//! `read_image`) remain valid as aliases so existing sessions, prompts, and
//! TUI summarizers keep working.
//!
//! Custom VSC tools (`git_*`, `web_fetch`, `todo_update`, `send_telegram`,
//! `vsc_help`, `task`) sit on top of pi's seven coding tools.

use crate::api::types::{FunctionDecl, ToolDeclaration};
use crate::telegram::bot::send_telegram_tool;
use crate::tools::{bash, edit, find, git, grep, ls, read, self_help, subagent, web, write};
use serde_json::{json, Value};

/// Execute a tool by name with the given arguments.
pub fn execute_tool(name: &str, args: &Value) -> Value {
    match name {
        // Pi-style canonical names
        "read" => read::read(args),
        "write" => write::write(args),
        "edit" => edit::edit(args),
        "ls" => ls::ls(args),
        "grep" => grep::grep(args),
        "find" => find::find(args),
        "bash" => bash::bash(args),
        // Subagent + self-help
        "task" => subagent::task(args),
        "vsc_help" => self_help::vsc_help(args),

        // Legacy VSC names — route to the new implementations
        "read_file" => read::read(args),
        "write_file" => write::write(args),
        "edit_file" => edit::edit(args),
        "list_directory" => ls::ls(args),
        "grep_search" => grep::grep(args),
        "find_files" => find::find(args),
        "run_command" => bash::bash(args),
        "read_image" => read::read(args),

        // VSC extensions
        "git_status" => git::git_status(args),
        "git_diff" => git::git_diff(args),
        "git_log" => git::git_log(args),
        "git_commit" => git::git_commit(args),
        "git_add" => git::git_add(args),
        "git_branch" => git::git_branch(args),
        "git_checkout" => git::git_checkout(args),
        "git_push" => git::git_push(args),
        "git_pull" => git::git_pull(args),
        "web_fetch" => web::web_fetch(args),
        "send_telegram" => send_telegram_tool(args),
        _ => json!({"error": format!("Unknown tool: {}", name)}),
    }
}

/// Tool declarations advertised to the model. Names match pi exactly so the
/// agent picks them up by the conventional names from training data.
pub fn get_tool_declarations() -> Vec<ToolDeclaration> {
    vec![ToolDeclaration {
        google_search: None,
        function_declarations: vec![
            // ---- Core pi-style coding tools ----
            FunctionDecl {
                name: "read".to_string(),
                description: format!(
                    "Read the contents of a file. Supports text files and images. \
                     Output is truncated to {} lines or {}KB (whichever first). \
                     Use offset/limit to page through large files.",
                    crate::tools::truncate::DEFAULT_MAX_LINES,
                    crate::tools::truncate::DEFAULT_MAX_BYTES / 1024
                ),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "Path to read (relative or absolute)"},
                        "offset": {"type": "integer", "description": "1-indexed start line"},
                        "limit": {"type": "integer", "description": "Max number of lines"}
                    },
                    "required": ["path"]
                }),
            },
            FunctionDecl {
                name: "write".to_string(),
                description: "Write content to a file (creates parent dirs, overwrites if exists). Max 5MB."
                    .to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string"},
                        "content": {"type": "string"}
                    },
                    "required": ["path", "content"]
                }),
            },
            FunctionDecl {
                name: "edit".to_string(),
                description:
                    "Edit a file with one or more exact-text replacements. Each edits[].oldText must \
                     match a unique, non-overlapping region of the file. Fuzzy match handles smart \
                     quotes / unicode dashes / trailing whitespace automatically. Pass either \
                     edits:[{oldText,newText}] or the legacy {old_string,new_string,replace_all}."
                        .to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string"},
                        "edits": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "oldText": {"type": "string"},
                                    "newText": {"type": "string"}
                                },
                                "required": ["oldText", "newText"]
                            }
                        },
                        "old_string": {"type": "string", "description": "Legacy single-edit shortcut"},
                        "new_string": {"type": "string"},
                        "replace_all": {"type": "boolean"}
                    },
                    "required": ["path"]
                }),
            },
            FunctionDecl {
                name: "ls".to_string(),
                description: "List directory contents (alphabetical, dirs marked with '/'). Default limit 500."
                    .to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string"},
                        "limit": {"type": "integer"}
                    }
                }),
            },
            FunctionDecl {
                name: "grep".to_string(),
                description: format!(
                    "Search file contents for a pattern. Default literal substring match (case-sensitive). \
                     Set ignore_case:true for CI search; pass glob to filter files; pass context for \
                     surrounding lines. Default limit {}.", DEFAULT_GREP_LIMIT
                ),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "pattern": {"type": "string"},
                        "path": {"type": "string"},
                        "glob": {"type": "string", "description": "Glob filter, e.g. '*.rs'"},
                        "ignore_case": {"type": "boolean"},
                        "context": {"type": "integer", "description": "Lines of context before/after each match"},
                        "limit": {"type": "integer"}
                    },
                    "required": ["pattern"]
                }),
            },
            FunctionDecl {
                name: "find".to_string(),
                description:
                    "Find files matching a glob pattern. Supports *, **, and ?. Default limit 1000. \
                     Skips .git, node_modules, target."
                        .to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "pattern": {"type": "string", "description": "Glob, e.g. '*.rs', 'src/**/*.ts'"},
                        "path": {"type": "string"},
                        "limit": {"type": "integer"}
                    },
                    "required": ["pattern"]
                }),
            },
            FunctionDecl {
                name: "bash".to_string(),
                description: format!(
                    "Execute a shell command. Output (stdout+stderr merged) is tail-truncated to \
                     last {} lines or {}KB. Default timeout {}s; pass timeout to override (5–600).",
                    crate::tools::truncate::DEFAULT_MAX_LINES,
                    crate::tools::truncate::DEFAULT_MAX_BYTES / 1024,
                    bash::command_timeout_secs()
                ),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "command": {"type": "string"},
                        "timeout": {"type": "integer", "description": "Timeout in seconds"}
                    },
                    "required": ["command"]
                }),
            },

            // ---- Subagent ----
            FunctionDecl {
                name: "task".to_string(),
                description:
                    "Spawn a focused subagent for a self-contained subtask. Subagent has its own \
                     context (saves parent tokens) and returns a single condensed text answer. \
                     read_only:true (default) restricts subagent to read/grep/find/ls/web_fetch."
                        .to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "task": {"type": "string", "description": "Self-contained instruction"},
                        "read_only": {"type": "boolean", "description": "Default true"},
                        "model_pref": {"type": "string", "description": "fast|smart (default: fast)"}
                    },
                    "required": ["task"]
                }),
            },

            // ---- VSC self-help ----
            FunctionDecl {
                name: "vsc_help".to_string(),
                description:
                    "Return VSC's own help: runs `vsc -h` and points at README/GitHub for deeper \
                     questions. Use this BEFORE guessing about VSC features (loop, telegram, MCP, \
                     models, slash commands, configuration)."
                        .to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "topic": {"type": "string", "description": "Optional topic hint"}
                    }
                }),
            },

            // ---- VSC custom tools ----
            FunctionDecl {
                name: "git_status".to_string(),
                description: "Show git status (modified, staged, untracked files)".to_string(),
                parameters: json!({"type": "object", "properties": {}}),
            },
            FunctionDecl {
                name: "git_diff".to_string(),
                description: "Show git diff of changes".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "staged": {"type": "boolean", "description": "Show staged changes"}
                    }
                }),
            },
            FunctionDecl {
                name: "git_log".to_string(),
                description: "Show recent git commit history".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "count": {"type": "integer", "description": "Number of commits (default 10)"}
                    }
                }),
            },
            FunctionDecl {
                name: "git_commit".to_string(),
                description: "Create a git commit with the given message".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "message": {"type": "string"},
                        "add_all": {"type": "boolean"}
                    },
                    "required": ["message"]
                }),
            },
            FunctionDecl {
                name: "git_add".to_string(),
                description: "Stage files for commit".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "files": {"type": "string"}
                    },
                    "required": ["files"]
                }),
            },
            FunctionDecl {
                name: "git_branch".to_string(),
                description: "List branches or create a new branch".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {"name": {"type": "string"}}
                }),
            },
            FunctionDecl {
                name: "git_checkout".to_string(),
                description: "Switch to a different branch".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {"branch": {"type": "string"}},
                    "required": ["branch"]
                }),
            },
            FunctionDecl {
                name: "git_push".to_string(),
                description: "Push commits to remote repository".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "remote": {"type": "string"},
                        "branch": {"type": "string"}
                    }
                }),
            },
            FunctionDecl {
                name: "git_pull".to_string(),
                description: "Pull latest changes from remote".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {"remote": {"type": "string"}}
                }),
            },
            FunctionDecl {
                name: "web_fetch".to_string(),
                description: "Fetch a URL and return text content (HTML stripped to plain text)"
                    .to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "url": {"type": "string"}
                    },
                    "required": ["url"]
                }),
            },
            FunctionDecl {
                name: "todo_update".to_string(),
                description: "Track tasks: add/start/done/remove/list. Persists across tool calls."
                    .to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "action": {"type": "string"},
                        "text": {"type": "string"},
                        "id": {"type": "integer"}
                    },
                    "required": ["action"]
                }),
            },
            FunctionDecl {
                name: "send_telegram".to_string(),
                description: "Send a message to the user via Telegram. Use ONLY for: asking the user a question that blocks progress, reporting that a task is complete, or sharing a final answer."
                    .to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "message": {"type": "string"}
                    },
                    "required": ["message"]
                }),
            },
        ],
    }]
}

const DEFAULT_GREP_LIMIT: usize = 100;

pub struct ToolRegistry;

impl ToolRegistry {
    pub fn declarations() -> Vec<ToolDeclaration> {
        get_tool_declarations()
    }

    /// Read-only tools for planning mode and subagents.
    pub fn read_only_declarations() -> Vec<ToolDeclaration> {
        let all = get_tool_declarations();
        let read_only_names = [
            "read",
            "ls",
            "grep",
            "find",
            "vsc_help",
            "task",
            // Legacy aliases
            "read_file",
            "list_directory",
            "grep_search",
            "find_files",
            "read_image",
            // Read-only VSC extras
            "git_status",
            "git_diff",
            "git_log",
            "web_fetch",
            "todo_update",
        ];
        vec![ToolDeclaration {
            google_search: None,
            function_declarations: all[0]
                .function_declarations
                .iter()
                .filter(|f| read_only_names.contains(&f.name.as_str()))
                .cloned()
                .collect(),
        }]
    }

    /// Subset declarations filtered to a custom allow-list (used by subagents).
    pub fn declarations_for(allowed: &[&str]) -> Vec<ToolDeclaration> {
        let all = get_tool_declarations();
        vec![ToolDeclaration {
            google_search: None,
            function_declarations: all[0]
                .function_declarations
                .iter()
                .filter(|f| allowed.contains(&f.name.as_str()))
                .cloned()
                .collect(),
        }]
    }

    pub fn execute(name: &str, args: &Value) -> Value {
        execute_tool(name, args)
    }
}
