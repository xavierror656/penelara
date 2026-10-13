"""Sample selected MotHor unit pointers in live teacher grids."""

import argparse
import ctypes
import json
import struct
import time
from pathlib import Path

from scan_scoring_object import kernel32, read_memory, read_u32


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("pid", type=int)
    parser.add_argument("scan_json", type=Path,
                        help="JSON from scan_scoring_object.py for this same process")
    parser.add_argument("--units", type=int, nargs="+", required=True)
    parser.add_argument("--seconds", type=float, default=15)
    parser.add_argument("--interval", type=float, default=0.1)
    args = parser.parse_args()
    if args.interval <= 0 or args.seconds <= 0:
        parser.error("seconds and interval must be positive")

    scan = json.loads(args.scan_json.read_text(encoding="ascii"))
    if scan["pid"] != args.pid:
        raise ValueError("scan PID does not match the process being watched")
    candidates = [entry for entry in scan["contexts"] if "objectPool" in entry]
    if len(candidates) != 1:
        raise ValueError(f"expected one analyzed context, got {len(candidates)}")
    context = int(candidates[0]["base"], 16)
    pool = int(candidates[0]["objectPool"]["base"], 16)
    stride = candidates[0]["objectPool"]["stride"]
    wanted = {pool + unit * stride: unit for unit in args.units}

    handle = kernel32.OpenProcess(0x410, False, args.pid)
    if not handle:
        raise OSError(ctypes.get_last_error(), "OpenProcess")
    try:
        rows = read_u32(handle, context + 0x4704)
        if rows is None or not 0 < rows <= 64:
            raise ValueError(f"invalid teacher row count: {rows}")
        started = time.monotonic()
        last_signature = None
        last_print = -1.0
        while time.monotonic() - started < args.seconds:
            elapsed = round(time.monotonic() - started, 3)
            located = {unit: [] for unit in args.units}
            occupied = {}
            valid = True
            for field, frame in ((0x3848, "A"), (0x384C, "B")):
                table_pointer = read_u32(handle, context + field)
                table = read_memory(handle, table_pointer, rows * 0xC00) if table_pointer else None
                if table is None:
                    valid = False
                    break
                values = struct.unpack(f"<{rows * 768}I", table)
                occupied[frame] = sum(value != 0 for value in values)
                for index, value in enumerate(values):
                    if value in wanted:
                        located[wanted[value]].append({"frame": frame,
                                                       "teacherIndex": index // 768,
                                                       "position": index % 768})
            if valid:
                signature = tuple(bool(located[unit]) for unit in args.units)
                if signature != last_signature or elapsed - last_print >= 1:
                    row = {"elapsed": elapsed, "units": located,
                           "occupiedCells": occupied,
                           "unplacedCounter": read_u32(handle, 0x493234),
                           "bestIncompleteWeight": read_u32(handle, 0x493240),
                           "bestCompleteWeight": read_u32(handle, 0x49324C)}
                    print(json.dumps(row), flush=True)
                    last_signature = signature
                    last_print = elapsed
            time.sleep(args.interval)
    finally:
        kernel32.CloseHandle(handle)


if __name__ == "__main__":
    main()
