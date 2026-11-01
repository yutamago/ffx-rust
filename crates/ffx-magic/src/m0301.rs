//! `magic_0301.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// An object's motion step (0x100019c0, cdecl `(obj)`): with `C` = `*G`
/// (`+0x05`), slot 460 `(C[0x198], C[0x19c + 4 C[0x19c]], 2, 2)` and slot
/// 447 `(C[0x198], 0xb1)`, then slots 450 `(obj, F)` (`+0x51`) and 451
/// `(obj, 12)`. Returns slot 451's `eax`. Each slot call passes the words
/// the original leaves on the stack.
pub fn motion_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let c = g.read::<u32>(w(0x05));
    let i = g.read::<u32>(c + 0x19c);
    let m = g.read::<u32>(c + 0x198);
    let mut p = Pushes::default();
    p.push_all(&[2, 2]);
    p.push(g.read::<u32>(c.wrapping_add(i.wrapping_mul(4)).wrapping_add(0x19c)));
    p.push(m);
    let s = slot_target(g, w(0x22), 0x730 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    p.push_all(&[0xb1, m]);
    let s = slot_target(g, w(0x30), 0x6fc / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    let obj = fr.arg(g, 0);
    p.push_all(&[w(0x51), obj]);
    let s = slot_target(g, w(0x43), 0x708 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    p.push_all(&[0xc, obj]);
    let s = slot_target(g, w(0x59), 0x70c / 4);
    // call width: 12 (Pushes caps the words at 12)
    Ret::eax(p.call(g, s, 0).eax)
}

/// [`motion_step`] (110 bytes, 1 overlay).
pub const MOTION_STEP: Sig = Sig {
    len: 110,
    hash: 0x45db_a315_a685_13a3,
    calls: &[],
    imports: &[],
};

/// An object's start (0x10002190, cdecl `(obj)`): slot 443 `(obj,
/// 0x40)`, the scale `+0x130..+0x138` 0.05 (`+0x13c` 0), slot 440 `(*C,
/// obj, 0, 2)` (`C` `+0x23`), slots 450 `(obj, F1)` and 449 `(obj, F2)`
/// (`+0x5d`, `+0x70`). Returns slot 449's `eax`. Each slot call passes
/// the words the original leaves on the stack.
pub fn object_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let obj = fr.arg(g, 0);
    let mut p = Pushes::default();
    p.push_all(&[0x40, obj]);
    let s = slot_target(g, w(0x04), 0x6ec / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    let c = g.read::<u32>(w(0x23));
    p.push_all(&[2, 0, obj, c]);
    for (off, v) in [
        (0x130, 0x3d4c_cccd),
        (0x134, 0x3d4c_cccd),
        (0x138, 0x3d4c_cccd),
        (0x13c, 0),
    ] {
        g.write::<u32>(obj + off, v);
    }
    let s = slot_target(g, w(0x18), 0x6e0 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    p.push_all(&[w(0x5d), obj]);
    let s = slot_target(g, w(0x58), 0x708 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    p.push_all(&[w(0x70), obj]);
    let s = slot_target(g, w(0x6b), 0x704 / 4);
    // call width: 12 (Pushes caps the words at 12)
    Ret::eax(p.call(g, s, 0).eax)
}

/// [`object_start`] (131 bytes, 1 overlay).
pub const OBJECT_START: Sig = Sig {
    len: 131,
    hash: 0x1de7_6716_d9b7_2c7c,
    calls: &[],
    imports: &[],
};

/// An object's mark step (0x10002250, cdecl `(obj)`): slot 446
/// `(C[0x1a0 + 4 i], 8)` for each `i` below `C[0x19c]` (`C` = `*G`, `+0x04`, read
/// again each time), then slots 450 `(obj, F)` (`+0x56`) and 451 `(obj,
/// 0x17)`. Returns slot 451's `eax`.
pub fn mark_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let mut c = g.read::<u32>(w(0x04));
    let mut i = 0u32;
    while i.cast_signed() < g.read::<i32>(c + 0x19c) {
        let v = g.read::<u32>(c.wrapping_add(0x1a0 + 4 * i));
        let s = slot_target(g, w(0x26), 0x6f8 / 4);
        g.call(s, 0, &[v, 8]);
        c = g.read::<u32>(w(0x33));
        i += 1;
    }
    let obj = fr.arg(g, 0);
    let s = slot_target(g, w(0x48), 0x708 / 4);
    g.call(s, 0, &[obj, w(0x56)]);
    let s = slot_target(g, w(0x5e), 0x70c / 4);
    // call width: 4 (the earlier call's words under it)
    Ret::eax(g.call(s, 0, &[obj, 0x17, obj, w(0x56)]).eax)
}

/// [`mark_step`] (115 bytes, 1 overlay).
pub const MARK_STEP: Sig = Sig {
    len: 115,
    hash: 0xf976_7125_cf0a_eaeb,
    calls: &[],
    imports: &[],
};

/// An effect object's set-up (0x100020c0, cdecl `(a)`): slot 443 `(a,
/// 0x40)`; `a + 0x130..0x138` 0.05 and `+0x13c` 0; slot 440 `(*G_E, a, 0,
/// 2)` (operand `+0x23`), slot 450 `(a, F)` (`F` the immediate `+0x5d`)
/// and slot 451 `(a, 9)`, each over the earlier calls' words. Returns
/// slot 451's `eax`.
pub fn object_setup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let a = fr.arg(g, 0);
    let mut s = Pushes::default();
    s.push_all(&[0x40, a]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x4), 443), fr.ecx);
    let e = g.read::<u32>(w(0x23));
    s.push_all(&[2, 0, a, e]);
    for o in [0x130, 0x134, 0x138] {
        g.write::<u32>(a + o, 0x3d4c_cccd);
    }
    g.write::<u32>(a + 0x13c, 0);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x18), 440), fr.ecx);
    s.push_all(&[w(0x5d), a]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x58), 450), fr.ecx);
    s.push_all(&[9, a]);
    // call width: 12 (Pushes caps the words at 12)
    Ret::eax(s.call(g, slot_target(g, w(0x6b), 451), fr.ecx).eax)
}

/// [`object_setup`] (128 bytes, 1 overlay).
pub const OBJECT_SETUP: Sig = Sig {
    len: 128,
    hash: 0xf4fd_d251_ddb9_4bb7,
    calls: &[],
    imports: &[],
};

/// The callbacks' set-up (0x10002140, cdecl `(a)`): slot 444 `(1)`, slot
/// 450 `(a, F1)` and slot 449 `(a, F2)` (`F1`, `F2` the immediates
/// `+0x18`, `+0x2d`), each over the earlier words. Returns slot 449's
/// `eax`.
pub fn callbacks_setup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let a = fr.arg(g, 0);
    let mut s = Pushes::default();
    s.push(1);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x4), 444), fr.ecx);
    s.push_all(&[w(0x18), a]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x13), 450), fr.ecx);
    s.push_all(&[w(0x2d), a]);
    // call width: 12 (Pushes caps the words at 12)
    Ret::eax(s.call(g, slot_target(g, w(0x28), 449), fr.ecx).eax)
}

/// [`callbacks_setup`] (65 bytes, 1 overlay).
pub const CALLBACKS_SETUP: Sig = Sig {
    len: 65,
    hash: 0x9e3b_c76d_1bd5_4356,
    calls: &[],
    imports: &[],
};

/// An effect object aimed at its point (0x10001ea0, cdecl `(a)`): with
/// the point `P` = `T + 16 * a[0x2c + 16 * a[0xac]]` (`T` the immediate
/// `+0x23`) and `K` the float at `+0x07`: `a + 0xe0..0xec` = `((P.x -
/// a.x) K, (P.y - a.y) K - K2, (P.z - a.z) K, 0)` (`a.x` = `a[0xd0]`, `K2`
/// at `+0x65`), `+0xf0..0xfc` `(0, 0.6, 0, 0)`, `+0x130..0x138` 0.06 and
/// `+0x13c` 0; slot 440 `(*G_E, a, 0, 0)` (operand `+0x55`), slot 450 `(a,
/// F)` (`F` the immediate `+0xed`) and slot 451 `(a, 0x14)`, each over the
/// earlier words. Returns slot 451's `eax`.
pub fn aimed_setup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let a = fr.arg(g, 0);
    let k = g.read::<f32>(w(0x7));
    let i = g.read::<u32>(a + 0xac);
    let e = g.read::<u32>(a.wrapping_add(i.wrapping_mul(16)).wrapping_add(0x2c));
    let p = e.wrapping_shl(4).wrapping_add(w(0x23));
    let y = sub(
        mul(sub(g.read::<f32>(p + 4), g.read::<f32>(a + 0xd4)), k),
        g.read::<f32>(w(0x65)),
    );
    let x = mul(sub(g.read::<f32>(p), g.read::<f32>(a + 0xd0)), k);
    let z = mul(sub(g.read::<f32>(p + 8), g.read::<f32>(a + 0xd8)), k);
    let mut s = Pushes::default();
    s.push_all(&[0, 0, a, g.read::<u32>(w(0x55))]);
    g.write::<f32>(a + 0xe0, x);
    g.write::<u32>(a + 0xec, 0);
    g.write::<f32>(a + 0xe4, y);
    g.write::<f32>(a + 0xe8, z);
    for (o, v) in [(0xf0u32, 0u32), (0xf4, 0x3f19_999a), (0xf8, 0), (0xfc, 0)] {
        g.write::<u32>(a + o, v);
    }
    for o in [0x130u32, 0x134, 0x138] {
        g.write::<u32>(a + o, 0x3d75_c28f);
    }
    g.write::<u32>(a + 0x13c, 0);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x4f), 440), fr.ecx);
    s.push_all(&[w(0xed), a]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0xe8), 450), fr.ecx);
    s.push_all(&[0x14, a]);
    // call width: 12 (Pushes caps the words at 12)
    Ret::eax(s.call(g, slot_target(g, w(0xfb), 451), fr.ecx).eax)
}

/// [`aimed_setup`] (272 bytes, 1 overlay).
pub const AIMED_SETUP: Sig = Sig {
    len: 272,
    hash: 0xfab1_2c23_376b_2a3d,
    calls: &[],
    imports: &[],
};

/// A thrown object from a step of its path (0x10001fb0, cdecl `(a)`):
/// `e = a[0x2c + 16 a[0xac]]` (written over the argument word); slot 434
/// `(*G_E, F)` (`*G_E` operand `+0x35`; `F` the immediate `+0x28` for an
/// odd `e`, `+0x2f` else) gives the object `O`; slot 477 `(T[e / 2])`
/// (`T` the immediate `+0x4b`, ST0 kept as f32 at `ebp - 4`) gives `v`;
/// `O + 0xd0` = `P[e].x`, `+0xd4` = `P[e].y - K1`, `+0xd8` = `P[e].z`
/// (`P` the 16-byte points at the immediates `+0x80`, `+0x8e`, `+0xad`;
/// `K1` at `+0x96`), `+0xdc`, `+0x100`, `+0x108`, `+0x10c` 0 and `+0x104`
/// = `(-v - K2) K3` (the sign mask at `+0x6a`, `K2`/`K3` at `+0x72`/`+0x9e`);
/// then slot 450 `(a, F1)` and slot 449 `(a, F2)` over slot 477's word
/// (the immediates `+0x7a`, `+0xfa`). Returns slot 449's `eax`.
pub fn thrown_setup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let ebp = fr.esp.wrapping_sub(4);
    let a = fr.arg(g, 0);
    let i = g.read::<u32>(a + 0xac);
    let e = g.read::<u32>(a.wrapping_add(i.wrapping_mul(16)).wrapping_add(0x2c));
    g.write::<u32>(fr.esp + 4, e);
    let cb = if e & 1 == 0 { w(0x2f) } else { w(0x28) };
    let t = slot_target(g, w(0x1b), 434);
    let obj = g.call(t, 0, &[g.read::<u32>(w(0x35)), cb]).eax;
    let half = (e.cast_signed() / 2).cast_unsigned();
    let x = g.read::<u32>(w(0x4b).wrapping_add(half.wrapping_mul(4)));
    let t = slot_target(g, w(0x50), 477);
    let v = crate::m0018::st0_f32(g.call(t, 0, &[x]).st0);
    g.write::<f32>(ebp.wrapping_sub(4), v);
    let neg = f32::from_bits(v.to_bits() ^ g.read::<u32>(w(0x6a)));
    let r = mul(sub(neg, k(g, 0x72)), k(g, 0x9e));
    let c = e.wrapping_shl(4);
    let px = g.read::<u32>(c.wrapping_add(w(0x80)));
    g.write::<u32>(obj + 0xd0, px);
    let py = sub(g.read::<f32>(c.wrapping_add(w(0x8e))), k(g, 0x96));
    g.write::<f32>(obj + 0xd4, py);
    let pz = g.read::<u32>(c.wrapping_add(w(0xad)));
    g.write::<u32>(obj + 0xd8, pz);
    g.write::<u32>(obj + 0xdc, 0);
    g.write::<u32>(obj + 0x100, 0);
    g.write::<f32>(obj + 0x104, r);
    g.write::<u32>(obj + 0x108, 0);
    g.write::<u32>(obj + 0x10c, 0);
    let mut s = Pushes::default();
    s.push_all(&[x, w(0x7a), a]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0xb8), 450), fr.ecx);
    s.push_all(&[w(0xfa), a]);
    // call width: 12 (Pushes caps the words at 12)
    Ret::eax(s.call(g, slot_target(g, w(0xf5), 449), fr.ecx).eax)
}

/// [`thrown_setup`] (272 bytes, 1 overlay).
pub const THROWN_SETUP: Sig = Sig {
    len: 272,
    hash: 0x5531_a7a2_b8a9_8937,
    calls: &[],
    imports: &[],
};
