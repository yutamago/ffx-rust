//! `magic_0705.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::s4_summon::trunc;
use crate::sb_sse::{cvt, div, mul, sub};
use crate::{Frame, Game, Image, Sig};

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
