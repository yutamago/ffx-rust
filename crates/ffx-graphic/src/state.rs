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
        let m: [u32; 16] = if zero_row {
            let mut id = [0u32; 16];
            for i in [0, 5, 10, 15] {
                id[i] = 1f32.to_bits();
            }
            id
        } else {
            let s = h.scratch();
            for (i, v) in (0u32..).zip(w) {
                h.write(s + 4 * i, float_bits(f32::from_bits(v)));
            }
            h.call_sym(INVERSE_MATRIX, Conv::Cdecl, &[s, s]);
            let mut m = words::<16>(h, s);
            // fld/fchs/fstp, in the original's order
            for i in [0, 1, 4, 5, 8, 9, 12, 13, 4, 5, 6, 8, 9, 10] {
                m[i] = float_bits(f32::from_bits(m[i])) ^ 0x8000_0000;
            }
            m
        };
        let out = [
            m[4], m[5], m[6], m[0], m[8], m[9], m[10], m[1], m[12], m[13], m[14], m[2],
        ];
        let scene = SCENE.get(h);
        for (i, v) in (0u32..).zip(out) {
            h.write(scene + 0xfa04 + 4 * i, float_bits(f32::from_bits(v)));
        }
    }

    /// `PhyreScene_setCameraWorldMatrix(this, m, alt)`: the 3x4 matrix to
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

    /// `PhyreScene_setCameraFOVy(this, fovy)`: the main camera (`+0xdc`),
    /// and in a 3D scene the minimap camera (`+0x654`, with fixed angles
    /// on maps `0xec1`, `0x107` and `0xf48`); the reflection camera
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
    /// in a 3D scene with 2D layers (`+0x1128`), the first layer of the
    /// current group (`+0x11d8`) with that id gets its shown byte, and its
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
