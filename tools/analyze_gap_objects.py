"""Correlate MotHor gap table objects with sessions and frames in an .xrho."""

import argparse
import collections
import gzip
import json
import struct
import xml.etree.ElementTree as ET
from decimal import Decimal
from pathlib import Path


def load_xml(path):
    payload = gzip.decompress(path.read_bytes())
    if payload[:1] != b"M" or len(payload) < 7:
        raise ValueError("unrecognized GHC payload")
    length = struct.unpack_from("<I", payload, 3)[0]
    return ET.fromstring(payload[7:7 + length])


def load_objects(path):
    data = path.read_bytes()
    if len(data) % 0x284:
        raise ValueError("object dump has a partial record")
    objects = {}
    for offset in range(0, len(data), 0x284):
        pointer = struct.unpack_from("<I", data, offset)[0]
        raw = data[offset + 4:offset + 0x284]
        packed = struct.unpack_from("<I", raw)[0]
        objects[pointer] = {"unit": packed & 0xFFFF,
                            "source": packed >> 16,
                            "frame": raw[0x270],
                            "teacherIndex": struct.unpack_from("<H", raw, 8)[0]}
    return objects


def integral_count(value, label):
    count = Decimal(value)
    if count != int(count) or count < 0:
        raise ValueError(f"{label} is not a nonnegative integral count: {value}")
    return int(count)


def auxiliary_sources(root, first_source):
    """Assign candidate IDs in the observed non-recess XML declaration order."""
    groups = []

    for guardia in root.findall("./guardias/guardia"):
        if guardia.findtext("enRecreos") == "true":
            continue
        for member in guardia.findall("./integrantes/integrante"):
            name = member.findtext("nombre")
            groups.append({"kind": "guardia", "name": guardia.findtext("nombre"),
                           "frame": guardia.attrib["subMarco"], "teacher": name,
                           "declaredCount": integral_count(member.findtext("numeroDeGuardias"),
                                                             f"guardia for {name}")})

    for reunion in root.findall("./reuniones/reunion"):
        groups.append({"kind": "reunion", "name": reunion.findtext("nombre"),
                       "frame": reunion.attrib["subMarco"],
                       "teachers": [e.text for e in reunion.findall("./integrantes/integrante")],
                       "declaredCountPerTeacher": integral_count(reunion.findtext("numeroDeReuniones"),
                                                                   "reunion count")})

    for complementaria in root.findall("./complementarias/complementaria"):
        count = complementaria.findtext("./distribucionSemanal/distribucionVariable/numSesiones")
        groups.append({"kind": "complementaria",
                       "name": complementaria.findtext("identificador"),
                       "frame": complementaria.attrib["subMarco"],
                       "teacher": complementaria.findtext("profesor"),
                       "declaredCount": integral_count(count, "complementaria count")})

    return {first_source + offset: group for offset, group in enumerate(groups)}


def scheduled_auxiliary(root):
    """Count the saved .xrho schedule independently of the live object dump."""
    complementary_teachers = {
        e.findtext("identificador"): e.findtext("profesor")
        for e in root.findall("./complementarias/complementaria")}
    meeting_teachers = {
        e.findtext("nombre"): [m.text for m in e.findall("./integrantes/integrante")]
        for e in root.findall("./reuniones/reunion")}
    scheduled = collections.defaultdict(collections.Counter)
    for slot in root.findall("./horario/tramo"):
        frame = slot.attrib["marco"]
        for entry in slot:
            if entry.tag == "guardia":
                scheduled[("guardia", entry.findtext("nombre"), frame)][entry.findtext("profesor")] += 1
            elif entry.tag == "reunion":
                for teacher in meeting_teachers[entry.text]:
                    scheduled[("reunion", entry.text, frame)][teacher] += 1
            elif entry.tag == "complementaria":
                teacher = complementary_teachers[entry.text]
                scheduled[("complementaria", entry.text, frame)][teacher] += 1
    return scheduled


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("prefix", type=Path, help="prefix passed to --dump-prefix")
    parser.add_argument("result", type=Path, help=".xrho from the same project")
    args = parser.parse_args()

    root = load_xml(args.result)
    sessions = {int(element.attrib["id"]): element.findtext("profesor")
                for element in root.findall(".//sesion[@id]")}
    scheduled_frames = collections.defaultdict(set)
    for tramo in root.findall(".//horario/tramo"):
        for session in tramo.findall(".//sesion"):
            if session.text:
                scheduled_frames[int(session.text)].add(tramo.attrib["marco"])
    calendars = {}
    for marco in root.findall(".//marcoHorario"):
        lective = [tramo for tramo in marco.findall("tramo")
                   if tramo.findtext("dia") == "0" and tramo.findtext("Tipo") == "lectivo"]
        days = {tramo.findtext("dia") for tramo in marco.findall("tramo")}
        calendars[marco.attrib["id"]] = {"days": len(days), "lectiveSlots": len(lective)}

    objects = load_objects(Path(str(args.prefix) + "-objects.bin"))
    units = collections.Counter(obj["unit"] for obj in objects.values())
    pool_bases = {pointer - 0x288 * obj["unit"] for pointer, obj in objects.items()}
    object_pool = {"candidateStrideBytes": 0x288,
                   "candidateBaseAddress": f"0x{next(iter(pool_bases)):08x}"
                   if len(pool_bases) == 1 else None,
                   "allPointersMatchCandidateStride": len(pool_bases) == 1,
                   "duplicateUnitIndices": [unit for unit, count in units.items() if count > 1],
                   "capturedUnitRange": [min(units), max(units)],
                   "unitIndicesAbsentFromCapturedTables":
                   [unit for unit in range(min(units), max(units) + 1) if unit not in units]}
    teachers = [None] + [e.findtext("nombre") for e in root.findall("./profesores/profesor")]
    expected_auxiliary = auxiliary_sources(root, max(sessions) + 1)
    scheduled = scheduled_auxiliary(root)
    context = Path(str(args.prefix) + "-context.bin").read_bytes()
    count = struct.unpack_from("<I", context, 0x4704)[0]
    if not 0 < count <= 64:
        raise ValueError(f"unexpected table row count: {count}")

    table_sets = {}
    by_row = [collections.Counter() for _ in range(count)]
    rows = []
    invalid_positions = []
    teacher_index_mismatches = []
    reference_count = 0
    raw_internal_gaps = {}
    for field, frame in ((0x3848, 0), (0x384C, 1)):
        path = Path(str(args.prefix) + f"-table-{field:04x}.bin")
        data = path.read_bytes()
        if len(data) < count * 0xC00:
            raise ValueError(f"{path} lacks {count} complete rows")
        pointer_set = set()
        calendar = calendars["A" if frame == 0 else "B"]
        frame_gaps = 0
        for row in range(count):
            values = struct.unpack_from("<768I", data, row * 0xC00)
            indexed = [(position, value)
                       for position, value in enumerate(values)
                       if value]
            if row:
                for day in range(calendar["days"]):
                    occupied = [slot for slot in range(calendar["lectiveSlots"])
                                if values[6 * slot + day]]
                    if occupied:
                        frame_gaps += occupied[-1] - occupied[0] + 1 - len(occupied)
            pointers = [value for _, value in indexed]
            pointer_set.update(pointers)
            names = collections.Counter()
            for position, pointer in indexed:
                reference_count += 1
                obj = objects.get(pointer)
                if obj is None:
                    raise ValueError(f"object 0x{pointer:x} missing from capture")
                if obj["frame"] != frame:
                    raise ValueError(f"object 0x{pointer:x} has wrong frame")
                if obj["teacherIndex"] != row:
                    teacher_index_mismatches.append(f"0x{pointer:08x}")
                if position % 6 >= calendar["days"] or position // 6 >= calendar["lectiveSlots"]:
                    invalid_positions.append({"frame": frame, "row": row, "position": position})
                name = sessions.get(obj["source"])
                if name:
                    names[name] += 1
                    by_row[row][name] += 1
            rows.append({"table": f"+0x{field:x}", "frame": frame,
                         "index": row, "references": len(pointers),
                         "sourceTeacherCounts": dict(names)})
        table_sets[field] = pointer_set
        raw_internal_gaps["A" if frame == 0 else "B"] = frame_gaps

    checked = 0
    mismatches = []
    for pointer, obj in objects.items():
        frames = scheduled_frames.get(obj["source"])
        if frames:
            checked += 1
            expected = "A" if obj["frame"] == 0 else "B"
            if expected not in frames:
                mismatches.append(f"0x{pointer:08x}")

    inferred_teachers = []
    for index, counts in enumerate(by_row):
        best = counts.most_common(2)
        inferred_teachers.append({"index": index, "teacher": best[0][0] if best else None,
                                  "support": best[0][1] if best else 0,
                                  "other": sum(counts.values()) - (best[0][1] if best else 0)})
    auxiliary_observed = collections.defaultdict(collections.Counter)
    auxiliary_frames = collections.defaultdict(collections.Counter)
    for obj in objects.values():
        if obj["source"] >= min(expected_auxiliary):
            auxiliary_observed[obj["source"]][obj["teacherIndex"]] += 1
            auxiliary_frames[obj["source"]][obj["frame"]] += 1
    auxiliary_rows = []
    for source, group in expected_auxiliary.items():
        expected = ({teachers.index(group["teacher"]): group["declaredCount"]}
                    if "teacher" in group else
                    {teachers.index(name): group["declaredCountPerTeacher"]
                     for name in group["teachers"]})
        observed = auxiliary_observed.get(source, collections.Counter())
        scheduled_group = scheduled.get((group["kind"], group["name"], group["frame"]),
                                        collections.Counter())
        scheduled_counts = {teachers[index]: scheduled_group[teachers[index]]
                            for index in expected}
        auxiliary_rows.append({"source": source, **group,
                               "declaredByTeacher": {teachers[index]: count
                                                     for index, count in expected.items()},
                               "capturedByTeacher": {teachers[index]: count
                                                     for index, count in observed.items()},
                               "scheduledByTeacher": scheduled_counts,
                               "capturedFrames": dict(auxiliary_frames[source]),
                               "matchesDeclaration": dict(observed) == expected and
                               set(auxiliary_frames[source]) == {0 if group["frame"] == "A" else 1},
                               "scheduleMatchesDeclaration":
                               scheduled_counts == {teachers[index]: count
                                                    for index, count in expected.items()}})
    unmapped_auxiliary = {source: dict(counts) for source, counts in auxiliary_observed.items()
                          if source not in expected_auxiliary}
    print(json.dumps({"objects": len(objects),
                      "objectPool": object_pool,
                      "tableReferences": reference_count,
                      "tableUniqueObjects": {f"+0x{k:x}": len(v) for k, v in table_sets.items()},
                      "tableOverlap": len(table_sets[0x3848] & table_sets[0x384C]),
                      "calendars": calendars,
                      "teacherIndexMismatches": teacher_index_mismatches,
                      "invalidPositions": invalid_positions,
                      "rawInternalGaps": {**raw_internal_gaps, "total": sum(raw_internal_gaps.values())},
                      "frameCheckedAgainstResult": checked,
                      "frameMismatches": mismatches,
                      "inferredTeachers": inferred_teachers,
                      "auxiliaryGroups": auxiliary_rows,
                      "auxiliaryMatches": sum(row["matchesDeclaration"] for row in auxiliary_rows),
                      "auxiliaryScheduleMatches": sum(row["scheduleMatchesDeclaration"]
                                                       for row in auxiliary_rows),
                      "recessGuardsScheduled": sum(scheduled[("guardia", e.findtext("nombre"),
                                                                e.attrib["subMarco"])].total()
                                                  for e in root.findall("./guardias/guardia")
                                                  if e.findtext("enRecreos") == "true"),
                      "unmappedAuxiliarySources": unmapped_auxiliary,
                      "rows": rows}, ensure_ascii=True, indent=2))


if __name__ == "__main__":
    main()
