//! `magic_0382.dll`'s own functions (besides the runtime, the object
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

/// A mark's start (0x100017f0, cdecl `(obj)`): with `m` = `(*A)[0x1a0]`
/// (`A` `+0x04`, read again for slot 442): slot 444 `(1)`, slot 442
/// `(obj, m, 6)`, slot 443 `(obj, 0x40)`, the scale `+0x130..+0x138`
/// 0.075 (`+0x13c` 0), slot 440 `(*A, obj, 0, 1)`, slot 446 `(m, 0)`,
/// slots 450 `(obj, F1)` and 449 `(obj, F2)` (`+0xa3`, `+0xb6`). Returns
/// slot 449's `eax`.
pub fn mark_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let m = g.read::<u32>(g.read::<u32>(w(0x04)) + 0x1a0);
    let mut p = Pushes::default();
    p.push(1);
    call(g, img, f, &p, 0x10, 0x6f0 / 4);
    let obj = fr.arg(g, 0);
    p.push_all(&[6, g.read::<u32>(g.read::<u32>(w(0x20)) + 0x1a0), obj]);
    call(g, img, f, &p, 0x30, 0x6e8 / 4);
    p.push_all(&[0x40, obj]);
    call(g, img, f, &p, 0x3e, 0x6ec / 4);
    p.push_all(&[1, 0, obj, g.read::<u32>(w(0x59))]);
    for (off, v) in [
        (0x130, 0x3d99_999a),
        (0x134, 0x3d99_999a),
        (0x138, 0x3d99_999a),
        (0x13c, 0),
    ] {
        g.write::<u32>(obj + off, v);
    }
    call(g, img, f, &p, 0x4e, 0x6e0 / 4);
    p.push_all(&[0, m]);
    call(g, img, f, &p, 0x8e, 0x6f8 / 4);
    p.push_all(&[w(0xa3), obj]);
    call(g, img, f, &p, 0x9e, 0x708 / 4);
    p.push_all(&[w(0xb6), obj]);
    Ret::eax(call(g, img, f, &p, 0xb1, 0x704 / 4))
}

/// [`mark_start`] (202 bytes, 1 overlay).
pub const MARK_START: Sig = Sig {
    len: 202,
    hash: 0x81d5_8206_5f7f_7fcb,
    calls: &[],
    imports: &[],
};

/// A big object's start (0x10001bb0, cdecl `(obj)`): the scale
/// `+0x130..+0x138` 3.0 (`+0x13c` 0), the rotation `+0x100..+0x10c` (90,
/// 0, 0, 0), the colour `+0x170..+0x17c` (1, 1, 1, 0.4); slot 490 `(*A,
/// obj, 2)` (`A` `+0x17`); the byte `+0x3d8` 0x48; slot 450 `(obj, F)`
/// (`+0xa8`). Returns slot 450's `eax`.
pub fn big_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let mut p = Pushes::default();
    p.push_all(&[2, obj, g.read::<u32>(w(0x17))]);
    for (off, v) in [
        (0x130, 0x4040_0000),
        (0x134, 0x4040_0000),
        (0x138, 0x4040_0000),
        (0x13c, 0),
        (0x100, 0x42b4_0000),
        (0x104, 0),
        (0x108, 0),
        (0x10c, 0),
        (0x170, 0x3f80_0000),
        (0x174, 0x3f80_0000),
        (0x178, 0x3f80_0000),
        (0x17c, 0x3ecc_cccd),
    ] {
        g.write::<u32>(obj + off, v);
    }
    call(g, img, f, &p, 0x04, 0x7a8 / 4);
    g.write::<u8>(obj + 0x3d8, 0x48);
    p.push_all(&[w(0xa8), obj]);
    Ret::eax(call(g, img, f, &p, 0x96, 0x708 / 4))
}

/// [`big_start`] (181 bytes, 1 overlay).
pub const BIG_START: Sig = Sig {
    len: 181,
    hash: 0xac96_e757_27b1_1547,
    calls: &[],
    imports: &[],
};
