"""Forbid every period of one teacher's day in a compressed XGHC fixture."""

import argparse
import gzip
import re
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("destination", type=Path)
    parser.add_argument("--teacher-index", type=int, required=True)
    parser.add_argument("--frame", default="A")
    parser.add_argument("--day", type=int, required=True)
    parser.add_argument("--periods", type=int, required=True)
    args = parser.parse_args()
    if args.day < 0 or args.periods < 1 or not re.fullmatch(r"[A-Z]", args.frame):
        parser.error("day, periods or frame is invalid")

    payload = gzip.decompress(args.source.read_bytes())
    start = payload.index(b"<?xml")
    end = payload.index(b"</datosGHC>") + len(b"</datosGHC>") + 1
    if (start != 7 or payload[:1] != b"M" or payload[end - 1:end] != b"\n"
            or int.from_bytes(payload[3:7], "little") != end - start):
        raise ValueError("unexpected XGHC envelope")
    xml = payload[start:end]
    teachers = list(re.finditer(rb"<profesor>.*?</profesor>", xml, re.DOTALL))
    if not 0 <= args.teacher_index < len(teachers):
        parser.error("teacher index outside input")
    teacher = teachers[args.teacher_index]
    template = re.search(rb"<plantilla>.*?</plantilla>", teacher.group(), re.DOTALL)
    if template is None:
        raise ValueError("teacher has no availability template")
    content = template.group()
    day = str(args.day).encode("ascii")
    frame = args.frame.encode("ascii")
    seen = set()
    pattern = re.compile(rb'<tramo dia="' + day + rb'" indice="(\d+)" marco="'
                         + frame + rb'">[^<]*</tramo>')

    def replace(match: re.Match[bytes]) -> bytes:
        period = int(match[1])
        if period >= args.periods or period in seen:
            raise ValueError("unexpected or duplicate period in source template")
        seen.add(period)
        return match.group().split(b">", 1)[0] + b">prohibido</tramo>"

    content = pattern.sub(replace, content)
    newline = b"\r\n" if b"\r\n" in content else b"\n"
    missing = b"".join(
        b"\t\t\t\t<tramo dia=\"" + day + b"\" indice=\""
        + str(index).encode("ascii") + b"\" marco=\"" + frame
        + b"\">prohibido</tramo>" + newline
        for index in range(args.periods) if index not in seen
    )
    content = content.replace(b"\t\t\t</plantilla>", missing + b"\t\t\t</plantilla>", 1)
    edited_teacher = teacher.group()[:template.start()] + content + teacher.group()[template.end():]
    updated_xml = xml[:teacher.start()] + edited_teacher + xml[teacher.end():]
    result = bytearray(payload[:start] + updated_xml + payload[end:])
    result[3:7] = len(updated_xml).to_bytes(4, "little")
    args.destination.write_bytes(gzip.compress(result, mtime=0))
    if gzip.decompress(args.destination.read_bytes()) != result:
        raise ValueError("roundtrip mismatch")
    print(f"{args.destination}: teacher={args.teacher_index} "
          f"frame={args.frame} day={args.day} periods={args.periods}")


if __name__ == "__main__":
    main()
