//! `magic_0359.dll`'s own functions (besides the runtime, the object
//! host and the shared code): the object steps of its effect. Every slot
//! call passes the words the original leaves on the stack.

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// Slot `slot` (table operand `table`) on the words pushed so far.
fn call<G: Game>(g: &mut G, img: &Image, f: u32, p: &Pushes, table: u32, slot: u32) -> u32 {
    let s = slot_target(g, img.word(f + table), slot);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0).eax
}

/// `m` = `(*A)[0x198]` (`A` at `+0x04`).
fn model<G: Game>(g: &G, img: &Image, f: u32) -> u32 {
    g.read::<u32>(g.read::<u32>(img.word(f + 0x04)) + 0x198)
}

/// The words under slot 486's first seven: four floats of 0 (`fresh`
/// pushes all four; else the word already on top is the deepest).
fn floats_486(p: &mut Pushes, fresh: bool) {
    if fresh {
        p.push(0);
    } else {
        p.set_top(0);
    }
    p.push_all(&[0, 0, 0]);
}

/// The objects' last two slots and the list word: 450 `(obj, F1)`, 449
/// `(obj, F2)`, then the object's current list word gets `m`. Returns
/// twice the list index.
#[allow(clippy::too_many_arguments)]
fn close_listed<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    p: &mut Pushes,
    obj: u32,
    m: u32,
    (t450, f1): (u32, u32),
    (t449, f2): (u32, u32),
) -> u32 {
    p.push_all(&[img.word(f + f1), obj]);
    call(g, img, f, p, t450, 0x708 / 4);
    p.push_all(&[img.word(f + f2), obj]);
    call(g, img, f, p, t449, 0x704 / 4);
    let i = g.read::<u32>(obj + 0xac).wrapping_mul(2);
    g.write::<u32>(obj.wrapping_add(i.wrapping_mul(8)).wrapping_add(0x2c), m);
    i
}

/// The first step (0x100010a0, cdecl `(obj)`): slot 444 `(0)`; slot 486
/// `(m, 2, 0, 0, 0x20, 0x20, 0x20, 0, 0, 0, 0)`; slot 447 `(m, 0xb2)`;
/// slot 434 `(*A, F)` (`+0x78`); slots 450 `(obj, F2)` (`+0x99`) and 451
/// `(obj, 4)`. Returns slot 451's `eax`.
pub fn step_a<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let m = model(g, img, f);
    let mut p = Pushes::default();
    p.push(0);
    call(g, img, f, &p, 0x10, 0x6f0 / 4);
    floats_486(&mut p, false);
    p.push_all(&[0x20, 0x20, 0x20, 0, 0, 2, m]);
    call(g, img, f, &p, 0x1f, 0x798 / 4);
    p.push_all(&[0xb2, m]);
    call(g, img, f, &p, 0x60, 0x6fc / 4);
    p.push_all(&[w(0x78), g.read::<u32>(w(0x7e))]);
    call(g, img, f, &p, 0x73, 0x6c8 / 4);
    let obj = fr.arg(g, 0);
    p.push_all(&[w(0x99), obj]);
    call(g, img, f, &p, 0x8b, 0x708 / 4);
    p.pop(17);
    p.push_all(&[4, obj]);
    Ret::eax(call(g, img, f, &p, 0xa1, 0x70c / 4))
}

/// [`step_a`] (185 bytes, 1 overlay).
pub const STEP_A: Sig = Sig {
    len: 185,
    hash: 0x2615_5761_bf14_aafe,
    calls: &[],
    imports: &[],
};

/// The second step (0x100011c0, cdecl `(obj)`): slot 447 `(m, 0xb3)`;
/// slot 434 `(*A, F)` (`+0x29`); slot 491 `(4, pi, 30, 10, 4)`; slot 478
/// `(2, 0x40, 30)`; slot 446 `(-5, 0)`; slots 450 `(obj, F1)` and 449
/// `(obj, F2)` (`+0x8e`, `+0x9b`); the list word gets `m`. Returns twice
/// the list index.
pub fn step_b<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let m = model(g, img, f);
    let mut p = Pushes::default();
    p.push_all(&[0xb3, m]);
    call(g, img, f, &p, 0x11, 0x6fc / 4);
    p.push_all(&[w(0x29), g.read::<u32>(w(0x2f))]);
    call(g, img, f, &p, 0x24, 0x6c8 / 4);
    p.pop(4);
    p.push_all(&[4, 0xa, 0x1e, 0x4049_0fdb, 4]);
    call(g, img, f, &p, 0x3c, 0x7ac / 4);
    p.push_all(&[0x1e, 0x40, 2]);
    call(g, img, f, &p, 0x5c, 0x778 / 4);
    p.push_all(&[0, u32::MAX - 4]);
    call(g, img, f, &p, 0x6f, 0x6f8 / 4);
    let obj = fr.arg(g, 0);
    Ret::eax(close_listed(
        g,
        img,
        f,
        &mut p,
        obj,
        m,
        (0x80, 0x8e),
        (0x96, 0x9b),
    ))
}

/// [`step_b`] (187 bytes, 1 overlay).
pub const STEP_B: Sig = Sig {
    len: 187,
    hash: 0x3749_7d26_0c87_ad3c,
    calls: &[],
    imports: &[],
};

/// The third step (0x10001280, cdecl `(obj)`): slot 486 `(m, 1, 0, 0,
/// 0x18, 0x20, 0x20, 0, 0, 0, 0)`; slot 447 `(m, 0xb4)`; two words 1
/// (`+0x7b`, `+0x85`); slots 450 `(obj, F1)` and 449 `(obj, F2)`
/// (`+0x74`, `+0x95`); the list word gets `m`. Returns twice the list
/// index.
pub fn step_c<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let m = model(g, img, f);
    let mut p = Pushes::default();
    floats_486(&mut p, true);
    p.push_all(&[0x20, 0x20, 0x18, 0, 0, 1, m]);
    call(g, img, f, &p, 0x11, 0x798 / 4);
    p.push_all(&[0xb4, m]);
    call(g, img, f, &p, 0x53, 0x6fc / 4);
    let obj = fr.arg(g, 0);
    p.push_all(&[w(0x74), obj]);
    g.write::<u32>(w(0x7b), 1);
    g.write::<u32>(w(0x85), 1);
    call(g, img, f, &p, 0x66, 0x708 / 4);
    p.push_all(&[w(0x95), obj]);
    call(g, img, f, &p, 0x90, 0x704 / 4);
    let i = g.read::<u32>(obj + 0xac).wrapping_mul(2);
    g.write::<u32>(obj.wrapping_add(i.wrapping_mul(8)).wrapping_add(0x2c), m);
    Ret::eax(i)
}

/// [`step_c`] (181 bytes, 1 overlay).
pub const STEP_C: Sig = Sig {
    len: 181,
    hash: 0xf9f2_048a_9b3a_1119,
    calls: &[],
    imports: &[],
};

/// The fourth step (0x10001340, cdecl `(obj)`): slot 447 `(m, 0xb5)`;
/// slot 434 `(*A, F)` (`+0x29`) with two words 3 (`+0x3b`, `+0x45`);
/// slots 450 `(obj, F1)` and 449 `(obj, F2)` (`+0x5e`, `+0x6b`); the list
/// word gets `m`. Returns twice the list index.
pub fn step_d<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let m = model(g, img, f);
    let mut p = Pushes::default();
    p.push_all(&[0xb5, m]);
    call(g, img, f, &p, 0x11, 0x6fc / 4);
    p.push_all(&[w(0x29), g.read::<u32>(w(0x2f))]);
    g.write::<u32>(w(0x3b), 3);
    g.write::<u32>(w(0x45), 3);
    call(g, img, f, &p, 0x24, 0x6c8 / 4);
    let obj = fr.arg(g, 0);
    Ret::eax(close_listed(
        g,
        img,
        f,
        &mut p,
        obj,
        m,
        (0x50, 0x5e),
        (0x66, 0x6b),
    ))
}

/// [`step_d`] (139 bytes, 1 overlay).
pub const STEP_D: Sig = Sig {
    len: 139,
    hash: 0x7b4e_8249_9968_7338,
    calls: &[],
    imports: &[],
};
