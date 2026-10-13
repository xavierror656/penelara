"""Add one T marker to an ordinary slot of an M duration fixture."""

import argparse
import gzip
import re
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--t-only-output", type=Path)
    parser.add_argument("--frame", default="A")
    parser.add_argument("--day", type=int, default=0)
    parser.add_argument("--index", type=int, default=0)
    args = parser.parse_args()

    original = args.source.read_bytes()
    payload = gzip.decompress(original)
    if payload.count(b"<duracion>M</duracion>") != 1 or b"<duracion>T</duracion>" in payload:
        raise ValueError("expected exactly one M marker and no T markers")

    if not re.fullmatch(r"[A-Za-z0-9_-]+", args.frame):
        raise ValueError("invalid frame id")
    pattern = re.compile(
        rb"<tramo>(?:(?!</tramo>).)*?<submarco>" + args.frame.encode("ascii") + rb"</submarco>"
        + rb"(?:(?!</tramo>).)*?<dia>" + str(args.day).encode("ascii") + rb"</dia>"
        + rb"(?:(?!</tramo>).)*?<indice>" + str(args.index).encode("ascii") + rb"</indice>"
        + rb"(?:(?!</tramo>).)*?</tramo>",
        re.DOTALL,
    )
    matches = list(pattern.finditer(payload))
    if len(matches) != 1:
        raise ValueError(f"expected one {args.frame}/{args.day}/{args.index} frame slot, found {len(matches)}")
    slot = matches[0].group()
    old = b"<duracion>1</duracion>"
    if slot.count(old) != 1 or b"<Tipo>lectivo</Tipo>" not in slot:
        raise ValueError("target must be an ordinary teaching slot")
    changed = payload[: matches[0].start()] + slot.replace(old, b"<duracion>T</duracion>") + payload[matches[0].end() :]
    differences = [index for index, (a, b) in enumerate(zip(payload, changed)) if a != b]
    if len(payload) != len(changed) or len(differences) != 1:
        raise ValueError("expected a single byte change")

    compressed = bytearray(gzip.compress(changed, mtime=0))
    compressed[9] = original[9]
    if args.output.exists():
        if gzip.decompress(args.output.read_bytes()) != changed:
            raise FileExistsError(args.output)
    else:
        args.output.write_bytes(compressed)
    if gzip.decompress(args.output.read_bytes()) != changed:
        raise ValueError("round-trip mismatch")
    print(f"{args.output}: {args.frame}/{args.day}/{args.index} duration 1→T at payload byte {differences[0]}")

    if args.t_only_output is not None:
        if args.t_only_output.exists():
            raise FileExistsError(args.t_only_output)
        t_only = changed.replace(b"<duracion>M</duracion>", b"<duracion>1</duracion>")
        t_compressed = bytearray(gzip.compress(t_only, mtime=0))
        t_compressed[9] = original[9]
        args.t_only_output.write_bytes(t_compressed)
        if gzip.decompress(args.t_only_output.read_bytes()) != t_only:
            raise ValueError("T-only round-trip mismatch")
        print(f"{args.t_only_output}: mixed T/M→T/1 with one byte changed")


if __name__ == "__main__":
    main()
