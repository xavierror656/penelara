"""Change the single T duration marker to M in a GHC gzip container."""

import argparse
import gzip
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    if args.output.exists():
        raise FileExistsError(args.output)
    original = args.source.read_bytes()
    payload = gzip.decompress(original)
    old = b"<duracion>T</duracion>"
    new = b"<duracion>M</duracion>"
    if payload.count(old) != 1:
        raise ValueError(f"expected exactly one T duration; found {payload.count(old)}")
    changed = payload.replace(old, new)
    compressed = bytearray(gzip.compress(changed, mtime=0))
    compressed[9] = original[9]  # preserve the source gzip OS byte
    args.output.write_bytes(compressed)
    assert gzip.decompress(args.output.read_bytes()) == changed
    print(f"{args.output}: one T→M byte in decompressed payload, {len(changed)} bytes")


if __name__ == "__main__":
    main()
