//! The character lighting manager (`g_CharacterLighting`, 0x234 bytes),
//! its tweak utility (`g_CharacterLightingTweakUtil`, 0xfc bytes) and the
//! morph manager's singleton: creation, small setters, the shade
//! coefficient, the managers' growable arrays.
//!
//! Layout used here (`CharacterLighting`): `+0xf0` the light set (its
//! `+4`: the `PLight` pointers by index), `+0x124 + 0x10 i` the lights'
//! colours, `+0x154` the flash timer, `+0x190` the ambient colour,
//! `+0x1a0` the global intensity, `+0x1a4` the ambient intensity,
//! `+0x1cc` the initialised byte, `+0x1d0..0x1dc` four intensities.
//!
//! Float copies go through the x87 (`fld`/`fstp`), so a signalling NaN is
//! stored quieted ([`crate::methods::quiet`]).

use ffx_bridge::x87;
use ffx_graphic::host::{Conv, Host};

/// `g_CharacterLighting`.
pub const G_LIGHTING: u32 = 0x00cc_ca84;
/// `g_CharacterLightingTweakUtil`.
pub const G_TWEAK: u32 = 0x00cc_ca88;
/// `g_MorphManager`.
pub const G_MORPH: u32 = 0x00cc_ca90;
/// `operator_new` `0x00630310` (cdecl `(size)`).
pub const OPERATOR_NEW: u32 = 0x0063_0310;
/// `CharacterLighting::CharacterLighting` `0x006a63f0` (thiscall `(cl)` ->
/// `cl`).
pub const LIGHTING_CTOR: u32 = 0x006a_63f0;
/// `TextUtil::TextUtil` (thiscall `(text)`).
pub const TEXT_CTOR: u32 = 0x006f_5b50;
/// `crt_memset_thunk` (cdecl `(p, value, n)`).
pub const MEMSET: u32 = 0x0094_964c;
/// `Phyre_CharacterLighting_applyIntensityMultiplier` (thiscall `(cl, i,
/// flag)`).
pub const APPLY_INTENSITY: u32 = 0x006a_6e90;
/// `Phyre_CharacterLighting_setShadeCoeff` (thiscall `(cl, inst, coeff)`).
pub const SET_SHADE_COEFF: u32 = 0x006a_aef0;
/// `0x006a8bc0` (thiscall `(cl)` -> `ST0`): the other shade coefficient
/// the tweak table's mode 1 picks.
pub const SHADE_COEFF_ALT: u32 = 0x006a_8bc0;
/// `Phyre_PhyreFree` (cdecl `(ptr)`).
pub const PHYRE_FREE: u32 = 0x0042_fb20;
/// The point-light type's vtable (`PLightType`, `0x00c10814`).
pub const POINT_LIGHT_TYPE: u32 = 0x00c1_0814;
/// `CharacterLighting_iteratorValid_006a9a80` (thiscall `(it)` -> `al`).
pub const ITERATOR_VALID: u32 = 0x006a_9a80;
/// `PhyreSceneCursor_SetNode` (thiscall `(cursor, node)`).
pub const CURSOR_SET_NODE: u32 = 0x0044_1dc0;
/// The light type `setLightColor` updates the light of (`0x00c10828`).
pub const COLOR_LIGHT_TYPE: u32 = 0x00c1_0828;

const ONE: u32 = 0x3f80_0000;

/// `CharacterLighting::defaultShadeCoeff` (`0x006a8c40`) of the four
/// intensities `+0x1d0..0x1dc`: their maximum (at least 0; a NaN never
/// wins) times 1.5, clamped to `[0.3, 1]` (as the `f32` it returns).
#[must_use]
pub fn default_shade_coeff(intensities: [u32; 4]) -> f32 {
    let mut best = 0.0f64;
    for bits in intensities {
        let v = x87::load(bits);
        if best < v {
            best = v;
        }
    }
    let v = x87::mul(best, 1.5);
    // the low clamp is a double holding 0.3f
    let low = f64::from(0.3f32);
    let r = if 1.0 < v {
        1.0
    } else if low > v {
        low
    } else {
        v
    };
    f32::from_bits(x87::store(r))
}

pub mod bridge {
    //! The originals over game memory.

    use super::{
        APPLY_INTENSITY, COLOR_LIGHT_TYPE, CURSOR_SET_NODE, Conv, G_LIGHTING, G_MORPH, G_TWEAK,
        Host, ITERATOR_VALID, LIGHTING_CTOR, MEMSET, ONE, OPERATOR_NEW, PHYRE_FREE,
        POINT_LIGHT_TYPE, SET_SHADE_COEFF, SHADE_COEFF_ALT, TEXT_CTOR, default_shade_coeff, x87,
    };
    use crate::methods::quiet;

    /// `CharacterLighting_create()` (`0x006a6cd0`): `g_CharacterLighting =
    /// new CharacterLighting` when there is none.
    pub fn lighting_create(h: &mut impl Host) {
        let g = h.rebase(G_LIGHTING);
        if h.read::<u32>(g) != 0 {
            return;
        }
        let f = h.rebase(OPERATOR_NEW);
        let p = h.call(f, Conv::Cdecl, &[0x234]);
        let v = if p == 0 {
            0
        } else {
            let f = h.rebase(LIGHTING_CTOR);
            h.call(f, Conv::Thiscall(p), &[])
        };
        h.write(g, v);
    }

    /// `CharacterLightingTweakUtil_create()` (`0x006a6d50`):
    /// `g_CharacterLightingTweakUtil = new CharacterLightingTweakUtil` when
    /// there is none.
    pub fn tweak_create(h: &mut impl Host) {
        let g = h.rebase(G_TWEAK);
        if h.read::<u32>(g) != 0 {
            return;
        }
        let f = h.rebase(OPERATOR_NEW);
        let p = h.call(f, Conv::Cdecl, &[0xfc]);
        let v = if p == 0 { 0 } else { tweak_ctor(h, p) };
        h.write(g, v);
    }

    /// `MorphManager_create()` (`0x006abe40`): `g_MorphManager = new(0x28)`
    /// zeroed (`+0x20` an `f32` 0) when there is none.
    pub fn morph_create(h: &mut impl Host) {
        let g = h.rebase(G_MORPH);
        if h.read::<u32>(g) != 0 {
            return;
        }
        let f = h.rebase(OPERATOR_NEW);
        let p = h.call(f, Conv::Cdecl, &[0x28]);
        if p != 0 {
            for at in (0..0x28).step_by(4) {
                h.write(p + at, 0u32);
            }
        }
        h.write(g, p);
    }

    /// `CharacterLightingTweakUtil::CharacterLightingTweakUtil(tw)`
    /// (`0x006a6610`, fastcall) -> `tw`: off (`+0`), the text at `+4`,
    /// step 0.02 (`+0xdc`), 1280x720 (`+0xe0`), the four intensities
    /// 1.0 (`+0xcc..0xd8`), `+0x94..0xc0` cleared.
    pub fn tweak_ctor(h: &mut impl Host, tw: u32) -> u32 {
        h.write(tw, 0u8);
        let f = h.rebase(TEXT_CTOR);
        h.call(f, Conv::Thiscall(tw + 4), &[]);
        h.write(tw + 0xdc, 0x3ca3_d70au32);
        h.write(tw + 0xc0, 0u32);
        h.write(tw + 0xc4, 0u32);
        h.write(tw + 0xc8, 0x100u16);
        h.write(tw + 0xe0, 0x500u32);
        h.write(tw + 0xe4, 0x2d0u32);
        h.write(tw + 0xe8, 0u32);
        h.write(tw + 0xec, 0u32);
        h.write(tw + 0xf0, 0u32);
        h.write(tw + 0xf4, ONE);
        h.write(tw + 0xf8, 0u32);
        let f = h.rebase(MEMSET);
        h.call(f, Conv::Cdecl, &[tw + 0x94, 0, 0x2c]);
        for at in [0xcc, 0xd0, 0xd4, 0xd8] {
            h.write(tw + at, ONE);
        }
        tw
    }

    /// `PLight` `i` of the manager's light set.
    fn light(h: &mut impl Host, cl: u32, i: u32) -> u32 {
        let set: u32 = h.read(cl + 0xf0);
        let lights: u32 = h.read(set + 4);
        h.read(lights.wrapping_add(i.wrapping_mul(4)))
    }

    /// `CharacterLighting::hasIntensity(cl, i)` (`0x006a9ae0`, `ret 4`) ->
    /// 1 when the intensity (3: the ambient `+0x1a4`; else light `i`'s
    /// `+0x24`) is above 0.01 in magnitude, on an initialised manager.
    pub fn has_intensity(h: &mut impl Host, cl: u32, i: u32) -> u32 {
        if h.read::<u8>(cl + 0x1cc) == 0 {
            return 0;
        }
        let bits: u32 = if i == 3 {
            h.read(cl + 0x1a4)
        } else {
            let l = light(h, cl, i);
            h.read(l + 0x24)
        };
        let mag = x87::load(bits).abs();
        u32::from(mag > f64::from(0.01f32))
    }

    /// `CharacterLighting::setPointLight(cl, r, g, b, i)` (`0x006aad20`,
    /// `ret 0x10`): on an initialised manager, light `i` becomes a point
    /// light (type, intensity 1, range 0..100, its node's `+0x20..0x28`
    /// cleared) when it isn't one, and gets the colour `(r, g, b, 1)`.
    pub fn set_point_light(h: &mut impl Host, cl: u32, r: u32, g: u32, b: u32, i: u32) {
        if h.read::<u8>(cl + 0x1cc) == 0 {
            return;
        }
        let l = light(h, cl, i);
        let point = h.rebase(POINT_LIGHT_TYPE);
        if h.read::<u32>(l + 0x18) != point {
            let node: u32 = h.read(l + 0x10);
            h.write(l + 0x24, ONE);
            h.write(l + 0x18, point);
            h.write(l + 0x28, 0u32);
            h.write(l + 0x2c, 0x42c8_0000u32);
            h.write(node + 0x20, 0u32);
            h.write(node + 0x24, 0u32);
            h.write(node + 0x28, 0u32);
        }
        h.write(l, quiet(r));
        h.write(l + 4, quiet(g));
        h.write(l + 8, quiet(b));
        h.write(l + 0xc, ONE);
    }

    /// `CharacterLighting::setLightColor(cl, kind, r, g, b)` (`0x006aa940`,
    /// `ret 0x10`): kind 3 the ambient colour (`+0x190`, alpha 1); else,
    /// on an initialised manager, light `kind`'s colour `+0x124 + 0x10
    /// kind`, and the `PLight`'s when it has the colour light type. Both
    /// end in `applyIntensityMultiplier(kind, 1)`.
    pub fn set_light_color(h: &mut impl Host, cl: u32, kind: u32, r: u32, g: u32, b: u32) {
        if kind == 3 {
            h.write(cl + 0x190, quiet(r));
            h.write(cl + 0x194, quiet(g));
            h.write(cl + 0x198, quiet(b));
            h.write(cl + 0x19c, ONE);
        } else {
            if h.read::<u8>(cl + 0x1cc) == 0 {
                return;
            }
            let l = light(h, cl, kind);
            let at = cl.wrapping_add(kind.wrapping_mul(0x10));
            h.write(at.wrapping_add(0x124), quiet(r));
            h.write(at.wrapping_add(0x128), quiet(g));
            h.write(at.wrapping_add(0x12c), quiet(b));
            if h.read::<u32>(l + 0x18) != h.rebase(COLOR_LIGHT_TYPE) {
                return;
            }
            h.write(l, quiet(r));
            h.write(l + 4, quiet(g));
            h.write(l + 8, quiet(b));
            h.write(l + 0xc, ONE);
        }
        let f = h.rebase(APPLY_INTENSITY);
        h.call(f, Conv::Thiscall(cl), &[kind, 1]);
    }

    /// `CharacterLighting::defaultShadeCoeff(cl)` (`0x006a8c40`, fastcall)
    /// -> `ST0`.
    pub fn shade_coeff_default(h: &mut impl Host, cl: u32) -> f32 {
        default_shade_coeff([
            h.read(cl + 0x1d0),
            h.read(cl + 0x1d4),
            h.read(cl + 0x1d8),
            h.read(cl + 0x1dc),
        ])
    }

    /// `CharacterLighting::resetShadeCoeff(cl, inst)` (`0x006aafa0`, `ret
    /// 4`): `setShadeCoeff(inst, defaultShadeCoeff())`; `eax` is
    /// `setShadeCoeff`'s.
    pub fn reset_shade_coeff(h: &mut impl Host, cl: u32, inst: u32) -> u32 {
        let c = shade_coeff_default(h, cl);
        let f = h.rebase(SET_SHADE_COEFF);
        h.call(f, Conv::Thiscall(cl), &[inst, c.to_bits()])
    }

    /// `CharacterLightingTweakUtil::applyShadeCoeffs(tw)` (`0x006ab720`,
    /// fastcall): when on (`[tw]`), each of the 100 entries of `[tw +
    /// 0xf8]` (instance, coefficient, mode) with an instance gets
    /// `g_CharacterLighting`'s `setShadeCoeff` by its mode: 0 the
    /// default, 1 `0x006a8bc0`'s, 2 its own; other modes nothing.
    pub fn tweak_apply_shade_coeffs(h: &mut impl Host, tw: u32) {
        if h.read::<u8>(tw) == 0 {
            return;
        }
        let cl: u32 = h.read(h.rebase(G_LIGHTING));
        for at in (0..0x4b0).step_by(0xc) {
            let table: u32 = h.read(tw + 0xf8);
            let inst: u32 = h.read(table.wrapping_add(at));
            if inst == 0 {
                continue;
            }
            let mode: u32 = h.read(table.wrapping_add(at + 8));
            let c = match mode {
                0 => shade_coeff_default(h, cl).to_bits(),
                1 => {
                    let f = h.rebase(SHADE_COEFF_ALT);
                    h.call_st0(f, Conv::Thiscall(cl), &[]).to_bits()
                }
                2 => quiet(h.read(table.wrapping_add(at + 4))),
                _ => continue,
            };
            let f = h.rebase(SET_SHADE_COEFF);
            h.call(f, Conv::Thiscall(cl), &[inst, c]);
        }
    }

    /// A `PArray<T>`'s grow-and-push (`ret 8`) -> 0, or 0xd without a
    /// buffer: copies the `count` (`[arr]`, bit 31: not owned) elements of
    /// `words` words to `buffer` (skipping a null slot), appends `*item`,
    /// frees the old buffer when owned and different (`free_null`: also
    /// only when non-null), takes `buffer`, count + 1 (owned).
    /// `0x006a94b0`: one word; `0x006ac380`: two words, `free_null`.
    pub fn array_push(
        h: &mut impl Host,
        arr: u32,
        buffer: u32,
        item: u32,
        words: u32,
        free_null: bool,
    ) -> u32 {
        if buffer == 0 {
            return 0xd;
        }
        let head: u32 = h.read(arr);
        let count = head & 0x7fff_ffff;
        let mut src: u32 = h.read(arr + 4);
        let mut dst = buffer;
        for _ in 0..count {
            if dst != 0 {
                for w in 0..words {
                    let v: u32 = h.read(src + 4 * w);
                    h.write(dst + 4 * w, v);
                }
                src = src.wrapping_add(4 * words);
            }
            dst = dst.wrapping_add(4 * words);
        }
        let slot = buffer.wrapping_add(count.wrapping_mul(4 * words));
        if slot != 0 {
            for w in 0..words {
                let v: u32 = h.read(item + 4 * w);
                h.write(slot + 4 * w, v);
            }
        }
        let old: u32 = h.read(arr + 4);
        let owned = h.read::<u32>(arr) >> 31 == 0;
        if old != buffer && owned && (!free_null || old != 0) {
            let f = h.rebase(PHYRE_FREE);
            h.call(f, Conv::Cdecl, &[old]);
        }
        h.write(arr + 4, buffer);
        h.write(arr, count + 1);
        0
    }

    /// The lighting iterator's advance: from its node (`+0x18`), follows
    /// the links (`[node]`, the end `+0x14` giving 0) until
    /// `ITERATOR_VALID` holds, checking first when `check_first`
    /// (`0x006a8d30`) or only after a step (`0x006a8da0`). With a node,
    /// `PhyreSceneCursor_SetNode(it, node)`'s `eax`; without, the cursor
    /// words `+0..0x10` cleared and 0.
    pub fn iterator_advance(h: &mut impl Host, it: u32, check_first: bool) -> u32 {
        let valid = h.rebase(ITERATOR_VALID);
        if !check_first || h.call(valid, Conv::Thiscall(it), &[]) as u8 == 0 {
            let mut node: u32 = h.read(it + 0x18);
            loop {
                if node != 0 {
                    let next: u32 = h.read(node);
                    let end: u32 = h.read(it + 0x14);
                    node = if next == end { 0 } else { next };
                    h.write(it + 0x18, node);
                }
                if h.call(valid, Conv::Thiscall(it), &[]) as u8 != 0 {
                    break;
                }
            }
        }
        let node: u32 = h.read(it + 0x18);
        if node == 0 {
            for at in (0..0x14).step_by(4) {
                h.write(it + at, 0u32);
            }
            return 0;
        }
        let f = h.rebase(CURSOR_SET_NODE);
        h.call(f, Conv::Thiscall(it), &[node])
    }
}
