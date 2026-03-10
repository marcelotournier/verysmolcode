# VerySmolCode

A lightweight TUI coding assistant powered by Gemini API free tier, designed for resource-constrained devices like Raspberry Pi 3.

## Features

- **Smart Model Routing**: Automatically selects between Gemini Pro, Flash, and Flash-Lite based on task complexity, with graceful fallback when rate limits are hit
- **Planning Mode**: `/plan` for read-only analysis using Pro models before making changes
- **Full Tool Suite**: File read/write/edit, grep search, find files, git operations, shell commands, web fetch, image reading
- **Web Search**: Gemini's native Google Search grounding for finding docs and examples
- **MCP Support**: Connect to MCP servers (context7, playwright, etc.) via `/mcp-add`
- **Token-Aware**: Conversation compaction, thinking budget control, and rate limit tracking
- **Safe by Default**: Blocks destructive operations, validates paths, and prevents dangerous commands
- **Lightweight**: ~5MB binary, minimal memory footprint, runs on Raspberry Pi 3

## Installation

### From Source (Rust)

```bash
# Install Rust if needed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/marcelotournier/verysmolcode.git
cd verysmolcode
cargo install --path .
```

### With pip (Python)

```bash
pip install verysmolcode
```

### From Source (Python)

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

# Run VerySmolCode
vsc
```

## Commands

| Command    | Description                                      |
|------------|--------------------------------------------------|
| `/help`    | Show available commands and keybindings           |
| `/plan`    | Toggle planning mode (read-only, Pro model)       |
| `/model`   | Show available models and rate limits             |
| `/config`  | Show current configuration                        |
| `/status`  | Show rate limits and token usage                  |
| `/compact` | Manually compact conversation to save tokens      |
| `/mcp`     | List configured MCP servers                       |
| `/mcp-add` | Add MCP server: `/mcp-add name command [args]`   |
| `/mcp-rm`  | Remove MCP server: `/mcp-rm name`                |
| `/version` | Show version information                          |
| `/clear`   | Clear conversation and screen                     |
| `/quit`    | Exit VerySmolCode                                 |

## Keybindings

| Key        | Action              |
|------------|---------------------|
| `Ctrl+C`   | Cancel/Quit         |
| `Ctrl+L`   | Clear screen        |
| `Up/Down`  | Input history       |
| `PgUp/PgDn`| Scroll output       |
| `Tab`      | Auto-complete       |
| `Ctrl+A/E` | Home/End of line    |
| `Ctrl+U/K` | Clear line before/after cursor |
| `Ctrl+W`   | Delete word backward |

## Model Tiers (Free Tier)

| Model              | RPM | RPD  | Best For         |
|--------------------|-----|------|------------------|
| Gemini 2.5 Pro     | 5   | 25   | Complex tasks    |
| Gemini 2.5 Flash   | 10  | 250  | General coding   |
| Gemini 2.0 Flash-Lite | 15 | 1000 | Simple tasks  |

VerySmolCode automatically manages rate limits across all models. When one model is exhausted, it falls back to the next available one.

## Configuration

Config file is stored at `~/.config/verysmolcode/config.json`:

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
    client.rs       - Gemini REST API client
    models.rs       - Model definitions, rate limiting, routing
    types.rs        - Request/response type definitions
  agent/
    loop_runner.rs  - Main agent loop with planning mode
  tools/
    file_ops.rs     - File read/write/edit/list
    grep.rs         - Search and find files
    git.rs          - Git operations and shell commands
    web.rs          - Web page fetching
    registry.rs     - Tool registration and dispatch (18 tools)
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

## License

MIT
