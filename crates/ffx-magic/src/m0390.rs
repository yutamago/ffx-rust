//! `magic_0390.dll`'s own functions (besides the runtime and the object
//! host, `l5_objhost`). `G_STATE` is the effect's state block (the
//! pointer at operand `+0x03` of [`e13f0`]), `G_H` the handle the slots
//! get (a word of the DLL's data).

use crate::l5_objhost::{ONE, run_objects};
use crate::runtime::slot_target;
use crate::sb_sse::{add, cvt, div, mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The x87 indefinite an `fstp` of an empty stack stores (a call-out
/// that left no float).
const INDEFINITE: f32 = f32::from_bits(0xffc0_0000);

/// A task step (stored at 0x100021ef, `(task)`): runs the objects
/// ([`run_objects`]); when they are all done and `G_STATE + 0xc7c` is 1,
/// slot 214 `(G_ARG)` and its result. Otherwise `run_objects`' result.
/// (`G_STATE` is read before the objects run.)
pub fn e13f0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let state = g.read::<u32>(w(0x03));
    // push esi; call
    let r = run_objects(g, img, img.rel32_target(f + 0x08), fr.esp - 8);
    if r != 1 || g.read::<u32>(state + 0xc7c) != 1 {
        return Ret::eax(r);
    }
    let s214 = slot_target(g, w(0x1a), 214);
    let arg = g.read::<u32>(w(0x20));
    Ret::eax(g.call(s214, state, &[arg]).eax)
}

/// 0x100013f0 (49 bytes, 1 overlay).
pub const E13F0: Sig = Sig {
    len: 49,
    hash: 0x4ffc_9c5f_dbae_fa74,
    calls: &[(0x08, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS))],
    imports: &[],
};

/// Table entry (0x100015b0, no arguments): slot 623 `(G_H, 0, 0, 0, 0,
/// 0, 0)` into `G_STATE + 0xc70`, returned.
pub fn e15b0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let s623 = slot_target(g, w(0x01), 623);
    let state = g.read::<u32>(w(0x0e));
    let h = g.read::<u32>(w(0x20));
    let r = g.call(s623, fr.ecx, &[h, 0, 0, 0, 0, 0, 0]).eax;
    g.write::<u32>(state + 0xc70, r);
    Ret::eax(r)
}

/// 0x100015b0 (49 bytes, 1 overlay).
pub const E15B0: Sig = Sig {
    len: 49,
    hash: 0x789d_e564_ce55_85d2,
    calls: &[],
    imports: &[],
};

/// An object's `+0x104` callback (stored at 0x1000198d, `(obj)`): slot
/// 237 `(G_H, 3, 0, obj + 0xd0)`, 1.0 into `obj + 0xdc`, slot 236 `(G_H,
/// 3, obj + 0x80)`, then the 3x3 part of the matrix at `obj + 0x80`
/// scaled by `K / s` (`s` slot 277's ST0 stored as `f32`, `K` the float
/// at operand `+0x70`), slot 235 `(m, G_H)` into the local matrix `m`
/// (`ebp - 0x44`), slot 330 `(m, m, pi/2)` and slot 231 `(obj + 0x80,
/// obj + 0x80, m)`; `obj + 0x104` cleared. Each call gets the words the
/// earlier ones left on the stack. Returns slot 231's result.
pub fn e15f0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let ebp = fr.esp - 4;
    let m = ebp - 0x44;
    let (rot, mat) = (obj + 0xd0, obj + 0x80);
    let s237 = slot_target(g, w(0x11), 237);
    let h = g.read::<u32>(w(0x2d));
    let mut stack = vec![h, 3, 0, rot];
    g.call(s237, fr.ecx, &stack);
    let s236 = slot_target(g, w(0x34), 236);
    g.write::<u32>(rot + 0xc, ONE);
    let h = g.read::<u32>(w(0x50));
    stack.splice(0..0, [h, 3, mat]);
    // call width: 7 (three words and the first call's four)
    g.call(s236, fr.ecx, &stack);
    let s277 = slot_target(g, w(0x57), 277);
    let h = g.read::<u32>(w(0x5d));
    stack.insert(0, h);
    // call width: 8
    let s = g
        .call(s277, fr.ecx, &stack)
        .st0
        .map_or(INDEFINITE, |v| v as f32);
    let k = div(g.read::<f32>(w(0x70)), s);
    for i in [0u32, 1, 2, 4, 5, 6, 8, 9, 10] {
        // the third: movaps xmm0, xmm1; mulss xmm0, [esi + 8]
        let m = g.read::<f32>(mat + 4 * i);
        let v = if i == 2 { mul(k, m) } else { mul(m, k) };
        g.write::<f32>(mat + 4 * i, v);
    }
    let s235 = slot_target(g, w(0x93), 235);
    stack.insert(0, m);
    // call width: 9
    g.call(s235, fr.ecx, &stack);
    // add esp, 0x20 leaves `m`, overwritten with pi/2
    let s330 = slot_target(g, w(0x115), 330);
    g.call(s330, fr.ecx, &[m, m, 0x3fc9_0fdb]);
    let s231 = slot_target(g, w(0x126), 231);
    let r = g.call(s231, fr.ecx, &[mat, mat, m, m, m, 0x3fc9_0fdb]).eax;
    g.write::<u32>(obj + 0x104, 0);
    Ret::eax(r)
}

/// 0x100015f0 (337 bytes, 1 overlay).
pub const E15F0: Sig = Sig {
    len: 337,
    hash: 0x2b60_af95_bf8f_f198,
    // __security_check_cookie
    calls: &[(0x149, Callee::Any)],
    imports: &[],
};

/// Table entry (0x10001be0, no arguments): the countdown `G_STATE +
/// 0xc6c` goes down by `*G_DT` (when positive; clamped at 0), `t =
/// n / N`, `u = 1 - t`, and a point on two cubic curves (Bernstein
/// weights, the middle control points from the ends' differences) from
/// `+0xab0` to `+0xac0` (x) and `+0xab8` to `+0xac8` (z); y is `+0xab4`,
/// plus a hump `u t C * +0xc64` while `n < 0x17`. With `G_FLAG` clear
/// and `G_INDEX < 0x20` the point goes to slot 255 `(G_INDEX, &p)` (a
/// local); otherwise slot 11 `(G_H)` picks slot 33 `(G_H, x, y, z)` (0)
/// or slot 32 `(G_H, x, z)`. Returns the last call's result.
#[allow(clippy::similar_names)]
pub fn e1be0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let k = |g: &G, off: u32| g.read::<f32>(w(off));
    let state = g.read::<u32>(w(0x12));
    let n = g.read::<u32>(state + 0xc6c).cast_signed();
    if n > 0 {
        let dt = g.read::<u32>(g.read::<u32>(w(0x21))).cast_signed();
        g.write::<u32>(state + 0xc6c, n.wrapping_sub(dt).cast_unsigned());
    }
    if g.read::<u32>(state + 0xc6c).cast_signed() < 0 {
        g.write::<u32>(state + 0xc6c, 0);
    }
    let n = g.read::<u32>(state + 0xc6c).cast_signed();
    let (c20, c1c, c38) = (k(g, 0xa6), k(g, 0xdd), k(g, 0x75));
    let d28 = f64::from_bits(g.read::<u64>(w(0xf0)));
    let t = div(cvt(n.cast_unsigned()), k(g, 0x59));
    let u = sub(k(g, 0x51), t);
    let ut = mul(u, t);
    let uu = mul(u, u);
    let tt = mul(t, t);
    // cvtps2pd; mulsd; cvtpd2ps (one NaN at most: the constant is a number)
    let rd = |v: f32| (f64::from(v) * d28) as f32;
    // x
    let d = sub(g.read::<f32>(state + 0xac0), g.read::<f32>(state + 0xab0));
    let mut a = mul(mul(mul(ut, uu), c38), mul(d, c20));
    a = add(a, mul(mul(uu, uu), d));
    a = add(a, mul(mul(mul(tt, uu), c38), mul(d, c1c)));
    a = add(a, mul(mul(mul(tt, ut), c38), rd(d)));
    a = add(a, mul(mul(tt, tt), 0.0));
    let x = add(a, g.read::<f32>(state + 0xab0));
    // z
    let z0 = g.read::<f32>(state + 0xab8);
    let d = sub(g.read::<f32>(state + 0xac8), z0);
    let ut = mul(u, t);
    let mut b = mul(mul(mul(ut, uu), c38), mul(d, c20));
    b = add(b, mul(mul(uu, uu), d));
    b = add(b, mul(mul(mul(tt, uu), c38), mul(d, c1c)));
    b = add(b, mul(mul(mul(tt, ut), c38), rd(d)));
    b = add(b, mul(mul(tt, tt), 0.0));
    let z = add(b, z0);
    let mut y = g.read::<f32>(state + 0xab4);
    if n < 0x17 {
        let t = div(cvt(n.cast_unsigned()), k(g, 0x1fc));
        let u = sub(k(g, 0x1f4), t);
        let mut v = mul(mul(mul(u, t), k(g, 0x213)), g.read::<f32>(state + 0xc64));
        v = add(v, mul(mul(u, u), 0.0));
        v = add(v, mul(mul(t, t), 0.0));
        y = add(v, y);
    }
    let p = fr.esp - 4 - 0x14;
    g.write::<f32>(p, x);
    g.write::<f32>(p + 4, y);
    g.write::<f32>(p + 8, z);
    let index = g.read::<u32>(w(0x246));
    if g.read::<u32>(w(0x23e)) == 0 && index.cast_signed() < 0x20 {
        let s255 = slot_target(g, w(0x255), 255);
        return Ret::eax(g.call(s255, p, &[index, p]).eax);
    }
    let s11 = slot_target(g, w(0x273), 11);
    let h = g.read::<u32>(w(0x27a));
    let (x, y, z) = (x.to_bits(), y.to_bits(), z.to_bits());
    if g.call(s11, state, &[h]).eax == 0 {
        let s33 = slot_target(g, w(0x28f), 33);
        Ret::eax(g.call(s33, state, &[h, x, y, z]).eax)
    } else {
        let s32 = slot_target(g, w(0x28f), 32);
        Ret::eax(g.call(s32, state, &[h, x, z]).eax)
    }
}

/// 0x10001be0 (764 bytes, 1 overlay).
pub const E1BE0: Sig = Sig {
    len: 764,
    hash: 0xc89b_701e_aac5_dcab,
    // __security_check_cookie
    calls: &[
        (0x26a, Callee::Any),
        (0x2c6, Callee::Any),
        (0x2f4, Callee::Any),
    ],
    imports: &[],
};

/// Table entry (0x10002100, no arguments): slot 624 `(G_STATE + 0xc70)`,
/// then `G_STATE + 0xc70 = -1`. Returns slot 624's result.
pub fn e2100<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let s624 = slot_target(g, w(0x01), 624);
    let state = g.read::<u32>(w(0x08));
    let v = g.read::<u32>(state + 0xc70);
    let r = g.call(s624, fr.ecx, &[v]).eax;
    g.write::<u32>(state + 0xc70, 0xffff_ffff);
    Ret::eax(r)
}

/// 0x10002100 (41 bytes, 1 overlay).
pub const E2100: Sig = Sig {
    len: 41,
    hash: 0x5e75_d4d6_6962_683f,
    calls: &[],
    imports: &[],
};
