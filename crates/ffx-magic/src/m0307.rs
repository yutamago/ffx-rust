//! `magic_0307.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The effect's end (0x10006e50, no arguments, an aligned frame): when
/// the effect (`*E`, `+0x09`) holds a pool node (`+0x2864`) it is given
/// back ([`give_back`](crate::sa_pool::give_back)), the field cleared,
/// and the handles of the 18 part blocks (`*P + 0x340..0x580`, 0x20 apart,
/// `P` `+0x31`) are registered: slot 772 `(block, 0)`'s 64-bit result to
/// slot 779 `(lo, hi, block, 0)` with the game flag `*L` (`+0x49`/`+0x65`)
/// 1 around it. Then the part item `+0x7a` leaves the object list `+0x7f`
/// ([`list_remove`](crate::l5_objhost::list_remove)); when it was there
/// the pointer `*Q` (`+0x8d`) gets its block, slot 713 `(item)` frees
/// it and the flag halfword `+0xb4` loses bit 15. Returns 0x7fff then,
/// else 0.
pub fn end<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let fx = g.read::<u32>(w(0x09));
    let node = g.read::<u32>(fx + 0x2864);
    if node != 0 {
        crate::sa_pool::give_back(g, img, img.rel32_target(f + 0x19), node);
        g.write::<u32>(fx + 0x2864, 0);
        for off in (0x340u32..0x580).step_by(0x20) {
            let block = g.read::<u32>(w(0x31)).wrapping_add(off);
            let s = slot_target(g, w(0x3b), 0xc10 / 4);
            let h = g.call(s, 0, &[block, 0]);
            let lock = g.read::<u32>(w(0x49));
            let saved = g.read::<u32>(lock);
            g.write::<u32>(lock, 1);
            let s = slot_target(g, w(0x52), 0xc2c / 4);
            g.call(s, 0, &[h.eax, h.edx, block, 0]);
            let lock = g.read::<u32>(w(0x65));
            g.write::<u32>(lock, saved);
        }
    }
    let removed =
        crate::l5_objhost::list_remove(g, img, img.rel32_target(f + 0x84), w(0x7f), w(0x7a));
    if removed == 0 {
        return Ret::eax(0);
    }
    let ptr = g.read::<u32>(w(0x8d));
    g.write::<u32>(ptr, w(0x98));
    let s = slot_target(g, w(0x9d), 0xb24 / 4);
    g.call(s, 0, &[w(0x92)]);
    let fl = g.read::<u16>(w(0xb4));
    g.write::<u16>(w(0xb4), fl & 0x7fff);
    Ret::eax(0x7fff)
}

/// [`end`] (190 bytes, 1 overlay).
pub const END: Sig = Sig {
    len: 190,
    hash: 0x7beb_93b8_bb55_073a,
    calls: &[
        (0x19, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x84, Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
    ],
    imports: &[],
};

/// A part's sound and model (0x10006f10, cdecl `(obj)`): with `i` the
/// object's actor (`+0x120`; -1 uses `*H` itself) slot 239 `(*H, B[i],
/// 2)` (`H` `+0x08`, the byte list `*B`), the actor's mode 0
/// ([`actor_mode`](crate::sa_script::actor_mode)), slot 388 `(B[i], 6,
/// 1)` and slot 528 `(B[i], 1)`; then slot 393 `(h, rec + 0x1090, 0x100)`
/// with `h` slot 719 `(F, 6, 3)`'s handle (`F` `+0xb0`) and `rec` the
/// actor's record (`*R + 0xd4 i`, `R` `+0x1e`), whose `+0x10d0`
/// becomes 1. Returns slot 393's `eax`. Each slot call passes the words the
/// original leaves on the stack.
pub fn part_model<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let h = g.read::<u32>(w(0x08));
    let i = g.read::<u32>(obj + 0x120);
    let rec = i.wrapping_mul(0xd4).wrapping_add(g.read::<u32>(w(0x1e)));
    let byte = |g: &G, minus: u32, list: u32| {
        if i == u32::MAX {
            g.read::<u32>(w(minus))
        } else {
            let b = g.read::<u32>(w(list));
            u32::from(g.read::<u8>(b.wrapping_add(i)))
        }
    };
    let b = if i == u32::MAX {
        h
    } else {
        let list = g.read::<u32>(w(0x2c));
        u32::from(g.read::<u8>(list.wrapping_add(i)))
    };
    let s = slot_target(g, w(0x38), 0x3bc / 4);
    g.call(s, 0, &[h, b, 2]);
    // push ebp; esi; edi; the call's return address
    crate::sa_script::actor_mode(
        g,
        img,
        &Frame {
            at: img.rel32_target(f + 0x4d),
            esp: fr.esp.wrapping_sub(0x10),
            ecx: i,
            edx: 0,
            ..*fr
        },
    );
    let b = byte(g, 0x57, 0x5e);
    let s = slot_target(g, w(0x6c), 0x610 / 4);
    g.call(s, 0, &[b, 6, 1]);
    let b = byte(g, 0x81, 0x88);
    let mut p = Pushes::default();
    p.push_all(&[1, b]);
    let s = slot_target(g, w(0x94), 0x840 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    p.push_all(&[3, 6, w(0xb0)]);
    let s = slot_target(g, w(0xa1), 0xb3c / 4);
    // call width: 12 (Pushes caps the words at 12)
    let handle = p.call(g, s, 0).eax;
    p.push_all(&[0x100, rec.wrapping_add(0x1090), handle]);
    let s = slot_target(g, w(0xc4), 0x624 / 4);
    // call width: 12 (Pushes caps the words at 12)
    let eax = p.call(g, s, 0).eax;
    g.write::<u32>(rec.wrapping_add(0x10d0), 1);
    Ret::eax(eax)
}

/// [`part_model`] (225 bytes, 1 overlay).
pub const PART_MODEL: Sig = Sig {
    len: 225,
    hash: 0x1c16_ee8c_3560_9966,
    calls: &[(0x4d, Callee::Sig(&crate::sa_script::ACTOR_MODE))],
    imports: &[],
};

/// The 30-count scale task on the block's `+0x255c` (0x10007980, a table
/// entry, no arguments; `*G_B` operand `+0x02`, `xmm2`/`xmm1` the floats
/// at `+0x0a`/`+0x12`): a tail jump into
/// [`scale_task_fixed`](crate::s4_small::scale_task_fixed). Returns its
/// node.
pub fn scale_255c<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    scale_jump(g, img, fr, w(0x2), (0xa, 0x12), 0x255c, 0x1d)
}

/// `ecx` = `*G_B + d`, `xmm2`/`xmm1` the floats at the operands `k`, then
/// a tail jump to the `rel32` at `at` ([`scale_task_fixed`](crate::s4_small::scale_task_fixed)).
fn scale_jump<G: Game>(
    g: &mut G,
    img: &Image,
    fr: &Frame,
    gb: u32,
    k: (u32, u32),
    d: u32,
    at: u32,
) -> Ret {
    let f = fr.at;
    let mut xmm = fr.xmm;
    xmm[2] = u128::from(g.read::<u32>(img.word(f + k.0)));
    xmm[1] = u128::from(g.read::<u32>(img.word(f + k.1)));
    let callee = Frame {
        at: img.rel32_target(f + at),
        ecx: g.read::<u32>(gb).wrapping_add(d),
        xmm,
        ..*fr
    };
    crate::s4_small::scale_task_fixed(g, img, &callee)
}

/// [`scale_255c`] (33 bytes, 1 overlay).
pub const SCALE_255C: Sig = Sig {
    len: 33,
    hash: 0xae4f_f4cc_8a05_5d7f,
    calls: &[(0x1d, Callee::Sig(&crate::s4_small::SCALE_TASK_30))],
    imports: &[],
};

/// Slot 371 `(0, 0x10000)` and `(1, 0x20000)` (the first call's words
/// left under the second's), then the 30-count scale task on the
/// block's `+0x2524` (0x10002940, a table entry, no arguments; `*G_B`
/// operand `+0x2a`, `xmm2`/`xmm1` the floats at `+0x32`/`+0x3a`; a tail
/// jump into [`scale_task_fixed`](crate::s4_small::scale_task_fixed)).
/// Returns its node.
pub fn scale_2524<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = slot_target(g, w(0x1), 371);
    g.call(t, 0, &[0, 0x1_0000]);
    let t = slot_target(g, w(0x15), 371);
    g.call(t, 0, &[1, 0x2_0000, 0, 0x1_0000]);
    scale_jump(g, img, fr, w(0x2a), (0x32, 0x3a), 0x2524, 0x48)
}

/// [`scale_2524`] (76 bytes, 1 overlay).
pub const SCALE_2524: Sig = Sig {
    len: 76,
    hash: 0xec8f_82e7_5494_3a4a,
    calls: &[(0x48, Callee::Sig(&crate::s4_small::SCALE_TASK_30))],
    imports: &[],
};

/// Starts the seventh object (0x10002aa0, a table entry, no arguments;
/// the block `S = *G_S`, operand `+0x0b`): the object at the immediate
/// `+0x17` ([`start_object_at`](crate::s4_misc::start_object_at), scale
/// the float at `+0x04`, index 7, owners 0, 0); the four globals at
/// `+0x2c`, `+0x3c`, `+0x54`, `+0x67` = `S + 0xc30..0xc3c`, `+0x77` = `S +
/// 0x2868`, then `+0x7f` = `S[0xc34] - K` (`+0x4c`). Returns the object.
pub fn seventh_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let scale = g.read::<f32>(w(0x4));
    let s = g.read::<u32>(w(0xb));
    let o = crate::s4_misc::start_object_at(
        g,
        img,
        img.rel32_target(f + 0x1c),
        w(0x17),
        scale,
        7,
        0,
        0,
    );
    for (k, at) in (0u32..).zip([0x2c, 0x3c, 0x54, 0x67]) {
        let v = g.read::<u32>(s + 0xc30 + 4 * k);
        g.write::<u32>(w(at), v);
    }
    let v = g.read::<u32>(s + 0x2868);
    g.write::<u32>(w(0x77), v);
    let y = crate::sb_sse::sub(g.read::<f32>(s + 0xc34), g.read::<f32>(w(0x4c)));
    g.write::<f32>(w(0x7f), y);
    Ret::eax(o)
}

/// [`seventh_start`] (133 bytes, 1 overlay).
pub const SEVENTH_START: Sig = Sig {
    len: 133,
    hash: 0x0c30_7068_4ff2_6ed5,
    calls: &[(0x1c, Callee::Sig(&crate::s4_misc::START_OBJECT_AT))],
    imports: &[],
};

/// Starts the two eleventh objects (0x10002b70, a table entry, no
/// arguments; `S = *G_S + 0x980`, operand `+0x0b`): `S` and `S + 0x130`
/// ([`start_object_at`](crate::s4_misc::start_object_at), scale the
/// floats at `+0x04`/`+0x27`, index 11, owners 0, 0), each then with `+0x104`
/// the immediate `+0x41`/`+0x5d` and `+0x120` 0x11/0x37. Returns the
/// second object.
pub fn pair_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let s = g.read::<u32>(w(0xb)).wrapping_add(0x980);
    let scale = g.read::<f32>(w(0x4));
    crate::s4_misc::start_object_at(g, img, img.rel32_target(f + 0x1f), s, scale, 0xb, 0, 0);
    let scale = g.read::<f32>(w(0x27));
    g.write::<u32>(s + 0x104, w(0x41));
    g.write::<u32>(s + 0x120, 0x11);
    let o = crate::s4_misc::start_object_at(
        g,
        img,
        img.rel32_target(f + 0x50),
        s + 0x130,
        scale,
        0xb,
        0,
        0,
    );
    g.write::<u32>(s + 0x234, w(0x5d));
    g.write::<u32>(s + 0x250, 0x37);
    Ret::eax(o)
}

/// [`pair_start`] (109 bytes, 1 overlay).
pub const PAIR_START: Sig = Sig {
    len: 109,
    hash: 0x8521_8178_6709_5cfb,
    calls: &[
        (0x1f, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x50, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
    ],
    imports: &[],
};
/// Restarts the third object (0x10005270, a table entry, no arguments;
/// the block `S = *G_S`, operand `+0x03`): when the object at the
/// immediate `+0x08` leaves the list at `+0x0d`
/// ([`list_remove`](crate::l5_objhost::list_remove)), `**G_P` (`+0x1b`)
/// gets the immediate `+0x26`, slot 713 `(obj)` and bit 15 of the
/// halfword `+0x42` is cleared. Then the object at `+0x56` starts
/// ([`start_object_at`](crate::s4_misc::start_object_at), scale the float
/// at `+0x4a`, index 4, owners 0, 0), the 16 bytes at `+0x62` are copied
/// to `+0x6c`, `+0x7c` = `S + 0x2868`, and the words at `+0x82`/`+0x8c`
/// get the immediates `+0x86`/`+0x90`. Returns the object.
pub fn third_restart<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let s = g.read::<u32>(w(0x3));
    let gone = crate::l5_objhost::list_remove(g, img, img.rel32_target(f + 0x12), w(0xd), w(0x8));
    if gone != 0 {
        let p = g.read::<u32>(w(0x1b));
        g.write::<u32>(p, w(0x26));
        let t = slot_target(g, w(0x2b), 713);
        g.call(t, 0, &[w(0x20)]);
        let h = g.read::<u16>(w(0x42));
        g.write::<u16>(w(0x42), h & 0x7fff);
    }
    let scale = g.read::<f32>(w(0x4a));
    let o = crate::s4_misc::start_object_at(
        g,
        img,
        img.rel32_target(f + 0x5b),
        w(0x56),
        scale,
        4,
        0,
        0,
    );
    let (src, dst) = (w(0x62), w(0x6c));
    for k in 0..4u32 {
        let v = g.read::<u32>(src + 4 * k);
        g.write::<u32>(dst + 4 * k, v);
    }
    let v = g.read::<u32>(s + 0x2868);
    g.write::<u32>(w(0x7c), v);
    g.write::<u32>(w(0x82), w(0x86));
    g.write::<u32>(w(0x8c), w(0x90));
    Ret::eax(o)
}

/// [`third_restart`] (150 bytes, 1 overlay).
pub const THIRD_RESTART: Sig = Sig {
    len: 150,
    hash: 0x400e_641e_55b0_7a1d,
    calls: &[
        (0x12, Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
        (0x5b, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
    ],
    imports: &[],
};
