#!/usr/bin/env python3
"""Tag census of SWF files (CWS/FWS): stage size, frame rate, frames, and
how many of each tag; with --fonts the fonts' names and glyph counts, and
the edit texts' font ids, flags and initial text.

  python3 -I tools/swf-pilot/swf-tags.py [--fonts] file.swf...
"""
import collections
import struct
import sys
import zlib

NAMES = {
    0: "End", 1: "ShowFrame", 2: "DefineShape", 4: "PlaceObject", 5: "RemoveObject", 6: "DefineBits",
    8: "JPEGTables", 9: "SetBackgroundColor", 10: "DefineFont", 11: "DefineText", 12: "DoAction",
    13: "DefineFontInfo", 14: "DefineSound", 15: "StartSound", 20: "DefineBitsLossless",
    21: "DefineBitsJPEG2", 22: "DefineShape2", 26: "PlaceObject2", 28: "RemoveObject2",
    32: "DefineShape3", 33: "DefineText2", 34: "DefineButton2", 35: "DefineBitsJPEG3",
    36: "DefineBitsLossless2", 37: "DefineEditText", 39: "DefineSprite", 43: "FrameLabel",
    46: "DefineMorphShape", 48: "DefineFont2", 56: "ExportAssets", 57: "ImportAssets",
    58: "EnableDebugger", 64: "EnableDebugger2", 65: "ScriptLimits", 69: "FileAttributes",
    70: "PlaceObject3", 71: "ImportAssets2", 73: "DefineFontAlignZones", 74: "CSMTextSettings",
    75: "DefineFont3", 76: "SymbolClass", 77: "Metadata", 78: "DefineScalingGrid", 82: "DoABC",
    83: "DefineShape4", 84: "DefineMorphShape2", 86: "DefineSceneAndFrameLabelData",
    87: "DefineBinaryData", 88: "DefineFontName", 89: "StartSound2", 90: "DefineBitsJPEG4",
    91: "DefineFont4", 41: "ProductInfo",
}


def body(path):
    d = open(path, "rb").read()
    if d[:3] == b"CWS":
        return d[3], zlib.decompress(d[8:])
    if d[:3] == b"FWS":
        return d[3], d[8:]
    raise SystemExit(f"{path}: not CWS/FWS")


def rect_size(b):
    n = b[0] >> 3
    bits = 5 + 4 * n
    vals, pos = [], 5
    for _ in range(4):
        v = 0
        for _ in range(n):
            v = (v << 1) | ((b[pos >> 3] >> (7 - (pos & 7))) & 1)
            pos += 1
        if v & (1 << (n - 1)):
            v -= 1 << n
        vals.append(v)
    return vals, (bits + 7) // 8


def tags(b, at, end):
    while at < end:
        code_len = struct.unpack_from("<H", b, at)[0]
        code, length = code_len >> 6, code_len & 0x3f
        at += 2
        if length == 0x3f:
            length = struct.unpack_from("<I", b, at)[0]
            at += 4
        yield code, at, length
        at += length
        if code == 0:
            break


def main():
    fonts = "--fonts" in sys.argv
    for path in [a for a in sys.argv[1:] if not a.startswith("--")]:
        ver, b = body(path)
        (x0, x1, y0, y1), n = rect_size(b)
        rate = struct.unpack_from("<H", b, n)[0] / 256
        frames = struct.unpack_from("<H", b, n + 2)[0]
        count = collections.Counter()
        notes = []
        stack = [(n + 4, len(b))]
        while stack:
            s, e = stack.pop()
            for code, at, length in tags(b, s, e):
                count[NAMES.get(code, code)] += 1
                if code == 39:
                    stack.append((at + 4, at + length))
                if fonts and code in (48, 75):
                    fid, flags = struct.unpack_from("<HB", b, at)
                    name_len = b[at + 4]
                    name = b[at + 5:at + 5 + name_len].decode("latin1").rstrip("\0")
                    glyphs = struct.unpack_from("<H", b, at + 5 + name_len)[0]
                    notes.append(f"  font {fid} {NAMES[code]} '{name}' {glyphs} glyphs flags {flags:#x}")
                if fonts and code == 37:
                    cid = struct.unpack_from("<H", b, at)[0]
                    _, rn = rect_size(b[at + 2:])
                    p = at + 2 + rn
                    f1, f2 = b[p], b[p + 1]
                    p += 2
                    extra = ""
                    if f1 & 1:  # HasFont
                        fid, height = struct.unpack_from("<HH", b, p)
                        p += 4
                        extra += f" font {fid} height {height / 20}"
                    if f2 & 0x80:  # HasFontClass
                        cls = b[p:b.index(b"\0", p)].decode("latin1")
                        p += len(cls) + 1
                        extra += f" class {cls}"
                    notes.append(f"  edittext {cid} flags {f1:#04x} {f2:#04x} useOutlines {bool(f2 & 0x01)}{extra}")
        print(f"{path}: SWF {ver}, {(x1 - x0) / 20}x{(y1 - y0) / 20}, {rate} fps, {frames} frames")
        print("  " + ", ".join(f"{k} {v}" for k, v in count.most_common()))
        for line in notes[:40]:
            print(line)
        if len(notes) > 40:
            print(f"  ... {len(notes) - 40} more")


if __name__ == "__main__":
    main()
