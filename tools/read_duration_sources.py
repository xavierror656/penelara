"""Read MotHor's linked source records used by 0x43a1d1–0x43a6e0."""

import argparse
import ctypes
import json
import struct

from scan_scoring_object import kernel32, read_memory, read_u32


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("pid", type=int)
    args = parser.parse_args()
    handle = kernel32.OpenProcess(0x410, False, args.pid)
    if not handle:
        raise OSError(ctypes.get_last_error(), "OpenProcess")
    try:
        app = read_u32(handle, 0x4932C0)
        bridge = read_u32(handle, app + 0xDC) if app else None
        node = read_u32(handle, bridge + 0xD047C) if bridge else None
        result = {"app": f"0x{app:08x}" if app else None,
                  "bridge": f"0x{bridge:08x}" if bridge else None,
                  "sourceHead": f"0x{node:08x}" if node else None,
                  "records": []}
        seen = set()
        while node and node not in seen and len(seen) < 256:
            seen.add(node)
            data = read_memory(handle, node, 0x334)
            if data is None:
                result["readError"] = f"0x{node:08x}"
                break
            result["records"].append({
                "address": f"0x{node:08x}",
                "next": f"0x{struct.unpack_from('<I', data)[0]:08x}",
                "baseCount310": data[0x310],
                "field311": data[0x311],
                "overrideFactor312": data[0x312],
                "field313": data[0x313],
                "frame314": data[0x314],
                "field324": struct.unpack_from("<I", data, 0x324)[0],
                "mask329": list(data[0x329:0x32F]),
                "tail300": data[0x300:0x330].hex(),
            })
            node = struct.unpack_from("<I", data)[0]
        result["terminated"] = node == 0
        result["loop"] = node in seen and node != 0
        print(json.dumps(result, ensure_ascii=False))
    finally:
        kernel32.CloseHandle(handle)


if __name__ == "__main__":
    main()
