#!/usr/bin/env python3
"""Writes crates/ffx-host/src/com_methods.rs: the vtable of every COM
interface ffx-host stands in for (docs/host.md, "D3D11 and DXGI").

The method order is the declaration order of d3d11.h / dxgi.h, read from
the `windows` crate's generated `*_Vtbl` structs (the facts only: names,
order, argument count). Every argument of these methods is one 32-bit word
on i686 (pointers, UINTs, enums, BOOLs, FLOATs, SIZE_T), so the stack words
are the argument count plus `this`; the script refuses any other type.

    python3 -I tools/host-com-methods.py [windows crate src dir]
"""

import glob
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "crates/ffx-host/src/com_methods.rs")

# (Rust const, interface) for every vtable the host builds: the most
# derived interface of each object class
CLASSES = [
    ("DEVICE", "ID3D11Device1"),
    ("CONTEXT", "ID3D11DeviceContext1"),
    ("BUFFER", "ID3D11Buffer"),
    ("TEXTURE1D", "ID3D11Texture1D"),
    ("TEXTURE2D", "ID3D11Texture2D"),
    ("TEXTURE3D", "ID3D11Texture3D"),
    ("SRV", "ID3D11ShaderResourceView"),
    ("RTV", "ID3D11RenderTargetView"),
    ("DSV", "ID3D11DepthStencilView"),
    ("UAV", "ID3D11UnorderedAccessView"),
    ("VERTEX_SHADER", "ID3D11VertexShader"),
    ("PIXEL_SHADER", "ID3D11PixelShader"),
    ("GEOMETRY_SHADER", "ID3D11GeometryShader"),
    ("HULL_SHADER", "ID3D11HullShader"),
    ("DOMAIN_SHADER", "ID3D11DomainShader"),
    ("COMPUTE_SHADER", "ID3D11ComputeShader"),
    ("INPUT_LAYOUT", "ID3D11InputLayout"),
    ("BLEND", "ID3D11BlendState"),
    ("DEPTH", "ID3D11DepthStencilState"),
    ("RASTER", "ID3D11RasterizerState"),
    ("SAMPLER", "ID3D11SamplerState"),
    ("QUERY", "ID3D11Query"),
    ("SWAP_CHAIN", "IDXGISwapChain"),
    ("FACTORY", "IDXGIFactory1"),
    ("ADAPTER", "IDXGIAdapter1"),
    ("OUTPUT", "IDXGIOutput"),
    ("DXGI_DEVICE", "IDXGIDevice1"),
    ("DINPUT", "IDirectInput8A"),
    ("DINPUT_DEVICE", "IDirectInputDevice8A"),
]

# interfaces whose IIDs the host answers in QueryInterface (or refuses
# by name in its log)
IIDS = [
    "IUnknown",
    "ID3D11DeviceChild",
    "ID3D11Device",
    "ID3D11Device1",
    "ID3D11DeviceContext",
    "ID3D11DeviceContext1",
    "ID3D11Resource",
    "ID3D11Buffer",
    "ID3D11Texture1D",
    "ID3D11Texture2D",
    "ID3D11Texture3D",
    "ID3D11Debug",
    "ID3D11InfoQueue",
    "IDXGIObject",
    "IDXGIDeviceSubObject",
    "IDXGIDevice",
    "IDXGIDevice1",
    "IDXGIAdapter",
    "IDXGIAdapter1",
    "IDXGIFactory",
    "IDXGIFactory1",
    "IDXGIOutput",
    "IDXGISwapChain",
    "IDXGISurface",
    "IDXGIResource",
    "IDirectInput8A",
    "IDirectInputDevice8A",
]

ONE_WORD = re.compile(
    r"^(\*(mut|const) .*|u32|i32|f32|u8|usize|windows_core::BOOL|windows_core::HRESULT"
    r"|HWND|super::super::Foundation::HWND|windows_core::PCSTR|windows_core::PCWSTR"
    r"|(\w+::)*[A-Z][A-Z0-9_]+)$"
)


def split_args(s):
    out, depth, cur = [], 0, ""
    for ch in s:
        if ch in "(<[":
            depth += 1
        elif ch in ")>]":
            depth -= 1
        if ch == "," and depth == 0:
            out.append(cur.strip())
            cur = ""
        else:
            cur += ch
    if cur.strip():
        out.append(cur.strip())
    return out


def main():
    src = sys.argv[1] if len(sys.argv) > 1 else None
    if src is None:
        hits = sorted(glob.glob(os.path.expanduser(
            "~/.cargo/registry/src/*/windows-0.62.2/src/Windows/Win32/Graphics")))
        src = hits[-1]
    text = ""
    for f in ("Direct3D11/mod.rs", "Dxgi/mod.rs", "../Devices/HumanInterfaceDevice/mod.rs"):
        with open(os.path.join(src, f)) as fh:
            text += fh.read()
    vtbls = {}
    for m in re.finditer(r"pub struct (\w+)_Vtbl \{(.*?)\n\}", text, re.S):
        name, body = m.group(1), m.group(2)
        base = None
        methods = []
        for line in body.split("\n"):
            line = line.strip()
            b = re.match(r"pub base__: (?:\w+::)*(\w+)_Vtbl,", line)
            if b:
                base = b.group(1)
                continue
            f = re.match(r"pub (\w+): unsafe extern \"system\" fn\((.*)\)( -> (.*))?,$", line)
            if f:
                args = split_args(f.group(2))
                if args[0] != "*mut core::ffi::c_void":
                    sys.exit(f"{name}::{f.group(1)}: first argument is {args[0]}")
                ret = (f.group(4) or "").strip()
                methods.append((f.group(1), args, ret))
        vtbls[name] = (base, methods)
    vtbls["IUnknown"] = (None, [
        ("QueryInterface", ["*mut"] * 3, "windows_core::HRESULT"),
        ("AddRef", ["*mut"], "u32"),
        ("Release", ["*mut"], "u32"),
    ])

    def words(iface, meth, args):
        n = 0
        for a in args:
            if a in ("u64", "i64", "f64"):
                n += 2
            elif a == "*mut" or ONE_WORD.match(a):
                n += 1
            else:
                sys.exit(f"{iface}::{meth}: argument {a!r} is not one word")
        return n

    def flat(name):
        base, methods = vtbls[name]
        return (flat(base) if base else []) + [(name, *m) for m in methods]

    iids = {}
    for m in re.finditer(r"define_interface!\((\w+), \w+, 0x([0-9a-f_]+)\);", text):
        iids[m.group(1)] = m.group(2).replace("_", "")
    iids["IUnknown"] = "00000000" "0000" "0000" "c000" "000000000046"

    out = [
        "//! Generated by `tools/host-com-methods.py`; do not edit.",
        "//!",
        "//! The vtable of every COM object class ffx-host stands in for, in",
        "//! `d3d11.h` / `dxgi.h` declaration order: (interface, method, stack",
        "//! words with `this`, what it returns), and the IIDs it knows.",
        "",
        "/// What a method returns.",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub enum Returns {",
        "    /// An `HRESULT` in `eax`.",
        "    Hresult,",
        "    /// Nothing.",
        "    Void,",
        "    /// Another word in `eax` (a count, a `UINT`, a `BOOL`, an enum).",
        "    Word,",
        "    /// A `FLOAT` in `ST0`.",
        "    Float,",
        "}",
        "",
        "/// One vtable slot: declaring interface, method, stack words (with",
        "/// `this`), what it returns.",
        "pub type Method = (&'static str, &'static str, usize, Returns);",
        "",
    ]
    for const, iface in CLASSES:
        ms = flat(iface)
        out.append(f"/// `{iface}`: {len(ms)} slots.")
        out.append(f"pub const {const}: &[Method] = &[")
        for decl, meth, args, ret in ms:
            n = words(decl, meth, args)
            if ret.endswith("HRESULT"):
                r = "Returns::Hresult"
            elif ret == "":
                r = "Returns::Void"
            elif ret == "f32":
                r = "Returns::Float"
            elif ONE_WORD.match(ret):
                r = "Returns::Word"
            else:
                sys.exit(f"{decl}::{meth}: returns {ret!r}")
            out.append(f'    ("{decl}", "{meth}", {n}, {r}),')
        out.append("];")
        out.append("")
    out.append("/// Every vtable: (class, interface, slots).")
    out.append("pub const CLASSES: &[(&str, &str, &[Method])] = &[")
    for const, iface in CLASSES:
        out.append(f'    ("{const}", "{iface}", {const}),')
    out.append("];")
    out.append("")
    out.append("/// Each class's index in [`CLASSES`].")
    out.append("pub mod class {")
    for i, (const, iface) in enumerate(CLASSES):
        out.append(f"    /// `{iface}`.")
        out.append(f"    pub const {const}: usize = {i};")
    out.append("}")
    out.append("")
    out.append("/// The IIDs (`GUID` as `u128`, `data1` first) the host knows by name.")
    out.append("pub const IIDS: &[(&str, u128)] = &[")
    for name in IIDS:
        v = iids[name]
        out.append(f'    ("{name}", 0x{v[0:8]}_{v[8:12]}_{v[12:16]}_{v[16:20]}_{v[20:32]}),')
    out.append("];")
    with open(OUT, "w") as fh:
        fh.write("\n".join(out) + "\n")
    print(OUT)


if __name__ == "__main__":
    main()
