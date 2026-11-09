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
        let tex = g.read::<u32>(g.read::<u32>(w(0x3d)) + 0x24);
        g.write::<u32>(l(0x58), tex);
        let mut r = g.read::<u32>(w(0x47)) + 0x2ec0;
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
