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

/// The spin set-up of `magic_0612` (0x10008860, 1615 bytes) and
/// `magic_0563` (0x10003f30, the same code with other globals and two
/// records instead of one: the loop bound, the immediate at `+0x534`);
/// internal, no arguments, `esp` the entry stack pointer; a cookie
/// frame, `ebp = E - 4`, the matrix `M` at `ebp - 0xa4`, the points at
/// `ebp - 0x34`, `- 0x44`, `- 0x64`, `- 0x54`, the vector `V` at `ebp -
/// 0x14`, the half span `H` at `ebp - 0x24`. With the state `S = *G_S`
/// and the object `*G_OBJ`: its bone points 0x11 (`A`) and 0x37 (`B`)
/// (slot 237), the middle `B + (A - B)` into `S + 0x3620` (its `y` less
/// `K1`, `w` 1.0), `H = A - B`; the matrix `S + 0x35a0` (slot 235) turned
/// by `S + 0x3638`, `+0x3630`, `+0x3634` (slots 330, 328, 329); `S +
/// 0x35d8` -100 while `S + 0x3640` steps by `K2` (wrapped once by `K4` at
/// `K3`/`K5`) and turns `M` (slots 235, 328), slot 231 `(S + 0x35a0, S +
/// 0x35a0, M)`; `V = (0, 0, -40, 1)` through it (slot 229) is added to
/// `S + 0x3620`, `S + 0x35d8` 0. The ground `y0 = ` slot 12 `(*G_OBJ)`
/// less `K6`; bone points 0x92 and 0xa3 (slot 237) start the records
/// taken from the pool ([`pool_take`](crate::sb_misc::pool_take)), one
/// per point up to the bound or the first failed take: `+0x30` =
/// `(*G_X)[0x24]`, the u16s `+0x38` -1 and `+0x3a` 0x200, `+0x34` 1; `M`
/// (slot 235) turned by a random angle (slots 314 `(pi)`, 329), `V = (0,
/// 0, rand 10, 1)` through it into `H` (slot 229); the record's point the
/// bone point plus `H` (`x`), a random 1.0 (`y`) and `H`'s `z` (the old
/// `ebp - 0x1c`); `+0x10` rand 0.2, `+0x14` rand 0.1 less `K7`, `+0x18`
/// rand 0.2, `+0x1c` rand pi, `+0x20..` (0, 1e-4, 0, 1), `+0x4c..` (0x20,
/// 0x10, -16), `+0x3c` `(rand 0.2 + K6') K8`, the bytes `+0x67` 0x40 and
/// `+0x6b` 0, `+0x60` 0, its `y` held below `y0 - K6'`. When the count
/// `S + 0x3660` runs out: the steps `S + 0x3580..` towards the target
/// `S + 0x2b40` (its `y` less `K9`) over `K10`, scaled by `K11` and `K12`,
/// count 0x1e, `S + 0x3664` 1. Returns what `eax` holds (the last slot
/// 314's, or 0 after a failed take).
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn spin_setup_0612<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32) -> u32 {
    use crate::sb_sse::{add, div, mul, sub};
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let ebp = esp.wrapping_sub(4);
    let l = |off: u32| ebp.wrapping_sub(off);
    let rf = |g: &G, at: u32| g.read::<f32>(at);
    let s = g.read::<u32>(w(0x16));
    g.write::<u32>(l(0xb0), s);
    let (pa, pb) = (l(0x34), l(0x44));
    // call width: 4
    g.call(t(g, 0x20, 237), 0, &[g.read::<u32>(w(0x30)), 0x11, 0, pa]);
    g.write::<u32>(l(0x28), ONE);
    // call width: 4
    g.call(t(g, 0x41, 237), 0, &[g.read::<u32>(w(0x4b)), 0x37, 0, pb]);
    let (by, ay, bx, bz, ax, az) = (
        rf(g, pb + 4),
        rf(g, pa + 4),
        rf(g, pb),
        rf(g, pb + 8),
        rf(g, pa),
        rf(g, pa + 8),
    );
    let (dy, dx, dz) = (sub(ay, by), sub(ax, bx), sub(az, bz));
    let aw = g.read::<u32>(l(0x28));
    g.write::<u32>(l(0x18), aw);
    g.write::<u32>(l(0x38), ONE);
    let (my, mx, mz) = (add(by, dy), add(bx, dx), add(bz, dz));
    g.write::<f32>(l(0x24), dx);
    g.write::<f32>(l(0x20), dy);
    let my = sub(my, k(g, 0xbc));
    g.write::<f32>(s + 0x3620, mx);
    g.write::<f32>(s + 0x3628, mz);
    g.write::<u32>(s + 0x362c, ONE);
    g.write::<f32>(l(0x1c), dz);
    g.write::<f32>(s + 0x3624, my);
    let m0 = s + 0x35a0;
    g.call(t(g, 0x82, 235), 0, &[m0]);
    for (op, slot, off) in [
        (0xffu32, 330u32, 0x3638u32),
        (0x11e, 328, 0x3630),
        (0x13d, 329, 0x3634),
    ] {
        let a = g.read::<u32>(s + off);
        // call width: 3
        g.call(t(g, op, slot), 0, &[m0, m0, a]);
    }
    g.write::<u32>(s + 0x35d8, 0xc2c8_0000);
    let mut x = add(rf(g, s + 0x3640), k(g, 0x169));
    // comiss; jbe: unordered keeps the value
    if x > k(g, 0x173) {
        x = sub(x, k(g, 0x17d));
    } else if k(g, 0x187) > x {
        x = add(x, k(g, 0x194));
    }
    let m = l(0xa4);
    g.write::<f32>(s + 0x3640, x);
    // (the saved `ebx` under it: not passed)
    g.call(t(g, 0x1a1, 235), 0, &[m]);
    let a = g.read::<u32>(s + 0x3640);
    // call width: 3
    g.call(t(g, 0x1cb, 328), 0, &[m, m, a]);
    // call width: 3
    g.call(t(g, 0x1df, 231), 0, &[m0, m0, m]);
    let v = l(0x14);
    g.write::<u32>(s + 0x35d8, 0);
    for (i, x) in (0u32..).zip([0u32, 0, 0xc220_0000, ONE]) {
        g.write::<u32>(v + 4 * i, x);
    }
    // call width: 3
    g.call(t(g, 0x1f4, 229), 0, &[v, m0, v]);
    let x = add(rf(g, s + 0x3620), rf(g, v));
    g.write::<f32>(s + 0x3620, x);
    let y = add(rf(g, v + 4), rf(g, s + 0x3624));
    g.write::<f32>(s + 0x3624, y);
    let z = add(rf(g, s + 0x3628), rf(g, v + 8));
    g.write::<f32>(s + 0x3628, z);
    let tag = g.read::<u32>(g.read::<u32>(w(0x266)) + 0x24);
    g.write::<u32>(l(0xac), tag);
    let obj = g.read::<u32>(w(0x26c));
    let r = crate::ob_model::st0_f32(g.call(t(g, 0x27a, 12), 0, &[obj]).st0);
    g.write::<f32>(l(0xa8), r);
    let ground = sub(r, k(g, 0x295));
    let (p1, p2) = (l(0x64), l(0x54));
    // call width: 4
    g.call(t(g, 0x29e, 237), 0, &[g.read::<u32>(w(0x2b1)), 0x92, 0, p1]);
    g.write::<f32>(l(0xb4), ground);
    g.write::<u32>(l(0x58), ONE);
    // call width: 4
    g.call(t(g, 0x2c4, 237), 0, &[g.read::<u32>(w(0x2d1)), 0xa3, 0, p2]);
    g.write::<u32>(l(0x48), ONE);
    g.write::<u32>(v, 0);
    g.write::<u32>(v + 4, 0);
    g.write::<u32>(v + 0xc, ONE);
    let h = l(0x24);
    let bound = w(0x534) & 0xff;
    let random = |g: &mut G, op: u32, x: u32| {
        let o = g.call(t(g, op, 314), 0, &[x]);
        (crate::ob_model::st0_f32(o.st0), o.eax)
    };
    let mut eax = 0;
    for i in 0..bound {
        let rec = crate::sb_misc::pool_take(g, img, img.rel32_target(f + 0x309));
        if rec == 0 {
            eax = 0;
            break;
        }
        let tag = g.read::<u32>(l(0xac));
        g.write::<u32>(rec + 0x30, tag);
        g.write::<u16>(rec + 0x38, 0xffff);
        g.write::<u16>(rec + 0x3a, 0x200);
        g.write::<u32>(rec + 0x34, 1);
        g.call(t(g, 0x338, 235), 0, &[m]);
        let (a, _) = random(g, 0x34c, 0x4049_0fdb);
        g.write::<f32>(l(0xa8), a);
        // call width: 3
        g.call(t(g, 0x37b, 329), 0, &[m, m, a.to_bits()]);
        let (z, _) = random(g, 0x388, 0x4120_0000);
        g.write::<f32>(v + 8, z);
        // call width: 3
        g.call(t(g, 0x3b1, 229), 0, &[h, m, v]);
        let p = p1 + 0x10 * i;
        for j in 0..4u32 {
            let x = g.read::<u32>(p + 4 * j);
            g.write::<u32>(rec + 4 * j, x);
        }
        let x = add(rf(g, rec), rf(g, h));
        g.write::<f32>(rec, x);
        let (r1, _) = random(g, 0x3dd, ONE);
        g.write::<f32>(l(0xa8), r1);
        let y = add(r1, rf(g, rec + 4));
        g.write::<f32>(rec + 4, y);
        let z = add(rf(g, l(0x1c)), rf(g, rec + 8));
        g.write::<f32>(rec + 8, z);
        let (r2, _) = random(g, 0x40b, 0x3e4c_cccd);
        g.write::<f32>(rec + 0x10, r2);
        let (r3, _) = random(g, 0x436, 0x3dcc_cccd);
        g.write::<f32>(l(0xa8), r3);
        g.write::<f32>(rec + 0x14, sub(r3, k(g, 0x45b)));
        let (r4, _) = random(g, 0x460, 0x3e4c_cccd);
        g.write::<f32>(rec + 0x18, r4);
        let (r5, _) = random(g, 0x47c, 0x4049_0fdb);
        g.write::<f32>(rec + 0x1c, r5);
        for (off, x) in [(0x20u32, 0u32), (0x24, 0x38d1_b717), (0x28, 0), (0x2c, ONE)] {
            g.write::<u32>(rec + off, x);
        }
        let (r6, e) = random(g, 0x493, 0x3e4c_cccd);
        eax = e;
        g.write::<f32>(l(0xa8), r6);
        let k6 = k(g, 0x4d4);
        let top = rf(g, l(0xb4));
        let x = add(r6, k6);
        g.write::<u32>(rec + 0x4c, 0x20);
        g.write::<u32>(rec + 0x50, 0x10);
        g.write::<u32>(rec + 0x54, 0xffff_fff0);
        let x = mul(x, k(g, 0x500));
        g.write::<u8>(rec + 0x67, 0x40);
        g.write::<u8>(rec + 0x6b, 0);
        g.write::<f32>(rec + 0x3c, x);
        let y = rf(g, rec + 4);
        g.write::<u32>(rec + 0x60, 0);
        // comiss; jbe: unordered keeps the point
        if y > top {
            g.write::<f32>(rec + 4, sub(top, k6));
        }
    }
    let n = g.read::<u32>(s + 0x3660).wrapping_sub(1);
    g.write::<u32>(s + 0x3660, n);
    if n != 0 {
        return eax;
    }
    let k10 = k(g, 0x572);
    let x4 = div(sub(rf(g, s + 0x2b40), rf(g, s + 0x3620)), k10);
    let x5 = sub(rf(g, s + 0x2b44), k(g, 0x57a));
    let x3 = div(sub(rf(g, s + 0x2b48), rf(g, s + 0x3628)), k10);
    let x5 = div(sub(x5, rf(g, s + 0x3624)), k10);
    let (k11, k12) = (k(g, 0x596), k(g, 0x5a6));
    g.write::<f32>(s + 0x3580, mul(x4, k11));
    g.write::<f32>(s + 0x3588, mul(x3, k11));
    let x4 = div(mul(x4, k12), k10);
    let y5 = add(div(mul(x5, k12), k10), k(g, 0x5e7));
    let x3 = div(mul(x3, k12), k10);
    g.write::<u32>(s + 0x358c, 0);
    g.write::<f32>(s + 0x3590, x4);
    let x0 = sub(mul(x5, k11), k(g, 0x605));
    g.write::<f32>(s + 0x3598, x3);
    g.write::<u32>(s + 0x359c, 0);
    g.write::<f32>(s + 0x3594, y5);
    g.write::<u32>(s + 0x3660, 0x1e);
    g.write::<f32>(s + 0x3584, x0);
    g.write::<u32>(s + 0x3664, 1);
    eax
}

/// [`spin_setup_0612`] in `magic_0612` (1615 bytes, 1 overlay).
pub const SPIN_SETUP_0612: Sig = Sig {
    len: 1615,
    hash: 0xe5a8_fb4e_4850_1a67,
    calls: &[
        (0x309, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
        // __security_check_cookie
        (0x647, Callee::Any),
    ],
    imports: &[],
};

/// [`spin_setup_0612`] in `magic_0563` 0x10003f30 (1615 bytes, 1
/// overlay; two records).
pub const SPIN_SETUP_0563: Sig = Sig {
    len: 1615,
    hash: 0x8f6e_fbf9_4475_8b0a,
    calls: &[
        (0x309, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
        // __security_check_cookie
        (0x647, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0573`'s main task (0x10007ee0, 2430 bytes; a task function,
/// cdecl `(task)`, the record `R = task[1]`; an 8-aligned cookie frame:
/// `A = ((E - 4) & !7) - 0x4c`, the task at `A + 0x1c`, a cursor or a
/// node record at `A + 0x20`, the particle records and the group at `A +
/// 4`, the free cursor `A + 0x34`, the colour words `A + 0x38`). By the
/// state `R + 0x18` (a jump table for 0..0x35; a state set and jumped to
/// runs at once), the count `R + 0x20`:
/// - 0: the model nodes `R + 0x24` and `R + 0xbc` (the DLL's vtable, the
///   list `G_L`, `*G_A` and the byte `*G_B`:
///   [`model_node_init`](crate::sa_summon::model_node_init)); the node `R +
///   0xea0` linked ([`link_node_x_ff`](crate::s4_small::link_node_x_ff),
///   `R + 0xbc`); the node `R + 0x39c` set up
///   ([`node_init_big_table`](crate::m0070::node_init_big_table)); `R +
///   0x4e8` = `R + 0x78`, `+0x4ec` 0x46; the node `R + 0xec8` added
///   ([`add_node`](crate::m0573::add_node), `R[0x110]`), the motion `R +
///   0xf34` ([`motion_setup`](crate::m0573::motion_setup)), the summon
///   `R + 0x1140` ([`summon_setup_0573`](crate::r23::summon_setup_0573)), the
///   effect node `R + 0xefc`
///   ([`effect_node_a`](crate::s4_fam446::effect_node_a), `(R + 0xec8,
///   0)`); bit 0 of `R + 0xf28`, `R + 0xf2c` 0, state 10;
/// - 10: the model `R + 0xbc` shown ([`show`](crate::m0070::show), pass
///   0xff), state 11, count 0x32;
/// - 11: at the count's end the nodes `R + 0x248`, `+0x4f0`, `+0x530`,
///   `+0x1ac`, `+0x970`, `+0xabc` appended to `G_L` with their words, the
///   particle nodes `R + 0x570` and `+0x670`
///   ([`particle_node`](crate::sb_nodes::particle_node)), state 20;
/// - 20: state 21, count 10;
/// - 21: at the count's end the nodes `R + 0x154`, `+0x180`, `+0xc08` and
///   the particle node `R + 0x770` (index 3), state 30;
/// - 30: state 31, count 14;
/// - 31: at the count's end the set-up `R + 0x1690`
///   ([`init`](crate::m0573::init), `(R + 0x39c, R + 0x154)`), state 50;
/// - 50: state 51, count 0x93;
/// - 51: at the count's end the node `R + 0x1d8` appended
///   ([`node_append`](crate::sb_misc::node_append)) with its colour (1, 1,
///   1, 0.17), the node `R + 0xd54` inserted
///   ([`node_insert`](crate::nodes::node_insert)), the particle node `R +
///   0x870` ([`particle_node_with`](crate::s3_nodes::particle_node_with),
///   4), `R + 0x770`'s colour from zero words
///   ([`colour_from_words`](crate::s4_small::colour_from_words)); state 52,
///   count 10;
/// - 52: at the count's end slot 239 `(*G_W, *G_B, 4)`, state 53;
/// - 53: unless bit 0 of `R + 0x858`, the particle node `R + 0x770` freed
///   ([`free_then_unlink`](crate::nodes::free_then_unlink)) and, unless
///   bit 2 of its `+0xe8`, `*G_P` = its `+0x50` and slot 713 `(+0x60)`;
///   state 10000;
/// - 10000: with `R + 0x1158` at 20000, bit 0 of `R + 0xec4` and state
///   10001;
/// - 10001: with `R + 0xf4c` at 20000, the node `R + 0x24` leaves
///   ([`leave`](crate::sa_misc::leave)), its model set up
///   ([`model_setup`](crate::sa_summon::model_setup)), the task's node
///   unlinked.
///
/// Every way out adds one to `R + 0x1c` and takes one from the count, and
/// returns what `eax` holds there.
#[allow(clippy::too_many_lines)] // the original's states, in order
pub fn main_task_0573<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r22::list_node_0645 as list_node;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    let a = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x4c);
    // after `push ebx; push esi; push edi`
    let s = a.wrapping_sub(0xc);
    g.write::<u32>(a + 0x1c, task);
    g.write::<u32>(a, r);
    let at = |c: u32, esp: u32, ecx: u32, edx: u32| Frame {
        at: call(c),
        esp,
        ecx,
        edx,
        ..*fr
    };
    // the particle record `[node, LIST, parent, *G_OWNER, PART, index]` at
    // `A + 4` (operands: the node's vtable, the list, the part)
    let particle = |g: &mut G, node: u32, ops: [u32; 4], parent: u32, owner: u32, i: u32| {
        let rec = a + 4;
        g.write::<u32>(rec, node);
        g.write::<u32>(node, w(ops[0]));
        for (j, x) in (1u32..).zip([w(ops[1]), parent, owner, w(ops[2]), i]) {
            g.write::<u32>(rec + 4 * j, x);
        }
        crate::sb_nodes::particle_node(g, img, call(ops[3]), rec)
    };
    let mut eax;
    loop {
        let state = g.read::<u32>(r + 0x18);
        eax = state;
        match state {
            0 => {
                let first = r + 0x24;
                let cur = a + 0x20;
                let g_a = g.read::<u32>(w(0x6c8));
                g.write::<u32>(cur, first);
                g.write::<u32>(first, w(0x6d5));
                g.write::<u32>(cur + 4, w(0x6e1));
                g.write::<u32>(cur + 8, g_a);
                g.write::<u8>(cur + 0xc, 0);
                crate::sa_summon::model_node_init(g, img, call(0x6ef), cur);
                let b = u32::from(g.read::<u8>(w(0x6f6)));
                let second = r + 0xbc;
                g.write::<u32>(cur, second);
                g.write::<u32>(cur + 4, w(0x70c));
                g.write::<u32>(cur + 8, b);
                g.write::<u8>(cur + 0xc, 0);
                g.write::<u32>(second, w(0x71b));
                crate::sa_summon::model_node_init(g, img, call(0x720), cur);
                // push ecx; push ebx: the `ecx` word (what the node init
                // leaves) is not read
                g.write::<u32>(s.wrapping_sub(4), 0);
                g.write::<u32>(s.wrapping_sub(8), second);
                crate::s4_small::link_node_x_ff(
                    g,
                    img,
                    &at(0x72d, s.wrapping_sub(0xc), r + 0xea0, 0),
                );
                let big = r + 0x39c;
                for (i, x) in (0u32..).zip([big, w(0x743), w(0x74b), w(0x753), w(0x75b)]) {
                    g.write::<u32>(cur + 4 * i, x);
                }
                crate::m0070::node_init_big_table(g, img, call(0x760), cur);
                let x = g.read::<u32>(r + 0x78);
                g.write::<u32>(r + 0x4e8, x);
                g.write::<u32>(r + 0x4ec, 0x46);
                let owner = g.read::<u32>(r + 0x110);
                g.write::<u32>(s.wrapping_sub(8), owner);
                let added = r + 0xec8;
                crate::m0573::add_node(g, img, &at(0x78d, s.wrapping_sub(0xc), added, 0));
                crate::m0573::motion_setup(
                    g,
                    img,
                    &at(0x79f, s.wrapping_sub(0xc), r + 0xf34, first),
                );
                g.write::<u32>(s.wrapping_sub(0xc), big);
                g.write::<u32>(s.wrapping_sub(0x10), first);
                crate::r23::summon_setup_0573(
                    g,
                    img,
                    &at(0x7b5, s.wrapping_sub(0x14), r + 0x1140, second),
                );
                // push 0; push ecx; push edi; push ecx: the `ecx` words
                // (what the set-up leaves) are not read
                for (i, x) in (1u32..).zip([0, added, 0, 0]) {
                    g.write::<u32>(s.wrapping_sub(0x14) + 4 * i, x);
                }
                eax = crate::s4_fam446::effect_node_a(
                    g,
                    img,
                    &at(0x7c8, s.wrapping_sub(0x14), r + 0xefc, 0),
                )
                .eax;
                let fl = g.read::<u32>(r + 0xf28) | 1;
                g.write::<u32>(r + 0xf28, fl);
                g.write::<u32>(r + 0xf2c, 0);
                g.write::<u32>(r + 0x18, 10);
                break;
            }
            10 | 11 => {
                if state == 10 {
                    let cur = a + 0x20;
                    g.write::<u32>(cur, r + 0xbc);
                    g.write::<u8>(cur + 4, 0xff);
                    eax = crate::m0070::show(g, img, call(0x66), cur);
                    g.write::<u32>(r + 0x18, 11);
                    g.write::<u32>(r + 0x20, 0x32);
                } else {
                    eax = 2;
                }
                if g.read::<i32>(r + 0x20) > 0 {
                    break;
                }
                let n1 = r + 0x248;
                list_node(
                    g,
                    img,
                    f,
                    n1,
                    [0x93, 0x9a, 0x9f, 0xa7, 0xac, 0xb4, 0xc0, 0xc6],
                );
                for (off, op) in [(0x13cu32, 0xd0u32), (0x144, 0xda), (0x140, 0xe4)] {
                    g.write::<u32>(n1 + off, w(op));
                }
                g.write::<u32>(n1 + 0x128, 0);
                g.write::<u8>(n1 + 0x148, 0);
                g.write::<u32>(n1, w(0xfb));
                let x = g.read::<u32>(r + 0x78);
                g.write::<u32>(r + 0x394, x);
                g.write::<u32>(r + 0x398, 0xc);
                // two nodes pointing back at the first
                let lit = [
                    (
                        r + 0x4f0,
                        [0x127, 0x12e, 0x133, 0x13b, 0x140, 0x148, 0x154, 0x15a],
                        0x16a,
                        0x51c,
                        0xbecc_cccd,
                    ),
                    (
                        r + 0x530,
                        [0x1a1, 0x1a8, 0x1ad, 0x1b5, 0x1ba, 0x1c2, 0x1ce, 0x1d4],
                        0x1e4,
                        0x55c,
                        0x3ecc_cccd,
                    ),
                ];
                for (node, ops, vt, v, z) in lit {
                    list_node(g, img, f, node, ops);
                    g.write::<u32>(node + 0x38, ONE);
                    g.write::<u32>(node + 0x3c, n1);
                    g.write::<u32>(node, w(vt));
                    for (i, x) in (0u32..).zip([0x3fd3_3333u32, 0xbe0f_5c29, z]) {
                        g.write::<u32>(r + v + 4 * i, x);
                    }
                }
                let (n2, n3) = (r + 0x4f0, r + 0x530);
                let n4 = r + 0x1ac;
                list_node(
                    g,
                    img,
                    f,
                    n4,
                    [0x21b, 0x222, 0x227, 0x22f, 0x234, 0x23c, 0x248, 0x24e],
                );
                g.write::<u32>(n4, w(0x254));
                for (i, x) in (0u32..).zip([ONE, ONE, ONE, 0x3f33_3333]) {
                    g.write::<u32>(r + 0x1c8 + 4 * i, x);
                }
                for (node, vt, ops, owner_op, b, c, blk, pops) in [
                    (
                        r + 0x970,
                        0x288u32,
                        [0x29b, 0x2a2, 0x2a7, 0x2af, 0x2b4, 0x2bc, 0x2c8, 0x2d3],
                        0x2cdu32,
                        (n2, n4),
                        (0x2efu32, 0x2f9u32),
                        r + 0x570,
                        [0x30a, 0x316, 0x326, 0x333],
                    ),
                    (
                        r + 0xabc,
                        0x33f,
                        [0x352, 0x359, 0x35e, 0x366, 0x36b, 0x373, 0x37f, 0x385],
                        0x39c,
                        (n3, n4),
                        (0x3ac, 0x3b6),
                        r + 0x670,
                        [0x3c7, 0x3d3, 0x3e3, 0x3f0],
                    ),
                ] {
                    g.write::<u32>(node, w(vt));
                    list_node(g, img, f, node, ops);
                    let owner = g.read::<u32>(w(owner_op));
                    g.write::<u32>(node + 0x140, b.1);
                    g.write::<u32>(node + 0x13c, b.0);
                    g.write::<u32>(node + 0x144, w(c.0));
                    g.write::<u32>(node + 0x128, w(c.1));
                    g.write::<u8>(node + 0x148, 0);
                    particle(g, blk, pops, node, owner, 1);
                }
                g.write::<u32>(r + 0x18, 0x14);
            }
            0x14 => {
                g.write::<u32>(r + 0x20, 10);
                g.write::<u32>(r + 0x18, 0x15);
                eax = 0x15;
                break;
            }
            0x15 => {
                eax = 4;
                if g.read::<i32>(r + 0x20) > 0 {
                    break;
                }
                let n7 = r + 0x154;
                list_node(
                    g,
                    img,
                    f,
                    n7,
                    [0x42c, 0x433, 0x438, 0x440, 0x445, 0x44d, 0x459, 0x45f],
                );
                g.write::<u32>(n7, w(0x465));
                for (i, x) in (0u32..).zip([0x4140_0000u32, 0xc040_0000, 0xc040_0000, ONE]) {
                    g.write::<u32>(r + 0x170 + 4 * i, x);
                }
                let n8 = r + 0x180;
                list_node(
                    g,
                    img,
                    f,
                    n8,
                    [0x4a6, 0x4ad, 0x4b2, 0x4ba, 0x4bf, 0x4c7, 0x4d3, 0x4d9],
                );
                g.write::<u32>(n8, w(0x4df));
                for (i, x) in (0u32..).zip([ONE, ONE, ONE, 0x3da3_d70a]) {
                    g.write::<u32>(r + 0x19c + 4 * i, x);
                }
                let n9 = r + 0xc08;
                g.write::<u32>(n9, w(0x519));
                list_node(
                    g,
                    img,
                    f,
                    n9,
                    [0x52c, 0x533, 0x538, 0x540, 0x545, 0x54d, 0x559, 0x564],
                );
                let owner = g.read::<u32>(w(0x55e));
                g.write::<u32>(n9 + 0x13c, n7);
                g.write::<u32>(n9 + 0x144, w(0x57a));
                g.write::<u32>(n9 + 0x140, n8);
                g.write::<u32>(n9 + 0x128, r + 0x39c);
                g.write::<u8>(n9 + 0x148, 0);
                particle(g, r + 0x770, [0x597, 0x5a3, 0x5b3, 0x5c0], n9, owner, 3);
                g.write::<u32>(r + 0x18, 0x1e);
            }
            0x1e => {
                g.write::<u32>(r + 0x20, 0xe);
                g.write::<u32>(r + 0x18, 0x1f);
                eax = 0x1f;
                break;
            }
            0x1f => {
                eax = 6;
                if g.read::<i32>(r + 0x20) > 0 {
                    break;
                }
                g.write::<u32>(s.wrapping_sub(4), r + 0x154);
                crate::m0573::init(g, img, &at(0x5fb, s.wrapping_sub(8), r + 0x1690, r + 0x39c));
                g.write::<u32>(r + 0x18, 0x32);
            }
            0x32 => {
                g.write::<u32>(r + 0x18, 0x33);
                g.write::<u32>(r + 0x20, 0x93);
                eax = 7;
                break;
            }
            0x33 => {
                eax = 8;
                if g.read::<i32>(r + 0x20) > 0 {
                    break;
                }
                let (node, list) = (r + 0x1d8, w(0x809));
                crate::sb_misc::node_append(g, img, call(0x80e), node, list, eax);
                let ins = r + 0xd54;
                for (i, x) in (0u32..).zip([ONE, ONE, ONE, 0x3e2e_147b]) {
                    g.write::<u32>(r + 0x1f4 + 4 * i, x);
                }
                // the group: the append leaves `ecx` = the node, `edx` = the
                // list
                let group = a + 4;
                for (i, x) in (0u32..).zip([ins, list, r + 0x39c, r + 0x154, w(0x868), node]) {
                    g.write::<u32>(group + 4 * i, x);
                }
                crate::nodes::node_insert(g, img, &at(0x86d, s.wrapping_sub(4), group, list));
                // push 4; sub esp, 8; push the node: the two middle words
                // are never written (the callee does not read them)
                let esp = s.wrapping_sub(0x14);
                g.write::<u32>(esp + 4, ins);
                g.write::<u32>(esp + 0x10, 4);
                crate::s3_nodes::particle_node_with(g, img, &at(0x87e, esp, r + 0x870, list));
                let words = a + 0x38;
                for i in 0..4u32 {
                    g.write::<u32>(words + 4 * i, 0);
                }
                eax = crate::s4_small::colour_from_words(
                    g,
                    img,
                    &at(0x8b0, s.wrapping_sub(8), r + 0x770, words),
                )
                .eax;
                g.write::<u32>(r + 0x18, 0x34);
                g.write::<u32>(r + 0x20, 10);
                break;
            }
            0x34 => {
                eax = 9;
                if g.read::<i32>(r + 0x20) > 0 {
                    break;
                }
                let b = u32::from(g.read::<u8>(w(0x8d3)));
                // call width: 3
                eax = g
                    .call(t(g, 0x8db, 239), 0, &[g.read::<u32>(w(0x8e1)), b, 4])
                    .eax;
                g.write::<u32>(r + 0x18, 0x35);
                break;
            }
            0x35 => {
                eax = 10;
                if g.read::<u8>(r + 0x858) & 1 != 0 {
                    break;
                }
                let p = r + 0x770;
                let cur = a + 0x34;
                g.write::<u32>(cur, p);
                crate::nodes::free_then_unlink(g, img, &at(0x627, s.wrapping_sub(4), cur, 0));
                if g.read::<u8>(p + 0xe8) & 4 == 0 {
                    let x = g.read::<u32>(p + 0x50);
                    g.write::<u32>(g.read::<u32>(w(0x635)), x);
                    g.call(t(g, 0x63f, 713), 0, &[p + 0x60]);
                }
                g.write::<u32>(r + 0x18, 0x2710);
            }
            0x2710 => {
                if g.read::<u32>(r + 0x1158) == 0x4e20 {
                    let fl = g.read::<u32>(r + 0xec4) | 1;
                    g.write::<u32>(r + 0xec4, fl);
                    g.write::<u32>(r + 0x18, 0x2711);
                }
                break;
            }
            0x2711 => {
                if g.read::<u32>(r + 0xf4c) != 0x4e20 {
                    break;
                }
                let first = r + 0x24;
                crate::sa_misc::leave(g, first, eax);
                // node_leave leaves `ecx` = the node (it never writes it)
                let cur = a + 0x1c;
                g.write::<u32>(cur, first);
                crate::sa_summon::model_setup(g, img, &at(0x691, s.wrapping_sub(4), cur, 0));
                let node = g.read::<u32>(task);
                let (prev, next) = (g.read::<u32>(node + 4), g.read::<u32>(node + 8));
                g.write::<u32>(prev + 8, next);
                let (next, prev) = (g.read::<u32>(node + 8), g.read::<u32>(node + 4));
                g.write::<u32>(next + 4, prev);
                g.write::<u32>(node + 8, node);
                g.write::<u32>(node + 4, node);
                eax = prev;
                break;
            }
            s if s <= 0x35 => {
                eax = 11;
                break;
            }
            _ => break,
        }
    }
    let n = g.read::<u32>(r + 0x1c).wrapping_add(1);
    g.write::<u32>(r + 0x1c, n);
    let c = g.read::<u32>(r + 0x20).wrapping_sub(1);
    g.write::<u32>(r + 0x20, c);
    Ret::eax(eax)
}

/// [`main_task_0573`] (2430 bytes, 1 overlay).
pub const MAIN_TASK_0573: Sig = Sig {
    len: 2430,
    hash: 0x0b3f_8412_fb17_b68f,
    calls: &[
        (0x66, Callee::Sig(&crate::m0070::SHOW)),
        (0x333, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x3f0, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x5c0, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x5fb, Callee::Sig(&crate::m0573::INIT)),
        (0x627, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0x684, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x691, Callee::Sig(&crate::sa_summon::MODEL_SETUP)),
        // __security_check_cookie
        (0x6bf, Callee::Any),
        (0x6ef, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0x720, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0x72d, Callee::Sig(&crate::s4_small::LINK_NODE_X_FF)),
        (0x760, Callee::Sig(&crate::m0070::NODE_INIT_BIG_TABLE)),
        (0x78d, Callee::Sig(&crate::m0573::ADD_NODE)),
        (0x79f, Callee::Sig(&crate::m0573::MOTION_SETUP)),
        (0x7b5, Callee::Sig(&crate::r23::SUMMON_SETUP_0573)),
        (0x7c8, Callee::Sig(&crate::s4_fam446::EFFECT_NODE_A)),
        (0x80e, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x86d, Callee::Sig(&crate::nodes::INSERT)),
        (0x87e, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x8b0, Callee::Sig(&crate::s4_small::COLOUR_FROM_WORDS)),
    ],
    imports: &[],
};

/// `magic_0514`'s summon task (0x10007de0, 2193 bytes; a task function,
/// cdecl `(task)`, the record `R = task[1]`; an 8-aligned frame without
/// a cookie: `A = ((E - 4) & !7) - 0x4c`, the cursors and groups at `A +
/// 8`, `+0xc`, `+0x14`, `+0x1c`, `+0x24`, `+0x34`). By the state `R +
/// 0x18` (a jump table for 0..0x14; a state set and jumped to runs at
/// once), the count `R + 0x20`, the time `R + 0x1c`:
/// - 0: slot 719 `(PART, 5, 0)` into slot 306 `(7, 0, part)`; the model
///   nodes `R + 0x514` and `R + 0x24` (`*G_A`;
///   [`model_node_init`](crate::sa_summon::model_node_init),
///   [`model_nodes`](crate::s3_summon::model_nodes)); the nodes `R +
///   0x5ac`, `+0x5d8`, `+0x65c`, `+0x604`, `+0x630` appended to `G_L`;
///   slot 262 into `R + 0x5c8..`, the model states `R + 0x5f4`, `+0x64c`
///   ([`model_state`](crate::m0070::model_state)); the count `R + 0x67c`
///   from the model's alpha (slot 333, or slot 601's value) times `K2`
///   over `K3`; the nodes `R + 0x688` and `R + 0x7d4` inserted
///   ([`node_insert`](crate::nodes::node_insert)); slot 268 `(R[0x94],
///   0x11, R + 0x620)`; the colour node `G_C`
///   ([`colour_node`](crate::s3_misc::colour_node)) and its words (`K4 -
///   c`, 0.5, 0.03); the effect node `G_E`
///   ([`effect_node`](crate::sb_nodes::effect_node)); the task `R + 0x920`
///   on the DLL's ring; the scaled object `R + 0x26f0`
///   ([`scaled_init`](crate::m0514::scaled_init)); state 10;
/// - 10: from the time 0x28 the colour object `R + 0x950`
///   ([`colour_setup`](crate::m0514::colour_setup)) and state 11;
/// - 11: from the time 0x28 state 0x1e;
/// - 0x1e: the timed object `R + 0xc20`
///   ([`timed_init`](crate::m0514::timed_init)), state 0x1f, count 0x27;
/// - 0x1f: at the count's end state 0x20, count 0x59, `R + 0x938` 10;
/// - 0x20: at the count's end `R + 0x968` 20, `R + 0xc38` 10000, state
///   20;
/// - 20: state 21, count 20;
/// - 21: at the count's end the colour node's words again (`K - c`, 1.0,
///   0.02, its flags bit 2), state 10000;
/// - 10000: when `R + 0x2708`, `+0xc38`, `+0x968` and `+0x938` share a
///   bit of 0x4e20, the model nodes `R + 0x24` and `R + 0x514` leave
///   ([`leave`](crate::sa_misc::leave)), the second's model set up
///   ([`model_setup`](crate::sa_summon::model_setup)), the task's node
///   unlinked.
///
/// Every way out adds one to the time and takes one from the count, and
/// returns what `eax` holds there (the entry's `eax` where the original
/// never sets it).
#[allow(clippy::too_many_lines)] // the original's states, in order
pub fn summon_task_0514<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    let a = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x4c);
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
    let mut eax = fr.eax;
    let time = |g: &G| g.read::<i32>(r + 0x1c);
    loop {
        let state = g.read::<u32>(r + 0x18);
        match state {
            0 => {
                // call width: 3
                let part = g.call(t(g, 0x4a, 719), 0, &[w(0x59), 5, 0]).eax;
                // call width: 3
                g.call(t(g, 0x64, 306), 0, &[7, 0, part]);
                let m2 = r + 0x514;
                let cur = a + 0x24;
                let ga = g.read::<u32>(w(0x75));
                for (i, x) in (0u32..).zip([m2, w(0x8e), ga]) {
                    g.write::<u32>(cur + 4 * i, x);
                }
                g.write::<u8>(cur + 0xc, 0);
                g.write::<u32>(m2, w(0x9d));
                crate::sa_summon::model_node_init(g, img, call(0xa2), cur);
                let ga = g.read::<u32>(w(0xa7));
                let m1 = r + 0x24;
                let grp = a + 0x34;
                g.write::<u32>(grp, m1);
                g.write::<u32>(m1, w(0xb4));
                for (i, x) in (1u32..).zip([w(0xc0), ga, 0]) {
                    g.write::<u32>(grp + 4 * i, x);
                }
                g.write::<u8>(grp + 0x10, 0);
                crate::s3_summon::model_nodes(g, img, &at(0xd6, s.wrapping_sub(4), grp, 0));
                let n1 = r + 0x5ac;
                list_node(
                    g,
                    img,
                    f,
                    n1,
                    [0xef, 0xf6, 0xfb, 0x103, 0x108, 0x110, 0x11c, 0x122],
                );
                g.write::<u32>(n1, w(0x128));
                let p = r + 0x5c8;
                let m = g.read::<u32>(m2 + 0x54);
                // call width: 4
                g.call(t(g, 0x13b, 262), 0, &[m, p, p + 4, p + 8]);
                g.write::<u32>(r + 0x5d4, ONE);
                let n2 = r + 0x5d8;
                list_node(
                    g,
                    img,
                    f,
                    n2,
                    [0x16d, 0x174, 0x179, 0x181, 0x186, 0x18e, 0x19a, 0x1a6],
                );
                g.write::<u32>(n2, w(0x1b0));
                let me = a + 0xc;
                g.write::<u32>(me, m2);
                g.write::<u32>(me + 4, r + 0x5f4);
                crate::m0070::model_state(g, img, call(0x1bd), me);
                let n3 = r + 0x65c;
                g.write::<u32>(n3, w(0x1c9));
                list_node(
                    g,
                    img,
                    f,
                    n3,
                    [0x1dc, 0x1e3, 0x1e8, 0x1f0, 0x1f5, 0x1fd, 0x209, 0x214],
                );
                g.write::<u32>(r + 0x680, 0);
                g.write::<u32>(r + 0x678, 0);
                let x = if g.read::<u8>(r + 0x5a4) & 1 != 0 {
                    let o = g.call(t(g, 0x20e, 333), 0, &[g.read::<u32>(r + 0x568)]);
                    let v = crate::ob_model::st0_f32(o.st0);
                    // fstp dword [esp + 0x14] (one word pushed)
                    g.write::<f32>(a + 4, v);
                    sub(g.read::<f32>(w(0x24b)), v)
                } else {
                    let out = a + 8;
                    // call width: 2
                    g.call(t(g, 0x20e, 601), 0, &[g.read::<u32>(r + 0x564), out]);
                    sub(g.read::<f32>(w(0x271)), g.read::<f32>(out))
                };
                let x = div(mul(x, g.read::<f32>(w(0x282))), g.read::<f32>(w(0x296)));
                let n5 = r + 0x688;
                for (i, x) in (1u32..).zip([w(0x28e), w(0x29e)]) {
                    g.write::<u32>(grp + 4 * i, x);
                }
                g.write::<u32>(grp + 0x10, n3);
                g.write::<u32>(r + 0x67c, crate::s4_summon::trunc(x));
                g.write::<u32>(grp, n5);
                g.write::<u32>(grp + 0xc, n1);
                g.write::<u32>(grp + 0x14, n2);
                crate::nodes::node_insert(g, img, &at(0x2c9, s.wrapping_sub(4), grp, 0));
                let n4 = r + 0x604;
                list_node(
                    g,
                    img,
                    f,
                    n4,
                    [0x2e2, 0x2e9, 0x2ee, 0x2f6, 0x2fb, 0x303, 0x30f, 0x320],
                );
                let v = r + 0x620;
                g.write::<u32>(n4, w(0x327));
                // call width: 3
                g.call(t(g, 0x314, 268), 0, &[g.read::<u32>(r + 0x94), 0x11, v]);
                g.write::<u32>(v + 0xc, ONE);
                g.write::<u32>(r + 0x62c, ONE);
                let n6 = r + 0x630;
                list_node(
                    g,
                    img,
                    f,
                    n6,
                    [0x364, 0x36b, 0x370, 0x378, 0x37d, 0x385, 0x391, 0x3a4],
                );
                let me = a + 0x14;
                g.write::<u32>(me, r + 0x40);
                g.write::<u32>(n6, w(0x3ae));
                g.write::<u32>(me + 4, r + 0x64c);
                crate::m0070::model_state(g, img, call(0x3b7), me);
                for (i, x) in (0u32..).zip([r + 0x7d4, w(0x3d3), w(0x3db), n4, n3, n6]) {
                    g.write::<u32>(grp + 4 * i, x);
                }
                crate::nodes::node_insert(g, img, &at(0x3ec, s.wrapping_sub(4), grp, 0));
                let cg = a + 0x1c;
                g.write::<u32>(cg, w(0x3f8));
                g.write::<u32>(cg + 4, w(0x400));
                g.write::<u32>(w(0x406), w(0x40a));
                crate::s3_misc::colour_node(g, img, &at(0x40f, s.wrapping_sub(4), cg, 0));
                let k4 = g.read::<f32>(w(0x417));
                let fl = g.read::<u32>(w(0x428)) & !1;
                // the flags first (before the pairs)
                let first = sub(k4, g.read::<f32>(w(0x422)));
                g.write::<u32>(w(0x428), fl);
                g.write::<u32>(w(0x439), 0x3f00_0000);
                g.write::<f32>(w(0x445), first);
                let half = 0x3f00_0000u32;
                let pairs = [
                    (
                        0x450u32,
                        0x480u32,
                        &[0x456u32, 0x460, 0x46a, 0x474][..],
                        half,
                    ),
                    (0x48b, 0x4bb, &[0x491, 0x49b][..], half),
                ];
                for (rd, wr, cs, v) in pairs {
                    let x = sub(k4, g.read::<f32>(w(rd)));
                    for &o in cs {
                        g.write::<u32>(w(o), v);
                    }
                    if rd == 0x48b {
                        g.write::<u32>(w(0x4a5), 0x3cf5_c28f);
                        g.write::<u32>(w(0x4af), 0);
                    }
                    g.write::<f32>(w(wr), x);
                }
                let x = sub(k4, g.read::<f32>(w(0x4c6)));
                let er = a + 0x34;
                for (i, x) in (0u32..).zip([w(0x4ce), w(0x4d6), n5, w(0x4e2)]) {
                    g.write::<u32>(er + 4 * i, x);
                }
                g.write::<f32>(w(0x4ea), x);
                let x = sub(k4, g.read::<f32>(w(0x4f5)));
                g.write::<u32>(er + 0x10, w(0x4fd));
                g.write::<u32>(er + 0x14, 0);
                g.write::<f32>(w(0x50d), x);
                let x = sub(k4, g.read::<f32>(w(0x518)));
                g.write::<f32>(w(0x520), x);
                let x = sub(g.read::<f32>(w(0x528)), g.read::<f32>(w(0x530)));
                g.write::<f32>(w(0x538), x);
                let x = sub(k4, g.read::<f32>(w(0x543)));
                g.write::<f32>(w(0x54b), x);
                crate::sb_nodes::effect_node(g, img, call(0x550), er);
                let node = g.read::<u32>(er);
                g.write::<u32>(node, w(0x560));
                let fl = g.read::<u32>(w(0x566)) | 1;
                g.write::<u32>(w(0x566), fl);
                g.write::<u32>(w(0x56d), 0);
                // the task `R + 0x920` put last on the DLL's ring
                let tk = r + 0x920;
                g.write::<u32>(tk, w(0x577));
                g.write::<u32>(tk + 8, tk);
                g.write::<u32>(tk + 4, tk);
                g.write::<u32>(tk + 0x10, w(0x584));
                g.write::<u32>(tk + 0x14, tk);
                let last = g.read::<u32>(w(0x58c));
                g.write::<u32>(tk + 8, last);
                g.write::<u32>(tk + 4, w(0x596));
                let last = g.read::<u32>(w(0x59b));
                g.write::<u32>(last + 4, tk);
                let fl = g.read::<u32>(w(0x5aa)) & !1;
                g.write::<u32>(w(0x5aa), fl);
                g.write::<u32>(w(0x5b1), tk);
                g.write::<u32>(tk + 0x20, m2);
                g.write::<u32>(tk + 0x18, 0);
                // sub esp, 8; push the node: the return address at `S -
                // 0x10` (the two words are never written)
                let esp = s.wrapping_sub(0x10);
                g.write::<u32>(esp + 4, n5);
                eax = crate::m0514::scaled_init(g, img, &at(0x5ca, esp, r + 0x26f0, m2));
                g.write::<u32>(r + 0x18, 10);
            }
            10 => {
                eax = 1;
                if time(g) >= 0x28 {
                    eax = crate::m0514::colour_setup(
                        g,
                        img,
                        &at(0x773, s.wrapping_sub(4), r + 0x950, r + 0x688),
                    );
                    g.write::<u32>(r + 0x18, 11);
                }
                break;
            }
            11 => {
                eax = 2;
                if time(g) < 0x28 {
                    break;
                }
                g.write::<u32>(r + 0x18, 0x1e);
            }
            0x14 => {
                g.write::<u32>(r + 0x20, 0x14);
                g.write::<u32>(r + 0x18, 0x15);
                eax = 0x15;
                break;
            }
            0x15 => {
                if g.read::<i32>(r + 0x20) > 0 {
                    break;
                }
                let k = g.read::<f32>(w(0x16));
                let x = sub(k, g.read::<f32>(w(0x611)));
                eax = (g.read::<u32>(w(0x616)) & !1) | 4;
                g.write::<u32>(w(0x622), ONE);
                g.write::<f32>(w(0x62e), x);
                let x = sub(k, g.read::<f32>(w(0x639)));
                for o in [0x63fu32, 0x649, 0x653, 0x65d] {
                    g.write::<u32>(w(o), ONE);
                }
                g.write::<f32>(w(0x669), x);
                let x = sub(k, g.read::<f32>(w(0x674)));
                g.write::<u32>(w(0x67a), ONE);
                g.write::<u32>(w(0x684), ONE);
                g.write::<u32>(w(0x68e), 0x3ca3_d70a);
                g.write::<u32>(w(0x698), 0);
                g.write::<f32>(w(0x6a4), x);
                let x = sub(k, g.read::<f32>(w(0x6af)));
                g.write::<u32>(w(0x6b4), eax);
                g.write::<u32>(w(0x6ba), ONE);
                g.write::<f32>(w(0x6c6), x);
                let x = sub(k, g.read::<f32>(w(0x6d1)));
                g.write::<f32>(w(0x6d9), x);
                let x = sub(k, g.read::<f32>(w(0x6e4)));
                g.write::<f32>(w(0x6ec), x);
                let x = sub(g.read::<f32>(w(0x6f4)), g.read::<f32>(w(0x6fc)));
                g.write::<f32>(w(0x704), x);
                let x = sub(k, g.read::<f32>(w(0x70f)));
                g.write::<f32>(w(0x717), x);
                g.write::<u32>(r + 0x18, 0x2710);
            }
            0x1e => {
                for (i, x) in (1u32..).zip([r + 0x7d4, r + 0x40, r + 0x65c]) {
                    g.write::<u32>(s.wrapping_sub(0x10) + 4 * i, x);
                }
                eax = crate::m0514::timed_init(
                    g,
                    img,
                    &at(0x7bb, s.wrapping_sub(0x10), r + 0xc20, r + 0x688),
                );
                g.write::<u32>(r + 0x18, 0x1f);
                // the count is never 0 here: the wait starts
                g.write::<u32>(r + 0x20, 0x27);
                break;
            }
            0x1f => {
                if g.read::<i32>(r + 0x20) > 0 {
                    break;
                }
                g.write::<u32>(r + 0x18, 0x20);
                let n = g.read::<u32>(r + 0x1c).wrapping_add(1);
                g.write::<u32>(r + 0x1c, n);
                g.write::<u32>(r + 0x20, 0x59);
                g.write::<u32>(r + 0x938, 10);
                return Ret::eax(eax);
            }
            0x20 => {
                if g.read::<i32>(r + 0x20) > 0 {
                    break;
                }
                g.write::<u32>(r + 0x968, 0x14);
                g.write::<u32>(r + 0xc38, 0x2710);
                g.write::<u32>(r + 0x18, 0x14);
            }
            0x2710 => {
                eax = [0x2708u32, 0xc38, 0x968, 0x938]
                    .iter()
                    .fold(u32::MAX, |x, &o| x & g.read::<u32>(r + o));
                if eax & 0x4e20 == 0 {
                    break;
                }
                let (m1, m2) = (r + 0x24, r + 0x514);
                crate::sa_misc::leave(g, m1, eax);
                crate::sa_misc::leave(g, m2, eax);
                // node_leave leaves `ecx` = the node (it never writes it)
                let cur = a + 8;
                g.write::<u32>(cur, m2);
                crate::sa_summon::model_setup(g, img, &at(0x83a, s.wrapping_sub(4), cur, 0));
                let node = g.read::<u32>(task);
                let (prev, next) = (g.read::<u32>(node + 4), g.read::<u32>(node + 8));
                g.write::<u32>(prev + 8, next);
                let (next, prev) = (g.read::<u32>(node + 8), g.read::<u32>(node + 4));
                g.write::<u32>(next + 4, prev);
                g.write::<u32>(node + 8, node);
                g.write::<u32>(node + 4, node);
                eax = prev;
                break;
            }
            1..=9 | 12..=19 => {
                eax = 4;
                break;
            }
            _ => break,
        }
    }
    let n = g.read::<u32>(r + 0x1c).wrapping_add(1);
    g.write::<u32>(r + 0x1c, n);
    let c = g.read::<u32>(r + 0x20).wrapping_sub(1);
    g.write::<u32>(r + 0x20, c);
    Ret::eax(eax)
}

/// [`summon_task_0514`] (2193 bytes, 1 overlay).
pub const SUMMON_TASK_0514: Sig = Sig {
    len: 2193,
    hash: 0xb828_6f76_74bb_6913,
    calls: &[
        (0xa2, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0xd6, Callee::Sig(&crate::s3_summon::MODEL_NODES)),
        (0x1bd, Callee::Sig(&crate::m0070::MODEL_STATE)),
        (0x2c9, Callee::Sig(&crate::nodes::INSERT)),
        (0x3b7, Callee::Sig(&crate::m0070::MODEL_STATE)),
        (0x3ec, Callee::Sig(&crate::nodes::INSERT)),
        (0x40f, Callee::Sig(&crate::s3_misc::COLOUR_NODE)),
        (0x550, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
        (0x5ca, Callee::Sig(&crate::m0514::SCALED_INIT)),
        (0x773, Callee::Sig(&crate::m0514::COLOUR_SETUP)),
        (0x7bb, Callee::Sig(&crate::m0514::TIMED_INIT)),
        (0x822, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x82d, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x83a, Callee::Sig(&crate::sa_summon::MODEL_SETUP)),
    ],
    imports: &[],
};

/// A node with its own empty child list appended to the children of
/// `parent` as the overlays inline it (`+0x1a` 0xff, `+0x10`/`+0xc`
/// itself, `+0x14` the parent; with a parent the links `+4`/`+8`, the
/// parent's first `+0xc` when it had none, its last `+0x10` and u16
/// count `+0x18`).
fn child_append<G: Game>(g: &mut G, node: u32, parent: u32) {
    g.write::<u16>(node + 0x1a, 0xff);
    g.write::<u32>(node + 0x10, node);
    g.write::<u32>(node + 0xc, node);
    g.write::<u32>(node + 0x14, parent);
    if parent == 0 {
        return;
    }
    g.write::<u32>(node + 4, parent);
    let last = g.read::<u32>(parent + 0x10);
    g.write::<u32>(node + 8, last);
    let last = g.read::<u32>(parent + 0x10);
    if last == parent {
        g.write::<u32>(parent + 0xc, node);
    } else {
        g.write::<u32>(last + 4, node);
    }
    let n = g.read::<u16>(parent + 0x18).wrapping_add(1);
    g.write::<u16>(parent + 0x18, n);
    g.write::<u32>(parent + 0x10, node);
}

/// `magic_0513`'s summon task (0x10007a90, 3382 bytes; a task function,
/// cdecl `(task)`, the record `R = task[1]`; an 8-aligned cookie frame:
/// `A = ((E - 4) & !7) - 0x64` and `S = A - 0xc` after the pushes, the
/// cursors and records at `S + 0xc`, `+0x28`, `+0x3c`, `+0x44`, `+0x4c`,
/// the vector `V` at `S + 0x5c`). By the state `R + 0x18` (a jump table
/// for 0..0x3d; a state set and jumped to runs at once), the count `R +
/// 0x50`, the time `R + 0x4c`:
/// - 0: the model nodes `R + 0x54c`, `R + 0x5c`
///   ([`model_node_init`](crate::sa_summon::model_node_init),
///   [`model_nodes`](crate::s3_summon::model_nodes)); the nodes `R +
///   0x5e4`, `+0x610`, `+0x694`, `+0x724`, `+0x63c`, `+0x668`, `+0x870`
///   on `G_L`, `R + 0x6c0` under `R + 0x5e4` and `R + 0x6f8` under it;
///   slots 262, 325, the model states `R + 0x62c`, `+0x684`
///   ([`model_state`](crate::m0070::model_state)); the count `R + 0x6b4`
///   from the model's alpha (slot 333, or slot 601's value); slots 211,
///   268, 12 (`R + 0x65c`), 234, 232, 251 times `K6` (233), 234; the
///   colour node `G_C` ([`colour_node`](crate::s3_misc::colour_node)) and
///   its words (`K4 - c`, 0.5, 0.03); the effect node `R + 0x9bc`
///   ([`effect_node`](crate::sb_nodes::effect_node)); the task `R +
///   0xaa8` on the DLL's ring (slot 325 into its `+0x28`); state 10;
/// - 10: the node `R + 0x13d0` ([`over_node`](crate::m0513::over_node)),
///   state 11, count 0x3c;
/// - 11: at the count's end state 30;
/// - 30: `R + 0x6e0` -10, state 31;
/// - 31: the node `R + 0xb00` ([`lit_node`](crate::m0513::lit_node)),
///   state 32, `R + 0xac0` 10, count 15;
/// - 32: at the count's end state 20;
/// - 20: slot 325 `(R + 0x2c, R + 0x600)`, `R + 0xac0` 0x1e, `R +
///   0x718` 0.5; the nodes `R + 0xa64`
///   ([`node_init`](crate::m0003::node_init)) and `R + 0x9f4` (effect
///   node, 1); state 21 with `R + 0x54`, `+0x58` 0, and on into 21;
/// - 21: the speed `R + 0x58` up by `K5` and the share `R + 0x54` by it,
///   from 1.0 on held there and state 22; the point `R + 0x600`/`+0x608`
///   the start `R + 0x2c` plus the share of the way `R + 0x1c`;
/// - 22: once `R + 0x65c` is not above `R + 0x604`: `R + 0x604` = it,
///   `R + 0x6e0`, `+0x718` 0, `*G_F` bit 0, the light block `G_LB`, the node
///   `R + 0xdf0` ([`between`](crate::m0513::between)), slot 268 into `V`,
///   the node `R + 0x10e0` ([`for_target`](crate::m0513::for_target)), the
///   effect node `R + 0xa2c` (2); state 60;
/// - 60: `R + 0xac0` 10000, state 61, count 0x1e;
/// - 61: at the count's end the colour node's words again (`K - c`, 1.0,
///   0.02, its flags bit 2), state 10000;
/// - 10000: with a bit of 0x4e20 in `R + 0xac0`, the task's node
///   unlinked.
///
/// Every way out adds one to the time and takes one from the count, and
/// returns what `eax` holds there.
#[allow(clippy::too_many_lines)] // the original's states, in order
pub fn summon_task_0513<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    let a = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x64);
    // after `push ebx; push esi; push edi`
    let s = a.wrapping_sub(0xc);
    g.write::<u32>(s + 0x34, task);
    let at = |c: u32, esp: u32, ecx: u32, edx: u32| Frame {
        at: call(c),
        esp,
        ecx,
        edx,
        ..*fr
    };
    let list_node = crate::r22::list_node_0645::<G>;
    let v = s + 0x5c;
    // the effect node record at `S + 0xc`: `[node, LIST, a, b, NAME, i]`
    let effect = |g: &mut G, node: u32, ops: [u32; 3], b: u32, i: u32, c: u32, vt: u32| {
        let rec = s + 0xc;
        g.write::<u32>(rec, node);
        g.write::<u32>(rec + 4, w(ops[0]));
        g.write::<u32>(rec + 8, ops[1]);
        g.write::<u32>(rec + 0xc, b);
        g.write::<u32>(rec + 0x10, w(ops[2]));
        g.write::<u32>(rec + 0x14, i);
        crate::sb_nodes::effect_node(g, img, call(c), rec);
        let n = g.read::<u32>(rec);
        g.write::<u32>(n, w(vt));
        let fl = g.read::<u32>(node + 0x2c) | 1;
        g.write::<u32>(node + 0x2c, fl);
        g.write::<u32>(node + 0x30, 0);
        n
    };
    let mut eax;
    loop {
        let state = g.read::<u32>(r + 0x18);
        eax = state;
        match state {
            0 => {
                let m1 = r + 0x54c;
                let cur = s + 0x4c;
                let ga = g.read::<u32>(w(0x5e));
                for (i, x) in (0u32..).zip([m1, w(0x74), ga]) {
                    g.write::<u32>(cur + 4 * i, x);
                }
                g.write::<u8>(cur + 0xc, 0);
                g.write::<u32>(m1, w(0x83));
                crate::sa_summon::model_node_init(g, img, call(0x88), cur);
                let ga = g.read::<u32>(w(0x8d));
                let m0 = r + 0x5c;
                let grp = s + 0xc;
                g.write::<u32>(grp, m0);
                g.write::<u32>(m0, w(0x9a));
                for (i, x) in (1u32..).zip([w(0xa6), ga, 0]) {
                    g.write::<u32>(grp + 4 * i, x);
                }
                g.write::<u8>(grp + 0x10, 0);
                crate::s3_summon::model_nodes(g, img, &at(0xbc, s.wrapping_sub(4), grp, 0));
                let n1 = r + 0x5e4;
                list_node(
                    g,
                    img,
                    f,
                    n1,
                    [0xd5, 0xdc, 0xe1, 0xe9, 0xee, 0xf6, 0x102, 0x10e],
                );
                g.write::<u32>(n1, w(0x121));
                let p = r + 0x600;
                // call width: 4
                g.call(
                    t(g, 0x11b, 262),
                    0,
                    &[g.read::<u32>(m1 + 0x54), p, p + 4, p + 8],
                );
                g.write::<u32>(r + 0x60c, ONE);
                // call width: 2
                g.call(t(g, 0x139, 325), 0, &[r + 0x3c, p]);
                let n2 = r + 0x610;
                list_node(
                    g,
                    img,
                    f,
                    n2,
                    [0x16b, 0x172, 0x177, 0x17f, 0x184, 0x18c, 0x198, 0x19e],
                );
                g.write::<u32>(n2, w(0x1a4));
                let me = s + 0x44;
                g.write::<u32>(me, m1);
                g.write::<u32>(me + 4, r + 0x62c);
                crate::m0070::model_state(g, img, call(0x1bb), me);
                let n3 = r + 0x694;
                g.write::<u32>(n3, w(0x1c7));
                list_node(
                    g,
                    img,
                    f,
                    n3,
                    [0x1da, 0x1e1, 0x1e6, 0x1ee, 0x1f3, 0x1fb, 0x207, 0x20d],
                );
                g.write::<u32>(r + 0x6b8, 0);
                g.write::<u32>(r + 0x6b0, 0);
                let x = if g.read::<u8>(m1 + 0x90) & 1 != 0 {
                    let o = g.call(t(g, 0x22f, 333), 0, &[g.read::<u32>(m1 + 0x54)]);
                    let y = crate::ob_model::st0_f32(o.st0);
                    // fstp dword [esp + 0x28] (one word pushed)
                    g.write::<f32>(s + 0x24, y);
                    sub(g.read::<f32>(w(0x246)), y)
                } else {
                    let out = s + 0x38;
                    // call width: 2
                    g.call(t(g, 0x25b, 601), 0, &[g.read::<u32>(m1 + 0x50), out]);
                    sub(g.read::<f32>(w(0x26e)), g.read::<f32>(out))
                };
                let x = div(mul(x, g.read::<f32>(w(0x27f))), g.read::<f32>(w(0x28d)));
                g.write::<u32>(r + 0x6b4, crate::s4_summon::trunc(x));
                let n4 = r + 0x724;
                g.write::<u32>(n4, w(0x29d));
                list_node(
                    g,
                    img,
                    f,
                    n4,
                    [0x2b0, 0x2b7, 0x2bc, 0x2c4, 0x2c9, 0x2d1, 0x2dd, 0x2e3],
                );
                g.write::<u32>(n4 + 0x144, n3);
                g.write::<u32>(n4 + 0x140, n2);
                g.write::<u32>(n4 + 0x13c, n1);
                g.write::<u32>(n4 + 0x128, w(0x317));
                g.write::<u8>(n4 + 0x148, 0);
                // call width: 2
                g.call(t(g, 0x307, 211), 0, &[g.read::<u32>(r + 0x5a0), n4 + 0x9c]);
                let fl = g.read::<u16>(r + 0x5dc) | 0x10;
                g.write::<u16>(r + 0x5dc, fl);
                g.write::<u32>(r + 0x598, n4);
                let n5 = r + 0x63c;
                list_node(
                    g,
                    img,
                    f,
                    n5,
                    [0x356, 0x35d, 0x362, 0x36a, 0x36f, 0x377, 0x383, 0x395],
                );
                g.write::<u32>(n5, w(0x39b));
                let obj = g.read::<u32>(r + 0xcc);
                // call width: 3
                g.call(t(g, 0x388, 268), 0, &[obj, 0x11, r + 0x658]);
                g.write::<u32>(r + 0x664, ONE);
                let obj = g.read::<u32>(r + 0xcc);
                let h = crate::ob_model::st0_f32(g.call(t(g, 0x3b0, 12), 0, &[obj]).st0);
                g.write::<f32>(r + 0x65c, h);
                g.write::<u32>(r + 0x664, ONE);
                let n6 = r + 0x668;
                list_node(
                    g,
                    img,
                    f,
                    n6,
                    [0x3f4, 0x3fb, 0x400, 0x408, 0x40d, 0x415, 0x421, 0x42a],
                );
                let me = s + 0x3c;
                g.write::<u32>(me, r + 0x78);
                g.write::<u32>(n6, w(0x434));
                g.write::<u32>(me + 4, r + 0x684);
                crate::m0070::model_state(g, img, call(0x447), me);
                let n7 = r + 0x870;
                g.write::<u32>(n7, w(0x453));
                list_node(
                    g,
                    img,
                    f,
                    n7,
                    [0x466, 0x46d, 0x472, 0x47a, 0x47f, 0x487, 0x493, 0x499],
                );
                g.write::<u32>(n7 + 0x144, n3);
                g.write::<u32>(n7 + 0x13c, n5);
                g.write::<u32>(n7 + 0x128, w(0x4b5));
                g.write::<u8>(n7 + 0x148, 0);
                g.write::<u32>(n7 + 0x140, n6);
                let c1 = r + 0x6c0;
                child_append(g, c1, n1);
                g.write::<u32>(c1 + 0x34, 0);
                g.write::<u32>(c1, w(0x50d));
                let x = g.read::<u32>(r + 0x65c);
                g.write::<u32>(r + 0x6f4, x);
                for o in [0x6ecu32, 0x6f0, 0x6dc, 0x6e0, 0x6e4, 0x6e8] {
                    g.write::<u32>(r + o, 0);
                }
                let c2 = r + 0x6f8;
                g.write::<u32>(c2, w(0x561));
                child_append(g, c2, c1);
                for o in [0x714u32, 0x718, 0x71c, 0x720] {
                    g.write::<u32>(r + o, 0);
                }
                let b = r + 0x1c;
                // call width: 3
                g.call(t(g, 0x5a6, 234), 0, &[b, r + 0x658, p]);
                // call width: 2
                g.call(t(g, 0x5e8, 232), 0, &[v, b]);
                let o = g.call(t(g, 0x5f5, 251), 0, &[g.read::<u32>(r + 0x59c)]);
                let y = crate::ob_model::st0_f32(o.st0);
                // fstp dword [esp + 0x30] (three words pushed)
                g.write::<f32>(s + 0x24, y);
                let y = mul(y, g.read::<f32>(w(0x615)));
                // call width: 3
                g.call(t(g, 0x628, 233), 0, &[v, v, y.to_bits()]);
                // call width: 3
                g.call(t(g, 0x63d, 234), 0, &[b, b, v]);
                let cg = s + 0x28;
                g.write::<u32>(cg, w(0x656));
                g.write::<u32>(cg + 4, w(0x65e));
                g.write::<u32>(w(0x664), w(0x668));
                crate::s3_misc::colour_node(g, img, &at(0x66d, s.wrapping_sub(4), cg, 0));
                let k4 = g.read::<f32>(w(0x675));
                let half = 0x3f00_0000u32;
                let x = sub(k4, g.read::<f32>(w(0x680)));
                let fl = g.read::<u32>(w(0x686)) & !1;
                g.write::<u32>(w(0x686), fl);
                for o in [0x68du32, 0x697, 0x6a1] {
                    g.write::<u32>(w(o), half);
                }
                g.write::<f32>(w(0x6ad), x);
                let x = sub(k4, g.read::<f32>(w(0x6b8)));
                for o in [0x6beu32, 0x6c8, 0x6d2, 0x6dc] {
                    g.write::<u32>(w(o), half);
                }
                g.write::<f32>(w(0x6e8), x);
                let x = sub(k4, g.read::<f32>(w(0x6f3)));
                g.write::<u32>(w(0x6f9), 0x3cf5_c28f);
                g.write::<u32>(w(0x703), 0);
                g.write::<f32>(w(0x70f), x);
                for (rd, wr) in [(0x71au32, 0x722u32), (0x72d, 0x735), (0x740, 0x748)] {
                    let x = sub(k4, g.read::<f32>(w(rd)));
                    g.write::<f32>(w(wr), x);
                }
                let x = sub(g.read::<f32>(w(0x750)), g.read::<f32>(w(0x758)));
                g.write::<f32>(w(0x760), x);
                let x = sub(k4, g.read::<f32>(w(0x76b)));
                let e1 = r + 0x9bc;
                g.write::<u32>(s + 0xc, e1);
                g.write::<f32>(w(0x787), x);
                effect(g, e1, [0x78f, n4, 0x7a3], w(0x79b), 0, 0x7b0, 0x7ba);
                // the task `R + 0xaa8` put last on the DLL's ring
                let tk = r + 0xaa8;
                g.write::<u32>(tk + 8, tk);
                g.write::<u32>(tk + 4, tk);
                g.write::<u32>(tk, w(0x7d7));
                g.write::<u32>(tk + 0x10, w(0x7de));
                g.write::<u32>(tk + 0x14, tk);
                let last = g.read::<u32>(w(0x7e6));
                g.write::<u32>(tk + 8, last);
                g.write::<u32>(tk + 4, w(0x7f0));
                let last = g.read::<u32>(w(0x7f5));
                g.write::<u32>(last + 4, tk);
                let fl = g.read::<u32>(w(0x7fe)) & !1;
                g.write::<u32>(w(0x7fe), fl);
                g.write::<u32>(w(0x812), tk);
                // call width: 2
                g.call(t(g, 0x80c, 325), 0, &[tk + 0x28, r + 0x3c]);
                g.write::<u32>(tk + 0x4c, n1);
                g.write::<u32>(tk + 0x48, m1);
                g.write::<u32>(tk + 0x18, 0);
                g.write::<u32>(r + 0x18, 10);
            }
            10 => {
                g.write::<u32>(s.wrapping_sub(4), r + 0x694);
                eax = crate::m0513::over_node(
                    g,
                    img,
                    &at(0x851, s.wrapping_sub(8), r + 0x13d0, r + 0x3c),
                );
                g.write::<u32>(r + 0x18, 11);
                // the count is never 0 here: the wait starts
                g.write::<u32>(r + 0x50, 0x3c);
                break;
            }
            11 => {
                eax = 2;
                if g.read::<i32>(r + 0x50) > 0 {
                    break;
                }
                g.write::<u32>(r + 0x18, 0x1e);
            }
            0x1e => {
                g.write::<u32>(r + 0x6e0, 0xc120_0000);
                g.write::<u32>(r + 0x18, 0x1f);
                eax = 6;
                break;
            }
            0x1f => {
                g.write::<u32>(s.wrapping_sub(4), r + 0x694);
                eax = crate::m0513::lit_node(
                    g,
                    img,
                    &at(0x889, s.wrapping_sub(8), r + 0xb00, r + 0x3c),
                );
                g.write::<u32>(r + 0x18, 0x20);
                g.write::<u32>(r + 0xac0, 10);
                g.write::<u32>(r + 0x50, 0xf);
                break;
            }
            0x20 => {
                eax = 8;
                if g.read::<i32>(r + 0x50) > 0 {
                    break;
                }
                g.write::<u32>(r + 0x18, 0x14);
            }
            0x14 | 0x15 => {
                if state == 0x14 {
                    g.write::<u32>(r + 0xac0, 0x1e);
                    g.write::<u32>(r + 0x718, 0x3f00_0000);
                    // call width: 2
                    g.call(t(g, 0xb75, 325), 0, &[r + 0x2c, r + 0x600]);
                    let n = r + 0xa64;
                    let cur = s + 0x28;
                    for (i, x) in (0u32..).zip([n, w(0xbb0), r + 0x54c]) {
                        g.write::<u32>(cur + 4 * i, x);
                    }
                    crate::m0003::node_init(g, img, call(0xbb9), cur);
                    g.write::<u8>(r + 0xaa4, 3);
                    let e = r + 0x9f4;
                    eax = effect(g, e, [0xbd6, w(0xbde), 0xbea], n, 1, 0xbf7, 0xc09);
                    g.write::<u32>(r + 0x18, 0x15);
                    g.write::<u32>(r + 0x54, 0);
                    g.write::<u32>(r + 0x58, 0);
                } else {
                    eax = 4;
                }
                let k = g.read::<f32>(w(0x35));
                let speed = add(g.read::<f32>(r + 0x58), g.read::<f32>(w(0xc32)));
                g.write::<f32>(r + 0x58, speed);
                let mut share = add(speed, g.read::<f32>(r + 0x54));
                g.write::<f32>(r + 0x54, share);
                // comiss; jb: below or unordered keeps going
                if !(share.is_nan() || share < k) {
                    g.write::<u32>(r + 0x18, 0x16);
                    g.write::<u32>(r + 0x54, ONE);
                    share = 1.0;
                }
                let x = add(mul(g.read::<f32>(r + 0x1c), share), g.read::<f32>(r + 0x2c));
                g.write::<f32>(r + 0x600, x);
                let z = add(mul(g.read::<f32>(r + 0x24), share), g.read::<f32>(r + 0x34));
                g.write::<f32>(r + 0x608, z);
                break;
            }
            0x16 => {
                eax = 5;
                let h = g.read::<f32>(r + 0x65c);
                // comiss; ja: above waits (unordered goes on)
                if h > g.read::<f32>(r + 0x604) {
                    break;
                }
                g.write::<f32>(r + 0x604, h);
                g.write::<u32>(r + 0x6e0, 0);
                g.write::<u32>(r + 0x718, 0);
                let b = g.read::<u8>(w(0x8ed)) | 1;
                g.write::<u8>(w(0x8ed), b);
                let k = g.read::<f32>(w(0x35));
                // xmm2 is 0.0 here
                g.write::<f32>(w(0x910), sub(0.0, 0.0));
                let n1 = r + 0x5e4;
                for (o, x) in [
                    (0x91fu32, 0u32),
                    (0x929, 0x40a0_0000),
                    (0x933, 0x40e0_0000),
                    (0x93d, 0x41a0_0000),
                    (0x947, 0x41a0_0000),
                    (0x951, 0),
                ] {
                    g.write::<u32>(w(o), x);
                }
                g.write::<f32>(w(0x95d), sub(0.0, k));
                g.write::<u32>(s.wrapping_sub(4), r + 0x694);
                crate::m0513::between(g, img, &at(0x962, s.wrapping_sub(8), r + 0xdf0, n1));
                let obj = g.read::<u32>(r + 0xcc);
                // call width: 3
                g.call(t(g, 0x96f, 268), 0, &[obj, 0x11, v]);
                g.write::<u32>(s.wrapping_sub(4), r + 0x54c);
                g.write::<u32>(v + 0xc, ONE);
                crate::m0513::for_target(
                    g,
                    img,
                    &at(0x99f, s.wrapping_sub(8), r + 0x10e0, r + 0x78),
                );
                let e = r + 0xa2c;
                effect(g, e, [0x9b8, w(0x9c0), 0x9cc], n1, 2, 0x9d9, 0x9e3);
                g.write::<u32>(r + 0x18, 0x3c);
            }
            0x3c => {
                g.write::<u32>(r + 0xac0, 0x2710);
                g.write::<u32>(r + 0x50, 0x1e);
                g.write::<u32>(r + 0x18, 0x3d);
                eax = 0x3d;
                break;
            }
            0x3d => {
                eax = 10;
                if g.read::<i32>(r + 0x50) > 0 {
                    break;
                }
                let k = g.read::<f32>(w(0x35));
                let x = sub(k, g.read::<f32>(w(0xa26)));
                let fl = (g.read::<u32>(w(0xa2b)) & !1) | 4;
                g.write::<u32>(w(0xa37), ONE);
                g.write::<f32>(w(0xa43), x);
                let x = sub(k, g.read::<f32>(w(0xa4e)));
                for o in [0xa54u32, 0xa5e, 0xa68, 0xa72] {
                    g.write::<u32>(w(o), ONE);
                }
                g.write::<f32>(w(0xa7e), x);
                let x = sub(k, g.read::<f32>(w(0xa89)));
                g.write::<u32>(w(0xa8f), ONE);
                g.write::<u32>(w(0xa99), ONE);
                g.write::<u32>(w(0xaa3), 0x3ca3_d70a);
                g.write::<u32>(w(0xaad), 0);
                g.write::<f32>(w(0xab9), x);
                let x = sub(k, g.read::<f32>(w(0xac4)));
                g.write::<u32>(w(0xac9), fl);
                g.write::<u32>(w(0xacf), ONE);
                g.write::<f32>(w(0xadb), x);
                for (rd, wr) in [(0xae6u32, 0xaeeu32), (0xaf9, 0xb01)] {
                    let x = sub(k, g.read::<f32>(w(rd)));
                    g.write::<f32>(w(wr), x);
                }
                let x = sub(g.read::<f32>(w(0xb09)), g.read::<f32>(w(0xb11)));
                g.write::<f32>(w(0xb19), x);
                let x = sub(k, g.read::<f32>(w(0xb24)));
                g.write::<f32>(w(0xb2c), x);
                g.write::<u32>(r + 0x18, 0x2710);
            }
            0x2710 => {
                if g.read::<u32>(r + 0xac0) & 0x4e20 == 0 {
                    break;
                }
                let node = g.read::<u32>(task);
                let (prev, next) = (g.read::<u32>(node + 4), g.read::<u32>(node + 8));
                g.write::<u32>(prev + 8, next);
                let (next, prev) = (g.read::<u32>(node + 8), g.read::<u32>(node + 4));
                g.write::<u32>(next + 4, prev);
                g.write::<u32>(node + 8, node);
                g.write::<u32>(node + 4, node);
                eax = prev;
                break;
            }
            s if s <= 0x3d => {
                eax = 11;
                break;
            }
            _ => break,
        }
    }
    let n = g.read::<u32>(r + 0x4c).wrapping_add(1);
    g.write::<u32>(r + 0x4c, n);
    let c = g.read::<u32>(r + 0x50).wrapping_sub(1);
    g.write::<u32>(r + 0x50, c);
    Ret::eax(eax)
}

/// [`summon_task_0513`] (3382 bytes, 1 overlay).
pub const SUMMON_TASK_0513: Sig = Sig {
    len: 3382,
    hash: 0x0b9a_9967_72a8_7770,
    calls: &[
        (0x88, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0xbc, Callee::Sig(&crate::s3_summon::MODEL_NODES)),
        (0x1bb, Callee::Sig(&crate::m0070::MODEL_STATE)),
        (0x447, Callee::Sig(&crate::m0070::MODEL_STATE)),
        (0x66d, Callee::Sig(&crate::s3_misc::COLOUR_NODE)),
        (0x7b0, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
        (0x851, Callee::Sig(&crate::m0513::OVER_NODE)),
        (0x889, Callee::Sig(&crate::m0513::LIT_NODE)),
        (0x962, Callee::Sig(&crate::m0513::BETWEEN)),
        (0x99f, Callee::Sig(&crate::m0513::FOR_TARGET)),
        (0x9d9, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
        (0xbb9, Callee::Sig(&crate::m0003::NODE_INIT)),
        (0xbf7, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
        // __security_check_cookie
        (0xcbe, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0603`'s summon task (0x1000c840, 716 bytes; a task function,
/// cdecl `(task)`, the record `R = task[1]`; an 8-aligned frame without a
/// cookie, `A = ((E - 4) & !7) - 8`, the cursor at `A + 4`). The time `R +
/// 0x18` steps by `K` first; by the step byte `R + 0x1c` (a jump table;
/// steps 0 to 4 run on into each other):
/// - 0: slot 719 `(PART, 5, 0)` into slot 306 `(7, 1, part)`, the time 0;
/// - 2: the node `R + 0x370` appended to `G_L`
///   ([`node_append`](crate::sb_misc::node_append)), the owned node `R +
///   0x39c` under the list ([`node_append_owned`](crate::sa_misc::node_append_owned),
///   `(G_A, G_B, G_C, R + 0x370)`), its colour (1, 1, 1, 0.6);
/// - 3: `*G_V` less `K2`, `R + 0x60` 0.1;
/// - 4: the nodes `R + 0xec` and `R + 0xc0` on `G_L`, the owned node `R +
///   0x118` (`(G_D, G_E, R + 0xec, R + 0xc0)`, colour (1, 1, 1, 0.13)),
///   the child `R + 0x70` under `R + 0xec`
///   ([`child_node`](crate::s4_small::child_node), `(R + 0x118, G_E)`,
///   the owned node's `ecx` as it leaves it), the byte `R + 0x260` 1, `R +
///   0xb0..` 1.0; step 5 (returns the child's `eax`);
/// - 5: the scene `R + 0x600`
///   ([`scene_ctor_p030`](crate::r11_misc::scene_ctor_p030), `edx` `R +
///   0x118`), the effect node `G_N`
///   ([`effect_node_with`](crate::s3_nodes::effect_node_with), `(R +
///   0x118, G_F, _, 2)`), bit 0 of `*G_W`, `*G_W2` 0; step 6;
/// - 6: the models `G_M1` and `G_M2` leave their nodes
///   ([`leave`](crate::sa_misc::leave)) and are set up
///   ([`model_setup`](crate::sa_summon::model_setup)), the task's node
///   unlinked (returns its old `+4`).
///
/// A later step returns the step.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn summon_task_0603<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    let a = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(8);
    // after `push esi; push edi`
    let s = a.wrapping_sub(8);
    let at = |c: u32, esp: u32, ecx: u32, edx: u32| Frame {
        at: call(c),
        esp,
        ecx,
        edx,
        ..*fr
    };
    let tm = add(g.read::<f32>(r + 0x18), g.read::<f32>(w(0x1a)));
    let step = g.read::<u8>(r + 0x1c);
    g.write::<f32>(r + 0x18, tm);
    match step {
        0..=4 => {
            if step == 0 {
                // call width: 3
                let part = g.call(t(g, 0x38, 719), 0, &[w(0x47), 5, 0]).eax;
                // call width: 3
                g.call(t(g, 0x52, 306), 0, &[7, 1, part]);
                g.write::<u32>(r + 0x18, 0);
                g.write::<u8>(r + 0x1c, 1);
            }
            if step <= 2 {
                g.write::<u8>(r + 0x1c, 2);
                let (n, list) = (r + 0x370, w(0x7b));
                crate::sb_misc::node_append(g, img, call(0x80), n, list, 0);
                for (i, x) in (0u32..).zip([ONE, ONE, ONE, 0x3f19_999a]) {
                    g.write::<u32>(r + 0x38c + 4 * i, x);
                }
                // the append leaves `ecx` = the node, `edx` = the list
                let esp = s.wrapping_sub(0x14);
                for (i, x) in (1u32..).zip([w(0x90), w(0x8b), w(0x86), n]) {
                    g.write::<u32>(esp + 4 * i, x);
                }
                crate::sa_misc::node_append_owned(g, img, &at(0xc3, esp, r + 0x39c, list));
            }
            if step <= 3 {
                g.write::<u8>(r + 0x1c, 3);
                let x = sub(g.read::<f32>(w(0xd2)), g.read::<f32>(w(0xda)));
                g.write::<f32>(w(0xe2), x);
                g.write::<u32>(r + 0x60, 0x3dcc_cccd);
            }
            g.write::<u8>(r + 0x1c, 4);
            let list_node = crate::r22::list_node_0645::<G>;
            let (n1, n2) = (r + 0xec, r + 0xc0);
            g.write::<u32>(n1, w(0xf9));
            list_node(
                g,
                img,
                f,
                n1,
                [0x10c, 0x113, 0x118, 0x120, 0x125, 0x12d, 0x139, 0x13f],
            );
            list_node(
                g,
                img,
                f,
                n2,
                [0x158, 0x15f, 0x164, 0x16c, 0x171, 0x179, 0x185, 0x18c],
            );
            g.write::<u32>(n2, w(0x192));
            let owned = r + 0x118;
            for (i, x) in (0u32..).zip([ONE, ONE, ONE, 0x3e05_1eb8]) {
                g.write::<u32>(r + 0xdc + 4 * i, x);
            }
            let esp = s.wrapping_sub(0x14);
            for (i, x) in (1u32..).zip([w(0x19d), w(0x198), n1, n2]) {
                g.write::<u32>(esp + 4 * i, x);
            }
            crate::sa_misc::node_append_owned(g, img, &at(0x1d5, esp, owned, w(0x1a8)));
            // node_append_owned never writes `ecx`: the owned node is the
            // child's first word
            let esp = s.wrapping_sub(0xc);
            g.write::<u32>(esp + 4, owned);
            g.write::<u32>(esp + 8, w(0x1df));
            g.write::<u8>(r + 0x260, 1);
            let eax = crate::s4_small::child_node(g, img, &at(0x1ef, esp, r + 0x70, n1)).eax;
            for o in [0xb0u32, 0xb4, 0xb8] {
                g.write::<u32>(r + o, ONE);
            }
            g.write::<u8>(r + 0x1c, 5);
            Ret::eax(eax)
        }
        5 => {
            let owned = r + 0x118;
            crate::r11_misc::scene_ctor_p030(
                g,
                img,
                &at(0x22d, s.wrapping_sub(4), r + 0x600, owned),
            );
            // push 2; push ecx; push G_F; push the node: the `ecx` word (what
            // the scene leaves) is not read
            let esp = s.wrapping_sub(0x14);
            for (i, x) in (1u32..).zip([owned, w(0x235), 0, 2]) {
                g.write::<u32>(esp + 4 * i, x);
            }
            let eax = crate::s3_nodes::effect_node_with(g, img, &at(0x240, esp, w(0x23b), 0));
            let fl = g.read::<u32>(w(0x246)) | 1;
            g.write::<u32>(w(0x246), fl);
            g.write::<u32>(w(0x250), 0);
            g.write::<u8>(r + 0x1c, 6);
            Ret::eax(eax)
        }
        6 => {
            let cur = a + 4;
            for (m, ms) in [(0x262u32, 0x274u32), (0x279, 0x28b)] {
                let node = w(m);
                crate::sa_misc::leave(g, node, 0);
                // node_leave leaves `ecx` = the node (it never writes it)
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
            Ret::eax(prev)
        }
        s => Ret::eax(u32::from(s)),
    }
}

/// [`summon_task_0603`] (716 bytes, 1 overlay).
pub const SUMMON_TASK_0603: Sig = Sig {
    len: 716,
    hash: 0x2910_57be_a50c_9fa4,
    calls: &[
        (0x80, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0xc3, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x1d5, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x1ef, Callee::Sig(&crate::s4_small::CHILD_NODE)),
        (0x22d, Callee::Sig(&crate::r11_misc::SCENE_CTOR_0603)),
        (0x240, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0x267, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x274, Callee::Sig(&crate::sa_summon::MODEL_SETUP)),
        (0x27e, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x28b, Callee::Sig(&crate::sa_summon::MODEL_SETUP)),
    ],
    imports: &[],
};

/// The run of constant stores to absolute addresses in the code from `f +
/// from` to `f + to`, read as the instructions hold them: `c7 05 [G]
/// imm32`, `66 c7 05 [G] imm16` and `c6 05 [G] imm8`. Stops at any other
/// instruction (the callers pass whole runs).
fn abs_stores<G: Game>(g: &mut G, img: &Image, f: u32, from: u32, to: u32) {
    let byte = |at: u32| img.word(at) & 0xff;
    let mut at = f + from;
    while at < f + to {
        match (byte(at), byte(at + 1), byte(at + 2)) {
            (0xc7, 0x05, _) => {
                g.write::<u32>(img.word(at + 2), img.word(at + 6));
                at += 10;
            }
            (0x66, 0xc7, 0x05) => {
                g.write::<u16>(img.word(at + 3), img.word(at + 7) as u16);
                at += 9;
            }
            (0xc6, 0x05, _) => {
                g.write::<u8>(img.word(at + 2), byte(at + 6) as u8);
                at += 7;
            }
            _ => return,
        }
    }
}

/// A static node appended to the static list as `magic_0603`'s start-up
/// inlines it: the list's last read (operand `last`), the node's own
/// words (the store run `pre`), its `+8` = the old last (operand `n8`);
/// an empty list (the old last is the list, operand `cmp`) gets it as its
/// first (the store at `eq`), else the old last's `+4` is it (the
/// immediate `ne`); the count up (operand `count`); the run `post` (the
/// list's last word, and whatever follows).
fn static_append_0603<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    (last, n8, cmp, eq, ne, count): (u32, u32, u32, u32, u32, u32),
    pre: (u32, u32),
    post: (u32, u32),
) {
    let w = |o: u32| img.word(f + o);
    let old = g.read::<u32>(w(last));
    abs_stores(g, img, f, pre.0, pre.1);
    g.write::<u32>(w(n8), old);
    if old == w(cmp) {
        abs_stores(g, img, f, eq, eq + 10);
    } else {
        g.write::<u32>(old.wrapping_add(4), w(ne));
    }
    let n = g.read::<u16>(w(count)).wrapping_add(1);
    g.write::<u16>(w(count), n);
    abs_stores(g, img, f, post.0, post.1);
}

/// `magic_0603`'s start-up (0x1000ccf0, 2986 bytes; a table entry, cdecl
/// `(actor)`; an 8-aligned cookie frame, `A = ((E - 4) & !7) - 0x44` and
/// `S = A - 4` after `push esi`: the vector `V` at `A + 4`, the bone
/// cursor `A + 4`, the node record `A + 0xc`, the aim `D` at `A + 0x20`,
/// slot 232's point `P` at `A + 0x30`). The targets
/// ([`targets_init_c`](crate::s4_summon::targets_init_c)); two buffers
/// (slot 803 `(0x80000, 0x10)`, `(0x48000, 0x10)`); the statics' words
/// (the DLL's ring heads and four static nodes); slot 217 `(*G_C, G_P)`;
/// the static nodes appended to `G_L` with their words, slot 325, the
/// mean offset ([`targets_mean_offset`](crate::s5_misc::targets_mean_offset)
/// into `G_D`), slot 215 `(*G_C)` the object `*G_O`; slot 601's value
/// (`K - v`, times `K2` over `K3`, truncated) into `*G_N`; slot 262 into
/// `G_Q`; the object's slots 15, 16, 17 and 277 (ST0 each) into `G_F..`;
/// the bone node `G_B` ([`bone_node`](crate::s8_misc::bone_node), the
/// pass byte `*G_PASS`); the node record `(G_E, L, G_1, G_2, G_3)`
/// ([`node_init_big`](crate::m0070::node_init_big)); slot 234 `(G_R,
/// G_D, G_P)`, slot 232 `(P, G_R)`; by the model id slot 390 `(*G_ID)`
/// gives (4203, 4209, 4232, 4233, 4236 or another), the aim flag `*G_AIM`
/// and the aim `D` from `P` (its `z` and `-x` scaled, `y` = `*G_Y`; slot
/// 248 `(G_A, G_D, D)` with the first scales, then the second; 4209 asks
/// slot 535 and may move `*G_H` first); slot 234 `(D, G_D, D)` and
/// `(G_S, D, G_A)`; `*G_S2` 0; the part loads (slots 718, 719, 711 under
/// the lock, 710, 222, 221, 719, 551); the model start
/// ([`model_start_0603`](crate::r18::model_start_0603)), the summon set-up
/// ([`summon_setup_0603`](crate::r23::summon_setup_0603)) and slot 10
/// `(*G_O, 0)`. Returns 0.
#[allow(clippy::too_many_lines)] // one straight sequence
pub fn start_up_0603<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let rd = |g: &G, o: u32| g.read::<u32>(w(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let actor = fr.arg(g, 0);
    let a = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x44);
    // after `push esi`
    let s = a.wrapping_sub(4);
    let at = |c: u32, ecx: u32| Frame {
        at: call(c),
        esp: s.wrapping_sub(4),
        ecx,
        ..*fr
    };
    crate::s4_summon::targets_init_c(g, img, call(0x19), s.wrapping_sub(4), actor);
    // call width: 2
    let b = g.call(t(g, 0x1e, 803), 0, &[0x80000, 0x10]).eax;
    g.write::<u32>(w(0x32), b);
    // call width: 2
    let b = g.call(t(g, 0x37, 803), 0, &[0x48000, 0x10]).eax;
    g.write::<u32>(w(0x4b), b);
    abs_stores(g, img, f, 0x5f, 0x187);
    // call width: 2
    g.call(t(g, 0x50, 217), 0, &[rd(g, 0x5b), w(0x55)]);
    abs_stores(g, img, f, 0x197, 0x1a1);
    static_append_0603(
        g,
        img,
        f,
        (0x190, 0x1d3, 0x1d8, 0x1de, 0x1ed, 0x1f9),
        (0x1a1, 0x1d2),
        (0x1fd, 0x211),
    );
    // call width: 2
    g.call(t(g, 0x1f2, 325), 0, &[w(0x21d), w(0x218)]);
    let x = rd(g, 0x22c);
    g.write::<u32>(w(0x237), x);
    static_append_0603(
        g,
        img,
        f,
        (0x224, 0x26d, 0x272, 0x278, 0x287, 0x28e),
        (0x23b, 0x26c),
        (0x297, 0x2ab),
    );
    crate::s5_misc::targets_mean_offset(g, img, &at(0x2ac, w(0x293)));
    let obj = g.call(t(g, 0x2b1, 215), 0, &[rd(g, 0x2b7)]).eax;
    g.write::<u32>(w(0x2c4), obj);
    static_append_0603(
        g,
        img,
        f,
        (0x2c9, 0x30c, 0x311, 0x317, 0x326, 0x32d),
        (0x2d0, 0x30b),
        (0x341, 0x355),
    );
    let v = a + 4;
    // call width: 2
    g.call(t(g, 0x337, 601), 0, &[rd(g, 0x33d), v]);
    let x = sub(k(g, 0x361), g.read::<f32>(v));
    abs_stores(g, img, f, 0x36e, 0x38b);
    let x = mul(x, k(g, 0x38f));
    abs_stores(g, img, f, 0x393, 0x3a7);
    let x = div(x, k(g, 0x3ab));
    abs_stores(g, img, f, 0x3af, 0x3b9);
    g.write::<u32>(w(0x3be), crate::s4_summon::trunc(x));
    let q = [w(0x3fd), w(0x3f8), w(0x3f3)];
    static_append_0603(
        g,
        img,
        f,
        (0x3c3, 0x3c8, 0x3cd, 0x3d3, 0x3e2, 0x3ee),
        (0x3c2, 0x3c2),
        (0x407, 0x41b),
    );
    // call width: 4
    g.call(t(g, 0x3e7, 262), 0, &[rd(g, 0x403), q[0], q[1], q[2]]);
    static_append_0603(
        g,
        img,
        f,
        (0x424, 0x467, 0x46c, 0x472, 0x481, 0x48d),
        (0x42b, 0x466),
        (0x497, 0x4ab),
    );
    for (op, slot, arg, out) in [
        (0x486u32, 15u32, 0x493u32, 0x4b2u32),
        (0x4b7, 16, 0x4c3, 0x4cb),
        (0x4d0, 17, 0x4dc, 0x4e4),
        (0x4e9, 277, 0x4f8, 0x500),
    ] {
        let o = g.call(t(g, op, slot), 0, &[rd(g, arg)]);
        g.write::<f32>(w(out), crate::ob_model::st0_f32(o.st0));
    }
    let old = rd(g, 0x505);
    abs_stores(g, img, f, 0x50c, 0x53d);
    g.write::<u32>(w(0x53e), old);
    if old == w(0x543) {
        abs_stores(g, img, f, 0x549, 0x553);
    } else {
        g.write::<u32>(old.wrapping_add(4), w(0x558));
    }
    let pass = g.read::<u8>(w(0x55d));
    let n = g.read::<u16>(w(0x564)).wrapping_add(1);
    g.write::<u16>(w(0x564), n);
    abs_stores(g, img, f, 0x56c, 0x5b7);
    let cur = a + 4;
    g.write::<u32>(cur, w(0x5b3));
    g.write::<u8>(cur + 5, pass);
    crate::s8_misc::bone_node(g, img, &at(0x5bc, cur));
    let rec = a + 0xc;
    for (i, o) in (0u32..).zip([0x5c8u32, 0x5d0, 0x5d8, 0x5e0, 0x5e8]) {
        g.write::<u32>(rec + 4 * i, w(o));
    }
    crate::m0070::node_init_big(g, rec);
    let x = rd(g, 0x5f2);
    g.write::<u32>(w(0x5f7), x);
    static_append_0603(
        g,
        img,
        f,
        (0x5fc, 0x646, 0x64b, 0x651, 0x660, 0x66c),
        (0x600, 0x645),
        (0x675, 0x6bb),
    );
    // call width: 3
    g.call(t(g, 0x665, 234), 0, &[w(0x6c7), w(0x6c2), w(0x671)]);
    static_append_0603(
        g,
        img,
        f,
        (0x6ce, 0x707, 0x70c, 0x712, 0x721, 0x728),
        (0x6d5, 0x706),
        (0x73b, 0x74f),
    );
    let p = a + 0x30;
    // call width: 2
    g.call(t(g, 0x737, 232), 0, &[p, w(0x731)]);
    let idb = u32::from(g.read::<u8>(w(0x75a)));
    let (px, pz) = (g.read::<f32>(p), g.read::<f32>(p + 8));
    g.write::<f32>(a + 0x3c, px);
    g.write::<f32>(p, pz);
    let mx = f32::from_bits(px.to_bits() ^ g.read::<u32>(w(0x773)));
    let y = rd(g, 0x781);
    g.write::<f32>(a + 0x38, mx);
    g.write::<u32>(a + 0x24, y);
    g.write::<u32>(a + 0x2c, 0);
    let id = g.call(t(g, 0x787, 390), 0, &[idb]).eax;
    g.write::<u32>(w(0x7a8), id);
    let d = a + 0x20;
    // the aim: `D.x = P.z * k1`, `D.z = -P.x * k1` for slot 248, then the
    // second scales
    // `ops`: the first scales, the second, slot 248's operand and its two
    // pushed immediates
    let aim = |g: &mut G, ops: [u32; 7], flag: Option<u32>| {
        let x = mul(g.read::<f32>(a + 0x30), g.read::<f32>(w(ops[0])));
        g.write::<f32>(d, x);
        let z = mul(g.read::<f32>(a + 0x38), g.read::<f32>(w(ops[1])));
        g.write::<f32>(a + 0x28, z);
        if let Some(o) = flag {
            g.write::<u32>(w(o), 1);
        }
        // call width: 3
        g.call(t(g, ops[4], 248), 0, &[w(ops[6]), w(ops[5]), d]);
        let x = mul(g.read::<f32>(a + 0x30), g.read::<f32>(w(ops[2])));
        g.write::<f32>(d, x);
        mul(g.read::<f32>(a + 0x38), g.read::<f32>(w(ops[3])))
    };
    let z = match id.wrapping_sub(0x106b) {
        0 => {
            g.write::<u32>(w(0x992), 2);
            aim(g, [0x9b0, 0x9ce, 0x9f4, 0xa08, 0x9ba, 0x9d3, 0x9d8], None)
        }
        6 => {
            let b = u32::from(g.read::<u8>(w(0x8c7)));
            g.write::<u32>(w(0x8d3), 1);
            match g.call(t(g, 0x8cd, 535), 0, &[b]).eax {
                0 => {
                    let h = sub(k(g, 0x97f), k(g, 0x987));
                    g.write::<f32>(w(0x90c), h);
                    aim(g, [0x91a, 0x938, 0x95e, 0x972, 0x924, 0x93d, 0x942], None)
                }
                2 => {
                    let h = sub(k(g, 0x8fc), k(g, 0x904));
                    g.write::<f32>(w(0x90c), h);
                    aim(g, [0x91a, 0x938, 0x95e, 0x972, 0x924, 0x93d, 0x942], None)
                }
                _ => aim(g, [0x9b0, 0x9ce, 0x9f4, 0xa08, 0x9ba, 0x9d3, 0x9d8], None),
            }
        }
        0x1d => aim(
            g,
            [0x7d5, 0x7f3, 0x823, 0x837, 0x7df, 0x7f8, 0x7fd],
            Some(0x80f),
        ),
        0x1e => aim(
            g,
            [0x84a, 0x868, 0x898, 0x8ac, 0x854, 0x86d, 0x872],
            Some(0x884),
        ),
        0x21 => {
            g.write::<u32>(w(0x8b7), 1);
            aim(g, [0x9b0, 0x9ce, 0x9f4, 0xa08, 0x9ba, 0x9d3, 0x9d8], None)
        }
        _ => {
            g.write::<u32>(w(0x99e), 0);
            aim(g, [0x9b0, 0x9ce, 0x9f4, 0xa08, 0x9ba, 0x9d3, 0x9d8], None)
        }
    };
    g.write::<f32>(a + 0x28, z);
    // call width: 3
    g.call(t(g, 0xa1b, 234), 0, &[d, w(0xa15), d]);
    // call width: 3
    g.call(t(g, 0xa3b, 234), 0, &[w(0xa40), d, w(0xa35)]);
    g.write::<u32>(w(0xa56), 0);
    g.call(t(g, 0xa4d, 718), 0, &[w(0xa65)]);
    // call width: 3
    let part = g.call(t(g, 0xa6c, 719), 0, &[w(0xa7e), 8, 0]).eax;
    let lock = rd(g, 0xa86);
    let old = g.read::<u32>(lock);
    g.write::<u32>(w(0xa92), part);
    g.write::<u32>(lock, 1);
    // call width: 3
    g.call(t(g, 0xa9d, 711), 0, &[part, w(0xa97), 0]);
    g.write::<u32>(rd(g, 0xab0), old);
    // call width: 4
    g.call(
        t(g, 0xaba, 710),
        0,
        &[w(0xad5), rd(g, 0xad0), w(0xaca), 0x80000],
    );
    // call width: 4
    g.call(t(g, 0xadc, 222), 0, &[w(0xaf0), 0, 0x3e8, 2]);
    // call width: 4
    g.call(t(g, 0xaf7, 221), 0, &[w(0xb0b), 0, 0x3e8, 2]);
    // call width: 3
    let part = g.call(t(g, 0xb12, 719), 0, &[w(0xb21), 6, 4]).eax;
    // call width: 3
    g.call(t(g, 0xb30, 551), 0, &[part, 0, 2]);
    crate::r18::model_start_0603(g, img, &at(0xb40, 0));
    crate::r23::summon_setup_0603(g, img, call(0xb45), s.wrapping_sub(4));
    // call width: 2
    g.call(t(g, 0xb4a, 10), 0, &[rd(g, 0xb52), 0]);
    Ret::eax(0)
}

/// [`start_up_0603`] (2986 bytes, 1 overlay).
pub const START_UP_0603: Sig = Sig {
    len: 2986,
    hash: 0x35e9_e6d2_c86c_70f4,
    calls: &[
        (0x19, Callee::Sig(&crate::s4_summon::TARGETS_INIT_C)),
        (0x2ac, Callee::Sig(&crate::s5_misc::TARGETS_MEAN_OFFSET)),
        (0x5bc, Callee::Sig(&crate::s8_misc::BONE_NODE)),
        (0x5ed, Callee::Sig(&crate::m0070::NODE_INIT_BIG)),
        (0xb40, Callee::Sig(&crate::r18::MODEL_START_0603)),
        (0xb45, Callee::Sig(&crate::r23::SUMMON_SETUP_0603)),
        // __security_check_cookie
        (0xb68, Callee::Any),
    ],
    imports: &[],
};

/// The operand offsets of one of [`camera_task_0603`]'s two closing
/// ramps: `xmm2`/`xmm3` (the time, the decay), the four time bounds, the
/// move (the flag word, the store run of seven 1.0s, the record's node
/// immediate, the call, the flag's clear), the byte of the summed shares
/// (the two shares, the scale, the byte), the random wobble (slot 307,
/// its offset, the scale word, slot 250), the decays of the scale word
/// and the first share, the fade (its word and the colour), the second
/// share's decay, the leave.
struct Ramp0603 {
    t: u32,
    k: u32,
    bounds: [u32; 4],
    flag: u32,
    run: (u32, u32),
    node: u32,
    clear: u32,
    shares: [u32; 4],
    wobble: [u32; 4],
    decay: [u32; 6],
    fade: [u32; 4],
    second: [u32; 3],
    leave: u32,
}

/// One closing ramp of [`camera_task_0603`] (`a` the frame base, `ret`
/// the `eax` it returns when the time is below the first bound). Every
/// bound the time `T` reaches adds a step: from the first, the move
/// started once (`*G_FLAG` cleared after,
/// [`move_start`](crate::sa_misc::move_start) of `[N, 0, 0.02, 1]`), the
/// byte `(s1 + s2) K`, slot 250 `(0, (rand - K) * W)` and the decays; the
/// second, the fade's step; the third, the second share's decay; the
/// fourth, the node `L` leaves and the task's node is unlinked. Returns
/// what `eax` holds at the end.
fn ramp_0603<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    a: u32,
    task: u32,
    r: &Ramp0603,
    ret: u32,
) -> u32 {
    use crate::sb_sse::{add, mul, sub};
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let mut tm = k(g, r.t);
    let mut decay = k(g, r.k);
    let mut eax = ret;
    // comiss T, K; jb: below or unordered skips
    let reached = |g: &G, tm: f32, o: u32| !(tm.is_nan() || tm < k(g, o));
    if reached(g, tm, r.bounds[0]) {
        if g.read::<u32>(w(r.flag)) != 0 {
            abs_stores(g, img, f, r.run.0, r.run.1);
            let rec = a + 0x18;
            for (i, x) in (0u32..).zip([w(r.node), 0, 0x3ca3_d70a, ONE]) {
                g.write::<u32>(rec + 4 * i, x);
            }
            crate::sa_misc::move_start(g, rec);
            abs_stores(g, img, f, r.clear, r.clear + 10);
        }
        let x = mul(add(k(g, r.shares[0]), k(g, r.shares[1])), k(g, r.shares[2]));
        g.write::<u8>(w(r.shares[3]), crate::s4_summon::trunc(x) as u8);
        let rnd = crate::ob_model::st0_f32(g.call(t(g, r.wobble[0], 307), 0, &[]).st0);
        g.write::<f32>(a, rnd);
        let x = mul(sub(rnd, k(g, r.wobble[1])), k(g, r.wobble[2]));
        // call width: 2
        eax = g.call(t(g, r.wobble[3], 250), 0, &[0, x.to_bits()]).eax;
        decay = k(g, r.decay[1]);
        tm = k(g, r.decay[2]);
        let x = mul(k(g, r.decay[0]), decay);
        g.write::<f32>(w(r.decay[3]), x);
        let x = mul(k(g, r.decay[4]), decay);
        g.write::<f32>(w(r.decay[5]), x);
    }
    if reached(g, tm, r.bounds[1]) {
        let c = k(g, r.fade[0]);
        let x = sub(k(g, r.fade[1]), c);
        g.write::<f32>(w(r.fade[2]), x);
        g.write::<f32>(w(r.fade[3]), mul(c, decay));
    }
    if reached(g, tm, r.bounds[2]) {
        let x = mul(k(g, r.second[0]), k(g, r.second[1]));
        g.write::<f32>(w(r.second[2]), x);
    }
    if !reached(g, tm, r.bounds[3]) {
        return eax;
    }
    crate::sa_misc::leave(g, w(r.leave), eax);
    let node = g.read::<u32>(task);
    let (prev, next) = (g.read::<u32>(node + 4), g.read::<u32>(node + 8));
    g.write::<u32>(prev + 8, next);
    let (next, prev) = (g.read::<u32>(node + 8), g.read::<u32>(node + 4));
    g.write::<u32>(next + 4, prev);
    g.write::<u32>(node + 4, node);
    g.write::<u32>(node + 8, node);
    prev
}

/// `magic_0603`'s camera task (0x1000b9e0, 3026 bytes; a task function,
/// cdecl `(task)`; an 8-aligned cookie frame, `A = ((E - 4) & !7) -
/// 0x2c`, `S = A - 4` after `push esi`: the scales at `A` and
/// `A + 0xc`..`A + 0x14`, the point `P` at `A + 0x18`). The time `*G_T` steps by `K`;
/// by the state `*G_S` (a jump table):
/// - 0: the camera tree `(G_A, G_B)`
///   ([`camera_tree`](crate::s5_misc::camera_tree), the vtable after),
///   slots 286 `(*G_CAM, 600)` and 211 `(*G_O, G_M)`; then as 4;
/// - 4: `*G_T` 4.5, `*G_Z` 0, state 5, and on;
/// - 5: with `*G_AIM` 1 state 100 at once; else the points `P` and `G_C`
///   (slot 232), the node `G_N` appended to `G_L`
///   ([`node_append`](crate::sb_misc::node_append)), slots 325, 286
///   `(*G_CAM, 600)`, 325; `G_E` moved back by `P` (`K1`), `G_E.y` =
///   `*G_Y - K2`; by slot 520's answer a second move (`K3`, `K5`); with
///   `*G_AIM` 2 its `y` lower; slot 233 `(G_C, G_C, 20)`; state 6 and its
///   words (returns slot 233's `eax`);
/// - 6: `G_V` steps by `G_C`, slot 233 `(G_C, G_C, 0.75)`, slot 285
///   `(*G_CAM, K - *G_R)`, `*G_R` decays, then the first closing ramp
///   ([`ramp_0603`]);
/// - 100: `P` (slot 232), the node appended, slots 325, 286 `(*G_CAM,
///   800)`; the scales by the model id `*G_ID` (0x1088, 0x1089, 0x108c,
///   else by slot 535's answer 0, 2 or another); slot 325; `G_E` moved by
///   `P` times the scales; `P` turned (`(z, 0, -x, 0)`) and scaled (slot
///   233), slots 248, 232; `G_V` steps by `G_C` times the scales, slots
///   233, 285; state 101 and its words, and on;
/// - 101: `G_V` steps back by `G_C`, slot 233 `(G_C, G_C, 0.95)`, then
///   the second ramp.
///
/// Other states return what `eax` holds.
#[allow(clippy::too_many_lines)] // the original's states, in order
pub fn camera_task_0603<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let rd = |g: &G, o: u32| g.read::<u32>(w(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let task = fr.arg(g, 0);
    let a = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x2c);
    let s = a.wrapping_sub(4);
    let tm = add(k(g, 0x18), k(g, 0x20));
    let mut state = rd(g, 0x26);
    g.write::<f32>(w(0x34), tm);
    let p = a + 0x18;
    loop {
        match state {
            0 | 4 | 5 => {
                if state == 0 {
                    let cur = a;
                    g.write::<u32>(cur, w(0x57));
                    g.write::<u32>(cur + 4, w(0x5f));
                    let cf = Frame {
                        at: call(0x64),
                        esp: s.wrapping_sub(4),
                        ecx: cur,
                        ..*fr
                    };
                    crate::s5_misc::camera_tree(g, img, &cf);
                    let n = g.read::<u32>(cur);
                    g.write::<u32>(n, w(0x6f));
                    // call width: 2
                    g.call(t(g, 0x74, 286), 0, &[rd(g, 0x81), 0x4416_0000]);
                    // call width: 2
                    g.call(t(g, 0x8e, 211), 0, &[rd(g, 0x99), w(0x93)]);
                }
                if state != 5 {
                    g.write::<u32>(w(0xb0), 0x4090_0000);
                    g.write::<u32>(w(0xba), 0);
                    g.write::<u32>(w(0xc4), 5);
                }
                if rd(g, 0xc9) == 1 {
                    g.write::<u32>(w(0xd7), 0x64);
                    state = 0x64;
                    continue;
                }
                // call width: 2
                g.call(t(g, 0xed, 232), 0, &[p, w(0xe7)]);
                // call width: 2
                g.call(t(g, 0xfa, 232), 0, &[w(0x10a), w(0xff)]);
                crate::sb_misc::node_append(g, img, call(0x11b), w(0x116), w(0x111), 0);
                // call width: 2
                g.call(t(g, 0x120, 325), 0, &[w(0x130), w(0x125)]);
                let y = rd(g, 0x13a);
                g.write::<u32>(w(0x14a), y);
                // call width: 2
                g.call(t(g, 0x13f, 286), 0, &[rd(g, 0x15d), 0x4416_0000]);
                let b = g.read::<u8>(w(0x16a)) | 1;
                g.write::<u8>(w(0x16a), b);
                g.write::<u32>(w(0x171), w(0x175));
                g.write::<u32>(w(0x17b), ONE);
                g.write::<u32>(w(0x185), w(0x189));
                // call width: 2
                g.call(t(g, 0x164, 325), 0, &[w(0x199), w(0x194)]);
                let dx = mul(g.read::<f32>(p), k(g, 0x1a9));
                let x = sub(k(g, 0x1b1), dx);
                let dz = mul(g.read::<f32>(p + 8), k(g, 0x1c8));
                g.write::<f32>(w(0x1d3), x);
                g.write::<f32>(w(0x1e7), sub(k(g, 0x1db), dz));
                g.write::<f32>(w(0x1ff), sub(k(g, 0x1ef), k(g, 0x1f7)));
                let kind = g.call(t(g, 0x1b6, 520), 0, &[]).eax;
                let k3 = k(g, 0x20f);
                let y = match kind {
                    0x0247_0000 => {
                        let dx = mul(g.read::<f32>(p), k(g, 0x283));
                        g.write::<f32>(w(0x2a5), sub(k(g, 0x28b), dx));
                        let dz = mul(g.read::<f32>(p + 8), k(g, 0x29d));
                        g.write::<f32>(w(0x2b9), sub(k(g, 0x2ad), dz));
                        let y = sub(k(g, 0x2c1), k(g, 0x2c9));
                        g.write::<f32>(w(0x2d1), y);
                        y
                    }
                    0x024a_0000 => {
                        let dx = mul(g.read::<f32>(p), k3);
                        let x = sub(k(g, 0x22b), dx);
                        let dz = mul(g.read::<f32>(p + 8), k3);
                        g.write::<f32>(w(0x245), x);
                        g.write::<f32>(w(0x259), sub(k(g, 0x24d), dz));
                        let y = sub(k(g, 0x261), k(g, 0x269));
                        g.write::<f32>(w(0x2d1), y);
                        y
                    }
                    _ => k(g, 0x273),
                };
                if rd(g, 0x2d7) == 2 {
                    g.write::<f32>(w(0x2ea), sub(y, k(g, 0x2e2)));
                }
                // call width: 3
                let eax = g
                    .call(t(g, 0x2ef, 233), 0, &[w(0x307), w(0x302), 0x41a0_0000])
                    .eax;
                let n = rd(g, 0x312).wrapping_add(1);
                g.write::<u32>(w(0x312), n);
                abs_stores(g, img, f, 0x316, 0x348);
                return Ret::eax(eax);
            }
            6 => {
                let x = add(k(g, 0x35c), k(g, 0x364));
                g.write::<u8>(w(0x370), 0);
                g.write::<f32>(w(0x380), x);
                let x = add(k(g, 0x388), k(g, 0x390));
                g.write::<u32>(w(0x396), ONE);
                g.write::<f32>(w(0x3ac), x);
                // call width: 3
                g.call(t(g, 0x369, 233), 0, &[w(0x3a4), w(0x39f), 0x3f40_0000]);
                let x = sub(k(g, 0x3bc), k(g, 0x3c4));
                // call width: 2
                let eax = g
                    .call(t(g, 0x3c9, 285), 0, &[rd(g, 0x3dd), x.to_bits()])
                    .eax;
                let x = mul(k(g, 0x3e7), k(g, 0x3ef));
                g.write::<f32>(w(0x40d), x);
                let ramp = Ramp0603 {
                    t: 0x3f7,
                    k: 0x3ef,
                    bounds: [0x405, 0x53c, 0x56d, 0x58e],
                    flag: 0x419,
                    run: (0x424, 0x46a),
                    node: 0x46e,
                    clear: 0x48f,
                    shares: [0x49d, 0x4a5, 0x4ad, 0x4b6],
                    wobble: [0x4bb, 0x4d5, 0x4eb, 0x4da],
                    decay: [0x502, 0x50a, 0x512, 0x521, 0x529, 0x535],
                    fade: [0x546, 0x54e, 0x55e, 0x566],
                    second: [0x577, 0x57f, 0x587],
                    leave: 0xb1d,
                };
                return Ret::eax(ramp_0603(g, img, f, a, task, &ramp, eax));
            }
            0x64 | 0x65 => {
                if state == 0x64 {
                    // call width: 2
                    g.call(t(g, 0x5a2, 232), 0, &[p, w(0x59c)]);
                    crate::sb_misc::node_append(g, img, call(0x5b9), w(0x5b4), w(0x5af), 0);
                    // call width: 2
                    g.call(t(g, 0x5be, 325), 0, &[w(0x5ce), w(0x5c3)]);
                    // call width: 2
                    g.call(t(g, 0x5d5, 286), 0, &[rd(g, 0x5eb), 0x4448_0000]);
                    let id = rd(g, 0x5f2);
                    let b = g.read::<u8>(w(0x5f8)) | 1;
                    g.write::<u8>(w(0x5f8), b);
                    g.write::<u32>(w(0x602), w(0x606));
                    g.write::<u32>(w(0x60c), ONE);
                    g.write::<u32>(w(0x616), w(0x61a));
                    let set = |g: &mut G, at: u32, o: u32| g.write::<f32>(at, k(g, o));
                    let last = match id.wrapping_sub(0x1088) {
                        0 | 4 => {
                            if id == 0x1088 {
                                set(g, a, 0x729);
                                set(g, a + 0x14, 0x737);
                            } else {
                                set(g, a, 0x6cf);
                                set(g, a + 0x14, 0x6dd);
                            }
                            set(g, a + 0x10, 0x745);
                            set(g, a + 0xc, 0x753);
                            k(g, 0x761)
                        }
                        1 => {
                            set(g, a, 0x6e7);
                            set(g, a + 0x14, 0x6f5);
                            set(g, a + 0x10, 0x703);
                            set(g, a + 0xc, 0x711);
                            k(g, 0x71f)
                        }
                        _ => {
                            let b = u32::from(g.read::<u8>(w(0x63c)));
                            let ans = g.call(t(g, 0x642, 535), 0, &[b]).eax;
                            set(g, a, 0x652);
                            set(g, a + 0x14, 0x660);
                            match ans {
                                0 | 2 => {
                                    let o = if ans == 0 { 0x6c5 } else { 0x69c };
                                    set(g, a + 0x10, o);
                                    set(g, a + 0xc, 0x6aa);
                                    k(g, 0x6b8)
                                }
                                _ => {
                                    set(g, a + 0x10, 0x67b);
                                    set(g, a + 0xc, 0x67b);
                                    k(g, 0x68f)
                                }
                            }
                        }
                    };
                    g.write::<f32>(a + 8, last);
                    // call width: 2
                    g.call(t(g, 0x766, 325), 0, &[w(0x776), w(0x76b)]);
                    let sc = g.read::<f32>(a + 0x14);
                    let x = add(mul(g.read::<f32>(p), sc), k(g, 0x79b));
                    g.write::<f32>(w(0x7a9), x);
                    let z = add(mul(g.read::<f32>(p + 8), sc), k(g, 0x7bd));
                    g.write::<f32>(w(0x7c5), z);
                    let y = sub(k(g, 0x7cd), g.read::<f32>(a + 0x10));
                    g.write::<f32>(w(0x7db), y);
                    // call width: 2
                    g.call(t(g, 0x7a1, 232), 0, &[p, w(0x793)]);
                    let (px, pz) = (g.read::<u32>(p), g.read::<u32>(p + 8));
                    g.write::<u32>(p, pz);
                    let f8 = g.read::<u32>(a + 8);
                    g.write::<u32>(p + 8, px ^ g.read::<u32>(w(0x7f6)));
                    g.write::<u32>(p + 4, 0);
                    g.write::<u32>(p + 0xc, 0);
                    // call width: 3
                    g.call(t(g, 0x815, 233), 0, &[p, p, f8]);
                    // call width: 3
                    g.call(t(g, 0x840, 248), 0, &[w(0x850), w(0x845), p]);
                    // call width: 2
                    g.call(t(g, 0x857, 232), 0, &[w(0x86a), w(0x865)]);
                    let s2 = g.read::<f32>(a + 0xc);
                    let x = add(mul(k(g, 0x874), s2), k(g, 0x88a));
                    g.write::<f32>(w(0x892), x);
                    let z = add(mul(k(g, 0x89a), s2), k(g, 0x8a8));
                    g.write::<f32>(w(0x8b0), z);
                    let f0 = g.read::<u32>(a);
                    // call width: 3
                    g.call(t(g, 0x87f, 233), 0, &[w(0x8cb), w(0x8c6), f0]);
                    // call width: 2
                    g.call(t(g, 0x8d2, 285), 0, &[rd(g, 0x8e8), 0x3e71_463b]);
                    let n = rd(g, 0x8f3).wrapping_add(1);
                    g.write::<u32>(w(0x8f3), n);
                    abs_stores(g, img, f, 0x8f7, 0x91f);
                }
                let x = sub(k(g, 0x923), k(g, 0x92b));
                g.write::<u8>(w(0x937), 0);
                g.write::<f32>(w(0x947), x);
                let x = sub(k(g, 0x94f), k(g, 0x957));
                g.write::<u32>(w(0x95d), ONE);
                g.write::<f32>(w(0x973), x);
                // call width: 3
                let eax = g
                    .call(t(g, 0x930, 233), 0, &[w(0x96b), w(0x966), 0x3f73_3333])
                    .eax;
                let ramp = Ramp0603 {
                    t: 0x983,
                    k: 0x98b,
                    bounds: [0x995, 0xac4, 0xaf5, 0xb16],
                    flag: 0x9a1,
                    run: (0x9ac, 0x9f2),
                    node: 0x9f6,
                    clear: 0xa17,
                    shares: [0xa25, 0xa2d, 0xa35, 0xa3e],
                    wobble: [0xa43, 0xa5d, 0xa73, 0xa62],
                    decay: [0xa8a, 0xa92, 0xa9a, 0xaa9, 0xab1, 0xabd],
                    fade: [0xace, 0xad6, 0xae6, 0xaee],
                    second: [0xaff, 0xb07, 0xb0f],
                    leave: 0xb1d,
                };
                return Ret::eax(ramp_0603(g, img, f, a, task, &ramp, eax));
            }
            // the jump table's other entries leave its index in `eax`
            s if s <= 0x65 => return Ret::eax(6),
            _ => return Ret::eax(state),
        }
    }
}

/// [`camera_task_0603`] (3026 bytes, 1 overlay).
pub const CAMERA_TASK_0603: Sig = Sig {
    len: 3026,
    hash: 0xe288_51e6_2dd2_c595,
    calls: &[
        (0x64, Callee::Sig(&crate::s5_misc::CAMERA_TREE)),
        (0x11b, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        // __security_check_cookie
        (0x350, Callee::Any),
        (0x48b, Callee::Sig(&crate::sa_misc::MOVE_START)),
        (0x5b9, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0xa13, Callee::Sig(&crate::sa_misc::MOVE_START)),
        (0xb22, Callee::Sig(&crate::sa_misc::LEAVE)),
        // __security_check_cookie
        (0xb48, Callee::Any),
    ],
    imports: &[],
};
