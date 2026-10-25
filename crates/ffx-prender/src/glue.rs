//! The game's render-side glue (M4 port round 1, list `h3-glue-render`):
//! the `graphic*` C entry points the game logic calls, and the small
//! `PhyreScene`, manager and dynamic-geometry methods behind them.
//!
//! Most of them are one-liners: load an object (the scene `g_PhyreScene`,
//! a manager singleton, a global) and call one of its methods, returning
//! the method's `eax` (a tail `jmp` or a `call` followed by `ret`). The
//! ports do the same through [`Host`], so a method that is ported later is
//! reached through its hook. A forwarder returns its callee's `eax` even
//! where the callee is `void`: the original leaves that register to its
//! callers, and returning it is exact.
//!
//! Singleton getters (`BrokenScreenEffect_GetInstance` and the like) stay
//! calls: some of them print a warning when the instance is missing.

use ffx_bridge::Global;

/// `g_PhyreScene`, the scene object.
pub const SCENE: Global<u32> = Global::new(0x00cc_b9d8);
/// `g_DynGeoFillBuffer`, the dynamic-geometry memory manager.
pub const DYN_GEO_FILL: Global<u32> = Global::new(0x00cd_edf0);
/// `g_ChrShaderRender`, the character shader renderer
/// (`ClassChrShaderRender`).
pub const CHR_SHADER_RENDER: Global<u32> = Global::new(0x00cc_ca74);
/// `g_MenuTextureManager`.
pub const MENU_TEXTURE_MANAGER: Global<u32> = Global::new(0x00cc_c820);

// ---- singleton getters (cdecl, no arguments, the instance in eax) ----

/// `Phyre_PFramework_PApplication_GetApplication`.
pub const GET_APPLICATION: u32 = 0x0062_3b90;
/// `AsyncLoadManager_sGetInstance`.
pub const ASYNC_LOAD_INSTANCE: u32 = 0x0069_1b40;
/// `BrokenScreenEffect_GetInstance` (prints when there is none).
pub const BROKEN_SCREEN_INSTANCE: u32 = 0x006d_4de0;
/// `PhyFMVPlayerManager_GetInstance` (prints when there is none).
pub const FMV_INSTANCE: u32 = 0x006d_73a0;
/// `VfxShaderManager_sGetInstance`.
pub const VFX_SHADER_INSTANCE: u32 = 0x006c_14c0;
/// `ShadowManager_getInstance`.
pub const SHADOW_INSTANCE: u32 = 0x006d_2390;

// ---- methods called (thiscall unless noted) ----

/// `PhyreScene_startLoadingScreen` (no `this` read).
pub const SCENE_START_LOADING_SCREEN: u32 = 0x0066_fb30;
/// `PhyreScene_stopLoadingScreen` (no `this` read).
pub const SCENE_STOP_LOADING_SCREEN: u32 = 0x0066_fb50;
/// The scene's list clear at `+0x10d84` (`0x00658fc0`).
pub const SCENE_CLEAR_LIST: u32 = 0x0065_8fc0;
/// `PhyreScene_drawMainMenuWaveEffect`.
pub const SCENE_DRAW_MAIN_MENU_WAVE: u32 = 0x0065_b5b0;
/// `PhyreScene_fieldMapShutdown`.
pub const SCENE_FIELD_MAP_SHUTDOWN: u32 = 0x0065_d0c0;
/// The scene's frame start (`0x00671be0`).
pub const SCENE_FRAME_START: u32 = 0x0067_1be0;
/// `PhyreScene_getCurrentLevelName`.
pub const SCENE_LEVEL_NAME: u32 = 0x0065_de70;
/// `PhyreScene_renderLoadingScreen`.
pub const SCENE_RENDER_LOADING_SCREEN: u32 = 0x0066_b1e0;
/// `PhyreScene_showAutoSavingPrompt`.
pub const SCENE_SHOW_AUTO_SAVE_PROMPT: u32 = 0x0066_f480;
/// `PhyreScene_UpdateTextureVideo`.
pub const SCENE_UPDATE_TEXTURE_VIDEO: u32 = 0x0065_4960;
/// `PhyreScene_fieldMapInitialize(this, first)` (`ret 4`).
pub const SCENE_FIELD_MAP_INITIALIZE: u32 = 0x0065_b8a0;
/// `PhyreScene_loadAtelVFXTexture_00663f00(this, id)` (`ret 4`).
pub const SCENE_LOAD_ATEL_VFX_TEXTURE: u32 = 0x0066_3f00;
/// `PhyreScene_unloadAtelVFXTexture_00670b00(this, id)` (`ret 4`).
pub const SCENE_UNLOAD_ATEL_VFX_TEXTURE: u32 = 0x0067_0b00;
/// `PhyreScene_loadVFXTextureAsync(this, kind, id)` (`ret 8`).
pub const SCENE_LOAD_VFX_TEXTURE_ASYNC: u32 = 0x0066_5f60;
/// `PhyreScene_unloadVFXTexture(this, id, which)` (`ret 8`).
pub const SCENE_UNLOAD_VFX_TEXTURE: u32 = 0x0067_1300;
/// The character pipeline add (`0x00656590`, `ret 4`).
pub const SCENE_ADD_TO_PIPELINE: u32 = 0x0065_6590;
/// `PhyreScene_getChaPos(this, out)` (`ret 4`).
pub const SCENE_GET_CHA_POS: u32 = 0x0065_d860;
/// `PhyreScene_setMagicPath_0066cfb0(this, path)` (`ret 4`).
pub const SCENE_SET_MAGIC_PATH: u32 = 0x0066_cfb0;
/// `PhyreScene_setToBeDeletePath_0066e830(this, path)` (`ret 4`).
pub const SCENE_SET_TO_BE_DELETE_PATH: u32 = 0x0066_e830;
/// `PhyreScene_loadMagicVFXTexture(this, id)` (`ret 4`).
pub const SCENE_LOAD_MAGIC_VFX_TEXTURE: u32 = 0x0066_4d80;
/// `PhyreScene_unloadMagicVFXTexture(this, id)` (`ret 4`).
pub const SCENE_UNLOAD_MAGIC_VFX_TEXTURE: u32 = 0x0067_0f00;
/// `PhyreScene_setMeshInstanceGroupIdVisible(this, id)` (`ret 4`).
pub const SCENE_SET_GROUP_VISIBLE: u32 = 0x0066_d610;
/// `PhyreScene_GetSplitScreenStat`.
pub const SCENE_SPLIT_SCREEN_STAT: u32 = 0x0065_17d0;
/// `Phyre_PString_Assign(this, text)` (`ret 4`).
pub const PSTRING_ASSIGN: u32 = 0x0044_5cf0;
/// The VFX mesh release (`0x00636970`) on the scene's `+0x10d54`.
pub const RELEASE_MESHES: u32 = 0x0063_6970;

/// `DynGeoMemManager_clearDynGeoMemory`.
pub const DYN_GEO_CLEAR: u32 = 0x006d_e700;
/// `DynGeoMemManager_getCurrentBufferId`.
pub const DYN_GEO_CURRENT_ID: u32 = 0x006d_e9f0;
/// `DynGeoMemManager_lockForRecordNumInKeLns`.
pub const DYN_GEO_LOCK: u32 = 0x006d_eaf0;
/// `DynGeoMemManager_unlockForRecordNumInKeLns`.
pub const DYN_GEO_UNLOCK: u32 = 0x006d_ed50;
/// The buffer swap `DynGeoMemManager_swapDynGeoMemory` jumps to
/// (`0x006dece0`, on the manager's first word).
pub const DYN_GEO_SWAP: u32 = 0x006d_ece0;
/// `KERNEL32!WaitForSingleObject`'s import slot.
pub const WAIT_FOR_SINGLE_OBJECT: u32 = 0x00b0_c0f8;
/// `KERNEL32!ReleaseMutex`'s import slot.
pub const RELEASE_MUTEX: u32 = 0x00b0_c1c0;

/// `ClassChrShaderRender_GetVertexInfo`.
pub const CHR_GET_VERTEX_INFO: u32 = 0x0069_f880;
/// `DynGeo_ResetCurrentBuffer_0069fcb0`.
pub const CHR_SHADOW_START: u32 = 0x0069_fcb0;
/// The shadow pass end (`0x0069fe40`).
pub const CHR_SHADOW_END: u32 = 0x0069_fe40;

/// `SetFieldConst5_006d4e10` (byte `+5` = 1).
pub const BROKEN_SET_5: u32 = 0x006d_4e10;
/// `SetFieldConst4_006d4e00` (byte `+4` = 1).
pub const BROKEN_SET_4: u32 = 0x006d_4e00;
/// `SetFieldConst6_006d5180` (byte `+6` = 1).
pub const BROKEN_SET_6: u32 = 0x006d_5180;
/// `BrokenScreenEffect_reset`.
pub const BROKEN_RESET: u32 = 0x006d_5970;
/// `graphicSetFieldConst6_006d59f0` (the keep-frame start).
pub const BROKEN_KEEP_FRAME: u32 = 0x006d_59f0;

/// `PhyFMVPlayerManager_getCurrentFrame`.
pub const FMV_CURRENT_FRAME: u32 = 0x006d_91b0;
/// `PhyFMVPlayerManager_pauseMovie`.
pub const FMV_PAUSE: u32 = 0x006d_a4b0;
/// `PhyFMVPlayerManager_DestroyFMVPlayer`.
pub const FMV_DESTROY: u32 = 0x006d_71b0;
/// `PhyFMVPlayerManager_StateCheck_006da2a0` (a byte in `al`).
pub const FMV_STATE_CHECK: u32 = 0x006d_a2a0;

/// `VfxShaderManager_unloadSpecialShaderTable_magic`.
pub const VFX_SHADER_UNLOAD_MAGIC: u32 = 0x006c_1c50;
/// `ShadowManager_clearCascades`.
pub const SHADOW_CLEAR_CASCADES: u32 = 0x006d_1780;
/// `MenuTextureManager_UnloadSpecialMenuTexture(this, id)` (`ret 4`).
pub const MENU_UNLOAD_SPECIAL: u32 = 0x0067_f830;
/// `Phyre_PFramework_PWindowWin32Base_SetFlipVSyncInterval(interval)`
/// (cdecl).
pub const SET_FLIP_VSYNC_INTERVAL: u32 = 0x0062_5090;

/// The application's loading-screen flag (a byte).
pub const APP_LOADING_SCREEN: u32 = 0x3a0;
/// The async load manager's file-read semaphore id.
pub const ASYNC_FILE_READ_SEMA: u32 = 0x8;
/// The async load manager's previous read-complete semaphore id.
pub const ASYNC_OLD_READ_COMPLETE_SEMA: u32 = 0xc;
/// The FMV manager's "playing" byte.
pub const FMV_PLAYING: u32 = 0x6d0;
/// The FMV manager's "playing video" byte the scene reads.
pub const FMV_PLAYING_VIDEO: u32 = 0x6fc;
/// `VFXDynamicGeometry`'s z correction (an f32).
pub const VFX_GEO_Z_CORRECTION: u32 = 0xdc;
/// The scene's magic path (a `PString`).
pub const SCENE_MAGIC_PATH: u32 = 0xfafc;
/// The scene's to-be-deleted magic path (a `PString`).
pub const SCENE_TO_BE_DELETE_PATH: u32 = 0xfb00;
/// The scene's VFX mesh list.
pub const SCENE_VFX_MESHES: u32 = 0x0001_0d54;

pub mod bridge {
    //! The originals over game memory.

    use super::{
        APP_LOADING_SCREEN, ASYNC_FILE_READ_SEMA, ASYNC_LOAD_INSTANCE,
        ASYNC_OLD_READ_COMPLETE_SEMA, BROKEN_KEEP_FRAME, BROKEN_RESET, BROKEN_SCREEN_INSTANCE,
        BROKEN_SET_4, BROKEN_SET_5, BROKEN_SET_6, CHR_GET_VERTEX_INFO, CHR_SHADER_RENDER,
        CHR_SHADOW_END, CHR_SHADOW_START, DYN_GEO_CLEAR, DYN_GEO_CURRENT_ID, DYN_GEO_FILL,
        DYN_GEO_LOCK, DYN_GEO_SWAP, DYN_GEO_UNLOCK, FMV_CURRENT_FRAME, FMV_DESTROY, FMV_INSTANCE,
        FMV_PAUSE, FMV_PLAYING, FMV_PLAYING_VIDEO, FMV_STATE_CHECK, GET_APPLICATION,
        MENU_TEXTURE_MANAGER, MENU_UNLOAD_SPECIAL, PSTRING_ASSIGN, RELEASE_MESHES, RELEASE_MUTEX,
        SCENE, SCENE_ADD_TO_PIPELINE, SCENE_CLEAR_LIST, SCENE_DRAW_MAIN_MENU_WAVE,
        SCENE_FIELD_MAP_INITIALIZE, SCENE_FIELD_MAP_SHUTDOWN, SCENE_FRAME_START, SCENE_GET_CHA_POS,
        SCENE_LEVEL_NAME, SCENE_LOAD_ATEL_VFX_TEXTURE, SCENE_LOAD_MAGIC_VFX_TEXTURE,
        SCENE_LOAD_VFX_TEXTURE_ASYNC, SCENE_MAGIC_PATH, SCENE_RENDER_LOADING_SCREEN,
        SCENE_SET_GROUP_VISIBLE, SCENE_SET_MAGIC_PATH, SCENE_SET_TO_BE_DELETE_PATH,
        SCENE_SHOW_AUTO_SAVE_PROMPT, SCENE_SPLIT_SCREEN_STAT, SCENE_START_LOADING_SCREEN,
        SCENE_STOP_LOADING_SCREEN, SCENE_TO_BE_DELETE_PATH, SCENE_UNLOAD_ATEL_VFX_TEXTURE,
        SCENE_UNLOAD_MAGIC_VFX_TEXTURE, SCENE_UNLOAD_VFX_TEXTURE, SCENE_UPDATE_TEXTURE_VIDEO,
        SCENE_VFX_MESHES, SET_FLIP_VSYNC_INTERVAL, SHADOW_CLEAR_CASCADES, SHADOW_INSTANCE,
        VFX_GEO_Z_CORRECTION, VFX_SHADER_INSTANCE, VFX_SHADER_UNLOAD_MAGIC, WAIT_FOR_SINGLE_OBJECT,
    };
    use ffx_bridge::{Global, Mem};
    use ffx_graphic::host::{Conv, Host};

    /// Calls `method` on the object in `global`; its `eax`.
    pub(crate) fn on_global(
        h: &mut impl Host,
        global: Global<u32>,
        method: u32,
        args: &[u32],
    ) -> u32 {
        let this = global.get(h);
        h.call_sym(method, Conv::Thiscall(this), args)
    }

    /// The instance `getter` returns.
    pub(crate) fn instance(h: &mut impl Host, getter: u32) -> u32 {
        h.call_sym(getter, Conv::Cdecl, &[])
    }

    /// Calls `method` on the instance `getter` returns; its `eax`.
    pub(crate) fn on_instance(h: &mut impl Host, getter: u32, method: u32, args: &[u32]) -> u32 {
        let this = instance(h, getter);
        h.call_sym(method, Conv::Thiscall(this), args)
    }

    /// Calls the `stdcall` import whose slot is at `slot`; its `eax`.
    fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let at = h.rebase(slot);
        let f: u32 = h.read(at);
        h.call(f, Conv::Stdcall, args)
    }

    // ---- g_PhyreScene one-liners (cdecl, `mov ecx, [g_PhyreScene]`) ----

    /// `graphicActivateLoadingScreen()` `0x00639130`.
    pub fn activate_loading_screen(h: &mut impl Host) -> u32 {
        on_global(h, SCENE, SCENE_START_LOADING_SCREEN, &[])
    }
    /// `graphicDeActivateLoadingScreen()` `0x0063de40`.
    pub fn deactivate_loading_screen(h: &mut impl Host) -> u32 {
        on_global(h, SCENE, SCENE_STOP_LOADING_SCREEN, &[])
    }
    /// `graphicSceneClearList_0063dac0()`.
    pub fn scene_clear_list(h: &mut impl Host) -> u32 {
        on_global(h, SCENE, SCENE_CLEAR_LIST, &[])
    }
    /// `graphicDrawMainMenuWaterEffect()` `0x0063e920`.
    pub fn draw_main_menu_water_effect(h: &mut impl Host) -> u32 {
        on_global(h, SCENE, SCENE_DRAW_MAIN_MENU_WAVE, &[])
    }
    /// `graphicFieldMapShutdown()` `0x006403b0`.
    pub fn field_map_shutdown(h: &mut impl Host) -> u32 {
        on_global(h, SCENE, SCENE_FIELD_MAP_SHUTDOWN, &[])
    }
    /// `graphicFrameStart_00640df0()`.
    pub fn frame_start(h: &mut impl Host) {
        on_global(h, SCENE, SCENE_FRAME_START, &[]);
    }
    /// `graphicLevelGetName()` `0x00641d90`.
    pub fn level_get_name(h: &mut impl Host) -> u32 {
        on_global(h, SCENE, SCENE_LEVEL_NAME, &[])
    }
    /// `graphicRenderLoadingScreen_00642c00()`.
    pub fn render_loading_screen(h: &mut impl Host) -> u32 {
        on_global(h, SCENE, SCENE_RENDER_LOADING_SCREEN, &[])
    }
    /// `graphicShowAutoSavePrompt()` `0x00643de0`.
    pub fn show_auto_save_prompt(h: &mut impl Host) {
        on_global(h, SCENE, SCENE_SHOW_AUTO_SAVE_PROMPT, &[]);
    }
    /// `graphicTextureVideoUpdate()` `0x006442c0`.
    pub fn texture_video_update(h: &mut impl Host) -> u32 {
        on_global(h, SCENE, SCENE_UPDATE_TEXTURE_VIDEO, &[])
    }
    /// `graphicFieldMapInitialize()` `0x00640200`: the scene's
    /// `fieldMapInitialize(1)`.
    pub fn field_map_initialize(h: &mut impl Host) -> u32 {
        on_global(h, SCENE, SCENE_FIELD_MAP_INITIALIZE, &[1])
    }
    /// `graphicIsSplitScreenEnabled()` `0x00641cc0`: 1 when the split-screen
    /// state is 1, else 0.
    pub fn is_split_screen_enabled(h: &mut impl Host) -> u32 {
        u32::from(on_global(h, SCENE, SCENE_SPLIT_SCREEN_STAT, &[]) == 1)
    }

    // ---- g_PhyreScene with one argument (cdecl(a)) ----

    /// `graphicAtelLoadTexture_00639310(id)`.
    pub fn atel_load_texture(h: &mut impl Host, id: u32) -> u32 {
        on_global(h, SCENE, SCENE_LOAD_ATEL_VFX_TEXTURE, &[id])
    }
    /// `graphicAtelUnloadTexture(id)` `0x00639330`.
    pub fn atel_unload_texture(h: &mut impl Host, id: u32) -> u32 {
        on_global(h, SCENE, SCENE_UNLOAD_ATEL_VFX_TEXTURE, &[id])
    }
    /// `graphicCharacterInstanceAddToPipeline(chr)` `0x0063a010`.
    pub fn character_instance_add_to_pipeline(h: &mut impl Host, chr: u32) {
        on_global(h, SCENE, SCENE_ADD_TO_PIPELINE, &[chr]);
    }
    /// `graphicSetTidusPos_00641080(out)`: the scene's `getChaPos(out)`.
    pub fn get_cha_pos(h: &mut impl Host, out: u32) -> u32 {
        on_global(h, SCENE, SCENE_GET_CHA_POS, &[out])
    }
    /// `graphicMagicSetPath(path)` `0x00642050`.
    pub fn magic_set_path(h: &mut impl Host, path: u32) -> u32 {
        on_global(h, SCENE, SCENE_SET_MAGIC_PATH, &[path])
    }
    /// `graphicMagicSetToBeDeletePath(path)` `0x00642070`.
    pub fn magic_set_to_be_delete_path(h: &mut impl Host, path: u32) -> u32 {
        on_global(h, SCENE, SCENE_SET_TO_BE_DELETE_PATH, &[path])
    }
    /// `graphicMagicVFXLoadTexture(id)` `0x006420b0`.
    pub fn magic_vfx_load_texture(h: &mut impl Host, id: u32) -> u32 {
        on_global(h, SCENE, SCENE_LOAD_MAGIC_VFX_TEXTURE, &[id])
    }
    /// `graphicMagicVFXUnloadTexture(id)` `0x006420f0`.
    pub fn magic_vfx_unload_texture(h: &mut impl Host, id: u32) -> u32 {
        on_global(h, SCENE, SCENE_UNLOAD_MAGIC_VFX_TEXTURE, &[id])
    }
    /// `graphicMeshInstanceGroupIdSetVisible(id)` `0x006423d0`.
    pub fn mesh_instance_group_id_set_visible(h: &mut impl Host, id: u32) -> u32 {
        on_global(h, SCENE, SCENE_SET_GROUP_VISIBLE, &[id])
    }
    /// `graphicMenuUnloadSpecialTexture(id)` `0x00642390`.
    pub fn menu_unload_special_texture(h: &mut impl Host, id: u32) -> u32 {
        on_global(h, MENU_TEXTURE_MANAGER, MENU_UNLOAD_SPECIAL, &[id])
    }

    // ---- dynamic geometry and the character shader renderer ----

    /// `graphicDynGeoMemManagerClearBuffer()` `0x0063fd80`.
    pub fn dyn_geo_clear_buffer(h: &mut impl Host) {
        on_global(h, DYN_GEO_FILL, DYN_GEO_CLEAR, &[]);
    }
    /// `graphicDynGeoMemManagerGetCurrentBufferId()` `0x0063ff50`.
    pub fn dyn_geo_current_buffer_id(h: &mut impl Host) -> u32 {
        on_global(h, DYN_GEO_FILL, DYN_GEO_CURRENT_ID, &[])
    }
    /// `graphicDynGeoMemManagerLockForRecordNumInKeLns()` `0x0063ff60`.
    pub fn dyn_geo_lock(h: &mut impl Host) -> u32 {
        on_global(h, DYN_GEO_FILL, DYN_GEO_LOCK, &[])
    }
    /// `graphicDynGeoMemManagerUnlockForRecordNumInKeLns()` `0x00640050`.
    pub fn dyn_geo_unlock(h: &mut impl Host) -> u32 {
        on_global(h, DYN_GEO_FILL, DYN_GEO_UNLOCK, &[])
    }
    /// `DynGeoMemManager_lockForRecordNumInKeLns(this)` `0x006deaf0`:
    /// `WaitForSingleObject(this +0xc, INFINITE)`.
    pub fn lock_for_record_num(h: &mut impl Host, this: u32) -> u32 {
        let mutex: u32 = h.read(this + 0xc);
        import(h, WAIT_FOR_SINGLE_OBJECT, &[mutex, 0xffff_ffff])
    }
    /// `DynGeoMemManager_unlockForRecordNumInKeLns(this)` `0x006ded50`:
    /// `ReleaseMutex(this +0xc)`.
    pub fn unlock_for_record_num(h: &mut impl Host, this: u32) -> u32 {
        let mutex: u32 = h.read(this + 0xc);
        import(h, RELEASE_MUTEX, &[mutex])
    }
    /// `DynGeoMemManager_swapDynGeoMemory(this)` `0x006ded40`: the swap
    /// (`0x006dece0`) on the manager's first word when it is set. The
    /// original returns the swap's `eax`, or leaves its caller's when the
    /// word is 0; its only caller (`graphicDynGeoMemManagerSwapBuffer`)
    /// passes it on, and nothing reads it (check-clobber).
    pub fn swap_dyn_geo_memory(h: &mut impl Host, this: u32) {
        let inner: u32 = h.read(this);
        if inner != 0 {
            h.call_sym(DYN_GEO_SWAP, Conv::Thiscall(inner), &[]);
        }
    }
    /// `graphicShaderGetVertexInfo()` `0x00643db0`.
    pub fn shader_get_vertex_info(h: &mut impl Host) -> u32 {
        on_global(h, CHR_SHADER_RENDER, CHR_GET_VERTEX_INFO, &[])
    }
    /// `graphicShadowStart_00643dc0()`.
    pub fn shadow_start(h: &mut impl Host) -> u32 {
        on_global(h, CHR_SHADER_RENDER, CHR_SHADOW_START, &[])
    }
    /// `graphicShadowEnd_00643dd0()`.
    pub fn shadow_end(h: &mut impl Host) {
        on_global(h, CHR_SHADER_RENDER, CHR_SHADOW_END, &[]);
    }

    // ---- manager singletons ----

    /// Calls the broken-screen effect's `setter` (a byte store that leaves
    /// `eax` alone); returns the instance, which the original leaves in
    /// `eax` (`opus_encount4_init` calls `graphicBrokenScreenSetFlag4`).
    fn set_and_keep(h: &mut impl Host, setter: u32) -> u32 {
        let this = instance(h, BROKEN_SCREEN_INSTANCE);
        h.call_sym(setter, Conv::Thiscall(this), &[]);
        this
    }

    /// `graphicBrokenScreenSetFlag5_00639360()`.
    pub fn broken_screen_set_flag5(h: &mut impl Host) -> u32 {
        set_and_keep(h, BROKEN_SET_5)
    }
    /// `graphicBrokenScreenSetFlag4_00639370()`.
    pub fn broken_screen_set_flag4(h: &mut impl Host) -> u32 {
        set_and_keep(h, BROKEN_SET_4)
    }
    /// `graphicBrokenScreenSetFlag6_00639390()`.
    pub fn broken_screen_set_flag6(h: &mut impl Host) -> u32 {
        set_and_keep(h, BROKEN_SET_6)
    }
    /// `graphicBrokenScreenReset()` `0x00639490`.
    pub fn broken_screen_reset(h: &mut impl Host) {
        on_instance(h, BROKEN_SCREEN_INSTANCE, BROKEN_RESET, &[]);
    }
    /// `graphicBrokenScreenStartKeepFrame()` `0x006394a0`.
    pub fn broken_screen_start_keep_frame(h: &mut impl Host) -> u32 {
        set_and_keep(h, BROKEN_KEEP_FRAME)
    }
    /// `graphicGetCurrentFrame()` `0x006410d0`.
    pub fn get_current_frame(h: &mut impl Host) -> u32 {
        on_instance(h, FMV_INSTANCE, FMV_CURRENT_FRAME, &[])
    }
    /// `graphicVideoPause()` `0x00645de0`.
    pub fn video_pause(h: &mut impl Host) {
        on_instance(h, FMV_INSTANCE, FMV_PAUSE, &[]);
    }
    /// `graphicDestroyFmv()` `0x0063df30`: destroys the FMV player when
    /// the manager exists (asking for it twice, as the original does);
    /// else returns the 0 the getter gave.
    pub fn destroy_fmv(h: &mut impl Host) -> u32 {
        let first = instance(h, FMV_INSTANCE);
        if first == 0 {
            return 0;
        }
        on_instance(h, FMV_INSTANCE, FMV_DESTROY, &[])
    }
    /// `graphicIsVideoBusy()` `0x00641ce0`: the state check's `al`,
    /// zero-extended.
    pub fn is_video_busy(h: &mut impl Host) -> u32 {
        on_instance(h, FMV_INSTANCE, FMV_STATE_CHECK, &[]) & 0xff
    }
    /// `graphicIsVideoPlaying()` `0x00641cf0`: the manager's byte `+0x6d0`.
    pub fn is_video_playing(h: &mut impl Host) -> u32 {
        let fmv = instance(h, FMV_INSTANCE);
        u32::from(h.read::<u8>(fmv + FMV_PLAYING))
    }
    /// `PhyreScene_isPlayingVideo()` `0x00645db0`: the manager's byte
    /// `+0x6fc`.
    pub fn scene_is_playing_video(h: &mut impl Host) -> u32 {
        let fmv = instance(h, FMV_INSTANCE);
        u32::from(h.read::<u8>(fmv + FMV_PLAYING_VIDEO))
    }
    /// `graphicMagicVFXUnloadLoadSpecialShaderTable()` `0x006420e0`.
    pub fn magic_vfx_unload_special_shader_table(h: &mut impl Host) -> u32 {
        on_instance(h, VFX_SHADER_INSTANCE, VFX_SHADER_UNLOAD_MAGIC, &[])
    }
    /// `graphicUpdateShadowVisibility()` `0x00644ab0`: the shadow
    /// manager's `clearCascades`.
    pub fn update_shadow_visibility(h: &mut impl Host) -> u32 {
        on_instance(h, SHADOW_INSTANCE, SHADOW_CLEAR_CASCADES, &[])
    }
    /// `graphicGetFileReadSemaId()` `0x00641150`.
    pub fn get_file_read_sema_id(h: &mut impl Host) -> u32 {
        let mgr = instance(h, ASYNC_LOAD_INSTANCE);
        h.read(mgr + ASYNC_FILE_READ_SEMA)
    }
    /// `graphicGetOldFileReadCompleteSemaId()` `0x00641360`.
    pub fn get_old_file_read_complete_sema_id(h: &mut impl Host) -> u32 {
        let mgr = instance(h, ASYNC_LOAD_INSTANCE);
        h.read(mgr + ASYNC_OLD_READ_COMPLETE_SEMA)
    }

    // ---- the application ----

    /// `PhyreScene_startLoadingScreen()` `0x0066fb30` (a scene method that
    /// reads no `this`): the application's loading-screen byte = 1.
    /// Returns the application.
    pub fn start_loading_screen(h: &mut impl Host) -> u32 {
        let app = instance(h, GET_APPLICATION);
        h.write(app + APP_LOADING_SCREEN, 1u8);
        app
    }
    /// `PhyreScene_stopLoadingScreen()` `0x0066fb50`: the byte = 0.
    pub fn stop_loading_screen(h: &mut impl Host) -> u32 {
        let app = instance(h, GET_APPLICATION);
        h.write(app + APP_LOADING_SCREEN, 0u8);
        app
    }
    /// `graphicIsLoadingScreenActive_00641c40()`: the byte, as 0 or 1.
    pub fn is_loading_screen_active(h: &mut impl Host) -> u32 {
        let app = instance(h, GET_APPLICATION);
        u32::from(h.read::<u8>(app + APP_LOADING_SCREEN) != 0)
    }
    /// `graphicSetFlipVsnc(interval)` `0x006430e0`: asks for the
    /// application (unused) and tail-calls
    /// `PWindowWin32Base::SetFlipVSyncInterval(interval)` (cdecl).
    pub fn set_flip_vsync(h: &mut impl Host, interval: u32) -> u32 {
        instance(h, GET_APPLICATION);
        h.call_sym(SET_FLIP_VSYNC_INTERVAL, Conv::Cdecl, &[interval])
    }

    // ---- scene and geometry methods (thiscall) ----

    /// `PhyreScene_setMagicPath_0066cfb0(this, path)`: `PString` assign
    /// into `this +0xfafc`.
    pub fn scene_set_magic_path(h: &mut impl Host, this: u32, path: u32) -> u32 {
        h.call_sym(
            PSTRING_ASSIGN,
            Conv::Thiscall(this + SCENE_MAGIC_PATH),
            &[path],
        )
    }
    /// `PhyreScene_setToBeDeletePath_0066e830(this, path)`: into
    /// `this +0xfb00`.
    pub fn scene_set_to_be_delete_path(h: &mut impl Host, this: u32, path: u32) -> u32 {
        h.call_sym(
            PSTRING_ASSIGN,
            Conv::Thiscall(this + SCENE_TO_BE_DELETE_PATH),
            &[path],
        )
    }
    /// `PhyreScene_loadAtelVFXTexture_00663f00(this, id)`:
    /// `loadVFXTextureAsync(4, id)`.
    pub fn scene_load_atel_vfx_texture(h: &mut impl Host, this: u32, id: u32) -> u32 {
        h.call_sym(SCENE_LOAD_VFX_TEXTURE_ASYNC, Conv::Thiscall(this), &[4, id])
    }
    /// `PhyreScene_unloadAtelVFXTexture_00670b00(this, id)`:
    /// `unloadVFXTexture(id, -1)`.
    pub fn scene_unload_atel_vfx_texture(h: &mut impl Host, this: u32, id: u32) -> u32 {
        h.call_sym(
            SCENE_UNLOAD_VFX_TEXTURE,
            Conv::Thiscall(this),
            &[id, 0xffff_ffff],
        )
    }
    /// `PhyreScene_releaseVFXMeshes(this)` `0x00659090`: the release on
    /// `this +0x10d54`.
    pub fn scene_release_vfx_meshes(h: &mut impl Host, this: u32) -> u32 {
        h.call_sym(RELEASE_MESHES, Conv::Thiscall(this + SCENE_VFX_MESHES), &[])
    }
    /// `VFXDynamicGeometry_setZCorrectionValue(this, value)` `0x006a3b20`:
    /// `this +0xdc` = `value` through `fld`/`fstp` (a signalling NaN
    /// becomes quiet). `eax` is the caller's.
    pub fn set_z_correction(m: &mut impl Mem, this: u32, value: f32) {
        m.write(
            this + VFX_GEO_Z_CORRECTION,
            ffx_bridge::x87::round_trip(value),
        );
    }
    /// `ClassDynamicGeometry_getReadyA8_006a2010(this)`: the record at
    /// `this +0xa8`'s first word while its byte `+4` is 0, else 0.
    pub fn get_ready_a8(m: &impl Mem, this: u32) -> u32 {
        let record: u32 = m.read(this + 0xa8);
        if m.read::<u8>(record + 4) == 0 {
            m.read(record)
        } else {
            0
        }
    }
}
