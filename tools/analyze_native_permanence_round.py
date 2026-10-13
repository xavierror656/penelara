"""Check a Trace-PermanenceRound capture against the 0x458df0 rounding branch."""

import argparse
import csv
import json
from pathlib import Path


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("trace", type=Path)
args = parser.parse_args()

raw_trace = args.trace.read_bytes()
encoding = "utf-16" if raw_trace.startswith((b"\xff\xfe", b"\xfe\xff")) else "utf-8-sig"
rows = list(csv.DictReader(raw_trace.decode(encoding).splitlines()))
if len(rows) < 2:
    raise ValueError("expected two breakpoint rows")
first, second = rows[:2]
if ([int(first["phase"]), int(second["phase"])] != [1, 2]
        or first["thread"] != second["thread"]
        or first["row"] != second["row"]
        or first["context"] != second["context"]
        or [int(first["mode"]), int(second["mode"])] not in ([0, 1], [1, 1])):
    raise ValueError("breakpoints do not describe the same controlled calculation")
raw = int(first["ecx"])
observed = int(second["ecx"])
original = int(first["original_ecx"]) if first.get("original_ecx") else raw
if raw < 0:
    raise ValueError("negative subtotal needs the signed division path")
predicted = (raw + 2) // 4
if observed != predicted:
    raise ValueError(f"observed {observed}, predicted {predicted}")
print(json.dumps({"teacherRow": int(first["row"]), "context": first["context"],
                  "rawSubtotal": raw, "originalSubtotal": original,
                  "subtotalForced": raw != original,
                  "modeForced": int(first["mode"]) == 0,
                  "observedRounded": observed,
                  "predictedRounded": predicted}))
