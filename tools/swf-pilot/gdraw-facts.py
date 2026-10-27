#!/usr/bin/env python3
"""GDraw's (Iggy's D3D11 back end in FFX.exe) fixed tables, decoded
(docs/formats/pc-ui.md, "How Iggy draws"): the sampler address modes and
MaxLOD (GDraw_createDeviceObjects_00a34cc0), the blend table
(GDraw_createBlendState_00a35670), the input layouts, and where the shader
bytecode lies (file offsets, for `dxbc FFX.exe`).

  FFX_GAME_DIR=~/steam-games/FFX-X2-HD python3 -I tools/swf-pilot/gdraw-facts.py
"""
import os
import pathlib
import struct

BLEND = {1: "ZERO", 2: "ONE", 3: "SRC_COLOR", 4: "INV_SRC_COLOR", 5: "SRC_ALPHA", 6: "INV_SRC_ALPHA",
         7: "DEST_ALPHA", 8: "INV_DEST_ALPHA", 9: "DEST_COLOR", 10: "INV_DEST_COLOR"}
ADDRESS = {1: "WRAP", 2: "MIRROR", 3: "CLAMP", 4: "BORDER", 5: "MIRROR_ONCE"}
FORMATS = {2: "R32G32B32A32_FLOAT", 6: "R32G32B32_FLOAT", 16: "R32G32_FLOAT", 28: "R8G8B8A8_UNORM",
           35: "R16G16_SINT", 13: "R16G16B16A16_SINT", 37: "R16G16_SNORM", 34: "R16G16_FLOAT",
           30: "R8G8B8A8_UINT", 31: "R8G8B8A8_SNORM", 11: "R16G16B16A16_UNORM", 41: "R32_FLOAT",
           38: "R16G16_SINT?", 10: "R16G16B16A16_FLOAT"}


class Image:
    def __init__(self, path):
        self.data = path.read_bytes()
        d = self.data
        nt = struct.unpack_from("<I", d, 0x3c)[0]
        count = struct.unpack_from("<H", d, nt + 6)[0]
        opt = struct.unpack_from("<H", d, nt + 20)[0]
        self.base = struct.unpack_from("<I", d, nt + 24 + 28)[0]
        self.secs = []
        at = nt + 24 + opt
        for _ in range(count):
            vsize, va, rsize, raw = struct.unpack_from("<IIII", d, at + 8)
            self.secs.append((self.base + va, rsize, raw))
            at += 40

    def off(self, a):
        for va, size, raw in self.secs:
            if va <= a < va + size:
                return raw + a - va
        raise KeyError(hex(a))

    def u32(self, a, n=1):
        return list(struct.unpack_from(f"<{n}I", self.data, self.off(a)))

    def f32(self, a):
        return struct.unpack_from("<f", self.data, self.off(a))[0]

    def cstr(self, a):
        o = self.off(a)
        return self.data[o:self.data.index(b"\0", o)].decode("latin1")


def main():
    exe = Image(pathlib.Path(os.environ.get("FFX_GAME_DIR", str(pathlib.Path.home() / "steam-games/FFX-X2-HD"))) / "FFX.exe")
    modes = exe.u32(0x00b7a894, 4)
    print("sampler address modes (index r.b[0xa]):", [ADDRESS.get(m, m) for m in modes])
    print("sampler filters (index r.b[0xb]): 0 MIN_MAG_MIP_LINEAR (0x15), 1 MIN_LINEAR_MAG_MIP_POINT (0x10)")
    print(f"MaxLOD {exe.f32(0x00b17504)}, MipLODBias 0, MaxAnisotropy 1, AddressW CLAMP")
    print("blend states (+0x7d4 + 4*r.b[0xc]): src, dest, op as D3D11 BlendEnable 1, alpha src/dest from the same with DEST_COLOR->DEST_ALPHA")
    for k in range(6):
        a, b, c = exe.u32(0x00c85ab0 + 12 * k, 3)
        print(f"  {k}: enable {a} src {BLEND.get(b, b)} dest {BLEND.get(c, c)}")
    print("vertex shaders and input layouts (0x00c85a98, 0x00c6d4f4):")
    for k in range(3):
        code, length = exe.u32(0x00c85a98 + 8 * k, 2)
        print(f"  vs {k}: bytecode {code:#x} (file {exe.off(code):#x}), {length} bytes")
    for k in range(3):
        elems, n = exe.u32(0x00c6d4f4 + 8 * k, 2)
        out = []
        for e in range(n):
            name, idx, fmt, slot, off, cls, step = exe.u32(elems + 28 * e, 7)
            out.append(f"{exe.cstr(name)}{idx} {FORMATS.get(fmt, fmt)} @{off}")
        print(f"  layout {k}: " + ", ".join(out))
    for table, n, name in [(0x00c770b8, 13, "basic"), (0x00c806d8, 32, "filters"), (0x00c842b8, 10, "blur")]:
        print(f"pixel shaders {name} ({table:#x}):")
        for k in range(n):
            code, length = exe.u32(table + 8 * k, 2)
            if code:
                print(f"  {k}: {code:#x} (file {exe.off(code):#x}) {length} bytes")


if __name__ == "__main__":
    main()
