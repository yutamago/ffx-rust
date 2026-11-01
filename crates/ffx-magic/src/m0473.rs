//! `magic_0473.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::o4_more::push_head;
use crate::sa_pool::take;
use crate::{Callee, Image, Mem, Sig};

/// `1.0f32`'s bits.
const ONE: u32 = 0x3f80_0000;

/// Starts a scaling object (0x10003c20, internal: `ecx` = the object,
/// `xmm1` and `xmm2` two scales): a scale that is not above 0 becomes
/// `K` (operand `+0x04`; a NaN stays); the two go to the globals at
/// `+0x27` and `+0x2f`, the object's `+0x0..+0x8` (the first, `+0xc`
/// 1.0) and `+0x10..+0x18` (the second, `+0x1c` 1.0); `+0x28` and
/// `+0x34` 0, `+0x2c` and `+0x30` 60. A pool node ([`take`], owner the
/// callback at `+0x1f`) holds the object (`+0x14`, byte `+0x18` 1, `+0x4`
/// 0) at the head of the list (`G` at `+0x81` 0: empty). Returns the
/// node.
pub fn scale_start(g: &mut impl Mem, img: &Image, f: u32, obj: u32, a: f32, b: f32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let k = g.read::<f32>(w(0x04));
    // comiss 0, x; jb: kept when above 0 or unordered
    let fix = |x: f32| if 0.0 < x || x.is_nan() { x } else { k };
    let (a, b) = (fix(a), fix(b));
    g.write::<f32>(w(0x27), a);
    g.write::<f32>(w(0x2f), b);
    for off in [8, 4, 0] {
        g.write::<f32>(obj + off, a);
    }
    g.write::<u32>(obj + 0xc, ONE);
    for off in [0x18, 0x14, 0x10] {
        g.write::<f32>(obj + off, b);
    }
    g.write::<u32>(obj + 0x1c, ONE);
    g.write::<u32>(obj + 0x28, 0);
    g.write::<u32>(obj + 0x34, 0);
    g.write::<u32>(obj + 0x30, 0x3c);
    g.write::<u32>(obj + 0x2c, 0x3c);
    let node = take(g, img, img.rel32_target(f + 0x7b), w(0x1f));
    let empty = g.read::<u32>(w(0x81)) == 0;
    g.write::<u32>(node.wrapping_add(0x14), obj);
    g.write::<u8>(node.wrapping_add(0x18), 1);
    g.write::<u32>(node.wrapping_add(4), 0);
    push_head(g, node, empty, (w(0x99), w(0xa3)), (w(0xb2), w(0xb7)));
    node
}

/// [`scale_start`] (191 bytes, 1 overlay).
pub const SCALE_START: Sig = Sig {
    len: 191,
    hash: 0x264a_f6eb_2f5e_63d5,
    calls: &[(0x7b, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};

/// Slot 239 `(*G_ARG, -5, 0)` (operands `+0x1`, `+0x11`), then the
/// objects freed ([`objects_freed`](crate::s6_misc::objects_freed), a
/// tail jump; 0x100020e0, no arguments). Returns its result.
pub fn end_objects<G: crate::Game>(g: &mut G, img: &Image, fr: &crate::Frame) -> crate::Ret {
    let w = |off: u32| img.word(fr.at + off);
    let t = crate::runtime::slot_target(g, w(0x1), 239);
    let a = g.read::<u32>(w(0x11));
    g.call(t, 0, &[a, 0xffff_fffb, 0]);
    let callee = crate::Frame {
        at: img.rel32_target(fr.at + 0x1b),
        ..*fr
    };
    crate::s6_misc::objects_freed(g, img, &callee)
}

/// [`end_objects`] (31 bytes, 1 overlay).
pub const END_OBJECTS: Sig = Sig {
    len: 31,
    hash: 0xa8d8_403c_6ae8_df20,
    calls: &[(0x1b, Callee::Sig(&crate::s6_misc::OBJECTS_FREED))],
    imports: &[],
};

/// The effect's per-frame task callback (0x10001740, cdecl `(node)`):
/// the script runs ([`run`](crate::sa_script::run)) and the motion steps
/// ([`motion_step`](crate::s3_motion::motion_step)); unless both say 1
/// the step's answer is returned. Else `*G_FX + 0x610` = 1; unless the
/// half-word at operand `+0x29` is set, slot 213 `(*G_M, 0)` (`+0x3f`);
/// the flag `+0x56` 1, the lights cleared and their node given back
/// ([`lights_and_give_back`](crate::s6_misc::lights_and_give_back),
/// `ecx` = `*G_FX + 0x380`), then the node given back
/// ([`give_back`](crate::sa_pool::give_back), a tail jump with `ecx` =
/// the node), whose `eax` it returns.
pub fn task<G: crate::Game>(g: &mut G, img: &Image, fr: &crate::Frame) -> crate::Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let fx = g.read::<u32>(w(0x7));
    let s = crate::sa_script::run(g, img, img.rel32_target(f + 0xc));
    let m = crate::s3_motion::motion_step(g, img, img.rel32_target(f + 0x13));
    if s != 1 || m != 1 {
        return crate::Ret::eax(m);
    }
    g.write::<u32>(fx + 0x610, 1);
    let fx = g.read::<u32>(w(0x30));
    if g.read::<u16>(w(0x29)) == 0 {
        let t = crate::runtime::slot_target(g, w(0x37), 213);
        let model = g.read::<u32>(w(0x3f));
        g.call(t, 0, &[model, 0]);
    }
    g.write::<u32>(w(0x56), 1);
    // push ebp, esi, edi, the return address
    let lights = crate::Frame {
        at: img.rel32_target(f + 0x5f),
        esp: fr.esp.wrapping_sub(16),
        ecx: fx.wrapping_add(0x380),
        ..*fr
    };
    crate::s6_misc::lights_and_give_back(g, img, &lights);
    let node = fr.arg(g, 0);
    crate::Ret::eax(crate::sa_pool::give_back(
        g,
        img,
        img.rel32_target(f + 0x6a),
        node,
    ))
}

/// [`task`] (114 bytes, 1 overlay).
pub const TASK: Sig = Sig {
    len: 114,
    hash: 0x060b_78b1_88c2_d493,
    calls: &[
        (0xc, Callee::Sig(&crate::sa_script::RUN)),
        (0x13, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0x5f, Callee::Sig(&crate::s6_misc::LIGHTS_AND_GIVE_BACK)),
        (0x6a, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
    ],
    imports: &[],
};
