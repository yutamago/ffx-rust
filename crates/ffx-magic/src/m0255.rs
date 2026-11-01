//! `magic_0255.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// The end both part draws share (operands from `at`, the first the
/// slot table's): slot 258 `(obj)`, slot 231 `(*D, *E, *C)`, slot 230
/// `(*B, *D)`, the pointer `*P` gets its block, and slot 712 `(obj,
/// 0x30)` with the game flag `*L` 1 around it. Returns 1 when slot 712
/// answers 0.
fn draw_end<G: Game>(g: &mut G, img: &Image, at: u32, obj: u32, p: &mut Pushes) -> u32 {
    let w = |off: u32| img.word(at + off);
    p.push(obj);
    let s = slot_target(g, w(0), 0x408 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    let c = g.read::<u32>(w(0xf));
    let e = g.read::<u32>(w(0x1a));
    let d = g.read::<u32>(w(0x26));
    p.push_all(&[c, e, d]);
    let s = slot_target(g, w(0x14), 0x39c / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    let d = g.read::<u32>(w(0x2e));
    let b = g.read::<u32>(w(0x39));
    p.push_all(&[d, b]);
    let s = slot_target(g, w(0x33), 0x398 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    let ptr = g.read::<u32>(w(0x46));
    g.write::<u32>(ptr, w(0x4e));
    p.push_all(&[0x30, obj]);
    let lock = g.read::<u32>(w(0x53));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let s = slot_target(g, w(0x61), 0xb20 / 4);
    // call width: 12 (Pushes caps the words at 12)
    let r = p.call(g, s, 0).eax;
    let lock = g.read::<u32>(w(0x6f));
    g.write::<u32>(lock, saved);
    u32::from(r == 0)
}

/// A part draw (0x100010a0, cdecl `(obj)`): when slot 261 `(2)` answers
/// non-zero the object's position `+0x40..+0x4c` becomes (7, 3, -61, 1);
/// then [`draw_end`]. Each slot call passes the words the original
/// leaves on the stack.
pub fn draw_placed<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let s = slot_target(g, img.word(f + 0x04), 0x414 / 4);
    let on = g.call(s, 0, &[2]).eax;
    let obj = fr.arg(g, 0);
    if on != 0 {
        for (off, v) in [
            (0x40, 0x40e0_0000),
            (0x44, 0x4040_0000),
            (0x48, 0xc274_0000),
            (0x4c, 0x3f80_0000),
        ] {
            g.write::<u32>(obj + off, v);
        }
    }
    Ret::eax(draw_end(g, img, f + 0x3b, obj, &mut Pushes::default()))
}

/// [`draw_placed`] (192 bytes, 1 overlay).
pub const DRAW_PLACED: Sig = Sig {
    len: 192,
    hash: 0x6102_4b17_07eb_39bb,
    calls: &[],
    imports: &[],
};

/// A part draw (0x10001230, cdecl `(obj)`): slot 237 `(*X, 0x20, 0,
/// obj + 0x40)` (`X` `+0x1d`) places the object, then [`draw_end`]. Each
/// slot call passes the words the original leaves on the stack.
pub fn draw_moved<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let mut p = Pushes::default();
    let x = g.read::<u32>(w(0x1d));
    p.push_all(&[obj + 0x40, 0, 0x20, x]);
    let s = slot_target(g, w(0x0d), 0x3b4 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    Ret::eax(draw_end(g, img, f + 0x24, obj, &mut p))
}

/// [`draw_moved`] (169 bytes, 1 overlay).
pub const DRAW_MOVED: Sig = Sig {
    len: 169,
    hash: 0x0a06_69ac_cf05_c8c9,
    calls: &[],
    imports: &[],
};
