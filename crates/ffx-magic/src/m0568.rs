//! `magic_0568.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::sb_sse::add;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Sways the summon's three heights (0x10001c30, internal, no
/// arguments): the time `t = S[0x16c4]` (`S = *G_S`) samples the DLL's
/// five-key table ([`ease_sample`](crate::sb_misc::ease_sample), stride 1
/// pushed) and steps by `K`; `S[0x464]`, `S[0x6c4]` and `S[0x142c]` =
/// `S[0x143c] + K_i + sample` (the base re-read each time). Returns the
/// sample's `eax` (scratch no caller reads).
pub fn sway<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let s = g.read::<u32>(w(0x3));
    let t = g.read::<f32>(s + 0x16c4);
    // push esi; push 1; call: the stride word at entry - 8
    let esp = fr.esp.wrapping_sub(0xc);
    g.write::<u32>(esp + 4, 1);
    let x = crate::sb_misc::ease_sample(g, img, img.rel32_target(f + 0x1f), esp, w(0x17), 5, t);
    let t = add(t, g.read::<f32>(w(0x27)));
    g.write::<f32>(s + 0x16c4, t);
    for (k, dst) in [(0x45u32, 0x464u32), (0x61, 0x6c4), (0x7d, 0x142c)] {
        let v = add(add(g.read::<f32>(s + 0x143c), g.read::<f32>(w(k))), x);
        g.write::<f32>(s + dst, v);
    }
    Ret::eax(0)
}

/// [`sway`] (143 bytes, 1 overlay).
pub const SWAY: Sig = Sig {
    len: 143,
    hash: 0x9dfe_6c0a_73b2_ae66,
    calls: &[(0x1f, Callee::Sig(&crate::sb_misc::EASE_SAMPLE))],
    imports: &[],
};

/// The effect's per-frame task callback (0x10001440, cdecl `(node)`):
/// the 0x30-byte block `B` (operand `+0x10`) zeroed (`memset`), its
/// half-word `+2` (`+0x21`) gets bit 0x80, `+0x33` = `*G` (`+0x26`); the
/// two scripts (`+0x2e`, `+0x3d`) run
/// ([`script_run`](crate::sb_misc::script_run)). While the first one
/// runs (0): the point `*G_FX + 0x1428` copied to `+0x1408`, the motion
/// steps ([`motion_step`](crate::s3_motion::motion_step)) and the camera
/// rigs step ([`camera_rigs_step`](crate::s8_misc::camera_rigs_step), a
/// tail jump). Once it is done: `+0x16c8` = 1, the node `*G_N` (`+0x8b`),
/// when there is one, given back
/// ([`give_back`](crate::sa_pool::give_back)) and `G_N` 0, the parts
/// registered ([`register_parts`](crate::sa_misc::register_parts)) and
/// the node given back (a tail jump with `ecx` = the node), whose `eax`
/// it returns.
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
    let mut eax = crate::sb_misc::script_run(g, img, img.rel32_target(f + 0x44), w(0x3d));
    if first == 0 {
        for i in 0..4 {
            let v = g.read::<u32>(fx + 0x1428 + 4 * i);
            g.write::<u32>(fx + 0x1408 + 4 * i, v);
        }
        crate::s3_motion::motion_step(g, img, img.rel32_target(f + 0x7d));
        let rigs = Frame {
            at: img.rel32_target(f + 0x85),
            ..*fr
        };
        return crate::s8_misc::camera_rigs_step(g, img, &rigs);
    }
    let node = g.read::<u32>(w(0x8b));
    g.write::<u32>(fx + 0x16c8, 1);
    if node != 0 {
        eax = crate::sa_pool::give_back(g, img, img.rel32_target(f + 0x9e), node);
        g.write::<u32>(w(0xa4), 0);
    }
    crate::sa_misc::register_parts(g, img, img.rel32_target(f + 0xad), eax);
    let own = fr.arg(g, 0);
    Ret::eax(crate::sa_pool::give_back(
        g,
        img,
        img.rel32_target(f + 0xb8),
        own,
    ))
}

/// [`task`] (188 bytes, 1 overlay).
pub const TASK: Sig = Sig {
    len: 188,
    hash: 0xbd86_508c_0fc6_e371,
    calls: &[
        (0x15, Callee::Import("memset")),
        (0x38, Callee::Sig(&crate::sb_misc::SCRIPT_RUN)),
        (0x44, Callee::Sig(&crate::sb_misc::SCRIPT_RUN)),
        (0x7d, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0x85, Callee::Sig(&crate::s8_misc::CAMERA_RIGS_STEP)),
        (0x9e, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0xad, Callee::Sig(&crate::sa_misc::REGISTER_PARTS)),
        (0xb8, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
    ],
    imports: &[],
};

/// Starts a host object (internal helper): the object `obj` started for
/// the host at the operand `host_op`
/// ([`start_host_object`](crate::s4_small::start_host_object): scale `*K`
/// at `k_op`, the stack `(index, 0)` at `esp + 4`, the call at
/// `call_op`).
#[allow(clippy::too_many_arguments)] // the call's registers and stack
fn host_object<G: Game>(
    g: &mut G,
    img: &Image,
    fr: &Frame,
    esp: u32,
    (call_op, host_op, k_op): (u32, u32, u32),
    obj: u32,
    index: u32,
) {
    let f = fr.at;
    g.write::<u32>(esp + 4, index);
    g.write::<u32>(esp + 8, 0);
    let mut xmm = fr.xmm;
    xmm[3] = u128::from(g.read::<u32>(img.word(f + k_op)));
    let callee = Frame {
        at: img.rel32_target(f + call_op),
        esp,
        ecx: img.word(f + host_op),
        edx: obj,
        xmm,
        ..*fr
    };
    crate::s4_small::start_host_object(g, img, &callee);
}

/// The point `*G_FX + from` copied to the object's `+0xd0..`; returns
/// the last word.
fn copy_point<G: Game>(g: &mut G, fx: u32, from: u32, obj: u32) -> u32 {
    let mut last = 0;
    for i in 0..4 {
        last = g.read::<u32>(fx + from + 4 * i);
        g.write::<u32>(obj + 0xd0 + 4 * i, last);
    }
    last
}

/// Starts the two objects `*G_FX + 0x390` and `+0x5f0` (0x10001cc0, an
/// overlay table entry, no arguments): each a host object (hosts the
/// immediates `+0x2c`, `+0x93`; scale `*K` `+0x8`/`+0x49`; index 0) at
/// the point `+0x1448` raised by `K` (`+0x8b`, `+0xf9`), its `+0xc4` the
/// angle `+0x16bc`; `+0x16c4` = 0. Returns the point's `w`.
/// `magic_0304` 0x10002270 ([`PAIR_START_0304`](crate::r21::PAIR_START_0304),
/// 312 bytes) reads the angle and the point 0x60 higher and also moves
/// `+0x146c` by `K` (into `+0x140c` too) before its `+0x1724` = 0; the
/// displacements are read from the code.
pub fn pair_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let fx = g.read::<u32>(w(0x10));
    // `movss xmm0, [edi + A]`, `mov eax, [edi + P]`
    let (angle, point) = (w(0x1b), w(0x3c));
    let a = g.read::<u32>(fx + angle);
    // push ebp, ecx, esi, edi; sub esp, 8; two words; the return address
    let esp = fr.esp.wrapping_sub(0x24);
    let mut last = 0;
    for (ops, obj, k) in [
        ((0x36, 0x2c, 0x8), 0x390, 0x8b),
        ((0xac, 0x93, 0x49), 0x5f0, 0xf9),
    ] {
        let o = fx.wrapping_add(obj);
        host_object(g, img, fr, esp, ops, o, 0);
        last = copy_point(g, fx, point, o);
        g.write::<u32>(o + 0xc4, a);
        let y = add(g.read::<f32>(o + 0xd4), g.read::<f32>(w(k)));
        g.write::<f32>(o + 0xd4, y);
    }
    // `magic_0304`: `movss xmm0, [edi + Y]` (`f3`) where `0568` has the
    // final `mov dword [edi + D], 0` (`c7`)
    let d = if w(0x108) & 0xff == 0xf3 {
        let y = add(g.read::<f32>(fx + w(0x10c)), g.read::<f32>(w(0x114)));
        g.write::<f32>(fx + w(0x11c), y);
        g.write::<f32>(fx + w(0x124), y);
        0x20
    } else {
        0
    };
    g.write::<u32>(fx + w(0x10a + d), 0);
    Ret::eax(last)
}

/// [`pair_start`] (280 bytes, 1 overlay).
pub const PAIR_START: Sig = Sig {
    len: 280,
    hash: 0x270e_daef_a0a6_7fac,
    calls: &[
        (0x36, Callee::Sig(&crate::s4_small::START_HOST_OBJECT)),
        (0xac, Callee::Sig(&crate::s4_small::START_HOST_OBJECT)),
    ],
    imports: &[],
};

/// Starts the object `*G_FX + 0x720` (0x10002840, an overlay table
/// entry, no arguments): a host object (host the immediate `+0x20`,
/// scale `*K` `+0x4`, index 1) at the point `+0x1428` raised by `K`
/// (`+0x65`), its `+0xc4` the angle `+0x16bc`, which it returns.
pub fn object_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let fx = g.read::<u32>(w(0xb));
    let o = fx.wrapping_add(0x720);
    // push esi, edi; sub esp, 8; two words; the return address
    host_object(
        g,
        img,
        fr,
        fr.esp.wrapping_sub(0x1c),
        (0x25, 0x20, 0x4),
        o,
        1,
    );
    copy_point(g, fx, 0x1428, o);
    let y = add(g.read::<f32>(o + 0xd4), g.read::<f32>(w(0x65)));
    let a = g.read::<u32>(fx + 0x16bc);
    g.write::<u32>(o + 0xc4, a);
    g.write::<f32>(o + 0xd4, y);
    Ret::eax(a)
}

/// [`object_start`] (131 bytes, 1 overlay).
pub const OBJECT_START: Sig = Sig {
    len: 131,
    hash: 0x2e34_2982_da79_2b80,
    calls: &[(0x25, Callee::Sig(&crate::s4_small::START_HOST_OBJECT))],
    imports: &[],
};
