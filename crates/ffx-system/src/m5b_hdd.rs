//! M5 `system-b` (round 2): the PS2 HDD-install layer's small pieces
//! (`hdd_*`, the install-menu natives' thunks), the CD layer's helpers,
//! and a few one-jump thunks the native and menu tables point at.
//!
//! Most of the natives here are `jmp` thunks the dump has no function
//! for: each port calls the target with the words the target reads.

/// Thunk targets (cdecl): `AtelGetScenarioFlag`, `AtelGetSphereMonitor`,
/// `hdd_rmode_get(i)`, `hdd_head_sectors(i)`, `SetTableEntry_00779590(i,
/// v)`, `MsPayGIL(n)`, `get_hdd_status`, `FFXSetLang(lang)`,
/// `SaveDataIsAlbhedLoaded`, `SaveDataIsBusy`.
pub const SCENARIO_FLAG: u32 = 0x0086_c470;
pub const SPHERE_MONITOR: u32 = 0x0086_c4b0;
pub const RMODE_GET: u32 = 0x0077_9570;
pub const HEAD_SECTORS: u32 = 0x0077_8960;
pub const SET_RMODE: u32 = 0x0077_9590;
pub const PAY_GIL: u32 = 0x0078_59a0;
pub const HDD_STATUS: u32 = 0x0077_7e90;
pub const SET_LANG: u32 = 0x0088_7d20;
pub const ALBHED_LOADED: u32 = 0x0064_8020;
pub const SAVE_BUSY: u32 = 0x0064_8040;

/// The pad state bytes the install menu reads: buttons `0x0112a08c`
/// (bits 5 and 6 read alone), the held word `0x0112a094`, the "pad on"
/// byte `0x0112a039`, and the word at `0x0112a078`.
pub const PAD_BUTTONS: u32 = 0x0112_a08c;
pub const PAD_HELD: u32 = 0x0112_a094;
pub const PAD_ON: u32 = 0x0112_a039;
pub const PAD_WORD_78: u32 = 0x0112_a078;

/// `g_FFXHddMode`, `hdd_mount`, `TkRefreshHDDMode` (cdecl).
pub const HDD_MODE: u32 = 0x0133_c913;
pub const HDD_MOUNT: u32 = 0x0077_9490;
pub const REFRESH_HDD_MODE: u32 = 0x0088_e930;
/// `TkGetFFXCofingAdrs`.
pub const CONFIG_ADDRESS: u32 = 0x0088_d5e0;
/// `TkyiAssert`'s switch `0x0112a520` and its target (a bare `ret`).
pub const ASSERT_OFF: u32 = 0x0112_a520;
pub const ASSERT_TARGET: u32 = 0x0088_ec80;
/// `0x00778980`'s two messages.
pub const HEADS_MSG: [u32; 2] = [0x00b5_380c, 0x00b5_3820];
/// `cd_module_file_sector(module, n)`, `CdQueueSectorRead_0076ce40(sector,
/// module, n)`, `ReturnAddress_00777230()` (cdecl).
pub const MODULE_FILE_SECTOR: u32 = 0x0076_c270;
pub const QUEUE_SECTOR_READ: u32 = 0x0076_ce40;
pub const RETURN_ADDRESS: u32 = 0x0077_7230;

/// `g_Hdd_Mounted`, `umount`, `dbgPrintf` and `hdd_unmount`'s strings
/// (the three asserts, the partition, the two messages).
pub const HDD_MOUNTED: u32 = 0x0112_a508;
pub const UMOUNT: u32 = 0x0062_fdf0;
pub const DBG_PRINTF: u32 = 0x0062_f500;
pub const UNMOUNT_ASSERT: [u32; 3] = [0x00b5_3664, 0x00b5_3674, 0x00b5_3698];
pub const PARTITION: u32 = 0x00b5_3618;
pub const UNMOUNT_OK_MSG: u32 = 0x00b5_3688;
pub const UNMOUNT_FAIL_MSG: u32 = 0x00b5_36ac;

/// `g_Hdd_Heads` (+4: the size in bytes) and the head count
/// `0x0112a51c`.
pub const HEADS: u32 = 0x0112_a528;
pub const HEAD_COUNT: u32 = 0x0112_a51c;

/// The install thread's state: the started byte `0x0112a50b`, `g_Hdd_Err`,
/// the bytes done `0x0112a7ac` and to do `0x0112a524`, the byte
/// `0x0112a7fc`, the two buffers `0x0112a50c`/`0x0112a510`;
/// `Sg_MemSetAlign`, `user_malloc`, `SetGlobal_0112a018`,
/// `TkRegistBGThreadFunc`, `TkSignalSemaBGThread`, the progress
/// callback `0x007797a0` and the thread `hdd_thread_main`.
pub const INSTALL_STATE: u32 = 0x0112_a50b;
pub const HDD_ERR: u32 = 0x0112_a50a;
pub const DONE: u32 = 0x0112_a7ac;
pub const TOTAL: u32 = 0x0112_a524;
pub const BYTE_7FC: u32 = 0x0112_a7fc;
pub const BUFFERS: [u32; 2] = [0x0112_a50c, 0x0112_a510];
pub const MEM_SET_ALIGN: u32 = 0x0083_c4a0;
pub const USER_MALLOC: u32 = 0x0068_7020;
pub const SET_GLOBAL_A018: u32 = 0x0077_5810;
pub const REGIST_BG_THREAD: u32 = 0x0088_8740;
pub const SIGNAL_BG_THREAD: u32 = 0x0088_8820;
pub const PROGRESS_CALLBACK: u32 = 0x0077_97a0;
pub const HDD_THREAD_MAIN: u32 = 0x0077_95b0;
/// `hdd_install_end`: its message, `disk_open_check_set`,
/// `read_thread_init`.
pub const INSTALL_END_MSG: u32 = 0x00b5_3b94;
pub const DISK_OPEN_CHECK_SET: u32 = 0x0076_d6b0;
pub const READ_THREAD_INIT: u32 = 0x0076_e630;
/// The word at `0x0112a518`.
pub const WORD_A518: u32 = 0x0112_a518;
/// `0x00779450`'s last progress `0x02310c1c`, the menu work pointer
/// `0x00c416cc`, the bar drawer `0x00779ad0` and the background
/// `0x007766e0`.
pub const LAST_PROGRESS: u32 = 0x0231_0c1c;
pub const MENU_WORK: u32 = 0x00c4_16cc;
pub const DRAW_BAR: u32 = 0x0077_9ad0;
/// `0x007789a0`: its messages and `sceFormat` (cdecl).
pub const FORMAT_MSG: u32 = 0x00b5_3ba8;
pub const FORMAT_DEVICE: u32 = 0x00b5_3b74;
pub const FORMAT_FAIL_MSG: u32 = 0x00b5_3bb8;
pub const SCE_FORMAT: u32 = 0x0062_f780;
/// `TrophyManager`'s instance cell and `user_free_thunk`.
pub const TROPHY_MANAGER: u32 = 0x0113_63b4;
pub const USER_FREE: u32 = 0x0068_7010;
/// `hd_file_read_no_wait`: `sceRead` (cdecl); the heads' size word.
pub const SCE_READ: u32 = 0x0062_fc00;
/// `0x0076d5a0`: `0x00777310`, `sceCdDiskReady`, `0x00a447a0`, its
/// message, `CdReadDirect_0076d140` and the pending read `0x01128acc`
/// (sector, size, buffer) with its mode block `0x01128aa8`.
pub const DISK_CHANGED: u32 = 0x0077_7310;
pub const DISK_READY: u32 = 0x0062_f550;
pub const CD_RESET: u32 = 0x00a4_47a0;
pub const RETRY_MSG: u32 = 0x00b5_1ecc;
pub const READ_DIRECT: u32 = 0x0076_d140;
pub const PENDING_READ: u32 = 0x0112_8acc;
pub const READ_MODE: u32 = 0x0112_8aa8;
/// `0x0076dc70`: its message, the buffer cell `0x01128ae4`, the word
/// `0x01128ab8`, `crt_memset_thunk`.
pub const BUFFER_MSG: u32 = 0x00b5_1d18;
pub const READ_BUFFER: u32 = 0x0112_8ae4;
pub const WORD_8AB8: u32 = 0x0112_8ab8;
pub const MEMSET: u32 = 0x0094_964c;
/// The install screens' sprite: `Sg_PacketGetPtr`, `sceDmaGetChan`,
/// `GsPkt_TexturedSprite_00774ff0` (7 words), `FlushCache`, `sceDmaSend`,
/// `sceGsSyncPath` (cdecl).
pub const PACKET_PTR: u32 = 0x0083_b650;
pub const DMA_GET_CHAN: u32 = 0x0062_f6e0;
pub const TEXTURED_SPRITE: u32 = 0x0077_4ff0;
pub const FLUSH_CACHE: u32 = 0x0062_f4c0;
pub const DMA_SEND: u32 = 0x0062_f710;
pub const GS_SYNC_PATH: u32 = 0x0062_f880;

/// Round 3: the HDD reader's requests. `get_hdd_name`, `hd_file_read_th`'s
/// two messages; `sceOpen`, `sceLseek64`, `sceLseek`, `sceClose`, the
/// descriptor cell `0x02310c9c`, `hd_file_read_th_main`'s two messages;
/// `sceDevctl` (`0x0062f6d0`) and `FUN_00777610`'s three messages.
pub const HDD_NAME: u32 = 0x0077_7eb0;
pub const READ_TH_MSG: [u32; 2] = [0x00b5_36c0, 0x00b5_36d8];
pub const SCE_OPEN: u32 = 0x0062_fa30;
pub const SCE_LSEEK64: u32 = 0x0062_f8f0;
pub const SCE_LSEEK: u32 = 0x0062_f8e0;
pub const SCE_CLOSE: u32 = 0x0062_f610;
pub const READ_FD: u32 = 0x0231_0c9c;
pub const READ_TH_MAIN_MSG: [u32; 2] = [0x00b5_3700, 0x00b5_3718];
pub const SCE_DEVCTL: u32 = 0x0062_f6d0;
pub const FREE_SPACE_MSG: [u32; 3] = [0x00b5_3aec, 0x00b5_3af8, 0x00b5_3b04];
/// `FUN_0076cff0`: the CD flag `0x02310c58`, `PcReadFunc_set`, and the
/// file layer's function table entries (cell, function) for the CD and
/// for both.
pub const READ_FUNCS_CD: u32 = 0x0231_0c58;
pub const PC_READ_FUNC_SET: u32 = 0x00a4_2ba0;
pub const READ_FUNCS_CD_TABLE: [(u32, u32); 9] = [
    (0x0231_0c54, 0x0076_c320),
    (0x0231_0c98, 0x0076_b960),
    (0x0231_0ca8, 0x0076_e780),
    (0x0231_0c94, 0x0076_b940),
    (0x0231_0ca0, 0x0076_b920),
    (0x0231_0c44, 0x0076_c250),
    (0x0231_0cb8, 0x0076_bdd0),
    (0x0231_0cc8, 0x0076_cf60),
    (0x0231_0ccc, 0x0076_ce20),
];
pub const READ_FUNCS_SHARED: [(u32, u32); 11] = [
    (0x0231_0c48, 0x0076_c490),
    (0x0231_0c5c, 0x0076_e130),
    (0x0231_0c40, 0x0076_b970),
    (0x0231_0cd0, 0x0076_bf70),
    (0x0231_0ca4, 0x0076_dcb0),
    (0x0231_0c4c, 0x0076_cff0),
    (0x0231_0c50, 0x0076_c510),
    (0x0231_0cbc, 0x0076_cfa0),
    (0x0231_0cc4, 0x0076_cf80),
    (0x0231_0cc0, 0x0076_d0e0),
    (0x0231_0cac, 0x0076_c270),
];

/// Round 3, batch 4: the disk-change result `0x0112a4ec`; the sector
/// stream (`0x0112a4b4` sector, `0x0112a4d8` bytes used, the read mode
/// `0x00c41748`, `CdReadSectors_00777080`); `hdd_ReadModuleFiles`; the
/// install wall (`0x02310c18` mark, `0x02310c14` frame, its draw
/// `0x007798b0`, message, `hdd_DrawWallSprite_00776740`).
pub const DISK_RESULT: u32 = 0x0112_a4ec;
pub const STREAM_SECTOR: u32 = 0x0112_a4b4;
pub const STREAM_USED: u32 = 0x0112_a4d8;
pub const STREAM_MODE: u32 = 0x00c4_1748;
pub const READ_SECTORS: u32 = 0x0077_7080;
pub const READ_MODULE_FILES: u32 = 0x0077_7f00;
pub const WALL_MARK: u32 = 0x0231_0c18;
pub const WALL_FRAME: u32 = 0x0231_0c14;
pub const WALL_DRAW: u32 = 0x0077_98b0;
pub const WALL_MSG: u32 = 0x00b5_3d84;
pub const WALL_SPRITE: u32 = 0x0077_6740;

/// `hdd_install_init`'s: the saved mode byte `0x0112a808`, `0x00778980`,
/// the word `0x0112a514`, its message; `hdd_mount`'s: `0x00779a80`, the
/// device `0x00c41758` and its partition name `0x00779ca0`, the CRT's
/// `sprintf` (import slot), `0x00777940`, the message call `0x00777680`,
/// "pfs0:", the mount `0x0062fa20`, its six messages.
pub const HDD_MODE_SAVE: u32 = 0x0112_a808;
pub const INSTALL_KIND: u32 = 0x0077_8980;
pub const WORD_A514: u32 = 0x0112_a514;
pub const INSTALL_MODE_MSG: u32 = 0x00b5_3b7c;
pub const MOUNT_PREPARE: u32 = 0x0077_9a80;
pub const MOUNT_DEVICE: u32 = 0x00c4_1758;
pub const PARTITION_NAME: u32 = 0x0077_9ca0;
pub const SPRINTF_IAT: u32 = 0x00b0_c3dc;
pub const MOUNT_PRE2: u32 = 0x0077_7940;
pub const MOUNT_LOG: u32 = 0x0077_7680;
pub const PFS: u32 = 0x00b5_3618;
pub const SCE_MOUNT: u32 = 0x0062_fa20;
pub const MOUNT_MSG: [u32; 6] = [
    0x00b5_35f0,
    0x00b5_35fc,
    0x00b5_3608,
    0x00b5_3620,
    0x00b5_3634,
    0x00b5_3650,
];

/// `file_offset_get`'s: `sceCdSearchFile`, `TkSyncV`, the three offsets
/// (`0x01128ab4`, `0x01128a98`/`9c`, `0x01128aa0`/`a4`).
pub const CD_SEARCH_FILE: u32 = 0x00a4_47d0;
pub const SYNC_V: u32 = 0x0088_7f00;
pub const FILE_OFFSETS: [u32; 5] = [
    0x0112_8ab4,
    0x0112_8a98,
    0x0112_8a9c,
    0x0112_8aa0,
    0x0112_8aa4,
];
/// `hdd_install_check`'s: the byte `0x0112a509`, `hdd_unmount`,
/// `hdd_ins_check`, the byte `0x01128a86`, `TkRefreshHDDMode`, its three
/// messages; `0x007787c0`'s: `hdd_rmode_get`, its messages.
pub const INSTALL_RESULT: u32 = 0x0112_a509;
pub const HDD_UNMOUNT: u32 = 0x0077_9730;
pub const INS_CHECK: u32 = 0x0077_8d70;
pub const BYTE_8A86: u32 = 0x0112_8a86;
pub const VERIFY_FILES: u32 = 0x0077_87c0;
pub const INSTALL_CHECK_MSG: [u32; 3] = [0x00b5_3c4c, 0x00b5_3c64, 0x00b5_3c78];
pub const VERIFY_MSG: [u32; 7] = [
    0x00b5_3bd0,
    0x00b5_3a70,
    0x00b5_3be4,
    0x00b5_3c00,
    0x00b5_3c10,
    0x00b5_3c1c,
    0x00b5_3c38,
];

/// `hdd_OpenFfxPartition`'s: the partition "ffxpassr" (`0x00c41764`),
/// its name format, `sceIoctl2`, the close `0x0062f630`,
/// `hdd_InstalledMb2_00778fb0`, the free-space check `0x00777610` and its
/// device, the five messages.
pub const FFX_PARTITION: u32 = 0x00c4_1764;
pub const PARTITION_FMT: u32 = 0x00b5_3b10;
pub const SCE_IOCTL2: u32 = 0x0062_f8b0;
pub const SCE_DCLOSE: u32 = 0x0062_f630;
pub const INSTALLED_MB: u32 = 0x0077_8fb0;
pub const FREE_SPACE_OK: u32 = 0x0077_7610;
pub const FREE_SPACE_DEVICE: u32 = 0x00b5_3b74;
pub const OPEN_PARTITION_MSG: [u32; 5] = [
    0x00b5_3b1c,
    0x00b5_3b30,
    0x00b5_3b48,
    0x00b5_3b5c,
    0x00b5_3b6c,
];
/// `FUN_00779ad0`'s: the help tables `0x0112a048` and picture buffer
/// `0x0112a030`, `crt_memset_thunk`, `sceGsSetDefLoadImage`,
/// `sceGsExecLoadImage`.
pub const HELP_TABLE: u32 = 0x0112_a048;
pub const HELP_PICTURE: u32 = 0x0112_a030;
pub const SET_LOAD_IMAGE: u32 = 0x0062_f820;
pub const EXEC_LOAD_IMAGE: u32 = 0x0062_f7a0;

/// `hdd_thread_main`'s: `0x00778a10`, the read-function setter cell
/// `0x02310c4c` (called through), `0x00777980`, its device format,
/// `0x00779d20`, the status word `0x0112a520`, its four messages.
pub const INSTALL_SETUP: u32 = 0x0077_8a10;
pub const SET_READ_FUNCS_CELL: u32 = 0x0231_0c4c;
pub const INSTALL_COPY: u32 = 0x0077_7980;
pub const FULL_PARTITION_FMT: u32 = 0x00b5_38ac;
pub const INSTALL_FINISH: u32 = 0x0077_9d20;
pub const INSTALL_BUSY: u32 = 0x0112_a520;
pub const THREAD_MSG: [u32; 4] = [0x00b5_3d28, 0x00b5_3d3c, 0x00b5_3d5c, 0x00b5_3d74];
/// `FUN_0076ccb0`'s: `0x00a44810`, the disk flag `0x01128a87`, the check
/// byte `0x01128aac`, the expected header `0x0112a064`, the read buffer
/// cell `0x01128ac8` and mode `0x00c40e18`, `sceCdRead`,
/// `hdd_state_clear`, `cd_module_file_sector`, its five messages.
pub const CD_STATUS: u32 = 0x00a4_4810;
pub const DISK_FLAG: u32 = 0x0112_8a87;
pub const DISK_CHECK_ON: u32 = 0x0112_8aac;
pub const DISK_HEADER: u32 = 0x0112_a064;
pub const DISK_BUFFER: u32 = 0x0112_8ac8;
pub const DISK_READ_MODE: u32 = 0x00c4_0e18;
pub const CD_READ: u32 = 0x00a4_47c0;
pub const STATE_CLEAR: u32 = 0x0077_7420;
pub const TASK_WAIT_SYM: u32 = 0x0077_a4e0;
pub const DISK_WAIT_MSG: [u32; 5] = [
    0x00b5_1e08,
    0x00b5_1e1c,
    0x00b5_1e34,
    0x00b5_1e48,
    0x00b5_1e64,
];

/// `FUN_007798b0`'s: the install wall's frame sizes (`i16` widths
/// `0x00c4190c`, heights `0x00c41918`) and its pictures `0x0112a04c`.
pub const WALL_WIDTHS: u32 = 0x00c4_190c;
pub const WALL_HEIGHTS: u32 = 0x00c4_1918;
pub const WALL_PICTURES: u32 = 0x0112_a04c;

/// The originals over game memory.
pub mod bridge {
    use super::{
        ALBHED_LOADED, ASSERT_OFF, ASSERT_TARGET, BUFFER_MSG, BUFFERS, BYTE_7FC, CD_RESET,
        CONFIG_ADDRESS, DBG_PRINTF, DISK_CHANGED, DISK_OPEN_CHECK_SET, DISK_READY, DMA_GET_CHAN,
        DMA_SEND, DONE, DRAW_BAR, FLUSH_CACHE, FORMAT_DEVICE, FORMAT_FAIL_MSG, FORMAT_MSG,
        GS_SYNC_PATH, HDD_ERR, HDD_MODE, HDD_MOUNT, HDD_MOUNTED, HDD_STATUS, HDD_THREAD_MAIN,
        HEAD_COUNT, HEAD_SECTORS, HEADS, HEADS_MSG, INSTALL_END_MSG, INSTALL_STATE, LAST_PROGRESS,
        MEM_SET_ALIGN, MEMSET, MENU_WORK, MODULE_FILE_SECTOR, PACKET_PTR, PAD_BUTTONS, PAD_HELD,
        PAD_ON, PAD_WORD_78, PARTITION, PAY_GIL, PENDING_READ, PROGRESS_CALLBACK,
        QUEUE_SECTOR_READ, READ_BUFFER, READ_DIRECT, READ_MODE, READ_THREAD_INIT, REFRESH_HDD_MODE,
        REGIST_BG_THREAD, RETRY_MSG, RETURN_ADDRESS, RMODE_GET, SAVE_BUSY, SCE_FORMAT, SCE_READ,
        SCENARIO_FLAG, SET_GLOBAL_A018, SET_LANG, SET_RMODE, SIGNAL_BG_THREAD, SPHERE_MONITOR,
        TEXTURED_SPRITE, TOTAL, TROPHY_MANAGER, UMOUNT, UNMOUNT_ASSERT, UNMOUNT_FAIL_MSG,
        UNMOUNT_OK_MSG, USER_FREE, USER_MALLOC, WORD_8AB8, WORD_A518,
    };
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    fn global(h: &impl Host, sym: u32) -> u32 {
        h.read(h.rebase(sym))
    }

    fn global8(h: &impl Host, sym: u32) -> u8 {
        h.read(h.rebase(sym))
    }

    // ---- one-jump thunks (natives and menu table entries) ----

    /// `code_007726d0()`: `jmp AtelGetScenarioFlag`.
    pub fn scenario_flag(h: &mut impl Host) -> u32 {
        cdecl(h, SCENARIO_FLAG, &[])
    }

    /// `code_00772730()`: `AtelGetSphereMonitor`'s word, zero-extended.
    pub fn sphere_monitor(h: &mut impl Host) -> u32 {
        cdecl(h, SPHERE_MONITOR, &[]) & 0xffff
    }

    /// `FUN_007727f0(i)`: `jmp hdd_rmode_get`.
    pub fn rmode_get(h: &mut impl Host, i: u32) -> u32 {
        cdecl(h, RMODE_GET, &[i])
    }

    /// `FUN_00773530(i)`: `jmp hdd_head_sectors`.
    pub fn head_sectors(h: &mut impl Host, i: u32) -> u32 {
        cdecl(h, HEAD_SECTORS, &[i])
    }

    /// `code_00773940(i, v)`: `jmp SetTableEntry_00779590`.
    pub fn set_rmode(h: &mut impl Host, i: u32, v: u32) -> u32 {
        cdecl(h, SET_RMODE, &[i, v])
    }

    /// `code_00773990(n)`: `jmp MsPayGIL`.
    pub fn pay_gil(h: &mut impl Host, n: u32) -> u32 {
        cdecl(h, PAY_GIL, &[n])
    }

    /// `code_00773b80()`: `jmp get_hdd_status`.
    pub fn hdd_status(h: &mut impl Host) -> u32 {
        cdecl(h, HDD_STATUS, &[])
    }

    /// `code_00887ca0(lang)`: `jmp FFXSetLang`.
    pub fn set_lang(h: &mut impl Host, lang: u32) -> u32 {
        cdecl(h, SET_LANG, &[lang])
    }

    /// `code_00822540()`: `jmp SaveDataIsAlbhedLoaded`.
    pub fn albhed_loaded(h: &mut impl Host) -> u32 {
        cdecl(h, ALBHED_LOADED, &[])
    }

    /// `SaveDataIsBusy_thunk()`: `jmp SaveDataIsBusy`.
    pub fn save_busy(h: &mut impl Host) -> u32 {
        cdecl(h, SAVE_BUSY, &[])
    }

    // ---- pad bits ----

    /// `code_00772800()` / `code_00772970()`: bit `bit` of the buttons
    /// byte.
    pub fn pad_bit(h: &mut impl Host, bit: u32) -> u32 {
        (u32::from(global8(h, PAD_BUTTONS)) >> bit) & 1
    }

    /// `code_00772860()`: 1 when the pad is on and the buttons or the
    /// held word have one of bits 12..15 set.
    pub fn pad_any_shoulder(h: &mut impl Host) -> u32 {
        if global8(h, PAD_ON) == 0 {
            return 0;
        }
        let bits = global(h, PAD_HELD) | global(h, PAD_BUTTONS);
        u32::from(bits & 0xf000 != 0)
    }

    /// `code_007729d0()`: the word at `0x0112a078`.
    pub fn pad_word(h: &mut impl Host) -> u32 {
        global(h, PAD_WORD_78)
    }

    // ---- the HDD layer ----

    /// `FUN_00773c60(on)`: `g_FFXHddMode` = whether `on`; on, the
    /// partition mounted (`hdd_mount`); then a tail jump to
    /// `TkRefreshHDDMode` (its `eax`).
    pub fn set_hdd_mode(h: &mut impl Host, on: u32) -> u32 {
        let mode = h.rebase(HDD_MODE);
        h.write(mode, u8::from(on != 0));
        if on != 0 {
            cdecl(h, HDD_MOUNT, &[]);
        }
        cdecl(h, REFRESH_HDD_MODE, &[])
    }

    /// `hdd_head_init()`: `TkGetFFXCofingAdrs()`, then 0.
    pub fn head_init(h: &mut impl Host) -> u32 {
        cdecl(h, CONFIG_ADDRESS, &[]);
        0
    }

    /// `TkyiAssert(msg)`: unless `0x0112a520` is set, a tail jump to the
    /// bare `ret` at `0x0088ec80` with `msg`. `eax` untouched either way.
    pub fn assert(h: &mut impl Host, msg: u32) {
        if global(h, ASSERT_OFF) == 0 {
            cdecl(h, ASSERT_TARGET, &[msg]);
        }
    }

    /// `FUN_00778980()`: two `TkyiAssert`s, then 3.
    pub fn heads_assert(h: &mut impl Host) -> u32 {
        for msg in HEADS_MSG {
            let msg = h.rebase(msg);
            assert(h, msg);
        }
        3
    }

    /// `FUN_0076cf60(module, n)`: a read of the module's file `n` queued
    /// (`CdQueueSectorRead(cd_module_file_sector(module, n), module,
    /// n)`); its id.
    pub fn queue_module_read(h: &mut impl Host, module: u32, n: u32) -> u32 {
        let sector = cdecl(h, MODULE_FILE_SECTOR, &[module, n]);
        cdecl(h, QUEUE_SECTOR_READ, &[sector, module, n])
    }

    /// `FUN_0076cf80(n)`: the argument slot (`slot`) decremented; the
    /// block `ReturnAddress_00777230()` returns gets byte `+1` = whether
    /// `n - 1` is not 0. Returns the block.
    pub fn set_block_flag(h: &mut impl Host, n: u32, slot: u32) -> u32 {
        let p = cdecl(h, RETURN_ADDRESS, &[]);
        let n = n.wrapping_sub(1);
        h.write(slot, n);
        h.write(p.wrapping_add(1), u8::from(n != 0));
        p
    }

    /// `hdd_unmount()`: mounted, an assert, `umount("pfs0:")`; when that
    /// returns 0, another assert, the success message and unmounted (0);
    /// else the third assert and the failure message (its result). Not
    /// mounted: 1.
    pub fn unmount(h: &mut impl Host) -> u32 {
        if global8(h, HDD_MOUNTED) == 0 {
            return 1;
        }
        let a = h.rebase(UNMOUNT_ASSERT[0]);
        assert(h, a);
        let part = h.rebase(PARTITION);
        let r = cdecl(h, UMOUNT, &[part]);
        if r == 0 {
            let a = h.rebase(UNMOUNT_ASSERT[1]);
            assert(h, a);
            let msg = h.rebase(UNMOUNT_OK_MSG);
            cdecl(h, DBG_PRINTF, &[msg]);
            let at = h.rebase(HDD_MOUNTED);
            h.write(at, 0u8);
            return 0;
        }
        let a = h.rebase(UNMOUNT_ASSERT[2]);
        assert(h, a);
        let msg = h.rebase(UNMOUNT_FAIL_MSG);
        cdecl(h, DBG_PRINTF, &[msg]);
        r
    }

    /// `FUN_00778f60()` (`extra` 1) / `FUN_00778fb0()` (`extra` 2): the
    /// sizes (`g_Hdd_Heads` `+4`) of the heads `hdd_rmode_get` reports
    /// installed (the count signed), summed, in MB (`>> 20`, unsigned),
    /// rounded up to 128 plus `extra` blocks of 128.
    pub fn installed_mb(h: &mut impl Host, extra: u32) -> u32 {
        let mut sum = 0u32;
        let mut i = 0u32;
        while i.cast_signed() < global(h, HEAD_COUNT).cast_signed() {
            if cdecl(h, RMODE_GET, &[i]) != 0 {
                let at = h.rebase(HEADS).wrapping_add(4 + 12 * i);
                sum = sum.wrapping_add(h.read(at));
            }
            i = i.wrapping_add(1);
        }
        (((sum >> 20) + 0xff) >> 7).wrapping_add(extra) << 7
    }

    /// `code_00779120()`: unless the install started (`0x0112a50b`): the
    /// error, done and total counts and `0x0112a7fc` cleared, two
    /// `0x100800`-byte buffers (`Sg_MemSetAlign(0x40)` before each), the
    /// progress callback set and `hdd_thread_main` registered as the
    /// background thread, then a tail jump to `TkSignalSemaBGThread` (its
    /// `eax`). Started already: `eax` untouched (`None`).
    pub fn install_start(h: &mut impl Host) -> Option<u32> {
        if global8(h, INSTALL_STATE) != 0 {
            return None;
        }
        let at = h.rebase(HDD_ERR);
        h.write(at, 0u8);
        let at = h.rebase(DONE);
        h.write(at, 0u32);
        let at = h.rebase(TOTAL);
        h.write(at, 0u32);
        let at = h.rebase(BYTE_7FC);
        h.write(at, 0u8);
        for cell in BUFFERS {
            cdecl(h, MEM_SET_ALIGN, &[0x40]);
            let p = cdecl(h, USER_MALLOC, &[0x0010_0800]);
            let cell = h.rebase(cell);
            h.write(cell, p);
        }
        let cb = h.rebase(PROGRESS_CALLBACK);
        cdecl(h, SET_GLOBAL_A018, &[cb]);
        let thread = h.rebase(HDD_THREAD_MAIN);
        cdecl(h, REGIST_BG_THREAD, &[thread]);
        Some(cdecl(h, SIGNAL_BG_THREAD, &[]))
    }

    /// The progress as `done * scale / total` (signed, 0 without a
    /// total; the original's `idiv` faults on `i32::MIN / -1`, the port
    /// wraps).
    fn progress(h: &impl Host, scale: u32) -> u32 {
        let total = global(h, TOTAL);
        if total == 0 {
            return 0;
        }
        let done = global(h, DONE).wrapping_mul(scale).cast_signed();
        done.wrapping_div(total.cast_signed()).cast_unsigned()
    }

    /// `code_00779290()`: the install's percentage, at most 100 (signed).
    pub fn install_percent(h: &mut impl Host) -> u32 {
        let p = progress(h, 100);
        if p.cast_signed() < 100 { p } else { 100 }
    }

    /// `hdd_install_end()`: its message, `disk_open_check_set(0)`,
    /// `SetGlobal_0112a018(0)`, then a tail jump to `read_thread_init`.
    pub fn install_end(h: &mut impl Host) -> u32 {
        let msg = h.rebase(INSTALL_END_MSG);
        cdecl(h, DBG_PRINTF, &[msg]);
        cdecl(h, DISK_OPEN_CHECK_SET, &[0]);
        cdecl(h, SET_GLOBAL_A018, &[0]);
        cdecl(h, READ_THREAD_INIT, &[])
    }

    /// `code_007793a0()`: the word at `0x0112a518`.
    pub fn word_a518(h: &mut impl Host) -> u32 {
        global(h, WORD_A518)
    }

    /// `code_00779880()`: the install state byte as a result: 2 -> 1,
    /// 0xff -> 2, 0xfe -> 3, else 0.
    pub fn install_result(h: &mut impl Host) -> u32 {
        match global8(h, INSTALL_STATE) {
            2 => 1,
            0xff => 2,
            0xfe => 3,
            _ => 0,
        }
    }

    /// `FUN_00779450()`: the progress in 64ths; when it changed, stored and
    /// the bar drawn (`0x00779ad0(progress, menu work)`); then a tail jump
    /// to the background sprite `0x007766e0`.
    pub fn draw_progress(h: &mut impl Host) -> u32 {
        let p = progress(h, 64);
        let last = h.rebase(LAST_PROGRESS);
        if p != h.read::<u32>(last) {
            let work = global(h, MENU_WORK);
            h.write(last, p);
            cdecl(h, DRAW_BAR, &[p, work]);
        }
        draw_background(h)
    }

    /// `code_007789a0()`: a message, `sceFormat("hdd0:", 0, 0, 0)`; a
    /// negative result printed. `eax` = the result, or the message
    /// call's `eax` when it failed.
    pub fn format(h: &mut impl Host) -> u32 {
        let msg = h.rebase(FORMAT_MSG);
        cdecl(h, DBG_PRINTF, &[msg]);
        let dev = h.rebase(FORMAT_DEVICE);
        let r = cdecl(h, SCE_FORMAT, &[dev, 0, 0, 0]);
        if r.cast_signed() >= 0 {
            return r;
        }
        let msg = h.rebase(FORMAT_FAIL_MSG);
        cdecl(h, DBG_PRINTF, &[msg, r])
    }

    /// `TrophyManager_DestroyInstance()`: the instance freed and cleared
    /// when set; `eax` = the free's, or 0.
    pub fn trophy_destroy(h: &mut impl Host) -> u32 {
        let cell = h.rebase(TROPHY_MANAGER);
        let p: u32 = h.read(cell);
        if p == 0 {
            return 0;
        }
        let r = cdecl(h, USER_FREE, &[p]);
        h.write(cell, 0u32);
        r
    }

    /// `hd_file_read_no_wait(_, buf, size, head, file)`: when the head has
    /// a size, `sceRead(file, buf, size)`; 0.
    pub fn read_no_wait(h: &mut impl Host, buf: u32, size: u32, head: u32, file: u32) -> u32 {
        let at = h
            .rebase(HEADS)
            .wrapping_add(4)
            .wrapping_add(head.wrapping_mul(12));
        if h.read::<u32>(at) != 0 {
            cdecl(h, SCE_READ, &[file, buf, size]);
        }
        0
    }

    /// `FUN_0076d5a0()`: the disk state (`0x00777310`): 0 -> 0; -1 -> 1;
    /// else `sceCdDiskReady(1)`: ready (2) resets the drive
    /// (`0x00a447a0`), prints and reissues the pending read
    /// (`CdReadDirect(sector, size, buffer, mode block)`), 1; not ready -1.
    pub fn check_disk(h: &mut impl Host) -> u32 {
        let s = cdecl(h, DISK_CHANGED, &[]);
        if s == 0 {
            return 0;
        }
        if s != u32::MAX {
            if cdecl(h, DISK_READY, &[1]) != 2 {
                return u32::MAX;
            }
            cdecl(h, CD_RESET, &[]);
            let msg = h.rebase(RETRY_MSG);
            cdecl(h, DBG_PRINTF, &[msg]);
            let pend = h.rebase(PENDING_READ);
            let sector: u32 = h.read(pend);
            let size: u32 = h.read(pend.wrapping_add(4));
            let buf: u32 = h.read(pend.wrapping_add(8));
            let mode = h.rebase(READ_MODE);
            cdecl(h, READ_DIRECT, &[sector, size, buf, mode]);
        }
        1
    }

    /// `FUN_0076dc70()`: `0x01128ab8` = 0, a message, a cleared
    /// `0x6800`-byte buffer into `0x01128ae4`; `memset`'s `eax`.
    pub fn alloc_read_buffer(h: &mut impl Host) -> u32 {
        let at = h.rebase(WORD_8AB8);
        h.write(at, 0u32);
        let msg = h.rebase(BUFFER_MSG);
        cdecl(h, DBG_PRINTF, &[msg, 0x6800]);
        let p = cdecl(h, USER_MALLOC, &[0x6800]);
        let cell = h.rebase(READ_BUFFER);
        h.write(cell, p);
        cdecl(h, MEMSET, &[p, 0, 0x6800])
    }

    /// A textured sprite sent on DMA channel 2: the packet, the two
    /// texture words, then position and size.
    fn sprite(h: &mut impl Host, tex: [u32; 2], rect: [u32; 4]) -> u32 {
        let pkt = cdecl(h, PACKET_PTR, &[]);
        let chan = cdecl(h, DMA_GET_CHAN, &[2]);
        let [left, top, width, height] = rect;
        cdecl(
            h,
            TEXTURED_SPRITE,
            &[pkt, tex[0], tex[1], left, top, width, height],
        );
        cdecl(h, FLUSH_CACHE, &[0]);
        cdecl(h, DMA_SEND, &[chan, pkt]);
        cdecl(h, GS_SYNC_PATH, &[0, 0])
    }

    /// `FUN_007766e0()`: the progress bar's sprite (texture words
    /// `work + 4`, `work`; at 0x170, 0xf6, 0x80 square).
    pub fn draw_background(h: &mut impl Host) -> u32 {
        let work = global(h, MENU_WORK);
        sprite(h, [work.wrapping_add(4), work], [0x170, 0xf6, 0x80, 0x80])
    }

    /// `FUN_00776740()`: the wall sprite (`work + 0x48`, `work + 0x44`; at
    /// 0, 0, 0x200 square).
    pub fn draw_wall(h: &mut impl Host) -> u32 {
        let work = global(h, MENU_WORK);
        sprite(
            h,
            [work.wrapping_add(0x48), work.wrapping_add(0x44)],
            [0, 0, 0x200, 0x200],
        )
    }
    // ---- round 3: the HDD reader's requests ----

    /// The request block (`0x0112a7b0`: the path, 0x40 bytes; `+0x40` the
    /// buffer, `+0x44` the size, `+0x48` the offset), the background
    /// thread's state byte `0x01128aad` (3 = read pending) and its wait
    /// `task_wait(1)`.
    const REQUEST_PATH: u32 = 0x0112_a7b0;
    const REQUEST_BUFFER: u32 = 0x0112_a7f0;
    const REQUEST_SIZE: u32 = 0x0112_a7f4;
    const REQUEST_OFFSET: u32 = 0x0112_a7f8;
    const BG_STATE: u32 = 0x0112_8aad;
    const TASK_WAIT: u32 = 0x0077_a4e0;

    /// Copies the string at `s` with its NUL to `dest`.
    fn strcpy(h: &mut impl Host, dest: u32, s: u32) {
        let mut i = 0u32;
        loop {
            let b: u8 = h.read(s.wrapping_add(i));
            h.write(dest.wrapping_add(i), b);
            if b == 0 {
                break;
            }
            i = i.wrapping_add(1);
        }
    }

    /// `task_wait(1)` until the background thread's state is 0; `eax`
    /// stays `eax` when it already is.
    fn wait_idle(h: &mut impl Host, mut eax: u32) -> u32 {
        while h.read::<u8>(h.rebase(BG_STATE)) != 0 {
            eax = cdecl(h, TASK_WAIT, &[1]);
        }
        eax
    }

    /// Hands the request to the background thread and waits for it: idle,
    /// state 3, `TkSignalSemaBGThread()`, idle again. The `eax` left (the
    /// signal's or the last wait's).
    fn run_request(h: &mut impl Host) -> u32 {
        wait_idle(h, 0);
        let at = h.rebase(BG_STATE);
        h.write(at, 3u8);
        let r = cdecl(h, SIGNAL_BG_THREAD, &[]);
        wait_idle(h, r)
    }

    /// `hd_file_read_th(sector, buf, size, head)`: for an installed head
    /// (`+4` of its 12-byte entry set), the read of `buf`/`size` from the
    /// head's file (`get_hdd_name(head)`) at `(sector - its first
    /// sector) << 11`, logged (`dbgPrintf`), done by the background
    /// thread. Returns 0.
    pub fn read_th(h: &mut impl Host, sector: u32, buf: u32, size: u32, head: u32) -> u32 {
        let entry = h.rebase(HEADS).wrapping_add(head.wrapping_mul(12));
        if h.read::<u32>(entry.wrapping_add(4)) == 0 {
            return 0;
        }
        let name = cdecl(h, super::HDD_NAME, &[head]);
        let fmt = h.rebase(super::READ_TH_MSG[0]);
        cdecl(h, DBG_PRINTF, &[fmt, name]);
        let first: u32 = h.read(entry);
        let offset = sector.wrapping_sub(first) << 11;
        let at = h.rebase(REQUEST_OFFSET);
        h.write(at, offset);
        let fmt = h.rebase(super::READ_TH_MSG[1]);
        cdecl(h, DBG_PRINTF, &[fmt, sector, buf, size, offset]);
        let path = h.rebase(REQUEST_PATH);
        strcpy(h, path, name);
        let at = h.rebase(REQUEST_BUFFER);
        h.write(at, buf);
        let at = h.rebase(REQUEST_SIZE);
        h.write(at, size);
        run_request(h);
        0
    }

    /// `hd_file_read_ex_th_main(path, buf, size, offset)`: the request
    /// (path copied) done by the background thread; the `eax` left (the
    /// signal's or the last wait's).
    pub fn read_ex_th(h: &mut impl Host, path: u32, buf: u32, size: u32, offset: u32) -> u32 {
        let dest = h.rebase(REQUEST_PATH);
        strcpy(h, dest, path);
        for (sym, v) in [
            (REQUEST_BUFFER, buf),
            (REQUEST_SIZE, size),
            (REQUEST_OFFSET, offset),
        ] {
            let at = h.rebase(sym);
            h.write(at, v);
        }
        run_request(h)
    }

    /// `hd_file_read_th_main()` (the background thread's read): the
    /// request's file opened (`sceOpen(path, 1)`, the descriptor in
    /// `0x02310c9c`), sought (`sceLseek64` from 2 GB up, else `sceLseek`),
    /// read and closed (the cell -1); an error sets `g_Hdd_Err` (an open
    /// error returns there, `dbgPrintf`'s `eax`). Else `sceClose`'s.
    pub fn read_th_main(h: &mut impl Host) -> u32 {
        let size = global(h, REQUEST_SIZE);
        let offset = global(h, REQUEST_OFFSET);
        let buf = global(h, REQUEST_BUFFER);
        let path = h.rebase(REQUEST_PATH);
        let fd = cdecl(h, super::SCE_OPEN, &[path, 1]);
        let cell = h.rebase(super::READ_FD);
        h.write(cell, fd);
        let err = h.rebase(HDD_ERR);
        if fd.cast_signed() < 0 {
            let fmt = h.rebase(super::READ_TH_MAIN_MSG[0]);
            let r = cdecl(h, DBG_PRINTF, &[fmt, path]);
            h.write(err, 1u8);
            return r;
        }
        let seek = if offset >= 0x8000_0000 {
            super::SCE_LSEEK64
        } else {
            super::SCE_LSEEK
        };
        if cdecl(h, seek, &[fd, offset, 0]).cast_signed() < 0 {
            let fmt = h.rebase(super::READ_TH_MAIN_MSG[1]);
            cdecl(h, DBG_PRINTF, &[fmt, path]);
            h.write(err, 1u8);
        } else {
            let fd: u32 = h.read(cell);
            if cdecl(h, SCE_READ, &[fd, buf, size]).cast_signed() < 0 {
                h.write(err, 1u8);
            }
        }
        let fd: u32 = h.read(cell);
        let r = cdecl(h, super::SCE_CLOSE, &[fd]);
        h.write(cell, u32::MAX);
        r
    }

    /// `FUN_00777610(device, need)` with its frame at `ebp`: the device's
    /// free space (`sceDevctl(device, 0x480a, 0, 0, ebp - 4, 4)`; a
    /// failure returns 0) in megabytes (`free * 2 >> 19`, sectors), logged
    /// with `need`; 1 when it is at least `need` (unsigned).
    pub fn free_space_ok(h: &mut impl Host, ebp: u32, device: u32, need: u32) -> u32 {
        let out = ebp.wrapping_sub(4);
        if cdecl(h, super::SCE_DEVCTL, &[device, 0x480a, 0, 0, out, 4]) != 0 {
            return 0;
        }
        let free: u32 = h.read(out);
        let mb = free.wrapping_mul(2) >> 19;
        let [m0, m1, m2] = super::FREE_SPACE_MSG;
        let fmt = h.rebase(m0);
        cdecl(h, DBG_PRINTF, &[fmt, free]);
        let fmt = h.rebase(m1);
        cdecl(h, DBG_PRINTF, &[fmt, need]);
        let fmt = h.rebase(m2);
        cdecl(h, DBG_PRINTF, &[fmt, mb]);
        u32::from(mb >= need)
    }

    /// `FUN_0076cff0(cd)`: the file layer's function table (`0x02310c40`..)
    /// filled for the CD (`cd` set: nine CD entries) or the PC
    /// (`PcReadFunc_set()`), then the eleven shared entries; `cd` kept in
    /// `0x02310c58`. Returns `cd`, or `PcReadFunc_set`'s `eax`.
    pub fn set_read_funcs(h: &mut impl Host, cd: u32) -> u32 {
        let at = h.rebase(super::READ_FUNCS_CD);
        h.write(at, cd);
        let r = if cd != 0 {
            for (cell, f) in super::READ_FUNCS_CD_TABLE {
                let (cell, f) = (h.rebase(cell), h.rebase(f));
                h.write(cell, f);
            }
            cd
        } else {
            cdecl(h, super::PC_READ_FUNC_SET, &[])
        };
        for (cell, f) in super::READ_FUNCS_SHARED {
            let (cell, f) = (h.rebase(cell), h.rebase(f));
            h.write(cell, f);
        }
        r
    }
    // ---- round 3, batch 4: the disk checks, sector reads, the install wall ----

    /// `FUN_007775e0(ctx)`: with the context's flag `+0x5075d` set, the
    /// remembered result (`i8 0x0112a4ec`, sign-extended); else
    /// `0x00777310()`: 1 when it reports a change, else the result
    /// cleared and 0.
    pub fn disk_changed_cached(h: &mut impl Host, ctx: u32) -> u32 {
        let cell = h.rebase(super::DISK_RESULT);
        if h.read::<u8>(ctx.wrapping_add(0x5_075d)) != 0 {
            return i32::from(h.read::<i8>(cell)).cast_unsigned();
        }
        if cdecl(h, DISK_CHANGED, &[]) != 0 {
            return 1;
        }
        h.write(cell, 0u8);
        0
    }

    /// `FUN_00777380(offset, buf, &skip, ctx)`: a read from byte `offset`
    /// of the stream at sector `0x0112a4b4` (with `0x0112a4d8` bytes of
    /// its sector already used): `*skip` = the bytes left in that sector
    /// (or 0), the sectors `offset` spans (rounded up; the remainder
    /// kept in `0x0112a4d8`) read by `CdReadSectors_00777080(sector,
    /// count, buf, 0x00c41748, ctx)`, the stream moved on. Returns 1.
    pub fn stream_read(h: &mut impl Host, args: [u32; 4]) -> u32 {
        let [offset, buf, skip, ctx] = args;
        let used_at = h.rebase(super::STREAM_USED);
        let sector_at = h.rebase(super::STREAM_SECTOR);
        let used: u32 = h.read(used_at);
        let mut sector: u32 = h.read(sector_at);
        let mut bytes = offset;
        if used == 0 {
            h.write(skip, 0u32);
        } else {
            let left = 0x800u32.wrapping_sub(used);
            h.write(skip, left);
            bytes = offset.wrapping_add(left);
            sector = sector.wrapping_sub(1);
            h.write(sector_at, sector);
        }
        let signed = bytes.cast_signed();
        let mut count = signed.wrapping_div(0x800).cast_unsigned();
        let rest = signed.wrapping_rem(0x800);
        if rest == 0 {
            h.write(used_at, 0u32);
        } else {
            h.write(used_at, 0x800i32.wrapping_sub(rest).cast_unsigned());
            count = count.wrapping_add(1);
        }
        let mode = h.rebase(super::STREAM_MODE);
        cdecl(h, super::READ_SECTORS, &[sector, count, buf, mode, ctx]);
        let s: u32 = h.read(sector_at);
        h.write(sector_at, s.wrapping_add(count));
        1
    }

    /// `code_00777970()`: `jmp hdd_ReadModuleFiles_00777f00` (no stack
    /// words).
    pub fn read_module_files(h: &mut impl Host) -> u32 {
        cdecl(h, super::READ_MODULE_FILES, &[])
    }

    /// `hdd_kabe_disp()`: the install progress in 64ths (`0x0112a7ac << 6 /
    /// 0x0112a524`, signed, 0 without a total); when the next frame's
    /// mark (`0x02310c18`) is reached, the mark moves 14 on, frame
    /// `0x02310c14` of the wall is drawn (`0x007798b0(frame, work +
    /// 0x44)`) and counted (past 4 the mark goes to 9999), logged. Then
    /// the tail jump to `hdd_DrawWallSprite_00776740`.
    pub fn kabe_disp(h: &mut impl Host) -> u32 {
        let total = global(h, TOTAL);
        let progress = if total == 0 {
            0
        } else {
            (global(h, DONE) << 6)
                .cast_signed()
                .wrapping_div(total.cast_signed())
                .cast_unsigned()
        };
        let mark_at = h.rebase(super::WALL_MARK);
        let mark: u32 = h.read(mark_at);
        if mark == progress {
            let work = global(h, MENU_WORK);
            let frame_at = h.rebase(super::WALL_FRAME);
            let frame: u32 = h.read(frame_at);
            h.write(mark_at, mark.wrapping_add(0xe));
            cdecl(h, super::WALL_DRAW, &[frame, work.wrapping_add(0x44)]);
            let frame = h.read::<u32>(frame_at).wrapping_add(1);
            h.write(frame_at, frame);
            let fmt = h.rebase(super::WALL_MSG);
            if frame.cast_signed() > 4 {
                h.write(mark_at, 9999u32);
                cdecl(h, DBG_PRINTF, &[fmt, 9999]);
            } else {
                let mark: u32 = h.read(mark_at);
                cdecl(h, DBG_PRINTF, &[fmt, mark]);
            }
        }
        cdecl(h, super::WALL_SPRITE, &[])
    }
    /// `hdd_install_init()`: the install state cleared (`0x0112a50b`,
    /// total, done, the mode `0x0112a518`; the HDD mode byte
    /// `0x0133c913` saved to `0x0112a808` and cleared meanwhile), the mode
    /// from `|0x00778980()| & 0xf`: 0 gives 5 when `0x0112a514` is
    /// negative, 1..3 themselves, others keep 0; logged (`dbgPrintf`) and
    /// the mode byte put back. Returns `dbgPrintf`'s `eax` with the byte
    /// in `al`.
    pub fn install_init(h: &mut impl Host) -> u32 {
        let mode_at = h.rebase(HDD_MODE);
        let saved = i32::from(h.read::<i8>(mode_at)).cast_unsigned();
        let save_at = h.rebase(super::HDD_MODE_SAVE);
        h.write(save_at, saved);
        h.write(mode_at, 0u8);
        let state = h.rebase(INSTALL_STATE);
        h.write(state, 0u8);
        for sym in [TOTAL, DONE, WORD_A518] {
            let at = h.rebase(sym);
            h.write(at, 0u32);
        }
        let r = cdecl(h, super::INSTALL_KIND, &[]);
        let kind = r.cast_signed().unsigned_abs() & 0xf;
        let set = match kind {
            0 if global(h, super::WORD_A514).cast_signed() < 0 => Some(5),
            1..=3 => Some(kind),
            _ => None,
        };
        let mode = match set {
            Some(m) => {
                let at = h.rebase(WORD_A518);
                h.write(at, m);
                m
            }
            None => global(h, WORD_A518),
        };
        let fmt = h.rebase(super::INSTALL_MODE_MSG);
        let eax = cdecl(h, DBG_PRINTF, &[fmt, mode]);
        let b: u8 = h.read(save_at);
        h.write(mode_at, b);
        (eax & !0xff) | u32::from(b)
    }

    /// `hdd_mount()` with its frame at `ebp`: once (`0x0112a508`), the HDD
    /// partition mounted: `0x00779a80()`, a log line, the device name
    /// `sprintf(ebp - 0x84, "hdd0:%s,%s", 0x00779ca0(0x00c41758), and the
    /// word the original leaves on its stack there, 0x00c41758)`,
    /// `0x00777940()`, messages through `0x00777680`, then
    /// `0x0062fa20("pfs0:", name, 0, 0, 0)`: negative logs the error and
    /// returns 1; else the flag set. Returns 0.
    pub fn mount(h: &mut impl Host, ebp: u32) -> u32 {
        let flag = h.rebase(HDD_MOUNTED);
        if h.read::<u8>(flag) != 0 {
            return 0;
        }
        cdecl(h, super::MOUNT_PREPARE, &[]);
        let [m0, m1, m2, m3, m4, m5] = super::MOUNT_MSG;
        let fmt = h.rebase(m0);
        cdecl(h, DBG_PRINTF, &[fmt]);
        let dev = h.rebase(super::MOUNT_DEVICE);
        let part = cdecl(h, super::PARTITION_NAME, &[dev]);
        let buf = ebp.wrapping_sub(0x84);
        let fmt = h.rebase(m1);
        let sprintf: u32 = h.read(h.rebase(super::SPRINTF_IAT));
        h.call(sprintf, Conv::Cdecl, &[buf, fmt, part, dev]);
        cdecl(h, super::MOUNT_PRE2, &[]);
        let msg = h.rebase(m2);
        cdecl(h, super::MOUNT_LOG, &[msg]);
        let pfs = h.rebase(super::PFS);
        let r = cdecl(h, super::SCE_MOUNT, &[pfs, buf, 0, 0, 0]);
        if r.cast_signed() < 0 {
            let msg = h.rebase(m3);
            cdecl(h, super::MOUNT_LOG, &[msg]);
            let fmt = h.rebase(m4);
            cdecl(h, DBG_PRINTF, &[fmt, r]);
            return 1;
        }
        let msg = h.rebase(m5);
        cdecl(h, super::MOUNT_LOG, &[msg]);
        h.write(flag, 1u8);
        0
    }

    /// Searches the CD for `name` into the entry at `entry` until found
    /// (`TkSyncV(1)` between tries).
    fn search_file(h: &mut impl Host, entry: u32, name: u32) {
        while cdecl(h, super::CD_SEARCH_FILE, &[entry, name]) == 0 {
            cdecl(h, super::SYNC_V, &[1]);
        }
    }

    /// `file_offset_get(names)` with its frame at `ebp`: the three files
    /// `names[0..3]` searched on the CD (`sceCdSearchFile` into `ebp -
    /// 0x28`, retried every frame); the first's sector to `0x01128ab4`,
    /// the others' first two words to `0x01128a98`/`9c` and
    /// `0x01128aa0`/`a4`. Returns the last word stored; with no names
    /// the original returns its cookie word (`None`).
    pub fn file_offset_get(h: &mut impl Host, ebp: u32, names: u32) -> Option<u32> {
        if names == 0 {
            return None;
        }
        let entry = ebp.wrapping_sub(0x28);
        let at = super::FILE_OFFSETS.map(|s| h.rebase(s));
        let name: u32 = h.read(names);
        search_file(h, entry, name);
        let v: u32 = h.read(entry);
        h.write(at[0], v);
        let mut last = 0;
        for (k, pair) in [[at[1], at[2]], [at[3], at[4]]].into_iter().enumerate() {
            let name: u32 = h.read(names.wrapping_add(4 + 4 * u32::from(k == 1)));
            search_file(h, entry, name);
            let lo: u32 = h.read(entry);
            h.write(pair[0], lo);
            last = h.read(entry.wrapping_add(4));
            h.write(pair[1], last);
        }
        Some(last)
    }

    /// `FUN_007787c0()`: each installed head's file (size `+4` set and
    /// `hdd_rmode_get(i)`) opened and its size (`sceLseek64(fd, 0, 2)`)
    /// compared with the head's, logged; returns the number of heads that
    /// failed to open or differ.
    pub fn verify_files(h: &mut impl Host) -> u32 {
        let [m0, m1, m2, m3, m4, m5, m6] = super::VERIFY_MSG;
        let msg = h.rebase(m0);
        cdecl(h, super::MOUNT_LOG, &[msg]);
        let fmt = h.rebase(m1);
        let count = global(h, HEAD_COUNT);
        cdecl(h, DBG_PRINTF, &[fmt, count]);
        let mut bad = 0u32;
        let mut i = 0u32;
        while i.cast_signed() < global(h, HEAD_COUNT).cast_signed() {
            let size: u32 = h.read(h.rebase(HEADS).wrapping_add(4 + 12 * i));
            if size != 0 && cdecl(h, RMODE_GET, &[i]) != 0 {
                let name = cdecl(h, super::HDD_NAME, &[i]);
                let fd = cdecl(h, super::SCE_OPEN, &[name, 1]);
                let fmt = h.rebase(m2);
                cdecl(h, DBG_PRINTF, &[fmt, name]);
                if fd.cast_signed() < 0 {
                    bad = bad.wrapping_add(1);
                    let fmt = h.rebase(m3);
                    cdecl(h, DBG_PRINTF, &[fmt, fd]);
                } else {
                    let end = cdecl(h, super::SCE_LSEEK64, &[fd, 0, 2]);
                    cdecl(h, super::SCE_CLOSE, &[fd]);
                    if size == end {
                        let fmt = h.rebase(m4);
                        cdecl(h, DBG_PRINTF, &[fmt]);
                    } else {
                        let fmt = h.rebase(m5);
                        cdecl(h, DBG_PRINTF, &[fmt, size, end]);
                        bad = bad.wrapping_add(1);
                    }
                }
            }
            i = i.wrapping_add(1);
        }
        let msg = h.rebase(m6);
        cdecl(h, super::MOUNT_LOG, &[msg]);
        bad
    }

    /// `hdd_install_check(arg)`: the install's state found: the result
    /// byte `0x0112a509` and `0x0112a514` cleared, the kind
    /// (`|0x00778980()| & 0xf`, logged): kind 0 mounts the HDD (failing:
    /// HDD mode off), checks the installed files (`0x007787c0` failing:
    /// result 2, mode off, `hdd_unmount`), else `hdd_ins_check(arg)`
    /// (result 1; with `0x01128a86` and the CD flag 1, HDD mode on and
    /// `TkRefreshHDDMode`); without heads the HDD is unmounted, the
    /// answer 0 and the mode off. Kinds 1..3 turn the mode off; others
    /// leave it. Both bytes logged; returns `hdd_ins_check`'s answer or
    /// 0.
    pub fn install_check(h: &mut impl Host, arg: u32) -> u32 {
        let a514 = h.rebase(super::WORD_A514);
        h.write(a514, 0u32);
        let result = h.rebase(super::INSTALL_RESULT);
        h.write(result, 0u8);
        let r = cdecl(h, super::INSTALL_KIND, &[]);
        let fmt = h.rebase(super::INSTALL_CHECK_MSG[0]);
        cdecl(h, DBG_PRINTF, &[fmt, r]);
        let kind = r.cast_signed().unsigned_abs() & 0xf;
        let mode = h.rebase(HDD_MODE);
        let mut answer = 0u32;
        if kind == 0 {
            if cdecl(h, HDD_MOUNT, &[]) != 0 {
                h.write(mode, 0u8);
            } else if cdecl(h, super::VERIFY_FILES, &[]) != 0 {
                h.write(result, 2u8);
                h.write(mode, 0u8);
                cdecl(h, super::HDD_UNMOUNT, &[]);
            } else {
                answer = cdecl(h, super::INS_CHECK, &[arg]);
                h.write(result, 1u8);
                let cd = global(h, super::READ_FUNCS_CD);
                if h.read::<u8>(h.rebase(super::BYTE_8A86)) != 0 && cd == 1 {
                    h.write(mode, 1u8);
                    cdecl(h, REFRESH_HDD_MODE, &[]);
                }
            }
            if global(h, HEAD_COUNT) == 0 {
                cdecl(h, super::HDD_UNMOUNT, &[]);
                answer = 0;
                h.write(mode, 0u8);
            }
        } else if kind <= 3 {
            h.write(mode, 0u8);
        }
        let fmt = h.rebase(super::INSTALL_CHECK_MSG[1]);
        let r = i32::from(h.read::<i8>(result)).cast_unsigned();
        cdecl(h, DBG_PRINTF, &[fmt, r]);
        let fmt = h.rebase(super::INSTALL_CHECK_MSG[2]);
        let m = i32::from(h.read::<i8>(mode)).cast_unsigned();
        cdecl(h, DBG_PRINTF, &[fmt, m, answer]);
        answer
    }
    /// `hdd_OpenFfxPartition_00779000()` with its frame at `ebp`: the
    /// HDD unmounted meanwhile when it was (and mounted again at the end),
    /// `0x0112a514` cleared; the "ffxpassr" partition's size: its device
    /// name (`sprintf(ebp - 0x44, "hdd0:%s,,%s", 0x00779ca0(partition),
    /// partition)`, the second word the one left on the stack), opened
    /// (`sceOpen(name, 1)`, failure logged), `sceIoctl2(fd, 0x6803, 0 x4)`
    /// (negative counts 0) and closed, plus 1; the space needed
    /// (`hdd_InstalledMb2() >> 7` less that) checked by `0x00777610` (no
    /// room: `0x0112a514` = -1). Returns the check's or the remount's
    /// `eax`.
    pub fn open_ffx_partition(h: &mut impl Host, ebp: u32) -> u32 {
        let mounted = h.read::<u8>(h.rebase(HDD_MOUNTED)) != 0;
        let a514 = h.rebase(super::WORD_A514);
        h.write(a514, 0u32);
        if mounted {
            cdecl(h, super::HDD_UNMOUNT, &[]);
        }
        cdecl(h, super::MOUNT_PREPARE, &[]);
        let part = h.rebase(super::FFX_PARTITION);
        let name = cdecl(h, super::PARTITION_NAME, &[part]);
        let buf = ebp.wrapping_sub(0x44);
        let fmt = h.rebase(super::PARTITION_FMT);
        let sprintf: u32 = h.read(h.rebase(super::SPRINTF_IAT));
        h.call(sprintf, Conv::Cdecl, &[buf, fmt, name, part]);
        let fd = cdecl(h, super::SCE_OPEN, &[buf, 1]);
        let [m0, m1, m2, m3, m4] = super::OPEN_PARTITION_MSG;
        let mut used = 0u32;
        if fd.cast_signed() < 0 {
            let msg = h.rebase(m1);
            cdecl(h, DBG_PRINTF, &[msg, buf]);
        } else {
            used = cdecl(h, super::SCE_IOCTL2, &[fd, 0x6803, 0, 0, 0, 0]);
            if used.cast_signed() < 0 {
                let msg = h.rebase(m0);
                cdecl(h, DBG_PRINTF, &[msg]);
                used = 0;
            }
            cdecl(h, super::SCE_DCLOSE, &[fd]);
            used = used.wrapping_add(1);
        }
        let msg = h.rebase(m2);
        cdecl(h, DBG_PRINTF, &[msg, used]);
        cdecl(h, super::MOUNT_PRE2, &[]);
        let mb = cdecl(h, super::INSTALLED_MB, &[0]);
        let msg = h.rebase(m3);
        cdecl(h, DBG_PRINTF, &[msg, mb]);
        let need = (mb >> 7).wrapping_sub(used);
        let msg = h.rebase(m4);
        cdecl(h, DBG_PRINTF, &[msg, need]);
        let device = h.rebase(super::FREE_SPACE_DEVICE);
        let mut r = cdecl(h, super::FREE_SPACE_OK, &[device, need]);
        if r == 0 {
            h.write(a514, u32::MAX);
        }
        if mounted {
            r = cdecl(h, HDD_MOUNT, &[]);
        }
        r
    }

    /// `FUN_00779ad0(page, clut)` with its (16-byte aligned) frame at
    /// `ebp`: help picture `page`: the picture buffer (`0x0112a030`)
    /// cleared (0x4000 bytes) and filled with 97 rows of 97 bytes (rows
    /// 0x80 apart) from the table `0x0112a048 + 0x10400 + page * 0x24c1`,
    /// then two GS uploads through the transfer at `ebp - 0x60`: the
    /// palette (`0x0112a048 + page << 10`, 16 x 16 at `clut`) and the
    /// picture (0x80 x 0x80 at `clut + 4`), each `FlushCache(0)`,
    /// `sceGsExecLoadImage`, `sceGsSyncPath(0, 0)`. The last sync's
    /// `eax`.
    pub fn help_picture(h: &mut impl Host, ebp: u32, page: u32, clut: u32) -> u32 {
        let table = global(h, super::HELP_TABLE);
        let palette = (page << 10).wrapping_add(table);
        let mut src = table
            .wrapping_add(0x1_0400)
            .wrapping_add(page.wrapping_mul(0x24c1));
        let dst = global(h, super::HELP_PICTURE);
        cdecl(h, MEMSET, &[dst, 0, 0x4000]);
        let mut row = 0u32;
        while row < 0x3080 {
            for col in 0..0x61u32 {
                let b: u8 = h.read(src);
                let dst = global(h, super::HELP_PICTURE);
                h.write(dst.wrapping_add(row).wrapping_add(col), b);
                src = src.wrapping_add(1);
            }
            row += 0x80;
        }
        let image = ebp.wrapping_sub(0x60);
        cdecl(
            h,
            super::SET_LOAD_IMAGE,
            &[image, clut, 1, 0, 0, 0, 0x10, 0x10],
        );
        cdecl(h, FLUSH_CACHE, &[0]);
        cdecl(h, super::EXEC_LOAD_IMAGE, &[image, palette]);
        cdecl(h, GS_SYNC_PATH, &[0, 0]);
        let at = clut.wrapping_add(4);
        cdecl(
            h,
            super::SET_LOAD_IMAGE,
            &[image, at, 2, 0x13, 0, 0, 0x80, 0x80],
        );
        cdecl(h, FLUSH_CACHE, &[0]);
        let picture = global(h, super::HELP_PICTURE);
        cdecl(h, super::EXEC_LOAD_IMAGE, &[image, picture]);
        cdecl(h, GS_SYNC_PATH, &[0, 0])
    }
    /// `hdd_thread_main()` with its frame at `ebp` (the install thread):
    /// install state 1 and busy; `0x00778a10()`; 0 runs the copy: the
    /// read functions for the HDD (`[0x02310c4c](1)`), the HDD mounted,
    /// `0x00777980()`; its 0 also sets the partition up (unmounted, the
    /// name `sprintf(ebp - 0x44, "hdd0:%s,%s,%s,128M,PFS",
    /// 0x00779ca0(0x00c41758), 0x00c41758, 0x00c41764)` with the two
    /// words left on the stack, `sceOpen(name, 3)`: `0x00779d20(fd)` and
    /// `sceClose`, or a message; mounted again); the read functions put
    /// back (`[0x02310c4c](cd)`). The two buffers freed; the result sets
    /// the install state (0: done 2 and checked 1; 1: unmounted, -1; 2:
    /// -2); `disk_open_check_set(0)`, a message; not busy. The message's
    /// `eax`.
    pub fn install_thread(h: &mut impl Host, ebp: u32) -> u32 {
        let [m0, m1, m2, m3] = super::THREAD_MSG.map(|s| h.rebase(s));
        cdecl(h, DBG_PRINTF, &[m0]);
        let state = h.rebase(INSTALL_STATE);
        h.write(state, 1u8);
        let busy = h.rebase(super::INSTALL_BUSY);
        h.write(busy, 1u32);
        let mut result = cdecl(h, super::INSTALL_SETUP, &[]);
        if result == 0 {
            let cd = global(h, super::READ_FUNCS_CD);
            let set = global(h, super::SET_READ_FUNCS_CELL);
            h.call(set, Conv::Cdecl, &[1]);
            cdecl(h, HDD_MOUNT, &[]);
            result = cdecl(h, super::INSTALL_COPY, &[]);
            if result == 0 {
                cdecl(h, super::HDD_UNMOUNT, &[]);
                cdecl(h, super::MOUNT_PREPARE, &[]);
                let (dev, part) = (
                    h.rebase(super::MOUNT_DEVICE),
                    h.rebase(super::FFX_PARTITION),
                );
                let name = cdecl(h, super::PARTITION_NAME, &[dev]);
                let buf = ebp.wrapping_sub(0x44);
                let fmt = h.rebase(super::FULL_PARTITION_FMT);
                let sprintf: u32 = h.read(h.rebase(super::SPRINTF_IAT));
                h.call(sprintf, Conv::Cdecl, &[buf, fmt, name, dev, part]);
                cdecl(h, super::MOUNT_PRE2, &[]);
                let fd = cdecl(h, super::SCE_OPEN, &[buf, 3]);
                if fd.cast_signed() < 0 {
                    cdecl(h, DBG_PRINTF, &[m1]);
                } else {
                    cdecl(h, super::INSTALL_FINISH, &[fd]);
                    cdecl(h, super::SCE_CLOSE, &[fd]);
                }
                cdecl(h, HDD_MOUNT, &[]);
            }
            cdecl(h, DBG_PRINTF, &[m2]);
            let set = global(h, super::SET_READ_FUNCS_CELL);
            h.call(set, Conv::Cdecl, &[cd]);
        }
        for buffer in BUFFERS {
            let at = h.rebase(buffer);
            let p: u32 = h.read(at);
            if p != 0 {
                cdecl(h, USER_FREE, &[p]);
                h.write(at, 0u32);
            }
        }
        match result {
            0 => {
                let checked = h.rebase(super::INSTALL_RESULT);
                h.write(checked, 1u8);
                h.write(state, 2u8);
            }
            1 => {
                cdecl(h, super::HDD_UNMOUNT, &[]);
                h.write(state, 0xffu8);
            }
            2 => h.write(state, 0xfeu8),
            _ => {}
        }
        cdecl(h, DISK_OPEN_CHECK_SET, &[0]);
        let cd = global(h, super::READ_FUNCS_CD);
        let r = cdecl(h, DBG_PRINTF, &[m3, cd]);
        h.write(busy, 0u32);
        r
    }

    /// `FUN_0076ccb0()`: waits for the right disc. Not ready
    /// (`0x00a44810() != 1`): logged, 1. Else, until the drive reports a
    /// disc (`sceCdDiskReady(1)` 2 with `0x00a44810()` not 1; meanwhile
    /// the flag `0x01128a87` and `task_wait(1)`): with the check off
    /// (`0x01128aac`) 0, without an expected header 1; else the header
    /// sector read (`sceCdRead(cd_module_file_sector(1, 1), 1, buffer,
    /// mode)`, retried from the start on failure, its completion awaited
    /// through `0x00777310`, -1 meaning busy), cut at 0x20 bytes and
    /// compared with the expected 32 bytes: equal 0; else logged, the
    /// tray waited on (`sceCdDiskReady(1)` 2) and the wait starts over.
    pub fn wait_disc(h: &mut impl Host) -> u32 {
        let [m0, m1, m2, m3, m4] = super::DISK_WAIT_MSG.map(|s| h.rebase(s));
        let status = cdecl(h, super::CD_STATUS, &[]);
        if status != 1 {
            cdecl(h, DBG_PRINTF, &[m4, status]);
            return 1;
        }
        let mut log_wait = true;
        loop {
            if log_wait {
                cdecl(h, DBG_PRINTF, &[m0]);
                // until a disc is in and the drive is past its first state
                loop {
                    if cdecl(h, DISK_READY, &[1]) == 2 && cdecl(h, super::CD_STATUS, &[]) != 1 {
                        break;
                    }
                    let flag = h.rebase(super::DISK_FLAG);
                    h.write(flag, 1u8);
                    cdecl(h, super::TASK_WAIT_SYM, &[1]);
                }
            }
            log_wait = true;
            cdecl(h, DBG_PRINTF, &[m1]);
            if h.read::<u8>(h.rebase(super::DISK_CHECK_ON)) == 0 {
                return 0;
            }
            let expected = global(h, super::DISK_HEADER);
            if expected == 0 {
                return 1;
            }
            cdecl(h, super::TASK_WAIT_SYM, &[1]);
            cdecl(h, super::STATE_CLEAR, &[]);
            let sector = cdecl(h, MODULE_FILE_SECTOR, &[1, 1]);
            cdecl(h, FLUSH_CACHE, &[0]);
            let buffer = global(h, super::DISK_BUFFER);
            let mode = h.rebase(super::DISK_READ_MODE);
            if cdecl(h, super::CD_READ, &[sector, 1, buffer, mode]) == 0 {
                continue;
            }
            let mut r = cdecl(h, DISK_CHANGED, &[]);
            while r == u32::MAX {
                cdecl(h, super::TASK_WAIT_SYM, &[1]);
                r = cdecl(h, DISK_CHANGED, &[]);
            }
            if r != 0 {
                continue;
            }
            cdecl(h, FLUSH_CACHE, &[0]);
            let buffer = global(h, super::DISK_BUFFER);
            h.write(buffer.wrapping_add(0x20), 0u8);
            let expected = global(h, super::DISK_HEADER);
            let same = (0..8u32).all(|k| {
                h.read::<u32>(buffer.wrapping_add(4 * k))
                    == h.read::<u32>(expected.wrapping_add(4 * k))
            });
            if same {
                cdecl(h, DBG_PRINTF, &[m2]);
                return 0;
            }
            cdecl(h, DBG_PRINTF, &[m3]);
            while cdecl(h, DISK_READY, &[1]) != 2 {
                cdecl(h, super::TASK_WAIT_SYM, &[1]);
            }
            // a disc in: the first wait again while the drive settles,
            // else straight to the check
            log_wait = cdecl(h, super::CD_STATUS, &[]) == 1;
        }
    }
    /// Install-wall frame `k`'s size, `w * h` of the `i16` tables.
    fn wall_size(h: &impl Host, k: u32) -> u32 {
        let w = i32::from(h.read::<i16>(h.rebase(super::WALL_WIDTHS).wrapping_add(2 * k)));
        let t = i32::from(h.read::<i16>(h.rebase(super::WALL_HEIGHTS).wrapping_add(2 * k)));
        w.wrapping_mul(t).cast_unsigned()
    }

    /// `FUN_007798b0(frame, clut)` with its (16-byte aligned) frame at
    /// `ebp`: install-wall frame `frame`: the picture buffer cleared
    /// (0x40000 bytes) and filled with its rows (`h` rows of `w` bytes,
    /// 0x200 apart; `w`, `h` from the `i16` tables) from the pictures
    /// `0x0112a04c` past the earlier frames (the sum of their `w * h`,
    /// summed in pairs as the original does) and 0x1400; then the palette
    /// (`pictures + frame << 10`, 16 x 16 at `clut`) and the picture
    /// (0x200 x 0x200 at `clut + 4`, `PSM` 0x13) uploaded through the
    /// transfer at `ebp - 0x60`. The last sync's `eax`.
    pub fn wall_frame(h: &mut impl Host, ebp: u32, frame: u32, clut: u32) -> u32 {
        let (mut even, mut odd, mut last) = (0u32, 0u32, 0u32);
        let mut k = 0u32;
        if frame.cast_signed() >= 2 {
            while k.cast_signed() < frame.wrapping_sub(1).cast_signed() {
                even = even.wrapping_add(wall_size(h, k));
                odd = odd.wrapping_add(wall_size(h, k + 1));
                k += 2;
            }
        }
        if k.cast_signed() < frame.cast_signed() {
            last = wall_size(h, k);
        }
        let pictures = global(h, super::WALL_PICTURES);
        let palette = (frame << 10).wrapping_add(pictures);
        let mut src = odd
            .wrapping_add(even)
            .wrapping_add(pictures)
            .wrapping_add(last)
            .wrapping_add(0x1400);
        let dst = global(h, super::HELP_PICTURE);
        cdecl(h, MEMSET, &[dst, 0, 0x4_0000]);
        let widths = h.rebase(super::WALL_WIDTHS).wrapping_add(2 * frame);
        let heights = h.rebase(super::WALL_HEIGHTS).wrapping_add(2 * frame);
        let mut row = 0i32;
        let mut offset = 0u32;
        while row < i32::from(h.read::<i16>(heights)) {
            let mut col = 0i32;
            while col < i32::from(h.read::<i16>(widths)) {
                let b: u8 = h.read(src);
                let dst = global(h, super::HELP_PICTURE);
                h.write(
                    dst.wrapping_add(offset).wrapping_add(col.cast_unsigned()),
                    b,
                );
                src = src.wrapping_add(1);
                col += 1;
            }
            row += 1;
            offset = offset.wrapping_add(0x200);
        }
        let image = ebp.wrapping_sub(0x60);
        cdecl(
            h,
            super::SET_LOAD_IMAGE,
            &[image, clut, 1, 0, 0, 0, 0x10, 0x10],
        );
        cdecl(h, FLUSH_CACHE, &[0]);
        cdecl(h, super::EXEC_LOAD_IMAGE, &[image, palette]);
        cdecl(h, GS_SYNC_PATH, &[0, 0]);
        let at = clut.wrapping_add(4);
        cdecl(
            h,
            super::SET_LOAD_IMAGE,
            &[image, at, 8, 0x13, 0, 0, 0x200, 0x200],
        );
        cdecl(h, FLUSH_CACHE, &[0]);
        let picture = global(h, super::HELP_PICTURE);
        cdecl(h, super::EXEC_LOAD_IMAGE, &[image, picture]);
        cdecl(h, GS_SYNC_PATH, &[0, 0])
    }
}
