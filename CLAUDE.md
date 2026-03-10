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
