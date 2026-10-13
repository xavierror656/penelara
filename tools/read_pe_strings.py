"""Read null-terminated ASCII or UTF-16LE strings at PE32 virtual addresses."""

import argparse
import struct
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("addresses", nargs="+", type=lambda value: int(value, 0))
    parser.add_argument("--utf16", action="store_true")
    args = parser.parse_args()
    data = args.binary.read_bytes()
    pe = struct.unpack_from("<I", data, 0x3C)[0]
    if data[pe:pe + 4] != b"PE\0\0":
        raise ValueError("not a PE image")
    count = struct.unpack_from("<H", data, pe + 6)[0]
    optional = pe + 24
    if struct.unpack_from("<H", data, optional)[0] != 0x10B:
        raise ValueError("expected PE32")
    image_base = struct.unpack_from("<I", data, optional + 28)[0]
    table = optional + struct.unpack_from("<H", data, pe + 20)[0]
    sections = []
    for i in range(count):
        entry = table + i * 40
        virtual_size, rva, raw_size, raw = struct.unpack_from("<IIII", data, entry + 8)
        sections.append((rva, min(virtual_size, raw_size), raw))

    for address in args.addresses:
        offset = None
        for rva, size, raw in sections:
            relative = address - image_base - rva
            if 0 <= relative < size:
                offset = raw + relative
                break
        if offset is None:
            raise ValueError(f"0x{address:x} is not file-backed")
        if args.utf16:
            chars = bytearray()
            while data[offset:offset + 2] != b"\0\0" and len(chars) < 512:
                chars += data[offset:offset + 2]
                offset += 2
            value = chars.decode("utf-16le", "replace")
        else:
            value = data[offset:offset + 512].split(b"\0", 1)[0].decode("latin1")
        print(f"0x{address:08x}: {value!r}")


if __name__ == "__main__":
    main()
