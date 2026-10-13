"""Read the native optativa and source lookup lists from a running MotHor."""

import argparse
import ctypes
import json
import struct

from scan_scoring_object import kernel32, read_memory, read_u32


def read_string(handle, address, limit=220):
    raw = read_memory(handle, address, limit)
    return raw.split(b"\0", 1)[0].decode("cp1252", errors="replace") if raw else None


def walk(handle, first, offsets, limit=25):
    rows = []
    seen = set()
    cursor = first
    while cursor and cursor not in seen and len(rows) < limit:
        seen.add(cursor)
        raw = read_memory(handle, cursor, max(offsets) + 4)
        if raw is None:
            rows.append({"address": f"0x{cursor:08x}", "readable": False})
            break
        row = {"address": f"0x{cursor:08x}"}
        row.update({f"+0x{off:x}": struct.unpack_from("<I", raw, off)[0] for off in offsets})
        row["name"] = read_string(handle, cursor + 8)
        rows.append(row)
        cursor = struct.unpack_from("<I", raw)[0]
    return rows


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("pid", type=int)
    args = parser.parse_args()
    handle = kernel32.OpenProcess(0x410, False, args.pid)
    if not handle:
        raise OSError(ctypes.get_last_error(), "OpenProcess")
    try:
        app = read_u32(handle, 0x4932C0)
        model = read_u32(handle, app + 0xDC) if app else None
        if not model:
            raise RuntimeError("model is not initialized")
        names = walk(handle, read_u32(handle, model + 0xD037C + 4), (0x0, 0x4))
        optativas = walk(handle, read_u32(handle, model + 0xD069C), (0x0, 0xD8, 0xDC, 0xE0, 0xE4, 0x1E0, 0x1E4))
        source_nodes = walk(handle, read_u32(handle, model + 0xD040C), (0x0, 0x24, 0x704), 500)
        source_ids = {int(node["address"], 16): node["+0x704"] for node in source_nodes if node.get("readable") is not False}
        for node in optativas:
            count = min(node["+0xdc"], 64)
            base = int(node["address"], 16)
            pointers = read_memory(handle, base + 0xE0, count * 4)
            flags = read_memory(handle, base + 0x1E0, count)
            node["members"] = [
                {"sourceID": source_ids.get(pointer), "sourcePointer": f"0x{pointer:08x}",
                 "solapable": bool(flags[i]) if flags else None}
                for i in range(count)
                for pointer in [struct.unpack_from("<I", pointers, i * 4)[0]]
            ] if pointers else None
        print(json.dumps({"pid": args.pid, "model": f"0x{model:08x}",
                          "lookupNames": names, "optativas": optativas,
                          "sourceNodes": source_nodes}, ensure_ascii=False, indent=2))
    finally:
        kernel32.CloseHandle(handle)


if __name__ == "__main__":
    main()
