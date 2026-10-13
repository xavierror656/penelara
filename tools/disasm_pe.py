"""Disassemble a PE32 VA with the local Capstone wheel shipped by Ghidra."""

import argparse
import glob
import re
import shutil
import struct
import subprocess
import sys
from pathlib import Path


parser = argparse.ArgumentParser()
parser.add_argument("binary", type=Path)
parser.add_argument("address", type=lambda value: int(value, 0))
parser.add_argument("--bytes", type=lambda value: int(value, 0), default=0x200)
args = parser.parse_args()

local_capstone = Path(__file__).resolve().parents[2] / ".runtime-replay" / "python"
if local_capstone.exists():
    sys.path.insert(0, str(local_capstone))
try:
    from capstone import CS_ARCH_X86, CS_MODE_32, Cs
except (ImportError, OSError):
    Cs = None


def dumpbin_fallback(binary: Path, address: int, length: int) -> None:
    candidates = []
    if command := shutil.which("dumpbin"):
        candidates.append(command)
    candidates.extend(glob.glob(
        "C:/Program Files (x86)/Microsoft Visual Studio/*/BuildTools/"
        "VC/Tools/MSVC/*/bin/Hostx64/x86/dumpbin.exe"
    ))
    if not candidates:
        raise SystemExit("Capstone no está disponible y no se encontró dumpbin.exe")
    command = sorted(candidates)[-1]
    process = subprocess.Popen(
        [command, "/disasm", str(binary)], stdout=subprocess.PIPE,
        stderr=subprocess.PIPE, text=True, encoding="cp1252", errors="replace",
    )
    shown = 0
    try:
        assert process.stdout is not None
        for line in process.stdout:
            match = re.match(r"\s*([0-9A-Fa-f]{8}):", line)
            if match:
                current = int(match.group(1), 16)
                if current >= address + length and shown:
                    break
                if address <= current < address + length:
                    print(line.rstrip())
                    shown += 1
        if shown:
            process.terminate()
        _, stderr = process.communicate()
        if not shown:
            raise SystemExit(stderr.strip() or f"sin instrucciones en 0x{address:x}")
    finally:
        if process.poll() is None:
            process.kill()
            process.communicate()

data = args.binary.read_bytes()
pe = struct.unpack_from("<I", data, 0x3C)[0]
section_count = struct.unpack_from("<H", data, pe + 6)[0]
optional_size = struct.unpack_from("<H", data, pe + 20)[0]
optional = pe + 24
image_base = struct.unpack_from("<I", data, optional + 28)[0]
section_table = optional + optional_size
address_rva = args.address - image_base
for index in range(section_count):
    entry = section_table + index * 40
    virtual_size, rva, raw_size, raw = struct.unpack_from("<IIII", data, entry + 8)
    if rva <= address_rva < rva + max(virtual_size, raw_size):
        offset = raw + address_rva - rva
        available = min(args.bytes, raw_size - (address_rva - rva))
        break
else:
    raise SystemExit(f"VA 0x{args.address:x} is outside the PE sections")

if Cs is None:
    dumpbin_fallback(args.binary, args.address, args.bytes)
    raise SystemExit(0)

decoder = Cs(CS_ARCH_X86, CS_MODE_32)
decoder.skipdata = True
for instruction in decoder.disasm(data[offset : offset + available], args.address):
    print(f"{instruction.address:08x}  {instruction.mnemonic:8s} {instruction.op_str}")
