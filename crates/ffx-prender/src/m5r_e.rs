//! M5 stage 1, list `render`, batch 5: more of the scene's small methods
//! and the `graphic*` forwarders the scripts call (mesh visibility, the
//! soft reset and the debug tweak switches, shadow solution reset,
//! texture animation and video, radial lines, wave strength), the scene
//! cursors' constructors, two word-array allocators, the vector
//! allocator, ATL's last-error throw, the loading-cluster world switch,
//! the water texture load and the font texture fix-up.
//!
//! `f32` words passed through the x87 are quieted as the FPU does
//! ([`crate::m5r_c::quiet`]).

use crate::m5r_c::quiet;
use ffx_bridge::Global;

/// `g_PhyreScene`.
pub const SCENE: Global<u32> = Global::new(0x00cc_b9d8);
/// `g_PhyreScene3D` (a byte).
pub const SCENE_3D: Global<u8> = Global::new(0x00cc_b9c0);

pub const GET_LAST_ERROR: u32 = 0x00b0_c030;
/// `MSVCP110!std::_Xlength_error(const char*)` and `std::_Xbad_alloc()`
/// (cdecl; both throw).
pub const XLENGTH_ERROR: u32 = 0x00b0_c29c;
pub const XBAD_ALLOC: u32 = 0x00b0_c2a0;
/// The vector's length-error message.
pub const VECTOR_TOO_LONG: u32 = 0x00b3_4498;

pub const SCENE_SET_MESHES_VISIBLE: u32 = 0x0066_e7b0;
pub const TEXT_SET_ENABLED: u32 = 0x006f_68f0;
pub const TEXT_RESIZE: u32 = 0x006f_6670;
/// The debug tweak utilities' getters, in the order `0x006b49c0` calls
/// them, and their `setEnabled` methods.
pub const TWEAK_GETTERS: [u32; 9] = [
    0x006a_6e40,
    0x0069_2bf0,
    0x006b_6330,
    0x006b_6360,
    0x006b_6340,
    0x006b_6320,
    0x006b_62f0,
    0x006b_6310,
    0x006b_6300,
];
pub const TWEAK_GETTER_LAST: u32 = 0x006b_6350;
pub const TWEAK_SET_ENABLED: [u32; 8] = [
    0x006a_a5f0,
    0x0069_84f0,
    0x006b_b670,
    0x006b_b880,
    0x006b_b6d0,
    0x006b_b5f0,
    0x006b_af60,
    0x006b_b480,
];
pub const DEBUG_MENU_MANAGER: u32 = 0x006b_49b0;
pub const DISABLE_TWEAKS: u32 = 0x006b_49c0;
pub const SHADOW_MANAGER: u32 = 0x006d_2390;
pub const SHADOW_RESET_SOLUTION: u32 = 0x006c_eb10;
pub const SHADOW_RESET_IF: u32 = 0x006c_e0c0;
pub const TEXANIM_MANAGER: u32 = 0x006b_02e0;
pub const TEXANIM_SET_CHAR: u32 = 0x006b_3060;
pub const SCENE_TEXTURE_VIDEO_PLAY: u32 = 0x0067_0750;
pub const RADIAL_LINE_UPDATE: u32 = 0x006f_8f00;
pub const BLUR_EFFECT: u32 = 0x006c_3580;
pub const BLUR_RADIAL_LINE: u32 = 0x006c_3fd0;
pub const FMV_MANAGER: u32 = 0x006d_73a0;
pub const FMV_CURRENT_FRAME: u32 = 0x006d_91b0;
pub const SCENE_SET_WAVE_STRENGTH: u32 = 0x0066_e850;
pub const CURSOR_SEEK2: u32 = 0x0065_ebd0;
pub const CURSOR_SEEK3: u32 = 0x0065_ec40;
pub const CURSOR_SEEK4: u32 = 0x0065_ecb0;
/// The class descriptors the three cursors look for.
pub const CURSOR_CLASS2: u32 = 0x00c9_22c0;
pub const CURSOR_CLASS3: u32 = 0x00ca_5ff0;
pub const CURSOR_CLASS4: u32 = 0x00ca_5268;
pub const PHYRE_MALLOC_ALIGNED: u32 = 0x0042_fb80;
pub const THROW: u32 = 0x0064_fa50;
pub const OPERATOR_NEW: u32 = 0x0063_0310;
pub const WORLD_REMOVE_CLUSTER: u32 = 0x0044_3b70;
pub const WORLD_ADD_CLUSTER: u32 = 0x0044_3250;
pub const UNIFY_FILENAME: u32 = 0x0067_9820;
pub const SCENE_LOAD_VFX_TEXTURE_ASYNC: u32 = 0x0066_5f60;
/// The water effect texture list's name.
pub const WATER_TEXTURES: u32 = 0x00b4_3ba4;
pub const ASYNC_LOAD_MANAGER: u32 = 0x0069_1b40;
pub const GET_APPLICATION: u32 = 0x0062_3b90;
pub const FIXUP_CLUSTERS: u32 = 0x0062_3580;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::Mem;
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, f: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Cdecl, args)
    }

    fn method(h: &mut impl Host, f: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Thiscall(this), args)
    }

    fn byte(h: &impl Mem, at: u32) -> u8 {
        h.read(at)
    }

    fn word(h: &impl Mem, at: u32) -> u32 {
        h.read(at)
    }

    fn import(h: &mut impl Host, slot: u32, conv: Conv, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, conv, args)
    }

    /// `graphicSetMeshesVisible_00643df0(on)`:
    /// `PhyreScene_SetMeshesVisible(g_PhyreScene; on != 0)`.
    pub fn set_meshes_visible(h: &mut impl Host, on: u32) -> u32 {
        let scene = SCENE.get(h);
        method(h, SCENE_SET_MESHES_VISIBLE, scene, &[u32::from(on != 0)])
    }

    /// `PostProcessTweakUtil_setEnabled_006984f0(this; on)` (`ret 4`, the
    /// byte): enabled when `on` and the byte `+4` are set; stored at
    /// `+0x180`, handed to the text (`TextUtil_setEnabled(this + 8)`),
    /// and when enabled the text resized to `+0x188` x `+0x18c`.
    pub fn post_tweak_set_enabled(h: &mut impl Host, this: u32, on: u32) -> u32 {
        let en = u32::from(on & 0xff != 0 && byte(h, this + 4) != 0);
        h.write(this + 0x180, en.to_le_bytes()[0]);
        let r = method(h, TEXT_SET_ENABLED, this + 8, &[en]);
        if byte(h, this + 0x180) == 0 {
            return r;
        }
        let (w, ht) = (word(h, this + 0x188), word(h, this + 0x18c));
        method(h, TEXT_RESIZE, this + 8, &[w, ht])
    }

    /// `DebugMenuManager_disableTweaks_006b49c0()` (reads no `ecx`): the
    /// tweak utilities (character lighting, post-process, ...) fetched in
    /// order, two more getters run, then each `setEnabled(false)`; the
    /// last call's `eax`.
    pub fn disable_tweaks(h: &mut impl Host) -> u32 {
        let mut inst = [0; 8];
        for (k, &g) in TWEAK_GETTERS.iter().enumerate() {
            let r = cdecl(h, g, &[]);
            if let Some(slot) = inst.get_mut(k) {
                *slot = r;
            }
        }
        cdecl(h, TWEAK_GETTER_LAST, &[]);
        let mut r = 0;
        for (&f, &this) in TWEAK_SET_ENABLED.iter().zip(&inst) {
            r = method(h, f, this, &[0]);
        }
        r
    }

    /// `PhyreScene_softResetScene(scene)`: the debug tweaks disabled (on
    /// the debug menu manager), the byte `+0xf9c4` cleared,
    /// `g_PhyreScene3D` set, the floats `+0xf9bc`/`+0xf9c0` zeroed; the
    /// disable's `eax`.
    pub fn soft_reset(h: &mut impl Host, scene: u32) -> u32 {
        let dmm = cdecl(h, DEBUG_MENU_MANAGER, &[]);
        let r = method(h, DISABLE_TWEAKS, dmm, &[]);
        h.write(scene + 0xf9c4, 0_u8);
        SCENE_3D.set(h, 1);
        h.write(scene + 0xf9bc, 0_u32);
        h.write(scene + 0xf9c0, 0_u32);
        r
    }

    /// `ShadowManager_resetIfSolution_006ce0c0(this; x)` (`ret 4`): with
    /// the solution `+0x4050` at 0x203b, a tail jump to
    /// `ShadowManager_ResetSolution(this; x)`; otherwise the solution.
    pub fn shadow_reset_if_solution(h: &mut impl Host, this: u32, x: u32) -> u32 {
        let s = word(h, this + 0x4050);
        if s.cast_signed() > 0 && s == 0x203b {
            return method(h, SHADOW_RESET_SOLUTION, this, &[x]);
        }
        s
    }

    /// `graphicShadowReset_00643e80(x)` -> 1.
    pub fn shadow_reset(h: &mut impl Host, x: u32) -> u32 {
        let m = cdecl(h, SHADOW_MANAGER, &[]);
        method(h, SHADOW_RESET_IF, m, &[x]);
        1
    }

    /// `graphicSetAnimatedMaterialsChar_006440e0(a0..a10)`:
    /// `TextureAnimationManager_setAnimatedMaterialsChar(a0, a1, a2,
    /// f3..f6, a9, a10)` (words 7 and 8 are never read).
    pub fn set_animated_materials_char(h: &mut impl Host, a: [u32; 11]) -> u32 {
        let w = [
            a[0],
            a[1],
            a[2],
            quiet(a[3]),
            quiet(a[4]),
            quiet(a[5]),
            quiet(a[6]),
            a[9],
            a[10],
        ];
        let m = cdecl(h, TEXANIM_MANAGER, &[]);
        method(h, TEXANIM_SET_CHAR, m, &w)
    }

    /// `graphicTextureVideoPlay(x)`: `PhyreScene_textureVideoPlay(x == 1)`.
    pub fn texture_video_play(h: &mut impl Host, x: u32) -> u32 {
        let scene = SCENE.get(h);
        method(h, SCENE_TEXTURE_VIDEO_PLAY, scene, &[u32::from(x == 1)])
    }

    /// `BlurEffect_updateRadialLine_006c3fd0(this; a, b, f, g)` (`ret
    /// 0x10`): the radial line `+0x1c`'s `0x006f8f00(a, b, f, g)`.
    pub fn blur_update_radial_line(h: &mut impl Host, this: u32, a: [u32; 4]) -> u32 {
        let line = word(h, this + 0x1c);
        method(
            h,
            RADIAL_LINE_UPDATE,
            line,
            &[a[0], a[1], quiet(a[2]), quiet(a[3])],
        )
    }

    /// `graphicUpdateRadialLineLogic(on, b, f, g)`: the blur effect's
    /// [`blur_update_radial_line`] with `on != 0`.
    pub fn update_radial_line_logic(h: &mut impl Host, a: [u32; 4]) -> u32 {
        let w = [u32::from(a[0] != 0), a[1], quiet(a[2]), quiet(a[3])];
        let b = cdecl(h, BLUR_EFFECT, &[]);
        method(h, BLUR_RADIAL_LINE, b, &w)
    }

    /// `graphicVideoGetElapsedFrame()` -> the FMV's current frame less its
    /// start (`+0x6e0`).
    pub fn video_elapsed_frame(h: &mut impl Host) -> u32 {
        let m = cdecl(h, FMV_MANAGER, &[]);
        let start = word(h, m + 0x6e0);
        let m = cdecl(h, FMV_MANAGER, &[]);
        method(h, FMV_CURRENT_FRAME, m, &[]).wrapping_sub(start)
    }

    /// `graphicSetWaveStrength_00645e40()`: a tail jump to
    /// `PhyreScene_setWaveStrength(g_PhyreScene)`.
    pub fn set_wave_strength(h: &mut impl Host) -> u32 {
        let scene = SCENE.get(h);
        method(h, SCENE_SET_WAVE_STRENGTH, scene, &[])
    }

    /// The scene cursors' constructors (`this; cluster`, `ret 4`) ->
    /// `this`: five words cleared, the cluster's list `+0x1c` and its
    /// first node (0 when empty), the class `class`; then `seek(this)`.
    pub fn cursor_ctor(h: &mut impl Host, this: u32, cluster: u32, class: u32, seek: u32) -> u32 {
        for k in 0..5 {
            h.write(this + 4 * k, 0_u32);
        }
        let list = cluster.wrapping_add(0x1c);
        h.write(this + 0x14, list);
        let first = word(h, list);
        h.write(this + 0x18, if first == list { 0 } else { first });
        let class = h.rebase(class);
        h.write(this + 0x1c, class);
        method(h, seek, this, &[]);
        this
    }

    /// The word-array allocators `0x0064f7f0`/`0x0064f860(n, src, count)`
    /// -> `n` words (`Phyre_PhyreMalloc_0042fb80(4n, 4)`), the first
    /// `count` copied from `src`, the rest zeroed; 0 for `n == 0` or a
    /// failed allocation.
    pub fn word_array_alloc(h: &mut impl Host, n: u32, src: u32, count: u32) -> u32 {
        if n == 0 {
            return 0;
        }
        let p = cdecl(h, PHYRE_MALLOC_ALIGNED, &[n.wrapping_mul(4), 4]);
        if p == 0 {
            return 0;
        }
        let mut s = src;
        for k in 0..count {
            let d = p.wrapping_add(4 * k);
            if d != 0 {
                let v = word(h, s);
                h.write(d, v);
                s = s.wrapping_add(4);
            }
        }
        let rest = n.wrapping_sub(count);
        for k in 0..rest {
            h.write(p.wrapping_add(4 * (count + k)), 0_u32);
        }
        p
    }

    /// `AtlThrowLastError_0064fa70()`: `GetLastError()` as an `HRESULT`
    /// (`0x8007xxxx` when positive) thrown.
    pub fn throw_last_error(h: &mut impl Host) -> u32 {
        let e = import(h, GET_LAST_ERROR, Conv::Stdcall, &[]);
        let hr = if e.cast_signed() > 0 {
            (e & 0xffff) | 0x8007_0000
        } else {
            e
        };
        cdecl(h, THROW, &[hr])
    }

    /// `StdVector_allocate_00654d40`/`_00654db0(this; n)` (`ret 4`) -> `al`
    /// (in the rest of `eax` what the original leaves): the three words
    /// cleared; `n == 0`: false; more than `0x3fffffff`: `_Xlength_error`;
    /// `operator_new(4n)` (none: `_Xbad_alloc`) as begin, end and begin +
    /// 4n for the capacity; true.
    pub fn vector_allocate(h: &mut impl Host, this: u32, n: u32) -> u32 {
        h.write(this, 0_u32);
        h.write(this + 4, 0_u32);
        h.write(this + 8, 0_u32);
        if n == 0 {
            return 0;
        }
        if n > 0x3fff_ffff {
            let text = h.rebase(VECTOR_TOO_LONG);
            return import(h, XLENGTH_ERROR, Conv::Cdecl, &[text]);
        }
        let p = cdecl(h, OPERATOR_NEW, &[n.wrapping_mul(4)]);
        if p == 0 {
            return import(h, XBAD_ALLOC, Conv::Cdecl, &[]);
        }
        h.write(this, p);
        h.write(this + 4, p);
        let cap = p.wrapping_add(n.wrapping_mul(4));
        h.write(this + 8, cap);
        (cap & !0xff) | 1
    }

    /// `PhyreScene_restoreLoadingCluster_0065a590(scene)`: the loading
    /// cluster `+0x28c4` put back into the world `+0xbc`: with `+0xf9d1`
    /// set (cleared) only added there, else with `+0xf9d0` set moved from
    /// `+0x7c`; `+0xf9d0` cleared.
    pub fn restore_loading_cluster(h: &mut impl Host, scene: u32) {
        let c = scene + 0x28c4;
        if byte(h, scene + 0xf9d1) != 0 {
            h.write(scene + 0xf9d1, 0_u8);
            method(h, WORLD_ADD_CLUSTER, scene + 0xbc, &[c]);
            h.write(scene + 0xf9d0, 0_u8);
            return;
        }
        if byte(h, scene + 0xf9d0) == 0 {
            return;
        }
        method(h, WORLD_REMOVE_CLUSTER, scene + 0x7c, &[c]);
        method(h, WORLD_ADD_CLUSTER, scene + 0xbc, &[c]);
        h.write(scene + 0xf9d0, 0_u8);
    }

    /// `PhyreScene_loadWaterTexture_00665fd0(scene)` (frame: the name at
    /// `ebp - 0x104`): the water texture list unified and loaded
    /// (`PhyreScene_loadVFXTextureAsync(3, name)`), its `eax`.
    pub fn load_water_texture(h: &mut impl Host, ebp: u32, scene: u32) -> u32 {
        let buf = ebp - 0x104;
        let name = h.rebase(WATER_TEXTURES);
        cdecl(h, UNIFY_FILENAME, &[name, buf, 0x100]);
        method(h, SCENE_LOAD_VFX_TEXTURE_ASYNC, scene, &[3, buf])
    }

    /// `PhyreScene_onFontTextureLoaded(cluster)`: the async loader and the
    /// application fetched, then `PApplication_FixupClusters(&cluster, 1)`
    /// on its own argument slot `slot`; its `eax`.
    pub fn on_font_texture_loaded(h: &mut impl Host, slot: u32) -> u32 {
        cdecl(h, ASYNC_LOAD_MANAGER, &[]);
        cdecl(h, GET_APPLICATION, &[]);
        cdecl(h, FIXUP_CLUSTERS, &[slot, 1])
    }
}
