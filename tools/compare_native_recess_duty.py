"""Cross-check native recess-duty records against an XGHC input and mask table."""

import argparse
import gzip
from pathlib import Path
import struct
import xml.etree.ElementTree as ET


def read_xml(path: Path) -> ET.Element:
    payload = gzip.decompress(path.read_bytes())
    start = payload.index(b"<?xml")
    end = payload.index(b"</datosGHC>") + len(b"</datosGHC>")
    return ET.fromstring(payload[start:end].decode("latin1"))


def recess_positions(root: ET.Element, frame_id: str) -> dict[int, set[int]]:
    frame = next(m for m in root.find("marcosDeHorario") if m.attrib["id"] == frame_id)
    by_day: dict[int, list[ET.Element]] = {}
    for tramo in frame.findall("tramo"):
        by_day.setdefault(int(tramo.findtext("dia")), []).append(tramo)
    result = {}
    for day, tramos in by_day.items():
        count_lective = 0
        result[day] = set()
        for tramo in sorted(tramos, key=lambda item: int(item.findtext("indice"))):
            kind = tramo.findtext("Tipo", "").lower()
            if kind == "recreo":
                result[day].add(count_lective)
            elif kind == "lectivo":
                count_lective += 1
    return result


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input", type=Path, help="original .xghc input")
    parser.add_argument("capture_prefix", type=Path)
    args = parser.parse_args()
    root = read_xml(args.input)
    teachers = [p.findtext("nombre") for p in root.find("profesores")]
    duties = [g for g in root.find("guardias") if g.findtext("enRecreos") == "true"]
    if len(duties) != 1:
        raise ValueError(f"expected one recess duty, found {len(duties)}")
    duty = duties[0]
    members = list(duty.find("integrantes"))
    records = Path(str(args.capture_prefix) + "-auxiliary-records.bin").read_bytes()
    mask = Path(str(args.capture_prefix) + "-table-3770.bin").read_bytes()
    if len(records) != len(members) * 0x19C:
        raise ValueError("native auxiliary record count differs from recess duty member count")
    positions = recess_positions(root, duty.attrib["subMarco"])
    for index, member in enumerate(members):
        record = records[index * 0x19C : (index + 1) * 0x19C]
        name = member.findtext("nombre")
        teacher_id = teachers.index(name) + 1
        native_id = struct.unpack_from("<I", record)[0]
        requested = int(member.findtext("numeroDeGuardias"))
        native_requested = record[4]
        if (teacher_id, requested) != (native_id, native_requested):
            raise ValueError(f"member {index} mismatch: {name}, IDs {teacher_id}/{native_id}, requested {requested}/{native_requested}")
        if record[0x19A] != 0:
            raise ValueError(f"member {index} native frame is {record[0x19A]}, expected frame 0")
        marks = []
        for day in range(5):
            for position in range(8):
                offset = teacher_id * 0x300 + 6 * position + day
                if mask[offset] & 1:
                    if position not in positions[day]:
                        raise ValueError(f"non-recess mark for {name} at day={day}, position={position}")
                    marks.append((day, position))
            recorded_day_count = record[0xC + day]
            observed_day_count = sum(mark_day == day for mark_day, _ in marks)
            if recorded_day_count != observed_day_count:
                raise ValueError(f"daily count mismatch for {name}, day={day}: {recorded_day_count}/{observed_day_count}")
        if len(marks) != requested:
            raise ValueError(f"duty count mismatch for {name}: {len(marks)}/{requested}")
        print(f"{index}: {name} teacher={teacher_id} assigned={marks}")
    print(f"validated {len(members)} native recess-duty records and their mask cells")


if __name__ == "__main__":
    main()
