"""Change one teacher's daily-load mode in an XGHC for a native replay."""

import argparse
import gzip
import re
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("destination", type=Path)
    parser.add_argument("--teacher-index", type=int, default=0)
    parser.add_argument("--bound", choices=("max", "min"), required=True)
    parser.add_argument("--mode", required=True)
    parser.add_argument("--only-lectivas", action="store_true",
                        help="set or insert the *SolamenteLectivas mode")
    args = parser.parse_args()

    base_tag = ("considerarMaximasHorasDiarias" if args.bound == "max"
                else "considerarMinimasHorasDiarias").encode("ascii")
    tag = base_tag + (b"SolamenteLectivas" if args.only_lectivas else b"")
    mode = args.mode.encode("ascii")
    if not re.fullmatch(rb"[A-Za-z]+", mode):
        parser.error("mode must be an ASCII identifier")

    payload = gzip.decompress(args.source.read_bytes())
    start = payload.index(b"<?xml")
    end = payload.index(b"</datosGHC>") + len(b"</datosGHC>") + 1
    if start != 7 or payload[0:1] != b"M" or payload[end - 1:end] != b"\n":
        raise ValueError("unexpected XGHC envelope")
    if int.from_bytes(payload[3:7], "little") != end - start:
        raise ValueError("XML length does not match XGHC header")
    xml = payload[start:end]
    teachers = list(re.finditer(rb"<profesor>.*?</profesor>", xml, re.DOTALL))
    if args.teacher_index < 0 or args.teacher_index >= len(teachers):
        raise ValueError("teacher index outside input")
    teacher = teachers[args.teacher_index]
    pattern = rb"(<" + tag + rb">)([^<]+)(</" + tag + rb">)"
    changed, count = re.subn(pattern, lambda m: m[1] + mode + m[3],
                             teacher.group(), count=1)
    if count == 0 and args.only_lectivas:
        anchor = b"</" + base_tag + b">"
        inserted = anchor + b"\n\t\t\t\t<" + tag + b">" + mode + b"</" + tag + b">"
        changed, count = re.subn(re.escape(anchor), lambda _: inserted,
                                 teacher.group(), count=1)
    if count != 1:
        raise ValueError("teacher has no matching daily-load mode")
    result_xml = xml[:teacher.start()] + changed + xml[teacher.end():]
    result = bytearray(payload[:start] + result_xml + payload[end:])
    result[3:7] = len(result_xml).to_bytes(4, "little")
    args.destination.write_bytes(gzip.compress(result, mtime=0))
    if gzip.decompress(args.destination.read_bytes()) != result:
        raise ValueError("roundtrip mismatch")
    print(f"{args.destination}: teacher={args.teacher_index} "
          f"{tag.decode()}={args.mode}; XML bytes={len(result_xml)}")


if __name__ == "__main__":
    main()
