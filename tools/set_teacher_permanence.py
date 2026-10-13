"""Change one teacher's non-strict permanence limit in a GHC fixture."""

import argparse
import gzip
from pathlib import Path
import re


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("source", type=Path)
parser.add_argument("destination", type=Path)
parser.add_argument("teacher_index", type=int)
parser.add_argument("value", help="new integer value or 'absent' to remove the element")
parser.add_argument("--period", choices=("daily", "weekly"), default="daily")
parser.add_argument("--strict", action="store_true", help="select estricto=true instead of false")
args = parser.parse_args()

if args.teacher_index < 0:
    parser.error("teacher_index must be nonnegative")
if args.value != "absent":
    try:
        new_value = int(args.value)
    except ValueError:
        parser.error("value must be an integer or 'absent'")
    if not 0 <= new_value <= 255:
        parser.error("value must fit one byte")

payload = gzip.decompress(args.source.read_bytes())
start = payload.index(b"<?xml")
end = payload.index(b"</datosGHC>") + len(b"</datosGHC>") + 1
if start != 7 or payload[0] != ord("M") or payload[end - 1 : end] != b"\n":
    raise ValueError("unexpected GHC XML container")
if int.from_bytes(payload[3:7], "little") != end - start:
    raise ValueError("GHC XML length header mismatch")

xml = payload[start:end]
teachers = list(re.finditer(rb"<profesor>.*?</profesor>", xml, re.DOTALL))
if args.teacher_index >= len(teachers):
    raise ValueError(f"only {len(teachers)} teachers found")
teacher = teachers[args.teacher_index]
block = teacher.group()
tag = (b"intervalosDePermanenciaDiarios" if args.period == "daily"
       else b"intervalosDePermanenciaSemanales")
strict = b"true" if args.strict else b"false"
pattern = b'(<%s estricto="%s">)(\\d+)(</%s>)' % (tag, strict, tag)
matches = list(re.finditer(pattern, block))
if len(matches) != 1:
    raise ValueError(f"teacher {args.teacher_index} has {len(matches)} matching {args.period} limits")
old_value = int(matches[0].group(2))
updated_block = re.sub(
    pattern,
    (lambda m: b"" if args.value == "absent"
     else m.group(1) + str(new_value).encode() + m.group(3)),
    block,
    count=1,
)
updated_xml = xml[:teacher.start()] + updated_block + xml[teacher.end():]
updated = bytearray(payload[:start] + updated_xml + payload[end:])
updated[3:7] = len(updated_xml).to_bytes(4, "little")
compressed = bytearray(gzip.compress(updated, mtime=0))
compressed[9] = 10
args.destination.write_bytes(compressed)
print(f"teacher {args.teacher_index}: {args.period} strict={args.strict} {old_value} -> {args.value}")
