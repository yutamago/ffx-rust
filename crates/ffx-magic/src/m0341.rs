//! `magic_0341.dll`'s own functions (besides the runtime, the object
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

/// The start-up (0x10001170, cdecl `(arg)`): `arg` kept (`+0x07`); slot
/// 803 `(0x100000, 0x10)` gives a buffer (`+0x25`), slot 861 `(buf, NAME,
/// 0x60b00)` (`NAME` `+0x2a`); slot 259 `()`; slot 260 `(7)` and `(0x14)`
/// into `+0x59` and `+0x6d`; for the effect `NAME` slots 718 `(NAME)`,
/// 222 and 221 `(NAME, 0, 1000, 0x100)`; seven words cleared. Returns 0.
pub fn start_up<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    g.write::<u32>(w(0x07), fr.arg(g, 0));
    let mut p = Pushes::default();
    p.push_all(&[0x10, 0x10_0000]);
    let buf = call(g, img, f, &p, 0x0c, 0xc8c / 4);
    p.push(0x60b00);
    g.write::<u32>(w(0x25), buf);
    p.push_all(&[w(0x2a), buf]);
    call(g, img, f, &p, 0x30, 0xd74 / 4);
    call(g, img, f, &p, 0x3d, 0x40c / 4);
    p.push(7);
    let v = call(g, img, f, &p, 0x4a, 0x410 / 4);
    g.write::<u32>(w(0x59), v);
    p.push(0x14);
    let v = call(g, img, f, &p, 0x5e, 0x410 / 4);
    g.write::<u32>(w(0x6d), v);
    p.push(w(0x77));
    call(g, img, f, &p, 0x72, 0xb38 / 4);
    p.push_all(&[0x100, 1000, 0, w(0x9b)]);
    call(g, img, f, &p, 0x84, 0x378 / 4);
    p.push_all(&[0x100, 1000, 0, w(0xb9)]);
    call(g, img, f, &p, 0xa2, 0x374 / 4);
    for at in [0xc4, 0xce, 0xd8, 0xe2, 0xec, 0xf6, 0x100] {
        g.write::<u32>(w(at), 0);
    }
    Ret::eax(0)
}

/// [`start_up`] (268 bytes, 1 overlay).
pub const START_UP: Sig = Sig {
    len: 268,
    hash: 0x1985_b875_eb9d_1e8b,
    calls: &[],
    imports: &[],
};

/// The parts freed (0x10001280, no arguments, no frame): six parts
/// (`+0x06`, `+0x23`, `+0x40`, `+0x5d`, `+0x7a`, `+0x97`) each freed by
/// slot 713 with the pointer `*P` (`+0x01`) set to its block first; then
/// slot 804 `(*B)` frees the buffer (`B` `+0xb5`) and `*B` is cleared.
/// Returns 0.
pub fn free_parts<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let mut p = Pushes::default();
    for (ptr, part, block, table) in [
        (0x01, 0x06, 0x0c, 0x11),
        (0x1e, 0x23, 0x29, 0x2e),
        (0x3b, 0x40, 0x46, 0x4b),
        (0x58, 0x5d, 0x63, 0x68),
        (0x75, 0x7a, 0x80, 0x85),
        (0x92, 0x97, 0x9d, 0xa2),
    ] {
        let at = g.read::<u32>(w(ptr));
        p.push(w(part));
        g.write::<u32>(at, w(block));
        call(g, img, f, &p, table, 0xb24 / 4);
    }
    p.push(g.read::<u32>(w(0xb5)));
    call(g, img, f, &p, 0xaf, 0xc90 / 4);
    g.write::<u32>(w(0xc6), 0);
    Ret::eax(0)
}

/// [`free_parts`] (209 bytes, 1 overlay).
pub const FREE_PARTS: Sig = Sig {
    len: 209,
    hash: 0x021f_06d5_2aee_c5bb,
    calls: &[],
    imports: &[],
};
