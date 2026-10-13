"""Summarize MotHor's 0x288-byte entry array captured at 0x46113e."""

import argparse
import json
import struct
from collections import Counter, defaultdict
from pathlib import Path


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("snapshot", type=Path)
    args = parser.parse_args()
    data = args.snapshot.read_bytes()
    stride = 0x288
    if len(data) % stride:
        raise ValueError(f"snapshot size {len(data)} is not a multiple of 0x288")
    entries = [data[offset:offset + stride] for offset in range(0, len(data), stride)]
    types = Counter(entry[0x271] for entry in entries)
    status = Counter(entry[0x26E] for entry in entries)
    type3_by_source = defaultdict(lambda: Counter())
    pending = []
    for index, entry in enumerate(entries):
        source_index = struct.unpack_from("<h", entry, 0x264)[0]
        if entry[0x271] == 3:
            type3_by_source[source_index][entry[0x26E]] += 1
        if entry[0x26E] == 1:
            pending.append({
                "index": index,
                "id": struct.unpack_from("<h", entry)[0],
                "recordId": struct.unpack_from("<h", entry, 2)[0],
                "sourceIndex": source_index,
                "type271": entry[0x271],
            })
    print(json.dumps({
        "entryCount": len(entries),
        "type271": dict(sorted(types.items())),
        "status26e": dict(sorted(status.items())),
        "type3BySourceAndStatus": {
            str(source): dict(sorted(counts.items()))
            for source, counts in sorted(type3_by_source.items())
        },
        "pending": pending,
    }, indent=2))


if __name__ == "__main__":
    main()
