//! `magic_0395.dll`'s own functions (besides the runtime, the object
//! host and [`crate::ob_summon`], which has its facing): the controller
//! vibration and the end's resets.

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// A callback (0x10004110, no arguments): the controller vibrates with
/// the effect's stream (slot 306 `(7, mode, slot 719 (E, 9, 0))`, mode 0
/// when `*G_VIBRATE` is set, else 1). Returns slot 306's `eax`.
pub fn vibrate<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let set = g.read::<u32>(w(0x02)) != 0;
    let s719 = slot_target(g, w(0x08), 719);
    let mut stack = vec![w(0x17), 9, 0];
    let e = g.call(s719, 0, &stack).eax;
    let (table, mode) = if set { (w(0x21), 0) } else { (w(0x39), 1) };
    let s306 = slot_target(g, table, 306);
    stack.splice(0..0, [7, mode, e]);
    Ret::eax(g.call(s306, 0, &stack).eax)
}

/// [`vibrate`] (77 bytes, 1 overlay).
pub const VIBRATE: Sig = Sig {
    len: 77,
    hash: 0x6634_e0fb_c497_9bfe,
    calls: &[],
    imports: &[],
};

/// The end's resets (0x100042c0, no arguments): the game unpaused
/// (slot 628 `(0)`), slot 933 `(1.0, 1.0, 1.0, 1.0)`, 1.0 into a data
/// word and slot 785 `(*G_SCALE)` twice (the same stack word carries each
/// call's argument). Returns slot 785's `eax`.
pub fn resets<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    const ONE: u32 = 0x3f80_0000;
    let w = |off: u32| img.word(f + off);
    let s628 = slot_target(g, w(0x01), 628);
    g.call(s628, 0, &[0]);
    let s933 = slot_target(g, w(0x10), 933);
    g.call(s933, 0, &[ONE, ONE, ONE, ONE]);
    let s785 = slot_target(g, w(0x3e), 785);
    let x = g.read::<u32>(w(0x46));
    g.write::<u32>(w(0x55), ONE);
    g.call(s785, 0, &[x]);
    let s785 = slot_target(g, w(0x65), 785);
    let x = g.read::<u32>(w(0x6d));
    g.call(s785, 0, &[x]).eax
}

/// [`resets`] (128 bytes, 1 overlay).
pub const RESETS: Sig = Sig {
    len: 128,
    hash: 0x78f1_f943_208a_582d,
    calls: &[],
    imports: &[],
};

/// A task's end (0x10004160, cdecl `(node)`): with `E` = `*G` (`+0x06`,
/// read first) the script runs ([`run`](crate::sa_script::run),
/// `+0x0c`); while the game is paused (slot 561, called through `ecx`)
/// its result is returned; else the motion steps
/// ([`motion_step`](crate::s3_motion::motion_step), `+0x25`) and when
/// neither is 0 `E + 0x1530` becomes 1, the end's [`resets`] run and the
/// node goes back to the pool ([`give_back`](crate::sa_pool::give_back),
/// a tail jump, `+0x47`). Returns the motion step's result otherwise.
pub fn script_end<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let e = g.read::<u32>(w(0x06));
    let a = crate::sa_script::run(g, img, img.rel32_target(f + 0x0c));
    let s = slot_target(g, w(0x12), 0x8c4 / 4);
    let paused = g.call(s, s, &[]).eax;
    if paused != 0 {
        return Ret::eax(paused);
    }
    let m = crate::s3_motion::motion_step(g, img, img.rel32_target(f + 0x25));
    if a == 0 || m == 0 {
        return Ret::eax(m);
    }
    g.write::<u32>(e + 0x1530, 1);
    resets(g, img, img.rel32_target(f + 0x3c));
    let node = fr.arg(g, 0);
    Ret::eax(crate::sa_pool::give_back(
        g,
        img,
        img.rel32_target(f + 0x47),
        node,
    ))
}

/// [`script_end`] (79 bytes, 1 overlay).
pub const SCRIPT_END: Sig = Sig {
    len: 79,
    hash: 0x8685_3f5f_4276_5fd2,
    calls: &[
        (0x0c, crate::Callee::Sig(&crate::sa_script::RUN)),
        (0x25, crate::Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0x3c, crate::Callee::Sig(&RESETS)),
        (0x47, crate::Callee::Sig(&crate::sa_pool::GIVE_BACK)),
    ],
    imports: &[],
};

/// The start-up (0x10004bc0, internal, no arguments): four block pointers set
/// (`*P` = the immediate, `+0x18`/`+0x1c` to `+0x36`/`+0x3a`), slot 719
/// `(T, 8, 0)` (`T` the immediate `+0x12`) into `+0x44`; the parts
/// ([`part_init`](crate::l5_objhost::part_init), `+0x49`) and the facing
/// ([`face`](crate::ob_summon::face), `+0x4e`); the callback word `+0x59`
/// = the immediate `+0x5d`, `+0x63` = 0, the halfword `+0x6e` = 0, the
/// byte `+0x76` = 1; slot 271 `(4)`; two pool nodes
/// ([`take`](crate::sa_pool::take), owners [`script_end`] `+0x89` and
/// `+0xc5`, kinds 1 and 2 at `+0x18`) go last on the task list (first
/// `+0x94`, last `+0xab`); three floats 1.0 (`+0x101`, `+0x111`,
/// `+0x11b`) and slot 790's float into `+0x127`. Returns 0. The slot calls
/// pass the words the original leaves on the stack.
pub fn start_up<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let s719 = slot_target(g, w(0x01), 0xb3c / 4);
    let mut p = crate::pushes::Pushes::default();
    p.push_all(&[0, 8, w(0x12)]);
    for (dst, v) in [(0x18, 0x1c), (0x22, 0x26), (0x2c, 0x30), (0x36, 0x3a)] {
        g.write::<u32>(w(dst), w(v));
    }
    // call width: 12 (Pushes caps the words at 12)
    let r = p.call(g, s719, 0).eax;
    p.pop(1);
    g.write::<u32>(w(0x44), r);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x49));
    crate::ob_summon::face(g, img, img.rel32_target(f + 0x4e));
    let s271 = slot_target(g, w(0x53), 0x43c / 4);
    g.write::<u32>(w(0x59), w(0x5d));
    g.write::<u32>(w(0x63), 0);
    g.write::<u16>(w(0x6e), 0);
    g.write::<u8>(w(0x76), 1);
    p.push(4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s271, 0);
    let first = crate::sa_pool::take(g, img, img.rel32_target(f + 0x8e), w(0x89));
    let head = g.read::<u32>(w(0x94));
    g.write::<u8>(first + 0x18, 1);
    g.write::<u32>(first + 4, 0);
    if head == 0 {
        g.write::<u32>(first, 0);
        g.write::<u32>(w(0xc0), first);
    } else {
        let last = g.read::<u32>(w(0xab));
        g.write::<u32>(first, last);
        g.write::<u32>(last + 4, first);
    }
    g.write::<u32>(first + 8, first);
    let second = crate::sa_pool::take(g, img, img.rel32_target(f + 0xcd), w(0xc5));
    g.write::<u8>(second + 0x18, 2);
    g.write::<u32>(second + 4, 0);
    // the list holds `first` by now: the empty-list branch never runs
    g.write::<u32>(second, first);
    g.write::<u32>(first + 4, second);
    g.write::<u32>(second + 8, second);
    g.write::<u32>(w(0xf6), second);
    let s790 = slot_target(g, w(0xfb), 0xc58 / 4);
    for off in [0x101, 0x111, 0x11b] {
        g.write::<u32>(w(off), 0x3f80_0000);
    }
    let v = crate::m0018::st0_f32(g.call(s790, 0, &[]).st0);
    g.write::<f32>(w(0x127), v);
    Ret::eax(0)
}

/// [`start_up`] (304 bytes, 1 overlay).
pub const START_UP: Sig = Sig {
    len: 304,
    hash: 0x2dc0_7bf6_78ef_4cce,
    calls: &[
        (0x49, crate::Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x4e, crate::Callee::Sig(&crate::ob_summon::FACE_0395)),
        (0x8e, crate::Callee::Sig(&crate::sa_pool::TAKE)),
        (0xcd, crate::Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};
