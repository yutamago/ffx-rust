//! Shared functions without a family module yet, ported by the shared-3
//! list. Each doc names the DLL and address it was ported from, its size
//! and how many DLLs carry it.

use crate::l3_common::{ListAt, attach};
use crate::runtime::slot_target;
use crate::sa_misc::{SET_FOUR, UNIT_BLOCK, set_four, unit_block};
use crate::sa_pool::{GIVE_BACK, TAKE, give_back, take};
use crate::sb_misc::{EASE_SAMPLE, ease_sample};
use crate::sb_sse::{add, div, mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `1.0f32`'s bits.
const ONE: u32 = 0x3f80_0000;

fn f32_at<G: Game>(g: &G, a: u32) -> f32 {
    f32::from_bits(g.read::<u32>(a))
}

#[allow(clippy::cast_precision_loss)] // cvtdq2ps rounds the same
fn cvt(i: i32) -> f32 {
    i as f32
}

/// A colour fade task (cdecl `(task)`, its state at `+0x14`: two colour
/// scales `+0x0` (from) and `+0x10` (to), two byte colours `+0x20`
/// (from) and `+0x24` (to), a delay `+0x28`, the time left `+0x2c` of
/// `+0x30`, a flag `+0x34`). While the delay is positive it counts down
/// by the game's frame time (`*G_CLOCK`, operand `+0x11`). Then the time
/// left counts down (to 0 at the least) and the scale `to * (K - r) +
/// from * r` (`r` = left / total, `K` the constant at `+0x60`) goes to
/// `yiSetColorScale` (slot 353) and [`set_four`]; with the flag 1, the
/// colour `(to * (total - left) + from * left) / total` per byte to
/// slot 397 (`idiv`). When no time is left the task is given back
/// ([`give_back`]). Returns the last call's `eax` (the clock pointer
/// while waiting).
///
/// `magic_0072` 0x10003500 (424 bytes, a /GS frame; 83 DLLs).
pub fn colour_fade<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let clock = g.read::<u32>(w(0x11));
    let task = fr.arg(g, 0);
    let s = g.read::<u32>(task + 0x14);
    let delay = g.read::<i32>(s + 0x28);
    if delay > 0 {
        let dt = g.read::<i32>(clock);
        g.write::<i32>(s + 0x28, delay.wrapping_sub(dt));
        return Ret::eax(clock);
    }
    let dt = g.read::<i32>(clock);
    let left = g.read::<i32>(s + 0x2c).wrapping_sub(dt);
    // cmovs: the sign of the subtraction
    let left = if left < 0 { 0 } else { left };
    g.write::<i32>(s + 0x2c, left);
    let total = cvt(g.read::<i32>(s + 0x30));
    let r = div(cvt(g.read::<i32>(s + 0x2c)), total);
    let q = sub(f32_at(g, w(0x60)), r);
    let v = ebp.wrapping_sub(0x14);
    for i in 0..4 {
        let x = add(
            mul(f32_at(g, s + 0x10 + 4 * i), q),
            mul(f32_at(g, s + 4 * i), r),
        );
        g.write::<u32>(v + 4 * i, x.to_bits());
    }
    let t = slot_target(g, w(0x82), 353);
    g.call(t, 0, &[v]);
    let mut eax = set_four(g, img, img.rel32_target(f + 0xef), v);
    if g.read::<u32>(s + 0x34) == 1 {
        let c = ebp.wrapping_sub(0x24);
        for k in 0..4 {
            let total = g.read::<i32>(s + 0x30);
            let left = g.read::<i32>(s + 0x2c);
            let to = i32::from(g.read::<u8>(s + 0x24 + k));
            let from = i32::from(g.read::<u8>(s + 0x20 + k));
            let n = total
                .wrapping_sub(left)
                .wrapping_mul(to)
                .wrapping_add(from.wrapping_mul(left));
            // idiv: a zero total (or an overflow) faults in the original
            let x = n.checked_div(total).unwrap_or(0);
            g.write::<i32>(c + 4 * k, x);
        }
        let t = slot_target(g, w(0x17b), 397);
        // the words on the stack: the colour, then the pushed esi (the
        // caller's, junk to the slot)
        eax = g.call(t, 0, &[c]).eax;
    }
    if g.read::<i32>(s + 0x2c) <= 0 {
        eax = give_back(g, img, img.rel32_target(f + 0x194), task);
    }
    Ret::eax(eax)
}

/// [`colour_fade`].
pub const COLOUR_FADE: Sig = Sig {
    len: 424,
    hash: 0x9ea5_c949_1a00_5fb5,
    calls: &[
        (0x31, Callee::Any),
        (0xef, Callee::Sig(&SET_FOUR)),
        (0x194, Callee::Sig(&GIVE_BACK)),
        (0x1a0, Callee::Any),
    ],
    imports: &[],
};

/// A colour step task of the `magic_0162` family (cdecl `(task)`, its
/// state at `+0x14`): a delay `+0x88` counts down first; then the time
/// left `+0x8c` of `+0x90` (or, once it is 0, a hold `+0x94`) counts
/// down and the four byte channels `(to * (total - left) + from * left)
/// / total` (`idiv`; `from` the bytes `+0x80`, `to` `+0x84`) go to slot
/// 915 as floats, the first three divided by `K` (operand `+0x100`), the
/// fourth times `K1` (`+0x119`), with slot 776 `(u8 +0x98)`'s answer.
/// The original keeps the second channel in the top byte of its own
/// argument word (`[ebp + 0xb]`), and the port writes it there too.
/// With neither left nor hold the task is given back ([`give_back`]).
/// Returns the last call's `eax` (the new delay while it counts).
///
/// `magic_0162` 0x10007a90 (375 bytes; 67 DLLs).
pub fn colour_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let task = fr.arg(g, 0);
    let s = g.read::<u32>(task + 0x14);
    let delay = g.read::<i32>(s + 0x88);
    if delay > 0 {
        let d = delay - 1;
        g.write::<i32>(s + 0x88, d);
        return Ret::eax(d.cast_unsigned());
    }
    let left = g.read::<u32>(s + 0x8c);
    if left != 0 {
        g.write::<u32>(s + 0x8c, left.wrapping_sub(1));
    } else {
        let hold = g.read::<u32>(s + 0x94);
        if hold == 0 {
            return Ret::eax(give_back(g, img, img.rel32_target(f + 0x3b), task));
        }
        g.write::<u32>(s + 0x94, hold.wrapping_sub(1));
    }
    let left = g.read::<i32>(s + 0x8c);
    let total = g.read::<i32>(s + 0x90);
    let channel = |g: &G, k: u32| {
        let to = i32::from(g.read::<u8>(s + 0x84 + k));
        let from = i32::from(g.read::<u8>(s + 0x80 + k));
        let n = total
            .wrapping_sub(left)
            .wrapping_mul(to)
            .wrapping_add(from.wrapping_mul(left));
        // idiv: a zero total (or an overflow) faults in the original
        n.checked_div(total).unwrap_or(0).cast_unsigned() as u8
    };
    let c0 = channel(g, 0);
    let c1 = channel(g, 1);
    // [ebp + 0xb]: the argument word's top byte
    g.write::<u8>(fr.esp + 7, c1);
    let c2 = channel(g, 2);
    let c3 = channel(g, 3);
    let t = slot_target(g, w(0xf0), 776);
    let x = g.call(t, 0, &[u32::from(g.read::<u8>(s + 0x98))]).eax;
    let k = f32_at(g, w(0x100));
    let c1 = g.read::<u8>(fr.esp + 7);
    let v = [
        div(f32::from(c0), k),
        div(f32::from(c1), k),
        div(f32::from(c2), k),
        mul(f32::from(c3), f32_at(g, w(0x119))),
    ];
    let t = slot_target(g, w(0x158), 915);
    let out = g.call(
        t,
        0,
        &[
            v[0].to_bits(),
            v[1].to_bits(),
            v[2].to_bits(),
            v[3].to_bits(),
            x,
        ],
    );
    Ret::eax(out.eax)
}

/// [`colour_step`].
pub const COLOUR_STEP: Sig = Sig {
    len: 375,
    hash: 0x3f32_af50_c000_1188,
    calls: &[(0x3b, Callee::Sig(&GIVE_BACK))],
    imports: &[],
};

/// A colour node set up from a group (`ecx`: `+0x0` the node, `+0x4` its
/// parent): pass byte `+0x1a` 0xff, an empty child list, appended to the
/// parent's list when there is one; [`unit_block`]; slot 331
/// (`node + 0x8c`); slot 558 fills four integers on the frame, which
/// become the floats `+0x7c..+0x88`; the 4x6 block `+0x1c` gets rows of
/// 1.0 except the third and sixth (0.0); slot 353 `(node + 0x1c)` and
/// slot 271 `(4)`, each seeing the earlier calls' words; `+0xa0` = 1.0,
/// `+0xa4` = 1.0, `+0xa8` = 0.005, `+0xac` = 1. Returns slot 271's `eax`.
///
/// `magic_0042` 0x10002190 (432 bytes, `ecx` = the group, a /GS frame; 67
/// DLLs).
pub fn colour_node<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let node = g.read::<u32>(fr.ecx);
    let parent = g.read::<u32>(fr.ecx + 4);
    g.write::<u16>(node + 0x1a, 0xff);
    g.write::<u32>(node + 0x10, node);
    g.write::<u32>(node + 0xc, node);
    g.write::<u32>(node + 0x14, parent);
    if parent != 0 {
        attach(g, node, parent, ListAt::of(parent));
    }
    unit_block(g, img, img.rel32_target(f + 0x49));
    let back = node + 0x8c;
    let t = slot_target(g, w(0x55), 331);
    g.call(t, 0, &[back]);
    let ints = ebp.wrapping_sub(0x14);
    let t = slot_target(g, w(0x66), 558);
    g.call(t, 0, &[ints, back]);
    for i in 0..4 {
        let x = cvt(g.read::<i32>(ints + 4 * i));
        g.write::<u32>(node + 0x7c + 4 * i, x.to_bits());
        for (row, v) in [
            (0x1c, ONE),
            (0x2c, ONE),
            (0x3c, 0),
            (0x4c, ONE),
            (0x5c, ONE),
            (0x6c, 0),
        ] {
            g.write::<u32>(node + row + 4 * i, v);
        }
    }
    let t = slot_target(g, w(0x129), 353);
    g.call(t, 0, &[node + 0x1c, ints, back]);
    let t = slot_target(g, w(0x168), 271);
    let eax = g.call(t, 0, &[4, node + 0x1c, ints, back]).eax;
    g.write::<u32>(node + 0xac, 1);
    g.write::<u32>(node + 0xa4, ONE);
    g.write::<u32>(node + 0xa8, 0x3ba3_d70a);
    g.write::<u32>(node + 0xa0, ONE);
    eax
}

/// [`colour_node`].
pub const COLOUR_NODE: Sig = Sig {
    len: 432,
    hash: 0xa074_af6a_01d0_819b,
    calls: &[(0x49, Callee::Sig(&UNIT_BLOCK)), (0x1a8, Callee::Any)],
    imports: &[],
};

/// A fade record (`ecx`: two colours of four floats, `xmm1`'s value in
/// the first three words of `+0x0` and `xmm2`'s in `+0x10`, each with
/// 1.0 last; a value not above 0 becomes `K`, operand `+0x07`; both kept
/// in the DLL's globals `+0x2d`/`+0x35`; the stack word at `+0x28`, `edx`
/// at `+0x2c` and `+0x30`, `+0x34` 0) handed to a pool node taken for the
/// DLL's task function (operand `+0x25`, [`take`]): the node's record
/// `+0x14`, flag byte `+0x18` 1, `+0x4` 0, then appended to the DLL's
/// task list (`G_HEAD` `+0x7b`, `G_TAIL` `+0x93`; a lone node is its own
/// list). Returns the node.
///
/// `magic_0084` 0x10001d60 (187 bytes, `ecx`/`edx`/`xmm1`/`xmm2` and one
/// stack word; 33 DLLs).
pub fn fade_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = f32_at(g, w(0x07));
    // comiss 0, x; jb: kept when 0 < x or unordered
    let keep = |x: f32| if 0.0 < x || x.is_nan() { x } else { k };
    let a = keep(fr.xmm_f32(1));
    let b = keep(fr.xmm_f32(2));
    let rec = fr.ecx;
    let arg = fr.arg(g, 0);
    g.write::<u32>(w(0x2d), a.to_bits());
    g.write::<u32>(w(0x35), b.to_bits());
    for off in [8, 4, 0] {
        g.write::<u32>(rec + off, a.to_bits());
    }
    g.write::<u32>(rec + 0xc, ONE);
    for off in [0x18, 0x14, 0x10] {
        g.write::<u32>(rec + off, b.to_bits());
    }
    g.write::<u32>(rec + 0x1c, ONE);
    g.write::<u32>(rec + 0x28, arg);
    g.write::<u32>(rec + 0x34, 0);
    g.write::<u32>(rec + 0x30, fr.edx);
    g.write::<u32>(rec + 0x2c, fr.edx);
    let node = take(g, img, img.rel32_target(f + 0x75), w(0x25));
    let head = w(0x7b);
    let empty = g.read::<u32>(head) == 0;
    g.write::<u32>(node + 0x14, rec);
    g.write::<u8>(node + 0x18, 1);
    g.write::<u32>(node + 4, 0);
    append_task(g, node, head, w(0x93), empty);
    node
}

/// Appends a pool node to a DLL's task list (`head`/`tail` globals;
/// `empty` as read before the node was filled): first and last of an
/// empty list, else linked after the last; its `+0x8` is itself.
pub(crate) fn append_task<G: Game>(g: &mut G, node: u32, head: u32, tail: u32, empty: bool) {
    if empty {
        g.write::<u32>(node, 0);
        g.write::<u32>(head, node);
        g.write::<u32>(tail, node);
    } else {
        let last = g.read::<u32>(tail);
        g.write::<u32>(node, last);
        g.write::<u32>(last + 4, node);
        g.write::<u32>(tail, node);
    }
    g.write::<u32>(node + 8, node);
}

/// [`fade_task`].
pub const FADE_TASK: Sig = Sig {
    len: 187,
    hash: 0x288b_ec0c_8bfe_c37e,
    calls: &[(0x75, Callee::Sig(&TAKE))],
    imports: &[],
};

/// The time a key track samples at: its own, or through the quartic
/// curve of five control floats `[+0x28]` (`u = K1 - t`, `K2` the
/// middle terms' weight).
#[derive(Clone, Copy)]
enum KeyTime {
    Plain,
    /// The operands of `K1` and `K2`.
    Curve(u32, u32),
}

/// The quartic curve of a key track's time `t`: five control floats at
/// `c`, `u = k1 - t`, the middle terms weighted by `k` (the original's
/// order: `u t u^2 k c1 + u^4 c0`, `+ t^2 u^2 k c2`, `+ t^2 u t k c3`,
/// `+ t^4 c4`).
pub(crate) fn quartic<G: Game>(g: &G, c: u32, t: f32, k1: f32, k: f32) -> f32 {
    let ck = |i: u32| f32_at(g, c + 4 * i);
    let u = sub(k1, t);
    let uu = mul(u, u);
    let ut = mul(u, t);
    let tt = mul(t, t);
    let mut x = add(mul(mul(mul(ut, uu), k), ck(1)), mul(mul(uu, uu), ck(0)));
    x = add(x, mul(mul(mul(tt, uu), k), ck(2)));
    let t4 = mul(tt, tt);
    x = add(x, mul(mul(mul(tt, ut), k), ck(3)));
    add(x, mul(t4, ck(4)))
}

/// One step of a key track (`ecx` = the track: the keys `+0x20`, three
/// floats per key, `n` keys `+0x38`, the time `+0x2c` and its step
/// `+0x30`, the steps left `+0x34`; `edx` = the three floats out). On the
/// last step the last key is copied out and `+0x3c` cleared; otherwise
/// the time moves on (`+0x30 + +0x2c`) and each of the three is
/// [`ease_sample`]d (the `rel32`s at `calls`; stride 3, each call seeing
/// the strides the earlier ones pushed) at the time [`KeyTime`] gives.
/// Returns the last copied word, or 0 for `ease_sample`'s scratch `eax`
/// (the callers load `eax` again at once).
fn key_track<G: Game>(g: &mut G, img: &Image, fr: &Frame, time: KeyTime, calls: [u32; 3]) -> u32 {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let (s, out) = (fr.ecx, fr.edx);
    let left = g.read::<u32>(s + 0x34).wrapping_sub(1);
    g.write::<u32>(s + 0x34, left);
    let keys = g.read::<u32>(s + 0x20);
    if left == 0 {
        let n = g.read::<u32>(s + 0x38);
        let last = keys.wrapping_add(n.wrapping_mul(12)).wrapping_sub(12);
        let mut eax = 0;
        for i in 0..3 {
            eax = g.read::<u32>(last + 4 * i);
            g.write::<u32>(out + 4 * i, eax);
        }
        g.write::<u32>(s + 0x3c, 0);
        return eax;
    }
    let t = add(f32_at(g, s + 0x30), f32_at(g, s + 0x2c));
    let curved = match time {
        KeyTime::Plain => None,
        KeyTime::Curve(k1, k2) => {
            let c = g.read::<u32>(s + 0x28);
            Some(quartic(g, c, t, f32_at(g, w(k1)), f32_at(g, w(k2))))
        }
    };
    g.write::<u32>(s + 0x2c, t.to_bits());
    // push ebx, esi, edi; then a stride pushed per call
    let mut esp = fr.esp.wrapping_sub(12);
    for (i, call) in (0u32..).zip(calls) {
        let n = g.read::<u32>(s + 0x38);
        let t = curved.unwrap_or_else(|| f32_at(g, s + 0x2c));
        esp = esp.wrapping_sub(4);
        g.write::<u32>(esp, 3);
        let target = img.rel32_target(f + call);
        let x = ease_sample(g, img, target, esp.wrapping_sub(4), keys + 4 * i, n, t);
        g.write::<u32>(out + 4 * i, x.to_bits());
    }
    0
}

/// [`key_track`] at the track's own time.
///
/// `magic_0162` 0x10005960 (136 bytes, `ecx`/`edx`; 27 DLLs).
pub fn key_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    key_track(g, img, fr, KeyTime::Plain, [0x26, 0x3c, 0x53])
}

/// [`key_step`].
pub const KEY_STEP: Sig = Sig {
    len: 136,
    hash: 0xb4bb_4e75_52a2_8635,
    calls: &[
        (0x26, Callee::Sig(&EASE_SAMPLE)),
        (0x3c, Callee::Sig(&EASE_SAMPLE)),
        (0x53, Callee::Sig(&EASE_SAMPLE)),
    ],
    imports: &[],
};

/// [`key_track`] through the quartic curve (`K1` operand `+0x21`, `K2`
/// `+0x29`).
///
/// `magic_0162` 0x100059f0 (261 bytes, `ecx`/`edx`; 25 DLLs).
pub fn key_step_curved<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    key_track(g, img, fr, KeyTime::Curve(0x21, 0x29), [0xa7, 0xbb, 0xd0])
}

/// [`key_step_curved`].
pub const KEY_STEP_CURVED: Sig = Sig {
    len: 261,
    hash: 0x038e_6441_3c90_db2e,
    calls: &[
        (0xa7, Callee::Sig(&EASE_SAMPLE)),
        (0xbb, Callee::Sig(&EASE_SAMPLE)),
        (0xd0, Callee::Sig(&EASE_SAMPLE)),
    ],
    imports: &[],
};

/// A value step task of the `magic_0162` family (cdecl `(task)`, its
/// state at `+0x14`: a delay `+0xc`, the time left `+0x10` of `+0x14`,
/// the values `+0x4` (from) and `+0x8` (to), a callback `+0x0`, flags
/// `u16 +0x18`): the delay counts down first; then slot 543 gets `(to *
/// (total - left) + from * left) / total` (`idiv`) and the time left
/// counts down. When none is left the callback runs `(state)`; unless bit
/// 0 of the flags is set, bit 15 is set and the task is given back
/// ([`give_back`], the original's tail jump). Returns the new delay or
/// time left, the flags, or `give_back`'s `eax`.
///
/// `magic_0162` 0x10007c10 (120 bytes; 24 DLLs).
pub fn value_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let task = fr.arg(g, 0);
    let s = g.read::<u32>(task + 0x14);
    let delay = g.read::<i32>(s + 0xc);
    if delay > 0 {
        let d = delay - 1;
        g.write::<i32>(s + 0xc, d);
        return Ret::eax(d.cast_unsigned());
    }
    let total = g.read::<i32>(s + 0x14);
    let left = g.read::<i32>(s + 0x10);
    let n = total
        .wrapping_sub(left)
        .wrapping_mul(g.read::<i32>(s + 8))
        .wrapping_add(g.read::<i32>(s + 4).wrapping_mul(left));
    // idiv: a zero total (or an overflow) faults in the original
    let v = n.checked_div(total).unwrap_or(0);
    let t = slot_target(g, img.word(f + 0x33), 543);
    g.call(t, 0, &[v.cast_unsigned()]);
    let left = g.read::<u32>(s + 0x10);
    if left != 0 {
        g.write::<u32>(s + 0x10, left.wrapping_sub(1));
        return Ret::eax(left.wrapping_sub(1));
    }
    let callback = g.read::<u32>(s);
    if callback != 0 {
        g.call(callback, 0, &[s]);
    }
    let flags = u32::from(g.read::<u16>(s + 0x18));
    if flags & 1 != 0 {
        return Ret::eax(flags);
    }
    g.write::<u16>(s + 0x18, (flags | 0x8000) as u16);
    Ret::eax(give_back(g, img, img.rel32_target(f + 0x6c), task))
}

/// [`value_step`].
pub const VALUE_STEP: Sig = Sig {
    len: 120,
    hash: 0x09bb_76a7_978a_f491,
    calls: &[(0x6c, Callee::Sig(&GIVE_BACK))],
    imports: &[],
};

/// The DLL's task started: the argument and the task's state block
/// (operands from `+0x07` on: the argument, zeroed words, the step
/// function's pointer `+0x40` at `+0x3c`, the flag 1 at `+0x60`, two
/// cleared halfwords) set, a pool node taken for the task function
/// (operand `+0x0e`, [`take`]) with the state block (`+0x7d`) and
/// appended to the task list (`G_HEAD` `+0x75`, `G_TAIL` `+0x8c`).
/// Returns the node.
///
/// `magic_0052` 0x10002670 (180 bytes, one stack word; 22 DLLs).
pub fn task_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    g.write::<u32>(w(0x07), fr.arg(g, 0));
    for op in [0x14, 0x1e, 0x28, 0x32] {
        g.write::<u32>(w(op), 0);
    }
    g.write::<u32>(w(0x3c), w(0x40));
    g.write::<u32>(w(0x46), 0);
    g.write::<u32>(w(0x50), 0);
    g.write::<u16>(w(0x5a), 0);
    g.write::<u32>(w(0x60), 1);
    g.write::<u16>(w(0x6a), 0);
    let node = take(g, img, img.rel32_target(f + 0x6f), w(0x0e));
    let head = w(0x75);
    let empty = g.read::<u32>(head) == 0;
    g.write::<u32>(node + 0x14, w(0x7d));
    g.write::<u32>(node + 4, 0);
    append_task(g, node, head, w(0x8c), empty);
    node
}

/// [`task_start`].
pub const TASK_START: Sig = Sig {
    len: 180,
    hash: 0xbffd_174e_fc53_e357,
    calls: &[(0x6f, Callee::Sig(&TAKE))],
    imports: &[],
};
