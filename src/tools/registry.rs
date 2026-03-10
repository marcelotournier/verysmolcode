use crate::api::types::{FunctionDecl, ToolDeclaration};
use crate::telegram::bot::send_telegram_tool;
use crate::tools::{file_ops, git, grep, web};
use serde_json::{json, Value};

/// Execute a tool by name with the given arguments
pub fn execute_tool(name: &str, args: &Value) -> Value {
    match name {
        "read_file" => file_ops::read_file(args),
        "write_file" => file_ops::write_file(args),
        "edit_file" => file_ops::edit_file(args),
        "list_directory" => file_ops::list_dir(args),
        "grep_search" => grep::grep_search(args),
        "find_files" => grep::find_files(args),
        "git_status" => git::git_status(args),
        "git_diff" => git::git_diff(args),
        "git_log" => git::git_log(args),
        "git_commit" => git::git_commit(args),
        "git_add" => git::git_add(args),
        "git_branch" => git::git_branch(args),
        "git_checkout" => git::git_checkout(args),
        "git_push" => git::git_push(args),
        "git_pull" => git::git_pull(args),
        "run_command" => git::run_shell(args),
        "web_fetch" => web::web_fetch(args),
        "read_image" => file_ops::read_image(args),
        "send_telegram" => send_telegram_tool(args),
        _ => json!({"error": format!("Unknown tool: {}", name)}),
    }
}

/// Get all tool declarations for the Gemini API
pub fn get_tool_declarations() -> Vec<ToolDeclaration> {
    vec![ToolDeclaration {
        google_search: None,
        function_declarations: vec![
            FunctionDecl {
                name: "read_file".to_string(),
                description: "Read the contents of a file at the given path".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to the file to read"
                        }
                    },
                    "required": ["path"]
                }),
            },
            FunctionDecl {
                name: "write_file".to_string(),
                description: "Write content to a file (creates if needed)".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to write to"
                        },
                        "content": {
                            "type": "string",
                            "description": "Content to write"
                        }
                    },
                    "required": ["path", "content"]
                }),
            },
            FunctionDecl {
                name: "edit_file".to_string(),
                description:
                    "Replace old_string with new_string in a file. Set replace_all:true for global replace"
                        .to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to the file"
                        },
                        "old_string": {
                            "type": "string",
                            "description": "Exact string to find"
                        },
                        "new_string": {
                            "type": "string",
                            "description": "Replacement string"
                        },
                        "replace_all": {
                            "type": "boolean",
                            "description": "Replace all occurrences (default false)"
                        }
                    },
                    "required": ["path", "old_string", "new_string"]
                }),
            },
            FunctionDecl {
                name: "list_directory".to_string(),
                description: "List files and directories at the given path".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Directory path"
                        }
                    }
                }),
            },
            FunctionDecl {
                name: "grep_search".to_string(),
                description: "Search for a text pattern in files recursively".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "Text pattern (case-insensitive)"
                        },
                        "path": {
                            "type": "string",
                            "description": "Directory to search in"
                        },
                        "include": {
                            "type": "string",
                            "description": "File extension filter, e.g. '*.rs'"
                        }
                    },
                    "required": ["pattern"]
                }),
            },
            FunctionDecl {
                name: "find_files".to_string(),
                description: "Find files matching a name pattern".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "Filename pattern, e.g. '*.rs'"
                        },
                        "path": {
                            "type": "string",
                            "description": "Directory to search in"
                        }
                    },
                    "required": ["pattern"]
                }),
            },
            FunctionDecl {
                name: "git_status".to_string(),
                description: "Show git status (modified, staged, untracked files)".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            FunctionDecl {
                name: "git_diff".to_string(),
                description: "Show git diff of changes".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "staged": {
                            "type": "boolean",
                            "description": "Show staged changes"
                        }
                    }
                }),
            },
            FunctionDecl {
                name: "git_log".to_string(),
                description: "Show recent git commit history".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "count": {
                            "type": "integer",
                            "description": "Number of commits (default: 10)"
                        }
                    }
                }),
            },
            FunctionDecl {
                name: "git_commit".to_string(),
                description: "Create a git commit with the given message".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "message": {
                            "type": "string",
                            "description": "Commit message"
                        },
                        "add_all": {
                            "type": "boolean",
                            "description": "Stage all changes before commit"
                        }
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
                        "files": {
                            "type": "string",
                            "description": "Files to stage (space-separated)"
                        }
                    },
                    "required": ["files"]
                }),
            },
            FunctionDecl {
                name: "git_branch".to_string(),
                description: "List branches or create a new branch".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "New branch name (omit to list)"
                        }
                    }
                }),
            },
            FunctionDecl {
                name: "git_checkout".to_string(),
                description: "Switch to a different branch".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "branch": {
                            "type": "string",
                            "description": "Branch to switch to"
                        }
                    },
                    "required": ["branch"]
                }),
            },
            FunctionDecl {
                name: "git_push".to_string(),
                description: "Push commits to remote repository".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "remote": {
                            "type": "string",
                            "description": "Remote (default: origin)"
                        },
                        "branch": {
                            "type": "string",
                            "description": "Branch to push"
                        }
                    }
                }),
            },
            FunctionDecl {
                name: "git_pull".to_string(),
                description: "Pull latest changes from remote".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "remote": {
                            "type": "string",
                            "description": "Remote (default: origin)"
                        }
                    }
                }),
            },
            FunctionDecl {
                name: "run_command".to_string(),
                description:
                    "Run a shell command. Times out after configured timeout (default 60s)."
                        .to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "Shell command to execute"
                        },
                        "timeout": {
                            "type": "integer",
                            "description": "Timeout in seconds (default: 60)"
                        }
                    },
                    "required": ["command"]
                }),
            },
            FunctionDecl {
                name: "read_image".to_string(),
                description: "Read an image file (PNG, JPEG, GIF, WebP, BMP) as base64".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to the image file"
                        }
                    },
                    "required": ["path"]
                }),
            },
            FunctionDecl {
                name: "web_fetch".to_string(),
                description: "Fetch a URL and return text content (HTML stripped to plain text)"
                    .to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "url": {
                            "type": "string",
                            "description": "URL to fetch (http:// or https://)"
                        }
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
                        "action": {
                            "type": "string",
                            "description": "Action: add, start, done, remove, list"
                        },
                        "text": {
                            "type": "string",
                            "description": "Task description (for 'add')"
                        },
                        "id": {
                            "type": "integer",
                            "description": "Task ID (for start/done/remove)"
                        }
                    },
                    "required": ["action"]
                }),
            },
            FunctionDecl {
                name: "send_telegram".to_string(),
                description: "Send a message to the user via Telegram. Use ONLY for: asking the user a question that blocks progress, reporting that a task is complete, or sharing a final answer. Do NOT use for routine status updates."
                    .to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "message": {
                            "type": "string",
                            "description": "Message to send to the user via Telegram"
                        }
                    },
                    "required": ["message"]
                }),
            },
        ],
    }]
}

pub struct ToolRegistry;

impl ToolRegistry {
    pub fn declarations() -> Vec<ToolDeclaration> {
        get_tool_declarations()
    }

    /// Read-only tools for planning mode - no file writes, no git mutations
    pub fn read_only_declarations() -> Vec<ToolDeclaration> {
        let all = get_tool_declarations();
        let read_only_names = [
            "read_file",
            "list_directory",
            "grep_search",
            "find_files",
            "git_status",
            "git_diff",
            "git_log",
            "web_fetch",
            "read_image",
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

    pub fn execute(name: &str, args: &Value) -> Value {
        execute_tool(name, args)
    }
}
