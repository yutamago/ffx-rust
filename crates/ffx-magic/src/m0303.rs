//! `magic_0303.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::sb_sse::add;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The frame callback (0x10001370, cdecl `(task)`): the motion step
/// ([`motion_step`](crate::s3_motion::motion_step)), two scripts
/// ([`script_run`](crate::sb_misc::script_run) on `+0x0c` and `+0x18`)
/// and the objects ([`run_objects`](crate::l5_objhost::run_objects));
/// when all four answer 1 the effect ends: the node `*Q` (`+0x3d`), when
/// set, given back to its pool ([`give_back`](crate::sa_pool::give_back))
/// and `*Q` cleared; slot 296 `(*S, 0)` while the flag `+0x56` is clear
/// and the sound `*S` (`+0x5e`) is below 0x20, else slot 257 `(*T, 1)`
/// (`+0x80`); the task gets state 3 and its record the callback `+0x9f`.
/// Returns the record, else the objects' answer.
pub fn frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let step = crate::s3_motion::motion_step(g, img, img.rel32_target(f + 0x07));
    let s1 = crate::sb_misc::script_run(g, img, img.rel32_target(f + 0x13), w(0x0c));
    let s2 = crate::sb_misc::script_run(g, img, img.rel32_target(f + 0x1f), w(0x18));
    // push ebp; ebx, esi, edi; the call's return address
    let r = crate::l5_objhost::run_objects(
        g,
        img,
        img.rel32_target(f + 0x26),
        fr.esp.wrapping_sub(0x14),
    );
    if r != 1 || step != 1 || s1 != 1 || s2 != 1 {
        return Ret::eax(r);
    }
    let node = g.read::<u32>(w(0x3d));
    if node != 0 {
        crate::sa_pool::give_back(g, img, img.rel32_target(f + 0x46), node);
        g.write::<u32>(w(0x4c), 0);
    }
    let sound = g.read::<i32>(w(0x5e));
    if g.read::<u32>(w(0x56)) == 0 && sound < 0x20 {
        let s = slot_target(g, w(0x6b), 0x4a0 / 4);
        g.call(s, 0, &[sound.cast_unsigned(), 0]);
    } else {
        let s = slot_target(g, w(0x78), 0x404 / 4);
        let t = g.read::<u32>(w(0x80));
        g.call(s, 0, &[t, 1]);
    }
    let task = fr.arg(g, 0);
    g.write::<u32>(task + 0x14, 3);
    let rec = g.read::<u32>(task + 8);
    g.write::<u32>(rec + 0x10, w(0x9f));
    Ret::eax(rec)
}

/// [`frame`] (168 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 168,
    hash: 0xd4f0_07b8_ff18_a71f,
    calls: &[
        (0x07, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0x13, Callee::Sig(&crate::sb_misc::SCRIPT_RUN)),
        (0x1f, Callee::Sig(&crate::sb_misc::SCRIPT_RUN)),
        (0x26, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
        (0x46, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
    ],
    imports: &[],
};

/// The caller placed (0x10001540, no arguments, a /GS frame of 0x14
/// bytes): slot 262 `(*S, &x, &y, &z)` (`S` `+0x23`) fills a vector on
/// the frame (`ebp - 0x14`); the height `*H` (`+0x33`) grows by `K`
/// (`+0x3b`) and is added to `y`, the fourth word is 1.0, and the vector
/// goes to [`place_me`](crate::sa_misc::place_me). Returns its `eax`.
pub fn place<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let v = fr.esp.wrapping_sub(0x18);
    let s = slot_target(g, w(0x1d), 0x418 / 4);
    let src = g.read::<u32>(w(0x23));
    g.call(s, 0, &[src, v, v + 4, v + 8]);
    let h = add(g.read::<f32>(w(0x33)), g.read::<f32>(w(0x3b)));
    g.write::<u32>(v + 0xc, 0x3f80_0000);
    g.write::<f32>(w(0x50), h);
    let y = add(h, g.read::<f32>(v + 4));
    g.write::<f32>(v + 4, y);
    Ret::eax(crate::sa_misc::place_me(
        g,
        img,
        img.rel32_target(f + 0x5f),
        v,
    ))
}

/// [`place`] (113 bytes, 1 overlay).
pub const PLACE: Sig = Sig {
    len: 113,
    hash: 0xc8fa_387f_94db_a745,
    calls: &[
        (0x5f, Callee::Sig(&crate::sa_misc::PLACE_ME)),
        // __security_check_cookie
        (0x69, Callee::Any),
    ],
    imports: &[],
};

/// The record's words, then its node (0x10001480, a table entry, no
/// arguments): operands `+0x02`, `+0x0c`, `+0x16`, `+0x20` get `[0, 0,
/// 0x50, 0]`, then a tail jump into
/// [`record_two_node`](crate::o4_static::record_two_node). Returns its
/// result.
pub fn record_nodes<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    for (o, v) in [(0x2u32, 0u32), (0xc, 0), (0x16, 0x50), (0x20, 0)] {
        g.write::<u32>(w(o), v);
    }
    let callee = Frame {
        at: img.rel32_target(f + 0x29),
        ..*fr
    };
    crate::o4_static::record_two_node(g, img, &callee)
}

/// [`record_nodes`] (45 bytes, 1 overlay).
pub const RECORD_NODES: Sig = Sig {
    len: 45,
    hash: 0x7530_f3ce_c13f_3f64,
    calls: &[(
        0x29,
        Callee::Sig(&crate::o4_static::RECORD_TWO_NODE_0303_48D0),
    )],
    imports: &[],
};

/// Starts the light object (0x100015c0, a table entry, no arguments; a
/// /GS frame): the object at the immediate `+0x24`
/// ([`start_object_anim_2`](crate::s6_misc::start_object_anim_2), scale
/// the float at `+0x14`, the stack `(0, ecx, 8, F)`, `F` the immediate
/// `+0x1a`); slot 262 `(*G_M, X, Y, Z)` (operand `+0x34`, the immediates
/// `+0x49`, `+0x44`, `+0x3f`) and slot 268 `(*G_M, 6, &P)` (`P` a local at
/// `ebp - 0x14`), each over the earlier words; `*W` (`+0x7b`) 1.0 and
/// `Y` (`+0x87`) = `P.y + K` (`+0x6d`). Returns slot 268's `eax`.
pub fn light_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let cookie = g.read::<u32>(w(0x7)) ^ ebp;
    g.write::<u32>(ebp.wrapping_sub(4), cookie);
    // sub esp, 0x14; push esi; four words; the call
    let esp = ebp.wrapping_sub(0x2c);
    let words = [0, fr.ecx, 8, w(0x1a)];
    for (i, v) in (1u32..).zip(words) {
        g.write::<u32>(esp + 4 * i, v);
    }
    let mut xmm = fr.xmm;
    xmm[3] = u128::from(g.read::<u32>(w(0x14)));
    let start = Frame {
        at: img.rel32_target(f + 0x29),
        esp,
        edx: w(0x24),
        xmm,
        ..*fr
    };
    crate::s6_misc::start_object_anim_2(g, img, &start);
    let m = g.read::<u32>(w(0x34));
    let mut s = crate::pushes::Pushes::default();
    s.push_all(&[w(0x1a), 8, fr.ecx, 0]);
    s.push_all(&[w(0x3f), w(0x44), w(0x49), m]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x2e), 262), fr.ecx);
    let p = ebp.wrapping_sub(0x14);
    s.push_all(&[p, 6, m]);
    // call width: 12 (Pushes caps the words at 12)
    let eax = s.call(g, slot_target(g, w(0x55), 268), fr.ecx).eax;
    let y = add(g.read::<f32>(p + 4), g.read::<f32>(w(0x6d)));
    g.write::<u32>(w(0x7b), 0x3f80_0000);
    g.write::<f32>(w(0x87), y);
    Ret::eax(eax)
}

/// [`light_start`] (149 bytes, 1 overlay; the `rel32` at `+0x8d` is
/// `__security_check_cookie`).
pub const LIGHT_START: Sig = Sig {
    len: 149,
    hash: 0x9f85_d31d_22fb_6887,
    calls: &[
        (0x29, Callee::Sig(&crate::s6_misc::START_OBJECT_ANIM_2)),
        (0x8d, Callee::Any),
    ],
    imports: &[],
};
