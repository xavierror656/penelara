"""Compare reconstructed daily bounds with saved MotHor scoring records."""

import argparse
import struct
from pathlib import Path

STRIDE = 0xBC
ADJUSTMENT = {0: 0, 1: -1, 2: 1}
TABLES = (
    ("a", 0x68, 0x80, 0x70, 0x81, 0x62, 0x86),
    ("b", 0x6C, 0x82, 0x71, 0x83, 0x64, 0x88),
)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("capture_dir", type=Path, help="directory with *-scoring-records-00.bin")
    args = parser.parse_args()

    files = sorted(
        p
        for p in args.capture_dir.glob("*-scoring-records-00.bin")
        if p.name.startswith(
            ("daily", "alfonso", "reunionbounds", "raquelonlymeeting", "raquelmeeting")
        )
    )
    if not files:
        parser.error("no matching scoring-record dumps")

    checked = 0
    mismatches = []
    for path in files:
        raw = path.read_bytes()
        if len(raw) % STRIDE:
            raise ValueError(f"{path}: byte count is not a multiple of {STRIDE}")
        scale = 4 if path.name.startswith("dailytmode") else 1
        for index in range(len(raw) // STRIDE):
            row = raw[index * STRIDE : (index + 1) * STRIDE]
            if struct.unpack_from("<i", row, 0x90)[0] != 5:
                continue
            for table, load_at, days_at, max_at, min_at, max_adj_at, min_adj_at in TABLES:
                load = struct.unpack_from("<i", row, load_at)[0]
                if load < 0:
                    raise ValueError(f"{path}: negative load for record {index}")
                days = max(row[days_at], 1)
                scaled_load = (load + scale // 2) // scale if scale == 4 else load
                numerator = scaled_load - 1
                quotient = numerator // days if numerator >= 0 else -((-numerator) // days)
                base = (quotient + 2) * scale
                maximum = max(base + ADJUSTMENT[row[max_adj_at]] * scale, row[0x72])
                minimum = max(0, base - 3 * scale + ADJUSTMENT[row[min_adj_at]] * scale)
                observed = (
                    struct.unpack_from("<b", row, min_at)[0],
                    struct.unpack_from("<b", row, max_at)[0],
                )
                checked += 1
                if observed != (minimum, maximum):
                    mismatches.append((path.name, index, table, observed, (minimum, maximum)))

    print(f"files={len(files)} bound_pairs={checked} mismatches={len(mismatches)}")
    for mismatch in mismatches:
        print("mismatch:", *mismatch)
    return int(bool(mismatches))


if __name__ == "__main__":
    raise SystemExit(main())
