//! The summons' target-throw task, a parameter family
//! (`tools/overlay-pfamilies.py`; docs/formats/magic.md, "Parameter
//! families"): 968 bytes in `magic_0411`, `0412`, `0414`, `0425`, `0439`
//! and `0441`, the same code up to parameters. The copies differ in their
//! globals, three float constants (the light words 0.5 or 0.3), the
//! object stride (0x310 or 0x5c0) and its `+0x340`-style offsets, the
//! sound id (0x6a or 0x72), the throw interval and the closing wait, and
//! in the object set-up they call (the one `rel32` callee that differs:
//! a summon object, a model object or one of three thrown effects). One
//! port, [`throws`], reads all of them from the copy's code at the
//! operand offsets below, which are the same in every copy; each copy
//! binds by a [`Sig`] of its own naming its set-up's port ([`copies!`]).
//! `magic_0411`'s copy waits on its set-up (0x10006d30, open).

use crate::o4_parts::disp8;
use crate::runtime::slot_target;
use crate::sa_summon::st0_f32;
use crate::sb_sse::{cvt, div, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// An object set-up the task calls (internal: `ecx` the object, `edx` the
/// target's record, `xmm2` a size).
pub type Setup<G> = fn(&mut G, &Image, &Frame) -> u32;

/// `[s + i*4 + 0x1c] = [s + i*4 + 0x48] / [s + 0x44]` (both converted
/// from integers) for every target `i` below `*G_N` (operand `n_at`,
/// read again each round; `count` the count read before). Returns the
/// last `i`'s address `+4` (`eax` of the loop), or `None` with no target.
fn shares<G: Game>(g: &mut G, s: u32, count: u32, n_at: u32, per_round: bool) -> Option<u32> {
    if count.cast_signed() <= 0 {
        return None;
    }
    let total = cvt(g.read::<u32>(s + 0x44));
    let mut i = 0u32;
    loop {
        let p = s + 0x1c + 4 * i;
        let total = if per_round {
            cvt(g.read::<u32>(s + 0x44))
        } else {
            total
        };
        let x = div(cvt(g.read::<u32>(p + 0x2c)), total);
        g.write::<f32>(p, x);
        i += 1;
        if i.cast_signed() >= g.read::<u32>(n_at).cast_signed() {
            return Some(p + 4);
        }
    }
}

/// The task (`(task)`, cdecl; the state record `S = task[+4]`, `ebp` the
/// entry `esp - 4`). By the state `S + 0x18`:
/// - 0: the target nodes ([`crate::s4_summon::target_nodes`], a record
///   at `ebp - 0x1c`: two node words, `*G_A`, 0, the byte 1; `*G_V` set
///   first), the colour node ([`crate::s3_misc::colour_node`], a record
///   at `ebp - 8`), seven light words (operand `+0x93`, value `+0x97`,
///   10 bytes apart) and the camera move ([`crate::sb_misc::move_start`],
///   time `[+0x8d]`); then the state 10 and on as 10;
/// - 10: `S + 0x44` = 0; for each target `i` below `*G_N` (`+0xeb`),
///   slot 291 `(*G_P, rec[i].0, &arg, &arg, &arg)` (the records `+0xf6`,
///   0x98 apart; the argument word gets written) counts it at
///   `S + 0x48 + 4i` and into `S + 0x44`; the shares ([`shares`]); six objects'
///   flags (displacements `+0x16f`, 10 bytes apart) 0xff; the state 11,
///   the wait `S + 0x3c` 0; on as 11;
/// - 11: when the wait runs out and the object `S.44 % 5` (stride
///   `+0x1d3`, flag displacement `+0x1da`) is free (0xff): a target drawn
///   by the shares from slot 307's ST0 less `K` (`+0x206`), the object
///   set up (the copy's set-up: `ecx` `S + 0x70 + object`, `edx` the
///   target's record, `xmm2` `(r - K) K1 + K2`, operands `+0x244`,
///   `+0x255`, `+0x25e`, `+0x266`) and counted down in `S + 0x44`; the
///   last one plays slot 228 `(*G_P, id)` (`+0x27c`) and moves to the
///   state 12 (the frame count 0); the others take one from the target's
///   count, wait `+0x2ad` frames and share again;
/// - 12: past the frame count `+0x35a`, the light words 1.0 and the
///   camera's way back ([`crate::sb_misc::move_start_2`], times `+0x369`
///   and `+0x361`); the state 10000;
/// - 10000: once slot 411 `(*G_P)` reports it done, slot 213 `(*G_P, 0)`
///   and the task leaves its list.
///
/// Every exit counts the frame `S + 0x40` up. Returns `eax` as the
/// original leaves it.
#[allow(clippy::too_many_lines)] // the original's states, one by one
pub fn throws<G: Game>(g: &mut G, img: &Image, fr: &Frame, setup: Setup<G>) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let table = w(0x10a);
    let task = fr.arg(g, 0);
    let arg = fr.esp + 4;
    let ebp = fr.esp.wrapping_sub(4);
    // the callees' return address: below `ebp - 0x1c` and esi, edi
    let call_esp = ebp.wrapping_sub(0x28);
    let s = g.read::<u32>(task + 4);
    let out = |g: &mut G, eax: u32| {
        let n = g.read::<u32>(s + 0x40).wrapping_add(1);
        g.write::<u32>(s + 0x40, n);
        Ret::eax(eax)
    };
    let lights = |g: &mut G, first: u32| {
        for i in 0..7 {
            let o = first + 10 * i;
            g.write::<u32>(w(o), w(o + 4));
        }
    };
    let state = g.read::<u32>(s + 0x18);
    match state {
        0 | 10 | 11 => {}
        12 => {
            if g.read::<u32>(s + 0x40).cast_signed() < disp8(img, f + 0x35a).cast_signed() {
                return out(g, state);
            }
            let t3 = g.read::<f32>(w(0x361));
            let t2 = g.read::<f32>(w(0x369));
            lights(g, 0x36f);
            let eax = crate::sb_misc::move_start_2(g, img, img.rel32_target(f + 0x3b4), t2, t3);
            g.write::<u32>(s + 0x18, w(0x3bb));
            return out(g, eax);
        }
        0x2710 => {
            let t = slot_target(g, table, 411);
            if g.call(t, 0, &[g.read::<u32>(w(0x305))]).eax == 0 {
                return out(g, 0);
            }
            let t = slot_target(g, table, 213);
            g.call(t, 0, &[g.read::<u32>(w(0x325)), 0]);
            let node = g.read::<u32>(task);
            let prev = g.read::<u32>(node + 4);
            let next = g.read::<u32>(node + 8);
            g.write::<u32>(prev + 8, next);
            let next = g.read::<u32>(node + 8);
            let eax = g.read::<u32>(node + 4);
            g.write::<u32>(next + 4, eax);
            g.write::<u32>(node + 8, node);
            g.write::<u32>(node + 4, node);
            return out(g, eax);
        }
        _ => return out(g, state),
    }
    if state == 0 {
        let a = g.read::<u32>(w(0x37));
        let rec = ebp.wrapping_sub(0x1c);
        g.write::<u32>(rec, w(0x41));
        g.write::<u32>(rec + 4, w(0x48));
        g.write::<u32>(rec + 8, a);
        g.write::<u32>(rec + 0xc, 0);
        g.write::<u8>(rec + 0x10, 1);
        g.write::<u32>(w(0x5c), w(0x60));
        let callee = Frame {
            at: img.rel32_target(f + 0x65),
            esp: call_esp,
            ecx: rec,
            ..Frame::default()
        };
        crate::s4_summon::target_nodes(g, img, &callee);
        let rec = ebp.wrapping_sub(8);
        g.write::<u32>(rec, w(0x6f));
        g.write::<u32>(rec + 4, w(0x76));
        g.write::<u32>(w(0x7c), w(0x80));
        let callee = Frame {
            at: img.rel32_target(f + 0x85),
            esp: call_esp,
            ecx: rec,
            ..Frame::default()
        };
        let eax = crate::s3_misc::colour_node(g, img, &callee);
        let t = g.read::<f32>(w(0x8d));
        lights(g, 0x93);
        crate::sb_misc::move_start(g, img, img.rel32_target(f + 0xd8), t, eax);
        g.write::<u32>(s + 0x18, w(0xdf));
    }
    if state != 11 {
        // state 10: count the targets
        g.write::<u32>(s + 0x44, 0);
        let mut n = g.read::<u32>(w(0xeb));
        if n.cast_signed() > 0 {
            let mut rec = w(0xf6);
            g.write::<u32>(ebp.wrapping_sub(4), rec);
            let mut i = 0u32;
            loop {
                let t = slot_target(g, table, 291);
                let first = g.read::<u32>(rec);
                // call width: 5
                let got = g
                    .call(t, rec, &[g.read::<u32>(w(0x110)), first, arg, arg, arg])
                    .eax;
                g.write::<u32>(s + 0x48 + 4 * i, got);
                let total = g.read::<u32>(s + 0x44).wrapping_add(got);
                g.write::<u32>(s + 0x44, total);
                n = g.read::<u32>(w(0x125));
                i += 1;
                rec = rec.wrapping_add(0x98);
                g.write::<u32>(ebp.wrapping_sub(4), rec);
                if i.cast_signed() >= n.cast_signed() {
                    break;
                }
            }
        }
        shares(g, s, n, w(0x167), false);
        for k in 0..6 {
            g.write::<u32>(s.wrapping_add(w(0x16f + 10 * k)), w(0x173 + 10 * k));
        }
        let st = g.read::<u32>(s + 0x18).wrapping_add(1);
        g.write::<u32>(s + 0x18, st);
        g.write::<u32>(s + 0x3c, 0);
    }
    // state 11
    let wait = g.read::<u32>(s + 0x3c);
    g.write::<u32>(s + 0x3c, wait.wrapping_sub(1));
    if wait.cast_signed() > 0 {
        return out(g, wait.wrapping_sub(1));
    }
    let left = g.read::<u32>(s + 0x44).cast_signed();
    let (q, r) = (left.wrapping_div(5), left.wrapping_rem(5));
    let obj = r.wrapping_mul(w(0x1d3).cast_signed()).cast_unsigned();
    if g.read::<u32>(obj.wrapping_add(s).wrapping_add(w(0x1da))) != 0xff {
        return out(g, q.cast_unsigned());
    }
    let t = slot_target(g, table, 307);
    let r1 = st0_f32(g.call(t, 0, &[]));
    g.write::<u32>(arg, r1);
    let n = g.read::<u32>(w(0x1f9)).wrapping_sub(1);
    let mut x0 = sub(f32::from_bits(r1), g.read::<f32>(w(0x206)));
    let mut x1 = 0f32;
    let mut i = 0u32;
    g.write::<u32>(arg, 0);
    if n.cast_signed() > 0 {
        loop {
            x1 = g.read::<f32>(s + 4 * i + 0x1c);
            if x1 > x0 {
                break;
            }
            i += 1;
            x0 = sub(x0, x1);
            g.write::<u32>(arg, i);
            if i.cast_signed() >= n.cast_signed() {
                break;
            }
        }
    }
    let t = slot_target(g, table, 307);
    let r2 = st0_f32(g.call(t, 0, &[]));
    g.write::<u32>(ebp.wrapping_sub(4), r2);
    let i = g.read::<u32>(arg);
    let size = sub(f32::from_bits(r2), g.read::<f32>(w(0x244)));
    let size = crate::sb_sse::mul(size, g.read::<f32>(w(0x255)));
    let size = crate::sb_sse::add(size, g.read::<f32>(w(0x266)));
    let callee = Frame {
        at: img.rel32_target(f + 0x26d),
        esp: call_esp,
        ecx: (s + 0x70).wrapping_add(obj),
        edx: i.wrapping_mul(0x98).wrapping_add(w(0x25e)),
        xmm: [
            u128::from(x0.to_bits()),
            u128::from(x1.to_bits()),
            u128::from(size.to_bits()),
            0,
        ],
        ..Frame::default()
    };
    setup(g, img, &callee);
    let left = g.read::<u32>(s + 0x44).wrapping_sub(1);
    g.write::<u32>(s + 0x44, left);
    if left == 0 {
        let t = slot_target(g, table, 228);
        let id = disp8(img, f + 0x27c);
        let eax = g.call(t, 0, &[g.read::<u32>(w(0x27f)), id]).eax;
        let st = g.read::<u32>(s + 0x18).wrapping_add(1);
        g.write::<u32>(s + 0x18, st);
        g.write::<u32>(s + 0x40, 0);
        return out(g, eax);
    }
    let i = g.read::<u32>(arg);
    let at = s.wrapping_add(i.wrapping_mul(4)).wrapping_add(0x48);
    let c = g.read::<u32>(at).wrapping_sub(1);
    g.write::<u32>(at, c);
    g.write::<u32>(s + 0x3c, w(0x2ad));
    let n = g.read::<u32>(w(0x2b3));
    if n.cast_signed() <= 0 {
        return out(g, i);
    }
    let end = shares(g, s, n, w(0x2df), true).unwrap_or(i);
    out(g, end)
}

/// One entry and one [`Sig`] per copy: `name => SIG, hash, set-up port,
/// its Sig`. The other callees are the same ports in every copy.
macro_rules! copies {
    ($($(#[$doc:meta])* $f:ident => $sig:ident, $hash:literal, $setup:path, $ssig:path;)*) => {
        $(
            $(#[$doc])*
            pub fn $f<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
                throws(g, img, fr, $setup)
            }

            #[doc = concat!("[`", stringify!($f), "`].")]
            pub const $sig: Sig = Sig {
                len: 968,
                hash: $hash,
                calls: &[
                    (0x65, Callee::Sig(&crate::s4_summon::TARGET_NODES)),
                    (0x85, Callee::Sig(&crate::s3_misc::COLOUR_NODE)),
                    (0xd8, Callee::Sig(&crate::sb_misc::MOVE_START)),
                    (0x26d, Callee::Sig(&$ssig)),
                    (0x3b4, Callee::Sig(&crate::sb_misc::MOVE_START_2)),
                ],
                imports: &[],
            };
        )*
    };
}

copies! {
    /// `magic_0412` 0x10006d90.
    throws_0412 => THROWS_0412, 0xe88d_7a7a_2ac6_97d3,
        crate::s7_misc::summon_object_setup, crate::s7_misc::SUMMON_OBJECT_SETUP;
    /// `magic_0414` 0x10006e40.
    throws_0414 => THROWS_0414, 0xe88d_7a7a_2ac6_97d3,
        crate::s7_misc::model_object_setup, crate::s7_misc::MODEL_OBJECT_SETUP;
    /// `magic_0425` 0x10006db0.
    throws_0425 => THROWS_0425, 0x0ad7_1f6b_8d3f_7822,
        crate::s6_misc::thrown_effect_plain, crate::s6_misc::THROWN_EFFECT_PLAIN;
    /// `magic_0439` 0x100070d0.
    throws_0439 => THROWS_0439, 0x95f6_82ea_db68_015d,
        crate::s5_misc::thrown_effect_far, crate::s5_misc::THROWN_EFFECT_FAR;
    /// `magic_0441` 0x10006e30.
    throws_0441 => THROWS_0441, 0x0642_cefc_0b69_a62f,
        crate::s5_misc::thrown_effect, crate::s5_misc::THROWN_EFFECT;
}
