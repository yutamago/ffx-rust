//! `magic_0606.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::sb_sse::{add, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Shows the targets (0x10007a10, an overlay table entry, no arguments):
/// for each of the `*G_N` targets (operand `+0x05`, signed; the list
/// `*G_L`, `+0x19`): outside a battle (`+0x12` 0) a party member (the
/// list byte, or `G_ME` `+0x20` for target -1, below 32) gets slot 530
/// `(id, 0)`; otherwise slot 249 `(OBJS[id], 0)` (the table at `+0x55`;
/// `+0x48` for target -1). Returns the last call's `eax`, or the caller's
/// `eax` without targets.
pub fn show_targets<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let mut eax = fr.eax;
    let mut i = 0u32;
    while i.cast_signed() < g.read::<i32>(w(if i == 0 { 0x5 } else { 0x6f })) {
        let battle = g.read::<u32>(w(0x12)) != 0;
        let list = g.read::<u32>(w(0x19));
        let mut member = None;
        if !battle {
            let a = if i == u32::MAX {
                g.read::<u32>(w(0x20))
            } else {
                u32::from(g.read::<u8>(list.wrapping_add(i)))
            };
            if a.cast_signed() < 0x20 {
                member = Some(a);
            }
        }
        if let Some(a) = member {
            let t = slot_target(g, w(0x36), 530);
            eax = g.call(t, 0, &[a, 0]).eax;
        } else {
            let o = if i == u32::MAX {
                g.read::<u32>(w(0x48))
            } else {
                let b = u32::from(g.read::<u8>(list.wrapping_add(i)));
                g.read::<u32>(w(0x55).wrapping_add(4 * b))
            };
            let t = slot_target(g, w(0x5d), 249);
            eax = g.call(t, 0, &[o, 0]).eax;
        }
        i = i.wrapping_add(1);
    }
    Ret::eax(eax)
}

/// [`show_targets`] (119 bytes, 1 overlay).
pub const SHOW_TARGETS: Sig = Sig {
    len: 119,
    hash: 0xf005_e27b_623a_7a0d,
    calls: &[],
    imports: &[],
};

/// Places the effect's camera points (0x10008990, an overlay table
/// entry, no arguments; a frame with a stack cookie): the matrix `m` at
/// `ebp - 0x54` from the data at `+0x1b` (slot 327), the point `R +
/// 0x9f0` (`R = *G_R`, operand `+0x13`) through it into `v` at `ebp -
/// 0x14` (slot 229, the first call's words under it). The look point
/// (`+0x62`, `+0x77`, `+0x8c`) is `v + (A, -B, C)` (operands `+0x52`,
/// `+0x6f`, `+0x84`); when slot 520 says 0x1590000 its z is `v.z + D`
/// (`+0xac`). The eye (`+0xd1`, `+0xee`, `+0xe6`) is `v - (0, E, 0)`
/// (`+0xde`). Returns slot 520's `eax`.
pub fn camera_points<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let k = |g: &mut G, off: u32| g.read::<f32>(w(off));
    let ebp = fr.esp.wrapping_sub(4);
    let (m, v) = (ebp.wrapping_sub(0x54), ebp.wrapping_sub(0x14));
    let r = g.read::<u32>(w(0x13));
    let t = slot_target(g, w(0x21), 327);
    g.call(t, 0, &[m, w(0x1b)]);
    let t = slot_target(g, w(0x3d), 229);
    g.call(t, 0, &[v, m, r + 0x9f0, m, w(0x1b)]);
    let x = add(g.read::<f32>(v), k(g, 0x52));
    g.write::<f32>(w(0x62), x);
    let y = sub(g.read::<f32>(v + 4), k(g, 0x6f));
    g.write::<f32>(w(0x77), y);
    let z = add(g.read::<f32>(v + 8), k(g, 0x84));
    g.write::<f32>(w(0x8c), z);
    let t = slot_target(g, w(0x57), 520);
    let eax = g.call(t, 0, &[]).eax;
    let vz = g.read::<f32>(v + 8);
    if eax == 0x0159_0000 {
        let z = add(vz, k(g, 0xac));
        g.write::<f32>(w(0xb4), z);
    }
    let ex = sub(g.read::<f32>(v), 0.0);
    g.write::<f32>(w(0xd1), ex);
    let ey = sub(g.read::<f32>(v + 4), k(g, 0xde));
    g.write::<f32>(w(0xe6), sub(vz, 0.0));
    g.write::<f32>(w(0xee), ey);
    Ret::eax(eax)
}

/// [`camera_points`] (251 bytes, 1 overlay).
pub const CAMERA_POINTS: Sig = Sig {
    len: 251,
    hash: 0xa7d8_4779_751d_daab,
    // __security_check_cookie
    calls: &[(0xf3, Callee::Any)],
    imports: &[],
};

/// Starts the effect's three tasks (0x10008720, internal, no arguments):
/// the colour block `S + 0x58c0` (`S = *G_S`) and the timer block `S +
/// 0x597c` set; three pool nodes ([`take`](crate::sa_pool::take), each
/// with its own owner) for them and for the step task (`S[0x5ee0]`, a
/// count `S[0x5ef4]` of 12) go at the head of the DLL's task list one
/// after another (the list's head word written once, at the end; the
/// first word only while the list is empty). Returns the last node.
pub fn start_tasks<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let take = |g: &mut G, call: u32, owner: u32| {
        crate::sa_pool::take(g, img, img.rel32_target(f + call), w(owner))
    };
    let s = g.read::<u32>(w(0x5));
    let e = s + 0x58c0;
    g.write::<u32>(e + 0x81, 0xff80_ffff);
    g.write::<u8>(e + 0x80, 0xff);
    g.write::<u16>(e + 0x85, 0xffff);
    g.write::<u8>(e + 0x87, 0);
    g.write::<u32>(e + 0x88, 0);
    g.write::<u16>(e + 0x98, 0x48);
    g.write::<u32>(e + 0x90, 0x18);
    g.write::<u32>(e + 0x8c, 0x18);
    g.write::<u32>(e + 0x94, 0);
    let n1 = take(g, 0x67, 0xa);
    let empty = g.read::<u32>(w(0x6d)) == 0;
    g.write::<u32>(n1 + 0x14, e);
    g.write::<u32>(n1 + 4, 0);
    if empty {
        g.write::<u32>(n1, 0);
        g.write::<u32>(w(0x95), n1);
    } else {
        let h = g.read::<u32>(w(0x82));
        g.write::<u32>(n1, h);
        g.write::<u32>(h + 4, n1);
    }
    let e2 = s + 0x597c;
    g.write::<u32>(n1 + 8, n1);
    for (o, v) in [
        (0u32, 0u32),
        (4, 0x50),
        (8, 0x14),
        (0xc, 0),
        (0x14, 0x5a),
        (0x10, 0x5a),
    ] {
        g.write::<u32>(e2 + o, v);
    }
    g.write::<u16>(e2 + 0x18, 0);
    let n2 = take(g, 0xd7, 0xa5);
    let empty = g.read::<u32>(w(0xdd)) == 0;
    g.write::<u8>(n2 + 0x18, 1);
    g.write::<u32>(n2 + 0x14, e2);
    g.write::<u32>(n2 + 4, 0);
    if empty {
        g.write::<u32>(n2, 0);
        g.write::<u32>(w(0x103), n2);
    } else {
        g.write::<u32>(n2, n1);
        g.write::<u32>(n1 + 4, n2);
    }
    let s2 = g.read::<u32>(w(0x109));
    g.write::<u32>(n2 + 8, n2);
    g.write::<u32>(s2 + 0x5ef4, 0xc);
    let n3 = take(g, 0x120, 0x111);
    let empty = g.read::<u32>(w(0x126)) == 0;
    g.write::<u32>(s2 + 0x5ee0, n3);
    g.write::<u32>(n3 + 4, 0);
    g.write::<u32>(w(0x139), n3);
    if empty {
        g.write::<u32>(n3, 0);
        g.write::<u32>(w(0x154), n3);
    } else {
        g.write::<u32>(n3, n2);
        g.write::<u32>(n2 + 4, n3);
    }
    g.write::<u32>(n3 + 8, n3);
    n3
}

/// [`start_tasks`] (349 bytes, 1 overlay).
pub const START_TASKS: Sig = Sig {
    len: 349,
    hash: 0x6aab_ec22_fb36_f6b6,
    calls: &[
        (0x67, Callee::Sig(&crate::sa_pool::TAKE)),
        (0xd7, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x120, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// Starts the object `*G_FX + 0x720` (0x10007b10, an overlay table
/// entry, no arguments): [`start_object_first`](crate::o6_fam_object::start_object_first)
/// (scale `*K` `+0x4`; its two stack words are the 16 bytes the code
/// reserves and never writes: a junk read, the port reads the same stack
/// words), slot 237 `(*G_A, 0x8a, 0, P)` fills its point `P = +0xd0`
/// (`w` 1.0), `+0xc0` = pi/4, `+0xc4` the angle `+0x5ee8 + K` wrapped
/// once ([`wrap_once`](crate::ob_model::wrap_once)), `y - K` (`+0x9a`).
/// Returns slot 237's `eax`.
pub fn object_720<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let fx = g.read::<u32>(w(0xd));
    let obj = fx.wrapping_add(0x720);
    let mut xmm = fr.xmm;
    xmm[3] = u128::from(g.read::<u32>(w(0x4)));
    // push ebx, esi, edi; sub esp, 0x10; the return address
    let start = Frame {
        at: img.rel32_target(f + 0x1d),
        esp: fr.esp.wrapping_sub(0x20),
        edx: obj,
        xmm,
        ..*fr
    };
    crate::o6_fam_object::start_object_first(g, img, start.at, &start);
    let p = obj.wrapping_add(0xd0);
    let t = slot_target(g, w(0x22), 237);
    let a = g.read::<u32>(w(0x3c));
    let eax = g.call(t, 0, &[a, 0x8a, 0, p]).eax;
    g.write::<u32>(p + 0xc, 0x3f80_0000);
    g.write::<u32>(obj + 0xc0, 0x3f49_0fdb);
    let x = add(g.read::<f32>(fx + 0x5ee8), g.read::<f32>(w(0x5f)));
    let x = crate::ob_model::wrap_once(g, img, f, x, [0x69, 0x73, 0x7d, 0x8a]);
    let y = sub(g.read::<f32>(obj + 0xd4), g.read::<f32>(w(0x9a)));
    g.write::<f32>(obj + 0xc4, x);
    g.write::<f32>(obj + 0xd4, y);
    Ret::eax(eax)
}

/// [`object_720`] (178 bytes, 1 overlay).
pub const OBJECT_720: Sig = Sig {
    len: 178,
    hash: 0xe17e_f280_a97b_fccb,
    calls: &[(
        0x1d,
        Callee::Sig(&crate::o6_fam_object::START_OBJECT_FIRST_0606_4D70),
    )],
    imports: &[],
};
