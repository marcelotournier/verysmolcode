# VerySmolCode
- Repository: https://github.com/marcelotournier/verysmolcode
- Create VerySmolCode, a rust-based TUI that mimics Claude Code and works with Gemini API free tier. GEMINI_API_KEY is in the env variable
- TUI must be lightweight enough to run in a memory constrained device such as a raspberrypi 3 (1GB RAM)
- Make sure this respects the free tier API limits for models. Smarter models get fewer requests: Pro gives 5/min and 100/day, Flash 10/min and 250/day, Flash-Lite 15/min and 1,000/day. Look for the most up to date models in google documentation
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

## Completed (v0.3.0)
- [x] Repo setup (gitignore, license, maturin/pyo3)
- [x] Gemini API client with Pro/Flash/Flash-Lite model routing
- [x] Rate limiting with per-model RPM/RPD tracking
- [x] Tool system: file read/write/edit, grep, find, git, shell, web fetch
- [x] TUI: blue theme, input history, slash commands, scrolling
- [x] Agent loop with automatic model fallback
- [x] Critic verification (Flash-Lite reviews completed work)
- [x] Planning mode (/plan) with read-only tools
- [x] Chain-of-thought for Flash models (thinkingConfig)
- [x] Safety: blocks destructive ops, validates paths
- [x] MCP client support (stdio JSON-RPC protocol)
- [x] MCP server management (/mcp, /mcp-add, /mcp-rm)
- [x] Pre-commit hooks (fmt, clippy, tests)
- [x] 47 unit tests
- [x] README.md documentation
- [x] CI/CD with GitHub Actions
- [x] GitHub releases (v0.2.0, v0.3.0)

## In Progress
- [ ] Integration tests (tmux + bottle.py todo app)
- [ ] Wire MCP tools into the agent loop (currently config-only)

## Planned
- [ ] Image reading tool (base64 encode + send to Gemini)
- [ ] Token usage dashboard in TUI
- [ ] Configuration editing via slash commands (/config set)
- [ ] More test coverage (target 100%)
- [ ] Release binary builds for ARM (Raspberry Pi)

