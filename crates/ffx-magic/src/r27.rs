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
