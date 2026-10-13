"""Summarize gap-related MotHor tables captured by scan_scoring_object.py."""

import argparse
import json
import struct
from pathlib import Path


def u32(data, offset):
    return struct.unpack_from("<I", data, offset)[0]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("prefix", type=Path, help="prefix passed to --dump-prefix")
    args = parser.parse_args()
    context = Path(str(args.prefix) + "-context.bin").read_bytes()
    count = u32(context, 0x4704)
    if not 0 < count <= 64:
        raise ValueError(f"unexpected context+0x4704 count: {count}")

    tables = {}
    for field, stride, width in ((0x3788, 0x300, 1), (0x378C, 0x300, 1),
                                 (0x3848, 0xC00, 4), (0x384C, 0xC00, 4)):
        path = Path(str(args.prefix) + f"-table-{field:04x}.bin")
        if not path.exists():
            tables[f"+0x{field:x}"] = {"pointer": f"0x{u32(context, field):08x}",
                                      "dump": "unavailable"}
            continue
        data = path.read_bytes()
        complete_rows = min(count, len(data) // stride)
        nonzero = []
        for index in range(complete_rows):
            row = data[index * stride:(index + 1) * stride]
            if width == 1:
                nonzero.append(sum(value != 0 for value in row))
            else:
                values = struct.unpack(f"<{stride // 4}I", row)
                nonzero.append(sum(value != 0 for value in values))
        tables[f"+0x{field:x}"] = {
            "pointer": f"0x{u32(context, field):08x}",
            "strideBytes": stride,
            "completeRows": complete_rows,
            "nonzeroByIndex": nonzero,
        }

    pairs = {}
    for first, second in ((0x3788, 0x3848), (0x378C, 0x384C)):
        pairs[f"+0x{first:x} minus +0x{second:x}"] = u32(context, first) - u32(context, second)
    print(json.dumps({"count": count, "tables": tables, "pointerDifferences": pairs}, indent=2))


if __name__ == "__main__":
    main()
