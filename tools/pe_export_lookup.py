"""Resolve an exported ordinal in a PE DLL when a name is available."""

import argparse
import struct
from pathlib import Path


parser = argparse.ArgumentParser()
parser.add_argument("binary", type=Path)
parser.add_argument("ordinal", type=int)
args = parser.parse_args()
data = args.binary.read_bytes()
pe = struct.unpack_from("<I", data, 0x3C)[0]
sections_count = struct.unpack_from("<H", data, pe + 6)[0]
optional_size = struct.unpack_from("<H", data, pe + 20)[0]
optional = pe + 24
section_table = optional + optional_size
sections = []
for i in range(sections_count):
    entry = section_table + i * 40
    size, rva, raw_size, raw = struct.unpack_from("<IIII", data, entry + 8)
    sections.append((rva, max(size, raw_size), raw))


def file_offset(rva):
    for start, size, raw in sections:
        if start <= rva < start + size:
            return raw + rva - start
    raise ValueError(f"RVA 0x{rva:x} is outside sections")


def c_string(rva):
    offset = file_offset(rva)
    return data[offset : data.index(b"\0", offset)].decode("ascii")


magic = struct.unpack_from("<H", data, optional)[0]
directory = optional + (96 if magic == 0x10B else 112)
export_rva = struct.unpack_from("<I", data, directory)[0]
export = file_offset(export_rva)
base, function_count, name_count, functions_rva, names_rva, ordinals_rva = struct.unpack_from(
    "<IIIIII", data, export + 16
)
index = args.ordinal - base
if not 0 <= index < function_count:
    raise SystemExit(f"ordinal {args.ordinal} outside export range")
target_rva = struct.unpack_from("<I", data, file_offset(functions_rva + index * 4))[0]
name = None
for i in range(name_count):
    name_index = struct.unpack_from("<H", data, file_offset(ordinals_rva + i * 2))[0]
    if name_index == index:
        name_rva = struct.unpack_from("<I", data, file_offset(names_rva + i * 4))[0]
        name = c_string(name_rva)
        break
print(f"ordinal {args.ordinal}: {name or '<unnamed>'} (RVA 0x{target_rva:x})")
