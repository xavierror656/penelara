"""Change only colocarSesionesLectivas/optativaSolapada in a GHC fixture."""

import argparse
import gzip
import xml.etree.ElementTree as ET
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("destination", type=Path)
    parser.add_argument("value", type=int, nargs="?")
    parser.add_argument("--omit", action="store_true")
    args = parser.parse_args()
    if args.destination.exists():
        raise FileExistsError(args.destination)
    if args.omit == (args.value is not None):
        raise ValueError("provide either a value or --omit")
    if args.value is not None and not 0 <= args.value <= 99:
        raise ValueError("probe value must be in 0..99")
    payload = gzip.decompress(args.source.read_bytes())
    xml_start = payload.index(b"<?xml")
    xml_end = payload.index(b"</datosGHC>") + len(b"</datosGHC>") + 1
    if payload[:1] != b"M" or xml_start != 7 or int.from_bytes(payload[3:7], "little") != xml_end - xml_start:
        raise ValueError("unexpected GHC container")
    original = b"<optativaSolapada>1</optativaSolapada>"
    replacement = b"" if args.omit else f"<optativaSolapada>{args.value}</optativaSolapada>".encode("ascii")
    if payload[xml_start:xml_end].count(original) != 1:
        raise ValueError("expected one default optativaSolapada=1")
    updated = bytearray(payload.replace(original, replacement, 1))
    change = len(replacement) - len(original)
    updated[3:7] = (xml_end - xml_start + change).to_bytes(4, "little")
    root = ET.fromstring(updated[xml_start:xml_end + change])
    values = [node.text for node in root.iter("optativaSolapada")]
    if values != ([] if args.omit else [str(args.value)]):
        raise ValueError("probe validation failed")
    compressed = bytearray(gzip.compress(updated, mtime=0))
    compressed[9] = 10
    args.destination.write_bytes(compressed)
    print(f"{args.destination}: optativaSolapada={'omitted' if args.omit else args.value}")


if __name__ == "__main__":
    main()
