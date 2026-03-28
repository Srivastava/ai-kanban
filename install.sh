#!/usr/bin/env bash
# One-shot installer: builds frontend + backend and copies binary to project root.
# After running, start the app with: ./ai-kanban
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# --- Prerequisite checks ---
check_cmd() {
  if ! command -v "$1" &>/dev/null; then
    echo "ERROR: '$1' not found. $2"
    exit 1
  fi
}

check_cmd node  "Install Node.js 20+ from https://nodejs.org"
check_cmd cargo "Install Rust from https://rustup.rs"
check_cmd claude "Install Claude CLI from https://docs.anthropic.com/en/docs/claude-cli"

echo "==> Building frontend..."
cd "$SCRIPT_DIR/frontend"
npm ci --silent
npm run build

echo "==> Building backend (embeds frontend)..."
cd "$SCRIPT_DIR/backend"
cargo build --release

echo "==> Installing binary to project root..."
cp "$SCRIPT_DIR/backend/target/release/ai-kanban-backend" "$SCRIPT_DIR/ai-kanban"
chmod +x "$SCRIPT_DIR/ai-kanban"

echo ""
echo "Done! Run the app with:"
echo "  ./ai-kanban"
echo ""
echo "Then open http://localhost:3001 in your browser."
