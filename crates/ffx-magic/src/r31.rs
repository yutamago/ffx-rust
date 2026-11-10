//! Round 31's own ports (own rows of `magic_0450` and up): the DLLs' own
//! code written out, and copies bound by their own `Sig`.

use crate::ob_model::{st0_f32, wrap_once};
use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

const ONE: u32 = 0x3f80_0000;

/// `magic_0605`'s spinning pieces (0x100082f0, 1280 bytes; a task
/// callback, no arguments; a /GS frame). With `S = *G_S` (read once) and
/// the records `R = S + D + 0xd4 k` for `k < *G_N` (operand `+0x12`, read
/// again each round; `D` the displacement `+0x23`), `E = R + 0x24`:
/// - the record's motion script runs ([`motion_script`](crate::s4_misc::motion_script),
///   `ecx = R`); "all done" stays set only while each answers non-zero
///   and has `E + 0xac` set;
/// - a delay `E + 0xa0` above 0 counts down and the record waits;
/// - else a blend count `n = E + 0x9c` above 0 makes the yaw `Y = (C1 -
///   n / C2) E[0x98] + E[0x94] n / C2`, adds `E + 0x50` to `E + 0x40` and
///   that to `E + 0x70`, counts down, and turns `E + 0x8c` by `E + 0x90`
///   (wrapped); without it `Y = E[0x98]`;
/// - mode `E + 0xa8` 1: without `E + 0xa4` the angles `E + 0x5c`, `E +
///   0x60` move by slots 404 / 314 `(pi / 200)` (ST0) plus constants, else
///   by constants; any other mode turns `E + 0x5c..0x64` by one constant
///   (each wrapped);
/// - a unit `M` (slot 235) turned by `E + 0x8c` (slot 329), `V` = slot 229
///   `(V, M, (0, 0, -Y, 0))`, the record's scale on `R + 0x20`
///   ([`actor_scale`](crate::sa_misc::actor_scale), `ecx = k`), slots
///   330 / 328 / 329 by `E + 0x64`, `E + 0x5c`, `E + 0x60`, its three rows
///   times `E + 0x7c`, `E + 0x80`, `E + 0x84`;
/// - mode 1 places it at `S + P + V` (`P` the displacements `+0x46a`..,
///   the fourth word copied), else at `E + 0x6c`, `E + 0x74`; `E + 0x30`
///   always gets `E + 0x70`.
///
/// When every record is done, `S + F` (displacement `+0x4e9`) is set to
/// 1 (also with no records). Returns the last `k` (the cookie word `ebp ^
/// cookie` with no records). `magic_0307` 0x10003770 is the same code with
/// other displacements (family P015).
#[allow(clippy::too_many_lines)] // the original's branches, one by one
pub fn spin_pieces_0605<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let c = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let ebp = fr.esp.wrapping_sub(4);
    let cookie = g.read::<u32>(w(0x7)) ^ ebp;
    g.write::<u32>(ebp.wrapping_sub(4), cookie);
    let s = g.read::<u32>(w(0x1a));
    let (m, v, l) = (
        ebp.wrapping_sub(0x64),
        ebp.wrapping_sub(0x24),
        ebp.wrapping_sub(0x14),
    );
    for o in [0u32, 4, 0xc] {
        g.write::<u32>(l.wrapping_add(o), 0);
    }
    let count = |g: &G| g.read::<u32>(w(0x12)).cast_signed();
    let mut all = 1u32;
    let mut eax = cookie;
    if count(g) > 0 {
        let script = img.rel32_target(f + 0x5a);
        let scale = img.rel32_target(f + 0x371);
        let mut r = s.wrapping_add(w(0x23));
        let mut k = 0u32;
        loop {
            let e = r.wrapping_add(0x24);
            let at = |o: u32| e.wrapping_add(o);
            let rf = |g: &G, o: u32| g.read::<f32>(at(o));
            let done = crate::s4_misc::motion_script(g, img, script, r);
            if done == 0 || g.read::<u32>(at(0xac)) == 0 {
                all = 0;
            }
            let delay = g.read::<u32>(at(0xa0)).cast_signed();
            if delay > 0 {
                g.write::<u32>(at(0xa0), (delay - 1).cast_unsigned());
            } else {
                let n = g.read::<u32>(at(0x9c)).cast_signed();
                let yaw = if n > 0 {
                    #[allow(clippy::cast_precision_loss)] // cvtdq2ps
                    let q = div(n as f32, c(g, 0xb4));
                    let a = mul(rf(g, 0x94), q);
                    let y = add(mul(sub(c(g, 0xac), q), rf(g, 0x98)), a);
                    let v40 = add(rf(g, 0x50), rf(g, 0x40));
                    g.write::<f32>(at(0x40), v40);
                    let v70 = add(rf(g, 0x70), v40);
                    g.write::<f32>(at(0x70), v70);
                    g.write::<u32>(at(0x9c), (n - 1).cast_unsigned());
                    let turn = add(rf(g, 0x90), rf(g, 0x8c));
                    let turn = wrap_once(g, img, f, turn, [0x95, 0x115, 0x127, 0x134]);
                    g.write::<f32>(at(0x8c), turn);
                    y
                } else {
                    rf(g, 0x98)
                };
                if g.read::<u32>(at(0xa8)) == 1 {
                    if g.read::<u32>(at(0xa4)) == 0 {
                        // call width: 1
                        let r1 = g.call(t(g, 0x16a, 404), 0, &[0x3c80_adfd]).st0;
                        let x = add(add(st0_f32(r1), c(g, 0x18a)), rf(g, 0x5c));
                        let x = wrap_once(g, img, f, x, [0x199, 0x1a3, 0x1ad, 0x1ba]);
                        g.write::<f32>(at(0x5c), x);
                        // call width: 1
                        let r2 = g.call(t(g, 0x1bf, 314), 0, &[0x3c80_adfd]).st0;
                        let y = add(add(st0_f32(r2), c(g, 0x1e4)), rf(g, 0x60));
                        let y = wrap_once(g, img, f, y, [0x1f3, 0x1fd, 0x20f, 0x21c]);
                        g.write::<f32>(at(0x60), y);
                    } else {
                        let x = add(rf(g, 0x5c), c(g, 0x233));
                        let x = wrap_once(g, img, f, x, [0x95, 0x243, 0x23b, 0x243]);
                        g.write::<f32>(at(0x5c), x);
                        let y = add(rf(g, 0x60), c(g, 0x269));
                        let y = wrap_once(g, img, f, y, [0x95, 0x243, 0x23b, 0x243]);
                        g.write::<f32>(at(0x60), y);
                    }
                } else {
                    for o in [0x5cu32, 0x60, 0x64] {
                        let x = add(rf(g, o), c(g, 0x299));
                        let x = wrap_once(g, img, f, x, [0x95, 0x2a9, 0x2a1, 0x2a9]);
                        g.write::<f32>(at(o), x);
                    }
                }
                // xorps with the sign mask at operand `+0x316`
                let neg = yaw.to_bits() ^ g.read::<u32>(w(0x316));
                g.write::<u32>(l.wrapping_add(8), neg);
                let mut st = Pushes::default();
                st.push(m);
                // call width: 12 (the most `Pushes::words` holds)
                st.call(g, t(g, 0x31f, 235), 0);
                st.set_top(rf(g, 0x8c).to_bits());
                st.push_all(&[m, m]);
                // call width: 12 (the most `Pushes::words` holds)
                st.call(g, t(g, 0x343, 329), 0);
                st.push_all(&[l, m, v]);
                // call width: 12 (the most `Pushes::words` holds)
                st.call(g, t(g, 0x35c, 229), 0);
                let mat = e.wrapping_sub(4);
                crate::sa_misc::actor_scale(g, img, scale, k, mat);
                st.pop(5);
                for (o, table, slot) in [
                    (0x64u32, 0x37bu32, 330u32),
                    (0x5c, 0x397, 328),
                    (0x60, 0x3b3, 329),
                ] {
                    st.set_top(rf(g, o).to_bits());
                    st.push_all(&[mat, mat]);
                    // call width: 12 (the most `Pushes::words` holds)
                    st.call(g, t(g, table, slot), 0);
                    st.pop(2);
                }
                // the rows `R + 0x20 + 0x10 j` times `E + 0x7c + 4 j`; the
                // middle row's second term is `mulss` the other way round
                for (j, by) in [(0u32, 0x7cu32), (1, 0x80), (2, 0x84)] {
                    let sc = rf(g, by);
                    for i in 0..3u32 {
                        let a = mat.wrapping_add(0x10 * j + 4 * i);
                        let x = g.read::<f32>(a);
                        let x = if (j, i) == (1, 1) {
                            mul(sc, x)
                        } else {
                            mul(x, sc)
                        };
                        g.write::<f32>(a, x);
                    }
                }
                if g.read::<u32>(at(0xa8)) == 1 {
                    for (i, d) in [(0u32, 0x46au32), (1, 0x47c), (2, 0x48e)] {
                        let x = add(
                            g.read::<f32>(s.wrapping_add(w(d))),
                            g.read::<f32>(v.wrapping_add(4 * i)),
                        );
                        g.write::<f32>(at(0x2c + 4 * i), x);
                    }
                    let x = g.read::<u32>(s.wrapping_add(w(0x49e)));
                    g.write::<u32>(at(0x38), x);
                } else {
                    let x = g.read::<u32>(at(0x6c));
                    g.write::<u32>(at(0x2c), x);
                    let x = g.read::<u32>(at(0x74));
                    g.write::<u32>(at(0x34), x);
                }
                let x = g.read::<u32>(at(0x70));
                g.write::<u32>(at(0x30), x);
            }
            k += 1;
            eax = k;
            r = r.wrapping_add(0xd4);
            if k.cast_signed() >= count(g) {
                break;
            }
        }
        if all != 1 {
            return Ret::eax(eax);
        }
    }
    g.write::<u32>(s.wrapping_add(w(0x4e9)), 1);
    Ret::eax(eax)
}

/// [`spin_pieces_0605`] (the `rel32`s: the motion script, the actor
/// scale, `__security_check_cookie`).
pub const SPIN_PIECES_0605: Sig = Sig {
    len: 1280,
    hash: 0x400c_7fa2_99a4_44f8,
    calls: &[
        (0x5a, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0x371, Callee::Sig(&crate::sa_misc::ACTOR_SCALE)),
        (0x4f8, Callee::Any),
    ],
    imports: &[],
};

/// [`spin_pieces_0605`] in `magic_0307` (0x10003770; family P015).
pub const SPIN_PIECES_0307: Sig = Sig {
    hash: 0x7e8c_0eef_e910_78b8,
    ..SPIN_PIECES_0605
};

/// `magic_0605`'s spinning pieces start (0x100087f0, 1086 bytes; a table
/// callback, no arguments; an aligned /GS frame). With `S = *G_S` and `n =
/// *G_N` (read once), for each record `k < n` (`R = S + 0x1850 + 0xd4 k`,
/// `P = R + 0x94`) and its actor's object `O = OBJS[LIST[k]]`:
/// - `R + 0x14` (the script's done word) 1; slot 262 `(O, &x, &y, &z)`
///   into a local vector (w 1.0); slot 12 `(O)`; `P = y`, then two slot
///   314 `(5.0)` (ST0; the second kept) less `C` plus `y`;
/// - the flat distance `d` from `S + 0x14a0` (the height difference
///   `S.y - S.y`), through `_libm_sse2_sqrt_precise`, to `P + 0x24`, `P +
///   0x28` (the largest kept); the yaw `K - atan2(dz, dx)` (`_CIatan2`,
///   wrapped once) to `P + 0x1c`, `P + 0x20` a constant, nine words from
///   `P - 0x34` 0; slot 333 `(O)`'s angle `K - a` wrapped to `P - 0x10`;
///   zeros, three scales 0.75 and the flags `P + 0x2c..0x3c`;
/// - the actor's mode ([`actor_mode`](crate::sa_script::actor_mode),
///   `ecx = k`, `edx = R + 0x20`) and slot 528 `(LIST[k], 0)`.
///
/// Then, when the largest distance is above `L`, each record's `P + 0x28`
/// times `L / d` (for `k < *G_N`, read again each round). A task node with
/// the callback [`spin_pieces_0605`] is taken from the free ring
/// ([`pool_take_0587`](crate::r27::pool_take_0587)) and goes last on the
/// task list, also kept at `S + 0x30b0`. Returns 0xffff or the list's old
/// tail.
#[allow(clippy::too_many_lines)] // the original's steps, one by one
pub fn spin_start_0605<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let c = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let rf = |g: &G, at: u32| g.read::<f32>(at);
    // push ebp; and esp, -8; sub esp, 0x44; three pushes
    let esp0 = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x44);
    let cookie = g.read::<u32>(w(0xa)) ^ esp0;
    g.write::<u32>(esp0 + 0x40, cookie);
    let b = esp0.wrapping_sub(0xc);
    let (px, py, pz) = (b + 0x3c, b + 0x40, b + 0x44);
    let s = g.read::<u32>(w(0x1c));
    let n = g.read::<u32>(w(0x15)).cast_signed();
    let mut far = 0.0f32;
    // `k` counts up from 0: the inlined "actor -1 is `*G_SELF`" branches
    // (operands `+0x4d`, `+0x91`, `+0x246`, `+0x302`) never run
    let obj = |g: &G, k: u32| {
        let i = u32::from(g.read::<u8>(g.read::<u32>(w(0x54)).wrapping_add(k)));
        g.read::<u32>(w(0x5f).wrapping_add(4 * i))
    };
    for k in 0..n.max(0).cast_unsigned() {
        let r = s.wrapping_add(0x1850 + 0xd4 * k);
        let p = r.wrapping_add(0x94);
        g.write::<u32>(r.wrapping_add(0x14), 1);
        let o = obj(g, k);
        // call width: 4
        g.call(t(g, 0x74, 262), 0, &[o, px, py, pz]);
        g.write::<u32>(b + 0x48, 0x3f80_0000);
        let o = obj(g, k);
        // call width: 1
        g.call(t(g, 0xa9, 12), 0, &[o]);
        let y = rf(g, py);
        g.write::<f32>(p, y);
        // call width: 1
        g.call(t(g, 0xbb, 314), 0, &[0x40a0_0000]);
        // call width: 1
        let r5 = st0_f32(g.call(t(g, 0xd5, 314), 0, &[0x40a0_0000]).st0);
        let x = rf(g, px);
        g.write::<f32>(p, add(sub(r5, c(g, 0xf6)), rf(g, py)));
        let sy = rf(g, s.wrapping_add(0x14a4));
        let dx = sub(x, rf(g, s.wrapping_add(0x14a0)));
        let z = rf(g, pz);
        g.write::<f32>(py, sy);
        let dy = sub(sy, sy);
        let dz = sub(z, rf(g, s.wrapping_add(0x14a8)));
        let flat = add(add(mul(dy, dy), mul(dx, dx)), mul(dz, dz));
        // `_libm_sse2_sqrt_precise` of the double: exact, as Rust's
        #[allow(clippy::cast_possible_truncation)]
        let d = f64::from(flat).sqrt() as f32;
        if d > far {
            far = d;
        }
        g.write::<f32>(p + 0x24, d);
        g.write::<f32>(p + 0x28, d);
        let dz = sub(z, rf(g, s.wrapping_add(0x14a8)));
        let dx = sub(x, rf(g, s.wrapping_add(0x14a0)));
        // `_CIatan2` on x87 (`ST1` = dz), rounded once
        #[allow(clippy::cast_possible_truncation)]
        let a = g.atan2(f64::from(dz), f64::from(dx)) as f32;
        let yaw = sub(c(g, 0x1c1), a);
        let yaw = wrap_once(g, img, f, yaw, [0x1d0, 0x1da, 0x1e4, 0x1f1]);
        g.write::<f32>(p + 0x1c, yaw);
        g.write::<u32>(p + 0x20, w(0x1fd));
        for i in 0..9u32 {
            g.write::<u32>(p.wrapping_sub(0x34 - 4 * i), 0);
        }
        let o = obj(g, k);
        // call width: 1
        let v = st0_f32(g.call(t(g, 0x25e, 333), 0, &[o]).st0);
        let a2 = sub(c(g, 0x272), v);
        let a2 = wrap_once(g, img, f, a2, [0x282, 0x28c, 0x296, 0x2a3]);
        g.write::<f32>(p.wrapping_sub(0x10), a2);
        g.write::<u32>(p.wrapping_sub(0xc), 0);
        g.write::<u32>(p.wrapping_sub(8), 0);
        for (o, at) in [(0xcu32, 0x2bdu32), (0x10, 0x2c4), (0x14, 0x2cb)] {
            g.write::<u32>(p + o, w(at));
        }
        for (o, v) in [(0x2cu32, 0u32), (0x30, 0), (0x34, 1), (0x38, 1), (0x3c, 0)] {
            g.write::<u32>(p + o, v);
        }
        let mode = Frame {
            at: img.rel32_target(f + 0x2f8),
            esp: b.wrapping_sub(4),
            ecx: k,
            edx: r.wrapping_add(0x20),
            ..*fr
        };
        crate::sa_script::actor_mode(g, img, &mode);
        let i = u32::from(g.read::<u8>(g.read::<u32>(w(0x309)).wrapping_add(k)));
        // call width: 2
        g.call(t(g, 0x315, 528), 0, &[i, 0]);
    }
    let lim = c(g, 0x339);
    if n > 0 && far > lim {
        let q = div(lim, far);
        let mut i = 0u32;
        while i.cast_signed() < g.read::<u32>(w(0x378)).cast_signed() {
            let at = s.wrapping_add(0x190c + 0xd4 * i);
            let x = mul(q, rf(g, at));
            g.write::<f32>(at, x);
            i += 1;
        }
    }
    let (node, eax) =
        crate::r27::pool_take_0587(g, img, f, [0x381, 0x390, 0x3ad, 0x3b5, 0x3e6, 0x3f1], 0);
    let eax = append_last(g, img, f, node, [0x3f7, 0x406, 0x419, 0x431], eax);
    g.write::<u32>(s.wrapping_add(0x30b0), node);
    Ret::eax(eax)
}

/// [`spin_start_0605`] (the `rel32`s: `_libm_sse2_sqrt_precise`,
/// `_CIatan2`, the actor's mode, `__security_check_cookie`).
pub const SPIN_START_0605: Sig = Sig {
    len: 1086,
    hash: 0xd0b3_a0ae_9e87_dce3,
    calls: &[
        (0x159, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x1af, Callee::Import("_CIatan2")),
        (0x2f8, Callee::Sig(&crate::sa_script::ACTOR_MODE)),
        (0x436, Callee::Any),
    ],
    imports: &[],
};

/// `node` goes last on the task list as the overlays inline it (operands
/// in `f`'s code: the head `head` tested, the tail `tail` read, the head's
/// store `head_w` when the list was empty, the tail's store `tail_w`).
/// Returns `eax` after it: the old tail, or `eax` as it was.
pub(crate) fn append_last<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    node: u32,
    [head, tail, head_w, tail_w]: [u32; 4],
    eax: u32,
) -> u32 {
    let w = |o: u32| img.word(f + o);
    let listed = g.read::<u32>(w(head)) != 0;
    g.write::<u32>(node.wrapping_add(4), 0);
    let mut eax = eax;
    if listed {
        let last = g.read::<u32>(w(tail));
        g.write::<u32>(node, last);
        g.write::<u32>(last.wrapping_add(4), node);
        eax = last;
    } else {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(head_w), node);
    }
    g.write::<u32>(node.wrapping_add(8), node);
    g.write::<u32>(w(tail_w), node);
    eax
}

/// `magic_0605`'s actors' set-up for the spin (0x1000a540, 1081 bytes; a
/// table callback, no arguments; a /GS frame). With `S = *G_S` and `n =
/// *G_N` (read once), for each record `k < n` (`R = S + 0x1850 + 0xd4 k`,
/// `Q = R + 0x84`), its actor's object `O = OBJS[LIST[k]]`, the object `B
/// = S + 0x850 + 0x130 k` and the shake block `H = S + 0x2bf4 + 0x30 k`:
/// - `R + 0x14` 1; slot 262 `(O, &x, &y, &z)`, slot 12 `(O)` (ST0 `h`);
///   `Q + 0xc..0x14` = `(x, Q[0x10] - C, z)`, three words 0 and the scales
///   1.0; the record's scale on `R + 0x20` ([`actor_scale`](crate::sa_misc::actor_scale),
///   `ecx = k`), slots 330 / 328 / 329 by `Q + 4`, `Q - 4`, `Q`;
/// - `Q - 0x34..` the place again, zeros, `Q - 0x20 = -(Q[0x10] - h) K`;
///   `Q + 0x40 = v` (`v` 0x20, then up by `1 + rand() % 10` each
///   record), `Q + 0x3c` 0x10;
/// - `B` starts ([`start_object_at`](crate::s4_misc::start_object_at),
///   part 5, owner `v + 0x10`, the actor-part callback, scale `K2`), placed
///   at `(x, h - C2, z, 1.0)`, `B + 0x120 = k`;
/// - `H` set up and a task node (the shake task, `+0x14 = H`, flag 1)
///   taken from the free ring and put last on the task list.
///
/// Then outside a battle (`*G_A` 0) the caster's colour is read and put
/// back for an actor `*G_B` below 0x20 (slots 217, 255), slot 211
/// `(*G_SELF, 0)`; slot 213 `(*G_D, 0)` unless `*G_C`; `*G_E = 1` and the
/// others shown ([`show_others`](crate::sb_vm::show_others), 1), whose
/// `eax` it returns.
#[allow(clippy::too_many_lines)] // the original's steps, one by one
pub fn spin_actors_0605<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let c = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let rf = |g: &G, at: u32| g.read::<f32>(at);
    let ebp = fr.esp.wrapping_sub(4);
    let cookie = g.read::<u32>(w(0x7)) ^ ebp;
    g.write::<u32>(ebp.wrapping_sub(4), cookie);
    let (px, py, pz, vec) = (
        ebp.wrapping_sub(0x14),
        ebp.wrapping_sub(0x10),
        ebp.wrapping_sub(0xc),
        ebp.wrapping_sub(0x24),
    );
    let s = g.read::<u32>(w(0x13));
    let n = g.read::<u32>(w(0x23)).cast_signed();
    let mut v = 0x20u32;
    // `k` counts up from 0: the inlined "actor -1 is `*G_SELF`" branch
    // (operand `+0x5d`) never runs
    for k in 0..n.max(0).cast_unsigned() {
        let r = s.wrapping_add(0x1850 + 0xd4 * k);
        let q = r.wrapping_add(0x84);
        let b = s.wrapping_add(0x850 + 0x130 * k);
        let h = s.wrapping_add(0x2bf4 + 0x30 * k);
        g.write::<u32>(r.wrapping_add(0x14), 1);
        let i = u32::from(g.read::<u8>(g.read::<u32>(w(0x64)).wrapping_add(k)));
        let o = g.read::<u32>(w(0x6f).wrapping_add(4 * i));
        let mut st = Pushes::default();
        st.push_all(&[pz, py, px, o]);
        // call width: 12 (the most `Pushes::words` holds)
        st.call(g, t(g, 0x84, 262), 0);
        st.push(o);
        // call width: 12 (the most `Pushes::words` holds)
        let hy = st0_f32(st.call(g, t(g, 0x91, 12), 0).st0);
        let x = rf(g, px);
        g.write::<f32>(q + 0xc, x);
        let y = sub(rf(g, q + 0x10), c(g, 0xb6));
        for o in [0xffff_fffcu32, 0, 4] {
            g.write::<u32>(q.wrapping_add(o), 0);
        }
        g.write::<u32>(q + 0x1c, w(0xd1));
        g.write::<f32>(q + 0x10, y);
        let z = rf(g, pz);
        g.write::<f32>(q + 0x14, z);
        g.write::<u32>(q + 0x20, w(0xe7));
        g.write::<u32>(q + 0x24, w(0xee));
        let one = w(0xfa);
        let mat = r.wrapping_add(0x20);
        crate::sa_misc::actor_scale(g, img, img.rel32_target(f + 0xff), k, mat);
        st.pop(4);
        for (o, table, slot) in [
            (4u32, 0x10fu32, 330u32),
            (0xffff_fffc, 0x12e, 328),
            (0, 0x14c, 329),
        ] {
            st.set_top(rf(g, q.wrapping_add(o)).to_bits());
            st.push_all(&[mat, mat]);
            // call width: 12 (the most `Pushes::words` holds)
            st.call(g, t(g, table, slot), 0);
            st.pop(2);
        }
        for (dst, src) in [(0x34u32, 0xcu32), (0x30, 0x10), (0x2c, 0x14)] {
            let x = g.read::<u32>(q + src);
            g.write::<u32>(q.wrapping_sub(dst), x);
        }
        g.write::<u32>(q + 0x34, 0);
        g.write::<u32>(q + 0x38, 0);
        let d = sub(rf(g, q + 0x10), hy);
        for o in [0x24u32, 0x1c, 0x18, 0x14] {
            g.write::<u32>(q.wrapping_sub(o), 0);
        }
        let d = mul(d, c(g, 0x1af));
        g.write::<u32>(q.wrapping_sub(0x10), 0);
        // xorps with the sign mask at operand `+0x1bd`
        let d = d.to_bits() ^ g.read::<u32>(w(0x1bd));
        g.write::<u32>(q.wrapping_sub(0x20), d);
        let scale = c(g, 0x1cd);
        g.write::<u32>(q.wrapping_sub(0xc), 0);
        g.write::<u32>(q.wrapping_sub(8), 0);
        g.write::<u32>(q + 0x40, v);
        let owner = v.wrapping_add(0x10);
        g.write::<u32>(q + 0x3c, 0x10);
        g.write::<u32>(q + 0x48, 0);
        g.write::<u32>(q + 0x4c, 0);
        let so = img.rel32_target(f + 0x20a);
        crate::s4_misc::start_object_at(g, img, so, b, scale, 5, owner, w(0x1e3));
        g.write::<f32>(b + 0xd0, rf(g, px));
        g.write::<f32>(b + 0xd8, rf(g, pz));
        g.write::<u32>(b + 0xdc, one);
        g.write::<f32>(b + 0xd4, sub(hy, c(g, 0x244)));
        g.write::<u32>(b + 0x120, k);
        g.write::<u32>(h, 0);
        g.write::<u32>(h + 0x1c, 0);
        g.write::<u32>(h + 0x20, w(0x26e));
        g.write::<u32>(h + 0x24, 0);
        g.write::<u32>(h + 0x28, 10);
        g.write::<u16>(h + 0x2c, 0);
        let (node, eax) =
            crate::r27::pool_take_0587(g, img, f, [0x287, 0x293, 0x2b7, 0x2bf, 0x2f0, 0x2fb], 0);
        g.write::<u8>(node.wrapping_add(0x18), 1);
        g.write::<u32>(node.wrapping_add(0x14), h);
        append_last(g, img, f, node, [0x301, 0x317, 0x32a, 0x368], eax);
        for (o, at) in [
            (4u32, 0x337u32),
            (8, 0x33e),
            (0xc, 0x345),
            (0x10, 0x34c),
            (0x14, 0x353),
            (0x18, 0x35a),
        ] {
            g.write::<u32>(h + o, w(at));
        }
        let fl = g.read::<u16>(h + 0x2c);
        g.write::<u16>(h + 0x2c, fl | 3);
        g.write::<u32>(h + 0x24, owner);
        let r = g.rand().cast_signed();
        v = v.wrapping_add(1).wrapping_add((r % 10).cast_unsigned());
    }
    let me = g.read::<u32>(w(0x3a8));
    if g.read::<u32>(w(0x3a1)) == 0 {
        let a = g.read::<u32>(w(0x3b0));
        if a.cast_signed() < 0x20 {
            // call width: 2
            g.call(t(g, 0x3be, 217), 0, &[a, vec]);
            // call width: 4 (slot 217's words stay on the stack)
            g.call(t(g, 0x3d0, 255), 0, &[a, vec, a, vec]);
        }
    }
    // call width: 2
    g.call(t(g, 0x3e1, 211), 0, &[me, 0]);
    if g.read::<u16>(w(0x3f6)) == 0 {
        let x = g.read::<u32>(w(0x408));
        // call width: 2
        g.call(t(g, 0x400, 213), 0, &[x, 0]);
    }
    g.write::<u32>(w(0x41e), 1);
    let show = img.rel32_target(f + 0x427);
    let eax = crate::sb_vm::show_others(g, img, show, ebp.wrapping_sub(0x40), 1);
    Ret::eax(eax)
}

/// [`spin_actors_0605`] (the `rel32`s: the actor scale, the object start,
/// the others shown, `__security_check_cookie`; `rand`).
pub const SPIN_ACTORS_0605: Sig = Sig {
    len: 1081,
    hash: 0x6210_6b9a_65ef_4e00,
    calls: &[
        (0xff, Callee::Sig(&crate::sa_misc::ACTOR_SCALE)),
        (0x20a, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x427, Callee::Sig(&crate::sb_vm::SHOW_OTHERS)),
        (0x431, Callee::Any),
    ],
    imports: &[(0x371, "rand")],
};

/// One of [`sparks_0605`]'s two records: the operand offsets in the code
/// (a slot's table operand, then the `mov [esp], imm32` value's), the
/// slot of `+0x14` and what differs at the end.
struct Spark {
    take: u32,
    s719: (u32, u32, u32, u32),
    s235: u32,
    turn: (u32, u32),
    s329: u32,
    far: (u32, u32),
    near: (u32, u32),
    s229: u32,
    x: (u32, u32),
    y: (u32, u32, u32, u32),
    z: (u32, u32),
    vx: (u32, u32, u32),
    w14: (u32, u32, u16),
    vz: (u32, u32, u32),
    yaw: (u32, u32),
    words: (u32, u32),
    size: (u32, u32, u32, u32),
    second: bool,
}

/// `magic_0605`'s sparks (0x1000a980, 1377 bytes; a table callback, no
/// arguments; a /GS frame): the actors' mean `P`
/// ([`mean_position`](crate::sb_misc::mean_position), its height then 0),
/// then three times two records of the 0x74-byte pool
/// ([`pool_take`](crate::sb_misc::pool_take); the first that fails ends
/// it, `eax` 0). Each gets slot 719 `(NAME, 9, flag)` at `+0x30` (flag 1,
/// then 0), frame -1, time 0x200, playing; a unit `M` (slot 235) turned by
/// slot 314 `(pi)` (slot 329); `V` = slot 229 `(V, M, (0, 0, a + b, 1))`
/// with `a`, `b` slot 314 `(100)`, `(50)` (ST0, each rounded to f32);
/// the place `P` with `x + r(50)`, `y = S[0x14b4] - r404(10) + C - C'`,
/// `z + r(50)`; the speed `(V.x K + r, r', V.z K + r'')`, spin `r(pi)`;
/// colour words and the size `(r + C) K`; the second record also an
/// alpha byte, 0x2828 and `+0x48 = r`. Returns slot 314's last `eax`.
#[allow(clippy::too_many_lines)] // the original's steps, one by one
pub fn sparks_0605<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let c = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let b8 = |o: u32| w(o) & 0xff;
    let ebp = fr.esp.wrapping_sub(4);
    let cookie = g.read::<u32>(w(0x7)) ^ ebp;
    g.write::<u32>(ebp.wrapping_sub(4), cookie);
    let s = g.read::<u32>(w(0x15));
    let (out, vec, m, v) = (
        ebp.wrapping_sub(0x24),
        ebp.wrapping_sub(0x14),
        ebp.wrapping_sub(0x74),
        ebp.wrapping_sub(0x34),
    );
    g.write::<u32>(vec, 0);
    g.write::<u32>(vec + 4, 0);
    g.write::<u32>(vec + 0xc, w(0x2d));
    let mean = img.rel32_target(f + 0x32);
    crate::sb_misc::mean_position(g, img, mean, ebp.wrapping_sub(0x8c), out);
    g.write::<u32>(out + 4, 0);
    let halves = [
        Spark {
            take: 0x41,
            s719: (0x51, 0x60, 0x5e, 0x56),
            s235: 0x7e,
            turn: (0x92, 0xa2),
            s329: 0xbb,
            far: (0xc8, 0xd8),
            near: (0xe2, 0xe9),
            s229: 0x10f,
            x: (0x126, 0x144),
            y: (0x157, 0x15e, 0x18e, 0x196),
            z: (0x17f, 0x186),
            vx: (0x1b5, 0x1bc, 0x1d9),
            w14: (0x1de, 0x1e5, 404),
            vz: (0x1ff, 0x206, 0x21e),
            yaw: (0x223, 0x22a),
            words: (0x252, 0x260),
            size: (0x244, 0x26d, 0x27f, 0x29f),
            second: false,
        },
        Spark {
            take: 0x2be,
            s719: (0x2ce, 0x2dd, 0x2db, 0x2d3),
            s235: 0x2fb,
            turn: (0x30f, 0x31f),
            s329: 0x338,
            far: (0x345, 0x355),
            near: (0x35f, 0x366),
            s229: 0x38c,
            x: (0x3a3, 0x3c1),
            y: (0x3d4, 0x3db, 0x40b, 0x413),
            z: (0x3fc, 0x403),
            vx: (0x432, 0x439, 0x456),
            w14: (0x45b, 0x462, 314),
            vz: (0x47c, 0x483, 0x49b),
            yaw: (0x4a0, 0x4a7),
            words: (0x4cf, 0x4dd),
            size: (0x4c1, 0x4ea, 0x4fc, 0x51d),
            second: true,
        },
    ];
    // a slot answering in ST0, its one word the immediate at `imm`
    let st0 = |g: &mut G, (table, imm): (u32, u32), slot: u32| {
        // call width: 1
        let out = g.call(t(g, table, slot), 0, &[w(imm)]);
        (st0_f32(out.st0), out.eax)
    };
    let mut eax = 0;
    for _ in 0..3 {
        for h in &halves {
            let p = crate::sb_misc::pool_take(g, img, img.rel32_target(f + h.take));
            if p == 0 {
                return Ret::eax(0);
            }
            let (table, name, nine, flag) = h.s719;
            // call width: 3
            let part = g
                .call(t(g, table, 719), 0, &[w(name), b8(nine), b8(flag)])
                .eax;
            g.write::<u32>(p + 0x30, part);
            g.write::<u16>(p + 0x38, 0xffff);
            g.write::<u16>(p + 0x3a, 0x200);
            g.write::<u32>(p + 0x34, 1);
            // call width: 1
            g.call(t(g, h.s235, 235), 0, &[m]);
            let (a, _) = st0(g, h.turn, 314);
            // call width: 3
            g.call(t(g, h.s329, 329), 0, &[m, m, a.to_bits()]);
            let (far, _) = st0(g, h.far, 314);
            let (near, _) = st0(g, h.near, 314);
            g.write::<f32>(vec + 8, add(near, far));
            // call width: 3
            g.call(t(g, h.s229, 229), 0, &[v, m, vec]);
            for o in [0u32, 4, 8, 0xc] {
                let x = g.read::<u32>(out + o);
                g.write::<u32>(p + o, x);
            }
            let (r, _) = st0(g, h.x, 314);
            let x = add(r, g.read::<f32>(p));
            g.write::<f32>(p, x);
            let (yt, yi, c1, c2) = h.y;
            let (r, _) = st0(g, (yt, yi), 404);
            let y = sub(
                add(sub(g.read::<f32>(s.wrapping_add(0x14b4)), r), c(g, c1)),
                c(g, c2),
            );
            g.write::<f32>(p + 4, y);
            let (r, _) = st0(g, h.z, 314);
            let z = add(g.read::<f32>(p + 8), r);
            g.write::<f32>(p + 8, z);
            let (vt, vi, k) = h.vx;
            let (r, _) = st0(g, (vt, vi), 314);
            let sx = add(mul(g.read::<f32>(v), c(g, k)), r);
            g.write::<f32>(p + 0x10, sx);
            let (wt, wi, slot) = h.w14;
            let (r, _) = st0(g, (wt, wi), u32::from(slot));
            g.write::<f32>(p + 0x14, r);
            let (zt, zi, k) = h.vz;
            let (r, _) = st0(g, (zt, zi), 314);
            let sz = add(mul(g.read::<f32>(v + 8), c(g, k)), r);
            g.write::<f32>(p + 0x18, sz);
            let (r, _) = st0(g, h.yaw, 314);
            g.write::<f32>(p + 0x1c, r);
            g.write::<u32>(p + 0x20, 0);
            g.write::<u32>(p + 0x24, w(h.words.0));
            g.write::<u32>(p + 0x28, 0);
            g.write::<u32>(p + 0x2c, w(h.words.1));
            let (st, si, ca, cm) = h.size;
            let (r, e) = st0(g, (st, si), 314);
            eax = e;
            let size = mul(add(r, c(g, ca)), c(g, cm));
            if h.second {
                g.write::<u32>(p + 0x4c, 0x50);
                g.write::<u32>(p + 0x50, 0);
                g.write::<u16>(p + 0x68, 0x2828);
                g.write::<u8>(p + 0x6a, 0x28);
                g.write::<u32>(p + 0x60, 0);
                g.write::<f32>(p + 0x3c, size);
                let (r, e) = st0(g, (0x501, 0x53a), 314);
                eax = e;
                g.write::<f32>(p + 0x48, r);
            } else {
                g.write::<u32>(p + 0x40, 0);
                g.write::<u32>(p + 0x58, 0x62);
                g.write::<u32>(p + 0x5c, 0);
                g.write::<u32>(p + 0x4c, 0x62);
                g.write::<u32>(p + 0x50, 0);
                g.write::<f32>(p + 0x3c, size);
                g.write::<u32>(p + 0x60, 0);
            }
        }
    }
    Ret::eax(eax)
}

/// [`sparks_0605`] (the `rel32`s: the mean position, the pool take twice,
/// `__security_check_cookie`).
pub const SPARKS_0605: Sig = Sig {
    len: 1377,
    hash: 0xb936_39da_c3ea_9c33,
    calls: &[
        (0x32, Callee::Sig(&crate::sb_misc::MEAN_POSITION)),
        (0x41, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
        (0x2be, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
        (0x559, Callee::Any),
    ],
    imports: &[],
};

/// The cursor visit's opening as `magic_0577`'s visits inline it (`ecx` =
/// the cursor `C`): when the node `N = *C` is marked already (`N + 0x1a`
/// = `C + 5`) `C + 4` is cleared and `None` comes back (the low byte of
/// `eax` the mark); else each child (from `N + 0x10`, along `+8`) is
/// visited (`C = child`, its `[[C] + 0x1c](C)`), then `C = N`, `N` marked
/// and `C + 4` set. Gives `N`.
pub(crate) fn visit_children<G: Game>(g: &mut G, c: u32) -> Result<u32, u8> {
    let node = g.read::<u32>(c);
    let mark = g.read::<u8>(node.wrapping_add(0x1a));
    if mark == g.read::<u8>(c.wrapping_add(5)) {
        g.write::<u8>(c.wrapping_add(4), 0);
        return Err(mark);
    }
    let mut child = g.read::<u32>(node.wrapping_add(0x10));
    while child != node {
        g.write::<u32>(c, child);
        let cur = g.read::<u32>(c);
        child = g.read::<u32>(child.wrapping_add(8));
        let visit = g.read::<u32>(g.read::<u32>(cur).wrapping_add(0x1c));
        // call width: 1 (the visits read their cursor from the stack)
        g.call(visit, c, &[c]);
    }
    let byte5 = g.read::<u8>(c.wrapping_add(5));
    g.write::<u32>(c, node);
    g.write::<u8>(node.wrapping_add(0x1a), byte5);
    g.write::<u8>(c.wrapping_add(4), 1);
    Ok(g.read::<u32>(c))
}

/// `magic_0577`'s node visit (0x10001fc0, 1202 bytes; internal, `ecx` =
/// the cursor, reached by its 12-byte thunk 0x100025e0; a /GS frame).
/// After the children ([`visit_children`]), with the node `N` and its rate
/// `r = N + 0xcc`:
/// - flag bit 0 of `N + 0xe4`: the place `N + 0x1c..0x28` and the colour
///   `N + 0x4c..0x58` ease towards `N + 0x2c..`, `N + 0x5c..` by `r`
///   (`x + (to - x) r`, interleaved), the fade `N + 0xd0` towards `K`;
/// - else the fade `N + 0xd0` grows by `r`: at `K` or above it is 1.0 and
///   the place and colour are copied from their targets, below it they
///   move by their speeds `N + 0x3c..`, `N + 0x6c..` times `r`;
/// - slot 353 `(N + 0x1c)`, slot 332 with the integer place `(N + 0xbc..
///   as float) x place` (truncated), slot 669 with the colour times `N +
///   0x7c..` (truncated);
/// - bit 3: the scale `N + 0xdc + N + 0xe0` up to `K` (bit 4: then bit 3
///   off) into `N + 0xdc`, slot 332 with `N + 0x8c.. + N + 0x9c.. s`;
/// - bit 1: when `N + 0xd8` is above the place's three axes slot 267 `(1)`
///   and bit 1 off; without it, when no axis is below `N + 0xd8` slot 271
///   `(1)` and bit 1 on;
/// - bit 2 and the fade at `N + 0xd4` or above: the node detached and
///   drawn ([`draw_detached`](crate::m0577::draw_detached)).
///
/// Returns the last call's `eax` (the flags word after slot 669 when
/// nothing follows; the mark in the low byte with the cookie's bits above
/// when the node was marked).
#[allow(clippy::too_many_lines)] // the original's branches, one by one
pub fn visit_0577<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let ebp = fr.esp.wrapping_sub(4);
    let cookie = g.read::<u32>(w(0x7)) ^ ebp;
    g.write::<u32>(ebp.wrapping_sub(4), cookie);
    let n = match visit_children(g, fr.ecx) {
        Ok(n) => n,
        Err(mark) => return Ret::eax((cookie & !0xff) | u32::from(mark)),
    };
    let at = |o: u32| n.wrapping_add(o);
    let rf = |g: &G, o: u32| g.read::<f32>(at(o));
    let flags = |g: &G| g.read::<u32>(at(0xe4));
    // (place or colour word, its target, its speed)
    let axes = [
        (0x1cu32, 0x2cu32, 0x3cu32),
        (0x4c, 0x5c, 0x6c),
        (0x20, 0x30, 0x40),
        (0x50, 0x60, 0x70),
        (0x24, 0x34, 0x44),
        (0x54, 0x64, 0x74),
        (0x28, 0x38, 0x48),
        (0x58, 0x68, 0x78),
    ];
    if flags(g) & 1 != 0 {
        for (x, to, _) in axes {
            let v = add(mul(sub(rf(g, to), rf(g, x)), rf(g, 0xcc)), rf(g, x));
            g.write::<f32>(at(x), v);
        }
        let fade = rf(g, 0xd0);
        let v = add(mul(sub(k(g, 0x157), fade), rf(g, 0xcc)), fade);
        g.write::<f32>(at(0xd0), v);
    } else {
        let r = rf(g, 0xcc);
        let fade = add(r, rf(g, 0xd0));
        g.write::<f32>(at(0xd0), fade);
        // comiss; jb: below or unordered keeps moving
        if fade
            .partial_cmp(&k(g, 0x18e))
            .is_none_or(core::cmp::Ordering::is_lt)
        {
            for (x, _, v) in axes {
                let y = add(mul(rf(g, v), r), rf(g, x));
                g.write::<f32>(at(x), y);
            }
        } else {
            g.write::<u32>(at(0xd0), w(0x1a2));
            for (x, to, _) in axes {
                let v = g.read::<u32>(at(to));
                g.write::<u32>(at(x), v);
            }
        }
    }
    let (v, place) = (ebp.wrapping_sub(0x14), at(0x1c));
    let mut st = Pushes::default();
    st.push(place);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, t(g, 0x28a, 353), 0);
    for i in 0..4u32 {
        #[allow(clippy::cast_precision_loss)] // cvtdq2ps
        let n_i = g.read::<u32>(at(0xbc + 4 * i)).cast_signed() as f32;
        let x = crate::sb_sse::cvtt(mul(n_i, rf(g, 0x1c + 4 * i)));
        g.write::<u32>(v + 4 * i, x.cast_unsigned());
    }
    st.push(v);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, t(g, 0x2f7, 332), 0);
    for i in 0..4u32 {
        let x = crate::sb_sse::cvtt(mul(rf(g, 0x7c + 4 * i), rf(g, 0x4c + 4 * i)));
        g.write::<u32>(v + 4 * i, x.cast_unsigned());
    }
    // call width: 1
    g.call(t(g, 0x32f, 669), 0, &[v]);
    let mut eax = flags(g);
    if eax & 8 != 0 {
        let mut s = add(rf(g, 0xe0), rf(g, 0xdc));
        let top = k(g, 0x360);
        // comiss; jb: below or unordered keeps the sum
        if !s.partial_cmp(&top).is_none_or(core::cmp::Ordering::is_lt) {
            s = top;
            if eax & 0x10 != 0 {
                g.write::<u32>(at(0xe4), eax & !8);
            }
        }
        g.write::<f32>(at(0xdc), s);
        for i in 0..4u32 {
            let x = add(mul(rf(g, 0x9c + 4 * i), s), rf(g, 0x8c + 4 * i));
            g.write::<u32>(v + 4 * i, crate::sb_sse::cvtt(x).cast_unsigned());
        }
        // call width: 1
        eax = g.call(t(g, 0x3f2, 332), 0, &[v]).eax;
    }
    let top = rf(g, 0xd8);
    let axes3 = |g: &G| [rf(g, 0x1c), rf(g, 0x20), rf(g, 0x24)];
    if flags(g) & 2 != 0 {
        // comiss top, axis; jbe: not above (or unordered) stops
        if axes3(g).iter().all(|&a| top > a) {
            // call width: 1
            eax = g.call(t(g, 0x424, 267), 0, &[1]).eax;
            let fl = flags(g);
            g.write::<u32>(at(0xe4), fl & !2);
        }
    } else if axes3(g).iter().all(|&a| a >= top) {
        // comiss axis, top; jb: below (or unordered) stops
        // call width: 1
        eax = g.call(t(g, 0x464, 271), 0, &[1]).eax;
        let fl = flags(g);
        g.write::<u32>(at(0xe4), fl | 2);
    }
    if flags(g) & 4 != 0 && rf(g, 0xd0) >= rf(g, 0xd4) {
        let holder = ebp.wrapping_sub(0x18);
        g.write::<u32>(holder, n);
        let draw = Frame {
            at: img.rel32_target(f + 0x49d),
            esp: ebp.wrapping_sub(0x28),
            ecx: holder,
            ..*fr
        };
        eax = crate::m0577::draw_detached(g, img, &draw);
    }
    Ret::eax(eax)
}

/// [`visit_0577`] (the `rel32`s: the detached draw,
/// `__security_check_cookie`).
pub const VISIT_0577: Sig = Sig {
    len: 1202,
    hash: 0xf509_8da9_1876_0a67,
    calls: &[
        (0x49d, Callee::Sig(&crate::m0577::DRAW_DETACHED)),
        (0x4aa, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0577`'s light visit (0x100038d0, 1292 bytes; internal, `ecx` =
/// the cursor, reached by its 12-byte thunk 0x10003de0; a /GS frame).
/// After the children ([`visit_children`]), with the node `N`:
/// - the parts `N + 0x124` and `N + 0x128` (when set) are visited through
///   local cursors (`ebp - 0x5c`, `ebp - 0xc`, the cursor's mark byte) and
///   the nodes they end on give `N + 0x2c..` and `N + 0x4c..` (four words
///   from `+0x1c`);
/// - mode byte `N + 0x136`: 0 eases `N + 0x1c..0x24` towards `N + 0x2c..`
///   (`(x - to) N[0x5c] + to`), `+0x28` copied; 1 counts the time `N +
///   0x11c` down by `K` (to 0), slot 233 `(M, N + 0x10c, t / N[0x120])`,
///   slot 234 `(N + 0x1c, N + 0xfc, M)` and `N + 0x28` 1.0;
/// - unless byte `N + 0x135`: `N + 0x3c..0x44` eased the same way towards
///   `N + 0x4c..` by `N + 0x60`, `+0x48` copied;
/// - without the halfword `N + 0x98`: `N + 0xe8 *= N + 0xf8`, `N + 0x64 =
///   N[0xd8] N[0xe8] + N[0xc8]`;
/// - mode byte `N + 0x134`: 0 scales the spins `N + 0x6c..0x74` by `N +
///   0x7c..`; 1 takes `N + 0x7c..` off them and stops each at 0 when it
///   crosses it;
/// - the angles `(+-d - (d x) as int)` (`d` the i16 `N + 0x8c..`, the sign
///   by the spin's), `N + 0x78 = a2 K K' + N[0x88]` (the third angle then
///   0), the rotation `M` ([`rotation_4250`](crate::s3_matrix::rotation_4250));
/// - flags `N + 0x12c` bits 3 and 0: `N + 0x4c..0x54 = N + 0x1c.. - M`'s
///   columns times `N + 0x9c..0xa4`, `+0x58` from `+0xa8`;
/// - slot 284 `(L, 1, N + 0x3c)`, `(L, 0, N + 0x1c)`, 285 `(L, N + 0x78)`,
///   286 `(L, N + 0x64)` (`L = N + 0x130`).
///
/// Returns slot 286's `eax` (the mark in the low byte with the cookie's
/// bits above when the node was marked).
#[allow(clippy::too_many_lines)] // the original's branches, one by one
pub fn light_visit_0577<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let ebp = fr.esp.wrapping_sub(4);
    let cookie = g.read::<u32>(w(0x7)) ^ ebp;
    g.write::<u32>(ebp.wrapping_sub(4), cookie);
    let c = fr.ecx;
    let n = match visit_children(g, c) {
        Ok(n) => n,
        Err(mark) => return Ret::eax((cookie & !0xff) | u32::from(mark)),
    };
    let at = |o: u32| n.wrapping_add(o);
    let rf = |g: &G, o: u32| g.read::<f32>(at(o));
    for (part, cur, to) in [
        (0x124u32, ebp.wrapping_sub(0x5c), 0x2cu32),
        (0x128, ebp.wrapping_sub(0xc), 0x4c),
    ] {
        let p = g.read::<u32>(at(part));
        if p == 0 {
            continue;
        }
        let mark = g.read::<u8>(c.wrapping_add(5));
        g.write::<u32>(cur, p);
        g.write::<u8>(cur + 5, mark);
        let visit = g.read::<u32>(g.read::<u32>(p).wrapping_add(0x1c));
        // call width: 1 (the visits read their cursor from the stack)
        g.call(visit, cur, &[cur]);
        let end = g.read::<u32>(cur);
        for i in 0..4u32 {
            let x = g.read::<u32>(end.wrapping_add(0x1c + 4 * i));
            g.write::<u32>(at(to + 4 * i), x);
        }
    }
    let m = ebp.wrapping_sub(0x54);
    match g.read::<u8>(at(0x136)) {
        0 => {
            for (x, to) in [(0x1cu32, 0x2cu32), (0x20, 0x30), (0x24, 0x34)] {
                let v = add(mul(sub(rf(g, x), rf(g, to)), rf(g, 0x5c)), rf(g, to));
                g.write::<f32>(at(x), v);
            }
            let x = g.read::<u32>(at(0x38));
            g.write::<u32>(at(0x28), x);
        }
        1 => {
            let left = sub(rf(g, 0x11c), k(g, 0xf9));
            g.write::<f32>(at(0x11c), left);
            // comiss 0, t; jbe: not above (or unordered) keeps it
            if 0.0 > left {
                g.write::<u32>(at(0x11c), 0);
            }
            let q = div(rf(g, 0x11c), rf(g, 0x120));
            // call width: 3
            g.call(t(g, 0x135, 233), 0, &[m, at(0x10c), q.to_bits()]);
            // call width: 3 (slot 233's words stay on the stack)
            g.call(t(g, 0x151, 234), 0, &[at(0x1c), at(0xfc), m]);
            g.write::<u32>(at(0x28), w(0x163));
        }
        _ => {}
    }
    if g.read::<u8>(at(0x135)) == 0 {
        for (x, to) in [(0x3cu32, 0x4cu32), (0x40, 0x50), (0x44, 0x54)] {
            let v = add(mul(sub(rf(g, x), rf(g, to)), rf(g, 0x60)), rf(g, to));
            g.write::<f32>(at(x), v);
        }
        let x = g.read::<u32>(at(0x58));
        g.write::<u32>(at(0x48), x);
    }
    if g.read::<u16>(at(0x98)) == 0 {
        let e8 = mul(rf(g, 0xf8), rf(g, 0xe8));
        let x = mul(rf(g, 0xd8), e8);
        g.write::<f32>(at(0xe8), e8);
        g.write::<f32>(at(0x64), add(x, rf(g, 0xc8)));
    }
    let spins = [(0x6cu32, 0x7cu32), (0x70, 0x80), (0x74, 0x84)];
    match g.read::<u8>(at(0x134)) {
        0 => {
            for (x, by) in spins {
                let v = mul(rf(g, by), rf(g, x));
                g.write::<f32>(at(x), v);
            }
        }
        1 => {
            for (x, by) in spins {
                let d = rf(g, by);
                let v = sub(rf(g, x), d);
                g.write::<f32>(at(x), v);
                // falling (d > 0) stops at 0 from above, rising from below
                let crossed = if d > 0.0 { 0.0 >= v } else { v > 0.0 };
                if crossed {
                    g.write::<u32>(at(x), 0);
                }
            }
        }
        _ => {}
    }
    let a = ebp.wrapping_sub(0x14);
    for i in 0..3u32 {
        let x = rf(g, 0x6c + 4 * i);
        let d = i32::from(g.read::<u16>(at(0x8c + 2 * i)).cast_signed());
        // comiss 0, x; jbe: not above (or unordered) keeps the sign
        let base = if 0.0 > x { d.wrapping_neg() } else { d };
        #[allow(clippy::cast_precision_loss)] // cvtdq2ps
        let part = crate::sb_sse::cvtt(mul(d as f32, x));
        g.write::<u32>(a + 4 * i, base.wrapping_sub(part).cast_unsigned());
    }
    #[allow(clippy::cast_precision_loss)] // cvtdq2ps
    let z = g.read::<u32>(a + 8).cast_signed() as f32;
    let z = add(mul(mul(z, k(g, 0x388)), k(g, 0x39a)), rf(g, 0x88));
    g.write::<u32>(a + 8, 0);
    g.write::<f32>(at(0x78), z);
    crate::s3_matrix::rotation_4250(g, img, img.rel32_target(f + 0x3ac), m, a);
    let fl = g.read::<u32>(at(0x12c));
    if fl & 8 != 0 && fl & 1 != 0 {
        let mf = |g: &G, o: u32| g.read::<f32>(m + o);
        for (i, x) in (0u32..).step_by(4).zip([0x1cu32, 0x20, 0x24]) {
            let v = sub(rf(g, x), mul(mf(g, i), rf(g, 0x9c)));
            let v = sub(v, mul(mf(g, 0x10 + i), rf(g, 0xa0)));
            let v = sub(v, mul(mf(g, 0x20 + i), rf(g, 0xa4)));
            g.write::<f32>(at(0x4c + i), v);
        }
        let x = g.read::<u32>(at(0xa8));
        g.write::<u32>(at(0x58), x);
    }
    let l = g.read::<u32>(at(0x130));
    let mut st = Pushes::default();
    st.push_all(&[at(0x3c), 1, l]);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, t(g, 0x48b, 284), 0);
    let l = g.read::<u32>(at(0x130));
    st.push_all(&[at(0x1c), 0, l]);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, t(g, 0x4a4, 284), 0);
    st.pop(5);
    st.set_top(rf(g, 0x78).to_bits());
    st.push(g.read::<u32>(at(0x130)));
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, t(g, 0x4be, 285), 0);
    st.pop(1);
    st.set_top(rf(g, 0x64).to_bits());
    st.push(g.read::<u32>(at(0x130)));
    // call width: 12 (the most `Pushes::words` holds)
    let eax = st.call(g, t(g, 0x4de, 286), 0).eax;
    Ret::eax(eax)
}

/// [`light_visit_0577`] (the `rel32`s: the rotation,
/// `__security_check_cookie`).
pub const LIGHT_VISIT_0577: Sig = Sig {
    len: 1292,
    hash: 0xd99f_b967_90fc_5e81,
    calls: &[
        (0x3ac, Callee::Sig(&crate::s9_misc::ROTATION_3650)),
        (0x504, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0577`'s motion task (0x1000ac90, 1173 bytes; a task callback
/// `(A)`, cdecl; an aligned /GS frame). `S = A[1]`, the state `S + 0x28`,
/// the timer `S + 0x2c`, the object `O = S[0x48]` (motion owner `O[0x50]`,
/// model `O[0x54]`, bit 0 of `O + 0x90` the model's flag), the summon `B =
/// S[0x4c]` (its points `B[0x13c] + 0x1c`, its body `B[0x144]`). Each run
/// dispatches the state (some states go on to the next at once):
/// - 0: slot 325 `(S, P)`, then state 1 with the timer 30; 1: at the timer's
///   end slot 228 `(O[0x50], 0xe2)`, on to 2;
/// - 2: once slot 411 `(O[0x50])` says the motion ended, state 30 at once;
///   30: slot 228 `(.., 0xe3)`, timer 130, on; 31: at the end slot 228
///   `(.., 0xe4)`, on; 32: when the motion ended, the model shown (slot
///   257 `(O[0x54], 1)` or 296 `(O[0x50], 0)`), state 10 at once;
/// - 10: on to 11 with the timer 75; 11: at the end the point's height
///   `+ K`, the model hidden (257 `(.., 0)` or 296 `(.., 1)`), slot 228 `(..,
///   0xe5)`, the shake `(0x10000, -0xe00)`, the body's height kept, the
///   push `(2000, 1000)`, timer 40, on;
/// - 12: at the end the shake steps (`S + 0x30 += d`, `d = d K`), slot 229
///   pushes the vector `(0, S[0x44], S[0x40], 0)` through `B + 0x9c`, slot
///   248 adds it to the points; the push decays by `K'`; when the shake is
///   at 0 or below: `d` 40, timer 40, the body's height back, on;
/// - 13: the shake `+= 0x30000 / d` raises the body; on at the timer's end;
/// - 14: when the motion ended, the model shown, state 10000 at once;
/// - 10000: slot 325 `(P, S)`, timer 260, on; 10001: at the end slot 228
///   `(.., 0xe6)`, on; 10002: once the object's motion slot says so
///   ([`slot_ecx`](crate::o4_tiny::slot_ecx)), the object back on its node
///   ([`object_to_node`](crate::sa_misc::object_to_node)), slot 213
///   `(O[0x50], 0)`, the task's node `*A` taken off its ring, the timer
///   down and state 20000.
///
/// Every other state, and every wait, only counts the timer down. "On"
/// moves the state up and counts the timer down. Returns `eax` as the
/// original leaves it (the state's table byte, a call's answer or the
/// word last read).
#[allow(clippy::too_many_lines)] // the original's states, one by one
pub fn motion_task_0577<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, mul};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let a = fr.arg(g, 0);
    let s = g.read::<u32>(a.wrapping_add(4));
    let at = |o: u32| s.wrapping_add(o);
    let rd = |g: &G, o: u32| g.read::<u32>(at(o));
    // push ebp; and esp, -8; sub esp, 0x18; push esi; push edi
    let e = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x20);
    let cookie = g.read::<u32>(w(0xa)) ^ e.wrapping_add(8);
    g.write::<u32>(e + 0x1c, cookie);
    let obj = |g: &G| rd(g, 0x48);
    let owner = |g: &G| g.read::<u32>(obj(g).wrapping_add(0x50));
    let points = |g: &G| {
        g.read::<u32>(rd(g, 0x4c).wrapping_add(0x13c))
            .wrapping_add(0x1c)
    };
    let table = w(0x42);
    // slot 228 `(O[0x50], m)` through the operand at `op`
    let motion = |g: &mut G, op: u32, m: u32| {
        let o = owner(g);
        // call width: 2
        g.call(t(g, op, 228), 0, &[o, m]).eax
    };
    // the model shown or hidden: slot 257 `(O[0x54], on)` with the model's
    // flag, else slot 296 `(O[0x50], off)`
    let show = |g: &mut G, (t257, t296): (u32, u32), on: u32| {
        let o = obj(g);
        if g.read::<u8>(o.wrapping_add(0x90)) & 1 != 0 {
            let m = g.read::<u32>(o.wrapping_add(0x54));
            // call width: 2
            g.call(t(g, t257, 257), 0, &[m, on]).eax
        } else {
            let x = g.read::<u32>(o.wrapping_add(0x50));
            // call width: 2
            g.call(t(g, t296, 296), 0, &[x, on ^ 1]).eax
        }
    };
    let ended = |g: &mut G, op: u32| {
        let o = owner(g);
        // call width: 1
        g.call(t(g, op, 411), 0, &[o]).eax
    };
    // "on": the state up, the timer down
    let on = |g: &mut G| {
        let x = rd(g, 0x28).wrapping_add(1);
        g.write::<u32>(at(0x28), x);
    };
    let tick = |g: &mut G| {
        let x = rd(g, 0x2c).wrapping_sub(1);
        g.write::<u32>(at(0x2c), x);
    };
    let waiting = |g: &G| rd(g, 0x2c).cast_signed() > 0;
    let eax = loop {
        let state = rd(g, 0x28);
        if state > 0x2710 {
            match state - 0x2711 {
                0 => {
                    if waiting(g) {
                        tick(g);
                        break 0;
                    }
                    let eax = motion(g, 0x41e, 0xe6);
                    on(g);
                    tick(g);
                    break eax;
                }
                1 => {
                    let ecx = owner(g);
                    let poll = Frame {
                        at: img.rel32_target(f + 0x156),
                        ecx,
                        ..*fr
                    };
                    let r = crate::o4_tiny::slot_ecx(g, img, &poll).eax;
                    if r == 0 {
                        tick(g);
                        break 0;
                    }
                    let o = obj(g);
                    crate::sa_misc::object_to_node(g, img, img.rel32_target(f + 0x166), o, r);
                    let x = owner(g);
                    // call width: 2
                    g.call(t(g, 0x173, 213), 0, &[x, 0]);
                    let node = g.read::<u32>(a);
                    let (next, prev) = (g.read::<u32>(node + 4), g.read::<u32>(node + 8));
                    g.write::<u32>(next + 8, prev);
                    let (prev, next) = (g.read::<u32>(node + 8), g.read::<u32>(node + 4));
                    g.write::<u32>(prev + 4, next);
                    g.write::<u32>(node + 8, node);
                    g.write::<u32>(node + 4, node);
                    tick(g);
                    g.write::<u32>(at(0x28), 0x4e20);
                    break next;
                }
                d => {
                    tick(g);
                    break d - 1;
                }
            }
        }
        if state == 0x2710 {
            let p = points(g);
            // call width: 2
            let eax = g.call(t(g, 0x3f7, 325), 0, &[p, s]).eax;
            g.write::<u32>(at(0x2c), 0x104);
            on(g);
            tick(g);
            break eax;
        }
        if state > 0x20 {
            tick(g);
            break state;
        }
        // the byte table after the code (read from the image: the code
        // section is not in game memory)
        let byte = img.word(table.wrapping_add(state)) & 0xff;
        match state {
            0 | 1 => {
                if state == 0 {
                    let p = points(g);
                    // call width: 2
                    let eax = g.call(t(g, 0x1c5, 325), 0, &[s, p]).eax;
                    on(g);
                    g.write::<u32>(at(0x2c), 0x1e);
                    tick(g);
                    break eax;
                }
                if waiting(g) {
                    tick(g);
                    break byte;
                }
                let eax = motion(g, 0x41e, 0xe2);
                on(g);
                tick(g);
                break eax;
            }
            2 => {
                if ended(g, 0x54) == 0 {
                    tick(g);
                    break 0;
                }
                g.write::<u32>(at(0x28), 0x1e);
            }
            32 => {
                if ended(g, 0x7e) == 0 {
                    tick(g);
                    break 0;
                }
                show(g, (0xa7, 0xc2), 1);
                g.write::<u32>(at(0x28), 0xa);
            }
            14 => {
                if ended(g, 0xde) == 0 {
                    tick(g);
                    break 0;
                }
                show(g, (0x107, 0x119), 1);
                g.write::<u32>(at(0x28), 0x2710);
            }
            30 => {
                let eax = motion(g, 0x1ff, 0xe3);
                g.write::<u32>(at(0x2c), 0x82);
                on(g);
                tick(g);
                break eax;
            }
            31 => {
                if waiting(g) {
                    tick(g);
                    break byte;
                }
                let eax = motion(g, 0x41e, 0xe4);
                on(g);
                tick(g);
                break eax;
            }
            10 | 11 => {
                if state == 10 {
                    on(g);
                    g.write::<u32>(at(0x2c), 0x4b);
                    tick(g);
                    break byte;
                }
                if waiting(g) {
                    tick(g);
                    break byte;
                }
                let p = g.read::<u32>(rd(g, 0x4c).wrapping_add(0x13c));
                let y = add(g.read::<f32>(p + 0x20), k(g, 0x251));
                g.write::<f32>(p + 0x20, y);
                show(g, (0x26c, 0x27e), 0);
                motion(g, 0x299, 0xe5);
                g.write::<u32>(at(0x30), 0x1_0000);
                g.write::<u32>(at(0x34), 0xffff_f200);
                let body = g.read::<u32>(rd(g, 0x4c).wrapping_add(0x144));
                let h = g.read::<u32>(body.wrapping_add(0x20));
                g.write::<u32>(at(0x38), h);
                g.write::<u32>(at(0x40), w(0x2c5));
                g.write::<u32>(at(0x44), w(0x2cc));
                g.write::<u32>(at(0x2c), 0x28);
                on(g);
                tick(g);
                break h;
            }
            12 => {
                if waiting(g) {
                    tick(g);
                    break byte;
                }
                let d = rd(g, 0x34);
                let sh = rd(g, 0x30).wrapping_add(d);
                g.write::<u32>(at(0x30), sh);
                #[allow(clippy::cast_precision_loss)] // cvtdq2ps
                let d = crate::sb_sse::cvtt(mul(d.cast_signed() as f32, k(g, 0x2f7)));
                g.write::<u32>(at(0x34), d.cast_unsigned());
                let v = e + 0xc;
                g.write::<u32>(v, 0);
                g.write::<u32>(v + 4, rd(g, 0x44));
                g.write::<u32>(v + 8, rd(g, 0x40));
                g.write::<u32>(v + 0xc, 0);
                let m = rd(g, 0x4c).wrapping_add(0x9c);
                // call width: 3
                g.call(t(g, 0x33c, 229), 0, &[v, m, v]);
                let p = points(g);
                // call width: 3
                let eax = g.call(t(g, 0x35f, 248), 0, &[p, p, v]).eax;
                let x40 = mul(g.read::<f32>(at(0x40)), k(g, 0x374));
                let still = rd(g, 0x30).cast_signed() > 0;
                g.write::<f32>(at(0x40), x40);
                let x44 = mul(g.read::<f32>(at(0x44)), k(g, 0x38d));
                g.write::<f32>(at(0x44), x44);
                if still {
                    tick(g);
                    break eax;
                }
                g.write::<u32>(at(0x34), 0x28);
                g.write::<u32>(at(0x2c), 0x28);
                g.write::<u32>(at(0x30), 0);
                let body = g.read::<u32>(rd(g, 0x4c).wrapping_add(0x144));
                let h = rd(g, 0x38);
                g.write::<u32>(body.wrapping_add(0x20), h);
                on(g);
                tick(g);
                break h;
            }
            13 => {
                // idiv: a zero step faults in the original (the port takes 0)
                let q = 0x3_0000i32
                    .checked_div(rd(g, 0x34).cast_signed())
                    .unwrap_or(0);
                let sh = rd(g, 0x30).wrapping_add(q.cast_unsigned());
                g.write::<u32>(at(0x30), sh);
                let body = g.read::<u32>(rd(g, 0x4c).wrapping_add(0x144));
                let h = rd(g, 0x38).wrapping_add(rd(g, 0x30));
                g.write::<u32>(body.wrapping_add(0x20), h);
                if !waiting(g) {
                    on(g);
                }
                tick(g);
                break body;
            }
            _ => {
                tick(g);
                break byte;
            }
        }
    };
    Ret::eax(eax)
}

/// [`motion_task_0577`] (the `rel32`s: the motion slot, the object back
/// on its node, `__security_check_cookie`).
pub const MOTION_TASK_0577: Sig = Sig {
    len: 1173,
    hash: 0xbc4c_4bdb_482e_8f42,
    calls: &[
        (0x156, Callee::Sig(&crate::o4_tiny::SLOT_ECX_0577_ECA0)),
        (0x166, Callee::Sig(&crate::sa_misc::OBJECT_TO_NODE)),
        (0x1af, Callee::Any),
        (0x43c, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0512`'s aim visit (0x100037a0, 929 bytes; internal, `ecx` = the
/// cursor, reached by its 12-byte thunk 0x10003b50; an aligned /GS frame,
/// `E` below the three pushes). After the children ([`visit_children`]),
/// with the node `N`:
/// - its parts `N[0x13c]`, `N[0x154]`, `N[0x140]` are visited through a
///   local cursor `E + 0x10` (the cursor's mark byte at `+5`);
/// - `V = N[0x154] + 0x1c - (N[0x13c] + 0x1c)` (slot 234 into `E + 0x1c`),
///   its length `l` (`_libm_sse2_sqrt_precise`), slot 232 normalises it;
///   with `N + 0x150` set the flat part again (`V.y` 0, `l` times its flat
///   length);
/// - slot 233 turns `V` round (-1.0); `V = N[0x3c..0x48] (1 - t) + V t` (`t =
///   N + 0x14c`), slot 232 `(N + 0x3c, V)`;
/// - with `U = (0, 1, 0, 1)`: slot 266 `(N + 0x1c, U, N + 0x3c)` and
///   `(N + 0x2c, N + 0x3c, N + 0x1c)` (the axes), slot 325
///   `(N + 0x12c, N[0x140] + 0x1c)`, slot 233 `(U, N + 0x12c, N[0x138])`,
///   the three axes scaled by `U.x`, `U.y` and `U.z l K`, slot 230
///   `(N + 0x9c, N + 0x1c)`, slot 325 `(N + 0xcc, N[0x13c] + 0x1c)`.
///
/// Returns slot 325's last `eax` (the mark in the low byte with the
/// cookie's bits above when the node was marked).
#[allow(clippy::too_many_lines)] // the original's steps, one by one
pub fn aim_visit_0512<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let rf = |g: &G, at: u32| g.read::<f32>(at);
    // push ebp; and esp, -8; sub esp, 0x24; three pushes
    let e0 = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x24);
    let cookie = g.read::<u32>(w(0xa)) ^ e0;
    g.write::<u32>(e0 + 0x20, cookie);
    let e = e0.wrapping_sub(0xc);
    let c = fr.ecx;
    let n = match visit_children(g, c) {
        Ok(n) => n,
        Err(mark) => return Ret::eax((cookie & !0xff) | u32::from(mark)),
    };
    g.write::<u32>(e + 0xc, n);
    let at = |o: u32| n.wrapping_add(o);
    let word = |g: &G, o: u32| g.read::<u32>(at(o));
    let holder = e + 0x10;
    for part in [0x13cu32, 0x154, 0x140] {
        let p = word(g, part);
        let mark = g.read::<u8>(c.wrapping_add(5));
        g.write::<u32>(holder, p);
        g.write::<u8>(holder + 5, mark);
        let visit = g.read::<u32>(g.read::<u32>(p).wrapping_add(0x1c));
        // call width: 1 (the visits read their cursor from the stack)
        g.call(visit, holder, &[holder]);
    }
    let v = e + 0x1c;
    let (pa, pb) = (
        word(g, 0x13c).wrapping_add(0x1c),
        word(g, 0x154).wrapping_add(0x1c),
    );
    // call width: 3
    g.call(t(g, 0xdd, 234), 0, &[v, pb, pa]);
    let (x, y, z) = (rf(g, v), rf(g, v + 4), rf(g, v + 8));
    let sq = add(add(mul(y, y), mul(x, x)), mul(z, z));
    // `_libm_sse2_sqrt_precise` of the double: exact, as Rust's
    #[allow(clippy::cast_possible_truncation)]
    let len = f64::from(sq).sqrt() as f32;
    g.write::<f32>(e + 0x18, len);
    // call width: 2
    g.call(t(g, 0x125, 232), 0, &[v, v]);
    if word(g, 0x150) != 0 {
        let (x, z) = (rf(g, v), rf(g, v + 8));
        #[allow(clippy::cast_possible_truncation)]
        let flat = f64::from(add(mul(z, z), mul(x, x))).sqrt() as f32;
        let flat = mul(flat, rf(g, e + 0x18));
        g.write::<u32>(v + 4, 0);
        g.write::<f32>(e + 0x18, flat);
        // call width: 2
        g.call(t(g, 0x174, 232), 0, &[v, v]);
    }
    // call width: 3
    g.call(t(g, 0x1a0, 233), 0, &[v, v, 0xbf80_0000]);
    let tw = rf(g, at(0x14c));
    let rest = sub(k(g, 0x1b8), tw);
    for i in 0..4u32 {
        let x = add(
            mul(rf(g, at(0x3c + 4 * i)), rest),
            mul(rf(g, v + 4 * i), tw),
        );
        g.write::<f32>(v + 4 * i, x);
    }
    let fwd = at(0x3c);
    let mut st = Pushes::default();
    st.push_all(&[v, fwd]);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, t(g, 0x1ed, 232), 0);
    let up = v;
    for (i, x) in (0u32..).zip([0u32, ONE, 0, ONE]) {
        g.write::<u32>(up + 4 * i, x);
    }
    let side = at(0x1c);
    st.push_all(&[fwd, up, side]);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, t(g, 0x254, 266), 0);
    let top = at(0x2c);
    // call width: 3
    g.call(t(g, 0x289, 266), 0, &[top, fwd, side]);
    let base = at(0x12c);
    let pc = word(g, 0x140).wrapping_add(0x1c);
    let mut st = Pushes::default();
    st.push_all(&[pc, base]);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, t(g, 0x2b6, 325), 0);
    st.pop(1);
    st.set_top(word(g, 0x138));
    st.push_all(&[base, up]);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, t(g, 0x2de, 233), 0);
    let ux = rf(g, up);
    st.pop(2);
    st.set_top(ux.to_bits());
    st.push_all(&[side, side]);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, t(g, 0x2f1, 233), 0);
    let uy = rf(g, up + 4);
    st.pop(2);
    st.set_top(uy.to_bits());
    st.push_all(&[top, top]);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, t(g, 0x30e, 233), 0);
    let uz = mul(mul(rf(g, up + 8), rf(g, e + 0x18)), k(g, 0x33b));
    st.pop(2);
    st.set_top(uz.to_bits());
    st.push_all(&[fwd, fwd]);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, t(g, 0x34a, 233), 0);
    // call width: 2
    g.call(t(g, 0x362, 230), 0, &[at(0x9c), side]);
    let pa = word(g, 0x13c).wrapping_add(0x1c);
    // call width: 4 (slot 230's words stay on the stack)
    let eax = g
        .call(t(g, 0x380, 325), 0, &[at(0xcc), pa, at(0x9c), side])
        .eax;
    Ret::eax(eax)
}

/// [`aim_visit_0512`] (the `rel32`s: `_libm_sse2_sqrt_precise` twice,
/// `__security_check_cookie`).
pub const AIM_VISIT_0512: Sig = Sig {
    len: 929,
    hash: 0xa5a9_21e5_f0ec_b8be,
    calls: &[
        (0x116, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x15f, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x399, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0512`'s product with its matrix (0x10006aa0, 1169 bytes;
/// internal, `ecx` = the output, the stack `(A)`): `out = A G` with `G`
/// the DLL's 4x4 matrix (its first word the operand `+0x2b`; the code
/// names all sixteen). Each element is `((a0 g0c + a1 g1c) + a2 g2c) +
/// a3 g3c` in that order, the last column's last product as `g33 a3`
/// (`mulss` the other way round); all sixteen are computed before the
/// output is written; the argument word ends holding the second row's
/// last element (a spill). Returns `A` (`eax`).
pub fn matrix_product_0512<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, mul};
    let m = img.word(fr.at + 0x2b);
    let a = fr.arg(g, 0);
    let gm = |g: &G, k: u32, c: u32| g.read::<f32>(m.wrapping_add(16 * k + 4 * c));
    let mut out = [0f32; 16];
    for r in 0..4u32 {
        let row = [0u32, 1, 2, 3].map(|k| g.read::<f32>(a.wrapping_add(16 * r + 4 * k)));
        for c in 0..4u32 {
            let s = add(mul(row[0], gm(g, 0, c)), mul(row[1], gm(g, 1, c)));
            let s = add(s, mul(row[2], gm(g, 2, c)));
            let last = if c == 3 {
                mul(gm(g, 3, 3), row[3])
            } else {
                mul(row[3], gm(g, 3, c))
            };
            out[(4 * r + c) as usize] = add(s, last);
        }
    }
    // the argument word is a spill slot: it ends holding row 1's last
    g.write::<f32>(fr.esp.wrapping_add(4), out[7]);
    for (i, x) in (0u32..).zip(out) {
        g.write::<f32>(fr.ecx.wrapping_add(4 * i), x);
    }
    Ret::eax(a)
}

/// [`matrix_product_0512`].
pub const MATRIX_PRODUCT_0512: Sig = Sig {
    len: 1169,
    hash: 0xf2b7_316a_fe2a_1605,
    calls: &[],
    imports: &[],
};

/// `magic_0512`'s two-part object set-up (0x10009150, 1222 bytes;
/// internal: `ecx` = the object `O`, `edx` = the actor's object `X`, the
/// stack `(A, b)`; an `ebp` frame, the insert group at `ebp - 0x1c`):
/// - `O` a node on the static list (vtable, callback, the list's flag bit
///   0 cleared);
/// - five children of the parent ([`child_of_0512`](crate::sa_misc::child_of_0512)),
///   each given its vtable: `O + 0x2bc` (slot 268 `(X[0x54], 0x11, O +
///   0x2d8)`, `O + 0x2e4` 1.0; without `b` slot 12 `(X[0x54])` (ST0) into
///   `O + 0x2dc`; that less `K`), `O + 0x36c` (`O + 0x390`, `+0x388` 0, the
///   tilt `O + 0x38c = (K - h) K' / K''` truncated, `h` slot 333 `(A[0x54])`
///   with the model's flag `A + 0x90` bit 0, else slot 601 `(A[0x50], &h)`
///   into the argument word), `O + 0x2e8` (scales 1.0, slot 251
///   `(X[0x50])`'s size clamped to 10..18 and scaled into `O + 0x310`),
///   `O + 0x314` (slot 268 into `O + 0x330`, slot 12's height less `K` at
///   `O + 0x334`, `O + 0x33c` 1.0), `O + 0x340` (as `O + 0x2e8`, into
///   `O + 0x368`);
/// - two model nodes inserted ([`node_insert`](crate::nodes::node_insert):
///   `O + 0x24` with the children `O + 0x2bc`, `O + 0x36c`, `O + 0x2e8`;
///   `O + 0x170` with `O + 0x314`, `O + 0x36c`, `O + 0x340`);
/// - `O + 0x1c = b`, `O + 0x18` 0. Returns `b`.
///
/// The slot 333 and 251 answers are stored (`fstp`) into the caller's
/// argument word `A`, as the original does.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn object_setup_0512<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sa_misc::child_of_0512 as child;
    use crate::sb_sse::{div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let (obj, x) = (fr.ecx, fr.edx);
    let arg_a = fr.esp.wrapping_add(4);
    let a = g.read::<u32>(arg_a);
    let b = fr.arg(g, 1);
    let ebp = fr.esp.wrapping_sub(4);
    let o = |off: u32| obj.wrapping_add(off);
    let rf = |g: &G, at: u32| g.read::<f32>(at);
    // the object on the static list
    g.write::<u32>(o(8), obj);
    g.write::<u32>(o(4), obj);
    g.write::<u32>(obj, w(0x15));
    g.write::<u32>(o(0x10), w(0x1c));
    g.write::<u32>(o(0x14), obj);
    let last = g.read::<u32>(w(0x24));
    g.write::<u32>(o(8), last);
    g.write::<u32>(o(4), w(0x2e));
    let last = g.read::<u32>(w(0x33));
    g.write::<u32>(last.wrapping_add(4), obj);
    let fl = g.read::<u32>(w(0x42)) & !1;
    g.write::<u32>(w(0x42), fl);
    g.write::<u32>(w(0x49), obj);
    // the first child: slot 268's place, the height without `b`
    let c1 = o(0x2bc);
    child(
        g,
        img,
        f,
        c1,
        [0x5c, 0x63, 0x68, 0x70, 0x78, 0x80, 0x8c, 0x9e],
    );
    g.write::<u32>(c1, w(0xa9));
    let m54 = g.read::<u32>(x.wrapping_add(0x54));
    // call width: 3
    g.call(t(g, 0x98, 268), 0, &[m54, 0x11, o(0x2d8)]);
    g.write::<u32>(o(0x2e4), ONE);
    if b == 0 {
        let m54 = g.read::<u32>(x.wrapping_add(0x54));
        // call width: 1
        let h = st0_f32(g.call(t(g, 0xcf, 12), 0, &[m54]).st0);
        g.write::<f32>(o(0x2dc), h);
    }
    let y = sub(rf(g, o(0x2dc)), k(g, 0xf0));
    g.write::<u32>(o(0x2e4), ONE);
    let c2 = o(0x36c);
    g.write::<f32>(o(0x2dc), y);
    g.write::<u32>(c2, w(0x10e));
    child(
        g,
        img,
        f,
        c2,
        [0x121, 0x128, 0x12d, 0x135, 0x13a, 0x142, 0x14e, 0x157],
    );
    g.write::<u32>(o(0x390), 0);
    g.write::<u32>(o(0x388), 0);
    let h = if g.read::<u8>(a.wrapping_add(0x90)) & 1 != 0 {
        let m = g.read::<u32>(a.wrapping_add(0x54));
        // call width: 1
        let h = st0_f32(g.call(t(g, 0x17c, 333), 0, &[m]).st0);
        g.write::<f32>(arg_a, h);
        sub(k(g, 0x18f), h)
    } else {
        let m = g.read::<u32>(a.wrapping_add(0x50));
        // call width: 2
        g.call(t(g, 0x1a5, 601), 0, &[m, arg_a]);
        sub(k(g, 0x1b5), rf(g, arg_a))
    };
    let tilt = div(mul(h, k(g, 0x1c5)), k(g, 0x1d3));
    g.write::<u32>(o(0x38c), crate::sb_sse::cvtt(tilt).cast_unsigned());
    // the third child and its size
    let c3 = o(0x2e8);
    child(
        g,
        img,
        f,
        c3,
        [0x1f0, 0x1f7, 0x1fc, 0x204, 0x209, 0x211, 0x21d, 0x226],
    );
    g.write::<u32>(c3, w(0x22c));
    // (the children, the scale words, slot 251's operand and the clamp's)
    let size = |g: &mut G, base: u32, [op, lo, hi, sc]: [u32; 4]| {
        for i in 0..3u32 {
            g.write::<u32>(o(base + 4 * i), ONE);
        }
        let m = g.read::<u32>(x.wrapping_add(0x50));
        // call width: 1
        let v = st0_f32(g.call(t(g, op, 251), 0, &[m]).st0);
        g.write::<f32>(arg_a, v);
        let at = o(base + 0xc);
        g.write::<f32>(at, v);
        // comiss K, v; jbe: not above (or unordered) keeps it
        if k(g, lo) > v {
            g.write::<u32>(at, 0x4120_0000);
        }
        if rf(g, at) > k(g, hi) {
            g.write::<u32>(at, 0x4190_0000);
        }
        let s = mul(rf(g, at), k(g, sc));
        g.write::<f32>(at, s);
    };
    size(g, 0x304, [0x259, 0x271, 0x29a, 0x2b6]);
    let grp = ebp.wrapping_sub(0x1c);
    let insert = |g: &mut G, call: u32, words: [u32; 6]| {
        for (i, v) in (0u32..).zip(words) {
            g.write::<u32>(grp + 4 * i, v);
        }
        let fr2 = Frame {
            at: img.rel32_target(f + call),
            ecx: grp,
            ..*fr
        };
        crate::nodes::node_insert(g, img, &fr2);
    };
    insert(g, 0x2e9, [o(0x24), w(0x2d4), w(0x2db), c1, c2, c3]);
    // the fourth child: slot 268's place and the height
    let c4 = o(0x314);
    child(
        g,
        img,
        f,
        c4,
        [0x302, 0x309, 0x30e, 0x316, 0x31b, 0x323, 0x32f, 0x33e],
    );
    g.write::<u32>(c4, w(0x345));
    let m54 = g.read::<u32>(x.wrapping_add(0x54));
    // call width: 3
    g.call(t(g, 0x34f, 268), 0, &[m54, 0x11, o(0x330)]);
    g.write::<u32>(o(0x33c), ONE);
    let m54 = g.read::<u32>(x.wrapping_add(0x54));
    // call width: 1
    let hy = st0_f32(g.call(t(g, 0x36c, 12), 0, &[m54]).st0);
    g.write::<f32>(o(0x334), hy);
    let y = sub(rf(g, o(0x334)), k(g, 0x384));
    g.write::<u32>(o(0x33c), ONE);
    g.write::<f32>(o(0x334), y);
    // the fifth child and its size
    let c5 = o(0x340);
    child(
        g,
        img,
        f,
        c5,
        [0x3b2, 0x3b9, 0x3be, 0x3c6, 0x3cb, 0x3d3, 0x3df, 0x3e8],
    );
    g.write::<u32>(c5, w(0x3ee));
    size(g, 0x35c, [0x41b, 0x433, 0x45c, 0x478]);
    insert(g, 0x4ae, [o(0x170), w(0x499), w(0x4a0), c4, c2, c5]);
    g.write::<u32>(o(0x1c), b);
    g.write::<u32>(o(0x18), 0);
    Ret::eax(b)
}

/// [`object_setup_0512`] (the `rel32`s: the node insert twice).
pub const OBJECT_SETUP_0512: Sig = Sig {
    len: 1222,
    hash: 0xe8a6_f645_911f_eed3,
    calls: &[
        (0x2e9, Callee::Sig(&crate::nodes::INSERT)),
        (0x4ae, Callee::Sig(&crate::nodes::INSERT)),
    ],
    imports: &[],
};

/// `magic_0601`'s screen object set-up (0x100083f0, 946 bytes; internal,
/// `ecx` = the object `O`; an `ebp` frame, the insert group at `ebp -
/// 0x18`):
/// - `O` a node on the static list (vtable, callback, the list's flag bit
///   0 cleared);
/// - `O + 0x20` = slot 687 `(1, 1, 7)`; slots 692 `(0, 1, 0)`, `(1, 1,
///   1)`; 691 `(n, 0, 0, 0x200, 0x1a0, n, -1)` and 690 `(n, 0, 0, 0x200,
///   0x1a0)` for `n` 0 and 1; 693 `(0x28, 0x14)`;
/// - five children of the parent ([`child_of_0512`](crate::sa_misc::child_of_0512)),
///   each given its vtable: `O + 0xb0` (`O + 0xcc..` = `(1, -15, 0, 1)`),
///   `O + 0xdc` (`O + 0xf8..` = `(1, 1, 1, 1000)`), `O + 0x108`; the node
///   `O + 0x134` inserted ([`node_insert`](crate::nodes::node_insert)) with
///   the three; `O + 0x124..` = `(0x1000, -0x1000, 0)`; `O + 0x30` and `O +
///   0x70` (each `+0x38` 1.0, `+0x3c` the inserted node; `O + 0x5c..` =
///   `(0, 0, -10)`, `O + 0x9c..` 0);
/// - `O + 0x24`, `+0x28` the last two children, `O + 0x2c` the DLL's
///   block, `O + 0x1c` 0.
///
/// Returns the parent's last child before the fifth was appended.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn screen_setup_0601<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sa_misc::child_of_0512 as child;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let obj = fr.ecx;
    let o = |off: u32| obj.wrapping_add(off);
    // the object on the static list
    g.write::<u32>(o(8), obj);
    g.write::<u32>(o(4), obj);
    g.write::<u32>(obj, w(0x13));
    g.write::<u32>(o(0x10), w(0x1a));
    g.write::<u32>(o(0x14), obj);
    let last = g.read::<u32>(w(0x22));
    g.write::<u32>(o(8), last);
    g.write::<u32>(o(4), w(0x2c));
    let last = g.read::<u32>(w(0x31));
    g.write::<u32>(last.wrapping_add(4), obj);
    let fl = g.read::<u32>(w(0x41)) & !1;
    g.write::<u32>(w(0x41), fl);
    g.write::<u32>(w(0x48), obj);
    // the screen slots
    // call width: 3
    let eax = g.call(t(g, 0x3b, 687), 0, &[1, 1, 7]).eax;
    g.write::<u32>(o(0x20), eax);
    // call width: 3
    g.call(t(g, 0x5f, 692), 0, &[0, 1, 0]);
    // call width: 3
    g.call(t(g, 0x72, 692), 0, &[1, 1, 1]);
    for (n, t691, t690) in [(0u32, 0x88u32, 0xacu32), (1, 0xcc, 0xf0)] {
        // call width: 7
        g.call(t(g, t691, 691), 0, &[n, 0, 0, 0x200, 0x1a0, n, u32::MAX]);
        // call width: 5
        g.call(t(g, t690, 690), 0, &[n, 0, 0, 0x200, 0x1a0]);
    }
    // call width: 2
    g.call(t(g, 0x117, 693), 0, &[0x28, 0x14]);
    // the children
    let c1 = o(0xb0);
    child(
        g,
        img,
        f,
        c1,
        [0x13b, 0x142, 0x147, 0x14f, 0x154, 0x15c, 0x168, 0x16e],
    );
    g.write::<u32>(c1, w(0x174));
    for (off, v) in [(0xccu32, ONE), (0xd0, 0xc170_0000), (0xd4, 0), (0xd8, ONE)] {
        g.write::<u32>(o(off), v);
    }
    let c2 = o(0xdc);
    child(
        g,
        img,
        f,
        c2,
        [0x1b5, 0x1bc, 0x1c1, 0x1c9, 0x1ce, 0x1d6, 0x1e2, 0x1e8],
    );
    g.write::<u32>(c2, w(0x1ee));
    for (off, v) in [
        (0xf8u32, ONE),
        (0xfc, ONE),
        (0x100, ONE),
        (0x104, 0x447a_0000),
    ] {
        g.write::<u32>(o(off), v);
    }
    let c3 = o(0x108);
    g.write::<u32>(c3, w(0x222));
    child(
        g,
        img,
        f,
        c3,
        [0x235, 0x23c, 0x241, 0x249, 0x24e, 0x256, 0x262, 0x268],
    );
    let node = o(0x134);
    for (off, v) in [(0x124u32, 0x1000u32), (0x128, 0xffff_f000), (0x12c, 0)] {
        g.write::<u32>(o(off), v);
    }
    let grp = fr.esp.wrapping_sub(4).wrapping_sub(0x18);
    for (i, v) in (0u32..).zip([node, w(0x29c), w(0x2a3), c1, c3, c2]) {
        g.write::<u32>(grp + 4 * i, v);
    }
    let ins = Frame {
        at: img.rel32_target(f + 0x2ae),
        ecx: grp,
        ..*fr
    };
    crate::nodes::node_insert(g, img, &ins);
    // the two children holding the inserted node
    let c4 = o(0x30);
    child(
        g,
        img,
        f,
        c4,
        [0x2c4, 0x2cb, 0x2d0, 0x2d8, 0x2dd, 0x2e5, 0x2f1, 0x2f7],
    );
    g.write::<u32>(c4 + 0x38, ONE);
    g.write::<u32>(c4 + 0x3c, node);
    g.write::<u32>(c4, w(0x307));
    for (off, v) in [(0x5cu32, 0u32), (0x60, 0), (0x64, 0xc120_0000)] {
        g.write::<u32>(o(off), v);
    }
    let c5 = o(0x70);
    let eax = g.read::<u32>(w(0x346));
    child(
        g,
        img,
        f,
        c5,
        [0x332, 0x339, 0x33e, 0x346, 0x34b, 0x353, 0x35f, 0x365],
    );
    g.write::<u32>(c5 + 0x38, ONE);
    g.write::<u32>(c5 + 0x3c, node);
    g.write::<u32>(c5, w(0x375));
    for off in [0x9cu32, 0xa0, 0xa4] {
        g.write::<u32>(o(off), 0);
    }
    g.write::<u32>(o(0x24), c4);
    g.write::<u32>(o(0x28), c5);
    g.write::<u32>(o(0x2c), w(0x3a1));
    g.write::<u32>(o(0x1c), 0);
    Ret::eax(eax)
}

/// [`screen_setup_0601`] (the `rel32`: the node insert).
pub const SCREEN_SETUP_0601: Sig = Sig {
    len: 946,
    hash: 0xd46e_5b5f_b635_2e26,
    calls: &[(0x2ae, Callee::Sig(&crate::nodes::INSERT))],
    imports: &[],
};

/// `node` a child of `parent` when there is one (the inlined append the
/// summon set-ups use, the parent from a register): the pass halfword
/// 0xff, its own empty child list, `+0x14` the parent; with a parent, its
/// `+4`, `+8` the parent and its last child, after which it goes (first
/// when the parent's list is empty), the parent's count up.
pub(crate) fn child_of<G: Game>(g: &mut G, node: u32, parent: u32) {
    g.write::<u16>(node.wrapping_add(0x1a), 0xff);
    g.write::<u32>(node.wrapping_add(0x10), node);
    g.write::<u32>(node.wrapping_add(0xc), node);
    g.write::<u32>(node.wrapping_add(0x14), parent);
    if parent == 0 {
        return;
    }
    g.write::<u32>(node.wrapping_add(4), parent);
    let last = g.read::<u32>(parent.wrapping_add(0x10));
    g.write::<u32>(node.wrapping_add(8), last);
    let last = g.read::<u32>(parent.wrapping_add(0x10));
    if last == parent {
        g.write::<u32>(parent.wrapping_add(0xc), node);
    } else {
        g.write::<u32>(last.wrapping_add(4), node);
    }
    let n = g.read::<u16>(parent.wrapping_add(0x18)).wrapping_add(1);
    g.write::<u16>(parent.wrapping_add(0x18), n);
    g.write::<u32>(parent.wrapping_add(0x10), node);
}

/// `magic_0613`'s summon set-up (0x10007640, 1572 bytes; internal, `ecx`
/// = the group `G` (`[S, P, a, b, flags byte]`); a /GS frame). With the
/// summon block `S` and the parent `P` (none when 0):
/// - `S` a child of `P` ([`child_of`]); `S + 0x1638 = b`, `+0x163c = a`,
///   byte `+0x1640` the flags with bit 1; `S + 0x1634` = slot 218 `(a, S +
///   0x162c)` (the count `n` at `S + 0x162c`, read again each round);
/// - `n` model nodes `S + 0xbc + 0x274 k` (vtable; [`model_node_init`](crate::sa_summon::model_node_init)
///   with the object byte `S[0x1634][k]`; `+0x26e` 0);
/// - `S + 0x15a8` a child of `P`; the models' mean place (slot 262 of each
///   `S + 0x110 + 0x274 k`, summed in order, times `K / n`) into `S +
///   0x15c4..` (w 1.0); `S + 0x15d4` (four 1.0), `S + 0x1600` (three 0),
///   `S + 0x145c` children of `P`, the last holding the three, its
///   pointer `K2` and byte `+0x148` 0, its bone set up ([`bone_plain`](crate::sa_summon::bone_plain));
/// - `M` = slot 327 `(M, S + 0x14f8)`; for each model `k` (`O = S + 0xbc +
///   0x274 k`): node `A = O + 0x210` (slot 262 of `O[0x54]` into `A +
///   0x1c..`, w 1.0, slot 229 through `M` into `S + 0x2c + 0x10 k`), node
///   `B = A + 0x2c` (slot 290 `(O[0x50], B + 0x1c)`, `B + 0x28` = slot 277
///   `(O[0x54])` K), node `C = O + 0x1e4` (`+0x24`, `+0x1c` 0, the tilt
///   `+0x20 = (K - h) K' / K''` truncated, `h` slot 333 `(O[0x54])` with
///   the flag `O + 0x90` bit 0, else slot 601 `(O[0x50], &h)`), node
///   `D = O + 0x98`; `O + 0x1d4`, `+0x1d8`, `+0x1dc` = `A`, `B`, `C`,
///   `O + 0x1c0 = S + 0x145c`, byte `O + 0x1e0` 0; slot 211
///   `(O[0x54], O + 0x134)`, flag bit 4 of `O + 0x90`, `O + 0x4c = D`.
///   Each node gets its vtable.
///
/// Returns `S + 0x162c` (slot 327's `eax` with no models).
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn summon_setup_0613<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let ebp = fr.esp.wrapping_sub(4);
    let cookie = g.read::<u32>(w(0xa)) ^ ebp;
    g.write::<u32>(ebp.wrapping_sub(4), cookie);
    let grp = fr.ecx;
    let s = g.read::<u32>(grp);
    let parent = |g: &G| g.read::<u32>(grp.wrapping_add(4));
    let p = parent(g);
    child_of(g, s, p);
    let s = g.read::<u32>(grp);
    let at = |o: u32| s.wrapping_add(o);
    let x = g.read::<u32>(grp.wrapping_add(0xc));
    g.write::<u32>(at(0x1638), x);
    let x = g.read::<u32>(grp.wrapping_add(8));
    g.write::<u32>(at(0x163c), x);
    let fl = g.read::<u8>(grp.wrapping_add(0x10)) | 2;
    g.write::<u8>(at(0x1640), fl);
    let a = g.read::<u32>(at(0x163c));
    // call width: 2
    let objs = g.call(t(g, 0x82, 218), 0, &[a, at(0x162c)]).eax;
    g.write::<u32>(at(0x1634), objs);
    let count = |g: &G| g.read::<u32>(at(0x162c)).cast_signed();
    // the model nodes
    let args = ebp.wrapping_sub(0x7c);
    let init = img.rel32_target(f + 0xd7);
    let mut i = 0u32;
    while i.cast_signed() < count(g) {
        let node = at(0xbc + 0x274 * i);
        let p = parent(g);
        let byte = g.read::<u8>(g.read::<u32>(at(0x1634)).wrapping_add(i));
        g.write::<u32>(args, node);
        g.write::<u32>(args + 4, p);
        g.write::<u32>(args + 8, u32::from(byte));
        g.write::<u8>(args + 0xc, 0);
        g.write::<u32>(node, w(0xd2));
        crate::sa_summon::model_node_init(g, img, init, args);
        g.write::<u16>(node + 0x26e, 0);
        i += 1;
    }
    // the mean place
    let mean = at(0x15a8);
    let p = parent(g);
    child_of(g, mean, p);
    g.write::<u32>(mean, w(0x138));
    let (mut sx, mut sy, mut sz) = (0f32, 0f32, 0f32);
    let (px, py, pz) = (
        ebp.wrapping_sub(0x14),
        ebp.wrapping_sub(0x10),
        ebp.wrapping_sub(0xc),
    );
    let mut i = 0u32;
    while i.cast_signed() < count(g) {
        let o = g.read::<u32>(at(0x110 + 0x274 * i));
        // call width: 4
        g.call(t(g, 0x16d, 262), 0, &[o, px, py, pz]);
        sx = add(g.read::<f32>(px), sx);
        sy = add(g.read::<f32>(py), sy);
        sz = add(g.read::<f32>(pz), sz);
        i += 1;
    }
    #[allow(clippy::cast_precision_loss)] // cvtdq2ps
    let q = div(k(g, 0x1d6), count(g) as f32);
    g.write::<f32>(at(0x15c4), mul(sx, q));
    g.write::<f32>(at(0x15c8), mul(sy, q));
    g.write::<f32>(at(0x15cc), mul(sz, q));
    g.write::<u32>(at(0x15d0), ONE);
    let scale = at(0x15d4);
    let p = parent(g);
    child_of(g, scale, p);
    g.write::<u32>(scale, w(0x24c));
    for o in [0x15f0u32, 0x15f4, 0x15f8, 0x15fc] {
        g.write::<u32>(at(o), ONE);
    }
    let turn = at(0x1600);
    g.write::<u32>(turn, w(0x283));
    let p = parent(g);
    child_of(g, turn, p);
    for o in [0x161cu32, 0x1620, 0x1624] {
        g.write::<u32>(at(o), 0);
    }
    let bone = at(0x145c);
    g.write::<u32>(bone, w(0x2e5));
    let p = parent(g);
    child_of(g, bone, p);
    g.write::<u32>(bone + 0x144, turn);
    g.write::<u32>(bone + 0x13c, mean);
    g.write::<u32>(bone + 0x140, scale);
    g.write::<u32>(bone + 0x128, w(0x34f));
    g.write::<u8>(bone + 0x148, 0);
    let holder = ebp.wrapping_sub(0x74);
    g.write::<u32>(holder, bone);
    g.write::<u8>(holder + 5, 0);
    let bf = Frame {
        at: img.rel32_target(f + 0x365),
        esp: ebp.wrapping_sub(0x94),
        ecx: holder,
        ..*fr
    };
    crate::sa_summon::bone_plain(g, img, &bf);
    let m = ebp.wrapping_sub(0x58);
    // call width: 2
    let mut eax = g.call(t(g, 0x375, 327), 0, &[m, at(0x14f8)]).eax;
    let mut i = 0u32;
    while i.cast_signed() < count(g) {
        let o = at(0xbc + 0x274 * i);
        let ob = |off: u32| o.wrapping_add(off);
        let (na, nb, nc, nd) = (ob(0x210), ob(0x23c), ob(0x1e4), ob(0x98));
        let p = parent(g);
        child_of(g, na, p);
        let m54 = g.read::<u32>(ob(0x54));
        let pos = na + 0x1c;
        g.write::<u32>(na, w(0x3ed));
        let mut st = Pushes::default();
        st.push_all(&[pos + 8, pos + 4, pos, m54]);
        // call width: 12 (the most `Pushes::words` holds)
        st.call(g, t(g, 0x3e4, 262), 0);
        let out = at(0x2c + 0x10 * i);
        g.write::<u32>(na + 0x28, ONE);
        st.push_all(&[pos, m, out]);
        // call width: 12 (the most `Pushes::words` holds)
        st.call(g, t(g, 0x406, 229), 0);
        let p = parent(g);
        child_of(g, nb, p);
        let m50 = g.read::<u32>(ob(0x50));
        let mut st = Pushes::default();
        st.push_all(&[nb + 0x1c, m50]);
        g.write::<u32>(nb, w(0x465));
        // call width: 12 (the most `Pushes::words` holds)
        st.call(g, t(g, 0x45f, 290), 0);
        st.push(g.read::<u32>(ob(0x54)));
        // call width: 12 (the most `Pushes::words` holds)
        let v = st0_f32(st.call(g, t(g, 0x478, 277), 0).st0);
        g.write::<f32>(nb + 0x28, mul(v, k(g, 0x496)));
        let p = parent(g);
        g.write::<u32>(nc, w(0x4aa));
        child_of(g, nc, p);
        g.write::<u32>(nc + 0x24, 0);
        g.write::<u32>(nc + 0x1c, 0);
        // the model's flag is 0 here (its node set-up copied the group's
        // byte 0); the slot 333 branch is kept as the original has it
        let h = if g.read::<u8>(ob(0x90)) & 1 != 0 {
            let m = g.read::<u32>(ob(0x54));
            // call width: 1
            let h = st0_f32(g.call(t(g, 0x4f8, 333), 0, &[m]).st0);
            sub(k(g, 0x511), h)
        } else {
            let m = g.read::<u32>(ob(0x50));
            let hp = ebp.wrapping_sub(0x70);
            // call width: 2
            g.call(t(g, 0x524, 601), 0, &[m, hp]);
            sub(k(g, 0x53a), g.read::<f32>(hp))
        };
        let tilt = div(mul(h, k(g, 0x54a)), k(g, 0x552));
        g.write::<u32>(nc + 0x20, crate::sb_sse::cvtt(tilt).cast_unsigned());
        let p = parent(g);
        g.write::<u32>(nd, w(0x571));
        child_of(g, nd, p);
        g.write::<u32>(ob(0x1dc), nc);
        g.write::<u32>(ob(0x1d8), nb);
        g.write::<u32>(ob(0x1c0), bone);
        g.write::<u32>(ob(0x1d4), na);
        g.write::<u8>(ob(0x1e0), 0);
        let m54 = g.read::<u32>(ob(0x54));
        // call width: 2
        g.call(t(g, 0x5ca, 211), 0, &[m54, ob(0x134)]);
        let fl = g.read::<u16>(ob(0x90));
        g.write::<u16>(ob(0x90), fl | 0x10);
        g.write::<u32>(ob(0x4c), nd);
        eax = at(0x162c);
        i += 1;
    }
    Ret::eax(eax)
}

/// [`summon_setup_0613`] (the `rel32`s: the model node set-up, the bone,
/// `__security_check_cookie`).
pub const SUMMON_SETUP_0613: Sig = Sig {
    len: 1572,
    hash: 0xb7e0_ed55_c716_7ed5,
    calls: &[
        (0xd7, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0x365, Callee::Sig(&crate::sa_summon::BONE)),
        (0x61c, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0609`'s summon set-up (0x10007660, 1787 bytes; internal, `ecx`
/// = the group `G` (`[S, P, a, b, flags byte, h]`); a /GS frame): as
/// [`summon_setup_0613`] with the block's words 0x10 further on and models
/// 0x278 apart, and these differences:
/// - each model node also gets slot 262 `(O[0x54])` twice (into `ebp -
///   0x24..` and `ebp - 0x14..`) and `O + 0x274` = the first's height less
///   slot 12 `(O[0x54])` (ST0);
/// - the mean place sums slot 366 `(O[0x50], &v)` of the models without
///   the flag `O + 0x90` bit 0; a model with it adds the vector left in the
///   frame again;
/// - after slot 327, `S + 0x15d8 += G.h` and the bone is set up a second
///   time with the mark byte 1;
/// - each model's last node `D` is set up as a bone too ([`bone_plain`](crate::sa_summon::bone_plain),
///   mark 0) before slot 211.
///
/// Returns `S + 0x163c` (the second bone's `eax` with no models).
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn summon_setup_0609<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let rf = |g: &G, at: u32| g.read::<f32>(at);
    let ebp = fr.esp.wrapping_sub(4);
    let cookie = g.read::<u32>(w(0xa)) ^ ebp;
    g.write::<u32>(ebp.wrapping_sub(4), cookie);
    let grp = fr.ecx;
    let s = g.read::<u32>(grp);
    let parent = |g: &G| g.read::<u32>(grp.wrapping_add(4));
    let p = parent(g);
    child_of(g, s, p);
    let s = g.read::<u32>(grp);
    let at = |o: u32| s.wrapping_add(o);
    let x = g.read::<u32>(grp.wrapping_add(0xc));
    g.write::<u32>(at(0x1648), x);
    let x = g.read::<u32>(grp.wrapping_add(8));
    g.write::<u32>(at(0x164c), x);
    let fl = g.read::<u8>(grp.wrapping_add(0x10)) | 2;
    g.write::<u8>(at(0x1650), fl);
    let a = g.read::<u32>(at(0x164c));
    // call width: 2
    let objs = g.call(t(g, 0x84, 218), 0, &[a, at(0x163c)]).eax;
    g.write::<u32>(at(0x1644), objs);
    let count = |g: &G| g.read::<u32>(at(0x163c)).cast_signed();
    let model = |i: u32| at(0xac + 0x278 * i);
    // the model nodes
    let args = ebp.wrapping_sub(0x94);
    let init = img.rel32_target(f + 0xe3);
    let (v1, v2) = (ebp.wrapping_sub(0x24), ebp.wrapping_sub(0x14));
    let mut i = 0u32;
    while i.cast_signed() < count(g) {
        let node = model(i);
        let p = parent(g);
        let byte = g.read::<u8>(g.read::<u32>(at(0x1644)).wrapping_add(i));
        g.write::<u32>(args, node);
        g.write::<u32>(args + 4, p);
        g.write::<u32>(args + 8, u32::from(byte));
        g.write::<u8>(args + 0xc, 0);
        g.write::<u32>(node, w(0xde));
        crate::sa_summon::model_node_init(g, img, init, args);
        for (v, op) in [(v1, 0xf4u32), (v2, 0x110)] {
            let m = g.read::<u32>(node + 0x54);
            // call width: 4
            g.call(t(g, op, 262), 0, &[m, v, v + 4, v + 8]);
        }
        let m = g.read::<u32>(node + 0x54);
        // call width: 1
        let h = st0_f32(g.call(t(g, 0x120, 12), 0, &[m]).st0);
        g.write::<f32>(v2 + 4, h);
        let d = sub(rf(g, v1 + 4), h);
        g.write::<u16>(node + 0x26e, 0);
        g.write::<f32>(node + 0x274, d);
        i += 1;
    }
    // the mean place
    let mean = at(0x15b8);
    let p = parent(g);
    child_of(g, mean, p);
    g.write::<u32>(mean, w(0x1a6));
    for o in [0u32, 4, 8] {
        g.write::<u32>(v2 + o, 0);
    }
    let mut i = 0u32;
    while i.cast_signed() < count(g) {
        let o = model(i);
        if g.read::<u8>(o + 0x90) & 1 == 0 {
            let m = g.read::<u32>(o + 0x50);
            // call width: 2
            g.call(t(g, 0x1df, 366), 0, &[m, v1]);
        }
        for j in 0..3u32 {
            let x = add(rf(g, v2 + 4 * j), rf(g, v1 + 4 * j));
            g.write::<f32>(v2 + 4 * j, x);
        }
        i += 1;
    }
    #[allow(clippy::cast_precision_loss)] // cvtdq2ps
    let q = div(k(g, 0x23c), count(g) as f32);
    for j in 0..3u32 {
        g.write::<f32>(at(0x15d4 + 4 * j), mul(rf(g, v2 + 4 * j), q));
    }
    g.write::<u32>(at(0x15e0), ONE);
    let scale = at(0x15e4);
    let p = parent(g);
    child_of(g, scale, p);
    g.write::<u32>(scale, w(0x2bc));
    for o in [0x1600u32, 0x1604, 0x1608, 0x160c] {
        g.write::<u32>(at(o), ONE);
    }
    let turn = at(0x1610);
    let p = parent(g);
    g.write::<u32>(turn, w(0x2f6));
    child_of(g, turn, p);
    for o in [0x162cu32, 0x1630, 0x1634] {
        g.write::<u32>(at(o), 0);
    }
    let bone = at(0x146c);
    let p = parent(g);
    g.write::<u32>(bone, w(0x366));
    child_of(g, bone, p);
    g.write::<u32>(bone + 0x144, turn);
    g.write::<u32>(bone + 0x13c, mean);
    g.write::<u32>(bone + 0x140, scale);
    g.write::<u32>(bone + 0x128, w(0x3d7));
    g.write::<u8>(bone + 0x148, 0);
    let bone_at = img.rel32_target(f + 0x3ed);
    let set_bone = |g: &mut G, holder: u32, node: u32, mark: u8| {
        g.write::<u32>(holder, node);
        g.write::<u8>(holder + 5, mark);
        let bf = Frame {
            at: bone_at,
            esp: ebp.wrapping_sub(0xb4),
            ecx: holder,
            ..*fr
        };
        crate::sa_summon::bone_plain(g, img, &bf).eax
    };
    set_bone(g, ebp.wrapping_sub(0x84), bone, 0);
    let m = ebp.wrapping_sub(0x68);
    // call width: 2
    g.call(t(g, 0x3fd, 327), 0, &[m, at(0x1508)]);
    let y = add(rf(g, grp + 0x14), rf(g, at(0x15d8)));
    g.write::<f32>(at(0x15d8), y);
    let mut eax = set_bone(g, ebp.wrapping_sub(0xa0), bone, 1);
    let mut i = 0u32;
    while i.cast_signed() < count(g) {
        let o = model(i);
        let ob = |off: u32| o.wrapping_add(off);
        let (na, nb, nc, nd) = (ob(0x210), ob(0x23c), ob(0x1e4), ob(0x98));
        let p = parent(g);
        child_of(g, na, p);
        let m54 = g.read::<u32>(ob(0x54));
        let pos = na + 0x1c;
        g.write::<u32>(na, w(0x4a9));
        let mut st = Pushes::default();
        st.push_all(&[pos + 8, pos + 4, pos, m54]);
        // call width: 12 (the most `Pushes::words` holds)
        st.call(g, t(g, 0x4a0, 262), 0);
        let out = at(0x2c + 0x10 * i);
        g.write::<u32>(na + 0x28, ONE);
        st.push_all(&[pos, m, out]);
        // call width: 12 (the most `Pushes::words` holds)
        st.call(g, t(g, 0x4c2, 229), 0);
        let p = parent(g);
        child_of(g, nb, p);
        let m50 = g.read::<u32>(ob(0x50));
        let mut st = Pushes::default();
        st.push_all(&[nb + 0x1c, m50]);
        g.write::<u32>(nb, w(0x521));
        // call width: 12 (the most `Pushes::words` holds)
        st.call(g, t(g, 0x51b, 290), 0);
        st.push(g.read::<u32>(ob(0x54)));
        // call width: 12 (the most `Pushes::words` holds)
        let v = st0_f32(st.call(g, t(g, 0x534, 277), 0).st0);
        g.write::<f32>(nb + 0x28, mul(v, k(g, 0x552)));
        let p = parent(g);
        g.write::<u32>(nc, w(0x566));
        child_of(g, nc, p);
        g.write::<u32>(nc + 0x24, 0);
        g.write::<u32>(nc + 0x1c, 0);
        // the model's flag is 0 here (its node set-up copied the group's
        // byte 0); the slot 333 branch is kept as the original has it
        let h = if g.read::<u8>(ob(0x90)) & 1 != 0 {
            let m = g.read::<u32>(ob(0x54));
            // call width: 1
            let h = st0_f32(g.call(t(g, 0x5b4, 333), 0, &[m]).st0);
            sub(k(g, 0x5cd), h)
        } else {
            let m = g.read::<u32>(ob(0x50));
            let hp = ebp.wrapping_sub(0x80);
            // call width: 2
            g.call(t(g, 0x5e0, 601), 0, &[m, hp]);
            sub(k(g, 0x5f6), g.read::<f32>(hp))
        };
        let tilt = div(mul(h, k(g, 0x606)), k(g, 0x60e));
        g.write::<u32>(nc + 0x20, crate::sb_sse::cvtt(tilt).cast_unsigned());
        let p = parent(g);
        g.write::<u32>(nd, w(0x62d));
        child_of(g, nd, p);
        g.write::<u32>(ob(0x1dc), nc);
        g.write::<u32>(ob(0x1d8), nb);
        g.write::<u32>(ob(0x1d4), na);
        g.write::<u32>(ob(0x1c0), bone);
        g.write::<u8>(ob(0x1e0), 0);
        set_bone(g, ebp.wrapping_sub(0x8c), nd, 0);
        let m54 = g.read::<u32>(ob(0x54));
        // call width: 2
        g.call(t(g, 0x6a8, 211), 0, &[m54, ob(0x134)]);
        let fl = g.read::<u16>(ob(0x90));
        g.write::<u16>(ob(0x90), fl | 0x10);
        g.write::<u32>(ob(0x4c), nd);
        eax = at(0x163c);
        i += 1;
    }
    Ret::eax(eax)
}

/// [`summon_setup_0609`] (the `rel32`s: the model node set-up, the bone
/// three times, `__security_check_cookie`).
pub const SUMMON_SETUP_0609: Sig = Sig {
    len: 1787,
    hash: 0xde8f_79fc_2040_fac0,
    calls: &[
        (0xe3, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0x3ed, Callee::Sig(&crate::sa_summon::BONE)),
        (0x43b, Callee::Sig(&crate::sa_summon::BONE)),
        (0x69c, Callee::Sig(&crate::sa_summon::BONE)),
        (0x6f3, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0601`'s screen task (0x100087b0, 1941 bytes; a task callback
/// `(A)`, cdecl; an aligned frame `E`). `S = A[1]`; each run moves `S +
/// 0x18`, `S + 0x1c` by `K`, counts `S + 0x20` up, then runs the state
/// `*G_STATE` (byte and jump tables after the code):
/// - 60: the screen's slots (530, 719, 306, 238, 296, 295), the flag call
///   ([`call_211_flag`](crate::s4_small::call_211_flag)), its nodes
///   (appended and inserted: [`node_append`](crate::sa_misc::node_append),
///   [`insert_with`](crate::sa_misc::insert_with), [`node_append`](crate::sb_misc::node_append)
///   twice, [`node_build_0601`](crate::r22::node_build_0601), the colour
///   node, the camera move ([`move_start`](crate::sb_misc::move_start)),
///   a node insert), the camera words by slot 520's answer, every actor of
///   group 0xb hidden (slot 296), on;
/// - 61: a linked node and the static node, on; 62: `S + 0x368` down by
///   `K`, on; 63: the globals' flags and colours, `S + 0x368` down twice,
///   on; 64: `S + 0x368` down; 70: two blocks and the camera words, on;
///   71: once `S + 0x1c` reaches `K`, the model flag slot, slot 228 and
///   the screen set-up ([`screen_setup_0601`]), on; 80, 81, 111, 112 (after
///   75 frames): flag and colour words, on; 110: slots 286 and 250, the
///   second camera move ([`move_start_2`](crate::sb_misc::move_start_2)),
///   on;
/// - 10000: the current list node unlinked, `S + 0x374` leaves its parent
///   and the task's node `*A` leaves its ring.
///
/// Returns `eax` as the original leaves it (a call's answer, the state's
/// table byte or a word read).
#[allow(clippy::too_many_lines)] // the original's states, one by one
pub fn screen_task_0601<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let rf = |g: &G, at: u32| g.read::<f32>(at);
    // `mov dword [G], imm32` at each listed offset (the address, the
    // immediate four bytes on)
    let abs = |g: &mut G, ops: &[u32]| {
        for &o in ops {
            g.write::<u32>(w(o), w(o + 4));
        }
    };
    let gstate = w(0x8);
    let state = g.read::<u32>(gstate);
    let a = fr.arg(g, 0);
    let s = g.read::<u32>(a.wrapping_add(4));
    let at = |o: u32| s.wrapping_add(o);
    let x18 = add(rf(g, at(0x18)), k(g, 0x22));
    let n20 = g.read::<u32>(at(0x20)).wrapping_add(1);
    g.write::<u32>(at(0x20), n20);
    g.write::<f32>(at(0x18), x18);
    let x1c = add(rf(g, at(0x1c)), k(g, 0x37));
    g.write::<f32>(at(0x1c), x1c);
    let next = |g: &mut G| g.write::<u32>(gstate, state.wrapping_add(1));
    let flags3 = |g: &mut G, op: u32| {
        let x = g.read::<u32>(w(op)) | 3;
        g.write::<u32>(w(op), x);
    };
    // push ebp; and esp, -8; sub esp, 0x20; two pushes
    let e = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x28);
    if state == 0x2710 {
        let unlink = img.rel32_target(f + 0x6fb);
        let eax = crate::l3_common::list_unlink_current(g, img, unlink, state);
        crate::sa_misc::leave(g, at(0x374), eax);
        let node = g.read::<u32>(a);
        let (nx, pv) = (g.read::<u32>(node + 4), g.read::<u32>(node + 8));
        g.write::<u32>(nx + 8, pv);
        let (pv, nx) = (g.read::<u32>(node + 8), g.read::<u32>(node + 4));
        g.write::<u32>(pv + 4, nx);
        g.write::<u32>(node + 8, node);
        g.write::<u32>(node + 4, node);
        return Ret::eax(nx);
    }
    let idx = state.wrapping_sub(0x3c);
    if state.cast_signed() > 0x2710 || idx > 0x34 {
        return Ret::eax(if state.cast_signed() > 0x2710 {
            state
        } else {
            idx
        });
    }
    // the byte table after the code (read from the image)
    let byte = img.word(w(0x60).wrapping_add(idx)) & 0xff;
    let step = k(g, 0x68);
    let eax = match state {
        60 => {
            let ga = g.read::<u32>(w(0x7f));
            // call width: 2
            g.call(t(g, 0x74, 530), 0, &[ga, 0x100]);
            // call width: 3
            let name = g.call(t(g, 0x8c, 719), 0, &[w(0x9b), 5, 0]).eax;
            // call width: 3
            g.call(t(g, 0xa6, 306), 0, &[7, 2, name]);
            let mut st = Pushes::default();
            for (op, slot, x, ga_op) in [
                (0xb7u32, 238u32, 1u32, 0xc8u32),
                (0xcf, 296, 0, 0xd7),
                (0xe4, 295, 0, 0xec),
            ] {
                let ga = g.read::<u32>(w(ga_op));
                st.push_all(&[x, ga]);
                // call width: 12 (the most `Pushes::words` holds)
                st.call(g, t(g, op, slot), 0);
            }
            let flag = Frame {
                at: img.rel32_target(f + 0xf9),
                esp: e.wrapping_sub(0x1c),
                ..*fr
            };
            let eax = crate::s4_small::call_211_flag(g, img, &flag).eax;
            let p1 = w(0x104);
            let n1 = at(0x2c4);
            crate::sa_misc::node_append(g, img, img.rel32_target(f + 0x109), n1, p1, eax);
            for o in [0x2e0u32, 0x2e4, 0x2e8] {
                g.write::<u32>(at(o), 0);
            }
            for (o, v) in [
                (0x1cu32, w(0x10e)),
                (0x20, n1),
                (0x24, w(0x114)),
                (0x28, w(0x119)),
            ] {
                g.write::<u32>(e.wrapping_sub(o), v);
            }
            let ins = Frame {
                at: img.rel32_target(f + 0x142),
                esp: e.wrapping_sub(0x2c),
                ecx: at(0x178),
                edx: p1,
                ..*fr
            };
            crate::sa_misc::insert_with(g, img, &ins);
            let p2 = w(0x14d);
            let (n2, n3) = (at(0x31c), at(0x348));
            g.write::<u8>(at(0x2c0), 1);
            crate::sb_misc::node_append(g, img, img.rel32_target(f + 0x15b), n2, p2, 0);
            g.write::<u32>(at(0x344), ONE);
            crate::sb_misc::node_append(g, img, img.rel32_target(f + 0x170), n3, p2, 0);
            g.write::<u32>(at(0x370), ONE);
            // push esi; push ecx; push ecx; sub esp, 0x10
            for (o, v) in [(8u32, n2), (0xc, n3), (0x10, n3)] {
                g.write::<u32>(e.wrapping_sub(o), v);
            }
            let build = Frame {
                at: img.rel32_target(f + 0x18d),
                esp: e.wrapping_sub(0x24),
                ecx: w(0x188),
                ..*fr
            };
            crate::r22::node_build_0601(g, img, &build);
            abs(g, &[0x19b]);
            let gb = g.read::<u32>(w(0x1b2));
            // call width: 2
            g.call(t(g, 0x192, 286), 0, &[gb, w(0x1ac)]);
            let grp = e + 8;
            g.write::<u32>(grp, w(0x1c3));
            g.write::<u32>(grp + 4, w(0x1cb));
            abs(g, &[0x1d1]);
            let colour = Frame {
                at: img.rel32_target(f + 0x1da),
                esp: e.wrapping_sub(4),
                ecx: grp,
                ..*fr
            };
            let eax = crate::s3_misc::colour_node(g, img, &colour);
            abs(g, &[0x1e0, 0x1ea, 0x1f4, 0x1fe, 0x208, 0x21a, 0x224]);
            crate::sb_misc::move_start(g, img, img.rel32_target(f + 0x22d), k(g, 0x214), eax);
            let node = at(0x2c);
            let grp = e + 0x10;
            for (i, v) in (0u32..).zip([node, w(0x240), w(0x248), w(0x250), w(0x258), w(0x260)]) {
                g.write::<u32>(grp + 4 * i, v);
            }
            let ins = Frame {
                at: img.rel32_target(f + 0x265),
                ecx: grp,
                ..*fr
            };
            crate::nodes::node_insert(g, img, &ins);
            g.write::<u32>(w(0x270), node);
            g.write::<u32>(at(0x364), 0);
            g.write::<u32>(at(0x368), 0x41f0_0000);
            g.write::<u32>(at(0x36c), 0);
            g.write::<u32>(at(0x2bc), w(0x298));
            abs(g, &[0x29e]);
            // call width: 0
            let kind = g.call(t(g, 0x26a, 520), 0, &[]).eax;
            g.write::<u32>(at(0x338), 0x41f0_0000);
            let (y, z) = if kind == 0x0205_0000 {
                (0xc2b4_0000, 0xc28c_0000)
            } else {
                (0xc28c_0000, 0xc348_0000)
            };
            g.write::<u32>(at(0x33c), y);
            g.write::<u32>(at(0x340), z);
            let count = e + 8;
            // call width: 2
            let mut eax = g.call(t(g, 0x2ef, 288), 0, &[0xb, count]).eax;
            let list = eax;
            let mut i = 0u32;
            while i.cast_signed() < g.read::<u32>(count).cast_signed() {
                let b = u32::from(g.read::<u8>(list.wrapping_add(i)));
                // call width: 2
                eax = g.call(t(g, 0x314, 296), 0, &[b, 0]).eax;
                i += 1;
            }
            abs(g, &[0x332]);
            let x = g.read::<u32>(w(0x33c)).wrapping_add(1);
            g.write::<u32>(w(0x33c), x);
            eax
        }
        61 => {
            let link = Frame {
                at: img.rel32_target(f + 0x34d),
                ecx: at(0x374),
                ..*fr
            };
            crate::s4_small::link_node(g, img, &link);
            g.write::<u32>(at(0x394), ONE);
            g.write::<u8>(at(0x390), 0x30);
            let eax = crate::sa_misc::static_node(g, img, img.rel32_target(f + 0x366));
            next(g);
            eax
        }
        62 => {
            let x = sub(rf(g, at(0x368)), k(g, 0x386));
            next(g);
            g.write::<f32>(at(0x368), x);
            byte
        }
        63 | 64 => {
            if state == 63 {
                flags3(g, 0x3a1);
                g.write::<u8>(w(0x3a8), 0);
                abs(
                    g,
                    &[
                        0x3af, 0x3b9, 0x3c3, 0x3cd, 0x3d7, 0x3e1, 0x3eb, 0x3f5, 0x3ff, 0x409,
                        0x413, 0x41d,
                    ],
                );
                let x = sub(rf(g, at(0x368)), step);
                next(g);
                g.write::<f32>(at(0x368), x);
            }
            let x = sub(rf(g, at(0x368)), step);
            g.write::<f32>(at(0x368), x);
            byte
        }
        70 => {
            g.write::<u32>(at(0x154), w(0x460));
            g.write::<u32>(at(0x168), w(0x46a));
            for (o, v) in [
                (0x368u32, 0u32),
                (0x338, 0x41f0_0000),
                (0x33c, 0xc1a0_0000),
                (0x340, 0xc2a0_0000),
                (0x1c, 0),
            ] {
                g.write::<u32>(at(o), v);
            }
            next(g);
            byte
        }
        71 => {
            // comiss; jb: below or unordered waits
            if x1c
                .partial_cmp(&k(g, 0x4ad))
                .is_none_or(core::cmp::Ordering::is_lt)
            {
                return Ret::eax(byte);
            }
            let pick = Frame {
                at: img.rel32_target(f + 0x4b8),
                ..*fr
            };
            crate::o4_single::flag_pick_slot(g, img, &pick);
            let who = u32::from(g.read::<u8>(w(0x4bf)));
            // call width: 2
            g.call(t(g, 0x4ca, 228), 0, &[who, 0xb8]);
            let setup = Frame {
                at: img.rel32_target(f + 0x4e0),
                esp: e.wrapping_sub(4),
                ecx: at(0x39c),
                ..*fr
            };
            let eax = screen_setup_0601(g, img, &setup).eax;
            let x = g.read::<u32>(w(0x4e6)).wrapping_add(1);
            g.write::<u32>(w(0x4e6), x);
            eax
        }
        80 => {
            flags3(g, 0x4f2);
            abs(g, &[0x4fa, 0x504, 0x50e, 0x518, 0x522, 0x52c]);
            next(g);
            byte
        }
        81 => {
            next(g);
            byte
        }
        110 => {
            let gb = g.read::<u32>(w(0x555));
            // call width: 2
            g.call(t(g, 0x541, 286), 0, &[gb, 0x4480_0000]);
            // call width: 2
            g.call(t(g, 0x55c, 250), 0, &[0, 0]);
            let (x3, x2) = (k(g, 0x57b), k(g, 0x583));
            abs(g, &[0x589]);
            for (o, v) in [(0x364u32, 0u32), (0x368, 0xc28c_0000), (0x36c, 0)] {
                g.write::<u32>(at(o), v);
            }
            abs(g, &[0x5b1]);
            for (o, v) in [
                (0x338u32, 0x4248_0000u32),
                (0x33c, 0xc316_0000),
                (0x340, 0xc2c8_0000),
            ] {
                g.write::<u32>(at(o), v);
            }
            abs(g, &[0x5dc, 0x5e6, 0x5f0, 0x5fa, 0x604, 0x60e, 0x618, 0x622]);
            let eax = crate::sb_misc::move_start_2(g, img, img.rel32_target(f + 0x62b), x2, x3);
            let x = g.read::<u32>(w(0x631)).wrapping_add(1);
            g.write::<u32>(w(0x631), x);
            eax
        }
        111 | 112 => {
            if state == 111 {
                flags3(g, 0x63d);
                abs(
                    g,
                    &[
                        0x644, 0x64e, 0x658, 0x662, 0x66c, 0x676, 0x680, 0x68a, 0x694,
                    ],
                );
                g.write::<u8>(at(0x390), 0);
                g.write::<u32>(at(0x20), 0);
                next(g);
            }
            if g.read::<u32>(at(0x20)) < 0x4b {
                return Ret::eax(byte);
            }
            flags3(g, 0x6b9);
            abs(g, &[0x6c1, 0x6cb, 0x6d5, 0x6df]);
            g.write::<u32>(at(0x20), 0);
            next(g);
            byte
        }
        _ => byte,
    };
    Ret::eax(eax)
}

/// [`screen_task_0601`] (the `rel32`s: its many node and slot helpers).
pub const SCREEN_TASK_0601: Sig = Sig {
    len: 1941,
    hash: 0xb41e_11b4_e28c_b2bd,
    calls: &[
        (0xf9, Callee::Sig(&crate::s4_small::CALL_211_FLAG)),
        (0x109, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0x142, Callee::Sig(&crate::sa_misc::INSERT_WITH)),
        (0x15b, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x170, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x18d, Callee::Sig(&crate::r22::NODE_BUILD_0601)),
        (0x1da, Callee::Sig(&crate::s3_misc::COLOUR_NODE)),
        (0x22d, Callee::Sig(&crate::sb_misc::MOVE_START)),
        (0x265, Callee::Sig(&crate::nodes::INSERT)),
        (0x34d, Callee::Sig(&crate::s4_small::LINK_NODE)),
        (0x366, Callee::Sig(&crate::sa_misc::STATIC_NODE)),
        (
            0x4b8,
            Callee::Sig(&crate::o4_single::FLAG_PICK_SLOT_0601_8070),
        ),
        (0x4e0, Callee::Sig(&SCREEN_SETUP_0601)),
        (0x62b, Callee::Sig(&crate::sb_misc::MOVE_START_2)),
        (0x6fb, Callee::Sig(&crate::l3_common::LIST_UNLINK_CURRENT)),
        (0x706, Callee::Sig(&crate::sa_misc::LEAVE)),
    ],
    imports: &[],
};

/// `magic_0601`'s summon task (0x10008f50, 1932 bytes; a task callback
/// `(A)`, cdecl; an aligned frame `E` below three pushes). `S = A[1]`, the
/// state `S + 0x28e8`, the frame count `S + 0x28f8` (up by one each run),
/// the timers `S + 0x28ec`, `S + 0x28f0` (up by `K`). The states:
/// - 0 becomes 0x36; 0x36: the summon's light (slots 26, 0 into `*G_L`,
///   212, 30, 10, 257, 211, 257, 37, 670), `S` a child of its list, slots
///   296, 37, 670, 211, a scale, on to 0x37 at once;
/// - 0x37: two nodes under the parent (slot 325, a height), a node insert
///   and a particle node ([`particle_node`](crate::sb_nodes::particle_node)),
///   on to 0x38; 0x38: a node insert, globals, slots 37 and 670, two
///   effect nodes ([`effect_node`](crate::sb_nodes::effect_node)) with
///   their vtables and flags, slot 222, on to 0x39;
/// - 0x39 and 0x3a wait for the timer `S + 0x28f0` (`G_STATE` 0x46 in
///   between), then `S + 0x24`, `+0x20` = `(-0.4, -1)` and 0x46;
/// - 0x46: slot 546, 0x47 (`G_STATE` 0x50); 0x47 after 178 frames: slot 37,
///   the light released ([`release`](crate::m0042::release)), an effect
///   record ([`effect_node_b`](crate::s4_fam446::effect_node_b)), its
///   flags, 0x50; 0x50 after 125 frames, or 0x2710: 0x2711 (`G_STATE`
///   0x2710) at once;
/// - 0x2711: slot 30, the current list node unlinked, the model set up
///   ([`model_setup`](crate::sa_summon::model_setup)) and the task's node
///   `*A` off its ring.
///
/// Returns `eax` as the original leaves it.
#[allow(clippy::too_many_lines)] // the original's states, one by one
pub fn summon_task_0601<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sa_misc::child_of_0512 as child;
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let rf = |g: &G, at: u32| g.read::<f32>(at);
    let call = |o: u32| img.rel32_target(f + o);
    let abs = |g: &mut G, ops: &[u32]| {
        for &o in ops {
            g.write::<u32>(w(o), w(o + 4));
        }
    };
    let flags7 = |g: &mut G, rd: u32, clr: u32, st: u32| {
        let x = g.read::<u32>(w(rd)) | 7;
        g.write::<u32>(w(clr), 0);
        g.write::<u32>(w(st), x);
        x
    };
    let a = fr.arg(g, 0);
    let s = g.read::<u32>(a.wrapping_add(4));
    let at = |o: u32| s.wrapping_add(o);
    // push ebp; and esp, -8; sub esp, 0x1c; three pushes
    let e = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x28);
    let x = add(rf(g, at(0x28ec)), k(g, 0x1e));
    let n = g.read::<u32>(at(0x28f8)).wrapping_add(1);
    g.write::<u32>(at(0x28f8), n);
    g.write::<f32>(at(0x28ec), x);
    let x = add(rf(g, at(0x28f0)), k(g, 0x48));
    g.write::<f32>(at(0x28f0), x);
    let mut state = g.read::<u32>(at(0x28e8));
    let up = |g: &mut G| {
        let x = g.read::<u32>(at(0x28e8)).wrapping_add(1);
        g.write::<u32>(at(0x28e8), x);
    };
    let group = |g: &mut G, base: u32, words: &[u32]| {
        for (i, v) in (0u32..).zip(words) {
            g.write::<u32>(base + 4 * i, *v);
        }
    };
    let insert = |g: &mut G, op: u32| {
        let ins = Frame {
            at: call(op),
            ecx: e + 0x10,
            ..*fr
        };
        crate::nodes::node_insert(g, img, &ins);
    };
    if state == 0 {
        g.write::<u32>(at(0x28e8), 0x36);
        state = 0x36;
    }
    let gl = |g: &G, op: u32| g.read::<u32>(w(op));
    let mut eax = state;
    // the states that run on into the next (0x36 to 0x39)
    let ran_on = (0x36..=0x38).contains(&state);
    if ran_on {
        if state == 0x36 {
            // call width: 2
            g.call(t(g, 0x7f, 26), 0, &[0x300f, 1]);
            // call width: 1
            let light = g.call(t(g, 0x90, 0), 0, &[0x300f]).eax;
            g.write::<u32>(w(0xa1), light);
            // call width: 2
            g.call(t(g, 0xa9, 212), 0, &[light, 0]);
            let l = gl(g, 0xbc);
            // call width: 1
            g.call(t(g, 0xb6, 30), 0, &[l]);
            let mut st = Pushes::default();
            let l = gl(g, 0xd4);
            st.push_all(&[0, l]);
            // call width: 12 (the most `Pushes::words` holds)
            st.call(g, t(g, 0xc6, 10), 0);
            let l = gl(g, 0xe3);
            st.push_all(&[1, l]);
            // call width: 12 (the most `Pushes::words` holds)
            st.call(g, t(g, 0xdb, 257), 0);
            let l = gl(g, 0xfb);
            st.push_all(&[w(0xf5), l]);
            // call width: 12 (the most `Pushes::words` holds)
            st.call(g, t(g, 0xf0, 211), 0);
            let l = gl(g, 0x110);
            st.push_all(&[0, l]);
            abs(g, &[0x116, 0x120, 0x12a]);
            // call width: 12 (the most `Pushes::words` holds)
            st.call(g, t(g, 0x108, 257), 0);
            let l = gl(g, 0x14e);
            st.push_all(&[0xb2, 1, l]);
            // call width: 12 (the most `Pushes::words` holds)
            st.call(g, t(g, 0x13b, 37), 0);
            let l = gl(g, 0x166);
            // call width: 2
            g.call(t(g, 0x155, 670), 0, &[l, 0]);
            g.write::<u32>(s, w(0x16e));
            child(
                g,
                img,
                f,
                s,
                [0x181, 0x188, 0x18d, 0x195, 0x19d, 0x1a5, 0x1b1, 0x1bc],
            );
            let ga = gl(g, 0x1c4);
            for (o, v) in [(0x1cu32, 0u32), (0x20, 0x4040_0000), (0x24, 0), (0x28, 0)] {
                g.write::<u32>(at(o), v);
            }
            let mut st = Pushes::default();
            st.push_all(&[1, ga]);
            // call width: 12 (the most `Pushes::words` holds)
            st.call(g, t(g, 0x1b6, 296), 0);
            let l = gl(g, 0x200);
            st.push_all(&[0xc9, 1, l]);
            // call width: 12 (the most `Pushes::words` holds)
            st.call(g, t(g, 0x1ed, 37), 0);
            let mut st = Pushes::default();
            let l = gl(g, 0x218);
            st.push_all(&[0, l]);
            // call width: 12 (the most `Pushes::words` holds)
            st.call(g, t(g, 0x207, 670), 0);
            let l = gl(g, 0x22a);
            st.push_all(&[w(0x224), l]);
            // call width: 12 (the most `Pushes::words` holds)
            st.call(g, t(g, 0x21f, 211), 0);
            let x = mul(gl_f(g, w(0x23a)), k(g, 0x242));
            g.write::<f32>(w(0x24d), x);
            up(g);
            g.write::<u32>(at(0x28f0), 0);
            g.write::<u32>(at(0x28f8), 0);
        }
        if state <= 0x37 {
            let n1 = at(0xd1c);
            child(
                g,
                img,
                f,
                n1,
                [0x280, 0x287, 0x28c, 0x294, 0x299, 0x2a1, 0x2cf, 0x2e6],
            );
            g.write::<u32>(n1, w(0x2ec));
            // call width: 2
            g.call(t(g, 0x2e0, 325), 0, &[at(0xd38), w(0x2da)]);
            let y = sub(rf(g, at(0xd3c)), k(g, 0x304));
            g.write::<f32>(at(0xd3c), y);
            let n2 = at(0xd48);
            child(
                g,
                img,
                f,
                n2,
                [0x328, 0x32f, 0x334, 0x33c, 0x341, 0x349, 0x355, 0x35b],
            );
            g.write::<u32>(n2, w(0x361));
            for (o, v) in [
                (0xd64u32, ONE),
                (0xd68, ONE),
                (0xd6c, ONE),
                (0xd70, 0x3eb3_3333),
            ] {
                g.write::<u32>(at(o), v);
            }
            let base = at(0xbd0);
            group(g, e + 0x10, &[base, w(0x3a3), w(0x3ab), n1, w(0x3b7), n2]);
            insert(g, 0x3bc);
            let x = gl(g, 0x3c1);
            let pn = at(0xad0);
            g.write::<u32>(pn, w(0x3d1));
            group(g, e + 0x10, &[pn, w(0x3dd), base, x, w(0x3ed), 2]);
            crate::sb_nodes::particle_node(g, img, call(0x3fa), e + 0x10);
            up(g);
        }
        group(
            g,
            e + 0x10,
            &[w(0x40c), w(0x414), w(0x41c), w(0x424), w(0x42c), w(0x434)],
        );
        insert(g, 0x439);
        abs(g, &[0x444, 0x44e]);
        g.write::<u32>(at(0x20), 0);
        let x = mul(gl_f(g, w(0x461)), k(g, 0x469));
        let l = gl(g, 0x476);
        g.write::<f32>(w(0x47e), x);
        let y = mul(gl_f(g, w(0x486)), k(g, 0x48e));
        abs(g, &[0x494]);
        g.write::<f32>(w(0x4a0), y);
        let z = mul(gl_f(g, w(0x4a8)), k(g, 0x4b0));
        g.write::<f32>(w(0x4b8), z);
        // call width: 3
        g.call(t(g, 0x43e, 37), 0, &[l, 1, 0xb1]);
        let l = gl(g, 0x4d6);
        let mut st = Pushes::default();
        st.push_all(&[0, l]);
        // call width: 12 (the most `Pushes::words` holds)
        st.call(g, t(g, 0x4c5, 670), 0);
        let base = at(0xbd0);
        g.write::<u32>(at(0x28f8), 0);
        for (ops, last, vt, (rd, clr, sto)) in [
            (
                [0x4f4u32, 0x4fc, 0x508, 0x510],
                0u32,
                0x52bu32,
                (0x530u32, 0x53cu32, 0x545u32),
            ),
            (
                [0x54d, 0x555, 0x561, 0x569],
                1,
                0x582,
                (0x587, 0x59f, 0x592),
            ),
        ] {
            let grp = e + 0x10;
            group(
                g,
                grp,
                &[w(ops[0]), w(ops[1]), base, w(ops[2]), w(ops[3]), last],
            );
            let at_call = if last == 0 { 0x51d } else { 0x576 };
            crate::sb_nodes::effect_node(g, img, call(at_call), grp);
            let node = g.read::<u32>(grp);
            g.write::<u32>(node, w(vt));
            flags7(g, rd, clr, sto);
        }
        st.push_all(&[2, 0xc, 0xc, w(0x5b0)]);
        // call width: 12 (the most `Pushes::words` holds)
        eax = st.call(g, t(g, 0x597, 222), 0).eax;
        up(g);
        g.write::<u32>(at(0x28f0), 0);
        state = 0x39;
    }
    if state == 0x39 || state == 0x3a {
        // entered here: the jump table's index (2, 3) is `eax`
        if !ran_on {
            eax = state - 0x37;
        }
        if state == 0x39 {
            // comiss K, timer; ja: above waits
            if k(g, 0x5cd) > rf(g, at(0x28f0)) {
                return Ret::eax(eax);
            }
            up(g);
            g.write::<u32>(w(0x5e6), 0x46);
        }
        if k(g, 0x5f2) > rf(g, at(0x28f0)) {
            return Ret::eax(eax);
        }
        g.write::<u32>(at(0x24), 0xbecc_cccd);
        g.write::<u32>(at(0x20), 0xbf80_0000);
        g.write::<u32>(at(0x28e8), 0x46);
        return Ret::eax(eax);
    }
    if (0x3b..=0x45).contains(&state) {
        return Ret::eax(state - 0x37);
    }
    match state {
        0x46 => {
            let l = gl(g, 0x629);
            // call width: 1
            let eax = g.call(t(g, 0x623, 546), 0, &[l]).eax;
            up(g);
            g.write::<u32>(w(0x640), 0x50);
            g.write::<u32>(at(0x28f0), 0);
            Ret::eax(eax)
        }
        0x47 => {
            if n.cast_signed() < 0xb2 {
                return Ret::eax(state);
            }
            let l = gl(g, 0x6bc);
            g.write::<u32>(w(0x6c2), 0x6e);
            // call width: 3
            g.call(t(g, 0x6a9, 37), 0, &[l, 1, 0xc8]);
            let l = gl(g, 0x6ce);
            crate::m0042::release(g, img, call(0x6d6), l);
            // push 2; sub esp, 8; push the base
            g.write::<u32>(e.wrapping_sub(4), 2);
            g.write::<u32>(e.wrapping_sub(0x10), at(0xbd0));
            let rec = Frame {
                at: call(0x6ec),
                esp: e.wrapping_sub(0x14),
                ecx: w(0x6e7),
                ..*fr
            };
            crate::s4_fam446::effect_node_b(g, img, &rec);
            let x = flags7(g, 0x6f1, 0x700, 0x709);
            g.write::<u32>(at(0x28e8), 0x50);
            g.write::<u32>(at(0x28f8), 0);
            Ret::eax(x)
        }
        0x50 | 0x2710 | 0x2711 => {
            if state == 0x50 && n.cast_signed() <= 0x7d {
                return Ret::eax(state);
            }
            if state != 0x2711 {
                g.write::<u32>(at(0x28e8), 0x2711);
                g.write::<u32>(w(0x68f), 0x2710);
            }
            let l = gl(g, 0x736);
            // call width: 1
            let eax = g.call(t(g, 0x730, 30), 0, &[l]).eax;
            crate::l3_common::list_unlink_current(g, img, call(0x743), eax);
            let holder = e + 0xc;
            g.write::<u32>(holder, w(0x74f));
            let ms = Frame {
                at: call(0x754),
                ecx: holder,
                ..*fr
            };
            crate::sa_summon::model_setup(g, img, &ms);
            let node = g.read::<u32>(a);
            let (nx, pv) = (g.read::<u32>(node + 4), g.read::<u32>(node + 8));
            g.write::<u32>(nx + 8, pv);
            let (pv, nx) = (g.read::<u32>(node + 8), g.read::<u32>(node + 4));
            g.write::<u32>(pv + 4, nx);
            g.write::<u32>(node + 8, node);
            g.write::<u32>(node + 4, node);
            Ret::eax(nx)
        }
        _ => Ret::eax(state),
    }
}

/// An `f32` global (`movss xmm0, [G]`).
fn gl_f<G: Game>(g: &G, at: u32) -> f32 {
    g.read::<f32>(at)
}

/// [`summon_task_0601`] (the `rel32`s: its node helpers).
pub const SUMMON_TASK_0601: Sig = Sig {
    len: 1932,
    hash: 0xaa93_b5ac_16e9_b801,
    calls: &[
        (0x3bc, Callee::Sig(&crate::nodes::INSERT)),
        (0x3fa, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x439, Callee::Sig(&crate::nodes::INSERT)),
        (0x51d, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
        (0x576, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
        (0x6d6, Callee::Sig(&crate::m0042::RELEASE)),
        (0x6ec, Callee::Sig(&crate::s4_fam446::EFFECT_NODE_B)),
        (0x743, Callee::Sig(&crate::l3_common::LIST_UNLINK_CURRENT)),
        (0x754, Callee::Sig(&crate::sa_summon::MODEL_SETUP)),
    ],
    imports: &[],
};

/// A static node appended under the parent as `magic_0601`'s start-up
/// inlines it, the last child already read (`old`): the node's `+8` =
/// `old` (the store at operand `n8`); an empty list (`old` is the parent,
/// operand `cmp`) gets it as its first (the store run at `eq`), else
/// `old + 4` is it (the immediate `ne`); the count up (operand `count`).
pub(crate) fn append_0601<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    old: u32,
    [n8, cmp, eq, ne, count]: [u32; 5],
) {
    let w = |o: u32| img.word(f + o);
    g.write::<u32>(w(n8), old);
    if old == w(cmp) {
        crate::r25::abs_stores(g, img, f, eq, eq + 10);
    } else {
        g.write::<u32>(old.wrapping_add(4), w(ne));
    }
    let n = g.read::<u16>(w(count)).wrapping_add(1);
    g.write::<u16>(w(count), n);
}

/// Two static nodes appended one after the other as the start-up inlines
/// them (the second's old last known to be the first): the first as
/// [`append_0601`] without its count, the store run `run` (the first's
/// words, the second's own words with its `+8` the first), the first
/// compared with the parent (`cmp2`: the parent's first `eq2`, else the
/// store run at `ne2`), the count up by two (read at `count`, stored at
/// `count_w`).
pub(crate) fn append_pair_0601<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    old: u32,
    [n8, cmp, eq, ne, count]: [u32; 5],
    (first, run, (cmp2, eq2, ne2), count_w): (u32, (u32, u32), (u32, u32, u32), u32),
) {
    use crate::r25::abs_stores;
    let w = |o: u32| img.word(f + o);
    g.write::<u32>(w(n8), old);
    if old == w(cmp) {
        abs_stores(g, img, f, eq, eq + 10);
    } else {
        g.write::<u32>(old.wrapping_add(4), w(ne));
    }
    let n = g.read::<u16>(w(count)).wrapping_add(2);
    abs_stores(g, img, f, run.0, run.1);
    if w(first) == w(cmp2) {
        abs_stores(g, img, f, eq2, eq2 + 10);
    } else {
        abs_stores(g, img, f, ne2, ne2 + 10);
    }
    g.write::<u16>(w(count_w), n);
}

/// `magic_0601`'s start-up (0x100096e0, 3266 bytes; a table entry, cdecl
/// `(actor)`; an aligned frame `E` below `push esi`). The targets
/// ([`targets_init_b`](crate::s4_summon::targets_init_b)); the DLL's parts
/// and buffers (slots 718, 719, 711 with `*G_P` set to 1 around it, 710,
/// 222, 221, 551); the statics' words and the nodes of its scene, each
/// appended to the static parent with its words ([`append_0601`],
/// [`append_pair_0601`]) between slots 531, 325, 215, 290, 601 (a tilt,
/// `(K - v) K' / K''` truncated), 27, 4, 281 and 694, node inserts
/// ([`node_insert`](crate::nodes::node_insert)), a node init
/// ([`node_init`](crate::m0042::node_init)), the static node
/// ([`static_node`](crate::m0601::static_node)) and a model node
/// ([`model_node_init`](crate::sa_summon::model_node_init)); the two task
/// nodes (the summon and screen tasks) put on the static list, the state
/// 0x3c. Returns 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn start_up_0601<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r25::abs_stores;
    use crate::sb_sse::{div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let call = |o: u32| img.rel32_target(f + o);
    let runs = |g: &mut G, rs: &[(u32, u32)]| {
        for &(a, b) in rs {
            abs_stores(g, img, f, a, b);
        }
    };
    let actor = fr.arg(g, 0);
    // push ebp; and esp, -8; sub esp, 0x1c; push esi
    let e = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x20);
    crate::s4_summon::targets_init_b(g, img, call(0xe), e.wrapping_sub(4), actor);
    abs_stores(g, img, f, 0x22, 0x2c);
    // call width: 1
    g.call(t(g, 0x13, 718), 0, &[w(0x18)]);
    // call width: 1
    g.call(t(g, 0x2f, 718), 0, &[w(0x3d)]);
    // call width: 3
    let part = g.call(t(g, 0x44, 719), 0, &[w(0x56), 8, 0]).eax;
    let gp = w(0x5e);
    let pp = g.read::<u32>(gp);
    let saved = g.read::<u32>(pp);
    g.write::<u32>(w(0x6a), part);
    g.write::<u32>(pp, 1);
    // call width: 3
    g.call(t(g, 0x75, 711), 0, &[part, w(0x6f), 0]);
    let pp = g.read::<u32>(w(0x88));
    g.write::<u32>(pp, saved);
    let x = gw(g, 0xa8);
    let mut st = Pushes::default();
    st.push_all(&[0x10_0000, w(0xa2), x, w(0xad)]);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, t(g, 0x92, 710), 0);
    st.push_all(&[2, 0xb, 0, w(0xc5)]);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, t(g, 0xb4, 222), 0);
    st.push_all(&[2, 0x3e8, 0, w(0xe0)]);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, t(g, 0xcc, 221), 0);
    st.push_all(&[1, 6, w(0xf6)]);
    // call width: 12 (the most `Pushes::words` holds)
    let r2 = st.call(g, t(g, 0xe7, 719), 0).eax;
    // call width: 3
    g.call(t(g, 0x105, 551), 0, &[r2, 0, 2]);
    // call width: 3
    let r3 = g.call(t(g, 0x112, 719), 0, &[w(0x124), 6, 2]).eax;
    // call width: 3
    g.call(t(g, 0x133, 551), 0, &[r3, 1, 2]);
    // the statics' words, slot 531
    abs_stores(g, img, f, 0x147, 0x151);
    let x = gw(g, 0x158);
    abs_stores(g, img, f, 0x15c, 0x27a);
    // call width: 2
    g.call(t(g, 0x140, 531), 0, &[x, w(0x152)]);
    // the scene's nodes
    let old = gw(g, 0x283);
    runs(g, &[(0x28a, 0x294), (0x294, 0x2c5)]);
    append_0601(g, img, f, old, [0x2c6, 0x2cb, 0x2d1, 0x2e0, 0x2ec]);
    abs_stores(g, img, f, 0x2f0, 0x304);
    // call width: 2
    g.call(t(g, 0x2e5, 325), 0, &[w(0x310), w(0x30b)]);
    let y = sub(g.read::<f32>(w(0x31a)), k(g, 0x322));
    let old = gw(g, 0x327);
    abs_stores(g, img, f, 0x32e, 0x341);
    g.write::<f32>(w(0x345), y);
    abs_stores(g, img, f, 0x349, 0x367);
    append_0601(g, img, f, old, [0x368, 0x36d, 0x373, 0x382, 0x38e]);
    abs_stores(g, img, f, 0x392, 0x3a6);
    let mut st = Pushes::default();
    st.push_all(&[w(0x3ad), w(0x3b2)]);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, t(g, 0x387, 325), 0);
    let insert = |g: &mut G, grp: u32, ops: [u32; 6], at: u32| {
        for (i, o) in (0u32..).zip(ops) {
            g.write::<u32>(grp + 4 * i, w(o));
        }
        let ins = Frame {
            at: call(at),
            ecx: grp,
            ..*fr
        };
        crate::nodes::node_insert(g, img, &ins);
    };
    insert(g, e + 8, [0x3c0, 0x3c8, 0x3d0, 0x3d8, 0x3e0, 0x3e8], 0x3ed);
    let x = gw(g, 0x3f8);
    st.push(x);
    // call width: 12 (the most `Pushes::words` holds)
    let r = st.call(g, t(g, 0x3f2, 215), 0).eax;
    g.write::<u32>(w(0x405), r);
    let old = gw(g, 0x40a);
    abs_stores(g, img, f, 0x411, 0x442);
    g.write::<u32>(w(0x443), old);
    if old == w(0x448) {
        abs_stores(g, img, f, 0x44e, 0x458);
    } else {
        g.write::<u32>(old.wrapping_add(4), w(0x45d));
    }
    let q0 = g.read::<u64>(w(0x465));
    let n = g.read::<u16>(w(0x46b)).wrapping_add(2);
    g.write::<u64>(w(0x473), q0);
    let q1 = g.read::<u64>(w(0x47b));
    abs_stores(g, img, f, 0x486, 0x490);
    g.write::<u64>(w(0x494), q1);
    abs_stores(g, img, f, 0x498, 0x4d3);
    if w(0x480) == w(0x4d5) {
        abs_stores(g, img, f, 0x4db, 0x4e5);
    } else {
        abs_stores(g, img, f, 0x4e7, 0x4f1);
    }
    g.write::<u16>(w(0x4f5), n);
    let x = gw(g, 0x505);
    abs_stores(g, img, f, 0x509, 0x51d);
    // call width: 2
    g.call(t(g, 0x4fa, 290), 0, &[x, w(0x4ff)]);
    let old = gw(g, 0x526);
    runs(g, &[(0x52d, 0x537), (0x537, 0x572)]);
    append_0601(g, img, f, old, [0x573, 0x578, 0x57e, 0x58d, 0x594]);
    let x = gw(g, 0x5a4);
    abs_stores(g, img, f, 0x5a8, 0x5bc);
    let hv = e + 4;
    // call width: 2
    g.call(t(g, 0x59e, 601), 0, &[x, hv]);
    let h = sub(k(g, 0x5c8), g.read::<f32>(hv));
    abs_stores(g, img, f, 0x5d9, 0x5e3);
    let tilt = div(mul(h, k(g, 0x5ef)), k(g, 0x607));
    g.write::<u32>(w(0x620), crate::sb_sse::cvtt(tilt).cast_unsigned());
    insert(g, e + 8, [0x5e7, 0x5f7, 0x5ff, 0x60f, 0x61b, 0x628], 0x62d);
    let x = gw(g, 0x632);
    abs_stores(g, img, f, 0x63a, 0x641);
    for (i, v) in (0u32..).zip([w(0x645), w(0x64d), x, 0x6b]) {
        g.write::<u32>(e + 8 + 4 * i, v);
    }
    crate::m0042::node_init(g, img, call(0x65e), e + 8);
    let old = gw(g, 0x663);
    abs_stores(g, img, f, 0x667, 0x6a2);
    append_0601(g, img, f, old, [0x6a3, 0x6a8, 0x6ae, 0x6bd, 0x6c9]);
    for (src, dst) in [(0x6c2u32, 0x6ceu32), (0x6d3, 0x6d8)] {
        let x = gw(g, src);
        g.write::<u32>(w(dst), x);
    }
    let x = gw(g, 0x6dd);
    abs_stores(g, img, f, 0x6e5, 0x6ef);
    g.write::<u32>(w(0x6f0), x);
    insert(g, e + 8, [0x6f8, 0x700, 0x708, 0x710, 0x718, 0x720], 0x725);
    let b = u32::from(g.read::<u8>(w(0x72c)));
    // call width: 1
    let r = g.call(t(g, 0x732, 215), 0, &[b]).eax;
    g.write::<u32>(w(0x73f), r);
    let old = gw(g, 0x744);
    abs_stores(g, img, f, 0x74b, 0x77c);
    append_pair_0601(
        g,
        img,
        f,
        old,
        [0x77d, 0x782, 0x788, 0x797, 0x79d],
        (0x7a2, (0x7a8, 0x815), (0x817, 0x81d, 0x829), 0x837),
    );
    let b = u32::from(g.read::<u8>(w(0x83e)));
    abs_stores(g, img, f, 0x84d, 0x861);
    // call width: 2
    g.call(t(g, 0x849, 290), 0, &[b, w(0x843)]);
    let old = gw(g, 0x86a);
    runs(g, &[(0x871, 0x87b), (0x87b, 0x8b6)]);
    append_0601(g, img, f, old, [0x8b7, 0x8bc, 0x8c2, 0x8d1, 0x8d8]);
    abs_stores(g, img, f, 0x8e0, 0x908);
    insert(g, e + 8, [0x90c, 0x914, 0x91c, 0x924, 0x92c, 0x934], 0x939);
    abs_stores(g, img, f, 0x942, 0x949);
    let mut st = Pushes::default();
    st.push_all(&[1, 0x300f]);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, t(g, 0x93e, 27), 0);
    st.push(0x300f);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, t(g, 0x956, 4), 0);
    let old = gw(g, 0x965);
    abs_stores(g, img, f, 0x96c, 0x9a7);
    append_pair_0601(
        g,
        img,
        f,
        old,
        [0x9a8, 0x9ad, 0x9b3, 0x9c2, 0x9c8],
        (0x9cd, (0x9d3, 0xa2c), (0xa2e, 0xa34, 0xa40), 0xa4e),
    );
    abs_stores(g, img, f, 0xa57, 0xa6b);
    let mut st = Pushes::default();
    st.push_all(&[w(0xa72), w(0xa77)]);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, t(g, 0xa53, 325), 0);
    let y = mul(g.read::<f32>(w(0xa81)), k(g, 0xa89));
    let old = gw(g, 0xa8e);
    g.write::<f32>(w(0xa99), y);
    abs_stores(g, img, f, 0xa9d, 0xace);
    append_0601(g, img, f, old, [0xacf, 0xad4, 0xada, 0xae9, 0xb00]);
    let z = mul(g.read::<f32>(w(0xaf1)), k(g, 0xaf9));
    abs_stores(g, img, f, 0xb13, 0xb31);
    g.write::<f32>(w(0xb35), z);
    abs_stores(g, img, f, 0xb39, 0xb4d);
    let parent = w(0xb0f);
    // push imm; push imm; call (cdecl `(a, b)`, `edx` the parent)
    g.write::<u32>(e.wrapping_sub(4), w(0xb05));
    g.write::<u32>(e.wrapping_sub(8), w(0xb0a));
    let sn = Frame {
        at: call(0xb4e),
        esp: e.wrapping_sub(0xc),
        edx: parent,
        ..*fr
    };
    crate::m0601::static_node(g, img, &sn);
    // the group at `esp + 0x10` with the two words still pushed
    let grp = e + 8;
    for (i, v) in (0u32..).zip([w(0xb5a), parent, w(0xb66), w(0xb6e), w(0xb76), w(0xb7e)]) {
        g.write::<u32>(grp + 4 * i, v);
    }
    let ins = Frame {
        at: call(0xb83),
        ecx: grp,
        ..*fr
    };
    crate::nodes::node_insert(g, img, &ins);
    abs_stores(g, img, f, 0xb8c, 0xb93);
    let mut st = Pushes::default();
    st.push_all(&[w(0xb05), w(0xb0a), 1]);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, t(g, 0xb88, 281), 0);
    // call width: 4
    g.call(t(g, 0xb9e, 694), 0, &[1, 0, 0, 0]);
    let b = u32::from(g.read::<u8>(w(0xbb8)));
    for (i, v) in (0u32..).zip([w(0xbc7), w(0xbcf), b]) {
        g.write::<u32>(e + 8 + 4 * i, v);
    }
    g.write::<u8>(e + 0x14, 0);
    abs_stores(g, img, f, 0xbdc, 0xbe6);
    crate::sa_summon::model_node_init(g, img, call(0xbe7), e + 8);
    // the two task nodes on the static list
    let tail = gw(g, 0xbec);
    g.write::<u32>(w(0xbf1), tail);
    abs_stores(g, img, f, 0xbf5, 0xc1d);
    g.write::<u32>(tail.wrapping_add(4), w(0xc20));
    let tail = gw(g, 0xc25);
    let fl = gw(g, 0xc2b) & !1;
    g.write::<u32>(w(0xc2b), fl);
    g.write::<u32>(w(0xc31), tail);
    abs_stores(g, img, f, 0xc35, 0xc85);
    g.write::<u32>(tail.wrapping_add(4), w(0xc88));
    let fl = gw(g, 0xc8e) & !1;
    g.write::<u32>(w(0xc8e), fl);
    abs_stores(g, img, f, 0xc93, 0xcbb);
    Ret::eax(0)
}

/// [`start_up_0601`] (the `rel32`s: its targets, node and set-up helpers).
pub const START_UP_0601: Sig = Sig {
    len: 3266,
    hash: 0x5698_ee22_824b_5053,
    calls: &[
        (0xe, Callee::Sig(&crate::s4_summon::TARGETS_INIT_B)),
        (0x3ed, Callee::Sig(&crate::nodes::INSERT)),
        (0x62d, Callee::Sig(&crate::nodes::INSERT)),
        (0x65e, Callee::Sig(&crate::m0042::NODE_INIT)),
        (0x725, Callee::Sig(&crate::nodes::INSERT)),
        (0x939, Callee::Sig(&crate::nodes::INSERT)),
        (0xb4e, Callee::Sig(&crate::m0601::STATIC_NODE)),
        (0xb83, Callee::Sig(&crate::nodes::INSERT)),
        (0xbe7, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
    ],
    imports: &[],
};

/// `magic_0613`'s summon task (0x10009780, 2902 bytes; a task callback
/// `(A)`, cdecl; an aligned /GS frame `E` below three pushes). `S = A[1]`,
/// the state `S + 0x18`; every run ends with `S + 0x34` and `S + 0x38`
/// up by one. The states:
/// - 0 becomes 1; 0x190: the camera's words by slot 520's answer, the
///   time from 0 frames ([`frames_time`](crate::s4_small::frames_time)),
///   on; 0x191 and 0x192: a flag word, a timer `+ K` and, when the byte
///   flag is set, slot 325 and a word, on;
/// - 0x19a: the summon's model node, task, nodes under the static
///   parents ([`model_node_init`](crate::sa_summon::model_node_init),
///   [`task_start`](crate::m0613::task_start), the time left, two owned
///   node appends, the colour node, the camera move, the summon set-up
///   ([`summon_setup_0613`]), a static link, a sound node), slots 262, 12,
///   288; every actor of group 0x10 that is neither the caster nor one of
///   the models nor of the kinds 0x106a/0x10f8/0x10fa is hidden (slot
///   296); then 0x19c at once;
/// - 0x19b: on with the counter `S + 0x38` 0; 0x19c: a height down by
///   `K`, on after 40 runs;
/// - 0x2bc: the camera place (slots 325, 262, 12, 327, 229) and the
///   camera words by slot 520's answers, on;
/// - 0x2710: the current list node unlinked and the task's node `*A` off
///   its ring.
///
/// Returns `eax` as the original leaves it.
#[allow(clippy::too_many_lines)] // the original's states, one by one
pub fn summon_task_0613<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r25::abs_stores;
    use crate::sb_sse::{add, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let call = |o: u32| img.rel32_target(f + o);
    let zero = |g: &mut G, ops: &[u32]| {
        for &o in ops {
            g.write::<u32>(w(o), 0);
        }
    };
    // push ebp; and esp, -8; sub esp, 0x74; three pushes
    let a0 = fr.esp.wrapping_sub(4) & !7;
    let cookie = g.read::<u32>(w(0xa)) ^ a0.wrapping_sub(0x74);
    g.write::<u32>(a0.wrapping_sub(4), cookie);
    let e = a0.wrapping_sub(0x80);
    let a = fr.arg(g, 0);
    let s = g.read::<u32>(a.wrapping_add(4));
    let at = |o: u32| s.wrapping_add(o);
    let up = |g: &mut G| {
        let x = g.read::<u32>(at(0x18)).wrapping_add(1);
        g.write::<u32>(at(0x18), x);
    };
    let state = g.read::<u32>(at(0x18));
    let eax = match state {
        0 => {
            g.write::<u32>(at(0x18), 1);
            1
        }
        0x190 => {
            abs_stores(g, img, f, 0x5e, 0x7c);
            // call width: 0
            let r = g.call(t(g, 0x5a, 520), 0, &[]).eax;
            let x = if r == 0x024b_0005 || r == 0x024f_000a {
                let x = sub(g.read::<f32>(w(0xbc)), k(g, 0xc4));
                abs_stores(g, img, f, 0xc8, 0xdc);
                x
            } else {
                let x = sub(g.read::<f32>(w(0x96)), k(g, 0x9e));
                abs_stores(g, img, f, 0xa2, 0xb6);
                x
            };
            g.write::<u16>(w(0xe2), 0);
            g.write::<f32>(w(0xea), x);
            zero(g, &[0xef]);
            abs_stores(g, img, f, 0xf3, 0xfd);
            zero(g, &[0xfe, 0x103, 0x108]);
            abs_stores(g, img, f, 0x10c, 0x116);
            zero(g, &[0x117]);
            for o in [0x28u32, 0x2c, 0x30] {
                g.write::<u32>(e + o, 0);
            }
            zero(g, &[0x128, 0x12d, 0x132, 0x137, 0x13c, 0x141]);
            let ft = Frame {
                at: call(0x146),
                edx: 0,
                eax: 0,
                ..*fr
            };
            crate::s4_small::frames_time(g, img, &ft);
            abs_stores(g, img, f, 0x14a, 0x154);
            up(g);
            0
        }
        0x191 | 0x192 => {
            if state == 0x191 {
                abs_stores(g, img, f, 0x165, 0x176);
                up(g);
            }
            let on = g.read::<u8>(w(0x188)) & 1 != 0;
            let x = add(g.read::<f32>(w(0x191)), k(g, 0x199));
            g.write::<f32>(w(0x1a1), x);
            if on {
                abs_stores(g, img, f, 0x1b0, 0x1ba);
                // call width: 2
                let r = g.call(t(g, 0x1ac, 325), 0, &[w(0x1c6), w(0x1c1)]).eax;
                abs_stores(g, img, f, 0x1cf, 0x1d9);
                up(g);
                r
            } else {
                state
            }
        }
        0x19a..=0x19c => {
            let mut eax = 0;
            if state == 0x19a {
                eax = summon_start_0613(g, img, fr, e);
                let x = g.read::<u32>(at(0x18)).wrapping_add(2);
                g.write::<u32>(at(0x18), x);
                g.write::<u32>(at(0x38), 0);
            } else if state == 0x19b {
                up(g);
                g.write::<u32>(at(0x38), 0);
            }
            let y = sub(g.read::<f32>(w(0x8fb)), k(g, 0x903));
            g.write::<f32>(w(0x90b), y);
            if g.read::<u32>(at(0x38)).cast_signed() >= 0x28 {
                up(g);
            }
            eax
        }
        0x2bc => {
            abs_stores(g, img, f, 0x942, 0x953);
            let mut st = Pushes::default();
            st.push_all(&[w(0x95a), w(0x95f)]);
            // call width: 12 (the most `Pushes::words` holds)
            st.call(g, t(g, 0x93e, 325), 0);
            let x = gw(g, 0x97b);
            st.push_all(&[e + 0x30, e + 0x2c, e + 0x28, x]);
            // call width: 12 (the most `Pushes::words` holds)
            st.call(g, t(g, 0x975, 262), 0);
            st.push(gw(g, 0x98e));
            // call width: 12 (the most `Pushes::words` holds)
            let v = st0_f32(st.call(g, t(g, 0x988, 12), 0).st0);
            g.write::<f32>(e + 0x2c, v);
            let y = sub(v, k(g, 0x9a5));
            let m = e + 0x38;
            let mut st = Pushes::default();
            st.push_all(&[w(0x9b1), m]);
            g.write::<f32>(w(0x9bf), y);
            // call width: 12 (the most `Pushes::words` holds)
            st.call(g, t(g, 0x9b7, 327), 0);
            st.push_all(&[w(0x9cc), m, w(0x9db)]);
            // call width: 12 (the most `Pushes::words` holds)
            st.call(g, t(g, 0x9d6, 229), 0);
            abs_stores(g, img, f, 0x9ec, 0xa3c);
            // call width: 0
            let r = g.call(t(g, 0x9e8, 520), 0, &[]).eax;
            if r == 0x0237_0000 {
                abs_stores(g, img, f, 0xa94, 0xab2);
            } else if r == 0x0238_0000 {
                abs_stores(g, img, f, 0xa88, 0xa92);
                abs_stores(g, img, f, 0xa9e, 0xab2);
            } else {
                abs_stores(g, img, f, 0xa55, 0xa69);
                if r == 0x024f_000a {
                    abs_stores(g, img, f, 0xa7c, 0xa86);
                } else {
                    abs_stores(g, img, f, 0xa70, 0xa7a);
                }
            }
            // call width: 0
            let r2 = g.call(t(g, 0xab3, 520), 0, &[]).eax;
            abs_stores(g, img, f, 0xabf, 0xac9);
            let v = if r2 == 0x0238_0000 {
                abs_stores(g, img, f, 0xaf9, 0xb0d);
                g.write::<u16>(w(0xb0f), 0x800);
                0x800
            } else {
                abs_stores(g, img, f, 0xad5, 0xae9);
                g.write::<u16>(w(0xaeb), 0x200);
                0x200
            };
            up(g);
            v
        }
        0x2710 => {
            crate::l3_common::list_unlink_current(g, img, call(0xb20), state);
            let node = g.read::<u32>(a);
            let (nx, pv) = (g.read::<u32>(node + 4), g.read::<u32>(node + 8));
            g.write::<u32>(nx + 8, pv);
            let (pv, nx) = (g.read::<u32>(node + 8), g.read::<u32>(node + 4));
            g.write::<u32>(pv + 4, nx);
            g.write::<u32>(node + 8, node);
            g.write::<u32>(node + 4, node);
            nx
        }
        0x19d..=0x2bb => state - 0x19c,
        _ => state,
    };
    for o in [0x34u32, 0x38] {
        let x = g.read::<u32>(at(o)).wrapping_add(1);
        g.write::<u32>(at(o), x);
    }
    Ret::eax(eax)
}

/// [`summon_task_0613`]'s state 0x19a (the summon started), on the
/// task's frame `E`. Returns `eax` as the original leaves it after the
/// actors' loop.
#[allow(clippy::too_many_lines)] // the original's steps, in order
fn summon_start_0613<G: Game>(g: &mut G, img: &Image, fr: &Frame, e: u32) -> u32 {
    use crate::r25::abs_stores;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let call = |o: u32| img.rel32_target(f + o);
    let grp = e + 0x10;
    let x = gw(g, 0x1e2);
    for (i, v) in (0u32..).zip([w(0x1ee), w(0x1f6), x]) {
        g.write::<u32>(grp + 4 * i, v);
    }
    g.write::<u8>(grp + 0xc, 0);
    abs_stores(g, img, f, 0x203, 0x20d);
    crate::sa_summon::model_node_init(g, img, call(0x20e), grp);
    let ts = Frame {
        at: call(0x218),
        edx: w(0x213),
        ..*fr
    };
    crate::m0613::task_start(g, img, &ts);
    let old = gw(g, 0x21d);
    abs_stores(g, img, f, 0x221, 0x252);
    append_0601(g, img, f, old, [0x253, 0x258, 0x25e, 0x26d, 0x279]);
    let x = gw(g, 0x28e);
    abs_stores(g, img, f, 0x292, 0x2a6);
    let mut st = Pushes::default();
    st.push_all(&[w(0x27e), w(0x283), w(0x288), x]);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, t(g, 0x272, 262), 0);
    st.push(gw(g, 0x2b5));
    // call width: 12 (the most `Pushes::words` holds)
    let h = st0_f32(st.call(g, t(g, 0x2af, 12), 0).st0);
    g.write::<f32>(w(0x2c0), h);
    let old = gw(g, 0x2c5);
    abs_stores(g, img, f, 0x2cc, 0x2d6);
    abs_stores(g, img, f, 0x2d6, 0x311);
    append_0601(g, img, f, old, [0x312, 0x317, 0x31d, 0x32c, 0x333]);
    abs_stores(g, img, f, 0x33b, 0x345);
    g.write::<u32>(grp, w(0x349));
    g.write::<u32>(grp + 4, w(0x351));
    crate::m0042::time_left(g, img, call(0x356), grp, e.wrapping_sub(4));
    // four pushes for the owned append (cdecl, the words stay)
    let owned = |g: &mut G, ops: [u32; 4], (edx, ecx): (u32, u32), at: u32| {
        for (i, o) in (0u32..).zip(ops) {
            g.write::<u32>(e.wrapping_sub(0x10 - 4 * i), w(o));
        }
        let fa = Frame {
            at: call(at),
            esp: e.wrapping_sub(0x14),
            ecx: w(ecx),
            edx: w(edx),
            ..*fr
        };
        crate::sa_misc::node_append_owned(g, img, &fa);
    };
    owned(g, [0x36a, 0x365, 0x360, 0x35b], (0x36f, 0x374), 0x379);
    g.write::<u32>(grp, w(0x385));
    g.write::<u32>(grp + 4, w(0x38d));
    abs_stores(g, img, f, 0x391, 0x39b);
    let colour = Frame {
        at: call(0x39c),
        esp: e.wrapping_sub(0x14),
        ecx: grp,
        ..*fr
    };
    let eax = crate::s3_misc::colour_node(g, img, &colour);
    let x2 = k(g, 0x3a4);
    abs_stores(g, img, f, 0x3a8, 0x3ee);
    crate::sb_misc::move_start(g, img, call(0x3ef), x2, eax);
    let x = gw(g, 0x3f4);
    for (i, v) in (0u32..).zip([w(0x400), w(0x408), x, 0]) {
        g.write::<u32>(grp + 4 * i, v);
    }
    g.write::<u8>(grp + 0x10, 0);
    abs_stores(g, img, f, 0x41d, 0x427);
    let ss = Frame {
        at: call(0x428),
        esp: e.wrapping_sub(0x14),
        ecx: grp,
        ..*fr
    };
    summon_setup_0613(g, img, &ss);
    let sl = Frame {
        at: call(0x430),
        esp: e.wrapping_sub(8),
        ..*fr
    };
    crate::s4_small::static_link_word(g, img, &sl);
    let old = gw(g, 0x435);
    abs_stores(g, img, f, 0x43c, 0x46d);
    g.write::<u32>(w(0x46e), old);
    if old == w(0x473) {
        abs_stores(g, img, f, 0x479, 0x483);
    } else {
        g.write::<u32>(old.wrapping_add(4), w(0x488));
    }
    let old2 = gw(g, 0x48d);
    let n = g.read::<u16>(w(0x494)).wrapping_add(1);
    g.write::<u16>(w(0x494), n);
    abs_stores(g, img, f, 0x498, 0x4de);
    abs_stores(g, img, f, 0x4de, 0x50f);
    append_pair_0601(
        g,
        img,
        f,
        old2,
        [0x510, 0x515, 0x51b, 0x52a, 0x530],
        (0x535, (0x53b, 0x5b2), (0x5b4, 0x5ba, 0x5c6), 0x5fc),
    );
    abs_stores(g, img, f, 0x5f0, 0x5fa);
    abs_stores(g, img, f, 0x600, 0x61e);
    owned(g, [0x5e2, 0x5dd, 0x5d6, 0x5d1], (0x5e7, 0x5ec), 0x61f);
    let parent = w(0x5e7);
    let old = gw(g, 0x624);
    abs_stores(g, img, f, 0x62b, 0x648);
    g.write::<u32>(w(0x64a), parent);
    g.write::<u32>(w(0x650), parent);
    g.write::<u32>(w(0x655), old);
    if old == parent {
        abs_stores(g, img, f, 0x65d, 0x667);
    } else {
        g.write::<u32>(old.wrapping_add(4), w(0x66c));
    }
    let n = g.read::<u16>(w(0x67a)).wrapping_add(2);
    for (src, dst) in [(0x674u32, 0x682u32), (0x68a, 0x692)] {
        let x = g.read::<u32>(w(src));
        g.write::<u32>(w(dst), x);
    }
    let z = g.read::<u32>(w(0x69a));
    abs_stores(g, img, f, 0x6a5, 0x6c3);
    g.write::<u32>(w(0x6c7), z);
    abs_stores(g, img, f, 0x6cb, 0x706);
    if w(0x69f) == w(0x708) {
        abs_stores(g, img, f, 0x70e, 0x718);
    } else {
        abs_stores(g, img, f, 0x71a, 0x724);
    }
    abs_stores(g, img, f, 0x72a, 0x734);
    g.write::<u16>(w(0x736), n);
    abs_stores(g, img, f, 0x73a, 0x776);
    for (i, v) in (0u32..).zip([w(0x77a), w(0x782), 0x4444_8000]) {
        g.write::<u32>(grp + 4 * i, v);
    }
    abs_stores(g, img, f, 0x78e, 0x798);
    crate::o6_fam_node::node_reset_sound(g, img, call(0x799), grp);
    abs_stores(g, img, f, 0x7a7, 0x855);
    // call width: 2
    let list = g.call(t(g, 0x7a3, 288), 0, &[0x10, grp]).eax;
    let mut eax = list;
    let mut i = 0u32;
    while i.cast_signed() < g.read::<u32>(grp).cast_signed() {
        let who = u32::from(g.read::<u8>(list.wrapping_add(i)));
        i += 1;
        if who == gw(g, 0x878) {
            continue;
        }
        let n = gw(g, 0x86d).cast_signed();
        let mut found = false;
        let mut j = 0i32;
        let mut p = w(0x885);
        if n > 0 {
            eax = p;
        }
        while j < n {
            if who == g.read::<u32>(p) {
                found = true;
                break;
            }
            j += 1;
            p = p.wrapping_add(0x274);
            eax = p;
        }
        if found {
            continue;
        }
        // call width: 1
        let kind = g.call(t(g, 0x89f, 390), 0, &[who]).eax;
        let d = kind.wrapping_sub(0x106a);
        eax = d;
        if d == 0 {
            continue;
        }
        let d = d.wrapping_sub(0x8e);
        eax = d;
        if d == 0 {
            continue;
        }
        let d = d.wrapping_sub(2);
        eax = d;
        if d == 0 {
            continue;
        }
        let b = u32::from(g.read::<u8>(list.wrapping_add(i - 1)));
        // call width: 2
        eax = g.call(t(g, 0x8c9, 296), 0, &[b, 0]).eax;
    }
    eax
}

/// [`summon_task_0613`] (the `rel32`s: its helpers and the summon start's).
pub const SUMMON_TASK_0613: Sig = Sig {
    len: 2902,
    hash: 0x8b46_0092_9773_952a,
    calls: &[
        (0x146, Callee::Sig(&crate::s4_small::FRAMES_TIME)),
        (0x20e, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0x218, Callee::Sig(&crate::m0613::TASK_START)),
        (0x356, Callee::Sig(&crate::m0042::TIME_LEFT)),
        (0x379, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x39c, Callee::Sig(&crate::s3_misc::COLOUR_NODE)),
        (0x3ef, Callee::Sig(&crate::sb_misc::MOVE_START)),
        (0x428, Callee::Sig(&SUMMON_SETUP_0613)),
        (0x430, Callee::Sig(&crate::s4_small::STATIC_LINK_WORD)),
        (0x61f, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (
            0x799,
            Callee::Sig(&crate::o6_fam_node::NODE_RESET_SOUND_0155_2FC0),
        ),
        (0xb20, Callee::Sig(&crate::l3_common::LIST_UNLINK_CURRENT)),
        (0xb4e, Callee::Any),
    ],
    imports: &[],
};
