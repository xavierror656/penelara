"""Mutate one group field of DosMarcos session 12 for a controlled probe."""

import argparse
import gzip
import xml.etree.ElementTree as ET
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("destination", type=Path)
    parser.add_argument("--field", choices=("otrosGrupos", "grupoMateria"), default="otrosGrupos")
    args = parser.parse_args()
    if args.destination.exists():
        raise FileExistsError(args.destination)
    payload = gzip.decompress(args.source.read_bytes())
    xml_start = payload.index(b"<?xml")
    xml_end = payload.index(b"</datosGHC>") + len(b"</datosGHC>") + 1
    if payload[:1] != b"M" or xml_start != 7 or int.from_bytes(payload[3:7], "little") != xml_end - xml_start:
        raise ValueError("unexpected GHC container")
    start12 = payload.index(b'<sesion id="12">')
    start13 = payload.index(b'<sesion id="13">', start12)
    if b"<otrosGrupos>" in payload[start12:start13]:
        raise ValueError("session 12 already has other groups")
    if args.field == "otrosGrupos":
        insertion = payload.index(b"\t\t</sesion>\n", start12, start13)
        block = b"\t\t\t<otrosGrupos>\n\t\t\t\t<grupo>3\xba B</grupo>\n\t\t\t</otrosGrupos>\n"
        updated = bytearray(payload[:insertion] + block + payload[insertion:])
        change = len(block)
    else:
        original = b"<grupoMateria tipo=\"N\">ED.FIS./3\xba A</grupoMateria>"
        replacement = b"<grupoMateria tipo=\"N\">ED.FIS./3\xba A/3\xba B</grupoMateria>"
        if payload[start12:start13].count(original) != 1:
            raise ValueError("unexpected grupoMateria for session 12")
        updated = bytearray(payload[:start12] + payload[start12:start13].replace(original, replacement) + payload[start13:])
        change = len(replacement) - len(original)
    updated[3:7] = (xml_end - xml_start + change).to_bytes(4, "little")
    root = ET.fromstring(updated[xml_start:xml_end + change])
    sessions = {node.attrib["id"]: node for node in root.find("sesionesLectivas")}
    if args.field == "otrosGrupos":
        if [g.text for g in sessions["12"].find("otrosGrupos")] != ["3º B"]:
            raise ValueError("probe validation failed")
    elif sessions["12"].findtext("grupoMateria") != "ED.FIS./3º A/3º B":
        raise ValueError("probe validation failed")
    compressed = bytearray(gzip.compress(updated, mtime=0))
    compressed[9] = 10
    args.destination.write_bytes(compressed)
    print(f"{args.destination}: changed {args.field} for session 12, preserving other fields")


if __name__ == "__main__":
    main()
