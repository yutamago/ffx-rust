//! `magic_0563.dll`'s own functions (besides the runtime and the shared
//! code). The effect block `*G_FX` (operand `+0x0d` of [`frame`] and its
//! neighbours) keeps the effect's state up to `+0x3ba8`.

use crate::runtime::slot_target;
use crate::sb_sse::sub;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The effect's per-frame step (0x100014a0, an overlay table entry, no
/// arguments): at frame 0x82 (the count at operand `+0x2`) the scene
/// light ([`scene_light`](crate::s5_misc::scene_light)); the objects run
/// ([`run_objects`](crate::l5_objhost::run_objects)); when they are done
/// and `*G_FX + 0x3ba8` is 1, slot 214 `(*G_M)` (`+0x31`) and the parts'
/// registration ([`register_parts`](crate::sa_misc::register_parts))
/// with its `eax`. The count goes up. Returns the last `eax`.
pub fn frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let fx = g.read::<u32>(w(0xd));
    // push esi, the return address
    let esp = fr.esp.wrapping_sub(8);
    if g.read::<u32>(w(0x2)) == 0x82 {
        let light = Frame {
            at: img.rel32_target(f + 0x14),
            esp,
            ..*fr
        };
        crate::s5_misc::scene_light(g, img, &light);
    }
    let mut eax = crate::l5_objhost::run_objects(g, img, img.rel32_target(f + 0x19), esp);
    if eax == 1 && g.read::<u32>(fx + 0x3ba8) == 1 {
        let t = slot_target(g, w(0x2b), 214);
        let m = g.read::<u32>(w(0x31));
        let r = g.call(t, 0, &[m]).eax;
        eax = crate::sa_misc::register_parts(g, img, img.rel32_target(f + 0x41), r);
    }
    let n = g.read::<u32>(w(0x47));
    g.write::<u32>(w(0x47), n.wrapping_add(1));
    Ret::eax(eax)
}

/// [`frame`] (77 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 77,
    hash: 0x8fcf_d601_cab9_dc91,
    calls: &[
        (0x14, Callee::Sig(&crate::s5_misc::SCENE_LIGHT)),
        (0x19, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
        (0x41, Callee::Sig(&crate::sa_misc::REGISTER_PARTS)),
    ],
    imports: &[],
};

/// Starts the colour scale (0x10002440, an overlay table entry, no
/// arguments): slot 371 `(0, 0x10000)` and `(1, 0x20000)`, then the
/// scale task ([`scale_task_fixed`](crate::s4_small::scale_task_fixed),
/// a tail jump: `ecx` = `*G_FX + 0x3864`, `xmm1`/`xmm2` the constants at
/// `+0x3a`/`+0x32`), whose node it returns.
pub fn scale_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let t = slot_target(g, w(0x1), 371);
    g.call(t, 0, &[0, 0x1_0000]);
    let t = slot_target(g, w(0x15), 371);
    g.call(t, 0, &[1, 0x2_0000, 0, 0x1_0000]);
    let fx = g.read::<u32>(w(0x2a));
    let mut xmm = fr.xmm;
    xmm[2] = u128::from(g.read::<u32>(w(0x32)));
    xmm[1] = u128::from(g.read::<u32>(w(0x3a)));
    let task = Frame {
        at: img.rel32_target(f + 0x48),
        ecx: fx.wrapping_add(0x3864),
        xmm,
        ..*fr
    };
    crate::s4_small::scale_task_fixed(g, img, &task)
}

/// [`scale_start`] (76 bytes, 1 overlay).
pub const SCALE_START: Sig = Sig {
    len: 76,
    hash: 0xcdad_a3b9_c558_2073,
    calls: &[(0x48, Callee::Sig(&crate::s4_small::SCALE_TASK_30))],
    imports: &[],
};

/// Starts the object `*G_FX` itself (0x100025a0, an overlay table entry,
/// no arguments): [`start_object_anim`](crate::s4_misc::start_object_anim)
/// (scale `*K` `+0x4`, index 0, 0); the point `+0x2b20` copied to its
/// `+0xd0` with `y - *K` (`+0x59`), its `+0xc4` = `+0x3ba4`, which it
/// returns.
pub fn object_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let scale = g.read::<f32>(w(0x4));
    let o = g.read::<u32>(w(0xb));
    crate::s4_misc::start_object_anim(g, img, img.rel32_target(f + 0x19), o, scale, 0, 0);
    for i in 0..4 {
        let v = g.read::<u32>(o + 0x2b20 + 4 * i);
        g.write::<u32>(o + 0xd0 + 4 * i, v);
    }
    let y = sub(g.read::<f32>(o + 0xd4), g.read::<f32>(w(0x59)));
    let a = g.read::<u32>(o + 0x3ba4);
    g.write::<u32>(o + 0xc4, a);
    g.write::<f32>(o + 0xd4, y);
    Ret::eax(a)
}

/// [`object_start`] (118 bytes, 1 overlay).
pub const OBJECT_START: Sig = Sig {
    len: 118,
    hash: 0xde44_0523_cafa_1835,
    calls: &[(0x19, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM))],
    imports: &[],
};

/// Starts the object pair `*G_FX + 0x2860` and `+0x2990` (0x10002660, an
/// overlay table entry, no arguments): each
/// [`start_object_anim`](crate::s4_misc::start_object_anim) (scale `*K`
/// `+0x4`/`+0x27`, index 1, 0) with the callback `+0x104` the immediate
/// (`+0x3e`, `+0x5a`) and `+0x120` 0x11 and 0x37. Returns the second
/// object.
pub fn pair_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let scale = g.read::<f32>(w(0x4));
    let o = g.read::<u32>(w(0xb)).wrapping_add(0x2860);
    crate::s4_misc::start_object_anim(g, img, img.rel32_target(f + 0x1f), o, scale, 1, 0);
    let scale = g.read::<f32>(w(0x27));
    g.write::<u32>(o + 0x104, w(0x3e));
    g.write::<u32>(o + 0x120, 0x11);
    let o2 = o.wrapping_add(0x130);
    let r = crate::s4_misc::start_object_anim(g, img, img.rel32_target(f + 0x4d), o2, scale, 1, 0);
    g.write::<u32>(o + 0x234, w(0x5a));
    g.write::<u32>(o + 0x250, 0x37);
    Ret::eax(r)
}

/// [`pair_start`] (106 bytes, 1 overlay).
pub const PAIR_START: Sig = Sig {
    len: 106,
    hash: 0x8b14_c84f_15e2_1857,
    calls: &[
        (0x1f, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x4d, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
    ],
    imports: &[],
};
