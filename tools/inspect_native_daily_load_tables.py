"""Capture MotHor's two per-frame daily resource-load tables."""

import argparse
import ctypes
import hashlib
import json
import struct
from pathlib import Path

from scan_scoring_object import MemoryBasicInformation, kernel32, read_memory, read_u32


def find_context(handle, data: int) -> int:
    needle = struct.pack("<I", data)
    address = 0x10000
    while address < 0x100000000:
        region = MemoryBasicInformation()
        if not kernel32.VirtualQueryEx(handle, address, ctypes.byref(region),
                                        ctypes.sizeof(region)):
            break
        base = region.BaseAddress or address
        size = region.RegionSize
        next_address = base + size
        if next_address <= address:
            break
        if region.State == 0x1000 and not region.Protect & 0x101:
            offset = 0
            while offset < size:
                chunk = read_memory(handle, base + offset,
                                    min(0x100000, size - offset))
                if chunk:
                    position = chunk.find(needle)
                    while position >= 0:
                        context = base + offset + position - 0xC900
                        if (context > 0 and read_u32(handle, context + 0xC900) == data
                                and read_u32(handle, context + 0x4814) == 1
                                and read_u32(handle, context + 0x4704) is not None):
                            pointer = read_u32(handle, context + 0x1258)
                            if pointer and read_memory(handle, pointer, 6):
                                return context
                        position = chunk.find(needle, position + 1)
                offset += 0x100000
        address = next_address
    raise RuntimeError("MotHor context containing daily-load tables was not found")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("pid", type=int)
    parser.add_argument("--resources", type=int, default=12)
    parser.add_argument("--dump-prefix", type=Path)
    parser.add_argument("--suspend", action="store_true",
                        help="pause the isolated engine during the table read")
    args = parser.parse_args()
    if not 1 <= args.resources <= 1000:
        parser.error("resources must be between 1 and 1000")

    access = 0x1000 | 0x400 | 0x10 | (0x800 if args.suspend else 0)
    handle = kernel32.OpenProcess(access, False, args.pid)
    if not handle:
        raise OSError(ctypes.get_last_error(), "OpenProcess")
    suspended = False
    try:
        data = read_u32(handle, 0x4932C0)
        if data is None:
            raise RuntimeError("MotHor has not published its data pointer")
        context = find_context(handle, data)
        if args.suspend:
            ntdll = ctypes.WinDLL("ntdll")
            ntdll.NtSuspendProcess.argtypes = [ctypes.c_void_p]
            ntdll.NtSuspendProcess.restype = ctypes.c_long
            ntdll.NtResumeProcess.argtypes = [ctypes.c_void_p]
            ntdll.NtResumeProcess.restype = ctypes.c_long
            status = ntdll.NtSuspendProcess(handle)
            if status:
                raise OSError(status, "NtSuspendProcess")
            suspended = True
        frames = []
        for frame in range(2):
            pair = {}
            for name, offset in (("first", 0x1258), ("second", 0x1270)):
                address = read_u32(handle, context + offset + 4 * frame)
                if not address:
                    pair[name] = None
                    continue
                raw = read_memory(handle, address, 6 * args.resources)
                if raw is None:
                    raise RuntimeError(f"cannot read {name} table for frame {frame}")
                entry = {
                    "address": address,
                    "sha256": hashlib.sha256(raw).hexdigest(),
                    "rows": [list(raw[i:i + 6])
                             for i in range(0, len(raw), 6)],
                }
                if args.dump_prefix:
                    path = Path(f"{args.dump_prefix}-frame{frame}-{name}.bin")
                    path.write_bytes(raw)
                    entry["dump"] = str(path)
                pair[name] = entry
            frames.append({"frame": frame, "tables": pair})
        print(json.dumps({"pid": args.pid, "data": data,
                          "context": context, "suspended_for_read": suspended,
                          "frames": frames}))
    finally:
        try:
            if suspended:
                status = ntdll.NtResumeProcess(handle)
                if status:
                    raise OSError(status, "NtResumeProcess")
        finally:
            kernel32.CloseHandle(handle)


if __name__ == "__main__":
    main()
