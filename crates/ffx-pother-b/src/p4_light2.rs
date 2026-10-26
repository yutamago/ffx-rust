//! The character lighting manager's per-light updates (h2p4 round): the
//! intensity multiplier, the light rotation, the luminance balance with
//! the glow range, and the tweak utility's set-up.
//!
//! Layout (`CharacterLighting`, see [`crate::p4_light`]): `+0xf0` the
//! light set (`+4`: the `PLight` pointers), `+0xf4 + 0x10 i` the lights'
//! directions, `+0x124 + 0x10 i` their colours, `+0x18c` the scene (its
//! `+0x80`/`+0x84` the glow range, `+0x88`/`+0x8c` two factors, `+0xac`
//! the ambient colour), `+0x190` the ambient colour, `+0x1a0` the ambient
//! intensity, `+0x1a8 + 0xc i` the lights' intensity, pitch (`+0x1ac`)
//! and yaw (`+0x1b0`) in degrees, `+0x1cc` the initialised byte,
//! `+0x1d0 + 4 i` the luminances, `+0x1e0 + 4 i` the glow factors,
//! `+0x220` the map name, `+0x230` the ambient flag.
//!
//! Every value rounds to `f32` where the original stores (the x87 at 53
//! bits); `f32` copies through `fld`/`fstp` are quieted.

use ffx_bridge::x87;
use ffx_graphic::host::{Conv, Host};

use crate::methods::quiet;

const ONE: u32 = 0x3f80_0000;

/// `Phyre_CharacterLighting_applyIntensityMultiplier` (thiscall `(cl, i,
/// flag)`, `ret 8`).
pub const APPLY_INTENSITY: u32 = 0x006a_6e90;
/// `MSVCR110!strstr`'s import slot.
pub const STRSTR_SLOT: u32 = 0x00b0_c378;
/// `"znkd10"`, then the three maps of the ambient boost (`"znkd14"`,
/// `"znkd08"`, `"znkd09"`).
pub const LIGHT_MAPS: [u32; 4] = [0x00b4_8100, 0x00b4_80e8, 0x00b4_80f0, 0x00b4_80f8];
/// `0x006a9b40` (cdecl `(out, t, a, b)` -> out): `out = a + t (b - a)`.
pub const LERP3: u32 = 0x006a_9b40;
/// `Phyre_Vec3Max_004f84f0` (cdecl `(out, a, b)` -> out).
pub const VEC3_MAX: u32 = 0x004f_84f0;
/// `Phyre_Vec3Min_004f8580` (cdecl `(out, a, b)` -> out).
pub const VEC3_MIN: u32 = 0x004f_8580;
/// `PMatrix4_lookAt` (thiscall `(m, eye, target, up)`, `ret 0xc`).
pub const LOOK_AT: u32 = 0x0045_1c30;
/// `Matrix_RotationX` (cdecl `(out, angle)` -> out).
pub const ROTATION_X: u32 = 0x0066_c8a0;
/// `Matrix_RotationY` (cdecl `(out, angle)` -> out).
pub const ROTATION_Y: u32 = 0x0066_c920;
/// `PMatrix4_Multiply_004ca360` (cdecl `(out, a, b)` -> out).
pub const MULTIPLY: u32 = 0x004c_a360;
/// `PMatrix4_Copy_004c9430` (thiscall `(out, m)`, `ret 4`).
pub const COPY: u32 = 0x004c_9430;
/// `PMatrix4_toWorldMatrix3x4` (thiscall `(dst, m)`, `ret 4`).
pub const TO_WORLD_3X4: u32 = 0x0045_1190;
/// `operator_new_006304c0` (cdecl `(size)`).
pub const OPERATOR_NEW_ARRAY: u32 = 0x0063_04c0;
/// `TextUtil_init` (thiscall `(text)` -> 0 on success).
pub const TEXT_INIT: u32 = 0x006f_6370;
/// `TextUtil_appendLine` (thiscall `(text, line)`, `ret 4`).
pub const TEXT_APPEND_LINE: u32 = 0x006f_5e30;
/// `TextUtil_setEnabled` (thiscall `(text, on)`, `ret 4`).
pub const TEXT_SET_ENABLED: u32 = 0x006f_68f0;
/// `Phyre_CharacterLightingTweakUtil_getLineText` (thiscall `(tw, line,
/// out)`, `ret 8`).
pub const GET_LINE_TEXT: u32 = 0x006a_8580;
/// `crt_memset_thunk` (cdecl `(p, value, n)`).
pub const MEMSET: u32 = 0x0094_964c;
/// The tweak utility's eleven line names (`"Light index"` ..
/// `"Auto reset intensity"`), for `tw + 0x94 + 4 k`.
pub const LINE_NAMES: [u32; 11] = [
    0x00b4_8190,
    0x00b4_819c,
    0x00b4_81ac,
    0x00b4_81bc,
    0x00b4_81d4,
    0x00b4_81e8,
    0x00b4_81fc,
    0x00b4_8208,
    0x00b4_8214,
    0x00b4_8220,
    0x00b4_822c,
];

/// `f32` constants: 1.16 (the dim ambient factor), 2.6 (the boost),
/// 0.001 (the smallest rotation), 1.57 (the z-up turn), 1.1 and 0.01
/// (the glow range's clamp), 0.99.
const F_DIM: u32 = 0x00b4_82e8;
const F_BOOST: u32 = 0x00b4_8360;
const F_EPS: u32 = 0x00b3_68a4;
const F_TURN: u32 = 0x00b4_82f8;
const F_RANGE_MAX: u32 = 0x00b4_3d28;
const F_LOW: u32 = 0x00b1_f2b0;
const F_HIGH: u32 = 0x00b4_82dc;
/// `double` constants: 0.333, 1.2, 1.05, 0.95, the luminance weights 0.3,
/// 0.5, 0.2; 180, 3.14, 0.9; 0.99, 1.4, 0.01.
const D_THIRD: u32 = 0x00b4_8310;
const D_1_2: u32 = 0x00b4_8340;
const D_1_05: u32 = 0x00b4_8338;
const D_0_95: u32 = 0x00b4_8330;
const D_W0: u32 = 0x00b4_8308;
const D_W1: u32 = 0x00b9_2220;
const D_W2: u32 = 0x00b3_eb60;
const D_180: u32 = 0x00b4_8378;
const D_PI: u32 = 0x00b4_8358;
const D_UP: u32 = 0x00b4_8328;
const D_HIGH: u32 = 0x00b3_eb70;
const D_SQ_MAX: u32 = 0x00b4_8348;
const D_LOW: u32 = 0x00b1_f2b8;

fn f(bits: u32) -> f64 {
    x87::load(bits)
}

fn double(h: &impl Host, at: u32) -> f64 {
    let at = h.rebase(at);
    let lo: u32 = h.read(at);
    let hi: u32 = h.read(at + 4);
    f64::from_bits(u64::from(hi) << 32 | u64::from(lo))
}

fn single(h: &impl Host, at: u32) -> u32 {
    quiet(h.read(h.rebase(at)))
}

fn less(a: f64, b: f64) -> bool {
    a.partial_cmp(&b) == Some(std::cmp::Ordering::Less)
}

pub mod bridge {
    //! The originals over game memory.

    use super::{
        APPLY_INTENSITY, COPY, Conv, D_0_95, D_1_2, D_1_05, D_180, D_HIGH, D_LOW, D_PI, D_SQ_MAX,
        D_THIRD, D_UP, D_W0, D_W1, D_W2, F_BOOST, F_DIM, F_EPS, F_HIGH, F_LOW, F_RANGE_MAX, F_TURN,
        GET_LINE_TEXT, Host, LERP3, LIGHT_MAPS, LINE_NAMES, LOOK_AT, MEMSET, MULTIPLY, ONE,
        OPERATOR_NEW_ARRAY, ROTATION_X, ROTATION_Y, STRSTR_SLOT, TEXT_APPEND_LINE, TEXT_INIT,
        TEXT_SET_ENABLED, TO_WORLD_3X4, VEC3_MAX, VEC3_MIN, double, f, less, quiet, single, x87,
    };

    /// Whether the map name (`cl + 0x220`, re-read for each test) holds
    /// one of `maps` (`strstr` through its import slot, read once).
    fn on_map(h: &mut impl Host, cl: u32, maps: &[u32]) -> bool {
        if h.read::<u32>(cl + 0x220) == 0 {
            return false;
        }
        let strstr: u32 = h.read(h.rebase(STRSTR_SLOT));
        for &s in maps {
            let name: u32 = h.read(cl + 0x220);
            let s = h.rebase(s);
            if h.call(strstr, Conv::Cdecl, &[name, s]) != 0 {
                return true;
            }
        }
        false
    }

    /// The colour boost of `applyIntensityMultiplier` on the frame: the
    /// colour at `ebp - 0x28` moved away from its grey (`ebp - 0x38`, the
    /// mean times 0.333) by 2.6 (`0x006a9b40` into `ebp - 0x48`), then
    /// clamped to `[0, 1]` (`Phyre_Vec3Max` against zeros at `ebp - 0x28`,
    /// `Phyre_Vec3Min` against ones at `ebp - 0x38`, into `ebp - 0x28`);
    /// the result also at `ebp - 0x18`. Returns the grey (what the
    /// original leaves in its argument slot).
    fn boost(h: &mut impl Host, ebp: u32) -> u32 {
        let (out, grey, col, res) = (ebp - 0x48, ebp - 0x38, ebp - 0x28, ebp - 0x18);
        let x: u32 = h.read(col);
        let y: u32 = h.read(col + 4);
        let z: u32 = h.read(col + 8);
        let s = x87::store(x87::add(f(y), f(x)));
        let s = x87::store(x87::add(f(s), f(z)));
        let avg = x87::store(x87::mul(f(s), double(h, D_THIRD)));
        for k in 0..3 {
            h.write(grey + 4 * k, avg);
        }
        let t = single(h, F_BOOST);
        let r = h.call_sym(LERP3, Conv::Cdecl, &[out, t, grey, col]);
        for k in 0..3 {
            let w = quiet(h.read(r.wrapping_add(4 * k)));
            h.write(res + 4 * k, w);
        }
        for k in 0..3 {
            h.write(grey + 4 * k, ONE);
        }
        for k in 0..3 {
            h.write(col + 4 * k, 0u32);
        }
        let r = h.call_sym(VEC3_MAX, Conv::Cdecl, &[out, col, res]);
        h.call_sym(VEC3_MIN, Conv::Cdecl, &[col, r, grey]);
        for k in 0..3 {
            let w = quiet(h.read(col + 4 * k));
            h.write(res + 4 * k, w);
        }
        avg
    }

    /// `CharacterLighting::applyIntensityMultiplier(cl, i, flag)`
    /// (`0x006a6e90`, `ret 8`; `flag` unused). The factor is 1, or 1.16
    /// without the ambient flag (`+0x230`), at `ebp - 4`.
    ///
    /// Light `i` other than 3: its level `(1 - [+0x1e0 + 4i]) * 1.2 *
    /// [+0x1a8 + 0xc i] * factor * 0.95` goes to the light (`+0x24`) and,
    /// times the colour's luminance (0.3, 0.5, 0.2, summed in one x87
    /// expression), to `+0x1d0 + 4i`. On the `znkd10/14/08/09` maps the
    /// colour is boosted first ([`boost`]). Returns the light (`eax`).
    ///
    /// Light 3, the ambient: the level `(1 - [+0x1ec]) * 1.2 * [+0x1a0] *
    /// factor * 1.05` to `+0x1a4`, the scene's ambient colour (`scene +
    /// 0xac`, when there is a scene) the ambient colour times it, and
    /// `+0x1dc` the colour's luminance (rounded at each sum) times it; the
    /// boost on `znkd14/08/09` only. Returns the scene (`eax`).
    ///
    /// The second word is what the original leaves in its argument slot
    /// (`None`: the argument), `ebp` its frame (`ebp - 0x48` up).
    pub fn apply_intensity(h: &mut impl Host, ebp: u32, cl: u32, i: u32) -> (u32, Option<u32>) {
        let factor = if h.read::<u8>(cl + 0x230) == 0 {
            single(h, F_DIM)
        } else {
            ONE
        };
        h.write(ebp - 4, factor);
        if i == 3 {
            return apply_ambient_level(h, ebp, cl);
        }
        let glow: u32 = h.read(cl.wrapping_add(0x1e0).wrapping_add(i.wrapping_mul(4)));
        let set: u32 = h.read(cl + 0xf0);
        let lights: u32 = h.read(set.wrapping_add(4));
        let light: u32 = h.read(lights.wrapping_add(i.wrapping_mul(4)));
        h.write(ebp - 8, light);
        let t = x87::mul(x87::sub(1.0, f(glow)), double(h, D_1_2));
        let intensity: u32 = h.read(cl.wrapping_add(0x1a8).wrapping_add(i.wrapping_mul(0xc)));
        let t = x87::mul(t, f(intensity));
        let colour = cl.wrapping_add(0x124).wrapping_add(i.wrapping_mul(0x10));
        for k in 0..4 {
            let w: u32 = h.read(colour.wrapping_add(4 * k));
            h.write(ebp - 0x28 + 4 * k, w);
        }
        let t = x87::mul(t, f(h.read(ebp - 4)));
        for k in 0..4 {
            let w: u32 = h.read(colour.wrapping_add(4 * k));
            h.write(ebp - 0x18 + 4 * k, w);
        }
        let level = x87::store(x87::mul(t, double(h, D_0_95)));
        h.write(ebp - 4, level);
        let slot = if on_map(h, cl, &LIGHT_MAPS) {
            Some(boost(h, ebp))
        } else {
            None
        };
        let level: u32 = h.read(ebp - 4);
        let light: u32 = h.read(ebp - 8);
        h.write(light.wrapping_add(0x24), level);
        let y = x87::mul(f(h.read(ebp - 0x14)), double(h, D_W1));
        let x = x87::mul(f(h.read(ebp - 0x18)), double(h, D_W0));
        let s = x87::add(y, x);
        let z = x87::mul(f(h.read(ebp - 0x10)), double(h, D_W2));
        let s = x87::add(s, z);
        let lum = x87::store(x87::mul(f(level), s));
        h.write(cl.wrapping_add(0x1d0).wrapping_add(i.wrapping_mul(4)), lum);
        (light, slot)
    }

    /// Light 3 of [`apply_intensity`].
    fn apply_ambient_level(h: &mut impl Host, ebp: u32, cl: u32) -> (u32, Option<u32>) {
        for (k, o) in [0x190u32, 0x194, 0x198].into_iter().enumerate() {
            let w = quiet(h.read(cl + o));
            h.write(ebp - 0x28 + 4 * k as u32, w);
            h.write(ebp - 0x18 + 4 * k as u32, w);
        }
        // junk read: [ebp-0x1c] never written; 0 (brief 2026-10-24): the
        // colour's unused fourth word, which nothing reads
        h.write(ebp - 0xc, 0u32);
        if on_map(h, cl, &LIGHT_MAPS[1..]) {
            boost(h, ebp);
        }
        let t = x87::sub(1.0, f(h.read(cl + 0x1ec)));
        let t = x87::mul(t, double(h, D_1_2));
        let t = x87::mul(t, f(h.read(cl + 0x1a0)));
        let t = x87::mul(t, f(h.read(ebp - 4)));
        let level = x87::store(x87::mul(t, double(h, D_1_05)));
        h.write(cl + 0x1a4, level);
        let scene: u32 = h.read(cl + 0x18c);
        let z: u32 = h.read(ebp - 0x10);
        let y: u32 = h.read(ebp - 0x14);
        let x: u32 = h.read(ebp - 0x18);
        if scene != 0 {
            let p = [
                x87::store(x87::mul(f(x), f(level))),
                x87::store(x87::mul(f(y), f(level))),
                x87::store(x87::mul(f(level), f(z))),
            ];
            for (k, w) in p.into_iter().enumerate() {
                h.write(ebp - 0x28 + 4 * k as u32, w);
            }
            for (k, w) in p.into_iter().enumerate() {
                h.write(scene.wrapping_add(0xac + 4 * k as u32), w);
            }
        }
        let a = x87::store(x87::mul(f(x), double(h, D_W0)));
        let s = x87::store(x87::add(f(a), x87::mul(f(y), double(h, D_W1))));
        let lum = x87::store(x87::add(f(s), x87::mul(f(z), double(h, D_W2))));
        let level: u32 = h.read(cl + 0x1a4);
        h.write(cl + 0x1dc, x87::store(x87::mul(f(lum), f(level))));
        (scene, Some(lum))
    }

    /// `CharacterLighting::applyRotation(cl, i)` (`0x006a7280`, `ret 4`):
    /// for light `i` other than 3, with its yaw (`+0x1b0 + 0xc i`) or
    /// pitch (`+0x1ac`) turned to radians (`/ 180 * 3.14`, the yaw kept in
    /// the argument slot, the pitch at `ebp - 0x48`) above 0.001 in size:
    /// the light's node (`light + 0x10`) gets the matrix that looks along
    /// its direction (`+0xf4 + 0x10 i`; the identity at `ebp - 0x44`, from
    /// the origin `ebp - 0x98`, up `(0, 1, 0)` at `ebp - 0xa8`, or `(0, 0,
    /// 1)` when the direction's y is not below 0.9, which also builds an
    /// unused 1.57 x-rotation at `ebp - 0x88`), turned by the yaw and then
    /// the pitch (`Matrix_RotationY/X` into `ebp - 0xe8`/`-0x128`, times
    /// the matrix, copied through `ebp - 0x88`), as a 3x4. Returns
    /// `PMatrix4_toWorldMatrix3x4`'s `eax` (`None` where the original's
    /// is the status word or the caller's) and the argument slot's word.
    pub fn apply_rotation(
        h: &mut impl Host,
        ebp: u32,
        cl: u32,
        i: u32,
    ) -> (Option<u32>, Option<u32>) {
        if i == 3 {
            return (None, None);
        }
        let set: u32 = h.read(cl + 0xf0);
        let lights: u32 = h.read(set.wrapping_add(4));
        let light: u32 = h.read(lights.wrapping_add(i.wrapping_mul(4)));
        let at = cl.wrapping_add(i.wrapping_mul(0xc));
        let (k180, kpi) = (double(h, D_180), double(h, D_PI));
        let yaw = x87::div(f(h.read(at.wrapping_add(0x1b0))), k180);
        let yaw = x87::store(x87::mul(yaw, kpi));
        let pitch = x87::div(f(h.read(at.wrapping_add(0x1ac))), k180);
        let pitch = x87::store(x87::mul(kpi, pitch));
        h.write(ebp - 0x48, pitch);
        let eps = f(single(h, F_EPS));
        let size = x87::store(f(yaw).abs());
        h.write(ebp - 4, size);
        let mut turn = less(eps, f(size));
        if !turn {
            let size = x87::store(f(pitch).abs());
            h.write(ebp - 4, size);
            turn = less(eps, f(size));
        }
        if !turn {
            return (None, Some(yaw));
        }
        let dir = cl.wrapping_add(0xf4).wrapping_add(i.wrapping_mul(0x10));
        let (up, eye, m, tmp) = (ebp - 0xa8, ebp - 0x98, ebp - 0x44, ebp - 0x88);
        h.write(up, 0u32);
        for o in [0xa4, 0x44, 0x30, 0x1c, 0x8] {
            h.write(ebp - o, ONE);
        }
        for o in [
            0xa0, 0x40, 0x3c, 0x38, 0x34, 0x2c, 0x28, 0x24, 0x20, 0x18, 0x14, 0x10, 0xc,
        ] {
            h.write(ebp - o, 0u32);
        }
        let t = x87::store(x87::mul(f(h.read(dir)), 0.0));
        h.write(ebp - 4, t);
        let t = x87::store(x87::add(f(t), f(h.read(dir.wrapping_add(4)))));
        h.write(ebp - 4, t);
        let d = x87::store(x87::add(
            x87::mul(0.0, f(h.read(dir.wrapping_add(8)))),
            f(t),
        ));
        h.write(ebp - 4, d);
        let z_up = !less(f(d), double(h, D_UP));
        if z_up {
            for o in [0, 4] {
                h.write(up + o, 0u32);
            }
        }
        for o in [0, 4, 8] {
            h.write(eye + o, 0u32);
        }
        if z_up {
            h.write(up + 8, ONE);
        }
        h.call_sym(LOOK_AT, Conv::Thiscall(m), &[eye, dir, up]);
        if z_up {
            let a = single(h, F_TURN);
            h.call_sym(ROTATION_X, Conv::Cdecl, &[tmp, a]);
        }
        for (rot, angle, (r, out)) in [
            (ROTATION_Y, yaw, (ebp - 0xe8, ebp - 0x128)),
            (ROTATION_X, pitch, (ebp - 0x128, ebp - 0xe8)),
        ] {
            let r = h.call_sym(rot, Conv::Cdecl, &[r, angle]);
            let p = h.call_sym(MULTIPLY, Conv::Cdecl, &[out, r, m]);
            h.call_sym(COPY, Conv::Thiscall(tmp), &[p]);
            for k in 0..16 {
                let w = quiet(h.read(tmp + 4 * k));
                h.write(m + 4 * k, w);
            }
        }
        let node: u32 = h.read(light.wrapping_add(0x10));
        let r = h.call_sym(TO_WORLD_3X4, Conv::Thiscall(node), &[m]);
        (Some(r), Some(yaw))
    }

    /// `CharacterLighting::balanceLumWithGlow(cl)` (`0x006a7550`,
    /// fastcall). On an initialised manager: the scene's glow range
    /// (`hi = scene + 0x80`, `lo = +0x84`, their difference clamped to
    /// `[0.01, 1.1]`) and factors `a = +0x88`, `b = +0x8c` (each clamped to
    /// `[0.01, 0.99]`; a NaN stays); every light applied again with its
    /// glow factor (`+0x1e0 + 4i`) zeroed, keeping the largest luminance
    /// (`+0x1d0 + 4i`, from 0, read before each apply). When that is above
    /// 0.01 each light's glow factor becomes its luminance over the
    /// largest, clamped to `[lo, hi]`, as a fraction of the range, times
    /// `a^2 + b^2` clamped to `[0.01, 1.4]`, halved, clamped to `[0.01,
    /// 0.99]`; and every light is applied again. Returns the last apply's
    /// `eax` (`None` where the original's is junk: the cookie or a status
    /// word).
    pub fn balance_lum_with_glow(h: &mut impl Host, cl: u32) -> Option<u32> {
        if h.read::<u8>(cl + 0x1cc) == 0 {
            return None;
        }
        let scene: u32 = h.read(cl + 0x18c);
        let a = quiet(h.read(scene.wrapping_add(0x88)));
        let hi = quiet(h.read(scene.wrapping_add(0x80)));
        let lo = quiet(h.read(scene.wrapping_add(0x84)));
        let range = x87::store(x87::sub(f(hi), f(lo)));
        let b = quiet(h.read(scene.wrapping_add(0x8c)));
        let (low, high) = (single(h, F_LOW), single(h, F_HIGH));
        let high_d = double(h, D_HIGH);
        let range = if less(f(single(h, F_RANGE_MAX)), f(range)) {
            single(h, F_RANGE_MAX)
        } else if less(f(range), f(low)) {
            low
        } else {
            range
        };
        let factor = |v: u32| {
            if less(high_d, f(v)) {
                high
            } else if less(f(v), f(low)) {
                low
            } else {
                v
            }
        };
        let (a, b) = (factor(a), factor(b));
        let mut top = 0u32;
        for i in 0..4u32 {
            let lum: u32 = h.read(cl + 0x1d0 + 4 * i);
            if less(f(top), f(lum)) {
                top = quiet(h.read(cl + 0x1d0 + 4 * i));
            }
            h.write(cl + 0x1e0 + 4 * i, 0u32);
            h.call_sym(APPLY_INTENSITY, Conv::Thiscall(cl), &[i, 1]);
        }
        if !less(f(low), f(top)) {
            return None;
        }
        let sq = x87::add(x87::mul(f(b), f(b)), x87::mul(f(a), f(a)));
        let (sq_max, low_d, half) = (double(h, D_SQ_MAX), double(h, D_LOW), double(h, D_W1));
        let scale = if less(sq_max, sq) {
            sq_max
        } else if less(sq, low_d) {
            low_d
        } else {
            sq
        };
        for k in 0..4u32 {
            let lum: u32 = h.read(cl + 0x1d0 + 4 * k);
            let r = x87::store(x87::div(f(lum), f(top)));
            let r = if less(f(hi), f(r)) {
                hi
            } else if less(f(r), f(lo)) {
                lo
            } else {
                r
            };
            let at = cl + 0x1e0 + 4 * k;
            let t = x87::store(x87::div(x87::sub(f(r), f(lo)), f(range)));
            h.write(at, t);
            let u = x87::store(x87::mul(f(t), scale));
            let w = x87::store(x87::mul(f(u), half));
            h.write(at, w);
            if less(high_d, f(w)) {
                h.write(at, high);
            } else if less(f(w), f(low)) {
                h.write(at, low);
            }
        }
        let mut eax = 0;
        for i in 0..4u32 {
            eax = h.call_sym(APPLY_INTENSITY, Conv::Thiscall(cl), &[i, 1]);
        }
        Some(eax)
    }

    /// `CharacterLightingTweakUtil::initialize(tw)` (`0x006a91e0`,
    /// fastcall): the 100 shade entries (`new[](0x4b0)` at `+0xf8`, each
    /// `{0, 0.0, 2}`, the pointer re-read for every store), then the text
    /// (`TextUtil_init(tw + 4)`). On success `+0 = 1`, the eleven line
    /// names at `+0x94`, the selected line `+0xc4 = 0`, and each line's
    /// text (`getLineText` into the zeroed 0x80 bytes at `ebp - 0x84`)
    /// appended; otherwise `+0 = 0`. The text is disabled either way.
    /// Returns `TextUtil_setEnabled`'s `eax`.
    pub fn tweak_initialize(h: &mut impl Host, ebp: u32, tw: u32) -> u32 {
        let p = h.call_sym(OPERATOR_NEW_ARRAY, Conv::Cdecl, &[0x4b0]);
        h.write(tw + 0xf8, p);
        for e in (0..0x4b0u32).step_by(0xc) {
            for (o, v) in [(0, 0u32), (4, 0), (8, 2)] {
                let base: u32 = h.read(tw + 0xf8);
                h.write(base.wrapping_add(e + o), v);
            }
        }
        let text = tw + 4;
        let r = h.call_sym(TEXT_INIT, Conv::Thiscall(text), &[]);
        if r != 0 {
            let eax = h.call_sym(TEXT_SET_ENABLED, Conv::Thiscall(text), &[0]);
            h.write(tw, 0u8);
            return eax;
        }
        h.write(tw, 1u8);
        for (k, s) in LINE_NAMES.into_iter().enumerate() {
            let s = h.rebase(s);
            h.write(tw + 0x94 + 4 * k as u32, s);
        }
        h.write(tw + 0xc4, 0u32);
        let line = ebp - 0x84;
        for i in 0..11 {
            h.call_sym(MEMSET, Conv::Cdecl, &[line, 0, 0x80]);
            h.call_sym(GET_LINE_TEXT, Conv::Thiscall(tw), &[i, line]);
            h.call_sym(TEXT_APPEND_LINE, Conv::Thiscall(text), &[line]);
        }
        h.call_sym(TEXT_SET_ENABLED, Conv::Thiscall(text), &[0])
    }
}

/// `Phyre_PCluster_create` (thiscall `(cluster, count, desc, flags)`, `ret
/// 0xc`).
pub const CLUSTER_CREATE: u32 = 0x0044_1010;
/// The material-list class (`0x00c998e8`) the instance's copy is made of.
pub const MATERIAL_LIST_DESC: u32 = 0x00c9_98e8;
/// `Phyre_PArray_resizeInline_0048c3b0` (thiscall `(array, n)`, `ret 4`).
pub const ARRAY_RESIZE: u32 = 0x0048_c3b0;
/// `Phyre_PFreeList_allocateNewBlock` (thiscall `(list, n)`, `ret 4`).
pub const FREE_LIST_GROW: u32 = 0x0049_c8d0;
/// `Phyre_PFreeList_deallocate` (thiscall `(list, p)`, `ret 4`).
pub const FREE_LIST_DEALLOCATE: u32 = 0x0049_c980;
/// `SearchTree_insertUnsigned_006a5fc0` (thiscall `(tree, node)` -> 0 or
/// not, `ret 4`).
pub const TREE_INSERT_UNSIGNED: u32 = 0x006a_5fc0;
/// `Phyre_PRedBlackTree_postInsertFinalize` (thiscall `(tree, node)`,
/// `ret 4`).
pub const TREE_FINALIZE: u32 = 0x0057_4980;
/// `PArray_Data_004d45a0` (thiscall `(array)` -> its elements).
pub const ARRAY_DATA: u32 = 0x004d_45a0;
/// `Phyre_PRendering_PEffectVariant_createMaterial` (thiscall `(variant,
/// cluster, a, b)` -> material, `ret 0xc`).
pub const CREATE_MATERIAL: u32 = 0x004e_6a40;
/// `Phyre_PhyreMalloc_0042fb80` (cdecl `(size, align)`).
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
/// `Phyre_PhyreFree` (cdecl `(p)`).
pub const PHYRE_FREE: u32 = 0x0042_fb20;
/// `Phyre_PRendering_PMaterial_setEffectVariant` (thiscall `(material,
/// variant, params)`, `ret 8`).
pub const SET_EFFECT_VARIANT: u32 = 0x004d_8940;
/// `ShaderParams_FindByName` (thiscall `(shader, name)` -> parameter).
pub const FIND_PARAM: u32 = 0x0056_cbf0;
/// `memcpy` (cdecl `(dst, src, n)`).
pub const MEMCPY: u32 = 0x0094_9256;
/// `Phyre_CharacterLighting_setLightMatrix` (thiscall `(cl, inst, a, b)`,
/// `ret 0xc`).
pub const SET_LIGHT_MATRIX: u32 = 0x006a_abf0;
/// `g_CharacterLightingTweakUtil`.
pub const G_TWEAK: u32 = 0x00cc_ca88;
/// `"USE_CHARACTER_LIGHTING"`, the empty string, `"CharacterAmbientLight"`.
pub const USE_LIGHTING: u32 = 0x00b4_8160;
pub const EMPTY: u32 = 0x00b8_fbeb;
pub const AMBIENT_PARAM: u32 = 0x00b4_8178;

pub mod bridge2 {
    //! The original over game memory.

    use super::{
        AMBIENT_PARAM, ARRAY_DATA, ARRAY_RESIZE, CLUSTER_CREATE, CREATE_MATERIAL, Conv, EMPTY,
        FIND_PARAM, FREE_LIST_DEALLOCATE, FREE_LIST_GROW, G_TWEAK, Host, MATERIAL_LIST_DESC,
        MEMCPY, PHYRE_FREE, PHYRE_MALLOC, SET_EFFECT_VARIANT, SET_LIGHT_MATRIX, TREE_FINALIZE,
        TREE_INSERT_UNSIGNED, USE_LIGHTING,
    };

    const COUNT: u32 = 0x7fff_ffff;

    /// A tagged string pointer (bit 0 set: a pointer + 1).
    fn untag(h: &impl Host, at: u32) -> u32 {
        let w: u32 = h.read(at);
        w.wrapping_sub(w & 1)
    }

    /// Whether the C strings at `a` and `b` are equal (the inline compare,
    /// two bytes a step).
    fn same(h: &impl Host, a: u32, b: u32) -> bool {
        let mut k = 0u32;
        loop {
            let x: u8 = h.read(a.wrapping_add(k));
            let y: u8 = h.read(b.wrapping_add(k));
            if x != y {
                return false;
            }
            if x == 0 {
                return true;
            }
            k += 1;
        }
    }

    /// Whether the effect variant's switch at `at` (`{name, value}`, both
    /// tagged) turns the character lighting on: `"USE_CHARACTER_LIGHTING"`
    /// (the string itself or a copy) with a value that is null or not
    /// empty. `None`: not that switch.
    fn lighting_switch(h: &impl Host, at: u32) -> Option<bool> {
        let key = untag(h, at);
        let use_lighting = h.rebase(USE_LIGHTING);
        if key != use_lighting && (key == 0 || !same(h, key, use_lighting)) {
            return None;
        }
        let v = untag(h, at + 4);
        let empty = h.rebase(EMPTY);
        if v == empty {
            return Some(false);
        }
        Some(v == 0 || !same(h, v, empty))
    }

    /// The `PArray` element block (more than one at `[arr + 4]`, one inline
    /// at `arr + 4`, none 0).
    fn elements(h: &impl Host, arr: u32) -> u32 {
        match h.read::<u32>(arr) & COUNT {
            0 => 0,
            1 => arr + 4,
            _ => h.read(arr + 4),
        }
    }

    /// `CharacterLighting::instantiateUniqueMaterialParameters(cl, inst)`
    /// (`0x006a96b0`, `ret 4`) -> 0, 0xe (no free tree node) or 0x12 (the
    /// insert failed). On an initialised manager the instance gets its own
    /// material list: a new list (`PCluster_create(cl + 0xa0; 1,
    /// 0x00c998e8, 0)`) sized as the instance's (`inst + 0x10`),
    /// remembered in the shade tree (`cl + 0x158`: a node from its free
    /// list `+0x170`, key the old list `+0x10`, the new one `+0xc`). For
    /// each old element `{variant, params}` a material of the variant
    /// (`createMaterial(cl + 0xa0, 0, 0)`) gets the parameters that fit
    /// both buffers copied, a copy of the element's name (`+0x10`, tagged;
    /// `PhyreMalloc(n, 2)`, the old untagged one freed; a failed copy
    /// keeps the old name), the variant (`setEffectVariant`), and `+0x14`
    /// the variant's `"CharacterAmbientLight"` parameter when a
    /// `"USE_CHARACTER_LIGHTING"` switch is on (searched from the end); it
    /// goes into the new list. Then `inst + 0x10` is the new list (or
    /// `[inst] + 0x30` when there is none), `setLightMatrix(inst, zero,
    /// zero)` with a zero matrix at `ebp - 0x74`, and the tweak utility
    /// (when on) records the instance in its first free shade entry
    /// (`{inst, 0.0, 2}`).
    ///
    /// The switch test against the empty string's own address
    /// (`0x00b8fbeb`) never holds: the value is untagged (even), the
    /// string's address odd.
    #[allow(clippy::too_many_lines)]
    pub fn instantiate(h: &mut impl Host, ebp: u32, cl: u32, inst: u32) -> u32 {
        h.write(ebp - 0x28, cl);
        if h.read::<u8>(cl + 0x1cc) == 0 {
            return 0;
        }
        let cluster = cl + 0xa0;
        h.write(ebp - 0x2c, cluster);
        let desc = h.rebase(MATERIAL_LIST_DESC);
        let new_list = h.call_sym(CLUSTER_CREATE, Conv::Thiscall(cluster), &[1, desc, 0]);
        h.write(ebp - 0x10, new_list);
        let list: u32 = h.read(inst + 0x10);
        h.write(ebp - 0x14, list);
        let n = h.read::<u32>(list) & COUNT;
        h.write(ebp - 0x24, n);
        h.call_sym(ARRAY_RESIZE, Conv::Thiscall(new_list), &[n]);
        let free = cl + 0x170;
        let mut node: u32 = h.read(free);
        if node == 0 {
            let k: u32 = h.read(cl + 0x178);
            h.call_sym(FREE_LIST_GROW, Conv::Thiscall(free), &[k]);
            node = h.read(free);
            if node == 0 {
                return 0xe;
            }
        }
        let next: u32 = h.read(node);
        h.write(free, next);
        h.write(node + 0xc, new_list);
        h.write(node, node);
        h.write(node + 4, node);
        h.write(node + 8, node);
        h.write(node + 0x10, list);
        let tree = cl + 0x158;
        if h.call_sym(TREE_INSERT_UNSIGNED, Conv::Thiscall(tree), &[node]) != 0 {
            h.call_sym(FREE_LIST_DEALLOCATE, Conv::Thiscall(free), &[node]);
            return 0x12;
        }
        h.call_sym(TREE_FINALIZE, Conv::Thiscall(tree), &[node]);
        h.write(ebp - 0x20, 0u32);
        for k in 0..n {
            let data = h.call_sym(ARRAY_DATA, Conv::Thiscall(list), &[]);
            let el: u32 = h.read(data.wrapping_add(4 * k));
            let variant: u32 = h.read(el);
            h.write(ebp - 0x1c, el);
            h.write(ebp - 0x34, variant);
            let mat = h.call_sym(CREATE_MATERIAL, Conv::Thiscall(variant), &[cluster, 0, 0]);
            let src: u32 = h.read(el + 4);
            h.write(ebp - 0xc, mat);
            let dst: u32 = h.read(mat.wrapping_add(4));
            let count = h.read::<u32>(src.wrapping_add(8)) & COUNT;
            h.write(ebp - 8, dst);
            h.write(ebp - 4, src);
            h.write(ebp - 0x18, count);
            for j in 0..count {
                let params: u32 = h.read(src.wrapping_add(0xc));
                let p = params.wrapping_add(0x10 * j);
                let off = u32::from(h.read::<u16>(p.wrapping_add(8)));
                let size = u32::from(h.read::<u16>(p.wrapping_add(0xa)) & 0x1fff);
                if off + size <= h.read::<u32>(src) && off >= 4 {
                    h.write(ebp - 0x30, off);
                    if off + size <= h.read::<u32>(dst) {
                        h.call_sym(
                            MEMCPY,
                            Conv::Cdecl,
                            &[dst.wrapping_add(off), off.wrapping_add(src), size],
                        );
                    }
                }
                h.write(ebp - 0x18, count - j - 1);
            }
            // the name: a copy of the element's, or none
            let name = untag(h, el + 0x10);
            let copy = if name == 0 {
                Some(0)
            } else {
                let mut len = 0u32;
                while h.read::<u8>(name.wrapping_add(len)) != 0 {
                    len += 1;
                }
                let q = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[len + 1, 2]);
                if q == 0 {
                    None
                } else {
                    h.call_sym(MEMCPY, Conv::Cdecl, &[q, name, len + 1]);
                    Some(q)
                }
            };
            if let Some(q) = copy {
                let old: u32 = h.read(mat + 0x10);
                if old & 1 == 0 {
                    h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
                }
                h.write(mat + 0x10, q);
            }
            let variant: u32 = h.read(el);
            let dst: u32 = h.read(ebp - 8);
            h.call_sym(SET_EFFECT_VARIANT, Conv::Thiscall(mat), &[variant, dst]);
            let variant: u32 = h.read(el);
            let switches = h.read::<u32>(variant + 4) & COUNT;
            if switches != 0 {
                let base: u32 = h.read(variant + 8);
                for s in (0..switches).rev() {
                    if lighting_switch(h, base.wrapping_add(8 * s)) == Some(true) {
                        let shader: u32 = h.read(ebp - 0x34);
                        let name = h.rebase(AMBIENT_PARAM);
                        let p = h.call_sym(FIND_PARAM, Conv::Thiscall(shader), &[name]);
                        h.write(mat + 0x14, p);
                        break;
                    }
                }
            }
            if k < h.read::<u32>(new_list) & COUNT {
                let at = elements(h, new_list);
                h.write(at.wrapping_add(4 * k), mat);
            }
            h.write(ebp - 0x20, k + 1);
        }
        let own = if new_list == 0 {
            h.read::<u32>(inst).wrapping_add(0x30)
        } else {
            new_list
        };
        h.write(inst + 0x10, own);
        let m = ebp - 0x74;
        for k in 0..16 {
            h.write(m + 4 * k, 0u32);
        }
        h.call_sym(SET_LIGHT_MATRIX, Conv::Thiscall(cl), &[inst, m, m]);
        let tw: u32 = h.read(h.rebase(G_TWEAK));
        if h.read::<u8>(tw) == 0 {
            return 0;
        }
        let entries: u32 = h.read(tw + 0xf8);
        for i in 0..100u32 {
            if h.read::<u32>(entries + 0xc * i) == 0 {
                h.write(entries + 0xc * i, inst);
                let e: u32 = h.read(tw + 0xf8);
                h.write(e + 0xc * i + 4, 0u32);
                let e: u32 = h.read(tw + 0xf8);
                h.write(e + 0xc * i + 8, 2u32);
                return 0;
            }
        }
        0
    }
}
