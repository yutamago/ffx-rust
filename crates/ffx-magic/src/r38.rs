//! Round 38's own ports (own rows of `magic_0250` .. `magic_0449`): the
//! DLLs' own code written out.

use crate::m0018::{st0_f32, wrap};
use crate::runtime::slot_target;
use crate::sb_sse::{add, mul, sub};
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
