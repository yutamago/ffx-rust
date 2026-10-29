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
}
