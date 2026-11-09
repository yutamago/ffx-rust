//! Round 25's own ports (own rows of `magic_0450` and up): the DLLs' own
//! code written out, and copies of ported functions bound by their own
//! `Sig`.

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

const ONE: u32 = 0x3f80_0000;

/// `magic_0573`'s summon task (0x100077b0, 956 bytes; a task function,
/// cdecl `(task)`, the record `R = task[1]`; an 8-aligned cookie frame:
/// `A = ((E - 4) & !7) - 0x3c`, the task at `A + 4`, the bone cursor at
/// `A + 8`, the vector `V` at `A + 0x28`). By the state `R + 0x18`, the
/// count `R + 0x20` (a state set here and jumped to runs at once):
/// - 0: count 0x1b, state 1;
/// - 1: when the count runs out, state 10;
/// - 10: `R + 0x258` = `R + 0x38`, the byte `R + 0x255` 1, `R + 0x2b4`
///   0.25, `R + 0xfc` less 0x1800, state 11, count 0x11a;
/// - 11: when the count runs out, state 0x14;
/// - 0x14: the bone node `N = R + 0x10c` gets the parent `*G_PARENT`
///   (`N + 0x14c`, the byte `+0x149` 1, `+0x1a8` 1.0) and is updated
///   ([`bone_0573`](crate::r23::bone_0573), the cursor's pass byte
///   `*G_PASS`); slot 325 `(R + 0x3c, R + 0x29c)`; unless bit 0 of the
///   object `O = R[0x30]`'s `+0x90`, slot 531 `(O[0x50], V)`; slot 234
///   `(R + 0x4c, V, R + 0x3c)`; state 0x15, `R + 0x2c` 0, count 4, and on
///   into 0x15;
/// - 0x15: `t = R[0x2c] + K` kept; slot 233 `(V, R + 0x4c, t)`, slot 248
///   `(R + 0x29c, R + 0x3c, V)`, `R + 0x2a8` 1.0; at count 0 slot 228
///   `(O[0x50], 0x49)`; once `t >= K2`: slot 248 `(R + 0x29c, R + 0x3c,
///   R + 0x4c)`, `R + 0x2a8` 1.0 and state 0x1e;
/// - 0x1e: the object back at its node
///   ([`object_to_node`](crate::sa_misc::object_to_node)), the nodes `R +
///   0x88` and `R + 0xb4` appended to `G_LIST`
///   ([`node_append`](crate::sb_misc::node_append)) with slot 325 `(R +
///   0xa4, R + 0x29c)` and `R + 0xa8` less `K` between them; `R + 0xd0..`
///   (1, 1, 1, 0.8); the node `R + 0x2bc` inserted
///   ([`node_insert`](crate::nodes::node_insert), the group `(node, the
///   list the append leaves in edx, G_A, R + 0x88, G_B, R + 0xb4)`), the
///   particle node `R + 0x410`
///   ([`particle_node_with`](crate::s3_nodes::particle_node_with), `(R +
///   0x2bc, 2)`) and the effect node `R + 0x510`
///   ([`effect_node_a`](crate::s4_fam446::effect_node_a), `(R + 0x88,
///   1)`); bit 0 of `R + 0x53c` set, `R + 0x540` 0; slots 239 `(*G_W,
///   *G_BYTE, 2)` and 213 `(O[0x50], 5)`; state 10000;
/// - 10000: the task's node unlinked, state 20000 (returns its old
///   `+4`).
///
/// Every way out but 10000's counts down once and returns what `eax`
/// holds there: the jump table's index (1 for 0 and 1, 3 for 11, 4 for
/// 2..9), `R[0x38]` after 10, the last slot's answer after 0x15 and 0x1e,
/// the task for other states.
#[allow(clippy::too_many_lines)] // the original's states, in order
pub fn summon_task_0573<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    let a = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x3c);
    g.write::<u32>(a + 4, task);
    let v = a + 0x28;
    let count_down = |g: &mut G, eax: u32| {
        let n = g.read::<u32>(r + 0x20);
        g.write::<u32>(r + 0x20, n.wrapping_sub(1));
        Ret::eax(eax)
    };
    let set_state = |g: &mut G, s: u32| g.write::<u32>(r + 0x18, s);
    let mut eax = task;
    loop {
        let state = g.read::<u32>(r + 0x18);
        match state {
            0 | 1 => {
                if state == 0 {
                    g.write::<u32>(r + 0x20, 0x1b);
                    set_state(g, 1);
                }
                if g.read::<i32>(r + 0x20) > 0 {
                    return count_down(g, 1);
                }
                set_state(g, 10);
            }
            10 => {
                let x = g.read::<u32>(r + 0x38);
                g.write::<u32>(r + 0x258, x);
                g.write::<u8>(r + 0x255, 1);
                g.write::<u32>(r + 0x2b4, 0x3e80_0000);
                let y = g.read::<u32>(r + 0xfc).wrapping_sub(0x1800);
                g.write::<u32>(r + 0xfc, y);
                set_state(g, 11);
                g.write::<u32>(r + 0x20, 0x11a);
                // the count is never 0 here: the wait starts
                return count_down(g, x);
            }
            11 => {
                if g.read::<i32>(r + 0x20) > 0 {
                    return count_down(g, 3);
                }
                set_state(g, 0x14);
            }
            0x14 | 0x15 => {
                let o = g.read::<u32>(r + 0x30);
                if state == 0x14 {
                    let n = r + 0x10c;
                    let cursor = a + 8;
                    g.write::<u32>(cursor, n);
                    g.write::<u32>(n + 0x14c, w(0xba));
                    g.write::<u8>(n + 0x149, 1);
                    g.write::<u32>(n + 0x1a8, ONE);
                    let pass = g.read::<u8>(w(0xd0));
                    g.write::<u8>(cursor + 5, pass);
                    // three pushes below `A`, then the call's return address
                    crate::r23::bone_0573(g, img, call(0xdd), a.wrapping_sub(0x10), cursor);
                    // call width: 2
                    g.call(t(g, 0xe9, 325), 0, &[r + 0x3c, r + 0x29c]);
                    if g.read::<u8>(o + 0x90) & 1 == 0 {
                        // call width: 2
                        g.call(t(g, 0x111, 531), 0, &[g.read::<u32>(o + 0x50), v]);
                    }
                    // call width: 3
                    g.call(t(g, 0x12b, 234), 0, &[r + 0x4c, v, r + 0x3c]);
                    set_state(g, 0x15);
                    g.write::<u32>(r + 0x2c, 0);
                    g.write::<u32>(r + 0x20, 4);
                }
                let k = g.read::<f32>(w(0x15f));
                let x = add(g.read::<f32>(r + 0x2c), k);
                g.write::<f32>(r + 0x2c, x);
                // call width: 3
                g.call(t(g, 0x173, 233), 0, &[v, r + 0x4c, x.to_bits()]);
                // call width: 3
                eax = g.call(t(g, 0x198, 248), 0, &[r + 0x29c, r + 0x3c, v]).eax;
                g.write::<u32>(r + 0x2a8, ONE);
                if g.read::<u32>(r + 0x20) == 0 {
                    // call width: 2
                    eax = g
                        .call(t(g, 0x1c0, 228), 0, &[g.read::<u32>(o + 0x50), 0x49])
                        .eax;
                }
                let k2 = g.read::<f32>(w(0x1d7));
                // comiss; jb: below or unordered waits
                let x = g.read::<f32>(r + 0x2c);
                if x.is_nan() || x < k2 {
                    return count_down(g, eax);
                }
                // call width: 3
                eax = g
                    .call(t(g, 0x1eb, 248), 0, &[r + 0x29c, r + 0x3c, r + 0x4c])
                    .eax;
                g.write::<u32>(r + 0x2a8, ONE);
                set_state(g, 0x1e);
            }
            0x1e => {
                let o = g.read::<u32>(r + 0x30);
                let e = crate::sa_misc::object_to_node(g, img, call(0x25f), o, eax);
                let (m1, m2) = (r + 0x88, r + 0xb4);
                crate::sb_misc::node_append(g, img, call(0x271), m1, w(0x26a), e);
                // call width: 2
                g.call(t(g, 0x284, 325), 0, &[r + 0xa4, r + 0x29c]);
                let y = sub(g.read::<f32>(r + 0xa8), g.read::<f32>(w(0x29c)));
                g.write::<f32>(r + 0xa8, y);
                // node_append leaves ecx = the node and edx = the list
                let list = w(0x2aa);
                crate::sb_misc::node_append(g, img, call(0x2b7), m2, list, 0);
                g.write::<u32>(a + 0x24, m2);
                let node = r + 0x2bc;
                for off in [0xd0u32, 0xd4, 0xd8] {
                    g.write::<u32>(r + off, ONE);
                }
                g.write::<u32>(r + 0xdc, 0x3f4c_cccd);
                let group = a + 0x10;
                for (i, x) in (0u32..).zip([node, list, w(0x2fd), m1, w(0x309)]) {
                    g.write::<u32>(group + 4 * i, x);
                }
                crate::nodes::node_insert(
                    g,
                    img,
                    &Frame {
                        at: call(0x30e),
                        esp: a.wrapping_sub(0x10),
                        ecx: group,
                        ..*fr
                    },
                );
                // push 2; sub esp, 8; push the node: the return address at
                // `A - 0x20` (the two middle words are never written; the
                // callee does not read them)
                let esp = a.wrapping_sub(0x20);
                g.write::<u32>(esp + 4, node);
                g.write::<u32>(esp + 0x10, 2);
                let pf = Frame {
                    at: call(0x31f),
                    esp,
                    ecx: r + 0x410,
                    ..*fr
                };
                crate::s3_nodes::particle_node_with(g, img, &pf);
                // push 1; push ecx; push edi; push ecx: the `ecx` words, what
                // the particle node leaves, are not read by the callee (0
                // here)
                for (i, x) in (1u32..).zip([0, m1, 0, 1]) {
                    g.write::<u32>(esp + 4 * i, x);
                }
                let ef = Frame {
                    at: call(0x332),
                    esp,
                    ecx: r + 0x510,
                    ..*fr
                };
                crate::s4_fam446::effect_node_a(g, img, &ef);
                let fl = g.read::<u32>(r + 0x53c) | 1;
                g.write::<u32>(r + 0x53c, fl);
                g.write::<u32>(r + 0x540, 0);
                let b = u32::from(g.read::<u8>(w(0x34a)));
                // call width: 3
                g.call(t(g, 0x352, 239), 0, &[g.read::<u32>(w(0x358)), b, 2]);
                // call width: 2
                let eax = g
                    .call(t(g, 0x370, 213), 0, &[g.read::<u32>(o + 0x50), 5])
                    .eax;
                set_state(g, 0x2710);
                return count_down(g, eax);
            }
            0x2710 => {
                let node = g.read::<u32>(task);
                let (prev, next) = (g.read::<u32>(node + 4), g.read::<u32>(node + 8));
                g.write::<u32>(prev + 8, next);
                let (next, prev) = (g.read::<u32>(node + 8), g.read::<u32>(node + 4));
                g.write::<u32>(next + 4, prev);
                g.write::<u32>(node + 8, node);
                g.write::<u32>(node + 4, node);
                let n = g.read::<u32>(r + 0x20);
                g.write::<u32>(r + 0x20, n.wrapping_sub(1));
                set_state(g, 0x4e20);
                return Ret::eax(prev);
            }
            2..=9 => return count_down(g, 4),
            _ => return count_down(g, eax),
        }
    }
}

/// [`summon_task_0573`] (956 bytes, 1 overlay).
pub const SUMMON_TASK_0573: Sig = Sig {
    len: 956,
    hash: 0xa375_2bb4_d309_5bec,
    calls: &[
        (0xdd, Callee::Sig(&crate::r23::BONE_0573)),
        // __security_check_cookie
        (0x253, Callee::Any),
        (0x25f, Callee::Sig(&crate::sa_misc::OBJECT_TO_NODE)),
        (0x271, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x2b7, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x30e, Callee::Sig(&crate::nodes::INSERT)),
        (0x31f, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x332, Callee::Sig(&crate::s4_fam446::EFFECT_NODE_A)),
        // __security_check_cookie
        (0x393, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0708`'s draw (0x10001f80, 1721 bytes; a table entry, no
/// arguments; a 16-aligned cookie frame: `A = ((E - 4) & !0xf) - 0x328`,
/// the draw record `D = A + 0x18`, the matrices `A + 0xa0`, `A + 0xe0`,
/// `A + 0x160`, `A + 0x1a0`, `A + 0x1e0`, the blocks `A + 0x120`, `A +
/// 0x220`, the colour `A + 0x90`). Nothing unless `*G_ON` and the time
/// `T = *G_T` is not negative. Slots 723 `(*G_M1)` and 724 `(*G_M2)` after
/// `**G_P1` = `X1` and `**G_P2` = `X2`; from `T = 1` the fade
/// ([`fade`](crate::m0708::fade) `(T, 0x1cc)`); slots 355 and 354
/// `(*G_O, -1)`; slot 230 `(A + 0xe0, V)`, slot 231 `(A + 0xa0, *G_M2,
/// A + 0xe0)` and `(A + 0xa0, *G_M1, A + 0xa0)`; `D` (0x1c, `**G_TEX`, u16
/// 0x44, colour 0x80808080 and 0x808080, three matrices from slot 235,
/// the byte `*G_B`, the rest 0); up to `T = 0x190` its parts bound
/// ([`bind_parts`](crate::m0708::bind_parts) `(*G_REC, D, *G_BASE)`).
/// Slot 719 `(PART, 2, 0)` gives the part `P`, slot 243 `(A + 0x120,
/// T << 10)`. From `T = 15` to `0xdb` the quad ([`quad_0708`]) with the colour
/// `0x80000080`, its alpha `(T - 15) * 128 / 90` up to 0x68, 0x80 up to
/// 0x77, then `(100 - (T - 0x78)) * 128 / 100`. From `T = 0xbe` to
/// `0x171` the rise: below `0xbe + 0x96` with `*G_R` 0, `*G_S = T -
/// 0xbe`, `*G_H` = trunc of a random number scaled (slot 307) and
/// `*G_R` = trunc of a second plus `*G_H`, less 1; else `*G_R` one less;
/// [`rise`](crate::m0708::rise) `(T - 0xbe - *G_S, *G_H)` while that is
/// positive. The 16 parts at `PARTS` (stride 0x80) with their flag set and
/// the part `PB` refreshed
/// ([`part_refresh_b`](crate::sa_misc::part_refresh_b)); past `T = 0x172`
/// [`rise`](crate::m0708::rise) `(T - 0x172, 12)`, and unless the u16
/// `*G_BUSY`: slots 357, 356 `(*G_O, -1, -1)`, 346 `(*G_O)`, 211 `(*G_O,
/// 0)`, 238 and 213 `(*G_C, 0)`, 214 `(*G_D)`. Unless `**G_STOP`, `T` one
/// up. Returns 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn draw_0708<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::s4_summon::trunc;
    use crate::sb_sse::{add, cvt, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let rd = |g: &G, o: u32| g.read::<u32>(w(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let a = (fr.esp.wrapping_sub(4) & !0xf).wrapping_sub(0x328);
    // after `push esi; push edi`
    let s = a.wrapping_sub(8);
    if rd(g, 0x1c) == 0 || rd(g, 0x2b).cast_signed() < 0 {
        return Ret::eax(0);
    }
    let m1 = rd(g, 0x3d);
    g.write::<u32>(rd(g, 0x37), w(0x43));
    g.write::<u32>(rd(g, 0x48), w(0x4e));
    g.call(t(g, 0x53, 723), 0, &[m1]);
    // call width: 2 (723's word stays under it)
    g.call(t(g, 0x60, 724), 0, &[rd(g, 0x66), m1]);
    let tm = rd(g, 0x74);
    if tm.cast_signed() > 0 {
        let ff = Frame {
            at: call(0x85),
            esp: s.wrapping_sub(4),
            ecx: tm,
            edx: 0x1cc,
            ..*fr
        };
        crate::m0708::fade(g, img, &ff);
    }
    // call width: 2
    g.call(t(g, 0x8a, 355), 0, &[rd(g, 0x92), u32::MAX]);
    // call width: 2
    g.call(t(g, 0x9f, 354), 0, &[rd(g, 0xa7), u32::MAX]);
    let (ma, me) = (a + 0xa0, a + 0xe0);
    // call width: 2
    g.call(t(g, 0xc1, 230), 0, &[me, w(0xbb)]);
    // call width: 3
    g.call(t(g, 0xe4, 231), 0, &[ma, rd(g, 0xd7), me]);
    // call width: 3
    g.call(t(g, 0x100, 231), 0, &[ma, rd(g, 0xfa), ma]);
    let d = a + 0x18;
    g.write::<u16>(d + 8, 0x44);
    g.write::<u32>(d + 0x18, 0x8080_8080);
    g.write::<u32>(d, 0x1c);
    g.write::<u8>(d + 0xa, 0);
    g.write::<u32>(d + 0x1c, 0x0080_8080);
    let tex = g.read::<u32>(rd(g, 0x117));
    g.write::<u32>(d + 4, tex);
    for o in [0x3cu32, 0x38, 0xc, 0x10, 0x14, 0x40] {
        g.write::<u32>(d + o, 0);
    }
    let mats = [a + 0x1a0, a + 0x160, a + 0x1e0];
    for (m, o) in mats.into_iter().zip([0x147u32, 0x18e, 0x1a3]) {
        g.call(t(g, o, 235), 0, &[m]);
    }
    for (i, m) in (0u32..).zip(mats) {
        g.write::<u32>(d + 0x24 + 4 * i, m);
    }
    let b = g.read::<u8>(w(0x1da));
    g.write::<u32>(d + 0x44, 0);
    g.write::<u8>(d + 0x4c, b);
    g.write::<u8>(d + 0x4d, 0);
    if rd(g, 0x1e3).cast_signed() <= 0x190 {
        // push *G_BASE: the return address at `S - 8`
        let esp = s.wrapping_sub(8);
        g.write::<u32>(esp + 4, rd(g, 0x200));
        let bf = Frame {
            at: call(0x20f),
            esp,
            ecx: rd(g, 0x206),
            edx: d,
            ..*fr
        };
        crate::m0708::bind_parts(g, img, &bf);
    }
    // call width: 3
    let part = g.call(t(g, 0x217, 719), 0, &[w(0x226), 2, 0]).eax;
    let shifted = rd(g, 0x22e) << 10;
    g.write::<u32>(a + 0xc, part);
    // call width: 2
    g.call(t(g, 0x238, 243), 0, &[a + 0x120, shifted]);
    let tm = rd(g, 0x253);
    g.write::<u32>(d + 0x18, 0x8000_0080);
    let ti = tm.cast_signed();
    let alpha = if ti < 0xf {
        None
    } else if ti < 0x69 {
        // unsigned: (T - 15) << 7 over 90
        Some((tm.wrapping_sub(0xf) << 7) / 90)
    } else if ti < 0x78 {
        Some(0x80)
    } else {
        let u = tm.wrapping_sub(0x78);
        (u < 0x64).then(|| ((0x64 - u) << 7) / 100)
    };
    if let Some(alpha) = alpha {
        g.write::<u8>(d + 0x1b, alpha as u8);
        quad_0708(g, img, f, a, part);
    }
    let tm = rd(g, 0x4cf);
    let mut left = tm.wrapping_sub(0xbe);
    if left < 0xb4 {
        let r = rd(g, 0x4e6);
        let (h, n) = if r == 0 && left < 0x96 {
            g.write::<u32>(w(0x501), left);
            let random = |g: &mut G, o: u32| {
                let x = crate::ob_model::st0_f32(g.call(t(g, o, 307), 0, &[]).st0);
                // fstp dword [esp + 0x10]
                g.write::<f32>(s + 0x10, x);
                x
            };
            let r1 = random(g, 0x4fb);
            let x = add(mul(sub(r1, k(g, 0x51b)), k(g, 0x523)), k(g, 0x52b));
            g.write::<u32>(w(0x534), trunc(x));
            let r2 = random(g, 0x539);
            let h = rd(g, 0x559);
            let y = add(mul(sub(r2, k(g, 0x553)), k(g, 0x568)), cvt(h));
            (h, trunc(y))
        } else {
            (rd(g, 0x578), r)
        };
        left = left.wrapping_sub(rd(g, 0x57e));
        g.write::<u32>(w(0x584), n.wrapping_sub(1));
        if left.cast_signed() > 0 {
            let rf = Frame {
                at: call(0x58f),
                esp: s.wrapping_sub(4),
                ecx: left,
                edx: h,
                ..*fr
            };
            crate::m0708::rise(g, img, &rf);
        }
    }
    let mut p = w(0x594);
    for _ in 0..0x10 {
        if g.read::<u16>(p + 0x1e) != 0 {
            crate::sa_misc::part_refresh_b(g, img, call(0x5aa), p, 0);
        }
        p = p.wrapping_add(0x80);
    }
    crate::sa_misc::part_refresh_b(g, img, call(0x5ba), w(0x5b5), 0);
    let mut tm = rd(g, 0x5c0);
    let n = tm.wrapping_sub(0x172);
    if n.cast_signed() > 0 {
        let rf = Frame {
            at: call(0x5d2),
            esp: s.wrapping_sub(4),
            ecx: n,
            edx: 0xc,
            ..*fr
        };
        crate::m0708::rise(g, img, &rf);
        tm = rd(g, 0x5d8);
    }
    if tm.cast_signed() > 0x172 && g.read::<u16>(w(0x5eb)) == 0 {
        // call width: 3
        g.call(t(g, 0x5f7, 357), 0, &[rd(g, 0x607), u32::MAX, u32::MAX]);
        // call width: 3
        g.call(t(g, 0x60e, 356), 0, &[rd(g, 0x61e), u32::MAX, u32::MAX]);
        // `ecx`: the original passes what it last held, rise's `ecx`
        // (`T - 0x172`, which rise never writes; this branch always
        // runs it) as far as the overlay's code goes. In the game slots
        // 357 and 356 (Ch_TextureSet*Base, cdecl) end with it scratch,
        // and Ch_ClutDispose (ported cdecl) does not read it, so the port
        // passes rise's value (brief 2026-11-08, registers left behind).
        g.call(t(g, 0x625, 346), n, &[rd(g, 0x62b)]);
        // call width: 2
        g.call(t(g, 0x638, 211), 0, &[rd(g, 0x640), 0]);
        // call width: 2
        g.call(t(g, 0x64d, 238), 0, &[rd(g, 0x655), 0]);
        // call width: 2
        g.call(t(g, 0x662, 213), 0, &[rd(g, 0x66a), 0]);
        g.call(t(g, 0x677, 214), 0, &[rd(g, 0x67d)]);
        tm = rd(g, 0x68b);
    }
    if g.read::<u32>(rd(g, 0x693)) == 0 {
        g.write::<u32>(w(0x69f), tm.wrapping_add(1));
    }
    Ret::eax(0)
}

/// [`draw_0708`]'s quad (its `+0x2bc..+0x4ce`, `f` the draw, `A` its
/// frame, `part` slot 719's part): the part's 64-bit word `q` gives the
/// sizes `w1 = 1 << (q >> 26 & 15)`, `w2 = 1 << (q >> 30 & 15)`; slot 885
/// `(q & 0x3fff, q >> 20 & 0x3f, 0, 0, w1, w2, A + 0x220)` with the lock
/// `**G_LOCK` 1 (put back after); when it answers, the colour `A + 0x90`
/// (the record's colour bytes times `K`) and slot 879 `(*G_O[0x830], A +
/// 0x220, u, v, A + 0x90, alpha, A + 0x120, w1, w2, 0)` under the lock,
/// `u` and `v` the record's `+0x10`/`+0x14` words in 16.16 of the sizes,
/// the remainders' sixteenths over the sizes, times `K2`.
fn quad_0708<G: Game>(g: &mut G, img: &Image, f: u32, a: u32, part: u32) {
    use crate::sb_sse::{cvt, mul};
    let w = |o: u32| img.word(f + o);
    let rd = |g: &G, o: u32| g.read::<u32>(w(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let d = a + 0x18;
    let tex = g.read::<u32>(rd(g, 0x2bd));
    g.write::<u32>(d + 4, tex);
    g.write::<u32>(d + 0x20, a + 0xa0);
    g.write::<u32>(d + 0x3c, 0);
    g.write::<u32>(d + 0x38, 0);
    let o830 = g.read::<u32>(rd(g, 0x2d5) + 0x830);
    g.write::<u32>(a + 8, o830);
    let q = u64::from(g.read::<u32>(part)) | (u64::from(g.read::<u32>(part + 4)) << 32);
    // `shl ax, cl` / `shl dx, cl`: 16-bit, the count below 16
    let w1 = u32::from(1u16 << ((q >> 26) & 0xf));
    let w2 = u32::from(1u16 << ((q >> 30) & 0xf));
    g.write::<u32>(a + 0x10, w1);
    let lock = rd(g, 0x32a);
    let old = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    g.write::<u32>(a + 0x14, w2);
    g.write::<u32>(a + 0xc, old);
    let blk = a + 0x220;
    let lo = q as u32;
    // call width: 7
    let shown = g
        .call(
            t(g, 0x35f, 885),
            0,
            &[lo & 0x3fff, (lo >> 20) & 0x3f, 0, 0, w1, w2, blk],
        )
        .eax;
    g.write::<u32>(rd(g, 0x37f), old);
    if shown == 0 {
        return;
    }
    let kc = k(g, 0x39d);
    let col = a + 0x90;
    let bytes: [u8; 4] = core::array::from_fn(|i| g.read::<u8>(d + 0x18 + i as u32));
    for (i, x) in (0u32..).zip(bytes) {
        g.write::<f32>(col + 4 * i, mul(cvt(u32::from(x)), kc));
    }
    let alpha = mul(cvt(u32::from(bytes[3])), kc);
    // idiv: the record's words by the size in 16.16, the remainder's
    // sixteenths by the size
    let step = |num: u32, size: u32| {
        let r = num.cast_signed().wrapping_rem((size << 16).cast_signed());
        (r >> 4).wrapping_div(size.cast_signed()).cast_unsigned()
    };
    let su = step(g.read::<u32>(d + 0x10), w1);
    let sv = step(g.read::<u32>(d + 0x14), w2);
    let u = mul(cvt(su), k(g, 0x451));
    let v = mul(cvt(sv), k(g, 0x47f));
    g.write::<u32>(rd(g, 0x46e), 1);
    // call width: 10
    g.call(
        t(g, 0x4b0, 879),
        0,
        &[
            o830,
            blk,
            u.to_bits(),
            v.to_bits(),
            col,
            alpha.to_bits(),
            a + 0x120,
            cvt(w1).to_bits(),
            cvt(w2).to_bits(),
            0,
        ],
    );
    g.write::<u32>(rd(g, 0x4c1), old);
}

/// [`draw_0708`] (1721 bytes, 1 overlay).
pub const DRAW_0708: Sig = Sig {
    len: 1721,
    hash: 0xea98_456a_560c_9add,
    calls: &[
        (0x85, Callee::Sig(&crate::m0708::FADE)),
        (0x20f, Callee::Sig(&crate::m0708::BIND_PARTS)),
        (0x58f, Callee::Sig(&crate::m0708::RISE)),
        (0x5aa, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0x5ba, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0x5d2, Callee::Sig(&crate::m0708::RISE)),
        // __security_check_cookie
        (0x6b1, Callee::Any),
    ],
    imports: &[],
};
