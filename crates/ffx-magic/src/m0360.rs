//! `magic_0360.dll`'s own functions (besides the runtime, the object
//! host and the shared code). Every slot call passes the words the
//! original leaves on the stack.

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// Slot `slot` (table operand `table`) on the words pushed so far.
fn call<G: Game>(g: &mut G, img: &Image, f: u32, p: &Pushes, table: u32, slot: u32) -> u32 {
    let s = slot_target(g, img.word(f + table), slot);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0).eax
}

/// Slot 485's fade words: `(0xff, 0xff, 0xff, 0x80, 0, 0, 0, 0x40, n)`.
fn fade(p: &mut Pushes, n: u32) {
    p.push_all(&[n, 0x40, 0, 0, 0, 0x80, 0xff, 0xff, 0xff]);
}

/// The first step (0x10001160, cdecl `(obj)`): with `m` = `(*A)[0x198]`
/// (`A` `+0x04`): slot 485 fade with 8; slot 434 `(*A, F)` for `+0x41`
/// and `+0x59`, the second's answer into `+0x6f`; slots 450 `(obj, F1)`
/// and 449 `(obj, F2)` (`+0x79`, `+0x8c`); the list word gets `m`.
/// Returns twice the list index.
pub fn step_a<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let m = g.read::<u32>(g.read::<u32>(w(0x04)) + 0x198);
    let mut p = Pushes::default();
    fade(&mut p, 8);
    call(g, img, f, &p, 0x11, 0x794 / 4);
    p.push_all(&[w(0x41), g.read::<u32>(w(0x47))]);
    call(g, img, f, &p, 0x3c, 0x6c8 / 4);
    p.push_all(&[w(0x59), g.read::<u32>(w(0x5f))]);
    let r = call(g, img, f, &p, 0x54, 0x6c8 / 4);
    let obj = fr.arg(g, 0);
    g.write::<u32>(w(0x6f), r);
    p.push_all(&[w(0x79), obj]);
    call(g, img, f, &p, 0x74, 0x708 / 4);
    p.push_all(&[w(0x8c), obj]);
    call(g, img, f, &p, 0x87, 0x704 / 4);
    let i = g.read::<u32>(obj + 0xac).wrapping_mul(2);
    g.write::<u32>(obj.wrapping_add(i.wrapping_mul(8)).wrapping_add(0x2c), m);
    Ret::eax(i)
}

/// [`step_a`] (172 bytes, 1 overlay).
pub const STEP_A: Sig = Sig {
    len: 172,
    hash: 0x93cf_4cbd_c7ff_e89a,
    calls: &[],
    imports: &[],
};

/// The second step (0x10001260, cdecl `(obj)`): slot 485 fade with 12;
/// slot 444 `(1)`; slot 446 `(-5, 0)`; slot 434 `(*A, F)` (`+0x54`,
/// `A` `+0x5a`); slots 450 `(obj, F1)` and 449 `(obj, F2)` (`+0x6c`,
/// `+0x8a`). Returns slot 449's `eax`.
pub fn step_b<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let mut p = Pushes::default();
    fade(&mut p, 0xc);
    call(g, img, f, &p, 0x04, 0x794 / 4);
    p.push(1);
    call(g, img, f, &p, 0x2f, 0x6f0 / 4);
    p.push_all(&[0, u32::MAX - 4]);
    call(g, img, f, &p, 0x3e, 0x6f8 / 4);
    p.push_all(&[w(0x54), g.read::<u32>(w(0x5a))]);
    call(g, img, f, &p, 0x4f, 0x6c8 / 4);
    let obj = fr.arg(g, 0);
    p.push_all(&[w(0x6c), obj]);
    call(g, img, f, &p, 0x67, 0x708 / 4);
    p.pop(16);
    p.push_all(&[w(0x8a), obj]);
    Ret::eax(call(g, img, f, &p, 0x7c, 0x704 / 4))
}

/// [`step_b`] (152 bytes, 1 overlay).
pub const STEP_B: Sig = Sig {
    len: 152,
    hash: 0xb1eb_ae36_b615_6d97,
    calls: &[],
    imports: &[],
};

/// An object's place (0x10001a80, cdecl `(obj)`): its words
/// `+0xd0..+0xdc` (0, -0.1, 0, 0) and scale `+0x130..+0x138` 0.5
/// (`+0x13c` 0); slot 440 `(*A, obj, 0, 0)` (`A` `+0x19`); slots 450
/// `(obj, F1)` and 449 `(obj, F2)` (`+0x75`, `+0x88`). Returns slot 449's
/// `eax`.
pub fn place<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let mut p = Pushes::default();
    p.push_all(&[0, 0, obj, g.read::<u32>(w(0x19))]);
    for (off, v) in [
        (0xd0, 0),
        (0xd4, 0xbdcc_cccd),
        (0xd8, 0),
        (0xdc, 0),
        (0x130, 0x3f00_0000),
        (0x134, 0x3f00_0000),
        (0x138, 0x3f00_0000),
        (0x13c, 0),
    ] {
        g.write::<u32>(obj + off, v);
    }
    call(g, img, f, &p, 0x04, 0x6e0 / 4);
    p.push_all(&[w(0x75), obj]);
    call(g, img, f, &p, 0x70, 0x708 / 4);
    p.push_all(&[w(0x88), obj]);
    Ret::eax(call(g, img, f, &p, 0x83, 0x704 / 4))
}

/// [`place`] (155 bytes, 1 overlay).
pub const PLACE: Sig = Sig {
    len: 155,
    hash: 0xa4da_151b_4a37_842c,
    calls: &[],
    imports: &[],
};
