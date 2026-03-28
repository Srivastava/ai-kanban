# Productionize AI Kanban

**Date:** 2026-03-27
**Status:** Approved

## Overview

Prepare AI Kanban for public distribution via GitHub. Users can download a prebuilt binary or build from source. A daily GitHub Actions workflow publishes nightly releases for all major platforms.

## Repository Setup

- Create private repo `github.com/Srivastava/ai-kanban`
- Root `README.md` covers: what the app does, prerequisites (Claude CLI required), binary quick-start, build-from-source instructions
- `.gitignore` excludes: `backend/target/`, `frontend/.next/`, `frontend/out/`, `node_modules/`, `*.db`, `logs/`, `data/`
- `LICENSE`: Business Source License 1.1 (BUSL 1.1)
  - Free for individuals and non-production use
  - Commercial/production use by companies requires a separate commercial license
  - Automatically converts to Apache 2.0 after 4 years from release date
  - Change Date: 2030-03-27

## License

BUSL 1.1 with the following parameters:
- **Licensor**: Srivastava
- **Licensed Work**: AI Kanban
- **Additional Use Grant**: None (production use by legal entities requires commercial license)
- **Change Date**: 2030-03-27
- **Change License**: Apache License, Version 2.0

## Production Build

### Frontend
- Set `output: 'export'` in `next.config.ts`
- `npm run build` produces static HTML/CSS/JS in `frontend/out/`
- No Node.js runtime required at run time

### Backend
- Add `rust-embed` crate to `backend/Cargo.toml`
- Embed `frontend/out/` directory at compile time using `#[derive(RustEmbed)]`
- Add static file handler in Axum: `GET /*` serves embedded frontend assets
- API routes (`/api/*`) and WebSocket (`/ws`) continue to work as before
- `cargo build --release` produces a single self-contained binary

### Updated start.sh
The production start script replaces the current dev-mode script:
1. `cd frontend && npm ci && npm run build`
2. `cargo build --release`
3. Run `./target/release/ai-kanban-backend` (serves both API and frontend on port 3001)

### install.sh (new)
A one-shot script for source users:
1. Check prerequisites (Rust toolchain, Node.js, Claude CLI)
2. `cd frontend && npm ci && npm run build`
3. `cd backend && cargo build --release`
4. Copy binary to project root as `./ai-kanban`
5. Print success message with run instructions

## GitHub Actions

### ci.yml
Triggered on: push and pull_request to `main`

Jobs:
- **backend**: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`
- **frontend**: `npm ci`, `npx eslint`, `npm test`

### release.yml
Triggered on:
- Schedule: daily at 02:00 UTC (`0 2 * * *`)
- Manual: `workflow_dispatch`

Build matrix (4 jobs):

| Job | Runner | Target triple | Binary name |
|-----|--------|---------------|-------------|
| linux-x86_64 | ubuntu-latest | x86_64-unknown-linux-gnu | ai-kanban-linux-x86_64 |
| macos-x86_64 | macos-latest | x86_64-apple-darwin | ai-kanban-macos-x86_64 |
| macos-arm64 | macos-latest | aarch64-apple-darwin | ai-kanban-macos-arm64 |
| windows-x86_64 | windows-latest | x86_64-pc-windows-msvc | ai-kanban-windows-x86_64.exe |

Each job:
1. Checkout repo
2. Install Node.js (LTS) and Rust (stable)
3. `cd frontend && npm ci && npm run build`
4. `cd backend && cargo build --release --target <triple>`
5. Compute SHA256 checksum of binary
6. Upload binary as workflow artifact

Release job (runs after all build jobs succeed):
1. Create GitHub Release tagged `nightly-YYYY-MM-DD`
2. Attach all 4 binaries and a `checksums.txt` file
3. Mark as pre-release
4. Delete releases older than 7 days (keep last 7 nightly tags)

## Install Experience

### Binary (quick start)
```bash
# Linux
curl -L https://github.com/Srivastava/ai-kanban/releases/latest/download/ai-kanban-linux-x86_64 -o ai-kanban
chmod +x ai-kanban
./ai-kanban

# macOS (Apple Silicon)
curl -L https://github.com/Srivastava/ai-kanban/releases/latest/download/ai-kanban-macos-arm64 -o ai-kanban
chmod +x ai-kanban
./ai-kanban
```

App is available at `http://localhost:3001` after launch.

### Source (power users)
```bash
git clone https://github.com/Srivastava/ai-kanban
cd ai-kanban
./install.sh
./ai-kanban
```

## Prerequisites (all install paths)
- **Claude CLI** installed and authenticated (`claude` must be on PATH)
- For source builds: Rust stable toolchain, Node.js 20+

## Out of Scope
- Docker/container support (Claude CLI requires host filesystem and credentials)
- Auto-update mechanism
- Code signing / notarization for macOS/Windows binaries
- Systemd service setup (can be added later)
