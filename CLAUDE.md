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
