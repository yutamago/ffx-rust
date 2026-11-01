//! `magic_0705.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::pushes::Pushes;
use crate::runtime::slot_target;
use crate::s4_summon::trunc;
use crate::sb_sse::{cvt, div, mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `1.0f32`'s bits.
const ONE: u32 = 0x3f80_0000;

/// Sets the fade for step `i` (0x100011b0, internal: `ecx` = `i`):
/// nothing from step 31 (returns the caller's `eax`). The level byte is
/// `i << 7` (its low byte) up to step 1, else `((31 - i) << 7) / 30`;
/// slot 776 `(0x44)` gives a handle `h`, slot 915 `(0x3fff0000 three
/// times, level * K, h)` (`K` at operand `+0x5b`). Returns slot 915's
/// `eax`.
pub fn fade<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let w = |off: u32| img.word(fr.at + off);
    let i = fr.ecx.cast_signed();
    if i >= 0x1f {
        return fr.eax;
    }
    let level = if i <= 1 {
        (fr.ecx << 7) & 0xff
    } else {
        (((0x1f - i) << 7) / 30).cast_unsigned() & 0xff
    };
    let t = slot_target(g, w(0x30), 776);
    let h = g.call(t, 0, &[0x44]).eax;
    let x = mul(cvt(level), g.read::<f32>(w(0x5b)));
    let t = slot_target(g, w(0x4a), 915);
    let c = 0x3fff_0000;
    g.call(t, 0, &[c, c, c, x.to_bits(), h]).eax
}

/// [`fade`] (134 bytes, 1 overlay).
pub const FADE: Sig = Sig {
    len: 134,
    hash: 0xfae8_142e_2320_153b,
    calls: &[],
    imports: &[],
};

/// Tints the screen by the time (0x100010d0, internal, cdecl with the
/// colour in the stack words 3..6; a frame with a stack cookie): with
/// `t = *G_T` (operand `+0x11`) the strength `k` is 0 outside 1..330,
/// `t / D` up to 60 (`D` at `+0x38`), 1 (`+0x19`) up to 270, else `1 -
/// (t - 270) / D` (`+0x58`); slot 353 gets the vector `1 - k * colour`
/// at `ebp - 0x14`. Returns slot 353's `eax`.
pub fn tint<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let w = |off: u32| img.word(fr.at + off);
    let t = g.read::<u32>(w(0x11)).cast_signed();
    let one = g.read::<f32>(w(0x19));
    let k = if t <= 0 || t >= 0x14a {
        0.0
    } else if t <= 0x3c {
        div(cvt(t.cast_unsigned()), g.read::<f32>(w(0x38)))
    } else if t <= 0x10e {
        one
    } else {
        sub(
            one,
            div(cvt((t - 0x10e).cast_unsigned()), g.read::<f32>(w(0x58))),
        )
    };
    let v = fr.esp.wrapping_sub(4 + 0x14);
    for i in 0..4 {
        let c = f32::from_bits(fr.arg(g, 3 + i));
        g.write::<f32>(v + 4 * i, sub(one, mul(k, c)));
    }
    let s = slot_target(g, w(0x81), 353);
    g.call(s, 0, &[v]).eax
}

/// [`tint`] (209 bytes, 1 overlay).
pub const TINT: Sig = Sig {
    len: 209,
    hash: 0xe0b6_f7f3_bf36_d8dc,
    // __security_check_cookie
    calls: &[(0xc9, crate::Callee::Any)],
    imports: &[],
};

/// `lahf` after `ucomiss a, b`: `ZF`, `PF`, `CF` from the compare, bit 1
/// set.
fn lahf(a: f32, b: f32) -> u32 {
    match a.partial_cmp(&b) {
        None => 0x47,
        Some(core::cmp::Ordering::Less) => 0x03,
        Some(core::cmp::Ordering::Equal) => 0x42,
        Some(core::cmp::Ordering::Greater) => 0x02,
    }
}

/// Sets the glow for the time (0x10001240, internal, no arguments; `t =
/// *G_T`, operand `+0x01`): nothing outside 1..120. The level is `t` up
/// to 1, 1 up to 100, else `1 - (t - 100) / D` (`+0x42`; at exactly 0
/// nothing more); `n` = the level times `K` (`+0x65`) truncated, and
/// when positive slot 821 `(0)`, slot 822 `(1, 1, 1, n * S)` (`+0x9e`),
/// slot 823 `(1.0078125, 1.0096...)` and slot 824 `(0.5, 0.5, 0, 0)`
/// (the original rewrites one stack area for them). Returns slot 824's
/// `eax`, else `t` (with the compare's `lahf` byte in bits 8..15 when
/// the level was compared with 0).
pub fn glow<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let w = |off: u32| img.word(fr.at + off);
    let t = g.read::<u32>(w(0x1));
    let ti = t.cast_signed();
    if ti <= 0 || ti >= 0x78 {
        return t;
    }
    let mut eax = t;
    let x = if ti > 1 && ti <= 0x64 {
        g.read::<f32>(w(0x5d))
    } else {
        let x = if ti <= 1 {
            cvt(t)
        } else {
            let one = g.read::<f32>(w(0x1a));
            sub(one, div(cvt(t.wrapping_sub(100)), g.read::<f32>(w(0x42))))
        };
        let ah = lahf(x, 0.0);
        eax = eax & !0xff00 | ah << 8;
        if ah == 0x42 {
            return eax;
        }
        x
    };
    let n = trunc(mul(x, g.read::<f32>(w(0x65))));
    if n.cast_signed() <= 0 {
        return eax;
    }
    let t = slot_target(g, w(0x77), 821);
    g.call(t, 0, &[0]);
    let v = mul(cvt(n), g.read::<f32>(w(0x9e)));
    let t = slot_target(g, w(0x86), 822);
    g.call(t, 0, &[ONE, ONE, ONE, v.to_bits()]);
    let t = slot_target(g, w(0xc2), 823);
    g.call(t, 0, &[0x3f81_0000, 0x3f81_3b14]);
    let t = slot_target(g, w(0xe1), 824);
    g.call(t, 0, &[0x3f00_0000, 0x3f00_0000, 0, 0]).eax
}

/// [`glow`] (275 bytes, 1 overlay).
pub const GLOW: Sig = Sig {
    len: 275,
    hash: 0xed3f_ebde_3b07_fe88,
    calls: &[],
    imports: &[],
};

/// `fstp dword` of a slot's ST0: the `f32` it rounds to (an empty x87
/// stack stores the indefinite NaN).
fn st0_f32(o: crate::Out) -> f32 {
    o.st0.map_or(f32::from_bits(0xffc0_0000), |v| v as f32)
}

/// The effect's start (0x100014d0, an overlay table entry, cdecl
/// `(arg)`): `arg` into `G_ARG` (operand `+0x8`); the actor = slot 168
/// `(arg)`; its mode byte through slot 218 `(*G_ARG, &b)` (the local at
/// `ebp - 4`); slot 215 the actor's and the mode's models. Unless the
/// actor is 0xff, its model (slot 215), its shade `trunc(slot 169 (id) *
/// K / D)` (ST0 stored into the argument's word as `f32` first) and slot
/// 217 `(id, RECORD)`. Then the targets: slot 218 `(*G_ARG, &n)` gives
/// `n` ids (bytes); each gets a record of 0x28 bytes (from the operand
/// `+0x134` minus 0x18: the id, its model, its shade the same way and
/// slot 217 `(id, rec + 0x18)` unless it is 0xff), and the centre sums
/// `rec[+0x18]`, slot 289 `(id)` (ST0) and `rec[+0x20]`; the sums are
/// divided by `n` (the centre's `w` 1.0). Four state words are reset, the
/// part handles taken
/// ([`part_handles`](crate::s4_small::part_handles)), slot 711 `(BLOCK,
/// LIST, 0)` with `**G_LOCK` 1 around it, slot 710 `(ENV, BLOCK, FLAG,
/// 0x100000)` and slot 271 `(4)`. Each call sees the words the earlier
/// ones left. Returns 0.
#[allow(clippy::too_many_lines)] // the start as the original orders it
pub fn start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::add;
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let r = |g: &G, off: u32| g.read::<u32>(w(off));
    let k = |g: &G, off: u32| f32::from_bits(r(g, off));
    let call = |g: &mut G, p: &Pushes, slot: u32| {
        let t = slot_target(g, w(0xe), slot);
        // call width: 12 (Pushes caps the words at 12)
        p.call(g, t, 0)
    };
    let arg_word = fr.esp + 4;
    let arg = fr.arg(g, 0);
    g.write::<u32>(w(0x8), arg);
    let mut p = Pushes::default();
    p.push(arg);
    let actor = call(g, &p, 168).eax;
    g.write::<u32>(w(0x1b), actor);
    p.push_all(&[fr.esp.wrapping_sub(8), r(g, 0x2a)]);
    let at = call(g, &p, 218).eax;
    let mode = u32::from(g.read::<u8>(at));
    p.push(r(g, 0x3b));
    g.write::<u32>(w(0x40), mode);
    let m = call(g, &p, 215).eax;
    p.push(r(g, 0x53));
    g.write::<u32>(w(0x58), m);
    let m = call(g, &p, 215).eax;
    g.write::<u32>(w(0x6a), m);
    let c = r(g, 0x6f);
    g.write::<u32>(w(0x77), c);
    if c != 0xff {
        let mut p = Pushes::default();
        p.push(c);
        let m = call(g, &p, 215).eax;
        p.push(r(g, 0x92));
        g.write::<u32>(w(0x97), m);
        let x = st0_f32(call(g, &p, 169));
        g.write::<u32>(arg_word, x.to_bits());
        let y = mul(x, k(g, 0xb4));
        p.push_all(&[w(0xb9), r(g, 0xbf)]);
        let y = div(y, k(g, 0xc7));
        g.write::<u16>(w(0xd1), trunc(y) as u16);
        call(g, &p, 217);
    }
    let mut p = Pushes::default();
    p.push_all(&[w(0xee), r(g, 0xf4)]);
    for off in [0xfa, 0x104, 0x10e] {
        g.write::<u32>(w(off), 0);
    }
    let list = call(g, &p, 218).eax;
    let mut n = r(g, 0x120);
    let (s0, s1, s2) = (w(0x1ab), w(0x1ce), w(0x1e0));
    if n.cast_signed() > 0 {
        let mut rec = w(0x134);
        let mut i = 0u32;
        loop {
            let id = u32::from(g.read::<u8>(list.wrapping_add(i)));
            g.write::<u32>(rec.wrapping_sub(0x18), id);
            if id != 0xff {
                let mut p = Pushes::default();
                p.push(id);
                let m = call(g, &p, 215).eax;
                p.push(g.read::<u32>(rec.wrapping_sub(0x18)));
                g.write::<u32>(rec.wrapping_sub(0x14), m);
                let x = st0_f32(call(g, &p, 169));
                g.write::<u32>(arg_word, x.to_bits());
                let y = mul(x, k(g, 0x17b));
                p.push_all(&[rec, g.read::<u32>(rec.wrapping_sub(0x18))]);
                let y = div(y, k(g, 0x187));
                g.write::<u16>(rec.wrapping_sub(4), trunc(y) as u16);
                call(g, &p, 217);
            }
            let fl = |g: &G, at: u32| f32::from_bits(g.read::<u32>(at));
            let sx = add(fl(g, rec), fl(g, s0));
            let mut p = Pushes::default();
            p.push(g.read::<u32>(rec.wrapping_sub(0x18)));
            g.write::<u32>(s0, sx.to_bits());
            let z = st0_f32(call(g, &p, 289));
            g.write::<u32>(arg_word, z.to_bits());
            let sy = add(fl(g, s1), z);
            let sz = add(fl(g, rec + 8), fl(g, s2));
            n = r(g, 0x1e6);
            i += 1;
            rec = rec.wrapping_add(0x28);
            g.write::<u32>(s1, sy.to_bits());
            g.write::<u32>(s2, sz.to_bits());
            if i.cast_signed() >= n.cast_signed() {
                break;
            }
        }
    }
    let count = cvt(n);
    g.write::<u32>(w(0x22c), 0x3f80_0000);
    for (src, dst) in [(0x21fu32, 0x244u32), (0x217, 0x24c), (0x20f, 0x254)] {
        let v = div(k(g, src), count);
        g.write::<u32>(w(dst), v.to_bits());
    }
    g.write::<u32>(w(0x25a), 0);
    g.write::<u32>(w(0x264), 0);
    g.write::<u32>(w(0x26e), 0xffff_ffff);
    g.write::<u32>(w(0x278), 0);
    // push ebp, push ecx, push ebx/esi/edi, the return address
    let callee = Frame {
        at: img.rel32_target(f + 0x281),
        esp: fr.esp.wrapping_sub(0x18),
        eax: 0,
        ecx: 0,
        edx: 0,
        xmm: [0; 4],
    };
    crate::s4_small::part_handles(g, img, &callee);
    let lockp = r(g, 0x286);
    let mut p = Pushes::default();
    p.push(0);
    let saved = g.read::<u32>(lockp);
    g.write::<u32>(lockp, 1);
    p.push_all(&[w(0x29a), w(0x2a5)]);
    call(g, &p, 711);
    let lockp = r(g, 0x2ac);
    p.push(0x10_0000);
    g.write::<u32>(lockp, saved);
    p.push_all(&[w(0x2bd), w(0x2c8), w(0x2cd)]);
    call(g, &p, 710);
    p.push(4);
    call(g, &p, 271);
    Ret::eax(0)
}

/// [`start`] (750 bytes, 1 overlay).
pub const START: Sig = Sig {
    len: 750,
    hash: 0xc133_f793_7faa_eb97,
    calls: &[(0x281, Callee::Sig(&crate::s4_small::PART_HANDLES))],
    imports: &[],
};
