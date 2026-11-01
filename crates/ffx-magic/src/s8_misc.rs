//! Shared functions of the shared-8 list (the rows of the shared work
//! list, `worklist5.tsv`, that the earlier shared rounds left) without a
//! family module yet. Each doc names the DLL and address it was ported
//! from, its size and how many DLLs carry it; "`G`" is a global of the
//! DLL (an operand of the code).

// The originals' float tests are `comiss`/`ucomiss` with their own
// unordered outcomes: `!(a >= b)` and exact `==` say what they do.
#![allow(clippy::neg_cmp_op_on_partial_ord, clippy::float_cmp)]
// Long ports follow the original's order of stores and calls.
#![allow(clippy::too_many_lines)]

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::sb_sse::{add, cvt, div, mul, sub};
use crate::{Callee, Frame, Game, Image, Out, Ret, Sig};

/// The word at `f + off` of the code (an operand).
fn op(img: &Image, f: u32, off: u32) -> u32 {
    img.word(f + off)
}

/// The `f32` at `a`.
fn f32_at<G: Game>(g: &G, a: u32) -> f32 {
    g.read::<f32>(a)
}

/// `cvttss2si`: truncated, `0x80000000` when out of range or NaN.
fn trunc(x: f32) -> u32 {
    crate::s4_summon::trunc(x)
}

/// One of the three camera rigs' tracks: the state word and where the
/// track (`ecx` of its step functions) and its output (`edx`) are, from
/// the rig `E`; the step's time `+0`, step `+4` and count `+8` (from the
/// track's time word), the end point copied when the blend ends, and the
/// operand offsets of the slot table and of the step calls' `rel32`s.
struct Track {
    state: u32,
    at: u32,
    out: u32,
    time: u32,
    end: u32,
    table: u32,
    calls: [u32; 4],
}

/// The rig's two point tracks (their outputs go to `MsCameraSetRect`
/// with 0 and with 1).
const TRACKS: [Track; 2] = [
    Track {
        state: 0x10,
        at: 0x2c,
        out: 0x7c,
        time: 0,
        end: 0x1c,
        table: 0x61,
        calls: [0x114, 0x121, 0x12e, 0x13b],
    },
    Track {
        state: 0x30u32.wrapping_neg(),
        at: 0x6c,
        out: 0x8c,
        time: 0x40u32.wrapping_neg(),
        end: 0x5c,
        table: 0x185,
        calls: [0x24e, 0x25e, 0x26e, 0x27e],
    },
];

/// One track's step (state 1: the blend by slot 326 at the time `+0`,
/// moved on by `+4`, until it reaches `K1`, then the end point; 2: a
/// drift of the track's three floats by `end` and of the output by them,
/// for `+8` steps; 3-6: the key and angle steps; anything else: nothing).
/// `k1` is the operand `K1` was last loaded from (the original keeps it
/// in `xmm1` across the tracks); returns the one after.
fn track_step<G: Game>(
    g: &mut G,
    img: &Image,
    fr: &Frame,
    e: u32,
    t: &Track,
    k1: u32,
    esp: u32,
) -> u32 {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let at = |o: u32| e.wrapping_add(o);
    let below = |o: u32| e.wrapping_sub(o);
    let state = g.read::<u32>(at(t.state)).wrapping_sub(1);
    let (track, out, end) = (below(t.at), below(t.out), below(t.end));
    match state {
        0 => {
            let k1 = if t.state == 0x10 { w(0x41) } else { k1 };
            let time = at(t.time);
            let x = add(f32_at(g, time + 4), f32_at(g, time));
            g.write::<f32>(time, x);
            if f32_at(g, k1) > x {
                let target = slot_target(g, w(t.table), 326);
                g.call(target, 0, &[out, end, track, x.to_bits()]);
                return w(0x143);
            }
            for i in 0..4 {
                let v = g.read::<u32>(end + 4 * i);
                g.write::<u32>(out + 4 * i, v);
            }
            g.write::<u32>(at(t.state), 0);
            k1
        }
        1 => {
            for i in 0..3 {
                let x = add(f32_at(g, end + 4 * i), f32_at(g, track + 4 * i));
                g.write::<f32>(track + 4 * i, x);
            }
            for i in 0..3 {
                let (o, d) = (out + 4 * i, track + 4 * i);
                // the first track adds the drift to the output, the second
                // the output to the drift (the operands' order)
                let x = if t.state == 0x10 {
                    add(f32_at(g, d), f32_at(g, o))
                } else {
                    add(f32_at(g, o), f32_at(g, d))
                };
                g.write::<f32>(o, x);
            }
            let left = at(t.time + 8);
            let n = g.read::<i32>(left).wrapping_sub(1);
            g.write::<i32>(left, n);
            if n <= 0 {
                g.write::<u32>(at(t.state), 0);
            }
            if t.state == 0x10 { w(0xa7) } else { k1 }
        }
        2..=5 => {
            let call = t.calls[(state - 2) as usize];
            let callee = Frame {
                at: img.rel32_target(f + call),
                esp,
                eax: state,
                ecx: track,
                edx: out,
                xmm: [0; 4],
            };
            match state {
                2 => {
                    crate::s3_misc::key_step(g, img, &callee);
                }
                3 => {
                    crate::s3_misc::key_step_curved(g, img, &callee);
                }
                4 => {
                    crate::s4_misc::angle_step(g, img, &callee);
                }
                _ => {
                    crate::s4_misc::angle_step_curved(g, img, &callee);
                }
            }
            w(0x143)
        }
        _ => w(0x143),
    }
}

/// The roll (`+0x2c` its state; the result `-0x94`): state 1 blends
/// from `+0x18` to `+0x1c` over `+0x28` steps (`+0x24` left), wrapped
/// by `KC` when past `KB` or below `KD`, and ends on `+0x20`; state 2
/// eases the angle keys at `*(+0x14)` (`+0x28` of them, `+0x24` steps
/// left) at the time `+0x18` moved on by `+0x1c` ([`ease_angle`]), and
/// ends on the last key. Returns the operand `K1` comes from next.
///
/// [`ease_angle`]: crate::sb_misc::ease_angle
fn roll_step<G: Game>(g: &mut G, img: &Image, f: u32, e: u32, esp: u32) -> u32 {
    let w = |o: u32| op(img, f, o);
    let roll = e.wrapping_sub(0x94);
    match g.read::<u32>(e + 0x2c) {
        1 => {
            let left = g.read::<u32>(e + 0x24).wrapping_sub(1);
            g.write::<u32>(e + 0x24, left);
            if left == 0 {
                let v = g.read::<u32>(e + 0x20);
                g.write::<u32>(roll, v);
                g.write::<u32>(e + 0x2c, 0);
                return w(0x324);
            }
            let k = div(cvt(left), cvt(g.read::<u32>(e + 0x28)));
            let from = mul(sub(f32_at(g, w(0x324)), k), f32_at(g, e + 0x18));
            let v = add(from, mul(f32_at(g, e + 0x1c), k));
            let v = if v > f32_at(g, w(0x35d)) {
                sub(v, f32_at(g, w(0x367)))
            } else if f32_at(g, w(0x37c)) > v {
                add(v, f32_at(g, w(0x389)))
            } else {
                v
            };
            g.write::<f32>(roll, v);
            w(0x324)
        }
        2 => {
            let left = g.read::<u32>(e + 0x24).wrapping_sub(1);
            g.write::<u32>(e + 0x24, left);
            let keys = g.read::<u32>(e + 0x14);
            let n = g.read::<u32>(e + 0x28);
            if left == 0 {
                let v = g.read::<u32>(keys.wrapping_add(n.wrapping_mul(4)).wrapping_sub(4));
                g.write::<u32>(roll, v);
                g.write::<u32>(e + 0x2c, 0);
            } else {
                let t = add(f32_at(g, e + 0x1c), f32_at(g, e + 0x18));
                g.write::<f32>(e + 0x18, t);
                // `push 1`: the keys' stride
                g.write::<u32>(esp, 1);
                let callee = img.rel32_target(f + 0x2ac);
                let v = crate::sb_misc::ease_angle(g, img, callee, esp.wrapping_sub(4), keys, n, t);
                g.write::<f32>(roll, v);
            }
            w(0x2bf)
        }
        _ => w(0x2bf),
    }
}

/// The screen depth (`+0x48` its state, 1 = moving; the result
/// `+0x60`): `+0x40` steps left; the time `+0x34` moved on by `+0x38`
/// picks key `i` of the `+0x44` keys at `*(+0x30)` and the fraction `u`
/// after it; the two keys `a`, `b` around it are pulled toward their
/// outer neighbours `p`, `c`: below `KH`, `a` by `(p + b) KH - a` over
/// `KJ / (KH - u)`, else `b` by `(c + a) KH - b` over `KJ / (u - KI)`.
/// The result is `a + (b - a) / (K1 / u)` (`a` when `u` is 0). The last
/// step ends on the last key.
fn depth_step<G: Game>(g: &mut G, img: &Image, f: u32, e: u32, k1: u32) {
    let w = |o: u32| op(img, f, o);
    if g.read::<u32>(e + 0x48) != 1 {
        return;
    }
    let left = g.read::<u32>(e + 0x40).wrapping_sub(1);
    g.write::<u32>(e + 0x40, left);
    let keys = g.read::<u32>(e + 0x30);
    let n = g.read::<u32>(e + 0x44);
    let key = |g: &G, i: u32| f32_at(g, keys.wrapping_add(i.wrapping_mul(4)));
    if left == 0 {
        let v = g.read::<u32>(keys.wrapping_add(n.wrapping_mul(4)).wrapping_sub(4));
        g.write::<u32>(e + 0x60, v);
        g.write::<u32>(e + 0x48, 0);
        return;
    }
    let t = add(f32_at(g, e + 0x38), f32_at(g, e + 0x34));
    let last = n.wrapping_sub(1);
    let i = trunc(t);
    g.write::<f32>(e + 0x34, t);
    let mut a = key(g, i);
    let mut b = if i == last {
        a
    } else {
        key(g, i.wrapping_add(1))
    };
    let u = sub(t, cvt(i));
    let kh = f32_at(g, w(0x3c4));
    if kh > u {
        if i != 0 {
            let p = mul(add(key(g, i.wrapping_sub(1)), b), kh);
            let d = div(f32_at(g, w(0x3ea)), sub(kh, u));
            a = add(div(sub(p, a), d), a);
        }
    } else {
        let j = if i.cast_signed() >= n.wrapping_sub(2).cast_signed() {
            last
        } else {
            i.wrapping_add(2)
        };
        let p = mul(add(key(g, j), a), kh);
        let d = div(f32_at(g, w(0x429)), sub(u, f32_at(g, w(0x41d))));
        b = add(div(sub(p, b), d), b);
    }
    let v = if u == 0.0 {
        add(0.0, a)
    } else {
        add(div(sub(b, a), div(f32_at(g, k1), u)), a)
    };
    g.write::<f32>(e + 0x60, v);
}

/// The three camera rigs' step (cdecl, no arguments; a /GS frame). The
/// rigs `E` are `0x148` bytes apart from `G_R`; each has two point
/// tracks (`E - 0x2c`, output `P0 = E - 0x7c`; `E - 0x6c`, output `P1 =
/// E - 0x8c`) stepped by their states ([`track_step`]), the roll and the
/// screen depth; byte `E + 0x68` = 1 drifts the object at `E - 0xdc`
/// ([`drift`]). With `E + 0x64` = 1 the camera is set: `P1` and `P0`
/// (turned by the object's matrix, slot 229 `FFXVu0ApplyMatrix`, when byte
/// `E + 0x6a` is set) as two locals to slot 284 `MsCameraSetRect(*G_S,
/// 1, P1)` and `(*G_S, 0, P0)`, the roll to slot 285 and the depth to
/// slot 286. Returns the last rig's `eax`: slot 286's, else the drift's
/// or byte `+0x68` - 1.
///
/// [`drift`]: crate::sa_misc::drift
///
/// `magic_0162` 0x10005370 (1520 bytes; 25 DLLs).
pub fn camera_rigs_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    // push ebp; sub esp, 0x24; push ebx, esi, edi
    let ebp = fr.esp.wrapping_sub(4);
    let esp = fr.esp.wrapping_sub(0x34);
    let (p1, p0) = (ebp.wrapping_sub(0x14), ebp.wrapping_sub(0x24));
    let mut eax = 0;
    let mut e = w(0x14);
    for _ in 0..3 {
        let mut k1 = 0;
        for t in &TRACKS {
            k1 = track_step(g, img, fr, e, t, k1, esp.wrapping_sub(4));
        }
        let k1 = roll_step(g, img, f, e, esp.wrapping_sub(4));
        depth_step(g, img, f, e, k1);
        let obj = e.wrapping_sub(0xdc);
        eax = u32::from(g.read::<u8>(e + 0x68)).wrapping_sub(1);
        if eax == 0 {
            eax = crate::sa_misc::drift(g, img, img.rel32_target(f + 0x490), obj, 0);
        }
        if g.read::<u32>(e + 0x64) == 1 {
            let mut st = Pushes::default();
            let (p1_at, p0_at) = (e.wrapping_sub(0x8c), e.wrapping_sub(0x7c));
            if g.read::<u8>(e + 0x6a) == 0 {
                for i in 0..4 {
                    let v = g.read::<u32>(p1_at + 4 * i);
                    g.write::<u32>(p1 + 4 * i, v);
                }
                for i in 0..4 {
                    let v = g.read::<u32>(p0_at + 4 * i);
                    g.write::<u32>(p0 + 4 * i, v);
                }
            } else {
                st.push_all(&[p1_at, obj, p1]);
                // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
                st.call(g, slot_target(g, w(0x512), 229), 0);
                st.push_all(&[p0_at, obj, p0]);
                // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
                st.call(g, slot_target(g, w(0x528), 229), 0);
                st.pop(6);
            }
            let s = g.read::<u32>(w(0x539));
            st.push_all(&[p1, 1, s]);
            // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
            st.call(g, slot_target(g, w(0x542), 284), 0);
            st.push_all(&[p0, 0, s]);
            // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
            st.call(g, slot_target(g, w(0x556), 284), 0);
            st.pop(5);
            st.set_top(g.read::<u32>(e.wrapping_sub(0x94)));
            st.push(s);
            // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
            st.call(g, slot_target(g, w(0x56e), 285), 0);
            st.pop(1);
            st.set_top(g.read::<u32>(e + 0x60));
            st.push(s);
            // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
            eax = st.call(g, slot_target(g, w(0x589), 286), 0).eax;
        }
        e = e.wrapping_add(0x148);
    }
    Ret::eax(eax)
}

/// [`camera_rigs_step`] (the `rel32`s: the key and angle steps, the
/// angle easing, the drift and `__security_check_cookie`).
pub const CAMERA_RIGS_STEP: Sig = Sig {
    len: 1520,
    hash: 0x1610_3043_ac14_4729,
    calls: &[
        (0x114, Callee::Sig(&crate::s3_misc::KEY_STEP)),
        (0x121, Callee::Sig(&crate::s3_misc::KEY_STEP_CURVED)),
        (0x12e, Callee::Sig(&crate::s4_misc::ANGLE_STEP)),
        (0x13b, Callee::Sig(&crate::s4_misc::ANGLE_STEP_CURVED)),
        (0x24e, Callee::Sig(&crate::s3_misc::KEY_STEP)),
        (0x25e, Callee::Sig(&crate::s3_misc::KEY_STEP_CURVED)),
        (0x26e, Callee::Sig(&crate::s4_misc::ANGLE_STEP)),
        (0x27e, Callee::Sig(&crate::s4_misc::ANGLE_STEP_CURVED)),
        (0x2ac, Callee::Sig(&crate::sb_misc::EASE_ANGLE)),
        (0x490, Callee::Sig(&crate::sa_misc::DRIFT)),
        (0x5b7, Callee::Any),
    ],
    imports: &[],
};

/// A call through slot `s` of the import table whose global is the
/// operand at `f + t`.
fn slot<G: Game>(g: &mut G, img: &Image, f: u32, t: u32, s: u32, ecx: u32, args: &[u32]) -> Out {
    let target = slot_target(g, op(img, f, t), s);
    g.call(target, ecx, args)
}

/// The reel game's phases (cdecl `(O)`; the phase `*G_P`):
/// - 0: `*G_C` counts down; at 0 slots 655 and 656, `*G_F = 1`, phase 1;
/// - 1: once slot 652 answers 1, the variant is picked
///   ([`pick_variant`](crate::sb_misc::pick_variant)), phase 2;
/// - 2: once the reels' phases ([`reel_phases`](crate::s7_misc::reel_phases))
///   answer 1, slot 649, `*G_F = 0`, `*G_C = 0`, phase 3;
/// - 3: `*G_C` counts up; at 10 phase 4 and byte `O + 0x56` = 1;
/// - any other: nothing.
///
/// Returns what the original leaves in `eax`: the phase, the count, the
/// last call's answer or `O`.
///
/// `magic_0565` 0x100016c0 (232 bytes with its jump table; 10 DLLs).
pub fn reel_game_phases<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let phase = g.read::<u32>(w(0x4));
    let eax = match phase {
        0 => {
            let c = g.read::<i32>(w(0x19)).wrapping_sub(1);
            g.write::<i32>(w(0x1f), c);
            if c > 0 {
                return Ret::eax(c.cast_unsigned());
            }
            slot(g, img, f, 0x2c, 655, fr.ecx, &[]);
            let eax = slot(g, img, f, 0x39, 656, fr.ecx, &[]).eax;
            g.write::<u32>(w(0x47), 1);
            g.write::<u32>(w(0x51), 1);
            eax
        }
        1 => {
            let eax = slot(g, img, f, 0x5c, 652, fr.ecx, &[]).eax;
            if eax != 1 {
                return Ret::eax(eax);
            }
            let eax = crate::sb_misc::pick_variant(g, img, img.rel32_target(f + 0x6e));
            g.write::<u32>(w(0x74), 2);
            eax
        }
        2 => {
            let sub = Frame {
                at: img.rel32_target(f + 0x7f),
                esp: fr.esp.wrapping_sub(8),
                eax: phase,
                ..*fr
            };
            let eax = crate::s7_misc::reel_phases(g, img, &sub).eax;
            if eax != 1 {
                return Ret::eax(eax);
            }
            let eax = slot(g, img, f, 0x89, 649, fr.ecx, &[]).eax;
            g.write::<u32>(w(0x97), 0);
            g.write::<u32>(w(0xa1), 0);
            g.write::<u32>(w(0xab), 3);
            eax
        }
        3 => {
            let c = g.read::<i32>(w(0xb6)).wrapping_add(1);
            g.write::<i32>(w(0xbc), c);
            if c < 10 {
                return Ret::eax(c.cast_unsigned());
            }
            let o = fr.arg(g, 0);
            g.write::<u32>(w(0xca), 4);
            g.write::<u8>(o.wrapping_add(0x56), 1);
            o
        }
        _ => phase,
    };
    Ret::eax(eax)
}

/// [`reel_game_phases`] (the `rel32`s: the variant pick and the reels'
/// phases).
pub const REEL_GAME_PHASES: Sig = Sig {
    len: 232,
    hash: 0x1f2f_c415_b6f9_9aac,
    calls: &[
        (0x6e, Callee::Sig(&crate::sb_misc::PICK_VARIANT)),
        (0x7f, Callee::Sig(&crate::s7_misc::REEL_PHASES)),
    ],
    imports: &[],
};

/// A node of the DLL's pool for the object `O` (internal, `ecx` = `O`):
/// `O + 0x108` and `+0x104` = 0x30, `+0x10c` = 8, u16 `+0x126` = 0; a
/// node `N` taken ([`take`](crate::sa_pool::take), its callback the
/// immediate `CB`), `N + 0x14 = O`, `N + 4 = 0`, and `N` appended to the
/// DLL's list (`*G_L` the first, `*G_T` the last; `N + 8 = N`). Returns
/// `N`.
///
/// `magic_0304` 0x10007070 (112 bytes; 3 DLLs).
pub fn object_node<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let o = fr.ecx;
    g.write::<u32>(o + 0x108, 0x30);
    g.write::<u32>(o + 0x104, 0x30);
    g.write::<u32>(o + 0x10c, 8);
    g.write::<u16>(o + 0x126, 0);
    let n = crate::sa_pool::take(g, img, img.rel32_target(f + 0x30), w(0x6));
    let first = g.read::<u32>(w(0x36));
    g.write::<u32>(n.wrapping_add(0x14), o);
    g.write::<u32>(n.wrapping_add(4), 0);
    if first == 0 {
        g.write::<u32>(n, 0);
        g.write::<u32>(w(0x63), n);
        g.write::<u32>(w(0x68), n);
    } else {
        let last = g.read::<u32>(w(0x4a));
        g.write::<u32>(n, last);
        g.write::<u32>(last.wrapping_add(4), n);
        g.write::<u32>(w(0x54), n);
    }
    g.write::<u32>(n.wrapping_add(8), n);
    n
}

/// [`object_node`] (the `rel32`: the pool take).
pub const OBJECT_NODE: Sig = Sig {
    len: 112,
    hash: 0xc96d_ed37_a6a5_e81c,
    calls: &[(0x30, Callee::Sig(&crate::sa_pool::TAKE))],
    imports: &[],
};

/// An object's flight step on one of the DLL's tables (cdecl `(O)`): the
/// [`flight_step`](crate::s7_misc::flight_step) of the record
/// `*G_B + K + 0x2c [byte [O + 8] + 0xe]` (`K` the immediate at
/// `+0x23`, a table of the DLL's block) into `O + 0xa0`, with `O + 0xd0`
/// on the stack.
/// Returns its `eax`.
///
/// `magic_0304` 0x100026c0 (`K` 0x6f4) and 0x10002700 (0xe84) (51 bytes
/// each; 2 DLLs).
pub fn table_flight_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let o = fr.arg(g, 0);
    // push ebp; push O + 0xd0
    let esp = fr.esp.wrapping_sub(8);
    g.write::<u32>(esp, o.wrapping_add(0xd0));
    let kind = u32::from(g.read::<u8>(g.read::<u32>(o.wrapping_add(8)).wrapping_add(0xe)));
    let table = g.read::<u32>(w(0x1b)).wrapping_add(w(0x23));
    let callee = Frame {
        at: img.rel32_target(f + 0x2a),
        esp: esp.wrapping_sub(4),
        eax: table,
        ecx: kind.wrapping_mul(0x2c).wrapping_add(table),
        edx: o.wrapping_add(0xa0),
        xmm: fr.xmm,
    };
    Ret::eax(crate::s7_misc::flight_step(g, img, &callee))
}

/// [`table_flight_step`] with `K` 0x6f4.
pub const TABLE_FLIGHT_STEP_A: Sig = Sig {
    len: 51,
    hash: 0x0639_3ebb_5bc8_095e,
    calls: &[(0x2a, Callee::Sig(&crate::s7_misc::FLIGHT_STEP))],
    imports: &[],
};

/// [`table_flight_step`] with `K` 0xe84.
pub const TABLE_FLIGHT_STEP_B: Sig = Sig {
    hash: 0x0d99_bfdf_457a_f9e6,
    ..TABLE_FLIGHT_STEP_A
};

/// The summon's start task (cdecl, no arguments): nothing while slot 225
/// (`op_pray_effect_check(1)`) answers non-zero; once (`*G_F`) slot 226
/// `MsEffectReady(*G_A, 0)`; then, when slot 227
/// `MsEffectMotionLevel(*G_A)` is at least 1, the summon's objects start
/// (`start`, the `rel32` at `+0x5b`) and the DLL's draw and update
/// callbacks `*G_D`, `*G_U` become the immediates. Returns 0.
fn summon_start_task<G: Game>(
    g: &mut G,
    img: &Image,
    fr: &Frame,
    start: fn(&mut G, &Image, &Frame) -> Ret,
) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    if slot(g, img, f, 0x1, 225, fr.ecx, &[1]).eax != 0 {
        return Ret::eax(0);
    }
    if g.read::<u32>(w(0x18)) == 0 {
        let a = g.read::<u32>(w(0x26));
        slot(g, img, f, 0x20, 226, fr.ecx, &[a, 0]);
        g.write::<u32>(w(0x37), 1);
    }
    let a = g.read::<u32>(w(0x46));
    if slot(g, img, f, 0x40, 227, fr.ecx, &[a]).eax.cast_signed() < 1 {
        return Ret::eax(0);
    }
    let sub = Frame {
        at: img.rel32_target(f + 0x5b),
        esp: fr.esp.wrapping_sub(4),
        ..*fr
    };
    start(g, img, &sub);
    g.write::<u32>(w(0x61), w(0x65));
    g.write::<u32>(w(0x6b), w(0x6f));
    Ret::eax(0)
}

/// [`summon_start_task`] with
/// [`summon_objects_start`](crate::s7_misc::summon_objects_start).
///
/// `magic_0248` 0x10001210 (118 bytes; 2 DLLs).
pub fn summon_start_task_0248<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    summon_start_task(g, img, fr, crate::s7_misc::summon_objects_start)
}

/// [`summon_start_task_0248`].
pub const SUMMON_START_TASK_0248: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&crate::s7_misc::SUMMON_OBJECTS_START))],
    imports: &[],
};

/// The OPU host's start-up with a 6 MiB buffer (a table entry, cdecl
/// `(arg)`): the buffer `B` (slot 803 `(0x600000, 0x10)`) cleared
/// (`memset`) and kept in `*G_OBJ`; the effect data `D` = the battle
/// object `*G_E`'s `+0xa8` when the flags block `*G_FL`'s byte `+0x10`
/// is set and the object's byte `+0x40` has bit 4, else the immediate;
/// stored at `*G_S + 4`; slot 718 `(D)`, slot 719 `(D, 1, 0)` for the
/// entry, slot 715 `(1, entry, D, B, 0x200, 0, arg)` for the OPU object,
/// and the tail jump to its set-up ([`opu::setup`](crate::opu::setup)),
/// whose `eax` it returns. Each call sees the words the earlier ones
/// left.
///
/// `magic_0124` 0x100017a0 (175 bytes; 2 DLLs).
pub fn opu_boot_6m<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let arg = fr.arg(g, 0);
    let e = g.read::<u32>(w(0x13));
    let mut st = Pushes::default();
    st.push_all(&[0x10, 0x0060_0000]);
    // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
    let b = st.call(g, slot_target(g, w(0x4), 803), fr.ecx).eax;
    // memset(B, 0, 0x600000)
    g.write_bytes(b, &vec![0; 0x0060_0000]);
    let flags = g.read::<u32>(w(0x31));
    let at = g.read::<u32>(w(0x37)).wrapping_add(4);
    g.write::<u32>(w(0x40), b);
    let data = if g.read::<u8>(flags.wrapping_add(0x10)) != 0
        && g.read::<u8>(e.wrapping_add(0x40)) & 0x10 != 0
    {
        g.read::<u32>(e.wrapping_add(0xa8))
    } else {
        w(0x5f)
    };
    g.write::<u32>(at, data);
    let d = g.read::<u32>(at);
    let mut st = Pushes::default();
    st.push(d);
    // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
    st.call(g, slot_target(g, w(0x64), 718), fr.ecx);
    st.push_all(&[0, 1, d]);
    // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
    let entry = st.call(g, slot_target(g, w(0x74), 719), fr.ecx).eax;
    st.push_all(&[arg, 0, 0x200, b, d, entry, 1]);
    // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
    let obj = st.call(g, slot_target(g, w(0x93), 715), fr.ecx).eax;
    Ret::eax(crate::opu::setup(g, img, img.rel32_target(f + 0xab), obj))
}

/// [`opu_boot_6m`] (the `rel32`s: `memset`'s thunk and the set-up).
pub const OPU_BOOT_6M: Sig = Sig {
    len: 175,
    hash: 0x9162_13d3_c316_a7e1,
    calls: &[
        (0x2b, Callee::Import("memset")),
        (0xab, Callee::Sig(&crate::opu::SETUP)),
    ],
    imports: &[],
};

/// The ring of parts placed (cdecl `(-, -, word)`): when `*G_F` is set,
/// each of the `*G_N` parts (`0xcc` bytes apart from the immediate `P`)
/// gets the point `+0..+8`: the first part the centre `(*G_X, *G_Y,
/// *G_Z)`, the others the centre plus their offset `+(-0x10..-8)` minus
/// `(*G_A', *G_B', *G_C')`, scaled by `*G_U / *G_V K`; `+0xc` = 1.0 and
/// u16 `-0x26` = 0. Returns the address after the last part (the
/// caller's `eax` for none). Else the word becomes 0 and the parts are
/// set on their circle ([`parts_ring`](crate::s6_misc::parts_ring), a
/// tail jump with `ecx` the immediate base and `edx` = `*G_N`).
///
/// `magic_0515` 0x10002c70 (241 bytes; 2 DLLs).
pub fn parts_placed<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let r = |g: &G, o: u32| f32_at(g, w(o));
    if g.read::<u32>(w(0x5)) == 0 {
        g.write::<u32>(fr.esp + 0xc, 0);
        let callee = Frame {
            at: img.rel32_target(f + 0xed),
            ecx: w(0xe7),
            edx: g.read::<u32>(w(0xdb)),
            ..*fr
        };
        return Ret::eax(crate::s6_misc::parts_ring(g, img, &callee));
    }
    let n = g.read::<i32>(w(0x12));
    if n <= 0 {
        return Ret::eax(fr.eax);
    }
    let (z, y, x, k) = (r(g, 0x24), r(g, 0x2c), r(g, 0x34), r(g, 0x3c));
    let mut p = w(0x41);
    for i in 0..n {
        if i == 0 {
            g.write::<f32>(p, x);
            g.write::<f32>(p + 4, y);
            g.write::<f32>(p + 8, z);
        } else {
            let s = mul(div(r(g, 0x5e), r(g, 0x66)), k);
            let a = add(mul(sub(f32_at(g, p.wrapping_sub(0x10)), r(g, 0x73)), s), x);
            g.write::<f32>(p, a);
            let b = add(mul(sub(f32_at(g, p.wrapping_sub(0xc)), r(g, 0x90)), s), y);
            g.write::<f32>(p + 4, b);
            let c = add(mul(sub(f32_at(g, p.wrapping_sub(8)), r(g, 0xaa)), s), z);
            g.write::<f32>(p + 8, c);
        }
        g.write::<u32>(p + 0xc, 0x3f80_0000);
        g.write::<u16>(p.wrapping_sub(0x26), 0);
        p = p.wrapping_add(0xcc);
    }
    Ret::eax(p)
}

/// [`parts_placed`] (the `rel32`: [`parts_ring`](crate::s6_misc::parts_ring)).
pub const PARTS_PLACED: Sig = Sig {
    len: 241,
    hash: 0xc687_0915_79fc_9c85,
    calls: &[(0xed, Callee::Sig(&crate::s6_misc::PARTS_RING))],
    imports: &[],
};

/// A child object linked under its parent and set up (internal, `ecx` =
/// the parameters `P`: the object `O`, the parent `R`, then the words
/// `+8..+0x1c`). `O`'s u16 `+0x1a` = 0xff, its child ring empty (`+0xc`,
/// `+0x10` = `O`), `+0x14 = R`; with a parent, `O` goes last on `R`'s
/// child ring (`R + 0xc` first, `+0x10` last, u16 `+0x18` the count).
/// Then `O`'s fields from `P` (`+0x1c`, u16 `+0x20`, `+0x22`, `+0xdc`,
/// `+0xe0`, the model `M` at `+0xf0`, `+0xfc`), zeros and `+0xec` =
/// 0x10000; `+0xf4` = slot 390 `MsGetChrModelID(M[0x50])`. For model
/// ids 0x1071, 0x1088, 0x1089 and 0x108c slot 362 `(P[0x1c], M[0x54],
/// 0, 0)` and its `eax`; for the others `+0xfc = 0` and 1 (the byte
/// table's), or the id - 0x1071 past the table.
///
/// `magic_0150` 0x10006fc0 (288 bytes with its tables; 2 DLLs).
pub fn child_object_setup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let p = fr.ecx;
    let pw = |g: &G, o: u32| g.read::<u32>(p.wrapping_add(o));
    let o = pw(g, 0);
    let r = pw(g, 4);
    g.write::<u16>(o + 0x1a, 0xff);
    g.write::<u32>(o + 0x10, o);
    g.write::<u32>(o + 0xc, o);
    g.write::<u32>(o + 0x14, r);
    if r != 0 {
        g.write::<u32>(o + 4, r);
        let last = g.read::<u32>(r.wrapping_add(0x10));
        g.write::<u32>(o + 8, last);
        let last = g.read::<u32>(r.wrapping_add(0x10));
        if last == r {
            g.write::<u32>(r.wrapping_add(0xc), o);
        } else {
            g.write::<u32>(last.wrapping_add(4), o);
        }
        let n = g.read::<u16>(r.wrapping_add(0x18)).wrapping_add(1);
        g.write::<u16>(r.wrapping_add(0x18), n);
        g.write::<u32>(r.wrapping_add(0x10), o);
    }
    let v = pw(g, 8);
    g.write::<u32>(o + 0x1c, v);
    let v = g.read::<u16>(p.wrapping_add(0xc));
    g.write::<u16>(o + 0x20, v);
    let v = g.read::<u16>(p.wrapping_add(0xe));
    g.write::<u16>(o + 0x22, v);
    g.write::<u32>(o + 0x26, 0);
    g.write::<u32>(o + 0x24, 0);
    let v = pw(g, 0x10);
    g.write::<u32>(o + 0xdc, v);
    let v = pw(g, 0x14);
    g.write::<u32>(o + 0xe0, v);
    g.write::<u16>(o + 0x28, 0);
    g.write::<u32>(o + 0xe8, 0);
    let m = pw(g, 0x18);
    g.write::<u32>(o + 0xf0, m);
    g.write::<u32>(o + 0xe4, 0);
    g.write::<u32>(o + 0xec, 0x10000);
    let id = g.read::<u32>(m.wrapping_add(0x50));
    let id = slot(g, img, f, 0x98, 390, fr.ecx, &[id]).eax;
    let x = pw(g, 0x1c);
    g.write::<u32>(o + 0xf4, id);
    let k = id.wrapping_sub(0x1071);
    g.write::<u32>(o + 0xfc, x);
    // the byte table after the code: 0 (slot 362) for 0, 23, 24 and 27
    if matches!(k, 0 | 23 | 24 | 27) {
        let m = g.read::<u32>(o + 0xf0);
        let y = g.read::<u32>(m.wrapping_add(0x54));
        return slot(g, img, f, 0xdc, 362, x, &[x, y, 0, 0]).eax;
    }
    g.write::<u32>(o + 0xfc, 0);
    // `eax` is the table's byte (1) once the index was in range
    if k <= 0x1b { 1 } else { k }
}

/// [`child_object_setup`].
pub const CHILD_OBJECT_SETUP: Sig = Sig {
    len: 288,
    hash: 0x29f0_ca24_8098_7283,
    calls: &[],
    imports: &[],
};

/// A weapon's point placed (cdecl `(O)`; a /GS frame): the weapon `H` =
/// slot 219 `MsGetWeaponHandle(*G_A, 0)`, its matrix `L` (slot 236
/// `Ch_ElemGetMatrix(H, 0, L)`, a local) scaled by `K / s` (`s` slot 277
/// `Ch_GetDataScale(H)`'s ST0, as f32) in its first three rows' first
/// three terms, and the point `(0, 7, 0, 1)` turned by it into `O + 0xd0`
/// (slot 229 `FFXVu0ApplyMatrix(O + 0xd0, L, V)`). Each call sees the
/// words the earlier ones left. Returns slot 229's `eax`.
///
/// `magic_0392` 0x10001ff0 (296 bytes; 2 DLLs).
pub fn weapon_point<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let o = fr.arg(g, 0);
    let ebp = fr.esp.wrapping_sub(4);
    let (l, v) = (ebp.wrapping_sub(0x54), ebp.wrapping_sub(0x14));
    let mut st = Pushes::default();
    let a = g.read::<u32>(w(0x24));
    st.push_all(&[0, a]);
    // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
    let h = st.call(g, slot_target(g, w(0x11), 219), fr.ecx).eax;
    st.push_all(&[l, 0, h]);
    // `call ecx`: the slot's address is in `ecx`
    let t = slot_target(g, w(0x2c), 236);
    // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
    st.call(g, t, t);
    st.push(h);
    let t = slot_target(g, w(0x42), 277);
    // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
    let s = crate::ob_model::st0_f32(st.call(g, t, t).st0);
    let k = div(f32_at(g, w(0x56)), s);
    for i in [0u32, 1, 2, 4, 5, 6, 8, 9, 10] {
        let x = mul(f32_at(g, l + 4 * i), k);
        g.write::<f32>(l + 4 * i, x);
    }
    for (i, x) in (0u32..).zip([0u32, 0x40e0_0000, 0, 0x3f80_0000]) {
        g.write::<u32>(v + 4 * i, x);
    }
    st.push_all(&[v, l, o.wrapping_add(0xd0)]);
    let t = slot_target(g, w(0xa2), 229);
    // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
    Ret::eax(st.call(g, t, t).eax)
}

/// [`weapon_point`] (the `rel32`: `__security_check_cookie`).
pub const WEAPON_POINT: Sig = Sig {
    len: 296,
    hash: 0x36ab_184e_2967_abcd,
    calls: &[(0x120, Callee::Any)],
    imports: &[],
};

/// A summon's staged effect script (cdecl `(T)`; the state `S = T[1]`:
/// the stage `S + 0x18`, the parts `S + 0x1c..+0x2c`, two counters `+0x3c`
/// and `+0x40` that every stage moves on):
/// - 0: the first two parts' scales `(30, 1, 5)` and `(30, 1, 20)`,
///   flags `0x18` and `0x10` on the third's and fourth's byte `+0x34`;
/// - 10: bit 0 of the fourth's byte `+0x54`;
/// - 11 and 20: the next stage;
/// - 12: the next stage, `+0x40` back to 0;
/// - 13: once `+0x40` is at least 5, stage 20 (and on as 20);
/// - 30: the next stage, then as 31;
/// - 31: the fifth part's `+0x30` grows by `K`;
/// - 10000: the node `T[0]` unlinked from its ring and made a ring of its
///   own;
/// - any other: only the counters.
///
/// Returns what the original leaves in `eax` (the stage less what the
/// dispatch took off, the fifth part, or the node's old `+4`).
///
/// `magic_0609` 0x10009a70 (315 bytes; 2 DLLs).
pub fn staged_script<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let t = fr.arg(g, 0);
    let s = g.read::<u32>(t.wrapping_add(4));
    let part = |g: &G, i: u32| g.read::<u32>(s.wrapping_add(0x1c + 4 * i));
    let (p0, p1, p2, p3, p4) = (part(g, 0), part(g, 1), part(g, 2), part(g, 3), part(g, 4));
    let inc = |g: &mut G, o: u32| {
        let v = g.read::<u32>(s.wrapping_add(o)).wrapping_add(1);
        g.write::<u32>(s.wrapping_add(o), v);
    };
    let mut stage = g.read::<u32>(s.wrapping_add(0x18));
    if stage == 13 && g.read::<i32>(s.wrapping_add(0x40)) >= 5 {
        g.write::<u32>(s.wrapping_add(0x18), 0x14);
        stage = 0x14;
    }
    let eax = match stage {
        0 => {
            for (q, z) in [(p0, 0x40a0_0000u32), (p1, 0x41a0_0000)] {
                g.write::<u32>(q.wrapping_add(0x2c), 0x41f0_0000);
                g.write::<u32>(q.wrapping_add(0x30), 0x3f80_0000);
                g.write::<u32>(q.wrapping_add(0x34), z);
            }
            let b = g.read::<u8>(p2.wrapping_add(0x34)) | 0x18;
            g.write::<u8>(p2.wrapping_add(0x34), b);
            let b = g.read::<u8>(p3.wrapping_add(0x34)) | 0x10;
            g.write::<u8>(p3.wrapping_add(0x34), b);
            inc(g, 0x18);
            0
        }
        10 => {
            let b = g.read::<u8>(p3.wrapping_add(0x54)) | 1;
            g.write::<u8>(p3.wrapping_add(0x54), b);
            inc(g, 0x18);
            10
        }
        11 => {
            inc(g, 0x18);
            0
        }
        0x14 => {
            inc(g, 0x18);
            0x14
        }
        12 => {
            inc(g, 0x18);
            inc(g, 0x3c);
            g.write::<u32>(s.wrapping_add(0x40), 1);
            return Ret::eax(0);
        }
        30 | 31 => {
            if stage == 30 {
                inc(g, 0x18);
            }
            let x = add(
                f32_at(g, p4.wrapping_add(0x30)),
                f32_at(g, op(img, f, 0xca)),
            );
            g.write::<f32>(p4.wrapping_add(0x30), x);
            p4
        }
        10000 => {
            let n = g.read::<u32>(t);
            let (next, prev) = (g.read::<u32>(n + 4), g.read::<u32>(n + 8));
            g.write::<u32>(next.wrapping_add(8), prev);
            let (prev, next) = (g.read::<u32>(n + 8), g.read::<u32>(n + 4));
            g.write::<u32>(prev.wrapping_add(4), next);
            g.write::<u32>(n + 8, n);
            g.write::<u32>(n + 4, n);
            next
        }
        // what the dispatch's `sub`/`dec` chain left
        1..=9 => stage,
        13..=0x13 => stage.wrapping_sub(13),
        _ => stage.wrapping_sub(10000),
    };
    inc(g, 0x3c);
    inc(g, 0x40);
    Ret::eax(eax)
}

/// [`staged_script`].
pub const STAGED_SCRIPT: Sig = Sig {
    len: 315,
    hash: 0x5a55_342f_6086_8004,
    calls: &[],
    imports: &[],
};

/// The opening of the summon frames below: the block `A` (the immediate
/// at `+0x10`) cleared (`memset(A, 0, 0x30)`) with bit 7 of its u16
/// `+2` set, `*G_Y = *G_X`, and the two scripts run
/// ([`script_run`](crate::sb_misc::script_run), contexts the immediates
/// at `+0x2e` and `+0x3d`). Returns the first script's answer (1: it
/// ended) and the second's (the `eax` after the calls).
fn summon_frame_scripts<G: Game>(g: &mut G, img: &Image, f: u32) -> (u32, u32) {
    let w = |o: u32| op(img, f, o);
    let a = w(0x10);
    g.write_bytes(a, &[0; 0x30]);
    // `or word [A + 2], 0x80` (its own operand)
    let v = g.read::<u16>(w(0x21)) | 0x80;
    g.write::<u16>(w(0x21), v);
    let x = g.read::<u32>(w(0x26));
    g.write::<u32>(w(0x33), x);
    let first = crate::sb_misc::script_run(g, img, img.rel32_target(f + 0x38), w(0x2e));
    let second = crate::sb_misc::script_run(g, img, img.rel32_target(f + 0x44), w(0x3d));
    (first, second)
}

/// The words `from..from + 0x10` of `e` copied to `to..`.
fn copy4<G: Game>(g: &mut G, e: u32, from: u32, to: u32) {
    for i in 0..4 {
        let v = g.read::<u32>(e.wrapping_add(from + 4 * i));
        g.write::<u32>(e.wrapping_add(to + 4 * i), v);
    }
}

/// A summon's frame with the camera rigs (cdecl `(N)`; `E = *G_E`): the
/// opening ([`summon_frame_scripts`]); while the first script runs, the
/// point `E + 0x2b10` becomes `E + 0x2af0`, the motion steps
/// ([`motion_step`](crate::s3_motion::motion_step)) and the frame ends in
/// the camera rigs' step ([`camera_rigs_step`], a tail jump). Once it
/// ended `E + 0x3ba8 = 1`, the held node `*G_P` (when set) is given back
/// and cleared, and the frame ends giving `N` back
/// ([`give_back`](crate::sa_pool::give_back), a tail jump). Returns the
/// last call's `eax`.
///
/// `magic_0563` 0x100013e0 (183 bytes; 2 DLLs).
pub fn summon_frame_rigs<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let e = g.read::<u32>(w(0x7));
    let (done, _) = summon_frame_scripts(g, img, f);
    if done == 0 {
        copy4(g, e, 0x2b10, 0x2af0);
        let eax = crate::s3_motion::motion_step(g, img, img.rel32_target(f + 0x7d));
        let tail = Frame {
            at: img.rel32_target(f + 0x85),
            eax,
            ..*fr
        };
        return camera_rigs_step(g, img, &tail);
    }
    g.write::<u32>(e.wrapping_add(0x3ba8), 1);
    let held = g.read::<u32>(w(0x8b));
    if held != 0 {
        crate::sa_pool::give_back(g, img, img.rel32_target(f + 0x9e), held);
        g.write::<u32>(w(0xa4), 0);
    }
    let n = fr.arg(g, 0);
    Ret::eax(crate::sa_pool::give_back(
        g,
        img,
        img.rel32_target(f + 0xb3),
        n,
    ))
}

/// [`summon_frame_rigs`] (the `rel32`s: `memset`'s thunk, the scripts,
/// the motion step, the rigs' step and the pool's give-back).
pub const SUMMON_FRAME_RIGS: Sig = Sig {
    len: 183,
    hash: 0x1ef4_56ea_2945_7b8a,
    calls: &[
        (0x15, Callee::Import("memset")),
        (0x38, Callee::Sig(&crate::sb_misc::SCRIPT_RUN)),
        (0x44, Callee::Sig(&crate::sb_misc::SCRIPT_RUN)),
        (0x7d, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0x85, Callee::Sig(&CAMERA_RIGS_STEP)),
        (0x9e, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0xb3, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
    ],
    imports: &[],
};

/// A summon's frame with the camera rigs and a colour fade (cdecl `(N)`;
/// `E = *G_E`): the opening ([`summon_frame_scripts`]); while the first
/// script runs, the points `E + 0xa90` and `E + 0xab0` become `E +
/// 0x9b0` and `E + 0x9f0`, the two motion scripts (the immediates at
/// `+0xab` and `+0xbb`, [`motion_script`](crate::s4_misc::motion_script))
/// and the fade `E + 0xe70` ([`int_fade`](crate::s4_small::int_fade))
/// step, and the frame ends in the camera rigs' step
/// ([`camera_rigs_step`], a tail jump). Once it ended `E + 0x3160 = 1`,
/// the effect ends ([`effect_end_show`](crate::s5_misc::effect_end_show))
/// and the frame ends giving `N` back
/// ([`give_back`](crate::sa_pool::give_back), a tail jump). Returns the
/// last call's `eax`.
///
/// `magic_0168` 0x100014e0 (241 bytes; 2 DLLs).
pub fn summon_frame_fade<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let e = g.read::<u32>(w(0x7));
    let (done, eax) = summon_frame_scripts(g, img, f);
    // push ebp, esi, edi; the callees' return address
    let sub = |at: u32, ecx: u32, eax: u32| Frame {
        at: img.rel32_target(f + at),
        esp: fr.esp.wrapping_sub(0x10),
        eax,
        ecx,
        ..*fr
    };
    if done == 0 {
        copy4(g, e, 0xa90, 0x9b0);
        copy4(g, e, 0xab0, 0x9f0);
        crate::s4_misc::motion_script(g, img, img.rel32_target(f + 0xb6), w(0xab));
        crate::s4_misc::motion_script(g, img, img.rel32_target(f + 0xc0), w(0xbb));
        let eax = crate::s4_small::int_fade(g, img, &sub(0xcb, e.wrapping_add(0xe70), eax)).eax;
        let tail = Frame {
            at: img.rel32_target(f + 0xd3),
            eax,
            ..*fr
        };
        return camera_rigs_step(g, img, &tail);
    }
    g.write::<u32>(e.wrapping_add(0x3160), 1);
    crate::s5_misc::effect_end_show(g, img, &sub(0xe2, fr.ecx, eax));
    let n = fr.arg(g, 0);
    Ret::eax(crate::sa_pool::give_back(
        g,
        img,
        img.rel32_target(f + 0xed),
        n,
    ))
}

/// [`summon_frame_fade`] (the `rel32`s: `memset`'s thunk, the scripts,
/// the motion scripts, the fade, the rigs' step, the effect's end and the
/// pool's give-back).
pub const SUMMON_FRAME_FADE: Sig = Sig {
    len: 241,
    hash: 0xb542_4608_036e_8da4,
    calls: &[
        (0x15, Callee::Import("memset")),
        (0x38, Callee::Sig(&crate::sb_misc::SCRIPT_RUN)),
        (0x44, Callee::Sig(&crate::sb_misc::SCRIPT_RUN)),
        (0xb6, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0xc0, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0xcb, Callee::Sig(&crate::s4_small::INT_FADE)),
        (0xd3, Callee::Sig(&CAMERA_RIGS_STEP)),
        (0xe2, Callee::Sig(&crate::s5_misc::EFFECT_END_SHOW)),
        (0xed, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
    ],
    imports: &[],
};

/// The byte at `f + off` of the code (an `imm8` operand).
fn byte_op(img: &Image, f: u32, off: u32) -> u32 {
    img.word(f + off) & 0xff
}

/// An effect object started with its sprite, task node, fan and fades:
/// the body shared by [`object_fan_two_fades`] and
/// [`objects_freed_fan_fade`], at `b` plus the offsets of `magic_0511`
/// 0x10004810's (`e` the effect block, `scale` the object's scale
/// operand, `esp` the stack before its `sub esp, 8`, `fades` 1 or 2):
/// - the object `O = e + D` ([`start_object_anim`](crate::s4_misc::start_object_anim),
///   index and value the two pushed bytes); its `+0xd0..+0xdc` the four
///   words at `e + S`, `+0xd4` less `K`, `+0xc4` the word `e + 0x2258`;
/// - the sprite `P = e + 0x1ad0`: colour `0x808080` with alpha 0x80
///   (the original's overlapping byte, word and dword stores), u16 `+0x86`
///   and byte `+0x85` 0, sizes `+0x88`, `+0x8c`, `+0x90` 10, `+0x94` 0,
///   kind u16 `+0x98` 0x48; a pool node ([`take`](crate::sa_pool::take),
///   the callback immediate) holding it (`+0x14`), appended to the DLL's
///   task list (`*G_H` the first, `*G_T` the last);
/// - the fan `C = e + F` (`+0xf0` = `0x4080_8080`, u16 `+0x124` 4, the
///   steps 0.02 and 0.01, the rest 0) started
///   ([`sprite_fan_start`](crate::s6_misc::sprite_fan_start));
/// - each fade ([`fade_task`](crate::s3_misc::fade_task): `xmm2`/`xmm1`
///   the two constants, the pushed byte, `ecx` = `e` + its offset, `edx`
///   the immediate).
///
/// Returns the last fade's node.
fn object_fan_body<G: Game>(
    g: &mut G,
    img: &Image,
    fr: &Frame,
    b: u32,
    (scale, e, esp, fades): (u32, u32, u32, u32),
) -> u32 {
    let f = fr.at;
    let w = |o: u32| op(img, f, b + o);
    let obj = e.wrapping_add(w(0x15));
    let start = img.rel32_target(f + b + 0x20);
    let scale = f32_at(g, op(img, f, scale));
    let (index, value) = (byte_op(img, f, b + 0x1c), byte_op(img, f, b + 0x1a));
    crate::s4_misc::start_object_anim(g, img, start, obj, scale, index, value);
    for i in 0..4 {
        let v = g.read::<u32>(e.wrapping_add(w(0x26 + 0xc * i)));
        g.write::<u32>(obj.wrapping_add(0xd0 + 4 * i), v);
    }
    let x = sub(f32_at(g, obj.wrapping_add(0xd4)), f32_at(g, w(0x66)));
    let v = g.read::<u32>(e.wrapping_add(w(0x5e)));
    g.write::<u32>(obj.wrapping_add(0xc4), v);
    g.write::<f32>(obj.wrapping_add(0xd4), x);
    let p = e.wrapping_add(w(0x82));
    g.write::<u32>(p.wrapping_add(0x81), 0x0080_8080);
    g.write::<u8>(p.wrapping_add(0x80), 0x80);
    g.write::<u16>(p.wrapping_add(0x86), 0);
    g.write::<u8>(p.wrapping_add(0x85), 0);
    g.write::<u32>(p.wrapping_add(0x88), 10);
    g.write::<u16>(p.wrapping_add(0x98), 0x48);
    g.write::<u32>(p.wrapping_add(0x90), 10);
    g.write::<u32>(p.wrapping_add(0x8c), 10);
    g.write::<u32>(p.wrapping_add(0x94), 0);
    let n = crate::sa_pool::take(g, img, img.rel32_target(f + b + 0xd9), w(0x74));
    let first = g.read::<u32>(w(0xdf));
    g.write::<u32>(n.wrapping_add(0x14), p);
    g.write::<u32>(n.wrapping_add(4), 0);
    if first == 0 {
        g.write::<u32>(n, 0);
        g.write::<u32>(w(0x104), n);
    } else {
        let last = g.read::<u32>(w(0xf2));
        g.write::<u32>(n, last);
        g.write::<u32>(last.wrapping_add(4), n);
    }
    g.write::<u32>(n.wrapping_add(8), n);
    let c = e.wrapping_add(w(0x10d));
    g.write::<u32>(w(0x112), n);
    g.write::<u32>(c.wrapping_add(0xf0), 0x4080_8080);
    g.write::<u16>(c.wrapping_add(0x124), 4);
    g.write::<u32>(c.wrapping_add(0x120), 0);
    g.write::<u32>(c.wrapping_add(0x110), 0);
    g.write::<u32>(c.wrapping_add(0xf8), 0x3ca3_d70a);
    g.write::<u32>(c.wrapping_add(0xfc), 0x3c23_d70a);
    g.write::<u32>(c.wrapping_add(0x100), 0);
    let fan = Frame {
        at: img.rel32_target(f + b + 0x15f),
        esp: esp.wrapping_sub(4),
        eax: 4,
        ecx: c,
        edx: fr.edx,
        xmm: [0; 4],
    };
    let mut eax = crate::s6_misc::sprite_fan_start(g, img, &fan).eax;
    let mut at = esp;
    for i in 0..fades {
        let o = 0x163 + 0x22 * i;
        let (x2, x1) = (g.read::<u32>(w(o + 4)), g.read::<u32>(w(o + 0xc)));
        // push imm8 (the fade's stack word)
        at = at.wrapping_sub(4);
        g.write::<u32>(at, byte_op(img, f, b + o + 0x11));
        let fade = Frame {
            at: img.rel32_target(f + b + o + 0x1e),
            esp: at.wrapping_sub(4),
            eax,
            ecx: e.wrapping_add(w(o + 0x14)),
            edx: w(o + 0x19),
            xmm: [0, u128::from(x1), u128::from(x2), 0],
        };
        eax = crate::s3_misc::fade_task(g, img, &fade);
    }
    eax
}

/// An effect object with its sprite, fan and two fades (cdecl, no
/// arguments; the block `e = *G_E`): [`object_fan_body`] with both fades.
/// Returns the second fade's node.
///
/// `magic_0511` 0x10004810 and 0x10004660 (429 bytes each, their object
/// and copy offsets differ; 2 DLLs).
pub fn object_fan_two_fades<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let e = g.read::<u32>(op(img, fr.at, 0xc));
    // push esi, edi
    let esp = fr.esp.wrapping_sub(8);
    Ret::eax(object_fan_body(g, img, fr, 0, (0x4, e, esp, 2)))
}

/// The `rel32`s of [`object_fan_body`] at `b`.
macro_rules! object_fan_calls {
    ($b:expr, $($more:expr),*) => {
        &[
            $($more,)*
            ($b + 0x20, Callee::Sig(&crate::s4_misc::START_OBJECT_ANIM)),
            ($b + 0xd9, Callee::Sig(&crate::sa_pool::TAKE)),
            ($b + 0x15f, Callee::Sig(&crate::s6_misc::SPRITE_FAN_START)),
            ($b + 0x181, Callee::Sig(&crate::s3_misc::FADE_TASK)),
        ]
    };
}

/// [`object_fan_two_fades`] at 0x10004810.
pub const OBJECT_FAN_TWO_FADES: Sig = Sig {
    len: 429,
    hash: 0x174f_e5a7_a180_aeba,
    calls: object_fan_calls!(0, (0x1a3, Callee::Sig(&crate::s3_misc::FADE_TASK))),
    imports: &[],
};

/// [`object_fan_two_fades`] at 0x10004660.
pub const OBJECT_FAN_TWO_FADES_B: Sig = Sig {
    hash: 0xc476_b1c9_e027_f098,
    ..OBJECT_FAN_TWO_FADES
};

/// Four objects taken off the DLL's list, then an effect object with its
/// sprite, fan and one fade (cdecl, no arguments; `e = *G_E`): for each
/// of `e + 0x5f0`, `+0x980`, `+0x850`, `+0x720` (their own operands) that
/// [`list_remove`](crate::l5_objhost::list_remove) finds in the list (the
/// immediate), `**G_X` = the immediate, slot 713 `(O)` and bit 15 of its
/// u16 `+0x124` cleared; then [`object_fan_body`] with one fade. Returns
/// the fade's node.
///
/// `magic_0511` 0x100043e0 (630 bytes; 2 DLLs).
pub fn objects_freed_fan_fade<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let e = g.read::<u32>(w(0x5));
    // (the object, the list, the rel32, *G_X, its value, the slot table)
    for (d, list, call, x, v, t) in [
        (0x10u32, 0xau32, 0x17u32, 0x25u32, 0x2cu32, 0x31u32),
        (0x49, 0x50, 0x55, 0x5e, 0x65, 0x6a),
        (0x82, 0x89, 0x8e, 0x97, 0x9e, 0xa3),
        (0xbb, 0xc2, 0xc7, 0xd0, 0xd7, 0xdc),
    ] {
        let o = e.wrapping_add(w(d));
        let found = crate::l5_objhost::list_remove(g, img, img.rel32_target(f + call), w(list), o);
        if found != 0 {
            let at = g.read::<u32>(w(x));
            g.write::<u32>(at, w(v));
            slot(g, img, f, t, 713, 0, &[o]);
            let h = g.read::<u16>(o.wrapping_add(0x124)) & 0x7fff;
            g.write::<u16>(o.wrapping_add(0x124), h);
        }
    }
    // push ebx, esi, edi
    let esp = fr.esp.wrapping_sub(0xc);
    Ret::eax(object_fan_body(g, img, fr, 0xea, (0xf6, e, esp, 1)))
}

/// [`objects_freed_fan_fade`].
pub const OBJECTS_FREED_FAN_FADE: Sig = Sig {
    len: 630,
    hash: 0xbe00_52dc_4f0a_2bb4,
    calls: object_fan_calls!(
        0xea,
        (0x17, Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
        (0x55, Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
        (0x8e, Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
        (0xc7, Callee::Sig(&crate::l5_objhost::LIST_REMOVE))
    ),
    imports: &[],
};

/// The sounds of the DLL's 256 emitters (cdecl, no arguments; a frame
/// aligned to 16 with /GS). A matrix `M` on the frame (`ebp - 0x48`) is
/// made a unit matrix (slot 235) once; then for each record `E` (0x4c
/// bytes from the immediate `R`; its point `P = E - 0x34`) with a sound
/// bank `E - 0xc`:
/// - with `E - 8` set: `M`'s diagonal is the scale `E + 0`, else `M` is
///   turned by the angle `i16 E + 0xc` (slot 244) and its three rows
///   scaled (slot 233); either way `M`'s last row is `P`, and without
///   `E - 8` it is also turned by `*G_C` (slot 299 `(M, *G_C, M)`);
/// - slot 230 `(L, M)` (`L` = `ebp - 0x88`); the sound record `S`
///   (`ebp - 0xc0`: u16 `+2` 0x80, `+4` = `E + 0x10`, `+8` 0, `+0x10` =
///   `L`, `+0x14` = `**G_D`, byte `+0x18` 0, `+0x1c` 0); the entry `Q`
///   of the bank: `B + i16 [B + 8 (i16 E + 0x16 + 2)]`;
/// - the 64-bit handle `h` = slot 772 `(P, 0)`; with the lock `*G_L` held
///   at 1 (and put back after each call), slot 773 `(h)`; when it answers
///   non-zero slots 875 `(h, Q, S)` and 775 `(h)`, else slot 873 `(h, Q,
///   S, slot 776 (byte [Q + 4 + Q[4]]))`;
/// - unless `**G_F` is set, the record's callback `E - 4` gets `(P, B +
///   8 (i16 E + 0x16 + 2))`.
///
/// Each call sees the words the earlier ones left. Returns 0 (the
/// counter).
///
/// `magic_0515` 0x10003a20 (802 bytes; 4 DLLs).
pub fn emitter_sounds<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let tab = |g: &G, o: u32, s: u32| slot_target(g, w(o), s);
    let ebp = fr.esp.wrapping_sub(8) & !0xf;
    let at = |o: u32| ebp.wrapping_sub(o);
    let (m, l, s) = (at(0x48), at(0x88), at(0xc0));
    g.write::<u32>(s + 0x10, l);
    g.write::<u16>(s + 2, 0x80);
    g.write::<u32>(s + 8, 0);
    g.write::<u8>(s + 0x18, 0);
    g.write::<u32>(s + 0x1c, 0);
    let mut st = Pushes::default();
    st.push(m);
    // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
    st.call(g, tab(g, 0x45, 235), 0);
    st.pop(1);
    let lock = |g: &G, o: u32| g.read::<u32>(w(o));
    let mut e = w(0x7a);
    for _ in 0..0x100 {
        let bank = g.read::<u32>(e.wrapping_sub(0xc));
        if bank == 0 {
            e = e.wrapping_add(0x4c);
            continue;
        }
        let p = e.wrapping_sub(0x34);
        if g.read::<u32>(e.wrapping_sub(8)) == 0 {
            let angle = i32::from(g.read::<i16>(e + 0xc)).cast_unsigned();
            st.push_all(&[angle, m]);
            // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
            st.call(g, tab(g, 0xcb, 244), 0);
            st.pop(1);
            let scale = g.read::<u32>(e);
            for (row, t) in [(m, 0xe9u32), (at(0x38), 0x113), (at(0x28), 0x135)] {
                st.set_top(scale);
                st.push_all(&[row, row]);
                // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
                st.call(g, tab(g, t, 233), 0);
                st.pop(2);
            }
            st.pop(1);
            for i in 0..4 {
                let v = g.read::<u32>(p + 4 * i);
                g.write::<u32>(at(0x18) + 4 * i, v);
            }
            let c = g.read::<u32>(w(0x14f));
            st.push_all(&[m, c, m]);
            // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
            st.call(g, tab(g, 0x162, 299), 0);
            st.pop(3);
        } else {
            let scale = g.read::<u32>(e);
            for o in [0x20u32, 0x34, 0x48] {
                g.write::<u32>(at(o), scale);
            }
            for i in 0..4 {
                let v = g.read::<u32>(p + 4 * i);
                g.write::<u32>(at(0x18) + 4 * i, v);
            }
        }
        st.push_all(&[m, l]);
        // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
        st.call(g, tab(g, 0x182, 230), 0);
        let v = g.read::<u32>(e + 0x10);
        g.write::<u32>(s + 4, v);
        let i = i32::from(g.read::<i16>(e + 0x16))
            .wrapping_add(2)
            .cast_unsigned();
        let key = bank.wrapping_add(i.wrapping_mul(8));
        let q = i32::from(g.read::<i16>(key))
            .cast_unsigned()
            .wrapping_add(bank);
        st.push(0);
        let d = g.read::<u32>(g.read::<u32>(w(0x1b8)));
        g.write::<u32>(s + 0x14, d);
        st.push(p);
        // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
        let h = st.call(g, tab(g, 0x1c6, 772), 0);
        let saved = g.read::<u32>(lock(g, 0x1d4));
        g.write::<u32>(lock(g, 0x1d4), 1);
        st.push_all(&[h.edx, h.eax]);
        // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
        let found = st.call(g, tab(g, 0x1f4, 773), 0).eax;
        let lk = lock(g, 0x20a);
        g.write::<u32>(lk, saved);
        st.pop(6);
        if found != 0 {
            st.push_all(&[s, q, h.edx]);
            g.write::<u32>(lk, 1);
            st.push(h.eax);
            // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
            st.call(g, tab(g, 0x22b, 875), 0);
            st.push(h.edx);
            g.write::<u32>(lock(g, 0x24a), 1);
            st.push(h.eax);
            // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
            st.call(g, tab(g, 0x25b, 775), 0);
            g.write::<u32>(lock(g, 0x26e), saved);
            st.pop(6);
        } else {
            let k = g.read::<u32>(q.wrapping_add(4));
            let b = u32::from(g.read::<u8>(k.wrapping_add(q).wrapping_add(4)));
            st.push(b);
            // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
            let r = st.call(g, tab(g, 0x28f, 776), 0).eax;
            let lk = lock(g, 0x29d);
            st.pop(1);
            let held = g.read::<u32>(lk);
            st.push_all(&[r, s, q, h.edx]);
            g.write::<u32>(lk, 1);
            st.push(h.eax);
            // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
            st.call(g, tab(g, 0x2b5, 873), 0);
            g.write::<u32>(lock(g, 0x2d4), held);
            st.pop(5);
        }
        if g.read::<u32>(g.read::<u32>(w(0x2e1))) == 0 {
            st.push_all(&[key, p]);
            let cb = g.read::<u32>(e.wrapping_sub(4));
            // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
            st.call(g, cb, 0);
            st.pop(2);
        }
        e = e.wrapping_add(0x4c);
    }
    Ret::eax(0)
}

/// [`emitter_sounds`] (the `rel32`: `__security_check_cookie`).
pub const EMITTER_SOUNDS: Sig = Sig {
    len: 802,
    hash: 0x4dc7_2770_fbd5_24d5,
    calls: &[(0x317, Callee::Any)],
    imports: &[],
};

/// `(x << 7) / 128` (signed, toward zero: the original's `cdq; and edx,
/// 0x7f; add; sar 7`) as `cvtdq2ps` gives it, times the `f32` at `k`.
fn scaled_low<G: Game>(g: &G, x: u32, k: u32) -> f32 {
    let v = (x << 7).cast_signed() / 128;
    mul(cvt(v.cast_unsigned()), f32_at(g, k))
}

/// A model effect placed on the object `O` (`ecx`; cdecl `(-, a1, a2, a3,
/// a4, a5)`, a /GS frame; `M = O[0x830]`). The effect's entry (slot 719
/// `(E, 2, 14)`, `E` the immediate) gives a 64-bit key `v`; with the lock
/// `*G_L` held at 1, slot 885 looks it up `(v & 0x3fff, (v >> 20) & 0x3f,
/// 0, 0, 1 << ((v >> 26) & 0xf), 1 << ((v >> 30) & 0xf), T)` into the
/// frame's `T`. When found: the colour `C = (1, 1, 1, a5 K)` (`a5` and
/// `a4` cut to their low 25 bits, signed); the camera matrix (slot 545)
/// copied (slot 230) with its second column's first three terms negated
/// (`xor` with `S`) and multiplied by `a1` into `W` (slot 231 `(W, V,
/// a1)`), `W`'s last row cleared; slots 922 `(M, 0)`, 931 `(M, 0)` when
/// `*G_Q` is set, 912 `(M, T, a3, 0, 0, 0, 0, C, slot 776 (0x48), W, a2,
/// 1)`, 834 `(M)` and 833 `(M, u16 O[0], a4 K)` (`a4 K` held to `LO..HI`).
/// Each call sees the words the earlier ones left. Returns slot 833's
/// `eax`, or 0 when the lookup fails.
///
/// `magic_0162` 0x10001d20 (784 bytes; 2 DLLs).
pub fn model_effect_placed<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let tab = |g: &G, o: u32, s: u32| slot_target(g, w(o), s);
    let ebp = fr.esp.wrapping_sub(4);
    let at = |o: u32| ebp.wrapping_sub(o);
    let o = fr.ecx;
    let arg = |g: &G, i: u32| fr.arg(g, i);
    let (a1, a2, a3) = (arg(g, 1), arg(g, 2), arg(g, 3));
    let m = g.read::<u32>(o.wrapping_add(0x830));
    let mut st = Pushes::default();
    st.push_all(&[0xe, 2, w(0x4e)]);
    // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
    let r = st.call(g, tab(g, 0x36, 719), 0).eax;
    let lo = g.read::<u32>(r);
    let hi = g.read::<u32>(r.wrapping_add(4));
    let v = u64::from(lo) | (u64::from(hi) << 32);
    let lock = g.read::<u32>(w(0x60));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let t = at(0x10c);
    let bit = |s: u32| u32::from(1u16 << ((v >> s) & 0xf));
    let key = ((v >> 20) as u32) & 0x3f;
    st.push_all(&[t, bit(30), bit(26), 0, 0, key, lo & 0x3fff]);
    // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
    let found = st.call(g, tab(g, 0xbd, 885), 0).eax;
    st.pop(10);
    let lock = g.read::<u32>(w(0xd5));
    g.write::<u32>(lock, saved);
    if found == 0 {
        return Ret::eax(0);
    }
    let c = at(0x11c);
    let x5 = scaled_low(g, arg(g, 5), w(0x10c));
    g.write::<f32>(c + 0xc, x5);
    let x4 = scaled_low(g, arg(g, 4), w(0x13b));
    for i in 0..3 {
        g.write::<u32>(c + 4 * i, 0x3f80_0000);
    }
    // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
    let cam = st.call(g, tab(g, 0x126, 545), 0).eax;
    let (vm, wm) = (at(0x15c), at(0x19c));
    st.push_all(&[cam, vm]);
    // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
    st.call(g, tab(g, 0x16c, 230), 0);
    let sign = g.read::<u32>(w(0x184));
    for o in [4u32, 0x14, 0x24] {
        let x = g.read::<u32>(vm + o) ^ sign;
        g.write::<u32>(vm + o, x);
    }
    st.push_all(&[a1, vm, wm]);
    // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
    st.call(g, tab(g, 0x1c3, 231), 0);
    for i in 0..4 {
        g.write::<u32>(wm + 0x30 + 4 * i, 0);
    }
    st.push_all(&[0, m]);
    // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
    st.call(g, tab(g, 0x1db, 922), 0);
    st.pop(7);
    if g.read::<u32>(w(0x21d)) != 0 {
        st.push_all(&[0, m]);
        // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
        st.call(g, tab(g, 0x225, 931), 0);
        st.pop(2);
    }
    st.push(0x48);
    // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
    let r776 = st.call(g, tab(g, 0x238, 776), 0).eax;
    st.pop(1);
    st.push_all(&[1, a2, wm, r776, c, 0, 0, 0, 0, a3, t, m]);
    // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
    st.call(g, tab(g, 0x290, 912), 0);
    st.pop(12);
    let low = f32_at(g, w(0x2a1));
    let x = if low > x4 {
        low
    } else {
        let high = f32_at(g, w(0x2b9));
        if x4 > high { high } else { x4 }
    };
    st.push(m);
    // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
    st.call(g, tab(g, 0x2cb, 834), 0);
    st.set_top(x.to_bits());
    st.push_all(&[u32::from(g.read::<u16>(o)), m]);
    // call width: 12 (the most `Pushes::words` holds; slots take at most 11)
    Ret::eax(st.call(g, tab(g, 0x2f0, 833), 0).eax)
}

/// [`model_effect_placed`] (the `rel32`: `__security_check_cookie`).
pub const MODEL_EFFECT_PLACED: Sig = Sig {
    len: 784,
    hash: 0x264e_b9bc_855c_680f,
    calls: &[(0x308, Callee::Any)],
    imports: &[],
};

/// Calls a cdecl colour helper at the `rel32` `call` with `words` pushed
/// below `esp` (the first word on top), `ecx` and `edx` as given.
fn colour_call<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    (call, esp, ecx, edx): (u32, u32, u32, u32),
    words: &[u32],
    port: fn(&mut G, &Image, &Frame) -> Ret,
) -> u32 {
    let top = esp.wrapping_sub(4 * words.len() as u32);
    for (i, &x) in (0u32..).zip(words) {
        g.write::<u32>(top + 4 * i, x);
    }
    let fr = Frame {
        at: img.rel32_target(f + call),
        esp: top.wrapping_sub(4),
        eax: 0,
        ecx,
        edx,
        xmm: [0; 4],
    };
    port(g, img, &fr).eax
}

/// The opening of [`summon_frame_colours`] and [`summon_frame_colours_b`]
/// (their first 0x149 bytes are the same): `None` when `*G_A` or `*G_B`
/// is clear, else the slots, the sound record and the 17 parts; returns
/// the aligned stack.
fn colours_head<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Option<u32> {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let esp = fr.esp.wrapping_sub(4) & !7;
    if g.read::<u32>(w(0x8)) == 0 || g.read::<u32>(w(0x15)) == 0 {
        return None;
    }
    let c = g.read::<u32>(w(0x27));
    slot(g, img, f, 0x21, 723, fr.ecx, &[c]);
    let d = g.read::<u32>(w(0x43));
    slot(g, img, f, 0x34, 724, fr.ecx, &[d]);
    let e = g.read::<u32>(w(0x4a));
    g.write::<u32>(e, w(0x53));
    let sub_at = |at: u32, eax: u32| Frame {
        at: img.rel32_target(f + at),
        esp: esp.wrapping_sub(4),
        eax,
        ..*fr
    };
    if g.read::<u16>(w(0x5a)) != 0 {
        slot(g, img, f, 0x62, 315, fr.ecx, &[0]);
        crate::s7_misc::sound_record(g, img, &sub_at(0x74, 0));
    }
    let at = g.read::<u32>(w(0x79));
    g.write::<u32>(at, w(0x84));
    let mut eax = at;
    for i in 0..17u32 {
        // `mov ecx, imm32; call`: the first part's `mov ecx` comes before
        // the store
        let (ecx_at, call) = if i == 0 {
            (0x7e, 0x89)
        } else {
            (0x84 + 0xa * i, 0x89 + 0xa * i)
        };
        let refresh = img.rel32_target(f + call);
        eax = crate::sa_misc::part_refresh_b(g, img, refresh, w(ecx_at), eax);
    }
    if g.read::<u16>(w(0x130)) != 0 {
        slot(g, img, f, 0x138, 275, fr.ecx, &[1]);
    }
    Some(esp)
}

/// The end of [`summon_frame_colours`] and [`summon_frame_colours_b`]
/// at `b` (their emitters' call): the emitters' sounds
/// ([`emitter_sounds`]); unless `**G_V` is set the stage becomes `*G_W`
/// and the count `*G_N` goes up; `*G_X = **G_Y`.
fn colours_tail<G: Game>(g: &mut G, img: &Image, fr: &Frame, esp: u32, b: u32) {
    let f = fr.at;
    let w = |o: u32| op(img, f, b + o);
    let em = Frame {
        at: img.rel32_target(f + b + 1),
        esp: esp.wrapping_sub(4),
        eax: 0,
        ..*fr
    };
    emitter_sounds(g, img, &em);
    if g.read::<u32>(g.read::<u32>(w(0x6))) == 0 {
        let st = g.read::<u32>(w(0x10));
        let n = g.read::<u32>(w(0x16)).wrapping_add(1);
        g.write::<u32>(w(0x16), n);
        g.write::<u32>(w(0x1b), st);
    }
    let x = g.read::<u32>(g.read::<u32>(w(0x20)));
    g.write::<u32>(w(0x27), x);
}

/// A summon's frame with its parts, colours and emitters (cdecl, no
/// arguments; a frame aligned to 8). Nothing (0) unless `*G_A` and `*G_B`
/// are set. Slots 723 `(*G_C)` and 724 `(*G_D)`, `**G_E` the immediate;
/// with the u16 `*G_F` set slot 315 `(0)` and the model's sound record
/// ([`sound_record`](crate::s7_misc::sound_record)); `**G_G` the
/// immediate and the 17 parts (the immediates) refreshed
/// ([`part_refresh_b`](crate::sa_misc::part_refresh_b)); slot 275 `(1)`
/// with `*G_F` set. Then by the stage `*G_ST` a colour:
/// - 1: the level `(*G_Q >> 12) - (*G_P >> 12) + 0x41` of 20, black
///   ([`level_colour`](crate::s4_small::level_colour));
/// - 3: `40 - *G_R` of 40, black;
/// - 16: the pulses ([`pulse_colour`](crate::s4_small::pulse_colour)) at
///   `*G_S - 20` and `*G_S - 274`, `(0x40, 0x40, 0x40)` with 0x48;
/// - 21: the white pulse at `*G_T - 55` (0x44), then from `*G_T` past
///   0x91 the level `*G_T - 0x91` of 60 in white (up to 60);
/// - 23: `30 - *G_U` of 30, white.
///
/// Then the emitters' sounds ([`emitter_sounds`]); unless `**G_V` is set
/// the stage becomes `*G_W` and the count `*G_N` goes up; `*G_X =
/// **G_Y`. Returns 0.
///
/// `magic_0515` 0x10008cb0 (771 bytes with its tables; 2 DLLs).
pub fn summon_frame_colours<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let Some(esp) = colours_head(g, img, fr) else {
        return Ret::eax(0);
    };
    let level = crate::s4_small::level_colour;
    let pulse = crate::s4_small::pulse_colour;
    let sx16 = |g: &G, o: u32| i32::from(g.read::<i16>(w(o)));
    match g.read::<u32>(w(0x14a)) {
        1 => {
            let a = (g.read::<i32>(w(0x167)) >> 12) - 0x41;
            let c = g.read::<i32>(w(0x16d)) >> 12;
            if c >= a && c - a > 0 {
                let n = (c - a).cast_unsigned();
                colour_call(g, img, f, (0x29b, esp, n, 0x14), &[0, 0, 0], level);
            }
        }
        3 => {
            let r = sx16(g, 0x19e);
            if r < 0x28 {
                let n = (0x28 - r).cast_unsigned();
                colour_call(g, img, f, (0x29b, esp, n, 0x28), &[0, 0, 0], level);
            }
        }
        16 => {
            let s = sx16(g, 0x1ce) - 0x14;
            if s > 0 {
                let n = s.cast_unsigned();
                colour_call(
                    g,
                    img,
                    f,
                    (0x1e7, esp, n, 0x40),
                    &[0x40, 0x40, 0x48, 0],
                    pulse,
                );
            }
            let s = sx16(g, 0x1ed) - 0x112;
            if s > 0 {
                let n = s.cast_unsigned();
                colour_call(
                    g,
                    img,
                    f,
                    (0x210, esp, n, 0x40),
                    &[0x40, 0x40, 0x48, 0],
                    pulse,
                );
            }
        }
        21 => {
            let s = sx16(g, 0x21e) - 0x37;
            if s > 0 {
                let n = s.cast_unsigned();
                colour_call(
                    g,
                    img,
                    f,
                    (0x23e, esp, n, 0xff),
                    &[0xff, 0xff, 0x44, 0],
                    pulse,
                );
            }
            let t = sx16(g, 0x244);
            let s = t - 0x91;
            if t > 0x91 && s <= 0x3c && s > 0 {
                let n = s.cast_unsigned();
                colour_call(g, img, f, (0x29b, esp, n, 0x3c), &[0xff, 0xff, 0xff], level);
            }
        }
        23 => {
            let u = sx16(g, 0x270);
            if u < 0x1e {
                let n = (0x1e - u).cast_unsigned();
                colour_call(g, img, f, (0x29b, esp, n, 0x1e), &[0xff, 0xff, 0xff], level);
            }
        }
        _ => {}
    }
    colours_tail(g, img, fr, esp, 0x2a2);
    Ret::eax(0)
}

/// [`summon_frame_colours`] (the `rel32`s: the sound record, the 17
/// part refreshes, the colours and the emitters' sounds).
pub const SUMMON_FRAME_COLOURS: Sig = Sig {
    len: 771,
    hash: 0x49e4_0e10_5ef8_d3c0,
    calls: &[
        (0x74, Callee::Sig(&crate::s7_misc::SOUND_RECORD)),
        (0x89, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0x93, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0x9d, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0xa7, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0xb1, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0xbb, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0xc5, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0xcf, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0xd9, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0xe3, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0xed, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0xf7, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0x101, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0x10b, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0x115, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0x11f, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0x129, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0x1e7, Callee::Sig(&crate::s4_small::PULSE_COLOUR)),
        (0x210, Callee::Sig(&crate::s4_small::PULSE_COLOUR)),
        (0x23e, Callee::Sig(&crate::s4_small::PULSE_COLOUR)),
        (0x29b, Callee::Sig(&crate::s4_small::LEVEL_COLOUR)),
        (0x2a3, Callee::Sig(&EMITTER_SOUNDS)),
    ],
    imports: &[],
};

/// [`summon_frame_colours`]'s copy with other stages (cdecl, no
/// arguments): by the stage `*G_ST`
/// - 0: `*G_A'` (up to 20) of 20, black
///   ([`level_colour`](crate::s4_small::level_colour));
/// - 1: the level `(*G_Q >> 12) - (*G_P >> 12) + 0x41` of 20, black;
/// - 3: `40 - *G_R` of 40, black;
/// - 16: the pulses ([`pulse_colour`](crate::s4_small::pulse_colour)) at
///   `*G_S - 20` and `*G_S - 274`;
/// - 21: `40 - *G_T` of 40 black, the white pulse at `*G_T - 85`, then
///   from `*G_T` past 0xaf the level `*G_T - 0xaf` of 60 in white (up to
///   60);
/// - 23: `30 - *G_U` of 30, white.
///
/// The rest is [`summon_frame_colours`]'s. Returns 0.
///
/// `magic_0611` 0x10008bf0 (840 bytes with its tables; 2 DLLs).
pub fn summon_frame_colours_b<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| op(img, f, o);
    let Some(esp) = colours_head(g, img, fr) else {
        return Ret::eax(0);
    };
    let level = crate::s4_small::level_colour;
    let pulse = crate::s4_small::pulse_colour;
    let sx16 = |g: &G, o: u32| i32::from(g.read::<i16>(w(o)));
    let black = |g: &mut G, n: i32, full: u32| {
        if n > 0 {
            let n = n.cast_unsigned();
            colour_call(g, img, f, (0x2d9, esp, n, full), &[0, 0, 0], level);
        }
    };
    match g.read::<u32>(w(0x14a)) {
        0 => {
            let a = sx16(g, 0x167);
            if a <= 0x14 {
                black(g, a, 0x14);
            }
        }
        1 => {
            let a = (g.read::<i32>(w(0x191)) >> 12) - 0x41;
            let c = g.read::<i32>(w(0x197)) >> 12;
            if c >= a {
                black(g, c - a, 0x14);
            }
        }
        3 => {
            let r = sx16(g, 0x1b2);
            if r < 0x28 {
                black(g, 0x28 - r, 0x28);
            }
        }
        16 => {
            let s = sx16(g, 0x1e2) - 0x14;
            if s > 0 {
                let n = s.cast_unsigned();
                colour_call(
                    g,
                    img,
                    f,
                    (0x1fb, esp, n, 0x40),
                    &[0x40, 0x40, 0x48, 0],
                    pulse,
                );
            }
            let s = sx16(g, 0x201) - 0x112;
            if s > 0 {
                let n = s.cast_unsigned();
                colour_call(
                    g,
                    img,
                    f,
                    (0x224, esp, n, 0x40),
                    &[0x40, 0x40, 0x48, 0],
                    pulse,
                );
            }
        }
        21 => {
            let t = sx16(g, 0x233);
            if t < 0x28 && 0x28 - t > 0 {
                let n = (0x28 - t).cast_unsigned();
                colour_call(g, img, f, (0x257, esp, n, 0x28), &[0, 0, 0], level);
            }
            let s = sx16(g, 0x25e) - 0x55;
            if s > 0 {
                let n = s.cast_unsigned();
                colour_call(
                    g,
                    img,
                    f,
                    (0x281, esp, n, 0xff),
                    &[0xff, 0xff, 0x44, 0],
                    pulse,
                );
            }
            let t = sx16(g, 0x288);
            let s = t - 0xaf;
            if t > 0xaf && s <= 0x3c && s > 0 {
                let n = s.cast_unsigned();
                colour_call(g, img, f, (0x2d9, esp, n, 0x3c), &[0xff, 0xff, 0xff], level);
            }
        }
        23 => {
            let u = sx16(g, 0x2ae);
            if u < 0x1e && 0x1e - u > 0 {
                let n = (0x1e - u).cast_unsigned();
                colour_call(g, img, f, (0x2d9, esp, n, 0x1e), &[0xff, 0xff, 0xff], level);
            }
        }
        _ => {}
    }
    colours_tail(g, img, fr, esp, 0x2e0);
    Ret::eax(0)
}

/// [`summon_frame_colours_b`].
pub const SUMMON_FRAME_COLOURS_B: Sig = Sig {
    len: 840,
    hash: 0xa3a1_524b_cd37_ce19,
    calls: &[
        (0x74, Callee::Sig(&crate::s7_misc::SOUND_RECORD)),
        (0x89, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0x93, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0x9d, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0xa7, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0xb1, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0xbb, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0xc5, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0xcf, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0xd9, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0xe3, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0xed, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0xf7, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0x101, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0x10b, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0x115, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0x11f, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0x129, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0x1fb, Callee::Sig(&crate::s4_small::PULSE_COLOUR)),
        (0x224, Callee::Sig(&crate::s4_small::PULSE_COLOUR)),
        (0x257, Callee::Sig(&crate::s4_small::LEVEL_COLOUR)),
        (0x281, Callee::Sig(&crate::s4_small::PULSE_COLOUR)),
        (0x2d9, Callee::Sig(&crate::s4_small::LEVEL_COLOUR)),
        (0x2e1, Callee::Sig(&EMITTER_SOUNDS)),
    ],
    imports: &[],
};
