"""Replay the covered teacher-scan branches in a frozen MotHor snapshot.

This covers context+8=0 and teacher records with +0x5b=0. The two criterion
components, the daily count increment, and the span excess come from
0x459b40–0x459f83. The caller's permanence subtotal is from
0x458dbe–0x458e09, for context+8=0.
The per-teacher predictions can be checked against Trace-GapReturn.c output
when both use the same frozen native context.
"""

import argparse
from pathlib import Path
import struct
import json


def u32(data: bytes, offset: int) -> int:
    return struct.unpack_from("<I", data, offset)[0]


def spans(context: bytes, day: int, split: bool) -> list[tuple[int, int]]:
    end = u32(context, 0x47FC + 4 * day)
    if not split:
        return [(0, end)]
    boundary = u32(context, 0x47E4 + 4 * day)
    return [(0, boundary), (boundary, end)]


def bounded_empty_runs(cells: list[bool]) -> list[tuple[int, int]]:
    """Return (start, length) only when occupied cells bound the empty run."""
    runs = []
    seen_occupied = False
    empty_start = 0
    empty_count = 0
    for pos, occupied in enumerate(cells):
        if occupied:
            if seen_occupied and empty_count:
                runs.append((empty_start, empty_count))
            seen_occupied = True
            empty_count = 0
        elif seen_occupied:
            if empty_count == 0:
                empty_start = pos
            empty_count += 1
    return runs


def load_objects(path: Path) -> dict[int, bytes]:
    data = path.read_bytes()
    if len(data) % 0x284:
        raise ValueError("object dump has a partial record")
    return {u32(data, offset): data[offset + 4 : offset + 0x284]
            for offset in range(0, len(data), 0x284)}


def load_records(path: Path) -> list[dict]:
    raw = path.read_bytes()
    if len(raw) % 0xBC:
        raise ValueError("scoring record dump has a partial record")
    return [{"index": i, "active": struct.unpack_from("<i", raw, i * 0xBC + 0x6C)[0],
             "kind": struct.unpack_from("<i", raw, i * 0xBC + 0x90)[0],
             "weeklyNonStrictPermanenceLimit": struct.unpack_from(
                 "<i", raw, i * 0xBC + 0x1C)[0],
             "dailyNonStrictPermanenceLimit": struct.unpack_from(
                 "<i", raw, i * 0xBC + 0x24)[0],
             "maximas": struct.unpack_from("<b", raw, i * 0xBC + 0x28)[0],
             "gapMode5b": raw[i * 0xBC + 0x5B]}
            for i in range(len(raw) // 0xBC)]


def analyze(prefix: Path) -> dict:
    stem = str(prefix)
    context = Path(f"{stem}-context.bin").read_bytes()
    header = Path(f"{stem}-app-header.bin").read_bytes()
    criterion = Path(f"{stem}-criterion-bytes.bin").read_bytes()
    records = load_records(Path(f"{stem}-scoring-records-00.bin"))
    model = Path(f"{stem}-model.bin").read_bytes()
    if len(model) <= 0xCFB0C:
        raise ValueError("model dump is too short for the options byte")
    if u32(context, 8) != 0:
        raise ValueError("only context+8=0 is covered")
    frame = u32(context, 0xC)
    if frame != 0:
        raise ValueError("this capture layout expects frame 0")
    grid = Path(f"{stem}-table-37e8.bin").read_bytes()
    objects = load_objects(Path(f"{stem}-objects.bin"))
    synthetic = u32(header, 0xE4) != 0
    mask_path = Path(f"{stem}-table-3770.bin")
    if synthetic and not mask_path.exists():
        raise ValueError("synthetic occupancy requires the +0x3770 mask dump")
    mask = mask_path.read_bytes() if synthetic else b""
    options_byte = model[0xCFB0C]
    split = u32(context, 0x4814) != 0 and not (options_byte & 2)
    weight = criterion[5]
    run_weight = criterion[12]
    permanence_weight = criterion[13]
    output = []
    total = 0
    unmarked_total = 0
    class_run_total = 0
    daily_count_increment = 0
    per_teacher = {}
    per_teacher_daily_count = {}
    per_teacher_span_excess = {}
    per_teacher_calls = {}
    per_teacher_weekly_excess = {}
    per_teacher_permanence_penalty = {}
    permanence_penalty = 0
    for record in records:
        if record["kind"] != 5 or record["active"] == 0:
            continue
        if record["gapMode5b"] != 0:
            raise ValueError(f"record {record['index']} has an uncovered +0x5b mode")
        row = record["index"]
        row_cost = 0
        row_daily_count = 0
        row_span_excess = 0
        call_count = 2 if split else 1
        calls = [{"return": 0, "dailyCount": 0, "spanExcess": 0}
                 for _ in range(call_count)]
        for day in range(u32(header, 0xE0)):
            for call_index, (start, end) in enumerate(spans(context, day, split)):
                if end < start or end > 128:
                    raise ValueError(f"invalid range {start}..{end} for day {day}")
                occupied = []
                without_marks = []
                marked_nulls = []
                grouped = []
                for pos in range(start, end):
                    index = 6 * pos + day
                    pointer = u32(grid, row * 0xC00 + 4 * index)
                    mask_value = (struct.unpack_from("<b", mask, row * 0x300 + index)[0]
                                  if synthetic else 0)
                    marked = not pointer and mask_value >= 1
                    occupied.append(bool(pointer) or marked)
                    without_marks.append(bool(pointer))
                    obj = objects.get(pointer) if pointer else None
                    if pointer and obj is None:
                        raise ValueError(f"missing object 0x{pointer:08x}")
                    grouped.append(bool(obj and struct.unpack_from("<H", obj, 0x10)[0]))
                    if marked:
                        marked_nulls.append(pos)
                runs = [(start + pos, n) for pos, n in bounded_empty_runs(occupied)]
                plain_runs = bounded_empty_runs(without_marks)
                cost = sum(weight * n * (n + 1) // 2 for _, n in runs)
                plain_cost = sum(weight * n * (n + 1) // 2 for _, n in plain_runs)
                streak = 0
                run_cost = 0
                for is_grouped in grouped:
                    streak = streak + 1 if is_grouped else 0
                    run_cost += run_weight * max(0, streak - record["maximas"])
                occupied_positions = [pos for pos, value in enumerate(occupied) if value]
                span_count = (occupied_positions[-1] - occupied_positions[0] + 1
                              if occupied_positions else 0)
                if runs or marked_nulls or run_cost:
                    output.append({"teacher": row, "day": day, "range": [start, end],
                                   "runs": runs, "markedNulls": marked_nulls,
                                   "triangularCost": cost, "withoutMarks": plain_cost,
                                   "classRunCost": run_cost, "dailyCountIncrement": span_count})
                total += cost
                row_cost += cost + run_cost
                unmarked_total += plain_cost
                class_run_total += run_cost
                daily_count_increment += span_count
                row_daily_count += span_count
                excess = max(0, span_count - record["dailyNonStrictPermanenceLimit"])
                row_span_excess += excess
                calls[call_index]["return"] += cost + run_cost
                calls[call_index]["dailyCount"] += span_count
                calls[call_index]["spanExcess"] += excess
        per_teacher[str(row)] = row_cost
        per_teacher_daily_count[str(row)] = row_daily_count
        per_teacher_span_excess[str(row)] = row_span_excess
        per_teacher_calls[str(row)] = calls
        weekly_excess = max(0, row_daily_count - record["weeklyNonStrictPermanenceLimit"])
        per_teacher_weekly_excess[str(row)] = weekly_excess
        penalty = permanence_weight * (weekly_excess + row_span_excess)
        per_teacher_permanence_penalty[str(row)] = penalty
        permanence_penalty += penalty
    return {"snapshot": stem, "gapWeight": weight, "runWeight": run_weight,
            "permanenceWeight": permanence_weight,
            "optionsByteCFB0C": options_byte,
            "splitAtNoon": split,
            "teacherRows": sum(r["kind"] == 5 and r["active"] != 0 for r in records),
            "triangularComponent": total, "withoutMarks": unmarked_total,
            "classRunComponent": class_run_total,
            "predictedReturn": total + class_run_total,
            "perTeacherPredictedReturn": per_teacher,
            "perTeacherDailyCountIncrement": per_teacher_daily_count,
            "perTeacherSpanExcess": per_teacher_span_excess,
            "perTeacherCalls": per_teacher_calls,
            "perTeacherWeeklyExcess": per_teacher_weekly_excess,
            "perTeacherPermanencePenalty": per_teacher_permanence_penalty,
            "permanencePenalty": permanence_penalty,
            "dailyCountIncrement": daily_count_increment,
            "rows": output}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("prefix", type=Path)
    args = parser.parse_args()
    print(json.dumps(analyze(args.prefix), indent=2))


if __name__ == "__main__":
    main()
