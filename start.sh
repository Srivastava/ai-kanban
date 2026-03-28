#!/usr/bin/env bash
# Build and run AI Kanban in production mode.
# Requires: Rust stable, Node.js 20+, Claude CLI on PATH.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOGS="$SCRIPT_DIR/logs"
mkdir -p "$LOGS"

echo "==> Building frontend..."
cd "$SCRIPT_DIR/frontend"
npm ci --silent
npm run build

echo "==> Building backend (embeds frontend)..."
cd "$SCRIPT_DIR/backend"
cargo build --release 2>&1 | tee -a "$LOGS/build.log"

echo "==> Starting AI Kanban on http://localhost:3001"
exec ./target/release/ai-kanban-backend
