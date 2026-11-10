//! Round 38's own ports (own rows of `magic_0250` .. `magic_0449`): the
//! DLLs' own code written out.

use crate::m0018::{st0_f32, wrap};
use crate::runtime::slot_target;
use crate::sb_sse::{add, div, mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

const ONE: u32 = 0x3f80_0000;

/// `magic_0316`'s spark tick (0x100014b0, 448 bytes; a task callback,
/// cdecl `(node)`; an `ebp` /GS frame, the caster's point `P` at `ebp -
/// 0x14`). The node's count `+0x14` steps down; at 0 or below the node
/// goes back to the pool ([`give_back`](crate::sa_pool::give_back)).
/// On an even count one record of the DLL's pool
/// ([`pool_take`](crate::sb_misc::pool_take)) gets: `P` = slot 262 of the
/// caster `*G_A` with `y` its height (slot 12, ST0) and `w` 1.0; `+0x30`
/// slot 719 `(D, 9, 0)`, the halfwords `+0x38` -1 and `+0x3a` 0x200,
/// `+0x34` 1; the place `(x + rnd 35, y - slot 404 (50) - K, z + rnd 35,
/// 1)` (slot 314 random floats); `+0x10`..`+0x18`, `+0x20`..`+0x28` 0,
/// `+0x2c` 1.0, `+0x1c` rnd pi, `+0x3c` `(slot 404 (1) + K1) * K2`,
/// `+0x4c` -1, `+0x50` 0.
///
/// Returns the last slot's `eax`, 0 with the pool empty, and with an odd
/// count the cookie word (`*G_COOKIE ^ ebp`) or the give-back's answer.
pub fn spark_tick_0316<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x51), n);
    let ebp = fr.esp.wrapping_sub(4);
    let mut eax = g.read::<u32>(w(0x07)) ^ ebp;
    let node = fr.arg(g, 0);
    let n = g.read::<i32>(node + 0x14).wrapping_sub(1);
    g.write::<i32>(node + 0x14, n);
    if n <= 0 {
        eax = crate::sa_pool::give_back(g, img, img.rel32_target(f + 0x20), node);
    }
    if g.read::<u8>(node + 0x14) & 1 != 0 {
        return Ret::eax(eax);
    }
    let rec = crate::sb_misc::pool_take(g, img, img.rel32_target(f + 0x30));
    if rec == 0 {
        return Ret::eax(0);
    }
    let chr = g.read::<u32>(w(0x40));
    let p = ebp.wrapping_sub(0x14);
    // call width: 4
    g.call(slot(g, 262), 0, &[chr, p, p + 4, p + 8]);
    // call width: 1
    let y = st0_f32(g.call(slot(g, 12), 0, &[chr]).st0);
    g.write::<f32>(p + 4, y);
    g.write::<u32>(p + 0xc, ONE);
    // call width: 3
    let h = g.call(slot(g, 719), 0, &[w(0x83), 9, 0]).eax;
    g.write::<u32>(rec + 0x30, h);
    g.write::<u16>(rec + 0x38, 0xffff);
    g.write::<u16>(rec + 0x3a, 0x200);
    g.write::<u32>(rec + 0x34, 1);
    let mut rnd = |g: &mut G, n: u32, x: u32| {
        // call width: 1
        let out = g.call(slot(g, n), 0, &[x]);
        eax = out.eax;
        st0_f32(out.st0)
    };
    let r = rnd(g, 314, 0x420c_0000);
    g.write::<f32>(rec, add(g.read::<f32>(p), r));
    let r = rnd(g, 404, 0x4248_0000);
    let y = sub(sub(g.read::<f32>(p + 4), r), k(g, 0xfc));
    g.write::<f32>(rec + 4, y);
    let r = rnd(g, 314, 0x420c_0000);
    let z = add(g.read::<f32>(p + 8), r);
    g.write::<u32>(rec + 0xc, ONE);
    g.write::<u32>(rec + 0x20, 0);
    g.write::<u32>(rec + 0x24, 0);
    g.write::<f32>(rec + 8, z);
    g.write::<u32>(rec + 0x28, 0);
    g.write::<u32>(rec + 0x2c, ONE);
    for o in [0x10u32, 0x14, 0x18] {
        g.write::<u32>(rec + o, 0);
    }
    let r = rnd(g, 314, 0x4049_0fdb);
    g.write::<f32>(rec + 0x1c, r);
    let r = rnd(g, 404, ONE);
    g.write::<u32>(rec + 0x4c, u32::MAX);
    g.write::<u32>(rec + 0x50, 0);
    g.write::<f32>(rec + 0x3c, mul(add(r, k(g, 0x18e)), k(g, 0x1a7)));
    Ret::eax(eax)
}

/// [`spark_tick_0316`] (448 bytes, 1 overlay).
pub const SPARK_TICK_0316: Sig = Sig {
    len: 448,
    hash: 0x2045_fa0a_911c_2e48,
    calls: &[
        (0x20, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x30, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
        // __security_check_cookie
        (0x1b8, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0316`'s spark ring (0x10001800, 980 bytes; no arguments; an
/// `ebp` /GS frame: the yaw `ebp - 0xa8`, the caster's point `P` `ebp -
/// 0x24`, the vector `V` `ebp - 0x14`, the matrices `M` `ebp - 0x64` and
/// `N` `ebp - 0xa4`). The yaw `K - slot 333 (*G_A)` wrapped once; `P` =
/// slot 262 of the caster with `y` its height (slot 12) and `w` 1.0; `M`
/// the unit (slot 235) turned by the yaw (slot 329), `V = M (8, 0, 20,
/// 0)` (slot 229) added to `P`; `N` the unit with `N[10]` 1.2 turned by
/// the yaw. Then 28 records of the DLL's pool
/// ([`pool_take`](crate::sb_misc::pool_take); the first empty take ends
/// it), each: `V = (0, 0, slot 404 (30) + K, 0)`, `M` the unit turned
/// by rnd pi (slots 314, 329), `M = N M` (slot 231), `V = M V`; `s` = slot
/// 404 (0.02); `+0x30` slot 719 `(D, 9, 1)`, `+0x34` 1, the halfwords
/// `+0x38` -1 and `+0x3a` 0x200; the place `(P.x + V.x, P.y - K', P.z +
/// V.z, 1)`, `+0x20` 0, `+0x24` `-slot 404 (0.02)`, `+0x28` 0, `+0x2c`
/// 1.0, `+0x10` `V.x s`, `+0x14` 0, `+0x18` `V.z s`, `+0x1c` rnd pi,
/// `+0x4c`/`+0x50` 0x20, the bytes `+0x67` 0x60 and `+0x6b` 0, `+0x3c`
/// `(slot 404 (0.5) + K1) * K2`.
///
/// Returns the last slot 404's `eax`, or 0 when the pool ran empty.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn spark_ring_0316<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x14), n);
    let ebp = fr.esp.wrapping_sub(4);
    let at = |o: u32| ebp.wrapping_sub(o);
    let (p, v, m, nm) = (at(0x24), at(0x14), at(0x64), at(0xa4));
    let a = g.read::<u32>(w(0x1a));
    // call width: 1
    let t = st0_f32(g.call(slot(g, 333), 0, &[a]).st0);
    let yaw = wrap(sub(k(g, 0x30), t), k(g, 0x42), k(g, 0x5e), k(g, 0x54));
    let chr = g.read::<u32>(w(0x7a));
    // call width: 4
    g.call(slot(g, 262), 0, &[chr, p, p + 4, p + 8]);
    // call width: 1
    let y = st0_f32(g.call(slot(g, 12), 0, &[chr]).st0);
    g.write::<f32>(p + 4, y);
    g.write::<u32>(p + 0xc, ONE);
    // call width: 1
    g.call(slot(g, 235), 0, &[m]);
    // call width: 3
    g.call(slot(g, 329), 0, &[m, m, yaw.to_bits()]);
    for (i, x) in (0u32..).zip([0x4100_0000u32, 0, 0x41a0_0000, 0]) {
        g.write::<u32>(v + 4 * i, x);
    }
    // call width: 3
    g.call(slot(g, 229), 0, &[v, m, v]);
    let rf = |g: &G, at: u32| g.read::<f32>(at);
    let x = add(rf(g, p), rf(g, v));
    g.write::<f32>(p, x);
    let y = add(rf(g, v + 4), rf(g, p + 4));
    g.write::<f32>(p + 4, y);
    let z = add(rf(g, p + 8), rf(g, v + 8));
    g.write::<f32>(p + 8, z);
    // call width: 1
    g.call(slot(g, 235), 0, &[nm]);
    g.write::<u32>(nm + 0x28, 0x3f99_999a);
    // call width: 3
    g.call(slot(g, 329), 0, &[nm, nm, yaw.to_bits()]);
    let take = img.rel32_target(f + 0x189);
    let mut eax = 0;
    for _ in 0..0x1c {
        let rec = crate::sb_misc::pool_take(g, img, take);
        if rec == 0 {
            return Ret::eax(0);
        }
        g.write::<u32>(v, 0);
        g.write::<u32>(v + 4, 0);
        // call width: 1
        let r = st0_f32(g.call(slot(g, 404), 0, &[0x41f0_0000]).st0);
        g.write::<u32>(v + 0xc, 0);
        g.write::<f32>(v + 8, add(r, k(g, 0x1cd)));
        // call width: 1
        g.call(slot(g, 235), 0, &[m]);
        // call width: 1
        let r = st0_f32(g.call(slot(g, 314), 0, &[0x4049_0fdb]).st0);
        // call width: 3
        g.call(slot(g, 329), 0, &[m, m, r.to_bits()]);
        // call width: 3
        g.call(slot(g, 231), 0, &[m, nm, m]);
        // call width: 3
        g.call(slot(g, 229), 0, &[v, m, v]);
        // call width: 1
        let s = st0_f32(g.call(slot(g, 404), 0, &[0x3ca3_d70a]).st0);
        // call width: 3
        let h = g.call(slot(g, 719), 0, &[w(0x28c), 9, 1]).eax;
        g.write::<u32>(rec + 0x30, h);
        g.write::<u32>(rec + 0x34, 1);
        g.write::<u16>(rec + 0x38, 0xffff);
        g.write::<u16>(rec + 0x3a, 0x200);
        g.write::<f32>(rec, add(rf(g, p), rf(g, v)));
        g.write::<f32>(rec + 4, sub(rf(g, p + 4), k(g, 0x2d2)));
        let z = add(rf(g, p + 8), rf(g, v + 8));
        g.write::<u32>(rec + 0xc, ONE);
        g.write::<u32>(rec + 0x20, 0);
        g.write::<f32>(rec + 8, z);
        // call width: 1
        let u = st0_f32(g.call(slot(g, 404), 0, &[0x3ca3_d70a]).st0);
        let sign = g.read::<u32>(w(0x311));
        g.write::<u32>(rec + 0x24, u.to_bits() ^ sign);
        g.write::<u32>(rec + 0x28, 0);
        g.write::<u32>(rec + 0x2c, ONE);
        g.write::<u32>(rec + 0x14, 0);
        g.write::<f32>(rec + 0x10, mul(rf(g, v), s));
        g.write::<f32>(rec + 0x18, mul(rf(g, v + 8), s));
        // call width: 1
        let r = st0_f32(g.call(slot(g, 314), 0, &[0x4049_0fdb]).st0);
        g.write::<f32>(rec + 0x1c, r);
        // call width: 1
        let out = g.call(slot(g, 404), 0, &[0x3f00_0000]);
        eax = out.eax;
        let q = st0_f32(out.st0);
        g.write::<u32>(rec + 0x4c, 0x20);
        g.write::<u32>(rec + 0x50, 0x20);
        g.write::<u8>(rec + 0x67, 0x60);
        g.write::<u8>(rec + 0x6b, 0);
        g.write::<f32>(rec + 0x3c, mul(add(q, k(g, 0x390)), k(g, 0x3aa)));
    }
    Ret::eax(eax)
}

/// [`spark_ring_0316`] (980 bytes, 1 overlay).
pub const SPARK_RING_0316: Sig = Sig {
    len: 980,
    hash: 0x93d6_cff4_8d20_dd67,
    calls: &[
        (0x189, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
        // __security_check_cookie
        (0x3cc, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0316`'s start-up (0x10001bf0, 1278 bytes; internal, no
/// arguments, run by its load step [`load_0316`]; an `ebp` frame with
/// the yaw at `ebp - 4`):
/// - three globals set to their immediates, a count 0x14 and a word 0;
///   the actors looked up ([`lookup_actors`](crate::sb_misc::lookup_actors));
///   `*G_H` = slot 719 `(D, 8, 0)`; the part set up
///   ([`part_init`](crate::l5_objhost::part_init));
/// - the yaw `K - slot 333 (*G_A)` wrapped once; object 1 started
///   ([`start_object_owners`](crate::s4_misc::start_object_owners), scale
///   `K1`, `(1, 0xb9, 0x78, CB1)`);
/// - the point `P` (three globals) from slot 262 of the caster, `P.y`
///   its height (slot 12) less `K2`, `P.w` 1.0, the yaw in a global, a
///   record of nine globals; object 0 started (scale `K2`, `(0, 0x131,
///   0x3c, CB2)`);
/// - the point `Q` from slot 262, `Q.w` 1.0, `Q.y` less `K3`, the yaw
///   plus `K4` wrapped once in a global, another record; three words 1.0;
///   slot 790's float in a global;
/// - two fades ([`fade_task`](crate::s3_misc::fade_task)), slot 271
///   `(4)`, two handles (slot 719 `(D, 6, 0 / 1)`) through slot 220 `(h,
///   0x40, 0x100)`;
/// - the actor record: `*G_N` stored, slot 215 or (with `*G_F` set) 413
///   of it, the side word 0 or 1;
/// - a pool node ([`take`](crate::sa_pool::take)) for the step task
///   appended to the task list, the records set up
///   ([`records_init`](crate::sa_misc::records_init)), nodes for the
///   spark tick ([`spark_tick_0316`], count 0xa0) and the frame task
///   appended after it.
///
/// Returns 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn start_up_0316<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x3c), n);
    let ebp = fr.esp.wrapping_sub(4);
    g.write::<u32>(w(0x06), w(0x0a));
    g.write::<u32>(w(0x10), w(0x14));
    g.write::<u32>(w(0x1a), w(0x1e));
    g.write::<u32>(w(0x24), 0x14);
    g.write::<u32>(w(0x2e), 0);
    crate::sb_misc::lookup_actors(g, img, img.rel32_target(f + 0x37));
    // call width: 3
    let h = g.call(slot(g, 719), 0, &[w(0x4b), 8, 0]).eax;
    g.write::<u32>(w(0x55), h);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x5a));
    let a = g.read::<u32>(w(0x65));
    // call width: 1
    let t = st0_f32(g.call(slot(g, 333), 0, &[a]).st0);
    let yaw = wrap(sub(k(g, 0x78), t), k(g, 0x8f), k(g, 0xa4), k(g, 0x85));
    let start = |g: &mut G, call: u32, obj: u32, scale: f32, words: [u32; 4]| {
        let [index, owner, a, b] = words;
        let at = img.rel32_target(f + call);
        crate::s4_misc::start_object_owners(g, img, at, w(obj), scale, index, owner, (a, b));
    };
    start(g, 0xd3, 0xce, k(g, 0xba), [1, 0xb9, 0x78, w(0xc0)]);
    let chr = g.read::<u32>(w(0xde));
    // call width: 4
    g.call(slot(g, 262), 0, &[chr, w(0xf3), w(0xee), w(0xe9)]);
    // call width: 1
    let y = st0_f32(g.call(slot(g, 12), 0, &[chr]).st0);
    g.write::<f32>(w(0x107), y);
    let k2 = k(g, 0x114);
    g.write::<f32>(w(0x11c), yaw);
    let y = sub(g.read::<f32>(w(0x124)), k(g, 0x12c));
    g.write::<u32>(w(0x146), ONE);
    g.write::<f32>(w(0x152), y);
    for o in [0x158u32, 0x162, 0x16c, 0x176] {
        g.write::<u32>(w(o), 0);
    }
    g.write::<u32>(w(0x180), w(0x184));
    g.write::<u32>(w(0x18a), w(0x18e));
    g.write::<u32>(w(0x194), 0);
    g.write::<u32>(w(0x19e), 0);
    g.write::<u16>(w(0x1a8), 0);
    g.write::<u32>(w(0x1ae), w(0x1b2));
    start(g, 0x1b7, 0x140, k2, [0, 0x131, 0x3c, w(0x131)]);
    let chr = g.read::<u32>(w(0x1d7));
    // call width: 4
    g.call(slot(g, 262), 0, &[chr, w(0x1d1), w(0x1cc), w(0x1c1)]);
    let yaw2 = wrap(add(yaw, k(g, 0x1e6)), k(g, 0x1fa), k(g, 0x20e), k(g, 0x204));
    g.write::<u32>(w(0x1ef), ONE);
    g.write::<f32>(w(0x223), yaw2);
    let y = sub(g.read::<f32>(w(0x22b)), k(g, 0x233));
    g.write::<u16>(w(0x23b), 0);
    g.write::<u32>(w(0x246), 0);
    g.write::<f32>(w(0x252), y);
    for o in [0x258u32, 0x262, 0x26c] {
        g.write::<u32>(w(o), 0);
    }
    g.write::<u32>(w(0x276), w(0x27a));
    g.write::<u32>(w(0x280), w(0x284));
    g.write::<u32>(w(0x28a), 0);
    g.write::<u32>(w(0x294), 0);
    g.write::<u32>(w(0x29e), w(0x2a2));
    g.write::<u32>(w(0x2ae), w(0x2b2));
    for o in [0x2b8u32, 0x2c2, 0x2cc] {
        g.write::<u32>(w(o), ONE);
    }
    // call width: 0
    let v = st0_f32(g.call(slot(g, 790), 0, &[]).st0);
    g.write::<f32>(w(0x2d8), v);
    // the fades: `push ecx` and `push esi` below `ebp`, then each word
    for (i, (call, ecx, edx, x1, x2, word)) in (0u32..).zip([
        (0x2f9u32, w(0x2f4), 0x40u32, 0x2e8u32, 0x2e0u32, 0u32),
        (0x31d, w(0x318), 0x34, 0x309, 0x301, 0x1c7),
    ]) {
        let sp = ebp.wrapping_sub(0xc + 4 * i);
        g.write::<u32>(sp, word);
        let fade = Frame {
            at: img.rel32_target(f + call),
            esp: sp.wrapping_sub(4),
            eax: 0,
            ecx,
            edx,
            xmm: [
                0,
                crate::Xmm::f32(k(g, x1)).reg(),
                crate::Xmm::f32(k(g, x2)).reg(),
                0,
            ],
        };
        crate::s3_misc::fade_task(g, img, &fade);
    }
    // call width: 1
    g.call(slot(g, 271), 0, &[4]);
    for (op, side) in [(0x340u32, 0u32), (0x36b, 1)] {
        // call width: 3
        let h = g.call(slot(g, 719), 0, &[w(op), 6, side]).eax;
        // call width: 3
        g.call(slot(g, 220), 0, &[h, 0x40, 0x100]);
    }
    let n = g.read::<u32>(w(0x387));
    g.write::<u32>(w(0x396), n);
    g.write::<u32>(w(0x3a2), w(0x3a6));
    g.write::<u32>(w(0x3ac), 0);
    g.write::<u16>(w(0x3b7), 0);
    g.write::<u8>(w(0x3bf), 1);
    g.write::<u32>(w(0x3c6), w(0x3ca));
    g.write::<u32>(w(0x3d0), u32::MAX);
    let s = if g.read::<u32>(w(0x390)) == 0 {
        215
    } else {
        413
    };
    // call width: 1
    let r = g.call(slot(g, s), 0, &[n]).eax;
    g.write::<u32>(w(0x3fa), r);
    g.write::<u32>(w(0x400), u32::MAX);
    g.write::<u32>(w(0x40a), 0);
    g.write::<u32>(w(0x414), 1);
    if g.read::<u32>(w(0x3ef)) == 0 {
        g.write::<u16>(w(0x41e), 0);
    } else {
        g.write::<u16>(w(0x427), 1);
    }
    // the step task's node, appended to the task list
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x431), w(0x42c));
    g.write::<u8>(node + 0x18, 1);
    g.write::<u32>(node + 4, 0);
    if g.read::<u32>(w(0x437)) == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x45d), node);
    } else {
        let last = g.read::<u32>(w(0x44b));
        g.write::<u32>(node, last);
        g.write::<u32>(last + 4, node);
    }
    g.write::<u32>(node + 8, node);
    g.write::<u32>(w(0x46b), node);
    crate::sa_misc::records_init(g, img, img.rel32_target(f + 0x470), w(0x463));
    // the spark tick's node and the frame task's after it
    let n2 = crate::sa_pool::take(g, img, img.rel32_target(f + 0x47a), w(0x475));
    let head = g.read::<u32>(w(0x480));
    g.write::<u32>(n2 + 0x14, 0xa0);
    g.write::<u32>(n2 + 4, 0);
    let head = if head == 0 {
        g.write::<u32>(n2, 0);
        g.write::<u32>(w(0x4af), n2);
        n2
    } else {
        let last = g.read::<u32>(w(0x49a));
        g.write::<u32>(n2, last);
        g.write::<u32>(last + 4, n2);
        head
    };
    g.write::<u32>(n2 + 8, n2);
    let n3 = crate::sa_pool::take(g, img, img.rel32_target(f + 0x4bc), w(0x4b4));
    g.write::<u8>(n3 + 0x18, 2);
    g.write::<u32>(n3 + 4, 0);
    if head == 0 {
        g.write::<u32>(n3, 0);
        g.write::<u32>(w(0x4eb), n3);
        g.write::<u32>(n3 + 8, n3);
        g.write::<u32>(w(0x4f3), n3);
    } else {
        g.write::<u32>(n3, n2);
        g.write::<u32>(n2 + 4, n3);
        g.write::<u32>(n3 + 8, n3);
        g.write::<u32>(w(0x4d9), n3);
    }
    Ret::eax(0)
}

/// [`start_up_0316`].
pub const START_UP_0316: Sig = Sig {
    len: 1278,
    hash: 0xaf5a_7733_6068_a1d5,
    calls: &[
        (0x37, Callee::Sig(&crate::sb_misc::LOOKUP_ACTORS)),
        (0x5a, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0xd3, Callee::Sig(&crate::s4_misc::START_OBJECT_OWNERS)),
        (0x1b7, Callee::Sig(&crate::s4_misc::START_OBJECT_OWNERS)),
        (0x2f9, Callee::Sig(&crate::s3_misc::FADE_TASK)),
        (0x31d, Callee::Sig(&crate::s3_misc::FADE_TASK)),
        (0x431, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x470, Callee::Sig(&crate::sa_misc::RECORDS_INIT)),
        (0x47a, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x4bc, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// `magic_0316`'s load step (0x100012b0, the 118-byte shape) running
/// [`start_up_0316`].
pub fn load_0316<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::r9_load::load_step(g, img, fr, crate::r9_load::Shape::Zero, start_up_0316)
}

/// [`load_0316`].
pub const LOAD_0316: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&START_UP_0316))],
    imports: &[],
};

/// `magic_0361`'s globals as the operands name them: the block `*G_E`
/// (`+0x198` the caster's word `C`, `+0x1a0` the target's `T`).
fn block_words<G: Game>(g: &G, img: &Image, f: u32, op: u32) -> (u32, u32) {
    let e = g.read::<u32>(img.word(f + op));
    (g.read::<u32>(e + 0x198), g.read::<u32>(e + 0x1a0))
}

/// `magic_0361`'s aim (0x10001cb0, 320 bytes; cdecl `(ev)`; an `ebp` /GS
/// frame, the point `Q` at `ebp - 0x14`): the step count `*G_N` 0; slot
/// 456 `(T, 0, &G_P)`; slot 503 `(C, &Q)`; `V = (Q.x, -1, Q.z, 0)` (four
/// globals) handed to slot 464; `V *= K`, then `U = V.xyz / K'` (three
/// globals); slot 461 `(ev, CB)` with the next step's callback. Returns
/// slot 461's `eax`.
pub fn aim_0361<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x2e), n);
    let ebp = fr.esp.wrapping_sub(4);
    let q = ebp.wrapping_sub(0x14);
    let ev = fr.arg(g, 0);
    let (c, t) = block_words(g, img, f, 0x11);
    g.write::<u32>(w(0x34), 0);
    // call width: 3
    g.call(slot(g, 456), 0, &[t, 0, w(0x21)]);
    // call width: 2
    g.call(slot(g, 503), 0, &[c, q]);
    let x = g.read::<u32>(q);
    g.write::<u32>(w(0x64), x);
    g.write::<u32>(w(0x6f), 0xbf80_0000);
    let z = g.read::<u32>(q + 8);
    g.write::<u32>(w(0x7b), z);
    g.write::<u32>(w(0x81), 0);
    // call width: 1
    g.call(slot(g, 464), 0, &[w(0x90)]);
    let s = k(g, 0x9a);
    let v3 = mul(k(g, 0xa2), s);
    let v0 = mul(k(g, 0xaa), s);
    let v1 = mul(k(g, 0xb2), s);
    let v2 = mul(k(g, 0xba), s);
    g.write::<f32>(w(0xd2), v3);
    let d = k(g, 0xda);
    g.write::<f32>(w(0xe2), v0);
    g.write::<f32>(w(0xea), v1);
    g.write::<f32>(w(0xf2), v2);
    g.write::<f32>(w(0x10b), div(v0, d));
    g.write::<f32>(w(0x113), div(v1, d));
    g.write::<f32>(w(0x11b), div(v2, d));
    // call width: 2
    Ret::eax(g.call(slot(g, 461), 0, &[ev, w(0x126)]).eax)
}

/// [`aim_0361`] (320 bytes, 1 overlay).
pub const AIM_0361: Sig = Sig {
    len: 320,
    hash: 0x0d7f_f1d6_5aa5_6303,
    calls: &[
        // __security_check_cookie
        (0x138, Callee::Any),
    ],
    imports: &[],
};

/// The twelve words of the 3x4 matrix at the operands `ops` (each read
/// and written through its own operand pair) times `s`.
fn scale_rows<G: Game>(g: &mut G, img: &Image, f: u32, ops: &[(u32, u32); 12], s: f32) {
    for &(from, to) in ops {
        let v = mul(g.read::<f32>(img.word(f + from)), s);
        g.write::<f32>(img.word(f + to), v);
    }
}

/// `magic_0361`'s fade-in (0x100027d0, 336 bytes; cdecl `(ev)`; an `ebp`
/// /GS frame, the colours `A` at `ebp - 0x24` and `B` at `ebp - 0x14`).
/// Unless `*G_HOLD` is set: `t = (10 - *G_N) / K`, `A = (t K1, t K2, t
/// K3, t K3)`, `B = (t K4 + K0, t K5 + K0, t K6 + K0, t K3 + K0)`, slot 499
/// `(T, &A, &B)`. Then `*G_N` counts up; past 10, slot 502 `(T)` (unless
/// `*G_HOLD`), slots 450 `(ev, CB1)` and 449 `(ev, CB2)`, and the event's
/// record `i = ev + 0xac` gets `T` at `ev + 0x2c + 16 i`. Returns the count
/// (at most 10) or `2 i`.
pub fn fade_in_0361<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0xac), n);
    let ebp = fr.esp.wrapping_sub(4);
    let (a, b) = (ebp.wrapping_sub(0x24), ebp.wrapping_sub(0x14));
    let ev = fr.arg(g, 0);
    let mut hold = g.read::<u32>(w(0x17));
    let (_, t) = block_words(g, img, f, 0x11);
    if hold == 0 {
        let n = 10i32.wrapping_sub(g.read::<i32>(w(0x35)));
        #[allow(clippy::cast_precision_loss)] // cvtdq2ps
        let x = div(n as f32, k(g, 0x47));
        let a0 = mul(x, k(g, 0x53));
        let a3 = mul(x, k(g, 0x5e));
        g.write::<f32>(a, a0);
        let a1 = mul(x, k(g, 0x6e));
        let b0 = mul(x, k(g, 0x79));
        let b1 = mul(x, k(g, 0x84));
        let b2 = mul(x, k(g, 0x8c));
        g.write::<f32>(a + 4, a1);
        let k0 = k(g, 0x99);
        g.write::<f32>(a + 8, a3);
        g.write::<f32>(a + 0xc, a3);
        for (i, v) in (0u32..).zip([add(b0, k0), add(b1, k0), add(b2, k0), add(a3, k0)]) {
            g.write::<f32>(b + 4 * i, v);
        }
        // call width: 3
        g.call(slot(g, 499), 0, &[t, a, b]);
        hold = g.read::<u32>(w(0xdf));
    }
    let n = g.read::<i32>(w(0xe7)).wrapping_add(1);
    g.write::<i32>(w(0xed), n);
    if n <= 10 {
        return Ret::eax(n.cast_unsigned());
    }
    if hold == 0 {
        // call width: 1
        g.call(slot(g, 502), 0, &[t]);
    }
    // call width: 2
    g.call(slot(g, 450), 0, &[ev, w(0x111)]);
    // call width: 2
    g.call(slot(g, 449), 0, &[ev, w(0x124)]);
    let i2 = g.read::<u32>(ev + 0xac).wrapping_mul(2);
    g.write::<u32>(ev.wrapping_add(i2.wrapping_mul(8)).wrapping_add(0x2c), t);
    Ret::eax(i2)
}

/// [`fade_in_0361`] (336 bytes, 1 overlay).
pub const FADE_IN_0361: Sig = Sig {
    len: 336,
    hash: 0xf57a_31c0_c917_6a3f,
    calls: &[
        // __security_check_cookie
        (0x148, Callee::Any),
    ],
    imports: &[],
};

/// The matrix rows' operands (read, write) of [`flight_0361`].
const ROWS_23B0: [(u32, u32); 12] = [
    (0x29e, 0x2b1),
    (0x2b9, 0x2cb),
    (0x2d3, 0x2df),
    (0x2e7, 0x2f3),
    (0x2fb, 0x307),
    (0x30f, 0x31b),
    (0x323, 0x32f),
    (0x337, 0x343),
    (0x34b, 0x357),
    (0x35f, 0x36b),
    (0x373, 0x37f),
    (0x387, 0x393),
];

/// `magic_0361`'s flight (0x100023b0, 950 bytes; cdecl `(ev)`; a 16-aligned
/// /GS frame `E`: the point `P` at `E + 8`, slot 500's block at `E +
/// 0x18`). The matrix `M` (`*G_M`, 3x4) set to the unit (slot 235); the
/// velocity `U` (four globals) added to the place `V` (four globals) and
/// `V` to the path `S` (four globals). With `V.y` and `S.y` not below 0
/// (and ordered): `V *= K` with `V.y` negated, `U *= K'`, `S.y` 0; slot 446
/// `(T, 2)` on the first bounce (`*G_N` 0), slot 434 `(*G_E, CB)`; `*G_N`
/// counts up, and from 3 it is 0 again and the event goes on (slots 450
/// `(ev, CB1)`, 449 `(ev, CB2)`; `T` at `ev + 0x2c + 16 i`, `i = ev +
/// 0xac`). Then slot 500 `(&B)`; `P = (0, slot 477 (T), 0, 0)`, slot 496
/// `(&P)`, slot 494 `(M, &B, &P, &S)`; `M`'s twelve words times `K2`;
/// slot 484 `(T, M, 0)`. Returns slot 484's `eax`.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn flight_0361<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x25), n);
    let e = (fr.esp.wrapping_sub(4) & !0xf).wrapping_sub(0x68);
    let ev = fr.arg(g, 0);
    let (_, t) = block_words(g, img, f, 0x15);
    // call width: 1
    g.call(slot(g, 235), 0, &[w(0x2a)]);
    let u0 = k(g, 0x3a);
    let s0 = k(g, 0x42);
    let v2 = add(k(g, 0x4a), k(g, 0x52));
    let v3 = add(k(g, 0x5a), k(g, 0x62));
    let u1 = k(g, 0x6a);
    let s1 = k(g, 0x72);
    let v0 = add(u0, k(g, 0x7d));
    let v1 = add(u1, k(g, 0x88));
    g.write::<f32>(w(0x93), v2);
    g.write::<f32>(w(0x9f), v0);
    g.write::<f32>(w(0xab), v1);
    g.write::<f32>(w(0xb3), v3);
    g.write::<f32>(w(0xbb), add(s0, v0));
    let s1 = add(s1, v1);
    g.write::<f32>(w(0xcf), s1);
    let s2 = add(k(g, 0xc3), v2);
    g.write::<f32>(w(0xd7), s2);
    let s3 = add(k(g, 0xdf), v3);
    g.write::<f32>(w(0xeb), s3);
    // comiss x, 0; jb: on below or unordered
    if !(v1 < 0.0 || v1.is_nan() || s1 < 0.0 || s1.is_nan()) {
        let first = g.read::<u32>(w(0x106)) == 0;
        let kk = k(g, 0x10f);
        g.write::<f32>(w(0x11f), mul(v0, kk));
        let ny = mul(v1, kk).to_bits() ^ g.read::<u32>(w(0x12e));
        let k2 = k(g, 0x13e);
        let x = mul(k(g, 0x127), k2);
        g.write::<f32>(w(0x14e), x);
        let u3 = mul(k(g, 0x156), k2);
        g.write::<f32>(w(0x166), mul(v2, kk));
        g.write::<f32>(w(0x16e), mul(v3, kk));
        g.write::<f32>(w(0x176), mul(u0, k2));
        g.write::<f32>(w(0x17e), mul(u1, k2));
        g.write::<f32>(w(0x186), u3);
        g.write::<u32>(w(0x18e), ny);
        g.write::<u32>(w(0x194), 0);
        if first {
            // call width: 2
            g.call(slot(g, 446), 0, &[t, 2]);
        }
        let blk = g.read::<u32>(w(0x1bd));
        // call width: 2
        g.call(slot(g, 434), 0, &[blk, w(0x1b7)]);
        let n = g.read::<i32>(w(0x1ca)).wrapping_add(1);
        g.write::<i32>(w(0x1d3), n);
        if n >= 3 {
            g.write::<u32>(w(0x1ef), 0);
            // call width: 2
            g.call(slot(g, 450), 0, &[ev, w(0x1e2)]);
            // call width: 2
            g.call(slot(g, 449), 0, &[ev, w(0x1ff)]);
            let i2 = g.read::<u32>(ev + 0xac).wrapping_mul(2);
            g.write::<u32>(ev.wrapping_add(i2.wrapping_mul(8)).wrapping_add(0x2c), t);
        }
    }
    let (p, bk) = (e + 8, e + 0x18);
    // call width: 1
    g.call(slot(g, 500), 0, &[bk]);
    for o in [0x14u32, 0x10, 0xc, 8] {
        g.write::<u32>(e + o, 0);
    }
    // call width: 1
    let y = st0_f32(g.call(slot(g, 477), 0, &[t]).st0);
    g.write::<f32>(p + 4, y);
    // call width: 1
    g.call(slot(g, 496), 0, &[p]);
    // call width: 4
    g.call(slot(g, 494), 0, &[w(0x286), bk, p, w(0x272)]);
    scale_rows(g, img, f, &ROWS_23B0, k(g, 0x296));
    // call width: 3
    Ret::eax(g.call(slot(g, 484), 0, &[t, w(0x2c2), 0]).eax)
}

/// [`flight_0361`] (950 bytes, 1 overlay).
pub const FLIGHT_0361: Sig = Sig {
    len: 950,
    hash: 0x801e_5951_730f_78ce,
    calls: &[
        // __security_check_cookie
        (0x3ae, Callee::Any),
    ],
    imports: &[],
};

/// The matrix rows' operands (read, write) of [`throw_0361`].
const ROWS_1DF0: [(u32, u32); 12] = [
    (0x20a, 0x216),
    (0x21e, 0x22a),
    (0x232, 0x23e),
    (0x246, 0x252),
    (0x25a, 0x266),
    (0x26e, 0x27a),
    (0x282, 0x28e),
    (0x296, 0x2a2),
    (0x2aa, 0x2bd),
    (0x2c5, 0x2d7),
    (0x2df, 0x2eb),
    (0x2f3, 0x2ff),
];

/// `magic_0361`'s throw (0x10001df0, 1468 bytes; cdecl `(ev)`; an `ebp`
/// /GS frame: the matrices `A` `ebp - 0x74`, `B` `ebp - 0xb4`, `D` `ebp -
/// 0xf4`, the point `P` `ebp - 0x14`, the colours `ebp - 0x34`/`- 0x24`,
/// the handles `h1` `ebp - 0xf8`, `h2` `ebp - 0xfc`). `h1` = slot 481
/// `(C)`, `h2` = slot 480 `(T, 6)`; `A`, `B`, `D` and `M` (`*G_M`) the
/// unit (slot 235). By the count `n = *G_N`:
/// - 5..9: [`flight_0361`]'s step of `V` and `S` (no bounce), slot 500
///   `(&D)`, `P = (n K, slot 477 (T), 0, 0)`, slot 496 `(&P)`, slot 494
///   `(M, &D, &P, &S)`, `M`'s rows times `K2`, slot 484 `(T, M, 0)`;
/// - 10 and up (at 10 first slots 447 `(T, 0x49)` and 508 `(T, 0)`): slot
///   236 `(h1, 0x119, &A)`, slot 504 `(&A)`, `P = (0, slot 497 (slot 477
///   (T) + K), 0, 0)`, slot 498 `(T, h2, &B, &P)`, `M = A B` (slot 231),
///   slot 484 `(T, M, &G_X)`.
///
/// `*G_N` counts up. Once the event's record `i = ev + 0xac` has a
/// positive word at `ev + 0x2c + 16 i`: unless `*G_HOLD`, slots 501
/// `(T)` and 499 `(T, &(24, 16, 0, 0), &(1.4, 1.2, 0.1, 1))`; slots 507
/// `(T, 1)`, 508 `(T, 1)`, 456 `(T, 0, &S)`, 503 `(C, &V)`; `V.x *= K3`,
/// `V.z *= K3`, `V.w *= K3`, `V.y = V.y K3 + K4`, `U = (0, 1, V.z / K5,
/// 0)` and `V.z *= K6`; `*G_N` 0, slot 447 `(T, 0x48)`, slot 461 `(ev,
/// CB)`. Returns slot 461's `eax`, or `2 i` when the record's word is not
/// positive.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn throw_0361<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x20), n);
    let ebp = fr.esp.wrapping_sub(4);
    let at = |o: u32| ebp.wrapping_sub(o);
    let (am, bm, dm, p) = (at(0x74), at(0xb4), at(0xf4), at(0x14));
    let ev = fr.arg(g, 0);
    let c = g.read::<u32>(g.read::<u32>(w(0x14)) + 0x198);
    // call width: 1
    let h1 = g.call(slot(g, 481), 0, &[c]).eax;
    let t = g.read::<u32>(g.read::<u32>(w(0x34)) + 0x1a0);
    g.write::<u32>(at(0xf8), h1);
    // call width: 2
    let h2 = g.call(slot(g, 480), 0, &[t, 6]).eax;
    g.write::<u32>(at(0xfc), h2);
    for m in [am, bm, dm, w(0x9c)] {
        // call width: 1
        g.call(slot(g, 235), 0, &[m]);
    }
    let n = g.read::<i32>(w(0xa9));
    if (5..10).contains(&n) {
        let v0 = add(k(g, 0xc6), k(g, 0xce));
        let s0 = k(g, 0xd6);
        let v1 = add(k(g, 0xde), k(g, 0xe6));
        let v2 = add(k(g, 0xee), k(g, 0xf6));
        let v3 = add(k(g, 0xfe), k(g, 0x106));
        g.write::<f32>(w(0x11e), v0);
        g.write::<f32>(w(0x126), add(s0, v0));
        let s1 = add(k(g, 0x12e), v1);
        g.write::<f32>(w(0x13a), v1);
        g.write::<f32>(w(0x142), v2);
        g.write::<f32>(w(0x14a), v3);
        g.write::<f32>(w(0x152), s1);
        let s2 = add(k(g, 0x15a), v2);
        g.write::<f32>(w(0x166), s2);
        let s3 = add(k(g, 0x16e), v3);
        g.write::<f32>(w(0x17a), s3);
        // call width: 1
        g.call(slot(g, 500), 0, &[dm]);
        for o in [8u32, 0xc, 0x10, 0x14] {
            g.write::<u32>(at(o), 0);
        }
        // call width: 1
        let y = st0_f32(g.call(slot(g, 477), 0, &[t]).st0);
        g.write::<f32>(p + 4, y);
        #[allow(clippy::cast_precision_loss)] // cvtdq2ps
        let x = mul(g.read::<i32>(w(0x1b7)) as f32, k(g, 0x1c5));
        g.write::<f32>(p, x);
        // call width: 1
        g.call(slot(g, 496), 0, &[p]);
        // call width: 4
        g.call(slot(g, 494), 0, &[w(0x1f2), dm, p, w(0x1dd)]);
        scale_rows(g, img, f, &ROWS_1DF0, k(g, 0x202));
        // call width: 3
        g.call(slot(g, 484), 0, &[t, w(0x2ce), 0]);
    } else if n >= 10 {
        if n == 10 {
            // call width: 2
            g.call(slot(g, 447), 0, &[t, 0x49]);
            // call width: 2
            g.call(slot(g, 508), 0, &[t, 0]);
        }
        // call width: 3
        g.call(slot(g, 236), 0, &[h1, 0x119, am]);
        // call width: 1
        g.call(slot(g, 504), 0, &[am]);
        for o in [8u32, 0xc, 0x10, 0x14] {
            g.write::<u32>(at(o), 0);
        }
        // call width: 1
        let y = st0_f32(g.call(slot(g, 477), 0, &[t]).st0);
        g.write::<f32>(at(0xf8), y);
        let x = add(y, k(g, 0x3a1));
        // call width: 1
        let y = st0_f32(g.call(slot(g, 497), 0, &[x.to_bits()]).st0);
        g.write::<f32>(p + 4, y);
        // call width: 4
        g.call(slot(g, 498), 0, &[t, h2, bm, p]);
        // call width: 3
        g.call(slot(g, 231), 0, &[w(0x3ed), am, bm]);
        // call width: 3
        g.call(slot(g, 484), 0, &[t, w(0x40a), w(0x3ff)]);
    }
    let n = g.read::<i32>(w(0x415)).wrapping_add(1);
    g.write::<i32>(w(0x41b), n);
    let i2 = g.read::<u32>(ev + 0xac).wrapping_mul(2);
    let word = g.read::<i32>(ev.wrapping_add(i2.wrapping_mul(8)).wrapping_add(0x2c));
    if word <= 0 {
        return Ret::eax(i2);
    }
    let (ca, cb) = (at(0x34), at(0x24));
    for (i, x) in (0u32..).zip([
        0x41c0_0000u32,
        0x4180_0000,
        0,
        0,
        0x3fb3_3333,
        0x3f99_999a,
        0x3dcc_cccd,
        ONE,
    ]) {
        g.write::<u32>(ca + 4 * i, x);
    }
    if g.read::<u32>(w(0x434)) == 0 {
        // call width: 1
        g.call(slot(g, 501), 0, &[t]);
        // call width: 3
        g.call(slot(g, 499), 0, &[t, ca, cb]);
    }
    for s in [507u32, 508] {
        // call width: 2
        g.call(slot(g, s), 0, &[t, 1]);
    }
    // call width: 3
    g.call(slot(g, 456), 0, &[t, 0, w(0x4c0)]);
    // call width: 2
    g.call(slot(g, 503), 0, &[c, w(0x4d5)]);
    let s = k(g, 0x4e6);
    let v0 = mul(k(g, 0x4ee), s);
    let v2 = mul(k(g, 0x4f6), s);
    g.write::<f32>(w(0x506), v0);
    let v3 = mul(k(g, 0x50e), s);
    let v1 = mul(k(g, 0x516), s);
    g.write::<f32>(w(0x52b), v3);
    let u2 = div(v2, k(g, 0x536));
    let v1 = add(v1, k(g, 0x53e));
    let v2 = mul(v2, k(g, 0x546));
    g.write::<u32>(w(0x54c), 0);
    g.write::<u32>(w(0x556), ONE);
    g.write::<u32>(w(0x560), 0);
    g.write::<f32>(w(0x56c), v1);
    g.write::<f32>(w(0x574), u2);
    g.write::<f32>(w(0x57c), v2);
    g.write::<u32>(w(0x58b), 0);
    // call width: 2
    g.call(slot(g, 447), 0, &[t, 0x48]);
    // call width: 2
    Ret::eax(g.call(slot(g, 461), 0, &[ev, w(0x59b)]).eax)
}

/// [`throw_0361`] (1468 bytes, 1 overlay).
pub const THROW_0361: Sig = Sig {
    len: 1468,
    hash: 0xeff5_b381_e7ca_7436,
    calls: &[
        // __security_check_cookie
        (0x5b4, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0350`'s task (0x100059a0, 97 bytes; a task callback, cdecl
/// `(node)`): the 0x30-byte block `*G_B` cleared and its flags (`+0x2`)
/// given bit 7, `G_B + 0x14` = `*G_T`; the script ([`run`](crate::sa_script::run))
/// and the motion ([`motion_step`](crate::s3_motion::motion_step)) step;
/// once both answer 1, `S + 0xe60` = 1 (`S = *G_S`), the lights, models
/// and parts ([`lights_models_parts`](crate::m0350::lights_models_parts))
/// and the node goes back to the pool (a tail jump into
/// [`give_back`](crate::sa_pool::give_back)). Returns the give-back's
/// `eax`, or the motion step's.
pub fn task_0350<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let s = g.read::<u32>(w(0x07));
    g.write_bytes(w(0x10), &[0u8; 0x30]);
    let h = g.read::<u16>(w(0x21));
    g.write::<u16>(w(0x21), h | 0x80);
    let v = g.read::<u32>(w(0x26));
    g.write::<u32>(w(0x2e), v);
    let script = crate::sa_script::run(g, img, img.rel32_target(f + 0x33));
    let motion = crate::s3_motion::motion_step(g, img, img.rel32_target(f + 0x3a));
    if script != 1 || motion != 1 {
        return Ret::eax(motion);
    }
    g.write::<u32>(s + 0xe60, 1);
    // push ebp, esi, edi; the call
    let parts = Frame {
        at: img.rel32_target(f + 0x4e),
        esp: fr.esp.wrapping_sub(0x10),
        ..*fr
    };
    crate::m0350::lights_models_parts(g, img, &parts);
    let node = fr.arg(g, 0);
    Ret::eax(crate::sa_pool::give_back(
        g,
        img,
        img.rel32_target(f + 0x59),
        node,
    ))
}

/// [`task_0350`] (97 bytes, 1 overlay).
pub const TASK_0350: Sig = Sig {
    len: 97,
    hash: 0x063c_f503_9556_ffe9,
    calls: &[
        (0x15, Callee::Import("memset")),
        (0x33, Callee::Sig(&crate::sa_script::RUN)),
        (0x3a, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0x4e, Callee::Sig(&crate::m0350::LIGHTS_MODELS_PARTS)),
        (0x59, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
    ],
    imports: &[],
};

/// `magic_0350`'s packed sprite (0x10005c60, 328 bytes; a task callback,
/// cdecl `(node)`; a /GS frame of 0x8c bytes, `ebp` = `esp - 4`), as
/// [`packed_frame`](crate::m0672::packed_frame) with the state `S = *G_S`:
/// while its fade-out `S + 0xe50` is not above 0 the node goes back to
/// the pool (a give-back, its `eax` returned); else the fade `n` is
/// `(s - c) * 80 / s` while the count `c = S + 0xe48` is above 0 (it
/// counts down; `s = S + 0xe4c`), else `S + 0xe50 - 1` (stored) times 80
/// over `S + 0xe54` (a zero divisor gives 0, where the original faults);
/// the angle `S + 0xe44 -= DA`, plus `T` below 0; slot 719 `(P, 2, 6)`
/// gives the descriptor `(lo, hi)`; slot 235 `(M)` (`ebp - 0x44`), slot
/// 210 `(*G_O, M2)` (`ebp - 0x84`); the texture drawn
/// ([`packed_texture`](crate::s5_misc::packed_texture), `ecx` = `*G_O`,
/// `(-, M, lo, hi, 0, angle, n)`; the first word is the `ecx` slot 210
/// leaves, which the callee does not read: 0 here). Returns the draw's
/// `eax`.
pub fn packed_0350<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let s = g.read::<u32>(w(0x19));
    let out = g.read::<i32>(s + 0xe50);
    if out <= 0 {
        let node = fr.arg(g, 0);
        return Ret::eax(crate::sa_pool::give_back(
            g,
            img,
            img.rel32_target(f + 0x2e),
            node,
        ));
    }
    let c = g.read::<i32>(s + 0xe48);
    let n = if c > 0 {
        let total = g.read::<i32>(s + 0xe4c);
        let c = c - 1;
        g.write::<i32>(s + 0xe48, c);
        total
            .wrapping_sub(c)
            .wrapping_mul(80)
            .checked_div(total)
            .unwrap_or(0)
    } else {
        let left = out - 1;
        g.write::<i32>(s + 0xe50, left);
        left.wrapping_mul(80)
            .checked_div(g.read::<i32>(s + 0xe54))
            .unwrap_or(0)
    };
    let angle = sub(g.read::<f32>(s + 0xe44), k(g, 0x89));
    g.write::<f32>(s + 0xe44, angle);
    // comiss 0, a; jbe: only below 0 (ordered)
    if 0.0 > angle {
        g.write::<f32>(s + 0xe44, add(angle, k(g, 0xa7)));
    }
    let o = g.read::<u32>(w(0xba));
    let t = slot_target(g, w(0xb4), 719);
    // call width: 3
    let d = g.call(t, 0, &[w(0xc9), 2, 6]).eax;
    let (lo, hi) = (g.read::<u32>(d), g.read::<u32>(d + 4));
    let ebp = fr.esp.wrapping_sub(4);
    let (m, m2) = (ebp.wrapping_sub(0x44), ebp.wrapping_sub(0x84));
    let t = slot_target(g, w(0xd9), 235);
    // call width: 1
    g.call(t, 0, &[m]);
    let a = g.read::<u32>(w(0xf3));
    let t = slot_target(g, w(0xed), 210);
    // call width: 2
    g.call(t, 0, &[a, m2]);
    let angle = g.read::<u32>(s + 0xe44);
    // the frame, ebx/esi/edi, n, 8 bytes and four pushes, the return
    // address
    let esp = ebp.wrapping_sub(0xb8);
    for (i, v) in (1u32..).zip([0, m, lo, hi, 0, angle, n.cast_unsigned()]) {
        g.write::<u32>(esp + 4 * i, v);
    }
    let callee = Frame {
        at: img.rel32_target(f + 0x130),
        esp,
        ecx: o,
        ..Frame::default()
    };
    crate::s5_misc::packed_texture(g, img, &callee)
}

/// [`packed_0350`] (328 bytes, 1 overlay).
pub const PACKED_0350: Sig = Sig {
    len: 328,
    hash: 0xf931_02de_709c_4502,
    calls: &[
        (0x2e, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        // __security_check_cookie
        (0x39, Callee::Any),
        (0x130, Callee::Sig(&crate::s5_misc::PACKED_TEXTURE)),
        (0x140, Callee::Any),
    ],
    imports: &[],
};

/// The records of [`set_up_0350`]: its offset in `S`, `+0x20`, `+0x28`,
/// `+0x10`, `+0x14`, `+0x18` and `+0x24` (when written again).
type Record0350 = (u32, u32, u32, u32, u32, u32, Option<u32>);

/// [`Record0350`]s.
const RECORDS_0350: [Record0350; 3] = [
    (0xb18, 0x3bda_740e, 0x96, 0, 0x40a0_0000, 0x3f00_0000, None),
    (
        0xb48,
        0x3c23_d70a,
        0x64,
        0x3f00_0000,
        0x40a0_0000,
        0,
        Some(0x96),
    ),
    (0xb78, 0x3ccc_cccd, 0x28, 0x4080_0000, ONE, 0, Some(0x160)),
];

/// `magic_0350`'s set-up (0x10005db0, 1232 bytes; no arguments; an `ebp`
/// /GS frame) on the state `S = *G_S`: the object `S + 0x130` started
/// ([`start_object_plain`](crate::s4_misc::start_object_plain), scale
/// `K`, index 1), its point `+0xd0` = `S + 0x990` with `y` less `K4`, its
/// `+0xc4` = `S + 0x984`; a scale task on `S + 0xba8`
/// ([`scale_task_fixed`](crate::s4_small::scale_task_fixed), `xmm1` `K1`,
/// `xmm2` `K4`); three records (`S + 0xb18`, `+0xb48`, `+0xb78`) each run
/// by a pool node ([`take`](crate::sa_pool::take)) appended to the task
/// list; then six key records copied (four words each) with their counts
/// 0x3c or 0x78 and fixed values, the last one's point `S + 0xe30` times
/// `K4`.
///
/// Junk reads: `[ebp - 0xc]` and `[ebp - 8]` are never written; 0 (brief
/// 2026-10-24) in `S + 0xcb0`, `+0xcb4`, `+0xcec`, `+0xd24`, `+0xd5c` and
/// `+0xd94`. Returns `S + 0xd74`.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn set_up_0350<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let s = g.read::<u32>(w(0x1d));
    let obj = s + 0x130;
    let scale = k(g, 0x14);
    crate::s4_misc::start_object_plain(g, img, img.rel32_target(f + 0x2f), obj, scale, 1);
    let k4 = k(g, 0x3d);
    for i in 0..4u32 {
        let v = g.read::<u32>(s + 0x990 + 4 * i);
        g.write::<u32>(obj + 0xd0 + 4 * i, v);
    }
    let y = sub(g.read::<f32>(obj + 0xd4), k4);
    let v = g.read::<u32>(s + 0x984);
    g.write::<u32>(obj + 0xc4, v);
    g.write::<f32>(obj + 0xd4, y);
    // ebp, the frame, ebx/esi/edi, 0xc bytes and the start's word (0xc
    // of them popped), the call
    let ebp = fr.esp.wrapping_sub(4);
    let task = Frame {
        at: img.rel32_target(f + 0xa0),
        esp: ebp.wrapping_sub(0x2c),
        ecx: s + 0xba8,
        xmm: [
            0,
            crate::Xmm::f32(k(g, 0x45)).reg(),
            crate::Xmm::f32(k4).reg(),
            0,
        ],
        ..Frame::default()
    };
    crate::s4_small::scale_task_fixed(g, img, &task);
    // the three records and their nodes
    let mut last = 0u32;
    for (j, (&(off, k20, n28, k10, k14, k18, w24), (call, cb, head, tail, set))) in
        (0u32..).zip(RECORDS_0350.iter().zip([
            (0xdbu32, 0xb0u32, 0xe1u32, 0xfbu32, 0x10du32),
            (0x177, 0x14c, 0x17d, 0, 0x1a6),
            (0x217, 0x1ec, 0x21d, 0, 0x240),
        ]))
    {
        let r = s + off;
        g.write::<u32>(r, 0);
        g.write::<u32>(r + 0x1c, 0);
        g.write::<u32>(r + 0x20, k20);
        g.write::<u32>(r + 0x24, 0);
        g.write::<u32>(r + 0x28, n28);
        g.write::<u16>(r + 0x2c, 0);
        let node = crate::sa_pool::take(g, img, img.rel32_target(f + call), w(cb));
        let empty = g.read::<u32>(w(head)) == 0;
        g.write::<u8>(node + 0x18, 1);
        g.write::<u32>(node + 0x14, r);
        g.write::<u32>(node + 4, 0);
        if empty {
            g.write::<u32>(node, 0);
            g.write::<u32>(w(set), node);
        } else {
            let prev = if j == 0 { g.read::<u32>(w(tail)) } else { last };
            g.write::<u32>(node, prev);
            g.write::<u32>(prev + 4, node);
        }
        g.write::<u32>(node + 8, node);
        g.write::<u32>(r + 4, 0);
        g.write::<u32>(r + 8, 0x3e4c_cccd);
        g.write::<u32>(r + 0xc, 0);
        g.write::<u32>(r + 0x10, k10);
        g.write::<u32>(r + 0x14, k14);
        g.write::<u32>(r + 0x18, k18);
        let fl = g.read::<u16>(r + 0x2c);
        g.write::<u16>(r + 0x2c, fl | 3);
        if let Some(x) = w24 {
            g.write::<u32>(r + 0x24, x);
        }
        last = node;
    }
    g.write::<u32>(w(0x283), last);
    let copy = |g: &mut G, from: u32, to: u32| {
        for i in 0..4u32 {
            let v = g.read::<u32>(s + from + 4 * i);
            g.write::<u32>(s + to + 4 * i, v);
        }
    };
    // junk read: [ebp-0xc] and [ebp-0x8] never written; 0 (brief 2026-10-24)
    let junk = 0u32;
    copy(g, 0xc88, 0xc98);
    g.write::<u32>(s + 0xcb0, junk);
    g.write::<u32>(s + 0xca8, 0x401c_61ab);
    g.write::<u32>(s + 0xcac, 0xc044_980a);
    g.write::<u32>(s + 0xcb4, junk);
    g.write::<u32>(s + 0xcbc, 0x3c);
    g.write::<u32>(s + 0xcb8, 0x3c);
    copy(g, 0xcc0, 0xcd0);
    for o in [0xce0u32, 0xce4, 0xce8] {
        g.write::<u32>(s + o, 0);
    }
    g.write::<u32>(s + 0xcec, junk);
    g.write::<u32>(s + 0xcf4, 0x78);
    g.write::<u32>(s + 0xcf0, 0x78);
    copy(g, 0xcf8, 0xd08);
    for o in [0xd18u32, 0xd1c, 0xd20] {
        g.write::<u32>(s + o, 0);
    }
    g.write::<u32>(s + 0xd24, junk);
    g.write::<u32>(s + 0xd2c, 0x78);
    g.write::<u32>(s + 0xd28, 0x78);
    copy(g, 0xd30, 0xd40);
    g.write::<u32>(s + 0xd50, 0x3f4c_cccd);
    g.write::<u32>(s + 0xd54, 0x3f55_d5d6);
    g.write::<u32>(s + 0xd58, ONE);
    g.write::<u32>(s + 0xd5c, junk);
    g.write::<u32>(s + 0xd64, 0x78);
    g.write::<u32>(s + 0xd60, 0x78);
    let p = [0u32, 4, 8].map(|o| mul(g.read::<f32>(s + 0xe30 + o), k4));
    copy(g, 0xd68, 0xd78);
    for (i, v) in (0u32..).zip(p) {
        g.write::<f32>(s + 0xd88 + 4 * i, v);
    }
    g.write::<u32>(s + 0xd94, junk);
    g.write::<u32>(s + 0xd9c, 0x78);
    g.write::<u32>(s + 0xd98, 0x78);
    Ret::eax(g.read::<u32>(s + 0xd74))
}

/// [`set_up_0350`] (1232 bytes, 1 overlay).
pub const SET_UP_0350: Sig = Sig {
    len: 1232,
    hash: 0x9b96_3318_f126_e66c,
    calls: &[
        (0x2f, Callee::Sig(&crate::s4_misc::START_OBJECT_PLAIN)),
        (0xa0, Callee::Sig(&crate::s4_small::SCALE_TASK_30)),
        (0xdb, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x177, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x217, Callee::Sig(&crate::sa_pool::TAKE)),
        // __security_check_cookie
        (0x4c8, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0350`'s sparks (0x10006320, 1110 bytes; no arguments; an `ebp`
/// /GS frame, the point `P` at `ebp - 0x14`). While the state's count `c
/// = S + 0xe58` (`S = *G_S`) is above 0 it counts down, `a = (c << 7) /
/// t` and `m = 4 c / t` (`t = S + 0xe5c`; a zero divisor gives 0, where
/// the original faults). For each of the five bone numbers at `*G_BONES`
/// .. `G_END`: `P` = slot 237 `(*G_M, bone, 0, &P)` with `w` 1.0, then
/// `m` times two records of the DLL's pool
/// ([`pool_take`](crate::sb_misc::pool_take); none: the end): `+0x30` the
/// handle (`*G_H` `+0xc`, then `+0x8`), `+0x34` 1, the halfwords `+0x38`
/// -1 and `+0x3a` 0x200, the place `P` plus rnd 2 per axis (slot 314),
/// `+0x10` rnd 0.2, `+0x14` `-(slot 404 (1) + K)`, `+0x18` rnd 0.2,
/// `+0x1c` rnd pi, `+0x20`..`+0x28` those three times `K1`, `+0x2c` 1.0,
/// `+0x3c` `(rnd 0.7 + K2) * K3`, `+0x4c` 0x20; the first `+0x50` 0 and the
/// byte `+0x6b` `a`, the second `+0x50` 0x20, the byte `+0x67` `a` and the
/// colour bytes `+0x64`..`+0x6b` (0x80, 0x60, 0x80, -, 0x80, 0x60, 0x80,
/// 0).
///
/// Returns 0 when the pool ran out, `m` (or slot 237's `eax` when `m` is
/// not above 0) after the last bone, and the cookie word (`*G_COOKIE ^
/// ebp`) when the count was out.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn sparks_0350<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, n: u32| slot_target(g, w(0x57), n);
    let ebp = fr.esp.wrapping_sub(4);
    let s = g.read::<u32>(w(0x13));
    let c = g.read::<i32>(s + 0xe58);
    if c <= 0 {
        return Ret::eax(g.read::<u32>(w(0x07)) ^ ebp);
    }
    let c = c - 1;
    g.write::<i32>(s + 0xe58, c);
    let t = g.read::<i32>(s + 0xe5c);
    let a = c.wrapping_shl(7).checked_div(t).unwrap_or(0);
    let m = c.wrapping_mul(4).checked_div(t).unwrap_or(0);
    let p = ebp.wrapping_sub(0x14);
    let mut eax = 0u32;
    let mut bone = w(0x3c);
    while bone < w(0x43c) {
        let b = g.read::<u32>(bone);
        let model = g.read::<u32>(w(0x67));
        // call width: 4
        eax = g.call(slot(g, 237), 0, &[model, b, 0, p]).eax;
        g.write::<u32>(p + 0xc, ONE);
        let mut j = 0i32;
        while j < m {
            for (second, ops) in [(false, &SPARK_0350_A), (true, &SPARK_0350_B)] {
                let rec = crate::sb_misc::pool_take(g, img, img.rel32_target(f + ops.take));
                if rec == 0 {
                    return Ret::eax(0);
                }
                let v = spark_0350(g, img, f, rec, p, ops);
                let a8 = a.to_le_bytes()[0];
                if second {
                    g.write::<u8>(rec + 0x67, a8);
                    g.write::<u32>(rec + 0x4c, 0x20);
                    g.write::<u32>(rec + 0x50, 0x20);
                    g.write::<u16>(rec + 0x64, 0x6080);
                    g.write::<u8>(rec + 0x66, 0x80);
                    g.write::<f32>(rec + 0x3c, v);
                    g.write::<u32>(rec + 0x68, 0x0080_6080);
                } else {
                    g.write::<u32>(rec + 0x4c, 0x20);
                    g.write::<u32>(rec + 0x50, 0);
                    g.write::<u8>(rec + 0x6b, a8);
                    g.write::<f32>(rec + 0x3c, v);
                }
            }
            j += 1;
            eax = j.cast_unsigned();
        }
        bone += 4;
    }
    Ret::eax(eax)
}

/// Where one of [`sparks_0350`]'s two records has its operands.
struct SparkOps {
    take: u32,
    handle: (u32, u32),
    k14: u32,
    neg: u32,
    k1: u32,
    k3c: (u32, u32),
}

const SPARK_0350_A: SparkOps = SparkOps {
    take: 0x87,
    handle: (0x97, 0xc),
    k14: 0x18e,
    neg: 0x1a1,
    k1: 0x1d0,
    k3c: (0x228, 0x244),
};

const SPARK_0350_B: SparkOps = SparkOps {
    take: 0x24e,
    handle: (0x25e, 0x8),
    k14: 0x355,
    neg: 0x368,
    k1: 0x397,
    k3c: (0x3ef, 0x40b),
};

/// One record of [`sparks_0350`] up to its `+0x2c`; returns its `+0x3c`
/// (`(rnd 0.7 + K2) * K3`), which the caller stores.
fn spark_0350<G: Game>(g: &mut G, img: &Image, f: u32, rec: u32, p: u32, o: &SparkOps) -> f32 {
    let w = |op: u32| img.word(f + op);
    let k = |g: &G, op: u32| g.read::<f32>(w(op));
    let slot = |g: &G, n: u32| slot_target(g, w(0x57), n);
    let h = g.read::<u32>(g.read::<u32>(w(o.handle.0)) + o.handle.1);
    g.write::<u16>(rec + 0x38, 0xffff);
    g.write::<u32>(rec + 0x30, h);
    g.write::<u32>(rec + 0x34, 1);
    g.write::<u16>(rec + 0x3a, 0x200);
    for i in 0..4u32 {
        let v = g.read::<u32>(p + 4 * i);
        g.write::<u32>(rec + 4 * i, v);
    }
    let rnd = |g: &mut G, n: u32, x: u32| {
        // call width: 1
        st0_f32(g.call(slot(g, n), 0, &[x]).st0)
    };
    for i in 0..3u32 {
        let r = rnd(g, 314, 0x4000_0000);
        let v = add(g.read::<f32>(rec + 4 * i), r);
        if i == 2 {
            g.write::<u32>(rec + 0xc, ONE);
        }
        g.write::<f32>(rec + 4 * i, v);
    }
    let r = rnd(g, 314, 0x3e4c_cccd);
    g.write::<f32>(rec + 0x10, r);
    let r = rnd(g, 404, ONE);
    let v = add(r, k(g, o.k14)).to_bits() ^ g.read::<u32>(w(o.neg));
    g.write::<u32>(rec + 0x14, v);
    let r = rnd(g, 314, 0x3e4c_cccd);
    g.write::<f32>(rec + 0x18, r);
    let r = rnd(g, 314, 0x4049_0fdb);
    g.write::<f32>(rec + 0x1c, r);
    let k1 = k(g, o.k1);
    g.write::<u32>(rec + 0x2c, ONE);
    for i in 0..3u32 {
        let v = mul(g.read::<f32>(rec + 0x10 + 4 * i), k1);
        g.write::<f32>(rec + 0x20 + 4 * i, v);
    }
    let q = rnd(g, 314, 0x3f33_3333);
    mul(add(q, k(g, o.k3c.0)), k(g, o.k3c.1))
}

/// [`sparks_0350`] (1110 bytes, 1 overlay).
pub const SPARKS_0350: Sig = Sig {
    len: 1110,
    hash: 0x4d57_ddf3_87ca_a17f,
    calls: &[
        (0x87, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
        (0x24e, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
        // __security_check_cookie
        (0x44e, Callee::Any),
    ],
    imports: &[],
};

/// The source words of [`keys_0350`]'s seven records (their second
/// four words).
const KEYS_0350: [u32; 7] = [0xda0, 0xdb0, 0xdc0, 0xe00, 0xe10, 0xe20, 0xe30];

/// `magic_0350`'s keys (0x10006830, 845 bytes; no frame): on the state `S
/// = *G_S`, seven 0x38-byte records from `S + 0xc28`: each gets the four
/// words before it (`S + 0xc18 + 0x38 i`), four words of `S + 0xda0`..
/// (see [`KEYS_0350`]) and both counts 0x1e; then a tail jump into the
/// scale task on `S + 0xbe0` ([`scale_task_fixed`](crate::s4_small::scale_task_fixed),
/// `xmm1` `K1`, `xmm2` `K2`). Returns the task's answer.
pub fn keys_0350<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let s = g.read::<u32>(w(0x02));
    for (i, src) in (0u32..).zip(KEYS_0350) {
        let r = s + 0xc28 + 0x38 * i;
        for j in 0..4u32 {
            let v = g.read::<u32>(r - 0x10 + 4 * j);
            g.write::<u32>(r + 4 * j, v);
        }
        for j in 0..4u32 {
            let v = g.read::<u32>(s + src + 4 * j);
            g.write::<u32>(r + 0x10 + 4 * j, v);
        }
        g.write::<u32>(r + 0x24, 0x1e);
        g.write::<u32>(r + 0x20, 0x1e);
    }
    let task = Frame {
        at: img.rel32_target(f + 0x349),
        esp: fr.esp,
        ecx: s + 0xbe0,
        xmm: [
            0,
            crate::Xmm::f32(k(g, 0x2e8)).reg(),
            crate::Xmm::f32(k(g, 0x2e0)).reg(),
            0,
        ],
        ..Frame::default()
    };
    crate::s4_small::scale_task_fixed(g, img, &task)
}

/// [`keys_0350`] (845 bytes, 1 overlay).
pub const KEYS_0350_SIG: Sig = Sig {
    len: 845,
    hash: 0x6c94_22c6_ddea_34cd,
    calls: &[(0x349, Callee::Sig(&crate::s4_small::SCALE_TASK_30))],
    imports: &[],
};

/// `magic_0275`'s target `T[sel[i]]`: the battle character table (operand
/// `tab`) at the selection's byte `i` (the selection pointer at `sel`).
fn target_0275<G: Game>(g: &G, img: &Image, f: u32, (sel, tab): (u32, u32), i: u32) -> u32 {
    let s = g.read::<u32>(img.word(f + sel));
    let b = u32::from(g.read::<u8>(s.wrapping_add(i)));
    g.read::<u32>(img.word(f + tab).wrapping_add(4 * b))
}

/// `magic_0275`'s summon shown (0x10003f90, 450 bytes; internal, no
/// arguments, no frame): the summon's handle `h` = slot 390 `(sel[0])`,
/// its model `m` = slot 0 `(h)`, slot 26 `(h, 1)`; the byte `G_B` = 0x20,
/// `G_M` = `m`, `G_S` = 2; for `T = T[sel[1]]` slots 357 `(T, -1,
/// 0x3c00)` and 356 `(T, -1, 0x2ee0)`; with `sel[0]` 13 slot 278 `(m, 1,
/// 1)`; slot 362 `(*G_W, m, 1, 0)`; with `sel[0]` 13 (read again) slot 278
/// `(T[sel[0]], 1, 0)`; slot 23 `(m, 0)`, slot 37 `(m, 1, 0x12)` for
/// `sel[0]` 8 (else 0x10), slots 249 and 10 `(m, 0)`, slots 6 `(T, 1)`,
/// 548 `(T)`, 345 `(m)`, 349 `(m, *G_X)` and 35 `(T, 7, 0, 0, 0)`.
/// Returns slot 35's `eax`.
pub fn show_0275<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, n: u32| slot_target(g, w(0x0d), n);
    let sel = |g: &G, i: u32| u32::from(g.read::<u8>(g.read::<u32>(w(0x01)).wrapping_add(i)));
    let b0 = sel(g, 0);
    // call width: 1
    let h = g.call(slot(g, 390), 0, &[b0]).eax;
    // call width: 1
    let m = g.call(slot(g, 0), 0, &[h]).eax;
    // call width: 2
    g.call(slot(g, 26), 0, &[h, 1]);
    g.write::<u8>(w(0x3d), 0x20);
    g.write::<u32>(w(0x44), m);
    g.write::<u32>(w(0x4a), 2);
    let kind = u32::from(g.read::<u8>(g.read::<u32>(w(0x37))));
    let t = target_0275(g, img, f, (0x37, 0x61), 1);
    // call width: 3
    g.call(slot(g, 357), 0, &[t, u32::MAX, 0x3c00]);
    // call width: 3
    g.call(slot(g, 356), 0, &[t, u32::MAX, 0x2ee0]);
    if kind == 0xd {
        // call width: 3
        g.call(slot(g, 278), 0, &[m, 1, 1]);
    }
    let x = g.read::<u32>(w(0xb9));
    // call width: 4
    g.call(slot(g, 362), 0, &[x, m, 1, 0]);
    if g.read::<u8>(g.read::<u32>(w(0xc0))) == 0xd {
        let t0 = target_0275(g, img, f, (0xc0, 0xd6), 0);
        // call width: 3
        g.call(slot(g, 278), 0, &[t0, 1, 0]);
    }
    // call width: 2
    g.call(slot(g, 23), 0, &[m, 0]);
    let n = if kind == 8 { 0x12 } else { 0x10 };
    // call width: 3
    g.call(slot(g, 37), 0, &[m, 1, n]);
    for s in [249u32, 10] {
        // call width: 2
        g.call(slot(g, s), 0, &[m, 0]);
    }
    let t = target_0275(g, img, f, (0x136, 0x143), 1);
    // call width: 2
    g.call(slot(g, 6), 0, &[t, 1]);
    // call width: 1
    g.call(slot(g, 548), 0, &[t]);
    // `ecx`: slot 26's address, which the code loaded to call it and
    // nothing after changed here; Ch_ClutCreate does not depend on it
    // (its one use, in 0x7fcc80, is `or ecx, -1`)
    let ecx = slot(g, 26);
    // call width: 1
    g.call(slot(g, 345), ecx, &[m]);
    let x = g.read::<u32>(w(0x170));
    // call width: 2
    g.call(slot(g, 349), 0, &[m, x]);
    let t = target_0275(g, img, f, (0x183, 0x1aa), 1);
    // call width: 5
    g.call(slot(g, 35), 0, &[t, 7, 0, 0, 0]).eax
}

/// [`show_0275`] (450 bytes, 1 overlay).
pub const SHOW_0275: Sig = Sig {
    len: 450,
    hash: 0x6026_f5b2_28d5_d726,
    calls: &[],
    imports: &[],
};

/// `magic_0275`'s summon matrix (0x10004190, 378 bytes; internal, `ecx` =
/// the matrix `X`; an 8-aligned /GS frame `E`: `C` at `E`, `A` at `E +
/// 0x40`, `B` at `E + 0x80`, two more at `E + 0xc0` and `E + 0x100`): for
/// `T = T[sel[1]]` slot 348 `(T, *G_P, 0, -0x80, -0x20, 0)`, slots 355 and
/// 354 `(T, -1)`, slot 562; `A` = slot 723, `B` = slot 724, `C = B X`,
/// `C = A C` (slot 231), the other two units (slot 235); with `sel[0]` 13
/// slots 922 and 931 `(T[0x830], 0)` and 847 `(T[0x830], CB)`. Returns
/// the last slot's `eax`.
pub fn matrix_0275<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, x: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, n: u32| slot_target(g, w(0x3d), n);
    // push ebp; and esp, -8; sub esp, 0x148
    let e = (esp.wrapping_sub(4) & !7).wrapping_sub(0x148);
    let t = target_0275(g, img, f, (0x1b, 0x38), 1);
    let p = g.read::<u32>(w(0x2f));
    // call width: 6
    g.call(slot(g, 348), 0, &[t, p, 0, 0xffff_ff80, 0xffff_ffe0, 0]);
    let t = target_0275(g, img, f, (0x4a, 0x57), 1);
    for s in [355u32, 354] {
        // call width: 2
        g.call(slot(g, s), 0, &[t, u32::MAX]);
    }
    // call width: 0
    g.call(slot(g, 562), 0, &[]);
    let (c, a, b) = (e, e + 0x40, e + 0x80);
    // call width: 1
    g.call(slot(g, 723), 0, &[a]);
    // call width: 1
    g.call(slot(g, 724), 0, &[b]);
    // call width: 3
    g.call(slot(g, 231), 0, &[c, b, x]);
    // call width: 3
    g.call(slot(g, 231), 0, &[c, a, c]);
    for o in [0xc0u32, 0x100] {
        // call width: 1
        g.call(slot(g, 235), 0, &[e + o]);
    }
    // the selection pointer stays in `eax` when the branch is not taken
    let sel = g.read::<u32>(w(0x115));
    if g.read::<u8>(sel) != 0xd {
        return sel;
    }
    let h = g.read::<u32>(t + 0x830);
    for s in [922u32, 931] {
        // call width: 2
        g.call(slot(g, s), 0, &[h, 0]);
    }
    let h = g.read::<u32>(t + 0x830);
    // call width: 2
    g.call(slot(g, 847), 0, &[h, w(0x151)]).eax
}

/// [`matrix_0275`] (378 bytes, 1 overlay).
pub const MATRIX_0275: Sig = Sig {
    len: 378,
    hash: 0x385b_255b_205a_14c1,
    calls: &[
        // __security_check_cookie
        (0x172, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0275`'s start-up (0x100036a0, 849 bytes; internal, no
/// arguments, run by its load step [`load_0275`]; an `ebp` /GS frame: the
/// yaw `ebp - 0x58`, the matrix `M` `ebp - 0x54`, `V` `ebp - 0x14`):
/// five globals set to their immediates (the state `S = *G_S` among
/// them); `*G_H` = slot 719 `(D, 8, 0)`; the part set up
/// ([`part_init`](crate::l5_objhost::part_init)); the yaw `K - slot 333
/// (*G_A)` wrapped once; object 2 started
/// ([`start_object_at`](crate::s4_misc::start_object_at), scale `K1`,
/// owner 0x32); the caster's point `P` (four globals) from slot 262 with
/// `y` its height (slot 12), `w` 1.0; `V = M (0, 0, -6, 1)` with `M` the
/// unit turned by the yaw; `S + 0x10` = `P + V` (`w` too), then `P.y -=
/// K2`; `S + 0x0` 0, `S + 0x4` the yaw plus `K3` wrapped once, `+0x8`,
/// `+0xc`, `+0xe0`, `+0xe8` 0, `+0xe4` 50; the summon shown
/// ([`show_0275`]); `S + 0xdc` = the table word at `sel[0]`, the state
/// task on `S + 0x88` ([`state_task`](crate::s4_misc::state_task),
/// `(CB, that word)`), `S + 0xd0` -1; the actor block
/// ([`me_block`](crate::sa_misc::me_block)); pool nodes
/// ([`take`](crate::sa_pool::take)) for [`task_0275`] and the second task
/// appended to the task list; `*G_SAME` = the node with the same id
/// ([`find_same_id`](crate::o4_single::find_same_id)). Returns 0.
///
/// Object 2's start pushes the `ecx` slot 333 leaves as its third word,
/// which the start does not read.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn start_up_0275<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x11), n);
    let ebp = fr.esp.wrapping_sub(4);
    let (m, v) = (ebp.wrapping_sub(0x54), ebp.wrapping_sub(0x14));
    for (to, imm) in [
        (0x28u32, 0x2cu32),
        (0x32, 0x36),
        (0x3c, 0x40),
        (0x46, 0x4a),
        (0x50, 0x54),
    ] {
        g.write::<u32>(w(to), w(imm));
    }
    // call width: 3
    let h = g.call(slot(g, 719), 0, &[w(0x22), 8, 0]).eax;
    g.write::<u32>(w(0x5e), h);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x63));
    let a = g.read::<u32>(w(0x6e));
    let s = g.read::<u32>(w(0x7a));
    // call width: 1
    let t = st0_f32(g.call(slot(g, 333), 0, &[a]).st0);
    let yaw = wrap(sub(k(g, 0x87), t), k(g, 0x9e), k(g, 0xb3), k(g, 0x94));
    let at = img.rel32_target(f + 0xda);
    crate::s4_misc::start_object_at(g, img, at, w(0xd5), k(g, 0xc9), 2, 0x32, 0);
    let chr = g.read::<u32>(w(0xe5));
    // call width: 4
    g.call(slot(g, 262), 0, &[chr, w(0xfa), w(0xf5), w(0xf0)]);
    // call width: 1
    let y = st0_f32(g.call(slot(g, 12), 0, &[chr]).st0);
    g.write::<f32>(w(0x10e), y);
    g.write::<u32>(w(0x122), ONE);
    g.write::<f32>(w(0x12e), yaw);
    g.write::<u32>(w(0x134), 0xc0c0_0000);
    for (i, x) in (0u32..).zip([0u32, 0, 0xc0c0_0000, ONE]) {
        g.write::<u32>(v + 4 * i, x);
    }
    // call width: 1
    g.call(slot(g, 235), 0, &[m]);
    // call width: 3
    g.call(slot(g, 329), 0, &[m, m, yaw.to_bits()]);
    // call width: 3
    g.call(slot(g, 229), 0, &[v, m, v]);
    for (i, op) in (0u32..).zip([0x19cu32, 0x1a9, 0x1b6, 0x1c3]) {
        let x = g.read::<u32>(w(op));
        g.write::<u32>(s + 0x10 + 4 * i, x);
    }
    for i in 0..3u32 {
        let x = add(g.read::<f32>(s + 0x10 + 4 * i), g.read::<f32>(v + 4 * i));
        g.write::<f32>(s + 0x10 + 4 * i, x);
    }
    let y = sub(k(g, 0x200), k(g, 0x208));
    g.write::<f32>(w(0x210), y);
    let yaw2 = add(yaw, k(g, 0x21d));
    g.write::<u32>(s, 0);
    let yaw2 = wrap(yaw2, k(g, 0x22a), k(g, 0x23e), k(g, 0x234));
    g.write::<f32>(s + 4, yaw2);
    g.write::<u32>(s + 8, 0);
    g.write::<u32>(s + 0xc, 0);
    g.write::<u32>(s + 0xe4, 0x32);
    g.write::<u32>(s + 0xe0, 0);
    g.write::<u32>(s + 0xe8, 0);
    show_0275(g, img, img.rel32_target(f + 0x281));
    let b = u32::from(g.read::<u8>(g.read::<u32>(w(0x286))));
    let word = g.read::<u32>(w(0x290).wrapping_add(4 * b));
    let s2 = g.read::<u32>(w(0x295));
    g.write::<u32>(s2 + 0xdc, word);
    // ebp, the frame, esi/edi, the two words, the call
    let sp = ebp.wrapping_sub(0x58 + 8 + 8);
    g.write::<u32>(sp, w(0x2a1));
    g.write::<u32>(sp + 4, word);
    let task = Frame {
        at: img.rel32_target(f + 0x2ac),
        esp: sp.wrapping_sub(4),
        ecx: s + 0x88,
        ..Frame::default()
    };
    crate::s4_misc::state_task(g, img, &task);
    g.write::<u32>(s + 0xd0, u32::MAX);
    crate::sa_misc::me_block(g, img, img.rel32_target(f + 0x2c0), w(0x2b1));
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x2cd), w(0x2c8));
    let head = g.read::<u32>(w(0x2d3));
    g.write::<u32>(node + 4, 0);
    let head = if head == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x2fb), node);
        node
    } else {
        let last = g.read::<u32>(w(0x2e6));
        g.write::<u32>(node, last);
        g.write::<u32>(last + 4, node);
        head
    };
    g.write::<u32>(node + 8, node);
    let n2 = crate::sa_pool::take(g, img, img.rel32_target(f + 0x308), w(0x300));
    g.write::<u8>(n2 + 0x18, 2);
    g.write::<u32>(n2 + 4, 0);
    if head == 0 {
        g.write::<u32>(n2, 0);
        g.write::<u32>(w(0x329), n2);
    } else {
        g.write::<u32>(n2, node);
        g.write::<u32>(node + 4, n2);
    }
    g.write::<u32>(n2 + 8, n2);
    g.write::<u32>(w(0x331), n2);
    let same = Frame {
        at: img.rel32_target(f + 0x336),
        esp: ebp.wrapping_sub(0x58 + 8 + 4),
        ..Frame::default()
    };
    let r = crate::o4_single::find_same_id(g, img, &same).eax;
    g.write::<u32>(w(0x33f), r);
    Ret::eax(0)
}

/// [`start_up_0275`].
pub const START_UP_0275: Sig = Sig {
    len: 849,
    hash: 0xb5f6_5387_a894_6e19,
    calls: &[
        (0x63, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0xda, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x281, Callee::Sig(&SHOW_0275)),
        (0x2ac, Callee::Sig(&crate::s4_misc::STATE_TASK)),
        (0x2c0, Callee::Sig(&crate::sa_misc::ME_BLOCK)),
        (0x2cd, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x308, Callee::Sig(&crate::sa_pool::TAKE)),
        (
            0x336,
            Callee::Sig(&crate::o4_single::FIND_SAME_ID_0275_4310),
        ),
        // __security_check_cookie
        (0x349, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0275`'s load step (0x10003570, the 118-byte shape) running
/// [`start_up_0275`].
pub fn load_0275<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::r9_load::load_step(g, img, fr, crate::r9_load::Shape::Zero, start_up_0275)
}

/// [`load_0275`].
pub const LOAD_0275: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&START_UP_0275))],
    imports: &[],
};

/// `magic_0275`'s summon task (0x10003a40, 1360 bytes with its jump table;
/// a task callback, cdecl `(node)`; an `ebp` /GS frame: the matrix `M` at
/// `ebp - 0x44`, the three scales `a`, `b`, `c` at `ebp - 0x58`, `- 0x5c`,
/// `- 0x60`), on the state `S = *G_S` and its scale `s = S + 0xdc`:
/// `a = b = c = s`; with `sel[0]` 13 slot 278 `(T[sel[0]], 1, 0)`; the
/// count `n = S + 0xe4` steps down and the state `S + 0xe0` decides:
/// - 0: at `n` 0, state 1 for 60 steps; returns `n`;
/// - 1: while `n` runs `a = b = c = ((1 - n / K) + n / K K') s`; at 0
///   objects 3 (owner 0x55) and 0 (owner 0x5f, `CB`) start
///   ([`start_object_at`](crate::s4_misc::start_object_at)) with the
///   summon's place `S + 0x10` copied (its `y` less `K2`) and `S + 0x4`;
///   the target's point (slot 262) and slot 268 `(T, 6, &L)`'s `y`, `w`
///   1.0; the yaw `K3 - slot 333 (T)` wrapped once; the record `S + 0x54`
///   reset ([`record_reset`](crate::s4_small::record_reset), `(CB2)`) and
///   `*G_R` = it (the `ecx` the reset leaves); state 2 for 90 steps;
/// - 2: at 0 object 1 starts with the place copied again (its `y` less
///   `K4`); state 3 for 40 steps;
/// - 3: while `n` runs `b`, `a`, `c` blend toward the scale by `x = n /
///   K5` (`((1 - x) K + x) s` with their own `K`); at 0 slots 346 and 384
///   `(T[sel[1]])`, `S + 0xe8` 1 and
///   the node goes back ([`give_back`](crate::sa_pool::give_back); its
///   `eax` returned).
///
/// Then (and for states above 3) `M` = slot 210 `(*G_O)` with its rows
/// scaled by `b`, `c`, `a` and its last row the place `S + 0x10`; for
/// `T = T[sel[1]]` slots 33 `(T, x, y, z)`, 35 `(T, 7, b, c, a)` and 34
/// `(T, 1.57)`; [`matrix_0275`] of `M`. Returns its `eax`.
///
/// The objects' starts push the `ecx` left behind as their third word,
/// which the start does not read.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn task_0275<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x4e), n);
    let ebp = fr.esp.wrapping_sub(4);
    let node = fr.arg(g, 0);
    let s = g.read::<u32>(w(0x1f));
    let scale = g.read::<f32>(s + 0xdc);
    let (mut a, mut b, mut c) = (scale, scale, scale);
    if g.read::<u8>(g.read::<u32>(w(0x11))) == 0xd {
        let t = target_0275(g, img, f, (0x11, 0x49), 0);
        // call width: 3
        g.call(slot(g, 278), 0, &[t, 1, 0]);
    }
    let n = g.read::<u32>(s + 0xe4).wrapping_sub(1);
    g.write::<u32>(s + 0xe4, n);
    let state = g.read::<u32>(s + 0xe0);
    let start =
        |g: &mut G, call: u32, obj: u32, scale: u32, (index, owner, owner_b): (u32, u32, u32)| {
            let at = img.rel32_target(f + call);
            crate::s4_misc::start_object_at(g, img, at, w(obj), k(g, scale), index, owner, owner_b);
        };
    let ratio = |g: &G, n: u32, d: u32| {
        #[allow(clippy::cast_precision_loss)] // cvtdq2ps
        let x = n.cast_signed() as f32;
        div(x, k(g, d))
    };
    match state {
        0 => {
            if n == 0 {
                g.write::<u32>(s + 0xe4, 0x3c);
                g.write::<u32>(s + 0xe0, 1);
            }
            return Ret::eax(n);
        }
        1 if n != 0 => {
            let x = ratio(g, n, 0x238);
            let v = mul(add(sub(k(g, 0x230), x), mul(x, k(g, 0x244))), scale);
            (a, b, c) = (v, v, v);
        }
        1 => {
            start(g, 0xcc, 0xc7, 0xbc, (3, 0x55, 0));
            let x = g.read::<u32>(s + 0x10);
            g.write::<u32>(w(0xe1), x);
            let y = g.read::<f32>(s + 0x14);
            g.write::<f32>(w(0xee), y);
            let z = g.read::<u32>(s + 0x18);
            g.write::<u32>(w(0x106), z);
            let ww = g.read::<u32>(s + 0x1c);
            g.write::<u32>(w(0x119), ww);
            g.write::<f32>(w(0x121), sub(y, k(g, 0xfb)));
            let r = g.read::<u32>(s + 4);
            g.write::<u32>(w(0x137), r);
            start(g, 0x13c, 0x12f, 0xd9, (0, 0x5f, w(0x110)));
            let t = target_0275(g, img, f, (0x141, 0x155), 0);
            // call width: 4
            g.call(slot(g, 262), 0, &[t, w(0x15f), w(0x14e), w(0x146)]);
            let l = ebp.wrapping_sub(0x54);
            // call width: 3
            g.call(slot(g, 268), 0, &[t, 6, l]);
            let y = g.read::<u32>(l + 4);
            g.write::<u32>(w(0x18e), y);
            g.write::<u32>(w(0x194), ONE);
            let t = target_0275(g, img, f, (0x186, 0x1a2), 0);
            // call width: 1
            let r = st0_f32(g.call(slot(g, 333), 0, &[t]).st0);
            let yaw = wrap(sub(k(g, 0x1ba), r), k(g, 0x1c9), k(g, 0x1dd), k(g, 0x1d3));
            g.write::<f32>(w(0x1fa), yaw);
            let rec = s + 0x54;
            // the frame, esi/edi, the word, the call
            let sp = ebp.wrapping_sub(0x64 + 8 + 4);
            g.write::<u32>(sp, w(0x1f2));
            let reset = Frame {
                at: img.rel32_target(f + 0x1ff),
                esp: sp.wrapping_sub(4),
                ecx: rec,
                ..Frame::default()
            };
            crate::s4_small::record_reset(g, img, &reset);
            // the `ecx` the reset leaves: the record
            g.write::<u32>(w(0x205), rec);
            g.write::<u32>(s + 0xe4, 0x5a);
            g.write::<u32>(s + 0xe0, 2);
        }
        2 if n == 0 => {
            start(g, 0x282, 0x27d, 0x273, (1, 0, 0));
            for (o, op) in [
                (0x10u32, 0x28fu32),
                (0x14, 0x29c),
                (0x18, 0x2b1),
                (0x1c, 0x2be),
                (4, 0x2cb),
            ] {
                let x = g.read::<u32>(s + o);
                g.write::<u32>(w(op), x);
            }
            let y = sub(g.read::<f32>(s + 0x14), k(g, 0x2a9));
            g.write::<f32>(w(0x2d3), y);
            g.write::<u32>(s + 0xe4, 0x28);
            g.write::<u32>(s + 0xe0, 3);
        }
        3 if n == 0 => {
            // `ecx` at slot 346: the state (3), the jump table's index
            for (sel, tab, sn, ecx) in [(0x2f8u32, 0x303u32, 346u32, 3u32), (0x315, 0x320, 384, 0)]
            {
                let t = target_0275(g, img, f, (sel, tab), 1);
                // call width: 1
                g.call(slot(g, sn), ecx, &[t]);
            }
            g.write::<u32>(s + 0xe8, 1);
            return Ret::eax(crate::sa_pool::give_back(
                g,
                img,
                img.rel32_target(f + 0x341),
                node,
            ));
        }
        3 => {
            let x = ratio(g, n, 0x368);
            let one = k(g, 0x360);
            let u = sub(one, x);
            b = mul(add(mul(u, k(g, 0x377)), x), scale);
            let ua = mul(u, k(g, 0x397));
            let uc = mul(u, k(g, 0x39f));
            a = mul(add(ua, x), scale);
            c = mul(add(uc, x), scale);
        }
        _ => {}
    }
    // the matrix, its rows scaled, its last row the place
    let m = ebp.wrapping_sub(0x44);
    let o = g.read::<u32>(w(0x3cb));
    // call width: 2
    g.call(slot(g, 210), 0, &[o, m]);
    for (row, sc) in [(0u32, b), (0x10, c), (0x20, a)] {
        for i in 0..3u32 {
            let v = mul(g.read::<f32>(m + row + 4 * i), sc);
            g.write::<f32>(m + row + 4 * i, v);
        }
    }
    let p: [u32; 4] = [0u32, 4, 8, 0xc].map(|i| g.read::<u32>(s + 0x10 + i));
    for (i, x) in (0u32..).zip(p) {
        g.write::<u32>(m + 0x30 + 4 * i, x);
    }
    let t = target_0275(g, img, f, (0x3fe, 0x4a9), 1);
    // call width: 4
    g.call(slot(g, 33), 0, &[t, p[0], p[1], p[2]]);
    let t = target_0275(g, img, f, (0x4c5, 0x4eb), 1);
    // call width: 5
    g.call(
        slot(g, 35),
        0,
        &[t, 7, b.to_bits(), c.to_bits(), a.to_bits()],
    );
    let t = target_0275(g, img, f, (0x4fd, 0x512), 1);
    // call width: 2
    g.call(slot(g, 34), 0, &[t, 0x3fc8_f5c3]);
    // the frame, esi/edi, the call
    let mx = img.rel32_target(f + 0x52a);
    Ret::eax(matrix_0275(g, img, mx, ebp.wrapping_sub(0x70), m))
}

/// [`task_0275`] (1360 bytes with its jump table, 1 overlay).
pub const TASK_0275: Sig = Sig {
    len: 1360,
    hash: 0x7776_45ff_ad65_9ba7,
    calls: &[
        // __security_check_cookie
        (0xa8, Callee::Any),
        (0xcc, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x13c, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x1ff, Callee::Sig(&crate::s4_small::RECORD_RESET)),
        (0x282, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x341, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x34d, Callee::Any),
        (0x52a, Callee::Sig(&MATRIX_0275)),
        (0x536, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0270`'s spark burst (0x100018a0, 618 bytes; a task callback,
/// cdecl `(node)`; an `ebp` /GS frame: the caster's point `P` `ebp -
/// 0x14`, `V` `ebp - 0x24`, `W` `ebp - 0x34`, `M` `ebp - 0x74`). The node's
/// count `+0x14` steps down; at 0 or below the node goes back to the pool
/// ([`give_back`](crate::sa_pool::give_back), its `eax` returned). Else
/// `P` = slot 262 of the caster with `y` its height (slot 12) less `K`,
/// `w` 1.0, and six records of the ring
/// ([`ring_take`](crate::sa_misc::ring_take); none: returns 0) each get:
/// `+0x0` slot 719 `(D, 9, 0)`, the halfwords `+0x8` -1 and `+0xa` 0x200,
/// `+0x4` 1; `Q = +0xc` = `P` plus `M (0, 0, rnd 10 + K1, 1)` with `M` the
/// unit turned by rnd 2 pi about Y (slots 235, 328 `(0)`, 329, 229, 248),
/// `Q.w` 1.0; `+0x1c`..`+0x24` 0, `+0x28` rnd 2 pi, `+0x2c` `(slot 404
/// (0.5) + K2) * K3`, `+0x34` -1, `+0x38` 0, the byte `+0x47` 0x10.
/// Returns the last slot 404's `eax`.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn burst_0270<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x47), n);
    let ebp = fr.esp.wrapping_sub(4);
    let at = |o: u32| ebp.wrapping_sub(o);
    let node = fr.arg(g, 0);
    let n = g.read::<i32>(node + 0x14).wrapping_sub(1);
    g.write::<i32>(node + 0x14, n);
    if n <= 0 {
        return Ret::eax(crate::sa_pool::give_back(
            g,
            img,
            img.rel32_target(f + 0x20),
            node,
        ));
    }
    let (p, v, wv, m) = (at(0x14), at(0x24), at(0x34), at(0x74));
    let chr = g.read::<u32>(w(0x36));
    // call width: 4
    g.call(slot(g, 262), 0, &[chr, p, p + 4, p + 8]);
    // call width: 1
    let y = st0_f32(g.call(slot(g, 12), 0, &[chr]).st0);
    let y = sub(y, k(g, 0x6b));
    g.write::<u32>(p + 0xc, ONE);
    g.write::<u32>(v, 0);
    g.write::<f32>(p + 4, y);
    g.write::<u32>(v + 4, 0);
    g.write::<u32>(v + 0xc, ONE);
    let take = img.rel32_target(f + 0xa1);
    let mut eax = 0;
    for _ in 0..6 {
        let rec = crate::sa_misc::ring_take(g, img, take);
        if rec == 0 {
            return Ret::eax(0);
        }
        // call width: 3
        let h = g.call(slot(g, 719), 0, &[w(0xc0), 9, 0]).eax;
        g.write::<u32>(rec, h);
        g.write::<u16>(rec + 8, 0xffff);
        g.write::<u32>(rec + 4, 1);
        g.write::<u16>(rec + 0xa, 0x200);
        let q = rec + 0xc;
        for i in 0..4u32 {
            let x = g.read::<u32>(p + 4 * i);
            g.write::<u32>(q + 4 * i, x);
        }
        // call width: 1
        let a = st0_f32(g.call(slot(g, 314), 0, &[0x40c9_0fdb]).st0);
        // call width: 1
        let r = st0_f32(g.call(slot(g, 314), 0, &[0x4120_0000]).st0);
        g.write::<f32>(v + 8, add(r, k(g, 0x143)));
        // call width: 1
        g.call(slot(g, 235), 0, &[m]);
        // call width: 3
        g.call(slot(g, 328), 0, &[m, m, 0]);
        // call width: 3
        g.call(slot(g, 329), 0, &[m, m, a.to_bits()]);
        g.write::<u32>(v + 0xc, ONE);
        // call width: 3
        g.call(slot(g, 229), 0, &[wv, m, v]);
        // call width: 3
        g.call(slot(g, 248), 0, &[q, q, wv]);
        g.write::<u32>(q + 0xc, ONE);
        for o in [0x1cu32, 0x20, 0x24] {
            g.write::<u32>(rec + o, 0);
        }
        // call width: 1
        let r = st0_f32(g.call(slot(g, 314), 0, &[0x40c9_0fdb]).st0);
        g.write::<f32>(rec + 0x28, r);
        // call width: 1
        let out = g.call(slot(g, 404), 0, &[0x3f00_0000]);
        eax = out.eax;
        let s = add(st0_f32(out.st0), k(g, 0x224));
        g.write::<u32>(rec + 0x34, u32::MAX);
        g.write::<u32>(rec + 0x38, 0);
        g.write::<u8>(rec + 0x47, 0x10);
        g.write::<f32>(rec + 0x2c, mul(s, k(g, 0x23e)));
    }
    Ret::eax(eax)
}

/// [`burst_0270`] (618 bytes, 1 overlay).
pub const BURST_0270: Sig = Sig {
    len: 618,
    hash: 0x5247_4422_c5af_c956,
    calls: &[
        (0x20, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        // __security_check_cookie
        (0x2a, Callee::Any),
        (0xa1, Callee::Sig(&crate::sa_misc::RING_TAKE)),
        (0x262, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0270`'s drop (0x10001460, 1078 bytes; a task callback, cdecl
/// `(node)`; an `ebp` /GS frame: the point `P` `ebp - 0x24`, `L` `ebp -
/// 0x44`, `Q` `ebp - 0x34`, `V` `ebp - 0x14`, the matrices `A` `ebp -
/// 0x84` and `B` `ebp - 0xc4`, the yaw `ebp - 0xc8`). The record `R =
/// node + 0x14`'s count `R + 4` steps down; at 0 or below the node goes
/// back to the pool (the give-back's `eax` returned). Else for the actor
/// `R + 0` (-1: the caster, else `T[sel[i]]`): `P` = slot 262 with `y` the
/// `y` of slot 268 `(actor, 6, &L)`, `w` 1.0; the yaw `K - slot 333
/// (caster)` wrapped once; `P += A (0, 0, 2, 1)` and `Q = (0, 0, 0, 1) + B
/// (0, 0, -0.2, 1)` with `A`, `B` the unit turned by the yaw (slots 235,
/// 328 `(0)`, 329, 229, 248). One record of the ring
/// ([`ring_take`](crate::sa_misc::ring_take); none: returns 0) gets `+0x0`
/// slot 719 `(D, 9, 1)`, the halfwords `+0x8` -1 and `+0xa` 0x200, `+0x4`
/// 1, the place `P` plus rnd 3, rnd 6, rnd 3 (slot 314), `w` 1.0,
/// `+0x1c` `(slot 404 (0.5) + K1) Q.x`, `+0x20` rnd 0.1, `+0x24` `(slot
/// 404 (0.5) + K1) Q.z`, `+0x28` rnd 2 pi, `+0x2c` `(slot 404 (1) + K1)
/// K2`, `+0x34` -1, `+0x38` 0. Returns the last slot 404's `eax`.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn drop_0270<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x62), n);
    let ebp = fr.esp.wrapping_sub(4);
    let at = |o: u32| ebp.wrapping_sub(o);
    let node = fr.arg(g, 0);
    let r = g.read::<u32>(node + 0x14);
    let n = g.read::<i32>(r + 4).wrapping_sub(1);
    g.write::<i32>(r + 4, n);
    if n <= 0 {
        return Ret::eax(crate::sa_pool::give_back(
            g,
            img,
            img.rel32_target(f + 0x23),
            node,
        ));
    }
    let (p, l, q, v, am, bm) = (at(0x24), at(0x44), at(0x34), at(0x14), at(0x84), at(0xc4));
    let idx = g.read::<u32>(r);
    let actor = if idx == u32::MAX {
        g.read::<u32>(w(0x3f))
    } else {
        let b = u32::from(g.read::<u8>(g.read::<u32>(w(0x46)).wrapping_add(idx)));
        g.read::<u32>(w(0x51).wrapping_add(4 * b))
    };
    // call width: 4
    g.call(slot(g, 262), 0, &[actor, p, p + 4, p + 8]);
    // call width: 3
    g.call(slot(g, 268), 0, &[actor, 6, l]);
    let y = g.read::<u32>(l + 4);
    let caster = g.read::<u32>(w(0x8f));
    g.write::<u32>(p + 4, y);
    g.write::<u32>(p + 0xc, ONE);
    // call width: 1
    let t = st0_f32(g.call(slot(g, 333), 0, &[caster]).st0);
    let yaw = wrap(sub(k(g, 0xb1), t), k(g, 0xcb), k(g, 0xe3), k(g, 0xc1));
    for (i, x) in (0u32..).zip([0u32, 0, 0x4000_0000, ONE]) {
        g.write::<u32>(v + 4 * i, x);
    }
    let turned = |g: &mut G, m: u32| {
        // call width: 1
        g.call(slot(g, 235), 0, &[m]);
        // call width: 3
        g.call(slot(g, 328), 0, &[m, m, 0]);
        // call width: 3
        g.call(slot(g, 329), 0, &[m, m, yaw.to_bits()]);
    };
    turned(g, am);
    g.write::<u32>(v + 0xc, ONE);
    // call width: 3
    g.call(slot(g, 229), 0, &[l, am, v]);
    // call width: 3
    g.call(slot(g, 248), 0, &[p, p, l]);
    g.write::<u32>(p + 0xc, ONE);
    for (i, x) in (0u32..).zip([0u32, 0, 0, ONE]) {
        g.write::<u32>(q + 4 * i, x);
    }
    for (i, x) in (0u32..).zip([0u32, 0, 0xbe4c_cccd, ONE]) {
        g.write::<u32>(v + 4 * i, x);
    }
    turned(g, bm);
    g.write::<u32>(v + 0xc, ONE);
    // call width: 3
    g.call(slot(g, 229), 0, &[l, bm, v]);
    // call width: 3
    g.call(slot(g, 248), 0, &[q, q, l]);
    g.write::<u32>(q + 0xc, ONE);
    let rec = crate::sa_misc::ring_take(g, img, img.rel32_target(f + 0x27d));
    if rec == 0 {
        return Ret::eax(0);
    }
    // call width: 3
    let h = g.call(slot(g, 719), 0, &[w(0x29c), 9, 1]).eax;
    g.write::<u32>(rec, h);
    g.write::<u16>(rec + 8, 0xffff);
    g.write::<u16>(rec + 0xa, 0x200);
    g.write::<u32>(rec + 4, 1);
    let rnd = |g: &mut G, n: u32, x: u32| {
        // call width: 1
        let out = g.call(slot(g, n), 0, &[x]);
        (st0_f32(out.st0), out.eax)
    };
    let rf = |g: &G, a: u32| g.read::<f32>(a);
    let (r1, _) = rnd(g, 314, 0x4040_0000);
    g.write::<f32>(rec + 0xc, add(rf(g, p), r1));
    let (r2, _) = rnd(g, 314, 0x40c0_0000);
    g.write::<f32>(rec + 0x10, add(rf(g, p + 4), r2));
    let (r3, _) = rnd(g, 314, 0x4040_0000);
    g.write::<u32>(rec + 0x18, ONE);
    g.write::<f32>(rec + 0x14, add(rf(g, p + 8), r3));
    let (s, _) = rnd(g, 404, 0x3f00_0000);
    g.write::<f32>(rec + 0x1c, mul(add(s, k(g, 0x36f)), rf(g, q)));
    let (s, _) = rnd(g, 314, 0x3dcc_cccd);
    g.write::<f32>(rec + 0x20, s);
    let (s, _) = rnd(g, 404, 0x3f00_0000);
    g.write::<f32>(rec + 0x24, mul(add(s, k(g, 0x3ba)), rf(g, q + 8)));
    let (s, _) = rnd(g, 314, 0x40c9_0fdb);
    g.write::<f32>(rec + 0x28, s);
    let (s, eax) = rnd(g, 404, ONE);
    g.write::<u32>(rec + 0x34, u32::MAX);
    g.write::<u32>(rec + 0x38, 0);
    g.write::<f32>(rec + 0x2c, mul(add(s, k(g, 0x405)), k(g, 0x41e)));
    Ret::eax(eax)
}

/// [`drop_0270`] (1078 bytes, 1 overlay).
pub const DROP_0270: Sig = Sig {
    len: 1078,
    hash: 0xd77b_8d09_893a_dd9e,
    calls: &[
        (0x23, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        // __security_check_cookie
        (0x2d, Callee::Any),
        (0x27d, Callee::Sig(&crate::sa_misc::RING_TAKE)),
        (0x42e, Callee::Any),
    ],
    imports: &[],
};

/// The `atan2(dz, dx)` of `to - from` (`x` and `z` of two points),
/// through `_CIatan2` as the original's x87 hand-off, rounded to `f32`.
fn heading<G: Game>(g: &mut G, to: u32, from: u32) -> f32 {
    let dz = sub(g.read::<f32>(to + 8), g.read::<f32>(from + 8));
    let dx = sub(g.read::<f32>(to), g.read::<f32>(from));
    #[allow(clippy::cast_possible_truncation)] // cvtpd2ps
    let a = g.atan2(f64::from(dz), f64::from(dx)) as f32;
    a
}

/// `magic_0270`'s aim (0x10001cc0, 633 bytes; internal, no arguments; an
/// 8-aligned /GS frame `E`: the caster's point `C` at `E + 0x20`, the
/// targets' `Q` at `E + 0xc`). `C` = slot 262 of the caster with `y` its
/// height (slot 12), `w` 1.0. By the mode `*G_MODE`: 1 and 3 pick the
/// colour `D1` / `D3`; 2 the angle between the first and second target
/// as seen from the caster: each target's point (slots 262, 12; the
/// second's height dropped), `K - atan2(dz, dx)` wrapped once, `d = |(a2
/// K') / K'' - (a1 K') / K''|` (truncated, folded above 180 to `360 -
/// d`), `D2a` when `d` is at most 25, else `D2b`. The picked colour goes
/// to `*G_C`, `*G_W` 0, the halfword `*G_H` 0 and the byte `*G_B` 1. Other
/// modes store nothing. Returns `d` (mode 2), 0 (modes 1, 3) or the mode
/// less 3.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn aim_0270<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32) -> u32 {
    use crate::sb_sse::cvtt;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x2b), n);
    // push ebp; and esp, -8; sub esp, 0x34
    let e = (esp.wrapping_sub(4) & !7).wrapping_sub(0x34);
    let (c, q) = (e + 0x20, e + 0xc);
    let chr = g.read::<u32>(w(0x17));
    // call width: 4
    g.call(slot(g, 262), 0, &[chr, c, c + 4, c + 8]);
    // call width: 1
    let y = st0_f32(g.call(slot(g, 12), 0, &[chr]).st0);
    g.write::<f32>(c + 4, y);
    let mode = g.read::<u32>(w(0x48));
    g.write::<u32>(c + 0xc, ONE);
    let (colour, eax) = match mode {
        1 => (w(0x245), 0),
        3 => (w(0x69), 0),
        2 => {
            let target = |g: &G, sel: u32, tab: u32, i: u32| {
                let b = u32::from(g.read::<u8>(g.read::<u32>(w(sel)).wrapping_add(i)));
                g.read::<u32>(w(tab).wrapping_add(4 * b))
            };
            let t0 = target(g, 0x73, 0x7d, 0);
            // call width: 4
            g.call(slot(g, 262), 0, &[t0, q, q + 4, q + 8]);
            // call width: 1
            let y = st0_f32(g.call(slot(g, 12), 0, &[t0]).st0);
            g.write::<f32>(q + 4, y);
            let a = heading(g, q, c);
            g.write::<u32>(q + 0xc, ONE);
            let x0 = k(g, 0x10d);
            let a1 = wrap(sub(k(g, 0xfd), a), k(g, 0x114), k(g, 0x12a), x0);
            let t1 = target(g, 0x13e, 0x149, 1);
            // call width: 4
            g.call(slot(g, 262), 0, &[t1, q, q + 4, q + 8]);
            // call width: 1 (the height is dropped)
            g.call(slot(g, 12), 0, &[t1]);
            let b = heading(g, q, c);
            let k3 = k(g, 0x1c7);
            let x1 = sub(k(g, 0x1bf), b);
            let a2 = if x1 > k3 {
                sub(x1, k(g, 0x1dc))
            } else if k(g, 0x1e6) > x1 {
                add(x1, k(g, 0x1f3))
            } else {
                x1
            };
            let d = sub(div(mul(a2, k(g, 0x201)), k3), div(mul(a1, k(g, 0x209)), k3));
            let d = cvtt(d).wrapping_abs();
            let d = if d > 0xb4 { 0x168 - d } else { d };
            let colour = if d <= 0x19 { w(0x238) } else { w(0x233) };
            (colour, d.cast_unsigned())
        }
        _ => return mode.wrapping_sub(3),
    };
    g.write::<u32>(w(0x24b), colour);
    g.write::<u32>(w(0x251), 0);
    g.write::<u16>(w(0x25c), 0);
    g.write::<u8>(w(0x264), 1);
    eax
}

/// [`aim_0270`] (633 bytes, 1 overlay).
pub const AIM_0270: Sig = Sig {
    len: 633,
    hash: 0x4d0d_2a79_c452_4d19,
    calls: &[
        (0xeb, Callee::Import("_CIatan2")),
        (0x1ad, Callee::Import("_CIatan2")),
        // __security_check_cookie
        (0x271, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0270`'s start-up (0x10001f40, 1058 bytes; internal, no
/// arguments, run by its load step [`load_0270`]; no frame): three
/// globals set to their immediates; the block filled
/// ([`obj_block_fill`](crate::l5_objhost::obj_block_fill), `arg` =
/// `*G_A`, the bytes slot 407 `(*G_A, &G_N, 0)` gives); `*G_H` = slot 719
/// `(D, 8, 0)`; the part set up ([`part_init`](crate::l5_objhost::part_init));
/// three objects started ([`start_object_anim`](crate::s4_misc::start_object_anim):
/// 0 with 0x10, 1 with 0x7a, 2 with 0x10), each at the caster's point
/// (slot 262; `y` less a constant, a second point's `y` less another, `w`
/// 1.0, the callback) and object 1's record of nine globals; the caster's
/// point with its height for the third and the colour bytes; the counted
/// task ([`counted_task`](crate::s4_small::counted_task)), two fades
/// ([`fade_task`](crate::s3_misc::fade_task)), slots 271 `(4)` and 537
/// `(*G_M, 2, 0, sel[0], 2, 12)`; the actor record (slot 215 or, with
/// `*G_F`, 413 of `*G_M`; the side 0 or 1); the aim ([`aim_0270`]); pool
/// nodes ([`take`](crate::sa_pool::take)) for two tasks around the pool's
/// reset ([`pool_reset_take`](crate::o6_fam_pool::pool_reset_take)),
/// appended to the task list. Returns 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn start_up_0270<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x01), n);
    let a = g.read::<u32>(w(0x08));
    for (to, imm) in [(0x1cu32, 0x20u32), (0x26, 0x2a), (0x30, 0x34)] {
        g.write::<u32>(w(to), w(imm));
    }
    // call width: 3
    let bytes = g.call(slot(g, 407), 0, &[a, w(0x15), 0]).eax;
    crate::l5_objhost::obj_block_fill(g, img, img.rel32_target(f + 0x43), a, w(0x3c), bytes);
    // call width: 3
    let h = g.call(slot(g, 719), 0, &[w(0x57), 8, 0]).eax;
    g.write::<u32>(w(0x61), h);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x66));
    let start = |g: &mut G, call: u32, obj: u32, scale: u32, (index, value): (u32, u32)| {
        let at = img.rel32_target(f + call);
        crate::s4_misc::start_object_anim(g, img, at, w(obj), k(g, scale), index, value);
    };
    start(g, 0x7c, 0x77, 0x6e, (0, 0x10));
    let chr = g.read::<u32>(w(0x9c));
    // call width: 4
    g.call(slot(g, 262), 0, &[chr, w(0x96), w(0x91), w(0x86)]);
    let y = sub(k(g, 0xa6), k(g, 0xae));
    g.write::<f32>(w(0xc5), y);
    let y2 = sub(k(g, 0xcd), k(g, 0xd5));
    g.write::<u32>(w(0xe2), ONE);
    g.write::<u32>(w(0xec), w(0xf0));
    g.write::<f32>(w(0xf8), y2);
    for o in [0xfeu32, 0x108, 0x112, 0x11c] {
        g.write::<u32>(w(o), 0);
    }
    g.write::<u32>(w(0x126), w(0x12a));
    g.write::<u32>(w(0x130), w(0x134));
    g.write::<u32>(w(0x13a), 0);
    g.write::<u32>(w(0x144), 0);
    g.write::<u16>(w(0x14e), 0);
    g.write::<u32>(w(0x154), w(0x158));
    start(g, 0x15d, 0xdc, 0xb6, (1, 0x7a));
    let chr = g.read::<u32>(w(0x17d));
    // call width: 4
    g.call(slot(g, 262), 0, &[chr, w(0x177), w(0x172), w(0x167)]);
    let y = sub(k(g, 0x187), k(g, 0x18f));
    g.write::<u32>(w(0x1a0), ONE);
    g.write::<f32>(w(0x1ae), y);
    let y2 = sub(k(g, 0x1b6), k(g, 0x1be));
    g.write::<u32>(w(0x1c4), w(0x1c8));
    g.write::<f32>(w(0x1d7), y2);
    start(g, 0x1dc, 0x1cf, 0x197, (2, 0x10));
    let chr = g.read::<u32>(w(0x1e7));
    // call width: 4
    g.call(slot(g, 262), 0, &[chr, w(0x1fc), w(0x1f7), w(0x1f2)]);
    // call width: 1
    let y = st0_f32(g.call(slot(g, 12), 0, &[chr]).st0);
    g.write::<f32>(w(0x210), y);
    let y = sub(k(g, 0x218), k(g, 0x220));
    g.write::<u32>(w(0x226), ONE);
    g.write::<u32>(w(0x230), w(0x234));
    g.write::<u32>(w(0x23a), 0x0080_ffff);
    g.write::<u8>(w(0x244), 0xff);
    g.write::<f32>(w(0x24d), y);
    let y2 = sub(k(g, 0x255), k(g, 0x25d));
    g.write::<u16>(w(0x264), 0);
    g.write::<u8>(w(0x26c), 0);
    g.write::<u32>(w(0x273), 0x79);
    g.write::<u16>(w(0x27e), 0x48);
    g.write::<f32>(w(0x288), y2);
    let counted = Frame {
        at: img.rel32_target(f + 0x28d),
        esp: fr.esp.wrapping_sub(4 + 4 * 9 + 4),
        ..Frame::default()
    };
    crate::s4_small::counted_task(g, img, &counted);
    // the fades: their word on the stack (esi, nine words the calls left,
    // then each word; the call)
    for (i, (call, ecx, edx, x1, x2, word)) in (0u32..).zip([
        (0x2aeu32, w(0x2a9), 0x20u32, 0x29du32, 0x295u32, 0x20u32),
        (0x2d2, w(0x2cd), 0x10, 0x2be, 0x2b6, 0xdc),
    ]) {
        let sp = fr.esp.wrapping_sub(4 + 4 * 10 + 4 * i);
        g.write::<u32>(sp, word);
        let fade = Frame {
            at: img.rel32_target(f + call),
            esp: sp.wrapping_sub(4),
            eax: 0,
            ecx,
            edx,
            xmm: [
                0,
                crate::Xmm::f32(k(g, x1)).reg(),
                crate::Xmm::f32(k(g, x2)).reg(),
                0,
            ],
        };
        crate::s3_misc::fade_task(g, img, &fade);
    }
    // call width: 1
    g.call(slot(g, 271), 0, &[4]);
    let b = u32::from(g.read::<u8>(g.read::<u32>(w(0x2e6))));
    let mm = g.read::<u32>(w(0x303));
    // call width: 6
    g.call(slot(g, 537), 0, &[mm, 2, 0, b, 2, 0xc]);
    let n = g.read::<u32>(w(0x30a));
    g.write::<u32>(w(0x31a), n);
    g.write::<u32>(w(0x326), w(0x32a));
    g.write::<u32>(w(0x330), u32::MAX);
    let s = if g.read::<u32>(w(0x313)) == 0 {
        215
    } else {
        413
    };
    // call width: 1
    let r = g.call(slot(g, s), 0, &[n]).eax;
    g.write::<u32>(w(0x35a), r);
    g.write::<u32>(w(0x360), u32::MAX);
    g.write::<u32>(w(0x36a), 0);
    g.write::<u32>(w(0x374), 1);
    if g.read::<u32>(w(0x34f)) == 0 {
        g.write::<u16>(w(0x37e), 0);
    } else {
        g.write::<u16>(w(0x387), 1);
    }
    aim_0270(g, img, img.rel32_target(f + 0x38c), fr.esp.wrapping_sub(4));
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x396), w(0x391));
    g.write::<u8>(node + 0x18, 1);
    g.write::<u32>(node + 4, 0);
    if g.read::<u32>(w(0x39c)) == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x3c2), node);
    } else {
        let last = g.read::<u32>(w(0x3b0));
        g.write::<u32>(node, last);
        g.write::<u32>(last + 4, node);
    }
    g.write::<u32>(node + 8, node);
    g.write::<u32>(w(0x3cf), node);
    crate::o6_fam_pool::pool_reset_take(g, img, img.rel32_target(f + 0x3d4), w(0x3c7));
    let n2 = crate::sa_pool::take(g, img, img.rel32_target(f + 0x3de), w(0x3d9));
    g.write::<u8>(n2 + 0x18, 2);
    g.write::<u32>(n2 + 4, 0);
    if g.read::<u32>(w(0x3e4)) == 0 {
        g.write::<u32>(n2, 0);
        g.write::<u32>(w(0x413), n2);
        g.write::<u32>(n2 + 8, n2);
        g.write::<u32>(w(0x41b), n2);
    } else {
        let last = g.read::<u32>(w(0x3f8));
        g.write::<u32>(n2, last);
        g.write::<u32>(last + 4, n2);
        g.write::<u32>(n2 + 8, n2);
        g.write::<u32>(w(0x405), n2);
    }
    Ret::eax(0)
}

/// [`start_up_0270`].
pub const START_UP_0270: Sig = Sig {
    len: 1058,
    hash: 0xdf5f_3dc5_e0b8_d95e,
    calls: &[
        (0x43, Callee::Sig(&crate::l5_objhost::OBJ_BLOCK_FILL)),
        (0x66, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x7c, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x15d, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x1dc, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x28d, Callee::Sig(&crate::s4_small::COUNTED_TASK_16)),
        (0x2ae, Callee::Sig(&crate::s3_misc::FADE_TASK)),
        (0x2d2, Callee::Sig(&crate::s3_misc::FADE_TASK)),
        (0x38c, Callee::Sig(&AIM_0270)),
        (0x396, Callee::Sig(&crate::sa_pool::TAKE)),
        (
            0x3d4,
            Callee::Sig(&crate::o6_fam_pool::POOL_RESET_TAKE_0243_2800),
        ),
        (0x3de, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// `magic_0270`'s load step (0x100012b0, the 118-byte shape) running
/// [`start_up_0270`].
pub fn load_0270<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::r9_load::load_step(g, img, fr, crate::r9_load::Shape::Zero, start_up_0270)
}

/// [`load_0270`].
pub const LOAD_0270: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&START_UP_0270))],
    imports: &[],
};

/// `magic_0380`'s bone matrix (0x10001b60, 308 bytes; internal, `ecx` =
/// the matrix out `X`; an `ebp` /GS frame: `A` `ebp - 0x44`, `B` `ebp -
/// 0x84`): `A` = slot 236 `(*G_T, 6)` (the target's bone 6) with its
/// three rows' first three words times `K / slot 277 (*G_T)` (ST0), `B`
/// the unit turned by `-pi/2` (slots 235, 329), `X = A B` (slot 231).
/// Returns slot 231's `eax`.
pub fn bone_matrix_0380<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, x: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, n: u32| slot_target(g, w(0x19), n);
    let ebp = esp.wrapping_sub(4);
    let (a, b) = (ebp.wrapping_sub(0x44), ebp.wrapping_sub(0x84));
    let t = g.read::<u32>(w(0x21));
    // call width: 3
    g.call(slot(g, 236), 0, &[t, 6, a]);
    let t = g.read::<u32>(w(0x36));
    // call width: 1
    let s = st0_f32(g.call(slot(g, 277), 0, &[t]).st0);
    let r = div(g.read::<f32>(w(0x4c)), s);
    for row in [0u32, 0x10, 0x20] {
        for i in 0..3u32 {
            let v = mul(g.read::<f32>(a + row + 4 * i), r);
            g.write::<f32>(a + row + 4 * i, v);
        }
    }
    // call width: 1
    g.call(slot(g, 235), 0, &[b]);
    // call width: 3
    g.call(slot(g, 329), 0, &[b, b, 0xbfc9_0fdb]);
    // call width: 3
    g.call(slot(g, 231), 0, &[x, a, b]).eax
}

/// [`bone_matrix_0380`] (308 bytes, 1 overlay).
pub const BONE_MATRIX_0380: Sig = Sig {
    len: 308,
    hash: 0x32c3_8a91_5f33_658d,
    calls: &[
        // __security_check_cookie
        (0x12c, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0380`'s object at the bone (0x10001a90, 125 bytes; cdecl
/// `(obj)`; an `ebp` /GS frame, `P` at `ebp - 0x14`): the object's matrix
/// `obj + 0x80` ([`bone_matrix_0380`]); slot 237 `(*G_T, 6, &(500, 0, 0,
/// 1), obj + 0xd0)` with `obj + 0xdc` 1.0 after; the byte `obj + 0x126`
/// 2. Returns slot 237's `eax`.
pub fn at_bone_0380<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, n: u32| slot_target(g, w(0x2c), n);
    let ebp = fr.esp.wrapping_sub(4);
    let obj = fr.arg(g, 0);
    // ebp, the frame, esi/edi, the call
    let callee = img.rel32_target(f + 0x1c);
    bone_matrix_0380(g, img, callee, ebp.wrapping_sub(0x14 + 8 + 4), obj + 0x80);
    let p = ebp.wrapping_sub(0x14);
    for (i, v) in (0u32..).zip([0x43fa_0000u32, 0, 0, ONE]) {
        g.write::<u32>(p + 4 * i, v);
    }
    let t = g.read::<u32>(w(0x34));
    let out = obj + 0xd0;
    // call width: 4
    let eax = g.call(slot(g, 237), 0, &[t, 6, p, out]).eax;
    g.write::<u32>(out + 0xc, ONE);
    g.write::<u8>(obj + 0x126, 2);
    Ret::eax(eax)
}

/// [`at_bone_0380`] (125 bytes, 1 overlay).
pub const AT_BONE_0380: Sig = Sig {
    len: 125,
    hash: 0x002c_42c1_5524_3134,
    calls: &[
        (0x1c, Callee::Sig(&BONE_MATRIX_0380)),
        // __security_check_cookie
        (0x75, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0380`'s facing test (0x10001800, 543 bytes; internal, `ecx` =
/// an actor's index `i`, -1 for the caster; an 8-aligned /GS frame `E`:
/// `M` `E + 0x3c`, `V` `E + 0xc` (later the actor's point), `R` `E +
/// 0x1c`, `P` `E + 0x2c`): `M` ([`bone_matrix_0380`]), `V = M (0, 0,
/// -1000, 1)` (slot 229), `R` = slot 237 `(*G_T, 6, &(500, 0, 0, 1))`; `a =
/// K - atan2(V.z - R.z, V.x - R.x)` wrapped once; the actor's point (slot
/// 262; the caster `*G_A` or `T[sel[i]]`) gives `b` the same way. Returns
/// 1 when `|a - b|` (as `f64`) is below `K'`, else 0.
pub fn facing_0380<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, i: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x36), n);
    // push ebp; and esp, -8; sub esp, 0x84
    let e = (esp.wrapping_sub(4) & !7).wrapping_sub(0x84);
    let (m, v, r, p) = (e + 0x3c, e + 0xc, e + 0x1c, e + 0x2c);
    // esi pushed, the call
    bone_matrix_0380(g, img, img.rel32_target(f + 0x22), e.wrapping_sub(8), m);
    for (j, x) in (0u32..).zip([0u32, 0, 0xc47a_0000, ONE]) {
        g.write::<u32>(v + 4 * j, x);
    }
    // call width: 3
    g.call(slot(g, 229), 0, &[v, m, v]);
    for (j, x) in (0u32..).zip([0x43fa_0000u32, 0, 0, ONE]) {
        g.write::<u32>(p + 4 * j, x);
    }
    let t = g.read::<u32>(w(0x75));
    // call width: 4
    g.call(slot(g, 237), 0, &[t, 6, p, r]);
    let a = heading(g, v, r);
    g.write::<u32>(r + 0xc, ONE);
    let a = wrap(sub(k(g, 0xf1), a), k(g, 0x108), k(g, 0x11e), k(g, 0x101));
    let actor = if i == u32::MAX {
        g.read::<u32>(w(0x137))
    } else {
        let b = u32::from(g.read::<u8>(g.read::<u32>(w(0x13e)).wrapping_add(i)));
        g.read::<u32>(w(0x149).wrapping_add(4 * b))
    };
    let q = v;
    // call width: 4
    g.call(slot(g, 262), 0, &[actor, q, q + 4, q + 8]);
    let b = heading(g, q, r);
    let b = wrap(sub(k(g, 0x1b2), b), k(g, 0x1c1), k(g, 0x1d5), k(g, 0x1cb));
    let d = f64::from(sub(a, b));
    let mask = g.read::<u64>(w(0x209));
    let d = f64::from_bits(d.to_bits() & mask);
    u32::from(g.read::<f64>(w(0x201)) > d)
}

/// [`facing_0380`] (543 bytes, 1 overlay).
pub const FACING_0380: Sig = Sig {
    len: 543,
    hash: 0xfbab_2e7a_020b_94d9,
    calls: &[
        (0x22, Callee::Sig(&BONE_MATRIX_0380)),
        (0xdf, Callee::Import("_CIatan2")),
        (0x1a0, Callee::Import("_CIatan2")),
        // __security_check_cookie
        (0x217, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0380`'s actor waits (0x10001770, 132 bytes; a task callback, no
/// arguments read): for each of the `*G_N` actors' records `R` (0xc
/// bytes from `G_R`: a delay, a count, a flag): a running delay counts
/// down; else until the flag is set the actor's facing test
/// ([`facing_0380`]) runs and sets it on 1; then the count runs down and
/// at 0 slot 239 `(*G_A, sel[i], 0)`. Returns the last value it left in
/// `eax` (the entry's `eax` with no actors).
pub fn waits_0380<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let mut eax = fr.eax;
    let mut i = 0u32;
    let mut r = w(0x13);
    // push ebp; and esp, -8; esi/edi; the call
    let esp = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(8 + 4);
    while i.cast_signed() < g.read::<i32>(if i == 0 { w(0xc) } else { w(0x78) }) {
        let delay = g.read::<i32>(r - 4);
        if delay > 0 {
            eax = (delay - 1).cast_unsigned();
            g.write::<u32>(r - 4, eax);
        } else if g.read::<u32>(r + 4) == 0 {
            eax = facing_0380(g, img, img.rel32_target(f + 0x2d), esp, i);
            if eax == 1 {
                g.write::<u32>(r + 4, 1);
            }
        } else {
            let n = g.read::<i32>(r);
            eax = n.cast_unsigned();
            if n > 0 {
                eax = (n - 1).cast_unsigned();
                g.write::<u32>(r, eax);
                if eax == 0 {
                    let a = g.read::<u32>(w(0x48));
                    let b = if i == u32::MAX {
                        a
                    } else {
                        u32::from(g.read::<u8>(g.read::<u32>(w(0x56)).wrapping_add(i)))
                    };
                    let t = slot_target(g, w(0x62), 239);
                    // call width: 3
                    eax = g.call(t, 0, &[a, b, 0]).eax;
                }
            }
        }
        i += 1;
        r += 0xc;
    }
    Ret::eax(eax)
}

/// [`waits_0380`] (132 bytes, 1 overlay).
pub const WAITS_0380: Sig = Sig {
    len: 132,
    hash: 0xf438_3d9f_40a7_9b13,
    calls: &[(0x2d, Callee::Sig(&FACING_0380))],
    imports: &[],
};

/// `magic_0380`'s throw (0x100013a0, 969 bytes; a task callback, cdecl
/// `(node)`; an 8-aligned /GS frame `E`), on the summon's state `S` (the
/// node's `+0x14`): while `S + 0x4d0` runs it counts down (returned).
/// Then object 1 starts on `S`
/// ([`start_object_at`](crate::s4_misc::start_object_at), scale `K`);
/// its place `S + 0xd0` = slot 237 of the target's bone 6 at `(500, 0,
/// 0, 1)`, `w` 1.0; the first target's point `Q` (slot 262); `d` = the
/// flat distance from it (`sqrt` of the squares, `y` 0) times `K1` plus
/// `K1`; the goal `S + 0x358` = slot 237 of the bone at `(d + rnd 500,
/// 0, 0, 1)` with `y` the target's height (slot 12) less `K2` plus rnd
/// 2, `w` 1.0, the callback and four words cleared; the aimed emitter on
/// `S` ([`aimed_emitter_angles`](crate::s7_misc::aimed_emitter_angles),
/// speed `K3`); object 3 starts on `S + 0x3a0` (scale `(slot 404 (0.25) +
/// K4) K`, owner the place-to-goal distance over `K3`, `CB`), its place the
/// goal and its two angles `S`'s plus `K5` wrapped once; the node goes
/// back ([`give_back`](crate::sa_pool::give_back)). Returns the give-back's
/// `eax`.
///
/// The objects' starts push the `ecx` left behind as their third word,
/// which the start does not read.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn throw_0380<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x67), n);
    // push ebp; and esp, -8; sub esp, 0x4c
    let e = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x4c);
    let node = fr.arg(g, 0);
    let s = g.read::<u32>(node + 0x14);
    let n = g.read::<i32>(s + 0x4d0);
    if n > 0 {
        g.write::<i32>(s + 0x4d0, n - 1);
        return Ret::eax((n - 1).cast_unsigned());
    }
    let at = img.rel32_target(f + 0x56);
    crate::s4_misc::start_object_at(g, img, at, s, k(g, 0x48), 1, 0, 0);
    let pos = s + 0xd0;
    let p = e + 0x28;
    for (i, x) in (0u32..).zip([0x43fa_0000u32, 0, 0, ONE]) {
        g.write::<u32>(p + 4 * i, x);
    }
    let t = g.read::<u32>(w(0x6f));
    // call width: 4
    g.call(slot(g, 237), 0, &[t, 6, p, pos]);
    g.write::<u32>(pos + 0xc, ONE);
    let b = u32::from(g.read::<u8>(g.read::<u32>(w(0xa3))));
    let (q, a) = (e + 0x18, e + 0x38);
    let px = g.read::<u32>(pos);
    g.write::<u32>(a, px);
    let pz = g.read::<u32>(s + 0xd8);
    g.write::<u32>(a + 8, pz);
    let pw = g.read::<u32>(s + 0xdc);
    g.write::<u32>(a + 0xc, pw);
    let t0 = g.read::<u32>(w(0xce).wrapping_add(4 * b));
    // call width: 4
    g.call(slot(g, 262), 0, &[t0, q, q + 4, q + 8]);
    g.write::<u32>(q + 0xc, ONE);
    g.write::<u32>(a + 4, 0);
    g.write::<u32>(q + 4, 0);
    let rf = |g: &G, at: u32| g.read::<f32>(at);
    let dx = sub(rf(g, a), rf(g, q));
    let dy = sub(rf(g, a + 4), rf(g, q + 4));
    let dz = sub(rf(g, a + 8), rf(g, q + 8));
    let sq = add(add(mul(dy, dy), mul(dx, dx)), mul(dz, dz));
    #[allow(clippy::cast_possible_truncation)] // cvtsd2ss
    let d = f64::from(sq).sqrt() as f32;
    let d = add(mul(d, k(g, 0x15c)), k(g, 0x16b));
    // call width: 1
    let r = st0_f32(g.call(slot(g, 314), 0, &[0x43fa_0000]).st0);
    let goal = s + 0x358;
    let p2 = e + 8;
    for (i, x) in (0u32..).zip([add(d, r).to_bits(), 0, 0, ONE]) {
        g.write::<u32>(p2 + 4 * i, x);
    }
    let t = g.read::<u32>(w(0x1a2));
    // call width: 4
    g.call(slot(g, 237), 0, &[t, 6, p2, goal]);
    g.write::<u32>(goal + 0xc, ONE);
    let t = g.read::<u32>(w(0x1da));
    // call width: 1
    let h = st0_f32(g.call(slot(g, 12), 0, &[t]).st0);
    let y = sub(h, k(g, 0x1f1));
    g.write::<f32>(s + 0x35c, y);
    // call width: 1
    let r = st0_f32(g.call(slot(g, 314), 0, &[0x4000_0000]).st0);
    let y = add(r, g.read::<f32>(s + 0x35c));
    g.write::<f32>(s + 0x35c, y);
    g.write::<u32>(s + 0x364, ONE);
    g.write::<u32>(s + 0x378, w(0x24d));
    for o in [0x37cu32, 0x380, 0x38c] {
        g.write::<u32>(s + o, 0);
    }
    g.write::<u16>(s + 0x394, 0);
    let speed = k(g, 0x22a);
    let aim = Frame {
        at: img.rel32_target(f + 0x277),
        esp: e.wrapping_sub(0x10),
        ecx: s,
        xmm: [0, crate::Xmm::f32(speed).reg(), 0, 0],
        ..Frame::default()
    };
    crate::s7_misc::aimed_emitter_angles(g, img, &aim);
    let dx = sub(rf(g, pos), rf(g, goal));
    let dy = sub(rf(g, s + 0xd4), rf(g, s + 0x35c));
    let dz = sub(rf(g, s + 0xd8), rf(g, s + 0x360));
    let sq = add(add(mul(dy, dy), mul(dx, dx)), mul(dz, dz));
    #[allow(clippy::cast_possible_truncation)] // cvtsd2ss
    let dist = f64::from(sq).sqrt() as f32;
    let owner = crate::sb_sse::cvtt(div(dist, speed)).cast_unsigned();
    // call width: 1
    let q = st0_f32(g.call(slot(g, 404), 0, &[0x3e80_0000]).st0);
    let scale = mul(add(q, k(g, 0x2ff)), k(g, 0x30e));
    let obj = s + 0x3a0;
    let at = img.rel32_target(f + 0x313);
    crate::s4_misc::start_object_at(g, img, at, obj, scale, 3, owner, w(0x2a4));
    for (i, o) in (0u32..).zip([0x358u32, 0x35c, 0x360, 0x364]) {
        let x = g.read::<u32>(s + o);
        g.write::<u32>(obj + 0xd0 + 4 * i, x);
    }
    let (k2, k1, k3) = (k(g, 0x31d), k(g, 0x325), k(g, 0x32d));
    for o in [0xc0u32, 0xc4] {
        let v = wrap(add(g.read::<f32>(s + o), k2), k2, k3, k1);
        g.write::<f32>(obj + o, v);
    }
    Ret::eax(crate::sa_pool::give_back(
        g,
        img,
        img.rel32_target(f + 0x3b3),
        node,
    ))
}

/// [`throw_0380`] (969 bytes, 1 overlay).
pub const THROW_0380: Sig = Sig {
    len: 969,
    hash: 0x1d1f_3a35_574e_78d6,
    calls: &[
        // __security_check_cookie
        (0x3c, Callee::Any),
        (0x56, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x145, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x277, Callee::Sig(&crate::s7_misc::AIMED_EMITTER_ANGLES)),
        (0x2c7, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x313, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x3b3, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x3c1, Callee::Any),
    ],
    imports: &[],
};

/// [`start_up_0380`]'s inlined pool take: the free ring's head (`G_FREE`,
/// the count `G_N` u16) unlinked, `G_FREE` its next (0 with one left),
/// the count one less, the node's words cleared, `+8` itself, `+0x10` the
/// callback `cb`, byte `+0x18` 0. With the ring empty the original skips
/// all of this and writes through node 0 (it faults); so does this.
fn ring_node_0380<G: Game>(g: &mut G, free: u32, count: u32, cb: u32) -> u32 {
    let n = g.read::<u16>(count);
    if n == 0 {
        return 0;
    }
    let node = g.read::<u32>(free);
    let (prev, next) = (g.read::<u32>(node), g.read::<u32>(node + 4));
    g.write::<u32>(prev.wrapping_add(4), next);
    let (next, prev) = (g.read::<u32>(node + 4), g.read::<u32>(node));
    g.write::<u32>(next, prev);
    let head = if n > 1 { g.read::<u32>(node + 4) } else { 0 };
    g.write::<u32>(free, head);
    for o in [0u32, 4, 0xc, 0x14] {
        g.write::<u32>(node + o, 0);
    }
    g.write::<u32>(node + 8, node);
    g.write::<u32>(node + 0x10, cb);
    g.write::<u8>(node + 0x18, 0);
    g.write::<u16>(count, n - 1);
    node
}

/// `magic_0380`'s start-up (0x10001ca0, 1565 bytes; internal, no
/// arguments, run by its load step [`load_0380`]; an `ebp` frame): two
/// globals set to their immediates; `*G_H` = slot 719 `(D, 8, 0)`; the
/// part set up ([`part_init`](crate::l5_objhost::part_init)); object 0
/// started ([`start_object_at`](crate::s4_misc::start_object_at)); five
/// objects (0x130 apart, owners from a table, `CB`) each with a record
/// (0x4e0 apart, `+0x4d0` its owner) run by a node of the pool's ring
/// (inlined takes, appended to the task list); the `*G_N` actors' wait
/// records (0x45, 12, 0); the effect's task
/// ([`task`](crate::m0380::task)); two actor records (slot 215 or 413 of
/// the caster, sides) and slot 215 `(0x14)`; three more records with
/// ring nodes (their callbacks, the last [`waits_0380`]'s) and a fourth
/// node (kind 2) for the scripts' step. Returns 0.
///
/// The objects' starts push the `ecx` left behind as their third word,
/// which the start does not read. With the ring empty the original
/// writes through node 0 (it faults); so does this.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn start_up_0380<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x05), n);
    let (free, count) = (w(0xc1), w(0xa6));
    let (first, last) = (w(0x12b), w(0x13d));
    g.write::<u32>(w(0x1d), w(0x21));
    g.write::<u32>(w(0x27), w(0x2b));
    // call width: 3
    let h = g.call(slot(g, 719), 0, &[w(0x17), 8, 0]).eax;
    g.write::<u32>(w(0x35), h);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x3a));
    let start = img.rel32_target(f + 0x56);
    crate::s4_misc::start_object_at(g, img, start, w(0x4a), k(g, 0x42), 0, 0, 0);
    g.write::<u32>(w(0x64), w(0x68));
    // the five objects and their records
    let mut obj = w(0x5b);
    let mut rec = w(0x70);
    let mut entry = w(0x75);
    while entry < w(0x168) {
        let owner = g.read::<u32>(entry);
        let at = img.rel32_target(f + 0x96);
        crate::s4_misc::start_object_at(g, img, at, obj, k(g, 0x86), 2, owner, w(0x8b));
        g.write::<u32>(rec + 0x4d0, owner);
        obj += 0x130;
        let node = ring_node_0380(g, free, count, w(0x11a));
        let listed = g.read::<u32>(first) != 0;
        g.write::<u32>(node + 0x14, rec);
        g.write::<u32>(node + 4, 0);
        if listed {
            let l = g.read::<u32>(last);
            g.write::<u32>(node, l);
            g.write::<u32>(l.wrapping_add(4), node);
        } else {
            g.write::<u32>(node, 0);
            g.write::<u32>(w(0x150), node);
        }
        entry += 4;
        rec += 0x4e0;
        g.write::<u32>(node + 8, node);
        g.write::<u32>(w(0x162), node);
    }
    // the actors' wait records
    let n = g.read::<i32>(w(0x174));
    let mut word = 0u32;
    if n > 0 {
        for j in 0..n.cast_unsigned() {
            let r = w(0x17d) + 0xc * j;
            g.write::<u32>(r - 8, 0x45);
            g.write::<u32>(r - 4, 0xc);
            g.write::<u32>(r, 0);
        }
    } else {
        // `push ecx`: the count itself
        word = n.cast_unsigned();
    }
    // ebp, ecx, ebx/esi/edi, the two words, the call
    let sp = fr.esp.wrapping_sub(4 + 4 + 12 + 8);
    g.write::<u32>(sp, w(0x19d));
    g.write::<u32>(sp + 4, word);
    let task = Frame {
        at: img.rel32_target(f + 0x1a2),
        esp: sp.wrapping_sub(4),
        ..Frame::default()
    };
    crate::m0380::task(g, img, &task);
    // the two actor records
    let a = g.read::<u32>(w(0x1a7));
    g.write::<u32>(w(0x1b6), a);
    g.write::<u32>(w(0x1c2), w(0x1c6));
    g.write::<u32>(w(0x1cc), u32::MAX);
    let s = if g.read::<u32>(w(0x1b0)) == 0 {
        215
    } else {
        413
    };
    // call width: 1
    let r = g.call(slot(g, s), 0, &[a]).eax;
    let flag = g.read::<u32>(w(0x1e8));
    g.write::<u32>(w(0x1f5), r);
    g.write::<u32>(w(0x1fb), u32::MAX);
    g.write::<u32>(w(0x205), 0);
    g.write::<u32>(w(0x20f), 1);
    if flag == 0 {
        g.write::<u16>(w(0x21b), 0);
    } else {
        g.write::<u16>(w(0x224), 1);
    }
    let a = g.read::<u32>(w(0x229));
    g.write::<u32>(w(0x22e), a);
    g.write::<u32>(w(0x23a), w(0x23e));
    g.write::<u32>(w(0x244), u32::MAX);
    let s = if flag == 0 { 215 } else { 413 };
    // call width: 1
    let r = g.call(slot(g, s), 0, &[a]).eax;
    g.write::<u32>(w(0x26b), r);
    g.write::<u32>(w(0x271), u32::MAX);
    g.write::<u32>(w(0x27b), 0);
    g.write::<u32>(w(0x285), 1);
    if g.read::<u32>(w(0x265)) == 0 {
        g.write::<u16>(w(0x28f), 0);
    } else {
        g.write::<u16>(w(0x298), 1);
    }
    // call width: 1
    let r = g.call(slot(g, 215), 0, &[0x14]).eax;
    g.write::<u32>(w(0x2b3), r);
    g.write::<u16>(w(0x2be), 2);
    for o in [0x2c9u32, 0x2d3, 0x2dd, 0x2e7] {
        g.write::<u32>(w(o), 0);
    }
    g.write::<u32>(w(0x2f1), 1);
    g.write::<u32>(w(0x2f7), 1);
    g.write::<u16>(w(0x2fd), 0);
    // the second record's node
    let n2 = ring_node_0380(g, free, count, w(0x361));
    let listed = g.read::<u32>(first) != 0;
    g.write::<u8>(n2 + 0x18, 1);
    g.write::<u32>(n2 + 0x14, w(0x37e));
    g.write::<u32>(n2 + 4, 0);
    if listed {
        let l = g.read::<u32>(last);
        g.write::<u32>(n2, l);
        g.write::<u32>(l.wrapping_add(4), n2);
    } else {
        g.write::<u32>(n2, 0);
        g.write::<u32>(w(0x39f), n2);
    }
    g.write::<u32>(n2 + 8, n2);
    let fl = g.read::<u16>(w(0x3a9));
    g.write::<u16>(w(0x3a9), fl | 1);
    for (o, v) in [
        (0x3b2u32, 0u32),
        (0x3bc, 0),
        (0x3c6, ONE),
        (0x3d0, 0),
        (0x3da, 1),
    ] {
        g.write::<u32>(w(o), v);
    }
    g.write::<u16>(w(0x3e4), 0);
    // the third, after the second
    let n3 = ring_node_0380(g, free, count, w(0x446));
    let listed = g.read::<u32>(first) != 0;
    g.write::<u8>(n3 + 0x18, 1);
    g.write::<u32>(n3 + 0x14, w(0x463));
    g.write::<u32>(n3 + 4, 0);
    if listed {
        g.write::<u32>(n3, n2);
        g.write::<u32>(n2.wrapping_add(4), n3);
    } else {
        g.write::<u32>(n3, 0);
        g.write::<u32>(w(0x47f), n3);
    }
    g.write::<u32>(n3 + 8, n3);
    let fl = g.read::<u16>(w(0x489));
    g.write::<u16>(w(0x489), fl | 7);
    for (o, v) in [
        (0x490u32, ONE),
        (0x49a, ONE),
        (0x4a4, 0),
        (0x4ae, 0x4080_0000),
        (0x4b8, 0x4080_0000),
        (0x4c2, 0),
        (0x4cc, 0),
    ] {
        g.write::<u32>(w(o), v);
    }
    // the waits' node, after the third
    let n4 = ring_node_0380(g, free, count, w(0x532));
    let listed = g.read::<u32>(first) != 0;
    g.write::<u32>(n4 + 4, 0);
    let head = if listed {
        g.write::<u32>(n4, n3);
        g.write::<u32>(n3.wrapping_add(4), n4);
        g.read::<u32>(w(0x558))
    } else {
        g.write::<u32>(n4, 0);
        g.write::<u32>(w(0x568), n4);
        n4
    };
    g.write::<u32>(n4 + 8, n4);
    // the scripts' node (kind 2), last
    let n5 = ring_node_0380(g, free, count, w(0x5ca));
    g.write::<u8>(n5 + 0x18, 2);
    g.write::<u32>(n5 + 4, 0);
    if head == 0 {
        g.write::<u32>(n5, 0);
        g.write::<u32>(w(0x609), n5);
        g.write::<u32>(n5 + 8, n5);
        g.write::<u32>(w(0x612), n5);
    } else {
        g.write::<u32>(n5, n4);
        g.write::<u32>(n4 + 4, n5);
        g.write::<u32>(n5 + 8, n5);
        g.write::<u32>(w(0x5f4), n5);
    }
    Ret::eax(0)
}

/// [`start_up_0380`].
pub const START_UP_0380: Sig = Sig {
    len: 1565,
    hash: 0x97dd_90a3_02de_41c5,
    calls: &[
        (0x3a, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x56, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x96, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x1a2, Callee::Sig(&crate::m0380::TASK)),
    ],
    imports: &[],
};

/// `magic_0380`'s load step (0x10001210, the 118-byte shape) running
/// [`start_up_0380`].
pub fn load_0380<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::r9_load::load_step(g, img, fr, crate::r9_load::Shape::Zero, start_up_0380)
}

/// [`load_0380`].
pub const LOAD_0380: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&START_UP_0380))],
    imports: &[],
};

/// `magic_0298`'s pose (0x10001b90, 520 bytes; internal, no arguments; an
/// `ebp` /GS frame: `A` `ebp - 0x84`, `B` `ebp - 0xd4`, `C` `ebp - 0x94`,
/// `D` `ebp - 0x114`, `V` `ebp - 0x14`, `U` `ebp - 0x24`, `Q1` `ebp -
/// 0x34`, `Q2` `ebp - 0x44`): `A` the unit (slot 235) through slot 554
/// `(A, A, &G_Y)`; `V` = the place `G_P` with `y` the caster's height
/// (slot 12); `B` the unit turned by `G_Y` (slot 328) and `G_R` (slot
/// 329), `C = B (1.5, 0, -1.5, 1)` (slot 229), `V += C` (slot 248); `A`'s
/// last row `V K` with `w` 1.0; `D` = slot 327 `(A)`; slot 858 `(1, D,
/// (-7.9, 0, -7.9, 1), (7.8, 0, 7.8, 1), 0x3800)`. Returns its `eax`.
pub fn pose_0298<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x1b), n);
    let ebp = esp.wrapping_sub(4);
    let at = |o: u32| ebp.wrapping_sub(o);
    let (a, b, c, d) = (at(0x84), at(0xd4), at(0x94), at(0x114));
    let (v, u, q1, q2) = (at(0x14), at(0x24), at(0x34), at(0x44));
    // call width: 1
    g.call(slot(g, 235), 0, &[a]);
    // call width: 3
    g.call(slot(g, 554), 0, &[a, a, w(0x2e)]);
    for (i, o) in (0u32..).zip([0x45u32, 0x5d, 0x6a, 0x77]) {
        let x = g.read::<u32>(w(o));
        g.write::<u32>(v + 4 * i, x);
    }
    let chr = g.read::<u32>(w(0x50));
    // call width: 1
    let y = st0_f32(g.call(slot(g, 12), 0, &[chr]).st0);
    g.write::<f32>(v + 4, y);
    for (i, x) in (0u32..).zip([0x3fc0_0000u32, 0, 0xbfc0_0000, ONE]) {
        g.write::<u32>(u + 4 * i, x);
    }
    // call width: 1
    g.call(slot(g, 235), 0, &[b]);
    let yaw = g.read::<u32>(w(0xbc));
    // call width: 3
    g.call(slot(g, 328), 0, &[b, b, yaw]);
    let roll = g.read::<u32>(w(0xe1));
    // call width: 3
    g.call(slot(g, 329), 0, &[b, b, roll]);
    g.write::<u32>(u + 0xc, ONE);
    // call width: 3
    g.call(slot(g, 229), 0, &[c, b, u]);
    // call width: 3
    g.call(slot(g, 248), 0, &[v, v, c]);
    let s = k(g, 0x145);
    let row = a + 0x30;
    for i in 0..3u32 {
        let x = mul(g.read::<f32>(v + 4 * i), s);
        g.write::<f32>(row + 4 * i, x);
    }
    g.write::<u32>(v + 0xc, ONE);
    g.write::<u32>(row + 0xc, ONE);
    // call width: 2
    g.call(slot(g, 327), 0, &[d, a]);
    for (i, x) in (0u32..).zip([0xc0fc_cccdu32, 0, 0xc0fc_cccd, ONE]) {
        g.write::<u32>(q1 + 4 * i, x);
    }
    for (i, x) in (0u32..).zip([0x40f9_999au32, 0, 0x40f9_999a, ONE]) {
        g.write::<u32>(q2 + 4 * i, x);
    }
    // call width: 5
    g.call(slot(g, 858), 0, &[1, d, q1, q2, 0x3800]).eax
}

/// [`pose_0298`] (520 bytes, 1 overlay).
pub const POSE_0298: Sig = Sig {
    len: 520,
    hash: 0x6f92_1e3c_939f_3c8f,
    calls: &[
        // __security_check_cookie
        (0x200, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0298`'s actor step (0x10001e60, 378 bytes; a task callback,
/// cdecl `(node)`; an `ebp` /GS frame, `P` at `ebp - 0x14`) on the record
/// `S = node + 0x14`: while `S + 0x40` runs it counts down (returned).
/// Unless `S + 0x14` is set, the motion script
/// ([`motion_script`](crate::s4_misc::motion_script)) runs and on 1 slot
/// 239 `(*G_A, S + 8, 2)`. `S + 0x20` += `S + 0x30` (three words); the
/// actor `S + 0x48`'s point (slot 262; -1: the caster) plus that offset
/// goes to the actor ([`actor_point`](crate::s4_misc::actor_point)'s
/// shape: `ecx` the index, `edx` the point). `S + 0x44` counts down; at 0
/// `S + 0x20` and `S + 0x28` times `K`, `S + 0x24`, `S + 0x34` 0; with
/// `S + 0x14` 1 then `S + 0x50` 1, slot 538 for the actor
/// ([`actor_slot_edx`](crate::o4_single::actor_slot_edx), `S + 0x4c`) and
/// the node goes back ([`give_back`](crate::sa_pool::give_back), its
/// `eax` returned). Returns `S + 0x44` otherwise.
pub fn actor_step_0298<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x46), n);
    let ebp = fr.esp.wrapping_sub(4);
    let node = fr.arg(g, 0);
    let s = g.read::<u32>(node + 0x14);
    let n = g.read::<i32>(s + 0x40);
    if n > 0 {
        g.write::<i32>(s + 0x40, n - 1);
        return Ret::eax((n - 1).cast_unsigned());
    }
    if g.read::<u32>(s + 0x14) == 0
        && crate::s4_misc::motion_script(g, img, img.rel32_target(f + 0x3c), s) == 1
    {
        let a = g.read::<u32>(w(0x57));
        let b = g.read::<u32>(s + 8);
        // call width: 3
        g.call(slot(g, 239), 0, &[a, b, 2]);
    }
    let rf = |g: &G, at: u32| g.read::<f32>(at);
    for i in 0..3u32 {
        let x = add(rf(g, s + 0x30 + 4 * i), rf(g, s + 0x20 + 4 * i));
        g.write::<f32>(s + 0x20 + 4 * i, x);
    }
    let idx = g.read::<u32>(s + 0x48);
    let actor = if idx == u32::MAX {
        g.read::<u32>(w(0x96))
    } else {
        let b = u32::from(g.read::<u8>(g.read::<u32>(w(0x9d)).wrapping_add(idx)));
        g.read::<u32>(w(0xa8).wrapping_add(4 * b))
    };
    let p = ebp.wrapping_sub(0x14);
    // call width: 4
    g.call(slot(g, 262), 0, &[actor, p, p + 4, p + 8]);
    g.write::<u32>(p + 0xc, ONE);
    g.write::<f32>(p, add(rf(g, s + 0x20), rf(g, p)));
    g.write::<f32>(p + 4, add(rf(g, p + 4), rf(g, s + 0x24)));
    g.write::<f32>(p + 8, add(rf(g, p + 8), rf(g, s + 0x28)));
    let idx = g.read::<u32>(s + 0x48);
    // esi/edi, the frame, the call
    let to = Frame {
        at: img.rel32_target(f + 0x104),
        esp: ebp.wrapping_sub(0x14 + 8 + 4),
        ecx: idx,
        edx: p,
        ..Frame::default()
    };
    crate::s4_small::actor_index_to_point(g, img, &to);
    let mut left = g.read::<i32>(s + 0x44);
    if left > 0 {
        left -= 1;
        g.write::<i32>(s + 0x44, left);
        if left > 0 {
            return Ret::eax(left.cast_unsigned());
        }
    }
    let kk = k(g, 0x122);
    g.write::<f32>(s + 0x20, mul(rf(g, s + 0x20), kk));
    g.write::<u32>(s + 0x24, 0);
    g.write::<u32>(s + 0x34, 0);
    g.write::<f32>(s + 0x28, mul(rf(g, s + 0x28), k(g, 0x142)));
    if g.read::<u32>(s + 0x14) != 1 {
        return Ret::eax(left.cast_unsigned());
    }
    let (i, e) = (g.read::<u32>(s + 0x48), g.read::<u32>(s + 0x4c));
    g.write::<u32>(s + 0x50, 1);
    crate::o4_single::actor_slot_edx(g, img, img.rel32_target(f + 0x15f), i, e);
    Ret::eax(crate::sa_pool::give_back(
        g,
        img,
        img.rel32_target(f + 0x166),
        node,
    ))
}

/// [`actor_step_0298`] (378 bytes, 1 overlay).
pub const ACTOR_STEP_0298: Sig = Sig {
    len: 378,
    hash: 0x7eef_a505_989b_bd60,
    calls: &[
        // __security_check_cookie
        (0x2b, Callee::Any),
        (0x3c, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0x104, Callee::Sig(&crate::s4_small::ACTOR_INDEX_TO_POINT)),
        (
            0x15f,
            Callee::Sig(&crate::o4_single::ACTOR_SLOT_EDX_0298_2250),
        ),
        (0x166, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x172, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0298`'s start-up (0x10001390, 2033 bytes; internal, no
/// arguments, run by its load step [`load_0298`]; an 8-aligned /GS frame
/// `E`: the place `G` at `E + 0xc`/`E + 0x10`, `V` `E + 0x34`, `Q` `E +
/// 0x24`, `W` `E + 0x44`, `U` `E + 0x54`, `M` `E + 0x64`, the yaw `E +
/// 4`): three globals set to their immediates; `*G_H` = slot 719 `(D, 8,
/// 0)`; the part set up; object 0 started (the bare reset); the caster's
/// point (slot 262) with its height (slot 12), `w` 1.0; the yaw `K - slot
/// 333 (caster)` wrapped once; a record of globals; then for each of the
/// `*G_N` actors a 0x54-byte record (from `G_R`): its model (slot 538),
/// callback, index and slot 215/413 answer, -1, 0, 1 and the side; its
/// heading from the caster's place (`K - atan2`, wrapped once) turning
/// `(0, -20, -24, 1)` (slots 235, 329, 229) added to the actor's point
/// (slot 248) and slot 564 `(U, U, 20)`; the record's motion words from
/// those, the counts 0x4c and 0x14; a ring node (inlined take, the
/// actor step's callback) for it appended to the task list. Then the
/// static record's node, a record of globals and its ring node; the pose
/// ([`pose_0298`]); the models of slots 288 `(9)` and 288 `(11)` to the
/// list; the actor record (slot 215/413, the side); the last ring node
/// (kind 2). Returns 0.
///
/// With the ring empty the original writes through node 0 (it faults); so
/// does this.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn start_up_0298<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x1b), n);
    // push ebp; and esp, -8; sub esp, 0xac
    let e = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0xac);
    let (free, count) = (w(0x469), w(0x419));
    let (first, last) = (w(0x4d0), w(0x4e5));
    for (to, imm) in [(0x33u32, 0x37u32), (0x3d, 0x41), (0x47, 0x4b)] {
        g.write::<u32>(w(to), w(imm));
    }
    // call width: 3
    let h = g.call(slot(g, 719), 0, &[w(0x2d), 8, 0]).eax;
    g.write::<u32>(w(0x55), h);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x5a));
    // ebx/esi/edi, the slot's two words left, 8 bytes, the call
    let bare = Frame {
        at: img.rel32_target(f + 0x6f),
        esp: e.wrapping_sub(0xc + 8 + 8 + 4),
        edx: w(0x6a),
        xmm: [0, 0, 0, crate::Xmm::f32(k(g, 0x62)).reg()],
        ..Frame::default()
    };
    crate::r9_misc::start_object_reset_bare(g, img, &bare);
    let chr = g.read::<u32>(w(0x7a));
    // call width: 4
    g.call(slot(g, 262), 0, &[chr, w(0x8f), w(0x8a), w(0x85)]);
    // call width: 1
    let y = st0_f32(g.call(slot(g, 12), 0, &[chr]).st0);
    g.write::<f32>(w(0xa3), y);
    g.write::<u32>(w(0xb1), ONE);
    let a = g.read::<u32>(w(0xbb));
    // call width: 1
    let t = st0_f32(g.call(slot(g, 333), 0, &[a]).st0);
    let yaw = wrap(sub(k(g, 0xcf), t), k(g, 0xe7), k(g, 0xf7), k(g, 0xdd));
    g.write::<f32>(w(0x108), yaw);
    let y = add(k(g, 0x110), k(g, 0x118));
    g.write::<u32>(w(0x122), 0);
    g.write::<u32>(w(0x12c), 0);
    g.write::<f32>(w(0x138), y);
    let gx = g.read::<u32>(w(0x140));
    g.write::<u32>(e + 0x10, gx);
    let gz = g.read::<u32>(w(0x14e));
    g.write::<u32>(w(0x154), 0);
    g.write::<u32>(w(0x15e), 0);
    g.write::<u32>(w(0x168), w(0x16c));
    g.write::<u32>(w(0x172), w(0x176));
    g.write::<u32>(w(0x17c), 0);
    g.write::<u32>(w(0x186), 0);
    g.write::<u16>(w(0x190), 0);
    g.write::<u32>(w(0x196), w(0x19a));
    g.write::<u32>(e + 0xc, gz);
    let (v, q, wv, u, m) = (e + 0x34, e + 0x24, e + 0x44, e + 0x54, e + 0x64);
    for (i, x) in (0u32..).zip([0u32, 0xc1a0_0000, 0xc1c0_0000, ONE]) {
        g.write::<u32>(v + 4 * i, x);
    }
    let byte =
        |g: &G, sel: u32, i: u32| u32::from(g.read::<u8>(g.read::<u32>(w(sel)).wrapping_add(i)));
    let mut rec = w(0x1d0);
    let mut i = 0u32;
    while i.cast_signed() < g.read::<i32>(if i == 0 { w(0x1c5) } else { w(0x510) }) {
        let b = byte(g, 0x1e1, i);
        // call width: 2
        let model = g.call(slot(g, 538), 0, &[b, 0]).eax;
        g.write::<u32>(rec + 0x48, model);
        g.write::<u32>(rec - 4, w(0x202));
        g.write::<u32>(rec, i);
        let b = byte(g, 0x215, i);
        g.write::<u32>(rec + 4, b);
        let s = if g.read::<u32>(w(0x21f)) == 0 {
            215
        } else {
            413
        };
        // call width: 1
        let r = g.call(slot(g, s), 0, &[b]).eax;
        g.write::<u32>(rec + 8, r);
        g.write::<u32>(rec + 0xc, u32::MAX);
        g.write::<u32>(rec + 0x10, 0);
        g.write::<u32>(rec + 0x14, 1);
        let side = u16::from(g.read::<u32>(w(0x244)) != 0);
        g.write::<u16>(rec + 0x18, side);
        let actor = {
            let b = byte(g, 0x27d, i);
            g.read::<u32>(w(0x288).wrapping_add(4 * b))
        };
        // call width: 4
        g.call(slot(g, 262), 0, &[actor, q, q + 4, q + 8]);
        // call width: 1
        let y = st0_f32(g.call(slot(g, 12), 0, &[actor]).st0);
        g.write::<f32>(q + 4, y);
        let a = heading_from(g, q, (e + 0x10, e + 0xc));
        g.write::<u32>(q + 0xc, ONE);
        let yaw = wrap(sub(k(g, 0x308), a), k(g, 0x317), k(g, 0x331), k(g, 0x327));
        // call width: 1
        g.call(slot(g, 235), 0, &[m]);
        // call width: 3
        g.call(slot(g, 329), 0, &[m, m, yaw.to_bits()]);
        // call width: 3
        g.call(slot(g, 229), 0, &[wv, m, v]);
        // call width: 3
        g.call(slot(g, 248), 0, &[u, q, wv]);
        // call width: 3
        g.call(slot(g, 564), 0, &[u, u, 0x41a0_0000]);
        let x2 = k(g, 0x3d7);
        let rf = |g: &G, at: u32| g.read::<f32>(at);
        g.write::<f32>(rec + 0x1c, div(rf(g, wv), x2));
        let uy = rf(g, u + 4);
        g.write::<f32>(rec + 0x20, mul(uy, k(g, 0x3f7)));
        let r1 = mul(uy, k(g, 0x3ff));
        g.write::<f32>(rec + 0x24, div(rf(g, wv + 8), x2));
        g.write::<u32>(rec + 0x28, 0);
        g.write::<u32>(rec + 0x2c, 0);
        g.write::<f32>(rec + 0x30, div(r1, x2));
        g.write::<u32>(rec + 0x34, 0);
        g.write::<u32>(rec + 0x38, 0);
        g.write::<u32>(rec + 0x3c, 0x4c);
        g.write::<u32>(rec + 0x40, 0x14);
        g.write::<u32>(rec + 0x44, i);
        g.write::<u32>(rec + 0x4c, 0);
        let node = ring_node_0380(g, free, count, w(0x4bf));
        let listed = g.read::<u32>(first) != 0;
        g.write::<u32>(node + 0x14, rec - 4);
        g.write::<u32>(node + 4, 0);
        if listed {
            let l = g.read::<u32>(last);
            g.write::<u32>(node, l);
            g.write::<u32>(l.wrapping_add(4), node);
        } else {
            g.write::<u32>(node, 0);
            g.write::<u32>(w(0x4f8), node);
        }
        i += 1;
        g.write::<u32>(node + 8, node);
        rec += 0x54;
        g.write::<u32>(w(0x505), node);
    }
    let rn = Frame {
        at: img.rel32_target(f + 0x51b),
        esp: e.wrapping_sub(0xc + 4),
        ..Frame::default()
    };
    crate::o4_small::static_record_node(g, img, &rn);
    let fl = g.read::<u16>(w(0x529));
    g.write::<u16>(w(0x529), fl | 3);
    for (o, v) in [
        (0x532u32, 0x4080_0000u32),
        (0x53c, 0x4000_0000),
        (0x546, 0),
        (0x550, 0x4080_0000),
        (0x55a, 0x4000_0000),
        (0x564, 0),
        (0x56e, 0x4c),
        (0x578, 0),
        (0x582, 0x50),
        (0x58c, 0),
        (0x596, 0x47),
        (0x5a0, 0x50),
        (0x5aa, 0x50),
    ] {
        g.write::<u32>(w(o), v);
    }
    g.write::<u16>(w(0x5b4), 0);
    let n2 = ring_node_0380(g, free, count, w(0x61e));
    let listed = g.read::<u32>(first) != 0;
    g.write::<u8>(n2 + 0x18, 1);
    g.write::<u32>(n2 + 0x14, w(0x63b));
    g.write::<u32>(n2 + 4, 0);
    if listed {
        let l = g.read::<u32>(last);
        g.write::<u32>(n2, l);
        g.write::<u32>(l.wrapping_add(4), n2);
    } else {
        g.write::<u32>(n2, 0);
        g.write::<u32>(w(0x65c), n2);
    }
    g.write::<u32>(n2 + 8, n2);
    g.write::<u32>(w(0x665), n2);
    pose_0298(g, img, img.rel32_target(f + 0x66a), e.wrapping_sub(0xc + 4));
    let cnt = e + 4;
    for (kind, call) in [(9u32, 0x68bu32), (0xb, 0x6af)] {
        // call width: 2
        let bytes = g.call(slot(g, 288), 0, &[kind, cnt]).eax;
        let n = g.read::<u32>(cnt);
        // the list word above the call (ebx/esi/edi, the slot's two
        // words, the word; the call)
        let sp = e.wrapping_sub(0xc + 8 + 4);
        g.write::<u32>(sp, 1);
        let to = Frame {
            at: img.rel32_target(f + call),
            esp: sp.wrapping_sub(4),
            ecx: bytes,
            edx: n,
            ..Frame::default()
        };
        crate::m0298::models_to_list(g, img, &to);
    }
    let a = g.read::<u32>(w(0x6b4));
    g.write::<u32>(w(0x6c3), a);
    g.write::<u32>(w(0x6cf), w(0x6d3));
    g.write::<u32>(w(0x6d9), u32::MAX);
    let s = if g.read::<u32>(w(0x6bd)) == 0 {
        215
    } else {
        413
    };
    // call width: 1
    let r = g.call(slot(g, s), 0, &[a]).eax;
    g.write::<u32>(w(0x6fe), r);
    g.write::<u32>(w(0x704), u32::MAX);
    g.write::<u32>(w(0x70e), 0);
    g.write::<u32>(w(0x718), 1);
    if g.read::<u32>(w(0x6f8)) == 0 {
        g.write::<u16>(w(0x726), 0);
    } else {
        g.write::<u16>(w(0x72f), 1);
    }
    let n3 = ring_node_0380(g, free, count, w(0x796));
    let listed = g.read::<u32>(first) != 0;
    g.write::<u8>(n3 + 0x18, 2);
    g.write::<u32>(n3 + 4, 0);
    if listed {
        let l = g.read::<u32>(last);
        g.write::<u32>(n3, l);
        g.write::<u32>(l.wrapping_add(4), n3);
    } else {
        g.write::<u32>(n3, 0);
        g.write::<u32>(w(0x7cd), n3);
    }
    g.write::<u32>(n3 + 8, n3);
    g.write::<u32>(w(0x7e2), n3);
    Ret::eax(0)
}

/// `K - atan2(dz, dx)`'s `atan2` part for [`start_up_0298`]: from the
/// place `(x, z)` (two frame words) to `to`.
fn heading_from<G: Game>(g: &mut G, to: u32, (x, z): (u32, u32)) -> f32 {
    let dz = sub(g.read::<f32>(to + 8), g.read::<f32>(z));
    let dx = sub(g.read::<f32>(to), g.read::<f32>(x));
    #[allow(clippy::cast_possible_truncation)] // cvtpd2ps
    let a = g.atan2(f64::from(dz), f64::from(dx)) as f32;
    a
}

/// [`start_up_0298`].
pub const START_UP_0298: Sig = Sig {
    len: 2033,
    hash: 0xcddf_29b3_72f2_4ec4,
    calls: &[
        (0x5a, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x6f, Callee::Sig(&crate::r9_misc::START_OBJECT_BARE_0298)),
        (0x2f6, Callee::Import("_CIatan2")),
        (
            0x51b,
            Callee::Sig(&crate::o4_small::STATIC_RECORD_NODE_0298_2850),
        ),
        (0x66a, Callee::Sig(&POSE_0298)),
        (0x68b, Callee::Sig(&crate::m0298::MODELS_TO_LIST)),
        (0x6af, Callee::Sig(&crate::m0298::MODELS_TO_LIST)),
        // __security_check_cookie
        (0x7e9, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0298`'s load step (0x10001260, the 118-byte shape) running
/// [`start_up_0298`].
pub fn load_0298<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::r9_load::load_step(g, img, fr, crate::r9_load::Shape::Zero, start_up_0298)
}

/// [`load_0298`].
pub const LOAD_0298: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&START_UP_0298))],
    imports: &[],
};

/// `magic_0430`'s effect end (0x10001ef0, 741 bytes; internal, no
/// arguments; an 8-aligned /GS frame `E`, the locals `E + 4` and `E +
/// 0x14`): slot 384 `(T[sel[2]])`; slots 25 `(0x3016, 1)` and 24 `(0x3016,
/// 0)`; for the first selected actor, the actor `*G_B` and the second
/// selected actor in turn: unless `*G_F` (or above index 0x1f) slots 217
/// and 255 `(i, &L)`, then slot 211 `(actor, 0)`; slots 213 `(*G_X, 0)` and
/// `(*G_Y, 0)` for halfwords `*G_HX`/`*G_HY` 0, `*G_S` and `*G_T` 1; slot
/// 296 `(i, 1)` for a party member, else 257 `(actor, 0)` (the first
/// selected actor); the party run ([`each_actor`](crate::sa_script::each_actor)
/// with 1); slot 296/257 again for `*G_B`'s index; slots 371 `(0, 0)` and
/// `(1, 0)`; the held node given back and cleared; the objects freed
/// ([`objects_freed`](crate::s6_misc::objects_freed)), slot 574 and the
/// parts registered ([`register_parts`](crate::sa_misc::register_parts));
/// slot 772 `(0x1ae, 0)` and slot 779 `(eax, edx, 0x1ae, 0)` with
/// `**G_LOCK` 1 around it. Returns `*G_LOCK`.
#[allow(clippy::too_many_lines)] // the original's calls, in order
pub fn effect_end_0430<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let rd = |g: &G, o: u32| g.read::<u32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x27), n);
    // push ebp; and esp, -8; sub esp, 0x28; esi/edi
    let e = (esp.wrapping_sub(4) & !7).wrapping_sub(0x28);
    let sel = |g: &G, op: u32, i: u32| u32::from(g.read::<u8>(rd(g, op).wrapping_add(i)));
    let t = g.read::<u32>(w(0x22).wrapping_add(4 * sel(g, 0x15, 2)));
    // call width: 1
    g.call(slot(g, 384), 0, &[t]);
    // call width: 2
    g.call(slot(g, 25), 0, &[0x3016, 1]);
    // call width: 2
    g.call(slot(g, 24), 0, &[0x3016, 0]);
    let (l4, l14) = (e + 4, e + 0x14);
    let points = |g: &mut G, i: u32, l: u32| {
        // call width: 2
        g.call(slot(g, 217), 0, &[i, l]);
        // call width: 2
        g.call(slot(g, 255), 0, &[i, l]);
    };
    // the first selected actor
    let b0 = sel(g, 0x57, 0);
    let a0 = g.read::<u32>(w(0x6b).wrapping_add(4 * b0));
    if rd(g, 0x60) == 0 && b0 < 0x20 {
        points(g, b0, l4);
    }
    // call width: 2
    g.call(slot(g, 211), 0, &[a0, 0]);
    // the actor `*G_B`
    let ab = rd(g, 0xb4);
    if rd(g, 0xbd) == 0 {
        let i = rd(g, 0xc6);
        if i.cast_signed() < 0x20 {
            points(g, i, l14);
        }
    }
    // call width: 2
    g.call(slot(g, 211), 0, &[ab, 0]);
    // the second selected actor
    let b1 = sel(g, 0x109, 1);
    let a1 = g.read::<u32>(w(0x11e).wrapping_add(4 * b1));
    if rd(g, 0x112) == 0 && b1 < 0x20 {
        points(g, b1, l4);
    }
    // call width: 2
    g.call(slot(g, 211), 0, &[a1, 0]);
    if g.read::<u16>(w(0x168)) == 0 {
        let x = rd(g, 0x178);
        // call width: 2
        g.call(slot(g, 213), 0, &[x, 0]);
    }
    let hy = g.read::<u16>(w(0x18a));
    g.write::<u32>(w(0x191), 1);
    if hy == 0 {
        let x = rd(g, 0x1a4);
        // call width: 2
        g.call(slot(g, 213), 0, &[x, 0]);
    }
    let flag = rd(g, 0x1b5);
    let s = rd(g, 0x1bb);
    g.write::<u32>(w(0x1c1), 1);
    let b = u32::from(g.read::<u8>(s));
    if flag == 0 && b < 0x20 {
        // call width: 2
        g.call(slot(g, 296), 0, &[b, 1]);
    } else {
        let a = g.read::<u32>(w(0x1eb).wrapping_add(4 * b));
        // call width: 2
        g.call(slot(g, 257), 0, &[a, 0]);
    }
    // esi/edi, the call
    let callee_esp = e.wrapping_sub(0xc);
    crate::sa_script::each_actor(g, img, img.rel32_target(f + 0x205), 1, callee_esp);
    let i = rd(g, 0x213);
    if rd(g, 0x20b) == 0 && i.cast_signed() < 0x20 {
        // call width: 2
        g.call(slot(g, 296), 0, &[i, 1]);
    } else {
        let a = rd(g, 0x235);
        // call width: 2
        g.call(slot(g, 257), 0, &[a, 0]);
    }
    for x in [0u32, 1] {
        // call width: 2
        g.call(slot(g, 371), 0, &[x, 0]);
    }
    let held = rd(g, 0x268);
    if held != 0 {
        crate::sa_pool::give_back(g, img, img.rel32_target(f + 0x274), held);
        g.write::<u32>(w(0x27a), 0);
    }
    let freed = Frame {
        at: img.rel32_target(f + 0x283),
        esp: callee_esp,
        ..Frame::default()
    };
    crate::s6_misc::objects_freed(g, img, &freed);
    // call width: 0
    let eax = g.call(slot(g, 574), 0, &[]).eax;
    crate::sa_misc::register_parts(g, img, img.rel32_target(f + 0x295), eax);
    // call width: 2
    let h = g.call(slot(g, 772), 0, &[0x1ae, 0]);
    let lock = rd(g, 0x2af);
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    // call width: 4
    g.call(slot(g, 779), 0, &[h.eax, h.edx, 0x1ae, 0]);
    let lock = rd(g, 0x2cb);
    g.write::<u32>(lock, saved);
    lock
}

/// [`effect_end_0430`] (741 bytes, 1 overlay).
pub const EFFECT_END_0430: Sig = Sig {
    len: 741,
    hash: 0xab5d_1316_f4e4_c6a5,
    calls: &[
        (0x205, Callee::Sig(&crate::sa_script::EACH_ACTOR)),
        (0x274, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x283, Callee::Sig(&crate::s6_misc::OBJECTS_FREED)),
        (0x295, Callee::Sig(&crate::sa_misc::REGISTER_PARTS)),
        // __security_check_cookie
        (0x2dd, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0430`'s per-frame task (0x10001730, 346 bytes; cdecl `(node)`):
/// [`task_0162`](crate::r34::task_0162)'s steps with sixteen words copied
/// (the displacement pairs read from the code) and, once the first script
/// is done, `S + 0xec8` = 1, the effect end ([`effect_end_0430`]) and the
/// node back to the pool (a tail jump). Returns the tail jump's answer.
pub fn task_0430<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let s = g.read::<u32>(w(0x7));
    g.write_bytes(w(0x10), &[0u8; 0x30]);
    let h = g.read::<u16>(w(0x21));
    g.write::<u16>(w(0x21), h | 0x80);
    let v = g.read::<u32>(w(0x26));
    g.write::<u32>(w(0x33), v);
    let first = crate::sb_misc::script_run(g, img, img.rel32_target(f + 0x38), w(0x2e));
    crate::sb_misc::script_run(g, img, img.rel32_target(f + 0x44), w(0x3d));
    if first != 0 {
        g.write::<u32>(s + w(0x142), w(0x146));
        // push ebp, esi, edi; the call
        effect_end_0430(
            g,
            img,
            img.rel32_target(f + 0x14b),
            fr.esp.wrapping_sub(0x10),
        );
        let node = fr.arg(g, 0);
        return Ret::eax(crate::sa_pool::give_back(
            g,
            img,
            img.rel32_target(f + 0x156),
            node,
        ));
    }
    // the sixteen copies `mov eax, [edi + from]; mov [edi + to], eax`
    // (the last pair split by a `mov ecx`)
    for k in 0..16u32 {
        let (from, to) = if k == 15 {
            (0x106, 0x111)
        } else {
            (0x52 + 12 * k, 0x58 + 12 * k)
        };
        let x = g.read::<u32>(s + w(from));
        g.write::<u32>(s + w(to), x);
    }
    for (op, call) in [
        (0x10bu32, 0x116u32),
        (0x11b, 0x120),
        (0x125, 0x12a),
        (0x12f, 0x134),
    ] {
        crate::s4_misc::motion_script(g, img, img.rel32_target(f + call), w(op));
    }
    let rigs = Frame {
        at: img.rel32_target(f + 0x13c),
        ..*fr
    };
    crate::s8_misc::camera_rigs_step(g, img, &rigs)
}

/// [`task_0430`] (346 bytes, 1 overlay).
pub const TASK_0430: Sig = Sig {
    len: 346,
    hash: 0x8526_56ed_e852_8107,
    calls: &[
        (0x15, Callee::Import("memset")),
        (0x38, Callee::Sig(&crate::sb_misc::SCRIPT_RUN)),
        (0x44, Callee::Sig(&crate::sb_misc::SCRIPT_RUN)),
        (0x116, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0x120, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0x12a, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0x134, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0x13c, Callee::Sig(&crate::s8_misc::CAMERA_RIGS_STEP)),
        (0x14b, Callee::Sig(&EFFECT_END_0430)),
        (0x156, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
    ],
    imports: &[],
};

/// `magic_0430`'s spark at the object (0x100022e0, 367 bytes; cdecl
/// `(obj)`): past 0x78 steps (`obj + 0x118`) only `obj + 0x104` = 0.
/// Else `obj + 0xd0` = slot 268 `(slot 219 (sel[0], 0), 14)`, and one
/// record of the DLL's pool ([`pool_take`](crate::sb_misc::pool_take);
/// none: returns 0) gets `+0x30` slot 719 `(D, 9, 0)`, the halfwords `+0x38`
/// -1 and `+0x3a` 0x200, `+0x34` 1, the place `obj + 0xd0`, `+0x10` rnd
/// 0.03, `+0x14` rnd 0.04 + `K`, `+0x18` rnd 0.03, `+0x20`/`+0x28` those
/// two times `K'`, `+0x1c` 0, `+0x24` 0.003, `+0x2c` 1.0, `+0x3c` 0.1,
/// `+0x4c` 0x18, `+0x50` 0, the halfword `+0x68` 0x6060 and the byte
/// `+0x6b` 0x50. The two random floats pass through the argument's stack
/// word (`[ebp + 8]`, last the third). Returns the last slot's `eax`, or
/// the entry's `eax` past 0x78.
pub fn spark_0430<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x2b), n);
    let obj = fr.arg(g, 0);
    if g.read::<i32>(obj + 0x118) > 0x78 {
        g.write::<u32>(obj + 0x104, 0);
        return Ret::eax(fr.eax);
    }
    let b = u32::from(g.read::<u8>(g.read::<u32>(w(0x1e))));
    // call width: 2
    let m = g.call(slot(g, 219), 0, &[b, 0]).eax;
    let p = obj + 0xd0;
    // call width: 3
    g.call(slot(g, 268), 0, &[m, 0xe, p]);
    let rec = crate::sb_misc::pool_take(g, img, img.rel32_target(f + 0x52));
    if rec == 0 {
        return Ret::eax(0);
    }
    // call width: 3
    let h = g.call(slot(g, 719), 0, &[w(0x71), 9, 0]).eax;
    g.write::<u32>(rec + 0x30, h);
    g.write::<u16>(rec + 0x38, 0xffff);
    g.write::<u32>(rec + 0x34, 1);
    g.write::<u16>(rec + 0x3a, 0x200);
    for i in 0..4u32 {
        let x = g.read::<u32>(p + 4 * i);
        g.write::<u32>(rec + 4 * i, x);
    }
    let arg = fr.esp + 4;
    let rnd = |g: &mut G, x: u32| {
        // call width: 1
        let out = g.call(slot(g, 314), 0, &[x]);
        (st0_f32(out.st0), out.eax)
    };
    let (r, _) = rnd(g, 0x3cf5_c28f);
    g.write::<f32>(rec + 0x10, r);
    let (r2, _) = rnd(g, 0x3d23_d70a);
    g.write::<f32>(arg, r2);
    g.write::<f32>(rec + 0x14, add(r2, k(g, 0xea)));
    let (r3, eax) = rnd(g, 0x3cf5_c28f);
    g.write::<f32>(arg, r3);
    let x = mul(g.read::<f32>(rec + 0x10), k(g, 0x118));
    g.write::<f32>(rec + 0x18, r3);
    let z = mul(r3, k(g, 0x125));
    g.write::<u32>(rec + 0x1c, 0);
    g.write::<f32>(rec + 0x20, x);
    g.write::<u32>(rec + 0x24, 0x3b44_9ba6);
    g.write::<f32>(rec + 0x28, z);
    g.write::<u32>(rec + 0x2c, ONE);
    g.write::<u32>(rec + 0x3c, 0x3dcc_cccd);
    g.write::<u32>(rec + 0x4c, 0x18);
    g.write::<u32>(rec + 0x50, 0);
    g.write::<u16>(rec + 0x68, 0x6060);
    g.write::<u8>(rec + 0x6b, 0x50);
    Ret::eax(eax)
}

/// [`spark_0430`] (367 bytes, 1 overlay).
pub const SPARK_0430: Sig = Sig {
    len: 367,
    hash: 0x2718_74e0_b0fc_771f,
    calls: &[(0x52, Callee::Sig(&crate::sb_misc::POOL_TAKE))],
    imports: &[],
};

/// The object `obj` placed at the state's point `S + 0xcc4` (four words)
/// with its facing `S + 0xeb4` (`+0xc4`).
fn place_at_0430<G: Game>(g: &mut G, s: u32, obj: u32) {
    for i in 0..4u32 {
        let x = g.read::<u32>(s + 0xcc4 + 4 * i);
        g.write::<u32>(obj + 0xd0 + 4 * i, x);
    }
}

/// `magic_0430`'s ring object (0x10002750, 355 bytes; no arguments; an
/// `ebp` /GS frame: `M` `ebp - 0x64`, `V` `ebp - 0x14`, `W` `ebp -
/// 0x24`): object 6 at `S + 0x720` started
/// ([`start_object_anim`](crate::s4_misc::start_object_anim), scale `K`),
/// placed at the state's point and facing; its place plus `M (-6, -24, 0,
/// 1)` with `M` the unit turned by the facing (slots 235, 328 `(0)`, 329,
/// 229, 248), `w` 1.0; slot 268 `(slot 219 (sel[0], 0), 14, place)`.
/// Returns slot 268's `eax`.
pub fn ring_object_0430<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x84), n);
    let ebp = fr.esp.wrapping_sub(4);
    let s = g.read::<u32>(w(0x1d));
    let obj = s + 0x720;
    let at = img.rel32_target(f + 0x31);
    crate::s4_misc::start_object_anim(g, img, at, obj, k(g, 0x14), 6, 0);
    place_at_0430(g, s, obj);
    let facing = g.read::<u32>(s + 0xeb4);
    g.write::<u32>(obj + 0xc4, facing);
    let (m, v, wv) = (
        ebp.wrapping_sub(0x64),
        ebp.wrapping_sub(0x14),
        ebp.wrapping_sub(0x24),
    );
    for (i, x) in (0u32..).zip([0xc0c0_0000u32, 0xc1c0_0000, 0, ONE]) {
        g.write::<u32>(v + 4 * i, x);
    }
    // call width: 1
    g.call(slot(g, 235), 0, &[m]);
    // call width: 3
    g.call(slot(g, 328), 0, &[m, m, 0]);
    // call width: 3
    g.call(slot(g, 329), 0, &[m, m, facing]);
    g.write::<u32>(v + 0xc, ONE);
    // call width: 3
    g.call(slot(g, 229), 0, &[wv, m, v]);
    let p = obj + 0xd0;
    // call width: 3
    g.call(slot(g, 248), 0, &[p, p, wv]);
    g.write::<u32>(p + 0xc, ONE);
    let b = u32::from(g.read::<u8>(g.read::<u32>(w(0x127))));
    // call width: 2
    let model = g.call(slot(g, 219), 0, &[b, 0]).eax;
    // call width: 3
    Ret::eax(g.call(slot(g, 268), 0, &[model, 0xe, p]).eax)
}

/// [`ring_object_0430`] (355 bytes, 1 overlay).
pub const RING_OBJECT_0430: Sig = Sig {
    len: 355,
    hash: 0x5615_280b_4a7c_31c8,
    calls: &[
        (0x31, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        // __security_check_cookie
        (0x15b, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0430`'s part matrix (0x10002a60, 360 bytes; cdecl `(a, b)`; an
/// `ebp` /GS frame: `M` `ebp - 0x54`, `V` `ebp - 0x14`, `L` `ebp -
/// 0x24`): `P` = slot 941 `(a + 0x164)`'s answer, slot 941 `(a +
/// 0x168)`; `i` = byte `*(a + 8) + 0xe`; `M` the unit with the diagonal
/// `b + 0x60..0x68` (slot 235) through slot 554 `(M, M, b + 0xc0)`; slot
/// 552 `(&L, b + 0xd0)`; `V = M (P, 1)` (slot 229) into the state's
/// record `i` (`S + 0xbc4 + 16 i`, `w` 10.0) and `S + 0xc04 + 16 i` =
/// (0.5, 0.5, 0.5, 1). Returns `S`.
pub fn part_matrix_0430<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, n: u32| slot_target(g, w(0x1e), n);
    let ebp = fr.esp.wrapping_sub(4);
    let s = g.read::<u32>(w(0x11));
    let (a, b) = (fr.arg(g, 0), fr.arg(g, 1));
    let x = g.read::<u32>(a + 0x164);
    // call width: 1
    let p = g.call(slot(g, 941), 0, &[x]).eax;
    let x = g.read::<u32>(a + 0x168);
    // call width: 1
    g.call(slot(g, 941), 0, &[x]);
    let i = u32::from(g.read::<u8>(g.read::<u32>(a + 8) + 0xe));
    let (m, v, l) = (
        ebp.wrapping_sub(0x54),
        ebp.wrapping_sub(0x14),
        ebp.wrapping_sub(0x24),
    );
    // call width: 1
    g.call(slot(g, 235), 0, &[m]);
    for (o, d) in [(0x60u32, 0u32), (0x64, 0x14), (0x68, 0x28)] {
        let x = g.read::<u32>(b + o);
        g.write::<u32>(m + d, x);
    }
    // call width: 3
    g.call(slot(g, 554), 0, &[m, m, b + 0xc0]);
    // call width: 2
    g.call(slot(g, 552), 0, &[l, b + 0xd0]);
    for j in 0..3u32 {
        let x = g.read::<u32>(p + 4 * j);
        g.write::<u32>(v + 4 * j, x);
    }
    g.write::<u32>(v + 0xc, ONE);
    // call width: 3
    g.call(slot(g, 229), 0, &[v, m, v]);
    let r = s + 0x10 * i;
    for j in 0..3u32 {
        let x = g.read::<u32>(v + 4 * j);
        g.write::<u32>(r + 0xbc4 + 4 * j, x);
    }
    g.write::<u32>(r + 0xbd0, 0x4120_0000);
    for j in 0..3u32 {
        g.write::<u32>(r + 0xc04 + 4 * j, 0x3f00_0000);
    }
    g.write::<u32>(r + 0xc10, ONE);
    Ret::eax(s)
}

/// [`part_matrix_0430`] (360 bytes, 1 overlay).
pub const PART_MATRIX_0430: Sig = Sig {
    len: 360,
    hash: 0x4dae_4703_44b1_3ede,
    calls: &[
        // __security_check_cookie
        (0x160, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0430`'s objects (0x10002bd0, 418 bytes; no arguments, no
/// frame): objects 1 (`S + 0x130`), 0 (`S`) and 7 (`S + 0x850`, value
/// 0x50) started ([`start_object_anim`](crate::s4_misc::start_object_anim))
/// at the state's point and facing, their `y` less `K` (the third first
/// plus `K'`); the callback record `S + 0xab0` (three words the callback,
/// two 0) set as object 7's `+0x10c`. Returns the record.
pub fn objects_0430<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let s = g.read::<u32>(w(0x0c));
    let start = |g: &mut G, call: u32, obj: u32, scale: u32, (index, value): (u32, u32)| {
        let at = img.rel32_target(f + call);
        crate::s4_misc::start_object_anim(g, img, at, obj, k(g, scale), index, value);
    };
    let o1 = s + 0x130;
    start(g, 0x20, o1, 0x04, (1, 0));
    place_at_0430(g, s, o1);
    let y = sub(g.read::<f32>(o1 + 0xd4), k(g, 0x68));
    let facing = g.read::<u32>(s + 0xeb4);
    g.write::<u32>(o1 + 0xc4, facing);
    g.write::<f32>(o1 + 0xd4, y);
    start(g, 0x8a, s, 0x2e, (0, 0));
    place_at_0430(g, s, s);
    let y = sub(g.read::<f32>(s + 0xd4), k(g, 0xd2));
    let facing = g.read::<u32>(s + 0xeb4);
    g.write::<u32>(s + 0xc4, facing);
    g.write::<f32>(s + 0xd4, y);
    let o7 = s + 0x850;
    start(g, 0xfa, o7, 0x98, (7, 0x50));
    place_at_0430(g, s, o7);
    let y = add(g.read::<f32>(o7 + 0xd4), k(g, 0x128));
    g.write::<f32>(o7 + 0xd4, y);
    let facing = g.read::<u32>(s + 0xeb4);
    let y = sub(y, k(g, 0x153));
    g.write::<u32>(o7 + 0xc4, facing);
    let r = s + 0xab0;
    g.write::<f32>(o7 + 0xd4, y);
    g.write::<u32>(s + 0xabc, 0);
    g.write::<u32>(s + 0xac0, 0);
    g.write::<u32>(s + 0xab8, w(0x185));
    g.write::<u32>(s + 0xab4, w(0x18f));
    g.write::<u32>(r, w(0x195));
    g.write::<u32>(o7 + 0x10c, r);
    Ret::eax(r)
}

/// [`objects_0430`] (418 bytes, 1 overlay).
pub const OBJECTS_0430: Sig = Sig {
    len: 418,
    hash: 0x532f_a3d7_9a63_d307,
    calls: &[
        (0x20, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0x8a, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
        (0xfa, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
    ],
    imports: &[],
};

/// `magic_0430`'s summon matrix (0x10003350, 736 bytes; no arguments; an
/// `ebp` /GS frame: `M` `ebp - 0x74`, `V` `ebp - 0x14`, `W` `ebp - 0x24`,
/// `L` `ebp - 0x34`): `V = (-20, 1.65 when slot 520 answers 0x18b0046
/// else 0, -50, 1)`; the place `S + 0xc54` plus `M V` with `M` the unit
/// turned by the facing `S + 0xeb4` (slots 235, 328 `(0)`, 329, 229,
/// 248), `w` 1.0; the matrix `X = S + 0xb44`: slot 210 `(T[sel[1]], X)`
/// (with `X` 0, which cannot be, slots 217/255 for a party member and
/// slot 211 instead), `S + 0xc84..` = `S + 0xc44..` (eight words), `X`
/// the unit (slot 235) with its diagonal slot 277's scale times `K` times
/// slots 15, 16, 17 of the target (ST0), turned by slots 330, 328, 329
/// with `S + 0xc8c`, `+0xc84`, `+0xc88`. Returns slot 329's `eax`.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn summon_matrix_0430<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x11), n);
    let ebp = fr.esp.wrapping_sub(4);
    let at = |o: u32| ebp.wrapping_sub(o);
    let (m, v, wv, l) = (at(0x74), at(0x14), at(0x24), at(0x34));
    let s = g.read::<u32>(w(0x20));
    g.write::<u32>(v, 0xc1a0_0000);
    let facing = g.read::<u32>(s + 0xeb4);
    g.write::<u32>(v + 4, 0);
    // call width: 0
    if g.call(slot(g, 520), 0, &[]).eax == 0x018b_0046 {
        g.write::<u32>(v + 4, 0x3fd3_3333);
    }
    g.write::<u32>(v + 8, 0xc248_0000);
    g.write::<u32>(v + 0xc, ONE);
    // call width: 1
    g.call(slot(g, 235), 0, &[m]);
    // call width: 3
    g.call(slot(g, 328), 0, &[m, m, 0]);
    // call width: 3
    g.call(slot(g, 329), 0, &[m, m, facing]);
    g.write::<u32>(v + 0xc, ONE);
    // call width: 3
    g.call(slot(g, 229), 0, &[wv, m, v]);
    let p = s + 0xc54;
    // call width: 3
    g.call(slot(g, 248), 0, &[p, p, wv]);
    g.write::<u32>(p + 0xc, ONE);
    let x = s + 0xb44;
    let b = u32::from(g.read::<u8>(g.read::<u32>(w(0xe7)) + 1));
    let t = g.read::<u32>(w(0xfb).wrapping_add(4 * b));
    if x == 0 {
        if g.read::<u32>(w(0x11c)) == 0 && b < 0x20 {
            for s2 in [217u32, 255] {
                // call width: 2
                g.call(slot(g, s2), 0, &[b, l]);
            }
        }
    } else {
        // call width: 2
        g.call(slot(g, 210), 0, &[t, x]);
    }
    // call width: 2
    g.call(slot(g, 211), 0, &[t, x]);
    for i in 0..8u32 {
        let y = g.read::<u32>(s + 0xc44 + 4 * i);
        g.write::<u32>(s + 0xc84 + 4 * i, y);
    }
    // call width: 1
    g.call(slot(g, 235), 0, &[x]);
    let target = |g: &G, sel: u32, tab: u32| {
        let b = u32::from(g.read::<u8>(g.read::<u32>(w(sel)) + 1));
        g.read::<u32>(w(tab).wrapping_add(4 * b))
    };
    let t = target(g, 0x1d0, 0x1db);
    // call width: 1
    let sc = mul(st0_f32(g.call(slot(g, 277), 0, &[t]).st0), k(g, 0x1f8));
    for o in [0x28u32, 0x14, 0] {
        g.write::<f32>(x + o, sc);
    }
    let t = target(g, 0x20b, 0x216);
    for (n, o) in [(15u32, 0u32), (16, 0x14), (17, 0x28)] {
        // call width: 1
        let r = st0_f32(g.call(slot(g, n), 0, &[t]).st0);
        let v = mul(g.read::<f32>(x + o), r);
        g.write::<f32>(x + o, v);
    }
    let mut eax = 0;
    for (n, o) in [(330u32, 0xc8cu32), (328, 0xc84), (329, 0xc88)] {
        let a = g.read::<u32>(s + o);
        // call width: 3
        eax = g.call(slot(g, n), 0, &[x, x, a]).eax;
    }
    Ret::eax(eax)
}

/// [`summon_matrix_0430`] (736 bytes, 1 overlay).
pub const SUMMON_MATRIX_0430: Sig = Sig {
    len: 736,
    hash: 0x4813_fa44_df78_9a92,
    calls: &[
        // __security_check_cookie
        (0x2d8, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0430`'s glow draw (0x10002ea0, 551 bytes; no arguments; a
/// 16-aligned /GS frame `E`: the draw's descriptor `D` at `E + 8`, the
/// matrices `A` `E + 0x80`, `B` `E + 0xc0`, `C` `E + 0x100`): for `T =
/// T[sel[0]]` slots 357 `(T, -1, 0x3c00)`, 356 `(T, -1, 0x2ee0)`, 355 and
/// 354 `(T, -1)`; the count `S + 0xec4` steps down while above 0, and with
/// `n` its value the descriptor: `+0` 0x5c, `+4` slot 562, the halfword
/// `+8` 0x8044, the byte `+0xa` 0, `+0xc`..`+0x14` 0, `+0x18` 0x80808080,
/// `+0x1c` 0, `+0x20` `B`, `+0x24` `S + 0xc04`, `+0x28` `S + 0xbc4`,
/// `+0x2c` `S + 0xac4`, `+0x34`..`+0x48` 0, the bytes `+0x4c` `(200 - n)
/// 64 / 100` and `+0x4d` `n 64 / 100`; `C` = slot 723, `A` = slot 724,
/// `B = A (S + 0xac4)` (slot 231); slot 567 `(*G_D, D, S + 0xac4, A, C)`;
/// slot 382 `(0, 0)`; slots 357 and 356 `(T, -1, -1)`. Returns slot 356's
/// `eax`.
///
/// Junk reads: the descriptor's byte `+0xb`, word `+0x30` and bytes
/// `+0x4e`/`+0x4f` are never written; 0 (brief 2026-10-24).
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn glow_draw_0430<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, n: u32| slot_target(g, w(0x2c), n);
    let e = (fr.esp.wrapping_sub(4) & !0xf).wrapping_sub(0x148);
    let target = |g: &G, sel: u32, tab: u32| {
        let b = u32::from(g.read::<u8>(g.read::<u32>(w(sel))));
        g.read::<u32>(w(tab).wrapping_add(4 * b))
    };
    let t = target(g, 0x1b, 0x27);
    let s = g.read::<u32>(w(0x32));
    // call width: 3
    g.call(slot(g, 357), 0, &[t, u32::MAX, 0x3c00]);
    // call width: 3
    g.call(slot(g, 356), 0, &[t, u32::MAX, 0x2ee0]);
    let t = target(g, 0x5c, 0x68);
    for n in [355u32, 354] {
        // call width: 2
        g.call(slot(g, n), 0, &[t, u32::MAX]);
    }
    let c = g.read::<i32>(s + 0xec4);
    if c > 0 {
        g.write::<i32>(s + 0xec4, c - 1);
    }
    let d = e + 8;
    g.write::<u32>(d, 0x5c);
    // call width: 0
    let r = g.call(slot(g, 562), 0, &[]).eax;
    g.write::<u32>(d + 4, r);
    let n = g.read::<i32>(s + 0xec4);
    for o in [0xcu32, 0x10, 0x14, 0x1c, 0x34, 0x38, 0x3c, 0x40, 0x44, 0x48] {
        g.write::<u32>(d + o, 0);
    }
    // junk read: the descriptor's byte +0xb, word +0x30 and bytes
    // +0x4e/+0x4f never written; 0 (brief 2026-10-24)
    g.write::<u8>(d + 0xb, 0);
    g.write::<u32>(d + 0x30, 0);
    g.write::<u16>(d + 0x4e, 0);
    let a = 0xc8i32.wrapping_sub(n).wrapping_shl(6) / 100;
    let b = n.wrapping_shl(6) / 100;
    g.write::<u8>(d + 0x4d, b.to_le_bytes()[0]);
    g.write::<u16>(d + 8, 0x8044);
    g.write::<u8>(d + 0xa, 0);
    g.write::<u32>(d + 0x18, 0x8080_8080);
    g.write::<u8>(d + 0x4c, a.to_le_bytes()[0]);
    let (am, bm, cm) = (e + 0x80, e + 0xc0, e + 0x100);
    // call width: 1
    g.call(slot(g, 723), 0, &[cm]);
    // call width: 1
    g.call(slot(g, 724), 0, &[am]);
    let x = s + 0xac4;
    // call width: 3
    g.call(slot(g, 231), 0, &[bm, am, x]);
    g.write::<u32>(d + 0x20, bm);
    g.write::<u32>(d + 0x24, s + 0xc04);
    g.write::<u32>(d + 0x28, s + 0xbc4);
    let gd = g.read::<u32>(w(0x1bc));
    g.write::<u32>(d + 0x2c, x);
    // call width: 5
    g.call(slot(g, 567), 0, &[gd, d, x, am, cm]);
    // call width: 2
    g.call(slot(g, 382), 0, &[0, 0]);
    let t = target(g, 0x1de, 0x1ea);
    // call width: 3
    g.call(slot(g, 357), 0, &[t, u32::MAX, u32::MAX]);
    // call width: 3
    Ret::eax(g.call(slot(g, 356), 0, &[t, u32::MAX, u32::MAX]).eax)
}

/// [`glow_draw_0430`] (551 bytes, 1 overlay).
pub const GLOW_DRAW_0430: Sig = Sig {
    len: 551,
    hash: 0xbf08_f285_d3dc_20f9,
    calls: &[
        // __security_check_cookie
        (0x21f, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0393`'s rise start (0x10002e50, 280 bytes; an overlay table
/// entry, no arguments; the block `S = *G_S`, operand `+0x04`): `S +
/// 0x1ba0..` = `S + 0x1c10..` with the second word less `K` (`+0x1a`);
/// for the actor `T = T[sel[1]]` slot 632 `(T, 0, 0)` unless `S + 0x1eec`
/// is -1, then slot 632 `(T, *G_C, 0x12)`; object `S + 0x850` started
/// ([`start_object_at`](crate::s4_misc::start_object_at): scale `*K2`,
/// index 7, owners 0 and 0), its callback `+0x104` the immediate `+0xb0`,
/// byte `+0x126` 2, `+0x120` 6; slot 257 `(slot 219 (*G_D, 0), 1)`; with
/// `S + 0x1ee8` 6, slot 278 `(slot 215 (6), 1, 1)`. Returns the last
/// slot's `eax`.
///
/// The start pushes the `ecx` slot 632 leaves behind as its third word,
/// which the start does not read.
pub fn rise_start_0393<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, n: u32| slot_target(g, w(0xa6), n);
    let s = g.read::<u32>(w(0x4));
    let a = g.read::<u32>(s + 0x1c10);
    let y = sub(g.read::<f32>(s + 0x1c14), g.read::<f32>(w(0x1a)));
    g.write::<u32>(s + 0x1ba0, a);
    let a = g.read::<u32>(s + 0x1c18);
    g.write::<u32>(s + 0x1ba8, a);
    let a = g.read::<u32>(s + 0x1c1c);
    g.write::<u32>(s + 0x1bac, a);
    g.write::<f32>(s + 0x1ba4, y);
    let b = u32::from(g.read::<u8>(g.read::<u32>(w(0x4c)) + 1));
    let t = g.read::<u32>(w(0x57).wrapping_add(4 * b));
    if g.read::<u32>(s + 0x1eec) != u32::MAX {
        // call width: 3
        g.call(slot(g, 632), 0, &[t, 0, 0]);
    }
    let c = g.read::<u32>(w(0x7b));
    // call width: 3
    g.call(slot(g, 632), 0, &[t, c, 0x12]);
    let scale = g.read::<f32>(w(0x8c));
    crate::s4_misc::start_object_at(
        g,
        img,
        img.rel32_target(f + 0xa1),
        s + 0x850,
        scale,
        7,
        0,
        0,
    );
    g.write::<u32>(s + 0x954, w(0xb0));
    g.write::<u8>(s + 0x976, 2);
    g.write::<u32>(s + 0x970, 6);
    let d = g.read::<u32>(w(0xcf));
    // call width: 2
    let r = g.call(slot(g, 219), 0, &[d, 0]).eax;
    // call width: 2
    let mut eax = g.call(slot(g, 257), 0, &[r, 1]).eax;
    if g.read::<u32>(s + 0x1ee8) == 6 {
        // call width: 1
        let r = g.call(slot(g, 215), 0, &[6]).eax;
        // call width: 3
        eax = g.call(slot(g, 278), 0, &[r, 1, 1]).eax;
    }
    Ret::eax(eax)
}

/// [`rise_start_0393`] (280 bytes, 1 overlay).
pub const RISE_START_0393: Sig = Sig {
    len: 280,
    hash: 0xc4cb_61c6_dadd_75ca,
    calls: &[(0xa1, Callee::Sig(&crate::s4_misc::START_OBJECT_AT))],
    imports: &[],
};

/// `magic_0393`'s caster rise step (0x100039a0, 804 bytes; an overlay
/// table entry, no arguments), `magic_0450`'s
/// [`rise_0450`](crate::r16::rise_0450) with the counter `n = B + 0x1ef4`
/// and the second object chosen: while `n` is above 0 the sparks
/// ([`sparks_p034`](crate::r11_misc::sparks_p034)) and `n` one less, `s
/// = n / K` (else `s` = `K0`, 1.0); `B + 0x1c00` = `B + 0x1c30 s + B +
/// 0x1c40 (K0 - s)` (`w` 1.0). Unless `n` is now 0 that is all (returns
/// the sparks' `eax`, or the entry's). Else `O = B + 0x130` leaves the
/// DLL's list ([`list_remove`](crate::l5_objhost::list_remove); when it
/// was there `*G_C = CB`, slot 713 `(O)` and bit 15 of `O + 0x124`
/// cleared); the index `i` is 3 when slot 390 `(sel[0])` is in the table
/// `G_L` (ended by -1) or slot 534 `(sel[0])` answers non-zero, else 4;
/// the object `B + 0x130 i` starts at `B + 0x1c40`
/// ([`start_object_at`](crate::s4_misc::start_object_at): index `i`,
/// `owner_b` the callback `+0x177`, scale `*K2`), its yaw `B + 0x1ee0`;
/// the record `R = B + 0x1c60` its colour bytes and counts; a ring node
/// (callback `+0x287`, `+0x14 = R`) last on the task list; the actor `a =
/// sel[1]`: slot 296 `(a, 0)` outside a battle while `a` is below 0x20,
/// else slot 257 `(T[a], 1)`; `n` one less. Returns the last slot's
/// `eax`.
///
/// With the ring empty the original writes through node 0 (it faults);
/// so does this. The start pushes the `ecx` left behind as its third
/// word, which it does not read.
#[allow(clippy::too_many_lines)] // the original's stores, in order
pub fn rise_0393<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let b = g.read::<u32>(w(0xb));
    let at = |o: u32| b.wrapping_add(o);
    let mut eax = fr.eax;
    let (s, one) = if g.read::<i32>(at(0x1ef4)) > 0 {
        // push edi; call
        let sparks = Frame {
            at: img.rel32_target(f + 0x19),
            esp: fr.esp.wrapping_sub(8),
            ..*fr
        };
        eax = crate::r11_misc::sparks_p034(g, img, &sparks).eax;
        let n = g.read::<u32>(at(0x1ef4)).wrapping_sub(1);
        g.write::<u32>(at(0x1ef4), n);
        (div(crate::sb_sse::cvt(n), k(g, 0x3a)), k(g, 0x2f))
    } else {
        (k(g, 0x04), k(g, 0x04))
    };
    let km = sub(one, s);
    let p = |g: &G, o: u32| g.read::<f32>(at(o));
    let x = add(mul(p(g, 0x1c30), s), mul(p(g, 0x1c40), km));
    let y = add(mul(p(g, 0x1c34), s), mul(km, p(g, 0x1c44)));
    let z = add(mul(p(g, 0x1c38), s), mul(km, p(g, 0x1c48)));
    g.write::<f32>(at(0x1c00), x);
    g.write::<f32>(at(0x1c04), y);
    g.write::<u32>(at(0x1c0c), ONE);
    g.write::<f32>(at(0x1c08), z);
    if g.read::<u32>(at(0x1ef4)) != 0 {
        return Ret::eax(eax);
    }
    let o = at(0x130);
    let removed = crate::l5_objhost::list_remove(g, img, img.rel32_target(f + 0xd5), w(0xd0), o);
    if removed != 0 {
        let c = g.read::<u32>(w(0xde));
        g.write::<u32>(c, w(0xe5));
        // call width: 1
        g.call(slot_target(g, w(0xea), 713), 0, &[o]);
        let h = g.read::<u16>(o + 0x124) & 0x7fff;
        g.write::<u16>(o + 0x124, h);
    }
    let a = u32::from(g.read::<u8>(g.read::<u32>(w(0x106))));
    // call width: 1
    let r = g.call(slot_target(g, w(0x10f), 390), 0, &[a]).eax;
    let mut p = w(0x125);
    let mut c = g.read::<u32>(w(0x11d));
    let mut found = false;
    while c != u32::MAX {
        if c == r {
            found = true;
            break;
        }
        p = p.wrapping_add(4);
        c = g.read::<u32>(p);
    }
    let i = if found {
        3
    } else {
        let a = u32::from(g.read::<u8>(g.read::<u32>(w(0x140))));
        // call width: 1
        if g.call(slot_target(g, w(0x149), 534), 0, &[a]).eax == 0 {
            4
        } else {
            3
        }
    };
    let o2 = b.wrapping_add(0x130 * i);
    let start = img.rel32_target(f + 0x184);
    crate::s4_misc::start_object_at(g, img, start, o2, k(g, 0x16a), i, 0, w(0x177));
    let count = g.read::<u16>(w(0x191));
    for c in [0u32, 4, 8, 0xc] {
        let v = g.read::<u32>(at(0x1c40 + c));
        g.write::<u32>(o2 + 0xd0 + c, v);
    }
    let yaw = g.read::<u32>(at(0x1ee0));
    g.write::<u32>(o2 + 0xc4, yaw);
    let r = at(0x1c60);
    g.write::<u32>(r + 0x81, 0xff80_ffff);
    g.write::<u8>(r + 0x80, 0xff);
    g.write::<u16>(r + 0x85, 0xffff);
    g.write::<u8>(r + 0x87, 0);
    g.write::<u32>(r + 0x88, 0);
    g.write::<u16>(r + 0x98, 0x48);
    g.write::<u32>(r + 0x90, 0x14);
    g.write::<u32>(r + 0x8c, 0x14);
    g.write::<u32>(r + 0x94, 0);
    let node = if count == 0 {
        // none free: the original writes through 0 (it faults)
        0
    } else {
        let node = g.read::<u32>(w(0x231));
        crate::r13_misc::pool_pop(g, img, f, node, (0x191, 0x24e, 0x256, 0x287, 0x292));
        node
    };
    let any = g.read::<u32>(w(0x298));
    g.write::<u32>(node.wrapping_add(0x14), r);
    g.write::<u32>(node.wrapping_add(4), 0);
    if any == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x2bf), node);
    } else {
        let tail = g.read::<u32>(w(0x2ac));
        g.write::<u32>(node, tail);
        g.write::<u32>(tail.wrapping_add(4), node);
    }
    g.write::<u32>(node.wrapping_add(8), node);
    let battle = g.read::<u32>(w(0x2c8)) != 0;
    let actor = g.read::<u32>(w(0x2cf));
    g.write::<u32>(w(0x2d5), node);
    let a = u32::from(g.read::<u8>(actor.wrapping_add(1)));
    let eax = if !battle && a < 0x20 {
        // call width: 2
        g.call(slot_target(g, w(0x2e8), 296), 0, &[a, 0]).eax
    } else {
        let m = g.read::<u32>(a.wrapping_mul(4).wrapping_add(w(0x308)));
        // call width: 2
        g.call(slot_target(g, w(0x30d), 257), 0, &[m, 1]).eax
    };
    let n = g.read::<u32>(at(0x1ef4)).wrapping_sub(1);
    g.write::<u32>(at(0x1ef4), n);
    Ret::eax(eax)
}

/// [`rise_0393`] (804 bytes, 1 overlay).
pub const RISE_0393: Sig = Sig {
    len: 804,
    hash: 0x82e5_37fb_29b5_350f,
    calls: &[
        (0x19, Callee::Sig(&crate::r11_misc::SPARKS_0393)),
        (0xd5, Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
        (0x184, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
    ],
    imports: &[],
};

/// `magic_0393`'s caster drop step (0x10003e40, 884 bytes; an overlay
/// table entry, no arguments; an `ebp` /GS frame, `L` `ebp - 0x14`): with
/// the block `B = *G_B` and its counter `n = B + 0x1efc`: nothing while
/// `n` is at most 0 (returns `n`); else `n` one less, `s = n / K`, `B +
/// 0x1ba0` = `B + 0x1c30 s + B + 0x1c10 (K0 - s)` (`w` 1.0); the yaw `B +
/// 0x1b94` plus `K1`, wrapped once (less `T` past `H`, plus `T2` below
/// `Lo`); the matrix `M = B + 0x1b50` rebuilt as in
/// [`caster_start_0450`](crate::r16::caster_start_0450) (slots 235, 277
/// `(*G_T)` times `K2`, 15-17 `(*G_T)`, 330, 328, 329 with the angles `B
/// + 0x1b98`, `+0x1b90`, `+0x1b94`). Once `n` is 0: outside a battle
/// with the actor `a = *G_A` below 0x20 slots 217 and 255 `(a, L)`; slot
/// 211 `(*G_T, 0)`; the actor record (`*G_A`, the table `+0x27c`, -1,
/// slot 215 or in a battle 413 `(*G_A)`, -1, 0, 1, the halfword 0 or in
/// a battle 1); object `B` started
/// ([`start_object_at`](crate::s4_misc::start_object_at): scale `*K3`,
/// index 0, owners 0), `B + 0xd0..` = `B + 0x1c10..`, its yaw `B + 0xc4`
/// = `B + 0x1ee0`, `B + 0xd4` less `K4`; the scale task
/// ([`scale_task`](crate::s4_misc::scale_task): `ecx` `B + 0x1e08`,
/// scales `K4` and `K0`, count 0), whose `eax` is returned (else slot
/// 329's).
///
/// The start pushes the `ecx` 1 as its third word, which it does not
/// read.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn drop_0393<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let b = g.read::<u32>(w(0x13));
    let at = |o: u32| b.wrapping_add(o);
    let n = g.read::<u32>(at(0x1efc));
    if n.cast_signed() <= 0 {
        return Ret::eax(n);
    }
    let n = n.wrapping_sub(1);
    let s = div(crate::sb_sse::cvt(n), k(g, 0x39));
    g.write::<u32>(at(0x1efc), n);
    let km = sub(k(g, 0x31), s);
    let p = |g: &G, o: u32| g.read::<f32>(at(o));
    let x = add(mul(p(g, 0x1c30), s), mul(km, p(g, 0x1c10)));
    let y = add(mul(p(g, 0x1c34), s), mul(p(g, 0x1c14), km));
    let z = add(mul(p(g, 0x1c38), s), mul(p(g, 0x1c18), km));
    g.write::<f32>(at(0x1ba0), x);
    g.write::<f32>(at(0x1ba4), y);
    g.write::<u32>(at(0x1bac), ONE);
    g.write::<f32>(at(0x1ba8), z);
    let mut a = add(p(g, 0x1b94), k(g, 0xc8));
    // `comiss` + `jbe`: an unordered yaw stays
    if a > k(g, 0xcf) {
        a = sub(a, k(g, 0xd9));
    } else if k(g, 0xe3) > a {
        a = add(a, k(g, 0xf0));
    }
    let slot = |g: &G, n: u32| slot_target(g, w(0xf5), n);
    let m = at(0x1b50);
    g.write::<f32>(at(0x1b94), a);
    // call width: 1
    g.call(slot(g, 235), 0, &[m]);
    let tt = g.read::<u32>(w(0x119));
    // call width: 1
    let sc = mul(st0_f32(g.call(slot(g, 277), 0, &[tt]).st0), k(g, 0x131));
    for o in [0x28u32, 0x14, 0] {
        g.write::<f32>(m + o, sc);
    }
    let t = g.read::<u32>(w(0x14a));
    // call width: 1
    let r = st0_f32(g.call(slot(g, 15), 0, &[t]).st0);
    let v = mul(g.read::<f32>(m), r);
    g.write::<f32>(m, v);
    // call width: 1
    let r = st0_f32(g.call(slot(g, 16), 0, &[t]).st0);
    let v = mul(g.read::<f32>(m + 0x14), r);
    g.write::<f32>(m + 0x14, v);
    // call width: 1
    let r = st0_f32(g.call(slot(g, 17), 0, &[t]).st0);
    let v = mul(r, g.read::<f32>(m + 0x28));
    g.write::<f32>(m + 0x28, v);
    let mut eax = 0;
    for (n, o) in [(330u32, 0x1b98u32), (328, 0x1b90), (329, 0x1b94)] {
        let a = g.read::<u32>(at(o));
        // call width: 3
        eax = g.call(slot(g, n), 0, &[m, m, a]).eax;
    }
    if g.read::<u32>(at(0x1efc)) != 0 {
        return Ret::eax(eax);
    }
    let ebp = fr.esp.wrapping_sub(4);
    let l = ebp.wrapping_sub(0x14);
    let tt = g.read::<u32>(w(0x214));
    if g.read::<u32>(w(0x20d)) == 0 {
        let a = g.read::<u32>(w(0x21c));
        if a.cast_signed() < 0x20 {
            for n in [217u32, 255] {
                // call width: 2
                g.call(slot(g, n), 0, &[a, l]);
            }
        }
    }
    // call width: 2
    g.call(slot(g, 211), 0, &[tt, 0]);
    let a = g.read::<u32>(w(0x25d));
    g.write::<u32>(w(0x26c), a);
    g.write::<u32>(w(0x278), w(0x27c));
    g.write::<u32>(w(0x282), u32::MAX);
    let n = if g.read::<u32>(w(0x266)) == 0 {
        215
    } else {
        413
    };
    // call width: 1
    let r = g.call(slot(g, n), 0, &[a]).eax;
    let battle = g.read::<u32>(w(0x2a1)) != 0;
    g.write::<u32>(w(0x2ac), r);
    g.write::<u32>(w(0x2b2), u32::MAX);
    g.write::<u32>(w(0x2bc), 0);
    g.write::<u32>(w(0x2c6), 1);
    if battle {
        g.write::<u16>(w(0x2d9), 1);
    } else {
        g.write::<u16>(w(0x2d0), 0);
    }
    crate::s4_misc::start_object_at(g, img, img.rel32_target(f + 0x2ef), b, k(g, 0x2e1), 0, 0, 0);
    for c in [0u32, 4, 8, 0xc] {
        let v = g.read::<u32>(at(0x1c10 + c));
        g.write::<u32>(at(0xd0 + c), v);
    }
    let y = sub(p(g, 0xd4), k(g, 0x2fd));
    let yaw = g.read::<u32>(at(0x1ee0));
    g.write::<u32>(at(0xc4), yaw);
    g.write::<f32>(at(0xd4), y);
    // ebp, the 0x18 bytes, ebx/esi/edi; the start's four words; push 0
    let e = fr.esp.wrapping_sub(0x28);
    for (i, v) in (1u32..).zip([0u32, 0, 1, 0]) {
        g.write::<u32>(e.wrapping_sub(0x14 - 4 * i), v);
    }
    g.write::<u32>(e.wrapping_sub(0x14), 0);
    let task = Frame {
        at: img.rel32_target(f + 0x35c),
        esp: e.wrapping_sub(0x18),
        ecx: at(0x1e08),
        xmm: [
            crate::Xmm::f32(y).reg(),
            u128::from(g.read::<u32>(w(0x2fd))),
            u128::from(g.read::<u32>(w(0x305))),
            0,
        ],
        ..*fr
    };
    crate::s4_misc::scale_task(g, img, &task)
}

/// [`drop_0393`] (884 bytes, 1 overlay).
pub const DROP_0393: Sig = Sig {
    len: 884,
    hash: 0x68f3_7e47_8f63_f68c,
    calls: &[
        (0x2ef, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x35c, Callee::Sig(&crate::s4_misc::SCALE_TASK)),
        // __security_check_cookie
        (0x36c, Callee::Any),
    ],
    imports: &[],
};
