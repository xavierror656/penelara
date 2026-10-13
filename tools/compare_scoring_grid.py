"""Compare captured teacher-grid positions with a saved native .xrho schedule."""

import argparse
from collections import Counter, defaultdict
import gzip
from pathlib import Path
import struct
import xml.etree.ElementTree as ET


def load_schedule(path: Path):
    data = gzip.decompress(path.read_bytes())
    start = data.find(b"<datosGHC")
    end = data.find(b"</datosGHC>") + len(b"</datosGHC>")
    if start < 0 or end < len(b"</datosGHC>"):
        raise ValueError("no datosGHC XML in result")
    root = ET.fromstring(data[start:end].decode("latin1"))
    by_source_day = defaultdict(set)
    for tramo in root.find("horario").findall("tramo"):
        for sesion in tramo.iter("sesion"):
            by_source_day[(int(sesion.text), int(tramo.attrib["dia"]))].add(
                (tramo.attrib["marco"], int(tramo.attrib["indice"]))
            )
    return by_source_day


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("capture_prefix", type=Path)
    parser.add_argument("result", type=Path)
    args = parser.parse_args()
    prefix = str(args.capture_prefix)
    grid = Path(prefix + "-table-37e8.bin").read_bytes()
    object_bytes = Path(prefix + "-objects.bin").read_bytes()
    objects = {
        struct.unpack_from("<I", object_bytes, i)[0]: object_bytes[i + 4 : i + 0x284]
        for i in range(0, len(object_bytes), 0x284)
    }
    schedule = load_schedule(args.result)
    observed = Counter()
    unmatched = Counter()
    grouped_cells = Counter()
    for teacher in range(1, 11):
        for position in range(8):
            for day in range(5):
                pointer = struct.unpack_from("<I", grid, teacher * 0xC00 + 4 * (6 * position + day))[0]
                obj = objects.get(pointer)
                if not obj or struct.unpack_from("<H", obj, 0x10)[0] == 0:
                    continue
                source = struct.unpack_from("<H", obj, 2)[0]
                frame = obj[0x270]
                grouped_cells[(frame, position)] += 1
                matches = schedule.get((source, day), set())
                if len(matches) == 1:
                    marco, indice = next(iter(matches))
                    observed[(frame, position, marco, indice)] += 1
                else:
                    unmatched[(frame, position, source, day, len(matches))] += 1
    print("Grouped live cells by frame and native position:")
    for key, count in sorted(grouped_cells.items()):
        print(*key, count)
    print("Unambiguous source/day matches (live grid and saved result can differ even when frozen):")
    for key, count in sorted(observed.items()):
        print(*key, count)
    by_position = defaultdict(Counter)
    for (frame, position, marco, indice), count in observed.items():
        by_position[(frame, position)][(marco, indice)] += count
    print("Modal XML position among unambiguous matches:")
    for key, choices in sorted(by_position.items()):
        (marco, indice), count = choices.most_common(1)[0]
        print(*key, marco, indice, count, "/", sum(choices.values()))
    print("Unmatched or ambiguous source/day occurrences:", sum(unmatched.values()))


if __name__ == "__main__":
    main()
