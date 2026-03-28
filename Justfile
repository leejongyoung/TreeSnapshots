# TreeSnapshots — command runner
# Install: brew install just
# Usage:   just <recipe>

# Show available recipes
default:
    @just --list

# ── CLI ───────────────────────────────────────────────────────────────────────

# Run CLI in development mode
dev-cli:
    cargo run -p treesnapshots-cli

# Build CLI for production
build-cli:
    cargo build --release -p treesnapshots-cli

# Install CLI globally (~/.cargo/bin/treesnapshots)
install:
    cargo install --path apps/cli

# Uninstall CLI from ~/.cargo/bin
uninstall:
    cargo uninstall treesnapshots-cli

# Clean build artifacts
clean:
    cargo clean

# ── GUI ───────────────────────────────────────────────────────────────────────

# Run GUI in development mode
dev-gui:
    cd apps/gui && npm install --silent && npm run tauri dev

# Build GUI for production
build-gui:
    cd apps/gui && npm install --silent && npm run tauri build
