#!/usr/bin/env bash
# Manage and view logs for the ai-kanban project
#
# Usage:
#   ./logs.sh              — tail both logs (last 40 lines each, then follow)
#   ./logs.sh backend      — tail backend log only
#   ./logs.sh frontend     — tail frontend log only
#   ./logs.sh status       — show PID status + last 10 lines of each log
#   ./logs.sh errors       — show only ERROR lines from both logs
#   ./logs.sh clean        — rotate / archive old log files
#   ./logs.sh help         — show this help

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOGS="$SCRIPT_DIR/logs"
BACKEND_LOG="$LOGS/backend.log"
FRONTEND_LOG="$LOGS/frontend.log"
PIDS="$LOGS/pids"

cmd="${1:-tail}"

_pid_status() {
  if [[ -f "$PIDS" ]]; then
    echo "--- Running processes ---"
    while IFS= read -r pid; do
      if kill -0 "$pid" 2>/dev/null; then
        name=$(ps -p "$pid" -o comm= 2>/dev/null || echo "?")
        echo "  PID $pid  [$name]  running"
      else
        echo "  PID $pid  NOT running"
      fi
    done < "$PIDS"
  else
    echo "  No PID file — services may not be running."
  fi
  echo ""
}

case "$cmd" in
  backend)
    echo "=== backend log (Ctrl-C to stop) ==="
    tail -n 60 -f "$BACKEND_LOG" 2>/dev/null || echo "No backend log yet."
    ;;
  frontend)
    echo "=== frontend log (Ctrl-C to stop) ==="
    tail -n 60 -f "$FRONTEND_LOG" 2>/dev/null || echo "No frontend log yet."
    ;;
  status)
    _pid_status
    echo "--- Last 10 lines: backend ---"
    tail -n 10 "$BACKEND_LOG" 2>/dev/null || echo "  (no log)"
    echo ""
    echo "--- Last 10 lines: frontend ---"
    tail -n 10 "$FRONTEND_LOG" 2>/dev/null || echo "  (no log)"
    ;;
  errors)
    echo "=== ERROR lines (backend) ==="
    grep -i "error\|WARN\|panic" "$BACKEND_LOG" 2>/dev/null | tail -n 30 || echo "  none"
    echo ""
    echo "=== ERROR lines (frontend) ==="
    grep -i "error\|warn\|failed" "$FRONTEND_LOG" 2>/dev/null | tail -n 30 || echo "  none"
    ;;
  clean)
    ARCHIVE="$LOGS/archive-$(date '+%Y%m%d-%H%M%S')"
    mkdir -p "$ARCHIVE"
    for f in "$BACKEND_LOG" "$FRONTEND_LOG"; do
      [[ -f "$f" ]] && mv "$f" "$ARCHIVE/" && echo "Archived $(basename $f)"
    done
    echo "Logs archived to $ARCHIVE"
    ;;
  help|--help|-h)
    grep '^#' "$0" | sed 's/^# \?//'
    ;;
  tail|*)
    echo "=== Tailing both logs (Ctrl-C to stop) ==="
    tail -n 40 -f "$BACKEND_LOG" "$FRONTEND_LOG" 2>/dev/null \
      || echo "No logs yet — run ./start.sh first."
    ;;
esac
