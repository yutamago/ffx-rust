//! The render-side glue, second batch (M4 port round 1, list
//! `h3-glue-render`): the post effects' setters (blur, distortion, motion
//! blur, fog, the broken screen), the menus' and the sphere grid's
//! entry points, the async load tasks, the FMV player's, and
//! `ShadowManager_Initialize`.
//!
//! As in [`crate::glue`], a forwarder returns what its original leaves in
//! `eax`: its callee's `eax` where the callee sets it on every path, the
//! instance where the callee is a store that leaves `eax` alone (and then
//! the stub in the test leaves it alone too), and nothing (`()`) where the
//! original leaves its caller's `eax` on some path and no caller reads it
//! (check-clobber).

use ffx_bridge::Global;

pub use crate::glue::{
    ASYNC_LOAD_INSTANCE, BROKEN_SCREEN_INSTANCE, FMV_INSTANCE, GET_APPLICATION, SCENE,
    VFX_SHADER_INSTANCE,
};

/// `g_GraphicAbmap`, the sphere grid's render object.
pub const GRAPHIC_ABMAP: Global<u32> = Global::new(0x00cc_c838);
/// `g_GraphicUi`, the remaster's UI object.
pub const GRAPHIC_UI: Global<u32> = Global::new(0x00cc_c81c);
/// `g_MenuTextureManager`.
pub const MENU_TEXTURE_MANAGER: Global<u32> = crate::glue::MENU_TEXTURE_MANAGER;
/// `g_GraphicSkipFrame`: set while a frame is skipped.
pub const GRAPHIC_SKIP_FRAME: Global<u32> = Global::new(0x012f_b7d0);
/// `g_GraphicMagicFileAsyncLoad`.
pub const MAGIC_FILE_ASYNC_LOAD: Global<u32> = Global::new(0x00cc_b464);
/// `g_RenderDelegationHelper`.
pub const RENDER_DELEGATION_HELPER: Global<u32> = Global::new(0x00cc_ca98);
/// `g_ShadowSize`.
pub const SHADOW_SIZE: Global<i32> = Global::new(0x00c3_4408);
/// `g_IggyMenuManager`.
pub const IGGY_MENU_MANAGER: Global<u32> = Global::new(0x0198_4c6c);
/// The model buffers `ModelBuffer_ClearAll` zeroes (`0x18c00` bytes).
pub const MODEL_BUFFERS: u32 = 0x00d3_2860;
/// Their length.
pub const MODEL_BUFFERS_LEN: u32 = 0x18c00;
/// The model buffer index `ModelBuffer_ClearAll` sets to -1.
pub const MODEL_BUFFER_INDEX: Global<u32> = Global::new(0x00d4_b2d4);
/// The distortion's start blur (an f32 constant in `.rdata`).
pub const DISTORTION_START_BLUR: u32 = 0x00b9_2180;

// ---- getters (cdecl, no arguments) ----

/// `BlurEffectEx_GetInstance` (prints when there is none).
pub const BLUR_EX_INSTANCE: u32 = 0x006c_4a70;
/// `Distortion_GetInstance` (prints when there is none).
pub const DISTORTION_INSTANCE: u32 = 0x006c_2290;
/// `MotionBlur_GetInstance` (prints when there is none).
pub const MOTION_BLUR_INSTANCE: u32 = 0x006c_5130;
/// `Phyre_FogEffectManager_GetInstance`.
pub const FOG_INSTANCE: u32 = 0x006e_32c0;
/// `ClusterManager_GetInstance`.
pub const CLUSTER_INSTANCE: u32 = 0x0069_a1a0;
/// `LocalizationManager_GetInstance` (prints when there is none).
pub const LOCALIZATION_INSTANCE: u32 = 0x006d_b070;
/// `MorphManager_GetInstance`.
pub const MORPH_INSTANCE: u32 = 0x006a_bf40;
/// `TextureAnimationManager_GetInstance`.
pub const TEX_ANIM_INSTANCE: u32 = 0x006b_02e0;
/// `PostProcessManager_GetInstance`.
pub const POST_PROCESS_INSTANCE: u32 = 0x0069_2be0;
/// `Phyre_PRendering_PRenderThread_GetInstance`.
pub const RENDER_THREAD_INSTANCE: u32 = 0x0061_4410;
/// `Virtuos_InputManager_getInstance`.
pub const INPUT_INSTANCE: u32 = 0x0063_1300;
/// `GetGlobal01984c68_00a25a30`.
pub const IGGY_GLOBAL: u32 = 0x00a2_5a30;

// ---- methods (thiscall unless noted; `ret n` as the words they take) ----

/// `BlurEffect_setTarget(this, target)`.
pub const BLUR_SET_TARGET: u32 = 0x006c_4a90;
/// `BlurEffect_SetField84(this, v)`.
pub const BLUR_SET_FIELD84: u32 = 0x006c_44d0;
/// `BlurEffect_SetFloats6c(this, a, b)` (leaves `eax`).
pub const BLUR_SET_FLOATS6C: u32 = 0x006c_4710;
/// `VfxShaderManager_loadSpecialShaderTableForMagicFile(this, v)`: an
/// empty `ret 4` (leaves `eax`; too short to hook).
pub const VFX_SHADER_LOAD_MAGIC: u32 = 0x006c_0fb0;
/// `AbmapManager_GetVertexInfo(this, a, b)`.
pub const ABMAP_GET_VERTEX_INFO: u32 = 0x0068_4cd0;
/// `AbmapManager_ResetLayers_006851d0(this, v)`.
pub const ABMAP_RESET_LAYERS: u32 = 0x0068_51d0;
/// `AbmapManager_ToDestroy(this)` (leaves `eax`).
pub const ABMAP_TO_DESTROY: u32 = 0x0068_57b0;
/// The sphere grid's creation (`0x00684a60`, cdecl `(a)`).
pub const ABMAP_CREATE: u32 = 0x0068_4a60;
/// `MapPart_LangOverride(this, a, b)`.
pub const UI_LANG_OVERRIDE: u32 = 0x0067_fb30;
/// `graphicSetField5b8c_0067e4d0(this, v)`.
pub const UI_SET_DRAW_TYPE: u32 = 0x0067_e4d0;
/// The UI's menu reset (`0x0067dfc0`).
pub const UI_MENU_RESET: u32 = 0x0067_dfc0;
/// `MenuManager_UpdateMenu(this)`.
pub const UI_MENU_UPDATE: u32 = 0x0067_faa0;
/// `PhyreScene_setChadist(this, f)`: an empty `ret 4` (too short to hook).
pub const SCENE_SET_CHADIST: u32 = 0x0066_cf70;
/// `PhyreScene_addChrDrawSituation(this, chr)`.
pub const SCENE_ADD_CHR_DRAW_SITUATION: u32 = 0x0065_64a0;
/// `PhyreScene_textureVideoUnload(this)` (leaves `eax`).
pub const SCENE_TEXTURE_VIDEO_UNLOAD: u32 = 0x0067_0760;
/// `PhyreScene_fieldMapUnload(this)`.
pub const SCENE_FIELD_MAP_UNLOAD: u32 = 0x0065_d240;
/// `PhyreScene_render(this)`.
pub const SCENE_RENDER: u32 = 0x0066_8930;
/// `PhyreScene_doDeleteMeshInstances(this)`.
pub const SCENE_DO_DELETE_MESH_INSTANCES: u32 = 0x0065_a620;
/// `PhyreScene_setMaterialVisibilityBySetAlpha(material, alpha)` (`ret 8`,
/// reads no `this`; leaves `eax`).
pub const SCENE_SET_MATERIAL_VISIBILITY: u32 = 0x0066_d5c0;
/// `PhyreScene_loadCharacter(this, entry, b)`.
pub const SCENE_LOAD_CHARACTER: u32 = 0x0066_3f20;
/// `PhyreScene_loadMagicFileVFXTexture(this, a, entry, c)`.
pub const SCENE_LOAD_MAGIC_FILE_VFX_TEXTURE: u32 = 0x0066_4550;
/// `PhyreScene_loadVFXTexture(this, a, entry, c)`.
pub const SCENE_LOAD_VFX_TEXTURE: u32 = 0x0066_5770;
/// `PhyreScene_onMenuTextureLoaded` (the callback `loadMenuTexture` hands
/// the loader).
pub const SCENE_ON_MENU_TEXTURE_LOADED: u32 = 0x0066_6740;
/// The scene's cut-buffer shader (written by `onCutbufferShaderLoaded`).
pub const SCENE_CUTBUFFER_SHADER: u32 = 0xfaa4;
/// The scene's "magic file async load off" byte.
pub const SCENE_MAGIC_ASYNC_OFF: u32 = 0x24;
/// The scene's Iggy menu.
pub const SCENE_IGGY_MENU: u32 = 0x0001_0dc8;
/// `Distortion_updateDistortionBlur(this, f)`.
pub const DISTORTION_UPDATE_BLUR: u32 = 0x006c_3230;
/// The fog limit setter (`0x006e3aa0`, `(this, f)`).
pub const FOG_SET_LIMIT: u32 = 0x006e_3aa0;
/// `MotionBlur_setMotionBlurWeight(this, f)`.
pub const MOTION_BLUR_SET_WEIGHT: u32 = 0x006c_5600;
/// `BrokenScreenEffect_setOpacity(this, f)` (leaves `eax`).
pub const BROKEN_SET_OPACITY: u32 = 0x006d_59e0;
/// The block data setup (`0x006d55b0`, `(this, a, b, c)`).
pub const BROKEN_INIT_BLOCK_DATA: u32 = 0x006d_55b0;
/// `BrokenScreenEffect_updateLightParameters(this, 7 words)`.
pub const BROKEN_UPDATE_LIGHT: u32 = 0x006d_5a70;
/// The polygon lookup (`0x006d5190`, `(this, a, b)` -> polygon).
pub const BROKEN_POLYGON: u32 = 0x006d_5190;
/// The polygon's matrix update (`0x006d4a90`, `(polygon, m)`).
pub const BROKEN_POLYGON_MATRIX: u32 = 0x006d_4a90;
/// `ClusterManager_forceReleasePCluster(this, id)`.
pub const CLUSTER_FORCE_RELEASE: u32 = 0x0069_b310;
/// `ClusterManager_releasePCluster_0069be90(this, id)`.
pub const CLUSTER_RELEASE: u32 = 0x0069_be90;
/// `MenuTextureManager_LoadSpecialMenuTexture(this, a, b, c)`.
pub const MENU_LOAD_SPECIAL: u32 = 0x0067_d550;
/// `LocalizationManager_getText(this, a, b, c)`.
pub const LOCALIZATION_GET_TEXT: u32 = 0x006d_b1f0;
/// The morph manager's material UV (`0x006abfc0`, `(this, a, b, c)`).
pub const MORPH_SET_MATERIAL_UV: u32 = 0x006a_bfc0;
/// `TextureAnimationManager_SetPaused(this, a, paused)`.
pub const TEX_ANIM_SET_PAUSED: u32 = 0x006b_3600;
/// `TextureAnimationManager_getMaterialByLayNum(this, layer)`.
pub const TEX_ANIM_MATERIAL_BY_LAYER: u32 = 0x006b_2070;
/// `PostProcessManager_setCrossFadeCaptureEnable(this, on)` (leaves `eax`).
pub const POST_SET_CROSS_FADE_CAPTURE: u32 = 0x0069_8490;
/// `PostProcessManager_SetByte123bb(this, on)` (leaves `eax`).
pub const POST_SET_BYTE_123BB: u32 = 0x0069_8480;
/// `PostProcessManager_copyBuffer(this, 6 words)`.
pub const POST_COPY_BUFFER: u32 = 0x0069_2c40;
/// `PhyFMVPlayerManager_PlayVideo(this, on)`.
pub const FMV_PLAY_VIDEO: u32 = 0x006d_7430;
/// `PhyFMVPlayerManager_initialize(this, a, b, c)`.
pub const FMV_INITIALIZE: u32 = 0x006d_9c80;
/// `PhyFMVPlayerManager_handleInput(this, app)` (leaves `eax`).
pub const FMV_HANDLE_INPUT: u32 = 0x006d_9460;
/// `PhyFMVPlayerManager_UpdateTexture(this)`.
pub const FMV_UPDATE_TEXTURE: u32 = 0x006d_7680;
/// `RenderDelegationHelper_registerDelegation(this, 4 words)`.
pub const REGISTER_DELEGATION: u32 = 0x006a_de20;
/// The screen-fade reset delegate (`0x00637c50`) it registers.
pub const RESET_SCREEN_FADE_DELEGATE: u32 = 0x0063_7c50;
/// `Phyre_PRendering_PRenderThread_sync(this)`.
pub const RENDER_THREAD_SYNC: u32 = 0x0061_5ea0;
/// `DepthBiasHooks_clearDepthBias()` (cdecl).
pub const CLEAR_DEPTH_BIAS: u32 = 0x006e_40b0;
/// `JobScheduleWait_pppPart_fillBuffer()` (cdecl).
pub const WAIT_PPP_FILL: u32 = 0x0071_e810;
/// `AsyncLoadManager_loadCluster(mgr, a, id, callback, e)` (cdecl).
pub const ASYNC_LOAD_CLUSTER: u32 = 0x0069_1660;
/// `AsyncLoadManager_registAsyncLoadTask(mgr, callback, kind, arg, 0,
/// i64 b)` (cdecl, 7 words).
pub const ASYNC_REGIST_TASK: u32 = 0x0069_18b0;
/// `graphicAsyncTaskCallback` (the task callback the graphic layer
/// registers).
pub const ASYNC_TASK_CALLBACK: u32 = 0x0063_7860;
/// `operator_delete_00630540(p)` (cdecl).
pub const OPERATOR_DELETE_TASK: u32 = 0x0063_0540;
/// `operator_delete(p)` (cdecl).
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
/// `IggyMenu_dtor_IggyMenu(this)` (leaves `eax`).
pub const IGGY_MENU_DTOR: u32 = 0x00a2_8a40;
/// `ShadowManager_loadShadowShaderBinFiles(this)`.
pub const SHADOW_LOAD_SHADERS: u32 = 0x006d_2c10;
/// `ShadowManager_createshadowbuffer(this, w, h)`.
pub const SHADOW_CREATE_BUFFER: u32 = 0x006d_18b0;
/// The shadow set-up `ShadowManager_Initialize` ends with (`0x006d1b40`).
pub const SHADOW_SETUP: u32 = 0x006d_1b40;
/// `Virtuos_InputManager_updateInput(this)`.
pub const INPUT_UPDATE: u32 = 0x0063_2a30;
/// The Iggy global's update (`0x00a25ab0`).
pub const IGGY_GLOBAL_UPDATE: u32 = 0x00a2_5ab0;
/// The Iggy menu manager's update (`0x00a26ce0`).
pub const IGGY_MENU_UPDATE: u32 = 0x00a2_6ce0;
/// `inputClear()` (cdecl).
pub const INPUT_CLEAR: u32 = 0x0063_0c30;
/// `PhyreFIOS_LoadPrxInProgress` through its thunk `0x006793c0` (cdecl).
pub const LOAD_PRX_IN_PROGRESS: u32 = 0x0067_93c0;
/// `crt_memset_thunk(dst, byte, n)` (cdecl).
pub const MEMSET: u32 = 0x0094_964c;
/// `Phyre_PFramework_PWindowWin32Base_SetFlipVSyncInterval`'s neighbour:
/// the application's "no vsync" byte and its "changed" byte.
pub const APP_NO_VSYNC: u32 = 0x1c;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use crate::glue::bridge::{instance, on_global, on_instance};
    use ffx_bridge::Mem;
    use ffx_bridge::x87::round_trip;
    use ffx_graphic::host::{Conv, Host};

    /// The bits of `f` after `fld dword` / `fstp dword` (a signalling NaN
    /// becomes quiet).
    fn rt(f: u32) -> u32 {
        round_trip(f32::from_bits(f)).to_bits()
    }

    // ---- post effects ----

    /// `graphicActivateBlurEffectEx(target)` `0x00639110`.
    pub fn activate_blur_effect_ex(h: &mut impl Host, target: u32) -> u32 {
        on_instance(h, BLUR_EX_INSTANCE, BLUR_SET_TARGET, &[target])
    }
    /// `graphicSetFlipVsnc_00643810(v)`: the blur effect's (`[instance]`)
    /// `SetField84(v)`.
    pub fn blur_set_field84(h: &mut impl Host, v: u32) -> u32 {
        let ex = instance(h, BLUR_EX_INSTANCE);
        let blur: u32 = h.read(ex);
        h.call_sym(BLUR_SET_FIELD84, Conv::Thiscall(blur), &[v])
    }
    /// `graphicBlurEffectSetFloats6c_00643890(a, b)`: the blur effect's
    /// `SetFloats6c(a, b)` (floats through `fld`/`fstp`). Returns the
    /// instance, which the setter leaves in `eax`.
    pub fn blur_set_floats(h: &mut impl Host, a: u32, b: u32) -> u32 {
        let ex = instance(h, BLUR_EX_INSTANCE);
        let blur: u32 = h.read(ex);
        h.call_sym(BLUR_SET_FLOATS6C, Conv::Thiscall(blur), &[rt(a), rt(b)]);
        ex
    }
    /// `graphicMagicVFXLoadSpecialShaderTable(v)` `0x00642090`: the empty
    /// `loadSpecialShaderTableForMagicFile(v)`; returns the instance.
    pub fn magic_vfx_load_special_shader_table(h: &mut impl Host, v: u32) -> u32 {
        let mgr = instance(h, VFX_SHADER_INSTANCE);
        h.call_sym(VFX_SHADER_LOAD_MAGIC, Conv::Thiscall(mgr), &[v]);
        mgr
    }
    /// `graphicSetDistortionSkipped(on)` `0x00642f80`: byte `+0x7d` =
    /// `on != 0`. Returns the instance.
    pub fn set_distortion_skipped(h: &mut impl Host, on: u32) -> u32 {
        let d = instance(h, DISTORTION_INSTANCE);
        h.write(d + 0x7d, u8::from(on != 0));
        d
    }
    /// `graphicSetDistortionEnable(on)` `0x00642f50`: byte `+0x7c` =
    /// `on != 0`; when it turns on, byte `+0x7e` = 1 and the blur `+0x78`
    /// = the start blur constant. Returns the instance.
    pub fn set_distortion_enable(h: &mut impl Host, on: u32) -> u32 {
        let d = instance(h, DISTORTION_INSTANCE);
        let on = u8::from(on != 0);
        let was: u8 = h.read(d + 0x7c);
        h.write(d + 0x7c, on);
        if was == 0 && on == 1 {
            let start: f32 = h.read(h.rebase(DISTORTION_START_BLUR));
            h.write(d + 0x7e, on);
            h.write(d + 0x78, round_trip(start));
        }
        d
    }
    /// `graphicSetDistortionBlur(f)` `0x00642f30`.
    pub fn set_distortion_blur(h: &mut impl Host, f: u32) -> u32 {
        on_instance(h, DISTORTION_INSTANCE, DISTORTION_UPDATE_BLUR, &[rt(f)])
    }
    /// `graphicSetFogLimit(f)` `0x006431c0`.
    pub fn set_fog_limit(h: &mut impl Host, f: u32) -> u32 {
        on_instance(h, FOG_INSTANCE, FOG_SET_LIMIT, &[rt(f)])
    }
    /// `graphicSetMotionBlurWeight(f)` `0x00643750` (the setter leaves its
    /// caller's `eax` on some paths; no caller reads it).
    pub fn set_motion_blur_weight(h: &mut impl Host, f: u32) {
        on_instance(h, MOTION_BLUR_INSTANCE, MOTION_BLUR_SET_WEIGHT, &[rt(f)]);
    }
    /// `graphicSetMotionBlurEnable(on)` `0x00643730`: byte `+0x58` =
    /// `on != 0`, and byte `+0x59` = 1 when on. Returns the instance.
    pub fn set_motion_blur_enable(h: &mut impl Host, on: u32) -> u32 {
        let mb = instance(h, MOTION_BLUR_INSTANCE);
        let on = on != 0;
        h.write(mb + 0x58, u8::from(on));
        if on {
            h.write(mb + 0x59, 1u8);
        }
        mb
    }
    /// `graphicCrossFadeCaptureStart_00639150()`: the post-process
    /// manager's cross-fade capture and its byte `+0x123bb` on (both
    /// leave `eax`); returns the instance.
    pub fn cross_fade_capture_start(h: &mut impl Host) -> u32 {
        let pp = instance(h, POST_PROCESS_INSTANCE);
        h.call_sym(POST_SET_CROSS_FADE_CAPTURE, Conv::Thiscall(pp), &[1]);
        let pp = instance(h, POST_PROCESS_INSTANCE);
        h.call_sym(POST_SET_BYTE_123BB, Conv::Thiscall(pp), &[1]);
        pp
    }
    /// `WaterDistortionEffectDelegations_captureFrame(p)` `0x006f5420`:
    /// with `p`, `copyBuffer(p[0], p[1], p[2], 1, 0, 0)`; 0 without.
    pub fn water_distortion_capture_frame(h: &mut impl Host, p: u32) -> u32 {
        if p == 0 {
            return 0;
        }
        let a: u32 = h.read(p);
        let b: u32 = h.read(p + 4);
        let c: u32 = h.read(p + 8);
        on_instance(
            h,
            POST_PROCESS_INSTANCE,
            POST_COPY_BUFFER,
            &[a, b, c, 1, 0, 0],
        )
    }

    // ---- the broken screen ----

    /// `graphicBrokenScreenUpdateOpacity(f)` `0x006394e0`: `setOpacity`
    /// (a store); returns the instance.
    pub fn broken_screen_update_opacity(h: &mut impl Host, f: u32) -> u32 {
        let b = instance(h, BROKEN_SCREEN_INSTANCE);
        h.call_sym(BROKEN_SET_OPACITY, Conv::Thiscall(b), &[rt(f)]);
        b
    }
    /// `graphicBrokenScreenInitializeBlockDataWithGivenBuffer(a, b, c)`
    /// `0x00639470`.
    pub fn broken_screen_init_block_data(h: &mut impl Host, a: u32, b: u32, c: u32) -> u32 {
        on_instance(
            h,
            BROKEN_SCREEN_INSTANCE,
            BROKEN_INIT_BLOCK_DATA,
            &[a, b, c],
        )
    }
    /// `graphicBrokenScreenUpdateLightParameters(7 words)` `0x006394b0`.
    pub fn broken_screen_update_light_parameters(h: &mut impl Host, args: [u32; 7]) -> u32 {
        on_instance(h, BROKEN_SCREEN_INSTANCE, BROKEN_UPDATE_LIGHT, &args)
    }
    /// `graphicBrokenScreenUpdatePolygonWMatrix(a, b, m)` `0x00639540`:
    /// the polygon `(a, b)`'s matrix update with `m`.
    pub fn broken_screen_update_polygon_matrix(h: &mut impl Host, a: u32, b: u32, m: u32) -> u32 {
        let polygon = on_instance(h, BROKEN_SCREEN_INSTANCE, BROKEN_POLYGON, &[a, b]);
        h.call_sym(BROKEN_POLYGON_MATRIX, Conv::Thiscall(polygon), &[m])
    }
    /// `graphicBrokenScreenBlockBytes_006393a0(n)`: asks for the instance
    /// (unused) and returns `((4n - 1) >> 4 + 23n + 1) << 4`.
    pub fn broken_screen_block_bytes(h: &mut impl Host, n: u32) -> u32 {
        instance(h, BROKEN_SCREEN_INSTANCE);
        let a = n.wrapping_mul(4).wrapping_sub(1) >> 4;
        let b = n.wrapping_mul(0x17).wrapping_add(1);
        a.wrapping_add(b) << 4
    }

    // ---- scene ----

    /// `graphicSetChadist(f)` `0x00642ef0`: the empty `setChadist(f)`;
    /// `eax` is the caller's.
    pub fn set_chadist(h: &mut impl Host, f: u32) {
        on_global(h, SCENE, SCENE_SET_CHADIST, &[rt(f)]);
    }
    /// `graphicCharacterInsertDrawSituation(chr)` `0x00639ff0`: unless the
    /// frame is skipped.
    pub fn character_insert_draw_situation(h: &mut impl Host, chr: u32) {
        if GRAPHIC_SKIP_FRAME.get(h) == 0 {
            on_global(h, SCENE, SCENE_ADD_CHR_DRAW_SITUATION, &[chr]);
        }
    }
    /// `graphicFieldMapUnload()` `0x006403c0`: depth bias, texture video
    /// and the field map unloaded; returns 0.
    pub fn field_map_unload(h: &mut impl Host) -> u32 {
        h.call_sym(CLEAR_DEPTH_BIAS, Conv::Cdecl, &[]);
        on_global(h, SCENE, SCENE_TEXTURE_VIDEO_UNLOAD, &[]);
        on_global(h, SCENE, SCENE_FIELD_MAP_UNLOAD, &[]);
        0
    }
    /// `graphicRender()` `0x00642bb0`: the scene's render, the particle
    /// fill job's wait, the deferred mesh deletes; returns the render's
    /// result.
    pub fn render(h: &mut impl Host) -> u32 {
        let r = on_global(h, SCENE, SCENE_RENDER, &[]);
        h.call_sym(WAIT_PPP_FILL, Conv::Cdecl, &[]);
        on_global(h, SCENE, SCENE_DO_DELETE_MESH_INSTANCES, &[]);
        r
    }
    /// `graphicTextureAnimationSetMaterialVisiability(layer, alpha)`
    /// `0x00644120`: the layer's material's visibility; returns the
    /// material (0 without one), which the setter leaves in `eax`.
    pub fn texture_animation_set_material_visibility(
        h: &mut impl Host,
        layer: u32,
        alpha: u32,
    ) -> u32 {
        let material = on_instance(h, TEX_ANIM_INSTANCE, TEX_ANIM_MATERIAL_BY_LAYER, &[layer]);
        if material != 0 {
            on_global(h, SCENE, SCENE_SET_MATERIAL_VISIBILITY, &[material, alpha]);
        }
        material
    }
    /// `graphicSetMagicFileAsyncLoadEnable(v)` `0x006435e0`: unless a PRX
    /// load is running or the scene's byte `+0x24` is 1, stores `v` in
    /// `g_GraphicMagicFileAsyncLoad`. Returns the last value the original
    /// leaves in `eax`: the PRX check, the scene, or `v`.
    pub fn set_magic_file_async_load_enable(h: &mut impl Host, v: u32) -> u32 {
        let busy = h.call_sym(LOAD_PRX_IN_PROGRESS, Conv::Cdecl, &[]);
        if busy != 0 {
            return busy;
        }
        let scene = SCENE.get(h);
        if h.read::<u8>(scene + SCENE_MAGIC_ASYNC_OFF) == 1 {
            return scene;
        }
        MAGIC_FILE_ASYNC_LOAD.set(h, v);
        v
    }
    /// `PhyreScene_forceUnloadMenuTexture(this, id)` `0x0065d780`: the
    /// cluster's forced release; returns 0.
    pub fn scene_force_unload_menu_texture(h: &mut impl Host, id: u32) -> u32 {
        on_instance(h, CLUSTER_INSTANCE, CLUSTER_FORCE_RELEASE, &[id]);
        0
    }
    /// `PhyreScene_unloadMenuTexture(this, id)` `0x00671120`: the cluster's
    /// release; returns 0.
    pub fn scene_unload_menu_texture(h: &mut impl Host, id: u32) -> u32 {
        on_instance(h, CLUSTER_INSTANCE, CLUSTER_RELEASE, &[id]);
        0
    }
    /// `PhyreScene_loadMenuTexture(this, id)` `0x006651d0`: an async
    /// cluster load with `onMenuTextureLoaded`; returns 0.
    pub fn scene_load_menu_texture(h: &mut impl Host, id: u32) -> u32 {
        let mgr = instance(h, ASYNC_LOAD_INSTANCE);
        let callback = h.rebase(SCENE_ON_MENU_TEXTURE_LOADED);
        h.call_sym(ASYNC_LOAD_CLUSTER, Conv::Cdecl, &[mgr, 0, id, callback, 0]);
        0
    }
    /// `PhyreScene_onCutbufferShaderLoaded(shader, task)` `0x00666610`
    /// (cdecl): the task's scene `+0xfaa4` = `shader`; returns `shader`.
    pub fn scene_on_cutbuffer_shader_loaded(h: &mut impl Host, shader: u32, task: u32) -> u32 {
        instance(h, ASYNC_LOAD_INSTANCE);
        let scene: u32 = h.read(task);
        h.write(scene + SCENE_CUTBUFFER_SHADER, shader);
        shader
    }
    /// `PhyreScene_deleteIggyMenu_0065b750(this)`: destroys and frees the
    /// Iggy menu at `+0x10dc8` when there is one (the field keeps it).
    pub fn scene_delete_iggy_menu(h: &mut impl Host, this: u32) {
        let menu: u32 = h.read(this + SCENE_IGGY_MENU);
        if menu != 0 {
            h.call_sym(IGGY_MENU_DTOR, Conv::Thiscall(menu), &[]);
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[menu]);
        }
    }

    // ---- async tasks ----

    /// `graphicAsyncTaskCallback(task)` `0x00637860`: `task[0](task[1])`.
    pub fn async_task_callback(h: &mut impl Host, task: u32) -> u32 {
        instance(h, ASYNC_LOAD_INSTANCE);
        let f: u32 = h.read(task);
        let arg: u32 = h.read(task + 4);
        h.call(f, Conv::Cdecl, &[arg])
    }
    /// `graphicRegistAsyncTask(arg, b)` `0x00642950` and
    /// `graphicTryAsyncLoadIfAvailable` `0x006442f0` (the same code):
    /// a task of kind 2 running `graphicAsyncTaskCallback(arg)`, with `b`
    /// sign-extended to 64 bits.
    pub fn regist_async_task(h: &mut impl Host, arg: u32, b: u32) -> u32 {
        let mgr = instance(h, ASYNC_LOAD_INSTANCE);
        let callback = h.rebase(ASYNC_TASK_CALLBACK);
        let high = if b & 0x8000_0000 != 0 { 0xffff_ffff } else { 0 };
        h.call_sym(
            ASYNC_REGIST_TASK,
            Conv::Cdecl,
            &[mgr, callback, 2, arg, 0, b, high],
        )
    }
    /// The three `PhyreScene_asyncDo*Task(task)` bodies: `task = (scene,
    /// entry, c)`; the load, then `operator delete(entry)`.
    fn async_load_task(h: &mut impl Host, task: u32, load: u32, magic: bool) -> u32 {
        instance(h, ASYNC_LOAD_INSTANCE);
        let scene: u32 = h.read(task);
        let entry: u32 = h.read(task + 4);
        let c: u32 = h.read(task + 8);
        if magic {
            h.call_sym(load, Conv::Thiscall(scene), &[c, entry, 1]);
        } else {
            h.call_sym(load, Conv::Thiscall(scene), &[entry, c]);
        }
        h.call_sym(OPERATOR_DELETE_TASK, Conv::Cdecl, &[entry])
    }
    /// `PhyreScene_asyncDoLoadCharacterTask(task)` `0x00657850`:
    /// `loadCharacter(entry, c)`.
    pub fn async_load_character_task(h: &mut impl Host, task: u32) -> u32 {
        async_load_task(h, task, SCENE_LOAD_CHARACTER, false)
    }
    /// `PhyreScene_asyncDoloadMagicFileVFXTextureTask(task)` `0x00657880`:
    /// `loadMagicFileVFXTexture(c, entry, 1)`.
    pub fn async_load_magic_file_vfx_texture_task(h: &mut impl Host, task: u32) -> u32 {
        async_load_task(h, task, SCENE_LOAD_MAGIC_FILE_VFX_TEXTURE, true)
    }
    /// `PhyreScene_asyncDoloadVFXTextureTask(task)` `0x006578b0`:
    /// `loadVFXTexture(c, entry, 1)`.
    pub fn async_load_vfx_texture_task(h: &mut impl Host, task: u32) -> u32 {
        async_load_task(h, task, SCENE_LOAD_VFX_TEXTURE, true)
    }

    // ---- menus, the sphere grid, the UI ----

    /// `graphicMenuLoadSpecialTexture(a, b, c)` `0x00642300`.
    pub fn menu_load_special_texture(h: &mut impl Host, a: u32, b: u32, c: u32) -> u32 {
        on_global(h, MENU_TEXTURE_MANAGER, MENU_LOAD_SPECIAL, &[a, b, c])
    }
    /// `graphicAbmapGetVertexInfo(a, b)` `0x00638fd0`.
    pub fn abmap_get_vertex_info(h: &mut impl Host, a: u32, b: u32) -> u32 {
        on_global(h, GRAPHIC_ABMAP, ABMAP_GET_VERTEX_INFO, &[a, b])
    }
    /// `graphicAbmapResetLayers_006390d0(v)`: when the sphere grid exists.
    pub fn abmap_reset_layers(h: &mut impl Host, v: u32) {
        let abmap = GRAPHIC_ABMAP.get(h);
        if abmap != 0 {
            h.call_sym(ABMAP_RESET_LAYERS, Conv::Thiscall(abmap), &[v]);
        }
    }
    /// `graphicAbmapCreate(a)` `0x00638f90`: waits for the render thread,
    /// then creates the sphere grid (`0x00684a60`, which reads the
    /// caller's argument: Ghidra shows none).
    pub fn abmap_create(h: &mut impl Host, a: u32) -> u32 {
        render_thread_sync(h);
        h.call_sym(ABMAP_CREATE, Conv::Cdecl, &[a])
    }
    /// `graphicAbmapDestory()` `0x00638fb0`: waits for the render thread,
    /// then `AbmapManager::ToDestroy`.
    pub fn abmap_destroy(h: &mut impl Host) {
        render_thread_sync(h);
        on_global(h, GRAPHIC_ABMAP, ABMAP_TO_DESTROY, &[]);
    }
    /// The application (asked for, unused) and the render thread's sync.
    fn render_thread_sync(h: &mut impl Host) {
        instance(h, GET_APPLICATION);
        let thread = instance(h, RENDER_THREAD_INSTANCE);
        h.call_sym(RENDER_THREAD_SYNC, Conv::Thiscall(thread), &[]);
    }
    /// `graphicMapPartLangOverride_006412b0(a, b)`.
    pub fn map_part_lang_override(h: &mut impl Host, a: u32, b: u32) -> u32 {
        on_global(h, GRAPHIC_UI, UI_LANG_OVERRIDE, &[a, b])
    }
    /// `graphicSetUiDrawType(v)` `0x00643af0`: when the UI exists.
    pub fn set_ui_draw_type(h: &mut impl Host, v: u32) {
        let ui = GRAPHIC_UI.get(h);
        if ui != 0 {
            h.call_sym(UI_SET_DRAW_TYPE, Conv::Thiscall(ui), &[v]);
        }
    }
    /// `graphicMenuReset()` `0x00642320` and `graphicMenuUpdate()`
    /// `0x006423b0`: the UI's `method` unless the sphere grid exists in a
    /// state below 2 (signed), when they return the grid.
    fn menu_call(h: &mut impl Host, method: u32) -> u32 {
        let abmap = GRAPHIC_ABMAP.get(h);
        if abmap != 0 && h.read::<i32>(abmap + 4) < 2 {
            return abmap;
        }
        on_global(h, GRAPHIC_UI, method, &[])
    }
    /// `graphicMenuReset()` `0x00642320`.
    pub fn menu_reset(h: &mut impl Host) -> u32 {
        menu_call(h, UI_MENU_RESET)
    }
    /// `graphicMenuUpdate()` `0x006423b0`.
    pub fn menu_update(h: &mut impl Host) -> u32 {
        menu_call(h, UI_MENU_UPDATE)
    }
    /// `graphicUpdateMapYIQ()` `0x00644990`: the input manager's update,
    /// the Iggy global's and the Iggy menu manager's, then `inputClear`.
    pub fn update_map_yiq(h: &mut impl Host) -> u32 {
        let input = instance(h, INPUT_INSTANCE);
        h.call_sym(INPUT_UPDATE, Conv::Thiscall(input), &[]);
        let iggy = instance(h, IGGY_GLOBAL);
        h.call_sym(IGGY_GLOBAL_UPDATE, Conv::Thiscall(iggy), &[]);
        let menus = IGGY_MENU_MANAGER.get(h);
        h.call_sym(IGGY_MENU_UPDATE, Conv::Thiscall(menus), &[]);
        h.call_sym(INPUT_CLEAR, Conv::Cdecl, &[])
    }

    // ---- other managers ----

    /// `graphicGetLocTextWithLength(a, b, c)` `0x006412f0`.
    pub fn get_loc_text_with_length(h: &mut impl Host, a: u32, b: u32, c: u32) -> u32 {
        on_instance(h, LOCALIZATION_INSTANCE, LOCALIZATION_GET_TEXT, &[a, b, c])
    }
    /// `graphicSetMaterialUV_006436e0(a, b, c)`.
    pub fn set_material_uv(h: &mut impl Host, a: u32, b: u32, c: u32) -> u32 {
        on_instance(h, MORPH_INSTANCE, MORPH_SET_MATERIAL_UV, &[a, b, c])
    }
    /// `graphicTextureAnimationSetPaused_00643a00(a, paused)`: `paused != 0`.
    pub fn texture_animation_set_paused(h: &mut impl Host, a: u32, paused: u32) -> u32 {
        on_instance(
            h,
            TEX_ANIM_INSTANCE,
            TEX_ANIM_SET_PAUSED,
            &[a, u32::from(paused != 0)],
        )
    }
    /// `graphicPlayVideo(on)` `0x006427c0`: `on != 0`.
    pub fn play_video(h: &mut impl Host, on: u32) -> u32 {
        on_instance(h, FMV_INSTANCE, FMV_PLAY_VIDEO, &[u32::from(on != 0)])
    }
    /// `graphicInitFMVPlayer(a, b)` `0x00641690`: `initialize(a, 0, b !=
    /// 0) != 0`.
    pub fn init_fmv_player(h: &mut impl Host, a: u32, b: u32) -> u32 {
        let r = on_instance(h, FMV_INSTANCE, FMV_INITIALIZE, &[a, 0, u32::from(b != 0)]);
        u32::from(r != 0)
    }
    /// `graphicVideoUpdate()` `0x00645df0`: while a video plays (byte
    /// `+0x6d0`), the player's input and texture; else returns the
    /// manager.
    pub fn video_update(h: &mut impl Host) -> u32 {
        let fmv = instance(h, FMV_INSTANCE);
        if h.read::<u8>(fmv + crate::glue::FMV_PLAYING) == 0 {
            return fmv;
        }
        let app = instance(h, GET_APPLICATION);
        on_instance(h, FMV_INSTANCE, FMV_HANDLE_INPUT, &[app]);
        on_instance(h, FMV_INSTANCE, FMV_UPDATE_TEXTURE, &[])
    }
    /// `graphicsRegisterResetScreenFadeDelegation(v)` `0x00645e60`: `v`
    /// (`0x801` for -1) with the screen-fade reset delegate.
    pub fn register_reset_screen_fade_delegation(h: &mut impl Host, v: u32) -> u32 {
        let v = if v == 0xffff_ffff { 0x801 } else { v };
        let delegate = h.rebase(RESET_SCREEN_FADE_DELEGATE);
        on_global(
            h,
            RENDER_DELEGATION_HELPER,
            REGISTER_DELEGATION,
            &[0, 0, v, delegate],
        )
    }
    /// `ShadowManager_Initialize(this)` `0x006ce100`: the shaders, a square
    /// buffer of `g_ShadowSize` rounded toward 0 to a multiple of 8, the
    /// set-up.
    pub fn shadow_manager_initialize(h: &mut impl Host, this: u32) -> u32 {
        h.call_sym(SHADOW_LOAD_SHADERS, Conv::Thiscall(this), &[]);
        let size = SHADOW_SIZE.get(h);
        let size = size.wrapping_add((size >> 31) & 7) & !7;
        h.call_sym(
            SHADOW_CREATE_BUFFER,
            Conv::Thiscall(this),
            &[size as u32, size as u32],
        );
        h.call_sym(SHADOW_SETUP, Conv::Thiscall(this), &[])
    }
    /// `ModelBuffer_ClearAll()` `0x0072ac30`: zeroes the model buffers
    /// (the CRT's `memset`, whose `eax` it returns) and sets the index to
    /// -1.
    pub fn model_buffer_clear_all(h: &mut impl Host) -> u32 {
        let at = h.rebase(MODEL_BUFFERS);
        let r = h.call_sym(MEMSET, Conv::Cdecl, &[at, 0, MODEL_BUFFERS_LEN]);
        MODEL_BUFFER_INDEX.set(h, 0xffff_ffff);
        r
    }

    // ---- the application ----

    /// `graphicSetForceNoVsync(on)` `0x00643200`: the application's byte
    /// `+0x1c` = `on != 0`, `+0x1d` = whether it changed. Returns the
    /// application.
    pub fn set_force_no_vsync(h: &mut impl Host, on: u32) -> u32 {
        let app = instance(h, GET_APPLICATION);
        let on = u8::from(on != 0);
        let was: u8 = h.read(app + APP_NO_VSYNC);
        h.write(app + APP_NO_VSYNC, on);
        h.write(app + APP_NO_VSYNC + 1, u8::from(on != was));
        app
    }
    /// `FFXApplication_getModeOr2_0042f750(this)`: 2 while byte `+0x338`
    /// or `+0x33c` is set, else the first word.
    pub fn application_mode_or_2(m: &impl Mem, this: u32) -> u32 {
        if m.read::<u8>(this + 0x338) != 0 || m.read::<u8>(this + 0x33c) != 0 {
            2
        } else {
            m.read(this)
        }
    }
}
