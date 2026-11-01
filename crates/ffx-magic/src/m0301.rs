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
