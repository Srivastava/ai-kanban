#!/usr/bin/env bash
# Start backend and frontend, writing logs to ./logs/
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOGS="$SCRIPT_DIR/logs"
PIDS="$LOGS/pids"

mkdir -p "$LOGS"

# ---- Stop any already-running instances ----
_kill_port() {
  local port="$1"
  fuser -k "${port}/tcp" 2>/dev/null && echo "  freed port $port" || true
}

if [[ -f "$PIDS" ]]; then
  echo "Stopping previous instances..."
  while IFS= read -r pid; do
    kill "$pid" 2>/dev/null && echo "  killed PID $pid" || true
  done < "$PIDS"
  rm -f "$PIDS"
fi
# Also free ports in case of orphaned processes
_kill_port 3001
_kill_port 3002
sleep 1

# Log rotation: 1 MiB max per file, archived as <name>_<timestamp>.log
ROTATOR="python3 $SCRIPT_DIR/rotate_log.py"
LOG_MAX_BYTES=1048576

# ---- Backend ----
BACKEND_LOG="$LOGS/backend.log"
echo "[$(date '+%Y-%m-%d %H:%M:%S')] === Starting backend ===" >> "$BACKEND_LOG"

cd "$SCRIPT_DIR/backend"
cargo build 2>&1 | tee -a "$BACKEND_LOG"
mkfifo "$LOGS/backend.pipe"
$ROTATOR "$BACKEND_LOG" $LOG_MAX_BYTES < "$LOGS/backend.pipe" &
./target/debug/ai-kanban-backend > "$LOGS/backend.pipe" 2>&1 &
BACKEND_PID=$!
rm -f "$LOGS/backend.pipe"
echo "$BACKEND_PID" >> "$PIDS"
echo "Backend started (PID $BACKEND_PID) → $BACKEND_LOG"

# Give backend a moment to bind
sleep 1

# ---- Frontend ----
FRONTEND_LOG="$LOGS/frontend.log"
echo "[$(date '+%Y-%m-%d %H:%M:%S')] === Starting frontend ===" >> "$FRONTEND_LOG"

cd "$SCRIPT_DIR/frontend"
mkfifo "$LOGS/frontend.pipe"
$ROTATOR "$FRONTEND_LOG" $LOG_MAX_BYTES < "$LOGS/frontend.pipe" &
npm run dev -- --port 3002 > "$LOGS/frontend.pipe" 2>&1 &
FRONTEND_PID=$!
rm -f "$LOGS/frontend.pipe"
echo "$FRONTEND_PID" >> "$PIDS"
echo "Frontend started (PID $FRONTEND_PID) → $FRONTEND_LOG"

echo ""
echo "Both services running. Use ./logs.sh to view logs, ./stop.sh to stop."
