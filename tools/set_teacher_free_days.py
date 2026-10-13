"""Change one teacher's requested free-day count in a compressed XGHC."""

import argparse
import gzip
import re
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("destination", type=Path)
    parser.add_argument("--teacher-index", type=int, required=True)
    parser.add_argument("--days", type=int, required=True)
    args = parser.parse_args()
    if args.days < 0 or args.teacher_index < 0:
        parser.error("teacher index and free-day count must be nonnegative")

    payload = gzip.decompress(args.source.read_bytes())
    start = payload.index(b"<?xml")
    end = payload.index(b"</datosGHC>") + len(b"</datosGHC>") + 1
    if (start != 7 or payload[:1] != b"M" or payload[end - 1:end] != b"\n"
            or int.from_bytes(payload[3:7], "little") != end - start):
        raise ValueError("unexpected XGHC envelope")
    xml = payload[start:end]
    teachers = list(re.finditer(rb"<profesor>.*?</profesor>", xml, re.DOTALL))
    if args.teacher_index >= len(teachers):
        parser.error("teacher index outside input")
    teacher = teachers[args.teacher_index]
    updated_teacher, count = re.subn(
        rb"(<periodosLibres>.*?<cantidadDeDias>)(\d+)(</cantidadDeDias>)",
        lambda match: match[1] + str(args.days).encode("ascii") + match[3],
        teacher.group(), count=1, flags=re.DOTALL,
    )
    if count != 1:
        raise ValueError("teacher has no free-day count")
    updated_xml = xml[:teacher.start()] + updated_teacher + xml[teacher.end():]
    result = bytearray(payload[:start] + updated_xml + payload[end:])
    result[3:7] = len(updated_xml).to_bytes(4, "little")
    args.destination.write_bytes(gzip.compress(result, mtime=0))
    if gzip.decompress(args.destination.read_bytes()) != result:
        raise ValueError("roundtrip mismatch")
    print(f"{args.destination}: teacher={args.teacher_index} free_days={args.days}")


if __name__ == "__main__":
    main()
