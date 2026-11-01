//! `magic_0598.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::o4_more::push_head;
use crate::runtime::slot_target;
use crate::sa_pool::take;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Starts the fade-out (0x100077f0, an overlay table entry, no
/// arguments; the record `R = *G_R`, operand `+0x13`): a fade task on
/// `R + 0xb78` ([`fade_task`](crate::s3_misc::fade_task): `edx` 1,
/// `xmm1`/`xmm2` the constants at `+0x0c`/`+0x04`, the stack word 0);
/// the timer `P = R + 0xae0` (`+0x4` 100, `+0x10`/`+0x14` 150, the rest
/// 0) held by a pool node ([`take`], the owner the immediate at `+0x35`;
/// byte `+0x18` 1, `+0x4` 0) put at the head of the list ([`push_head`]:
/// the empty test `+0x6d`, head `+0x85`, set `+0x97`/`+0x9f`); slot 371
/// `(0, 0x10000)` and `(1, 0x8000)` (the first call's words under the
/// second's). Returns slot 371's `eax`.
pub fn fade_out<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let (x2, x1) = (g.read::<u32>(w(0x4)), g.read::<u32>(w(0xc)));
    let r = g.read::<u32>(w(0x13));
    // push esi; push 0; the call
    g.write::<u32>(fr.esp.wrapping_sub(8), 0);
    let callee = Frame {
        at: img.rel32_target(f + 0x25),
        esp: fr.esp.wrapping_sub(0xc),
        eax: fr.eax,
        ecx: r.wrapping_add(0xb78),
        edx: 1,
        xmm: [0, u128::from(x1), u128::from(x2), 0],
    };
    crate::s3_misc::fade_task(g, img, &callee);
    let p = r.wrapping_add(0xae0);
    for (off, v) in [
        (0, 0),
        (4, 0x64),
        (8, 0),
        (0xc, 0),
        (0x14, 0x96),
        (0x10, 0x96),
    ] {
        g.write::<u32>(p + off, v);
    }
    g.write::<u16>(p + 0x18, 0);
    let node = take(g, img, img.rel32_target(f + 0x67), w(0x35));
    let empty = g.read::<u32>(w(0x6d)) == 0;
    g.write::<u32>(node + 0x14, p);
    g.write::<u8>(node + 0x18, 1);
    g.write::<u32>(node + 4, 0);
    push_head(g, node, empty, (w(0x85), w(0x9f)), (w(0x97), w(0x9f)));
    let t = slot_target(g, w(0xa4), 371);
    g.call(t, 0, &[0, 0x1_0000]);
    let t = slot_target(g, w(0xb8), 371);
    Ret::eax(g.call(t, 0, &[1, 0x8000, 0, 0x1_0000]).eax)
}

/// [`fade_out`] (207 bytes, 1 overlay).
pub const FADE_OUT: Sig = Sig {
    len: 207,
    hash: 0xc5e3_d7bd_e80b_bffc,
    calls: &[
        (0x25, Callee::Sig(&crate::s3_misc::FADE_TASK)),
        (0x67, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// The effect's per-frame task callback (0x10006b90, cdecl `(node)`):
/// the 0x30-byte block `B` (operand `+0x10`) zeroed (`memset`), its
/// half-word `+2` (`+0x21`) gets bit 0x80, `+0x33` = `*G` (`+0x26`); the
/// two scripts (`+0x2e`, `+0x3d`) run
/// ([`script_run`](crate::sb_misc::script_run)). While the first runs
/// (0): the points `*G_FX + 0x920`, `+0x940`, `+0x960` copied to
/// `+0x880`, `+0x8c0`, `+0x900`, the three motion scripts (`+0xdb`,
/// `+0xeb`, `+0xf5`) step
/// ([`motion_script`](crate::s4_misc::motion_script)) and the camera
/// rigs step ([`camera_rigs_step`](crate::s8_misc::camera_rigs_step), a
/// tail jump). Once it is done: `+0xd3c` = 1 and the node given back
/// ([`give_back`](crate::sa_pool::give_back), a tail jump with `ecx` =
/// the node). Returns the tail jump's answer.
pub fn task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let fx = g.read::<u32>(w(0x7));
    g.write_bytes(w(0x10), &[0u8; 0x30]);
    let h = g.read::<u16>(w(0x21));
    g.write::<u16>(w(0x21), h | 0x80);
    let v = g.read::<u32>(w(0x26));
    g.write::<u32>(w(0x33), v);
    let first = crate::sb_misc::script_run(g, img, img.rel32_target(f + 0x38), w(0x2e));
    crate::sb_misc::script_run(g, img, img.rel32_target(f + 0x44), w(0x3d));
    if first != 0 {
        g.write::<u32>(fx + 0xd3c, 1);
        let node = fr.arg(g, 0);
        return Ret::eax(crate::sa_pool::give_back(
            g,
            img,
            img.rel32_target(f + 0x117),
            node,
        ));
    }
    for (from, to) in [(0x920, 0x880), (0x940, 0x8c0), (0x960, 0x900)] {
        for i in 0..4 {
            let v = g.read::<u32>(fx + from + 4 * i);
            g.write::<u32>(fx + to + 4 * i, v);
        }
    }
    for (op, call) in [(0xdb, 0xe6), (0xeb, 0xf0), (0xf5, 0xfa)] {
        crate::s4_misc::motion_script(g, img, img.rel32_target(f + call), w(op));
    }
    let rigs = Frame {
        at: img.rel32_target(f + 0x102),
        ..*fr
    };
    crate::s8_misc::camera_rigs_step(g, img, &rigs)
}

/// [`task`] (283 bytes, 1 overlay).
pub const TASK: Sig = Sig {
    len: 283,
    hash: 0xf3b4_d654_2d0c_fc5c,
    calls: &[
        (0x15, Callee::Import("memset")),
        (0x38, Callee::Sig(&crate::sb_misc::SCRIPT_RUN)),
        (0x44, Callee::Sig(&crate::sb_misc::SCRIPT_RUN)),
        (0xe6, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0xf0, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0xfa, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0x102, Callee::Sig(&crate::s8_misc::CAMERA_RIGS_STEP)),
        (0x117, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
    ],
    imports: &[],
};
