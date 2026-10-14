"""Maps the shaders a capture saw (FNV-1a of their bytecode, as the gfx
layer hashes them) to the shader clusters they come from.

Usage: python3 tools/gfx/shader_map.py <extracted shaders dir> > map.tsv
Prints: hash, kind (vs/ps/...), cluster file, index of the DXBC blob.
"""
import struct
import sys
from pathlib import Path


def fnv(b):
    h = 0xCBF29CE484222325
    for x in b:
        h ^= x
        h = (h * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return h


def kind(blob):
    # The SHDR/SHEX chunk's version token: program type in the high word.
    count = struct.unpack_from("<I", blob, 28)[0]
    for i in range(count):
        off = struct.unpack_from("<I", blob, 32 + 4 * i)[0]
        tag = blob[off : off + 4]
        if tag in (b"SHDR", b"SHEX"):
            ver = struct.unpack_from("<I", blob, off + 8)[0]
            return {0: "ps", 1: "vs", 2: "gs", 3: "hs", 4: "ds", 5: "cs"}.get(ver >> 16, "?")
    return "?"


for f in sorted(Path(sys.argv[1]).rglob("*.phyre")):
    data = f.read_bytes()
    i = n = 0
    while (i := data.find(b"DXBC", i)) >= 0:
        size = struct.unpack_from("<I", data, i + 24)[0]
        blob = data[i : i + size]
        if len(blob) == size and size > 32:
            print(f"{fnv(blob):016x}\t{kind(blob)}\t{f.name}\t{n}")
            n += 1
        i += 4
