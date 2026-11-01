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

/// A motion sequence task (cdecl `(task)`; the state `S = task[1]`, its
/// object `O = S[0x3c]`, the character `c = O[0x50]`): by the step
/// `S+0x18` (a jump table over 0..0x15):
/// - 0: `MsEffectSetMotion(c, 0xba)` (slot 228), the step goes up;
/// - 1: once `MsEffectCheckMotion(c)` (slot 411) answers non-zero,
///   `MsEffectSetMotion(c, 0xbb)`, the step goes up and the frame count
///   `S+0x34` starts again;
/// - 2: from frame 0x59 on, step 10 (`MsEffectSetMotion(c, 0xbc)`, the
///   step goes up);
/// - 11: once the motion ends, step 20 (`MsEffectSetMotion(c, 0xbe)`);
/// - 21: once the motion ends, the task's node leaves its ring and the
///   step becomes 0x4e20;
/// - any other: nothing.
///
/// The frame count goes up on every path. Returns slot 228's `eax`, the
/// node's re-read `+0x4` after the unlink, 0 when the motion runs on, or
/// what the dispatch left (the step above 0x15, else its table index).
///
/// `magic_0387` 0x100088d0 (350 bytes with its jump table and index
/// bytes, 1 DLL; stored in a task's `+0x10`).
pub fn motion_sequence_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let task = fr.arg(g, 0);
    let s = g.read::<u32>(task.wrapping_add(4));
    let o = g.read::<u32>(s.wrapping_add(0x3c));
    let step_at = s.wrapping_add(0x18);
    let frames = s.wrapping_add(0x34);
    let chr = |g: &G| g.read::<u32>(o.wrapping_add(0x50));
    let bump = |g: &mut G, a: u32| {
        let v = g.read::<u32>(a).wrapping_add(1);
        g.write::<u32>(a, v);
    };
    // MsEffectSetMotion(c, motion), the step up, the frame count up
    let set_motion = |g: &mut G, table: u32, motion: u32| {
        let c = chr(g);
        let t = slot_target(g, w(table), 228);
        let eax = g.call(t, fr.ecx, &[c, motion]).eax;
        bump(g, step_at);
        eax
    };
    let check_motion = |g: &mut G, table: u32| {
        let c = chr(g);
        let t = slot_target(g, w(table), 411);
        g.call(t, fr.ecx, &[c]).eax
    };
    let mut step = g.read::<u32>(step_at);
    let eax = loop {
        if step > 0x15 {
            break step;
        }
        // movzx eax, byte [INDEX + step]; jmp [CASES + 4 eax] (both in
        // the code section: read from the image)
        let index = img.word(w(0x23).wrapping_add(step)) & 0xff;
        match index {
            0 => break set_motion(g, 0x79, 0xba),
            1 => {
                let r = check_motion(g, 0x97);
                if r == 0 {
                    break r;
                }
                let eax = set_motion(g, 0xae, 0xbb);
                g.write::<u32>(frames, 0);
                break eax;
            }
            2 => {
                if g.read::<i32>(frames) < 0x59 {
                    break index;
                }
                g.write::<u32>(step_at, 0xa);
            }
            3 => break set_motion(g, 0x79, 0xbc),
            4 => {
                let r = check_motion(g, 0x42);
                if r == 0 {
                    break r;
                }
                g.write::<u32>(step_at, 0x14);
            }
            5 => break set_motion(g, 0x79, 0xbe),
            6 => {
                let r = check_motion(g, 0xe6);
                if r == 0 {
                    break r;
                }
                let node = g.read::<u32>(task);
                let prev = g.read::<u32>(node.wrapping_add(4));
                let next = g.read::<u32>(node.wrapping_add(8));
                g.write::<u32>(prev.wrapping_add(8), next);
                let next = g.read::<u32>(node.wrapping_add(8));
                let prev = g.read::<u32>(node.wrapping_add(4));
                g.write::<u32>(next.wrapping_add(4), prev);
                g.write::<u32>(node.wrapping_add(8), node);
                g.write::<u32>(node.wrapping_add(4), node);
                g.write::<u32>(step_at, 0x4e20);
                break prev;
            }
            _ => break index,
        }
        step = g.read::<u32>(step_at);
    };
    bump(g, frames);
    Ret::eax(eax)
}

/// [`motion_sequence_task`].
pub const MOTION_SEQUENCE_TASK: Sig = Sig {
    len: 350,
    hash: 0xa335_8dac_e5ef_60e4,
    calls: &[],
    imports: &[],
};

/// Unlinks the task's own node from its ring (`node = task[0]`: its
/// neighbours `+0x4`/`+0x8` linked past it, then it points at itself).
/// Returns the node's `+0x4` as the original re-reads it last.
fn unring_task<G: Game>(g: &mut G, task: u32) -> u32 {
    let node = g.read::<u32>(task);
    let prev = g.read::<u32>(node.wrapping_add(4));
    let next = g.read::<u32>(node.wrapping_add(8));
    g.write::<u32>(prev.wrapping_add(8), next);
    let next = g.read::<u32>(node.wrapping_add(8));
    let prev = g.read::<u32>(node.wrapping_add(4));
    g.write::<u32>(next.wrapping_add(4), prev);
    g.write::<u32>(node.wrapping_add(8), node);
    g.write::<u32>(node.wrapping_add(4), node);
    prev
}

/// Writes `words` at `at` (a set-up record on the frame).
fn record<G: Game>(g: &mut G, at: u32, words: &[u32]) {
    for (i, &x) in (0u32..).zip(words) {
        g.write::<u32>(at + 4 * i, x);
    }
}

/// Appends model node `n` (`+0x4` its list sentinel `L`, last `*L_LAST`)
/// to the list: `n+0x1a` = 0xff (u16), `n+0x10`/`+0xc` = `n`, `n+0x14`/
/// `+0x4` = `L`, `n+0x8` = the old last, which links to `n` (`+0x4`, or
/// `L+0xc` when it is `L`); the count (u16) up, `*L_LAST = n`, `n+0x0` the
/// vtable. `ops` are the operand offsets in order: `L_LAST` read, then
/// the stores' addresses and values as the code has them.
fn append_model<G: Game>(g: &mut G, img: &Image, f: u32, ops: &AppendOps) {
    let w = |o: u32| op(img, f, o);
    let last = g.read::<u32>(w(ops.last));
    g.write::<u16>(w(ops.pass), 0xff);
    for (a, v) in ops.stores {
        g.write::<u32>(w(a), w(v));
    }
    g.write::<u32>(w(ops.prev), last);
    if last == w(ops.sentinel) {
        g.write::<u32>(w(ops.first), w(ops.first_v));
    } else {
        g.write::<u32>(last.wrapping_add(4), w(ops.next_v));
    }
}

/// The operand offsets [`append_model`] reads.
struct AppendOps {
    last: u32,
    pass: u32,
    stores: [(u32, u32); 4],
    prev: u32,
    sentinel: u32,
    first: u32,
    first_v: u32,
    next_v: u32,
}

/// A two-model summon task (cdecl `(task)`; the state `S = task[1]`,
/// a frame aligned to 8 with a set-up record on it). By the step
/// `S+0x18`:
/// - 0: the effect's model node `M` set up
///   ([`model_node_init`](crate::sa_summon::model_node_init) on `{M,
///   LIST, *G_OBJ, 0}`, `M+0` its vtable), the step 10, then as 10;
/// - 10: the two parts `A` and `B` are appended to the list (each `+0x1a`
///   0xff, an empty child list, the list's last), `A`'s point from the
///   character (slot 237 `Ch_ElemGetPos(*G_CHR, 0, 0, A+0x1c)`, `w` 1.0),
///   `B`'s colour `(1, 1, 1, 0.099)`; the node `N` inserted
///   ([`node_insert`](crate::nodes::node_insert)), the particle node
///   ([`particle_node`](crate::sb_nodes::particle_node)) and the effect
///   node ([`effect_node`](crate::sb_nodes::effect_node), its node's
///   vtable set, flag bit 0, a word cleared) set up, `M` faded to 0 over
///   0x1e ([`chr_fade`](crate::l3_common::chr_fade)); the frame count
///   `S+0x2c` 0 and the step 0x2710. Returns the fade's `eax`;
/// - 0x2710: once bit 2 of `G_FLAGS` is set, slot 695 `(*G_CHR)`,
///   `G_DONE` 2, the list's current node unlinked
///   ([`list_unlink_current`](crate::l3_common::list_unlink_current)),
///   `M`'s model set up ([`model_setup`](crate::sa_summon::model_setup))
///   and the task's node leaves its ring (returning its `+0x4`);
/// - any other: nothing (returning the step).
///
/// The counts `S+0x28` and `S+0x2c` go up on every path.
///
/// `magic_0310` 0x10006570 (785 bytes, 1 DLL; stored at 0x10006a00).
#[allow(clippy::too_many_lines)] // the original's set-up, store by store
pub fn two_model_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let task = fr.arg(g, 0);
    let s = g.read::<u32>(task.wrapping_add(4));
    let step = g.read::<u32>(s.wrapping_add(0x18));
    // push ebp; and esp, -8; sub esp, 0x20; push esi; push edi
    let e = fr.esp.wrapping_sub(4) & !7;
    let body = e.wrapping_sub(0x28);
    let rec = body + 0x10;
    let callee = |at_off: u32, ecx: u32| Frame {
        at: img.rel32_target(f + at_off),
        esp: body.wrapping_sub(4),
        ecx,
        ..*fr
    };
    let eax = match step {
        0 | 0xa => {
            if step == 0 {
                let obj = g.read::<u32>(w(0x96));
                record(g, rec, &[w(0xa2), w(0xaa), obj]);
                g.write::<u8>(rec + 0xc, 0);
                g.write::<u32>(w(0xb9), w(0xbd));
                crate::sa_summon::model_node_init(g, img, img.rel32_target(f + 0xc2), rec);
                g.write::<u32>(s.wrapping_add(0x18), 0xa);
            }
            append_model(
                g,
                img,
                f,
                &AppendOps {
                    last: 0xce,
                    pass: 0xd5,
                    stores: [(0xdd, 0xe1), (0xe7, 0xeb), (0xf1, 0xf5), (0xfb, 0xff)],
                    prev: 0x104,
                    sentinel: 0x109,
                    first: 0x111,
                    first_v: 0x115,
                    next_v: 0x11e,
                },
            );
            let n = g.read::<u16>(w(0x12a)).wrapping_add(1);
            g.write::<u16>(w(0x12a), n);
            g.write::<u32>(w(0x13f), w(0x143));
            g.write::<u32>(w(0x149), w(0x14d));
            let chr = g.read::<u32>(w(0x139));
            let t = slot_target(g, w(0x123), 237);
            g.call(t, fr.ecx, &[chr, 0, 0, w(0x12f)]);
            g.write::<u32>(w(0x163), 0x3f80_0000);
            append_model(
                g,
                img,
                f,
                &AppendOps {
                    last: 0x15a,
                    pass: 0x16e,
                    stores: [
                        (0x176, 0x17a),
                        (0x180, 0x184),
                        (0x18a, 0x18e),
                        (0x194, 0x198),
                    ],
                    prev: 0x19d,
                    sentinel: 0x1a2,
                    first: 0x1aa,
                    first_v: 0x1ae,
                    next_v: 0x1b7,
                },
            );
            let n = g.read::<u16>(w(0x1be)).wrapping_add(1);
            g.write::<u16>(w(0x1be), n);
            g.write::<u32>(w(0x1c8), w(0x1cc));
            g.write::<u32>(w(0x1d2), w(0x1d6));
            for (o, v) in [
                (0x1dcu32, 0x3f80_0000u32),
                (0x1e6, 0x3f80_0000),
                (0x1f0, 0x3f80_0000),
                (0x1fa, 0x3dca_c084),
            ] {
                g.write::<u32>(w(o), v);
            }
            let words = [0x206, 0x20e, 0x216, 0x21e, 0x226, 0x22e].map(w);
            record(g, rec, &words);
            crate::nodes::node_insert(g, img, &callee(0x233, rec));
            let x = g.read::<u32>(w(0x238));
            g.write::<u8>(w(0x242), 1);
            record(g, rec, &[w(0x24b), w(0x253), w(0x25b), x, w(0x267), 0]);
            g.write::<u32>(w(0x275), w(0x279));
            crate::sb_nodes::particle_node(g, img, img.rel32_target(f + 0x27e), rec);
            let words = [w(0x28a), w(0x292), w(0x29a), w(0x2a2), w(0x2aa), 0];
            record(g, rec, &words);
            crate::sb_nodes::effect_node(g, img, img.rel32_target(f + 0x2b7), rec);
            let node = g.read::<u32>(rec);
            g.write::<u32>(node, w(0x2c5));
            let fl = g.read::<u32>(w(0x2cb)) | 1;
            g.write::<u32>(w(0x2cb), fl);
            g.write::<u32>(w(0x2d2), 0);
            record(g, rec, &[w(0x2de), 0, 0x1e]);
            let eax = crate::l3_common::chr_fade(g, img, img.rel32_target(f + 0x2f3), rec);
            g.write::<u32>(s.wrapping_add(0x2c), 0);
            g.write::<u32>(s.wrapping_add(0x18), 0x2710);
            eax
        }
        0x2710 if g.read::<u8>(w(0x2e)) & 4 != 0 => {
            let chr = g.read::<u32>(w(0x40));
            let t = slot_target(g, w(0x3a), 695);
            let eax = g.call(t, fr.ecx, &[chr]).eax;
            g.write::<u32>(w(0x51), 2);
            crate::l3_common::list_unlink_current(g, img, img.rel32_target(f + 0x5a), eax);
            let cursor = body + 0xc;
            g.write::<u32>(cursor, w(0x66));
            crate::sa_summon::model_setup(g, img, &callee(0x6b, cursor));
            let eax = unring_task(g, task);
            bump2(g, s);
            return Ret::eax(eax);
        }
        _ => step,
    };
    bump2(g, s);
    Ret::eax(eax)
}

/// `S+0x28` and `S+0x2c` up by one.
fn bump2<G: Game>(g: &mut G, s: u32) {
    for o in [0x28u32, 0x2c] {
        let v = g.read::<u32>(s.wrapping_add(o)).wrapping_add(1);
        g.write::<u32>(s.wrapping_add(o), v);
    }
}

/// [`two_model_task`] (the `rel32`s: the unlink, the model set-up, the
/// model node, the insert, the particle node, the effect node, the fade).
pub const TWO_MODEL_TASK: Sig = Sig {
    len: 785,
    hash: 0xaaa0_7139_beee_5b5e,
    calls: &[
        (0x5a, Callee::Sig(&crate::l3_common::LIST_UNLINK_CURRENT)),
        (0x6b, Callee::Sig(&crate::sa_summon::MODEL_SETUP)),
        (0xc2, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0x233, Callee::Sig(&crate::nodes::INSERT)),
        (0x27e, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x2b7, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
        (0x2f3, Callee::Sig(&crate::l3_common::CHR_FADE)),
    ],
    imports: &[],
};

/// `S+0x2c` (the frame count) up by one.
fn bump_frames<G: Game>(g: &mut G, s: u32) {
    let v = g.read::<u32>(s.wrapping_add(0x2c)).wrapping_add(1);
    g.write::<u32>(s.wrapping_add(0x2c), v);
}

/// The operand offsets of a [`centred_pair`].
struct PairOps {
    /// (`rel32`, node, list) of the first append.
    append_a: (u32, u32, u32),
    hide: u32,
    /// (table, the part's point, the character) of slot 366.
    centre: (u32, u32, u32),
    w_one: u32,
    append_b: (u32, u32, u32),
    colour: [(u32, u32); 4],
    /// The node, its parent, the first part, the part's owner.
    insert: [u32; 4],
    insert_call: u32,
    flag: u32,
    /// The node, its list, its parent node, the owner global, the part.
    particle: [u32; 5],
    index: u32,
    vtable: (u32, u32),
    particle_call: u32,
}

/// A centred part pair of [`centred_parts_task`]: two parts appended to
/// the list ([`node_append`](crate::sb_misc::node_append)), the first
/// placed on the character's centre unless bit 0 of `G_HIDE` is set (slot
/// 366 `MsGetChrCenterPos(*G_CHR, A+0x1c)`), the second given `w` 1.0
/// first and its colour; then the node inserted with a record whose last
/// word is the second part (the `ecx` the append leaves) and the particle
/// node set up. `between` runs after the first part is placed (the second
/// pair's scale step). Returns the particle node's `eax`.
fn centred_pair<G: Game>(
    g: &mut G,
    img: &Image,
    fr: &Frame,
    rec: u32,
    o: &PairOps,
    between: impl FnOnce(&mut G),
) -> u32 {
    let f = fr.at;
    let w = |x: u32| op(img, f, x);
    let append = |g: &mut G, (call, node, list): (u32, u32, u32)| {
        crate::sb_misc::node_append(g, img, img.rel32_target(f + call), w(node), w(list), 0);
    };
    append(g, o.append_a);
    if g.read::<u8>(w(o.hide)) & 1 == 0 {
        let chr = g.read::<u32>(w(o.centre.2));
        let t = slot_target(g, w(o.centre.0), 366);
        g.call(t, fr.ecx, &[chr, w(o.centre.1)]);
    }
    between(g);
    g.write::<u32>(w(o.w_one), 0x3f80_0000);
    append(g, o.append_b);
    // mov [esp + 0x24], ecx: the append leaves the part in ecx
    g.write::<u32>(rec + 0x14, w(o.append_b.1));
    for (a, v) in o.colour {
        g.write::<u32>(w(a), v);
    }
    // mov [esp + 0x14], edx: the append leaves the list in edx
    let words = [
        w(o.insert[0]),
        w(o.append_b.2),
        w(o.insert[1]),
        w(o.insert[2]),
        w(o.insert[3]),
    ];
    record(g, rec, &words);
    let ins = Frame {
        at: img.rel32_target(f + o.insert_call),
        esp: rec.wrapping_sub(0x14),
        ecx: rec,
        ..*fr
    };
    crate::nodes::node_insert(g, img, &ins);
    let owner = g.read::<u32>(w(o.particle[3]));
    g.write::<u8>(w(o.flag), 1);
    let words = [
        w(o.particle[0]),
        w(o.particle[1]),
        w(o.particle[2]),
        owner,
        w(o.particle[4]),
        o.index,
    ];
    record(g, rec, &words);
    g.write::<u32>(w(o.vtable.0), w(o.vtable.1));
    crate::sb_nodes::particle_node(g, img, img.rel32_target(f + o.particle_call), rec)
}

/// A summon task with centred parts (cdecl `(task)`; the state `S =
/// task[1]`, a frame aligned to 8 with a set-up record on it). By the
/// step `S+0x18`:
/// - 0: the model nodes ([`model_nodes`](crate::s3_summon::model_nodes)
///   on `{G, LIST, *G_ACTOR, 0, 1}`) and the effect's model node `M`
///   ([`model_node_init`](crate::sa_summon::model_node_init)), step 10,
///   then as 10;
/// - 10: `MsEffectSetMotion(*G_CHR, 0x4a)` (slot 228), the first part
///   pair ([`centred_pair`]), the effect level's low two bits cleared
///   (slots 7 and 8), the effect node set up
///   ([`effect_node`](crate::sb_nodes::effect_node): its vtable, flag
///   bit 0, a word cleared), step 0x1e (returning the effect node's
///   record word);
/// - 0x1e: from frame 0xaa on, `M` faded to 0 over 10
///   ([`chr_fade`](crate::l3_common::chr_fade)), step 0x3c;
/// - 0x3c: from frame 0 on, the second pair (its scale `+= K` between),
///   the step up;
/// - 0x3d: from frame 0xc8 on, step 0x2710;
/// - 0x2710: the list's current node unlinked
///   ([`list_unlink_current`](crate::l3_common::list_unlink_current)),
///   `M`'s model set up ([`model_setup`](crate::sa_summon::model_setup))
///   and the task's node leaves its ring;
/// - any other: nothing.
///
/// The frame count `S+0x2c` goes up on every path (after being cleared
/// where a step starts). Returns the last callee's `eax`, the node's
/// `+0x4` after the unlink, or the step.
///
/// `magic_0257` 0x10006690 (1039 bytes, 1 DLL; stored at 0x10006c10).
#[allow(clippy::too_many_lines)] // the original's steps, store by store
pub fn centred_parts_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let task = fr.arg(g, 0);
    let s = g.read::<u32>(task.wrapping_add(4));
    let step_at = s.wrapping_add(0x18);
    let frames = s.wrapping_add(0x2c);
    let step = g.read::<u32>(step_at);
    // push ebp; and esp, -8; sub esp, 0x20; push esi; push edi
    let e = fr.esp.wrapping_sub(4) & !7;
    let body = e.wrapping_sub(0x28);
    let rec = body + 0x10;
    let callee = |at_off: u32, ecx: u32| Frame {
        at: img.rel32_target(f + at_off),
        esp: body.wrapping_sub(4),
        ecx,
        ..*fr
    };
    // a step starts: the frame count 0, then up
    let restart = |g: &mut G, next: u32| {
        g.write::<u32>(frames, 1);
        g.write::<u32>(step_at, next);
    };
    let eax = match step {
        0 | 0xa => {
            if step == 0 {
                let actor = g.read::<u32>(w(0x7f));
                record(g, rec, &[w(0x8b), w(0x93), actor, 0]);
                g.write::<u8>(rec + 0x10, 1);
                g.write::<u32>(w(0xaa), w(0xae));
                crate::s3_summon::model_nodes(g, img, &callee(0xb3, rec));
                let x = g.read::<u32>(w(0xb8));
                record(g, rec, &[w(0xc4), w(0xcc), x]);
                g.write::<u8>(rec + 0xc, 0);
                g.write::<u32>(w(0xdb), w(0xdf));
                crate::sa_summon::model_node_init(g, img, img.rel32_target(f + 0xe4), rec);
                g.write::<u32>(step_at, 0xa);
            }
            let chr = g.read::<u32>(w(0xf8));
            let t = slot_target(g, w(0xf0), 228);
            g.call(t, fr.ecx, &[chr, 0x4a]);
            let first = PairOps {
                append_a: (0x112, 0x10d, 0x108),
                hide: 0x118,
                centre: (0x120, 0x125, 0x12b),
                w_one: 0x146,
                append_b: (0x14f, 0x140, 0x13b),
                colour: [
                    (0x15d, 0x3f80_0000),
                    (0x167, 0x3f80_0000),
                    (0x171, 0x3f80_0000),
                    (0x17b, 0x3e85_1eb8),
                ],
                insert: [0x187, 0x193, 0x19b, 0x1a3],
                insert_call: 0x1a8,
                flag: 0x1b7,
                particle: [0x1c0, 0x1c8, 0x1d0, 0x1ad, 0x1dc],
                index: 0,
                vtable: (0x1ea, 0x1ee),
                particle_call: 0x1f3,
            };
            centred_pair(g, img, fr, rec, &first, |_| {});
            let c = g.read::<u32>(w(0x1fe));
            let t = slot_target(g, w(0x1f8), 7);
            let level = g.call(t, fr.ecx, &[c]).eax & !3;
            let c = g.read::<u32>(w(0x215));
            let t = slot_target(g, w(0x20f), 8);
            g.call(t, fr.ecx, &[c, level]);
            let words = [w(0x229), w(0x231), w(0x239), w(0x241), w(0x249), 0];
            record(g, rec, &words);
            crate::sb_nodes::effect_node(g, img, img.rel32_target(f + 0x256), rec);
            let node = g.read::<u32>(rec);
            g.write::<u32>(node, w(0x260));
            let fl = g.read::<u32>(w(0x266)) | 1;
            g.write::<u32>(w(0x266), fl);
            g.write::<u32>(w(0x26d), 0);
            restart(g, 0x1e);
            return Ret::eax(node);
        }
        0x1e if g.read::<i32>(frames) >= 0xaa => {
            record(g, rec, &[w(0x4e), 0, 0xa]);
            let eax = crate::l3_common::chr_fade(g, img, img.rel32_target(f + 0x63), rec);
            restart(g, 0x3c);
            return Ret::eax(eax);
        }
        0x3c if g.read::<i32>(frames) >= 0 => {
            let second = PairOps {
                append_a: (0x2a1, 0x29c, 0x297),
                hide: 0x2a7,
                centre: (0x2af, 0x2b4, 0x2ba),
                w_one: 0x2e5,
                append_b: (0x2f6, 0x2df, 0x2da),
                colour: [
                    (0x304, 0x3f80_0000),
                    (0x30e, 0x3f80_0000),
                    (0x318, 0x3f80_0000),
                    (0x322, 0x3fe6_6666),
                ],
                insert: [0x32e, 0x33a, 0x342, 0x34a],
                insert_call: 0x34f,
                flag: 0x35e,
                particle: [0x367, 0x36f, 0x377, 0x354, 0x383],
                index: 1,
                vtable: (0x391, 0x395),
                particle_call: 0x39a,
            };
            let eax = centred_pair(g, img, fr, rec, &second, |g| {
                let x = crate::sb_sse::add(g.read::<f32>(w(0x2cd)), g.read::<f32>(w(0x2d5)));
                // stored after `w` 1.0 in the original: other words, so the
                // order is not visible
                g.write::<f32>(w(0x2f1), x);
            });
            let next = g.read::<u32>(step_at).wrapping_add(1);
            restart(g, next);
            return Ret::eax(eax);
        }
        0x3d => {
            if g.read::<i32>(frames) >= 0xc8 {
                g.write::<u32>(step_at, 0x2710);
            }
            step
        }
        0x2710 => {
            crate::l3_common::list_unlink_current(g, img, img.rel32_target(f + 0x3be), step);
            let cursor = body + 0xc;
            g.write::<u32>(cursor, w(0x3ca));
            crate::sa_summon::model_setup(g, img, &callee(0x3cf, cursor));
            let eax = unring_task(g, task);
            bump_frames(g, s);
            return Ret::eax(eax);
        }
        _ => step,
    };
    bump_frames(g, s);
    Ret::eax(eax)
}

/// [`centred_parts_task`] (the `rel32`s: the fade, the model nodes, the
/// model node, the appends, the inserts, the particle nodes, the effect
/// node, the unlink and the model set-up).
pub const CENTRED_PARTS_TASK: Sig = Sig {
    len: 1039,
    hash: 0x2fb3_b091_cfa8_875d,
    calls: &[
        (0x63, Callee::Sig(&crate::l3_common::CHR_FADE)),
        (0xb3, Callee::Sig(&crate::s3_summon::MODEL_NODES)),
        (0xe4, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0x112, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x14f, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x1a8, Callee::Sig(&crate::nodes::INSERT)),
        (0x1f3, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x256, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
        (0x2a1, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x2f6, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x34f, Callee::Sig(&crate::nodes::INSERT)),
        (0x39a, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x3be, Callee::Sig(&crate::l3_common::LIST_UNLINK_CURRENT)),
        (0x3cf, Callee::Sig(&crate::sa_summon::MODEL_SETUP)),
    ],
    imports: &[],
};

/// Appends node `n` to the list `l` (as the summon code does inline):
/// `n+0x1a` 0xff (u16), an empty child list (`+0x10`/`+0xc` = `n`), the
/// list `+0x14`/`+0x4`, the list's last `+0x8`, which links to `n`
/// (`+0x4`, or the list's `+0xc` when it is the list itself), the count
/// `l+0x18` (u16) up and `l+0x10 = n`.
fn ring_append<G: Game>(g: &mut G, n: u32, l: u32) {
    let last = g.read::<u32>(l + 0x10);
    g.write::<u16>(n + 0x1a, 0xff);
    g.write::<u32>(n + 0x10, n);
    g.write::<u32>(n + 0xc, n);
    g.write::<u32>(n + 0x14, l);
    g.write::<u32>(n + 4, l);
    g.write::<u32>(n + 8, last);
    if last == l {
        g.write::<u32>(l + 0xc, n);
    } else {
        g.write::<u32>(last.wrapping_add(4), n);
    }
    let count = g.read::<u16>(l + 0x18).wrapping_add(1);
    g.write::<u16>(l + 0x18, count);
    g.write::<u32>(l + 0x10, n);
}

/// A summon task with a shade fade (cdecl `(task)`; the state `S =
/// task[1]`, a frame aligned to 8 with set-up records on it; the steps
/// through a jump table and an index table over 0..0x3d, 0x2710 apart).
/// By the step `S+0x18`:
/// - 0: the effect's model node `M`
///   ([`model_node_init`](crate::sa_summon::model_node_init) on `{M,
///   LIST, *G_OBJ, 0}`) and the part `A` appended to the list (`A+0x28`
///   1.0, `+0x3c` the character `*G_CHR`, `+0x40` 1, `+0x2c..+0x38` 0, its
///   vtable), step 10, then as 10;
/// - 10: the nodes `C = S+0x74` and `D = S+0xa0` appended (`C`'s colour
///   `(1, 1, 1, 0.2)` at `S+0x90`; `D` takes `C`, `A`, `G_B`, `G_N` and
///   byte 0 at `+0x140`, `+0x13c`, `+0x144`, `+0x128`, `+0x148`), the
///   particle node `P = S+0x1f0`
///   ([`particle_node`](crate::sb_nodes::particle_node)), the node `E`
///   appended with slot 719's answer `(PROG, 6, 0)` and its words; step
///   0x1e, dispatched again at once;
/// - 0x1e: `Ch_SetShadeCol(*G_CHR, 1, 0.4, 0.4, 0xf)` (slot 256), the
///   step up, the frame count restarted;
/// - 0x1f..0x23: the same from frame 0xf on with the colours `(1, 1,
///   0.4)`, `(0.4, 1, 0.4)`, `(0.4, 1, 1)`, `(0.4, 0.4, 1)`, `(1, 0.4,
///   1)`;
/// - 0x24: from frame 0xf on `(1, 1, 1)`, step 0x2710 (the frame count
///   not restarted);
/// - 0x3c: from frame 0 on, the step up and the count restarted;
/// - 0x3d: from frame 0xc8 on, step 0x2710;
/// - 0x2710: the list's current node unlinked
///   ([`list_unlink_current`](crate::l3_common::list_unlink_current)),
///   `M`'s model set up ([`model_setup`](crate::sa_summon::model_setup))
///   and the task's node leaves its ring;
/// - any other: nothing.
///
/// The frame count `S+0x2c` goes up on every path. Returns slot 256's
/// `eax`, the node's `+0x4` after the unlink, the table index on a path
/// that does nothing (a gate not met, 9 and 10 for 0x3c and 0x3d), or the
/// step outside the table.
///
/// `magic_0342` 0x10006c40 (1382 bytes with its two tables, 1 DLL;
/// stored at 0x10007320).
#[allow(clippy::too_many_lines)] // the original's set-up, store by store
pub fn shade_fade_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let task = fr.arg(g, 0);
    let s = g.read::<u32>(task.wrapping_add(4));
    let step_at = s.wrapping_add(0x18);
    let frames = s.wrapping_add(0x2c);
    // push ebp; and esp, -8; sub esp, 0x30; push esi; push edi
    let e = fr.esp.wrapping_sub(4) & !7;
    let body = e.wrapping_sub(0x38);
    let list = w(0x57);
    let shade = |g: &mut G, (table, chr): (u32, u32), [r, gr, b]: [u32; 3]| {
        let c = g.read::<u32>(w(chr));
        let t = slot_target(g, w(table), 256);
        g.call(t, fr.ecx, &[c, r, gr, b, 0xf]).eax
    };
    const ONE: u32 = 0x3f80_0000;
    const K: u32 = 0x3ecc_cccd;
    let mut step = g.read::<u32>(step_at);
    let eax = loop {
        if step == 0x2710 {
            crate::l3_common::list_unlink_current(g, img, img.rel32_target(f + 0x4be), step);
            let cursor = body + 0xc;
            g.write::<u32>(cursor, w(0x4ca));
            let setup = Frame {
                at: img.rel32_target(f + 0x4cf),
                esp: body.wrapping_sub(4),
                ecx: cursor,
                ..*fr
            };
            crate::sa_summon::model_setup(g, img, &setup);
            break unring_task(g, task);
        }
        if step > 0x3d {
            break step;
        }
        // movzx eax, byte [INDEX + step]: the index table is in the code
        let index = img.word(w(0x37).wrapping_add(step)) & 0xff;
        let gate = |g: &G, at_least: i32| g.read::<i32>(frames) >= at_least;
        let colours = match index {
            0 | 1 => {
                if index == 0 {
                    let rec = body + 0x10;
                    let obj = g.read::<u32>(w(0x43));
                    record(g, rec, &[w(0x4f), list, obj]);
                    g.write::<u8>(rec + 0xc, 0);
                    g.write::<u32>(w(0x66), w(0x6a));
                    crate::sa_summon::model_node_init(g, img, img.rel32_target(f + 0x6f), rec);
                    let chr = g.read::<u32>(w(0x7a));
                    let a = w(0x8d);
                    ring_append(g, a, list);
                    g.write::<u32>(a + 0x28, ONE);
                    g.write::<u32>(a + 0x3c, chr);
                    g.write::<u32>(a + 0x40, 1);
                    for o in [0x2cu32, 0x30, 0x34, 0x38] {
                        g.write::<u32>(a + o, 0);
                    }
                    g.write::<u32>(a, w(0x127));
                    g.write::<u32>(step_at, 0xa);
                }
                let c = s.wrapping_add(0x74);
                ring_append(g, c, list);
                g.write::<u32>(c, w(0x17d));
                for (o, v) in [
                    (0x90u32, ONE),
                    (0x94, ONE),
                    (0x98, ONE),
                    (0x9c, 0x3e4c_cccd),
                ] {
                    g.write::<u32>(s.wrapping_add(o), v);
                }
                let d = s.wrapping_add(0xa0);
                g.write::<u32>(d, w(0x1b1));
                ring_append(g, d, list);
                let owner = g.read::<u32>(w(0x1f6));
                g.write::<u32>(d + 0x140, c);
                g.write::<u32>(d + 0x13c, w(0x212));
                g.write::<u32>(d + 0x144, w(0x21c));
                g.write::<u32>(d + 0x128, w(0x226));
                g.write::<u8>(d + 0x148, 0);
                let p = s.wrapping_add(0x1f0);
                let rec = body + 0x20;
                g.write::<u32>(p, w(0x237));
                record(g, rec, &[p, w(0x243), d, owner, w(0x253), 0]);
                crate::sb_nodes::particle_node(g, img, img.rel32_target(f + 0x260), rec);
                let n = w(0x278);
                ring_append(g, n, list);
                let t = slot_target(g, w(0x2ba), 719);
                let r = g.call(t, fr.ecx, &[w(0x2da), 6, 0]).eax;
                g.write::<u32>(n + 0x24, r);
                g.write::<u32>(n + 0x1c, w(0x2eb));
                g.write::<u32>(n + 0x20, w(0x2f5));
                g.write::<u32>(n + 0x34, 0x40);
                g.write::<u32>(n + 0x28, 0x100);
                g.write::<u32>(n, w(0x313));
                g.write::<u32>(n + 0x30, 0);
                g.write::<u32>(n + 0x2c, 1);
                g.write::<u32>(step_at, 0x1e);
                step = 0x1e;
                continue;
            }
            2 => [ONE, K, K],
            3 if gate(g, 0xf) => [ONE, ONE, K],
            4 if gate(g, 0xf) => [K, ONE, K],
            5 if gate(g, 0xf) => [K, ONE, ONE],
            6 if gate(g, 0xf) => [K, K, ONE],
            7 if gate(g, 0xf) => [ONE, K, ONE],
            8 if gate(g, 0xf) => {
                let eax = shade(g, (0x44a, 0x472), [ONE, ONE, ONE]);
                g.write::<u32>(step_at, 0x2710);
                break eax;
            }
            9 if gate(g, 0) => {
                let next = g.read::<u32>(step_at).wrapping_add(1);
                g.write::<u32>(step_at, next);
                g.write::<u32>(frames, 0);
                break index;
            }
            10 if gate(g, 0xc8) => {
                g.write::<u32>(step_at, 0x2710);
                break index;
            }
            _ => break index,
        };
        let eax = shade(g, (0x35c, 0x362), colours);
        let next = g.read::<u32>(step_at).wrapping_add(1);
        g.write::<u32>(step_at, next);
        g.write::<u32>(frames, 0);
        break eax;
    };
    bump_frames(g, s);
    Ret::eax(eax)
}

/// [`shade_fade_task`] (the `rel32`s: the model node, the particle node,
/// the unlink and the model set-up).
pub const SHADE_FADE_TASK: Sig = Sig {
    len: 1382,
    hash: 0xb08e_aa2f_ce1c_4acf,
    calls: &[
        (0x6f, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0x260, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x4be, Callee::Sig(&crate::l3_common::LIST_UNLINK_CURRENT)),
        (0x4cf, Callee::Sig(&crate::sa_summon::MODEL_SETUP)),
    ],
    imports: &[],
};
