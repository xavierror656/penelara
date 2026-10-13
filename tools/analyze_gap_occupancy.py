"""Compare simple teacher-gap counts from a native .xrho result.

This is a diagnostic, not a reimplementation of MotHor's scoring routine.
"""

import argparse
import collections
import gzip
import xml.etree.ElementTree as ET
from pathlib import Path


def read_xml(path: Path):
    payload = gzip.decompress(path.read_bytes())
    start = payload.index(b"<?xml")
    end = payload.index(b"</datosGHC>") + len(b"</datosGHC>")
    return ET.fromstring(payload[start:end])


def gap_count(root, kinds, include_breaks, extra_types=()):
    slot_types = {}
    for frame in root.find("marcosDeHorario"):
        for slot in frame.findall("tramo"):
            key = (frame.attrib["id"], int(slot.findtext("dia")), int(slot.findtext("indice")))
            slot_types[key] = slot.findtext("Tipo")

    comp = {e.findtext("identificador"): e.findtext("profesor") for e in root.find("complementarias")}
    meetings = {
        e.findtext("nombre"): [v.text for v in e.findall("integrantes/integrante")]
        for e in root.find("reuniones")
    }
    occupied = collections.defaultdict(set)
    by_teacher = collections.Counter()
    for slot in root.find("horario"):
        key = (slot.attrib["marco"], int(slot.attrib["dia"]), int(slot.attrib["indice"]))
        for entry in slot:
            if entry.tag not in kinds:
                continue
            if entry.tag in ("aula", "guardia"):
                teachers = [entry.findtext("profesor")]
            elif entry.tag == "complementaria":
                teachers = [comp[entry.text]]
            else:
                teachers = meetings[entry.text]
            for teacher in teachers:
                if teacher:
                    occupied[(teacher, key[0], key[1])].add(key[2])

    total = 0
    for (teacher, frame, day), indexes in occupied.items():
        if len(indexes) < 2:
            continue
        for index in range(min(indexes) + 1, max(indexes)):
            if index not in indexes and (include_breaks or slot_types.get((frame, day, index)) in ("lectivo", *extra_types)):
                total += 1
                by_teacher[teacher] += 1
    return total, by_teacher


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("result", type=Path)
    args = parser.parse_args()
    root = read_xml(args.result)
    for kinds in [("aula",), ("aula", "guardia"), ("aula", "guardia", "complementaria"),
                  ("aula", "guardia", "complementaria", "reunion")]:
        for label, breaks, extra_types in (("lectivo", False, ()), ("con mediodía", False, ("mediodia",)),
                                           ("con recreos", False, ("recreo",)), ("todos", True, ())):
            total, by_teacher = gap_count(root, kinds, breaks, extra_types)
            print(f"{','.join(kinds):44s} slots={label:12s} gaps={total:2d} by_teacher={dict(sorted(by_teacher.items()))}")


if __name__ == "__main__":
    main()
