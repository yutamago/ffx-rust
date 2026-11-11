//! Round 47's ports (own rows of `magic_0251`, `0267`, `0297`, `0348`,
//! `0385`, `0417` and `0434`): copies of ported functions on their own
//! offsets, and the DLLs' own code.

use crate::ob_model::{st0_f32, wrap_once};
use crate::runtime::slot_target;
use crate::sb_sse::{add, cvtt, div, mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

const ONE: u32 = 0x3f80_0000;

/// A node off the free ring (inlined in the start-ups): with `free`
/// nodes left, the node `n` (the head) leaves the ring (its neighbours
/// joined, each re-read), the head word (operand `head`) becomes `n`'s
/// next (0 for the last), the node is cleared (`+0x8` itself, `+0x10`
/// the step `cb`, byte `+0x18` 0) and the count stored (operand
/// `count`). Returns the node and the new head; with no free node the
/// original goes on with node 0 (it faults), the port too.
/// (`r44::ring_take`'s steps.)
fn ring_take<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    n: u32,
    free: &mut u16,
    (head, cb, count): (u32, u32, u32),
) -> (u32, u32) {
    let w = |o: u32| img.word(f + o);
    if *free == 0 {
        return (0, n);
    }
    let prev = g.read::<u32>(n);
    let next = g.read::<u32>(n + 4);
    g.write::<u32>(prev + 4, next);
    let next = g.read::<u32>(n + 4);
    let prev = g.read::<u32>(n);
    g.write::<u32>(next, prev);
    let new_head = if *free > 1 { g.read::<u32>(n + 4) } else { 0 };
    g.write::<u32>(w(head), new_head);
    *free = free.wrapping_sub(1);
    for o in [0u32, 4, 0xc, 0x14] {
        g.write::<u32>(n + o, 0);
    }
    g.write::<u32>(n + 8, n);
    g.write::<u32>(n + 0x10, w(cb));
    g.write::<u8>(n + 0x18, 0);
    g.write::<u16>(w(count), *free);
    (n, new_head)
}

/// The node `n` appended to the task list: `+0x4` 0; with no first node
/// (`first` 0) `n` is the first (`+0` 0, the operand `to_first`'s word),
/// else after `last` (`+0` = `last`, `last + 4` = `n`); `+0x8` itself.
fn task_append<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    n: u32,
    first: u32,
    last: u32,
    to_first: u32,
) {
    g.write::<u32>(n + 4, 0);
    if first == 0 {
        g.write::<u32>(n, 0);
        g.write::<u32>(img.word(f + to_first), n);
    } else {
        g.write::<u32>(n, last);
        g.write::<u32>(last + 4, n);
    }
    g.write::<u32>(n + 8, n);
}

/// `magic_0417`'s start-up (0x10003290, 2217 bytes; internal, no
/// arguments, run by its load step; an 8-aligned /GS frame, `S` = `esp`
/// after the three pushes). `magic_0072`'s start-up
/// ([`start_up_0072`](crate::r44::start_up_0072)) with these changes:
/// - the part buffers ([`parts_buffer`](crate::s4_small::parts_buffer))
///   instead of the part set-up, object 0's last word the code pointer
///   `+0x69`;
/// - the summon's yaw: `K - ` slot 333 `(*G_A)` wrapped once into a
///   global, the place's `w` 1, a global -0.5; no slot 268 for the centre
///   `C` or the targets' places in the distance loop (their `y` is slot
///   262's);
/// - a global 0x3c with the count `n`; the targets' `y` from slot 268's
///   vector at `S + 0x5c`;
/// - the second light block's word `base + 0x2c` (`base` = `n + 0x49`);
/// - the camera words: the two waits 0x1d and 0x59, no 0x1e;
/// - the sprite pool reset by
///   [`pool_reset_take`](crate::o6_fam_pool::pool_reset_take).
///
/// Returns 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn start_up_0417<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    // push ebp; and esp, -8; sub esp, 0x64; three pushes
    let b0 = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x64);
    let cookie = gw(g, 0xa) ^ b0;
    g.write::<u32>(b0 + 0x60, cookie);
    let s = b0.wrapping_sub(0xc);
    let rf = |g: &G, o: u32| g.read::<f32>(s + o);
    for (to, v) in [(0x2du32, 0x31u32), (0x37, 0x3b), (0x41, 0x45)] {
        g.write::<u32>(w(to), w(v));
    }
    // call width: 3
    let c = g.call(t(g, 0x15, 719), 0, &[w(0x27), 8, 0]).eax;
    g.write::<u32>(w(0x4f), c);
    // add esp, 4: two of slot 719's words stay
    let buf = Frame {
        at: call(0x54),
        esp: s.wrapping_sub(0xc),
        ..*fr
    };
    crate::s4_small::parts_buffer(g, img, &buf);
    let start =
        |g: &mut G, (at, obj, scale): (u32, u32, f32), (index, owner, b): (u32, u32, u32)| {
            crate::s4_misc::start_object_at(g, img, call(at), obj, scale, index, owner, b);
        };
    let scale = k(g, 0x5c);
    start(g, (0x73, w(0x64), scale), (0, 0x1d, w(0x69)));
    // the summon's place and yaw
    let a = gw(g, 0x7e);
    // call width: 4
    g.call(t(g, 0x78, 262), 0, &[a, w(0x93), w(0x8e), w(0x89)]);
    // call width: 3
    g.call(t(g, 0xa0, 268), 0, &[a, 6, s + 0x48]);
    let y = rf(g, 0x4c);
    g.write::<f32>(w(0xc1), y);
    let a = gw(g, 0xc7);
    g.write::<u32>(w(0xcd), ONE);
    // call width: 1
    let r = st0_f32(g.call(t(g, 0xb6, 333), 0, &[a]).st0);
    g.write::<f32>(s + 0x10, r);
    let yaw = sub(k(g, 0xe5), r);
    let yaw = wrap_once(g, img, f, yaw, [0xfd, 0xf3, 0x10d, 0xf3]);
    g.write::<u16>(w(0x11e), 0);
    let a = gw(g, 0x138);
    for (to, v) in [
        (0x146u32, 0xbf00_0000u32),
        (0x150, w(0x154)),
        (0x15a, 0),
        (0x164, 0),
        (0x16e, 0),
        (0x178, 0),
        (0x182, w(0x186)),
        (0x18c, w(0x190)),
        (0x196, 0),
        (0x1a0, 0),
        (0x1aa, w(0x1ae)),
    ] {
        g.write::<u32>(w(to), v);
    }
    g.write::<f32>(w(0x140), yaw);
    // the centre C at S + 0x28
    let cv = s + 0x28;
    // call width: 4
    g.call(t(g, 0x132, 262), 0, &[a, cv, cv + 4, cv + 8]);
    let count = gw(g, 0x1bc);
    g.write::<u32>(cv + 0xc, ONE);
    g.write::<f32>(s + 0x10, 0.0);
    g.write::<u32>(s + 0x14, count);
    // a target's handle (`i` = -1 never comes)
    let target = |g: &G, i: u32, (bytes, tab): (u32, u32)| {
        let b = g.read::<u8>(gw(g, bytes).wrapping_add(i));
        g.read::<u32>(w(tab).wrapping_add(4 * u32::from(b)))
    };
    let mut far = 0f32;
    let mut i = 0u32;
    while i.cast_signed() < count.cast_signed() {
        let h = target(g, i, (0x1ef, 0x1fa));
        let p = s + 0x38;
        // call width: 4
        g.call(t(g, 0x20f, 262), 0, &[h, p, p + 4, p + 8]);
        g.write::<u32>(p + 0xc, ONE);
        let d = [0u32, 4, 8].map(|o| sub(g.read::<f32>(cv + o), g.read::<f32>(p + o)));
        let sq = add(add(mul(d[1], d[1]), mul(d[0], d[0])), mul(d[2], d[2]));
        // `_libm_sse2_sqrt_precise` of the double: exact, as Rust's
        #[allow(clippy::cast_possible_truncation)] // cvtsd2ss
        let dist = f64::from(sq).sqrt() as f32;
        if dist > far {
            far = dist;
            g.write::<f32>(s + 0x10, far);
        }
        i += 1;
    }
    g.write::<f32>(w(0x289), far);
    let n = cvtt(div(far, k(g, 0x291)));
    g.write::<u32>(w(0x29d), 0x3c);
    g.write::<i32>(w(0x2a6), n);
    let base = n.wrapping_add(0x49);
    g.write::<i32>(s + 0x10, base);
    let mut i = 0u32;
    let mut v = w(0x2ba);
    while i.cast_signed() < count.cast_signed() {
        // `rand() % 10` (`idiv` by `ebx`, 10 on every turn)
        let r = g.rand().cast_signed() % 10;
        let scale = k(g, 0x2d4);
        start(
            g,
            (0x2e6, v - 0xd4, scale),
            (1, r.wrapping_add(base).cast_unsigned(), w(0x2c2)),
        );
        let h = target(g, i, (0x2fb, 0x306));
        // call width: 4
        g.call(t(g, 0x30f, 262), 0, &[h, v - 4, v, v + 4]);
        // call width: 3
        g.call(t(g, 0x327, 268), 0, &[h, 6, s + 0x5c]);
        let y = rf(g, 0x60);
        g.write::<f32>(v, y);
        g.write::<u32>(v + 8, ONE);
        let b = g.read::<u8>(gw(g, 0x357).wrapping_add(i));
        g.write::<u32>(v + 0x4c, u32::from(b));
        let dz = sub(rf(g, 0x30), g.read::<f32>(v + 4));
        let dx = sub(rf(g, 0x28), g.read::<f32>(v - 4));
        #[allow(clippy::cast_possible_truncation)] // cvtpd2ps
        let ang = g.atan2(f64::from(dz), f64::from(dx)) as f32;
        let yaw = sub(k(g, 0x3a5), ang);
        let yaw = if yaw > k(g, 0x3b4) {
            sub(yaw, k(g, 0x3be))
        } else if k(g, 0x3c8) > yaw {
            add(yaw, k(g, 0x3d5))
        } else {
            yaw
        };
        let rec = w(0x3e5).wrapping_add(i.wrapping_mul(0x34));
        g.write::<f32>(v - 0x10, yaw);
        for o in [0x18u32, 0x14, 0x10, 0x1c] {
            g.write::<u32>(rec + o, 0);
        }
        g.write::<u32>(v + 0x3c, rec);
        g.write::<u32>(rec + 0x20, w(0x414));
        g.write::<u32>(rec + 0x24, w(0x41b));
        g.write::<u32>(rec + 0x28, 0);
        g.write::<u32>(rec + 0x2c, 0);
        g.write::<u16>(rec + 0x30, 0);
        i += 1;
        v += 0x130;
    }
    // the light blocks and two nodes off the free ring
    let mut free = g.read::<u16>(w(0x441));
    let put = |g: &mut G, list: &[(u32, u32)]| {
        for &(to, v) in list {
            g.write::<u32>(w(to), v);
        }
    };
    put(
        g,
        &[
            (0x447, ONE),
            (0x451, ONE),
            (0x45b, ONE),
            (0x465, ONE),
            (0x46f, 0x3f00_0000),
            (0x479, 0x3f00_0000),
            (0x483, 0x3f00_0000),
            (0x48d, ONE),
            (0x497, 0),
            (0x4a1, 0),
            (0x4ab, 0x20),
            (0x4b5, 0x20),
        ],
    );
    let head = gw(g, 0x4d3);
    let (n1, head) = ring_take(g, img, f, head, &mut free, (0x4f8, 0x51d, 0x528));
    let first = gw(g, 0x52e);
    g.write::<u8>(n1 + 0x18, 1);
    g.write::<u32>(n1 + 0x14, w(0x539));
    let last = gw(g, 0x549);
    task_append(g, img, f, n1, first, last, 0x55e);
    let first = if first == 0 { n1 } else { first };
    put(
        g,
        &[
            (0x56e, 0x3f00_0000),
            (0x578, ONE),
            (0x582, 0x3f00_0000),
            (0x58c, 0x3f00_0000),
            (0x596, 0x3f00_0000),
            (0x5a0, ONE),
            (0x5aa, ONE),
            (0x5b4, ONE),
            (0x5be, ONE),
            (0x5c8, ONE),
            (0x5d1, base.wrapping_add(0x2c).cast_unsigned()),
            (0x5d7, 0),
            (0x5e1, 0x20),
            (0x5eb, 0x20),
        ],
    );
    // (the second take starts from the head the first one left: the
    // original keeps it in `edx`)
    let (n2, _) = ring_take(g, img, f, head, &mut free, (0x615, 0x64e, 0x659));
    g.write::<u8>(n2 + 0x18, 1);
    g.write::<u32>(n2 + 0x14, w(0x664));
    task_append(g, img, f, n2, first, n1, 0x682);
    g.write::<u32>(w(0x698), n2);
    // call width: 1
    g.call(t(g, 0x687, 271), 0, &[4]);
    // the camera words and the flames' two waits
    let x = gw(g, 0x69f);
    let flag = gw(g, 0x6a8);
    g.write::<u32>(w(0x6ae), x);
    put(
        g,
        &[
            (0x6ba, 0x1d),
            (0x6c4, 0x59),
            (0x6ce, w(0x6d2)),
            (0x6d8, u32::MAX),
        ],
    );
    let slot = if flag == 0 {
        t(g, 0x6b4, 215)
    } else {
        t(g, 0x6b4, 413)
    };
    // call width: 1
    let c = g.call(slot, 0, &[x]).eax;
    let flag = gw(g, 0x6f7);
    put(g, &[(0x702, c), (0x708, u32::MAX), (0x712, 0), (0x71c, 1)]);
    if flag == 0 {
        g.write::<u16>(w(0x726), 0);
    } else {
        g.write::<u16>(w(0x72f), 1);
    }
    // the sprite pool's node and the frame's node
    let mut free = g.read::<u16>(w(0x736));
    let head = gw(g, 0x74a);
    let (n3, _) = ring_take(g, img, f, head, &mut free, (0x767, 0x7a0, 0x7ab));
    let first = gw(g, 0x7b1);
    g.write::<u8>(n3 + 0x18, 1);
    let last = gw(g, 0x7c4);
    task_append(g, img, f, n3, first, last, 0x7d7);
    g.write::<u32>(w(0x7e5), n3);
    crate::o6_fam_pool::pool_reset_take(g, img, call(0x7ea), w(0x7dc));
    let mut free = g.read::<u16>(w(0x7f1));
    let head = gw(g, 0x800);
    let (n4, _) = ring_take(g, img, f, head, &mut free, (0x81d, 0x851, 0x85c));
    let first = gw(g, 0x862);
    g.write::<u8>(n4 + 0x18, 2);
    let last = gw(g, 0x875);
    task_append(g, img, f, n4, first, last, 0x888);
    g.write::<u32>(w(0x89a), n4);
    Ret::eax(0)
}

/// [`start_up_0417`] (the `rel32`s: the part buffers, the object starts,
/// `_libm_sse2_sqrt_precise`, `_CIatan2`, the pool reset,
/// `__security_check_cookie`; `rand` through the IAT).
pub const START_UP_0417: Sig = Sig {
    len: 2217,
    hash: 0x9122_e66f_3331_60a4,
    calls: &[
        (0x54, Callee::Sig(&crate::s4_small::PARTS_BUFFER_80000)),
        (0x73, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x25d, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x2e6, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x393, Callee::Import("_CIatan2")),
        (
            0x7ea,
            Callee::Sig(&crate::o6_fam_pool::POOL_RESET_TAKE_0417_4350),
        ),
        (0x8a1, Callee::Any),
    ],
    imports: &[(0x2c9, "rand")],
};

/// `magic_0417`'s load step (0x100030e0, 118 bytes; a table entry): the
/// object hosts' load step ([`load_step`](crate::r9_load::load_step))
/// running [`start_up_0417`].
pub fn load_0417<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::r9_load::load_step(g, img, fr, crate::r9_load::Shape::Zero, start_up_0417)
}

/// [`load_0417`] (the 118-byte load step's `Sig`, the `rel32` the
/// start-up).
pub const LOAD_0417: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&START_UP_0417))],
    imports: &[],
};

/// One of [`flames_0417`]'s sprite records: taken by
/// [`take_record`](crate::m0417::take_record) at the `rel32` `take` (none
/// ends the burst); the point `V` (`ebp - 0x14`) at `+0xc..+0x18`;
/// `(0, 0, r + K, 0)` at `+0x1c` (`r` slot 314 `(lift)`) turned by the
/// matrix `M` (slot 229); `+0x28` slot 314 `(pi)`; the colour `(0, g, 0,
/// 1)` at `+0x2c`; `+0x44` the count `n` as a float; u16s `+0x8` 0xffff,
/// `+0xa` 0x200; the handle at `+0` (slot 719 `(D, 9, 0)` with byte
/// `+0x3e` 0x60 and byte `+0x3f` `rand() % 65 + 32` for the first kind,
/// else the given word), `+4` 1, byte `+0x43` 0, `+0x64` 8. `ops` = the
/// operands `[314's load, K, 229's load, the second 314's load]`.
/// Returns the record, or `None`.
fn record_0417<G: Game>(
    g: &mut G,
    img: &Image,
    fr: &Frame,
    take: u32,
    handle: Option<u32>,
    (lift, green, n): (u32, u32, u32),
    ops: [u32; 4],
) -> Option<u32> {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, s: u32| slot_target(g, w(o), s);
    let ebp = fr.esp.wrapping_sub(4);
    let rec = crate::m0417::take_record(g, img, &Frame { at: take, ..*fr }).eax;
    if rec == 0 {
        return None;
    }
    let h = match handle {
        Some(h) => h,
        // call width: 3
        None => g.call(t(g, 0x233, 719), 0, &[w(0x242), 9, 0]).eax,
    };
    for (i, o) in (0u32..).zip([0xcu32, 0x10, 0x14, 0x18]) {
        let x = g.read::<u32>(ebp.wrapping_sub(0x14) + 4 * i);
        g.write::<u32>(rec + o, x);
    }
    let v = rec + 0x1c;
    g.write::<u32>(v, 0);
    g.write::<u32>(rec + 0x20, 0);
    // call width: 1
    let r = st0_f32(g.call(t(g, ops[0], 314), 0, &[lift]).st0);
    g.write::<f32>(rec + 0x24, add(r, g.read::<f32>(w(ops[1]))));
    g.write::<u32>(rec + 0x28, 0);
    // call width: 3
    g.call(t(g, ops[2], 229), 0, &[v, ebp.wrapping_sub(0x54), v]);
    // call width: 1
    let a = st0_f32(g.call(t(g, ops[3], 314), 0, &[0x4049_0fdb]).st0);
    g.write::<f32>(rec + 0x28, a);
    for (o, x) in [(0x2cu32, 0u32), (0x30, green), (0x34, 0), (0x38, ONE)] {
        g.write::<u32>(rec + o, x);
    }
    g.write::<f32>(rec + 0x44, crate::sb_sse::cvt(n));
    g.write::<u16>(rec + 8, 0xffff);
    g.write::<u32>(rec, h);
    g.write::<u32>(rec + 4, 1);
    g.write::<u16>(rec + 0xa, 0x200);
    if handle.is_none() {
        g.write::<u8>(rec + 0x3e, 0x60);
        let b = (g.rand().cast_signed() % 0x41).to_le_bytes()[0];
        g.write::<u8>(rec + 0x3f, b.wrapping_add(0x20));
    }
    g.write::<u8>(rec + 0x43, 0);
    g.write::<u32>(rec + 0x64, 8);
    Some(rec)
}

/// The size and speed of one of [`flames_0417`]'s first two kinds:
/// `s = (slot 404 (1.0) + K) K'` at `+0x4c`, `+0x5c` = `s K''`, `+0x50`
/// 0.1, `+0x54` 1.0, `+0x58` -5.0, `+0x60` 0, `+0x6c` 10, `+0x70` 0
/// (`ops` = 404's load, `K`, `K'`, `K''`).
fn size_0417<G: Game>(g: &mut G, img: &Image, f: u32, rec: u32, ops: [u32; 4]) {
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    // call width: 1
    let r = st0_f32(g.call(slot_target(g, w(ops[0]), 404), 0, &[ONE]).st0);
    let s = mul(add(r, k(g, ops[1])), k(g, ops[2]));
    for (o, x) in [
        (0x50u32, 0x3dcc_cccdu32),
        (0x54, ONE),
        (0x6c, 0xa),
        (0x70, 0),
        (0x58, 0xc0a0_0000),
        (0x60, 0),
    ] {
        g.write::<u32>(rec + o, x);
    }
    g.write::<f32>(rec + 0x4c, s);
    g.write::<f32>(rec + 0x5c, mul(s, k(g, ops[3])));
}

/// `magic_0417`'s flame burst (0x10003cc0, 1670 bytes; internal, no
/// arguments; a /GS `ebp` frame). While the wait `*G_W` is positive it
/// counts down (returns it); with the burst count `*G_N` 0 or less it
/// returns it; else `*G_N` counts down, the fade count `*G_F` too while
/// positive, and a burst starts:
/// - `L = (K - x) K' + x K''` with `x = f / D` (`f` the fade count as a
///   float);
/// - the summon's point `V` (slot 237 `(*G_A, 0xb, 0, V)`, `w` 1); its yaw
///   `Y` = `K0 - ` slot 333 `(*G_A)` wrapped once;
/// - the matrix `M` (slot 235) turned by `P` = slot 404 `(0.157)` + slot
///   314 `(0.157)` + 0, wrapped once (slot 328), and by `Q` = slot 314
///   `(L)` + `Y` + `PI`, wrapped once (slot 329);
/// - twice the pair of sprite records ([`record_0417`]: the first kind
///   with slot 719's handle and the random byte, lift 1.5; the second
///   with the handle `G_H[1]`; both sized by [`size_0417`]), then a last
///   record (the handle `G_H[2]`, green 0.2, slot 404 `(0.5)`, `+0x58`
///   0, `+0x5c` 1000, `+0x60` 0, `+0x74` 1).
///
/// Returns the last slot 404's `eax`, or 0 when the records ran out.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn flames_0417<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let ebp = fr.esp.wrapping_sub(4);
    let lf = |o: u32| ebp.wrapping_sub(o);
    let cookie = gw(g, 0x7) ^ ebp;
    g.write::<u32>(lf(4), cookie);
    let wait = gw(g, 0x11).cast_signed();
    if wait > 0 {
        let x = wait.wrapping_sub(1).cast_unsigned();
        g.write::<u32>(w(0x1b), x);
        return Ret::eax(x);
    }
    let c = gw(g, 0x2e);
    if c.cast_signed() <= 0 {
        return Ret::eax(c);
    }
    g.write::<u32>(w(0x3c), c.wrapping_sub(1));
    let mut fade = gw(g, 0x41);
    if fade.cast_signed() > 0 {
        fade = fade.wrapping_sub(1);
        g.write::<u32>(w(0x4b), fade);
    }
    let x = div(crate::sb_sse::cvt(fade), k(g, 0x62));
    let l = add(mul(sub(k(g, 0x5a), x), k(g, 0x7f)), mul(x, k(g, 0x71)));
    let v = lf(0x14);
    let a = gw(g, 0x93);
    // call width: 4
    g.call(t(g, 0x77, 237), 0, &[a, 0xb, 0, v]);
    let a = gw(g, 0xa5);
    g.write::<u32>(lf(8), ONE);
    // call width: 1
    let r = st0_f32(g.call(t(g, 0x9f, 333), 0, &[a]).st0);
    let yaw = wrap_once(g, img, f, sub(k(g, 0xbf), r), [0xd6, 0xcc, 0xeb, 0xcc]);
    // call width: 1
    let a1 = st0_f32(g.call(t(g, 0xfe, 314), 0, &[0x3e20_d97c]).st0);
    // call width: 1
    let a2 = st0_f32(g.call(t(g, 0x116, 404), 0, &[0x3e20_d97c]).st0);
    let p = add(add(a2, a1), 0.0);
    // call width: 1
    let r3 = st0_f32(g.call(t(g, 0x137, 314), 0, &[l.to_bits()]).st0);
    let (hi, tw, lo) = (k(g, 0x165), k(g, 0x17f), k(g, 0x177));
    let q = add(add(r3, yaw), hi);
    let p = if p > hi {
        sub(p, tw)
    } else if lo > p {
        add(p, tw)
    } else {
        p
    };
    let q = if q > hi {
        sub(q, tw)
    } else if lo > q {
        add(q, tw)
    } else {
        q
    };
    let m = lf(0x54);
    // call width: 1
    g.call(t(g, 0x1c6, 235), 0, &[m]);
    // call width: 3
    g.call(t(g, 0x1e2, 328), 0, &[m, m, p.to_bits()]);
    // call width: 3
    g.call(t(g, 0x201, 329), 0, &[m, m, q.to_bits()]);
    let n = gw(g, 0x20e);
    let take = |o: u32| img.rel32_target(f + o);
    let words = |g: &G, o: u32, i: u32| g.read::<u32>(gw(g, o) + 4 * i);
    for _ in 0..2 {
        let Some(rec) = record_0417(
            g,
            img,
            fr,
            take(0x223),
            None,
            (0x3fc0_0000, 0, n),
            [0x26e, 0x2a5, 0x2af, 0x2c9],
        ) else {
            return Ret::eax(0);
        };
        size_0417(g, img, f, rec, [0x337, 0x367, 0x387, 0x3a2]);
        let h = words(g, 0x3c3, 1);
        let Some(rec) = record_0417(
            g,
            img,
            fr,
            take(0x3b3),
            Some(h),
            (ONE, 0, n),
            [0x3d0, 0x423, 0x42d, 0x447],
        ) else {
            return Ret::eax(0);
        };
        size_0417(g, img, f, rec, [0x49c, 0x4d2, 0x4ec, 0x507]);
    }
    let h = words(g, 0x53b, 2);
    let Some(rec) = record_0417(
        g,
        img,
        fr,
        take(0x52b),
        Some(h),
        (ONE, 0x3e4c_cccd, n),
        [0x548, 0x59b, 0x5a5, 0x5bf],
    ) else {
        return Ret::eax(0);
    };
    // call width: 1
    let out = g.call(t(g, 0x612, 404), 0, &[0x3f00_0000]);
    let s = mul(add(st0_f32(out.st0), k(g, 0x648)), k(g, 0x66c));
    for (o, x) in [(0x58u32, 0u32), (0x5c, 0x447a_0000), (0x60, 0), (0x74, 1)] {
        g.write::<u32>(rec + o, x);
    }
    g.write::<f32>(rec + 0x4c, s);
    Ret::eax(out.eax)
}

/// [`flames_0417`] (the `rel32`s: the record takes, the cookie check
/// twice; `rand` through the IAT).
pub const FLAMES_0417: Sig = Sig {
    len: 1670,
    hash: 0x85ee_a6d5_958b_ad5d,
    calls: &[
        (0x25, Callee::Any),
        (0x223, Callee::Sig(&crate::m0417::TAKE_RECORD)),
        (0x3b3, Callee::Sig(&crate::m0417::TAKE_RECORD)),
        (0x52b, Callee::Sig(&crate::m0417::TAKE_RECORD)),
        (0x67e, Callee::Any),
    ],
    imports: &[(0x32a, "rand")],
};

/// `magic_0417`'s frame step (0x10003b70, 326 bytes; a task step, cdecl
/// `(task)`; an 8-aligned frame): the flame burst ([`flames_0417`]), the
/// objects ([`run_objects`](crate::l5_objhost::run_objects)) and the
/// motion script ([`motion_step`](crate::s3_motion::motion_step)); when
/// both give 1, `*G_F` is 0 and `*G_W` not positive: the task's state 3
/// and its record's `+0x10` the next step, then for each of `*G_N`
/// handles `i` three sounds (slot 772 `(h, i)` of slot 719 `(D, 9, 0)`'s
/// handle, then of `G_H[1]` and `G_H[2]`, each played by slot 779 `(lo,
/// hi)` with the lock word set to 1 and put back). Returns the motion's
/// result when it stops early, the record with no sounds, else the lock
/// global's word.
pub fn frame_0417<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    // push ebp; and esp, -8; push esi; push edi; the calls
    let esp = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0xc);
    let ff = Frame {
        at: call(0x9),
        esp,
        ..*fr
    };
    flames_0417(g, img, &ff);
    let objs = crate::l5_objhost::run_objects(g, img, call(0xe), esp);
    let mv = crate::s3_motion::motion_step(g, img, call(0x15));
    if mv != 1 || objs != 1 || gw(g, 0x2c) != 0 || gw(g, 0x39).cast_signed() > 0 {
        return Ret::eax(mv);
    }
    let task = fr.arg(g, 0);
    g.write::<u32>(task + 0x14, 3);
    let rec = g.read::<u32>(task + 8);
    g.write::<u32>(rec + 0x10, w(0x56));
    if gw(g, 0x5c).cast_signed() <= 0 {
        return Ret::eax(rec);
    }
    let mut i = 0u32;
    loop {
        // call width: 3
        let h0 = g.call(t(g, 0x71, 719), 0, &[w(0x80), 9, 0]).eax;
        // (the handle's source, slot 772's load, the lock's global
        // before and after, slot 779's load)
        for (src, s772, lock_at, s779, back) in [
            (None, 0x8cu32, 0x9au32, 0xa3u32, 0xb6u32),
            (Some((0xbe, 4)), 0xc6, 0xd4, 0xdd, 0xf0),
            (Some((0xf8, 8)), 0x100, 0x10e, 0x117, 0x12a),
        ] {
            let h = match src {
                None => h0,
                Some((o, d)) => g.read::<u32>(gw(g, o) + d),
            };
            // call width: 2
            let out = g.call(t(g, s772, 772), 0, &[h, i]);
            let lock = gw(g, lock_at);
            let old = g.read::<u32>(lock);
            g.write::<u32>(lock, 1);
            // call width: 2
            g.call(t(g, s779, 779), 0, &[out.eax, out.edx]);
            let lock = gw(g, back);
            g.write::<u32>(lock, old);
        }
        i += 1;
        if i.cast_signed() >= gw(g, 0x136).cast_signed() {
            break;
        }
    }
    Ret::eax(gw(g, 0x12a))
}

/// [`frame_0417`] (the `rel32`s: the flames, the objects, the motion).
pub const FRAME_0417: Sig = Sig {
    len: 326,
    hash: 0xb478_3e5d_d6fe_61c5,
    calls: &[
        (0x9, Callee::Sig(&FLAMES_0417)),
        (0xe, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
        (0x15, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
    ],
    imports: &[],
};

/// `magic_0417`'s sprite draw (0x100044a0, 1080 bytes; a stored callback,
/// no arguments; an 8-aligned /GS frame, `S` = `esp` after the three
/// pushes). The view matrix `A` (`S + 0xd8`, slot 723, then slot 235),
/// the camera `B` (`S + 0x118`, slot 724), the scale matrix `C` (`S +
/// 0x98`, slot 235); `G_D` = `S + 0x18`, `G_E` = `S + 0x58`. For each of
/// the `*G_N` records `R` (0x7c bytes from `*G_R`) with `+0x78` set:
/// - while the wait `+0x60` is positive it counts down;
/// - else the speed `+0x1c..` gains `+0x2c..` and the point `+0xc..`
///   the speed; the colour: while the fade `+0x44` is positive it drops
///   by `K` and the colour global is `+0x3c`; else with the blend count
///   `+0x64` positive each byte is `(a (n - |t|) + b |t|) / n` (`a`
///   `+0x3c..`, `b` `+0x40..`, `t` `+0x68`, which counts up; past `n` the
///   count is 0); else the colour is `+0x40` and `+0x78` 0;
/// - past the ceiling `+0x5c` the point's `y` stops there, the speed's
///   `y` and its gain 0, `+0x64` doubles, `+0x1c` and `+0x24` scaled by
///   slot 404 `(0.5) + K`;
/// - the size `s = ((K - x) +0x50 + +0x54 x) +0x4c` with `x = +0x70 /
///   +0x6c` while `+0x6c` is positive (`+0x70` counting up, `+0x6c` 0
///   past it), else `+0x54 +0x4c`; `C`'s diagonal `s` and slot 330 `(D,
///   C, +0x28)`; slot 229 `(S + 0x48, B, R + 0xc)`; slot 230 `(E, D)`
///   with `E`'s `z` moved by `+0x58`; slots 231 `(E, A, E)` and `(D, A,
///   D)`;
/// - the frame ([`anim_step`](crate::sb_common::anim_step) on `R`),
///   `+0x78` 0 once `+0x74` is 1 and `+4` is 0, and the part linked
///   ([`link_part`](crate::sa_misc::link_part) `(G_L, i)`).
///
/// The drawn records' count goes into a global. Returns `eax` as the
/// original leaves it (the last link's, a wait's new value or slot
/// 235's).
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn draw_0417<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    // push ebp; and esp, -8; sub esp, 0x154; three pushes
    let b0 = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x154);
    let cookie = gw(g, 0xd) ^ b0;
    g.write::<u32>(b0 + 0x150, cookie);
    let s = b0.wrapping_sub(0xc);
    let (a, b, c, d, e, pt) = (s + 0xd8, s + 0x118, s + 0x98, s + 0x18, s + 0x58, s + 0x48);
    let recs = gw(g, 0x1f);
    // call width: 1
    g.call(t(g, 0x2c, 723), 0, &[a]);
    // call width: 1
    g.call(t(g, 0x44, 724), 0, &[b]);
    // call width: 1
    g.call(t(g, 0x5c, 235), 0, &[a]);
    g.write::<u32>(w(0x6d), d);
    g.write::<u32>(w(0x76), e);
    // call width: 1
    let mut eax = g.call(t(g, 0x86, 235), 0, &[c]).eax;
    let mut count = gw(g, 0x94);
    let mut drawn = 0u32;
    let mut i = 0u32;
    while i.cast_signed() < count.cast_signed() {
        let r = recs.wrapping_add(i.wrapping_mul(0x7c));
        let rd = |g: &G, o: u32| g.read::<f32>(r + o);
        if g.read::<u32>(r + 0x78) == 0 {
            i += 1;
            continue;
        }
        let wait = g.read::<u32>(r + 0x60);
        if wait.cast_signed() > 0 {
            eax = wait.wrapping_sub(1);
            g.write::<u32>(r + 0x60, eax);
            i += 1;
            continue;
        }
        for o in [0x1cu32, 0x20, 0x24] {
            let x = add(rd(g, o + 0x10), rd(g, o));
            g.write::<f32>(r + o, x);
        }
        // (`+0xc` gains the speed, the other two the speed gains them:
        // the operands in the original's order)
        let x = add(rd(g, 0x1c), rd(g, 0xc));
        g.write::<f32>(r + 0xc, x);
        for o in [0x10u32, 0x14] {
            let x = add(rd(g, o), rd(g, o + 0x10));
            g.write::<f32>(r + o, x);
        }
        let fade = rd(g, 0x44);
        if fade > 0.0 {
            g.write::<f32>(r + 0x44, sub(fade, k(g, 0xb3)));
            let x = g.read::<u32>(r + 0x3c);
            g.write::<u32>(w(0x148), x);
        } else {
            let n = g.read::<i32>(r + 0x64);
            if n > 0 {
                let tt = g.read::<i32>(r + 0x68).wrapping_abs();
                for (j, op) in (0u32..).zip([0x181u32, 0x19f, 0x1bd, 0x1df]) {
                    let from = i32::from(g.read::<u8>(r + 0x3c + j));
                    let to = i32::from(g.read::<u8>(r + 0x40 + j));
                    let v = from
                        .wrapping_mul(n.wrapping_sub(tt))
                        .wrapping_add(to.wrapping_mul(tt))
                        / n;
                    g.write::<u8>(w(op), v.to_le_bytes()[0]);
                }
                let tt = g.read::<i32>(r + 0x68).wrapping_add(1);
                g.write::<i32>(r + 0x68, tt);
                if tt > g.read::<i32>(r + 0x64) {
                    g.write::<u32>(r + 0x64, 0);
                }
            } else {
                let x = g.read::<u32>(r + 0x40);
                g.write::<u32>(w(0x1ff), x);
                g.write::<u32>(r + 0x78, 0);
            }
        }
        let top = rd(g, 0x5c);
        if rd(g, 0x10) > top {
            g.write::<f32>(r + 0x10, top);
            g.write::<u32>(r + 0x20, 0);
            g.write::<u32>(r + 0x30, 0);
            let n = g.read::<u32>(r + 0x64).wrapping_mul(2);
            g.write::<u32>(r + 0x64, n);
            for (o, op, ko) in [(0x1cu32, 0x239u32, 0x25bu32), (0x24, 0x260, 0x28b)] {
                // call width: 1
                let q = st0_f32(g.call(t(g, op, 404), 0, &[0x3f00_0000]).st0);
                let x = mul(add(q, k(g, ko)), rd(g, o));
                g.write::<f32>(r + o, x);
            }
        }
        let steps = g.read::<i32>(r + 0x6c);
        let big = rd(g, 0x4c);
        let size = if steps > 0 {
            let at = g.read::<i32>(r + 0x70);
            #[allow(clippy::cast_precision_loss)] // cvtdq2ps
            let x = div(at as f32, steps as f32);
            let hi = mul(rd(g, 0x54), x);
            let lo = mul(sub(k(g, 0x28b), x), rd(g, 0x50));
            g.write::<i32>(r + 0x70, at.wrapping_add(1));
            if at.wrapping_add(1) > steps {
                g.write::<u32>(r + 0x6c, 0);
            }
            mul(add(lo, hi), big)
        } else {
            mul(rd(g, 0x54), big)
        };
        let ang = g.read::<u32>(r + 0x28);
        for o in [0x28u32, 0x14, 0] {
            g.write::<f32>(c + o, size);
        }
        // call width: 3
        g.call(t(g, 0x30c, 330), 0, &[d, c, ang]);
        // call width: 3
        g.call(t(g, 0x348, 229), 0, &[pt, b, r + 0xc]);
        // call width: 2
        g.call(t(g, 0x362, 230), 0, &[e, d]);
        let z = add(rd(g, 0x58), g.read::<f32>(e + 0x38));
        g.write::<f32>(e + 0x38, z);
        // call width: 3
        g.call(t(g, 0x38f, 231), 0, &[e, a, e]);
        // call width: 3
        g.call(t(g, 0x3ba, 231), 0, &[d, a, d]);
        let entry = crate::sb_common::anim_step(g, img, call(0x3cd), r);
        if g.read::<u32>(r + 0x74) == 1 && g.read::<u32>(r + 4) == 0 {
            g.write::<u32>(r + 0x78, 0);
        }
        // push i; push G_L; the call's return address below them
        let esp = s.wrapping_sub(0xc);
        g.write::<u32>(esp + 4, w(0x3e6));
        g.write::<u32>(esp + 8, i);
        let link = Frame {
            at: call(0x3f0),
            esp,
            ecx: r,
            edx: entry,
            ..*fr
        };
        eax = crate::sa_misc::link_part(g, img, &link);
        count = gw(g, 0x3f6);
        drawn += 1;
        i += 1;
    }
    g.write::<u32>(w(0x427), drawn);
    Ret::eax(eax)
}

/// [`draw_0417`] (the `rel32`s: the frame step, the part link, the
/// cookie check).
pub const DRAW_0417: Sig = Sig {
    len: 1080,
    hash: 0xa3b3_3768_f20b_a900,
    calls: &[
        (0x3cd, Callee::Sig(&crate::sb_common::ANIM_STEP)),
        (0x3f0, Callee::Sig(&crate::sa_misc::LINK_PART)),
        (0x430, Callee::Any),
    ],
    imports: &[],
};
