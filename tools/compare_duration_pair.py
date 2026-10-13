"""Compare a T/1 or T/M XGHC pair with their native XRHO exports."""

import argparse
import gzip
import json
import xml.etree.ElementTree as ET
from pathlib import Path


def payload_and_xml(path: Path) -> tuple[bytes, ET.Element]:
    payload = gzip.decompress(path.read_bytes())
    start = payload.index(b"<?xml")
    end = payload.index(b"</datosGHC>") + len(b"</datosGHC>")
    return payload, ET.fromstring(payload[start:end])


def exported_complementarias(root: ET.Element) -> list[dict[str, str | None]]:
    frames = {
        (tramo.findtext("submarco"), tramo.findtext("dia"), tramo.findtext("indice")): tramo
        for tramo in root.findall("marcosDeHorario/marcoHorario/tramo")
    }
    exported = []
    for tramo in root.findall("horario/tramo"):
        key = (tramo.get("marco"), tramo.get("dia"), tramo.get("indice"))
        for child in tramo:
            if child.tag != "complementaria":
                continue
            frame = frames.get(key)
            if frame is None:
                raise ValueError(f"exported assignment has no frame slot: {key}")
            exported.append({
                "id": child.text,
                "dia": tramo.get("dia"),
                "indice": tramo.get("indice"),
                "marco": tramo.get("marco"),
                "duracion": frame.findtext("duracion"),
                "horaEntrada": frame.findtext("horaEntrada"),
                "horaSalida": frame.findtext("horaSalida"),
            })
    return exported


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("t_xghc", type=Path)
    parser.add_argument("other_xghc", type=Path)
    parser.add_argument("t_xrho", type=Path)
    parser.add_argument("other_xrho", type=Path)
    parser.add_argument("--other-marker", choices=("1", "M"), default="1")
    args = parser.parse_args()

    t_payload, t_source = payload_and_xml(args.t_xghc)
    other_payload, other_source = payload_and_xml(args.other_xghc)
    old = b"<duracion>T</duracion>"
    new = f"<duracion>{args.other_marker}</duracion>".encode("ascii")
    if t_payload.count(old) != 1 or t_payload.replace(old, new) != other_payload:
        raise ValueError(f"source payloads must differ only in one T-to-{args.other_marker} duration")

    t_duties_node = t_source.find("complementarias")
    other_duties_node = other_source.find("complementarias")
    t_duties = list(t_duties_node) if t_duties_node is not None else []
    other_duties = list(other_duties_node) if other_duties_node is not None else []
    if len(t_duties) != 1 or len(other_duties) != 1:
        raise ValueError("expected one complementary duty in each source")
    weekly = t_duties[0].findtext("intervalosSemanales")
    if weekly is None or weekly != other_duties[0].findtext("intervalosSemanales"):
        raise ValueError("weekly counts differ between source files")

    _, t_result = payload_and_xml(args.t_xrho)
    _, other_result = payload_and_xml(args.other_xrho)
    t_export = exported_complementarias(t_result)
    other_export = exported_complementarias(other_result)
    t_result_duties = t_result.find("complementarias")
    other_result_duties = other_result.find("complementarias")
    if t_result_duties is None or other_result_duties is None or len(t_result_duties) != 1 or len(other_result_duties) != 1:
        raise ValueError("expected one complementary duty in each result")
    t_declared = t_result_duties[0].findtext(
        "distribucionSemanal/distribucionVariable/numSesiones"
    )
    other_declared = other_result_duties[0].findtext(
        "distribucionSemanal/distribucionVariable/numSesiones"
    )
    if (t_declared is None or other_declared is None or
            float(t_declared) != int(weekly) or float(other_declared) != int(weekly)):
        raise ValueError("result declarations do not match the source weekly count")
    print(json.dumps({
        "source_payload_byte_differences": 1,
        "other_marker": args.other_marker,
        "source_weekly_intervals": weekly,
        "t": {"declared_in_result": t_declared, "exported": t_export},
        "normal" if args.other_marker == "1" else "m": {
            "declared_in_result": other_declared,
            "exported": other_export,
        },
    }, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
