//! `ffx-system`: the PS2 CD layer, save data, singletons, configuration,
//! text codes, the `TO` window, pad, battle-window and scene accessors,
//! the archive digest map, the `cdc` matrix math.

use ffx_bridge::Mem;
use ffx_menu::scene::bridge as menu_scene;
use ffx_system::{
    accessor, btl, cd, config, daps, files, fixup, heap, instance, math, menu, msg, names, pad,
    pkt, render, save, scene, stl, stub, vbf, window,
};

use super::{call0, mem, ported};

/// `Phyre_PhyrePrintf(level, format)` (`0x004352f0`, cdecl, varargs), the
/// engine's printf, for bridges that print.
fn phyre_printf(level: u32, format: u32) {
    // SAFETY: the engine's cdecl printf, a format without arguments.
    unsafe {
        let f: unsafe extern "C" fn(u32, u32) = std::mem::transmute(crate::rebase(0x0043_52f0));
        f(level, format);
    }
}

/// Calls the cdecl function at `f` (an absolute address, as the game's
/// function tables hold them) with up to four arguments.
fn call_ptr<M>(_mem: &mut M, f: u32, args: &[u32]) -> u32 {
    super::call_cdecl(f, args)
}

/// Calls the original cdecl function at `sym` (an `ffx.sym` address).
fn call_sym<M>(mem: &mut M, sym: u32, args: &[u32]) -> u32 {
    #[allow(clippy::cast_possible_truncation)] // 32-bit process
    call_ptr(mem, crate::rebase(sym) as u32, args)
}

/// `Phyre::PSerialization::PStreamFile::read(buf, n)` (`0x00608090`,
/// thiscall), not ported yet.
fn stream_read<M>(_mem: &mut M, stream: u32, buf: u32, n: u32) -> u32 {
    // SAFETY: the game's thiscall stream read.
    unsafe {
        let f: unsafe extern "thiscall" fn(u32, u32, u32) -> u32 =
            std::mem::transmute(crate::rebase(0x0060_8090));
        f(stream, buf, n)
    }
}

/// `GetFileSizeEx(handle, &size)` through its import slot.
fn get_file_size_ex<M>(_mem: &mut M, handle: u32) -> Option<u64> {
    let mut size = 0u64;
    // SAFETY: GetFileSizeEx, stdcall, on the stream's handle.
    let ok = unsafe {
        let f: unsafe extern "stdcall" fn(u32, *mut u64) -> u32 =
            std::mem::transmute(GameOs::import(0x00b0_c13c));
        f(handle, &raw mut size)
    };
    (ok != 0).then_some(size)
}

/// The heap's Win32 calls, through the game's own import slots (so they are
/// the calls the originals make, to the same critical sections).
struct GameOs;

impl GameOs {
    /// The function in import slot `slot`.
    fn import(slot: u32) -> usize {
        // SAFETY: an import slot of this build, filled by the loader.
        unsafe { (crate::rebase(slot) as *const usize).read() }
    }
}

impl heap::Os for GameOs {
    fn enter_critical_section(&mut self, cs: u32) {
        // SAFETY: EnterCriticalSection(cs), stdcall; `cs` is the game's.
        unsafe {
            let f: unsafe extern "stdcall" fn(u32) = std::mem::transmute(Self::import(0x00b0_c154));
            f(cs);
        }
    }
    fn leave_critical_section(&mut self, cs: u32) {
        // SAFETY: LeaveCriticalSection(cs), stdcall.
        unsafe {
            let f: unsafe extern "stdcall" fn(u32) = std::mem::transmute(Self::import(0x00b0_c158));
            f(cs);
        }
    }
    fn virtual_alloc(&mut self, addr: u32, size: u32, alloc_type: u32, protect: u32) -> u32 {
        // SAFETY: VirtualAlloc, stdcall.
        unsafe {
            let f: unsafe extern "stdcall" fn(u32, u32, u32, u32) -> u32 =
                std::mem::transmute(Self::import(0x00b0_c09c));
            f(addr, size, alloc_type, protect)
        }
    }
    fn virtual_free(&mut self, addr: u32, size: u32, free_type: u32) -> u32 {
        // SAFETY: VirtualFree, stdcall.
        unsafe {
            let f: unsafe extern "stdcall" fn(u32, u32, u32) -> u32 =
                std::mem::transmute(Self::import(0x00b0_c098));
            f(addr, size, free_type)
        }
    }
    fn report_free_failure(&mut self) {
        // As the originals: FormatMessageA(ALLOCATE_BUFFER | FROM_SYSTEM,
        // 0, GetLastError(), 0, &message, 0x1000, &args), then the error
        // hook at 0x0193cd4c with the message, when one is set.
        // SAFETY: GetLastError and FormatMessageA (stdcall) through their
        // import slots; the hook is the game's cdecl callback.
        unsafe {
            let last: unsafe extern "stdcall" fn() -> u32 =
                std::mem::transmute(Self::import(0x00b0_c030));
            let format: unsafe extern "stdcall" fn(
                u32,
                u32,
                u32,
                u32,
                *mut u32,
                u32,
                *mut u32,
            ) -> u32 = std::mem::transmute(Self::import(0x00b0_c070));
            let mut message = 0u32;
            let mut args = 0u32;
            format(
                0x1100,
                0,
                last(),
                0,
                &raw mut message,
                0x1000,
                &raw mut args,
            );
            let hook = (crate::rebase(0x0193_cd4c) as *const usize).read();
            if hook != 0 {
                let f: unsafe extern "C" fn(u32) = std::mem::transmute(hook);
                f(message);
            }
        }
    }
    fn initialize_critical_section(&mut self, cs: u32) {
        // SAFETY: InitializeCriticalSection(cs), stdcall.
        unsafe {
            let f: unsafe extern "stdcall" fn(u32) = std::mem::transmute(Self::import(0x00b0_c150));
            f(cs);
        }
    }
    fn crt_malloc(&mut self, size: u32) -> u32 {
        // SAFETY: the C runtime's malloc, cdecl.
        unsafe {
            let f: unsafe extern "C" fn(u32) -> u32 =
                std::mem::transmute(Self::import(0x00b0_c468));
            f(size)
        }
    }
    fn crt_free(&mut self, p: u32) {
        // SAFETY: the C runtime's free, cdecl.
        unsafe {
            let f: unsafe extern "C" fn(u32) = std::mem::transmute(Self::import(0x00b0_c370));
            f(p);
        }
    }
    fn mem_system_init(&mut self) {
        // the original slow path of 0x009428a0 (cdecl, no arguments)
        call0(0x0094_28b0);
    }
}

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

    /// `cdc::Mul4x4(dst, m, v)`: a 4-vector times a matrix.
    cdc_Mul4x4 @ 0x0070_58f0 => extern "C" fn cdc_mul4x4(dst: u32, m: u32, v: u32) -> u32 {
        math::bridge::mul4x4_vector(&mut mem(), dst, m, v);
        dst // left in eax; FFXVu0ApplyMatrix returns it
    }
    /// `cdc::Mul4x4(dst, a, b)`: `b × a`.
    cdc_Mul4x4_007059b0 @ 0x0070_59b0 => extern "C" fn cdc_mul4x4_matrix(dst: u32, a: u32, b: u32) {
        math::bridge::mul4x4(&mut mem(), dst, a, b);
    }
    /// `cdc::Mul3x4(dst, m, v)`: a point transformed.
    cdc_Mul3x4 @ 0x0070_5850 => extern "C" fn cdc_mul3x4(dst: u32, m: u32, v: u32) -> u32 {
        math::bridge::mul3x4(&mut mem(), dst, m, v);
        dst // left in eax; FFXVu0RotTransPers and the envelope code use it
    }
    /// `cdc::Transpose(dst, m)`.
    cdc_Transpose @ 0x0070_63f0 => extern "C" fn cdc_transpose(dst: u32, m: u32) {
        math::bridge::transpose(&mut mem(), dst, m);
    }
    /// `cdc::OrthonormalInverse3x4(dst, m)`.
    cdc_OrthonormalInverse3x4 @ 0x0070_5db0 => extern "C" fn cdc_orthonormal_inverse3x4(dst: u32, m: u32) {
        math::bridge::orthonormal_inverse3x4(&mut mem(), dst, m);
    }
    /// `cdc::Matrix::RotX(sin, cos)`.
    cdc_Matrix_RotX @ 0x0070_5ed0 => extern "thiscall" fn cdc_matrix_rot_x(this: u32, sin: f32, cos: f32) -> u32 {
        math::bridge::rotate(&mut mem(), this, 1, sin, cos)
    }
    /// `cdc::Matrix::RotY(sin, cos)`.
    cdc_Matrix_RotY @ 0x0070_5fa0 => extern "thiscall" fn cdc_matrix_rot_y(this: u32, sin: f32, cos: f32) -> u32 {
        math::bridge::rotate_y(&mut mem(), this, sin, cos)
    }
    /// `cdc::Matrix::RotZ(sin, cos)`.
    cdc_Matrix_RotZ @ 0x0070_6070 => extern "thiscall" fn cdc_matrix_rot_z(this: u32, sin: f32, cos: f32) -> u32 {
        math::bridge::rotate(&mut mem(), this, 0, sin, cos)
    }
    /// `cdc::Vector::CanNormalize3()` (`this` in ECX): `al` 1 when the
    /// square sum is a finite number above `FLT_MIN`; the rest of `eax` as
    /// the original leaves it.
    cdc_Vector_CanNormalize3 @ 0x0070_3bc0 => extern "thiscall" fn cdc_vector_can_normalize3(this: u32) -> u32 {
        math::bridge::can_normalize3(&mem(), this)
    }
    /// `cdc::Vector::SafeNormalize3()` (`this` in ECX, fastcall).
    cdc_Vector_SafeNormalize3 @ 0x0070_4200 => extern "fastcall" fn cdc_vector_safe_normalize3(v: u32) {
        math::bridge::safe_normalize3(&mut mem(), v);
    }
    /// `cdc::Inverse4x4(dst, m)`: the general inverse (cofactors).
    cdc_Inverse4x4 @ 0x0070_4dd0 => extern "C" fn cdc_inverse4x4(dst: u32, m: u32) -> u32 {
        math::bridge::inverse4x4(&mut mem(), dst, m)
    }
    /// `sceVu0InversMatrix4x4(dst, m)`.
    sceVu0InversMatrix4x4 @ 0x006e_d460 => extern "C" fn sce_vu0_invers_matrix4x4(dst: u32, m: u32) -> u32 {
        math::bridge::invers_matrix4x4(&mut mem(), dst, m)
    }
    /// `sceVu0VectorLengthSquared(v)`.
    sceVu0VectorLengthSquared @ 0x006e_d9b0 => extern "C" fn sce_vu0_vector_length_squared(v: u32) -> f32 {
        math::bridge::length_squared(&mem(), v)
    }

    /// `TOGetWindowStructAdrs(i)`.
    TOGetWindowStructAdrs @ 0x0090_6fb0 => extern "C" fn to_get_window_struct_adrs(i: i32) -> u32 {
        window::bridge::window(&mem(), i)
    }
    /// `TOGetWindowType(i)`.
    TOGetWindowType @ 0x0090_6c40 => extern "C" fn to_get_window_type(i: i32) -> i32 {
        window::bridge::window_type(&mem(), i)
    }
    /// `TOGetWindowStatus(i)`.
    TOGetWindowStatus @ 0x0090_6c20 => extern "C" fn to_get_window_status(i: i32) -> i32 {
        window::bridge::window_status(&mem(), i)
    }
    /// `TOCloseWindow(i)`.
    TOCloseWindow @ 0x0090_6ba0 => extern "C" fn to_close_window(i: i32) {
        window::bridge::close_window(&mut mem(), i);
    }
    /// `TOGetMessageStatus(i)`.
    TOGetMessageStatus @ 0x008b_7ca0 => extern "C" fn to_get_message_status(i: i32) -> i32 {
        window::bridge::message_status(&mem(), i)
    }
    /// `TOCloseMessage(i)`.
    TOCloseMessage @ 0x008b_5810 => extern "C" fn to_close_message(i: i32) {
        window::bridge::close_message(&mut mem(), i);
    }
    /// `TOSetMessageFunc(i, n)`.
    TOSetMessageFunc @ 0x008b_8af0 => extern "C" fn to_set_message_func(i: i32, n: i32) {
        window::bridge::set_message_func(&mut mem(), i, n);
    }
    /// `TOBtlSetMacroCommandType(i, n, type)` (a byte).
    TOBtlSetMacroCommandType @ 0x008b_57c0 => extern "C" fn to_btl_set_macro_command_type(i: i32, n: i32, ty: u32) {
        #[allow(clippy::cast_possible_truncation)] // the original stores CL
        window::bridge::set_macro_command_type(&mut mem(), i, n, ty as u8);
    }
    /// `TOBtlSetMacroCommandValue(i, n, value)`.
    TOBtlSetMacroCommandValue @ 0x008b_57f0 => extern "C" fn to_btl_set_macro_command_value(i: i32, n: i32, value: u32) {
        window::bridge::set_macro_command_value(&mut mem(), i, n, value);
    }
    /// `TOInitMacroCmd(i)`.
    TOInitMacroCmd @ 0x008b_81d0 => extern "C" fn to_init_macro_cmd(i: i32) {
        window::bridge::init_macro_cmd(&mut mem(), i);
    }
    /// `MacroDic_Section(n, table)`.
    MacroDic_Section @ 0x008b_70a0 => extern "C" fn macro_dic_section(n: i32, table: i32) -> u32 {
        window::bridge::macro_dic_section(&mem(), n, table)
    }
    /// `TOGetSaveDicAdrs(table)`.
    TOGetSaveDicAdrs @ 0x008b_7f60 => extern "C" fn to_get_save_dic_adrs(table: i32) -> u32 {
        window::bridge::save_dic(&mem(), table)
    }

    /// `TOMenuGetControlPad()`.
    TOMenuGetControlPad @ 0x008b_e410 => extern "C" fn to_menu_get_control_pad() -> u32 {
        let flag = ffx_menu::tk::bridge::tk_menu_get_dummy_controler_flag(&mem());
        u32::from(pad::bridge::control_pad(&mem(), 0, flag))
    }
    /// `TOMenuGetControlPadTrg()`.
    TOMenuGetControlPadTrg @ 0x008b_e4b0 => extern "C" fn to_menu_get_control_pad_trg() -> u32 {
        let flag = ffx_menu::tk::bridge::tk_menu_get_dummy_controler_flag(&mem());
        u32::from(pad::bridge::control_pad(&mem(), 1, flag))
    }
    /// `TOMenuGetControlPadRep()`.
    TOMenuGetControlPadRep @ 0x008b_e470 => extern "C" fn to_menu_get_control_pad_rep() -> u32 {
        let flag = ffx_menu::tk::bridge::tk_menu_get_dummy_controler_flag(&mem());
        u32::from(pad::bridge::control_pad(&mem(), 2, flag))
    }
    /// `TOMenuGetControlPad2()` (only `AX` defined).
    TOMenuGetControlPad2 @ 0x008b_e440 => extern "C" fn to_menu_get_control_pad2() -> u32 {
        u32::from(pad::bridge::control_pad2(&mem()))
    }
    /// `TOMenuGetDummyAnalogControler(axis)` (only `AL` defined).
    TOMenuGetDummyAnalogControler @ 0x008c_a320 => extern "C" fn to_menu_get_dummy_analog_controler(axis: u32) -> u32 {
        u32::from(pad::bridge::dummy_analog(&mem(), axis))
    }
    /// `SetTOPadGetCtrl(v)` (a byte).
    SetTOPadGetCtrl @ 0x008a_b580 => extern "C" fn set_to_pad_get_ctrl(v: u32) {
        #[allow(clippy::cast_possible_truncation)] // the original stores AL
        pad::bridge::set_get_ctrl(&mut mem(), v as u8);
    }
    /// `TODebugAddPadPrintFlag()`.
    TODebugAddPadPrintFlag @ 0x008a_bd00 => extern "C" fn to_debug_add_pad_print_flag() {
        pad::bridge::add_print_flag(&mut mem());
    }

    /// `TOBtlCloseHelpWin()`.
    TOBtlCloseHelpWin @ 0x0089_0eb0 => extern "C" fn to_btl_close_help_win() {
        btl::bridge::close_help_win(&mut mem());
    }
    /// `TOBtlInitAnm()`.
    TOBtlInitAnm @ 0x008a_0cd0 => extern "C" fn to_btl_init_anm() {
        btl::bridge::init_anm(&mut mem());
    }
    /// `TOBtlCloseSignWindow()`.
    TOBtlCloseSignWindow @ 0x0089_0ee0 => extern "C" fn to_btl_close_sign_window() {
        btl::bridge::close_sign_window(&mut mem());
    }
    /// `TOBtlCloseSimpleHelpMes()`.
    TOBtlCloseSimpleHelpMes @ 0x0089_0ef0 => extern "C" fn to_btl_close_simple_help_mes() {
        btl::bridge::close_simple_help(&mut mem());
    }
    /// `TOGetBtlWindowControlEnable()`.
    TOGetBtlWindowControlEnable @ 0x0089_b590 => extern "C" fn to_get_btl_window_control_enable() -> u32 {
        btl::bridge::window_control_enable(&mem())
    }
    /// `TOBtlSetCurPos(x, y, z)` (16-bit each).
    TOBtlSetCurPos @ 0x0089_99f0 => extern "C" fn to_btl_set_cur_pos(x: u32, y: u32, z: u32) {
        #[allow(clippy::cast_possible_truncation)] // the original stores AX
        btl::bridge::set_cur_pos(&mut mem(), x as u16, y as u16, z as u16);
    }
    /// `TOBtlSetLuluLimitTbl(digits)`.
    TOBtlSetLuluLimitTbl @ 0x0089_a2f0 => extern "C" fn to_btl_set_lulu_limit_tbl(digits: u32) {
        btl::bridge::set_lulu_limit_table(&mut mem(), digits);
    }
    /// `TOBtlSetTargetCallback(w)`.
    TOBtlSetTargetCallback @ 0x008a_1090 => extern "C" fn to_btl_set_target_callback(w: u32) {
        btl::bridge::set_target_callback(&mut mem(), w);
    }

    /// `TOSeqGetNum(i, name, &n)`.
    TOSeqGetNum @ 0x008a_e410 => extern "C" fn to_seq_get_num(i: i32, name: u32, out: u32) {
        scene::bridge::get_num(&mut mem(), i, name, out, menu_scene::tk_menu_get_param_handle, menu_scene::tk_menu_get_param_offset);
    }
    /// `TOSeqGetColor(i, name, &rgba)`.
    TOSeqGetColor @ 0x008a_e360 => extern "C" fn to_seq_get_color(i: i32, name: u32, out: u32) {
        scene::bridge::get_color(&mut mem(), i, name, out, menu_scene::tk_menu_get_param_handle, menu_scene::tk_menu_get_param_offset);
    }
    /// `TOSeqGetPoint(i, name, &x, &y)`.
    TOSeqGetPoint @ 0x008a_e450 => extern "C" fn to_seq_get_point(i: i32, name: u32, x: u32, y: u32) {
        scene::bridge::get_point(&mut mem(), i, name, [x, y], menu_scene::tk_menu_get_param_handle, menu_scene::tk_menu_get_param_offset);
    }
    /// `TOSeqGetRect(i, name, &x, &y, &w, &h)`.
    TOSeqGetRect @ 0x008a_e4b0 => extern "C" fn to_seq_get_rect(i: i32, name: u32, x: u32, y: u32, w: u32, h: u32) {
        scene::bridge::get_rect(&mut mem(), i, name, [x, y, w, h], menu_scene::tk_menu_get_param_handle, menu_scene::tk_menu_get_param_offset);
    }

    /// `TOMenuGetFirstReadEndFlag()`.
    TOMenuGetFirstReadEndFlag @ 0x008a_8e50 => extern "C" fn to_menu_get_first_read_end_flag() -> i32 {
        menu::bridge::first_read_end(&mem())
    }
    /// `TOMenuHelpSetExtMode(mode, arg)`.
    TOMenuHelpSetExtMode @ 0x008b_ef10 => extern "C" fn to_menu_help_set_ext_mode(mode: u32, arg: u32) {
        menu::bridge::help_set_ext_mode(&mut mem(), mode, arg);
    }
    /// `TOMenuGetAbilitymapWorkAdrs()`.
    TOMenuGetAbilitymapWorkAdrs @ 0x008b_ea30 => extern "C" fn to_menu_get_abilitymap_work_adrs() -> u32 {
        menu::bridge::abilitymap_work(&mem())
    }
    /// `TOAmapGetItemListMax()`.
    TOAmapGetItemListMax @ 0x008c_8c30 => extern "C" fn to_amap_get_item_list_max() -> u32 {
        menu::bridge::amap_item_list_max(&mem())
    }
    /// `TOAmapGetItemType(i)`.
    TOAmapGetItemType @ 0x008c_6710 => extern "C" fn to_amap_get_item_type(i: i32) -> u32 {
        u32::from(menu::bridge::amap_item_type(&mem(), i))
    }
    /// `IsInTkMenuPause()`.
    IsInTkMenuPause @ 0x008a_8e10 => extern "C" fn is_in_tk_menu_pause() -> u32 {
        menu::bridge::is_in_pause(&mem())
    }
    /// `SetInvokedFromSelectCursor(v)`.
    SetInvokedFromSelectCursor @ 0x008e_89c0 => extern "C" fn set_invoked_from_select_cursor(v: u32) {
        menu::bridge::set_invoked_from_select_cursor(&mut mem(), v);
    }
    /// `TOGetMessageWindowMode()`.
    TOGetMessageWindowMode @ 0x008a_c7b0 => extern "C" fn to_get_message_window_mode() -> u32 {
        menu::bridge::message_window_mode(&mem())
    }
    /// `TOGetFFXToAlbhedTblAdrs()`.
    TOGetFFXToAlbhedTblAdrs @ 0x0090_6b20 => extern "C" fn to_get_ffx_to_albhed_tbl_adrs() -> u32 {
        menu::bridge::albhed_table(&mem())
    }
    /// `TOGetFontWidthTblAdrs(font)`.
    TOGetFontWidthTblAdrs @ 0x008a_c3d0 => extern "C" fn to_get_font_width_tbl_adrs(font: i32) -> u32 {
        menu::bridge::font_width_table(&mem(), font)
    }
    /// `TOGetLineFeedHeight()`: 16.
    TOGetLineFeedHeight @ 0x008b_7c60 => extern "C" fn to_get_line_feed_height() -> u32 {
        stub::LINE_FEED_HEIGHT
    }
    /// `TOFFXToSJIS(text, out)`.
    TOFFXToSJIS @ 0x008b_6200 => extern "C" fn to_ffx_to_sjis(_text: u32, out: u32) {
        menu::bridge::ffx_to_sjis(&mut mem(), out);
    }
    /// `TOMenuGetGetImportantName(id)`.
    TOMenuGetGetImportantName @ 0x008d_dee0 => extern "C" fn to_menu_get_get_important_name(id: u32) -> u32 {
        important_text(id, 0)
    }
    /// `TOMenuGetGetImportantHelp(id)`.
    TOMenuGetGetImportantHelp @ 0x008d_dea0 => extern "C" fn to_menu_get_get_important_help(id: u32) -> u32 {
        important_text(id, 8)
    }

    /// `TOInitAddPkt()`.
    TOInitAddPkt @ 0x0090_4650 => extern "C" fn to_init_add_pkt() {
        pkt::bridge::init_add_pkt(&mut mem());
    }
    /// `TOSetAddPkt()`.
    TOSetAddPkt @ 0x008f_ae10 => extern "C" fn to_set_add_pkt() {
        pkt::bridge::set_add_pkt(&mut mem());
    }
    /// `TOSetPktBuffAdrs(base, a, b)`.
    TOSetPktBuffAdrs @ 0x0090_47c0 => extern "C" fn to_set_pkt_buff_adrs(base: u32, a: u32, b: u32) {
        pkt::bridge::set_pkt_buff(&mut mem(), base, a, b);
    }
    /// `TOGetScreenWH(&w, &h)`.
    TOGetScreenWH @ 0x008f_a900 => extern "C" fn to_get_screen_wh(w: u32, h: u32) {
        pkt::bridge::screen_wh(&mut mem(), w, h);
    }

    /// `Fixup_ReadLeb128(&cursor)`.
    Fixup_ReadLeb128 @ 0x0060_ac00 => extern "C" fn fixup_read_leb128(cursor: u32) -> u32 {
        fixup::bridge::read_leb128(&mut mem(), cursor)
    }
    /// `Fixup_SetObject(record, index)`.
    Fixup_SetObject @ 0x0060_ae40 => extern "C" fn fixup_set_object(record: u32, index: u32) {
        fixup::bridge::set_object(&mut mem(), record, index);
    }
    /// `Fixup_SetArrayObject(record, index)`.
    Fixup_SetArrayObject @ 0x0060_ae30 => extern "C" fn fixup_set_array_object(record: u32, index: u32) {
        fixup::bridge::set_object(&mut mem(), record, index);
    }
    /// `Md5Digest_Less(b)`.
    Md5Digest_Less @ 0x0061_d6e0 => extern "thiscall" fn md5_digest_less(this: u32, b: u32) -> bool {
        vbf::bridge::less(&mem(), this, b)
    }
    /// `Vbf_MapLookup(digest)`.
    Vbf_MapLookup @ 0x0061_d140 => extern "thiscall" fn vbf_map_lookup(this: u32, digest: u32) -> u32 {
        vbf::bridge::map_lookup(&mem(), this, digest)
    }

    /// `AssetListReader::getAssetPath(i)`.
    AssetListReader_getAssetPath @ 0x006f_41b0 => extern "thiscall" fn asset_list_reader_get_asset_path(this: u32, i: i32) -> u32 {
        instance::bridge::asset_path(&mem(), this, i)
    }
    /// `AssetListReader::isModel(id)`.
    AssetListReader_isModel @ 0x006f_46b0 => extern "thiscall" fn asset_list_reader_is_model(this: u32, id: u32) -> bool {
        instance::bridge::is_model(&mem(), this, id)
    }
    /// `AssetListReader::getSymbolById(id)`.
    AssetListReader_getSymbolById @ 0x006f_42d0 => extern "thiscall" fn asset_list_reader_get_symbol_by_id(this: u32, id: u32) -> u32 {
        instance::bridge::symbol_by_id(&mem(), this, id)
    }
    /// `AbmapManager::ToDestroy()`.
    AbmapManager_ToDestroy @ 0x0068_57b0 => extern "thiscall" fn abmap_manager_to_destroy(this: u32) {
        instance::bridge::abmap_to_destroy(&mut mem(), this);
    }
    /// `ChrResourceCacheManager::isChrDelayedRelease()`.
    ChrResourceCacheManager_isChrDelayedRelease @ 0x0069_b7b0 => extern "thiscall" fn chr_resource_cache_manager_is_chr_delayed_release(this: u32) -> u32 {
        instance::bridge::is_chr_delayed_release(&mem(), this)
    }
    /// `AsyncLoadNewCharacter()`.
    AsyncLoadNewCharacter @ 0x0078_2f80 => extern "C" fn async_load_new_character() -> u32 {
        instance::bridge::async_load_new_character(&mut mem())
    }
    /// `LoadingScreen::setParameterManually(&a, &b, &c)`.
    LoadingScreen_setParameterManually @ 0x006e_4ff0 => extern "thiscall" fn loading_screen_set_parameter_manually(this: u32, a: u32, b: u32, c: u32) {
        instance::bridge::loading_screen_set_parameter(&mut mem(), this, a, b, c);
    }
    /// `abmCalcChrMoveCurMoveCheck(chr)`.
    abmCalcChrMoveCurMoveCheck @ 0x00a4_5000 => extern "C" fn abm_calc_chr_move_cur_move_check(chr: u32) -> u32 {
        instance::bridge::chr_move_cur_move_check(&mem(), chr)
    }

    /// `useExtendedAlpha(on)`.
    useExtendedAlpha @ 0x0071_e460 => extern "C" fn use_extended_alpha(on: u32) {
        render::bridge::use_extended_alpha(&mut mem(), on);
    }
    /// `useShaderPointLight()`.
    useShaderPointLight @ 0x0071_e6f0 => extern "C" fn use_shader_point_light() -> u32 {
        render::bridge::use_shader_point_light(&mem())
    }
    /// `DepthBiasHooks::clearDepthBias()`.
    DepthBiasHooks_clearDepthBias @ 0x006e_40b0 => extern "C" fn depth_bias_hooks_clear_depth_bias() {
        render::bridge::clear_depth_bias(&mut mem());
    }
    /// `JobSchedule::pppPart::clearFillBufferRequest()`.
    JobSchedule_pppPart_clearFillBufferRequest @ 0x0071_ecd0 => extern "C" fn job_schedule_ppp_part_clear_fill_buffer_request() {
        render::bridge::clear_fill_buffer_request(&mut mem());
    }

    /// `SaveDataIsBusy()`.
    SaveDataIsBusy @ 0x0064_8040 => extern "C" fn save_data_is_busy() -> u32 {
        save::bridge::is_busy_now(&mem())
    }
    /// `SaveDataSetScreenState(state)`.
    SaveDataSetScreenState @ 0x0064_86e0 => extern "C" fn save_data_set_screen_state(state: u32) {
        save::bridge::set_screen_state(&mut mem(), state);
    }
    /// `SaveDataIsOperationCanceled()`.
    SaveDataIsOperationCanceled @ 0x0064_80c0 => extern "C" fn save_data_is_operation_canceled() -> u32 {
        save::bridge::is_operation_canceled(&mem())
    }
    /// `SaveDataIsLoadingAlbhed()`.
    SaveDataIsLoadingAlbhed @ 0x0064_8070 => extern "C" fn save_data_is_loading_albhed() -> u32 {
        save::bridge::is_loading_albhed(&mem())
    }
    /// `SaveDataSetAlbhedFade(v)`.
    SaveDataSetAlbhedFade @ 0x0064_85e0 => extern "C" fn save_data_set_albhed_fade(v: u32) {
        save::bridge::set_albhed_fade(&mut mem(), v);
    }
    /// `SetLocalDataSlotAndTime(_, slot, a, b)` (slot 16-bit).
    SetLocalDataSlotAndTime @ 0x0064_8fa0 => extern "C" fn set_local_data_slot_and_time(_unused: u32, slot: u32, a: u32, b: u32) {
        #[allow(clippy::cast_possible_truncation)] // the original stores CX
        save::bridge::set_local_data_slot_and_time(&mut mem(), slot as u16, a, b);
    }
    /// `SaveDataGetCrc(save)`.
    SaveDataGetCrc @ 0x0064_7e80 => extern "C" fn save_data_get_crc(save_buf: u32) -> u32 {
        u32::from(save::bridge::get_crc(&mut mem(), save_buf, ffx_menu::tk::bridge::tk_crc))
    }
    /// `SaveDataWriteCrc(save)`.
    SaveDataWriteCrc @ 0x0064_8f20 => extern "C" fn save_data_write_crc(save_buf: u32) -> u32 {
        save::bridge::write_crc(&mut mem(), save_buf, ffx_menu::tk::bridge::tk_crc)
    }
    /// `SaveDataCheckCrc()`.
    SaveDataCheckCrc @ 0x0064_7d70 => extern "C" fn save_data_check_crc() -> u32 {
        save::bridge::check_crc(&mut mem(), ffx_menu::tk::bridge::tk_crc)
    }
    /// `Virtuos::InputManager::getInstance()`.
    Virtuos_InputManager_getInstance @ 0x0063_1300 => extern "C" fn virtuos_input_manager_get_instance() -> u32 {
        instance::bridge::input_manager(&mem())
    }
    /// `Virtuos::InputManager::getVibrationFlag()` (`this` in ECX).
    Virtuos_InputManager_getVibrationFlag @ 0x0063_1320 => extern "fastcall" fn virtuos_input_manager_get_vibration_flag(this: u32) -> bool {
        instance::bridge::vibration_flag(&mem(), this)
    }
    /// `Input_KeyDown(key, pad)` (`this` the input manager).
    Input_KeyDown @ 0x0062_9ca0 => extern "thiscall" fn input_key_down(this: u32, key: u32, pad: u32) -> u32 {
        instance::bridge::key_down(&mem(), this, key, pad)
    }
    /// `Instance_006dc800_Get()`.
    Instance_006dc800_Get @ 0x006d_c8a0 => extern "C" fn instance_006dc800_get() -> u32 {
        instance::bridge::morph_manager(&mem())
    }
    /// `TrophyManager::Instance()`.
    TrophyManager_Instance @ 0x0067_ac90 => extern "C" fn trophy_manager_instance() -> u32 {
        instance::bridge::trophy_manager(&mem())
    }
    /// `FFXGetVideoMode()`.
    FFXGetVideoMode @ 0x0088_7c40 => extern "C" fn ffx_get_video_mode() -> i32 {
        config::bridge::ffx_video_mode(&mem())
    }
    /// `sceGsSetDefDrawEnv2(...)`.
    sceGsSetDefDrawEnv2 @ 0x0088_7c60 => extern "C" fn sce_gs_set_def_draw_env2() -> i32 {
        config::bridge::gs_set_def_draw_env2(&mem())
    }
    /// `MsGetSavePlayerPtr_thunk(chr)`.
    MsGetSavePlayerPtr_thunk @ 0x0077_6f50 => extern "C" fn ms_get_save_player_ptr_thunk(chr: u32) -> u32 {
        names::bridge::save_player_ptr(&mem(), chr)
    }
    /// `hdd_state_clear()`.
    hdd_state_clear @ 0x0077_7420 => extern "C" fn hdd_state_clear() {
        cd::bridge::hdd_state_clear(&mut mem());
    }
    /// `hdd_set_pass_names()`.
    hdd_set_pass_names @ 0x0077_7940 => extern "C" fn hdd_set_pass_names() {
        cd::bridge::hdd_set_pass_names(&mut mem());
    }
    /// `get_hdd_err()`.
    get_hdd_err @ 0x0077_7e80 => extern "C" fn get_hdd_err() -> i32 {
        cd::bridge::hdd_err(&mem())
    }
    /// `get_hdd_status()`.
    get_hdd_status @ 0x0077_7e90 => extern "C" fn get_hdd_status() -> i32 {
        cd::bridge::hdd_status(&mem())
    }
    /// `TOCloseMessageWindow(i)`.
    TOCloseMessageWindow @ 0x008a_b8f0 => extern "C" fn to_close_message_window(i: i32) {
        window::bridge::close_message_window(&mut mem(), i);
    }
    /// `TOSetMessageWindowActive(i)`.
    TOSetMessageWindowActive @ 0x008a_f7d0 => extern "C" fn to_set_message_window_active(i: i32) -> u32 {
        window::bridge::set_message_window_active(&mut mem(), i)
    }
    /// `TOSetMessageWindowValue(i, v)`.
    TOSetMessageWindowValue @ 0x008a_fb90 => extern "C" fn to_set_message_window_value(i: i32, v: u32) -> u32 {
        window::bridge::set_message_window_value(&mut mem(), i, v)
    }
    /// `TODrawScissor(x0, y0, x1, y1)`.
    TODrawScissor @ 0x008f_91f0 => extern "C" fn to_draw_scissor(x0: i32, y0: i32, x1: i32, y1: i32) {
        pkt::bridge::make_pkt_scissor(&mut mem(), x0, y0, x1, y1);
    }
    /// `TOMakePktScissor(x0, y0, x1, y1)`.
    TOMakePktScissor @ 0x008f_dee0 => extern "C" fn to_make_pkt_scissor(x0: i32, y0: i32, x1: i32, y1: i32) {
        pkt::bridge::make_pkt_scissor(&mut mem(), x0, y0, x1, y1);
    }
    /// `TOGetGsRegWord()`: a constant in `edx:eax`.
    TOGetGsRegWord @ 0x008f_a7b0 => extern "C" fn to_get_gs_reg_word() -> u64 {
        pkt::bridge::GS_WORD_008FA7B0
    }
    /// `TouchManager::GetInstance()`.
    TouchManager_GetInstance @ 0x0068_98c0 => extern "C" fn touch_manager_get_instance() -> u32 {
        instance::bridge::word(&mem(), instance::bridge::TOUCH_MANAGER)
    }
    /// `AchievementManager_GetInstance()`.
    AchievementManager_GetInstance @ 0x006f_15b0 => extern "C" fn achievement_manager_get_instance() -> u32 {
        instance::bridge::word(&mem(), instance::bridge::ACHIEVEMENT_MANAGER)
    }
    /// `MagicFile_GetToBeDeleteMagicId()`.
    MagicFile_GetToBeDeleteMagicId @ 0x009d_a3d0 => extern "C" fn magic_file_get_to_be_delete_magic_id() -> u32 {
        instance::bridge::word(&mem(), instance::bridge::MAGIC_TO_DELETE)
    }
    /// `get_hdd_word_518()`.
    get_hdd_word_518 @ 0x0077_87b0 => extern "C" fn get_hdd_word_518() -> u32 {
        instance::bridge::word(&mem(), instance::bridge::HDD_WORD_518)
    }
    /// `TOGetWord_01841e08()`.
    TOGetWord_01841e08 @ 0x008a_c2d0 => extern "C" fn to_get_word_01841e08() -> u32 {
        instance::bridge::word(&mem(), instance::bridge::TO_GET_WORDS[0])
    }
    /// `TOGetWord_01841d98()`.
    TOGetWord_01841d98 @ 0x008a_c3f0 => extern "C" fn to_get_word_01841d98() -> u32 {
        instance::bridge::word(&mem(), instance::bridge::TO_GET_WORDS[1])
    }
    /// `TOGetWord_01841e00()`.
    TOGetWord_01841e00 @ 0x008a_c790 => extern "C" fn to_get_word_01841e00() -> u32 {
        instance::bridge::word(&mem(), instance::bridge::TO_GET_WORDS[2])
    }
    /// `TOGetWord_01841d9c()`.
    TOGetWord_01841d9c @ 0x008a_cf10 => extern "C" fn to_get_word_01841d9c() -> u32 {
        instance::bridge::word(&mem(), instance::bridge::TO_GET_WORDS[3])
    }
    /// `Table60_Get(i)` (`this` in ECX).
    Table60_Get @ 0x0063_11b0 => extern "thiscall" fn table60_get(this: u32, i: u32) -> u32 {
        instance::bridge::table_60(&mem(), this, i)
    }
    /// `EighthAtLeastOne(n, _)`.
    EighthAtLeastOne @ 0x006d_b870 => extern "stdcall" fn eighth_at_least_one(n: i32, _unused: u32) -> i32 {
        instance::bridge::eighth_at_least_one(n)
    }
    /// `TOSetMessageWindowMode(v)`.
    TOSetMessageWindowMode @ 0x008a_f840 => extern "C" fn to_set_message_window_mode(v: u32) -> u32 {
        window::bridge::set_message_window_mode(&mut mem(), v)
    }
    /// `TOIsMessageStatus3Pressed(i)`.
    TOIsMessageStatus3Pressed @ 0x008b_8530 => extern "C" fn to_is_message_status3_pressed(i: i32) -> u32 {
        window::bridge::is_message_status3_pressed(&mem(), i)
    }
    /// `TOMenuSet_018762ee(i, v)`.
    TOMenuSet_018762ee @ 0x0090_0f60 => extern "C" fn to_menu_set_018762ee(i: i32, v: u32) {
        window::bridge::menu_set_018762ee(&mut mem(), i, v);
    }
    /// `user_free(p)`: the original leaves `LeaveCriticalSection`'s `eax`,
    /// which is `RtlLeaveCriticalSection`'s `STATUS_SUCCESS`, 0 (what
    /// `tools/check-clobber.py` sees read after `0x007783cb` is the end of a
    /// function body Ghidra cut short; the caller clears `eax`).
    user_free @ 0x006f_b8b0 => extern "C" fn user_free(p: u32) -> u32 {
        heap::user_free(&mut mem(), &mut GameOs, p);
        0
    }
    /// `UserHeap_Free(p)` (`this` the allocator).
    UserHeap_Free @ 0x0094_3130 => extern "thiscall" fn user_heap_free(this: u32, p: u32) -> bool {
        heap::heap_free(&mut mem(), &mut GameOs, this, p)
    }
    /// `UserHeap_InsertFree(node, arena)`.
    UserHeap_InsertFree @ 0x0094_34c0 => extern "thiscall" fn user_heap_insert_free(this: u32, node: u32, arena: u32) {
        heap::insert_free(&mut mem(), this, node, arena);
    }
    /// `UserHeap_RemoveFree(node)`: leaves `node` in eax.
    UserHeap_RemoveFree @ 0x0094_35a0 => extern "thiscall" fn user_heap_remove_free(_this: u32, node: u32) -> u32 {
        heap::remove_free(&mut mem(), node);
        node
    }
    /// `UserHeap_BinIndex(size)`: in edx:eax.
    UserHeap_BinIndex @ 0x0094_32e0 => extern "thiscall" fn user_heap_bin_index(this: u32, size: u32) -> i64 {
        heap::bin_index(&mem(), this, size)
    }
    /// `UserHeap_ReleaseArena(arena)`.
    UserHeap_ReleaseArena @ 0x0094_3050 => extern "thiscall" fn user_heap_release_arena(this: u32, arena: u32) {
        heap::release_arena(&mut mem(), &mut GameOs, this, arena);
    }
    /// `UserHeap_NodeAlloc()` (the pool in ECX).
    UserHeap_NodeAlloc @ 0x0094_85b0 => extern "fastcall" fn user_heap_node_alloc(pool: u32) -> u32 {
        heap::node_alloc(&mut mem(), &mut GameOs, pool)
    }
    /// `UserHeap_NodeNewPage()` (the pool in ECX).
    UserHeap_NodeNewPage @ 0x0094_85f0 => extern "fastcall" fn user_heap_node_new_page(pool: u32) -> u32 {
        heap::node_new_page(&mut mem(), &mut GameOs, pool)
    }
    /// `UserHeap_NodeFree(node)` (`this` the pool).
    UserHeap_NodeFree @ 0x0094_86a0 => extern "thiscall" fn user_heap_node_free(this: u32, node: u32) {
        heap::node_free(&mut mem(), &mut GameOs, this, node);
    }
    /// `PageManager_Alloc(size, align)`.
    PageManager_Alloc @ 0x0094_8130 => extern "thiscall" fn page_manager_alloc(this: u32, size: u32, align: u32) -> u32 {
        heap::page_alloc(&mut mem(), &mut GameOs, this, size, align)
    }
    /// `PageAlloc(size, align)`: on `*g_PageManager`.
    PageAlloc @ 0x0094_8110 => extern "C" fn page_alloc(size: u32, align: u32) -> u32 {
        let m = &mut mem();
        let mgr = heap::PAGE_MANAGER.get(m);
        heap::page_alloc(m, &mut GameOs, mgr, size, align)
    }
    /// `PageManager_Commit(addr, size)`.
    PageManager_Commit @ 0x0094_7590 => extern "thiscall" fn page_manager_commit(this: u32, addr: u32, size: u32) -> bool {
        heap::commit(&mut mem(), &mut GameOs, this, addr, size)
    }
    /// `PageCommit(addr, size)`: on `*g_PageManager`.
    PageCommit @ 0x0094_7570 => extern "C" fn page_commit(addr: u32, size: u32) -> bool {
        let m = &mut mem();
        let mgr = heap::PAGE_MANAGER.get(m);
        heap::commit(m, &mut GameOs, mgr, addr, size)
    }
    /// `PageManager_CommitRange(&addr, &size)`.
    PageManager_CommitRange @ 0x0094_83d0 => extern "thiscall" fn page_manager_commit_range(this: u32, pa: u32, ps: u32) -> bool {
        heap::commit_range_at(&mut mem(), &mut GameOs, this, pa, ps)
    }
    /// `PageManager_QueueDecommit(addr, size)`.
    PageManager_QueueDecommit @ 0x0094_7690 => extern "thiscall" fn page_manager_queue_decommit(this: u32, addr: u32, size: u32) {
        heap::queue_decommit(&mut mem(), &mut GameOs, this, addr, size);
    }
    /// `PageQueueDecommit(addr, size)`: on `*g_PageManager`.
    PageQueueDecommit @ 0x0094_7670 => extern "C" fn page_queue_decommit(addr: u32, size: u32) {
        let m = &mut mem();
        let mgr = heap::PAGE_MANAGER.get(m);
        heap::queue_decommit(m, &mut GameOs, mgr, addr, size);
    }
    /// `PageManager_Release(addr)`.
    PageManager_Release @ 0x0094_7ec0 => extern "thiscall" fn page_manager_release(this: u32, addr: u32) {
        heap::release_pages(&mut mem(), &mut GameOs, this, addr);
    }
    /// `PageRelease(addr)`: on `*g_PageManager`.
    PageRelease @ 0x0094_7e80 => extern "C" fn page_release(addr: u32) {
        let m = &mut mem();
        let mgr = heap::PAGE_MANAGER.get(m);
        heap::release_pages(m, &mut GameOs, mgr, addr);
    }
    /// `PageManager_Flush(force)` (a byte).
    PageManager_Flush @ 0x0094_78e0 => extern "thiscall" fn page_manager_flush(this: u32, force: u32) {
        heap::flush(&mut mem(), &mut GameOs, this, force.to_le_bytes()[0]);
    }
    /// `PageHash_Insert(&key, &data)`.
    PageHash_Insert @ 0x0094_7270 => extern "thiscall" fn page_hash_insert(this: u32, key: u32, data: u32) {
        heap::hash_insert_at(&mut mem(), &mut GameOs, this, key, data);
    }
    /// `PageHash_Remove(&out, &key)`: returns `out`.
    PageHash_Remove @ 0x0094_8000 => extern "thiscall" fn page_hash_remove(this: u32, out: u32, key: u32) -> u32 {
        heap::hash_remove_at(&mut mem(), this, out, key)
    }
    /// `PageHash_Rehash(n)`.
    PageHash_Rehash @ 0x0094_82c0 => extern "thiscall" fn page_hash_rehash(this: u32, n: i32) {
        heap::hash_rehash(&mut mem(), &mut GameOs, this, n);
    }
    /// `PageHash_NodeAlloc()` (the table in ECX).
    PageHash_NodeAlloc @ 0x0094_7430 => extern "fastcall" fn page_hash_node_alloc(table: u32) -> u32 {
        heap::alloc_hash_node(&mut mem(), &mut GameOs, table)
    }
    /// `PageManager_QueueNodeAlloc()` (the pool in ECX).
    PageManager_QueueNodeAlloc @ 0x0094_7350 => extern "fastcall" fn page_manager_queue_node_alloc(pool: u32) -> u32 {
        heap::alloc_queue_node(&mut mem(), &mut GameOs, pool)
    }
    /// `PageManager_CacheNodeAlloc()` (the pool in ECX).
    PageManager_CacheNodeAlloc @ 0x0094_73c0 => extern "fastcall" fn page_manager_cache_node_alloc(pool: u32) -> u32 {
        heap::alloc_cache_node(&mut mem(), &mut GameOs, pool)
    }
    /// `VirtualAlloc_Commit_RW(addr, size)`.
    VirtualAlloc_Commit_RW @ 0x0094_38b0 => extern "C" fn virtual_alloc_commit_rw(addr: u32, size: u32) -> u32 {
        heap::commit_rw(&mut mem(), &mut GameOs, addr, size)
    }
    /// `VirtualFree_Decommit(addr, size)`.
    VirtualFree_Decommit @ 0x0094_38e0 => extern "C" fn virtual_free_decommit(addr: u32, size: u32) {
        heap::decommit(&mut mem(), &mut GameOs, addr, size);
    }
    /// `VirtualFree_Release(addr, size)`: returns `size`.
    VirtualFree_Release @ 0x0094_3940 => extern "C" fn virtual_free_release(addr: u32, size: u32) -> u32 {
        heap::release(&mut mem(), &mut GameOs, addr, size)
    }
    /// `VirtualAlloc_Reserve_NA(size)`.
    VirtualAlloc_Reserve_NA @ 0x0094_39a0 => extern "C" fn virtual_alloc_reserve_na(size: u32) -> u32 {
        heap::reserve(&mut mem(), &mut GameOs, size)
    }
    /// `user_malloc(size)`: a jump to `UserHeap_Malloc`.
    user_malloc @ 0x0068_7020 => extern "C" fn user_malloc(size: u32) -> u32 {
        heap::user_malloc(&mut mem(), &mut GameOs, size)
    }
    /// `UserHeap_Malloc(size)`.
    UserHeap_Malloc @ 0x006f_b910 => extern "C" fn user_heap_malloc(size: u32) -> u32 {
        heap::user_malloc(&mut mem(), &mut GameOs, size)
    }
    /// `user_aligned_malloc(size, align)`: a jump to `UserHeap_AlignedMalloc`.
    user_aligned_malloc @ 0x0068_6ff0 => extern "C" fn user_aligned_malloc(size: u32, align: u32) -> u32 {
        heap::user_aligned_malloc(&mut mem(), &mut GameOs, size, align)
    }
    /// `UserHeap_AlignedMalloc(size, align)`.
    UserHeap_AlignedMalloc @ 0x006f_b770 => extern "C" fn user_heap_aligned_malloc(size: u32, align: u32) -> u32 {
        heap::user_aligned_malloc(&mut mem(), &mut GameOs, size, align)
    }
    /// `user_mem_size(p)`: a jump to `UserHeap_MemSize`.
    user_mem_size @ 0x0068_7050 => extern "C" fn user_mem_size(p: u32) -> u32 {
        heap::user_mem_size(&mut mem(), &mut GameOs, p)
    }
    /// `UserHeap_MemSize(p)`.
    UserHeap_MemSize @ 0x006f_ba90 => extern "C" fn user_heap_mem_size(p: u32) -> u32 {
        heap::user_mem_size(&mut mem(), &mut GameOs, p)
    }
    /// `user_realloc(p, size)`: a jump to `UserHeap_Realloc`.
    user_realloc @ 0x0068_7060 => extern "C" fn user_realloc(p: u32, size: u32) -> u32 {
        heap::user_realloc(&mut mem(), &mut GameOs, p, size)
    }
    /// `UserHeap_Realloc(p, size)`.
    UserHeap_Realloc @ 0x006f_bb00 => extern "C" fn user_heap_realloc(p: u32, size: u32) -> u32 {
        heap::user_realloc(&mut mem(), &mut GameOs, p, size)
    }
    /// `UserHeap_Alloc(size)` (`this` the allocator).
    UserHeap_Alloc @ 0x0094_2b60 => extern "thiscall" fn user_heap_alloc(this: u32, size: u32) -> u32 {
        heap::heap_alloc(&mut mem(), &mut GameOs, this, size)
    }
    /// `UserHeap_FindFree(size, &arena)`.
    UserHeap_FindFree @ 0x0094_3380 => extern "thiscall" fn user_heap_find_free(this: u32, size: u32, arena: u32) -> u32 {
        heap::find_free_at(&mut mem(), &mut GameOs, this, size, arena)
    }
    /// `UserHeap_NewArena()` (the allocator in ECX).
    UserHeap_NewArena @ 0x0094_2dc0 => extern "fastcall" fn user_heap_new_arena(this: u32) -> u32 {
        heap::new_arena(&mut mem(), &mut GameOs, this)
    }
    /// `UserHeap_BlockSize(p)`.
    UserHeap_BlockSize @ 0x0094_3480 => extern "thiscall" fn user_heap_block_size(this: u32, p: u32) -> u32 {
        heap::block_size(&mem(), this, p)
    }
    /// `LocalizationManager_GetInstance()`.
    LocalizationManager_GetInstance @ 0x006d_b070 => extern "C" fn localization_manager_get_instance() -> u32 {
        instance::bridge::localization_manager(&mem(), &mut phyre_printf)
    }
    /// `TOGetFFXLang()`.
    TOGetFFXLang @ 0x0064_10e0 => extern "C" fn to_get_ffx_lang() -> u32 {
        instance::bridge::ffx_lang(&mem(), &mut phyre_printf)
    }
    /// `BufferAt8_dtor_00633b70()` (`this` in ECX).
    BufferAt8_dtor_00633b70 @ 0x0063_3b70 => extern "thiscall" fn buffer_at8_dtor_00633b70(this: u32) -> u32 {
        heap::free_member8(&mut mem(), &mut GameOs, this)
    }
    /// `BufferAt8_dtor_00633b80()` (`this` in ECX).
    BufferAt8_dtor_00633b80 @ 0x0063_3b80 => extern "thiscall" fn buffer_at8_dtor_00633b80(this: u32) -> u32 {
        heap::free_member8(&mut mem(), &mut GameOs, this)
    }
    /// `BufferAt8_dtor_00633b90()` (`this` in ECX).
    BufferAt8_dtor_00633b90 @ 0x0063_3b90 => extern "thiscall" fn buffer_at8_dtor_00633b90(this: u32) -> u32 {
        heap::free_member8(&mut mem(), &mut GameOs, this)
    }
    /// `BufferAt8_dtor_00633ba0()` (`this` in ECX).
    BufferAt8_dtor_00633ba0 @ 0x0063_3ba0 => extern "thiscall" fn buffer_at8_dtor_00633ba0(this: u32) -> u32 {
        heap::free_member8(&mut mem(), &mut GameOs, this)
    }
    /// `BufferAt8_dtor_00633bb0()` (`this` in ECX).
    BufferAt8_dtor_00633bb0 @ 0x0063_3bb0 => extern "thiscall" fn buffer_at8_dtor_00633bb0(this: u32) -> u32 {
        heap::free_member8(&mut mem(), &mut GameOs, this)
    }
    /// `BufferAt8_dtor_00633bc0()` (`this` in ECX).
    BufferAt8_dtor_00633bc0 @ 0x0063_3bc0 => extern "thiscall" fn buffer_at8_dtor_00633bc0(this: u32) -> u32 {
        heap::free_member8(&mut mem(), &mut GameOs, this)
    }
    /// `BufferAt8_dtor_00633bd0()` (`this` in ECX).
    BufferAt8_dtor_00633bd0 @ 0x0063_3bd0 => extern "thiscall" fn buffer_at8_dtor_00633bd0(this: u32) -> u32 {
        heap::free_member8(&mut mem(), &mut GameOs, this)
    }
    /// `BufferAt8_dtor_00633bf0()` (`this` in ECX).
    BufferAt8_dtor_00633bf0 @ 0x0063_3bf0 => extern "thiscall" fn buffer_at8_dtor_00633bf0(this: u32) -> u32 {
        heap::free_member8(&mut mem(), &mut GameOs, this)
    }
    /// `BufferAt8_dtor_00633c00()` (`this` in ECX).
    BufferAt8_dtor_00633c00 @ 0x0063_3c00 => extern "thiscall" fn buffer_at8_dtor_00633c00(this: u32) -> u32 {
        heap::free_member8(&mut mem(), &mut GameOs, this)
    }
    /// `BufferAt8_dtor_00633c10()` (`this` in ECX).
    BufferAt8_dtor_00633c10 @ 0x0063_3c10 => extern "thiscall" fn buffer_at8_dtor_00633c10(this: u32) -> u32 {
        heap::free_member8(&mut mem(), &mut GameOs, this)
    }
    /// `BufferAt8_dtor_00633c20()` (`this` in ECX).
    BufferAt8_dtor_00633c20 @ 0x0063_3c20 => extern "thiscall" fn buffer_at8_dtor_00633c20(this: u32) -> u32 {
        heap::free_member8(&mut mem(), &mut GameOs, this)
    }
    /// `BufferAt8_dtor_00633c30()` (`this` in ECX).
    BufferAt8_dtor_00633c30 @ 0x0063_3c30 => extern "thiscall" fn buffer_at8_dtor_00633c30(this: u32) -> u32 {
        heap::free_member8(&mut mem(), &mut GameOs, this)
    }
    /// `BufferAt8_dtor_00690a20()` (`this` in ECX).
    BufferAt8_dtor_00690a20 @ 0x0069_0a20 => extern "thiscall" fn buffer_at8_dtor_00690a20(this: u32) -> u32 {
        heap::free_member8(&mut mem(), &mut GameOs, this)
    }
    /// `Identity_008f2f70(x)`.
    Identity_008f2f70 @ 0x008f_2f70 => extern "C" fn identity_008f2f70(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// `Identity_008ed040(x)`.
    Identity_008ed040 @ 0x008e_d040 => extern "C" fn identity_008ed040(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// `TOAdpWords4(p, a, b, c, d)`.
    TOAdpWords4 @ 0x008e_f8e0 => extern "C" fn to_adp_words4(p: u32, a: u32, b: u32, c: u32, d: u32) -> u32 {
        pkt::bridge::adp_words4(&mut mem(), p, [a, b, c, d])
    }
    /// `TOAdpXY2(p, x, y)`.
    TOAdpXY2 @ 0x008e_fb90 => extern "C" fn to_adp_xy2(p: u32, x: u32, y: u32) -> u32 {
        pkt::bridge::adp_xy2(&mut mem(), p, x, y)
    }
    /// `TOAdpXYZ2Offset(p, x, y)`.
    TOAdpXYZ2Offset @ 0x008e_fbf0 => extern "C" fn to_adp_xyz2_offset(p: u32, x: u32, y: u32) -> u32 {
        pkt::bridge::adp_xyz2_offset(&mut mem(), p, x, y)
    }
    /// `TOAdpGsDirectRGBA(p, rgba)`.
    TOAdpGsDirectRGBA @ 0x008e_f790 => extern "C" fn to_adp_gs_direct_rgba(p: u32, rgba: u32) -> u32 {
        pkt::bridge::adp_gs_direct_rgba(&mut mem(), p, rgba)
    }
    /// `OpenFontScissor(x0, y0, x1, y1)`.
    OpenFontScissor @ 0x008e_8940 => extern "C" fn open_font_scissor(x0: f32, y0: f32, x1: f32, y1: f32) {
        pkt::bridge::open_font_scissor(&mut mem(), [x0, y0, x1, y1]);
    }
    /// `TOGetBtlMenuTexAdrs()`.
    TOGetBtlMenuTexAdrs @ 0x008a_c2c0 => extern "C" fn to_get_btl_menu_tex_adrs() -> u32 {
        instance::bridge::word(&mem(), instance::bridge::BTL_MENU_TEX)
    }
    /// `TOMenuGetYesNoWindow()`.
    TOMenuGetYesNoWindow @ 0x008a_9b60 => extern "C" fn to_menu_get_yes_no_window() -> u32 {
        instance::bridge::word(&mem(), instance::bridge::YES_NO_WINDOW)
    }
    /// `TOMacroSetType(win, n, t)`.
    TOMacroSetType @ 0x008b_8960 => extern "C" fn to_macro_set_type(win: i32, n: i32, t: u32) {
        window::bridge::macro_set_type(&mut mem(), win, n, t.to_le_bytes()[0]);
    }
    /// `TOMacroSetValue(win, n, v)`.
    TOMacroSetValue @ 0x008b_8980 => extern "C" fn to_macro_set_value(win: i32, n: i32, v: u32) -> u32 {
        window::bridge::macro_set_value(&mut mem(), win, n, v)
    }
    /// `TOGetTable_01868a90(i)`.
    TOGetTable_01868a90 @ 0x008b_7c40 => extern "C" fn to_get_table_01868a90(i: i32) -> u32 {
        window::bridge::table_01868a90(&mem(), i)
    }
    /// `TOCloseBtlWindow()`.
    TOCloseBtlWindow @ 0x0089_ade0 => extern "C" fn to_close_btl_window() -> u32 {
        btl::bridge::close_btl_window(&mut mem())
    }
    /// `hdd_set_pass_names_x()`.
    hdd_set_pass_names_x @ 0x0077_9a80 => extern "C" fn hdd_set_pass_names_x() {
        cd::bridge::hdd_set_pass_names_x(&mut mem());
    }
    /// `cd_module_file_sector(m, n)`.
    cd_module_file_sector @ 0x0076_c270 => extern "C" fn cd_module_file_sector(m: i32, n: i32) -> u32 {
        cd::bridge::module_file_sector(&mem(), m, n)
    }
    /// `std::_Uninitialized_copy` of 12-byte elements.
    std_Uninitialized_copy12_0059e830 @ 0x0059_e830 => extern "C" fn uninitialized_copy12(first: u32, last: u32, dest: u32) -> u32 {
        stl::bridge::uninitialized_copy12(&mut mem(), first, last, dest)
    }
    /// `memcmp` as compiled into the game.
    memcmp_00401020 @ 0x0040_1020 => extern "C" fn memcmp_00401020(a: u32, b: u32, n: u32) -> i32 {
        stl::bridge::memcmp(&mem(), a, b, n)
    }
    /// `SetWordAt_00402a00(i, v)` (`this` in ECX).
    SetWordAt_00402a00 @ 0x0040_2a00 => extern "thiscall" fn set_word_at(this: u32, i: u32, v: u32) -> u32 {
        stl::bridge::set_word(&mut mem(), this, i, v)
    }
    /// `MapFindU32_0062fe50(&key)` (`this` in ECX).
    MapFindU32_0062fe50 @ 0x0062_fe50 => extern "thiscall" fn map_find_u32(this: u32, key: u32) -> u32 {
        stl::bridge::map_find_u32(&mem(), this, key)
    }
    /// `cd_channel_clear(n)`.
    cd_channel_clear @ 0x0076_d530 => extern "C" fn cd_channel_clear(n: u32) {
        cd::bridge::cd_channel_clear(&mut mem(), n);
    }
    /// `hdd_install_find(archive, n)`.
    hdd_install_find @ 0x0076_d600 => extern "C" fn hdd_install_find(archive: u32, n: u32) -> u32 {
        cd::bridge::hdd_install_find(&mem(), archive, n)
    }
    /// `TOSetWindowRGBA(i, r, g, b, a)`.
    TOSetWindowRGBA @ 0x0090_6d90 => extern "C" fn to_set_window_rgba(i: i32, r: u32, g: u32, b: u32, a: u32) {
        window::bridge::set_window_rgba(&mut mem(), i, [r, g, b, a].map(|v| v.to_le_bytes()[0]));
    }
    /// `dvd_cd_unk8_lang3()`.
    dvd_cd_unk8_lang3 @ 0x0077_a600 => extern "C" fn dvd_cd_unk8_lang3() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a600)]
    }
    /// `dvd_cd_fat_data_size_lang2()`.
    dvd_cd_fat_data_size_lang2 @ 0x0077_a610 => extern "C" fn dvd_cd_fat_data_size_lang2() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a610)]
    }
    /// `dvd_cd_fat_sector_lang2()`.
    dvd_cd_fat_sector_lang2 @ 0x0077_a620 => extern "C" fn dvd_cd_fat_sector_lang2() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a620)]
    }
    /// `dvd_cd_fat_size_lang2()`.
    dvd_cd_fat_size_lang2 @ 0x0077_a630 => extern "C" fn dvd_cd_fat_size_lang2() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a630)]
    }
    /// `dvd_cd_fid_sector_lang2()`.
    dvd_cd_fid_sector_lang2 @ 0x0077_a640 => extern "C" fn dvd_cd_fid_sector_lang2() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a640)]
    }
    /// `dvd_cd_fid_size_lang2()`.
    dvd_cd_fid_size_lang2 @ 0x0077_a650 => extern "C" fn dvd_cd_fid_size_lang2() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a650)]
    }
    /// `dvd_cd_id_sector_lang2()`.
    dvd_cd_id_sector_lang2 @ 0x0077_a660 => extern "C" fn dvd_cd_id_sector_lang2() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a660)]
    }
    /// `dvd_cd_id_size_lang2()`.
    dvd_cd_id_size_lang2 @ 0x0077_a670 => extern "C" fn dvd_cd_id_size_lang2() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a670)]
    }
    /// `dvd_cd_unk7_lang2()`.
    dvd_cd_unk7_lang2 @ 0x0077_a680 => extern "C" fn dvd_cd_unk7_lang2() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a680)]
    }
    /// `dvd_cd_unk8_lang2()`.
    dvd_cd_unk8_lang2 @ 0x0077_a690 => extern "C" fn dvd_cd_unk8_lang2() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a690)]
    }
    /// `dvd_cd_fat_data_size_lang5()`.
    dvd_cd_fat_data_size_lang5 @ 0x0077_a6a0 => extern "C" fn dvd_cd_fat_data_size_lang5() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a6a0)]
    }
    /// `dvd_cd_fat_sector_lang5()`.
    dvd_cd_fat_sector_lang5 @ 0x0077_a6b0 => extern "C" fn dvd_cd_fat_sector_lang5() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a6b0)]
    }
    /// `dvd_cd_fat_size_lang5()`.
    dvd_cd_fat_size_lang5 @ 0x0077_a6c0 => extern "C" fn dvd_cd_fat_size_lang5() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a6c0)]
    }
    /// `dvd_cd_fid_sector_lang5()`.
    dvd_cd_fid_sector_lang5 @ 0x0077_a6d0 => extern "C" fn dvd_cd_fid_sector_lang5() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a6d0)]
    }
    /// `dvd_cd_fid_size_lang5()`.
    dvd_cd_fid_size_lang5 @ 0x0077_a6e0 => extern "C" fn dvd_cd_fid_size_lang5() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a6e0)]
    }
    /// `dvd_cd_id_sector_lang5()`.
    dvd_cd_id_sector_lang5 @ 0x0077_a6f0 => extern "C" fn dvd_cd_id_sector_lang5() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a6f0)]
    }
    /// `dvd_cd_id_size_lang5()`.
    dvd_cd_id_size_lang5 @ 0x0077_a700 => extern "C" fn dvd_cd_id_size_lang5() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a700)]
    }
    /// `dvd_cd_unk7_lang5()`.
    dvd_cd_unk7_lang5 @ 0x0077_a710 => extern "C" fn dvd_cd_unk7_lang5() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a710)]
    }
    /// `dvd_cd_unk8_lang5()`.
    dvd_cd_unk8_lang5 @ 0x0077_a720 => extern "C" fn dvd_cd_unk8_lang5() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a720)]
    }
    /// `dvd_cd_fat_data_size_lang_other()`.
    dvd_cd_fat_data_size_lang_other @ 0x0077_a730 => extern "C" fn dvd_cd_fat_data_size_lang_other() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a730)]
    }
    /// `dvd_cd_fat_sector_lang_other()`.
    dvd_cd_fat_sector_lang_other @ 0x0077_a740 => extern "C" fn dvd_cd_fat_sector_lang_other() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a740)]
    }
    /// `dvd_cd_fat_size_lang_other()`.
    dvd_cd_fat_size_lang_other @ 0x0077_a750 => extern "C" fn dvd_cd_fat_size_lang_other() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a750)]
    }
    /// `dvd_cd_fid_sector_lang_other()`.
    dvd_cd_fid_sector_lang_other @ 0x0077_a760 => extern "C" fn dvd_cd_fid_sector_lang_other() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a760)]
    }
    /// `dvd_cd_fid_size_lang_other()`.
    dvd_cd_fid_size_lang_other @ 0x0077_a770 => extern "C" fn dvd_cd_fid_size_lang_other() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a770)]
    }
    /// `dvd_cd_id_sector_lang_other()`.
    dvd_cd_id_sector_lang_other @ 0x0077_a780 => extern "C" fn dvd_cd_id_sector_lang_other() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a780)]
    }
    /// `dvd_cd_id_size_lang_other()`.
    dvd_cd_id_size_lang_other @ 0x0077_a790 => extern "C" fn dvd_cd_id_size_lang_other() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a790)]
    }
    /// `dvd_cd_unk7_lang_other()`.
    dvd_cd_unk7_lang_other @ 0x0077_a7a0 => extern "C" fn dvd_cd_unk7_lang_other() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a7a0)]
    }
    /// `dvd_cd_unk8_lang_other()`.
    dvd_cd_unk8_lang_other @ 0x0077_a7b0 => extern "C" fn dvd_cd_unk8_lang_other() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a7b0)]
    }
    /// `dvd_cd_fat_data_size_lang18()`.
    dvd_cd_fat_data_size_lang18 @ 0x0077_a7c0 => extern "C" fn dvd_cd_fat_data_size_lang18() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a7c0)]
    }
    /// `dvd_cd_fat_sector_lang18()`.
    dvd_cd_fat_sector_lang18 @ 0x0077_a7d0 => extern "C" fn dvd_cd_fat_sector_lang18() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a7d0)]
    }
    /// `dvd_cd_fat_size_lang18()`.
    dvd_cd_fat_size_lang18 @ 0x0077_a7e0 => extern "C" fn dvd_cd_fat_size_lang18() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a7e0)]
    }
    /// `dvd_cd_fid_sector_lang18()`.
    dvd_cd_fid_sector_lang18 @ 0x0077_a7f0 => extern "C" fn dvd_cd_fid_sector_lang18() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a7f0)]
    }
    /// `dvd_cd_fid_size_lang18()`.
    dvd_cd_fid_size_lang18 @ 0x0077_a800 => extern "C" fn dvd_cd_fid_size_lang18() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a800)]
    }
    /// `dvd_cd_id_sector_lang18()`.
    dvd_cd_id_sector_lang18 @ 0x0077_a810 => extern "C" fn dvd_cd_id_sector_lang18() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a810)]
    }
    /// `dvd_cd_id_size_lang18()`.
    dvd_cd_id_size_lang18 @ 0x0077_a820 => extern "C" fn dvd_cd_id_size_lang18() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a820)]
    }
    /// `dvd_cd_unk7_lang18()`.
    dvd_cd_unk7_lang18 @ 0x0077_a830 => extern "C" fn dvd_cd_unk7_lang18() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a830)]
    }
    /// `dvd_cd_unk8_lang18()`.
    dvd_cd_unk8_lang18 @ 0x0077_a840 => extern "C" fn dvd_cd_unk8_lang18() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a840)]
    }
    /// `dvd_cd_fat_data_size_english()`.
    dvd_cd_fat_data_size_english @ 0x0077_a850 => extern "C" fn dvd_cd_fat_data_size_english() -> u32 {
        cd::DVD_CD_LAYOUT[cd::dvd_cd_getter_kind(0x0077_a850)]
    }
    /// `dvd_cd_fat_data_size()`: the same for every language.
    dvd_cd_fat_data_size @ 0x0076_d6c0 => extern "C" fn dvd_cd_fat_data_size() -> u32 {
        cd::DVD_CD_LAYOUT[0]
    }
    /// `dvd_cd_fat_sector()`: the same for every language.
    dvd_cd_fat_sector @ 0x0076_d740 => extern "C" fn dvd_cd_fat_sector() -> u32 {
        cd::DVD_CD_LAYOUT[1]
    }
    /// `dvd_cd_fat_size()`: the same for every language.
    dvd_cd_fat_size @ 0x0076_d7c0 => extern "C" fn dvd_cd_fat_size() -> u32 {
        cd::DVD_CD_LAYOUT[2]
    }
    /// `dvd_cd_fid_sector()`: the same for every language.
    dvd_cd_fid_sector @ 0x0076_d840 => extern "C" fn dvd_cd_fid_sector() -> u32 {
        cd::DVD_CD_LAYOUT[3]
    }
    /// `dvd_cd_fid_size()`: the same for every language.
    dvd_cd_fid_size @ 0x0076_d8c0 => extern "C" fn dvd_cd_fid_size() -> u32 {
        cd::DVD_CD_LAYOUT[4]
    }
    /// `dvd_cd_id_sector()`: the same for every language.
    dvd_cd_id_sector @ 0x0076_d940 => extern "C" fn dvd_cd_id_sector() -> u32 {
        cd::DVD_CD_LAYOUT[5]
    }
    /// `dvd_cd_id_size()`: the same for every language.
    dvd_cd_id_size @ 0x0076_d9c0 => extern "C" fn dvd_cd_id_size() -> u32 {
        cd::DVD_CD_LAYOUT[6]
    }
    /// `dvd_cd_unk7()`: the same for every language.
    dvd_cd_unk7 @ 0x0076_da40 => extern "C" fn dvd_cd_unk7() -> u32 {
        cd::DVD_CD_LAYOUT[7]
    }
    /// `dvd_cd_unk8()`: the same for every language.
    dvd_cd_unk8 @ 0x0076_dac0 => extern "C" fn dvd_cd_unk8() -> u32 {
        cd::DVD_CD_LAYOUT[8]
    }
    /// `() -> 0x80`.
    ReturnConst_00405ba0 @ 0x0040_5ba0 => extern "C" fn returnconst_00405ba0() -> u32 {
        accessor::lookup(&accessor::CONSTANTS, 0x0040_5ba0)
    }
    /// `() -> 0x4`.
    ReturnConst_0059e5c0 @ 0x0059_e5c0 => extern "C" fn returnconst_0059e5c0() -> u32 {
        accessor::lookup(&accessor::CONSTANTS, 0x0059_e5c0)
    }
    /// `() -> 0x6900`.
    ReturnConst_00647cb0 @ 0x0064_7cb0 => extern "C" fn returnconst_00647cb0() -> u32 {
        accessor::lookup(&accessor::CONSTANTS, 0x0064_7cb0)
    }
    /// `() -> 0x98`.
    ReturnConst_006f1580 @ 0x006f_1580 => extern "C" fn returnconst_006f1580() -> u32 {
        accessor::lookup(&accessor::CONSTANTS, 0x006f_1580)
    }
    /// `() -> 0x18`.
    ReturnConst_006f1590 @ 0x006f_1590 => extern "C" fn returnconst_006f1590() -> u32 {
        accessor::lookup(&accessor::CONSTANTS, 0x006f_1590)
    }
    /// `() -> 0x10`.
    ReturnConst_006f15a0 @ 0x006f_15a0 => extern "C" fn returnconst_006f15a0() -> u32 {
        accessor::lookup(&accessor::CONSTANTS, 0x006f_15a0)
    }
    /// `() -> 0x640064`.
    ReturnConst_008b7cc0 @ 0x008b_7cc0 => extern "C" fn returnconst_008b7cc0() -> u32 {
        accessor::lookup(&accessor::CONSTANTS, 0x008b_7cc0)
    }
    /// `() -> &0x00b7c79c`.
    ReturnAddress_004013a0 @ 0x0040_13a0 => extern "C" fn returnaddress_004013a0() -> u32 {
        accessor::bridge::address(&mem(), 0x0040_13a0)
    }
    /// `() -> &0x00b7c7b4`.
    ReturnAddress_00401480 @ 0x0040_1480 => extern "C" fn returnaddress_00401480() -> u32 {
        accessor::bridge::address(&mem(), 0x0040_1480)
    }
    /// `() -> &0x00b7c7d8`.
    ReturnAddress_00401530 @ 0x0040_1530 => extern "C" fn returnaddress_00401530() -> u32 {
        accessor::bridge::address(&mem(), 0x0040_1530)
    }
    /// `() -> &0x00b42a34`.
    ReturnAddress_00679a00 @ 0x0067_9a00 => extern "C" fn returnaddress_00679a00() -> u32 {
        accessor::bridge::address(&mem(), 0x0067_9a00)
    }
    /// `() -> &0x00c41748`.
    ReturnAddress_00777230 @ 0x0077_7230 => extern "C" fn returnaddress_00777230() -> u32 {
        accessor::bridge::address(&mem(), 0x0077_7230)
    }
    /// `() -> &0x023cc480`.
    ReturnAddress_008bec70 @ 0x008b_ec70 => extern "C" fn returnaddress_008bec70() -> u32 {
        accessor::bridge::address(&mem(), 0x008b_ec70)
    }
    /// `() -> *0x01325ac4`.
    GetU32_01325ac4 @ 0x0064_7c90 => extern "C" fn getu32_01325ac4() -> u32 {
        accessor::bridge::global(&mem(), 0x0064_7c90)
    }
    /// `() -> *0x0112a010`.
    GetU32_0112a010 @ 0x0077_4670 => extern "C" fn getu32_0112a010() -> u32 {
        accessor::bridge::global(&mem(), 0x0077_4670)
    }
    /// `() -> *0x01841d30`.
    GetU32_01841d30 @ 0x008a_cf20 => extern "C" fn getu32_01841d30() -> u32 {
        accessor::bridge::global(&mem(), 0x008a_cf20)
    }
    /// `() -> *0x025d16c4`.
    GetU32_025d16c4 @ 0x008b_9130 => extern "C" fn getu32_025d16c4() -> u32 {
        accessor::bridge::global(&mem(), 0x008b_9130)
    }
    /// `() -> *0x00c5a084`.
    GetU32_00c5a084 @ 0x008b_e320 => extern "C" fn getu32_00c5a084() -> u32 {
        accessor::bridge::global(&mem(), 0x008b_e320)
    }
    /// `() -> *0x018763e0`.
    GetU32_018763e0 @ 0x0090_6730 => extern "C" fn getu32_018763e0() -> u32 {
        accessor::bridge::global(&mem(), 0x0090_6730)
    }
    /// `() -> *0x01940ab4`.
    GetU32_01940ab4 @ 0x009d_a2c0 => extern "C" fn getu32_01940ab4() -> u32 {
        accessor::bridge::global(&mem(), 0x009d_a2c0)
    }
    /// `() -> *0x00c64ca4`.
    GetU32_00c64ca4 @ 0x009d_a3a0 => extern "C" fn getu32_00c64ca4() -> u32 {
        accessor::bridge::global(&mem(), 0x009d_a3a0)
    }
    /// `() -> *0x01a85ba8`.
    GetFullArenaLocalization @ 0x00a4_2fa0 => extern "C" fn getfullarenalocalization() -> u32 {
        accessor::bridge::global(&mem(), 0x00a4_2fa0)
    }
    /// `() -> *0x023cbeb0`.
    GetU32_023cbeb0 @ 0x00a7_ae90 => extern "C" fn getu32_023cbeb0() -> u32 {
        accessor::bridge::global(&mem(), 0x00a7_ae90)
    }
    /// `ret 0xc`, returns -1.
    ReturnMinus1_0061a110 @ 0x0061_a110 => extern "stdcall" fn returnminus1_0061a110(_a: u32, _b: u32, _c: u32) -> i32 {
        -1
    }
    /// `ret 0xc`, returns -1.
    ReturnMinus1_006765d0 @ 0x0067_65d0 => extern "stdcall" fn returnminus1_006765d0(_a: u32, _b: u32, _c: u32) -> i32 {
        -1
    }
    /// `ret 0xc`, returns -1.
    ReturnMinus1_00677080 @ 0x0067_7080 => extern "stdcall" fn returnminus1_00677080(_a: u32, _b: u32, _c: u32) -> i32 {
        -1
    }
    /// `ret 0xc`, returns -1.
    ReturnMinus1_00677c30 @ 0x0067_7c30 => extern "stdcall" fn returnminus1_00677c30(_a: u32, _b: u32, _c: u32) -> i32 {
        -1
    }
    /// `ret 0xc`, returns -1.
    ReturnMinus1_00678b50 @ 0x0067_8b50 => extern "stdcall" fn returnminus1_00678b50(_a: u32, _b: u32, _c: u32) -> i32 {
        -1
    }
    /// `ret 0xc`, returns -1.
    ReturnMinus1_00a42490 @ 0x00a4_2490 => extern "stdcall" fn returnminus1_00a42490(_a: u32, _b: u32, _c: u32) -> i32 {
        -1
    }
    /// Its argument.
    Identity_0060ab50 @ 0x0060_ab50 => extern "C" fn identity_0060ab50(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_0060ab60 @ 0x0060_ab60 => extern "C" fn identity_0060ab60(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_0060b850 @ 0x0060_b850 => extern "C" fn identity_0060b850(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_0060b860 @ 0x0060_b860 => extern "C" fn identity_0060b860(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_0060c010 @ 0x0060_c010 => extern "C" fn identity_0060c010(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_0060c020 @ 0x0060_c020 => extern "C" fn identity_0060c020(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_0060c700 @ 0x0060_c700 => extern "C" fn identity_0060c700(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_0060c710 @ 0x0060_c710 => extern "C" fn identity_0060c710(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_0060c720 @ 0x0060_c720 => extern "C" fn identity_0060c720(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_0060c730 @ 0x0060_c730 => extern "C" fn identity_0060c730(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_0060c870 @ 0x0060_c870 => extern "C" fn identity_0060c870(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_0060c880 @ 0x0060_c880 => extern "C" fn identity_0060c880(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_0061e720 @ 0x0061_e720 => extern "C" fn identity_0061e720(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_0061e730 @ 0x0061_e730 => extern "C" fn identity_0061e730(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_006332d0 @ 0x0063_32d0 => extern "C" fn identity_006332d0(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_006332e0 @ 0x0063_32e0 => extern "C" fn identity_006332e0(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_006332f0 @ 0x0063_32f0 => extern "C" fn identity_006332f0(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_00633300 @ 0x0063_3300 => extern "C" fn identity_00633300(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_00633310 @ 0x0063_3310 => extern "C" fn identity_00633310(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_00633320 @ 0x0063_3320 => extern "C" fn identity_00633320(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_00633330 @ 0x0063_3330 => extern "C" fn identity_00633330(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_00686cb0 @ 0x0068_6cb0 => extern "C" fn identity_00686cb0(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_00686f80 @ 0x0068_6f80 => extern "C" fn identity_00686f80(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_0069a3d0 @ 0x0069_a3d0 => extern "C" fn identity_0069a3d0(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_0069bda0 @ 0x0069_bda0 => extern "C" fn identity_0069bda0(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_006f4870 @ 0x006f_4870 => extern "C" fn identity_006f4870(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_006f4880 @ 0x006f_4880 => extern "C" fn identity_006f4880(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_008ed020 @ 0x008e_d020 => extern "C" fn identity_008ed020(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_008ed030 @ 0x008e_d030 => extern "C" fn identity_008ed030(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_008ed050 @ 0x008e_d050 => extern "C" fn identity_008ed050(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_00a41900 @ 0x00a4_1900 => extern "C" fn identity_00a41900(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument.
    Identity_00a41910 @ 0x00a4_1910 => extern "C" fn identity_00a41910(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument (`ret 4`).
    IdentityStd_00686c50 @ 0x0068_6c50 => extern "stdcall" fn identitystd_00686c50(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument (`ret 4`).
    IdentityStd_0069a350 @ 0x0069_a350 => extern "stdcall" fn identitystd_0069a350(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument (`ret 4`).
    IdentityStd_0069a360 @ 0x0069_a360 => extern "stdcall" fn identitystd_0069a360(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument (`ret 4`).
    IdentityStd_0069a390 @ 0x0069_a390 => extern "stdcall" fn identitystd_0069a390(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument (`ret 4`).
    IdentityStd_00a41850 @ 0x00a4_1850 => extern "stdcall" fn identitystd_00a41850(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument (`ret 4`).
    IdentityStd_00a41860 @ 0x00a4_1860 => extern "stdcall" fn identitystd_00a41860(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument (`ret 4`).
    IdentityStd_00a41870 @ 0x00a4_1870 => extern "stdcall" fn identitystd_00a41870(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its argument (`ret 4`).
    IdentityStd_00a418f0 @ 0x00a4_18f0 => extern "stdcall" fn identitystd_00a418f0(x: u32) -> u32 {
        stub::ptr32(x)
    }
    /// Its second argument.
    Second_00401000 @ 0x0040_1000 => extern "C" fn second_00401000(a: u32, b: u32) -> u32 {
        accessor::second(a, b)
    }
    /// Its argument's low byte (only `al` set).
    IdentityU8_0060ab40 @ 0x0060_ab40 => extern "C" fn identityu8_0060ab40(x: u8) -> u8 {
        accessor::first_u8(x)
    }
    /// Its argument's low half (only `ax` set).
    IdentityU16_0061e710 @ 0x0061_e710 => extern "C" fn identityu16_0061e710(x: u16) -> u16 {
        accessor::first_u16(x)
    }
    /// Its argument as a `u64`.
    ZeroExtend64_0060ab70 @ 0x0060_ab70 => extern "C" fn zeroextend64_0060ab70(x: u32) -> u64 {
        accessor::zero_extend(x)
    }
    /// `a + b`.
    Add_0060ab80 @ 0x0060_ab80 => extern "C" fn add_0060ab80(a: u32, b: u32) -> u32 {
        accessor::add(a, b)
    }
    /// `a + b`.
    Add_0061d110 @ 0x0061_d110 => extern "C" fn add_0061d110(a: u32, b: u32) -> u32 {
        accessor::add(a, b)
    }
    /// `b - a`.
    Difference_0061e6e0 @ 0x0061_e6e0 => extern "C" fn difference_0061e6e0(a: u32, b: u32) -> u32 {
        accessor::difference(a, b)
    }
    /// `() -> 0x18` (`ret 4`).
    ReturnConstStd_006770b0 @ 0x0067_70b0 => extern "stdcall" fn returnconststd_006770b0(_a: u32) -> u32 {
        accessor::SIZE_0X18
    }
    /// Stores `Phyre::PInputs::PInputSource::vftable` in `this`.
    PInputSource_setVftable @ 0x009e_24f0 => extern "thiscall" fn pinputsource_setvftable(this: u32) {
        accessor::bridge::set_vftable(&mut mem(), this, accessor::bridge::PINPUT_SOURCE_VFTABLE);
    }
    /// Stores `Phyre::PWorldMatrixController::vftable` in `this`.
    PWorldMatrixController_setVftable @ 0x0061_a730 => extern "thiscall" fn pworldmatrixcontroller_setvftable(this: u32) {
        accessor::bridge::set_vftable(&mut mem(), this, accessor::bridge::PWORLD_MATRIX_CONTROLLER_VFTABLE);
    }
    /// `this[0] = 1`.
    SetWord0_006f1db0 @ 0x006f_1db0 => extern "thiscall" fn setword0_006f1db0(this: u32) {
        accessor::bridge::set_word0(&mut mem(), this);
    }
    /// Byte `+0x364` of `this` set.
    SetByte364_006f1dc0 @ 0x006f_1dc0 => extern "thiscall" fn setbyte364_006f1dc0(this: u32) {
        accessor::bridge::set_byte364(&mut mem(), this);
    }
    /// Whether the state (`+8`) is 6.
    SaveDataManager_isDeletingFinished @ 0x006f_20d0 => extern "thiscall" fn savedatamanager_isdeletingfinished(this: u32) -> u32 {
        accessor::bridge::state_is(&mem(), this, 6)
    }
    /// Whether the state (`+8`) is 4.
    SaveDataManager_isLoadingFinished @ 0x006f_2150 => extern "thiscall" fn savedatamanager_isloadingfinished(this: u32) -> u32 {
        accessor::bridge::state_is(&mem(), this, 4)
    }
    /// Whether the state (`+8`) is 2.
    SaveDataManager_isSavingFinished @ 0x006f_21a0 => extern "thiscall" fn savedatamanager_issavingfinished(this: u32) -> u32 {
        accessor::bridge::state_is(&mem(), this, 2)
    }
    /// The 64-bit size at `+8` of `*(this + 0xc)`.
    PStreamFile_entrySize64 @ 0x0061_acb0 => extern "thiscall" fn pstreamfile_entrysize64(this: u32) -> u64 {
        accessor::bridge::entry_size64(&mem(), this)
    }
    /// `this + 0x8c`.
    Virtuos_InputManager_getStickInfo @ 0x0063_1310 => extern "thiscall" fn getstickinfo_00631310(this: u32) -> u32 {
        accessor::bridge::stick_info(this)
    }
    /// Word `+0x64` of `g_TkMenuWorldMapWindow`.
    TkMenuWorldMapWindowWord64 @ 0x008e_2d50 => extern "C" fn tkmenuworldmapwindowword64() -> u32 {
        accessor::bridge::world_map_window_word64(&mem())
    }
    /// `TODapsInit()`.
    TODapsInit @ 0x0089_dc40 => extern "C" fn to_daps_init() -> u32 {
        daps::bridge::init(&mut mem())
    }
    /// `TODapsDraw()`: the same code as `TODapsInit`.
    TODapsDraw @ 0x0089_dc10 => extern "C" fn to_daps_draw() -> u32 {
        daps::bridge::init(&mut mem())
    }
    /// `TODapsString(s)`.
    TODapsString @ 0x0089_dec0 => extern "C" fn to_daps_string(s: u32) -> u32 {
        daps::bridge::string(&mut mem(), s)
    }
    /// `TODapsSetPos(x, y)`.
    TODapsSetPos @ 0x0089_de30 => extern "C" fn to_daps_set_pos(x: i32, y: i32) -> u32 {
        daps::bridge::set_pos(&mut mem(), x, y)
    }
    /// `TOAdpTEST(p, ate, atst, aref, afail, date, datm, zte, ztst)`.
    TOAdpTEST @ 0x008e_f970 => extern "C" fn to_adp_test(p: u32, ate: i32, atst: i32, aref: i32, afail: i32, date: i32, datm: i32, zte: i32, ztst: i32) -> u32 {
        pkt::bridge::adp_test(&mut mem(), p, [ate, atst, aref, afail, date, datm, zte, ztst])
    }
    /// `TouchRecords_Init(base, n)`.
    TouchRecords_Init @ 0x0068_9a10 => extern "C" fn touch_records_init(base: u32, n: u32) {
        pad::bridge::touch_records_init(&mut mem(), base, n);
    }
    /// `std::map<u32>::equal_range(&out, &key)` (`this` in ECX).
    std_map_u32_equal_range_0069a250 @ 0x0069_a250 => extern "thiscall" fn map_u32_equal_range(this: u32, out: u32, key: u32) -> u32 {
        stl::bridge::equal_range_u32(&mut mem(), this, out, key)
    }
    /// `TOFontGlyphSprite(s, out)`.
    TOFontGlyphSprite @ 0x0089_e360 => extern "C" fn to_font_glyph_sprite(s: u32, out: u32) -> u32 {
        msg::bridge::glyph_sprite(&mut mem(), s, out)
    }
    /// `TOSeqCheckTermScene(i, name)`.
    TOSeqCheckTermScene @ 0x008a_e070 => extern "C" fn to_seq_check_term_scene(i: i32, name: u32) -> u32 {
        scene::bridge::check_term_scene(&mut mem(), i, name, menu_scene::tk_menu_is_term_scene)
    }
    /// `Fixup_ReadRecord(&cursor, flags)` (the record in ECX).
    Fixup_ReadRecord @ 0x0060_e730 => extern "thiscall" fn fixup_read_record(record: u32, cursor: u32, flags: u32) -> u32 {
        fixup::bridge::read_record(&mut mem(), record, cursor, flags)
    }
    /// `SortSaveTable_006f0fb0()`.
    SortSaveTable_006f0fb0 @ 0x006f_0fb0 => extern "C" fn sort_save_table() -> u32 {
        save::bridge::sort_table(&mut mem())
    }
    /// `ResetRecords_00774740()`.
    ResetRecords_00774740 @ 0x0077_4740 => extern "C" fn reset_records_00774740() -> u32 {
        instance::bridge::reset_records(&mut mem())
    }
    /// `TOSeqStartScene(i, name)`.
    TOSeqStartScene @ 0x008a_f0d0 => extern "C" fn to_seq_start_scene(i: i32, name: u32) -> u32 {
        scene::bridge::start_scene(&mut mem(), i, name, false, menu_scene::tk_menu_get_scene_handle)
    }
    /// `TOSeqStartSceneDrawContinue(i, name)`.
    TOSeqStartSceneDrawContinue @ 0x008a_f1b0 => extern "C" fn to_seq_start_scene_draw_continue(i: i32, name: u32) -> u32 {
        scene::bridge::start_scene(&mut mem(), i, name, true, menu_scene::tk_menu_get_scene_handle)
    }
    /// `TOSeqStopScene(i, name)`.
    TOSeqStopScene @ 0x008a_f4f0 => extern "C" fn to_seq_stop_scene(i: i32, name: u32) -> u32 {
        scene::bridge::stop_scene(&mut mem(), i, name)
    }
    /// `TOSeqInitAllScene(i)`.
    TOSeqInitAllScene @ 0x008a_e5a0 => extern "C" fn to_seq_init_all_scene(i: i32) -> u32 {
        scene::bridge::init_all_scene(&mut mem(), i)
    }
    /// `TOBtlSetPrevAction()`.
    TOBtlSetPrevAction @ 0x0089_a630 => extern "C" fn to_btl_set_prev_action() -> u32 {
        btl::bridge::set_prev_action(&mut mem())
    }
    /// `JobSchedule_pppPart_getFillBufferRequest(size)`.
    JobSchedule_pppPart_getFillBufferRequest @ 0x0072_8660 => extern "C" fn get_fill_buffer_request(size: i32) -> u32 {
        render::bridge::get_fill_buffer_request(&mut mem(), size)
    }
    /// `TouchTracks_Init(base, n)`.
    TouchTracks_Init @ 0x0068_98d0 => extern "C" fn touch_tracks_init(base: u32, n: u32) -> u32 {
        pad::bridge::touch_tracks_init(&mut mem(), base, n)
    }
    /// `std_map_cstr_find_006dc010(&key)` (`this` in ECX).
    std_map_cstr_find_006dc010 @ 0x006d_c010 => extern "thiscall" fn map_cstr_find(this: u32, key: u32) -> u32 {
        stl::bridge::map_find_cstr(&mem(), this, key)
    }
    /// `GetTable_00c8a538(i)` (`ret 4`).
    GetTable_00c8a538 @ 0x0040_2be0 => extern "stdcall" fn get_table_00c8a538(i: u32) -> u32 {
        instance::bridge::table_00c8a538(&mem(), i)
    }
    /// `LocalizationManager_GetWord0()`.
    LocalizationManager_GetWord0 @ 0x0064_1100 => extern "C" fn localization_manager_get_word0() -> u32 {
        instance::bridge::localization_manager_word0(&mem(), &mut phyre_printf)
    }
    /// `SetTable_02310f20(i, v)`.
    SetTable_02310f20 @ 0x008b_dfc0 => extern "C" fn set_table_02310f20(i: u32, v: u32) -> u32 {
        instance::bridge::set_table_02310f20(&mut mem(), i, v)
    }
    /// `SaveDataManager_SetWord360(v)`.
    SaveDataManager_SetWord360 @ 0x0064_8640 => extern "C" fn save_data_manager_set_word360(v: u32) -> u32 {
        save::bridge::set_manager_word360(&mut mem(), v)
    }
    /// `TOGetTableKind_008b7fb0(i)`.
    TOGetTableKind_008b7fb0 @ 0x008b_7fb0 => extern "C" fn to_get_table_kind(i: i32) -> i32 {
        window::bridge::table_kind(&mem(), i)
    }
    /// `TOGetTableKind2Data_008b7fd0(i)`.
    TOGetTableKind2Data_008b7fd0 @ 0x008b_7fd0 => extern "C" fn to_get_table_kind2_data(i: i32) -> u32 {
        window::bridge::table_kind2_data(&mem(), i)
    }
    /// `TOGetTableKind2Byte7_008b7ff0(i)`.
    TOGetTableKind2Byte7_008b7ff0 @ 0x008b_7ff0 => extern "C" fn to_get_table_kind2_byte7(i: i32) -> i32 {
        window::bridge::table_kind2_byte7(&mem(), i)
    }
    /// `cd_module_file_sector_rel(n)`.
    cd_module_file_sector_rel @ 0x0076_c250 => extern "C" fn cd_module_file_sector_rel(n: i32) -> u32 {
        cd::bridge::cd_module_file_sector_rel(&mem(), n)
    }
    /// `TOMacroSetByte1(win, n, v)`.
    TOMacroSetByte1 @ 0x008b_84f0 => extern "C" fn to_macro_set_byte1(win: i32, n: i32, v: u8) {
        window::bridge::macro_set_byte1(&mut mem(), win, n, v);
    }
    /// `TOMacroSetValue_008b8510(win, n, v)`: the same code as `TOMacroSetValue`.
    TOMacroSetValue_008b8510 @ 0x008b_8510 => extern "C" fn to_macro_set_value_008b8510(win: i32, n: i32, v: u32) -> u32 {
        window::bridge::macro_set_value(&mut mem(), win, n, v)
    }
    /// `TOAllCloseMessageWindow()`.
    TOAllCloseMessageWindow @ 0x008a_b7b0 => extern "C" fn to_all_close_message_window() {
        window::bridge::all_close_message_window(&mut mem());
    }
    /// `Kernel_GetAbilityName(id)`.
    Kernel_GetAbilityName @ 0x0077_4720 => extern "C" fn kernel_get_ability_name(id: u32) -> u32 {
        names::bridge::ability_name(&mut mem(), id, ffx_menu::tk::bridge::tk_mn2_get_ability_mes)
    }
    /// `user_aligned_free(p)`.
    user_aligned_free @ 0x006f_b6c0 => extern "C" fn user_aligned_free(p: u32) {
        heap::user_aligned_free(&mut mem(), &mut GameOs, p);
    }
    /// `UserHeap_Calloc(n, size)`.
    UserHeap_Calloc @ 0x006f_b820 => extern "C" fn user_heap_calloc(n: u32, size: u32) -> u32 {
        heap::user_calloc(&mut mem(), &mut GameOs, n, size)
    }
    /// `user_calloc(n, size)`: the jump to `UserHeap_Calloc`.
    user_calloc @ 0x0068_7000 => extern "C" fn user_calloc(n: u32, size: u32) -> u32 {
        heap::user_calloc(&mut mem(), &mut GameOs, n, size)
    }
    /// `malloc_pool_init()`.
    malloc_pool_init @ 0x006f_b9b0 => extern "C" fn malloc_pool_init() {
        heap::malloc_pool_init(&mut mem(), &mut GameOs);
    }
    /// `MemSystem_Ensure()`.
    MemSystem_Ensure @ 0x0094_28a0 => extern "C" fn mem_system_ensure() {
        heap::mem_system_ensure(&mem(), &mut GameOs);
    }
    /// `UserHeap_Init(arena, a, b)` (`this` in ECX).
    UserHeap_Init @ 0x0094_2a40 => extern "thiscall" fn user_heap_init(this: u32, arena: u32, a: u32, b: u32) -> u32 {
        heap::heap_init(&mut mem(), this, arena, a, b)
    }
    /// `Phyre::PhyreMalloc(size)`.
    Phyre_PhyreMalloc @ 0x0042_fb40 => extern "C" fn phyre_malloc(size: u32) -> u32 {
        heap::phyre_malloc(&mut mem(), &mut GameOs, size)
    }
    /// `Phyre::PhyreMalloc(size, align)`.
    Phyre_PhyreMalloc_0042fb80 @ 0x0042_fb80 => extern "C" fn phyre_malloc_aligned(size: u32, align: u32) -> u32 {
        heap::phyre_malloc_aligned(&mut mem(), &mut GameOs, size, align)
    }
    /// `Phyre::PhyreFree(p)`.
    Phyre_PhyreFree @ 0x0042_fb20 => extern "C" fn phyre_free(p: u32) {
        heap::phyre_free(&mut mem(), &mut GameOs, p);
    }
    /// `operator new(size)`: `user_malloc`.
    operator_new @ 0x0063_0310 => extern "C" fn operator_new(size: u32) -> u32 {
        heap::user_malloc(&mut mem(), &mut GameOs, size)
    }
    /// `operator new[](size)`: `user_malloc`.
    operator_new_006304c0 @ 0x0063_04c0 => extern "C" fn operator_new_array(size: u32) -> u32 {
        heap::user_malloc(&mut mem(), &mut GameOs, size)
    }
    /// `memclear(p, n)`.
    memclear @ 0x0062_f4e0 => extern "C" fn memclear(p: u32, n: u32) -> u32 {
        heap::memclear(&mut mem(), p, n)
    }
    /// `PStreamFileWin32::isOpen()` (`this` in ECX).
    Phyre_PSerialization_PStreamFileWin32_isOpen @ 0x0060_7f20 => extern "thiscall" fn stream_is_open(this: u32) -> bool {
        files::bridge::stream_is_open(&mem(), this)
    }
    /// `PStreamFile::getFileSize()` (`this` in ECX).
    Phyre_PSerialization_PStreamFile_getFileSize @ 0x0060_7dc0 => extern "thiscall" fn stream_file_size(this: u32) -> u64 {
        files::bridge::stream_file_size(&mut mem(), this, get_file_size_ex)
    }
    /// `PhyreFIOSStreamFile::getFileSize()` (`this` in ECX).
    PhyreFIOSStreamFile_getFileSize @ 0x006f_9d50 => extern "thiscall" fn fios_stream_file_size(this: u32) -> u64 {
        files::bridge::fios_stream_file_size(&mut mem(), this, get_file_size_ex)
    }
    /// `fiosFileSize(stream)`.
    fiosFileSize @ 0x0067_9570 => extern "C" fn fios_file_size(stream: u32) -> u32 {
        files::bridge::fios_file_size(&mut mem(), stream, get_file_size_ex)
    }
    /// `sceFileSize(stream)`.
    sceFileSize @ 0x0062_f760 => extern "C" fn sce_file_size(stream: u32) -> u32 {
        files::bridge::fios_file_size(&mut mem(), stream, get_file_size_ex)
    }
    /// `fiosRead(stream, buf, n)`.
    fiosRead @ 0x0067_9800 => extern "C" fn fios_read(stream: u32, buf: u32, n: u32) -> u32 {
        files::bridge::fios_read(&mut mem(), stream, buf, n, stream_read)
    }
    /// `sceRead(fd, buf, n)`.
    sceRead @ 0x0062_fc00 => extern "C" fn sce_read(fd: u32, buf: u32, n: u32) -> u32 {
        files::bridge::sce_read(&mut mem(), fd, buf, n, stream_read)
    }
    /// `PhyreFIOS::isFileExist(name)`: `File_Probe`.
    PhyreFIOS_isFileExist @ 0x006f_9d70 => extern "C" fn phyre_fios_is_file_exist(name: u32) -> u32 {
        call_sym(&mut mem(), 0x0060_7e00, &[name])
    }
    /// `fiosIsFileExist(name)`.
    fiosIsFileExist @ 0x0067_96f0 => extern "C" fn fios_is_file_exist(name: u32) -> u32 {
        files::bridge::fios_is_file_exist(&mut mem(), name, |m, n| call_sym(m, 0x0060_7e00, &[n]))
    }
    /// `TODVDFileRead(file, buf, len)`.
    TODVDFileRead @ 0x008a_ba30 => extern "C" fn to_dvd_file_read(file: u32, buf: u32, len: u32) -> u32 {
        files::bridge::dvd_file_read(&mut mem(), file, buf, len, call_ptr)
    }
    /// `TOCDGetFileName(file, out)`.
    TOCDGetFileName @ 0x008a_b7d0 => extern "C" fn to_cd_get_file_name(file: u32, out: u32) -> u32 {
        files::bridge::cd_get_file_name(&mut mem(), file, out, call_ptr)
    }
    /// `CDfileName_PC(n)`.
    CDfileName_PC @ 0x00a4_2830 => extern "C" fn cd_file_name_pc(n: u32) -> u32 {
        files::bridge::cd_file_name_pc(&mut mem(), n, |m, at| call_sym(m, 0x00a4_2c00, &[at]))
    }
    /// `GetCDFileName_PC(n)`.
    GetCDFileName_PC @ 0x00a4_29d0 => extern "C" fn get_cd_file_name_pc(n: u32) -> u32 {
        files::bridge::get_cd_file_name_pc(&mut mem(), n, |m, at| call_sym(m, 0x00a4_2c00, &[at]))
    }
    /// `set_read_chgname(from, to)`.
    set_read_chgname @ 0x00a4_3220 => extern "C" fn set_read_chgname(from: u32, to: u32) -> u32 {
        cd::bridge::set_read_chgname(&mut mem(), from, to, |m, e, f, l| {
            call_sym(m, 0x0076_efa0, &[e, f, l]);
        })
    }
    /// `VFile::assign(handle)` (`this` in ECX).
    VFile_assign @ 0x0061_aa10 => extern "thiscall" fn vfile_assign(this: u32, handle: u32) -> u32 {
        files::bridge::vfile_assign(&mut mem(), this, handle)
    }
    /// `VFile::fileRead(buf, n)` (`this` in ECX).
    VFile_fileRead @ 0x0061_aa60 => extern "thiscall" fn vfile_file_read(this: u32, buf: u32, n: u32) -> u32 {
        files::bridge::vfile_file_read(&mut mem(), this, buf, n, |_, t, b, c| {
            // SAFETY: FileSys_Read (thiscall, ret 8), not ported yet.
            unsafe {
                let f: unsafe extern "thiscall" fn(u32, u32, u32) -> u32 =
                    std::mem::transmute(crate::rebase(0x0061_aa90));
                f(t, b, c)
            }
        })
    }
    /// `BigFileStream::setStreamPrefix(path)` (`this` in ECX).
    BigFileStream_setStreamPrefix @ 0x0061_c3a0 => extern "thiscall" fn set_stream_prefix(this: u32, path: u32) -> u32 {
        files::bridge::set_stream_prefix(&mut mem(), &mut GameOs, this, path)
    }
    /// `TOControlWindow(i)`.
    TOControlWindow @ 0x0090_6bd0 => extern "C" fn to_control_window(i: i32) -> u32 {
        window::bridge::control_window(&mut mem(), i, call_ptr)
    }
    /// `TODrawWindow(i)`.
    TODrawWindow @ 0x0090_6bf0 => extern "C" fn to_draw_window(i: i32) -> u32 {
        window::bridge::draw_window(&mut mem(), i, call_ptr)
    }
    /// `TOControlMessage(i)`.
    TOControlMessage @ 0x008b_5850 => extern "C" fn to_control_message(i: i32) -> u32 {
        window::bridge::control_message(&mut mem(), i, call_ptr)
    }
    /// `TOMessageCallF8_008b8ca0(i)`.
    TOMessageCallF8_008b8ca0 @ 0x008b_8ca0 => extern "C" fn to_message_call_f8(i: i32) -> u32 {
        window::bridge::message_call_f8(&mut mem(), i, call_ptr)
    }
    /// `TOGetControlPadStatus(i)`.
    TOGetControlPadStatus @ 0x008b_6b70 => extern "C" fn to_get_control_pad_status(i: i32) -> u32 {
        window::bridge::control_pad_status(&mut mem(), i, call_ptr)
    }
    /// `TOMessageTableByte2_008b8010(i)`.
    TOMessageTableByte2_008b8010 @ 0x008b_8010 => extern "C" fn to_message_table_byte2(i: i32) -> i32 {
        window::bridge::message_table_byte2(&mem(), i)
    }
    /// `TOSetPolyG4A_XYWH(p, x, y, w, h)`.
    TOSetPolyG4A_XYWH @ 0x0090_48a0 => extern "C" fn to_set_poly_g4a_xywh(p: u32, x: u32, y: u32, width: u32, height: u32) {
        window::bridge::set_poly_g4a_xywh(&mut mem(), p, x, y, width, height);
    }
    /// `cd_size_from_table(n)`.
    cd_size_from_table @ 0x0076_dc20 => extern "C" fn cd_size_from_table(n: i32) -> u32 {
        cd::bridge::cd_size_from_table(&mem(), n)
    }
    /// `TOAdpALPHA(p, A, B, C, D, FIX)`.
    TOAdpALPHA @ 0x008e_f670 => extern "C" fn toadpalpha(p: u32, f0: i32, f1: i32, f2: i32, f3: i32, f4: i32) -> u32 {
        pkt::bridge::adp_reg_at(&mut mem(), 0x008e_f670, p, &[f0, f1, f2, f3, f4])
    }
    /// `TOAdpCLAMP(p, WMS, WMT, MINU, MAXU, MINV, MAXV)`.
    TOAdpCLAMP @ 0x008e_f6e0 => extern "C" fn toadpclamp(p: u32, f0: i32, f1: i32, f2: i32, f3: i32, f4: i32, f5: i32) -> u32 {
        pkt::bridge::adp_reg_at(&mut mem(), 0x008e_f6e0, p, &[f0, f1, f2, f3, f4, f5])
    }
    /// `TOAdpCOLCLAMP(p, value)`.
    TOAdpCOLCLAMP @ 0x008e_f760 => extern "C" fn toadpcolclamp(p: u32, f0: i32) -> u32 {
        pkt::bridge::adp_reg_at(&mut mem(), 0x008e_f760, p, &[f0])
    }
    /// `TOAdpFBA(p, value)`.
    TOAdpFBA @ 0x008e_f7d0 => extern "C" fn toadpfba(p: u32, f0: i32) -> u32 {
        pkt::bridge::adp_reg_at(&mut mem(), 0x008e_f7d0, p, &[f0])
    }
    /// `TOAdpFRAME(p, FBP, FBW, PSM, FBMSK)`.
    TOAdpFRAME @ 0x008e_f880 => extern "C" fn toadpframe(p: u32, f0: i32, f1: i32, f2: i32, f3: i32) -> u32 {
        pkt::bridge::adp_reg_at(&mut mem(), 0x008e_f880, p, &[f0, f1, f2, f3])
    }
    /// `TOAdpSCISSOR(p, SCAX0, SCAX1, SCAY0, SCAY1)`.
    TOAdpSCISSOR @ 0x008e_f910 => extern "C" fn toadpscissor(p: u32, f0: i32, f1: i32, f2: i32, f3: i32) -> u32 {
        pkt::bridge::adp_reg_at(&mut mem(), 0x008e_f910, p, &[f0, f1, f2, f3])
    }
    /// `TOAdpTEX1(p, LCM, MXL, MMAG, MMIN, MTBA, L, K)`.
    TOAdpTEX1 @ 0x008e_fad0 => extern "C" fn toadptex1(p: u32, f0: i32, f1: i32, f2: i32, f3: i32, f4: i32, f5: i32, f6: i32) -> u32 {
        pkt::bridge::adp_reg_at(&mut mem(), 0x008e_fad0, p, &[f0, f1, f2, f3, f4, f5, f6])
    }
    /// `TOAdpXYOFFSET(p, OFX, OFY)`.
    TOAdpXYOFFSET @ 0x008e_fc60 => extern "C" fn toadpxyoffset(p: u32, f0: i32, f1: i32) -> u32 {
        pkt::bridge::adp_reg_at(&mut mem(), 0x008e_fc60, p, &[f0, f1])
    }
    /// `TOAdpFOG`.
    TOAdpFOG @ 0x008e_f800 => extern "C" fn toadpfog(p: u32, fog: u32) -> u32 {
        pkt::bridge::adp_fog(&mut mem(), p, fog)
    }
    /// `TOAdpTEXFLUSH`.
    TOAdpTEXFLUSH @ 0x008e_fb60 => extern "C" fn toadptexflush(p: u32) -> u32 {
        pkt::bridge::adp_texflush(&mut mem(), p)
    }
    /// `TOAdpXYFrac`.
    TOAdpXYFrac @ 0x008e_fbc0 => extern "C" fn toadpxyfrac(p: u32, x: u32, fx: u32, y: u32, fy: u32) -> u32 {
        pkt::bridge::adp_xy_frac(&mut mem(), p, [x, fx, y, fy])
    }
    /// `TOAdpWords2Reg10`.
    TOAdpWords2Reg10 @ 0x008e_fc30 => extern "C" fn toadpwords2reg10(p: u32, a: u32, b: u32) -> u32 {
        pkt::bridge::adp_words2_reg10(&mut mem(), p, a, b)
    }
    /// `TOAdpWords3`.
    TOAdpWords3 @ 0x008e_fcd0 => extern "C" fn toadpwords3(p: u32, a: u32, b: u32, c: u32) -> u32 {
        pkt::bridge::adp_words3(&mut mem(), p, a, b, c)
    }
    /// `TOAdpXYZOffset`.
    TOAdpXYZOffset @ 0x008e_fc90 => extern "C" fn toadpxyzoffset(p: u32, x: u32, y: u32, z: u32) -> u32 {
        pkt::bridge::adp_xyz_offset(&mut mem(), p, x, y, z, None)
    }
    /// `TOAdpXYZWOffset`.
    TOAdpXYZWOffset @ 0x008e_fd00 => extern "C" fn toadpxyzwoffset(p: u32, x: u32, y: u32, z: u32, w: u32) -> u32 {
        pkt::bridge::adp_xyz_offset(&mut mem(), p, x, y, z, Some(w))
    }
    /// `Virtuos::InputManager::isHoldButton(mask)` (`this` in ECX).
    Virtuos_InputManager_isHoldButton @ 0x0063_1340 => extern "thiscall" fn is_hold_button(this: u32, mask: u32) -> u32 {
        pad::bridge::is_hold_button(&mem(), this, mask)
    }
    /// `Virtuos::InputManager::isPressButton(mask)` (`this` in ECX).
    Virtuos_InputManager_isPressButton @ 0x0063_1360 => extern "thiscall" fn is_press_button(this: u32, mask: u32) -> bool {
        pad::bridge::is_press_button(&mem(), this, mask)
    }
    /// `Virtuos_InputManager_isReleaseButton(mask)` (`this` in ECX).
    Virtuos_InputManager_isReleaseButton @ 0x0063_1380 => extern "thiscall" fn is_release_button(this: u32, mask: u32) -> bool {
        pad::bridge::is_release_button(&mem(), this, mask)
    }
    /// `TOSetPair_008bdfe0(prev, cur)`.
    TOSetPair_008bdfe0 @ 0x008b_dfe0 => extern "C" fn to_set_pair(prev: u32, cur: u32) -> u32 {
        window::bridge::set_pair(&mut mem(), prev, cur)
    }
    /// `TOClearEntry_008ff140(i)`.
    TOClearEntry_008ff140 @ 0x008f_f140 => extern "C" fn to_clear_entry(i: i32) -> u32 {
        window::bridge::clear_entry_18762e0(&mut mem(), i)
    }
    /// `TOSetScreenWH(w, h)`.
    TOSetScreenWH @ 0x0090_48f0 => extern "C" fn to_set_screen_wh(w: u32, h: u32) {
        window::bridge::set_screen_wh(&mut mem(), w, h);
    }
    /// `TOAdpTEX0(p, TBP0, TBW, PSM, TW, TH, TCC, TFX, CBP, CPSM, CSM, CSA, CLD)`.
    TOAdpTEX0 @ 0x008e_fa00 => extern "C" fn toadptex0(p: u32, f0: i32, f1: i32, f2: i32, f3: i32, f4: i32, f5: i32, f6: i32, f7: i32, f8: i32, f9: i32, f10: i32, f11: i32) -> u32 {
        pkt::bridge::adp_reg_at(&mut mem(), 0x008e_fa00, p, &[f0, f1, f2, f3, f4, f5, f6, f7, f8, f9, f10, f11])
    }
    /// `Fixup_WriteLeb128(value, &cursor)`.
    Fixup_WriteLeb128 @ 0x0060_aba0 => extern "C" fn fixup_write_leb128(value: u32, cursor: u32) {
        fixup::bridge::write_leb128(&mut mem(), value, cursor);
    }
    /// `TrophyManager::Install(arg)`: 0.
    TrophyManager_Install @ 0x0059_e520 => extern "thiscall" fn trophy_manager_install(_this: u32, _arg: u32) -> u32 {
        stub::TROPHY_INSTALL
    }
    /// `std::map` iterator `++` (fastcall).
    std_Tree_const_iterator_inc_006352b0 @ 0x0063_52b0 => extern "fastcall" fn tree_iterator_inc_006352b0(it: u32) -> u32 {
        stl::bridge::tree_iterator_inc(&mut mem(), it)
    }
    /// `std::map` iterator `++` (fastcall).
    std_Tree_const_iterator_inc_00635300 @ 0x0063_5300 => extern "fastcall" fn tree_iterator_inc_00635300(it: u32) -> u32 {
        stl::bridge::tree_iterator_inc(&mut mem(), it)
    }
    /// `std::map` iterator `++` (fastcall).
    std_Tree_const_iterator_inc_00635350 @ 0x0063_5350 => extern "fastcall" fn tree_iterator_inc_00635350(it: u32) -> u32 {
        stl::bridge::tree_iterator_inc(&mut mem(), it)
    }
    /// `std::map` iterator `++` (fastcall).
    std_Tree_const_iterator_inc_00699ee0 @ 0x0069_9ee0 => extern "fastcall" fn tree_iterator_inc_00699ee0(it: u32) -> u32 {
        stl::bridge::tree_iterator_inc(&mut mem(), it)
    }
    /// `TODrawScissorXYWH(x, y, w, h)`.
    TODrawScissorXYWH @ 0x008f_9230 => extern "C" fn to_draw_scissor_xywh(x: i32, y: i32, w: i32, h: i32) -> u32 {
        pkt::bridge::draw_scissor_xywh(&mut mem(), x, y, w, h)
    }
    /// `Kernel_GetItemName(id)`.
    Kernel_GetItemName @ 0x0077_67a0 => extern "C" fn kernel_get_item_name(id: u32) -> u32 {
        names::bridge::item_name(&mem(), id)
    }
}

/// `TOMenuGetGetImportant*`: the key item's text at `field`, with the
/// callees in `ffx-battle` (`MsImportantPtr` stores the strings base into a
/// local, as the original's).
fn important_text(id: u32, field: u32) -> u32 {
    let hiragana = ffx_battle::save::bridge::ms_get_save_config_hiragana(&mem());
    let mut strings = 0u32;
    #[allow(clippy::cast_possible_truncation)] // 32-bit process
    let out = std::ptr::from_mut(&mut strings) as usize as u32;
    let record = ffx_battle::save::bridge::ms_important_ptr(&mut mem(), id, out);
    let strings = mem().read::<u32>(out);
    menu::bridge::important_text(&mem(), field, hiragana, record, strings)
}
