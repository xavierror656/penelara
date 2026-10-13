"""Print optativaSolapada across source, planner temporary, and engine result."""

import argparse
import gzip
import xml.etree.ElementTree as ET
from pathlib import Path


def read_value(path: Path) -> str:
    data = gzip.decompress(path.read_bytes())
    start = data.index(b"<?xml")
    end = data.index(b"</datosGHC>") + len(b"</datosGHC>")
    root = ET.fromstring(data[start:end])
    values = [node.text for node in root.iter("optativaSolapada")]
    if len(values) != 1:
        raise ValueError(f"expected one optativaSolapada in {path}")
    return values[0]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("core", type=Path)
    parser.add_argument("values", type=int, nargs="+")
    args = parser.parse_args()
    for value in args.values:
        stem = f"ejemploDosMarcosHorariosOptativaPena{value}"
        run = f"optativa-pena{value}-01"
        root = args.core / f"run-{run}"
        temp = root / f"temp-{run}"
        paths = [args.core / f"{stem}.xghc", temp / "auxghcompP0",
                 temp / "auxghcompM0", root / f"{stem}R1.xrho"]
        print(value, [(path.name, read_value(path)) for path in paths if path.exists()])


if __name__ == "__main__":
    main()
