"""Show profile and selected XML counts across an isolated GHC replay."""

import argparse
import gzip
import xml.etree.ElementTree as ET
from pathlib import Path


def inspect(path: Path) -> str:
    if not path.exists():
        return "missing"
    data = gzip.decompress(path.read_bytes())
    start = data.index(b"<?xml")
    end = data.index(b"</datosGHC>") + len(b"</datosGHC>")
    root = ET.fromstring(data[start:end])
    profile = root.findtext("otros/perfil")
    department_refs = [node.text for node in root.iter("departamento") if len(node) == 0]
    return (
        f"profile={profile}, optativas={len(root.findall('optativas/optativa'))}, "
        f"guardias={len(root.findall('guardias/guardia'))}, "
        f"departamentos={len(root.findall('departamentos/departamento'))}, "
        f"grupos={len(root.findall('grupos/grupo'))}, "
        f"department_refs={len(department_refs)}, "
        f"department_ref_values={sorted(set(department_refs))}"
    )


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("core", type=Path)
    parser.add_argument("run")
    parser.add_argument("input_name")
    args = parser.parse_args()
    run_root = args.core / f"run-{args.run}"
    temp = run_root / f"temp-{args.run}"
    stem = Path(args.input_name).stem
    paths = (
        args.core / args.input_name,
        temp / "auxghcompP0",
        temp / "auxghcompM0",
        run_root / f"{stem}R1.xrho",
    )
    for path in paths:
        print(f"{path}: {inspect(path)}")


if __name__ == "__main__":
    main()
