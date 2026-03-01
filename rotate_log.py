#!/usr/bin/env python3
"""
Stream log rotator.

Reads stdin line-by-line and writes to <log_file>.
When <log_file> exceeds <max_bytes> (default 1 MiB), it is renamed to
<stem>_<timestamp>.log and a new <log_file> is opened.

Usage: rotate_log.py <log_file> [max_bytes]
"""
import sys
import os
import datetime


def archived_name(log_file: str) -> str:
    ts = datetime.datetime.now().strftime("%Y-%m-%dT%H-%M-%S")
    log_dir = os.path.dirname(log_file)
    base = os.path.basename(log_file)
    stem = base[:-4] if base.endswith(".log") else base
    return os.path.join(log_dir, f"{stem}_{ts}.log")


def main() -> None:
    if len(sys.argv) < 2:
        print("Usage: rotate_log.py <log_file> [max_bytes]", file=sys.stderr)
        sys.exit(1)

    log_file = os.path.abspath(sys.argv[1])
    max_bytes = int(sys.argv[2]) if len(sys.argv) > 2 else 1_048_576  # 1 MiB

    os.makedirs(os.path.dirname(log_file), exist_ok=True)
    fh = open(log_file, "a", buffering=1, encoding="utf-8", errors="replace")
    counter = 0

    try:
        for line in sys.stdin:
            fh.write(line)
            counter += 1
            # Check file size every 100 lines to keep overhead low
            if counter % 100 == 0:
                fh.flush()
                if os.path.exists(log_file) and os.path.getsize(log_file) >= max_bytes:
                    fh.close()
                    dest = archived_name(log_file)
                    os.rename(log_file, dest)
                    fh = open(log_file, "a", buffering=1, encoding="utf-8", errors="replace")
    finally:
        fh.flush()
        fh.close()


if __name__ == "__main__":
    main()
