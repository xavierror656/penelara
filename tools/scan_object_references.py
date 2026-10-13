"""Find live references to selected MotHor object-pool entries.

Use a scan_scoring_object.py JSON capture from the same PID. This reads memory
only; a reference is an address containing the object's 32-bit pointer, not
proof that the native code treats that address as a schedule grid.
"""

import argparse
import ctypes
import json
import struct
from collections import defaultdict
from pathlib import Path

from scan_scoring_object import MemoryBasicInformation, kernel32, read_memory


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("pid", type=int)
    parser.add_argument("scan_json", type=Path)
    parser.add_argument("--units", default="0,1,2,3,82,83,84,85")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()

    raw_scan = args.scan_json.read_bytes()
    encoding = "utf-16" if raw_scan.startswith((b"\xff\xfe", b"\xfe\xff")) else "utf-8-sig"
    scan = json.loads(raw_scan.decode(encoding))
    if scan["pid"] != args.pid:
        parser.error("scan JSON PID does not match the live process")
    pools = [c["objectPool"] for c in scan["contexts"] if "objectPool" in c]
    if len(pools) != 1:
        parser.error(f"expected one inferred object pool, found {len(pools)}")
    pool = pools[0]
    base = int(pool["base"], 16)
    stride = pool["stride"]
    units = [int(x, 0) for x in args.units.split(",")]
    pointers = {base + unit * stride: unit for unit in units}

    handle = kernel32.OpenProcess(0x410, False, args.pid)
    if not handle:
        raise OSError(ctypes.get_last_error(), "OpenProcess")
    found = defaultdict(list)
    scanned_bytes = 0
    try:
        address = 0x10000
        while address < 0x100000000:
            region = MemoryBasicInformation()
            if not kernel32.VirtualQueryEx(handle, address, ctypes.byref(region), ctypes.sizeof(region)):
                break
            region_base = region.BaseAddress or address
            region_size = region.RegionSize
            next_address = region_base + region_size
            if next_address <= address:
                break
            if region.State == 0x1000 and not region.Protect & 0x101:
                offset = 0
                while offset < region_size:
                    size = min(0x100000, region_size - offset)
                    data = read_memory(handle, region_base + offset, size)
                    if data is not None:
                        scanned_bytes += len(data)
                        for i in range(0, len(data) - 3, 4):
                            value = struct.unpack_from("<I", data, i)[0]
                            if value in pointers:
                                found[pointers[value]].append({
                                    "address": f"0x{region_base + offset + i:08x}",
                                    "allocationBase": f"0x{region.AllocationBase:08x}",
                                    "protect": f"0x{region.Protect:x}",
                                    "nearbyWords": [f"0x{word:08x}" for word in
                                                    struct.unpack_from("<4I", data, max(0, i - 8))]
                                    if i >= 8 and i + 8 <= len(data) else [],
                                })
                    offset += size
            address = next_address
    finally:
        kernel32.CloseHandle(handle)

    result = {"pid": args.pid, "poolBase": pool["base"], "stride": stride,
              "scannedBytes": scanned_bytes,
              "references": {str(unit): found[unit] for unit in units}}
    output = json.dumps(result, indent=2)
    if args.output:
        args.output.write_text(output + "\n", encoding="utf-8")
    else:
        print(output)


if __name__ == "__main__":
    main()
