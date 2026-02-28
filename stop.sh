#!/usr/bin/env bash
# Stop backend and frontend processes
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PIDS="$SCRIPT_DIR/logs/pids"

_kill_port() {
  local port="$1"
  fuser -k "${port}/tcp" 2>/dev/null && echo "  freed port $port" || true
}

if [[ -f "$PIDS" ]]; then
  while IFS= read -r pid; do
    kill "$pid" 2>/dev/null && echo "Stopped PID $pid" || echo "PID $pid not running"
  done < "$PIDS"
  rm -f "$PIDS"
fi

# Also free ports in case of orphaned processes
_kill_port 3001
_kill_port 3002

echo "Done."
