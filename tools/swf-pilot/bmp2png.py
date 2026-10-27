#!/usr/bin/env python3
"""Harness screenshots (32-bit bottom-up BMP) to PNG, without PIL.

  python3 -I tools/swf-pilot/bmp2png.py out_dir file.bmp...
"""
import os
import struct
import sys
import zlib


def read_bmp(path):
    d = open(path, "rb").read()
    off = struct.unpack_from("<I", d, 10)[0]
    w, h = struct.unpack_from("<ii", d, 18)
    bpp = struct.unpack_from("<H", d, 28)[0]
    if bpp != 32:
        raise SystemExit(f"{path}: {bpp} bpp")
    rows = []
    for y in range(abs(h)):
        src = y if h < 0 else abs(h) - 1 - y
        row = d[off + src * 4 * w: off + (src + 1) * 4 * w]
        out = bytearray(3 * w)
        out[0::3] = row[2::4]
        out[1::3] = row[1::4]
        out[2::3] = row[0::4]
        rows.append(bytes(out))
    return w, abs(h), rows


def write_png(path, w, h, rows):
    raw = b"".join(b"\0" + r for r in rows)

    def chunk(t, data):
        c = struct.pack(">I", len(data)) + t + data
        return c + struct.pack(">I", zlib.crc32(t + data) & 0xffffffff)

    png = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(raw, 6)) + chunk(b"IEND", b"")
    open(path, "wb").write(png)


def main():
    out = sys.argv[1]
    os.makedirs(out, exist_ok=True)
    for p in sys.argv[2:]:
        w, h, rows = read_bmp(p)
        write_png(os.path.join(out, os.path.basename(p)[:-4] + ".png"), w, h, rows)


if __name__ == "__main__":
    main()
