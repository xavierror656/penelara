"""Wrap a modern XRHO XML with the native app header and trailer for a paired fixture."""

import argparse
import gzip
from pathlib import Path


def parts(path: Path) -> tuple[bytes, bytes, bytes, bytes]:
    original = path.read_bytes()
    payload = gzip.decompress(original)
    start = payload.index(b"<?xml")
    end = payload.index(b"</datosGHC>") + len(b"</datosGHC>") + 1
    if start != 7 or int.from_bytes(payload[3:7], "little") != end - start:
        raise ValueError(f"unexpected GHC container: {path}")
    return original[:10], payload[:7], payload[start:end], payload[end:]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("modern", type=Path)
    parser.add_argument("native", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--header", choices=("modern", "native"), default="native")
    parser.add_argument("--trailer", choices=("modern", "native"), default="native")
    args = parser.parse_args()
    if args.output.exists():
        raise FileExistsError(args.output)
    gzip_header, modern_header, xml, modern_trailer = parts(args.modern)
    _, native_header, _, native_trailer = parts(args.native)
    selected_header = native_header if args.header == "native" else modern_header
    selected_trailer = native_trailer if args.trailer == "native" else modern_trailer
    header = selected_header[:3] + len(xml).to_bytes(4, "little")
    payload = header + xml + selected_trailer
    compressed = bytearray(gzip.compress(payload, mtime=0))
    compressed[9] = gzip_header[9]
    args.output.write_bytes(compressed)
    if gzip.decompress(args.output.read_bytes()) != payload:
        raise ValueError("round-trip mismatch")
    print(f"{args.output}: {args.header} header, {args.trailer} trailer, modern XML")


if __name__ == "__main__":
    main()
