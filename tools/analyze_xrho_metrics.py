"""Align MotHor replay counters with the serialized XRHO trailer."""

import argparse
import gzip
import hashlib
import json
from pathlib import Path


def trailer(path: Path) -> bytes:
    payload = gzip.decompress(path.read_bytes())
    end = payload.index(b"</datosGHC>") + len(b"</datosGHC>") + 1
    return payload[end:]


def result_tail(path: Path) -> bytes:
    data = trailer(path)
    marker = b"DESKTOP-"
    start = data.find(marker)
    if start < 1:
        raise ValueError(f"machine marker absent: {path}")
    length = data[start - 1]
    if length < len(marker):
        raise ValueError(f"invalid machine string length: {path}")
    return data[start + length:]


def inspect_log(path: Path) -> dict:
    run = path.name.removesuffix("-replay.log")
    files = list((path.parent / "core" / f"run-{run}").glob("*.xrho"))
    if len(files) != 1:
        raise ValueError(f"expected one XRHO for {run}, found {len(files)}")
    result_path = files[0]
    digest = hashlib.sha256(result_path.read_bytes()).hexdigest().upper()
    snapshots = [json.loads(line) for line in path.read_text(encoding="utf-8-sig").splitlines() if line.startswith("{")]
    matching = [entry for entry in snapshots if (entry.get("result") or {}).get("sha256") == digest]
    if not matching:
        raise ValueError(f"no snapshot matches XRHO hash for {run}")
    memory = matching[-1].get("engineMemory") or {}
    tail = result_tail(result_path)
    return {
        "run": run,
        "file": str(result_path),
        "tail_bytes": len(tail),
        "weight": memory.get("bestCompleteWeight"),
        "counters": memory.get("resultCounters"),
        "words16": {str(offset): int.from_bytes(tail[offset:offset + 2], "little")
                    for offset in range(0, min(len(tail), 108), 2)},
        "candidate_fields32": {str(offset): int.from_bytes(tail[offset:offset + 4], "little")
                               for offset in range(58, 103, 4)
                               if len(tail) >= offset + 4},
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("logs", nargs="*", type=Path)
    parser.add_argument("--discover", type=Path, help="scan matching replay logs in this directory")
    parser.add_argument("--verify", action="store_true", help="cross-check identified fields against matching MotHor memory snapshots")
    args = parser.parse_args()
    paths = list(args.logs)
    if args.discover is not None:
        paths.extend(sorted(args.discover.glob("*-replay.log")))
    results = []
    for path in paths:
        try:
            results.append(inspect_log(path))
        except (ValueError, OSError, KeyError):
            if args.discover is None:
                raise
    if args.verify:
        fields = {
            "studentGaps": 70,
            "teacherGaps": 74,
            "teacherExtraPresence": 78,
            "roomNonpreferred": 82,
            "studentNonpreferred": 86,
            "teacherNonpreferred": 90,
            "subjectNonpreferred": 94,
            "lessonNonpreferred": 98,
        }
        complete = [r for r in results if r["weight"] is not None
                    and r["weight"] != 0xffffffff and r["counters"] is not None]
        if not complete:
            raise SystemExit("No complete result has matching MotHor counters")
        mismatches = []
        for result in complete:
            if result["candidate_fields32"].get("58") != result["weight"]:
                mismatches.append([result["run"], "bestCompleteWeight"])
            for field, offset in fields.items():
                if result["candidate_fields32"].get(str(offset)) != result["counters"].get(field):
                    mismatches.append([result["run"], field])
            if result["words16"].get("104") != result["counters"].get("teacherDailyImbalance"):
                mismatches.append([result["run"], "teacherDailyImbalance"])
        print(json.dumps({
            "compared_results": len(complete),
            "field_matches": 10 * len(complete) - len(mismatches),
            "field_comparisons": 10 * len(complete),
            "mismatches": mismatches,
            "nonzero_snapshot_counts": {
                "studentNonpreferred_at_86_nonzero": sum(
                    r["counters"].get("studentNonpreferred", 0) != 0 for r in complete),
                "lessonNonpreferred_at_98_nonzero": sum(
                    r["counters"].get("lessonNonpreferred", 0) != 0 for r in complete),
                "teacherDailyImbalance_at_104_nonzero": sum(
                    r["counters"].get("teacherDailyImbalance", 0) != 0 for r in complete),
            },
        }, ensure_ascii=False, indent=2))
        if mismatches:
            raise SystemExit(1)
        return
    print(json.dumps(results, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
