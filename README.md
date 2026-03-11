# VerySmolCode

A lightweight TUI coding assistant powered by Gemini API free tier, designed for resource-constrained devices like Raspberry Pi 3.

## Features

- **CLI Prompt Mode**: `vsc -p "prompt"` for single-shot usage (like `claude -p`), supports piped input
- **Command Autocomplete**: Type `/` to see all available commands with descriptions, navigate with arrow keys
- **Smart Model Routing**: 6 models across Gemini 3.x and 2.5 — priority order: `3.1 Pro → 3 Flash → 3.1 Flash-Lite → 2.5 Pro → 2.5 Flash → 2.5 Flash-Lite`. Exponential backoff when all models are rate-limited, then retries from top
- **Planning Mode**: `/plan` for thorough analysis — reads code, creates architecture plans, and builds a todo list to guide implementation
- **Task Tracking**: Built-in todo list (like Claude Code) — the agent creates and tracks tasks during complex work, visible with `/todo`
- **Full Tool Suite**: File read/write/edit, grep search, find files, git operations, shell commands, web fetch, image reading (19 tools)
- **MCP Support**: Connect to MCP servers (context7, playwright, etc.) via `/mcp-add` — tools are live in the agent loop
- **Code Reviewer**: After file changes, a silent critic reviews the `git diff`. For non-Pro models, `NEEDS_WORK` triggers an automatic silent fix turn (user never sees the review). For Pro, review is shown.
- **Agent Slash Commands**: The agent can emit `CMD:/compact` or `CMD:/loop 5m prompt` to control TUI features
- **Chain-of-Thought**: All 6 models use thinking tokens for better reasoning, with tier-scaled budgets (Pro 2048, Flash 1024, Lite 512)
- **Token-Aware**: `/tokens` dashboard, `/fast`/`/smart` model selection, rate limit warnings, conversation compaction at 160K tokens
- **Tool Timing**: Each tool call shows execution time — helps identify bottlenecks on slow hardware
- **Safe by Default**: Blocks destructive operations, validates paths, and prevents dangerous commands
- **Loop Mode**: `/loop <prompt>` runs a prompt repeatedly — immediately after each completion (Ralph-style) or on a timed interval (e.g. `5m`, `30s`). Great for iterative refinement and monitoring tasks
- **Telegram Integration**: Connect a Telegram bot to receive agent messages on your phone, send prompts back, and share photos/documents — attachments are included in the agent's context
- **Text Selection**: Terminal mouse events are not captured, so you can freely select and copy text from the output
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

| Command         | Description                                        |
|-----------------|----------------------------------------------------|
| `/help`         | Show available commands and keybindings            |
| `/fast`         | Use Flash models for next message (saves budget)   |
| `/smart`        | Use Pro models for next message (best quality)     |
| `/plan`         | Toggle planning mode (read-only, Pro model)        |
| `/undo`         | Undo the last batch of file changes                |
| `/save`         | Save conversation to file: `/save [filename]`      |
| `/tokens`       | Show detailed token usage and rate limits          |
| `/status`       | Show rate limits and token usage                   |
| `/model`        | Show available models and rate limits              |
| `/config`       | Show current configuration                         |
| `/config set`   | Edit config: `/config set temperature 0.5`         |
| `/compact`      | Manually compact conversation to save tokens       |
| `/mcp`          | List configured MCP servers                        |
| `/mcp-add`      | Add MCP server: `/mcp-add name command [args]`     |
| `/mcp-rm`       | Remove MCP server: `/mcp-rm name`                  |
| `/todo`         | Show current task list (alias: `/t`)               |
| `/retry`        | Retry the last message (alias: `/r`)               |
| `/loop`         | Loop a prompt: `/loop [5m] [--max N] <prompt>`     |
| `/loop-cancel`  | Cancel the active loop                             |
| `/search`       | Toggle Google Search grounding                     |
| `/diff`         | Show git diff                                      |
| `/copy`         | Copy last response to clipboard                    |
| `/new`          | Start a new conversation (saves current session)   |
| `/resume`       | Resume a previous session: `/resume [id]`          |
| `/agents`       | Show loaded AGENTS.md / CLAUDE.md instruction files|
| `/telegram`     | Telegram bot setup: `/telegram setup <token> <id>` |
| `/telegram-test`| Send a test Telegram message                       |
| `/telegram-off` | Disable Telegram integration                       |
| `/version`      | Show version information                           |
| `/clear`        | Clear conversation and screen                      |
| `/quit`         | Exit VerySmolCode                                  |

## Keybindings

| Key          | Action                                      |
|--------------|---------------------------------------------|
| `Ctrl+C`     | Cancel current task / Quit                  |
| `Ctrl+D`     | Quit (only when input is empty)             |
| `Ctrl+L`     | Clear screen                                |
| `Ctrl+R`     | Reverse search input history                |
| `Ctrl+P`     | Open command palette                        |
| `Ctrl+T`     | Toggle todo list popup                      |
| `Up/Down`    | Input history / Navigate command popup      |
| `PgUp/PgDn`  | Scroll output (2 lines per step)            |
| `Tab`        | Select from command/file popup              |
| `Esc`        | Cancel task / Dismiss popup                 |
| `Ctrl+A/E`   | Home/End of line                            |
| `Ctrl+U/K`   | Clear line before/after cursor              |
| `Ctrl+W`     | Delete word backward                        |
| `@`          | Trigger file autocomplete                   |
| `\` + Enter  | Multi-line input mode                       |
| `!command`   | Run shell command directly (bash mode)      |

## Header Layout

The TUI header has three columns:

```
🫐 Thinking        🧠 VerySmolCode         Gemini 3.1 Pro
```

- **Left**: Agent status (`✨ Ready` or `🫐 Thinking`)
- **Center**: App title + mode badges (`[PLAN]`, `[WEB]`)
- **Right**: Currently active model name

## Model Tiers (Free Tier)

| Model                    | RPM | RPD  | Best For         |
|--------------------------|-----|------|------------------|
| Gemini 3.1 Pro           | 5   | 25   | Complex tasks    |
| Gemini 3 Flash           | 10  | 250  | General coding   |
| Gemini 3.1 Flash-Lite    | 15  | 1000 | Simple tasks     |
| Gemini 2.5 Pro           | 5   | 25   | Fallback complex |
| Gemini 2.5 Flash         | 10  | 250  | Fallback general |
| Gemini 2.5 Flash-Lite    | 15  | 1000 | Fallback simple  |

VerySmolCode tries models in order: `3.1 Pro → 3 Flash → 3.1 Flash-Lite → 2.5 Pro → 2.5 Flash → 2.5 Flash-Lite`. When rate-limited, it falls back to the next model silently — the current model is always visible in the header top-right. If all models are exhausted, it applies exponential backoff (2s → 4s → … → 64s) and retries from the top.

## Configuration

Config file is stored at `~/.config/verysmolcode/config.json`. You can edit it directly or use `/config set` in the TUI:

```bash
/config set temperature 0.5          # Lower = more focused, higher = more creative
/config set max_tokens 2048          # Limit response length to save tokens
/config set compact_threshold 80000  # Compact conversation earlier (default: 160000)
/config set command_timeout 120      # Shell command timeout in seconds (default: 60)
/config set safety off               # Disable safety checks (not recommended)
```

Default values:
```json
{
  "max_tokens_per_response": 4096,
  "max_conversation_tokens": 32000,
  "temperature": 0.7,
  "auto_compact_threshold": 160000,
  "safety_enabled": true,
  "command_timeout": 60
}
```

## Loop Mode

Loop mode runs a prompt repeatedly — useful for iterative refinement (Ralph-style) or timed monitoring tasks:

```bash
# Run immediately after each completion (Ralph-style refinement)
/loop check for build errors and fix them

# Run every 5 minutes (timed polling)
/loop 5m run the test suite and report results

# Max 3 iterations then auto-stop
/loop --max 3 optimize the code further

# Combined: every 10 minutes, max 5 times
/loop 10m --max 5 check if CI is green

# Cancel the active loop
/loop off

# Show loop status
/loop
```

The loop status is also broadcast to Telegram if configured.

## Telegram Integration

Connect a Telegram bot to receive agent messages on your phone:

```bash
# 1. Chat with @BotFather on Telegram to get a bot token
# 2. Send a message to your bot, then get your chat_id:
#    https://api.telegram.org/bot<TOKEN>/getUpdates
# 3. Setup in vsc:
/telegram setup <bot_token> <chat_id>

# Send a test message
/telegram-test

# Disable
/telegram-off
```

Once configured:
- Agent responses, tool calls, and warnings are forwarded to Telegram
- You can send text messages from Telegram and they'll reach the agent
- **Photos and documents** sent to the bot are downloaded and included in the agent's context
- The agent can send files back using the `send_telegram` tool

## Project Instructions (AGENTS.md / CLAUDE.md)

VerySmolCode loads instruction files automatically at startup:

- `~/.config/verysmolcode/AGENTS.md` — user-level instructions (applies to all projects)
- `AGENTS.md` or `CLAUDE.md` in the git root — project-specific instructions

Use `/agents` to see which files are loaded.

## Architecture

```
src/
  main.rs           - Entry point
  config.rs         - Configuration management
  utils.rs          - Shared utilities (safe UTF-8 truncation)
  api/
    client.rs       - Gemini REST API client with fallback
    models.rs       - 6 model definitions, rate limiting, routing
    types.rs        - Request/response type definitions
  agent/
    loop_runner.rs  - Main agent loop with planning mode and silent critic
  tools/
    file_ops.rs     - File read/write/edit/list + image reading
    grep.rs         - Search and find files
    git.rs          - Git operations and shell commands
    web.rs          - Web page fetching
    todo.rs         - Task tracking (agent todo list)
    registry.rs     - Tool registration and dispatch (19 tools)
    undo.rs         - Undo history for file mutations
  mcp/
    client.rs       - MCP client (stdio JSON-RPC 2.0)
    config.rs       - MCP server configuration
    types.rs        - MCP protocol types
  telegram/
    bot.rs          - Telegram bot client (send/receive text + attachments)
    config.rs       - Telegram configuration
  tui/
    app.rs          - Application state and event handling
    ui.rs           - Terminal UI rendering (header, messages, status bar)
    input.rs        - Keyboard input handling
    commands.rs     - Slash command processing
    session.rs      - Session persistence and resume
```

## Testing

```bash
# Unit tests (495+ tests)
cargo test

# Integration test (requires tmux + GEMINI_API_KEY)
./tests/integration_test.sh
```

## License

MIT
