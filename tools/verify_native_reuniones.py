"""Verify one weekly meeting per source and its teacher-time constraints."""

import argparse
from collections import Counter, defaultdict
import gzip
import json
from pathlib import Path
import xml.etree.ElementTree as ET


def read_xml(path: Path) -> ET.Element:
    payload = gzip.decompress(path.read_bytes())
    start = payload.index(b"<?xml")
    end = payload.index(b"</datosGHC>") + len(b"</datosGHC>")
    return ET.fromstring(payload[start:end].decode("latin1"))


def minute(value: str) -> int:
    hour, minute, _ = map(int, value.split(":"))
    return 60 * hour + minute


def overlap(left: tuple[int, int], right: tuple[int, int]) -> bool:
    return left[0] < right[1] and right[0] < left[1]


def forbidden(template: ET.Element | None) -> set[tuple[str, int, int]]:
    if template is None:
        return set()
    return {
        (tramo.attrib["marco"], int(tramo.attrib["dia"]), int(tramo.attrib["indice"]))
        for tramo in template.findall("tramo")
        if (tramo.text or "").strip() == "prohibido"
    }


def verify(source: Path, result: Path) -> dict:
    original = read_xml(source)
    solved = read_xml(result)
    sites = {}
    for frame in original.find("marcosDeHorario"):
        frame_id = frame.attrib["id"]
        for tramo in frame.findall("tramo"):
            site = (frame_id, int(tramo.findtext("dia")), int(tramo.findtext("indice")))
            if site in sites:
                raise ValueError(f"duplicate source site {site}")
            sites[site] = (
                tramo.findtext("Tipo", "").lower(),
                minute(tramo.findtext("horaEntrada")),
                minute(tramo.findtext("horaSalida")),
                tramo.findtext("permitido") != "false",
            )
    teachers = {
        p.findtext("nombre"): forbidden(p.find("plantilla"))
        for p in original.find("profesores")
    }
    meetings = {}
    for meeting in original.find("reuniones"):
        key = (meeting.attrib["subMarco"], meeting.findtext("nombre"))
        if key in meetings:
            raise ValueError(f"duplicate meeting {key}")
        members = [m.text for m in meeting.findall("./integrantes/integrante")]
        if len(members) != len(set(members)):
            raise ValueError(f"duplicate meeting member {key}")
        for member in members:
            if member not in teachers:
                raise ValueError(f"unknown meeting teacher {key}: {member}")
        meetings[key] = (members, forbidden(meeting.find("plantilla")))
    complementary_teacher = {
        (item.attrib["subMarco"], item.findtext("identificador")): item.findtext("profesor")
        for item in original.find("complementarias")
    }
    counts = Counter()
    activities = defaultdict(list)
    horario = solved.find("horario")
    if horario is None:
        raise ValueError("result has no horario")
    for tramo in horario.findall("tramo"):
        site = (tramo.attrib["marco"], int(tramo.attrib["dia"]), int(tramo.attrib["indice"]))
        if site not in sites:
            raise ValueError(f"unknown result site {site}")
        kind, start, end, permitted = sites[site]
        for item in tramo.findall("reunion"):
            key = (site[0], item.text)
            if key not in meetings:
                raise ValueError(f"unknown meeting assignment {key}")
            if kind != "lectivo" or not permitted or site in meetings[key][1]:
                raise ValueError(f"meeting on forbidden site {key}: {site}")
            counts[key] += 1
            for teacher in meetings[key][0]:
                if site in teachers[teacher]:
                    raise ValueError(f"meeting in forbidden teacher site {teacher}: {site}")
                activities[(site[1], teacher)].append((start, end, "meeting", key, site))
        for aula in tramo.findall("aula"):
            teacher = aula.findtext("profesor")
            if teacher:
                activities[(site[1], teacher)].append((start, end, "class", aula.findtext("sesion"), site))
        for guard in tramo.findall("guardia"):
            teacher = guard.findtext("profesor")
            if teacher:
                activities[(site[1], teacher)].append((start, end, "guard", guard.findtext("nombre"), site))
        for comp in tramo.findall("complementaria"):
            key = (site[0], comp.text)
            if key not in complementary_teacher:
                raise ValueError(f"unknown complementary assignment {key}")
            teacher = complementary_teacher[key]
            activities[(site[1], teacher)].append((start, end, "complementary", key, site))
    for key in meetings:
        if counts[key] != 1:
            raise ValueError(f"meeting {key} occurs {counts[key]} times, expected one")
    for person_day, entries in activities.items():
        for i, left in enumerate(entries):
            if left[2] != "meeting":
                continue
            for right in entries[i + 1:]:
                if overlap(left[:2], right[:2]):
                    raise ValueError(f"meeting collision for {person_day}: {left[2:]} / {right[2:]}")
            for right in entries[:i]:
                if overlap(left[:2], right[:2]):
                    raise ValueError(f"meeting collision for {person_day}: {left[2:]} / {right[2:]}")
    return {
        "source": str(source),
        "result": str(result),
        "meetings": len(meetings),
        "assignments": sum(counts.values()),
        "onePerMeeting": True,
        "forbiddenSites": 0,
        "teacherTimeCollisions": 0,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("result", type=Path)
    args = parser.parse_args()
    print(json.dumps(verify(args.source, args.result), indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
