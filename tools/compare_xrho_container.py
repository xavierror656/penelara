"""Summarize GHC containers and XML structure for native/modern comparison."""

import argparse
import gzip
import hashlib
import json
import xml.etree.ElementTree as ET
from pathlib import Path


def summarize(path: Path) -> dict:
    original = path.read_bytes()
    payload = gzip.decompress(original)
    start = payload.index(b"<?xml")
    end = payload.index(b"</datosGHC>") + len(b"</datosGHC>") + 1
    root = ET.fromstring(payload[start:end])
    trailer = payload[end:]
    return {
        "path": str(path),
        "compressed_bytes": len(original),
        "payload_bytes": len(payload),
        "gzip_header": original[:10].hex(),
        "app_header": payload[:start].hex(),
        "xml_bytes": end - start,
        "declared_xml_bytes": int.from_bytes(payload[3:7], "little"),
        "xml_declaration": payload[start:payload.find(b"?>", start) + 2].decode("latin-1"),
        "trailer_bytes": len(trailer),
        "trailer_sha256": hashlib.sha256(trailer).hexdigest(),
        "children": [child.tag for child in root],
        "horario_tramos": len(root.findall("horario/tramo")),
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("paths", nargs="+", type=Path)
    args = parser.parse_args()
    print(json.dumps([summarize(path) for path in args.paths], ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
