//! List `p5-core`, large-function round: Iggy's `GDraw` D3D11 device
//! objects (`0x00a34cc0`, cdecl, no arguments; the only caller,
//! `0x00a35780`, overwrites `eax` at once). It runs once when the game
//! sets `GDraw` up, on the `GDraw` state at `[0x01a858b4]` (`+0` the
//! device):
//! 1. eighteen state entries `+8 + 8i` from the table `0x00c72f48`
//!    (`0x00a35b40(entry, source)`);
//! 2. the pixel shaders: three tables of (bytecode, length) pairs
//!    (`0x00c770b8` 13 into `+0x98`, `0x00c806d8` 32 into `+0x100`,
//!    `0x00c842b8` 10 into `+0x200`) and two single pairs (`0x00c847d8`
//!    into `+0x250`, `0x00c84a8c` into `+0x258`): each pair copied into
//!    the state, and with bytecode, `CreatePixelShader` (device slot 15)
//!    into the pair's second word, which a failure clears after the
//!    warning;
//! 3. three vertex shaders the same way (`0x00c85a98` into `+0x270`,
//!    `CreateVertexShader`, slot 12) each followed by its input layout
//!    (`CreateInputLayout`, slot 11, the element table `0x00c6d4f4`, into
//!    `+0x260 + 4k`);
//! 4. two rasterizer states (`+0x7ac`, multisampling off then on), eight
//!    sampler states (`+0x7b4`: filter 0x15 then 0x10, by the address
//!    modes at `0x00b7a894`), six blend states (`0x00a35670` with the
//!    table `0x00c85ab0`, `+0x7d4`), one more from a zeroed description
//!    (`+0x7ec`), four depth-stencil states (`+0x7f0`): the first failure
//!    warns and ends the set-up (a null blend state ends it quietly);
//! 5. five dynamic buffers (`0x00a35a70(size, 4)`, `+0x984..+0x994`);
//! 6. the quad index buffer: `0x800` quads' six indices (`v, v+1, v+2,
//!    v, v+2, v+3`) in a block from `_IggyGDrawMallocAnnotated@12`,
//!    an immutable index buffer of them (`CreateBuffer`, slot 3) into
//!    `+0x7a0` (0 when the block or the buffer fails), the block freed
//!    (`_IggyGDrawFree@4`).
//!
//! Warnings go through `IggyGDrawSendWarning` (cdecl, an import slot):
//! `(0, "GDraw D3D out of memory in %s%s", what, suffix)` for
//! `E_OUTOFMEMORY`, else `(0, "GDraw D3D error in %s%s: 0x%08x", what,
//! suffix, hr)`. The descriptions the device methods read are built in
//! the frame as the original builds them.

use ffx_graphic::host::{Conv, Host};

use crate::p5_gdraw::{
    E_OUTOFMEMORY, EMPTY, ERROR_FORMAT, MEMSET, OUT_OF_MEMORY, STATE, WARN_SLOT,
};

/// The state entries' set-up (cdecl `(entry, source)`) and its sources.
const INIT_ENTRY: u32 = 0x00a3_5b40;
const ENTRY_SOURCES: u32 = 0x00c7_2f48;
/// The pixel shaders' tables: (source, pairs, state offset).
const PIXEL_TABLES: [(u32, u32, u32); 5] = [
    (0x00c7_70b8, 13, 0x98),
    (0x00c8_06d8, 32, 0x100),
    (0x00c8_42b8, 10, 0x200),
    (0x00c8_47d8, 1, 0x250),
    (0x00c8_4a8c, 1, 0x258),
];
/// The vertex shaders' (bytecode, length) pairs and their input
/// layouts' (elements, count).
const VERTEX_TABLE: u32 = 0x00c8_5a98;
const LAYOUT_TABLE: u32 = 0x00c6_d4f4;
/// The sampler states' address modes (four words) and their `MaxLOD`.
const ADDRESS_MODES: u32 = 0x00b7_a894;
const MAX_LOD: u32 = 0x00b1_7504;
/// The blend states' maker and table (three words each).
const CREATE_BLEND: u32 = 0x00a3_5670;
const BLEND_TABLE: u32 = 0x00c8_5ab0;
/// The dynamic buffers' maker (cdecl `(size, 4)`) and theirs: (state
/// offset, size).
const DYNAMIC_BUFFER: u32 = 0x00a3_5a70;
const DYNAMIC_BUFFERS: [(u32, u32); 5] = [
    (0x984, 0x80),
    (0x988, 0x40),
    (0x98c, 0x50),
    (0x990, 0x50),
    (0x994, 0xa0),
];
/// `_IggyGDrawMallocAnnotated@12` and `_IggyGDrawFree@4` (import slots,
/// stdcall), and the source file and line `GDraw` names.
const MALLOC_SLOT: u32 = 0x00b0_c86c;
const FREE_SLOT: u32 = 0x00b0_c870;
const SOURCE: u32 = 0x00b7_a5e8;
const LINE: u32 = 0x815;
/// The warnings' `what` strings.
const PIXEL_SHADER: u32 = 0x00b7_a938;
const VERTEX_SHADER: u32 = 0x00b7_a94c;
const INPUT_LAYOUT: u32 = 0x00b7_a868;
const RASTERIZER: u32 = 0x00b7_a87c;
const SAMPLER: u32 = 0x00b7_a8a4;
const BLEND: u32 = 0x00b7_a414;
const DEPTH_STENCIL: u32 = 0x00b7_a588;
const INDEX_BUFFER: u32 = 0x00b7_a6e4;
const INDEX_BUFFER_SUFFIX: u32 = 0x00b7_a8b8;

/// The index buffer's bytes: 0x800 quads of six 16-bit indices.
const INDEX_BYTES: u32 = 0x6000;

/// The quad indices `v, v+1, v+2, v, v+2, v+3` for `v = 0, 4, ...`.
#[must_use]
pub fn quad_indices() -> Vec<u8> {
    let mut out = Vec::with_capacity(INDEX_BYTES as usize);
    for q in 0..INDEX_BYTES / 12 {
        let v = (4 * q) as u16;
        for i in [v, v + 1, v + 2, v, v + 2, v + 3] {
            out.extend(i.to_le_bytes());
        }
    }
    out
}

/// What `fld dword` / `fstp dword` leave of the 32 bits `v` (a signalling
/// NaN quieted).
#[must_use]
pub fn through_x87(v: u32) -> u32 {
    if v & 0x7f80_0000 == 0x7f80_0000 && v & 0x007f_ffff != 0 {
        v | 0x0040_0000
    } else {
        v
    }
}

pub mod bridge {
    //! The original over game memory.

    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// The frame's bytes the port keeps (the descriptions), at the
    /// original's `ebp` offsets: `ebp - 0x188` up to `ebp`.
    pub const FRAME: u32 = 0x188;

    /// A COM method: the vtable word at `off` of `obj`, `obj` first
    /// (stdcall).
    fn com(h: &mut impl Host, obj: u32, off: u32, args: &[u32]) -> u32 {
        let vt: u32 = h.read(obj);
        let f: u32 = h.read(vt.wrapping_add(off));
        let mut words = [0u32; 7];
        words[0] = obj;
        words[1..=args.len()].copy_from_slice(args);
        h.call(f, Conv::Stdcall, &words[..=args.len()])
    }

    /// The `GDraw` state.
    fn state(h: &impl Host) -> u32 {
        h.read(h.rebase(STATE))
    }

    /// `GDraw`'s warning for a failed `what` (`suffix` after it).
    fn warn(h: &mut impl Host, f: u32, hr: u32, what: u32, suffix: u32) {
        if hr == E_OUTOFMEMORY {
            let format = h.rebase(OUT_OF_MEMORY);
            h.call(f, Conv::Cdecl, &[0, format, what, suffix]);
        } else {
            let format = h.rebase(ERROR_FORMAT);
            h.call(f, Conv::Cdecl, &[0, format, what, suffix, hr]);
        }
    }

    /// Whether an `HRESULT` failed.
    fn failed(hr: u32) -> bool {
        hr & 0x8000_0000 != 0
    }

    /// A shader pair copied from `source` into the state at `off`; with
    /// bytecode, the device's method at `method` makes the shader into
    /// the pair's second word (cleared after the warning when it fails).
    fn shader(h: &mut impl Host, f: u32, source: u32, off: u32, (method, what): (u32, u32)) {
        let g = state(h);
        let slot = g.wrapping_add(off);
        let pair: [u32; 2] = h.read(source);
        h.write(slot, pair);
        let code: u32 = h.read(slot);
        if code == 0 {
            return;
        }
        let dev: u32 = h.read(g);
        let len: u32 = h.read(slot + 4);
        let hr = com(h, dev, method, &[code, len, 0, slot + 4]);
        if failed(hr) {
            let empty = h.rebase(EMPTY);
            let what = h.rebase(what);
            warn(h, f, hr, what, empty);
            h.write(slot + 4, 0u32);
        }
    }

    /// `0x00a34cc0()`: `GDraw`'s device objects. `frame` is [`FRAME`]
    /// bytes for the descriptions.
    pub fn create_device_objects(h: &mut impl Host, frame: u32) {
        let ebp = frame + FRAME;
        let dev: u32 = h.read(state(h));
        let sources = h.rebase(ENTRY_SOURCES);
        for i in 0..0x12u32 {
            let g = state(h);
            h.call_sym(
                INIT_ENTRY,
                Conv::Cdecl,
                &[g.wrapping_add(8 + 8 * i), sources + 8 * i],
            );
        }
        let f: u32 = h.read(h.rebase(WARN_SLOT));
        for (source, n, off) in PIXEL_TABLES {
            let source = h.rebase(source);
            for k in 0..n {
                shader(h, f, source + 8 * k, off + 8 * k, (0x3c, PIXEL_SHADER));
            }
        }
        let vertex = h.rebase(VERTEX_TABLE);
        let layouts = h.rebase(LAYOUT_TABLE);
        for k in 0..3u32 {
            shader(h, f, vertex + 8 * k, 0x270 + 8 * k, (0x30, VERTEX_SHADER));
            let out = state(h).wrapping_add(0x260 + 4 * k);
            let [elements, count]: [u32; 2] = h.read(layouts + 8 * k);
            let [code, len]: [u32; 2] = h.read(vertex + 8 * k);
            let hr = com(h, dev, 0x2c, &[elements, count, code, len, out]);
            if failed(hr) {
                let empty = h.rebase(EMPTY);
                let what = h.rebase(INPUT_LAYOUT);
                warn(h, f, hr, what, empty);
                let g = state(h);
                h.write(g.wrapping_add(0x260 + 4 * k), 0u32);
            }
        }
        if states(h, dev, f, ebp).is_some() {
            index_buffer(h, ebp);
        }
    }

    /// `None` after the warning when `hr` failed.
    fn check(h: &mut impl Host, f: u32, hr: u32, what: u32) -> Option<()> {
        if failed(hr) {
            let empty = h.rebase(EMPTY);
            let what = h.rebase(what);
            warn(h, f, hr, what, empty);
            return None;
        }
        Some(())
    }

    /// Step 4 and the dynamic buffers; `None` when a state failed.
    fn states(h: &mut impl Host, dev: u32, f: u32, ebp: u32) -> Option<()> {
        // rasterizer states: solid, no culling, depth clip and scissor
        // on, multisampling off then on
        let desc = ebp - 0x2c;
        for k in 0..2u32 {
            h.write(desc, [3u32, 1, 0, 0, 0, 0, 1, 1, k, 0]);
            let out = state(h).wrapping_add(0x7ac + 4 * k);
            let hr = com(h, dev, 0x58, &[desc, out]);
            check(h, f, hr, RASTERIZER)?;
        }
        // sampler states: by filter, by address mode
        let desc = ebp - 0x60;
        let modes = h.rebase(ADDRESS_MODES);
        for k in 0..2u32 {
            let filter = if k == 0 { 0x15 } else { 0x10 };
            for m in 0..4u32 {
                h.call_sym(MEMSET, Conv::Cdecl, &[desc, 0, 0x34]);
                let max_lod = through_x87(h.read(h.rebase(MAX_LOD)));
                h.write(desc + 0x30, max_lod);
                h.write(desc, filter);
                let mode: u32 = h.read(modes + 4 * m);
                h.write(desc + 4, [mode, mode, 3]);
                h.write(desc + 0x14, 1u32);
                let out = state(h).wrapping_add(0x7b4 + 0x10 * k + 4 * m);
                let hr = com(h, dev, 0x5c, &[desc, out]);
                check(h, f, hr, SAMPLER)?;
            }
        }
        // blend states: the table's, then a zeroed description's
        let table = h.rebase(BLEND_TABLE);
        for k in 0..6u32 {
            let [a, b, c]: [u32; 3] = h.read(table + 12 * k);
            let s = h.call_sym(CREATE_BLEND, Conv::Cdecl, &[dev, a, b, c]);
            let g = state(h);
            h.write(g.wrapping_add(0x7d4 + 4 * k), s);
            if s == 0 {
                return None;
            }
        }
        let desc = ebp - 0x168;
        h.call_sym(MEMSET, Conv::Cdecl, &[desc, 0, 0x108]);
        let out = state(h).wrapping_add(0x7ec);
        let hr = com(h, dev, 0x50, &[desc, out]);
        check(h, f, hr, BLEND)?;
        // depth-stencil states: depth write by i, the test (less) by j
        let desc = ebp - 0x60;
        for i in 0..2u32 {
            for j in 0..2u32 {
                h.call_sym(MEMSET, Conv::Cdecl, &[desc, 0, 0x34]);
                h.write(
                    desc,
                    [
                        u32::from(i != 0 || j != 0),
                        u32::from(i != 0),
                        if j != 0 { 2 } else { 8 },
                        0,
                    ],
                );
                let out = state(h).wrapping_add(0x7f0 + 8 * i + 4 * j);
                let hr = com(h, dev, 0x54, &[desc, out]);
                check(h, f, hr, DEPTH_STENCIL)?;
            }
        }
        for (off, size) in DYNAMIC_BUFFERS {
            let b = h.call_sym(DYNAMIC_BUFFER, Conv::Cdecl, &[size, 4]);
            let g = state(h);
            h.write(g.wrapping_add(off), b);
        }
        Some(())
    }

    /// Step 6.
    fn index_buffer(h: &mut impl Host, ebp: u32) {
        let malloc: u32 = h.read(h.rebase(MALLOC_SLOT));
        let source = h.rebase(SOURCE);
        let block = h.call(malloc, Conv::Stdcall, &[INDEX_BYTES, source, LINE]);
        if block == 0 {
            let g = state(h);
            h.write(g.wrapping_add(0x7a0), 0u32);
            return;
        }
        let desc = ebp - 0x1c;
        h.write(desc, [INDEX_BYTES, 1, 2, 0, 0, 0]);
        let init = ebp - 0x188;
        h.write(init, [block, 0, 0]);
        h.write_bytes(block, &quad_indices());
        let g = state(h);
        let dev: u32 = h.read(g);
        let hr = com(h, dev, 0xc, &[desc, init, g.wrapping_add(0x7a0)]);
        let f: u32 = h.read(h.rebase(WARN_SLOT));
        if failed(hr) {
            let what = h.rebase(INDEX_BUFFER);
            let suffix = h.rebase(INDEX_BUFFER_SUFFIX);
            warn(h, f, hr, what, suffix);
            let g = state(h);
            h.write(g.wrapping_add(0x7a0), 0u32);
        }
        let free: u32 = h.read(h.rebase(FREE_SLOT));
        h.call(free, Conv::Stdcall, &[block]);
    }
}
