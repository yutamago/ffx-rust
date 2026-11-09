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
