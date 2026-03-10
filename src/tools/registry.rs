use serde_json::{json, Value};
use crate::api::types::{ToolDeclaration, FunctionDecl};
use crate::tools::{file_ops, grep, git};

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
        _ => json!({"error": format!("Unknown tool: {}", name)}),
    }
}

/// Get all tool declarations for the Gemini API
pub fn get_tool_declarations() -> Vec<ToolDeclaration> {
    vec![ToolDeclaration {
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
                description: "Write content to a file, creating it if it doesn't exist".to_string(),
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
                description: "Edit a file by replacing old_string with new_string. The old_string must be unique in the file.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to the file"
                        },
                        "old_string": {
                            "type": "string",
                            "description": "Exact string to find and replace (must be unique)"
                        },
                        "new_string": {
                            "type": "string",
                            "description": "Replacement string"
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
                            "description": "Directory path (default: current directory)"
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
                            "description": "Text pattern to search for (case-insensitive)"
                        },
                        "path": {
                            "type": "string",
                            "description": "Directory to search in (default: current directory)"
                        },
                        "include": {
                            "type": "string",
                            "description": "File extension filter, e.g. '*.rs' or '*.py'"
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
                            "description": "Filename pattern to match, e.g. '*.rs' or 'Cargo.toml'"
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
                            "description": "Show staged changes (default: false)"
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
                            "description": "Number of commits to show (default: 10)"
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
                            "description": "Stage all changes before committing (default: false)"
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
                            "description": "Space-separated list of files to stage"
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
                            "description": "Name for new branch (omit to list branches)"
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
                            "description": "Branch name to switch to"
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
                            "description": "Remote name (default: origin)"
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
                            "description": "Remote name (default: origin)"
                        }
                    }
                }),
            },
            FunctionDecl {
                name: "run_command".to_string(),
                description: "Run a shell command and return its output. Use for tasks like running tests, installing packages, or checking system state.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "Shell command to execute"
                        }
                    },
                    "required": ["command"]
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

    pub fn execute(name: &str, args: &Value) -> Value {
        execute_tool(name, args)
    }
}
