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

/// `strstr`'s import slot (cdecl `(s, sub)`).
pub const STRSTR: u32 = 0x00b0_c378;
/// `"znkd06"`.
pub const ZNKD06: u32 = 0x00b4_2670;
/// `Sg_PLightAllocate` (cdecl `(n)`).
pub const PLIGHT_ALLOCATE: u32 = 0x0083_1950;
/// `Sg_PLightDefine` (cdecl, nine words).
pub const PLIGHT_DEFINE: u32 = 0x0083_19b0;
/// `Sg_PLightSetColor` (cdecl `(i, r, g, b)`).
pub const PLIGHT_SET_COLOR: u32 = 0x0083_1c10;
/// `CharacterLighting_setPointLight_006aad20` (thiscall `(cl, r, g, b, i)`).
pub const SET_POINT_LIGHT: u32 = 0x006a_ad20;
/// `0x005f0ce0` (cdecl `(n, lo, hi)` -> `ST0`): the flash's random level.
pub const FLASH_LEVEL: u32 = 0x005f_0ce0;

/// The flash's brightness from `0x005f0ce0`'s level `r` (as stored, an
/// `f32`): `f32(r * 0.06f + 0.6f) * 255`, truncated (`_ftol2`'s x87 path
/// gives the same as SSE2's `cvttsd2si` in range).
#[must_use]
pub fn flash_color(mem: &impl ffx_bridge::Mem, r: u32) -> u32 {
    let v = x87::add(
        x87::mul(x87::load(r), f64::from(0.06f32)),
        f64::from(0.6f32),
    );
    let v = x87::load(x87::store(v));
    x87::ftol(mem, x87::mul(v, 255.0))
}

pub mod bridge4 {
    //! The originals over game memory.

    use super::{
        Conv, FLASH_LEVEL, Host, PLIGHT_ALLOCATE, PLIGHT_DEFINE, PLIGHT_SET_COLOR, SET_POINT_LIGHT,
        STRSTR, ZNKD06, flash_color, x87,
    };

    /// `CharacterLighting::setMapName(cl, name)` (`0x006ab0d0`, `ret 4`):
    /// on an initialised manager, the name at `+0x220`; a name holding
    /// `"znkd06"` starts the flash (`+0x154` = 0.01) and defines one
    /// scene light (`Sg_PLightAllocate(1)`, `Sg_PLightDefine(0, -17, -1.5,
    /// 30, 30, 3, 3, 3, 0)`); any other name (or none) stops it (-1) and
    /// makes light 3 a black point light.
    pub fn set_map_name(h: &mut impl Host, cl: u32, name: u32) {
        if h.read::<u8>(cl + 0x1cc) == 0 {
            return;
        }
        h.write(cl + 0x220, name);
        if name != 0 {
            let f: u32 = h.read(h.rebase(STRSTR));
            let sub = h.rebase(ZNKD06);
            if h.call(f, Conv::Cdecl, &[name, sub]) != 0 {
                h.write(cl + 0x154, 0x3c23_d70au32);
                let f = h.rebase(PLIGHT_ALLOCATE);
                h.call(f, Conv::Cdecl, &[1]);
                let f = h.rebase(PLIGHT_DEFINE);
                h.call(
                    f,
                    Conv::Cdecl,
                    &[
                        0,
                        0xc188_0000,
                        0xbfc0_0000,
                        0x41f0_0000,
                        0x41f0_0000,
                        3,
                        3,
                        3,
                        0,
                    ],
                );
                return;
            }
        }
        h.write(cl + 0x154, 0xbf80_0000u32);
        let f = h.rebase(SET_POINT_LIGHT);
        h.call(f, Conv::Thiscall(cl), &[0, 0, 0, 3]);
    }

    /// `CharacterLighting::updateFlash(cl, dt)` (`0x006ab630`, `ret 4`): on
    /// an initialised manager, `+0x228` and `+0x224` = 1; while the flash
    /// (`+0x154`) is positive: a level `r = 0x005f0ce0(trunc(flash * 7),
    /// 0.0, 1.0)`, the flash advances by `dt` (back to 0.01 past 1000),
    /// and scene light 0's colour becomes [`flash_color`]`(r)` grey.
    pub fn update_flash(h: &mut impl Host, cl: u32, dt: u32) {
        if h.read::<u8>(cl + 0x1cc) == 0 {
            return;
        }
        h.write(cl + 0x228, 1u32);
        h.write(cl + 0x224, 1u32);
        let flash = x87::load(h.read(cl + 0x154));
        // a NaN flash is not positive
        if flash.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
            return;
        }
        let n = x87::ftol2(x87::mul(flash, 7.0));
        let f = h.rebase(FLASH_LEVEL);
        let r = h.call_st0(f, Conv::Cdecl, &[n, 0, 0x3f80_0000]).to_bits();
        let next = x87::store(x87::add(x87::load(dt), x87::load(h.read(cl + 0x154))));
        h.write(cl + 0x154, next);
        if x87::load(next) > 1000.0 {
            h.write(cl + 0x154, 0x3c23_d70au32);
        }
        let c = flash_color(h, r);
        let f = h.rebase(PLIGHT_SET_COLOR);
        h.call(f, Conv::Cdecl, &[0, c, c, c]);
    }
}

/// `PMatrix4_toWorldMatrix3x4` (thiscall `(dst, m)`, `ret 4`).
pub const TO_WORLD_3X4: u32 = 0x0045_1190;

pub mod bridge5 {
    //! The originals over game memory.

    use super::{Conv, Host, ONE, POINT_LIGHT_TYPE, TO_WORLD_3X4};
    use crate::methods::quiet;

    /// `CharacterLighting::setLightPos(cl, x, y, z, i)` (`0x006aad90`,
    /// `ret 0x10`): on an initialised manager, light `i` becomes a point
    /// light (type, intensity 1, range 0..100, its node's `+0x20..0x28`
    /// cleared) when it isn't one, and its node gets the translation
    /// `(x, y, z)` (an identity 4x4 with that last row, at `m`, 16 words,
    /// through `PMatrix4_toWorldMatrix3x4`), whose `eax` it returns.
    pub fn set_light_pos(h: &mut impl Host, cl: u32, xyz: [u32; 3], i: u32, m: u32) -> u32 {
        if h.read::<u8>(cl + 0x1cc) == 0 {
            return 0;
        }
        let set: u32 = h.read(cl + 0xf0);
        let lights: u32 = h.read(set + 4);
        let l: u32 = h.read(lights.wrapping_add(i.wrapping_mul(4)));
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
        let node: u32 = h.read(l + 0x10);
        for k in 0..12 {
            let v = if k % 5 == 0 { ONE } else { 0 };
            h.write(m + 4 * k, v);
        }
        for (k, v) in xyz.into_iter().enumerate() {
            h.write(m + 0x30 + 4 * k as u32, quiet(v));
        }
        h.write(m + 0x3c, ONE);
        h.call_sym(TO_WORLD_3X4, Conv::Thiscall(node), &[m])
    }
}

/// `PMatrix4_Identity_00451bb0` (cdecl `(out)`).
pub const IDENTITY: u32 = 0x0045_1bb0;
/// `PMatrix4_lookAt` (thiscall `(m, eye, target, up)`, `ret 0xc`).
pub const LOOK_AT: u32 = 0x0045_1c30;
/// `Matrix_RotationX` (cdecl `(out, angle)`).
pub const ROTATION_X: u32 = 0x0066_c8a0;
/// `Phyre_CharacterLighting_applyRotation` (thiscall `(cl, i)`, `ret 4`).
pub const APPLY_ROTATION: u32 = 0x006a_7280;

pub mod bridge6 {
    //! The originals over game memory.

    use super::{
        APPLY_ROTATION, COLOR_LIGHT_TYPE, Conv, Host, IDENTITY, LOOK_AT, ONE, ROTATION_X,
        TO_WORLD_3X4, x87,
    };
    use crate::methods::quiet;

    /// `CharacterLighting::setLightDir(cl, i, x, y, z)` (`0x006aa9f0`,
    /// `ret 0x10`) -> `applyRotation`'s `eax`. On an initialised manager,
    /// for light `i` other than 3, the direction `(-x, -y, z)` goes to
    /// `cl + 0xf4 + 0x10 i`. A colour light (type `0x00c10828`) also gets
    /// its node turned to it: a look-at matrix (`ebp - 0x70`, from the
    /// identity) from the origin (`ebp - 0x10`) to the direction (`ebp -
    /// 0x30`) with up `(0, 1, 0)` (`ebp - 0x20`), or `(0, 0, 1)` when the
    /// direction's y (`x * 0 + y + z * 0` in the x87's order) is not below
    /// 0.9 (then also a rotation by 1.57 at `ebp - 0xb0`, which nothing
    /// reads); then `PMatrix4_toWorldMatrix3x4(node, m)` and
    /// `applyRotation(i)`. Returns 0 where the original's `eax` is
    /// whatever it was (no caller reads it).
    pub fn set_light_dir(h: &mut impl Host, ebp: u32, cl: u32, i: u32, xyz: [u32; 3]) -> u32 {
        if h.read::<u8>(cl + 0x1cc) == 0 || i == 3 {
            return 0;
        }
        let [x, y, z] = xyz;
        let set: u32 = h.read(cl + 0xf0);
        let lights: u32 = h.read(set + 4);
        let ny = x87::store(x87::mul(x87::load(y), -1.0));
        let l: u32 = h.read(lights.wrapping_add(i.wrapping_mul(4)));
        let nx = x87::store(x87::mul(-1.0, x87::load(x)));
        let z = quiet(z);
        let at = cl.wrapping_add(i.wrapping_mul(0x10));
        h.write(at.wrapping_add(0xf4), nx);
        h.write(at.wrapping_add(0xf8), ny);
        h.write(at.wrapping_add(0xfc), z);
        if h.read::<u32>(l + 0x18) != h.rebase(COLOR_LIGHT_TYPE) {
            return 0;
        }
        let (m, eye, target, up) = (ebp - 0x70, ebp - 0x10, ebp - 0x30, ebp - 0x20);
        h.write(up, 0u32);
        h.write(up + 4, ONE);
        h.write(up + 8, 0u32);
        h.write(target, nx);
        h.write(target + 4, ny);
        h.write(target + 8, z);
        h.call_sym(IDENTITY, Conv::Cdecl, &[m]);
        let t = x87::store(x87::mul(x87::load(nx), 0.0));
        let t = x87::store(x87::add(x87::load(t), x87::load(ny)));
        let d = x87::store(x87::add(x87::mul(0.0, x87::load(z)), x87::load(t)));
        // fcomp: below 0.9f (as a double) keeps the y-up basis; not below
        // or unordered takes z up
        let z_up = x87::load(d).partial_cmp(&f64::from(0.9f32)) != Some(std::cmp::Ordering::Less);
        if z_up {
            h.write(up, 0u32);
            h.write(up + 4, 0u32);
            h.write(up + 8, ONE);
        }
        for k in 0..3 {
            h.write(eye + 4 * k, 0u32);
        }
        h.call_sym(LOOK_AT, Conv::Thiscall(m), &[eye, target, up]);
        if z_up {
            h.call_sym(ROTATION_X, Conv::Cdecl, &[ebp - 0xb0, 0x3fc8_f5c3]);
        }
        let node: u32 = h.read(l + 0x10);
        h.call_sym(TO_WORLD_3X4, Conv::Thiscall(node), &[m]);
        h.call_sym(APPLY_ROTATION, Conv::Thiscall(cl), &[i])
    }
}

/// `0x00576630` (thiscall `(obj)`): the constructor of the three embedded
/// objects at `+0`, `+0x50`, `+0xa0` (`fh` guesses
/// `PCluster::operator new`; it is called on `this`).
pub const EMBEDDED_CTOR: u32 = 0x0057_6630;
/// `Phyre_PFreeList_PFreeList` (thiscall `(list, size, a, b, name)`, `ret
/// 0x10`).
pub const FREE_LIST_CTOR: u32 = 0x0049_c480;
/// The free lists' name (`.rdata`).
pub const FREE_LIST_NAME: u32 = 0x00b1_3bc8;
/// `KERNEL32!CreateSemaphoreW`'s import slot (stdcall).
pub const CREATE_SEMAPHORE_W: u32 = 0x00b0_c1e0;
/// `CharacterLighting`'s ambient intensity at construction (1.2f).
const AMBIENT_START: u32 = 0x3f99_999a;

pub mod bridge7 {
    //! The originals over game memory.

    use super::{
        AMBIENT_START, CREATE_SEMAPHORE_W, Conv, EMBEDDED_CTOR, FREE_LIST_CTOR, FREE_LIST_NAME,
        Host, ONE, OPERATOR_NEW,
    };

    /// Points the six words at `at` at `at` (two empty lists' heads, in
    /// the original's order).
    fn self_links(h: &mut impl Host, at: u32) {
        for k in [0, 4, 8, 0x14, 0x10, 0xc] {
            h.write(at + k, at);
        }
    }

    /// `CharacterLighting::CharacterLighting(cl)` (`0x006a63f0`, thiscall)
    /// -> `cl`: three embedded objects (`+0`, `+0x50`, `+0xa0`) each
    /// followed by an empty list (`+0x48`, `+0x98`, `+0xe8`); the flash
    /// timer 0 (`+0x154`); no light set (`+0xf0`); two list heads
    /// (`+0x158`, `+0x1f0`) with free lists behind them (`+0x170`: 0x14
    /// bytes, `+0x208`: 0x1c bytes; 8 per block, align 4); not
    /// initialised (`+0x1cc`); `+0x220..0x230`; intensities 1.0, the
    /// ambient intensity 1.2 (`+0x1a4`), colours and the eight words at
    /// `+0x1d0` 0; and a semaphore (`CreateSemaphoreW(0, 1, 1, 0)`) in a
    /// 4-byte block at `+0x188` (0 when `operator_new` fails).
    pub fn lighting_ctor(h: &mut impl Host, cl: u32) -> u32 {
        h.call_sym(EMBEDDED_CTOR, Conv::Thiscall(cl), &[]);
        for (list, next) in [(0x48, 0x50), (0x98, 0xa0)] {
            let l = cl + list;
            h.write(l, l);
            h.write(l + 4, l);
            h.call_sym(EMBEDDED_CTOR, Conv::Thiscall(cl + next), &[]);
        }
        let l = cl + 0xe8;
        h.write(l, l);
        h.write(l + 4, l);
        h.write(cl + 0x154, 0u32);
        h.write(cl + 0xf0, 0u32);
        let name = h.rebase(FREE_LIST_NAME);
        self_links(h, cl + 0x158);
        h.call_sym(
            FREE_LIST_CTOR,
            Conv::Thiscall(cl + 0x170),
            &[0x14, 8, 4, name],
        );
        h.write(cl + 0x1cc, 0u8);
        self_links(h, cl + 0x1f0);
        h.call_sym(
            FREE_LIST_CTOR,
            Conv::Thiscall(cl + 0x208),
            &[0x1c, 8, 4, name],
        );
        h.write(cl + 0x220, 0u32);
        h.write(cl + 0x224, 1u32);
        h.write(cl + 0x228, 1u32);
        h.write(cl + 0x22c, 0u32);
        h.write(cl + 0x230, 0u8);
        for (at, v) in [
            (0x1a8, ONE),
            (0x1ac, 0),
            (0x1b0, 0),
            (0x1b8, 0),
            (0x1bc, 0),
            (0x1c4, 0),
            (0x1c8, 0),
            (0x1b4, ONE),
            (0x1c0, ONE),
            (0x1a0, ONE),
            (0x1a4, AMBIENT_START),
            (0x190, 0),
            (0x194, 0),
            (0x198, 0),
            (0x19c, 0),
        ] {
            h.write(cl + at, v);
        }
        for k in 0..8 {
            h.write(cl + 0x1d0 + 4 * k, 0u32);
        }
        let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[4]);
        if p == 0 {
            h.write(cl + 0x188, 0u32);
        } else {
            let f: u32 = h.read(h.rebase(CREATE_SEMAPHORE_W));
            let sem = h.call(f, Conv::Stdcall, &[0, 1, 1, 0]);
            h.write(p, sem);
            h.write(cl + 0x188, p);
        }
        cl
    }
}

/// The threshold [`bridge8::sync_intensity`] compares with (0.01f).
pub const SYNC_EPSILON: u32 = 0x00b1_f2b0;
/// `Phyre_CharacterLightingTweakUtil_getLineText` (thiscall `(tw, line,
/// buf)`, `ret 8`).
pub const GET_LINE_TEXT: u32 = 0x006a_8580;
/// `TextUtil_changeLine` (thiscall `(text, line, s)`, `ret 8`).
pub const TEXT_CHANGE_LINE: u32 = 0x006f_6070;
/// `CharacterLightingTweakUtil_applyShadeCoeffs_006ab720` (thiscall
/// `(tw)`).
pub const APPLY_SHADE_COEFFS: u32 = 0x006a_b720;

pub mod bridge8 {
    //! The original over game memory.

    use super::{
        APPLY_INTENSITY, APPLY_SHADE_COEFFS, Conv, G_LIGHTING, GET_LINE_TEXT, Host, MEMSET,
        SYNC_EPSILON, TEXT_CHANGE_LINE, x87,
    };
    use crate::methods::quiet;

    /// `CharacterLightingTweakUtil::syncIntensity(tw)` (`0x006aa260`,
    /// fastcall): when the tweak utility is on (`+0`) and syncing
    /// (`+0xc9`), each of its four intensities (`+0xcc`) that differs from
    /// the lighting's (3: `+0x1a0`; else `+0x1a8 + 0xc i`) by more than
    /// 0.01 (`|a - b|` rounded to `f32` at each step; equal, below or NaN
    /// keeps it) is copied over (light intensities only on an
    /// initialised manager) and applied (`applyIntensityMultiplier(i, 1)`;
    /// lights only with a light set `+0xf0`); the selected line's (`+0xc0`)
    /// text is rebuilt in the frame's buffer (`ebp - 0x84`, 0x80 bytes)
    /// while the utility is on. Any change ends in `applyShadeCoeffs`.
    pub fn sync_intensity(h: &mut impl Host, ebp: u32, tw: u32) {
        if h.read::<u8>(tw) == 0 || h.read::<u8>(tw + 0xc9) == 0 {
            return;
        }
        let eps = x87::load(h.read(h.rebase(SYNC_EPSILON)));
        let cl: u32 = h.read(h.rebase(G_LIGHTING));
        let mut changed = false;
        for i in 0..4u32 {
            let a = quiet(h.read(tw + 0xcc + 4 * i));
            let at = if i == 3 {
                cl + 0x1a0
            } else {
                cl + 0x1a8 + 0xc * i
            };
            let b = quiet(h.read(at));
            let d = x87::store(x87::sub(x87::load(a), x87::load(b)));
            let d = x87::store(x87::load(d).abs());
            // fcomp, then `test ah, 0x41`: below, equal or unordered keeps it
            if x87::load(d).partial_cmp(&eps) != Some(std::cmp::Ordering::Greater) {
                continue;
            }
            if i == 3 {
                h.write(cl + 0x1a0, a);
                h.call_sym(APPLY_INTENSITY, Conv::Thiscall(cl), &[3, 1]);
            } else if h.read::<u8>(cl + 0x1cc) != 0 {
                h.write(at, a);
                if h.read::<u32>(cl + 0xf0) != 0 {
                    h.call_sym(APPLY_INTENSITY, Conv::Thiscall(cl), &[i, 1]);
                }
            }
            changed = true;
            if i == h.read::<u32>(tw + 0xc0) && h.read::<u8>(tw) != 0 {
                let buf = ebp - 0x84;
                h.call_sym(MEMSET, Conv::Cdecl, &[buf, 0, 0x80]);
                h.call_sym(GET_LINE_TEXT, Conv::Thiscall(tw), &[2, buf]);
                h.call_sym(TEXT_CHANGE_LINE, Conv::Thiscall(tw + 4), &[2, buf]);
            }
        }
        if changed {
            h.call_sym(APPLY_SHADE_COEFFS, Conv::Thiscall(tw), &[]);
        }
    }
}

/// `MSVCR110!sprintf_s`'s import slot (cdecl).
pub const SPRINTF_S: u32 = 0x00b0_c394;
/// `"CharacterLighting: %.2f fps"` (`.rdata`).
pub const FPS_FORMAT: u32 = 0x00b4_82bc;
/// `TextUtil_setTitleText` (thiscall `(text, s)`, `ret 4`).
pub const TEXT_SET_TITLE: u32 = 0x006f_6930;
/// `0x006a7850` (thiscall `(tw, x)`, `ret 4`): the tweak utility's
/// per-frame step before its lines are drawn.
pub const TWEAK_STEP: u32 = 0x006a_7850;
/// `TextUtil_render` (thiscall `(text, ctx)`, `ret 4`).
pub const TEXT_RENDER: u32 = 0x006f_65b0;

pub mod bridge9 {
    //! The original over game memory.

    use super::{
        Conv, FPS_FORMAT, GET_LINE_TEXT, Host, MEMSET, SPRINTF_S, TEXT_CHANGE_LINE, TEXT_RENDER,
        TEXT_SET_TITLE, TWEAK_STEP, x87,
    };

    /// `CharacterLightingTweakUtil::render(tw, ctx, x, fps)` (`0x006a9f00`,
    /// `ret 0xc`) -> `TextUtil_render`'s `eax`, or 0 when the utility
    /// isn't shown (`+0xc8`) or there is no context: the title
    /// `"CharacterLighting: %.2f fps"` (`sprintf_s` into the zeroed
    /// 0x20-byte buffer at `ebp - 0x24`, `fps` as a double), the step
    /// `0x006a7850(x)`, lines 6..9 rebuilt (in `ebp - 0xa4`) while the
    /// utility is on (`+0`), then the text drawn into `ctx`.
    pub fn render(h: &mut impl Host, ebp: u32, tw: u32, ctx: u32, x: u32, fps: u32) -> u32 {
        if h.read::<u8>(tw + 0xc8) == 0 || ctx == 0 {
            return 0;
        }
        let title = ebp - 0x24;
        for k in 0..8 {
            h.write(title + 4 * k, 0u32);
        }
        let fps = x87::load(fps).to_bits();
        let f: u32 = h.read(h.rebase(SPRINTF_S));
        let format = h.rebase(FPS_FORMAT);
        h.call(
            f,
            Conv::Cdecl,
            &[title, 0x20, format, fps as u32, (fps >> 32) as u32],
        );
        h.call_sym(TEXT_SET_TITLE, Conv::Thiscall(tw + 4), &[title]);
        h.call_sym(TWEAK_STEP, Conv::Thiscall(tw), &[x]);
        let line = ebp - 0xa4;
        for i in 6..=9 {
            if h.read::<u8>(tw) != 0 {
                h.call_sym(MEMSET, Conv::Cdecl, &[line, 0, 0x80]);
                h.call_sym(GET_LINE_TEXT, Conv::Thiscall(tw), &[i, line]);
                h.call_sym(TEXT_CHANGE_LINE, Conv::Thiscall(tw + 4), &[i, line]);
            }
        }
        h.call_sym(TEXT_RENDER, Conv::Thiscall(tw + 4), &[ctx])
    }
}

/// `Phyre_PRedBlackTree_remove` (thiscall `(tree, node)`, `ret 4`).
pub const TREE_REMOVE: u32 = 0x0057_5000;
/// `Phyre_PFreeList_reset` (thiscall `(list)`).
pub const FREE_LIST_RESET: u32 = 0x0049_cc10;
/// `Phyre_CharacterLighting_setPointLightPosition` (thiscall `(cl, x, y,
/// z, i)`, `ret 0x10`).
pub const SET_POINT_LIGHT_POSITION: u32 = 0x006a_ad90;
/// The `f32` constants `reset` stores: the colour lights' range
/// (`0x00b9232c`), the point lights' range (`0x00b304f8`), the flash
/// timer's off value (`0x00b92340`).
pub const RESET_CONSTANTS: [u32; 3] = [0x00b9_232c, 0x00b3_04f8, 0x00b9_2340];

pub mod bridge10 {
    //! The original over game memory.

    use super::{
        APPLY_INTENSITY, COLOR_LIGHT_TYPE, Conv, FREE_LIST_RESET, Host, ONE, POINT_LIGHT_TYPE,
        RESET_CONSTANTS, SET_POINT_LIGHT_POSITION, TREE_REMOVE,
    };
    use crate::methods::quiet;

    fn light(h: &mut impl Host, cl: u32, i: u32) -> u32 {
        let set: u32 = h.read(cl + 0xf0);
        let lights: u32 = h.read(set + 4);
        h.read(lights.wrapping_add(4 * i))
    }

    /// Light 3 becomes a point light (as `setPointLight` does) unless it
    /// is one; `range` is the point range's bits.
    fn make_point(h: &mut impl Host, l: u32, range: u32) {
        if h.read::<u32>(l + 0x18) == h.rebase(POINT_LIGHT_TYPE) {
            return;
        }
        let node: u32 = h.read(l + 0x10);
        h.write(l + 0x24, ONE);
        h.write(l + 0x18, h.rebase(POINT_LIGHT_TYPE));
        h.write(l + 0x28, 0u32);
        h.write(l + 0x2c, range);
        h.write(node + 0x20, 0u32);
        h.write(node + 0x24, 0u32);
        h.write(node + 0x28, 0u32);
    }

    /// `CharacterLighting::reset(cl)` (`0x006aa010`, fastcall): lights 0..2
    /// become colour lights with the stored colours (`+0x124 + 0x10 i`),
    /// alpha and intensity 1, range 0..`[0x00b9232c]`, and are applied;
    /// their three intensity words (`+0x1a8 + 0xc i`) become 1, 0, 0 (and
    /// apply again on an initialised manager with a light set); the
    /// global and ambient intensities 1; the shade tree (`+0x1f0`) is
    /// emptied and its free list (`+0x208`) reset; on an initialised
    /// manager light 3 becomes a black point light at the origin
    /// (`setPointLightPosition(0, 0, 0, 3)`) with range 0..100; the
    /// flash is off (`[0x00b92340]`), `+0x1d0..0x1ec`, `+0x220`, `+0x22c`
    /// cleared.
    pub fn reset(h: &mut impl Host, cl: u32) {
        let [far, point_range, flash_off] = RESET_CONSTANTS.map(|a| h.rebase(a));
        let far: u32 = h.read(far);
        h.write(cl + 0x230, 0u8);
        for i in 0..3u32 {
            let l = light(h, cl, i);
            h.write(l + 0x18, h.rebase(COLOR_LIGHT_TYPE));
            let at = cl + 0x10 * i;
            for k in 0..3 {
                // through two fld/fstp pairs
                let c = quiet(h.read(at + 0x124 + 4 * k));
                h.write(l + 4 * k, c);
            }
            h.write(l + 0xc, ONE);
            h.write(l + 0x24, ONE);
            h.write(l + 0x28, 0u32);
            h.write(l + 0x2c, far);
            h.call_sym(APPLY_INTENSITY, Conv::Thiscall(cl), &[i, 1]);
            let w = cl + 0xc * i;
            h.write(w + 0x1a8, ONE);
            h.write(w + 0x1b0, 0u32);
            h.write(w + 0x1ac, 0u32);
            if h.read::<u8>(cl + 0x1cc) != 0 {
                h.write(w + 0x1a8, ONE);
                if h.read::<u32>(cl + 0xf0) != 0 {
                    h.call_sym(APPLY_INTENSITY, Conv::Thiscall(cl), &[i, 1]);
                }
            }
        }
        h.write(cl + 0x1a0, ONE);
        h.write(cl + 0x1a4, ONE);
        let tree = cl + 0x1f0;
        loop {
            let node: u32 = h.read(tree + 0xc);
            if node == tree || node == 0 {
                break;
            }
            h.call_sym(TREE_REMOVE, Conv::Thiscall(tree), &[node]);
        }
        h.call_sym(FREE_LIST_RESET, Conv::Thiscall(tree + 0x18), &[]);
        let on = h.read::<u8>(cl + 0x1cc) != 0;
        h.write(cl + 0x22c, 0u32);
        if on {
            let range: u32 = h.read(point_range);
            let l = light(h, cl, 3);
            make_point(h, l, range);
            for k in 0..3 {
                h.write(l + 4 * k, 0u32);
            }
            h.write(l + 0xc, ONE);
            h.call_sym(SET_POINT_LIGHT_POSITION, Conv::Thiscall(cl), &[0, 0, 0, 3]);
            if h.read::<u8>(cl + 0x1cc) != 0 {
                let range: u32 = h.read(point_range);
                let l = light(h, cl, 3);
                make_point(h, l, range);
                h.write(l + 0x28, 0u32);
                h.write(l + 0x2c, range);
            }
        }
        let off: u32 = h.read(flash_off);
        h.write(cl + 0x154, off);
        for k in 0..8 {
            h.write(cl + 0x1d0 + 4 * k, 0u32);
        }
        h.write(cl + 0x220, 0u32);
    }
}

/// `Phyre_PhyreMalloc_0042fb80` (cdecl `(size, align)`).
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
/// `Phyre_PCluster_create` (`(cluster; count, desc, flags)` -> objects,
/// `ret 0xc`; the callers leave the cluster in `ecx`).
pub const CLUSTER_CREATE: u32 = 0x0044_1010;
/// `PLight`'s class descriptor.
pub const PLIGHT_DESC: u32 = 0x00c9_af80;
/// The scene node class `initialize` creates one of per light.
pub const NODE_DESC: u32 = 0x00c9_2090;
/// `CharacterLighting_iteratorSkipInvalid_006a8d30` (thiscall `(it)`).
pub const ITERATOR_BEGIN: u32 = 0x006a_8d30;
/// `CharacterLighting_iteratorNext_006a8da0` (thiscall `(it)`).
pub const ITERATOR_NEXT: u32 = 0x006a_8da0;
/// `PArray_pushWord_006a94b0` (thiscall `(arr, buffer, &item)`, `ret 8`).
pub const ARRAY_PUSH_WORD: u32 = 0x006a_94b0;
/// The `double` -1.0 the first lights' direction is scaled by
/// (`0x00b92350`).
const MINUS_ONE: f64 = -1.0;

pub mod bridge11 {
    //! The original over game memory.

    use super::{
        APPLY_INTENSITY, APPLY_ROTATION, ARRAY_PUSH_WORD, CLUSTER_CREATE, COLOR_LIGHT_TYPE, Conv,
        Host, ITERATOR_BEGIN, ITERATOR_NEXT, MINUS_ONE, NODE_DESC, ONE, PHYRE_MALLOC, PLIGHT_DESC,
        POINT_LIGHT_TYPE, RESET_CONSTANTS, x87,
    };

    const COUNT: u32 = 0x7fff_ffff;

    /// Appends the light in the frame's item cell to the light set,
    /// through a buffer one word longer (`Phyre_PhyreMalloc(4 n + 4, 4)`).
    fn push(h: &mut impl Host, set: u32, item_cell: u32) {
        let n = h.read::<u32>(set) & COUNT;
        let buf = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[4 * n + 4, 4]);
        h.call_sym(ARRAY_PUSH_WORD, Conv::Thiscall(set), &[buf, item_cell]);
    }

    /// One `PLight`, already in the frame's item cell.
    fn add_light(h: &mut impl Host, cl: u32, item_cell: u32) {
        let set: u32 = h.read(cl + 0xf0);
        let [_, point_range, flash_off] = RESET_CONSTANTS.map(|a| h.rebase(a));
        if h.read::<u32>(set) & COUNT < 3 {
            // one of the three directional colour lights
            push(h, set, item_cell);
            let light: u32 = h.read(item_cell);
            h.write(light + 0x18, h.rebase(COLOR_LIGHT_TYPE));
            let set: u32 = h.read(cl + 0xf0);
            let i = (h.read::<u32>(set) & COUNT).wrapping_sub(1);
            h.call_sym(APPLY_INTENSITY, Conv::Thiscall(cl), &[i, 0]);
            let node_desc = h.rebase(NODE_DESC);
            let node = h.call_sym(
                CLUSTER_CREATE,
                Conv::Thiscall(cl + 0x50),
                &[1, node_desc, 0],
            );
            let light: u32 = h.read(item_cell);
            h.write(light + 0x10, node);
            // -0.0 through the item cell (the original's argument slot)
            let z = x87::store(x87::mul(0.0, MINUS_ONE));
            h.write(item_cell, z);
            let set: u32 = h.read(cl + 0xf0);
            let n = h.read::<u32>(set) & COUNT;
            let at = cl.wrapping_add(n.wrapping_mul(0x10));
            h.write(at.wrapping_add(0xe4), z);
            h.write(at.wrapping_add(0xe8), z);
            let off: u32 = h.read(flash_off);
            h.write(at.wrapping_add(0xec), off);
            let set: u32 = h.read(cl + 0xf0);
            let n = h.read::<u32>(set) & COUNT;
            let at = cl.wrapping_add(n.wrapping_mul(0x10));
            for k in [0x114, 0x118, 0x11c] {
                h.write(at.wrapping_add(k), ONE);
            }
            let set: u32 = h.read(cl + 0xf0);
            let i = (h.read::<u32>(set) & COUNT).wrapping_sub(1);
            h.call_sym(APPLY_ROTATION, Conv::Thiscall(cl), &[i]);
        } else {
            // a point light
            push(h, set, item_cell);
            let light: u32 = h.read(item_cell);
            h.write(light + 0x18, h.rebase(POINT_LIGHT_TYPE));
            let node_desc = h.rebase(NODE_DESC);
            let node = h.call_sym(
                CLUSTER_CREATE,
                Conv::Thiscall(cl + 0x50),
                &[1, node_desc, 0],
            );
            let light: u32 = h.read(item_cell);
            h.write(light, 0u32);
            h.write(light + 0x10, node);
            h.write(light + 4, 0u32);
            h.write(light + 8, 0u32);
            h.write(light + 0xc, ONE);
            h.write(light + 0x24, ONE);
            h.write(light + 0x28, 0u32);
            let range: u32 = h.read(point_range);
            h.write(light + 0x2c, range);
        }
    }

    /// `CharacterLighting::initialize(cl, scene)` (`0x006a8f80`, `ret 4`)
    /// -> 0: the light set is the scene's `+0xa4` (`cl + 0xf0`), the scene
    /// at `+0x18c`; `PCluster::create(4, PLight)` on the manager's own
    /// cluster; then each `PLight` of it (a cursor at `ebp - 0x28`, the
    /// original's inline step as in `registerClusterMorphs`) is pushed
    /// onto the set (the light passes through the frame's item cell at
    /// `ebp + 8`, the original's argument slot): the first three become
    /// colour lights facing `(-0, -0, -1)` with white colours, intensity
    /// applied and rotation set; the rest point lights (black, range
    /// 0..100). Each gets a scene node (`PCluster::create(1, ...)` on
    /// `cl + 0x50`). Initialised (`+0x1cc`) when the set holds four.
    /// Also returns the word left in the item cell.
    pub fn initialize(h: &mut impl Host, ebp: u32, cl: u32, scene: u32) -> (u32, u32) {
        let item_cell = ebp + 8;
        h.write(item_cell, scene);
        h.write(cl + 0xf0, scene.wrapping_add(0xa4));
        h.write(cl + 0x18c, scene);
        let desc = h.rebase(PLIGHT_DESC);
        h.call_sym(CLUSTER_CREATE, Conv::Thiscall(cl), &[4, desc, 0]);
        let cur = ebp - 0x28;
        let lists = cl + 0x1c;
        let first: u32 = h.read(lists);
        let first = if first == lists { 0 } else { first };
        for at in [0, 4, 8, 0xc, 0x10] {
            h.write(cur + at, 0u32);
        }
        h.write(cur + 0x14, lists);
        h.write(cur + 0x18, first);
        h.write(cur + 0x1c, desc);
        h.call_sym(ITERATOR_BEGIN, Conv::Thiscall(cur), &[]);
        let mut left: u32 = h.read(cur + 0xc);
        if left != 0 {
            let mut at: u32 = h.read(cur);
            let mut node: u32 = h.read(cur + 4);
            loop {
                let off: u32 = h.read(cur + 0x10);
                let light = off.wrapping_add(at);
                h.write(item_cell, light);
                if light != 0 {
                    add_light(h, cl, item_cell);
                }
                let mut next_block = false;
                loop {
                    let stride: u32 = h.read(cur + 8);
                    at = at.wrapping_add(stride);
                    left -= 1;
                    h.write(cur, at);
                    h.write(cur + 0xc, left);
                    if node == at {
                        node = h.read(node);
                        h.write(cur + 4, node);
                        if left != 0 {
                            continue;
                        }
                        next_block = true;
                    } else if left == 0 {
                        next_block = true;
                    }
                    break;
                }
                if next_block {
                    h.call_sym(ITERATOR_NEXT, Conv::Thiscall(cur), &[]);
                    left = h.read(cur + 0xc);
                    if left == 0 {
                        break;
                    }
                    at = h.read(cur);
                    node = h.read(cur + 4);
                }
            }
        }
        let set: u32 = h.read(cl + 0xf0);
        let four = h.read::<u32>(set) & COUNT == 4;
        h.write(cl + 0x1cc, u8::from(four));
        let word: u32 = h.read(item_cell);
        (0, word)
    }
}

/// `MSVCR110!strstr`'s import slot.
pub const STRSTR_SLOT: u32 = 0x00b0_c378;
/// The three map names whose ambient colour is boosted (`"znkd14"`,
/// `"znkd08"`, `"znkd09"`).
pub const BOOSTED_MAPS: [u32; 3] = [0x00b4_80e8, 0x00b4_80f0, 0x00b4_80f8];
/// `Phyre_Vec3Max_004f84f0` (cdecl `(out, a, b)` -> out).
pub const VEC3_MAX: u32 = 0x004f_84f0;
/// `Phyre_Vec3Min_004f8580` (cdecl `(out, a, b)` -> out).
pub const VEC3_MIN: u32 = 0x004f_8580;
/// The `f32` ambient factor without the flag (1.16, `0x00b482e8`).
pub const AMBIENT_DIM: u32 = 0x00b4_82e8;
/// The `double` constants, by address: the average's 0.333, the boost's
/// 2.6, the intensity's 1.2 and 1.05, the luminance weights 0.3, 0.5, 0.2.
pub const AMBIENT_DOUBLES: [u32; 7] = [
    0x00b4_8310,
    0x00b4_8350,
    0x00b4_8340,
    0x00b4_8338,
    0x00b4_8308,
    0x00b9_2220,
    0x00b3_eb60,
];

pub mod bridge12 {
    //! The original over game memory.

    use super::{
        AMBIENT_DIM, AMBIENT_DOUBLES, APPLY_INTENSITY, BOOSTED_MAPS, Conv, Host, ONE, STRSTR_SLOT,
        VEC3_MAX, VEC3_MIN, x87,
    };
    use crate::methods::quiet;

    fn f(bits: u32) -> f64 {
        x87::load(bits)
    }

    fn double(h: &mut impl Host, at: u32) -> f64 {
        let at = h.rebase(at);
        let lo: u32 = h.read(at);
        let hi: u32 = h.read(at + 4);
        f64::from_bits(u64::from(hi) << 32 | u64::from(lo))
    }

    /// `CharacterLighting::applyAmbient(cl, flag)` (`0x006aa650`, `ret 4`):
    /// `+0x230 = flag`; on an initialised manager lights 0..2 are applied
    /// again and the ambient intensity becomes `(1 - [+0x1ec]) * 1.2 *
    /// [+0x1a0] * (flag ? 1 : 1.16) * 1.05` (`+0x1a4`), the scene's
    /// ambient colour (`scene +0xac`) the ambient colour (`+0x190`) times
    /// it, and `+0x1dc` its luminance (0.3, 0.5, 0.2) times it. On the
    /// maps of [`BOOSTED_MAPS`] the colour's distance from its grey is
    /// first scaled by 2.6 and the result clamped to `[0, 1]`
    /// (`Phyre_Vec3Max`/`Vec3Min` on the frame's vectors: `ebp - 0x4c`,
    /// `-0x3c` (zeros, then the result), `-0x2c` (ones), `-0x1c` the
    /// colour). Every step rounds to `f32` where the original stores.
    /// Returns the word the original leaves in its argument slot (the
    /// luminance), `None` on an uninitialised manager.
    pub fn apply_ambient(h: &mut impl Host, ebp: u32, cl: u32, flag: u8) -> Option<u32> {
        h.write(cl + 0x230, flag);
        if h.read::<u8>(cl + 0x1cc) == 0 {
            return None;
        }
        for i in 0..3 {
            h.call_sym(APPLY_INTENSITY, Conv::Thiscall(cl), &[i, 1]);
        }
        let m = if h.read::<u8>(cl + 0x230) == 0 {
            h.read(h.rebase(AMBIENT_DIM))
        } else {
            ONE
        };
        let [k333, k26, k12, k105, w0, w1, w2] = AMBIENT_DOUBLES.map(|a| double(h, a));
        let (c, v) = (ebp - 0x2c, ebp - 0x1c);
        let col: [u32; 3] = [0x190, 0x194, 0x198].map(|o| quiet(h.read(cl + o)));
        for (k, w) in col.into_iter().enumerate() {
            h.write(c + 4 * k as u32, w);
            h.write(v + 4 * k as u32, w);
        }
        // junk read: [ebp-0x20] never written; 0 (brief 2026-10-24): it
        // goes to the colour's unused fourth word, which nothing reads
        h.write(v + 0xc, 0u32);
        let mut col = col;
        let name: u32 = h.read(cl + 0x220);
        if name != 0 {
            let strstr: u32 = h.read(h.rebase(STRSTR_SLOT));
            let mut boosted = false;
            for s in BOOSTED_MAPS {
                let name: u32 = h.read(cl + 0x220);
                let s = h.rebase(s);
                if h.call(strstr, Conv::Cdecl, &[name, s]) != 0 {
                    boosted = true;
                    break;
                }
            }
            if boosted {
                let [x, y, z] = col;
                let s1 = x87::store(x87::add(f(y), f(x)));
                let s2 = x87::store(x87::add(f(s1), f(z)));
                let avg = x87::store(x87::mul(f(s2), k333));
                let d = col.map(|e| x87::store(x87::sub(f(e), f(avg))));
                for (k, w) in d.into_iter().enumerate() {
                    h.write(c + 4 * k as u32, w);
                }
                let a = x87::store(x87::mul(f(d[0]), k26));
                let b = x87::store(x87::mul(f(d[1]), k26));
                let e = x87::store(x87::mul(k26, f(d[2])));
                let a = x87::store(x87::add(f(a), f(avg)));
                let b = x87::store(x87::add(f(b), f(avg)));
                let e = x87::store(x87::add(f(avg), f(e)));
                for (k, w) in [a, b, e].into_iter().enumerate() {
                    h.write(v + 4 * k as u32, w);
                    h.write(c + 4 * k as u32, ONE);
                    h.write(ebp - 0x3c + 4 * k as u32, 0u32);
                }
                let r = h.call_sym(VEC3_MAX, Conv::Cdecl, &[ebp - 0x4c, ebp - 0x3c, v]);
                h.call_sym(VEC3_MIN, Conv::Cdecl, &[ebp - 0x3c, r, c]);
                col = [0, 4, 8].map(|o| quiet(h.read(ebp - 0x3c + o)));
                for (k, w) in col.into_iter().enumerate() {
                    h.write(v + 4 * k as u32, w);
                }
            }
        }
        let t = x87::sub(1.0, f(h.read(cl + 0x1ec)));
        let t = x87::mul(t, k12);
        let t = x87::mul(t, f(h.read(cl + 0x1a0)));
        let t = x87::mul(t, f(m));
        let level = x87::store(x87::mul(t, k105));
        h.write(cl + 0x1a4, level);
        let [x, y, z] = col;
        let scene: u32 = h.read(cl + 0x18c);
        if scene != 0 {
            let p = [
                x87::store(x87::mul(f(x), f(level))),
                x87::store(x87::mul(f(y), f(level))),
                x87::store(x87::mul(f(level), f(z))),
            ];
            for (k, w) in p.into_iter().enumerate() {
                h.write(c + 4 * k as u32, w);
            }
            for (k, w) in p.into_iter().enumerate() {
                h.write(scene + 0xac + 4 * k as u32, w);
            }
        }
        let a = x87::store(x87::mul(f(x), w0));
        let s = x87::store(x87::add(f(a), x87::mul(f(y), w1)));
        let lum = x87::store(x87::add(f(s), x87::mul(f(z), w2)));
        let level: u32 = h.read(cl + 0x1a4);
        h.write(cl + 0x1dc, x87::store(x87::mul(f(lum), f(level))));
        Some(lum)
    }
}

/// `SearchTree_findUnsigned_006a5ec0` (thiscall `(tree, &key)` -> node).
pub const TREE_FIND_UNSIGNED: u32 = 0x006a_5ec0;
/// `PArray_Data_004d45a0` (thiscall `(array)` -> its elements).
pub const ARRAY_DATA: u32 = 0x004d_45a0;
/// `Phyre_PCluster_deleteObject` (thiscall `(cluster, obj)`, `ret 4`).
pub const DELETE_OBJECT: u32 = 0x0044_1240;
/// `Phyre_PFreeList_deallocate` (thiscall `(list, p)`, `ret 4`).
pub const FREE_LIST_DEALLOCATE: u32 = 0x0049_c980;

pub mod bridge13 {
    //! The original over game memory.

    use super::{
        ARRAY_DATA, Conv, DELETE_OBJECT, FREE_LIST_DEALLOCATE, G_TWEAK, Host, TREE_FIND_UNSIGNED,
        TREE_REMOVE,
    };
    use crate::PHYRE_FREE;

    const COUNT: u32 = 0x7fff_ffff;

    /// A `PArray`'s elements as the original's inline code finds them:
    /// more than one at `[arr + 4]`, one inline at `arr + 4`, none 0.
    fn elements(h: &mut impl Host, arr: u32) -> u32 {
        let n = h.read::<u32>(arr) & COUNT;
        match n {
            0 => 0,
            1 => arr + 4,
            _ => h.read(arr + 4),
        }
    }

    /// `CharacterLighting::destroyUniqueMaterialParameters(cl, inst)`
    /// (`0x006a8210`, `ret 4`): undoes `instantiateUniqueMaterialParameters`
    /// for the instance's material list (`inst + 0x10`, the key at `ebp -
    /// 8` for the shade tree's `findUnsigned`): when the tree has it the
    /// instance gets back the node's saved list (`+0x10`; `[inst] + 0x30`
    /// when that is 0);
    /// then each element's two objects are deleted from the manager's
    /// cluster (`+0xa0`) and the element cleared, the list's storage freed
    /// (when owned) and its count zeroed, the tree node removed and freed
    /// (`+0x170` free list), and the list itself deleted. In any case the
    /// tweak utility forgets the instance (its 100 shade entries at `[tw +
    /// 0xf8]`: `{0, 0.0, 2}`).
    pub fn destroy_unique(h: &mut impl Host, ebp: u32, cl: u32, inst: u32) {
        let list: u32 = h.read(inst + 0x10);
        let key = ebp - 8;
        h.write(ebp - 4, cl);
        h.write(key, list);
        let node = h.call_sym(TREE_FIND_UNSIGNED, Conv::Thiscall(cl + 0x158), &[key]);
        if node != 0 && node.wrapping_add(0x10) != 0 {
            // the saved list, else the instance's default one
            let saved: u32 = h.read(node.wrapping_add(0x10));
            let back = if saved == 0 {
                h.read::<u32>(inst).wrapping_add(0x30)
            } else {
                saved
            };
            h.write(inst + 0x10, back);
            let n = h.read::<u32>(list) & COUNT;
            h.write(key, n);
            for k in 0..n {
                let data = h.call_sym(ARRAY_DATA, Conv::Thiscall(list), &[]);
                let e: u32 = h.read(data.wrapping_add(4 * k));
                let other: u32 = h.read(e + 4);
                h.call_sym(DELETE_OBJECT, Conv::Thiscall(cl + 0xa0), &[e]);
                h.call_sym(DELETE_OBJECT, Conv::Thiscall(cl + 0xa0), &[other]);
                if k < h.read::<u32>(list) & COUNT {
                    let at = elements(h, list);
                    h.write(at.wrapping_add(4 * k), 0u32);
                }
            }
            let count: u32 = h.read(list);
            if count != 0 {
                let data = elements(h, list);
                if data != list + 4 && count >> 31 == 0 {
                    h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
                }
                h.write(list, 0u32);
            }
            let tree = cl + 0x158;
            let mut at: u32 = h.read(cl + 0x164);
            while at != tree {
                let k: u32 = h.read(at + 0xc);
                if k == list {
                    h.call_sym(TREE_REMOVE, Conv::Thiscall(tree), &[at]);
                    h.call_sym(FREE_LIST_DEALLOCATE, Conv::Thiscall(tree + 0x18), &[at]);
                    break;
                }
                // larger keys to the left
                let side = u32::from(k < list);
                at = h.read(at + 4 * side);
            }
            h.call_sym(DELETE_OBJECT, Conv::Thiscall(cl + 0xa0), &[list]);
        }
        let tw: u32 = h.read(h.rebase(G_TWEAK));
        if h.read::<u8>(tw) == 0 {
            return;
        }
        let entries: u32 = h.read(tw + 0xf8);
        for i in 0..100u32 {
            if h.read::<u32>(entries + 0xc * i) == inst {
                h.write(entries + 0xc * i, 0u32);
                let entries: u32 = h.read(tw + 0xf8);
                h.write(entries + 0xc * i + 4, 0u32);
                let entries: u32 = h.read(tw + 0xf8);
                h.write(entries + 0xc * i + 8, 2u32);
                return;
            }
        }
    }
}
