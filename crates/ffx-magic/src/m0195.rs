//! `magic_0195.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// An object's step (0x10001350, cdecl `(obj)`): the counter `*C`
/// (`+0x10`) goes up; below 7, slot 461 `(obj, F)` (`+0x25`) and its
/// `eax`. Else the counter `*D` (`+0x3b`) goes up: below 2 `*C` restarts
/// at 0 and the callback is `F1` (`+0x64`), else `F2` (`+0x6b`); slots
/// 450 `(obj, F)` and 449 `(obj, F3)` (`+0x78`), and the object's current
/// list word (`+0x2c + 16 * [+0xac]`) gets `(*A)[0x198]` (`+0x04`, read
/// first). Returns twice the list index. Each slot call passes the words
/// the original leaves on the stack.
pub fn count_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let obj = fr.arg(g, 0);
    let v = g.read::<u32>(g.read::<u32>(w(0x04)) + 0x198);
    let c = g.read::<u32>(w(0x10)).wrapping_add(1);
    g.write::<u32>(w(0x16), c);
    if c.cast_signed() < 7 {
        let s = slot_target(g, w(0x20), 461);
        return Ret::eax(g.call(s, 0, &[obj, w(0x25)]).eax);
    }
    let d = g.read::<u32>(w(0x3b)).wrapping_add(1);
    g.write::<u32>(w(0x41), d);
    let s = slot_target(g, w(0x49), 450);
    let cb = if d.cast_signed() >= 2 {
        w(0x6b)
    } else {
        g.write::<u32>(w(0x5b), 0);
        w(0x64)
    };
    g.call(s, 0, &[obj, cb]);
    let s = slot_target(g, w(0x73), 449);
    g.call(s, 0, &[obj, w(0x78), obj, cb]);
    let i = g
        .read::<u32>(obj + 0xac)
        .wrapping_add(g.read::<u32>(obj + 0xac));
    g.write::<u32>(obj.wrapping_add(i.wrapping_mul(8)).wrapping_add(0x2c), v);
    Ret::eax(i)
}

/// [`count_step`] (152 bytes, 1 overlay).
pub const COUNT_STEP: Sig = Sig {
    len: 152,
    hash: 0x29b4_0bee_6a34_3dc5,
    calls: &[],
    imports: &[],
};

/// An object's flag step (0x10001910, cdecl `(obj)`): with `i` the
/// object's list index (`+0xac`), slot 444 `(1)`, slot 446 `(T[b], 0 /
/// 8)` with `b` the list word `+0x2c + 16 i`, `T` at `+0x49`, 0 when the
/// word `+0x30 + 16 i` is set; slots 450 `(obj, F1)` and 449 `(obj, F2)`
/// (`+0x5e`, `+0x6b`). Returns slot 449's `eax`.
///
/// The census counts the callback after it (0x10001990, [`next_step`])
/// as part of this function.
pub fn flag_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let obj = fr.arg(g, 0);
    let i = g.read::<u32>(obj + 0xac);
    let b = g.read::<u32>(obj.wrapping_add(i.wrapping_mul(16)).wrapping_add(0x2c));
    let e = g.read::<u32>(obj.wrapping_add(i.wrapping_add(3).wrapping_mul(16)));
    let s = slot_target(g, w(0x22), 444);
    g.call(s, 0, &[1]);
    let tb = g.read::<u32>(w(0x49).wrapping_add(b.wrapping_mul(4)));
    let s = slot_target(g, w(0x2f), 446);
    g.call(s, 0, &[tb, if e == 0 { 8 } else { 0 }]);
    let s = slot_target(g, w(0x50), 450);
    g.call(s, 0, &[obj, w(0x5e)]);
    let s = slot_target(g, w(0x66), 449);
    Ret::eax(g.call(s, 0, &[obj, w(0x6b), obj, w(0x5e)]).eax)
}

/// [`flag_step`] (128 bytes, 1 overlay).
pub const FLAG_STEP: Sig = Sig {
    len: 128,
    hash: 0x5f79_b484_0eed_bfd9,
    calls: &[],
    imports: &[],
};

/// The next step (0x10001990, cdecl `(obj)`): slots 450 `(obj, F)`
/// (`+0x09`) and 451 `(obj, 0x20)`. Returns slot 451's `eax`.
pub fn next_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let obj = fr.arg(g, 0);
    let s = slot_target(g, w(0x04), 450);
    g.call(s, 0, &[obj, w(0x09)]);
    let s = slot_target(g, w(0x19), 451);
    Ret::eax(g.call(s, 0, &[obj, 0x20, obj, w(0x09)]).eax)
}

/// [`next_step`] (47 bytes, 1 overlay).
pub const NEXT_STEP: Sig = Sig {
    len: 47,
    hash: 0xc6c8_2521_c9db_72d2,
    calls: &[],
    imports: &[],
};

/// An object's start (0x10001430, cdecl `(obj)`; `m0194`'s
/// `object_start` kin): slot 441 `(obj, M, T[*C])` with `M` =
/// `(*A)[0x198]` (`A` at `+0x11`, the table `T` at `+0x0c` indexed by
/// `*C`, `+0x04`), the object's words `+0xd0..+0x13c` set, slot 440 `(*A,
/// obj, 0, 0)`, slots 450 `(obj, F1)` and 449 `(obj, F2)` (`+0xc2`,
/// `+0xd5`). Returns slot 449's `eax`. Each slot call passes the words
/// the original leaves on the stack.
pub fn object_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let obj = fr.arg(g, 0);
    let c = g.read::<u32>(w(0x04));
    let t = g.read::<u32>(w(0x0c).wrapping_add(c.wrapping_mul(4)));
    let m = g.read::<u32>(g.read::<u32>(w(0x11)) + 0x198);
    let s = slot_target(g, w(0x1f), 441);
    g.call(s, 0, &[obj, m, t]);
    let a = g.read::<u32>(w(0x38));
    for (off, v) in [
        (0xd0, 0xbff3_3333),
        (0xd4, 0),
        (0xd8, 0),
        (0xdc, 0),
        (0x100, 0),
        (0x104, 0),
        (0x108, 0x42b4_0000),
        (0x10c, 0),
        (0x130, 0x3dcc_cccd),
        (0x134, 0x3dcc_cccd),
        (0x138, 0x3dcc_cccd),
        (0x13c, 0),
    ] {
        g.write::<u32>(obj + off, v);
    }
    let s = slot_target(g, w(0x2d), 440);
    g.call(s, 0, &[a, obj, 0, 0, obj, m, t]);
    let s = slot_target(g, w(0xbd), 450);
    g.call(s, 0, &[obj, w(0xc2), a, obj, 0, 0, obj, m, t]);
    let s = slot_target(g, w(0xd0), 449);
    // call width: 11 (the earlier calls' words left on the stack)
    let words = [obj, w(0xd5), obj, w(0xc2), a, obj, 0, 0, obj, m, t];
    Ret::eax(g.call(s, 0, &words).eax)
}

/// [`object_start`] (232 bytes, 1 overlay).
pub const OBJECT_START: Sig = Sig {
    len: 232,
    hash: 0xa6d3_c559_85f0_8736,
    calls: &[],
    imports: &[],
};

/// A copy of an object (0x100017d0, cdecl `(obj)`): slot 434 `(*G,
/// START)` (`G` at `+0x17`, `START` the callback [`table_start`] at
/// `+0x11`) gives the new object; its current list words (`+0x2c +
/// 16 i` and `+0x30 + 16 (i + 2)`, `i` its index `+0xac`) get the
/// source's, and the source's flags (`+0`) get bit 2. Returns the second
/// word copied.
///
/// The census counts [`table_start`] (0x10001830) as part of this
/// function.
pub fn copy_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let s = slot_target(g, w(0x04), 0x6c8 / 4);
    let src = fr.arg(g, 0);
    let list = g.read::<u32>(w(0x17));
    let obj = g.call(s, 0, &[list, w(0x11)]).eax;
    let word = |g: &G, o: u32, k: u32| {
        let i = g.read::<u32>(o + 0xac);
        o.wrapping_add(i.wrapping_add(k).wrapping_mul(16))
    };
    let v = g.read::<u32>(word(g, src, 0) + 0x2c);
    g.write::<u32>(word(g, obj, 0) + 0x2c, v);
    let v = g.read::<u32>(word(g, src, 3));
    g.write::<u32>(word(g, obj, 3), v);
    let fl = g.read::<u32>(src);
    g.write::<u32>(src, fl | 4);
    Ret::eax(v)
}

/// [`copy_start`] (96 bytes, 1 overlay).
pub const COPY_START: Sig = Sig {
    len: 96,
    hash: 0x93ae_2b23_b2a8_5a8f,
    calls: &[],
    imports: &[],
};

/// The copy's start (0x10001830, cdecl `(obj)`; [`object_start`]'s
/// kin): with `c` the object's current list word (`+0x2c + 16 i`) the
/// mode `T1[c]` (`+0x77`) and the position `T2[c]` (three words, `+0x81`,
/// `+0x8d`, `+0x99`, rows of 16 bytes) into `+0xd0..+0xd8`; `+0xdc` 0 and
/// the scale `+0x130..+0x138` 0.2 (`+0x13c` 0); slot 440 `(*G, obj, 0,
/// mode)`. Mode 1: slots 450 `(obj, F1)` and 451 `(obj, 2)`; else 450
/// `(obj, F2)` and 449 `(obj, F3)`. Returns the last slot's `eax`. Each
/// slot call passes the words the original leaves on the stack.
pub fn table_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let obj = fr.arg(g, 0);
    let i = g.read::<u32>(obj + 0xac);
    let c = g.read::<u32>(obj.wrapping_add(i.wrapping_mul(16)).wrapping_add(0x2c));
    let mode = g.read::<u32>(w(0x17).wrapping_add(c.wrapping_mul(4)));
    for (k, op) in (0u32..).zip([0x21, 0x2d, 0x39]) {
        let v = g.read::<u32>(w(op).wrapping_add(c.wrapping_mul(16)));
        g.write::<u32>(obj + 0xd0 + 4 * k, v);
    }
    let s = slot_target(g, w(0x44), 0x6e0 / 4);
    let list = g.read::<u32>(w(0x4d));
    for (off, v) in [
        (0xdc, 0),
        (0x130, 0x3e4c_cccd),
        (0x134, 0x3e4c_cccd),
        (0x138, 0x3e4c_cccd),
        (0x13c, 0),
    ] {
        g.write::<u32>(obj + off, v);
    }
    g.call(s, 0, &[list, obj, 0, mode]);
    let s450 = slot_target(g, w(0x8c), 0x708 / 4);
    if mode == 1 {
        g.call(s450, 0, &[obj, w(0x9f)]);
        let s = slot_target(g, w(0xa7), 0x70c / 4);
        Ret::eax(g.call(s, 0, &[obj, 2, obj, w(0x9f)]).eax)
    } else {
        g.call(s450, 0, &[obj, w(0xbe)]);
        let s = slot_target(g, w(0xc6), 0x704 / 4);
        Ret::eax(g.call(s, 0, &[obj, w(0xcb), obj, w(0xbe)]).eax)
    }
}

/// [`table_start`] (223 bytes, 1 overlay).
pub const TABLE_START: Sig = Sig {
    len: 223,
    hash: 0x390a_6af1_e338_9f78,
    calls: &[],
    imports: &[],
};
