//! `magic_0390.dll`'s own functions (besides the runtime and the object
//! host, `l5_objhost`). `G_STATE` is the effect's state block (the
//! pointer at operand `+0x03` of [`e13f0`]), `G_H` the handle the slots
//! get (a word of the DLL's data).

use crate::l5_objhost::{ONE, run_objects};
use crate::runtime::slot_target;
use crate::sb_sse::{add, cvt, div, mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The x87 indefinite an `fstp` of an empty stack stores (a call-out
/// that left no float).
const INDEFINITE: f32 = f32::from_bits(0xffc0_0000);

/// A task step (stored at 0x100021ef, `(task)`): runs the objects
/// ([`run_objects`]); when they are all done and `G_STATE + 0xc7c` is 1,
/// slot 214 `(G_ARG)` and its result. Otherwise `run_objects`' result.
/// (`G_STATE` is read before the objects run.)
pub fn e13f0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let state = g.read::<u32>(w(0x03));
    // push esi; call
    let r = run_objects(g, img, img.rel32_target(f + 0x08), fr.esp - 8);
    if r != 1 || g.read::<u32>(state + 0xc7c) != 1 {
        return Ret::eax(r);
    }
    let s214 = slot_target(g, w(0x1a), 214);
    let arg = g.read::<u32>(w(0x20));
    Ret::eax(g.call(s214, state, &[arg]).eax)
}

/// 0x100013f0 (49 bytes, 1 overlay).
pub const E13F0: Sig = Sig {
    len: 49,
    hash: 0x4ffc_9c5f_dbae_fa74,
    calls: &[(0x08, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS))],
    imports: &[],
};

/// Table entry (0x100015b0, no arguments): slot 623 `(G_H, 0, 0, 0, 0,
/// 0, 0)` into `G_STATE + 0xc70`, returned.
pub fn e15b0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let s623 = slot_target(g, w(0x01), 623);
    let state = g.read::<u32>(w(0x0e));
    let h = g.read::<u32>(w(0x20));
    let r = g.call(s623, fr.ecx, &[h, 0, 0, 0, 0, 0, 0]).eax;
    g.write::<u32>(state + 0xc70, r);
    Ret::eax(r)
}

/// 0x100015b0 (49 bytes, 1 overlay).
pub const E15B0: Sig = Sig {
    len: 49,
    hash: 0x789d_e564_ce55_85d2,
    calls: &[],
    imports: &[],
};

/// An object's `+0x104` callback (stored at 0x1000198d, `(obj)`): slot
/// 237 `(G_H, 3, 0, obj + 0xd0)`, 1.0 into `obj + 0xdc`, slot 236 `(G_H,
/// 3, obj + 0x80)`, then the 3x3 part of the matrix at `obj + 0x80`
/// scaled by `K / s` (`s` slot 277's ST0 stored as `f32`, `K` the float
/// at operand `+0x70`), slot 235 `(m, G_H)` into the local matrix `m`
/// (`ebp - 0x44`), slot 330 `(m, m, pi/2)` and slot 231 `(obj + 0x80,
/// obj + 0x80, m)`; `obj + 0x104` cleared. Each call gets the words the
/// earlier ones left on the stack. Returns slot 231's result.
pub fn e15f0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let ebp = fr.esp - 4;
    let m = ebp - 0x44;
    let (rot, mat) = (obj + 0xd0, obj + 0x80);
    let s237 = slot_target(g, w(0x11), 237);
    let h = g.read::<u32>(w(0x2d));
    let mut stack = vec![h, 3, 0, rot];
    g.call(s237, fr.ecx, &stack);
    let s236 = slot_target(g, w(0x34), 236);
    g.write::<u32>(rot + 0xc, ONE);
    let h = g.read::<u32>(w(0x50));
    stack.splice(0..0, [h, 3, mat]);
    // call width: 7 (three words and the first call's four)
    g.call(s236, fr.ecx, &stack);
    let s277 = slot_target(g, w(0x57), 277);
    let h = g.read::<u32>(w(0x5d));
    stack.insert(0, h);
    // call width: 8
    let s = g
        .call(s277, fr.ecx, &stack)
        .st0
        .map_or(INDEFINITE, |v| v as f32);
    let k = div(g.read::<f32>(w(0x70)), s);
    for i in [0u32, 1, 2, 4, 5, 6, 8, 9, 10] {
        // the third: movaps xmm0, xmm1; mulss xmm0, [esi + 8]
        let m = g.read::<f32>(mat + 4 * i);
        let v = if i == 2 { mul(k, m) } else { mul(m, k) };
        g.write::<f32>(mat + 4 * i, v);
    }
    let s235 = slot_target(g, w(0x93), 235);
    stack.insert(0, m);
    // call width: 9
    g.call(s235, fr.ecx, &stack);
    // add esp, 0x20 leaves `m`, overwritten with pi/2
    let s330 = slot_target(g, w(0x115), 330);
    g.call(s330, fr.ecx, &[m, m, 0x3fc9_0fdb]);
    let s231 = slot_target(g, w(0x126), 231);
    let r = g.call(s231, fr.ecx, &[mat, mat, m, m, m, 0x3fc9_0fdb]).eax;
    g.write::<u32>(obj + 0x104, 0);
    Ret::eax(r)
}

/// 0x100015f0 (337 bytes, 1 overlay).
pub const E15F0: Sig = Sig {
    len: 337,
    hash: 0x2b60_af95_bf8f_f198,
    // __security_check_cookie
    calls: &[(0x149, Callee::Any)],
    imports: &[],
};

/// Table entry (0x10001be0, no arguments): the countdown `G_STATE +
/// 0xc6c` goes down by `*G_DT` (when positive; clamped at 0), `t =
/// n / N`, `u = 1 - t`, and a point on two cubic curves (Bernstein
/// weights, the middle control points from the ends' differences) from
/// `+0xab0` to `+0xac0` (x) and `+0xab8` to `+0xac8` (z); y is `+0xab4`,
/// plus a hump `u t C * +0xc64` while `n < 0x17`. With `G_FLAG` clear
/// and `G_INDEX < 0x20` the point goes to slot 255 `(G_INDEX, &p)` (a
/// local); otherwise slot 11 `(G_H)` picks slot 33 `(G_H, x, y, z)` (0)
/// or slot 32 `(G_H, x, z)`. Returns the last call's result.
#[allow(clippy::similar_names)]
pub fn e1be0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let k = |g: &G, off: u32| g.read::<f32>(w(off));
    let state = g.read::<u32>(w(0x12));
    let n = g.read::<u32>(state + 0xc6c).cast_signed();
    if n > 0 {
        let dt = g.read::<u32>(g.read::<u32>(w(0x21))).cast_signed();
        g.write::<u32>(state + 0xc6c, n.wrapping_sub(dt).cast_unsigned());
    }
    if g.read::<u32>(state + 0xc6c).cast_signed() < 0 {
        g.write::<u32>(state + 0xc6c, 0);
    }
    let n = g.read::<u32>(state + 0xc6c).cast_signed();
    let (c20, c1c, c38) = (k(g, 0xa6), k(g, 0xdd), k(g, 0x75));
    let d28 = f64::from_bits(g.read::<u64>(w(0xf0)));
    let t = div(cvt(n.cast_unsigned()), k(g, 0x59));
    let u = sub(k(g, 0x51), t);
    let ut = mul(u, t);
    let uu = mul(u, u);
    let tt = mul(t, t);
    // cvtps2pd; mulsd; cvtpd2ps (one NaN at most: the constant is a number)
    let rd = |v: f32| (f64::from(v) * d28) as f32;
    // x
    let d = sub(g.read::<f32>(state + 0xac0), g.read::<f32>(state + 0xab0));
    let mut a = mul(mul(mul(ut, uu), c38), mul(d, c20));
    a = add(a, mul(mul(uu, uu), d));
    a = add(a, mul(mul(mul(tt, uu), c38), mul(d, c1c)));
    a = add(a, mul(mul(mul(tt, ut), c38), rd(d)));
    a = add(a, mul(mul(tt, tt), 0.0));
    let x = add(a, g.read::<f32>(state + 0xab0));
    // z
    let z0 = g.read::<f32>(state + 0xab8);
    let d = sub(g.read::<f32>(state + 0xac8), z0);
    let ut = mul(u, t);
    let mut b = mul(mul(mul(ut, uu), c38), mul(d, c20));
    b = add(b, mul(mul(uu, uu), d));
    b = add(b, mul(mul(mul(tt, uu), c38), mul(d, c1c)));
    b = add(b, mul(mul(mul(tt, ut), c38), rd(d)));
    b = add(b, mul(mul(tt, tt), 0.0));
    let z = add(b, z0);
    let mut y = g.read::<f32>(state + 0xab4);
    if n < 0x17 {
        let t = div(cvt(n.cast_unsigned()), k(g, 0x1fc));
        let u = sub(k(g, 0x1f4), t);
        let mut v = mul(mul(mul(u, t), k(g, 0x213)), g.read::<f32>(state + 0xc64));
        v = add(v, mul(mul(u, u), 0.0));
        v = add(v, mul(mul(t, t), 0.0));
        y = add(v, y);
    }
    let p = fr.esp - 4 - 0x14;
    g.write::<f32>(p, x);
    g.write::<f32>(p + 4, y);
    g.write::<f32>(p + 8, z);
    let index = g.read::<u32>(w(0x246));
    if g.read::<u32>(w(0x23e)) == 0 && index.cast_signed() < 0x20 {
        let s255 = slot_target(g, w(0x255), 255);
        return Ret::eax(g.call(s255, p, &[index, p]).eax);
    }
    let s11 = slot_target(g, w(0x273), 11);
    let h = g.read::<u32>(w(0x27a));
    let (x, y, z) = (x.to_bits(), y.to_bits(), z.to_bits());
    if g.call(s11, state, &[h]).eax == 0 {
        let s33 = slot_target(g, w(0x28f), 33);
        Ret::eax(g.call(s33, state, &[h, x, y, z]).eax)
    } else {
        let s32 = slot_target(g, w(0x28f), 32);
        Ret::eax(g.call(s32, state, &[h, x, z]).eax)
    }
}

/// 0x10001be0 (764 bytes, 1 overlay).
pub const E1BE0: Sig = Sig {
    len: 764,
    hash: 0xc89b_701e_aac5_dcab,
    // __security_check_cookie
    calls: &[
        (0x26a, Callee::Any),
        (0x2c6, Callee::Any),
        (0x2f4, Callee::Any),
    ],
    imports: &[],
};

/// Table entry (0x10002100, no arguments): slot 624 `(G_STATE + 0xc70)`,
/// then `G_STATE + 0xc70 = -1`. Returns slot 624's result.
pub fn e2100<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let s624 = slot_target(g, w(0x01), 624);
    let state = g.read::<u32>(w(0x08));
    let v = g.read::<u32>(state + 0xc70);
    let r = g.call(s624, fr.ecx, &[v]).eax;
    g.write::<u32>(state + 0xc70, 0xffff_ffff);
    Ret::eax(r)
}

/// 0x10002100 (41 bytes, 1 overlay).
pub const E2100: Sig = Sig {
    len: 41,
    hash: 0x5e75_d4d6_6962_683f,
    calls: &[],
    imports: &[],
};

/// A task step (the owner value of the start-up's first task node,
/// 0x100013b0, cdecl `(node)`): the script ([`run`](crate::sa_script::run))
/// and the motion ([`motion_step`](crate::s3_motion::motion_step)); once
/// both report done (non-zero), `G_STATE + 0xc7c` = 1, the colours reset
/// ([`colour_one`](crate::s4_small::colour_one)) and the node goes back
/// to its pool ([`give_back`](crate::sa_pool::give_back), a tail jump
/// with `ecx` = `node`; the flag's offset is the code's displacement). Returns the motion's result, or the give-back's.
/// (`G_STATE` is read before the script runs.)
pub fn task_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let state = g.read::<u32>(img.word(f + 0x07));
    let script = crate::sa_script::run(g, img, img.rel32_target(f + 0x0c));
    let motion = crate::s3_motion::motion_step(g, img, img.rel32_target(f + 0x13));
    if script == 0 || motion == 0 {
        return Ret::eax(motion);
    }
    // the field is a displacement (`magic_0447`'s copy has another)
    g.write::<u32>(state.wrapping_add(img.word(f + 0x21)), 1);
    // push ebp; push esi; push edi; call
    let colour = Frame {
        at: img.rel32_target(f + 0x2a),
        esp: fr.esp.wrapping_sub(0x10),
        ..*fr
    };
    crate::s4_small::colour_one(g, img, &colour);
    let node = fr.arg(g, 0);
    Ret::eax(crate::sa_pool::give_back(
        g,
        img,
        img.rel32_target(f + 0x35),
        node,
    ))
}

/// [`task_step`] (61 bytes, 1 overlay).
pub const TASK_STEP: Sig = Sig {
    len: 61,
    hash: 0x8169_68e4_7496_4946,
    calls: &[
        (0x0c, Callee::Sig(&crate::sa_script::RUN)),
        (0x13, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0x2a, Callee::Sig(&crate::s4_small::COLOUR_ONE)),
        (0x35, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
    ],
    imports: &[],
};

/// [`task_step`] in `magic_0447` (0x10004ab0; the done flag `+0x7d8`).
pub const TASK_STEP_0447: Sig = Sig {
    len: 61,
    hash: 0x95ff_ea7a_4f59_052f,
    calls: TASK_STEP.calls,
    imports: &[],
};

/// The effect's facing (0x10001430, internal, no arguments; `esp` the
/// entry's; the block's fields are the code's displacements, given here
/// as `magic_0390`'s): slot 262 `(*G_H2, B, B + 4, B + 8)` gives the point `B` =
/// `G_STATE + 0xab0` (`B + 0xc` = 1.0), slot 333 `(*G_H2)`'s float `v`
/// (ST0, as `f32`) turns into `K - v` wrapped once into `(LO, HI]` by
/// `T` ([`wrap`](crate::m0018::wrap); operands `+0x4f`, `+0x5e`, `+0x72`,
/// `+0x68`/`+0x7f`) in `G_STATE + 0xc68`; slot 622 `(*G_H, *G_SEL's
/// byte)` into `+0xc78`; then the state task `G_STATE + 0xbe0`
/// ([`state_task`](crate::s4_misc::state_task), the stack word the
/// immediate `+0xb1`); `+0xc28` and `+0xc70` -1, `+0xc7c` 0 and `G_DONE`
/// (`+0xe4`) 1. Each slot call passes the words the original leaves on
/// the stack. Returns the state task's node.
pub fn facing<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let state = g.read::<u32>(w(0x08));
    // the block's fields are displacements in the code (`magic_0447`'s
    // copy has others)
    let b = state.wrapping_add(w(0x0e));
    let mut s = crate::pushes::Pushes::default();
    let h2 = g.read::<u32>(w(0x22));
    s.push_all(&[b + 8, b + 4, b, h2]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x1b), 0x418 / 4), 0);
    g.write::<u32>(b + 0xc, ONE);
    s.push(g.read::<u32>(w(0x3c)));
    // call width: 12 (Pushes caps the words at 12)
    let v = crate::m0018::st0_f32(s.call(g, slot_target(g, w(0x2f), 0x534 / 4), 0).st0);
    let x = sub(k(g, 0x4f), v);
    // comiss + jbe twice: an unordered value stays
    let x = if x > k(g, 0x5e) {
        sub(x, k(g, 0x68))
    } else if k(g, 0x72) > x {
        add(x, k(g, 0x7f))
    } else {
        x
    };
    g.write::<f32>(state.wrapping_add(w(0x87)), x);
    let sel = u32::from(g.read::<u8>(g.read::<u32>(w(0x8c))));
    let h = g.read::<u32>(w(0x9b));
    let r = g.call(slot_target(g, w(0x95), 0x9b8 / 4), 0, &[h, sel]).eax;
    // push ebp; push ecx; push esi; push edi; the slot's `sel` stays
    // under the pushed immediate; call
    let task = Frame {
        at: img.rel32_target(f + 0xbc),
        esp: esp.wrapping_sub(0x1c),
        ecx: state.wrapping_add(w(0xac)),
        ..Frame::default()
    };
    g.write::<u32>(task.esp + 4, w(0xb1));
    g.write::<u32>(task.esp + 8, sel);
    g.write::<u32>(state.wrapping_add(w(0xb7)), r);
    let node = crate::s4_misc::state_task(g, img, &task).eax;
    g.write::<u32>(state.wrapping_add(w(0xc2)), u32::MAX);
    g.write::<u32>(state.wrapping_add(w(0xcf)), u32::MAX);
    g.write::<u32>(state.wrapping_add(w(0xd9)), 0);
    g.write::<u32>(w(0xe4), 1);
    node
}

/// [`facing`] (241 bytes, 1 overlay).
pub const FACING: Sig = Sig {
    len: 241,
    hash: 0x3ad5_fc98_9414_3a37,
    calls: &[(0xbc, Callee::Sig(&crate::s4_misc::STATE_TASK))],
    imports: &[],
};

/// [`facing`] in `magic_0447` (0x10004b30; the block's fields at `+0x5f0`,
/// `+0x7c4`, `+0x740`, `+0x7d4`, `+0x788`, `+0x7d0`, `+0x7d8`).
pub const FACING_0447: Sig = Sig {
    len: 241,
    hash: 0x8920_cf59_a08e_a7db,
    calls: FACING.calls,
    imports: &[],
};

/// The start-up (0x10002130, internal, no arguments; the load step's
/// callee): `G_PART` (`+0x18`) 0, two block pointers (`+0x22`/`+0x26`,
/// `+0x2c`/`+0x30`), slot 719 `(NAME, 8, 0)` (`+0x12`) into `+0x3a`; the
/// particle set-up ([`part_setup`](crate::sb_misc::part_setup), the
/// stack word the immediate `+0x3f`) and the facing ([`facing`]); the
/// callback word `+0x54` = the immediate `+0x58`, `+0x5e` 0, the
/// halfword `+0x69` 0, the byte `+0x71` 1; slot 271 `(4)`; two pool nodes
/// ([`take`](crate::sa_pool::take), owners [`task_step`] `+0x84` and the
/// objects' step `+0xc0`, kinds 1 and 2 at `+0x18`) go last on the task
/// list (first `+0x8f`, last `+0xa6`); three floats 1.0 (`+0xfc`,
/// `+0x10c`, `+0x116`) and slot 790's float into `+0x122`. Returns 0.
/// The slot calls pass the words the original leaves on the stack.
pub fn start_up<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let (first, second, held) = start_common(g, img, fr);
    if held == 0 {
        // only with no first node (an empty pool)
        g.write::<u32>(second, 0);
        g.write::<u32>(w(0xe9), second);
    } else {
        g.write::<u32>(second, first);
        g.write::<u32>(first + 4, second);
    }
    g.write::<u32>(second + 8, second);
    g.write::<u32>(w(0xf1), second);
    let s790 = slot_target(g, w(0xf6), 0xc58 / 4);
    for off in [0xfc, 0x10c, 0x116] {
        g.write::<u32>(w(off), ONE);
    }
    let v = crate::m0018::st0_f32(g.call(s790, 0, &[]).st0);
    g.write::<f32>(w(0x122), v);
    Ret::eax(0)
}

/// [`start_up`] and [`start_up_0447`] up to the second task node:
/// returns the two nodes and the list's head as the second's append
/// sees it (the old head, or the first node when the list was empty).
fn start_common<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> (u32, u32, u32) {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let s719 = slot_target(g, w(0x01), 0xb3c / 4);
    let mut s = crate::pushes::Pushes::default();
    s.push_all(&[0, 8, w(0x12)]);
    g.write::<u32>(w(0x18), 0);
    g.write::<u32>(w(0x22), w(0x26));
    g.write::<u32>(w(0x2c), w(0x30));
    // call width: 12 (Pushes caps the words at 12)
    let r = s.call(g, s719, 0).eax;
    s.pop(2);
    g.write::<u32>(w(0x3a), r);
    s.push(w(0x3f));
    // push esi; push edi; slot 719's last word and the pushed block
    // stay; call
    let esp = fr.esp.wrapping_sub(0x14);
    g.write::<u32>(esp + 4, w(0x3f));
    g.write::<u32>(esp + 8, 0);
    crate::sb_misc::part_setup(g, img, img.rel32_target(f + 0x44), esp);
    facing(g, img, img.rel32_target(f + 0x49), esp);
    let s271 = slot_target(g, w(0x4e), 0x43c / 4);
    g.write::<u32>(w(0x54), w(0x58));
    g.write::<u32>(w(0x5e), 0);
    g.write::<u16>(w(0x69), 0);
    g.write::<u8>(w(0x71), 1);
    s.push(4);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, s271, 0);
    let first = crate::sa_pool::take(g, img, img.rel32_target(f + 0x89), w(0x84));
    let head = g.read::<u32>(w(0x8f));
    g.write::<u8>(first + 0x18, 1);
    g.write::<u32>(first + 4, 0);
    let after_first = if head == 0 {
        g.write::<u32>(first, 0);
        g.write::<u32>(w(0xbb), first);
        first
    } else {
        let last = g.read::<u32>(w(0xa6));
        g.write::<u32>(first, last);
        g.write::<u32>(last + 4, first);
        head
    };
    g.write::<u32>(first + 8, first);
    let second = crate::sa_pool::take(g, img, img.rel32_target(f + 0xc8), w(0xc0));
    g.write::<u8>(second + 0x18, 2);
    g.write::<u32>(second + 4, 0);
    (first, second, after_first)
}

/// `magic_0447`'s start-up (0x100054e0, internal, no arguments):
/// [`start_up`] without the floats at the end; the second node becomes
/// the list's last (`+0xe5`, or with no first node head `+0xf5` and last
/// `+0xfd`). Returns 0.
pub fn start_up_0447<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |o: u32| img.word(fr.at + o);
    let (first, second, held) = start_common(g, img, fr);
    if held == 0 {
        // only with no first node (an empty pool)
        g.write::<u32>(second, 0);
        g.write::<u32>(w(0xf5), second);
        g.write::<u32>(second + 8, second);
        g.write::<u32>(w(0xfd), second);
    } else {
        g.write::<u32>(second, first);
        g.write::<u32>(first + 4, second);
        g.write::<u32>(second + 8, second);
        g.write::<u32>(w(0xe5), second);
    }
    Ret::eax(0)
}

/// [`start_up_0447`] (261 bytes, 1 overlay).
pub const START_UP_0447: Sig = Sig {
    len: 261,
    hash: 0x9d76_2e58_703a_fee0,
    calls: &[
        (0x44, Callee::Sig(&crate::sb_misc::PART_SETUP)),
        (0x49, Callee::Sig(&FACING_0447)),
        (0x89, Callee::Sig(&crate::sa_pool::TAKE)),
        (0xc8, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// [`start_up`] (299 bytes, 1 overlay).
pub const START_UP: Sig = Sig {
    len: 299,
    hash: 0x8857_2514_cd64_26ce,
    calls: &[
        (0x44, Callee::Sig(&crate::sb_misc::PART_SETUP)),
        (0x49, Callee::Sig(&FACING)),
        (0x89, Callee::Sig(&crate::sa_pool::TAKE)),
        (0xc8, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// Table entry (0x10001ef0, no arguments): object 0 (`G_STATE + 0`) when
/// `G_STATE + 0xc78` is set, else object 4 (`+ 4 * 0x130`), starts
/// ([`start_object_owners`](crate::s4_misc::start_object_owners): scale
/// `*K_S` `+0x32`, `(index, 0, 2, FINISH)`, `FINISH` the immediate
/// `+0x42`); the character `CHR[*G_SEL's byte]` (`+0x6b`, `+0x58`) gives
/// the object's point `+0xd0` (slot 262 `(CHR, p, p + 4, p + 8)`), slot
/// 268 `(CHR, 0x10, p)` and slot 12 `(CHR)`'s float into `p + 4`, `p +
/// 0xc` 1.0; `obj + 0xc4` = `G_STATE + 0xc68`; `p + 4` becomes
/// `G_STATE + 0xab4` when slot 520 says 0x18b0046, minus `K` (`+0xe7`)
/// either way. The record `G_STATE + 0xc30` (`+0x10..+0x1c`, `+0x28`,
/// `+0x2c`, halfword `+0x30` 0; `+0x20` the immediate `+0x11e`; `+0x24`
/// the callback `+0x1c` or `+0x2a`) goes into `obj + 0x110`; the colour
/// block `G_STATE + 0xad0` is set (`+0x80` 0x80808080, `+0x84`
/// 0x808080, `+0x88`/`+0x94` 0, `+0x8c`/`+0x90` 0x14, halfword `+0x98`
/// 0x48) and handed to a task node ([`take`](crate::sa_pool::take),
/// owner `+0x14f`) appended to the task list (first `+0x18b`, last
/// `+0x19e`); slot 538 `(*G_X, G_STATE + 0xc74)`, then the fade task
/// `G_STATE + 0xba8` ([`fade_task`](crate::s3_misc::fade_task): `edx`
/// 0x1e, `xmm1`/`xmm2` the floats `+0x1e1`/`+0x1d9`, the stack word
/// 0x23). Returns the fade task's node. Each slot call passes the words
/// the original leaves on the stack.
pub fn e1ef0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let state = g.read::<u32>(w(0x07));
    let (index, callback) = if g.read::<u32>(state + 0xc78) == 0 {
        (4u32, w(0x2a))
    } else {
        (0, w(0x1c))
    };
    let obj = state.wrapping_add(index * 0x130);
    let mut s = crate::pushes::Pushes::default();
    s.push_all(&[w(0x42), 2, 0, index]);
    crate::s4_misc::start_object_owners(
        g,
        img,
        img.rel32_target(f + 0x53),
        obj,
        k(g, 0x32),
        index,
        0,
        (2, w(0x42)),
    );
    let sel = u32::from(g.read::<u8>(g.read::<u32>(w(0x58))));
    let p = obj + 0xd0;
    let chr = g.read::<u32>(w(0x6b).wrapping_add(4 * sel));
    s.push_all(&[p + 8, p + 4, p, chr]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x74), 0x418 / 4), 0);
    s.push_all(&[p, 0x10, chr]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x84), 0x430 / 4), 0);
    s.push(chr);
    // call width: 12 (Pushes caps the words at 12)
    let y = crate::m0018::st0_f32(s.call(g, slot_target(g, w(0x95), 0x30 / 4), 0).st0);
    g.write::<f32>(p + 4, y);
    g.write::<u32>(p + 0xc, ONE);
    let v = g.read::<u32>(state + 0xc68);
    g.write::<u32>(obj + 0xc4, v);
    let mode = g.call(slot_target(g, w(0xbe), 0x820 / 4), 0, &[]).eax;
    let y = if mode == 0x018b_0046 {
        g.read::<f32>(state + 0xab4)
    } else {
        g.read::<f32>(obj + 0xd4)
    };
    g.write::<f32>(obj + 0xd4, sub(y, k(g, 0xe7)));
    let rec = state + 0xc30;
    for o in [0x18, 0x14, 0x10, 0x1c] {
        g.write::<u32>(rec + o, 0);
    }
    g.write::<u32>(rec + 0x24, callback);
    g.write::<u32>(rec + 0x20, w(0x11e));
    g.write::<u32>(rec + 0x28, 0);
    g.write::<u32>(rec + 0x2c, 0);
    g.write::<u16>(rec + 0x30, 0);
    g.write::<u32>(obj + 0x110, rec);
    let colour = state + 0xad0;
    g.write::<u32>(colour + 0x88, 0);
    g.write::<u32>(colour + 0x94, 0);
    g.write::<u32>(colour + 0x80, 0x8080_8080);
    g.write::<u32>(colour + 0x84, 0x0080_8080);
    g.write::<u16>(colour + 0x98, 0x48);
    g.write::<u32>(colour + 0x90, 0x14);
    g.write::<u32>(colour + 0x8c, 0x14);
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x185), w(0x14f));
    let head = g.read::<u32>(w(0x18b));
    g.write::<u32>(node + 0x14, colour);
    g.write::<u32>(node + 4, 0);
    if head == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x1b0), node);
    } else {
        let last = g.read::<u32>(w(0x19e));
        g.write::<u32>(node, last);
        g.write::<u32>(last + 4, node);
    }
    g.write::<u32>(node + 8, node);
    let c74 = g.read::<u32>(state + 0xc74);
    g.write::<u32>(w(0x1be), node);
    let x = g.read::<u32>(w(0x1c9));
    g.call(slot_target(g, w(0x1c3), 0x868 / 4), 0, &[x, c74]);
    // push ebp; sub esp, 0xc; push ebx/esi/edi; the slot's two words
    // under the pushed 0x23; call
    let fade = Frame {
        at: img.rel32_target(f + 0x1f3),
        esp: fr.esp.wrapping_sub(0x2c),
        ecx: state + 0xba8,
        edx: 0x1e,
        xmm: [
            0,
            u128::from(g.read::<u32>(w(0x1e1))),
            u128::from(g.read::<u32>(w(0x1d9))),
            0,
        ],
        ..*fr
    };
    g.write::<u32>(fade.esp + 4, 0x23);
    g.write::<u32>(fade.esp + 8, x);
    g.write::<u32>(fade.esp + 12, c74);
    Ret::eax(crate::s3_misc::fade_task(g, img, &fade))
}

/// [`e1ef0`] (513 bytes, 1 overlay).
pub const E1EF0: Sig = Sig {
    len: 513,
    hash: 0x4da2_3489_7a69_a7c2,
    calls: &[
        (0x53, Callee::Sig(&crate::s4_misc::START_OBJECT_OWNERS)),
        (0x185, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x1f3, Callee::Sig(&crate::s3_misc::FADE_TASK)),
    ],
    imports: &[],
};

/// Table entry (0x10001750, no arguments; an 8-aligned /GS frame): the
/// character `CHR[*G_SEL's byte]` (`+0x2d`, `+0x1b`) gives the point `B`
/// = `G_STATE + 0xac0` (slot 262 `(CHR, B, B + 4, B + 8)`; slot 12's
/// float into `B + 4`, `B + 0xc` 1.0), slot 333 `(*G_H2)`'s float `v`
/// the yaw `K - v` wrapped once ([`facing`]'s wrap; operands `+0x88`,
/// `+0x98`, `+0xb2`, `+0xa8`/`+0xbf`). A height `h`: slot 251 `(sel)`'s
/// float when `*G_FLAG` (`+0xcb`) is 0 and `sel < 0x20`, else slot 17
/// `(C)` times slot 19 `(C)` (`C` = `CHR2[sel]`, `+0x113`); plus `K5`
/// (`+0x148`). The local matrix `M` (slot 235), turned by slot 328 `(M,
/// M, 0)` and slot 329 `(M, M, yaw)`, takes the local `V = (0, 0, h,
/// 1)` to `O` (slot 229 `(O, M, V)`), and slot 248 `(B, B, O)` moves
/// `B` (`B + 0xc` 1.0 again). `G_STATE + 0xc6c` = 0x32; four objects
/// `G_STATE + 0x5f0 + i * 0x130` start
/// ([`start_object_owners`](crate::s4_misc::start_object_owners): scale
/// `(slot 314 (0.2) + K6) * K7`, `(2, 0x1c + 2i, 0, 0)`) with the
/// callback `+0x104` = [`e15f0`] and byte `+0x126` 2. Slot 538 `(*G_X,
/// 0)` into `G_STATE + 0xc74`; slot 262 `(*G_H2, Q)` and slot 12's
/// float into `Q.y`; `d` = the square root of `dz * dz + dx * dx` (`Q -
/// B`) sets `G_STATE + 0xc64`: 0 up to `D0` (`+0x2ec`), 30.0 from `D1`
/// (`+0x300`), between them `(1 - t) * 0 + t * K10` with `t = (d - D0) /
/// D0`. Then the colour block `G_STATE + 0xb70` (eight floats 1.0/0.5,
/// `+0x28`/`+0x34` 0, `+0x2c`/`+0x30` 0x14, `*G_A` 1.0, `*G_B` 0.5) goes
/// to a node taken inline from the free ring (head `+0x3cf`, count
/// `u16 +0x353`; owner the immediate `+0x425`, byte `+0x18` 1) appended
/// to the task list (first `+0x436`, last `+0x44c`). Returns the old
/// last node, or 0xffff with an empty list. Each slot call passes the
/// words the original leaves on the stack.
#[allow(clippy::too_many_lines)]
pub fn e1750<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    // push ebp; and esp, -8; sub esp, 0x8c; push ebx/esi/edi
    let s0 = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x8c);
    let (q, v, o, m) = (s0 + 0x14, s0 + 0x24, s0 + 0x34, s0 + 0x44);
    let sel_p = g.read::<u32>(w(0x1b));
    let state = g.read::<u32>(w(0x25));
    let chr = g.read::<u32>(w(0x2d).wrapping_add(4 * u32::from(g.read::<u8>(sel_p))));
    let b = state + 0xac0;
    let mut s = crate::pushes::Pushes::default();
    s.push_all(&[b + 8, b + 4, b, chr]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x3d), 0x418 / 4), 0);
    s.push(chr);
    // call width: 12 (Pushes caps the words at 12)
    let y = crate::m0018::st0_f32(s.call(g, slot_target(g, w(0x54), 0x30 / 4), 0).st0);
    g.write::<f32>(b + 4, y);
    g.write::<u32>(b + 0xc, ONE);
    s.push(g.read::<u32>(w(0x74)));
    // call width: 12 (Pushes caps the words at 12)
    let a = crate::m0018::st0_f32(s.call(g, slot_target(g, w(0x5f), 0x534 / 4), 0).st0);
    s.pop(6);
    let x = sub(k(g, 0x88), a);
    let yaw = if x > k(g, 0x98) {
        sub(x, k(g, 0xa8))
    } else if k(g, 0xb2) > x {
        add(x, k(g, 0xbf))
    } else {
        x
    };
    let flag = g.read::<u32>(w(0xcb));
    let sel_p = g.read::<u32>(w(0xd2));
    g.write::<u32>(v, 0);
    g.write::<u32>(v + 4, 0);
    let sel = u32::from(g.read::<u8>(sel_p));
    let h = if flag == 0 && sel < 0x20 {
        s.push(sel);
        // call width: 12 (Pushes caps the words at 12)
        let r = crate::m0018::st0_f32(s.call(g, slot_target(g, w(0xf2), 0x3ec / 4), 0).st0);
        s.pop(1);
        r
    } else {
        let c = g.read::<u32>(w(0x113).wrapping_add(4 * sel));
        s.push(c);
        // call width: 12 (Pushes caps the words at 12)
        let a = crate::m0018::st0_f32(s.call(g, slot_target(g, w(0x118), 0x4c / 4), 0).st0);
        s.push(c);
        // call width: 12 (Pushes caps the words at 12)
        let b = crate::m0018::st0_f32(s.call(g, slot_target(g, w(0x127), 0x44 / 4), 0).st0);
        s.pop(2);
        mul(b, a)
    };
    let h = add(h, k(g, 0x148));
    s.push(m);
    g.write::<f32>(v + 8, h);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x152), 0x3ac / 4), 0);
    s.set_top(0);
    s.push_all(&[m, m]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x172), 0x520 / 4), 0);
    s.pop(2);
    s.set_top(yaw.to_bits());
    s.push_all(&[m, m]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x193), 0x524 / 4), 0);
    s.push_all(&[v, m, o]);
    g.write::<u32>(v + 0xc, ONE);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x1af), 0x394 / 4), 0);
    s.push_all(&[o, b, b]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x1c9), 0x3e0 / 4), 0);
    g.write::<u32>(b + 0xc, ONE);
    g.write::<u32>(state + 0xc6c, 0x32);
    s.pop(9);
    let mut obj = state + 0x5f0;
    for owner in [0x1cu32, 0x1e, 0x20, 0x22] {
        // push ecx; mov [esp], 0.2
        s.push(0x3e4c_cccd);
        // call width: 12 (Pushes caps the words at 12)
        let r = crate::m0018::st0_f32(s.call(g, slot_target(g, w(0x201), 0x4e8 / 4), 0).st0);
        let scale = mul(add(r, k(g, 0x223)), k(g, 0x232));
        crate::s4_misc::start_object_owners(
            g,
            img,
            img.rel32_target(f + 0x239),
            obj,
            scale,
            2,
            owner,
            (0, 0),
        );
        g.write::<u32>(obj + 0x104, w(0x243));
        g.write::<u8>(obj + 0x126, 2);
        s.pop(1);
        obj += 0x130;
    }
    let x = g.read::<u32>(w(0x265));
    let r = g.call(slot_target(g, w(0x25e), 0x868 / 4), 0, &[x, 0]).eax;
    g.write::<u32>(state + 0xc74, r);
    let h2 = g.read::<u32>(w(0x27d));
    s.push_all(&[0, x, q + 8, q + 4, q, h2]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x291), 0x418 / 4), 0);
    s.push(h2);
    // call width: 12 (Pushes caps the words at 12)
    let out = s.call(g, slot_target(g, w(0x29f), 0x30 / 4), 0);
    g.write::<f32>(q + 4, crate::m0018::st0_f32(out.st0));
    let dx = sub(g.read::<f32>(q), g.read::<f32>(b));
    let dz = sub(g.read::<f32>(q + 8), g.read::<f32>(b + 8));
    g.write::<u32>(q + 0xc, ONE);
    let d = f64::from(add(mul(dz, dz), mul(dx, dx))).sqrt() as f32;
    let d0 = k(g, 0x2ec);
    // comiss + jbe twice: unordered goes to 0, then to 30.0
    let c64 = if d > d0 {
        if k(g, 0x300) > d {
            let t = div(sub(d, d0), d0);
            let u = sub(k(g, 0x311), t);
            add(mul(u, 0.0), mul(t, k(g, 0x324))).to_bits()
        } else {
            0x41f0_0000
        }
    } else {
        0
    };
    g.write::<u32>(state + 0xc64, c64);
    // the colour block and its task node (the pool's take inlined)
    let free = g.read::<u16>(w(0x353));
    let block = state + 0xb70;
    g.write::<u32>(w(0x35f), ONE);
    g.write::<u32>(w(0x369), 0x3f00_0000);
    for (off, val) in [
        (8u32, ONE),
        (4, ONE),
        (0, ONE),
        (0xc, ONE),
        (0x18, 0x3f00_0000),
        (0x14, 0x3f00_0000),
        (0x10, 0x3f00_0000),
        (0x1c, ONE),
        (0x28, 0),
        (0x34, 0),
        (0x30, 0x14),
        (0x2c, 0x14),
    ] {
        g.write::<u32>(block + off, val);
    }
    let mut eax = 0xffff;
    let node = if free == 0 {
        // no node left: the original goes on with node 0 (and faults);
        // `eax` is still slot 12's
        eax = out.eax;
        0
    } else {
        let n = g.read::<u32>(w(0x3cf));
        let (next, prev) = (g.read::<u32>(n), g.read::<u32>(n + 4));
        g.write::<u32>(next + 4, prev);
        let (prev, next) = (g.read::<u32>(n + 4), g.read::<u32>(n));
        g.write::<u32>(prev, next);
        let head = if free > 1 { g.read::<u32>(n + 4) } else { 0 };
        g.write::<u32>(w(0x3ec), head);
        for off in [0u32, 4, 0xc, 0x14] {
            g.write::<u32>(n + off, 0);
        }
        g.write::<u32>(n + 8, n);
        g.write::<u32>(n + 0x10, w(0x425));
        g.write::<u8>(n + 0x18, 0);
        g.write::<u16>(w(0x430), free - 1);
        n
    };
    let first = g.read::<u32>(w(0x436));
    g.write::<u8>(node.wrapping_add(0x18), 1);
    g.write::<u32>(node.wrapping_add(0x14), block);
    g.write::<u32>(node.wrapping_add(4), 0);
    if first == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x45f), node);
    } else {
        let last = g.read::<u32>(w(0x44c));
        g.write::<u32>(node, last);
        g.write::<u32>(last + 4, node);
        eax = last;
    }
    g.write::<u32>(w(0x471), node);
    g.write::<u32>(node.wrapping_add(8), node);
    Ret::eax(eax)
}

/// [`e1750`] (1153 bytes, 1 overlay).
pub const E1750: Sig = Sig {
    len: 1153,
    hash: 0xefae_1c7a_acf5_8894,
    calls: &[
        (0x239, Callee::Sig(&crate::s4_misc::START_OBJECT_OWNERS)),
        (0x2e4, Callee::Import("_libm_sse2_sqrt_precise")),
        // __security_check_cookie
        (0x479, Callee::Any),
    ],
    imports: &[],
};
