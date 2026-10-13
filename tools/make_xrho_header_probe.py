"""Change only the application format byte of a modern XRHO to the native value."""

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
    payload = bytearray(gzip.decompress(original))
    if len(payload) < 7 or payload[0:2] != b"M\x06":
        raise ValueError("expected an M/6 application header")
    payload[1] = 7
    compressed = bytearray(gzip.compress(payload, mtime=0))
    compressed[9] = original[9]
    args.output.write_bytes(compressed)
    if gzip.decompress(args.output.read_bytes()) != payload:
        raise ValueError("round-trip mismatch")
    print(f"{args.output}: application header M/6→M/7; XML and trailer unchanged")


if __name__ == "__main__":
    main()
