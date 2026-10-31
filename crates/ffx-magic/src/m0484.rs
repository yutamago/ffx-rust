//! `magic_0484.dll`'s own functions (besides the runtime and the shared
//! code): the object callbacks of its table, its start-up and frame.

use crate::part_draw::{DrawOps, draw_restart, start_part};
use crate::runtime::slot_target;
use crate::sb_sse::{add, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `1.0f32`'s bits.
const ONE: u32 = 0x3f80_0000;

/// Writes `(addr offset, value)` words into the object.
fn fill<G: Game>(g: &mut G, obj: u32, words: &[(u32, u32)]) {
    for &(off, v) in words {
        g.write::<u32>(obj + off, v);
    }
}

/// An object's start (0x10001000, `(obj)`): the part started (program 0,
/// [`start_part`]), the position `+0x40` `(0, 0, 0, 1)`, `+0x50..` 0,
/// `+0x5c` 1, the scales `+0x60..` 0.2377 (`+0x6c` 1.0); slot 400 `(1)`,
/// slot 401 `(0, 0, 0, 0, 1000.0, 0xff, 0, 0, 0)`, slot 398 `(*G_H, 4)`.
/// Returns slot 398's `eax`.
pub fn e1000<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let below = start_part(g, img, f, obj, [0x4, 0xb, 0x10, 0x18, 0x35], 0, &[]);
    let t = slot_target(g, w(0x3a), 400);
    fill(
        g,
        obj,
        &[
            (0x40, 0),
            (0x44, 0),
            (0x48, 0),
            (0x4c, ONE),
            (0x50, 0),
            (0x54, 0),
            (0x58, 0),
            (0x5c, 1),
            (0x60, 0x3e73_66d8),
            (0x64, 0x3e73_66d8),
            (0x68, 0x3e73_66d8),
            (0x6c, ONE),
        ],
    );
    let mut args = vec![1];
    args.extend(below);
    // call width: 5
    g.call(t, 0, &args);
    let t = slot_target(g, w(0x9d), 401);
    let mut stack = vec![0, 0, 0, 0, 0x447a_0000, 0xff, 0, 0, 0];
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0xdc), 398);
    stack.splice(0..0, [g.read::<u32>(w(0xe4)), 4]);
    Ret::eax(g.call(t, 0, &stack).eax)
}

/// 0x10001000 (246 bytes, 1 overlay).
pub const E1000: Sig = Sig {
    len: 246,
    hash: 0x5149_9221_b0b2_ba11,
    calls: &[(0x35, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// An object's start (0x10001360, `(obj)`): the part started (program 1,
/// [`start_part`]), slot 325 `(obj + 0x40, P)` (`P` at operand `+0x3e`)
/// sets its position, `+0x50..` 0, `+0x5c` 1, the scales `+0x60..` 1.3194
/// (`+0x6c` 1.0). Returns slot 325's `eax`.
pub fn e1360<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let below = start_part(g, img, f, obj, [0x4, 0xb, 0x10, 0x18, 0x36], 4, &[]);
    let t = slot_target(g, w(0x44), 325);
    let mut args = vec![obj + 0x40, w(0x3e)];
    args.extend(below);
    // call width: 6
    let r = g.call(t, 0, &args).eax;
    fill(
        g,
        obj,
        &[
            (0x50, 0),
            (0x54, 0),
            (0x58, 0),
            (0x5c, 1),
            (0x60, 0x3fa8_e1ef),
            (0x64, 0x3fa8_e1ef),
            (0x68, 0x3fa8_e1ef),
            (0x6c, ONE),
        ],
    );
    Ret::eax(r)
}

/// 0x10001360 (142 bytes, 1 overlay).
pub const E1360: Sig = Sig {
    len: 142,
    hash: 0x5695_ba26_db8c_a900,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// An object's frame (0x10001100, `(obj)`; a cookie frame). Up to the
/// effect's frame 0x9b (the count at operand `+0x12`): slot 237 `(*G_H,
/// 0x5d, v, obj + 0x40)` with the local vector `v = (0, 0, 1500, 1)`
/// (`ebp - 0x14`), slot 401 `(0, pos, S, 0xdc, 0xc8, 0xa0, 2)` with the
/// global size `S` (`+0x62`), which then grows by `K` and is capped to 10
/// above `L`. Later: the position's `y` drops by the global `T`
/// (`+0xe5`), `T` by `K2`, and slot 325 `(P, obj + 0x40)`. Then the
/// object is drawn ([`draw_restart`]). Returns 1.
pub fn e1100<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let ebp = fr.esp.wrapping_sub(4);
    let obj = fr.arg(g, 0);
    let pos = obj + 0x40;
    if g.read::<i32>(w(0x12)) <= 0x9b {
        let v = ebp - 0x14;
        let t = slot_target(g, w(0x2e), 237);
        let h = g.read::<u32>(w(0x36));
        for (i, c) in (0u32..).zip([0u32, 0, 0x44bb_8000, ONE]) {
            g.write::<u32>(v + 4 * i, c);
        }
        g.call(t, 0, &[h, 0x5d, v, pos]);
        let s = g.read::<u32>(w(0x62));
        let t = slot_target(g, w(0x67), 401);
        let p = [0u32, 4, 8].map(|o| g.read::<u32>(pos + o));
        g.call(t, 0, &[0, p[0], p[1], p[2], s, 0xdc, 0xc8, 0xa0, 2]);
        let s = add(g.read::<f32>(w(0xb5)), g.read::<f32>(w(0xbd)));
        let limit = g.read::<f32>(w(0xc7));
        g.write::<f32>(w(0xcf), s);
        if s > limit {
            g.write::<u32>(w(0xd7), 0x4120_0000);
        }
    } else {
        let tt = g.read::<f32>(w(0xe5));
        let y = sub(g.read::<f32>(obj + 0x44), tt);
        let tt = sub(tt, g.read::<f32>(w(0xf6)));
        let t = slot_target(g, w(0xff), 325);
        g.write::<f32>(obj + 0x44, y);
        g.write::<f32>(w(0x117), tt);
        g.call(t, 0, &[w(0x10f), pos]);
    }
    let ops = DrawOps {
        t258: 0x121,
        a231: [0x130, 0x13b, 0x147],
        t231: 0x135,
        a230: [0x14f, 0x15a],
        t230: 0x154,
        lock: 0x168,
        gp: 0x16d,
        item: 0x179,
        t712: 0x17e,
        lock2: 0x197,
        gp2: 0x1a5,
        item2: 0x1ac,
        t713: 0x1b1,
        start: [0x1be, 0x1c4, 0x1c9, 0x1d1, 0x1eb],
        index: 0,
        lock3: 0x1f0,
    };
    draw_restart(g, img, f, obj, &ops);
    Ret::eax(1)
}

/// 0x10001100 (538 bytes, 1 overlay).
pub const E1100: Sig = Sig {
    len: 538,
    hash: 0xef61_9dbe_65c6_6543,
    calls: &[
        (0x1eb, Callee::Sig(&crate::opu_parts::START_PART)),
        // __security_check_cookie
        (0x212, Callee::Any),
    ],
    imports: &[],
};

/// An object's frame (0x100013f0, `(obj)`): the object drawn
/// ([`draw_restart`], program 1 on a restart); then slot 391 `(0xa0,
/// 0x96, 0x8b, *G_N)` and the count `*G_N` (operand `+0x10a`) drops by 6,
/// not below 0. Returns 1.
pub fn e13f0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let ops = DrawOps {
        t258: 0x4,
        a231: [0x18, 0x23, 0x2f],
        t231: 0x1d,
        a230: [0x37, 0x42],
        t230: 0x3c,
        lock: 0x50,
        gp: 0x55,
        item: 0x61,
        t712: 0x66,
        lock2: 0x7f,
        gp2: 0x8d,
        item2: 0x94,
        t713: 0x99,
        start: [0xa6, 0xac, 0xb1, 0xb9, 0xd4],
        index: 4,
        lock3: 0xda,
    };
    draw_restart(g, img, f, obj, &ops);
    let n = g.read::<u32>(w(0xe3));
    let t = slot_target(g, w(0xe8), 391);
    g.call(t, 0, &[0xa0, 0x96, 0x8b, n]);
    let n = g.read::<u32>(w(0x10a)).wrapping_sub(6);
    // sub eax, 6; cmovs: 0 when the result's sign is set
    let n = if n.cast_signed() < 0 { 0 } else { n };
    g.write::<u32>(w(0x11e), n);
    Ret::eax(1)
}

/// 0x100013f0 (296 bytes, 1 overlay).
pub const E13F0: Sig = Sig {
    len: 296,
    hash: 0x31fe_3064_1741_5bdc,
    calls: &[(0xd4, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// The effect's start-up (0x10001520, an overlay table entry, `(arg)`):
/// slot 803 `(0x1000000, 0x10)` the buffer (globals `+0x19`, and `+0x23`
/// half a MiB on), `arg` to `+0x2b`; the name (`+0x35`) to `+0x41` and
/// slot 718, slot 222 `(name, 0, 1000, 0x2000)`, slot 719 `(name, 8, 0)`
/// the data `d` (`+0x8c`); slot 711 `(d, PARTS, 0)` with the game flag
/// `*G_LOCK` 1; slot 710 `(ITEM, d, buf, 0x80000)`; three slot 941
/// `(d[5 + k])` fill the program tables; the part block (`+0x127`)
/// cleared and given its defaults; slot 260 `(5)` a handle (`+0x177`)
/// and slot 399 `(h, 1)`. Every call passes the words on the original's
/// stack. Returns 1.
pub fn start_up<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let t = slot_target(g, w(0x4), 803);
    let mut stack = vec![0x0100_0000, 0x10];
    let buf = g.call(t, 0, &stack).eax;
    g.write::<u32>(w(0x19), buf);
    g.write::<u32>(w(0x23), buf.wrapping_add(0x8_0000));
    let arg = fr.arg(g, 0);
    g.write::<u32>(w(0x2b), arg);
    let t = slot_target(g, w(0x30), 718);
    stack.insert(0, w(0x35));
    g.write::<u32>(w(0x41), w(0x45));
    // call width: 17
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x4c), 222);
    let name = g.read::<u32>(w(0x64));
    stack.splice(0..0, [name, 0, 0x3e8, 0x2000]);
    // call width: 17
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x6b), 719);
    let name = g.read::<u32>(w(0x7b));
    stack.splice(0..0, [name, 8, 0]);
    // call width: 17
    let d = g.call(t, 0, &stack).eax;
    let lock = g.read::<u32>(w(0x83));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(w(0x8c), d);
    g.write::<u32>(lock, 1);
    let t = slot_target(g, w(0x97), 711);
    stack.splice(0..0, [d, w(0x91), 0]);
    // call width: 17
    g.call(t, 0, &stack);
    let lock = g.read::<u32>(w(0xaa));
    let b = g.read::<u32>(w(0xb5));
    g.write::<u32>(lock, saved);
    let d = g.read::<u32>(w(0xbd));
    let t = slot_target(g, w(0xc2), 710);
    stack.splice(0..0, [w(0xc7), d, b, 0x8_0000]);
    // call width: 17
    g.call(t, 0, &stack);
    // add esp, 0x44
    let mut stack = Vec::new();
    for (k, (dop, tab, dst)) in (0u32..).zip([
        (0xd4u32, 0xdfu32, 0xecu32),
        (0xf1, 0xf9, 0x106),
        (0x10b, 0x113, 0x12c),
    ]) {
        let d = g.read::<u32>(w(dop));
        stack.insert(0, g.read::<u32>(d.wrapping_add(0x14 + 4 * k)));
        let t = slot_target(g, w(tab), 941);
        // call width: 17
        let r = g.call(t, 0, &stack).eax;
        g.write::<u32>(w(dst), r);
    }
    g.write_bytes(w(0x127), &[0u8; 0x110]);
    stack.splice(0..0, [w(0x127), 0, 0x110]);
    let t = slot_target(g, w(0x136), 260);
    g.write::<u32>(w(0x13c), w(0x140));
    for (op, v) in [(0x146u32, 2u32), (0x150, 2), (0x15a, 1), (0x164, 1)] {
        g.write::<u32>(w(op), v);
    }
    stack.insert(0, 5);
    // call width: 17
    let h = g.call(t, 0, &stack).eax;
    g.write::<u32>(w(0x177), h);
    let t = slot_target(g, w(0x17f), 399);
    stack.splice(0..0, [h, 1]);
    // call width: 17
    g.call(t, 0, &stack);
    Ret::eax(1)
}

/// 0x10001520 (406 bytes, 1 overlay).
pub const START_UP: Sig = Sig {
    len: 406,
    hash: 0x47bd_20b9_04a0_c805,
    calls: &[(0x131, Callee::Import("memset"))],
    imports: &[],
};

/// The effect's frame (0x10001790, an overlay table entry, no arguments):
/// 0 while the flag at operand `+0x02` is 0. Else `*G_P = ITEM`, slots
/// 723 `(*G_CAM)` and 724 `(*G_M)`, `*G_V = *G_W`; when slot 261 `(0)`
/// says nonzero, the timeline step
/// ([`timeline`](crate::sa_misc::timeline)); slot 261 `(1)` and `(2)`;
/// when the last says nonzero, the timeline's stop
/// ([`timeline_stop_global`](crate::sb_misc::timeline_stop_global)),
/// slot 399 `(*G_H, 0)` and slot 214 `(*G_E)`. Otherwise the frame count
/// at `+0xc2` goes up. Returns 1.
pub fn frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    if g.read::<u32>(w(0x2)) == 0 {
        return Ret::eax(0);
    }
    let gp = g.read::<u32>(w(0xd));
    let cam = g.read::<u32>(w(0x13));
    g.write::<u32>(gp, w(0x19));
    let t = slot_target(g, w(0x1e), 723);
    g.call(t, 0, &[cam]);
    let t = slot_target(g, w(0x2b), 724);
    let m = g.read::<u32>(w(0x31));
    g.call(t, 0, &[m, cam]);
    let v = g.read::<u32>(w(0x3f));
    let x = g.read::<u32>(w(0x44));
    g.write::<u32>(v, x);
    let t = slot_target(g, w(0x4b), 261);
    if g.call(t, 0, &[0, m, cam]).eax != 0 {
        crate::sa_misc::timeline(g, img, img.rel32_target(f + 0x61));
    }
    let t = slot_target(g, w(0x66), 261);
    g.call(t, 0, &[1]);
    let t = slot_target(g, w(0x75), 261);
    if g.call(t, 0, &[2, 1]).eax == 0 {
        let n = w(0xc2);
        g.write::<u32>(n, g.read::<u32>(n).wrapping_add(1));
        return Ret::eax(1);
    }
    crate::sb_misc::timeline_stop_global(g, img, img.rel32_target(f + 0x8b));
    let t = slot_target(g, w(0x90), 399);
    let h = g.read::<u32>(w(0x98));
    g.call(t, 0, &[h, 0]);
    let t = slot_target(g, w(0xa5), 214);
    let e = g.read::<u32>(w(0xab));
    g.call(t, 0, &[e, h, 0]);
    Ret::eax(1)
}

/// 0x10001790 (204 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 204,
    hash: 0x1532_13aa_b52a_8611,
    calls: &[
        (0x61, Callee::Sig(&crate::sa_misc::TIMELINE)),
        (0x8b, Callee::Sig(&crate::sb_misc::TIMELINE_STOP_GLOBAL)),
    ],
    imports: &[],
};
