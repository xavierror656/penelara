"""Correlate paired MotHor object references across adjacent group planes.

The default object pairs are from the DosMarcos fixture. The script reports
observed addresses and offsets; it does not infer native write instructions.
"""

import argparse
import json
import struct
from pathlib import Path


def read_json(path):
    raw = path.read_bytes()
    encoding = "utf-16" if raw.startswith((b"\xff\xfe", b"\xfe\xff")) else "utf-8-sig"
    return json.loads(raw.decode(encoding))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("scan_json", type=Path)
    parser.add_argument("references_json", type=Path, nargs="+")
    parser.add_argument("--pairs", default="0:1,2:3,4:5,6:7,8:9,10:11,82:83,84:85")
    parser.add_argument("--primary-index", type=int, default=2)
    parser.add_argument("--plane-size", type=lambda s: int(s, 0), default=0x600)
    args = parser.parse_args()

    scan = read_json(args.scan_json)
    contexts = [c for c in scan["contexts"] if "contextDump" in c]
    if len(contexts) != 1:
        parser.error(f"expected one dumped context, found {len(contexts)}")
    context_path = Path(contexts[0]["contextDump"])
    if not context_path.is_absolute():
        context_path = Path.cwd() / context_path
    context = context_path.read_bytes()
    base = struct.unpack_from("<I", context, 0x39E0)[0]
    other_copy = struct.unpack_from("<I", context, 0x39F8)[0]
    # The observed structure has one leading plane before group index zero.
    primary_start = base + (args.primary_index + 1) * args.plane_size
    primary_end = primary_start + args.plane_size
    pairs = [tuple(map(int, item.split(":"))) for item in args.pairs.split(",")]

    result = {"context": contexts[0]["base"], "firstBuffer": f"0x{base:08x}",
              "secondBuffer": f"0x{other_copy:08x}",
              "bufferSeparation": f"0x{other_copy - base:x}",
              "primaryPlane": [f"0x{primary_start:08x}", f"0x{primary_end:08x}"],
              "samples": []}
    for path in args.references_json:
        refs = read_json(path)
        if refs["pid"] != scan["pid"]:
            parser.error(f"{path}: PID does not match scan JSON")
        addresses = {int(row["address"], 16): int(unit)
                     for unit, rows in refs["references"].items() for row in rows}
        matches = []
        for primary, companion in pairs:
            for address, observed in addresses.items():
                if observed != primary or not primary_start <= address < primary_end:
                    continue
                companion_address = address + args.plane_size
                if addresses.get(companion_address) == companion:
                    matches.append({"primaryUnit": primary, "companionUnit": companion,
                                    "primaryAddress": f"0x{address:08x}",
                                    "companionAddress": f"0x{companion_address:08x}",
                                    "cellOffset": f"0x{address - primary_start:x}"})
        result["samples"].append({"path": str(path), "matches": matches})
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
