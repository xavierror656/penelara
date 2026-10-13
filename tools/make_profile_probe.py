"""Build GHC probes by changing profile, optativas, guardias, or departamentos."""

import argparse
import gzip
import re
import xml.etree.ElementTree as ET
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("destination", type=Path)
    parser.add_argument("--profile", choices=("secundaria", "primaria", "universidad", "conservatorio"))
    parser.add_argument("--empty-optativas", action="store_true")
    parser.add_argument("--clear-guardias", action="store_true")
    parser.add_argument("--departments", type=int, choices=(1, 2))
    args = parser.parse_args()

    if (args.profile is None and not args.empty_optativas and
            not args.clear_guardias and args.departments is None):
        parser.error("select at least one change")
    if args.destination.exists():
        raise FileExistsError(args.destination)

    payload = args.source.read_bytes()
    plain = gzip.decompress(payload)
    start = plain.index(b"<?xml")
    close = plain.index(b"</datosGHC>")
    end = close + len(b"</datosGHC>") + 1
    if plain[:1] != b"M" or start != 7 or plain[end - 1:end] != b"\n":
        raise ValueError("unexpected GHC container framing")
    if int.from_bytes(plain[3:7], "little") != end - start:
        raise ValueError("GHC XML length header mismatch")

    root = ET.fromstring(plain[start:end])
    updated = bytearray(plain)
    source_profile = root.find("otros/perfil")
    if args.profile is not None:
        if source_profile is None or not source_profile.text:
            raise ValueError("source has no profile")
        old = (f"<perfil>{source_profile.text}</perfil>").encode("ascii")
        new = (f"<perfil>{args.profile}</perfil>").encode("ascii")
        if updated.count(old) != 1:
            raise ValueError("profile tag is not unique")
        updated = updated.replace(old, new, 1)
    if args.empty_optativas:
        if root.find("optativas") is not None:
            raise ValueError("source already has optativas")
        close = updated.index(b"</datosGHC>")
        updated[close:close] = b"\t<optativas/>\n"
    if args.clear_guardias:
        matches = list(re.finditer(rb"<guardias>.*?</guardias>", updated, re.DOTALL))
        if len(matches) != 1:
            raise ValueError("expected exactly one guardias section")
        updated[matches[0].start():matches[0].end()] = b"<guardias/>"
    if args.departments is not None:
        definitions = root.findall("departamentos/departamento")
        if len(definitions) <= args.departments:
            raise ValueError("source does not have enough departments to reduce")
        names = [node.findtext("nombre") for node in definitions]
        if any(not name for name in names):
            raise ValueError("department without a name")
        matches = list(re.finditer(rb"<departamentos>.*?</departamentos>", updated, re.DOTALL))
        if len(matches) != 1:
            raise ValueError("expected exactly one departamentos section")
        block = updated[matches[0].start():matches[0].end()]
        definitions_raw = re.findall(rb"<departamento>.*?</departamento>", block, re.DOTALL)
        if len(definitions_raw) != len(definitions):
            raise ValueError("department definitions do not match source XML")
        retained = names[:args.departments]
        replacement = b"<departamentos>\n" + b"\n".join(definitions_raw[:args.departments]) + b"\n</departamentos>"
        updated[matches[0].start():matches[0].end()] = replacement
        for removed in names[args.departments:]:
            old = b"<departamento>" + removed.encode("latin1") + b"</departamento>"
            new = b"<departamento>" + retained[0].encode("latin1") + b"</departamento>"
            updated = updated.replace(old, new)

    xml_end = updated.index(b"</datosGHC>") + len(b"</datosGHC>") + 1
    ET.fromstring(updated[start:xml_end])
    updated[3:7] = (xml_end - start).to_bytes(4, "little")
    compressed = bytearray(gzip.compress(updated, mtime=0))
    compressed[9] = 10
    args.destination.write_bytes(compressed)
    effective_profile = args.profile or (source_profile.text if source_profile is not None else None)
    print(f"{args.destination}: profile={effective_profile}, empty_optativas={args.empty_optativas}, clear_guardias={args.clear_guardias}, departments={args.departments}")


if __name__ == "__main__":
    main()
