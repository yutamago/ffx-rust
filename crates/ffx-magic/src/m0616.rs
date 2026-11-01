//! `magic_0616.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The effect's frame (0x100013b0, an overlay table entry, no
/// arguments): 0 while the flag at operand `+0x02` is clear. Else `*G_P
/// = ITEM` (`+0x0d`, `+0x19`), slots 723 `(*G_A)` and 724 `(*G_B)`, `*G_V
/// = *G_W` (`+0x3f`, `+0x44`), slot 261 `(1)` (each call seeing the
/// earlier words). When 261 says 0, or the timeline still runs
/// ([`timeline`](crate::sa_misc::timeline) nonzero), the frame count
/// `+0x8c` goes up; else the timeline stops
/// ([`timeline_stop_global`](crate::sb_misc::timeline_stop_global)) and
/// slot 214 `(*G_E)` (`+0x75`) runs. Returns 1.
pub fn frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    if g.read::<u32>(w(0x2)) == 0 {
        return Ret::eax(0);
    }
    let gp = g.read::<u32>(w(0xd));
    let a = g.read::<u32>(w(0x13));
    g.write::<u32>(gp, w(0x19));
    let t = slot_target(g, w(0x1e), 723);
    g.call(t, 0, &[a]);
    let b = g.read::<u32>(w(0x31));
    let t = slot_target(g, w(0x2b), 724);
    g.call(t, 0, &[b, a]);
    let v = g.read::<u32>(w(0x3f));
    let x = g.read::<u32>(w(0x44));
    g.write::<u32>(v, x);
    let t = slot_target(g, w(0x4b), 261);
    let stop = g.call(t, 0, &[1, b, a]).eax != 0
        && crate::sa_misc::timeline(g, img, img.rel32_target(f + 0x61)) == 0;
    if !stop {
        let n = w(0x8c);
        g.write::<u32>(n, g.read::<u32>(n).wrapping_add(1));
        return Ret::eax(1);
    }
    crate::sb_misc::timeline_stop_global(g, img, img.rel32_target(f + 0x6a));
    let t = slot_target(g, w(0x6f), 214);
    let e = g.read::<u32>(w(0x75));
    g.call(t, 0, &[e]);
    Ret::eax(1)
}

/// [`frame`] (150 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 150,
    hash: 0x0598_9357_1c41_64d2,
    calls: &[
        (0x61, Callee::Sig(&crate::sa_misc::TIMELINE)),
        (0x6a, Callee::Sig(&crate::sb_misc::TIMELINE_STOP_GLOBAL)),
    ],
    imports: &[],
};

/// The effect's start-up (0x10001160, an overlay table entry, `(arg)`):
/// the common head ([`start_up_head`](crate::part_draw::start_up_head)),
/// the timeline's callback (`+0x13c`/`+0x140`) and the three flags 1;
/// slot 260 `(5)` (the head's words left under it) into `+0x170`.
/// Returns 1.
pub fn start_up<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let mut stack = crate::part_draw::start_up_head(g, img, fr);
    g.write::<u32>(w(0x13c), w(0x140));
    for op in [0x146, 0x150, 0x15a] {
        g.write::<u32>(w(op), 1);
    }
    let t = slot_target(g, w(0x136), 260);
    stack.insert(0, 5);
    // call width: 7 (the head's words stay on the stack)
    let eax = g.call(t, 0, &stack).eax;
    g.write::<u32>(w(0x170), eax);
    Ret::eax(1)
}

/// [`start_up`] (380 bytes, 1 overlay).
pub const START_UP: Sig = Sig {
    len: 380,
    hash: 0x5652_9a6c_06f0_ed7c,
    calls: &[(0x131, Callee::Import("memset"))],
    imports: &[],
};
