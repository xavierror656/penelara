"""Show grid occupancy and synthetic occupancy masks from a frozen MotHor snapshot."""

import argparse
from pathlib import Path
import struct


def u32(data: bytes, offset: int) -> int:
    return struct.unpack_from("<I", data, offset)[0]


def inspect(prefix: Path, frame: int, teacher: int, positions: int, synthetic_enabled: bool) -> None:
    stem = str(prefix)
    grid_path = Path(f"{stem}-table-{0x37e8 + 4 * frame:04x}.bin")
    mask_path = Path(f"{stem}-table-{0x3770 + 4 * frame:04x}.bin")
    if not grid_path.exists() or not mask_path.exists():
        raise ValueError(f"frame {frame} has no captured grid/mask tables in {stem}")
    grid = grid_path.read_bytes()
    mask = mask_path.read_bytes()
    grid_start = teacher * 0xC00
    mask_start = teacher * 0x300
    if len(grid) < grid_start + 4 * (6 * (positions - 1) + 5):
        raise ValueError("grid dump does not cover requested teacher/positions")
    if len(mask) < mask_start + 6 * (positions - 1) + 5:
        raise ValueError("mask dump does not cover requested teacher/positions")
    for day in range(5):
        cells = []
        masks = []
        for pos in range(positions):
            index = 6 * pos + day
            pointer = u32(grid, grid_start + 4 * index)
            level = struct.unpack_from("<b", mask, mask_start + index)[0]
            cells.append("#" if pointer else ("+" if synthetic_enabled and level >= 1 else "."))
            masks.append(level)
        print(f"frame={frame} teacher={teacher} day={day} {''.join(cells)} mask={masks}")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("prefix", type=Path)
    parser.add_argument("--frame", type=int, choices=(0, 1), default=0)
    parser.add_argument("--teacher", type=int, default=1)
    parser.add_argument("--positions", type=int, default=8)
    args = parser.parse_args()
    if args.teacher < 0 or args.positions < 1:
        parser.error("teacher must be nonnegative and positions must be positive")
    context = Path(f"{args.prefix}-context.bin").read_bytes()
    app_header = Path(f"{args.prefix}-app-header.bin").read_bytes()
    context_mode = u32(context, 8)
    app_e4 = u32(app_header, 0xE4)
    print(
        "context+8=", context_mode,
        "context+4814=", u32(context, 0x4814),
        "app+e4=", app_e4,
    )
    print("#: pointer, +: null pointer treated as occupied by mask, .: other null pointer")
    inspect(args.prefix, args.frame, args.teacher, args.positions, bool(app_e4) and context_mode == 0)


if __name__ == "__main__":
    main()
