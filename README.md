# VerySmolCode

A lightweight TUI coding assistant powered by Gemini API free tier, designed for resource-constrained devices like Raspberry Pi 3.

## Features

- **CLI Prompt Mode**: `vsc -p "prompt"` for single-shot usage (like `claude -p`), supports piped input
- **Command Autocomplete**: Type `/` to see all available commands with descriptions, navigate with arrow keys
- **Smart Model Routing**: 6 models across Gemini 3.x and 2.5 — automatically selects the best available model based on task complexity, with graceful fallback when rate-limited or overloaded
- **Planning Mode**: `/plan` for thorough analysis — reads code, creates architecture plans, and builds a todo list to guide implementation
- **Task Tracking**: Built-in todo list (like Claude Code) — the agent creates and tracks tasks during complex work, visible with `/todo`
- **Full Tool Suite**: File read/write/edit, grep search, find files, git operations, shell commands, web fetch, image reading (19 tools)
- **MCP Support**: Connect to MCP servers (context7, playwright, etc.) via `/mcp-add` — tools are live in the agent loop
- **Code Reviewer**: After tool use, reviews actual `git diff` with a structured checklist (correctness, bugs, completeness, style)
- **Chain-of-Thought**: All 6 models use thinking tokens for better reasoning, with tier-scaled budgets (Pro 2048, Flash 1024, Lite 512)
- **Token-Aware**: `/tokens` dashboard, `/fast`/`/smart` model selection, rate limit warnings, conversation compaction, and thinking budget control
- **Safe by Default**: Blocks destructive operations, validates paths, and prevents dangerous commands
- **Lightweight**: ~5MB binary, minimal memory footprint, runs on Raspberry Pi 3

## Installation

### With pip (Python)

Pre-built wheels available for Linux (x86_64, aarch64, armv7), macOS (Intel + Apple Silicon), and Windows (x86_64):

```bash
pip install verysmolcode
```

### From Source (Rust)

```bash
# Install Rust if needed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/marcelotournier/verysmolcode.git
cd verysmolcode
cargo install --path .
```

### From Source (Python wheel)

```bash
pip install maturin
git clone https://github.com/marcelotournier/verysmolcode.git
cd verysmolcode
maturin develop --features python
```

## Usage

```bash
# Set your Gemini API key (get one free at https://aistudio.google.com/apikey)
export GEMINI_API_KEY=your_key_here

# Run interactive TUI
vsc

# Run a single prompt (like claude -p)
vsc -p "explain this codebase"

# Pipe input as prompt
cat error.log | vsc -p "what's wrong here?"

# Show version
vsc -v
```

## Commands

| Command      | Description                                      |
|--------------|--------------------------------------------------|
| `/help`      | Show available commands and keybindings           |
| `/fast`      | Use Flash models for next message (saves budget)  |
| `/smart`     | Use Pro models for next message (best quality)    |
| `/plan`      | Toggle planning mode (read-only, Pro model)       |
| `/undo`      | Undo the last batch of file changes                |
| `/save`      | Save conversation to file: `/save [filename]`      |
| `/tokens`    | Show detailed token usage and rate limits         |
| `/status`    | Show rate limits and token usage                  |
| `/model`     | Show available models and rate limits             |
| `/config`    | Show current configuration                        |
| `/config set`| Edit config: `/config set temperature 0.5`       |
| `/compact`   | Manually compact conversation to save tokens      |
| `/mcp`       | List configured MCP servers                       |
| `/mcp-add`   | Add MCP server: `/mcp-add name command [args]`   |
| `/mcp-rm`    | Remove MCP server: `/mcp-rm name`                |
| `/todo`      | Show current task list (alias: `/t`)             |
| `/retry`     | Retry the last message (alias: `/r`)             |
| `/version`   | Show version information                          |
| `/clear`     | Clear conversation and screen                     |
| `/quit`      | Exit VerySmolCode                                 |

## Keybindings

| Key        | Action              |
|------------|---------------------|
| `Ctrl+C`   | Cancel/Quit         |
| `Ctrl+L`   | Clear screen        |
| `Up/Down`  | Input history / Navigate command popup |
| `PgUp/PgDn`| Scroll output       |
| `Tab`      | Select from command popup |
| `Esc`      | Dismiss command popup |
| `Ctrl+A/E` | Home/End of line    |
| `Ctrl+U/K` | Clear line before/after cursor |
| `Ctrl+W`   | Delete word backward |

## Model Tiers (Free Tier)

| Model                    | RPM | RPD  | Best For         |
|--------------------------|-----|------|------------------|
| Gemini 3.1 Pro           | 5   | 25   | Complex tasks    |
| Gemini 3 Flash           | 10  | 250  | General coding   |
| Gemini 3.1 Flash-Lite    | 15  | 1000 | Simple tasks     |
| Gemini 2.5 Pro           | 5   | 25   | Fallback complex |
| Gemini 2.5 Flash         | 10  | 250  | Fallback general |
| Gemini 2.5 Flash-Lite    | 15  | 1000 | Fallback simple  |

VerySmolCode automatically manages rate limits across all 6 models independently. When one model is exhausted, it falls back to the next available one. Gemini 3 models are preferred; 2.5 models serve as fallbacks, effectively doubling your daily quota per tier.

## Configuration

Config file is stored at `~/.config/verysmolcode/config.json`. You can edit it directly or use `/config set` in the TUI:

```bash
/config set temperature 0.5     # Lower = more focused, higher = more creative
/config set max_tokens 2048     # Limit response length to save tokens
/config set compact_threshold 16000  # Compact conversation earlier
/config set safety off          # Disable safety checks (not recommended)
```

Default values:
```json
{
  "max_tokens_per_response": 4096,
  "max_conversation_tokens": 32000,
  "temperature": 0.7,
  "auto_compact_threshold": 24000,
  "safety_enabled": true
}
```

## Architecture

```
src/
  main.rs           - Entry point
  config.rs         - Configuration management
  api/
    client.rs       - Gemini REST API client with fallback
    models.rs       - 6 model definitions, rate limiting, routing
    types.rs        - Request/response type definitions
  agent/
    loop_runner.rs  - Main agent loop with planning mode and critic
  tools/
    file_ops.rs     - File read/write/edit/list + image reading
    grep.rs         - Search and find files
    git.rs          - Git operations and shell commands
    web.rs          - Web page fetching
    todo.rs         - Task tracking (agent todo list)
    registry.rs     - Tool registration and dispatch (19 tools)
  mcp/
    client.rs       - MCP client (stdio JSON-RPC 2.0)
    config.rs       - MCP server configuration
    types.rs        - MCP protocol types
  tui/
    app.rs          - Application state and event handling
    ui.rs           - Terminal UI rendering
    input.rs        - Keyboard input handling
    commands.rs     - Slash command processing
```

## Testing

```bash
# Unit tests (483 tests)
cargo test

# Integration test (requires tmux + GEMINI_API_KEY)
./tests/integration_test.sh
```

## License

MIT
