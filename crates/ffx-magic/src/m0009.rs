//! `magic_0009.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::{Callee, Game, Image, Sig};

/// `1.0f32`'s bits.
const ONE: u32 = 0x3f80_0000;

/// Sets up the four 0x250-byte records of the effect block `G_FX`
/// (operand `+0x11`) and its model (0x100019d0, internal: `ecx` = a
/// position `pos` (4 words), `edx` = another position `from`, `arg` the
/// stack argument, a rotation (3 floats); `esp` at the return address).
/// Two slot 719 lookups `(NAME, 9, 0)` and `(NAME, 9, 1)` (the first
/// call's words left on the stack) give the records' two handles; each
/// record gets them, `pos`, flags and colours, 32 copies of `pos`, and a
/// word of the table at operand `+0x64`. Then slot 235 `(G_FX)` and the
/// rotations 330, 328, 329 `(G_FX, G_FX, arg[2] / arg[0] / arg[1])`,
/// `pos` at `G_FX + 0x60`, the distance `|pos - from|` (`sqrt`) at
/// `+0x58` and the rest of the model's words.
pub fn setup<G: Game>(g: &mut G, img: &Image, f: u32, pos: u32, from: u32, arg: u32) {
    let w = |off: u32| img.word(f + off);
    let fx = g.read::<u32>(w(0x11));
    let name = w(0x29);
    let s719 = slot_target(g, w(0x1a), 719);
    let a = g.call(s719, pos, &[name, 9, 0]).eax;
    let s719 = slot_target(g, w(0x3b), 719);
    let name2 = w(0x4a);
    // call width: 6 (three words and the first call's three)
    let b = g.call(s719, pos, &[name2, 9, 1, name, 9, 0]).eax;
    let p = [0u32, 4, 8, 12].map(|o| g.read::<u32>(pos + o));
    let table = w(0x64);
    let end = w(0x171);
    let mut rec = fx.wrapping_add(0x2a2);
    let mut at = table;
    loop {
        g.write::<u32>(rec - 0x232, a);
        g.write::<u16>(rec - 0x22a, 0xffff);
        g.write::<u32>(rec - 0x22e, 1);
        g.write::<u16>(rec - 0x228, 0x200);
        for (i, v) in (0u32..).zip(p) {
            g.write::<u32>(rec - 0x226 + 4 * i, v);
        }
        g.write::<u32>(rec - 0x216, 0x8080_8080);
        g.write::<u32>(rec - 0x212, 0x3e4c_cccd);
        g.write::<u32>(rec - 0x20e, b);
        g.write::<u16>(rec - 0x206, 0xffff);
        g.write::<u32>(rec - 0x20a, 1);
        g.write::<u16>(rec - 0x204, 0x200);
        g.write::<u32>(rec - 0x2, 0x8080_8080);
        g.write::<u32>(rec + 0x2, 0);
        g.write::<u32>(rec + 0x12, 0);
        g.write::<u32>(rec + 0x6, ONE);
        g.write::<u32>(rec + 0xa, ONE);
        for k in 0..32u32 {
            for (i, v) in (0u32..).zip(p) {
                g.write::<u32>(rec - 0x202 + 0x10 * k + 4 * i, v);
            }
        }
        g.write::<u32>(rec - 0x2, 0x3050_7896);
        g.write::<u32>(rec + 0x2, 0x0864_3c78);
        g.write::<u32>(rec + 0x16, 0x46);
        g.write::<u32>(rec + 0xe, 0x3f0c_cccd);
        let v = g.read::<u32>(at);
        g.write::<u32>(rec + 0x1a, v);
        at = at.wrapping_add(4);
        rec = rec.wrapping_add(0x250);
        // cmp ebx, END; jl
        if at.cast_signed() >= end.cast_signed() {
            break;
        }
    }
    let s235 = slot_target(g, w(0x17c), 235);
    g.call(s235, rec, &[fx]);
    for (t, slot, o) in [(0x192, 330u32, 8u32), (0x1af, 328, 0), (0x1cb, 329, 4)] {
        let s = slot_target(g, w(t), slot);
        let r = g.read::<u32>(arg + o);
        g.call(s, rec, &[fx, fx, r]);
    }
    for (i, v) in (0u32..).zip(p) {
        g.write::<u32>(fx + 0x60 + 4 * i, v);
    }
    for (o, v) in [
        (0x40u32, 0u32),
        (0x44, 0),
        (0x48, 0),
        (0x4c, ONE),
        (0x50, 0),
        (0x54, 0),
    ] {
        g.write::<u32>(fx + o, v);
    }
    let d = |o: u32| g.read::<f32>(pos + o) - g.read::<f32>(from + o);
    let (dx, dy, dz) = (d(0), d(4), d(8));
    let s = dy * dy + dx * dx + dz * dz;
    g.write::<f32>(fx + 0x58, f64::from(s).sqrt() as f32);
    g.write::<u32>(fx + 0x5c, ONE);
    g.write::<u32>(fx + 0x9b0, 0);
    g.write::<u32>(fx + 0x9b4, 0);
    g.write::<u32>(fx + 0x9b8, 0x14);
}

/// [`setup`] (662 bytes, 1 overlay).
pub const SETUP: Sig = Sig {
    len: 662,
    hash: 0x956c_ecbc_7857_627b,
    calls: &[(0x25d, Callee::Import("_libm_sse2_sqrt_precise"))],
    imports: &[],
};

/// Fills 0x46 entries of a model (0x10002aa0, internal: `ecx` = the
/// model): for each index, slot 772 `(model, i)` gives a 64-bit value
/// (`eax:edx`) that goes to slot 779 `(lo, hi, model, i)` (the first
/// call's words left on the stack), the game flag `*G_LOCK` (operand
/// `+0x21`) set to 1 around the call and restored after.
pub fn fill<G: Game>(g: &mut G, img: &Image, f: u32, model: u32) {
    let w = |off: u32| img.word(f + off);
    for i in 0..0x46u32 {
        let s772 = slot_target(g, w(0x11), 772);
        let r = g.call(s772, model, &[model, i]);
        let lock = g.read::<u32>(w(0x21));
        let saved = g.read::<u32>(lock);
        g.write::<u32>(lock, 1);
        let s779 = slot_target(g, w(0x2a), 779);
        g.call(s779, lock, &[r.eax, r.edx, model, i]);
        let lock = g.read::<u32>(w(0x3d));
        g.write::<u32>(lock, saved);
    }
}

/// [`fill`] (80 bytes, 1 overlay).
pub const FILL: Sig = Sig {
    len: 80,
    hash: 0xefad_cf98_910f_56c6,
    calls: &[],
    imports: &[],
};
