"""Read bytes at a PE32 virtual address without an external disassembler."""

import argparse
import struct
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("address", type=lambda value: int(value, 0))
    parser.add_argument("count", type=lambda value: int(value, 0))
    args = parser.parse_args()
    if args.count <= 0:
        parser.error("count must be positive")

    data = args.binary.read_bytes()
    pe = struct.unpack_from("<I", data, 0x3C)[0]
    if data[pe:pe + 4] != b"PE\0\0":
        raise ValueError("not a PE image")
    sections = struct.unpack_from("<H", data, pe + 6)[0]
    optional = pe + 24
    if struct.unpack_from("<H", data, optional)[0] != 0x10B:
        raise ValueError("only PE32 images are supported")
    image_base = struct.unpack_from("<I", data, optional + 28)[0]
    rva = args.address - image_base
    section_table = optional + struct.unpack_from("<H", data, pe + 20)[0]
    for index in range(sections):
        entry = section_table + index * 40
        virtual_size, section_rva, raw_size, raw = struct.unpack_from("<IIII", data, entry + 8)
        relative = rva - section_rva
        if 0 <= relative < min(virtual_size, raw_size):
            if relative + args.count > raw_size:
                raise ValueError("requested bytes extend past the section's file data")
            start = raw + relative
            if start + args.count > len(data):
                raise ValueError("section's file data is truncated")
            print(data[start:start + args.count].hex(" "))
            return
    raise ValueError(f"address 0x{args.address:x} is outside file-backed PE sections")


if __name__ == "__main__":
    main()
