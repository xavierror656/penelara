"""Print the imported symbol bound to a PE32 IAT virtual address."""

import argparse
import struct
from pathlib import Path


parser = argparse.ArgumentParser()
parser.add_argument("binary", type=Path)
parser.add_argument("iat_va", nargs="?", type=lambda value: int(value, 0))
parser.add_argument("--match", help="show imported symbols whose name contains this text")
args = parser.parse_args()
data = args.binary.read_bytes()
pe = struct.unpack_from("<I", data, 0x3C)[0]
sections_count = struct.unpack_from("<H", data, pe + 6)[0]
optional_size = struct.unpack_from("<H", data, pe + 20)[0]
optional = pe + 24
image_base = struct.unpack_from("<I", data, optional + 28)[0]
section_table = optional + optional_size
sections = []
for i in range(sections_count):
    entry = section_table + i * 40
    name = data[entry : entry + 8].split(b"\0", 1)[0].decode("ascii")
    size, rva, raw_size, raw = struct.unpack_from("<IIII", data, entry + 8)
    sections.append((name, rva, max(size, raw_size), raw))


def file_offset(rva):
    for _, start, size, raw in sections:
        if start <= rva < start + size:
            return raw + rva - start
    raise ValueError(f"RVA 0x{rva:x} is outside sections")


import_rva = struct.unpack_from("<I", data, optional + 104)[0]
descriptor = file_offset(import_rva)
while True:
    original, _, _, name_rva, first = struct.unpack_from("<IIIII", data, descriptor)
    if not name_rva:
        break
    dll_offset = file_offset(name_rva)
    dll = data[dll_offset : data.index(b"\0", dll_offset)].decode("ascii")
    thunk_rva = original or first
    slot = 0
    while True:
        entry_rva = struct.unpack_from("<I", data, file_offset(thunk_rva + slot * 4))[0]
        if not entry_rva:
            break
        slot_va = image_base + first + slot * 4
        if entry_rva & 0x80000000:
            symbol = f"ordinal {entry_rva & 0xFFFF}"
        else:
            name_offset = file_offset(entry_rva) + 2
            symbol = data[name_offset : data.index(b"\0", name_offset)].decode("ascii")
        if slot_va == args.iat_va:
            print(f"{dll}!{symbol}")
            raise SystemExit(0)
        if args.match and args.match.lower() in symbol.lower():
            print(f"0x{slot_va:x} {dll}!{symbol}")
        slot += 1
    descriptor += 20
if args.iat_va is not None:
    raise SystemExit(f"IAT VA 0x{args.iat_va:x} not found")
