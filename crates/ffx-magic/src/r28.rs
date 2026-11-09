//! Round 28's ports (magic-r28): the shared rows and the own rows of
//! overlays below `magic_0450`.

use crate::m0018::st0_f32;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The spin's second state (internal, no arguments; `magic_0563`
/// 0x100045f0 and `magic_0612`, 1581 bytes; an `ebp` /GS frame, the
/// matrix `M` at `ebp - 0x54`, the vector `V` at `ebp - 0x14`; `S =
/// *G_S`), run each frame while `S + 0x3664` is 1:
/// - the step `S + 0x3590..` added to `S + 0x3580..`, that to the point
///   `S + 0x3620..`; `M` = slot 235, turned by slot 328 `(M, M, -K)`; slot
///   231 `(P, P, M)` turns the point `P = S + 0x35a0`;
/// - the count `S + 0x3660` one less; at 0, the 18 records `R` (0x60
///   apart from `S + 0x2ec0`) get `V` = slot 229 `(V, P, R)` times `K0`,
///   their directions `V K1` and `V.x K2`, slot 314 `(1.0)` less `K3`,
///   0.3, 0, `V.z K2`, `V.x K4`, `V.z K4` and three slot 314 `(K)`;
/// - the object `S + 0x980` started
///   ([`start_object_anim`](crate::s4_misc::start_object_anim), scale
///   `K1`, `(8, 0)`), its draw the code pointer `VIEW`, kind 2, its point
///   from `S + 0x2b40`; the state 2;
/// - two colour records (`S + 0x3670`, `S + 0x3710`) and the sprite fan
///   record `S + 0x3a00` filled, each with a node of the pool's ring
///   ([`take_busy_node`](crate::o6_fam_pairs5::take_busy_node)) on the
///   task list; between them slot 388 `(B[i], 6, 1)` for each of the `N`
///   target bytes.
///
/// Returns slot 231's `eax` while the count runs, else 0xffff.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn spin_step_0563<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32) -> u32 {
    use crate::sb_sse::{add, mul, sub};
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x2d), n);
    let rf = |g: &G, at: u32| g.read::<f32>(at);
    let ebp = esp.wrapping_sub(4);
    let l = |o: u32| ebp.wrapping_sub(o);
    let s = g.read::<u32>(w(0x13));
    let (m, v) = (l(0x54), l(0x14));
    g.write::<u32>(l(0x5c), s);
    for i in 0..3u32 {
        let o = 4 * i;
        let x = add(rf(g, s + 0x3590 + o), rf(g, s + 0x3580 + o));
        g.write::<f32>(s + 0x3580 + o, x);
    }
    for i in 0..3u32 {
        let o = 4 * i;
        let x = add(rf(g, s + 0x3580 + o), rf(g, s + 0x3620 + o));
        g.write::<f32>(s + 0x3620 + o, x);
    }
    // call width: 1
    g.call(slot(g, 235), 0, &[m]);
    // call width: 3
    g.call(slot(g, 328), 0, &[m, m, 0xbdb2_b8c3]);
    let p = s + 0x35a0;
    // call width: 3
    let eax = g.call(slot(g, 231), 0, &[p, p, m]).eax;
    let n = g.read::<u32>(s + 0x3660).wrapping_sub(1);
    g.write::<u32>(s + 0x3660, n);
    if n != 0 {
        return eax;
    }
    // the 18 records
    for i in 0..18u32 {
        let r = s + 0x2ef4 + 0x60 * i;
        // call width: 3
        g.call(slot(g, 229), 0, &[v, p, r - 0x34]);
        let k0 = k(g, 0x12a);
        let x = mul(rf(g, v), k0);
        let y = mul(rf(g, v + 4), k0);
        let z = mul(rf(g, v + 8), k0);
        g.write::<f32>(v, x);
        g.write::<f32>(v + 4, y);
        g.write::<f32>(v + 8, z);
        let k1 = k(g, 0x141);
        let x2 = mul(x, k(g, 0x170));
        g.write::<f32>(r - 4, mul(x, k1));
        g.write::<f32>(r, mul(y, k1));
        g.write::<f32>(r + 4, mul(z, k1));
        g.write::<f32>(r + 0xc, x2);
        // call width: 1
        let one = st0_f32(g.call(slot(g, 314), 0, &[0x3f80_0000]).st0);
        g.write::<f32>(l(0x58), one);
        let z = rf(g, v + 8);
        g.write::<f32>(r + 0x10, sub(one, k(g, 0x1b7)));
        g.write::<u32>(r + 0x20, 0x3e99_999a);
        g.write::<u32>(r + 0x28, 0);
        g.write::<f32>(r + 0x14, mul(z, k(g, 0x1dc)));
        let x = rf(g, v);
        g.write::<f32>(r + 0x24, mul(z, k(g, 0x1e4)));
        g.write::<f32>(r + 0x1c, mul(x, k(g, 0x1f6)));
        for o in [0x14u32, 0x10, 0xc] {
            // call width: 1
            let q = st0_f32(g.call(slot(g, 314), 0, &[0x3db2_b8c3]).st0);
            g.write::<f32>(r - o, q);
        }
    }
    // the object
    let obj = s + 0x980;
    let start = img.rel32_target(f + 0x265);
    crate::s4_misc::start_object_anim(g, img, start, obj, k(g, 0x251), 8, 0);
    let free = g.read::<u16>(w(0x26c));
    g.write::<u32>(obj + 0x104, w(0x276));
    g.write::<u8>(obj + 0x126, 2);
    for i in 0..4u32 {
        let q = g.read::<u32>(s + 0x2b40 + 4 * i);
        g.write::<u32>(obj + 0xd0 + 4 * i, q);
    }
    g.write::<u32>(s + 0x3664, 2);
    // the two colour records, each with a node
    let ops =
        |head, callback, free_w, last_cmp, last, last_w, first_w| crate::o6_fam_pairs5::NodeOps {
            a: 0,
            t217: (0, 0),
            t234: (0, 0),
            t233: [(0, 0); 3],
            free: 0,
            k_a4: 0,
            k_9c: 0,
            head,
            callback,
            free_w,
            last_cmp,
            last,
            last_w,
            first_w,
        };
    let take = |g: &mut G, free: u16, ops: &crate::o6_fam_pairs5::NodeOps, r: u32| {
        crate::o6_fam_pairs5::take_busy_node(g, img, f, free, ops, r, false);
    };
    let c1 = s + 0x3670;
    colour_record(g, c1, (0xff00_0000, 0, 0xffff, 0x80), (0x1d, 10));
    // (the first node's append leaves the tail global alone; the second's
    // sets it, so writing it twice ends the same)
    take(
        g,
        free,
        &ops(0x32c, 0x37d, 0x388, 0x38e, 0x3a0, 0x3b3, 0x3a0),
        c1,
    );
    g.write::<u32>(c1 + 0x94, 4);
    let c2 = c1 + 0xa0;
    colour_record(g, c2, (0x0080_ffff, 0xff, 0, 0), (0x2b, 0x3c));
    let free = g.read::<u16>(w(0x440));
    take(
        g,
        free,
        &ops(0x463, 0x481, 0x48c, 0x492, 0x3a0, 0x4b2, 0x4bd),
        c2,
    );
    // the target bytes
    let mut i = 0i32;
    while i < g.read::<i32>(w(0x4c3)) {
        let b = g.read::<u32>(w(0x4dd));
        let b = u32::from(g.read::<u8>(b.wrapping_add(i.cast_unsigned())));
        // call width: 3
        g.call(slot(g, 388), 0, &[b, 6, 1]);
        i += 1;
    }
    // the sprite fan record
    let free = g.read::<u16>(w(0x515));
    let e = s + 0x3a00;
    for (o, v) in [
        (0xf0u32, 0x4080_8080u32),
        (0x124, 4),
        (0x120, 0),
        (0x110, 0),
        (0xf8, 0),
        (0xfc, 0x3d4c_cccd),
        (0x100, 0),
        (0x108, 0x28),
        (0x104, 0x28),
        (0x10c, 8),
    ] {
        g.write::<u32>(e + o, v);
    }
    take(
        g,
        free,
        &ops(0x505, 0x5de, 0x5e9, 0x5ef, 0x50b, 0x610, 0x619),
        e,
    );
    0xffff
}

/// A colour record's fields (`r + 0x80..0x98`): the colour word `+0x81`,
/// the bytes `+0x80`, `+0x87` and the u16 `+0x85`, the kind `+0x88`, the
/// u16 `+0x98` 0x48, the two times `+0x8c`/`+0x90`, `+0x94` 0.
fn colour_record<G: Game>(
    g: &mut G,
    r: u32,
    (colour, b80, h85, b87): (u32, u8, u16, u8),
    (kind, time): (u32, u32),
) {
    g.write::<u32>(r + 0x81, colour);
    g.write::<u8>(r + 0x80, b80);
    g.write::<u16>(r + 0x85, h85);
    g.write::<u8>(r + 0x87, b87);
    g.write::<u32>(r + 0x88, kind);
    g.write::<u16>(r + 0x98, 0x48);
    g.write::<u32>(r + 0x90, time);
    g.write::<u32>(r + 0x8c, time);
    g.write::<u32>(r + 0x94, 0);
}

/// [`spin_step_0563`] (the `rel32`s: the object start,
/// `__security_check_cookie`).
pub const SPIN_STEP_0563: Sig = Sig {
    len: 1581,
    hash: 0x87ad_0ece_60bb_ec42,
    calls: &[
        (0x265, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x625, Callee::Any),
    ],
    imports: &[],
};

/// The four actors' matrices of `magic_0511` (0x10001610) and `magic_0602`
/// (0x10006d00), 2607 bytes (internal, no arguments; a /GS frame, `esp`
/// at the return address; the point `P` at `ebp - 0x14`, the buffer `L`
/// at `ebp - 0x24`): [`mats_0168`](crate::r17::mats_0168)'s layout with
/// four blocks. With `E = *G_E` and `C[n] = CHRS[SEL[n]]`: `C[0]` gives
/// `P` (slot 262; into `E + 0x1a60`, `w` 1.0) and the yaw `K - ` slot 333
/// `(C[0])` wrapped once; then for `(at, M)` = `(0x1980, 0x17c0)`,
/// `(0x19a0, 0x1800)`, `(0x19c0, 0x1840)`, `(0x19e0, 0x1880)` and the
/// actors `C[0]`, `*G_B`, `C[1]`, `C[2]`: `E + at` `(0, yaw, 0, 0)` and
/// `+0x10` `P`, the matrix `E + M`
/// ([`mats_block`](crate::r17::mats_block)) and its point `E + M + 0x30`
/// from `E + at + 0x10`; slot 238 `(i, 1)` for `*G_I`, `SEL[1]`,
/// `SEL[2]`; for each of those slot 296 `(i, 0)` (`*G_F` 0 and the index
/// below 32) or 257 `(actor, 1)`; the yaw into `E + 0x2258`, `E + 0x2274`
/// 0, four flags 1. Returns the last slot's `eax`.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn mats_0511<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32) -> u32 {
    use crate::r17::{Who, mats_block, who, wrap_once};
    use crate::sb_sse::sub;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let ebp = esp.wrapping_sub(4);
    let (p, l) = (ebp.wrapping_sub(0x14), ebp.wrapping_sub(0x24));
    let tab = w(0x33);
    let e = g.read::<u32>(w(0x1b));
    let c = who(g, img, f, Who::Chr(0x11, 0x2e));
    // call width: 4
    g.call(slot_target(g, tab, 262), 0, &[c, p, p + 4, p + 8]);
    for i in 0..3 {
        let v = g.read::<u32>(p + 4 * i);
        g.write::<u32>(e + 0x1a60 + 4 * i, v);
    }
    g.write::<u32>(e + 0x1a6c, 0x3f80_0000);
    g.write::<u32>(p + 0xc, 0x3f80_0000);
    let c = who(g, img, f, Who::Chr(0x71, 0x82));
    let r = st0_f32(g.call(slot_target(g, tab, 333), 0, &[c]).st0);
    let yaw = wrap_once(g, img, f, sub(k(g, 0x9a), r), (0xa9, 0xb8, 0xc2, 0xcf));
    g.write::<f32>(ebp.wrapping_sub(0x2c), yaw);
    let quad = |g: &mut G, at: u32| {
        g.write::<u32>(e + at, 0);
        g.write::<f32>(e + at + 4, yaw);
        g.write::<u32>(e + at + 8, 0);
        g.write::<u32>(e + at + 0xc, 0);
        for i in 0..4 {
            let v = g.read::<u32>(p + 4 * i);
            g.write::<u32>(e + at + 0x10 + 4 * i, v);
        }
    };
    // (quad, matrix, actor, the null way's (flag, index), scale, factors,
    // K, order; the null way needs `E + M` = 0, which no real `E` gives)
    #[allow(clippy::type_complexity)] // one row per block
    let blocks: [(u32, u32, Who, (u32, Who), Who, Who, u32, u8); 4] = [
        (
            0x1980,
            0x17c0,
            Who::Chr(0x134, 0x145),
            (0x167, Who::ChrAt(0x134, 0, 0)),
            Who::Chr(0x1be, 0x1c8),
            Who::Chr(0x1f8, 0x202),
            0x1e5,
            0b011,
        ),
        (
            0x19a0,
            0x1800,
            Who::Global(0x33c),
            (0x363, Who::Global(0x36c)),
            Who::Global(0x3c3),
            Who::Global(0x3f4),
            0x3db,
            0b111,
        ),
        (
            0x19c0,
            0x1840,
            Who::ChrAt(0x529, 0x53a, 1),
            (0x55b, Who::ChrAt(0x529, 0, 1)),
            Who::ChrAt(0x5af, 0x5ba, 1),
            Who::ChrAt(0x5ea, 0x5f5, 1),
            0x5d7,
            0b000,
        ),
        (
            0x19e0,
            0x1880,
            Who::ChrAt(0x72f, 0x740, 2),
            (0x761, Who::ChrAt(0x72f, 0, 2)),
            Who::ChrAt(0x7b5, 0x7c0, 2),
            Who::ChrAt(0x7f0, 0x7fb, 2),
            0x7dd,
            0b100,
        ),
    ];
    for (at, mo, actor, (flag, index), scale_of, e_of, k_op, order) in blocks {
        quad(g, at);
        let a = who(g, img, f, actor);
        let m = e.wrapping_add(mo);
        let null = if m == 0 && g.read::<u32>(w(flag)) == 0 {
            let i = index_of(g, img, f, index);
            (i < 0x20).then_some(i.cast_unsigned())
        } else {
            None
        };
        mats_block(
            g,
            img,
            f,
            tab,
            m,
            (a, null, l),
            (scale_of, e_of, k_op, order),
            yaw,
        );
        for i in 0..4 {
            let v = g.read::<u32>(e + at + 0x10 + 4 * i);
            g.write::<u32>(m + 0x30 + 4 * i, v);
        }
    }
    let byte = |g: &G, op: u32, n: u32| {
        let s = g.read::<u32>(w(op)).wrapping_add(n);
        u32::from(g.read::<u8>(s))
    };
    let i = g.read::<u32>(w(0x8db));
    // call width: 2
    g.call(slot_target(g, tab, 238), 0, &[i, 1]);
    let i = byte(g, 0x8e8, 1);
    // call width: 4
    g.call(slot_target(g, tab, 238), 0, &[i, 1]);
    let i = byte(g, 0x901, 2);
    // call width: 6
    g.call(slot_target(g, tab, 238), 0, &[i, 1]);
    // the three actors shown: by index or by actor
    let mut eax;
    let i = g.read::<i32>(w(0x928));
    if g.read::<u32>(w(0x91e)) == 0 && i < 0x20 {
        // call width: 2
        eax = g
            .call(slot_target(g, tab, 296), 0, &[i.cast_unsigned(), 0])
            .eax;
    } else {
        let a = g.read::<u32>(w(0x94a));
        // call width: 2
        eax = g.call(slot_target(g, tab, 257), 0, &[a, 1]).eax;
    }
    for (n, flag, chrs) in [(1u32, 0x960u32, 0x989u32), (2, 0x9a4, 0x9cd)] {
        let sel_op = if n == 1 { 0x957 } else { 0x99b };
        let b = byte(g, sel_op, n);
        if g.read::<u32>(w(flag)) == 0 && b < 0x20 {
            // call width: 2
            eax = g.call(slot_target(g, tab, 296), 0, &[b, 0]).eax;
        } else {
            let a = g.read::<u32>(w(chrs).wrapping_add(4 * b));
            // call width: 2
            eax = g.call(slot_target(g, tab, 257), 0, &[a, 1]).eax;
        }
    }
    g.write::<f32>(e + 0x2258, yaw);
    g.write::<u32>(e + 0x2274, 0);
    for o in [0x9ffu32, 0xa09, 0xa13, 0xa1d] {
        g.write::<u32>(w(o), 1);
    }
    eax
}

/// The null way's actor index of [`mats_0511`]: a selector byte or a
/// global (signed).
fn index_of<G: Game>(g: &G, img: &Image, f: u32, w: crate::r17::Who) -> i32 {
    use crate::r17::Who;
    match w {
        Who::ChrAt(sel, _, n) => {
            let s = g.read::<u32>(img.word(f + sel)).wrapping_add(n);
            i32::from(g.read::<u8>(s))
        }
        Who::Chr(sel, _) => index_of(g, img, f, Who::ChrAt(sel, 0, 0)),
        Who::Global(op) => g.read::<i32>(img.word(f + op)),
    }
}

/// [`mats_0511`] (the `rel32`: `__security_check_cookie`).
pub const MATS_0511: Sig = Sig {
    len: 2607,
    hash: 0x0646_2d34_b5fa_444d,
    calls: &[(0xa27, Callee::Any)],
    imports: &[],
};

/// The spin's spark task (`magic_0563` 0x10003be0 and `magic_0612`,
/// 847 bytes; a task function, cdecl `(node)`; an `ebp` /GS frame, the
/// matrix `M` at `ebp - 0x44`): the node's tick `+0x14` (u16) rises;
/// every 16th tick while the count `+0x16` is below 90 (every 2nd after),
/// with `S = *G_S`, the floor `y0` = slot 12 `(*G_X)` less `K0` and `M` =
/// slot 230 `(M, S + 0x35a0)` with its 3x3 part times `K1`: for each of
/// the 18 records `R` (0x60 apart from `S + 0x2ec0`) a pool record `P`
/// ([`pool_take`](crate::sb_misc::pool_take); none: the next) gets the
/// texture `*(*G_T + 0x24)`, `+0x38` 0xffff, `+0x3a` 0x200, `+0x34` 1;
/// slot 229 `(P, M, R)`; random offsets from slot 314 (10, 4 less `K2`,
/// 10), random `+0x10`, `+0x14` (less `K3`), `+0x18`, `+0x1c`, the
/// constants `0, 1e-4, 0, 1`, `+0x3c` = (slot 404 `(0.5)` + `K0`) `K4`,
/// `+0x4c..` `(0x20, 0x10, -16)`, bytes `+0x67` 0x40 and `+0x6b` 0, `+0x60`
/// 0; a `y` above `y0` becomes `y0 - K0`. Then the count rises; past 95 the
/// node is given back ([`give_back`](crate::sa_pool::give_back)).
///
/// Returns the give-back's `eax`, else the tick mask (1 or 0xf) or the last
/// record's slot 404 `eax` (0 when its take failed).
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn sparks_0563<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let rf = |g: &G, at: u32| g.read::<f32>(at);
    let node = fr.arg(g, 0);
    let ebp = fr.esp.wrapping_sub(4);
    let l = |o: u32| ebp.wrapping_sub(o);
    let tick = g.read::<u16>(node + 0x14).wrapping_add(1);
    g.write::<u16>(node + 0x14, tick);
    let mask: u32 = if g.read::<i16>(node + 0x16) < 0x5a {
        0xf
    } else {
        1
    };
    g.write::<u32>(l(0x54), node);
    let mut eax = mask;
    if u32::from(tick) & mask == 0 {
        // the texture word's offset (disp8 `+0x44`) and the records'
        // (imm32 `+0x5f`) differ between copies (0x24, 0x2ec0 here)
        let tex = g.read::<u32>(g.read::<u32>(w(0x3d)) + (w(0x44) & 0xff));
        g.write::<u32>(l(0x58), tex);
        let mut r = g.read::<u32>(w(0x47)) + w(0x5f);
        let x = g.read::<u32>(w(0x56));
        // call width: 1
        let y = st0_f32(g.call(slot(g, 0x4f, 12), 0, &[x]).st0);
        g.write::<f32>(l(0x4c), y);
        let y0 = sub(y, k(g, 0x71));
        g.write::<f32>(l(0x50), y0);
        let m = l(0x44);
        // call width: 3
        g.call(slot(g, 0x81, 230), 0, &[m, r + 0x6e0, x]);
        for i in 0..4 {
            let v = g.read::<u32>(r + 0x760 + 4 * i);
            g.write::<u32>(l(0x14 - 4 * i), v);
        }
        let k1 = k(g, 0x9e);
        g.write::<u32>(l(0x4c), 0x12);
        for row in 0..3u32 {
            for col in 0..3u32 {
                let at = m + 0x10 * row + 4 * col;
                g.write::<f32>(at, mul(rf(g, at), k1));
            }
        }
        for n in (1..=0x12u32).rev() {
            let p = crate::sb_misc::pool_take(g, img, img.rel32_target(f + 0x159));
            eax = p;
            if p != 0 {
                eax = spark(g, img, f, p, (tex, m, r), y0);
            }
            r += 0x60;
            g.write::<u32>(l(0x4c), n - 1);
        }
    }
    let count = g.read::<u16>(node + 0x16).wrapping_add(1);
    g.write::<u16>(node + 0x16, count);
    if count.cast_signed() > 0x5f {
        eax = crate::sa_pool::give_back(g, img, img.rel32_target(f + 0x33d), node);
    }
    Ret::eax(eax)
}

/// One spark of [`sparks_0563`]: the pool record `p` for the record `r`
/// (the texture `tex`, the matrix `m`, the floor `y0`). Returns slot
/// 404's `eax`.
fn spark<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    p: u32,
    (tex, m, r): (u32, u32, u32),
    y0: f32,
) -> u32 {
    use crate::sb_sse::{add, mul, sub};
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let rf = |g: &G, at: u32| g.read::<f32>(at);
    let slot = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let random = |g: &mut G, o: u32, x: u32| st0_f32(g.call(slot(g, o, 314), 0, &[x]).st0);
    g.write::<u32>(p + 0x30, tex);
    g.write::<u16>(p + 0x38, 0xffff);
    g.write::<u16>(p + 0x3a, 0x200);
    g.write::<u32>(p + 0x34, 1);
    // call width: 3
    g.call(slot(g, 0x183, 229), 0, &[p, m, r]);
    let v = random(g, 0x198, 0x4120_0000);
    g.write::<f32>(p, add(rf(g, p), v));
    let v = random(g, 0x1bb, 0x4080_0000);
    let v = add(sub(v, k(g, 0x1de)), rf(g, p + 4));
    g.write::<f32>(p + 4, v);
    let v = random(g, 0x1e3, 0x4120_0000);
    g.write::<f32>(p + 8, add(rf(g, p + 8), v));
    let v = random(g, 0x20e, 0x3e4c_cccd);
    g.write::<f32>(p + 0x10, v);
    let v = random(g, 0x22a, 0x3dcc_cccd);
    g.write::<f32>(p + 0x14, sub(v, k(g, 0x249)));
    let v = random(g, 0x24e, 0x3e4c_cccd);
    g.write::<f32>(p + 0x18, v);
    let v = random(g, 0x26a, 0x4049_0fdb);
    g.write::<f32>(p + 0x1c, v);
    g.write::<u32>(p + 0x20, 0);
    g.write::<u32>(p + 0x24, 0x38d1_b717);
    g.write::<u32>(p + 0x28, 0);
    g.write::<u32>(p + 0x2c, 0x3f80_0000);
    // call width: 1
    let out = g.call(slot(g, 0x281, 404), 0, &[0x3f00_0000]);
    let k0 = k(g, 0x2bc);
    let v = mul(add(st0_f32(out.st0), k0), k(g, 0x2e0));
    g.write::<u32>(p + 0x4c, 0x20);
    g.write::<u32>(p + 0x50, 0x10);
    g.write::<u32>(p + 0x54, 0xffff_fff0);
    g.write::<f32>(p + 0x3c, v);
    g.write::<u8>(p + 0x67, 0x40);
    g.write::<u8>(p + 0x6b, 0);
    g.write::<u32>(p + 0x60, 0);
    if rf(g, p + 4) > y0 {
        g.write::<f32>(p + 4, sub(y0, k0));
    }
    out.eax
}

/// [`sparks_0563`] (the `rel32`s: the pool take, the give-back,
/// `__security_check_cookie`).
pub const SPARKS_0563: Sig = Sig {
    len: 847,
    hash: 0x6139_6ff6_7967_f019,
    calls: &[
        (0x159, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
        (0x33d, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x347, Callee::Any),
    ],
    imports: &[],
};

/// An actor's model drawn by its effect record with a clip plane
/// (`magic_0434` 0x10003fc0 and `magic_0610`, 1242 bytes; cdecl `(obj)`;
/// a /GS frame of locals): the actor index `idx` whose character is `obj`
/// ([`actor_index`](crate::s4_small::actor_index); the search inlined
/// again for the actor `a` itself, `T[G_MAP[i]]` from 1); its model `u16
/// a[0]` picks the draw as in
/// [`actor_clip_draw`](crate::s5_misc::actor_clip_draw). With no actor
/// (`idx` -1) it returns -1 (the original reads the model at 0 first and
/// faults; so does the port's host). With `A = *G_S + 0xc08 + 0xc8 idx`: `M` =
/// slot 235 turned by slots 330, 328, 329 by `A + 0x88`, `+0x80`, `+0x84`;
/// `N` = slot 235; `Q` = slot 231 `(N, M)`, `M` = slot 230 `(Q)`; the
/// point `P` (slot 325) of `M` times `(0, A[0xac], 0, 0)` (229; the
/// vector's other words are never written: 0 here, junk reads); `N` =
/// slot 230 `(A)`, `Q` = 231 `(N, M)`, `M` = 230 `(Q)`; `P += N (0,
/// A[0xac] K, 0, 1)`; `W` = 230 `(M)`, `X` = 231 `(M, G_K)`. Then the
/// named parts with `W` ([`named_parts`](crate::s4_small::named_parts))
/// or the parts by `X`
/// ([`model_parts_draw`](crate::s5_misc::model_parts_draw)); the points
/// `v = G_A[idx]`, `n = G_B[idx]` (slot 325) through `W` (229) with their
/// `x` and `y` negated before and after, `n` normalised (232); the plane
/// `(n, -(n . v))` to slot 930 `(obj, plane)`, slot 833 `(obj, model,
/// A[0xb4] K2)` and slot 847 `(obj, 0)`. Returns slot 847's `eax`.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn actor_draw_0434<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, mul};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let rf = |g: &G, at: u32| g.read::<f32>(at);
    let obj = fr.arg(g, 0);
    let e = fr.esp;
    let ebp = e.wrapping_sub(4);
    let l = |o: u32| ebp.wrapping_sub(o);
    let index = Frame {
        at: img.rel32_target(f + 0x1b),
        esp: e.wrapping_sub(0x190),
        ecx: obj,
        ..*fr
    };
    let idx = crate::s4_small::actor_index(g, img, &index).eax;
    g.write::<u32>(l(0x17c), idx);
    let n = g.read::<i32>(w(0x21));
    let mut actor = 0;
    for i in 1..n.max(1) {
        let k = u32::from(g.read::<u8>(g.read::<u32>(w(0x42)).wrapping_add(i.cast_unsigned())));
        let c = g.read::<u32>(w(0x4d).wrapping_add(4 * k));
        if g.read::<u32>(c.wrapping_add(0x830)) == obj {
            actor = c;
            break;
        }
    }
    let model = g.read::<u16>(actor);
    let by_name = matches!(model, 0x10fb..=0x10fd | 0x1100);
    g.write::<u32>(l(0x180), u32::from(by_name));
    if idx == u32::MAX {
        return Ret::eax(idx);
    }
    let a = g
        .read::<u32>(w(0xca))
        .wrapping_add(0xc08)
        .wrapping_add(idx.wrapping_mul(0xc8));
    g.write::<u32>(l(0x178), a);
    let (m, nn, q) = (l(0x64), l(0xb4), l(0xf4));
    let (t, u, p, v) = (l(0xc4), l(0x84), l(0x34), l(0x14));
    let (wm, x, y, z) = (l(0x134), l(0x174), l(0x24), l(0x74));
    // call width: 1
    g.call(slot(g, 0xe6, 235), 0, &[m]);
    for (o, n, at) in [
        (0x10bu32, 330u32, 0x88u32),
        (0x133, 328, 0x80),
        (0x15b, 329, 0x84),
    ] {
        let turn = g.read::<u32>(a + at);
        // call width: 3
        g.call(slot(g, o, n), 0, &[m, m, turn]);
    }
    // call width: 1
    g.call(slot(g, 0x16f, 235), 0, &[nn]);
    let h = g.read::<u32>(a + 0xac);
    g.write::<u32>(l(0x80), h);
    // call width: 3
    g.call(slot(g, 0x19c, 231), 0, &[q, nn, m]);
    // call width: 2
    g.call(slot(g, 0x1b9, 230), 0, &[m, q]);
    // junk read: [ebp-0x84], [ebp-0x7c], [ebp-0x78] never written; 0 (brief 2026-10-24)
    for o in [0x84u32, 0x7c, 0x78] {
        g.write::<u32>(l(o), 0);
    }
    // call width: 3
    g.call(slot(g, 0x1d8, 229), 0, &[t, m, u]);
    // call width: 2
    g.call(slot(g, 0x1f0, 325), 0, &[p, t]);
    // call width: 2
    g.call(slot(g, 0x20a, 230), 0, &[nn, a]);
    // call width: 3
    g.call(slot(g, 0x22c, 231), 0, &[q, nn, m]);
    // call width: 2
    g.call(slot(g, 0x244, 230), 0, &[m, q]);
    g.write::<u32>(v, 0);
    let hy = mul(rf(g, a + 0xac), g.read::<f32>(w(0x269)));
    g.write::<f32>(v + 4, hy);
    g.write::<u32>(v + 8, 0);
    g.write::<u32>(v + 0xc, 0x3f80_0000);
    for o in [0x84u32, 0x80, 0x7c] {
        g.write::<u32>(l(o), 0);
    }
    // call width: 3
    g.call(slot(g, 0x27d, 229), 0, &[v, nn, v]);
    for i in 0..3u32 {
        let s = add(rf(g, p + 4 * i), rf(g, v + 4 * i));
        g.write::<f32>(p + 4 * i, s);
    }
    // call width: 2
    g.call(slot(g, 0x2ca, 230), 0, &[wm, m]);
    // call width: 3
    g.call(slot(g, 0x30a, 231), 0, &[x, m, w(0x2fa)]);
    if by_name {
        // push W; call: the callee's frame
        g.write::<u32>(e.wrapping_sub(0x190), wm);
        let named = Frame {
            at: img.rel32_target(f + 0x32e),
            esp: e.wrapping_sub(0x194),
            ecx: obj,
            edx: actor,
            ..*fr
        };
        crate::s4_small::named_parts(g, img, &named);
    } else {
        let at = img.rel32_target(f + 0x340);
        crate::s5_misc::model_parts_draw(g, img, at, e.wrapping_sub(0x190), actor, x);
    }
    let i16 = idx << 4;
    g.write::<u32>(l(0x17c), i16);
    // call width: 2
    g.call(slot(g, 0x35f, 325), 0, &[v, w(0x355).wrapping_add(i16)]);
    // call width: 2
    g.call(slot(g, 0x37d, 325), 0, &[y, w(0x373).wrapping_add(i16)]);
    let flip = |g: &mut G, at: u32, op: u32| {
        let s = g.read::<u32>(w(op));
        let x = g.read::<u32>(at) ^ s;
        g.write::<u32>(at, x);
    };
    flip(g, y, 0x391);
    flip(g, y + 4, 0x3ad);
    // call width: 3
    g.call(slot(g, 0x3b6, 229), 0, &[v, wm, v]);
    // call width: 3
    g.call(slot(g, 0x3d7, 229), 0, &[y, wm, y]);
    for at in [v, v + 4, y, y + 4] {
        flip(g, at, 0x3e7);
    }
    // call width: 2
    g.call(slot(g, 0x41d, 232), 0, &[y, y]);
    let [nx, ny, nz] = [0u32, 4, 8].map(|o| rf(g, y + o));
    g.write::<f32>(z + 4, ny);
    g.write::<f32>(z, nx);
    g.write::<f32>(z + 8, nz);
    let d = add(mul(ny, rf(g, v + 4)), mul(nx, rf(g, v)));
    let d = add(d, mul(nz, rf(g, v + 8)));
    let d = d.to_bits() ^ g.read::<u32>(w(0x479));
    g.write::<u32>(z + 0xc, d);
    // call width: 2
    g.call(slot(g, 0x467, 930), 0, &[obj, z]);
    #[allow(clippy::cast_precision_loss)] // cvtdq2ps
    let s = g.read::<i32>(a + 0xb4) as f32;
    let s = mul(s, g.read::<f32>(w(0x49f)));
    let model = u32::from(g.read::<u16>(actor));
    // call width: 3
    g.call(slot(g, 0x4aa, 833), 0, &[obj, model, s.to_bits()]);
    // call width: 2
    let out = g.call(slot(g, 0x4b8, 847), 0, &[obj, 0]);
    Ret::eax(out.eax)
}

/// [`actor_draw_0434`] (the `rel32`s: the actor index, the named parts,
/// the parts draw, `__security_check_cookie`).
pub const ACTOR_DRAW_0434: Sig = Sig {
    len: 1242,
    hash: 0xcc95_0793_464a_ddb6,
    calls: &[
        (0x1b, Callee::Sig(&crate::s4_small::ACTOR_INDEX)),
        (0x32e, Callee::Sig(&crate::s4_small::NAMED_PARTS)),
        (0x340, Callee::Sig(&crate::s5_misc::MODEL_PARTS_DRAW)),
        (0x4d2, Callee::Any),
    ],
    imports: &[],
};

/// The nearest hit of a segment on a model's triangles (`magic_0150`
/// 0x10007690 and `magic_0603`, 1372 bytes; internal: `ecx` = the parts
/// list `L` (count byte `+4`, entries from `+0x10`, 16 bytes each), `edx`
/// = the point `P0`, cdecl `(P1, out, far)`; a /GS frame, the triangle's
/// points `A`, `B`, `C` at `ebp - 0x34`, `-0x54`, `-0x64`, its edges at
/// `-0x84`, `-0x74`, `-0x94`, the normal `N` at `-0x24`, `X` at `-0x14`
/// and the hit `Y` at `-0x44`). A record `R` of 0xdc bytes from slot 264
/// (given back by slot 265 at the end): its best `t` `+0xd0` starts at
/// 1.0 (`far` set) or -1.0, the flag `+0xd4` 0, `+0x80` = `P0`, `+0x90` =
/// `P1` (`w` 0 and 1). For each part (`L + 0x10 + 16 i`): its model `D` =
/// slot 941 `(e[0])` (the vertices `V = D + D[8]`, the groups from `D +
/// D[4]`), its matrix slot 230 `(R + 0x40, 941 (e[1]))` with the scale
/// row times `K`, slot 327 `(R, R + 0x40)` and the segment through it
/// (229 into `R + 0xa0`, `R + 0xb0`). Each group (kind byte `+1` 0x10, a
/// u16 count `+2`, 28-byte triangles from `+0x20`; 0xff ends the part,
/// any other kind ends all): `A`, `B`, `C` copied (325) from `V`, the
/// edges (234), `N` (266), `t = (N.b - N.A) / N.a` (slot 394, the dot
/// product; `a`, `b` the segment's ends), `X` = slot 233 `(a, t)`, `Y` =
/// `b - X`... (234), `X` = `Y - B` (234) and its cross with each edge
/// (266) must not face away from `N` (394 below 0 skips); a `t` beyond
/// the best (below with `far`, above without) becomes the best, the flag
/// 1 and `out` = slot 229 `(R + 0x40, Y)`. Returns the flag.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn ray_hit_0150<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let ebp = fr.esp.wrapping_sub(4);
    let l = |o: u32| ebp.wrapping_sub(o);
    let (list, p0) = (fr.ecx, fr.edx);
    let (p1, out, far) = (fr.arg(g, 0), fr.arg(g, 1), fr.arg(g, 2));
    g.write::<u32>(l(0xa0), p1);
    g.write::<u32>(l(0xa4), out);
    // call width: 1
    let r = g.call(slot(g, 0x28, 264), 0, &[0xdc]).eax;
    g.write::<u32>(r + 0xc4, list.wrapping_add(0x10));
    let parts = g.read::<u8>(list.wrapping_add(4));
    g.write::<u8>(r + 0xda, parts);
    g.write::<u32>(r + 0xd4, 0);
    g.write::<u8>(r + 0xdb, far.to_le_bytes()[0]);
    g.write::<u32>(r + 0xd0, if far != 0 { 0x3f80_0000 } else { 0xbf80_0000 });
    // call width: 2
    g.call(slot(g, 0x8b, 325), 0, &[r + 0x80, p0]);
    g.write::<u32>(r + 0x8c, 0);
    // call width: 2
    g.call(slot(g, 0xa5, 325), 0, &[r + 0x90, p1]);
    g.write::<u32>(r + 0x9c, 0x3f80_0000);
    let (a, b, c) = (l(0x34), l(0x54), l(0x64));
    let (e1, e2, e3) = (l(0x84), l(0x74), l(0x94));
    let (n, x, y) = (l(0x24), l(0x14), l(0x44));
    let (sa, sb, m) = (r + 0xa0, r + 0xb0, r + 0x40);
    let dot = |g: &mut G, o: u32, u: u32, v: u32| {
        // call width: 2
        st0_f32(g.call(slot(g, o, 394), 0, &[u, v]).st0)
    };
    let mut kinds_ok = g.read::<u8>(r + 0xda) != 0;
    while kinds_ok {
        let e = g.read::<u32>(r + 0xc4);
        let first = g.read::<u32>(e);
        // call width: 1
        let d = g.call(slot(g, 0xe9, 941), 0, &[first]).eax;
        g.write::<u32>(r + 0xcc, d);
        let verts = d.wrapping_add(g.read::<u32>(d.wrapping_add(8)));
        g.write::<u32>(l(0xa0), verts);
        let groups = d.wrapping_add(g.read::<u32>(d.wrapping_add(4)));
        let e = g.read::<u32>(r + 0xc4);
        g.write::<u32>(r + 0xc8, groups);
        let second = g.read::<u32>(e + 4);
        // call width: 1
        let mtx = g.call(slot(g, 0x11b, 941), 0, &[second]).eax;
        // call width: 2
        g.call(slot(g, 0x129, 230), 0, &[m, mtx]);
        let k = g.read::<f32>(w(0x13f));
        for o in [0x70u32, 0x74, 0x78] {
            let v = mul(g.read::<f32>(r + o), k);
            g.write::<f32>(r + o, v);
        }
        // call width: 2
        g.call(slot(g, 0x144, 327), 0, &[r, m]);
        // call width: 3
        g.call(slot(g, 0x17f, 229), 0, &[sa, r, r + 0x80]);
        // call width: 3
        g.call(slot(g, 0x19d, 229), 0, &[sb, r, r + 0x90]);
        loop {
            let grp = g.read::<u32>(r + 0xc8);
            if g.read::<u8>(grp.wrapping_add(1)) != 0x10 {
                kinds_ok = false;
                break;
            }
            let count = g.read::<u16>(grp.wrapping_add(2));
            let mut tri = grp.wrapping_add(0x20);
            g.write::<u16>(r + 0xd8, count);
            if count.cast_signed() > 0 {
                loop {
                    let verts = g.read::<u32>(l(0xa0));
                    for (dst, o, op) in [(a, 0u32, 0x1f2u32), (b, 2, 0x211), (c, 4, 0x230)] {
                        let i = u32::from(g.read::<u16>(tri.wrapping_add(o)));
                        // call width: 2
                        g.call(slot(g, op, 325), 0, &[dst, verts.wrapping_add(i << 4)]);
                    }
                    for (dst, u, v, op) in
                        [(e1, b, a, 0x24cu32), (e2, c, b, 0x265), (e3, a, c, 0x281)]
                    {
                        // call width: 3
                        g.call(slot(g, op, 234), 0, &[dst, u, v]);
                    }
                    // call width: 3
                    g.call(slot(g, 0x29d, 266), 0, &[n, e1, e2]);
                    let d1 = dot(g, 0x2b8, n, sb);
                    g.write::<f32>(l(0x9c), d1);
                    let d2 = dot(g, 0x2d3, n, a);
                    g.write::<f32>(l(0x98), d2);
                    let num = sub(d1, d2);
                    g.write::<f32>(l(0x9c), num);
                    let den = dot(g, 0x301, n, sa);
                    g.write::<f32>(l(0x98), den);
                    let t = div(num, den);
                    g.write::<f32>(l(0x9c), t);
                    // call width: 3
                    g.call(slot(g, 0x33f, 233), 0, &[x, sa, t.to_bits()]);
                    // call width: 3
                    g.call(slot(g, 0x363, 234), 0, &[y, sb, x]);
                    let mut inside = true;
                    for (corner, edge, o234, o266, o394) in [
                        (b, e1, 0x37cu32, 0x398u32, 0x3adu32),
                        (c, e2, 0x3df, 0x3f8, 0x40d),
                        (a, e3, 0x43f, 0x45b, 0x470),
                    ] {
                        // call width: 3
                        g.call(slot(g, o234, 234), 0, &[x, y, corner]);
                        // call width: 3
                        g.call(slot(g, o266, 266), 0, &[x, edge, x]);
                        let s = dot(g, o394, n, x);
                        g.write::<f32>(l(0x98), s);
                        if 0.0 > s {
                            inside = false;
                            break;
                        }
                    }
                    if inside {
                        let best = g.read::<f32>(r + 0xd0);
                        let nearer = if g.read::<u8>(r + 0xdb) != 0 {
                            best >= t
                        } else {
                            t > best
                        };
                        if nearer {
                            g.write::<f32>(r + 0xd0, t);
                            g.write::<u32>(r + 0xd4, 1);
                            // call width: 3
                            g.call(slot(g, 0x4bf, 229), 0, &[out, m, y]);
                        }
                    }
                    let left = g.read::<u16>(r + 0xd8).wrapping_sub(1);
                    g.write::<u16>(r + 0xd8, left);
                    tri = tri.wrapping_add(0x1c);
                    if left.cast_signed() <= 0 {
                        break;
                    }
                }
            }
            g.write::<u32>(r + 0xc8, tri);
            if g.read::<u8>(tri.wrapping_add(1)) == 0xff {
                break;
            }
        }
        if !kinds_ok {
            break;
        }
        let left = g.read::<u8>(r + 0xda).wrapping_sub(1);
        g.write::<u8>(r + 0xda, left);
        let e = g.read::<u32>(r + 0xc4);
        g.write::<u32>(r + 0xc4, e.wrapping_add(0x10));
        kinds_ok = left != 0;
    }
    // call width: 1
    g.call(slot(g, 0x531, 265), 0, &[0xdc]);
    Ret::eax(g.read::<u32>(r + 0xd4))
}

/// [`ray_hit_0150`] (the `rel32`: `__security_check_cookie`).
pub const RAY_HIT_0150: Sig = Sig {
    len: 1372,
    hash: 0x2920_1bec_91f8_4a0f,
    calls: &[(0x554, Callee::Any)],
    imports: &[],
};

/// The summon's strip handler (`magic_0150` 0x100070e0 and `magic_0603`,
/// 1452 bytes; internal, `ecx` = the handle `H` (`+0` its node `N`, byte
/// `+4` the step flag, byte `+5` the pass); an 8-aligned /GS frame `Z`
/// (the `esp` after the pushes): the request `Z + 0xc`, the bone cursor
/// `Z + 0x20`, the vectors `L1..L5` at `Z + 0x40`, `+0x30`, `+0x50`,
/// `+0x60`, `+0x70`, the matrices `M1` `Z + 0x80` and `M2` `Z + 0xc0`).
/// - A pass already seen (`N` byte `+0x1a` = the pass): the flag 0;
///   else each sibling from `N + 0x10` round to `N` becomes `H[0]` and its
///   vtable's `+0x1c` runs `(H)`, then `H[0] = N`, the pass stored, the
///   flag 1. Without the flag it stops.
/// - The two bones `N + 0xdc` and `N + 0xe0` stepped
///   ([`bone_plain`](crate::sa_summon::bone_plain), the pass in `eax`);
///   stops while `*G_C` is set.
/// - While `u16 N + 0xee`: by the kind `N + 0xf4` (0x1071, 0x1088, 0x108c;
///   0x1089 far) the actor `A = N[0xf0][0x54]`'s matrix (210) scaled by
///   `K / slot 277 (A)` and slot 15 `(A) K2`, the segment between the two
///   bones' points (229, 233, 234, 248) and its hit on the parts `N +
///   0xfc` ([`ray_hit_0150`]) into `N + 0xf8`: hit, `L2` the hit (229)
///   with `w` 0.98; missed, `L2` 229 then moved by the bone (234, 248),
///   `w` 1.0. Other kinds: `L2 = (10, N[0xe4] K, 0, 1)` through the first
///   bone (229), `y` = `*G_Y`, `w` 0.98, `N + 0xf8` 1. Then a ring slot
///   ([`ring_take`](crate::s6_misc::ring_take)) gets `L2` (325), `N +
///   0xcc` = the direction to the second bone (234, 232), `w` 1.0; while
///   fewer than 10 points (`u16 N + 0xec`) and `N + 0xe4` reaches the
///   count times `K3`, the point is kept at `N + 0x2c + 16 i` with `w`
///   the hit (1.0 or 0).
/// - Unless `*G_C`: the strip's window
///   ([`strip_window`](crate::s4_small::strip_window)) fades its two
///   pieces' alphas (`+0xc` of 16-byte entries: at most `K4`, less `K5`,
///   not below 0); `N + 0xe8 += K6`, `N + 0xe4` its square.
///
/// Returns `eax` as the original leaves it (the last alpha's address, the
/// window's, slot 325's, the count, a bone's `eax`); after the pass test
/// fails, and in a bone's `eax`, the upper bytes are a frame's cookie
/// (junk).
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn strip_task_0150<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let rf = |g: &G, at: u32| g.read::<f32>(at);
    let h = fr.ecx;
    let z = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x108);
    let at = |o: u32| z.wrapping_add(o);
    // the walk over the node's children (their vtable `+0x1c` with `ecx`
    // = H); the flag `H + 4` set when it ran
    let (eax, moved) = crate::nodes::walk(g, h, h, fr.eax);
    if moved.is_none() {
        // the cookie's upper bytes are not the port's (`eax` junk)
        return Ret::eax(eax);
    }
    let n = g.read::<u32>(h);
    let bone = |g: &mut G, o: u32, call: u32| {
        let b = g.read::<u32>(n + o);
        g.write::<u32>(at(0x20), b);
        let pass = g.read::<u8>(h + 5);
        g.write::<u8>(at(0x25), pass);
        let fr = Frame {
            at: img.rel32_target(f + call),
            esp: z.wrapping_sub(4),
            ecx: at(0x20),
            eax: u32::from(pass),
            ..*fr
        };
        crate::sa_summon::bone_plain(g, img, &fr).eax
    };
    bone(g, 0xdc, 0x7b);
    let mut eax = bone(g, 0xe0, 0x96);
    if g.read::<u32>(w(0x9c)) != 0 {
        return Ret::eax(eax);
    }
    if g.read::<u16>(n + 0xee) != 0 {
        g.write::<u16>(at(0x10), 1);
        let kind = g.read::<u32>(n + 0xf4);
        g.write::<u32>(at(0xc), n);
        g.write::<u32>(n + 0xf8, 0);
        let (l1, l2, l3, l4, l5) = (at(0x40), at(0x30), at(0x50), at(0x60), at(0x70));
        let (m1, m2) = (at(0x80), at(0xc0));
        let far = match kind.wrapping_sub(0x1071) {
            0 | 0x17 | 0x1b => Some(0u32),
            0x18 => Some(1),
            _ => None,
        };
        if let Some(far) = far {
            let y = mul(rf(g, n + 0xe4), k(g, 0xfe));
            g.write::<f32>(l1 + 4, y);
            g.write::<u32>(l1, 0x4120_0000);
            g.write::<u32>(l1 + 8, 0);
            g.write::<u32>(l1 + 0xc, 0x3f80_0000);
            let b0 = g.read::<u32>(n + 0xdc).wrapping_add(0x9c);
            // call width: 3
            g.call(slot(g, 0x137, 229), 0, &[l2, b0, l1]);
            let actor = |g: &G| g.read::<u32>(g.read::<u32>(n + 0xf0).wrapping_add(0x54));
            let a = actor(g);
            // call width: 2
            g.call(slot(g, 0x158, 210), 0, &[a, m1]);
            let a = actor(g);
            // call width: 1
            let s = st0_f32(g.call(slot(g, 0x16e, 277), 0, &[a]).st0);
            g.write::<f32>(at(0x20), s);
            let inv = div(k(g, 0x182), s);
            g.write::<f32>(l1, inv);
            let a = actor(g);
            // call width: 1
            let r = st0_f32(g.call(slot(g, 0x19f, 15), 0, &[a]).st0);
            g.write::<f32>(at(0x20), r);
            let sc = mul(mul(r, k(g, 0x1b6)), rf(g, l1));
            g.write::<f32>(l1, sc);
            // call width: 2
            g.call(slot(g, 0x1d4, 327), 0, &[m2, m1]);
            // call width: 3
            g.call(slot(g, 0x1f9, 229), 0, &[l3, m2, l2]);
            let sc = g.read::<u32>(l1);
            // call width: 3
            g.call(slot(g, 0x21a, 233), 0, &[l3, l3, sc]);
            g.write::<u32>(l3 + 0xc, 0x3f80_0000);
            let b1 = g.read::<u32>(n + 0xe0).wrapping_add(0xcc);
            // call width: 3
            g.call(slot(g, 0x24b, 229), 0, &[l4, m2, b1]);
            let sc = g.read::<u32>(l1);
            // call width: 3
            g.call(slot(g, 0x26c, 233), 0, &[l4, l4, sc]);
            g.write::<u32>(l3 + 0xc, 0x3f80_0000);
            // call width: 3
            g.call(slot(g, 0x28b, 234), 0, &[l5, l3, l4]);
            // call width: 3
            g.call(slot(g, 0x2b0, 233), 0, &[l5, l5, 0x4000_0000]);
            // call width: 3
            g.call(slot(g, 0x2cf, 248), 0, &[l3, l4, l5]);
            // the hit: `(P1 = L4, out = L3, far)` on the stack, `edx` = L5
            let list = g.read::<u32>(n + 0xfc);
            g.write::<u32>(z.wrapping_sub(0xc), l4);
            g.write::<u32>(z.wrapping_sub(8), l3);
            g.write::<u32>(z.wrapping_sub(4), far);
            let hit = Frame {
                at: img.rel32_target(f + 0x2f4),
                esp: z.wrapping_sub(0x10),
                ecx: list,
                edx: l5,
                ..*fr
            };
            let found = ray_hit_0150(g, img, &hit).eax;
            g.write::<u32>(n + 0xf8, found);
            g.write::<u32>(l3 + 0xc, 0x3f80_0000);
            // call width: 3
            g.call(slot(g, 0x316, 229), 0, &[l2, m1, l3]);
            if found != 0 {
                g.write::<u32>(l2 + 0xc, 0x3f7a_e148);
            } else {
                let b1 = g.read::<u32>(n + 0xe0).wrapping_add(0xcc);
                // call width: 3
                g.call(slot(g, 0x355, 234), 0, &[l1, l2, b1]);
                let b1 = g.read::<u32>(n + 0xe0).wrapping_add(0xcc);
                // call width: 3
                g.call(slot(g, 0x37b, 248), 0, &[l2, b1, l1]);
                g.write::<u32>(l2 + 0xc, 0x3f80_0000);
            }
        } else {
            let y = mul(rf(g, n + 0xe4), k(g, 0x39d));
            g.write::<f32>(l2 + 4, y);
            g.write::<u32>(l2, 0x4120_0000);
            g.write::<u32>(l2 + 8, 0);
            g.write::<u32>(l2 + 0xc, 0x3f80_0000);
            let b0 = g.read::<u32>(n + 0xdc).wrapping_add(0x9c);
            // call width: 3
            g.call(slot(g, 0x3d2, 229), 0, &[l2, b0, l2]);
            let y = g.read::<u32>(w(0x3e2));
            g.write::<u32>(l2 + 0xc, 0x3f7a_e148);
            g.write::<u32>(l2 + 4, y);
            g.write::<u32>(n + 0xf8, 1);
        }
        let take = Frame {
            at: img.rel32_target(f + 0x406),
            esp: z.wrapping_sub(4),
            ecx: at(0xc),
            ..*fr
        };
        crate::s6_misc::ring_take(g, &take);
        let dst = g.read::<u32>(at(0x18));
        // call width: 2
        g.call(slot(g, 0x410, 325), 0, &[dst, l2]);
        let b1 = g.read::<u32>(n + 0xe0).wrapping_add(0xcc);
        // call width: 3
        g.call(slot(g, 0x437, 234), 0, &[l1, l2, b1]);
        // call width: 2
        g.call(slot(g, 0x453, 232), 0, &[n + 0xcc, l1]);
        g.write::<u32>(n + 0xd8, 0x3f80_0000);
        let count = g.read::<u16>(n + 0xec);
        eax = u32::from(count);
        #[allow(clippy::cast_precision_loss)] // cvtdq2ps of a u16
        let need = mul(f32::from(count), k(g, 0x48b));
        if count < 10 && rf(g, n + 0xe4) >= need {
            g.write::<u32>(l2 + 0xc, 0x3f80_0000);
            if g.read::<u32>(n + 0xf8) == 0 {
                g.write::<u32>(l2 + 0xc, 0);
            }
            let i = u32::from(g.read::<u16>(n + 0xec));
            // call width: 2
            eax = g
                .call(slot(g, 0x4c3, 325), 0, &[n + 0x2c + (i << 4), l2])
                .eax;
            let c = g.read::<u16>(n + 0xec).wrapping_add(1);
            g.write::<u16>(n + 0xec, c);
        }
        if g.read::<u32>(w(0x4db)) != 0 {
            return Ret::eax(eax);
        }
    }
    g.write::<u32>(at(0x20), n);
    let win = Frame {
        at: img.rel32_target(f + 0x4ef),
        esp: z.wrapping_sub(4),
        ecx: at(0x20),
        ..*fr
    };
    eax = crate::s4_small::strip_window(g, img, &win).eax;
    let step = k(g, 0x4f7);
    for i in 0..2u32 {
        let count = g.read::<u16>(at(0x24 + 2 * i));
        if count == 0 {
            continue;
        }
        let top = k(g, 0x511);
        let mut p = g.read::<u32>(at(0x28 + 4 * i)).wrapping_add(0xc);
        for _ in 0..count {
            let x = rf(g, p);
            if top >= x {
                let x = sub(x, step);
                g.write::<f32>(p, x);
                if 0.0 > x {
                    g.write::<u32>(p, 0);
                }
            }
            p = p.wrapping_add(0x10);
        }
        eax = p;
    }
    let e8 = add(rf(g, n + 0xe8), k(g, 0x557));
    g.write::<f32>(n + 0xe8, e8);
    g.write::<f32>(n + 0xe4, mul(e8, e8));
    Ret::eax(eax)
}

/// [`strip_task_0150`] (the `rel32`s: the two bones, the hit, the ring
/// take, the window, `__security_check_cookie`; the case jump at `+0x334`
/// stays inside).
pub const STRIP_TASK_0150: Sig = Sig {
    len: 1452,
    hash: 0x0a9d_4e98_2e30_11e9,
    calls: &[
        (0x7b, Callee::Sig(&crate::sa_summon::BONE)),
        (0x96, Callee::Sig(&crate::sa_summon::BONE)),
        (0x2f4, Callee::Sig(&RAY_HIT_0150)),
        (0x406, Callee::Sig(&crate::s6_misc::RING_TAKE)),
        (0x4ef, Callee::Sig(&crate::s4_small::STRIP_WINDOW)),
        (0x57c, Callee::Any),
    ],
    imports: &[],
};

/// A strip's draw packet (`magic_0150` 0x10006210 and `magic_0603`, 1935
/// bytes; internal, `ecx` = the request `C`: `+0` the write cursor, `+0xc`
/// a matrix, `+0x14`, `+0x18`, `+0x1c` three colours (four floats each),
/// `+0x20` the width, `+0x24` and `+0x30` the two ends; a /GS frame). At
/// `D = *C`: a header (`u16 +0x10` 1, `+0x12` 10, `+0x14` 0x480000, the
/// vertices `V = (D + 0x2f) & !0xf` and indices `I = (V + 0xa3) & !3` as
/// offsets from `D`), `M` = slot 230 `(C[0xc])` (`ebp - 0x148`), the ends
/// `A`, `B` through it (229, `w` 1.0), `B -= A` (234) and normalised
/// (232) into `U`; with `w = C[0x20]`: `Q = (U.y w, -U.x w) w K`, `U =
/// (U.y w, -U.x w, 0, 0)`. The colours as bytes (`cvttss2si` of each
/// float times `K2`, packed little-endian). Two rows of five vertices
/// along `T = A + B t` (233, 248): `T + Q`, `T + U`, `T`, `T - U`, `T -
/// Q` (`w` 1.0) at `t` = 0 and the first row of `t = K3` repeated five
/// times (the loop's count is the constant 1, so the forward copy after
/// it never runs); the index records (`0x40400`, then 0x60 bytes: the
/// colour words and the u16 vertex numbers 0..9). The cursor moves past
/// them, 16-aligned, with the byte before it 0xff. Returns the new
/// cursor.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn strip_mesh_0150<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, cvtt, mul};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let rf = |g: &G, at: u32| g.read::<f32>(at);
    let c = fr.ecx;
    let ebp = fr.esp.wrapping_sub(4);
    let l = |o: u32| ebp.wrapping_sub(o);
    let one = 0x3f80_0000u32;
    g.write::<u32>(l(0x190), c);
    let d = g.read::<u32>(c);
    let v = d.wrapping_add(0x2f) & !0xf;
    let idx = v.wrapping_add(0xa3) & !3;
    g.write::<u16>(d + 0x12, 10);
    g.write::<u32>(d, 0);
    let base = g.read::<u32>(c);
    g.write::<u32>(d + 4, idx.wrapping_sub(base));
    g.write::<u32>(l(0x17c), v);
    g.write::<u32>(l(0x184), idx);
    let base = g.read::<u32>(c);
    g.write::<u32>(d + 0xc, 0);
    g.write::<u32>(d + 8, v.wrapping_sub(base));
    g.write::<u16>(d + 0x10, 1);
    g.write::<u32>(d + 0x14, 0x48_0000);
    g.write::<u32>(d + 0x1c, 0);
    let rec = idx + 0x10;
    g.write::<u32>(l(0x1a4), rec);
    let (m, p, a, b, u) = (l(0x148), l(0x34), l(0xb8), l(0x54), l(0x24));
    let (q, t) = (l(0x44), l(0x14));
    let mtx = g.read::<u32>(c + 0xc);
    // call width: 2
    g.call(slot(g, 0x80, 230), 0, &[m, mtx]);
    for (dst, src, op) in [(a, 0x24u32, 0xc1u32), (b, 0x30, 0x102)] {
        for i in 0..3 {
            let x = g.read::<u32>(c + src + 4 * i);
            g.write::<u32>(p + 4 * i, x);
        }
        g.write::<u32>(p + 0xc, one);
        // call width: 3
        g.call(slot(g, op, 229), 0, &[dst, m, p]);
    }
    // call width: 3
    g.call(slot(g, 0x127, 234), 0, &[b, b, a]);
    // call width: 2
    g.call(slot(g, 0x13c, 232), 0, &[u, b]);
    let wd = rf(g, c + 0x20);
    let ux = mul(rf(g, u), wd);
    let uy = mul(rf(g, u + 4), wd);
    let ww = mul(wd, k(g, 0x169));
    let nx = f32::from_bits(ux.to_bits() ^ g.read::<u32>(w(0x170)));
    g.write::<f32>(u + 4, nx);
    g.write::<f32>(u, uy);
    g.write::<f32>(q + 4, mul(nx, ww));
    g.write::<f32>(q, mul(uy, ww));
    g.write::<u32>(l(0x1a0), 0);
    for o in [0x18u32, 0x1c, 0x38, 0x3c] {
        g.write::<u32>(l(o), 0);
    }
    // the colours as bytes
    let k2 = k(g, 0x1a5);
    let byte = |g: &G, col: u32, i: u32| {
        let at = g.read::<u32>(c + col).wrapping_add(4 * i);
        cvtt(mul(rf(g, at), k2)).cast_unsigned().to_le_bytes()[0]
    };
    for i in 0..4u32 {
        let x = byte(g, 0x1c, i);
        g.write::<u8>(l(0x158 - 4 * i), x);
        let x = byte(g, 0x18, i);
        g.write::<u8>(l(0x178 - 4 * i), x);
        let x = byte(g, 0x14, i);
        g.write::<u8>(l(0x168 - 4 * i), x);
    }
    let word = |g: &G, at: u32| u32::from_le_bytes([0, 4, 8, 12].map(|o| g.read::<u8>(l(at - o))));
    // call width: 3
    g.call(slot(g, 0x242, 233), 0, &[t, b, 0]);
    // call width: 3
    g.call(slot(g, 0x25b, 248), 0, &[t, t, a]);
    g.write::<u32>(idx, 0x4_0400);
    g.write::<u32>(idx + 8, 0);
    g.write::<u32>(idx + 0xc, 0);
    let c0 = word(g, 0x158);
    g.write::<u32>(l(0x194), c0);
    let c1 = word(g, 0x178);
    g.write::<u32>(l(0x19c), c1);
    let c2 = word(g, 0x168);
    g.write::<u32>(l(0x198), c2);
    // the five points of a row at `row` (`l(row)`, 16 bytes apart)
    let five = |g: &mut G, row: u32, ops: [u32; 5]| {
        let r = |i: u32| l(row - 0x10 * i);
        // call width: 3
        g.call(slot(g, ops[0], 248), 0, &[r(0), t, q]);
        g.write::<u32>(r(0) + 0xc, one);
        // call width: 3
        g.call(slot(g, ops[1], 248), 0, &[r(1), t, u]);
        g.write::<u32>(r(1) + 0xc, one);
        // call width: 2
        g.call(slot(g, ops[2], 325), 0, &[r(2), t]);
        g.write::<u32>(r(2) + 0xc, one);
        // call width: 3
        g.call(slot(g, ops[3], 234), 0, &[r(3), t, u]);
        g.write::<u32>(r(3) + 0xc, one);
        // call width: 3
        g.call(slot(g, ops[4], 234), 0, &[r(4), t, q]);
        g.write::<u32>(r(4) + 0xc, one);
    };
    five(g, 0xa8, [0x31e, 0x340, 0x362, 0x385, 0x3a8]);
    let v0 = g.read::<u32>(l(0x17c));
    for i in 0..20 {
        let x = g.read::<u32>(l(0xa8) + 4 * i);
        g.write::<u32>(v0 + 4 * i, x);
    }
    let v1 = v0 + 0x50;
    g.write::<u32>(l(0x17c), v1);
    g.write::<u32>(l(0x18c), 5);
    g.write::<u32>(l(0x184), 0);
    g.write::<u32>(l(0x188), v1 + 0x20);
    g.write::<u32>(l(0x180), l(0x108));
    // the loop (once)
    let s = add(rf(g, l(0x1a0)), k(g, 0x48c));
    g.write::<f32>(l(0x1a0), s);
    // call width: 3
    g.call(slot(g, 0x49f, 233), 0, &[t, b, s.to_bits()]);
    // call width: 3
    g.call(slot(g, 0x4c0, 248), 0, &[t, t, a]);
    five(g, 0x108, [0x4dc, 0x4f8, 0x51a, 0x540, 0x569]);
    let src = l(0x108);
    for i in 0..5 {
        for j in 0..4 {
            let x = g.read::<u32>(src + 4 * j);
            g.write::<u32>(v1 + 0x10 * i + 4 * j, x);
        }
    }
    // the index records
    let n = 5u16;
    for (o, x) in [
        (4u32, c1),
        (0xc, c1),
        (0x1c, c2),
        (0x24, c2),
        (0x30, c2),
        (0x38, c2),
    ] {
        g.write::<u32>(rec + o, x);
    }
    for (o, x) in [
        (0x4cu32, c0),
        (0x18, c1),
        (0x20, c1),
        (0x34, c1),
        (0x3c, c1),
        (0x48, c1),
    ] {
        g.write::<u32>(rec + o, x);
    }
    for (o, x) in [(0x50u32, c1), (0, c0), (8, c0), (0x54, c0)] {
        g.write::<u32>(rec + o, x);
    }
    for (o, d) in [
        (0x10u32, -5i16),
        (0x12, -4),
        (0x28, -4),
        (0x2a, -3),
        (0x40, -3),
        (0x42, -2),
        (0x58, -2),
        (0x5a, -1),
        (0x16, 1),
        (0x2c, 1),
        (0x5e, 4),
        (0x2e, 2),
        (0x44, 2),
        (0x14, 0),
        (0x46, 3),
        (0x5c, 3),
    ] {
        g.write::<u16>(rec + o, n.wrapping_add_signed(d));
    }
    g.write::<u32>(l(0x184), 1);
    g.write::<u32>(l(0x180), l(0x108) + 0x10);
    let end = rec + 0x60;
    let cur = (end.wrapping_add(3) & !3).wrapping_add(0x1f);
    g.write::<u8>(cur.wrapping_sub(0x1e), 0xff);
    let cur = cur & !0xf;
    let c = g.read::<u32>(l(0x190));
    g.write::<u32>(c, cur);
    Ret::eax(cur)
}

/// [`strip_mesh_0150`] (the `rel32`: `__security_check_cookie`; the
/// loop's back jump stays inside).
pub const STRIP_MESH_0150: Sig = Sig {
    len: 1935,
    hash: 0x547d_a943_eae5_037e,
    calls: &[(0x787, Callee::Any)],
    imports: &[],
};

/// The summon strip's draw (`magic_0150` 0x100069a0 and `magic_0603`, 1215
/// bytes; a draw callback, cdecl `(obj)`; a 16-aligned /GS frame `Z`, the
/// `esp` after the pushes, the request `C` at `Z + 0x20` and the packet
/// record `R` at `Z + 0x60`). Every call to the game runs with the word
/// at `*G_F` set to 1 and restored after.
/// - `obj[0x930]` at most 0 (not NaN): slot 311 `(obj[0x93c])`, slot 772 `(obj,
///   0x54c55f)` and slot 779 with its 64-bit result.
/// - Else the strip's window (`obj[0x920]`,
///   [`strip_window`](crate::s4_small::strip_window)): `P1` = its second
///   piece's last point, `P2` = the bone point `obj[0x924] + 0xcc` (325);
///   `M` = slot 231 `(G_M, obj[0x91c] + 0x9c)`, `M2` = 230 `(M)`, `M3` =
///   235; the buffer `G_BUF` cleared (`memset`, 0x4000 bytes); the
///   colours `(0.9375, 0.5, 0.5, s)`, `(0.5, 0.125, 0, s K1)`, `(0, 0.125,
///   0.5, s K2)` (`s = obj[0x930]`), the width `(slot 307 - K3) K4 + K5`
///   and the ends `P2`, `P1` into `C`; the packet
///   ([`strip_mesh_0150`]); `R` from the buffer's header, `M2`, the
///   texture words; slot 772 `(obj, 0x54c55f)` gives a 64-bit handle `h`;
///   slot 773 `(h)` true: slots 819 `(h, C, R)` and 865 `(h,
///   0x80000000)`; else slot 820 `(h, C, R, 776 (R byte 8) | 777 (M2))`.
///
/// Returns the pointer `*G_F` (as read last). `C + 0x10`, `C + 0x3c`,
/// `R + 0xa` (u16) and `R + 0x14` are never written by the original
/// (junk reads by the game's packet calls): 0 here (brief 2026-10-24).
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn strip_draw_0150<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let obj = fr.arg(g, 0);
    let z = (fr.esp.wrapping_sub(4) & !0xf).wrapping_sub(0x1f0);
    let at = |o: u32| z.wrapping_add(o);
    // a call with the flag word at `*G_F` 1, restored after
    let flagged = |g: &mut G, (op, table): (u32, u32), n: u32, args: &[u32]| {
        let p = g.read::<u32>(w(op));
        let saved = g.read::<u32>(p);
        g.write::<u32>(p, 1);
        let out = g.call(slot(g, table, n), 0, args);
        (out, saved)
    };
    let s = g.read::<f32>(obj + 0x930);
    // `comiss 0, s; jb`: a NaN size takes the long way
    if s <= 0.0 {
        let x = g.read::<u32>(obj + 0x93c);
        // call width: 1
        g.call(slot(g, 0x2c, 311), 0, &[x]);
        // call width: 2
        let h = g.call(slot(g, 0x3f, 772), 0, &[obj, 0x54_c55f]);
        // call width: 2
        let (_, saved) = flagged(g, (0x56, 0x5f), 779, &[h.eax, h.edx]);
        let p = g.read::<u32>(w(0x72));
        g.write::<u32>(p, saved);
        return Ret::eax(p);
    }
    let cursor = g.read::<u32>(obj + 0x920);
    g.write::<u32>(at(0x20), cursor);
    let win = Frame {
        at: img.rel32_target(f + 0x9e),
        esp: z.wrapping_sub(4),
        ecx: at(0x20),
        ..*fr
    };
    crate::s4_small::strip_window(g, img, &win);
    let (p1, p2, m, m2, m3) = (at(0xd8), at(0x118), at(0x1a8), at(0x128), at(0x168));
    let n = u32::from(g.read::<u16>(at(0x26)));
    let last = g
        .read::<u32>(at(0x2c))
        .wrapping_sub(0x10)
        .wrapping_add(n << 4);
    // call width: 2
    g.call(slot(g, 0xbd, 325), 0, &[p1, last]);
    let bone = g.read::<u32>(obj + 0x924).wrapping_add(0xcc);
    g.write::<u32>(p1 + 0xc, 0x3f80_0000);
    // call width: 2
    g.call(slot(g, 0xde, 325), 0, &[p2, bone]);
    let b = g.read::<u32>(obj + 0x91c).wrapping_add(0x9c);
    // call width: 3
    g.call(slot(g, 0x10f, 231), 0, &[m, w(0x109), b]);
    // call width: 2
    g.call(slot(g, 0x12f, 230), 0, &[m2, m]);
    // call width: 1
    g.call(slot(g, 0x144, 235), 0, &[m3]);
    let s = g.read::<f32>(obj + 0x930);
    g.write::<f32>(at(0x104), s);
    let s1 = mul(s, k(g, 0x162));
    let s2 = mul(s, k(g, 0x173));
    for (o, v) in [
        (0xe8u32, 0x3f00_0000u32),
        (0xec, 0x3e00_0000),
        (0xf0, 0),
        (0xf4, s1.to_bits()),
        (0xf8, 0x3f70_0000),
        (0xfc, 0x3f00_0000),
        (0x100, 0x3f00_0000),
        (0x108, 0),
        (0x10c, 0x3e00_0000),
        (0x110, 0x3f00_0000),
        (0x114, s2.to_bits()),
    ] {
        g.write::<u32>(at(o), v);
    }
    let buf = w(0x17f);
    g.write_bytes(buf, &[0u8; 0x4000]);
    let c = at(0x20);
    g.write::<u32>(c + 0x14, at(0xf8));
    g.write::<u32>(c + 0x18, at(0xe8));
    g.write::<u32>(c + 0x1c, at(0x108));
    g.write::<u32>(c + 0xc, m3);
    g.write::<u32>(c, w(0x23b));
    // junk read: C + 0x10 and C + 0x3c never written; 0 (brief 2026-10-24)
    g.write::<u32>(c + 0x10, 0);
    g.write::<u32>(c + 0x3c, 0);
    // call width: 0
    let r = st0_f32(g.call(slot(g, 0x22a, 307), 0, &[]).st0);
    g.write::<f32>(at(0x14), r);
    let width = add(mul(sub(r, k(g, 0x24f)), k(g, 0x25b)), k(g, 0x263));
    g.write::<f32>(c + 0x20, width);
    for i in 0..3 {
        let x = g.read::<u32>(p2 + 4 * i);
        g.write::<u32>(c + 0x24 + 4 * i, x);
    }
    for i in 0..3 {
        let x = g.read::<u32>(p1 + 4 * i);
        g.write::<u32>(c + 0x30 + 4 * i, x);
    }
    let mesh = Frame {
        at: img.rel32_target(f + 0x2c8),
        esp: z.wrapping_sub(4),
        ecx: c,
        ..*fr
    };
    strip_mesh_0150(g, img, &mesh);
    let r = at(0x60);
    let h8 = g.read::<u16>(w(0x2ce));
    g.write::<u16>(r + 8, h8);
    let v = g.read::<u32>(w(0x2d8));
    g.write::<u32>(r, 0x2404);
    g.write::<u32>(r + 0x38, v.wrapping_add(w(0x2e6)));
    let i = g.read::<u32>(w(0x2f2));
    g.write::<u32>(r + 0x1c, 0);
    g.write::<u32>(r + 0x3c, i.wrapping_add(w(0x300)));
    let x = g.read::<u32>(g.read::<u32>(w(0x30c)));
    g.write::<u32>(r + 4, x);
    g.write::<u32>(r + 0x20, m2);
    g.write::<u32>(r + 0x44, m2);
    let y = g.read::<u32>(w(0x331));
    g.write::<u32>(r + 0x34, y);
    for (o, v) in [(0xcu32, 0u32), (0x10, 0), (0x18, 0x8080_8080), (0x40, 0)] {
        g.write::<u32>(r + o, v);
    }
    for (o, op) in [
        (0x24u32, 0x36bu32),
        (0x28, 0x376),
        (0x30, 0x381),
        (0x2c, 0x38c),
    ] {
        g.write::<u32>(r + o, w(op));
    }
    // junk read: R + 0xa (u16) and R + 0x14 never written; 0 (brief 2026-10-24)
    g.write::<u16>(r + 0xa, 0);
    g.write::<u32>(r + 0x14, 0);
    // call width: 2
    let h = g.call(slot(g, 0x33d, 772), 0, &[obj, 0x54_c55f]);
    let (lo, hi) = (h.eax, h.edx);
    g.write::<u32>(c + 0x1c, w(0x3a5));
    // call width: 2
    let (ok, saved) = flagged(g, (0x39b, 0x3b2), 773, &[lo, hi]);
    g.write::<u32>(at(0x14), saved);
    g.write::<u32>(at(0x18), lo);
    g.write::<u32>(at(0x1c), hi);
    let p = g.read::<u32>(w(0x3d0));
    g.write::<u32>(p, saved);
    if ok.eax != 0 {
        g.write::<u32>(p, 1);
        // call width: 4
        g.call(slot(g, 0x3ec, 819), 0, &[lo, hi, c, r]);
        let p = g.read::<u32>(w(0x401));
        g.write::<u32>(p, 1);
        // call width: 3
        g.call(slot(g, 0x411, 865), 0, &[lo, hi, 0x8000_0000]);
        let p = g.read::<u32>(w(0x420));
        g.write::<u32>(p, saved);
        return Ret::eax(p);
    }
    let b8 = u32::from(g.read::<u8>(r + 8));
    // call width: 1
    let a = g.call(slot(g, 0x448, 776), 0, &[b8]).eax;
    let m2w = g.read::<u32>(r + 0x44);
    // call width: 1
    let bits = a | g.call(slot(g, 0x456, 777), 0, &[m2w]).eax;
    let p = g.read::<u32>(w(0x470));
    let saved = g.read::<u32>(p);
    g.write::<u32>(p, 1);
    // call width: 5
    g.call(slot(g, 0x48b, 820), 0, &[lo, hi, c, r, bits]);
    let p = g.read::<u32>(w(0x4a2));
    g.write::<u32>(p, saved);
    Ret::eax(p)
}

/// [`strip_draw_0150`] (the `rel32`s: three `__security_check_cookie`,
/// the window, `memset`, the packet).
pub const STRIP_DRAW_0150: Sig = Sig {
    len: 1215,
    hash: 0xc30c_65c1_b01c_4a26,
    calls: &[
        (0x87, Callee::Any),
        (0x9e, Callee::Sig(&crate::s4_small::STRIP_WINDOW)),
        (0x1f9, Callee::Import("memset")),
        (0x2c8, Callee::Sig(&STRIP_MESH_0150)),
        (0x439, Callee::Any),
        (0x4b7, Callee::Any),
    ],
    imports: &[],
};

/// The vertices of one ribbon segment of [`ribbon_draw_0150`]: three at
/// `v` around the point `q` (`q.y - K` first, then `+-` the segment's
/// normal `n` (`n = (nx, nz)`, scaled by `K3` into the frame temporaries)
/// times `K1 - q.w`), through slot 325 from the frame's `R` and `S`.
fn ribbon_vertices<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    z: u32,
    (v, q): (u32, u32),
    ops: &[u32; 8],
) {
    use crate::sb_sse::{add, mul, sub};
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let rf = |g: &G, at: u32| g.read::<f32>(at);
    let slot = |g: &G, o: u32| slot_target(g, w(o), 325);
    let at = |o: u32| z.wrapping_add(o);
    let (r, s) = (at(0xe8), at(0xf8));
    let [k_y, t1, t2, t3, k3a, k3b, k1a, k1b] = *ops;
    for o in [0u32, 8, 0xc] {
        let x = g.read::<u32>(q + o);
        g.write::<u32>(r + o, x);
    }
    let y = sub(rf(g, q + 4), k(g, k_y));
    g.write::<f32>(r + 4, y);
    // call width: 2
    g.call(slot(g, t1), 0, &[v, r]);
    let ry = g.read::<u32>(r + 4);
    let back = sub(k(g, k1a), rf(g, q + 0xc));
    g.write::<u32>(s + 4, ry);
    let nz = mul(rf(g, at(0xc)), k(g, k3a));
    g.write::<f32>(at(0xc), nz);
    g.write::<f32>(s, add(mul(nz, back), rf(g, r)));
    let nx = mul(rf(g, at(0x10)), k(g, k3b));
    g.write::<f32>(at(0x10), nx);
    g.write::<f32>(s + 8, add(mul(nx, back), rf(g, r + 8)));
    // call width: 2
    g.call(slot(g, t2), 0, &[v + 0x10, s]);
    let back = sub(k(g, k1b), rf(g, q + 0xc));
    let nz = mul(rf(g, at(0xc)), back);
    g.write::<f32>(s, sub(rf(g, r), nz));
    let nx = mul(rf(g, at(0x10)), back);
    g.write::<f32>(s + 8, sub(rf(g, r + 8), nx));
    // call width: 2
    g.call(slot(g, t3), 0, &[v + 0x20, s]);
}

/// The segment's direction of [`ribbon_draw_0150`]: `C` = the point at
/// `p` (slot 325), `d = C - Q` (x and z), its length (`sqrt` of `dz^2 +
/// dx^2` in `f64`, rounded) dividing each, scaled by `Kz` and `Kx` into
/// the frame temporaries (`nz` at `Z + 0xc`, `nx` at `Z + 0x10`).
fn ribbon_dir<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    z: u32,
    p: u32,
    (t, kx, kz): (u32, u32, u32),
) {
    use crate::sb_sse::{add, div, mul, sub};
    let w = |o: u32| img.word(f + o);
    let rf = |g: &G, at: u32| g.read::<f32>(at);
    let at = |o: u32| z.wrapping_add(o);
    let (c, q) = (at(0x108), at(0xd8));
    // call width: 2
    g.call(slot_target(g, w(t), 325), 0, &[c, p]);
    let dx = sub(rf(g, c), rf(g, q));
    let dz = sub(rf(g, c + 8), rf(g, q + 8));
    #[allow(clippy::cast_possible_truncation)] // cvtsd2ss
    let len = f64::from(add(mul(dz, dz), mul(dx, dx))).sqrt() as f32;
    let nx = mul(div(dx, len), g.read::<f32>(w(kx)));
    let nz = mul(div(dz, len), g.read::<f32>(w(kz)));
    g.write::<f32>(at(0xc), nz);
    g.write::<f32>(at(0x10), nx);
}

/// A ribbon's index record (`magic_0150`'s [`ribbon_draw_0150`]): when
/// neither the saved point's `w` nor `Q`'s passes `TOP`, six words of the
/// two colours and the u16 vertex numbers `k - 3 .. k + 2` at `rec`, the
/// record count `u16 I + 2` two more. Returns the next record.
fn ribbon_record<G: Game>(g: &mut G, img: &Image, f: u32, z: u32, rec: u32, top_op: u32) -> u32 {
    let at = |o: u32| z.wrapping_add(o);
    let top = g.read::<f32>(img.word(f + top_op));
    if !(top >= g.read::<f32>(at(0x124)) && top >= g.read::<f32>(at(0xe4))) {
        return rec;
    }
    let (a, b) = (g.read::<u32>(at(0x1c)), g.read::<u32>(at(0x18)));
    let k = g.read::<u16>(at(0x20));
    for o in [0x20u32, 0x18, 8, 0] {
        g.write::<u32>(rec + o, a);
    }
    for o in [0x24u32, 0x1c, 0xc, 4] {
        g.write::<u32>(rec + o, b);
    }
    for (o, d) in [
        (0x12u32, -2i16),
        (0x16, 1),
        (0x2a, -1),
        (0x2e, 2),
        (0x10, -3),
        (0x14, 0),
        (0x28, -3),
        (0x2c, 0),
    ] {
        g.write::<u16>(rec + o, k.wrapping_add_signed(d));
    }
    let i = g.read::<u32>(at(0x28));
    let n = g.read::<u16>(i + 2).wrapping_add(2);
    g.write::<u16>(i + 2, n);
    rec + 0x30
}

/// The summon strip's ribbon (`magic_0150` 0x10004c90 and `magic_0603`,
/// 3087 bytes; internal, `ecx` = the object `O`, `edx` = the strip; a
/// 16-aligned /GS frame `Z`, the `esp` after the pushes: the strip's
/// window request `C` at `Z + 0x40`, the packet record `R` at `Z + 0x60`).
/// The window ([`strip_window`](crate::s4_small::strip_window)); `P` = the
/// bone point `O[0x924] + 0xcc` (325); `M1`, `M2` = slot 231 `(G_M,
/// O[0x91c] + 0x9c)`, `M3` = 230 `(M1)`; the buffer `G_BUF` cleared
/// (`memset`) and its header (vertices at `V0`, the index records `I` for
/// three vertices per window point); the colours `cvtt(O[0x934] K) << 24
/// | O[0x948]` and `O[0x94c]`. Along the window's two pieces: each
/// segment from `Q` to the next point (its direction,
/// [`ribbon_dir`]) gives three vertices around `Q` ([`ribbon_vertices`])
/// and, from the second on, an index record ([`ribbon_record`]); the
/// records end with a byte 0xff. A first piece of one point loops for
/// ever, as the original does (`jmp $`). Then `R` (0x404, the buffer, the
/// matrices; `O[0x944]` set: `M4` = 230 `(M2)` with `M4[14] -= K`), and
/// unless there are no records: slot 772 `(O, 0x12d644)` gives the
/// handle `h`, slot 773 `(h)` true: 819 `(h, C, R)` and 865 `(h,
/// 0x80000000)`; else 820 `(h, C, R, 776 (0x44) | 777 (M4))` with the
/// record counts 0x80. Each game call after the handle runs with `*G_F`'s
/// word at 1, restored after.
///
/// Returns `*G_F` as read last, or `*G_Y` with no records. `C + 0x10..0x18`,
/// `R + 9..0xb` and `R + 0x14` are never written by the original (junk
/// reads by the packet calls): 0 here (brief 2026-10-24).
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn ribbon_draw_0150<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{cvtt, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let (obj, strip) = (fr.ecx, fr.edx);
    let z = (fr.esp.wrapping_sub(4) & !0xf).wrapping_sub(0x240);
    let at = |o: u32| z.wrapping_add(o);
    let (c, r) = (at(0x40), at(0x60));
    let (p, m1, m2, m3, m4) = (at(0x1a8), at(0x128), at(0x1b8), at(0x1f8), at(0x168));
    g.write::<u32>(at(0x2c), obj);
    g.write::<u32>(c, strip);
    let win = Frame {
        at: img.rel32_target(f + 0x2d),
        esp: z.wrapping_sub(4),
        ecx: c,
        ..*fr
    };
    crate::s4_small::strip_window(g, img, &win);
    let bone = g.read::<u32>(obj + 0x924).wrapping_add(0xcc);
    // call width: 2
    g.call(slot(g, 0x46, 325), 0, &[p, bone]);
    for (dst, gm, t) in [(m1, 0x66u32, 0x6cu32), (m2, 0x8f, 0x95)] {
        let b = g.read::<u32>(obj + 0x91c).wrapping_add(0x9c);
        // call width: 3
        g.call(slot(g, t, 231), 0, &[dst, w(gm), b]);
    }
    // call width: 2
    g.call(slot(g, 0xb5, 230), 0, &[m3, m1]);
    g.write_bytes(w(0xc9), &[0u8; 0x4000]);
    // the header
    let n3 = g.read::<u16>(strip + 0x26).wrapping_mul(3);
    g.write::<u16>(w(0xe2), n3);
    let vbase = w(0xea) & !0xf;
    let span = (i32::from(n3.cast_signed()) << 4).cast_unsigned();
    let idx = vbase.wrapping_add(3).wrapping_add(span) & !3;
    g.write::<u32>(w(0x104), idx.wrapping_sub(w(0xff)));
    g.write::<u32>(w(0x113), vbase.wrapping_sub(w(0x10e)));
    for (op, v) in [(0x119u32, 0u32), (0x123, 0), (0x12d, 0x44_0000), (0x137, 0)] {
        g.write::<u32>(w(op), v);
    }
    g.write::<u16>(w(0x146), 1);
    g.write::<u32>(idx, 0x400);
    g.write::<u32>(idx + 8, 0);
    g.write::<u32>(idx + 0xc, 0);
    g.write::<u32>(at(0x14), vbase);
    g.write::<u32>(at(0x34), vbase);
    g.write::<u32>(at(0x28), idx);
    let mut rec = idx + 0x10;
    g.write::<u32>(at(0x104), 0x3f80_0000);
    let alpha = mul(g.read::<f32>(obj + 0x934), g.read::<f32>(w(0x184)));
    let col_a = (cvtt(alpha).cast_unsigned() << 24) | g.read::<u32>(obj + 0x948);
    g.write::<u32>(at(0x1c), col_a);
    let col_b = g.read::<u32>(obj + 0x94c);
    g.write::<u32>(at(0x18), col_b);
    let n0 = i32::from(g.read::<u16>(at(0x44))) - 1;
    let n1 = u32::from(g.read::<u16>(at(0x46)));
    g.write::<u32>(at(0x3c), n1);
    let first = g.read::<u32>(at(0x48));
    let q = at(0xd8);
    for i in 0..4 {
        let x = g.read::<u32>(first + 4 * i);
        g.write::<u32>(q + 4 * i, x);
    }
    g.write::<i32>(at(0x30), n0);
    g.write::<i32>(at(0x38), n0);
    let mut ptr = first + 0x10;
    g.write::<u32>(at(0x24), ptr);
    g.write::<u32>(at(0x20), 0);
    if n0 <= 0 {
        // `jmp $` in the original
        loop {
            core::hint::spin_loop();
        }
    }
    ribbon_dir(g, img, f, z, ptr, (0x21c, 0x2aa, 0x2a2));
    let v = g.read::<u32>(at(0x14));
    ribbon_vertices(
        g,
        img,
        f,
        z,
        (v, q),
        &[0x2e8, 0x2ee, 0x368, 0x409, 0x340, 0x37f, 0x320, 0x3ab],
    );
    let mut piece = 0u32;
    loop {
        // the step to the next point
        let c_pt = at(0x108);
        // call width: 2
        g.call(slot(g, 0x431, 325), 0, &[at(0x118), q]);
        // call width: 2
        g.call(slot(g, 0x44e, 325), 0, &[q, c_pt]);
        let n = g.read::<i32>(at(0x30)) - 1;
        let v = g.read::<u32>(at(0x14)) + 0x30;
        g.write::<u32>(at(0x14), v);
        let k = g.read::<u32>(at(0x20)) + 3;
        g.write::<u32>(at(0x20), k);
        ptr = g.read::<u32>(at(0x24)) + 0x10;
        g.write::<u32>(at(0x24), ptr);
        g.write::<i32>(at(0x30), n);
        let mut more = n > 0;
        while !more {
            piece += 1;
            if piece >= 2 {
                break;
            }
            let n = g.read::<i32>(at(0x38 + 4 * piece));
            ptr = g.read::<u32>(at(0x4c));
            g.write::<i32>(at(0x30), n);
            g.write::<u32>(at(0x24), ptr);
            more = n > 0;
        }
        if !more {
            break;
        }
        ribbon_dir(g, img, f, z, ptr, (0x4a8, 0x521, 0x529));
        rec = ribbon_record(g, img, f, z, rec, 0x531);
        let v = g.read::<u32>(at(0x14));
        ribbon_vertices(
            g,
            img,
            f,
            z,
            (v, q),
            &[0x5f9, 0x606, 0x674, 0x6e8, 0x64c, 0x68b, 0x62c, 0x6b7],
        );
    }
    // the last point
    ribbon_dir(g, img, f, z, ptr, (0x733, 0x7ac, 0x7b4));
    rec = ribbon_record(g, img, f, z, rec, 0x7bc);
    let v = g.read::<u32>(at(0x14));
    ribbon_vertices(
        g,
        img,
        f,
        z,
        (v, q),
        &[0x885, 0x892, 0x8f9, 0x969, 0x8d5, 0x910, 0x8b5, 0x93c],
    );
    let v0 = g.read::<u32>(at(0x34));
    g.write::<u8>(rec + 1, 0xff);
    // the record `R`
    g.write::<u32>(r + 0x38, v0);
    g.write::<u32>(r, 0x404);
    g.write::<u8>(r + 8, 0x44);
    g.write::<u32>(r + 0x1c, 0);
    g.write::<u32>(r + 0x3c, 0);
    let x = g.read::<u32>(g.read::<u32>(w(0x9b6)));
    g.write::<u32>(r + 4, x);
    for (o, v) in [
        (0xcu32, 0u32),
        (0x10, 0),
        (0x18, 0x8080_8080),
        (0x40, 0),
        (0x20, m3),
    ] {
        g.write::<u32>(r + o, v);
    }
    // junk read: R + 9..0xb, R + 0x14, C + 0x10..0x18 never written; 0 (brief 2026-10-24)
    g.write_bytes(r + 9, &[0u8; 3]);
    g.write::<u32>(r + 0x14, 0);
    for o in [0x10u32, 0x14, 0x18] {
        g.write::<u32>(c + o, 0);
    }
    if g.read::<u32>(obj + 0x944) != 0 {
        // call width: 2
        g.call(slot(g, 0xa34, 230), 0, &[m4, m2]);
        let y = sub(g.read::<f32>(m4 + 0x38), g.read::<f32>(w(0xa4d)));
        g.write::<u32>(r + 0x44, m4);
        g.write::<f32>(m4 + 0x38, y);
    } else {
        g.write::<u32>(r + 0x44, 0);
    }
    let idx = g.read::<u32>(at(0x28));
    for (o, v) in [(0x24u32, m1), (0x28, m1), (0x30, m2), (0x2c, m1)] {
        g.write::<u32>(r + o, v);
    }
    let gy = g.read::<u32>(w(0xaae));
    g.write::<u32>(r + 0x34, gy);
    if g.read::<i16>(idx + 2) <= 0 {
        return Ret::eax(gy);
    }
    // call width: 2
    let h = g.call(slot(g, 0xac5, 772), 0, &[obj, 0x12_d644]);
    let (lo, hi) = (h.eax, h.edx);
    let pf = g.read::<u32>(w(0xad9));
    g.write::<u32>(at(0x1c), hi);
    g.write::<u32>(c + 0x1c, w(0xaef));
    let saved = g.read::<u32>(pf);
    g.write::<u32>(pf, 1);
    g.write::<u32>(at(0x2c), lo);
    g.write::<u32>(at(0x18), saved);
    // call width: 2
    let ok = g.call(slot(g, 0xae7, 773), 0, &[lo, hi]).eax;
    let pf = g.read::<u32>(w(0xb0e));
    g.write::<u32>(pf, saved);
    if ok != 0 {
        g.write::<u32>(pf, 1);
        // call width: 4
        g.call(slot(g, 0xb2e, 819), 0, &[lo, hi, c, r]);
        let pf = g.read::<u32>(w(0xb43));
        g.write::<u32>(pf, 1);
        // call width: 3
        g.call(slot(g, 0xb53, 865), 0, &[lo, hi, 0x8000_0000]);
        let pf = g.read::<u32>(w(0xb62));
        g.write::<u32>(pf, saved);
        return Ret::eax(pf);
    }
    let b = u32::from(g.read::<u8>(r + 8));
    // call width: 1
    let a = g.call(slot(g, 0xb8a, 776), 0, &[b]).eax;
    let m = g.read::<u32>(r + 0x44);
    // call width: 1
    let bits = a | g.call(slot(g, 0xb98, 777), 0, &[m]).eax;
    g.write::<u16>(idx + 2, 0x80);
    g.write::<u16>(w(0xbbd), 0x80);
    let pf = g.read::<u32>(w(0xbc2));
    let saved = g.read::<u32>(pf);
    g.write::<u32>(pf, 1);
    // call width: 5
    g.call(slot(g, 0xbe1, 820), 0, &[lo, hi, c, r, bits]);
    let pf = g.read::<u32>(w(0xbf2));
    g.write::<u32>(pf, saved);
    Ret::eax(pf)
}

/// [`ribbon_draw_0150`] (the `rel32`s: the window, `memset`, three
/// `_libm_sse2_sqrt_precise`, two `__security_check_cookie`; the loop's
/// jump stays inside).
pub const RIBBON_DRAW_0150: Sig = Sig {
    len: 3087,
    hash: 0xb876_f4e5_9460_eeb8,
    calls: &[
        (0x2d, Callee::Sig(&crate::s4_small::STRIP_WINDOW)),
        (0xce, Callee::Import("memset")),
        (0x272, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x501, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x78c, Callee::Import("_libm_sse2_sqrt_precise")),
        (0xb7b, Callee::Any),
        (0xc07, Callee::Any),
    ],
    imports: &[],
};

/// The summon strip's frame (`magic_0150` 0x100041e0 and `magic_0603`,
/// 2451 bytes; internal, `ecx` = the handle `H` (`+0` its node `N`, byte
/// `+5` the pass), reached through its `ecx` thunk; a 16-aligned /GS frame
/// `Z`, the `esp` after the pushes).
/// - The walk over `N`'s children (their callbacks get `ecx` = `N`); a
///   pass already seen stops (`eax` junk).
/// - The bone `N[0x91c]` stepped ([`bone_plain`](crate::sa_summon::bone_plain)),
///   the strip `T = N[0x920]` ([`strip_task_0150`]), the bone `N[0x924]`;
///   the window's last point `P` (325), the bone point, `M` = 231 `(G_M,
///   N[0x91c] + 0x9c)`.
/// - `T` hit (`T[0xf8]`): a mark at the hit: the matrix `M5` turned by two
///   slot 324 numbers (the rotation behind the pointer `*G_ROT`, run as
///   the overlay runs it), times `M`'s, its rows scaled by 1e-4, `P` its
///   row 3, `M5 = 231 (G_M, M5)`; the packet record `R` (`Z + 0x80`) for
///   `N[0x92c]`'s model, alpha `cvtt(N[0x930] K)`; slot 772 `(N,
///   0x5d40043)` gives `h`, then 773 `(h)` true: 819 `(h, C, R)` and 775
///   `(h)`; else 820 `(h, C, R, 776 | 777)` (each with `*G_F`'s word 1).
/// - A window of two or more: the ribbon ([`ribbon_draw_0150`]).
/// - `T` hit: unless `*G_C`, the spark timer `N[0x938]` counts down; at 0,
///   `(324 & 3) + 1` sparks into the ring of 64 (`N[0x939]`): the point
///   `P`, a speed (with `N[0x944]`: 233 of `T + 0xcc` and three slot 307
///   jitters; else three slot 307 terms), `w` the size. Then up to 64 live
///   sparks (more than `*G_MAX` raises it) are drawn: alpha `cvtt(w K)`;
///   not 0: `M8` = the scale `(K1 - w) K2 + K3` on its diagonal and row 3
///   = 229 `(M, point)`, `M7` = 230 `(M8)`, the stream block
///   ([`stream_block`](crate::s7_misc::stream_block)); unless `*G_C` the
///   point moves by the speed, the speed's `y` scales by `K4` (with
///   `N[0x944]`: `z` by `K4'` and `x` = 0.72) and `w` by `K5`. `*G_Y` is
///   put back.
///
/// Returns `eax` as the original leaves it (the last speed pointer, the
/// new `*G_MAX`, the ribbon's or slot 231's).
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn strip_frame_0150<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, cvtt, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let rf = |g: &G, at: u32| g.read::<f32>(at);
    let h = fr.ecx;
    let z = (fr.esp.wrapping_sub(4) & !0xf).wrapping_sub(0x230);
    let at = |o: u32| z.wrapping_add(o);
    let n0 = g.read::<u32>(h);
    g.write::<u32>(at(0x18), n0);
    let (eax, moved) = crate::nodes::walk(g, h, n0, fr.eax);
    if moved.is_none() {
        // the cookie's upper bytes are not the port's (`eax` junk)
        return Ret::eax(eax);
    }
    let n = g.read::<u32>(h);
    // the two bones and the strip, each with the cursor at `Z + 0x1c`
    let cur = at(0x1c);
    let step = |g: &mut G, o: u32, call: u32, bone: bool| {
        let x = g.read::<u32>(n + o);
        g.write::<u32>(cur, x);
        if o == 0x91c {
            let s = g.read::<u32>(n + 0x930);
            g.write::<u32>(at(0x2c), s);
        }
        let pass = g.read::<u8>(h + 5);
        g.write::<u8>(cur + 5, pass);
        let fr = Frame {
            at: img.rel32_target(f + call),
            esp: z.wrapping_sub(4),
            ecx: cur,
            eax: u32::from(pass),
            ..*fr
        };
        if bone {
            crate::sa_summon::bone_plain(g, img, &fr);
        } else {
            strip_task_0150(g, img, &fr);
        }
    };
    step(g, 0x91c, 0x97, true);
    step(g, 0x920, 0xb2, false);
    step(g, 0x924, 0xcd, true);
    let n = g.read::<u32>(h);
    let t = g.read::<u32>(n + 0x920);
    g.write::<u32>(at(8), n);
    g.write::<u32>(at(0x10), t);
    g.write::<u32>(cur, t);
    let win = Frame {
        at: img.rel32_target(f + 0xea),
        esp: z.wrapping_sub(4),
        ecx: cur,
        ..*fr
    };
    crate::s4_small::strip_window(g, img, &win);
    let (p, p2, m, m5) = (at(0x138), at(0x198), at(0x1e8), at(0xf8));
    let c2 = u32::from(g.read::<u16>(at(0x22)));
    let last = g
        .read::<u32>(at(0x28))
        .wrapping_sub(0x10)
        .wrapping_add(c2 << 4);
    // call width: 2
    g.call(slot(g, 0x109, 325), 0, &[p, last]);
    g.write::<u32>(p + 0xc, 0x3f80_0000);
    let b1 = g.read::<u32>(n + 0x924).wrapping_add(0xcc);
    // call width: 2
    g.call(slot(g, 0x135, 325), 0, &[p2, b1]);
    let b0 = g.read::<u32>(n + 0x91c).wrapping_add(0x9c);
    // call width: 3
    let mut eax = g.call(slot(g, 0x15b, 231), 0, &[m, w(0x155), b0]).eax;
    let flag = |g: &mut G, op: u32| {
        let pf = g.read::<u32>(w(op));
        g.write::<u32>(pf, 1);
    };
    if g.read::<u32>(t + 0xf8) != 0 {
        // the mark at the hit
        // call width: 0
        let r1 = g.call(slot(g, 0x178, 324), 0, &[]).eax;
        g.write::<u32>(at(0x188), r1);
        // call width: 0
        let r2 = g.call(slot(g, 0x18c, 324), 0, &[]).eax;
        g.write::<u32>(at(0x18c), r2);
        // junk read: the third angle `Z + 0x190` never written; 0 (brief 2026-10-24)
        g.write::<u32>(at(0x190), 0);
        let rot = g.read::<u32>(w(0x1b1));
        crate::l5_objhost::call_code(g, img, rot, z.wrapping_sub(0xc), &[m5, at(0x188)]);
        let b0 = g.read::<u32>(n + 0x91c).wrapping_add(0x9c);
        // call width: 3
        g.call(slot(g, 0x1cb, 231), 0, &[m5, m5, b0]);
        for (row, op) in [(0u32, 0x1ebu32), (0x10, 0x20b), (0x20, 0x22b)] {
            // call width: 3
            g.call(slot(g, op, 233), 0, &[m5 + row, m5 + row, 0x39d1_b717]);
        }
        // call width: 2
        g.call(slot(g, 0x24b, 325), 0, &[m5 + 0x30, p]);
        // call width: 3
        g.call(slot(g, 0x266, 231), 0, &[m5, w(0x260), m5]);
        let model = g.read::<u32>(n + 0x92c);
        let (r, c) = (at(0x80), at(0x30));
        let b = i32::from(g.read::<i8>(model));
        let mut bits = ((b & 1) << 7).cast_unsigned();
        g.write::<u32>(r, bits);
        if g.read::<u8>(model) & 2 == 0 {
            bits |= 0x10;
            g.write::<u32>(r, bits);
        }
        let h16 = g.read::<u16>(model + 0x16);
        let alpha = mul(rf(g, at(0x2c)), k(g, 0x2a8));
        g.write::<u16>(r + 8, h16);
        g.write::<u32>(r + 0x1c, 0);
        let e8 = model.wrapping_add(g.read::<u32>(model + 8));
        g.write::<u32>(r + 0x38, e8);
        let ec = model.wrapping_add(g.read::<u32>(model + 0xc));
        g.write::<u32>(r + 0x3c, ec);
        let x = g.read::<u32>(g.read::<u32>(w(0x2d8)));
        g.write::<u32>(r + 4, x);
        g.write::<u32>(r + 0x18, 0x8080_8080);
        g.write::<u8>(r + 0x1b, cvtt(alpha).cast_unsigned().to_le_bytes()[0]);
        for o in [0x20u32, 0x44, 0x24, 0x28, 0x2c] {
            g.write::<u32>(r + o, m5);
        }
        let gy = g.read::<u32>(w(0x32b));
        g.write::<u32>(r + 0x34, gy);
        for o in [0xcu32, 0x10, 0x40] {
            g.write::<u32>(r + o, 0);
        }
        g.write::<u32>(c + 0x1c, model);
        // junk read: R + 0xa (u16), R + 0x14, R + 0x30 and the request's
        // words before `+0x1c` never written; 0 (brief 2026-10-24)
        g.write::<u16>(r + 0xa, 0);
        for o in [0x14u32, 0x30] {
            g.write::<u32>(r + o, 0);
        }
        for o in (0..0x1c).step_by(4) {
            g.write::<u32>(c + o, 0);
        }
        // call width: 2
        let hd = g.call(slot(g, 0x337, 772), 0, &[n, 0x5d4_0043]);
        let (lo, hi) = (hd.eax, hd.edx);
        let pf = g.read::<u32>(w(0x36b));
        let saved = g.read::<u32>(pf);
        g.write::<u32>(pf, 1);
        g.write::<u32>(at(0xc), hi);
        g.write::<u32>(at(0x18), lo);
        g.write::<u32>(at(0x14), saved);
        // call width: 2
        let ok = g.call(slot(g, 0x37d, 773), 0, &[lo, hi]).eax;
        let pf = g.read::<u32>(w(0x399));
        g.write::<u32>(pf, saved);
        if ok != 0 {
            g.write::<u32>(pf, 1);
            // call width: 4
            g.call(slot(g, 0x3b8, 819), 0, &[lo, hi, c, r]);
            flag(g, 0x3d0);
            // call width: 2
            g.call(slot(g, 0x3df, 775), 0, &[lo, hi]);
            let pf = g.read::<u32>(w(0x3ed));
            g.write::<u32>(pf, saved);
            eax = pf;
        } else {
            let b = u32::from(g.read::<u8>(r + 8));
            // call width: 1
            let a = g.call(slot(g, 0x406, 776), 0, &[b]).eax;
            let m = g.read::<u32>(r + 0x44);
            // call width: 1
            let bits = a | g.call(slot(g, 0x414, 777), 0, &[m]).eax;
            let pf = g.read::<u32>(w(0x42e));
            let saved = g.read::<u32>(pf);
            g.write::<u32>(pf, 1);
            // call width: 5
            g.call(slot(g, 0x44c, 820), 0, &[lo, hi, c, r, bits]);
            let pf = g.read::<u32>(w(0x463));
            g.write::<u32>(pf, saved);
            eax = pf;
        }
    }
    let n = g.read::<u32>(at(8));
    let t = g.read::<u32>(at(0x10));
    let strip = g.read::<u32>(n + 0x920);
    if g.read::<u16>(strip + 0x26) >= 2 {
        let ribbon = Frame {
            at: img.rel32_target(f + 0x484),
            esp: z.wrapping_sub(4),
            ecx: n,
            edx: strip,
            ..*fr
        };
        eax = ribbon_draw_0150(g, img, &ribbon).eax;
    }
    if g.read::<u32>(t + 0xf8) == 0 {
        return Ret::eax(eax);
    }
    // the sparks
    let busy = g.read::<u32>(w(0x497)) != 0;
    g.write::<u32>(at(0xc), n + 0x41c);
    let ring = g.read::<u32>(n + 0x928);
    g.write::<u32>(at(0x1c), ring);
    g.write::<u32>(at(0x14), n + 0x81c);
    if !busy {
        let timer = g.read::<u8>(n + 0x938).wrapping_sub(1);
        g.write::<u8>(n + 0x938, timer);
    }
    if g.read::<i8>(n + 0x938) <= 0 {
        // call width: 0
        let count = (g.call(slot(g, 0x4d0, 324), 0, &[]).eax & 3) + 1;
        g.write::<u32>(at(0x18), count);
        let mut left = count;
        while left > 0 {
            let i = u32::from(g.read::<u8>(n + 0x939) & 0x3f);
            g.write::<u32>(n + 0x81c + 4 * i, 0);
            // call width: 2
            g.call(slot(g, 0x515, 325), 0, &[n + 0x1c + (i << 4), p]);
            let v = n + 0x41c + (i << 4);
            let rnd = |g: &mut G, op: u32| {
                // call width: 0
                let r = st0_f32(g.call(slot(g, op, 307), 0, &[]).st0);
                g.write::<f32>(at(8), r);
                r
            };
            if g.read::<u32>(n + 0x944) != 0 {
                let r = rnd(g, 0x522);
                let x = add(mul(sub(r, k(g, 0x54c)), k(g, 0x564)), k(g, 0x56e));
                let x = x.to_bits() ^ g.read::<u32>(w(0x575));
                let tc = g.read::<u32>(at(0x10)).wrapping_add(0xcc);
                // call width: 3
                g.call(slot(g, 0x580, 233), 0, &[v, tc, x]);
                for (o, (t1, k1, k2)) in [
                    (0u32, (0x58eu32, 0x5abu32, 0x5b8u32)),
                    (4, (0x5b0, 0x5da, 0x5e7)),
                    (8, (0x5df, 0x60b, 0x613)),
                ] {
                    let r = rnd(g, t1);
                    let d = mul(sub(r, k(g, k1)), k(g, k2));
                    g.write::<f32>(v + o, add(d, rf(g, v + o)));
                }
            } else {
                let r = rnd(g, 0x522);
                let x = mul(
                    sub(k(g, 0x646), mul(sub(r, k(g, 0x63e)), k(g, 0x653))),
                    k(g, 0x65f),
                );
                g.write::<f32>(v, x);
                let r = rnd(g, 0x64b);
                let z2 = mul(
                    sub(k(g, 0x685), mul(sub(r, k(g, 0x67d)), k(g, 0x692))),
                    k(g, 0x69e),
                );
                g.write::<f32>(v + 8, z2);
                let r = rnd(g, 0x68a);
                let y = add(mul(sub(r, k(g, 0x6bd)), k(g, 0x6c5)), k(g, 0x6cd));
                g.write::<u32>(v + 4, y.to_bits() ^ g.read::<u32>(w(0x6d4)));
            }
            let s = g.read::<u32>(at(0x2c));
            g.write::<u32>(v + 0xc, s);
            let next = g.read::<u8>(n + 0x939).wrapping_add(1);
            g.write::<u8>(n + 0x939, next);
            left -= 1;
            g.write::<u32>(at(0x18), left);
            g.write::<u8>(n + 0x938, 1);
        }
    }
    // the draws
    let b0 = g.read::<u32>(n + 0x91c).wrapping_add(0x9c);
    // call width: 3
    g.call(slot(g, 0x71f, 231), 0, &[m, w(0x719), b0]);
    let (m7, m8, r2) = (at(0x1a8), at(0x148), at(0x50));
    let live = i32::from(g.read::<u8>(n + 0x939)).min(0x40);
    let gy = g.read::<u32>(w(0x734));
    g.write::<u32>(at(0x60), m7);
    g.write::<u32>(at(0x54 + 0x18), m7);
    g.write::<u16>(r2 + 2, 0x80);
    let most = g.read::<i32>(w(0x75f)).max(live);
    for o in [
        0x14cu32, 0x150, 0x154, 0x158, 0x160, 0x164, 0x168, 0x16c, 0x174,
    ] {
        g.write::<u32>(at(o), 0);
    }
    g.write::<u32>(r2 + 8, 0);
    g.write::<u32>(r2 + 4, 0x8080_8080);
    g.write::<u8>(r2 + 0x18, 0);
    g.write::<u32>(r2 + 0x14, gy);
    g.write::<i32>(at(0x10), live);
    g.write::<i32>(w(0x7e9), most);
    if live <= 0 {
        let gy = g.read::<u32>(at(0x64));
        g.write::<u32>(w(0x975), gy);
        return Ret::eax(most.cast_unsigned());
    }
    let mut sp = n + 0x24;
    let mut cursor = n + 0x81c;
    let mut v = g.read::<u32>(at(0xc));
    let mut left = live;
    while left > 0 {
        let wv = rf(g, sp + 0x404);
        let a = cvtt(mul(wv, k(g, 0x810))).cast_unsigned().to_le_bytes()[0];
        g.write::<u8>(r2 + 7, a);
        if a != 0 {
            let sc = add(mul(sub(k(g, 0x828), wv), k(g, 0x840)), k(g, 0x850));
            g.write::<f32>(m8 + 0x28, sc);
            g.write::<f32>(m8 + 0x14, sc);
            g.write::<f32>(m8, sc);
            // call width: 3
            g.call(slot(g, 0x855, 229), 0, &[m8 + 0x30, m, sp - 8]);
            // call width: 2
            g.call(slot(g, 0x890, 230), 0, &[m7, m8]);
            let ring = g.read::<u32>(at(0x1c));
            let cur = g.read::<u32>(at(0x14));
            g.write::<u32>(z.wrapping_sub(0xc), r2);
            let blk = Frame {
                at: img.rel32_target(f + 0x8aa),
                esp: z.wrapping_sub(0x10),
                ecx: cur,
                edx: ring,
                ..*fr
            };
            crate::s7_misc::stream_block(g, img, &blk);
            v = g.read::<u32>(at(0xc));
            if g.read::<u32>(w(0x8b7)) == 0 {
                for (o, d) in [(0u32, v), (4, sp + 0x3fc), (8, sp + 0x400)] {
                    let x = add(rf(g, sp - 8 + o), rf(g, d));
                    g.write::<f32>(sp - 8 + o, x);
                }
                let y = mul(rf(g, sp + 0x3fc), k(g, 0x8fe));
                g.write::<f32>(sp + 0x3fc, y);
                if g.read::<u32>(n + 0x944) != 0 {
                    let z3 = mul(rf(g, sp + 0x400), k(g, 0x91f));
                    g.write::<u32>(v, 0x3f38_f5c3);
                    g.write::<f32>(sp + 0x400, z3);
                }
                let wv = mul(rf(g, sp + 0x404), k(g, 0x93d));
                g.write::<f32>(sp + 0x404, wv);
            }
        } else {
            v = g.read::<u32>(at(0xc));
        }
        cursor = cursor.wrapping_add(4);
        v = v.wrapping_add(0x10);
        sp = sp.wrapping_add(0x10);
        left -= 1;
        g.write::<u32>(at(0x14), cursor);
        g.write::<u32>(at(0xc), v);
        g.write::<i32>(at(0x10), left);
    }
    let gy = g.read::<u32>(at(0x64));
    g.write::<u32>(w(0x975), gy);
    Ret::eax(v)
}

/// [`strip_frame_0150`] (the `rel32`s: two bones, the strip task, the
/// window, the ribbon, the stream block, `__security_check_cookie`; the
/// loop's jump stays inside).
pub const STRIP_FRAME_0150: Sig = Sig {
    len: 2451,
    hash: 0xce83_1859_4466_dae2,
    calls: &[
        (0x97, Callee::Sig(&crate::sa_summon::BONE)),
        (0xb2, Callee::Sig(&STRIP_TASK_0150)),
        (0xcd, Callee::Sig(&crate::sa_summon::BONE)),
        (0xea, Callee::Sig(&crate::s4_small::STRIP_WINDOW)),
        (0x484, Callee::Sig(&RIBBON_DRAW_0150)),
        (0x8aa, Callee::Sig(&crate::s7_misc::STREAM_BLOCK)),
        (0x985, Callee::Any),
    ],
    imports: &[],
};

/// `fistp qword` of an `f32` with the rounding set toward zero (the
/// original's `fnstcw`/`fldcw` pairs): the low word of the 64-bit
/// truncation, 0 for a NaN or a value out of range.
fn fistp_trunc(x: f32) -> u32 {
    ffx_bridge::x87::ftol2(f64::from(x))
}

/// The summon strip's tail draw (`magic_0150` 0x100058a0 and `magic_0603`,
/// 2415 bytes; a draw callback, cdecl `(obj)`; a 16-aligned /GS frame `Z`,
/// the `esp` after the pushes: the request `C` at `Z + 0xb0` (its window
/// cursor at `C + 0x20`), the record `R` at `Z + 0xe0`). Every game call
/// after the handle runs with `*G_F`'s word 1, restored after.
/// - `obj[0x930]` at most 0 (not NaN): slot 311 `(obj[0x940])`, slot 772
///   `(obj, 0x7dea)`, slot 779 `(h)`.
/// - Else the window ([`strip_window`](crate::s4_small::strip_window)) of
///   `n` points (at most 16; fewer than 2: nothing more); `P1` = the bone
///   point (325); `M1` = 230 `(B)`, `M2` = 231 `(G_M, B)`, `M3` = 230
///   `(M2)` (`B = obj[0x91c] + 0x9c`); `G_BUF2` cleared (`memset`) with
///   its header (vertices `V`, the index block `I`: u16 `+0` 0x400, `+2`
///   `n - 1`); `V[0] = P1`; then the window's points newest first (the
///   second piece's last point down), each a vertex (`w` 1.0) and, after
///   the first, a 24-byte index record: the two colour words before and
///   after, the u16 numbers `(0, k, 0, k + 1)`. The colours run from the
///   frame's constant terms by `t` (the points left, as a float, less
///   `K1` each step) through `fistp` toward zero; the first pair uses
///   `t * 0`. The records end with 0xff; `G_BUF2[0x12]` = the vertex count.
/// - `R` (0x2404, the buffer, the matrices); slot 772 `(obj, 0x7dea)`
///   gives `h`; 773 `(h)` true: 819 `(h, C, R)` and 865 `(h,
///   0x80000000)`; else `I[2]` 0xf, `G_BUF2[0x12]` 0x11 and 820 `(h, C, R,
///   776 (0x48) | 777 (M3))`.
///
/// Returns `*G_F` as read last, or the window's `eax` for fewer than two
/// points. `C + 0..0x18`, `R + 9..0xb` and `R + 0x14` are never written by
/// the original (junk reads by the packet calls): 0 here (brief
/// 2026-10-24).
#[allow(clippy::too_many_lines, clippy::cast_possible_truncation)] // the original's steps; u16 counts
pub fn tail_draw_0150<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, cvt, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let rf = |g: &G, at: u32| g.read::<f32>(at);
    let obj = fr.arg(g, 0);
    let z = (fr.esp.wrapping_sub(4) & !0xf).wrapping_sub(0x230);
    let at = |o: u32| z.wrapping_add(o);
    let s = rf(g, obj + 0x930);
    g.write::<u32>(at(0x44), obj);
    g.write::<f32>(at(0x4c), s);
    if s <= 0.0 {
        let x = g.read::<u32>(obj + 0x940);
        // call width: 1
        g.call(slot(g, 0x3a, 311), 0, &[x]);
        // call width: 2
        let h = g.call(slot(g, 0x4d, 772), 0, &[obj, 0x7dea]);
        let pf = g.read::<u32>(w(0x64));
        let saved = g.read::<u32>(pf);
        g.write::<u32>(pf, 1);
        // call width: 2
        g.call(slot(g, 0x6d, 779), 0, &[h.eax, h.edx]);
        let pf = g.read::<u32>(w(0x80));
        g.write::<u32>(pf, saved);
        return Ret::eax(pf);
    }
    let (c, r) = (at(0xb0), at(0xe0));
    let cur = c + 0x20;
    let strip = g.read::<u32>(obj + 0x920);
    g.write::<u32>(cur, strip);
    let win = Frame {
        at: img.rel32_target(f + 0xb2),
        esp: z.wrapping_sub(4),
        ecx: cur,
        ..*fr
    };
    let wax = crate::s4_small::strip_window(g, img, &win).eax;
    let mut n = u32::from(g.read::<u16>(strip + 0x26));
    g.write::<u32>(at(0x38), n);
    if n >= 0x10 {
        n = 0x10;
        g.write::<u32>(at(0x38), n);
    } else if n < 2 {
        return Ret::eax(wax);
    }
    let (p1, m1, m2, m3) = (at(0x158), at(0x168), at(0x1e8), at(0x1a8));
    let bone = g.read::<u32>(obj + 0x924).wrapping_add(0xcc);
    // call width: 2
    g.call(slot(g, 0xec, 325), 0, &[p1, bone]);
    let b = g.read::<u32>(obj + 0x91c).wrapping_add(0x9c);
    // call width: 2
    g.call(slot(g, 0x10d, 230), 0, &[m1, b]);
    let b = g.read::<u32>(obj + 0x91c).wrapping_add(0x9c);
    // call width: 3
    g.call(slot(g, 0x133, 231), 0, &[m2, w(0x12d), b]);
    // call width: 2
    g.call(slot(g, 0x153, 230), 0, &[m3, m2]);
    let buf = w(0x167);
    g.write_bytes(buf, &[0u8; 0x4000]);
    // the header
    let v0 = w(0x171) & !0xf;
    let idx = v0.wrapping_add(0x113) & !3;
    let tw = cvt(n);
    let mut rec = idx + 0x10;
    g.write::<u32>(at(0x28), rec);
    g.write::<u32>(w(0x197), idx.wrapping_sub(w(0x192)));
    g.write::<u32>(w(0x1a6), v0.wrapping_sub(w(0x1a1)));
    for (op, v) in [
        (0x1acu32, 0x11_0001u32),
        (0x1b6, 0),
        (0x1c0, 0),
        (0x1ca, 0x48_0000),
        (0x1d4, 0),
    ] {
        g.write::<u32>(w(op), v);
    }
    g.write::<u32>(at(0x40), v0);
    g.write::<u32>(at(0x48), v0);
    g.write::<u32>(at(0x9c), idx);
    g.write::<u16>(idx, 0x400);
    g.write::<u16>(idx + 2, (n - 1) as u16);
    g.write::<u32>(idx + 8, 0);
    g.write::<u32>(idx + 0xc, 0);
    g.write::<f32>(at(0x3c), tw);
    // the colour terms (`(a, b)` of each byte: `a t + b`; the frame's)
    for (o, v) in [
        (0x64u32, 0u32),
        (0x68, 0),
        (0x6c, 0x437f_0000),
        (0x70, 0),
        (0x8c, 0),
        (0x90, 0),
        (0x94, 0),
        (0x98, 0),
        (0x78, 0),
        (0x7c, 0),
        (0x80, 0x437f_0000),
        (0x84, 0),
        (0x50, 0),
        (0x54, 0),
        (0x58, 0),
    ] {
        g.write::<u32>(at(o), v);
    }
    let d = div(k(g, 0x299), tw);
    g.write::<f32>(at(0x5c), d);
    // call width: 2
    g.call(slot(g, 0x2aa, 325), 0, &[v0, p1]);
    // the newest piece with points
    let count = |g: &G, piece: u32| u32::from(g.read::<u16>(at(0xd4 + 2 * piece)));
    let last = |g: &G, piece: u32, cnt: u32| {
        (cnt << 4)
            .wrapping_sub(0x10)
            .wrapping_add(g.read::<u32>(at(0xd8 + 4 * piece)))
    };
    let mut piece = 1i32;
    let mut k_out = 0u32;
    let mut start = None;
    while piece >= 0 {
        let cnt = count(g, piece.cast_unsigned());
        if cnt > 0 {
            start = Some(cnt);
            break;
        }
        piece -= 1;
    }
    if let Some(mut left) = start {
        let fz = |g: &G, o: u32| rf(g, at(o));
        // the first colour pair, from `t * 0`
        let p = mul(fz(g, 0x3c), 0.0);
        let e = fistp_trunc(add(p, k(g, 0x306)));
        let cc = fistp_trunc(add(p, 0.0));
        let ps = add(mul(p, s), 0.0);
        let dd = add(mul(mul(fz(g, 0x5c), fz(g, 0x3c)), fz(g, 0x4c)), 0.0);
        let col1 = (((fistp_trunc(ps) << 8 | e) << 8 | cc) << 8) | cc;
        g.write::<u32>(at(0x20), col1);
        let col2 = (((fistp_trunc(dd) << 8 | e) << 8 | cc) << 8) | cc;
        let mut pt = last(g, piece.cast_unsigned(), left);
        g.write::<u32>(at(0x24), col2);
        g.write::<u32>(at(0x2c), pt);
        let mut kk = 0u32;
        loop {
            // the next vertex
            let v = g.read::<u32>(at(0x40)) + 0x10;
            g.write::<u32>(at(0x40), v);
            kk += 1;
            g.write::<u32>(at(0x14), kk);
            // call width: 2
            g.call(slot(g, 0x422, 325), 0, &[v, pt]);
            let t = sub(fz(g, 0x3c), k(g, 0x43d));
            let remaining = g.read::<i32>(at(0x38)) - 1;
            let p = g.read::<u32>(at(0x2c)).wrapping_sub(0x10);
            g.write::<u32>(at(0x2c), p);
            left -= 1;
            g.write::<u32>(v + 0xc, 0x3f80_0000);
            g.write::<f32>(at(0x3c), t);
            g.write::<i32>(at(0x38), remaining);
            // the next piece when this one runs out
            let mut go = left.cast_signed() > 0 && remaining > 0;
            while !go {
                piece -= 1;
                if piece < 0 {
                    break;
                }
                left = count(g, piece.cast_unsigned());
                let pl = last(g, piece.cast_unsigned(), left);
                g.write::<u32>(at(0x2c), pl);
                go = left.cast_signed() > 0 && remaining > 0;
            }
            if !go {
                k_out = kk;
                break;
            }
            // the index record and the new colours
            let rec_at = g.read::<u32>(at(0x28));
            let col1 = g.read::<u32>(at(0x20));
            g.write::<u32>(rec_at, col1);
            let col2 = g.read::<u32>(at(0x24));
            g.write::<u32>(rec_at + 4, col2);
            let lin = |g: &G, a: u32, b: u32| add(mul(fz(g, a), t), fz(g, b));
            let v1 = add(mul(mul(fz(g, 0x98), t), s), fz(g, 0x70));
            g.write::<f32>(at(0x74), v1);
            let v2 = lin(g, 0x94, 0x6c);
            g.write::<f32>(at(0xa4), v2);
            let v3 = lin(g, 0x90, 0x68);
            g.write::<f32>(at(0x88), v3);
            let v4 = lin(g, 0x8c, 0x64);
            g.write::<f32>(at(0xac), v4);
            let v5 = add(mul(mul(fz(g, 0x5c), t), s), fz(g, 0x84));
            g.write::<f32>(at(0xa0), v5);
            let v6 = lin(g, 0x58, 0x80);
            let c1 = (((fistp_trunc(v1) << 8 | fistp_trunc(v2)) << 8 | fistp_trunc(v3)) << 8)
                | fistp_trunc(v4);
            g.write::<u32>(at(0x20), c1);
            g.write::<f32>(at(0xa8), v6);
            let v7 = lin(g, 0x54, 0x7c);
            g.write::<u32>(rec_at + 8, c1);
            g.write::<f32>(at(0x60), v7);
            let v8 = lin(g, 0x50, 0x78);
            g.write::<f32>(at(0x30), v8);
            let c2 = (((fistp_trunc(v5) << 8 | fistp_trunc(v6)) << 8 | fistp_trunc(v7)) << 8)
                | fistp_trunc(v8);
            g.write::<u32>(rec_at + 0xc, c2);
            g.write::<u32>(at(0x24), c2);
            g.write::<u16>(rec_at + 0x14, 0);
            g.write::<u16>(rec_at + 0x10, 0);
            g.write::<u32>(at(0x28), rec_at + 0x18);
            g.write::<u16>(rec_at + 0x12, kk as u16);
            g.write::<u16>(rec_at + 0x16, (kk + 1) as u16);
            pt = g.read::<u32>(at(0x2c));
        }
    }
    rec = g.read::<u32>(at(0x28));
    g.write::<u8>(rec + 1, 0xff);
    g.write::<u16>(w(0x744) + 0x12, k_out as u16);
    // the record `R`
    g.write::<u32>(r, 0x2404);
    let vb = g.read::<u32>(at(0x48));
    g.write::<u32>(r + 0x38, vb);
    let x = g.read::<u32>(g.read::<u32>(w(0x763)));
    g.write::<u8>(r + 8, 0x48);
    g.write::<u32>(r + 0x1c, 0);
    g.write::<u32>(r + 0x3c, 0);
    g.write::<u32>(r + 4, x);
    for (o, v) in [
        (0x20u32, m3),
        (0x44, m3),
        (0x24, m1),
        (0x28, m1),
        (0x30, m2),
        (0x2c, m1),
    ] {
        g.write::<u32>(r + o, v);
    }
    let gy = g.read::<u32>(w(0x7d5));
    g.write::<u32>(r + 0x34, gy);
    for (o, v) in [(0xcu32, 0u32), (0x10, 0), (0x18, 0x8080_8080), (0x40, 0)] {
        g.write::<u32>(r + o, v);
    }
    // junk read: C + 0..0x18, R + 9..0xb and R + 0x14 never written; 0 (brief 2026-10-24)
    g.write_bytes(c, &[0u8; 0x1c]);
    g.write_bytes(r + 9, &[0u8; 3]);
    g.write::<u32>(r + 0x14, 0);
    // call width: 2
    let h = g.call(slot(g, 0x7e1, 772), 0, &[obj, 0x7dea]);
    let (lo, hi) = (h.eax, h.edx);
    let pf = g.read::<u32>(w(0x81b));
    g.write::<u32>(c + 0x1c, w(0x822));
    let saved = g.read::<u32>(pf);
    g.write::<u32>(at(0x14), saved);
    g.write::<u32>(pf, 1);
    g.write::<u32>(at(0x48), lo);
    g.write::<u32>(at(0x44), hi);
    // call width: 2
    let ok = g.call(slot(g, 0x836, 773), 0, &[lo, hi]).eax;
    let pf = g.read::<u32>(w(0x854));
    g.write::<u32>(pf, saved);
    if ok != 0 {
        g.write::<u32>(pf, 1);
        // call width: 4
        g.call(slot(g, 0x876, 819), 0, &[lo, hi, c, r]);
        let pf = g.read::<u32>(w(0x88b));
        g.write::<u32>(pf, 1);
        // call width: 3
        g.call(slot(g, 0x89b, 865), 0, &[lo, hi, 0x8000_0000]);
        let pf = g.read::<u32>(w(0x8aa));
        g.write::<u32>(pf, saved);
        return Ret::eax(pf);
    }
    let b8 = u32::from(g.read::<u8>(r + 8));
    // call width: 1
    let a = g.call(slot(g, 0x8d5, 776), 0, &[b8]).eax;
    let m = g.read::<u32>(r + 0x44);
    // call width: 1
    let bits = a | g.call(slot(g, 0x8e3, 777), 0, &[m]).eax;
    let idx = g.read::<u32>(at(0x9c));
    g.write::<u16>(idx + 2, 0xf);
    g.write::<u16>(w(0x90e) + 0x12, 0x11);
    let pf = g.read::<u32>(w(0x91c));
    let saved = g.read::<u32>(pf);
    g.write::<u32>(pf, 1);
    // call width: 5
    g.call(slot(g, 0x941, 820), 0, &[lo, hi, c, r, bits]);
    let pf = g.read::<u32>(w(0x952));
    g.write::<u32>(pf, saved);
    Ret::eax(pf)
}

/// [`tail_draw_0150`] (the `rel32`s: three `__security_check_cookie`, the
/// window, `memset`; the loops' jumps stay inside).
pub const TAIL_DRAW_0150: Sig = Sig {
    len: 2415,
    hash: 0x6e8a_ed7c_a88c_3c83,
    calls: &[
        (0x95, Callee::Any),
        (0xb2, Callee::Sig(&crate::s4_small::STRIP_WINDOW)),
        (0x16c, Callee::Import("memset")),
        (0x8c3, Callee::Any),
        (0x967, Callee::Any),
    ],
    imports: &[],
};

/// The summon's start (`magic_0667` 0x100040e0 and `magic_0668`, 1714
/// bytes; cdecl `(a)`, an `ebp` frame with one local).
/// - `*G_A = a`, `*G_B` 0; the summoner `S` = slot 168 `(a)` is the first
///   record's id; its side (slot 293) gives the next two ids, `S + 1` and
///   `S + 2` or `S - 1` and `S - 2`.
/// - The three records measured
///   ([`target_measure`](crate::s5_misc::target_measure), `ecx` from
///   `G_REC - 0x44`, 0xcc apart); their centre `(x, z) / K` with `y` =
///   slot 289 `(S)` and `w` 1.0, copied (325).
/// - The targets (slot 218 `(*G_A, &N)`, `N` bytes) measured; the last
///   record's id slot 294 `(S)` (or 0xff by slot 293) measured; slot 288
///   `(0x10, &a)` lists `a` ids, each one not yet among the records
///   added after them and measured.
/// - The part sets (slots 718, 222, 719 twelve times, 221) into their
///   globals; 34 u16 flags 0 and one 0x7fff; slot 291 `(*G_A, first
///   target, 0, 0, 0)`; the count `N * 5`; 20 more u16 and two words 0;
///   the camera blocks reset
///   ([`camera_blocks_reset`](crate::s6_misc::camera_blocks_reset)); the
///   block table entry and three record lists cleared; slot 312 `(CB, 0,
///   0, 0)` into a global.
///
/// Returns 0.
#[allow(clippy::too_many_lines, clippy::cast_possible_truncation)] // the steps; a u16 count
pub fn summon_start_0667<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, div};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let e = fr.esp;
    let arg = e + 4;
    let a = fr.arg(g, 0);
    let measure = |g: &mut G, call: u32, rec: u32| {
        let fr = Frame {
            at: img.rel32_target(f + call),
            esp: e.wrapping_sub(0x18),
            ecx: rec,
            ..*fr
        };
        crate::s5_misc::target_measure(g, img, &fr);
    };
    g.write::<u32>(w(0x8), a);
    g.write::<u32>(w(0x14), 0);
    // call width: 1
    let s = g.call(slot(g, 0xe, 168), 0, &[a]).eax;
    g.write::<u32>(w(0x25), s);
    // call width: 1
    let side = g.call(slot(g, 0x2b, 293), 0, &[s]).eax;
    let id = g.read::<u32>(w(0x39));
    let next = if side == 0 {
        g.write::<u32>(w(0x48), id.wrapping_add(1));
        id.wrapping_add(2)
    } else {
        g.write::<u32>(w(0x55), id.wrapping_sub(1));
        id.wrapping_sub(2)
    };
    g.write::<u32>(w(0x5f), next);
    // the three records and their centre
    let mut r = w(0x65);
    loop {
        measure(g, 0x74, r.wrapping_sub(0x44));
        let x = add(g.read::<f32>(r.wrapping_sub(8)), g.read::<f32>(w(0x85)));
        let z = add(g.read::<f32>(r), g.read::<f32>(w(0x8d)));
        r = r.wrapping_add(0xcc);
        g.write::<f32>(w(0x9b), x);
        g.write::<f32>(w(0xa3), z);
        if r.cast_signed() >= w(0xa9).cast_signed() {
            break;
        }
    }
    let x = div(g.read::<f32>(w(0x9b)), g.read::<f32>(w(0xb3)));
    let z = div(g.read::<f32>(w(0xa3)), g.read::<f32>(w(0xbb)));
    let s = g.read::<u32>(w(0xc6));
    g.write::<f32>(w(0xce), x);
    g.write::<f32>(w(0xd6), z);
    // call width: 1
    let y = st0_f32(g.call(slot(g, 0xc0, 289), 0, &[s]).st0);
    g.write::<f32>(w(0xe9), y);
    g.write::<u32>(w(0xef), 0x3f80_0000);
    // call width: 2
    g.call(slot(g, 0xe3, 325), 0, &[w(0x103), w(0xfe)]);
    let ga = g.read::<u32>(w(0x115));
    // call width: 2
    let list = g.call(slot(g, 0x10a, 218), 0, &[ga, w(0x10f)]).eax;
    // the targets
    let mut i = 0i32;
    let mut rec = w(0x131);
    while i < g.read::<i32>(w(0x12a)) {
        let b = u32::from(g.read::<u8>(list.wrapping_add(i.cast_unsigned())));
        g.write::<u32>(rec, b);
        measure(g, 0x13e, rec);
        i += 1;
        rec = rec.wrapping_add(0xcc);
        if i >= g.read::<i32>(w(0x14b)) {
            break;
        }
    }
    let s = g.read::<u32>(w(0x158));
    // call width: 1
    if g.call(slot(g, 0x152, 293), 0, &[s]).eax == 0 {
        let s = g.read::<u32>(w(0x172));
        // call width: 1
        let id = g.call(slot(g, 0x16c, 294), 0, &[s]).eax;
        g.write::<u32>(w(0x182), id);
    } else {
        g.write::<u32>(w(0x18a), 0xff);
    }
    measure(g, 0x198, w(0x193));
    // the listed ids not among the records yet
    // call width: 2
    let ids = g.call(slot(g, 0x1a1, 288), 0, &[0x10, arg]).eax;
    g.write::<u32>(e.wrapping_sub(8), ids);
    let mut added = 0u32;
    g.write::<u32>(w(0x1bb), 0);
    let mut j = 0i32;
    while j < g.read::<i32>(arg) {
        let id = u32::from(g.read::<u8>(ids.wrapping_add(j.cast_unsigned())));
        let seen = |g: &G, at: u32, n: u32| (0..n).any(|k| g.read::<u32>(at + 0xcc * k) == id);
        let cnt = g.read::<u32>(w(0x1ca));
        let known = seen(g, w(0x1da), 3) || seen(g, w(0x1f2), cnt) || seen(g, w(0x215), 1);
        if !known {
            let at = added.wrapping_mul(0xcc).wrapping_add(w(0x239));
            added += 1;
            g.write::<u32>(w(0x240), added);
            g.write::<u32>(at, id);
            measure(g, 0x247, at);
            added = g.read::<u32>(w(0x253));
        }
        j += 1;
    }
    // the part sets
    for (t, s) in [(0x265u32, 0x26au32), (0x292, 0x297)] {
        // call width: 1
        g.call(slot(g, t, 718), 0, &[w(s)]);
        let (t2, s2) = if s == 0x26a {
            (0x277, 0x28b)
        } else {
            (0x2a4, 0x2b8)
        };
        // call width: 4
        g.call(slot(g, t2, 222), 0, &[w(s2), 0, 0xa, 0x100]);
    }
    g.write::<u32>(w(0x2d0), w(0x2d4));
    // call width: 1
    g.call(slot(g, 0x2bf, 718), 0, &[w(0x2c4)]);
    g.write::<u32>(w(0x2ec), w(0x2f0));
    // call width: 1
    g.call(slot(g, 0x2db, 718), 0, &[w(0x2e0)]);
    let parts: [(u32, u32, u32); 11] = [
        (0x2f7, 0x307, 0x30e),
        (0x313, 0x323, 0x32d),
        (0x332, 0x342, 0x349),
        (0x34e, 0x35e, 0x365),
        (0x36a, 0x37a, 0x381),
        (0x386, 0x396, 0x39d),
        (0x3a2, 0x3b2, 0x3b9),
        (0x3be, 0x3ce, 0x3d8),
        (0x3dd, 0x3ed, 0x3f4),
        (0x3f9, 0x409, 0x410),
        (0x415, 0x425, 0x431),
    ];
    for (i, (t, h, out)) in (0u32..).zip(parts) {
        let set = g.read::<u32>(w(h));
        // call width: 3
        let x = g.call(slot(g, t, 719), 0, &[set, 6, i]).eax;
        g.write::<u32>(w(out), x);
    }
    let set = g.read::<u32>(w(0x446));
    // call width: 4
    g.call(slot(g, 0x436, 221), 0, &[set, 0, 0xa, 0x100]);
    let set = g.read::<u32>(w(0x45d));
    // call width: 3
    let x = g.call(slot(g, 0x44d, 719), 0, &[set, 5, 1]).eax;
    g.write::<u32>(w(0x464), x);
    for o in (0x46fu32..=0x52f).step_by(6) {
        g.write::<u16>(w(o), 0);
    }
    g.write::<u16>(w(0x53a), 0x7fff);
    let (ga, t0) = (g.read::<u32>(w(0x557)), g.read::<u32>(w(0x551)));
    // call width: 5
    g.call(slot(g, 0x53f, 291), 0, &[ga, t0, 0, 0, 0]);
    let n5 = g.read::<u32>(w(0x55e)).wrapping_mul(5);
    g.write::<u16>(w(0x56a), n5 as u16);
    for o in (0x572u32..=0x5de).step_by(6) {
        g.write::<u16>(w(o), 0);
    }
    g.write::<u32>(w(0x5e3), 0);
    g.write::<u32>(w(0x5e8), 0);
    let reset = Frame {
        at: img.rel32_target(f + 0x5ed),
        esp: e.wrapping_sub(0x18),
        ..*fr
    };
    crate::s6_misc::camera_blocks_reset(g, img, &reset);
    let i = g.read::<u32>(w(0x5f2)).wrapping_mul(0x190);
    g.write::<u32>(i.wrapping_add(w(0x5ff)), 0);
    g.write::<u32>(w(0x60a), 1);
    g.write::<u32>(w(0x614), 0);
    for k in 0..0x100u32 {
        g.write::<u32>(w(0x622) + 0x4c * k, 0);
    }
    let mut p = w(0x63d);
    loop {
        g.write::<u32>(p - 0x24, 0);
        g.write::<u32>(p, 0);
        p += 0xa8;
        if p.cast_signed() >= w(0x654).cast_signed() {
            break;
        }
    }
    for k in 0..0x200u32 {
        let q = w(0x660) + 0x3c * k;
        g.write::<u32>(q - 0x18, 0);
        g.write::<u32>(q, 0);
    }
    g.write::<u32>(w(0x698), 0);
    g.write::<u32>(w(0x69e), 0);
    // call width: 4
    let x = g.call(slot(g, 0x684, 312), 0, &[w(0x692), 0, 0, 0]).eax;
    g.write::<u32>(w(0x6a8), x);
    Ret::eax(0)
}

/// [`summon_start_0667`] (the `rel32`s: four target measures, the camera
/// blocks reset).
pub const SUMMON_START_0667: Sig = Sig {
    len: 1714,
    hash: 0x57b7_ceb5_aefe_2350,
    calls: &[
        (0x74, Callee::Sig(&crate::s5_misc::TARGET_MEASURE)),
        (0x13e, Callee::Sig(&crate::s5_misc::TARGET_MEASURE)),
        (0x198, Callee::Sig(&crate::s5_misc::TARGET_MEASURE)),
        (0x247, Callee::Sig(&crate::s5_misc::TARGET_MEASURE)),
        (0x5ed, Callee::Sig(&crate::s6_misc::CAMERA_BLOCKS_RESET)),
    ],
    imports: &[],
};

/// `magic_0173`'s summon task (0x10006b40, 1581 bytes; a task function,
/// cdecl `(task)`, the record `R = task[1]`; an 8-aligned frame `Z`, the
/// `esp` after the pushes, the requests `S` at `Z + 0x1c` and `S2` at
/// `Z + 0x34`). By the state `R + 0x18` and the count `R + 0x2c` (one more
/// on every way out):
/// - 0: the model group `[G_N1 (vtable), G_PAR, *G_P, 0, 1]`
///   ([`model_nodes`](crate::s3_summon::model_nodes)), state 10;
/// - 10: slot 228 `(*G_K, 0xc2)`, state 11, the count 0;
/// - 11, from 5 on: the main node (slot 215 `(*G_K)`,
///   [`node_init`](crate::m0042::node_init)), the static child, a node
///   appended to `G_PAR` (colour 1, 1, 1, 0.3) and inserted with its
///   group ([`node_insert`](crate::nodes::node_insert)), the particle node
///   ([`particle_node_with`](crate::s3_nodes::particle_node_with)) and
///   the effect node ([`effect_node_a`](crate::s4_fam446::effect_node_a));
///   state 0x14, the count 0;
/// - 0x14, from 0x14 on: slot 676, then per target a node
///   ([`node_init`](crate::m0003::node_init)), its linked node
///   ([`link_node_942`](crate::s4_small::link_node_942), `(15, 1, 6)`) and
///   a colour node ([`colour_node_init`](crate::sa_misc::colour_node_init));
///   the second effect node; state 0x15, the count 0;
/// - 0x15: up to the count 15, each target not done yet whose model (slot
///   941) has bit 0x800 gets a node appended and inserted and a particle
///   node ([`particle_node`](crate::sb_nodes::particle_node)), slot 239,
///   done; a fade `*G_A` shrinks by `K2` while below `K` (0.8 once past
///   it); at 15 state 0x1e;
/// - 0x1e: the two colour bytes as floats, the scale 1.0, state 0x1f;
///   0x1f: the scale less `K3` (not below 0) times the colours back into
///   the bytes; at 0, state 0x2710;
/// - 0x2710: when slot 411 `(*G_K)` says so, slot 213 `(*G_K, 0)` and the
///   task node unlinked.
///
/// Returns `eax` as the original leaves it (the dispatch's table byte,
/// the last call's, a fade byte, the caller's for state 0x15).
#[allow(clippy::too_many_lines)] // the original's states, in order
pub fn summon_task_0173<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{cvt, cvtt, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    let z = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x50);
    let at = |o: u32| z.wrapping_add(o);
    g.write::<u32>(at(0x18), r);
    let s = at(0x1c);
    let call = |o: u32| img.rel32_target(f + o);
    let done = |g: &mut G, eax: u32| {
        let c = g.read::<u32>(r + 0x2c).wrapping_add(1);
        g.write::<u32>(r + 0x2c, c);
        Ret::eax(eax)
    };
    let state = g.read::<u32>(r + 0x18);
    let effect = |g: &mut G, node_op: u32, n2_op: u32, a3: u32, at_call: u32| {
        // push a3; push ecx; push G_N2; push ecx (the `ecx` words unread)
        g.write::<u32>(z.wrapping_sub(4), a3);
        g.write::<u32>(z.wrapping_sub(8), 0);
        g.write::<u32>(z.wrapping_sub(0xc), w(n2_op));
        g.write::<u32>(z.wrapping_sub(0x10), 0);
        let fr = Frame {
            at: call(at_call),
            esp: z.wrapping_sub(0x14),
            ecx: w(node_op),
            ..*fr
        };
        crate::s4_fam446::effect_node_a(g, img, &fr).eax
    };
    match state {
        0 | 10 | 11 | 0x14 | 0x15 => {}
        0x1e | 0x1f => {
            if state == 0x1e {
                let c1 = cvt(u32::from(g.read::<u8>(w(0x569))));
                g.write::<f32>(r + 0x1c, c1);
                let c2 = cvt(u32::from(g.read::<u8>(w(0x57c))));
                g.write::<f32>(r + 0x20, c2);
                g.write::<u32>(r + 0x24, 0x3f80_0000);
                g.write::<u32>(r + 0x18, 0x1f);
            }
            let x = g.read::<f32>(r + 0x24);
            if x == 0.0 {
                g.write::<u32>(r + 0x18, 0x2710);
                // `lahf` after `ucomiss` of an equal pair: `ah` = ZF | 2
                let eax = if state == 0x1e { 0x1f } else { 0 };
                return done(g, eax | 0x4200);
            }
            let x = sub(x, k(g, 0x5ae));
            g.write::<f32>(r + 0x24, x);
            if 0.0 > x {
                g.write::<u32>(r + 0x24, 0);
            }
            let a = cvtt(mul(g.read::<f32>(r + 0x1c), g.read::<f32>(r + 0x24)));
            g.write::<u8>(w(0x5d2), a.cast_unsigned().to_le_bytes()[0]);
            let b = cvtt(mul(g.read::<f32>(r + 0x20), g.read::<f32>(r + 0x24)));
            g.write::<u8>(w(0x5e5), b.cast_unsigned().to_le_bytes()[0]);
            return done(g, b.cast_unsigned());
        }
        0x2710 => {
            let gk = g.read::<u32>(w(0x513));
            // call width: 1
            let ok = g.call(slot(g, 0x50d, 411), 0, &[gk]).eax;
            if ok == 0 {
                return done(g, ok);
            }
            let gk = g.read::<u32>(w(0x533));
            // call width: 2
            g.call(slot(g, 0x52b, 213), 0, &[gk, 0]);
            let node = g.read::<u32>(task);
            let (prev, next) = (g.read::<u32>(node + 4), g.read::<u32>(node + 8));
            g.write::<u32>(prev + 8, next);
            let next = g.read::<u32>(node + 8);
            let prev = g.read::<u32>(node + 4);
            g.write::<u32>(next + 4, prev);
            g.write::<u32>(node + 8, node);
            g.write::<u32>(node + 4, node);
            return done(g, prev);
        }
        s if s > 0x15 => {
            let eax = s.wrapping_sub(0x1e).wrapping_sub(1).wrapping_sub(0x26f1);
            return done(g, eax);
        }
        _ => return done(g, 4),
    }
    if state == 0x15 {
        if g.read::<i32>(r + 0x2c) >= 0xf {
            g.write::<u32>(r + 0x18, 0x1e);
            return done(g, fr.eax);
        }
        let mut eax = fr.eax;
        let n = g.read::<i32>(w(0x342));
        if n > 0 {
            let (mut tc, mut pn, mut ta) = (w(0x34d), w(0x355), w(0x35d));
            let (mut node2, mut cn) = (w(0x362), w(0x36b));
            g.write::<u32>(at(0x10), pn);
            g.write::<u32>(at(0xc), ta);
            g.write::<u32>(at(0x14), tc);
            let mut i = 0u32;
            loop {
                if g.read::<u8>(w(0x382).wrapping_add(i)) == 0 {
                    let id = g.read::<u32>(tc);
                    // call width: 1
                    let model = g.call(slot(g, 0x390, 941), 0, &[id]).eax;
                    if g.read::<u16>(model + 0x3e) & 0x800 != 0 {
                        let node = cn - 0x20;
                        let parent = w(0x3b2);
                        crate::sb_misc::node_append(g, img, call(0x3b7), node, parent, 0);
                        g.write::<u32>(at(0x30), node);
                        for (o, v) in [
                            (-4i32, 0x3f80_0000u32),
                            (0, 0x3f80_0000),
                            (4, 0x3f80_0000),
                            (8, 0x3fa6_6666),
                        ] {
                            g.write::<u32>(cn.wrapping_add_signed(o), v);
                        }
                        let tav = g.read::<u32>(at(0xc));
                        for (o, v) in [
                            (0u32, node2),
                            (4, parent),
                            (8, w(0x3ee)),
                            (0xc, tav),
                            (0x10, w(0x3fa)),
                        ] {
                            g.write::<u32>(s + o, v);
                        }
                        let ins = Frame {
                            at: call(0x3ff),
                            esp: z.wrapping_sub(4),
                            ecx: s,
                            ..*fr
                        };
                        crate::nodes::node_insert(g, img, &ins);
                        let pnode = g.read::<u32>(at(0x10));
                        let q = g.read::<u32>(w(0x408));
                        g.write::<u8>(node2 + 0x148, 1);
                        g.write::<u32>(s, pnode);
                        g.write::<u32>(pnode, w(0x419));
                        for (o, v) in [
                            (4u32, w(0x425)),
                            (8, node2),
                            (0xc, q),
                            (0x10, w(0x435)),
                            (0x14, 1),
                        ] {
                            g.write::<u32>(s + o, v);
                        }
                        crate::sb_nodes::particle_node(g, img, call(0x442), s);
                        let tav = g.read::<u32>(at(0xc));
                        let x = g.read::<u32>(g.read::<u32>(tav + 0x38) + 0x50);
                        let gk = g.read::<u32>(w(0x459));
                        // call width: 3
                        g.call(slot(g, 0x453, 239), 0, &[gk, x, 0]);
                        g.write::<u8>(w(0x46a).wrapping_add(i), 1);
                    }
                    ta = g.read::<u32>(at(0xc));
                    pn = g.read::<u32>(at(0x10));
                    tc = g.read::<u32>(at(0x14));
                }
                i += 1;
                tc = tc.wrapping_add(0x60);
                ta = ta.wrapping_add(0x44);
                pn = pn.wrapping_add(0x100);
                node2 = node2.wrapping_add(0x14c);
                cn = cn.wrapping_add(0x2c);
                g.write::<u32>(at(0x14), tc);
                g.write::<u32>(at(0xc), ta);
                g.write::<u32>(at(0x10), pn);
                eax = tc;
                if i.cast_signed() >= g.read::<i32>(w(0x49f)) {
                    break;
                }
            }
        }
        let k1 = k(g, 0x4b1);
        let a = g.read::<f32>(w(0x4b9));
        if k1 > a {
            let a = mul(a, k(g, 0x4ca));
            g.write::<f32>(w(0x4d5), a);
            if a > k1 {
                g.write::<u32>(w(0x4e1), 0x3f4c_cccd);
            }
        }
        return done(g, eax);
    }
    if state == 0x14 {
        if g.read::<i32>(r + 0x2c) < 0x14 {
            return done(g, 3);
        }
        // call width: 3
        g.call(slot(g, 0x1d7, 676), 0, &[w(0x1e6), 6, 0x60]);
        g.write::<u32>(at(0x14), 0);
        if g.read::<i32>(w(0x1f1)) > 0 {
            let (mut node, mut val) = (w(0x205), w(0x20a));
            g.write::<u32>(at(0x20), w(0x212));
            let (mut cn, mut ln) = (w(0x217), w(0x21c));
            g.write::<u32>(at(0x10), node);
            g.write::<u32>(at(0xc), val);
            loop {
                g.write::<u32>(at(0x24), val);
                g.write::<u32>(s, node);
                crate::m0003::node_init(g, img, call(0x23d), s);
                // push ecx (unread); push the node
                g.write::<u32>(z.wrapping_sub(4), 0);
                g.write::<u32>(z.wrapping_sub(8), node);
                g.write::<u8>(node + 0x40, 3);
                let link = ln.wrapping_sub(0x30);
                let lf = Frame {
                    at: call(0x250),
                    esp: z.wrapping_sub(0xc),
                    ecx: link,
                    ..*fr
                };
                crate::s4_small::link_node_942(g, img, &lf);
                let colour = cn.wrapping_sub(0x54);
                g.write::<u32>(ln.wrapping_sub(4), 0x4170_0000);
                g.write::<u32>(ln, 0x3f80_0000);
                g.write::<u32>(ln + 4, 0x40c0_0000);
                let s2 = at(0x34);
                for (o, v) in [
                    (0u32, colour),
                    (4, w(0x27a)),
                    (8, link),
                    (0xc, w(0x286)),
                    (0x10, w(0x28e)),
                    (0x14, w(0x296)),
                    (0x18, w(0x29e)),
                ] {
                    g.write::<u32>(s2 + o, v);
                }
                g.write::<u32>(colour, w(0x2a4));
                crate::sa_misc::colour_node_init(g, img, call(0x2a9), s2);
                let b = g.read::<u8>(cn - 0x20) | 0x10;
                g.write::<u8>(cn - 0x20, b);
                let b = g.read::<u8>(cn) | 1;
                g.write::<u8>(cn, b);
                let i = g.read::<i32>(at(0x14)) + 1;
                node = g.read::<u32>(at(0x10)) + 0x44;
                val = g.read::<u32>(at(0xc)) + 0x98;
                cn += 0x60;
                ln += 0x44;
                g.write::<u32>(at(0x10), node);
                g.write::<i32>(at(0x14), i);
                g.write::<u32>(at(0xc), val);
                if i >= g.read::<i32>(w(0x2de)) {
                    break;
                }
            }
        }
        let eax = effect(g, 0x2f6, 0x2f0, 1, 0x2fb);
        let x = g.read::<u32>(w(0x301)) | 1;
        g.write::<u32>(w(0x301), x);
        g.write::<u32>(w(0x308), 0);
        let st = g.read::<u32>(r + 0x18).wrapping_add(1);
        g.write::<u32>(r + 0x18, st);
        g.write::<u32>(r + 0x2c, 0);
        return done(g, eax);
    }
    if state == 0 {
        let p = g.read::<u32>(w(0x40));
        for (o, v) in [(0u32, w(0x4c)), (4, w(0x54)), (8, p), (0xc, 0)] {
            g.write::<u32>(s + o, v);
        }
        g.write::<u8>(s + 0x10, 1);
        g.write::<u32>(w(0x6b), w(0x6f));
        let mf = Frame {
            at: call(0x74),
            esp: z.wrapping_sub(4),
            ecx: s,
            ..*fr
        };
        crate::s3_summon::model_nodes(g, img, &mf);
        g.write::<u32>(r + 0x18, 0xa);
    }
    if state == 0 || state == 10 {
        let gk = g.read::<u32>(w(0x8b));
        // call width: 2
        let eax = g.call(slot(g, 0x80, 228), 0, &[gk, 0xc2]).eax;
        let st = g.read::<u32>(r + 0x18).wrapping_add(1);
        g.write::<u32>(r + 0x18, st);
        g.write::<u32>(r + 0x2c, 0);
        // the count is below 5 now
        return done(g, eax);
    }
    // state 11
    if g.read::<i32>(r + 0x2c) < 5 {
        return done(g, 2);
    }
    let gk = g.read::<u32>(w(0xb5));
    g.write::<u32>(s, w(0xc3));
    g.write::<u32>(s + 4, w(0xcb));
    // call width: 1
    let h = g.call(slot(g, 0xaf, 215), 0, &[gk]).eax;
    g.write::<u32>(s + 8, h);
    g.write::<u32>(s + 0xc, 0x16);
    crate::m0042::node_init(g, img, call(0xe5), s);
    g.write::<u32>(w(0xf0), 0xbf80_0000);
    crate::sa_misc::static_child(g, img, call(0xf9), w(0xea));
    let b = g.read::<u8>(w(0xfe));
    let (parent, node) = (w(0x103), w(0x108));
    g.write::<u8>(w(0x10d), b);
    crate::sb_misc::node_append(g, img, call(0x112), node, parent, 0);
    g.write::<u32>(at(0x30), node);
    for op in [0x120u32, 0x12a, 0x134] {
        g.write::<u32>(w(op), 0x3f80_0000);
    }
    g.write::<u32>(w(0x13e), 0x3e99_999a);
    for (o, v) in [
        (0u32, w(0x14a)),
        (4, parent),
        (8, w(0x156)),
        (0xc, w(0x15e)),
        (0x10, w(0x166)),
    ] {
        g.write::<u32>(s + o, v);
    }
    let ins = Frame {
        at: call(0x16b),
        esp: z.wrapping_sub(4),
        ecx: s,
        ..*fr
    };
    crate::nodes::node_insert(g, img, &ins);
    // push 0; sub esp, 8; push G_N5: the particle node's `(a, _, _, 0)`
    g.write::<u32>(z.wrapping_sub(4), 0);
    g.write::<u32>(z.wrapping_sub(8), 0);
    g.write::<u32>(z.wrapping_sub(0xc), 0);
    g.write::<u32>(z.wrapping_sub(0x10), w(0x17a));
    g.write::<u8>(w(0x180), 1);
    let pf = Frame {
        at: call(0x186),
        esp: z.wrapping_sub(0x14),
        ecx: w(0x175),
        ..*fr
    };
    crate::s3_nodes::particle_node_with(g, img, &pf);
    let eax = effect(g, 0x197, 0x191, 0, 0x19c);
    let x = g.read::<u32>(w(0x1a2)) | 1;
    g.write::<u32>(w(0x1a2), x);
    g.write::<u32>(w(0x1a9), 0);
    g.write::<u32>(r + 0x2c, 0);
    let c = g.read::<u32>(r + 0x2c).wrapping_add(1);
    g.write::<u32>(r + 0x2c, c);
    g.write::<u32>(r + 0x18, 0x14);
    Ret::eax(eax)
}

/// [`summon_task_0173`] (the `rel32`s: its fourteen node set-ups; the
/// state table stays inside).
pub const SUMMON_TASK_0173: Sig = Sig {
    len: 1581,
    hash: 0x1b4c_2bb3_a6a2_a6ca,
    calls: &[
        (0x74, Callee::Sig(&crate::s3_summon::MODEL_NODES)),
        (0xe5, Callee::Sig(&crate::m0042::NODE_INIT)),
        (0xf9, Callee::Sig(&crate::sa_misc::STATIC_CHILD)),
        (0x112, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x16b, Callee::Sig(&crate::nodes::INSERT)),
        (0x186, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x19c, Callee::Sig(&crate::s4_fam446::EFFECT_NODE_A)),
        (0x23d, Callee::Sig(&crate::m0003::NODE_INIT)),
        (0x250, Callee::Sig(&crate::s4_small::LINK_NODE_942)),
        (0x2a9, Callee::Sig(&crate::sa_misc::COLOUR_NODE_INIT)),
        (0x2fb, Callee::Sig(&crate::s4_fam446::EFFECT_NODE_A)),
        (0x3b7, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x3ff, Callee::Sig(&crate::nodes::INSERT)),
        (0x442, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
    ],
    imports: &[],
};

/// A static node appended last to the list `L` (inline in
/// [`summon_task_0272`]): `node`'s pass u16 0xff (or its words at `pass`,
/// `own` given as operands), its own empty child list, the parent `L`,
/// the previous last `*G_LAST`, the list's first (`G_FIRST`) when it was
/// empty or the old last's next, the count u16 up, `*G_LAST = node`.
#[allow(clippy::too_many_arguments)] // the inline stores' operands
fn static_append<G: Game>(g: &mut G, img: &Image, f: u32, node: u32, ops: [u32; 5]) {
    let w = |o: u32| img.word(f + o);
    let [parent_op, tail_op, empty_op, head_op, count_op] = ops;
    let parent = w(parent_op);
    g.write::<u16>(node + 0x1a, 0xff);
    g.write::<u32>(node + 0x10, node);
    g.write::<u32>(node + 0xc, node);
    g.write::<u32>(node + 0x14, parent);
    g.write::<u32>(node + 4, parent);
    let prev = g.read::<u32>(w(tail_op));
    g.write::<u32>(node + 8, prev);
    let prev = g.read::<u32>(w(tail_op));
    if prev == w(empty_op) {
        g.write::<u32>(w(head_op), node);
    } else {
        g.write::<u32>(prev + 4, node);
    }
    let n = g.read::<u16>(w(count_op)).wrapping_add(1);
    g.write::<u16>(w(count_op), n);
}

/// `magic_0272`'s summon task (0x10006960, 1985 bytes; a task function,
/// cdecl `(task)`, the record `R = task[1]`; an 8-aligned frame `Z`, the
/// `esp` after the pushes). By the state `R + 0x18` (a state set and
/// below 0x2711 runs at once; the count `R + 0x2c` one more on the other
/// ways out):
/// - 0: the summon's nodes ([`summon_nodes`](crate::s4_small::summon_nodes)
///   for `[N, G_PAR, *G_K, 0x10]`), `R + 0x30` = the record of the actor
///   id `*G_ID` (0x98 apart), each record shown
///   ([`show`](crate::m0070::show)), three static nodes appended
///   ([`static_append`] and the same stores inline; the second's vector
///   op, [`vector_op`](crate::sa_summon::vector_op)), slot 719
///   `(PART, 6, 0)` into the third, state 10;
/// - 10: slot 228 `(*G_K, 0xb8)`, a flag; 11: when slot 411 says so, slot
///   228 `(.., 0xb9)` and slot 701 `(*G_K, 2, actor, 0, that, 0, 0, 0,
///   1.0, 0, 0)`, two nodes appended
///   ([`node_append`](crate::sb_misc::node_append)) with slot 531 between,
///   inserted ([`node_insert`](crate::nodes::node_insert)) and the
///   particle node ([`particle_node_with`](crate::s3_nodes::particle_node_with));
/// - 12: when slot 411 says so, the bone node
///   ([`bone_node_setup`](crate::o6_fam_bone::bone_node_setup) on `R +
///   0x34`), state 20; 20: slot 228 `(.., 0xba)`, the count 0; 21, from
///   5 on: a node appended and inserted, a particle node, slot 239 `(*G_K,
///   -5, 0)`; 22: when slot 411 says so, state 30;
/// - 30: slot 228 `(.., 0xbb)` and slot 701 `(*G_K, 1, ..)`; 31: when 411
///   says so, slot 228 `(.., 0xbc)`; 32: when 411 says so, slot 213
///   `(*G_K, 0)`, state 0x2710;
/// - 0x2710: with bit 2 of `*G_FL`, a flag, slot 213 `(*G_K, 0)` and the
///   task node unlinked.
///
/// Returns `eax` as the original leaves it (the dispatch's table byte or
/// the state, the last call's, the node's link).
#[allow(clippy::too_many_lines)] // the original's states, in order
pub fn summon_task_0272<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    let z = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x38);
    let at = |o: u32| z.wrapping_add(o);
    let done = |g: &mut G, eax: u32| {
        let c = g.read::<u32>(r + 0x2c).wrapping_add(1);
        g.write::<u32>(r + 0x2c, c);
        Ret::eax(eax)
    };
    let gk = |g: &G, o: u32| g.read::<u32>(w(o));
    let says = |g: &mut G, t: u32, k: u32| {
        let x = gk(g, k);
        // call width: 1
        g.call(slot(g, t, 411), 0, &[x]).eax
    };
    // slot 701 with the frame's argument words (the 228 answer in the
    // middle)
    let s701 = |g: &mut G, t: u32, k: u32, mode: u32, a: u32| {
        let actor = g.read::<u32>(g.read::<u32>(r + 0x30) + 0x50);
        let x = gk(g, k);
        // call width: 11
        g.call(
            slot(g, t, 701),
            0,
            &[x, mode, actor, 0, a, 0, 0, 0, 0x3f80_0000, 0, 0],
        )
        .eax
    };
    loop {
        let state = g.read::<u32>(r + 0x18);
        match state {
            0 => {
                let k = gk(g, 0x3e);
                let s = at(0x20);
                g.write::<u32>(w(0x48), w(0x4c));
                g.write::<u32>(w(0x52), w(0x56));
                for (o, v) in [(0u32, w(0x5e)), (4, w(0x66)), (8, k), (0xc, 0x10)] {
                    g.write::<u32>(s + o, v);
                }
                g.write::<u32>(w(0x78), w(0x7c));
                let nf = Frame {
                    at: call(0x81),
                    esp: z.wrapping_sub(4),
                    ecx: s,
                    ..*fr
                };
                crate::s4_small::summon_nodes(g, img, &nf);
                // the actor's record
                let n = g.read::<i32>(w(0x87));
                if n > 0 {
                    let id = u32::from(g.read::<u8>(w(0x94)));
                    let found =
                        (0..n.cast_unsigned()).find(|&i| g.read::<u32>(w(0x99) + 0x98 * i) == id);
                    if let Some(i) = found {
                        g.write::<u32>(r + 0x30, i.wrapping_mul(0x98).wrapping_add(w(0xb8)));
                    }
                }
                let mut i = 0i32;
                let mut rec = w(0xcc);
                while i < g.read::<i32>(w(0xc1)) {
                    g.write::<u32>(at(0x10), rec);
                    g.write::<u8>(at(0x14), 0xff);
                    crate::m0070::show(g, img, call(0xde), at(0x10));
                    i += 1;
                    rec = rec.wrapping_add(0x98);
                    if i >= g.read::<i32>(w(0xeb)) {
                        break;
                    }
                }
                // the first static node
                let last = g.read::<u32>(w(0xf2));
                let n1 = w(0xfc - 4);
                g.write::<u32>(n1, w(0xfc));
                g.write::<u16>(w(0x103), 0xff);
                g.write::<u32>(w(0x10b), w(0x10f));
                g.write::<u32>(w(0x115), w(0x119));
                g.write::<u32>(w(0x11f), w(0x123));
                g.write::<u32>(w(0x129), w(0x12d));
                g.write::<u32>(w(0x132), last);
                if last == w(0x137) {
                    g.write::<u32>(w(0x13f), w(0x143));
                } else {
                    g.write::<u32>(last + 4, w(0x14c));
                }
                let c = g.read::<u16>(w(0x153)).wrapping_add(1);
                g.write::<u16>(w(0x153), c);
                let n2 = g.read::<u32>(w(0x159));
                g.write::<u32>(w(0x15f), w(0x163));
                g.write::<u32>(w(0x169), w(0x16d));
                g.write::<u32>(w(0x173), 0xff);
                let rec = g.read::<u32>(r + 0x30);
                // the second, appended to `G_PAR`
                g.write::<u32>(n2, w(0x180));
                static_append(g, img, f, n2, [0x193, 0x19f, 0x1ac, 0x1b4, 0x1c0]);
                let b = g.read::<u8>(w(0x1c5));
                g.write::<u32>(w(0x1cb), n2);
                g.write::<u32>(n2 + 0x28, 0x3f80_0000);
                g.write::<u32>(n2 + 0x38, rec);
                g.write::<u8>(n2 + 0x40, 1);
                g.write::<u32>(n2 + 0x3c, 6);
                for o in [0x2cu32, 0x30, 0x34] {
                    g.write::<u32>(n2 + o, 0);
                }
                g.write::<u8>(n2 + 0x40, 0);
                g.write::<u32>(n2 + 0x30, 0);
                g.write::<u32>(at(0x18), n2);
                let pass = b.wrapping_add(2);
                g.write::<u8>(at(0x1d), pass);
                let vf = Frame {
                    at: call(0x213),
                    esp: z.wrapping_sub(4),
                    ecx: at(0x18),
                    eax: u32::from(pass),
                    ..*fr
                };
                crate::sa_summon::vector_op(g, img, &vf);
                // the third, appended to `G_PAR`
                let n3 = g.read::<u32>(w(0x219));
                static_append(g, img, f, n3, [0x22c, 0x238, 0x245, 0x24d, 0x259]);
                let b = g.read::<u8>(w(0x25e));
                let n4 = g.read::<u32>(w(0x264));
                g.write::<u32>(w(0x26a), n3);
                g.write::<u32>(n3 + 0x2c, 0);
                g.write::<u32>(n3 + 0x20, 0);
                g.write::<u32>(n3, w(0x27e));
                g.write::<u32>(n3 + 0x1c, 0);
                g.write::<u32>(n3 + 0x24, 0);
                g.write::<u8>(n3 + 0x30, b);
                // the static part node at `G_P`, appended inline
                let last = g.read::<u32>(w(0x294));
                g.write::<u16>(w(0x29b), 0xff);
                g.write::<u32>(w(0x2a3), w(0x2a7));
                g.write::<u32>(w(0x2ad), w(0x2b1));
                g.write::<u32>(w(0x2b7), w(0x2bb));
                g.write::<u32>(w(0x2c1), w(0x2c5));
                g.write::<u32>(w(0x2ca), last);
                if last == w(0x2cf) {
                    g.write::<u32>(w(0x2d7), w(0x2db));
                } else {
                    g.write::<u32>(last + 4, w(0x2e4));
                }
                let c = g.read::<u16>(w(0x2f0)).wrapping_add(1);
                g.write::<u16>(w(0x2f0), c);
                g.write::<u32>(w(0x2f8), w(0x2fc));
                // call width: 3
                let p = g.call(slot(g, 0x2e9, 719), 0, &[w(0x309), 6, 0]).eax;
                g.write::<u32>(w(0x310), p);
                g.write::<u32>(w(0x316), w(0x31a));
                g.write::<u32>(w(0x320), n4);
                g.write::<u32>(w(0x326), 0x40);
                g.write::<u32>(w(0x330), 0x100);
                g.write::<u32>(w(0x33a), 0);
                g.write::<u32>(w(0x344), w(0x348));
                g.write::<u32>(r + 0x18, 0xa);
            }
            0xa => {
                let k = gk(g, 0x416);
                // call width: 2
                let eax = g.call(slot(g, 0x40b, 228), 0, &[k, 0xb8]).eax;
                let x = g.read::<u32>(w(0x424)) | 1;
                g.write::<u32>(w(0x424), x);
                g.write::<u32>(w(0x42b), 0);
                g.write::<u32>(r + 0x18, 0xb);
                return done(g, eax);
            }
            0xb => {
                if says(g, 0x444, 0x44a) == 0 {
                    return done(g, 0);
                }
                let k = gk(g, 0x46d);
                // call width: 2
                let a = g.call(slot(g, 0x462, 228), 0, &[k, 0xb9]).eax;
                s701(g, 0x4ae, 0x4b6, 2, a);
                let list = w(0x4c6);
                crate::sb_misc::node_append(g, img, call(0x4d0), w(0x4cb), list, 0);
                let k = gk(g, 0x4e0);
                // call width: 2
                g.call(slot(g, 0x4d5, 531), 0, &[k, w(0x4da)]);
                g.write::<u32>(w(0x4fb), 0x3f80_0000);
                let node = w(0x4f5);
                crate::sb_misc::node_append(g, img, call(0x504), node, w(0x4f0), 0);
                g.write::<u32>(at(0x34), node);
                for op in [0x512u32, 0x51c, 0x526, 0x530] {
                    g.write::<u32>(w(op), 0x3f80_0000);
                }
                let s = at(0x20);
                for (o, v) in [
                    (0u32, w(0x53c)),
                    (4, w(0x4f0)),
                    (8, w(0x548)),
                    (0xc, w(0x550)),
                    (0x10, w(0x558)),
                ] {
                    g.write::<u32>(s + o, v);
                }
                let ins = Frame {
                    at: call(0x55d),
                    esp: z.wrapping_sub(4),
                    ecx: s,
                    ..*fr
                };
                crate::nodes::node_insert(g, img, &ins);
                let eax = particle(g, img, fr, z, (0x567, 0x56c, 0x572, 0x578), 1);
                g.write::<u32>(r + 0x18, 0xc);
                return done(g, eax);
            }
            0xc => {
                if says(g, 0x35c, 0x362) == 0 {
                    return done(g, 0);
                }
                // push ecx (unread): the bone set-up's frame
                g.write::<u32>(z.wrapping_sub(4), 0);
                let bf = Frame {
                    at: call(0x37e),
                    esp: z.wrapping_sub(8),
                    ecx: r + 0x34,
                    ..*fr
                };
                crate::o6_fam_bone::bone_node_setup(g, img, &bf);
                g.write::<u32>(r + 0x18, 0x14);
            }
            0x14 => {
                let k = gk(g, 0x598);
                // call width: 2
                let eax = g.call(slot(g, 0x58d, 228), 0, &[k, 0xba]).eax;
                g.write::<u32>(r + 0x18, 0x15);
                g.write::<u32>(r + 0x2c, 0);
                return done(g, eax);
            }
            0x15 => {
                if g.read::<i32>(r + 0x2c) < 5 {
                    return done(g, 5);
                }
                let node = w(0x5cb);
                crate::sb_misc::node_append(g, img, call(0x5d0), node, w(0x5c6), 0);
                g.write::<u32>(at(0x34), node);
                for op in [0x5deu32, 0x5e8, 0x5f2] {
                    g.write::<u32>(w(op), 0x3f80_0000);
                }
                g.write::<u32>(w(0x5fc), 0x3da3_d70a);
                let s = at(0x20);
                for (o, v) in [
                    (0u32, w(0x608)),
                    (4, w(0x5c6)),
                    (8, w(0x614)),
                    (0xc, w(0x61c)),
                    (0x10, w(0x624)),
                ] {
                    g.write::<u32>(s + o, v);
                }
                let ins = Frame {
                    at: call(0x629),
                    esp: z.wrapping_sub(4),
                    ecx: s,
                    ..*fr
                };
                crate::nodes::node_insert(g, img, &ins);
                particle(g, img, fr, z, (0x633, 0x638, 0x63e, 0x644), 0);
                let k = gk(g, 0x659);
                // call width: 3
                let eax = g.call(slot(g, 0x649, 239), 0, &[k, 0xffff_fffb, 0]).eax;
                g.write::<u32>(r + 0x18, 0x16);
                return done(g, eax);
            }
            0x16 => {
                if says(g, 0x38f, 0x395) == 0 {
                    return done(g, 0);
                }
                g.write::<u32>(r + 0x18, 0x1e);
            }
            0x1e => {
                let k = gk(g, 0x67b);
                // call width: 2
                let a = g.call(slot(g, 0x670, 228), 0, &[k, 0xbb]).eax;
                let eax = s701(g, 0x6bc, 0x6c4, 1, a);
                g.write::<u32>(r + 0x18, 0x1f);
                return done(g, eax);
            }
            0x1f => {
                if says(g, 0x6e1, 0x6e7) == 0 {
                    return done(g, 0);
                }
                let k = gk(g, 0x706);
                // call width: 2
                let eax = g.call(slot(g, 0x6fb, 228), 0, &[k, 0xbc]).eax;
                g.write::<u32>(r + 0x18, 0x20);
                return done(g, eax);
            }
            0x20 => {
                if says(g, 0x3b6, 0x3bc) == 0 {
                    return done(g, 0);
                }
                let k = gk(g, 0x3dc);
                // call width: 2
                g.call(slot(g, 0x3d4, 213), 0, &[k, 0]);
                g.write::<u32>(r + 0x18, 0x2710);
            }
            0x2710 => {
                if g.read::<u8>(w(0x724)) & 4 == 0 {
                    return done(g, 0x2710);
                }
                let x = g.read::<u32>(w(0x732)) | 1;
                g.write::<u32>(w(0x732), x);
                let k = gk(g, 0x741);
                // call width: 2
                g.call(slot(g, 0x72c, 213), 0, &[k, 0]);
                let node = g.read::<u32>(task);
                let (prev, next) = (g.read::<u32>(node + 4), g.read::<u32>(node + 8));
                g.write::<u32>(prev + 8, next);
                let next = g.read::<u32>(node + 8);
                let prev = g.read::<u32>(node + 4);
                g.write::<u32>(next + 4, prev);
                g.write::<u32>(node + 8, node);
                g.write::<u32>(node + 4, node);
                return done(g, prev);
            }
            s if s > 0x20 => return done(g, s),
            _ => return done(g, 10),
        }
    }
}

/// The particle node of [`summon_task_0272`]: `push i; sub esp, 8; push
/// N` with `ecx` = the part (operands `(part, N, flag, call)`), the
/// flag byte 1 first. Returns its `eax`.
fn particle<G: Game>(
    g: &mut G,
    img: &Image,
    fr: &Frame,
    z: u32,
    ops: (u32, u32, u32, u32),
    i: u32,
) -> u32 {
    let w = |o: u32| img.word(fr.at + o);
    let (part, n, flag, at) = ops;
    g.write::<u32>(z.wrapping_sub(4), i);
    g.write::<u32>(z.wrapping_sub(8), 0);
    g.write::<u32>(z.wrapping_sub(0xc), 0);
    g.write::<u32>(z.wrapping_sub(0x10), w(n));
    g.write::<u8>(w(flag), 1);
    let pf = Frame {
        at: img.rel32_target(fr.at + at),
        esp: z.wrapping_sub(0x14),
        ecx: w(part),
        ..*fr
    };
    crate::s3_nodes::particle_node_with(g, img, &pf)
}

/// [`summon_task_0272`] (the `rel32`s: its node set-ups; the state
/// table and the re-dispatch jump stay inside).
pub const SUMMON_TASK_0272: Sig = Sig {
    len: 1985,
    hash: 0xed8c_88ae_725f_f674,
    calls: &[
        (0x81, Callee::Sig(&crate::s4_small::SUMMON_NODES)),
        (0xde, Callee::Sig(&crate::m0070::SHOW)),
        (0x213, Callee::Sig(&crate::sa_summon::VECTOR_OP)),
        (0x37e, Callee::Sig(&crate::r24::BONE_NODE_SETUP_0272)),
        (0x4d0, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x504, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x55d, Callee::Sig(&crate::nodes::INSERT)),
        (0x578, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x5d0, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x629, Callee::Sig(&crate::nodes::INSERT)),
        (0x644, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
    ],
    imports: &[],
};

/// `magic_0341`'s per-frame effect (0x100014d0, 2283 bytes; no arguments,
/// an `ebp` /GS frame with the points `L1` at `ebp - 0x14` and `L2` at
/// `ebp - 0x24`). Off while `*G_ON` is 0. Every part refresh
/// ([`part_refresh_b`](crate::sa_misc::part_refresh_b)) runs with the
/// word at `*G_FLAG` 1, cleared after; a phase counter goes up only while
/// the word at `*G_DONE` is 0.
/// - Not loaded yet: slot 711 `(NAME, X, 0)` with the flag word 1 (put
///   back after), slot 710 `(OUT, NAME, *G_BUF, 0x80000)`.
/// - Then: `*G_DONE` = slot 253 is 0; slots 723 `(*G_A)`, 724 `(*G_B)`;
///   `**G_C = *G_BUF + 0x80400`. By slot 261 `(4)`, `(3)`, `(2)`, `(1)`
///   (the first true one):
///   - 4: once, `**G_D = OUT`, the part `P0` started
///     ([`start_part`](crate::opu_parts::start_part) with slot 941's
///     program), three constant rows copied; `P0`'s point = the middle of
///     slot 237's two bones (0x13, 0x23) times `K` plus `G_OFF`; `P0` and
///     `P1` refreshed;
///   - 3: once, `P2`'s motion ([`part_motion`](crate::s6_misc::part_motion),
///     program 5); the actor's points (slot 237) plus their offsets, slot
///     247 `(M, -*G_R)`, three slot 359 scales `K / G_S[i]` into the
///     negated vector; slot 160 `(P2, G_X, 0)`; three parts refreshed;
///   - 2: at 0, three motions (2, 3, 4) and slot 262; from 20, 70 and 80
///     on, a part refreshed (two with the point moved by its offset); the
///     last part refreshed;
///   - 1: once, the last part's motion (program 0); refreshed.
/// - At the end: slot 214 `(*G_E2)` when the first counter is 50; the
///   load counter up.
///
/// Returns 0.
#[allow(clippy::too_many_lines)] // the original's phases, in order
pub fn effect_frame_0341<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, div, mul};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let e = fr.esp;
    let ebp = e.wrapping_sub(4);
    let l = |o: u32| ebp.wrapping_sub(o);
    if g.read::<u32>(w(0x12)) == 0 {
        return Ret::eax(0);
    }
    let set_flag = |g: &mut G, op: u32, v: u32| {
        let p = g.read::<u32>(w(op));
        g.write::<u32>(p, v);
    };
    let refresh = |g: &mut G, at: u32, part_op: u32| {
        crate::sa_misc::part_refresh_b(g, img, call(at), w(part_op), 0);
    };
    let done_up = |g: &mut G, done_op: u32, counter_op: u32| {
        let p = g.read::<u32>(w(done_op));
        if g.read::<u32>(p) == 0 {
            let c = g.read::<u32>(w(counter_op)).wrapping_add(1);
            g.write::<u32>(w(counter_op), c);
        }
    };
    // part_motion: `ecx` = the part, `edx` = the program, `xmm2`/`xmm3`,
    // seven stack words (`sub esp, 0x1c` below the frame)
    let motion =
        |g: &mut G, at: u32, part: u32, index: u32, (x2, x3): (u32, u32), words: [u32; 7]| {
            let esp = e.wrapping_sub(0x4c);
            for (i, v) in (0u32..).zip(words) {
                g.write::<u32>(esp + 4 + 4 * i, v);
            }
            let fr = Frame {
                at: call(at),
                esp,
                eax: 0,
                ecx: part,
                edx: index,
                xmm: [0, 0, u128::from(x2), u128::from(x3)],
            };
            crate::s6_misc::part_motion(g, img, &fr);
        };
    // a point of slot 237 `(*G, 0x23, 0, at)` moved by the offsets `by`
    let point = |g: &mut G, (t, gop, at_op): (u32, u32, u32), adds: [(u32, u32, u32); 3]| {
        let x = g.read::<u32>(w(gop));
        // call width: 4
        g.call(slot(g, t, 237), 0, &[x, 0x23, 0, w(at_op)]);
        for (a, b, out) in adds {
            let v = add(g.read::<f32>(w(a)), g.read::<f32>(w(b)));
            g.write::<f32>(w(out), v);
        }
    };
    if g.read::<u32>(w(0x1f)) == 0 {
        let p = g.read::<u32>(w(0x27));
        let saved = g.read::<u32>(p);
        g.write::<u32>(p, 1);
        // call width: 3
        g.call(slot(g, 0x35, 711), 0, &[w(0x47), w(0x42), 0]);
        let buf = g.read::<u32>(w(0x59));
        let p = g.read::<u32>(w(0x4e));
        g.write::<u32>(p, saved);
        // call width: 4
        g.call(slot(g, 0x60, 710), 0, &[w(0x70), w(0x65), buf, 0x8_0000]);
    }
    // call width: 0
    let loaded = g.call(slot(g, 0x7b, 253), 0, &[]).eax;
    let a = g.read::<u32>(w(0x8f));
    let done = g.read::<u32>(w(0x89));
    g.write::<u32>(done, u32::from(loaded == 0));
    // call width: 1
    g.call(slot(g, 0x9b, 723), 0, &[a]);
    let b = g.read::<u32>(w(0xae));
    // call width: 1
    g.call(slot(g, 0xa8, 724), 0, &[b]);
    let c = g.read::<u32>(w(0xbc)).wrapping_add(0x8_0400);
    let cp = g.read::<u32>(w(0xc1));
    g.write::<u32>(cp, c);
    // call width: 1
    if g.call(slot(g, 0xce, 261), 0, &[4]).eax != 0 {
        if g.read::<u32>(w(0xe9)) == 0 && g.read::<u32>(w(0xf2)) == 0 {
            let dp = g.read::<u32>(w(0xfa));
            g.write::<u32>(dp, w(0x100));
            let x = g.read::<u32>(w(0x10b));
            // call width: 1
            let table = g.call(slot(g, 0x105, 941), 0, &[x]).eax;
            let program = g.read::<u32>(table + 4);
            crate::opu_parts::start_part(g, img, call(0x129), w(0x124), 0x1000, program, 1);
            for (src, dst) in [(0x130u32, 0x137u32), (0x13f, 0x14c)] {
                let v: Vec<u8> = (0..16).map(|i| g.read::<u8>(w(src) + i)).collect();
                g.write_bytes(w(dst), &v);
            }
            g.write::<u16>(w(0x15c), 1);
            let v: Vec<u8> = (0..16).map(|i| g.read::<u8>(w(0x153) + i)).collect();
            g.write_bytes(w(0x163), &v);
            g.write::<u32>(w(0x168), 1);
        }
        let (l1, l2) = (l(0x14), l(0x24));
        let x = g.read::<u32>(w(0x181));
        // call width: 4
        g.call(slot(g, 0x171, 237), 0, &[x, 0x13, 0, l1]);
        let x = g.read::<u32>(w(0x19c));
        // call width: 4
        g.call(slot(g, 0x18c, 237), 0, &[x, 0x23, 0, l2]);
        let kk = k(g, 0x1ba);
        let mx = mul(add(g.read::<f32>(l2), g.read::<f32>(l1)), kk);
        let my = mul(add(g.read::<f32>(l1 + 4), g.read::<f32>(l2 + 4)), kk);
        let mz = mul(add(g.read::<f32>(l1 + 8), g.read::<f32>(l2 + 8)), kk);
        g.write::<f32>(w(0x1f1), add(k(g, 0x1dd), mx));
        g.write::<f32>(w(0x205), add(k(g, 0x1f9), my));
        g.write::<f32>(w(0x219), add(k(g, 0x20d), mz));
        set_flag(g, 0x1cd, 1);
        refresh(g, 0x224, 0x1e9);
        refresh(g, 0x22e, 0x229);
        set_flag(g, 0x233, 0);
        done_up(g, 0x23e, 0x24d);
    // call width: 1
    } else if g.call(slot(g, 0x257, 261), 0, &[3]).eax != 0 {
        if g.read::<u32>(w(0x272)) == 0 && g.read::<u32>(w(0x27b)) == 0 {
            let s = 0x3ee6_6666;
            motion(g, 0x2cd, w(0x2c1), 5, (0, 0), [0, 0, 0, 0, s, s, s]);
            g.write::<u32>(w(0x2d6), w(0x2da));
            g.write::<u32>(w(0x2e0), 1);
        }
        point(
            g,
            (0x2e9, 0x2fe, 0x2ee),
            [
                (0x308, 0x310, 0x32c),
                (0x334, 0x33c, 0x344),
                (0x34c, 0x354, 0x35c),
            ],
        );
        point(
            g,
            (0x315, 0x324, 0x31a),
            [
                (0x36c, 0x374, 0x389),
                (0x391, 0x399, 0x3a6),
                (0x3ae, 0x3b6, 0x3be),
            ],
        );
        let r = g.read::<u32>(w(0x379)).wrapping_neg();
        // call width: 2
        g.call(slot(g, 0x381, 247), 0, &[w(0x39e), r]);
        let q_at = l(0x28);
        for (kop, dop, t, m1, m2, vop, mask, out) in [
            (
                0x3ceu32, 0x3d6u32, 0x3dbu32, 0x3f9u32, 0x3e9u32, 0x403u32, 0x417u32, 0x41fu32,
            ),
            (0x427, 0x42f, 0x40d, 0x44a, 0x43a, 0x454, 0x468, 0x470),
            (0x478, 0x480, 0x45e, 0x49b, 0x48b, 0x4a5, 0x4b8, 0x4c0),
        ] {
            let q = div(k(g, kop), k(g, dop));
            g.write::<f32>(q_at, q);
            // call width: 3
            g.call(slot(g, t, 359), 0, &[w(m1), q.to_bits(), w(m2)]);
            let v = mul(k(g, vop), g.read::<f32>(q_at));
            g.write::<u32>(w(out), v.to_bits() ^ g.read::<u32>(w(mask)));
        }
        set_flag(g, 0x4af, 1);
        // call width: 3
        g.call(slot(g, 0x4cb, 160), 0, &[w(0x4db), w(0x4d0), 0]);
        refresh(g, 0x4ea, 0x4e5);
        refresh(g, 0x4f4, 0x4ef);
        refresh(g, 0x4fe, 0x4f9);
        set_flag(g, 0x503, 0);
        done_up(g, 0x50e, 0x51d);
    // call width: 1
    } else if g.call(slot(g, 0x527, 261), 0, &[2]).eax != 0 {
        let mut u = g.read::<u32>(w(0x541));
        let fresh = u == 0;
        if fresh && g.read::<u32>(w(0x54f)) != 0 {
            // straight to the last part
        } else {
            if fresh {
                let s = 0x3cf5_c28f;
                motion(g, 0x592, w(0x58a), 2, (0, 0), [0, 0, 0, 0, s, s, s]);
                let x = g.read::<u32>(w(0x5b2));
                // call width: 4
                g.call(slot(g, 0x597, 262), 0, &[x, w(0x5ac), w(0x5a7), w(0x59c)]);
                motion(g, 0x603, w(0x5f7), 3, (0, 0), [0, 0, 0, 0, s, s, s]);
                let s = 0x3d1b_a5e3;
                motion(g, 0x64f, w(0x643), 4, (0, 0), [0, 0, 0, 0, s, s, s]);
                u = g.read::<u32>(w(0x654));
                g.write::<u32>(w(0x65d), 1);
            }
            if u.cast_signed() >= 0x14 {
                set_flag(g, 0x66b, 1);
                refresh(g, 0x67b, 0x670);
                set_flag(g, 0x680, 0);
                u = g.read::<u32>(w(0x68b));
            }
            if u.cast_signed() >= 0x46 {
                let x = g.read::<u32>(w(0x6ae));
                // call width: 4
                g.call(slot(g, 0x699, 237), 0, &[x, 0x23, 0, w(0x69e)]);
                for (a, b, out) in [
                    (0x6b8u32, 0x6c0u32, 0x6d5u32),
                    (0x6dd, 0x6e5, 0x6ed),
                    (0x6f5, 0x6fd, 0x705),
                ] {
                    let v = add(k(g, a), k(g, b));
                    g.write::<f32>(w(out), v);
                }
                set_flag(g, 0x6c5, 1);
                refresh(g, 0x710, 0x6cd);
                set_flag(g, 0x715, 0);
                u = g.read::<u32>(w(0x720));
            }
            if u.cast_signed() >= 0x50 {
                let x = g.read::<u32>(w(0x743));
                // call width: 4
                g.call(slot(g, 0x72e, 237), 0, &[x, 0x23, 0, w(0x733)]);
                for (a, b, out) in [
                    (0x74du32, 0x755u32, 0x76au32),
                    (0x772, 0x77a, 0x782),
                    (0x78a, 0x792, 0x79a),
                ] {
                    let v = add(k(g, a), k(g, b));
                    g.write::<f32>(w(out), v);
                }
                set_flag(g, 0x75a, 1);
                refresh(g, 0x7a5, 0x762);
                set_flag(g, 0x7aa, 0);
            }
        }
        set_flag(g, 0x7b5, 1);
        refresh(g, 0x7c5, 0x7ba);
        set_flag(g, 0x7ca, 0);
        done_up(g, 0x7d5, 0x7e4);
    // call width: 1
    } else if g.call(slot(g, 0x7ee, 261), 0, &[1]).eax != 0 {
        if g.read::<u32>(w(0x809)) == 0 && g.read::<u32>(w(0x812)) == 0 {
            let s = 0x3d95_8106;
            let (x3, x2) = (g.read::<u32>(w(0x81d)), g.read::<u32>(w(0x825)));
            motion(
                g,
                0x86b,
                w(0x85f),
                0,
                (x2, x3),
                [0xc261_e0ee, 0, 0, 0, s, s, s],
            );
            g.write::<u32>(w(0x874), 1);
        }
        set_flag(g, 0x87d, 1);
        refresh(g, 0x88d, 0x882);
        set_flag(g, 0x892, 0);
        done_up(g, 0x89d, 0x8a8);
    }
    if g.read::<u32>(w(0x8ae)) == 0x32 {
        let x = g.read::<u32>(w(0x8bc));
        // call width: 1
        g.call(slot(g, 0x8b6, 214), 0, &[x]);
    }
    done_up(g, 0x8cc, 0x8d7);
    Ret::eax(0)
}

/// [`effect_frame_0341`] (the `rel32`s: the part start, five part motions,
/// ten part refreshes, `__security_check_cookie`; the three jumps to the
/// tail stay inside).
pub const EFFECT_FRAME_0341: Sig = Sig {
    len: 2283,
    hash: 0xb692_3047_a530_b5c7,
    calls: &[
        (0x129, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x224, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0x22e, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0x2cd, Callee::Sig(&crate::s6_misc::PART_MOTION)),
        (0x4ea, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0x4f4, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0x4fe, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0x592, Callee::Sig(&crate::s6_misc::PART_MOTION)),
        (0x603, Callee::Sig(&crate::s6_misc::PART_MOTION)),
        (0x64f, Callee::Sig(&crate::s6_misc::PART_MOTION)),
        (0x67b, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0x710, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0x7a5, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0x7c5, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0x86b, Callee::Sig(&crate::s6_misc::PART_MOTION)),
        (0x88d, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0x8e3, Callee::Any),
    ],
    imports: &[],
};
