"""Move one complementary duty to another frame without changing its XGHC XML."""

import argparse
import gzip
import xml.etree.ElementTree as ET
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("destination", type=Path)
    parser.add_argument("--from-frame", default="A")
    parser.add_argument("--to-frame", default="B")
    args = parser.parse_args()
    if len(args.from_frame) != 1 or len(args.to_frame) != 1:
        raise ValueError("frame identifiers must be one byte each")
    payload = gzip.decompress(args.source.read_bytes())
    start = payload.index(b"<?xml")
    end = payload.index(b"</datosGHC>") + len(b"</datosGHC>") + 1
    if (start != 7 or payload[0] != ord("M") or
            int.from_bytes(payload[3:7], "little") != end - start):
        raise ValueError("unexpected GHC XML container")
    xml = payload[start:end]
    before = f'<complementaria subMarco="{args.from_frame}">'.encode("ascii")
    after = f'<complementaria subMarco="{args.to_frame}">'.encode("ascii")
    if xml.count(before) != 1:
        raise ValueError("expected exactly one complementary duty in source frame")
    changed = xml.replace(before, after)
    root = ET.fromstring(changed)
    duties = root.findall("complementarias/complementaria")
    if len(duties) != 1 or duties[0].get("subMarco") != args.to_frame:
        raise ValueError("frame replacement failed")
    updated = payload[:start] + changed + payload[end:]
    if len(updated) != len(payload) or sum(a != b for a, b in zip(payload, updated)) != 1:
        raise ValueError("the decompressed payload must differ by exactly one byte")
    compressed = bytearray(gzip.compress(updated, mtime=0))
    compressed[9] = 10
    args.destination.write_bytes(compressed)
    print(f"{args.destination}: {args.from_frame}->{args.to_frame}, one payload byte changed")


if __name__ == "__main__":
    main()
