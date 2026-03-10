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

## Completed (v0.7.4)
- [x] Repo setup (gitignore, license, maturin/pyo3)
- [x] Gemini API client with 6-model routing (Gemini 3.x + 2.5)
- [x] Rate limiting with per-model RPM/RPD tracking
- [x] Tool system: 18 tools (file read/write/edit, grep, find, git, shell, web fetch, image reading)
- [x] TUI: blue theme, input history, slash commands, scrolling
- [x] Agent loop with automatic model fallback (503/429 aware)
- [x] Deep fallback chain: tries ALL 6 models before giving up
- [x] Critic verification (cheapest available model reviews completed work)
- [x] Planning mode (/plan) with read-only tools
- [x] Chain-of-thought for Flash and Gemini 3.1 Pro models (thinkingConfig)
- [x] Safety: blocks destructive ops, validates paths
- [x] MCP client support (stdio JSON-RPC protocol) - wired into agent loop
- [x] MCP server management (/mcp, /mcp-add, /mcp-rm)
- [x] Image reading tool (base64 encode + send as InlineData to Gemini)
- [x] Pre-commit hooks (fmt, clippy, tests)
- [x] 90 unit tests across 5 test files
- [x] Integration test (tmux + bottle.py todo app - continue-on-error, API-dependent)
- [x] README.md documentation
- [x] CI/CD pipeline: test -> build-wheels -> test-wheel -> publish-pypi
- [x] Rayon parallelism for grep_search and find_files (leverages multi-core on RPi3)
- [x] GitHub releases (v0.2.0 through v0.7.4)
- [x] PyPI publishing pipeline (sdist works, wheel builds need TLS fix)

## In Progress - Token Consumption Optimization (HIGH PRIORITY)
- [ ] Audit and reduce unnecessary token usage in agent loop
  - Current: Each iteration clones and sends FULL conversation history
  - Current: max_tokens_per_response=4096, auto_compact at 24K tokens
  - Current: Critic sends full conversation again after tool use
  - Issue: Pro only has 25 req/day, Flash 250/day - every wasted request hurts
- [ ] Add token budget awareness to the agent loop
  - Show remaining requests in TUI prominently
  - Warn user when approaching daily limits
  - Let user choose model tier per request (/fast, /smart)
- [ ] Smarter conversation compaction
  - Compact tool results aggressively (keep only summaries, not full file contents)
  - Prune thinking tokens from history before resending
  - Truncate large tool results (e.g. grep with 50 matches)
- [ ] Add /tokens slash command to show detailed usage breakdown

## In Progress - ARM Binary Distribution (HIGH PRIORITY)
- [ ] Fix ARM wheel cross-compilation in CI
  - ring crate fails in manylinux Docker for aarch64 (#error "ARM assembler must define __ARM_ARCH")
  - native-tls approach fails because manylinux lacks OpenSSL dev headers
  - Options to explore: (1) set CFLAGS=-D__ARM_ARCH=8 for ring, (2) vendored-openssl, (3) before-script-linux to install openssl-dev
  - RPi3 native build works (builds with ring/rustls natively on aarch64)
- [ ] Verify pip install installs binary wheel (not sdist) on RPi3
  - User has Python 3.13 and RPi3 (aarch64, Debian)
  - Must use venv for testing
- [ ] Consider building wheels natively on RPi or ARM CI runner as fallback

## Planned
- [ ] Token usage dashboard in TUI
- [ ] Configuration editing via slash commands (/config set)
- [ ] Increase test coverage toward 100%
- [ ] Wait-and-retry when per-minute rate limit hit (instead of immediate fallback to weaker model)

# Lessons Learned

## TLS Crate Selection for Cross-Platform Builds
- **rustls** (via ring): Works perfectly for ALL native builds (x86_64, aarch64, armv7). Fails to cross-compile for ARM in manylinux Docker containers due to ring's assembly requirements.
- **native-tls** (via openssl): Requires OpenSSL dev headers at compile time AND OpenSSL library at runtime. Fails in manylinux Docker (no openssl-dev). Also produced "no TLS backend" errors on local machine (v0.7.2-0.7.3 were broken).
- **Conclusion**: Use rustls (ureq default features) for the binary. Solve cross-compilation separately with CFLAGS or vendored OpenSSL for manylinux Docker builds.

## Gemini API Rate Limits Are Harsh
- Pro: 5 RPM, 25 RPD - extremely limited. A single complex task with 10+ tool iterations burns nearly half the daily budget.
- Flash: 10 RPM, 250 RPD - more workable but still tight.
- Flash-Lite: 15 RPM, 1000 RPD - most budget but least capable.
- All 6 models have independent limits, giving ~2550 total requests/day.
- The 3.x preview models may have different/stricter limits than documented. When ALL 6 models return 429, the API key's global quota may be exhausted.
- Integration tests are inherently flaky because they depend on API availability.

## Agent Loop Token Consumption
- The agent loop sends full conversation history on EVERY iteration (up to 15 iterations).
- Tool results (especially grep, read_file) can be very large and stay in conversation history.
- The critic step adds another full-conversation API call after the agent completes.
- Auto-compact at 24K tokens helps but doesn't prevent large intermediate states.
- Need aggressive truncation of tool results and conversation pruning to stay within budget.

## RPi3 Build Considerations
- RPi3 has 906MB RAM, Cortex-A53 (4 cores). Release builds with LTO use ~422MB RAM.
- Build needs 2GB swap file to complete. /tmp is 100MB tmpfs - must build in home dir.
- serde_derive takes ~5 min to compile, full build takes ~30+ min with -j1.
- Rayon leverages all 4 cores for grep/find, good for runtime performance.
- Native aarch64 build with ring/rustls works fine - only cross-compilation fails.

## CI Pipeline Design
- Integration test depends on Gemini API availability. Made it continue-on-error so wheel builds aren't blocked.
- Wheel builds only depend on unit tests (deterministic) not integration test (flaky).
- matrix strategy with fail-fast means one arch failure cancels all others. May want to set fail-fast: false.
