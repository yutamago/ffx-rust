//! Node handlers of the summon family (`magic_0003`'s code, the same
//! bytes in about 107 overlays, `magic_0087`/`0088` with their own
//! callees' places), ported by the shared-4 list. Like
//! [`crate::sa_summon`]'s, each is internal (`ecx` = the cursor), reached
//! through a 12-byte thunk ([`crate::s4_wrap`]), and starts with the
//! walk ([`crate::nodes::walk`]); its own work runs only when the cursor
//! moved. Each doc names the DLL and address it was ported from, its size
//! and how many DLLs carry it.

use crate::l3_common::{ListAt, attach};
use crate::nodes::walk;
use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::sa_summon::st0_f32;
use crate::sb_sse::{add, mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig, s3_nodes, sb_matrix, sb_nodes};
use ffx_bridge::x87::cvttsd2si;

/// `1.0f32`'s bits.
const ONE: u32 = 0x3f80_0000;

fn f32_at<G: Game>(g: &G, a: u32) -> f32 {
    f32::from_bits(g.read::<u32>(a))
}

fn set_f32<G: Game>(g: &mut G, a: u32, x: f32) {
    g.write::<u32>(a, x.to_bits());
}

/// `cvttss2si`: truncated, `0x80000000` when out of range or NaN.
fn trunc(x: f32) -> u32 {
    cvttsd2si(f64::from(x))
}

/// The walk of one of the node's objects with a cursor in the frame
/// (`local`: the object, the pass byte at `+5`, the "moved" byte at
/// `+4`), as the original inlines it: the object's ring is walked with
/// `ecx` = the local cursor.
fn walk_object<G: Game>(g: &mut G, local: u32, obj: u32, pass: u8) {
    g.write::<u32>(local, obj);
    g.write::<u8>(local + 5, pass);
    walk(g, local, local, 0);
}

/// A bone placed by a character's element (`ecx` = the cursor; a /GS
/// frame). The walk; when the cursor moved, the rings of the node's three
/// objects (`+0x13c`, `+0x144`, `+0x140`) are walked with cursors in the
/// frame. Slot 236 (`Ch_ElemGetMatrix`) gives the matrix `M` (`[ebp -
/// 0x94]`) of the character `+0x14c`'s element `+0x150`;
/// [`sb_matrix::rotation_19d0`] turns the node's `+0x5c` into the target
/// `+0x144`'s vector; slot 233 interpolates the local vector `[ebp -
/// 0x14]` towards the `+0x140` object by its rate `+0x28` times `K`
/// (operand `+0x201`), then the node's three vectors by it; slot 325
/// moves the `+0x13c` object into `+0x4c`. The node takes the target's
/// position into `+0x11c..+0x124` and the source's into `+0x12c`; its
/// scale `+0x138` is multiplied by the character's data scale (slot 277,
/// ST0). Slot 232 normalises the first three rows of `M` into a local
/// matrix (`[ebp - 0x54]`, last row `(0, 0, 0, 1)`); slots 231 and 229
/// compose the node's matrices `+0xdc`, `+0x9c` and `+0xcc` from them.
/// Returns slot 229's `eax`, or the walk's.
///
/// `magic_0003` 0x10001f60 (1024 bytes; 107 DLLs, `magic_0087`/`0088`
/// 0x100021a0).
pub fn elem_bone<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let this = fr.ecx;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let n = g.read::<u32>(this);
    let (eax, moved) = walk(g, this, this, fr.eax);
    if moved.is_none() {
        return Ret::eax(eax);
    }
    let ebp = fr.esp.wrapping_sub(4);
    // the frame's words the third cursor overlaps: `[ebp - 0x98]` the
    // node, `[ebp - 0x9c]` the object of the walk before (its `+4..+7`)
    g.write::<u32>(ebp.wrapping_sub(0x98), n);
    for (local, off) in [(0xb0u32, 0x13c), (0xa8, 0x144), (0xa0, 0x140)] {
        let obj = g.read::<u32>(n + off);
        let pass = g.read::<u8>(this + 5);
        if off != 0x140 {
            g.write::<u32>(ebp.wrapping_sub(0x9c), obj);
        }
        walk_object(g, ebp.wrapping_sub(local), obj, pass);
    }
    let m = ebp.wrapping_sub(0x94);
    let unit = ebp.wrapping_sub(0x54);
    let v = ebp.wrapping_sub(0x14);
    let (chara, elem) = (g.read::<u32>(n + 0x14c), g.read::<u32>(n + 0x150));
    // the words each call sees, the earlier calls' left below its own
    let mut s = Pushes::default();
    s.push_all(&[m, elem, chara]);
    s.call(g, slot_target(g, w(0x1c4), 236), 0);
    let target = g.read::<u32>(n + 0x144).wrapping_add(0x1c);
    sb_matrix::rotation_19d0(g, img, img.rel32_target(f + 0x1eb), n + 0x5c, target);
    s.pop(2);
    let src = g.read::<u32>(n + 0x140);
    let rate = mul(f32_at(g, src + 0x28), f32_at(g, w(0x201)));
    s.set_top(rate.to_bits());
    s.push_all(&[src + 0x1c, v]);
    s.call(g, slot_target(g, w(0x213), 233), 0);
    for (i, op, (a, b)) in [
        (0u32, 0x225, (0x1c, 0x5c)),
        (1, 0x254, (0x2c, 0x6c)),
        (2, 0x276, (0x3c, 0x7c)),
    ] {
        s.pop(2);
        s.set_top(g.read::<u32>(v + 4 * i));
        s.push_all(&[n + b, n + a]);
        s.call(g, slot_target(g, w(op), 233), 0);
    }
    let obj = g.read::<u32>(n + 0x13c).wrapping_add(0x1c);
    s.push_all(&[obj, n + 0x4c]);
    s.call(g, slot_target(g, w(0x296), 325), 0);
    for i in 0..3 {
        let tgt = g.read::<u32>(n + 0x144);
        let x = g.read::<u32>(tgt + 0x1c + 4 * i);
        g.write::<u32>(n + 0x11c + 4 * i, x);
    }
    let src = g.read::<u32>(n + 0x140).wrapping_add(0x1c);
    s.push_all(&[src, n + 0x12c]);
    s.call(g, slot_target(g, w(0x2e8), 325), 0);
    s.push(g.read::<u32>(n + 0x14c));
    let scale = st0_f32(s.call(g, slot_target(g, w(0x301), 277), 0));
    let x = mul(f32_at(g, n + 0x138), f32::from_bits(scale));
    set_f32(g, n + 0x138, x);
    for (row, op) in [(0u32, 0x33d), (1, 0x355), (2, 0x36a)] {
        s.push_all(&[m + 0x10 * row, unit + 0x10 * row]);
        s.call(g, slot_target(g, w(op), 232), 0);
    }
    for (i, x) in [0, 0, 0, ONE].into_iter().enumerate() {
        g.write::<u32>(unit + 0x30 + 4 * i as u32, x);
    }
    s.push_all(&[n + 0x5c, unit, n + 0xdc]);
    s.call(g, slot_target(g, w(0x389), 231), 0);
    // add esp, 0x44: every word so far
    s.pop(17);
    s.push_all(&[n + 0x1c, m, n + 0x9c]);
    s.call(g, slot_target(g, w(0x3c4), 231), 0);
    s.push_all(&[n + 0x4c, m, n + 0xcc]);
    Ret::eax(s.call(g, slot_target(g, w(0x3e0), 229), 0).eax)
}

/// [`elem_bone`] (the `rel32`s: [`sb_matrix::rotation_19d0`] and
/// `__security_check_cookie`).
pub const ELEM_BONE: Sig = Sig {
    len: 1024,
    hash: 0x8d52_5ba5_c9a3_306e,
    calls: &[
        (0x1eb, Callee::Sig(&sb_matrix::ROTATION_19D0)),
        (0x3f8, Callee::Any),
    ],
    imports: &[],
};

/// `comiss a, b; jb` not taken: `a >= b`, both ordered.
fn at_least(a: f32, b: f32) -> bool {
    a >= b
}

/// A colour node fading in (`ecx` = the cursor; a /GS frame). The walk;
/// when the cursor moved, by bit 0 of the node's flags `+0xac`:
/// - set: the two colours `+0x1c` and `+0x4c` move towards `+0x2c` and
///   `+0x5c` by the rate `+0x9c` (`(to - x) * rate + x`, interleaved as
///   the original goes), and so does the progress `+0xa0` towards `K`
///   (operand `+0x157`);
/// - clear: the progress grows by the rate; at `K` or past it, it becomes
///   1.0 and the colours take their targets, else they move by their
///   speeds `+0x3c` and `+0x6c` times the rate.
///
/// Then slot 353 (`yiSetColorScale`, `+0x1c`); slot 332
/// (`yiSetBackColor`) with the four integers `+0x8c` scaled by the first
/// colour, truncated, on the frame; slot 669 with the second colour times
/// `+0x7c` likewise; [`sb_nodes::scale_ratio_801`] on the first colour.
/// Bit 1 hides (slot 267, `yiRenderDisable`, 1) a node whose first three
/// channels all dropped below the limit `+0xa8` and clears the bit, or
/// shows (slot 271, `yiRenderEnable`, 1) one whose channels are all at it
/// or above and sets it. With bit 2, a progress at the end `+0xa4` or
/// past it makes the node leave ([`s3_nodes::leave_and_reset`], a cursor
/// on the frame). Returns the last call's `eax`, or the walk's.
///
/// `magic_0003` 0x10001470 (1014 bytes; 107 DLLs, `magic_0087`/`0088`
/// 0x10001620).
#[allow(clippy::too_many_lines)] // the original's branches, step by step
pub fn colour_node_fade<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let this = fr.ecx;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let (eax, moved) = walk(g, this, this, fr.eax);
    if moved.is_none() {
        return Ret::eax(eax);
    }
    let ebp = fr.esp.wrapping_sub(4);
    let n = g.read::<u32>(this);
    let k = f32_at(g, w(0x157));
    // the colours' channels in the original's order (first, second,
    // first, second, ...)
    let pairs = [0x1cu32, 0x4c, 0x20, 0x50, 0x24, 0x54, 0x28, 0x58];
    if g.read::<u8>(n + 0xac) & 1 != 0 {
        for x in pairs {
            let rate = f32_at(g, n + 0x9c);
            let (to, at) = (f32_at(g, n + x + 0x10), f32_at(g, n + x));
            set_f32(g, n + x, add(mul(sub(to, at), rate), at));
        }
        let p = f32_at(g, n + 0xa0);
        let rate = f32_at(g, n + 0x9c);
        set_f32(g, n + 0xa0, add(mul(sub(k, p), rate), p));
    } else {
        let rate = f32_at(g, n + 0x9c);
        let p = add(rate, f32_at(g, n + 0xa0));
        set_f32(g, n + 0xa0, p);
        if at_least(p, k) {
            g.write::<u32>(n + 0xa0, ONE);
            for x in pairs {
                let to = g.read::<u32>(n + x + 0x10);
                g.write::<u32>(n + x, to);
            }
        } else {
            for x in pairs {
                // the first product takes the rate from its register,
                // the others from memory (the same value)
                let speed = f32_at(g, n + x + 0x20);
                let r = f32_at(g, n + 0x9c);
                set_f32(g, n + x, add(mul(speed, r), f32_at(g, n + x)));
            }
        }
    }
    let first = n + 0x1c;
    let ints = ebp.wrapping_sub(0x14);
    let t = slot_target(g, w(0x28a), 353);
    g.call(t, 0, &[first]);
    for i in 0..4 {
        let scale = crate::sb_sse::cvt(g.read::<u32>(n + 0x8c + 4 * i));
        let x = mul(scale, f32_at(g, first + 4 * i));
        g.write::<u32>(ints + 4 * i, trunc(x));
    }
    let t = slot_target(g, w(0x2f7), 332);
    g.call(t, 0, &[ints, first]);
    for i in 0..4 {
        let x = mul(f32_at(g, n + 0x7c + 4 * i), f32_at(g, n + 0x4c + 4 * i));
        g.write::<u32>(ints + 4 * i, trunc(x));
    }
    let t = slot_target(g, w(0x32f), 669);
    g.call(t, 0, &[ints]);
    let mut eax = sb_nodes::scale_ratio_801(g, img, img.rel32_target(f + 0x341), first);
    let flags = g.read::<u32>(n + 0xac);
    let limit = f32_at(g, n + 0xa8);
    let channels = [
        f32_at(g, n + 0x1c),
        f32_at(g, n + 0x20),
        f32_at(g, n + 0x24),
    ];
    if flags & 2 != 0 {
        // comiss limit, x; jbe: hidden once every channel is below
        if channels.iter().all(|&x| limit > x) {
            let t = slot_target(g, w(0x368), 267);
            eax = g.call(t, 0, &[1]).eax;
            let flags = g.read::<u32>(n + 0xac);
            g.write::<u32>(n + 0xac, flags & !2);
        }
    } else if channels.iter().all(|&x| at_least(x, limit)) {
        let t = slot_target(g, w(0x3a8), 271);
        eax = g.call(t, 0, &[1]).eax;
        let flags = g.read::<u32>(n + 0xac);
        g.write::<u32>(n + 0xac, flags | 2);
    }
    if g.read::<u8>(n + 0xac) & 4 != 0 && at_least(f32_at(g, n + 0xa0), f32_at(g, n + 0xa4)) {
        // the cursor `[ebp - 0x18]`; the call's return address below the
        // frame (`sub esp, 0x18` and three pushes)
        let cursor = ebp.wrapping_sub(0x18);
        g.write::<u32>(cursor, n);
        let callee = Frame {
            at: img.rel32_target(f + 0x3e1),
            esp: ebp.wrapping_sub(0x28),
            ecx: cursor,
            ..*fr
        };
        eax = s3_nodes::leave_and_reset(g, img, &callee);
    }
    Ret::eax(eax)
}

/// [`colour_node_fade`] (the `rel32`s: [`sb_nodes::scale_ratio_801`],
/// [`s3_nodes::leave_and_reset`], `__security_check_cookie`).
pub const COLOUR_NODE_FADE: Sig = Sig {
    len: 1014,
    hash: 0x180b_6c7e_ebf4_bb2f,
    calls: &[
        (0x341, Callee::Sig(&sb_nodes::SCALE_RATIO_801)),
        (0x3e1, Callee::Sig(&s3_nodes::LEAVE_AND_RESET)),
        (0x3ee, Callee::Any),
    ],
    imports: &[],
};

/// A character node's motion state (`ecx` = the cursor; a /GS frame).
/// The walk; when the cursor moved, the node `n` (its flags `u16 +0x90`;
/// bit 0 picks the character `+0x54` over the model `+0x50` in most
/// steps):
/// - bit 4: the object `+0x4c`'s handler runs once with a cursor in the
///   frame;
/// - bit 8 (waiting for a motion): when slot 20 (`+0x54`) or 411
///   (`+0x50`) answers non-zero, bit 5 starts the motion `+0x60` (slot 36
///   or 228), else a model gets motion 0x10 (slot 228) and bit 8 is
///   cleared;
/// - bit 1 with the count `+0x58` at 0: slot 257 (`+0x54`, 1) or 296
///   (`+0x50`, 0); the count always goes down by one;
/// - bit 2 (a model only): a wait `u16 +0x94` counts down first; then
///   the steps `+0x5c` count down, each moving the model: slot 234 gives
///   `+0x1c - +0x2c` (`[ebp - 0x14]`), slot 667 scales it by `+0x70`,
///   slot 233 divides it by the steps left (as an unsigned count, through
///   the DLL's `u32` correction table, operand `+0x1e5`), slot 248 adds
///   `+0x2c`, slot 255 places the model. With no steps left bit 2 is
///   cleared;
/// - else bit 3 (a model only): slot 217 reads the model's position, slot
///   234 the offset to `+0x2c`, slot 233 scales it by `+0x74`, slot 248
///   adds it with no height, slot 255 places the model;
/// - else bit 6 (a model only): a falling height `+0x78` (`+0x7c` less,
///   then damped by `+0x88` towards the limit `K`, operand `+0x322`) is
///   taken off the model's height; at the floor ([`sb_nodes::float_289`]
///   plus slot 664's float when slot 534 answers) it lands (bit 10): a
///   bounce `+0x92` counts down, each taking `+0x84 * +0x80` off, the last
///   one ending the fall (slot 538 with `+0x64`, bit 6 cleared). Slot 255
///   places the model.
///
/// Returns the last call's `eax`, the flags (or the wait or flags left
/// as the original computes them) where it stops early, or the walk's.
///
/// `magic_0003` 0x10002600 (1114 bytes; 103 DLLs, `magic_0087`/`0088`
/// 0x10002840).
#[allow(clippy::too_many_lines)] // the original's branches, step by step
pub fn motion_state<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let this = fr.ecx;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let (eax, moved) = walk(g, this, this, fr.eax);
    if moved.is_none() {
        return Ret::eax(eax);
    }
    let ebp = fr.esp.wrapping_sub(4);
    let n = g.read::<u32>(this);
    let flags_of = |g: &G| u32::from(g.read::<u16>(n + 0x90));
    if g.read::<u8>(n + 0x90) & 0x10 != 0 {
        let obj = g.read::<u32>(n + 0x4c);
        let local = ebp.wrapping_sub(0x30);
        g.write::<u32>(local, obj);
        let pass = g.read::<u8>(this + 5);
        g.write::<u8>(local + 5, pass);
        let handler = g.read::<u32>(g.read::<u32>(obj) + 0x1c);
        g.call(handler, local, &[local]);
    }
    let flags = flags_of(g);
    if flags & 0x100 != 0 {
        let ready = if flags & 1 != 0 {
            let t = slot_target(g, w(0x92), 20);
            g.call(t, 0, &[g.read::<u32>(n + 0x54)]).eax
        } else {
            let t = slot_target(g, w(0x92), 411);
            g.call(t, 0, &[g.read::<u32>(n + 0x50)]).eax
        };
        if ready != 0 {
            let flags = flags_of(g);
            if flags & 0x20 != 0 {
                let motion = g.read::<u32>(n + 0x60);
                if flags & 1 != 0 {
                    let t = slot_target(g, w(0xc3), 36);
                    g.call(t, 0, &[g.read::<u32>(n + 0x54), motion]);
                } else {
                    let t = slot_target(g, w(0xc3), 228);
                    g.call(t, 0, &[g.read::<u32>(n + 0x50), motion]);
                }
            } else {
                if flags & 1 == 0 {
                    let t = slot_target(g, w(0xee), 228);
                    g.call(t, 0, &[g.read::<u32>(n + 0x50), 0x10]);
                }
                let v = g.read::<u16>(n + 0x90) & 0xfeff;
                g.write::<u16>(n + 0x90, v);
            }
        }
    }
    let flags = flags_of(g);
    if flags & 2 != 0 && g.read::<u32>(n + 0x58) == 0 {
        if flags & 1 != 0 {
            let t = slot_target(g, w(0x122), 257);
            g.call(t, 0, &[g.read::<u32>(n + 0x54), 1]);
        } else {
            let t = slot_target(g, w(0x122), 296);
            g.call(t, 0, &[g.read::<u32>(n + 0x50), 0]);
        }
    }
    let flags = flags_of(g);
    let count = g.read::<u32>(n + 0x58).wrapping_sub(1);
    g.write::<u32>(n + 0x58, count);
    let model = || n + 0x50;
    let v = ebp.wrapping_sub(0x14);
    if flags & 4 != 0 {
        if flags & 1 != 0 {
            return Ret::eax(flags);
        }
        let wait = u32::from(g.read::<u16>(n + 0x94));
        if wait != 0 {
            let left = wait - 1;
            g.write::<u16>(n + 0x94, left as u16);
            return Ret::eax(left);
        }
        let steps = g.read::<u32>(n + 0x5c);
        if steps == 0 {
            let flags = flags & 0xfffb;
            g.write::<u16>(n + 0x90, flags as u16);
            return Ret::eax(flags);
        }
        g.write::<u32>(n + 0x5c, steps - 1);
        let t = slot_target(g, w(0x1a4), 234);
        g.call(t, 0, &[v, n + 0x1c, n + 0x2c]);
        let t = slot_target(g, w(0x1c3), 667);
        let k = g.read::<u32>(n + 0x70);
        g.call(t, 0, &[v, v, k]);
        // cvtdq2pd, then the table's 0.0 or 2^32 by the sign bit: the
        // count as unsigned; cvtpd2ps rounds it
        let left = g.read::<u32>(n + 0x5c);
        let fix = f64::from_bits(g.read::<u64>(w(0x1e5).wrapping_add(8 * (left >> 31))));
        let left = (f64::from(left.cast_signed()) + fix) as f32;
        let t = slot_target(g, w(0x1f8), 233);
        g.call(t, 0, &[v, v, left.to_bits()]);
        let t = slot_target(g, w(0x20b), 248);
        g.call(t, 0, &[v, n + 0x2c, v, v, v, left.to_bits()]);
        let t = slot_target(g, w(0x21c), 255);
        let m = g.read::<u32>(model());
        let words = [m, v, v, n + 0x2c, v, v, v, left.to_bits()];
        return Ret::eax(g.call(t, 0, &words).eax);
    }
    if flags & 8 != 0 {
        if flags & 1 != 0 {
            return Ret::eax(flags);
        }
        let d = ebp.wrapping_sub(0x24);
        let t = slot_target(g, w(0x271), 217);
        g.call(t, 0, &[g.read::<u32>(model()), v]);
        let t = slot_target(g, w(0x28d), 234);
        let m = g.read::<u32>(model());
        g.call(t, 0, &[d, n + 0x2c, v, m, v]);
        let t = slot_target(g, w(0x2ac), 233);
        let k = g.read::<u32>(n + 0x74);
        g.call(t, 0, &[d, d, k]);
        // [ebp - 0x18]: the offset's fourth word
        g.write::<u32>(d + 0xc, 0);
        let t = slot_target(g, w(0x2c2), 248);
        g.call(t, 0, &[v, v, d, d, d, k]);
        let t = slot_target(g, w(0x2da), 255);
        let m = g.read::<u32>(model());
        let words = [m, v, v, v, d, d, d, k];
        return Ret::eax(g.call(t, 0, &words).eax);
    }
    if flags & 0x40 == 0 || flags & 1 != 0 {
        return Ret::eax(flags);
    }
    let y = sub(f32_at(g, n + 0x78), f32_at(g, n + 0x7c));
    let damp = f32_at(g, n + 0x88);
    let d = mul(sub(y, damp), damp);
    set_f32(g, n + 0x78, y);
    // comiss y, K; jb: below the limit (or unordered) the damping adds
    let fall = if y < f32_at(g, w(0x322)) || y.is_nan() {
        add(d, y)
    } else {
        sub(y, d)
    };
    set_f32(g, n + 0x78, fall);
    let t = slot_target(g, w(0x351), 217);
    g.call(t, 0, &[g.read::<u32>(model()), v]);
    let h = sub(f32_at(g, v + 4), f32_at(g, n + 0x78));
    set_f32(g, v + 4, h);
    let floor_at = img.rel32_target(f + 0x373);
    let (mut floor, _) = sb_nodes::float_289(g, img, floor_at, g.read::<u32>(model()));
    let t = slot_target(g, w(0x378), 534);
    let m = g.read::<u32>(model());
    // 217's words stay below
    let on = g.call(t, 0, &[m, m, v]).eax;
    if on != 0 {
        let t = slot_target(g, w(0x394), 664);
        let extra = st0_f32(g.call(t, 0, &[g.read::<u32>(model())]));
        floor = add(f32::from_bits(extra), floor);
    }
    let h = f32_at(g, v + 4);
    if at_least(h, floor) {
        let flags = g.read::<u16>(n + 0x90) | 0x400;
        g.write::<u16>(n + 0x90, flags);
        let bounces = g.read::<u16>(n + 0x92).wrapping_add(0xffff);
        g.write::<u16>(n + 0x92, bounces);
        if bounces == 0 {
            let t = slot_target(g, w(0x40a), 538);
            let (m, k) = (g.read::<u32>(model()), g.read::<u32>(n + 0x64));
            g.call(t, 0, &[m, k]);
            let flags = g.read::<u16>(n + 0x90) & 0xffbf;
            g.write::<u16>(n + 0x90, flags);
            set_f32(g, v + 4, floor);
        } else {
            let lift = mul(f32_at(g, n + 0x84), f32_at(g, n + 0x80));
            set_f32(g, n + 0x78, lift);
            set_f32(g, v + 4, sub(floor, lift));
        }
    }
    let t = slot_target(g, w(0x437), 255);
    let m = g.read::<u32>(model());
    Ret::eax(g.call(t, 0, &[m, v]).eax)
}

/// [`motion_state`] (the `rel32`s: [`sb_nodes::float_289`] and five
/// `__security_check_cookie`s).
pub const MOTION_STATE: Sig = Sig {
    len: 1114,
    hash: 0xabd2_aa5a_d137_84cc,
    calls: &[
        (0x17e, Callee::Any),
        (0x237, Callee::Any),
        (0x254, Callee::Any),
        (0x2f5, Callee::Any),
        (0x373, Callee::Sig(&sb_nodes::FLOAT_289)),
        (0x452, Callee::Any),
    ],
    imports: &[],
};

/// The summon's targets (internal, `ecx` = the effect's actor; a /GS
/// frame): `G_ACTOR` (operand `+0x21`) = `ecx`; `G_CHR` (`+0x28`) = slot
/// 168 (`MsGetEff2Chr`); slot 218 (`MsGetTargetList`) gives the targets
/// and their count `G_N` (`+0x32`), copied as bytes into `G_LIST`
/// (`+0x69`); each target's battle position (slot 531, `MsGetChrBtlPos`)
/// goes into `G_POS` (`+0x75`, 16 bytes each, `w` = 1.0); the
/// character's own into the frame (`[ebp - 0x14]`); its ground height
/// (slot 289, `MsGetChrGround`, ST0) into `G_GROUND` (`+0xd8`). Then the
/// statics are reset ([`crate::sa_summon::statics_init`]). Returns its
/// `eax`.
///
/// `magic_0003` 0x10006390 (245 bytes; 69 DLLs).
pub fn targets_init<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, actor: u32) -> u32 {
    let w = |o: u32| img.word(f + o);
    let ebp = esp.wrapping_sub(4);
    g.write::<u32>(w(0x21), actor);
    let t = slot_target(g, w(0x11), 168);
    let chr = g.call(t, 0, &[actor]).eax;
    g.write::<u32>(w(0x28), chr);
    let count_at = w(0x32);
    let t = slot_target(g, w(0x2d), 218);
    let stored = g.read::<u32>(w(0x38));
    // 168's word (the actor) stays below
    let src = g.call(t, 0, &[stored, count_at, actor]).eax;
    let n = g.read::<i32>(w(0x46));
    let list = w(0x69);
    for i in 0..n.max(0).cast_unsigned() {
        let b = g.read::<u8>(src.wrapping_add(i));
        g.write::<u8>(list + i, b);
    }
    if n > 0 {
        // a do-while: the count is read again after each target
        let pos = w(0x75);
        let mut i = 0u32;
        loop {
            let target = u32::from(g.read::<u8>(w(0x83) + i));
            let p = pos + 0x10 * i;
            let t = slot_target(g, w(0x8a), 531);
            g.call(t, 0, &[target, p]);
            g.write::<u32>(p + 0xc, ONE);
            i += 1;
            if i.cast_signed() >= g.read::<i32>(w(0xa6)) {
                break;
            }
        }
    }
    let t = slot_target(g, w(0xb1), 531);
    let chr_b = g.read::<u32>(w(0xb7));
    g.call(t, 0, &[chr_b, ebp.wrapping_sub(0x14)]);
    let t = slot_target(g, w(0xc4), 289);
    let chr = g.read::<u32>(w(0xca));
    // 531's words stay below
    let ground = st0_f32(g.call(t, 0, &[chr, chr_b, ebp.wrapping_sub(0x14)]));
    g.write::<u32>(w(0xd8), ground);
    // `sub esp, 0x14`, three pushes and the return address
    crate::sa_summon::statics_init(g, img, img.rel32_target(f + 0xe0), ebp.wrapping_sub(0x24))
}

/// [`targets_init`] (the `rel32`s: the statics' reset and
/// `__security_check_cookie`).
pub const TARGETS_INIT: Sig = Sig {
    len: 245,
    hash: 0x8cd0_72a7_90ae_cfc8,
    calls: &[
        (0xe0, Callee::Sig(&crate::sa_summon::STATICS_INIT)),
        (0xed, Callee::Any),
    ],
    imports: &[],
};

/// A summon's start (a table entry, cdecl `(actor)`): the targets
/// ([`targets_init`]); the DLL's task list, two static nodes and their
/// links set up (words and immediates from the code); slot 718 and slot
/// 719 `(part, 8, 0)` on the DLL's part, whose handle goes into `G_H`;
/// slot 711 `(handle, program, 0)` with `**G_LOCK` 1 around it; slot 710
/// loads the part's data; slots 222 and 221 `(part, 0, 1000, 2)`; a task
/// is appended to the list (its links and three words cleared, the
/// list's flag bit 0 cleared). Returns 0. `cleared`: the operand
/// offsets of the task's words cleared last.
fn start<G: Game>(g: &mut G, img: &Image, fr: &Frame, cleared: &[u32]) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let actor = fr.arg(g, 0);
    // push ebp; push esi; call
    targets_init(
        g,
        img,
        img.rel32_target(f + 0x08),
        fr.esp.wrapping_sub(12),
        actor,
    );
    for (dst, src) in [(0x13, 0x17), (0x1d, 0x21), (0x27, 0x2b), (0x31, 0x35)] {
        g.write::<u32>(w(dst), w(src));
    }
    g.write::<u32>(w(0x3b), 1);
    g.write::<u32>(w(0x45), w(0x49));
    g.write::<u16>(w(0x50), 0x1ff);
    g.write::<u32>(w(0x58), w(0x5c));
    g.write::<u32>(w(0x62), w(0x66));
    g.write::<u32>(w(0x6c), 0);
    g.write::<u32>(w(0x76), w(0x7a));
    g.write::<u16>(w(0x81), 0xff);
    g.write::<u32>(w(0x89), w(0x8d));
    g.write::<u32>(w(0x93), w(0x97));
    g.write::<u32>(w(0x9d), 0);
    let mut s = Pushes::default();
    s.push(w(0xac));
    g.write::<u32>(w(0xb2), w(0xb6));
    s.call(g, slot_target(g, w(0x0d), 718), 0);
    s.push_all(&[0, 8, w(0xcc)]);
    let handle = s.call(g, slot_target(g, w(0xbd), 719), 0).eax;
    let lock = g.read::<u32>(w(0xd4));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(w(0xdd), handle);
    s.push_all(&[0, w(0xe2), handle]);
    g.write::<u32>(lock, 1);
    s.call(g, slot_target(g, w(0xe8), 711), 0);
    let lock = g.read::<u32>(w(0xfb));
    g.write::<u32>(lock, saved);
    let handle = g.read::<u32>(w(0x112));
    s.push_all(&[0x0010_0000, w(0x10c), handle, w(0x11d)]);
    s.call(g, slot_target(g, w(0x107), 710), 0);
    s.push_all(&[2, 1000, 0, w(0x138)]);
    s.call(g, slot_target(g, w(0x124), 222), 0);
    s.push_all(&[2, 1000, 0, w(0x153)]);
    s.call(g, slot_target(g, w(0x13f), 221), 0);
    let tail = g.read::<u32>(w(0x15a));
    g.write::<u32>(w(0x162), tail);
    for (dst, src) in [
        (0x168, 0x16c),
        (0x172, 0x176),
        (0x17c, 0x180),
        (0x186, 0x18a),
    ] {
        g.write::<u32>(w(dst), w(src));
    }
    g.write::<u32>(tail.wrapping_add(4), w(0x191));
    let flags = g.read::<u32>(w(0x197));
    g.write::<u32>(w(0x197), flags & !1);
    g.write::<u32>(w(0x19e), w(0x1a2));
    for &dst in cleared {
        g.write::<u32>(w(dst), 0);
    }
    Ret::eax(0)
}

/// [`start`] with three task words cleared.
///
/// `magic_0003` 0x100068d0 (457 bytes; 18 DLLs).
pub fn summon_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    start(g, img, fr, &[0x1a8, 0x1b2, 0x1bc])
}

/// [`start`] with one task word cleared.
///
/// `magic_0202` 0x10007760 (437 bytes; 21 DLLs).
pub fn summon_start_b<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    start(g, img, fr, &[0x1a8])
}

/// [`summon_start_b`] (the `rel32`: [`targets_init`]).
pub const SUMMON_START_B: Sig = Sig {
    len: 437,
    hash: 0xe46b_13bb_4b92_a306,
    calls: &[(0x08, Callee::Sig(&TARGETS_INIT))],
    imports: &[],
};

/// [`summon_start`] (the `rel32`: [`targets_init`]).
pub const SUMMON_START: Sig = Sig {
    len: 457,
    hash: 0x3e98_7198_16df_4276,
    calls: &[(0x08, Callee::Sig(&TARGETS_INIT))],
    imports: &[],
};

/// A node holding one model node per target (internal, `ecx` = a set-up
/// record: `+0x0` the node, `+0x4` its parent, `+0x8` the character,
/// `+0xc` a word and `+0x10` a flags byte kept in the node; an 8-aligned
/// frame): the node's lists set up and the node appended to the parent's
/// children ([`attach`]); `+0x4e4`, `+0x4ec` and the character `+0x4e8`
/// kept; slot 218 (`MsGetTargetList`) gives the targets (`+0x4e0`) and
/// their count (`+0x4dc`). Each target `i` gets the node at `+0x1c +
/// 0x98 i` (vtable `V`, operand `+0xb8`), set up by
/// [`crate::sa_summon::model_node_init`] from a record on the frame
/// (`[esp + 0x10]`: the node, the record's parent, the target's byte, no
/// flags). Returns the last set-up's `eax`, or slot 218's.
///
/// `magic_0411` 0x10006000 (215 bytes; 21 DLLs).
pub fn target_nodes<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let rec = fr.ecx;
    let parent = g.read::<u32>(rec + 4);
    let n = g.read::<u32>(rec);
    g.write::<u16>(n + 0x1a, 0xff);
    g.write::<u32>(n + 0x10, n);
    g.write::<u32>(n + 0xc, n);
    g.write::<u32>(n + 0x14, parent);
    if parent != 0 {
        attach(g, n, parent, ListAt::of(parent));
    }
    let x = g.read::<u32>(rec + 0xc);
    let chr = g.read::<u32>(rec + 8);
    g.write::<u32>(n + 0x4e4, x);
    let flags = g.read::<u8>(rec + 0x10);
    g.write::<u8>(n + 0x4ec, flags);
    g.write::<u32>(n + 0x4e8, chr);
    let t = slot_target(g, w(0x5f), 218);
    let targets = g.call(t, 0, &[chr, n + 0x4dc]).eax;
    g.write::<u32>(n + 0x4e0, targets);
    let mut eax = targets;
    if g.read::<i32>(n + 0x4dc) > 0 {
        // push ebp; and esp, -8; sub esp, 0x14; three pushes
        let e = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x14 + 12);
        let args = e + 0x10;
        let init = img.rel32_target(f + 0xbd);
        let mut i = 0u32;
        loop {
            let node = n + 0x1c + 0x98 * i;
            let parent = g.read::<u32>(rec + 4);
            let target = u32::from(g.read::<u8>(g.read::<u32>(n + 0x4e0).wrapping_add(i)));
            for (k, x) in [node, parent, target].into_iter().enumerate() {
                g.write::<u32>(args + 4 * k as u32, x);
            }
            g.write::<u8>(args + 0xc, 0);
            g.write::<u32>(node, w(0xb8));
            eax = crate::sa_summon::model_node_init(g, img, init, args);
            i += 1;
            if i.cast_signed() >= g.read::<i32>(n + 0x4dc) {
                break;
            }
        }
    }
    eax
}

/// [`target_nodes`].
pub const TARGET_NODES: Sig = Sig {
    len: 215,
    hash: 0x8465_1687_28da_7b85,
    calls: &[(0xbd, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT))],
    imports: &[],
};
