"""Build a controlled two-frame GHC fixture with an earlier noon boundary.

The input must be ejemploDosMarcosHorarios with contarHuecosMediodia=false.
For frame B, swap the tramo types at indices 4 and 6 on all five days,
then set every teacher's non-strict daily permanence limit to 2.
"""

import argparse
import gzip
from pathlib import Path
import re


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("source", type=Path)
parser.add_argument("destination", type=Path)
args = parser.parse_args()

payload = gzip.decompress(args.source.read_bytes())
start = payload.index(b"<?xml")
end = payload.index(b"</datosGHC>") + len(b"</datosGHC>") + 1
if (start != 7 or payload[0] != ord("M") or payload[end - 1 : end] != b"\n"
        or int.from_bytes(payload[3:7], "little") != end - start):
    raise ValueError("unexpected GHC XML container")
xml = payload[start:end]
if xml.count(b"<contarHuecosMediodia>false</contarHuecosMediodia>") != 1:
    raise ValueError("expected contarHuecosMediodia=false")

swaps = {4: 0, 6: 0}


def change_tramo(match: re.Match[bytes]) -> bytes:
    tramo = match.group()
    if b"<submarco>B</submarco>" not in tramo:
        return tramo
    for index in swaps:
        if b"<indice>" + str(index).encode() + b"</indice>" in tramo:
            old = b"<Tipo>lectivo</Tipo>" if index == 4 else b"<Tipo>mediodia</Tipo>"
            new = b"<Tipo>mediodia</Tipo>" if index == 4 else b"<Tipo>lectivo</Tipo>"
            if tramo.count(old) != 1:
                raise ValueError(f"unexpected frame B tramo {index}")
            swaps[index] += 1
            return tramo.replace(old, new)
    return tramo


xml = re.sub(rb"<tramo>.*?</tramo>", change_tramo, xml, flags=re.DOTALL)
if swaps != {4: 5, 6: 5}:
    raise ValueError(f"expected five swaps per index, got {swaps}")
xml, limits = re.subn(
    rb'(<intervalosDePermanenciaDiarios estricto="false">)\d+(</intervalosDePermanenciaDiarios>)',
    rb'\g<1>2\g<2>', xml,
)
if limits != 11:
    raise ValueError(f"expected 11 teacher limits, got {limits}")

updated = bytearray(payload[:start] + xml + payload[end:])
updated[3:7] = len(xml).to_bytes(4, "little")
compressed = bytearray(gzip.compress(updated, mtime=0))
compressed[9] = 10
args.destination.write_bytes(compressed)
print(f"{args.destination}: 10 tramo types swapped, {limits} teacher limits set to 2")
