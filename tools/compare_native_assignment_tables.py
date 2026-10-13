"""Compare two frozen MotHor cell tables without using a saved .xrho."""

import argparse
from collections import defaultdict
import gzip
from pathlib import Path
import struct
import xml.etree.ElementTree as ET


def words(path):
    blob = path.read_bytes()
    if len(blob) % 4:
        raise ValueError(f"{path}: size is not divisible by four")
    return struct.unpack(f"<{len(blob) // 4}I", blob)


def xml_lective_indices(path):
    blob = gzip.decompress(path.read_bytes())
    start = blob.find(b"<datosGHC")
    end = blob.find(b"</datosGHC>") + len(b"</datosGHC>")
    if start < 0 or end < len(b"</datosGHC>"):
        raise ValueError(f"{path}: no datosGHC XML")
    root = ET.fromstring(blob[start:end].decode("latin1"))
    result = {}
    for frame, marco in enumerate(root.findall("./marcosDeHorario/marcoHorario")):
        days = defaultdict(list)
        for tramo in marco.findall("tramo"):
            day = int(tramo.findtext("dia"))
            days[day].append((int(tramo.findtext("indice")), tramo.findtext("Tipo")))
        for day, tramos in days.items():
            lective = [index for index, kind in tramos if kind == "lectivo"]
            if len(lective) != len(set(lective)):
                raise ValueError(f"{path}: duplicate lective index in frame {frame}, day {day}")
            result[(frame, day)] = lective
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("capture_prefix", type=Path)
    parser.add_argument("--result", type=Path,
                        help="Saved .xrho whose frame definitions match the captured model")
    args = parser.parse_args()
    prefix = str(args.capture_prefix)
    lective_indices = xml_lective_indices(args.result) if args.result else None
    model = Path(prefix + "-model.bin").read_bytes() if args.result else None
    teacher_grid = words(Path(prefix + "-table-37e8.bin"))
    teacher_cells = defaultdict(set)
    for teacher in range(1, 11):
        for position in range(8):
            for day in range(5):
                pointer = teacher_grid[teacher * 0xC00 // 4 + 6 * position + day]
                if pointer:
                    teacher_cells[pointer].add((day, position, teacher))

    for frame in (0, 1):
        assignments = words(Path(prefix + f"-assignment-{frame:02d}.bin"))
        present = matched = same_cell = different_cell = 0
        shifts = []
        for index, pointer in enumerate(assignments[:48]):
            if not pointer:
                continue
            present += 1
            day, position = index % 6, index // 6
            cells = teacher_cells.get(pointer, set())
            if cells:
                matched += 1
                if any((d, p) == (day, position) for d, p, _ in cells):
                    same_cell += 1
                else:
                    different_cell += 1
                    shifts.append((day, position, hex(pointer), sorted(cells)))
        print(f"frame={frame} occupied={present} in_teacher_grid={matched} "
              f"same_day_position={same_cell} shifted_day_position={different_cell}")
        for shift in shifts[:5]:
            print("  shift", shift)
        if lective_indices is not None:
            for day in range(5):
                xml_indices = lective_indices[(frame, day)]
                native_count = model[(6 * frame + day) * 0x74AC + 0x72]
                if len(xml_indices) != native_count:
                    raise ValueError(f"frame {frame} day {day}: model has {native_count} "
                                     f"lective slots, XML has {len(xml_indices)}")
                if day == 0:
                    block = (6 * frame + day) * 0x74AC
                    special = model[block + 0x73]
                    cut_count = model[block + 0x74]
                    cuts = list(model[block + 0x75:block + 0x75 + cut_count])
                    print(f"  native_position_to_xml_indice={list(enumerate(xml_indices))}")
                    print(f"  model_special_position={special} model_cut_positions={cuts}")


if __name__ == "__main__":
    main()
