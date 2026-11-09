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
