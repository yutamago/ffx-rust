//! Round 20's own ports (own rows of `magic_0450` and up): the DLLs' own
//! code written out, and copies of ported functions bound by their own
//! `Sig`.

use crate::m0018::st0_f32;
use crate::r11_fades::stores;
use crate::runtime::slot_target;
use crate::sb_sse::{add, mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `y` wrapped once (`comiss` + `jbe`: an unordered value stays): above
/// `HI` less `T`, below `LO` plus `T'` (the floats at the operands `hi`,
/// `t`, `lo`, `t2` of `f`).
pub(crate) fn wrap_once<G: Game>(
    g: &G,
    img: &Image,
    f: u32,
    y: f32,
    (hi, t, lo, t2): (u32, u32, u32, u32),
) -> f32 {
    let c = |o: u32| g.read::<f32>(img.word(f + o));
    if y > c(hi) {
        sub(y, c(t))
    } else if c(lo) > y {
        add(y, c(t2))
    } else {
        y
    }
}

/// `magic_0508`'s start-up (0x10001340, 1755 bytes; internal, no
/// arguments; a /GS frame, `ebp = E - 4`: the point `V` at `ebp - 0x14`,
/// the matrix `M` at `ebp - 0x74`, `P` at `ebp - 0x34`, `Q` at `ebp -
/// 0x24`). Two words set, slot 719 `(PART, 8, 0)` into `G_H`, the part set
/// up ([`part_init`](crate::l5_objhost::part_init)); the yaw `K - slot 333
/// (*G_A)` wrapped once. The first object
/// ([`start_object_at`](crate::s4_misc::start_object_at) `(3, 0x35, -,
/// 0)`) at the caster (slot 262), its yaw, `y - K`, two words; `V` = the
/// first target's point (slot 262, slot 12's ST0 as `y`, `w` 1.0); its
/// height `Q.z` slot 251 of a near target (`*G_F` 0 and below 0x20), else
/// slot 19 times slot 17, plus `K`; `V += M Q` (`M` slot 235, 328 `(0)`
/// and 329 `(yaw)`; slot 229 into `P`, slot 248); the flight to `V`
/// ([`flight_task`](crate::s5_misc::flight_task) `(V, 1, 1, 0x1b)`, `ecx`
/// the flight object). Four words, then by
/// [`state_listed`](crate::s4_misc::state_listed): listed, two objects
/// `(0, 0x50, -, CB)` (the point and slot 268's height of the first
/// target) and `(2, 0x53, -, 0)` (the caster's point), else one `(1, 0x53,
/// -, CB)`, each with `V`'s words, the yaw and their block. The record
/// node ([`static_record_node`](crate::o4_small::static_record_node)), the
/// light words, the counted task
/// ([`counted_task`](crate::s4_small::counted_task)), the task
/// ([`task_start`](crate::s3_misc::task_start)), slot 623 of the caster
/// into a word; when slot 293 `(*G_S)` is 0, slot 306 `(7, 0, slot 719
/// (PART, 9, 0))`. Last the actor block
/// ([`me_block`](crate::sa_misc::me_block)) and a pool node
/// ([`take`](crate::sa_pool::take), kind 2; none writes through address 0
/// as the original does) last on the task list. Returns 0.
///
/// The `ecx` words pushed under the object starts' arguments (what an
/// earlier callee left) are read by no callee: the calls after a start
/// pass only the words above them (docs/formats/magic.md: a prefix of the
/// original's).
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn start_up_0508<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let rd = |g: &G, o: u32| g.read::<u32>(w(o));
    let call = |o: u32| img.rel32_target(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let e = fr.esp;
    let (v, m, p, q) = (
        e.wrapping_sub(0x18),
        e.wrapping_sub(0x78),
        e.wrapping_sub(0x38),
        e.wrapping_sub(0x28),
    );
    let one = 0x3f80_0000u32;
    // the first target's actor: `T[*(*G_B)]` (the table operand `tab`)
    let target = |g: &G, b: u32, tab: u32| {
        let i = u32::from(g.read::<u8>(rd(g, b)));
        g.read::<u32>(w(tab).wrapping_add(4 * i))
    };
    stores(g, img, f, &[0x27, 0x31]);
    // call width: 3
    let h = g.call(t(g, 0x14, 719), 0, &[w(0x23), 8, 0]).eax;
    g.write::<u32>(w(0x41), h);
    crate::l5_objhost::part_init(g, img, call(0x46));
    // call width: 3 (slot 719's words stay under the caster)
    let r = g.call(t(g, 0x4b, 333), 0, &[rd(g, 0x51), 8, 0]);
    let r = st0_f32(r.st0);
    let yaw = wrap_once(g, img, f, sub(k(g, 0x64), r), (0x73, 0x82, 0x8c, 0x99));
    let scale = k(g, 0xa6);
    crate::s4_misc::start_object_at(g, img, call(0xb9), w(0xb4), scale, 3, 0x35, 0);
    // call width: 6 (the start's index and owner stay)
    g.call(
        t(g, 0xbe, 262),
        0,
        &[rd(g, 0xd9), w(0xd3), w(0xce), w(0xc3), 3, 0x35],
    );
    g.write::<f32>(w(0xed), yaw);
    let y = sub(g.read::<f32>(w(0xf5)), k(g, 0xfd));
    stores(g, img, f, &[0x101, 0x10b]);
    g.write::<f32>(w(0x119), y);
    let a = target(g, 0xe5, 0x123);
    // call width: 4
    g.call(t(g, 0x134, 262), 0, &[a, v, v + 4, v + 8]);
    let r = g.call(t(g, 0x142, 12), 0, &[a, a, v, v + 4]);
    g.write::<f32>(v + 4, st0_f32(r.st0));
    let b = rd(g, 0x151);
    g.write::<u32>(v + 0xc, one);
    g.write::<u32>(q, 0);
    g.write::<u32>(q + 4, 0);
    let near = (rd(g, 0x15a) == 0).then(|| u32::from(g.read::<u8>(b)));
    let height = match near {
        Some(i) if i.cast_signed() < 0x20 => {
            let r = g.call(t(g, 0x180, 251), 0, &[i]);
            st0_f32(r.st0)
        }
        _ => {
            let i = u32::from(g.read::<u8>(b));
            let a = g.read::<u32>(w(0x19f).wrapping_add(4 * i));
            let r1 = st0_f32(g.call(t(g, 0x1a4, 19), 0, &[a]).st0);
            let r2 = st0_f32(g.call(t(g, 0x1b2, 17), 0, &[a, a]).st0);
            mul(r2, r1)
        }
    };
    let height = add(height, k(g, 0x1d0));
    g.write::<f32>(q + 8, height);
    g.call(t(g, 0x1d9, 235), 0, &[m]);
    // call width: 3
    g.call(t(g, 0x1f7, 328), 0, &[m, m, 0]);
    // call width: 3
    g.call(t(g, 0x216, 329), 0, &[m, m, yaw.to_bits()]);
    g.write::<u32>(q + 0xc, one);
    // call width: 3
    g.call(t(g, 0x22f, 229), 0, &[p, m, q]);
    // call width: 3
    g.call(t(g, 0x24c, 248), 0, &[v, v, p]);
    // push 0x1b; sub esp, 8; push V: the return address at E - 0xa0
    let esp = e.wrapping_sub(0xa0);
    for (i, x) in (1u32..).zip([v, one, one, 0x1b]) {
        g.write::<u32>(esp + 4 * i, x);
    }
    g.write::<u32>(v + 0xc, one);
    let fl = Frame {
        at: call(0x288),
        esp,
        ecx: w(0x26c),
        xmm: [0, 0, 0, u128::from(rd(g, 0x25c))],
        ..*fr
    };
    crate::s5_misc::flight_task(g, img, &fl);
    stores(g, img, f, &[0x291, 0x29b, 0x2a5]);
    g.write::<u16>(w(0x2b1), 0);
    let sl = Frame {
        at: call(0x2b6),
        esp: e.wrapping_sub(0x90),
        ..*fr
    };
    let listed = crate::s4_misc::state_listed(g, img, &sl).eax;
    let scale = k(g, 0x2be);
    let cb = w(0x2c3);
    // the block of an object: the words from `+0x10` on (the operand
    // offsets of `mov dword [abs], imm32`), the halfword and the pointer
    let block = |g: &mut G, ats: &[u32], half: u32, ptr: u32| {
        stores(g, img, f, ats);
        g.write::<u16>(w(half), 0);
        stores(g, img, f, &[ptr]);
    };
    let edi = if listed == 0 {
        crate::s4_misc::start_object_at(g, img, call(0x2da), w(0x2d5), scale, 1, 0x53, cb);
        let a = target(g, 0x2df, 0x2f3);
        // call width: 6 (the start's index and owner stay)
        g.call(
            t(g, 0x2f8, 262),
            0,
            &[a, w(0x2fd), w(0x2ec), w(0x2e4), 1, 0x53],
        );
        g.call(t(g, 0x30b, 12), 0, &[a, a, w(0x2fd), w(0x2ec)]);
        for (to, from) in [(0x31e, v), (0x32b, v + 8), (0x338, v + 0xc)] {
            let x = g.read::<u32>(from);
            g.write::<u32>(w(to), x);
        }
        g.write::<f32>(w(0x345), yaw);
        let y = sub(g.read::<f32>(v + 4), k(g, 0x352));
        stores(g, img, f, &[0x35b, 0x365]);
        g.write::<f32>(w(0x373), y);
        block(
            g,
            &[0x377, 0x381, 0x38b, 0x395, 0x39f, 0x3a9, 0x3b3],
            0x3bf,
            0x3c3,
        );
        0x53
    } else {
        crate::s4_misc::start_object_at(g, img, call(0x3df), w(0x3da), scale, 0, 0x50, cb);
        let a = target(g, 0x3e4, 0x3f8);
        // call width: 6 (the start's index and owner stay)
        g.call(
            t(g, 0x3fd, 262),
            0,
            &[a, w(0x402), w(0x3f1), w(0x3e9), 0, 0x50],
        );
        // call width: 3
        g.call(t(g, 0x410, 268), 0, &[a, 0x10, w(0x415)]);
        let scale = k(g, 0x42d);
        g.write::<f32>(w(0x43b), yaw);
        let x = g.read::<u32>(v);
        g.write::<u32>(w(0x449), x);
        stores(g, img, f, &[0x45b]);
        let z = g.read::<u32>(v + 8);
        g.write::<u32>(w(0x469), z);
        stores(g, img, f, &[0x46d, 0x477]);
        block(
            g,
            &[0x481, 0x48b, 0x495, 0x49f, 0x4a9, 0x4b3, 0x4bd, 0x4c7],
            0x4d3,
            0x4d7,
        );
        crate::s4_misc::start_object_at(g, img, call(0x4e5), w(0x457), scale, 2, 0x53, 0);
        let c = rd(g, 0x4f0);
        // call width: 6 (the start's index and owner stay)
        g.call(
            t(g, 0x4ea, 262),
            0,
            &[c, w(0x505), w(0x500), w(0x4fb), 2, 0x53],
        );
        g.call(t(g, 0x50d, 12), 0, &[c, c, w(0x505), w(0x500)]);
        g.write::<f32>(w(0x520), yaw);
        for (to, from) in [(0x52d, v), (0x53a, v + 8), (0x547, v + 0xc)] {
            let x = g.read::<u32>(from);
            g.write::<u32>(w(to), x);
        }
        let y = sub(g.read::<f32>(v + 4), k(g, 0x554));
        stores(g, img, f, &[0x55b]);
        g.write::<f32>(w(0x569), y);
        0x50
    };
    let rn = Frame {
        at: call(0x570),
        esp: e.wrapping_sub(0x90),
        ..*fr
    };
    crate::o4_small::static_record_node(g, img, &rn);
    let fl = g.read::<u16>(w(0x577));
    g.write::<u16>(w(0x577), fl | 3);
    stores(
        g,
        img,
        f,
        &[0x57c, 0x586, 0x590, 0x59a, 0x5a4, 0x5ae, 0x5b8, 0x5c2],
    );
    g.write::<u8>(w(0x5ce), 0x80);
    g.write::<u16>(w(0x5d6), 0);
    g.write::<u8>(w(0x5de), 0);
    g.write::<u32>(w(0x5e5), edi);
    g.write::<u16>(w(0x5ec), 0x48);
    let ct = Frame {
        at: call(0x5f3),
        esp: e.wrapping_sub(0x90),
        ..*fr
    };
    crate::s4_small::counted_task(g, img, &ct);
    // push ecx; push FN: the return address at E - 0x98
    let esp = e.wrapping_sub(0x98);
    g.write::<u32>(esp + 4, w(0x5f9));
    let ts = Frame {
        at: call(0x5fe),
        esp,
        ..*fr
    };
    crate::s3_misc::task_start(g, img, &ts);
    let c = rd(g, 0x615);
    stores(g, img, f, &[0x619]);
    // call width: 8 (the task's word stays)
    let x = g
        .call(t(g, 0x603, 623), 0, &[c, 0, 0, 0, 0, 0, 0, w(0x5f9)])
        .eax;
    let s = rd(g, 0x62d);
    g.write::<u32>(w(0x632), x);
    stores(g, img, f, &[0x63b]);
    if g.call(t(g, 0x637, 293), 0, &[s]).eax == 0 {
        // call width: 3
        let h = g.call(t(g, 0x658, 719), 0, &[w(0x665), 9, 0]).eax;
        // call width: 3
        g.call(t(g, 0x66d, 306), 0, &[7, 0, h]);
    }
    crate::sa_misc::me_block(g, img, call(0x686), w(0x681));
    let node = crate::sa_pool::take(g, img, call(0x693), w(0x68e));
    let listed = rd(g, 0x699);
    g.write::<u8>(node.wrapping_add(0x18), 2);
    g.write::<u32>(node.wrapping_add(4), 0);
    if listed == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x6bf), node);
    } else {
        let tail = rd(g, 0x6ad);
        g.write::<u32>(node, tail);
        g.write::<u32>(tail.wrapping_add(4), node);
    }
    g.write::<u32>(node.wrapping_add(8), node);
    g.write::<u32>(w(0x6ca), node);
    Ret::eax(0)
}

/// [`start_up_0508`] (1755 bytes, 1 overlay).
pub const START_UP_0508: Sig = Sig {
    len: 1755,
    hash: 0x95d1_d73b_9558_226f,
    calls: &[
        (0x46, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0xb9, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x288, Callee::Sig(&crate::s5_misc::FLIGHT_TASK)),
        (0x2b6, Callee::Sig(&crate::s4_misc::STATE_LISTED)),
        (0x2da, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x3df, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x4e5, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (
            0x570,
            Callee::Sig(&crate::o4_small::STATIC_RECORD_NODE_0508_2B80),
        ),
        (0x5f3, Callee::Sig(&crate::s4_small::COUNTED_TASK_20)),
        (0x5fe, Callee::Sig(&crate::s3_misc::TASK_START)),
        (0x686, Callee::Sig(&crate::sa_misc::ME_BLOCK)),
        (0x693, Callee::Sig(&crate::sa_pool::TAKE)),
        // __security_check_cookie
        (0x6d3, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0508`'s load step (0x10001210, the 118-byte shape) running
/// [`start_up_0508`].
pub fn load_0508<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::r9_load::load_step(g, img, fr, crate::r9_load::Shape::Zero, start_up_0508)
}

/// [`load_0508`].
pub const LOAD_0508: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&START_UP_0508))],
    imports: &[],
};

/// `magic_0514`'s glow and spark task (0x100078f0, cdecl `(task)`; an
/// 8-aligned frame `A`): the record `R = task[1]`, its state `+0x18`,
/// wait `+0x1c` and count `+0x20`. State 0 sets the wait 0x28 and goes to
/// 1; 1, the wait out, goes to 2 with the count 0x10; 2, the wait out:
/// with `i = n % 5` (signed), when the glow `R + 0x2d8 + 0x1ec i` and the
/// spark `R + 0xc80 + 0x2d0 i` are both done (`+0x18` 20000), the glow
/// set up ([`glow_setup`](crate::m0514::glow_setup), `edx` = `R[0x170]`,
/// `(R + 0x178)`) and the spark
/// ([`spark_setup`](crate::m0514::spark_setup), `edx` = `R[0x174]`,
/// `(R[0x2d0], R[0x2d4])`); the count down: not 0, the wait 6 (less the
/// closing step: 5) and the spark's `eax` returned; 0, state 10000. State
/// 10000 with bit 2 of byte `+0x2b8` and the five glows' states all
/// sharing a bit of 20000: the three nodes `+0x24`, `+0x178`, `+0x1a4`
/// leave their parents ([`leave`](crate::sa_misc::leave)), the task's node
/// leaves its ring, state 20000. Every other way out steps the wait down
/// by one. Returns the `eax` each way leaves (the state, the count, the
/// spark's answer, the count over 5, the node's old link).
#[allow(clippy::too_many_lines)] // the original's states, in order
pub fn glow_task_0514<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task.wrapping_add(4));
    let at = |o: u32| r.wrapping_add(o);
    let wait = |g: &G| g.read::<u32>(at(0x1c)).cast_signed();
    // the closing `dec [R + 0x1c]` of every way out but the spark's
    let out = |g: &mut G, eax: u32| {
        let c = g.read::<u32>(at(0x1c));
        g.write::<u32>(at(0x1c), c.wrapping_sub(1));
        Ret::eax(eax)
    };
    // push ebp; and esp, -8; push ecx, ebx, esi, edi
    let a = fr.esp.wrapping_sub(4) & !7;
    let mut state = g.read::<u32>(at(0x18));
    let mut eax = state;
    while state <= 2 {
        if state == 0 {
            g.write::<u32>(at(0x1c), 0x28);
            g.write::<u32>(at(0x18), 1);
            state = 1;
            eax = 1;
        } else if state == 1 {
            eax = 0;
        }
        if state == 1 {
            if wait(g) > 0 {
                return out(g, eax);
            }
            let s = g.read::<u32>(at(0x18));
            g.write::<u32>(at(0x18), s.wrapping_add(1));
            g.write::<u32>(at(0x1c), 0);
            g.write::<u32>(at(0x20), 0x10);
        } else {
            if wait(g) > 0 {
                return out(g, 2);
            }
            let n = g.read::<u32>(at(0x20));
            // cdq; idiv 5: the quotient stays in `eax`
            let (q, i) = (
                n.cast_signed().wrapping_div(5).cast_unsigned(),
                n.cast_signed().wrapping_rem(5).cast_unsigned(),
            );
            let glow = at(i.wrapping_mul(0x1ec));
            if g.read::<u32>(glow.wrapping_add(0x2f0)) != 0x4e20 {
                return out(g, q);
            }
            let spark = at(i.wrapping_mul(0x2d0));
            if g.read::<u32>(spark.wrapping_add(0xc98)) != 0x4e20 {
                return out(g, q);
            }
            // push R + 0x178: the return address at A - 0x18
            let esp = a.wrapping_sub(0x18);
            g.write::<u32>(esp + 4, at(0x178));
            let gf = Frame {
                at: img.rel32_target(f + 0xb8),
                esp,
                ecx: glow.wrapping_add(0x2d8),
                edx: g.read::<u32>(at(0x170)),
                ..*fr
            };
            crate::m0514::glow_setup(g, img, &gf);
            // push R[0x2d4]; push R[0x2d0]: the return address at A - 0x1c
            let esp = a.wrapping_sub(0x1c);
            let (w0, w1) = (g.read::<u32>(at(0x2d0)), g.read::<u32>(at(0x2d4)));
            g.write::<u32>(esp + 4, w0);
            g.write::<u32>(esp + 8, w1);
            let sf = Frame {
                at: img.rel32_target(f + 0xd8),
                esp,
                ecx: spark.wrapping_add(0xc80),
                edx: g.read::<u32>(at(0x174)),
                ..*fr
            };
            let e = crate::m0514::spark_setup(g, img, &sf);
            let n = g.read::<u32>(at(0x20)).wrapping_sub(1);
            g.write::<u32>(at(0x20), n);
            if n != 0 {
                g.write::<u32>(at(0x1c), 5);
                return Ret::eax(e);
            }
            g.write::<u32>(at(0x18), 0x2710);
        }
        state = g.read::<u32>(at(0x18));
        eax = state;
    }
    if state != 0x2710 {
        return out(g, eax);
    }
    if g.read::<u8>(at(0x2b8)) & 4 == 0 {
        return out(g, 0x2710);
    }
    let mut all = 0x4e20;
    for o in [0xaa0u32, 0x8b4, 0x6c8, 0x4dc, 0x2f0] {
        all &= g.read::<u32>(at(o));
    }
    if all == 0 {
        return out(g, 0);
    }
    let mut eax = all;
    for o in [0x24u32, 0x178, 0x1a4] {
        eax = crate::sa_misc::leave(g, at(o), eax);
    }
    let node = g.read::<u32>(task);
    let (prev, next) = (
        g.read::<u32>(node.wrapping_add(4)),
        g.read::<u32>(node.wrapping_add(8)),
    );
    g.write::<u32>(prev.wrapping_add(8), next);
    let (next, prev) = (
        g.read::<u32>(node.wrapping_add(8)),
        g.read::<u32>(node.wrapping_add(4)),
    );
    g.write::<u32>(next.wrapping_add(4), prev);
    g.write::<u32>(node.wrapping_add(8), node);
    g.write::<u32>(node.wrapping_add(4), node);
    g.write::<u32>(at(0x18), 0x4e20);
    out(g, prev)
}

/// [`glow_task_0514`] (401 bytes, 1 overlay).
pub const GLOW_TASK_0514: Sig = Sig {
    len: 401,
    hash: 0x84df_b67e_221d_61dd,
    calls: &[
        (0xb8, Callee::Sig(&crate::m0514::GLOW_SETUP)),
        (0xd8, Callee::Sig(&crate::m0514::SPARK_SETUP)),
        (0x13b, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x146, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x151, Callee::Sig(&crate::sa_misc::LEAVE)),
    ],
    imports: &[],
};

/// A pool node taken as `magic_0680`'s start-up inlines it (the count
/// `u16` at the operand `n`, the free head `node`; `pop`: the operands of
/// [`pool_pop`](crate::r13_misc::pool_pop) after the count) and appended
/// to the task list: `rec` the node's byte `+0x18` 1 and `+0x14` the
/// immediate at that operand; the list's head word `first` (written at
/// `first_w` when it was empty), else after `tail` (or the tail word at
/// `last`). With none free the node is 0 and the original writes through
/// it (not run). Returns the node and the new free head.
#[allow(clippy::too_many_arguments)] // the inlined steps' operands
fn pool_append_0680<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    node: u32,
    n: u32,
    pop: (u32, u32, u32, u32),
    rec: Option<u32>,
    (first, first_w, tail, last): (u32, u32, Option<u32>, u32),
) -> (u32, u32) {
    let w = |o: u32| img.word(f + o);
    let (node, head) = if g.read::<u16>(w(n)) == 0 {
        (0, 0)
    } else {
        let (free, free0, cb, n_out) = pop;
        let (_, head) = crate::r13_misc::pool_pop(g, img, f, node, (n, free, free0, cb, n_out));
        (node, head)
    };
    let any = g.read::<u32>(w(first));
    if let Some(r) = rec {
        g.write::<u8>(node.wrapping_add(0x18), 1);
        g.write::<u32>(node.wrapping_add(0x14), w(r));
    }
    g.write::<u32>(node.wrapping_add(4), 0);
    if any == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(first_w), node);
    } else {
        let t = tail.unwrap_or_else(|| g.read::<u32>(w(last)));
        g.write::<u32>(node, t);
        g.write::<u32>(t.wrapping_add(4), node);
    }
    g.write::<u32>(node.wrapping_add(8), node);
    (node, head)
}

/// `magic_0680`'s start-up (0x100013e0, 1802 bytes, run by its load
/// step; cdecl, no arguments; a cookie frame, `ebp = E - 4`, the point
/// `L` at `ebp - 0x24`): three words set, slot 719 `(PART, 8, 0)` into
/// `G_H`, the part set up ([`part_init`](crate::l5_objhost::part_init)),
/// the actors dealt into the bytes at `ebp - 0x14`
/// ([`deal`](crate::sb_misc::deal)); slot 333 `(*G_X)` asked (its ST0
/// dropped), two words. For each of the `*G_N` dealt bytes `d` (signed;
/// the object `O` from `+0x70`, 0x130 a step, the record `E` from
/// `+0xa4`, 0x34 a step, the owner 0x2d a step): an angle slot 314 `(pi)`
/// and a size `s = slot 404 (0.2) + K`; the actor `A` (`*G_X` for -1,
/// else `T[B[d]]`) gives `L` (slot 262, slot 268 `(A, 0x11, L)`, `w`
/// 1.0). By [`actor_kind_check`](crate::sa_misc::actor_kind_check) of
/// `d`: 0 starts two objects
/// ([`start_object_owners`](crate::s4_misc::start_object_owners): `O`
/// `(1, owner, 0x12, CB)` at `s K1` and `O + 0x130` `(0, owner + 0x14, 0,
/// 0)` at `s K2`, the second with slot 12's height of `A` less `K` and
/// `+0x108` its block; the object step taken twice); else one `(2, owner,
/// 0x12, CB)` at `s K2` with `+0x108` its block. Each gets `L`, the
/// angle, and the first its number (`*G_X2` or `B[d]`) and the record `E`
/// cleared with its two callbacks. Then the light words, a pool node
/// (record `+0x4d6`) last on the task list, more light words with the
/// last owner `+ 0x28`, a second node (record `+0x60f`) after it, slot 271
/// `(4)` and a third node without a record. Returns 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn start_up_0680<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let rd = |g: &G, o: u32| g.read::<u32>(w(o));
    let call = |o: u32| img.rel32_target(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let e = fr.esp;
    let dealt = e.wrapping_sub(0x18);
    let l = e.wrapping_sub(0x28);
    stores(g, img, f, &[0x27, 0x31, 0x3b]);
    // call width: 3
    let h = g.call(t(g, 0x11, 719), 0, &[w(0x23), 8, 0]).eax;
    g.write::<u32>(w(0x4b), h);
    crate::l5_objhost::part_init(g, img, call(0x50));
    // ebx, esi, edi and two of slot 719's words above: the return
    // address at E - 0x58
    crate::sb_misc::deal(g, img, call(0x58), e.wrapping_sub(0x58), dealt);
    // call width: 3 (slot 719's words stay)
    g.call(t(g, 0x5d, 333), 0, &[rd(g, 0x63), 8, 0]);
    stores(g, img, f, &[0x80, 0x8a]);
    let actor = |g: &G, b: i32, x: u32, bt: u32, tab: u32| {
        if b == -1 {
            rd(g, x)
        } else {
            let n = g.read::<u8>(rd(g, bt).wrapping_add(b.cast_unsigned()));
            g.read::<u32>(w(tab).wrapping_add(4 * u32::from(n)))
        }
    };
    let number = |g: &G, b: i32, x2: u32, bt: u32| {
        if b == -1 {
            rd(g, x2)
        } else {
            u32::from(g.read::<u8>(rd(g, bt).wrapping_add(b.cast_unsigned())))
        }
    };
    // `L` into `O + 0xd0..+0xdc`, the angle into `O + 0xc4`
    let place = |g: &mut G, o: u32, ang: f32| {
        for i in 0..4u32 {
            let v = g.read::<u32>(l + 4 * i);
            g.write::<u32>(o + 0xd0 + 4 * i, v);
        }
        g.write::<f32>(o + 0xc4, ang);
    };
    // the record `E`: its words cleared, the two callbacks
    let record = |g: &mut G, r: u32, cb: (u32, u32)| {
        for o in [4u32, 0, 0xffff_fffc, 8] {
            g.write::<u32>(r.wrapping_add(o), 0);
        }
        g.write::<u16>(r.wrapping_add(0x1c), 0);
        g.write::<u32>(r.wrapping_add(0xc), w(cb.0));
        g.write::<u32>(r.wrapping_add(0x10), w(cb.1));
        g.write::<u32>(r.wrapping_add(0x14), 0);
        g.write::<u32>(r.wrapping_add(0x18), 0);
    };
    let (mut owner, mut obj, mut rec) = (0u32, w(0x70), w(0xa4));
    let mut i = 0u32;
    while i.cast_signed() < g.read::<i32>(w(0x99)) {
        let b = i32::from(g.read::<i8>(dealt.wrapping_add(i)));
        let ang = st0_f32(g.call(t(g, 0xb6, 314), 0, &[0x4049_0fdb]).st0);
        let r = st0_f32(g.call(t(g, 0xcb, 404), 0, &[0x3e4c_cccd]).st0);
        let s = add(r, k(g, 0xf0));
        let a = actor(g, b, 0xff, 0x106, 0x111);
        // call width: 4
        g.call(t(g, 0x126, 262), 0, &[a, l, l + 4, l + 8]);
        // call width: 3
        g.call(t(g, 0x137, 268), 0, &[a, 0x11, l]);
        g.write::<u32>(l + 0xc, 0x3f80_0000);
        let kind = crate::sa_misc::actor_kind_check(g, img, call(0x155), b.cast_unsigned());
        let cb = w(0x15a);
        if kind == 0 {
            crate::s4_misc::start_object_owners(
                g,
                img,
                call(0x17d),
                obj,
                mul(s, k(g, 0x176)),
                1,
                owner,
                (0x12, cb),
            );
            place(g, obj, ang);
            let n = number(g, b, 0x1cb, 0x1d2);
            let scale = mul(s, k(g, 0x1e3));
            g.write::<u32>(obj + 0x120, n);
            record(g, rec, (0x232, 0x239));
            g.write::<u32>(obj + 0x110, rec.wrapping_sub(0x14));
            obj = obj.wrapping_add(0x130);
            crate::s4_misc::start_object_owners(
                g,
                img,
                call(0x24c),
                obj,
                scale,
                0,
                owner.wrapping_add(0x14),
                (0, 0),
            );
            for i in 0..4u32 {
                let v = g.read::<u32>(l + 4 * i);
                g.write::<u32>(obj + 0xd0 + 4 * i, v);
            }
            let a = actor(g, b, 0x28d, 0x294, 0x29f);
            let y = st0_f32(g.call(t(g, 0x2a5, 12), 0, &[a]).st0);
            let y = sub(y, k(g, 0x2ba));
            g.write::<u32>(obj + 0x108, w(0x2c7));
            g.write::<f32>(obj + 0xd4, y);
            g.write::<f32>(obj + 0xc4, ang);
        } else {
            crate::s4_misc::start_object_owners(
                g,
                img,
                call(0x2f8),
                obj,
                mul(s, k(g, 0x2ee)),
                2,
                owner,
                (0x12, cb),
            );
            place(g, obj, ang);
            let n = number(g, b, 0x346, 0x34d);
            g.write::<u32>(obj + 0x120, n);
            record(g, rec, (0x382, 0x389));
            g.write::<u32>(obj + 0x110, rec.wrapping_sub(0x14));
            g.write::<u32>(obj + 0x108, w(0x3a7));
        }
        i += 1;
        owner = owner.wrapping_add(0x2d);
        obj = obj.wrapping_add(0x130);
        rec = rec.wrapping_add(0x34);
    }
    stores(
        g,
        img,
        f,
        &[
            0x3d7, 0x3e1, 0x3eb, 0x3f5, 0x3ff, 0x409, 0x413, 0x41d, 0x427, 0x431, 0x43b, 0x445,
            0x44f,
        ],
    );
    let node = rd(g, 0x46a);
    let (first, head) = pool_append_0680(
        g,
        img,
        f,
        node,
        0x3d3,
        (0x494, 0x494, 0x4b9, 0x4c4),
        Some(0x4d6),
        (0x4ca, 0x4f7, None, 0x4e4),
    );
    stores(
        g,
        img,
        f,
        &[
            0x501, 0x50b, 0x515, 0x51f, 0x529, 0x533, 0x53d, 0x547, 0x551, 0x55b, 0x565,
        ],
    );
    g.write::<u32>(w(0x570), owner.wrapping_add(0x28));
    stores(g, img, f, &[0x574, 0x57e, 0x588]);
    let (second, _) = pool_append_0680(
        g,
        img,
        f,
        head,
        0x5fd,
        (0x5b9, 0x5c1, 0x5f2, 0x5fd),
        Some(0x60f),
        (0x603, 0x62b, Some(first), 0),
    );
    stores(g, img, f, &[0x637]);
    g.write::<u32>(w(0x64b), second);
    g.call(t(g, 0x630, 271), 0, &[4]);
    let node = rd(g, 0x666);
    let (third, _) = pool_append_0680(
        g,
        img,
        f,
        node,
        0x654,
        (0x683, 0x68b, 0x6b7, 0x6c2),
        None,
        (0x6c8, 0x6ed, None, 0x6da),
    );
    g.write::<u32>(w(0x6fb), third);
    Ret::eax(0)
}

/// [`start_up_0680`] (1802 bytes, 1 overlay).
pub const START_UP_0680: Sig = Sig {
    len: 1802,
    hash: 0x7c44_df13_d65b_d454,
    calls: &[
        (0x50, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x58, Callee::Sig(&crate::sb_misc::DEAL)),
        (0x155, Callee::Sig(&crate::sa_misc::ACTOR_KIND_CHECK)),
        (0x17d, Callee::Sig(&crate::s4_misc::START_OBJECT_OWNERS)),
        (0x24c, Callee::Sig(&crate::s4_misc::START_OBJECT_OWNERS)),
        (0x2f8, Callee::Sig(&crate::s4_misc::START_OBJECT_OWNERS)),
        // __security_check_cookie
        (0x702, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0680`'s load step (0x100012b0, the 119-byte shape) running
/// [`start_up_0680`].
pub fn load_0680<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::r9_load::load_step(g, img, fr, crate::r9_load::Shape::One, start_up_0680)
}

/// [`load_0680`].
pub const LOAD_0680: Sig = Sig {
    len: 119,
    hash: 0x2da6_1889_6875_2369,
    calls: &[(0x5c, Callee::Sig(&START_UP_0680))],
    imports: &[],
};

/// `magic_0645`'s colour object set-up (0x100077e0, internal: `ecx` = the
/// object, `edx` a word; an 8-aligned frame `A`, the record at `A -
/// 0x18`), as [`colour_setup`](crate::m0514::colour_setup) with other
/// offsets: the object joins the DLL's ring
/// ([`ring_join`](crate::h6_setup::ring_join)); `b = obj + 0x168` a child
/// of the DLL's node list (handlers `+0xa3` after), slot 325 `(obj +
/// 0x184, edx)` (a copy) and `+0x190` 1.0; `c = obj + 0x194` a child too
/// (`+0x10a`); `+0x1b0..` = `(1, 1, 1, 0.2)`; the node insert
/// ([`node_insert`](crate::nodes::node_insert)) of `[obj + 0x1c, LIST, G,
/// b, NAME, c]` and a particle node
/// ([`particle_node`](crate::sb_nodes::particle_node)) of `[obj + 0x1c0,
/// LIST2, obj + 0x1c, *G_O, PART, 1]` (vtable `+0x177` first); `+0x18` 0.
/// Returns the particle node's `eax`.
pub fn colour_setup_0645<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    use crate::h6_setup::{ring_join, static_child};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let obj = fr.ecx;
    ring_join(g, img, f, obj, [0x16, 0x1d, 0x25, 0x2f, 0x34, 0x43, 0x4a]);
    let b = obj + 0x168;
    static_child(
        g,
        img,
        f,
        b,
        [0x5d, 0x64, 0x69, 0x71, 0x76, 0x7e, 0x8a, 0x9d],
    );
    g.write::<u32>(b, w(0xa3));
    // call width: 2
    g.call(slot_target(g, w(0x97), 325), 0, &[obj + 0x184, fr.edx]);
    g.write::<u32>(obj + 0x190, 0x3f80_0000);
    let c = obj + 0x194;
    static_child(
        g,
        img,
        f,
        c,
        [0xd1, 0xd8, 0xdd, 0xe5, 0xea, 0xf2, 0xfe, 0x104],
    );
    g.write::<u32>(c, w(0x10a));
    for (o, v) in [
        (0x1b0u32, 0x3f80_0000u32),
        (0x1b4, 0x3f80_0000),
        (0x1b8, 0x3f80_0000),
        (0x1bc, 0x3e4c_cccd),
    ] {
        g.write::<u32>(obj + o, v);
    }
    // push ebp; and esp, -8; sub esp, 0x1c; three pushes
    let a = fr.esp.wrapping_sub(4) & !7;
    let rec = a.wrapping_sub(0x18);
    for (i, v) in (0u32..).zip([obj + 0x1c, w(0x149), w(0x151), b, w(0x15d), c]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    crate::nodes::node_insert(
        g,
        img,
        &Frame {
            at: img.rel32_target(f + 0x162),
            esp: a.wrapping_sub(0x2c),
            ecx: rec,
            ..Frame::default()
        },
    );
    let o = g.read::<u32>(w(0x167));
    let node = obj + 0x1c0;
    g.write::<u32>(rec, node);
    g.write::<u32>(node, w(0x177));
    for (i, v) in (1u32..).zip([w(0x183), obj + 0x1c, o, w(0x193), 1]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    let r = crate::sb_nodes::particle_node(g, img, img.rel32_target(f + 0x1a0), rec);
    g.write::<u32>(obj + 0x18, 0);
    r
}

/// [`colour_setup_0645`] (434 bytes, 1 overlay).
pub const COLOUR_SETUP_0645: Sig = Sig {
    len: 434,
    hash: 0xfb6c_3d45_a6d2_51b5,
    calls: &[
        (0x162, Callee::Sig(&crate::nodes::INSERT)),
        (0x1a0, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
    ],
    imports: &[],
};

/// `magic_0610`'s summon matrix set-up (0x1000b500, 622 bytes; a table
/// entry, no arguments; a cookie frame, `ebp = E - 4`, the point `L` at
/// `ebp - 0x14`): with `R = *G_E`: a buffer slot 0 `(0x3018)` (into
/// `*G_BUF`, cleared by slot 26 `(buf, 0x3018, 1)`), the byte `G_N` 0x20,
/// slot 10 `(buf, 0)`; the first target `A = T[*(*G_B)]`; with `M = R +
/// 0xc20` (never 0 in the game) slot 210 `(A, M)`, else, unless `*G_F` or
/// a target 0x20 and up, slots 217 and 255 `(i, L)`; slot 211 `(A, M)`;
/// `R + 0xc60..` copied to `R + 0xc80..` (8 words); `M` the unit matrix
/// (slot 235), its diagonal slot 277 `(A)` times `K`, times slots 15, 16
/// and 17 of `A` (the slot's factor first, then the diagonal's, then
/// it last again); slots 330, 328 and 329 `(M, M, a)` by `R + 0xc88`,
/// `+0xc80`, `+0xc84`; slot 257 `(A, 1)`. Returns slot 257's `eax`.
pub fn summon_matrix_0610<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let rd = |g: &G, o: u32| g.read::<u32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    // the first target (`*G_B`'s byte into the table)
    let target = |g: &G, b: u32, tab: u32| {
        let i = u32::from(g.read::<u8>(rd(g, b)));
        g.read::<u32>(w(tab).wrapping_add(4 * i))
    };
    let r = rd(g, 0x1a);
    let buf = g.call(t(g, 0x11, 0), 0, &[0x3018]).eax;
    // call width: 3 (the first call's word stays)
    g.call(t(g, 0x29, 26), 0, &[0x3018, 1, 0x3018]);
    g.write::<u8>(w(0x42), 0x20);
    g.write::<u32>(w(0x49), buf);
    // call width: 5 (the earlier calls' words stay)
    g.call(t(g, 0x3c, 10), 0, &[buf, 0, 0x3018, 1, 0x3018]);
    let b = rd(g, 0x57);
    let m = r.wrapping_add(0xc20);
    let a = target(g, 0x57, 0x6a);
    if m == 0 {
        let i = u32::from(g.read::<u8>(b));
        if rd(g, 0x8b) == 0 && i.cast_signed() < 0x20 {
            let l = fr.esp.wrapping_sub(0x18);
            g.call(t(g, 0x9f, 217), 0, &[i, l]);
            g.call(t(g, 0xb1, 255), 0, &[i, l]);
        }
    } else {
        g.call(t(g, 0x78, 210), 0, &[a, m]);
    }
    g.call(t(g, 0xc7, 211), 0, &[a, m]);
    for i in 0..8u32 {
        let x = g.read::<u32>(r.wrapping_add(0xc60 + 4 * i));
        g.write::<u32>(r.wrapping_add(0xc80 + 4 * i), x);
    }
    g.call(t(g, 0x134, 235), 0, &[m, a, m]);
    let a2 = target(g, 0x142, 0x14c);
    let d = st0_f32(g.call(t(g, 0x151, 277), 0, &[a2, m, a, m]).st0);
    let d = mul(d, g.read::<f32>(w(0x169)));
    for o in [0x28u32, 0x14, 0] {
        g.write::<f32>(m.wrapping_add(o), d);
    }
    let a3 = target(g, 0x17c, 0x186);
    for (i, (n, tab, o)) in [
        (15u32, 0x18bu32, 0u32),
        (16, 0x1a2, 0x14),
        (17, 0x1be, 0x28),
    ]
    .into_iter()
    .enumerate()
    {
        let r = st0_f32(g.call(t(g, tab, n), 0, &[a3]).st0);
        let x = g.read::<f32>(m.wrapping_add(o));
        let v = if i == 0 { mul(r, x) } else { mul(x, r) };
        g.write::<f32>(m.wrapping_add(o), v);
    }
    for (n, tab, at) in [
        (330u32, 0x1db, 0xc88u32),
        (328, 0x207, 0xc80),
        (329, 0x229, 0xc84),
    ] {
        let x = g.read::<u32>(r.wrapping_add(at));
        // call width: 3
        g.call(t(g, tab, n), 0, &[m, m, x]);
    }
    let a4 = target(g, 0x23d, 0x249);
    let ret = g.call(t(g, 0x24e, 257), 0, &[a4, 1, m]).eax;
    Ret::eax(ret)
}

/// [`summon_matrix_0610`] (622 bytes, 1 overlay).
pub const SUMMON_MATRIX_0610: Sig = Sig {
    len: 622,
    hash: 0x370e_d27f_d6d8_6eba,
    calls: &[
        // __security_check_cookie
        (0x266, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0705`'s per-frame step (0x100020c0, 618 bytes; cdecl, no
/// arguments; a cookie frame, `ebp = E - 4`, the matrix `M` at `ebp -
/// 0x54`, the point `V` at `ebp - 0x14`): nothing while `*G_ON` is 0 or
/// the time `*G_T` negative. Else two pointers get their words, slots 723
/// `(*G_A)` and 724 `(*G_B)`, the tint ([`tint`](crate::m0705::tint), the
/// colour 0.66 four times); past 0x50 of `*G_C` the fade of `*G_C - 0x50`
/// ([`fade`](crate::m0705::fade)); the part refreshes
/// ([`part_refresh_b`](crate::sa_misc::part_refresh_b)) of the two
/// objects and of the 16 parts from `P0` (0x80 apart; `done` while none
/// has its `u16 +0x1e` set, read after its refresh); slot 236 `(*G_D, 6,
/// M)`; for the 12 points from `Q0` (0x80 apart) whose `u16 -0x22` is
/// set, slot 229 `(Q, M, V)` with `V` the point (put back over slot 229's
/// output after the refresh of `Q - 0x40`). With `*G_E` set and `done`:
/// slot 213 `(*G_F, 0)`, slot 267 `(4)` and slot 713 for the two objects,
/// the 16 parts and the 12 points (each after `**G_P` = `W`), slot 214
/// `(*G_G)`. Then the glow ([`glow`](crate::m0705::glow)) and, unless
/// `**G_S` is set, the time one up. Returns 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn frame_0705<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let rd = |g: &G, o: u32| g.read::<u32>(w(o));
    let call = |o: u32| img.rel32_target(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let e = fr.esp;
    let (m, v) = (e.wrapping_sub(0x58), e.wrapping_sub(0x18));
    if rd(g, 0x12) == 0 || rd(g, 0x1f).cast_signed() < 0 {
        return Ret::eax(0);
    }
    let a = rd(g, 0x31);
    g.write::<u32>(rd(g, 0x2b), w(0x37));
    g.write::<u32>(rd(g, 0x3c), w(0x42));
    g.call(t(g, 0x47, 723), 0, &[a]);
    let b = rd(g, 0x5a);
    // call width: 2 (slot 723's word stays)
    g.call(t(g, 0x54, 724), 0, &[b, a]);
    // sub esp, 0x1c; the colour in words 3..6: the return address at E -
    // 0x80
    let esp = e.wrapping_sub(0x80);
    for i in 0..4u32 {
        g.write::<u32>(esp + 0x10 + 4 * i, 0x3f28_f5c3);
    }
    let tf = Frame {
        at: call(0x8a),
        esp,
        ..*fr
    };
    let mut eax = crate::m0705::tint(g, img, &tf);
    let c = rd(g, 0x90).wrapping_sub(0x50);
    if c.cast_signed() > 0 {
        let ff = Frame {
            at: call(0x9f),
            esp: e.wrapping_sub(0x5c),
            eax,
            ecx: c,
            ..*fr
        };
        eax = crate::m0705::fade(g, img, &ff);
    }
    let refresh = |g: &mut G, at: u32, obj: u32, eax: u32| {
        crate::sa_misc::part_refresh_b(g, img, call(at), obj, eax)
    };
    let eax = refresh(g, 0xab, w(0xa6), eax);
    refresh(g, 0xb5, w(0xb0), eax);
    let mut done = true;
    // (each loop runs once before its bound is compared, `jl`)
    let mut p = w(0xbf);
    loop {
        refresh(g, 0xc7, p.wrapping_sub(0x1e), 0);
        if g.read::<u16>(p) != 0 {
            done = false;
        }
        p = p.wrapping_add(0x80);
        if p.cast_signed() >= w(0xd8).cast_signed() {
            break;
        }
    }
    // call width: 3
    g.call(t(g, 0xe7, 236), 0, &[rd(g, 0xef), 6, m]);
    let mut q = w(0xff);
    for _ in 0..12 {
        if g.read::<u16>(q.wrapping_sub(0x22)) != 0 {
            for i in 0..4u32 {
                let x = g.read::<u32>(q + 4 * i);
                g.write::<u32>(v + 4 * i, x);
            }
            // call width: 3
            let r = g.call(t(g, 0x117, 229), 0, &[q, m, v]).eax;
            refresh(g, 0x13a, q.wrapping_sub(0x40), r);
            for i in 0..4u32 {
                let x = g.read::<u32>(v + 4 * i);
                g.write::<u32>(q + 4 * i, x);
            }
        }
        q = q.wrapping_add(0x80);
    }
    if rd(g, 0x159) != 0 && done {
        // call width: 2
        g.call(t(g, 0x16e, 213), 0, &[rd(g, 0x176), 0]);
        g.call(t(g, 0x183, 267), 0, &[4]);
        let off = |g: &mut G, tab: u32, p: u32, at: u32, obj: u32| {
            g.write::<u32>(rd(g, p), w(at));
            g.call(t(g, tab, 713), 0, &[obj]);
        };
        off(g, 0x1a2, 0x192, 0x19d, w(0x197));
        off(g, 0x1bf, 0x1af, 0x1ba, w(0x1b4));
        for (start, end, tab, p, at) in [
            (0x1cfu32, 0x1f4u32, 0x1e0u32, 0x1d4u32, 0x1dbu32),
            (0x1fb, 0x221, 0x20d, 0x201, 0x208),
        ] {
            let mut o = w(start);
            loop {
                off(g, tab, p, at, o);
                o = o.wrapping_add(0x80);
                if o.cast_signed() >= w(end).cast_signed() {
                    break;
                }
            }
        }
        g.call(t(g, 0x228, 214), 0, &[rd(g, 0x22e)]);
    }
    // sub esp, 0x10 (esi and edi saved): the return address at E - 0x74
    let gf = Frame {
        at: call(0x241),
        esp: e.wrapping_sub(0x74),
        ..*fr
    };
    crate::m0705::glow(g, img, &gf);
    if g.read::<u32>(rd(g, 0x246)) == 0 {
        let n = rd(g, 0x256);
        g.write::<u32>(w(0x256), n.wrapping_add(1));
    }
    Ret::eax(0)
}

/// [`frame_0705`] (618 bytes, 1 overlay).
pub const FRAME_0705: Sig = Sig {
    len: 618,
    hash: 0xbed2_ff4f_dd60_09e1,
    calls: &[
        (0x8a, Callee::Sig(&crate::m0705::TINT)),
        (0x9f, Callee::Sig(&crate::m0705::FADE)),
        (0xab, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0xb5, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0xc7, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0x13a, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0x241, Callee::Sig(&crate::m0705::GLOW)),
        // __security_check_cookie
        (0x262, Callee::Any),
    ],
    imports: &[],
};
