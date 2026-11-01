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

/// The midpoint of two bones (0x10001400, cdecl `(a)`, a /GS frame):
/// slot 237 `(*G_M, 0x13, 0, &A)` and `(*G_M, 0x23, 0, &B)` (operands
/// `+0x29`/`+0x44`; `A`, `B` locals at `ebp - 0x14`, `- 0x24`), `A =
/// ((A + B) K, 1)` (`K` at `+0x58`), then slot 229 `(a + 0xa0, M, &A)`
/// (`M` the immediate `+0x6b`; each call over the earlier words). Returns
/// slot 229's `eax`.
pub fn bone_midpoint<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, mul};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let cookie = g.read::<u32>(w(0x7)) ^ ebp;
    g.write::<u32>(ebp.wrapping_sub(4), cookie);
    let a = fr.arg(g, 0);
    let (pa, pb) = (ebp.wrapping_sub(0x14), ebp.wrapping_sub(0x24));
    let mut s = Pushes::default();
    s.push_all(&[pa, 0, 0x13, g.read::<u32>(w(0x29))]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x19), 237), fr.ecx);
    s.push_all(&[pb, 0, 0x23, g.read::<u32>(w(0x44))]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x34), 237), fr.ecx);
    let k = g.read::<f32>(w(0x58));
    for i in 0..3u32 {
        let v = mul(add(g.read::<f32>(pa + 4 * i), g.read::<f32>(pb + 4 * i)), k);
        g.write::<f32>(pa + 4 * i, v);
    }
    g.write::<u32>(pa + 0xc, 0x3f80_0000);
    s.push_all(&[pa, w(0x6b), a + 0xa0]);
    // call width: 12 (Pushes caps the words at 12)
    Ret::eax(s.call(g, slot_target(g, w(0x80), 229), fr.ecx).eax)
}

/// [`bone_midpoint`] (193 bytes, 1 overlay; the `rel32` at `+0xb9` is
/// `__security_check_cookie`).
pub const BONE_MIDPOINT: Sig = Sig {
    len: 193,
    hash: 0x2cee_3bf6_5033_f155,
    calls: &[(0xb9, crate::Callee::Any)],
    imports: &[],
};
