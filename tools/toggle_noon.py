"""Make a one-field GHC input variant for controlled native replay."""

import argparse
import gzip
from pathlib import Path


parser = argparse.ArgumentParser()
parser.add_argument("source", type=Path)
parser.add_argument("destination", type=Path)
parser.add_argument("value", choices=("true", "false", "absent"))
args = parser.parse_args()

payload = gzip.decompress(args.source.read_bytes())
start = payload.index(b"<?xml")
end = payload.index(b"</datosGHC>") + len(b"</datosGHC>")
assert start == 7 and payload[0] == ord("M"), "unexpected GHC header"
assert payload[end:end + 1] == b"\n", "expected native XML trailing newline"
xml_end = end + 1
assert int.from_bytes(payload[3:7], "little") == xml_end - start, "source XML length header mismatch"
name = b"contarHuecosMediodia"
variants = [b"<" + name + b">" + value + b"</" + name + b">" for value in (b"true", b"false")]
assert sum(payload.count(variant) for variant in variants) == 1, "expected exactly one noon option"
old = variants[0] if variants[0] in payload else variants[1]
new = variants[0] if args.value == "true" else variants[1] if args.value == "false" else b""
updated = bytearray(payload.replace(old, new))
updated[3:7] = (xml_end - start + len(new) - len(old)).to_bytes(4, "little")
out = bytearray(gzip.compress(updated, mtime=0))
out[9] = 10  # OS field used by the native writer; gzip CRC stays valid.
args.destination.write_bytes(out)
print(f"{args.destination}: {old.decode()} -> {new.decode()}")
