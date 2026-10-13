"""Add one availability marker to the sole complementary duty of an XGHC."""

import argparse
import gzip
import re
import xml.etree.ElementTree as ET
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("destination", type=Path)
    parser.add_argument("--frame", default="B")
    parser.add_argument("--day", type=int, default=0)
    parser.add_argument("--index", type=int, default=7)
    parser.add_argument("--value", default="fijado")
    args = parser.parse_args()
    payload = gzip.decompress(args.source.read_bytes())
    start = payload.index(b"<?xml")
    end = payload.index(b"</datosGHC>") + len(b"</datosGHC>") + 1
    if (start != 7 or payload[0] != ord("M") or
            int.from_bytes(payload[3:7], "little") != end - start):
        raise ValueError("unexpected GHC XML container")
    xml = payload[start:end]
    match = re.search(rb'<complementaria subMarco="([^"]+)">(.*?)</complementaria>',
                      xml, re.DOTALL)
    if match is None or len(re.findall(rb'<complementaria subMarco=', xml)) != 1:
        raise ValueError("expected exactly one complementary duty")
    if match.group(1).decode("ascii") != args.frame:
        raise ValueError("complementary duty belongs to a different frame")
    before = b"<plantilla>\n\t\t\t\t</plantilla>"
    if match.group(0).count(before) != 1:
        raise ValueError("expected an empty complementary duty template")
    slot = (f'<tramo dia="{args.day}" indice="{args.index}" '
            f'marco="{args.frame}">{args.value}</tramo>').encode("utf-8")
    after = b"<plantilla>\n\t\t\t\t" + slot + b"\n\t\t\t\t</plantilla>"
    changed_duty = match.group(0).replace(before, after)
    xml = xml[:match.start()] + changed_duty + xml[match.end():]
    root = ET.fromstring(xml)
    entries = root.findall("complementarias/complementaria/plantilla/tramo")
    if (len(entries) != 1 or entries[0].get("dia") != str(args.day) or
            entries[0].get("indice") != str(args.index) or
            entries[0].get("marco") != args.frame or
            entries[0].text != args.value):
        raise ValueError("template replacement failed")
    updated = bytearray(payload[:start] + xml + payload[end:])
    updated[3:7] = len(xml).to_bytes(4, "little")
    compressed = bytearray(gzip.compress(updated, mtime=0))
    compressed[9] = 10
    args.destination.write_bytes(compressed)
    print(f"{args.destination}: {args.frame}/{args.day}/{args.index}={args.value}")


if __name__ == "__main__":
    main()
