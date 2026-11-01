//! `magic_0421.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// The copies' start (0x10001300, cdecl `(obj)`): with `m` = `C[0x198]`
/// (`C` = `*A`, `+0x04`), `*N` (`+0x14`, read again each time) copies
/// (slot 434 `(*A, F)`, `F` `+0x21`) each get their number in their
/// current list word; then slot 444 `(0)`, slot 522 `(0, 0)`, slot 434
/// `(*A, F2)` (`+0x78`), slots 450 `(obj, F3)` and 449 `(obj, F4)`
/// (`+0x99`, `+0xa6`); the list word gets `m`. Returns twice the list
/// index. Each slot call passes the words the original leaves on the
/// stack.
pub fn copies_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let m = g.read::<u32>(g.read::<u32>(w(0x04)) + 0x198);
    let mut i = 0u32;
    while i.cast_signed() < g.read::<i32>(w(0x14)) {
        let a = g.read::<u32>(w(if i == 0 { 0x04 } else { 0x4c }));
        let s = slot_target(g, w(0x27), 0x6c8 / 4);
        let copy = g.call(s, 0, &[a, w(0x21)]).eax;
        let k = g.read::<u32>(copy + 0xac).wrapping_mul(2);
        g.write::<u32>(copy.wrapping_add(k.wrapping_mul(8)).wrapping_add(0x2c), i);
        i += 1;
    }
    let mut p = Pushes::default();
    p.push(0);
    let s = slot_target(g, w(0x53), 0x6f0 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    p.push_all(&[0, 0]);
    let s = slot_target(g, w(0x62), 0x828 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    p.push_all(&[w(0x78), g.read::<u32>(w(0x7e))]);
    let s = slot_target(g, w(0x73), 0x6c8 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    let obj = fr.arg(g, 0);
    p.push_all(&[w(0x99), obj]);
    let s = slot_target(g, w(0x8b), 0x708 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    p.push_all(&[w(0xa6), obj]);
    let s = slot_target(g, w(0xa1), 0x704 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    let k = g.read::<u32>(obj + 0xac).wrapping_mul(2);
    g.write::<u32>(obj.wrapping_add(k.wrapping_mul(8)).wrapping_add(0x2c), m);
    Ret::eax(k)
}

/// [`copies_start`] (198 bytes, 1 overlay).
pub const COPIES_START: Sig = Sig {
    len: 198,
    hash: 0xa29b_0d9b_fd8a_a9b8,
    calls: &[],
    imports: &[],
};
