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
