//! Round 6 shape families, second round (agent magic-families2), second
//! file: exact pairs, two DLLs' copies of one function that differ only
//! in their globals and immediates (families of
//! `~/ffx-work/m5/magic/r4/families.tsv`). Each port reads its operands
//! from the copy's code (`w(off)` = the word at `+off`).

use crate::o4_parts::disp8;
use crate::o6_fam_pairs5::{NodeOps, take_busy_node, wrap_turn};
use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// One effect object aimed at each target, with a sprite for the first
/// pass (no arguments; an aligned /GS frame: `ebp` = the entry `esp - 4`,
/// `E` = `(ebp & !7) - 0x68` after the saved registers; `magic_0565`
/// 0x100040d0 and `magic_0644` 0x100080f0, 1122 bytes; family
/// `F0565_100040d0`; [`crate::o6_fam_pairs5::target_objects`]'s
/// sibling). With `O = *G_O`, its target list `T = O + TL` (the count
/// `T + 4`, the ids bytes from `T + 8`): `h` = slot 219 `(*G_H, 0)`, slot
/// 237 `(h, 0, 0, &L)` (`L` = `E + 0x54`). For each target `k`, its
/// object `o = O + OB + 0x520 k` and record `R = o + 0x278`:
/// - `R - 0x18..` = `(L.x, L.y, L.z, K)`; the key `C[A[id]]`: slot 262
///   `(C, R - 8, R - 4, R)`, slot 268 `(C, 0x10, R - 8)`; `R + 4` = 1.0;
/// - `R + 8` = `atan2(dy, sqrt(dz dz + dx dx))` and `R + 0xc` = `K2 -
///   atan2(dz, dx)` wrapped once by a turn into `[KC, KB]` (`dy = R[-4] -
///   R[-0x14]`, `dx = R[-8] - R[-0x18]`, `dz = R[0] - R[-0x10]`; the
///   square root correctly rounded, the arcs the game's `_CIatan2`), `R +
///   0x10` = 0;
/// - `n` = the distance `/ K3` truncated, 1 when negative, into `R +
///   0x294` and `R + 0x298`;
/// - with the pass `O + P` 0 the sprite `R + 0x48` set up
///   ([`crate::sb_misc::sprite_init`] with the owner `*(*G_S + D)`) and
///   `R + 0x270` = 0x10, `R + 0x264` = 1.0, `R + 0x260` = 0.5, the byte
///   `R + 0x257` = 0x40;
/// - the object started ([`crate::s4_misc::start_object_at`] with the
///   scale `F[e]` and index `I[e]`, `e` = `O + P`), `R + 8..0x14` copied
///   to `R - 0x1b8..`, `R + 0x29c` = the id, `R + 0x2a0` = `k`, and a node
///   taken for the object ([`take_busy_node`], unmarked).
///
/// Then `O + P + 4` = the count, `O + P + 8` = 0; slot 257 `(slot 219
/// (*G_H, 0), 1)`, slot 38 `(slot 219 (*G_H, 0), 0, 1)`.
///
/// Each call sees the words the earlier ones left on the stack. Returns
/// slot 38's `eax`.
#[allow(clippy::too_many_lines)] // the original's calls, one by one
pub fn aimed_objects<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x33), n);
    let ebp = fr.esp.wrapping_sub(4);
    let e = (ebp & !7).wrapping_sub(0x68);
    let l = e.wrapping_add(0x54);
    let obj = g.read::<u32>(w(0x18));
    let targets = g.read::<u32>(obj.wrapping_add(w(0x1f)));
    let count = g.read::<u32>(targets.wrapping_add(4));
    let mut o = obj.wrapping_add(w(0x39));
    let mut st = Pushes::default();
    st.push_all(&[0, g.read::<u32>(w(0x2a))]);
    let t = slot(g, 219);
    // call width: 12 (Pushes caps the words at 12)
    let h = st.call(g, t, fr.ecx).eax;
    st.push_all(&[l, 0, 0, h]);
    let t = slot(g, 237);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t, fr.ecx);
    let kw = g.read::<u32>(w(0x6c));
    let mut r = o.wrapping_add(w(0x85));
    let node_ops = NodeOps {
        a: 0,
        t217: (0, 0),
        t234: (0, 0),
        t233: [(0, 0); 3],
        free: 0x2e0,
        k_a4: 0,
        k_9c: 0,
        head: 0x323,
        callback: 0x379,
        free_w: 0x384,
        last_cmp: 0x38a,
        last: 0x3a0,
        last_w: 0x3b3,
        first_w: 0x3cd,
    };
    let mut i = 0i32;
    while i < count.cast_signed() {
        let rf = |off: u32| r.wrapping_add(off);
        let rb = |off: u32| r.wrapping_sub(off);
        let list = g.read::<u32>(obj.wrapping_add(w(0x9c)));
        let id = u32::from(g.read::<u8>(list.wrapping_add(i.cast_unsigned()).wrapping_add(8)));
        for n in 0..3u32 {
            let x = g.read::<u32>(l.wrapping_add(4 * n));
            g.write::<u32>(rb(0x18 - 4 * n), x);
        }
        g.write::<u32>(rb(0xc), kw);
        // (`cmp ecx, -1` on a zero-extended byte: the id is never -1)
        let a = g.read::<u8>(g.read::<u32>(w(0xd6)).wrapping_add(id));
        let key = g.read::<u32>(w(0xe1).wrapping_add(4 * u32::from(a)));
        let mut st = Pushes::default();
        st.push_all(&[r, rb(4), rb(8), key]);
        let t = slot(g, 262);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, t, fr.ecx);
        st.push_all(&[rb(8), disp8(img, f + 0x10a), key]);
        let t = slot(g, 268);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, t, fr.ecx);
        g.write::<u32>(rf(4), w(0x11a));
        let at = |g: &G, off: u32| g.read::<f32>(rb(off));
        let dy = sub(at(g, 4), at(g, 0x14));
        let dx = sub(at(g, 8), at(g, 0x18));
        let dz = sub(at(g, 0), at(g, 0x10));
        let flat = add(mul(dz, dz), mul(dx, dx));
        let pitch = g.atan2(f64::from(dy), f64::from(flat).sqrt());
        #[allow(clippy::cast_possible_truncation)] // cvtpd2ps: round to f32
        g.write::<f32>(rf(8), pitch as f32);
        #[allow(clippy::cast_possible_truncation)] // cvtpd2ps: round to f32
        let yaw = g.atan2(f64::from(dz), f64::from(dx)) as f32;
        let turn = wrap_turn(sub(k(g, 0x1d0), yaw), k(g, 0x1df), k(g, 0x1f3), k(g, 0x1e9));
        g.write::<f32>(rf(0xc), turn);
        g.write::<u32>(rf(0x10), w(0x20c));
        let a = sub(at(g, 0x14), at(g, 4));
        let b = sub(at(g, 0x18), at(g, 8));
        let c = sub(at(g, 0x10), at(g, 0));
        let sq = add(add(mul(a, a), mul(b, b)), mul(c, c));
        #[allow(clippy::cast_possible_truncation)] // cvtsd2ss: round to f32
        let dist = f64::from(sq).sqrt() as f32;
        let n = crate::s4_summon::trunc(div(dist, k(g, 0x251)));
        let n = if n.cast_signed() < 0 { w(0x25e) } else { n };
        g.write::<u32>(rf(0x294), n);
        g.write::<u32>(rf(0x298), n);
        let pass_at = obj.wrapping_add(w(0x277));
        if g.read::<u32>(pass_at) == 0 {
            let owner = g.read::<u32>(g.read::<u32>(w(0x280)).wrapping_add(disp8(img, f + 0x289)));
            crate::sb_misc::sprite_init(g, rf(0x48), owner);
            for (off, v) in [(0x270u32, 0x299u32), (0x264, 0x2a3), (0x260, 0x2ad)] {
                g.write::<u32>(rf(off), w(v));
            }
            g.write::<u8>(rf(0x257), w(0x2b7) as u8);
        }
        let e_idx = g.read::<u32>(obj.wrapping_add(w(0x2ba)));
        let scale = g.read::<f32>(w(0x2c5).wrapping_add(4 * e_idx));
        let index = g.read::<u32>(w(0x2cf).wrapping_add(4 * e_idx));
        crate::s4_misc::start_object_at(g, img, img.rel32_target(f + 0x2d6), o, scale, index, 0, 0);
        let free = g.read::<u16>(w(0x2e0));
        for n in 0..4u32 {
            let x = g.read::<u32>(rf(8 + 4 * n));
            g.write::<u32>(rb(0x1b8 - 4 * n), x);
        }
        g.write::<u32>(rf(0x29c), id);
        g.write::<u32>(rf(0x2a0), i.cast_unsigned());
        take_busy_node(g, img, f, free, &node_ops, o, false);
        i += 1;
        o = o.wrapping_add(w(0x3bd));
        r = r.wrapping_add(w(0x3c4));
    }
    g.write::<u32>(obj.wrapping_add(w(0x3ea)), count);
    g.write::<u32>(obj.wrapping_add(w(0x3f5)), w(0x3f9));
    let mut st = Pushes::default();
    st.push_all(&[0, g.read::<u32>(w(0x407))]);
    let t = slot(g, 219);
    // call width: 12 (Pushes caps the words at 12)
    let h = st.call(g, t, fr.ecx).eax;
    st.push_all(&[disp8(img, f + 0x40e), h]);
    let t = slot(g, 257);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t, fr.ecx);
    st.push_all(&[0, g.read::<u32>(w(0x426))]);
    let t = slot(g, 219);
    // call width: 12 (Pushes caps the words at 12)
    let h2 = st.call(g, t, fr.ecx).eax;
    let mut st = Pushes::default();
    st.push_all(&[disp8(img, f + 0x436), w(0x43b), h2]);
    let t = slot(g, 38);
    // call width: 12 (Pushes caps the words at 12)
    Ret::eax(st.call(g, t, fr.ecx).eax)
}

/// One coordinate of a cubic Bezier curve at `u` (`s = 1 - u` as `K1 -
/// u`), in the original's order: `s u s K3 b + s s s a`, then `+ u u s K3
/// c`, then `+ u u u d` (one f32 rounding per instruction).
fn bezier(u: f32, s: f32, k3: f32, [a, b, c, d]: [f32; 4]) -> f32 {
    use crate::sb_sse::{add, mul};
    let x = add(
        mul(mul(mul(mul(s, u), s), k3), b),
        mul(mul(mul(s, s), s), a),
    );
    let uu = mul(u, u);
    let x = add(x, mul(mul(mul(uu, s), k3), c));
    add(x, mul(mul(uu, u), d))
}

/// Two curve flyers, each frame (`(task)`; a /GS frame of 0x28 bytes,
/// `ebp` = `esp - 4` at the entry; `magic_0307` 0x10002be0, 1041 bytes,
/// and `magic_0563` 0x100026d0, 1042 bytes (its owner word `*(*G_R +
/// 0x14)` instead of `**G_R` makes every later operand one byte later);
/// family `F0307_10002be0`). For both records `R = *G_O + D + 0x190 i`:
/// - the time `R.180 += R.184`, past `K1` it is 1.0; the point `P`
///   (`ebp - 0x14`, `w` 1.0) is the cubic Bezier ([`bezier`]) of the
///   control points `R + 0x130`, `+0x140`, `+0x150`, `+0x160` at that
///   time, also into `R + 0xd0..` (`w` 1.0);
/// - the countdown `R + 0x188` drops; at 0 the record leaves the list `L`
///   ([`crate::l5_objhost::list_remove`]; when it was there `*G_P =
///   ITEM`, slot 713 `(R)` and bit 15 of `R + 0x124` cleared) and the
///   task ends after the loop;
/// - four particles from the pool ([`crate::sb_misc::pool_take`]; none
///   left ends the record's): the halfword `+0x3a` = 0x200, `+0x30` the
///   owner word, `+0x34` = 1, the halfword `+0x38` = 0xffff, `+0..` =
///   `P`; `+0`, `+4`, `+8` plus slot 314 `(8.0)` each; `+0x10..+0x18`
///   slot 314 `(1.0)`, `+0x1c` slot 314 `(pi)`; `+0x20` 0, `+0x24` -0.1,
///   `+0x28` 0, `+0x2c` 1.0; `+0x3c = (slot 314 (0.2) + K1) * K4`; `+0x40`
///   0, `+0x58` 0x10, `+0x5c` 0, `+0x4c` 0x10, `+0x50` 0, `+0x60` 0 (the
///   slots' ST0 as f32; the immediates the copy's).
///
/// When a record ended, the task goes back to the pool
/// ([`crate::sa_pool::give_back`]). Each call sees the words the earlier
/// ones left on the stack. Returns the last call's `eax` (0 when the pool
/// ran out last).
#[allow(clippy::too_many_lines)] // the original's calls, one by one
pub fn curve_flyers<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::ob_model::st0_f32;
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    // `mov ecx, [ecx]` (8b 09) or `mov ecx, [ecx + 0x14]` (8b 49 14) at +0x24a
    let late = u32::from(img.word(f + 0x24b) as u8 == 0x49);
    let w = |o: u32| img.word(f + o);
    // an operand at or after +0x24c, in the copy's place
    let wl = |o: u32| img.word(f + o + late);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x200), n);
    let task = fr.arg(g, 0);
    let ebp = fr.esp.wrapping_sub(4);
    let p = ebp.wrapping_sub(0x14);
    let (k1, k3) = (k(g, 0x17), k(g, 0x1f));
    let mut r = g.read::<u32>(w(0x27)).wrapping_add(w(0x2d));
    let mut gone = false;
    let mut eax = 0;
    g.write::<u32>(p.wrapping_add(0xc), w(0x40));
    let mut left = w(0x47);
    loop {
        let rf = |o: u32| r.wrapping_add(o);
        let rd = |g: &G, o: u32| g.read::<f32>(rf(o));
        let t = add(rd(g, 0x184), rd(g, 0x180));
        g.write::<f32>(rf(0x180), t);
        if t > k1 {
            g.write::<u32>(rf(0x180), w(0x73));
        }
        let u = rd(g, 0x180);
        let s = sub(k1, u);
        let mut xyz = [0.0f32; 3];
        for (c, x) in xyz.iter_mut().enumerate() {
            let c = 4 * c as u32;
            let pts = [
                rd(g, 0x130 + c),
                rd(g, 0x140 + c),
                rd(g, 0x150 + c),
                rd(g, 0x160 + c),
            ];
            *x = bezier(u, s, k3, pts);
        }
        for (i, x) in (0u32..).zip(xyz) {
            g.write::<f32>(p.wrapping_add(4 * i), x);
        }
        g.write::<f32>(rf(0xd0), xyz[0]);
        g.write::<f32>(rf(0xd4), xyz[1]);
        g.write::<u32>(rf(0xdc), w(0x1c4));
        g.write::<f32>(rf(0xd8), xyz[2]);
        let n = g.read::<u32>(rf(0x188));
        if n == 0 {
            let list = w(0x1e6);
            if crate::l5_objhost::list_remove(g, img, img.rel32_target(f + 0x1eb), list, r) != 0 {
                let gp = g.read::<u32>(w(0x1f4));
                g.write::<u32>(gp, w(0x1fb));
                let t = slot(g, 713);
                // call width: 1
                g.call(t, list, &[r]);
                let v = g.read::<u16>(rf(0x124)) & w(0x4c) as u16;
                g.write::<u16>(rf(0x124), v);
            }
            gone = true;
        } else {
            g.write::<u32>(rf(0x188), n.wrapping_sub(1));
        }
        let take = img.rel32_target(f + 0x231);
        let mut j = 0i32;
        while j < disp8(img, f + 0x3bf + late).cast_signed() {
            let q = crate::sb_misc::pool_take(g, img, take);
            eax = q;
            if q == 0 {
                break;
            }
            let qf = |o: u32| q.wrapping_add(o);
            let owner_at = g.read::<u32>(w(0x241));
            let owner_at = if late == 1 {
                owner_at.wrapping_add(disp8(img, f + 0x24c))
            } else {
                owner_at
            };
            let owner = g.read::<u32>(owner_at);
            g.write::<u16>(qf(0x3a), wl(0x24d) as u16);
            g.write::<u32>(qf(0x30), owner);
            g.write::<u32>(qf(0x34), wl(0x260));
            g.write::<u16>(qf(0x38), 0xffff);
            for i in 0..4u32 {
                let x = g.read::<u32>(p.wrapping_add(4 * i));
                g.write::<u32>(qf(4 * i), x);
            }
            let mut st = Pushes::default();
            st.push(owner);
            let st0 = |g: &mut G, st: &mut Pushes, x: u32| {
                st.set_top(x);
                let t = slot(g, 314);
                // call width: 12 (Pushes caps the words at 12)
                let out = st.call(g, t, fr.ecx);
                (out.eax, st0_f32(out.st0))
            };
            let (_, t) = st0(g, &mut st, wl(0x280));
            g.write::<f32>(qf(0), add(t, g.read::<f32>(qf(0))));
            for (o, x) in [(4u32, 0x29au32), (8, 0x2bf)] {
                let (_, t) = st0(g, &mut st, wl(x));
                g.write::<f32>(qf(o), add(g.read::<f32>(qf(o)), t));
            }
            for (o, x) in [
                (0x10u32, 0x2e5u32),
                (0x14, 0x301),
                (0x18, 0x318),
                (0x1c, 0x32f),
            ] {
                let (_, t) = st0(g, &mut st, wl(x));
                g.write::<f32>(qf(o), t);
            }
            for (o, x) in [
                (0x20u32, 0x346u32),
                (0x24, 0x34d),
                (0x28, 0x354),
                (0x2c, 0x35b),
            ] {
                g.write::<u32>(qf(o), wl(x));
            }
            let (e, t) = st0(g, &mut st, wl(0x368));
            eax = e;
            let x = mul(add(t, g.read::<f32>(wl(0x37a))), g.read::<f32>(wl(0x398)));
            for (o, v) in [
                (0x40u32, 0x389u32),
                (0x58, 0x390),
                (0x5c, 0x39f),
                (0x4c, 0x3a6),
            ] {
                g.write::<u32>(qf(o), wl(v));
            }
            g.write::<f32>(qf(0x3c), x);
            for (o, v) in [(0x50u32, 0x3b2u32), (0x60, 0x3b9)] {
                g.write::<u32>(qf(o), wl(v));
            }
            j += 1;
        }
        r = r.wrapping_add(wl(0x3dd));
        left = left.wrapping_sub(1);
        if left == 0 {
            break;
        }
    }
    if gone {
        eax = crate::sa_pool::give_back(g, img, img.rel32_target(f + 0x3ff + late), task);
    }
    Ret::eax(eax)
}

/// The actors' spawner, each frame (`(task)`; an aligned /GS frame: `ebp`
/// = the entry `esp - 4`, `E` = `(ebp & !7) - 0x58` after the saved
/// registers; `magic_0565` 0x10005bd0 and `magic_0644` 0x10009a00, 1288
/// bytes; family `F0565_10005bd0`). With `O = *G_O` and `A = O + D`:
/// while the wait `A + 0x5200` is above 0 it counts down (returns it).
/// Else the next actor `i = A + 0x5204`, its record `R = A + 0x520 i`:
/// - the start `V` (`E + 0x34`): with the mode `A + 0x5208` 0 the point
///   `O + P..` and, turned by `(0, O + F)` ([`crate::s4_small::turn_point`]
///   on `(slot 314 (5.0), slot 314 (2.0) - K1, 0, 1)`), moved; else slot
///   237 `(slot 219 (*G_H, 0), 0, 0, &V)`, `w` 1.0; `V` into `R + 0x260..`;
/// - the target `Q = R + 0x270` from slots 262 `(C, Q, Q + 4, Q + 8)` and
///   268 `(C, 0x10, Q)` (`C` the id's key, as in
///   [`crate::o6_fam_pairs5::target_objects`]), `w` 1.0; with the mode 0
///   each of `x, y, z` plus slot 314 `(4.0)`;
/// - `R + 0x280` = `atan2(dy, sqrt(dz dz + dx dx))`, `R + 0x284` = `K2 -
///   atan2(dz, dx)` wrapped once by a turn, `R + 0x288` = 0 (`d = Q -
///   V`); the countdown `R + 0x50c` and `+0x510` = `1 - |V - Q| / K3`
///   truncated (1 when negative); `R + 0x514` = the id;
/// - a node with the callback `CB`
///   ([`crate::o6_fam_pairs5::actor_light`]'s address; [`crate::sa_pool::take`])
///   appended to the busy list with `R`;
/// - the wait = `slot 324 () % 8 + 4` (`idiv`'s sign), `i` up by one; with
///   all `A + 0x520c` spawned `O + T` = `CB2`, `O + T + 0x1c` = 0x3c and
///   the task's byte `+0x56` = 1 (returns the task), else returns `i`.
///
/// Each call sees the words the earlier ones left on the stack.
#[allow(clippy::too_many_lines)] // the original's calls, one by one
pub fn actor_spawner<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::ob_model::st0_f32;
    use crate::sb_sse::{add, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x91), n);
    let task = fr.arg(g, 0);
    let ebp = fr.esp.wrapping_sub(4);
    let e = (ebp & !7).wrapping_sub(0x58);
    let at = |o: u32| e.wrapping_add(o);
    let obj = g.read::<u32>(w(0x1a));
    let a = obj.wrapping_add(w(0x22));
    let af = |o: u32| a.wrapping_add(o);
    let wait = g.read::<u32>(af(0x5200));
    if wait.cast_signed() > 0 {
        let wait = wait.wrapping_sub(1);
        g.write::<u32>(af(0x5200), wait);
        return Ret::eax(wait);
    }
    let v = at(0x34);
    if g.read::<u32>(af(0x5208)) == 0 {
        for (i, o) in (0u32..).zip([0x5au32, 0x68, 0x76, 0x84]) {
            let x = g.read::<u32>(obj.wrapping_add(w(o)));
            g.write::<u32>(v.wrapping_add(4 * i), x);
        }
    } else {
        let mut st = Pushes::default();
        st.push_all(&[0, g.read::<u32>(w(0x99))]);
        let t = slot(g, 219);
        // call width: 12 (Pushes caps the words at 12)
        let h = st.call(g, t, fr.ecx).eax;
        st.push_all(&[v, 0, 0, h]);
        let t = slot(g, 237);
        // call width: 12 (Pushes caps the words at 12)
        st.call(g, t, fr.ecx);
        g.write::<u32>(v.wrapping_add(0xc), w(0xc3));
    }
    let i = g.read::<u32>(af(0x5204));
    let turnf = g.read::<u32>(obj.wrapping_add(w(0xd1)));
    let r = a.wrapping_add(i.wrapping_mul(w(0xd7)));
    let rf = |o: u32| r.wrapping_add(o);
    let (wv, z) = (at(0x10), at(0x44));
    for (i, x) in (0u32..).zip([w(0xe8), turnf, w(0xf6), w(0xfe)]) {
        g.write::<u32>(wv.wrapping_add(4 * i), x);
    }
    for (i, o) in (0u32..).zip([0x106u32, 0x10e, 0x116, 0x11e]) {
        g.write::<u32>(z.wrapping_add(4 * i), w(o));
    }
    let st0 = |g: &mut G, st: &mut Pushes, x: u32| {
        st.set_top(x);
        let t = slot(g, 314);
        // call width: 12 (Pushes caps the words at 12)
        st0_f32(st.call(g, t, fr.ecx).st0)
    };
    if g.read::<u32>(af(0x5208)) == 0 {
        let mut st = Pushes::default();
        st.push(0);
        let t1 = st0(g, &mut st, w(0x133));
        g.write::<f32>(z, t1);
        let t2 = st0(g, &mut st, w(0x145));
        g.write::<f32>(z.wrapping_add(4), sub(t2, k(g, 0x15f)));
        // push &Z; the callee's return address at `E - 0xc`
        let esp = e.wrapping_sub(0xc);
        g.write::<u32>(esp.wrapping_add(4), z);
        let turn = Frame {
            at: img.rel32_target(f + 0x177),
            esp,
            ecx: wv,
            edx: v,
            ..Frame::default()
        };
        crate::s4_small::turn_point(g, img, &turn);
    }
    for i in 0..4u32 {
        let x = g.read::<u32>(v.wrapping_add(4 * i));
        g.write::<u32>(rf(0x260 + 4 * i), x);
    }
    let list = g.read::<u32>(obj.wrapping_add(w(0x1b8)));
    let id = u32::from(g.read::<u8>(list.wrapping_add(g.read::<u32>(af(0x5204))).wrapping_add(8)));
    let q = rf(0x270);
    // (`cmp edx, -1` on a zero-extended byte: the id is never -1)
    let b = g.read::<u8>(g.read::<u32>(w(0x1de)).wrapping_add(id));
    let key = g.read::<u32>(w(0x1e9).wrapping_add(4 * u32::from(b)));
    let mut st = Pushes::default();
    st.push_all(&[q.wrapping_add(8), q.wrapping_add(4), q, key]);
    let t = slot(g, 262);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t, fr.ecx);
    st.push_all(&[q, disp8(img, f + 0x21c), key]);
    let t = slot(g, 268);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, t, fr.ecx);
    g.write::<u32>(q.wrapping_add(0xc), w(0x23b));
    if g.read::<u32>(af(0x5208)) == 0 {
        let mut st = Pushes::default();
        st.push(0);
        for (o, x) in [(0x270u32, 0x25bu32), (0x274, 0x27b), (0x278, 0x2a9)] {
            let t = st0(g, &mut st, w(x));
            g.write::<f32>(rf(o), add(t, g.read::<f32>(rf(o))));
        }
    }
    let rd = |g: &G, o: u32| g.read::<f32>(rf(o));
    let dx = sub(rd(g, 0x270), rd(g, 0x260));
    let dy = sub(rd(g, 0x274), rd(g, 0x264));
    let dz = sub(rd(g, 0x278), rd(g, 0x268));
    let flat = add(mul(dz, dz), mul(dx, dx));
    let pitch = g.atan2(f64::from(dy), f64::from(flat).sqrt());
    #[allow(clippy::cast_possible_truncation)] // cvtpd2ps: round to f32
    g.write::<f32>(rf(0x280), pitch as f32);
    #[allow(clippy::cast_possible_truncation)] // cvtpd2ps: round to f32
    let yaw = g.atan2(f64::from(dz), f64::from(dx)) as f32;
    let turn = wrap_turn(sub(k(g, 0x39e), yaw), k(g, 0x3ad), k(g, 0x3c1), k(g, 0x3b7));
    g.write::<f32>(rf(0x284), turn);
    g.write::<u32>(rf(0x288), w(0x3e0));
    let a2 = sub(rd(g, 0x264), rd(g, 0x274));
    let b2 = sub(rd(g, 0x260), rd(g, 0x270));
    let c2 = sub(rd(g, 0x268), rd(g, 0x278));
    let sq = add(add(mul(a2, a2), mul(b2, b2)), mul(c2, c2));
    #[allow(clippy::cast_possible_truncation)] // cvtsd2ss: round to f32
    let dist = f64::from(sq).sqrt() as f32;
    let n = crate::s4_summon::trunc(div(dist, k(g, 0x43d)));
    let n = w(0x435).wrapping_sub(n);
    let n = if n.cast_signed() < 0 { w(0x44a) } else { n };
    g.write::<u32>(rf(0x50c), n);
    g.write::<u32>(rf(0x510), n);
    g.write::<u32>(rf(0x514), id);
    let node = crate::sa_pool::take(g, img, img.rel32_target(f + 0x46d), w(0x462));
    let first = g.read::<u32>(w(0x473));
    g.write::<u32>(node.wrapping_add(0x14), r);
    g.write::<u32>(node.wrapping_add(4), 0);
    if first != 0 {
        let tail = g.read::<u32>(w(0x486));
        g.write::<u32>(node, tail);
        g.write::<u32>(tail.wrapping_add(4), node);
    } else {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x498), node);
    }
    g.write::<u32>(node.wrapping_add(8), node);
    g.write::<u32>(w(0x4a0), node);
    let t = slot(g, 324);
    // call width: 0
    let roll = g.call(t, fr.ecx, &[]).eax.cast_signed();
    let next = g.read::<u32>(af(0x5204)).wrapping_add(1);
    g.write::<u32>(af(0x5204), next);
    let wait = roll
        .wrapping_rem(8)
        .wrapping_add(disp8(img, f + 0x4c5).cast_signed());
    g.write::<u32>(af(0x5200), wait.cast_unsigned());
    let next = g.read::<u32>(af(0x5204));
    if next.cast_signed() < g.read::<u32>(af(0x520c)).cast_signed() {
        return Ret::eax(next);
    }
    g.write::<u32>(obj.wrapping_add(w(0x4e0)), w(0x4e4));
    g.write::<u32>(obj.wrapping_add(w(0x4ea)), w(0x4ee));
    g.write::<u8>(task.wrapping_add(disp8(img, f + 0x4f4)), w(0x4f5) as u8);
    Ret::eax(task)
}

/// A node `n` with an empty child list put last among the children of
/// `p` (`+0x1a` = 0xff as a u16, `+0x10`/`+0xc` itself, `+0x14`/`+4` the
/// parent, `+8` the parent's last, the parent's count `+0x18` up, its last
/// `+0x10` = `n`; the first `+0xc` when it had none), as the original
/// inlines it; `n[0]` = `vtable` after.
fn append_child<G: Game>(g: &mut G, n: u32, p: u32, vtable: u32) {
    g.write::<u16>(n.wrapping_add(0x1a), 0xff);
    g.write::<u32>(n.wrapping_add(0x10), n);
    g.write::<u32>(n.wrapping_add(0xc), n);
    g.write::<u32>(n.wrapping_add(0x14), p);
    g.write::<u32>(n.wrapping_add(4), p);
    let last = g.read::<u32>(p.wrapping_add(0x10));
    g.write::<u32>(n.wrapping_add(8), last);
    let last = g.read::<u32>(p.wrapping_add(0x10));
    if last == p {
        g.write::<u32>(p.wrapping_add(0xc), n);
    } else {
        g.write::<u32>(last.wrapping_add(4), n);
    }
    let count = g.read::<u16>(p.wrapping_add(0x18)).wrapping_add(1);
    g.write::<u16>(p.wrapping_add(0x18), count);
    g.write::<u32>(p.wrapping_add(0x10), n);
    g.write::<u32>(n, vtable);
}

/// A staged effect's task (`(task)` with the state record `S =
/// task[+4]`; an aligned frame: `ebp` = the entry `esp - 4`, `E` = `(ebp &
/// !7) - 0x38` after the saved registers; `magic_0398` and `magic_0402`
/// 0x10006960, 1103 bytes; family `F0398_10006960`). Each frame `S.1c`
/// and `S.20` grow by `K0` and the counter `S + 0x28` by one; by the
/// state `S + 0x18`:
/// - 0: the model nodes ([`crate::s3_summon::model_nodes`], a record
///   on the frame), the colour node ([`crate::s3_misc::colour_node`]),
///   seven light words 0.5 and the camera move
///   ([`crate::sb_misc::move_start`], `K1`); then as 10;
/// - 10: the state 11, the counter 100, the index `S + 0x24` = 0; then as
///   11;
/// - 11: past the counter `N` the index's nodes: two children of the
///   parent `P` (the first with slot 268 `(T[i].54, 0x11, N1 + 0x1c)` and
///   its scale 1.0, the second with the scale `(1, 1, 1, A)`), the owned
///   node ([`crate::nodes::node_insert`]), its particle node
///   ([`crate::sb_nodes::particle_node`]) and effect node
///   ([`crate::sb_nodes::effect_node`]) on records on the frame; the
///   counter 0 and the state 12;
/// - 12: past 20, slot 239 `(*G_X, T[i], 0)` and the next index; with all
///   `*G_N` done the state 13, else back to 11;
/// - 13: past 80, seven light words 1.0, the camera's second move
///   ([`crate::sb_misc::move_start_2`]) and the state 10000;
/// - 10000: the task leaves its list.
///
/// Returns `eax` as the original leaves it (the state, the index, a
/// callee's or the task's old link, by the way taken).
#[allow(clippy::too_many_lines)] // the original's states, one by one
pub fn staged_nodes<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::add;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x231), n);
    let task = fr.arg(g, 0);
    let s = g.read::<u32>(task.wrapping_add(4));
    let sf = |o: u32| s.wrapping_add(o);
    let ebp = fr.esp.wrapping_sub(4);
    let e = (ebp & !7).wrapping_sub(0x38);
    let at = |o: u32| e.wrapping_add(o);
    let bump = |g: &mut G, o: u32, by: u32| {
        let v = g.read::<u32>(sf(o)).wrapping_add(by);
        g.write::<u32>(sf(o), v);
    };
    let x = add(g.read::<f32>(sf(0x1c)), k(g, 0x1b));
    bump(g, 0x28, 1);
    let state = g.read::<u32>(sf(0x18));
    let counter = g.read::<u32>(sf(0x28));
    g.write::<f32>(sf(0x1c), x);
    let y = add(g.read::<f32>(sf(0x20)), k(g, 0x36));
    g.write::<f32>(sf(0x20), y);
    // the common way out: the counter up once more
    let out = |g: &mut G, eax: u32| {
        bump(g, 0x28, 1);
        Ret::eax(eax)
    };
    let lights = |g: &mut G, ops: [u32; 7]| {
        for o in ops {
            g.write::<u32>(w(o + 2), w(o + 6));
        }
    };
    let spawn_from = match state {
        0 => {
            let rec = at(0x20);
            for (i, o) in (0u32..).zip([0x63u32, 0x6b]) {
                g.write::<u32>(rec.wrapping_add(4 * i), w(o));
            }
            let c = g.read::<u32>(w(0x57));
            g.write::<u32>(rec.wrapping_add(8), c);
            g.write::<u32>(rec.wrapping_add(0xc), 0);
            g.write::<u8>(rec.wrapping_add(0x10), 0);
            g.write::<u32>(w(0x82), w(0x86));
            let callee = Frame {
                at: img.rel32_target(f + 0x8b),
                esp: e.wrapping_sub(4),
                ecx: rec,
                ..Frame::default()
            };
            crate::s3_summon::model_nodes(g, img, &callee);
            let rec = at(0x18);
            g.write::<u32>(rec, w(0x97));
            g.write::<u32>(rec.wrapping_add(4), w(0x9f));
            g.write::<u32>(w(0xa5), w(0xa9));
            let callee = Frame {
                at: img.rel32_target(f + 0xae),
                esp: e.wrapping_sub(4),
                ecx: rec,
                ..Frame::default()
            };
            let eax = crate::s3_misc::colour_node(g, img, &callee);
            let t = k(g, 0xb6);
            lights(g, [0xba, 0xc4, 0xce, 0xd8, 0xe2, 0xec, 0xf6]);
            crate::sb_misc::move_start(g, img, img.rel32_target(f + 0x101), t, eax);
            g.write::<u32>(sf(0x18), w(0x108));
            true
        }
        10 => true,
        11 => counter.cast_signed() > disp8(img, f + 0x120).cast_signed(),
        12 => {
            if counter.cast_signed() <= disp8(img, f + 0x1b7).cast_signed() {
                return out(g, 0);
            }
            let i = g.read::<u32>(sf(0x24));
            let key = g.read::<u32>(i.wrapping_mul(w(0x1c4)).wrapping_add(w(0x1ca)));
            let gx = g.read::<u32>(w(0x1d5));
            let t = slot(g, 239);
            // call width: 3
            g.call(t, fr.ecx, &[gx, key, 0]);
            bump(g, 0x24, 1);
            let i = g.read::<u32>(sf(0x24));
            if i.cast_signed() >= g.read::<u32>(w(0x1ec)).cast_signed() {
                bump(g, 0x18, 1);
            } else {
                bump(g, 0x18, 0u32.wrapping_sub(1));
            }
            return out(g, i);
        }
        13 => {
            if counter.cast_signed() <= disp8(img, f + 0x3b1).cast_signed() {
                return out(g, 13);
            }
            let (t3, t2) = (k(g, 0x3bc), k(g, 0x3c4));
            lights(g, [0x3c8, 0x3d2, 0x3dc, 0x3e6, 0x3f0, 0x3fa, 0x404]);
            let eax = crate::sb_misc::move_start_2(g, img, img.rel32_target(f + 0x40f), t2, t3);
            bump(g, 0x28, 1);
            g.write::<u32>(sf(0x18), w(0x419));
            return Ret::eax(eax);
        }
        0x2710 => {
            let node = g.read::<u32>(task);
            let prev = g.read::<u32>(node.wrapping_add(4));
            let next = g.read::<u32>(node.wrapping_add(8));
            g.write::<u32>(prev.wrapping_add(8), next);
            let next = g.read::<u32>(node.wrapping_add(8));
            let eax = g.read::<u32>(node.wrapping_add(4));
            g.write::<u32>(next.wrapping_add(4), eax);
            g.write::<u32>(node.wrapping_add(8), node);
            g.write::<u32>(node.wrapping_add(4), node);
            return out(g, eax);
        }
        _ => return out(g, state),
    };
    if state == 0 || state == 10 {
        bump(g, 0x18, 1);
        g.write::<u32>(sf(0x28), w(0x112));
        g.write::<u32>(sf(0x24), w(0x119));
        let counter = g.read::<u32>(sf(0x28));
        if counter.cast_signed() <= disp8(img, f + 0x120).cast_signed() {
            return out(g, state);
        }
    } else if !spawn_from {
        return out(g, 0);
    }
    // state 11: the index's nodes
    let i = g.read::<u32>(sf(0x24));
    let owned = i.wrapping_mul(w(0x12e)).wrapping_add(w(0x133));
    let e2c = i.wrapping_mul(disp8(img, f + 0x13f));
    let n1 = e2c.wrapping_add(w(0x14b));
    let n2 = e2c.wrapping_add(w(0x151));
    let parent = w(0x162);
    let particle = (i << 8).wrapping_add(w(0x17f));
    append_child(g, n1, parent, w(0x215));
    let i = g.read::<u32>(sf(0x24));
    let entry = i.wrapping_mul(w(0x21e)).wrapping_add(w(0x223));
    let key = g.read::<u32>(entry.wrapping_add(disp8(img, f + 0x22f)));
    let t = slot(g, 268);
    // call width: 3
    g.call(
        t,
        fr.ecx,
        &[key, disp8(img, f + 0x22c), n1.wrapping_add(0x1c)],
    );
    g.write::<u32>(n1.wrapping_add(0x28), w(0x244));
    g.write::<u32>(n1.wrapping_add(0x28), w(0x24b));
    append_child(g, n2, parent, w(0x29e));
    for (o, v) in [
        (0x1cu32, 0x2a5u32),
        (0x20, 0x2ac),
        (0x24, 0x2b3),
        (0x28, 0x2ba),
    ] {
        g.write::<u32>(n2.wrapping_add(o), w(v));
    }
    let rec = at(0x20);
    let insert = [owned, w(0x2ce), w(0x2d6), n1, w(0x2e2), n2];
    for (i, v) in (0u32..).zip(insert) {
        g.write::<u32>(rec.wrapping_add(4 * i), v);
    }
    let callee = Frame {
        at: img.rel32_target(f + 0x2e7),
        esp: e.wrapping_sub(4),
        ecx: rec,
        ..Frame::default()
    };
    crate::nodes::node_insert(g, img, &callee);
    let gy = g.read::<u32>(w(0x2f0));
    g.write::<u8>(owned.wrapping_add(0x148), w(0x2fa) as u8);
    g.write::<u32>(particle, w(0x301));
    for (i, v) in (0u32..).zip([particle, w(0x30d), owned, gy, w(0x31d), w(0x325)]) {
        g.write::<u32>(rec.wrapping_add(4 * i), v);
    }
    crate::sb_nodes::particle_node(g, img, img.rel32_target(f + 0x32a), rec);
    let i = g.read::<u32>(sf(0x24));
    let effect = i.wrapping_mul(0x38).wrapping_add(w(0x349));
    for (i, v) in (0u32..).zip([effect, w(0x335), owned, w(0x359), w(0x361), w(0x369)]) {
        g.write::<u32>(rec.wrapping_add(4 * i), v);
    }
    crate::sb_nodes::effect_node(g, img, img.rel32_target(f + 0x36e), rec);
    let made = g.read::<u32>(rec);
    g.write::<u32>(made, w(0x378));
    let i = g.read::<u32>(sf(0x24));
    let flags_at = i.wrapping_mul(0x38).wrapping_add(w(0x38b));
    let v = g.read::<u32>(flags_at) | 1;
    g.write::<u32>(flags_at, v);
    g.write::<u32>(i.wrapping_mul(0x38).wrapping_add(w(0x393)), w(0x397));
    g.write::<u32>(sf(0x28), w(0x39e));
    bump(g, 0x18, 1);
    out(g, i)
}

/// A copy of [`trail_draw`]: its frame's locals (offsets from `E`) and the
/// operands after `+0x338`, where the copies' code differs.
struct TrailOps {
    acc: u32,
    seg: u32,
    idx: u32,
    seed: u32,
    left: u32,
    p: u32,
    d: u32,
    scale: u32,
    k1: u32,
    k2: u32,
    k3: u32,
    k4: u32,
    alpha: u32,
    rec: u32,
    call: u32,
    /// `link_part` with `(G_REC, k)` (`0544`) instead of `draw_frame`
    /// with `(G_REC)` (`0446`).
    link: bool,
}

/// A trail of up to 32 points drawn as sprites along it (internal: `ecx`
/// = the trail `o`, `edx` = a 3x3 basis `B`, stack `(point)`; an aligned
/// /GS frame: `ebp` = the entry `esp - 4`, `E` = `(ebp & !0xf) - 0x170`
/// after the saved registers; `magic_0446` 0x10004e20, 1413 bytes, and
/// `magic_0544` 0x10004ce0, 1436 bytes, which lays its frame out
/// differently and links each sprite as a part
/// ([`crate::sa_misc::link_part`]) where `0446` draws it
/// ([`crate::sb_misc::draw_frame`]); family `F0446_10004e20`):
/// - slot 723 `(M1)`, 724 `(M2)`; `*G_M3`, `*G_M4` = the frame's `M3`,
///   `M4`; `point` into the ring `o + 0xc + 0x10 h` (`h = o + 0x224`,
///   marked in the bytes `o + 0x22c..`, then `h + 1` modulo 32); the
///   animation stepped ([`crate::sb_common::anim_step`]);
/// - for each of the `o + 0x228` sprites `k`: the distance `acc` along
///   the trail (from 0, `+ o.21c` per sprite) walks back one segment (to
///   the previous marked point, its length `|D|`, `D` = slot 234 of the
///   two points) whenever it passed the current one; an unmarked point,
///   32 segments, or a zero-length segment ends it (the last skips the
///   sprite); the seed `(seed * 0x80d + 7) & 0xffff`; the colour `G_C`
///   blends `o + 0x20c` and `+0x210` by `k / count`; the place `Q` = the
///   point, or `P + acc / |D| * D` (slot 248); the scale `(1 - s) o.214 +
///   s o.218` (`s = (k + 1) / count`) times `B` into `M3` (slot 235),
///   turned by `(seed - 0x7fff) * K2 * K3` (slot 330), placed by slot 229
///   `(M3 + 0x30, M2, Q)`, copied to `M4` (slot 230); `M4.tz -= K4`; with
///   the colour's alpha set, the sprite of frame `o + 8 + seed % n` (`n`
///   the slot 941 table's count, wrapped) is drawn.
///
/// Returns 0x1f, the count left when it ran out, or the animation step's
/// result without sprites.
#[allow(clippy::too_many_lines, clippy::float_cmp)] // the original's loop; it compares exactly
pub fn trail_draw<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, cvt, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x25), n);
    // `inc eax` (0446) or `lea eax, [esi + 1]` (0544) at +0x344
    let ops = if img.word(f + 0x344) as u8 == 0x40 {
        TrailOps {
            acc: 0x14,
            seg: 0x18,
            idx: 0x1c,
            seed: 0x20,
            left: 0x24,
            p: 0x40,
            d: 0x58,
            scale: 0x28,
            k1: 0x353,
            k2: 0x44e,
            k3: 0x456,
            k4: 0x4c7,
            alpha: 0x4b5,
            rec: 0x53c,
            call: 0x546,
            link: false,
        }
    } else {
        TrailOps {
            acc: 0x18,
            seg: 0x1c,
            idx: 0x20,
            seed: 0x24,
            left: 0x28,
            p: 0x50,
            d: 0x40,
            scale: 0x14,
            k1: 0x355,
            k2: 0x44f,
            k3: 0x457,
            k4: 0x4c8,
            alpha: 0x4b6,
            rec: 0x556,
            call: 0x55d,
            link: true,
        }
    };
    let o = fr.ecx;
    let basis = fr.edx;
    let point = fr.arg(g, 0);
    let ebp = fr.esp.wrapping_sub(4);
    let e = (ebp & !0xf).wrapping_sub(0x170);
    let at = |off: u32| e.wrapping_add(off);
    let (m1, m2, m3, m4, q) = (at(0x128), at(0xe8), at(0x68), at(0xa8), at(0x30));
    let of = |off: u32| o.wrapping_add(off);
    for (m, n) in [(m1, 723u32), (m2, 724)] {
        let t = slot(g, n);
        // call width: 1
        g.call(t, o, &[m]);
    }
    g.write::<u32>(w(0x57), m3);
    g.write::<u32>(w(0x63), m4);
    let h = g.read::<u32>(of(0x224));
    for i in 0..4u32 {
        let x = g.read::<u32>(point.wrapping_add(4 * i));
        g.write::<u32>(of(0xc + 16 * h + 4 * i), x);
    }
    let h = g.read::<u32>(of(0x224));
    g.write::<u8>(of(0x22c).wrapping_add(h), 1);
    let h = g.read::<u32>(of(0x224));
    let ring = disp8(img, f + 0xad);
    let next = h.wrapping_add(1);
    let next = if next.cast_signed() >= ring.cast_signed() {
        0
    } else {
        next
    };
    g.write::<u32>(of(0x224), next);
    let mut seed = g.read::<u32>(of(0x220));
    let mut idx = h;
    g.write::<u32>(at(ops.idx), idx);
    g.write::<u32>(at(ops.left), w(0xca));
    g.write::<u32>(at(ops.seed), seed);
    g.write::<f32>(at(ops.seg), 0.0);
    g.write::<f32>(at(ops.acc), 0.0);
    let stepped = crate::sb_common::anim_step(g, img, img.rel32_target(f + 0xdf), o);
    let mut kk = 0u32;
    g.write::<u32>(at(0x10), kk);
    if g.read::<u32>(of(0x228)).cast_signed() <= 0 {
        return Ret::eax(stepped);
    }
    let last = ring.wrapping_sub(1);
    let rd = |g: &G, off: u32| g.read::<f32>(at(off));
    loop {
        let acc = rd(g, ops.acc);
        let seg = rd(g, ops.seg);
        let mut draw = true;
        if !(acc < seg || acc.is_nan() || seg.is_nan()) {
            idx = idx.wrapping_sub(1);
            g.write::<f32>(at(ops.acc), sub(acc, seg));
            let prev = if idx.cast_signed() < 0 { last } else { idx };
            if g.read::<u8>(of(0x22c).wrapping_add(prev)) != 1 {
                return Ret::eax(last);
            }
            let cur = g.read::<u32>(at(ops.idx));
            let p = at(ops.p);
            for i in 0..4u32 {
                let x = g.read::<u32>(of(0xc + 16 * cur + 4 * i));
                g.write::<u32>(p.wrapping_add(4 * i), x);
            }
            let d = at(ops.d);
            let t = slot(g, 234);
            // call width: 3
            g.call(t, o, &[d, of(0xc + 16 * prev), p]);
            let (dx, dy, dz) = (rd(g, ops.d), rd(g, ops.d + 4), rd(g, ops.d + 8));
            let sq = add(add(mul(dy, dy), mul(dx, dx)), mul(dz, dz));
            #[allow(clippy::cast_possible_truncation)] // cvtsd2ss: round to f32
            let len = f64::from(sq).sqrt() as f32;
            if idx.cast_signed() < 0 {
                idx = last;
            }
            g.write::<f32>(at(ops.seg), len);
            g.write::<u32>(at(ops.idx), idx);
            let left = g.read::<u32>(at(ops.left)).wrapping_sub(1);
            g.write::<u32>(at(ops.left), left);
            if left.cast_signed() <= 0 {
                return Ret::eax(left);
            }
            // `ucomiss; lahf; test ah, 0x44; jnp`: taken when equal (exactly)
            draw = len != k(g, 0x1e4);
        }
        if draw {
            let count = g.read::<u32>(of(0x228));
            seed = seed
                .wrapping_mul(w(0x208))
                .wrapping_add(disp8(img, f + 0x20e))
                & w(0x210);
            g.write::<u32>(at(ops.seed), seed);
            for (ch, out) in [(0u32, 0x236u32), (1, 0x262), (2, 0x28e), (3, 0x2bd)] {
                let count = g.read::<u32>(of(0x228));
                let kk = g.read::<u32>(at(0x10));
                let c0 = u32::from(g.read::<u8>(of(0x20c + ch)));
                let c1 = u32::from(g.read::<u8>(of(0x210 + ch)));
                let sum = c0
                    .wrapping_mul(count.wrapping_sub(kk))
                    .wrapping_add(c1.wrapping_mul(kk));
                let v = sum.cast_signed().wrapping_div(count.cast_signed());
                g.write::<u8>(w(out), v.cast_unsigned() as u8);
            }
            let _ = count;
            let acc = rd(g, ops.acc);
            let seg = rd(g, ops.seg);
            let p = at(ops.p);
            if acc < seg || acc.is_nan() || seg.is_nan() {
                let t = div(acc, seg);
                let d = at(ops.d);
                let qx = mul(t, g.read::<f32>(d));
                let qy = mul(t, g.read::<f32>(d.wrapping_add(4)));
                let qz = mul(t, g.read::<f32>(d.wrapping_add(8)));
                for (i, x) in (0u32..).zip([qx, qy, qz]) {
                    g.write::<f32>(q.wrapping_add(4 * i), x);
                }
                let t = slot(g, 248);
                // call width: 3
                g.call(t, o, &[q, p, q]);
                let acc = add(g.read::<f32>(of(0x21c)), rd(g, ops.acc));
                g.write::<f32>(at(ops.acc), acc);
            } else {
                for i in 0..4u32 {
                    let x = g.read::<u32>(p.wrapping_add(4 * i));
                    g.write::<u32>(q.wrapping_add(4 * i), x);
                }
            }
            let kk = g.read::<u32>(at(0x10));
            let s = div(cvt(kk.wrapping_add(1)), cvt(g.read::<u32>(of(0x228))));
            let x = mul(g.read::<f32>(of(0x218)), s);
            g.write::<u32>(q.wrapping_add(0xc), 0x3f80_0000);
            let scale = add(mul(sub(k(g, ops.k1), s), g.read::<f32>(of(0x214))), x);
            g.write::<f32>(at(ops.scale), scale);
            let mut st = Pushes::default();
            st.push(m3);
            let t = slot(g, 235);
            // call width: 12 (Pushes caps the words at 12)
            st.call(g, t, o);
            for (row, base) in [(0u32, 0u32), (0x10, 0x10), (0x20, 0x20)] {
                for c in 0..3u32 {
                    let b = g.read::<f32>(basis.wrapping_add(base + 4 * c));
                    let v = if row == 0x20 && c == 0 {
                        mul(scale, b)
                    } else {
                        mul(b, scale)
                    };
                    g.write::<f32>(m3.wrapping_add(row + 4 * c), v);
                }
            }
            let turn = mul(
                mul(cvt(seed.wrapping_sub(0x7fff)), k(g, ops.k2)),
                k(g, ops.k3),
            );
            st.set_top(turn.to_bits());
            st.push_all(&[m3, m3]);
            let t = slot(g, 330);
            // call width: 12 (Pushes caps the words at 12)
            st.call(g, t, o);
            let t = slot(g, 229);
            // call width: 3
            g.call(t, o, &[m3.wrapping_add(0x30), m2, q]);
            let t = slot(g, 230);
            // call width: 2
            g.call(t, o, &[m4, m3]);
            let tz = sub(g.read::<f32>(at(0xe0)), k(g, ops.k4));
            g.write::<f32>(at(0xe0), tz);
            if g.read::<u8>(w(ops.alpha)) != 0 {
                let table = |g: &mut G| {
                    let key = g.read::<u32>(o);
                    let t = slot(g, 941);
                    // call width: 1
                    g.call(t, o, &[key]).eax
                };
                let t1 = table(g);
                let n = i32::from(g.read::<i16>(t1.wrapping_add(6)));
                let pick = seed.cast_signed().wrapping_rem(n);
                let mut frame = i32::from(g.read::<i16>(of(8))).wrapping_add(pick);
                let t2 = table(g);
                if frame >= i32::from(g.read::<i16>(t2.wrapping_add(6))) {
                    let t3 = table(g);
                    frame -= i32::from(g.read::<i16>(t3.wrapping_add(6)));
                }
                let t4 = table(g);
                let entry = t4.wrapping_add(frame.wrapping_add(2).cast_unsigned() << 3);
                let callee_at = img.rel32_target(f + ops.call);
                let esp = e.wrapping_sub(0xc);
                if ops.link {
                    let callee = crate::o6_fam_pairs5::stack_args(
                        g,
                        callee_at,
                        esp,
                        o,
                        entry,
                        &[w(ops.rec), g.read::<u32>(at(0x10))],
                    );
                    crate::sa_misc::link_part(g, img, &callee);
                } else {
                    g.write::<u32>(esp.wrapping_add(4), w(ops.rec));
                    crate::sb_misc::draw_frame(g, img, callee_at, esp, o, entry);
                }
            }
        }
        idx = g.read::<u32>(at(ops.idx));
        kk = g.read::<u32>(at(0x10)).wrapping_add(1);
        g.write::<u32>(at(0x10), kk);
        if kk.cast_signed() >= g.read::<u32>(of(0x228)).cast_signed() {
            return Ret::eax(last);
        }
    }
}

/// The task of a fading spark source with a trail (stack `(task)`, `o` =
/// the task's `+0x14`, its trail `T = o + 0x2c0`, `W = *G_W`; a /GS
/// frame, `ebp` = the entry `esp - 4`; `magic_0544`, `magic_0555`,
/// `magic_0556` 0x10002250, 629 bytes; family `F0544_10002250`):
/// - the timer `o + 0x50c` drops while positive; still `>= 0`: the place
///   `V` = `(1 - s) o.270.. + s o.260..` (`s` = timer `/ o.510`, `K`
///   = 1 from the code), `V.w` = 1; `W + 0x5c04` picks the sparks at `V`
///   (0: [`crate::s4_fam446::sparks_a`], 2: `sparks_c`, 3: `sparks_d`);
///   the part's matrix ([`crate::sa_misc::part_matrix`], `magic_0555`:
///   [`crate::s5_misc::weapon_matrix`]) from `o + 0x280` and `V`; `o +
///   0xd0..` = the place, `+0xdc` 1.0. At timer 0: a child starts
///   ([`crate::s4_fam446::start_child_a`] with `W + 0x5c04`); with that
///   word 0 the timer goes on below 0, else the object leaves the host
///   list (slot 713 `(o)`, `*G_P` = the code's word, `o + 0x124 &=
///   0x7fff` when it was there), `W + 0x5c0c` rises and the task is given
///   back ([`crate::sa_pool::give_back`]);
/// - below 0: the trail shortens (`T + 0x228`; at 0 `W + 0x5c0c` rises
///   and the task is given back, the object staying listed), `V` = `o +
///   0x270..`;
/// - with `W + 0x5c04` 0 the trail is drawn
///   ([`trail_draw`]) with the basis 0.2 times the unit (slot 235).
///
/// Returns the last callee's `eax` (the trail count left in the second
/// case without drawing).
#[allow(clippy::too_many_lines)] // the original's three paths, one step at a time
pub fn trail_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, cvt, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let task = fr.arg(g, 0);
    let o = g.read::<u32>(task.wrapping_add(0x14));
    let ww = g.read::<u32>(w(0x21));
    let trail = o.wrapping_add(0x2c0);
    let ebp = fr.esp.wrapping_sub(4);
    let at = |off: u32| ebp.wrapping_sub(off);
    let (v, m) = (at(0x18), at(0x5c));
    let mode = |g: &G| g.read::<u32>(ww.wrapping_add(0x5c04));
    let timer = o.wrapping_add(0x50c);
    let t0 = g.read::<u32>(timer);
    if t0.cast_signed() > 0 {
        g.write::<u32>(timer, t0.wrapping_sub(1));
    }
    let t = g.read::<u32>(timer);
    // the object leaves (the host list first when `unlist`), `W + 0x5c0c`
    // rises, the task is given back
    let leave = |g: &mut G, unlist: bool| {
        let list_at = img.rel32_target(f + 0x188);
        if unlist && crate::l5_objhost::list_remove(g, img, list_at, w(0x183), o) != 0 {
            let p = g.read::<u32>(w(0x191));
            g.write::<u32>(p, w(0x198));
            let s = slot_target(g, w(0x19d), 713);
            // call width: 1
            g.call(s, 0, &[o]);
            let x = g.read::<u16>(o.wrapping_add(0x124)) & 0x7fff;
            g.write::<u16>(o.wrapping_add(0x124), x);
        }
        let n = g.read::<u32>(ww.wrapping_add(0x5c0c)).wrapping_add(1);
        g.write::<u32>(ww.wrapping_add(0x5c0c), n);
        Ret::eax(crate::sa_pool::give_back(
            g,
            img,
            img.rel32_target(f + 0x1c1),
            task,
        ))
    };
    if t.cast_signed() >= 0 {
        let s = div(cvt(t), cvt(g.read::<u32>(o.wrapping_add(0x510))));
        let r = sub(g.read::<f32>(w(0x5d)), s);
        let md = mode(g);
        g.write::<u32>(at(0xc), 0x3f80_0000);
        for (i, keep) in [(0u32, 0x6cu32), (1, 0x64), (2, 0x60)] {
            let a = mul(g.read::<f32>(o.wrapping_add(0x260 + 4 * i)), s);
            let x = add(mul(g.read::<f32>(o.wrapping_add(0x270 + 4 * i)), r), a);
            g.write::<f32>(at(keep), x);
            g.write::<f32>(v.wrapping_add(4 * i), x);
        }
        let sparks = match md {
            0 => Some((
                0x10d,
                crate::s4_fam446::sparks_a::<G> as fn(&mut G, &Image, &Frame) -> Ret,
            )),
            2 => Some((
                0x103,
                crate::s4_fam446::sparks_c::<G> as fn(&mut G, &Image, &Frame) -> Ret,
            )),
            3 => Some((
                0xf9,
                crate::s4_fam446::sparks_d::<G> as fn(&mut G, &Image, &Frame) -> Ret,
            )),
            _ => None,
        };
        if let Some((call, port)) = sparks {
            let fr = Frame {
                at: img.rel32_target(f + call),
                esp: at(0x80),
                ecx: v,
                ..Frame::default()
            };
            port(g, img, &fr);
        }
        let mat_at = img.rel32_target(f + 0x11c);
        let ang = o.wrapping_add(0x280);
        let eax = if img.matches(mat_at, &crate::s5_misc::WEAPON_MATRIX) {
            crate::s5_misc::weapon_matrix(g, img, mat_at, at(0x84), ang, v)
        } else {
            let fr = Frame {
                at: mat_at,
                esp: at(0x84),
                ecx: ang,
                edx: v,
                ..Frame::default()
            };
            crate::sa_misc::part_matrix(g, img, &fr)
        };
        for (i, keep) in [(0u32, 0x6cu32), (1, 0x64), (2, 0x60)] {
            let x = g.read::<u32>(at(keep));
            g.write::<u32>(o.wrapping_add(0xd0 + 4 * i), x);
        }
        g.write::<u32>(o.wrapping_add(0xdc), 0x3f80_0000);
        if g.read::<u32>(timer) != 0 {
            if mode(g) != 0 {
                return Ret::eax(eax);
            }
        } else {
            let child = crate::o6_fam_pairs5::stack_args(
                g,
                img.rel32_target(f + 0x170),
                at(0x84),
                o,
                ang,
                &[mode(g)],
            );
            crate::s4_fam446::start_child_a(g, img, &child);
            if mode(g) != 0 {
                return leave(g, true);
            }
            let t = g.read::<u32>(timer).wrapping_sub(1);
            g.write::<u32>(timer, t);
        }
    } else {
        let count = g.read::<u32>(trail.wrapping_add(0x228));
        if count.cast_signed() <= 0 {
            return leave(g, false);
        }
        let count = count.wrapping_sub(1);
        g.write::<u32>(trail.wrapping_add(0x228), count);
        for i in 0..4u32 {
            let x = g.read::<u32>(o.wrapping_add(0x270 + 4 * i));
            g.write::<u32>(v.wrapping_add(4 * i), x);
        }
        if mode(g) != 0 {
            return Ret::eax(count);
        }
    }
    let s = slot_target(g, w(0x231), 235);
    // call width: 1 (the word stays on the stack under the next call's)
    g.call(s, 0, &[m]);
    for i in [0u32, 5, 10] {
        g.write::<u32>(m.wrapping_add(4 * i), 0x3e4c_cccd);
    }
    let draw = crate::o6_fam_pairs5::stack_args(
        g,
        img.rel32_target(f + 0x25d),
        at(0x88),
        trail,
        m,
        &[v, m],
    );
    trail_draw(g, img, &draw)
}

/// Where a copy of [`packed_sprite`] keeps its operands: the lock
/// global (taken, put back), the scale `K`, the three slot-table
/// globals, slot 776's byte, the two words of slot 905's pair, the
/// global the second float comes from instead of the stack (`magic_0266`)
/// and the colour words `S` (offsets below `ebp`, then the code word
/// written there, `None` for the scaled size `x`).
struct PackedOps {
    lock: u32,
    unlock: u32,
    k: u32,
    slots: [u32; 3],
    kind: u32,
    pair: [u32; 2],
    second: Option<u32>,
    colour: [(u32, Option<u32>); 4],
}

/// The operands of the [`packed_sprite`] copy at `f`.
fn packed_ops(img: &Image, f: u32) -> PackedOps {
    // `sub esp, 0x120` (0266); `mulss` right after the shift (0353)
    if img.word(f + 0x5) as u8 == 0x20 {
        PackedOps {
            lock: 0x51,
            unlock: 0xf5,
            k: 0x93,
            slots: [0xd5, 0x106, 0x158],
            kind: 0x10b,
            pair: [0x13c, 0x134],
            second: Some(0x26),
            colour: [
                (0x114, Some(0xb0)),
                (0x110, Some(0xa6)),
                (0x10c, Some(0xba)),
                (0x108, None),
            ],
        }
    } else if img.word(f + 0x79) as u8 == 0xf3 {
        PackedOps {
            lock: 0x3c,
            unlock: 0xe3,
            k: 0x7d,
            slots: [0xc3, 0xf4, 0x143],
            kind: 0xf9,
            pair: [0x127, 0x11f],
            second: None,
            colour: [
                (0x114, None),
                (0x110, Some(0xa0)),
                (0x10c, Some(0x96)),
                (0x108, None),
            ],
        }
    } else {
        PackedOps {
            lock: 0x3c,
            unlock: 0xe5,
            k: 0x83,
            slots: [0xc5, 0xf6, 0x145],
            kind: 0xfb,
            pair: [0x129, 0x121],
            second: None,
            colour: [
                (0x114, Some(0xa0)),
                (0x110, Some(0x96)),
                (0x10c, Some(0xaa)),
                (0x108, None),
            ],
        }
    }
}

/// A sprite from a packed 64-bit descriptor (internal: `ecx` = the
/// owner, stack `(-, buffer, lo, hi, a, b, n)`; a /GS frame, `ebp` = the
/// entry `esp - 4`; `magic_0353` 0x100013b0, 358 bytes, `magic_0672`
/// 0x10001310, 360 bytes, and `magic_0266` 0x10001310, 379 bytes;
/// family `F0353_100013b0`): with the lock `**G_L` 1, slot 885 `(lo &
/// 0x3fff, (d >> 20) & 0x3f, 0, 0, 1 << ((d >> 26) & 15), 1 << ((d >>
/// 30) & 15), &B)` (`d` the descriptor, the shifts 16-bit, `B` 0x100
/// bytes at `ebp - 0x104`) and the lock put back. Then, if slot 885
/// answered, slot 905 `(owner + 0x830, &B, buffer, a, b, k, k, &S, slot
/// 776 (kind))`. `S` is four floats at `ebp - 0x114` built from `x =
/// (n * 128 / 128) * K` (`0353`: `(x, 1, 1, x)`; `0672`: `(0.75, 0.75,
/// 1, x)`; `0266`: `(0.5, 0.5, 1, x)`, and its `b` is a constant of the
/// DLL, its `k` 0). Returns slot 905's `eax`, or 0.
pub fn packed_sprite<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{cvt, mul};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let ops = packed_ops(img, f);
    let ebp = fr.esp.wrapping_sub(4);
    let (b, s) = (ebp.wrapping_sub(0x104), ebp.wrapping_sub(0x114));
    let owner = g.read::<u32>(fr.ecx.wrapping_add(0x830));
    let buffer = fr.arg(g, 1);
    let (lo, hi) = (fr.arg(g, 2), fr.arg(g, 3));
    let first = fr.arg(g, 4);
    let n = fr.arg(g, 6);
    // `shl 7; cdq; and edx, 0x7f; add; sar 7`: a division by 128 toward 0
    let m = n << 7;
    let q = m
        .wrapping_add((m.cast_signed() >> 31).cast_unsigned() & 0x7f)
        .cast_signed()
        >> 7;
    let x = mul(cvt(q.cast_unsigned()), g.read::<f32>(w(ops.k)));
    let lock = g.read::<u32>(w(ops.lock));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let d = (u64::from(hi) << 32) | u64::from(lo);
    let bit = |sh: u32| u32::from(1u16 << ((d >> sh) & 0xf));
    for (off, word) in ops.colour {
        let v = word.map_or(x.to_bits(), &w);
        g.write::<u32>(ebp.wrapping_sub(off), v);
    }
    let t = slot_target(g, w(ops.slots[0]), 885);
    // call width: 7
    let made = g
        .call(
            t,
            0,
            &[
                lo & 0x3fff,
                ((d >> 20) & 0x3f) as u32,
                0,
                0,
                bit(26),
                bit(30),
                b,
            ],
        )
        .eax;
    let lock = g.read::<u32>(w(ops.unlock));
    g.write::<u32>(lock, saved);
    if made == 0 {
        return Ret::eax(0);
    }
    let t = slot_target(g, w(ops.slots[1]), 776);
    // call width: 1
    let kind = g.call(t, 0, &[disp8(img, f + ops.kind)]).eax;
    let second = ops
        .second
        .map_or_else(|| fr.arg(g, 5), |o| g.read::<u32>(w(o)));
    let t = slot_target(g, w(ops.slots[2]), 905);
    // call width: 9
    let args = [
        owner,
        b,
        buffer,
        first,
        second,
        w(ops.pair[0]),
        w(ops.pair[1]),
        s,
        kind,
    ];
    Ret::eax(g.call(t, 0, &args).eax)
}

/// How a copy of [`basis_start`] picks slot 236's kind: a byte of the
/// code, or one of two bytes by whether the object lies in the 0x80
/// bytes from a base (`magic_0664`).
enum BasisKind {
    Byte(u32),
    Range {
        base: u32,
        inside: u32,
        outside: u32,
    },
}

/// A copy of [`basis_start`]: the frame's locals (offsets below `ebp`:
/// the matrix rows `A`, slot 325's input row, `B`, `C`, `D`, `E`, `F`,
/// `M` and `G`), `B`'s scale (`magic_0665`), and the code offsets of the
/// slot-table global, the globals and words of the tail (`G1`, `G2`,
/// `G3`, `G4`, `G5`, `G6`, the word written through `G6`, the lock taken
/// and put back, slot 712's byte).
struct BasisOps {
    kind: BasisKind,
    a: u32,
    row: u32,
    b: u32,
    scale: Option<u32>,
    c: u32,
    d: u32,
    e: u32,
    f: u32,
    m: u32,
    g: u32,
    slots: u32,
    g1: u32,
    tail: [u32; 7],
    unlock: u32,
    byte: u32,
}

/// The operands of the [`basis_start`] copy at `f`.
fn basis_ops(img: &Image, f: u32) -> BasisOps {
    if img.word(f + 0x18) as u8 == 0x8b {
        // `mov eax, edi; sub eax, base; test eax, -0x80` (0664)
        BasisOps {
            kind: BasisKind::Range {
                base: 0x1b,
                inside: 0x39,
                outside: 0x40,
            },
            a: 0xc4,
            row: 0xc4,
            b: 0xd4,
            scale: None,
            c: 0x84,
            d: 0x34,
            e: 0x14,
            f: 0x24,
            m: 0x74,
            g: 0x94,
            slots: 0x2c,
            g1: 0x46,
            tail: [0x255, 0x26e, 0x27a, 0x28d, 0x29a, 0x2a2, 0x2a7],
            unlock: 0x2c3,
            byte: 0x29f,
        }
    } else if img.word(f + 0x8b) as u8 == 0xf3 {
        // `movss xmm0, [ebp - 0x24]` after slot 325: `B` scaled (0665)
        BasisOps {
            kind: BasisKind::Byte(0x25),
            a: 0xd4,
            row: 0xd4,
            b: 0x24,
            scale: Some(0x94),
            c: 0x94,
            d: 0x34,
            e: 0x14,
            f: 0x44,
            m: 0x84,
            g: 0xa4,
            slots: 0x20,
            g1: 0x28,
            tail: [0x26c, 0x285, 0x291, 0x2ad, 0x2b4, 0x2ba, 0x2bf],
            unlock: 0x2dd,
            byte: 0x2d1,
        }
    } else {
        BasisOps {
            kind: BasisKind::Byte(0x25),
            a: 0xd4,
            row: 0xc4,
            b: 0x94,
            scale: None,
            c: 0x84,
            d: 0x34,
            e: 0x14,
            f: 0x24,
            m: 0x74,
            g: 0xa4,
            slots: 0x20,
            g1: 0x28,
            tail: [0x23a, 0x253, 0x25f, 0x27b, 0x282, 0x28a, 0x28f],
            unlock: 0x2ab,
            byte: 0x287,
        }
    }
}

/// An object started in a frame made from a slot 236 matrix (stack
/// `(P)`; a /GS frame, `ebp` = the entry `esp - 4`; `magic_0666`
/// 0x10001900, 717 bytes, `magic_0664` 0x100018d0, 741 bytes, and
/// `magic_0665` 0x10001b10, 767 bytes; family `F0666_10001900`):
/// - slot 236 `(*G1, kind, A)` (`kind` a byte of the code; `0664`: 0xba
///   for a `P` in the 0x80 bytes from a base, else 0xbe), its three rows
///   normalized (slot 232 `(r, r)`); slot 325 `(B, row)` (the first or
///   second row); `B.y` = 0 (`0665`: `B` times `K`, `B.y` = `0 * K`);
///   `C` = `(0, 1, 0, 1)`; slot 232 `(D, B)`;
/// - slot 266 `(E, C, D)` and `(F, D, E)`, both normalized;
/// - `M` = the rows `E`, `F`, `D` (`w` 0) and `H = M + 0x30` = `(0, 0,
///   0, 1)`; slot 325 `(H, G)`, then `H.y` = -0.01;
/// - slot 235 `(I)` (`I` at `ebp - 0x114`), its diagonal `P + 0x60`;
///   slot 231 `(M, M, I)`, 230 `(*G2, M)`, 231 `(*G4, *G3, *G2)`, 230
///   `(*G5, *G4)`;
/// - `**G6` = a word of the code; with the lock `**G_L` 1, slot 712 `(P,
///   0x30)`, the lock put back.
///
/// Returns 1 when slot 712 answered 0, else 0.
pub fn basis_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::mul;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let ops = basis_ops(img, f);
    let p = fr.arg(g, 0);
    let ebp = fr.esp.wrapping_sub(4);
    let at = |off: u32| ebp.wrapping_sub(off);
    let slot = |g: &G, n: u32| slot_target(g, w(ops.slots), n);
    let call = |g: &mut G, n: u32, args: &[u32]| {
        let t = slot(g, n);
        g.call(t, 0, args).eax
    };
    let kind = match ops.kind {
        BasisKind::Byte(o) => disp8(img, f + o),
        BasisKind::Range {
            base,
            inside,
            outside,
        } => {
            let o = if p.wrapping_sub(w(base)) & !0x7f == 0 {
                inside
            } else {
                outside
            };
            // `push imm32`
            w(o)
        }
    };
    let a = at(ops.a);
    let g1 = g.read::<u32>(w(ops.g1));
    // call width: 3
    call(g, 236, &[g1, kind, a]);
    for i in 0..3u32 {
        let r = a.wrapping_add(0x10 * i);
        // call width: 2
        call(g, 232, &[r, r]);
    }
    let (b, c, d, e, fv, m) = (
        at(ops.b),
        at(ops.c),
        at(ops.d),
        at(ops.e),
        at(ops.f),
        at(ops.m),
    );
    // call width: 2
    call(g, 325, &[b, at(ops.row)]);
    if let Some(k) = ops.scale {
        let k = g.read::<f32>(w(k));
        let x = mul(g.read::<f32>(b), k);
        g.write::<f32>(b, x);
        g.write::<f32>(b.wrapping_add(4), mul(0.0, k));
        let z = mul(g.read::<f32>(b.wrapping_add(8)), k);
        g.write::<f32>(b.wrapping_add(8), z);
    } else {
        g.write::<u32>(b.wrapping_add(4), 0);
    }
    for (i, v) in (0u32..).zip([0u32, 0x3f80_0000, 0, 0x3f80_0000]) {
        g.write::<u32>(c.wrapping_add(4 * i), v);
    }
    // call width: 2
    call(g, 232, &[d, b]);
    // call width: 3
    call(g, 266, &[e, c, d]);
    // call width: 3
    call(g, 266, &[fv, d, e]);
    for v in [e, fv] {
        // call width: 2
        call(g, 232, &[v, v]);
    }
    for (row, v) in (0u32..).zip([e, fv, d]) {
        for i in 0..3u32 {
            let x = g.read::<u32>(v.wrapping_add(4 * i));
            g.write::<u32>(m.wrapping_add(0x10 * row + 4 * i), x);
        }
        g.write::<u32>(m.wrapping_add(0x10 * row + 0xc), 0);
    }
    let h = m.wrapping_add(0x30);
    for (i, v) in (0u32..).zip([0u32, 0, 0, 0x3f80_0000]) {
        g.write::<u32>(h.wrapping_add(4 * i), v);
    }
    // call width: 2
    call(g, 325, &[h, at(ops.g)]);
    let scale = g.read::<u32>(p.wrapping_add(0x60));
    g.write::<u32>(h.wrapping_add(4), 0xbc23_d70a);
    let i_m = at(0x114);
    // call width: 1
    call(g, 235, &[i_m]);
    for o in [0x28u32, 0x14, 0] {
        g.write::<u32>(i_m.wrapping_add(o), scale);
    }
    let gv = |g: &G, k: usize| g.read::<u32>(w(ops.tail[k]));
    // call width: 3
    call(g, 231, &[m, m, i_m]);
    let g2 = gv(g, 0);
    // call width: 2
    call(g, 230, &[g2, m]);
    let (g4, g3, g2) = (gv(g, 2), gv(g, 1), gv(g, 0));
    // call width: 3
    call(g, 231, &[g4, g3, g2]);
    let (g5, g4) = (gv(g, 3), gv(g, 2));
    // call width: 2
    call(g, 230, &[g5, g4]);
    let g6 = gv(g, 4);
    g.write::<u32>(g6, w(ops.tail[5]));
    let lock = gv(g, 6);
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    // call width: 2
    let r = call(g, 712, &[p, disp8(img, f + ops.byte)]);
    let lock = g.read::<u32>(w(ops.unlock));
    g.write::<u32>(lock, saved);
    Ret::eax(u32::from(r == 0))
}

/// An effect overlay's set-up (stack `(arg)`; `magic_0673` 0x10001000,
/// 272 bytes, and `magic_0253` 0x10001000, 284 bytes; family
/// `F0673_10001000`): `*G_ARG` = `arg` (`0253` first runs the object
/// block's set-up, [`crate::l5_objhost::obj_block_init`] with `arg`);
/// slots 556, 555 and 331 on three of the overlay's records; three
/// scales set to 1.0; slot 790's float into a global; the task ring
/// ([`crate::l5_objhost::tasks_init`]); the 0x3ff entries of 12 bytes from
/// a table cleared (their first word), the free count 0x3ff and the used
/// count 0 (`u16`); slot 720 `(0x3400, 0x800)`, slot 718 `(R)`, slot 222
/// `(R, 0, kind, 1)` and slot 221 `(R, 0, 1000, 1)` (`R` a record of the
/// overlay, `kind` a byte of the code); a global cleared. Returns 0.
pub fn overlay_setup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    // `mov ecx, [ebp + 8]` (0253: the block's set-up first) or `mov eax`
    let block = (img.word(f + 3) >> 8) as u8 == 0x4d;
    // the code offsets are `0673`'s; `0253`'s are 6 bytes later up to its
    // loop, 12 after (its loop is aligned)
    let sh = |o: u32| match (block, o < 0x84) {
        (false, _) => o,
        (true, true) => o + 6,
        (true, false) => o + 0xc,
    };
    let w = |o: u32| img.word(f + sh(o));
    let arg = fr.arg(g, 0);
    // `0673`'s store is `a3` (one byte shorter than `0253`'s `89 0d`)
    g.write::<u32>(img.word(f + if block { 0x8 } else { 0x7 }), arg);
    if block {
        crate::l5_objhost::obj_block_init(g, img, img.rel32_target(f + 0xd), arg);
    }
    let slot = |g: &G, n: u32| slot_target(g, w(0xc), n);
    for (n, o) in [(556u32, 0x11u32), (555, 0x23), (331, 0x35)] {
        let t = slot(g, n);
        // call width: 1
        g.call(t, 0, &[w(o)]);
    }
    for o in [0x51u32, 0x5b, 0x65] {
        g.write::<u32>(w(o), 0x3f80_0000);
    }
    let t = slot(g, 790);
    // call width: 0
    let x = crate::ob_model::st0_f32(g.call(t, 0, &[]).st0);
    g.write::<f32>(w(0x71), x);
    crate::l5_objhost::tasks_init(g, img, img.rel32_target(f + sh(0x76)));
    let table = w(0x7b);
    for i in 0..w(0x80) {
        g.write::<u32>(table.wrapping_add(0xc * i), 0);
    }
    g.write::<u16>(w(0x97), w(0x91) as u16);
    g.write::<u16>(w(0x9f), 0);
    let t = slot(g, 720);
    // call width: 2
    g.call(t, 0, &[w(0xb4), w(0xa9)]);
    let rec = w(0xc0);
    let t = slot(g, 718);
    // call width: 1
    g.call(t, 0, &[rec]);
    let t = slot(g, 222);
    // call width: 4
    g.call(
        t,
        0,
        &[
            w(0xde),
            disp8(img, f + sh(0xdc)),
            disp8(img, f + sh(0xda)),
            disp8(img, f + sh(0xd2)),
        ],
    );
    let t = slot(g, 221);
    // call width: 4
    g.call(
        t,
        0,
        &[
            w(0xf9),
            disp8(img, f + sh(0xf7)),
            w(0xf2),
            disp8(img, f + sh(0xea)),
        ],
    );
    g.write::<u32>(w(0x104), 0);
    Ret::eax(0)
}

/// A scene's end step (no arguments; a /GS frame, `ebp` = the entry
/// `esp - 4`; `magic_0610` 0x1000ab20, 374 bytes, and `magic_0434`
/// 0x10005b90, 384 bytes; family `F0610_1000ab20`):
/// - slot 384 on the first actor's word (`T[G_IDS[0]]`), slots 25 and 24
///   `(0x3018, 1 / 0)`; slot 547 on the word of every further actor (1
///   up to `*G_N`; the code's `-1` case reads `*G_ONE`). `magic_0434`
///   then registers the parts ([`crate::sa_misc::register_parts`]) and
///   clears a global;
/// - unless `*G_S + K` (an offset of the code) is set: the party's
///   actors run ([`crate::sa_script::each_actor`] with 1); with `*G_F`
///   0 and the index `*G_I` below 0x20, slots 217 and 255 `(i, &L)`;
///   slot 211 `(*G_ONE, 0)`; with the halfword `*G_H` 0, slot 213
///   `(*G_W, 0)`; `*G_SHOWN` = 1 and the others shown
///   ([`crate::sb_vm::show_others`] with 1); a held node given back
///   ([`crate::sa_pool::give_back`]) and cleared;
/// - `magic_0610` registers the parts here; `**G_DONE` = 0.
///
/// Returns `*G_DONE`.
pub fn scene_end<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    // `call rel32` right after the loop (0434: the parts first)
    let early = img.word(f + 0x96) as u8 == 0xe8;
    // the code offsets are `0610`'s; `0434`'s are 5 bytes later after its
    // early call, 15 after the global it clears, 10 after `0610`'s late
    // call
    let sh = |o: u32| match (early, o) {
        (false, _) | (true, ..0x96) => o,
        (true, 0x96..0x9b) => o + 5,
        (true, 0x9b..0x15c) => o + 0xf,
        (true, _) => o + 0xa,
    };
    let w = |o: u32| img.word(f + sh(o));
    let ebp = fr.esp.wrapping_sub(4);
    let slot = |g: &G, n: u32| slot_target(g, w(0x21), n);
    let word = |g: &G, ids: u32, table: u32, i: u32| {
        let id = u32::from(g.read::<u8>(g.read::<u32>(w(ids)).wrapping_add(i)));
        g.read::<u32>(w(table).wrapping_add(4 * id))
    };
    let first = word(g, 0x11, 0x1c, 0);
    let t = slot(g, 384);
    // call width: 1
    let mut eax = g.call(t, 0, &[first]).eax;
    for (n, o) in [(25u32, 0x33u32), (24, 0x44)] {
        let t = slot(g, n);
        // call width: 2
        eax = g.call(t, 0, &[w(o + 5), disp8(img, f + o)]).eax;
    }
    let mut i = 1u32;
    while i.cast_signed() < g.read::<u32>(w(0x59)).cast_signed() {
        let v = if i == u32::MAX {
            g.read::<u32>(w(0x66))
        } else {
            word(g, 0x6d, 0x78, i)
        };
        let t = slot(g, 547);
        // call width: 1
        eax = g.call(t, 0, &[v]).eax;
        i = i.wrapping_add(1);
    }
    let parts = |g: &mut G, at: u32, eax: u32| {
        crate::sa_misc::register_parts(g, img, img.rel32_target(f + at), eax);
    };
    if early {
        parts(g, 0x97, eax);
        g.write::<u32>(img.word(f + 0xa2), 0);
    }
    let s = g.read::<u32>(w(0x97));
    if g.read::<u32>(s.wrapping_add(w(0x9d))) == 0 {
        crate::sa_script::each_actor(
            g,
            img,
            img.rel32_target(f + sh(0xaf)),
            1,
            ebp.wrapping_sub(0x20),
        );
        let one = g.read::<u32>(w(0xbc));
        if g.read::<u32>(w(0xb5)) == 0 {
            let idx = g.read::<u32>(w(0xc4));
            if idx.cast_signed() < 0x20 {
                let l = ebp.wrapping_sub(0x14);
                for n in [217u32, 255] {
                    let t = slot(g, n);
                    // call width: 2
                    g.call(t, 0, &[idx, l]);
                }
            }
        }
        let t = slot(g, 211);
        // call width: 2
        g.call(t, 0, &[one, 0]);
        if g.read::<u16>(w(0x10a)) == 0 {
            let x = g.read::<u32>(w(0x11b));
            let t = slot(g, 213);
            // call width: 2
            g.call(t, 0, &[x, 0]);
        }
        g.write::<u32>(w(0x131), 1);
        eax = crate::sb_vm::show_others(
            g,
            img,
            img.rel32_target(f + sh(0x13a)),
            ebp.wrapping_sub(0x1c),
            1,
        );
        let node = g.read::<u32>(w(0x140));
        if node != 0 {
            eax = crate::sa_pool::give_back(g, img, img.rel32_target(f + sh(0x149)), node);
            g.write::<u32>(w(0x14f), 0);
        }
    } else {
        eax = s;
    }
    if !early {
        parts(g, 0x158, eax);
    }
    let done = g.read::<u32>(w(0x15d));
    g.write::<u32>(done, 0);
    Ret::eax(done)
}

include!("o6_fam_pairs6_table.rs.in");
