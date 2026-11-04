//! The game state the remaster's renderer keeps for the scene: fog,
//! ambient and background colours (on the `PhyreEngine` application), the
//! screen fade (on the post-process manager), the cameras (on the scene),
//! the 2D layers' visibility, the characters' skeleton matrices and
//! lighting parameters, and the `graphic*` wrappers the game calls them
//! through. These are what `ffx-render` reads from the scene stream
//! (`docs/formats/render.md`, "Scene state").
//!
//! Floats the originals move with `fld`/`fstp` go through
//! [`x87::round_trip`] (a signalling NaN comes out quiet).

/// The originals, over game memory.
#[allow(clippy::many_single_char_names)] // colour and vector components
pub mod bridge {
    use crate::host::{Conv, Host};
    use ffx_bridge::{Global, Mem, x87};

    pub use crate::managers::bridge::POST_PROCESS;
    pub use crate::scene::bridge::SCENE;

    /// `g_PhyreApplication` (`Phyre_PFramework_PApplication_GetApplication`).
    pub const APPLICATION: Global<u32> = Global::new(0x00cc_9cd8);
    /// `g_CharacterLighting` (`Phyre_CharacterLighting_GetInstance`).
    pub const CHARACTER_LIGHTING: Global<u32> = Global::new(0x00cc_ca84);
    /// `g_FogEffectManager` (`Phyre_FogEffectManager_GetInstance`).
    pub const FOG_EFFECT: Global<u32> = Global::new(0x00ce_5170);
    /// `g_CharacterLightingTweakUtil` (`Phyre_CharacterLightingTweakUtil_GetInstance`).
    pub const LIGHTING_TWEAK: Global<u32> = Global::new(0x00cc_ca88);
    /// `g_AsyncLoadManager` (`AsyncLoadManager_sGetInstance`).
    pub const ASYNC_LOAD: Global<u32> = Global::new(0x00cc_c88c);
    /// `g_GraphicUi`: the menu manager (`graphicFontSetUiShadowType` tests it).
    pub const GRAPHIC_UI: Global<u32> = Global::new(0x00cc_c81c);
    /// `g_TipSystem` (`TipSystem_Get`).
    pub const TIP_SYSTEM: Global<u32> = Global::new(0x00ce_6ea4);
    /// `g_TextureAnimationManager`.
    pub use crate::managers::bridge::TEXTURE_ANIMATION;
    /// `g_GraphicSkipFrame`: set, the camera keeps last frame's view.
    pub const SKIP_FRAME: Global<u32> = Global::new(0x012f_b7d0);

    // ---- application fields ----
    /// Global ambient colour: three floats.
    pub const AMBIENT: u32 = 0x50;
    /// Fog colour: three floats.
    pub const FOG_COLOR: u32 = 0x60;
    /// Fog near, far and limit distances.
    pub const FOG_NEAR: u32 = 0x70;
    pub const FOG_FAR: u32 = 0x74;
    pub const FOG_LIMIT: u32 = 0x7c;
    /// Background (clear) colour: three floats.
    pub const BACKGROUND: u32 = 0x80;
    /// Whether keyframe animation is on (a byte).
    pub const KEYFRAME_ANIM: u32 = 0x3ac;

    // ---- post-process manager fields (the screen fade) ----
    /// Screen fade colour: four floats (r, g, b, alpha).
    pub const FADE_COLOR: u32 = 0x1_238c;
    /// Screen fade alpha mode.
    pub const FADE_ALPHA_MODE: u32 = 0x1_239c;
    /// Byte: the fade colour is locked (the setters do nothing).
    pub const FADE_LOCKED: u32 = 0x1_23a0;
    /// The last fade's reason.
    pub const FADE_LAST_REASON: u32 = 0x1_23a4;
    /// Byte: `graphicDrawFilter` ran this frame.
    pub const RAN_DRAW_FILTER: u32 = 0x1_23a8;
    /// Byte: `graphicDrawFocus` ran this frame.
    pub const RAN_DRAW_FOCUS: u32 = 0x1_23c5;

    // ---- originals called ----
    pub const SCENE_PREVIOUS_LEVEL_NAME_HASH: u32 = 0x0065_e310;
    pub const SCENE_IS_3D: u32 = 0x0066_01d0;
    pub const SCENE_IS_DRAWING_NON_UI_2D: u32 = 0x0066_0350;
    pub const SCENE_CURRENT_MAGIC_PATH: u32 = 0x0065_dea0;
    pub const SCENE_TO_BE_DELETE_MAGIC_PATH: u32 = 0x0065_e5e0;
    pub const SCENE_SET_BYTE_01: u32 = 0x0066_cfe0;
    pub const SCENE_REMOVE_CHR_FROM_PIPELINE: u32 = 0x0066_8640;
    pub const SCENE_SET_MESH_VISIBLE: u32 = 0x0066_d910;
    pub const ASYNC_LOAD_ALL_DONE: u32 = 0x0069_1630;
    pub const FOG_SET_CHR_FADE_TYPE: u32 = 0x006e_36c0;
    pub const POST_PROCESS_SET_FOCUS_DEPTH: u32 = 0x0069_8550;
    pub const MENU_SET_FONT_SHADOW: u32 = 0x0067_e4e0;
    pub const SCENE_SET_TEXTURE_VIDEO_INFO: u32 = 0x0066_e800;
    pub const SCENE_SET_CAMERA_WORLD_MATRIX: u32 = 0x0066_ce70;
    pub const SCENE_SET_MINIMAP_CAMERA_WORLD_MATRIX: u32 = 0x0066_d940;
    pub const SCENE_SET_MESH_INSTANCE_WORLD_MATRIX: u32 = 0x0066_d810;
    pub const SCENE_SET_CAMERA_FOVY: u32 = 0x0066_cda0;
    pub const SCENE_SET_CAMERA_ASPECT_RATIO: u32 = 0x0066_cbe0;
    pub const SCENE_SET_OBJECT_TRANSLATION: u32 = 0x0066_e3e0;
    pub const SCENE_SHOW_2D_LAYER: u32 = 0x0066_f2f0;
    pub const SCENE_HIDE_2D_LAYER: u32 = 0x0065_f050;
    /// The list cursor's set-up on a node (`thiscall(cursor, node)`).
    pub const CURSOR_SET_NODE: u32 = 0x0044_1dc0;
    pub const SCENE_CURRENT_LEVEL_NAME_HASH: u32 = 0x0065_de90;
    pub const LIGHTING_CALC_SHADE_COEFF: u32 = 0x006a_77a0;
    pub const LIGHTING_SET_SHADE_COEFF: u32 = 0x006a_aef0;
    pub const LIGHTING_TWEAK_SET_SHADE_COEFF: u32 = 0x006a_afc0;
    pub const POST_PROCESS_SET_CROSS_FADE_WEIGHT: u32 = 0x0069_84b0;
    pub const POST_PROCESS_SET_CROSS_FADE_TIME: u32 = 0x0069_84a0;
    pub const TIP_PUSH: u32 = 0x006e_8810;
    pub const TIP_PUSH_BY_LOC_ID: u32 = 0x006e_8770;
    pub const LIGHTING_SET_LIGHT_DIR: u32 = 0x006a_a9f0;
    pub const LIGHTING_SET_MAP_ID_FLAG_CHANGED: u32 = 0x006a_ad00;
    pub const LIGHTING_SET_POINT_LIGHT_COLOR: u32 = 0x006a_ad20;
    pub const LIGHTING_SET_POINT_LIGHT_POSITION: u32 = 0x006a_ad90;
    pub const LIGHTING_SET_POINT_LIGHT_RANGE: u32 = 0x006a_ae40;
    pub const LIGHTING_SET_SHADE_COLOR: u32 = 0x006a_b010;
    pub const PHYRE_PRINTF: u32 = 0x0043_52f0;
    /// `Phyre_PString_Assign(this, text)`.
    pub const PSTRING_ASSIGN: u32 = 0x0044_5cf0;
    /// `sceVu0InversMatrix4x4(dst, m)`.
    pub const INVERSE_MATRIX: u32 = 0x006e_d460;
    /// `PMatrix4_lookAt(m, eye, target, up)`.
    pub const MATRIX_LOOK_AT: u32 = 0x0045_1c30;
    /// `PMatrix4_toWorldMatrix3x4(dst, m)`.
    pub const MATRIX_TO_3X4: u32 = 0x0045_1190;
    /// `Phyre_PCamera_updateViewMatrices(camera)`.
    pub const CAMERA_UPDATE_VIEW: u32 = 0x0067_20c0;

    fn copy_float(m: &mut impl Mem, from: u32, to: u32) {
        let v = x87::round_trip(f32::from_bits(m.read(from)));
        m.write(to, v.to_bits());
    }

    /// `N` words at `at`.
    fn words<const N: usize>(m: &impl Mem, at: u32) -> [u32; N] {
        #[allow(clippy::cast_possible_truncation)] // N <= 16
        std::array::from_fn(|i| m.read(at + 4 * i as u32))
    }

    fn float_bits(x: f32) -> u32 {
        x87::round_trip(x).to_bits()
    }

    // ---- the application: fog, ambient, background ----

    /// `graphicGetFogColor(r*, g*, b*)` (`+0x60`) and
    /// `graphicGetGlobalAmbientColor(r*, g*, b*)` (`+0x50`): each non-null
    /// output gets its float.
    pub fn get_app_color(m: &mut impl Mem, base: u32, out: [u32; 3]) {
        let app = APPLICATION.get(m);
        for (i, p) in (0u32..).zip(out) {
            if p != 0 {
                copy_float(m, app + base + 4 * i, p);
            }
        }
    }

    /// `graphicGetFogNear()` (`+0x70`), `graphicGetFogFar()` (`+0x74`),
    /// `graphicGetFogLimit()` (`+0x7c`): a float of the application.
    pub fn app_float(m: &impl Mem, offset: u32) -> f32 {
        let app = APPLICATION.get(m);
        f32::from_bits(m.read(app + offset))
    }

    /// `graphicSetGlobalAmbientColor(r, g, b)` (`+0x50`) and
    /// `graphicSetBackgroundColour(r, g, b)` (`+0x80`). Returns the
    /// application (`eax`).
    pub fn set_app_color(m: &mut impl Mem, base: u32, rgb: [f32; 3]) -> u32 {
        let app = APPLICATION.get(m);
        for (i, v) in (0u32..).zip(rgb) {
            m.write(app + base + 4 * i, float_bits(v));
        }
        app
    }

    /// `graphicFFXApplicationEnableKeyframeAnim(on)`: the byte `+0x3ac`.
    pub fn enable_keyframe_anim(m: &mut impl Mem, on: u32) -> u32 {
        let app = APPLICATION.get(m);
        m.write(app + KEYFRAME_ANIM, u8::from(on != 0));
        app
    }

    // ---- the screen fade ----

    /// `graphicGetScreenFadeColor(r*, g*, b*, a*)`. Returns the manager.
    pub fn get_screen_fade_color(m: &mut impl Mem, out: [u32; 4]) -> u32 {
        let pp = POST_PROCESS.get(m);
        for (i, p) in (0u32..).zip(out) {
            if p != 0 {
                copy_float(m, pp + FADE_COLOR + 4 * i, p);
            }
        }
        pp
    }

    /// `graphicGetScreenFadeLastReason()`.
    pub fn screen_fade_last_reason(m: &impl Mem) -> u32 {
        let pp = POST_PROCESS.get(m);
        m.read(pp + FADE_LAST_REASON)
    }

    /// `graphicIsScreenFadeColorLocked()`: the byte, zero-extended.
    pub fn is_screen_fade_color_locked(m: &impl Mem) -> u32 {
        let pp = POST_PROCESS.get(m);
        u32::from(m.read::<u8>(pp + FADE_LOCKED))
    }

    /// `graphicLockScreenFadeColor()`, `graphicHasRunDrawFilterInThisFrame()`
    /// (`+0x123a8`), `graphicHasRunDrawFocusInThisFrame()` (`+0x123c5`) and
    /// `graphicResetHasRunDrawFocusInThisFrame()` (that one with 0): a byte
    /// of the manager. Returns the manager.
    pub fn set_post_process_byte(m: &mut impl Mem, offset: u32, v: u8) -> u32 {
        let pp = POST_PROCESS.get(m);
        m.write(pp + offset, v);
        pp
    }

    /// `graphicUnlockScreenFadeColor()`: unlocks and clears the fade alpha.
    pub fn unlock_screen_fade_color(m: &mut impl Mem) -> u32 {
        let pp = POST_PROCESS.get(m);
        m.write(pp + FADE_LOCKED, 0u8);
        m.write(pp + FADE_COLOR + 12, 0u32);
        pp
    }

    /// `graphicSetScreenFadeAlphaMode(mode)`: unless locked.
    pub fn set_screen_fade_alpha_mode(m: &mut impl Mem, mode: u32) -> u32 {
        let pp = POST_PROCESS.get(m);
        if m.read::<u8>(pp + FADE_LOCKED) == 0 {
            m.write(pp + FADE_ALPHA_MODE, mode);
        }
        pp
    }

    /// `graphicSetScreenFadeColor(r, g, b, a)`: unless locked.
    pub fn set_screen_fade_color(m: &mut impl Mem, rgba: [f32; 4]) -> u32 {
        let pp = POST_PROCESS.get(m);
        if m.read::<u8>(pp + FADE_LOCKED) == 0 {
            for (i, v) in (0u32..).zip(rgba) {
                m.write(pp + FADE_COLOR + 4 * i, float_bits(v));
            }
        }
        pp
    }

    /// `graphicSetScreenFadeLastReason(reason)`.
    pub fn set_screen_fade_last_reason(m: &mut impl Mem, reason: u32) -> u32 {
        let pp = POST_PROCESS.get(m);
        m.write(pp + FADE_LAST_REASON, reason);
        pp
    }

    /// `graphicSetFocusZ(which, z)`: the depth-of-field focus from a PS2
    /// z-buffer value, `((z - 16770000) / -16769999) * 2 - 1` (`z` 0
    /// gives -2, none), through `PostProcessManager_setFocusDepth`.
    pub fn set_focus_z(h: &mut impl Host, which: u32, z: u32) -> u32 {
        #[allow(clippy::cast_possible_wrap)] // fild: signed
        let depth = if z == 0 {
            0xc000_0000
        } else {
            let f = x87::load(x87::store(f64::from(z as i32)));
            let d = x87::div(x87::sub(f, 16_770_000.0), -16_769_999.0);
            x87::store(x87::sub(x87::add(d, d), 1.0))
        };
        let pp = POST_PROCESS.get(h);
        h.call_sym(
            POST_PROCESS_SET_FOCUS_DEPTH,
            Conv::Thiscall(pp),
            &[which, depth],
        )
    }

    // ---- wrappers over the scene and the other managers ----

    /// The `graphic*` one-liners that call a `PhyreScene` method on
    /// `g_PhyreScene` and return its `eax`: `graphicPreviousLevelGetNameHash`,
    /// `graphicIs3DScene`, `graphicIsInCut2DScene`, `graphicMagicGetPath`,
    /// `graphicMagicGetToBeDeletePath` (no arguments),
    /// `graphicDttPageFlag(v)` and `graphicCharacterInstanceRemoveFromPipeline(chr)`.
    pub fn scene_call(h: &mut impl Host, method: u32, args: &[u32]) -> u32 {
        let scene = SCENE.get(h);
        h.call_sym(method, Conv::Thiscall(scene), args)
    }

    /// `graphicIsAllSceneAssetsLoaded()`: `AsyncLoadManager_isAllRequestDone`,
    /// zero-extended.
    pub fn is_all_scene_assets_loaded(h: &mut impl Host) -> u32 {
        let mgr = ASYNC_LOAD.get(h);
        h.call_sym(ASYNC_LOAD_ALL_DONE, Conv::Thiscall(mgr), &[]) & 0xff
    }

    /// `graphicSetChrFogFade(on)`: the fog manager's character fade type,
    /// 1 when `on` is 0.
    pub fn set_chr_fog_fade(h: &mut impl Host, on: u32) -> u32 {
        let fog = FOG_EFFECT.get(h);
        h.call_sym(
            FOG_SET_CHR_FADE_TYPE,
            Conv::Thiscall(fog),
            &[u32::from(on == 0)],
        )
    }

    /// `graphicSet2DStartLayer(layer)`: the texture-animation manager's
    /// `+8`. Returns the manager.
    pub fn set_2d_start_layer(m: &mut impl Mem, layer: u32) -> u32 {
        let mgr = TEXTURE_ANIMATION.get(m);
        m.write(mgr + 8, layer);
        mgr
    }

    /// `graphicFontSetUiShadowType(type)`: the menu manager's font shadow,
    /// when there is a menu manager.
    pub fn font_set_ui_shadow_type(h: &mut impl Host, kind: u32) {
        let menu = GRAPHIC_UI.get(h);
        if menu != 0 {
            h.call_sym(MENU_SET_FONT_SHADOW, Conv::Thiscall(menu), &[kind]);
        }
    }

    /// `graphicPushTip(id)` (`TipSystem_PlayTip`) and
    /// `graphicPushTipByLocId(id)` (`TipSystem_PushTip`): the tip system's
    /// result, zero-extended.
    pub fn push_tip(h: &mut impl Host, method: u32, id: u32) -> u32 {
        let tips = TIP_SYSTEM.get(h);
        h.call_sym(method, Conv::Thiscall(tips), &[id]) & 0xff
    }

    /// `graphicConvertScreenCoords(x, y, *fx, *fy)`: PS2 screen units to
    /// the 1920x1080 frame (`x * 3.75`, `y * 2.5961537`). Returns `fy`.
    pub fn convert_screen_coords(m: &mut impl Mem, x: u32, y: u32, fx: u32, fy: u32) -> u32 {
        // the y factor: the double at 0xb41d10
        let ky = f64::from_bits(0x4004_c4ec_4000_0000);
        #[allow(clippy::cast_possible_wrap)] // fild: signed
        let scale = |v: u32, k: f64| {
            let f = x87::load(x87::store(f64::from(v as i32)));
            x87::store(x87::mul(f, k))
        };
        m.write(fx, scale(x, 3.75));
        m.write(fy, scale(y, ky));
        fy
    }

    /// `graphicGuidemapSetup(lo, hi)`: the guide map's effect has its data
    /// and geometry, and the geometry's virtual `1(0, 0)` returns 0; an
    /// instance of 0 is logged as an error.
    pub fn guidemap_setup(h: &mut impl Host, lo: u32, hi: u32) -> u32 {
        use crate::vfx::bridge as vfx;
        if lo | hi == 0 {
            // "Virtuos Error: invalid VFX instance or invalid VFX data!\n"
            let msg = h.rebase(0x00b4_1a6c);
            h.call_sym(PHYRE_PRINTF, Conv::Cdecl, &[1, msg]);
            return 0;
        }
        let table = vfx::TABLE.get(h);
        let data = vfx::get_vfx_data(h, table, lo, hi);
        let table = vfx::TABLE.get(h);
        let geo = vfx::get_dynamic_geometry(h, table, lo, hi);
        if data == 0 || geo == 0 {
            return 0;
        }
        u32::from(h.call_virtual(geo, 1, &[0, 0]) == 0)
    }

    // ---- character lighting ----

    /// `graphicCharacterLightingSetLightDir(light, x, y, z)`.
    pub fn lighting_set_light_dir(h: &mut impl Host, light: u32, v: [f32; 3]) -> u32 {
        let l = CHARACTER_LIGHTING.get(h);
        let [x, y, z] = v.map(float_bits);
        h.call_sym(LIGHTING_SET_LIGHT_DIR, Conv::Thiscall(l), &[light, x, y, z])
    }

    /// `graphicCharacterLightingSetMapIdFlagChanged(a, b)`.
    pub fn lighting_set_map_id_flag_changed(h: &mut impl Host, a: u32, b: u32) -> u32 {
        let l = CHARACTER_LIGHTING.get(h);
        h.call_sym(LIGHTING_SET_MAP_ID_FLAG_CHANGED, Conv::Thiscall(l), &[a, b])
    }

    /// `graphicCharacterLightingSetPointLightColor(light, r, g, b)` and
    /// `...Position(light, x, y, z)`: the floats first, then the light.
    pub fn lighting_set_point_light3(
        h: &mut impl Host,
        method: u32,
        light: u32,
        v: [f32; 3],
    ) -> u32 {
        let l = CHARACTER_LIGHTING.get(h);
        let [x, y, z] = v.map(float_bits);
        h.call_sym(method, Conv::Thiscall(l), &[x, y, z, light])
    }

    /// `graphicCharacterLightingSetPointLightRange(light, near, far)`.
    pub fn lighting_set_point_light_range(h: &mut impl Host, light: u32, v: [f32; 2]) -> u32 {
        let l = CHARACTER_LIGHTING.get(h);
        let [a, b] = v.map(float_bits);
        h.call_sym(
            LIGHTING_SET_POINT_LIGHT_RANGE,
            Conv::Thiscall(l),
            &[a, b, light],
        )
    }

    /// `graphicCharacterLightingSetShadeColor(list, r, g, b)`: each
    /// character of the instance list (a count with bit 31 a flag, then
    /// the pointer array).
    pub fn lighting_set_shade_color(h: &mut impl Host, list: u32, v: [f32; 3]) {
        if list == 0 {
            return;
        }
        let [r, g, b] = v.map(float_bits);
        let mut i = 0;
        while i < h.read::<u32>(list) & 0x7fff_ffff {
            let items: u32 = h.read(list + 4);
            let chr: u32 = h.read(items + 4 * i);
            let l = CHARACTER_LIGHTING.get(h);
            h.call_sym(LIGHTING_SET_SHADE_COLOR, Conv::Thiscall(l), &[chr, r, g, b]);
            i += 1;
        }
    }

    // ---- character instances ----

    /// `graphicCharacterInstanceSetVisible(list, visible)`: the list's
    /// byte `+0x14` (`visible == 1`) and each mesh's visibility
    /// (`PhyreScene_setMeshVisible`).
    pub fn character_set_visible(h: &mut impl Host, list: u32, visible: u32) {
        if list == 0 {
            return;
        }
        let on = u32::from(visible == 1);
        #[allow(clippy::cast_possible_truncation)] // a byte
        h.write(list + 0x14, on as u8);
        let mut i = 0;
        while i < h.read::<u32>(list) & 0x7fff_ffff {
            let items: u32 = h.read(list + 4);
            let mesh: u32 = h.read(items + 4 * i);
            let scene = SCENE.get(h);
            h.call_sym(SCENE_SET_MESH_VISIBLE, Conv::Thiscall(scene), &[mesh, on]);
            i += 1;
        }
    }

    /// `graphicCharacterInstanceSetSkeletonMatrices(chr_id, list, bones)`:
    /// every mesh's skin matrices from the game's bone matrices (`bones`,
    /// 64 bytes each), through the mesh's bone-index table (`list +0x10`
    /// `+0x14`, one per mesh). For character `0x206a` bones from `0x42` on
    /// keep theirs.
    pub fn character_set_skeleton_matrices(m: &mut impl Mem, chr: u32, list: u32, bones: u32) {
        if list == 0 {
            return;
        }
        let mut i = 0;
        while i < m.read::<u32>(list) & 0x7fff_ffff {
            let items: u32 = m.read(list + 4);
            let inst: u32 = m.read(items + 4 * i);
            let mut out: u32 = m.read(inst + 0xc);
            let skel: u32 = m.read(inst);
            let count = m.read::<u32>(skel + 0x18) & 0x7fff_ffff;
            let tables: u32 = m.read(list + 0x10);
            let tables: u32 = m.read(tables + 0x14);
            let index: u32 = m.read(tables + 4 * i);
            for j in 0..count {
                let bone: u32 = m.read(index + 4 * j);
                #[allow(clippy::cast_possible_wrap)] // jge: signed
                if chr != 0x206a || (bone as i32) < 0x42 {
                    let from = bones.wrapping_add(bone << 6);
                    for row in 0..4 {
                        // a row at a time, through a temporary
                        let r =
                            words::<4>(m, from + 16 * row).map(|w| float_bits(f32::from_bits(w)));
                        for (k, v) in (0u32..).zip(r) {
                            m.write(out + 16 * row + 4 * k, v);
                        }
                    }
                }
                out += 0x40;
            }
            i += 1;
        }
    }

    /// `graphicCharacterInstanceSetSkeletonMatricesRestoreDefault(list, bone)`:
    /// the first mesh whose bone-index table (from entry 1) holds `bone`
    /// gets that bone's default matrix back (the skeleton's `+0x1c`).
    pub fn character_restore_skeleton_matrix(m: &mut impl Mem, list: u32, bone: u32) {
        if list == 0 {
            return;
        }
        let n = m.read::<u32>(list) & 0x7fff_ffff;
        let tables: u32 = m.read(list + 0x10);
        let tables: u32 = m.read(tables + 0x14);
        let items: u32 = m.read(list + 4);
        for i in 0..n {
            let inst: u32 = m.read(items + 4 * i);
            let index: u32 = m.read(tables + 4 * i);
            let out: u32 = m.read(inst + 0xc);
            let skel: u32 = m.read(inst);
            let count = m.read::<u32>(skel + 0x18) & 0x7fff_ffff;
            for j in 1..count {
                if m.read::<u32>(index + 4 * j) == bone {
                    let skel: u32 = m.read(inst);
                    let from = m.read::<u32>(skel + 0x1c).wrapping_add(j << 6);
                    for k in 0..16 {
                        copy_float(m, from + 4 * k, out.wrapping_add((j << 6) + 4 * k));
                    }
                    return;
                }
            }
        }
    }

    // ---- cameras ----

    /// `graphicCameraSetWorldMatrix(view)`: the game's 4x4 view matrix
    /// (PS2 conventions) to the scene's 3x4 camera world matrix (`+0xfa04`):
    /// inverted, x and y of rows 0 and 3 and z of rows 1 and 2 negated, then
    /// rows 1-3 with row 0's first three as the last column. A matrix with
    /// an all-zero row (bit for bit) gives the identity instead. Skipped
    /// frames keep the old one.
    pub fn camera_set_world_matrix(h: &mut impl Host, view: u32) {
        if SKIP_FRAME.get(h) != 0 {
            return;
        }
        let w = words::<16>(h, view);
        let zero_row = (0..4).any(|r| w[4 * r] | w[4 * r + 1] | w[4 * r + 2] | w[4 * r + 3] == 0);
        let out = if zero_row {
            let mut id = [0u32; 16];
            for i in [0, 5, 10, 15] {
                id[i] = 1f32.to_bits();
            }
            to_world_3x4(id)
        } else {
            view_to_world(h, w)
        };
        let scene = SCENE.get(h);
        for (i, v) in (0u32..).zip(out) {
            h.write(scene + 0xfa04 + 4 * i, float_bits(f32::from_bits(v)));
        }
    }

    /// `PMatrix4_toWorldMatrix3x4` of a 4x4: rows 1-3, row 0's first three
    /// as the last column.
    fn to_world_3x4(m: [u32; 16]) -> [u32; 12] {
        [
            m[4], m[5], m[6], m[0], m[8], m[9], m[10], m[1], m[12], m[13], m[14], m[2],
        ]
    }

    /// The game's 4x4 view (words `w`) as the camera's 3x4 world matrix:
    /// inverted (`sceVu0InversMatrix4x4` in the scratch), x and y of rows
    /// 0 and 3 and z of rows 1 and 2 negated (the original's 14
    /// `fld`/`fchs`/`fstp`), converted.
    fn view_to_world(h: &mut impl Host, w: [u32; 16]) -> [u32; 12] {
        let s = h.scratch();
        for (i, v) in (0u32..).zip(w) {
            h.write(s + 4 * i, float_bits(f32::from_bits(v)));
        }
        h.call_sym(INVERSE_MATRIX, Conv::Cdecl, &[s, s]);
        let mut m = words::<16>(h, s);
        for i in [0, 1, 4, 5, 8, 9, 12, 13, 4, 5, 6, 8, 9, 10] {
            m[i] = float_bits(f32::from_bits(m[i])) ^ 0x8000_0000;
        }
        to_world_3x4(m)
    }

    /// Writes a 3x4 into the scratch after the inverse's 64 bytes, for a
    /// callee that copies it. Returns its address.
    fn scratch_rows(h: &mut impl Host, rows: [u32; 12]) -> u32 {
        let at = h.scratch() + 0x40;
        for (i, v) in (0u32..).zip(rows) {
            h.write(at + 4 * i, v);
        }
        at
    }

    /// `graphicCameraSetWorldMatrixSub(view, on)` (`0x00639b30`): with `on`,
    /// the view (as `graphicCameraSetWorldMatrix`, without the zero-row and
    /// skipped-frame checks) to the scene's alternative camera matrix
    /// (`PhyreScene_setCameraWorldMatrix(m, 1)`); else back to the main
    /// one (byte `+0xfaa1` 0).
    pub fn camera_set_world_matrix_sub(h: &mut impl Host, view: u32, on: u32) {
        if on == 0 {
            let scene = SCENE.get(h);
            h.write(scene + 0xfaa1, 0u8);
            return;
        }
        let w = words::<16>(h, view);
        let rows = view_to_world(h, w);
        let at = scratch_rows(h, rows);
        let scene = SCENE.get(h);
        h.call_sym(
            SCENE_SET_CAMERA_WORLD_MATRIX,
            Conv::Thiscall(scene),
            &[at, 1],
        );
    }

    /// `graphicMinimapCameraSetWorldMatrix(view)` (`0x00642410`): the view,
    /// as `graphicCameraSetWorldMatrix`, to the minimap camera
    /// (`PhyreScene_setMinimapCameraWorldMatrix`).
    pub fn minimap_camera_set_world_matrix(h: &mut impl Host, view: u32) {
        let w = words::<16>(h, view);
        let rows = view_to_world(h, w);
        let at = scratch_rows(h, rows);
        let scene = SCENE.get(h);
        h.call_sym(
            SCENE_SET_MINIMAP_CAMERA_WORLD_MATRIX,
            Conv::Thiscall(scene),
            &[at],
        );
    }

    /// `graphicCameraSetFOVy(&v)` (`0x006395e0`) and
    /// `graphicCameraSetAspectRatio(&v)` (`0x006395c0`): the float at `v`
    /// to the scene's setter. Returns its `eax`.
    pub fn camera_set_from_ptr(h: &mut impl Host, method: u32, v: u32) -> u32 {
        let f = float_bits(f32::from_bits(h.read(v)));
        scene_call(h, method, &[f])
    }

    /// `graphicCharacterInstanceSetWorldMatrix(list, m)` (`0x0063c3e0`):
    /// `PhyreScene_setMeshInstanceWorldMatrix(mesh, m)` for each mesh of
    /// the list.
    pub fn character_set_world_matrix(h: &mut impl Host, list: u32, mat: u32) {
        if list == 0 {
            return;
        }
        let mut i = 0;
        while i < h.read::<u32>(list) & 0x7fff_ffff {
            let items: u32 = h.read(list + 4);
            let mesh: u32 = h.read(items + 4 * i);
            h.call_sym(
                SCENE_SET_MESH_INSTANCE_WORLD_MATRIX,
                Conv::Stdcall,
                &[mesh, mat],
            );
            i += 1;
        }
    }

    /// `graphicCharacterInstanceSetWorldMatrixByMdlID(list, m, id)`
    /// (`0x0063c430`): every mesh one of whose segments (`0x6c` bytes,
    /// the id at `+0x18`) has model id `id` gets the world matrix (once per
    /// matching segment); with none, mesh number `id` when there is one.
    pub fn character_set_world_matrix_by_mdl_id(h: &mut impl Host, list: u32, mat: u32, id: u32) {
        if list == 0 {
            return;
        }
        let mut found = false;
        let mut i = 0;
        while i < h.read::<u32>(list) & 0x7fff_ffff {
            let items: u32 = h.read(list + 4);
            let mesh: u32 = h.read(items + 4 * i);
            let holder: u32 = h.read(mesh);
            let n = h.read::<u32>(holder) & 0x7fff_ffff;
            for k in 0..n {
                let table: u32 = h.read(holder + 4);
                if h.read::<u32>(table.wrapping_add(0x6c * k + 0x18)) == id {
                    let items: u32 = h.read(list + 4);
                    let mesh: u32 = h.read(items + 4 * i);
                    found = true;
                    h.call_sym(
                        SCENE_SET_MESH_INSTANCE_WORLD_MATRIX,
                        Conv::Stdcall,
                        &[mesh, mat],
                    );
                }
            }
            i += 1;
        }
        if found {
            return;
        }
        if id < h.read::<u32>(list) & 0x7fff_ffff {
            let items: u32 = h.read(list + 4);
            let mesh: u32 = h.read(items + 4 * id);
            h.call_sym(
                SCENE_SET_MESH_INSTANCE_WORLD_MATRIX,
                Conv::Stdcall,
                &[mesh, mat],
            );
        }
    }

    /// The scene's list cursors' steps (`0x0065eaf0`, `0x0065ebd0`,
    /// `0x0065ec40`, `0x0065ecb0`: seek, checking first; `0x0065ed20`,
    /// `0x0065ed80`, `0x0065ede0`, `0x0065ee40`: advance, stepping first;
    /// fastcall): while the cursor is not valid (`PhyreCursor_IsValid_*`),
    /// its node (`+0x18`) moves to the next (`*node`, 0 at the end node
    /// `+0x14`); then the cursor is set up on the node (`0x00441dc0`), or
    /// its first five words cleared without one. Returns the set-up's
    /// `eax`, or 0.
    pub fn cursor_step(h: &mut impl Host, this: u32, seek: bool) -> u32 {
        use crate::managers::bridge::cursor_valid;
        if !(seek && cursor_valid(h, this) != 0) {
            loop {
                let node: u32 = h.read(this + 0x18);
                if node != 0 {
                    let next: u32 = h.read(node);
                    let end: u32 = h.read(this + 0x14);
                    h.write(this + 0x18, if next == end { 0 } else { next });
                }
                if cursor_valid(h, this) != 0 {
                    break;
                }
            }
        }
        let node: u32 = h.read(this + 0x18);
        if node == 0 {
            for k in 0..5 {
                h.write(this + 4 * k, 0u32);
            }
            return 0;
        }
        h.call_sym(CURSOR_SET_NODE, Conv::Thiscall(this), &[node])
    }

    /// the camera's (`+0xfa04`) or, `alt` set, the alternative one
    /// (`+0xfa3c`); the byte `+0xfaa1` says which. Returns `m`.
    pub fn scene_set_camera_world_matrix(m: &mut impl Mem, scene: u32, mat: u32, alt: u32) -> u32 {
        let alt = alt & 0xff != 0;
        let to = scene + if alt { 0xfa3c } else { 0xfa04 };
        for i in 0..12 {
            copy_float(m, mat + 4 * i, to + 4 * i);
        }
        m.write(scene + 0xfaa1, u8::from(alt));
        mat
    }

    /// `PhyreScene_setMinimapCameraWorldMatrix(this, m)`: with a minimap
    /// camera (`+0x654`), the 3x4 matrix to `+0x7b0`, which the camera
    /// then points at, and its view matrices are updated.
    pub fn scene_set_minimap_camera_world_matrix(h: &mut impl Host, scene: u32, mat: u32) {
        let camera: u32 = h.read(scene + 0x654);
        if camera == 0 {
            return;
        }
        for i in 0..12 {
            copy_float(h, mat + 4 * i, scene + 0x7b0 + 4 * i);
        }
        h.write(camera + 0xb4, scene + 0x7b0);
        let camera: u32 = h.read(scene + 0x654);
        h.call_sym(CAMERA_UPDATE_VIEW, Conv::Thiscall(camera + 4), &[]);
    }

    /// Sets a camera's float at `offset` and calls its virtual `1`
    /// (the update).
    fn camera_set(h: &mut impl Host, scene: u32, camera_at: u32, offset: u32, v: u32) -> u32 {
        let camera: u32 = h.read(scene + camera_at);
        h.write(camera + offset, float_bits(f32::from_bits(v)));
        h.call_virtual(camera, 1, &[])
    }

    /// `PhyreScene_setCameraFOVy(this, fovy)`: the main camera (`+0xdc`)
    /// and the minimap camera (`+0x654`); with the 2D level loaded (byte
    /// `+0x93c`, not a 3D flag) fixed angles on levels `0xec1` (both),
    /// `0x107` and `0xf48` (the minimap camera); the reflection camera
    /// (`+0x238`) when `+0x10d8c` is 1. Returns the last update's `eax`.
    pub fn scene_set_camera_fovy(h: &mut impl Host, scene: u32, fovy: u32) -> u32 {
        const FOV: u32 = 0x154;
        let mut fovy = fovy;
        let r = if h.read::<u8>(scene + 0x93c) == 0 {
            camera_set(h, scene, 0xdc, FOV, fovy);
            camera_set(h, scene, 0x654, FOV, fovy)
        } else {
            if h.read::<u32>(scene + 0xfaf4) == 0xec1 {
                fovy = float_bits(f32::from_bits(0x3ef1_26e9));
            }
            camera_set(h, scene, 0xdc, FOV, fovy);
            let mut r = camera_set(h, scene, 0x654, FOV, fovy);
            if h.read::<u32>(scene + 0xfaf4) == 0x107 {
                r = camera_set(h, scene, 0x654, FOV, 0x3f2c_49ba);
            }
            if h.read::<u32>(scene + 0xfaf4) == 0xf48 {
                r = camera_set(h, scene, 0x654, FOV, 0x3f51_26e9);
            }
            r
        };
        if h.read::<u32>(scene + 0x10d8c) == 1 {
            return camera_set(h, scene, 0x238, FOV, fovy);
        }
        r
    }

    /// `PhyreScene_setCameraAspectRatio(this, aspect)`: the main and
    /// minimap cameras' aspect (`+0x150`; in a 3D scene `aspect * 3 / 4 *
    /// 16 / 9`); with a reflection camera (`+0x10d8c` 1) of kind 0 or 1
    /// (`+0x10d90`) that one at half or twice it, and the main camera's
    /// from the window size (the application's `+0x26c`/`+0x270`). Returns
    /// the last update's `eax` (or the kind, for another kind).
    pub fn scene_set_camera_aspect_ratio(h: &mut impl Host, scene: u32, aspect: u32) -> u32 {
        const ASPECT: u32 = 0x150;
        // the double at 0xb43d68
        let k169 = f64::from_bits(0x3ffc_71c7_2000_0000);
        let a = x87::load(aspect);
        let v = if crate::scene::bridge::IS_3D.get(h) == 0 {
            camera_set(h, scene, 0xdc, ASPECT, aspect);
            aspect
        } else {
            let v = x87::store(x87::mul(x87::mul(x87::mul(a, 3.0), 0.25), k169));
            camera_set(h, scene, 0xdc, ASPECT, v);
            v
        };
        let mut r = camera_set(h, scene, 0x654, ASPECT, v);
        if h.read::<u32>(scene + 0x10d8c) != 1 {
            return r;
        }
        let kind: u32 = h.read(scene + 0x10d90);
        let k = match kind {
            0 => 0.5,
            1 => 2.0,
            _ => return kind,
        };
        let t = x87::mul(x87::mul(a, 3.0), 0.25);
        let refl = x87::store(x87::mul(x87::mul(t, k169), k));
        camera_set(h, scene, 0x238, ASPECT, refl);
        let app = APPLICATION.get(h);
        let w: u32 = h.read(app + 0x26c);
        let ht: u32 = h.read(app + 0x270);
        let w = x87::load(x87::store(f64::from(w)));
        let ht = x87::load(x87::store(f64::from(ht)));
        let main = x87::store(x87::mul(x87::div(x87::mul(w, t), ht), k));
        r = camera_set(h, scene, 0xdc, ASPECT, main);
        r
    }

    /// `PhyreScene_setAbmapCamera(this, x, y, z, height, scale)`: the
    /// area-map camera (`+0xbe0`, its world matrix at `+0xd34`): a look-at
    /// from `(-z*1920/512, x*1080*16/13/scale - h*1080*4/1536,
    /// 1 - y*1080*16/13/scale)` towards `(-z*1920/512, -h*1080*4/1536, 1)`
    /// with up `(0, y, x)`.
    pub fn scene_set_abmap_camera(h: &mut impl Host, scene: u32, p: [u32; 5]) -> u32 {
        let [a, b, c, d, e] = p.map(x87::load);
        // the double at 0xb43d60 (16/13)
        let k = f64::from_bits(0x3ff3_b13b_2000_0000);
        let c1 = x87::store(x87::mul(x87::mul(c, 1920.0), 0.001_953_125));
        let d1 = x87::store(x87::div(x87::mul(x87::mul(d, 1080.0), 4.0), 1536.0));
        let x = x87::store(-x87::load(c1));
        let q = x87::div(x87::mul(x87::mul(a, 1080.0), k), e);
        let y = x87::store(x87::sub(q, x87::load(d1)));
        let s = x87::div(x87::mul(x87::mul(b, 1080.0), k), e);
        let z = x87::store(x87::sub(1.0, s));
        let sc = h.scratch();
        let (mat, eye, target, up) = (sc, sc + 0x40, sc + 0x50, sc + 0x60);
        for (at, v) in [
            (eye, [x, y, z, 0]),
            (target, [x, x87::store(-x87::load(d1)), 1f32.to_bits(), 0]),
            (up, [0, x87::store(b), x87::store(a), 0]),
        ] {
            for (i, w) in (0u32..).zip(v) {
                h.write(at + 4 * i, w);
            }
        }
        h.call_sym(MATRIX_LOOK_AT, Conv::Thiscall(mat), &[eye, target, up]);
        h.call_sym(MATRIX_TO_3X4, Conv::Thiscall(scene + 0xd34), &[mat]);
        h.write(scene + 0xc90, scene + 0xd34);
        h.call_sym(CAMERA_UPDATE_VIEW, Conv::Thiscall(scene + 0xbe0), &[])
    }

    // ---- 2D layers ----

    /// `PhyreScene_show2DLayer(this, id)` and `PhyreScene_hide2DLayer`:
    /// with the 2D level loaded (`+0x93c`) and 2D layers (`+0x1128`), the
    /// first layer of the current group (`+0x11d8`) with that id gets its
    /// shown byte, and its
    /// mesh's draw pointer (`+0x1c` of the mesh's `+0x1c`) is set to the
    /// mesh (shown) or cleared.
    pub fn scene_set_2d_layer_shown(m: &mut impl Mem, scene: u32, id: u32, shown: bool) {
        if m.read::<u8>(scene + 0x93c) == 0 || m.read::<u32>(scene + 0x1128) == 0 {
            return;
        }
        let mut i = 0;
        while i < m.read::<u32>(scene + 0x1138) {
            let layers: u32 = m.read(scene + 0x1140);
            let e = layers + 8 * i;
            let group = i32::from(m.read::<i8>(e + 1));
            #[allow(clippy::cast_possible_wrap)]
            let matches = m.read::<i32>(scene + 0x11d8) == group
                && id as i32 == i32::from(m.read::<i16>(e + 2));
            if matches {
                m.write(e, u8::from(shown));
                let layers: u32 = m.read(scene + 0x1140);
                let mesh: u32 = m.read(layers + 8 * i + 4);
                let draw: u32 = m.read(mesh + 0x1c);
                if draw != 0 {
                    let v = if shown {
                        let layers: u32 = m.read(scene + 0x1140);
                        m.read(layers + 8 * i + 4)
                    } else {
                        0
                    };
                    m.write(draw + 0x1c, v);
                    return;
                }
            }
            i += 1;
        }
    }

    /// `setFarPlane(far)`: the main camera's far plane (`+0xbc`), then its
    /// update (a tail call: its `eax`).
    pub fn set_far_plane(h: &mut impl Host, far: u32) -> u32 {
        let scene = SCENE.get(h);
        camera_set(h, scene, 0xdc, 0xbc, far)
    }

    /// `PhyreScene_setMeshInstanceWorldMatrix(mesh, m)` (`stdcall`): the
    /// game's 4x4 matrix with x and y negated in every row, to the mesh's
    /// 3x4 world matrix (`*(mesh +4)`, as [`MATRIX_TO_3X4`]), which the
    /// draw entry (`*(mesh +0x1c) +0xc`) then points at. Returns the world
    /// matrix.
    pub fn scene_set_mesh_instance_world_matrix(h: &mut impl Host, mesh: u32, mat: u32) -> u32 {
        let world: u32 = h.read(mesh + 4);
        let s = h.scratch();
        for i in 0..16 {
            let mut v = float_bits(f32::from_bits(h.read(mat + 4 * i)));
            if i % 4 < 2 {
                v ^= 0x8000_0000;
            }
            h.write(s + 4 * i, v);
        }
        let r = h.call_sym(MATRIX_TO_3X4, Conv::Thiscall(world), &[s]);
        let draw: u32 = h.read(mesh + 0x1c);
        if draw == 0 {
            return r;
        }
        let world: u32 = h.read(mesh + 4);
        h.write(draw + 0xc, world);
        world
    }

    /// `PhyreScene_setObjectTranslation(this, id, pos)`: with the object
    /// tree up (`+0x10d4`), the object `id`'s meshes (the tree at
    /// `+0x1198` maps it to a list: `+0` count, `+8` the meshes) get the
    /// translation row of their 3x4 world matrix (`*(mesh +4)`, words
    /// 8-10) set to `(-x, -y, z)`, the rest rewritten as it was; each
    /// mesh's draw entry then points at it. Returns 0.
    pub fn scene_set_object_translation(m: &mut impl Mem, scene: u32, id: u32, pos: u32) -> u32 {
        use crate::managers::bridge::tree_find_key;
        if m.read::<u32>(scene + 0x10d4) == 0 {
            return 0;
        }
        let tree = scene + 0x1198;
        let node = tree_find_key(m, tree, id);
        if node == 0 || node.wrapping_add(0x10) == 0 {
            return 0;
        }
        let node = tree_find_key(m, tree, id);
        let value = if node == 0 { 0 } else { node + 0x10 };
        let list: u32 = m.read(value);
        let mut i = 0;
        while i < m.read::<u32>(list) {
            let meshes: u32 = m.read(list + 8);
            let mesh: u32 = m.read(meshes + 4 * i);
            let world: u32 = m.read(mesh + 4);
            let old = words::<12>(m, world);
            i += 1;
            let [x, y, z] = words::<3>(m, pos).map(|w| float_bits(f32::from_bits(w)));
            let t = [x ^ 0x8000_0000, y ^ 0x8000_0000, z];
            for (k, v) in (0u32..).zip(old) {
                let v = match k {
                    8..=10 => t[k as usize - 8],
                    _ => float_bits(f32::from_bits(v)),
                };
                m.write(world + 4 * k, v);
            }
            let meshes: u32 = m.read(list + 8);
            let mesh: u32 = m.read(meshes + 4 * (i - 1));
            let draw: u32 = m.read(mesh + 0x1c);
            let world: u32 = m.read(mesh + 4);
            m.write(draw + 0xc, world);
        }
        0
    }

    /// `PMatrix4_FromMatrix3x4(this, m34)`, `0x00531a40(this, rows)` (the
    /// upper 3x3 from three padded rows, `thiscall`, unnamed),
    /// `Matrix_RotationX(out, angle)`, `Matrix_RotationY(out, angle)`,
    /// `0x004cd1f0(out, angle)` (the z rotation, cdecl, unnamed),
    /// `0x005e4400(out, a, b)` (a product, cdecl, unnamed),
    /// `PMatrix4_Copy(this, src)`, `PMatrix4_RowTimes(this, out, row)`,
    /// `PMatrix4_toWorldMatrix3x4(this, m44)` and the `.rdata` 10.0 the
    /// scale uses.
    pub const PMATRIX4_FROM_3X4: u32 = 0x0045_1080;
    pub const PMATRIX4_SET_UPPER3X3: u32 = 0x0053_1a40;
    pub const MATRIX_ROTATION_X: u32 = 0x0066_c8a0;
    pub const MATRIX_ROTATION_Y: u32 = 0x0066_c920;
    pub const MATRIX_ROTATION_Z: u32 = 0x004c_d1f0;
    pub const MATRIX_MUL: u32 = 0x005e_4400;
    pub const PMATRIX4_COPY: u32 = 0x004c_9430;
    pub const PMATRIX4_ROW_TIMES: u32 = 0x004c_9c20;
    pub const PMATRIX4_TO_WORLD_3X4: u32 = 0x0045_1190;
    pub const ROTATION_SCALE: u32 = 0x00b9_2318;
    /// Bytes of work [`scene_set_object_rotation`] needs, laid out as the
    /// original's frame from `ebp - 0x1f8`.
    pub const OBJECT_ROTATION_WORK: u32 = 0x1f8;
    // the frame's matrices: the object's (`ebp - 0x40`), two scratch ones
    // (`- 0x80`, `- 0xc0`, the scale rows inside the second at `- 0xb0`),
    // the 3x4 world copy (`- 0xf8`) and the rotation (`- 0x138`); the x
    // rotations' row products of each stage (rows +0x30, +0x20, +0x10, +0)
    const ROT_M: u32 = 0x1b8;
    const ROT_A: u32 = 0x178;
    const ROT_B: u32 = 0x138;
    const ROT_SCALE: u32 = 0x148;
    const ROT_W: u32 = 0x100;
    const ROT_R: u32 = 0xc0;
    const ROT_ROWS: [[u32; 4]; 3] = [
        [0x10, 0x80, 0x60, 0x40],
        [0x20, 0x00, 0x90, 0xb0],
        [0x70, 0x30, 0x50, 0xa0],
    ];

    /// The axes `PhyreScene_setObjectRotation` turns about, in order, for
    /// rotation order `mode`; none for an order beyond 5.
    #[must_use]
    pub fn rotation_order(mode: i16) -> &'static [u8] {
        match mode {
            0 => b"zyx",
            1 => b"zxy",
            2 => b"yxz",
            3 => b"yzx",
            4 => b"xzy",
            5 => b"xyz",
            _ => b"",
        }
    }

    /// `-x` as `fld; fchs; fstp dword` makes it.
    fn negated(bits: u32) -> u32 {
        x87::store(-x87::load(bits))
    }

    /// The 16 floats at `from` to `to` through the x87.
    fn copy_matrix(m: &mut impl Mem, to: u32, from: u32) {
        for k in 0..16 {
            let v = float_bits(f32::from_bits(m.read(from + 4 * k)));
            m.write(to + 4 * k, v);
        }
    }

    /// `PhyreScene_setObjectRotation(this, id, angles, order)`
    /// (`0x0066db00`, Fahrenheit's name; `thiscall`, `ret 0xc`, from
    /// `graphicObjectSetRotation`): with the object tree up (`+0x10d4`),
    /// every mesh of object `id` (as `PhyreScene_setObjectTranslation`
    /// finds them) gets a new 3x4 world matrix: its own (`*(mesh +4)`,
    /// copied, made a `PMatrix4`) with the upper 3x3 set to 10 times the
    /// unit matrix (`0x00531a40`), then turned about the axes of
    /// [`rotation_order`] (`order`, an `i16`) by `-x` (`Matrix_RotationX`,
    /// applied row by row through `PMatrix4_RowTimes`), `-y`
    /// (`Matrix_RotationY`) and `z` (`0x004cd1f0`; both multiplied in by
    /// `0x005e4400`) of `angles`, each result copied back
    /// (`PMatrix4_Copy`, then float by float), and written back
    /// (`PMatrix4_toWorldMatrix3x4`, float by float, the mesh re-read).
    /// `work` stands for the original's frame. Returns 0.
    #[allow(clippy::too_many_lines)] // the three kinds of turn
    pub fn scene_set_object_rotation(
        h: &mut impl Host,
        scene: u32,
        id: u32,
        angles: u32,
        order: u32,
        work: u32,
    ) -> u32 {
        use crate::managers::bridge::tree_find_key;
        if h.read::<u32>(scene + 0x10d4) == 0 {
            return 0;
        }
        let tree = scene + 0x1198;
        let node = tree_find_key(h, tree, id);
        if node == 0 || node.wrapping_add(0x10) == 0 {
            return 0;
        }
        let node = tree_find_key(h, tree, id);
        let value = if node == 0 { 0 } else { node + 0x10 };
        let list: u32 = h.read(value);
        if h.read::<u32>(list) == 0 {
            return 0;
        }
        let mode = i16::from_le_bytes(order.to_le_bytes()[..2].try_into().unwrap_or([0; 2]));
        let at = |off: u32| work + off;
        let (m4, a, b, w, r) = (at(ROT_M), at(ROT_A), at(ROT_B), at(ROT_W), at(ROT_R));
        let mut i = 0;
        loop {
            let meshes: u32 = h.read(list + 8);
            let mesh: u32 = h.read(meshes + 4 * i);
            let world: u32 = h.read(mesh + 4);
            for k in 0..12 {
                let v: u32 = h.read(world + 4 * k);
                h.write(w + 4 * k, v);
            }
            h.call_sym(PMATRIX4_FROM_3X4, Conv::Thiscall(m4), &[w]);
            let ten = float_bits(f32::from_bits(h.read(h.rebase(ROTATION_SCALE))));
            let s = at(ROT_SCALE);
            for (k, v) in [
                (0, ten),
                (1, 0),
                (2, 0),
                (4, 0),
                (5, ten),
                (6, 0),
                (8, 0),
                (9, 0),
                (10, ten),
            ] {
                h.write(s + 4 * k, v);
            }
            h.call_sym(PMATRIX4_SET_UPPER3X3, Conv::Thiscall(m4), &[s]);
            for (stage, &axis) in rotation_order(mode).iter().enumerate() {
                let first = stage == 0;
                match axis {
                    b'x' => {
                        let angle = negated(h.read(angles));
                        let rot = h.call_sym(MATRIX_ROTATION_X, Conv::Cdecl, &[r, angle]);
                        let temps = ROT_ROWS[stage];
                        let mut rows = [0u32; 4];
                        for (k, (&t, out)) in (0u32..).zip(temps.iter().zip(rows.iter_mut().rev()))
                        {
                            let row = rot.wrapping_add(0x30 - 0x10 * k);
                            *out =
                                h.call_sym(PMATRIX4_ROW_TIMES, Conv::Thiscall(m4), &[at(t), row]);
                        }
                        let (fill, copy) = if first { (b, a) } else { (a, b) };
                        for (k, &row) in (0u32..).zip(&rows) {
                            for j in 0..4 {
                                let v = float_bits(f32::from_bits(h.read(row + 4 * j)));
                                h.write(fill + 0x10 * k + 4 * j, v);
                            }
                        }
                        h.call_sym(PMATRIX4_COPY, Conv::Thiscall(copy), &[fill]);
                        copy_matrix(h, m4, copy);
                    }
                    turn => {
                        let rot = if turn == b'y' {
                            let angle = negated(h.read(angles + 4));
                            h.call_sym(MATRIX_ROTATION_Y, Conv::Cdecl, &[r, angle])
                        } else {
                            let angle = float_bits(f32::from_bits(h.read(angles + 8)));
                            let out = if first { a } else { r };
                            h.call_sym(MATRIX_ROTATION_Z, Conv::Cdecl, &[out, angle])
                        };
                        let out = match (first, turn) {
                            (true, b'y') => a,
                            (true, _) => r,
                            (false, _) => b,
                        };
                        let product = h.call_sym(MATRIX_MUL, Conv::Cdecl, &[out, m4, rot]);
                        let copy = if first { b } else { a };
                        h.call_sym(PMATRIX4_COPY, Conv::Thiscall(copy), &[product]);
                        copy_matrix(h, m4, copy);
                    }
                }
            }
            h.call_sym(PMATRIX4_TO_WORLD_3X4, Conv::Thiscall(w), &[m4]);
            i += 1;
            let meshes: u32 = h.read(list + 8);
            let mesh: u32 = h.read(meshes + 4 * (i - 1));
            let world: u32 = h.read(mesh + 4);
            for k in 0..12 {
                let v = float_bits(f32::from_bits(h.read(w + 4 * k)));
                h.write(world + 4 * k, v);
            }
            if i >= h.read::<u32>(list) {
                return 0;
            }
        }
    }

    /// `g_MenuTextureManager` (name ours): the menus' texture manager.
    pub const MENU_TEXTURE_MANAGER: Global<u32> = Global::new(0x00cc_c820);
    /// `MenuTextureManager_LoadLanguageTextures_0067c920(this, lang)`
    /// (name ours; `thiscall`, `ret 4`): the menu textures of a language
    /// (`ClusterManager_GetInstance` and `0x0069bb00` per path).
    pub const MENU_TEXTURE_LOAD_LANGUAGE: u32 = 0x0067_c920;

    /// `SpecialFeatureDisplay_dtor_SpecialFeatureDisplay(lang)`
    /// (`0x006422e0`, Fahrenheit's name from a Switch match; `cdecl`, from
    /// `TOInit` with `TOGetFFXLangJmp()`):
    /// [`MENU_TEXTURE_LOAD_LANGUAGE`] on `g_MenuTextureManager`, its `eax`.
    pub fn menu_texture_load_language(h: &mut impl Host, lang: u32) -> u32 {
        let mgr = MENU_TEXTURE_MANAGER.get(h);
        h.call_sym(MENU_TEXTURE_LOAD_LANGUAGE, Conv::Thiscall(mgr), &[lang])
    }

    /// `PhyreScene_setObjectRotation(this, id, angles, order)`.
    pub const SCENE_SET_OBJECT_ROTATION: u32 = 0x0066_db00;

    /// `graphicObjectSetRotation(id, angles, order)` (`0x006426a0`):
    /// [`scene_set_object_rotation`] on `g_PhyreScene` (the original's
    /// through the game), its `eax`.
    pub fn graphic_object_set_rotation(h: &mut impl Host, id: u32, angles: u32, order: u32) -> u32 {
        let scene = SCENE.get(h);
        h.call_sym(
            SCENE_SET_OBJECT_ROTATION,
            Conv::Thiscall(scene),
            &[id, angles, order],
        )
    }

    /// `graphicCharacterLightingSetShadeCoeff(list, coeff)`: the shade
    /// coefficient `Phyre_CharacterLighting_calcShadeCoeffWithConfidence`
    /// derives from `coeff` (1.0 on maps `0x226d` and `0x1739`, `coeff`
    /// itself on `0x1c45`, `0xf62` and `0xc04`) to each character of the
    /// list, and to the tweak utility's hook with mode 1 when it is more
    /// than 0.1 from `coeff`, else 2.
    pub fn lighting_set_shade_coeff(h: &mut impl Host, list: u32, coeff: f32) {
        if list == 0 {
            return;
        }
        let arg = float_bits(coeff);
        let l = CHARACTER_LIGHTING.get(h);
        let mut c = h.call_sym_st0(
            LIGHTING_CALC_SHADE_COEFF,
            Conv::Thiscall(l),
            &[list, arg, 0],
        );
        let scene = SCENE.get(h);
        let hash = h.call_sym(SCENE_CURRENT_LEVEL_NAME_HASH, Conv::Thiscall(scene), &[]);
        if hash == 0x226d || hash == 0x1739 {
            c = 1.0;
        }
        if hash == 0x1c45 || hash == 0xf62 || hash == 0xc04 {
            c = f32::from_bits(arg);
        }
        let count = h.read::<u32>(list) & 0x7fff_ffff;
        if count == 0 {
            return;
        }
        let c = c.to_bits();
        // |c - coeff|, as stored
        let d = x87::store(x87::sub(x87::load(c), x87::load(arg))) & 0x7fff_ffff;
        let d = x87::load(d);
        // fcomp 0.1; test ah, 0x41: 1 only when greater (not unordered)
        let mode = if d > f64::from(0.1f32) { 1 } else { 2 };
        let mut i = 0;
        while i < h.read::<u32>(list) & 0x7fff_ffff {
            let items: u32 = h.read(list + 4);
            let chr: u32 = h.read(items + 4 * i);
            let l = CHARACTER_LIGHTING.get(h);
            h.call_sym(LIGHTING_SET_SHADE_COEFF, Conv::Thiscall(l), &[chr, c]);
            let tweak = LIGHTING_TWEAK.get(h);
            h.call_sym(
                LIGHTING_TWEAK_SET_SHADE_COEFF,
                Conv::Thiscall(tweak),
                &[chr, c, mode],
            );
            i += 1;
        }
    }

    /// `PhyreScene_SetMeshesVisible(this, on)` (`0x0066e7b0`): every mesh of
    /// the list at `+0x11d0` (`+0x11c8` of them) with a draw entry gets
    /// its draw pointer (`+0x1c` of the entry) set to the mesh, or 0.
    pub fn scene_set_meshes_visible(m: &mut impl Mem, scene: u32, on: u32) {
        let on = on & 0xff != 0;
        let mut i = 0;
        while i < m.read::<u32>(scene + 0x11c8) {
            let meshes: u32 = m.read(scene + 0x11d0);
            let mesh: u32 = m.read(meshes + 4 * i);
            if mesh != 0 {
                let draw: u32 = m.read(mesh + 0x1c);
                if draw != 0 {
                    m.write(draw + 0x1c, if on { mesh } else { 0 });
                }
            }
            i += 1;
        }
    }

    /// `PhyreScene_SetColourPair(this, r, g, b, r2, g2, b2, second)`
    /// (`0x0066ea70`): two RGBA colours at `+0xfc8c` (or `+0xfcac` with
    /// `second`): `(r, g, b, 1)` and `(r2, g2, b2, 0)`.
    pub fn scene_set_colour_pair(m: &mut impl Mem, scene: u32, c: [f32; 6], second: u32) {
        let at = scene + if second & 0xff != 0 { 0xfcac } else { 0xfc8c };
        let w = [
            float_bits(c[0]),
            float_bits(c[1]),
            float_bits(c[2]),
            1f32.to_bits(),
            float_bits(c[3]),
            float_bits(c[4]),
            float_bits(c[5]),
            0,
        ];
        for (i, v) in (0u32..).zip(w) {
            m.write(at + 4 * i, v);
        }
    }

    /// `SegmentList_AllReady(this)` (`0x00652250`, fastcall): when
    /// pending (`+0x24`), and every segment of `+0x54` (`+0x58` of them)
    /// is null or its scene state (`*(scene +0x60)[i]`) is 1, clears the
    /// pending byte and returns 1; else 0.
    pub fn segments_ready(m: &mut impl Mem, this: u32) -> u32 {
        if m.read::<u8>(this + 0x24) == 0 {
            return 0;
        }
        let n: u32 = m.read(this + 0x58);
        let list: u32 = m.read(this + 0x54);
        for i in 0..n {
            let seg: u32 = m.read(list + 4 * i);
            if seg != 0 {
                let scene = SCENE.get(m);
                let states: u32 = m.read(scene + 0x60);
                if m.read::<u32>(states + 4 * i) != 1 {
                    return 0;
                }
            }
        }
        m.write(this + 0x24, 0u8);
        1
    }

    /// `g_Settings`: the game settings object (a pointer).
    pub const SETTINGS: Global<u32> = Global::new(0x022f_b504);
    /// `ShadowManager_getInstance`.
    pub const SHADOW_GET_INSTANCE: u32 = 0x006d_2390;
    /// `ShadowManager_SetQuality(this, q)`.
    pub const SHADOW_SET_QUALITY: u32 = 0x006c_eaa0;
    /// The shadow buffers' resize (`0x006d2df0`, `thiscall(sm)`).
    pub const SHADOW_RESIZE: u32 = 0x006d_2df0;
    /// The post-process's texture rebind (`0x00697e10`, `thiscall(pp, v)`).
    pub const POST_PROCESS_REBIND: u32 = 0x0069_7e10;

    /// `PhyreScene_ApplySettings(this)` (`0x0066f010`): the graphics
    /// settings (`g_Settings`) to the scene: `+0x24` as a float to the
    /// post-process (`*(scene +0x10cc) +0x12328`), the shadow quality
    /// (`+0x30`, then the shadow buffers resized), `+0x44 != 0` to the
    /// post-process byte `+0x123d3`, `+0x48` to its `+0x1264`, `+0x38` to
    /// its rebind, and the anti-aliasing mode (`+0x40`): 1 sets the scene's
    /// `+0x10dd8` to 1, 2-5 set it to 0 and the post-process's `+0x1231c`
    /// to 1-4, anything else sets it to 2. Returns `eax` (the mode - 1, or
    /// the post-process).
    pub fn scene_apply_settings(h: &mut impl Host, scene: u32) -> u32 {
        let cfg = SETTINGS.get(h);
        #[allow(clippy::cast_possible_wrap)] // fild: signed
        let f = x87::store(f64::from(h.read::<u32>(cfg + 0x24) as i32));
        let pp: u32 = h.read(scene + 0x10cc);
        h.write(pp + 0x12328, f);
        let cfg = SETTINGS.get(h);
        let q: u32 = h.read(cfg + 0x30);
        let sm = h.call_sym(SHADOW_GET_INSTANCE, Conv::Cdecl, &[]);
        h.call_sym(SHADOW_SET_QUALITY, Conv::Thiscall(sm), &[q]);
        let sm = h.call_sym(SHADOW_GET_INSTANCE, Conv::Cdecl, &[]);
        h.call_sym(SHADOW_RESIZE, Conv::Thiscall(sm), &[]);
        let cfg = SETTINGS.get(h);
        let on = u8::from(h.read::<u32>(cfg + 0x44) != 0);
        let pp: u32 = h.read(scene + 0x10cc);
        h.write(pp + 0x123d3, on);
        let cfg = SETTINGS.get(h);
        let v: u32 = h.read(cfg + 0x48);
        let pp: u32 = h.read(scene + 0x10cc);
        h.write(pp + 0x1264, v);
        let cfg = SETTINGS.get(h);
        let v: u32 = h.read(cfg + 0x38);
        let pp: u32 = h.read(scene + 0x10cc);
        h.call_sym(POST_PROCESS_REBIND, Conv::Thiscall(pp), &[v]);
        let cfg = SETTINGS.get(h);
        let mode = h.read::<u32>(cfg + 0x40).wrapping_sub(1);
        match mode {
            0 => {
                h.write(scene + 0x10dd8, 1u32);
                mode
            }
            1..=4 => {
                let pp: u32 = h.read(scene + 0x10cc);
                h.write(scene + 0x10dd8, 0u32);
                h.write(pp + 0x1231c, mode);
                pp
            }
            _ => {
                h.write(scene + 0x10dd8, 2u32);
                mode
            }
        }
    }

    /// `graphicSetTextureVideoInfo(a, b)`: `PhyreScene_setTextureVideoInfo`
    /// and `g_GraphicTextureVideoInfo = b << 8 | a` (16 bits). Returns the
    /// scene call's `eax`.
    pub fn set_texture_video_info(h: &mut impl Host, a: u32, b: u32) -> u32 {
        let scene = SCENE.get(h);
        let r = h.call_sym(SCENE_SET_TEXTURE_VIDEO_INFO, Conv::Thiscall(scene), &[a, b]);
        #[allow(clippy::cast_possible_truncation)] // a 16-bit store
        crate::scene::bridge::TEXTURE_VIDEO_INFO.set(h, (b << 8 | a) as u16);
        r
    }

    /// `graphicUpdateCrossFade(weight, time)`: the post-process cross
    /// fade's weight and time.
    pub fn update_cross_fade(h: &mut impl Host, weight: u32, time: u32) -> u32 {
        let pp = POST_PROCESS.get(h);
        h.call_sym(
            POST_PROCESS_SET_CROSS_FADE_WEIGHT,
            Conv::Thiscall(pp),
            &[weight],
        );
        let pp = POST_PROCESS.get(h);
        h.call_sym(
            POST_PROCESS_SET_CROSS_FADE_TIME,
            Conv::Thiscall(pp),
            &[time],
        )
    }

    /// `PhyreScene_setCurrentLevelName(this, name)`: the level name
    /// (`+0xfaf0`, a `PString`).
    pub fn scene_set_current_level_name(h: &mut impl Host, scene: u32, name: u32) -> u32 {
        h.call_sym(PSTRING_ASSIGN, Conv::Thiscall(scene + 0xfaf0), &[name])
    }
}
