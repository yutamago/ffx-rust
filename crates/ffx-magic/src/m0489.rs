//! `magic_0489.dll`'s own functions (besides the runtime and the shared
//! code): the object callbacks of its table. The objects lie in arrays
//! of 0x80-byte records; a callback tells them apart by the record's
//! offset from an array's start.

use crate::part_draw::{Camera, camera, start_part};
use crate::runtime::slot_target;
use crate::sb_sse::sub;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `1.0f32`'s bits.
const ONE: u32 = 0x3f80_0000;

/// A timeline block of the start-up: the block, its callback word
/// `(dst, value)` operands and its defaults `(dst, value)`.
type Block = (u32, (u32, u32), &'static [(u32, u32)]);

/// Whether `obj` is the first 0x80-byte record of the array at the
/// operand `op` (`sub obj, BASE; test 0xffffff80`).
fn first(img: &Image, f: u32, obj: u32, op: u32) -> bool {
    obj.wrapping_sub(img.word(f + op)) & 0xffff_ff80 == 0
}

/// Writes `(offset, value)` words into the object.
fn fill<G: Game>(g: &mut G, obj: u32, words: &[(u32, u32)]) {
    for &(off, v) in words {
        g.write::<u32>(obj + off, v);
    }
}

/// An object's frame (0x10001980, `(obj)`): unless the flag `**G_F`
/// (operand `+0x04`) is set, slot 237 `(*G_A, 7, 0, obj + 0x40)` places
/// it (`G_A` at `+0x35` for the first record of the array at `+0x15`,
/// else `+0x3d`); then slot 258 `(obj)`, slots 231 and 230 with the
/// camera globals, `*G_P = ITEM` and slot 712 `(obj, 0x30)` with the
/// game flag `*G_LOCK` 1 (restored after; the earlier words left on the
/// stack). Returns 1 when slot 712 says 0, else 0.
pub fn e1980<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let flag = g.read::<u32>(w(0x4));
    if g.read::<u32>(flag) == 0 {
        let t = slot_target(g, w(0x23), 237);
        let op = if first(img, f, obj, 0x15) { 0x35 } else { 0x3d };
        let a = g.read::<u32>(w(op));
        g.call(t, 0, &[a, 7, 0, obj + 0x40]);
    }
    let t = slot_target(g, w(0x47), 258);
    let mut stack = vec![obj];
    g.call(t, 0, &stack);
    for op in [0x56, 0x61, 0x6d] {
        stack.insert(0, g.read::<u32>(w(op)));
    }
    let t = slot_target(g, w(0x5b), 231);
    // call width: 8
    g.call(t, 0, &stack);
    for op in [0x75, 0x80] {
        stack.insert(0, g.read::<u32>(w(op)));
    }
    let t = slot_target(g, w(0x7a), 230);
    // call width: 8
    g.call(t, 0, &stack);
    let gp = g.read::<u32>(w(0x8d));
    g.write::<u32>(gp, w(0x95));
    let lock = g.read::<u32>(w(0x9a));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let t = slot_target(g, w(0xa8), 712);
    stack.splice(0..0, [obj, 0x30]);
    // call width: 8
    let r = g.call(t, 0, &stack).eax;
    let lock = g.read::<u32>(w(0xb6));
    g.write::<u32>(lock, saved);
    Ret::eax(u32::from(r == 0))
}

/// 0x10001980 (204 bytes, 1 overlay).
pub const E1980: Sig = Sig {
    len: 204,
    hash: 0xfafd_ac4e_7ed4_2c1f,
    calls: &[],
    imports: &[],
};

/// An object's start (0x10001a80, `(obj)`): slot 237 `(*G_A, 0x6e, 0,
/// obj + 0x40)` places it, then `x` and `z` are set (0.917, -28.73);
/// the first record of the array at `+0x26` starts program 0 with the
/// scales 0.1875, the others program 1 with the scales 0.625 and `y`
/// lowered twice by `K` ([`start_part`]); then `+0x6c` 1.0, `+0x50..`
/// 0, `+0x5c` 1. Returns 1.
pub fn e1a80<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let t = slot_target(g, w(0x4), 237);
    let a = g.read::<u32>(w(0x1d));
    g.call(t, 0, &[a, 0x6e, 0, obj + 0x40]);
    g.write::<u32>(obj + 0x40, 0x3f6a_dfc6);
    g.write::<u32>(obj + 0x48, 0xc1e5_dd21);
    if first(img, f, obj, 0x26) {
        start_part(g, img, f, obj, [0x33, 0x46, 0x4b, 0x53, 0x6f], 0, &[]);
        fill(
            g,
            obj,
            &[
                (0x60, 0x3e40_0000),
                (0x64, 0x3e40_0000),
                (0x68, 0x3e40_0000),
            ],
        );
    } else {
        start_part(g, img, f, obj, [0x33, 0x46, 0x4b, 0x53, 0x9b], 4, &[]);
        let y = sub(g.read::<f32>(obj + 0x44), g.read::<f32>(w(0xa8)));
        fill(
            g,
            obj,
            &[
                (0x60, 0x3f20_0000),
                (0x64, 0x3f20_0000),
                (0x68, 0x3f20_0000),
            ],
        );
        let y = sub(y, g.read::<f32>(w(0xc5)));
        g.write::<f32>(obj + 0x44, y);
    }
    fill(
        g,
        obj,
        &[(0x6c, ONE), (0x50, 0), (0x54, 0), (0x58, 0), (0x5c, 1)],
    );
    Ret::eax(1)
}

/// 0x10001a80 (253 bytes, 1 overlay).
pub const E1A80: Sig = Sig {
    len: 253,
    hash: 0x28de_b5b3_7943_6b65,
    calls: &[
        (0x6f, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x9b, Callee::Sig(&crate::opu_parts::START_PART)),
    ],
    imports: &[],
};

/// An object's start (0x10001c20, `(obj)`): program 2 started
/// ([`start_part`]), the position `(0, 0, 0, 1)`, `+0x50..` 0, `+0x5c`
/// 1, the scales 0.0625 (`+0x6c` 1.0); slot 237 `(*G_A, 5, 0, obj +
/// 0x40)` places it (`G_A` at `+0xb4` for the first record of the array
/// at `+0x8f`, else `+0xc6`). Returns 1.
pub fn e1c20<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    start_part(g, img, f, obj, [0x4, 0xb, 0x10, 0x18, 0x36], 8, &[]);
    fill(
        g,
        obj,
        &[
            (0x44, 0),
            (0x48, 0),
            (0x4c, ONE),
            (0x50, 0),
            (0x54, 0),
            (0x58, 0),
            (0x5c, 1),
            (0x60, 0x3d80_0000),
            (0x64, 0x3d80_0000),
            (0x68, 0x3d80_0000),
            (0x6c, ONE),
        ],
    );
    g.write::<u32>(obj + 0x40, 0);
    let t = slot_target(g, w(0xa2), 237);
    let op = if first(img, f, obj, 0x8f) { 0xb4 } else { 0xc6 };
    let a = g.read::<u32>(w(op));
    g.call(t, 0, &[a, 5, 0, obj + 0x40]);
    Ret::eax(1)
}

/// 0x10001c20 (214 bytes, 1 overlay).
pub const E1C20: Sig = Sig {
    len: 214,
    hash: 0x6a7f_3c6a_b219_1093,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// An object's start (0x10001d00, `(obj)`): program 4 started
/// ([`start_part`]), the position `(0, 0, 0, 1)`, `+0x50..` 0, `+0x5c`
/// 1, the scales 0.1725 (`+0x6c` 1.0); slot 237 `(*G_A, 0x25, 0, obj +
/// 0x40)` (the start's words left on the stack); then by the record's
/// index in the array at `+0xb9` (1, 2, 3) a fixed position (index 1
/// and 2 share `x` and `y`; `w` 1.0 again). The stack argument ends as
/// `obj + 0x48` (the original keeps the pointer there). Returns 1.
pub fn e1d00<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let below = start_part(g, img, f, obj, [0x5, 0xc, 0x11, 0x1b, 0x39], 0x10, &[]);
    g.write::<u32>(fr.esp + 4, obj + 0x48);
    g.write::<u32>(obj + 0x48, 0);
    g.write::<u32>(obj + 0x4c, ONE);
    let t = slot_target(g, w(0x5a), 237);
    let a = g.read::<u32>(w(0x67));
    fill(
        g,
        obj,
        &[
            (0x40, 0),
            (0x44, 0),
            (0x50, 0),
            (0x54, 0),
            (0x58, 0),
            (0x5c, 1),
            (0x60, 0x3e30_a3d7),
            (0x64, 0x3e30_a3d7),
            (0x68, 0x3e30_a3d7),
            (0x6c, ONE),
        ],
    );
    let mut args = vec![a, 0x25, 0, obj + 0x40];
    args.extend(below);
    // call width: 8
    g.call(t, 0, &args);
    // sar 7: the signed record index
    let k = obj.wrapping_sub(w(0xb9)).cast_signed() >> 7;
    let pos: &[(u32, u32)] = match k {
        1 => &[
            (0x48, 0xc2ee_3b6f),
            (0x44, 0xc2d4_e8c0),
            (0x40, 0xc19b_80b3),
        ],
        2 => &[
            (0x48, 0xc2fc_3b6e),
            (0x44, 0xc2d4_e8c0),
            (0x40, 0xc19b_80b3),
        ],
        3 => &[
            (0x40, 0xc195_eccd),
            (0x44, 0xc2d7_7fa7),
            (0x48, 0xc2f2_bfcc),
        ],
        _ => return Ret::eax(1),
    };
    fill(g, obj, pos);
    g.write::<u32>(obj + 0x4c, ONE);
    Ret::eax(1)
}

/// 0x10001d00 (280 bytes, 1 overlay).
pub const E1D00: Sig = Sig {
    len: 280,
    hash: 0x593b_c9f6_2749_c730,
    calls: &[(0x39, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// The effect's start-up (0x10001e20, an overlay table entry, `(arg)`):
/// `arg` to `+0x07`; slot 803 `(0x1000000, 0x10)` the buffer, cut into
/// parts (globals `+0x21`, `+0x2b`, `+0x35`, `+0x3f`/`+0x44`, and
/// `+0x4e` 0x39d0 into the last); the name (`+0x58`) to `+0x64` and slot
/// 718, slot 222 `(name, 0, 1000, 0x2000)`, slot 719 `(name, 8, 0)` the
/// data `d` (`+0xaf`); slot 711 `(d, PARTS, 0)` with the game flag
/// `*G_LOCK` 1; slot 710 `(ITEM, d, buf, 0x80000)`; three slot 941
/// `(d[5 + k])` the program tables; a 0x30-byte block cleared (`+0x2`
/// 0x80) and five 0x110-byte timelines cleared and given their
/// defaults; slot 260 `(0x00, 0x15, 0x16, 0x14, 0x18, 0x19, 0x17)` seven
/// handles. Every call passes the words on the original's stack.
/// Returns 1.
#[allow(clippy::too_many_lines)]
pub fn start_up<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let arg = fr.arg(g, 0);
    g.write::<u32>(w(0x7), arg);
    let t = slot_target(g, w(0xc), 803);
    let mut stack = vec![0x0100_0000, 0x10];
    let buf = g.call(t, 0, &stack).eax;
    g.write::<u32>(w(0x21), buf);
    g.write::<u32>(w(0x2b), buf.wrapping_add(0x8_0000));
    g.write::<u32>(w(0x35), buf.wrapping_add(0x10_0000));
    g.write::<u32>(w(0x3f), buf.wrapping_add(0x18_0000));
    g.write::<u32>(w(0x44), buf.wrapping_add(0x18_0000));
    g.write::<u32>(w(0x4e), buf.wrapping_add(0x18_39d0));
    let t = slot_target(g, w(0x53), 718);
    stack.insert(0, w(0x58));
    g.write::<u32>(w(0x64), w(0x68));
    // call width: 17
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x6f), 222);
    let name = g.read::<u32>(w(0x87));
    stack.splice(0..0, [name, 0, 0x3e8, 0x2000]);
    // call width: 17
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x8e), 719);
    let name = g.read::<u32>(w(0x9e));
    stack.splice(0..0, [name, 8, 0]);
    // call width: 17
    let d = g.call(t, 0, &stack).eax;
    let lock = g.read::<u32>(w(0xa6));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(w(0xaf), d);
    g.write::<u32>(lock, 1);
    let t = slot_target(g, w(0xba), 711);
    stack.splice(0..0, [d, w(0xb4), 0]);
    // call width: 17
    g.call(t, 0, &stack);
    let lock = g.read::<u32>(w(0xcd));
    let b = g.read::<u32>(w(0xd8));
    g.write::<u32>(lock, saved);
    let d = g.read::<u32>(w(0xe0));
    let t = slot_target(g, w(0xe5), 710);
    stack.splice(0..0, [w(0xea), d, b, 0x8_0000]);
    // call width: 17
    g.call(t, 0, &stack);
    // add esp, 0x44
    let mut stack = Vec::new();
    for (k, (dop, tab, dst)) in (0u32..).zip([
        (0xf7u32, 0x102u32, 0x10fu32),
        (0x114, 0x11c, 0x129),
        (0x12e, 0x136, 0x14c),
    ]) {
        let d = g.read::<u32>(w(dop));
        stack.insert(0, g.read::<u32>(d.wrapping_add(0x14 + 4 * k)));
        let t = slot_target(g, w(tab), 941);
        // call width: 17
        let r = g.call(t, 0, &stack).eax;
        g.write::<u32>(w(dst), r);
    }
    g.write_bytes(w(0x147), &[0u8; 0x30]);
    g.write::<u16>(w(0x15c), 0x80);
    let zero = [0u8; 0x110];
    g.write_bytes(w(0x168), &zero);
    // each block's callback word and defaults, set before the next clear
    let blocks: [Block; 3] = [
        (
            0x179,
            (0x17f, 0x183),
            &[
                (0x189, 4),
                (0x193, 4),
                (0x19d, 1),
                (0x1a7, 1),
                (0x1b1, 1),
                (0x1bb, 1),
            ],
        ),
        (
            0x1d0,
            (0x1d6, 0x1da),
            &[(0x1e0, 2), (0x1ea, 2), (0x1f4, 1), (0x1fe, 1)],
        ),
        (
            0x213,
            (0x219, 0x21d),
            &[(0x223, 2), (0x22d, 2), (0x237, 1), (0x241, 1)],
        ),
    ];
    for (dst, (cb, v), defaults) in blocks {
        g.write::<u32>(w(cb), w(v));
        for &(op, x) in defaults {
            g.write::<u32>(w(op), x);
        }
        g.write_bytes(w(dst), &zero);
    }
    // add esp, 0x48
    g.write::<u32>(w(0x253), w(0x257));
    for (op, x) in [(0x269u32, 2u32), (0x273, 2), (0x27d, 1), (0x287, 1)] {
        g.write::<u32>(w(op), x);
    }
    g.write_bytes(w(0x263), &zero);
    let mut stack = vec![w(0x263), 0, 0x110];
    let t = slot_target(g, w(0x295), 260);
    g.write::<u32>(w(0x29b), w(0x29f));
    for (op, x) in [(0x2a5u32, 2u32), (0x2af, 2), (0x2b9, 1), (0x2c3, 1)] {
        g.write::<u32>(w(op), x);
    }
    stack.insert(0, 0);
    // call width: 17
    let h = g.call(t, 0, &stack).eax;
    g.write::<u32>(w(0x2d6), h);
    for (tab, n, dst) in [
        (0x2dbu32, 0x15u32, 0x2eau32),
        (0x2ef, 0x16, 0x2fe),
        (0x303, 0x14, 0x312),
        (0x317, 0x18, 0x326),
        (0x32b, 0x19, 0x33a),
        (0x33f, 0x17, 0x351),
    ] {
        let t = slot_target(g, w(tab), 260);
        stack.insert(0, n);
        // call width: 17
        let h = g.call(t, 0, &stack).eax;
        g.write::<u32>(w(dst), h);
    }
    Ret::eax(1)
}

/// 0x10001e20 (861 bytes, 1 overlay).
pub const START_UP: Sig = Sig {
    len: 861,
    hash: 0xfff0_9523_541e_d607,
    calls: &[
        (0x151, Callee::Import("memset")),
        (0x16d, Callee::Import("memset")),
        (0x1c4, Callee::Import("memset")),
        (0x207, Callee::Import("memset")),
        (0x24a, Callee::Import("memset")),
        (0x290, Callee::Import("memset")),
    ],
    imports: &[],
};

/// The effect's frame (0x10002240, an overlay table entry, no
/// arguments): 0 while the flag at operand `+0x02` is clear. Else the
/// matrices ([`camera`]), `*G_V = *G_W`, `*G_X = *G_Y`; slot 261 `(k)`
/// for `k` = 0..4 each runs a timeline
/// ([`timeline_run`](crate::sb_misc::timeline_run)) when it says
/// nonzero; for `k` = 1 first, when the trail timeline's clock
/// (`+0x8f`) is 0, slot 237 `(*G_H, 0, 0, p)` fills its ten points, and
/// while the clock is below 170 the trail shifts one place and slot 237
/// fills the newest. When `(5)` says nonzero the five timelines stop
/// ([`timeline_stop`](crate::sb_misc::timeline_stop)) and slot 214
/// `(*G_E)` runs, else the frame count `+0x376` goes up. Returns 1.
#[allow(clippy::too_many_lines)]
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
    let y = g.read::<u32>(w(0x4b));
    g.write::<u32>(w(0x50), y);
    let run = |g: &mut G, call: u32, this: u32| {
        crate::sb_misc::timeline_run(g, img, img.rel32_target(f + call), w(this));
    };
    let t = slot_target(g, w(0x55), 261);
    stack.insert(0, 0);
    // call width: 3
    if g.call(t, 0, &stack).eax != 0 {
        run(g, 0x70, 0x6b);
    }
    let t = slot_target(g, w(0x75), 261);
    if g.call(t, 0, &[1]).eax != 0 {
        let mut clock = g.read::<u32>(w(0x8f));
        if clock == 0 {
            let mut p = w(0x99);
            loop {
                let t = slot_target(g, w(0xa1), 237);
                let h = g.read::<u32>(w(0xb2));
                g.call(t, 0, &[h, 0, 0, p]);
                p = p.wrapping_add(0x10);
                if p.cast_signed() >= w(0xc0).cast_signed() {
                    break;
                }
            }
            clock = g.read::<u32>(w(0xc7));
        }
        if clock.cast_signed() < 0xaa {
            for (src, dst) in [
                (0xdb, 0xe3),
                (0xeb, 0xf3),
                (0xfb, 0x103),
                (0x10b, 0x113),
                (0x11b, 0x123),
                (0x12b, 0x133),
                (0x13b, 0x143),
                (0x14b, 0x153),
                (0x15b, 0x163),
                (0x16b, 0x173),
                (0x17b, 0x183),
                (0x18b, 0x193),
                (0x19b, 0x1a3),
                (0x1ab, 0x1b3),
                (0x1bb, 0x1c3),
                (0x1cb, 0x1d3),
                (0x1db, 0x1e3),
                (0x1eb, 0x1f3),
                (0x1fb, 0x208),
                (0x210, 0x218),
                (0x220, 0x228),
                (0x230, 0x238),
                (0x240, 0x248),
                (0x250, 0x258),
                (0x260, 0x268),
                (0x270, 0x27d),
                (0x285, 0x297),
            ] {
                let v = g.read::<u32>(w(src));
                g.write::<u32>(w(dst), v);
            }
            let t = slot_target(g, w(0x200), 237);
            let h = g.read::<u32>(w(0x28f));
            g.call(t, 0, &[h, 0, 0, w(0x275)]);
            run(g, 0x2ac, 0x2a7);
        }
    }
    for (tab, k, call, this) in [
        (0x2b1u32, 2u32, 0x2ccu32, 0x2c7u32),
        (0x2d1, 3, 0x2ec, 0x2e7),
        (0x2f1, 4, 0x30c, 0x307),
    ] {
        let t = slot_target(g, w(tab), 261);
        if g.call(t, 0, &[k]).eax != 0 {
            run(g, call, this);
        }
    }
    let t = slot_target(g, w(0x311), 261);
    if g.call(t, 0, &[5]).eax == 0 {
        let n = w(0x376);
        g.write::<u32>(n, g.read::<u32>(n).wrapping_add(1));
        return Ret::eax(1);
    }
    for op in [0x327, 0x331, 0x33b, 0x345, 0x34f] {
        crate::sb_misc::timeline_stop(g, w(op));
    }
    let t = slot_target(g, w(0x359), 214);
    let e = g.read::<u32>(w(0x35f));
    g.call(t, 0, &[e]);
    Ret::eax(1)
}

/// 0x10002240 (896 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 896,
    hash: 0x78c1_4d82_7b67_a488,
    calls: &[
        (0x70, Callee::Sig(&crate::sb_misc::TIMELINE_RUN)),
        (0x2ac, Callee::Sig(&crate::sb_misc::TIMELINE_RUN)),
        (0x2cc, Callee::Sig(&crate::sb_misc::TIMELINE_RUN)),
        (0x2ec, Callee::Sig(&crate::sb_misc::TIMELINE_RUN)),
        (0x30c, Callee::Sig(&crate::sb_misc::TIMELINE_RUN)),
        (0x32c, Callee::Sig(&crate::sb_misc::TIMELINE_STOP)),
        (0x336, Callee::Sig(&crate::sb_misc::TIMELINE_STOP)),
        (0x340, Callee::Sig(&crate::sb_misc::TIMELINE_STOP)),
        (0x34a, Callee::Sig(&crate::sb_misc::TIMELINE_STOP)),
        (0x354, Callee::Sig(&crate::sb_misc::TIMELINE_STOP)),
    ],
    imports: &[],
};
