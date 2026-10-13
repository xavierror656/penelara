"""Sample MotHor group-grid buffers during one isolated replay.

The hashes cover the five 0x600-byte planes, excluding eight trailing bytes.
Samples are sequential, so a concurrent engine update can produce a mixed row.
"""

import argparse
import ctypes
import hashlib
import json
import struct
import time
from pathlib import Path

from scan_scoring_object import kernel32, read_memory, read_u32


def read_json(path):
    raw = path.read_bytes()
    encoding = "utf-16" if raw.startswith((b"\xff\xfe", b"\xfe\xff")) else "utf-8-sig"
    return json.loads(raw.decode(encoding))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("pid", type=int)
    parser.add_argument("scan_json", type=Path)
    parser.add_argument("--seconds", type=float, default=20)
    parser.add_argument("--interval", type=float, default=0.1)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    if args.seconds <= 0 or args.interval <= 0:
        parser.error("seconds and interval must be positive")
    scan = read_json(args.scan_json)
    contexts = [entry for entry in scan["contexts"] if "objectPool" in entry]
    if scan["pid"] != args.pid or len(contexts) != 1:
        parser.error("scan JSON must contain one context from this live PID")
    context = int(contexts[0]["base"], 16)

    handle = kernel32.OpenProcess(0x410, False, args.pid)
    if not handle:
        raise OSError(ctypes.get_last_error(), "OpenProcess")
    output = args.output.open("w", encoding="utf-8") if args.output else None
    try:
        started = time.monotonic()
        while time.monotonic() - started < args.seconds:
            row = {"elapsed": round(time.monotonic() - started, 3),
                   "bestCompleteWeight": read_u32(handle, 0x49324C),
                   "currentWeight": read_u32(handle, 0x493230),
                   "unplacedCount": read_u32(handle, 0x493234), "buffers": {}}
            for field in (0x1298, 0x39C8, 0x39E0, 0x39F8):
                pointer = read_u32(handle, context + field)
                data = read_memory(handle, pointer, 0x1E08) if pointer else None
                if data is None:
                    row["buffers"][f"0x{field:x}"] = None
                    continue
                planes = data[:0x1E00]
                row["buffers"][f"0x{field:x}"] = {
                    "pointer": f"0x{pointer:08x}",
                    "sha256": hashlib.sha256(planes).hexdigest(),
                    "occupied": [sum(value != 0 for value in struct.unpack_from("<384I", planes, 0x600 * i))
                                 for i in range(5)],
                }
            line = json.dumps(row)
            if output:
                output.write(line + "\n")
                output.flush()
            else:
                print(line, flush=True)
            time.sleep(args.interval)
    finally:
        if output:
            output.close()
        kernel32.CloseHandle(handle)


if __name__ == "__main__":
    main()
