//! Round 12 (own rows from `magic_0450` up): the short volley task of
//! `magic_0495` and `magic_0500` (the same code; family `P038`), a
//! sibling of [`crate::s7_misc::staged_volley_task`] without the colour
//! node and the camera moves.

use crate::runtime::slot_target;
use crate::sb_sse::{add, cvt, div, mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The task's node leaves its ring (`node.prev.next = node.next`,
/// `node.next.prev = node.prev`, then it links to itself). Returns the
/// old `prev` word (`+4`), which the original leaves in `eax`.
pub(crate) fn unring<G: Game>(g: &mut G, node: u32) -> u32 {
    let prev = g.read::<u32>(node + 4);
    let next = g.read::<u32>(node + 8);
    g.write::<u32>(prev + 8, next);
    let next = g.read::<u32>(node + 8);
    let prev = g.read::<u32>(node + 4);
    g.write::<u32>(next + 4, prev);
    g.write::<u32>(node + 8, node);
    g.write::<u32>(node + 4, node);
    prev
}

/// A summon's short volley task (cdecl `(task)`, `S = task[1]`, its step
/// `S+0x18`; `ebp` = `esp - 4` at the entry, the task's argument word
/// `A = ebp+8` reused as scratch). The steps:
/// - 0: the target nodes of `[G_N, G_P, *G_C, 0, 1]` (`*G_N = VT`;
///   [`target_nodes`](crate::s4_summon::target_nodes)), then as 10;
/// - 10: the weights: for each of the `*G_M` targets slot 291 `(*G_X,
///   rec_i[0], &A, &A, &A)` (`rec_i = G_R + 0x98 i`) into `S+0x44 + 4i`,
///   summed in `S+0x40`, and `S+0x1c + 4i` the share; six slot flags
///   `0xff` (displacements from the code), step 11 and the wait `S+0x3c =
///   0`, then as 11;
/// - 11: the wait counts down; then the slot `j = S+0x40 % 5`, if free:
///   a random target by its share (slot 307's ST0 less `K1`, less each
///   share in turn; the index into `A`), a random time `(r - K1) K2 +
///   K3`, the throw ([`summon_object_setup`](crate::s7_misc::summon_object_setup),
///   in `magic_0500` [`model_object_setup`](crate::s7_misc::model_object_setup):
///   `ecx` = `S+0x70 + 0x310 j`, `edx` = `G_R2 + 0x98 i`); with throws
///   left one fewer for the target, the wait `W` and the shares again,
///   else slot 228 `(*G_X, 0x6a)` and step 0x2710;
/// - 0x2710: once slot 411 `(*G_X)` answers, slot 213 `(*G_X, 0)` and the
///   task's node leaves its ring.
///
/// Returns what the original leaves in `eax`.
///
/// `magic_0495` 0x10006960 and `magic_0500` 0x10006a20 (724 bytes).
#[allow(clippy::too_many_lines)] // the original's steps, one by one
pub fn short_volley_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let call = |o: u32| img.rel32_target(f + o);
    let task = fr.arg(g, 0);
    let s = g.read::<u32>(task + 4);
    let a = fr.esp + 4;
    let ebp = fr.esp.wrapping_sub(4);
    // push ebp; sub esp, 0x18; push esi; push edi; the return address
    let callee_esp = fr.esp.wrapping_sub(0x28);
    let step = g.read::<u32>(s + 0x18);
    match step {
        0 | 10 | 11 => {}
        0x2710 => {
            let x = g.read::<u32>(w(0x289));
            let t = slot_target(g, w(0x283), 411);
            // call width: 1
            let r = g.call(t, 0, &[x]).eax;
            if r == 0 {
                return Ret::eax(0);
            }
            let x = g.read::<u32>(w(0x2a5));
            let t = slot_target(g, w(0x29d), 213);
            // call width: 2
            g.call(t, 0, &[x, 0]);
            let node = g.read::<u32>(task);
            return Ret::eax(unring(g, node));
        }
        _ => return Ret::eax(step),
    }
    if step == 0 {
        let rec = ebp.wrapping_sub(0x18);
        let c = g.read::<u32>(w(0x30));
        for (i, v) in (0u32..).zip([w(0x3a), w(0x41), c, 0]) {
            g.write::<u32>(rec + 4 * i, v);
        }
        g.write::<u8>(rec + 0x10, 1);
        g.write::<u32>(w(0x55), w(0x59));
        let nodes = Frame {
            at: call(0x5e),
            esp: callee_esp,
            ecx: rec,
            ..*fr
        };
        crate::s4_summon::target_nodes(g, img, &nodes);
        g.write::<u32>(s + 0x18, 10);
    }
    // the shares again (the count's operand `n`)
    let shares = |g: &mut G, n: u32| -> u32 {
        let total = cvt(g.read::<u32>(s + 0x40));
        let mut i = 0u32;
        while i.cast_signed() < g.read::<i32>(w(n)) {
            let v = div(cvt(g.read::<u32>(s + 0x44 + 4 * i)), total);
            g.write::<f32>(s + 0x1c + 4 * i, v);
            i += 1;
        }
        s + 0x1c + 4 * i
    };
    if step != 11 {
        g.write::<u32>(s + 0x40, 0);
        let mut i = 0u32;
        let mut rec = w(0x7c);
        while i.cast_signed() < g.read::<i32>(if i == 0 { w(0x71) } else { w(0xab) }) {
            let x = g.read::<u32>(w(0x96));
            let r0 = g.read::<u32>(rec);
            let t = slot_target(g, w(0x90), 291);
            // call width: 5
            let r = g.call(t, rec, &[x, r0, a, a, a]).eax;
            g.write::<u32>(s + 0x44 + 4 * i, r);
            let total = g.read::<u32>(s + 0x40).wrapping_add(r);
            g.write::<u32>(s + 0x40, total);
            i += 1;
            rec = rec.wrapping_add(0x98);
        }
        if i > 0 {
            shares(g, 0xec);
        }
        // six `mov dword [esi + d32], 0xff` (10 bytes each)
        for j in 0..6u32 {
            g.write::<u32>(s.wrapping_add(w(0xf4 + 10 * j)), 0xff);
        }
        let n = g.read::<u32>(s + 0x18).wrapping_add(1);
        g.write::<u32>(s + 0x18, n);
        g.write::<u32>(s + 0x3c, 0);
    }
    let c = g.read::<i32>(s + 0x3c);
    g.write::<i32>(s + 0x3c, c.wrapping_sub(1));
    if c > 0 {
        return Ret::eax(c.wrapping_sub(1).cast_unsigned());
    }
    let total = g.read::<i32>(s + 0x40);
    let j = total.wrapping_rem(5);
    let q = total.wrapping_div(5);
    // the slot's stride (`imul imm32`) and its flag's displacement
    let jo = j.cast_unsigned().wrapping_mul(w(0x158));
    if g.read::<u32>(s.wrapping_add(jo).wrapping_add(w(0x15f))) != 0xff {
        return Ret::eax(q.cast_unsigned());
    }
    let t = slot_target(g, w(0x16e), 307);
    // call width: 0
    let r = crate::m0018::st0_f32(g.call(t, s, &[]).st0);
    g.write::<f32>(a, r);
    let mut x = sub(r, k(g, 0x18b));
    let m = g.read::<i32>(w(0x17e)).wrapping_sub(1);
    let mut i = 0u32;
    g.write::<u32>(a, 0);
    while i.cast_signed() < m {
        let share = g.read::<f32>(s + 0x1c + 4 * i);
        if share > x {
            break;
        }
        i += 1;
        x = sub(x, share);
        g.write::<u32>(a, i);
    }
    let t = slot_target(g, w(0x1b8), 307);
    // call width: 0
    let r2 = crate::m0018::st0_f32(g.call(t, s, &[]).st0);
    g.write::<f32>(ebp.wrapping_sub(4), r2);
    let time = add(mul(sub(r2, k(g, 0x1d0)), k(g, 0x1e1)), k(g, 0x1f2));
    let i = g.read::<u32>(a);
    let mut xmm = fr.xmm;
    xmm[2] = u128::from(time.to_bits());
    let throw = Frame {
        at: call(0x1f9),
        esp: callee_esp,
        ecx: s.wrapping_add(0x70).wrapping_add(jo),
        edx: i.wrapping_mul(0x98).wrapping_add(w(0x1ea)),
        xmm,
        ..*fr
    };
    // `magic_0500` throws the model object
    if img.matches(throw.at, &crate::s7_misc::MODEL_OBJECT_SETUP) {
        crate::s7_misc::model_object_setup(g, img, &throw);
    } else {
        crate::s7_misc::summon_object_setup(g, img, &throw);
    }
    let left = g.read::<u32>(s + 0x40).wrapping_sub(1);
    g.write::<u32>(s + 0x40, left);
    if left == 0 {
        let x = g.read::<u32>(w(0x20b));
        let t = slot_target(g, w(0x203), 228);
        // call width: 2
        let r = g.call(t, 0, &[x, 0x6a]).eax;
        g.write::<u32>(s + 0x18, 0x2710);
        return Ret::eax(r);
    }
    let i = g.read::<u32>(a);
    let at = s + 0x44 + 4 * i;
    let v = g.read::<u32>(at).wrapping_sub(1);
    g.write::<u32>(at, v);
    g.write::<u32>(s + 0x3c, w(0x233));
    if g.read::<i32>(w(0x239)) <= 0 {
        return Ret::eax(i);
    }
    Ret::eax(shares(g, 0x26f))
}

/// [`short_volley_task`] in `magic_0495` (0x10006960).
pub const SHORT_VOLLEY_TASK_0495: Sig = Sig {
    len: 724,
    hash: 0xe701_1c0b_87a6_093c,
    calls: &[
        (0x5e, Callee::Sig(&crate::s4_summon::TARGET_NODES)),
        (0x1f9, Callee::Sig(&crate::s7_misc::SUMMON_OBJECT_SETUP)),
    ],
    imports: &[],
};

/// [`short_volley_task`] in `magic_0500` (0x10006a20; the model object).
pub const SHORT_VOLLEY_TASK_0500: Sig = Sig {
    len: 724,
    hash: 0xe701_1c0b_87a6_093c,
    calls: &[
        (0x5e, Callee::Sig(&crate::s4_summon::TARGET_NODES)),
        (0x1f9, Callee::Sig(&crate::s7_misc::MODEL_OBJECT_SETUP)),
    ],
    imports: &[],
};
