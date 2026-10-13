"""Summarize the ordered MotHor XML callback trace and check its list state."""

import argparse
import csv
import json
from pathlib import Path


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("trace", type=Path)
args = parser.parse_args()

payload = args.trace.read_bytes()
encoding = "utf-16" if payload.startswith((b"\xff\xfe", b"\xfe\xff")) else "utf-8-sig"
rows = [row for row in csv.DictReader(payload.decode(encoding).splitlines())
        if row.get("point") in ("tramo", "marcos", "mode")]
tramos = [row for row in rows if row["point"] == "tramo"]
parents = [row for row in rows if row["point"] == "marcos"]
modes = [row for row in rows if row["point"] == "mode"]
if not tramos or len(parents) != 1 or len(modes) != 1:
    raise ValueError("expected tramo callbacks followed by one parent and one mode event")
parent, mode = parents[0], modes[0]
if ([row["point"] for row in rows] != ["tramo"] * len(tramos) + ["marcos", "mode"]
        or len({row["pid"] for row in rows}) != 1
        or len({row["thread"] for row in rows}) != 1
        or parent["object"] != mode["object"]
        or int(parent["listCount"]) != len(tramos)
        or int(mode["listCount"]) != len(tramos)):
    raise ValueError("callback order, process or list count does not match")
nonzero = int(mode["nonzeroDc"])
to_write = int(mode["modeToWrite"])
if to_write != int(nonzero > 0):
    raise ValueError("mode byte does not match the observed node values")
print(json.dumps({"pid": int(mode["pid"]), "tramoCallbacks": len(tramos),
                  "parentListCount": int(parent["listCount"]),
                  "nonzeroNodeDc": nonzero, "modeToWrite": to_write,
                  "allTramoFieldsZero": all(int(row["field1c470"]) == 0
                                             for row in tramos)}))
