"""Change one teacher's computed daily max/min adjustment in an XGHC."""

import argparse
import gzip
import re
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("destination", type=Path)
    parser.add_argument("--teacher-index", type=int, required=True)
    parser.add_argument("--bound", choices=("max", "min"), required=True)
    parser.add_argument("--value", choices=("automatico", "unaHoraMas", "unaHoraMenos"),
                        required=True)
    parser.add_argument("--only-lectivas", action="store_true",
                        help="set or insert the *SolamenteLectivas value")
    args = parser.parse_args()
    if args.teacher_index < 0:
        parser.error("teacher index must be nonnegative")
    base_tag = (b"valorMaximasHorasCalculado" if args.bound == "max"
                else b"valorMinimasHorasCalculado")
    tag = base_tag + (b"SolamenteLectivas" if args.only_lectivas else b"")

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
    pattern = rb"(<" + tag + rb">)[^<]+(</" + tag + rb">)"
    updated_teacher, count = re.subn(
        pattern,
        lambda match: match[1] + args.value.encode("ascii") + match[2],
        teacher.group(), count=1,
    )
    if count == 0 and args.only_lectivas:
        anchor = b"</" + base_tag + b">"
        inserted = (anchor + b"\n\t\t\t\t<" + tag + b">"
                    + args.value.encode("ascii") + b"</" + tag + b">")
        updated_teacher, count = re.subn(
            re.escape(anchor), lambda _: inserted, teacher.group(), count=1,
        )
    if count != 1:
        raise ValueError("teacher has no computed daily-bound value")
    updated_xml = xml[:teacher.start()] + updated_teacher + xml[teacher.end():]
    result = bytearray(payload[:start] + updated_xml + payload[end:])
    result[3:7] = len(updated_xml).to_bytes(4, "little")
    args.destination.write_bytes(gzip.compress(result, mtime=0))
    if gzip.decompress(args.destination.read_bytes()) != result:
        raise ValueError("roundtrip mismatch")
    print(f"{args.destination}: teacher={args.teacher_index} {tag.decode()}={args.value}")


if __name__ == "__main__":
    main()
