//! `magic_0539.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::opu_parts::start_part;
use crate::part_draw::{Camera, camera};
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// An object's frame (0x100010a0, `(obj)`): slot 258 `(obj)`, slots 231
/// and 230 with the camera globals, `*G_P = ITEM` and slot 712 `(obj,
/// 0x30)` with the game flag `*G_LOCK` 1 (restored after; the earlier
/// words left on the stack). Returns 0 when slot 712 says nonzero, else
/// whether the second timeline's clock (operand `+0x8e`) is not 31.
pub fn frame_obj<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let obj = fr.arg(g, 0);
    let t = slot_target(g, w(0x4), 258);
    let mut stack = vec![obj];
    g.call(t, 0, &stack);
    for op in [0x16, 0x21, 0x2d] {
        stack.insert(0, g.read::<u32>(w(op)));
    }
    let t = slot_target(g, w(0x1b), 231);
    // call width: 4
    g.call(t, 0, &stack);
    for op in [0x35, 0x40] {
        stack.insert(0, g.read::<u32>(w(op)));
    }
    let t = slot_target(g, w(0x3a), 230);
    // call width: 6
    g.call(t, 0, &stack);
    let gp = g.read::<u32>(w(0x4d));
    g.write::<u32>(gp, w(0x58));
    let lock = g.read::<u32>(w(0x5d));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let t = slot_target(g, w(0x6a), 712);
    stack.splice(0..0, [obj, 0x30]);
    // call width: 8
    let r = g.call(t, 0, &stack).eax;
    let lock = g.read::<u32>(w(0x78));
    g.write::<u32>(lock, saved);
    if r != 0 {
        return Ret::eax(0);
    }
    Ret::eax(u32::from(g.read::<u32>(w(0x8e)) != 0x1f))
}

/// 0x100010a0 (152 bytes, 1 overlay).
pub const FRAME_OBJ: Sig = Sig {
    len: 152,
    hash: 0x9a67_8df2_9a14_4b19,
    calls: &[],
    imports: &[],
};

/// The effect's frame (0x10001640, an overlay table entry, no
/// arguments): 0 while the flag at operand `+0x02` is clear. Else the
/// matrices ([`camera`]), `*G_V = *G_W`; slot 261 `(0)` and `(1)` each
/// run a timeline ([`timeline_run`](crate::sb_misc::timeline_run)) when
/// they say nonzero; when `(2)` does both stop
/// ([`timeline_stop`](crate::sb_misc::timeline_stop)) and slot 214
/// `(*G_E)` runs, else the frame count `+0xd2` goes up. Returns 1.
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
    stack.insert(0, 0);
    // call width: 3
    if g.call(t, 0, &stack).eax != 0 {
        crate::sb_misc::timeline_run(g, img, img.rel32_target(f + 0x66), w(0x61));
    }
    let t = slot_target(g, w(0x6b), 261);
    if g.call(t, 0, &[1]).eax != 0 {
        crate::sb_misc::timeline_run(g, img, img.rel32_target(f + 0x86), w(0x81));
    }
    let t = slot_target(g, w(0x8b), 261);
    if g.call(t, 0, &[2]).eax == 0 {
        let n = w(0xd2);
        g.write::<u32>(n, g.read::<u32>(n).wrapping_add(1));
        return Ret::eax(1);
    }
    crate::sb_misc::timeline_stop(g, w(0xa1));
    crate::sb_misc::timeline_stop(g, w(0xab));
    let t = slot_target(g, w(0xb5), 214);
    let e = g.read::<u32>(w(0xbb));
    g.call(t, 0, &[e]);
    Ret::eax(1)
}

/// 0x10001640 (220 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 220,
    hash: 0x90f5_2861_f3f4_6c07,
    calls: &[
        (0x66, Callee::Sig(&crate::sb_misc::TIMELINE_RUN)),
        (0x86, Callee::Sig(&crate::sb_misc::TIMELINE_RUN)),
        (0xa6, Callee::Sig(&crate::sb_misc::TIMELINE_STOP)),
        (0xb0, Callee::Sig(&crate::sb_misc::TIMELINE_STOP)),
    ],
    imports: &[],
};

/// The effect's start (0x10001170, an overlay table entry, no
/// arguments): the name global set, slot 923 `(1)`, slot 718 `(*G_N)`,
/// slot 222 `(*G_N, 0, 1000, 0x2000)`, slot 378 `(0x2000)`, a handle `h`
/// from slot 719 `(*G_N, 8, 0)` (kept), slot 711 `(h, PROG, 0)` under
/// `**G_LOCK` 1 and slot 710 `(BLOCK, *G_H, *G_BUF, 0x80000)`; the
/// handle's three program tables (slot 941 `(h[0x14 + 4i])`, kept), then
/// `**G_P = ITEM` and the part started with program 0 of the table of
/// `h[0x10]` (the `pppStartPart` wrapper,
/// [`start_part`](crate::opu_parts::start_part)); three 16-byte blocks
/// copied from `.rdata`. Each call sees the words the earlier ones left on
/// the stack, up to the clean-up. Returns the wrapper's `eax`.
pub fn start_effect<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let r = |g: &G, op: u32| g.read::<u32>(w(op));
    let t = slot_target(g, w(0x1), 923);
    g.write::<u32>(w(0x10), w(0x14));
    let mut stack = vec![1];
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x1b), 718);
    let n = r(g, 0x21);
    stack.insert(0, n);
    // call width: 18 at most (the earlier calls' words)
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x2e), 222);
    let n = r(g, 0x46);
    stack.splice(0..0, [n, 0, 0x3e8, 0x2000]);
    // call width: 6
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x4d), 378);
    stack.insert(0, 0x2000);
    // call width: 7
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x5f), 719);
    let n = r(g, 0x6f);
    stack.splice(0..0, [n, 8, 0]);
    // call width: 10
    let h = g.call(t, 0, &stack).eax;
    let lock = r(g, 0x77);
    let saved = g.read::<u32>(lock);
    g.write::<u32>(w(0x80), h);
    let t = slot_target(g, w(0x8b), 711);
    g.write::<u32>(lock, 1);
    stack.splice(0..0, [h, w(0x85), 0]);
    // call width: 13
    g.call(t, 0, &stack);
    let lock = r(g, 0x9e);
    let buf = r(g, 0xa9);
    g.write::<u32>(lock, saved);
    let h2 = r(g, 0xb1);
    let t = slot_target(g, w(0xb6), 710);
    stack.splice(0..0, [w(0xbb), h2, buf, 0x8_0000]);
    // call width: 17
    g.call(t, 0, &stack);
    let mut stack = Vec::new();
    for (k, (hop, tab, dst)) in [
        (0xc8u32, 0xd3u32, 0xe0u32),
        (0xe5, 0xed, 0xfa),
        (0xff, 0x107, 0x114),
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
    let pp = r(g, 0x119);
    g.write::<u32>(pp, w(0x11f));
    let h = r(g, 0x124);
    let x = g.read::<u32>(h + 0x10);
    let t = slot_target(g, w(0x12c), 941);
    stack.insert(0, x);
    // call width: 18 at most (the earlier calls' words)
    let table = g.call(t, 0, &stack).eax;
    let prog = g.read::<u32>(table);
    let eax = start_part(
        g,
        img,
        img.rel32_target(f + 0x149),
        w(0x144),
        0x1000,
        prog,
        1,
    );
    for (src, dst) in [(0x150u32, 0x15au32), (0x162, 0x16a), (0x171, 0x178)] {
        for i in 0..4u32 {
            let v = g.read::<u32>(w(src) + 4 * i);
            g.write::<u32>(w(dst) + 4 * i, v);
        }
    }
    Ret::eax(eax)
}

/// [`start_effect`] (382 bytes, 1 overlay).
pub const START_EFFECT: Sig = Sig {
    len: 382,
    hash: 0x556f_9e5a_e8a9_e453,
    calls: &[(0x149, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// The effect's set-up (0x100013b0, an overlay table entry, `(arg)`):
/// slot 803 `(0x1000000, 16)` the buffer (its address and `+0x80000` to
/// two globals, the argument to a third), the name global set, slot 923
/// `(0)`, slot 718 `(*G_N)`, slot 222 `(*G_N, 0, 1000, 0x2000)`, a handle
/// `h` from slot 719 `(*G_N, 8, 0)` (kept), slot 711 `(h, PROG, 0)` under
/// `**G_LOCK` 1 and slot 710 `(BLOCK, *G_H, *G_BUF, 0x80000)`; the
/// handle's three program tables (slot 941 `(h[0x14 + 4i])`, kept); the
/// two 0x110-byte timeline blocks zeroed (`memset`) with their callbacks,
/// counts and flags. Each call sees the words the earlier ones left on
/// the stack, up to the clean-up. Returns 1.
pub fn setup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let r = |g: &G, op: u32| g.read::<u32>(w(op));
    let t = slot_target(g, w(0x4), 803);
    let mut stack = vec![0x100_0000, 0x10];
    let buf = g.call(t, 0, &stack).eax;
    g.write::<u32>(w(0x19), buf);
    g.write::<u32>(w(0x23), buf.wrapping_add(0x8_0000));
    let arg = fr.arg(g, 0);
    g.write::<u32>(w(0x2b), arg);
    let t = slot_target(g, w(0x30), 923);
    g.write::<u32>(w(0x3e), w(0x42));
    stack.insert(0, 0);
    // call width: 18 at most (the earlier calls' words)
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x49), 718);
    let n = r(g, 0x4f);
    stack.insert(0, n);
    // call width: 18 at most (the earlier calls' words)
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x5c), 222);
    let n = r(g, 0x74);
    stack.splice(0..0, [n, 0, 0x3e8, 0x2000]);
    // call width: 8
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x7b), 719);
    let n = r(g, 0x8b);
    stack.splice(0..0, [n, 8, 0]);
    // call width: 11
    let h = g.call(t, 0, &stack).eax;
    let lock = r(g, 0x93);
    let saved = g.read::<u32>(lock);
    g.write::<u32>(w(0x9c), h);
    let t = slot_target(g, w(0xa7), 711);
    g.write::<u32>(lock, 1);
    stack.splice(0..0, [h, w(0xa1), 0]);
    // call width: 14
    g.call(t, 0, &stack);
    let lock = r(g, 0xba);
    let b = r(g, 0xc5);
    g.write::<u32>(lock, saved);
    let h2 = r(g, 0xcd);
    let t = slot_target(g, w(0xd2), 710);
    stack.splice(0..0, [w(0xd7), h2, b, 0x8_0000]);
    // call width: 18
    g.call(t, 0, &stack);
    let mut stack = Vec::new();
    for (k, (hop, tab, dst)) in [
        (0xe4u32, 0xefu32, 0xfcu32),
        (0x101, 0x109, 0x116),
        (0x11b, 0x123, 0x13c),
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
    g.write_bytes(w(0x137), &[0u8; 0x110]);
    g.write::<u32>(w(0x147), w(0x14b));
    for (op, v) in [(0x151u32, 2u32), (0x15b, 2), (0x165, 1), (0x16f, 1)] {
        g.write::<u32>(w(op), v);
    }
    g.write_bytes(w(0x17f), &[0u8; 0x110]);
    g.write::<u32>(w(0x18d), w(0x191));
    for op in [0x197u32, 0x1a1, 0x1ab] {
        g.write::<u32>(w(op), 1);
    }
    Ret::eax(1)
}

/// [`setup`] (443 bytes, 1 overlay).
pub const SETUP: Sig = Sig {
    len: 443,
    hash: 0x5da4_f6f5_d560_d8a5,
    calls: &[
        (0x141, Callee::Import("memset")),
        (0x184, Callee::Import("memset")),
    ],
    imports: &[],
};
