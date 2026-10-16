//! `ffx-system`: the PS2 CD layer, save-data flags, singletons,
//! configuration, text codes.

use ffx_system::{cd, config, instance, msg, save, stub};

use super::{mem, ported};

ported! {
    /// `set_ptr32(p)`: `p`.
    set_ptr32 @ 0x00a4_4700 => extern "C" fn set_ptr32(p: u32) -> u32 {
        stub::ptr32(p)
    }
    /// `get_ptr32_func(p)`: `p`.
    get_ptr32_func @ 0x00a4_4650 => extern "C" fn get_ptr32_func(p: u32) -> u32 {
        stub::ptr32(p)
    }
    /// `is_ptr32_null(p)`.
    is_ptr32_null @ 0x00a4_46a0 => extern "C" fn is_ptr32_null(p: u32) -> u32 {
        stub::is_ptr32_null(p)
    }
    /// `sceCdDiskReady(mode)`: 2.
    sceCdDiskReady @ 0x0062_f550 => extern "C" fn sce_cd_disk_ready() -> u32 {
        stub::CD_DISK_READY
    }
    /// `sceCdRead(...)`: 1.
    sceCdRead @ 0x00a4_47c0 => extern "C" fn sce_cd_read() -> u32 {
        stub::CD_READ
    }
    /// `sceSifSetDma(...)`: 1.
    sceSifSetDma @ 0x00a4_4790 => extern "C" fn sce_sif_set_dma() -> u32 {
        stub::SIF_SET_DMA
    }
    /// `scePadGetState(port, slot)`: 6.
    scePadGetState @ 0x0062_fb30 => extern "C" fn sce_pad_get_state() -> u32 {
        stub::PAD_GET_STATE
    }
    /// `sceMcSync(mode, &cmd, &result)`.
    sceMcSync @ 0x0062_f9f0 => extern "C" fn sce_mc_sync(_mode: u32, cmd: u32, result: u32) -> i32 {
        stub::bridge::mc_sync(&mut mem(), cmd, result)
    }
    /// `ModelBuffer_AllocateBuffer(...)`: 0xffff.
    ModelBuffer_AllocateBuffer @ 0x0072_ac20 => extern "C" fn model_buffer_allocate_buffer() -> u32 {
        stub::MODEL_BUFFER_ALLOCATE
    }

    /// `GetCD()`: `&g_CdLib`.
    GetCD @ 0x0076_d0d0 => extern "C" fn get_cd() -> u32 {
        cd::bridge::get_cd(&mem())
    }
    /// `cd_file_sector(n)`.
    cd_file_sector @ 0x0076_d510 => extern "C" fn cd_file_sector(n: i32) -> u32 {
        cd::bridge::cd_file_sector(&mem(), n)
    }
    /// `CDfileSize_CD_EX(n)`.
    CDfileSize_CD_EX @ 0x0076_c410 => extern "C" fn cd_file_size_cd_ex(n: i32) -> u32 {
        cd::bridge::cd_file_size_ex(&mem(), n)
    }
    /// `check_ex_file_size(archive, n)`.
    check_ex_file_size @ 0x0076_d660 => extern "C" fn check_ex_file_size(archive: u32, n: i32) -> u32 {
        cd::bridge::check_ex_file_size(&mem(), archive, n)
    }
    /// `disk_open_check_set(flag)` (a byte).
    disk_open_check_set @ 0x0076_d6b0 => extern "C" fn disk_open_check_set(flag: u32) {
        #[allow(clippy::cast_possible_truncation)] // the original stores AL
        cd::bridge::disk_open_check_set(&mut mem(), flag as u8);
    }
    /// `hdd_mount_flg_get()`.
    hdd_mount_flg_get @ 0x0077_9560 => extern "C" fn hdd_mount_flg_get() -> i32 {
        cd::bridge::hdd_mount_flg_get(&mem())
    }
    /// `hdd_rmode_get(n)`.
    hdd_rmode_get @ 0x0077_9570 => extern "C" fn hdd_rmode_get(n: i32) -> i32 {
        cd::bridge::hdd_rmode_get(&mem(), n)
    }
    /// `hdd_head_reset()`.
    hdd_head_reset @ 0x0077_89e0 => extern "C" fn hdd_head_reset() {
        cd::bridge::hdd_head_reset(&mut mem());
    }
    /// `clear_read_chgname(i)`.
    clear_read_chgname @ 0x00a4_2f30 => extern "C" fn clear_read_chgname(i: i32) {
        cd::bridge::clear_read_chgname(&mut mem(), i);
    }
    /// `setFullArenaLocalizationEnable()`.
    setFullArenaLocalizationEnable @ 0x00a4_31f0 => extern "C" fn set_full_arena_localization_enable() {
        cd::bridge::set_full_arena_localization_enable(&mut mem());
    }

    /// `SaveDataManager::instance()`.
    SaveDataManager_instance @ 0x006f_2070 => extern "C" fn save_data_manager_instance() -> u32 {
        save::bridge::manager_instance(&mem())
    }
    /// `SaveDataManager::getRefBuffer()`.
    SaveDataManager_getRefBuffer @ 0x006f_1ed0 => extern "thiscall" fn save_data_manager_get_ref_buffer(this: u32) -> u32 {
        save::bridge::get_ref_buffer(&mem(), this)
    }
    /// `SaveDataManager::setCbResult(result)`.
    SaveDataManager_setCbResult @ 0x006f_2500 => extern "thiscall" fn save_data_manager_set_cb_result(this: u32, result: u32) {
        save::bridge::set_cb_result(&mut mem(), this, result);
    }
    /// `SaveDataManager::isBusy()`.
    SaveDataManager_isBusy @ 0x006f_2090 => extern "thiscall" fn save_data_manager_is_busy(this: u32) -> u32 {
        save::bridge::is_busy(&mem(), this)
    }
    /// `SaveDataSetEventId(id)`.
    SaveDataSetEventId @ 0x0064_8630 => extern "C" fn save_data_set_event_id(id: u32) {
        save::bridge::set_event_id(&mut mem(), id);
    }
    /// `SaveDataGetScreenState()`.
    SaveDataGetScreenState @ 0x0064_7f70 => extern "C" fn save_data_get_screen_state() -> u32 {
        save::bridge::get_screen_state(&mem())
    }
    /// `SaveDataIsScreenClosed()`.
    SaveDataIsScreenClosed @ 0x0064_8100 => extern "C" fn save_data_is_screen_closed() -> u32 {
        save::bridge::is_screen_closed(&mem())
    }
    /// `SaveDataGetDialogState()`.
    SaveDataGetDialogState @ 0x0064_7ee0 => extern "C" fn save_data_get_dialog_state() -> u32 {
        save::bridge::get_dialog_state(&mem())
    }
    /// `SaveDataSetDialogState(state)`.
    SaveDataSetDialogState @ 0x0064_8620 => extern "C" fn save_data_set_dialog_state(state: u32) {
        save::bridge::set_dialog_state(&mut mem(), state);
    }
    /// `SaveDataIsSave()`.
    SaveDataIsSave @ 0x0064_80d0 => extern "C" fn save_data_is_save() -> u32 {
        save::bridge::is_save(&mem())
    }
    /// `SaveDataSetSave(save)`.
    SaveDataSetSave @ 0x0064_86b0 => extern "C" fn save_data_set_save(save: u32) {
        save::bridge::set_save(&mut mem(), save);
    }
    /// `SaveDataIsLoadingSuccessful()`.
    SaveDataIsLoadingSuccessful @ 0x0064_80a0 => extern "C" fn save_data_is_loading_successful() -> u32 {
        save::bridge::is_loading_successful(&mem())
    }

    /// `ClusterManager::GetInstance()`.
    ClusterManager_GetInstance @ 0x0069_a1a0 => extern "C" fn cluster_manager_get_instance() -> u32 {
        instance::bridge::cluster_manager(&mem())
    }
    /// `AsyncLoadManager::sGetInstance()`.
    AsyncLoadManager_sGetInstance @ 0x0069_1b40 => extern "C" fn async_load_manager_s_get_instance() -> u32 {
        instance::bridge::async_load_manager(&mem())
    }
    /// `AsyncLoadManager::isAllRequestDone()`.
    AsyncLoadManager_isAllRequestDone @ 0x0069_1630 => extern "thiscall" fn async_load_manager_is_all_request_done(this: u32) -> bool {
        instance::bridge::is_all_request_done(&mem(), this)
    }
    /// `BigFileStream::get()`.
    BigFileStream_get @ 0x0061_bdb0 => extern "C" fn big_file_stream_get() -> u32 {
        instance::bridge::big_file_stream(&mem())
    }
    /// `MagicFile_GetMagicId()`.
    MagicFile_GetMagicId @ 0x009d_a3b0 => extern "C" fn magic_file_get_magic_id() -> u32 {
        instance::bridge::magic_id(&mem())
    }
    /// `MagicFile_GetHandle()`.
    MagicFile_GetHandle @ 0x009d_a390 => extern "C" fn magic_file_get_handle() -> u32 {
        instance::bridge::magic_handle(&mem())
    }
    /// `MagicFile_IsNexData()`.
    MagicFile_IsNexData @ 0x009d_a420 => extern "C" fn magic_file_is_nex_data() -> u32 {
        instance::bridge::magic_is_nex(&mem())
    }
    /// `AssetListReader::getAssetCount()`.
    AssetListReader_getAssetCount @ 0x006f_4170 => extern "thiscall" fn asset_list_reader_get_asset_count(this: u32) -> i32 {
        instance::bridge::asset_count(&mem(), this)
    }

    /// `FFXIsDebug()`.
    FFXIsDebug @ 0x0088_7c50 => extern "C" fn ffx_is_debug() -> i32 {
        config::bridge::is_debug(&mem())
    }
    /// `FFXGetAsiaMode()`.
    FFXGetAsiaMode @ 0x0088_7c20 => extern "C" fn ffx_get_asia_mode() -> u32 {
        config::bridge::get_asia_mode(&mem())
    }
    /// `FFXSetAsiaMode(mode)` (a byte).
    FFXSetAsiaMode @ 0x0088_7c70 => extern "C" fn ffx_set_asia_mode(mode: u32) {
        #[allow(clippy::cast_possible_truncation)] // the original stores AL
        config::bridge::set_asia_mode(&mut mem(), mode as u8);
    }
    /// `TOGetFFXLangSwapFlag()`.
    TOGetFFXLangSwapFlag @ 0x008a_c390 => extern "C" fn to_get_ffx_lang_swap_flag() -> u32 {
        config::bridge::lang_swap_flag(&mem())
    }
    /// `TOGetFFXVideoMode()`.
    TOGetFFXVideoMode @ 0x008a_c3a0 => extern "C" fn to_get_ffx_video_mode() -> u32 {
        config::bridge::video_mode(&mem())
    }
    /// `TOGetSystemConfirmButtonAssign()`.
    TOGetSystemConfirmButtonAssign @ 0x008a_cf30 => extern "C" fn to_get_system_confirm_button_assign() -> i32 {
        config::bridge::confirm_button_assign(&mut mem())
    }
    /// `isEternalCalmScene(n)`.
    isEternalCalmScene @ 0x0067_ab50 => extern "C" fn is_eternal_calm_scene(n: i32) -> u32 {
        config::bridge::is_eternal_calm(&mem(), n)
    }
    /// `isEternalCalmMgrp(n)`.
    isEternalCalmMgrp @ 0x0067_ab30 => extern "C" fn is_eternal_calm_mgrp(n: i32) -> u32 {
        config::bridge::is_eternal_calm(&mem(), n)
    }
    /// `getEternalCalmSceneName(n)`.
    getEternalCalmSceneName @ 0x0067_aac0 => extern "C" fn get_eternal_calm_scene_name(n: i32) -> u32 {
        config::bridge::eternal_calm_scene_name(&mem(), n)
    }
    /// `setBattleEnable(on)`.
    setBattleEnable @ 0x0078_2f70 => extern "C" fn set_battle_enable(on: u32) {
        config::bridge::set_battle_enable(&mut mem(), on);
    }

    /// `TOSkipCode(p)`.
    TOSkipCode @ 0x008d_fd80 => extern "C" fn to_skip_code(p: u32) -> u32 {
        msg::bridge::skip_code(&mem(), p)
    }
    /// `TOGetGroup(code)`.
    TOGetGroup @ 0x008d_f610 => extern "C" fn to_get_group(code: u32) -> u32 {
        msg::group(code)
    }
}
