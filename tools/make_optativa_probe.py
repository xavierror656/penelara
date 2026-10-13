"""Build a controlled optativa variant of the shipped DosMarcos project."""

import argparse
import gzip
import xml.etree.ElementTree as ET
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("destination", type=Path)
    parser.add_argument("--ids", default="12,20,22", help="comma-separated session IDs")
    parser.add_argument("--solapable", default="12,22", help="comma-separated marked IDs")
    parser.add_argument("--max-solapadas", type=int, default=2)
    parser.add_argument(
        "--simultaneous-block",
        help="comma-separated optativa member IDs for one simultaneous block",
    )
    args = parser.parse_args()

    payload = gzip.decompress(args.source.read_bytes())
    start = payload.index(b"<?xml")
    close = payload.index(b"</datosGHC>")
    xml_end = close + len(b"</datosGHC>") + 1
    if payload[0:1] != b"M" or start != 7 or payload[xml_end - 1:xml_end] != b"\n":
        raise ValueError("unexpected GHC container framing")
    if int.from_bytes(payload[3:7], "little") != xml_end - start:
        raise ValueError("GHC XML length header mismatch")

    root = ET.fromstring(payload[start:xml_end])
    if root.find("optativas") is not None:
        raise ValueError("source already contains optativas")
    sessions = root.find("sesionesLectivas")
    if sessions is None:
        raise ValueError("source has no sesionesLectivas")
    desired = tuple(int(part) for part in args.ids.split(",") if part)
    marked = {int(part) for part in args.solapable.split(",") if part}
    if not desired or len(desired) != len(set(desired)) or not marked.issubset(desired):
        raise ValueError("IDs must be nonempty, unique, and include every marked ID")
    if not 0 <= args.max_solapadas <= 99:
        raise ValueError("probe maxSolapadas must be between 0 and 99")
    matches = {int(node.attrib["id"]): node for node in sessions if node.attrib.get("id", "").isdigit()}
    for session_id in desired:
        if session_id not in matches or matches[session_id].findtext("grupo") != "3º A":
            raise ValueError(f"session {session_id} is missing or belongs to a different group")

    block = b""
    if args.simultaneous_block:
        block_ids = tuple(int(part) for part in args.simultaneous_block.split(",") if part)
        if len(block_ids) < 2 or len(block_ids) != len(set(block_ids)) or not set(block_ids).issubset(desired):
            raise ValueError("simultaneous block needs at least two unique optativa members")
        block_sessions = [matches[session_id] for session_id in block_ids]
        if any(session.findall("./otrosGrupos/grupo") for session in block_sessions):
            raise ValueError("simultaneous optativa members must not have otrosGrupos")
        if len({session.findtext("duracionSemanal") for session in block_sessions}) != 1:
            raise ValueError("simultaneous block members need equal weekly duration")
        if len({session.findtext("profesor") for session in block_sessions}) != len(block_sessions):
            raise ValueError("simultaneous block members need distinct professors")
        simultaneous = root.find("./listasDeRelacion/simultaneas")
        if simultaneous is None or any(
            node.attrib.get("id") == str(block_ids[0])
            for node in simultaneous.findall("bloqueDeSesiones")
        ):
            raise ValueError("simultaneous block is missing or its ID already exists")
        block = (
            f'\t\t\t<bloqueDeSesiones id="{block_ids[0]}" submarco="A">\n'
            "\t\t\t\t<sesiones>\n"
            + "".join(f"\t\t\t\t\t<sesion>{session_id}</sesion>\n" for session_id in block_ids)
            + "\t\t\t\t</sesiones>\n"
            "\t\t\t\t<plantilla>\n"
            "\t\t\t\t</plantilla>\n"
            "\t\t\t</bloqueDeSesiones>\n"
        ).encode("ascii")

    members = b"".join(
        (f'\t\t\t\t<asignaturaOptativa solapable="{str(session_id in marked).lower()}">'
         f'{session_id}</asignaturaOptativa>\n').encode("ascii")
        for session_id in desired
    )
    prefix = (
        b"\t<optativas>\n"
        b"\t\t<optativa>\n"
        b"\t\t\t<nombre>REA optativa probe</nombre>\n"
        b"\t\t\t<grupo>3\xba A</grupo>\n"
        b"\t\t\t<maxSolapadas>"
        + str(args.max_solapadas).encode("ascii")
        + b"</maxSolapadas>\n"
        + b"\t\t\t<asignaturasOptativas>\n"
    )
    suffix = (
        b"\t\t\t</asignaturasOptativas>\n"
        b"\t\t</optativa>\n"
        b"\t</optativas>\n"
    )
    addition = prefix + members + suffix
    simultaneous_end = payload.index(b"\t\t</simultaneas>") if block else close
    updated = bytearray(
        payload[:simultaneous_end]
        + block
        + payload[simultaneous_end:close]
        + addition
        + payload[close:]
    )
    added_length = len(block) + len(addition)
    ET.fromstring(updated[start:xml_end + added_length])
    updated[3:7] = (xml_end - start + added_length).to_bytes(4, "little")
    compressed = bytearray(gzip.compress(updated, mtime=0))
    compressed[9] = 10
    if args.destination.exists():
        raise FileExistsError(args.destination)
    args.destination.write_bytes(compressed)
    print(
        f"{args.destination}: optativa with session IDs {desired}, "
        f"marked {sorted(marked)}, maxSolapadas={args.max_solapadas}, "
        f"simultaneous block={block_ids if block else None}"
    )


if __name__ == "__main__":
    main()
