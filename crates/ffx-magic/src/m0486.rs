//! `magic_0486.dll`'s own functions (besides the runtime and the shared
//! code): three part draws, the start-up and the frame.

use crate::part_draw::{Axis, Camera, Restore, Transform, camera, draw_restore, transform};
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The matrices' operands of the draws without a slot 237 call.
const CAMERA: Camera = Camera {
    gp: 0x1,
    cam: 0x8,
    item: 0xe,
    t723: 0x13,
    t724: 0x20,
    m: 0x26,
};

/// The transform's operands of 0x10001250 and 0x100013f0.
const TRANSFORM: Transform = Transform {
    t258: 0x3b,
    part: 0x58,
    a231: [0x8e, 0x99, 0xa5],
    t231: 0x93,
    a230: [0xad, 0xb8],
    t230: 0xb2,
};

/// The draw's operands of 0x10001250 and 0x100013f0.
const RESTORE: Restore = Restore {
    gp: 0xc5,
    item: 0xcd,
    lock: 0xd2,
    part: 0xd7,
    t712: 0xe4,
    lock2: 0xf1,
};

/// 0x10001070 (an overlay table entry, no arguments): slot 237 `(*G_A,
/// 0x66, 0, V)` fills the point `V` (operand `+0x0d`), its `y` drops by
/// `K` (`+0x29`); the part is drawn at the point
/// ([`camera`], [`transform`], [`draw_restore`]). Returns 1.
pub fn e1070<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let t = slot_target(g, w(0x1), 237);
    let a = g.read::<u32>(w(0x17));
    let below = [a, 0x66, 0, w(0xd)];
    g.call(t, 0, &below);
    Axis {
        src: 0x21,
        steps: &[(false, 0x29)],
        dst: 0x3c,
    }
    .run(g, img, f);
    let c = Camera {
        gp: 0x2e,
        cam: 0x34,
        item: 0x42,
        t723: 0x47,
        t724: 0x54,
        m: 0x5a,
    };
    let mut stack = camera(g, img, f, &c, &below);
    let t = Transform {
        t258: 0x6f,
        part: 0xa2,
        a231: [0xaa, 0xb5, 0xc1],
        t231: 0xaf,
        a230: [0xc9, 0xd4],
        t230: 0xce,
    };
    let axes = [
        Axis::copy(0x6a, 0x77),
        Axis::copy(0x7f, 0x87),
        Axis::copy(0x8f, 0x97),
    ];
    transform(g, img, f, &t, &axes, &mut stack);
    let r = Restore {
        gp: 0xe1,
        item: 0xe9,
        lock: 0xee,
        part: 0xf3,
        t712: 0x100,
        lock2: 0x10d,
    };
    draw_restore(g, img, f, &r, &stack);
    Ret::eax(1)
}

/// 0x10001070 (285 bytes, 1 overlay).
pub const E1070: Sig = Sig {
    len: 285,
    hash: 0x888c_0286_9348_8d1e,
    calls: &[],
    imports: &[],
};

/// The part drawn at a point `(x, y - K1 ± K2, z + K3)` (`plus`: `+K2`).
fn draw_at<G: Game>(g: &mut G, img: &Image, f: u32, plus: bool) -> Ret {
    let mut stack = camera(g, img, f, &CAMERA, &[]);
    let axes = [
        Axis::copy(0x36, 0x43),
        Axis {
            src: 0x4b,
            steps: if plus {
                &[(false, 0x53), (true, 0x60)]
            } else {
                &[(false, 0x53), (false, 0x60)]
            },
            dst: 0x68,
        },
        Axis {
            src: 0x70,
            steps: &[(true, 0x78)],
            dst: 0x80,
        },
    ];
    transform(g, img, f, &TRANSFORM, &axes, &mut stack);
    draw_restore(g, img, f, &RESTORE, &stack);
    Ret::eax(1)
}

/// 0x10001250 (an overlay table entry, no arguments): the part drawn at
/// `(x, y - K1 + K2, z + K3)` ([`draw_at`]). Returns 1.
pub fn e1250<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    draw_at(g, img, fr.at, true)
}

/// 0x10001250 (257 bytes, 1 overlay).
pub const E1250: Sig = Sig {
    len: 257,
    hash: 0xa2c7_ecc0_627d_2231,
    calls: &[],
    imports: &[],
};

/// 0x100013f0 (an overlay table entry, no arguments): the part drawn at
/// `(x, y - K1 - K2, z + K3)` ([`draw_at`]). Returns 1.
pub fn e13f0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    draw_at(g, img, fr.at, false)
}

/// 0x100013f0 (257 bytes, 1 overlay).
pub const E13F0: Sig = Sig {
    len: 257,
    hash: 0x1e71_edcd_06d6_cc1d,
    calls: &[],
    imports: &[],
};

/// The effect's start-up (0x10001500, an overlay table entry, `(arg)`):
/// slot 803 `(0x1000000, 0x10)` the buffer, cut into three parts
/// (globals `+0x19`, `+0x23`, `+0x2d`); `arg` to `+0x35`; the name
/// (`+0x3f`) to `+0x4b` and slot 718, slot 222 `(name, 0, 1000,
/// 0x2000)`, slot 719 `(name, 8, 0)` the data `d` (`+0x96`); slot 711
/// `(d, PARTS, 0)` with the game flag `*G_LOCK` 1; slot 710 `(ITEM, d,
/// buf, 0x80000)`; three slot 941 `(d[5 + k])` the program tables; two
/// 0x110-byte timelines cleared and given their defaults; slot 413 `(0)`
/// a handle (`+0x1bd`). Every call passes the words on the original's
/// stack. Returns 1.
pub fn start_up<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let t = slot_target(g, w(0x4), 803);
    let mut stack = vec![0x0100_0000, 0x10];
    let buf = g.call(t, 0, &stack).eax;
    g.write::<u32>(w(0x19), buf);
    g.write::<u32>(w(0x23), buf.wrapping_add(0x8_0000));
    g.write::<u32>(w(0x2d), buf.wrapping_add(0x10_0000));
    let arg = fr.arg(g, 0);
    g.write::<u32>(w(0x35), arg);
    let t = slot_target(g, w(0x3a), 718);
    stack.insert(0, w(0x3f));
    g.write::<u32>(w(0x4b), w(0x4f));
    // call width: 17
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x56), 222);
    let name = g.read::<u32>(w(0x6e));
    stack.splice(0..0, [name, 0, 0x3e8, 0x2000]);
    // call width: 17
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x75), 719);
    let name = g.read::<u32>(w(0x85));
    stack.splice(0..0, [name, 8, 0]);
    // call width: 17
    let d = g.call(t, 0, &stack).eax;
    let lock = g.read::<u32>(w(0x8d));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(w(0x96), d);
    g.write::<u32>(lock, 1);
    let t = slot_target(g, w(0xa1), 711);
    stack.splice(0..0, [d, w(0x9b), 0]);
    // call width: 17
    g.call(t, 0, &stack);
    let lock = g.read::<u32>(w(0xb4));
    let b = g.read::<u32>(w(0xbf));
    g.write::<u32>(lock, saved);
    let d = g.read::<u32>(w(0xc7));
    let t = slot_target(g, w(0xcc), 710);
    stack.splice(0..0, [w(0xd1), d, b, 0x8_0000]);
    // call width: 17
    g.call(t, 0, &stack);
    // add esp, 0x44
    let mut stack = Vec::new();
    for (k, (dop, tab, dst)) in (0u32..).zip([
        (0xdeu32, 0xe9u32, 0xf6u32),
        (0xfb, 0x103, 0x110),
        (0x115, 0x11d, 0x136),
    ]) {
        let d = g.read::<u32>(w(dop));
        stack.insert(0, g.read::<u32>(d.wrapping_add(0x14 + 4 * k)));
        let t = slot_target(g, w(tab), 941);
        // call width: 17
        let r = g.call(t, 0, &stack).eax;
        g.write::<u32>(w(dst), r);
    }
    let zero = [0u8; 0x110];
    g.write_bytes(w(0x131), &zero);
    stack.splice(0..0, [w(0x131), 0, 0x110]);
    g.write::<u32>(w(0x14d), w(0x151));
    for op in [0x157, 0x161, 0x16b] {
        g.write::<u32>(w(op), 1);
    }
    g.write_bytes(w(0x147), &zero);
    stack.splice(0..0, [w(0x147), 0, 0x110]);
    let t = slot_target(g, w(0x179), 413);
    g.write::<u32>(w(0x17f), w(0x183));
    for (op, v) in [(0x189u32, 2u32), (0x193, 2), (0x19d, 1), (0x1a7, 1)] {
        g.write::<u32>(w(op), v);
    }
    stack.insert(0, 0);
    // call width: 17
    let h = g.call(t, 0, &stack).eax;
    g.write::<u32>(w(0x1bd), h);
    Ret::eax(1)
}

/// 0x10001500 (457 bytes, 1 overlay).
pub const START_UP: Sig = Sig {
    len: 457,
    hash: 0x15a7_cca9_3442_1c66,
    calls: &[
        (0x13b, Callee::Import("memset")),
        (0x174, Callee::Import("memset")),
    ],
    imports: &[],
};

/// The effect's frame (0x10001790, an overlay table entry, no
/// arguments): 0 when slot 253 says 0 or the flag at operand `+0x16` is
/// clear. Else `*G_V = *G_W`, `*G_X = *G_Y`; when slot 261 `(1)` says
/// nonzero the first timeline (`+0x4b`) runs
/// ([`timeline_run`](crate::sb_misc::timeline_run)), when `(2)` does the
/// second (`+0x6b`); when `(3)` does, both are stopped
/// ([`timeline_stop`](crate::sb_misc::timeline_stop)) and slot 214
/// `(*G_E)` runs, else the frame count `+0xbc` goes up. Returns 1.
pub fn frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let t = slot_target(g, w(0x1), 253);
    if g.call(t, 0, &[]).eax == 0 || g.read::<u32>(w(0x16)) == 0 {
        return Ret::eax(0);
    }
    let v = g.read::<u32>(w(0x1f));
    let x = g.read::<u32>(w(0x24));
    g.write::<u32>(v, x);
    let y = g.read::<u32>(w(0x2b));
    g.write::<u32>(w(0x30), y);
    let t = slot_target(g, w(0x35), 261);
    if g.call(t, 0, &[1]).eax != 0 {
        crate::sb_misc::timeline_run(g, img, img.rel32_target(f + 0x50), w(0x4b));
    }
    let t = slot_target(g, w(0x55), 261);
    if g.call(t, 0, &[2]).eax != 0 {
        crate::sb_misc::timeline_run(g, img, img.rel32_target(f + 0x70), w(0x6b));
    }
    let t = slot_target(g, w(0x75), 261);
    if g.call(t, 0, &[3]).eax == 0 {
        let n = w(0xbc);
        g.write::<u32>(n, g.read::<u32>(n).wrapping_add(1));
        return Ret::eax(1);
    }
    crate::sb_misc::timeline_stop(g, w(0x8b));
    crate::sb_misc::timeline_stop(g, w(0x95));
    let t = slot_target(g, w(0x9f), 214);
    let e = g.read::<u32>(w(0xa5));
    g.call(t, 0, &[e]);
    Ret::eax(1)
}

/// 0x10001790 (198 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 198,
    hash: 0xe789_5995_c563_9301,
    calls: &[
        (0x50, Callee::Sig(&crate::sb_misc::TIMELINE_RUN)),
        (0x70, Callee::Sig(&crate::sb_misc::TIMELINE_RUN)),
        (0x90, Callee::Sig(&crate::sb_misc::TIMELINE_STOP)),
        (0x9a, Callee::Sig(&crate::sb_misc::TIMELINE_STOP)),
    ],
    imports: &[],
};
