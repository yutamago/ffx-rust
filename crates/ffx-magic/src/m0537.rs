//! `magic_0537.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::part_draw::{Camera, camera, start_part};
use crate::runtime::slot_target;
use crate::sb_sse::{add, cvt, div, mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Starts the part (0x10001000, internal: `ecx` = the part): program 0
/// started ([`start_part`]), the position `(0, 0, 0, 1)`, `+0x50..` 0,
/// `+0x5c` 1, a random scale `r / D * M + A` with `r` slot 324's low 15
/// bits (`+0x6c` 1.0). Returns `r`.
pub fn start<G: Game>(g: &mut G, img: &Image, f: u32, obj: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    start_part(g, img, f, obj, [0x1, 0x8, 0xd, 0x17, 0x31], 0, &[]);
    let t = slot_target(g, w(0x36), 324);
    for (off, v) in [
        (0x40u32, 0u32),
        (0x44, 0),
        (0x48, 0),
        (0x4c, 0x3f80_0000),
        (0x50, 0),
        (0x54, 0),
        (0x58, 0),
        (0x5c, 1),
    ] {
        g.write::<u32>(obj + off, v);
    }
    let r = g.call(t, 0, &[]).eax & 0x7fff;
    g.write::<u32>(obj + 0x6c, 0x3f80_0000);
    let k = |g: &G, op: u32| g.read::<f32>(w(op));
    let s = add(mul(div(cvt(r), k(g, 0x94)), k(g, 0x9c)), k(g, 0xa4));
    for off in [0x60, 0x64, 0x68] {
        g.write::<f32>(obj + off, s);
    }
    r
}

/// [`start`] (185 bytes, 1 overlay).
pub const START: Sig = Sig {
    len: 185,
    hash: 0xb334_28f4_23e5_b098,
    calls: &[(0x31, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// Draws the part (0x10001150, internal: `ecx` = the part): slot 258
/// `(part)`, slots 231 and 230 with the camera globals, `*G_P = ITEM` and
/// slot 712 `(part, 0x30)` with the game flag `*G_LOCK` 1 (restored
/// after; the earlier words left on the stack); when 712 says nonzero,
/// slot 713 `(part)` removes it and program 0 starts again
/// ([`start_part`]). Returns 1.
pub fn draw<G: Game>(g: &mut G, img: &Image, f: u32, part: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let t = slot_target(g, w(0x1), 258);
    let mut stack = vec![part];
    g.call(t, 0, &stack);
    for op in [0x14, 0x1f, 0x2b] {
        stack.insert(0, g.read::<u32>(w(op)));
    }
    let t = slot_target(g, w(0x19), 231);
    // call width: 4
    g.call(t, 0, &stack);
    for op in [0x33, 0x3e] {
        stack.insert(0, g.read::<u32>(w(op)));
    }
    let t = slot_target(g, w(0x38), 230);
    // call width: 6
    g.call(t, 0, &stack);
    let gp = g.read::<u32>(w(0x4b));
    g.write::<u32>(gp, w(0x53));
    let lock = g.read::<u32>(w(0x58));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let t = slot_target(g, w(0x66), 712);
    stack.splice(0..0, [part, 0x30]);
    // call width: 8
    let r = g.call(t, 0, &stack).eax;
    let lock = g.read::<u32>(w(0x74));
    g.write::<u32>(lock, saved);
    if r != 0 {
        let gp = g.read::<u32>(w(0x82));
        g.write::<u32>(gp, w(0x89));
        let t = slot_target(g, w(0x8e), 713);
        g.call(t, 0, &[part]);
        start_part(g, img, f, part, [0x9b, 0xa1, 0xa6, 0xae, 0xc8], 0, &[part]);
    }
    1
}

/// [`draw`] (215 bytes, 1 overlay).
pub const DRAW: Sig = Sig {
    len: 215,
    hash: 0x45e5_c524_97d1_cb4f,
    calls: &[(0xc8, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// The effect's frame (0x10001580, an overlay table entry, no
/// arguments): 0 when slot 253 says 0 or the flag at operand `+0x16` is
/// clear. Else the matrices ([`camera`]), `*G_V = *G_W`; when slot 261
/// `(0)` says nonzero the timeline runs
/// ([`timeline`](crate::sa_misc::timeline)); when `(1)` does it stops
/// ([`timeline_stop_global`](crate::sb_misc::timeline_stop_global)) and
/// slot 214 `(*G_E)` runs, else the frame count `+0xaf` goes up. Returns
/// 1.
pub fn frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let t = slot_target(g, w(0x1), 253);
    if g.call(t, 0, &[]).eax == 0 || g.read::<u32>(w(0x16)) == 0 {
        return Ret::eax(0);
    }
    let c = Camera {
        gp: 0x1e,
        cam: 0x24,
        item: 0x2a,
        t723: 0x2f,
        t724: 0x3c,
        m: 0x42,
    };
    let mut stack = camera(g, img, f, &c, &[]);
    let v = g.read::<u32>(w(0x50));
    let x = g.read::<u32>(w(0x55));
    g.write::<u32>(v, x);
    let t = slot_target(g, w(0x5c), 261);
    stack.insert(0, 0);
    // call width: 3
    if g.call(t, 0, &stack).eax != 0 {
        crate::sa_misc::timeline(g, img, img.rel32_target(f + 0x72));
    }
    let t = slot_target(g, w(0x77), 261);
    if g.call(t, 0, &[1]).eax == 0 {
        let n = w(0xaf);
        g.write::<u32>(n, g.read::<u32>(n).wrapping_add(1));
        return Ret::eax(1);
    }
    crate::sb_misc::timeline_stop_global(g, img, img.rel32_target(f + 0x8d));
    let t = slot_target(g, w(0x92), 214);
    let e = g.read::<u32>(w(0x98));
    g.call(t, 0, &[e]);
    Ret::eax(1)
}

/// 0x10001580 (185 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 185,
    hash: 0x444d_7da5_bc5c_257d,
    calls: &[
        (0x72, Callee::Sig(&crate::sa_misc::TIMELINE)),
        (0x8d, Callee::Sig(&crate::sb_misc::TIMELINE_STOP_GLOBAL)),
    ],
    imports: &[],
};

/// The effect's set-up (0x100012a0, an overlay table entry, `(arg)`): the
/// point `P` (three floats, operand `+0x07`..) moved a part `K` (an
/// `f64`, `+0x17`) of the way to `Q` (`+0x0f`..: `P + (Q - P) K` in
/// double, rounded to `f32`); slot 803 `(0x1000000, 16)` the buffer (its
/// address and `+0x80000` to two globals, the argument to a third), the
/// name global set and slot 718 `(NAME)`, slot 222 `(*G_N, 0, 1000,
/// 0x2000)`, a handle `h` from slot 719 `(*G_N, 8, 0)` (kept), slot 711
/// `(h, PROG, 0)` under `**G_LOCK` 1 and slot 710 `(BLOCK, *G_H, *G_BUF,
/// 0x80000)`; the handle's three program tables (slot 941 `(h[0x14 +
/// 4i])`, kept); the 0x110-byte timeline block zeroed (`memset`) with its
/// callback, counts and flags. Each call sees the words the earlier ones
/// left on the stack, up to the clean-up. Returns 1.
pub fn setup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let r = |g: &G, op: u32| g.read::<u32>(w(op));
    let k = g.read::<f64>(w(0x17));
    let t = slot_target(g, w(0x20), 803);
    for (a, b, dst) in [
        (0x7u32, 0xfu32, 0x4au32),
        (0x36, 0x52, 0x78),
        (0x64, 0x80, 0x9e),
    ] {
        let p = g.read::<f32>(w(a));
        let d = sub(g.read::<f32>(w(b)), p);
        // cvtpd2ps: rounded to the nearest f32
        #[allow(clippy::cast_possible_truncation)]
        let x = (f64::from(d) * k + f64::from(p)) as f32;
        g.write::<f32>(w(dst), x);
    }
    let mut stack = vec![0x100_0000, 0x10];
    let buf = g.call(t, 0, &stack).eax;
    g.write::<u32>(w(0xab), buf);
    g.write::<u32>(w(0xb5), buf.wrapping_add(0x8_0000));
    let arg = fr.arg(g, 0);
    g.write::<u32>(w(0xbd), arg);
    let t = slot_target(g, w(0xc2), 718);
    g.write::<u32>(w(0xd3), w(0xd7));
    stack.insert(0, w(0xc7));
    // call width: 18 at most (the earlier calls' words)
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0xde), 222);
    let n = r(g, 0xf6);
    stack.splice(0..0, [n, 0, 0x3e8, 0x2000]);
    // call width: 7
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0xfd), 719);
    let n = r(g, 0x10d);
    stack.splice(0..0, [n, 8, 0]);
    // call width: 10
    let h = g.call(t, 0, &stack).eax;
    let lock = r(g, 0x115);
    let saved = g.read::<u32>(lock);
    g.write::<u32>(w(0x11e), h);
    let t = slot_target(g, w(0x129), 711);
    g.write::<u32>(lock, 1);
    stack.splice(0..0, [h, w(0x123), 0]);
    // call width: 13
    g.call(t, 0, &stack);
    let lock = r(g, 0x13c);
    let b = r(g, 0x147);
    g.write::<u32>(lock, saved);
    let h2 = r(g, 0x14f);
    let t = slot_target(g, w(0x154), 710);
    stack.splice(0..0, [w(0x159), h2, b, 0x8_0000]);
    // call width: 17
    g.call(t, 0, &stack);
    let mut stack = Vec::new();
    for (k, (hop, tab, dst)) in [
        (0x166u32, 0x171u32, 0x17eu32),
        (0x183, 0x18b, 0x198),
        (0x19d, 0x1a5, 0x1be),
    ]
    .into_iter()
    .enumerate()
    {
        let h = r(g, hop);
        let x = g.read::<u32>(h + 0x14 + 4 * k as u32);
        let t = slot_target(g, w(tab), 941);
        stack.insert(0, x);
        // call width: 18 at most (the earlier calls' words)
        let v = g.call(t, 0, &stack).eax;
        g.write::<u32>(w(dst), v);
    }
    g.write_bytes(w(0x1b9), &[0u8; 0x110]);
    g.write::<u32>(w(0x1cc), w(0x1d0));
    for (op, v) in [(0x1d6u32, 2u32), (0x1e0, 2), (0x1ea, 1), (0x1f4, 1)] {
        g.write::<u32>(w(op), v);
    }
    Ret::eax(1)
}

/// [`setup`] (516 bytes, 1 overlay).
pub const SETUP: Sig = Sig {
    len: 516,
    hash: 0x5c74_8806_70e3_731f,
    calls: &[(0x1c3, Callee::Import("memset"))],
    imports: &[],
};

/// The part of a table entry drawn (0x10001230, cdecl `(entry)`): the
/// part `PARTS + 0x80 * ((entry - TABLE) >> 2)` (operands `+0x14`,
/// `+0x8`; an arithmetic shift) through [`draw`]. Returns 1.
pub fn draw_entry<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let i = fr.arg(g, 0).wrapping_sub(w(0x8)).cast_signed() >> 2;
    let part = (i.cast_unsigned() << 7).wrapping_add(w(0x14));
    draw(g, img, img.rel32_target(fr.at + 0x19), part);
    Ret::eax(1)
}

/// [`draw_entry`] (36 bytes, 1 overlay).
pub const DRAW_ENTRY: Sig = Sig {
    len: 36,
    hash: 0x6da3_bfc9_69e5_ae9f,
    calls: &[(0x19, Callee::Sig(&DRAW))],
    imports: &[],
};
