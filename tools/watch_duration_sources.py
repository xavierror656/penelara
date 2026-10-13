"""Sample MotHor duration-source counters during a replay (read-only)."""

import argparse
import ctypes
import json
import time

from scan_scoring_object import kernel32, read_u32


def capture(handle):
    app = read_u32(handle, 0x4932C0)
    bridge = read_u32(handle, app + 0xDC) if app else None
    node = read_u32(handle, bridge + 0xD047C) if bridge else None
    records = []
    seen = set()
    while node and node not in seen and len(records) < 256:
        seen.add(node)
        records.append({
            "address": f"0x{node:08x}",
            "field31c": read_u32(handle, node + 0x31C),
            "field324": read_u32(handle, node + 0x324),
        })
        node = read_u32(handle, node)
    return {
        "app": f"0x{app:08x}" if app else None,
        "records": records,
        "unplacedCount": read_u32(handle, 0x493234),
        "bestCompleteWeight": read_u32(handle, 0x49324C),
        "listTerminated": node == 0,
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("pid", type=int)
    parser.add_argument("--seconds", type=float, default=8)
    parser.add_argument("--interval-ms", type=int, default=100)
    args = parser.parse_args()
    handle = kernel32.OpenProcess(0x410, False, args.pid)
    if not handle:
        raise OSError(ctypes.get_last_error(), "OpenProcess")
    try:
        start = time.monotonic()
        while time.monotonic() - start < args.seconds:
            row = capture(handle)
            row["elapsed"] = round(time.monotonic() - start, 3)
            print(json.dumps(row, separators=(",", ":")), flush=True)
            time.sleep(args.interval_ms / 1000)
    finally:
        kernel32.CloseHandle(handle)


if __name__ == "__main__":
    main()
