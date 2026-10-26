//! The character-lighting tweak utility's line texts
//! (`CharacterLightingTweakUtil::getLineText`, h2p4 round).
//!
//! Layout (`g_CharacterLightingTweakUtil`, see [`crate::p4_light`]): `+0`
//! on, `+0x94 + 4 k` the eleven line names, `+0xbc` the auto-reset line's
//! name, `+0xc0` the light index shown, `+0xc4` the selected line, `+0xc9`
//! the auto-reset flag.

use ffx_bridge::x87;
use ffx_graphic::host::{Conv, Host};

use crate::methods::quiet;

/// `MSVCR110!sprintf_s`'s import slot (cdecl `(out, n, format, ...)`).
pub const SPRINTF_S: u32 = 0x00b0_c394;
/// `g_CharacterLighting`.
pub const G_LIGHTING: u32 = 0x00cc_ca84;
/// `CharacterLighting_hasIntensity_006a9ae0` (thiscall `(cl, i)` -> `al`).
pub const HAS_INTENSITY: u32 = 0x006a_9ae0;
/// The CRT's `_CIasin`, `_CIsqrt` and `_CIacos` import thunks (`ST0` in
/// and out).
pub const CI_ASIN: u32 = 0x0094_98f2;
pub const CI_SQRT: u32 = 0x0094_97b8;
pub const CI_ACOS: u32 = 0x0094_98da;
/// The formats: `"init failed"`, `"%s %s: %d"`, `"%s %s: %s"`, `"%s %s:
/// %.3f"`, `"%s %s: %.3f %.3f %.1f"`, `"%s %s: %d, %d, %d, %d, %d"`.
pub const F_FAILED: u32 = 0x00b4_8244;
pub const F_INT: u32 = 0x00b4_8254;
pub const F_STR: u32 = 0x00b4_8270;
pub const F_FLOAT: u32 = 0x00b4_827c;
pub const F_COLOUR: u32 = 0x00b4_8288;
pub const F_INTS: u32 = 0x00b4_82a0;
/// The selection marks (`">"` and `" "`) and the flag words (`"True"`,
/// `"False"`).
pub const MARK_ON: u32 = 0x00b4_8250;
pub const MARK_OFF: u32 = 0x00b2_0b30;
pub const TRUE: u32 = 0x00b4_8260;
pub const FALSE: u32 = 0x00b4_8268;

/// `double` constants: 255, -1, 57.3248 (radians to degrees), 0.0001,
/// 360, 0.5; `f32` 0.001 and -90.
const D_255: u32 = 0x00b1_70c0;
const D_MINUS_ONE: u32 = 0x00b9_2350;
const D_DEGREES: u32 = 0x00b4_8368;
const D_TINY: u32 = 0x00b3_eb50;
const D_360: u32 = 0x00b4_8380;
const D_HALF: u32 = 0x00b9_2220;
const F_EPS: u32 = 0x00b3_68a4;
const F_MINUS_90: u32 = 0x00b4_8394;

fn f(bits: u32) -> f64 {
    x87::load(bits)
}

fn double(h: &impl Host, at: u32) -> f64 {
    let at = h.rebase(at);
    let lo: u32 = h.read(at);
    let hi: u32 = h.read(at + 4);
    f64::from_bits(u64::from(hi) << 32 | u64::from(lo))
}

fn less(a: f64, b: f64) -> bool {
    a.partial_cmp(&b) == Some(std::cmp::Ordering::Less)
}

/// A `double` vararg's two words.
fn words(x: f64) -> [u32; 2] {
    let b = x.to_bits();
    [b as u32, (b >> 32) as u32]
}

pub mod bridge {
    //! The original over game memory.

    use super::{
        CI_ACOS, CI_ASIN, CI_SQRT, Conv, D_255, D_360, D_DEGREES, D_HALF, D_MINUS_ONE, D_TINY,
        F_COLOUR, F_EPS, F_FAILED, F_FLOAT, F_INT, F_INTS, F_MINUS_90, F_STR, FALSE, G_LIGHTING,
        HAS_INTENSITY, Host, MARK_OFF, MARK_ON, SPRINTF_S, TRUE, double, f, less, quiet, words,
        x87,
    };

    /// `sprintf_s(out, 0x80, format, rest...)` through its import slot.
    fn print(h: &mut impl Host, out: u32, format: u32, rest: &[u32]) -> u32 {
        let fun: u32 = h.read(h.rebase(SPRINTF_S));
        let mut args = vec![out, 0x80, h.rebase(format)];
        args.extend_from_slice(rest);
        h.call(fun, Conv::Cdecl, &args)
    }

    /// `_ftol2_sse` of `x`.
    fn ftol(h: &impl Host, x: f64) -> u32 {
        x87::ftol(h, x)
    }

    /// A CRT intrinsic of `x` (an `f32`), stored as `f32`.
    fn intrinsic(h: &mut impl Host, at: u32, x: u32) -> u32 {
        let at = h.rebase(at);
        h.call_intrinsic(at, f(x)).to_bits()
    }

    /// `CharacterLightingTweakUtil::getLineText(tw, line, out)`
    /// (`0x006a8580`, `ret 8`): `out` (0x80 bytes, `sprintf_s`) gets
    /// `"init failed"` while the utility is off, else line `line`'s text,
    /// `">"` or `" "` (the selected line `+0xc4`) and its name first:
    ///
    /// - 0: the light index (`+0xc0`); 1: whether that light has an
    ///   intensity (`"True"`/`"False"`; `"False"` without a light set);
    /// - 2, 3, 4: its intensity, pitch, yaw (`cl + 0x1a8/0x1ac/0x1b0 + 0xc
    ///   i`; light 3: the ambient intensity `+0x1a0`, 0, 0), as a double
    ///   of its `f32`, also left in the argument slot;
    /// - 5: the scene's first three words (`[cl + 0x18c]`; zeros without a
    ///   scene) as doubles;
    /// - 6, 7, 8: light `line - 6`'s colour times 255 (each rounded to
    ///   `f32`, plus 0.5, rounded, truncated), its direction's pitch
    ///   (`asin(-y)` in degrees) and yaw: from the horizontal length `s =
    ///   sqrt(1 - y^2)` (0 when it is below 0.0001), `A = -90` when `|x -
    ///   s| < 0.001` else `asin(-x / s)`, `B = acos(z / s)` (degrees), the
    ///   yaw `B + 360` (`A > 0`, `B < 0`), `|A|` (both negative), `360 -
    ///   B` (`A < 0`, `B > 0`) or `B`, plus 0.5, truncated;
    /// - 9: the ambient colour times 255 plus 0.5, truncated in x87
    ///   precision, and two zeros;
    /// - 10: the auto-reset flag (`+0xc9`).
    ///
    /// Lines above 10 write nothing. Returns `sprintf_s`'s `eax` (the
    /// line when nothing was printed) and the argument slot's word
    /// (`None`: the line).
    #[allow(clippy::too_many_lines)]
    pub fn get_line_text(h: &mut impl Host, tw: u32, line: u32, out: u32) -> (u32, Option<u32>) {
        if h.read::<u8>(tw) == 0 {
            return (print(h, out, F_FAILED, &[]), None);
        }
        let mark = h.rebase(if h.read::<u32>(tw + 0xc4) == line {
            MARK_ON
        } else {
            MARK_OFF
        });
        let cl: u32 = h.read(h.rebase(G_LIGHTING));
        let scene: u32 = h.read(cl.wrapping_add(0x18c));
        let index: u32 = h.read(tw + 0xc0);
        let at = cl.wrapping_add(index.wrapping_mul(0xc));
        match line {
            0 => {
                let name: u32 = h.read(tw + 0x94);
                (print(h, out, F_INT, &[mark, name, index]), None)
            }
            1 => {
                let has = h.read::<u32>(cl.wrapping_add(0xf0)) != 0
                    && h.call_sym(HAS_INTENSITY, Conv::Thiscall(cl), &[index]) & 0xff != 0;
                let s = h.rebase(if has { TRUE } else { FALSE });
                let name: u32 = h.read(tw + 0x98);
                (print(h, out, F_STR, &[mark, name, s]), None)
            }
            2..=4 => {
                let k = line - 2;
                let v = if index == 3 {
                    if k == 0 {
                        quiet(h.read(cl.wrapping_add(0x1a0)))
                    } else {
                        0
                    }
                } else {
                    quiet(h.read(at.wrapping_add(0x1a8 + 4 * k)))
                };
                let [lo, hi] = words(f(v));
                let name: u32 = h.read(tw + 0x9c + 4 * k);
                (print(h, out, F_FLOAT, &[mark, name, lo, hi]), Some(v))
            }
            5 => {
                let xyz: [u32; 3] = if scene == 0 {
                    [0; 3]
                } else {
                    [0, 4, 8].map(|o| h.read(scene + o))
                };
                let [a, b, c] = xyz.map(|w| words(f(w)));
                let name: u32 = h.read(tw + 0xa8);
                let args = [mark, name, a[0], a[1], b[0], b[1], c[0], c[1]];
                (print(h, out, F_COLOUR, &args), None)
            }
            6..=8 => {
                let base = cl.wrapping_add((line - 6) * 0x10);
                let k255 = double(h, D_255);
                let half = double(h, D_HALF);
                let c: [u32; 3] = [0x124, 0x128, 0x12c].map(|o| h.read(base.wrapping_add(o)));
                let r = x87::store(x87::mul(f(c[0]), k255));
                let g = x87::store(x87::mul(f(c[1]), k255));
                let b = x87::store(x87::mul(k255, f(c[2])));
                let r = x87::store(x87::add(f(r), half));
                let g = x87::store(x87::add(f(g), half));
                let b = x87::store(x87::add(half, f(b)));
                let rgb = [r, g, b].map(|w| ftol(h, f(w)));
                let [dx, dy, dz]: [u32; 3] =
                    [0xf4, 0xf8, 0xfc].map(|o| h.read(base.wrapping_add(o)));
                let (minus_one, degrees) = (double(h, D_MINUS_ONE), double(h, D_DEGREES));
                let t = x87::store(x87::mul(f(dy), minus_one));
                let a = intrinsic(h, CI_ASIN, t);
                let pitch = ftol(h, x87::add(x87::mul(f(a), degrees), half));
                let (y1, y2) = (quiet(dy), quiet(dy));
                let sq = x87::store(x87::mul(f(y1), f(y2)));
                let w = x87::store(x87::sub(1.0, f(sq)));
                let s = intrinsic(h, CI_SQRT, w);
                let size = x87::store(f(s).abs());
                let yaw = if less(double(h, D_TINY), f(size)) {
                    let d = x87::store(x87::sub(f(dx), f(s)));
                    let d = x87::store(f(d).abs());
                    let a = if less(f(d), f(quiet(h.read(h.rebase(F_EPS))))) {
                        quiet(h.read(h.rebase(F_MINUS_90)))
                    } else {
                        let t = x87::store(x87::div(x87::mul(f(dx), minus_one), f(s)));
                        let a = intrinsic(h, CI_ASIN, t);
                        x87::store(x87::mul(f(a), degrees))
                    };
                    let t = x87::store(x87::div(f(dz), f(s)));
                    let b = intrinsic(h, CI_ACOS, t);
                    let b = x87::store(x87::mul(f(b), degrees));
                    let (fa, fb) = (f(a), f(b));
                    let k360 = double(h, D_360);
                    let r = if less(0.0, fa) && less(fb, 0.0) {
                        x87::add(fb, k360)
                    } else if less(fa, 0.0) && less(fb, 0.0) {
                        f(x87::store(fa.abs()))
                    } else if less(fa, 0.0) && less(0.0, fb) {
                        x87::sub(k360, fb)
                    } else {
                        fb
                    };
                    let r = x87::store(r);
                    ftol(h, x87::add(f(r), half))
                } else {
                    0
                };
                let name: u32 = h.read(tw + 0x94 + 4 * line);
                let args = [mark, name, rgb[0], rgb[1], rgb[2], yaw, pitch];
                (print(h, out, F_INTS, &args), None)
            }
            9 => {
                let k255 = double(h, D_255);
                let half = double(h, D_HALF);
                let z: u32 = h.read(cl.wrapping_add(0x198));
                let b = ftol(h, x87::add(x87::mul(f(z), k255), half));
                let y: u32 = h.read(cl.wrapping_add(0x194));
                let g = ftol(h, x87::add(x87::mul(f(y), k255), half));
                let x: u32 = h.read(cl.wrapping_add(0x190));
                let r = ftol(h, x87::add(half, x87::mul(k255, f(x))));
                let name: u32 = h.read(tw + 0xb8);
                (print(h, out, F_INTS, &[mark, name, r, g, b, 0, 0]), None)
            }
            10 => {
                let s = h.rebase(if h.read::<u8>(tw + 0xc9) != 0 {
                    TRUE
                } else {
                    FALSE
                });
                let name: u32 = h.read(tw + 0xbc);
                (print(h, out, F_STR, &[mark, name, s]), None)
            }
            _ => (line, None),
        }
    }
}
