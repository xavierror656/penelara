"""Read a Windows RT_STRING entry from a local PE file without running it."""

import argparse
import struct
from pathlib import Path


def read_string(path: Path, string_id: int) -> str:
    data = path.read_bytes()
    pe = struct.unpack_from("<I", data, 0x3C)[0]
    if data[pe:pe + 4] != b"PE\0\0":
        raise ValueError("not a PE file")
    optional = pe + 24
    if struct.unpack_from("<H", data, optional)[0] != 0x10B:
        raise ValueError("expected PE32")
    table = optional + struct.unpack_from("<H", data, pe + 20)[0]
    resource_rva = struct.unpack_from("<I", data, optional + 96 + 16)[0]
    if resource_rva == 0:
        raise ValueError("PE has no resources")
    sections = []
    for index in range(struct.unpack_from("<H", data, pe + 6)[0]):
        entry = table + 40 * index
        virtual_size, rva, raw_size, raw = struct.unpack_from("<IIII", data, entry + 8)
        sections.append((rva, max(virtual_size, raw_size), raw))

    def file_offset(rva: int) -> int:
        for first, size, raw in sections:
            if first <= rva < first + size:
                return raw + rva - first
        raise ValueError(f"RVA 0x{rva:x} is not file-backed")

    root = file_offset(resource_rva)

    def find(directory: int, target: int | None) -> int:
        named, numbered = struct.unpack_from("<HH", data, root + directory + 12)
        for index in range(named + numbered):
            ident, child = struct.unpack_from("<II", data, root + directory + 16 + 8 * index)
            if target is None or ident == target:
                return child
        raise ValueError(f"resource entry {target} is absent")

    block = string_id // 16 + 1
    type_entry = find(0, 6)
    block_entry = find(type_entry & 0x7FFFFFFF, block)
    language_entry = find(block_entry & 0x7FFFFFFF, None)
    if not (type_entry & 0x80000000) or not (block_entry & 0x80000000) \
            or (language_entry & 0x80000000):
        raise ValueError("unexpected RT_STRING directory layout")
    data_rva, size = struct.unpack_from("<II", data, root + language_entry)
    start = file_offset(data_rva)
    end = start + size
    for index in range(16):
        length = struct.unpack_from("<H", data, start)[0]
        start += 2
        byte_length = 2 * length
        if start + byte_length > end:
            raise ValueError("RT_STRING block is truncated")
        if index == string_id % 16:
            return data[start:start + byte_length].decode("utf-16le")
        start += byte_length
    raise AssertionError("unreachable")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("string_id", type=lambda value: int(value, 0))
    args = parser.parse_args()
    print(f"0x{args.string_id:x}: {read_string(args.binary, args.string_id)}")


if __name__ == "__main__":
    main()
