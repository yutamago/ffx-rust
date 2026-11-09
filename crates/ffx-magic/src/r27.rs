//! Round 27's own ports (own rows of `magic_0450` and up): the DLLs' own
//! code written out, and copies of ported functions bound by their own
//! `Sig`.

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

const ONE: u32 = 0x3f80_0000;

/// `magic_0609`'s motion task (0x100093d0, 510 bytes; a task function,
/// cdecl `(task)`, the record `R = task[1]`; an 8-aligned frame with the
/// model-id pair at `A = ((E - 4) & !7) - 8`): the same states as
/// [`motion_task_0613`](crate::r23::motion_task_0613) with other values.
/// By the state `R + 0x18` (a state set here runs at once): 10, 20, 30,
/// 40 and 60 give the object `O = R + 0x1c` the values 0xb8, 0xb9, 0xba,
/// 0xbb, 0xbd ([`set_value`](crate::m0042::set_value)) and step on; 0x32
/// steps on; 11 and 31 wait for `O`'s motion (slot 20 of `O[0x54]` with
/// bit 0 of `O + 0x90`, else slot 411 of `O[0x50]`) and go to 20 and 40;
/// 61 waits for `O`'s model id ([`model_id`](crate::sa_misc::model_id)),
/// gives it 0xbc and steps on; 10000 unlinks the task node. Every way
/// ends with `R + 0x2c` and `R + 0x30` up (a step also the state).
/// Returns the last `eax`: the set's answer, the wait's 0, the state (less
/// 10 below 0x20, the byte table's 5 for its other entries), the node's
/// old `+4` after the unlink.
pub fn motion_task_0609<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    let bump = |g: &mut G, o: u32| {
        let v = g.read::<u32>(r + o).wrapping_add(1);
        g.write::<u32>(r + o, v);
    };
    let end = |g: &mut G, step: bool, eax: u32| {
        if step {
            bump(g, 0x18);
        }
        bump(g, 0x2c);
        bump(g, 0x30);
        Ret::eax(eax)
    };
    let set = |g: &mut G, call: u32, v: u32| {
        let obj = g.read::<u32>(r + 0x1c);
        crate::m0042::set_value(g, img, img.rel32_target(f + call), obj, v)
    };
    let motion = |g: &mut G, o20: u32, o411: u32| {
        let obj = g.read::<u32>(r + 0x1c);
        if g.read::<u8>(obj + 0x90) & 1 != 0 {
            g.call(t(g, o20, 20), 0, &[g.read::<u32>(obj + 0x54)]).eax
        } else {
            g.call(t(g, o411, 411), 0, &[g.read::<u32>(obj + 0x50)]).eax
        }
    };
    loop {
        let state = g.read::<u32>(r + 0x18);
        match state {
            10 => {
                let eax = set(g, 0x106, 0xb8);
                return end(g, true, eax);
            }
            11 | 31 => {
                let (o20, o411, next) = if state == 11 {
                    (0x50u32, 0x5du32, 0x14u32)
                } else {
                    (0x8d, 0x9a, 0x28)
                };
                let eax = motion(g, o20, o411);
                if eax == 0 {
                    return end(g, false, 0);
                }
                g.write::<u32>(r + 0x18, next);
            }
            20 => {
                let eax = set(g, 0x122, 0xb9);
                return end(g, true, eax);
            }
            30 => {
                let eax = set(g, 0x13e, 0xba);
                return end(g, true, eax);
            }
            0x28 => {
                let eax = set(g, 0x15a, 0xbb);
                return end(g, true, eax);
            }
            0x32 => return end(g, true, 0x32),
            0x3c => {
                let eax = set(g, 0xea, 0xbd);
                return end(g, true, eax);
            }
            0x3d => {
                let a = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(8);
                let obj = g.read::<u32>(r + 0x1c);
                g.write::<u32>(a, obj);
                let id = crate::sa_misc::model_id(g, img, img.rel32_target(f + 0x179), a);
                if g.read::<u32>(a + 4) == 0 {
                    return end(g, false, id);
                }
                let eax = set(g, 0x18d, 0xbc);
                return end(g, true, eax);
            }
            0x2710 => {
                let node = g.read::<u32>(task);
                let (prev, next) = (g.read::<u32>(node + 4), g.read::<u32>(node + 8));
                g.write::<u32>(prev + 8, next);
                let (next, prev) = (g.read::<u32>(node + 8), g.read::<u32>(node + 4));
                g.write::<u32>(next + 4, prev);
                g.write::<u32>(node + 8, node);
                g.write::<u32>(node + 4, node);
                return end(g, false, prev);
            }
            // `sub eax, 10` and the byte table: another state below 0x28
            // leaves its table index (5) or the state less 10
            s if (10..0x20).contains(&s) => return end(g, false, 5),
            s if s < 0x28 => return end(g, false, s.wrapping_sub(10)),
            s => return end(g, false, s),
        }
    }
}

/// [`motion_task_0609`] (510 bytes, 1 overlay).
pub const MOTION_TASK_0609: Sig = Sig {
    len: 510,
    hash: 0x534b_a7a3_a8b8_6915,
    calls: &[
        (0xea, Callee::Sig(&crate::m0042::SET_VALUE)),
        (0x106, Callee::Sig(&crate::m0042::SET_VALUE)),
        (0x122, Callee::Sig(&crate::m0042::SET_VALUE)),
        (0x13e, Callee::Sig(&crate::m0042::SET_VALUE)),
        (0x15a, Callee::Sig(&crate::m0042::SET_VALUE)),
        (0x179, Callee::Sig(&crate::sa_misc::MODEL_ID)),
        (0x18d, Callee::Sig(&crate::m0042::SET_VALUE)),
    ],
    imports: &[],
};

/// `magic_0586`'s caster frames (0x10002170, 1491 bytes; internal, no
/// arguments, `esp` the entry stack pointer; a cookie frame, `ebp = E -
/// 4`: the position `P` at `ebp - 0x14` (with `ebp - 8` = 1.0), a vector
/// at `ebp - 0x34`, the offset `ebp - 0x24`, two matrices at `ebp - 0xb4`
/// and `ebp - 0x74`). `S = *G_S`, `A = *G_A`:
/// - `P` = slot 262 `(A)`'s three words, kept at `S + 0x970` (and 1.0);
/// - the turn `a = K - f` (`f` = slot 333 `(A)`, rounded to f32) brought
///   into `-pi..pi` (above `K1`: less `K2`; else below `K3`: plus `K2`;
///   unordered unchanged), at `S + 0x910` as `(0, a, 0, 0)`, `P` at `S +
///   0x920`;
/// - the model matrix `M = S + 0x850`: slot 210 `(A, M)` (with `M` null,
///   which no caller reaches: slots 217 and 255 `(I, ebp - 0x34)` for an
///   index `I = *G_I` below 32 and the battle word `*G_W` 0), slot 211
///   `(A, M)`, 235 `(M)`, the scale slot 277 `(A)` times `K4` on the
///   diagonal, each diagonal word times slots 15, 16, 17 `(A)`, slots 330,
///   328 `(M, M, 0)` and 329 `(M, M, a)`; `S + 0x880` = `S + 0x920`;
/// - slot 238 `(I, 1)`; the caster shown (slot 296 `(I, 0)` for `I`
///   below 32 and `*G_W` 0, else slot 257 `(A, 1)`); slot 8 `(A, 0)`;
///   `S + 0x8568` = `a`;
/// - two points ahead of the caster, `S + 0x980` and `S + 0x990`: `P`
///   moved by `(0, 0, d, 1)` (`d` 15 and 24) turned by `a` (slots 235,
///   328, 329, 229, 248), their `+0xc` 1.0;
/// - the flat mesh coordinates
///   ([`flat_coords_0586`](crate::r25::flat_coords_0586)); `S + 0x9a0..`
///   (12 words), `+0x8560`, `+0x8564`, `+0x858c` 0, three flags 1.
///
/// Returns the coordinates' `eax`.
#[allow(clippy::too_many_lines)] // one straight sequence
pub fn caster_frames_0586<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32) -> u32 {
    use crate::ob_model::st0_f32;
    use crate::sb_sse::{add, mul, sub};
    let w = |o: u32| img.word(f + o);
    let rd = |g: &G, o: u32| g.read::<u32>(w(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let ebp = esp.wrapping_sub(4);
    let l = |off: u32| ebp.wrapping_sub(off);
    let (p, v, off) = (l(0x14), l(0x34), l(0x24));
    let s = rd(g, 0x18);
    // call width: 4
    g.call(t(g, 0x29, 262), 0, &[rd(g, 0x2f), p, l(0x10), l(0xc)]);
    for i in 0..3 {
        let x = g.read::<u32>(p + 4 * i);
        g.write::<u32>(s + 0x970 + 4 * i, x);
    }
    g.write::<u32>(s + 0x97c, ONE);
    g.write::<u32>(l(8), ONE);
    let turn = st0_f32(g.call(t(g, 0x41, 333), 0, &[rd(g, 0x73)]).st0);
    let mut a = sub(k(g, 0x90), turn);
    if a > k(g, 0xa2) {
        a = sub(a, k(g, 0xb4));
    } else if k(g, 0xbe) > a {
        a = add(a, k(g, 0xcb));
    }
    g.write::<u32>(s + 0x910, 0);
    g.write::<f32>(s + 0x914, a);
    g.write::<u32>(s + 0x918, 0);
    g.write::<u32>(s + 0x91c, 0);
    for i in 0..4 {
        let x = g.read::<u32>(p + 4 * i);
        g.write::<u32>(s + 0x920 + 4 * i, x);
    }
    let actor = rd(g, 0x132);
    let m = s.wrapping_add(0x850);
    if m == 0 {
        // `S + 0x850` null: no caller's `S` gets here
        let i = rd(g, 0x165);
        if rd(g, 0x15c) == 0 && i.cast_signed() < 0x20 {
            g.call(t(g, 0x173, 217), 0, &[i, v]);
            g.call(t(g, 0x185, 255), 0, &[i, v]);
        }
    } else {
        g.call(t(g, 0x149, 210), 0, &[actor, m]);
    }
    g.call(t(g, 0x19e, 211), 0, &[actor, m]);
    g.call(t(g, 0x1ab, 235), 0, &[m]);
    let x = st0_f32(g.call(t(g, 0x1b9, 277), 0, &[rd(g, 0x1bf)]).st0);
    let x = mul(x, k(g, 0x1dd));
    for o in [0x28u32, 0x14, 0] {
        g.write::<f32>(m + o, x);
    }
    let a2 = rd(g, 0x1f6);
    let x = st0_f32(g.call(t(g, 0x1e2, 15), 0, &[a2]).st0);
    let y = mul(g.read::<f32>(m), x);
    g.write::<f32>(m, y);
    let x = st0_f32(g.call(t(g, 0x213, 16), 0, &[a2]).st0);
    let y = mul(x, g.read::<f32>(m + 0x14));
    g.write::<f32>(m + 0x14, y);
    let x = st0_f32(g.call(t(g, 0x222, 17), 0, &[a2]).st0);
    let y = mul(g.read::<f32>(m + 0x28), x);
    g.write::<f32>(m + 0x28, y);
    // call width: 3
    g.call(t(g, 0x258, 330), 0, &[m, m, 0]);
    // call width: 3
    g.call(t(g, 0x276, 328), 0, &[m, m, 0]);
    // call width: 3
    g.call(t(g, 0x297, 329), 0, &[m, m, a.to_bits()]);
    for i in 0..4 {
        let x = g.read::<u32>(s + 0x920 + 4 * i);
        g.write::<u32>(s + 0x880 + 4 * i, x);
    }
    // call width: 2
    g.call(t(g, 0x2de, 238), 0, &[rd(g, 0x2e6), 1]);
    let i = rd(g, 0x2ff);
    if rd(g, 0x2f7) == 0 && i.cast_signed() < 0x20 {
        // call width: 2
        g.call(t(g, 0x30c, 296), 0, &[i, 0]);
    } else {
        // call width: 2
        g.call(t(g, 0x319, 257), 0, &[rd(g, 0x321), 1]);
    }
    // call width: 2
    g.call(t(g, 0x32e, 8), 0, &[rd(g, 0x33c), 0]);
    g.write::<f32>(s + 0x8568, a);
    // the two points ahead: (point, matrix, distance, operands)
    for (pt, mx, d, ops) in [
        (
            0x980u32,
            l(0xb4),
            0x4170_0000u32,
            [0x37bu32, 0x3c4, 0x3e9, 0x405, 0x41d],
        ),
        (
            0x990,
            l(0x74),
            0x41c0_0000,
            [0x458, 0x49e, 0x4c0, 0x4d9, 0x4f1],
        ),
    ] {
        let q = s + pt;
        for i in 0..4 {
            let x = g.read::<u32>(p + 4 * i);
            g.write::<u32>(q + 4 * i, x);
        }
        g.write::<u32>(off, 0);
        g.write::<u32>(off + 4, 0);
        g.write::<u32>(off + 8, d);
        g.call(t(g, ops[0], 235), 0, &[mx]);
        // call width: 3
        g.call(t(g, ops[1], 328), 0, &[mx, mx, 0]);
        // call width: 3
        g.call(t(g, ops[2], 329), 0, &[mx, mx, a.to_bits()]);
        g.write::<u32>(off + 0xc, ONE);
        // call width: 3
        g.call(t(g, ops[3], 229), 0, &[v, mx, off]);
        // call width: 3
        g.call(t(g, ops[4], 248), 0, &[q, q, v]);
        g.write::<u32>(q + 0xc, ONE);
    }
    // the call's return address under the three saved registers
    let eax = crate::r25::flat_coords_0586(
        g,
        img,
        img.rel32_target(f + 0x50a),
        ebp.wrapping_sub(0xbc + 0xc + 4),
    );
    for o in (0x9a0u32..0x9d0).step_by(4).chain([0x8560, 0x8564, 0x858c]) {
        g.write::<u32>(s + o, 0);
    }
    crate::r25::abs_stores(g, img, f, 0x5ab, 0x5c9);
    eax
}

/// [`caster_frames_0586`] (1491 bytes, 1 overlay).
pub const CASTER_FRAMES_0586: Sig = Sig {
    len: 1491,
    hash: 0x4529_5ba5_caee_56e3,
    calls: &[
        (0x50a, Callee::Sig(&crate::r25::FLAT_COORDS_0586)),
        // __security_check_cookie
        (0x5cb, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0586`'s start-up (0x100043f0, 346 bytes; no arguments, called
/// by its load step; no frame): the statics' store run, slot 719 `(P, 8,
/// 0)` into `G_H`; the part set up
/// ([`part_setup`](crate::sb_misc::part_setup) `(BUF)`, slot 719's `0`
/// under it), the caster frames ([`caster_frames_0586`]), the turned start
/// ([`start_turned`](crate::o4_effect::start_turned)); the two script
/// blocks' words; slot 271 `(4)`; two task nodes (kinds 1 and 2,
/// [`take`](crate::sa_pool::take)) linked onto the task list (its first
/// `G_FIRST`, its last `G_LAST`), the records block set up after the first
/// ([`records_init_400`](crate::sa_misc::records_init_400)). Returns 0.
pub fn start_up_0586<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let s719 = t(g, 0x1, 719);
    crate::r25::abs_stores(g, img, f, 0x14, 0x50);
    // call width: 3
    let h = g.call(s719, 0, &[w(0x10), 8, 0]).eax;
    g.write::<u32>(w(0x56), h);
    // slot 719's last word stays; push BUF: the return address at `E - 0xc`
    let esp = fr.esp.wrapping_sub(0xc);
    g.write::<u32>(esp + 8, 0);
    g.write::<u32>(esp + 4, w(0x5b));
    crate::sb_misc::part_setup(g, img, call(0x60), esp);
    caster_frames_0586(g, img, call(0x65), esp);
    let at = Frame {
        at: call(0x6a),
        esp,
        ..Frame::default()
    };
    crate::o4_effect::start_turned(g, img, &at);
    let s271 = t(g, 0x6f, 271);
    crate::r25::abs_stores(g, img, f, 0x73, 0xbb);
    g.call(s271, 0, &[4]);
    let n1 = crate::sa_pool::take(g, img, call(0xce), w(0xc9));
    g.write::<u8>(n1 + 0x18, 1);
    g.write::<u32>(n1 + 4, 0);
    if g.read::<u32>(w(0xd4)) == 0 {
        g.write::<u32>(n1, 0);
        g.write::<u32>(w(0xfa), n1);
    } else {
        let last = g.read::<u32>(w(0xe8));
        g.write::<u32>(n1, last);
        g.write::<u32>(last + 4, n1);
    }
    g.write::<u32>(n1 + 8, n1);
    g.write::<u32>(w(0x107), n1);
    crate::sa_misc::records_init_400(g, img, call(0x10c), w(0xff));
    let n2 = crate::sa_pool::take(g, img, call(0x116), w(0x111));
    g.write::<u8>(n2 + 0x18, 2);
    g.write::<u32>(n2 + 4, 0);
    if g.read::<u32>(w(0x11c)) == 0 {
        g.write::<u32>(n2, 0);
        g.write::<u32>(w(0x14b), n2);
        g.write::<u32>(n2 + 8, n2);
        g.write::<u32>(w(0x153), n2);
    } else {
        let last = g.read::<u32>(w(0x130));
        g.write::<u32>(n2, last);
        g.write::<u32>(last + 4, n2);
        g.write::<u32>(n2 + 8, n2);
        g.write::<u32>(w(0x13d), n2);
    }
    Ret::eax(0)
}

/// [`start_up_0586`] (346 bytes, 1 overlay).
pub const START_UP_0586: Sig = Sig {
    len: 346,
    hash: 0x31cb_33ff_cf4e_729c,
    calls: &[
        (0x60, Callee::Sig(&crate::sb_misc::PART_SETUP)),
        (0x65, Callee::Sig(&CASTER_FRAMES_0586)),
        (0x6a, Callee::Sig(&crate::o4_effect::START_TURNED_0586_2750)),
        (0xce, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x10c, Callee::Sig(&crate::sa_misc::RECORDS_INIT_400)),
        (0x116, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// `magic_0586`'s load step (0x10001340, the 118-byte shape) running
/// [`start_up_0586`].
pub fn load_0586<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::r9_load::load_step(g, img, fr, crate::r9_load::Shape::Zero, start_up_0586)
}

/// [`load_0586`].
pub const LOAD_0586: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&START_UP_0586))],
    imports: &[],
};

/// `magic_0587`'s trail parts (0x10001d40, 1187 bytes; internal: `ecx` =
/// the trail `T`, `edx` = a 3x3 basis `B`, stack `(point)`; an 8-aligned
/// /GS frame `A = ((E - 4) & !7) - 0x114`: the distance `acc` at `A`, the
/// segment's length at `A + 4`, the step `V2` at `A + 0x1c`, the points
/// `V0` at `A + 0x2c` and `V1` (their difference) at `A + 0x3c`, the
/// part's matrix `M` at `A + 0x4c`, slot 724's matrix `N` at `A + 0x8c`,
/// slot 723's at `A + 0xcc`):
/// - slots 723, 724; the animation stepped
///   ([`anim_step`](crate::sb_common::anim_step)); the part record's
///   `+0x1c` 0 and `+0x10` = `M` (`G_REC`); `point` into the ring `T +
///   0xc + 0x10 h` (`h = T + 0x820`, marked in the bytes `T + 0x828..`,
///   then `h + 1` modulo 0x80);
/// - for each of the `T + 0x824` parts `k`: once `acc` is not below the
///   segment's length, the walk goes back one point (`acc` less the
///   length; an unmarked point or 0x80 segments end it): `V0` the point,
///   `V1` = slot 234 `(V1, previous, V0)`, the length `sqrt(V1.V1)`; a
///   zero length skips the part; the colour `G_REC + 4..8` blends `T +
///   0x80c` and `+0x810` by `k / count` (bytes, signed division); the
///   place `V2` = `V0`, or below the length `V0 + acc / len * V1` (slot
///   248) with `acc` then up by `T + 0x81c`; the scale `(1 - s) T.814 + s
///   T.818` (`s = (k + 1) / count`) times `B` into `M` (slot 235 first),
///   placed by slot 229 `(M + 0x30, N, V2)`; with the colour's alpha set
///   the part linked ([`link_part`](crate::sa_misc::link_part) `(G_REC,
///   index << 16 | k)`, the trail's index in its block of 0x8a8-byte
///   trails at `*G_BLOCK + 0xe48`).
///
/// Returns 0x7f, or the old `h + 1` without parts.
#[allow(clippy::too_many_lines, clippy::float_cmp)] // the original's loop; it compares exactly
pub fn trail_parts_0587<G: Game>(g: &mut G, img: &Image, f: u32, fr: &Frame) -> u32 {
    use crate::sb_sse::{add, cvt, div, mul, sub};
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let (tr, basis) = (fr.ecx, fr.edx);
    let point = fr.arg(g, 0);
    let a = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x114);
    let (v2, v0, v1, m, n) = (a + 0x1c, a + 0x2c, a + 0x3c, a + 0x4c, a + 0x8c);
    g.call(t(g, 0x26, 723), 0, &[a + 0xcc]);
    g.call(t(g, 0x44, 724), 0, &[n]);
    let anim = crate::sb_common::anim_step(g, img, img.rel32_target(f + 0x56), tr);
    g.write::<u32>(w(0x5f), 0);
    g.write::<u32>(w(0x70), m);
    let pt = |h: u32| tr.wrapping_add(0xc).wrapping_add(h.wrapping_mul(0x10));
    let h = g.read::<u32>(tr + 0x820);
    for i in 0..4 {
        let x = g.read::<u32>(point + 4 * i);
        g.write::<u32>(pt(h) + 4 * i, x);
    }
    let h = g.read::<u32>(tr + 0x820);
    g.write::<u8>(tr.wrapping_add(0x828).wrapping_add(h), 1);
    let h = g.read::<u32>(tr + 0x820);
    let next = h.wrapping_add(1);
    g.write::<u32>(
        tr + 0x820,
        if next.cast_signed() >= 0x80 { 0 } else { next },
    );
    let count = |g: &G| g.read::<i32>(tr + 0x824);
    if count(g) <= 0 {
        return next;
    }
    let (mut acc, mut seg, mut cur, mut left) = (0.0f32, 0.0f32, h, 0x80i32);
    let mut part = 0u32;
    loop {
        let mut skip = false;
        // `comiss acc, seg; jb`: unordered keeps the segment
        if acc >= seg {
            let prev = cur.wrapping_sub(1);
            let prev = if prev.cast_signed() < 0 { 0x7f } else { prev };
            acc = sub(acc, seg);
            if g.read::<u8>(tr.wrapping_add(0x828).wrapping_add(prev)) == 0 {
                return 0x7f;
            }
            for i in 0..4 {
                let x = g.read::<u32>(pt(cur) + 4 * i);
                g.write::<u32>(v0 + 4 * i, x);
            }
            // call width: 3
            g.call(t(g, 0x160, 234), 0, &[v1, pt(prev), v0]);
            let (x, y, z) = (
                g.read::<f32>(v1),
                g.read::<f32>(v1 + 4),
                g.read::<f32>(v1 + 8),
            );
            let d2 = add(add(mul(y, y), mul(x, x)), mul(z, z));
            seg = f64::from(d2).sqrt() as f32;
            cur = prev;
            left -= 1;
            if left <= 0 {
                return 0x7f;
            }
            skip = seg == 0.0;
        }
        if !skip {
            let n_ = g.read::<i32>(tr + 0x824);
            let kk = part.cast_signed();
            for (c, gl) in [0x20bu32, 0x233, 0x25b, 0x286].into_iter().enumerate() {
                let c = c as u32;
                let lo = i32::from(g.read::<u8>(tr + 0x80c + c));
                let hi = i32::from(g.read::<u8>(tr + 0x810 + c));
                let v = lo
                    .wrapping_mul(n_.wrapping_sub(kk))
                    .wrapping_add(hi.wrapping_mul(kk));
                g.write::<u8>(w(gl), v.wrapping_div(n_).to_le_bytes()[0]);
            }
            // `comiss acc, seg; jb`: below or unordered steps part of the way
            if acc >= seg {
                for i in 0..4 {
                    let x = g.read::<u32>(v0 + 4 * i);
                    g.write::<u32>(v2 + 4 * i, x);
                }
            } else {
                let r = div(acc, seg);
                g.write::<f32>(v2, mul(r, g.read::<f32>(v1)));
                g.write::<f32>(v2 + 4, mul(r, g.read::<f32>(v1 + 4)));
                g.write::<f32>(v2 + 8, mul(r, g.read::<f32>(v1 + 8)));
                // call width: 3
                g.call(t(g, 0x2db, 248), 0, &[v2, v0, v2]);
                acc = add(g.read::<f32>(tr + 0x81c), acc);
            }
            let s = div(cvt(part.wrapping_add(1)), cvt(count(g).cast_unsigned()));
            let hi = mul(g.read::<f32>(tr + 0x818), s);
            let scale = add(mul(sub(k(g, 0x326), s), g.read::<f32>(tr + 0x814)), hi);
            g.write::<u32>(v2 + 0xc, ONE);
            g.call(t(g, 0x34c, 235), 0, &[m]);
            for o in [0u32, 4, 8, 0x10, 0x14, 0x18, 0x20, 0x24, 0x28] {
                let x = mul(g.read::<f32>(basis + o), scale);
                g.write::<f32>(m + o, x);
            }
            // call width: 3
            g.call(t(g, 0x3c2, 229), 0, &[m + 0x30, n, v2]);
            if g.read::<u8>(w(0x42d)) != 0 {
                let off = tr.wrapping_sub(g.read::<u32>(w(0x438))).wrapping_sub(0xe48);
                let index = off.cast_signed() / 0x8a8;
                // push index << 16 | k; push G_REC: the return address at
                // `A - 0x18`
                let esp = a.wrapping_sub(0x18);
                g.write::<u32>(esp + 4, w(0x45e));
                g.write::<u32>(esp + 8, (index.cast_unsigned() << 16).wrapping_add(part));
                let lf = Frame {
                    at: img.rel32_target(f + 0x465),
                    esp,
                    ecx: tr,
                    edx: anim,
                    ..*fr
                };
                crate::sa_misc::link_part(g, img, &lf);
            }
        }
        part += 1;
        if part.cast_signed() >= count(g) {
            return 0x7f;
        }
    }
}

/// [`trail_parts_0587`] (1187 bytes, 1 overlay).
pub const TRAIL_PARTS_0587: Sig = Sig {
    len: 1187,
    hash: 0x8828_1feb_d8e0_26ee,
    calls: &[
        (0x56, Callee::Sig(&crate::sb_common::ANIM_STEP)),
        (0x19f, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x465, Callee::Sig(&crate::sa_misc::LINK_PART)),
        // __security_check_cookie
        (0x49b, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0587`'s trails task (0x10002440, 214 bytes; a stored callback,
/// no arguments; an 8-aligned /GS frame `A = ((E - 4) & !7) - 0x5c`, the
/// point `P` at `A + 4`, the basis `B` at `A + 0x14`): `B` = slot 235's
/// matrix with 0.3 on its diagonal; for each of the four trails `T =
/// *G_BLOCK + 0xe48 + 0x8a8 i`: `P` = slot 237 `(*G_O, BONES[i], 0)`
/// (`P.w` 1.0); unless `*G_BLOCK + 0x30fc`, the alpha byte `T + 0x80f`
/// fades by one down to 0 (from 1 straight to 0); the trail's parts
/// ([`trail_parts_0587`] `(P)`). Returns the last one's `eax`.
pub fn trails_task_0587<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let a = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x5c);
    let (p, b) = (a + 4, a + 0x14);
    let block = g.read::<u32>(w(0x17));
    g.call(t(g, 0x23, 235), 0, &[b]);
    for o in [0x28u32, 0x14, 0] {
        g.write::<u32>(b + o, 0x3e99_999a);
    }
    let (bones, end) = (w(0x51), w(0xbe));
    let mut eax = 0;
    let mut tr = block.wrapping_add(0xe48);
    let mut bone = bones;
    while bone.cast_signed() < end.cast_signed() {
        // call width: 4
        g.call(
            t(g, 0x5b, 237),
            0,
            &[g.read::<u32>(w(0x6b)), g.read::<u32>(bone), 0, p],
        );
        g.write::<u32>(p + 0xc, ONE);
        if g.read::<u32>(block + 0x30fc) == 0 {
            let al = g.read::<u8>(tr + 0x80f);
            g.write::<u8>(tr + 0x80f, al.saturating_sub(1));
        }
        // push P: the return address at `A - 0x14`
        let esp = a.wrapping_sub(0x14);
        g.write::<u32>(esp + 4, p);
        let cf = Frame {
            esp,
            ecx: tr,
            edx: b,
            ..*fr
        };
        eax = trail_parts_0587(g, img, img.rel32_target(f + 0xac), &cf);
        bone = bone.wrapping_add(4);
        tr = tr.wrapping_add(0x8a8);
    }
    Ret::eax(eax)
}

/// [`trails_task_0587`] (214 bytes, 1 overlay).
pub const TRAILS_TASK_0587: Sig = Sig {
    len: 214,
    hash: 0xe27f_2312_0d3f_9682,
    calls: &[
        (0xac, Callee::Sig(&TRAIL_PARTS_0587)),
        // __security_check_cookie
        (0xce, Callee::Any),
    ],
    imports: &[],
};

/// One of `magic_0649`'s three model groups in its summon task's start
/// (operand and record offsets): the nodes `a` (with slot 262's point at
/// `pt` from the object `R[src]`, 1.0 at `one`), `b` (`+0x24`/`+0x1c`
/// words 0), the alpha count `out`, `c` (the model state from the cursor
/// `(R + cur.0, R + cur.1)`) and the group node `d` naming `a`, `c` and
/// `b`.
struct Group0649 {
    a: u32,
    a_ops: [u32; 8],
    a_vt: u32,
    s262: u32,
    src: u32,
    pt: u32,
    one: u32,
    b: u32,
    b_vt: u32,
    b_ops: [u32; 8],
    zeros: [u32; 2],
    flag: u32,
    s333: (u32, u32),
    s601: (u32, u32, u32),
    scale: (u32, u32),
    out: u32,
    c: u32,
    c_ops: [u32; 8],
    c_vt: u32,
    cur: (u32, u32),
    state: u32,
    d: u32,
    d_vt: u32,
    d_ops: [u32; 8],
    d_g: u32,
}

const GROUPS_0649: [Group0649; 3] = [
    Group0649 {
        a: 0x244,
        a_ops: [0x2b6, 0x2bd, 0x2c2, 0x2ca, 0x2cf, 0x2d7, 0x2e3, 0x2ef],
        a_vt: 0x302,
        s262: 0x2fc,
        src: 0x78,
        pt: 0x260,
        one: 0x26c,
        b: 0x34c,
        b_vt: 0x327,
        b_ops: [0x33a, 0x341, 0x346, 0x34e, 0x353, 0x35b, 0x367, 0x36d],
        zeros: [0x370, 0x368],
        flag: 0xb4,
        s333: (0x38f, 0x3a6),
        s601: (0x3bb, 0x74, 0x3ce),
        scale: (0x3df, 0x3ed),
        out: 0x36c,
        c: 0x270,
        c_ops: [0x40a, 0x411, 0x416, 0x41e, 0x423, 0x42b, 0x437, 0x44a],
        c_vt: 0x454,
        cur: (0x24, 0x28c),
        state: 0x45d,
        d: 0x3d0,
        d_vt: 0x469,
        d_ops: [0x47c, 0x483, 0x488, 0x490, 0x495, 0x49d, 0x4a9, 0x4af],
        d_g: 0x4c5,
    },
    Group0649 {
        a: 0x29c,
        a_ops: [0x4f1, 0x4f8, 0x4fd, 0x505, 0x50a, 0x512, 0x51e, 0x52a],
        a_vt: 0x53d,
        s262: 0x537,
        src: 0x110,
        pt: 0x2b8,
        one: 0x2c4,
        b: 0x378,
        b_vt: 0x565,
        b_ops: [0x578, 0x57f, 0x584, 0x58c, 0x591, 0x599, 0x5a5, 0x5ab],
        zeros: [0x39c, 0x394],
        flag: 0x14c,
        s333: (0x5cd, 0x5e7),
        s601: (0x5fc, 0x10c, 0x612),
        scale: (0x623, 0x631),
        out: 0x398,
        c: 0x2f4,
        c_ops: [0x64e, 0x655, 0x65a, 0x662, 0x667, 0x66f, 0x67b, 0x691],
        c_vt: 0x69b,
        cur: (0xbc, 0x310),
        state: 0x6a4,
        d: 0x51c,
        d_vt: 0x6b0,
        d_ops: [0x6c3, 0x6ca, 0x6cf, 0x6d7, 0x6dc, 0x6e4, 0x6f0, 0x6f6],
        d_g: 0x70c,
    },
    Group0649 {
        a: 0x2c8,
        a_ops: [0x738, 0x73f, 0x744, 0x74c, 0x751, 0x759, 0x765, 0x771],
        a_vt: 0x784,
        s262: 0x77e,
        src: 0x1a8,
        pt: 0x2e4,
        one: 0x2f0,
        b: 0x3a4,
        b_vt: 0x7ac,
        b_ops: [0x7bf, 0x7c6, 0x7cb, 0x7d3, 0x7d8, 0x7e0, 0x7ec, 0x7f2],
        zeros: [0x3c8, 0x3c0],
        flag: 0x1e4,
        s333: (0x814, 0x82e),
        s601: (0x843, 0x1a4, 0x859),
        scale: (0x86a, 0x878),
        out: 0x3c4,
        c: 0x320,
        c_ops: [0x895, 0x89c, 0x8a1, 0x8a9, 0x8ae, 0x8b6, 0x8c2, 0x8d8],
        c_vt: 0x8e2,
        cur: (0x154, 0x33c),
        state: 0x8eb,
        d: 0x668,
        d_vt: 0x8f7,
        d_ops: [0x90a, 0x911, 0x916, 0x91e, 0x923, 0x92b, 0x937, 0x93d],
        d_g: 0x953,
    },
];

/// `magic_0649`'s light summon task (0x10007da0, 3278 bytes; a task
/// function, cdecl `(task)`, the record `R = task[1]`; an 8-aligned frame
/// without a cookie, `A = ((E - 4) & !7) - 0x2c`: the model id at `A +
/// 4`, slot 288's count at `A + 8`, a cursor at `A + 0xc`, the set-up
/// record at `A + 0x14`). By the state `R + 0x18` (a state set and jumped
/// to runs at once), the count `R + 0x20` (one down on every way out):
/// - 0: the model nodes `R + 0xbc` or `R + 0x154` (the one matching slot
///   390's answer for `*G_ID`: 0x10ad picks `R + 0xbc` with the id 0x10ae,
///   else `R + 0x154` with 0x10ad), `R + 0x24` (the byte `*G_B`) and the
///   other one (the first byte of slot 288 `(0xb)`'s list whose slot 390
///   answer is the id, or the one past the list)
///   ([`model_node_init`](crate::sa_summon::model_node_init) on `G_L`);
///   three groups of static nodes ([`Group0649`]: slot 262's point, the
///   alpha count from slot 333 or slot 601, `K - v` times `K2` over `K3`,
///   truncated; [`model_state`](crate::m0070::model_state)); the nodes
///   `R + 0x1ec` (12, -5, 0, 1) and `R + 0x218` (-12, -5, 0, 1); two fade
///   tasks `R + 0x810` and `R + 0xde0` on the DLL's task ring (step
///   `0x10007060`, kinds 0 and 1); the effect node `R + 0x9c8c`
///   ([`effect_node`](crate::sb_nodes::effect_node)) with its vtable, bit
///   0 of `+0x2c` and `+0x30` 0; state 1, count 15;
/// - 1: at the count's end the node `R + 0x7b4` on `G_L2` (its `+0x4c..`
///   words, slot 234 `(+0x3c, +0x2c, +0x1c)`, then 0, 0.05, 1 and bit 0
///   of `+0x58`), the colour `R + 0x7d0..` (255, 255, 255, 64) and `R +
///   0x7e0..` 0; state 10, and on;
/// - 10: state 11, count 0x26;
/// - 11: at the count's end the two fade objects `R + 0x13b0` and `R +
///   0x5780` ([`fade_setup_0649`](crate::r25::fade_setup_0649), kinds 0
///   and 1); state 12, count 0x28;
/// - 12: at the count's end the light object `R + 0x9b50`
///   ([`light_object`](crate::s4_small::light_object)); state 13, count
///   0x3c;
/// - 13: at the count's end state 10000, and on;
/// - 10000: once the four objects' end words share a bit of 0x4e20, the
///   model nodes `R + 0xbc` and `R + 0x154` leave their parents and are
///   set up ([`model_setup`](crate::sa_summon::model_setup)), the task's
///   node unlinked.
///
/// Returns what `eax` holds on the way out.
#[allow(clippy::too_many_lines)] // the original's states, in order
pub fn light_task_0649<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    let a = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x2c);
    // after `push ebx; push esi; push edi`
    let s = a.wrapping_sub(0xc);
    let at = |c: u32, esp: u32, ecx: u32, edx: u32| Frame {
        at: call(c),
        esp,
        ecx,
        edx,
        ..*fr
    };
    let list_node = crate::r22::list_node_0645::<G>;
    let count = |g: &G| g.read::<i32>(r + 0x20);
    let end = |g: &mut G, eax: u32| {
        let c = g.read::<u32>(r + 0x20).wrapping_sub(1);
        g.write::<u32>(r + 0x20, c);
        Ret::eax(eax)
    };
    let step = |g: &mut G, n: u32, eax: u32| {
        let st = g.read::<u32>(r + 0x18).wrapping_add(1);
        g.write::<u32>(r + 0x18, st);
        g.write::<u32>(r + 0x20, n.wrapping_sub(1));
        Ret::eax(eax)
    };
    loop {
        let state = g.read::<u32>(r + 0x18);
        match state {
            0 => {
                let cur = a + 0x14;
                let model = |g: &mut G, node: u32, vt: u32, c: u32| {
                    g.write::<u32>(cur, node);
                    g.write::<u32>(node, w(vt));
                    crate::sa_summon::model_node_init(g, img, call(c), cur);
                };
                let id_src = g.read::<u32>(w(0x177));
                let ans = g.call(t(g, 0x171, 390), 0, &[id_src]).eax;
                let v = g.read::<u32>(w(0x18c));
                g.write::<u32>(cur + 4, w(0x194));
                g.write::<u8>(cur + 0xc, 0);
                g.write::<u32>(cur + 8, v);
                let id = if ans == 0x10ad {
                    model(g, r + 0xbc, 0x1af, 0x1b8);
                    0x10ae
                } else {
                    model(g, r + 0x154, 0x1d2, 0x1db);
                    0x10ad
                };
                g.write::<u32>(a + 4, id);
                let b = u32::from(g.read::<u8>(w(0x1ea)));
                g.write::<u32>(cur + 4, w(0x203));
                g.write::<u32>(cur + 8, b);
                g.write::<u8>(cur + 0xc, 0);
                model(g, r + 0x24, 0x1f7, 0x211);
                // call width: 2
                let mut p = g.call(t(g, 0x21b, 288), 0, &[0xb, a + 8]).eax;
                let n = g.read::<i32>(a + 8);
                let mut i = 0;
                while i < n {
                    let c = u32::from(g.read::<u8>(p));
                    if g.call(t(g, 0x244, 390), 0, &[c]).eax == id {
                        break;
                    }
                    i += 1;
                    p = p.wrapping_add(1);
                }
                let b = u32::from(g.read::<u8>(p));
                g.write::<u32>(cur + 4, w(0x273));
                g.write::<u8>(cur + 0xc, 0);
                g.write::<u32>(cur + 8, b);
                let other = if id == 0x10ae { r + 0x154 } else { r + 0xbc };
                model(g, other, 0x294, 0x29d);
                for gr in &GROUPS_0649 {
                    let na = r + gr.a;
                    list_node(g, img, f, na, gr.a_ops);
                    g.write::<u32>(na, w(gr.a_vt));
                    let pt = r + gr.pt;
                    let src = g.read::<u32>(r + gr.src);
                    // call width: 4
                    g.call(t(g, gr.s262, 262), 0, &[src, pt, pt + 4, pt + 8]);
                    g.write::<u32>(r + gr.one, ONE);
                    let nb = r + gr.b;
                    g.write::<u32>(nb, w(gr.b_vt));
                    list_node(g, img, f, nb, gr.b_ops);
                    for z in gr.zeros {
                        g.write::<u32>(r + z, 0);
                    }
                    let x = if g.read::<u8>(r + gr.flag) & 1 != 0 {
                        let o = g.call(t(g, gr.s333.0, 333), 0, &[src]);
                        sub(k(g, gr.s333.1), crate::ob_model::st0_f32(o.st0))
                    } else {
                        let (tb, from, kk) = gr.s601;
                        let v = a + 0xc;
                        // call width: 2
                        g.call(t(g, tb, 601), 0, &[g.read::<u32>(r + from), v]);
                        sub(k(g, kk), g.read::<f32>(v))
                    };
                    let x = div(mul(x, k(g, gr.scale.0)), k(g, gr.scale.1));
                    g.write::<u32>(r + gr.out, crate::s4_summon::trunc(x));
                    let nc = r + gr.c;
                    list_node(g, img, f, nc, gr.c_ops);
                    g.write::<u32>(nc, w(gr.c_vt));
                    let me = a + 0xc;
                    g.write::<u32>(me, r + gr.cur.0);
                    g.write::<u32>(me + 4, r + gr.cur.1);
                    crate::m0070::model_state(g, img, call(gr.state), me);
                    let nd = r + gr.d;
                    g.write::<u32>(nd, w(gr.d_vt));
                    list_node(g, img, f, nd, gr.d_ops);
                    g.write::<u32>(nd + 0x140, nc);
                    g.write::<u32>(nd + 0x13c, na);
                    g.write::<u32>(nd + 0x128, w(gr.d_g));
                    g.write::<u8>(nd + 0x148, 0);
                    g.write::<u32>(nd + 0x144, nb);
                }
                // the two points
                for (node, ops, vt, x) in [
                    (
                        0x1ecu32,
                        [0x97fu32, 0x986, 0x98b, 0x993, 0x998, 0x9a0, 0x9ac, 0x9b2],
                        0x9b8u32,
                        0x4140_0000u32,
                    ),
                    (
                        0x218,
                        [0x9f9, 0xa00, 0xa05, 0xa0d, 0xa12, 0xa1a, 0xa26, 0xa2c],
                        0xa32,
                        0xc140_0000,
                    ),
                ] {
                    let n = r + node;
                    list_node(g, img, f, n, ops);
                    g.write::<u32>(n, w(vt));
                    for (i, v) in (0u32..).zip([x, 0xc0a0_0000, 0, ONE]) {
                        g.write::<u32>(n + 0x1c + 4 * i, v);
                    }
                }
                // the two fade tasks on the task ring: (task, vtable, step,
                // ring last, sentinel, last, flags, last, record words)
                g.write::<u32>(cur + 4, w(0xa7c));
                g.write::<u32>(cur + 0xc, w(0xaa1));
                let tasks = [
                    (
                        0x810u32,
                        [0xa64u32, 0xa6e, 0xa8a, 0xa94, 0xa99, 0xaaa, 0xab1],
                        0x1ecu32,
                        0x51cu32,
                        0xbcu32,
                        0u32,
                    ),
                    (
                        0xde0,
                        [0xaec, 0xaf9, 0xb01, 0xb0b, 0xb10, 0xb1d, 0xb24],
                        0x218,
                        0x668,
                        0x154,
                        1,
                    ),
                ];
                for (tk, ops, n80, n3cc, n24, kind) in tasks {
                    let tn = r + tk;
                    g.write::<u32>(tn, w(ops[0]));
                    g.write::<u32>(tn + 8, tn);
                    g.write::<u32>(tn + 4, tn);
                    g.write::<u32>(tn + 0x10, w(ops[1]));
                    g.write::<u32>(tn + 0x14, tn);
                    let last = g.read::<u32>(w(ops[2]));
                    g.write::<u32>(tn + 8, last);
                    g.write::<u32>(tn + 4, w(ops[3]));
                    let last = g.read::<u32>(w(ops[4]));
                    g.write::<u32>(last + 4, tn);
                    let fl = g.read::<u32>(w(ops[5])) & !1;
                    g.write::<u32>(w(ops[5]), fl);
                    g.write::<u32>(w(ops[6]), tn);
                    g.write::<u32>(tn + 0x80, r + n80);
                    g.write::<u32>(tn + 0x20, kind);
                    g.write::<u32>(tn + 0x18, 0);
                    g.write::<u32>(tn + 0x3cc, r + n3cc);
                    g.write::<u32>(tn + 0x24, r + n24);
                }
                let en = r + 0x9c8c;
                g.write::<u32>(cur, en);
                g.write::<u32>(cur + 8, r + 0x3d0);
                g.write::<u32>(cur + 0x10, w(0xb5d));
                g.write::<u32>(cur + 0x14, 0);
                crate::sb_nodes::effect_node(g, img, call(0xb6a), cur);
                let node = g.read::<u32>(cur);
                g.write::<u32>(node, w(0xb74));
                let fl = g.read::<u32>(en + 0x2c) | 1;
                g.write::<u32>(en + 0x2c, fl);
                g.write::<u32>(en + 0x30, 0);
                return step(g, 0xf, node);
            }
            1 => {
                if count(g) > 0 {
                    return end(g, 0);
                }
                let n = r + 0x7b4;
                g.write::<u32>(n, w(0x53));
                list_node(
                    g,
                    img,
                    f,
                    n,
                    [0x66, 0x6d, 0x72, 0x7a, 0x7f, 0x87, 0x93, 0x99],
                );
                for (o, v) in [(0x4cu32, 0u32), (0x50, 0), (0x54, 0), (0x58, 0x48_0000)] {
                    g.write::<u32>(n + o, v);
                }
                for (o, v) in [
                    (0x7d8u32, 0x437f_0000u32),
                    (0x7d4, 0x437f_0000),
                    (0x7d0, 0x437f_0000),
                    (0x7dc, 0x4280_0000),
                    (0x7e8, 0),
                    (0x7e4, 0),
                    (0x7e0, 0),
                    (0x7ec, 0),
                ] {
                    g.write::<u32>(r + o, v);
                }
                // call width: 3
                g.call(t(g, 0xc6, 234), 0, &[n + 0x3c, n + 0x2c, n + 0x1c]);
                let fl = g.read::<u16>(n + 0x58) | 1;
                g.write::<u16>(n + 0x58, fl);
                g.write::<u32>(n + 0x54, 1);
                g.write::<u32>(n + 0x50, 0x3d4c_cccd);
                g.write::<u32>(n + 0x4c, 0);
                g.write::<u32>(r + 0x18, 10);
            }
            10 => return step(g, 0x26, 10),
            11 => {
                if count(g) > 0 {
                    return end(g, 0);
                }
                let a0 = r + 0xbc;
                // push c; push b; push kind; push a: the return address
                // at `A - 0x20`, the second call's at `A - 0x30`
                let mut eax = 0;
                for (esp, obj, kind, b, c) in [
                    (a.wrapping_sub(0x20), 0x13b0u32, 0u32, 0x1ecu32, 0x51cu32),
                    (a.wrapping_sub(0x30), 0x5780, 1, 0x218, 0x668),
                ] {
                    for (i, v) in (1u32..).zip([a0, kind, r + b, r + c]) {
                        g.write::<u32>(esp + 4 * i, v);
                    }
                    let c = if kind == 0 { 0xc1a } else { 0xc39 };
                    eax = crate::r25::fade_setup_0649(g, img, &at(c, esp, r + obj, r + 0x24));
                }
                return step(g, 0x28, eax);
            }
            12 => {
                if count(g) > 0 {
                    return end(g, 0);
                }
                // sub esp, 8; push b: the return address at `A - 0x1c`
                let esp = a.wrapping_sub(0x1c);
                g.write::<u32>(esp + 4, r + 0x3d0);
                let lo = &at(0xbd4, esp, r + 0x9b50, r + 0x24);
                let eax = crate::s4_small::light_object(g, img, lo).eax;
                return step(g, 0x3c, eax);
            }
            13 => {
                if count(g) > 0 {
                    return end(g, 0xd);
                }
                g.write::<u32>(r + 0x18, 0x2710);
            }
            0x2710 => {
                let eax = [0x5798u32, 0x13c8, 0xdf8, 0x828]
                    .iter()
                    .fold(u32::MAX, |x, &o| x & g.read::<u32>(r + o));
                if eax & 0x4e20 == 0 {
                    return end(g, eax);
                }
                // node_leave leaves `ecx` = the node (it never writes it)
                let cur = a + 0xc;
                let mut eax = eax;
                for (m, ms) in [(0xbcu32, 0xc8eu32), (0x154, 0xca6)] {
                    crate::sa_misc::leave(g, r + m, eax);
                    g.write::<u32>(cur, r + m);
                    eax = crate::sa_summon::model_setup(g, img, &at(ms, s.wrapping_sub(4), cur, 0))
                        .eax;
                }
                let node = g.read::<u32>(task);
                let (prev, next) = (g.read::<u32>(node + 4), g.read::<u32>(node + 8));
                g.write::<u32>(prev + 8, next);
                let (next, prev) = (g.read::<u32>(node + 8), g.read::<u32>(node + 4));
                g.write::<u32>(next + 4, prev);
                g.write::<u32>(node + 8, node);
                g.write::<u32>(node + 4, node);
                return end(g, prev);
            }
            // `dec eax; jne` below 10: the state less one
            s if s < 10 => return end(g, s.wrapping_sub(1)),
            s => return end(g, s),
        }
    }
}

/// [`light_task_0649`] (3278 bytes, 1 overlay).
pub const LIGHT_TASK_0649: Sig = Sig {
    len: 3278,
    hash: 0x1f21_8fdc_ea91_54c9,
    calls: &[
        (0x1b8, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0x1db, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0x211, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0x29d, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0x45d, Callee::Sig(&crate::m0070::MODEL_STATE)),
        (0x6a4, Callee::Sig(&crate::m0070::MODEL_STATE)),
        (0x8eb, Callee::Sig(&crate::m0070::MODEL_STATE)),
        (0xb6a, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
        (0xbd4, Callee::Sig(&crate::s4_small::LIGHT_OBJECT)),
        (0xc1a, Callee::Sig(&crate::r25::FADE_SETUP_0649)),
        (0xc39, Callee::Sig(&crate::r25::FADE_SETUP_0649)),
        (0xc81, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0xc8e, Callee::Sig(&crate::sa_summon::MODEL_SETUP)),
        (0xc99, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0xca6, Callee::Sig(&crate::sa_summon::MODEL_SETUP)),
    ],
    imports: &[],
};

/// `magic_0586`'s second point object (0x10002ce0, 749 bytes; a table
/// callback, no arguments; a cookie frame, `ebp = E - 4`, the caster's
/// place at `ebp - 0x14`). The record `S = *G_S`, every record offset read
/// from the code (`magic_0162`'s copy at 0x100030d0 has its record laid
/// out further on): the object `S + 0x260` started
/// ([`start_object_plain`](crate::s4_misc::start_object_plain), scale
/// `K`, part 2) at the second point ahead (`P2`, its `y` less `K1`; its
/// turn `S.a`); the place `Q` = `P2` with `y` less `K2`; the model matrix
/// `M` as [`caster_frames_0586`]'s (slots 210 or 217/255, 211, 235, 277
/// times `K3`, 15, 16, 17) from the actor `ACTORS[(*G_SEL)[1]]`; the old
/// turn and place copied on (`S + 0x930..`), then slots 330, 328, 329
/// `(M, M, angle)` with those angles; the copied place into `S + 0x8c0`.
/// Returns its last word.
#[allow(clippy::too_many_lines)] // one straight sequence
pub fn point_object_0586<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::ob_model::st0_f32;
    use crate::sb_sse::{mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let rd = |g: &G, o: u32| g.read::<u32>(w(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let ebp = fr.esp.wrapping_sub(4);
    let s = rd(g, 0x1d);
    // `[edi + d]` at the operand `o`
    let at = |o: u32| s.wrapping_add(w(o));
    let copy = |g: &mut G, src: u32, dst: u32| {
        let x = g.read::<u32>(at(src));
        g.write::<u32>(at(dst), x);
    };
    let obj = s + 0x260;
    let scale = k(g, 0x14);
    crate::s4_misc::start_object_plain(g, img, img.rel32_target(f + 0x2f), obj, scale, 2);
    for (i, o) in (0u32..).zip([0x35u32, 0x41, 0x4d, 0x59]) {
        let x = g.read::<u32>(at(o));
        g.write::<u32>(obj + 0xd0 + 4 * i, x);
    }
    let y = sub(g.read::<f32>(obj + 0xd4), k(g, 0x75));
    let turn = g.read::<u32>(at(0x6d));
    g.write::<u32>(obj + 0xc4, turn);
    let m = at(0x81);
    g.write::<f32>(obj + 0xd4, y);
    let y = sub(g.read::<f32>(at(0x9a)), k(g, 0xa2));
    copy(g, 0x92, 0xa8);
    copy(g, 0xae, 0xb4);
    copy(g, 0xba, 0xc0);
    g.write::<f32>(at(0xc8), y);
    let actor = |g: &G, tbl: u32, sel: u32| {
        let i = u32::from(g.read::<u8>(rd(g, sel) + 1));
        (i, g.read::<u32>(w(tbl).wrapping_add(4 * i)))
    };
    let (i, a) = actor(g, 0xd8, 0xcd);
    if m == 0 {
        // `S + 0x890` null: no caller's `S` gets here
        let v = ebp.wrapping_sub(0x14);
        if rd(g, 0xf9) == 0 && i < 0x20 {
            g.call(t(g, 0x10a, 217), 0, &[i, v]);
            g.call(t(g, 0x11c, 255), 0, &[i, v]);
        }
    } else {
        g.call(t(g, 0xe6, 210), 0, &[a, m]);
    }
    g.call(t(g, 0x132, 211), 0, &[a, m]);
    for (src, dst) in [
        (0x140u32, 0x146u32),
        (0x14c, 0x152),
        (0x158, 0x15e),
        (0x164, 0x16a),
        (0x170, 0x176),
        (0x17c, 0x182),
        (0x188, 0x18e),
        (0x194, 0x19a),
    ] {
        copy(g, src, dst);
    }
    g.call(t(g, 0x19f, 235), 0, &[m]);
    let (_, a2) = actor(g, 0x1b8, 0x1ad);
    let x = st0_f32(g.call(t(g, 0x1bd, 277), 0, &[a2]).st0);
    let x = mul(x, k(g, 0x1d5));
    for o in [0x28u32, 0x14, 0] {
        g.write::<f32>(m + o, x);
    }
    let (_, a3) = actor(g, 0x1f3, 0x1e8);
    let x = st0_f32(g.call(t(g, 0x1f8, 15), 0, &[a3]).st0);
    let y = mul(x, g.read::<f32>(m));
    g.write::<f32>(m, y);
    let x = st0_f32(g.call(t(g, 0x20f, 16), 0, &[a3]).st0);
    let y = mul(x, g.read::<f32>(m + 0x14));
    g.write::<f32>(m + 0x14, y);
    let x = st0_f32(g.call(t(g, 0x21e, 17), 0, &[a3]).st0);
    let y = mul(g.read::<f32>(m + 0x28), x);
    g.write::<f32>(m + 0x28, y);
    for (tb, n, o) in [
        (0x248u32, 330u32, 0x258u32),
        (0x274, 328, 0x26f),
        (0x296, 329, 0x291),
    ] {
        let ang = g.read::<u32>(at(o));
        // call width: 3
        g.call(t(g, tb, n), 0, &[m, m, ang]);
    }
    for (src, dst) in [
        (0x2abu32, 0x2b4u32),
        (0x2ba, 0x2c0),
        (0x2c6, 0x2cf),
        (0x2d5, 0x2db),
    ] {
        copy(g, src, dst);
    }
    Ret::eax(g.read::<u32>(at(0x2d5)))
}

/// [`point_object_0586`] (749 bytes; `magic_0586`, and `magic_0162`'s
/// copy with its own record offsets).
pub const POINT_OBJECT_0586: Sig = Sig {
    len: 749,
    hash: 0xf901_2168_7017_383a,
    calls: &[
        (0x2f, Callee::Sig(&crate::s4_misc::START_OBJECT_PLAIN)),
        // __security_check_cookie
        (0x2e5, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0162`'s copy of [`point_object_0586`] (0x100030d0, 749 bytes;
/// its record offsets read from its code).
pub const POINT_OBJECT_0162: Sig = Sig {
    len: 749,
    hash: 0x6454_bded_0da9_175a,
    calls: &[
        (0x2f, Callee::Sig(&crate::s4_misc::START_OBJECT_PLAIN)),
        // __security_check_cookie
        (0x2e5, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0586`'s glow effects (0x10002fd0, 1112 bytes; a pool task's
/// callback, no arguments; an 8-aligned /GS frame `A = ((E - 4) & !7) -
/// 0x14c`: `S` kept at `A`, the matrix `M2` at `A + 4`, `M1` at `A + 0x44`,
/// the camera rows at `A + 0x84`, slot 723's and 724's matrices at `A +
/// 0xc4` and `A + 0x104`). The record `S = *G_S`, its offsets read from
/// the code (`magic_0162`'s copy at 0x100033c0 has them 0x60 further on):
/// - the spin `S.ry` up by `K` and brought into `-pi..pi`; `M1` = slot
///   235's matrix turned by slots 330, 328, 329 with `S.rz`, `S.rx`,
///   `S.ry`; slots 723, 724;
/// - by the fade state `S.st` with the count `S.n` (one down each time):
///   0 fades in (`S.b = (0xa0 - n) * 128 / 160`; at 0: `S.a` 0, `S.b`
///   0x80, count 0x6e, state 1); 1 holds (at 0: count 0x20, state 2); 2
///   fades out (`S.a = (0x20 - n) * 128 / 32`, `S.b = n * 128 / 32`; at
///   0: `S.a` 0x80, `S.b` 0, state 3); others keep the levels;
/// - the camera rows ([`scaled_camera`](crate::m0052::scaled_camera));
///   for the caster `*G_A` and then the selected actor `ACTORS[(*G_SEL)[1]]`:
///   slots 357 `(X, -1, 0x3c00)`, 356 `(X, -1, 0x2ee0)`, 355 and 354 `(X,
///   -1)`; `M2` = slot 235's matrix turned by slots 330, 328, 329 with the
///   caster's turn `S + 0x910..` (the second time the old one, `S +
///   0x930..`); the model effect placed on `X`
///   ([`model_effect_placed`](crate::s8_misc::model_effect_placed) `(M2,
///   rows, M1, S.a, S.b)`; its unused first word is the `ecx` slot 329
///   leaves, 0 here);
/// - with `*G_A` set and `*G_OFF` 0, slot 938 `(G_A[0x830])`.
///
/// Returns slot 938's `eax`, or `*G_A`.
#[allow(clippy::too_many_lines)] // one straight sequence
pub fn glow_effects_0586<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let rd = |g: &G, o: u32| g.read::<u32>(w(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let a = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x14c);
    let (m2, m1, cam) = (a + 4, a + 0x44, a + 0x84);
    let s = rd(g, 0x1f);
    // `[S + d]` at the operand `o`
    let at = |o: u32| s.wrapping_add(w(o));
    let mut ry = add(g.read::<f32>(at(0x2b)), k(g, 0x33));
    if ry > k(g, 0x3a) {
        ry = sub(ry, k(g, 0x44));
    } else if k(g, 0x4e) > ry {
        ry = add(ry, k(g, 0x5b));
    }
    g.write::<f32>(at(0x6d), ry);
    g.call(t(g, 0x65, 235), 0, &[m1]);
    for (tb, n, o) in [
        (0x8du32, 330u32, 0x7du32),
        (0xb0, 328, 0x9d),
        (0xd3, 329, 0xc0),
    ] {
        let ang = g.read::<u32>(at(o));
        // call width: 3
        g.call(t(g, tb, n), 0, &[m1, m1, ang]);
    }
    g.call(t(g, 0xeb, 723), 0, &[a + 0xc4]);
    g.call(t(g, 0x103, 724), 0, &[a + 0x104]);
    let (st, cnt, la, lb) = (at(0x111), at(0x12d), at(0x14c), at(0x160));
    let down = |g: &mut G| {
        let n = g.read::<u32>(cnt).wrapping_sub(1);
        g.write::<u32>(cnt, n);
        n
    };
    match g.read::<u32>(st) {
        0 => {
            let n = down(g);
            let x = 0xa0u32.wrapping_sub(n).wrapping_shl(7).cast_signed() / 160;
            g.write::<u32>(lb, x.cast_unsigned());
            if n == 0 {
                g.write::<u32>(la, 0);
                g.write::<u32>(lb, 0x80);
                g.write::<u32>(cnt, 0x6e);
                g.write::<u32>(st, 1);
            }
        }
        1 => {
            if down(g) == 0 {
                g.write::<u32>(cnt, 0x20);
                g.write::<u32>(st, 2);
            }
        }
        2 => {
            let n = down(g);
            let x = 0x20u32.wrapping_sub(n).wrapping_shl(7).cast_signed() / 32;
            g.write::<u32>(la, x.cast_unsigned());
            let y = n.wrapping_shl(7).cast_signed() / 32;
            g.write::<u32>(lb, y.cast_unsigned());
            if n == 0 {
                g.write::<u32>(la, 0x80);
                g.write::<u32>(lb, 0);
                g.write::<u32>(st, 3);
            }
        }
        _ => {}
    }
    let (lv_a, lv_b) = (g.read::<u32>(la), g.read::<u32>(lb));
    // the return address under the three saved registers
    crate::m0052::scaled_camera(g, img, call(0x20f), cam, a.wrapping_sub(0x10));
    // (the caster or the selected actor, the slots' operands, the turn's
    // operands, the call)
    for (second, ops, turn, c) in [
        (
            false,
            [0x214u32, 0x22f, 0x247, 0x25a, 0x26f],
            [
                (0x296u32, 330u32, 0x286u32),
                (0x2bd, 328, 0x2ad),
                (0x2e4, 329, 0x2d4),
            ],
            0x30bu32,
        ),
        (
            true,
            [0x328, 0x338, 0x350, 0x363, 0x378],
            [
                (0x39f, 330, 0x38f),
                (0x3c6, 328, 0x3b6),
                (0x3ed, 329, 0x3dd),
            ],
            0x414,
        ),
    ] {
        let x = if second {
            let sel = u32::from(g.read::<u8>(rd(g, 0x310) + 1));
            g.read::<u32>(w(0x323).wrapping_add(4 * sel))
        } else {
            rd(g, 0x21a)
        };
        // call width: 3
        g.call(t(g, ops[0], 357), 0, &[x, u32::MAX, 0x3c00]);
        // call width: 3
        g.call(t(g, ops[1], 356), 0, &[x, u32::MAX, 0x2ee0]);
        // call width: 2
        g.call(t(g, ops[2], 355), 0, &[x, u32::MAX]);
        // call width: 2
        g.call(t(g, ops[3], 354), 0, &[x, u32::MAX]);
        g.call(t(g, ops[4], 235), 0, &[m2]);
        for (tb, n, o) in turn {
            let ang = g.read::<u32>(at(o));
            // call width: 3
            g.call(t(g, tb, n), 0, &[m2, m2, ang]);
        }
        // push b, a, M1, rows, M2, ecx: the return address at `A - 0x2c`
        let esp = a.wrapping_sub(0x2c);
        // junk word: the `ecx` slot 329 leaves; 0 (brief 2026-11-08), the
        // callee never reads it
        for (i, v) in (1u32..).zip([0, m2, cam, m1, lv_a, lv_b]) {
            g.write::<u32>(esp + 4 * i, v);
        }
        let mf = Frame {
            at: call(c),
            esp,
            ecx: x,
            ..*fr
        };
        crate::s8_misc::model_effect_placed(g, img, &mf);
    }
    let caster = rd(g, 0x419);
    if caster == 0 || rd(g, 0x426) != 0 {
        return Ret::eax(caster);
    }
    let m = g.read::<u32>(caster + 0x830);
    Ret::eax(g.call(t(g, 0x434, 938), 0, &[m]).eax)
}

/// [`glow_effects_0586`] (1112 bytes, 1 overlay).
pub const GLOW_EFFECTS_0586: Sig = Sig {
    len: 1112,
    hash: 0xefed_3ecf_5015_1690,
    calls: &[
        (0x20f, Callee::Sig(&crate::m0052::SCALED_CAMERA)),
        (0x30b, Callee::Sig(&crate::s8_misc::MODEL_EFFECT_PLACED)),
        (0x414, Callee::Sig(&crate::s8_misc::MODEL_EFFECT_PLACED)),
        // __security_check_cookie
        (0x450, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0162`'s copy of [`glow_effects_0586`] (0x100033c0, 1112
/// bytes).
pub const GLOW_EFFECTS_0162: Sig = Sig {
    len: 1112,
    hash: 0xdb7f_2f6a_b711_bc0d,
    calls: &[
        (0x20f, Callee::Sig(&crate::m0052::SCALED_CAMERA)),
        (0x30b, Callee::Sig(&crate::s8_misc::MODEL_EFFECT_PLACED)),
        (0x414, Callee::Sig(&crate::s8_misc::MODEL_EFFECT_PLACED)),
        // __security_check_cookie
        (0x450, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0586`'s model placement (0x10003780, 1454 bytes; a table
/// callback, no arguments; a cookie frame, `ebp = E - 4`: the matrix `L`
/// at `ebp - 0x64`, the turned offset at `ebp - 0x24`, the offset at
/// `ebp - 0x14`). The record `S = *G_S`, every record offset read from the code
/// (`magic_0162`'s copy at 0x10003b70 has its record laid out further on):
/// - the object `S + 0x390` taken out of `G_LIST`
///   ([`list_remove`](crate::l5_objhost::list_remove)): then `**G_P` =
///   `ITEM`, slot 713 `(O)` and bit 15 of u16 `O + 0x124` cleared;
/// - the old turn and place copied (`S.t2 = S.t`, `S.q2 = S.q`); `S.q2`
///   moved by `(0, -16, -2)` turned by the copied turn's `x` and `y`
///   (slots 235, 328, 329, 229, 248), its `w` 1.0; the constants at `S +
///   0x9b0..`; slot 371 `(0, 0x50000)` and `(1, 0x50000)`;
/// - unless bit 0 of slot 324's answer: `S.on` 0. Else the turn `(x, y -
///   K, z)` (`y` into `-pi..pi`) on the two model matrices `S.m1` (the
///   caster `*G_A`) and `S.m2` (the selected actor
///   `ACTORS[(*G_SEL)[1]]`): slot 235, slot 277's scale times `K2` on the
///   diagonal, times slots 15, 16, 17, then slots 330, 328, 329 with `z`,
///   `x`, `y`; then the place `S.q` moved by `(0, 0.6, 1.5)` turned by `x`
///   and `y - K3` (into `-pi..pi`), its `w` 1.0, copied to `S.q3`; `S.on`
///   1;
/// - a task node ([`take`](crate::sa_pool::take), step `STEP`) linked
///   last on the task list (`G_FIRST`/`G_LAST`) and kept at `S.node`;
///   `*G_F` 1.
///
/// Returns the node.
#[allow(clippy::too_many_lines)] // one straight sequence
pub fn models_place_0586<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::ob_model::st0_f32;
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let rd = |g: &G, o: u32| g.read::<u32>(w(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let (lm, lv, lo) = (
        ebp.wrapping_sub(0x64),
        ebp.wrapping_sub(0x24),
        ebp.wrapping_sub(0x14),
    );
    let s = rd(g, 0x13);
    // `[S + d]` at the operand `o`
    let at = |o: u32| s.wrapping_add(w(o));
    let copy = |g: &mut G, src: u32, dst: u32| {
        let x = g.read::<u32>(src);
        g.write::<u32>(dst, x);
    };
    let obj = at(0x1a);
    if crate::l5_objhost::list_remove(g, img, call(0x26), w(0x21), obj) != 0 {
        g.write::<u32>(rd(g, 0x2f), w(0x36));
        g.call(t(g, 0x3b, 713), 0, &[obj]);
        let b = g.read::<u16>(obj + 0x124) & 0x7fff;
        g.write::<u16>(obj + 0x124, b);
    }
    for (src, dst) in [
        (0x58u32, 0x5eu32),
        (0x64, 0x6a),
        (0x70, 0x76),
        (0x7c, 0x82),
        (0x88, 0x8e),
        (0x94, 0xa0),
        (0xa6, 0xac),
        (0xb2, 0xb8),
    ] {
        copy(g, at(src), at(dst));
    }
    let q = at(0x9a);
    // the offset moved and turned into `out` (the matrix `L`, slot 229's
    // vector at `ebp - 0x24`): (operands of 235, 328, 329, 229, 248)
    let moved = |g: &mut G, off: [u32; 3], rx: u32, ry: u32, out: u32, ops: [u32; 5]| {
        for (i, v) in (0u32..).zip(off) {
            g.write::<u32>(lo + 4 * i, v);
        }
        g.call(t(g, ops[0], 235), 0, &[lm]);
        // call width: 3
        g.call(t(g, ops[1], 328), 0, &[lm, lm, rx]);
        // call width: 3
        g.call(t(g, ops[2], 329), 0, &[lm, lm, ry]);
        g.write::<u32>(lo + 0xc, ONE);
        // call width: 3
        g.call(t(g, ops[3], 229), 0, &[lv, lm, lo]);
        // call width: 3
        g.call(t(g, ops[4], 248), 0, &[out, out, lv]);
        g.write::<u32>(out + 0xc, ONE);
    };
    let (rx, ry) = (g.read::<u32>(at(0xe6)), g.read::<u32>(at(0x105)));
    moved(
        g,
        [0, 0xc180_0000, 0xc000_0000],
        rx,
        ry,
        q,
        [0xc1, 0xf5, 0x117, 0x130, 0x148],
    );
    for (o, v) in [
        (0x164u32, 0xbf40_0000u32),
        (0x16e, ONE),
        (0x178, 0),
        (0x182, 0x3f20_d97c),
        (0x18c, 0),
    ] {
        g.write::<u32>(at(o), v);
    }
    // call width: 2
    g.call(t(g, 0x157, 371), 0, &[0, 0x5_0000]);
    // call width: 2
    g.call(t(g, 0x1a4, 371), 0, &[1, 0x5_0000]);
    let on = at(0x540);
    if g.call(t(g, 0x1b8, 324), 0, &[]).eax & 1 == 0 {
        g.write::<u32>(on, 0);
    } else {
        let (x, z) = (g.read::<u32>(at(0x1d3)), g.read::<u32>(at(0x1db)));
        let wrap = |g: &G, y: f32, (hi, step, lo_, step2): (u32, u32, u32, u32)| {
            if y > k(g, hi) {
                sub(y, k(g, step))
            } else if k(g, lo_) > y {
                add(y, k(g, step2))
            } else {
                y
            }
        };
        let y = sub(g.read::<f32>(at(0x1e8)), k(g, 0x1f0));
        let y = wrap(g, y, (0x204, 0x1fd, 0x219, 0x1fd));
        // (the matrix's operand, the actor's, slot 235's, 277's, the
        // scale constant, 15's, 16's, 17's, 330's, 328's, 329's)
        for (mo, second, ops) in [
            (
                0x239u32,
                false,
                [
                    0x22cu32, 0x241, 0x25f, 0x264, 0x28f, 0x2ab, 0x2c8, 0x2ee, 0x30a,
                ],
            ),
            (
                0x327,
                true,
                [
                    0x321, 0x345, 0x35d, 0x380, 0x397, 0x3a6, 0x3d0, 0x3f6, 0x412,
                ],
            ),
        ] {
            let m = at(mo);
            g.call(t(g, ops[0], 235), 0, &[m]);
            let actor = |g: &G, sel: u32, tbl: u32| {
                let i = u32::from(g.read::<u8>(rd(g, sel) + 1));
                g.read::<u32>(w(tbl).wrapping_add(4 * i))
            };
            let a = if second {
                actor(g, 0x335, 0x340)
            } else {
                rd(g, 0x247)
            };
            let sc = st0_f32(g.call(t(g, ops[1], 277), 0, &[a]).st0);
            let sc = mul(sc, k(g, ops[2]));
            for o in [0x28u32, 0x14, 0] {
                g.write::<f32>(m + o, sc);
            }
            let a2 = if second {
                actor(g, 0x370, 0x37b)
            } else {
                rd(g, 0x278)
            };
            let f15 = st0_f32(g.call(t(g, ops[3], 15), 0, &[a2]).st0);
            let v = mul(g.read::<f32>(m), f15);
            g.write::<f32>(m, v);
            let f16 = st0_f32(g.call(t(g, ops[4], 16), 0, &[a2]).st0);
            // `0586`'s caster: `[M + 0x14] * f`; the actor: `f * [M + 0x14]`
            let v = if second {
                mul(f16, g.read::<f32>(m + 0x14))
            } else {
                mul(g.read::<f32>(m + 0x14), f16)
            };
            g.write::<f32>(m + 0x14, v);
            let f17 = st0_f32(g.call(t(g, ops[5], 17), 0, &[a2]).st0);
            let v = mul(g.read::<f32>(m + 0x28), f17);
            g.write::<f32>(m + 0x28, v);
            // call width: 3
            g.call(t(g, ops[6], 330), 0, &[m, m, z]);
            // call width: 3
            g.call(t(g, ops[7], 328), 0, &[m, m, x]);
            // call width: 3
            g.call(t(g, ops[8], 329), 0, &[m, m, y.to_bits()]);
        }
        let y2 = sub(g.read::<f32>(at(0x42c)), k(g, 0x434));
        let y2 = wrap(g, y2, (0x43f, 0x44e, 0x458, 0x465));
        let p = at(0x4fa);
        moved(
            g,
            [0, 0x3f19_999a, 0x3fc0_0000],
            x,
            y2.to_bits(),
            p,
            [0x473, 0x4a4, 0x4c3, 0x4dc, 0x4f4],
        );
        copy(g, p, at(0x513));
        for (src, dst) in [(0x519u32, 0x51fu32), (0x525, 0x52b), (0x531, 0x537)] {
            copy(g, at(src), at(dst));
        }
        g.write::<u32>(on, 1);
    }
    let n = crate::sa_pool::take(g, img, call(0x55a), w(0x555));
    g.write::<u32>(n + 4, 0);
    if g.read::<u32>(w(0x560)) == 0 {
        g.write::<u32>(n, 0);
        g.write::<u32>(w(0x582), n);
    } else {
        let last = g.read::<u32>(w(0x570));
        g.write::<u32>(n, last);
        g.write::<u32>(last + 4, n);
    }
    g.write::<u32>(n + 8, n);
    g.write::<u32>(at(0x58f), n);
    g.write::<u32>(w(0x596), n);
    g.write::<u32>(w(0x59c), 1);
    Ret::eax(n)
}

/// [`models_place_0586`] (1454 bytes, 1 overlay).
pub const MODELS_PLACE_0586: Sig = Sig {
    len: 1454,
    hash: 0xfd45_9260_236f_ff0f,
    calls: &[
        (0x26, Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
        (0x55a, Callee::Sig(&crate::sa_pool::TAKE)),
        // __security_check_cookie
        (0x5a6, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0162`'s copy of [`models_place_0586`] (0x10003b70, 1454
/// bytes).
pub const MODELS_PLACE_0162: Sig = Sig {
    len: 1454,
    hash: 0x04c2_a22d_cc2b_bf3d,
    calls: &[
        (0x26, Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
        (0x55a, Callee::Sig(&crate::sa_pool::TAKE)),
        // __security_check_cookie
        (0x5a6, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0586`'s point objects and fan (0x10003d60, 1667 bytes; a table
/// callback, no arguments; a cookie frame, `ebp = E - 4`, the caster's
/// place at `ebp - 0x14`). The record `S = *G_S`, every record offset read
/// from the code (`magic_0162`'s copy at 0x10004150):
/// - three objects `O1 = S + 0x4c0`, `O2 = O1 + 0x130`, `O3 = O2 + 0x130`
///   started ([`start_object_plain`](crate::s4_misc::start_object_plain),
///   part 0, scales `K1`, `K2`, `K3`) at the first point ahead `P1`; `O1`
///   turned by `S.a`, `O2` by `S.a + K` and `O3` by `S.a - K` (both
///   brought into `-pi..pi`), each one's `y` less `K4` (written when the
///   next one starts), `O2` and `O3` with `+0xc8` = `±pi/4` and `+0xe0`
///   = `±15`;
/// - with `S.on` 1 the caster's model matrix `S.m1` as in
///   [`models_place_0586`] (slots 235, 277, 15, 16, 17, 330, 328, 329) and
///   `S.q` = `S.p2`;
/// - the second selected actor's model matrix `S.m2` as in
///   [`point_object_0586`] (slots 210 or 217/255, 211, 235, ...) with the
///   turn and place copied on (`S + 0x950..`), `S + 0x900..` = the copied
///   place; slot 399 `(actor, 1)`;
/// - the fan record `S + 0x81f0` (colours 0x80, sizes 0x18, kind 0x48)
///   with a task node ([`take`](crate::sa_pool::take), step `STEP`) on the
///   task list; the fan `S + 0x8438` (colour 0x30808080, 4 parts, 0.05)
///   started ([`sprite_fan_start`](crate::s6_misc::sprite_fan_start));
///   slot 371 `(0, 0)` and `(1, 0)`.
///
/// Returns slot 371's `eax`.
#[allow(clippy::too_many_lines)] // one straight sequence
pub fn point_objects_0586<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::ob_model::st0_f32;
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let rd = |g: &G, o: u32| g.read::<u32>(w(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let s = rd(g, 0x1b);
    // `[S + d]` at the operand `o`
    let at = |o: u32| s.wrapping_add(w(o));
    let cp = |g: &mut G, src: u32, dst: u32| {
        let x = g.read::<u32>(src);
        g.write::<u32>(dst, x);
    };
    let wrap = |g: &G, y: f32, (hi, step, lo, step2): (u32, u32, u32, u32)| {
        if y > k(g, hi) {
            sub(y, k(g, step))
        } else if k(g, lo) > y {
            add(y, k(g, step2))
        } else {
            y
        }
    };
    let start = |g: &mut G, c: u32, obj: u32, scale: f32, p: [u32; 4]| {
        crate::s4_misc::start_object_plain(g, img, call(c), obj, scale, 0);
        for (i, o) in (0u32..).zip(p) {
            cp(g, at(o), obj + 0xd0 + 4 * i);
        }
    };
    let o1 = at(0x25);
    let sc = k(g, 0x14);
    start(g, 0x2e, o1, sc, [0x34, 0x48, 0x54, 0x60]);
    let y = sub(g.read::<f32>(o1 + 0xd4), k(g, 0x76));
    cp(g, at(0x7c), o1 + 0xc4);
    let o2 = o1 + 0x130;
    g.write::<f32>(o1 + 0xd4, y);
    let sc = k(g, 0x3c);
    start(g, 0x99, o2, sc, [0x9f, 0xb3, 0xbf, 0xcb]);
    let a = add(g.read::<f32>(at(0xd9)), k(g, 0xe1));
    let a = wrap(g, a, (0xeb, 0xa7, 0xfb, 0xa7));
    g.write::<f32>(o2 + 0xc4, a);
    let y = sub(g.read::<f32>(o2 + 0xd4), k(g, 0x124));
    g.write::<u32>(o2 + 0xc8, 0x3f49_0fdb);
    g.write::<u32>(o2 + 0xe0, 0x4170_0000);
    let o3 = o2 + 0x130;
    g.write::<f32>(o2 + 0xd4, y);
    let sc = k(g, 0x10c);
    start(g, 0x14f, o3, sc, [0x155, 0x161, 0x16d, 0x179]);
    let b = sub(g.read::<f32>(at(0x187)), k(g, 0x18f));
    let b = wrap(g, b, (0x199, 0x1a3, 0x1ad, 0x1ba));
    g.write::<f32>(o3 + 0xc4, b);
    let y = sub(g.read::<f32>(o3 + 0xd4), k(g, 0x1d2));
    g.write::<u32>(o3 + 0xc8, 0xbf49_0fdb);
    g.write::<u32>(o3 + 0xe0, 0xc170_0000);
    g.write::<f32>(o3 + 0xd4, y);
    let actor = |g: &G, sel: u32, tbl: u32| {
        let i = u32::from(g.read::<u8>(rd(g, sel) + 2));
        (i, g.read::<u32>(w(tbl).wrapping_add(4 * i)))
    };
    if g.read::<u32>(at(0x1f5)) == 1 {
        let m1 = at(0x207);
        g.call(t(g, 0x201, 235), 0, &[m1]);
        let x = st0_f32(g.call(t(g, 0x215, 277), 0, &[rd(g, 0x21b)]).st0);
        let x = mul(x, k(g, 0x233));
        for o in [0x28u32, 0x14, 0] {
            g.write::<f32>(m1 + o, x);
        }
        let a15 = rd(g, 0x24c);
        let f15 = st0_f32(g.call(t(g, 0x238, 15), 0, &[a15]).st0);
        let v = mul(g.read::<f32>(m1), f15);
        g.write::<f32>(m1, v);
        let f16 = st0_f32(g.call(t(g, 0x263, 16), 0, &[a15]).st0);
        let v = mul(g.read::<f32>(m1 + 0x14), f16);
        g.write::<f32>(m1 + 0x14, v);
        let f17 = st0_f32(g.call(t(g, 0x27f, 17), 0, &[a15]).st0);
        let v = mul(g.read::<f32>(m1 + 0x28), f17);
        g.write::<f32>(m1 + 0x28, v);
        for (tb, n, ang) in [
            (0x29cu32, 330u32, 0x2acu32),
            (0x2c8, 328, 0x2c3),
            (0x2e7, 329, 0x2e2),
        ] {
            let ang = g.read::<u32>(at(ang));
            // call width: 3
            g.call(t(g, tb, n), 0, &[m1, m1, ang]);
        }
        for (src, dst) in [
            (0x2ffu32, 0x305u32),
            (0x30b, 0x311),
            (0x317, 0x31d),
            (0x323, 0x32c),
        ] {
            cp(g, at(src), at(dst));
        }
    }
    let (i, a) = actor(g, 0x331, 0x342);
    let m2 = at(0x337);
    if m2 == 0 {
        // `S + 0x8d0` null: no caller's `S` gets here
        let v = ebp.wrapping_sub(0x14);
        if rd(g, 0x363) == 0 && i < 0x20 {
            g.call(t(g, 0x374, 217), 0, &[i, v]);
            g.call(t(g, 0x386, 255), 0, &[i, v]);
        }
    } else {
        g.call(t(g, 0x350, 210), 0, &[a, m2]);
    }
    g.call(t(g, 0x39c, 211), 0, &[a, m2]);
    for (src, dst) in [
        (0x3aau32, 0x3b0u32),
        (0x3b6, 0x3bc),
        (0x3c2, 0x3c8),
        (0x3ce, 0x3d4),
        (0x3da, 0x3e0),
        (0x3e6, 0x3ec),
        (0x3f2, 0x3f8),
        (0x3fe, 0x404),
    ] {
        cp(g, at(src), at(dst));
    }
    // the actors are read between the calls: the 277 one first
    g.call(t(g, 0x409, 235), 0, &[m2]);
    let (_, a2) = actor(g, 0x417, 0x422);
    let x = st0_f32(g.call(t(g, 0x427, 277), 0, &[a2]).st0);
    let x = mul(x, k(g, 0x43f));
    for o in [0x28u32, 0x14, 0] {
        g.write::<f32>(m2 + o, x);
    }
    let (_, a3) = actor(g, 0x452, 0x45d);
    let f15 = st0_f32(g.call(t(g, 0x462, 15), 0, &[a3]).st0);
    let v = mul(g.read::<f32>(m2), f15);
    g.write::<f32>(m2, v);
    let f16 = st0_f32(g.call(t(g, 0x479, 16), 0, &[a3]).st0);
    let v = mul(f16, g.read::<f32>(m2 + 0x14));
    g.write::<f32>(m2 + 0x14, v);
    let f17 = st0_f32(g.call(t(g, 0x488, 17), 0, &[a3]).st0);
    let v = mul(g.read::<f32>(m2 + 0x28), f17);
    g.write::<f32>(m2 + 0x28, v);
    for (tb, n, ang) in [
        (0x4b2u32, 330u32, 0x4c2u32),
        (0x4de, 328, 0x4d9),
        (0x500, 329, 0x4fb),
    ] {
        let ang = g.read::<u32>(at(ang));
        // call width: 3
        g.call(t(g, tb, n), 0, &[m2, m2, ang]);
    }
    for (src, dst) in [
        (0x515u32, 0x51bu32),
        (0x521, 0x527),
        (0x52d, 0x533),
        (0x539, 0x53f),
    ] {
        cp(g, at(src), at(dst));
    }
    let (_, a4) = actor(g, 0x544, 0x551);
    // call width: 2
    g.call(t(g, 0x556, 399), 0, &[a4, 1]);
    let fan = at(0x564);
    g.write::<u32>(fan + 0x81, 0x8080_8080);
    g.write::<u8>(fan + 0x80, 0x80);
    g.write::<u16>(fan + 0x85, 0x8080);
    g.write::<u8>(fan + 0x87, 0);
    g.write::<u32>(fan + 0x88, 0);
    g.write::<u16>(fan + 0x98, 0x48);
    g.write::<u32>(fan + 0x90, 0x18);
    g.write::<u32>(fan + 0x8c, 0x18);
    g.write::<u32>(fan + 0x94, 0);
    let n = crate::sa_pool::take(g, img, call(0x5c3), w(0x56c));
    g.write::<u32>(n + 0x14, fan);
    g.write::<u32>(n + 4, 0);
    if g.read::<u32>(w(0x5c9)) == 0 {
        g.write::<u32>(n, 0);
        g.write::<u32>(w(0x5ef), n);
    } else {
        let last = g.read::<u32>(w(0x5dd));
        g.write::<u32>(n, last);
        g.write::<u32>(last + 4, n);
    }
    g.write::<u32>(n + 8, n);
    let c = at(0x5f8);
    g.write::<u32>(w(0x5fd), n);
    g.write::<u32>(c + 0xf0, 0x3080_8080);
    g.write::<u16>(c + 0x124, 4);
    for (o, v) in [
        (0x120u32, 0u32),
        (0x110, 0),
        (0xf8, 0),
        (0xfc, 0x3d4c_cccd),
        (0x100, 0),
    ] {
        g.write::<u32>(c + o, v);
    }
    // the return address under the two saved registers
    let ff = Frame {
        at: call(0x64a),
        esp: ebp.wrapping_sub(0x18 + 8 + 4),
        ecx: c,
        ..*fr
    };
    crate::s6_misc::sprite_fan_start(g, img, &ff);
    // call width: 2
    g.call(t(g, 0x64f, 371), 0, &[0, 0]);
    // call width: 2
    Ret::eax(g.call(t(g, 0x660, 371), 0, &[1, 0]).eax)
}

/// [`point_objects_0586`] (1667 bytes, 1 overlay).
pub const POINT_OBJECTS_0586: Sig = Sig {
    len: 1667,
    hash: 0x86a9_d9d7_272c_f65e,
    calls: &[
        (0x2e, Callee::Sig(&crate::s4_misc::START_OBJECT_PLAIN)),
        (0x99, Callee::Sig(&crate::s4_misc::START_OBJECT_PLAIN)),
        (0x14f, Callee::Sig(&crate::s4_misc::START_OBJECT_PLAIN)),
        (0x5c3, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x64a, Callee::Sig(&crate::s6_misc::SPRITE_FAN_START)),
        // __security_check_cookie
        (0x67b, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0162`'s copy of [`point_objects_0586`] (0x10004150, 1667
/// bytes).
pub const POINT_OBJECTS_0162: Sig = Sig {
    len: 1667,
    hash: 0x4743_2b59_4c6f_cfde,
    calls: &[
        (0x2e, Callee::Sig(&crate::s4_misc::START_OBJECT_PLAIN)),
        (0x99, Callee::Sig(&crate::s4_misc::START_OBJECT_PLAIN)),
        (0x14f, Callee::Sig(&crate::s4_misc::START_OBJECT_PLAIN)),
        (0x5c3, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x64a, Callee::Sig(&crate::s6_misc::SPRITE_FAN_START)),
        // __security_check_cookie
        (0x67b, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0587`'s spin step (0x10003770, 678 bytes; a table callback, no
/// arguments; a cookie frame, `ebp = E - 4`: the matrix `L` at `ebp -
/// 0x54`, the point `V` at `ebp - 0x14`). `S = *G_S`:
/// - the spin `S + 0xa44` up by `K` (into `-pi..pi`), the model matrix
///   `S + 0x980` scaled and turned by `S + 0xa40`
///   ([`scale_rotate`](crate::sa_misc::scale_rotate)); `S + 0x7f4`, `+0x924`
///   and `+0xa54` up by `K1`;
/// - when the count `S + 0x30f4` runs out: a new aim `S + 0xa00..` from
///   the place `S + 0xa50..` plus `(0, 0, slot 404 (10) + K2)` turned by
///   slot 314 (pi) (slots 235, 329, 229) and `y + K3`, `w` 20; the count
///   0x28;
/// - `s = count / K4`: the aim's `y` = `(1 - s) (y - K5) + s (y + K5)`;
///   the scale `(1 - s)² 0 + (1 - s) s K6 + s² 0` into `S + 0x9c0..0x9c8`;
/// - slot 371 `(0, rand % 0x8000)` and `(1, rand % 0x8000)`.
///
/// Returns slot 371's `eax`.
#[allow(clippy::too_many_lines)] // one straight sequence
pub fn spin_step_0587<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::ob_model::st0_f32;
    use crate::sb_sse::{add, cvt, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let ebp = fr.esp.wrapping_sub(4);
    let (lm, v) = (ebp.wrapping_sub(0x54), ebp.wrapping_sub(0x14));
    let s = g.read::<u32>(w(0x13));
    let mut y = add(g.read::<f32>(s + 0xa44), k(g, 0x23));
    if y > k(g, 0x2a) {
        y = sub(y, k(g, 0x34));
    } else if k(g, 0x3e) > y {
        y = add(y, k(g, 0x4b));
    }
    g.write::<f32>(s + 0xa44, y);
    crate::sa_misc::scale_rotate(g, img, img.rel32_target(f + 0x65), s + 0x980, s + 0xa40);
    let d = k(g, 0x75);
    for o in [0x7f4u32, 0x924] {
        let x = add(g.read::<f32>(s + o), d);
        g.write::<f32>(s + o, x);
    }
    let n = g.read::<u32>(s + 0x30f4).wrapping_sub(1);
    g.write::<u32>(s + 0x30f4, n);
    let x = add(g.read::<f32>(s + 0xa54), d);
    g.write::<f32>(s + 0xa54, x);
    let one_op = if n == 0 {
        g.call(t(g, 0xcc, 235), 0, &[lm]);
        let a = st0_f32(g.call(t(g, 0xd9, 314), 0, &[0x4049_0fdb]).st0);
        // call width: 3
        g.call(t(g, 0xff, 329), 0, &[lm, lm, a.to_bits()]);
        g.write::<u32>(v, 0);
        g.write::<u32>(v + 4, 0);
        let r = st0_f32(g.call(t(g, 0x10c, 404), 0, &[0x4120_0000]).st0);
        g.write::<f32>(v + 8, add(r, k(g, 0x13c)));
        g.write::<u32>(v + 0xc, ONE);
        // call width: 3
        g.call(t(g, 0x14d, 229), 0, &[v, lm, v]);
        let x = add(g.read::<f32>(s + 0xa50), g.read::<f32>(v));
        g.write::<f32>(s + 0xa00, x);
        let x = add(g.read::<f32>(s + 0xa54), k(g, 0x194));
        g.write::<f32>(s + 0xa04, x);
        let x = add(g.read::<f32>(s + 0xa58), g.read::<f32>(v + 8));
        g.write::<u32>(s + 0xa0c, 0x41a0_0000);
        g.write::<f32>(s + 0xa08, x);
        g.write::<u32>(s + 0x30f4, 0x28);
        0x176
    } else {
        0x7d
    };
    let sv = div(cvt(g.read::<u32>(s + 0x30f4)), k(g, 0x1e0));
    let y = g.read::<f32>(s + 0xa54);
    let lo = sub(y, k(g, 0x1eb));
    let hi = add(y, k(g, 0x1f3));
    let u = sub(k(g, one_op), sv);
    let aim = add(mul(lo, u), mul(hi, sv));
    let sc = add(
        add(mul(mul(u, u), 0.0), mul(mul(u, sv), k(g, 0x21a))),
        mul(mul(sv, sv), 0.0),
    );
    g.write::<f32>(s + 0xa04, aim);
    for o in [0x9c8u32, 0x9c4, 0x9c0] {
        g.write::<f32>(s + o, sc);
    }
    let r = g.rand().cast_signed() % 0x8000;
    // call width: 2
    g.call(t(g, 0x266, 371), 0, &[0, r.cast_unsigned()]);
    let r = g.rand().cast_signed() % 0x8000;
    // call width: 2
    Ret::eax(g.call(t(g, 0x286, 371), 0, &[1, r.cast_unsigned()]).eax)
}

/// [`spin_step_0587`] (678 bytes, 1 overlay).
pub const SPIN_STEP_0587: Sig = Sig {
    len: 678,
    hash: 0x2ad1_7a27_36cd_cf17,
    calls: &[
        (0x65, Callee::Sig(&crate::sa_misc::SCALE_ROTATE)),
        // __security_check_cookie
        (0x29e, Callee::Any),
    ],
    imports: &[(0x250, "rand")],
};

/// `magic_0587`'s second start (0x10003b20, 654 bytes; a table callback,
/// no arguments, no frame). `S = *G_S`:
/// - slot 371 `(0, 0x10000)` and `(1, 0x20000)`;
/// - the objects `S + 0x720` and `S + 0x850` each taken out of `G_LIST`
///   ([`list_remove`](crate::l5_objhost::list_remove)): then `**G_P` =
///   `ITEM`, slot 713 `(O)` and bit 15 of u16 `O + 0x124` cleared;
/// - the spin `S + 0xa44` = `S + 0x30f0`, `S + 0xa54` = `S + 0xa64`; the
///   model matrix `S + 0x980` scaled and turned by `S + 0xa40`
///   ([`scale_rotate`](crate::sa_misc::scale_rotate));
/// - the objects `S + 0x260` (part 2, value 0x15, scale `K1`) and `S`
///   itself (part 0, value 0x3a, scale `K2`) started
///   ([`start_object_anim`](crate::s4_misc::start_object_anim)) at the
///   point `S + 0xa60` turned by `S + 0x30f0`, their `y` less `K3` and
///   `K4`;
/// - the sizes `S + 0xbfc..` (0, 40, 0, 40, 150, 150) with a task node
///   ([`take`](crate::sa_pool::take), step `STEP`, kind 1) on the task
///   list; the fader `S + 0xc18`
///   ([`fader_task`](crate::s4_small::fader_task)) with bits 0-1 of `+0x2c`,
///   (1, 1, 0, 4, 2, 0) and `+0x24` 0x3a;
/// - slot 560 `(slot 557 () | 4)`.
///
/// Returns slot 560's `eax`.
#[allow(clippy::too_many_lines)] // one straight sequence
pub fn second_start_0587<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::sub;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    // call width: 2
    g.call(t(g, 0x1, 371), 0, &[0, 0x1_0000]);
    // call width: 2
    g.call(t(g, 0x18, 371), 0, &[1, 0x2_0000]);
    let s = g.read::<u32>(w(0x2d));
    for (o, c, l, p, item, tb) in [
        (0x720u32, 0x42u32, 0x3du32, 0x50u32, 0x57u32, 0x5cu32),
        (0x850, 0x80, 0x7b, 0x89, 0x90, 0x95),
    ] {
        let obj = s + o;
        if crate::l5_objhost::list_remove(g, img, call(c), w(l), obj) != 0 {
            g.write::<u32>(g.read::<u32>(w(p)), w(item));
            g.call(t(g, tb, 713), 0, &[obj]);
            let b = g.read::<u16>(obj + 0x124) & 0x7fff;
            g.write::<u16>(obj + 0x124, b);
        }
    }
    let turn = g.read::<u32>(s + 0x30f0);
    g.write::<u32>(s + 0xa44, turn);
    let y = g.read::<u32>(s + 0xa64);
    g.write::<u32>(s + 0xa54, y);
    crate::sa_misc::scale_rotate(g, img, call(0xd1), s + 0x980, s + 0xa40);
    // (the object, its call, part, value, scale's operand, `y`'s operand)
    for (obj, c, part, value, sc, ky) in [
        (s + 0x260, 0xebu32, 2u32, 0x15u32, 0xd9u32, 0x13fu32),
        (s, 0x155, 0, 0x3a, 0xf9, 0x19b),
    ] {
        let scale = k(g, sc);
        crate::s4_misc::start_object_anim(g, img, call(c), obj, scale, part, value);
        for i in 0..4 {
            let x = g.read::<u32>(s + 0xa60 + 4 * i);
            g.write::<u32>(obj + 0xd0 + 4 * i, x);
        }
        let turn = g.read::<u32>(s + 0x30f0);
        g.write::<u32>(obj + 0xc4, turn);
        // the first's `y` from `S + 0xa54`, the second's from its own copy
        let from = if part == 2 { s + 0xa54 } else { obj + 0xd4 };
        let y = sub(g.read::<f32>(from), k(g, ky));
        g.write::<f32>(obj + 0xd4, y);
    }
    let q = s + 0xbfc;
    for (o, v) in [
        (0u32, 0u32),
        (4, 0x28),
        (8, 0),
        (0xc, 0x28),
        (0x14, 0x96),
        (0x10, 0x96),
    ] {
        g.write::<u32>(q + o, v);
    }
    g.write::<u16>(q + 0x18, 0);
    let n = crate::sa_pool::take(g, img, call(0x1eb), w(0x1b9));
    g.write::<u8>(n + 0x18, 1);
    g.write::<u32>(n + 0x14, q);
    g.write::<u32>(n + 4, 0);
    if g.read::<u32>(w(0x1f1)) == 0 {
        g.write::<u32>(n, 0);
        g.write::<u32>(w(0x21a), n);
    } else {
        let last = g.read::<u32>(w(0x208));
        g.write::<u32>(n, last);
        g.write::<u32>(last + 4, n);
    }
    g.write::<u32>(n + 8, n);
    g.write::<u32>(w(0x22a), n);
    let fd = s + 0xc18;
    // push ebx; push esi; push edi: the return address at `E - 0x10`
    let ff = Frame {
        at: call(0x22f),
        esp: fr.esp.wrapping_sub(0x10),
        ecx: fd,
        ..*fr
    };
    crate::s4_small::fader_task(g, img, &ff);
    let s557 = t(g, 0x234, 557);
    let b = g.read::<u16>(fd + 0x2c) | 3;
    g.write::<u16>(fd + 0x2c, b);
    for (o, v) in [
        (4u32, ONE),
        (8, ONE),
        (0xc, 0),
        (0x10, 0x4080_0000),
        (0x14, 0x4000_0000),
        (0x18, 0),
        (0x24, 0x3a),
    ] {
        g.write::<u32>(fd + o, v);
    }
    let r = g.call(s557, 0, &[]).eax | 4;
    Ret::eax(g.call(t(g, 0x27b, 560), 0, &[r]).eax)
}

/// [`second_start_0587`] (654 bytes, 1 overlay).
pub const SECOND_START_0587: Sig = Sig {
    len: 654,
    hash: 0x8ea5_a43b_b6d1_130a,
    calls: &[
        (0x42, Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
        (0x80, Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
        (0xd1, Callee::Sig(&crate::sa_misc::SCALE_ROTATE)),
        (0xeb, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x155, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x1eb, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x22f, Callee::Sig(&crate::s4_small::FADER_TASK)),
    ],
    imports: &[],
};

/// `magic_0587`'s four trails `S + 0xe48 + 0x8a8 i` reset as its starts
/// inline it: the animation key, frame -1, time 0x200, playing; colours
/// 0x80808080 (alpha 0x18) and 0, scales 1.0, the step `step`, 0x96
/// parts, no points.
fn trails_reset_0587<G: Game>(g: &mut G, s: u32, key: u32, step: u32) {
    for i in 0..4 {
        let tr = s + 0xe48 + 0x8a8 * i;
        g.write::<u16>(tr + 8, 0xffff);
        g.write::<u16>(tr + 0xa, 0x200);
        g.write::<u32>(tr, key);
        g.write::<u32>(tr + 4, 1);
        g.write::<u32>(tr + 0x80c, 0x8080_8080);
        g.write::<u32>(tr + 0x810, 0);
        g.write::<u32>(tr + 0x820, 0);
        g.write::<u32>(tr + 0x814, ONE);
        g.write::<u32>(tr + 0x818, ONE);
        for j in 0..0x20 {
            g.write::<u32>(tr + 0x828 + 4 * j, 0);
        }
        g.write::<u32>(tr + 0x824, 0x96);
        g.write::<u32>(tr + 0x81c, step);
        g.write::<u8>(tr + 0x80f, 0x18);
    }
}

/// `magic_0587`'s pool take as its starts inline it (operands: the u16
/// count, the free list, its stores with more than one node and with the
/// last, the node's step immediate, the count's store): the free list's
/// first node unlinked and reset (`+0x8` itself, `+0x10` the step). Gives
/// the node and `eax` after it (0xffff; with the pool empty no node, 0,
/// and `empty`).
fn pool_take_0587<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    ops: [u32; 6],
    empty: u32,
) -> (u32, u32) {
    let w = |o: u32| img.word(f + o);
    let left = g.read::<u16>(w(ops[0]));
    if left == 0 {
        return (0, empty);
    }
    let n = g.read::<u32>(w(ops[1]));
    let (prev, next) = (g.read::<u32>(n), g.read::<u32>(n + 4));
    g.write::<u32>(prev + 4, next);
    let (next, prev) = (g.read::<u32>(n + 4), g.read::<u32>(n));
    g.write::<u32>(next, prev);
    if left <= 1 {
        g.write::<u32>(w(ops[3]), 0);
    } else {
        let next = g.read::<u32>(n + 4);
        g.write::<u32>(w(ops[2]), next);
    }
    for o in [0u32, 4, 0xc, 0x14] {
        g.write::<u32>(n + o, 0);
    }
    g.write::<u32>(n + 8, n);
    g.write::<u32>(n + 0x10, w(ops[4]));
    g.write::<u8>(n + 0x18, 0);
    g.write::<u16>(w(ops[5]), left - 1);
    (n, 0xffff)
}

/// `magic_0587`'s trails start (0x10002bc0, 763 bytes; a table callback,
/// no arguments; an `ebp` frame keeping `S` at `ebp - 8`). `S = *G_S`:
/// - the object `S + 0x5f0` taken out of `G_LIST`
///   ([`list_remove`](crate::l5_objhost::list_remove)): then `**G_P` =
///   `ITEM`, slot 713 `(O)` and bit 15 of u16 `O + 0x124` cleared;
/// - the object `S + 0x390` started
///   ([`start_object_anim`](crate::s4_misc::start_object_anim), part 3,
///   value 0, scale `K`) at `S + 0xa60` turned by `S + 0x30f0`;
/// - `S + 0xa84` 15, `+0xa94` 0.75, the turn `S + 0xa40` `(0, S + 0x30f0,
///   0)` on the model matrix `S + 0x980`
///   ([`scale_rotate`](crate::sa_misc::scale_rotate)); the place `S +
///   0xa50` = `S + 0xa60` with `y` up by `K1`;
/// - the four trails `S + 0xe48 + 0x8a8 i` reset: the animation key
///   `(*G_A)[0x20]`, frame -1, time 0x200, playing; colours 0x80808080
///   (alpha 0x18) and 0, scales 1.0, step 4.0, 0x96 parts, no points;
/// - slot 256 `(*G_O, 1.0, 1.0, 1.0, 1)`; the sizes `S + 0xbe0..` (0,
///   0x50, 0, 0, 0x2d, 0x2d) with a task node taken from the DLL's pool
///   as `take` does it inline (the free list `G_FREE`, the count
///   `G_COUNT`; an empty pool leaves no node, and the stores then go to
///   address 0 as in the original) on the task list (`G_FIRST`,
///   `G_LAST`).
///
/// Returns the old last node, or 0xffff (0 with the pool empty) for an
/// empty list.
#[allow(clippy::too_many_lines)] // one straight sequence
pub fn trails_start_0587<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::add;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let s = g.read::<u32>(w(0x9));
    let obj = s + 0x5f0;
    if crate::l5_objhost::list_remove(g, img, call(0x20), w(0x18), obj) != 0 {
        g.write::<u32>(g.read::<u32>(w(0x29)), w(0x30));
        g.call(t(g, 0x35, 713), 0, &[obj]);
        let b = g.read::<u16>(obj + 0x124) & 0x7fff;
        g.write::<u16>(obj + 0x124, b);
    }
    let o1 = s + 0x390;
    let scale = k(g, 0x54);
    crate::s4_misc::start_object_anim(g, img, call(0x68), o1, scale, 3, 0);
    for i in 0..4 {
        let x = g.read::<u32>(s + 0xa60 + 4 * i);
        g.write::<u32>(o1 + 0xd0 + 4 * i, x);
    }
    let turn = g.read::<u32>(s + 0x30f0);
    g.write::<u32>(o1 + 0xc4, turn);
    g.write::<u32>(s + 0xa84, 0x4170_0000);
    g.write::<u32>(s + 0xa94, 0x3f40_0000);
    g.write::<u32>(s + 0xa40, 0);
    let turn = g.read::<u32>(s + 0x30f0);
    g.write::<u32>(s + 0xa44, turn);
    g.write::<u32>(s + 0xa48, 0);
    crate::sa_misc::scale_rotate(g, img, call(0xe6), s + 0x980, s + 0xa40);
    let y = add(g.read::<f32>(s + 0xa64), k(g, 0xfc));
    for (src, dst) in [(0xa60u32, 0xa50u32), (0xa68, 0xa58), (0xa6c, 0xa5c)] {
        let x = g.read::<u32>(s + src);
        g.write::<u32>(s + dst, x);
    }
    g.write::<f32>(s + 0xa54, y);
    let key = g.read::<u32>(g.read::<u32>(w(0x127)) + 0x20);
    trails_reset_0587(g, s, key, 0x4080_0000);
    let o = g.read::<u32>(w(0x1f9));
    // call width: 5
    g.call(t(g, 0x1d1, 256), 0, &[o, ONE, ONE, ONE, 1]);
    let q = s + 0xbe0;
    for (o, v) in [
        (0u32, 0u32),
        (4, 0x50),
        (8, 0),
        (0xc, 0),
        (0x14, 0x2d),
        (0x10, 0x2d),
    ] {
        g.write::<u32>(q + o, v);
    }
    g.write::<u16>(q + 0x18, 0);
    let (n, mut eax) = pool_take_0587(g, img, f, [0x205, 0x24c, 0x269, 0x271, 0x2a2, 0x2ad], 0);
    let first = g.read::<u32>(w(0x2b3));
    g.write::<u32>(n.wrapping_add(0x14), q);
    g.write::<u8>(n.wrapping_add(0x18), 1);
    g.write::<u32>(n.wrapping_add(4), 0);
    if first == 0 {
        g.write::<u32>(n, 0);
        g.write::<u32>(w(0x2ea), n);
        g.write::<u32>(w(0x2f0), n);
    } else {
        let last = g.read::<u32>(w(0x2cc));
        g.write::<u32>(n, last);
        g.write::<u32>(last.wrapping_add(4), n);
        g.write::<u32>(w(0x2d7), n);
        eax = last;
    }
    g.write::<u32>(n.wrapping_add(8), n);
    Ret::eax(eax)
}

/// [`trails_start_0587`] (763 bytes, 1 overlay).
pub const TRAILS_START_0587: Sig = Sig {
    len: 763,
    hash: 0xf8c7_86a7_7355_98a7,
    calls: &[
        (0x20, Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
        (0x68, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0xe6, Callee::Sig(&crate::sa_misc::SCALE_ROTATE)),
    ],
    imports: &[],
};

/// `magic_0587`'s third start (0x100032f0, 1140 bytes; a table callback,
/// no arguments; an `ebp` frame). `S = *G_S`:
/// - the four trails reset ([`trails_reset_0587`], the animation key
///   `(*G_A)[0x20]`, step 2.0); `S + 0x30fc` 1;
/// - the object `S + 0x130` taken out of `G_LIST`
///   ([`list_remove`](crate::l5_objhost::list_remove)): then `**G_P` =
///   `ITEM`, slot 713 `(O)` and bit 15 of u16 `O + 0x124` cleared;
/// - the objects `S + 0x720` (part 6, scale `K1`; its `+0x64` times `K2`)
///   and `S + 0x850` (part 7, scale `K3`) started
///   ([`start_object_anim`](crate::s4_misc::start_object_anim), value 0)
///   at `S + 0xa60`, their `y` less `K4`/`K5`, turned by `S + 0x30f0`;
/// - `S + 0xa54` = `S + 0xa64` less `K6`; the spin `S + 0xa44` = `S +
///   0x30f0` plus `K7` (into `-pi..pi`) on the model matrix `S + 0x980`
///   ([`scale_rotate`](crate::sa_misc::scale_rotate));
/// - the matrix `S + 0x9c0..0xa40` (diagonal 2, 1, 1, 1; `+0x48` -20,
///   `+0x4c` 20), `S + 0x30f4` 1, `+0x30f8` 100;
/// - a task node from the pool ([`pool_take_0587`]) kept at `S + 0x30ec`
///   and appended to the task list.
///
/// Returns the old last node, or 0xffff for an empty list (with the pool
/// empty the model matrix call's `eax`).
#[allow(clippy::too_many_lines)] // one straight sequence
pub fn third_start_0587<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let key = g.read::<u32>(g.read::<u32>(w(0x7)) + 0x20);
    let s = g.read::<u32>(w(0xe));
    trails_reset_0587(g, s, key, 0x4000_0000);
    let obj = s + 0x130;
    g.write::<u32>(s + 0x30fc, 1);
    if crate::l5_objhost::list_remove(g, img, call(0xce), w(0xb7), obj) != 0 {
        g.write::<u32>(g.read::<u32>(w(0xd8)), w(0xdf));
        g.call(t(g, 0xe4, 713), 0, &[obj]);
        let b = g.read::<u16>(obj + 0x124) & 0x7fff;
        g.write::<u16>(obj + 0x124, b);
    }
    // (the object, its call, part, scale's operand, `y`'s operand)
    for (o, c, part, sc, ky) in [
        (0x720u32, 0x117u32, 6u32, 0x103u32, 0x14du32),
        (0x850, 0x199, 7, 0x125, 0x1c7),
    ] {
        let obj = s + o;
        let scale = k(g, sc);
        crate::s4_misc::start_object_anim(g, img, call(c), obj, scale, part, 0);
        for i in 0..4 {
            let x = g.read::<u32>(s + 0xa60 + 4 * i);
            g.write::<u32>(obj + 0xd0 + 4 * i, x);
        }
        let y = sub(g.read::<f32>(obj + 0xd4), k(g, ky));
        g.write::<f32>(obj + 0xd4, y);
        if part == 6 {
            let x = mul(g.read::<f32>(obj + 0x64), k(g, 0x177));
            let turn = g.read::<u32>(s + 0x30f0);
            g.write::<u32>(obj + 0xc4, turn);
            g.write::<f32>(obj + 0x64, x);
        } else {
            let turn = g.read::<u32>(s + 0x30f0);
            g.write::<u32>(obj + 0xc4, turn);
        }
    }
    let y = sub(g.read::<f32>(s + 0xa64), k(g, 0x200));
    g.write::<f32>(s + 0xa54, y);
    let mut a = add(g.read::<f32>(s + 0x30f0), k(g, 0x218));
    if a > k(g, 0x21f) {
        a = sub(a, k(g, 0x229));
    } else if k(g, 0x233) > a {
        a = add(a, k(g, 0x240));
    }
    g.write::<f32>(s + 0xa44, a);
    let eax = crate::sa_misc::scale_rotate(g, img, call(0x25a), s + 0x980, s + 0xa40);
    let two = 0x4000_0000;
    let mut m = [0u32; 32];
    for (i, v) in [
        (0usize, two),
        (1, two),
        (2, two),
        (3, ONE),
        (7, ONE),
        (11, ONE),
        (15, ONE),
    ] {
        m[i] = v;
    }
    for (i, v) in [
        (18usize, 0xc1a0_0000u32),
        (19, 0x41a0_0000),
        (23, ONE),
        (27, ONE),
        (31, ONE),
    ] {
        m[i] = v;
    }
    for (i, v) in (0u32..).zip(m) {
        g.write::<u32>(s + 0x9c0 + 4 * i, v);
    }
    g.write::<u32>(s + 0x30f4, 1);
    g.write::<u32>(s + 0x30f8, 0x64);
    let (n, mut eax) = pool_take_0587(g, img, f, [0x261, 0x3c7, 0x3e4, 0x3ec, 0x41d, 0x428], eax);
    let first = g.read::<u32>(w(0x42e));
    g.write::<u32>(s + 0x30ec, n);
    g.write::<u32>(n.wrapping_add(4), 0);
    if first == 0 {
        g.write::<u32>(n, 0);
        g.write::<u32>(w(0x463), n);
        g.write::<u32>(w(0x469), n);
    } else {
        let last = g.read::<u32>(w(0x445));
        g.write::<u32>(n, last);
        g.write::<u32>(last.wrapping_add(4), n);
        g.write::<u32>(w(0x450), n);
        eax = last;
    }
    g.write::<u32>(n.wrapping_add(8), n);
    Ret::eax(eax)
}

/// [`third_start_0587`] (1140 bytes, 1 overlay).
pub const THIRD_START_0587: Sig = Sig {
    len: 1140,
    hash: 0xd060_a379_b3ae_b67e,
    calls: &[
        (0xce, Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
        (0x117, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x199, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x25a, Callee::Sig(&crate::sa_misc::SCALE_ROTATE)),
    ],
    imports: &[],
};

/// `magic_0587`'s first start (0x10002520, 1161 bytes; a table callback,
/// no arguments; a cookie frame, `ebp = E - 4`: the matrix `L` at `ebp -
/// 0x54`, the point `V` at `ebp - 0x14`). `S = *G_S`:
/// - the object `S + 0x5f0` started
///   ([`start_object_anim`](crate::s4_misc::start_object_anim), part 5,
///   value 0, scale `K`) at `S + 0xa60` turned by `S + 0x30f0`, then
///   moved by `(0, 0, 3700)` turned by that (slots 235, 329, 229); the
///   move over `K1` into `S + 0xa94`/`+0xa9c`;
/// - the turn `S + 0xa40` less `K2` and `S + 0xa48` plus `K3` (into
///   `-pi..pi`) on the model matrix `S + 0x980`
///   ([`scale_rotate`](crate::sa_misc::scale_rotate));
/// - `S + 0xa80` = `(50, 0, -1200)` turned (slot 229); the place `S +
///   0xa50` = `S + 0xa60` plus it, `S + 0xa90` that plus `(10, 0, 0)`
///   turned; `S + 0xa80`, `+0xa88` over `K4`;
/// - slot 256 `(*G_O, 0.5, 0.5, 0.5, 1)`; the four trails reset
///   ([`trails_reset_0587`], step 4.0); `S + 0x30fc` 1; without a task
///   node at `S + 0x30e8` one from the pool ([`pool_take_0587`]) appended
///   to the task list and kept there.
///
/// Returns the old last node, 0xffff for an empty list, or -1 (a node
/// already there, or the pool empty).
#[allow(clippy::too_many_lines)] // one straight sequence
pub fn first_start_0587<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, div, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let (lm, v) = (ebp.wrapping_sub(0x54), ebp.wrapping_sub(0x14));
    let s = g.read::<u32>(w(0x1b));
    let o = s + 0x5f0;
    let scale = k(g, 0x14);
    crate::s4_misc::start_object_anim(g, img, call(0x33), o, scale, 5, 0);
    for i in 0..4 {
        let x = g.read::<u32>(s + 0xa60 + 4 * i);
        g.write::<u32>(o + 0xd0 + 4 * i, x);
    }
    let turn = g.read::<u32>(s + 0x30f0);
    g.write::<u32>(o + 0xc4, turn);
    g.call(t(g, 0x78, 235), 0, &[lm]);
    let turn = g.read::<u32>(s + 0x30f0);
    // call width: 3
    g.call(t(g, 0x9a, 329), 0, &[lm, lm, turn]);
    let set_v = |g: &mut G, xyz: [u32; 3]| {
        for (i, x) in (0u32..).zip(xyz) {
            g.write::<u32>(v + 4 * i, x);
        }
        g.write::<u32>(v + 0xc, ONE);
    };
    set_v(g, [0, 0, 0x4567_4000]);
    // call width: 3
    g.call(t(g, 0xb3, 229), 0, &[v, lm, v]);
    let (x, z) = (g.read::<f32>(v), g.read::<f32>(v + 8));
    let ox = add(x, g.read::<f32>(o + 0xd0));
    g.write::<f32>(o + 0xd0, ox);
    let oz = add(z, g.read::<f32>(o + 0xd8));
    g.write::<f32>(o + 0xd8, oz);
    g.write::<f32>(s + 0xa94, div(x, k(g, 0xf4)));
    g.write::<f32>(s + 0xa9c, div(z, k(g, 0x107)));
    let (lo, hi, step) = (k(g, 0x117), k(g, 0x138), k(g, 0x154));
    let wrap = |a: f32| {
        if a > hi {
            sub(a, step)
        } else if lo > a {
            add(a, step)
        } else {
            a
        }
    };
    let a0 = wrap(sub(g.read::<f32>(s + 0xa40), k(g, 0x14c)));
    g.write::<f32>(s + 0xa40, a0);
    let a2 = wrap(add(g.read::<f32>(s + 0xa48), hi));
    g.write::<f32>(s + 0xa48, a2);
    crate::sa_misc::scale_rotate(g, img, call(0x1a0), s + 0x980, s + 0xa40);
    g.write::<u32>(s + 0xa84, 0);
    set_v(g, [0x4248_0000, 0, 0xc496_0000]);
    let p = s + 0xa80;
    // call width: 3
    g.call(t(g, 0x1ad, 229), 0, &[p, lm, v]);
    let x = add(g.read::<f32>(s + 0xa60), g.read::<f32>(p));
    set_v(g, [0x4120_0000, 0, 0]);
    g.write::<f32>(s + 0xa50, x);
    for (dst, src, d) in [(0xa54u32, 0xa64u32, 0xa84u32), (0xa58, 0xa68, 0xa88)] {
        let x = add(g.read::<f32>(s + src), g.read::<f32>(s + d));
        g.write::<f32>(s + dst, x);
    }
    for (src, dst) in [(0xa6cu32, 0xa5cu32), (0xa50, 0xa90), (0xa58, 0xa98)] {
        let x = g.read::<u32>(s + src);
        g.write::<u32>(s + dst, x);
    }
    // call width: 3
    g.call(t(g, 0x277, 229), 0, &[v, lm, v]);
    let x = add(g.read::<f32>(v), g.read::<f32>(s + 0xa90));
    g.write::<f32>(s + 0xa90, x);
    let z = add(g.read::<f32>(s + 0xa98), g.read::<f32>(v + 8));
    g.write::<f32>(s + 0xa98, z);
    let x = div(g.read::<f32>(p), k(g, 0x2d2));
    g.write::<f32>(p, x);
    let z = div(g.read::<f32>(s + 0xa88), k(g, 0x2ed));
    g.write::<f32>(s + 0xa88, z);
    let half = 0x3f00_0000;
    let ob = g.read::<u32>(w(0x2fb));
    // call width: 5
    g.call(t(g, 0x291, 256), 0, &[ob, half, half, half, 1]);
    let key = g.read::<u32>(g.read::<u32>(w(0x308)) + 0x20);
    trails_reset_0587(g, s, key, 0x4080_0000);
    let have = g.read::<u32>(s + 0x30e8);
    g.write::<u32>(s + 0x30fc, 1);
    if have != 0 {
        return Ret::eax(u32::MAX);
    }
    let (n, mut eax) = pool_take_0587(
        g,
        img,
        f,
        [0x3ce, 0x3dd, 0x3fa, 0x402, 0x433, 0x43e],
        u32::MAX,
    );
    let first = g.read::<u32>(w(0x444));
    g.write::<u32>(n.wrapping_add(4), 0);
    if first == 0 {
        g.write::<u32>(n, 0);
        g.write::<u32>(w(0x466), n);
    } else {
        let last = g.read::<u32>(w(0x453));
        g.write::<u32>(n, last);
        g.write::<u32>(last.wrapping_add(4), n);
        eax = last;
    }
    g.write::<u32>(n.wrapping_add(8), n);
    g.write::<u32>(w(0x46f), n);
    g.write::<u32>(s + 0x30e8, n);
    Ret::eax(eax)
}

/// [`first_start_0587`] (1161 bytes, 1 overlay).
pub const FIRST_START_0587: Sig = Sig {
    len: 1161,
    hash: 0xc63c_69d9_edcc_4c7f,
    calls: &[
        (0x33, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x1a0, Callee::Sig(&crate::sa_misc::SCALE_ROTATE)),
        // __security_check_cookie
        (0x481, Callee::Any),
    ],
    imports: &[],
};

/// The spark step of `magic_0570` (0x10009830), `magic_0597` (0x10009060)
/// and `magic_0070` (0x10009b40) (744 bytes each; family P013, the wait
/// `N` the immediate at `+0x43`: 0x60, 0x6c, 0x72). A callback, cdecl
/// `(obj)`; an 8-aligned /GS frame `A = ((E - 4) & !7) - 0x30`, the
/// random range `Q` at `A + 0xc`, the start `P` at `A + 0x1c`. Each call
/// takes the next spark `R = SPARKS + 0x30 * *G_I` (`*G_I` one up after)
/// and its age u16 `R + 0x20` one up; by the state byte `R + 0x22`:
/// - 0: `obj + 0x148` = `ITEM`, state 1;
/// - 1: the speed `s` = `Q.y + rand * Q.x` (`Q` = slot 325 `(Q, P)` with
///   `P` = (12, 1.8, 10, 3), its `z` times a random fraction and that
///   added to `w`; the fractions from slot 324 as `1.0 + mantissa - 1`):
///   `R` = `obj.dir / |obj.dir| * s` with `y` less `Q.w`, the colour
///   steps `+0x24..+0x2c` from a third slot 324 word (three 10-bit
///   fields shifted by its top two bits, times 4), `R + 0x10..` (0, 0,
///   -0.9), age 0, state 2;
/// - 2: at age `N` state 3, and on;
/// - 3: `R` += `R + 0x10` (gravity), `obj + 0xa0..` += `R`, the colour
///   `obj + 0xd0..` += the steps.
///
/// Returns 0 (states 0, 1), 2 while waiting, the last colour step (3), or
/// the state.
#[allow(clippy::too_many_lines)] // the original's states, in order
pub fn spark_step_0570<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let obj = fr.arg(g, 0);
    let a = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x30);
    let (q, p) = (a + 0xc, a + 0x1c);
    let gi = w(0x16);
    let idx = g.read::<u32>(gi);
    let r = idx.wrapping_mul(0x30).wrapping_add(w(0x27));
    let mut state = u32::from(g.read::<u8>(r + 0x22));
    let eax = loop {
        match state {
            0 => {
                g.write::<u32>(obj + 0x148, w(0x5b));
                g.write::<u8>(r + 0x22, 1);
                break 0;
            }
            1 => {
                let (x, y, z) = (
                    g.read::<f32>(obj + 0xb0),
                    g.read::<f32>(obj + 0xb4),
                    g.read::<f32>(obj + 0xb8),
                );
                let d2 = add(add(add(mul(x, x), 0.0), mul(y, y)), mul(z, z));
                for (i, v) in (0u32..).zip([0x4140_0000u32, 0x3fe6_6667, 0x4120_0000, 0x4040_0000])
                {
                    g.write::<u32>(p + 4 * i, v);
                }
                let len = f64::from(d2).sqrt() as f32;
                let inv = div(k(g, 0xc7), len);
                // call width: 2
                g.call(t(g, 0xde, 325), 0, &[q, p]);
                let frac = |g: &mut G, tb: u32, one: u32| {
                    let r = g.call(t(g, tb, 324), 0, &[]).eax;
                    sub(f32::from_bits((r & 0x7f_ffff) + 0x3f80_0000), k(g, one))
                };
                let fz = frac(g, 0xf1, 0x11e);
                let z2 = mul(fz, g.read::<f32>(q + 8));
                let qw = add(g.read::<f32>(q + 0xc), z2);
                g.write::<f32>(q + 8, z2);
                g.write::<f32>(q + 0xc, qw);
                let fx = frac(g, 0x123, 0x169);
                let s = add(g.read::<f32>(q + 4), mul(fx, g.read::<f32>(q)));
                let rx = mul(mul(inv, g.read::<f32>(obj + 0xb0)), s);
                g.write::<f32>(q, s);
                g.write::<f32>(r, rx);
                let ry = mul(mul(g.read::<f32>(obj + 0xb4), inv), s);
                g.write::<f32>(r + 4, ry);
                let rz = mul(mul(g.read::<f32>(obj + 0xb8), inv), s);
                g.write::<f32>(r + 8, rz);
                let ry = sub(g.read::<f32>(r + 4), g.read::<f32>(q + 0xc));
                g.write::<f32>(r + 4, ry);
                let bits = g.call(t(g, 0x16e, 324), 0, &[]).eax;
                let c = (bits >> 30) & 3;
                let field =
                    |sh: u32| ((((bits.cast_signed() >> sh).cast_unsigned()) & 0x3ff) << c) << 2;
                g.write::<u32>(r + 0x24, field(0));
                g.write::<u32>(r + 0x28, field(10));
                g.write::<u32>(r + 0x2c, field(20));
                g.write::<u32>(r + 0x10, 0);
                g.write::<u32>(r + 0x14, 0);
                g.write::<u32>(r + 0x18, 0xbf66_6666);
                g.write::<u16>(r + 0x20, 0);
                g.write::<u8>(r + 0x22, 2);
                break 0;
            }
            2 => {
                let n = img.word(f + 0x40) >> 24;
                if g.read::<u16>(r + 0x20) < n as u16 {
                    break 2;
                }
                g.write::<u8>(r + 0x22, 3);
                state = 3;
            }
            3 => {
                let ry = add(g.read::<f32>(r + 0x14), g.read::<f32>(r + 4));
                let rx = add(g.read::<f32>(r + 0x10), g.read::<f32>(r));
                g.write::<f32>(r + 4, ry);
                let rz = add(g.read::<f32>(r + 0x18), g.read::<f32>(r + 8));
                g.write::<f32>(r, rx);
                g.write::<f32>(r + 8, rz);
                let ox = add(rx, g.read::<f32>(obj + 0xa0));
                g.write::<f32>(obj + 0xa0, ox);
                let oy = add(g.read::<f32>(obj + 0xa4), g.read::<f32>(r + 4));
                g.write::<f32>(obj + 0xa4, oy);
                let oz = add(g.read::<f32>(r + 8), g.read::<f32>(obj + 0xa8));
                g.write::<f32>(obj + 0xa8, oz);
                let mut last = 0;
                for (o, d) in [(0xd0u32, 0x24u32), (0xd4, 0x28), (0xd8, 0x2c)] {
                    last = g.read::<u32>(r + d);
                    let v = g.read::<u32>(obj + o).wrapping_add(last);
                    g.write::<u32>(obj + o, v);
                }
                break last;
            }
            s => break s,
        }
    };
    let age = g.read::<u16>(r + 0x20).wrapping_add(1);
    g.write::<u16>(r + 0x20, age);
    let i = g.read::<u32>(gi).wrapping_add(1);
    // state 1 rereads `*G_I` after its calls; the others keep `idx`
    let i = if state == 1 { i } else { idx.wrapping_add(1) };
    g.write::<u32>(gi, i);
    Ret::eax(eax)
}

/// [`spark_step_0570`] in `magic_0570`.
pub const SPARK_STEP_0570: Sig = Sig {
    len: 744,
    hash: 0x1068_6853_cd9f_2f04,
    calls: &[
        (0xbf, Callee::Import("_libm_sse2_sqrt_precise")),
        // __security_check_cookie
        (0x2d0, Callee::Any),
    ],
    imports: &[],
};

/// [`spark_step_0570`] in `magic_0597`.
pub const SPARK_STEP_0597: Sig = Sig {
    len: 744,
    hash: 0x7e1f_30be_08f8_58c0,
    calls: &[
        (0xbf, Callee::Import("_libm_sse2_sqrt_precise")),
        // __security_check_cookie
        (0x2d0, Callee::Any),
    ],
    imports: &[],
};

/// [`spark_step_0570`] in `magic_0070`.
pub const SPARK_STEP_0070: Sig = Sig {
    len: 744,
    hash: 0xe816_6d54_3a80_4cd2,
    calls: &[
        (0xbf, Callee::Import("_libm_sse2_sqrt_precise")),
        // __security_check_cookie
        (0x2d0, Callee::Any),
    ],
    imports: &[],
};

/// The operands of one of [`summon_models_0589`]'s three actor blocks.
struct ModelBlock0589 {
    /// The selection byte (3, 4, 5) and the `(*G_SEL, ACTORS)` operand
    /// pairs of the four actor reads (the matrix, slot 277, slots
    /// 15-17, slot 31).
    byte: u32,
    actors: [(u32, u32); 4],
    /// The matrix (`lea ebx, [edi + d]`).
    m: u32,
    /// Slots 210, the battle word, 217, 255, 211, 235, 277's constant,
    /// 15, 16, 17, 330, 328, 329, 31 (table operands; the constant's own).
    slots: [u32; 14],
    /// The four copied words, then `+0x10`, `+0x14` (its `y` less `K`,
    /// the constant's operand), `+0x18`, `+0x1c` of the place (source and
    /// destination displacements).
    copies: [(u32, u32); 4],
    tail: [(u32, u32); 4],
    k: u32,
    /// The angles' displacements for slots 330, 328, 329.
    angles: [u32; 3],
    /// Which product order slots 15, 16 and 17 use (`f * m` when true).
    f_first: [bool; 3],
}

const BLOCKS_0589: [ModelBlock0589; 3] = [
    ModelBlock0589 {
        byte: 3,
        actors: [(0xce, 0xe6), (0x1ca, 0x1d5), (0x205, 0x210), (0x350, 0x35b)],
        m: 0xdf,
        slots: [
            0xf7, 0x10a, 0x11b, 0x12d, 0x143, 0x1b4, 0x1f2, 0x1da, 0x215, 0x22c, 0x248, 0x258,
            0x291, 0x2b0,
        ],
        copies: [
            (0x151, 0x157),
            (0x15d, 0x163),
            (0x169, 0x16f),
            (0x175, 0x17b),
        ],
        tail: [
            (0x181, 0x197),
            (0x189, 0x1c3),
            (0x19d, 0x1a3),
            (0x1a9, 0x1af),
        ],
        k: 0x191,
        angles: [0x275, 0x28c, 0x2ab],
        f_first: [true, false, true],
    },
    ModelBlock0589 {
        byte: 4,
        actors: [
            (0x36a, 0x37e),
            (0x45f, 0x46a),
            (0x49a, 0x4a5),
            (0x5e5, 0x5f0),
        ],
        m: 0x370,
        slots: [
            0x38c, 0x39f, 0x3b0, 0x3c2, 0x3d8, 0x449, 0x487, 0x46f, 0x4aa, 0x4c1, 0x4dd, 0x4fa,
            0x526, 0x545,
        ],
        copies: [
            (0x3e6, 0x3ec),
            (0x3f2, 0x3f8),
            (0x3fe, 0x404),
            (0x40a, 0x410),
        ],
        tail: [
            (0x416, 0x42c),
            (0x41e, 0x458),
            (0x432, 0x438),
            (0x43e, 0x444),
        ],
        k: 0x426,
        angles: [0x50a, 0x521, 0x540],
        f_first: [true, false, false],
    },
    ModelBlock0589 {
        byte: 5,
        actors: [
            (0x5ff, 0x613),
            (0x6f4, 0x6ff),
            (0x72f, 0x73a),
            (0x87a, 0x885),
        ],
        m: 0x605,
        slots: [
            0x621, 0x634, 0x645, 0x657, 0x66d, 0x6de, 0x71c, 0x704, 0x73f, 0x756, 0x765, 0x78f,
            0x7bb, 0x7da,
        ],
        copies: [
            (0x67b, 0x681),
            (0x687, 0x68d),
            (0x693, 0x699),
            (0x69f, 0x6a5),
        ],
        tail: [
            (0x6ab, 0x6c1),
            (0x6b3, 0x6ed),
            (0x6c7, 0x6cd),
            (0x6d3, 0x6d9),
        ],
        k: 0x6bb,
        angles: [0x79f, 0x7b6, 0x7d5],
        f_first: [false, true, true],
    },
];

/// The summons' model set-up of `magic_0589` (0x10002000) and
/// `magic_0511` (0x10002240) (2215 bytes; family P003, the record
/// offsets read from the code; a table callback, no arguments; a cookie
/// frame, `ebp = E - 4`, the caster's place at `ebp - 0x14`). `S = *G_S`:
/// - for the models 0x301a, 0x301b, 0x301c: slot 0 `(id)`, slot 26 `(id,
///   1)`, the byte `0x20 + i` and the handle into the record's tables,
///   slot 10 `(handle, 0)`; `*G_N` up by 3;
/// - for the third, fourth and fifth selected actors
///   ([`ModelBlock0589`]): the model matrix as [`point_object_0586`]'s
///   (slots 210 or 217/255, 211, 235, 277, 15, 16, 17, 330, 328, 329)
///   with the old place copied on (its `y` less `K`), every term then
///   times 0, and slot 31 `(actor, 0.0016)`.
///
/// Returns the last slot 31's `eax`.
#[allow(clippy::too_many_lines)] // one straight sequence
pub fn summon_models_0589<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::ob_model::st0_f32;
    use crate::sb_sse::{mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let rd = |g: &G, o: u32| g.read::<u32>(w(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let ebp = fr.esp.wrapping_sub(4);
    let s = rd(g, 0x1c);
    // (the model id, slot 0's, 26's, 10's table operands, the byte's
    // operand, the handle's)
    for (id, ops) in [
        (0x301au32, [0x11u32, 0x29, 0x3c, 0x42, 0x49]),
        (0x301b, [0x56, 0x65, 0x78, 0x7e, 0x85]),
        (0x301c, [0x92, 0xa1, 0xb4, 0xba, 0xc1]),
    ] {
        let h = g.call(t(g, ops[0], 0), 0, &[id]).eax;
        // call width: 2
        g.call(t(g, ops[1], 26), 0, &[id, 1]);
        let b = (img.word(f + ops[3] + 4) & 0xff) as u8;
        g.write::<u8>(w(ops[3]), b);
        g.write::<u32>(w(ops[4]), h);
        // call width: 2
        g.call(t(g, ops[2], 10), 0, &[h, 0]);
    }
    let n = rd(g, 0xd4).wrapping_add(3);
    g.write::<u32>(w(0xd4), n);
    let at = |o: u32| s.wrapping_add(w(o));
    let mut eax = 0;
    for bl in &BLOCKS_0589 {
        let actor = |g: &G, (sel, tbl): (u32, u32)| {
            let i = u32::from(g.read::<u8>(rd(g, sel) + bl.byte));
            (i, g.read::<u32>(w(tbl).wrapping_add(4 * i)))
        };
        let sl = bl.slots;
        let (i, a) = actor(g, bl.actors[0]);
        let m = at(bl.m);
        if m == 0 {
            // `S + d` null: no caller's `S` gets here
            let v = ebp.wrapping_sub(0x14);
            if rd(g, sl[1]) == 0 && i < 0x20 {
                g.call(t(g, sl[2], 217), 0, &[i, v]);
                g.call(t(g, sl[3], 255), 0, &[i, v]);
            }
        } else {
            g.call(t(g, sl[0], 210), 0, &[a, m]);
        }
        g.call(t(g, sl[4], 211), 0, &[a, m]);
        for (src, dst) in bl.copies {
            let x = g.read::<u32>(at(src));
            g.write::<u32>(at(dst), x);
        }
        let x10 = g.read::<u32>(at(bl.tail[0].0));
        let y = sub(g.read::<f32>(at(bl.tail[1].0)), k(g, bl.k));
        g.write::<u32>(at(bl.tail[0].1), x10);
        for (src, dst) in [bl.tail[2], bl.tail[3]] {
            let x = g.read::<u32>(at(src));
            g.write::<u32>(at(dst), x);
        }
        g.write::<f32>(at(bl.tail[1].1), y);
        g.call(t(g, sl[5], 235), 0, &[m]);
        let (_, a2) = actor(g, bl.actors[1]);
        let x = st0_f32(g.call(t(g, sl[7], 277), 0, &[a2]).st0);
        let x = mul(x, k(g, sl[6]));
        for o in [0x28u32, 0x14, 0] {
            g.write::<f32>(m + o, x);
        }
        let (_, a3) = actor(g, bl.actors[2]);
        for ((tb, n), (o, ff)) in [(sl[8], 15u32), (sl[9], 16), (sl[10], 17)]
            .into_iter()
            .zip([0u32, 0x14, 0x28].into_iter().zip(bl.f_first))
        {
            let x = st0_f32(g.call(t(g, tb, n), 0, &[a3]).st0);
            let v = g.read::<f32>(m + o);
            g.write::<f32>(m + o, if ff { mul(x, v) } else { mul(v, x) });
        }
        for ((tb, n), ang) in [(sl[11], 330u32), (sl[12], 328), (sl[13], 329)]
            .into_iter()
            .zip(bl.angles)
        {
            let ang = g.read::<u32>(s.wrapping_add(w(ang)));
            // call width: 3
            g.call(t(g, tb, n), 0, &[m, m, ang]);
        }
        for o in [0u32, 4, 8, 0x10, 0x14, 0x18, 0x20, 0x24, 0x28] {
            let x = mul(g.read::<f32>(m + o), 0.0);
            g.write::<f32>(m + o, x);
        }
        let (_, a4) = actor(g, bl.actors[3]);
        let tb = bl.actors[3].0 + 0x10;
        // call width: 2
        eax = g.call(t(g, tb, 31), 0, &[a4, 0x38d1_b717]).eax;
    }
    Ret::eax(eax)
}

/// [`summon_models_0589`] in `magic_0589`.
pub const SUMMON_MODELS_0589: Sig = Sig {
    len: 2215,
    hash: 0x9d54_0d76_d1ec_be2c,
    calls: &[
        // __security_check_cookie
        (0x89f, Callee::Any),
    ],
    imports: &[],
};

/// [`summon_models_0589`] in `magic_0511`.
pub const SUMMON_MODELS_0511: Sig = Sig {
    len: 2215,
    hash: 0x78d4_0046_14a5_ce80,
    calls: &[
        // __security_check_cookie
        (0x89f, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0578`'s fade task (0x10007320, 1035 bytes; a task function,
/// cdecl `(task)`, the record `R = task[1]`; two jump tables). The entry
/// `E = ENTRIES + 0x50 * R[0x28]` (its float `E + 0x24`, colour bytes
/// `E..E + 8`). By the state `R + 0x18` (a state set and jumped to runs at
/// once), the count `R + 0x20` (one down on every way out but 40's):
/// - 0: state 1, count 5, and on into 1;
/// - 1: at the count's end `E + 0x24` up by `K`; once it is at least
///   `K2`, state 10, and on;
/// - 10: state 11, count 30, `R + 0x24` 1, and on into 11;
/// - 11: at the count's end: while `R + 0x24` counts down, count 15;
///   then state 40, and on;
/// - 40: state 41, count 14 (no step down);
/// - 41: at the count's end state 42, `R + 0x1c` 1.0, and on;
/// - 42: `R + 0x1c` less `K`; while above 0 (or unordered) the entry's
///   eight colour bytes scaled by it (truncated); else the nodes `R +
///   0x750` and `R + 0x7b0` leave their parents and their parts are given
///   back (slot 779 `(slot 771 (node))`, the lock `*G_L` held at 1),
///   state 20, and on;
/// - 20: the task's node unlinked, state 20000.
///
/// Returns what `eax` holds on the way out: the table's index (1, 3, 5, 6,
/// 7, 8 for the others below 0x2b), the state plus one (0, 10), the entry's
/// offset (1 below `K2`), the last colour byte's product, the node's old
/// `+4` (20), the state past the table.
#[allow(clippy::too_many_lines)] // the original's states, in order
pub fn fade_task_0578<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, cvt, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    let count = |g: &G| g.read::<i32>(r + 0x20);
    let end = |g: &mut G, eax: u32| {
        let c = g.read::<u32>(r + 0x20).wrapping_sub(1);
        g.write::<u32>(r + 0x20, c);
        Ret::eax(eax)
    };
    let entry = |g: &G| g.read::<u32>(r + 0x28).wrapping_mul(0x50);
    let step = k(g, 0x22);
    let mut state = g.read::<u32>(r + 0x18);
    loop {
        match state {
            0 | 1 => {
                let mut eax = 1;
                if state == 0 {
                    g.write::<u32>(r + 0x20, 5);
                    g.write::<u32>(r + 0x18, 1);
                }
                if count(g) > 0 {
                    return end(g, eax);
                }
                let e = w(0x62).wrapping_add(entry(g));
                let x = add(g.read::<f32>(e), step);
                g.write::<f32>(e, x);
                eax = entry(g);
                let e = w(0x7f).wrapping_add(eax);
                // `comiss; jb`: below or unordered waits
                if g.read::<f32>(e)
                    .partial_cmp(&k(g, 0x86))
                    .is_none_or(std::cmp::Ordering::is_lt)
                {
                    return end(g, eax);
                }
                g.write::<u32>(r + 0x18, 10);
            }
            10 | 11 => {
                let eax = if state == 10 {
                    g.write::<u32>(r + 0x20, 0x1e);
                    g.write::<u32>(r + 0x24, 1);
                    g.write::<u32>(r + 0x18, 11);
                    11
                } else {
                    3
                };
                if count(g) > 0 {
                    return end(g, eax);
                }
                if g.read::<u32>(r + 0x24) != 0 {
                    let n = g.read::<u32>(r + 0x24).wrapping_sub(1);
                    g.write::<u32>(r + 0x24, n);
                    g.write::<u32>(r + 0x20, 0xe);
                    return Ret::eax(eax);
                }
                g.write::<u32>(r + 0x18, 0x28);
            }
            0x14 => {
                let node = g.read::<u32>(task);
                let (prev, next) = (g.read::<u32>(node + 4), g.read::<u32>(node + 8));
                g.write::<u32>(prev + 8, next);
                let (next, prev) = (g.read::<u32>(node + 8), g.read::<u32>(node + 4));
                g.write::<u32>(next + 4, prev);
                g.write::<u32>(node + 8, node);
                g.write::<u32>(node + 4, node);
                g.write::<u32>(r + 0x18, 0x4e20);
                return end(g, prev);
            }
            0x28 => {
                let st = g.read::<u32>(r + 0x18).wrapping_add(1);
                g.write::<u32>(r + 0x18, st);
                g.write::<u32>(r + 0x20, 0xd);
                return Ret::eax(5);
            }
            0x29 => {
                if count(g) > 0 {
                    return end(g, 6);
                }
                g.write::<u32>(r + 0x1c, ONE);
                g.write::<u32>(r + 0x18, 0x2a);
            }
            0x2a => {
                let x = sub(g.read::<f32>(r + 0x1c), step);
                g.write::<f32>(r + 0x1c, x);
                // `comiss 0, x; jb`: above 0 or unordered
                if x.partial_cmp(&0.0).is_none_or(std::cmp::Ordering::is_gt) {
                    let mut eax = 0;
                    for (i, o) in
                        (0u32..).zip([0x24eu32, 0x273, 0x299, 0x2bf, 0x2e5, 0x30b, 0x331, 0x357])
                    {
                        let at = w(o).wrapping_add(entry(g));
                        let s = if i == 0 { x } else { g.read::<f32>(r + 0x1c) };
                        eax = crate::s4_summon::trunc(mul(cvt(u32::from(g.read::<u8>(at))), s));
                        g.write::<u8>(at, eax.to_le_bytes()[0]);
                    }
                    return end(g, eax);
                }
                // the two nodes leave their parents; their parts go back
                for (n, s771, lock, s779, lock2) in [
                    (0x750u32, 0x14cu32, 0x15bu32, 0x167u32, 0x17au32),
                    (0x7b0, 0x1cb, 0x1da, 0x1e6, 0x1f9),
                ] {
                    let node = r + n;
                    let parent = g.read::<u32>(node + 0x14);
                    if parent != 0 {
                        let c = g.read::<u16>(parent + 0x18).wrapping_add(0xffff);
                        g.write::<u16>(parent + 0x18, c);
                        let (next, prev) = (g.read::<u32>(node + 4), g.read::<u32>(node + 8));
                        if next == parent {
                            g.write::<u32>(parent + 0x10, prev);
                        } else {
                            g.write::<u32>(next + 8, prev);
                        }
                        let (prev, next) = (g.read::<u32>(node + 8), g.read::<u32>(node + 4));
                        if prev == parent {
                            g.write::<u32>(parent + 0xc, next);
                        } else {
                            g.write::<u32>(prev + 4, next);
                        }
                        g.write::<u32>(node + 8, node);
                        g.write::<u32>(node + 4, node);
                        g.write::<u32>(node + 0x14, 0);
                    }
                    let id = g.call(t(g, s771, 771), 0, &[node]);
                    let l = g.read::<u32>(w(lock));
                    let saved = g.read::<u32>(l);
                    g.write::<u32>(l, 1);
                    // call width: 2
                    g.call(t(g, s779, 779), 0, &[id.eax, id.edx]);
                    g.write::<u32>(g.read::<u32>(w(lock2)), saved);
                }
                g.write::<u32>(r + 0x18, 0x14);
            }
            s if s <= 0x2a => return end(g, 8),
            s => return end(g, s),
        }
        state = g.read::<u32>(r + 0x18);
    }
}

/// [`fade_task_0578`] (1035 bytes, 1 overlay).
pub const FADE_TASK_0578: Sig = Sig {
    len: 1035,
    hash: 0xb197_e433_274c_4a7e,
    calls: &[],
    imports: &[],
};

/// `magic_0578`'s summon task (0x10006fc0, 858 bytes; a task function,
/// cdecl `(task)`, the record `R = task[1]`, the object `O = R[0x24]`; an
/// 8-aligned frame without a cookie, `A = ((E - 4) & !7) - 0x1c`, the
/// group and the record at `A + 4`; the shape of
/// [`summon_task_0649`](crate::r25::summon_task_0649)). By the state `R +
/// 0x18` (a jump table; a state set and jumped to runs at once), the count
/// `R + 0x1c` (one down on every way out) and the kind `R + 0x20`:
/// - 0: slot 228 `(O[0x50], 0xb2)`, state 1;
/// - 1: once slot 411 `(O[0x50])` answers: slot 228 `(.., 0xb3)`; the
///   nodes `R + 0x28` and `R + 0x54` appended to `G_L`
///   ([`node_append`](crate::sb_misc::node_append)), between them `R +
///   0x44..` = slot 262 `(O[0x54])`'s point moved by `((kind K1 - K2)
///   K3, -K4, K5)` with `R + 0x50` 1.0; the colour `R + 0x70..` (1, 1, 1,
///   0.13); the node `R + 0xb0` inserted
///   ([`node_insert`](crate::nodes::node_insert)), the particle node `R +
///   0x350` ([`particle_node_with`](crate::s3_nodes::particle_node_with),
///   the kind's part `PARTS[kind].0`); state 2, count 0x28;
/// - 2: at the count's end state 10, and on;
/// - 10: the node `R + 0x84` appended with the colour (1, 1, 1, 0.6), the
///   node `R + 0x1fc` inserted, the particle node `R + 0x450`
///   ([`particle_node`](crate::sb_nodes::particle_node), the kind's
///   `PARTS[kind].1`); state 11, count 0x6e;
/// - 11: at the count's end state 20, and on;
/// - 20: slot 228 `(.., 0xb4)`, state 21, the kind 0;
/// - 21: once slot 411 answers: slot 213 `(O[0x50], 0)`, the task's node
///   unlinked, state 20000.
///
/// Returns what `eax` holds on the way out.
#[allow(clippy::too_many_lines)] // the original's states, in order
pub fn summon_task_0578<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, cvt, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    let a = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x1c);
    // after `push ebx; push esi; push edi`
    let s = a.wrapping_sub(0xc);
    let obj = |g: &G| g.read::<u32>(g.read::<u32>(r + 0x24) + 0x50);
    let down = |g: &mut G, eax: u32| {
        let n = g.read::<u32>(r + 0x1c).wrapping_sub(1);
        g.write::<u32>(r + 0x1c, n);
        Ret::eax(eax)
    };
    let next = |g: &mut G| {
        let n = g.read::<u32>(r + 0x18).wrapping_add(1);
        g.write::<u32>(r + 0x18, n);
    };
    loop {
        let state = g.read::<u32>(r + 0x18);
        let waiting = g.read::<i32>(r + 0x1c) > 0;
        match state {
            0 | 20 => {
                let (o, id) = if state == 0 {
                    (0x70, 0xb2)
                } else {
                    (0x2a3, 0xb4)
                };
                let o50 = obj(g);
                // call width: 2
                let eax = g.call(t(g, o, 228), 0, &[o50, id]).eax;
                next(g);
                let ret = down(g, eax);
                if state == 20 {
                    g.write::<u32>(r + 0x20, 0);
                }
                return ret;
            }
            1 => {
                if g.call(t(g, 0x93, 411), 0, &[obj(g)]).eax == 0 {
                    return down(g, 0);
                }
                let o50 = obj(g);
                // call width: 2
                g.call(t(g, 0xb6, 228), 0, &[o50, 0xb3]);
                let (n1, n2, list) = (r + 0x28, r + 0x54, w(0xc6));
                crate::sb_misc::node_append(g, img, call(0xcd), n1, list, 0);
                let p = r + 0x44;
                let model = g.read::<u32>(g.read::<u32>(r + 0x24) + 0x54);
                // call width: 4
                g.call(t(g, 0xe0, 262), 0, &[model, p, p + 4, p + 8]);
                let x = mul(cvt(g.read::<u32>(r + 0x20)), k(g, 0x103));
                let x = mul(sub(x, k(g, 0x113)), k(g, 0x122));
                g.write::<u32>(r + 0x50, ONE);
                let v = sub(g.read::<f32>(p), x);
                g.write::<f32>(p, v);
                let v = add(g.read::<f32>(r + 0x48), k(g, 0x137));
                g.write::<f32>(r + 0x48, v);
                let v = sub(g.read::<f32>(r + 0x4c), k(g, 0x149));
                g.write::<f32>(r + 0x4c, v);
                // the append leaves `ecx` = the node, `edx` = the list
                crate::sb_misc::node_append(g, img, call(0x153), n2, w(0x10b), 0);
                for (i, x) in (0u32..).zip([ONE, ONE, ONE, 0x3e05_1eb8]) {
                    g.write::<u32>(r + 0x70 + 4 * i, x);
                }
                let ins = r + 0xb0;
                let grp = a + 4;
                let list = w(0x10b);
                for (i, x) in (0u32..).zip([ins, list, w(0x18d), n1, w(0x199), n2]) {
                    g.write::<u32>(grp + 4 * i, x);
                }
                let nf = Frame {
                    at: call(0x19e),
                    esp: s.wrapping_sub(4),
                    ecx: grp,
                    edx: list,
                    ..*fr
                };
                crate::nodes::node_insert(g, img, &nf);
                let kind = g.read::<u32>(r + 0x20);
                let part = g.read::<u32>(w(0x1ae).wrapping_add(8 * kind));
                // push part; sub esp, 8; push the node: the two middle words
                // are never written (the callee does not read them)
                let esp = s.wrapping_sub(0x14);
                g.write::<u32>(esp + 4, ins);
                g.write::<u32>(esp + 0x10, part);
                let pf = Frame {
                    at: call(0x1b7),
                    esp,
                    ecx: r + 0x350,
                    edx: list,
                    ..*fr
                };
                let eax = crate::s3_nodes::particle_node_with(g, img, &pf);
                next(g);
                g.write::<u32>(r + 0x1c, 0x27);
                return Ret::eax(eax);
            }
            2 | 11 => {
                if waiting {
                    return down(g, if state == 2 { 2 } else { 4 });
                }
                g.write::<u32>(r + 0x18, if state == 2 { 10 } else { 0x14 });
            }
            10 => {
                let (n, list) = (r + 0x84, w(0x1d9));
                crate::sb_misc::node_append(g, img, call(0x1de), n, list, 0);
                for (i, x) in (0u32..).zip([ONE, ONE, ONE, 0x3f19_999a]) {
                    g.write::<u32>(r + 0xa0 + 4 * i, x);
                }
                let ins = r + 0x1fc;
                let grp = a + 4;
                let b = g.read::<u32>(r + 0x80);
                for (i, x) in (0u32..).zip([ins, list, w(0x22a), b, w(0x236), n]) {
                    g.write::<u32>(grp + 4 * i, x);
                }
                let nf = Frame {
                    at: call(0x23b),
                    esp: s.wrapping_sub(4),
                    ecx: grp,
                    edx: list,
                    ..*fr
                };
                crate::nodes::node_insert(g, img, &nf);
                let owner = g.read::<u32>(w(0x240));
                let kind = g.read::<u32>(r + 0x20);
                let part = g.read::<u32>(w(0x254).wrapping_add(8 * kind));
                let pn = r + 0x450;
                let rec = a + 4;
                g.write::<u32>(rec, pn);
                g.write::<u32>(pn, w(0x25e));
                for (i, x) in (1u32..).zip([w(0x26a), ins, owner, w(0x276), part]) {
                    g.write::<u32>(rec + 4 * i, x);
                }
                let eax = crate::sb_nodes::particle_node(g, img, call(0x27f), rec);
                next(g);
                g.write::<u32>(r + 0x1c, 0x6d);
                return Ret::eax(eax);
            }
            21 => {
                if g.call(t(g, 0x2cd, 411), 0, &[obj(g)]).eax == 0 {
                    return down(g, 0);
                }
                let o50 = obj(g);
                // call width: 2
                g.call(t(g, 0x2e9, 213), 0, &[o50, 0]);
                let node = g.read::<u32>(task);
                let (prev, next) = (g.read::<u32>(node + 4), g.read::<u32>(node + 8));
                g.write::<u32>(prev + 8, next);
                let (next, prev) = (g.read::<u32>(node + 8), g.read::<u32>(node + 4));
                g.write::<u32>(next + 4, prev);
                g.write::<u32>(node + 8, node);
                g.write::<u32>(node + 4, node);
                g.write::<u32>(r + 0x18, 0x4e20);
                return down(g, prev);
            }
            s if s <= 0x15 => return down(g, 7),
            s => return down(g, s),
        }
    }
}

/// [`summon_task_0578`] (858 bytes, 1 overlay).
pub const SUMMON_TASK_0578: Sig = Sig {
    len: 858,
    hash: 0x36b5_a452_83f9_35ac,
    calls: &[
        (0xcd, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x153, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x19e, Callee::Sig(&crate::nodes::INSERT)),
        (0x1b7, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x1de, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x23b, Callee::Sig(&crate::nodes::INSERT)),
        (0x27f, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
    ],
    imports: &[],
};

/// `magic_0578`'s fade object set-up (0x10007730, 1783 bytes; internal,
/// `ecx` = the object `O`, `edx` = the source `S`, cdecl `(a, kind, b)`;
/// an `ebp` frame, the records at `ebp - 0x30..`; two of
/// [`fade_setup_0649`](crate::r25::fade_setup_0649)'s groups). `O` goes
/// last on the DLL's task ring (its vtable and step); the big node `O +
/// 0x164` ([`node_init_big_table`](crate::m0070::node_init_big_table));
/// `O + 0x2b0` = `S[0x54]`, `+0x2b4` 6. On `G_L`
/// ([`list_node_0645`](crate::r22::list_node_0645)), in two groups:
/// - the nodes `O + 0x34` and `O + 0x74` (1.0, the big node; the points
///   `O + 0x60..` (12, 0.5, `(kind K1 - K2) K3`) and `O + 0xa0..` (`K4 -
///   (1 - kind) K5`, `K6 - (1 - kind) K7`, `(kind K1 - K2) K8`)), `O +
///   0x10c` and `O + 0x138` (their words `+0x24`, `+0x1c` 0), the counts
///   `O + 0x12c` and `O + 0x158` = the source's alpha (slot 333, or slot
///   601's value; `K - v` times `K9` over `K10`, truncated) plus `kind
///   0x8000 - 0x4000` and `kind 0xc000 - 0x6000`, `O + 0xb4` and `O + 0xe0`
///   with the colour (1, 1, 1, 0.5);
/// - the nodes `O + 0x2b8` and `O + 0x404` inserted
///   ([`node_insert`](crate::nodes::node_insert)) with the particle nodes
///   `O + 0x550` and `O + 0x650`
///   ([`particle_node`](crate::sb_nodes::particle_node), `PARTS[kind]`),
///   the bytes `O + 0x400` and `O + 0x54c` 1;
///
/// then the node `O + 0x810` (`b`, 0; 100, 1, 5), slot 676 `(O + 0x860, 2,
/// 0x40)` and `(O + 0x2110, 2, 0x40)` around the colour node `O + 0x750`,
/// and the colour node `O + 0x7b0`
/// ([`colour_node_init`](crate::sa_misc::colour_node_init), the kind's
/// table entry), each with its flags; `O + 0x28` = the kind, `+0x30` =
/// `S`, `+0x2c` = `a`, `+0x18` 0. Returns `a`.
#[allow(clippy::too_many_lines)] // one straight sequence
pub fn fade_setup_0578<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    use crate::r22::list_node_0645 as list_node;
    use crate::sb_sse::{cvt, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let (o, src) = (fr.ecx, fr.edx);
    let (a0, kind, b) = (fr.arg(g, 0), fr.arg(g, 1), fr.arg(g, 2));
    let ebp = fr.esp.wrapping_sub(4);
    let l = |off: u32| ebp.wrapping_sub(off);
    // the task ring
    g.write::<u32>(o + 8, o);
    g.write::<u32>(o + 4, o);
    g.write::<u32>(o, w(0x12));
    g.write::<u32>(o + 0x10, w(0x19));
    g.write::<u32>(o + 0x14, o);
    let last = g.read::<u32>(w(0x21));
    g.write::<u32>(o + 8, last);
    g.write::<u32>(o + 4, w(0x2b));
    let last = g.read::<u32>(w(0x30));
    g.write::<u32>(last + 4, o);
    let fl = g.read::<u32>(w(0x3f)) & !1;
    g.write::<u32>(w(0x3f), fl);
    g.write::<u32>(l(8), src);
    g.write::<u32>(w(0x4d), o);
    let big = o + 0x164;
    let rec = l(0x28);
    for (i, x) in (0u32..).zip([big, w(0x57), w(0x5e), w(0x65), w(0x6c)]) {
        g.write::<u32>(rec + 4 * i, x);
    }
    crate::m0070::node_init_big_table(g, img, call(0x71), rec);
    let m = g.read::<u32>(src + 0x54);
    g.write::<u32>(o + 0x2b0, m);
    g.write::<u32>(o + 0x2b4, 6);
    let n1 = o + 0x34;
    list_node(
        g,
        img,
        f,
        n1,
        [0x9d, 0xa4, 0xa9, 0xb1, 0xb6, 0xbe, 0xcf, 0xd8],
    );
    let kf = cvt(kind);
    let kx = sub(mul(kf, k(g, 0xe5)), k(g, 0xf7));
    g.write::<u32>(n1 + 0x38, ONE);
    g.write::<u32>(n1 + 0x3c, big);
    g.write::<u32>(n1, w(0xfd));
    g.write::<u32>(o + 0x60, 0x4140_0000);
    g.write::<u32>(o + 0x64, 0x3f00_0000);
    g.write::<f32>(o + 0x68, mul(kx, k(g, 0x11e)));
    // the source's alpha as a count: (slot 333 / 601 operands, the
    // constants, the value's local)
    let alpha =
        |g: &mut G, (s333, k333, s601, k601, kk, kd): (u32, u32, u32, u32, u32, u32), v: u32| {
            let x = if g.read::<u8>(src + 0x90) & 1 != 0 {
                let st = g.call(t(g, s333, 333), 0, &[g.read::<u32>(src + 0x54)]).st0;
                let y = crate::ob_model::st0_f32(st);
                g.write::<f32>(v, y);
                sub(k(g, k333), y)
            } else {
                // call width: 2
                g.call(t(g, s601, 601), 0, &[g.read::<u32>(src + 0x50), v]);
                sub(k(g, k601), g.read::<f32>(v))
            };
            crate::s4_summon::trunc(div(mul(x, k(g, kk)), k(g, kd)))
        };
    let n2 = o + 0x10c;
    g.write::<u32>(n2, w(0x129));
    list_node(
        g,
        img,
        f,
        n2,
        [0x13c, 0x143, 0x148, 0x150, 0x155, 0x15d, 0x169, 0x16f],
    );
    g.write::<u32>(o + 0x130, 0);
    g.write::<u32>(o + 0x128, 0);
    let cnt = alpha(g, (0x191, 0x1a7, 0x1ba, 0x1cd, 0x1dd, 0x1eb), l(0xc));
    g.write::<u32>(o + 0x12c, cnt);
    let add = (kind << 15).wrapping_sub(0x4000);
    g.write::<u32>(o + 0x12c, cnt.wrapping_add(add));
    let n3 = o + 0xb4;
    list_node(
        g,
        img,
        f,
        n3,
        [0x219, 0x220, 0x225, 0x22d, 0x232, 0x23a, 0x246, 0x24c],
    );
    g.write::<u32>(n3, w(0x252));
    let grp = l(0x2c);
    // the node inserted after the group's colour: (insert node, its
    // operands, the group's nodes, the call; particle node, its vtable's,
    // ...)
    let insert = |g: &mut G, ins: u32, ops: (u32, u32), nodes: [u32; 3], c: u32| {
        for (i, x) in (0u32..).zip([ins, w(ops.0), w(ops.1), nodes[0], nodes[1], nodes[2]]) {
            g.write::<u32>(grp + 4 * i, x);
        }
        let nf = Frame {
            at: call(c),
            esp: ebp.wrapping_sub(0x40),
            ecx: grp,
            edx: src,
            ..*fr
        };
        crate::nodes::node_insert(g, img, &nf);
    };
    let particles = |g: &mut G, ins: u32, flag: u32, pn: u32, ops: [u32; 6]| {
        let owner = g.read::<u32>(w(ops[0]));
        g.write::<u8>(o + flag, 1);
        let part = g.read::<u32>(w(ops[1]).wrapping_add(4 * kind));
        g.write::<u32>(grp, pn);
        g.write::<u32>(pn, w(ops[2]));
        for (i, x) in (1u32..).zip([w(ops[3]), ins, owner, w(ops[4]), part]) {
            g.write::<u32>(grp + 4 * i, x);
        }
        crate::sb_nodes::particle_node(g, img, call(ops[5]), grp);
    };
    for (i, x) in (0u32..).zip([ONE, ONE, ONE, 0x3f00_0000]) {
        g.write::<u32>(o + 0xd0 + 4 * i, x);
    }
    let ins1 = o + 0x2b8;
    insert(g, ins1, (0x290, 0x297), [n1, n2, n3], 0x2a2);
    particles(
        g,
        ins1,
        0x400,
        o + 0x550,
        [0x2b0, 0x2ca, 0x2d3, 0x2dd, 0x2e4, 0x2ec],
    );
    let n4 = o + 0x74;
    list_node(
        g,
        img,
        f,
        n4,
        [0x302, 0x309, 0x30e, 0x316, 0x31b, 0x323, 0x33c, 0x34a],
    );
    let u = sub(k(g, 0x330), kf);
    g.write::<u32>(n4 + 0x38, ONE);
    let (u1, u2) = (mul(u, k(g, 0x35c)), mul(u, k(g, 0x364)));
    g.write::<u32>(n4, w(0x36a));
    g.write::<u32>(n4 + 0x3c, big);
    g.write::<f32>(o + 0xa0, sub(k(g, 0x344), u1));
    g.write::<f32>(o + 0xa4, sub(k(g, 0x38d), u2));
    g.write::<f32>(o + 0xa8, mul(kx, k(g, 0x3a6)));
    let n5 = o + 0x138;
    g.write::<u32>(n5, w(0x3b4));
    list_node(
        g,
        img,
        f,
        n5,
        [0x3c7, 0x3ce, 0x3d3, 0x3db, 0x3e0, 0x3e8, 0x3f4, 0x3fd],
    );
    g.write::<u32>(o + 0x15c, 0);
    g.write::<u32>(o + 0x154, 0);
    let cnt = alpha(g, (0x422, 0x435, 0x44b, 0x45b, 0x46b, 0x479), l(0x14));
    g.write::<u32>(o + 0x158, cnt);
    let add = kind.wrapping_mul(0xc000).wrapping_sub(0x6000);
    g.write::<u32>(o + 0x158, cnt.wrapping_add(add));
    let n6 = o + 0xe0;
    list_node(
        g,
        img,
        f,
        n6,
        [0x4aa, 0x4b1, 0x4b6, 0x4be, 0x4c3, 0x4cb, 0x4d7, 0x4dd],
    );
    g.write::<u32>(n6, w(0x4e3));
    for (i, x) in (0u32..).zip([ONE, ONE, ONE, 0x3f00_0000]) {
        g.write::<u32>(o + 0xfc + 4 * i, x);
    }
    let ins2 = o + 0x404;
    insert(g, ins2, (0x524, 0x52b), [n4, n5, n6], 0x536);
    particles(
        g,
        ins2,
        0x54c,
        o + 0x650,
        [0x544, 0x55e, 0x567, 0x571, 0x578, 0x580],
    );
    let n7 = o + 0x810;
    g.write::<u32>(n7, w(0x58c));
    list_node(
        g,
        img,
        f,
        n7,
        [0x59f, 0x5a6, 0x5ab, 0x5b3, 0x5b8, 0x5c0, 0x5cc, 0x5d5],
    );
    g.write::<u32>(n7 + 0x3c, b);
    g.write::<u32>(n7 + 0x40, 0);
    for (off, x) in [
        (0x83cu32, 0x42c8_0000u32),
        (0x840, ONE),
        (0x844, 0x40a0_0000),
    ] {
        g.write::<u32>(o + off, x);
    }
    let ent = w(0x636).wrapping_add(kind.wrapping_mul(0x50));
    let cur = l(0x30);
    // (the colour node, its block, the slot 676 operand, the block, the
    // second word's node, the insert node, flags, the call)
    for (cn, vt, s676, blk, n, ins, flags, c) in [
        (
            o + 0x750,
            0x645u32,
            0x5ddu32,
            o + 0x860,
            n1,
            ins1,
            (0x784u32, 0x7a4u32),
            0x660u32,
        ),
        (
            o + 0x7b0,
            0x6c0,
            0x665,
            o + 0x2110,
            n4,
            ins2,
            (0x7e4, 0x804),
            0x6c5,
        ),
    ] {
        // call width: 3
        g.call(t(g, s676, 676), 0, &[blk, 2, 0x40]);
        g.write::<u32>(cur, cn);
        g.write::<u32>(cn, w(vt));
        let op2 = if cn == o + 0x750 { 0x652 } else { 0x6b4 };
        for (i, x) in (1u32..).zip([w(op2), n7, n, blk, ent, ins]) {
            g.write::<u32>(cur + 4 * i, x);
        }
        crate::sa_misc::colour_node_init(g, img, call(c), cur);
        let b1 = g.read::<u8>(o + flags.0) | 0x18;
        g.write::<u8>(o + flags.0, b1);
        let b2 = g.read::<u8>(o + flags.1) | 1;
        g.write::<u8>(o + flags.1, b2);
    }
    g.write::<u32>(o + 0x28, kind);
    g.write::<u32>(o + 0x30, src);
    g.write::<u32>(o + 0x2c, a0);
    g.write::<u32>(o + 0x18, 0);
    a0
}

/// [`fade_setup_0578`] (1783 bytes, 1 overlay).
pub const FADE_SETUP_0578: Sig = Sig {
    len: 1783,
    hash: 0x84ce_6c9e_a0d0_b32a,
    calls: &[
        (0x71, Callee::Sig(&crate::m0070::NODE_INIT_BIG_TABLE)),
        (0x2a2, Callee::Sig(&crate::nodes::INSERT)),
        (0x2ec, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x536, Callee::Sig(&crate::nodes::INSERT)),
        (0x580, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x660, Callee::Sig(&crate::sa_misc::COLOUR_NODE_INIT)),
        (0x6c5, Callee::Sig(&crate::sa_misc::COLOUR_NODE_INIT)),
    ],
    imports: &[],
};

/// `magic_0578`'s light summon task (0x10008110, 2650 bytes; a task
/// function, cdecl `(task)`, the record `R = task[1]`; an 8-aligned /GS
/// frame `A = ((E - 4) & !7) - 0x44`: slot 288's count at `A + 4`, the
/// task at `A + 8`, the model nodes at `A + 0xc`, a cursor at `A + 0x18`,
/// the ids at `A + 0x28` and the models at `A + 0x34`; the shape of
/// [`light_task_0649`] with the model state inline). By the state `R +
/// 0x18` (a state set and jumped to runs at once), the count `R + 0x20`
/// (one down on every way out):
/// - 0: the model nodes `R + 0x24`, `R + 0xbc`, `R + 0x154` given the
///   first of the ids 0x14..0x16 whose slot 390 answer is their model
///   0x1084, 0x10ad, 0x10ae
///   ([`model_node_init`](crate::sa_summon::model_node_init) on `G_L`;
///   none: left alone); the static nodes `R + 0x244` (slot 262's point),
///   `R + 0x29c`, the alpha count `R + 0x2bc`, `R + 0x270` with the model
///   state `R + 0x28c..` (slots 15, 16, 17 with bit 0 of `R + 0xb4`, else
///   slot 290; slot 277 times `K`), the group node `R + 0x2c8`; the
///   points `R + 0x1ec` and `R + 0x218` (slot 262 of `R[0x110]` and
///   `R[0x1a8]`, less `K1..K3`); two fade tasks `R + 0x470` and `R +
///   0x9c0` on the task ring (step `0x10006fc0`, kinds 0 and 1); the
///   effect node `R + 0x9a4c` (slot 719's entry, the group node); the
///   kept ids `R + 0x9a08..` from slot 288 `(0x10)`'s list less 0x14..0x16
///   (count `R + 0x9a48`); state 1, count 15;
/// - 1: at the count's end the node `R + 0x414`
///   ([`node_init_root`](crate::m0070::node_init_root)), the colour `R +
///   0x430..` (255, 255, 255, 64), slot 234 and its words; state 10, and
///   on;
/// - 10: state 11, count 0x26;
/// - 11: at the count's end slot 228 `(R[0x74], 0xb0 or 0xb8 by slot
///   415's 0x10)`, the two fade objects `R + 0xf10` and `R + 0x5410`
///   ([`fade_setup_0578`]); state 12, count 0x28;
/// - 12: at the count's end the light object `R + 0x9910`
///   ([`light_object`](crate::s4_small::light_object)); state 13;
/// - 13: once slot 411 `(R[0x74])` answers: slot 213 `(R[0x74], 0)`, slot
///   296 `(id, 1)` for each kept id; state 10000, and on;
/// - 10000: once the four objects' end words share a bit of 0x4e20, the
///   model nodes `R + 0xbc` and `R + 0x154` leave their parents and are
///   set up ([`model_setup`](crate::sa_summon::model_setup)), the task's
///   node unlinked.
///
/// Returns what `eax` holds on the way out.
#[allow(clippy::too_many_lines)] // the original's states, in order
pub fn light_task_0578<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::ob_model::st0_f32;
    use crate::sb_sse::{div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    let a = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x44);
    // after `push ebx; push esi; push edi`
    let s = a.wrapping_sub(0xc);
    let at = |c: u32, esp: u32, ecx: u32, edx: u32| Frame {
        at: call(c),
        esp,
        ecx,
        edx,
        ..*fr
    };
    let list_node = crate::r22::list_node_0645::<G>;
    let count = |g: &G| g.read::<i32>(r + 0x20);
    let end = |g: &mut G, eax: u32| {
        let c = g.read::<u32>(r + 0x20).wrapping_sub(1);
        g.write::<u32>(r + 0x20, c);
        Ret::eax(eax)
    };
    let step = |g: &mut G, n: u32, eax: u32| {
        let st = g.read::<u32>(r + 0x18).wrapping_add(1);
        g.write::<u32>(r + 0x18, st);
        g.write::<u32>(r + 0x20, n.wrapping_sub(1));
        Ret::eax(eax)
    };
    let o74 = |g: &G| g.read::<u32>(r + 0x74);
    loop {
        let state = g.read::<u32>(r + 0x18);
        match state {
            0 => {
                g.write::<u32>(a + 8, task);
                let nodes = [r + 0x24, r + 0xbc, r + 0x154];
                let ids = [0x14u32, 0x15, 0x16];
                let models = [0x1084u32, 0x10ad, 0x10ae];
                for (i, x) in (0u32..).zip(nodes.into_iter().chain(ids).chain(models)) {
                    let at = if i < 3 {
                        a + 0xc + 4 * i
                    } else {
                        a + 0x28 + 4 * (i - 3)
                    };
                    g.write::<u32>(at, x);
                }
                let cur = a + 0x18;
                for (node, model) in nodes.into_iter().zip(models) {
                    for id in ids {
                        if g.call(t(g, 0x1d1, 390), 0, &[id]).eax == model {
                            g.write::<u32>(cur, node);
                            g.write::<u32>(node, w(0x200));
                            g.write::<u32>(cur + 4, w(0x20c));
                            g.write::<u32>(cur + 8, id);
                            g.write::<u8>(cur + 0xc, 0);
                            crate::sa_summon::model_node_init(g, img, call(0x21a), cur);
                            break;
                        }
                    }
                }
                // the first group: points, alpha, model state, group node
                let na = r + 0x244;
                list_node(
                    g,
                    img,
                    f,
                    na,
                    [0x23b, 0x242, 0x247, 0x24f, 0x254, 0x25c, 0x268, 0x26e],
                );
                g.write::<u32>(na, w(0x274));
                let pt = r + 0x260;
                // call width: 4
                g.call(
                    t(g, 0x287, 262),
                    0,
                    &[g.read::<u32>(r + 0x78), pt, pt + 4, pt + 8],
                );
                g.write::<u32>(r + 0x26c, ONE);
                let nb = r + 0x29c;
                g.write::<u32>(nb, w(0x2ac));
                list_node(
                    g,
                    img,
                    f,
                    nb,
                    [0x2bf, 0x2c6, 0x2cb, 0x2d3, 0x2d8, 0x2e0, 0x2ec, 0x2f2],
                );
                g.write::<u32>(r + 0x2c0, 0);
                g.write::<u32>(r + 0x2b8, 0);
                let x = if g.read::<u8>(r + 0xb4) & 1 != 0 {
                    let o = g.call(t(g, 0x314, 333), 0, &[g.read::<u32>(r + 0x78)]);
                    sub(k(g, 0x32b), st0_f32(o.st0))
                } else {
                    let v = a + 4;
                    // call width: 2
                    g.call(t(g, 0x340, 601), 0, &[o74(g), v]);
                    sub(k(g, 0x353), g.read::<f32>(v))
                };
                let x = div(mul(x, k(g, 0x364)), k(g, 0x372));
                g.write::<u32>(r + 0x2bc, crate::s4_summon::trunc(x));
                let nc = r + 0x270;
                list_node(
                    g,
                    img,
                    f,
                    nc,
                    [0x38f, 0x396, 0x39b, 0x3a3, 0x3a8, 0x3b0, 0x3bc, 0x3c2],
                );
                g.write::<u32>(nc, w(0x3c8));
                if g.read::<u8>(r + 0xb4) & 1 != 0 {
                    for (o, n, d) in [
                        (0x3dcu32, 15u32, 0x28cu32),
                        (0x3ef, 16, 0x290),
                        (0x405, 17, 0x294),
                    ] {
                        let x = st0_f32(g.call(t(g, o, n), 0, &[g.read::<u32>(r + 0x78)]).st0);
                        g.write::<f32>(r + d, x);
                    }
                } else {
                    // call width: 2
                    g.call(t(g, 0x421, 290), 0, &[o74(g), r + 0x28c]);
                }
                let x = st0_f32(g.call(t(g, 0x434, 277), 0, &[g.read::<u32>(r + 0x78)]).st0);
                g.write::<f32>(r + 0x298, mul(x, k(g, 0x451)));
                let nd = r + 0x2c8;
                g.write::<u32>(nd, w(0x468));
                list_node(
                    g,
                    img,
                    f,
                    nd,
                    [0x47b, 0x482, 0x487, 0x48f, 0x494, 0x49c, 0x4a8, 0x4ae],
                );
                g.write::<u32>(nd + 0x140, nc);
                g.write::<u32>(nd + 0x128, w(0x4be));
                g.write::<u8>(nd + 0x148, 0);
                g.write::<u32>(nd + 0x13c, na);
                g.write::<u32>(nd + 0x144, nb);
                // the two points: (node, its operands, vtable, slot 262's
                // table operand and source, the point, the constants)
                for (n, ops, vt, s262, src, p, ks) in [
                    (
                        0x1ecu32,
                        [0x4f6u32, 0x4fd, 0x502, 0x50a, 0x50f, 0x517, 0x523, 0x52f],
                        0x542u32,
                        0x53cu32,
                        0x110u32,
                        0x208u32,
                        [0x55du32, 0x584, 0x59c],
                    ),
                    (
                        0x218,
                        [0x5b7, 0x5be, 0x5c3, 0x5cb, 0x5d0, 0x5d8, 0x5e4, 0x5f0],
                        0x603,
                        0x5fd,
                        0x1a8,
                        0x234,
                        [0x61e, 0x645, 0x663],
                    ),
                ] {
                    let node = r + n;
                    list_node(g, img, f, node, ops);
                    g.write::<u32>(node, w(vt));
                    let p = r + p;
                    // call width: 4
                    g.call(
                        t(g, s262, 262),
                        0,
                        &[g.read::<u32>(r + src), p, p + 4, p + 8],
                    );
                    let x = sub(g.read::<f32>(p), k(g, ks[0]));
                    g.write::<u32>(p + 0xc, ONE);
                    g.write::<f32>(p, x);
                    let y = sub(g.read::<f32>(p + 4), k(g, ks[1]));
                    g.write::<f32>(p + 4, y);
                    let z = sub(g.read::<f32>(p + 8), k(g, ks[2]));
                    g.write::<f32>(p + 8, z);
                }
                // the two fade tasks on the task ring
                for (tk, ops, n80, n24, kind) in [
                    (
                        0x470u32,
                        [0x671u32, 0x67e, 0x686, 0x690, 0x695, 0x69e, 0x6a5],
                        0x1ecu32,
                        0xbcu32,
                        0u32,
                    ),
                    (
                        0x9c0,
                        [0x6d2, 0x6dc, 0x6f0, 0x6fa, 0x6ff, 0x708, 0x70f],
                        0x218,
                        0x154,
                        1,
                    ),
                ] {
                    let tn = r + tk;
                    if kind == 1 {
                        g.write::<u32>(tn, w(ops[0]));
                        g.write::<u32>(tn + 0x10, w(ops[1]));
                    } else {
                        g.write::<u32>(tn, w(ops[0]));
                    }
                    g.write::<u32>(tn + 8, tn);
                    g.write::<u32>(tn + 4, tn);
                    if kind == 0 {
                        g.write::<u32>(tn + 0x10, w(ops[1]));
                    }
                    g.write::<u32>(tn + 0x14, tn);
                    let last = g.read::<u32>(w(ops[2]));
                    g.write::<u32>(tn + 8, last);
                    g.write::<u32>(tn + 4, w(ops[3]));
                    let last = g.read::<u32>(w(ops[4]));
                    g.write::<u32>(last + 4, tn);
                    let fl = g.read::<u32>(w(ops[5])) & !1;
                    g.write::<u32>(w(ops[5]), fl);
                    g.write::<u32>(w(ops[6]), tn);
                    g.write::<u32>(tn + 0x20, kind);
                    g.write::<u32>(tn + 0x18, 0);
                    g.write::<u32>(tn + 0x80, r + n80);
                    g.write::<u32>(tn + 0x24, r + n24);
                }
                // the effect node, inline
                let en = r + 0x9a4c;
                list_node(
                    g,
                    img,
                    f,
                    en,
                    [0x745, 0x74c, 0x751, 0x759, 0x75e, 0x766, 0x777, 0x77f],
                );
                // call width: 3
                let e = g.call(t(g, 0x770, 719), 0, &[w(0x78c), 6, 0]).eax;
                g.write::<u32>(en + 0x24, e);
                g.write::<u32>(en + 0x1c, nd);
                g.write::<u32>(en + 0x20, w(0x79b));
                g.write::<u32>(en + 0x34, 0x40);
                g.write::<u32>(en + 0x28, 0x100);
                g.write::<u32>(en + 0x2c, 0);
                g.write::<u32>(en, w(0x7b6));
                let fl = g.read::<u32>(en + 0x2c) | 1;
                g.write::<u32>(en + 0x2c, fl);
                g.write::<u32>(en + 0x30, 0);
                g.write::<u16>(r + 0x9a48, 0);
                // the kept ids
                // call width: 2
                let mut p = g.call(t(g, 0x7d7, 288), 0, &[0x10, a + 4]).eax;
                let mut i = 0i32;
                while i < g.read::<i32>(a + 4) {
                    let c = g.read::<u8>(p);
                    if !(0x14..=0x16).contains(&c) {
                        let n = g.read::<u16>(r + 0x9a48);
                        g.write::<u16>(r + 0x9a08 + 2 * u32::from(n), u16::from(c));
                        g.write::<u16>(r + 0x9a48, n.wrapping_add(1));
                    }
                    i += 1;
                    p = p.wrapping_add(1);
                }
                let n = g.read::<u32>(a + 4);
                return step(g, 0xf, n);
            }
            1 => {
                if count(g) > 0 {
                    return end(g, 0);
                }
                let n = r + 0x414;
                crate::m0070::node_init_root(g, img, call(0x55), n);
                for (o, v) in [
                    (0x438u32, 0x437f_0000u32),
                    (0x434, 0x437f_0000),
                    (0x430, 0x437f_0000),
                    (0x43c, 0x4280_0000),
                    (0x448, 0),
                    (0x444, 0),
                    (0x440, 0),
                    (0x44c, 0),
                ] {
                    g.write::<u32>(r + o, v);
                }
                // call width: 3
                g.call(t(g, 0x66, 234), 0, &[n + 0x3c, n + 0x2c, n + 0x1c]);
                let fl = g.read::<u16>(n + 0x58) | 1;
                g.write::<u16>(n + 0x58, fl);
                g.write::<u32>(n + 0x54, 1);
                g.write::<u32>(n + 0x50, 0x3d4c_cccd);
                g.write::<u32>(n + 0x4c, 0);
                g.write::<u32>(r + 0x18, 10);
            }
            10 => return step(g, 0x26, 10),
            11 => {
                if count(g) > 0 {
                    return end(g, 0);
                }
                let id = if g.call(t(g, 0x8b9, 415), 0, &[o74(g)]).eax == 0x10 {
                    0xb0
                } else {
                    0xb8
                };
                // call width: 2
                g.call(t(g, 0x8cf, 228), 0, &[o74(g), id]);
                // push c; push kind; push a: the return address at `A - 0x1c`
                let esp = a.wrapping_sub(0x1c);
                let mut eax = 0;
                for (obj, kind, a0, c, cl) in [
                    (0xf10u32, 0u32, 0xbcu32, 0x1ecu32, 0x909u32),
                    (0x5410, 1, 0x154, 0x218, 0x92a),
                ] {
                    for (i, v) in (1u32..).zip([r + a0, kind, r + c]) {
                        g.write::<u32>(esp + 4 * i, v);
                    }
                    eax = fade_setup_0578(g, img, &at(cl, esp, r + obj, r + 0x24));
                }
                return step(g, 0x28, eax);
            }
            12 => {
                if count(g) > 0 {
                    return end(g, 0);
                }
                // sub esp, 8; push b: the return address at `A - 0x1c`
                let esp = a.wrapping_sub(0x1c);
                g.write::<u32>(esp + 4, r + 0x2c8);
                let lo = &at(0x88f, esp, r + 0x9910, r + 0x24);
                let eax = crate::s4_small::light_object(g, img, lo).eax;
                let st = g.read::<u32>(r + 0x18).wrapping_add(1);
                g.write::<u32>(r + 0x18, st);
                return end(g, eax);
            }
            13 => {
                if g.call(t(g, 0xfb, 411), 0, &[o74(g)]).eax == 0 {
                    return end(g, 0);
                }
                // call width: 2
                g.call(t(g, 0x116, 213), 0, &[o74(g), 0]);
                let mut i = 0u32;
                while i < u32::from(g.read::<u16>(r + 0x9a48)) {
                    let id = u32::from(g.read::<u16>(r + 0x9a08 + 2 * i));
                    // call width: 2
                    g.call(t(g, 0x147, 296), 0, &[id, 1]);
                    i += 1;
                }
                g.write::<u32>(r + 0x18, 0x2710);
            }
            0x2710 => {
                let eax = [0x5428u32, 0xf28, 0x9d8, 0x488]
                    .iter()
                    .fold(u32::MAX, |x, &o| x & g.read::<u32>(r + o));
                if eax & 0x4e20 == 0 {
                    return end(g, eax);
                }
                let cur = a + 4;
                for (n, ms) in [(0xbcu32, 0x9d1u32), (0x154, 0xa23)] {
                    let node = r + n;
                    let parent = g.read::<u32>(node + 0x14);
                    if parent != 0 {
                        let c = g.read::<u16>(parent + 0x18).wrapping_add(0xffff);
                        g.write::<u16>(parent + 0x18, c);
                        let (next, prev) = (g.read::<u32>(node + 4), g.read::<u32>(node + 8));
                        if next == parent {
                            g.write::<u32>(parent + 0x10, prev);
                        } else {
                            g.write::<u32>(next + 8, prev);
                        }
                        let (prev, next) = (g.read::<u32>(node + 8), g.read::<u32>(node + 4));
                        if prev == parent {
                            g.write::<u32>(parent + 0xc, next);
                        } else {
                            g.write::<u32>(prev + 4, next);
                        }
                        g.write::<u32>(node + 8, node);
                        g.write::<u32>(node + 4, node);
                        g.write::<u32>(node + 0x14, 0);
                    }
                    g.write::<u32>(cur, node);
                    crate::sa_summon::model_setup(g, img, &at(ms, s.wrapping_sub(4), cur, 0));
                }
                let node = g.read::<u32>(task);
                let (prev, next) = (g.read::<u32>(node + 4), g.read::<u32>(node + 8));
                g.write::<u32>(prev + 8, next);
                let (next, prev) = (g.read::<u32>(node + 8), g.read::<u32>(node + 4));
                g.write::<u32>(next + 4, prev);
                g.write::<u32>(node + 8, node);
                g.write::<u32>(node + 4, node);
                return end(g, prev);
            }
            // `dec eax; jne` below 10: the state less one
            s if s < 10 => return end(g, s.wrapping_sub(1)),
            s => return end(g, s),
        }
    }
}

/// [`light_task_0578`] (2650 bytes, 1 overlay).
pub const LIGHT_TASK_0578: Sig = Sig {
    len: 2650,
    hash: 0x31cc_260f_5ae8_50a9,
    calls: &[
        (0x55, Callee::Sig(&crate::m0070::NODE_INIT_ROOT)),
        (0x21a, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        // __security_check_cookie
        (0x83f, Callee::Any),
        (0x85e, Callee::Any),
        (0x88f, Callee::Sig(&crate::s4_small::LIGHT_OBJECT)),
        (0x8a6, Callee::Any),
        (0x909, Callee::Sig(&FADE_SETUP_0578)),
        (0x92a, Callee::Sig(&FADE_SETUP_0578)),
        (0x948, Callee::Any),
        (0x9d1, Callee::Sig(&crate::sa_summon::MODEL_SETUP)),
        (0xa23, Callee::Sig(&crate::sa_summon::MODEL_SETUP)),
        (0xa52, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0577`'s bone update (0x10005270, 1005 bytes; internal, `ecx` =
/// the cursor): [`bone_0573`](crate::r23::bone_0573) without the final
/// show. Returns the new parent after a re-parent, else the last slot
/// call's `eax` (or the walk's).
pub fn bone_0577<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, this: u32) -> u32 {
    crate::r23::bone_with(g, img, f, esp, this, false)
}

/// [`bone_0577`] (1005 bytes, 1 overlay).
pub const BONE_0577: Sig = Sig {
    len: 1005,
    hash: 0x6087_b706_3597_6340,
    calls: &[
        // __security_check_cookie
        (0x3e5, Callee::Any),
    ],
    imports: &[],
};

/// The thunk over [`bone_0577`] (0x10005660, 15 bytes; as
/// [`thunk_bone_0573`](crate::r23::thunk_bone_0573)).
pub fn thunk_bone_0577<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let cursor = fr.arg(g, 0);
    let esp = fr.esp.wrapping_sub(0xc);
    // push ebp; push ecx: the entry's `ecx` in the thunk's frame
    g.write::<u32>(fr.esp.wrapping_sub(8), fr.ecx);
    Ret::eax(bone_0577(g, img, img.rel32_target(fr.at + 8), esp, cursor))
}

/// [`thunk_bone_0577`] (15 bytes, 1 overlay).
pub const THUNK_BONE_0577: Sig = Sig {
    len: 15,
    hash: 0xd192_63e1_ae73_f895,
    calls: &[(8, Callee::Sig(&BONE_0577))],
    imports: &[],
};

/// `magic_0609`'s start-up (0x1000bcd0, 734 bytes; cdecl `(actor)`; a
/// table entry): [`start_up_0613`](crate::r23::start_up_0613)'s first
/// part ([`start_up_head_0613`](crate::r23::start_up_head_0613)), then the
/// same two rings with the first object's stores in another order, and
/// slot 560 `(slot 557 () & !4)`. Returns 0.
pub fn start_up_0609<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r23::stores_0613;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    crate::r23::start_up_head_0613(g, img, fr);
    stores_0613(g, img, f, &[0x1fa]);
    // the first object joins the second's ring
    let last = g.read::<u32>(w(0x205));
    g.write::<u32>(w(0x20a), last);
    stores_0613(g, img, f, &[0x20e, 0x218, 0x222]);
    g.write::<u32>(last + 4, w(0x22f));
    let last = g.read::<u32>(w(0x234));
    let fl = g.read::<u32>(w(0x23a));
    g.write::<u32>(w(0x23a), fl & !1);
    g.write::<u32>(w(0x240), last);
    stores_0613(
        g,
        img,
        f,
        &[0x244, 0x24e, 0x258, 0x262, 0x26c, 0x276, 0x280],
    );
    // and the second the first's
    g.write::<u32>(last + 4, w(0x28d));
    let s557 = t(g, 0x292, 557);
    let fl = g.read::<u32>(w(0x298));
    g.write::<u32>(w(0x298), fl & !1);
    stores_0613(g, img, f, &[0x29d, 0x2a7, 0x2b1]);
    let r = g.call(s557, 0, &[]).eax & !4;
    g.call(t(g, 0x2c8, 560), 0, &[r]);
    Ret::eax(0)
}

/// [`start_up_0609`] (734 bytes, 1 overlay).
pub const START_UP_0609: Sig = Sig {
    len: 734,
    hash: 0xe45f_7f41_fd62_aa03,
    calls: &[(0xc, Callee::Sig(&crate::s4_summon::TARGETS_INIT_C))],
    imports: &[],
};

/// The operands of one of [`summon_frames_0589`]'s three model blocks.
struct FrameBlock0589 {
    /// The actor: `None` the caster `*G_A` (operands of its two reads),
    /// else the selection byte and the `(*G_SEL, ACTORS)` operand pairs of
    /// the matrix's, slot 277's and slots 15-17's reads.
    sel: Option<(u32, [(u32, u32); 3])>,
    caster: (u32, u32, u32),
    /// Slots 210, the battle word, 217, 255, 211, 235, 277, its constant,
    /// 15, 16, 17, 330, 328, 329 (table operands; the constant's own).
    slots: [u32; 14],
    /// The products' order for slots 16 and 17 (`f * m` when true).
    f_first: [bool; 2],
}

const FRAME_BLOCKS_0589: [FrameBlock0589; 3] = [
    FrameBlock0589 {
        sel: None,
        caster: (0x122, 0x1aa, 0x1db),
        slots: [
            0x137, 0x14a, 0x161, 0x173, 0x189, 0x196, 0x1a4, 0x1c2, 0x1c7, 0x1f2, 0x20e, 0x22b,
            0x249, 0x267,
        ],
        f_first: [false, false],
    },
    FrameBlock0589 {
        sel: Some((1, [(0x310, 0x321), (0x396, 0x3a1), (0x3d1, 0x3dc)])),
        caster: (0, 0, 0),
        slots: [
            0x32f, 0x342, 0x353, 0x365, 0x37b, 0x388, 0x3a6, 0x3be, 0x3e1, 0x3f8, 0x407, 0x431,
            0x44f, 0x46d,
        ],
        f_first: [true, true],
    },
    FrameBlock0589 {
        sel: Some((2, [(0x516, 0x527), (0x59c, 0x5a7), (0x5d7, 0x5e2)])),
        caster: (0, 0, 0),
        slots: [
            0x535, 0x548, 0x559, 0x56b, 0x581, 0x58e, 0x5ac, 0x5c4, 0x5e7, 0x5fe, 0x60d, 0x637,
            0x655, 0x673,
        ],
        f_first: [true, false],
    },
];

/// `magic_0589`'s summon frames (0x100015e0, 2070 bytes; internal, no
/// arguments, called by its start-up; a cookie frame, `ebp = E - 4`: the
/// caster's place `P` at `ebp - 0x14` (with `ebp - 8` = 1.0), a vector at
/// `ebp - 0x24`). `S = *G_S`, `A = *G_A`:
/// - `P` = slot 262 `(A)`'s point, kept at `S + 0x1a00` (and 1.0); the
///   turn `a = K - f` (`f` = slot 333 `(A)`) brought into `-pi..pi`;
/// - three model blocks ([`FrameBlock0589`]): the caster's and the first
///   and second selected actors' matrices `S + 0x17c0 + 0x40 i`, each as
///   [`caster_frames_0586`]'s (slots 210 or 217/255, 211, 235, 277 times
///   `K`, 15, 16, 17, 330 and 328 with 0, 329 with `a`), each after its
///   record `S + 0x1940 + 0x20 i` = `(0, a, 0, 0, P)`, its translation then
///   the record's `P`;
/// - slot 238 `(I, 1)` for the caster's index and both selection bytes;
///   each shown (slot 296 `(index, 0)` below 32 with the battle word 0,
///   else slot 257 `(actor, 1)`); `S + 0x21f8` = `a`, `S + 0x2214` 0, four
///   flags 1.
///
/// Returns the last show's `eax`.
#[allow(clippy::too_many_lines)] // one straight sequence
pub fn summon_frames_0589<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32) -> u32 {
    use crate::ob_model::st0_f32;
    use crate::sb_sse::{add, mul, sub};
    let w = |o: u32| img.word(f + o);
    let rd = |g: &G, o: u32| g.read::<u32>(w(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let ebp = esp.wrapping_sub(4);
    let l = |off: u32| ebp.wrapping_sub(off);
    let p = l(0x14);
    let s = rd(g, 0x13);
    // call width: 4
    g.call(t(g, 0x24, 262), 0, &[rd(g, 0x2a), p, l(0x10), l(0xc)]);
    for i in 0..3 {
        let x = g.read::<u32>(p + 4 * i);
        g.write::<u32>(s + 0x1a00 + 4 * i, x);
    }
    g.write::<u32>(s + 0x1a0c, ONE);
    g.write::<u32>(l(8), ONE);
    let turn = st0_f32(g.call(t(g, 0x3c, 333), 0, &[rd(g, 0x6e)]).st0);
    let mut a = sub(k(g, 0x88), turn);
    if a > k(g, 0x97) {
        a = sub(a, k(g, 0xa6));
    } else if k(g, 0xb0) > a {
        a = add(a, k(g, 0xbd));
    }
    let record = |g: &mut G, i: u32| {
        let r = s + 0x1940 + 0x20 * i;
        for (o, v) in [(0u32, 0u32), (4, a.to_bits()), (8, 0), (0xc, 0)] {
            g.write::<u32>(r + o, v);
        }
        for j in 0..4 {
            let x = g.read::<u32>(p + 4 * j);
            g.write::<u32>(r + 0x10 + 4 * j, x);
        }
    };
    let v = l(0x24);
    record(g, 0);
    for (i, bl) in (0u32..).zip(&FRAME_BLOCKS_0589) {
        let sl = bl.slots;
        let m = s + 0x17c0 + 0x40 * i;
        let actor = |g: &G, n: usize| match bl.sel {
            None => {
                let o = [bl.caster.0, bl.caster.1, bl.caster.2][n];
                (rd(g, 0x153), rd(g, o))
            }
            Some((b, ops)) => {
                let (sel, tbl) = ops[n];
                let i = u32::from(g.read::<u8>(rd(g, sel) + b));
                (i, g.read::<u32>(w(tbl).wrapping_add(4 * i)))
            }
        };
        let (idx, act) = actor(g, 0);
        if m == 0 {
            // `S + d` null: no caller's `S` gets here
            if rd(g, sl[1]) == 0 && idx.cast_signed() < 0x20 {
                g.call(t(g, sl[2], 217), 0, &[idx, v]);
                g.call(t(g, sl[3], 255), 0, &[idx, v]);
            }
        } else {
            g.call(t(g, sl[0], 210), 0, &[act, m]);
        }
        g.call(t(g, sl[4], 211), 0, &[act, m]);
        g.call(t(g, sl[5], 235), 0, &[m]);
        let (_, a2) = actor(g, 1);
        let x = st0_f32(g.call(t(g, sl[6], 277), 0, &[a2]).st0);
        let x = mul(x, k(g, sl[7]));
        for o in [0x28u32, 0x14, 0] {
            g.write::<f32>(m + o, x);
        }
        let (_, a3) = actor(g, 2);
        let x = st0_f32(g.call(t(g, sl[8], 15), 0, &[a3]).st0);
        let y = mul(g.read::<f32>(m), x);
        g.write::<f32>(m, y);
        for ((tb, n), (o, ff)) in [(sl[9], 16u32), (sl[10], 17)]
            .into_iter()
            .zip([0x14u32, 0x28].into_iter().zip(bl.f_first))
        {
            let x = st0_f32(g.call(t(g, tb, n), 0, &[a3]).st0);
            let y = g.read::<f32>(m + o);
            g.write::<f32>(m + o, if ff { mul(x, y) } else { mul(y, x) });
        }
        // call width: 3
        g.call(t(g, sl[11], 330), 0, &[m, m, 0]);
        // call width: 3
        g.call(t(g, sl[12], 328), 0, &[m, m, 0]);
        // call width: 3
        g.call(t(g, sl[13], 329), 0, &[m, m, a.to_bits()]);
        let r = s + 0x1940 + 0x20 * i;
        for j in 0..4 {
            let x = g.read::<u32>(r + 0x10 + 4 * j);
            g.write::<u32>(m + 0x30 + 4 * j, x);
        }
        if i < 2 {
            record(g, i + 1);
        }
    }
    let ix = rd(g, 0x6c2);
    // call width: 2
    g.call(t(g, 0x6ba, 238), 0, &[ix, 1]);
    for (sel, b, tb) in [(0x6cfu32, 1u32, 0x6dbu32), (0x6e8, 2, 0x6f4)] {
        let i = u32::from(g.read::<u8>(rd(g, sel) + b));
        // call width: 2
        g.call(t(g, tb, 238), 0, &[i, 1]);
    }
    let i = rd(g, 0x70f);
    let mut eax = if rd(g, 0x705) == 0 && i.cast_signed() < 0x20 {
        // call width: 2
        g.call(t(g, 0x71c, 296), 0, &[i, 0]).eax
    } else {
        // call width: 2
        g.call(t(g, 0x729, 257), 0, &[rd(g, 0x731), 1]).eax
    };
    // (the selection's reread, byte, battle word, 296's, ACTORS, 257's)
    for (sel, b, war, s296, tbl, s257) in [
        (0x73eu32, 1u32, 0x747u32, 0x758u32, 0x770u32, 0x775u32),
        (0x782, 2, 0x78b, 0x79c, 0x7b4, 0x7b9),
    ] {
        let i = u32::from(g.read::<u8>(rd(g, sel) + b));
        eax = if rd(g, war) == 0 && i < 0x20 {
            // call width: 2
            g.call(t(g, s296, 296), 0, &[i, 0]).eax
        } else {
            let act = g.read::<u32>(w(tbl).wrapping_add(4 * i));
            // call width: 2
            g.call(t(g, s257, 257), 0, &[act, 1]).eax
        };
    }
    g.write::<f32>(s + 0x21f8, a);
    g.write::<u32>(s + 0x2214, 0);
    crate::r25::abs_stores(g, img, f, 0x7e4, 0x80c);
    eax
}

/// [`summon_frames_0589`] (2070 bytes, 1 overlay).
pub const SUMMON_FRAMES_0589: Sig = Sig {
    len: 2070,
    hash: 0x1304_3f6e_9cf1_f78b,
    calls: &[
        // __security_check_cookie
        (0x80e, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0589`'s start-up (0x100049c0, 418 bytes; no arguments, called
/// by its load step; no frame, `ebx` pushed): slot 718 `(P0)`; the actor
/// count `*G_N` less one and less two as the bytes `G_B1`/`G_B2` and the
/// actors `ACTORS[n - 1]`/`[n - 2]` = slot 215 `(n - 1)`/`(n - 2)` (the
/// statics' store run between); `*G_K` 3; slot 719 `(P, 8, 0)` into
/// `G_H`; the part set up ([`part_setup`](crate::sb_misc::part_setup)
/// `(BUF)`, slot 718's word under it), the summon frames
/// ([`summon_frames_0589`]), the turned start
/// ([`start_turned`](crate::o4_effect::start_turned)); the two script
/// blocks' words; slot 271 `(4)`; two task nodes (kinds 1 and 2,
/// [`take`](crate::sa_pool::take)) on the task list, the records block
/// after the first ([`records_init_400`](crate::sa_misc::records_init_400)).
/// Returns 0.
pub fn start_up_0589<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    g.call(t(g, 0x1, 718), 0, &[w(0xd)]);
    let n = g.read::<u32>(w(0x15)).wrapping_sub(1);
    let s215 = t(g, 0x1a, 215);
    g.write::<u8>(w(0x21), n as u8);
    crate::r25::abs_stores(g, img, f, 0x2c, 0x54);
    let r = g.call(s215, 0, &[n]).eax;
    g.write::<u32>(w(0x59).wrapping_add(4 * n), r);
    let n = n.wrapping_sub(1);
    let s215 = t(g, 0x5e, 215);
    g.write::<u8>(w(0x65), n as u8);
    let r = g.call(s215, 0, &[n]).eax;
    g.write::<u32>(w(0x75).wrapping_add(4 * n), r);
    let s719 = t(g, 0x7a, 719);
    crate::r25::abs_stores(g, img, f, 0x80, 0x8a);
    // call width: 3
    let h = g.call(s719, 0, &[w(0x93), 8, 0]).eax;
    g.write::<u32>(w(0x9d), h);
    // push ebx; slot 718's word stays; push BUF: the return address at
    // `E - 0x10`
    let esp = fr.esp.wrapping_sub(0x10);
    g.write::<u32>(esp + 8, w(0xd));
    g.write::<u32>(esp + 4, w(0xa2));
    crate::sb_misc::part_setup(g, img, call(0xa7), esp);
    summon_frames_0589(g, img, call(0xac), esp);
    let at = Frame {
        at: call(0xb1),
        esp,
        ..Frame::default()
    };
    crate::o4_effect::start_turned(g, img, &at);
    let s271 = t(g, 0xb6, 271);
    crate::r25::abs_stores(g, img, f, 0xba, 0x102);
    g.call(s271, 0, &[4]);
    let n1 = crate::sa_pool::take(g, img, call(0x115), w(0x110));
    g.write::<u8>(n1 + 0x18, 1);
    g.write::<u32>(n1 + 4, 0);
    if g.read::<u32>(w(0x11b)) == 0 {
        g.write::<u32>(n1, 0);
        g.write::<u32>(w(0x142), n1);
    } else {
        let last = g.read::<u32>(w(0x130));
        g.write::<u32>(n1, last);
        g.write::<u32>(last + 4, n1);
    }
    g.write::<u32>(n1 + 8, n1);
    g.write::<u32>(w(0x14f), n1);
    crate::sa_misc::records_init_400(g, img, call(0x154), w(0x147));
    let n2 = crate::sa_pool::take(g, img, call(0x15e), w(0x159));
    g.write::<u8>(n2 + 0x18, 2);
    g.write::<u32>(n2 + 4, 0);
    if g.read::<u32>(w(0x164)) == 0 {
        g.write::<u32>(n2, 0);
        g.write::<u32>(w(0x193), n2);
        g.write::<u32>(n2 + 8, n2);
        g.write::<u32>(w(0x19b), n2);
    } else {
        let last = g.read::<u32>(w(0x178));
        g.write::<u32>(n2, last);
        g.write::<u32>(last + 4, n2);
        g.write::<u32>(n2 + 8, n2);
        g.write::<u32>(w(0x185), n2);
    }
    Ret::eax(0)
}

/// [`start_up_0589`] (418 bytes, 1 overlay).
pub const START_UP_0589: Sig = Sig {
    len: 418,
    hash: 0xca7e_c413_678f_bf3f,
    calls: &[
        (0xa7, Callee::Sig(&crate::sb_misc::PART_SETUP)),
        (0xac, Callee::Sig(&SUMMON_FRAMES_0589)),
        (0xb1, Callee::Sig(&crate::o4_effect::START_TURNED_0589_1E00)),
        (0x115, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x154, Callee::Sig(&crate::sa_misc::RECORDS_INIT_400)),
        (0x15e, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// `magic_0589`'s load step (0x100012e0, the 118-byte shape) running
/// [`start_up_0589`].
pub fn load_0589<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::r9_load::load_step(g, img, fr, crate::r9_load::Shape::Zero, start_up_0589)
}

/// [`load_0589`].
pub const LOAD_0589: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&START_UP_0589))],
    imports: &[],
};

/// `magic_0589`'s three points (0x10004370, 1112 bytes; a table callback,
/// no arguments; a cookie frame, `ebp = E - 4`: the matrices at `ebp -
/// 0x64` and `ebp - 0xa4`, the turned offset at `ebp - 0x24`, the offset at
/// `ebp - 0x14`; the shape of
/// [`three_points_around`](crate::s6_misc::three_points_around)). With `S =
/// *G_S` and its yaw `S + 0x21f8`: for each point `i` the record `S +
/// 0x1a10 + 0x10 i` = `S + 0x1950 + 0x20 i` and `P_i = S + 0x1a40 + 0x10 i`
/// a copy of `S + 0x1a00`, then the offset `(-16.5, 0, -20)`, `(0, 0,
/// -25)`, `(16.5, 0, -20)` turned by the yaw (slots 235, 328 with 0, 329,
/// 229; the second point with a matrix of its own) added to `P_i` (slot
/// 248, `w` 1.0). Then `S + 0x2208..` = `(0x43, 0x1c, 0x23)` and the
/// objects `S + 0x1430` and `S + 0x1560` started
/// ([`start_object_anim`](crate::s4_misc::start_object_anim): part 0xd,
/// values 0x13 and 0x1b, scale `K`) at the first two points, turned by the
/// yaw, `y` less `K2`. Returns the yaw's word.
#[allow(clippy::too_many_lines)] // one straight sequence
pub fn three_points_0589<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::sub;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let l = |off: u32| ebp.wrapping_sub(off);
    let (lv, lo) = (l(0x24), l(0x14));
    let s = g.read::<u32>(w(0x17));
    let yaw = g.read::<u32>(s + 0x21f8);
    let copy4 = |g: &mut G, src: u32, dst: u32| {
        for i in 0..4 {
            let x = g.read::<u32>(src + 4 * i);
            g.write::<u32>(dst + 4 * i, x);
        }
    };
    g.write::<u32>(lo + 4, 0);
    // (the offset's x and z, the matrix, the slots' table operands)
    for (i, (x, z, m, ops)) in (0u32..).zip([
        (
            0xc184_0000u32,
            0xc1a0_0000u32,
            l(0x64),
            [0x8fu32, 0xd2, 0xf4, 0x10d, 0x125],
        ),
        (0, 0xc1c8_0000, l(0xa4), [0x1a4, 0x1d1, 0x1f6, 0x212, 0x22a]),
        (
            0x4184_0000,
            0xc1a0_0000,
            l(0x64),
            [0x2b0, 0x2da, 0x2fc, 0x315, 0x32d],
        ),
    ]) {
        copy4(g, s + 0x1950 + 0x20 * i, s + 0x1a10 + 0x10 * i);
        let p = s + 0x1a40 + 0x10 * i;
        copy4(g, s + 0x1a00, p);
        g.write::<u32>(lo, x);
        g.write::<u32>(lo + 8, z);
        g.write::<u32>(lo + 0xc, ONE);
        g.call(t(g, ops[0], 235), 0, &[m]);
        // call width: 3
        g.call(t(g, ops[1], 328), 0, &[m, m, 0]);
        // call width: 3
        g.call(t(g, ops[2], 329), 0, &[m, m, yaw]);
        g.write::<u32>(lo + 0xc, ONE);
        // call width: 3
        g.call(t(g, ops[3], 229), 0, &[lv, m, lo]);
        // call width: 3
        g.call(t(g, ops[4], 248), 0, &[p, p, lv]);
        g.write::<u32>(p + 0xc, ONE);
    }
    for (o, v) in [(0x2208u32, 0x43u32), (0x220c, 0x1c), (0x2210, 0x23)] {
        g.write::<u32>(s + o, v);
    }
    for (o, c, value, sc, p, ky) in [
        (0x1430u32, 0x37eu32, 0x13u32, 0x33fu32, 0x1a40u32, 0x3c8u32),
        (0x1560, 0x3f0, 0x1b, 0x38e, 0x1a50, 0x42f),
    ] {
        let obj = s + o;
        let scale = k(g, sc);
        crate::s4_misc::start_object_anim(g, img, call(c), obj, scale, 0xd, value);
        copy4(g, s + p, obj + 0xd0);
        let y = sub(g.read::<f32>(obj + 0xd4), k(g, ky));
        let yaw = g.read::<u32>(s + 0x21f8);
        g.write::<u32>(obj + 0xc4, yaw);
        g.write::<f32>(obj + 0xd4, y);
    }
    Ret::eax(g.read::<u32>(s + 0x21f8))
}

/// [`three_points_0589`] (1112 bytes, 1 overlay).
pub const THREE_POINTS_0589: Sig = Sig {
    len: 1112,
    hash: 0x6e7f_6124_8c85_07d4,
    calls: &[
        (0x37e, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x3f0, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        // __security_check_cookie
        (0x450, Callee::Any),
    ],
    imports: &[],
};

/// One object of [`three_objects_with`]: the object's offset in `S`, its
/// part, the scale's and the call's operands, the offset `(x, 0, z)`, the
/// slots 235/328/329/229/248's table operands, the turn, `y`'s constant
/// operand and the height global's operand (`magic_0511`'s copy adds it).
pub(crate) struct ThreeObj {
    pub o: u32,
    pub part: u32,
    pub sc: u32,
    pub c: u32,
    pub x: u32,
    pub z: u32,
    pub ops: [u32; 5],
    pub turn: Turn,
    pub ky: u32,
    pub h: Option<u32>,
}

/// How [`three_objects_with`] turns an object: the yaw kept, or the yaw
/// less (`add` false) or plus a constant, wrapped into `-pi..pi`
/// (operands: the constant, the upper bound, its step, the lower bound,
/// its step).
pub(crate) enum Turn {
    Kept,
    Wrap { add: bool, ops: [u32; 5] },
}

/// The record layout of a three-objects copy: `G_S`'s operand, the yaw,
/// the place copied and where each place is kept (`+ 0x20 i`).
pub(crate) struct ThreeObjects {
    pub s: u32,
    pub yaw: u32,
    pub src: u32,
    pub keep: u32,
    pub objs: [ThreeObj; 3],
}

/// `magic_0589`'s three objects (0x10002e30, 1109 bytes; a table
/// callback, no arguments; a cookie frame, `ebp = E - 4`: the matrix at
/// `ebp - 0x64`, the turned offset at `ebp - 0x24`, the offset at `ebp -
/// 0x14`). With `S = *G_S` and its yaw `S + 0x21f8`: the objects `S +
/// 0x980`, `S + 0x850`, `S + 0x720` started
/// ([`start_object_anim`](crate::s4_misc::start_object_anim): parts 8, 7,
/// 6, value 0, scales `K1..K3`) at `S + 0x1a00` moved by `(-16.5, 0,
/// -0.8)`, `(0, 0, 15.5)`, `(16.5, 0, -0.8)` turned by the yaw (slots 235,
/// 328 with 0, 329, 229, 248; `w` 1.0), each place kept at `S + 0x1950 +
/// 0x20 i`; turned by the yaw less `K4`, the yaw, the yaw plus `K5` (into
/// `-pi..pi`), `y` less `K6`. Returns the last place's `w`.
pub fn three_objects_0589<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let l = ThreeObjects {
        s: 0x1b,
        yaw: 0x21f8,
        src: 0x1a00,
        keep: 0x1950,
        objs: [
            ThreeObj {
                o: 0x980,
                part: 8,
                sc: 0x14,
                c: 0x4c,
                x: 0xc184_0000,
                z: 0xbf4c_cccd,
                ops: [0x8b, 0xb5, 0xd4, 0xed, 0x105],
                turn: Turn::Wrap {
                    add: false,
                    ops: [0x15a, 0x164, 0x117, 0x174, 0x117],
                },
                ky: 0x19d,
                h: None,
            },
            ThreeObj {
                o: 0x850,
                part: 7,
                sc: 0x185,
                c: 0x1b9,
                x: 0,
                z: 0x4178_0000,
                ops: [0x1f8, 0x222, 0x241, 0x25a, 0x272],
                turn: Turn::Kept,
                ky: 0x2c7,
                h: None,
            },
            ThreeObj {
                o: 0x720,
                part: 6,
                sc: 0x28d,
                c: 0x2ef,
                x: 0x4184_0000,
                z: 0xbf4c_cccd,
                ops: [0x32e, 0x358, 0x377, 0x390, 0x3a8],
                turn: Turn::Wrap {
                    add: true,
                    ops: [0x3f5, 0x3ff, 0x409, 0x413, 0x420],
                },
                ky: 0x43b,
                h: None,
            },
        ],
    };
    three_objects_with(g, img, fr, &l)
}

/// The body of [`three_objects_0589`] and its copies, from the copy's
/// layout `l`.
pub(crate) fn three_objects_with<G: Game>(
    g: &mut G,
    img: &Image,
    fr: &Frame,
    l: &ThreeObjects,
) -> Ret {
    use crate::sb_sse::{add, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let lc = |off: u32| ebp.wrapping_sub(off);
    let (m, lv, lo) = (lc(0x64), lc(0x24), lc(0x14));
    let s = g.read::<u32>(w(l.s));
    let yaw = g.read::<u32>(s + l.yaw);
    g.write::<u32>(lo + 4, 0);
    let mut eax = 0;
    for (i, ob) in (0u32..).zip(&l.objs) {
        let obj = s + ob.o;
        let scale = k(g, ob.sc);
        crate::s4_misc::start_object_anim(g, img, call(ob.c), obj, scale, ob.part, 0);
        let q = obj + 0xd0;
        for j in 0..4 {
            let v = g.read::<u32>(s + l.src + 4 * j);
            g.write::<u32>(q + 4 * j, v);
        }
        g.write::<u32>(lo, ob.x);
        g.write::<u32>(lo + 8, ob.z);
        g.call(t(g, ob.ops[0], 235), 0, &[m]);
        // call width: 3
        g.call(t(g, ob.ops[1], 328), 0, &[m, m, 0]);
        // call width: 3
        g.call(t(g, ob.ops[2], 329), 0, &[m, m, yaw]);
        g.write::<u32>(lo + 0xc, ONE);
        // call width: 3
        g.call(t(g, ob.ops[3], 229), 0, &[lv, m, lo]);
        // call width: 3
        g.call(t(g, ob.ops[4], 248), 0, &[q, q, lv]);
        g.write::<u32>(q + 0xc, ONE);
        for j in 0..4 {
            eax = g.read::<u32>(q + 4 * j);
            g.write::<u32>(s + l.keep + 0x20 * i + 4 * j, eax);
        }
        let yf = g.read::<f32>(s + l.yaw);
        let turn = match ob.turn {
            Turn::Kept => yf,
            Turn::Wrap { add: up, ops } => {
                let a = if up {
                    add(yf, k(g, ops[0]))
                } else {
                    sub(yf, k(g, ops[0]))
                };
                if a > k(g, ops[1]) {
                    sub(a, k(g, ops[2]))
                } else if k(g, ops[3]) > a {
                    add(a, k(g, ops[4]))
                } else {
                    a
                }
            }
        };
        let mut y = sub(g.read::<f32>(obj + 0xd4), k(g, ob.ky));
        if let Some(h) = ob.h {
            y = add(y, k(g, h));
        }
        g.write::<f32>(obj + 0xc4, turn);
        g.write::<f32>(obj + 0xd4, y);
    }
    Ret::eax(eax)
}

/// [`three_objects_0589`] (1109 bytes, 1 overlay).
pub const THREE_OBJECTS_0589: Sig = Sig {
    len: 1109,
    hash: 0x58d1_3314_2963_3279,
    calls: &[
        (0x4c, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x1b9, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x2ef, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        // __security_check_cookie
        (0x44d, Callee::Any),
    ],
    imports: &[],
};

/// One of [`fading_summons_0589`]'s blocks: slot 235, slot 277 (sel,
/// ACTORS, table), its constant, slot 15 (sel, ACTORS, table), 16, 17,
/// 330/328/329, the products `f * m` for 15/16/17, slot 31 (sel, ACTORS,
/// table) (operand offsets).
type FadeBlock0589 = (
    u32,
    [u32; 3],
    u32,
    [u32; 3],
    u32,
    u32,
    [u32; 3],
    [bool; 3],
    [u32; 3],
);

/// `magic_0589`'s fading summons (0x10003290, 1491 bytes; a task
/// callback, cdecl `(task)`; an `ebp` frame keeping its floats in the
/// argument's own word). With `S = *G_S`: the count `task + 0x2c` one
/// down while above 0, `s` = it over `K`; a fade record `S + 0x1be8`
/// ([`fade_task`](crate::s3_misc::fade_task): 0 and `K1`, `edx` 0x1e);
/// for the third, fourth and fifth selected actors the model matrices `S +
/// 0x1880 + 0x40 i` as [`summon_models_0589`]'s (slots 235, 277, 15, 16,
/// 17, 330, 328, 329 with `S + 0x19a0 + 0x20 i`), every term then times
/// `c` (first `s 0 + (1 - s) K2`, then `s 0 + (1 - s) K3` twice) and slot 31 `(actor, v)` with `v` = `s K4 + (1 - s) K2`, then
/// that plus `(1 - s) K3`, then the same (kept in the argument word).
///
/// Returns the last slot 31's `eax`.
#[allow(clippy::too_many_lines)] // one straight sequence
pub fn fading_summons_0589<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::ob_model::st0_f32;
    use crate::sb_sse::{add, cvt, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let rd = |g: &G, o: u32| g.read::<u32>(w(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let task = fr.arg(g, 0);
    let argw = fr.esp + 4;
    let s = rd(g, 0xf);
    let n = g.read::<i32>(task + 0x2c);
    if n > 0 {
        g.write::<i32>(task + 0x2c, n - 1);
    }
    let sv = div(cvt(g.read::<u32>(task + 0x2c)), k(g, 0x31));
    g.write::<f32>(argw, sv);
    // push 0 below the three saved registers: the return address at `E -
    // 0x24`
    let esp = fr.esp.wrapping_sub(0x24);
    g.write::<u32>(esp + 4, 0);
    let ff = Frame {
        at: img.rel32_target(f + 0x4b),
        esp,
        ecx: s + 0x1be8,
        edx: 0x1e,
        xmm: [0, 0, u128::from(g.read::<u32>(w(0x29))), 0],
        ..*fr
    };
    crate::s3_misc::fade_task(g, img, &ff);
    // (slot 235, 277 (sel, ACTORS, table), K, 15's (sel, ACTORS, table),
    // 16, 17, 330, 328, 329, products `f * m` for 15/16/17, the slot 31
    // operands (sel, ACTORS, table))
    let blocks: [FadeBlock0589; 3] = [
        (
            0x50,
            [0x64, 0x6f, 0x74],
            0x8c,
            [0x9f, 0xaa, 0xaf],
            0xc6,
            0xe2,
            [0xff, 0x12b, 0x14a],
            [false, false, false],
            [0x21c, 0x22c, 0x231],
        ),
        (
            0x23b,
            [0x24f, 0x25a, 0x25f],
            0x277,
            [0x28a, 0x295, 0x29a],
            0x2b1,
            0x2cd,
            [0x2ea, 0x316, 0x335],
            [false, true, false],
            [0x3ea, 0x3fe, 0x403],
        ),
        (
            0x412,
            [0x426, 0x431, 0x436],
            0x44e,
            [0x461, 0x46c, 0x471],
            0x488,
            0x4a4,
            [0x4b4, 0x4ed, 0x50c],
            [true, false, true],
            [0x5a6, 0x5bb, 0x5c0],
        ),
    ];
    let mut eax = 0;
    let (mut u, mut z, mut c) = (0.0, 0.0, 0.0);
    for (i, (s235, a277, k277, a15, s16, s17, turn, ff, a31)) in (0u32..).zip(blocks) {
        let b = 3 + i;
        let actor = |g: &G, sel: u32, tbl: u32| {
            let x = u32::from(g.read::<u8>(rd(g, sel) + b));
            g.read::<u32>(w(tbl).wrapping_add(4 * x))
        };
        let m = s + 0x1880 + 0x40 * i;
        g.call(t(g, s235, 235), 0, &[m]);
        let a2 = actor(g, a277[0], a277[1]);
        let x = st0_f32(g.call(t(g, a277[2], 277), 0, &[a2]).st0);
        let x = mul(x, k(g, k277));
        for o in [0x28u32, 0x14, 0] {
            g.write::<f32>(m + o, x);
        }
        let a3 = actor(g, a15[0], a15[1]);
        for ((tb, n), (o, first)) in [(a15[2], 15u32), (s16, 16), (s17, 17)]
            .into_iter()
            .zip([0u32, 0x14, 0x28].into_iter().zip(ff))
        {
            let x = st0_f32(g.call(t(g, tb, n), 0, &[a3]).st0);
            let y = g.read::<f32>(m + o);
            g.write::<f32>(m + o, if first { mul(x, y) } else { mul(y, x) });
        }
        for ((tb, n), d) in [(turn[0], 330u32), (turn[1], 328), (turn[2], 329)]
            .into_iter()
            .zip([0x19a8u32, 0x19a0, 0x19a4])
        {
            let ang = g.read::<u32>(s + d + 0x20 * i);
            // call width: 3
            g.call(t(g, tb, n), 0, &[m, m, ang]);
        }
        let terms = [0u32, 4, 8, 0x10, 0x14, 0x18, 0x20, 0x24, 0x28];
        let v = match i {
            0 => {
                let sv = g.read::<f32>(argw);
                u = sub(k(g, 0x164), sv);
                let x2 = mul(u, k(g, 0x178));
                z = mul(sv, 0.0);
                let x3 = mul(sv, k(g, 0x18f));
                c = add(z, x2);
                g.write::<f32>(argw, x3);
                for o in terms {
                    let y = mul(c, g.read::<f32>(m + o));
                    g.write::<f32>(m + o, y);
                }
                add(x3, x2)
            }
            1 => {
                // the saved `s 0` (not the first factor) plus `(1 - s) K3`
                let x1 = mul(u, k(g, 0x354));
                c = add(z, x1);
                for o in terms {
                    let y = if o == 8 {
                        mul(c, g.read::<f32>(m + o))
                    } else {
                        mul(g.read::<f32>(m + o), c)
                    };
                    g.write::<f32>(m + o, y);
                }
                let v = add(g.read::<f32>(argw), x1);
                g.write::<f32>(argw, v);
                v
            }
            _ => {
                for o in terms {
                    let y = if o == 0 {
                        mul(c, g.read::<f32>(m + o))
                    } else {
                        mul(g.read::<f32>(m + o), c)
                    };
                    g.write::<f32>(m + o, y);
                }
                g.read::<f32>(argw)
            }
        };
        let a4 = actor(g, a31[0], a31[1]);
        // call width: 2
        eax = g.call(t(g, a31[2], 31), 0, &[a4, v.to_bits()]).eax;
    }
    Ret::eax(eax)
}

/// [`fading_summons_0589`] (1491 bytes, 1 overlay).
pub const FADING_SUMMONS_0589: Sig = Sig {
    len: 1491,
    hash: 0x6406_ebcb_d8a3_663e,
    calls: &[(0x4b, Callee::Sig(&crate::s3_misc::FADE_TASK))],
    imports: &[],
};

/// `magic_0597`'s start-up (0x1000a9c0, 868 bytes; cdecl `(actor)`; a
/// table entry; an 8-aligned frame; [`start_up_0613`](crate::r23::start_up_0613)'s
/// kind): the targets of the actor
/// ([`targets_init_c`](crate::s4_summon::targets_init_c)); the statics'
/// store run (two objects, four static nodes); slot 718 `(P)`, slot 719
/// `(P, 8, 0)` into `G_H`, slot 711 `(h, BUF, 0)` with the game lock
/// `**G_LOCK` 1 around it, slot 710 `(OUT, *G_H, DST, 0x100000)`; 29
/// groups of three entries: the bytes `REC + 0x60/0x30/0` 0 and the step
/// table `STEPS[ids]` = `STEP`; slots 222 `(P, 0, 9, 2)`, 221 `(P, 0, 1000,
/// 2)`, 551 `(slot 719 (P, 6, 2), 0, 2)` (the part shared with
/// [`start_up_0570`]), 238 `(*G_A, 1)`, 296 `(*G_A,
/// 0)`; the two objects each joining the other's ring (their words, the
/// flag bit 0 cleared). Returns 0.
pub fn start_up_0597<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r25::abs_stores;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    start_up_head_0597(g, img, fr);
    // the first object joins the second's ring
    let last = g.read::<u32>(w(0x2ab));
    g.write::<u32>(w(0x2b0), last);
    abs_stores(g, img, f, 0x2b4, 0x2dc);
    g.write::<u32>(last + 4, w(0x2df));
    let last = g.read::<u32>(w(0x2e4));
    let fl = g.read::<u32>(w(0x2ea));
    g.write::<u32>(w(0x2ea), fl & !1);
    g.write::<u32>(w(0x2f4), 0);
    g.write::<u32>(w(0x2fa), 0);
    g.write::<u32>(w(0x2ff), last);
    abs_stores(g, img, f, 0x303, 0x335);
    // and the second the first's
    g.write::<u32>(last + 4, w(0x338));
    let fl = g.read::<u32>(w(0x33e));
    g.write::<u32>(w(0x33e), fl & !1);
    g.write::<u32>(w(0x345), 0);
    abs_stores(g, img, f, 0x349, 0x35d);
    Ret::eax(0)
}

/// The part [`start_up_0597`] and [`start_up_0570`] share, up to slot 296
/// (the same code at the same offsets; slot 719's last argument read from
/// its `push imm8`).
fn start_up_head_0597<G: Game>(g: &mut G, img: &Image, fr: &Frame) {
    use crate::r25::abs_stores;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let actor = fr.arg(g, 0);
    // push ebp; and esp, -8; push ecx; push esi: the return address 4
    // below
    let esp = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0xc);
    crate::s4_summon::targets_init_c(g, img, img.rel32_target(f + 0xc), esp, actor);
    let s718 = t(g, 0x11, 718);
    abs_stores(g, img, f, 0x15, 0x13d);
    abs_stores(g, img, f, 0x148, 0x15c);
    g.call(s718, 0, &[w(0x144)]);
    // call width: 3
    let h = g.call(t(g, 0x15f, 719), 0, &[w(0x171), 8, 0]).eax;
    let lock = g.read::<u32>(w(0x179));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(w(0x185), h);
    g.write::<u32>(lock, 1);
    // call width: 3
    g.call(t(g, 0x190, 711), 0, &[h, w(0x18a), 0]);
    let lock = g.read::<u32>(w(0x1a3));
    g.write::<u32>(lock, saved);
    let hh = g.read::<u32>(w(0x1c3));
    // call width: 4
    g.call(t(g, 0x1ad, 710), 0, &[w(0x1c8), hh, w(0x1bd), 0x10_0000]);
    let (mut ids, mut rec) = (w(0x1d2), w(0x1d7));
    for _ in 0..0x1d {
        for (b, r, tbl, imm) in [(1u32, 0x60u32, 0x1ebu32, 0x1efu32), (0, 0x30, 0x1fd, 0x201)] {
            let c = u32::from(g.read::<u8>(ids + b));
            g.write::<u8>(rec + r, 0);
            g.write::<u32>(w(tbl).wrapping_add(4 * c), w(imm));
        }
        let c = u32::from(g.read::<u8>(ids.wrapping_sub(1)));
        g.write::<u8>(rec, 0);
        g.write::<u32>(w(0x20f).wrapping_add(4 * c), w(0x213));
        ids = ids.wrapping_sub(3);
        rec = rec.wrapping_sub(0x90);
    }
    // call width: 4
    g.call(t(g, 0x224, 222), 0, &[w(0x234), 0, 9, 2]);
    // call width: 4
    g.call(t(g, 0x23b, 221), 0, &[w(0x24e), 0, 0x3e8, 2]);
    // call width: 3
    let r = g
        .call(t(g, 0x255, 719), 0, &[w(0x264), 6, w(0x25a) & 0xff])
        .eax;
    // call width: 3
    g.call(t(g, 0x272, 551), 0, &[r, 0, 2]);
    // call width: 2
    g.call(t(g, 0x27f, 238), 0, &[g.read::<u32>(w(0x290)), 1]);
    // call width: 2
    g.call(t(g, 0x297, 296), 0, &[g.read::<u32>(w(0x29e)), 0]);
}

/// [`start_up_0597`] (868 bytes, 1 overlay).
pub const START_UP_0597: Sig = Sig {
    len: 868,
    hash: 0x903d_736b_5a47_832b,
    calls: &[(0xc, Callee::Sig(&crate::s4_summon::TARGETS_INIT_C))],
    imports: &[],
};

/// `magic_0570`'s start-up (0x1000c740, 909 bytes; cdecl `(actor)`):
/// [`start_up_0597`] with slot 719 `(P, 6, 5)` and a longer end: the two
/// objects joining each other's rings in another store order (word `0x96`
/// for 0597's `0x384`), then slot 276 `(slot 215 (*G_A), 0xff)` (the actor
/// read before the rings). Returns 0.
pub fn start_up_0570<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r25::abs_stores;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    start_up_head_0597(g, img, fr);
    let last = g.read::<u32>(w(0x2ab));
    let actor = g.read::<u32>(w(0x2b1));
    // the first object joins the second's ring
    g.write::<u32>(w(0x2b6), last);
    abs_stores(g, img, f, 0x2ba, 0x2e2);
    g.write::<u32>(last + 4, w(0x2e5));
    let last = g.read::<u32>(w(0x2ea));
    let fl = g.read::<u32>(w(0x2f0));
    g.write::<u32>(w(0x2f0), fl & !1);
    g.write::<u32>(w(0x2f6), last);
    abs_stores(g, img, f, 0x2fa, 0x304);
    g.write::<u32>(w(0x306), 0);
    g.write::<u32>(w(0x30c), 0);
    abs_stores(g, img, f, 0x310, 0x338);
    // and the second the first's
    g.write::<u32>(last + 4, w(0x33b));
    let s215 = t(g, 0x340, 215);
    let fl = g.read::<u32>(w(0x346));
    g.write::<u32>(w(0x346), fl & !1);
    abs_stores(g, img, f, 0x34b, 0x355);
    g.write::<u32>(w(0x357), 0);
    abs_stores(g, img, f, 0x35b, 0x365);
    let r = g.call(s215, 0, &[actor]).eax;
    // call width: 2
    g.call(t(g, 0x377, 276), 0, &[r, 0xff]);
    Ret::eax(0)
}

/// [`start_up_0570`] (909 bytes, 1 overlay).
pub const START_UP_0570: Sig = Sig {
    len: 909,
    hash: 0xb5fa_4f01_3b29_6d26,
    calls: &[(0xc, Callee::Sig(&crate::s4_summon::TARGETS_INIT_C))],
    imports: &[],
};
