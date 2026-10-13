"""Probe whether an XGHC trailer plus native result suffix opens as XRHO."""

import argparse
import gzip
from pathlib import Path

from make_xrho_envelope_probe import parts


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("modern", type=Path)
    parser.add_argument("native", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--suffix-bytes", type=int, default=108)
    parser.add_argument("--zero-suffix", action="store_true")
    args = parser.parse_args()
    if args.output.exists():
        raise FileExistsError(args.output)
    gzip_header, _, xml, source_trailer = parts(args.modern)
    _, native_header, _, native_trailer = parts(args.native)
    if args.suffix_bytes <= 0 or args.suffix_bytes >= len(native_trailer):
        raise ValueError("invalid suffix length")
    suffix = bytes(args.suffix_bytes) if args.zero_suffix else native_trailer[-args.suffix_bytes:]
    trailer = bytearray(source_trailer + suffix)
    trailer[:4] = (len(trailer) - 4).to_bytes(4, "little")
    payload = native_header[:3] + len(xml).to_bytes(4, "little") + xml + trailer
    compressed = bytearray(gzip.compress(payload, mtime=0))
    compressed[9] = gzip_header[9]
    args.output.write_bytes(compressed)
    if gzip.decompress(args.output.read_bytes()) != payload:
        raise ValueError("round-trip mismatch")
    print(f"{args.output}: source trailer {len(source_trailer)} + {'zero' if args.zero_suffix else 'native'} suffix {args.suffix_bytes} bytes")


if __name__ == "__main__":
    main()
