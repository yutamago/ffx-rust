//! `magic_0564.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Sig};

/// Hides or fades an actor (0x10001b40, internal: `ecx` = an index into
/// the actor bytes `b = *G_B`, -1 for the effect's own actor `*G_A`):
/// unless the flag at operand `+0x02` is set, an actor number below 32
/// (`b[i]`, or `*G_A`) goes to slot 296 `(n, 0)`; otherwise slot 257
/// `(model, 1)` with the effect's model (`+0x3c`) or the actor's
/// (`T[b[i]]`, `T` at `+0x5d`). Returns the slot's `eax`.
pub fn hide<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let w = |off: u32| img.word(fr.at + off);
    let i = fr.ecx;
    let b = g.read::<u32>(w(0x9));
    if g.read::<u32>(w(0x2)) == 0 {
        let n = if i == u32::MAX {
            g.read::<u32>(w(0x10))
        } else {
            u32::from(g.read::<u8>(b.wrapping_add(i)))
        };
        if n.cast_signed() < 0x20 {
            let t = slot_target(g, w(0x26), 296);
            return g.call(t, 0, &[n, 0]).eax;
        }
    }
    if i == u32::MAX {
        let t = slot_target(g, w(0x44), 257);
        let m = g.read::<u32>(w(0x3c));
        return g.call(t, 0, &[m, 1]).eax;
    }
    let k = u32::from(g.read::<u8>(b.wrapping_add(i)));
    let m = g.read::<u32>(w(0x5d).wrapping_add(4 * k));
    let t = slot_target(g, w(0x63), 257);
    g.call(t, 0, &[m, 1]).eax
}

/// [`hide`] (115 bytes, 1 overlay).
pub const HIDE: Sig = Sig {
    len: 115,
    hash: 0x5c9d_6a63_8604_5dce,
    calls: &[],
    imports: &[],
};

/// The effect's per-frame task callback (0x10001990, cdecl `(node)`):
/// each of the three hide counts `C[i]` (operand `+0x12`) above 0 steps
/// down, and reaching 0 hides actor `A[i]` ([`hide`], `ecx` = the word
/// table `+0x27`); the three motion scripts (`+0x39`, `+0x43`, `+0x4f`;
/// [`motion_script`](crate::s4_misc::motion_script)) step and the
/// objects run ([`run_objects`](crate::l5_objhost::run_objects)). When
/// all four say 1 and no count is above 0, the node's `+0x14` = 3 and
/// its task's `+0x10` the callback (the immediate `+0x9d`), returning
/// the task; else the objects' answer.
pub fn task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> crate::Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    // push ebp, ebx, esi, edi, the return address
    let esp = fr.esp.wrapping_sub(0x14);
    for i in 0..3u32 {
        let c = g.read::<u32>(w(0x12) + 4 * i).cast_signed();
        if c > 0 {
            g.write::<u32>(w(0x1d) + 4 * i, (c - 1).cast_unsigned());
            if c - 1 <= 0 {
                let a = g.read::<u32>(w(0x27) + 4 * i);
                let callee = Frame {
                    at: img.rel32_target(f + 0x2c),
                    esp,
                    ecx: a,
                    ..*fr
                };
                hide(g, img, &callee);
            }
        }
    }
    let mut done = [0u32; 3];
    for (d, (op, call)) in done
        .iter_mut()
        .zip([(0x39, 0x3e), (0x43, 0x4a), (0x4f, 0x56)])
    {
        *d = crate::s4_misc::motion_script(g, img, img.rel32_target(f + call), w(op));
    }
    let r = crate::l5_objhost::run_objects(g, img, img.rel32_target(f + 0x5d), esp);
    if r != 1
        || done.iter().any(|&d| d != 1)
        || [0x74, 0x7d, 0x86]
            .iter()
            .any(|&op| g.read::<u32>(w(op)).cast_signed() > 0)
    {
        return crate::Ret::eax(r);
    }
    let node = fr.arg(g, 0);
    g.write::<u32>(node.wrapping_add(0x14), 3);
    let task = g.read::<u32>(node.wrapping_add(8));
    g.write::<u32>(task.wrapping_add(0x10), w(0x9d));
    crate::Ret::eax(task)
}

/// [`task`] (166 bytes, 1 overlay).
pub const TASK: Sig = Sig {
    len: 166,
    hash: 0x6a95_2a57_09c0_1495,
    calls: &[
        (0x2c, crate::Callee::Sig(&HIDE)),
        (0x3e, crate::Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0x4a, crate::Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0x56, crate::Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0x5d, crate::Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
    ],
    imports: &[],
};
