//! Round 12 (own rows from `magic_0450` up): parameter families
//! (`~/ffx-work/m5/magic/r4/pfamilies.tsv`) whose members are all own
//! rows of this range: one port, a `Sig` per copy, the operands read from
//! each copy's code (`w(off)` = the word at `+off`).

use crate::runtime::slot_target;
use crate::sb_sse::sub;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The summon's object swapped (`magic_0511` 0x10003000 and `magic_0589`
/// 0x10002be0, no arguments; a /GS frame with the colour `C` at
/// `ebp - 0x24`; family `P067`). With the record `R = *G_R`, the object
/// `O1` (at `R + d1`) leaves the object list
/// ([`crate::l5_objhost::list_remove`] on `L`); when it was on it,
/// `*G_P = ITEM`, slot 713 `(O1)` and `O1`'s halfword `+0x124` loses bit
/// 15. The object `O2` (at `R + d2`) starts
/// ([`crate::s4_misc::start_object_anim`], scale `K`, part 10, value 0;
/// two never-written stack words under them, which it does not read),
/// its `+0xd0..+0xdc` and `+0xc4` from `R` words, `+0xd4 -= D`; `C` =
/// the three floats `*G_C..` and 0x80, slot 332 `(&C)`; the fade task on
/// `R + d3` ([`crate::s3_misc::fade_task`], `(30, 0.0, K2, 0)`).
/// Returns the fade task's node.
pub fn object_swap<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let k = |g: &G, op: u32| g.read::<f32>(w(op));
    let r = g.read::<u32>(w(0x14));
    let o1 = r.wrapping_add(w(0x1f));
    let listed = crate::l5_objhost::list_remove(g, img, img.rel32_target(f + 0x26), w(0x19), o1);
    if listed != 0 {
        let p = g.read::<u32>(w(0x2f));
        g.write::<u32>(p, w(0x36));
        let t = slot_target(g, w(0x3b), 713);
        // call width: 1
        g.call(t, 0, &[o1]);
        let v = g.read::<u16>(o1 + 0x124);
        g.write::<u16>(o1 + 0x124, v & 0x7fff);
    }
    let scale = k(g, 0x5a);
    let o2 = r.wrapping_add(w(0x63));
    let (index, value) = (w(0x6a) & 0xff, w(0x68) & 0xff);
    crate::s4_misc::start_object_anim(g, img, img.rel32_target(f + 0x6e), o2, scale, index, value);
    let rd = |g: &G, op: u32| g.read::<u32>(r.wrapping_add(w(op)));
    let v = rd(g, 0x74);
    g.write::<u32>(o2 + 0xd0, v);
    let v = rd(g, 0x80);
    g.write::<u32>(o2 + 0xd4, v);
    let x = sub(g.read::<f32>(o2 + 0xd4), k(g, 0x9c));
    let v = rd(g, 0x94);
    g.write::<u32>(o2 + 0xd8, v);
    let v = rd(g, 0xa8);
    g.write::<u32>(o2 + 0xdc, v);
    let ebp = fr.esp.wrapping_sub(4);
    let c = ebp.wrapping_sub(0x24);
    g.write::<u32>(ebp.wrapping_sub(0x18), 0x80);
    g.write::<f32>(o2 + 0xd4, x);
    let v = rd(g, 0xc3);
    g.write::<u32>(o2 + 0xc4, v);
    for (i, op) in (0u32..).zip([0xd1u32, 0xde, 0xf1]) {
        let v = g.read::<u32>(w(op));
        g.write::<u32>(ebp.wrapping_sub(0x14 - 4 * i), v);
        g.write::<u32>(c + 4 * i, v);
    }
    let t = slot_target(g, w(0x10b), 332);
    // call width: 3 (the object start's words under it; below them the
    // two never-written ones)
    g.call(t, 0, &[c, index, value]);
    // the fade task's word: ebp, the frame, esi/edi, 8 bytes, the start's
    // two words, slot 332's, its own, the return address
    let esp = fr.esp.wrapping_sub(0x4c);
    g.write::<u32>(esp + 4, 0);
    let callee = Frame {
        at: img.rel32_target(f + 0x130),
        esp,
        ecx: r.wrapping_add(w(0x123)),
        edx: w(0x128),
        xmm: [0, 0, u128::from(g.read::<u32>(w(0x11b))), 0],
        ..Frame::default()
    };
    Ret::eax(crate::s3_misc::fade_task(g, img, &callee))
}

/// [`object_swap`]'s callees.
const OBJECT_SWAP_CALLS: &[(u32, Callee)] = &[
    (0x26, Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
    (0x6e, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
    (0x130, Callee::Sig(&crate::s3_misc::FADE_TASK)),
    // __security_check_cookie
    (0x13f, Callee::Any),
];

/// [`object_swap`] in `magic_0511` (0x10003000).
pub const OBJECT_SWAP_0511: Sig = Sig {
    len: 327,
    hash: 0x23bb_c2be_2e32_ef81,
    calls: OBJECT_SWAP_CALLS,
    imports: &[],
};

/// [`object_swap`] in `magic_0589` (0x10002be0).
pub const OBJECT_SWAP_0589: Sig = Sig {
    len: 327,
    hash: 0xd2ac_fac6_bdcb_eff9,
    calls: OBJECT_SWAP_CALLS,
    imports: &[],
};

/// One object of [`three_objects`]: the scale's operand, the start's
/// `rel32`, the four copies' displacements, `D`, the `+0xc4` copy's
/// displacement, and `DY` (`None`: the copy without it).
type ObjectOps = (u32, u32, [u32; 4], u32, u32, Option<u32>);

/// The summon's three objects started (`magic_0511` 0x10003c80, 368
/// bytes, and `magic_0589` 0x10003870, 344 bytes, no arguments; family
/// `F0511_3c80`): with the record `R = *G_R`, the objects `O_i = R +
/// 0x10a0 + 0x130 i`, each started ([`crate::s4_misc::start_object_anim`],
/// scale `K`, part 4, value 0; two never-written stack words under the
/// first one's words, which it does not read), its `+0xd0..+0xdc` from
/// `R + d..`, `+0xc4` from `R + d4` and `+0xd4 -= D` (`magic_0511`: then
/// `+= *G_DY`). Returns the last `R[d4]`.
fn three_objects<G: Game>(g: &mut G, img: &Image, fr: &Frame, ops: &[ObjectOps; 3]) -> Ret {
    use crate::sb_sse::add;
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let k = |g: &G, op: u32| g.read::<f32>(w(op));
    let r = g.read::<u32>(w(0x0b));
    let first = r.wrapping_add(w(0x15));
    let mut eax = 0;
    for (i, &(scale, call, copies, d, c4, dy)) in (0u32..).zip(ops) {
        let obj = first.wrapping_add(0x130 * i);
        let s = k(g, scale);
        crate::s4_misc::start_object_anim(g, img, img.rel32_target(f + call), obj, s, 4, 0);
        for (j, op) in (0u32..).zip(copies) {
            let v = g.read::<u32>(r.wrapping_add(w(op)));
            g.write::<u32>(obj + 0xd0 + 4 * j, v);
        }
        let mut x = sub(g.read::<f32>(obj + 0xd4), k(g, d));
        eax = g.read::<u32>(r.wrapping_add(w(c4)));
        g.write::<u32>(obj + 0xc4, eax);
        if let Some(dy) = dy {
            x = add(x, g.read::<f32>(w(dy)));
        }
        g.write::<f32>(obj + 0xd4, x);
    }
    Ret::eax(eax)
}

/// [`three_objects`] in `magic_0511` (0x10003c80).
pub fn three_objects_0511<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    three_objects(
        g,
        img,
        fr,
        &[
            (0x04, 0x20, [0x26, 0x32, 0x46, 0x52], 0x68, 0x6e, Some(0x85)),
            (0x3a, 0x95, [0x9b, 0xa7, 0xbb, 0xc7], 0xdd, 0xe3, Some(0xfa)),
            (
                0xaf,
                0x10a,
                [0x110, 0x11c, 0x128, 0x134],
                0x14a,
                0x150,
                Some(0x161),
            ),
        ],
    )
}

/// [`three_objects`] in `magic_0589` (0x10003870).
pub fn three_objects_0589<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    three_objects(
        g,
        img,
        fr,
        &[
            (0x04, 0x20, [0x26, 0x3a, 0x46, 0x52], 0x68, 0x6e, None),
            (0x2e, 0x8d, [0x93, 0xa7, 0xb3, 0xbf], 0xd5, 0xdb, None),
            (0x9b, 0xfa, [0x100, 0x10c, 0x118, 0x124], 0x13a, 0x140, None),
        ],
    )
}

/// [`three_objects_0511`] (368 bytes, 1 overlay).
pub const THREE_OBJECTS_0511: Sig = Sig {
    len: 368,
    hash: 0x10f8_7bba_a236_9080,
    calls: &[
        (0x20, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x95, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x10a, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
    ],
    imports: &[],
};

/// [`three_objects_0589`] (344 bytes, 1 overlay).
pub const THREE_OBJECTS_0589: Sig = Sig {
    len: 344,
    hash: 0x2578_1c65_7d8b_dd06,
    calls: &[
        (0x20, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x8d, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0xfa, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
    ],
    imports: &[],
};
