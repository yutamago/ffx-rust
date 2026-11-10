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
