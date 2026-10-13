"""Reconcile ordinal-indexed MotHor object dumps from runs of one project."""

import argparse
import collections
import json
import struct
from pathlib import Path


def read_dump(path):
    data = path.read_bytes()
    if len(data) % 0x284:
        raise ValueError(f"{path} has a partial object record")
    result = {}
    for offset in range(0, len(data), 0x284):
        pointer = struct.unpack_from("<I", data, offset)[0]
        raw = data[offset + 4:offset + 0x284]
        unit, source = struct.unpack_from("<HH", raw)
        if unit in result:
            raise ValueError(f"{path} repeats unit {unit}")
        result[unit] = {
            "pointer": pointer,
            "source": source,
            "teacherIndex": struct.unpack_from("<H", raw, 8)[0],
            "frame": raw[0x270],
            "marker10": raw[0x10],
            "marker1f": raw[0x1F],
        }
    return result


def identity(row):
    return tuple(row[field] for field in
                 ("source", "teacherIndex", "frame", "marker10", "marker1f"))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("baseline", type=Path)
    parser.add_argument("followup", type=Path, nargs="+")
    parser.add_argument("--first-aux-source", type=int, required=True)
    args = parser.parse_args()

    baseline = read_dump(args.baseline)
    followup = {}
    for path in args.followup:
        for unit, row in read_dump(path).items():
            if unit in followup:
                raise ValueError(f"follow-up dumps repeat unit {unit}")
            followup[unit] = row
    shared = baseline.keys() & followup.keys()
    mismatches = [unit for unit in sorted(shared)
                  if identity(baseline[unit]) != identity(followup[unit])]
    if mismatches:
        raise ValueError(f"object identity changed between runs: {mismatches}")

    objects = baseline | followup
    missing = sorted(set(range(min(objects), max(objects) + 1)) - objects.keys())
    companions = []
    for unit, row in sorted(objects.items()):
        if (row["marker10"], row["marker1f"]) != (4, 1):
            continue
        previous = objects.get(unit - 1)
        paired = (previous is not None and
                  (previous["source"], previous["teacherIndex"], previous["frame"]) ==
                  (row["source"], row["teacherIndex"], row["frame"]) and
                  (previous["marker10"], previous["marker1f"]) == (3, 2))
        companions.append({"unit": unit, "previousUnitPaired": paired})

    auxiliary = collections.Counter(row["source"] for row in objects.values()
                                    if row["source"] >= args.first_aux_source)
    markers = collections.Counter((row["marker10"], row["marker1f"])
                                  for row in objects.values())
    print(json.dumps({
        "baselineCount": len(baseline),
        "followupCount": len(followup),
        "sharedIdentityMatches": len(shared),
        "unitRange": [min(objects), max(objects)],
        "reconstructedCount": len(objects),
        "missingUnitIndices": missing,
        "unitsAbsentFromBaseline": sorted(objects.keys() - baseline.keys()),
        "lectiveUnitCount": len(objects) - sum(auxiliary.values()),
        "auxiliaryUnitCount": sum(auxiliary.values()),
        "auxiliarySourceCounts": dict(sorted(auxiliary.items())),
        "markerCounts": {f"{first},{second}": count
                         for (first, second), count in sorted(markers.items())},
        "companionCandidates": companions,
    }, indent=2))


if __name__ == "__main__":
    main()
