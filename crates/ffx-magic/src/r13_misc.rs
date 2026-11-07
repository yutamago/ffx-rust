//! Round 13 (magic-r13): small own functions and parameter families
//! (`~/ffx-work/m5/magic/r4/pfamilies.tsv`), one port per family reading
//! its parameters from each member's code.

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// An object's task listed (internal: `ecx` = the object `O`; family
/// P055, `magic_0610` 0x10004670 and `magic_0673` 0x100043a0, 112 bytes):
/// `O + 0x108` and `+0x104` = the copy's time (the immediates `+0x10`,
/// `+0x1a`), `+0x10c` = the immediate `+0x24`, u16 `+0x126` = 0; a pool
/// node ([`take`](crate::sa_pool::take), the owner `+0x06`) gets `+0x14 =
/// O` and is appended to the task list (first `+0x36`, last `+0x4a`).
/// Returns the node.
pub fn object_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let o = fr.ecx;
    g.write::<u32>(o.wrapping_add(0x108), w(0x10));
    g.write::<u32>(o.wrapping_add(0x104), w(0x1a));
    g.write::<u32>(o.wrapping_add(0x10c), w(0x24));
    g.write::<u16>(o.wrapping_add(0x126), 0);
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x30), w(0x06));
    let first = g.read::<u32>(w(0x36));
    g.write::<u32>(node.wrapping_add(0x14), o);
    g.write::<u32>(node.wrapping_add(4), 0);
    if first == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x63), node);
        g.write::<u32>(w(0x68), node);
    } else {
        let last = g.read::<u32>(w(0x4a));
        g.write::<u32>(node, last);
        g.write::<u32>(last.wrapping_add(4), node);
        g.write::<u32>(w(0x54), node);
    }
    g.write::<u32>(node.wrapping_add(8), node);
    Ret::eax(node)
}

/// [`object_task`] in `magic_0610` (0x10004670; the `rel32`: the pool's
/// take).
pub const OBJECT_TASK_0610: Sig = Sig {
    len: 112,
    hash: 0x7311_c261_c09a_0e5c,
    calls: &[(0x30, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};

/// [`object_task`] in `magic_0673` (0x100043a0, its time 0x82).
pub const OBJECT_TASK_0673: Sig = Sig {
    len: 112,
    hash: 0x9280_7162_a54d_d6b8,
    calls: &[(0x30, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};

/// The three object lists run and, when all of them and the scene word
/// are done, the sound stopped (a pool task, no arguments; family P057,
/// `magic_0304` 0x10001560 and `magic_0568` 0x10001500, 100 bytes): the
/// lists `L1`, `L2`, `L3` (operands `+0x10`, `+0x1c`, `+0x2a`) run
/// ([`host_objects_run`](crate::s9_misc::host_objects_run), `edx` = `*G_T`
/// (`+0x02`) for the first, 0 after); when each answers 1 and `S[D]` is 1
/// (`S = *G_S`, `+0x0b`; `D` the displacement `+0x44`), slot 214 `(*G_SND)`
/// (`+0x51`). Returns the last call's `eax`.
pub fn lists_done<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let s = g.read::<u32>(w(0x0b));
    // push ebx; push esi; push edi: each list's run sees `esp` below them
    let esp = fr.esp.wrapping_sub(0x10);
    let run = |g: &mut G, call: u32, list: u32, edx: u32| {
        let callee = Frame {
            at: img.rel32_target(f + call),
            esp,
            ecx: list,
            edx,
            ..*fr
        };
        crate::s9_misc::host_objects_run(g, img, &callee).eax
    };
    let t = g.read::<u32>(w(0x02));
    let a = run(g, 0x15, w(0x10), t);
    let b = run(g, 0x23, w(0x1c), 0);
    let c = run(g, 0x31, w(0x2a), 0);
    if a != 1 || b != a || c != a || g.read::<u32>(s.wrapping_add(w(0x44))) != a {
        return Ret::eax(c);
    }
    let snd = g.read::<u32>(w(0x51));
    let t = slot_target(g, w(0x4b), 0x358 / 4);
    // call width: 1
    Ret::eax(g.call(t, 0, &[snd]).eax)
}

/// [`lists_done`] in `magic_0304` (0x10001560; the `rel32`s: the lists'
/// run three times).
pub const LISTS_DONE_0304: Sig = Sig {
    len: 100,
    hash: 0xb612_480d_33ec_af79,
    calls: &[
        (0x15, Callee::Sig(&crate::s9_misc::HOST_OBJECTS_RUN)),
        (0x23, Callee::Sig(&crate::s9_misc::HOST_OBJECTS_RUN)),
        (0x31, Callee::Sig(&crate::s9_misc::HOST_OBJECTS_RUN)),
    ],
    imports: &[],
};

/// [`lists_done`] in `magic_0568` (0x10001500, its scene word at
/// `+0x16c8`).
pub const LISTS_DONE_0568: Sig = Sig {
    len: 100,
    hash: 0xd369_f253_2ddd_70ca,
    calls: &[
        (0x15, Callee::Sig(&crate::s9_misc::HOST_OBJECTS_RUN)),
        (0x23, Callee::Sig(&crate::s9_misc::HOST_OBJECTS_RUN)),
        (0x31, Callee::Sig(&crate::s9_misc::HOST_OBJECTS_RUN)),
    ],
    imports: &[],
};
