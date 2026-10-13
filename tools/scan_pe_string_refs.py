"""Locate direct x86 references to ASCII strings in a PE32 file."""

import argparse
import struct
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("strings", nargs="+")
    args = parser.parse_args()
    data = args.binary.read_bytes()
    pe = struct.unpack_from("<I", data, 0x3C)[0]
    if data[pe:pe + 4] != b"PE\0\0":
        raise ValueError("not a PE file")
    count = struct.unpack_from("<H", data, pe + 6)[0]
    optional_size = struct.unpack_from("<H", data, pe + 20)[0]
    opt = pe + 24
    if struct.unpack_from("<H", data, opt)[0] != 0x10B:
        raise ValueError("expected PE32")
    image_base = struct.unpack_from("<I", data, opt + 28)[0]
    sections = []
    for i in range(count):
        pos = opt + optional_size + i * 40
        name = data[pos:pos + 8].split(b"\0", 1)[0].decode("ascii", "replace")
        virtual_address, raw_size, raw_offset = struct.unpack_from("<III", data, pos + 12)
        sections.append((name, virtual_address, raw_offset, raw_size))

    def location(offset: int) -> str:
        for name, virtual_address, raw_offset, raw_size in sections:
            if raw_offset <= offset < raw_offset + raw_size:
                return f"{name} VA=0x{image_base + virtual_address + offset - raw_offset:08x}"
        return "unmapped"

    for term in args.strings:
        print(term)
        start = 0
        while (offset := data.find(term.encode("ascii"), start)) >= 0:
            string_start = offset
            while string_start > 0 and data[string_start - 1] != 0 and offset - string_start < 256:
                string_start -= 1
            string_va = None
            for _, virtual_address, raw_offset, raw_size in sections:
                if raw_offset <= string_start < raw_offset + raw_size:
                    string_va = image_base + virtual_address + string_start - raw_offset
                    break
            print(f"  string file=0x{string_start:x} {location(string_start)}")
            if string_va is not None:
                for name, _, raw_offset, raw_size in sections:
                    ref = raw_offset
                    needle = struct.pack("<I", string_va)
                    end = raw_offset + raw_size
                    while (ref := data.find(needle, ref, end)) >= 0:
                        print(f"    ref file=0x{ref:x} {location(ref)} bytes={data[max(ref - 8, 0):ref + 12].hex(' ')}")
                        ref += 1
            start = offset + len(term)


if __name__ == "__main__":
    main()
