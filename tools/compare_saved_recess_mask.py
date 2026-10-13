"""Compare live native recess-duty mask cells with a saved XRHO from a run."""

import argparse
import json
from pathlib import Path

from compare_native_recess_duty import read_xml


def source_sites(root):
    sites = {}
    for frame in root.find("marcosDeHorario"):
        frame_id = frame.attrib["id"]
        by_day = {}
        for tramo in frame.findall("tramo"):
            by_day.setdefault(int(tramo.findtext("dia")), []).append(tramo)
        for day, tramos in by_day.items():
            position = 0
            for tramo in sorted(tramos, key=lambda t: int(t.findtext("indice"))):
                index = int(tramo.findtext("indice"))
                kind = tramo.findtext("Tipo", "").lower()
                sites[(frame_id, day, index)] = (kind, position)
                if kind != "recreo":
                    position += 1
    return sites


def compare(source: Path, result: Path, prefix: Path) -> dict:
    original = read_xml(source)
    solved = read_xml(result)
    teachers = [p.findtext("nombre") for p in original.find("profesores")]
    teacher_id = {name: i + 1 for i, name in enumerate(teachers)}
    recess_duties = set()
    for duty in original.find("guardias"):
        if duty.findtext("enRecreos") != "true":
            continue
        for frame in original.find("marcosDeHorario"):
            if any(
                tramo.findtext("submarco") == duty.attrib["subMarco"]
                for tramo in frame.findall("tramo")
            ):
                recess_duties.add((frame.attrib["id"], duty.findtext("nombre")))
    sites = source_sites(original)
    saved = set()
    for tramo in solved.find("horario").findall("tramo"):
        frame = tramo.attrib["marco"]
        day = int(tramo.attrib["dia"])
        index = int(tramo.attrib["indice"])
        kind, position = sites[(frame, day, index)]
        for duty in tramo.findall("guardia"):
            name = duty.findtext("nombre")
            if (frame, name) not in recess_duties:
                continue
            if kind != "recreo":
                raise ValueError(f"saved recess duty outside recess: {(frame, day, index)}")
            teacher = duty.findtext("profesor")
            saved.add((teacher_id[teacher], day, position))
    mask = Path(f"{prefix}-table-3770.bin").read_bytes()
    native = set()
    for teacher in range(1, len(teachers) + 1):
        for day in range(5):
            for position in range(128):
                offset = teacher * 0x300 + 6 * position + day
                if offset < len(mask) and mask[offset] & 1:
                    native.add((teacher, day, position))
    common = native & saved
    return {
        "native": len(native),
        "saved": len(saved),
        "same": len(common),
        "native_only": sorted(native - saved),
        "saved_only": sorted(saved - native),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path, help="source XGHC")
    parser.add_argument("result", type=Path, help="saved XRHO")
    parser.add_argument("capture_prefix", type=Path)
    args = parser.parse_args()
    print(json.dumps(compare(args.source, args.result, args.capture_prefix), indent=2))


if __name__ == "__main__":
    main()
