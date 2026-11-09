//! Round 26's ports (magic-r26): the shared rows and the own rows of
//! overlays below `magic_0450`.

use crate::m0018::st0_f32;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `magic_0246`'s start-up (0x10001340, 1153 bytes; internal, no
/// arguments, run by its load step `r9_load::load_0246`; an `ebp` /GS
/// frame; `A = *G_A` the actor, `T = TAB[*I]` the target):
/// - `*G_C` = slot 719 `(D, 8, 0)` (two globals set to their immediates
///   first); the part set up ([`part_init`](crate::l5_objhost::part_init));
///   object 0 started ([`start_object_owners`](crate::s4_misc::start_object_owners),
///   scale `K1`, `(0, 0xc, 0, 0)`);
/// - the point `P` (three globals) from slot 262 `(A, &P)`, `P.w` 1.0;
///   the yaw `K2 - slot 333 (A)` wrapped once (above `H` less `T`, below
///   `L` plus `T`) into a global, a u16 cleared, a record of nine globals;
/// - the matrix `M` (`ebp - 0x64`) from slot 235, turned by slots 328
///   `(M, M, 0)` and 329 `(M, M, yaw)`; slot 229 `(V, M, X)` with `X =
///   (0, 0, -40, 1)` (`V` at `ebp - 0x24`), slot 248 `(P, P, V)`; `P.y -=
///   K3`, `P.w` 1.0 and `Q` (four globals) = `P`;
/// - slot 262 `(T, &R)`, `s` = slot 16 `(T)` times slot 364 `(T)`, slot
///   268 `(T, 6, V)`; `R.y = V.y - s * K4 + K5`, the time 0, the counts
///   0x10 and 0x13; object 1 started (scale `K6`, `(1, 0x10, 0x14, CB)`);
/// - slot 262 `(T, &S)`, then `S` = `Q`; the actor block (nine globals,
///   [`me_block`](crate::sa_misc::me_block));
/// - two pool nodes ([`take`](crate::sa_pool::take)) for the DLL's task
///   ([`task_0246`]) and the motion-word task (`0x100017d0`), appended to
///   the task list; the done flag 0.
///
/// Returns 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn start_up_0246<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x11), n);
    let ebp = fr.esp.wrapping_sub(4);
    let at = |o: u32| ebp.wrapping_sub(o);
    let one = 0x3f80_0000u32;
    g.write::<u32>(w(0x26), w(0x2a));
    g.write::<u32>(w(0x30), w(0x34));
    // call width: 3
    let c = g.call(slot(g, 719), 0, &[w(0x20), 8, 0]).eax;
    g.write::<u32>(w(0x3e), c);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x43));
    let start = |g: &mut G, call: u32, obj: u32, scale: u32, words: [u32; 4]| {
        let [index, owner, a, b] = words;
        let at = img.rel32_target(f + call);
        crate::s4_misc::start_object_owners(g, img, at, w(obj), k(g, scale), index, owner, (a, b));
    };
    start(g, 0x5d, 0x58, 0x4b, [0, 0xc, 0, 0]);
    // the point `P` (three globals, `P.w` the fourth)
    let a = g.read::<u32>(w(0x7d));
    // call width: 4
    g.call(slot(g, 262), 0, &[a, w(0x77), w(0x72), w(0x67)]);
    let a = g.read::<u32>(w(0x8a));
    g.write::<u32>(w(0x90), one);
    // call width: 1
    let t = st0_f32(g.call(slot(g, 333), 0, &[a]).st0);
    let mut yaw = sub(k(g, 0xa7), t);
    if yaw > k(g, 0xb6) {
        yaw = sub(yaw, k(g, 0xc5));
    } else if k(g, 0xcf) > yaw {
        yaw = add(yaw, k(g, 0xdc));
    }
    g.write::<f32>(at(0x68), yaw);
    g.write::<u16>(w(0xe9), 0);
    let (m, v, x) = (at(0x64), at(0x24), at(0x14));
    g.write::<u32>(x, 0);
    g.write::<u32>(x + 4, 0);
    g.write::<f32>(w(0x102), yaw);
    for o in [0x108u32, 0x112, 0x11c, 0x126] {
        g.write::<u32>(w(o), 0);
    }
    g.write::<u32>(w(0x130), w(0x134));
    g.write::<u32>(w(0x13a), w(0x13e));
    for o in [0x144u32, 0x14e] {
        g.write::<u32>(w(o), 0);
    }
    g.write::<u32>(w(0x158), w(0x15c));
    g.write::<u32>(x + 8, 0xc220_0000);
    // call width: 1
    g.call(slot(g, 235), 0, &[m]);
    // call width: 3
    g.call(slot(g, 328), 0, &[m, m, 0]);
    // call width: 3
    g.call(slot(g, 329), 0, &[m, m, yaw.to_bits()]);
    g.write::<u32>(x + 0xc, one);
    // call width: 3
    g.call(slot(g, 229), 0, &[v, m, x]);
    // call width: 3
    g.call(slot(g, 248), 0, &[w(0x1dc), w(0x1d1), v]);
    // `Q` = `P` with `P.y` less `K3`, both `w` 1.0
    let py = sub(g.read::<f32>(w(0x1e6)), k(g, 0x1ee));
    let px = g.read::<u32>(w(0x1f6));
    g.write::<u32>(w(0x203), px);
    let pz = g.read::<u32>(w(0x20b));
    g.write::<u32>(w(0x211), one);
    g.write::<f32>(w(0x21d), py);
    g.write::<f32>(w(0x225), py);
    g.write::<u32>(w(0x22d), pz);
    g.write::<u32>(w(0x233), one);
    // the target's point `R` and its height
    let target = |g: &G, sel: u32, tab: u32| {
        let i = u32::from(g.read::<u8>(g.read::<u32>(w(sel))));
        g.read::<u32>(w(tab).wrapping_add(4 * i))
    };
    let tr = target(g, 0x1fb, 0x246);
    // call width: 4
    g.call(slot(g, 262), 0, &[tr, w(0x25b), w(0x250), w(0x23f)]);
    let ts = target(g, 0x263, 0x26d);
    // call width: 1
    let h = st0_f32(g.call(slot(g, 364), 0, &[ts]).st0);
    g.write::<f32>(at(0x6c), h);
    // call width: 1
    let s = st0_f32(g.call(slot(g, 16), 0, &[ts]).st0);
    g.write::<f32>(at(0x68), s);
    let s = mul(s, h);
    g.write::<f32>(at(0x6c), s);
    // call width: 3
    g.call(slot(g, 268), 0, &[tr, 6, v]);
    let y = sub(g.read::<f32>(v + 4), mul(s, k(g, 0x2bc)));
    let y = add(y, k(g, 0x2e3));
    g.write::<u32>(w(0x2ee), one);
    g.write::<u32>(w(0x2f8), 0);
    g.write::<u32>(w(0x302), 0x10);
    g.write::<f32>(w(0x30e), y);
    g.write::<u32>(w(0x314), 0x13);
    start(g, 0x31d, 0x2e8, 0x2c9, [1, 0x10, 0x14, w(0x2d5)]);
    let tu = target(g, 0x322, 0x33b);
    // call width: 4
    g.call(slot(g, 262), 0, &[tu, w(0x334), w(0x32f), w(0x327)]);
    for (to, from) in [
        (0x358u32, 0x350u32),
        (0x368, 0x360),
        (0x378, 0x370),
        (0x38f, 0x380),
    ] {
        let q = g.read::<u32>(w(from));
        g.write::<u32>(w(to), q);
    }
    // the actor block's record
    for o in [0x395u32, 0x39f, 0x3a9, 0x3b3] {
        g.write::<u32>(w(o), 0);
    }
    g.write::<u32>(w(0x3bd), w(0x3c1));
    g.write::<u32>(w(0x3c7), w(0x3cb));
    for o in [0x3d1u32, 0x3db] {
        g.write::<u32>(w(o), 0);
    }
    g.write::<u16>(w(0x3e5), 0);
    g.write::<u32>(w(0x3eb), w(0x3ef));
    crate::sa_misc::me_block(g, img, img.rel32_target(f + 0x3f4), w(0x387));
    // the two task nodes
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x401), w(0x3fc));
    let first = g.read::<u32>(w(0x407));
    g.write::<u32>(node + 4, 0);
    let first = if first == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x42f), node);
        node
    } else {
        let tail = g.read::<u32>(w(0x41a));
        g.write::<u32>(node, tail);
        g.write::<u32>(tail + 4, node);
        first
    };
    g.write::<u32>(node + 8, node);
    let n2 = crate::sa_pool::take(g, img, img.rel32_target(f + 0x43c), w(0x434));
    g.write::<u32>(n2 + 4, 0);
    if first == 0 {
        g.write::<u32>(n2, 0);
        g.write::<u32>(w(0x459), n2);
    } else {
        g.write::<u32>(n2, node);
        g.write::<u32>(node + 4, n2);
    }
    g.write::<u32>(n2 + 8, n2);
    g.write::<u32>(w(0x465), n2);
    g.write::<u32>(w(0x46d), 0);
    Ret::eax(0)
}

/// [`start_up_0246`] (the `rel32`s: the part set-up, the two object
/// starts, the actor block, the two pool takes,
/// `__security_check_cookie`).
pub const START_UP_0246: Sig = Sig {
    len: 1153,
    hash: 0xbc34_3026_9c22_64fc,
    calls: &[
        (0x43, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x5d, Callee::Sig(&crate::s4_misc::START_OBJECT_OWNERS)),
        (0x31d, Callee::Sig(&crate::s4_misc::START_OBJECT_OWNERS)),
        (0x3f4, Callee::Sig(&crate::sa_misc::ME_BLOCK)),
        (0x401, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x43c, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x479, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0246`'s task (0x10001810, 440 bytes; cdecl `(node)`, an `ebp`
/// /GS frame):
/// - while the delay `*G_D` runs: one less, returned;
/// - the count `*G_N` (above 0: one less) at 0: object 2 started
///   ([`start_object_owners`](crate::s4_misc::start_object_owners),
///   scale `K1`, `(2, 0, 0, 0)`), slot 325 `(O, P)`, two globals their
///   immediates, slot 262 `(T, ..)` (`T = TAB[*I]`) into the frame and its
///   `y` into a global, twelve flag bytes cleared (the `/RTCs`-style range
///   check on the index never fails), the count -1;
/// - each time: slot 326 `(G, P, Q, t)`; `t' = t + K2`; the arc `t (K1 -
///   t) K3 K4 + (K1 - t)^2 0 + t^2 0` taken off a global's `y`; once `t'`
///   reaches `K1` the done flag 1 and the node given back
///   ([`give_back`](crate::sa_pool::give_back)).
///
/// Returns the delay, or slot 326's `eax` or the give-back's.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn task_0246<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let node = fr.arg(g, 0);
    let ebp = fr.esp.wrapping_sub(4);
    let at = |o: u32| ebp.wrapping_sub(o);
    let delay = g.read::<u32>(w(0x11));
    if delay != 0 {
        let d = delay.wrapping_sub(1);
        g.write::<u32>(w(0x1f), d);
        return Ret::eax(d);
    }
    let mut n = g.read::<i32>(w(0x33));
    if n > 0 {
        n -= 1;
        g.write::<i32>(w(0x3d), n);
    }
    if n == 0 {
        let at0 = img.rel32_target(f + 0x5f);
        crate::s4_misc::start_object_owners(g, img, at0, w(0x5a), k(g, 0x4d), 2, 0, (0, 0));
        let t = slot_target(g, w(0x64), 325);
        // call width: 2
        g.call(t, 0, &[w(0x74), w(0x69)]);
        g.write::<u32>(w(0x85), w(0x89));
        g.write::<u32>(w(0x8f), w(0x93));
        let i = u32::from(g.read::<u8>(g.read::<u32>(w(0x7b))));
        let who = g.read::<u32>(w(0xa5).wrapping_add(4 * i));
        let t = slot_target(g, w(0xaa), 262);
        // call width: 4
        g.call(t, 0, &[who, at(0x14), at(0x10), at(0xc)]);
        let y = g.read::<u32>(at(0x10));
        g.write::<u32>(at(8), 0x3f80_0000);
        g.write::<u32>(w(0xcb), y);
        for i in 0..0xc {
            g.write::<u8>(w(0xda) + i, 0);
        }
        g.write::<u32>(w(0xe7), u32::MAX);
    }
    let t0 = g.read::<u32>(w(0xf3));
    let s = slot_target(g, w(0xf8), 326);
    // call width: 4
    let eax = g.call(s, 0, &[w(0x113), w(0x10e), w(0x109), t0]).eax;
    let t = g.read::<f32>(w(0x11d));
    let u = sub(k(g, 0x125), t);
    let arc = mul(mul(mul(t, u), k(g, 0x142)), k(g, 0x14e));
    let arc = add(arc, mul(mul(u, u), 0.0));
    let t2 = add(t, k(g, 0x161));
    let arc = add(arc, mul(mul(t, t), 0.0));
    let y = sub(g.read::<f32>(w(0x178)), arc);
    g.write::<f32>(w(0x180), t2);
    g.write::<f32>(w(0x18c), y);
    if t2 >= k(g, 0x16c) {
        g.write::<u32>(w(0x196), 1);
        let back = img.rel32_target(f + 0x19f);
        return Ret::eax(crate::sa_pool::give_back(g, img, back, node));
    }
    Ret::eax(eax)
}

/// [`task_0246`] (the `rel32`s: `__security_check_cookie` (twice), the
/// object start, the give-back and the range check's failure).
pub const TASK_0246: Sig = Sig {
    len: 440,
    hash: 0x78d5_7604_b943_f225,
    calls: &[
        (0x2a, Callee::Any),
        (0x5f, Callee::Sig(&crate::s4_misc::START_OBJECT_OWNERS)),
        (0x19f, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x1aa, Callee::Any),
        (0x1b3, Callee::Sig(&RANGE_CHECK_FAILURE)),
    ],
    imports: &[],
};

/// The CRT's `__report_rangecheckfailure` (`magic_0246` 0x100039e6, 12
/// bytes: `__report_securityfailure(8)`, which ends the process). Internal
/// and never reached by the ports: its one caller, [`task_0246`], checks
/// a constant index against a constant bound. No body: the `Sig` marks the
/// row covered.
pub const RANGE_CHECK_FAILURE: Sig = Sig {
    len: 12,
    hash: 0x04f1_d63e_52c5_98ff,
    calls: &[(0x6, Callee::Any)],
    imports: &[],
};

/// `magic_0180`'s start-up (0x100042f0, 1259 bytes; internal, no
/// arguments, run by its load step `r9_load::load_0180`; an 8-aligned /GS
/// frame, the locals from `S` = `(ebp & !7) - 0x88` (after `push esi`);
/// `A = *G_A` the actor, `T = TAB[*I]` the target):
/// - three globals set to their immediates, `*G_C` = slot 719 `(D, 8,
///   0)`; the part set up ([`part_init`](crate::l5_objhost::part_init));
///   object 0 started ([`start_object_at`](crate::s4_misc::start_object_at),
///   scale `K1`, `(0, 0x18, -, 0)`: the third word is the `ecx` the part
///   set-up leaves, which the start never reads);
/// - the point `P` (three globals) from slot 262 `(A, &P)`, `P.w` 1.0;
///   the yaw `K2 - slot 333 (A)` wrapped once into a global; `P.y -= 8`
///   (`K3`), a u16 and a record of nine globals set;
/// - `R` (`S + 0x10`) = slot 262 `(T, ..)`, `R.y` = `V.y` from slot 268
///   `(T, 6, V)` (`V` at `S + 0x30`), `R.w` 1; the turn `K2 - atan2(R.z -
///   P.z, R.x - P.x)` (`_CIatan2`, x87, one rounding) wrapped once;
/// - the matrix `M` (`S + 0x40`) from slot 235, turned by slots 328 `(M,
///   M, 0)` and 329 `(M, M, turn)`; slot 229 `(V, M, X)` with `X = (-0.5,
///   0, -10.5, 1)` (`S + 0x20`), slot 248 `(P, P, V)`;
/// - the spark record (`Q` = `P`, `R` into four globals, the spark set-up
///   [`sparks_0180`] and its step as callbacks, counts, colours) and its
///   sprite ([`sprite_init_colour`](crate::sb_misc::sprite_init_colour),
///   `ecx` the sprite, `edx` the owner `**G_O`, the stack word `Q`);
/// - four globals, the aim task ([`aim_task`](crate::s5_misc::aim_task),
///   `xmm1` = 8.0); a global 0 and the actor block
///   ([`me_block`](crate::sa_misc::me_block));
/// - two task nodes ([`take`](crate::sa_pool::take), bytes `+0x18` 1 and
///   2) appended to the task list, the second after the pool reset
///   ([`pool_reset_take`](crate::o6_fam_pool::pool_reset_take)).
///
/// Returns 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn start_up_0180<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::sub;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x1b), n);
    let ebp = fr.esp.wrapping_sub(4);
    // `S`: the frame after `push esi`
    let s0 = (ebp & !7).wrapping_sub(0x88);
    let at = |o: u32| s0.wrapping_add(o);
    let one = 0x3f80_0000u32;
    for (o, v) in [(0x31u32, 0x35u32), (0x3b, 0x3f), (0x45, 0x49)] {
        g.write::<u32>(w(o), w(v));
    }
    // call width: 3
    let c = g.call(slot(g, 719), 0, &[w(0x2b), 8, 0]).eax;
    g.write::<u32>(w(0x53), c);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x58));
    // (the third stack word, the `ecx` the part set-up leaves, is never
    // read by the start)
    let start = img.rel32_target(f + 0x74);
    crate::s4_misc::start_object_at(g, img, start, w(0x68), k(g, 0x60), 0, 0x18, 0);
    let a = g.read::<u32>(w(0x94));
    // call width: 4
    g.call(slot(g, 262), 0, &[a, w(0x8e), w(0x89), w(0x7e)]);
    let a = g.read::<u32>(w(0xa1));
    g.write::<u32>(w(0xa7), one);
    // call width: 1
    let t = st0_f32(g.call(slot(g, 333), 0, &[a]).st0);
    g.write::<f32>(at(8), t);
    let yaw = crate::r20::wrap_once(g, img, f, sub(k(g, 0xbf), t), (0xd7, 0xcd, 0xe7, 0xcd));
    g.write::<f32>(w(0xf8), yaw);
    let py = sub(g.read::<f32>(w(0x100)), k(g, 0x108));
    g.write::<u16>(w(0x110), 0);
    let sel = g.read::<u32>(w(0x115));
    g.write::<u32>(w(0x11b), 0);
    g.write::<f32>(w(0x127), py);
    for o in [0x12du32, 0x137, 0x141] {
        g.write::<u32>(w(o), 0);
    }
    g.write::<u32>(w(0x14b), w(0x14f));
    g.write::<u32>(w(0x155), w(0x159));
    for o in [0x15fu32, 0x169] {
        g.write::<u32>(w(o), 0);
    }
    g.write::<u32>(w(0x173), w(0x177));
    // the target's point `R` and its height `V`
    let i = u32::from(g.read::<u8>(sel));
    let who = g.read::<u32>(w(0x181).wrapping_add(4 * i));
    let (r, v, x, m) = (at(0x10), at(0x30), at(0x20), at(0x40));
    // call width: 4
    g.call(slot(g, 262), 0, &[who, r, r + 4, r + 8]);
    // call width: 3
    g.call(slot(g, 268), 0, &[who, 6, v]);
    let vy = g.read::<u32>(v + 4);
    g.write::<u32>(r + 4, vy);
    let dz = sub(g.read::<f32>(r + 8), g.read::<f32>(w(0x1cd)));
    g.write::<u32>(r + 0xc, one);
    let dx = sub(g.read::<f32>(r), g.read::<f32>(w(0x1f3)));
    // `_CIatan2(dz, dx)` on x87, the result rounded once (`cvtpd2ps`)
    #[allow(clippy::cast_possible_truncation)]
    let ang = g.atan2(f64::from(dz), f64::from(dx)) as f32;
    let turn = crate::r20::wrap_once(
        g,
        img,
        f,
        sub(k(g, 0x217), ang),
        (0x226, 0x236, 0x240, 0x24d),
    );
    g.write::<f32>(at(8), turn);
    // `X` = (-0.5, 0, -10.5), its `w` 1.0 after the turns
    for (i, b) in (0u32..).zip([0xbf00_0000u32, 0, 0xc128_0000]) {
        g.write::<u32>(x + 4 * i, b);
    }
    // call width: 1
    g.call(slot(g, 235), 0, &[m]);
    // call width: 3
    g.call(slot(g, 328), 0, &[m, m, 0]);
    // call width: 3
    g.call(slot(g, 329), 0, &[m, m, turn.to_bits()]);
    g.write::<u32>(x + 0xc, one);
    // call width: 3
    g.call(slot(g, 229), 0, &[v, m, x]);
    // call width: 3
    g.call(slot(g, 248), 0, &[w(0x2f6), w(0x2eb), v]);
    // the spark record: `Q` = `P`, `R` into four globals
    for (to, from) in [(0x30du32, 0x300u32), (0x31d, 0x315), (0x32d, 0x325)] {
        let q = g.read::<u32>(w(from));
        g.write::<u32>(w(to), q);
    }
    for (to, o) in [(0x33bu32, 0u32), (0x349, 4), (0x357, 8)] {
        let q = g.read::<u32>(r + o);
        g.write::<u32>(w(to), q);
    }
    let rw = g.read::<u32>(r + 0xc);
    let owner = g.read::<u32>(g.read::<u32>(w(0x305)));
    g.write::<u16>(w(0x369), 1);
    g.write::<u32>(w(0x36f), one);
    g.write::<u32>(w(0x379), one);
    g.write::<u32>(w(0x385), rw);
    g.write::<u32>(w(0x38b), w(0x38f));
    g.write::<u32>(w(0x395), w(0x399));
    g.write::<u32>(w(0x39f), 0x1c);
    g.write::<u32>(w(0x3a9), 0);
    g.write::<u16>(w(0x3b8), 0xffff);
    g.write::<u32>(w(0x3be), 1);
    g.write::<u16>(w(0x3ca), 1);
    // `push Q` over the nine words slots 229 and 248 left: the sprite
    // set-up's stack word, at `S - 0x28`
    let sp = s0.wrapping_sub(0x28);
    g.write::<u32>(sp, w(0x3cf));
    g.write::<u32>(w(0x3da), owner);
    g.write::<u16>(w(0x3e0), 0x200);
    g.write::<u32>(w(0x3e6), 0x8080_8080);
    g.write::<u32>(w(0x3f0), 0x3d4c_cccd);
    g.write::<u32>(w(0x3fa), 0x3d4c_cccd);
    crate::sb_misc::sprite_init_colour(g, sp.wrapping_sub(4), w(0x3d4), owner);
    g.write::<u32>(w(0x409), 0x40);
    g.write::<u32>(w(0x413), 0x3f4c_cccd);
    g.write::<u8>(w(0x41d), 0x60);
    g.write::<u32>(w(0x42c), 0x3dcc_cccd);
    let aim = img.rel32_target(f + 0x435);
    crate::s5_misc::aim_task(g, img, aim, sp.wrapping_sub(4), k(g, 0x426));
    g.write::<u32>(w(0x440), 0);
    crate::sa_misc::me_block(g, img, img.rel32_target(f + 0x449), w(0x43a));
    // the two task nodes
    let append = |g: &mut G, node: u32, byte: u8, [first, last, set_first, set_last]: [u32; 4]| {
        let listed = g.read::<u32>(w(first)) != 0;
        g.write::<u8>(node + 0x18, byte);
        g.write::<u32>(node + 4, 0);
        if listed {
            let tail = g.read::<u32>(w(last));
            g.write::<u32>(node, tail);
            g.write::<u32>(tail + 4, node);
        } else {
            g.write::<u32>(node, 0);
            g.write::<u32>(w(set_first), node);
        }
        g.write::<u32>(node + 8, node);
        g.write::<u32>(w(set_last), node);
    };
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x456), w(0x451));
    append(g, node, 1, [0x45c, 0x470, 0x482, 0x48f]);
    crate::o6_fam_pool::pool_reset_take(g, img, img.rel32_target(f + 0x494), w(0x487));
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x49e), w(0x499));
    append(g, node, 2, [0x4a4, 0x4b8, 0x4ca, 0x4da]);
    Ret::eax(0)
}

/// [`start_up_0180`] (the `rel32`s: the part set-up, the object start,
/// `_CIatan2`, the sprite set-up, the aim task, the actor block, the two
/// pool takes and the pool reset, `__security_check_cookie`).
pub const START_UP_0180: Sig = Sig {
    len: 1259,
    hash: 0xa59a_d832_d798_3ac7,
    calls: &[
        (0x58, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x74, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x205, Callee::Import("_CIatan2")),
        (0x403, Callee::Sig(&crate::sb_misc::SPRITE_INIT_COLOUR)),
        (0x435, Callee::Sig(&crate::s5_misc::AIM_TASK)),
        (0x449, Callee::Sig(&crate::sa_misc::ME_BLOCK)),
        (0x456, Callee::Sig(&crate::sa_pool::TAKE)),
        (
            0x494,
            Callee::Sig(&crate::o6_fam_pool::POOL_RESET_TAKE_0180_2140),
        ),
        (0x49e, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x4e3, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0180`'s spark set-up (0x10004870, 519 bytes; cdecl `(p)`, a
/// callback of its spark record): four times, the next free record of
/// the ring (`n = *G_N` records of 0x50 bytes at `*G_R`, from the cursor
/// `*G_I`, which wraps at `n`; free while `+0x48` is 0) is set up:
/// colours `+0x40`/`+0x44` 0x80808080, `+0x3c` 0, `+0x48` 1, the u16
/// `+0x4c` 0, `+0` = `(*G_O)[1]`, the u16s `+8` -1 and `+0xa` 0x200, `+4`
/// 1; slot 325 `(rec + 0xc, p + 0xc)`, then `+0xc..+0x14` each plus slot
/// 314 `(1.0)`; `+0x1c..+0x24` each `(slot 307 () - 1.5) * 0.5` (`+0x28`
/// 0); `+0x2c` = `(slot 404 (0.5) + K) * K'`; `+0x34`, `+0x38` 0x10, the
/// byte `+0x47` 0. A record not found (or `n` not above 0) ends it.
///
/// Returns the cursor (no free record) or the last slot 404's `eax`.
pub fn sparks_0180<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0xa4), n);
    let p = fr.arg(g, 0);
    let mut eax = 0;
    for _ in 0..4 {
        let n = g.read::<i32>(w(0x12));
        let mut cur = g.read::<i32>(w(0x17));
        if n <= 0 {
            return Ret::eax(cur.cast_unsigned());
        }
        let base = g.read::<u32>(w(0x27));
        let mut found = None;
        for _ in 0..n {
            let rec = base.wrapping_add(cur.cast_unsigned().wrapping_mul(0x50));
            cur += 1;
            if cur >= n {
                cur = 0;
            }
            if g.read::<u32>(rec + 0x48) == 0 {
                found = Some(rec);
                break;
            }
        }
        let Some(rec) = found else {
            return Ret::eax(cur.cast_unsigned());
        };
        g.write::<i32>(w(0x59), cur);
        g.write::<u32>(rec + 0x40, 0x8080_8080);
        g.write::<u32>(rec + 0x44, 0x8080_8080);
        g.write::<u32>(rec + 0x3c, 0);
        g.write::<u32>(rec + 0x48, 1);
        g.write::<u16>(rec + 0x4c, 0);
        let o = g.read::<u32>(g.read::<u32>(w(0x80)) + 4);
        g.write::<u32>(rec, o);
        g.write::<u16>(rec + 8, 0xffff);
        g.write::<u16>(rec + 0xa, 0x200);
        g.write::<u32>(rec + 4, 1);
        // call width: 2
        g.call(slot(g, 325), 0, &[rec + 0xc, p.wrapping_add(0xc)]);
        for o in [0xcu32, 0x10, 0x14] {
            // call width: 1
            let r = st0_f32(g.call(slot(g, 314), 0, &[0x3f80_0000]).st0);
            let x = add(g.read::<f32>(rec + o), r);
            g.write::<f32>(rec + o, x);
        }
        for o in [0x1cu32, 0x20, 0x24] {
            if o == 0x24 {
                g.write::<u32>(rec + 0x28, 0);
            }
            // call width: 0 (the 1.0 slot 314 got is still on the stack)
            let r = st0_f32(g.call(slot(g, 307), 0, &[0x3f80_0000]).st0);
            g.write::<f32>(rec + o, mul(sub(r, k(g, 0x144)), k(g, 0x151)));
        }
        // call width: 1
        let out = g.call(slot(g, 404), 0, &[0x3f00_0000]);
        eax = out.eax;
        let r = st0_f32(out.st0);
        g.write::<u32>(rec + 0x34, 0x10);
        g.write::<f32>(rec + 0x2c, mul(add(r, k(g, 0x1d0)), k(g, 0x1db)));
        g.write::<u32>(rec + 0x38, 0x10);
        g.write::<u8>(rec + 0x47, 0);
    }
    Ret::eax(eax)
}

/// [`sparks_0180`].
pub const SPARKS_0180: Sig = Sig {
    len: 519,
    hash: 0xc2a9_d23f_37a1_b8b0,
    calls: &[],
    imports: &[],
};

/// `magic_0426`'s mesh packet build (0x100035b0, 687 bytes; internal,
/// `ecx` = the draw record `R` [`draw_0426`] builds). A work block `B` of
/// 0x39b8 bytes from slot 264 holds two 0x1c00-byte packet buffers
/// (`+0x1a8`, swapped by the count `+0xd0`; `+0xd4` the start, `+0xd8` the
/// end, `+0xdc` the cursor), the header (16 bytes from the immediate
/// `+0x47`, `+0xc` 6), `+0xe0` = `R[8]`, `+0xe4` 6. For each of the
/// model's `n` parts (`R[0x48]`: the byte `+4`, the 16-byte entries from
/// `+0x10`; slot 941 `(entry[0])` gives the part `P`, its vertices at `P +
/// P[8]`, its strips at `P + P[4]`): while a strip's byte `+1` is 0x10, its
/// u16 count `+2` of 0x1c-byte faces (from strip `+0x20`) is walked: a
/// cursor too near its buffer's end (under 0x148 bytes) gets an 8-byte 0
/// end mark and the other buffer; each face's three vertex indices give
/// the points `B + 0x30`, `+0x40`, `+0x50`, its six u16s `+0xa0..+0xc4`,
/// and `B[8]` = the strip's word `+8`. A strip whose next byte `+1` is 0xff
/// ends the part. At the end a cursor past the start gets its end mark;
/// slot 265 `(0x39b8)` gives the block back.
///
/// Returns slot 265's `eax`.
pub fn mesh_packets_0426<G: Game>(g: &mut G, img: &Image, f: u32, rec: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, n: u32| slot_target(g, w(0x1), n);
    // call width: 1
    let b = g.call(slot(g, 264), 0, &[0x39b8]).eax;
    let at = |o: u32| b.wrapping_add(o);
    let x8 = g.read::<u32>(rec + 8);
    let buf = at(0x1a8);
    g.write::<u32>(at(0xd0), 0);
    g.write::<u32>(at(0xd4), buf);
    g.write::<u32>(at(0xdc), buf);
    g.write::<u32>(at(0xd8), buf.wrapping_add(0x1c00));
    let model = g.read::<u32>(rec + 0x48);
    let h0 = g.read::<u64>(w(0x47));
    g.write::<u64>(b, h0);
    let h1 = g.read::<u64>(w(0x56));
    g.write::<u64>(at(8), h1);
    g.write::<u32>(at(0xe0), x8);
    g.write::<u32>(at(0xc), 6);
    g.write::<u32>(at(0xe4), 6);
    g.write::<u32>(at(0x1a4), model.wrapping_add(0x10));
    let parts = g.read::<u8>(model + 4);
    g.write::<u8>(at(0x39ae), parts);
    if parts != 0 {
        'parts: loop {
            let entry = g.read::<u32>(at(0x1a4));
            let e0 = g.read::<u32>(entry);
            // call width: 1
            let p = g.call(slot(g, 941), 0, &[e0]).eax;
            g.write::<u32>(at(0x198), p);
            let verts = p.wrapping_add(g.read::<u32>(p + 8));
            g.write::<u32>(at(0x19c), verts);
            let strips = p.wrapping_add(g.read::<u32>(p + 4));
            g.write::<u32>(at(0x1a0), strips);
            loop {
                let strip = g.read::<u32>(at(0x1a0));
                if g.read::<u8>(strip + 1) != 0x10 {
                    break 'parts;
                }
                let mut face = strip.wrapping_add(0x20);
                let n = g.read::<u16>(strip + 2);
                g.write::<u16>(at(0x39ac), n);
                if n.cast_signed() > 0 {
                    loop {
                        let cur = g.read::<u32>(at(0xdc));
                        let end = g.read::<u32>(at(0xd8));
                        if end.wrapping_sub(cur).wrapping_sub(8) < 0x140 {
                            g.write::<u32>(cur, 0);
                            g.write::<u32>(cur + 4, 0);
                            let k = g.read::<u32>(at(0xd0)).wrapping_add(1);
                            g.write::<u32>(at(0xd0), k);
                            let next = buf.wrapping_add((k & 1) * 0x1c00);
                            g.write::<u32>(at(0xd4), next);
                            g.write::<u32>(at(0xd8), next.wrapping_add(0x1c00));
                            g.write::<u32>(at(0xdc), next);
                        }
                        for (i, to) in [0x30u32, 0x40, 0x50].into_iter().enumerate() {
                            let ix = u32::from(g.read::<u16>(face + 2 * i as u32));
                            let v = g.read::<u32>(at(0x19c)).wrapping_add(ix << 4);
                            g.write::<u32>(at(0x39a8), v);
                            for j in 0..3 {
                                let x = g.read::<u32>(v + 4 * j);
                                g.write::<u32>(at(to + 4 * j), x);
                            }
                        }
                        for (i, to) in [0xa0u32, 0xa4, 0xb0, 0xb4, 0xc0, 0xc4]
                            .into_iter()
                            .enumerate()
                        {
                            let x = u32::from(g.read::<u16>(face + 8 + 2 * i as u32));
                            g.write::<u32>(at(to), x);
                        }
                        face = face.wrapping_add(0x1c);
                        let s = g.read::<u32>(at(0x1a0));
                        let x = g.read::<u32>(s + 8);
                        g.write::<u32>(at(8), x);
                        let n = g.read::<u16>(at(0x39ac)).wrapping_sub(1);
                        g.write::<u16>(at(0x39ac), n);
                        if n.cast_signed() <= 0 {
                            break;
                        }
                    }
                }
                g.write::<u32>(at(0x1a0), face);
                if g.read::<u8>(face + 1) == 0xff {
                    break;
                }
            }
            let left = g.read::<u8>(at(0x39ae)).wrapping_sub(1);
            g.write::<u8>(at(0x39ae), left);
            let e = g.read::<u32>(at(0x1a4)).wrapping_add(0x10);
            g.write::<u32>(at(0x1a4), e);
            if left == 0 {
                break;
            }
        }
    }
    let cur = g.read::<u32>(at(0xdc));
    if cur != g.read::<u32>(at(0xd4)) {
        g.write::<u32>(cur, 0);
        g.write::<u32>(cur + 4, 0);
    }
    // call width: 1
    g.call(slot(g, 265), 0, &[0x39b8]).eax
}

/// [`mesh_packets_0426`].
pub const MESH_PACKETS_0426: Sig = Sig {
    len: 687,
    hash: 0x81c3_fca8_2662_d1fe,
    calls: &[],
    imports: &[],
};

/// `magic_0426`'s draw visitor (0x10003130, 1143 bytes; `ecx` = the cursor
/// `C` = `{n0, flag, pass}`; a 16-aligned /GS frame, the locals from `S` =
/// `(ebp & !0xf) - 0x150` after two pushes): the children walked
/// ([`walk`](crate::nodes::walk), each method `+0x1c` with `ecx` = `n0`).
/// With the flag set, `n0`'s u16 delay `+0xba` counts down; at its end, with
/// the shown bit 1 of `+0xb4` and a model `+0x90`:
/// - bit 2: slots 355 and 354 `(h, -1)` (`h` = `n0[0x88][0x54]`);
/// - four buffers from slot 544 `(i, S + 0x108 + 0x10 i)`, three from slot
///   588 `(i, S + 0xd8 + 0x10 i)` (two words cleared first); the matrix
///   `M` (`S + 0x98`) from slot 327 `(M, n0[0x8c] + 0x9c)`, its three rows
///   through slot 232 `(r, r)` and 233 `(r, r, -1.0)`, slot 229 `(v, M, v)`
///   for the three 588 buffers, slot 231 `(M, G, n0[0x8c] + 0x9c)`;
/// - `s` = slot 277 `(h)`: the draw record `R` (`S + 0x10`; `R + 8`/`+0xc`
///   = `n0[0x80]`/`[0x84]`, the buffers, the colours `A`/`B` (1.0 each),
///   `M`, the model, the sizes `n0[0x94] s 10` and `n0[0x98] s 10` plus
///   it, `*G`) built by [`mesh_packets_0426`];
/// - bit 4: the node leaves its parent ([`leave`](crate::sa_misc::leave)),
///   slots 357 and 356 `(h, -1, -1)`.
///
/// Returns the walk's `eax` (the pass byte low), the delay less one, the
/// flags word, the packets' or slot 356's `eax`.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn draw_0426<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, mul};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, n: u32| slot_target(g, w(0x10a), n);
    let ebp = fr.esp.wrapping_sub(4);
    let e = (ebp & !0xf).wrapping_sub(0x148);
    let s0 = e.wrapping_sub(8);
    let at = |o: u32| s0.wrapping_add(o);
    let c = fr.ecx;
    let n0 = g.read::<u32>(c);
    // `eax` is the cookie word `xor`ed with the frame until the walk
    let cookie = g.read::<u32>(w(0xd)) ^ e;
    let (eax, owner) = crate::nodes::walk(g, c, n0, cookie);
    if owner.is_none() || g.read::<u8>(c + 4) & 1 == 0 {
        return Ret::eax(eax);
    }
    let n0 = g.read::<u32>(c);
    let d = g.read::<u16>(n0 + 0xba);
    g.write::<u16>(n0 + 0xba, d.wrapping_sub(1));
    if d.cast_signed() > 0 {
        return Ret::eax(u32::from(d).wrapping_sub(1));
    }
    let flags = u32::from(g.read::<u16>(n0 + 0xb4));
    if flags & 1 == 0 || g.read::<u32>(n0 + 0x90) == 0 {
        return Ret::eax(flags);
    }
    let one = 0x3f80_0000u32;
    for o in 0..4 {
        g.write::<u32>(at(0x88 + 4 * o), one);
        g.write::<u32>(at(0x78 + 4 * o), one);
    }
    let handle = |g: &G| g.read::<u32>(g.read::<u32>(n0 + 0x88) + 0x54);
    if flags & 2 != 0 {
        for s in [355u32, 354] {
            let h = handle(g);
            // call width: 2
            g.call(slot(g, s), 0, &[h, u32::MAX]);
        }
    }
    for i in 0..4 {
        // call width: 2
        g.call(slot(g, 544), 0, &[i, at(0x108 + 0x10 * i)]);
    }
    for i in 0..3 {
        if i > 0 {
            // the previous buffer's fourth word
            g.write::<u32>(at(0xd4 + 0x10 * i), 0);
        }
        // call width: 2
        g.call(slot(g, 588), 0, &[i, at(0xd8 + 0x10 * i)]);
    }
    let m = at(0x98);
    g.write::<u32>(at(0x104), 0);
    let cam = g.read::<u32>(n0 + 0x8c).wrapping_add(0x9c);
    // call width: 2
    g.call(slot(g, 327), 0, &[m, cam]);
    for r in [m, m + 0x10, m + 0x20] {
        // call width: 2
        g.call(slot(g, 232), 0, &[r, r]);
        // call width: 3
        g.call(slot(g, 233), 0, &[r, r, 0xbf80_0000]);
    }
    for v in [at(0xd8), at(0xe8), at(0xf8)] {
        // call width: 3
        g.call(slot(g, 229), 0, &[v, m, v]);
    }
    let cam = g.read::<u32>(n0 + 0x8c).wrapping_add(0x9c);
    // call width: 3
    g.call(slot(g, 231), 0, &[m, w(0x33c), cam]);
    let h = handle(g);
    // call width: 1
    let sc = st0_f32(g.call(slot(g, 277), 0, &[h]).st0);
    g.write::<f32>(at(0x84), sc);
    let k = g.read::<f32>(w(0x397));
    let y = mul(mul(g.read::<f32>(n0 + 0x98), sc), k);
    let x = mul(mul(g.read::<f32>(n0 + 0x94), sc), g.read::<f32>(w(0x39f)));
    let y = add(y, x);
    // the draw record `R` at `S + 0x10`
    let rec = at(0x10);
    let model = g.read::<u32>(n0 + 0x90);
    g.write::<u32>(at(0x58), model);
    let x80 = g.read::<u32>(n0 + 0x80);
    g.write::<u32>(at(0x18), x80);
    let x84 = g.read::<u32>(n0 + 0x84);
    g.write::<u32>(at(0x1c), x84);
    let gw = g.read::<u32>(w(0x3bf));
    g.write::<u32>(at(0x68), gw);
    g.write::<u32>(at(0x54), m);
    g.write::<u32>(at(0x40), at(0x88));
    g.write::<u32>(at(0x5c), at(0x78));
    g.write::<f32>(at(0x64), x);
    g.write::<f32>(at(0x60), y);
    g.write::<u32>(at(0x20), at(0x108));
    g.write::<u32>(at(0x24), at(0xd8));
    let eax = mesh_packets_0426(g, img, img.rel32_target(f + 0x41f), rec);
    if g.read::<u8>(n0 + 0xb4) & 0x10 == 0 {
        return Ret::eax(eax);
    }
    crate::sa_misc::leave(g, n0, eax);
    let h = handle(g);
    // call width: 3
    g.call(slot(g, 357), 0, &[h, u32::MAX, u32::MAX]);
    // call width: 3
    Ret::eax(g.call(slot(g, 356), 0, &[h, u32::MAX, u32::MAX]).eax)
}

/// [`draw_0426`] (the `rel32`s: the packet build, the node leave,
/// `__security_check_cookie`).
pub const DRAW_0426: Sig = Sig {
    len: 1143,
    hash: 0xcb08_8827_1784_28b1,
    calls: &[
        (0x41f, Callee::Sig(&MESH_PACKETS_0426)),
        (0x42f, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x46f, Callee::Any),
    ],
    imports: &[],
};

/// The thunk of [`draw_0426`] (`magic_0426` 0x100038a0: `ecx` = the
/// stack word, a jump with the caller's stack).
pub fn draw_0426_thunk<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::o6_fam_pairs3::thunk_to(g, img, fr, draw_0426::<G>)
}

/// [`draw_0426_thunk`].
pub const DRAW_0426_THUNK: Sig = Sig {
    len: 12,
    hash: 0x896f_4fde_1224_a07f,
    calls: &[(0x8, Callee::Sig(&DRAW_0426))],
    imports: &[],
};

/// `magic_0442`'s start-up (0x100027f0, 1814 bytes; internal, no
/// arguments, run by its load step `r9_load::load_0442`; an `ebp` /GS
/// frame, `P` at `ebp - 0x14`; `A = *G_A`, `T = TAB[*I]`):
/// - two globals set, `*G_C` = slot 719 `(D, 8, 0)`; the part set up
///   ([`part_init`](crate::l5_objhost::part_init)); `P` = the actors' mean
///   position ([`mean_position`](crate::sb_misc::mean_position));
/// - slot 390 `(*I)`: a kind in the list `L` (-1 ended) puts `P` at the
///   target's point (slots 262 and 268 `(T, 0x11, P)`, `P.w` 1); else
///   `P.y` = slot 12 `(T)` less 40;
/// - the yaw `K1 - slot 333 (A)` wrapped once;
/// - `n = *G_N` times: a scale `(slot 404 (0.2) + 1) * 0.27`, two objects
///   started at `OBJ + 0x260 i` and `+ 0x130`
///   ([`start_object_owners`](crate::s4_misc::start_object_owners): `(0,
///   0x4b i, 0x1e, CB)` and `(1, 0x4b i, 0, 0)`), each given the point
///   (`+0xd0`) and the yaw (`+0xc4`), the record `R_i` (0x34 bytes apart)
///   between them; the point for the next pair `P + (slot 314 (10), -slot
///   404 (10), slot 314 (10))`;
/// - the count word `0x4b (n - 1)` (its sign picks one of two tables), the
///   actor block (slot 413 in a battle, else 215), two colour records of
///   globals;
/// - three nodes off the free ring (inlined takes, the callbacks the
///   immediates): the first two marked and holding the colour records,
///   the third plain; slot 271 `(4)` between the second and the third;
///   each appended to the task list.
///
/// Returns 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn start_up_0442<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x11), n);
    let e = fr.esp;
    let ebp = e.wrapping_sub(4);
    let at = |o: u32| ebp.wrapping_sub(o);
    let p = at(0x14);
    let one = 0x3f80_0000u32;
    g.write::<u32>(w(0x27), w(0x2b));
    g.write::<u32>(w(0x31), w(0x35));
    // call width: 3
    let c = g.call(slot(g, 719), 0, &[w(0x21), 8, 0]).eax;
    g.write::<u32>(w(0x3f), c);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x44));
    crate::sb_misc::mean_position(g, img, img.rel32_target(f + 0x4c), e.wrapping_sub(0x44), p);
    let byte = |g: &G, sel: u32| u32::from(g.read::<u8>(g.read::<u32>(w(sel))));
    let i = byte(g, 0x51);
    // call width: 1
    let kind = g.call(slot(g, 390), 0, &[i]).eax;
    // the -1 ended list of kinds
    let mut listed = false;
    let mut l = w(0x70);
    loop {
        let x = g.read::<u32>(l);
        if x == u32::MAX {
            break;
        }
        if x == kind {
            listed = true;
            break;
        }
        l = l.wrapping_add(4);
    }
    if listed {
        let who = g.read::<u32>(w(0x119).wrapping_add(4 * byte(g, 0x10f)));
        // call width: 4
        g.call(slot(g, 262), 0, &[who, p, p + 4, p + 8]);
        // call width: 3
        g.call(slot(g, 268), 0, &[who, 0x11, p]);
        g.write::<u32>(p + 0xc, one);
    } else {
        let who = g.read::<u32>(w(0x9e).wrapping_add(4 * byte(g, 0x94)));
        // call width: 1
        let h = st0_f32(g.call(slot(g, 12), 0, &[who]).st0);
        g.write::<f32>(at(0x18), h);
        g.write::<f32>(p + 4, sub(h, k(g, 0xb8)));
    }
    let a = g.read::<u32>(w(0xd3));
    g.write::<u32>(at(0x18), 0);
    // call width: 1
    let t = st0_f32(g.call(slot(g, 333), 0, &[a]).st0);
    g.write::<f32>(at(0x20), t);
    let yaw = crate::r20::wrap_once(g, img, f, sub(k(g, 0xea), t), (0xf9, 0x108, 0x15e, 0x16b));
    g.write::<f32>(at(0x1c), yaw);
    // the point for each pair (`ebp - 0x28`, `- 0x2c`, `- 0x24`, `- 0x30`)
    let q = [at(0x28), at(0x2c), at(0x24), at(0x30)];
    for (j, &to) in q.iter().enumerate() {
        let x = g.read::<u32>(p + 4 * j as u32);
        g.write::<u32>(to, x);
    }
    let n = g.read::<i32>(w(0x17a));
    g.write::<i32>(at(0x20), n);
    let mut owner = 0u32;
    if n > 0 {
        let mut obj = w(0xda);
        let mut rec = w(0x1ad);
        let mut left = n;
        loop {
            // call width: 1
            let r = st0_f32(g.call(slot(g, 404), 0, &[0x3e4c_cccd]).st0);
            let scale = mul(add(r, k(g, 0x1d2)), k(g, 0x1e4));
            g.write::<f32>(at(0x18), scale);
            let put = |g: &mut G, obj: u32| {
                for (j, &from) in q.iter().enumerate() {
                    let x = g.read::<u32>(from);
                    g.write::<u32>(obj + 0xd0 + 4 * j as u32, x);
                }
                g.write::<f32>(obj + 0xc4, yaw);
            };
            let s1 = img.rel32_target(f + 0x1f3);
            crate::s4_misc::start_object_owners(g, img, s1, obj, scale, 0, owner, (0x1e, w(0x1d7)));
            put(g, obj);
            for o in [4u32, 0, 0xffff_fffc, 8, 0x14, 0x18] {
                g.write::<u32>(rec.wrapping_add(o), 0);
            }
            g.write::<u16>(rec + 0x1c, 0);
            g.write::<u32>(obj + 0x110, rec.wrapping_sub(0x14));
            g.write::<u32>(rec + 0xc, w(0x279));
            g.write::<u32>(rec + 0x10, w(0x280));
            let obj2 = obj.wrapping_add(0x130);
            let s2 = img.rel32_target(f + 0x293);
            crate::s4_misc::start_object_owners(g, img, s2, obj2, scale, 1, owner, (0, 0));
            put(g, obj2);
            obj = obj2.wrapping_add(0x130);
            // the next pair's point: `P` plus three random offsets
            for (j, (s, neg)) in [(314u32, false), (404, true), (314, false)]
                .into_iter()
                .enumerate()
            {
                // call width: 1
                let r = st0_f32(g.call(slot(g, s), 0, &[0x4120_0000]).st0);
                g.write::<f32>(at(0x18), r);
                let base = g.read::<f32>(p + 4 * j as u32);
                let x = if neg { sub(base, r) } else { add(base, r) };
                g.write::<f32>(q[j], x);
            }
            owner = owner.wrapping_add(0x4b);
            rec = rec.wrapping_add(0x34);
            left -= 1;
            if left == 0 {
                break;
            }
        }
    }
    // the count word and the actor block
    g.write::<u32>(at(0x18), owner);
    let last = owner.wrapping_sub(0x4b);
    g.write::<u32>(w(0x36e), last);
    let j = g.read::<u32>(w(0x373));
    let tab = if last.cast_signed() > 0 {
        w(0x378)
    } else {
        w(0x37d)
    };
    let battle = g.read::<u32>(w(0x386)) != 0;
    g.write::<u32>(w(0x38c), j);
    g.write::<u32>(w(0x398), tab);
    g.write::<u32>(w(0x39e), u32::MAX);
    // call width: 1
    let h = g.call(slot(g, if battle { 413 } else { 215 }), 0, &[j]).eax;
    let battle = g.read::<u32>(w(0x3bd)) != 0;
    g.write::<u32>(w(0x3c8), h);
    g.write::<u32>(w(0x3ce), u32::MAX);
    g.write::<u32>(w(0x3d8), 0);
    g.write::<u32>(w(0x3e2), 1);
    if battle {
        g.write::<u16>(w(0x3f5), 1);
    } else {
        g.write::<u16>(w(0x3ec), 0);
    }
    // the colour records
    let mut free = g.read::<u16>(w(0x3fc));
    for (o, v) in [
        (0x402u32, one),
        (0x40c, one),
        (0x416, one),
        (0x420, one),
        (0x42a, 0x3fc0_0000),
        (0x434, 0x3fc0_0000),
        (0x43e, 0x3fc0_0000),
        (0x448, one),
        (0x452, 0),
        (0x45c, 0),
        (0x466, 8),
        (0x470, 8),
    ] {
        g.write::<u32>(w(o), v);
    }
    // an inlined take off the free ring (`head` the operand holding the
    // ring's head, `heads` where the new head goes: the next node or 0)
    let take =
        |g: &mut G, free: &mut u16, head: u32, (to_next, to_0): (u32, u32), cb: u32, count: u32| {
            if *free == 0 {
                return 0;
            }
            let node = g.read::<u32>(w(head));
            let prev = g.read::<u32>(node);
            let next = g.read::<u32>(node + 4);
            g.write::<u32>(prev + 4, next);
            let next = g.read::<u32>(node + 4);
            let prev = g.read::<u32>(node);
            g.write::<u32>(next, prev);
            if *free > 1 {
                let next = g.read::<u32>(node + 4);
                g.write::<u32>(w(to_next), next);
            } else {
                g.write::<u32>(w(to_0), 0);
            }
            *free -= 1;
            for o in [0u32, 4, 0xc, 0x14] {
                g.write::<u32>(node + o, 0);
            }
            g.write::<u32>(node + 8, node);
            g.write::<u32>(node + 0x10, w(cb));
            g.write::<u8>(node + 0x18, 0);
            g.write::<u16>(w(count), *free);
            node
        };
    let n1 = take(g, &mut free, 0x48e, (0x4b3, 0x4b3), 0x4d8, 0x4e3);
    let first = g.read::<u32>(w(0x4e9));
    g.write::<u8>(n1 + 0x18, 1);
    g.write::<u32>(n1 + 0x14, w(0x4f4));
    g.write::<u32>(n1 + 4, 0);
    let first = if first == 0 {
        g.write::<u32>(n1, 0);
        g.write::<u32>(w(0x519), n1);
        n1
    } else {
        let last = g.read::<u32>(w(0x504));
        g.write::<u32>(n1, last);
        g.write::<u32>(last + 4, n1);
        first
    };
    g.write::<u32>(n1 + 8, n1);
    for (o, v) in [
        (0x528u32, 0x3fc0_0000u32),
        (0x532, one),
        (0x53c, 0x3fc0_0000),
        (0x546, 0x3fc0_0000),
        (0x550, 0x3fc0_0000),
        (0x55a, one),
        (0x564, one),
        (0x56e, one),
        (0x578, one),
        (0x582, one),
        (0x58b, owner.wrapping_add(5)),
        (0x591, 0),
        (0x59b, 0x10),
        (0x5a5, 0x10),
    ] {
        g.write::<u32>(w(o), v);
    }
    // (the second take starts from the head the first one left: the
    // original keeps it in `edx`)
    let n2 = take(g, &mut free, 0x4b3, (0x5cf, 0x5d7), 0x608, 0x613);
    g.write::<u8>(n2 + 0x18, 1);
    g.write::<u32>(n2 + 0x14, w(0x621));
    g.write::<u32>(n2 + 4, 0);
    if first == 0 {
        g.write::<u32>(n2, 0);
        g.write::<u32>(w(0x63e), n2);
    } else {
        g.write::<u32>(n2, n1);
        g.write::<u32>(n1 + 4, n2);
    }
    g.write::<u32>(n2 + 8, n2);
    g.write::<u32>(w(0x654), n2);
    // call width: 1
    g.call(slot(g, 271), 0, &[4]);
    let mut free = g.read::<u16>(w(0x65d));
    let n3 = take(g, &mut free, 0x66f, (0x68c, 0x694), 0x6c5, 0x6d0);
    let listed = g.read::<u32>(w(0x6d6)) != 0;
    g.write::<u32>(n3 + 4, 0);
    if listed {
        let last = g.read::<u32>(w(0x6e6));
        g.write::<u32>(n3, last);
        g.write::<u32>(last + 4, n3);
    } else {
        g.write::<u32>(n3, 0);
        g.write::<u32>(w(0x6f9), n3);
    }
    g.write::<u32>(n3 + 8, n3);
    g.write::<u32>(w(0x707), n3);
    Ret::eax(0)
}

/// [`start_up_0442`] (the `rel32`s: the part set-up, the mean position,
/// the two object starts, `__security_check_cookie`).
pub const START_UP_0442: Sig = Sig {
    len: 1814,
    hash: 0x766e_c641_a0e9_c2d9,
    calls: &[
        (0x44, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x4c, Callee::Sig(&crate::sb_misc::MEAN_POSITION)),
        (0x1f3, Callee::Sig(&crate::s4_misc::START_OBJECT_OWNERS)),
        (0x293, Callee::Sig(&crate::s4_misc::START_OBJECT_OWNERS)),
        (0x70e, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0260`'s start-up (0x10003d40, 1977 bytes; internal, no
/// arguments, run by its load step `r9_load::load_0260`; an 8-aligned /GS
/// frame, the locals from `S` = `(ebp & !7) - 0xd0` (after three pushes);
/// `A = *G_A` the actor, `T = TAB[*I]` the target):
/// - three globals set, `*G_C` = slot 719 `(D, 8, 0)`; the part set up
///   ([`part_init`](crate::l5_objhost::part_init)); object 1 started
///   ([`start_object_at`](crate::s4_misc::start_object_at), scale `K1`,
///   `(1, 0x12, -, 0)`; the third word is a callee-left `ecx` the start
///   never reads);
/// - `P` (three globals) from slot 262 `(A, &P)`, `P.w` 1; the yaw `K2 -
///   slot 333 (A)` wrapped once; `P.y -= K3`, a u16 and nine globals;
/// - `R` (`S + 0x28`) = slot 262 `(T, ..)`, `R.y` = `V.y` from slot 268
///   `(T, 6, V)` (`V` at `S + 0x38`), `R.w` 1; the turn `K2 - atan2(R.z -
///   P.z, R.x - P.x)` wrapped; the matrix `M` (`S + 0x48`: slots 235, 328
///   `(M, M, 0)`, 329 `(M, M, turn)`) moves `X = (0, 0, -12, 1)` (`S +
///   0x18`) into `V` (slot 229), `P += V` (slot 248), `P.w` 1;
/// - the turn back `K2 - atan2(P.z - R.z, P.x - R.x)` wrapped, the matrix
///   `M2` (`S + 0x88`: slot 235, 329 `(M2, M2, turn2)`);
/// - five shards `j` (0x3a0 bytes apart from `B`): object 2 started at
///   `O = B - 0x138` (scale `K4`, owner `TAB2[j] + 0x14`), slot 325 `(Q,
///   P)` with `Q = B - 0x68`; `Y` = `M2 (slot 314 (1), slot 314 (2.2),
///   -2, 1)` (slot 229), `Q += Y` (slot 248, `Y.w` 0); slot 325 `(B +
///   0x220, R)`, plus `Y`; the shard's words, its part (slot 941
///   `(**G_P)`), its 32 copies of `Q`, colours and sizes; the aim at `O`
///   ([`aim_object`](crate::s5_misc::aim_object), `xmm1` = 8);
/// - the actor block (slot 413 in a battle, else 215);
/// - two nodes off the free ring (inlined takes, the callbacks the
///   immediates, bytes `+0x18` 1 and 2) appended to the task list.
///
/// Returns 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn start_up_0260<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::sub;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x1b), n);
    let ebp = fr.esp.wrapping_sub(4);
    // `S`: the frame after `push ebx; push esi; push edi`
    let s0 = (ebp & !7).wrapping_sub(0xd0);
    let at = |o: u32| s0.wrapping_add(o);
    let one = 0x3f80_0000u32;
    for (o, v) in [(0x33u32, 0x37u32), (0x3d, 0x41), (0x47, 0x4b)] {
        g.write::<u32>(w(o), w(v));
    }
    // call width: 3
    let c = g.call(slot(g, 719), 0, &[w(0x2d), 8, 0]).eax;
    g.write::<u32>(w(0x55), c);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x5a));
    // (the third stack word, a callee-left `ecx`, is never read)
    let start = img.rel32_target(f + 0x76);
    crate::s4_misc::start_object_at(g, img, start, w(0x6a), k(g, 0x62), 1, 0x12, 0);
    let a = g.read::<u32>(w(0x96));
    // call width: 4
    g.call(slot(g, 262), 0, &[a, w(0x90), w(0x8b), w(0x80)]);
    let a = g.read::<u32>(w(0xa3));
    g.write::<u32>(w(0xa9), one);
    // call width: 1
    let t = st0_f32(g.call(slot(g, 333), 0, &[a]).st0);
    g.write::<f32>(at(0x10), t);
    let yaw = crate::r20::wrap_once(g, img, f, sub(k(g, 0xc1), t), (0xd9, 0xcf, 0xe9, 0xcf));
    g.write::<f32>(w(0xfa), yaw);
    let py = sub(g.read::<f32>(w(0x102)), k(g, 0x10a));
    g.write::<u16>(w(0x112), 0);
    let sel = g.read::<u32>(w(0x117));
    g.write::<u32>(w(0x11d), 0);
    g.write::<f32>(w(0x129), py);
    for o in [0x12fu32, 0x139, 0x143] {
        g.write::<u32>(w(o), 0);
    }
    g.write::<u32>(w(0x14d), w(0x151));
    g.write::<u32>(w(0x157), w(0x15b));
    for o in [0x161u32, 0x16b] {
        g.write::<u32>(w(o), 0);
    }
    g.write::<u32>(w(0x175), w(0x179));
    // the target's point `R` and its height `V`
    let i = u32::from(g.read::<u8>(sel));
    let who = g.read::<u32>(w(0x183).wrapping_add(4 * i));
    let (r, v, x, m, m2) = (at(0x28), at(0x38), at(0x18), at(0x48), at(0x88));
    // call width: 4
    g.call(slot(g, 262), 0, &[who, r, r + 4, r + 8]);
    // call width: 3
    g.call(slot(g, 268), 0, &[who, 6, v]);
    let vy = g.read::<u32>(v + 4);
    g.write::<u32>(r + 4, vy);
    let rd = |g: &G, a: u32| g.read::<f32>(a);
    let dz = sub(rd(g, r + 8), rd(g, w(0x1cf)));
    g.write::<u32>(r + 0xc, one);
    let dx = sub(rd(g, r), rd(g, w(0x1f5)));
    // `_CIatan2(dz, dx)` on x87, the result rounded once (`cvtpd2ps`)
    #[allow(clippy::cast_possible_truncation)]
    let ang = g.atan2(f64::from(dz), f64::from(dx)) as f32;
    let turn = crate::r20::wrap_once(
        g,
        img,
        f,
        sub(k(g, 0x219), ang),
        (0x228, 0x238, 0x242, 0x24f),
    );
    g.write::<f32>(at(0x10), turn);
    // `X` = (0, 0, -12), its `w` 1.0 after the turns
    for (j, b) in (0u32..).zip([0u32, 0, 0xc140_0000]) {
        g.write::<u32>(x + 4 * j, b);
    }
    // call width: 1
    g.call(slot(g, 235), 0, &[m]);
    // call width: 3
    g.call(slot(g, 328), 0, &[m, m, 0]);
    // call width: 3
    g.call(slot(g, 329), 0, &[m, m, turn.to_bits()]);
    g.write::<u32>(x + 0xc, one);
    // call width: 3
    g.call(slot(g, 229), 0, &[v, m, x]);
    // call width: 3
    g.call(slot(g, 248), 0, &[w(0x2fe), w(0x2f3), v]);
    // the turn back from `R`
    let dz = sub(rd(g, w(0x308)), rd(g, r + 8));
    g.write::<u32>(w(0x317), one);
    let dx = sub(rd(g, w(0x32c)), rd(g, r));
    #[allow(clippy::cast_possible_truncation)]
    let ang = g.atan2(f64::from(dz), f64::from(dx)) as f32;
    let turn2 = crate::r20::wrap_once(
        g,
        img,
        f,
        sub(k(g, 0x35a), ang),
        (0x369, 0x379, 0x383, 0x390),
    );
    g.write::<f32>(at(0x10), turn2);
    // call width: 1
    g.call(slot(g, 235), 0, &[m2]);
    // call width: 3
    g.call(slot(g, 329), 0, &[m2, m2, turn2.to_bits()]);
    // the five shards
    let mut b = w(0x3da);
    for j in 0..5u32 {
        g.write::<u32>(at(0x10), j);
        let owner = g
            .read::<u32>(w(0x3e3).wrapping_add(4 * j))
            .wrapping_add(0x14);
        let obj = b.wrapping_sub(0x138);
        let s1 = img.rel32_target(f + 0x3ff);
        crate::s4_misc::start_object_at(g, img, s1, obj, k(g, 0x3eb), 2, owner, 0);
        let p0 = b.wrapping_sub(0x68);
        // call width: 2
        g.call(slot(g, 325), 0, &[p0, w(0x412)]);
        let y = at(0x18);
        // call width: 1
        let r1 = st0_f32(g.call(slot(g, 314), 0, &[one]).st0);
        g.write::<f32>(y, r1);
        // call width: 1
        let r2 = st0_f32(g.call(slot(g, 314), 0, &[0x400c_cccd]).st0);
        g.write::<f32>(y + 4, r2);
        g.write::<u32>(y + 8, 0xc000_0000);
        g.write::<u32>(y + 0xc, one);
        // call width: 3
        g.call(slot(g, 229), 0, &[y, m2, y]);
        g.write::<u32>(y + 0xc, 0);
        // call width: 3
        g.call(slot(g, 248), 0, &[p0, p0, y]);
        let p1 = b.wrapping_add(0x220);
        // call width: 2
        g.call(slot(g, 325), 0, &[p1, r]);
        // call width: 3
        g.call(slot(g, 248), 0, &[p1, p1, y]);
        g.write::<u32>(b + 0x254, owner);
        g.write::<u16>(b + 0x25c, 1);
        g.write::<u32>(b + 0x240, w(0x4ee));
        g.write::<u32>(b + 0x244, 0);
        g.write::<u32>(b + 0x248, w(0x502));
        g.write::<u32>(b + 0x258, j);
        g.write::<u32>(b + 0x250, 0x3cf5_c28f);
        let part = g.read::<u32>(g.read::<u32>(w(0x4e4)));
        // call width: 1
        let pt = g.call(slot(g, 941), 0, &[part]).eax;
        g.write::<u32>(b.wrapping_sub(8), pt);
        g.write::<u32>(b.wrapping_sub(4), 1);
        g.write::<u16>(b, 0xffff);
        g.write::<u16>(b + 2, 0x200);
        g.write::<u32>(b + 0x204, 0x8080_8080);
        g.write::<u32>(b + 0x208, 0);
        g.write::<u32>(b + 0x218, 0);
        g.write::<u32>(b + 0x20c, one);
        g.write::<u32>(b + 0x210, one);
        for n in 0..0x20u32 {
            for q in 0..4 {
                let x = g.read::<u32>(p0 + 4 * q);
                g.write::<u32>(b + 4 + 0x10 * n + 4 * q, x);
            }
        }
        g.write::<u32>(b + 0x21c, 0x40);
        g.write::<u32>(b + 0x214, 0x3f00_0000);
        g.write::<u16>(b + 0x204, 0x4020);
        g.write::<u8>(b + 0x206, 0xc0);
        let aim = img.rel32_target(f + 0x5d0);
        crate::s5_misc::aim_object(g, img, aim, s0.wrapping_sub(4), obj, k(g, 0x5a1));
        b = b.wrapping_add(0x3a0);
    }
    g.write::<u32>(at(0x10), 5);
    // the actor block
    let jw = g.read::<u32>(w(0x5e9));
    g.write::<u32>(w(0x5ee), jw);
    g.write::<u32>(w(0x5fa), w(0x5fe));
    g.write::<u32>(w(0x604), u32::MAX);
    let battle = g.read::<u32>(w(0x60e)) != 0;
    // call width: 1
    let h = g
        .call(slot(g, if battle { 413 } else { 215 }), 0, &[jw])
        .eax;
    let battle = g.read::<u32>(w(0x629)) != 0;
    g.write::<u32>(w(0x634), h);
    g.write::<u32>(w(0x63a), u32::MAX);
    g.write::<u32>(w(0x644), 0);
    g.write::<u32>(w(0x64e), 1);
    if battle {
        g.write::<u16>(w(0x661), 1);
    } else {
        g.write::<u16>(w(0x658), 0);
    }
    // two inlined takes off the free ring
    let mut free = g.read::<u16>(w(0x668));
    let take =
        |g: &mut G, free: &mut u16, head: u32, (to_next, to_0): (u32, u32), cb: u32, count: u32| {
            if *free == 0 {
                return 0;
            }
            let node = g.read::<u32>(w(head));
            let prev = g.read::<u32>(node);
            let next = g.read::<u32>(node + 4);
            g.write::<u32>(prev + 4, next);
            let next = g.read::<u32>(node + 4);
            let prev = g.read::<u32>(node);
            g.write::<u32>(next, prev);
            if *free > 1 {
                let next = g.read::<u32>(node + 4);
                g.write::<u32>(w(to_next), next);
            } else {
                g.write::<u32>(w(to_0), 0);
            }
            *free -= 1;
            for o in [0u32, 4, 0xc, 0x14] {
                g.write::<u32>(node + o, 0);
            }
            g.write::<u32>(node + 8, node);
            g.write::<u32>(node + 0x10, w(cb));
            g.write::<u8>(node + 0x18, 0);
            g.write::<u16>(w(count), *free);
            node
        };
    let n1 = take(g, &mut free, 0x682, (0x6a7, 0x6a7), 0x6cc, 0x6d7);
    let first = g.read::<u32>(w(0x6dd));
    g.write::<u8>(n1 + 0x18, 1);
    g.write::<u32>(n1 + 4, 0);
    let first = if first == 0 {
        g.write::<u32>(n1, 0);
        g.write::<u32>(w(0x706), n1);
        n1
    } else {
        let last = g.read::<u32>(w(0x6f1));
        g.write::<u32>(n1, last);
        g.write::<u32>(last + 4, n1);
        first
    };
    g.write::<u32>(n1 + 8, n1);
    // (the second take starts from the head the first one left: the
    // original keeps it in `edx`)
    let n2 = take(g, &mut free, 0x6a7, (0x72f, 0x737), 0x768, 0x773);
    g.write::<u8>(n2 + 0x18, 2);
    g.write::<u32>(n2 + 4, 0);
    if first == 0 {
        g.write::<u32>(n2, 0);
        g.write::<u32>(w(0x795), n2);
    } else {
        g.write::<u32>(n2, n1);
        g.write::<u32>(n1 + 4, n2);
    }
    g.write::<u32>(n2 + 8, n2);
    g.write::<u32>(w(0x7aa), n2);
    Ret::eax(0)
}

/// [`start_up_0260`] (the `rel32`s: the part set-up, the object start,
/// `_CIatan2` twice, the shards' object start, the aim,
/// `__security_check_cookie`).
pub const START_UP_0260: Sig = Sig {
    len: 1977,
    hash: 0xcbf7_ae28_1d25_9b27,
    calls: &[
        (0x5a, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x76, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x207, Callee::Import("_CIatan2")),
        (0x348, Callee::Import("_CIatan2")),
        (0x3ff, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x5d0, Callee::Sig(&crate::s5_misc::AIM_OBJECT)),
        (0x7b1, Callee::Any),
    ],
    imports: &[],
};

/// `cvttss2si`: toward 0, 0x80000000 for a NaN or a value out of range.
fn cvtt(x: f32) -> i32 {
    if x.is_nan() || !(-2_147_483_648.0..2_147_483_648.0).contains(&x) {
        i32::MIN
    } else {
        #[allow(clippy::cast_possible_truncation)] // in range: toward 0
        let n = x as i32;
        n
    }
}

/// `magic_0305`'s start-up (0x100013e0, 2001 bytes; internal, no
/// arguments, run by its load step `r9_load::load_0305`; an 8-aligned /GS
/// frame, the locals from `S` = `(ebp & !7) - 0xa8` (after two pushes);
/// `A = *G_A` the actor, `T = TAB[*I]` the target):
/// - three globals set, `*G_C` = slot 719 `(D, 8, 0)`; the part set up
///   ([`part_init`](crate::l5_objhost::part_init));
/// - `h` = slot 12 `(T)` less 10; `Pa` (`S + 0x30`) = slot 262 `(A, ..)`,
///   `Pt` (`S + 0x20`) = slot 262 `(T, ..)` with slot 268 `(T, 0x10, Pt)`;
///   `Pt.y` less 2, no higher than `h`; `d = Pt - Pa`: the pitch
///   `atan2(d.y, sqrt(d.z^2 + d.x^2))` and the yaw `K1 - atan2(d.z, d.x)`
///   (`_CIatan2`, the root in double), the yaw wrapped once;
/// - the matrix `M` (`S + 0x60`: slots 235, 328 `(M, M, pitch)`, 329 `(M,
///   M, yaw)`) moves `X = (0, -13, -12, 1)` (`S + 0x50`) into `V` (slot
///   229), `Pa += V` (slot 248);
/// - the objects ([`start_object_at`](crate::s4_misc::start_object_at);
///   their third stack word a callee-left `ecx` the start never reads):
///   object 2 `(2, 0)`; slot 262 `(A, ..)` into a record; object 4 `(4,
///   0x18)` with the yaw, a callback and `Pa`; the aim task's record (`Pt`,
///   the yaw, nine globals, the count 0x2d) and the task
///   ([`aim_task`](crate::s5_misc::aim_task), `xmm1` = 10); object 3 `(3,
///   0x2d)`, slot 262 `(A, ..)` into its record, then `Pa` and the yaw;
///   `n = trunc(|Pa - Pt| / -10)`: object 1 `(1, 0x2c - n, -, CB)` at `Pt`,
///   object 0 `(0, 0x2e - n)` at `Pt` with `yaw + pi` wrapped, -6.0 each;
/// - two fades ([`fade_task`](crate::s3_misc::fade_task): `(0x1e, 1.0,
///   0.5, 0)` and `(0x10, 0.5, 1.0, 0x38 - n)`), slot 271 `(4)`, two
///   globals, the actor block ([`me_block`](crate::sa_misc::me_block));
/// - two task nodes ([`take`](crate::sa_pool::take), bytes `+0x18` 1 and
///   2) appended to the task list, the pool reset
///   ([`pool_reset_take`](crate::o6_fam_pool::pool_reset_take)) between.
///
/// Returns 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn start_up_0305<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x1b), n);
    let ebp = fr.esp.wrapping_sub(4);
    // `S`: the frame after `push esi; push edi`
    let s0 = (ebp & !7).wrapping_sub(0xa8);
    let at = |o: u32| s0.wrapping_add(o);
    let rd = |g: &G, a: u32| g.read::<f32>(a);
    let one = 0x3f80_0000u32;
    for (o, v) in [(0x32u32, 0x36u32), (0x3c, 0x40), (0x46, 0x4a)] {
        g.write::<u32>(w(o), w(v));
    }
    // call width: 3
    let c = g.call(slot(g, 719), 0, &[w(0x2c), 8, 0]).eax;
    g.write::<u32>(w(0x54), c);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x59));
    let target = |g: &G, sel: u32, tab: u32| {
        let i = u32::from(g.read::<u8>(g.read::<u32>(w(sel))));
        g.read::<u32>(w(tab).wrapping_add(4 * i))
    };
    let t12 = target(g, 0x5e, 0x68);
    // call width: 1
    let h = st0_f32(g.call(slot(g, 12), 0, &[t12]).st0);
    let h = sub(h, k(g, 0x84));
    g.write::<f32>(at(0xc), h);
    let (pa, pt) = (at(0x30), at(0x20));
    let a = g.read::<u32>(w(0x9e));
    // call width: 4
    g.call(slot(g, 262), 0, &[a, pa, pa + 4, pa + 8]);
    g.write::<u32>(pa + 0xc, one);
    let who = target(g, 0xb1, 0xc3);
    // call width: 4
    g.call(slot(g, 262), 0, &[who, pt, pt + 4, pt + 8]);
    // call width: 3
    g.call(slot(g, 268), 0, &[who, 0x10, pt]);
    let mut y = sub(rd(g, pt + 4), k(g, 0x103));
    g.write::<u32>(pt + 0xc, one);
    if y > h {
        y = h;
    }
    g.write::<f32>(pt + 4, y);
    let dy = sub(y, rd(g, pa + 4));
    let dz = sub(rd(g, pt + 8), rd(g, pa + 8));
    let dx = sub(rd(g, pt), rd(g, pa));
    g.write::<f32>(at(0x1c), dx);
    g.write::<f32>(at(0xc), dz);
    let s = add(mul(dz, dz), mul(dx, dx));
    let hd = f64::from(s).sqrt();
    let pitch = g.atan2(f64::from(dy), hd);
    // `_CIatan2(dz, dx)` on x87, the result rounded once (`cvtpd2ps`)
    #[allow(clippy::cast_possible_truncation)]
    let ang = g.atan2(f64::from(dz), f64::from(dx)) as f32;
    let yaw = crate::r20::wrap_once(
        g,
        img,
        f,
        sub(k(g, 0x1c9), ang),
        (0x1e0, 0x1d9, 0x1f6, 0x1d9),
    );
    g.write::<f32>(at(0xc), yaw);
    // the offset `X` = (0, -13, -12), its `w` 1.0 after the turns
    let (m, x, v) = (at(0x60), at(0x50), at(0x40));
    for (j, b) in (0u32..).zip([0u32, 0xc150_0000, 0xc140_0000]) {
        g.write::<u32>(x + 4 * j, b);
    }
    // call width: 1
    g.call(slot(g, 235), 0, &[m]);
    #[allow(clippy::cast_possible_truncation)]
    let pitch = (pitch as f32).to_bits();
    // call width: 3
    g.call(slot(g, 328), 0, &[m, m, pitch]);
    // call width: 3
    g.call(slot(g, 329), 0, &[m, m, yaw.to_bits()]);
    g.write::<u32>(x + 0xc, one);
    // call width: 3
    g.call(slot(g, 229), 0, &[v, m, x]);
    // call width: 3
    g.call(slot(g, 248), 0, &[pa, pa, v]);
    // the objects (`start_object_at`'s third word is never read)
    let start = |g: &mut G, call: u32, obj: u32, scale: u32, (index, owner, b): (u32, u32, u32)| {
        let at = img.rel32_target(f + call);
        crate::s4_misc::start_object_at(g, img, at, w(obj), k(g, scale), index, owner, b);
    };
    g.write::<u32>(pa + 0xc, one);
    start(g, 0x2d2, 0x2be, 0x2b6, (2, 0, 0));
    let a = g.read::<u32>(w(0x2f2));
    // call width: 4
    g.call(slot(g, 262), 0, &[a, w(0x2ec), w(0x2e7), w(0x2dc)]);
    let copy = |g: &mut G, from: u32, ops: [u32; 4]| {
        for (j, o) in (0u32..).zip(ops) {
            let x = g.read::<u32>(from + 4 * j);
            g.write::<u32>(w(o), x);
        }
    };
    g.write::<u32>(w(0x317), one);
    g.write::<f32>(w(0x323), yaw);
    g.write::<u32>(w(0x329), w(0x32d));
    start(g, 0x332, 0x30a, 0x302, (4, 0x18, 0));
    copy(g, pa, [0x340, 0x34e, 0x35c, 0x36d]);
    // the aim task's record
    g.write::<f32>(w(0x383), yaw);
    copy(g, pt, [0x391, 0x39f, 0x3af, 0x41d]);
    for o in [0x3bbu32, 0x3c5, 0x3cf, 0x3d9] {
        g.write::<u32>(w(o), 0);
    }
    g.write::<u32>(w(0x3e3), w(0x3e7));
    g.write::<u32>(w(0x3ed), w(0x3f1));
    for o in [0x3f7u32, 0x401] {
        g.write::<u32>(w(o), 0);
    }
    g.write::<u16>(w(0x40b), 0);
    g.write::<u32>(w(0x411), w(0x415));
    g.write::<u32>(w(0x423), w(0x427));
    g.write::<u32>(w(0x42d), w(0x431));
    g.write::<u32>(w(0x436), 0);
    g.write::<u32>(w(0x43c), 0x2d);
    g.write::<u16>(w(0x446), 0);
    let aim = img.rel32_target(f + 0x44b);
    crate::s5_misc::aim_task(g, img, aim, s0.wrapping_sub(4), k(g, 0x37b));
    start(g, 0x464, 0x45f, 0x453, (3, 0x2d, 0));
    let a = g.read::<u32>(w(0x484));
    // call width: 4
    g.call(slot(g, 262), 0, &[a, w(0x47e), w(0x479), w(0x46e)]);
    copy(g, pa, [0x499, 0x4a7, 0x4b5, 0x4c3]);
    g.write::<f32>(w(0x4d1), yaw);
    // `n` from the distance `|Pa - Pt|` (`subps`, the root in double)
    let d = |j: u32| sub(rd(g, pa + 4 * j), rd(g, pt + 4 * j));
    let (ex, ey, ez) = (d(0), d(1), d(2));
    let l2 = add(add(mul(ey, ey), mul(ex, ex)), mul(ez, ez));
    #[allow(clippy::cast_possible_truncation)]
    let l = f64::from(l2).sqrt() as f32;
    let n = cvtt(div(l, k(g, 0x51e)));
    let esi = (-1i32).wrapping_sub(n).cast_unsigned();
    let owner1 = esi.wrapping_add(0x2d);
    start(g, 0x53d, 0x538, 0x516, (1, owner1, w(0x52a)));
    copy(g, pt, [0x54b, 0x559, 0x567, 0x575]);
    let yaw2 = add(yaw, k(g, 0x583));
    g.write::<u32>(w(0x58c), 0xc0c0_0000);
    g.write::<f32>(at(0xc), yaw2);
    let y1 = crate::r20::wrap_once(g, img, f, yaw2, (0x597, 0x5aa, 0x5b4, 0x5c4));
    g.write::<u16>(w(0x5d5), 0);
    g.write::<f32>(w(0x5e9), y1);
    for o in [0x5efu32, 0x5f9, 0x603, 0x60d] {
        g.write::<u32>(w(o), 0);
    }
    g.write::<u32>(w(0x617), w(0x61b));
    g.write::<u32>(w(0x621), w(0x625));
    for o in [0x62bu32, 0x635] {
        g.write::<u32>(w(o), 0);
    }
    g.write::<u32>(w(0x63f), w(0x643));
    start(g, 0x648, 0x5e1, 0x5cc, (0, esi.wrapping_add(0x2f), 0));
    copy(g, pt, [0x656, 0x664, 0x672, 0x680]);
    g.write::<u32>(w(0x696), 0xc0c0_0000);
    let y2 = crate::r20::wrap_once(g, img, f, yaw2, (0x690, 0x6a4, 0x6ae, 0x6bb));
    g.write::<f32>(w(0x6e2), y2);
    // the two fades: (`rel32`, `ecx`, `edx`, `xmm1`, `xmm2`, the stack word)
    let fades = [
        (0x6e7u32, w(0x6da), 0x1e, k(g, 0x6c3), k(g, 0x6cb), 0u32),
        (
            0x705,
            w(0x700),
            0x10,
            k(g, 0x6ef),
            k(g, 0x6c3),
            esi.wrapping_add(0x39),
        ),
    ];
    for (i, (call, ecx, edx, x1, x2, word)) in (0u32..).zip(fades) {
        let sp = s0.wrapping_sub(4 + 4 * i);
        g.write::<u32>(sp, word);
        let fade = Frame {
            at: img.rel32_target(f + call),
            esp: sp.wrapping_sub(4),
            eax: 0,
            ecx,
            edx,
            xmm: [0, crate::Xmm::f32(x1).reg(), crate::Xmm::f32(x2).reg(), 0],
        };
        crate::s3_misc::fade_task(g, img, &fade);
    }
    // call width: 1
    g.call(slot(g, 271), 0, &[4]);
    g.write::<u32>(w(0x71f), 0x18);
    g.write::<u32>(w(0x729), owner1);
    crate::sa_misc::me_block(g, img, img.rel32_target(f + 0x72e), w(0x719));
    // the two task nodes
    let append = |g: &mut G, node: u32, byte: u8, [first, last, set_first, set_last]: [u32; 4]| {
        let listed = g.read::<u32>(w(first)) != 0;
        g.write::<u8>(node + 0x18, byte);
        g.write::<u32>(node + 4, 0);
        if listed {
            let tail = g.read::<u32>(w(last));
            g.write::<u32>(node, tail);
            g.write::<u32>(tail + 4, node);
        } else {
            g.write::<u32>(node, 0);
            g.write::<u32>(w(set_first), node);
        }
        g.write::<u32>(node + 8, node);
        g.write::<u32>(w(set_last), node);
    };
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x73b), w(0x736));
    append(g, node, 1, [0x741, 0x755, 0x767, 0x774]);
    crate::o6_fam_pool::pool_reset_take(g, img, img.rel32_target(f + 0x779), w(0x76c));
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x783), w(0x77e));
    append(g, node, 2, [0x789, 0x79d, 0x7af, 0x7c0]);
    Ret::eax(0)
}

/// [`start_up_0305`] (the `rel32`s: the part set-up, the root and
/// `_CIatan2` (twice), the six object starts and the aim task, the root
/// again, the two fades, the actor block, the pool takes and the reset,
/// `__security_check_cookie`).
pub const START_UP_0305: Sig = Sig {
    len: 2001,
    hash: 0x762d_eaeb_ea36_1999,
    calls: &[
        (0x59, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x175, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x188, Callee::Import("_CIatan2")),
        (0x1b7, Callee::Import("_CIatan2")),
        (0x2d2, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x332, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x44b, Callee::Sig(&crate::s5_misc::AIM_TASK)),
        (0x464, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x50a, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x53d, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x648, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x6e7, Callee::Sig(&crate::s3_misc::FADE_TASK)),
        (0x705, Callee::Sig(&crate::s3_misc::FADE_TASK)),
        (0x72e, Callee::Sig(&crate::sa_misc::ME_BLOCK)),
        (0x73b, Callee::Sig(&crate::sa_pool::TAKE)),
        (
            0x779,
            Callee::Sig(&crate::o6_fam_pool::POOL_RESET_TAKE_0205_2A60),
        ),
        (0x783, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x7c9, Callee::Any),
    ],
    imports: &[],
};

/// [`actor_matrix`](crate::m0588::actor_matrix) in `magic_0430`
/// (0x10003630, 559 bytes; the copy without the step down, read from the
/// code).
pub const ACTOR_MATRIX_0430: Sig = Sig {
    len: 559,
    hash: 0x3b4d_2f19_bf4c_3bea,
    calls: &[
        // __security_check_cookie
        (0x227, Callee::Any),
    ],
    imports: &[],
};

/// [`trail_draw`](crate::o6_fam_pairs6::trail_draw) in `magic_0543`
/// (0x10004d20, 1520 bytes: `magic_0644`'s code with its own frame
/// layout, linking each sprite as a part; the `rel32`s: the animation
/// step, the root, the part link, `__security_check_cookie`).
pub const TRAIL_DRAW_0543: Sig = Sig {
    len: 1520,
    hash: 0x1377_1308_0dbd_6dec,
    calls: &[
        (0xd9, Callee::Sig(&crate::sb_common::ANIM_STEP)),
        (0x1c2, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x5b6, Callee::Sig(&crate::sa_misc::LINK_PART)),
        (0x5e8, Callee::Any),
    ],
    imports: &[],
};

/// [`trail_head`](crate::s9_misc::trail_head) in `magic_0543`
/// (0x10001e80, 395 bytes: `magic_0446`'s code, its trail draw
/// [`TRAIL_DRAW_0543`]).
pub const TRAIL_HEAD_0543: Sig = Sig {
    len: 395,
    hash: 0x7ccd_415a_28c9_5e5d,
    calls: &[
        (0xbe, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0xcb, Callee::Any),
        (0xea, Callee::Sig(&crate::s4_fam446::SPARKS_D)),
        (0xf4, Callee::Sig(&crate::s4_fam446::SPARKS_C)),
        (0xfe, Callee::Sig(&crate::s4_fam446::SPARKS_A)),
        (0x173, Callee::Sig(&TRAIL_DRAW_0543)),
        (0x183, Callee::Any),
    ],
    imports: &[],
};

/// [`trail_task`](crate::o6_fam_pairs6::trail_task) in `magic_0543`
/// (0x10002250, 633 bytes: `r24::TRAIL_TASK_0446`'s code with 0543's
/// light set-up and trail draw).
pub const TRAIL_TASK_0543: Sig = Sig {
    len: 633,
    hash: 0xb4fb_fde7_2d07_d968,
    calls: &[
        (0xf9, Callee::Sig(&crate::s4_fam446::SPARKS_D)),
        (0x103, Callee::Sig(&crate::s4_fam446::SPARKS_C)),
        (0x10d, Callee::Sig(&crate::s4_fam446::SPARKS_A)),
        (0x120, Callee::Sig(&crate::r24::LIGHT_SETUP_0543)),
        (0x174, Callee::Sig(&crate::s4_fam446::START_CHILD_A)),
        (0x18c, Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
        (0x1c5, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x1d2, Callee::Any),
        (0x261, Callee::Sig(&TRAIL_DRAW_0543)),
        (0x271, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0395`'s first stage (0x10004380, 1426 bytes; an overlay table
/// entry, no arguments; an `ebp` /GS frame; `B = *G_B` the effect block):
/// - the actors dealt into the frame ([`deal`](crate::sb_misc::deal),
///   `ebp - 0xc`); the record callback one of two immediates by the battle
///   flag `*G_F`;
/// - for each actor `a` (`*G_N` of them): an object at `O = B + 0x720 +
///   0x130 i` ([`start_object_at`](crate::s4_misc::start_object_at), owner
///   the running word `n`, the callback `CB` as the second owner, its third
///   stack word a callee-left `ecx` never read): an actor whose kind is not
///   listed ([`actor_kind_check`](crate::sa_misc::actor_kind_check) 0)
///   gets index 2 (scale 0.14) in a battle or 3 (0.09), its point from
///   slots 262/268 `(c, ..)` and `y` = slot 12 `(c)` less 0.5; a listed one
///   index 1 outside a battle or 0 (0.09), the point from slots 262/268;
///   `c` the actor's character (the effect's own for -1); then `+0xdc`
///   1.0, the angle `B[0x1524]`, `+0x120` the actor (its byte, or the own
///   character), the record `R = B + 0x1398 + 0x34 i` linked at `+0x110`;
///   the spark block `S = B + 0x1160 + 0x30 i` (two sizes by `*G_F`) held
///   by a node off the free ring (inlined take, the callback the
///   immediate) appended to the task list; `n += 8 + rand () % 3`;
/// - in a battle a last block `B + 0x1318` (colours, counts) and its node.
///
/// Returns 0 outside a battle, else the old last node or 0xffff.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn stage_0395_a<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::sub;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0xe9), n);
    let ebp = fr.esp.wrapping_sub(4);
    let b = g.read::<u32>(w(0x13));
    let out = ebp.wrapping_sub(0xc);
    crate::sb_misc::deal(
        g,
        img,
        img.rel32_target(f + 0x20),
        ebp.wrapping_sub(0x3c),
        out,
    );
    let cb = if g.read::<u32>(w(0x25)) == 0 {
        w(0x31)
    } else {
        w(0x2c)
    };
    let mut owner = 0u32;
    let mut obj = b.wrapping_add(0x720);
    let mut spark = b.wrapping_add(0x1160);
    let mut rec = b.wrapping_add(0x1398);
    // an inlined take off the free ring: (head, new head, the callback,
    // the count, the list's first, last and the first's store)
    let take = |g: &mut G, ops: [u32; 8], block: u32| {
        let [count, head, to_next, to_0, cb, count_w, first, last_r] = ops;
        let mut free = g.read::<u16>(w(count));
        let mut node = 0u32;
        let mut eax = 0u32;
        if free != 0 {
            node = g.read::<u32>(w(head));
            let prev = g.read::<u32>(node);
            let next = g.read::<u32>(node + 4);
            g.write::<u32>(prev + 4, next);
            let next = g.read::<u32>(node + 4);
            let prev = g.read::<u32>(node);
            g.write::<u32>(next, prev);
            if free > 1 {
                let next = g.read::<u32>(node + 4);
                g.write::<u32>(w(to_next), next);
            } else {
                g.write::<u32>(w(to_0), 0);
            }
            eax = 0xffff;
            free -= 1;
            for o in [0u32, 4, 0xc, 0x14] {
                g.write::<u32>(node + o, 0);
            }
            g.write::<u32>(node + 8, node);
            g.write::<u32>(node + 0x10, w(cb));
            g.write::<u8>(node + 0x18, 0);
            g.write::<u16>(w(count_w), free);
        }
        let listed = g.read::<u32>(w(first)) != 0;
        g.write::<u8>(node + 0x18, 1);
        g.write::<u32>(node + 0x14, block);
        g.write::<u32>(node + 4, 0);
        (node, listed, eax, last_r)
    };
    let n = g.read::<i32>(w(0x58));
    let mut i = 0i32;
    while i < n {
        let a = u32::from(g.read::<u8>(out.wrapping_add(i.cast_unsigned())));
        let c = obj.wrapping_add(0xd4);
        let kind = crate::sa_misc::actor_kind_check(g, img, img.rel32_target(f + 0x7c), a);
        let chr = |g: &G, (own, sel, tab): (u32, u32, u32)| {
            if a == u32::MAX {
                g.read::<u32>(w(own))
            } else {
                let i = u32::from(g.read::<u8>(g.read::<u32>(w(sel)).wrapping_add(a)));
                g.read::<u32>(w(tab).wrapping_add(4 * i))
            }
        };
        let battle = g.read::<u32>(w(0x8a)) != 0;
        if kind == 0 {
            let (scale, index) = if battle { (0x94, 2) } else { (0xa3, 3) };
            let s = img.rel32_target(f + 0xba);
            crate::s4_misc::start_object_at(g, img, s, obj, k(g, scale), index, owner, w(0xb0));
            let who = chr(g, (0xc8, 0xcf, 0xda));
            // call width: 4
            g.call(slot(g, 262), 0, &[who, c - 4, c, c + 4]);
            g.write::<u32>(ebp.wrapping_sub(0x10), who);
            // call width: 3
            g.call(slot(g, 268), 0, &[who, 0x10, c - 4]);
            // call width: 1
            let h = st0_f32(g.call(slot(g, 12), 0, &[who]).st0);
            g.write::<f32>(ebp.wrapping_sub(0x10), h);
            g.write::<f32>(c, sub(h, k(g, 0x126)));
        } else {
            let index = u32::from(g.read::<u32>(w(0x145)) == 0);
            let s = img.rel32_target(f + 0x157);
            crate::s4_misc::start_object_at(g, img, s, obj, k(g, 0x13a), index, owner, w(0x14a));
            let who = chr(g, (0x165, 0x16c, 0x177));
            // call width: 4
            g.call(slot(g, 262), 0, &[who, c - 4, c, c + 4]);
            g.write::<u32>(ebp.wrapping_sub(0x10), who);
            // call width: 3
            g.call(slot(g, 268), 0, &[who, 0x10, c - 4]);
        }
        g.write::<u32>(c + 8, 0x3f80_0000);
        let yaw = g.read::<u32>(b + 0x1524);
        g.write::<u32>(c.wrapping_sub(0x10), yaw);
        let who = if a == u32::MAX {
            g.read::<u32>(w(0x1ca))
        } else {
            u32::from(g.read::<u8>(g.read::<u32>(w(0x1d1)).wrapping_add(a)))
        };
        g.write::<u32>(c + 0x4c, who);
        for o in [4u32, 0, 0xffff_fffc, 8] {
            g.write::<u32>(rec.wrapping_add(o), 0);
        }
        g.write::<u32>(rec + 0x10, cb);
        g.write::<u16>(rec + 0x1c, 0);
        g.write::<u32>(rec + 0xc, w(0x217));
        g.write::<u32>(rec + 0x14, 0);
        g.write::<u32>(rec + 0x18, 0);
        g.write::<u32>(c + 0x3c, rec.wrapping_sub(0x14));
        for o in [0u32, 0x1c, 0x24] {
            g.write::<u32>(spark + o, 0);
        }
        g.write::<u16>(spark + 0x2c, 0);
        let flag = g.read::<u32>(w(0x254)) != 0;
        let (sizes, ops) = if flag {
            g.write::<u32>(spark + 0x20, 0x3d08_8889);
            g.write::<u32>(spark + 0x28, 0x1e);
            (
                [0x3f80_0000u32, 0x3f00_0000, 0x4080_0000, 0x4000_0000],
                [0x1e6, 0x277, 0x294, 0x29c, 0x2cd, 0x2d8, 0x2de, 0x2f4],
            )
        } else {
            g.write::<u32>(spark + 0x20, 0x3d4c_cccd);
            g.write::<u32>(spark + 0x28, 0x14);
            (
                [0x3f00_0000, 0x3e80_0000, 0x4000_0000, 0x3f80_0000],
                [0x1e6, 0x348, 0x365, 0x36d, 0x39e, 0x3a9, 0x3af, 0x3c5],
            )
        };
        let (node, listed, _, last_r) = take(g, ops, spark);
        if listed {
            let last = g.read::<u32>(w(last_r));
            g.write::<u32>(node, last);
            g.write::<u32>(last + 4, node);
        } else {
            g.write::<u32>(node, 0);
            g.write::<u32>(w(if flag { 0x307 } else { 0x3d8 }), node);
        }
        g.write::<u32>(node + 8, node);
        for (o, v) in [4u32, 8, 0x10, 0x14].into_iter().zip(sizes) {
            g.write::<u32>(spark + o, v);
        }
        g.write::<u32>(spark + 0xc, 0);
        g.write::<u32>(spark + 0x18, 0);
        let fl = g.read::<u16>(spark + 0x2c) | 3;
        g.write::<u16>(spark + 0x2c, fl);
        g.write::<u32>(spark + 0x24, owner);
        g.write::<u32>(w(0x416), node);
        spark = spark.wrapping_add(0x30);
        let r = g.rand().cast_signed() % 3;
        owner = owner.wrapping_add(8).wrapping_add(r.cast_unsigned());
        rec = rec.wrapping_add(0x34);
        obj = obj.wrapping_add(0x130);
        i += 1;
        if i >= g.read::<i32>(w(0x445)) {
            break;
        }
    }
    if g.read::<u32>(w(0x450)) == 0 {
        return Ret::eax(0);
    }
    // the battle's last block
    let s = b.wrapping_add(0x1318);
    g.write::<u32>(w(0x46e), 0x3f00_0000);
    g.write::<u32>(w(0x478), 0x3f80_0000);
    for (o, v) in [
        (8u32, 0x3f00_0000u32),
        (4, 0x3f00_0000),
        (0, 0x3f00_0000),
        (0xc, 0x3f80_0000),
        (0x18, 0x3f80_0000),
        (0x14, 0x3f80_0000),
        (0x10, 0x3f80_0000),
        (0x1c, 0x3f80_0000),
        (0x28, 0x3c),
        (0x34, 0),
        (0x30, 0x1e),
        (0x2c, 0x1e),
    ] {
        g.write::<u32>(s + o, v);
    }
    let ops = [0x462, 0x4de, 0x4fb, 0x503, 0x534, 0x53f, 0x545, 0x55b];
    let (node, listed, mut eax, last_r) = take(g, ops, s);
    if listed {
        let last = g.read::<u32>(w(last_r));
        eax = last;
        g.write::<u32>(node, last);
        g.write::<u32>(last + 4, node);
    } else {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x56e), node);
    }
    g.write::<u32>(node + 8, node);
    g.write::<u32>(w(0x577), node);
    g.write::<u32>(s, 0x3f19_999a);
    Ret::eax(eax)
}

/// [`stage_0395_a`] (the `rel32`s: the deal, the kind check, the two
/// object starts, `__security_check_cookie`; `rand`).
pub const STAGE_0395_A: Sig = Sig {
    len: 1426,
    hash: 0x8d42_1a06_8ef3_7063,
    calls: &[
        (0x20, Callee::Sig(&crate::sb_misc::DEAL)),
        (0x7c, Callee::Sig(&crate::sa_misc::ACTOR_KIND_CHECK)),
        (0xba, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x157, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x58a, Callee::Any),
    ],
    imports: &[(0x41f, "rand")],
};

/// `magic_0395`'s second stage (0x10004950, 621 bytes; an overlay table
/// entry, no arguments; `B = *G_B` the effect block):
/// - object 4 started at `O = B + 0x4c0`
///   ([`start_object_at`](crate::s4_misc::start_object_at), scale `K1`,
///   `(4, 8, -, 0)`; the third word the caller's `ecx`, never read) at the
///   point `B + 0x10a0..` with `y` less 0.5, its angle `B[0x1524]`, `+0xe0`
///   -1.7 and `+0xe8` -10; the record `R = B + 0x1350` (its words, two
///   immediates) linked at `O + 0x110`;
/// - the sprite block `S = B + 0x10c0` (colours, sizes, the u16 0x48) held
///   by a pool node for the task ([`take`](crate::sa_pool::take), the
///   immediate) appended to the task list;
/// - the actor block (slot 413 in a battle, else 215; one of two callbacks
///   by `*G_F`); with `*G_F` set a fade
///   ([`fade_task`](crate::s3_misc::fade_task): `ecx` = `B + 0x12e0`,
///   `edx` 0x14, `xmm1` 1.0, `xmm2` 0.5, the word 8) and `B[0x12f0]` 0.6.
///
/// Returns the fade's `eax`, or the battle flag (1 or 0) without it.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn stage_0395<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::sub;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let b = g.read::<u32>(w(0xc));
    let o = b.wrapping_add(0x4c0);
    // (the third stack word, the caller's `ecx`, is never read)
    let start = img.rel32_target(f + 0x20);
    crate::s4_misc::start_object_at(g, img, start, o, k(g, 0x4), 4, 8, 0);
    for j in 0..4u32 {
        let x = g.read::<u32>(b + 0x10a0 + 4 * j);
        g.write::<u32>(o + 0xd0 + 4 * j, x);
    }
    let yaw = g.read::<u32>(b + 0x1524);
    let y = sub(g.read::<f32>(o + 0xd4), k(g, 0x66));
    g.write::<u32>(o + 0xc4, yaw);
    g.write::<u32>(o + 0xe0, 0xbfd9_999a);
    g.write::<u32>(o + 0xe8, 0xc120_0000);
    let r = b.wrapping_add(0x1350);
    g.write::<f32>(o + 0xd4, y);
    for off in [0x18u32, 0x14, 0x10, 0x1c] {
        g.write::<u32>(r + off, 0);
    }
    g.write::<u32>(r + 0x20, w(0xb1));
    g.write::<u32>(r + 0x24, w(0xb8));
    g.write::<u32>(r + 0x28, 0);
    g.write::<u32>(r + 0x2c, 0);
    g.write::<u16>(r + 0x30, 0);
    g.write::<u32>(o + 0x110, r);
    // the sprite block and its task node
    let s = b.wrapping_add(0x10c0);
    g.write::<u32>(s + 0x94, 0);
    g.write::<u32>(s + 0x80, 0x8080_8080);
    g.write::<u32>(s + 0x84, 0x0080_8080);
    g.write::<u32>(s + 0x88, 8);
    g.write::<u16>(s + 0x98, 0x48);
    g.write::<u32>(s + 0x90, 0x14);
    g.write::<u32>(s + 0x8c, 0x14);
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x126), w(0xe6));
    let listed = g.read::<u32>(w(0x12c)) != 0;
    g.write::<u32>(node + 0x14, s);
    g.write::<u32>(node + 4, 0);
    if listed {
        let tail = g.read::<u32>(w(0x13f));
        g.write::<u32>(node, tail);
        g.write::<u32>(tail + 4, node);
    } else {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x151), node);
    }
    let faded = g.read::<u32>(w(0x157)) != 0;
    g.write::<u32>(node + 8, node);
    g.write::<u32>(w(0x160), node);
    // the actor block
    let j = g.read::<u32>(w(0x165));
    g.write::<u32>(w(0x16a), j);
    g.write::<u32>(w(0x176), u32::MAX);
    let ops = if faded {
        [
            0x18d, 0x191, 0x186, 0x1a8, 0x1b1, 0x1bd, 0x1c7, 0x1d0, 0x1f7,
        ]
    } else {
        [
            0x219, 0x21d, 0x212, 0x234, 0x23d, 0x249, 0x253, 0x25c, 0x267,
        ]
    };
    let [cb, cb_v, battle_at, h_at, battle2, m1, z, one, flag] = ops;
    g.write::<u32>(w(cb), w(cb_v));
    let battle = g.read::<u32>(w(battle_at)) != 0;
    let t = slot_target(g, w(0x170), if battle { 413 } else { 215 });
    // call width: 1
    let h = g.call(t, 0, &[j]).eax;
    g.write::<u32>(w(h_at), h);
    let battle = g.read::<u32>(w(battle2)) != 0;
    g.write::<u32>(w(m1), u32::MAX);
    g.write::<u32>(w(z), 0);
    g.write::<u32>(w(one), 1);
    let ax = u16::from(battle);
    g.write::<u16>(w(flag), ax);
    if !faded {
        return Ret::eax(u32::from(ax));
    }
    // `push 8` under the two saved registers
    let sp = fr.esp.wrapping_sub(0xc);
    g.write::<u32>(sp, 8);
    let fade = Frame {
        at: img.rel32_target(f + 0x1fc),
        esp: sp.wrapping_sub(4),
        eax: 0,
        ecx: b.wrapping_add(0x12e0),
        edx: 0x14,
        xmm: [
            0,
            crate::Xmm::f32(k(g, 0x1e4)).reg(),
            crate::Xmm::f32(k(g, 0x1dc)).reg(),
            0,
        ],
    };
    let eax = crate::s3_misc::fade_task(g, img, &fade);
    g.write::<u32>(b + 0x12f0, 0x3f19_999a);
    Ret::eax(eax)
}

/// [`stage_0395`] (the `rel32`s: the object start, the pool take, the
/// fade).
pub const STAGE_0395: Sig = Sig {
    len: 621,
    hash: 0xd978_40aa_72a6_d292,
    calls: &[
        (0x20, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x126, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x1fc, Callee::Sig(&crate::s3_misc::FADE_TASK)),
    ],
    imports: &[],
};

/// [`lift_0301`](crate::r21::lift_0301) in `magic_0301` (0x10001810, 431
/// bytes: the kinds 0 and 1 and a tail of its own, read from the code; the
/// cookie check).
pub const LIFT_0301_1810: Sig = Sig {
    len: 431,
    hash: 0x36cc_69d6_760f_317f,
    calls: &[(0x1a7, Callee::Any)],
    imports: &[],
};

/// `magic_0301`'s two jittered boxes (0x10001000, 1765 bytes; cdecl
/// `(obj)`, an `ebp` /GS frame; `A = *G_A`): the points of bones 0x18 and
/// 0x25 of `A[0x198]` (slot 476, `P` at `ebp - 0x14`, `Q` at `ebp -
/// 0x64`), `D = P - Q` and `s` = slot 453 `(D)` times 0.5; for the bone
/// matrices of `A[0x1a0]` and then of `A[0x19c + 4 A[0x19c]]` (slot 463
/// `(b, 0, M)`, `M` at `ebp - 0x54`): `a` = the first row times `s`, `c`
/// = the third times -20, each part scaled by its own `r = rand () /
/// 32767 * 0.4 + 1` (`b = a r1`, `a' = a r2`, `c' = c r3`, `c'' = c r4`):
/// the corners `X = M3 + c'' - a'` and `Y = M3 + (c' + b)` into eight
/// globals per bone (the second bone's `Y` adds `M3` last). Then slots 450
/// `(obj, CB)` and 449 `(obj, CB2)`. (The parts' copies on the frame,
/// which nothing reads back, are left out.)
///
/// Returns slot 449's `eax`.
#[allow(clippy::too_many_lines)] // the original's two rounds, in order
pub fn boxes_0301<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, cvt, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x43), n);
    let obj = fr.arg(g, 0);
    let ebp = fr.esp.wrapping_sub(4);
    let at = |o: u32| ebp.wrapping_sub(o);
    let a = g.read::<u32>(w(0x15));
    let i = g.read::<u32>(a + 0x19c);
    let bone2 = g.read::<u32>(a.wrapping_add(4 * i).wrapping_add(0x19c));
    let skel = g.read::<u32>(a + 0x198);
    let bone1 = g.read::<u32>(a + 0x1a0);
    let (p, q, m) = (at(0x14), at(0x64), at(0x54));
    // call width: 3
    g.call(slot(g, 476), 0, &[skel, 0x18, p]);
    // call width: 3
    g.call(slot(g, 476), 0, &[skel, 0x25, q]);
    for j in 0..4 {
        let d = sub(g.read::<f32>(p + 4 * j), g.read::<f32>(q + 4 * j));
        g.write::<f32>(p + 4 * j, d);
    }
    // call width: 1
    let s = st0_f32(g.call(slot(g, 453), 0, &[p]).st0);
    let s = mul(s, k(g, 0xbf));
    // `rand () / 32767 * 0.4 + 1` (the three constants' operands)
    let rnd = |g: &mut G, (d, m, a): (u32, u32, u32)| {
        let r = cvt(g.rand());
        add(mul(div(r, k(g, d)), k(g, m)), k(g, a))
    };
    let row = |g: &G, r: u32, x: f32| -> [f32; 4] {
        core::array::from_fn(|j| mul(g.read::<f32>(m + 0x10 * r + 4 * j as u32), x))
    };
    let scale = |v: [f32; 4], r: f32| v.map(|x| mul(x, r));
    let m3 =
        |g: &G| -> [f32; 4] { core::array::from_fn(|j| g.read::<f32>(m + 0x30 + 4 * j as u32)) };
    // the first bone
    // call width: 3
    g.call(slot(g, 463), 0, &[bone1, 0, m]);
    g.write::<u32>(w(0xf2), bone1);
    let av = row(g, 0, s);
    let cv = row(g, 2, k(g, 0x124));
    let r1 = rnd(g, (0x17d, 0x185, 0x18d));
    let b = scale(av, r1);
    let r2 = rnd(g, (0x1e2, 0x1ea, 0x1f2));
    let a2 = scale(av, r2);
    let r3 = rnd(g, (0x23e, 0x246, 0x24e));
    let c1 = scale(cv, r3);
    let r4 = rnd(g, (0x2a6, 0x2b6, 0x2cb));
    let c2 = scale(cv, r4);
    let t = m3(g);
    for (j, o) in [0x336u32, 0x34e, 0x366, 0x37e].into_iter().enumerate() {
        let x = sub(add(t[j], c2[j]), a2[j]);
        g.write::<f32>(w(o), x);
    }
    for (j, o) in [0x3c6u32, 0x3ac, 0x3b4, 0x3bc].into_iter().enumerate() {
        let y = add(t[j], add(c1[j], b[j]));
        g.write::<f32>(w(o), y);
    }
    // the second bone
    // call width: 3
    g.call(slot(g, 463), 0, &[bone2, 0, m]);
    g.write::<u32>(w(0x3e3), bone2);
    let av = row(g, 0, s);
    let cv = row(g, 2, k(g, 0x415));
    let r1 = rnd(g, (0x46b, 0x473, 0x47b));
    let b = scale(av, r1);
    let r2 = rnd(g, (0x4cb, 0x4d3, 0x4db));
    let a2 = scale(av, r2);
    let r3 = rnd(g, (0x529, 0x531, 0x539));
    let c1 = scale(cv, r3);
    let r4 = rnd(g, (0x591, 0x5a1, 0x5b3));
    let c2 = scale(cv, r4);
    let t = m3(g);
    for (j, o) in [0x613u32, 0x630, 0x648, 0x660].into_iter().enumerate() {
        let x = sub(add(t[j], c2[j]), a2[j]);
        g.write::<f32>(w(o), x);
    }
    for (j, o) in [0x699u32, 0x691, 0x689, 0x6ac].into_iter().enumerate() {
        let y = add(add(c1[j], b[j]), t[j]);
        g.write::<f32>(w(o), y);
    }
    // call width: 2
    g.call(slot(g, 450), 0, &[obj, w(0x6b7)]);
    // call width: 2
    let eax = g.call(slot(g, 449), 0, &[obj, w(0x6c4)]).eax;
    Ret::eax(eax)
}

/// [`boxes_0301`] (the cookie check; `rand` through the IAT).
pub const BOXES_0301: Sig = Sig {
    len: 1765,
    hash: 0x2494_2eac_7df7_157f,
    calls: &[(0x6dd, Callee::Any)],
    imports: &[(0xec, "rand")],
};

/// `magic_0354`'s start-up (0x100042b0, 1071 bytes; internal, no
/// arguments, run by its load step `r9_load::load_0354`; an 8-aligned /GS
/// frame, the locals from `S` = `(ebp & !7) - 0x38` (after `push esi`);
/// `A = *G_A` the actor, `T = TAB[*I]` the target):
/// - three globals set, `*G_C` = slot 719 `(D, 8, 0)`; the part set up
///   ([`part_init`](crate::l5_objhost::part_init));
/// - `P` (`S + 0x24`) = slot 262 `(A, ..)`, `P.w` 1; `Q` (`S + 0x14`) =
///   slot 531 `(*G_B, Q)` with `Q.y = P.y`; `n = trunc(|P - Q| / 5)`
///   (`subps`, the root in double, `cvttss2si`); the camera path task
///   ([`camera_path_task`](crate::s7_misc::camera_path_task): `(Q, 1.1,
///   1.0, n)`, `xmm3` 0.2); the spark callback and a flag set;
/// - the yaw `K1 - atan2(Q.z - P.z, Q.x - P.x)` (`_CIatan2`) wrapped once;
///   the count word `max(n - 0x14, 1)`; object 2
///   ([`start_object_at`](crate::s4_misc::start_object_at), scale 0.1,
///   `(2, 0xa, -, CB)`) at slot 262 `(T, ..)` with `y` less 15; object 1
///   (scale 0.15, `(1, count + 4, -, 0)`) with `yaw + pi` wrapped and a
///   record of globals, its point `Q` with `y` less 0.5 and the yaw;
///   the third and later stack words of the starts are words the code
///   left there (a 1, a callee-left `ecx`), which no start reads;
/// - the task started ([`task_start`](crate::s3_misc::task_start)), a
///   global -1, the actor block ([`me_block`](crate::sa_misc::me_block));
/// - two task nodes ([`take`](crate::sa_pool::take), bytes `+0x18` 1 and
///   2) appended to the task list, the records set up
///   ([`records_init`](crate::sa_misc::records_init)) between them.
///
/// Returns 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn start_up_0354<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x15), n);
    let ebp = fr.esp.wrapping_sub(4);
    let s0 = (ebp & !7).wrapping_sub(0x38);
    let at = |o: u32| s0.wrapping_add(o);
    let rd = |g: &G, a: u32| g.read::<f32>(a);
    let one = 0x3f80_0000u32;
    for (o, v) in [(0x2bu32, 0x2fu32), (0x35, 0x39), (0x3f, 0x43)] {
        g.write::<u32>(w(o), w(v));
    }
    // call width: 3
    let c = g.call(slot(g, 719), 0, &[w(0x25), 8, 0]).eax;
    g.write::<u32>(w(0x4d), c);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x52));
    let (p, q) = (at(0x24), at(0x14));
    let a = g.read::<u32>(w(0x6c));
    // call width: 4
    g.call(slot(g, 262), 0, &[a, p, p + 4, p + 8]);
    let b = g.read::<u32>(w(0x84));
    g.write::<u32>(p + 0xc, one);
    // call width: 2
    g.call(slot(g, 531), 0, &[b, q]);
    let py = g.read::<u32>(p + 4);
    g.write::<u32>(q + 4, py);
    let d = |g: &G, j: u32| sub(rd(g, p + 4 * j), rd(g, q + 4 * j));
    let (dx, dy, dz) = (d(g, 0), d(g, 1), d(g, 2));
    let l2 = add(add(mul(dy, dy), mul(dx, dx)), mul(dz, dz));
    #[allow(clippy::cast_possible_truncation)] // cvtsd2ss
    let l = f64::from(l2).sqrt() as f32;
    let n = cvtt(div(l, k(g, 0xf2)));
    // the path task's `(Q, 1.1, 1.0, n)`, `xmm3` = 0.2
    for (j, x) in (0u32..).zip([q, 0x3f8c_cccd, one, n.cast_unsigned()]) {
        g.write::<u32>(s0.wrapping_sub(0x10) + 4 * j, x);
    }
    let path = Frame {
        at: img.rel32_target(f + 0x113),
        esp: s0.wrapping_sub(0x14),
        eax: 0,
        ecx: 0,
        edx: 0,
        xmm: [0, 0, 0, crate::Xmm::f32(k(g, 0xea)).reg()],
    };
    crate::s7_misc::camera_path_task(g, img, &path);
    let dz = sub(rd(g, q + 8), rd(g, p + 8));
    g.write::<u32>(w(0x128), w(0x12c));
    g.write::<u32>(w(0x132), 1);
    let dx = sub(rd(g, q), rd(g, p));
    // `_CIatan2(dz, dx)` on x87, the result rounded once (`cvtpd2ps`)
    #[allow(clippy::cast_possible_truncation)]
    let ang = g.atan2(f64::from(dz), f64::from(dx)) as f32;
    let yaw = crate::r20::wrap_once(
        g,
        img,
        f,
        sub(k(g, 0x173), ang),
        (0x18a, 0x183, 0x1a0, 0x183),
    );
    g.write::<f32>(at(8), yaw);
    let count = n.wrapping_sub(0x14);
    let count = if count <= 0 { 1 } else { count };
    g.write::<i32>(w(0x1d8), count);
    g.write::<u32>(w(0x1de), 0);
    // (the starts' third stack word is a word the code left there: never
    // read)
    let s1 = img.rel32_target(f + 0x1e7);
    crate::s4_misc::start_object_at(g, img, s1, w(0x1d3), k(g, 0x1b7), 2, 0xa, w(0x1c1));
    let i = u32::from(g.read::<u8>(g.read::<u32>(w(0x1ec))));
    let who = g.read::<u32>(w(0x205).wrapping_add(4 * i));
    // call width: 4
    g.call(slot(g, 262), 0, &[who, w(0x1fe), w(0x1f9), w(0x1f1)]);
    let y = sub(rd(g, w(0x21a)), k(g, 0x222));
    g.write::<u32>(w(0x22b), one);
    g.write::<f32>(w(0x237), y);
    let y2 = crate::r20::wrap_once(
        g,
        img,
        f,
        add(yaw, k(g, 0x245)),
        (0x24c, 0x256, 0x260, 0x26d),
    );
    g.write::<u16>(w(0x27e), 0);
    let owner = g.read::<u32>(w(0x283)).wrapping_add(4);
    g.write::<f32>(w(0x297), y2);
    for o in [0x29du32, 0x2a7, 0x2b1, 0x2bb] {
        g.write::<u32>(w(o), 0);
    }
    g.write::<u32>(w(0x2c5), w(0x2c9));
    g.write::<u32>(w(0x2cf), w(0x2d3));
    for o in [0x2d9u32, 0x2e3] {
        g.write::<u32>(w(o), 0);
    }
    g.write::<u32>(w(0x2ed), w(0x2f1));
    let s2 = img.rel32_target(f + 0x2f6);
    crate::s4_misc::start_object_at(g, img, s2, w(0x28f), k(g, 0x275), 1, owner, 0);
    let i = u32::from(g.read::<u8>(g.read::<u32>(w(0x2fb))));
    let who = g.read::<u32>(w(0x314).wrapping_add(4 * i));
    // call width: 4
    g.call(slot(g, 262), 0, &[who, w(0x30d), w(0x308), w(0x300)]);
    for (to, o) in [(0x32fu32, 0u32), (0x33d, 8), (0x34b, 0xc)] {
        let x = g.read::<u32>(q + o);
        g.write::<u32>(w(to), x);
    }
    let qy = sub(rd(g, q + 4), k(g, 0x359));
    g.write::<f32>(w(0x369), qy);
    g.write::<f32>(w(0x377), yaw);
    // the task (one stack word under the start's last), the actor block
    let sp = s0.wrapping_sub(8);
    g.write::<u32>(sp, w(0x361));
    let ts = Frame {
        at: img.rel32_target(f + 0x37c),
        esp: sp.wrapping_sub(4),
        ..Frame::default()
    };
    crate::s3_misc::task_start(g, img, &ts);
    g.write::<u32>(w(0x387), u32::MAX);
    crate::sa_misc::me_block(g, img, img.rel32_target(f + 0x390), w(0x381));
    // the two task nodes
    let append = |g: &mut G, node: u32, byte: u8, [first, last, set_first, set_last]: [u32; 4]| {
        let listed = g.read::<u32>(w(first)) != 0;
        g.write::<u8>(node + 0x18, byte);
        g.write::<u32>(node + 4, 0);
        if listed {
            let tail = g.read::<u32>(w(last));
            g.write::<u32>(node, tail);
            g.write::<u32>(tail + 4, node);
        } else {
            g.write::<u32>(node, 0);
            g.write::<u32>(w(set_first), node);
        }
        g.write::<u32>(node + 8, node);
        g.write::<u32>(w(set_last), node);
    };
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x39d), w(0x398));
    append(g, node, 1, [0x3a3, 0x3b7, 0x3c9, 0x3d6]);
    crate::sa_misc::records_init(g, img, img.rel32_target(f + 0x3db), w(0x3ce));
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x3e5), w(0x3e0));
    append(g, node, 2, [0x3eb, 0x3ff, 0x411, 0x41e]);
    Ret::eax(0)
}

/// [`start_up_0354`] (the `rel32`s: the part set-up, the root, the path
/// task, `_CIatan2`, the two object starts, the task start, the actor
/// block, the two pool takes and the records, `__security_check_cookie`).
pub const START_UP_0354: Sig = Sig {
    len: 1071,
    hash: 0xc5a2_bece_e759_bbf8,
    calls: &[
        (0x52, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0xde, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x113, Callee::Sig(&crate::s7_misc::CAMERA_PATH_TASK)),
        (0x161, Callee::Import("_CIatan2")),
        (0x1e7, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x2f6, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x37c, Callee::Sig(&crate::s3_misc::TASK_START)),
        (0x390, Callee::Sig(&crate::sa_misc::ME_BLOCK)),
        (0x39d, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x3db, Callee::Sig(&crate::sa_misc::RECORDS_INIT)),
        (0x3e5, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x427, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0354`'s two sparks (0x100047a0, 957 bytes; a stored callback,
/// cdecl, no arguments; an `ebp` /GS frame): the point `P` (`ebp - 0x14`)
/// = slot 262 `(*G_A, ..)`, `P.w` 1; two records from the pool
/// ([`pool_take`](crate::sb_misc::pool_take); none: 0) with the parts
/// `(*G_P)[0]` and `[1]`, `+0x34` 1, the u16s `+0x38` -1 and `+0x3a`
/// 0x200, the point `P` moved by slot 314 `(2.0)` in `x` and `z` and 2.0
/// down; the velocities and the turn from slot 314 numbers (`(0.2, 0.1 -
/// 0.3, 0.2, pi)` then `(0.5, 0.5 - 2, 0.5, pi)`), the steps `v * K`
/// (the second's `y` `-(r + 0.1) v.y`), the size `(slot 314 (0.5) + 1)
/// K'`, the counts and colours.
///
/// Returns the last slot 314's `eax`, or 0 without a record.
#[allow(clippy::too_many_lines)] // the original's two sparks, in order
pub fn sparks_0354<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G| slot_target(g, w(0x1e), 314);
    let ebp = fr.esp.wrapping_sub(4);
    let p = ebp.wrapping_sub(0x14);
    let a = g.read::<u32>(w(0x24));
    let t262 = slot_target(g, w(0x1e), 262);
    // call width: 4
    g.call(t262, 0, &[a, p, p + 4, p + 8]);
    g.write::<u32>(p + 0xc, 0x3f80_0000);
    let mut eax = 0;
    let mut num = |g: &mut G, x: u32| {
        // call width: 1 (one word, rewritten for each call)
        let out = g.call(slot(g), 0, &[x]);
        eax = out.eax;
        st0_f32(out.st0)
    };
    let take = |g: &mut G, call: u32| crate::sb_misc::pool_take(g, img, img.rel32_target(f + call));
    let set_up = |g: &mut G, rec: u32, part_ix: u32| {
        let part = g.read::<u32>(g.read::<u32>(w(0x4b)).wrapping_add(4 * part_ix));
        g.write::<u32>(rec + 0x30, part);
        g.write::<u32>(rec + 0x34, 1);
        g.write::<u16>(rec + 0x3a, 0x200);
        g.write::<u16>(rec + 0x38, 0xffff);
        for j in 0..4u32 {
            let x = g.read::<u32>(p + 4 * j);
            g.write::<u32>(rec + 4 * j, x);
        }
    };
    let rec = take(g, 0x3b);
    if rec == 0 {
        return Ret::eax(0);
    }
    set_up(g, rec, 0);
    let r = num(g, 0x4000_0000);
    g.write::<f32>(rec, add(g.read::<f32>(rec), r));
    g.write::<f32>(rec + 4, sub(g.read::<f32>(rec + 4), k(g, 0xcd)));
    let r = num(g, 0x4000_0000);
    g.write::<f32>(rec + 8, add(g.read::<f32>(rec + 8), r));
    let v = num(g, 0x3e4c_cccd);
    g.write::<f32>(rec + 0x10, v);
    let v = sub(num(g, 0x3dcc_cccd), k(g, 0x127));
    g.write::<f32>(rec + 0x14, v);
    let v = num(g, 0x3e4c_cccd);
    g.write::<f32>(rec + 0x18, v);
    let v = num(g, 0x4049_0fdb);
    g.write::<f32>(rec + 0x1c, v);
    let kk = k(g, 0x162);
    for (from, to) in [(0x10u32, 0x20u32), (0x14, 0x24), (0x18, 0x28)] {
        let x = mul(g.read::<f32>(rec + from), kk);
        if from == 0x10 {
            g.write::<u32>(rec + 0x2c, 0x3f80_0000);
        }
        g.write::<f32>(rec + to, x);
    }
    let s = mul(add(num(g, 0x3f00_0000), k(g, 0x1b7)), k(g, 0x1c2));
    g.write::<f32>(rec + 0x3c, s);
    g.write::<u32>(rec + 0x4c, 0x20);
    g.write::<u32>(rec + 0x50, 0x20);
    g.write::<u16>(rec + 0x64, 0xfafa);
    g.write::<u8>(rec + 0x66, 0xfa);
    g.write::<u32>(rec + 0x68, 0x00fa_fafa);
    // the second spark
    let rec = take(g, 0x1eb);
    if rec == 0 {
        return Ret::eax(0);
    }
    set_up(g, rec, 1);
    let r = num(g, 0x4000_0000);
    g.write::<f32>(rec, add(g.read::<f32>(rec), r));
    g.write::<f32>(rec + 4, sub(g.read::<f32>(rec + 4), k(g, 0x27a)));
    let r = num(g, 0x4000_0000);
    g.write::<f32>(rec + 8, add(g.read::<f32>(rec + 8), r));
    let v = num(g, 0x3f00_0000);
    g.write::<f32>(rec + 0x10, v);
    let v = sub(num(g, 0x3f00_0000), k(g, 0x2d4));
    g.write::<f32>(rec + 0x14, v);
    let v = num(g, 0x3f00_0000);
    g.write::<f32>(rec + 0x18, v);
    let v = num(g, 0x4049_0fdb);
    g.write::<f32>(rec + 0x1c, v);
    let x = mul(g.read::<f32>(rec + 0x10), k(g, 0x314));
    g.write::<f32>(rec + 0x20, x);
    let r = num(g, 0x3ca3_d70a);
    g.write::<u32>(rec + 0x2c, 0x3f80_0000);
    // `xorps` with the sign mask
    let y = mul(add(r, k(g, 0x342)), g.read::<f32>(rec + 0x14));
    let y = f32::from_bits(y.to_bits() ^ g.read::<u32>(w(0x35c)));
    g.write::<f32>(rec + 0x24, y);
    let z = mul(g.read::<f32>(rec + 0x18), k(g, 0x36e));
    g.write::<f32>(rec + 0x28, z);
    let s = mul(add(num(g, 0x3f00_0000), k(g, 0x38e)), k(g, 0x3a4));
    g.write::<u32>(rec + 0x4c, 0x18);
    g.write::<u32>(rec + 0x50, 0);
    g.write::<f32>(rec + 0x3c, s);
    Ret::eax(eax)
}

/// [`sparks_0354`] (the `rel32`s: the two pool takes,
/// `__security_check_cookie`).
pub const SPARKS_0354: Sig = Sig {
    len: 957,
    hash: 0x3457_518e_fa19_b052,
    calls: &[
        (0x3b, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
        (0x1eb, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
        (0x3b5, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0243`'s start-up (0x10001340, 1232 bytes; internal, no
/// arguments, run by its load step `r9_load::load_0243`; an `ebp` /GS
/// frame, `V` at `ebp - 0x14`; `T = TAB[*I]` the target):
/// - three globals set, `*G_C` = slot 719 `(D, 8, 0)`; the parts buffer
///   ([`parts_buffer`](crate::s4_small::parts_buffer)); object 2
///   ([`start_object_owners`](crate::s4_misc::start_object_owners), scale
///   0.4, `(2, 0x30, 0xa, CB)`) at slot 262 `(T, ..)`, `w` 1, a callback;
/// - for each actor `i` (`*G_N`): object 1 at `OBJ + 0x130 i` (`(1, 0x3a,
///   0, CB2)`), its point from slot 262 `(c, ..)` with `y` = `V.y` from
///   slot 268 `(c, 6, V)` (`c` the actor's character), `w` 1, `+0x120`
///   the actor's byte;
/// - with `X = *G_X`: slot 596 `(0)`; slot 597 `(0, X, 6, 0)` twice, `(0,
///   X, 0, R)` for each 16-byte record `R` of the DLL's table, `(0, X, 1,
///   0)` twice; slot 598 `(0, X, 0x80, 0.5)`; the task started
///   ([`task_start`](crate::s3_misc::task_start));
/// - the actor block (slot 413 in a battle, else 215);
/// - three nodes off the free ring (inlined takes, the callbacks the
///   immediates; the first marked 1, the third 2) appended to the task
///   list, the pool reset ([`pool_reset_take`](crate::o6_fam_pool::pool_reset_take))
///   between the second and the third; a global 0x5a.
///
/// Returns 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn start_up_0243<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x11), n);
    let ebp = fr.esp.wrapping_sub(4);
    let v = ebp.wrapping_sub(0x14);
    let one = 0x3f80_0000u32;
    for (o, x) in [(0x29u32, 0x2du32), (0x33, 0x37), (0x3d, 0x41)] {
        g.write::<u32>(w(o), w(x));
    }
    // call width: 3
    let c = g.call(slot(g, 719), 0, &[w(0x23), 8, 0]).eax;
    g.write::<u32>(w(0x4b), c);
    let pb = Frame {
        at: img.rel32_target(f + 0x50),
        esp: ebp.wrapping_sub(0x2c),
        ..Frame::default()
    };
    crate::s4_small::parts_buffer(g, img, &pb);
    let start = |g: &mut G, (call, scale): (u32, u32), obj: u32, [index, owner, a, b]: [u32; 4]| {
        let at = img.rel32_target(f + call);
        crate::s4_misc::start_object_owners(g, img, at, obj, k(g, scale), index, owner, (a, b));
    };
    start(g, (0x6d, 0x58), w(0x68), [2, 0x30, 0xa, w(0x5d)]);
    let i = u32::from(g.read::<u8>(g.read::<u32>(w(0x72))));
    let who = g.read::<u32>(w(0x8b).wrapping_add(4 * i));
    // call width: 4
    g.call(slot(g, 262), 0, &[who, w(0x84), w(0x7f), w(0x77)]);
    g.write::<u32>(w(0xa3), one);
    g.write::<u32>(w(0xad), w(0xb1));
    // the actors' objects
    let n = g.read::<i32>(w(0xb7));
    let mut i = 0i32;
    let mut c = w(0xc2);
    while i < n {
        start(g, (0xea, 0xd4), c.wrapping_sub(0xd4), [1, 0x3a, 0, w(0xd9)]);
        let a = u32::from(g.read::<u8>(g.read::<u32>(w(0xff)).wrapping_add(i.cast_unsigned())));
        let who = g.read::<u32>(w(0x10a).wrapping_add(4 * a));
        // call width: 4
        g.call(slot(g, 262), 0, &[who, c - 4, c, c + 4]);
        // call width: 3
        g.call(slot(g, 268), 0, &[who, 6, v]);
        let vy = g.read::<u32>(v + 4);
        g.write::<u32>(c, vy);
        g.write::<u32>(c + 8, one);
        let b = u32::from(g.read::<u8>(g.read::<u32>(w(0x15c)).wrapping_add(i.cast_unsigned())));
        g.write::<u32>(c + 0x4c, b);
        i += 1;
        c = c.wrapping_add(0x130);
        if i >= g.read::<i32>(w(0x170)) {
            break;
        }
    }
    let x = g.read::<u32>(w(0x17e));
    // call width: 1
    g.call(slot(g, 596), 0, &[0]);
    for _ in 0..2 {
        // call width: 4
        g.call(slot(g, 597), 0, &[0, x, 6, 0]);
    }
    let mut r = w(0x1bd);
    while r < w(0x1dc) {
        // call width: 4
        g.call(slot(g, 597), 0, &[0, x, 0, r]);
        r = r.wrapping_add(0x10);
    }
    for _ in 0..2 {
        // call width: 4
        g.call(slot(g, 597), 0, &[0, x, 1, 0]);
    }
    // call width: 4 (one more word under them)
    g.call(slot(g, 598), 0, &[0, x, 0x80, 0x3f00_0000, 0]);
    // `push` over the word slot 598's first push left
    let sp = ebp.wrapping_sub(0x28);
    g.write::<u32>(sp, w(0x230));
    let ts = Frame {
        at: img.rel32_target(f + 0x235),
        esp: sp.wrapping_sub(4),
        ..Frame::default()
    };
    crate::s3_misc::task_start(g, img, &ts);
    // the actor block
    let j = g.read::<u32>(w(0x23a));
    let battle = g.read::<u32>(w(0x243)) != 0;
    g.write::<u32>(w(0x249), j);
    g.write::<u32>(w(0x255), u32::MAX);
    g.write::<u32>(w(0x25f), w(0x263));
    g.write::<u32>(w(0x269), u32::MAX);
    // call width: 1
    let h = g.call(slot(g, if battle { 413 } else { 215 }), 0, &[j]).eax;
    let battle = g.read::<u32>(w(0x288)) != 0;
    g.write::<u32>(w(0x293), h);
    g.write::<u32>(w(0x299), u32::MAX);
    g.write::<u32>(w(0x2a3), 0);
    g.write::<u32>(w(0x2ad), 1);
    if battle {
        g.write::<u16>(w(0x2c0), 1);
    } else {
        g.write::<u16>(w(0x2b7), 0);
    }
    // three inlined takes off the free ring
    let take =
        |g: &mut G, free: &mut u16, head: u32, (to_next, to_0): (u32, u32), cb: u32, count: u32| {
            if *free == 0 {
                return 0;
            }
            let node = g.read::<u32>(w(head));
            let prev = g.read::<u32>(node);
            let next = g.read::<u32>(node + 4);
            g.write::<u32>(prev + 4, next);
            let next = g.read::<u32>(node + 4);
            let prev = g.read::<u32>(node);
            g.write::<u32>(next, prev);
            if *free > 1 {
                let next = g.read::<u32>(node + 4);
                g.write::<u32>(w(to_next), next);
            } else {
                g.write::<u32>(w(to_0), 0);
            }
            *free -= 1;
            for o in [0u32, 4, 0xc, 0x14] {
                g.write::<u32>(node + o, 0);
            }
            g.write::<u32>(node + 8, node);
            g.write::<u32>(node + 0x10, w(cb));
            g.write::<u8>(node + 0x18, 0);
            g.write::<u16>(w(count), *free);
            node
        };
    let mut free = g.read::<u16>(w(0x2c7));
    let n1 = take(g, &mut free, 0x2e1, (0x306, 0x306), 0x32b, 0x336);
    let first = g.read::<u32>(w(0x33c));
    g.write::<u8>(n1 + 0x18, 1);
    g.write::<u32>(n1 + 4, 0);
    let first = if first == 0 {
        g.write::<u32>(n1, 0);
        g.write::<u32>(w(0x365), n1);
        n1
    } else {
        let last = g.read::<u32>(w(0x350));
        g.write::<u32>(n1, last);
        g.write::<u32>(last + 4, n1);
        first
    };
    g.write::<u32>(n1 + 8, n1);
    // (the second take starts from the head the first one left: the
    // original keeps it in `edx`)
    let n2 = take(g, &mut free, 0x306, (0x38e, 0x396), 0x3c7, 0x3d2);
    g.write::<u32>(n2 + 4, 0);
    if first == 0 {
        g.write::<u32>(n2, 0);
        g.write::<u32>(w(0x3f0), n2);
    } else {
        g.write::<u32>(n2, n1);
        g.write::<u32>(n1 + 4, n2);
    }
    g.write::<u32>(n2 + 8, n2);
    g.write::<u32>(w(0x3fe), n2);
    crate::o6_fam_pool::pool_reset_take(g, img, img.rel32_target(f + 0x403), w(0x3f5));
    let mut free = g.read::<u16>(w(0x40a));
    let n3 = take(g, &mut free, 0x419, (0x436, 0x43e), 0x46f, 0x47a);
    let listed = g.read::<u32>(w(0x480)) != 0;
    g.write::<u8>(n3 + 0x18, 2);
    g.write::<u32>(n3 + 4, 0);
    if listed {
        let last = g.read::<u32>(w(0x496));
        g.write::<u32>(n3, last);
        g.write::<u32>(last + 4, n3);
    } else {
        g.write::<u32>(n3, 0);
        g.write::<u32>(w(0x4a9), n3);
    }
    g.write::<u32>(n3 + 8, n3);
    g.write::<u32>(w(0x4b7), n3);
    g.write::<u32>(w(0x4bd), 0x5a);
    Ret::eax(0)
}

/// [`start_up_0243`] (the `rel32`s: the parts buffer, the two object
/// starts, the task start, the pool reset, `__security_check_cookie`).
pub const START_UP_0243: Sig = Sig {
    len: 1232,
    hash: 0xdedd_660d_4f2b_1d20,
    calls: &[
        (0x50, Callee::Sig(&crate::s4_small::PARTS_BUFFER_100000)),
        (0x6d, Callee::Sig(&crate::s4_misc::START_OBJECT_OWNERS)),
        (0xea, Callee::Sig(&crate::s4_misc::START_OBJECT_OWNERS)),
        (0x235, Callee::Sig(&crate::s3_misc::TASK_START)),
        (
            0x403,
            Callee::Sig(&crate::o6_fam_pool::POOL_RESET_TAKE_0243_2800),
        ),
        (0x4c8, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0243`'s burst of sparks (0x10001880, 1220 bytes; a stored
/// callback, cdecl, no arguments; an `ebp` /GS frame): while the count
/// `*G_N` runs (one less each call): `A` (`ebp - 0x24`) = slot 237 `(T, 6,
/// 0, A)` with `A.y + 5`, `C` (`ebp - 0x34`) = slot 237 `(T, 6, B, C)`
/// for `B = (-800, 0, 0, 1)`; eight ring entries
/// ([`ring_take`](crate::sa_misc::ring_take); none: the end) and then two
/// more found inline (the same ring, searched from its rover; none: the
/// end) are set up at `A` (even) or `C` (odd): the part `(*G_P)[1]` (the
/// last two `[0]`), the u16s `+8` -1 and `+0xa` 0x200, `+4` 1, the point
/// moved by three slot 314 `(5)` numbers, the velocity `((r - 1.5) 0.6,
/// (r - 1.5) 0.8 - 0.38, (r - 1.5) 0.6)` (slot 307), the turn slot 314
/// `(pi)`, the size `(slot 314 (1) + 1) K`, the life `rand () % 16 + 16`
/// (`+0x34`, `+0x38`); the last two also their colour bytes.
///
/// Returns the count (not running), 0 or the rover (no entry), or the
/// last life.
#[allow(clippy::too_many_lines)] // the original's two loops, in order
pub fn sparks_0243<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x2a), n);
    let ebp = fr.esp.wrapping_sub(4);
    let at = |o: u32| ebp.wrapping_sub(o);
    let n = g.read::<i32>(w(0x11));
    if n <= 0 {
        return Ret::eax(n.cast_unsigned());
    }
    g.write::<i32>(w(0x20), n - 1);
    let (a, b, c) = (at(0x24), at(0x44), at(0x34));
    let t = g.read::<u32>(w(0x3a));
    // call width: 4
    g.call(slot(g, 237), 0, &[t, 6, 0, a]);
    let ay = add(g.read::<f32>(a + 4), k(g, 0x49));
    let t = g.read::<u32>(w(0x5e));
    g.write::<u32>(a + 0xc, 0x3f80_0000);
    g.write::<f32>(a + 4, ay);
    for (j, x) in (0u32..).zip([0xc448_0000u32, 0, 0, 0x3f80_0000]) {
        g.write::<u32>(b + 4 * j, x);
    }
    // call width: 4
    g.call(slot(g, 237), 0, &[t, 6, b, c]);
    g.write::<u32>(c + 0xc, 0x3f80_0000);
    // one spark: (its entry, the even/odd point, the part's index, the
    // operands of its constants: 1.5, 0.6, 0.8, 0.38, 0.6, 1.0, the size's
    // factor)
    let spark = |g: &mut G, rec: u32, odd: bool, part: u32, ops: [u32; 7]| -> u32 {
        let p = if odd { c } else { a };
        let px: [f32; 3] = core::array::from_fn(|j| g.read::<f32>(p + 4 * j as u32));
        let pt = g.read::<u32>(g.read::<u32>(w(0xda)).wrapping_add(4 * part));
        g.write::<u32>(rec, pt);
        g.write::<u16>(rec + 8, 0xffff);
        g.write::<u16>(rec + 0xa, 0x200);
        g.write::<u32>(rec + 4, 1);
        let num = |g: &mut G, s: u32, x: u32| st0_f32(g.call(slot(g, s), 0, &[x]).st0);
        for (j, o) in [0xcu32, 0x10, 0x14].into_iter().enumerate() {
            let r = num(g, 314, 0x40a0_0000);
            g.write::<f32>(rec + o, add(r, px[j]));
        }
        g.write::<u32>(rec + 0x18, 0x3f80_0000);
        let [k15, k06, k08, k38, k06b, k1, ks] = ops;
        // (slot 307 sees the 5.0 still on the stack)
        let r = num(g, 307, 0x40a0_0000);
        g.write::<f32>(rec + 0x1c, mul(sub(r, k(g, k15)), k(g, k06)));
        let r = num(g, 307, 0x40a0_0000);
        g.write::<f32>(
            rec + 0x20,
            sub(mul(sub(r, k(g, k15 + 0x2a)), k(g, k08)), k(g, k38)),
        );
        let r = num(g, 307, 0x40a0_0000);
        g.write::<f32>(rec + 0x24, mul(sub(r, k(g, k15 + 0x5c)), k(g, k06b)));
        let r = num(g, 314, 0x4049_0fdb);
        g.write::<f32>(rec + 0x28, r);
        let r = num(g, 314, 0x3f80_0000);
        g.write::<f32>(rec + 0x2c, mul(add(r, k(g, k1)), k(g, ks)));
        let life = (g.rand().cast_signed() % 16 + 16).cast_unsigned();
        g.write::<u32>(rec + 0x34, life);
        g.write::<u32>(rec + 0x38, life);
        life
    };
    for i in 0..8u32 {
        let rec = crate::sa_misc::ring_take(g, img, img.rel32_target(f + 0xa6));
        if rec == 0 {
            return Ret::eax(0);
        }
        spark(
            g,
            rec,
            i & 1 != 0,
            1,
            [0x192, 0x19f, 0x1c9, 0x1d1, 0x202, 0x236, 0x241],
        );
        g.write::<u8>(rec + 0x47, 0);
    }
    let mut eax = 0;
    for i in 0..2u32 {
        let n = g.read::<i32>(w(0x273));
        let mut cur = g.read::<i32>(w(0x278));
        if n <= 0 {
            return Ret::eax(cur.cast_unsigned());
        }
        let base = g.read::<u32>(w(0x288));
        let mut found = None;
        for _ in 0..n {
            let e = base.wrapping_add(cur.cast_unsigned().wrapping_mul(0x50));
            cur += 1;
            if cur >= n {
                cur = 0;
            }
            if g.read::<u32>(e + 0x48) == 0 {
                found = Some(e);
                break;
            }
        }
        let Some(rec) = found else {
            return Ret::eax(cur.cast_unsigned());
        };
        g.write::<u32>(rec + 0x40, 0x8080_8080);
        g.write::<u32>(rec + 0x44, 0x8080_8080);
        g.write::<u32>(rec + 0x3c, 0);
        g.write::<u16>(rec + 0x4c, 0);
        g.write::<u32>(rec + 0x48, 1);
        g.write::<i32>(w(0x2e5), cur);
        eax = spark(
            g,
            rec,
            i & 1 != 0,
            0,
            [0x3c6, 0x3d3, 0x3fd, 0x405, 0x436, 0x46a, 0x475],
        );
        g.write::<u8>(rec + 0x41, 0xa0);
        g.write::<u8>(rec + 0x43, 0x40);
        g.write::<u8>(rec + 0x45, 0xa0);
        g.write::<u8>(rec + 0x47, 0);
    }
    Ret::eax(eax)
}

/// [`sparks_0243`] (the `rel32`s: the ring take, `__security_check_cookie`
/// twice; `rand` through the IAT).
pub const SPARKS_0243: Sig = Sig {
    len: 1220,
    hash: 0x91ff_9486_2a50_1529,
    calls: &[
        (0xa6, Callee::Sig(&crate::sa_misc::RING_TAKE)),
        (0x2ba, Callee::Any),
        (0x4bc, Callee::Any),
    ],
    imports: &[(0x94, "rand"), (0x480, "rand")],
};

/// `magic_0163`'s start-up (0x10001430, 2172 bytes; internal, no
/// arguments, run by its load step `r9_load::load_0163` (the 119-byte
/// form); an `ebp` /GS frame, `P` at `ebp - 0x14`; `T = TAB[*I]` the
/// target):
/// - two globals set, `*G_C` = slot 719 `(D, 8, 0)`; the part set up
///   ([`part_init`](crate::l5_objhost::part_init)); the yaw `K1 - slot
///   333 (T)` wrapped once;
/// - slot 390 `(*I)` the target's kind: 0x108c puts `P` at slot 262 `(T,
///   ..)` with slot 268 `(T, 0x11, P)`, any other at slot 262 alone; `P.w`
///   1;
/// - the objects ([`start_object_at`](crate::s4_misc::start_object_at),
///   their third stack word a leftover `ecx` no start reads), each at `P`
///   with the yaw: 1 `(1, 0)` with `y` less 14 and then 150, 0 `(0, 0x10,
///   -, CB)` with `y` less 14 and a record of globals; unless the state is
///   listed ([`state_listed`](crate::s4_misc::state_listed)) 3 `(3, 0x10)`
///   and 4 `(4, 0xe)` with `y` less 0.5 and 14; three more of index 2
///   (owners 10, 12, 14) climbing from object 1's `y` plus 75 by 18.75;
/// - the static record node
///   ([`static_record_node`](crate::o4_small::static_record_node)), the
///   records of two colour blocks, two nodes off the free ring holding them
///   (inlined takes, marked 1), slot 271 `(4)`, a sprite record, a third
///   node for it, the task started ([`task_start`](crate::s3_misc::task_start)),
///   a fourth node (marked 2); each appended to the task list.
///
/// Returns 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn start_up_0163<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x11), n);
    let ebp = fr.esp.wrapping_sub(4);
    let at = |o: u32| ebp.wrapping_sub(o);
    let p = at(0x14);
    let rd = |g: &G, a: u32| g.read::<f32>(a);
    let one = 0x3f80_0000u32;
    g.write::<u32>(w(0x26), w(0x2a));
    g.write::<u32>(w(0x30), w(0x34));
    // call width: 3
    let c = g.call(slot(g, 719), 0, &[w(0x20), 8, 0]).eax;
    g.write::<u32>(w(0x3e), c);
    crate::l5_objhost::part_init(g, img, img.rel32_target(f + 0x43));
    let target = |g: &G, sel: u32, tab: u32| {
        let i = u32::from(g.read::<u8>(g.read::<u32>(w(sel))));
        g.read::<u32>(w(tab).wrapping_add(4 * i))
    };
    let t = target(g, 0x48, 0x52);
    // call width: 1
    let a = st0_f32(g.call(slot(g, 333), 0, &[t]).st0);
    g.write::<f32>(at(0x1c), a);
    let yaw = crate::r20::wrap_once(g, img, f, sub(k(g, 0x6a), a), (0x79, 0x88, 0x92, 0x9f));
    g.write::<f32>(at(0x18), yaw);
    let i = u32::from(g.read::<u8>(g.read::<u32>(w(0xa9))));
    // call width: 1
    let kind = g.call(slot(g, 390), 0, &[i]).eax;
    if kind == 0x108c {
        let who = target(g, 0xca, 0xd6);
        // call width: 4
        g.call(slot(g, 262), 0, &[who, p, p + 4, p + 8]);
        // call width: 3
        g.call(slot(g, 268), 0, &[who, 0x11, p]);
    } else {
        let who = target(g, 0xca, 0x11c);
        // call width: 4
        g.call(slot(g, 262), 0, &[who, p, p + 4, p + 8]);
    }
    let start =
        |g: &mut G, (call, obj, scale): (u32, u32, u32), (index, owner, b): (u32, u32, u32)| {
            let at = img.rel32_target(f + call);
            crate::s4_misc::start_object_at(g, img, at, w(obj), k(g, scale), index, owner, b);
        };
    g.write::<u32>(p + 0xc, one);
    start(g, (0x14c, 0x140, 0x134), (1, 0, 0));
    let (px, pz, pw) = (
        g.read::<u32>(p),
        g.read::<u32>(p + 8),
        g.read::<u32>(p + 0xc),
    );
    g.write::<u32>(at(0x20), px);
    g.write::<u32>(at(0x28), pz);
    g.write::<u32>(at(0x24), pw);
    // object 1's point and angle, its `y` 14 and then 150 lower
    let place = |g: &mut G, [x, z, wo, a]: [u32; 4]| {
        g.write::<u32>(w(x), px);
        g.write::<u32>(w(z), pz);
        g.write::<u32>(w(wo), pw);
        g.write::<f32>(w(a), yaw);
    };
    place(g, [0x166, 0x178, 0x18a, 0x197]);
    let y = sub(rd(g, p + 4), k(g, 0x1a4));
    g.write::<f32>(w(0x1ba), y);
    let y2 = sub(y, k(g, 0x1c2));
    g.write::<u32>(w(0x1cc), w(0x1d0));
    g.write::<f32>(at(0x1c), y2);
    g.write::<f32>(w(0x1dd), y2);
    start(g, (0x1e2, 0x1ac, 0x159), (0, 0x10, w(0x1b1)));
    place(g, [0x1ef, 0x1fc, 0x209, 0x216]);
    let y = sub(rd(g, p + 4), k(g, 0x223));
    for o in [0x22eu32, 0x238] {
        g.write::<u32>(w(o), 0);
    }
    g.write::<f32>(w(0x244), y);
    for o in [0x24au32, 0x254] {
        g.write::<u32>(w(o), 0);
    }
    g.write::<u32>(w(0x25e), w(0x262));
    g.write::<u32>(w(0x268), w(0x26c));
    for o in [0x272u32, 0x27c] {
        g.write::<u32>(w(o), 0);
    }
    g.write::<u16>(w(0x286), 0);
    g.write::<u32>(w(0x28c), w(0x290));
    let sl = Frame {
        at: img.rel32_target(f + 0x295),
        esp: at(0x38),
        ..Frame::default()
    };
    if crate::s4_misc::state_listed(g, img, &sl).eax == 0 {
        start(g, (0x2b5, 0x2b0, 0x2a5), (3, 0x10, 0));
        place(g, [0x2ca, 0x2d7, 0x2e4, 0x2f4]);
        let y = sub(rd(g, p + 4), k(g, 0x301));
        g.write::<f32>(w(0x315), y);
        start(g, (0x31a, 0x30d, 0x2c2), (4, 0xe, 0));
        place(g, [0x327, 0x334, 0x341, 0x34e]);
        let y = sub(rd(g, p + 4), k(g, 0x35b));
        g.write::<f32>(w(0x366), y);
    }
    // three more climbing from object 1's `y`
    let mut y = add(y2, k(g, 0x373));
    let mut obj = w(0x37d);
    for n in 0..3u32 {
        y = add(y, k(g, 0x394));
        g.write::<f32>(at(0x1c), y);
        let s = img.rel32_target(f + 0x3b5);
        crate::s4_misc::start_object_at(
            g,
            img,
            s,
            obj.wrapping_sub(0xd0),
            k(g, 0x39c),
            2,
            10 + 2 * n,
            0,
        );
        g.write::<f32>(obj.wrapping_sub(0xc), yaw);
        g.write::<u32>(obj, px);
        g.write::<u32>(obj + 8, pz);
        g.write::<f32>(obj + 4, y);
        g.write::<u32>(obj + 0xc, pw);
        obj = obj.wrapping_add(0x130);
    }
    g.write::<u32>(w(0x3f8), 0xe);
    let srn = Frame {
        at: img.rel32_target(f + 0x401),
        esp: at(0x38),
        ..Frame::default()
    };
    crate::o4_small::static_record_node(g, img, &srn);
    let mut free = g.read::<u16>(w(0x408));
    let fl = g.read::<u16>(w(0x40f)) | 3;
    g.write::<u16>(w(0x40f), fl);
    for o in [0x416u32, 0x41c, 0x422, 0x428] {
        g.write::<u32>(w(o), 0);
    }
    for (o, v) in [
        (0x42eu32, one),
        (0x438, one),
        (0x442, 0x4080_0000),
        (0x44c, 0x4080_0000),
        (0x456, 0x10),
        (0x460, one),
        (0x46a, one),
        (0x474, one),
        (0x47e, one),
        (0x488, 0x3f00_0000),
        (0x492, 0x3f00_0000),
        (0x49c, 0x3f00_0000),
        (0x4a6, one),
        (0x4b0, 8),
        (0x4ba, 8),
    ] {
        g.write::<u32>(w(o), v);
    }
    // the inlined takes off the free ring
    let take =
        |g: &mut G, free: &mut u16, head: u32, (to_next, to_0): (u32, u32), cb: u32, count: u32| {
            if *free == 0 {
                return 0;
            }
            let node = g.read::<u32>(w(head));
            let prev = g.read::<u32>(node);
            let next = g.read::<u32>(node + 4);
            g.write::<u32>(prev + 4, next);
            let next = g.read::<u32>(node + 4);
            let prev = g.read::<u32>(node);
            g.write::<u32>(next, prev);
            if *free > 1 {
                let next = g.read::<u32>(node + 4);
                g.write::<u32>(w(to_next), next);
            } else {
                g.write::<u32>(w(to_0), 0);
            }
            *free -= 1;
            for o in [0u32, 4, 0xc, 0x14] {
                g.write::<u32>(node + o, 0);
            }
            g.write::<u32>(node + 8, node);
            g.write::<u32>(node + 0x10, w(cb));
            g.write::<u8>(node + 0x18, 0);
            g.write::<u16>(w(count), *free);
            node
        };
    let n1 = take(g, &mut free, 0x4d8, (0x4fd, 0x4fd), 0x522, 0x52d);
    let first = g.read::<u32>(w(0x533));
    g.write::<u8>(n1 + 0x18, 1);
    g.write::<u32>(n1 + 0x14, w(0x53e));
    g.write::<u32>(n1 + 4, 0);
    let first = if first == 0 {
        g.write::<u32>(n1, 0);
        g.write::<u32>(w(0x563), n1);
        n1
    } else {
        let last = g.read::<u32>(w(0x54e));
        g.write::<u32>(n1, last);
        g.write::<u32>(last + 4, n1);
        first
    };
    g.write::<u32>(n1 + 8, n1);
    for (o, v) in [
        (0x56cu32, 0x3f00_0000u32),
        (0x576, one),
        (0x580, 0x3f00_0000),
        (0x58a, 0x3f00_0000),
        (0x594, 0x3f00_0000),
        (0x59e, one),
        (0x5a8, one),
        (0x5b2, one),
        (0x5bc, one),
        (0x5c6, one),
        (0x5d0, 0x37),
        (0x5da, 0),
        (0x5e4, 0x10),
        (0x5ee, 0x10),
    ] {
        g.write::<u32>(w(o), v);
    }
    // (the second take starts from the head the first one left: the
    // original keeps it in `edx`)
    let n2 = take(g, &mut free, 0x4fd, (0x618, 0x620), 0x651, 0x65c);
    g.write::<u8>(n2 + 0x18, 1);
    g.write::<u32>(n2 + 0x14, w(0x667));
    g.write::<u32>(n2 + 4, 0);
    if first == 0 {
        g.write::<u32>(n2, 0);
        g.write::<u32>(w(0x685), n2);
    } else {
        g.write::<u32>(n2, n1);
        g.write::<u32>(n1 + 4, n2);
    }
    g.write::<u32>(n2 + 8, n2);
    g.write::<u32>(w(0x69b), n2);
    // call width: 1
    g.call(slot(g, 271), 0, &[4]);
    // the sprite record (an unaligned word over the first byte's neighbours)
    let mut free = g.read::<u16>(w(0x6a4));
    g.write::<u32>(w(0x6ad), 0x8060_8080);
    g.write::<u8>(w(0x6b7), 0x80);
    g.write::<u16>(w(0x6bf), 0x8080);
    g.write::<u8>(w(0x6c7), 0);
    g.write::<u32>(w(0x6ce), 0x10);
    g.write::<u16>(w(0x6d9), 0x48);
    g.write::<u32>(w(0x6e1), 0x20);
    g.write::<u32>(w(0x6eb), 0x20);
    g.write::<u32>(w(0x6f5), 0);
    let append = |g: &mut G, node: u32, [first, last, set_first]: [u32; 3]| {
        let listed = g.read::<u32>(w(first)) != 0;
        g.write::<u32>(node + 4, 0);
        if listed {
            let tail = g.read::<u32>(w(last));
            g.write::<u32>(node, tail);
            g.write::<u32>(tail + 4, node);
        } else {
            g.write::<u32>(node, 0);
            g.write::<u32>(w(set_first), node);
        }
        g.write::<u32>(node + 8, node);
    };
    let n3 = take(g, &mut free, 0x70d, (0x72a, 0x732), 0x763, 0x76e);
    g.write::<u32>(n3 + 0x14, w(0x77c));
    append(g, n3, [0x774, 0x78a, 0x79d]);
    g.write::<u32>(w(0x7ac), n3);
    // the task start's word over a leftover one
    let sp = at(0x3c);
    g.write::<u32>(sp, w(0x7a3));
    let ts = Frame {
        at: img.rel32_target(f + 0x7b1),
        esp: sp.wrapping_sub(4),
        ..Frame::default()
    };
    crate::s3_misc::task_start(g, img, &ts);
    let mut free = g.read::<u16>(w(0x7b8));
    g.write::<u32>(w(0x7c1), 0);
    let n4 = take(g, &mut free, 0x7d4, (0x7f1, 0x7f9), 0x825, 0x830);
    g.write::<u8>(n4 + 0x18, 2);
    append(g, n4, [0x836, 0x84c, 0x85f]);
    g.write::<u32>(w(0x86d), n4);
    Ret::eax(0)
}

/// [`start_up_0163`] (the `rel32`s: the part set-up, the object starts
/// (seven calls), the state check, the static record node, the task
/// start, `__security_check_cookie`).
pub const START_UP_0163: Sig = Sig {
    len: 2172,
    hash: 0xe984_4131_1d15_d708,
    calls: &[
        (0x43, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x14c, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x1e2, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x295, Callee::Sig(&crate::s4_misc::STATE_LISTED)),
        (0x2b5, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x31a, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x3b5, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (
            0x401,
            Callee::Sig(&crate::o4_small::STATIC_RECORD_NODE_0163_2410),
        ),
        (0x7b1, Callee::Sig(&crate::s3_misc::TASK_START)),
        (0x874, Callee::Any),
    ],
    imports: &[],
};
