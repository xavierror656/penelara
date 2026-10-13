"""Find x86 PE instructions mentioning an operand in executable sections."""

import argparse
import struct
import sys
from pathlib import Path


parser = argparse.ArgumentParser()
parser.add_argument("binary", type=Path)
parser.add_argument("operand")
args = parser.parse_args()

local_capstone = Path(__file__).resolve().parents[2] / ".runtime-replay" / "python"
if local_capstone.exists():
    sys.path.insert(0, str(local_capstone))
from capstone import CS_ARCH_X86, CS_MODE_32, Cs

data = args.binary.read_bytes()
pe = struct.unpack_from("<I", data, 0x3C)[0]
sections = struct.unpack_from("<H", data, pe + 6)[0]
optional = pe + 24
image_base = struct.unpack_from("<I", data, optional + 28)[0]
table = optional + struct.unpack_from("<H", data, pe + 20)[0]
decoder = Cs(CS_ARCH_X86, CS_MODE_32)
decoder.skipdata = True

for index in range(sections):
    entry = table + index * 40
    virtual_size, rva, raw_size, raw = struct.unpack_from("<IIII", data, entry + 8)
    flags = struct.unpack_from("<I", data, entry + 36)[0]
    if not flags & 0x20000000:
        continue
    section = data[raw:raw + min(raw_size, virtual_size)]
    for instruction in decoder.disasm(section, image_base + rva):
        if args.operand.lower() in instruction.op_str.lower():
            print(f"{instruction.address:08x}  {instruction.mnemonic:8s} {instruction.op_str}")
