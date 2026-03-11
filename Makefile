.PHONY: help build release install test test-integration lint fmt fmt-check clean run dev-wheel dist

# Default target
help:
	@echo ""
	@echo "VerySmolCode — Common Commands"
	@echo "================================"
	@echo ""
	@echo "Build:"
	@echo "  make build          Build debug binary (cargo build)"
	@echo "  make release        Build optimized release binary"
	@echo "  make install        Install vsc binary to ~/.cargo/bin"
	@echo "  make dev-wheel      Build Python wheel in dev mode (maturin develop)"
	@echo "  make dist           Build release wheels for Python 3.11/3.12/3.13"
	@echo ""
	@echo "Run:"
	@echo "  make run            Launch vsc TUI"
	@echo "  make version        Show installed vsc version"
	@echo ""
	@echo "Test & Quality:"
	@echo "  make test           Run all unit tests"
	@echo "  make test-integration  Run integration test (requires tmux + GEMINI_API_KEY)"
	@echo "  make lint           Run clippy with strict warnings"
	@echo "  make fmt            Auto-format code"
	@echo "  make fmt-check      Check formatting without modifying"
	@echo "  make check          fmt-check + lint + test (same as pre-commit hook)"
	@echo ""
	@echo "Maintenance:"
	@echo "  make clean          Remove build artifacts (cargo clean)"
	@echo "  make disk           Show disk usage of key directories"
	@echo ""
	@echo "Release:"
	@echo "  make tag VERSION=0.x.y   Create and push a git version tag"
	@echo ""

# ── Build ────────────────────────────────────────────────────────────────────

build:
	cargo build

release:
	cargo build --release

install:
	cargo install --path .

dev-wheel:
	maturin develop --features python

dist:
	maturin build --release --out dist --features python -i python3.11 python3.12 python3.13

# ── Run ──────────────────────────────────────────────────────────────────────

run:
	vsc

version:
	vsc --version

# ── Test & Quality ───────────────────────────────────────────────────────────

test:
	cargo test

test-integration:
	bash tests/integration_test.sh

lint:
	cargo clippy -- -D warnings

fmt:
	cargo fmt

fmt-check:
	cargo fmt -- --check

check: fmt-check lint test

# ── Maintenance ───────────────────────────────────────────────────────────────

clean:
	cargo clean

disk:
	@echo "--- Disk free ---"
	@df -h /
	@echo ""
	@echo "--- Cargo target dir ---"
	@du -sh target/ 2>/dev/null || echo "(no target/ dir)"
	@echo ""
	@echo "--- dist/ dir ---"
	@du -sh dist/ 2>/dev/null || echo "(no dist/ dir)"

# ── Release ───────────────────────────────────────────────────────────────────

tag:
ifndef VERSION
	$(error VERSION is not set. Usage: make tag VERSION=0.x.y)
endif
	git tag -a "v$(VERSION)" -m "Release v$(VERSION)"
	git push origin "v$(VERSION)"
