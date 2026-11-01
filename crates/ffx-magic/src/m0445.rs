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

/// A fading node handler (0x100034c0, `ecx` = the cursor; reached
/// through its `ecx` thunk): the walk (each child of the cursor's node
/// gets its method `+0x1c` with the cursor; then the node takes the
/// pass); when the cursor moved, with a count `node[0x54]`: slot 252
/// `(*G_H, colour, alpha, 0x80)` (`*G_H` operand `+0x82`; the answer back
/// into it, `+0x9e`), and once `node[0x4c]` is not below `K` (`+0x99`)
/// slot 234 `(node + 0x1c, node + 0x2c, node + 0x3c)`, the count down and `node[0x4c]` 0; with none, bit 0 of
/// `node[0x58]` makes the node leave its parent
/// ([`leave`](crate::sa_misc::leave)). Returns the last call's `eax`,
/// else what the walk left (the low byte a pass byte).
///
/// The colour is built from four stack words the original never writes
/// (`ebp - 0x8`, `- 0xc`, `- 0x10`, `- 0x14`) and the alpha word has
/// only its low byte written (`ebp - 4`, `node[0x5a]`).
pub fn fade_visit<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let cur = fr.ecx;
    let ebp = fr.esp.wrapping_sub(4);
    let node = g.read::<u32>(cur);
    let pass = g.read::<u8>(node + 0x1a);
    let mut eax = (fr.eax & !0xff) | u32::from(pass);
    if pass == g.read::<u8>(cur + 5) {
        g.write::<u8>(cur + 4, 0);
    } else {
        let mut child = g.read::<u32>(node + 0x10);
        while child != node {
            g.write::<u32>(cur, child);
            let c = g.read::<u32>(cur);
            child = g.read::<u32>(child + 8);
            let m = g.read::<u32>(g.read::<u32>(c) + 0x1c);
            eax = g.call(m, cur, &[cur]).eax;
        }
        let pass = g.read::<u8>(cur + 5);
        g.write::<u32>(cur, node);
        g.write::<u8>(node + 0x1a, pass);
        g.write::<u8>(cur + 4, 1);
        eax = (eax & !0xff) | u32::from(pass);
    }
    if g.read::<u8>(cur + 4) & 1 == 0 {
        return Ret::eax(eax);
    }
    let n = g.read::<u32>(cur);
    if g.read::<u32>(n + 0x54) == 0 {
        if g.read::<u8>(n + 0x58) & 1 == 0 {
            return Ret::eax(eax);
        }
        return Ret::eax(crate::sa_misc::leave(g, n, eax));
    }
    // junk read: [ebp-0x4]'s upper bytes never written; 0 (brief 2026-10-24)
    let alpha = u32::from(g.read::<u8>(n + 0x5a));
    g.write::<u8>(ebp.wrapping_sub(4), g.read::<u8>(n + 0x5a));
    // junk read: [ebp-0x8], [ebp-0xc], [ebp-0x10], [ebp-0x14] never
    // written; 0 (brief 2026-10-24)
    let colour = 0;
    let h = g.read::<u32>(w(0x82));
    let t = slot_target(g, w(0x7c), 252);
    let eax = g.call(t, 0, &[h, colour, alpha, 0x80]).eax;
    g.write::<u32>(w(0x9e), eax);
    // comiss x, K; jb: below or unordered
    let reached = g.read::<f32>(n + 0x4c) >= g.read::<f32>(w(0x99));
    if !reached {
        return Ret::eax(eax);
    }
    let t = slot_target(g, w(0xb1), 234);
    let eax = g.call(t, 0, &[n + 0x1c, n + 0x2c, n + 0x3c]).eax;
    let c = g.read::<u32>(n + 0x54);
    g.write::<u32>(n + 0x54, c.wrapping_sub(1));
    g.write::<u32>(n + 0x4c, 0);
    Ret::eax(eax)
}

/// [`fade_visit`] (227 bytes, 1 overlay).
pub const FADE_VISIT: Sig = Sig {
    len: 227,
    hash: 0x75c4_73d1_b3c6_a2e2,
    calls: &[(0xd9, Callee::Sig(&crate::sa_misc::LEAVE))],
    imports: &[],
};
