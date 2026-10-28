//! M5 stage 1, list `render`, batch 4: the `graphic*` setters the
//! scripts and effects call (fog, specular passes, button icon scale,
//! shadow floats, application floats, morph values, FMV pause, number
//! text), each a forwarder to a manager's method; the character loading
//! queue push; two delegation registrations; a matrix helper.
//!
//! The `f32` arguments these pass on through `fld`/`fstp` come out of the
//! x87 quiet ([`crate::m5r_c::quiet`]): the ports pass the words with a
//! signalling NaN's quiet bit set and every other word unchanged.

use crate::m5r_c::quiet;
use ffx_bridge::Global;

/// `g_PhyreScene`.
pub const SCENE: Global<u32> = Global::new(0x00cc_b9d8);
/// `g_GraphicUi`.
pub const GRAPHIC_UI: Global<u32> = Global::new(0x00cc_c81c);
/// `g_RenderDelegationHelper`.
pub const DELEGATIONS: Global<u32> = Global::new(0x00cc_ca98);
/// The character loading queue's mutex and array (count `+0`, capacity
/// `+4`, data `+8`) at `0x00ccb6e0`.
pub const LOADING_MUTEX: Global<u32> = Global::new(0x00cc_b6d0);
pub const LOADING_QUEUE: u32 = 0x00cc_b6e0;

pub const WAIT_FOR_SINGLE_OBJECT: u32 = 0x00b0_c0f8;
pub const RELEASE_MUTEX: u32 = 0x00b0_c1c0;

pub const MENU_SET_MATERIAL_UNLOADED: u32 = 0x0067_e6a0;
pub const LOADING_QUEUE_RESERVE: u32 = 0x0063_8240;
pub const REGISTER_DELEGATION: u32 = 0x006a_de20;
/// `graphicResetHSV_00637c10`, the delegation `0x00642a10` registers.
pub const RESET_HSV: u32 = 0x0063_7c10;
pub const REMAP_ICONS: u32 = 0x006e_7710;
pub const REMAP_RESET_SCALE: u32 = 0x006e_76b0;
pub const REMAP_SET_SCALE: u32 = 0x006e_7720;
pub const SCENE_RESIZE: u32 = 0x0066_c580;
pub const SPECULAR_MANAGER: u32 = 0x006c_57e0;
pub const SPECULAR_AKAZA_NO_LIGHT: u32 = 0x006c_5b80;
pub const SPECULAR_AKAZA: u32 = 0x006c_5c90;
pub const SPECULAR_NO_LIGHT: u32 = 0x006c_5e30;
pub const SPECULAR_SHADE: u32 = 0x006c_6830;
pub const SHADOW_MANAGER: u32 = 0x006d_2390;
pub const SHADOW_SET_FLOATS: u32 = 0x006c_ea80;
pub const FOG_MANAGER: u32 = 0x006e_32c0;
pub const FOG_SET_COLOUR: u32 = 0x006e_36f0;
pub const FOG_SET_3760: u32 = 0x006e_3760;
pub const FOG_SET_FAR: u32 = 0x006e_3a30;
pub const FOG_SET_NEAR: u32 = 0x006e_3b10;
pub const GET_APPLICATION: u32 = 0x0062_3b90;
pub const SCENE_SET_SIX: u32 = 0x0066_d3b0;
pub const MORPH_MANAGER: u32 = 0x006a_bf40;
pub const MORPH_SET_VERTEX_COLOR: u32 = 0x006a_c0b0;
pub const FMV_MANAGER: u32 = 0x006d_73a0;
pub const SCENE_SET_NUMBER_TEXT: u32 = 0x0066_e5e0;
pub const MATRIX_INVERSE: u32 = 0x0057_3d00;
pub const MATRIX_TRANSPOSE: u32 = 0x004f_93f0;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, f: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Cdecl, args)
    }

    fn method(h: &mut impl Host, f: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Thiscall(this), args)
    }

    /// `f(getter(); args)`'s `eax`.
    fn on_instance(h: &mut impl Host, getter: u32, f: u32, args: &[u32]) -> u32 {
        let this = cdecl(h, getter, &[]);
        method(h, f, this, args)
    }

    fn stdcall_import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, Conv::Stdcall, args)
    }

    /// `graphicMenuSetTextureUnloadedInMaterial(x)`:
    /// `MenuManager_SetMenuUiMaterialUnloaded(g_GraphicUi; x)`.
    pub fn menu_set_texture_unloaded(h: &mut impl Host, x: u32) -> u32 {
        let ui = GRAPHIC_UI.get(h);
        method(h, MENU_SET_MATERIAL_UNLOADED, ui, &[x])
    }

    /// `graphicCharacterLoadingPush_00642980(x)` -> 0: under the loading
    /// mutex, `x` appended to the queue (grown by `0x00638240` to `count +
    /// 2 + count / 8` (signed) when the count passes the capacity; a null
    /// slot is skipped as the original's test does).
    pub fn loading_push(h: &mut impl Host, x: u32) -> u32 {
        let m = LOADING_MUTEX.get(h);
        stdcall_import(h, WAIT_FOR_SINGLE_OBJECT, &[m, 0xffff_ffff]);
        let q = h.rebase(LOADING_QUEUE);
        let mut n: u32 = h.read::<u32>(q).wrapping_add(1);
        h.write(q, n);
        if n.cast_signed() > h.read::<u32>(q + 4).cast_signed() {
            let grown = n
                .wrapping_add(2)
                .wrapping_add(((n.cast_signed()) / 8).cast_unsigned());
            method(h, LOADING_QUEUE_RESERVE, q, &[grown]);
            n = h.read(q);
        }
        let data: u32 = h.read(q + 8);
        let slot = data.wrapping_add(n.wrapping_sub(1).wrapping_mul(4));
        if slot != 0 {
            h.write(slot, x);
        }
        let m = LOADING_MUTEX.get(h);
        stdcall_import(h, RELEASE_MUTEX, &[m]);
        0
    }

    /// `graphicRegisterDelegation_006429f0(a, b, c, d)`:
    /// `RenderDelegationHelper_registerDelegation(helper; a, b, c, d)`.
    pub fn register_delegation(h: &mut impl Host, a: [u32; 4]) -> u32 {
        let helper = DELEGATIONS.get(h);
        method(h, REGISTER_DELEGATION, helper, &a)
    }

    /// `graphicRegisterResetHSV_00642a10(chr)`: the delegation of
    /// `graphicResetHSV_00637c10` with its own argument slot as the 4-byte
    /// data (`registerDelegation(&chr, 4, 0x801, 0x00637c10)`).
    pub fn register_reset_hsv(h: &mut impl Host, slot: u32) -> u32 {
        let helper = DELEGATIONS.get(h);
        let f = h.rebase(RESET_HSV);
        method(h, REGISTER_DELEGATION, helper, &[slot, 4, 0x801, f])
    }

    /// `graphicResetButtonIconScale()` (a tail jump).
    pub fn reset_button_icon_scale(h: &mut impl Host) -> u32 {
        on_instance(h, REMAP_ICONS, REMAP_RESET_SCALE, &[])
    }

    /// `graphicResize_00642cd0()`: `PhyreScene_resize(g_PhyreScene)` (a
    /// tail jump).
    pub fn resize(h: &mut impl Host) -> u32 {
        let scene = SCENE.get(h);
        method(h, SCENE_RESIZE, scene, &[])
    }

    /// `graphicSetAkazaNoLightSpecularPassParam(a0..a8)`: words 3..6 are
    /// floats.
    pub fn akaza_no_light_specular(h: &mut impl Host, a: [u32; 9]) -> u32 {
        let mut w = a;
        for x in &mut w[3..7] {
            *x = quiet(*x);
        }
        on_instance(h, SPECULAR_MANAGER, SPECULAR_AKAZA_NO_LIGHT, &w)
    }

    /// `graphicSetAkazaSpecularPassParam(a0..a11)`: words 3..6 are
    /// floats.
    pub fn akaza_specular(h: &mut impl Host, a: [u32; 12]) -> u32 {
        let mut w = a;
        for x in &mut w[3..7] {
            *x = quiet(*x);
        }
        on_instance(h, SPECULAR_MANAGER, SPECULAR_AKAZA, &w)
    }

    /// `graphicSetNoLightSpecularPassParam(a0..a9)`: words 2, 3, 5, 7, 8
    /// and 9 are floats.
    pub fn no_light_specular(h: &mut impl Host, a: [u32; 10]) -> u32 {
        let mut w = a;
        for k in [2, 3, 5, 7, 8, 9] {
            w[k] = quiet(w[k]);
        }
        on_instance(h, SPECULAR_MANAGER, SPECULAR_NO_LIGHT, &w)
    }

    /// `graphicSetSpecularMapParams(a0, a1, f2, f3, f4)`.
    pub fn specular_map_params(h: &mut impl Host, a: [u32; 5]) -> u32 {
        let w = [a[0], a[1], quiet(a[2]), quiet(a[3]), quiet(a[4])];
        on_instance(h, SPECULAR_MANAGER, SPECULAR_SHADE, &w)
    }

    /// `graphicSetButtonIconScale(x, y)` (floats).
    pub fn set_button_icon_scale(h: &mut impl Host, x: u32, y: u32) -> u32 {
        on_instance(h, REMAP_ICONS, REMAP_SET_SCALE, &[quiet(x), quiet(y)])
    }

    /// `graphicSetShadowFloats_00642fb0(a, b)` (floats):
    /// `ShadowManager_SetFloats401c`.
    pub fn set_shadow_floats(h: &mut impl Host, a: u32, b: u32) -> u32 {
        on_instance(h, SHADOW_MANAGER, SHADOW_SET_FLOATS, &[quiet(a), quiet(b)])
    }

    /// `graphicSetFogColour(r, g, b)` (floats).
    pub fn set_fog_colour(h: &mut impl Host, c: [u32; 3]) -> u32 {
        let w = [quiet(c[0]), quiet(c[1]), quiet(c[2])];
        on_instance(h, FOG_MANAGER, FOG_SET_COLOUR, &w)
    }

    /// The fog's one-float setters (`0x00643180`, `graphicSetFogFar`,
    /// `graphicSetFogNear`): `f(fog manager; x)`.
    pub fn set_fog_float(h: &mut impl Host, f: u32, x: u32) -> u32 {
        on_instance(h, FOG_MANAGER, f, &[quiet(x)])
    }

    /// The application's float setters (`0x006432c0` and the next three):
    /// `[app + off] = x` (`fstp dword`); `eax` the application.
    pub fn set_app_float(h: &mut impl Host, off: u32, x: u32) -> u32 {
        let app = cdecl(h, GET_APPLICATION, &[]);
        h.write(app + off, quiet(x));
        app
    }

    /// `graphicSetSceneSix_00643630` / `_006436a0(f0..f5)`: the scene's
    /// `0x0066d3b0(f0..f5, which)` (floats, then 0 or 1).
    pub fn scene_set_six(h: &mut impl Host, f: [u32; 6], which: u32) -> u32 {
        let scene = SCENE.get(h);
        let mut w = [0; 7];
        for (k, &x) in f.iter().enumerate() {
            w[k] = quiet(x);
        }
        w[6] = which;
        method(h, SCENE_SET_SIX, scene, &w)
    }

    /// `graphicSetMeshVertexColorMorphValue(mesh, value)` (a float):
    /// `0x006ac0b0(MorphManager; mesh, value)`.
    pub fn set_vertex_color_morph(h: &mut impl Host, mesh: u32, value: u32) -> u32 {
        on_instance(
            h,
            MORPH_MANAGER,
            MORPH_SET_VERTEX_COLOR,
            &[mesh, quiet(value)],
        )
    }

    /// `graphicSetPauseVideoAtEndFrame(on)`: the FMV manager's byte
    /// `+0x720` = `on != 0`; `eax` the manager.
    pub fn set_pause_video_at_end(h: &mut impl Host, on: u32) -> u32 {
        let m = cdecl(h, FMV_MANAGER, &[]);
        h.write(m + 0x720, u8::from(on != 0));
        m
    }

    /// `graphicSetNumberText_006438c0(x)`: `PhyreScene_SetNumberText`.
    pub fn set_number_text(h: &mut impl Host, x: u32) -> u32 {
        let scene = SCENE.get(h);
        method(h, SCENE_SET_NUMBER_TEXT, scene, &[x])
    }

    /// `graphicSceneMatrixTranspose_00643b80()` (frame: the inverse at
    /// `ebp - 0x40`, the transpose at `ebp - 0x80`, both dropped): the
    /// scene's camera matrix (`[scene + 0xdc]` + 4, or 0, + 0x70) inverted
    /// and transposed into the frame; the transpose's `eax`.
    pub fn scene_matrix_transpose(h: &mut impl Host, ebp: u32) -> u32 {
        let scene = SCENE.get(h);
        let cam: u32 = h.read(scene + 0xdc);
        let base = if cam == 0 { 0 } else { cam + 4 };
        let inv = cdecl(h, MATRIX_INVERSE, &[ebp - 0x40, base + 0x70]);
        cdecl(h, MATRIX_TRANSPOSE, &[ebp - 0x80, inv])
    }
}
