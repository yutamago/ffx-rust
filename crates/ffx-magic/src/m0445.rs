//! `magic_0445.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The effect's step (0x10006b00, cdecl `(task)`, `E` = the task's
/// `+4`): by the state `E + 0x2d0`: in state 0, from frame 90 on (`E +
/// 0x2cc`) each of the model's `n` nodes (`M` = `E + 0x2d4`, `n` at `M +
/// 0x4dc`, read again after each; the nodes 0x98 bytes apart) gets slot
/// 239 `(*X, node + 0x6c, 0x20)` (`X` `+0xb3`) and the state goes up; in
/// state 1 with bit 2 of `E + 0x108` the four nodes `E + 0x120`, `+0x14c`,
/// `+0x178`, `+0x2d8` leave their parents
/// ([`leave`](crate::sa_misc::leave)), the task's node leaves its list,
/// the frame count goes up and the state becomes 0xff. The frame count
/// goes up in every other case too. Returns the last value in `eax` (`M`,
/// the node's old previous link, or the state less 1).
pub fn step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let task = fr.arg(g, 0);
    let e = g.read::<u32>(task + 4);
    let state = g.read::<u32>(e + 0x2d0);
    let count = e + 0x2cc;
    let bump = |g: &mut G| {
        let n = g.read::<u32>(count);
        g.write::<u32>(count, n.wrapping_add(1));
    };
    match state {
        0 => {
            if g.read::<i32>(count) < 0x5a {
                bump(g);
                return Ret::eax(0);
            }
            let mut m = g.read::<u32>(e + 0x2d4);
            let mut i = 0i32;
            let mut node = 0u32;
            while i < g.read::<i32>(m + 0x4dc) {
                let s = slot_target(g, w(0xad), 0x3bc / 4);
                let at = g.read::<u32>(m.wrapping_add(node).wrapping_add(0x6c));
                let x = g.read::<u32>(w(0xb3));
                g.call(s, 0, &[x, at, 0x20]);
                m = g.read::<u32>(e + 0x2d4);
                i += 1;
                node = node.wrapping_add(0x98);
            }
            let st = g.read::<u32>(e + 0x2d0);
            g.write::<u32>(e + 0x2d0, st.wrapping_add(1));
            bump(g);
            Ret::eax(m)
        }
        1 if g.read::<u8>(e + 0x108) & 4 != 0 => {
            let mut eax = 0;
            for off in [0x120, 0x14c, 0x178, 0x2d8] {
                eax = crate::sa_misc::leave(g, e + off, eax);
            }
            let node = g.read::<u32>(task);
            let prev = g.read::<u32>(node + 4);
            let next = g.read::<u32>(node + 8);
            g.write::<u32>(prev + 8, next);
            let next = g.read::<u32>(node + 8);
            let prev = g.read::<u32>(node + 4);
            g.write::<u32>(next + 4, prev);
            g.write::<u32>(node + 8, node);
            g.write::<u32>(node + 4, node);
            bump(g);
            g.write::<u32>(e + 0x2d0, 0xff);
            Ret::eax(prev)
        }
        _ => {
            bump(g);
            Ret::eax(state.wrapping_sub(1))
        }
    }
}

/// [`step`] (232 bytes, 1 overlay).
pub const STEP: Sig = Sig {
    len: 232,
    hash: 0x2ad9_6028_3228_1bef,
    calls: &[
        (0x31, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x3c, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x47, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x52, Callee::Sig(&crate::sa_misc::LEAVE)),
    ],
    imports: &[],
};
