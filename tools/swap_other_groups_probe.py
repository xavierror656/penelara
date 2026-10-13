"""Swap DosMarcos sessions 38 and 42 group fields for controlled probes."""

import argparse
import gzip
import xml.etree.ElementTree as ET
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("destination", type=Path)
    parser.add_argument("--mode", choices=("other-groups", "group-matter", "both"), default="other-groups")
    args = parser.parse_args()
    if args.destination.exists():
        raise FileExistsError(args.destination)
    payload = gzip.decompress(args.source.read_bytes())
    xml_start = payload.index(b"<?xml")
    xml_end = payload.index(b"</datosGHC>") + len(b"</datosGHC>") + 1
    if payload[:1] != b"M" or xml_start != 7 or int.from_bytes(payload[3:7], "little") != xml_end - xml_start:
        raise ValueError("unexpected GHC container")

    updated = payload
    if args.mode in ("other-groups", "both"):
        start38 = updated.index(b'<sesion id="38">')
        start40 = updated.index(b'<sesion id="40">', start38)
        start42 = updated.index(b'<sesion id="42">', start40)
        start43 = updated.index(b'<sesion id="43">', start42)
        other_start = updated.index(b"\t\t\t<otrosGrupos>\n", start38, start40)
        other_end = updated.index(b"\t\t\t</otrosGrupos>\n", other_start, start40) + len(b"\t\t\t</otrosGrupos>\n")
        block = updated[other_start:other_end]
        if b"<grupo>3\xba B</grupo>" not in block or b"<otrosGrupos>" in updated[start42:start43]:
            raise ValueError("unexpected session group structure")
        insertion = updated.index(b"\t\t\t<enDistintoDia>", start42, start43)
        updated = updated[:other_start] + updated[other_end:insertion] + block + updated[insertion:]
    if args.mode in ("group-matter", "both"):
        shared = b'<grupoMateria tipo="N">TECNOLOG/3\xba A/3\xba B</grupoMateria>'
        exclusive = b'<grupoMateria tipo="N">TECNOLOG/3\xba A</grupoMateria>'
        marker = b'<grupoMateria tipo="N">SWAP-PROBE-MARKER</grupoMateria>'
        if updated.count(shared) != 1 or updated.count(exclusive) != 1:
            raise ValueError("unexpected grupoMateria structure")
        updated = updated.replace(shared, marker).replace(exclusive, shared).replace(marker, exclusive)
    root = ET.fromstring(updated[xml_start:xml_end])
    sessions = {node.attrib["id"]: node for node in root.find("sesionesLectivas")}
    if args.mode in ("other-groups", "both"):
        if sessions["38"].find("otrosGrupos") is not None or [g.text for g in sessions["42"].find("otrosGrupos")] != ["3º B"]:
            raise ValueError("probe validation failed: otrosGrupos")
    if args.mode in ("group-matter", "both"):
        if sessions["38"].findtext("grupoMateria") != "TECNOLOG/3º A" or sessions["42"].findtext("grupoMateria") != "TECNOLOG/3º A/3º B":
            raise ValueError("probe validation failed: grupoMateria")
    compressed = bytearray(gzip.compress(updated, mtime=0))
    compressed[9] = 10
    args.destination.write_bytes(compressed)
    print(f"{args.destination}: mode={args.mode}; XML length unchanged")


if __name__ == "__main__":
    main()
