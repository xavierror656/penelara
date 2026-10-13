"""Assign guards around fixed classes and activities in a native XRHO.

The current verified profile covers one required teacher per eligible site.
Assignments are checked with verify_native_guardias before being emitted.
"""

import argparse
from collections import defaultdict
import json
from pathlib import Path
import xml.etree.ElementTree as ET

from verify_native_guardias import minute, overlap, read_xml, verify_roots


def forbidden_sites(owner: ET.Element) -> set[tuple[str, int, int]]:
    return {
        (item.attrib["marco"], int(item.attrib["dia"]), int(item.attrib["indice"]))
        for item in owner.findall("./plantilla/tramo")
        if item.text and item.text.strip() == "prohibido"
    }


def fixed_activities(original: ET.Element, solved: ET.Element) -> dict:
    complementary = {
        (item.attrib["subMarco"], item.findtext("identificador")): item.findtext("profesor")
        for item in original.find("complementarias")
    }
    meetings = {
        (item.attrib["subMarco"], item.findtext("nombre")):
            [member.text for member in item.findall("./integrantes/integrante")]
        for item in original.find("reuniones")
    }
    intervals = {}
    for frame in original.find("marcosDeHorario"):
        for tramo in frame.findall("tramo"):
            key = (frame.attrib["id"], int(tramo.findtext("dia")), int(tramo.findtext("indice")))
            intervals[key] = (minute(tramo.findtext("horaEntrada")),
                              minute(tramo.findtext("horaSalida")),
                              tramo.findtext("Tipo", "").lower())
    occupied = defaultdict(list)
    for tramo in solved.find("horario"):
        site = (tramo.attrib["marco"], int(tramo.attrib["dia"]), int(tramo.attrib["indice"]))
        start, end, _ = intervals[site]
        teachers = [aula.findtext("profesor") for aula in tramo.findall("aula")]
        for item in tramo.findall("complementaria"):
            teachers.append(complementary[(site[0], item.text)])
        for item in tramo.findall("reunion"):
            teachers.extend(meetings[(site[0], item.text)])
        for teacher in teachers:
            if teacher:
                occupied[(site[1], teacher)].append((start, end))
    return intervals, occupied


def schedule(original: ET.Element, solved: ET.Element, max_nodes: int) -> tuple[list[dict], int]:
    intervals, occupied = fixed_activities(original, solved)
    teacher_forbidden = {
        item.findtext("nombre"): forbidden_sites(item)
        for item in original.find("profesores")
    }
    duties = []
    quotas = {}
    sites = []
    for item in original.find("guardias"):
        duty = (item.attrib["subMarco"], item.findtext("nombre"))
        duty_index = len(duties)
        duties.append(duty)
        recess = item.findtext("enRecreos") == "true"
        requested = [int(quantity.text) for quantity in item.find("profesoresACadaHora")]
        if requested != [1]:
            raise ValueError(f"unverified guard coverage profile for {duty}: {requested}")
        duty_forbidden = forbidden_sites(item)
        member_forbidden = {
            member.findtext("nombre"): forbidden_sites(member)
            for member in item.find("integrantes")
        }
        for member in item.find("integrantes"):
            quotas[(duty_index, member.findtext("nombre"))] = int(member.findtext("numeroDeGuardias"))
        for key, (start, end, kind) in intervals.items():
            if key[0] != duty[0] or key in duty_forbidden:
                continue
            if kind not in ("lectivo", "recreo") or (kind == "recreo") != recess:
                continue
            choices = []
            for teacher in member_forbidden:
                if quotas[(duty_index, teacher)] <= 0:
                    continue
                if key in member_forbidden[teacher] or key in teacher_forbidden.get(teacher, set()):
                    continue
                if any(overlap((start, end), busy) for busy in occupied[(key[1], teacher)]):
                    continue
                choices.append(teacher)
            if not choices:
                raise ValueError(f"no available teacher for {duty} at {key}")
            sites.append({"key": key, "duty": duty_index, "start": start,
                          "end": end, "choices": choices})
    if sum(quotas.values()) != len(sites):
        raise ValueError(f"requested {sum(quotas.values())} guards for {len(sites)} sites")
    if not sites:
        return [], 0
    conflicts = []
    for first in sites:
        mask = 0
        for index, second in enumerate(sites):
            if first["key"][1] == second["key"][1] and overlap(
                (first["start"], first["end"]), (second["start"], second["end"])
            ):
                mask |= 1 << index
        conflicts.append(mask)
    blocked = {teacher: 0 for _, teacher in quotas}
    picks = [None] * len(sites)
    nodes = 0

    def search(free: int) -> bool:
        nonlocal nodes
        nodes += 1
        if nodes > max_nodes:
            raise RuntimeError(f"search exceeded {max_nodes} nodes")
        if free == 0:
            return True
        available_by_quota = defaultdict(int)
        chosen = None
        options = None
        for index, site in enumerate(sites):
            bit = 1 << index
            if not free & bit:
                continue
            duty_index = site["duty"]
            current = []
            for teacher in site["choices"]:
                key = (duty_index, teacher)
                if quotas[key] > 0 and not (blocked[teacher] & bit):
                    current.append(teacher)
                    available_by_quota[key] += 1
            if not current:
                return False
            if options is None or len(current) < len(options):
                chosen, options = index, current
        if any(available_by_quota[key] < remaining for key, remaining in quotas.items()
               if remaining > 0):
            return False
        site = sites[chosen]
        duty_index = site["duty"]
        options.sort(key=lambda teacher:
                     available_by_quota[(duty_index, teacher)] - quotas[(duty_index, teacher)])
        for teacher in options:
            quota_key = (duty_index, teacher)
            previous = blocked[teacher]
            blocked[teacher] |= conflicts[chosen]
            quotas[quota_key] -= 1
            picks[chosen] = teacher
            if search(free ^ (1 << chosen)):
                return True
            picks[chosen] = None
            quotas[quota_key] += 1
            blocked[teacher] = previous
        return False

    if not search((1 << len(sites)) - 1):
        raise ValueError("no guard assignment satisfies quotas and time constraints")
    rows = [{"marco": site["key"][0], "dia": site["key"][1],
             "indice": site["key"][2], "guardia": duties[site["duty"]][1],
             "profesor": picks[index]}
            for index, site in enumerate(sites)]
    return rows, nodes


def apply_to_result(solved: ET.Element, assignments: list[dict]) -> ET.Element:
    candidate = ET.fromstring(ET.tostring(solved))
    horario = candidate.find("horario")
    tramos = {}
    for tramo in horario.findall("tramo"):
        for duty in tramo.findall("guardia"):
            tramo.remove(duty)
        key = (tramo.attrib["marco"], int(tramo.attrib["dia"]), int(tramo.attrib["indice"]))
        tramos[key] = tramo
    for assignment in assignments:
        key = (assignment["marco"], assignment["dia"], assignment["indice"])
        tramo = tramos.get(key)
        if tramo is None:
            tramo = ET.SubElement(horario, "tramo", marco=key[0], dia=str(key[1]), indice=str(key[2]))
            tramos[key] = tramo
        guardia = ET.SubElement(tramo, "guardia")
        ET.SubElement(guardia, "nombre").text = assignment["guardia"]
        ET.SubElement(guardia, "profesor").text = assignment["profesor"]
    return candidate


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("fixed_schedule", type=Path,
                        help="XRHO whose classes/complementaries/meetings remain fixed")
    parser.add_argument("--output", type=Path, help="optional JSON file for the new guard assignments")
    parser.add_argument("--max-nodes", type=int, default=1_000_000)
    args = parser.parse_args()
    original, solved = read_xml(args.source), read_xml(args.fixed_schedule)
    assignments, nodes = schedule(original, solved, args.max_nodes)
    candidate = apply_to_result(solved, assignments)
    verification = verify_roots(original, candidate, str(args.source), "generated guards")
    payload = {"fixedSchedule": str(args.fixed_schedule), "searchNodes": nodes,
               "verification": verification, "assignments": assignments}
    if args.output:
        args.output.write_text(json.dumps(payload, indent=2, ensure_ascii=False), encoding="utf-8")
        print(json.dumps({"output": str(args.output), "searchNodes": nodes,
                          "assignments": len(assignments), "verification": verification},
                         indent=2, ensure_ascii=False))
    else:
        print(json.dumps(payload, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
