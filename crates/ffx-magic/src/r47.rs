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

/// `magic_0434`'s start-up (0x10009900, 995 bytes; internal, no
/// arguments, no frame; run by its load step). `magic_0610`'s start-up
/// ([`start_up_0610`](crate::r22::start_up_0610)) with its buffers in one
/// block: `B = *G_B`, or slot 803 `(0x230f90, 0x10)` when 0 (kept), and
/// six pointers into it; the history bytes moved up and their count one
/// up; slot 719 `(PART, 8, 0)` into `G_H`, slots 711 `(h, BUF, 0)` (the
/// game lock held) and 710 `(OUT, *G_H, B, 0x40000)`; four words 0; the
/// caster placed ([`caster_place_0588`](crate::r18::caster_place_0588));
/// 100 records (0x140 apart from `B + 0x226930`) cleared and their
/// counts; the effect reset ([`effect_reset`](crate::sb_misc::effect_reset));
/// the matrix `M` = slot 235 turned by the effect's `+0x1a68` (slot 329)
/// and its last row the effect's `+0xc70..`; two node heads; slot 271
/// `(4)`; two task nodes (bytes 1 and 2) with the records set up between
/// them ([`records_init`](crate::sa_misc::records_init) on `B +
/// 0x218000`); when slot 520 says 0x1ad0001 a global steps by `K`.
/// Returns 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn start_up_0434<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let mut b = gw(g, 0x5);
    if b == 0 {
        // call width: 2
        b = g.call(t(g, 0xe, 803), 0, &[0x0023_0f90, 0x10]).eax;
        g.write::<u32>(w(0x28), b);
    }
    for (o, d) in [
        (0x33u32, 0x15_0000u32),
        (0x3e, 0x19_0000),
        (0x49, 0x21_0000),
        (0x5b, 0x4_0000),
        (0x66, 0x22_e500),
    ] {
        g.write::<u32>(w(o), b.wrapping_add(d));
    }
    let n = gw(g, 0x6b);
    g.write::<u32>(w(0x71), b.wrapping_add(0x13_0000));
    g.write::<u32>(w(0x7a), 0);
    let mut i = n.cast_signed().wrapping_sub(1);
    if i >= 0 {
        loop {
            i = i.wrapping_sub(1);
            let x = g.read::<u8>(w(0x93).wrapping_add(i.cast_unsigned()));
            g.write::<u8>(w(0x99).wrapping_add(i.cast_unsigned()), x);
            if i < 0 {
                break;
            }
        }
    }
    let n = gw(g, 0xa0).wrapping_add(1);
    g.write::<u32>(w(0xa6), n);
    // call width: 3
    let h = g.call(t(g, 0xab, 719), 0, &[w(0xba), 8, 0]).eax;
    let lock = gw(g, 0xc2);
    g.write::<u32>(w(0xc9), h);
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    // call width: 3
    g.call(t(g, 0xd6, 711), 0, &[h, w(0xd0), 0]);
    let lock = gw(g, 0xe9);
    g.write::<u32>(lock, saved);
    let h = gw(g, 0xfc);
    // call width: 4
    g.call(t(g, 0xf5, 710), 0, &[w(0x107), h, b, 0x4_0000]);
    for o in [0x112u32, 0x11c, 0x126, 0x130] {
        g.write::<u32>(w(o), 0);
    }
    // push ebx; push esi; push edi: the return address at E - 0x10
    let cf = Frame {
        at: call(0x139),
        esp: fr.esp.wrapping_sub(0x10),
        ..*fr
    };
    crate::r18::caster_place_0588(g, img, &cf);
    let recs = b.wrapping_add(0x22_6800);
    g.write::<u32>(w(0x144), recs);
    let mut rec = recs.wrapping_add(0x130);
    for _ in 0..0x64 {
        g.write::<u32>(rec, 0);
        rec = rec.wrapping_add(0x140);
    }
    let e = gw(g, 0x163);
    g.write::<u32>(w(0x169), 0);
    g.write::<u32>(w(0x16f), 0x64);
    crate::sb_misc::effect_reset(g, img, call(0x178));
    g.write::<u8>(w(0x183), 1);
    // call width: 1
    g.call(t(g, 0x17d, 235), 0, &[w(0x18f)]);
    let a = g.read::<u32>(e.wrapping_add(0x1a68));
    // call width: 3
    g.call(t(g, 0x19e, 329), 0, &[w(0x1b3), w(0x1ae), a]);
    for (j, o) in (0u32..).zip([0x1cau32, 0x1da, 0x1ea, 0x1fa]) {
        let x = g.read::<u32>(e.wrapping_add(0xc70 + 4 * j));
        g.write::<u32>(w(o), x);
    }
    g.write::<u32>(w(0x200), w(0x204));
    g.write::<u32>(w(0x20a), 0);
    g.write::<u16>(w(0x215), 0);
    g.write::<u8>(w(0x21d), 1);
    g.write::<u32>(w(0x224), w(0x228));
    g.write::<u32>(w(0x22e), 0);
    g.write::<u16>(w(0x239), 0);
    g.write::<u8>(w(0x241), 1);
    // call width: 1
    g.call(t(g, 0x1c2, 271), 0, &[4]);
    // the two task nodes off the free ring, the records between them
    let mut free = g.read::<u16>(w(0x253));
    let head = gw(g, 0x26a);
    let (n1, _) = ring_take(g, img, f, head, &mut free, (0x287, 0x2bb, 0x2c6));
    let first = gw(g, 0x2cc);
    g.write::<u8>(n1 + 0x18, 1);
    let last = gw(g, 0x2df);
    task_append(g, img, f, n1, first, last, 0x2f2);
    g.write::<u32>(w(0x301), n1);
    crate::sa_misc::records_init(g, img, call(0x306), b.wrapping_add(0x21_8000));
    let mut free = g.read::<u16>(w(0x30d));
    let head = gw(g, 0x31c);
    let (n2, _) = ring_take(g, img, f, head, &mut free, (0x339, 0x36d, 0x378));
    let first = gw(g, 0x37e);
    g.write::<u8>(n2 + 0x18, 2);
    let last = gw(g, 0x394);
    task_append(g, img, f, n2, first, last, 0x3a7);
    g.write::<u32>(w(0x3bb), n2);
    // call width: 0
    if g.call(t(g, 0x3ac, 520), 0, &[]).eax == 0x01ad_0001 {
        let v = add(g.read::<f32>(w(0x3cc)), g.read::<f32>(w(0x3d4)));
        g.write::<f32>(w(0x3dc), v);
    }
    Ret::eax(0)
}

/// [`start_up_0434`] (the `rel32`s: the caster place, the effect reset,
/// the records' set-up).
pub const START_UP_0434: Sig = Sig {
    len: 995,
    hash: 0x5dbc_d9a3_cf11_a60b,
    calls: &[
        (0x139, Callee::Sig(&crate::r24::CASTER_PLACE_0434)),
        (0x178, Callee::Sig(&crate::sb_misc::EFFECT_RESET)),
        (0x306, Callee::Sig(&crate::sa_misc::RECORDS_INIT)),
    ],
    imports: &[],
};

/// `magic_0434`'s load step (0x100012c0, 181 bytes; a table entry): the
/// cue start ([`cue_start`](crate::m0591::cue_start)) with the summon's
/// handle `*(*G_E + 0x1a80)` for slots 5 and 28 instead of a fixed ID,
/// and [`start_up_0434`] at the call. Returns 0.
pub fn load_0434<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    // call width: 1
    if g.call(t(g, 0x1, 225), 0, &[1]).eax != 0 {
        return Ret::eax(0);
    }
    let id = g.read::<u32>(gw(g, 0x1b).wrapping_add(0x1a80));
    // call width: 1
    if g.call(t(g, 0x27, 5), 0, &[id]).eax != 0 {
        return Ret::eax(0);
    }
    // call width: 2
    if g.call(t(g, 0x39, 28), 0, &[id, 1]).eax != 0 {
        return Ret::eax(0);
    }
    if gw(g, 0x54) == 0 {
        let h = gw(g, 0x64);
        // call width: 2
        g.call(t(g, 0x5c, 226), 0, &[h, 0]);
        g.write::<u32>(w(0x75), 1);
    }
    let h = gw(g, 0x84);
    // call width: 1
    if g.call(t(g, 0x7e, 227), 0, &[h]).eax.cast_signed() < 1 {
        return Ret::eax(0);
    }
    // push esi: the call's return address below it
    let callee = Frame {
        at: img.rel32_target(f + 0x99),
        esp: fr.esp.wrapping_sub(8),
        ..*fr
    };
    start_up_0434(g, img, &callee);
    g.write::<u32>(w(0x9f), w(0xa3));
    g.write::<u32>(w(0xa9), w(0xad));
    Ret::eax(0)
}

/// [`load_0434`] (the `rel32` the start-up).
pub const LOAD_0434: Sig = Sig {
    len: 181,
    hash: 0x922f_4def_6f26_b417,
    calls: &[(0x99, Callee::Sig(&START_UP_0434))],
    imports: &[],
};

/// `magic_0251`'s start-up (0x10004470, 3124 bytes; internal, no
/// arguments, run by its load step; a /GS `ebp` frame: the point `V` at
/// `ebp - 0x14`, the matrices `M` at `ebp - 0x54` and `N` at `ebp -
/// 0xd4`):
/// - three words, `*G_C` = slot 719 `(D, 8, 0)`; the part set-up
///   ([`part_init`](crate::l5_objhost::part_init)); `V` = the actors'
///   mean place ([`mean_position`](crate::sb_misc::mean_position));
///   object 1 ([`start_object_at`](crate::s4_misc::start_object_at) `(1,
///   0x1d, -, 0)`); `V` into four globals (`y` less `K`); the yaw `K0 -`
///   slot 333 `(*G_A)` wrapped once; the record at `G_R` started on a
///   node ([`record_node_start`](crate::o6_fam_pairs5::record_node_start)
///   `(V, 0.03, 0.99)`, scale `K1`);
/// - the camera words (slot 413 or 215 of `*G_X` by a flag) and a u16 or-ed
///   with 1;
/// - per actor `i` (`*G_N`): object 0 `(0, 0x25, -, CB)` at the actor's
///   place (slot 262, `y` from slot 268), the yaw `K - ` slot 333 wrapped
///   once; the actor's mode ([`actor_mode`](crate::sa_script::actor_mode))
///   and its record `B` (0xdc apart): `+8` 0, `+0xc` the yaw again,
///   `+0x10`, `+0x14` 0, the scale matrix
///   ([`actor_scale`](crate::sa_misc::actor_scale)) turned by slots 330,
///   328, 329; its place (slot 262) twice, three offset points (slot 314
///   `(20)` and 404 `(30)` randoms through `M`/`N` turned by `+8`, `+0xc`;
///   slots 235, 328, 329, 229, 248), the wave words, a record of its own
///   at `B - 0x58` (the selection byte, slot 413/215's handle) on a task
///   node off the free ring;
/// - a block of globals, two task nodes (the first unflagged, the second
///   1), the records' set-up
///   ([`records_init_100`](crate::sa_misc::records_init_100)), the
///   sprite pool ([`pool_reset_take`](crate::o6_fam_pool::pool_reset_take)),
///   more globals and two more nodes (1, then 2, the second after the
///   first).
///
/// Returns 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn start_up_0251<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let lf = |o: u32| ebp.wrapping_sub(o);
    let cookie = gw(g, 0xa) ^ ebp;
    g.write::<u32>(lf(4), cookie);
    for (to, v) in [(0x29u32, 0x2du32), (0x33, 0x37), (0x3d, 0x41)] {
        g.write::<u32>(w(to), w(v));
    }
    // call width: 3
    let c = g.call(t(g, 0x14, 719), 0, &[w(0x23), 8, 0]).eax;
    g.write::<u32>(w(0x4b), c);
    crate::l5_objhost::part_init(g, img, call(0x50));
    let v = lf(0x14);
    crate::sb_misc::mean_position(g, img, call(0x5b), lf(0xe4), v);
    let start =
        |g: &mut G, (at, obj, scale): (u32, u32, f32), (index, owner, b): (u32, u32, u32)| {
            crate::s4_misc::start_object_at(g, img, call(at), obj, scale, index, owner, b);
        };
    let scale = k(g, 0x63);
    start(g, (0x74, w(0x6f), scale), (1, 0x1d, 0));
    let a = gw(g, 0x84);
    for (o, i) in [(0x8cu32, 0u32), (0x99, 2), (0xa6, 3)] {
        let x = g.read::<u32>(v + 4 * i);
        g.write::<u32>(w(o), x);
    }
    let y = sub(g.read::<f32>(v + 4), k(g, 0xb3));
    g.write::<f32>(w(0xbb), y);
    // call width: 1
    let r = st0_f32(g.call(t(g, 0x7e, 333), 0, &[a]).st0);
    g.write::<f32>(lf(0xe0), r);
    let yaw = wrap_once(g, img, f, sub(k(g, 0xd1), r), [0xeb, 0xe1, 0xfb, 0xe1]);
    g.write::<f32>(w(0x12f), yaw);
    // sub esp, 0xc; the two scales; push V: the call's return address
    // at ebp - 0xf4
    let esp = lf(0xf4);
    for (i, x) in (1u32..).zip([v, 0x3cf5_c28f, 0x3f7d_70a4]) {
        g.write::<u32>(esp + 4 * i, x);
    }
    let mut xmm = fr.xmm;
    xmm[3] = u128::from(gw(g, 0x10c));
    let rs = Frame {
        at: call(0x134),
        esp,
        ecx: w(0x127),
        xmm,
        ..*fr
    };
    crate::o6_fam_pairs5::record_node_start(g, img, &rs);
    // the camera words
    let x = gw(g, 0x139);
    let flag = gw(g, 0x142);
    g.write::<u32>(w(0x148), x);
    g.write::<u32>(w(0x154), w(0x158));
    g.write::<u32>(w(0x15e), u32::MAX);
    let slot = if flag == 0 {
        t(g, 0x14e, 215)
    } else {
        t(g, 0x14e, 413)
    };
    // call width: 1
    let c = g.call(slot, 0, &[x]).eax;
    let flag = gw(g, 0x17d);
    for (o, x) in [(0x188u32, c), (0x18e, u32::MAX), (0x198, 0), (0x1a2, 1)] {
        g.write::<u32>(w(o), x);
    }
    if flag == 0 {
        g.write::<u16>(w(0x1ac), 0);
    } else {
        g.write::<u16>(w(0x1b5), 1);
    }
    let x = g.read::<u16>(w(0x1bc)) | 1;
    g.write::<u16>(w(0x1bc), x);
    g.write::<u32>(w(0x1c7), 0xe);
    g.write::<u32>(w(0x1d1), w(0x1d5));
    // the actors
    let target = |g: &G, i: u32, (bytes, tab): (u32, u32)| {
        let b = g.read::<u8>(gw(g, bytes).wrapping_add(i));
        g.read::<u32>(w(tab).wrapping_add(4 * u32::from(b)))
    };
    let mut b = w(0x1ec);
    let mut o = w(0x1f1);
    let mut i = 0u32;
    while i.cast_signed() < gw(g, 0x1e1).cast_signed() {
        g.write::<u32>(lf(0xe0), i);
        let scale = k(g, 0x20a);
        start(g, (0x21f, o - 0xd4, scale), (0, 0x25, w(0x20f)));
        let h = target(g, i, (0x234, 0x23f));
        // call width: 4
        g.call(t(g, 0x24d, 262), 0, &[h, o - 4, o, o + 4]);
        // call width: 3
        g.call(t(g, 0x265, 268), 0, &[h, 6, lf(0x64)]);
        let y = g.read::<u32>(lf(0x60));
        g.write::<u32>(o, y);
        g.write::<u32>(o + 8, ONE);
        let h = target(g, i, (0x299, 0x2a4));
        // call width: 1
        let r = st0_f32(g.call(t(g, 0x2aa, 333), 0, &[h]).st0);
        let yaw = wrap_once(g, img, f, sub(k(g, 0x2c0), r), [0x2d2, 0x2dc, 0x2e6, 0x2f3]);
        g.write::<f32>(o - 0x10, yaw);
        o += 0x130;
        let m = b - 0x38;
        let mode = Frame {
            at: call(0x310),
            esp: lf(0xf0),
            ecx: i,
            edx: m,
            ..*fr
        };
        crate::sa_script::actor_mode(g, img, &mode);
        g.write::<u32>(b + 8, 0);
        let h = target(g, i, (0x328, 0x333));
        // call width: 1
        let r = st0_f32(g.call(t(g, 0x339, 333), 0, &[h]).st0);
        let yaw = wrap_once(g, img, f, sub(k(g, 0x34f), r), [0x361, 0x36b, 0x375, 0x382]);
        g.write::<f32>(b + 0xc, yaw);
        g.write::<u32>(b + 0x10, 0);
        g.write::<u32>(b + 0x14, 0);
        crate::sa_misc::actor_scale(g, img, call(0x39e), i, m);
        for (op, s, at) in [
            (0x3a8u32, 330u32, 0x10u32),
            (0x3c2, 328, 8),
            (0x3de, 329, 0xc),
        ] {
            let a = g.read::<u32>(b + at);
            // call width: 3
            g.call(t(g, op, s), 0, &[m, m, a]);
        }
        let h = target(g, i, (0x404, 0x40f));
        // call width: 4
        g.call(t(g, 0x41e, 262), 0, &[h, b - 8, b - 4, b]);
        g.write::<u32>(b + 4, ONE);
        for (to, from) in [(0x18u32, b - 8), (0x1c, b - 4), (0x20, b), (0x24, b + 4)] {
            let x = g.read::<u32>(from);
            g.write::<u32>(b + to, x);
        }
        for o in [0x30u32, 0x2c, 0x28] {
            g.write::<u32>(b + o, 0);
        }
        // call width: 1
        let r1 = st0_f32(g.call(t(g, 0x447, 314), 0, &[0x41a0_0000]).st0);
        g.write::<f32>(v, r1);
        g.write::<u32>(v + 4, 0);
        // call width: 1
        let r2 = st0_f32(g.call(t(g, 0x476, 404), 0, &[0x41f0_0000]).st0);
        let mut vz = add(r2, k(g, 0x4a5));
        // three points: the place `+0x28..` copied to `P`, moved by `V`
        // turned by `+8` and `+0xc` (`M` for the first two, `N` the third)
        for (p, mat, ops, out) in [
            (
                0x38u32,
                lf(0x54),
                [0x4c6u32, 0x4ea, 0x509, 0x525, 0x541],
                lf(0x84),
            ),
            (
                0x48,
                lf(0x54),
                [0x5c3, 0x5e7, 0x606, 0x61f, 0x637],
                lf(0x74),
            ),
            (
                0x58,
                lf(0xd4),
                [0x66e, 0x690, 0x6b2, 0x6d1, 0x6ec],
                lf(0x94),
            ),
        ] {
            if p == 0x48 {
                // the first point's `y` drops by slot 314 `(20)` + `K`;
                // `V.z` grows by `K'` of itself
                // call width: 1
                let r3 = st0_f32(g.call(t(g, 0x54f, 314), 0, &[0x41a0_0000]).st0);
                let back = add(r3, k(g, 0x57e));
                let x = sub(g.read::<f32>(b + 0x3c), back);
                g.write::<f32>(b + 0x3c, x);
                vz = add(mul(vz, k(g, 0x5ad)), vz);
            }
            let p = b + p;
            for j in 0..4u32 {
                let x = g.read::<u32>(b + 0x28 + 4 * j);
                g.write::<u32>(p + 4 * j, x);
            }
            g.write::<f32>(v + 8, vz);
            // call width: 1
            g.call(t(g, ops[0], 235), 0, &[mat]);
            for (op, s, at) in [(ops[1], 328u32, 8u32), (ops[2], 329, 0xc)] {
                let a = g.read::<u32>(b + at);
                // call width: 3
                g.call(t(g, op, s), 0, &[mat, mat, a]);
            }
            g.write::<u32>(v + 0xc, ONE);
            // call width: 3
            g.call(t(g, ops[3], 229), 0, &[out, mat, v]);
            // call width: 3
            g.call(t(g, ops[4], 248), 0, &[p, p, out]);
            g.write::<u32>(p + 0xc, ONE);
        }
        let z = add(g.read::<f32>(b + 0x5c), k(g, 0x70f));
        g.write::<f32>(b + 0x5c, z);
        for (o, x) in [
            (0x78u32, 0x25u32),
            (0x74, 0x32),
            (0x68, 0),
            (0x6c, 0x3ca3_d70a),
        ] {
            g.write::<u32>(b + o, x);
        }
        // call width: 1
        let r = st0_f32(g.call(t(g, 0x6fb, 404), 0, &[ONE]).st0);
        let wave = sub(mul(r, k(g, 0x758)), k(g, 0x769));
        g.write::<u32>(b + 0x80, 0);
        g.write::<u32>(b + 0x7c, i);
        g.write::<f32>(b + 0x70, wave);
        // the actor's record at B - 0x58
        let rec = b - 0x58;
        g.write::<u32>(rec, w(0x782));
        g.write::<u32>(rec + 4, i);
        let sel = u32::from(g.read::<u8>(gw(g, 0x796).wrapping_add(i)));
        let flag = gw(g, 0x7a0);
        g.write::<u32>(rec + 8, sel);
        let slot = if flag == 0 {
            t(g, 0x7aa, 215)
        } else {
            t(g, 0x7aa, 413)
        };
        // call width: 1
        let c = g.call(slot, 0, &[sel]).eax;
        let flag = gw(g, 0x7c5);
        for (o, x) in [(0xcu32, c), (0x10, u32::MAX), (0x14, 0), (0x18, 1)] {
            g.write::<u32>(rec + o, x);
        }
        g.write::<u16>(rec + 0x1c, u16::from(flag != 0));
        let mut free = g.read::<u16>(w(0x7f4));
        let head = gw(g, 0x807);
        let (n, _) = ring_take(g, img, f, head, &mut free, (0x837, 0x85c, 0x866));
        let first = gw(g, 0x86c);
        g.write::<u8>(n + 0x18, 1);
        g.write::<u32>(n + 0x14, rec);
        let last = gw(g, 0x885);
        task_append(g, img, f, n, first, last, 0x898);
        g.write::<u32>(w(0x8a8), n);
        b += 0xdc;
        i += 1;
    }
    // a block of globals, two task nodes
    let head = gw(g, 0x8c2);
    let last = gw(g, 0x8c8);
    let mut free = g.read::<u16>(w(0x8cf));
    for (o, x) in [
        (0x8d7u32, 0u32),
        (0x8e1, 0),
        (0x8eb, 0),
        (0x8f5, 0),
        (0x8ff, w(0x903)),
        (0x909, w(0x90d)),
        (0x913, 0),
        (0x91d, 0),
        (0x92d, 1),
    ] {
        g.write::<u32>(w(o), x);
    }
    g.write::<u16>(w(0x927), 0);
    g.write::<u16>(w(0x937), 0);
    let (n1, head) = ring_take(g, img, f, head, &mut free, (0x984, 0x9a2, 0x9ad));
    let first = gw(g, 0x9b3);
    g.write::<u32>(n1 + 0x14, w(0x9bb));
    task_append(g, img, f, n1, first, last, 0x9df);
    let first = if first == 0 { n1 } else { gw(g, 0x9cf) };
    g.write::<u32>(w(0x9e8), u32::MAX);
    let (n2, _) = ring_take(g, img, f, head, &mut free, (0xa12, 0xa4b, 0xa56));
    g.write::<u8>(n2 + 0x18, 1);
    task_append(g, img, f, n2, first, n1, 0xa78);
    g.write::<u32>(w(0xa86), n2);
    crate::sa_misc::records_init_100(g, img, call(0xa8b), w(0xa7d));
    crate::o6_fam_pool::pool_reset_take(g, img, call(0xa95), w(0xa90));
    let mut free = g.read::<u16>(w(0xa9c));
    for (o, x) in [
        (0xaa4u32, 0u32),
        (0xaae, 0x3c),
        (0xab8, 0),
        (0xac2, 0x25),
        (0xacc, 0x3c),
        (0xad6, 0x3c),
    ] {
        g.write::<u32>(w(o), x);
    }
    g.write::<u16>(w(0xae0), 0);
    let head = gw(g, 0xaf5);
    let (n3, head) = ring_take(g, img, f, head, &mut free, (0xb1f, 0xb44, 0xb4f));
    let first = gw(g, 0xb55);
    g.write::<u8>(n3 + 0x18, 1);
    g.write::<u32>(n3 + 0x14, w(0xb60));
    let last = gw(g, 0xb70);
    task_append(g, img, f, n3, first, last, 0xb85);
    let first = if first == 0 { n3 } else { first };
    let (n4, _) = ring_take(g, img, f, head, &mut free, (0xbae, 0xbe7, 0xbf2));
    g.write::<u8>(n4 + 0x18, 2);
    task_append(g, img, f, n4, first, n3, 0xc14);
    g.write::<u32>(w(0xc24), n4);
    Ret::eax(0)
}

/// [`start_up_0251`] (the `rel32`s: its callees and the cookie check).
pub const START_UP_0251: Sig = Sig {
    len: 3124,
    hash: 0xd817_f44d_cfb5_43e6,
    calls: &[
        (0x50, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x5b, Callee::Sig(&crate::sb_misc::MEAN_POSITION)),
        (0x74, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (
            0x134,
            Callee::Sig(&crate::o6_fam_pairs5::RECORD_NODE_START_0251_2550),
        ),
        (0x21f, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x310, Callee::Sig(&crate::sa_script::ACTOR_MODE)),
        (0x39e, Callee::Sig(&crate::sa_misc::ACTOR_SCALE)),
        (0xa8b, Callee::Sig(&crate::sa_misc::RECORDS_INIT_100)),
        (
            0xa95,
            Callee::Sig(&crate::o6_fam_pool::POOL_RESET_TAKE_0250_1E00),
        ),
        (0xc2c, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0251`'s load step (0x10004340, 118 bytes; a table entry): the
/// object hosts' load step ([`load_step`](crate::r9_load::load_step))
/// running [`start_up_0251`].
pub fn load_0251<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::r9_load::load_step(g, img, fr, crate::r9_load::Shape::Zero, start_up_0251)
}

/// [`load_0251`] (the 118-byte load step's `Sig`, the `rel32` the
/// start-up).
pub const LOAD_0251: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&START_UP_0251))],
    imports: &[],
};

/// `magic_0385`'s sprite draw (0x10001ed0, 990 bytes; a stored callback,
/// no arguments; an 8-aligned /GS frame, `S` = `esp` after the three
/// pushes). The view `A` (`S + 0x128`, slot 723), the camera `B` (`S +
/// 0xe8`, slot 724), `G_D` = `D` (`S + 0xa8`), the scale matrix `C` (`S +
/// 0x28`, slot 235). For each of the `*G_N` records `R` (0x68 bytes from
/// `*G_R`) with `+0x60` set (each one counted):
/// - while the wait `+0x54` is positive it counts down;
/// - else the size `s` = `+0x1c` scaled: by `(K - x) +0x20 + x` (`x =
///   +0x50 / +0x44`) while `+0x44` is positive, held while `+0x48` is
///   (both with `+0x50` counting up, the count and `+0x50` 0 past it), by
///   `+0x20 x + (K - x)` (`x = +0x50 / +0x4c`) while `+0x4c` is, else by
///   `+0x20`; `C`'s diagonal `s`; the angle `+0x28 += +0x2c` wrapped once
///   and slot 330 `(D, C, angle)`;
/// - the point `P` (`S + 0x18`) = slot 237 `(actor, +0x34, R or 0 by bit
///   1 of byte +0x64, P)`, `w` 1, through `B` into `D`'s last row (slot
///   229); `G_E` 0, or with bit 0 `E` (`S + 0x68`, slot 230 `(E, D)`, its
///   `z` moved by `+0x24`);
/// - the colour: each byte `(a (n - |t|) + b |t|) / n` while `n` (`+0x3c`)
///   is positive (`t` `+0x40` counting up, `n` 0 past it), else `+0x5c`;
/// - the frame ([`anim_step`](crate::sb_common::anim_step) on `R +
///   0x10`); with the life `+0x38` negative and `+0x14` not 1 the life
///   becomes 1, else the part is linked
///   ([`link_part`](crate::sa_misc::link_part) `(G_L, i)`); a positive
///   life counts down, at 0 `+0x60` is cleared.
///
/// The count goes into a global and is returned.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn draw_0385<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    // push ebp; and esp, -8; sub esp, 0x164; three pushes
    let b0 = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x164);
    let cookie = gw(g, 0xd) ^ b0;
    g.write::<u32>(b0 + 0x160, cookie);
    let s = b0.wrapping_sub(0xc);
    let (a, b, c, d, e, p, pt) = (
        s + 0x128,
        s + 0xe8,
        s + 0x28,
        s + 0xa8,
        s + 0x68,
        s + 0x18,
        s + 0xd8,
    );
    let recs = gw(g, 0x1f);
    // call width: 1
    g.call(t(g, 0x2c, 723), 0, &[a]);
    // call width: 1
    g.call(t(g, 0x44, 724), 0, &[b]);
    g.write::<u32>(w(0x58), d);
    // call width: 1
    g.call(t(g, 0x65, 235), 0, &[c]);
    let one = k(g, 0x88);
    let tw = k(g, 0x90);
    let mut drawn = 0u32;
    let mut i = 0u32;
    while i.cast_signed() < gw(g, 0x7e).cast_signed() {
        let r = recs.wrapping_add(i.wrapping_mul(0x68));
        let rd = |g: &G, o: u32| g.read::<f32>(r + o);
        let rw = |g: &G, o: u32| g.read::<i32>(r + o);
        if g.read::<u32>(r + 0x60) == 0 {
            i += 1;
            continue;
        }
        drawn += 1;
        let wait = rw(g, 0x54);
        if wait > 0 {
            g.write::<i32>(r + 0x54, wait.wrapping_sub(1));
            i += 1;
            continue;
        }
        let size0 = rd(g, 0x1c);
        let ratio = |at: i32, n: i32| {
            #[allow(clippy::cast_precision_loss)] // cvtdq2ps
            div(at as f32, n as f32)
        };
        let size = if rw(g, 0x44) > 0 {
            let (n, at) = (rw(g, 0x44), rw(g, 0x50));
            let x = ratio(at, n);
            g.write::<i32>(r + 0x50, at.wrapping_add(1));
            if at.wrapping_add(1) > n {
                g.write::<u32>(r + 0x44, 0);
                g.write::<u32>(r + 0x50, 0);
            }
            mul(add(mul(sub(one, x), rd(g, 0x20)), x), size0)
        } else if rw(g, 0x48) > 0 {
            let at = rw(g, 0x50).wrapping_add(1);
            g.write::<i32>(r + 0x50, at);
            if at > rw(g, 0x48) {
                g.write::<u32>(r + 0x48, 0);
                g.write::<u32>(r + 0x50, 0);
            }
            size0
        } else if rw(g, 0x4c) > 0 {
            let (n, at) = (rw(g, 0x4c), rw(g, 0x50));
            let x = ratio(at, n);
            let s = mul(add(mul(rd(g, 0x20), x), sub(one, x)), size0);
            g.write::<i32>(r + 0x50, at.wrapping_add(1));
            if at.wrapping_add(1) > n {
                g.write::<u32>(r + 0x4c, 0);
            }
            s
        } else {
            mul(rd(g, 0x20), size0)
        };
        let ang = add(rd(g, 0x2c), rd(g, 0x28));
        for o in [0x28u32, 0x14, 0] {
            g.write::<f32>(c + o, size);
        }
        let ang = if ang > k(g, 0x1cf) {
            sub(ang, tw)
        } else if k(g, 0x1df) > ang {
            add(ang, tw)
        } else {
            ang
        };
        g.write::<f32>(r + 0x28, ang);
        // call width: 3
        g.call(t(g, 0x200, 330), 0, &[d, c, ang.to_bits()]);
        let flags = g.read::<u8>(r + 0x64);
        let owner = if flags & 2 == 0 { 0 } else { r };
        let who = g.read::<u32>(r + 0x30);
        let h = if who == u32::MAX {
            gw(g, 0x228)
        } else {
            let sel = g.read::<u8>(gw(g, 0x22f).wrapping_add(who));
            g.read::<u32>(w(0x23a).wrapping_add(4 * u32::from(sel)))
        };
        let bone = g.read::<u32>(r + 0x34);
        // call width: 4
        g.call(t(g, 0x249, 237), 0, &[h, bone, owner, p]);
        g.write::<u32>(p + 0xc, ONE);
        // call width: 3
        g.call(t(g, 0x26b, 229), 0, &[pt, b, p]);
        g.write::<u32>(w(0x284), 0);
        if flags & 1 != 0 {
            // call width: 2
            g.call(t(g, 0x2a0, 230), 0, &[e, d]);
            let z = add(rd(g, 0x24), g.read::<f32>(e + 0x38));
            g.write::<u32>(w(0x2c2), e);
            g.write::<f32>(e + 0x38, z);
        }
        let n = rw(g, 0x3c);
        if n > 0 {
            let tt = rw(g, 0x40).wrapping_abs();
            for (j, op) in (0u32..).zip([0x2fbu32, 0x319, 0x337, 0x359]) {
                let from = i32::from(g.read::<u8>(r + 0x58 + j));
                let to = i32::from(g.read::<u8>(r + 0x5c + j));
                let v = from
                    .wrapping_mul(n.wrapping_sub(tt))
                    .wrapping_add(to.wrapping_mul(tt))
                    / n;
                g.write::<u8>(w(op), v.to_le_bytes()[0]);
            }
            let tt = rw(g, 0x40).wrapping_add(1);
            g.write::<i32>(r + 0x40, tt);
            if tt > rw(g, 0x3c) {
                g.write::<u32>(r + 0x3c, 0);
            }
        } else {
            let x = g.read::<u32>(r + 0x5c);
            g.write::<u32>(w(0x375), x);
        }
        let entry = crate::sb_common::anim_step(g, img, call(0x37d), r + 0x10);
        if rw(g, 0x38) < 0 && g.read::<u32>(r + 0x14) != 1 {
            g.write::<u32>(r + 0x38, 1);
        } else {
            // push i; push G_L; the call's return address below them
            let esp = s.wrapping_sub(0xc);
            g.write::<u32>(esp + 4, w(0x398));
            g.write::<u32>(esp + 8, i);
            let link = Frame {
                at: call(0x3a2),
                esp,
                ecx: r + 0x10,
                edx: entry,
                ..*fr
            };
            crate::sa_misc::link_part(g, img, &link);
        }
        let life = rw(g, 0x38);
        if life > 0 {
            g.write::<i32>(r + 0x38, life - 1);
            if life == 1 {
                g.write::<u32>(r + 0x60, 0);
            }
        }
        i += 1;
    }
    g.write::<u32>(w(0xce), drawn);
    Ret::eax(drawn)
}

/// [`draw_0385`] (the `rel32`s: the frame step, the part link, the
/// cookie check).
pub const DRAW_0385: Sig = Sig {
    len: 990,
    hash: 0x91e9_1f09_e6c9_02b8,
    calls: &[
        (0xd3, Callee::Any),
        (0x37d, Callee::Sig(&crate::sb_common::ANIM_STEP)),
        (0x3a2, Callee::Sig(&crate::sa_misc::LINK_PART)),
    ],
    imports: &[],
};

/// `magic_0385`'s start-up (0x10003f00, 2922 bytes; internal, no
/// arguments, run by its load step; a /GS `ebp` frame, the point `V` at
/// `ebp - 0x14`):
/// - three words, `*G_C` = slot 719 `(D, 8, 0)`; the part set-up
///   ([`part_init`](crate::l5_objhost::part_init)); slot 333 `(*G_A)`
///   (its answer dropped);
/// - 22 objects in pairs (0x130 apart; [`start_object_at`](crate::s4_misc::start_object_at)
///   `(1, 0x28 + 4 j, -, 0)`), each with its number at `+0x120`, the step
///   `CB` at `+0x104` and byte `+0x126` 2;
/// - per actor `i` (`*G_N`; 0x390 apart): `V` = slot 262 of its handle
///   (`w` 1), three objects `(0, o, -, CB2 or CB1)` (`o` from 0x8c up by
///   `13 + rand() % 8` each; `CB2` for the third), each with three
///   random spins (slot 314), `V` at `+0xd0` and the actor's pick
///   ([`pick`](crate::m0074::pick)) at `+0x120`;
/// - 64 records cleared and their counts; the sprite pool
///   ([`pool_reset_take`](crate::o6_fam_pool::pool_reset_take));
/// - 22 sprite records taken from the ring (`*G_R`, `*G_N`, the rover
///   `*G_I`; none left ends it): colours, sizes, the frame cursor
///   (`*G_F + 0xc`), a bone `TAB[j]`, waits 0, 0, 2, 2, 4, ...;
/// - blocks of globals and eight task nodes off the free ring: two marked
///   1 with their records, three with records and no mark, the camera
///   words (slot 413 or 215 of `*G_X` by a flag), then one marked 1, one
///   unmarked and one marked 2.
///
/// Returns 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn start_up_0385<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let lf = |o: u32| ebp.wrapping_sub(o);
    let cookie = gw(g, 0x7) ^ ebp;
    g.write::<u32>(lf(4), cookie);
    for (to, v) in [(0x29u32, 0x2du32), (0x33, 0x37), (0x3d, 0x41)] {
        g.write::<u32>(w(to), w(v));
    }
    // call width: 3
    let c = g.call(t(g, 0x11, 719), 0, &[w(0x23), 8, 0]).eax;
    g.write::<u32>(w(0x4b), c);
    crate::l5_objhost::part_init(g, img, call(0x50));
    let a = gw(g, 0x5b);
    // call width: 1 (`fstp st(0)`: the answer is dropped)
    g.call(t(g, 0x55, 333), 0, &[a]);
    let start =
        |g: &mut G, (at, obj, scale): (u32, u32, f32), (index, owner, b): (u32, u32, u32)| {
            crate::s4_misc::start_object_at(g, img, call(at), obj, scale, index, owner, b);
        };
    // the 22 objects
    let mut o = w(0x6d);
    let mut n = 0u32;
    let mut owner = 0x28u32;
    for _ in 0..0xb {
        let scale = k(g, 0x84);
        start(g, (0x91, o, scale), (1, owner, 0));
        let scale = k(g, 0x99);
        g.write::<u32>(o + 0x120, n);
        g.write::<u32>(o + 0x104, w(0xb8));
        g.write::<u8>(o + 0x126, 2);
        n += 1;
        start(g, (0xc5, o + 0x130, scale), (1, owner, 0));
        g.write::<u32>(o + 0x250, n);
        g.write::<u32>(o + 0x234, w(0xd5));
        g.write::<u8>(o + 0x256, 2);
        o += 0x260;
        n += 1;
        owner += 4;
    }
    // the actors' objects
    let v = lf(0x14);
    let mut i = 0u32;
    while i.cast_signed() < gw(g, 0xf6).cast_signed() {
        let mut o = i.wrapping_mul(0x390).wrapping_add(w(0x10a));
        let mut id = 0x8cu32;
        let h = {
            let b = g.read::<u8>(gw(g, 0x122).wrapping_add(i));
            g.read::<u32>(w(0x12d).wrapping_add(4 * u32::from(b)))
        };
        // call width: 4
        g.call(t(g, 0x13f, 262), 0, &[h, v, v + 4, v + 8]);
        g.write::<u32>(v + 0xc, ONE);
        for j in 0..3u32 {
            let r = g.rand().cast_signed() % 8;
            id = id.wrapping_add(0xd).wrapping_add(r.cast_unsigned());
            let scale = k(g, 0x170);
            let cb = if j == 2 { w(0x17a) } else { w(0x17f) };
            start(g, (0x194, o, scale), (0, id, cb));
            for (op, x, at) in [
                (0x199u32, 0x3e32_b8c3u32, 0xc0u32),
                (0x1b6, 0x40c9_0fdb, 0xc4),
                (0x1d0, 0x3e32_b8c3, 0xc8),
            ] {
                // call width: 1
                let r = st0_f32(g.call(t(g, op, 314), 0, &[x]).st0);
                g.write::<f32>(o + at, r);
            }
            for q in 0..4u32 {
                let x = g.read::<u32>(v + 4 * q);
                g.write::<u32>(o + 0xd0 + 4 * q, x);
            }
            let p = crate::m0074::pick(g, img, call(0x223), i);
            g.write::<u32>(o + 0x120, p);
            o += 0x130;
        }
        i += 1;
    }
    // the records, the sprite pool
    g.write::<u32>(w(0x24c), w(0x250));
    let mut rec = w(0x255);
    for _ in 0..0x40 {
        g.write::<u32>(rec, 0);
        rec = rec.wrapping_add(0x140);
    }
    g.write::<u32>(w(0x271), 0);
    g.write::<u32>(w(0x27c), 0x40);
    crate::o6_fam_pool::pool_reset_take(g, img, call(0x285), w(0x276));
    // the sprite records
    let count = gw(g, 0x28b).cast_signed();
    let base = gw(g, 0x291);
    let mut rover = gw(g, 0x297);
    let mut wait = 0u32;
    'recs: for j in 0..0x16u32 {
        let mut tried = 0i32;
        let r = loop {
            if tried >= count {
                break 'recs;
            }
            let r = rover.wrapping_mul(0x68).wrapping_add(base);
            rover = rover.wrapping_add(1);
            if rover.cast_signed() >= count {
                rover = 0;
            }
            if g.read::<u32>(r + 0x60) == 0 {
                break r;
            }
            tried += 1;
        };
        for (o, x) in [
            (0x58u32, 0x8080_8080u32),
            (0x5c, 0x8080_8080),
            (0x54, 0),
            (0x40, 0),
            (0x20, ONE),
            (0x60, 1),
        ] {
            g.write::<u32>(r + o, x);
        }
        g.write::<u16>(r + 0x64, 0);
        g.write::<u32>(r + 0x44, 0);
        g.write::<u32>(r + 0x2c, 0);
        let fr0 = gw(g, 0x30c);
        g.write::<u32>(w(0x312), rover);
        let x = g.read::<u32>(fr0 + 0xc);
        g.write::<u32>(r + 0x10, x);
        g.write::<u32>(r + 0x14, 1);
        g.write::<u16>(r + 0x18, 0xffff);
        g.write::<u16>(r + 0x1a, 0x200);
        for (o, x) in [
            (0x1cu32, 0x3e4c_cccdu32),
            (0x28, 0),
            (0x2c, 0),
            (0x30, u32::MAX),
        ] {
            g.write::<u32>(r + o, x);
        }
        let bone = g.read::<u32>(w(0x352).wrapping_add(4 * j));
        g.write::<u32>(r + 0x34, bone);
        for (o, x) in [
            (0x38u32, 0x28u32),
            (0x3c, 0),
            (0x20, 0),
            (0x44, 0xf),
            (0x48, 0xa),
            (0x4c, 0xf),
            (0x50, 0),
            (0x54, wait),
        ] {
            g.write::<u32>(r + o, x);
        }
        if j & 1 != 0 {
            wait = wait.wrapping_add(2);
        }
    }
    // blocks of globals and the task nodes
    let put = |g: &mut G, list: &[(u32, u32)]| {
        for &(to, v) in list {
            g.write::<u32>(w(to), v);
        }
    };
    let mut free = g.read::<u16>(w(0x3a8));
    put(
        g,
        &[
            (0x3b0, 0),
            (0x3ba, 0x50),
            (0x3c4, 0),
            (0x3ce, 0x90),
            (0x3d8, 0x4c),
            (0x3e2, 0x4c),
        ],
    );
    g.write::<u16>(w(0x3ec), 0);
    let head = gw(g, 0x406);
    let (na, head) = ring_take(g, img, f, head, &mut free, (0x42b, 0x450, 0x45b));
    let first = gw(g, 0x461);
    g.write::<u8>(na + 0x18, 1);
    g.write::<u32>(na + 0x14, w(0x46d));
    let last = gw(g, 0x47b);
    task_append(g, img, f, na, first, last, 0x48e);
    put(
        g,
        &[
            (0x499, w(0x49d)),
            (0x4a3, 0),
            (0x4ad, 0),
            (0x4b7, 0x3c57_9436),
            (0x4c1, 0),
            (0x4cb, 0x4c),
        ],
    );
    g.write::<u16>(w(0x4d5), 0);
    let (nb, head) = ring_take(g, img, f, head, &mut free, (0x519, 0x537, 0x542));
    let first = gw(g, 0x548);
    g.write::<u8>(nb + 0x18, 1);
    g.write::<u32>(nb + 0x14, w(0x554));
    task_append(g, img, f, nb, first, na, 0x578);
    let first = if first == 0 { nb } else { gw(g, 0x568) };
    let x = g.read::<u16>(w(0x589)) | 3;
    g.write::<u16>(w(0x589), x);
    for q in 0..4u32 {
        let x = g.read::<u32>(w(0x57f) + 4 * q);
        g.write::<u32>(w(0x593) + 4 * q, x);
    }
    put(
        g,
        &[
            (0x599, 0x4080_0000),
            (0x5a3, 0),
            (0x5ad, 0x90),
            (0x5b7, w(0x5bb)),
            (0x5c1, 0),
            (0x5cb, 0x8c),
            (0x5d5, 0x40),
            (0x5df, 0x4000_0000),
            (0x5e9, 0x4000_0000),
            (0x5f3, 0x4000_0000),
            (0x5fd, 0x20),
            (0x607, 0),
            (0x611, 0x10),
            (0x61b, 0x6e),
            (0x625, u32::MAX),
            (0x62f, ONE),
            (0x639, ONE),
            (0x643, ONE),
        ],
    );
    g.write::<u16>(w(0x64d), 0);
    let (nc, head) = ring_take(g, img, f, head, &mut free, (0x699, 0x6b7, 0x6c2));
    g.write::<u32>(nc + 0x14, w(0x6c9));
    task_append(g, img, f, nc, first, nb, 0x6ef);
    let first = if first == 0 { nc } else { gw(g, 0x6df) };
    put(
        g,
        &[
            (0x6fa, 0),
            (0x704, 0),
            (0x70e, 0),
            (0x718, 0),
            (0x722, w(0x726)),
            (0x72c, w(0x730)),
            (0x736, 0),
            (0x740, 0),
            (0x751, 1),
        ],
    );
    g.write::<u16>(w(0x74b), 0);
    g.write::<u16>(w(0x75c), 0);
    let (nd, head) = ring_take(g, img, f, head, &mut free, (0x7a3, 0x7c1, 0x7cc));
    g.write::<u32>(nd + 0x14, w(0x7d3));
    task_append(g, img, f, nd, first, nc, 0x7f3);
    let first = if first == 0 { nd } else { first };
    put(
        g,
        &[
            (0x7fe, u32::MAX),
            (0x808, 0),
            (0x812, 0),
            (0x81c, 0),
            (0x826, 0),
            (0x830, w(0x834)),
            (0x83a, w(0x83e)),
            (0x844, 0),
            (0x84e, 0),
            (0x85e, 2),
        ],
    );
    g.write::<u16>(w(0x858), 0);
    g.write::<u16>(w(0x868), 0);
    let (ne, _) = ring_take(g, img, f, head, &mut free, (0x88e, 0x8c7, 0x8d2));
    g.write::<u32>(ne + 0x14, w(0x8d9));
    task_append(g, img, f, ne, first, nd, 0x8f7);
    // the camera words
    let flag = gw(g, 0x900);
    let x = gw(g, 0x906);
    g.write::<u32>(w(0x90b), x);
    g.write::<u32>(w(0x917), ne);
    g.write::<u32>(w(0x91d), w(0x921));
    g.write::<u32>(w(0x927), u32::MAX);
    let slot = if flag == 0 {
        t(g, 0x911, 215)
    } else {
        t(g, 0x911, 413)
    };
    // call width: 1
    let c = g.call(slot, 0, &[x]).eax;
    let flag = gw(g, 0x946);
    put(g, &[(0x951, c), (0x957, u32::MAX), (0x961, 0), (0x96b, 1)]);
    if flag == 0 {
        g.write::<u16>(w(0x975), 0);
    } else {
        g.write::<u16>(w(0x97e), 1);
    }
    let mut free = g.read::<u16>(w(0x985));
    let head = gw(g, 0x99a);
    let (nf, head) = ring_take(g, img, f, head, &mut free, (0x9c4, 0x9e9, 0x9f4));
    let first = gw(g, 0x9fa);
    g.write::<u8>(nf + 0x18, 1);
    let last = gw(g, 0xa0d);
    task_append(g, img, f, nf, first, last, 0xa20);
    let (ng, head) = ring_take(g, img, f, head, &mut free, (0xa67, 0xa85, 0xa90));
    let first = gw(g, 0xa96);
    task_append(g, img, f, ng, first, nf, 0xabb);
    let first = if first == 0 { ng } else { gw(g, 0xaab) };
    let (nh, _) = ring_take(g, img, f, head, &mut free, (0xae4, 0xb1d, 0xb28));
    g.write::<u8>(nh + 0x18, 2);
    task_append(g, img, f, nh, first, ng, 0xb4a);
    g.write::<u32>(w(0xb5a), nh);
    Ret::eax(0)
}

/// [`start_up_0385`] (the `rel32`s: its callees and the cookie check;
/// `rand` through the IAT).
pub const START_UP_0385: Sig = Sig {
    len: 2922,
    hash: 0x590c_9ade_d57a_004f,
    calls: &[
        (0x50, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x91, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0xc5, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x194, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x223, Callee::Sig(&crate::m0074::PICK)),
        (
            0x285,
            Callee::Sig(&crate::o6_fam_pool::POOL_RESET_TAKE_0385_1DF0),
        ),
        (0xb62, Callee::Any),
    ],
    imports: &[(0x159, "rand")],
};

/// `magic_0385`'s load step (0x10003dd0, 118 bytes; a table entry): the
/// object hosts' load step ([`load_step`](crate::r9_load::load_step))
/// running [`start_up_0385`].
pub fn load_0385<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::r9_load::load_step(g, img, fr, crate::r9_load::Shape::Zero, start_up_0385)
}

/// [`load_0385`] (the 118-byte load step's `Sig`, the `rel32` the
/// start-up).
pub const LOAD_0385: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&START_UP_0385))],
    imports: &[],
};

/// `magic_0348`'s shard set-up from a model (0x10005360, 1405 bytes;
/// internal: `ecx` = the model `M`, `edx` = the shard records `O`, 0x7c
/// bytes each; an 8-aligned /GS frame, `S` = `esp` after the two
/// pushes). `M + [M + 4]` holds the groups, `M + [M + 8]` the vertices
/// (three `i16` each), `i16 M + 0x10` the faces in all. For each group
/// `G` (byte `+1` must be 2, else it returns that byte less 2) and each
/// of its `i16 +2` faces (0x20 bytes from `G + 0x10`, the corners at
/// `u16 +0xc`, `+0xe`, `+0x10`) one record:
/// - the corners' mean (divided by `K`) and `w` 1, a unit `+0x1c`;
/// - `+0x34` = `-(slot 314 (30) + K1)`, `+0x44` =
///   `-((slot 314 (0.0025) + K2) * +0x34)`;
/// - the spins `+0x50`, `+0x54` = slot 404 `(0.04)` plus `K3`, each
///   negated by `K4` on an even `rand()`, and `+0x60`, `+0x64` =
///   `-((slot 404 (0.005) + K2) * spin)`;
/// - the lives `+0x74`, `+0x78` = `50 + rand() % 11`.
///
/// Per group the counts (`3 n`, `3 n`, 1) go into three frame arrays and
/// slot 885 `(lo & 0x3fff, (Q >> 20) & 0x3f, 0, 0, 1 << ((Q >> 26) &
/// 0xf), 1 << ((Q >> 30) & 0xf), P)` (`Q` the group's 64-bit `+8`, `P`
/// the group's 0x100-byte frame buffer; the lock held) must answer
/// non-zero, else it returns 0. Then the 64-bit handle `H` = slot 772
/// `(0x15c, M)`: slot 881 `(H, slot 776 (0x48) | 8)`, slot 882 `(H,
/// groups, counts, counts, buffers, ones)`, slot 883 `(H)` (non-zero:
/// returned) and slot 779 `(H)` with the lock held. Returns the lock's
/// global word.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn shards_0348<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let (m, mut out) = (fr.ecx, fr.edx);
    // push ebp; and esp, -8; sub esp, 0x568; two pushes
    let b0 = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x568);
    let cookie = gw(g, 0xd) ^ b0;
    g.write::<u32>(b0 + 0x564, cookie);
    let s = b0.wrapping_sub(8);
    let verts = g.read::<u32>(m + 8).wrapping_add(m);
    let mut grp = g.read::<u32>(m + 4).wrapping_add(m);
    // the three count arrays (`A`, `B`, ones `C`), five words each
    let (ca, cb, cc) = (s + 0x40, s + 0x2c, s + 0x54);
    for i in 0..5u32 {
        g.write::<u32>(ca + 4 * i, 0);
        g.write::<u32>(cb + 4 * i, 0);
        g.write::<u32>(cc + 4 * i, u32::from(i == 0));
    }
    let mut left = i32::from(g.read::<i16>(m + 0x10));
    let corner = |g: &G, n: u16| {
        let v = verts.wrapping_add(6 * u32::from(n));
        // cvtdq2ps of each i16: exact
        [0u32, 2, 4].map(|o| f32::from(g.read::<i16>(v + o)))
    };
    let neg = |x: f32| f32::from_bits(x.to_bits() ^ 0x8000_0000);
    let mut groups = 0u32;
    if left > 0 {
        let mut buf = s + 0x68;
        loop {
            let kind = i32::from(g.read::<i8>(grp + 1)).wrapping_sub(2);
            if kind != 0 {
                return Ret::eax(kind.cast_unsigned());
            }
            let n = i32::from(g.read::<i16>(grp + 2));
            let mut face = grp + 0x10;
            let mut j = 0i32;
            while j < n {
                let a = corner(g, g.read::<u16>(face + 0xc));
                let b = corner(g, g.read::<u16>(face + 0xe));
                let c = corner(g, g.read::<u16>(face + 0x10));
                let third = k(g, 0x120);
                for q in 0..3usize {
                    let x = div(add(add(b[q], a[q]), c[q]), third);
                    g.write::<f32>(out + 4 * q as u32, x);
                }
                for (o, x) in [
                    (0xcu32, ONE),
                    (0x10, 0),
                    (0x14, 0),
                    (0x18, 0),
                    (0x1c, ONE),
                    (0x30, 0),
                ] {
                    g.write::<u32>(out + o, x);
                }
                // call width: 1
                let r = st0_f32(g.call(t(g, 0x180, 314), 0, &[0x41f0_0000]).st0);
                for o in [0x38u32, 0x3c, 0x40] {
                    g.write::<u32>(out + o, 0);
                }
                let up = neg(add(r, k(g, 0x1e7)));
                g.write::<f32>(out + 0x34, up);
                // call width: 1
                let r = st0_f32(g.call(t(g, 0x1d5, 314), 0, &[0x3b23_d70a]).st0);
                for o in [0x48u32, 0x4c, 0x20, 0x24, 0x28] {
                    g.write::<u32>(out + o, 0);
                }
                let x = neg(mul(add(r, k(g, 0x229)), g.read::<f32>(out + 0x34)));
                g.write::<f32>(out + 0x44, x);
                g.write::<u32>(out + 0x2c, 0);
                for (op, ko, at) in [(0x22eu32, 0x28fu32, 0x50u32), (0x27d, 0x2b5, 0x54)] {
                    // call width: 1
                    let r = st0_f32(g.call(t(g, op, 404), 0, &[0x3d23_d70a]).st0);
                    g.write::<f32>(out + at, add(r, k(g, ko)));
                }
                g.write::<u32>(out + 0x58, 0);
                g.write::<u32>(out + 0x5c, 0);
                for (ko, at) in [(0x2ecu32, 0x50u32), (0x312, 0x54)] {
                    if g.rand().cast_signed() % 2 == 0 {
                        let x = mul(g.read::<f32>(out + at), k(g, ko));
                        g.write::<f32>(out + at, x);
                    }
                }
                for (op, ko, at, spin) in [
                    (0x31cu32, 0x33eu32, 0x60u32, 0x50u32),
                    (0x343, 0x375, 0x64, 0x54),
                ] {
                    // call width: 1
                    let r = st0_f32(g.call(t(g, op, 404), 0, &[0x3ba3_d70a]).st0);
                    if at == 0x64 {
                        g.write::<u32>(out + 0x68, 0);
                        g.write::<u32>(out + 0x6c, 0);
                    }
                    let x = neg(mul(add(r, k(g, ko)), g.read::<f32>(out + spin)));
                    g.write::<f32>(out + at, x);
                }
                let life = (g.rand().cast_signed() % 11)
                    .wrapping_add(0x32)
                    .cast_unsigned();
                g.write::<u32>(out + 0x78, life);
                g.write::<u32>(out + 0x74, life);
                out += 0x7c;
                face += 0x20;
                j += 1;
            }
            let n = i32::from(g.read::<i16>(grp + 2));
            left = left.wrapping_sub(n);
            let n3 = n.wrapping_mul(3).cast_unsigned();
            g.write::<u32>(ca + 4 * groups, n3);
            g.write::<u32>(cb + 4 * groups, n3);
            g.write::<u32>(cc + 4 * groups, 1);
            let q = (u64::from(g.read::<u32>(grp + 0xc)) << 32) | u64::from(g.read::<u32>(grp + 8));
            let lock = gw(g, 0x3fc);
            let saved = g.read::<u32>(lock);
            g.write::<u32>(lock, 1);
            #[allow(clippy::cast_possible_truncation)] // the fields' low words
            let field = |sh: u32, mask: u64| ((q >> sh) & mask) as u32;
            let e1 = 1u32 << field(30, 0xf);
            let e2 = 1u32 << field(26, 0xf);
            let words = [
                field(0, 0x3fff),
                field(20, 0x3f),
                0,
                0,
                e2 & 0xffff,
                e1 & 0xffff,
                buf,
            ];
            // call width: 7
            let ok = g.call(t(g, 0x45f, 885), 0, &words).eax;
            let lock = gw(g, 0x477);
            g.write::<u32>(lock, saved);
            if ok == 0 {
                return Ret::eax(0);
            }
            buf += 0x100;
            grp = face;
            groups += 1;
            if left <= 0 {
                break;
            }
        }
    }
    // call width: 2
    let h = g.call(t(g, 0x4bb, 772), 0, &[0x15c, m]);
    let (lo, hi) = (h.eax, h.edx);
    // call width: 1
    let e = g.call(t(g, 0x4cf, 776), 0, &[0x48]).eax;
    // call width: 3
    g.call(t(g, 0x4eb, 881), 0, &[lo, hi, e | 8]);
    // call width: 7
    g.call(t(g, 0x514, 882), 0, &[lo, hi, groups, ca, cb, s + 0x68, cc]);
    // call width: 2
    let r = g.call(t(g, 0x528, 883), 0, &[lo, hi]).eax;
    if r != 0 {
        return Ret::eax(r);
    }
    let lock = gw(g, 0x541);
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    // call width: 2
    g.call(t(g, 0x552, 779), 0, &[lo, hi]);
    let lock = gw(g, 0x560);
    g.write::<u32>(lock, saved);
    Ret::eax(lock)
}

/// [`shards_0348`] (the `rel32`: the cookie check).
pub const SHARDS_0348: Sig = Sig {
    len: 1405,
    hash: 0xe439_63b1_0a5e_a9e8,
    calls: &[(0x575, Callee::Any)],
    imports: &[(0x2d1, "rand"), (0x2f7, "rand"), (0x39d, "rand")],
};

/// `magic_0348`'s shard start (0x10006c70, 46 bytes; a table entry, no
/// arguments): the model slot 719 `(PART, 4, 0)` gives, the records at
/// `*G_S + 0xd30`, run by [`shards_0348`] (a tail jump).
pub fn shards_start_0348<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let s = g.read::<u32>(w(0xe));
    // call width: 3
    let m = g.call(slot_target(g, w(0x1), 719), 0, &[w(0x17), 4, 0]).eax;
    let callee = Frame {
        at: img.rel32_target(f + 0x2a),
        ecx: m,
        edx: s.wrapping_add(0xd30),
        ..*fr
    };
    shards_0348(g, img, &callee)
}

/// [`shards_start_0348`] (the `rel32` the tail jump).
pub const SHARDS_START_0348: Sig = Sig {
    len: 46,
    hash: 0xe3e_43d7_c685_feb2,
    calls: &[(0x2a, Callee::Sig(&SHARDS_0348))],
    imports: &[],
};

/// `magic_0297`'s summon node visit (0x10002f70, 438 bytes; thiscall,
/// `ecx` = the group; a /GS `ebp` frame, the place `L` at `ebp - 0x14`,
/// a group of its own at `ebp - 0x1c`). The node walk
/// ([`walk`](crate::nodes::walk)); when it ran (the flag set), for the
/// owner `N`: its camera node `C` (`+0x14c`) visited through its vtable
/// with the local group (`C`, the mode), `L` = `C`'s facing
/// ([`facing_int`](crate::sa_misc::facing_int)) with `x`, `z` 0, the
/// shape function `TAB[byte N+0x148]` `(N + 0x5c, L)` (`ecx` = `L`),
/// slot 233 `(N + 0x1c + 0x10 i, N + 0x5c + 0x10 i, 0.001)` for `i` 0..3,
/// slot 262 `(C+0x54's handle, N + 0x4c, +0x50, +0x54)` with `w` 1, `L`
/// into `+0x11c`, slot 325 `(N + 0x12c, G_V)`, `+0x138` scaled by `K`,
/// slot 230 `(N + 0xdc, N + 0x5c)` and `(N + 0x9c, N + 0x1c)`. Returns
/// the last slot's `eax`, else the walk's.
pub fn visit_0297<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let ebp = fr.esp.wrapping_sub(4);
    let cookie = g.read::<u32>(w(0x7)) ^ ebp;
    g.write::<u32>(ebp.wrapping_sub(4), cookie);
    let this = fr.ecx;
    // (`eax` holds the cookie when the walk starts)
    let (eax, tail) = crate::nodes::walk(g, this, this, cookie);
    let Some(n) = tail else {
        return Ret::eax(eax);
    };
    let c = g.read::<u32>(n + 0x14c);
    let mode = g.read::<u8>(this + 5);
    let grp = ebp.wrapping_sub(0x1c);
    g.write::<u32>(grp, c);
    g.write::<u8>(grp + 5, mode);
    let method = g.read::<u32>(g.read::<u32>(c) + 0x1c);
    g.call(method, grp, &[grp]);
    let c = g.read::<u32>(n + 0x14c);
    let l = ebp.wrapping_sub(0x14);
    // three pushes and the visit's word under the call: its return
    // address at ebp - 0x30
    crate::sa_misc::facing_int(
        g,
        img,
        img.rel32_target(f + 0x80),
        c,
        l,
        ebp.wrapping_sub(0x30),
    );
    g.write::<u32>(l + 8, 0);
    g.write::<u32>(l, 0);
    let shape = g.read::<u32>(w(0x9f) + 4 * u32::from(g.read::<u8>(n + 0x148)));
    let v = n + 0x5c;
    g.call(shape, l, &[v, l]);
    for (op, i) in [(0xabu32, 0u32), (0xd9, 1), (0xf8, 2)] {
        // call width: 3
        g.call(
            t(g, op, 233),
            0,
            &[n + 0x1c + 0x10 * i, v + 0x10 * i, 0x3a83_126f],
        );
    }
    let h = g.read::<u32>(g.read::<u32>(n + 0x14c) + 0x54);
    let p = n + 0x4c;
    // call width: 4
    g.call(t(g, 0x116, 262), 0, &[h, p, p + 4, p + 8]);
    g.write::<u32>(n + 0x58, ONE);
    for i in 0..4u32 {
        let x = g.read::<u32>(l + 4 * i);
        g.write::<u32>(n + 0x11c + 4 * i, x);
    }
    // call width: 2
    g.call(t(g, 0x14c, 325), 0, &[n + 0x12c, w(0x146)]);
    let x = mul(g.read::<f32>(n + 0x138), g.read::<f32>(w(0x16c)));
    g.write::<f32>(n + 0x138, x);
    // call width: 2
    g.call(t(g, 0x179, 230), 0, &[n + 0xdc, v]);
    // call width: 2
    let eax = g.call(t(g, 0x196, 230), 0, &[n + 0x9c, n + 0x1c]).eax;
    Ret::eax(eax)
}

/// [`visit_0297`] (the `rel32`s: the facing, the cookie check).
pub const VISIT_0297: Sig = Sig {
    len: 438,
    hash: 0x8ae4_36dc_be2a_9e81,
    calls: &[
        (0x80, Callee::Sig(&crate::sa_misc::FACING_INT)),
        (0x1ae, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0297`'s visit thunk (0x10003130, 12 bytes; a vtable's `+0x1c`):
/// its stack word into `ecx`, then [`visit_0297`] (a tail jump).
pub fn visit_thunk_0297<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let callee = Frame {
        at: img.rel32_target(fr.at + 8),
        ecx: fr.arg(g, 0),
        ..*fr
    };
    visit_0297(g, img, &callee)
}

/// [`visit_thunk_0297`].
pub const VISIT_THUNK_0297: Sig = Sig {
    len: 12,
    hash: 0x896f_4fde_1224_a07f,
    calls: &[(8, Callee::Sig(&VISIT_0297))],
    imports: &[],
};

/// `magic_0297`'s image node visit (0x100034f0, 2028 bytes; thiscall,
/// `ecx` = the group; a 16-aligned /GS frame, `S` = `esp` after the two
/// pushes). The node walk ([`walk`](crate::nodes::walk), `ecx` the owner
/// for every visit); when it ran, for the owner `N`:
/// - unpaused (`*G_P` 0): its life `u16 +0xba` counts down; at 0 the node
///   leaves ([`leave`](crate::sa_misc::leave)) and its model (`*(+0x88) +
///   0x54`) is released (slots 357 and 356 `(h, -1, -1)`, the end); with
///   bit 2 of `u16 +0xb4` the scale `+0x94` steps by `+0xa0` up to the
///   limit `+0x9c` (then the bit is cleared);
/// - two colours 1.0 in the frame; with bit 1, slots 355 and 354 `(h,
///   -1)`;
/// - with bit 3 the fade: (unpaused) `+0xa4 -= +0xa8`, `c = (+0xa4 K)`
///   truncated, `b = u16 +0xb8`: the bytes `3 + (i & 0xe0) +
///   TAB[i & 0x1f]` of slot 941 `(+0xac)`'s image get `(i - c) / b K`
///   (truncated) for `i` from `max(c, 0)` below `b + c` and 0x100, then 0
///   up to 0x100; the image is loaded (slot 425 `(N + 0x1c, u16 +0xb0, 4,
///   byte +0xbc, 0, 0, 16, 16)`, slot 421 `(0)`, slot 423 `(N + 0x1c,
///   +0xac)`); with nothing left drawn the node leaves its parent
///   ([`leave_parent`](crate::nodes::leave_parent)) and is released as
///   above;
/// - else the draw: the view rows (slot 544 `(i, &M[i])`), the camera rows
///   (slot 588, `w` 0), slot 327 `(V, *(+0x8c) + 0x9c)`; `V`'s three rows
///   normalised and negated (slots 232, 233 `-1`), `R` rows through `V`
///   (slot 229), `V = V G * (+0x8c's)` (slot 231); the scale slot 277
///   `(h)`; the packet record (`+0x80`, `+0x84`, `M`, `R`, the colours,
///   `V`, the model `+0x90`, `+0x78` of `*G_C`, the sizes `(+0x98 s K +
///   +0x94 s K)` and `+0x94 s K`) for the mesh packets
///   ([`mesh_packets_0426`](crate::r26::mesh_packets_0426)); the fade
///   floats (`c` and `b`, or -99999 each) and the sizes over `K'` for
///   slot 857 `(h, NAME, ...)`; slot 885 `(u16 +0x80 & 0x3fff, 0x13, 0,
///   0, 0, 0, &BUF)` with the lock held, and slot 906 `(h, NAME2, BUF or
///   0)`.
///
/// Returns the last slot's `eax`, else the walk's.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn image_visit_0297<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    // push ebp; and esp, -16; sub esp, 0x268; two pushes
    let b0 = (fr.esp.wrapping_sub(4) & !0xf).wrapping_sub(0x268);
    let cookie = gw(g, 0xd) ^ b0;
    g.write::<u32>(b0 + 0x264, cookie);
    let s = b0.wrapping_sub(8);
    let this = fr.ecx;
    let owner = g.read::<u32>(this);
    // (`eax` holds the cookie when the walk starts)
    let (eax, tail) = crate::nodes::walk(g, this, owner, cookie);
    let Some(n) = tail else {
        return Ret::eax(eax);
    };
    let model = |g: &G| g.read::<u32>(g.read::<u32>(n + 0x88) + 0x54);
    let release = |g: &mut G| {
        let h = model(g);
        // call width: 3
        g.call(t(g, 0xac, 357), 0, &[h, u32::MAX, u32::MAX]);
        // call width: 3
        Ret::eax(g.call(t(g, 0xbc, 356), 0, &[h, u32::MAX, u32::MAX]).eax)
    };
    let mut paused = gw(g, 0x72);
    if paused == 0 {
        let life = g.read::<u16>(n + 0xba);
        g.write::<u16>(n + 0xba, life.wrapping_sub(1));
        if life == 0 {
            crate::sa_misc::leave(g, n, 0);
            return release(g);
        }
        let bits = g.read::<u16>(n + 0xb4);
        if bits & 4 != 0 {
            let step = g.read::<f32>(n + 0xa0);
            let x = add(step, g.read::<f32>(n + 0x94));
            g.write::<f32>(n + 0x94, x);
            let limit = g.read::<f32>(n + 0x9c);
            let past = if step > 0.0 { x > limit } else { limit > x };
            if past {
                g.write::<u16>(n + 0xb4, bits & 0xfffb);
                g.write::<f32>(n + 0x94, limit);
            }
        }
    }
    let (col_a, col_b) = (s + 0xa8, s + 0x98);
    for i in 0..4u32 {
        g.write::<u32>(col_a + 4 * i, ONE);
        g.write::<u32>(col_b + 4 * i, ONE);
    }
    if g.read::<u16>(n + 0xb4) & 2 != 0 {
        let h = model(g);
        // call width: 2
        g.call(t(g, 0x191, 355), 0, &[h, u32::MAX]);
        let h = model(g);
        // call width: 2
        g.call(t(g, 0x1a9, 354), 0, &[h, u32::MAX]);
        paused = gw(g, 0x1b7);
    }
    if g.read::<u16>(n + 0xb4) & 8 != 0 {
        if paused == 0 {
            let x = sub(g.read::<f32>(n + 0xa4), g.read::<f32>(n + 0xa8));
            g.write::<f32>(n + 0xa4, x);
        }
        let c = cvtt(mul(g.read::<f32>(n + 0xa4), k(g, 0x1f3)));
        let base = i32::from(g.read::<u16>(n + 0xb8));
        let end = base.wrapping_add(c);
        let mut i = c.max(0);
        let mut from = i.wrapping_sub(c);
        let put = |g: &mut G, i: i32, (op, tab): (u32, u32), v: u8| {
            let a = g.call(t(g, op, 941), 0, &[g.read::<u32>(n + 0xac)]).eax;
            let tab = gw(g, tab);
            let d = i32::from(g.read::<i8>(tab.wrapping_add((i & 0x1f).cast_unsigned())));
            let at = a
                .wrapping_add(d.cast_unsigned())
                .wrapping_add((i & 0xe0).cast_unsigned())
                .wrapping_add(3);
            g.write::<u8>(at, v);
        };
        if c < 0x100 {
            while i < end {
                #[allow(clippy::cast_precision_loss)] // cvtdq2ps
                let x = mul(div(from as f32, base as f32), k(g, 0x26b));
                from += 1;
                // call width: 1
                put(g, i, (0x235, 0x25b), cvtt(x).to_le_bytes()[0]);
                i += 1;
                if i >= 0x100 {
                    break;
                }
            }
        }
        let left = if i < 0x100 { 0x100 - i } else { 0 };
        while i < 0x100 {
            // call width: 1
            put(g, i, (0x2c1, 0x2d5), 0);
            i += 1;
        }
        let pic = n + 0x1c;
        let thunk = |g: &G, at: u32| {
            let th = call(at);
            slot_target(g, img.word(th + 1), img.word(th + 7) / 4)
        };
        let words = [
            pic,
            u32::from(g.read::<u16>(n + 0xb0)),
            4,
            u32::from(g.read::<u8>(n + 0xbc)),
            0,
            0,
            0x10,
            0x10,
        ];
        // call width: 8
        g.call(thunk(g, 0x31c), 0, &words);
        // call width: 1
        g.call(thunk(g, 0x323), 0, &[0]);
        let a = g.read::<u32>(n + 0xac);
        // call width: 2
        g.call(thunk(g, 0x332), 0, &[pic, a]);
        if left >= 0x100 {
            let parent = g.read::<u32>(n + 0x14);
            if parent != 0 {
                crate::nodes::leave_parent(g, n, parent);
            }
            return release(g);
        }
    }
    // the draw: the view rows, the camera rows
    let (m, r, v) = (s + 0x128, s + 0xf8, s + 0xb8);
    for (i, op) in (0u32..).zip([0x3a6u32, 0x3bd, 0x3d4, 0x3eb]) {
        // call width: 2
        g.call(t(g, op, 544), 0, &[i, m + 0x10 * i]);
    }
    for (i, op) in (0u32..).zip([0x402u32, 0x419, 0x43b]) {
        if i > 0 {
            g.write::<u32>(r + 0x10 * i - 4, 0);
        }
        // call width: 2
        g.call(t(g, op, 588), 0, &[i, r + 0x10 * i]);
    }
    let cam = g.read::<u32>(n + 0x8c).wrapping_add(0x9c);
    g.write::<u32>(r + 0x2c, 0);
    // call width: 2
    g.call(t(g, 0x469, 327), 0, &[v, cam]);
    for (i, (o232, o233)) in (0u32..).zip([(0x48du32, 0x4adu32), (0x4c6, 0x4e6), (0x4ff, 0x51f)]) {
        let row = v + 0x10 * i;
        // call width: 2
        g.call(t(g, o232, 232), 0, &[row, row]);
        // call width: 3
        g.call(t(g, o233, 233), 0, &[row, row, 0xbf80_0000]);
    }
    for (i, op) in (0u32..).zip([0x547u32, 0x56f, 0x597]) {
        let row = r + 0x10 * i;
        // call width: 3
        g.call(t(g, op, 229), 0, &[row, v, row]);
    }
    let cam = g.read::<u32>(n + 0x8c).wrapping_add(0x9c);
    // call width: 3
    g.call(t(g, 0x5c0, 231), 0, &[v, w(0x5ba), cam]);
    let h = model(g);
    // call width: 1
    let scale = st0_f32(g.call(t(g, 0x5d9, 277), 0, &[h]).st0);
    g.write::<f32>(s + 0xa4, scale);
    // the packet record at S + 0x20
    let rec = s + 0x20;
    for (o, from) in [(0x48u32, 0x90u32), (8, 0x80), (0xc, 0x84)] {
        let x = g.read::<u32>(n + from);
        g.write::<u32>(rec + o, x);
    }
    let c = gw(g, 0x613);
    let x = g.read::<u32>(c + 0x28);
    g.write::<u32>(rec + 0x58, x);
    for (o, p) in [
        (0x44u32, v),
        (0x30, col_a),
        (0x4c, col_b),
        (0x10, m),
        (0x14, r),
    ] {
        g.write::<u32>(rec + o, p);
    }
    let a = mul(mul(g.read::<f32>(n + 0x98), scale), k(g, 0x660));
    let bsz = mul(mul(g.read::<f32>(n + 0x94), scale), k(g, 0x668));
    let sum = add(a, bsz);
    g.write::<f32>(rec + 0x54, bsz);
    g.write::<f32>(rec + 0x50, sum);
    crate::r26::mesh_packets_0426(g, img, call(0x697), rec);
    let fl = s + 0x88;
    let x = div(g.read::<f32>(rec + 0x50), k(g, 0x6ac));
    g.write::<f32>(fl, x);
    let y = div(g.read::<f32>(rec + 0x54), k(g, 0x6c3));
    g.write::<f32>(fl + 4, y);
    if g.read::<u16>(n + 0xb4) & 8 != 0 {
        let c = cvtt(mul(g.read::<f32>(n + 0xa4), k(g, 0x6de)));
        #[allow(clippy::cast_precision_loss)] // cvtdq2ps
        let c = c as f32;
        g.write::<f32>(fl + 8, c);
        let b = f32::from(g.read::<u16>(n + 0xb8));
        g.write::<f32>(fl + 0xc, b);
    } else {
        g.write::<u32>(fl + 8, 0xc7c3_4f80);
        g.write::<u32>(fl + 0xc, 0xc7c3_4f80);
    }
    let h = model(g);
    // call width: 6
    g.call(
        t(g, 0x754, 857),
        0,
        &[h, w(0x74c), fl, fl + 4, fl + 8, fl + 0xc],
    );
    let id = u32::from(g.read::<u16>(n + 0x80) & 0x3fff);
    let lock = gw(g, 0x773);
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let buf = s + 0x168;
    // call width: 7
    let ok = g
        .call(t(g, 0x788, 885), 0, &[id, 0x13, 0, 0, 0, 0, buf])
        .eax;
    let lock = gw(g, 0x7a1);
    g.write::<u32>(lock, saved);
    let x = if ok == 0 { 0 } else { buf };
    let h = model(g);
    // call width: 3
    Ret::eax(g.call(t(g, 0x7c9, 906), 0, &[h, w(0x7c1), x]).eax)
}

/// [`image_visit_0297`] (the `rel32`s: the node leave, the slot thunks,
/// the mesh packets, the cookie check).
pub const IMAGE_VISIT_0297: Sig = Sig {
    len: 2028,
    hash: 0x60c3_8491_8172_2217,
    calls: &[
        (0x9c, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x31c, Callee::Sig(&crate::s5_misc::THUNK_425)),
        (0x323, Callee::Sig(&crate::s5_misc::THUNK_421)),
        (0x332, Callee::Sig(&crate::s5_misc::THUNK_423)),
        (0x697, Callee::Sig(&crate::r30::MESH_PACKETS_0297)),
        (0x7e4, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0297`'s image visit thunk (0x10003d20, 12 bytes; a vtable's
/// `+0x1c`): its stack word into `ecx`, then [`image_visit_0297`].
pub fn image_thunk_0297<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let callee = Frame {
        at: img.rel32_target(fr.at + 8),
        ecx: fr.arg(g, 0),
        ..*fr
    };
    image_visit_0297(g, img, &callee)
}

/// [`image_thunk_0297`].
pub const IMAGE_THUNK_0297: Sig = Sig {
    len: 12,
    hash: 0x896f_4fde_1224_a07f,
    calls: &[(8, Callee::Sig(&IMAGE_VISIT_0297))],
    imports: &[],
};

/// `magic_0297`'s summon task (0x10007520, 2312 bytes; a task step, cdecl
/// `(task)`; an 8-aligned frame, `S` = `esp` after the three pushes). The
/// task's record `R` (`task + 4`) holds the state `+0x18` and a counter
/// `+0x28` (one up at every exit). The states:
/// - 0: the summon's model node ([`model_node_init`](crate::sa_summon::model_node_init)),
///   slot 6 `(*G_A, 1)`, the scene's lists and nodes as immediates, slot
///   855 `(*G_A, 2)`, the motion ([`motion_setup`](crate::m0297::motion_setup)),
///   the two images slot 719 `(TEX, 2, 6)` / `(TEX, 3, 6)` describe, a
///   particle part (slot 719 `(TEX, 6, 0)`); then state 10 at once;
/// - 10: the model switch ([`mode_switch`](crate::m0297::mode_switch)
///   `(0x10751051)`), the light's colour and slot 234, state 11;
/// - 11: slot 20 or 411 (by a flag) must answer non-zero, then slot 30
///   and 36 or 228 with `0x10751053`, the flags, state 20 (counter 0);
/// - 20: after 20 steps, two nodes appended and the summon's facing
///   ([`facing_angle`](crate::m0159::facing_angle)), two inserted
///   ([`node_insert`](crate::nodes::node_insert)) around two appends, a
///   particle node ([`particle_node_with`](crate::s3_nodes::particle_node_with)),
///   state 21 (counter 0);
/// - 21: after 125 steps, a node appended and inserted with a frame group
///   ([`insert_with`](crate::sa_misc::insert_with)), a second particle
///   node, state 22;
/// - 22: the model id ([`model_id`](crate::sa_misc::model_id)); when set,
///   the switch `(0x10751054)` and state 23;
/// - 23: after 210 steps, state 10000 at once;
/// - 10000: the model set-up ([`model_setup`](crate::sa_summon::model_setup))
///   and the task's node out of its list.
///
/// Returns `eax` as the original leaves it (each path's last call, or the
/// state compare's value).
#[allow(clippy::too_many_lines)] // the original's states, in order
pub fn summon_task_0297<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let put = |g: &mut G, list: &[(u32, u32)]| {
        for &(to, v) in list {
            g.write::<u32>(w(to), v);
        }
    };
    let put_imm = |g: &mut G, list: &[u32]| {
        for &to in list {
            g.write::<u32>(w(to), w(to + 4));
        }
    };
    let inc16 = |g: &mut G, o: u32| {
        let x = g.read::<u16>(w(o)).wrapping_add(1);
        g.write::<u16>(w(o), x);
    };
    // push ebp; and esp, -8; sub esp, 0x3c; three pushes
    let s = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x48);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    let state = |g: &G| g.read::<u32>(r + 0x18);
    let end = |g: &mut G, eax: u32| {
        let c = g.read::<u32>(r + 0x28).wrapping_add(1);
        g.write::<u32>(r + 0x28, c);
        Ret::eax(eax)
    };
    let count = |g: &G| g.read::<i32>(r + 0x28);
    loop {
        let st = state(g);
        match st {
            0 => {
                let a = s + 0x10;
                g.write::<u32>(a, w(0x10e));
                g.write::<u32>(a + 4, w(0x116));
                g.write::<u32>(a + 8, 0x14);
                g.write::<u8>(a + 0xc, 1);
                put_imm(g, &[0x129]);
                crate::sa_summon::model_node_init(g, img, call(0x132), a);
                let x = gw(g, 0x13f);
                g.write::<u32>(w(0x145), 1);
                // call width: 2
                g.call(t(g, 0x137, 6), 0, &[x, 1]);
                let last = gw(g, 0x153);
                g.write::<u16>(w(0x15d), 0xff);
                put_imm(g, &[0x165, 0x16f, 0x179, 0x183]);
                g.write::<u32>(w(0x18c), last);
                if last == w(0x191) {
                    put_imm(g, &[0x199]);
                } else {
                    g.write::<u32>(last.wrapping_add(4), w(0x1a6));
                }
                let last2 = gw(g, 0x1ab);
                inc16(g, 0x1b2);
                put_imm(g, &[0x1b8]);
                put(g, &[(0x1c2, 0), (0x1cc, 0), (0x1d6, 0)]);
                put_imm(g, &[0x1e0, 0x1ea]);
                g.write::<u8>(w(0x1f4), 0);
                put_imm(g, &[0x1fb, 0x205]);
                g.write::<u16>(w(0x210), 0xff);
                put_imm(g, &[0x218, 0x222, 0x22c, 0x236]);
                g.write::<u32>(w(0x23f), last2);
                if last2 == w(0x244) {
                    put_imm(g, &[0x24c]);
                } else {
                    g.write::<u32>(last2.wrapping_add(4), w(0x259));
                }
                inc16(g, 0x260);
                g.write::<u16>(w(0x26b), 0x708);
                let x = gw(g, 0x278);
                put_imm(g, &[0x27e]);
                put(g, &[(0x288, 0), (0x292, 2)]);
                put_imm(g, &[0x29c, 0x2a6]);
                put(g, &[(0x2b0, 0x469c_4000), (0x2ba, 0)]);
                // call width: 2
                g.call(t(g, 0x270, 855), 0, &[x, 2]);
                // the motion
                let m = s + 0x30;
                let a1 = gw(g, 0x2cb);
                for (i, v) in (0u32..).zip([w(0x2d7), a1, 0xf_0000, 0xb_b800, 1, 0, 1]) {
                    g.write::<u32>(m + 4 * i, v);
                }
                let mf = Frame {
                    at: call(0x308),
                    esp: s.wrapping_sub(4),
                    ecx: m,
                    ..*fr
                };
                crate::m0297::motion_setup(g, img, &mf);
                // the two images
                // call width: 3
                let p = g.call(t(g, 0x30d, 719), 0, &[w(0x31c), 2, 6]).eax;
                let x = (g.read::<u16>(w(0x325)) & 0xfffd) | 2;
                g.write::<u16>(w(0x338), x);
                let x = g.read::<u32>(p);
                g.write::<u32>(w(0x340), x);
                let x = g.read::<u32>(p.wrapping_add(4));
                g.write::<u32>(w(0x348), x);
                g.write::<u16>(w(0x356), 2);
                // call width: 3
                let p = g.call(t(g, 0x35b, 719), 0, &[w(0x36a), 3, 6]).eax;
                let x = g.read::<u16>(p);
                g.write::<u16>(w(0x376), x);
                let x = g.read::<u32>(p.wrapping_add(4));
                g.write::<u32>(w(0x37f), x);
                let x = g.read::<u16>(p.wrapping_add(2));
                g.write::<u16>(w(0x38a), x);
                let x = g.read::<u16>(w(0x394)) | 1;
                g.write::<u16>(w(0x394), x);
                let x = g.read::<u8>(p.wrapping_add(8));
                g.write::<u8>(w(0x39a), x);
                let last = gw(g, 0x39f);
                put_imm(g, &[0x3a8]);
                g.write::<u16>(w(0x3b3), 0xff);
                put_imm(g, &[0x3bb, 0x3c5, 0x3cf, 0x3d9]);
                g.write::<u32>(w(0x3e2), last);
                if last == w(0x3e7) {
                    put_imm(g, &[0x3ef]);
                } else {
                    g.write::<u32>(last.wrapping_add(4), w(0x3fc));
                }
                let last2 = gw(g, 0x401);
                inc16(g, 0x408);
                put_imm(g, &[0x40e]);
                put(g, &[(0x418, 0), (0x422, 0), (0x42c, 0), (0x436, 0x48_0000)]);
                put_imm(g, &[0x440]);
                g.write::<u16>(w(0x44b), 0xff);
                put_imm(g, &[0x453, 0x45d, 0x467, 0x471]);
                g.write::<u32>(w(0x47a), last2);
                if last2 == w(0x47f) {
                    put_imm(g, &[0x487]);
                } else {
                    g.write::<u32>(last2.wrapping_add(4), w(0x494));
                }
                let mut n = g.read::<u16>(w(0x49a)).wrapping_add(1);
                g.write::<u32>(w(0x4a7), ONE);
                put_imm(g, &[0x4b1]);
                put(g, &[(0x4bb, 6), (0x4c5, 0), (0x4cf, 0), (0x4d9, 0)]);
                g.write::<u8>(w(0x4e3), 0);
                g.write::<u16>(w(0x4eb), 0xff);
                put_imm(g, &[0x4f3, 0x4fd, 0x507, 0x511, 0x51b]);
                // (two immediates compared: the list was not empty)
                if w(0x49f) == w(0x525) {
                    put_imm(g, &[0x52d]);
                } else {
                    put_imm(g, &[0x539]);
                }
                n = n.wrapping_add(1);
                g.write::<u16>(w(0x545), n);
                put_imm(g, &[0x552]);
                // call width: 3
                let h = g.call(t(g, 0x54a, 719), 0, &[w(0x563), 6, 0]).eax;
                g.write::<u32>(w(0x56a), h);
                put_imm(g, &[0x570, 0x57a]);
                put(g, &[(0x584, 0x40), (0x58e, 0x100)]);
                put_imm(g, &[0x594]);
                put(g, &[(0x59e, 0), (0x5a8, 1)]);
                g.write::<u32>(r + 0x18, 0xa);
            }
            0xa => {
                let mf = Frame {
                    at: call(0x5ed),
                    esp: s.wrapping_sub(4),
                    edx: 0x1075_1051,
                    ..*fr
                };
                crate::m0297::mode_switch(g, img, &mf);
                put(
                    g,
                    &[
                        (0x5fd, 0x437f_0000),
                        (0x607, 0x437f_0000),
                        (0x611, 0x437f_0000),
                        (0x61b, 0x4300_0000),
                        (0x625, 0),
                        (0x62f, 0),
                        (0x639, 0),
                        (0x643, 0x4300_0000),
                    ],
                );
                // call width: 3
                let eax = g
                    .call(t(g, 0x5f2, 234), 0, &[w(0x657), w(0x652), w(0x5f7)])
                    .eax;
                let x = g.read::<u16>(w(0x660)) | 1;
                g.write::<u16>(w(0x660), x);
                put(g, &[(0x667, 1), (0x671, 0x3d4c_cccd), (0x67b, 0)]);
                g.write::<u32>(r + 0x18, st.wrapping_add(1));
                return end(g, eax);
            }
            0xb => {
                let on = g.read::<u8>(w(0x4d)) & 1 != 0;
                let ok = if on {
                    let a = gw(g, 0x5b);
                    // call width: 1
                    g.call(t(g, 0x53, 20), 0, &[a]).eax
                } else {
                    let a = gw(g, 0x66);
                    // call width: 1
                    g.call(t(g, 0x53, 411), 0, &[a]).eax
                };
                if ok == 0 {
                    return end(g, 0);
                }
                let flags = if g.read::<u8>(w(0x7f)) & 1 != 0 {
                    let a = gw(g, 0x8d);
                    // call width: 1
                    g.call(t(g, 0x85, 30), 0, &[a]);
                    let a = gw(g, 0xab);
                    // call width: 2
                    g.call(t(g, 0x97, 36), 0, &[a, 0x1075_1053]);
                    g.read::<u16>(w(0xb3)) | 0x200
                } else {
                    let a = gw(g, 0xd1);
                    // call width: 2
                    g.call(t(g, 0x85, 228), 0, &[a, 0x1075_1053]);
                    g.read::<u16>(w(0xd9))
                };
                g.write::<u32>(w(0xe5), 0x1075_1053);
                g.write::<u16>(w(0xef), flags | 0x100);
                g.write::<u32>(r + 0x28, 0);
                g.write::<u32>(r + 0x18, 0x14);
            }
            0x14 => {
                if count(g) < 0x14 {
                    return end(g, st);
                }
                crate::sa_misc::node_append(g, img, call(0x6a8), w(0x6a3), w(0x69e), 0);
                crate::m0159::facing_angle(
                    g,
                    img,
                    call(0x6b2),
                    s.wrapping_sub(4),
                    w(0x6a3),
                    w(0x6ad),
                );
                let grp = s + 0x10;
                for (i, o) in (0u32..).zip([0x6beu32, 0x6c6, 0x6ce, 0x6d6, 0x6de, 0x6e6]) {
                    g.write::<u32>(grp + 4 * i, w(o));
                }
                let ins = Frame {
                    at: call(0x6eb),
                    esp: s.wrapping_sub(4),
                    ecx: grp,
                    ..*fr
                };
                crate::nodes::node_insert(g, img, &ins);
                let list = w(0x6f0);
                crate::sb_misc::node_append(g, img, call(0x6fa), w(0x6f5), list, 0);
                put(
                    g,
                    &[
                        (0x705, 0),
                        (0x70f, 0xc120_0000),
                        (0x719, 0xc120_0000),
                        (0x723, ONE),
                    ],
                );
                let node = w(0x6ff);
                crate::sb_misc::node_append(g, img, call(0x72c), node, list, 0);
                put(
                    g,
                    &[
                        (0x73a, ONE),
                        (0x744, ONE),
                        (0x74e, ONE),
                        (0x758, 0x4000_0000),
                    ],
                );
                for (i, v) in (0u32..).zip([w(0x764), list, w(0x770), w(0x778), w(0x780), node]) {
                    g.write::<u32>(grp + 4 * i, v);
                }
                let ins = Frame {
                    at: call(0x785),
                    ..ins
                };
                crate::nodes::node_insert(g, img, &ins);
                // push 0; sub esp, 8; push PART: the call's return address
                // at S - 0x14 (the two middle words never written)
                let esp = s.wrapping_sub(0x14);
                g.write::<u32>(esp + 4, w(0x794));
                g.write::<u32>(esp + 0x10, 0);
                let pf = Frame {
                    at: call(0x799),
                    esp,
                    ecx: w(0x78f),
                    ..*fr
                };
                let eax = crate::s3_nodes::particle_node_with(g, img, &pf);
                let x = g.read::<u16>(w(0x7a0)) | 4;
                g.write::<u16>(w(0x7a0), x);
                put(
                    g,
                    &[
                        (0x7a7, 0x42c8_0000),
                        (0x7b1, 0x4348_0000),
                        (0x7bb, 0xc4fa_0000),
                        (0x7c5, 0xc080_0000),
                    ],
                );
                g.write::<u32>(r + 0x18, st.wrapping_add(1));
                g.write::<u32>(r + 0x28, 0);
                return end(g, eax);
            }
            0x15 => {
                if count(g) < 0x7d {
                    return end(g, 0);
                }
                let list = w(0x82e);
                let node = w(0x833);
                crate::sb_misc::node_append(g, img, call(0x838), node, list, 0);
                put(
                    g,
                    &[
                        (0x853, ONE),
                        (0x85d, ONE),
                        (0x867, ONE),
                        (0x871, 0x3ea6_6666),
                    ],
                );
                // push node; three words; the call's return address at S - 0x14
                let esp = s.wrapping_sub(0x14);
                for (i, v) in (1u32..).zip([w(0x848), w(0x843), w(0x83e), node]) {
                    g.write::<u32>(esp + 4 * i, v);
                }
                let iw = Frame {
                    at: call(0x87a),
                    esp,
                    ecx: w(0x84d),
                    edx: list,
                    ..*fr
                };
                crate::sa_misc::insert_with(g, img, &iw);
                g.write::<u32>(esp + 4, w(0x88c));
                g.write::<u32>(esp + 0x10, 1);
                let pf = Frame {
                    at: call(0x891),
                    esp,
                    ecx: w(0x882),
                    ..*fr
                };
                crate::s3_nodes::particle_node_with(g, img, &pf);
                let x = g.read::<u16>(w(0x898)) | 8;
                g.write::<u16>(w(0x898), x);
                g.write::<u32>(w(0x8a4), ONE);
                g.write::<u16>(w(0x8ae), 0x10);
                g.write::<u32>(w(0x8b4), 0x3c08_8889);
                g.write::<u32>(r + 0x18, st.wrapping_add(1));
                return end(g, 0x10);
            }
            0x16 => {
                let a = s + 0x10;
                g.write::<u32>(a, w(0x7f8));
                let id = crate::sa_misc::model_id(g, img, call(0x7fd), a);
                if g.read::<u32>(a + 4) == 0 {
                    return end(g, id);
                }
                let mf = Frame {
                    at: call(0x812),
                    esp: s.wrapping_sub(4),
                    edx: 0x1075_1054,
                    ..*fr
                };
                let eax = crate::m0297::mode_switch(g, img, &mf).eax;
                g.write::<u32>(r + 0x18, st.wrapping_add(1));
                return end(g, eax);
            }
            0x17 => {
                if count(g) < 0xd2 {
                    return end(g, st);
                }
                g.write::<u32>(r + 0x18, 0x2710);
            }
            0x2710 => {
                let a = s + 0x10;
                g.write::<u32>(a, w(0x8db));
                let mf = Frame {
                    at: call(0x8e0),
                    esp: s.wrapping_sub(4),
                    ecx: a,
                    ..*fr
                };
                crate::sa_summon::model_setup(g, img, &mf);
                let node = g.read::<u32>(task);
                let prev = g.read::<u32>(node + 4);
                let next = g.read::<u32>(node + 8);
                g.write::<u32>(prev + 8, next);
                let next = g.read::<u32>(node + 8);
                let prev = g.read::<u32>(node + 4);
                g.write::<u32>(next + 4, prev);
                g.write::<u32>(node + 8, node);
                g.write::<u32>(node + 4, node);
                return end(g, prev);
            }
            // the dispatch's `sub` chain leaves its value
            _ if st < 0x14 => return end(g, st.wrapping_sub(0xb)),
            _ => return end(g, st),
        }
    }
}

/// [`summon_task_0297`] (the `rel32`s: its callees).
pub const SUMMON_TASK_0297: Sig = Sig {
    len: 2312,
    hash: 0x451d_eacf_8d8f_1a93,
    calls: &[
        (0x132, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0x308, Callee::Sig(&crate::m0297::MOTION_SETUP)),
        (0x5ed, Callee::Sig(&crate::m0297::MODE_SWITCH)),
        (0x6a8, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0x6b2, Callee::Sig(&crate::m0159::FACING_ANGLE)),
        (0x6eb, Callee::Sig(&crate::nodes::INSERT)),
        (0x6fa, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x72c, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x785, Callee::Sig(&crate::nodes::INSERT)),
        (0x799, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x7fd, Callee::Sig(&crate::sa_misc::MODEL_ID)),
        (0x812, Callee::Sig(&crate::m0297::MODE_SWITCH)),
        (0x838, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x87a, Callee::Sig(&crate::sa_misc::INSERT_WITH)),
        (0x891, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x8e0, Callee::Sig(&crate::sa_summon::MODEL_SETUP)),
    ],
    imports: &[],
};

/// `magic_0267`'s start-up (0x10002500, 1021 bytes; internal, no
/// arguments, run by its load step; an `ebp` frame, the place `P` at
/// `ebp - 0x14..`):
/// - four words; the actors looked up
///   ([`lookup_actors`](crate::sb_misc::lookup_actors)); `*G_C` = slot
///   719 `(D, 8, 0)`; the part set-up ([`part_init`](crate::l5_objhost::part_init));
/// - object 0 ([`start_object_anim`](crate::s4_misc::start_object_anim)
///   `(0, 0)`) at the summon's place (slot 262 of `*G_A`, `w` 1) with
///   the yaw `K - ` slot 333 `(*G_A)` wrapped once; `y` plus `K'` the
///   place `P`;
/// - object 1 `(1, 0xf0)` with its own place and yaw the same way, `P`
///   its place;
/// - the summon's part shown ([`part_toggle`](crate::s4_misc::part_toggle));
///   a light at the summon (slot 262, `y` the first yaw); blocks of
///   globals; a static node ([`static_node`](crate::o4_fam3::static_node));
///   slot 312 `(CB, 0, 0, 0)` into a global; the actor block
///   ([`me_block`](crate::sa_misc::me_block) `(D2)`);
/// - two pool nodes ([`take`](crate::sa_pool::take): marked 1 and 2) at
///   the task list's end.
///
/// Returns 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn start_up_0267<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let lf = |o: u32| ebp.wrapping_sub(o);
    g.write::<u32>(w(0x8), w(0xc));
    g.write::<u32>(w(0x12), w(0x16));
    g.write::<u32>(w(0x1c), 0x14);
    g.write::<u32>(w(0x26), 0);
    crate::sb_misc::lookup_actors(g, img, call(0x2f));
    // call width: 3
    let c = g.call(t(g, 0x34, 719), 0, &[w(0x43), 8, 0]).eax;
    g.write::<u32>(w(0x4d), c);
    crate::l5_objhost::part_init(g, img, call(0x52));
    let scale = k(g, 0x5a);
    crate::s4_misc::start_object_anim(g, img, call(0x68), w(0x63), scale, 0, 0);
    let a = gw(g, 0x88);
    // call width: 4
    g.call(t(g, 0x6d, 262), 0, &[a, w(0x82), w(0x7d), w(0x72)]);
    let a = gw(g, 0x95);
    g.write::<u32>(w(0x9b), ONE);
    // call width: 1
    let r = st0_f32(g.call(t(g, 0x8f, 333), 0, &[a]).st0);
    g.write::<f32>(lf(4), r);
    let yaw = wrap_once(g, img, f, sub(k(g, 0xb2), r), [0xc9, 0xbf, 0xd9, 0xbf]);
    g.write::<f32>(w(0xf2), yaw);
    let scale = k(g, 0xea);
    let y = add(g.read::<f32>(w(0xfa)), k(g, 0x102));
    g.write::<f32>(lf(0xc), y);
    g.write::<f32>(w(0x11e), y);
    for (o, from) in [(8u32, 0x126u32), (0x10, 0x133), (0x14, 0x140)] {
        let x = gw(g, from);
        g.write::<u32>(lf(o), x);
    }
    crate::s4_misc::start_object_anim(g, img, call(0x14a), w(0x10a), scale, 1, 0xf0);
    let a = gw(g, 0x16a);
    // call width: 4
    g.call(t(g, 0x14f, 262), 0, &[a, w(0x164), w(0x15f), w(0x154)]);
    let a = gw(g, 0x177);
    g.write::<u32>(w(0x17d), ONE);
    // call width: 1
    let r = st0_f32(g.call(t(g, 0x171, 333), 0, &[a]).st0);
    g.write::<f32>(lf(4), r);
    let yaw2 = sub(k(g, 0x194), r);
    let yaw2 = if yaw2 > k(g, 0x1a3) {
        sub(yaw2, k(g, 0x1ad))
    } else if k(g, 0x1b7) > yaw2 {
        add(yaw2, k(g, 0x1c4))
    } else {
        yaw2
    };
    g.write::<f32>(w(0x1cc), yaw2);
    for (o, from) in [(0x1d9u32, 8u32), (0x1e6, 0xc), (0x1f3, 0x10), (0x207, 0x14)] {
        let x = g.read::<u32>(lf(from));
        g.write::<u32>(w(o), x);
    }
    // push esi; push edi: the call's return address at ebp - 0x20
    let pt = Frame {
        at: call(0x20c),
        esp: lf(0x20),
        edx: w(0x1ff),
        ..*fr
    };
    crate::s4_misc::part_toggle(g, img, &pt);
    let y = gw(g, 0x214);
    let a = gw(g, 0x22e);
    g.write::<u32>(w(0x234), 0);
    g.write::<u32>(w(0x240), y);
    g.write::<u32>(w(0x246), 0);
    g.write::<u32>(w(0x250), 0);
    // call width: 4
    g.call(t(g, 0x219, 262), 0, &[a, w(0x228), w(0x223), w(0x21e)]);
    for (o, x) in [
        (0x262u32, ONE),
        (0x26c, 0),
        (0x276, ONE),
        (0x280, 0xbc36_0b61),
        (0x28a, 0x70),
        (0x294, 0x0080_ffff),
    ] {
        g.write::<u32>(w(o), x);
    }
    g.write::<u8>(w(0x29e), 0xff);
    g.write::<u16>(w(0x2a6), 0);
    g.write::<u8>(w(0x2ae), 0);
    g.write::<u32>(w(0x2b5), 0xf0);
    g.write::<u16>(w(0x2c0), 0x48);
    let sn = Frame {
        at: call(0x2c7),
        esp: lf(0x30),
        ..*fr
    };
    crate::o4_fam3::static_node(g, img, &sn);
    g.write::<u32>(w(0x2d6), 0);
    g.write::<u32>(w(0x2e0), 0);
    g.write::<u32>(w(0x2f7), 0x3dae_147b);
    g.write::<u32>(w(0x301), 0);
    // call width: 4
    let h = g.call(t(g, 0x2cc, 312), 0, &[w(0x2f1), 0, 0, 0]).eax;
    g.write::<u32>(w(0x30c), h);
    for (o, x) in [
        (0x319u32, 0u32),
        (0x323, 0),
        (0x32d, 0),
        (0x337, 0),
        (0x341, w(0x345)),
        (0x34b, w(0x34f)),
        (0x355, 0),
        (0x35f, 0),
    ] {
        g.write::<u32>(w(o), x);
    }
    g.write::<u16>(w(0x369), 0);
    crate::sa_misc::me_block(g, img, call(0x36e), w(0x313));
    // the two pool nodes at the task list's end
    let n1 = crate::sa_pool::take(g, img, call(0x37b), w(0x376));
    let first = gw(g, 0x381);
    g.write::<u8>(n1 + 0x18, 1);
    let last = gw(g, 0x398);
    task_append(g, img, f, n1, first, last, 0x3ad);
    let first = if first == 0 { n1 } else { first };
    let n2 = crate::sa_pool::take(g, img, call(0x3ba), w(0x3b2));
    g.write::<u8>(n2 + 0x18, 2);
    task_append(g, img, f, n2, first, n1, 0x3ea);
    g.write::<u32>(w(0x3d7), n2);
    Ret::eax(0)
}

/// [`start_up_0267`] (the `rel32`s: its callees).
pub const START_UP_0267: Sig = Sig {
    len: 1021,
    hash: 0x2712_b947_8b0b_d148,
    calls: &[
        (0x2f, Callee::Sig(&crate::sb_misc::LOOKUP_ACTORS)),
        (0x52, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x68, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x14a, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x20c, Callee::Sig(&crate::s4_misc::PART_TOGGLE)),
        (0x2c7, Callee::Sig(&crate::o4_fam3::STATIC_NODE_0267_37F0)),
        (0x36e, Callee::Sig(&crate::sa_misc::ME_BLOCK)),
        (0x37b, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x3ba, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// `magic_0267`'s load step (0x10001210, 118 bytes; a table entry): the
/// object hosts' load step ([`load_step`](crate::r9_load::load_step))
/// running [`start_up_0267`].
pub fn load_0267<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::r9_load::load_step(g, img, fr, crate::r9_load::Shape::Zero, start_up_0267)
}

/// [`load_0267`] (the 118-byte load step's `Sig`, the `rel32` the
/// start-up).
pub const LOAD_0267: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&START_UP_0267))],
    imports: &[],
};

/// `magic_0267`'s summon spin (0x100029c0, 797 bytes; a stored callback,
/// no arguments; a /GS `ebp` frame, the scale matrix `M` at `ebp -
/// 0x44`). While the wait `*G_W` is positive it counts down (at 0 slot
/// 525 `(*G_A)` once; returns the count or that slot's `eax`); with the
/// scale `*G_S` exactly 0 nothing more (`eax` as `lahf` leaves it).
/// Else the three angles `G_R[0..3]` step by `*G_D` (each wrapped once),
/// the scale by its speed `*G_V` (stopped at 0: the speed 0), the fade
/// `*G_F` drops by `K`; at scale 0 the summon's part is shown
/// ([`part_toggle`](crate::s4_misc::part_toggle) `(0)`), the count or
/// slot 257 ([`count_or_257`](crate::s4_small::count_or_257)) and slot
/// 311 `(*G_H)` with `*G_F` 0 and `*G_H` -1 after. Then the place's `y`
/// steps by `K'`, `M` is the summon's scale
/// ([`scale_matrix`](crate::m0267::scale_matrix)), the summon's matrix
/// `N` = slot 330 `(N, M, R[2])`, 328 and 329 `(N, N, R[0])` / `R[1]`,
/// its three rows scaled by the scale and its last row the place.
/// Returns slot 329's `eax`.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn spin_0267<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let cookie = gw(g, 0x7) ^ ebp;
    g.write::<u32>(ebp.wrapping_sub(4), cookie);
    let wait = gw(g, 0x11);
    if wait.cast_signed() > 0 {
        let x = wait.wrapping_sub(1);
        g.write::<u32>(w(0x1b), x);
        if x != 0 {
            return Ret::eax(x);
        }
        let a = gw(g, 0x2c);
        // call width: 1
        return Ret::eax(g.call(t(g, 0x26, 525), 0, &[a]).eax);
    }
    let s = k(g, 0x4d);
    if s == 0.0 {
        // `lahf` after `ucomiss` (equal: ZF, the fixed bit 1) into `ah`
        return Ret::eax((wait & 0xffff_00ff) | 0x4200);
    }
    let (d, hi, tw, lo) = (k(g, 0x6d), k(g, 0x75), k(g, 0x7d), k(g, 0x85));
    let wrap = |x: f32| {
        if x > hi {
            sub(x, tw)
        } else if lo > x {
            add(x, tw)
        } else {
            x
        }
    };
    for (from, to) in [(0x65u32, 0xa5u32), (0xad, 0xcd), (0xd5, 0x101)] {
        let x = wrap(add(g.read::<f32>(w(from)), d));
        g.write::<f32>(w(to), x);
    }
    let mut scale = add(k(g, 0xf5), s);
    g.write::<f32>(w(0x10c), scale);
    if 0.0 > scale {
        scale = 0.0;
        g.write::<f32>(w(0x119), 0.0);
        g.write::<u32>(w(0x11f), 0);
    }
    let fade = sub(k(g, 0x12b), k(g, 0x133));
    g.write::<f32>(w(0x13e), fade);
    let m = ebp.wrapping_sub(0x44);
    if scale == 0.0 {
        let pt = Frame {
            at: call(0x14b),
            esp: ebp.wrapping_sub(0x48),
            edx: 0,
            ..*fr
        };
        crate::s4_misc::part_toggle(g, img, &pt);
        let cf = Frame {
            at: call(0x150),
            ..pt
        };
        crate::s4_small::count_or_257(g, img, &cf);
        let h = gw(g, 0x15b);
        g.write::<u32>(w(0x167), 0);
        // call width: 1
        g.call(t(g, 0x155, 311), 0, &[h]);
        g.write::<u32>(w(0x176), u32::MAX);
    }
    let y = add(k(g, 0x182), k(g, 0x18a));
    g.write::<f32>(w(0x195), y);
    let sm = Frame {
        at: call(0x19a),
        esp: ebp.wrapping_sub(0x48),
        edx: m,
        ..*fr
    };
    crate::m0267::scale_matrix(g, img, &sm);
    let n = w(0x1b6);
    let a = gw(g, 0x1a2);
    // call width: 3
    g.call(t(g, 0x1b1, 330), 0, &[n, m, a]);
    let a = gw(g, 0x1c6);
    // call width: 3
    g.call(t(g, 0x1cb, 328), 0, &[w(0x1e3), w(0x1de), a]);
    let a = gw(g, 0x1ed);
    // call width: 3
    let eax = g.call(t(g, 0x1f2, 329), 0, &[w(0x20a), w(0x205), a]).eax;
    let scale = k(g, 0x214);
    for (from, to) in [
        (0x21cu32, 0x22bu32),
        (0x233, 0x23f),
        (0x247, 0x253),
        (0x25b, 0x267),
        (0x26f, 0x27b),
        (0x283, 0x28f),
        (0x297, 0x2a3),
        (0x2ab, 0x2b7),
        (0x2bf, 0x2cb),
    ] {
        let x = mul(g.read::<f32>(w(from)), scale);
        g.write::<f32>(w(to), x);
    }
    for (from, to) in [
        (0x2d3u32, 0x2dbu32),
        (0x2e3, 0x2eb),
        (0x2f3, 0x2fb),
        (0x303, 0x30b),
    ] {
        let x = gw(g, from);
        g.write::<u32>(w(to), x);
    }
    Ret::eax(eax)
}

/// [`spin_0267`] (the `rel32`s: its callees and the cookie check).
pub const SPIN_0267: Sig = Sig {
    len: 797,
    hash: 0x92fc_89c8_65c8_2108,
    calls: &[
        (0x41, Callee::Any),
        (0x14b, Callee::Sig(&crate::s4_misc::PART_TOGGLE)),
        (0x150, Callee::Sig(&crate::s4_small::COUNT_OR_257)),
        (0x19a, Callee::Sig(&crate::m0267::SCALE_MATRIX)),
        (0x315, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0267`'s spin thunk (0x10002900, 5 bytes; the start-up's task
/// step): a tail jump to [`spin_0267`].
pub fn spin_thunk_0267<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let callee = Frame {
        at: img.rel32_target(fr.at + 1),
        ..*fr
    };
    spin_0267(g, img, &callee)
}

/// [`spin_thunk_0267`] (`jmp rel32`).
pub const SPIN_THUNK_0267: Sig = Sig {
    len: 5,
    hash: 0x2ad2_8090_e3fa_4fe4,
    calls: &[(1, Callee::Sig(&SPIN_0267))],
    imports: &[],
};
