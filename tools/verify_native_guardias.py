"""Compare native guard assignments with source sites, quotas, and class time."""

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


def overlap(a: tuple[int, int], b: tuple[int, int]) -> bool:
    return max(a[0], b[0]) < min(a[1], b[1])


def verify(source: Path, result: Path) -> dict:
    original = read_xml(source)
    solved = read_xml(result)
    return verify_roots(original, solved, str(source), str(result))


def apply_tsv(solved: ET.Element, path: Path) -> ET.Element:
    candidate = ET.fromstring(ET.tostring(solved))
    horario = candidate.find("horario")
    tramos = {}
    for tramo in horario.findall("tramo"):
        for guardia in tramo.findall("guardia"):
            tramo.remove(guardia)
        key = (tramo.attrib["marco"], int(tramo.attrib["dia"]), int(tramo.attrib["indice"]))
        tramos[key] = tramo
    for line in path.read_text(encoding="utf-8-sig").splitlines():
        if not line or line.startswith("#") or line.startswith("marco\t"):
            continue
        frame, day, index, name, teacher = line.split("\t")
        key = (frame, int(day), int(index))
        tramo = tramos.get(key)
        if tramo is None:
            tramo = ET.SubElement(horario, "tramo", marco=frame, dia=day, indice=index)
            tramos[key] = tramo
        guardia = ET.SubElement(tramo, "guardia")
        ET.SubElement(guardia, "nombre").text = name
        ET.SubElement(guardia, "profesor").text = teacher
    return candidate


def verify_roots(original: ET.Element, solved: ET.Element,
                 source_label: str = "input", result_label: str = "result") -> dict:
    sites = {}
    for frame in original.find("marcosDeHorario"):
        frame_id = frame.attrib["id"]
        for tramo in frame.findall("tramo"):
            site = (frame_id, int(tramo.findtext("dia")), int(tramo.findtext("indice")))
            if site in sites:
                raise ValueError(f"duplicate source site {site}")
            sites[site] = (tramo.findtext("Tipo", "").lower(),
                           minute(tramo.findtext("horaEntrada")),
                           minute(tramo.findtext("horaSalida")))
    duties = {}
    expected = {}
    for duty in original.find("guardias"):
        key = (duty.attrib["subMarco"], duty.findtext("nombre"))
        if key in duties:
            raise ValueError(f"duplicate duty {key}")
        members = {m.findtext("nombre"): int(m.findtext("numeroDeGuardias"))
                   for m in duty.find("integrantes")}
        forbidden = {
            (item.attrib["marco"], int(item.attrib["dia"]), int(item.attrib["indice"]))
            for item in duty.findall("./plantilla/tramo")
            if item.text and item.text.strip() == "prohibido"
        }
        quantities = [int(item.text) for item in duty.find("profesoresACadaHora")]
        if len(quantities) != 1:
            raise ValueError(f"duty {key} uses an unverified per-hour quantity profile")
        expected[key] = {"members": members, "perSite": quantities[0],
                         "recess": duty.findtext("enRecreos") == "true"}
        duties[key] = [site for site, (kind, _, _) in sites.items()
                       if site[0] == key[0]
                       and (kind == "recreo") == expected[key]["recess"]
                       and kind in ("lectivo", "recreo")
                       and site not in forbidden]
    teacher_forbidden = {
        professor.findtext("nombre"): {
            (item.attrib["marco"], int(item.attrib["dia"]), int(item.attrib["indice"]))
            for item in professor.findall("./plantilla/tramo")
            if item.text and item.text.strip() == "prohibido"
        }
        for professor in original.find("profesores")
    }
    complementary_teacher = {
        (item.attrib["subMarco"], item.findtext("identificador")): item.findtext("profesor")
        for item in original.find("complementarias")
    }
    meeting_teachers = {
        (item.attrib["subMarco"], item.findtext("nombre")):
            [member.text for member in item.findall("./integrantes/integrante")]
        for item in original.find("reuniones")
    }
    coverage = Counter()
    quotas = Counter()
    assignments = defaultdict(list)
    class_assignments = defaultdict(list)
    other_assignments = defaultdict(list)
    for tramo in solved.find("horario"):
        site = (tramo.attrib["marco"], int(tramo.attrib["dia"]),
                int(tramo.attrib["indice"]))
        if site not in sites:
            raise ValueError(f"unknown result site {site}")
        _, start, end = sites[site]
        for duty in tramo.findall("guardia"):
            key = (site[0], duty.findtext("nombre"))
            teacher = duty.findtext("profesor")
            if key not in expected or site not in duties[key]:
                raise ValueError(f"guard at a non-duty site: {key}, {site}")
            if teacher not in expected[key]["members"]:
                raise ValueError(f"unlisted teacher {teacher} for duty {key}")
            if site in teacher_forbidden.get(teacher, set()):
                raise ValueError(f"guard in forbidden teacher site: {teacher}, {site}")
            coverage[(key, site)] += 1
            quotas[(key, teacher)] += 1
            assignments[(site[1], teacher)].append((start, end, key, site))
        for aula in tramo.findall("aula"):
            teacher = aula.findtext("profesor")
            if teacher:
                class_assignments[(site[1], teacher)].append((start, end, site))
        for item in tramo.findall("complementaria"):
            key = (site[0], item.text)
            if key not in complementary_teacher:
                raise ValueError(f"unknown complementary assignment {key}")
            teacher = complementary_teacher[key]
            other_assignments[(site[1], teacher)].append((start, end, site))
        for item in tramo.findall("reunion"):
            key = (site[0], item.text)
            if key not in meeting_teachers:
                raise ValueError(f"unknown meeting assignment {key}")
            for teacher in meeting_teachers[key]:
                other_assignments[(site[1], teacher)].append((start, end, site))
    for key, required in expected.items():
        for site in duties[key]:
            actual = coverage[(key, site)]
            if actual != required["perSite"]:
                raise ValueError(f"duty coverage {key}, {site}: {actual}/{required['perSite']}")
        for teacher, requested in required["members"].items():
            actual = quotas[(key, teacher)]
            if actual != requested:
                raise ValueError(f"duty quota {key}, {teacher}: {actual}/{requested}")
    for person_day, assigned in assignments.items():
        for index, (start, end, _, site) in enumerate(assigned):
            for other_start, other_end, _, other_site in assigned[index + 1:]:
                if overlap((start, end), (other_start, other_end)):
                    raise ValueError(f"overlapping guards for {person_day}: {site}/{other_site}")
            for class_start, class_end, class_site in class_assignments[person_day]:
                if overlap((start, end), (class_start, class_end)):
                    raise ValueError(f"guard/class collision for {person_day}: {site}/{class_site}")
            for other_start, other_end, other_site in other_assignments[person_day]:
                if overlap((start, end), (other_start, other_end)):
                    raise ValueError(f"guard/other activity collision for {person_day}: {site}/{other_site}")
    by_duty = []
    for key, required in expected.items():
        sites_count = len(duties[key])
        quotas_total = sum(required["members"].values())
        by_duty.append({"frame": key[0], "name": key[1], "recess": required["recess"],
                        "sites": sites_count, "perSite": required["perSite"],
                        "quotaTotal": quotas_total, "assigned": sum(coverage[(key, s)] for s in duties[key])})
    return {"source": source_label, "result": result_label,
            "assignments": sum(coverage.values()), "duties": by_duty,
            "guardClassCollisions": 0, "guardOtherCollisions": 0,
            "guardGuardCollisions": 0}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("result", type=Path)
    parser.add_argument("--assignments-tsv", type=Path,
                        help="replace XRHO guards with output from 'ghc guards'")
    args = parser.parse_args()
    if args.assignments_tsv:
        original = read_xml(args.source)
        candidate = apply_tsv(read_xml(args.result), args.assignments_tsv)
        report = verify_roots(original, candidate, str(args.source), str(args.assignments_tsv))
    else:
        report = verify(args.source, args.result)
    print(json.dumps(report, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
