//! `magic_0312.dll`'s own functions (besides the runtime, the object
//! host and the shared code): the object steps of its effect. Every slot
//! call passes the words the original leaves on the stack.

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// The effect context `*A` (operand `+0x04` in every function here).
fn ctx<G: Game>(g: &G, img: &Image, f: u32) -> u32 {
    g.read::<u32>(img.word(f + 0x04))
}

/// Slot `slot` (its table operand `table`) on the words pushed so far.
fn call<G: Game>(g: &mut G, img: &Image, f: u32, p: &Pushes, table: u32, slot: u32) -> u32 {
    let s = slot_target(g, img.word(f + table), slot);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0).eax
}

/// The object's last two slots: 450 `(obj, F1)`, then `last` `(obj, b)`.
#[allow(clippy::too_many_arguments)]
fn close<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    p: &mut Pushes,
    obj: u32,
    (t450, f1): (u32, u32),
    (t_last, last, b): (u32, u32, u32),
) -> u32 {
    p.push_all(&[img.word(f + f1), obj]);
    call(g, img, f, p, t450, 0x708 / 4);
    p.push_all(&[b, obj]);
    call(g, img, f, p, t_last, last)
}

/// The start (0x10001000, cdecl `(obj)`): `+0x0a` and eleven more words
/// cleared; slot 481 `(A[0x1a0])`'s answer into `+0xa9` and slot 14
/// `(it, P1, P2, P3)` (`+0xae`, `+0xa4`, `+0x9f`); slot 448 `(1, 0)`;
/// slots 450 `(obj, F1)` and 449 `(obj, F2)` (`+0xd4`, `+0xe9`). Returns
/// slot 449's `eax`.
pub fn start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let c = ctx(g, img, f);
    g.write::<u32>(w(0x0a), 0);
    let mut p = Pushes::default();
    p.push(g.read::<u32>(c + 0x1a0));
    for at in [
        0x20, 0x2a, 0x34, 0x3e, 0x48, 0x52, 0x5c, 0x66, 0x70, 0x7a, 0x84, 0x8e,
    ] {
        g.write::<u32>(w(at), 0);
    }
    let r = call(g, img, f, &p, 0x1a, 0x784 / 4);
    p.push_all(&[w(0x9f), w(0xa4)]);
    g.write::<u32>(w(0xa9), r);
    p.push_all(&[w(0xae), r]);
    call(g, img, f, &p, 0xb4, 0x38 / 4);
    p.push_all(&[0, 1]);
    call(g, img, f, &p, 0xbe, 0x700 / 4);
    let obj = fr.arg(g, 0);
    Ret::eax(close(
        g,
        img,
        f,
        &mut p,
        obj,
        (0xcf, 0xd4),
        (0xe4, 0x704 / 4, w(0xe9)),
    ))
}

/// [`start`] (253 bytes, 1 overlay).
pub const START: Sig = Sig {
    len: 253,
    hash: 0x3196_e69c_ac2c_0ab0,
    calls: &[],
    imports: &[],
};

/// The eleven words slot 486 gets (`(m, kind, 0, 0, n, 0x20, 0x20, x,
/// y, z, w)` with the four floats under them on the stack).
fn push_486(p: &mut Pushes, m: u32, kind: u32, n: u32, v: [u32; 4]) {
    p.push_all(&[v[3], v[2], v[1], v[0]]);
    p.push_all(&[0x20, 0x20, n, 0, 0, kind, m]);
}

/// The second start (0x10001150, cdecl `(obj)`, a local at `ebp - 4`):
/// slot 444 `(0)`; slot 367 `(A[4], A[0x1a0], 0, 0, &flags, 0, 0, 0)`
/// fills the local; slot 434 `(*A, F)` with `F` `+0x62` when its bit 8
/// is set, else `+0x69`; slot 486 `(m, 2, 0, 0, 20, 0x20, 0x20, 15.0, 0,
/// 0, 0)` and 447 `(m, 0xb0)` (`m` = `A[0x198]`); slots 450 `(obj, F1)`
/// and 449 `(obj, F2)` (`+0xdc`, `+0xe9`); the object's current list word
/// (`+0x2c + 16 [+0xac]`) gets `m`. Returns twice the list index.
pub fn start_flagged<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let c = ctx(g, img, f);
    let part = g.read::<u32>(c + 0x1a0);
    let m = g.read::<u32>(c + 0x198);
    let mut p = Pushes::default();
    p.push(0);
    call(g, img, f, &p, 0x18, 0x6f0 / 4);
    let flags = fr.esp.wrapping_sub(8);
    let c = ctx(g, img, f);
    p.push_all(&[0, 0, 0, flags, 0, 0, part, g.read::<u32>(c + 4)]);
    call(g, img, f, &p, 0x3e, 0x5bc / 4);
    p.pop(9);
    let s = slot_target(g, w(0x4b), 0x6c8 / 4);
    let cb = if g.read::<u32>(flags) & 0x100 != 0 {
        w(0x62)
    } else {
        w(0x69)
    };
    p.push_all(&[cb, ctx(g, img, f)]);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    p.pop(2);
    push_486(&mut p, m, 2, 0x14, [0x4170_0000, 0, 0, 0]);
    call(g, img, f, &p, 0x76, 0x798 / 4);
    p.push_all(&[0xb0, m]);
    call(g, img, f, &p, 0xbb, 0x6fc / 4);
    let obj = fr.arg(g, 0);
    close(
        g,
        img,
        f,
        &mut p,
        obj,
        (0xce, 0xdc),
        (0xe4, 0x704 / 4, w(0xe9)),
    );
    let i = g.read::<u32>(obj + 0xac).wrapping_mul(2);
    g.write::<u32>(obj.wrapping_add(i.wrapping_mul(8)).wrapping_add(0x2c), m);
    Ret::eax(i)
}

/// [`start_flagged`] (267 bytes, 1 overlay).
pub const START_FLAGGED: Sig = Sig {
    len: 267,
    hash: 0x556d_d3e6_9a20_c740,
    calls: &[],
    imports: &[],
};

/// The callbacks' set-up (0x10001260, cdecl `(obj)`): slot 434 `(*A, F)`
/// for the three callbacks `+0x10`, `+0x28`, `+0x40`; slot 447 `(m, 0xb1)`
/// (`m` = `A[0x198]`); slots 450 `(obj, F)` (`+0x74`) and 451 `(obj,
/// 0x18)`. Returns slot 451's `eax`.
pub fn callbacks<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let c = ctx(g, img, f);
    let m = g.read::<u32>(c + 0x198);
    let mut p = Pushes::default();
    p.push_all(&[w(0x10), c]);
    call(g, img, f, &p, 0x16, 0x6c8 / 4);
    for (cb, table) in [(0x28, 0x23), (0x40, 0x3b)] {
        p.push_all(&[w(cb), ctx(g, img, f)]);
        call(g, img, f, &p, table, 0x6c8 / 4);
    }
    p.push_all(&[0xb1, m]);
    call(g, img, f, &p, 0x53, 0x6fc / 4);
    let obj = fr.arg(g, 0);
    Ret::eax(close(
        g,
        img,
        f,
        &mut p,
        obj,
        (0x66, 0x74),
        (0x7c, 0x70c / 4, 0x18),
    ))
}

/// [`callbacks`] (145 bytes, 1 overlay).
pub const CALLBACKS: Sig = Sig {
    len: 145,
    hash: 0xa5f3_b19c_63f7_d40a,
    calls: &[],
    imports: &[],
};

/// The third start (0x10001350, cdecl `(obj)`): slot 486 `(m, 1, 0, 0,
/// 10, 0x20, 0x20, 0, 1.0, 0.4, 0)` and 447 `(m, 0xb2)` (`m` =
/// `A[0x198]`), slots 450 `(obj, F1)` and 449 `(obj, F2)` (`+0x74`,
/// `+0x81`); the object's current list word gets `m`. Returns twice the
/// list index.
pub fn start_plain<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let m = g.read::<u32>(ctx(g, img, f) + 0x198);
    let mut p = Pushes::default();
    push_486(&mut p, m, 1, 0xa, [0, 0x3f80_0000, 0x3ecc_cccd, 0]);
    call(g, img, f, &p, 0x11, 0x798 / 4);
    p.push_all(&[0xb2, m]);
    call(g, img, f, &p, 0x53, 0x6fc / 4);
    let obj = fr.arg(g, 0);
    close(
        g,
        img,
        f,
        &mut p,
        obj,
        (0x66, 0x74),
        (0x7c, 0x704 / 4, w(0x81)),
    );
    let i = g.read::<u32>(obj + 0xac).wrapping_mul(2);
    g.write::<u32>(obj.wrapping_add(i.wrapping_mul(8)).wrapping_add(0x2c), m);
    Ret::eax(i)
}

/// [`start_plain`] (161 bytes, 1 overlay).
pub const START_PLAIN: Sig = Sig {
    len: 161,
    hash: 0x5eec_4d88_9b65_01ab,
    calls: &[],
    imports: &[],
};

/// An object's placement (0x100014b0, cdecl `(obj)`): slots 443 `(obj,
/// 0x10)`, 442 `(obj, m, 7)` (`m` = `A[0x198]`), the object's words
/// `+0xd0..+0x13c` set, slot 440 `(*A, obj, 0, 0)`, slots 450 `(obj, F1)`
/// and 449 `(obj, F2)` (`+0xca`, `+0xdd`). Returns slot 449's `eax`.
pub fn place<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let m = g.read::<u32>(ctx(g, img, f) + 0x198);
    let obj = fr.arg(g, 0);
    let mut p = Pushes::default();
    p.push_all(&[0x10, obj]);
    call(g, img, f, &p, 0x10, 0x6ec / 4);
    p.push_all(&[7, m, obj]);
    call(g, img, f, &p, 0x24, 0x6e8 / 4);
    p.push_all(&[0, 0, obj, g.read::<u32>(w(0x40))]);
    for (off, v) in [
        (0xd0, 0),
        (0xd4, 0x40a0_0000),
        (0xd8, 0),
        (0xdc, 0),
        (0x100, 0),
        (0x104, 0x4334_0000),
        (0x108, 0),
        (0x10c, 0),
        (0x130, 0x3dcc_cccd),
        (0x134, 0x3dcc_cccd),
        (0x138, 0x3dcc_cccd),
        (0x13c, 0),
    ] {
        g.write::<u32>(obj + off, v);
    }
    call(g, img, f, &p, 0x35, 0x6e0 / 4);
    Ret::eax(close(
        g,
        img,
        f,
        &mut p,
        obj,
        (0xc5, 0xca),
        (0xd8, 0x704 / 4, w(0xdd)),
    ))
}

/// [`place`] (241 bytes, 1 overlay).
pub const PLACE: Sig = Sig {
    len: 241,
    hash: 0xc978_5ace_0010_ed75,
    calls: &[],
    imports: &[],
};

/// An object's start (0x10001640, cdecl `(obj)`): slots 442 `(obj,
/// A[0x1a0], 6)` and 443 `(obj, 0x40)`, the object's words `+0xd0..`
/// set, slots 450 `(obj, F)` (`+0x8e`) and 451 `(obj, 0x17)`. Returns
/// slot 451's `eax`.
pub fn object_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let c = ctx(g, img, f);
    let obj = fr.arg(g, 0);
    let mut p = Pushes::default();
    p.push_all(&[6, g.read::<u32>(c + 0x1a0), obj]);
    call(g, img, f, &p, 0x15, 0x6e8 / 4);
    p.push_all(&[0x40, obj]);
    call(g, img, f, &p, 0x23, 0x6ec / 4);
    for (off, v) in [
        (0xd0, 0),
        (0xd4, 0xc0e0_0000),
        (0xd8, 0),
        (0xdc, 0),
        (0x130, 0x3dcc_cccd),
        (0x134, 0x3dcc_cccd),
        (0x138, 0x3dcc_cccd),
        (0x13c, 0),
    ] {
        g.write::<u32>(obj + off, v);
    }
    Ret::eax(close(
        g,
        img,
        f,
        &mut p,
        obj,
        (0x33, 0x8e),
        (0x96, 0x70c / 4, 0x17),
    ))
}

/// [`object_start`] (171 bytes, 1 overlay).
pub const OBJECT_START: Sig = Sig {
    len: 171,
    hash: 0x81a4_5a12_6b61_fb6c,
    calls: &[],
    imports: &[],
};
