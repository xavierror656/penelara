"""Find candidate native metric blocks in XRHO trailers by their observed prefix."""

import argparse
import gzip
import json
from pathlib import Path


def scan(path: Path) -> dict:
    payload = gzip.decompress(path.read_bytes())
    end = payload.index(b"</datosGHC>") + len(b"</datosGHC>") + 1
    trailer = payload[end:]
    marker = b"\x2c\x00\x00\x00" + bytes(54)
    candidates = []
    start = 0
    while (offset := trailer.find(marker, start)) >= 0:
        if len(trailer) - offset >= 108:
            note_count = int.from_bytes(trailer[offset + 106:offset + 108], "little")
            cursor = offset + 108
            note_lengths = []
            for _ in range(note_count):
                if cursor >= len(trailer):
                    break
                length = trailer[cursor]
                cursor += 1
                if length == 0xff and cursor + 2 <= len(trailer):
                    length = int.from_bytes(trailer[cursor:cursor + 2], "little")
                    cursor += 2
                note_lengths.append(length)
                cursor += length
            candidates.append({
                "offset": offset,
                "tail_bytes": len(trailer) - offset,
                "score": int.from_bytes(trailer[offset + 58:offset + 62], "little"),
                "student_nonpreferred": int.from_bytes(
                    trailer[offset + 86:offset + 90], "little"),
                "lesson_nonpreferred": int.from_bytes(
                    trailer[offset + 98:offset + 102], "little"),
                "scan_tag": int.from_bytes(trailer[offset + 102:offset + 104], "little"),
                "teacher_daily_imbalance": int.from_bytes(
                    trailer[offset + 104:offset + 106], "little"),
                "note_count": note_count,
                "note_lengths": note_lengths,
                "notes_consume_tail": len(note_lengths) == note_count and cursor == len(trailer),
            })
        start = offset + 1
    declared = int.from_bytes(trailer[:4], "little") if len(trailer) >= 4 else None
    return {
        "file": str(path),
        "header": payload[:3].hex(),
        "trailer_length_valid": declared == len(trailer) - 4,
        "candidates": candidates,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("paths", nargs="+", type=Path)
    args = parser.parse_args()
    results = []
    for path in args.paths:
        try:
            results.append(scan(path))
        except (OSError, ValueError) as exc:
            results.append({"file": str(path), "error": str(exc)})
    print(json.dumps(results, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
