"""Reassign one teacher's lecture sessions in a compressed XGHC fixture."""

import argparse
import gzip
import re
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("destination", type=Path)
    parser.add_argument("--from-teacher", required=True)
    parser.add_argument("--to-teacher", required=True)
    parser.add_argument("--expect", type=int, required=True)
    args = parser.parse_args()
    if args.from_teacher == args.to_teacher or args.expect < 1:
        parser.error("teachers must differ and --expect must be positive")

    payload = gzip.decompress(args.source.read_bytes())
    start = payload.index(b"<?xml")
    end = payload.index(b"</datosGHC>") + len(b"</datosGHC>") + 1
    if (start != 7 or payload[:1] != b"M" or payload[end - 1:end] != b"\n"
            or int.from_bytes(payload[3:7], "little") != end - start):
        raise ValueError("unexpected XGHC envelope")
    xml = payload[start:end]
    src = b"<profesor>" + args.from_teacher.encode("utf-8") + b"</profesor>"
    dst = b"<profesor>" + args.to_teacher.encode("utf-8") + b"</profesor>"
    count = 0

    def replace_session(match: re.Match[bytes]) -> bytes:
        nonlocal count
        session = match.group()
        if src not in session:
            return session
        if session.count(src) != 1:
            raise ValueError("session has repeated teacher fields")
        count += 1
        return session.replace(src, dst, 1)

    updated_xml = re.sub(rb"<sesion\b[^>]*>.*?</sesion>", replace_session, xml, flags=re.DOTALL)
    if count != args.expect:
        raise ValueError(f"expected {args.expect} sessions, found {count}")
    result = bytearray(payload[:start] + updated_xml + payload[end:])
    result[3:7] = len(updated_xml).to_bytes(4, "little")
    args.destination.write_bytes(gzip.compress(result, mtime=0))
    if gzip.decompress(args.destination.read_bytes()) != result:
        raise ValueError("roundtrip mismatch")
    print(f"{args.destination}: {args.from_teacher} -> {args.to_teacher}, sessions={count}")


if __name__ == "__main__":
    main()
