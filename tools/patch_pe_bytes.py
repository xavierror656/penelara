"""Replace verified PE32 bytes in a separate output file."""

import argparse
import struct
from pathlib import Path


def file_offset(data: bytes, address: int) -> int:
    pe = struct.unpack_from("<I", data, 0x3C)[0]
    if data[pe:pe + 4] != b"PE\0\0":
        raise ValueError("not a PE image")
    count = struct.unpack_from("<H", data, pe + 6)[0]
    optional = pe + 24
    if struct.unpack_from("<H", data, optional)[0] != 0x10B:
        raise ValueError("expected PE32")
    image_base = struct.unpack_from("<I", data, optional + 28)[0]
    table = optional + struct.unpack_from("<H", data, pe + 20)[0]
    for index in range(count):
        entry = table + index * 40
        _, rva, raw_size, raw = struct.unpack_from("<IIII", data, entry + 8)
        relative = address - image_base - rva
        if 0 <= relative < raw_size:
            return raw + relative
    raise ValueError(f"0x{address:x} is not file-backed")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("destination", type=Path)
    parser.add_argument("address", type=lambda value: int(value, 0))
    parser.add_argument("expected_hex")
    parser.add_argument("replacement_hex")
    args = parser.parse_args()
    if args.source.resolve() == args.destination.resolve() or args.destination.exists():
        raise ValueError("destination must be a new, separate file")
    expected = bytes.fromhex(args.expected_hex)
    replacement = bytes.fromhex(args.replacement_hex)
    if not expected or len(expected) != len(replacement):
        raise ValueError("replacement must have the same nonzero length as expected bytes")
    data = bytearray(args.source.read_bytes())
    offset = file_offset(data, args.address)
    actual = data[offset:offset + len(expected)]
    if actual != expected:
        raise ValueError(f"unexpected bytes at 0x{args.address:x}: {actual.hex()}")
    data[offset:offset + len(expected)] = replacement
    args.destination.write_bytes(data)
    print(f"patched 0x{args.address:08x}: {expected.hex()} -> {replacement.hex()}")


if __name__ == "__main__":
    main()
