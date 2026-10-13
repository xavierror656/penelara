"""Compare ordinary-mode XGHC session and meeting loads with MotHor records."""

import argparse
from decimal import Decimal, ROUND_HALF_UP
import gzip
from pathlib import Path
import struct
import xml.etree.ElementTree as ET

CASES = (
    ("ejemploDosMarcosHorarios.xghc", "dailybounds01", "ordinary"),
    ("ejemploDosMarcosHorariosRaquelOnlyMeetingNoLectiva.xghc", "raquelonlymeetingnolectiva01", "ordinary"),
    ("ejemploDosMarcosHorariosRaquelOnlyMeeting.xghc", "raquelonlymeeting02", "ordinary"),
    ("ejemploDosMarcosHorariosRaquelMeetingCount2.xghc", "raquelmeetingcount201", "ordinary"),
    ("ejemploDosMarcosHorariosRaquelMeetingDouble.xghc", "raquelmeetingdouble01", "ordinary"),
    ("ejemploDosMarcosHorariosRaquelMeetingCount2Double.xghc", "raquelmeetingcount2double01", "ordinary"),
    ("ejemploDosMarcosDuracionTNoComp.xghc", "dailytmode01", "T"),
    ("ejemploDosMarcosDuracionTNoCompAliciaUnDiaLibre.xghc", "dailytmodefree02", "T"),
    ("ejemploDosMarcosDuracionTNoCompRaquelLectiva1.xghc", "dailytmodelectiva1", "T"),
    ("ejemploDosMarcosDuracionTNoCompRaquelLectiva2.xghc", "dailytmodelectiva2", "T"),
)


def xml_root(path: Path) -> ET.Element:
    payload = gzip.decompress(path.read_bytes())
    start = payload.index(b"<?xml")
    end = payload.index(b"</datosGHC>") + len(b"</datosGHC>")
    return ET.fromstring(payload[start:end])


def session_units(session: ET.Element, mode: str) -> int:
    blocks = session.findall("distribucionSemanal/distribucionFija/numSesiones")
    if blocks:
        return sum(int(block.text) for block in blocks) * (4 if mode == "T" else 1)
    if mode == "T":
        return 4
    value = session.findtext("distribucionSemanal/distribucionVariable/numSesiones")
    if value is None:
        value = session.findtext("duracionSemanal", "1")
    return max(1, int(Decimal(value).quantize(Decimal(1), rounding=ROUND_HALF_UP)))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("capture_dir", type=Path)
    args = parser.parse_args()
    mismatches = []
    checked = 0
    for fixture, dump_name, mode in CASES:
        root = xml_root(args.capture_dir / "core" / fixture)
        names = [p.findtext("nombre") for p in root.findall("./profesores/profesor")]
        loads = {name: [0, 0] for name in names}
        for session in root.findall("./sesionesLectivas/sesion"):
            teacher = session.findtext("profesor")
            if not teacher:
                continue
            units = session_units(session, mode)
            loads[teacher][0] += units
            loads[teacher][1] += units
        for meeting in root.findall("./reuniones/reunion"):
            if meeting.findtext("lectiva") != "true":
                continue
            units = int(meeting.findtext("numeroDeReuniones"))
            if meeting.findtext("dobleDuracion") == "true":
                units *= 2
            if mode == "T":
                units *= 4
            for member in meeting.findall("integrantes/integrante"):
                loads[member.text][1] += units

        raw = (args.capture_dir / f"{dump_name}-scoring-records-00.bin").read_bytes()
        for index, name in enumerate(names, 1):
            row = raw[index * 0xBC : (index + 1) * 0xBC]
            observed = list(struct.unpack_from("<ii", row, 0x68))
            checked += 2
            if observed != loads[name]:
                mismatches.append((dump_name, name, observed, loads[name]))

    print(f"files={len(CASES)} weekly_loads={checked} mismatches={len(mismatches)}")
    for mismatch in mismatches:
        print("mismatch:", *mismatch)
    return int(bool(mismatches))


if __name__ == "__main__":
    raise SystemExit(main())
