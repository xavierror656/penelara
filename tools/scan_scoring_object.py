"""Locate MotHor objects referencing its live CGHC data pointer.

Read-only scan of a process launched by Replay-Ghc.ps1. Candidate fields are
reported as raw offsets; their meanings still require code-path validation.
"""

import argparse
import ctypes
import hashlib
import json
import struct
from ctypes import wintypes


class MemoryBasicInformation(ctypes.Structure):
    _fields_ = [
        ("BaseAddress", ctypes.c_void_p),
        ("AllocationBase", ctypes.c_void_p),
        ("AllocationProtect", wintypes.DWORD),
        ("PartitionId", wintypes.WORD),
        ("_padding", wintypes.WORD),
        ("RegionSize", ctypes.c_size_t),
        ("State", wintypes.DWORD),
        ("Protect", wintypes.DWORD),
        ("Type", wintypes.DWORD),
    ]


kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
kernel32.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
kernel32.OpenProcess.restype = wintypes.HANDLE
kernel32.ReadProcessMemory.argtypes = [
    wintypes.HANDLE, ctypes.c_void_p, ctypes.c_void_p, ctypes.c_size_t,
    ctypes.POINTER(ctypes.c_size_t),
]
kernel32.ReadProcessMemory.restype = wintypes.BOOL
kernel32.VirtualQueryEx.argtypes = [
    wintypes.HANDLE, ctypes.c_void_p, ctypes.POINTER(MemoryBasicInformation), ctypes.c_size_t,
]
kernel32.VirtualQueryEx.restype = ctypes.c_size_t
kernel32.CloseHandle.argtypes = [wintypes.HANDLE]
kernel32.CloseHandle.restype = wintypes.BOOL


def read_memory(handle, address, size):
    buffer = ctypes.create_string_buffer(size)
    count = ctypes.c_size_t()
    if not kernel32.ReadProcessMemory(handle, address, buffer, size, ctypes.byref(count)):
        return None
    return buffer.raw[:count.value] if count.value == size else None


def read_u32(handle, address):
    data = read_memory(handle, address, 4)
    return struct.unpack("<I", data)[0] if data is not None else None


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("pid", type=int)
    parser.add_argument("--max-address", type=lambda s: int(s, 0), default=0x100000000)
    parser.add_argument("--dump-prefix")
    args = parser.parse_args()

    handle = kernel32.OpenProcess(0x410, False, args.pid)
    if not handle:
        raise OSError(ctypes.get_last_error(), "OpenProcess")
    try:
        app = read_u32(handle, 0x4932C0)
        if not app:
            raise RuntimeError("DAT_004932c0 is not initialized")
        data_bridge = read_u32(handle, app + 0xDC)
        options_slot = read_u32(handle, data_bridge + 0xD0320) if data_bridge else None
        options = read_u32(handle, options_slot) if options_slot else None
        model_byte_8 = read_memory(handle, options + 8, 1) if options else None
        options_byte = read_memory(handle, options + 0xCFB0C, 1) if options else None
        direct = {
            "dataObject": f"0x{app:08x}",
            "dataE0": read_u32(handle, app + 0xE0),
            "optionsPointer": f"0x{options:08x}" if options else None,
            "modelByte8": model_byte_8[0] if model_byte_8 else None,
            "optionsByteCFB0C": options_byte[0] if options_byte else None,
        }
        if args.dump_prefix:
            # 0x459b52 checks app+0xe4 before deciding whether a null
            # teacher-grid cell can extend the open gap. Keep the header in
            # the same frozen snapshot as the grids and context.
            app_header = read_memory(handle, app, 0x200)
            if app_header is None:
                raise RuntimeError("Could not capture the CGHCDatos header")
            from pathlib import Path
            app_header_path = Path(args.dump_prefix + "-app-header.bin")
            app_header_path.write_bytes(app_header)
            direct["appHeader"] = {
                "path": str(app_header_path),
                "sha256": hashlib.sha256(app_header).hexdigest(),
                "e4": struct.unpack_from("<I", app_header, 0xE4)[0],
            }
            auxiliary_count = struct.unpack_from("<I", app_header, 0x188)[0]
            auxiliary_pointer = struct.unpack_from("<I", app_header, 0x18C)[0]
            if 0 < auxiliary_count <= 256 and auxiliary_pointer:
                auxiliary = read_memory(handle, auxiliary_pointer, auxiliary_count * 0x19C)
                if auxiliary is not None:
                    auxiliary_path = Path(args.dump_prefix + "-auxiliary-records.bin")
                    auxiliary_path.write_bytes(auxiliary)
                    entries = []
                    for index in range(auxiliary_count):
                        base = index * 0x19C
                        matrix_pointer = struct.unpack_from("<I", auxiliary, base + 0x194)[0]
                        entry = {"index": index, "recordId": struct.unpack_from("<I", auxiliary, base)[0],
                                 "matrixPointer": f"0x{matrix_pointer:08x}" if matrix_pointer else None,
                                 "flag199": auxiliary[base + 0x199], "frame19a": auxiliary[base + 0x19A]}
                        if matrix_pointer:
                            matrix = read_memory(handle, matrix_pointer, 0x1400)
                            if matrix is not None:
                                matrix_path = Path(args.dump_prefix + f"-auxiliary-matrix-{index:02d}.bin")
                                matrix_path.write_bytes(matrix)
                                entry["matrixDump"] = str(matrix_path)
                        entries.append(entry)
                    direct["auxiliaryRecords"] = {
                        "path": str(auxiliary_path), "count": auxiliary_count,
                        "sha256": hashlib.sha256(auxiliary).hexdigest(), "entries": entries,
                    }
            if not options:
                raise RuntimeError("CGHCDatos+0xd0320 does not point to a model")
            # 0x417383 initializes exactly this many bytes. The schedule
            # serializer 0x40c050 reads from the same model base.
            model = read_memory(handle, options, 0xD02F4)
            if model is None:
                raise RuntimeError("Could not capture the complete serialized model")
            from pathlib import Path
            model_path = Path(args.dump_prefix + "-model.bin")
            model_path.write_bytes(model)
            direct["modelDump"] = {
                "path": str(model_path),
                "size": len(model),
                "sha256": hashlib.sha256(model).hexdigest(),
            }
            # 0x414d10 resolves <horario> objects through these CGHCDatos
            # pointers. The first two entries cover the two active frames.
            assignment_root = read_memory(handle, data_bridge + 0xD06D0, 0x50)
            if assignment_root is None:
                raise RuntimeError("Could not capture the schedule assignment roots")
            root_path = Path(args.dump_prefix + "-assignment-roots.bin")
            root_path.write_bytes(assignment_root)
            assignment_tables = {}
            for index in range(16):
                pointer = struct.unpack_from("<I", assignment_root, 0x10 + index * 4)[0]
                entry = {"pointer": f"0x{pointer:08x}" if pointer else None}
                if pointer:
                    table = read_memory(handle, pointer, 0x100)
                    if table is not None:
                        table_path = Path(args.dump_prefix + f"-assignment-{index:02d}.bin")
                        table_path.write_bytes(table)
                        entry["dump"] = str(table_path)
                        entry["sha256"] = hashlib.sha256(table).hexdigest()
                assignment_tables[str(index)] = entry
            direct["assignmentRoots"] = {
                "path": str(root_path),
                "base": f"0x{data_bridge + 0xD06D0:08x}",
                "sha256": hashlib.sha256(assignment_root).hexdigest(),
                "tables": assignment_tables,
            }
        needle = struct.pack("<I", app)
        matches = []
        contexts = []
        regions = scanned_bytes = 0
        address = 0x10000
        while address < args.max_address:
            region = MemoryBasicInformation()
            count = kernel32.VirtualQueryEx(handle, address, ctypes.byref(region), ctypes.sizeof(region))
            if not count:
                break
            base = region.BaseAddress or address
            size = region.RegionSize
            next_address = base + size
            if next_address <= address:
                break
            if region.State == 0x1000 and not region.Protect & 0x101:
                regions += 1
                offset = 0
                while offset < size:
                    chunk_size = min(0x100000, size - offset)
                    chunk = read_memory(handle, base + offset, chunk_size)
                    if chunk is not None:
                        scanned_bytes += len(chunk)
                        position = chunk.find(needle)
                        while position >= 0:
                            possible_context = base + offset + position - 0xC900
                            if possible_context > 0 and read_u32(handle, possible_context + 0xC900) == app:
                                first = read_u32(handle, possible_context)
                                if first is not None:
                                    item = {"base": f"0x{possible_context:08x}",
                                            "+0x8": read_u32(handle, possible_context + 8),
                                            "+0xc": read_u32(handle, possible_context + 0xC),
                                            "+0x4814": read_u32(handle, possible_context + 0x4814)}
                                    if args.dump_prefix and item["+0xc"] is not None and item["+0xc"] < 100 and item["+0x4814"] == 1:
                                        from pathlib import Path
                                        blob = read_memory(handle, possible_context, 0xD000)
                                        if blob is not None:
                                            path = Path(args.dump_prefix + "-context.bin")
                                            path.write_bytes(blob)
                                            item["contextDump"] = str(path)
                                            criterion_pointer = struct.unpack_from("<I", blob, 0xBC)[0]
                                            criterion = read_memory(handle, criterion_pointer, 0x40) if criterion_pointer else None
                                            if criterion is not None:
                                                criterion_path = Path(args.dump_prefix + "-criterion-bytes.bin")
                                                criterion_path.write_bytes(criterion)
                                                item["criterionBytes"] = {
                                                    "pointer": f"0x{criterion_pointer:08x}",
                                                    "dump": str(criterion_path),
                                                    "sha256": hashlib.sha256(criterion).hexdigest(),
                                                }
                                            tables = {}
                                            captured_tables = {}
                                            rows = struct.unpack_from("<I", blob, 0x4704)[0]
                                            for delta in (0x102C, 0x1030, 0x1298, 0x129C,
                                                          0x3770, 0x3774, 0x37E8, 0x37EC,
                                                          0x3788, 0x378C, 0x3848, 0x384C,
                                                          0x39C8, 0x39E0, 0x39F8):
                                                pointer = struct.unpack_from("<I", blob, delta)[0]
                                                size = (0x1000 if delta in (0x102C, 0x1030)
                                                        else 0x1E08 if delta in (0x1298, 0x129C, 0x39C8,
                                                                                  0x39E0, 0x39F8)
                                                        else rows * 0x300 if delta in (0x3770, 0x3774)
                                                        and 0 < rows <= 12 else 0x9000)
                                                table = read_memory(handle, pointer, size) if pointer else None
                                                entry = {"pointer": f"0x{pointer:08x}" if pointer else None}
                                                if table is not None:
                                                    table_path = Path(args.dump_prefix + f"-table-{delta:04x}.bin")
                                                    table_path.write_bytes(table)
                                                    entry["dump"] = str(table_path)
                                                    captured_tables[delta] = table
                                                tables[f"+0x{delta:x}"] = entry
                                            item["tables"] = tables
                                            object_pointers = set()
                                            for delta in (0x3848, 0x384C):
                                                table = captured_tables.get(delta)
                                                if table is None or rows * 0xC00 > len(table):
                                                    continue
                                                object_pointers.update(struct.unpack_from(f"<{rows * 0xC00 // 4}I", table))
                                            object_pointers.discard(0)
                                            object_dump = bytearray()
                                            unreadable = 0
                                            indexed_objects = {}
                                            for pointer in sorted(object_pointers):
                                                record = read_memory(handle, pointer, 0x280)
                                                if record is None:
                                                    unreadable += 1
                                                else:
                                                    object_dump.extend(struct.pack("<I", pointer))
                                                    object_dump.extend(record)
                                                    unit = struct.unpack_from("<H", record)[0]
                                                    indexed_objects.setdefault(unit, []).append(pointer)
                                            object_path = Path(args.dump_prefix + "-objects.bin")
                                            object_path.write_bytes(object_dump)
                                            item["objects"] = {"uniquePointers": len(object_pointers),
                                                               "readable": len(object_dump) // 0x284,
                                                               "unreadable": unreadable,
                                                               "dump": str(object_path)}
                                            # The low word is a unit ordinal. Only infer the
                                            # surrounding pool when every captured object agrees.
                                            stride = 0x288
                                            bases = {pointer - unit * stride
                                                     for unit, pointers in indexed_objects.items()
                                                     for pointer in pointers}
                                            if (len(bases) == 1 and indexed_objects and
                                                    all(len(pointers) == 1 for pointers in indexed_objects.values()) and
                                                    max(indexed_objects) - min(indexed_objects) < 10000):
                                                pool_base = bases.pop()
                                                missing = []
                                                missing_dump = bytearray()
                                                for unit in range(min(indexed_objects), max(indexed_objects) + 1):
                                                    if unit in indexed_objects:
                                                        continue
                                                    pointer = pool_base + unit * stride
                                                    record = read_memory(handle, pointer, 0x280)
                                                    entry = {"unit": unit, "pointer": f"0x{pointer:08x}",
                                                             "readable": record is not None}
                                                    if record is not None:
                                                        observed_unit = struct.unpack_from("<H", record)[0]
                                                        entry["observedUnit"] = observed_unit
                                                        entry["unitMatched"] = observed_unit == unit
                                                        entry["source"] = struct.unpack_from("<H", record, 2)[0]
                                                        entry["teacherIndex"] = struct.unpack_from("<H", record, 8)[0]
                                                        entry["frame"] = record[0x270]
                                                        entry["marker10"] = record[0x10]
                                                        entry["marker1f"] = record[0x1F]
                                                        missing_dump.extend(struct.pack("<I", pointer))
                                                        missing_dump.extend(record)
                                                    missing.append(entry)
                                                missing_path = Path(args.dump_prefix + "-unreferenced-objects.bin")
                                                missing_path.write_bytes(missing_dump)
                                                item["objectPool"] = {
                                                    "base": f"0x{pool_base:08x}", "stride": stride,
                                                    "missingFromTeacherTables": missing,
                                                    "dump": str(missing_path),
                                                }
                                    contexts.append(item)
                            candidate = base + offset + position - 0xCC
                            if candidate > 0 and read_u32(handle, candidate + 0xCC) == app:
                                fields = {f"+0x{delta:x}": read_u32(handle, candidate + delta)
                                          for delta in (0x8, 0xC, 0xBC, 0xD4, 0x4704, 0x4814, 0xC900)}
                                context = fields["+0xd4"]
                                if context:
                                    fields["context+0xc900"] = read_u32(handle, context + 0xC900)
                                    fields["context+0x4814"] = read_u32(handle, context + 0x4814)
                                matches.append({"base": f"0x{candidate:08x}", **fields})
                            position = chunk.find(needle, position + 1)
                    offset += chunk_size
            address = next_address
        print(json.dumps({"pid": args.pid, **direct, "regions": regions,
                          "scannedBytes": scanned_bytes, "matches": matches,
                          "contexts": contexts}, indent=2))
    finally:
        kernel32.CloseHandle(handle)


if __name__ == "__main__":
    main()
