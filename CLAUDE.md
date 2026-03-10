# VerySmolCode
- Repository: https://github.com/marcelotournier/verysmolcode
- Create VerySmolCode, a rust-based TUI that mimics Claude Code and works with Gemini API free tier. GEMINI_API_KEY is in the env variable
- TUI must be lightweight enough to run in a memory constrained device such as a raspberrypi 3 (1GB RAM)
- Make sure this respects the free tier API limits for models. Smarter models get fewer requests: Pro gives 5/min and 25/day, Flash 10/min and 250/day, Flash-Lite 15/min and 1,000/day. We use 6 models (3 from Gemini 3.x, 3 from Gemini 2.5) with independent rate limits per model
- Use pro for harder tasks and flash for simpler stuff OR fallback to flash if pro is capped
- Whenever you use flash, use chain-of-thought (<think> xml tokens) to maximize its performance, flash is bad for tasks if it doesn't use CoT.
- Gemini API documentation at https://ai.google.dev/gemini-api/docs/quickstart#rest
- Models must have basic tools to manipulate files, grepsearch, edit, read images, do git operations (commit, push, pull, diff, branch, checkout, etc)
- TUI must have slashcommands to help on configuration
- TUI must be configurable in terms of how we can tune up or down token consumption - think about the best way to do it
- Coder agent must mimic what Opencode loop does, adapted by the limitations of tokens for the Google Free tier. CRITICAL: THINK HARD and design a good algorithm on how to optimize tokens and requests and all models available (all per-minute+per-day limits are per model so you can change them when limits are reached)
- Coder agent must develop AND do a critic if the work is really done
- Coder agent must not do egregious things in software like deleting stuff from the OS or things like the early days of Claude Code horror stories. Check references on reddit and do not do those...
- Find creative ways to optimize model limitations. Add MCP access for tools like context7 and other MCPs
- Add MCP access to playwright from the beginning
- Add functionality to install MCP servers from slash commands.
- Design principles:
  - VERY VERY IMPORTANT: Build this with maturin/pyo3 integration so we can pip install this in a raspberry pi later! I want this agent to be very easy to install.
  - No need to be creative. Look at the Gemini CLI, Opencode, Claude Code and Codex source repositories in github and base yourself upon them.
  - CRITICAL REQUIREMENT: All decisions must take into account low end hardware. It should provide a snappy user experience on a Raspberrypi 3
  - CRITICAL: User experience on token consumption should be good as well, the user should not be surprised with "out of tokens" errors
  - Add a friendly UX design in the experience, blueish colors and visually comfortable colors that work well on TMUX limitations. Make the UX good enough to fix a 80 x 30 col terminal
- How to develop:
  - Start by checking if repo is good (gitignore, etc)
  - Setup rust development env
  - Start with the model wrappers (Nanocode backend), test them, make sure they work
  - After wrappers are minimally functional, develop the frontend TUI
  - Commit and push to remote repo frequently, so you can rollback if needed
  - Keep going without any stops until both backend and frontend are fully functional and can work the FINAL TEST successfully
- HOW TO TEST:
  - Unit tests: Reach coverage of 100%
  - Integration tests: Run the TUI on TMUX and send a command to build a todo list app in python bottle.py, check outputs for some tasks to see if it worked correctly in the task.
- HOW TO WORK:
  - You are autonomous to keep going. use Ralph loops. make releases on github for working versions using gh cli tool.
  - Add user documentation in README.md so humans can easily work
  - Add TODO items in this document and modify this document as VerySmolCode evolves.

# TODO / Progress Tracker

## Completed (v0.7.7)
- [x] Repo setup (gitignore, license, maturin/pyo3)
- [x] Gemini API client with 6-model routing (Gemini 3.x + 2.5)
- [x] Rate limiting with per-model RPM/RPD tracking
- [x] Tool system: 18 tools (file read/write/edit, grep, find, git, shell, web fetch, image reading)
- [x] TUI: blue theme, input history, slash commands, scrolling
- [x] Agent loop with automatic model fallback (503/429 aware)
- [x] Deep fallback chain: tries ALL 6 models before giving up
- [x] Critic verification (cheapest available model reviews completed work, only when files modified)
- [x] Planning mode (/plan) with read-only tools
- [x] Chain-of-thought for Flash and Gemini 3.1 Pro models (thinkingConfig)
- [x] Safety: blocks destructive ops, validates paths
- [x] MCP client support (stdio JSON-RPC protocol) - wired into agent loop
- [x] MCP server management (/mcp, /mcp-add, /mcp-rm)
- [x] Image reading tool (base64 encode + send as InlineData to Gemini)
- [x] Pre-commit hooks (fmt, clippy, tests)
- [x] 93 unit tests across 5 test files
- [x] Integration test (tmux + bottle.py todo app - continue-on-error, API-dependent)
- [x] README.md documentation
- [x] CI/CD pipeline: test -> build-wheels -> test-wheel -> publish-pypi
- [x] Rayon parallelism for grep_search and find_files (leverages multi-core on RPi3)
- [x] GitHub releases (v0.2.0 through v0.7.7)
- [x] PyPI publishing with binary wheels for x86_64, aarch64, armv7
- [x] Token optimization: truncation, thinking stripping, conditional critic, tier-scaled budgets
- [x] /tokens and /status commands (show usage locally, no API call wasted)
- [x] Enhanced status bar: In/Out/Ctx token breakdown
- [x] Wait-and-retry: waits up to 15s for RPM limit before falling back to weaker model
- [x] ARM wheel cross-compilation via manylinux_2_28 + Python 3.13 pin
- [x] RPi3 pip install verified: downloads binary wheel (1.3MB), no compilation needed

## Completed (v0.7.8)
- [x] Configuration editing via /config set (temperature, max_tokens, compact_threshold, safety)
- [x] User-selectable model tier: /fast (/f) and /smart (/s) per request
- [x] Rate limit warnings when Pro budget <= 5 or models exhausted
- [x] 101 unit tests across 5 test files
- [x] GitHub Releases with release notes (v0.7.7, v0.7.8)

## Completed (v0.7.9)
- [x] 181 unit tests across 8 test files (+ inline tests)
- [x] Test coverage: agent loop helpers, MCP types, tool registry, TUI summarizer
- [x] Improved safety: catch curl/wget URL | sh/bash/zsh pipe patterns
- [x] Exported agent helper functions for external test access

## Completed (v0.8.0)
- [x] /undo command (UndoHistory module) - reverts last turn's file changes
- [x] Tracks write_file and edit_file mutations with pre-mutation snapshots
- [x] Handles: existing file restore, new file deletion, per-file dedup, one-turn bound
- [x] 190 unit tests

## Completed (v0.8.0 continued)
- [x] /save command - exports conversation to timestamped .md file
- [x] 192 unit tests

## Completed (v0.8.1)
- [x] Command timeout (60s default) for shell and git commands — prevents hung processes on RPi3
- [x] MCP server startup error reporting — no longer silently swallowed
- [x] macOS and Windows wheel builds in CI
- [x] 213 unit tests

## Completed (v0.8.3)
- [x] Smarter conversation compaction (6 msgs + topic hints + token estimation)
- [x] UTF-8 safe truncation across all text cutting points
- [x] Consolidated safety path checks (/usr, /bin, /sbin, /lib)
- [x] Windows compatibility for run_shell (cmd /C)
- [x] Enhanced system prompt with working directory and tool guidance
- [x] Critic notifies user when skipped due to rate limits
- [x] 218 unit tests

## Completed (v0.8.5)
- [x] Fixed multi-Python wheel CI (--find-links instead of glob install)
- [x] Python 3.11/3.12/3.13 wheels all build and publish successfully

## Completed (v0.9.0)
- [x] Command autocomplete popup (type / to see commands, arrows to navigate, Tab/Enter to select)
- [x] CLI prompt mode: vsc -p "prompt" (like claude -p), supports piped stdin
- [x] CLI flags: --version/-v, --help/-h
- [x] Friendlier TUI: emoji header/status, ASCII art welcome, warmer placeholders
- [x] Friendlier system prompt with personality (emojis in agent responses)
- [x] System prompt encourages MCP tools usage (context7) for library docs lookup
- [x] 226 unit tests

## Completed (v0.9.1)
- [x] /retry (/r) command - resend last message on failure
- [x] /compact now actually compacts conversation (was no-op)
- [x] Transient error recovery (timeouts, 500/502/504 retry once with 2s delay)
- [x] Emoji indicators for all message types in TUI
- [x] Categorized /help display with emoji section headers
- [x] MCP-aware system prompt (lists servers/tools, encourages context7 for docs)
- [x] 228 unit tests

## Completed (v0.9.2)
- [x] TodoList system for agent task tracking (like Claude Code)
  - Agent uses todo_update tool to create/manage task lists during complex work
  - Task state injected into system prompt every turn (model never loses focus)
  - /todo command shows current task list to user
  - 12 unit tests for TodoList
- [x] Enhanced code reviewer (replaces shallow critic)
  - Uses actual git diff context (up to 3000 chars) for review
  - Structured review checklist: correctness, bugs, completeness, style
  - Outputs APPROVED or NEEDS_WORK with specific feedback
- [x] Enhanced plan mode (/plan)
  - Thorough planning prompt with architecture decisions, testing plan, risk analysis
  - Planning mode creates todo list from steps (todo_update available in read-only mode)
  - Todo list persists when switching back to build mode, guiding implementation
  - MCP tools and todo state injected into planning prompt
- [x] Deduplicated system prompt construction (MCP + todo injection shared between modes)
- [x] 239 unit tests

## Completed (v0.9.3)
- [x] Massive test coverage expansion: 239 → 483 tests
- [x] New inline test modules: ui.rs (wrap_text), grep.rs (is_likely_binary, collect_files), file_ops.rs (24 tests)
- [x] grep.rs: include filters, max_results, hidden dir/node_modules skipping, binary detection
- [x] file_ops.rs: safe path checks, file truncation, edit ambiguity, dir sorting, image MIME types
- [x] git.rs: all blocked command patterns (sudo rm, chmod 777, mkfs, rm -rf ~)
- [x] web.rs: localhost/127.0.0.1/0.0.0.0 blocking, HTML stripping edge cases
- [x] test_agent.rs: is_dangerous_tool_call for /usr, /bin, /sbin, /lib, /proc, /sys
- [x] test_api.rs: Pro/FlashLite thinking budgets, fallback chain end, record_request
- [x] test_commands.rs: config set (max_tokens, compact_threshold, safety), /retry, /todo aliases

## Completed (v0.9.4)
- [x] Gemini 2.5 Pro chain-of-thought enabled (was excluded, now all 6 models use thinking)
- [x] Smarter is_complex_task: two-tier keyword system (strong vs medium+complexity)
  - Simple "create a file" → Flash; "implement entire auth system" → Pro
  - Saves Pro budget (25/day) for truly complex tasks
- [x] README updated with todo list, code reviewer, 19 tools, 487 tests
- [x] 487 unit tests

## Completed (v0.9.5)
- [x] Fixed UTF-8 safety bugs in TUI rendering (panics on multi-byte chars)
  - Suggestion popup description truncation used unsafe byte slicing
  - wrap_text() used byte length instead of char count for width
  - Tool arg display in TUI and prompt mode used unsafe byte slicing
  - Underflow protection for narrow terminals (< 15 columns)
- [x] Tool result summarizers for todo_update, find_files, list_directory, web_fetch, read_image
- [x] 16 new command tests: config clamping, safety aliases, MCP add/rm success paths
- [x] 535 unit tests

## Completed (v0.9.6)
- [x] File write size limit (5MB max) to prevent disk exhaustion on RPi3
- [x] Better edit_file ambiguity feedback: returns match_lines array for model to provide unique context
- [x] API error cascading: message → status → code (503 hints no longer lost)
- [x] Expanded safety: block sudo rm, dd of=, find -delete, shell redirects to /etc /sys /proc
- [x] Smarter thinking token preservation: keep last 3 messages' thinking for multi-turn reasoning
- [x] 547 unit tests

## Completed (v0.9.7)
- [x] Hardened save_conversation: path traversal protection, absolute paths
- [x] Fixed flaky MCP tests with process-unique server names
- [x] 60s web_fetch timeout for slow RPi3 Wi-Fi connections
- [x] Fixed Ctrl+W not updating suggestion popup
- [x] Undo history committed before critic (prevents data loss on critic failure)
- [x] stderr truncation in run_command (prevents token bloat from verbose output)

## Completed (v0.9.8)
- [x] Shared utils module: safe_truncate() extracted from 4 duplicate implementations
- [x] DRYed run_shell() truncation to use shared safe_truncate() helper
- [x] 6 tests for utils::safe_truncate + 5 tests for git::safe_truncate edge cases

## Completed (v0.9.9)
- [x] App unit tests: 26 new tests for TUI App methods (scroll, history, suggestions, save, clear, cancel)
- [x] Test helper: App::test_new() constructor for unit testing without API key
- [x] Coverage: scroll_up/down, clear_screen, cancel_processing, history_up/down cycles
- [x] Coverage: update_suggestions, select_suggestion, last_user_message, token_summary
- [x] Coverage: save_conversation (path traversal blocking, write, default filename)
- [x] Fixed MCP test flakiness: Mutex serializes config file access across parallel threads
- [x] 417 unit tests

## Completed (v0.10.0)
- [x] Shared BLOCKED_PATH_PREFIXES: single source of truth (file_ops.rs), reused by is_dangerous_tool_call()
- [x] is_dangerous_tool_call now also checks edit_file (not just write_file)
- [x] New dangerous command patterns: chown -R, eval, exec, > /boot
- [x] Expanded run_shell blocked list: dd of=, chown -R /, > /dev/, > /etc/, > /boot/
- [x] MCP response loop safety: max 1000 lines prevents infinite loop on misbehaving servers
- [x] 12 new tests for new safety patterns
- [x] 434 unit tests

## Completed (v0.10.1)
- [x] Ctrl+L keybinding: clears screen (was listed in help but not implemented)
- [x] 12 new handle_key tests: Ctrl+A/E/U/K/W/L, Backspace, Delete, Left/Right, Home/End, Esc, input blocked during processing
- [x] App::test_new() made pub(crate) for cross-module test access
- [x] README: updated test count, added utils.rs to architecture
- [x] 458 unit tests

## Completed (v0.10.2)
- [x] System prompt reduced ~80% (2700→500 bytes, saves ~550 tokens per request)
- [x] HTML stripper rewritten: byte-level scanning avoids double Vec<char> allocation
- [x] 2 new HTML tests: multibyte emoji content, case-insensitive SCRIPT tags
- [x] 462 unit tests

## Completed (v0.10.3)
- [x] Tool execution timing: each tool call is timed with std::time::Instant
- [x] duration_ms added to AgentEvent::ToolResult (0 for blocked calls)
- [x] TUI shows timing in tool result summary (e.g. "read_file: ok (12ms)")
- [x] CLI mode also shows timing in tool result line
- [x] Robustness: stdin read errors reported (not silently swallowed)
- [x] Robustness: MCP stderr capped at 32KB (prevents OOM on chatty servers)
- [x] Robustness: saturating arithmetic for token count (prevents u32 overflow)
- [x] Robustness: early-exit topic collection in compaction (pre-allocated, stops at 5)
- [x] HTML stripper: <style> blocks now stripped (saves tokens on web fetches)
- [x] HTML stripper refactored with generic hidden-tag system (script + style)
- [x] 2 new tests: style stripping, case-insensitive STYLE tags

## Completed (v0.10.4)
- [x] Configurable command timeout: `/config set command_timeout 120` (5-600s range)
- [x] Atomic-based timeout sharing (no mutex needed, thread-safe)
- [x] edit_file diagnostic hints: whitespace mismatch, case mismatch, or "read first"
- [x] MCP tool errors now include source server name for debugging
- [x] Timeout initialized from config at AgentLoop startup
- [x] 4 new tests: edit hints (whitespace, case, fallback), set_command_timeout
- [x] 441 unit tests

## Completed (v0.10.5)
- [x] Tool declaration descriptions trimmed (~30% shorter, saves ~100 tokens/request)
- [x] Thinking retention increased from 3 to 5 messages (better multi-turn reasoning)
- [x] Actionable error hints: rate limit exhaustion, network failures, model quotas
- [x] Help text: added Ctrl+A/E/U/K/W keybindings (was missing)
- [x] 441 unit tests

## Completed (v0.10.6)
- [x] Safety test coverage: 11 tests for is_dangerous_tool_call() (rm, dd, mkfs, shutdown, find -delete, curl|sh, eval, redirects, write_file paths)
- [x] Token optimization tests: 4 tests for truncate_tool_result() (small passthrough, large content/output truncation, non-object fallback)
- [x] Grep case-insensitive tests: positive + negative matching
- [x] UTF-8 truncation boundary test for web_fetch
- [x] git_branch creation + git_add multi-file tests
- [x] /config set timeout: 5 tests (set, alias, clamp low/high, invalid)
- [x] Config defaults: command_timeout, compact_threshold, system_prompt, serde backwards compat, corrupted JSON
- [x] 472 unit tests

## Completed (v0.10.7)
- [x] Ctrl+D to quit (Unix EOF convention, only when input is empty)
- [x] Fixed MCP client BufReader data loss: persistent reader across requests
- [x] MCP notification serialization: skip instead of sending empty string on failure
- [x] Styled welcome screen: ratatui Span-based gradient logo replaces ASCII art
- [x] Fixed context7 MCP package name: @anthropic-ai -> @upstash/context7-mcp
- [x] 474 unit tests

## Completed (v0.11.0) — Phase 1 Start
- [x] Multi-line input: type \ + Enter to add newlines (shows [NL] line count prefix)
- [x] Bash mode: !command runs shell directly, output displayed in TUI
- [x] Multi-line visual: green border tint, line count indicator, cursor tracks last line
- [x] Help text updated with multi-line and bash mode docs
- [x] 477 unit tests

## Completed (v0.11.1) — Session Persistence
- [x] Session auto-save on exit (JSON files in ~/.config/verysmolcode/sessions/)
- [x] /resume command: lists recent sessions or resumes specific one by ID
- [x] Session data: messages, input history, token counts, cwd, timestamp
- [x] Auto-prune: keeps only 10 most recent sessions to save disk
- [x] Bash mode context injection: !commands inform the AI of what user ran
- [x] 483 unit tests

## Completed (v0.11.2) — File Autocomplete & Diff
- [x] @ file autocomplete: type @ to see project files, arrows/tab to select
- [x] Fuzzy matching against git-tracked files (git ls-files)
- [x] File cache with 10s TTL (avoids repeated git calls)
- [x] /diff (/d) command: shows git diff output in TUI
- [x] Green-themed file suggestion popup (distinct from blue command popup)
- [x] 492 unit tests

# Long-Term Roadmap: OpenCode + Claude Code Feature Parity

## Design Principles for VSC
- All features must work on RPi3 (1GB RAM, 80x30 terminal)
- Gemini free tier limits drive architecture (unlike Claude/OpenCode which have paid APIs)
- Sync-first design (no async runtime) to keep binary small
- Every feature must consider token budget impact

## Phase 1: Core UX Improvements (v0.11.x) — HIGH PRIORITY
These are table-stakes features that both OpenCode and Claude Code have.

### P1.1 - Session Persistence & Resume ✅ (v0.11.1)
- [x] Save conversations to disk (JSON files)
- [x] /resume command to list and resume past sessions
- [x] Auto-save on exit, load on resume
- [x] Session cleanup (max 10 sessions, auto-prune)

### P1.2 - File Reference with @ Autocomplete ✅ (v0.11.2)
- [x] Type @ to trigger file path autocomplete dropdown
- [x] Fuzzy matching against project files (git ls-files)
- [ ] @file.rs#10-25 line range syntax (future enhancement)
- [ ] File content injected into conversation context
- [ ] Frecency ranking (recently used files first)

### P1.3 - Multi-Line Input
- [x] \ + Enter for multi-line input
- [x] Visual indicator showing multi-line mode (green border, line count)
- [ ] Paste detection (auto multi-line for pasted content)

### P1.4 - Bash Mode (! prefix)
- [ ] Type !command to run shell directly without AI
- [ ] Output displayed in TUI and added to conversation context
- [ ] History integration

### P1.5 - Improved Diff Display ✅ (v0.11.2, basic)
- [x] /diff command showing git diff in TUI
- [ ] Syntax-colored diff output (future enhancement)
- [ ] Per-turn diff tracking (what changed in this response)
- [ ] Side-by-side or unified diff view (width-adaptive)

### P1.6 - Context Window Visualization
- [ ] /context command showing colored grid of token usage
- [ ] Visual indicator of how full the context window is
- [ ] Warning when approaching auto-compact threshold

## Phase 2: Session & Navigation (v0.12.x)
Features that improve workflow continuity.

### P2.1 - Rewind / Checkpoint System
- [ ] Track state before each edit (extend current /undo)
- [ ] /rewind command with interactive checkpoint picker
- [ ] Restore code, conversation, or both
- [ ] Visual checkpoint markers in message history

### P2.2 - Command Palette
- [ ] Ctrl+P opens searchable command palette
- [ ] All slash commands + descriptions in fuzzy-filterable list
- [ ] MCP tools listed alongside built-in commands

### P2.3 - Reverse Search History
- [ ] Ctrl+R for interactive history search
- [ ] Highlight matches as you type
- [ ] Select and re-execute from history

### P2.4 - Leader Key System
- [ ] Configurable leader key (default Ctrl+X)
- [ ] Two-key combos: Leader+N new, Leader+L list sessions, etc.
- [ ] Discoverable via /help

### P2.5 - Fork Sessions
- [ ] /fork to branch conversation at current point
- [ ] Independent exploration without losing original thread

## Phase 3: Advanced Agent Features (v0.13.x)
Features that make the agent more capable.

### P3.1 - Subagent / Background Tasks
- [ ] Spawn lightweight sub-tasks (e.g., Explore agent for codebase search)
- [ ] Background task execution with notification on completion
- [ ] /tasks command to list running background work
- [ ] Ctrl+B to background current task

### P3.2 - Permission Modes
- [ ] Normal: prompt for writes (current behavior)
- [ ] Auto-accept: skip permission for file edits
- [ ] Plan-only: read-only tools (current /plan, make it a mode toggle)
- [ ] Configurable per-tool permissions

### P3.3 - Hooks System (Lightweight)
- [ ] Pre-tool-use hooks (block/allow specific operations)
- [ ] Post-tool-use hooks (run lint after edit, test after write)
- [ ] Session start hooks (auto-load context)
- [ ] JSON config in .vsc/hooks.json

### P3.4 - Structured Output Mode
- [ ] --json-schema flag for validated JSON output
- [ ] Useful for scripting/CI integration
- [ ] --print / -p non-interactive mode

### P3.5 - /simplify Command
- [ ] Review changed code for reuse, quality, efficiency
- [ ] Auto-fix issues found
- [ ] Uses critic model to keep costs low

## Phase 4: Polish & Ecosystem (v0.14.x)
Nice-to-have features for power users.

### P4.1 - Theme System
- [ ] JSON-based theme configuration
- [ ] Built-in themes: dark (current), light, high-contrast
- [ ] /theme command to switch
- [ ] Custom color definitions for all UI elements

### P4.2 - Vim Mode
- [ ] /vim toggle for vim-style input editing
- [ ] h/j/k/l navigation, w/e/b word movement
- [ ] Visual mode indicator (NORMAL/INSERT)

### P4.3 - Export & Share
- [ ] /export to markdown (improve current /save)
- [ ] Include tool calls, diffs, timing in export
- [ ] Clipboard copy of last response (Ctrl+Y or /copy)

### P4.4 - Plugin System (Lightweight)
- [ ] Load custom tools from .vsc/plugins/
- [ ] JavaScript/Python plugin scripts
- [ ] Plugin commands appear as slash commands

### P4.5 - /loop Command
- [ ] Run a prompt on recurring interval
- [ ] Cron-based scheduling
- [ ] Auto-expire after configurable duration

### P4.6 - /batch Command
- [ ] Parallel code changes across multiple files
- [ ] Each change in isolated context
- [ ] Merge results back

### P4.7 - Image Clipboard Paste
- [ ] Ctrl+V to paste screenshot from clipboard
- [ ] Auto-encode and send to Gemini vision

### P4.8 - Output Styles
- [ ] Default, Explanatory (educational), Learning (interactive)
- [ ] /output-style command to switch

## Phase 5: IDE & Desktop (v1.0+)
Long-term goals for ecosystem integration.

### P5.1 - VS Code Extension
- [ ] Basic extension with prompt box
- [ ] File reference from editor
- [ ] Inline diff display

### P5.2 - Non-Interactive Print Mode
- [ ] vsc -p "prompt" for CI/CD usage
- [ ] JSON output format
- [ ] Max turns / budget limits

### P5.3 - Remote/Desktop Mode
- [ ] Web interface for remote access
- [ ] Session sharing URLs

## Priority Matrix

| Feature | User Value | Effort | Priority |
|---------|-----------|--------|----------|
| Session persistence | Very High | Medium | P1 |
| @ file autocomplete | Very High | Medium | P1 |
| Multi-line input | High | Low | P1 |
| ! bash mode | High | Low | P1 |
| /diff command | High | Medium | P1 |
| Context visualization | Medium | Low | P1 |
| Rewind/checkpoint | High | High | P2 |
| Command palette | Medium | Medium | P2 |
| Ctrl+R history search | Medium | Medium | P2 |
| Subagents | High | High | P3 |
| Permission modes | Medium | Medium | P3 |
| Hooks system | Medium | High | P3 |
| Theme system | Low | Medium | P4 |
| Vim mode | Low | High | P4 |
| Plugin system | Low | Very High | P4 |

# Lessons Learned

## TLS Crate Selection
- Use **rustls** (ureq default features). native-tls breaks local builds AND manylinux Docker.
- ARM cross-compile: use **manylinux_2_28** (newer GCC defines __ARM_ARCH for ring).

## Gemini API Rate Limits
- Pro: 5 RPM, 25 RPD. Flash: 10 RPM, 250 RPD. Lite: 15 RPM, 1000 RPD.
- All 6 models independent limits = ~2550 total req/day.
- Wait-and-retry (up to 15s) for RPM limits keeps quality vs falling back to weaker model.
- Integration tests are flaky (API quota dependent) - use continue-on-error.

## Token Optimization (v0.7.5+)
- Truncate tool results > 8K chars before adding to conversation.
- Strip thinking tokens from history before resending.
- Scale thinking budget by tier: Pro 2048, Flash 1024, FlashLite 512.
- Only use Pro on first iteration; Flash for follow-ups.
- Conditional critic: only runs when files_modified is true.
- /tokens and /status show local cached data (no API call).

## PyO3/Maturin Wheel Builds
- PyO3 0.23 max Python 3.13. Manylinux images now ship 3.14.
- Fix: pin `-i python3.13` in maturin args. Match test-wheel Python version too.
- fail-fast: false in CI matrix so one arch failure doesn't cancel others.

## RPi3
- 906MB RAM, 4-core Cortex-A53. Native build works, ~30 min with -j1.
- pip install gets binary wheel (1.3MB), no compilation needed.
