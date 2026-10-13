"""Read MotHor's saved maximum-consecutive-hours fields during a replay."""

import argparse
import ctypes
import json
import struct
from ctypes import wintypes


kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
kernel32.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
kernel32.OpenProcess.restype = wintypes.HANDLE
kernel32.ReadProcessMemory.argtypes = [
    wintypes.HANDLE,
    ctypes.c_void_p,
    ctypes.c_void_p,
    ctypes.c_size_t,
    ctypes.POINTER(ctypes.c_size_t),
]
kernel32.ReadProcessMemory.restype = wintypes.BOOL
kernel32.CloseHandle.argtypes = [wintypes.HANDLE]
kernel32.CloseHandle.restype = wintypes.BOOL


def read_bytes(handle, address, size):
    if not address:
        return None
    data = ctypes.create_string_buffer(size)
    count = ctypes.c_size_t()
    if not kernel32.ReadProcessMemory(handle, address, data, size, ctypes.byref(count)):
        return None
    return data.raw if count.value == size else None


def read_u32(handle, address):
    data = read_bytes(handle, address, 4)
    return struct.unpack("<I", data)[0] if data is not None else None


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("pid", type=int)
    args = parser.parse_args()
    handle = kernel32.OpenProcess(0x1000 | 0x10, False, args.pid)
    if not handle:
        raise OSError(ctypes.get_last_error(), "OpenProcess")
    try:
        app = read_u32(handle, 0x4932C0)
        bridge = read_u32(handle, app + 0xDC) if app else None
        slot = read_u32(handle, bridge + 0xD0320) if bridge else None
        model = read_u32(handle, slot) if slot else None
        global_maximas = read_bytes(handle, model + 0xCFAB4, 2) if model else None
        frames = []
        if app:
            for frame in range(2):
                records = read_u32(handle, app + 0xF8 + 4 * frame)
                values = []
                strict = []
                if records:
                    for index in range(12):
                        data = read_bytes(handle, records + 0xBC * index + 0x28, 2)
                        values.append(list(data) if data is not None else None)
                        strict_byte = read_bytes(handle, records + 0xBC * index + 0xB9, 1)
                        strict.append(strict_byte[0] if strict_byte is not None else None)
                frames.append({"frame": frame, "records": records, "plus28_plus29": values, "plusB9": strict})
        print(json.dumps({
            "pid": args.pid,
            "app": app,
            "model": model,
            "global_maximas": list(global_maximas) if global_maximas is not None else None,
            "frames": frames,
        }, ensure_ascii=False))
    finally:
        kernel32.CloseHandle(handle)


if __name__ == "__main__":
    main()
