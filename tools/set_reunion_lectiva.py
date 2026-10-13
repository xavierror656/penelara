"""Toggle one reunion's lectiva flag in a compressed XGHC fixture."""

import argparse
import gzip
import re
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("destination", type=Path)
    parser.add_argument("--index", type=int, default=0)
    parser.add_argument("--value", choices=("true", "false"), required=True)
    args = parser.parse_args()

    payload = gzip.decompress(args.source.read_bytes())
    start = payload.index(b"<?xml")
    end = payload.index(b"</datosGHC>") + len(b"</datosGHC>") + 1
    if (start != 7 or payload[:1] != b"M" or payload[end - 1:end] != b"\n"
            or int.from_bytes(payload[3:7], "little") != end - start):
        raise ValueError("unexpected XGHC envelope")
    xml = payload[start:end]
    meetings = list(re.finditer(rb"<reunion\b[^>]*>.*?</reunion>", xml, re.DOTALL))
    if args.index < 0 or args.index >= len(meetings):
        raise ValueError("meeting index outside input")
    meeting = meetings[args.index]
    edited, count = re.subn(
        rb"(<lectiva>)(true|false)(</lectiva>)",
        lambda match: match[1] + args.value.encode() + match[3],
        meeting.group(), count=1,
    )
    if count != 1:
        raise ValueError("meeting has no lectiva flag")
    updated_xml = xml[:meeting.start()] + edited + xml[meeting.end():]
    result = bytearray(payload[:start] + updated_xml + payload[end:])
    result[3:7] = len(updated_xml).to_bytes(4, "little")
    args.destination.write_bytes(gzip.compress(result, mtime=0))
    if gzip.decompress(args.destination.read_bytes()) != result:
        raise ValueError("roundtrip mismatch")
    print(f"{args.destination}: reunion={args.index} lectiva={args.value}")


if __name__ == "__main__":
    main()
