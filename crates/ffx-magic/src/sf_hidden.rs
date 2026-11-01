//! Functions the first overlay census hid inside merged rows (census2,
//! 2026-11-01, docs/formats/magic.md "The census's functions"): each
//! started right after a ported function, so its row counted as covered.
//! All are callbacks their DLL stores (a code pointer in reachable code),
//! so each is an entry.

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The word at `f + off` of the code (an operand: a global, a table).
fn op(img: &Image, f: u32, off: u32) -> u32 {
    img.word(f + off)
}

/// A part's point from its character's element (cdecl `(p)`): slot 237
/// (`Ch_ElemGetPos`) `(*G_CHR, 0xb, TABLE + 0x10 p[0x120], p + 0xd0)`,
/// then the point's `w` (`p + 0xdc`) 1.0. Returns slot 237's `eax`.
///
/// `magic_0314` 0x10001740 (63 bytes, 1 DLL; stored at 0x100015b7).
pub fn elem_point_table<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let p = fr.arg(g, 0);
    let pt = p.wrapping_add(0xd0);
    let i = g.read::<u32>(p.wrapping_add(0x120));
    // +0x16: add eax, TABLE
    let entry = (i << 4).wrapping_add(op(img, f, 0x17));
    let chr = g.read::<u32>(op(img, f, 0x26));
    let t = slot_target(g, op(img, f, 0x1e), 237);
    let eax = g.call(t, fr.ecx, &[chr, 0xb, entry, pt]).eax;
    g.write::<u32>(pt.wrapping_add(0xc), 0x3f80_0000);
    Ret::eax(eax)
}

/// [`elem_point_table`].
pub const ELEM_POINT_TABLE: Sig = Sig {
    len: 63,
    hash: 0x925c_ca26_b8ec_8656,
    calls: &[],
    imports: &[],
};

/// A part placed on its character's matrix (cdecl `(p)`): slot 621
/// (`Ch_OptposGetMatrix`) `(*G_CHR, 2, 0, p + 0x80)`, the byte `p + 0x126`
/// 2, and the point `p + 0xd0..+0xdc` = `p + 0xb0..+0xbc`. Returns the
/// last word copied.
///
/// `magic_0367` 0x100013a0 (98 bytes, 1 DLL; stored at 0x10001439 and
/// 0x10001469).
pub fn optpos_point<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let p = fr.arg(g, 0);
    let chr = g.read::<u32>(op(img, f, 0x1f));
    let t = slot_target(g, op(img, f, 0xf), 621);
    g.call(t, fr.ecx, &[chr, 2, 0, p.wrapping_add(0x80)]);
    g.write::<u8>(p.wrapping_add(0x126), 2);
    let mut last = 0;
    for i in 0..4 {
        last = g.read::<u32>(p.wrapping_add(0xb0 + 4 * i));
        g.write::<u32>(p.wrapping_add(0xd0 + 4 * i), last);
    }
    Ret::eax(last)
}

/// [`optpos_point`].
pub const OPTPOS_POINT: Sig = Sig {
    len: 98,
    hash: 0x3592_2cd1_986e_ac3a,
    calls: &[],
    imports: &[],
};

/// A part's point from its character's element 0xe with a fixed offset
/// (cdecl `(p)`; a /GS frame): the offset `(300, 400, 0, 1)` in the
/// frame (`[ebp - 0x14]`), slot 237 (`Ch_ElemGetPos`) `(*G_CHR, 0xe,
/// &offset, p + 0xd0)`, then `p + 0xdc` 1.0 and `p + 0x104` 0. Returns
/// slot 237's `eax`.
///
/// `magic_0274` 0x10003760 (117 bytes, 1 DLL; stored at 0x100034fe).
pub fn elem_point_offset<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let p = fr.arg(g, 0);
    let pt = p.wrapping_add(0xd0);
    // ebp = esp - 4; the offset at [ebp - 0x14]
    let off = fr.esp.wrapping_sub(0x18);
    for (i, w) in [0x4396_0000u32, 0x43c8_0000, 0, 0x3f80_0000]
        .into_iter()
        .enumerate()
    {
        g.write::<u32>(off + 4 * i as u32, w);
    }
    let chr = g.read::<u32>(op(img, f, 0x29));
    let t = slot_target(g, op(img, f, 0x21), 237);
    let eax = g.call(t, fr.ecx, &[chr, 0xe, off, pt]).eax;
    g.write::<u32>(pt.wrapping_add(0xc), 0x3f80_0000);
    g.write::<u32>(p.wrapping_add(0x104), 0);
    Ret::eax(eax)
}

/// [`elem_point_offset`] (the `rel32`: `__security_check_cookie`).
pub const ELEM_POINT_OFFSET: Sig = Sig {
    len: 117,
    hash: 0x6ea1_f4b0_e34a_3ea8,
    calls: &[(0x6d, Callee::Any)],
    imports: &[],
};

/// An angle triple turned by its character's spin (cdecl `(p)`): with
/// the character record `c = *G_CHRS + 0x4f0 p[0x390]`, each of `p +
/// 0xc0..+0xc8` gets `c + 0x4d0..` added and is wrapped once (`- T`
/// above `HI`, `+ T` below `LO`; the `comiss`/`jbe` pair skips both for
/// a NaN). Returns `p`.
///
/// `magic_0018` 0x10001b00 and `magic_0019` 0x10004010 (190 bytes;
/// stored in a record's `+0x2ac`).
pub fn spin_angles<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, sub};
    let f = fr.at;
    let k = |g: &G, o: u32| g.read::<f32>(op(img, f, o));
    let p = fr.arg(g, 0);
    let i = g.read::<u32>(p.wrapping_add(0x390));
    let c = i
        .wrapping_mul(0x4f0)
        .wrapping_add(g.read::<u32>(op(img, f, 0x24)));
    // +0x06 HI, +0x14 T, +0x28 LO
    let (hi, turn, lo) = (k(g, 0xa), k(g, 0x18), k(g, 0x2c));
    for j in 0..3 {
        let a = p.wrapping_add(0xc0 + 4 * j);
        let x = add(
            g.read::<f32>(c.wrapping_add(0x4d0 + 4 * j)),
            g.read::<f32>(a),
        );
        let x = if x > hi {
            sub(x, turn)
        } else if lo > x {
            add(x, turn)
        } else {
            x
        };
        g.write::<f32>(a, x);
    }
    Ret::eax(p)
}

/// [`spin_angles`].
pub const SPIN_ANGLES: Sig = Sig {
    len: 190,
    hash: 0x044e_bbef_e925_ad02,
    calls: &[],
    imports: &[],
};

/// A two-step task that shows the caster when its script ends (cdecl
/// `(task)`; the effect's block `B = *G_B`): `memset(R, 0, 0x30)`, u16
/// `R+2 |= 0x80`, `R+0x14 = *G_X`. By `B+0xebf8`:
/// - 0: the script `B+0xebf4` starts (`G_PC = *G_SCRIPTS[i]`, `G_WAIT` 0,
///   u16 `G_FLAGS` 0, byte `G_MODE` 1), the index counts up and the step
///   becomes 1, then as 1;
/// - 1: the script runs ([`run`](crate::sa_script::run)); when it ends
///   (1), with the index at 1 or more `B+0xebfc` = 1, the caster is shown
///   ([`show_caster`](crate::s4_fam446::show_caster)) and the task given
///   back ([`give_back`](crate::sa_pool::give_back)); with the index
///   below 1 the step goes back to 0;
/// - any other step: nothing.
///
/// Every path ends in the motion step (a tail jump,
/// [`motion_step`](crate::s3_motion::motion_step)), whose `eax` is
/// returned.
///
/// `magic_0543` 0x10005770 (193 bytes, 9 DLLs; its address goes to the
/// task set-up in `ecx`).
pub fn caster_script_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let task = fr.arg(g, 0);
    let b = g.read::<u32>(w(0x06));
    // memset(R, 0, 0x30)
    g.write_bytes(w(0x0f), &[0u8; 0x30]);
    let fl = g.read::<u16>(w(0x20)) | 0x80;
    g.write::<u16>(w(0x20), fl);
    let x = g.read::<u32>(w(0x25));
    g.write::<u32>(w(0x2a), x);
    let step = g.read::<u32>(b.wrapping_add(0xebf8));
    let tail = match step {
        0 | 1 => {
            if step == 0 {
                let i = g.read::<u32>(b.wrapping_add(0xebf4));
                let pc = g.read::<u32>(g.read::<u32>(w(0x4f).wrapping_add(i.wrapping_mul(4))));
                g.write::<u32>(w(0x56), pc);
                g.write::<u32>(w(0x5c), 0);
                g.write::<u16>(w(0x67), 0);
                g.write::<u8>(w(0x6f), 1);
                g.write::<u32>(b.wrapping_add(0xebf4), i.wrapping_add(1));
                g.write::<u32>(b.wrapping_add(0xebf8), 1);
            }
            if crate::sa_script::run(g, img, img.rel32_target(f + 0x85)) == 1 {
                if g.read::<i32>(b.wrapping_add(0xebf4)) >= 1 {
                    g.write::<u32>(b.wrapping_add(0xebfc), 1);
                    // push ebp; push esi; call
                    let shown = Frame {
                        at: img.rel32_target(f + 0x9d),
                        esp: fr.esp.wrapping_sub(0xc),
                        ..*fr
                    };
                    crate::s4_fam446::show_caster(g, img, &shown);
                    crate::sa_pool::give_back(g, img, img.rel32_target(f + 0xa5), task);
                    0xac
                } else {
                    g.write::<u32>(b.wrapping_add(0xebf8), 0);
                    0xbd
                }
            } else {
                0xbd
            }
        }
        _ => 0x42,
    };
    let eax = crate::s3_motion::motion_step(g, img, img.rel32_target(f + tail));
    Ret::eax(eax)
}

/// [`caster_script_task`] (the `rel32`s: `memset`, the motion step (three
/// tail jumps), the script run, the caster shown and the give-back).
pub const CASTER_SCRIPT_TASK: Sig = Sig {
    len: 193,
    hash: 0x788b_4f5f_ecb4_854a,
    calls: &[
        (0x14, Callee::Import("memset")),
        (0x42, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0x85, Callee::Sig(&crate::sa_script::RUN)),
        (0x9d, Callee::Sig(&crate::s4_fam446::SHOW_CASTER)),
        (0xa5, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0xac, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0xbd, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
    ],
    imports: &[],
};

/// The summon's frame with slot 673 first
/// ([`summon_frame_b`](crate::s4_summon::summon_frame_b) with the slot
/// call moved before the stage test: slot 673 runs every frame, the rest
/// from stage 4). Returns 0.
///
/// `magic_0426` 0x100077e0 (288 bytes, 1 DLL; a word in the overlay
/// table).
pub fn summon_frame_673_first<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::s4_summon::summon_frame_b_on(g, img, fr, true)
}

/// [`summon_frame_673_first`].
pub const SUMMON_FRAME_673_FIRST: Sig = Sig {
    len: 288,
    hash: 0xd2ee_2eae_62d7_2075,
    calls: &[
        (0x3a, Callee::Sig(&crate::sa_summon::BIND_MODELS)),
        (0xac, Callee::Sig(&crate::sa_summon::BONE)),
        (0xc9, Callee::Sig(&crate::nodes::GATED)),
        (0xe9, Callee::Sig(&crate::nodes::GATED)),
    ],
    imports: &[],
};

/// A fly-in task (cdecl `(task)`): a countdown `G_N` (`+0x04`); while it
/// is above 1 it only counts (returning the count). At 1 it reaches 0 and
/// the end point `P` is taken from the character (slot 237
/// `Ch_ElemGetPos` `(*G_CHR, ELEMS[*G_I], 0, P)`, `P.w` 1.0). Then each
/// frame the mover `V` is put between `A` and `P` at `t = *G_T` (slot 326
/// `FFXVu0InterVectorXYZ` `(V, A, P, t)`), its height `V.y` lowered by
/// the arc `(1 - t) t K2 K3 + (1 - t)^2 0 + t^2 0` (the zero terms kept
/// for the original's rounding), and `t` steps by `KS`. Returns slot
/// 326's `eax` while `t + KS < 1` (or unordered); else the node list
/// `L` loses `V`'s item ([`list_remove`](crate::l5_objhost::list_remove));
/// when one was removed `*G_P = ITEM` and the parts go (slot 713
/// `pppAllFreePObject` on the item, u16 `G_F &= 0x7fff`); then the task
/// is given back ([`give_back`](crate::sa_pool::give_back), a tail jump),
/// whose `eax` is returned.
///
/// `magic_0241` 0x100018b0 and four more (314 bytes, 5 DLLs; stored in a
/// task's `+0x10`).
pub fn fly_in_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let task = fr.arg(g, 0);
    let n = g.read::<i32>(w(0x04));
    if n > 0 {
        let n = n - 1;
        g.write::<i32>(w(0x0e), n);
        if n > 0 {
            return Ret::eax(n.cast_unsigned());
        }
        let i = g.read::<u32>(w(0x1b));
        let elem = g.read::<u32>(w(0x29).wrapping_add(i.wrapping_mul(4)));
        let chr = g.read::<u32>(w(0x34));
        let t = slot_target(g, w(0x2e), 237);
        g.call(t, fr.ecx, &[chr, elem, 0, w(0x20)]);
        g.write::<u32>(w(0x45), 0x3f80_0000);
    }
    let t0 = g.read::<u32>(w(0x51));
    let s = slot_target(g, w(0x56), 326);
    let eax = g.call(s, fr.ecx, &[w(0x71), w(0x6c), w(0x67), t0]).eax;
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = k(g, 0x7b);
    let u = sub(k(g, 0x83), t);
    let mid = mul(mul(mul(t, u), k(g, 0xa0)), k(g, 0xac));
    let ends = mul(mul(u, u), 0.0);
    let tail = mul(mul(t, t), 0.0);
    let next = add(t, k(g, 0xbf));
    let arc = add(add(mid, ends), tail);
    g.write::<f32>(w(0xde), next);
    let y = sub(k(g, 0xd6), arc);
    g.write::<f32>(w(0xea), y);
    // comiss next, ONE; jb: below or unordered
    if !(next >= k(g, 0xca)) {
        return Ret::eax(eax);
    }
    let removed =
        crate::l5_objhost::list_remove(g, img, img.rel32_target(f + 0xfb), w(0xf6), w(0xf1));
    if removed != 0 {
        g.write::<u32>(g.read::<u32>(w(0x104)), w(0x10f));
        let s = slot_target(g, w(0x114), 713);
        g.call(s, fr.ecx, &[w(0x109)]);
        let fl = g.read::<u16>(w(0x12b)) & 0x7fff;
        g.write::<u16>(w(0x12b), fl);
    }
    let eax = crate::sa_pool::give_back(g, img, img.rel32_target(f + 0x134), task);
    Ret::eax(eax)
}

/// [`fly_in_task`] (the `rel32`s: the list removal and the give-back).
pub const FLY_IN_TASK: Sig = Sig {
    len: 314,
    hash: 0x99f7_ee88_64c7_39cc,
    calls: &[
        (0xfb, Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
        (0x134, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
    ],
    imports: &[],
};
