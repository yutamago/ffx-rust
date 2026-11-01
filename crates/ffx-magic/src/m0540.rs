//! `magic_0540.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::part_draw::{Camera, camera, start_part};
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `1.0f32`'s bits.
const ONE: u32 = 0x3f80_0000;

/// Slot 276 `(model, on)` for each model of the table from operand
/// `start` to `end` (the table of slot 276 at `t`).
fn models<G: Game>(g: &mut G, img: &Image, f: u32, (start, end, t): (u32, u32, u32), on: u32) {
    let w = |off: u32| img.word(f + off);
    let mut p = w(start);
    loop {
        let s = slot_target(g, w(t), 276);
        let m = g.read::<u32>(p);
        g.call(s, 0, &[m, on]);
        p = p.wrapping_add(4);
        if p.cast_signed() >= w(end).cast_signed() {
            break;
        }
    }
}

/// An object's start (0x100010a0, `(obj)`): program 6 started
/// ([`start_part`]), its point `(0.2, -30.4, 139, 1)`, `+0x50..` 0,
/// `+0x5c` 1, the scales 0.1756 (`+0x6c` 1.0); the effect's models shown
/// (slot 276 `(model, 1)`, [`models`]) and slot 845 `()`. Returns 1.
pub fn e10a0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    start_part(g, img, f, obj, [0x4, 0xb, 0x10, 0x18, 0x36], 0x18, &[]);
    for (off, v) in [
        (0x40u32, 0x3e4c_cccdu32),
        (0x44, 0xc1f3_3332),
        (0x48, 0x430b_0000),
        (0x4c, ONE),
        (0x50, 0),
        (0x54, 0),
        (0x58, 0),
        (0x5c, 1),
        (0x60, 0x3e33_cf70),
        (0x64, 0x3e33_cf70),
        (0x68, 0x3e33_cf70),
        (0x6c, ONE),
    ] {
        g.write::<u32>(obj + off, v);
    }
    models(g, img, f, (0x92, 0xaf, 0x97), 1);
    let t = slot_target(g, w(0xb6), 845);
    g.call(t, 0, &[]);
    Ret::eax(1)
}

/// 0x100010a0 (202 bytes, 1 overlay).
pub const E10A0: Sig = Sig {
    len: 202,
    hash: 0x7ae8_9055_6f38_13e9,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// An object's frame (0x10001170, `(obj)`): slot 382 `(0, 0)`, slot 315
/// `(0)`, slot 845 `()`, slot 258 `(obj)`, slots 231 and 230 with the
/// camera globals, `*G_P = ITEM` and slot 712 `(obj, 0x30)` with the
/// game flag `*G_LOCK` 1 (restored after; the earlier words left on the
/// stack); slot 275 `(1)`. Returns 1 when slot 712 says 0, else 0.
pub fn e1170<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let obj = fr.arg(g, 0);
    let t = slot_target(g, w(0x4), 382);
    let mut stack = vec![0, 0];
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x16), 315);
    stack.insert(0, 0);
    // call width: 3
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x25), 845);
    // call width: 3
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x32), 258);
    stack.insert(0, obj);
    // call width: 4
    g.call(t, 0, &stack);
    for op in [0x43, 0x4e, 0x5a] {
        stack.insert(0, g.read::<u32>(w(op)));
    }
    let t = slot_target(g, w(0x48), 231);
    // call width: 7
    g.call(t, 0, &stack);
    for op in [0x62, 0x6d] {
        stack.insert(0, g.read::<u32>(w(op)));
    }
    let t = slot_target(g, w(0x67), 230);
    // call width: 9
    g.call(t, 0, &stack);
    let gp = g.read::<u32>(w(0x7a));
    g.write::<u32>(gp, w(0x85));
    let lock = g.read::<u32>(w(0x8a));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let t = slot_target(g, w(0x97), 712);
    stack.splice(0..0, [obj, 0x30]);
    // call width: 11
    let r = g.call(t, 0, &stack).eax;
    let lock = g.read::<u32>(w(0xa5));
    g.write::<u32>(lock, saved);
    let t = slot_target(g, w(0xaf), 275);
    g.call(t, 0, &[1]);
    Ret::eax(u32::from(r == 0))
}

/// 0x10001170 (213 bytes, 1 overlay).
pub const E1170: Sig = Sig {
    len: 213,
    hash: 0x0bec_7190_26aa_4ebd,
    calls: &[],
    imports: &[],
};

/// An object's end (0x10001250, `(obj)`): the effect's models hidden
/// (slot 276 `(model, 0)`, [`models`]), `*G_P = ITEM`, slot 713 `(obj)`
/// removes the part and slot 846 `()`. Returns 1.
pub fn e1250<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    models(g, img, f, (0x5, 0x29, 0x11), 0);
    let obj = fr.arg(g, 0);
    let gp = g.read::<u32>(w(0x30));
    g.write::<u32>(gp, w(0x39));
    let t = slot_target(g, w(0x3e), 713);
    g.call(t, 0, &[obj]);
    let t = slot_target(g, w(0x4b), 846);
    g.call(t, 0, &[]);
    Ret::eax(1)
}

/// 0x10001250 (98 bytes, 1 overlay).
pub const E1250: Sig = Sig {
    len: 98,
    hash: 0x78c4_0d9d_850e_9e1c,
    calls: &[],
    imports: &[],
};

/// An object's start (0x100012c0, `(obj)`): the first record of the
/// array at operand `+0x0a` starts program 3 (`y` -2.0, the scales
/// 0.0986), the others program 4 (`y` -35.4, 0.0796) ([`start_part`]);
/// then `+0x6c` 1.0, `+0x50..+0x5c` `(0, 0, 0, 1)`, `+0x4c` 1.0, `z`
/// 135, `x` -0.8. Returns 1.
pub fn e12c0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let first = obj.wrapping_sub(w(0xa)) & 0xffff_ff80 == 0;
    let (call, index, y, s) = if first {
        (0x44, 0xc, 0xbfff_ffe7, 0x3dc9_ecb3)
    } else {
        (0x77, 0x10, 0xc20d_993e, 0x3da3_033a)
    };
    start_part(g, img, f, obj, [0x14, 0x1a, 0x1f, 0x27, call], index, &[]);
    g.write::<u32>(obj + 0x44, y);
    for off in [0x60, 0x64, 0x68] {
        g.write::<u32>(obj + off, s);
    }
    for (off, v) in [
        (0x6cu32, ONE),
        (0x5c, 1),
        (0x58, 0),
        (0x54, 0),
        (0x50, 0),
        (0x4c, ONE),
        (0x48, 0x4307_0000),
        (0x40, 0xbf4c_ccbc),
    ] {
        g.write::<u32>(obj + off, v);
    }
    Ret::eax(1)
}

/// 0x100012c0 (214 bytes, 1 overlay).
pub const E12C0: Sig = Sig {
    len: 214,
    hash: 0xc55e_83bb_9c05_c63a,
    calls: &[
        (0x44, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x77, Callee::Sig(&crate::opu_parts::START_PART)),
    ],
    imports: &[],
};

/// The effect's frame (0x10001a00, an overlay table entry, no
/// arguments): 0 while the flag at operand `+0x02` is clear. Else the
/// matrices ([`camera`]), `*G_V = *G_W`; the first time slot 261 `(3)`
/// says nonzero (the count at `+0x62` 0), slot 417 `(CB, 0, 0, 0)`
/// starts a task (`+0x8b`; the count up); slot 261 `(3)` and `(2)` each
/// run a timeline ([`timeline_run`](crate::sb_misc::timeline_run)) when
/// they say nonzero; when `(1)` does, slot 418 `(task)`, the three
/// timelines stop ([`timeline_stop`](crate::sb_misc::timeline_stop)) and
/// slot 214 `(*G_E)` runs; else the frame count `+0x134` goes up.
/// Returns 1.
pub fn frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    if g.read::<u32>(w(0x2)) == 0 {
        return Ret::eax(0);
    }
    let c = Camera {
        gp: 0xd,
        cam: 0x13,
        item: 0x19,
        t723: 0x1e,
        t724: 0x2b,
        m: 0x31,
    };
    let mut stack = camera(g, img, f, &c, &[]);
    let v = g.read::<u32>(w(0x3f));
    let x = g.read::<u32>(w(0x44));
    g.write::<u32>(v, x);
    let t = slot_target(g, w(0x4b), 261);
    stack.insert(0, 3);
    // call width: 3
    if g.call(t, 0, &stack).eax != 0 && g.read::<u32>(w(0x62)) == 0 {
        let t = slot_target(g, w(0x6a), 417);
        let h = g.call(t, 0, &[w(0x7b), 0, 0, 0]).eax;
        let n = w(0x86);
        g.write::<u32>(n, g.read::<u32>(n).wrapping_add(1));
        g.write::<u32>(w(0x8b), h);
    }
    for (tab, k, call, this) in [(0x90u32, 3u32, 0xabu32, 0xa6u32), (0xb0, 2, 0xcb, 0xc6)] {
        let t = slot_target(g, w(tab), 261);
        if g.call(t, 0, &[k]).eax != 0 {
            crate::sb_misc::timeline_run(g, img, img.rel32_target(f + call), w(this));
        }
    }
    let t = slot_target(g, w(0xd0), 261);
    if g.call(t, 0, &[1]).eax == 0 {
        let n = w(0x134);
        g.write::<u32>(n, g.read::<u32>(n).wrapping_add(1));
        return Ret::eax(1);
    }
    let t = slot_target(g, w(0xe6), 418);
    let task = g.read::<u32>(w(0xec));
    g.call(t, 0, &[task]);
    for op in [0xf9, 0x103, 0x10d] {
        crate::sb_misc::timeline_stop(g, w(op));
    }
    let t = slot_target(g, w(0x117), 214);
    let e = g.read::<u32>(w(0x11d));
    g.call(t, 0, &[e, task]);
    Ret::eax(1)
}

/// 0x10001a00 (318 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 318,
    hash: 0xe93c_b381_8f5b_5b44,
    calls: &[
        (0xab, Callee::Sig(&crate::sb_misc::TIMELINE_RUN)),
        (0xcb, Callee::Sig(&crate::sb_misc::TIMELINE_RUN)),
        (0xfe, Callee::Sig(&crate::sb_misc::TIMELINE_STOP)),
        (0x108, Callee::Sig(&crate::sb_misc::TIMELINE_STOP)),
        (0x112, Callee::Sig(&crate::sb_misc::TIMELINE_STOP)),
    ],
    imports: &[],
};
