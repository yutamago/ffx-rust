//! `magic_0353.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The model's motion reset (0x10001ae0, no arguments, no frame): with
/// `m` = `*M` (`+0x08`): slots 357 `(m, -1, 0x3c00)` and 356 `(m, -1,
/// 0x2ee0)`, slot 362 `(*X, m, 1, 0)` (`X` `+0x43`), slots 357 and 356
/// `(m, -1, -1)`, then slot 399 `(*M, 1)`. Returns slot 399's `eax`. Each
/// slot call passes the words the original leaves on the stack.
pub fn motion_reset<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let m = g.read::<u32>(w(0x08));
    let mut p = Pushes::default();
    let calls: [(&[u32], u32, u32); 5] = [
        (&[0x3c00, u32::MAX, m], 0x01, 0x594),
        (&[0x2ee0, u32::MAX, m], 0x1d, 0x590),
        (&[0, 1, m], 0x32, 0x5a8),
        (&[u32::MAX, u32::MAX, m], 0x4a, 0x594),
        (&[u32::MAX, u32::MAX, m], 0x5c, 0x590),
    ];
    for (k, (words, table, slot)) in calls.into_iter().enumerate() {
        p.push_all(words);
        if k == 2 {
            p.push(g.read::<u32>(w(0x43)));
        }
        let s = slot_target(g, w(table), slot / 4);
        // call width: 12 (Pushes caps the words at 12)
        p.call(g, s, 0);
    }
    p.pop(16);
    p.push_all(&[1, g.read::<u32>(w(0x7f))]);
    let s = slot_target(g, w(0x6e), 0x63c / 4);
    // call width: 12 (Pushes caps the words at 12)
    Ret::eax(p.call(g, s, 0).eax)
}

/// [`motion_reset`] (138 bytes, 1 overlay).
pub const MOTION_RESET: Sig = Sig {
    len: 138,
    hash: 0x69e6_f43d_cac5_634d,
    calls: &[],
    imports: &[],
};

/// The fade-out step (0x10002300, cdecl `(node)`): the effect's (`*G`,
/// `+0x07`) countdown `+0x13e4`, when positive, drops; with `c` = it less
/// 80, slot 348 `(*X, *Y, 0, 48 c / 80, 16 c / 80, 0)` (`X` `+0x6b`, `Y`
/// `+0x5f`; the divisions truncate). While the countdown is still
/// positive returns slot 348's `eax`; else a tail jump gives the node
/// back to its pool ([`give_back`](crate::sa_pool::give_back)).
pub fn fade_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let fx = g.read::<u32>(w(0x07));
    let n = g.read::<i32>(fx + 0x13e4);
    if n > 0 {
        g.write::<i32>(fx + 0x13e4, n - 1);
    }
    let c = g.read::<i32>(fx + 0x13e4).wrapping_sub(0x50);
    let a = c.wrapping_mul(16) / 80;
    let b = c.wrapping_mul(48) / 80;
    let s = slot_target(g, w(0x57), 0x570 / 4);
    let x = g.read::<u32>(w(0x6b));
    let y = g.read::<u32>(w(0x5f));
    let eax = g
        .call(s, 0, &[x, y, 0, b.cast_unsigned(), a.cast_unsigned(), 0])
        .eax;
    if g.read::<i32>(fx + 0x13e4) > 0 {
        return Ret::eax(eax);
    }
    let node = fr.arg(g, 0);
    Ret::eax(crate::sa_pool::give_back(
        g,
        img,
        img.rel32_target(f + 0x84),
        node,
    ))
}

/// [`fade_step`] (138 bytes, 1 overlay).
pub const FADE_STEP: Sig = Sig {
    len: 138,
    hash: 0xc43e_a0e0_a54b_cf13,
    calls: &[(0x84, Callee::Sig(&crate::sa_pool::GIVE_BACK))],
    imports: &[],
};
