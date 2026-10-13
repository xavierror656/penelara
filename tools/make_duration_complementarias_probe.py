"""Change only the four DIR complementarias in a GHC duration-T fixture."""

import argparse
import gzip
import re
import xml.etree.ElementTree as ET
from pathlib import Path


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("source", type=Path)
    parser.add_argument("destination", type=Path)
    parser.add_argument("--action", choices=("three", "remove", "one"), required=True)
    parser.add_argument("--weekly-count", type=int)
    parser.add_argument("--max-daily", type=int)
    parser.add_argument("--no-consecutive", action="store_true")
    parser.add_argument("--teacher", choices=("Javier", "Alfonso"))
    parser.add_argument("--duration-normal", action="store_true")
    args = parser.parse_args()
    if args.weekly_count is not None and (args.action != "one" or not 1 <= args.weekly_count <= 9):
        raise ValueError("--weekly-count requires --action one and a value from 1 to 9")
    if args.max_daily is not None and (args.action != "one" or not 1 <= args.max_daily <= 9):
        raise ValueError("--max-daily requires --action one and a value from 1 to 9")
    if args.no_consecutive and args.action != "one":
        raise ValueError("--no-consecutive requires --action one")
    if args.teacher is not None and args.action != "one":
        raise ValueError("--teacher requires --action one")
    if args.duration_normal and args.action != "one":
        raise ValueError("--duration-normal requires --action one")
    payload = gzip.decompress(args.source.read_bytes())
    start = payload.index(b"<?xml")
    end = payload.index(b"</datosGHC>") + len(b"</datosGHC>") + 1
    if (start != 7 or payload[0] != ord("M") or
            int.from_bytes(payload[3:7], "little") != end - start):
        raise ValueError("unexpected GHC XML container")
    xml = payload[start:end]
    pattern = rb"(<complementarias>)(.*?)(</complementarias>)"
    match = re.search(pattern, xml, re.DOTALL)
    if not match or len(re.findall(pattern, xml, re.DOTALL)) != 1:
        raise ValueError("expected one complementarias section")
    block = match.group(2)
    if block.count(b"<complementaria subMarco=\"A\">") != 4:
        raise ValueError("expected four frame-A complementary duties")
    if block.count(b"<intervalosSemanales>5</intervalosSemanales>") != 4:
        raise ValueError("expected four five-session complementary duties")
    if args.action == "three":
        replacement = block.replace(b"<intervalosSemanales>5</intervalosSemanales>",
                                    b"<intervalosSemanales>3</intervalosSemanales>")
    elif args.action == "one":
        duties = re.findall(rb'<complementaria subMarco="A">.*?</complementaria>',
                          block, re.DOTALL)
        if len(duties) != 4:
            raise ValueError("expected four complete complementary duty elements")
        first = duties[0]
        if args.weekly_count is not None:
            first = first.replace(
                b"<intervalosSemanales>5</intervalosSemanales>",
                f"<intervalosSemanales>{args.weekly_count}</intervalosSemanales>".encode(),
            )
        if args.max_daily is not None:
            first = first.replace(
                b"<maxIntervalosDiarios>3</maxIntervalosDiarios>",
                f"<maxIntervalosDiarios>{args.max_daily}</maxIntervalosDiarios>".encode(),
            )
        if args.no_consecutive:
            first = first.replace(
                b"<noConsecutivos>false</noConsecutivos>",
                b"<noConsecutivos>true</noConsecutivos>",
            )
        if args.teacher is not None:
            first = first.replace(
                b"<profesor>Javier</profesor>",
                f"<profesor>{args.teacher}</profesor>".encode("utf-8"),
            )
        replacement = b"\n\t\t" + first + b"\n\t"
    else:
        replacement = b"\n\t"
    xml = xml[:match.start(2)] + replacement + xml[match.end(2):]
    if args.duration_normal:
        if xml.count(b"<duracion>T</duracion>") != 1:
            raise ValueError("expected exactly one T duration")
        xml = xml.replace(b"<duracion>T</duracion>", b"<duracion>1</duracion>")
    root = ET.fromstring(xml)
    complementarias = list(root.iter("complementaria"))
    expected = {"three": 4, "remove": 0, "one": 1}[args.action]
    if len(complementarias) != expected:
        raise ValueError(f"expected {expected} complementarias")
    if args.weekly_count is not None and complementarias[0].findtext("intervalosSemanales") != str(args.weekly_count):
        raise ValueError("weekly-count replacement failed")
    if args.max_daily is not None and complementarias[0].findtext("maxIntervalosDiarios") != str(args.max_daily):
        raise ValueError("max-daily replacement failed")
    if args.no_consecutive and complementarias[0].findtext("noConsecutivos") != "true":
        raise ValueError("no-consecutive replacement failed")
    if args.teacher is not None and complementarias[0].findtext("profesor") != args.teacher:
        raise ValueError("teacher replacement failed")
    count_t = sum(1 for t in root.iter("tramo") if t.findtext("duracion") == "T")
    if count_t != (0 if args.duration_normal else 1):
        raise ValueError("unexpected T duration count in source XML")
    updated = bytearray(payload[:start] + xml + payload[end:])
    updated[3:7] = len(xml).to_bytes(4, "little")
    compressed = bytearray(gzip.compress(updated, mtime=0))
    compressed[9] = 10
    args.destination.write_bytes(compressed)
    print(f"{args.destination}: action={args.action}, complementarias={expected}")


if __name__ == "__main__":
    main()
