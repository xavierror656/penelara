"""Compare binary CArchive trailers of two GHC containers."""

import argparse
import gzip
import re
from pathlib import Path


def trailer(path: Path) -> bytes:
    payload = gzip.decompress(path.read_bytes())
    end = payload.index(b"</datosGHC>") + len(b"</datosGHC>") + 1
    return payload[end:]


def text_runs(data: bytes, wide: bool = False) -> list[tuple[int, str]]:
    pattern = rb"(?:[\x20-\x7e]\x00){4,}" if wide else rb"[\x20-\x7e]{4,}"
    return [(match.start(), match.group().decode("utf-16le" if wide else "latin-1"))
            for match in re.finditer(pattern, data)]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("first", type=Path)
    parser.add_argument("second", type=Path)
    parser.add_argument("--range", action="append", dest="ranges", default=[],
                        help="decimal START:END byte range to hex dump from each trailer")
    parser.add_argument("--suffix-words", type=int, default=0,
                        help="show N trailing bytes as little-endian DWORDs")
    args = parser.parse_args()
    first, second = trailer(args.first), trailer(args.second)
    first_after_path = first[5 + first[4]:]
    second_after_path = second[5 + second[4]:]
    differing_after_path = [i for i, (a, b) in enumerate(zip(first_after_path, second_after_path)) if a != b]
    print(f"after_path_lengths={len(first_after_path)},{len(second_after_path)} byte_differences={len(differing_after_path)} first_offsets={differing_after_path[:40]}")
    prefix = next((i for i, (a, b) in enumerate(zip(first, second)) if a != b), min(len(first), len(second)))
    suffix = 0
    while suffix < min(len(first), len(second)) and first[-1 - suffix] == second[-1 - suffix]:
        suffix += 1
    print(f"lengths={len(first)},{len(second)} common_prefix={prefix} common_suffix={suffix}")
    for label, data in (("first", first), ("second", second)):
        declared = int.from_bytes(data[:4], "little")
        path_len = data[4]
        path = data[5:5 + path_len].decode("latin-1", errors="replace")
        print(f"{label} declared_after_length={declared} actual_after_length={len(data) - 4} path_len={path_len} path={path!r} next={data[5 + path_len:5 + path_len + 12].hex(' ')}")
        print(f"{label} head={data[:96].hex(' ')}")
        print(f"{label} tail={data[-96:].hex(' ')}")
        print(f"{label} ascii={text_runs(data)[:30]}")
        print(f"{label} utf16={text_runs(data, wide=True)[:30]}")
        if args.suffix_words:
            suffix_start = len(data) - args.suffix_words
            if args.suffix_words % 4 or suffix_start < 0:
                raise ValueError("suffix length must be a positive multiple of four")
            for offset in range(suffix_start, len(data), 4):
                print(f"{label} suffix+{offset - suffix_start:03d} absolute={offset:04x} u32={int.from_bytes(data[offset:offset + 4], 'little')} hex={data[offset:offset + 4].hex(' ')}")
        for raw_range in args.ranges:
            start, end = (int(value) for value in raw_range.split(":"))
            for offset in range(start, min(end, len(data)), 16):
                chunk = data[offset:min(offset + 16, end)]
                ascii_view = "".join(chr(value) if 32 <= value <= 126 else "." for value in chunk)
                print(f"{label} {offset:04x}: {chunk.hex(' '):47} {ascii_view}")


if __name__ == "__main__":
    main()
