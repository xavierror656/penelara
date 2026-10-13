"""Read MotHor scoring record kind, limit and strict flag from a live replay."""

import argparse
import ctypes
import json
from pathlib import Path
import struct

from inspect_native_limits import kernel32, read_bytes, read_u32


def describe_record(raw: bytes, index: int) -> dict:
    return {
        "index": index,
        "secondTableWeeklyLoad": struct.unpack_from("<i", raw, 0x68)[0],
        "active": struct.unpack_from("<i", raw, 0x6C)[0],
        "kind": struct.unpack_from("<i", raw, 0x90)[0],
        "weeklyNonStrictPermanenceLimit": struct.unpack_from("<i", raw, 0x1C)[0],
        "dailyNonStrictPermanenceLimit": struct.unpack_from("<i", raw, 0x24)[0],
        "maximas": struct.unpack_from("<b", raw, 0x28)[0],
        "gapMode5b": raw[0x5B],
        "estricto": raw[0xB9],
        "dailyLoadModes": {
            "max_a": raw[0x61],
            "max_b": raw[0x63],
            "min_a": raw[0x85],
            "min_b": raw[0x87],
            "max_mode": raw[0x63],
            "min_mode": raw[0x87],
        },
        "dailyLoadAdjustments": {
            "max_a": raw[0x62],
            "max_b": raw[0x64],
            "min_a": raw[0x86],
            "min_b": raw[0x88],
        },
        "dailyLoadBounds": {
            "upper_a": struct.unpack_from("<b", raw, 0x70)[0],
            "upper_b": raw[0x71],
            "lower_a": struct.unpack_from("<b", raw, 0x81)[0],
            "lower_b": raw[0x83],
        },
        "dailyLoadInputs": {
            "requested_free_days": raw[0x5C],
            "day_count_mode_5e": raw[0x5E],
            "upper_floor": raw[0x72],
            "available_days_a": raw[0x80],
            "available_days_b": raw[0x82],
        },
        "dailyLoadDayMarks": {
            "second_table_00": list(raw[0x00:0x06]),
            "first_table_06": list(raw[0x06:0x0C]),
            "shared_0c": list(raw[0x0C:0x12]),
            "source_94": list(raw[0x94:0x9A]),
            "source_98": list(raw[0x98:0x9E]),
            "source_9c": list(raw[0x9C:0xA2]),
        },
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("pid", type=int, nargs="?")
    parser.add_argument("--dump-prefix")
    parser.add_argument("--from-dump", type=Path, help="decode a saved 0xBC-stride frame dump")
    parser.add_argument("--compare-dump", type=Path, help="compare every saved record byte")
    args = parser.parse_args()
    if args.from_dump:
        raw = args.from_dump.read_bytes()
        if len(raw) % 0xBC:
            raise ValueError("record dump length is not a multiple of 0xBC")
        rows = [describe_record(raw[index:index + 0xBC], index // 0xBC)
                for index in range(0, len(raw), 0xBC)]
        result = {"file": str(args.from_dump), "rows": rows}
        if args.compare_dump:
            other = args.compare_dump.read_bytes()
            if len(other) != len(raw):
                raise ValueError("record dumps have different lengths")
            result["comparison"] = {
                "file": str(args.compare_dump),
                "changed_bytes": [
                    {"record": index // 0xBC, "offset": index % 0xBC,
                     "before": before, "after": after}
                    for index, (before, after) in enumerate(zip(raw, other))
                    if before != after
                ],
            }
        print(json.dumps(result))
        return
    if args.pid is None:
        parser.error("pid or --from-dump is required")
    if args.compare_dump:
        parser.error("--compare-dump requires --from-dump")
    handle = kernel32.OpenProcess(0x1000 | 0x10, False, args.pid)
    if not handle:
        raise OSError(ctypes.get_last_error(), "OpenProcess")
    try:
        app = read_u32(handle, 0x4932C0)
        if app is None:
            raise RuntimeError("MotHor did not expose CGHCDatos")
        frames = []
        for frame in range(2):
            records = read_u32(handle, app + 0xF8 + frame * 4)
            rows = []
            raw_rows = bytearray()
            if records:
                for index in range(12):
                    raw = read_bytes(handle, records + 0xBC * index, 0xBC)
                    if raw is None:
                        break
                    raw_rows.extend(raw)
                    rows.append(describe_record(raw, index))
            entry = {"frame": frame, "rows": rows}
            if args.dump_prefix and raw_rows:
                path = Path(f"{args.dump_prefix}-scoring-records-{frame:02d}.bin")
                path.write_bytes(raw_rows)
                entry["dump"] = str(path)
            frames.append(entry)
        print(json.dumps({
            "pid": args.pid,
            "app": app,
            "bestCompleteWeight": read_u32(handle, 0x49324C),
            "scoreSlot0": read_u32(handle, 0x493250),
            "frames": frames,
        }))
    finally:
        kernel32.CloseHandle(handle)


if __name__ == "__main__":
    main()
