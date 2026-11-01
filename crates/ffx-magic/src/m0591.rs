//! `magic_0591.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The effect's start on its cue (0x100044e0, an overlay table entry, no
/// arguments): nothing while slot 225 `(1)` answers non-zero, slot 5
/// `(0x10af)` answers non-zero, or slot 28 `(0x10af, 1)` answers non-zero;
/// else, once (the flag at operand `+0x50`), slot 226 `(*G_H, 1)`; then
/// when slot 227 `(*G_H)` answers 1 or more, the targets and tasks are
/// set up ([`targets_more_tasks`](crate::s5_misc::targets_more_tasks),
/// the `rel32` at `+0x94`) and the DLL's two callback words set (operands
/// `+0x9a`, `+0xa4`, to the immediates after them). Returns 0.
pub fn cue_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let one = |g: &mut G, t: u32, slot: u32, words: &[u32]| {
        let mut p = Pushes::default();
        p.push_all(words);
        let t = slot_target(g, w(t), slot);
        // call width: 12 (Pushes caps the words at 12)
        p.call(g, t, 0).eax
    };
    if one(g, 0x1, 225, &[1]) != 0 || one(g, 0x1b, 5, &[0x10af]) != 0 {
        return Ret::eax(0);
    }
    if one(g, 0x31, 28, &[1, 0x10af]) != 0 {
        return Ret::eax(0);
    }
    if g.read::<u32>(w(0x50)) == 0 {
        // `push eax` (1: slot 28 answered 0)
        let h = g.read::<u32>(w(0x5f));
        one(g, 0x59, 226, &[1, h]);
        g.write::<u32>(w(0x70), 1);
    }
    let h = g.read::<u32>(w(0x7f));
    if one(g, 0x79, 227, &[h]).cast_signed() >= 1 {
        let callee = Frame {
            at: img.rel32_target(f + 0x94),
            esp: fr.esp.wrapping_sub(4),
            eax: 0,
            ecx: 0,
            edx: 0,
            xmm: [0; 4],
        };
        crate::s5_misc::targets_more_tasks(g, img, &callee);
        g.write::<u32>(w(0x9a), w(0x9e));
        g.write::<u32>(w(0xa4), w(0xa8));
    }
    Ret::eax(0)
}

/// [`cue_start`] (175 bytes, 1 overlay).
pub const CUE_START: Sig = Sig {
    len: 175,
    hash: 0x5374_853b_674c_6690,
    calls: &[(0x94, Callee::Sig(&crate::s5_misc::TARGETS_MORE_TASKS))],
    imports: &[],
};

/// A scale task on the block's `+0x1a60` with the constants at operands
/// `+0x4` (`xmm2`) and `+0xc` (`xmm1`)
/// ([`scale_task_fixed`](crate::s4_small::scale_task_fixed), the `rel32`
/// at `+0x1f`; the block `*G_B`, operand `+0x13`), then the block's
/// `+field` 0.65. Returns the task's node.
fn scale_up<G: Game>(g: &mut G, img: &Image, fr: &Frame, field: u32) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let x2 = g.read::<u32>(w(0x4));
    let x1 = g.read::<u32>(w(0xc));
    let block = g.read::<u32>(w(0x13));
    // push esi; push ecx; call
    let callee = Frame {
        at: img.rel32_target(f + 0x1f),
        esp: fr.esp.wrapping_sub(0xc),
        eax: 0,
        ecx: block.wrapping_add(0x1a60),
        edx: 0,
        xmm: [0, u128::from(x1), u128::from(x2), 0],
    };
    let node = crate::s4_small::scale_task_fixed(g, img, &callee).eax;
    g.write::<u32>(block.wrapping_add(field), 0x3f26_6666);
    Ret::eax(node)
}

/// The scale task with `+0x1a74` set (0x10004a50, no arguments;
/// [`scale_up`]).
pub fn scale_up_a<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    scale_up(g, img, fr, 0x1a74)
}

/// [`scale_up_a`] (50 bytes, 1 overlay).
pub const SCALE_UP_A: Sig = Sig {
    len: 50,
    hash: 0x006a_c556_cae2_1243,
    calls: &[(0x1f, Callee::Sig(&crate::s4_small::SCALE_TASK_30))],
    imports: &[],
};

/// The scale task with `+0x1a64` set (0x10005110, no arguments;
/// [`scale_up`]; its two constants the other way round).
pub fn scale_up_b<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    scale_up(g, img, fr, 0x1a64)
}

/// [`scale_up_b`] (50 bytes, 1 overlay).
pub const SCALE_UP_B: Sig = Sig {
    len: 50,
    hash: 0x248a_6750_7747_49d3,
    calls: &[(0x1f, Callee::Sig(&crate::s4_small::SCALE_TASK_30))],
    imports: &[],
};
