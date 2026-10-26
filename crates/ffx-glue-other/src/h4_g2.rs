//! Continuation round 5 of list `h4-glue-other`, batch 2: the
//! distortion grid's positions and UVs, the extended blur's constructors,
//! `TextUtil`'s and the post-process tweak tool's constructors, an Iggy
//! menu's viewport and a `GDraw` sub-rectangle's UVs.
//!
//! Floats follow the x87 at 53 bits ([`crate::h4_e1`]). Constants the
//! original pushes with `fld1`/`fldz` are exact; image constants pass
//! through `fld`/`fst`.

use ffx_bridge::Global;
use ffx_bridge::x87::{add, div, ftol, load, mul, store, sub};
use ffx_graphic::host::{Conv, Host};

use crate::h4_e1::{int_f32, pass};

const ONE: u32 = 0x3f80_0000;

/// The `f32` constant at image address `sym` through `fld`/`fst`.
fn konst(h: &impl Host, sym: u32) -> u32 {
    pass(h.read(h.rebase(sym)))
}

/// The `f64` constant at image address `sym`.
fn konst64(h: &impl Host, sym: u32) -> f64 {
    h.read(h.rebase(sym))
}

// ---- the distortion grid

/// `Distortion_fillGridUV_006c2bb0(this)` (thiscall, `ecx` only): the
/// grid's UVs (`[this + 0x70]`: 9 rows of 18 floats): row `i` holds `f =
/// i * K` (`f64` 0x00b92200) at every even word, then `0 * K` at word 1,
/// the constants 0x00b49368, 0x00b92180, 0x00b4936c, 0x00b92194,
/// 0x00b49370, 0x00b43d18, 0x00b49374 at words 3..=15 (odd) and 1.0 at
/// word 17. Returns what the original leaves in `eax`: its cursor, the
/// grid plus 0x29c.
pub fn grid_uv(h: &mut impl Host, this: u32) -> u32 {
    let grid: u32 = h.read(this.wrapping_add(0x70));
    let k = konst64(h, 0x00b9_2200);
    let zero = store(mul(0.0, k));
    let c = [
        0x00b4_9368,
        0x00b9_2180,
        0x00b4_936c,
        0x00b9_2194,
        0x00b4_9370,
        0x00b4_3d18,
    ]
    .map(|s| konst(h, s));
    let last = konst(h, 0x00b4_9374);
    for i in 0..9u32 {
        let row = grid.wrapping_add(0x48 * i);
        let f = store(mul(k, load(int_f32(i.cast_signed()))));
        let w = |n: u32| row.wrapping_add(4 * n);
        h.write(w(0), f);
        h.write(w(1), zero);
        for (j, &cj) in c.iter().enumerate() {
            let j = j as u32;
            h.write(w(2 + 2 * j), f);
            h.write(w(3 + 2 * j), cj);
        }
        h.write(w(14), f);
        h.write(w(15), last);
        h.write(w(16), f);
        h.write(w(17), ONE);
    }
    grid.wrapping_add(0x14 + 0x48 * 9)
}

/// `Distortion_fillGridPositions_006c2ca0(this)` (thiscall, `ecx` only):
/// the grid's positions (`[this + 0x68]`: 9 rows of 9 points `x, y, z`):
/// row `i` has `x = i * K - 1` (`f64` 0x00b34f90) at every point, `y`
/// from `0 * K - 1`, the constants 0x00b47c9c, 0x00b92338, 0x00b49378,
/// 0 (`fldz`), 0x00b92180, 0x00b92194, 0x00b43d18 to 1.0, and `z` 1.0.
/// Returns its cursor in `eax`: the grid plus 0x3d8.
pub fn grid_positions(h: &mut impl Host, this: u32) -> u32 {
    let grid: u32 = h.read(this.wrapping_add(0x68));
    let k = konst64(h, 0x00b3_4f90);
    let first = store(sub(mul(0.0, k), 1.0));
    let ys = [
        first,
        konst(h, 0x00b4_7c9c),
        konst(h, 0x00b9_2338),
        konst(h, 0x00b4_9378),
        0,
        konst(h, 0x00b9_2180),
        konst(h, 0x00b9_2194),
        konst(h, 0x00b4_3d18),
        ONE,
    ];
    for i in 0..9u32 {
        let row = grid.wrapping_add(0x6c * i);
        let x = store(sub(mul(k, load(int_f32(i.cast_signed()))), 1.0));
        for (j, &y) in ys.iter().enumerate() {
            let p = row.wrapping_add(12 * j as u32);
            h.write(p, x);
            h.write(p.wrapping_add(4), y);
            h.write(p.wrapping_add(8), ONE);
        }
    }
    grid.wrapping_add(0x14 + 0x6c * 9)
}

// ---- the extended blur's constructors

/// `PSamplerStateD3D11_ctor(this)` (thiscall).
pub const SAMPLER_CTOR: u32 = 0x0058_3b20;
/// `operator_new(size)` (cdecl).
pub const OPERATOR_NEW: u32 = 0x0063_0310;
/// `BlurEffectExParams_ctor_006c4020(this)` (thiscall).
pub const BLUR_PARAMS_CTOR: u32 = 0x006c_4020;

/// `BlurEffectExParams_ctor_006c4020(this)` (thiscall, `ecx` only): words
/// `+0..+0x20` zero, the sampler at `+0x24` constructed, `+0x48..+0x54`
/// zero, `+0x5c..+0x64`, `+0x6c`, `+0x70`, `+0x80` 1.0, `+0x68` and
/// `+0x74..+0x7c` the constant 0x00b92194, `+0x84` 0. Returns `this`.
pub fn blur_params_ctor(h: &mut impl Host, this: u32) -> u32 {
    for k in 0..9u32 {
        h.write(this.wrapping_add(4 * k), 0u32);
    }
    h.call_sym(SAMPLER_CTOR, Conv::Thiscall(this.wrapping_add(0x24)), &[]);
    for off in [0x48u32, 0x4c, 0x50, 0x54] {
        h.write(this.wrapping_add(off), 0u32);
    }
    let half = konst(h, 0x00b9_2194);
    for off in [0x6cu32, 0x70] {
        h.write(this.wrapping_add(off), ONE);
    }
    for off in [0x74u32, 0x78, 0x7c] {
        h.write(this.wrapping_add(off), half);
    }
    for off in [0x80u32, 0x5c, 0x60, 0x64] {
        h.write(this.wrapping_add(off), ONE);
    }
    h.write(this.wrapping_add(0x68), half);
    h.write(this.wrapping_add(0x84), 0u32);
    this
}

/// `BlurEffectEx_ctor_006c4820(this)` (thiscall, `ecx` only): `+0x38` 0.0,
/// `+4..+0x24` zero, byte `+0x28` 0, `+0x2c` -1, `+0x30`, `+0x34`, `+0x3c`
/// zero; `+0` a new 0x88-byte [`blur_params_ctor`] object, or 0 when
/// `operator_new` fails. Returns `this`.
pub fn blur_ex_ctor(h: &mut impl Host, this: u32) -> u32 {
    h.write(this.wrapping_add(0x38), 0u32);
    for k in 1..10u32 {
        h.write(this.wrapping_add(4 * k), 0u32);
    }
    h.write(this.wrapping_add(0x28), 0u8);
    h.write(this.wrapping_add(0x2c), u32::MAX);
    for off in [0x30u32, 0x34, 0x3c] {
        h.write(this.wrapping_add(off), 0u32);
    }
    let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x88]);
    let params = if p == 0 {
        0
    } else {
        h.call_sym(BLUR_PARAMS_CTOR, Conv::Thiscall(p), &[])
    };
    h.write(this, params);
    this
}

// ---- TextUtil and the post-process tweak tool

/// `PMatrix4_Copy_004c9430(out; m)` (thiscall).
pub const MATRIX_COPY: u32 = 0x004c_9430;
/// `TextUtil_TextUtil(this)` (thiscall).
pub const TEXT_UTIL_CTOR: u32 = 0x006f_5b50;
/// Bytes of [`text_util_ctor`]'s frame: the identity matrix.
pub const IDENTITY_FRAME: u32 = 0x40;

/// `TextUtil_TextUtil(this)` (thiscall, `ecx` only; `frame` the identity
/// matrix at `ebp - 0x40`, which it copies into `+0x20` with
/// `PMatrix4_Copy`): `+8..+0x1c` zero, `+0x60..+0x68` 0.0, bytes `+0x70`
/// and `+0x80` 0, `+0x74..+0x7c` zero, `+0x84` the constant 0x00b3eb94,
/// `+0x88` 0.0, `+0x8c` 0x00b92194. Returns `this`.
pub fn text_util_ctor(h: &mut impl Host, frame: u32, this: u32) -> u32 {
    for k in 0..16u32 {
        let v = if k % 5 == 0 { ONE } else { 0 };
        h.write(frame.wrapping_add(4 * k), v);
    }
    for off in (8..0x20u32).step_by(4) {
        h.write(this.wrapping_add(off), 0u32);
    }
    h.call_sym(
        MATRIX_COPY,
        Conv::Thiscall(this.wrapping_add(0x20)),
        &[frame],
    );
    for off in [0x60u32, 0x64, 0x68] {
        h.write(this.wrapping_add(off), 0u32);
    }
    let three = konst(h, 0x00b3_eb94);
    h.write(this.wrapping_add(0x70), 0u8);
    for off in [0x74u32, 0x78, 0x7c] {
        h.write(this.wrapping_add(off), 0u32);
    }
    h.write(this.wrapping_add(0x80), 0u8);
    h.write(this.wrapping_add(0x84), three);
    h.write(this.wrapping_add(0x88), 0u32);
    let half = konst(h, 0x00b9_2194);
    h.write(this.wrapping_add(0x8c), half);
    this
}

/// `PostProcessTweakUtil_vftable`.
pub const TWEAK_VTABLE: u32 = 0x00b4_73fc;

/// `PostProcessTweakUtil_ctor_00692410(this)` (thiscall, `ecx` only): the
/// vtable, byte `+4` 0, `TextUtil_TextUtil(this + 8)`, `+0x168` 0.0,
/// `+0x184` the constant 0x00b1f2b0, `+0x178`, `+0x17c` zero, byte
/// `+0x180` 0, `+0x188` 1280, `+0x18c` 720, 0x58 zero bytes at `+0x98`
/// and at `+0xf0` (`memset`), `+0x16c..+0x174` zero and the eight floats
/// `+0x148..+0x164` 1.0. Returns `this`.
pub fn tweak_util_ctor(h: &mut impl Host, this: u32) -> u32 {
    let at = |o: u32| this.wrapping_add(o);
    let vt = h.rebase(TWEAK_VTABLE);
    h.write(this, vt);
    h.write(at(4), 0u8);
    h.call_sym(TEXT_UTIL_CTOR, Conv::Thiscall(at(8)), &[]);
    h.write(at(0x168), 0u32);
    let k = konst(h, 0x00b1_f2b0);
    h.write(at(0x184), k);
    h.write(at(0x178), 0u32);
    h.write(at(0x17c), 0u32);
    h.write(at(0x180), 0u8);
    h.write(at(0x188), 0x500u32);
    h.write(at(0x18c), 0x2d0u32);
    for base in [0x98u32, 0xf0] {
        for k in 0..0x58 / 4 {
            h.write(at(base + 4 * k), 0u32);
        }
    }
    for off in [0x16cu32, 0x170, 0x174] {
        h.write(at(off), 0u32);
    }
    for k in 0..8u32 {
        h.write(at(0x148 + 4 * k), ONE);
    }
    this
}

// ---- Iggy and GDraw

/// `SetFields_00a3dd00(this; x, y, w, h)` (thiscall).
pub const SET_VIEWPORT: u32 = 0x00a3_dd00;

/// `IggyMenu_setViewport_00a28f40(this; x, y, w, h)` (thiscall): the
/// player's viewport scaled by the floats `+0x74..+0x80`: `x' = ftol(w *
/// +0x74 + x)`, `y' = ftol(h * +0x78 + y)`, `w' = ftol(w * +0x7c)`, `h' =
/// ftol(h * +0x80)` (each integer as an `f32` first), kept at
/// `+0x84..+0x90` and handed to `SetFields_00a3dd00([this + 0x6c]; x',
/// y', w', h')`, whose `eax` it returns.
pub fn iggy_set_viewport(h: &mut impl Host, this: u32, [x, y, w, hh]: [u32; 4]) -> u32 {
    let at = |o: u32| this.wrapping_add(o);
    let fw = load(int_f32(w.cast_signed()));
    let x1 = add(
        mul(load(h.read(at(0x74))), fw),
        load(int_f32(x.cast_signed())),
    );
    let x1 = ftol(h, x1);
    h.write(at(0x84), x1);
    let fh = load(int_f32(hh.cast_signed()));
    let y1 = add(
        mul(load(h.read(at(0x78))), fh),
        load(int_f32(y.cast_signed())),
    );
    let y1 = ftol(h, y1);
    h.write(at(0x88), y1);
    let w1 = ftol(h, mul(fw, load(h.read(at(0x7c)))));
    h.write(at(0x8c), w1);
    let h1 = ftol(h, mul(fh, load(h.read(at(0x80)))));
    h.write(at(0x90), h1);
    let player: u32 = h.read(at(0x6c));
    h.call_sym(SET_VIEWPORT, Conv::Thiscall(player), &[x1, y1, w1, h1])
}

/// The `GDraw` state (`0x01a858b4`): the target's width at `+0x6c0`, its
/// height at `+0x6c4`, rectangles of 16 bytes from `+0x630`.
pub const GDRAW_STATE: Global<u32> = Global::new(0x01a8_58b4);

/// `GDraw_SubRectUV_00a3c1b0(out, slot)` (cdecl): the UVs of the
/// rectangle (i32 x0, y0, x1, y1) that belongs to `slot` (an entry of
/// 0x38 bytes from state `+0x438`; its rectangle at `+0x630 + 16 * n`),
/// half a texel inside: `(x0 - K) / w`, `(y0 - K) / h`, `(x1 + K) / w`,
/// `(y1 + K) / h` with `K` the `f64` 0x00b92220 and `w`, `h` the
/// target's size. Returns `out`.
pub fn gdraw_sub_rect_uv(h: &mut impl Host, out: u32, slot: u32) -> u32 {
    let g = GDRAW_STATE.get(h);
    let ht = load(int_f32(h.read::<u32>(g.wrapping_add(0x6c4)).cast_signed()));
    let n = slot.wrapping_sub(g).wrapping_sub(0x438).cast_signed() / 0x38;
    let rect = (n.wrapping_add(99) << 4).cast_unsigned().wrapping_add(g);
    let word: [f64; 4] = std::array::from_fn(|k| {
        let v: u32 = h.read(rect.wrapping_add(4 * k as u32));
        load(int_f32(v.cast_signed()))
    });
    let k = konst64(h, 0x00b9_2220);
    let v0 = store(div(sub(word[1], k), ht));
    let g = GDRAW_STATE.get(h);
    let wt = load(int_f32(h.read::<u32>(g.wrapping_add(0x6c0)).cast_signed()));
    let u1 = store(div(add(word[2], k), wt));
    let v1 = store(div(add(word[3], k), ht));
    let u0 = store(div(sub(word[0], k), wt));
    h.write(out, u0);
    h.write(out.wrapping_add(4), v0);
    h.write(out.wrapping_add(8), u1);
    h.write(out.wrapping_add(0xc), v1);
    out
}
