//! Round 35's own ports (own rows of `magic_0450` and up): the DLLs' own
//! code written out, and copies bound by their own `Sig`.

// `comiss` with `jb`/`jae`: an unordered compare takes one side
#![allow(clippy::neg_cmp_op_on_partial_ord)]

use crate::ob_model::st0_f32;
use crate::runtime::slot_target;
use crate::sb_sse::{add, cvtt, div, mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The `(lo, hi)` handle slot 772 gives.
fn handle<G: Game>(g: &mut G, t: u32, words: &[u32]) -> (u32, u32) {
    // call width: 2
    let h = g.call(t, 0, words);
    (h.eax, h.edx)
}

/// `magic_0512`'s trail (0x10007a40, 3116 bytes; a task callback `(A)`,
/// cdecl; a 16-aligned /GS frame `S` of 0x188 bytes below `push ebp`, two
/// pushes below it). The effect `E = A[4]` keeps a ring of 16 points
/// `E + 0x34` (`E + 0x24` counts them), its model `M = E[0x30]`, a fade
/// `E + 0x28`, a wait `E + 0x20`, a seed `E + 0x1c` and a sprite table `P =
/// E[0x2c]`. First `M`'s visit (its vtable `+0x1c`) gets a local cursor
/// (`M`, the DLL's mark byte at `+5`), then by the state `E + 0x18`:
/// - 0: once the wait is over, state 1 with the fade 1.0, the seed and
///   count 0 and the wait 20; the wait counts down;
/// - 1: while the wait runs the fade is `K2`, else it drops by `K1` each
///   call and at 0 the trail ends (state 0x1000 in the same call). The
///   newest point is `M + 0xcc` (slot 325 into the ring, at `15 - count %
///   16`); from two points on, 64 sprites are drawn back along the ring:
///   the colour `C = (128, 110, 60, fade K3)` steps by `(C - (128, 0, 0,
///   0)) / 63` (slots 234, 667), the camera matrix `G2` (slot 230) with
///   its diagonal scaled by `K4` gives the rows of each sprite's matrix
///   `W` (a unit matrix, slot 235) scaled by `k` (`K5`, less `K6` per
///   sprite); its last row is the point through the DLL's matrix `G3`,
///   and [`matrix_product_0512`](crate::r31::matrix_product_0512) finishes
///   it. Each sprite picks an entry of `P` from a 16-bit LCG and the seed,
///   gets a 64-bit handle (slot 772 `(E, n)`, `n` the sprites left) and,
///   with `*G_L` held at 1, slot 773 `(h)`; when it answers non-zero slots
///   875 `(h, Q, R)` and 775 `(h)`, else slot 873 `(h, Q, R, slot 776
///   (byte [Q + 4 + Q[4]]) | slot 777 (R[0x1c]))`, `R` the sprite record
///   (`+2` 0x80, the colour bytes `+4`, `W` at `+0x10`, `*G4` at `+0x14`).
///   The sprites are `d` apart along the ring's segments (their lengths
///   through slot 706 for the first, `_libm_sse2_sqrt_precise` after),
///   `d` starting at `K5` and shrinking by `K6` per sprite; the ring's
///   oldest point (from 17 points on; else the ring's start) or `d`
///   reaching 0 ends the trail. Then the seed steps by 0x200 and the wait
///   counts down;
/// - 0x1000: slot 779 frees the 65 handles `(E, 0..=0x40)`, the four
///   static nodes `E + 0x134`, `+0x160`, `+0x18c`, `+0x1b8` leave their
///   parents, the task's node leaves its ring, the wait counts down and
///   the state is 0x2000;
/// - any other: the wait counts down.
///
/// Returns `eax` as the original leaves it.
#[allow(clippy::too_many_lines)] // the original's states, one by one
pub fn trail_task_0512<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let rf = |g: &G, at: u32| g.read::<f32>(at);
    // push ebp; and esp, -16; sub esp, 0x188; two pushes
    let s = (fr.esp.wrapping_sub(4) & !0xf).wrapping_sub(0x188);
    g.write::<u32>(s + 0x184, g.read::<u32>(w(0xd)) ^ s);
    let l = |o: u32| s.wrapping_sub(8).wrapping_add(o);
    let a = fr.arg(g, 0);
    let e = g.read::<u32>(a + 4);
    let at = |o: u32| e.wrapping_add(o);
    let word = |g: &G, o: u32| g.read::<u32>(at(o));
    let count_down = |g: &mut G| {
        let x = word(g, 0x20).wrapping_sub(1);
        g.write::<u32>(at(0x20), x);
    };
    // the model's visit with a local cursor
    let cursor = l(0x68);
    let model = word(g, 0x30);
    g.write::<u32>(cursor, model);
    let mark = g.read::<u8>(w(0x2a));
    g.write::<u8>(cursor + 5, mark);
    let visit = g.read::<u32>(g.read::<u32>(model).wrapping_add(0x1c));
    // call width: 1 (the visits read their cursor from the stack)
    g.call(visit, cursor, &[cursor]);
    let mut state = word(g, 0x18);
    let fade = match state {
        0 => {
            if word(g, 0x20).cast_signed() <= 0 {
                g.write::<u32>(at(0x18), 1);
                g.write::<u32>(at(0x28), 0x3f80_0000);
                g.write::<u32>(at(0x1c), 0);
                g.write::<u32>(at(0x24), 0);
                g.write::<u32>(at(0x20), 0x14);
            }
            count_down(g);
            return Ret::eax(0);
        }
        1 if word(g, 0x20).cast_signed() > 0 => Some(k(g, 0x92)),
        1 => {
            let x = sub(rf(g, at(0x28)), k(g, 0x4a));
            g.write::<f32>(at(0x28), x);
            if 0.0 < x || x.is_nan() {
                Some(x)
            } else {
                g.write::<u32>(at(0x18), 0x1000);
                state = 0x1000;
                None
            }
        }
        _ => None,
    };
    if let Some(fade) = fade {
        return draw_0512(g, img, fr, s, a, fade);
    }
    if state != 0x1000 {
        count_down(g);
        return Ret::eax(state.wrapping_sub(0x1000));
    }
    // the end: the handles freed, the static nodes leave
    for i in 0..0x41u32 {
        let (lo, hi) = handle(g, t(g, 0xa4a, 772), &[e, i]);
        let lock = gw(g, 0xa59);
        let held = g.read::<u32>(lock);
        g.write::<u32>(lock, 1);
        // call width: 2
        g.call(t(g, 0xa62, 779), 0, &[lo, hi]);
        g.write::<u32>(gw(g, 0xa75), held);
    }
    for node in [0x134u32, 0x160, 0x18c, 0x1b8] {
        crate::sa_misc::leave(g, at(node), 0);
    }
    let eax = crate::h6_setup::unlink_task(g, a);
    count_down(g);
    g.write::<u32>(at(0x18), 0x2000);
    Ret::eax(eax)
}

/// [`trail_task_0512`]'s state 1 with the fade `fade`: the newest point,
/// then the sprites. `s` is the frame, `a` the task argument.
#[allow(clippy::too_many_lines)] // the original's steps, one by one
fn draw_0512<G: Game>(g: &mut G, img: &Image, fr: &Frame, s: u32, a: u32, fade: f32) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let rf = |g: &G, at: u32| g.read::<f32>(at);
    let vec = |g: &G, at: u32| [0u32, 1, 2, 3].map(|i| g.read::<f32>(at + 4 * i));
    let l = |o: u32| s.wrapping_sub(8).wrapping_add(o);
    let e = g.read::<u32>(a + 4);
    let at = |o: u32| e.wrapping_add(o);
    let word = |g: &G, o: u32| g.read::<u32>(at(o));
    let point = |i: u32| e.wrapping_add(0x34).wrapping_add(i.wrapping_mul(16));
    let count_down = |g: &mut G| {
        let x = word(g, 0x20).wrapping_sub(1);
        g.write::<u32>(at(0x20), x);
    };
    // the newest point goes in at 15 - count % 16 (a signed remainder)
    let back = 15u32.wrapping_sub((word(g, 0x24).cast_signed() % 16).cast_unsigned());
    let src = word(g, 0x30).wrapping_add(0xcc);
    // call width: 2
    let r325 = g.call(t(g, 0xc7, 325), 0, &[point(back), src]).eax;
    let n = word(g, 0x24).wrapping_add(1);
    g.write::<u32>(at(0x24), n);
    if word(g, 0x24).cast_signed() < 2 {
        count_down(g);
        return Ret::eax(r325);
    }
    // the colour and its step
    let (colour, red, out) = (l(0xa8), l(0xb8), l(0xc8));
    for (i, x) in (0u32..).zip([
        128.0f32,
        110.0,
        60.0,
        mul(fade, k(g, 0xf1)),
        128.0,
        0.0,
        0.0,
        0.0,
    ]) {
        g.write::<f32>(colour + 4 * i, x);
    }
    // call width: 3
    g.call(t(g, 0x10e, 234), 0, &[out, colour, red]);
    let step = l(0xf8);
    // call width: 3
    g.call(t(g, 0x193, 667), 0, &[step, out, 0x427c_0000]);
    // the sprite record and its matrix
    let (rec, mat, cam) = (l(0x70), l(0x108), l(0x148));
    g.write::<u32>(rec + 0x10, mat);
    g.write::<u16>(rec + 2, 0x80);
    g.write::<u32>(rec + 0xc, 0);
    g.write::<u32>(rec + 0x1c, 0);
    g.write::<u32>(rec + 8, 0);
    g.write::<u8>(rec + 0x18, 0);
    let table = word(g, 0x2c);
    // call width: 2
    g.call(t(g, 0x1f2, 230), 0, &[cam, w(0x1ec)]);
    let k4 = k(g, 0x20b);
    for o in [0u32, 0x14, 0x28] {
        let x = mul(rf(g, cam + o), k4);
        g.write::<f32>(cam + o, x);
    }
    // call width: 1
    g.call(t(g, 0x21c, 235), 0, &[mat]);
    let mut scale = k(g, 0x261);
    let oldest = if word(g, 0x24).cast_signed() > 0x10 {
        back
    } else {
        0
    };
    let (prev, next, delta) = (l(0xc8), l(0xd8), l(0xe8));
    // call width: 2
    g.call(t(g, 0x28e, 325), 0, &[red, point(back)]);
    // call width: 2
    g.call(t(g, 0x2b1, 325), 0, &[prev, red]);
    let mut i = if back == 15 { 0 } else { back.wrapping_add(1) };
    // call width: 2
    g.call(t(g, 0x2e6, 325), 0, &[next, point(i)]);
    // call width: 3
    g.call(t(g, 0x30b, 234), 0, &[delta, next, prev]);
    let mut left = k(g, 0x31b);
    // call width: 1
    let len = st0_f32(g.call(t(g, 0x32b, 706), 0, &[delta]).st0);
    let seed = word(g, 0x1c);
    let groups = i32::from(g.read::<i16>(table + 0x12)).cast_unsigned();
    let kinds = i32::from(g.read::<i16>(table + 6)).cast_unsigned();
    let mut reach = k(g, 0x261);
    let (mut seg, mut walked) = (len, len);
    let mut p = vec(g, red);
    let (mut from, mut d) = (vec(g, prev), vec(g, delta));
    let mut to = vec(g, next);
    let mut colour_now = vec(g, colour);
    let step_v = vec(g, step);
    let mut lcg = 0u32;
    let mut sprites = 0x40u32;
    let m3 = |g: &G, r: u32, c: u32| {
        k(
            g,
            [
                [0x4a1u32, 0x4df, 0x50c, 0x544],
                [0x3d1, 0x3dc, 0x48d, 0x524],
                [0x4bd, 0x4ee, 0x518, 0x554],
                [0x4d0, 0x4fd, 0x534, 0x566],
            ][r as usize][c as usize],
        )
    };
    let eax = 'sprites: loop {
        lcg = lcg.wrapping_mul(0x80d).wrapping_add(7) & 0xffff;
        // (the original divides without a check: a zero count faults)
        let pick = lcg
            .wrapping_add(seed)
            .checked_div(groups)
            .and_then(|q| q.checked_rem(kinds))
            .unwrap_or(0);
        let entry = i32::from(g.read::<i16>(table + 8 * pick + 0x10)).cast_unsigned();
        let sprite = table.wrapping_add(entry);
        // the camera's first two rows (their first three terms) scaled
        for o in [0u32, 4, 8, 0x10, 0x14, 0x18] {
            let x = mul(rf(g, cam + o), scale);
            g.write::<f32>(mat + o, x);
        }
        for (i, x) in (0u32..).zip(colour_now) {
            g.write::<u8>(rec + 4 + i, cvtt(x).to_le_bytes()[0]);
        }
        // the point through the DLL's matrix, `((p1 m1c + p0 m0c) + p2
        // m2c) + p3 m3c`, the products' operands in the code's order
        let term = |g: &G, r: u32, c: u32, mat_first: bool| {
            if mat_first {
                mul(m3(g, r, c), p[r as usize])
            } else {
                mul(p[r as usize], m3(g, r, c))
            }
        };
        let row = [0u32, 1, 2, 3].map(|c| {
            let x = add(term(g, 1, c, c == 3), term(g, 0, c, c == 3));
            let x = add(x, term(g, 2, c, c >= 2));
            add(x, term(g, 3, c, c >= 2))
        });
        for (i, x) in (0u32..).zip(row) {
            g.write::<f32>(mat + 0x30 + 4 * i, x);
        }
        let sp = s.wrapping_sub(0xc);
        g.write::<u32>(sp, mat);
        let pf = Frame {
            at: img.rel32_target(f + 0x5a1),
            esp: sp.wrapping_sub(4),
            ecx: mat,
            ..Frame::default()
        };
        crate::r31::matrix_product_0512(g, img, &pf);
        let g4 = gw(g, 0x5a6);
        g.write::<u32>(rec + 0x14, g4);
        let (lo, hi) = handle(g, t(g, 0x5b5, 772), &[e, sprites]);
        let lock = gw(g, 0x5c8);
        let held = g.read::<u32>(lock);
        g.write::<u32>(lock, 1);
        // call width: 2
        let found = g.call(t(g, 0x5d7, 773), 0, &[lo, hi]).eax;
        let lock = gw(g, 0x5f2);
        g.write::<u32>(lock, held);
        let lock_now = if found != 0 {
            g.write::<u32>(lock, 1);
            // call width: 4
            g.call(t(g, 0x60f, 875), 0, &[lo, hi, sprite, rec]);
            g.write::<u32>(gw(g, 0x62a), 1);
            // call width: 2
            g.call(t(g, 0x639, 775), 0, &[lo, hi]);
            let lock = gw(g, 0x64a);
            g.write::<u32>(lock, held);
            lock
        } else {
            let at4 = g.read::<u32>(sprite.wrapping_add(4));
            let b = u32::from(g.read::<u8>(at4.wrapping_add(sprite).wrapping_add(4)));
            // call width: 1
            let x = g.call(t(g, 0x667, 776), 0, &[b]).eax;
            let r1c = g.read::<u32>(rec + 0x1c);
            // call width: 1
            let y = g.call(t(g, 0x675, 777), 0, &[r1c]).eax;
            let lock = gw(g, 0x68f);
            let held = g.read::<u32>(lock);
            g.write::<u32>(lock, 1);
            // call width: 5
            g.call(t(g, 0x6a5, 873), 0, &[lo, hi, sprite, rec, x | y]);
            let lock = gw(g, 0x6c0);
            g.write::<u32>(lock, held);
            lock
        };
        sprites -= 1;
        if sprites == 0 {
            break lock_now;
        }
        for (c, s) in colour_now.iter_mut().zip(step_v) {
            *c = sub(*c, s);
        }
        scale = sub(scale, k(g, 0x754));
        reach = sub(reach, k(g, 0x768));
        if 0.0 >= reach {
            break lock_now;
        }
        // on along the ring's segments until the walked length reaches
        // `reach` (`jb` goes on below it or unordered)
        if !(walked >= reach) {
            loop {
                let old = i;
                i = if old == 15 { 0 } else { old.wrapping_add(1) };
                if i == oldest {
                    // the oldest point: the trail ends here
                    break 'sprites old;
                }
                from = to;
                to = vec(g, point(i));
                d = [0, 1, 2, 3].map(|c| sub(to[c], from[c]));
                left = sub(left, seg);
                let sq = add(add(mul(d[1], d[1]), mul(d[0], d[0])), mul(d[2], d[2]));
                // `_libm_sse2_sqrt_precise` of the double: exact, as Rust's
                #[allow(clippy::cast_possible_truncation)]
                let length = f64::from(sq).sqrt() as f32;
                walked = add(walked, length);
                seg = length;
                if walked >= reach {
                    break;
                }
            }
        }
        let share = div(left, seg);
        left = add(left, reach);
        walked = sub(walked, reach);
        p = [0, 1, 2, 3].map(|c| add(from[c], mul(d[c], share)));
    };
    let x = word(g, 0x1c).wrapping_add(0x200);
    g.write::<u32>(at(0x1c), x);
    count_down(g);
    Ret::eax(eax)
}

/// [`trail_task_0512`] (the `rel32`s:
/// [`matrix_product_0512`](crate::r31::matrix_product_0512),
/// `_libm_sse2_sqrt_precise`, `__security_check_cookie` three times).
pub const TRAIL_TASK_0512: Sig = Sig {
    len: 3116,
    hash: 0x427f_9490_e59b_3a62,
    calls: &[
        (0x5a1, Callee::Sig(&crate::r31::MATRIX_PRODUCT_0512)),
        (0x8d4, Callee::Import("_libm_sse2_sqrt_precise")),
        (0xa31, Callee::Any),
        (0xbe8, Callee::Any),
        (0xc24, Callee::Any),
    ],
    imports: &[],
};

const ONE: u32 = 0x3f80_0000;

/// `magic_0597`'s scene task (0x10009350, 3404 bytes; a task callback
/// `(A)`, cdecl; an aligned /GS frame `E` below three pushes; the task's
/// record `R = A[4]`, its state `R + 0x18`, its counts `R + 0x34` and `R +
/// 0x38`). By the state:
/// - 0: state 1; 1..0x383 wait;
/// - 0x384: the scene's set-up ([`start_0597`]), then on through 0x385
///   into 0x386 in the same call;
/// - 0x385: the count reset, on into 0x386;
/// - 0x386: a camera word drops by `K` each call; after 70 calls the
///   camera's place (slot 229 of a point), its vectors (slots 325, 234),
///   the camera block's words, state 0x387 with the speed and count reset;
/// - 0x387: the speed `R + 0x1c` is added to that word and grows by `K'`;
///   after 20 calls the light's place ([`light_set`](crate::s4_small::light_set)
///   of slot 229's point, radius 1.0), state 0x388;
/// - 0x388: after 65 calls the light again (0.4) and the move's set-up
///   ([`move_setup`](crate::s4_small::move_setup)), state 0x389;
/// - 0x2710: the current node unlinked ([`list_unlink_current`](crate::l3_common::list_unlink_current)),
///   the task's node leaves its ring.
///
/// Every exit counts both counts up. Returns `eax` as the original leaves
/// it (the state, a jump-table index, the last call's).
#[allow(clippy::too_many_lines)] // the original's states, in order
pub fn scene_task_0597<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    // push ebp; and esp, -8; sub esp, 0x34; three pushes
    let e = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x34);
    g.write::<u32>(e + 0x30, g.read::<u32>(w(0xa)) ^ e);
    let base = e.wrapping_sub(0xc);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    let at = |o: u32| r.wrapping_add(o);
    let count = |g: &G| g.read::<i32>(at(0x38));
    let up = |g: &mut G| {
        let s = g.read::<u32>(at(0x18)).wrapping_add(1);
        g.write::<u32>(at(0x18), s);
    };
    let state = g.read::<u32>(at(0x18));
    let mut eax = state;
    match state {
        0 => {
            eax = 1;
            g.write::<u32>(at(0x18), 1);
        }
        0x384..=0x388 => {
            let mut s = state;
            // the set-up falls into 0x385's code; the others come through
            // the jump table with `eax = state - 0x385`
            eax = if s == 0x384 {
                let x = start_0597(g, img, fr, base);
                up(g);
                g.write::<u32>(at(0x38), 0);
                s = 0x385;
                x
            } else {
                s - 0x385
            };
            if s == 0x385 {
                up(g);
                g.write::<u32>(at(0x38), 0);
                s = 0x386;
            }
            match s {
                0x386 => {
                    if let Some(x) = camera_0597(g, img, f, base, at(0)) {
                        eax = x;
                    }
                }
                0x387 => {
                    let speed = g.read::<f32>(at(0x1c));
                    let x = add(speed, g.read::<f32>(w(0xc09)));
                    g.write::<f32>(w(0xc11), x);
                    let speed = add(g.read::<f32>(at(0x1c)), g.read::<f32>(w(0xc22)));
                    g.write::<f32>(at(0x1c), speed);
                    if count(g) >= 0x14 {
                        let p = place_0597(g, img, f, base, 0xc37, [0xc0e0_0000, 0xc220_0000]);
                        eax = light_0597(g, img, f, base, (0xc79, 0xc6d), p);
                        up(g);
                    }
                }
                _ => {
                    if count(g) >= 0x41 {
                        let p = place_0597(g, img, f, base, 0xc95, [0xc0e0_0000, 0xc1a0_0000]);
                        let lit = light_0597(g, img, f, base, (0xcd7, 0xccb), p);
                        g.write::<u32>(w(0xcf2), 0);
                        let mf = Frame {
                            at: img.rel32_target(f + 0xcfb),
                            esp: base.wrapping_sub(4),
                            ecx: w(0xcec),
                            eax: lit,
                            xmm: [
                                0,
                                0,
                                u128::from(g.read::<u32>(w(0xce7))),
                                u128::from(g.read::<u32>(w(0xcdf))),
                            ],
                            ..Frame::default()
                        };
                        eax = crate::s4_small::move_setup(g, img, &mf).eax;
                        up(g);
                    }
                }
            }
        }
        0x2710 => {
            crate::l3_common::list_unlink_current(g, img, img.rel32_target(f + 0xd05), eax);
            eax = crate::h6_setup::unlink_task(g, task);
        }
        0x389..0x2710 => eax = state - 0x385,
        _ => {}
    }
    for o in [0x34u32, 0x38] {
        let x = g.read::<u32>(at(o)).wrapping_add(1);
        g.write::<u32>(at(o), x);
    }
    Ret::eax(eax)
}

/// [`scene_task_0597`]'s state 0x386 (the record at `r`): the camera word
/// drops; after 70 calls the camera's moves and the next state (`Some` of
/// the `eax` the stores leave), else `None`.
fn camera_0597<G: Game>(g: &mut G, img: &Image, f: u32, base: u32, r: u32) -> Option<u32> {
    use crate::r25::abs_stores;
    let w = |o: u32| img.word(f + o);
    let x = sub(g.read::<f32>(w(0xa45)), g.read::<f32>(w(0xa4d)));
    g.write::<f32>(w(0xa55), x);
    if g.read::<i32>(r + 0x38) < 0x46 {
        return None;
    }
    let p = place_0597(g, img, f, base, 0xa69, [0xc140_0000, 0xc1c8_0000]);
    vectors_0597(
        g,
        img,
        f,
        p,
        (0xaa8, 0xac3),
        [0xaca, 0xad1, 0xad6, 0xae1, 0xae6],
    );
    let x = sub(g.read::<f32>(w(0xaf0)), g.read::<f32>(w(0xaf8)));
    g.write::<u16>(w(0xb00), 0);
    abs_stores(g, img, f, 0xb04, 0xb90);
    g.write::<f32>(w(0xb94), x);
    abs_stores(g, img, f, 0xb98, 0xbac);
    g.write::<u16>(w(0xbb3), 0xf800);
    abs_stores(g, img, f, 0xbb7, 0xbc1);
    let s = g.read::<u32>(r + 0x18).wrapping_add(1);
    g.write::<u32>(r + 0x18, s);
    g.write::<u32>(r + 0x1c, 0);
    g.write::<u32>(r + 0x38, 0);
    Some(0xffff_f800)
}

/// Slot 229 `(P, G, P)` with `P = E + 0x10` the point `(x, y, -70, 1)`
/// (`G` the immediate at `gop`; the slot through the imports at `gop +
/// 6`).
/// Returns `P`.
fn place_0597<G: Game>(g: &mut G, img: &Image, f: u32, base: u32, gop: u32, xy: [u32; 2]) -> u32 {
    let w = |o: u32| img.word(f + o);
    let p = base + 0x1c;
    for (i, v) in (0u32..).zip([xy[0], xy[1], 0xc28c_0000, ONE]) {
        g.write::<u32>(p + 4 * i, v);
    }
    let t = slot_target(g, w(gop + 6), 229);
    // call width: 3
    g.call(t, 0, &[p, w(gop), p]);
    p
}

/// The camera's vectors after its place `p`: the store run `run`, slot 325
/// `(V, p)`, slot 234 `(D, V, B)` (`ops`: `V`, the imports, `B`, `V`, `D`).
fn vectors_0597<G: Game>(g: &mut G, img: &Image, f: u32, p: u32, run: (u32, u32), ops: [u32; 5]) {
    let w = |o: u32| img.word(f + o);
    crate::r25::abs_stores(g, img, f, run.0, run.1);
    let imports = w(ops[1]);
    // call width: 2
    g.call(slot_target(g, imports, 325), 0, &[w(ops[0]), p]);
    // call width: 3
    g.call(
        slot_target(g, imports, 234),
        0,
        &[w(ops[4]), w(ops[3]), w(ops[2])],
    );
}

/// [`light_set`](crate::s4_small::light_set) at the call `c.0` with the
/// point `p` (`edx`) and the radius at operand `c.1` (`xmm2`). Returns its
/// `eax`.
fn light_0597<G: Game>(g: &mut G, img: &Image, f: u32, base: u32, c: (u32, u32), p: u32) -> u32 {
    let lf = Frame {
        at: img.rel32_target(f + c.0),
        esp: base.wrapping_sub(4),
        edx: p,
        xmm: [0, 0, u128::from(g.read::<u32>(img.word(f + c.1))), 0],
        ..Frame::default()
    };
    crate::s4_small::light_set(g, img, &lf).eax
}

/// [`scene_task_0597`]'s state 0x384 (`base` the stack pointer below the
/// three pushes): the summon's model node, its place (slot 292, moved by
/// slot 520's answer) and slots 255 and 666 unless the model's flag bit 0,
/// its motion; the scene's static nodes appended to the static parent
/// with their words between slots 262, 12, 520, 211, 3, 228 and 670, the
/// time ([`time_left`](crate::m0042::time_left)), three owned nodes
/// ([`node_append_owned`](crate::sa_misc::node_append_owned)), the bones
/// ([`bone_plain`](crate::sa_summon::bone_plain)), the model's state
/// ([`model_state`](crate::m0070::model_state)), a sound node
/// ([`node_reset_sound`](crate::o6_fam_node::node_reset_sound)), the
/// camera's place and vectors, the move's ends
/// ([`move_ends`](crate::s4_small::move_ends)); unless slot 520 answers
/// 0x2370000, slot 296 for each byte slot 288 lists (except the DLL's
/// own, the last's and 0xff); the colour node
/// ([`colour_node`](crate::s3_misc::colour_node)) and its block's words
/// (`K - v`). Returns the colour node's `eax`.
#[allow(clippy::too_many_lines)] // the original's steps, in order
fn start_0597<G: Game>(g: &mut G, img: &Image, fr: &Frame, base: u32) -> u32 {
    use crate::r25::abs_stores;
    use crate::r31::{append_0601, append_pair_0601};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x77), n);
    let call = |o: u32| img.rel32_target(f + o);
    let l = |o: u32| base.wrapping_add(o);
    let below = base.wrapping_sub(4);
    let (grp, p) = (l(0xc), l(0x1c));
    // the summon's model node
    let flags = u32::from(g.read::<u8>(w(0x41)));
    for (i, v) in (0u32..).zip([w(0x4d), w(0x55), flags]) {
        g.write::<u32>(grp + 4 * i, v);
    }
    g.write::<u8>(grp + 0xc, 0);
    abs_stores(g, img, f, 0x62, 0x6c);
    crate::sa_summon::model_node_init(g, img, call(0x6d), grp);
    // call width: 1
    g.call(slot(g, 292), 0, &[p]);
    // call width: 0
    let who = g.call(slot(g, 520), 0, &[]).eax.cast_signed();
    let shift = |g: &mut G, o: u32, k_op: u32, plus: bool| {
        let x = g.read::<f32>(o);
        let y = if plus {
            add(x, k(g, k_op))
        } else {
            sub(x, k(g, k_op))
        };
        g.write::<f32>(o, y);
    };
    match who {
        0x0238_0000 => shift(g, p, 0xfb, false),
        0x0237_0000 => shift(g, p + 8, 0xde, false),
        0x0235_0000 | 0x0236_0000 => {
            shift(g, p, 0xb4, false);
            shift(g, p + 8, 0xc8, true);
        }
        _ => {}
    }
    if g.read::<u8>(w(0x106)) & 1 == 0 {
        let m = gw(g, 0x11a);
        // call width: 2
        g.call(slot(g, 255), 0, &[m, p]);
        if g.read::<u8>(w(0x127)) & 1 == 0 {
            let m = gw(g, 0x147);
            // call width: 2
            g.call(slot(g, 666), 0, &[m, 0]);
        }
    }
    let mark = g.read::<u8>(w(0x151));
    g.write::<u32>(grp, w(0x15d));
    g.write::<u8>(grp + 5, mark);
    let mf = Frame {
        at: call(0x166),
        esp: below,
        ecx: grp,
        ..Frame::default()
    };
    crate::s4_summon::motion_state(g, img, &mf);
    // the first node: its words, appended
    let old = gw(g, 0x16b);
    abs_stores(g, img, f, 0x16f, 0x1a0);
    append_0601(g, img, f, old, [0x1a1, 0x1a6, 0x1ac, 0x1bb, 0x1c7]);
    let m = gw(g, 0x1dc);
    abs_stores(g, img, f, 0x1e0, 0x1f4);
    // call width: 4
    g.call(slot(g, 262), 0, &[m, w(0x1d6), w(0x1d1), w(0x1cc)]);
    let m = gw(g, 0x203);
    // call width: 1 (slot 262's words stay under it)
    let h = crate::ob_model::st0_f32(g.call(slot(g, 12), 0, &[m]).st0);
    g.write::<f32>(w(0x20e), h);
    let old = gw(g, 0x213);
    abs_stores(g, img, f, 0x21a, 0x25f);
    append_0601(g, img, f, old, [0x260, 0x265, 0x26b, 0x27a, 0x281]);
    abs_stores(g, img, f, 0x289, 0x293);
    g.write::<u32>(grp, w(0x297));
    g.write::<u32>(grp + 4, w(0x29f));
    crate::m0042::time_left(g, img, call(0x2a4), grp, below);
    // call width: 0
    if g.call(slot(g, 520), 0, &[]).eax == 0x0237_0000 {
        let x = gw(g, 0x2be).wrapping_sub(0x4000);
        g.write::<u32>(w(0x2be), x);
    }
    let owned = |g: &mut G, (c, ecx, edx): (u32, u32, u32), words: [u32; 4]| {
        let sp = base.wrapping_sub(0x10);
        for (i, v) in (0u32..).zip(words) {
            g.write::<u32>(sp + 4 * i, v);
        }
        let of = Frame {
            at: call(c),
            esp: sp.wrapping_sub(4),
            ecx: w(ecx),
            edx: w(edx),
            ..Frame::default()
        };
        crate::sa_misc::node_append_owned(g, img, &of);
    };
    owned(
        g,
        (0x2e5, 0x2e0, 0x2db),
        [w(0x2d6), w(0x2d1), w(0x2cc), w(0x2c7)],
    );
    // the bones' cursor: the owned node (`ecx` as the append leaves it)
    let mark = g.read::<u8>(w(0x2ea));
    g.write::<u32>(grp, w(0x2e0));
    g.write::<u8>(grp + 5, mark);
    let bf = Frame {
        at: call(0x2fe),
        esp: below,
        ecx: grp,
        ..Frame::default()
    };
    crate::sa_summon::bone_plain(g, img, &bf);
    // two nodes, the second appended after the first
    let old = gw(g, 0x303);
    abs_stores(g, img, f, 0x307, 0x338);
    append_pair_0601(
        g,
        img,
        f,
        old,
        [0x339, 0x33e, 0x344, 0x353, 0x359],
        (0x35e, (0x364, 0x3d1), (0x3d3, 0x3d9, 0x3e5), 0x401),
    );
    abs_stores(g, img, f, 0x3f5, 0x3ff);
    abs_stores(g, img, f, 0x405, 0x40f);
    g.write::<u32>(grp, w(0x413));
    g.write::<u32>(grp + 4, w(0x41b));
    crate::m0070::model_state(g, img, call(0x420), grp);
    let old = gw(g, 0x425);
    abs_stores(g, img, f, 0x429, 0x464);
    append_0601(g, img, f, old, [0x465, 0x46a, 0x470, 0x47f, 0x486]);
    abs_stores(g, img, f, 0x4a8, 0x4d0);
    owned(
        g,
        (0x4d1, 0x4a4, 0x49f),
        [w(0x49a), w(0x495), w(0x490), w(0x48b)],
    );
    let m = gw(g, 0x4e1);
    // call width: 2
    g.call(slot(g, 211), 0, &[m, w(0x4db)]);
    let fl = g.read::<u16>(w(0x4f5)) | 0x10;
    g.write::<u16>(w(0x4f5), fl);
    let m = gw(g, 0x4fc);
    abs_stores(g, img, f, 0x500, 0x50a);
    // call width: 1
    g.call(slot(g, 3), 0, &[m]);
    let b = u32::from(g.read::<u8>(w(0x512)));
    abs_stores(g, img, f, 0x519, 0x523);
    abs_stores(g, img, f, 0x52e, 0x542);
    // call width: 2
    g.call(slot(g, 228), 0, &[b, 0xdf]);
    let m = gw(g, 0x553);
    // call width: 2
    g.call(slot(g, 670), 0, &[m, 0]);
    // the sound node `[N, parent, 700.0]` (`esp` four words lower)
    for (i, v) in (0u32..).zip([w(0x567), w(0x56f), 0x442f_0000]) {
        g.write::<u32>(grp + 4 * i, v);
    }
    abs_stores(g, img, f, 0x57b, 0x585);
    crate::o6_fam_node::node_reset_sound(g, img, call(0x586), grp);
    // the camera's place and vectors
    let p = place_0597(g, img, f, base, 0x590, [0xc000_0000, 0xc1a0_0000]);
    vectors_0597(
        g,
        img,
        f,
        p,
        (0x5cf, 0x5ea),
        [0x5f1, 0x5f8, 0x5fd, 0x608, 0x60d],
    );
    // two nodes, the second appended after the first, its owned node
    let old = gw(g, 0x614);
    abs_stores(g, img, f, 0x61b, 0x64c);
    append_pair_0601(
        g,
        img,
        f,
        old,
        [0x64d, 0x652, 0x658, 0x667, 0x66d],
        (0x672, (0x678, 0x6e5), (0x6e7, 0x6ed, 0x6f9), 0x72f),
    );
    abs_stores(g, img, f, 0x723, 0x72d);
    abs_stores(g, img, f, 0x733, 0x751);
    owned(
        g,
        (0x752, 0x71f, 0x71a),
        [w(0x715), w(0x710), w(0x709), w(0x704)],
    );
    // the move's node, its parent `edx` as the append leaves it
    let parent = w(0x71a);
    let old = gw(g, 0x757);
    abs_stores(g, img, f, 0x75e, 0x782);
    g.write::<u32>(w(0x784), parent);
    g.write::<u32>(w(0x78a), parent);
    g.write::<u32>(w(0x78f), old);
    // (the old last is the pair's second node, never the parent)
    if old == parent {
        abs_stores(g, img, f, 0x797, 0x7a1);
    } else {
        g.write::<u32>(old.wrapping_add(4), w(0x7a6));
    }
    let n = g.read::<u16>(w(0x7ad)).wrapping_add(1);
    g.write::<u16>(w(0x7ad), n);
    abs_stores(g, img, f, 0x7c1, 0x822);
    // the move's ends `[obj, from, to, 0]`, `from` and `to` on the frame
    let (from, to) = (l(0x1c), l(0x2c));
    for v in [from, from + 4, to, to + 4] {
        g.write::<u32>(v, 0);
    }
    for (i, v) in (0u32..).zip([w(0x846), from, to, 0]) {
        g.write::<u32>(grp + 4 * i, v);
    }
    let ef = Frame {
        at: call(0x857),
        esp: below,
        ecx: grp,
        ..Frame::default()
    };
    crate::s4_small::move_ends(g, img, &ef);
    let x = sub(k(g, 0x85f), k(g, 0x867));
    g.write::<u16>(w(0x86f), 0);
    abs_stores(g, img, f, 0x878, 0x896);
    g.write::<f32>(w(0x89a), x);
    // call width: 0
    if g.call(slot(g, 520), 0, &[]).eax != 0x0237_0000 {
        let n_at = grp;
        // call width: 2
        let list = g.call(slot(g, 288), 0, &[0x10, n_at]).eax;
        let mut n = g.read::<i32>(n_at);
        let mut i = 0i32;
        let mut at = list;
        while i < n {
            let c = g.read::<u8>(at);
            let own = g.read::<u8>(w(0x8d4));
            if c != own && u32::from(c) != gw(g, 0x8df) && c != 0xff {
                // call width: 2
                g.call(slot(g, 296), 0, &[u32::from(c), 0]);
                n = g.read::<i32>(n_at);
            }
            i += 1;
            at = at.wrapping_add(1);
        }
    }
    // the colour node and its block's words
    g.write::<u32>(grp, w(0x90f));
    g.write::<u32>(grp + 4, w(0x917));
    abs_stores(g, img, f, 0x91b, 0x925);
    let cf = Frame {
        at: call(0x926),
        esp: below,
        ecx: grp,
        ..Frame::default()
    };
    let eax = crate::s3_misc::colour_node(g, img, &cf);
    let half = k(g, 0x92e);
    let rest = |g: &G, o: u32| sub(half, k(g, o));
    let x = rest(g, 0x939);
    let fl = gw(g, 0x93f) & !1;
    g.write::<u32>(w(0x93f), fl);
    abs_stores(g, img, f, 0x944, 0x962);
    g.write::<f32>(w(0x966), x);
    let x = rest(g, 0x971);
    abs_stores(g, img, f, 0x975, 0x99d);
    g.write::<f32>(w(0x9a1), x);
    let x = rest(g, 0x9ac);
    abs_stores(g, img, f, 0x9b0, 0x9c4);
    g.write::<f32>(w(0x9c8), x);
    let x = rest(g, 0x9d3);
    g.write::<f32>(w(0x9db), x);
    let x = rest(g, 0x9e6);
    g.write::<f32>(w(0x9ee), x);
    let x = rest(g, 0x9f9);
    let y = rest(g, 0xa01);
    g.write::<f32>(w(0xa09), x);
    let x = sub(k(g, 0xa11), k(g, 0xa19));
    g.write::<f32>(w(0xa21), y);
    g.write::<f32>(w(0xa29), x);
    eax
}

/// [`scene_task_0597`] (the `rel32`s: its callees in order,
/// `__security_check_cookie`).
pub const SCENE_TASK_0597: Sig = Sig {
    len: 3404,
    hash: 0xd6ab_0d95_4845_a08b,
    calls: &[
        (0x6d, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0x166, Callee::Sig(&crate::s4_summon::MOTION_STATE)),
        (0x2a4, Callee::Sig(&crate::m0042::TIME_LEFT)),
        (0x2e5, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x2fe, Callee::Sig(&crate::sa_summon::BONE)),
        (0x420, Callee::Sig(&crate::m0070::MODEL_STATE)),
        (0x4d1, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (
            0x586,
            Callee::Sig(&crate::o6_fam_node::NODE_RESET_SOUND_0070_3ED0),
        ),
        (0x752, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x857, Callee::Sig(&crate::s4_small::MOVE_ENDS)),
        (0x926, Callee::Sig(&crate::s3_misc::COLOUR_NODE)),
        (0xc79, Callee::Sig(&crate::s4_small::LIGHT_SET)),
        (0xcd7, Callee::Sig(&crate::s4_small::LIGHT_SET)),
        (0xcfb, Callee::Sig(&crate::s4_small::MOVE_SETUP)),
        (0xd05, Callee::Sig(&crate::l3_common::LIST_UNLINK_CURRENT)),
        (0xd33, Callee::Any),
    ],
    imports: &[],
};

/// Where `magic_0609`'s copy of [`summon_main_0613`] has its code: 0613's
/// offset `x` lies at `x + d` past each `(from, d)` (0609 inserts three
/// stores, a fourth at the end, and turns 0x48's last call into a state of
/// its own).
const SHIFTS_0609: &[(u32, u32)] = &[
    (0, 0),
    (0xd69, 10),
    (0xd83, 20),
    (0x12a1, 9),
    (0x12b4, 6),
    (0x13d2, 18),
    (0x1456, 28),
];

/// `magic_0613`'s summon task (0x1000a2e0, 5502 bytes; a task callback
/// `(A)`, cdecl; an aligned frame `E` below three pushes, no /GS; the
/// record `R = A[4]`, its state `R + 0x18`, counts `R + 0x28`, `R + 0x2c`),
/// and `magic_0609`'s copy (0x1000a730, 5535 bytes; [`summon_main_0609`]).
/// A state from the byte and jump tables (states without code and those
/// past the table only count); several states go on into the next in the
/// same call, so one call from state 0 builds the whole scene:
/// - 0: the summon's model node (slot 294's answer as its flags, 1 when it
///   is 0xff); 0xa: the big node and the scene's static nodes appended to
///   their parents with their words, slots 676 twice, two colour nodes,
///   two moves' nodes and the light ([`light_link`](crate::s4_small::light_link));
///   0xb..0xd: more static nodes, slot 237 twice, on to 0x14;
/// - 0x14..0x18: a static pair, on to 0x1e; 0x1e..0x21: the static tail
///   node, two statics with slot 325 between, a particle node, on to 0x28;
/// - 0x28..0x2c: an owned node, the bones, a particle node, on to 0x32;
/// - 0x32, 0x33: slots 719 and 306, a particle node and an effect node;
/// - 0x34: a fade word drops to `K`; after 67 calls the part freed, a
///   particle and an effect node;
/// - 0x35 (with the light's flag): the first effect's part freed (slot 713
///   unless its flag), a static leaves, on to 0x3c; 0x3c, 0x3d: slot 325,
///   the second's part (slot 713), a particle and an effect node;
/// - 0x3e: after 8 calls a static link and the static move's set-up;
///   0x3f: after 30 the fade; 0x40: after 58 the third part freed, on to
///   0x46: slot 213 (0609: a store); 0x47: the parts freed and the nodes
///   leave; 0x48: after 2 calls the fade and (0613) slot 239 and the
///   model's move (0609: its own state 0x49 after 75 calls);
/// - the last: after 50 calls (0609: 90) 10000, which in the same call
///   frees the summon's node and model and unlinks the task.
///
/// Returns `eax` as the original leaves it (the table's byte, the state,
/// the last call's).
pub fn summon_main_0613<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    summon_main(g, img, fr, false)
}

/// `magic_0609`'s copy of [`summon_main_0613`].
pub fn summon_main_0609<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    summon_main(g, img, fr, true)
}

/// How a state of [`summon_main`] ends.
enum Next {
    /// The counts go up; the call returns this `eax`.
    Exit(u32),
    /// The state was set: dispatch again.
    Again,
}

#[allow(clippy::too_many_lines)] // the original's states, in order
fn summon_main<G: Game>(g: &mut G, img: &Image, fr: &Frame, is_0609: bool) -> Ret {
    use crate::r25::abs_stores;
    use crate::r31::{append_0601, append_pair_0601};
    let f = fr.at;
    // 0613's offset `x` in this copy
    let o = |x: u32| {
        if !is_0609 {
            return x;
        }
        let d = SHIFTS_0609
            .iter()
            .rev()
            .find(|(from, _)| *from <= x)
            .map_or(0, |(_, d)| *d);
        x + d
    };
    let w = |x: u32| img.word(f + o(x));
    let k = |g: &G, x: u32| g.read::<f32>(w(x));
    let gw = |g: &G, x: u32| g.read::<u32>(w(x));
    let call = |x: u32| img.rel32_target(f + o(x));
    let slot = |g: &G, n: u32| slot_target(g, w(0x62), n);
    let abs = |g: &mut G, from: u32, to: u32| abs_stores(g, img, f, o(from), o(to));
    // 0613's offsets for the append helpers
    let app = |g: &mut G, old: u32, ops: [u32; 5]| append_0601(g, img, f, old, ops.map(o));
    let pair = |g: &mut G,
                old: u32,
                ops: [u32; 5],
                (first, run, c2, cw): (u32, (u32, u32), (u32, u32, u32), u32)| {
        append_pair_0601(
            g,
            img,
            f,
            old,
            ops.map(o),
            (
                o(first),
                (o(run.0), o(run.1)),
                (o(c2.0), o(c2.1), o(c2.2)),
                o(cw),
            ),
        );
    };
    // push ebp; and esp, -8; sub esp, 0x54; three pushes
    let base = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x60);
    let l = |x: u32| base.wrapping_add(x);
    let below = base.wrapping_sub(4);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    let at = |x: u32| r.wrapping_add(x);
    let count = |g: &G| g.read::<i32>(at(0x2c));
    let up = |g: &mut G| {
        let s = g.read::<u32>(at(0x18)).wrapping_add(1);
        g.write::<u32>(at(0x18), s);
    };
    let set = |g: &mut G, s: u32| g.write::<u32>(at(0x18), s);
    let reset = |g: &mut G| g.write::<u32>(at(0x2c), 0);
    // the frame's three node words (set once per call)
    for (i, x) in (0u32..).zip([0x30u32, 0x38, 0x40]) {
        g.write::<u32>(l(0xc + 4 * i), w(x));
    }
    let last = |g: &G| g.read::<u32>(w(0x18f));
    let leave = |g: &mut G, node: u32, eax: u32| crate::sa_misc::leave(g, node, eax);
    let free_unlink = |g: &mut G, xop: u32, c: u32, eax: u32| {
        g.write::<u32>(l(0x18), w(xop));
        let ff = Frame {
            at: call(c),
            esp: below,
            ecx: l(0x18),
            eax,
            ..Frame::default()
        };
        crate::nodes::free_then_unlink(g, img, &ff).eax
    };
    // the part's `+0x50` through the pointer, slot 713, unless its flag
    let part_slot = |g: &mut G, (fl, ptr, val, arg): (u32, u32, u32, u32), eax: u32| {
        if g.read::<u8>(w(fl)) & 4 != 0 {
            return eax;
        }
        let v = gw(g, val);
        g.write::<u32>(gw(g, ptr), v);
        // call width: 1
        g.call(slot(g, 713), 0, &[w(arg)]).eax
    };
    // a particle node from the frame record `[N, list, X, *G, Q, n]`
    let particle = |g: &mut G, ops: [u32; 5], n: u32, run: u32, c: u32| {
        let x = gw(g, ops[0]);
        let rec = l(0x44);
        for (i, v) in (0u32..).zip([w(ops[1]), w(ops[2]), w(ops[3]), x, w(ops[4]), n]) {
            g.write::<u32>(rec + 4 * i, v);
        }
        abs(g, run, run + 10);
        crate::sb_nodes::particle_node(g, img, call(c), rec)
    };
    // an effect node from the frame record, its vtable stored after
    let effect = |g: &mut G, ops: [u32; 5], n: u32, c: u32, vt: u32| {
        let rec = l(0x44);
        for (i, v) in (0u32..).zip([w(ops[0]), w(ops[1]), w(ops[2]), w(ops[3]), w(ops[4]), n]) {
            g.write::<u32>(rec + 4 * i, v);
        }
        crate::sb_nodes::effect_node(g, img, call(c), rec);
        let node = g.read::<u32>(rec);
        g.write::<u32>(node, w(vt));
        node
    };
    let mut state = g.read::<u32>(at(0x18));
    let max = img.word(f + o(0x4c)) & 0xff;
    let table = w(0x56);
    let eax = loop {
        if state > 0x2710 {
            break state;
        }
        if state == 0x2710 {
            // the end: the summon's node and model, the task's node
            let node = w(0x1452);
            abs_stores(g, img, f, o(0x1451) + 5, o(0x1474));
            leave(g, node, 0x2710);
            g.write::<u32>(l(0x18), node);
            let mf = Frame {
                at: call(0x1482),
                esp: below,
                ecx: l(0x18),
                ..Frame::default()
            };
            crate::sa_summon::model_setup(g, img, &mf);
            break crate::h6_setup::unlink_task(g, task);
        }
        if state > max {
            break state;
        }
        let idx = img.word(table + state) & 0xff;
        let next = match state {
            0 | 0xa..=0xd => {
                if state == 0 {
                    start_model_0613(g, img, f, &o, l(0));
                    set(g, 0xa);
                }
                if state <= 0xa {
                    scene_0613(g, img, f, &o, base);
                    up(g);
                    reset(g);
                }
                if state <= 0xb {
                    // a static pair (`edx` holds the first)
                    let old = last(g);
                    abs(g, 0x3e8, 0x419);
                    pair(
                        g,
                        old,
                        [0x41a, 0x41f, 0x425, 0x434, 0x43a],
                        (0x28, (0x440, 0x4b7), (0x4b9, 0x4bf, 0x4cb), 0x4e3),
                    );
                    abs(g, 0x4d7, 0x4e1);
                    abs(g, 0x4e7, 0x516);
                    up(g);
                }
                if state <= 0xc {
                    let old = last(g);
                    abs(g, 0x51e, 0x54f);
                    pair(
                        g,
                        old,
                        [0x550, 0x555, 0x55b, 0x56a, 0x570],
                        (0x30, (0x57e, 0x5eb), (0x57a, 0x5ed, 0x5f9), 0x614),
                    );
                    let m = gw(g, 0x61f);
                    abs(g, 0x623, 0x637);
                    // call width: 4
                    g.call(slot(g, 237), 0, &[m, gw(g, 0x60e), 0, w(0x604)]);
                    let old = last(g);
                    abs(g, 0x647, 0x68c);
                    app(g, old, [0x68d, 0x692, 0x698, 0x6a7, 0x6ae]);
                    abs(g, 0x6b2, 0x6eb);
                    up(g);
                }
                let old = last(g);
                abs(g, 0x6f3, 0x724);
                app(g, old, [0x725, 0x72a, 0x730, 0x73f, 0x74b]);
                let x = gw(g, 0x758);
                abs(g, 0x75c, 0x766);
                let m = gw(g, 0x768);
                abs(g, 0x76c, 0x776);
                // call width: 4
                g.call(slot(g, 237), 0, &[m, x, 0, w(0x750)]);
                let old = last(g);
                abs(g, 0x786, 0x7cb);
                app(g, old, [0x7cc, 0x7d1, 0x7d7, 0x7e6, 0x7ed]);
                abs(g, 0x7f1, 0x82a);
                set(g, 0x14);
                Next::Again
            }
            0x14..=0x18 => {
                if state == 0x14 {
                    reset(g);
                    set(g, 0x15);
                }
                if state <= 0x15 {
                    up(g);
                }
                if state <= 0x16 {
                    let old = last(g);
                    abs(g, 0x84b, 0x87c);
                    pair(
                        g,
                        old,
                        [0x87d, 0x882, 0x888, 0x897, 0x89d],
                        (0x38, (0x8ab, 0x922), (0x8a7, 0x924, 0x930), 0x948),
                    );
                    abs(g, 0x93c, 0x946);
                    abs(g, 0x94c, 0x97b);
                    up(g);
                    reset(g);
                }
                if state <= 0x17 {
                    up(g);
                }
                set(g, 0x1e);
                Next::Again
            }
            0x1e..=0x21 => {
                if state == 0x1e {
                    reset(g);
                    set(g, 0x1f);
                }
                if state <= 0x1f {
                    up(g);
                    reset(g);
                }
                if state <= 0x20 {
                    tail_0613(g, img, f, &o, base);
                    up(g);
                    reset(g);
                }
                set(g, 0x28);
                Next::Again
            }
            0x28..=0x2c => {
                if state == 0x28 {
                    bones_0613(g, img, f, &o, base);
                    if is_0609 {
                        abs_stores(g, img, f, 0xd69, 0xd73);
                    }
                    abs(g, 0xd69, 0xd73);
                    up(g);
                    reset(g);
                }
                if state <= 0x29 {
                    up(g);
                }
                if state <= 0x2a {
                    up(g);
                }
                if state <= 0x2b {
                    if is_0609 {
                        abs_stores(g, img, f, 0xd8d, 0xd97);
                    }
                    up(g);
                    reset(g);
                }
                set(g, 0x32);
                Next::Again
            }
            0x32 | 0x33 => {
                if state == 0x32 {
                    up(g);
                }
                // call width: 3
                let h = g.call(slot(g, 719), 0, &[w(0xe8a), 5, 0]).eax;
                // call width: 3
                g.call(slot(g, 306), 0, &[7, 1, h]);
                abs(g, 0xeb1, 0xed9);
                particle(g, [0xea6, 0xedd, 0xee5, 0xeed, 0xef9], 4, 0xf05, 0xf10);
                let node = effect(g, [0xf1c, 0xf24, 0xf2c, 0xf34, 0xf3c], 2, 0xf49, 0xf53);
                let x = gw(g, 0xf59) | 1;
                g.write::<u32>(w(0xf59), x);
                abs(g, 0xf5e, 0xf68);
                up(g);
                reset(g);
                Next::Exit(node)
            }
            0x34 => {
                let x = k(g, 0xf83);
                // comiss; jbe: unordered keeps the word
                if x > k(g, 0xf8a) {
                    let y = sub(x, k(g, 0xf94));
                    g.write::<f32>(w(0xf9c), y);
                }
                if count(g) < 0x43 {
                    Next::Exit(idx)
                } else {
                    let part = w(0xfab);
                    abs(g, 0xfaf, 0xfff);
                    crate::m0070::free_part(g, img, call(0x1000), part, idx);
                    // push 6; sub esp, 8; push N (the middle words the frame's)
                    let sp = base.wrapping_sub(0x10);
                    g.write::<u32>(sp, w(0x100f));
                    g.write::<u32>(sp + 0xc, 6);
                    let pf = Frame {
                        at: call(0x1014),
                        esp: sp.wrapping_sub(4),
                        ecx: w(0x100a),
                        ..Frame::default()
                    };
                    crate::s3_nodes::particle_node_with(g, img, &pf);
                    // push 5; push ecx (the slot's leftover, unread: 0); two words
                    for (i, v) in (0u32..).zip([w(0x1024), w(0x101f), 0, 5]) {
                        g.write::<u32>(sp + 4 * i, v);
                    }
                    let ef = Frame {
                        at: call(0x102e),
                        esp: sp.wrapping_sub(4),
                        ecx: w(0x1029),
                        ..Frame::default()
                    };
                    let eax = crate::s3_nodes::effect_node_with(g, img, &ef);
                    let x = gw(g, 0x1034) | 1;
                    g.write::<u32>(w(0x1034), x);
                    abs(g, 0x1039, 0x1043);
                    up(g);
                    Next::Exit(eax)
                }
            }
            0x35 => {
                if g.read::<u8>(w(0xd9b)) & 1 == 0 {
                    Next::Exit(idx)
                } else {
                    let eax = free_unlink(g, 0xdae, 0xdb3, idx);
                    let eax = part_slot(g, (0xdb9, 0xdc1, 0xdc7, 0xdd3), eax);
                    leave(g, w(0xde3), eax);
                    set(g, 0x3c);
                    Next::Again
                }
            }
            0x3c | 0x3d => {
                if state == 0x3c {
                    up(g);
                }
                abs(g, 0x105e, 0x1086);
                // call width: 2
                g.call(slot(g, 325), 0, &[w(0x1092), w(0x108d)]);
                abs(g, 0x10a2, 0x10ac);
                part_slot(g, (0x109d, 0x10af, 0x10b5, 0x10c1), 0);
                particle(
                    g,
                    [0x10d1, 0x10dd, 0x10e5, 0x10ed, 0x10f9],
                    0,
                    0x1105,
                    0x1110,
                );
                let node = effect(
                    g,
                    [0x111c, 0x1124, 0x112c, 0x1134, 0x113c],
                    3,
                    0x1149,
                    0x1153,
                );
                let x = gw(g, 0x1159) | 1;
                g.write::<u32>(w(0x1159), x);
                abs(g, 0x115e, 0x1168);
                up(g);
                reset(g);
                Next::Exit(node)
            }
            0x3e | 0x3f => {
                let mut eax = idx;
                if state == 0x3e && count(g) >= 8 {
                    // push ecx (the state: a word under the argument)
                    let sp = base.wrapping_sub(8);
                    g.write::<u32>(sp, w(0x118b));
                    g.write::<u32>(sp + 4, state);
                    let xf = Frame {
                        at: call(0x1190),
                        esp: sp.wrapping_sub(4),
                        ..Frame::default()
                    };
                    crate::s4_small::static_link_x(g, img, &xf);
                    let speed = u128::from(gw(g, 0x1198));
                    abs(g, 0x11a2, 0x1206);
                    let mf = Frame {
                        at: call(0x1207),
                        esp: below,
                        xmm: [0, 0, speed, speed],
                        ..Frame::default()
                    };
                    eax = crate::s4_small::move_setup_static(g, img, &mf).eax;
                    up(g);
                    reset(g);
                }
                if count(g) < 0x1e {
                    Next::Exit(eax)
                } else {
                    let level = u128::from(gw(g, 0x1223));
                    abs(g, 0x1227, 0x1277);
                    let ff = Frame {
                        at: call(0x1278),
                        esp: below,
                        xmm: [0, level, 0, 0],
                        ..Frame::default()
                    };
                    let eax = crate::s4_misc::fade_set(g, img, &ff).eax;
                    up(g);
                    Next::Exit(eax)
                }
            }
            0x40 => {
                if count(g) < 0x3a {
                    Next::Exit(idx)
                } else {
                    let eax = free_unlink(g, 0xe07, 0xe0c, idx);
                    part_slot(g, (0xe12, 0xe1a, 0xe20, 0xe2c), eax);
                    set(g, 0x46);
                    Next::Again
                }
            }
            0x46 => {
                let eax = if is_0609 {
                    // 0609: a store instead of slot 213
                    abs_stores(g, img, f, 0x12a0, 0x12b4);
                    idx
                } else {
                    let m = gw(g, 0x1295);
                    // call width: 2
                    let eax = g.call(slot(g, 213), 0, &[m, 0]).eax;
                    abs(g, 0x12a1, 0x12ab);
                    eax
                };
                up(g);
                reset(g);
                Next::Exit(eax)
            }
            0x47 => {
                if count(g) < 1 {
                    Next::Exit(idx)
                } else {
                    let mut eax = crate::l3_common::list_unlink_current(g, img, call(0x12d0), idx);
                    for (n, c, part, pc) in [
                        (0x12dcu32, 0x12e1u32, 0x12e6u32, 0x12ebu32),
                        (0x12f7, 0x12fc, 0x1301, 0x1306),
                    ] {
                        eax = free_unlink(g, n, c, eax);
                        eax = crate::m0070::free_part(g, img, call(pc), w(part), eax);
                    }
                    eax = leave(g, w(0x130b), eax);
                    eax = leave(g, w(0x1315), eax);
                    for (n, c) in [(0x131fu32, 0x1329u32), (0x132e, 0x1338)] {
                        // `ecx` the node as the leave keeps it
                        leave(g, w(n), eax);
                        eax = crate::sa_misc::register_one(g, img, call(c), w(n));
                    }
                    eax = leave(g, w(0x23), eax);
                    eax = leave(g, w(0x1344), eax);
                    up(g);
                    Next::Exit(eax)
                }
            }
            0x48 => {
                if count(g) < 2 {
                    Next::Exit(idx)
                } else {
                    let level = u128::from(gw(g, 0x136b));
                    abs(g, 0x136f, 0x13bf);
                    let ff = Frame {
                        at: call(0x13c0),
                        esp: below,
                        xmm: [0, level, 0, 0],
                        ..Frame::default()
                    };
                    let mut eax = crate::s4_misc::fade_set(g, img, &ff).eax;
                    if !is_0609 {
                        // push ecx (the slot's leftover, unread: 0)
                        g.write::<u32>(base.wrapping_sub(4), 0);
                        let cf = Frame {
                            at: call(0x13cb),
                            esp: base.wrapping_sub(8),
                            edx: 0xffff_fffb,
                            ..Frame::default()
                        };
                        crate::s4_small::call_239_edx(g, img, &cf);
                        eax = move_0613(g, img, f, &o, base);
                    }
                    up(g);
                    Next::Exit(eax)
                }
            }
            0x49 if is_0609 => {
                if count(g) < 0x4b {
                    Next::Exit(idx)
                } else {
                    let eax = move_0613(g, img, f, &o, base);
                    up(g);
                    Next::Exit(eax)
                }
            }
            _ if state == max => {
                let bound = img.word(f + o(0xe44) + 3) & 0xff;
                if count(g) < bound.cast_signed() {
                    Next::Exit(idx)
                } else {
                    set(g, 0x2710);
                    Next::Again
                }
            }
            _ => Next::Exit(idx),
        };
        match next {
            Next::Exit(eax) => break eax,
            Next::Again => state = g.read::<u32>(at(0x18)),
        }
    };
    for x in [0x28u32, 0x2c] {
        let v = g.read::<u32>(at(x)).wrapping_add(1);
        g.write::<u32>(at(x), v);
    }
    Ret::eax(eax)
}

/// [`summon_main`]'s state 0: the summon's model node from a frame group
/// (`l` the frame base): slot 294's answer as its flags, or 1 at the other
/// place when it answers 0xff.
fn start_model_0613<G: Game>(g: &mut G, img: &Image, f: u32, o: &dyn Fn(u32) -> u32, base: u32) {
    let w = |x: u32| img.word(f + o(x));
    let slot = |g: &G, n: u32| slot_target(g, w(0x62), n);
    let m = g.read::<u32>(w(0x68));
    // call width: 1
    let a = g.call(slot(g, 294), 0, &[m]).eax;
    let grp = if a == 0xff {
        let grp = base + 0x24;
        for (i, v) in (0u32..).zip([w(0x82), w(0x8a), 1]) {
            g.write::<u32>(grp + 4 * i, v);
        }
        g.write::<u8>(grp + 0xc, 0);
        grp
    } else {
        let grp = base + 0x34;
        g.write::<u32>(grp, w(0xb6));
        g.write::<u32>(grp + 4, w(0xbe));
        let m = g.read::<u32>(w(0xa8));
        // call width: 1
        let b = g.call(slot(g, 294), 0, &[m]).eax;
        g.write::<u32>(grp + 8, b);
        g.write::<u8>(grp + 0xc, 0);
        grp
    };
    crate::r25::abs_stores(g, img, f, o(0xd4), o(0xde));
    crate::sa_summon::model_node_init(g, img, img.rel32_target(f + o(0xdf)), grp);
}

/// [`summon_main`]'s state 0xa: the big node, the scene's static nodes and
/// their words, slots 676, the colour nodes, the moves' nodes and the light.
#[allow(clippy::too_many_lines)] // the original's steps, in order
fn scene_0613<G: Game>(g: &mut G, img: &Image, f: u32, o: &dyn Fn(u32) -> u32, base: u32) {
    use crate::r25::abs_stores;
    let w = |x: u32| img.word(f + o(x));
    let call = |x: u32| img.rel32_target(f + o(x));
    let slot = |g: &G, n: u32| slot_target(g, w(0x62), n);
    let abs = |g: &mut G, from: u32, to: u32| abs_stores(g, img, f, o(from), o(to));
    let grp = base + 0x44;
    for (i, x) in (0u32..).zip([0xf2u32, 0xfa, 0x102, 0x10a, 0x112]) {
        g.write::<u32>(grp + 4 * i, w(x));
    }
    crate::m0070::node_init_big(g, grp);
    let x = g.read::<u32>(w(0x11c));
    g.write::<u32>(w(0x121), x);
    // the first static under its own parent
    let old = g.read::<u32>(w(0x126));
    abs(g, 0x12a, 0x16f);
    crate::r31::append_0601(g, img, f, old, [0x170, 0x175, 0x17b, 0x18a, 0x196].map(o));
    // a pair under the scene's parent (`ebx` the first)
    let old = g.read::<u32>(w(0x18f));
    abs(g, 0x19a, 0x21b);
    crate::r31::append_pair_0601(
        g,
        img,
        f,
        old,
        [0x21c, 0x221, 0x227, 0x236, 0x23c].map(o),
        (
            o(0x23),
            (o(0x242), o(0x29b)),
            (o(0x29d), o(0x2a3), o(0x2af)),
            o(0x2bd),
        ),
    );
    abs(g, 0x2c8, 0x2e6);
    // call width: 3
    g.call(slot(g, 676), 0, &[w(0x2ef), 1, 0x60]);
    // call width: 3
    g.call(slot(g, 676), 0, &[w(0x308), 4, 0x40]);
    for (ops, run, c) in [
        (
            [0x319u32, 0x321, 0x329, 0x331, 0x339, 0x341, 0x349],
            0x34du32,
            0x358u32,
        ),
        (
            [0x364, 0x36c, 0x374, 0x37c, 0x384, 0x38c, 0x394],
            0x398,
            0x3a3,
        ),
    ] {
        for (i, x) in (0u32..).zip(ops) {
            g.write::<u32>(grp + 4 * i, w(x));
        }
        abs(g, run, run + 10);
        crate::sa_misc::colour_node_init(g, img, call(c), grp);
    }
    // push ecx (the slot's leftover, unread: 0) under the two moves' nodes
    let sp = base.wrapping_sub(4);
    g.write::<u32>(sp, 0);
    for (n, c) in [(0x3a9u32, 0x3aeu32), (0x3b3, 0x3b8)] {
        let lf = Frame {
            at: call(c),
            esp: sp.wrapping_sub(4),
            ecx: w(n),
            ..Frame::default()
        };
        crate::s4_small::link_node_2c(g, img, &lf);
    }
    // the light `(a, the node the link leaves in ecx, c)`
    let sp = base.wrapping_sub(0x10);
    for (i, v) in (0u32..).zip([w(0x3c3), w(0x3b3), w(0x3bd), 0]) {
        g.write::<u32>(sp + 4 * i, v);
    }
    let lf = Frame {
        at: call(0x3cd),
        esp: sp.wrapping_sub(4),
        ecx: w(0x3c8),
        ..Frame::default()
    };
    crate::s4_small::light_link(g, img, &lf);
}

/// [`summon_main`]'s state 0x20: the static tail node, two statics (slot
/// 325 between), a particle node.
fn tail_0613<G: Game>(g: &mut G, img: &Image, f: u32, o: &dyn Fn(u32) -> u32, base: u32) {
    use crate::r25::abs_stores;
    let w = |x: u32| img.word(f + o(x));
    let call = |x: u32| img.rel32_target(f + o(x));
    let slot = |g: &G, n: u32| slot_target(g, w(0x62), n);
    let abs = |g: &mut G, from: u32, to: u32| abs_stores(g, img, f, o(from), o(to));
    let last = |g: &G| g.read::<u32>(w(0x18f));
    let tf = Frame {
        at: call(0x9ac),
        esp: base.wrapping_sub(4),
        ..Frame::default()
    };
    crate::s4_small::static_link_tail(g, img, &tf);
    let old = last(g);
    abs(g, 0x9b5, 0xa36);
    crate::r31::append_0601(g, img, f, old, [0xa37, 0xa3c, 0xa42, 0xa51, 0xa5d].map(o));
    abs(g, 0xa61, 0xa75);
    // call width: 2
    g.call(slot(g, 325), 0, &[w(0xa81), w(0xa7c)]);
    let old = last(g);
    abs(g, 0xa8f, 0xac0);
    crate::r31::append_pair_0601(
        g,
        img,
        f,
        old,
        [0xac1, 0xac6, 0xacc, 0xadb, 0xae1].map(o),
        (
            o(0x40),
            (o(0xaef), o(0xb66)),
            (o(0xaeb), o(0xb68), o(0xb74)),
            o(0xb82),
        ),
    );
    let x = g.read::<u32>(w(0xb87));
    abs(g, 0xb8f, 0xbc8);
    let rec = base + 0x44;
    for (i, v) in (0u32..).zip([w(0xbcc), w(0xbd4), w(0xbdc), x, w(0xbe8), 0xa]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    abs(g, 0xbf4, 0xbfe);
    crate::sb_nodes::particle_node(g, img, call(0xbff), rec);
    abs(g, 0xc03, 0xc3f);
}

/// [`summon_main`]'s state 0x28: a static, the owned node, the bones, a
/// particle node.
fn bones_0613<G: Game>(g: &mut G, img: &Image, f: u32, o: &dyn Fn(u32) -> u32, base: u32) {
    use crate::r25::abs_stores;
    let w = |x: u32| img.word(f + o(x));
    let call = |x: u32| img.rel32_target(f + o(x));
    let abs = |g: &mut G, from: u32, to: u32| abs_stores(g, img, f, o(from), o(to));
    let old = g.read::<u32>(w(0x18f));
    abs(g, 0xc5a, 0xc8b);
    crate::r31::append_0601(g, img, f, old, [0xc8c, 0xc91, 0xc97, 0xca6, 0xcad].map(o));
    abs(g, 0xccf, 0xd0b);
    let sp = base.wrapping_sub(0x10);
    for (i, x) in (0u32..).zip([0xcc1u32, 0xcbc, 0xcb7, 0xcb2]) {
        g.write::<u32>(sp + 4 * i, w(x));
    }
    let of = Frame {
        at: call(0xd0c),
        esp: sp.wrapping_sub(4),
        ecx: w(0xccb),
        edx: w(0xcc6),
        ..Frame::default()
    };
    crate::sa_misc::node_append_owned(g, img, &of);
    // the bones' cursor: the owned node (`ecx` as the append leaves it)
    let cur = base + 0x1c;
    g.write::<u32>(cur, w(0xccb));
    g.write::<u8>(cur + 5, 0xff);
    let bf = Frame {
        at: call(0xd21),
        esp: base.wrapping_sub(4),
        ecx: cur,
        ..Frame::default()
    };
    crate::sa_summon::bone_plain(g, img, &bf);
    let x = g.read::<u32>(w(0xd26));
    let rec = base + 0x44;
    for (i, v) in (0u32..).zip([w(0xd32), w(0xd3a), w(0xd42), x, w(0xd4e), 3]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    abs(g, 0xd5a, 0xd64);
    crate::sb_nodes::particle_node(g, img, call(0xd65), rec);
}

/// The model's move (`[N, 0, K, 1.0]` on the frame) after its colour
/// words. Returns the move's time.
fn move_0613<G: Game>(g: &mut G, img: &Image, f: u32, o: &dyn Fn(u32) -> u32, base: u32) -> u32 {
    let w = |x: u32| img.word(f + o(x));
    crate::r25::abs_stores(g, img, f, o(0x13d6), o(0x141c));
    let args = base + 0x34;
    for (i, v) in (0u32..).zip([w(0x1420), 0, img.word(f + o(0x142c) + 4), ONE]) {
        g.write::<u32>(args + 4 * i, v);
    }
    crate::sa_misc::move_start(g, args)
}

/// [`summon_main_0613`] (the `rel32`s: its callees in order).
pub const SUMMON_MAIN_0613: Sig = Sig {
    len: 5502,
    hash: 0x75f1_82be_ec8b_06fd,
    calls: &[
        (0xdf, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0x117, Callee::Sig(&crate::m0070::NODE_INIT_BIG)),
        (0x358, Callee::Sig(&crate::sa_misc::COLOUR_NODE_INIT)),
        (0x3a3, Callee::Sig(&crate::sa_misc::COLOUR_NODE_INIT)),
        (0x3ae, Callee::Sig(&crate::s4_small::LINK_NODE_2C)),
        (0x3b8, Callee::Sig(&crate::s4_small::LINK_NODE_2C)),
        (0x3cd, Callee::Sig(&crate::s4_small::LIGHT_LINK)),
        (0x9ac, Callee::Sig(&crate::s4_small::STATIC_LINK_TAIL)),
        (0xbff, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0xd0c, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0xd21, Callee::Sig(&crate::sa_summon::BONE)),
        (0xd65, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0xdb3, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0xde8, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0xe0c, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0xf10, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0xf49, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
        (0x1000, Callee::Sig(&crate::m0070::FREE_PART)),
        (0x1014, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x102e, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0x1110, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x1149, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
        (0x1190, Callee::Sig(&crate::s4_small::STATIC_LINK_X)),
        (0x1207, Callee::Sig(&crate::s4_small::MOVE_SETUP_STATIC)),
        (0x1278, Callee::Sig(&crate::s4_misc::FADE_SET)),
        (0x12d0, Callee::Sig(&crate::l3_common::LIST_UNLINK_CURRENT)),
        (0x12e1, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0x12eb, Callee::Sig(&crate::m0070::FREE_PART)),
        (0x12fc, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0x1306, Callee::Sig(&crate::m0070::FREE_PART)),
        (0x1310, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x131a, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x1324, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x1329, Callee::Sig(&crate::sa_misc::REGISTER_ONE)),
        (0x1333, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x1338, Callee::Sig(&crate::sa_misc::REGISTER_ONE)),
        (0x133f, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x1349, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x13c0, Callee::Sig(&crate::s4_misc::FADE_SET)),
        (0x13cb, Callee::Sig(&crate::s4_small::CALL_239_EDX)),
        (0x143d, Callee::Sig(&crate::sa_misc::MOVE_START)),
        (0x1475, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x1482, Callee::Sig(&crate::sa_summon::MODEL_SETUP)),
    ],
    imports: &[],
};

/// [`summon_main_0609`] (the `rel32`s: its callees in order).
pub const SUMMON_MAIN_0609: Sig = Sig {
    len: 5535,
    hash: 0x06f2_acea_ce73_95b5,
    calls: &[
        (0xdf, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0x117, Callee::Sig(&crate::m0070::NODE_INIT_BIG)),
        (0x358, Callee::Sig(&crate::sa_misc::COLOUR_NODE_INIT)),
        (0x3a3, Callee::Sig(&crate::sa_misc::COLOUR_NODE_INIT)),
        (0x3ae, Callee::Sig(&crate::s4_small::LINK_NODE_2C)),
        (0x3b8, Callee::Sig(&crate::s4_small::LINK_NODE_2C)),
        (0x3cd, Callee::Sig(&crate::s4_small::LIGHT_LINK)),
        (0x9ac, Callee::Sig(&crate::s4_small::STATIC_LINK_TAIL)),
        (0xbff, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0xd0c, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0xd21, Callee::Sig(&crate::sa_summon::BONE)),
        (0xd65, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0xdc7, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0xdfc, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0xe20, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0xf24, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0xf5d, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
        (0x1014, Callee::Sig(&crate::m0070::FREE_PART)),
        (0x1028, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x1042, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0x1124, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x115d, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
        (0x11a4, Callee::Sig(&crate::s4_small::STATIC_LINK_X)),
        (0x121b, Callee::Sig(&crate::s4_small::MOVE_SETUP_STATIC)),
        (0x128c, Callee::Sig(&crate::s4_misc::FADE_SET)),
        (0x12d6, Callee::Sig(&crate::l3_common::LIST_UNLINK_CURRENT)),
        (0x12e7, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0x12f1, Callee::Sig(&crate::m0070::FREE_PART)),
        (0x1302, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0x130c, Callee::Sig(&crate::m0070::FREE_PART)),
        (0x1316, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x1320, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x132a, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x132f, Callee::Sig(&crate::sa_misc::REGISTER_ONE)),
        (0x1339, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x133e, Callee::Sig(&crate::sa_misc::REGISTER_ONE)),
        (0x1345, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x134f, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x13c6, Callee::Sig(&crate::s4_misc::FADE_SET)),
        (0x144f, Callee::Sig(&crate::sa_misc::MOVE_START)),
        (0x1491, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x149e, Callee::Sig(&crate::sa_summon::MODEL_SETUP)),
    ],
    imports: &[],
};

/// `magic_0577`'s spark task (0x1000b530, 3558 bytes; a task callback
/// `(A)`, cdecl; an aligned /GS frame below three pushes; the record `R =
/// A[4]`, its state `R + 0x60`, its wait `R + 0x64` counting down at every
/// exit). A state from the byte and jump tables (states without code and
/// those past the table only count down); some go on into others in the
/// same call:
/// - 0, 1: after a wait of 67 the model's nodes (a small node, appends and
///   owned nodes, a particle and an effect node), state 2 with a wait of
///   30; 2: after the wait, 0x14;
/// - 0x14: two nodes, an owned node, a particle node, the character's
///   fade ([`chr_fade`](crate::l3_common::chr_fade)), the colour words
///   kept and cleared, on into 0x15; 0x15: the fade grows three steps
///   deep, the move's vector (slots 234, 233, 248, 325), at 1.0 state
///   0x16, the colour words scaled by it;
/// - 0x16: the vector again; past `K` the colour node
///   ([`colour`](crate::m0577::colour)), slot 325, on to 0x32;
///   0x32: nodes, slots 229 and 248, an owned and a particle node, the
///   last spark's words, on into 0x33; 0x33: past a wait of 10 the parts
///   freed and a particle node; then (0x34) the fade word shrinks, at
///   -32 the next state;
/// - 0x35: after the wait, 0x1e; 0x1e, 0x1f: after a wait of 10 the
///   colour node, state 0x20; 0x20: after the wait the part freed, on to
///   0xa: the bones ([`bone_0577`](crate::r27::bone_0577)), slot 325,
///   words, the part (slot 713 unless its flag) and a particle node, the
///   centre ([`centre`](crate::m0577::centre)), slot 234, state 0xb;
/// - 0xb: the move grows by `K` (slots 233, 248); past `K'` on to 0x3c:
///   the part freed, the matrix (slots 210, 230, 233 three times, 327),
///   20 sparks ([`spark_setup`](crate::m0577::spark_setup)), the last
///   spark's words, on into 0x3d: when every spark's flags (`& 0x4e20`)
///   agree, 10000, which in the same call has the nodes leave, slot 21 for
///   each of the DLL's ten words, the model set up and the task unlinked
///   (state 20000).
///
/// Returns `eax` as the original leaves it.
#[allow(clippy::too_many_lines)] // the original's states, in order
pub fn spark_task_0577<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, mul};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x7b), n);
    let call = |o: u32| img.rel32_target(f + o);
    // push ebp; and esp, -8; sub esp, 0x24; three pushes
    let e0 = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x24);
    g.write::<u32>(e0 + 0x20, g.read::<u32>(w(0xa)) ^ e0);
    let base = e0.wrapping_sub(0xc);
    let l = |o: u32| base.wrapping_add(o);
    let below = base.wrapping_sub(4);
    let task = fr.arg(g, 0);
    g.write::<u32>(l(0x10), task);
    let r = g.read::<u32>(task + 4);
    let at = |o: u32| r.wrapping_add(o);
    let rf = |g: &G, o: u32| g.read::<f32>(at(o));
    let wait = |g: &G| g.read::<i32>(at(0x64));
    let set = |g: &mut G, s: u32| g.write::<u32>(at(0x60), s);
    let up = |g: &mut G| {
        let s = g.read::<u32>(at(0x60)).wrapping_add(1);
        g.write::<u32>(at(0x60), s);
    };
    let list = w(0x402);
    let ctx = Ctx { f, base, r };
    let mut state = g.read::<u32>(at(0x60));
    let table = w(0x42);
    let eax = loop {
        if state > 0x2710 {
            break state;
        }
        if state == 0x2710 {
            break end_0577(g, img, &ctx);
        }
        if state > 0x3d {
            break state;
        }
        let idx = img.word(table + state) & 0xff;
        let next: Result<u32, ()> = match state {
            0 | 1 => {
                if state == 0 {
                    up(g);
                    g.write::<u32>(at(0x64), 0x43);
                }
                if wait(g) > 0 {
                    Ok(idx)
                } else {
                    Ok(model_0577(g, img, &ctx, list))
                }
            }
            2 | 0x35 => {
                if wait(g) > 0 {
                    Ok(idx)
                } else {
                    set(g, if state == 2 { 0x14 } else { 0x1e });
                    Err(())
                }
            }
            0x14 | 0x15 => {
                if state == 0x14 {
                    fade_0577(g, img, &ctx);
                }
                let x = add(rf(g, 0x80), k(g, 0x65e));
                g.write::<f32>(at(0x80), x);
                let y = add(x, rf(g, 0x70));
                g.write::<f32>(at(0x70), y);
                let z = add(y, rf(g, 0x68));
                g.write::<f32>(at(0x68), z);
                vector_0577(g, img, &ctx, (0x67d, 0x6ac, 0x6c1));
                let eax = g.call(slot(g, 325), 0, &[w(0x6ed), at(0x49c)]).eax;
                let x = add(rf(g, 0x84), k(g, 0x6ff));
                let y = add(rf(g, 0x74), x);
                g.write::<f32>(at(0x84), x);
                g.write::<f32>(at(0x74), y);
                let z = add(y, rf(g, 0x6c));
                g.write::<f32>(at(0x6c), z);
                // comiss; jb: below or unordered keeps the state
                if z >= k(g, 0x724) {
                    up(g);
                    g.write::<u32>(at(0x6c), ONE);
                }
                let s = rf(g, 0x6c);
                let x = mul(rf(g, 0x78), s);
                g.write::<f32>(at(0x1a8), x);
                let s = rf(g, 0x6c);
                let x = mul(rf(g, 0x7c), s);
                g.write::<f32>(at(0x200), x);
                Ok(eax)
            }
            0x16 => {
                let x = add(rf(g, 0x70), rf(g, 0x68));
                g.write::<f32>(at(0x68), x);
                vector_0577(g, img, &ctx, (0x7b, 0xa0, 0xb5));
                // call width: 2
                let eax = g.call(slot(g, 325), 0, &[w(0xe1), at(0x49c)]).eax;
                if rf(g, 0x68) < k(g, 0xf2) || rf(g, 0x68).is_nan() {
                    Ok(eax)
                } else {
                    // push ecx (the slot's leftover, unread: 0)
                    g.write::<u32>(below, 0);
                    let c = l(0x1c);
                    for (i, v) in (0u32..).zip([ONE, ONE, ONE, 0]) {
                        g.write::<u32>(c + 4 * i, v);
                    }
                    let cf = Frame {
                        at: call(0x128),
                        esp: below.wrapping_sub(4),
                        ecx: at(0x9f0),
                        edx: c,
                        ..Frame::default()
                    };
                    crate::m0577::colour(g, img, &cf);
                    // call width: 2
                    g.call(slot(g, 325), 0, &[at(0x49c), at(0x50)]);
                    set(g, 0x32);
                    Err(())
                }
            }
            0x32..=0x34 => {
                if state == 0x32 {
                    lights_0577(g, img, &ctx);
                    up(g);
                    g.write::<u32>(at(0x64), 0x1e);
                }
                if state <= 0x33 && wait(g) <= 0xa {
                    parts_0577(g, img, &ctx);
                    up(g);
                }
                // the fade word shrinks; at -32 the next state
                let v = g.read::<i32>(at(0x304));
                let x = mul(crate::sb_sse::cvt(v.cast_unsigned()), k(g, 0x95a));
                let n = crate::sb_sse::cvtt(x);
                g.write::<i32>(at(0x304), n);
                if n >= -32 {
                    g.write::<u32>(at(0x304), 0);
                    up(g);
                }
                Ok(n.cast_unsigned())
            }
            0x1e | 0x1f => {
                if state == 0x1e {
                    up(g);
                    g.write::<u32>(at(0x64), 0x1f);
                }
                if wait(g) > 0xa {
                    Ok(idx)
                } else {
                    // push ecx (the slot's leftover, unread: 0)
                    g.write::<u32>(below, 0);
                    let c = l(0x1c);
                    for (i, v) in (0u32..).zip([ONE, ONE, ONE, 0]) {
                        g.write::<u32>(c + 4 * i, v);
                    }
                    let cf = Frame {
                        at: call(0x9d3),
                        esp: below.wrapping_sub(4),
                        ecx: at(0xbf0),
                        edx: c,
                        ..Frame::default()
                    };
                    let eax = crate::m0577::colour(g, img, &cf);
                    up(g);
                    Ok(eax)
                }
            }
            0x20 => {
                if wait(g) > 0 {
                    Ok(idx)
                } else {
                    free_0577(g, img, &ctx, 0xbf0, (0x17f, 0x186), true);
                    set(g, 0xa);
                    Err(())
                }
            }
            0xa => Ok(bones_0577(g, img, &ctx)),
            0xb | 0x3c | 0x3d => {
                let mut eax = idx;
                let mut s = state;
                if s == 0xb {
                    let x = add(rf(g, 0x68), k(g, 0x19f));
                    // push ecx; movss [esp], x: the slot's leftover
                    // replaced by the scale
                    g.write::<f32>(at(0x68), x);
                    let v = l(0x1c);
                    // call width: 3
                    g.call(slot(g, 233), 0, &[v, at(0x50), x.to_bits()]);
                    // call width: 3
                    eax = g.call(slot(g, 248), 0, &[at(0x49c), at(0x40), v]).eax;
                    g.write::<u32>(at(0x4a8), ONE);
                    if rf(g, 0x68) < k(g, 0x1f9) || rf(g, 0x68).is_nan() {
                        s = 0;
                    } else {
                        // call width: 3
                        g.call(slot(g, 248), 0, &[at(0x49c), at(0x40), at(0x50)]);
                        g.write::<u32>(at(0x4a8), ONE);
                        set(g, 0x3c);
                        s = 0x3c;
                    }
                }
                if s == 0x3c {
                    matrix_0577(g, img, &ctx);
                    up(g);
                    s = 0x3d;
                }
                if s == 0x3d {
                    // every spark's flags, five at a time
                    let mut all = 0x4e20u32;
                    for i in 0..4u32 {
                        let mut x = 0xffff_ffffu32;
                        for j in [0u32, 1, 2, 4, 3] {
                            x &= g.read::<u32>(at(0xd4c + 0xe4 * (5 * i + j)));
                        }
                        eax = x & 0x4e20;
                        all &= eax;
                    }
                    if all == 0 {
                        Ok(eax)
                    } else {
                        set(g, 0x2710);
                        Err(())
                    }
                } else {
                    Ok(eax)
                }
            }
            _ => Ok(idx),
        };
        match next {
            Ok(eax) => break eax,
            Err(()) => state = g.read::<u32>(at(0x60)),
        }
    };
    let x = g.read::<u32>(at(0x64)).wrapping_sub(1);
    g.write::<u32>(at(0x64), x);
    Ret::eax(eax)
}

/// The frame and record of [`spark_task_0577`].
struct Ctx {
    f: u32,
    base: u32,
    r: u32,
}

/// Slots 234 `(V, R + 0x50, R + 0x40)` and 233 `(V, V, R[0x68])` and 248
/// `(R + 0x49c, R + 0x40, V)` with `V` the frame's vector (`w` 0), `ops`
/// the imports' operands.
fn vector_0577<G: Game>(g: &mut G, img: &Image, c: &Ctx, ops: (u32, u32, u32)) {
    let w = |o: u32| img.word(c.f + o);
    let at = |o: u32| c.r.wrapping_add(o);
    let v = c.base + 0x1c;
    // call width: 3
    g.call(slot_target(g, w(ops.0), 234), 0, &[v, at(0x50), at(0x40)]);
    let s = g.read::<u32>(at(0x68));
    // call width: 3
    g.call(slot_target(g, w(ops.1), 233), 0, &[v, v, s]);
    g.write::<u32>(v + 0xc, 0);
    // call width: 3
    g.call(slot_target(g, w(ops.2), 248), 0, &[at(0x49c), at(0x40), v]);
}

/// The part `R + o` freed: unless its flag (`skip` and byte `+0xe8` bit 2)
/// the node is freed and unlinked (the frame group at `E + 0xc`), then
/// [`free_part`](crate::m0070::free_part) (the calls at `c`).
fn free_0577<G: Game>(g: &mut G, img: &Image, c: &Ctx, o: u32, calls: (u32, u32), always: bool) {
    let part = c.r.wrapping_add(o);
    if always || g.read::<u8>(part + 0xe8) & 4 == 0 {
        let grp = c.base + 0xc;
        g.write::<u32>(grp, part);
        let ff = Frame {
            at: img.rel32_target(c.f + calls.0),
            esp: c.base.wrapping_sub(4),
            ecx: grp,
            ..Frame::default()
        };
        crate::nodes::free_then_unlink(g, img, &ff);
    }
    crate::m0070::free_part(g, img, img.rel32_target(c.f + calls.1), part, 0);
}

/// [`particle_node_with`](crate::s3_nodes::particle_node_with) at the
/// call `at` for the node `R + o` with `(R + a, -, -, n)` (the middle
/// words the frame's).
fn particle_0577<G: Game>(
    g: &mut G,
    img: &Image,
    c: &Ctx,
    at: u32,
    (o, a, n): (u32, u32, u32),
    sp: u32,
) -> u32 {
    g.write::<u32>(sp, c.r.wrapping_add(a));
    g.write::<u32>(sp + 0xc, n);
    let pf = Frame {
        at: img.rel32_target(c.f + at),
        esp: sp.wrapping_sub(4),
        ecx: c.r.wrapping_add(o),
        ..Frame::default()
    };
    crate::s3_nodes::particle_node_with(g, img, &pf)
}

/// [`node_append_owned`](crate::sa_misc::node_append_owned) of `R + o`
/// under `parent` with the four words (the stack below the frame).
fn owned_0577<G: Game>(
    g: &mut G,
    img: &Image,
    c: &Ctx,
    at: u32,
    o: u32,
    parent: u32,
    words: [u32; 4],
) {
    let sp = c.base.wrapping_sub(0x10);
    for (i, v) in (0u32..).zip(words) {
        g.write::<u32>(sp + 4 * i, v);
    }
    let of = Frame {
        at: img.rel32_target(c.f + at),
        esp: sp.wrapping_sub(4),
        ecx: c.r.wrapping_add(o),
        edx: parent,
        ..Frame::default()
    };
    crate::sa_misc::node_append_owned(g, img, &of);
}

/// [`spark_task_0577`]'s state 1 past its wait: the model's nodes.
/// Returns the effect node's `eax`.
fn model_0577<G: Game>(g: &mut G, img: &Image, c: &Ctx, list: u32) -> u32 {
    let w = |o: u32| img.word(c.f + o);
    let call = |o: u32| img.rel32_target(c.f + o);
    let at = |o: u32| c.r.wrapping_add(o);
    let node = at(0x140);
    g.write::<u32>(c.base.wrapping_sub(4), at(0x30c));
    crate::m0070::node_init_small(g, img, call(0x409), node, list, at(0x30c));
    for o in [0x16cu32, 0x170, 0x174] {
        g.write::<u32>(at(o), 0);
    }
    crate::sa_misc::node_append(g, img, call(0x435), at(0x2b4), list, 0);
    let x = g.read::<u32>(at(0x2a4));
    g.write::<u32>(at(0x2d0), x);
    let x = g.read::<u32>(at(0x2a8)).wrapping_add(0x4000);
    g.write::<u32>(at(0x2d4), x);
    let x = g.read::<u32>(at(0x2ac));
    g.write::<u32>(at(0x2d8), x);
    // `(T, node, the appended node as `ecx` keeps it, U)`
    owned_0577(
        g,
        img,
        c,
        0x475,
        0x4bc,
        list,
        [w(0x464), node, at(0x2b4), w(0x44c)],
    );
    let b = g.read::<u8>(at(0x454));
    g.write::<u8>(at(0x604), b);
    crate::sb_misc::node_append(g, img, call(0x48c), at(0x25c), list, 0);
    for (o, v) in [
        (0x278u32, ONE),
        (0x27c, ONE),
        (0x280, ONE),
        (0x284, 0x3dcc_cccd),
    ] {
        g.write::<u32>(at(o), v);
    }
    // the second owned node's words go under the first's (eight words)
    let sp = c.base.wrapping_sub(0x10);
    for (i, v) in (0u32..).zip([w(0x464), node, at(0x2b4), w(0x44c)]) {
        g.write::<u32>(sp + 4 * i, v);
    }
    let sp2 = c.base.wrapping_sub(0x20);
    for (i, v) in (0u32..).zip([w(0x49e), node, w(0x492), at(0x25c)]) {
        g.write::<u32>(sp2 + 4 * i, v);
    }
    let of = Frame {
        at: call(0x4cd),
        esp: sp2.wrapping_sub(4),
        ecx: at(0x8a0),
        edx: list,
        ..Frame::default()
    };
    crate::sa_misc::node_append_owned(g, img, &of);
    let sp = c.base.wrapping_sub(0x10);
    particle_0577(g, img, c, 0x4e1, (0xbf0, 0x8a0, 2), sp);
    // push 1; push ecx; push node; push ecx (the slot's leftovers, unread: 0)
    for (i, v) in (0u32..).zip([0, node, 0, 1]) {
        g.write::<u32>(sp + 4 * i, v);
    }
    let ef = Frame {
        at: call(0x4f4),
        esp: sp.wrapping_sub(4),
        ecx: at(0xcf0),
        ..Frame::default()
    };
    let eax = crate::s4_fam446::effect_node_a(g, img, &ef).eax;
    let x = g.read::<u32>(at(0xd1c)) | 1;
    g.write::<u32>(at(0xd1c), x);
    g.write::<u32>(at(0xd20), 0);
    let s = g.read::<u32>(at(0x60)).wrapping_add(1);
    g.write::<u32>(at(0x60), s);
    g.write::<u32>(at(0x64), 0x1e);
    eax
}

/// [`spark_task_0577`]'s state 0x14: two nodes, an owned node, a particle
/// node, the character's fade, the colour words kept, the fade words
/// cleared, state 0x15.
fn fade_0577<G: Game>(g: &mut G, img: &Image, c: &Ctx) {
    let w = |o: u32| img.word(c.f + o);
    let call = |o: u32| img.rel32_target(c.f + o);
    let at = |o: u32| c.r.wrapping_add(o);
    let list = w(0x532);
    crate::sb_misc::node_append(g, img, call(0x539), at(0x1ac), list, 0);
    for (o, v) in [(0x1c8u32, 0), (0x1cc, 0), (0x1d0, 0), (0x1d4, ONE)] {
        g.write::<u32>(at(o), v);
    }
    crate::sb_misc::node_append(g, img, call(0x56c), at(0x1d8), list, 0);
    for (o, v) in [
        (0x1f4u32, ONE),
        (0x1f8, ONE),
        (0x1fc, ONE),
        (0x200, 0x3f00_0000),
    ] {
        g.write::<u32>(at(o), v);
    }
    owned_0577(
        g,
        img,
        c,
        0x5af,
        0x608,
        list,
        [at(0x4bc), at(0x1ac), w(0x572), at(0x1d8)],
    );
    let b = g.read::<u8>(at(0x604));
    g.write::<u8>(at(0x750), b);
    particle_0577(
        g,
        img,
        c,
        0x5cf,
        (0x9f0, 0x608, 1),
        c.base.wrapping_sub(0x10),
    );
    let grp = c.base + 0x10;
    for (i, v) in (0u32..).zip([at(0xa8), ONE, 0xa]) {
        g.write::<u32>(grp + 4 * i, v);
    }
    crate::l3_common::chr_fade(g, img, call(0x5f5), grp);
    let a = g.read::<u32>(at(0x1a8));
    g.write::<u32>(at(0x78), a);
    let b = g.read::<u32>(at(0x200));
    g.write::<u32>(at(0x1a8), 0);
    g.write::<u32>(at(0x7c), b);
    g.write::<u32>(at(0x200), 0);
    let s = g.read::<u32>(at(0x60)).wrapping_add(1);
    g.write::<u32>(at(0x60), s);
    for o in [0x68u32, 0x70, 0x80, 0x6c, 0x74, 0x84] {
        g.write::<u32>(at(o), 0);
    }
}

/// [`spark_task_0577`]'s state 0x32: a node, the light's place (slots 229
/// and 248), a node and an owned node, a particle node, the last spark's
/// words.
fn lights_0577<G: Game>(g: &mut G, img: &Image, c: &Ctx) {
    let w = |o: u32| img.word(c.f + o);
    let call = |o: u32| img.rel32_target(c.f + o);
    let at = |o: u32| c.r.wrapping_add(o);
    let x = g.read::<u32>(at(0x88));
    let list = w(0x77f);
    g.write::<u32>(at(0x458), x);
    g.write::<u8>(at(0x455), 1);
    g.write::<u32>(at(0x4b4), 0x3e80_0000);
    crate::sb_misc::node_append(g, img, call(0x79d), at(0x204), list, 0);
    let v = c.base + 0x1c;
    for (i, x) in (0u32..).zip([0, 0, 0xc2c8_0000, 0]) {
        g.write::<u32>(v + 4 * i, x);
    }
    // call width: 3
    g.call(slot_target(g, w(0x7b3), 229), 0, &[v, at(0x6a4), v]);
    // call width: 3
    g.call(slot_target(g, w(0x7f6), 248), 0, &[at(0x220), at(0x15c), v]);
    crate::sb_misc::node_append(g, img, call(0x811), at(0x230), w(0x80c), 0);
    for (o, x) in [
        (0x24cu32, ONE),
        (0x250, ONE),
        (0x254, ONE),
        (0x258, 0x3dcc_cccd),
    ] {
        g.write::<u32>(at(o), x);
    }
    owned_0577(
        g,
        img,
        c,
        0x854,
        0x754,
        w(0x80c),
        [w(0x825), at(0x204), at(0x2b4), at(0x230)],
    );
    particle_0577(
        g,
        img,
        c,
        0x868,
        (0xaf0, 0x754, 0xa),
        c.base.wrapping_sub(0x10),
    );
    let s = g.read::<u32>(at(0xd28));
    let fl = g.read::<u8>(s + 0x40) | 1;
    g.write::<u8>(s + 0x40, fl);
    for (o, x) in [
        (0x1cu32, 0x4040_0000u32),
        (0x20, 0x40a0_0000),
        (0x24, 0x4040_0000),
        (0x28, 0x40a0_0000),
        (0x30, 0x4170_0000),
        (0x2c, 0x4170_0000),
        (0x38, ONE),
        (0x3c, 0),
    ] {
        g.write::<u32>(s + o, x);
    }
}

/// [`spark_task_0577`]'s state 0x33 past its wait: the parts freed (the
/// second unless its flag), a particle node.
fn parts_0577<G: Game>(g: &mut G, img: &Image, c: &Ctx) {
    let at = |o: u32| c.r.wrapping_add(o);
    free_0577(g, img, c, 0x9f0, (0x8d4, 0x8db), true);
    for (o, x) in [
        (0x278u32, ONE),
        (0x27c, ONE),
        (0x280, ONE),
        (0x284, 0x3dcc_cccd),
    ] {
        g.write::<u32>(at(o), x);
    }
    free_0577(g, img, c, 0xbf0, (0x91f, 0x92a), false);
    particle_0577(
        g,
        img,
        c,
        0x941,
        (0xbf0, 0x8a0, 0xb),
        c.base.wrapping_sub(0x10),
    );
}

/// [`spark_task_0577`]'s state 0xa: the bones, slot 325, the part and a
/// particle node, the centre and slot 234; state 0xb. Returns slot 234's
/// `eax`.
fn bones_0577<G: Game>(g: &mut G, img: &Image, c: &Ctx) -> u32 {
    let w = |o: u32| img.word(c.f + o);
    let call = |o: u32| img.rel32_target(c.f + o);
    let at = |o: u32| c.r.wrapping_add(o);
    let slot = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let b = at(0x30c);
    let cur = c.base + 0x10;
    g.write::<u32>(cur, b);
    g.write::<u32>(b + 0x14c, w(0xa02));
    g.write::<u8>(b + 0x149, 1);
    g.write::<u32>(b + 0x1a8, ONE);
    let mark = g.read::<u8>(w(0xa18));
    g.write::<u8>(cur + 5, mark);
    crate::r27::bone_0577(g, img, call(0xa25), c.base.wrapping_sub(4), cur);
    // call width: 2
    g.call(slot(g, 0xa38, 325), 0, &[at(0x220), at(0x15c)]);
    for (o, x) in [
        (0x24cu32, ONE),
        (0x250, ONE),
        (0x254, ONE),
        (0x258, 0x3dcc_cccd),
        (0x2d0, 0x3000),
        (0x2d4, 0x1000),
        (0x2d8, 0),
    ] {
        g.write::<u32>(at(o), x);
    }
    g.write::<u8>(at(0x89c), 1);
    free_0577(g, img, c, 0xaf0, (0xaac, 0xab7), false);
    let sp = c.base.wrapping_sub(0x10);
    particle_0577(g, img, c, 0xace, (0xaf0, 0x754, 9), sp);
    // call width: 2 (the particle node's words stay under it)
    g.call(slot(g, 0xada, 325), 0, &[at(0x40), at(0x49c)]);
    let v = c.base + 0x1c;
    let cf = Frame {
        at: call(0xaf4),
        esp: sp.wrapping_sub(0xc),
        ecx: v,
        edx: w(0xaeb),
        ..Frame::default()
    };
    crate::m0577::centre(g, img, &cf);
    // call width: 3
    let eax = g.call(slot(g, 0xb03, 234), 0, &[at(0x50), v, at(0x40)]).eax;
    let s = g.read::<u32>(at(0x60)).wrapping_add(1);
    g.write::<u32>(at(0x60), s);
    g.write::<u32>(at(0x68), 0);
    eax
}

/// [`spark_task_0577`]'s state 0x3c: the part freed, the matrix, the 20
/// sparks and the last spark's words.
fn matrix_0577<G: Game>(g: &mut G, img: &Image, c: &Ctx) {
    use crate::sb_sse::sub;
    let w = |o: u32| img.word(c.f + o);
    let call = |o: u32| img.rel32_target(c.f + o);
    let at = |o: u32| c.r.wrapping_add(o);
    let slot = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    free_0577(g, img, c, 0xaf0, (0x24a, 0x255), false);
    let m = g.read::<u32>(at(0xfc));
    // call width: 2
    let h = g.call(slot(g, 0x25a, 210), 0, &[m, 0]).eax;
    // call width: 2
    g.call(slot(g, 0x270, 230), 0, &[c.r, h]);
    for (o, op) in [(0u32, 0x27eu32), (0x10, 0x2a6), (0x20, 0x2c2)] {
        // call width: 3
        g.call(slot(g, op, 233), 0, &[at(o), at(o), 0x4774_2400]);
    }
    // call width: 2
    g.call(slot(g, 0x2cf, 327), 0, &[c.r, c.r]);
    let mut spark = at(0xd2c);
    for i in 0..0x14u32 {
        let sp = c.base.wrapping_sub(0xc);
        for (j, v) in (0u32..).zip([i, at(0x49c), c.r]) {
            g.write::<u32>(sp + 4 * j, v);
        }
        let sf = Frame {
            at: call(0x2ff),
            esp: sp.wrapping_sub(4),
            ecx: spark,
            edx: at(0xa8),
            ..Frame::default()
        };
        crate::m0577::spark_setup(g, img, &sf);
        spark = spark.wrapping_add(0xe4);
    }
    let s = g.read::<u32>(at(0xd28));
    let x = sub(0.0, g.read::<f32>(w(0x32b)));
    let fl = g.read::<u8>(s + 0x40) | 1;
    g.write::<u8>(s + 0x40, fl);
    for (o, v) in [
        (0x1cu32, 0x4000_0000u32),
        (0x20, 0x4040_0000),
        (0x24, 0x40c0_0000),
        (0x28, 0x4100_0000),
        (0x30, 0x41a0_0000),
        (0x2c, 0x41a0_0000),
        (0x38, 0),
    ] {
        g.write::<u32>(s + o, v);
    }
    g.write::<f32>(s + 0x3c, x);
}

/// [`spark_task_0577`]'s state 10000: the nodes leave, slot 21 for the
/// DLL's ten words, the model's set-up, the task unlinked; state 20000.
/// Returns the task node's next.
fn end_0577<G: Game>(g: &mut G, img: &Image, c: &Ctx) -> u32 {
    let w = |o: u32| img.word(c.f + o);
    let at = |o: u32| c.r.wrapping_add(o);
    for o in [0x8a0u32, 0x608, 0x4bc, 0x140, 0x30c] {
        crate::sa_misc::leave(g, at(o), 0);
    }
    let mut p = w(0xc90);
    while p < w(0xcb6) {
        let m = g.read::<u32>(at(0xfc));
        let x = g.read::<u32>(p);
        // call width: 3
        g.call(slot_target(g, w(0xc95), 21), 0, &[m, x, ONE]);
        p += 4;
    }
    crate::sa_misc::leave(g, at(0xa8), 0);
    let grp = c.base + 0xc;
    g.write::<u32>(grp, at(0xa8));
    let mf = Frame {
        at: img.rel32_target(c.f + 0xd0a),
        esp: c.base.wrapping_sub(4),
        ecx: grp,
        ..Frame::default()
    };
    crate::sa_summon::model_setup(g, img, &mf);
    let m = g.read::<u32>(at(0xfc));
    // call width: 1
    g.call(slot_target(g, w(0xd0f), 384), 0, &[m]);
    let task = g.read::<u32>(c.base + 0x10);
    let eax = crate::h6_setup::unlink_task(g, task);
    g.write::<u32>(at(0x60), 0x4e20);
    eax
}

/// [`spark_task_0577`] (the `rel32`s: its callees in order,
/// `__security_check_cookie`).
pub const SPARK_TASK_0577: Sig = Sig {
    len: 3558,
    hash: 0x07ca_7fb1_8212_e01e,
    calls: &[
        (0x128, Callee::Sig(&crate::m0577::COLOUR)),
        (0x17f, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0x186, Callee::Sig(&crate::m0070::FREE_PART)),
        (0x24a, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0x255, Callee::Sig(&crate::m0070::FREE_PART)),
        (0x2ff, Callee::Sig(&crate::m0577::SPARK_SETUP)),
        (0x3d8, Callee::Any),
        (0x409, Callee::Sig(&crate::m0070::NODE_INIT_SMALL)),
        (0x435, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0x475, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x48c, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x4cd, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x4e1, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x4f4, Callee::Sig(&crate::s4_fam446::EFFECT_NODE_A)),
        (0x523, Callee::Any),
        (0x539, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x56c, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x5af, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x5cf, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x5f5, Callee::Sig(&crate::l3_common::CHR_FADE)),
        (0x76a, Callee::Any),
        (0x79d, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x811, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x854, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x868, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x8d4, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0x8db, Callee::Sig(&crate::m0070::FREE_PART)),
        (0x91f, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0x92a, Callee::Sig(&crate::m0070::FREE_PART)),
        (0x941, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x98b, Callee::Any),
        (0x9d3, Callee::Sig(&crate::m0577::COLOUR)),
        (0x9ea, Callee::Any),
        (0xa25, Callee::Sig(&crate::r27::BONE_0577)),
        (0xaac, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0xab7, Callee::Sig(&crate::m0070::FREE_PART)),
        (0xace, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0xaf4, Callee::Sig(&crate::m0577::CENTRE)),
        (0xb29, Callee::Any),
        (0xd0a, Callee::Sig(&crate::sa_summon::MODEL_SETUP)),
        (0xd56, Callee::Any),
    ],
    imports: &[],
};

/// A fresh node `n` put last under the DLL's list as `magic_0577`'s
/// burst task inlines it: pass `u16 +0x1a` 0xff, no children, `+0x14` and
/// `+0x4` the list (operands `p14`, `p4`), `+0x8` the list's last (read at
/// `l1`); the old last (read again at `l2`) is the list (`cmp`): its
/// first (`first`) is `n`, else the old last's `+0x4`; the count (`count`)
/// up, the list's last (`tail`) `n`. `between` runs after the count (a
/// word the code loads there).
fn link_c750<G: Game>(g: &mut G, img: &Image, f: u32, n: u32, ops: [u32; 8]) {
    let w = |o: u32| img.word(f + o);
    let [p14, p4, l1, l2, cmp, first, count, tail] = ops;
    g.write::<u16>(n + 0x1a, 0xff);
    g.write::<u32>(n + 0x10, n);
    g.write::<u32>(n + 0xc, n);
    g.write::<u32>(n + 0x14, w(p14));
    g.write::<u32>(n + 4, w(p4));
    let last = g.read::<u32>(w(l1));
    g.write::<u32>(n + 8, last);
    let last = g.read::<u32>(w(l2));
    if last == w(cmp) {
        g.write::<u32>(w(first), n);
    } else {
        g.write::<u32>(last.wrapping_add(4), n);
    }
    let c = g.read::<u16>(w(count)).wrapping_add(1);
    g.write::<u16>(w(count), c);
    g.write::<u32>(w(tail), n);
}

/// `magic_0577`'s burst task (0x1000c750, 4375 bytes; a task callback
/// `(A)`, cdecl; an aligned /GS frame below three pushes; the record `R =
/// A[4]`, its state `R + 0x18`, a count `R + 0x1c` up and a wait `R +
/// 0x20` down at every exit). From the byte and jump tables:
/// - 0: the model's nodes ([`model_nodes`](crate::s3_summon::model_nodes)),
///   appends, slots 262 and 211, the model state, an owned node, the big
///   node, the root node, the task head and an effect node; state 0xa;
/// - 0xa: six static nodes put last under the DLL's list with their words
///   and two particle nodes, on to 0x14; 0x14/0x15: after a wait of 110
///   three more nodes, a particle node, the light node and the last
///   spark's words, on to 0x28 (state 0x2a after it);
/// - 0x2a: after the wait the burst ([`burst_setup_0577`](crate::r29::burst_setup_0577)),
///   on to 0x1e; 0x1e/0x1f: after a wait of 177 slot 234 on the colours;
///   0x20: after the wait slots 296 and 234, the move's words, the part
///   freed and a particle node, the body
///   ([`body_setup_0577`](crate::r29::body_setup_0577)), on to 0x32;
/// - 0x32, 0x33: the flag bit 1 set and cleared once the game's clock is
///   past 170 and 410; 0x3c/0x3d: a wait of 125 down to 10; 0x3e: the
///   centre, the part, a particle node, slot 222, the place and the head
///   nodes; 0x3f: slot 313's answer, the place, the part, glow off, an
///   effect node; 0x40: the part, slot 222; 0x41: slot 313, a particle
///   node, halve; 0x42: 10000, which moves the light and goes to 0x2711;
///   0x2711 (with both bursts done and the part's flag) the nodes reset
///   and the task unlinked (slot 568, the current node).
///
/// From state 0x3c up to 10000 every call also copies the light's place
/// (slot 325). Returns `eax` as the original leaves it.
#[allow(clippy::too_many_lines)] // the original's states, in order
pub fn burst_task_0577<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let call = |o: u32| img.rel32_target(f + o);
    let slot = |g: &G, n: u32| slot_target(g, w(0x9eb), n);
    // push ebp; and esp, -8; sub esp, 0x4c; three pushes
    let e0 = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x4c);
    g.write::<u32>(e0 + 0x48, g.read::<u32>(w(0xa)) ^ e0);
    let base = e0.wrapping_sub(0xc);
    let l = |o: u32| base.wrapping_add(o);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    g.write::<u32>(l(0xc), task);
    g.write::<u32>(l(0x10), r);
    let at = |o: u32| r.wrapping_add(o);
    let wait = |g: &G| g.read::<i32>(at(0x20));
    let set = |g: &mut G, s: u32| g.write::<u32>(at(0x18), s);
    let up = |g: &mut G| {
        let s = g.read::<u32>(at(0x18)).wrapping_add(1);
        g.write::<u32>(at(0x18), s);
    };
    let c = Ctx { f, base, r };
    let table = w(0x47);
    let mut state = g.read::<u32>(at(0x18));
    loop {
        if state > 0x2710 {
            if state == 0x2711 {
                end_c750(g, img, &c);
            }
            break;
        }
        if state == 0x2710 {
            for o in [
                0x11e4u32, 0x11e0, 0x11dc, 0x11b4, 0x11b0, 0x11ac, 0x11e8, 0x11b8,
            ] {
                g.write::<u32>(at(o), ONE);
            }
            let rec = l(0x14);
            for (i, v) in (0u32..).zip([at(0x1180), 0, 0x3d4c_cccd, ONE]) {
                g.write::<u32>(rec + 4 * i, v);
            }
            crate::m0577::move_start(g, rec);
            up(g);
            break;
        }
        if state > 0x42 {
            break;
        }
        let again = match img.word(table + state) & 0xff {
            _ if state == 0 => {
                start_c750(g, img, &c);
                false
            }
            _ if state == 0xa => {
                statics_c750(g, img, &c);
                set(g, 0x14);
                true
            }
            _ if state == 0x14 || state == 0x15 => {
                if state == 0x14 {
                    g.write::<u32>(at(0x20), 0x6e);
                    set(g, 0x15);
                }
                if wait(g) > 0 {
                    false
                } else {
                    lights_c750(g, img, &c);
                    set(g, 0x28);
                    true
                }
            }
            _ if state == 0x28 || state == 0x29 => {
                if state == 0x28 {
                    up(g);
                    g.write::<u32>(at(0x20), 0x3c);
                }
                up(g);
                false
            }
            _ if state == 0x2a => {
                if wait(g) > 0 {
                    false
                } else {
                    for (o, v) in [
                        (0x534u32, 0x40a0_0000u32),
                        (0x538, 0xc1f0_0000),
                        (0x53c, 0xc040_0000),
                        (0x540, ONE),
                    ] {
                        g.write::<u32>(at(o), v);
                    }
                    let sp = base.wrapping_sub(4);
                    g.write::<u32>(sp, at(0x518));
                    crate::r29::burst_setup_0577(
                        g,
                        img,
                        call(0x65c),
                        sp.wrapping_sub(4),
                        at(0x3290),
                        at(0x1034),
                    );
                    set(g, 0x1e);
                    true
                }
            }
            _ if state == 0x1e || state == 0x1f => {
                if state == 0x1e {
                    up(g);
                    g.write::<u32>(at(0x20), 0xb1);
                }
                if wait(g) > 0 {
                    false
                } else {
                    for o in [0x1290u32, 0x128c, 0x1288, 0x1284] {
                        g.write::<u32>(at(o), 0);
                    }
                    for o in [0x12a0u32, 0x129c, 0x1298, 0x1294] {
                        g.write::<u32>(at(o), 0x437f_0000);
                    }
                    // call width: 3
                    g.call(slot(g, 234), 0, &[at(0x12a4), at(0x1294), at(0x1284)]);
                    for (o, v) in [(0x12bcu32, 1u32), (0x12b8, 0x3e4c_cccd), (0x12b4, 0)] {
                        g.write::<u32>(at(o), v);
                    }
                    up(g);
                    g.write::<u32>(at(0x20), 4);
                    false
                }
            }
            _ if state == 0x20 => {
                if wait(g) > 0 {
                    false
                } else {
                    body_c750(g, img, &c);
                    set(g, 0x32);
                    true
                }
            }
            _ if state == 0x32 || state == 0x33 => {
                let (op, bound) = if state == 0x32 {
                    (0xc81, 0xaa)
                } else {
                    (0x91e, 0x19a)
                };
                if g.read::<u32>(w(op)) >= bound {
                    let fl = g.read::<u32>(at(0xc38));
                    if state == 0x32 {
                        g.write::<u32>(at(0xc38), fl | 2);
                        up(g);
                        false
                    } else {
                        g.write::<u32>(at(0xc38), fl & !2);
                        set(g, 0x3c);
                        true
                    }
                } else {
                    false
                }
            }
            _ if state == 0x3c || state == 0x3d => {
                if state == 0x3c {
                    up(g);
                    g.write::<u32>(at(0x20), 0x7d);
                }
                if wait(g) <= 0xa {
                    up(g);
                }
                false
            }
            _ if state == 0x3e => {
                if wait(g) <= 0 {
                    heads_c750(g, img, &c);
                }
                false
            }
            _ if state == 0x3f => {
                if wait(g) <= 0 {
                    glow_c750(g, img, &c);
                }
                false
            }
            _ if state == 0x40 => {
                if wait(g) <= 0 {
                    part_c750(g, img, &c, (0x950, 0xa38), (0xf53, 0xf5e));
                    // call width: 4
                    g.call(slot(g, 222), 0, &[w(0xf74), 0, 1, 2]);
                    up(g);
                }
                false
            }
            _ if state == 0x41 => {
                // call width: 1
                if g.call(slot(g, 313), 0, &[2]).eax == 0 {
                    g.write::<u32>(at(0x598), 0x3d4c_cccd);
                    particle_c750(g, img, &c, 0xfbb, (0x950, 0xc50, 0));
                    crate::m0577::halve(g, img, call(0xfc3));
                    sparks_c750(g, &c, 0x2710);
                    up(g);
                    g.write::<u32>(at(0x20), 0x5a);
                }
                false
            }
            _ if state == 0x42 => {
                if wait(g) > 0 {
                    false
                } else {
                    set(g, 0x2710);
                    true
                }
            }
            _ => false,
        };
        if !again {
            break;
        }
        state = g.read::<u32>(at(0x18));
    }
    // the tail: the light's place from 0x3c up to 10000
    let s = g.read::<u32>(at(0x18));
    let mut eax = s;
    if (0x3c..0x2710).contains(&s) {
        // call width: 2
        eax = g.call(slot(g, 325), 0, &[at(0x534), w(0x9e5)]).eax;
    }
    let x = g.read::<u32>(at(0x1c)).wrapping_add(1);
    g.write::<u32>(at(0x1c), x);
    let x = g.read::<u32>(at(0x20)).wrapping_sub(1);
    g.write::<u32>(at(0x20), x);
    Ret::eax(eax)
}

/// [`particle_node_with`](crate::s3_nodes::particle_node_with) of `R + o`
/// with `(R + a, -, -, n)` (the middle words the frame's).
fn particle_c750<G: Game>(g: &mut G, img: &Image, c: &Ctx, at: u32, (o, a, n): (u32, u32, u32)) {
    let sp = c.base.wrapping_sub(0x10);
    g.write::<u32>(sp, c.r.wrapping_add(a));
    g.write::<u32>(sp + 0xc, n);
    let pf = Frame {
        at: img.rel32_target(c.f + at),
        esp: sp.wrapping_sub(4),
        ecx: c.r.wrapping_add(o),
        ..Frame::default()
    };
    crate::s3_nodes::particle_node_with(g, img, &pf);
}

/// The part `R + o` freed (the frame group at `E + 0xc`) unless the flag
/// byte `R + fl` bit 2, then [`free_part`](crate::m0070::free_part).
fn part_c750<G: Game>(g: &mut G, img: &Image, c: &Ctx, (o, fl): (u32, u32), calls: (u32, u32)) {
    let part = c.r.wrapping_add(o);
    if g.read::<u8>(c.r.wrapping_add(fl)) & 4 == 0 {
        let grp = c.base + 0xc;
        g.write::<u32>(grp, part);
        let ff = Frame {
            at: img.rel32_target(c.f + calls.0),
            esp: c.base.wrapping_sub(4),
            ecx: grp,
            ..Frame::default()
        };
        crate::nodes::free_then_unlink(g, img, &ff);
    }
    crate::m0070::free_part(g, img, img.rel32_target(c.f + calls.1), part, 0);
}

/// The sparks' states: each of the `R[0x500]` records (0x24 apart from
/// `R + 0x37bc`) gets `v`.
fn sparks_c750<G: Game>(g: &mut G, c: &Ctx, v: u32) {
    let at = |o: u32| c.r.wrapping_add(o);
    let mut i = 0i32;
    while i < g.read::<i32>(at(0x500)) {
        g.write::<u32>(at(0x37bc).wrapping_add(0x24 * i.cast_unsigned()), v);
        i += 1;
    }
}

/// A particle node from the frame record `[N, B, P, *G, Q, n]` (`E +
/// 0x1c`), `N`'s vtable stored first.
#[allow(clippy::too_many_arguments)] // the record's words
fn pnode_c750<G: Game>(
    g: &mut G,
    img: &Image,
    c: &Ctx,
    (node, vt): (u32, u32),
    ops: (u32, u32, u32),
    p: u32,
    n: u32,
    call: u32,
) {
    let w = |o: u32| img.word(c.f + o);
    let x = g.read::<u32>(w(ops.1));
    let rec = c.base + 0x28;
    g.write::<u32>(node, w(vt));
    for (i, v) in (0u32..).zip([node, w(ops.0), p, x, w(ops.2), n]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    crate::sb_nodes::particle_node(g, img, img.rel32_target(c.f + call), rec);
}

/// [`burst_task_0577`]'s state 0.
fn start_c750<G: Game>(g: &mut G, img: &Image, c: &Ctx) {
    let w = |o: u32| img.word(c.f + o);
    let call = |o: u32| img.rel32_target(c.f + o);
    let at = |o: u32| c.r.wrapping_add(o);
    let slot = |g: &G, n: u32| slot_target(g, w(0xa6b), n);
    let list = w(0xa2c);
    let grp = c.base + 0x14;
    let x = g.read::<u32>(w(0xa13));
    g.write::<u32>(at(0x24), w(0xa20));
    for (i, v) in (0u32..).zip([at(0x24), list, x, 0]) {
        g.write::<u32>(grp + 4 * i, v);
    }
    g.write::<u8>(grp + 0x10, 0);
    let mf = Frame {
        at: call(0xa42),
        esp: c.base.wrapping_sub(4),
        ecx: grp,
        ..Frame::default()
    };
    crate::s3_summon::model_nodes(g, img, &mf);
    crate::sb_misc::node_append(g, img, call(0xa52), at(0x59c), w(0xa4d), 0);
    let m = g.read::<u32>(at(0x514));
    let o = at(0x5b8);
    let mw = g.read::<u32>(m + 0x54);
    // call width: 4
    g.call(slot(g, 262), 0, &[mw, o, o + 4, o + 8]);
    g.write::<u32>(at(0x5c4), ONE);
    crate::sa_misc::node_append(g, img, call(0xa93), at(0x5f4), w(0xa82), 0);
    for (o, v) in [(0x610u32, 0u32), (0x614, 0x10000), (0x618, 0)] {
        g.write::<u32>(at(o), v);
    }
    crate::sb_misc::node_append(g, img, call(0xabe), at(0x5c8), w(0xa82), 0);
    // the model state's group four words below (slot 262's words pushed)
    let grp = c.base.wrapping_sub(0x10) + 0x24;
    let m = g.read::<u32>(at(0x514));
    g.write::<u32>(grp, m);
    g.write::<u32>(grp + 4, at(0x5e4));
    crate::m0070::model_state(g, img, call(0xadb), grp);
    let sp = c.base.wrapping_sub(0x20);
    for (i, v) in (0u32..).zip([w(0xaf3), at(0x59c), at(0x5f4), at(0x5c8)]) {
        g.write::<u32>(sp + 4 * i, v);
    }
    let owned = at(0x1034);
    let of = Frame {
        at: call(0xaff),
        esp: sp.wrapping_sub(4),
        ecx: owned,
        edx: w(0xaf8),
        ..Frame::default()
    };
    crate::sa_misc::node_append_owned(g, img, &of);
    let m = g.read::<u32>(at(0x514));
    let mw = g.read::<u32>(m + 0x54);
    // call width: 2
    g.call(slot(g, 211), 0, &[mw, owned + 0x9c]);
    let fl = g.read::<u16>(m + 0x90) | 0x10;
    g.write::<u16>(m + 0x90, fl);
    let sp = c.base.wrapping_sub(0xc);
    g.write::<u32>(sp, w(0xb32));
    g.write::<u32>(m + 0x4c, owned);
    let bf = Frame {
        at: call(0xb3a),
        esp: sp.wrapping_sub(4),
        ecx: at(0x774),
        ..Frame::default()
    };
    crate::s4_small::big_node(g, img, &bf);
    let m = g.read::<u32>(at(0x514));
    let mw = g.read::<u32>(m + 0x54);
    g.write::<u32>(at(0x8c0), mw);
    g.write::<u32>(at(0x8c4), 0x46);
    crate::m0070::node_init_root(g, img, call(0xb61), at(0x1268));
    let sp = c.base.wrapping_sub(4);
    g.write::<u32>(sp, owned);
    let m = g.read::<u32>(at(0x514));
    let tf = Frame {
        at: call(0xb75),
        esp: sp.wrapping_sub(4),
        ecx: at(0x1338),
        edx: m,
        ..Frame::default()
    };
    crate::o4_single3::object_task_head(g, img, &tf);
    g.write::<u32>(at(0x18), 0xa);
    // push 0; push ecx; push T; push ecx (the head's leftovers, unread: 0)
    let sp = c.base.wrapping_sub(0x10);
    for (i, v) in (0u32..).zip([0, w(0xb87), 0, 0]) {
        g.write::<u32>(sp + 4 * i, v);
    }
    let ef = Frame {
        at: call(0xb93),
        esp: sp.wrapping_sub(4),
        ecx: at(0x12c4),
        ..Frame::default()
    };
    crate::s4_fam446::effect_node_a(g, img, &ef);
    let fl = g.read::<u32>(at(0x12f0)) | 6;
    g.write::<u32>(at(0x12f0), fl | 1);
    g.write::<u32>(at(0x12f4), 0);
}

/// [`burst_task_0577`]'s state 0xa: six static nodes and two particle
/// nodes.
#[allow(clippy::too_many_lines)] // the original's steps, in order
fn statics_c750<G: Game>(g: &mut G, img: &Image, c: &Ctx) {
    let w = |o: u32| img.word(c.f + o);
    let at = |o: u32| c.r.wrapping_add(o);
    let f = c.f;
    let n1 = at(0x620);
    link_c750(
        g,
        img,
        f,
        n1,
        [0x67, 0x6e, 0x73, 0x7b, 0x80, 0x88, 0x94, 0x9a],
    );
    for (o, v) in [
        (0x13cu32, w(0xa4)),
        (0x144, w(0xae)),
        (0x140, w(0xb8)),
        (0x128, 0),
    ] {
        g.write::<u32>(n1 + o, v);
    }
    g.write::<u8>(n1 + 0x148, 0);
    g.write::<u32>(n1, w(0xcf));
    let m = g.read::<u32>(at(0x514));
    let mw = g.read::<u32>(m + 0x54);
    g.write::<u32>(at(0x76c), mw);
    g.write::<u32>(at(0x770), 0xc);
    let n2 = at(0x8c8);
    link_c750(
        g,
        img,
        f,
        n2,
        [0x101, 0x108, 0x10d, 0x115, 0x11a, 0x122, 0x12e, 0x134],
    );
    g.write::<u32>(n2 + 0x38, ONE);
    g.write::<u32>(n2 + 0x3c, n1);
    g.write::<u32>(n2, w(0x144));
    for (o, v) in [
        (0x8f4u32, 0x3fd3_3333u32),
        (0x8f8, 0xbe0f_5c29),
        (0x8fc, 0xbecc_cccd),
    ] {
        g.write::<u32>(at(o), v);
    }
    let n3 = at(0x908);
    link_c750(
        g,
        img,
        f,
        n3,
        [0x17b, 0x182, 0x187, 0x18f, 0x194, 0x19c, 0x1a8, 0x1ae],
    );
    g.write::<u32>(n3 + 0x38, ONE);
    g.write::<u32>(n3 + 0x3c, n1);
    g.write::<u32>(n3, w(0x1be));
    for (o, v) in [
        (0x934u32, 0x3fd3_3333u32),
        (0x938, 0xbe0f_5c29),
        (0x93c, 0x3ecc_cccd),
    ] {
        g.write::<u32>(at(o), v);
    }
    let n4 = at(0x570);
    link_c750(
        g,
        img,
        f,
        n4,
        [0x1f5, 0x1fc, 0x201, 0x209, 0x20e, 0x216, 0x222, 0x228],
    );
    g.write::<u32>(n4, w(0x22e));
    for (o, v) in [
        (0x58cu32, ONE),
        (0x590, ONE),
        (0x594, ONE),
        (0x598, 0x3d4c_cccd),
    ] {
        g.write::<u32>(at(o), v);
    }
    let n5 = at(0xc50);
    g.write::<u32>(n5, w(0x262));
    link_c750(
        g,
        img,
        f,
        n5,
        [0x275, 0x27c, 0x281, 0x289, 0x28e, 0x296, 0x2a2, 0x2ad],
    );
    for (o, v) in [
        (0x140u32, n4),
        (0x13c, n2),
        (0x144, w(0x2c9)),
        (0x128, w(0x2d3)),
    ] {
        g.write::<u32>(n5 + o, v);
    }
    g.write::<u8>(n5 + 0x148, 0);
    pnode_c750(
        g,
        img,
        c,
        (at(0x950), 0x2e4),
        (0x2f0, 0x2a7, 0x300),
        n5,
        6,
        0x30d,
    );
    let n6 = at(0xd9c);
    g.write::<u32>(n6, w(0x319));
    link_c750(
        g,
        img,
        f,
        n6,
        [0x32c, 0x333, 0x338, 0x340, 0x345, 0x34d, 0x359, 0x35f],
    );
    for (o, v) in [
        (0x140u32, n4),
        (0x13c, n3),
        (0x144, w(0x386)),
        (0x128, w(0x390)),
    ] {
        g.write::<u32>(n6 + o, v);
    }
    g.write::<u8>(n6 + 0x148, 0);
    pnode_c750(
        g,
        img,
        c,
        (at(0xa50), 0x3a1),
        (0x3ad, 0x376, 0x3bd),
        n6,
        6,
        0x3ca,
    );
}

/// [`burst_task_0577`]'s state 0x15 past its wait: three nodes, a particle
/// node, the light node, the last spark's words.
fn lights_c750<G: Game>(g: &mut G, img: &Image, c: &Ctx) {
    use crate::sb_sse::sub;
    let w = |o: u32| img.word(c.f + o);
    let at = |o: u32| c.r.wrapping_add(o);
    let f = c.f;
    let n7 = at(0x518);
    link_c750(
        g,
        img,
        f,
        n7,
        [0x406, 0x40d, 0x412, 0x41a, 0x41f, 0x427, 0x433, 0x439],
    );
    g.write::<u32>(n7, w(0x43f));
    for (o, v) in [
        (0x534u32, 0xc170_0000u32),
        (0x538, 0x419d_999a),
        (0x53c, 0xc264_0000),
        (0x540, ONE),
    ] {
        g.write::<u32>(at(o), v);
    }
    let n8 = at(0x544);
    link_c750(
        g,
        img,
        f,
        n8,
        [0x480, 0x487, 0x48c, 0x494, 0x499, 0x4a1, 0x4ad, 0x4b3],
    );
    g.write::<u32>(n8, w(0x4b9));
    for (o, v) in [
        (0x560u32, ONE),
        (0x564, ONE),
        (0x568, ONE),
        (0x56c, 0x40c0_0000),
    ] {
        g.write::<u32>(at(o), v);
    }
    let n9 = at(0xee8);
    g.write::<u32>(n9, w(0x4f3));
    link_c750(
        g,
        img,
        f,
        n9,
        [0x506, 0x50d, 0x512, 0x51a, 0x51f, 0x527, 0x533, 0x53e],
    );
    for (o, v) in [
        (0x13cu32, n7),
        (0x144, w(0x554)),
        (0x140, n8),
        (0x128, at(0x1034)),
    ] {
        g.write::<u32>(n9 + o, v);
    }
    g.write::<u8>(n9 + 0x148, 0);
    pnode_c750(
        g,
        img,
        c,
        (at(0xb50), 0x571),
        (0x57d, 0x538, 0x58d),
        n9,
        7,
        0x59a,
    );
    let grp = c.base + 0x14;
    g.write::<u32>(grp, at(0x1180));
    g.write::<u32>(grp + 4, w(0x5b0));
    g.write::<u32>(at(0x1180), w(0x5b6));
    let lf = Frame {
        at: img.rel32_target(f + 0x5bb),
        esp: c.base.wrapping_sub(4),
        ecx: grp,
        ..Frame::default()
    };
    crate::m0577::light_node(g, img, &lf);
    let s = g.read::<u32>(at(0x1334));
    let x = sub(0.0, g.read::<f32>(w(0x5cc)));
    let fl = g.read::<u8>(s + 0x40) | 1;
    g.write::<u8>(s + 0x40, fl);
    for (o, v) in [
        (0x1cu32, ONE),
        (0x20, ONE),
        (0x24, 0x4080_0000u32),
        (0x28, 0x4080_0000),
        (0x30, 0x4170_0000),
        (0x2c, 0x4170_0000),
        (0x38, 0),
    ] {
        g.write::<u32>(s + o, v);
    }
    g.write::<f32>(s + 0x3c, x);
}

/// [`burst_task_0577`]'s state 0x20 past its wait.
#[allow(clippy::too_many_lines)] // the original's steps, in order
fn body_c750<G: Game>(g: &mut G, img: &Image, c: &Ctx) {
    use crate::sb_sse::sub;
    let w = |o: u32| img.word(c.f + o);
    let at = |o: u32| c.r.wrapping_add(o);
    let slot = |g: &G, n: u32| slot_target(g, w(0x67a), n);
    let rf = |g: &G, o: u32| g.read::<f32>(at(o));
    // call width: 2
    g.call(slot(g, 296), 0, &[0x15, 0]);
    // call width: 2
    g.call(slot(g, 296), 0, &[0x16, 0]);
    for o in [0x12a0u32, 0x129c, 0x1298, 0x1294] {
        g.write::<u32>(at(o), 0);
    }
    // call width: 3
    g.call(slot(g, 234), 0, &[at(0x12a4), at(0x1294), at(0x1284)]);
    let fl = g.read::<u16>(at(0x12c0)) | 1;
    g.write::<u16>(at(0x12c0), fl);
    for (o, v) in [(0x12bcu32, 1u32), (0x12b8, 0x3d88_8889), (0x12b4, 0)] {
        g.write::<u32>(at(o), v);
    }
    for o in [0x11e8u32, 0x11e4, 0x11e0, 0x11dc, 0x11b4, 0x11b0, 0x11ac] {
        g.write::<u32>(at(o), 0);
    }
    let fl = g.read::<u32>(at(0x1264)) & !1;
    g.write::<u32>(at(0x1264), fl);
    g.write::<u32>(at(0x124c), ONE);
    g.write::<u32>(at(0x1250), 0);
    for (d, a, b) in [
        (0x11bcu32, 0x11acu32, 0x119cu32),
        (0x11ec, 0x11dc, 0x11cc),
        (0x11c0, 0x11b0, 0x11a0),
        (0x11f0, 0x11e0, 0x11d0),
        (0x11c4, 0x11b4, 0x11a4),
        (0x11f4, 0x11e4, 0x11d4),
        (0x11c8, 0x11b8, 0x11a8),
        (0x11f8, 0x11e8, 0x11d8),
    ] {
        let x = sub(rf(g, a), rf(g, b));
        g.write::<f32>(at(d), x);
    }
    for (o, v) in [
        (0x560u32, ONE),
        (0x564, ONE),
        (0x568, ONE),
        (0x56c, 0x3eb8_51ec),
    ] {
        g.write::<u32>(at(o), v);
    }
    g.write::<u32>(at(0x1010), w(0x864));
    g.write::<u32>(at(0x1024), w(0x86e));
    let part = at(0xb50);
    if g.read::<u8>(at(0xc38)) & 4 == 0 {
        let grp = c.base + 0x40;
        g.write::<u32>(grp, part);
        let ff = Frame {
            at: img.rel32_target(c.f + 0x883),
            esp: c.base.wrapping_sub(4),
            ecx: grp,
            ..Frame::default()
        };
        crate::nodes::free_then_unlink(g, img, &ff);
    }
    if g.read::<u8>(at(0xc38)) & 4 == 0 {
        let v = g.read::<u32>(part + 0x50);
        g.write::<u32>(g.read::<u32>(w(0x897)), v);
        // call width: 1
        g.call(slot_target(g, w(0x8a5), 713), 0, &[part + 0x60]);
    }
    pnode_c750(
        g,
        img,
        c,
        (part, 0x8e9),
        (0x8cf, 0x8bf, 0x8db),
        at(0xee8),
        3,
        0x8ee,
    );
    let sp = c.base.wrapping_sub(8);
    let b = g.read::<u32>(at(0x1334));
    g.write::<u32>(sp, at(0x1034));
    g.write::<u32>(sp + 4, b);
    crate::r29::body_setup_0577(
        g,
        img,
        img.rel32_target(c.f + 0x90c),
        sp.wrapping_sub(4),
        at(0x1390),
        at(0x774),
    );
}

/// [`burst_task_0577`]'s state 0x3e past its wait.
fn heads_c750<G: Game>(g: &mut G, img: &Image, c: &Ctx) {
    let w = |o: u32| img.word(c.f + o);
    let call = |o: u32| img.rel32_target(c.f + o);
    let at = |o: u32| c.r.wrapping_add(o);
    let cf = Frame {
        at: call(0xcd0),
        esp: c.base.wrapping_sub(4),
        ecx: at(0x534),
        edx: w(0xccb),
        ..Frame::default()
    };
    crate::m0577::centre(g, img, &cf);
    for (o, v) in [
        (0x560u32, ONE),
        (0x564, ONE),
        (0x568, ONE),
        (0x56c, 0x3ce5_6041),
    ] {
        g.write::<u32>(at(o), v);
    }
    g.write::<u32>(at(0x1024), at(0x518));
    part_c750(g, img, c, (0xb50, 0xc38), (0xd20, 0xd2b));
    particle_c750(g, img, c, 0xd42, (0xb50, 0xee8, 8));
    // call width: 4 (the particle node's words stay under it)
    g.call(slot_target(g, w(0xd47), 222), 0, &[w(0xd58), 0x15, 0x19, 2]);
    let (grp, v) = (c.base + 0x14, c.base + 0x44);
    for (i, x) in (0u32..).zip([0x4348_0000u32, 0x4348_0000, 0x4348_0000, 0x4300_0000]) {
        g.write::<u32>(v + 4 * i, x);
    }
    for (i, x) in (0u32..).zip([at(0x1180), v, 0x3ca3_d70a, 0]) {
        g.write::<u32>(grp + 4 * i, x);
    }
    let pf = Frame {
        at: call(0xda8),
        esp: c.base.wrapping_sub(4),
        ecx: grp,
        ..Frame::default()
    };
    crate::m0577::place(g, img, &pf);
    // the counter `E + 0x10` (the record word there before)
    let n = c.base + 0x10;
    g.write::<u32>(n, 0);
    let mut i = 0u32;
    while i.cast_signed() < g.read::<i32>(at(0x500)) {
        crate::o4_single3::head_node(
            g,
            img,
            call(0xdcb),
            at(0x37a0) + 0x24 * i,
            at(0x40) + 0x98 * i,
        );
        i += 1;
        g.write::<u32>(n, i);
    }
    let s = g.read::<u32>(at(0x18)).wrapping_add(1);
    g.write::<u32>(at(0x18), s);
    g.write::<u32>(at(0x20), 0x5a);
}

/// [`burst_task_0577`]'s state 0x3f past its wait (when slot 313 answers
/// 0).
fn glow_c750<G: Game>(g: &mut G, img: &Image, c: &Ctx) {
    use crate::sb_sse::mul;
    let w = |o: u32| img.word(c.f + o);
    let call = |o: u32| img.rel32_target(c.f + o);
    let at = |o: u32| c.r.wrapping_add(o);
    let sf = Frame {
        at: call(0xe03),
        esp: c.base.wrapping_sub(4),
        ..Frame::default()
    };
    if crate::o4_tiny::slot_imm8(g, img, &sf).eax != 0 {
        return;
    }
    let grp = c.base + 0x14;
    for (i, x) in (0u32..).zip([at(0x1180), c.base + 0x44, ONE, 1]) {
        g.write::<u32>(grp + 4 * i, x);
    }
    let pf = Frame {
        at: call(0xe36),
        esp: c.base.wrapping_sub(4),
        ecx: grp,
        ..Frame::default()
    };
    crate::m0577::place(g, img, &pf);
    for (o, v) in [
        (0x58cu32, ONE),
        (0x590, ONE),
        (0x594, ONE),
        (0x598, 0x3d8f_5c29),
    ] {
        g.write::<u32>(at(o), v);
    }
    g.write::<u32>(at(0xd8c), at(0x518));
    let p = g.read::<u32>(at(0xd90));
    let x = mul(g.read::<f32>(p + 0x28), g.read::<f32>(w(0xe7d)));
    g.write::<f32>(p + 0x28, x);
    part_c750(g, img, c, (0x950, 0xa38), (0xe9e, 0xea9));
    particle_c750(g, img, c, 0xec0, (0x950, 0xc50, 0xc));
    crate::m0577::glow_off(g, img, call(0xec8));
    // push 3; push ecx; push T; push ecx (the leftovers, unread: 0)
    let sp = c.base.wrapping_sub(0x10);
    for (i, v) in (0u32..).zip([0, w(0xed0), 0, 3]) {
        g.write::<u32>(sp + 4 * i, v);
    }
    let ef = Frame {
        at: call(0xedc),
        esp: sp.wrapping_sub(4),
        ecx: at(0x12fc),
        ..Frame::default()
    };
    crate::s4_fam446::effect_node_a(g, img, &ef);
    let fl = g.read::<u32>(at(0x1328)) | 6;
    g.write::<u32>(at(0x1328), fl | 1);
    g.write::<u32>(at(0x132c), 0);
    sparks_c750(g, c, 0x32);
    let s = g.read::<u32>(at(0x18)).wrapping_add(1);
    g.write::<u32>(at(0x18), s);
    g.write::<u32>(at(0x20), 0x3e);
}

/// [`burst_task_0577`]'s 0x2711: with both bursts' states 20000 and the
/// part's flag, the owner's node leaves, its models reset, the task
/// unlinked, slot 568 and the current node unlinked.
fn end_c750<G: Game>(g: &mut G, img: &Image, c: &Ctx) {
    let w = |o: u32| img.word(c.f + o);
    let at = |o: u32| c.r.wrapping_add(o);
    if g.read::<u32>(at(0x1360)) != 0x4e20
        || g.read::<u32>(at(0x13f0)) != 0x4e20
        || g.read::<u8>(at(0xa38)) & 4 == 0
    {
        return;
    }
    let task = g.read::<u32>(c.base + 0xc);
    crate::sa_misc::leave(g, at(0x24), 0);
    let grp = c.base + 0xc;
    g.write::<u32>(grp, at(0x24));
    let nf = Frame {
        at: img.rel32_target(c.f + 0x99a),
        esp: c.base.wrapping_sub(4),
        ecx: grp,
        ..Frame::default()
    };
    crate::s4_misc::nodes_reset(g, img, &nf);
    crate::h6_setup::unlink_task(g, task);
    // call width: 1
    let eax = g.call(slot_target(g, w(0x9b5), 568), 0, &[1]).eax;
    crate::l3_common::list_unlink_current(g, img, img.rel32_target(c.f + 0x9cb), eax);
}

/// [`burst_task_0577`] (the `rel32`s: its callees in order,
/// `__security_check_cookie`).
pub const BURST_TASK_0577: Sig = Sig {
    len: 4375,
    hash: 0xabf5_6d49_02a3_6f84,
    calls: &[
        (0x30d, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x3ca, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x59a, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x5bb, Callee::Sig(&crate::m0577::LIGHT_NODE)),
        (0x65c, Callee::Sig(&crate::r29::BURST_SETUP_0577)),
        (0x883, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0x8ee, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x90c, Callee::Sig(&crate::r29::BODY_SETUP_0577)),
        (0x98d, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x99a, Callee::Sig(&crate::s4_misc::NODES_RESET)),
        (0x9cb, Callee::Sig(&crate::l3_common::LIST_UNLINK_CURRENT)),
        (0xa0a, Callee::Any),
        (0xa42, Callee::Sig(&crate::s3_summon::MODEL_NODES)),
        (0xa52, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0xa93, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0xabe, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0xadb, Callee::Sig(&crate::m0070::MODEL_STATE)),
        (0xaff, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0xb3a, Callee::Sig(&crate::s4_small::BIG_NODE)),
        (0xb61, Callee::Sig(&crate::m0070::NODE_INIT_ROOT)),
        (
            0xb75,
            Callee::Sig(&crate::o4_single3::OBJECT_TASK_HEAD_0577_B130),
        ),
        (0xb93, Callee::Sig(&crate::s4_fam446::EFFECT_NODE_A)),
        (0xcd0, Callee::Sig(&crate::m0577::CENTRE)),
        (0xd20, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0xd2b, Callee::Sig(&crate::m0070::FREE_PART)),
        (0xd42, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0xda8, Callee::Sig(&crate::m0577::PLACE)),
        (0xdcb, Callee::Sig(&crate::o4_single3::HEAD_NODE_0577_AC20)),
        (0xe03, Callee::Sig(&crate::o4_tiny::SLOT_IMM8_0577_EC80)),
        (0xe36, Callee::Sig(&crate::m0577::PLACE)),
        (0xe9e, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0xea9, Callee::Sig(&crate::m0070::FREE_PART)),
        (0xec0, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0xec8, Callee::Sig(&crate::m0577::GLOW_OFF)),
        (0xedc, Callee::Sig(&crate::s4_fam446::EFFECT_NODE_A)),
        (0xf53, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0xf5e, Callee::Sig(&crate::m0070::FREE_PART)),
        (0xfbb, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0xfc3, Callee::Sig(&crate::m0577::HALVE)),
        (0x1078, Callee::Sig(&crate::m0577::MOVE_START)),
    ],
    imports: &[],
};

/// `magic_0577`'s scene task (0x100091d0, 5248 bytes; a task callback
/// `(A)`, cdecl; an aligned /GS frame below three pushes; the record `R =
/// A[4]`, its state `R + 0x18`, its wait `R + 0x1c` counting down at
/// every exit, after slot 325 copies the light's place). The registers the
/// dispatch loop keeps (`esi` 0x600, `edi` 0x80, `ecx` 0x2e00, `xmm1` 0,
/// `xmm2` 1.0 and `xmm3` 0.9, the two floats from the code) are the same
/// at every state's entry. By the state (several going on into the next):
/// - 0: the scene's static nodes put last under the DLL's list with their
///   words, slot 262, the sound node, the first effect's words; 0x64 next;
/// - 0x64, 0x65: the camera block's words; the camera's way in (its speed
///   `R + 0x2c`, `R + 0x30` damped by 0.9) for 29 calls; 0x96, 0x97: the
///   camera's second block, slots 707 and 371 twice, a wait of 140;
/// - 0xa0..0xa2: slot 707, the camera words, two damped approaches;
/// - 0xaa, 0xab: slot 286, the model's fade, the camera block, slot 325,
///   two nodes, the sub-walk, an owned node, a particle node, the last
///   spark's words; 0xac, 0xad: the spark's words, slots 229 and 248;
/// - 0x19a..0x19e: slot 286, the fade, an effect node, the part freed,
///   the approaches with their limits (0x2e00, 0x600);
/// - 0x1f4..0x1f6: slots 229, 248, the centre, slot 234, the fade-out
///   (slots 233, 248, 325); 0x258, 0x259: the camera block back, slot 257
///   or 296 by the model's flag; 10000, 0x2711: the sound node leaves and
///   the task's node is unlinked.
///
/// Returns slot 325's `eax`.
#[allow(clippy::too_many_lines)] // the original's states, in order
pub fn scene_task_0577<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, cvtt, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x7e), n);
    // push ebp; and esp, -8; sub esp, 0x2c; three pushes
    let e0 = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x2c);
    g.write::<u32>(e0 + 0x28, g.read::<u32>(w(0xa)) ^ e0);
    let base = e0.wrapping_sub(0xc);
    let task = fr.arg(g, 0);
    g.write::<u32>(base + 0x10, task);
    let r = g.read::<u32>(task + 4);
    let at = |o: u32| r.wrapping_add(o);
    let rf = |g: &G, o: u32| g.read::<f32>(at(o));
    let wait = |g: &G| g.read::<i32>(at(0x1c));
    let set = |g: &mut G, s: u32| g.write::<u32>(at(0x18), s);
    let up = |g: &mut G| {
        let s = g.read::<u32>(at(0x18)).wrapping_add(1);
        g.write::<u32>(at(0x18), s);
    };
    let c = Ctx { f, base, r };
    // the dispatch loop's constant floats
    let damp = k(g, 0x37);
    let mut state = g.read::<u32>(at(0x18));
    loop {
        let again = if state >= 0x19a {
            match state {
                0x19a..=0x19e => {
                    let mut s = state;
                    if s == 0x19a {
                        fade_91d0(g, img, &c);
                        s = 0x19b;
                    }
                    if s == 0x19b {
                        if wait(g) <= 0x27 {
                            part_c750_at(g, img, &c, 0x700, (0x1211, 0x1218), base + 0x10);
                            up(g);
                        }
                        s = 0x19c;
                    }
                    if s == 0x19c {
                        let v = rf(g, 0x2c);
                        let x = add(v, rf(g, 0x28));
                        let v = mul(v, k(g, 0x1230));
                        g.write::<f32>(at(0x28), x);
                        g.write::<f32>(at(0x2c), v);
                        let x = g.read::<u32>(at(0x28));
                        g.write::<u32>(at(0x6ec), x);
                        if wait(g) <= 0 {
                            fade_setup_91d0(g, img, &c, 0x1265, (0x125d, 0x1255));
                            up(g);
                            g.write::<u32>(at(0x20), 0);
                            g.write::<u32>(at(0x24), 0);
                            g.write::<u32>(at(0x1c), 0x28);
                        }
                        false
                    } else if s == 0x19d {
                        let v = rf(g, 0x2c);
                        let x = add(v, rf(g, 0x28));
                        let v = mul(v, k(g, 0x1297));
                        g.write::<f32>(at(0x28), x);
                        g.write::<f32>(at(0x2c), v);
                        let x = g.read::<u32>(at(0x28));
                        g.write::<u32>(at(0x6ec), x);
                        let a = g.read::<u32>(at(0x20)).wrapping_add(0x10);
                        g.write::<u32>(at(0x20), a);
                        let v = g
                            .read::<i32>(at(0x258))
                            .wrapping_add(a.cast_signed())
                            .min(0x2e00);
                        g.write::<i32>(at(0x258), v);
                        let b = g.read::<u32>(at(0x24)).wrapping_add(4);
                        g.write::<u32>(at(0x24), b);
                        let v = g
                            .read::<i32>(at(0x25c))
                            .wrapping_sub(b.cast_signed())
                            .max(0x600);
                        g.write::<i32>(at(0x25c), v);
                        if wait(g) <= 0 {
                            let x = g.read::<u32>(at(0x6ec));
                            up(g);
                            g.write::<u32>(at(0x28), x);
                            g.write::<u32>(at(0x2c), 0x437a_0000);
                            g.write::<u32>(at(0x30), 0xc37a_0000);
                            g.write::<u32>(at(0x1c), 0x1e);
                        }
                        false
                    } else {
                        let v = g.read::<i32>(at(0x258)).wrapping_add(0x100).min(0x2e00);
                        let u = g.read::<i32>(at(0x25c)).wrapping_add(-0x40);
                        g.write::<i32>(at(0x25c), u);
                        g.write::<i32>(at(0x258), v);
                        let u = g.read::<i32>(at(0x25c)).max(0x600);
                        g.write::<i32>(at(0x25c), u);
                        let x0 = mul(rf(g, 0x30), damp);
                        let x1 = add(rf(g, 0x2c), x0);
                        g.write::<f32>(at(0x30), x0);
                        let x = sub(rf(g, 0x28), x1);
                        g.write::<f32>(at(0x6ec), x);
                        if wait(g) > 0 {
                            false
                        } else {
                            set(g, 0x1f4);
                            true
                        }
                    }
                }
                0x1f4..=0x1f6 => {
                    if state == 0x1f4 {
                        centre_91d0(g, img, &c);
                    }
                    if state <= 0x1f5 {
                        let x = sub(rf(g, 0x28), k(g, 0xa71));
                        g.write::<f32>(at(0x28), x);
                        // comiss 0, x; jb: above 0 or unordered stays
                        if !(0.0 < x || x.is_nan()) {
                            up(g);
                            g.write::<u32>(at(0x28), 0);
                        }
                        let v = base + 0x24;
                        let s = g.read::<u32>(at(0x28));
                        // call width: 3
                        g.call(slot(g, 233), 0, &[v, at(0x44), s]);
                        // call width: 3
                        g.call(slot(g, 248), 0, &[at(0x1a8), at(0x34), v]);
                        // call width: 2
                        g.call(slot(g, 325), 0, &[at(0x6a4), at(0x1a8)]);
                    }
                    let a = g.read::<i32>(at(0x20)).wrapping_add(3).min(0x80);
                    g.write::<i32>(at(0x20), a);
                    let v = g.read::<i32>(at(0x25c)).wrapping_add(a);
                    g.write::<i32>(at(0x25c), v);
                    if wait(g) > 0 {
                        false
                    } else {
                        set(g, 0x258);
                        true
                    }
                }
                0x258 | 0x259 => {
                    if state == 0x258 {
                        back_91d0(g, img, &c);
                        up(g);
                        g.write::<u32>(at(0x1c), 0x5a);
                    }
                    if wait(g) > 0 {
                        false
                    } else {
                        set(g, 0x2710);
                        true
                    }
                }
                0x2710 | 0x2711 => {
                    if state == 0x2710 {
                        up(g);
                        g.write::<u32>(at(0x1c), 0);
                    }
                    if wait(g) <= 0 {
                        crate::sa_misc::leave(g, at(0x54), 0);
                        let t = g.read::<u32>(base + 0x10);
                        crate::h6_setup::unlink_task(g, t);
                    }
                    false
                }
                _ => false,
            }
        } else if state > 0xad {
            false
        } else {
            match state {
                0 => {
                    statics_91d0(g, img, &c);
                    set(g, 0x64);
                    true
                }
                0x64 | 0x65 => {
                    if state == 0x64 {
                        camera_91d0(g, img, &c);
                        up(g);
                        g.write::<u32>(at(0x1c), 0x1d);
                    }
                    let x = add(rf(g, 0x28), rf(g, 0x2c));
                    g.write::<f32>(at(0x6ec), x);
                    let y = sub(sub(k(g, 0x5bc), rf(g, 0x30)), k(g, 0x5c9));
                    g.write::<i32>(at(0x25c), cvtt(y));
                    let a = mul(rf(g, 0x2c), damp);
                    g.write::<f32>(at(0x2c), a);
                    let b = mul(rf(g, 0x30), damp);
                    g.write::<f32>(at(0x30), b);
                    if wait(g) > 0 {
                        false
                    } else {
                        set(g, 0x96);
                        true
                    }
                }
                0x96 | 0x97 => {
                    if state == 0x96 {
                        second_91d0(g, img, &c);
                        up(g);
                        g.write::<u32>(at(0x1c), 0x8c);
                    }
                    let v = g.read::<u32>(at(0x25c)).wrapping_add(0x10);
                    g.write::<u32>(at(0x25c), v);
                    if wait(g) > 0 {
                        false
                    } else {
                        set(g, 0xa0);
                        true
                    }
                }
                0xa0 | 0xa1 => {
                    if state == 0xa0 {
                        approach_91d0(g, img, &c);
                        up(g);
                        g.write::<u32>(at(0x2c), 0xc2a0_0000);
                        g.write::<u32>(at(0x1c), 0xb4);
                    }
                    let v = g.read::<u32>(at(0x25c)).wrapping_add(0xffff_fff8);
                    g.write::<u32>(at(0x25c), v);
                    let x = add(sub(k(g, 0xdc9), rf(g, 0x2c)), rf(g, 0x28));
                    g.write::<f32>(at(0x6ec), x);
                    let s = mul(rf(g, 0x2c), k(g, 0xdec));
                    g.write::<f32>(at(0x2c), s);
                    if wait(g) <= 0 {
                        g.write::<u32>(at(0x1c), 0x46);
                        let x = g.read::<u32>(at(0x6ec));
                        up(g);
                        g.write::<u32>(at(0x28), x);
                        g.write::<u32>(at(0x2c), 0x437a_0000);
                    }
                    false
                }
                0xa2 => {
                    let v = g.read::<u32>(at(0x25c)).wrapping_add(0xffff_ffc0);
                    g.write::<u32>(at(0x25c), v);
                    let x0 = mul(rf(g, 0x2c), k(g, 0x7cf));
                    let x1 = sub(k(g, 0x7d7), x0);
                    g.write::<f32>(at(0x2c), x0);
                    let x = sub(rf(g, 0x28), x1);
                    g.write::<f32>(at(0x6ec), x);
                    if wait(g) > 0 {
                        false
                    } else {
                        set(g, 0xaa);
                        true
                    }
                }
                0xaa | 0xab => {
                    if state == 0xaa {
                        model_91d0(g, img, &c);
                        up(g);
                        g.write::<u32>(at(0x1c), 0x32);
                    }
                    if wait(g) <= 0 {
                        spark_91d0(g, (0x420c_0000, ONE, sub(k(g, 0x103e), 0.0)), &c);
                        up(g);
                        g.write::<u32>(at(0x1c), 0x46);
                    }
                    point_91d0(g, img, &c, (0x10b8, 0x10f9, 0x10f3));
                    false
                }
                0xac | 0xad => {
                    if state == 0xac && wait(g) <= 0x28 {
                        spark_91d0(g, (0x41f0_0000, 0, sub(0.0, k(g, 0x2f))), &c);
                        up(g);
                    }
                    point_91d0(g, img, &c, (0x86d, 0x8ae, 0x8a8));
                    if wait(g) > 0 {
                        false
                    } else {
                        set(g, 0x19a);
                        true
                    }
                }
                _ => false,
            }
        };
        if !again {
            break;
        }
        state = g.read::<u32>(at(0x18));
    }
    // call width: 2
    let eax = g.call(slot(g, 325), 0, &[w(0x1368), at(0x694)]).eax;
    let x = g.read::<u32>(at(0x1c)).wrapping_sub(1);
    g.write::<u32>(at(0x1c), x);
    Ret::eax(eax)
}

/// The part `R + o` freed (the frame group at `grp`) unless its flag
/// (byte `+0xe8` bit 2), then [`free_part`](crate::m0070::free_part).
fn part_c750_at<G: Game>(g: &mut G, img: &Image, c: &Ctx, o: u32, calls: (u32, u32), grp: u32) {
    let part = c.r.wrapping_add(o);
    g.write::<u32>(grp, part);
    let ff = Frame {
        at: img.rel32_target(c.f + calls.0),
        esp: c.base.wrapping_sub(4),
        ecx: grp,
        ..Frame::default()
    };
    crate::nodes::free_then_unlink(g, img, &ff);
    crate::m0070::free_part(g, img, img.rel32_target(c.f + calls.1), part, 0);
}

/// The model's fade set-up ([`fade_setup`](crate::o4_single3::fade_setup)
/// on `R + 0x54`, `xmm1`/`xmm2` the floats at the operands `k`; 0 for none).
fn fade_setup_91d0<G: Game>(g: &mut G, img: &Image, c: &Ctx, call: u32, k: (u32, u32)) {
    let w = |o: u32| img.word(c.f + o);
    let x1 = u128::from(g.read::<u32>(w(k.0)));
    let x2 = if k.1 == 0 {
        0
    } else {
        u128::from(g.read::<u32>(w(k.1)))
    };
    let ff = Frame {
        at: img.rel32_target(c.f + call),
        esp: c.base.wrapping_sub(4),
        ecx: c.r.wrapping_add(0x54),
        xmm: [0, x1, x2, 0],
        ..Frame::default()
    };
    crate::o4_single3::fade_setup(g, img, &ff);
}

/// The last spark's words (`R[0x83c]`): flag bit 0, sizes 4.0, `+0x30`
/// and `+0x2c` = `big`, `+0x38` = `w38`, `+0x3c` = `w3c`.
fn spark_91d0<G: Game>(g: &mut G, (big, w38, w3c): (u32, u32, f32), c: &Ctx) {
    let s = g.read::<u32>(c.r.wrapping_add(0x83c));
    let fl = g.read::<u8>(s + 0x40) | 1;
    g.write::<u8>(s + 0x40, fl);
    for (o, v) in [
        (0x1cu32, 0x4080_0000u32),
        (0x20, 0x4080_0000),
        (0x24, 0x4080_0000),
        (0x28, 0x4080_0000),
        (0x30, big),
        (0x2c, big),
        (0x38, w38),
    ] {
        g.write::<u32>(s + o, v);
    }
    g.write::<f32>(s + 0x3c, w3c);
}

/// Slot 229 `(V, R + 0x330, V)` with the frame's `V = (0, 100, -20, 0)`
/// and slot 248 `(R + 0x6a4, G, V)` (`ops`: the imports twice, `G`).
fn point_91d0<G: Game>(g: &mut G, img: &Image, c: &Ctx, ops: (u32, u32, u32)) {
    let w = |o: u32| img.word(c.f + o);
    let v = c.base + 0x24;
    for (i, x) in (0u32..).zip([0, 0x42c8_0000, 0xc1a0_0000, 0]) {
        g.write::<u32>(v + 4 * i, x);
    }
    // call width: 3
    g.call(
        slot_target(g, w(ops.0), 229),
        0,
        &[v, c.r.wrapping_add(0x330), v],
    );
    // call width: 3
    g.call(
        slot_target(g, w(ops.1), 248),
        0,
        &[c.r.wrapping_add(0x6a4), w(ops.2), v],
    );
}

/// [`scene_task_0577`]'s state 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
fn statics_91d0<G: Game>(g: &mut G, img: &Image, c: &Ctx) {
    let w = |o: u32| img.word(c.f + o);
    let at = |o: u32| c.r.wrapping_add(o);
    let f = c.f;
    // call width: 1
    g.call(slot_target(g, w(0x7e), 568), 0, &[0]);
    let n1 = at(0x1b8);
    link_c750(
        g,
        img,
        f,
        n1,
        [0xa4, 0xab, 0xb0, 0xb8, 0xbd, 0xc5, 0xd1, 0xdd],
    );
    g.write::<u32>(n1, w(0xf0));
    let m = g.read::<u32>(at(0x838));
    let mw = g.read::<u32>(m + 0x54);
    let o = at(0x1d4);
    // call width: 4
    g.call(slot_target(g, w(0xea), 262), 0, &[mw, o, o + 4, o + 8]);
    g.write::<u32>(at(0x1e0), ONE);
    let n2 = at(0x268);
    g.write::<u32>(n2, w(0x11b));
    link_c750(
        g,
        img,
        f,
        n2,
        [0x12e, 0x135, 0x13a, 0x142, 0x147, 0x14f, 0x15b, 0x161],
    );
    for (o, v) in [(0x284u32, 0u32), (0x288, 0x10000), (0x28c, 0)] {
        g.write::<u32>(at(o), v);
    }
    let n3 = at(0x3e0);
    g.write::<u32>(n3, w(0x18b));
    link_c750(
        g,
        img,
        f,
        n3,
        [0x19e, 0x1a5, 0x1aa, 0x1b2, 0x1b7, 0x1bf, 0x1cb, 0x1d1],
    );
    for (o, v) in [
        (0x13cu32, n1),
        (0x144, n2),
        (0x140, w(0x1e7)),
        (0x128, w(0x1f1)),
    ] {
        g.write::<u32>(n3 + o, v);
    }
    g.write::<u8>(n3 + 0x148, 0);
    let n4 = at(0x18c);
    link_c750(
        g,
        img,
        f,
        n4,
        [0x211, 0x218, 0x21d, 0x225, 0x22a, 0x232, 0x23e, 0x244],
    );
    g.write::<u32>(n4, w(0x24a));
    g.write::<u32>(at(0x1b4), ONE);
    let n5 = at(0x23c);
    g.write::<u32>(n5, w(0x260));
    link_c750(
        g,
        img,
        f,
        n5,
        [0x273, 0x27a, 0x27f, 0x287, 0x28c, 0x294, 0x2a0, 0x2a6],
    );
    let n6 = at(0x294);
    g.write::<u32>(n6, w(0x2b2));
    link_c750(
        g,
        img,
        f,
        n6,
        [0x2c5, 0x2cc, 0x2d1, 0x2d9, 0x2de, 0x2e6, 0x2f2, 0x2f8],
    );
    for (o, v) in [(0x144u32, n5), (0x13c, n4), (0x140, w(0x30e)), (0x128, n3)] {
        g.write::<u32>(n6 + o, v);
    }
    g.write::<u8>(n6 + 0x148, 0);
    g.write::<u8>(at(0x3dc), 4);
    let n7 = at(0x678);
    link_c750(
        g,
        img,
        f,
        n7,
        [0x33b, 0x342, 0x347, 0x34f, 0x354, 0x35c, 0x368, 0x36e],
    );
    g.write::<u32>(n7 + 0x38, ONE);
    g.write::<u32>(n7 + 0x3c, n3);
    g.write::<u32>(n7, w(0x37e));
    let n8 = at(0x6b8);
    link_c750(
        g,
        img,
        f,
        n8,
        [0x397, 0x39e, 0x3a3, 0x3ab, 0x3b0, 0x3b8, 0x3c4, 0x3ca],
    );
    g.write::<u32>(n8 + 0x38, ONE);
    g.write::<u32>(n8 + 0x3c, n6);
    g.write::<u32>(n8, w(0x3dd));
    // the sound node `[N, list, 600.0]`
    let s = at(0x54);
    let rec = c.base + 0x18;
    for (i, v) in (0u32..).zip([s, w(0x3ed), 0x4416_0000]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    g.write::<u32>(s, w(0x3fb));
    crate::o6_fam_node::node_reset_sound(g, img, img.rel32_target(f + 0x400), rec);
    g.write::<u16>(s + 0x135, 0);
    g.write::<u32>(s + 0x60, 0);
    g.write::<u32>(s + 0x5c, 0);
    g.write::<u32>(s + 0x124, n7);
    g.write::<u32>(s + 0x128, n8);
    g.write::<u32>(w(0x42f), s);
}

/// [`scene_task_0577`]'s state 0x64: the first block's words, the camera
/// block.
fn camera_91d0<G: Game>(g: &mut G, img: &Image, c: &Ctx) {
    use crate::r25::abs_stores;
    use crate::sb_sse::sub;
    let w = |o: u32| img.word(c.f + o);
    let at = |o: u32| c.r.wrapping_add(o);
    let f = c.f;
    for (o, v) in [
        (0x1a8u32, 0x8000_0000u32),
        (0x1ac, 0xc1c8_0000),
        (0x1b0, 0xc140_0000),
        (0x258, 0x3400),
        (0x25c, 0xffff_f000),
        (0x260, 0),
        (0x6a4, 0x8000_0000),
        (0x6a8, 0xc1c8_0000),
        (0x6ac, 0xc140_0000),
        (0x28, 0xc220_0000),
        (0x2c, 0xc1f0_0000),
        (0x30, 0x4500_0000),
        (0x6e4, 0),
        (0x6e8, 0),
    ] {
        g.write::<u32>(at(o), v);
    }
    let x = g.read::<u32>(at(0x28));
    g.write::<u32>(at(0x6ec), x);
    let x0 = sub(g.read::<f32>(w(0x4cf)), g.read::<f32>(w(0x4d7)));
    g.write::<u32>(w(0x4e2), at(0x678));
    let fl = (g.read::<u32>(w(0x4e7)) & !2) | 1;
    g.write::<f32>(w(0x4f2), x0);
    let x1 = sub(g.read::<f32>(w(0x4fa)), g.read::<f32>(w(0x502)));
    abs_stores(g, img, f, 0x509, 0x527);
    g.write::<f32>(w(0x52b), x1);
    let x2 = sub(g.read::<f32>(w(0x533)), g.read::<f32>(w(0x53b)));
    abs_stores(g, img, f, 0x53f, 0x567);
    g.write::<f32>(w(0x56b), x2);
    // xmm1 is 0 here
    let x3 = sub(g.read::<f32>(w(0x573)), 0.0);
    abs_stores(g, img, f, 0x57b, 0x58f);
    g.write::<u32>(w(0x590), fl);
    g.write::<f32>(w(0x598), x3);
}

/// [`scene_task_0577`]'s state 0x96: the camera's second block, slots 707
/// and 371.
fn second_91d0<G: Game>(g: &mut G, img: &Image, c: &Ctx) {
    use crate::r25::abs_stores;
    use crate::sb_sse::sub;
    let w = |o: u32| img.word(c.f + o);
    let at = |o: u32| c.r.wrapping_add(o);
    let f = c.f;
    let x0 = sub(g.read::<f32>(w(0x60d)), g.read::<f32>(w(0x615)));
    let fl = g.read::<u32>(w(0x61a));
    abs_stores(g, img, f, 0x61e, 0x63c);
    g.write::<f32>(w(0x640), x0);
    let x1 = sub(g.read::<f32>(w(0x648)), g.read::<f32>(w(0x650)));
    abs_stores(g, img, f, 0x654, 0x672);
    g.write::<f32>(w(0x679), x1);
    let x2 = sub(g.read::<f32>(w(0x681)), g.read::<f32>(w(0x689)));
    g.write::<u32>(w(0x691), (fl & !2) | 1);
    g.write::<f32>(w(0x6a0), x2);
    g.write::<f32>(w(0x6af), sub(0.0, 0.0));
    let x3 = sub(g.read::<f32>(w(0x6b7)), g.read::<f32>(w(0x6bf)));
    g.write::<f32>(w(0x6c7), x3);
    for (o, v) in [
        (0x1a8u32, 0xc170_0000u32),
        (0x1ac, 0x41cd_999a),
        (0x1b0, 0xc264_0000),
        (0x258, 0xffff_fc00),
        (0x25c, 0xffff_d000),
        (0x260, 0),
        (0x6e4, 0),
        (0x6e8, 0),
        (0x6ec, 0xc2f8_0000),
    ] {
        g.write::<u32>(at(o), v);
    }
    let slot = |g: &G, n: u32| slot_target(g, w(0x696), n);
    // call width: 1
    g.call(slot(g, 707), 0, &[1]);
    // call width: 2
    g.call(slot(g, 371), 0, &[0, 0x5000]);
    // call width: 2
    g.call(slot(g, 371), 0, &[1, 0x1d000]);
    g.write::<u32>(at(0xdc), 0x3e16_cbe4);
    g.write::<u32>(at(0xcc), 0x3e16_cbe4);
    g.write::<u16>(at(0xe4), 0x600);
    g.write::<u32>(at(0xc8), 0);
}

/// [`scene_task_0577`]'s state 0xa0: slot 707, the blocks' words.
fn approach_91d0<G: Game>(g: &mut G, img: &Image, c: &Ctx) {
    use crate::sb_sse::mul;
    let w = |o: u32| img.word(c.f + o);
    let at = |o: u32| c.r.wrapping_add(o);
    // xmm1 is 0 here
    let x = mul(mul(0.0, g.read::<f32>(w(0xcf3))), g.read::<f32>(w(0xcfe)));
    g.write::<f32>(at(0xdc), x);
    g.write::<f32>(at(0xcc), x);
    g.write::<u16>(at(0xe4), 0);
    g.write::<u32>(at(0xc8), 0);
    // call width: 1
    g.call(slot_target(g, w(0xd20), 707), 0, &[0]);
    for (o, v) in [
        (0x1a8u32, 0x40a0_0000u32),
        (0x1ac, 0xc1f0_0000),
        (0x1b0, 0xc040_0000),
        (0x6a4, 0x40a0_0000),
        (0x6a8, 0xc1f0_0000),
        (0x6ac, 0xc040_0000),
        (0x258, 0xffff_fc00),
        (0x25c, 0),
        (0x260, 0),
        (0x28, 0xc1a0_0000),
        (0x6e4, 0),
        (0x6e8, 0),
    ] {
        g.write::<u32>(at(o), v);
    }
    let x = g.read::<u32>(at(0x28));
    g.write::<u32>(at(0x6ec), x);
}

/// [`scene_task_0577`]'s state 0xaa: slot 286, the fade, the camera
/// block, slot 325, nodes, the sub-walk, an owned and a particle node.
#[allow(clippy::too_many_lines)] // the original's steps, in order
fn model_91d0<G: Game>(g: &mut G, img: &Image, c: &Ctx) {
    use crate::r25::abs_stores;
    use crate::sb_sse::sub;
    let w = |o: u32| img.word(c.f + o);
    let at = |o: u32| c.r.wrapping_add(o);
    let f = c.f;
    let slot = |g: &G, n: u32| slot_target(g, w(0xe1b), n);
    let m = g.read::<u32>(at(0x184));
    // call width: 2
    g.call(slot(g, 286), 0, &[m, 0x4396_0000]);
    fade_setup_91d0(g, img, c, 0xe44, (0xe39, 0));
    let x0 = sub(g.read::<f32>(w(0xe4c)), g.read::<f32>(w(0xe54)));
    let fl = (g.read::<u32>(w(0xe59)) & !2) | 1;
    g.write::<u32>(w(0xe64), fl);
    g.write::<f32>(w(0xe6c), x0);
    let x1 = sub(g.read::<f32>(w(0xe74)), g.read::<f32>(w(0xe7c)));
    g.write::<f32>(w(0xe95), x1);
    let x2 = sub(g.read::<f32>(w(0xe9d)), g.read::<f32>(w(0xea5)));
    abs_stores(g, img, f, 0xea9, 0xed1);
    g.write::<f32>(w(0xed5), x2);
    let x3 = sub(0.0, g.read::<f32>(w(0xee0)));
    abs_stores(g, img, f, 0xee4, 0xf0c);
    g.write::<f32>(w(0xf10), x3);
    abs_stores(g, img, f, 0xf14, 0xf1e);
    // call width: 2 (slot 286's words stay under it)
    g.call(slot_target(g, w(0xe8d), 325), 0, &[at(0x1a8), w(0xe87)]);
    let list = w(0xf2d);
    for (o, v) in [
        (0x258u32, 0xffff_dc00u32),
        (0x25c, 0),
        (0x260, 0),
        (0x6e4, 0x4248_0000),
        (0x6e8, 0xc248_0000),
        (0x6ec, 0x43fa_0000),
    ] {
        g.write::<u32>(at(o), v);
    }
    crate::sb_misc::node_append(g, img, img.rel32_target(f + 0xf70), at(0x210), list, 0);
    // the sub-walk's cursor at `E + 0x10` (four words lower)
    let cur = c.base + 0x10;
    g.write::<u32>(cur, at(0x6b8));
    let mark = g.read::<u8>(w(0xf7f)).wrapping_add(1);
    g.write::<u8>(cur + 5, mark);
    let sf = Frame {
        at: img.rel32_target(f + 0xf8e),
        esp: c.base.wrapping_sub(0x14),
        ecx: cur,
        ..Frame::default()
    };
    crate::sa_summon::sub_walk(g, img, &sf);
    // call width: 2
    g.call(slot_target(g, w(0xfa1), 325), 0, &[at(0x22c), at(0x6d4)]);
    for (o, v) in [
        (0x200u32, ONE),
        (0x204, ONE),
        (0x208, ONE),
        (0x20c, 0x3e8f_5c29),
    ] {
        g.write::<u32>(at(o), v);
    }
    crate::sb_misc::node_append(g, img, img.rel32_target(f + 0xfe1), at(0x1e4), w(0xfb4), 0);
    let sp = c.base.wrapping_sub(0x28);
    for (i, v) in (0u32..).zip([w(0x101b), at(0x210), w(0xfe7), at(0x1e4)]) {
        g.write::<u32>(sp + 4 * i, v);
    }
    let of = Frame {
        at: img.rel32_target(f + 0x1022),
        esp: sp.wrapping_sub(4),
        ecx: at(0x52c),
        edx: w(0xfb4),
        ..Frame::default()
    };
    crate::sa_misc::node_append_owned(g, img, &of);
    particle_c750(g, img, c, 0x1036, (0x700, 0x52c, 3));
}

/// [`scene_task_0577`]'s state 0x19a: slot 286, the fade, the blocks'
/// words, an effect node (on into 0x19b in the caller).
fn fade_91d0<G: Game>(g: &mut G, img: &Image, c: &Ctx) {
    let w = |o: u32| img.word(c.f + o);
    let at = |o: u32| c.r.wrapping_add(o);
    let m = g.read::<u32>(at(0x184));
    // call width: 2
    g.call(slot_target(g, w(0x110e), 286), 0, &[m, 0x43be_0000]);
    fade_setup_91d0(g, img, c, 0x113a, (0x112c, 0));
    for (o, v) in [
        (0x1a8u32, 0u32),
        (0x1ac, 0xc37a_0000),
        (0x1b0, 0),
        (0x6a4, 0),
        (0x6a8, 0xc37a_0000),
        (0x6ac, 0),
        (0x258, 0x1000),
        (0x25c, 0x1000),
        (0x260, 0),
        (0x28, 0xc248_0000),
        (0x6e4, 0),
        (0x6e8, 0),
    ] {
        g.write::<u32>(at(o), v);
    }
    let x = g.read::<u32>(at(0x28));
    g.write::<u32>(at(0x6ec), x);
    let s = g.read::<u32>(at(0x18)).wrapping_add(1);
    g.write::<u32>(at(0x18), s);
    g.write::<u32>(at(0x2c), 0xc040_0000);
    g.write::<u32>(at(0x1c), 0x28);
    // push 2; push ecx; push T; push ecx (the leftovers, unread: 0)
    let sp = c.base.wrapping_sub(0x10);
    for (i, v) in (0u32..).zip([0, w(0x11c3), 0, 2]) {
        g.write::<u32>(sp + 4 * i, v);
    }
    let ef = Frame {
        at: img.rel32_target(c.f + 0x11dd),
        esp: sp.wrapping_sub(4),
        ecx: at(0x800),
        ..Frame::default()
    };
    crate::s4_fam446::effect_node_a(g, img, &ef);
    let fl = g.read::<u32>(at(0x82c)) | 6;
    g.write::<u32>(at(0x82c), fl | 1);
    g.write::<u32>(at(0x830), 0);
}

/// [`scene_task_0577`]'s state 0x1f4: slots 229 and 248, the centre, slot
/// 234; on into 0x1f5.
fn centre_91d0<G: Game>(g: &mut G, img: &Image, c: &Ctx) {
    use crate::sb_sse::sub;
    let w = |o: u32| img.word(c.f + o);
    let at = |o: u32| c.r.wrapping_add(o);
    let slot = |g: &G, n: u32| slot_target(g, w(0x982), n);
    let v = c.base + 0x24;
    for (i, x) in (0u32..).zip([0, 0, 0xc38c_0000, 0]) {
        g.write::<u32>(v + 4 * i, x);
    }
    // call width: 3
    g.call(slot(g, 229), 0, &[v, at(0x330), v]);
    // call width: 3
    g.call(slot(g, 248), 0, &[at(0x1a8), at(0x1a8), v]);
    // call width: 3
    g.call(slot(g, 248), 0, &[at(0x6a4), at(0x6a4), v]);
    let x = sub(g.read::<f32>(at(0x6ec)), g.read::<f32>(w(0x9f4)));
    g.write::<f32>(at(0x6ec), x);
    let cf = Frame {
        at: img.rel32_target(c.f + 0xa0e),
        esp: c.base.wrapping_sub(4),
        ecx: at(0x34),
        edx: w(0x9ff),
        ..Frame::default()
    };
    crate::m0577::centre(g, img, &cf);
    // call width: 3
    g.call(slot(g, 234), 0, &[at(0x34), at(0x34), at(0x4ac)]);
    g.write::<u32>(at(0x40), ONE);
    // call width: 3
    g.call(slot(g, 234), 0, &[at(0x44), at(0x1a8), at(0x34)]);
    let s = g.read::<u32>(at(0x18)).wrapping_add(1);
    g.write::<u32>(at(0x18), s);
    g.write::<u32>(at(0x28), ONE);
    g.write::<u32>(at(0x1c), 0x122);
    g.write::<u32>(at(0x20), 0);
}

/// [`scene_task_0577`]'s state 0x258: the camera block back, slot 257 or
/// 296 by the model's flag.
fn back_91d0<G: Game>(g: &mut G, img: &Image, c: &Ctx) {
    use crate::r25::abs_stores;
    use crate::sb_sse::sub;
    let w = |o: u32| img.word(c.f + o);
    let at = |o: u32| c.r.wrapping_add(o);
    let f = c.f;
    let x0 = sub(g.read::<f32>(w(0xb5c)), g.read::<f32>(w(0xb64)));
    // eax is 0 here (the state less 0x258)
    g.write::<u32>(w(0xb69), 0);
    g.write::<u32>(w(0xb6e), 0);
    let fl = g.read::<u32>(w(0xb73));
    abs_stores(g, img, f, 0xb77, 0xb81);
    g.write::<f32>(w(0xb85), x0);
    let x1 = sub(g.read::<f32>(w(0xb8d)), g.read::<f32>(w(0xb95)));
    abs_stores(g, img, f, 0xb99, 0xbb7);
    g.write::<f32>(w(0xbbe), x1);
    let x2 = sub(g.read::<f32>(w(0xbc6)), g.read::<f32>(w(0xbce)));
    g.write::<u32>(w(0xbd6), (fl & !2) | 1);
    g.write::<f32>(w(0xbde), x2);
    g.write::<f32>(w(0xbed), sub(0.0, 0.0));
    let x3 = sub(g.read::<f32>(w(0xbf5)), g.read::<f32>(w(0xbfd)));
    g.write::<f32>(w(0xc05), x3);
    for (o, v) in [
        (0x1a8u32, 0u32),
        (0x1ac, 0xc2a0_0000),
        (0x1b0, 0),
        (0x6a4, 0),
        (0x6a8, 0xc2a0_0000),
        (0x6ac, 0),
        (0x258, 0),
        (0x25c, 0x3600),
        (0x260, 0xffff_dc00),
        (0x6e4, 0),
        (0x6e8, 0),
        (0x6ec, 0xc348_0000),
    ] {
        g.write::<u32>(at(o), v);
    }
    let m = g.read::<u32>(at(0x838));
    if g.read::<u8>(m + 0x90) & 1 != 0 {
        let x = g.read::<u32>(m + 0x54);
        // call width: 2
        g.call(slot_target(g, w(0xc96), 257), 0, &[x, 0]);
    } else {
        let x = g.read::<u32>(m + 0x50);
        // call width: 2
        g.call(slot_target(g, w(0xca8), 296), 0, &[x, 1]);
    }
}

/// [`scene_task_0577`] (the `rel32`s: its callees in order,
/// `__security_check_cookie`).
pub const SCENE_TASK_0577: Sig = Sig {
    len: 5248,
    hash: 0x4c4f_1e0a_3b18_397d,
    calls: &[
        (
            0x400,
            Callee::Sig(&crate::o6_fam_node::NODE_RESET_SOUND_0577_36D0),
        ),
        (0xa0e, Callee::Sig(&crate::m0577::CENTRE)),
        (0xe44, Callee::Sig(&crate::o4_single3::FADE_SETUP_0577_9170)),
        (0xf70, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0xf8e, Callee::Sig(&crate::sa_summon::SUB_WALK)),
        (0xfe1, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x1022, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x1036, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (
            0x113a,
            Callee::Sig(&crate::o4_single3::FADE_SETUP_0577_9170),
        ),
        (0x11dd, Callee::Sig(&crate::s4_fam446::EFFECT_NODE_A)),
        (0x1211, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0x1218, Callee::Sig(&crate::m0070::FREE_PART)),
        (
            0x1265,
            Callee::Sig(&crate::o4_single3::FADE_SETUP_0577_9170),
        ),
        (0x1339, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x1384, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0533`'s summon task (0x10008490, 4001 bytes; a task callback
/// `(A)`, cdecl; an aligned frame below three pushes, no /GS; the record
/// `R = A[4]`, the state the DLL's global `G_S`). Each call first moves
/// `R + 0x18` and `R + 0x1c` on by `K` and counts `R + 0x20` up; then by
/// the state (from the byte and jump tables):
/// - 0: the summon's nodes (slot 530, appends, an owned node, the model
///   set-up, the link node and static node, slots 557/560, the colour
///   node, the camera words, the globals' differences, slots 238, 296,
///   295, the model node, slot 211 and slot 3);
/// - the camera's moves and words through the states (slots 286, 285,
///   674, 250, 557, 560, 719, 306), an appended node and the owned node,
///   the move set-up; the fades (slot 307's float into slot 250);
/// - 10000: the current node and the link node leave, the task's node is
///   unlinked.
///
/// Most states step `G_S` on. Returns `eax` as the original leaves it (the
/// table's byte, the caller's `eax` past the table, the last call's).
#[allow(clippy::too_many_lines)] // the original's states, in order
pub fn summon_task_0533<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r25::abs_stores;
    use crate::sb_sse::{add, cvt, cvtt, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x79), n);
    let abs = |g: &mut G, a: u32, b: u32| abs_stores(g, img, f, a, b);
    // push ebp; and esp, -8; sub esp, 0x14; three pushes
    let base = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x20);
    let task = fr.arg(g, 0);
    let gs = w(0x10);
    let esi = g.read::<u32>(gs);
    let r = g.read::<u32>(task + 4);
    let at = |o: u32| r.wrapping_add(o);
    let rf = |g: &G, o: u32| g.read::<f32>(at(o));
    let x = add(rf(g, 0x18), k(g, 0x21));
    let n = g.read::<u32>(at(0x20)).wrapping_add(1);
    g.write::<u32>(at(0x20), n);
    g.write::<f32>(at(0x18), x);
    let y = add(rf(g, 0x1c), k(g, 0x36));
    g.write::<f32>(at(0x1c), y);
    let step = |g: &mut G, by: u32| g.write::<u32>(gs, esi.wrapping_add(by));
    let bump = |g: &mut G| {
        let s = g.read::<u32>(gs).wrapping_add(1);
        g.write::<u32>(gs, s);
    };
    let words = |g: &mut G, ws: [(u32, u32); 6]| {
        for (o, v) in ws {
            g.write::<u32>(at(o), v);
        }
    };
    let s = esi.cast_signed();
    if s > 0x2710 || (s != 0x2710 && esi > 0x70) {
        return Ret::eax(fr.eax);
    }
    if esi == 0x2710 {
        crate::l3_common::list_unlink_current(g, img, img.rel32_target(f + 0xe8b), fr.eax);
        crate::sa_misc::leave(g, at(0x374), 0);
        return Ret::eax(crate::h6_setup::unlink_task(g, task));
    }
    let idx = img.word(w(0x5d) + esi) & 0xff;
    // the dispatch's two floats
    let (x1, x2) = (k(g, 0x65), k(g, 0x6d));
    let eax = match esi {
        0 => start_0533(g, img, f, base, r),
        1 => {
            let x = add(rf(g, 0x368), k(g, 0x304));
            g.write::<f32>(at(0x368), x);
            let x = sub(rf(g, 0x340), k(g, 0x31c));
            g.write::<f32>(at(0x340), x);
            if g.read::<u32>(at(0x20)) >= 0x28 {
                let x = mul(k(g, 0x336), k(g, 0x33e));
                abs(g, 0x342, 0x34c);
                g.write::<f32>(at(0x368), x);
                abs(g, 0x354, 0x35e);
                step(g, 1);
            }
            idx
        }
        0x51 => {
            step(g, 1);
            idx
        }
        0x5a => {
            g.write::<u8>(at(0x390), 1);
            let m = g.read::<u32>(w(0x388));
            // call width: 2
            let eax = g.call(slot(g, 286), 0, &[m, 0x4422_8000]).eax;
            abs(g, 0x38e, 0x398);
            words(
                g,
                [
                    (0x364, 0),
                    (0x368, 0x47af_c800),
                    (0x36c, 0),
                    (0x338, 0x420c_0000),
                    (0x33c, 0xc402_0000),
                    (0x340, 0x420c_0000),
                ],
            );
            abs(g, 0x3b6, 0x3c0);
            bump(g);
            abs(g, 0x3e7, 0x3f1);
            eax
        }
        0x5b => {
            let fl = g.read::<u32>(w(0x3fa)) | 1;
            g.write::<u32>(w(0x3fa), fl);
            abs(g, 0x405, 0x469);
            g.write::<u32>(at(0x1c), 0);
            let m = g.read::<u32>(w(0x47f));
            // call width: 2
            let eax = g.call(slot(g, 285), 0, &[m, 0x3ef1_463b]).eax;
            bump(g);
            g.write::<u32>(at(0x394), 0x3f73_3333);
            eax
        }
        0x5c => {
            let x = add(rf(g, 0x33c), k(g, 0x4ab));
            let lo = k(g, 0x4b3);
            g.write::<f32>(at(0x33c), x);
            let v = rf(g, 0x1c);
            // comiss lo, v; ja: below `lo` waits
            if lo > v {
                idx
            } else {
                let n = cvtt(mul(sub(v, lo), k(g, 0x4df)));
                g.write::<u8>(at(0x390), n.to_le_bytes()[0]);
                // comiss v, K; jb: below or unordered waits
                if v >= k(g, 0x4d0) {
                    step(g, 1);
                }
                n.cast_unsigned()
            }
        }
        0xa | 0xb => {
            let mut s = esi;
            if esi == 0xa {
                camera_0533(g, img, f, r);
                s = g.read::<u32>(gs).wrapping_add(1);
            }
            crate::sb_misc::node_append(
                g,
                img,
                img.rel32_target(f + 0x5f2),
                at(0x2f0),
                at(0x2c4),
                0,
            );
            for o in [0x30cu32, 0x310, 0x314] {
                g.write::<u32>(at(o), 0);
            }
            g.write::<u32>(w(0x626), at(0x178));
            g.write::<u32>(gs, s.wrapping_add(1));
            fades_0533(g, img, f, r, (0x5fa, 0x63a, 0x653, 0x65b, 0x670), x2);
            at(0x178)
        }
        0x64..=0x66 => {
            let mut eax = idx;
            if esi == 0x64 {
                // call width: 0
                if g.call(slot(g, 520), 0, &[]).eax == 0x024b_0005 {
                    g.write::<u32>(at(0x2e4), 0);
                }
                bump(g);
            }
            let mut fall = esi == 0x66;
            if esi <= 0x65 {
                fades_0533(
                    g,
                    img,
                    f,
                    r,
                    (0x6b0, 0x6c0, 0x6d9, 0x6e1, 0x6f6),
                    k(g, 0x6a8),
                );
                let c = g.read::<u32>(at(0x310)).wrapping_add(1);
                g.write::<u32>(at(0x310), c);
                if g.read::<u8>(at(0x390)) > 2 {
                    let ff = Frame {
                        at: img.rel32_target(f + 0x70f),
                        esp: base.wrapping_sub(4),
                        ..Frame::default()
                    };
                    let v = match crate::s4_misc::float_307(g, img, &ff).xmm0 {
                        Some(crate::Xmm::F32(x)) => f32::from_bits(x),
                        _ => 0.0,
                    };
                    let x = mul(sub(v, k(g, 0x717)), rf(g, 0x24));
                    let cf = Frame {
                        at: img.rel32_target(f + 0x724),
                        esp: base.wrapping_sub(4),
                        xmm: [u128::from(x.to_bits()), u128::from(x.to_bits()), 0, 0],
                        ..Frame::default()
                    };
                    crate::s4_small::call_250(g, img, &cf);
                    let b = u32::from(g.read::<u8>(at(0x390)));
                    let n = cvtt(mul(cvt(b), k(g, 0x73a)));
                    let y = mul(rf(g, 0x24), k(g, 0x74b));
                    g.write::<u8>(at(0x390), n.to_le_bytes()[0]);
                    g.write::<f32>(at(0x24), y);
                    return Ret::eax(n.cast_unsigned());
                }
                // call width: 2
                eax = g.call(slot(g, 250), 0, &[0, 0]).eax;
                let s = g.read::<u32>(gs).wrapping_add(1);
                g.write::<u8>(at(0x390), 1);
                g.write::<u32>(gs, s);
                fall = true;
            }
            if fall {
                let c = g.read::<u32>(at(0x310)).wrapping_add(1);
                g.write::<u32>(at(0x310), c);
                if c.cast_signed() >= 0x80 {
                    let s = g.read::<u32>(gs);
                    let s = if esi == 0x66 { esi + 1 } else { s + 1 };
                    g.write::<u32>(gs, s);
                }
            }
            eax
        }
        0x14 => {
            let m = g.read::<u32>(w(0x7d0));
            // call width: 2
            let eax = g.call(slot(g, 286), 0, &[m, 0x4422_8000]).eax;
            abs(g, 0x7d6, 0x7e0);
            words(
                g,
                [
                    (0x364, 0),
                    (0x368, 0xc248_0000),
                    (0x36c, 0),
                    (0x338, 0),
                    (0x33c, 0xc25c_0000),
                    (0x340, 0x428c_0000),
                ],
            );
            abs(g, 0x7fe, 0x808);
            abs(g, 0x826, 0x830);
            bump(g);
            g.write::<u8>(at(0x390), 0x70);
            g.write::<u32>(at(0x398), 1);
            eax
        }
        0x1e => {
            abs(g, 0x856, 0x860);
            words(
                g,
                [
                    (0x364, 0),
                    (0x368, 0xc120_0000),
                    (0x36c, 0),
                    (0x338, 0),
                    (0x33c, 0xc5ea_6000),
                    (0x340, 0xc72f_c800),
                ],
            );
            abs(g, 0x87e, 0x888);
            abs(g, 0x8a9, 0x8b3);
            g.write::<u8>(at(0x390), 1);
            g.write::<u32>(at(0x398), 0);
            g.write::<u32>(at(0x28), 0x41a0_0000);
            let m = g.read::<u32>(w(0x8e2));
            // call width: 3
            g.call(slot(g, 674), 0, &[m, 0x41a0_0000, 0]);
            let m = g.read::<u32>(w(0x8ff));
            // call width: 2 (slot 674's last word stays under it)
            let eax = g.call(slot(g, 286), 0, &[m, 0x4454_8000]).eax;
            bump(g);
            eax
        }
        0x1f => {
            let x = sub(rf(g, 0x28), k(g, 0x91e));
            g.write::<f32>(at(0x28), x);
            let m = g.read::<u32>(w(0x944));
            // call width: 3
            g.call(slot(g, 674), 0, &[m, x.to_bits(), 0]).eax
        }
        0x28 => {
            abs(g, 0x959, 0x963);
            words(
                g,
                [
                    (0x364, 0xc2c8_0000),
                    (0x368, 0),
                    (0x36c, 0x42f0_0000),
                    (0x338, 0x41f0_0000),
                    (0x33c, 0xc120_0000),
                    (0x340, 0x4320_0000),
                ],
            );
            abs(g, 0x981, 0x98b);
            abs(g, 0x9ac, 0x9b6);
            g.write::<u32>(at(0x398), 0);
            // call width: 2
            let eax = g.call(slot(g, 250), 0, &[0, 0]).eax;
            bump(g);
            eax
        }
        0x29 | 0x33 => {
            let x = sub(rf(g, 0x36c), k(g, 0x9f3));
            g.write::<f32>(at(0x36c), x);
            idx
        }
        0x32 => {
            abs(g, 0xa06, 0xa10);
            words(
                g,
                [
                    (0x364, 0x42c8_0000),
                    (0x368, 0),
                    (0x36c, 0x42c8_0000),
                    (0x338, 0xc1f0_0000),
                    (0x33c, 0xc120_0000),
                    (0x340, 0x4320_0000),
                ],
            );
            abs(g, 0xa2e, 0xa38);
            abs(g, 0xa57, 0xa61);
            step(g, 1);
            idx
        }
        0x3c | 0x3d => {
            let mut s = esi;
            if esi == 0x3c {
                // call width: 3
                let h = g.call(slot(g, 719), 0, &[w(0xa7e), 5, 0]).eax;
                // call width: 3
                g.call(slot(g, 306), 0, &[7, 2, h]);
                s = g.read::<u32>(gs).wrapping_add(1);
            }
            let sp = base.wrapping_sub(0x10);
            for (i, v) in (0u32..).zip([w(0xab6), w(0xaae), w(0xaa9), w(0xaa4)]) {
                g.write::<u32>(sp + 4 * i, v);
            }
            let of = Frame {
                at: img.rel32_target(f + 0xac0),
                esp: sp.wrapping_sub(4),
                ecx: at(0x2c),
                edx: w(0xabb),
                ..Frame::default()
            };
            let eax = crate::sa_misc::node_append_owned(g, img, &of);
            g.write::<u32>(w(0xac6), at(0x2c));
            words(
                g,
                [
                    (0x364, 0),
                    (0x368, 0x41f0_0000),
                    (0x36c, 0),
                    (0x2bc, w(0xaee)),
                    (0x338, 0x41f0_0000),
                    (0x33c, 0xc28c_0000),
                ],
            );
            abs(g, 0xaf2, 0xafc);
            g.write::<u32>(at(0x340), 0xc316_0000);
            abs(g, 0xa57, 0xa61);
            g.write::<u32>(gs, s.wrapping_add(1));
            eax
        }
        0x3e => {
            let x = sub(rf(g, 0x368), k(g, 0xb2e));
            step(g, 1);
            g.write::<f32>(at(0x368), x);
            idx
        }
        0x3f | 0x40 => {
            if esi == 0x3f {
                let fl = g.read::<u32>(w(0xb4a)) | 3;
                g.write::<u32>(w(0xb4a), fl);
                abs(g, 0xb4f, 0xbce);
                let x = sub(rf(g, 0x368), x1);
                step(g, 1);
                g.write::<f32>(at(0x368), x);
            }
            let x = sub(rf(g, 0x368), x1);
            g.write::<f32>(at(0x368), x);
            idx
        }
        0x46 => {
            g.write::<u32>(at(0x154), w(0xc0a));
            g.write::<u32>(at(0x168), w(0xc14));
            words(
                g,
                [
                    (0x368, 0),
                    (0x338, 0x41f0_0000),
                    (0x33c, 0xc1a0_0000),
                    (0x340, 0xc2a0_0000),
                    (0x1c, 0),
                    (0x1c, 0),
                ],
            );
            step(g, 1);
            idx
        }
        0x47 => {
            // comiss R[0x1c], K; jb: below or unordered waits
            if y >= k(g, 0xc58) {
                step(g, 1);
            }
            idx
        }
        0x50 => {
            let fl = g.read::<u32>(w(0xc72)) | 3;
            g.write::<u32>(w(0xc72), fl);
            abs(g, 0xc78, 0xcb4);
            step(g, 1);
            idx
        }
        0x6e => {
            let m = g.read::<u32>(w(0xcd6));
            // call width: 2
            g.call(slot(g, 286), 0, &[m, 0x4480_0000]);
            // call width: 2
            g.call(slot(g, 250), 0, &[0, 0]);
            abs(g, 0xcf8, 0xd02);
            words(
                g,
                [
                    (0x364, 0),
                    (0x368, 0xc28c_0000),
                    (0x36c, 0),
                    (0x338, 0x4248_0000),
                    (0x33c, 0xc316_0000),
                    (0x340, 0xc2c8_0000),
                ],
            );
            abs(g, 0xd20, 0xd2a);
            abs(g, 0xd4f, 0xd9f);
            let rec = base + 0x10;
            for (i, v) in (0u32..).zip([w(0xda3), 0, 0x3c23_d70a, ONE]) {
                g.write::<u32>(rec + 4 * i, v);
            }
            let eax = crate::sa_misc::move_start(g, rec);
            bump(g);
            eax
        }
        0x6f | 0x70 => {
            let mut s = esi;
            if esi == 0x6f {
                let fl = g.read::<u32>(w(0xdd3)) | 3;
                g.write::<u32>(w(0xdd3), fl);
                abs(g, 0xdd9, 0xe33);
                g.write::<u32>(at(0x20), 0);
                s = esi + 1;
                g.write::<u32>(gs, s);
            }
            if g.read::<u32>(at(0x20)) >= 0x4b {
                let fl = g.read::<u32>(w(0xe48)) | 3;
                g.write::<u32>(w(0xe48), fl);
                abs(g, 0xe4e, 0xe76);
                g.write::<u32>(at(0x20), 0);
                g.write::<u32>(gs, s.wrapping_add(1));
            }
            idx
        }
        _ => idx,
    };
    Ret::eax(eax)
}

/// [`summon_task_0533`]'s three fade words after the move (`x2` the 1.0
/// the code holds): `+0x33c` = `K0 - (x2 - R[0x28]) K1`,
/// `+0x340` = `(x2 - R[0x28]) K2 + K3`, `R + 0x28` scaled by `K4` (`ops`
/// the constants).
fn fades_0533<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    r: u32,
    ops: (u32, u32, u32, u32, u32),
    x2: f32,
) {
    use crate::sb_sse::{add, mul, sub};
    let k = |g: &G, o: u32| g.read::<f32>(img.word(f + o));
    let at = |o: u32| r.wrapping_add(o);
    let x0 = k(g, ops.0);
    let x1 = mul(sub(x2, g.read::<f32>(at(0x28))), k(g, ops.1));
    g.write::<f32>(at(0x33c), sub(x0, x1));
    let y = add(
        mul(sub(x2, g.read::<f32>(at(0x28))), k(g, ops.2)),
        k(g, ops.3),
    );
    g.write::<f32>(at(0x340), y);
    let z = mul(g.read::<f32>(at(0x28)), k(g, ops.4));
    g.write::<f32>(at(0x28), z);
}

/// [`summon_task_0533`]'s state 0xa: slots 557/560 and 286, the camera
/// words, slot 285, the move's words; on into 0xb.
fn camera_0533<G: Game>(g: &mut G, img: &Image, f: u32, r: u32) {
    let w = |o: u32| img.word(f + o);
    let at = |o: u32| r.wrapping_add(o);
    let slot = |g: &G, n: u32| slot_target(g, w(0x502), n);
    // call width: 0
    let v = g.call(slot(g, 557), 0, &[]).eax | 4;
    // call width: 1
    g.call(slot(g, 560), 0, &[v]);
    let m = g.read::<u32>(w(0x52d));
    // call width: 2
    g.call(slot(g, 286), 0, &[m, 0x43a5_0000]);
    let x = g.read::<u32>(at(0x2e4)).wrapping_add(0xffff_c000);
    g.write::<u32>(at(0x2e4), x);
    g.write::<u8>(at(0x390), 0);
    crate::r25::abs_stores(g, img, f, 0x54f, 0x559);
    for (o, v) in [(0x364u32, 0u32), (0x368, 0xc0a0_0000), (0x36c, 0)] {
        g.write::<u32>(at(o), v);
    }
    crate::r25::abs_stores(g, img, f, 0x57a, 0x584);
    g.write::<u32>(at(0x338), 0);
    crate::r25::abs_stores(g, img, f, 0x58e, 0x598);
    let m = g.read::<u32>(w(0x5a7));
    // call width: 2
    g.call(slot(g, 285), 0, &[m, 0]);
    for (o, v) in [(0x28u32, ONE), (0x24, 0x41f0_0000), (0x1c, 0), (0x394, ONE)] {
        g.write::<u32>(at(o), v);
    }
    g.write::<u8>(at(0x390), 0x55);
}

/// [`summon_task_0533`]'s state 0. Returns slot 3's `eax`.
#[allow(clippy::too_many_lines)] // the original's steps, in order
fn start_0533<G: Game>(g: &mut G, img: &Image, f: u32, base: u32, r: u32) -> u32 {
    use crate::r25::abs_stores;
    let w = |o: u32| img.word(f + o);
    let call = |o: u32| img.rel32_target(f + o);
    let at = |o: u32| r.wrapping_add(o);
    let slot = |g: &G, n: u32| slot_target(g, w(0x79), n);
    let m = g.read::<u32>(w(0x84));
    // call width: 2
    g.call(slot(g, 530), 0, &[m, 0x100]);
    let list = w(0x97);
    crate::sa_misc::node_append(g, img, call(0x9c), at(0x2c4), list, 0);
    for o in [0x2e0u32, 0x2e4, 0x2e8] {
        g.write::<u32>(at(o), 0);
    }
    let sp = base.wrapping_sub(0x18);
    for (i, v) in (0u32..).zip([w(0xac), w(0xa7), at(0x2c4), w(0xa1)]) {
        g.write::<u32>(sp + 4 * i, v);
    }
    let of = Frame {
        at: call(0xd5),
        esp: sp.wrapping_sub(4),
        ecx: at(0x178),
        edx: list,
        ..Frame::default()
    };
    crate::sa_misc::node_append_owned(g, img, &of);
    g.write::<u8>(at(0x2c0), 1);
    crate::sb_misc::node_append(g, img, call(0xe9), at(0x31c), list, 0);
    g.write::<u32>(at(0x344), ONE);
    crate::sb_misc::node_append(g, img, call(0xfe), at(0x348), list, 0);
    g.write::<u32>(at(0x370), ONE);
    // the model set-up: four frame words, then (B, B, A)
    let sp = base.wrapping_sub(0x20);
    for (i, v) in [(4u32, at(0x348)), (5, at(0x348)), (6, at(0x31c))] {
        g.write::<u32>(sp + 4 * i, v);
    }
    let mf = Frame {
        at: call(0x11b),
        esp: sp.wrapping_sub(4),
        ecx: w(0x116),
        ..Frame::default()
    };
    crate::m0533::model_setup_args(g, img, &mf);
    abs_stores(g, img, f, 0x127, 0x131);
    let m = g.read::<u32>(w(0x140));
    // call width: 2
    g.call(slot(g, 286), 0, &[m, 0x4461_0000]);
    let lf = Frame {
        at: call(0x14d),
        esp: base.wrapping_sub(0xc),
        ecx: at(0x374),
        ..Frame::default()
    };
    crate::s4_small::link_node(g, img, &lf);
    g.write::<u32>(at(0x394), ONE);
    g.write::<u8>(at(0x390), 0x30);
    crate::sa_misc::static_node(g, img, call(0x163));
    // call width: 0
    let v = g.call(slot(g, 557), 0, &[]).eax & !4;
    // call width: 1
    g.call(slot(g, 560), 0, &[v]);
    let grp = base + 0x10;
    g.write::<u32>(grp, w(0x190));
    g.write::<u32>(grp + 4, w(0x198));
    abs_stores(g, img, f, 0x19c, 0x1a6);
    let cf = Frame {
        at: call(0x1a7),
        esp: base.wrapping_sub(4),
        ecx: grp,
        ..Frame::default()
    };
    crate::s3_misc::colour_node(g, img, &cf);
    let x2 = g.read::<u32>(w(0x1af));
    abs_stores(g, img, f, 0x1b3, 0x1bd);
    for (o, v) in [(0x364u32, 0u32), (0x368, 0xc66a_6000), (0x36c, 0)] {
        g.write::<u32>(at(o), v);
    }
    abs_stores(g, img, f, 0x1db, 0x1e5);
    for (o, v) in [
        (0x338u32, 0x457a_0000u32),
        (0x33c, 0xc5da_c000),
        (0x340, 0xc61c_4000),
    ] {
        g.write::<u32>(at(o), v);
    }
    abs_stores(g, img, f, 0x205, 0x255);
    let df = Frame {
        at: call(0x256),
        esp: base.wrapping_sub(4),
        edx: 0,
        xmm: [0, 0, u128::from(x2), 0],
        ..Frame::default()
    };
    crate::s7_misc::global_differences(g, img, &df);
    let m = g.read::<u32>(w(0x263));
    // call width: 2
    g.call(slot(g, 238), 0, &[m, 1]);
    let m = g.read::<u32>(w(0x278));
    // call width: 2
    g.call(slot(g, 296), 0, &[m, 0]);
    let m = g.read::<u32>(w(0x28d));
    // call width: 2
    g.call(slot(g, 295), 0, &[m, 0]);
    let fl = u32::from(g.read::<u8>(w(0x29c)));
    for (i, v) in (0u32..).zip([w(0x2a8), w(0x2b0), fl]) {
        g.write::<u32>(grp + 4 * i, v);
    }
    g.write::<u8>(grp + 0xc, 0);
    abs_stores(g, img, f, 0x2bd, 0x2c7);
    crate::sa_summon::model_node_init(g, img, call(0x2c8), grp);
    let sf = Frame {
        at: call(0x2cd),
        esp: base.wrapping_sub(0x1c),
        ..Frame::default()
    };
    crate::s4_small::call_211_flag(g, img, &sf);
    let m = g.read::<u32>(w(0x2d8));
    // call width: 1
    let eax = g.call(slot(g, 3), 0, &[m]).eax;
    let s = g.read::<u32>(w(0x2e6)).wrapping_add(1);
    g.write::<u32>(w(0x2e6), s);
    g.write::<u32>(at(0x20), 0);
    eax
}

/// [`summon_task_0533`] (the `rel32`s: its callees in order).
pub const SUMMON_TASK_0533: Sig = Sig {
    len: 4001,
    hash: 0xb3f3_b374_c3f8_443d,
    calls: &[
        (0x9c, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0xd5, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0xe9, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0xfe, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x11b, Callee::Sig(&crate::m0533::MODEL_SETUP_ARGS)),
        (0x14d, Callee::Sig(&crate::s4_small::LINK_NODE)),
        (0x163, Callee::Sig(&crate::sa_misc::STATIC_NODE)),
        (0x1a7, Callee::Sig(&crate::s3_misc::COLOUR_NODE)),
        (0x256, Callee::Sig(&crate::s7_misc::GLOBAL_DIFFERENCES)),
        (0x2c8, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0x2cd, Callee::Sig(&crate::s4_small::CALL_211_FLAG)),
        (0x5f2, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x70f, Callee::Sig(&crate::s4_misc::FLOAT_307)),
        (0x724, Callee::Sig(&crate::s4_small::CALL_250)),
        (0xac0, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0xdc0, Callee::Sig(&crate::sa_misc::MOVE_START)),
        (0xe8b, Callee::Sig(&crate::l3_common::LIST_UNLINK_CURRENT)),
        (0xe96, Callee::Sig(&crate::sa_misc::LEAVE)),
    ],
    imports: &[],
};

/// `magic_0570`'s summon task (0x1000b3e0, 4953 bytes; a task callback
/// `(A)`, cdecl; an aligned /GS frame `E` of 0x8c bytes below three
/// pushes). The record `R = A[4]` keeps the state `R + 0x18`, two floats
/// `R + 0x1c`/`R + 0x20` and the counts `R + 0x28`/`R + 0x2c` (both up at
/// every exit; `R + 0x2c` is the wait most states test). States leave in
/// one of three ways: on (the state steps by one), wait (the state stays)
/// or again (the next state runs in the same call). By the state:
/// - 0: slots 719/306, the small node, an effect node, the colour node,
///   then 0xa: the colour block's words (`0.5 - v`), on;
/// - 0xc: after 10 calls the summon's nodes, model node, slot 211, the
///   value, the show, slot 6, the model state, slot 366 unless the flag,
///   the matrices (slots 230/325/229), an owned and a particle node; 0x1e;
/// - 0x1e, 0x1f, 0x20 (after 0x50, 0x50, 0x5a calls): appends, slot 262,
///   owned, particle and effect nodes; the static node and its move, 0x3c;
/// - 0x3c, 0x3d: the root node and the light; the big, small and static
///   nodes, three appends, an owned node, particle nodes and the bone;
/// - 0x46..0x49: slot 546, the hide, slot 6, the scale's fall, 0x64;
/// - 0x14..0x16, 0x64..0x67: the value, the parts' bits, slots 229, 538,
///   255, 228, the static node, slots 719 and the node's start, an effect
///   node, the floats;
/// - 0x50, 0x51, 0x5a..0x5f: the model's place, slot 411 (on to 0x5a),
///   slots 719/306, the frees, slots 229/255/670/228, the bones' loop, the
///   effect and particle nodes, the move start;
/// - 0x3e, 0x32..0x34, 0x28, 0x29, 0x68, 0x60: slot 234, the effect node
///   and the free, slot 313, the camera words, then 10000 (again), 0x2711
///   and 0x2712: the parts freed, the static node leaves (slot 846), the
///   two models set up, the task unlinked.
///
/// Returns `eax` as the original leaves it.
#[allow(clippy::too_many_lines)] // the original's states, in order
pub fn summon_task_0570<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r25::abs_stores;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let call = |o: u32| img.rel32_target(f + o);
    let slot = |g: &G, n: u32| slot_target(g, w(0x5f), n);
    let abs = |g: &mut G, a: u32, b: u32| abs_stores(g, img, f, a, b);
    // push ebp; and esp, -8; sub esp, 0x8c; three pushes
    let e0 = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x8c);
    g.write::<u32>(e0 + 0x88, g.read::<u32>(w(0xd)) ^ e0);
    let base = e0.wrapping_sub(0xc);
    let c = Ctx570 { f, base };
    let l = |o: u32| base.wrapping_add(o);
    let below = base.wrapping_sub(4);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    g.write::<u32>(l(0x14), r);
    let at = |o: u32| r.wrapping_add(o);
    let rf = |g: &G, o: u32| g.read::<f32>(at(o));
    let wait = |g: &G, n: i32| g.read::<i32>(at(0x2c)) < n;
    let set = |g: &mut G, s: u32| g.write::<u32>(at(0x18), s);
    let up = |g: &mut G| {
        let s = g.read::<u32>(at(0x18)).wrapping_add(1);
        g.write::<u32>(at(0x18), s);
    };
    let reset = |g: &mut G| g.write::<u32>(at(0x2c), 0);
    let flag = |g: &mut G, o: u32, or: u32, and: u32| {
        let x = (g.read::<u32>(w(o)) | or) & and;
        g.write::<u32>(w(o), x);
    };
    let table = w(0x49);
    let mut state = g.read::<u32>(at(0x18));
    // (eax, on): `on` steps the state before the counts go up
    let (eax, on) = loop {
        if state > 0x2710 {
            if state == 0x2711 {
                abs(g, 0x1234, 0x123e);
                break (0, true);
            }
            if state == 0x2712 {
                break (end_0570(g, img, &c, task), false);
            }
            break (state.wrapping_sub(0x2712), false);
        }
        if state == 0x2710 {
            up(g);
            reset(g);
            abs(g, 0x1234, 0x123e);
            break (0x2710, true);
        }
        if state > 0x68 {
            break (state, false);
        }
        let idx = img.word(table + state) & 0xff;
        match state {
            0x3e => {
                if wait(g, 8) {
                    break (idx, false);
                }
                abs(g, 0x68, 0xb8);
                // call width: 3
                g.call(slot(g, 234), 0, &[w(0xc4), w(0xbf), w(0x64)]);
                flag(g, 0xcc, 0, !1);
                abs(g, 0xd8, 0xf6);
                let rec = l(0x28);
                let words = [w(0xfa), w(0x102), w(0x10a), w(0x112), w(0x11a), 2];
                for (i, x) in (0u32..).zip(words) {
                    g.write::<u32>(rec + 4 * i, x);
                }
                crate::sb_nodes::effect_node(g, img, call(0x127), rec);
                let node = g.read::<u32>(rec);
                g.write::<u32>(node, w(0x135));
                flag(g, 0x13b, 1, !0);
                abs(g, 0x140, 0x14a);
                g.write::<u32>(l(0x10), w(0x14e));
                let ff = Frame {
                    at: call(0x153),
                    esp: below,
                    ecx: l(0x10),
                    ..Frame::default()
                };
                crate::nodes::free_then_unlink(g, img, &ff);
                reset(g);
                set(g, 0x32);
            }
            0x32..=0x34 => {
                if state == 0x32 {
                    set(g, 0x33);
                }
                if state <= 0x33 {
                    up(g);
                    reset(g);
                }
                set(g, 0x28);
            }
            0x28 => {
                // call width: 1
                let eax = g.call(slot(g, 313), 0, &[2]).eax;
                if eax != 0 {
                    break (eax, false);
                }
                up(g);
            }
            0x29 => {
                reset(g);
                set(g, 0x46);
            }
            0x68 => {
                let x = add(rf(g, 0x20), k(g, 0x1bb));
                g.write::<f32>(w(0x1c3), x);
                let y = sub(k(g, 0x1cb), rf(g, 0x1c));
                g.write::<f32>(w(0x1d8), y);
                if wait(g, 0x1e) {
                    break (idx, false);
                }
                set(g, 0x5a);
            }
            0x60 => {
                if wait(g, 0x7d) {
                    break (idx, false);
                }
                set(g, 0x2710);
            }
            0 | 0xa => {
                let mut eax = idx;
                if state == 0 {
                    eax = start_0570(g, img, &c);
                    set(g, 0xa);
                }
                colour_block_0570(g, img, f);
                break (eax, true);
            }
            0xc => {
                if wait(g, 0xa) {
                    break (idx, false);
                }
                let eax = summon_0570(g, img, &c);
                reset(g);
                set(g, 0x1e);
                break (eax, false);
            }
            0x1e => {
                if wait(g, 0x50) {
                    break (idx, false);
                }
                let eax = rise_0570(g, img, &c);
                reset(g);
                break (eax, true);
            }
            0xb => {
                reset(g);
                break (idx, true);
            }
            0x1f => break (idx, !wait(g, 0x50)),
            0x20 => {
                if wait(g, 0x5a) {
                    break (idx, false);
                }
                // push ecx (the state, unread); push V
                let sp = base.wrapping_sub(8);
                g.write::<u32>(sp, w(0x861));
                g.write::<u32>(sp + 4, state);
                let sf = Frame {
                    at: call(0x86b),
                    esp: sp.wrapping_sub(4),
                    ecx: w(0x866),
                    ..Frame::default()
                };
                let eax = crate::s4_misc::static_node(g, img, &sf).eax;
                let speed = u128::from(g.read::<u32>(w(0x873)));
                abs(g, 0x87d, 0x8e1);
                // the static node keeps `ecx`
                let mf = Frame {
                    at: call(0x8e2),
                    esp: below,
                    ecx: w(0x866),
                    eax,
                    xmm: [0, 0, speed, speed],
                    ..Frame::default()
                };
                let eax = crate::s4_small::move_setup(g, img, &mf).eax;
                set(g, 0x3c);
                break (eax, false);
            }
            0x3c => {
                if wait(g, 0x75) {
                    break (idx, false);
                }
                crate::m0070::node_init_root(g, img, call(0x902), w(0x8fd));
                let level = u128::from(g.read::<u32>(w(0x90a)));
                abs(g, 0x90e, 0x95e);
                let lf = Frame {
                    at: call(0x95f),
                    esp: below,
                    xmm: [0, level, 0, 0],
                    ..Frame::default()
                };
                let eax = crate::s4_small::light_on(g, img, &lf).eax;
                reset(g);
                break (eax, true);
            }
            0x3d => {
                if wait(g, 7) {
                    break (idx, false);
                }
                break (statics_0570(g, img, &c), true);
            }
            0x46 => {
                if wait(g, 0x22) {
                    break (idx, false);
                }
                let m = g.read::<u32>(w(0xb3f));
                // call width: 1
                break (g.call(slot(g, 546), 0, &[m]).eax, true);
            }
            0x47 => {
                crate::m0070::hide_model(g, img, call(0xb59), w(0xb54));
                let m = g.read::<u32>(w(0xb66));
                // call width: 2
                let eax = g.call(slot(g, 6), 0, &[m, 0]).eax;
                g.write::<u32>(at(0x1c), ONE);
                g.write::<u32>(at(0x20), 0x3c23_d70a);
                break (eax, true);
            }
            0x48 | 0x49 => {
                if state == 0x48 {
                    up(g);
                }
                let v = mul(rf(g, 0x1c), k(g, 0xb91));
                g.write::<f32>(at(0x1c), v);
                let x = sub(k(g, 0xb9e), v);
                g.write::<f32>(w(0xbaa), x);
                if !wait(g, 0x74) {
                    reset(g);
                    set(g, 0x64);
                }
                break (idx, false);
            }
            0x14 => {
                let eax = crate::m0042::set_value(g, img, call(0xbd6), w(0xbd1), 0xde);
                abs(g, 0xbda, 0xbe4);
                reset(g);
                break (eax, true);
            }
            0x15 => {
                if wait(g, 2) {
                    break (idx, false);
                }
                flag(g, 0xbfc, 2, !0);
                flag(g, 0xc03, 2, !0);
                break (idx, true);
            }
            0x16 => {
                if !wait(g, 0x59) {
                    abs(g, 0xc17, 0xc21);
                    set(g, 0x64);
                }
                break (idx, false);
            }
            0x64 => {
                abs(g, 0xc2d, 0xc37);
                reset(g);
                break (idx, true);
            }
            0x65 => {
                if wait(g, 1) {
                    break (idx, false);
                }
                break (lift_0570(g, img, &c), true);
            }
            0x66 => {
                if wait(g, 0xc) {
                    break (idx, false);
                }
                break (lights_0570(g, img, &c), true);
            }
            0x67 => {
                if wait(g, 0xc) {
                    break (idx, false);
                }
                g.write::<u32>(at(0x1c), 0x4000_0000);
                g.write::<u32>(at(0x20), 0xbf00_0000);
                reset(g);
                break (idx, true);
            }
            0x50 => {
                let eax = crate::m0070::model_position(g, img, call(0xe01), w(0xdfc));
                let x = sub(k(g, 0xe09), k(g, 0xe11));
                abs(g, 0xe15, 0xe29);
                g.write::<f32>(w(0xe2d), x);
                let x = add(k(g, 0xe35), k(g, 0xe3d));
                g.write::<f32>(w(0xe45), x);
                reset(g);
                break (eax, true);
            }
            0x51 => {
                let b = u32::from(g.read::<u8>(w(0xe60)));
                let x = sub(k(g, 0xe59), k(g, 0xe68));
                g.write::<f32>(w(0xe76), x);
                // call width: 1
                let eax = g.call(slot(g, 411), 0, &[b]).eax;
                if eax != 0 {
                    set(g, 0x5a);
                }
                break (eax, false);
            }
            0x5a => {
                // call width: 3
                let h = g.call(slot(g, 719), 0, &[w(0xea9), 5, 0]).eax;
                // call width: 3
                let eax = g.call(slot(g, 306), 0, &[7, 1, h]).eax;
                abs(g, 0xec7, 0xed1);
                break (eax, true);
            }
            0x5b => {
                let eax = frees_0570(g, img, &c);
                reset(g);
                break (eax, true);
            }
            0x5c => {
                if wait(g, 0x20) {
                    break (idx, false);
                }
                let eax = bones_0570(g, img, &c);
                reset(g);
                break (eax, true);
            }
            0x5d => {
                if wait(g, 0x35) {
                    break (idx, false);
                }
                // push 5; push ecx (the state, unread); B; A
                let words = [w(0x10fb), w(0x10f6), state, 5];
                let eax = c.effect(g, img, (0x1105, w(0x1100)), words);
                flag(g, 0x110e, 1, !0);
                abs(g, 0x1113, 0x111d);
                break (eax, true);
            }
            0x5e => {
                if wait(g, 0x3c) {
                    break (idx, false);
                }
                abs(g, 0x113b, 0x1177);
                c.particle(g, img, (0x1178, w(0x1132)), (w(0x1137), 8));
                // push 4; push ecx (the leftover, unread: 0); B; A
                let words = [w(0x1188), w(0x1183), 0, 4];
                let eax = c.effect(g, img, (0x1192, w(0x118d)), words);
                flag(g, 0x119b, 1, !0);
                abs(g, 0x11a0, 0x11aa);
                break (eax, true);
            }
            0x5f => {
                if wait(g, 0x64) {
                    break (idx, false);
                }
                abs(g, 0x11bd, 0x1203);
                let rec = l(0x14);
                for (i, x) in (0u32..).zip([w(0x1207), 0, 0x3d4c_cccd, ONE]) {
                    g.write::<u32>(rec + 4 * i, x);
                }
                break (crate::sa_misc::move_start(g, rec), true);
            }
            _ => break (idx, false),
        }
        state = g.read::<u32>(at(0x18));
    };
    if on {
        up(g);
    }
    for o in [0x28u32, 0x2c] {
        let x = g.read::<u32>(at(o)).wrapping_add(1);
        g.write::<u32>(at(o), x);
    }
    Ret::eax(eax)
}

/// [`summon_task_0570`]'s code offset and frame `E` (after the pushes).
struct Ctx570 {
    f: u32,
    base: u32,
}

impl Ctx570 {
    /// The DLL's node list (operand `+0x4af`).
    fn list(&self, img: &Image) -> u32 {
        img.word(self.f + 0x4af)
    }

    /// `magic_0570`'s node append at 0x10008c00 (69 bytes) through the
    /// call at `c`, the list in `edx`.
    fn append<G: Game>(&self, g: &mut G, img: &Image, c: u32, node: u32) {
        let t = img.rel32_target(self.f + c);
        crate::sb_misc::node_append(g, img, t, node, self.list(img), 0);
    }

    /// The other append (0x10008c50, 63 bytes).
    fn append_a<G: Game>(&self, g: &mut G, img: &Image, c: u32, node: u32) {
        let t = img.rel32_target(self.f + c);
        crate::sa_misc::node_append(g, img, t, node, self.list(img), 0);
    }

    /// An owned node (`ecx`) appended under the list with four stack
    /// words at `sp` (the call `c`).
    fn owned<G: Game>(
        &self,
        g: &mut G,
        img: &Image,
        (c, ecx): (u32, u32),
        words: [u32; 4],
        sp: u32,
    ) {
        for (i, v) in (0u32..).zip(words) {
            g.write::<u32>(sp + 4 * i, v);
        }
        let of = Frame {
            at: img.rel32_target(self.f + c),
            esp: sp.wrapping_sub(4),
            ecx,
            edx: self.list(img),
            ..Frame::default()
        };
        crate::sa_misc::node_append_owned(g, img, &of);
    }

    /// A particle node (`ecx`) with `(a, -, -, n)` (the middle words the
    /// frame's) four words below `E`. Returns its `eax`.
    fn particle<G: Game>(
        &self,
        g: &mut G,
        img: &Image,
        (c, ecx): (u32, u32),
        (a, n): (u32, u32),
    ) -> u32 {
        let sp = self.base.wrapping_sub(0x10);
        g.write::<u32>(sp, a);
        g.write::<u32>(sp + 0xc, n);
        let pf = Frame {
            at: img.rel32_target(self.f + c),
            esp: sp.wrapping_sub(4),
            ecx,
            ..Frame::default()
        };
        crate::s3_nodes::particle_node_with(g, img, &pf)
    }

    /// An effect node (`ecx`) with four stack words below `E`. Returns
    /// the node.
    fn effect<G: Game>(
        &self,
        g: &mut G,
        img: &Image,
        (c, ecx): (u32, u32),
        words: [u32; 4],
    ) -> u32 {
        let sp = self.base.wrapping_sub(0x10);
        for (i, v) in (0u32..).zip(words) {
            g.write::<u32>(sp + 4 * i, v);
        }
        let ef = Frame {
            at: img.rel32_target(self.f + c),
            esp: sp.wrapping_sub(4),
            ecx,
            ..Frame::default()
        };
        crate::s3_nodes::effect_node_with(g, img, &ef)
    }

    /// A group word `[E + 0x10]` = `node` freed and unlinked (the call
    /// `c1`), then the part `node` freed (`c2`). Returns the free's `eax`.
    fn free<G: Game>(
        &self,
        g: &mut G,
        img: &Image,
        node: u32,
        (c1, c2): (u32, u32),
        eax: u32,
    ) -> u32 {
        let grp = self.base + 0x10;
        g.write::<u32>(grp, node);
        let ff = Frame {
            at: img.rel32_target(self.f + c1),
            esp: self.base.wrapping_sub(4),
            ecx: grp,
            eax,
            ..Frame::default()
        };
        let eax = crate::nodes::free_then_unlink(g, img, &ff).eax;
        crate::m0070::free_part(g, img, img.rel32_target(self.f + c2), node, eax)
    }
}

/// [`summon_task_0570`]'s state 0: slots 719/306, the small node, an
/// effect node, the colour node. Returns the colour node's `eax`.
fn start_0570<G: Game>(g: &mut G, img: &Image, c: &Ctx570) -> u32 {
    use crate::r25::abs_stores;
    let (f, base) = (c.f, c.base);
    let w = |o: u32| img.word(f + o);
    let call = |o: u32| img.rel32_target(f + o);
    let slot = |g: &G, n: u32| slot_target(g, w(0x2dc), n);
    // call width: 3
    let h = g.call(slot(g, 719), 0, &[w(0x2eb), 5, 0]).eax;
    // call width: 3
    g.call(slot(g, 306), 0, &[7, 0, h]);
    g.write::<u32>(base.wrapping_sub(4), w(0x30f));
    crate::m0070::node_init_small(g, img, call(0x319), w(0x314), w(0x30a), w(0x30f));
    abs_stores(g, img, f, 0x320, 0x32a);
    abs_stores(g, img, f, 0x33c, 0x350);
    // push 0; push ecx (the leftover, unread: 0); B; A
    c.effect(g, img, (0x351, w(0x338)), [w(0x333), w(0x32e), 0, 0]);
    let fl = g.read::<u32>(w(0x356)) | 7;
    abs_stores(g, img, f, 0x367, 0x371);
    g.write::<u32>(w(0x372), fl);
    let grp = base + 0x14;
    g.write::<u32>(grp, w(0x37a));
    g.write::<u32>(grp + 4, w(0x382));
    abs_stores(g, img, f, 0x386, 0x390);
    let cf = Frame {
        at: call(0x391),
        esp: base.wrapping_sub(4),
        ecx: grp,
        ..Frame::default()
    };
    crate::s3_misc::colour_node(g, img, &cf)
}

/// [`summon_task_0570`]'s state 0xa: the colour node's block (`0.5 - v`).
fn colour_block_0570<G: Game>(g: &mut G, img: &Image, f: u32) {
    use crate::r25::abs_stores;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let half = k(g, 0x3a0);
    let rest = |g: &G, o: u32| sub(half, k(g, o));
    let x = rest(g, 0x3ab);
    let fl = g.read::<u32>(w(0x3b1)) & !1;
    g.write::<u32>(w(0x3b1), fl);
    abs_stores(g, img, f, 0x3b6, 0x3d4);
    g.write::<f32>(w(0x3d8), x);
    let x = rest(g, 0x3e3);
    abs_stores(g, img, f, 0x3e7, 0x40f);
    g.write::<f32>(w(0x413), x);
    let x = rest(g, 0x41e);
    abs_stores(g, img, f, 0x422, 0x436);
    g.write::<f32>(w(0x43a), x);
    let x = rest(g, 0x445);
    g.write::<f32>(w(0x44d), x);
    let x = rest(g, 0x458);
    g.write::<f32>(w(0x460), x);
    let x = rest(g, 0x46b);
    let y = rest(g, 0x473);
    g.write::<f32>(w(0x47b), x);
    let x = sub(k(g, 0x483), k(g, 0x48b));
    g.write::<f32>(w(0x493), y);
    g.write::<f32>(w(0x49b), x);
}

/// [`summon_task_0570`]'s state 0xc past its wait. Returns the particle
/// node's `eax`.
fn summon_0570<G: Game>(g: &mut G, img: &Image, c: &Ctx570) -> u32 {
    use crate::r25::abs_stores;
    let (f, base) = (c.f, c.base);
    let l = |o: u32| base.wrapping_add(o);
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let call = |o: u32| img.rel32_target(f + o);
    let slot = |g: &G, n: u32| slot_target(g, w(0x59b), n);
    c.append(g, img, 0x4b9, w(0x4b4));
    let x = g.read::<u32>(w(0x4c1));
    g.write::<u32>(w(0x4c9), x);
    let x = add(k(g, 0x4d1), k(g, 0x4d9));
    abs_stores(g, img, f, 0x4e2, 0x4ec);
    g.write::<f32>(w(0x4f0), x);
    let x = add(k(g, 0x4f8), k(g, 0x500));
    g.write::<f32>(w(0x508), x);
    c.append_a(g, img, 0x50d, w(0x4de));
    abs_stores(g, img, f, 0x516, 0x534);
    c.append(g, img, 0x535, w(0x512));
    let owned = [w(0x545), w(0x540), w(0x53b), w(0x512)];
    c.owned(g, img, (0x54f, w(0x54a)), owned, base.wrapping_sub(0x10));
    // the owned node's last word stays under the model node's
    let m = g.read::<u32>(w(0x55d));
    let sp = base.wrapping_sub(8);
    g.write::<u32>(sp, m);
    let mf = Frame {
        at: call(0x562),
        esp: sp.wrapping_sub(4),
        ecx: w(0x557),
        edx: c.list(img),
        ..Frame::default()
    };
    crate::s4_misc::model_node_with(g, img, &mf);
    let af = Frame {
        at: call(0x571),
        esp: sp.wrapping_sub(4),
        ecx: w(0x56c),
        edx: w(0x567),
        ..Frame::default()
    };
    crate::s4_small::attach_211(g, img, &af);
    crate::m0042::set_value(g, img, call(0x580), w(0x57b), 0xb0);
    // the show's cursor `[E + 0x1c]`: the model, the byte 0xff
    g.write::<u32>(l(0x1c), w(0x58c));
    g.write::<u8>(l(0x20), 0xff);
    crate::m0070::show(g, img, call(0x596), l(0x1c));
    let m = g.read::<u32>(w(0x5a3));
    // call width: 2
    g.call(slot(g, 6), 0, &[m, 1]);
    g.write::<u32>(l(0x14), w(0x5b7));
    g.write::<u32>(l(0x18), w(0x5bf));
    crate::m0070::model_state(g, img, call(0x5c4), l(0x14));
    c.append(g, img, 0x5d3, w(0x5ce));
    if g.read::<u8>(w(0x5d9)) & 1 == 0 {
        let m = g.read::<u32>(w(0x5ec));
        // call width: 2
        g.call(slot(g, 366), 0, &[m, w(0x5e6)]);
    }
    let x = sub(k(g, 0x5ff), k(g, 0x607));
    g.write::<f32>(w(0x61e), x);
    abs_stores(g, img, f, 0x622, 0x62c);
    let (v, m50, m80) = (l(0x40), l(0x50), l(0x80));
    for (i, x) in (0u32..).zip([0, 0, 0, ONE]) {
        g.write::<u32>(v + 4 * i, x);
    }
    // call width: 2
    g.call(slot(g, 230), 0, &[m50, w(0x610)]);
    // call width: 2
    g.call(slot(g, 325), 0, &[m80, w(0x65c)]);
    // call width: 3
    g.call(slot(g, 229), 0, &[v, m50, v]);
    let x = add(g.read::<f32>(v), k(g, 0x694));
    g.write::<f32>(w(0x6a9), x);
    let x = add(g.read::<f32>(l(0x48)), k(g, 0x6b7));
    g.write::<f32>(w(0x6bf), x);
    c.append(g, img, 0x6c4, w(0x6a1));
    abs_stores(g, img, f, 0x6dd, 0x705);
    let owned = [w(0x6d4), w(0x6cf), w(0x6ca), w(0x6a1)];
    c.owned(g, img, (0x706, w(0x6d9)), owned, base.wrapping_sub(0x10));
    abs_stores(g, img, f, 0x70d, 0x714);
    c.particle(g, img, (0x720, w(0x71b)), (w(0x6d9), 0))
}

/// [`summon_task_0570`]'s state 0x1e past its wait: two appends, slot
/// 262, an owned, a particle and an effect node. Returns the effect
/// node.
fn rise_0570<G: Game>(g: &mut G, img: &Image, c: &Ctx570) -> u32 {
    use crate::r25::abs_stores;
    let f = c.f;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    abs_stores(g, img, f, 0x74e, 0x758);
    c.append(g, img, 0x759, w(0x74a));
    let m = g.read::<u32>(w(0x779));
    // call width: 4
    g.call(
        slot_target(g, w(0x75e), 262),
        0,
        &[m, w(0x773), w(0x76e), w(0x763)],
    );
    let x = sub(k(g, 0x783), k(g, 0x78b));
    abs_stores(g, img, f, 0x799, 0x7a3);
    g.write::<f32>(w(0x7a7), x);
    c.append(g, img, 0x7ac, w(0x795));
    abs_stores(g, img, f, 0x7c5, 0x7ed);
    // slot 262's words stay under the owned node's
    let owned = [w(0x7bc), w(0x7b7), w(0x7b2), w(0x795)];
    c.owned(g, img, (0x7ee, w(0x7c1)), owned, c.base.wrapping_sub(0x20));
    abs_stores(g, img, f, 0x7f5, 0x7fc);
    c.particle(g, img, (0x808, w(0x803)), (w(0x7c1), 1));
    // push 1; push ecx (the leftover, unread: 0); B; A
    let eax = c.effect(g, img, (0x822, w(0x81d)), [w(0x818), w(0x813), 0, 1]);
    let fl = g.read::<u32>(w(0x82b)) | 1;
    g.write::<u32>(w(0x82b), fl);
    abs_stores(g, img, f, 0x830, 0x83a);
    eax
}

/// [`summon_task_0570`]'s state 0x3d past its wait: the big, small and
/// static nodes, three appends, an owned node, particle nodes, the bone.
/// Returns the last particle node's `eax`.
fn statics_0570<G: Game>(g: &mut G, img: &Image, c: &Ctx570) -> u32 {
    use crate::r25::abs_stores;
    let (f, base) = (c.f, c.base);
    let w = |o: u32| img.word(f + o);
    let call = |o: u32| img.rel32_target(f + o);
    let grp = base + 0x14;
    for (i, x) in (0u32..).zip([w(0x981), w(0x989), w(0x991), w(0x999), w(0x9a1)]) {
        g.write::<u32>(grp + 4 * i, x);
    }
    crate::m0070::node_init_big(g, grp);
    let x = g.read::<u32>(w(0x9ab));
    let sp = base.wrapping_sub(4);
    g.write::<u32>(sp, w(0x9b0));
    abs_stores(g, img, f, 0x9be, 0x9c8);
    g.write::<u32>(w(0x9c9), x);
    abs_stores(g, img, f, 0x9cd, 0x9d7);
    crate::m0070::node_init_small(g, img, call(0x9d8), w(0x9ba), w(0x9b5), w(0x9b0));
    abs_stores(g, img, f, 0x9dc, 0x9fa);
    let lf = Frame {
        at: call(0x9fb),
        esp: sp.wrapping_sub(4),
        ..Frame::default()
    };
    let r = crate::s4_small::static_link_845(g, img, &lf);
    if let Some(t) = r.jump {
        g.call(t, 0, &[]);
    }
    c.append(g, img, 0xa0d, w(0xa08));
    abs_stores(g, img, f, 0xa16, 0xa3e);
    c.append_a(g, img, 0xa3f, w(0xa12));
    abs_stores(g, img, f, 0xa48, 0xa66);
    c.append(g, img, 0xa67, w(0xa44));
    abs_stores(g, img, f, 0xa80, 0xaa8);
    let owned = [w(0xa77), w(0xa72), w(0xa6d), w(0xa44)];
    c.owned(g, img, (0xaa9, w(0xa7c)), owned, base.wrapping_sub(0x10));
    abs_stores(g, img, f, 0xab0, 0xab7);
    c.particle(g, img, (0xac3, w(0xabe)), (w(0xa7c), 7));
    let p = g.read::<u32>(w(0xac8));
    abs_stores(g, img, f, 0xacc, 0xaf4);
    g.write::<u32>(p + 0x28, 0x3e01_0625);
    let bf = Frame {
        at: call(0xb04),
        esp: base.wrapping_sub(4),
        ecx: w(0xaf8),
        ..Frame::default()
    };
    crate::s4_small::bone_rate(g, img, &bf);
    let eax = c.particle(g, img, (0xb18, w(0xb0e)), (w(0xb13), 4));
    abs_stores(g, img, f, 0xb1f, 0xb29);
    eax
}

/// [`summon_task_0570`]'s state 0x65 past its wait: the vector `(0, -,
/// -50, 1)` through slot 229, its `y` lowered, slot 538's answer kept,
/// slot 255 on it, the parts' bits cleared, slot 228. Returns slot 228's
/// `eax`.
fn lift_0570<G: Game>(g: &mut G, img: &Image, c: &Ctx570) -> u32 {
    let f = c.f;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, n: u32| slot_target(g, w(0xc59), n);
    let v = c.base + 0x40;
    // `E + 0x44` is never written before slot 229 reads it (junk read:
    // the port uses 0, brief 2026-10-24)
    for (i, x) in (0u32..).zip([0, 0, 0xc248_0000, ONE]) {
        g.write::<u32>(v + 4 * i, x);
    }
    // call width: 3
    g.call(slot(g, 229), 0, &[v, w(0xc53), v]);
    let x = sub(g.read::<f32>(v + 4), g.read::<f32>(w(0xc87)));
    let m = g.read::<u32>(w(0xc97));
    g.write::<f32>(v + 4, x);
    // call width: 2
    let eax = g.call(slot(g, 538), 0, &[m, 0]).eax;
    g.write::<u32>(w(0xcb3), eax);
    crate::m0070::part_call(g, img, img.rel32_target(f + 0xcb8), 255, [eax, w(0xcae), v]);
    for o in [0xcc3u32, 0xcca] {
        let fl = g.read::<u32>(w(o)) & !2;
        g.write::<u32>(w(o), fl);
    }
    let m = g.read::<u32>(w(0xcdc));
    // call width: 2
    g.call(slot(g, 228), 0, &[m, 0xb1]).eax
}

/// [`summon_task_0570`]'s state 0x66 past its wait: the static node,
/// slots 719 twice, the node's start and an effect node. Returns the
/// effect node.
fn lights_0570<G: Game>(g: &mut G, img: &Image, c: &Ctx570) -> u32 {
    use crate::r25::abs_stores;
    let (f, base) = (c.f, c.base);
    let w = |o: u32| img.word(f + o);
    let call = |o: u32| img.rel32_target(f + o);
    let slot = |g: &G, n: u32| slot_target(g, w(0xd07), n);
    // push b; push a; sub esp, 0xc (the three words the frame's)
    let sp = base.wrapping_sub(0x14);
    g.write::<u32>(sp + 0xc, w(0xcfa));
    g.write::<u32>(sp + 0x10, w(0xcf5));
    let sf = Frame {
        at: call(0xd02),
        esp: sp.wrapping_sub(4),
        ..Frame::default()
    };
    crate::s4_small::static_node_args(g, img, &sf);
    // call width: 3
    let a = g.call(slot(g, 719), 0, &[w(0xd19), 9, 0]).eax;
    g.write::<u32>(w(0xd23), a);
    // call width: 3
    let b = g.call(slot(g, 719), 0, &[w(0xd37), 9, 0]).eax;
    let rate = u128::from(g.read::<u32>(w(0xd41)));
    g.write::<u32>(w(0xd4e), b);
    abs_stores(g, img, f, 0xd52, 0xda2);
    let nf = Frame {
        at: call(0xda3),
        esp: base.wrapping_sub(4),
        ecx: w(0xd49),
        xmm: [0, rate, 0, 0],
        ..Frame::default()
    };
    crate::sa_misc::node_start(g, img, &nf);
    // push 3; push ecx (the leftover, unread: 0); B; A
    let eax = c.effect(g, img, (0xdba, w(0xdb5)), [w(0xdb0), w(0xdab), 0, 3]);
    let fl = g.read::<u32>(w(0xdc3)) | 1;
    g.write::<u32>(w(0xdc3), fl);
    abs_stores(g, img, f, 0xdc8, 0xdd2);
    eax
}

/// [`summon_task_0570`]'s state 0x5b: three parts freed, the vector `(0,
/// 0, 30, 1)` through slot 229 and slot 255, slots 670, 228 and 670 (the
/// camera words between). Returns the last slot's `eax`.
fn frees_0570<G: Game>(g: &mut G, img: &Image, c: &Ctx570) -> u32 {
    let f = c.f;
    let w = |o: u32| img.word(f + o);
    let slot = |g: &G, n: u32| slot_target(g, w(0xf33), n);
    // the first free's `eax` is the dispatch's (the state's table byte)
    let mut eax = img.word(w(0x49) + 0x5b) & 0xff;
    for (n, c1, c2) in [
        (0xedeu32, 0xee3u32, 0xeedu32),
        (0xef9, 0xefe, 0xf08),
        (0xf14, 0xf19, 0xf23),
    ] {
        eax = c.free(g, img, w(n), (c1, c2), eax);
    }
    let v = c.base + 0x40;
    for (i, x) in (0u32..).zip([0, 0, 0x41f0_0000, ONE]) {
        g.write::<u32>(v + 4 * i, x);
    }
    // call width: 3
    let eax = g.call(slot(g, 229), 0, &[v, w(0xf2d), v]).eax;
    crate::m0070::part_call(g, img, img.rel32_target(f + 0xf6c), 255, [eax, w(0xf67), v]);
    let m = g.read::<u32>(w(0xf79));
    // call width: 2
    g.call(slot(g, 670), 0, &[m, 0]);
    let m = g.read::<u32>(w(0xf91));
    crate::r25::abs_stores(g, img, f, 0xf95, 0xfc7);
    // call width: 2
    g.call(slot(g, 228), 0, &[m, 0xb3]);
    let m = g.read::<u32>(w(0xfd8));
    // call width: 2
    g.call(slot(g, 670), 0, &[m, 0]).eax
}

/// [`summon_task_0570`]'s state 0x5c past its wait: an append, then for
/// each of the bones (0x14c-byte owners from `+0x1047` to `+0x10ce`, 0x2c
/// nodes, 0x100 particle nodes, a table of words): an append, slot 237,
/// the owned node and a particle node. Returns the last particle node's
/// `eax`.
fn bones_0570<G: Game>(g: &mut G, img: &Image, c: &Ctx570) -> u32 {
    let (f, base) = (c.f, c.base);
    let w = |o: u32| img.word(f + o);
    c.append(g, img, 0x1008, w(0x1003));
    crate::r25::abs_stores(g, img, f, 0x100c, 0x1034);
    let cur = base + 0x10;
    g.write::<u32>(cur, w(0x103d));
    let (mut part, mut node, mut owner) = (w(0x1035), w(0x1042), w(0x1047));
    loop {
        c.append(g, img, 0x1058, node);
        let p = g.read::<u32>(cur);
        let pv = g.read::<u32>(p);
        let m = g.read::<u32>(w(0x106f));
        // call width: 4
        g.call(slot_target(g, w(0x1069), 237), 0, &[m, pv, 0, node + 0x1c]);
        g.write::<u32>(node + 0x28, ONE);
        // slot 237's words stay under the owned node's
        let words = [w(0x1087), node, w(0x1081), w(0x107c)];
        c.owned(g, img, (0x109a, owner), words, base.wrapping_sub(0x20));
        g.write::<u8>(owner + 0x148, 1);
        let eax = c.particle(g, img, (0x10b1, part), (owner, 5));
        let p = g.read::<u32>(cur).wrapping_add(4);
        g.write::<u32>(cur, p);
        owner = owner.wrapping_add(0x14c);
        node = node.wrapping_add(0x2c);
        part = part.wrapping_add(0x100);
        if owner.cast_signed() >= w(0x10ce).cast_signed() {
            return eax;
        }
    }
}

/// [`summon_task_0570`]'s state 0x2712: the part freed, the static node
/// leaves (slot 846), the light node leaves, the two models' parts (slots
/// 531, 255) and set-ups, the task unlinked. Returns the task node's next.
fn end_0570<G: Game>(g: &mut G, img: &Image, c: &Ctx570, task: u32) -> u32 {
    let (f, base) = (c.f, c.base);
    let w = |o: u32| img.word(f + o);
    let call = |o: u32| img.rel32_target(f + o);
    let grp = base + 0x14;
    g.write::<u32>(grp, w(0x22a));
    // `eax` is 0 here (0x2712 - 0x2711 - 1)
    let ff = Frame {
        at: call(0x22f),
        esp: base.wrapping_sub(4),
        ecx: grp,
        ..Frame::default()
    };
    let eax = crate::nodes::free_then_unlink(g, img, &ff).eax;
    let eax = crate::m0070::free_part(g, img, call(0x239), w(0x234), eax);
    let lf = Frame {
        at: call(0x23e),
        esp: base.wrapping_sub(4),
        eax,
        ..Frame::default()
    };
    let r = crate::s4_small::static_leave_846(g, img, &lf);
    let mut eax = match r.jump {
        Some(t) => g.call(t, 0, &[]).eax,
        None => r.eax,
    };
    eax = crate::sa_misc::leave(g, w(0x243), eax);
    let v = base + 0x40;
    // (the parts' calls' operands, the leave's, the set-up's, a cleared word)
    for (n1, c531, n2, c255, n3, setup, clear) in [
        (
            0x251u32, 0x256u32, 0x25fu32, 0x264u32, 0x269u32, 0x27bu32, None,
        ),
        (0x284, 0x289, 0x292, 0x297, 0x29c, 0x2b8, Some(0x2a2u32)),
    ] {
        eax = crate::m0070::part_call(g, img, call(c531), 531, [eax, w(n1), v]);
        eax = crate::m0070::part_call(g, img, call(c255), 255, [eax, w(n2), v]);
        if let Some(o) = clear {
            g.write::<u32>(w(o), 0);
        }
        // the leave keeps `ecx`, the node
        let node = w(n3);
        crate::sa_misc::leave(g, node, eax);
        g.write::<u32>(grp, node);
        let mf = Frame {
            at: call(setup),
            esp: base.wrapping_sub(4),
            ecx: grp,
            ..Frame::default()
        };
        eax = crate::sa_summon::model_setup(g, img, &mf).eax;
    }
    crate::h6_setup::unlink_task(g, task)
}

/// [`summon_task_0570`] (the `rel32`s: its callees in order,
/// `__security_check_cookie`).
pub const SUMMON_TASK_0570: Sig = Sig {
    len: 4953,
    hash: 0xc5d3_089f_cfb4_4d99,
    calls: &[
        (0x127, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
        (0x153, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0x22f, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0x239, Callee::Sig(&crate::m0070::FREE_PART)),
        (0x23e, Callee::Sig(&crate::s4_small::STATIC_LEAVE_846)),
        (0x248, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x256, Callee::Sig(&crate::m0070::PART_CALL_531)),
        (0x264, Callee::Sig(&crate::m0070::PART_CALL_255)),
        (0x26e, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x27b, Callee::Sig(&crate::sa_summon::MODEL_SETUP)),
        (0x289, Callee::Sig(&crate::m0070::PART_CALL_531)),
        (0x297, Callee::Sig(&crate::m0070::PART_CALL_255)),
        (0x2ab, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x2b8, Callee::Sig(&crate::sa_summon::MODEL_SETUP)),
        (0x319, Callee::Sig(&crate::m0070::NODE_INIT_SMALL)),
        (0x351, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0x391, Callee::Sig(&crate::s3_misc::COLOUR_NODE)),
        (0x4b9, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x50d, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0x535, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x54f, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x562, Callee::Sig(&crate::s4_misc::MODEL_NODE_WITH)),
        (0x571, Callee::Sig(&crate::s4_small::ATTACH_211)),
        (0x580, Callee::Sig(&crate::m0042::SET_VALUE)),
        (0x596, Callee::Sig(&crate::m0070::SHOW)),
        (0x5c4, Callee::Sig(&crate::m0070::MODEL_STATE)),
        (0x5d3, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x6c4, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x706, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x720, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x759, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x7ac, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x7ee, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x808, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x822, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0x86b, Callee::Sig(&crate::s4_misc::STATIC_NODE)),
        (0x8e2, Callee::Sig(&crate::s4_small::MOVE_SETUP)),
        (0x902, Callee::Sig(&crate::m0070::NODE_INIT_ROOT)),
        (0x95f, Callee::Sig(&crate::s4_small::LIGHT_ON)),
        (0x9a6, Callee::Sig(&crate::m0070::NODE_INIT_BIG)),
        (0x9d8, Callee::Sig(&crate::m0070::NODE_INIT_SMALL)),
        (0x9fb, Callee::Sig(&crate::s4_small::STATIC_LINK_845)),
        (0xa0d, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0xa3f, Callee::Sig(&crate::sa_misc::NODE_APPEND)),
        (0xa67, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0xaa9, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0xac3, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0xb04, Callee::Sig(&crate::s4_small::BONE_RATE)),
        (0xb18, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0xb59, Callee::Sig(&crate::m0070::HIDE_MODEL)),
        (0xbd6, Callee::Sig(&crate::m0042::SET_VALUE)),
        (0xcb8, Callee::Sig(&crate::m0070::PART_CALL_255)),
        (0xd02, Callee::Sig(&crate::s4_small::STATIC_NODE_ARGS)),
        (0xda3, Callee::Sig(&crate::sa_misc::NODE_START)),
        (0xdba, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0xe01, Callee::Sig(&crate::m0070::MODEL_POSITION)),
        (0xee3, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0xeed, Callee::Sig(&crate::m0070::FREE_PART)),
        (0xefe, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0xf08, Callee::Sig(&crate::m0070::FREE_PART)),
        (0xf19, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        (0xf23, Callee::Sig(&crate::m0070::FREE_PART)),
        (0xf6c, Callee::Sig(&crate::m0070::PART_CALL_255)),
        (0x1008, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x1058, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x109a, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x10b1, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x1105, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0x1178, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x1192, Callee::Sig(&crate::s3_nodes::EFFECT_NODE_WITH)),
        (0x1224, Callee::Sig(&crate::sa_misc::MOVE_START)),
        (0x1254, Callee::Any),
    ],
    imports: &[],
};
