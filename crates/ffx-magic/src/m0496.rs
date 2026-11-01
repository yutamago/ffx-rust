//! `magic_0496.dll`'s own functions (besides the runtime and the shared
//! code): the object starts of its table (the objects lie in arrays of
//! 0x80-byte records; the record's index picks its program and its
//! place from a table of 16-byte points) and the frame.

use crate::part_draw::{Camera, camera, start_part};
use crate::runtime::slot_target;
use crate::sb_sse::{add, cvt, div, mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `1.0f32`'s bits.
const ONE: u32 = 0x3f80_0000;

/// The record index of `obj` in the array at operand `op` (`sub`, `sar
/// 7`).
fn index(img: &Image, f: u32, obj: u32, op: u32) -> u32 {
    (obj.wrapping_sub(img.word(f + op)).cast_signed() >> 7).cast_unsigned()
}

/// The object's place: `+0x40..+0x48` from the point table (operands
/// `ops`, `x`/`y`/`z`) at record `k`, `+0x4c` 1.0, `+0x50..+0x58` 0,
/// `+0x5c` 1.
fn place<G: Game>(g: &mut G, img: &Image, f: u32, obj: u32, k: u32, ops: [u32; 3]) {
    for (i, op) in (0u32..).zip(ops) {
        let at = img.word(f + op).wrapping_add(k << 4);
        let v = g.read::<u32>(at);
        g.write::<u32>(obj + 0x40 + 4 * i, v);
    }
    g.write::<u32>(obj + 0x4c, ONE);
    for off in [0x50, 0x54, 0x58] {
        g.write::<u32>(obj + off, 0);
    }
    g.write::<u32>(obj + 0x5c, 1);
}

/// The object's three scales `+0x60..+0x68` (bits).
fn scale<G: Game>(g: &mut G, obj: u32, s: u32) {
    for off in [0x60, 0x64, 0x68] {
        g.write::<u32>(obj + off, s);
    }
}

/// An object's start (0x10001000, `(obj)`): the first record of the
/// array at operand `+0x2a` starts program 0 with the scales 0.1875,
/// the others program 1 with 0.2095 ([`start_part`]); `+0x6c` 1.0 and
/// the record's place ([`place`]). Returns 1.
pub fn e1000<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let obj = fr.arg(g, 0);
    let k = index(img, f, obj, 0x2a);
    if k == 0 {
        start_part(g, img, f, obj, [0x4, 0xe, 0x13, 0x1c, 0x45], 0, &[]);
        scale(g, obj, 0x3e40_0000);
    } else {
        start_part(g, img, f, obj, [0x4, 0xe, 0x13, 0x1c, 0x64], 4, &[]);
        scale(g, obj, 0x3e56_876e);
    }
    g.write::<u32>(obj + 0x6c, ONE);
    place(g, img, f, obj, k, [0x8c, 0x95, 0x9e]);
    Ret::eax(1)
}

/// 0x10001000 (209 bytes, 1 overlay).
pub const E1000: Sig = Sig {
    len: 209,
    hash: 0x2d35_09fd_64c7_3a62,
    calls: &[
        (0x45, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x64, Callee::Sig(&crate::opu_parts::START_PART)),
    ],
    imports: &[],
};

/// An object's start (0x100011a0, `(obj)`): the first record of the
/// array at operand `+0x2a` starts program 5, the others program 6
/// ([`start_part`]); the record's place ([`place`]); the scales `K1`
/// (`+0x9e`) for record 4, else `K2` (`+0xa8`); `+0x6c` 1.0. Returns 1.
pub fn e11a0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let k = index(img, f, obj, 0x2a);
    let program = if k == 0 { 0x14 } else { 0x18 };
    start_part(g, img, f, obj, [0x4, 0xe, 0x13, 0x1c, 0x4b], program, &[]);
    place(g, img, f, obj, k, [0x59, 0x62, 0x6b]);
    let s = g.read::<u32>(w(if k == 4 { 0x9e } else { 0xa8 }));
    scale(g, obj, s);
    g.write::<u32>(obj + 0x6c, ONE);
    Ret::eax(1)
}

/// 0x100011a0 (203 bytes, 1 overlay).
pub const E11A0: Sig = Sig {
    len: 203,
    hash: 0x563a_84da_0253_56a9,
    calls: &[(0x4b, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// An object's start (0x10001310, `(obj)`): program 3 started
/// ([`start_part`]), the record's place in the array at `+0x3e`
/// ([`place`]), the scales 0.1515 (`+0x6c` 1.0). Returns 1.
pub fn e1310<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let obj = fr.arg(g, 0);
    start_part(g, img, f, obj, [0x4, 0xb, 0x10, 0x18, 0x36], 0xc, &[]);
    let k = index(img, f, obj, 0x3e);
    place(g, img, f, obj, k, [0x4d, 0x56, 0x5f]);
    scale(g, obj, 0x3e1b_2314);
    g.write::<u32>(obj + 0x6c, ONE);
    Ret::eax(1)
}

/// 0x10001310 (173 bytes, 1 overlay).
pub const E1310: Sig = Sig {
    len: 173,
    hash: 0x900d_bbe6_cbbf_d0e2,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// An object's start (0x100013c0, `(obj)`): program 4 started
/// ([`start_part`]), the record's place in the array at `+0x3e`
/// ([`place`]); a random scale `((r & 0x7fff) / D - S) * M1 * M2 + A`
/// with `r` slot 324's answer (`+0x6c` 1.0). Returns 1.
pub fn e13c0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let k = |g: &G, op: u32| g.read::<f32>(w(op));
    let obj = fr.arg(g, 0);
    start_part(g, img, f, obj, [0x4, 0xb, 0x10, 0x18, 0x36], 0x10, &[]);
    let i = index(img, f, obj, 0x3e);
    place(g, img, f, obj, i, [0x4d, 0x56, 0x5f]);
    let t = slot_target(g, w(0x67), 324);
    let r = g.call(t, 0, &[]).eax & 0x7fff;
    g.write::<u32>(obj + 0x6c, ONE);
    let x = div(cvt(r), k(g, 0xad));
    let x = sub(x, k(g, 0xba));
    let x = mul(x, k(g, 0xc2));
    let x = mul(x, k(g, 0xca));
    let x = add(x, k(g, 0xd2));
    scale(g, obj, x.to_bits());
    Ret::eax(1)
}

/// 0x100013c0 (232 bytes, 1 overlay).
pub const E13C0: Sig = Sig {
    len: 232,
    hash: 0xaef7_0a31_7202_5cc5,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// The effect's frame (0x10001800, an overlay table entry, no
/// arguments): 0 while the flag at operand `+0x02` is clear. Else the
/// matrices ([`camera`]), `*G_V = *G_W`; when slot 261 `(0)` says
/// nonzero the three timelines run
/// ([`timeline_run`](crate::sb_misc::timeline_run)); when `(1)` does
/// they stop ([`timeline_stop`](crate::sb_misc::timeline_stop)) and
/// slot 214 `(*G_E)` runs, else the frame count `+0xd0` goes up.
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
    stack.insert(0, 0);
    // call width: 3
    if g.call(t, 0, &stack).eax != 0 {
        for (call, this) in [(0x66, 0x61), (0x70, 0x6b), (0x7a, 0x75)] {
            crate::sb_misc::timeline_run(g, img, img.rel32_target(f + call), w(this));
        }
    }
    let t = slot_target(g, w(0x7f), 261);
    if g.call(t, 0, &[1]).eax == 0 {
        let n = w(0xd0);
        g.write::<u32>(n, g.read::<u32>(n).wrapping_add(1));
        return Ret::eax(1);
    }
    for op in [0x95, 0x9f, 0xa9] {
        crate::sb_misc::timeline_stop(g, w(op));
    }
    let t = slot_target(g, w(0xb3), 214);
    let e = g.read::<u32>(w(0xb9));
    g.call(t, 0, &[e]);
    Ret::eax(1)
}

/// 0x10001800 (218 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 218,
    hash: 0xa227_eebd_f4ef_282e,
    calls: &[
        (0x66, Callee::Sig(&crate::sb_misc::TIMELINE_RUN)),
        (0x70, Callee::Sig(&crate::sb_misc::TIMELINE_RUN)),
        (0x7a, Callee::Sig(&crate::sb_misc::TIMELINE_RUN)),
        (0x9a, Callee::Sig(&crate::sb_misc::TIMELINE_STOP)),
        (0xa4, Callee::Sig(&crate::sb_misc::TIMELINE_STOP)),
        (0xae, Callee::Sig(&crate::sb_misc::TIMELINE_STOP)),
    ],
    imports: &[],
};

/// The effect's start-up (0x100014b0, an overlay table entry, `(arg)`):
/// slot 803 `(0x1000000, 0x10)` the buffer (`+0x19`, and `+0x23` half a
/// MiB on), `arg` to `+0x2b`; the name (`+0x35`) to `+0x41` and slot
/// 718, slot 222 `(name, 0, 1000, 0x2000)`, slot 719 `(name, 8, 0)` the
/// data `d` (`+0x8c`); slot 711 `(d, PARTS, 0)` with the game flag
/// `*G_LOCK` 1; slot 710 `(ITEM, d, buf, 0x80000)`; three slot 941
/// `(d[5 + k])` the program tables; three timelines cleared and given
/// their defaults (23, 9 and 7 events, each event's flag 1); slot 260
/// `(0)` a handle (`+0x27e`). Every call passes the words on the
/// original's stack. Returns 1.
#[allow(clippy::too_many_lines)]
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
    let zero = [0u8; 0x110];
    g.write_bytes(w(0x127), &zero);
    // add esp, 0x18
    g.write::<u32>(w(0x13a), w(0x13e));
    g.write::<u32>(w(0x144), 0x17);
    g.write::<u32>(w(0x14e), 0x17);
    let mut p = w(0x157);
    loop {
        g.write::<u32>(p, 1);
        p = p.wrapping_add(8);
        if p.cast_signed() >= w(0x16a).cast_signed() {
            break;
        }
    }
    g.write_bytes(w(0x178), &zero);
    let mut stack = vec![w(0x178), 0, 0x110];
    g.write::<u32>(w(0x18f), w(0x193));
    g.write::<u32>(w(0x199), 9);
    g.write::<u32>(w(0x1a3), 9);
    for op in [
        0x1ad, 0x1b7, 0x1c1, 0x1cb, 0x1d5, 0x1df, 0x1e9, 0x1f3, 0x1fd,
    ] {
        g.write::<u32>(w(op), 1);
    }
    g.write_bytes(w(0x189), &zero);
    stack.splice(0..0, [w(0x189), 0, 0x110]);
    let t = slot_target(g, w(0x20b), 260);
    g.write::<u32>(w(0x211), w(0x215));
    g.write::<u32>(w(0x21b), 7);
    g.write::<u32>(w(0x225), 7);
    for op in [0x22f, 0x239, 0x243, 0x24d, 0x257, 0x261, 0x26b] {
        g.write::<u32>(w(op), 1);
    }
    stack.insert(0, 0);
    // call width: 17
    let h = g.call(t, 0, &stack).eax;
    g.write::<u32>(w(0x27e), h);
    Ret::eax(1)
}

/// 0x100014b0 (652 bytes, 1 overlay).
pub const START_UP: Sig = Sig {
    len: 652,
    hash: 0x2e40_4e1d_63e7_194e,
    calls: &[
        (0x131, Callee::Import("memset")),
        (0x17d, Callee::Import("memset")),
        (0x206, Callee::Import("memset")),
    ],
    imports: &[],
};
