//! M5 `system-a`: the PC platform glue around the PS2 file API
//! (`fios*`, `sce*`), the ATL string-resource lookup, and the Steam
//! achievement records (`g_AchievementRecords`, 34 of 400 bytes: `+0`
//! the API name, `+0x184` the unlocked byte, `+0x188` a word cleared on
//! unlock; record 0 is the platinum, `ACH_COMPLETION_OF_FFX`).

/// `g_AchievementRecords`.
pub const RECORDS: u32 = 0x00c3_5794;
/// A record's size.
pub const RECORD: u32 = 0x190;
/// Record 0's unlocked byte (`+0x184`); record `i`'s is `i * RECORD`
/// further on.
pub const UNLOCKED: u32 = 0x00c3_5918;
/// Record 0's word at `+0x188`, cleared on unlock.
pub const UNLOCK_WORD: u32 = 0x00c3_591c;
/// The highest achievement id `StatsAndAchievements_Unlock` takes.
pub const LAST_ID: u32 = 0x21;
/// `g_SteamReady` (byte).
pub const STEAM_READY: u32 = 0x00cc_9cf5;

/// The originals over game memory.
pub mod bridge {
    use super::{LAST_ID, RECORD, RECORDS, STEAM_READY, UNLOCK_WORD, UNLOCKED};
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// `Phyre_PSerialization_PStreamFile_closeFile` (thiscall) -> `al`.
    pub const CLOSE_FILE: u32 = 0x0060_7d80;
    /// `operator delete` (cdecl).
    pub const OPERATOR_DELETE: u32 = 0x0063_0390;
    /// `fiosClose` (cdecl).
    pub const FIOS_CLOSE: u32 = 0x0067_9510;
    /// `fiosOpen` (cdecl).
    pub const FIOS_OPEN: u32 = 0x0067_9730;
    /// `dbgPrintf` (cdecl, varargs).
    pub const DBG_PRINTF: u32 = 0x0062_f500;
    /// `"Virtuos sceWrite"`.
    pub const MSG_SCE_WRITE: u32 = 0x00b4_133c;
    /// `KERNEL32` import slots: `LoadResource`, `LockResource`,
    /// `SizeofResource` (stdcall).
    pub const LOAD_RESOURCE_IAT: u32 = 0x00b0_c170;
    pub const LOCK_RESOURCE_IAT: u32 = 0x00b0_c174;
    pub const SIZEOF_RESOURCE_IAT: u32 = 0x00b0_c178;
    /// `SaveDataManager_instance` (cdecl) and
    /// `SaveDataManager_isSaveEnabled` (thiscall).
    pub const SAVE_MANAGER: u32 = 0x006f_2070;
    pub const SAVE_IS_ENABLED: u32 = 0x006f_2190;
    /// `steam_api!SteamUserStats` (cdecl).
    pub const STEAM_USER_STATS_IAT: u32 = 0x00b0_c898;
    /// `AchievementManager_GetInstance` (cdecl).
    pub const ACHIEVEMENTS: u32 = 0x006f_15b0;

    /// Calls the import in `slot` (its `ffx.sym` address).
    fn import(h: &mut impl Host, slot: u32, conv: Conv, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, conv, args)
    }

    /// `fiosClose(f)` (cdecl): 9 for a null `f`; else
    /// `PStreamFile::closeFile` (its `al`), the file's bare-`ret`
    /// destructor (`0x00607c10`, not called: it does nothing) and
    /// `operator delete(f)`; 0 when `closeFile` gave 1, else 9.
    pub fn fios_close(h: &mut impl Host, f: u32) -> u32 {
        if f == 0 {
            return 9;
        }
        let closed = h.call_sym(CLOSE_FILE, Conv::Thiscall(f), &[]) & 0xff == 1;
        h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[f]);
        if closed { 0 } else { 9 }
    }

    /// `sceClose(fd)` (cdecl): [`fios_close`], then 0.
    pub fn sce_close(h: &mut impl Host, fd: u32) -> u32 {
        fios_close(h, fd);
        0
    }

    /// `sceWrite(fd, buf, ..)` (cdecl): `dbgPrintf("Virtuos sceWrite",
    /// buf)`, 0 (nothing is written).
    pub fn sce_write(h: &mut impl Host, buf: u32) -> u32 {
        let msg = h.rebase(MSG_SCE_WRITE);
        h.call_sym(DBG_PRINTF, Conv::Cdecl, &[msg, buf]);
        0
    }

    /// `0x0061b3c0(module, info, id)` (cdecl): ATL's string-resource image
    /// lookup. `LoadResource(module, info)` (0: 0), `LockResource` (0: 0),
    /// `SizeofResource(module, info)`; then `id & 0xf` length-prefixed
    /// UTF-16 strings skipped (each `2 * len + 2` bytes; past the end: 0).
    /// Returns the string's address when it lies before the end and its
    /// length is not 0, else 0.
    pub fn string_resource_image(h: &mut impl Host, module: u32, info: u32, id: u32) -> u32 {
        let global = import(h, LOAD_RESOURCE_IAT, Conv::Stdcall, &[module, info]);
        if global == 0 {
            return 0;
        }
        let mut p = import(h, LOCK_RESOURCE_IAT, Conv::Stdcall, &[global]);
        if p == 0 {
            return 0;
        }
        let size = import(h, SIZEOF_RESOURCE_IAT, Conv::Stdcall, &[module, info]);
        let end = p.wrapping_add(size);
        for _ in 0..id & 0xf {
            if p >= end {
                return 0;
            }
            let len: u16 = h.read(p);
            p = p.wrapping_add(2 * u32::from(len)).wrapping_add(2);
        }
        if p >= end {
            return 0;
        }
        let len: u16 = h.read(p);
        if len != 0 { p } else { 0 }
    }

    /// `SaveDataIsSaveEnabled()` (cdecl): a tail jump to
    /// `SaveDataManager_isSaveEnabled(SaveDataManager_instance())`; its
    /// `eax`.
    pub fn is_save_enabled(h: &mut impl Host) -> u32 {
        let m = h.call_sym(SAVE_MANAGER, Conv::Cdecl, &[]);
        h.call_sym(SAVE_IS_ENABLED, Conv::Thiscall(m), &[])
    }

    /// Record `id`'s address plus `field` (`RECORDS`, `UNLOCKED`, ...):
    /// `id * 0x190` in wrapping 32 bits, as the original's `imul`.
    fn record(h: &impl Host, field: u32, id: u32) -> u32 {
        h.rebase(field).wrapping_add(id.wrapping_mul(RECORD))
    }

    /// `StatsAndAchievements_SetAchievement_006f1900(this; id)` (thiscall,
    /// `ret 4`): only while `this + 0x4d` is set: record `id` unlocked
    /// (byte `+0x184` = 1, word `+0x188` = 0), then, the flag read again,
    /// `SteamUserStats()->SetAchievement(name)` (vtable `+0x1c`) and
    /// `SteamUserStats()->StoreStats()` (`+0x28`). Its `eax` is junk to
    /// the only caller.
    pub fn set_achievement(h: &mut impl Host, this: u32, id: u32) {
        if h.read::<u8>(this.wrapping_add(0x4d)) == 0 {
            return;
        }
        let name: u32 = h.read(record(h, RECORDS, id));
        h.write(record(h, UNLOCKED, id), 1u8);
        h.write(record(h, UNLOCK_WORD, id), 0u32);
        if h.read::<u8>(this.wrapping_add(0x4d)) == 0 {
            return;
        }
        for (slot, args) in [(0x1cu32, &[name][..]), (0x28, &[][..])] {
            let stats = import(h, STEAM_USER_STATS_IAT, Conv::Cdecl, &[]);
            let vtable: u32 = h.read(stats);
            let f: u32 = h.read(vtable.wrapping_add(slot));
            h.call(f, Conv::Thiscall(stats), args);
        }
    }

    /// `StatsAndAchievements_Unlock_006f1510(this; id)` (thiscall, `ret
    /// 4`): when `this + 0x4d` is set, `id` is at most 0x21 (unsigned)
    /// and its record is not unlocked yet: [`set_achievement`], then
    /// [`try_obtain_platinum`].
    pub fn unlock(h: &mut impl Host, this: u32, id: u32) {
        if h.read::<u8>(this.wrapping_add(0x4d)) == 0 || id > LAST_ID {
            return;
        }
        if h.read::<u8>(record(h, UNLOCKED, id)) != 0 {
            return;
        }
        set_achievement(h, this, id);
        try_obtain_platinum(h, this);
    }

    /// `AchievementTryObtainPlat(this)` (`ecx` = the achievement manager,
    /// passed on): unless record 0 (the platinum) is unlocked, once
    /// records 1..=33 all are, [`unlock`]`(this; 0)`.
    pub fn try_obtain_platinum(h: &mut impl Host, this: u32) {
        if h.read::<u8>(record(h, UNLOCKED, 0)) != 0 {
            return;
        }
        if (1..=0x21).any(|id| h.read::<u8>(record(h, UNLOCKED, id)) == 0) {
            return;
        }
        unlock(h, this, 0);
    }

    /// `SaveDataUnlockAchievement(id)` (cdecl): while `g_SteamReady`,
    /// [`unlock`] on `AchievementManager_GetInstance()`.
    pub fn unlock_achievement(h: &mut impl Host, id: u32) {
        if h.read::<u8>(h.rebase(STEAM_READY)) == 0 {
            return;
        }
        let m = h.call_sym(ACHIEVEMENTS, Conv::Cdecl, &[]);
        unlock(h, m, id);
    }

    /// `operator new` (cdecl).
    pub const OPERATOR_NEW: u32 = 0x0063_0310;
    /// `Phyre_PSerialization_PStreamFile_ctor(this; name, mode, 0, 0, 1)`
    /// (thiscall, `ret 0x14`).
    pub const STREAM_FILE_CTOR: u32 = 0x0060_7bc0;
    /// `Phyre_PSerialization_PStreamFileWin32_isOpen` (thiscall) -> `al`.
    pub const STREAM_FILE_IS_OPEN: u32 = 0x0060_7f20;
    /// ATL's string manager (`0x00c64bf0`; vtable `+0xc` the nil string,
    /// the string data's manager `+4` frees it).
    pub const STRING_MANAGER: u32 = 0x00c6_4bf0;
    /// `0x0064f970(id, module)` (cdecl): the string resource's info.
    pub const FIND_STRING_RESOURCE: u32 = 0x0064_f970;
    /// `0x00652310(cstring; info, id)` (thiscall, `ret 8`): loads it.
    pub const LOAD_STRING_RESOURCE: u32 = 0x0065_2310;
    /// The missing-file message's string id.
    pub const MSG_FILE_MISSING: u32 = 0x7836;
    /// `USER32` import slots: `ShowCursor`, `MessageBoxW` (stdcall).
    pub const SHOW_CURSOR_IAT: u32 = 0x00b0_c688;
    pub const MESSAGE_BOX_IAT: u32 = 0x00b0_c684;
    /// `L"error"`.
    pub const CAPTION_ERROR: u32 = 0x00b3_f9fc;
    /// The fatal-error flag set before `ResetToLauncher` (byte).
    pub const FATAL_FILE_ERROR: u32 = 0x00cc_c7bc;
    /// `ResetToLauncher` (cdecl).
    pub const RESET_TO_LAUNCHER: u32 = 0x0042_f4d0;

    /// `fiosOpen(name, mode)` (cdecl): `mode`'s low byte masked to bit 0
    /// in its argument slot (`mode_slot`, which the original then reuses
    /// for the error message's `CString`), a `PStreamFile` (`operator
    /// new(8)`, constructor `(name, mode, 0, 0, 1)`; none when `new`
    /// fails), `isOpen` asked even of a null file. Open: the file. Else
    /// the file deleted (its bare-`ret` destructor `0x00607c10` not
    /// called), the message (string 0x7836, the nil string when it has no
    /// resource) shown (`ShowCursor(1)`, `MessageBoxW(0, msg, L"error",
    /// MB_ICONERROR)`), the fatal flag `0x00ccc7bc` set,
    /// `ResetToLauncher()`, the `CString` released (its reference count at
    /// data `- 4`; the manager's `Free` at 0); 0.
    pub fn fios_open(h: &mut impl Host, name: u32, mode: u32, mode_slot: u32) -> u32 {
        let mode = mode & 0xffff_ff01;
        h.write(mode_slot, u8::from(mode & 1 != 0));
        let file = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[8]);
        if file != 0 {
            h.call_sym(
                STREAM_FILE_CTOR,
                Conv::Thiscall(file),
                &[name, mode, 0, 0, 1],
            );
        }
        if h.call_sym(STREAM_FILE_IS_OPEN, Conv::Thiscall(file), &[]) & 0xff != 0 {
            return file;
        }
        if file != 0 {
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[file]);
        }
        let manager = h.rebase(STRING_MANAGER);
        let vtable: u32 = h.read(manager);
        let nil_fn: u32 = h.read(vtable.wrapping_add(0xc));
        let nil = h.call(nil_fn, Conv::Thiscall(manager), &[]);
        let mut text = nil.wrapping_add(0x10);
        h.write(mode_slot, text);
        let info = h.call_sym(FIND_STRING_RESOURCE, Conv::Cdecl, &[MSG_FILE_MISSING, 0]);
        if info != 0 {
            h.call_sym(
                LOAD_STRING_RESOURCE,
                Conv::Thiscall(mode_slot),
                &[info, MSG_FILE_MISSING],
            );
            text = h.read(mode_slot);
        }
        import(h, SHOW_CURSOR_IAT, Conv::Stdcall, &[1]);
        let caption = h.rebase(CAPTION_ERROR);
        import(h, MESSAGE_BOX_IAT, Conv::Stdcall, &[0, text, caption, 0x10]);
        h.write(h.rebase(FATAL_FILE_ERROR), 1u8);
        h.call_sym(RESET_TO_LAUNCHER, Conv::Cdecl, &[]);
        // ~CString: `lock xadd` of -1 on the reference count
        let data = text.wrapping_sub(0x10);
        let count: u32 = h.read(data.wrapping_add(0xc));
        let left = count.wrapping_sub(1);
        h.write(data.wrapping_add(0xc), left);
        if left.cast_signed() <= 0 {
            let owner: u32 = h.read(data);
            let vtable: u32 = h.read(owner);
            let free: u32 = h.read(vtable.wrapping_add(4));
            h.call(free, Conv::Thiscall(owner), &[data]);
        }
        0
    }

    /// `"/ffx_ps2/"` (ten bytes with the terminator).
    pub const PS2_PREFIX: u32 = 0x00b4_1330;
    /// `crt_memset_thunk` and `crt_memcpy_thunk` (cdecl).
    pub const MEMSET: u32 = 0x0094_964c;
    pub const MEMCPY: u32 = 0x0094_9256;
    /// `fiosUnifyFilename(in, out, size)` and `fiosIsFileExist(path)`
    /// (cdecl).
    pub const UNIFY_FILENAME: u32 = 0x0067_9820;
    pub const FILE_EXISTS: u32 = 0x0067_96f0;
    /// `sceOpen`'s frame: two 0x400-byte paths at `ebp - 0x404` and
    /// `ebp - 0x804`.
    pub const SCE_OPEN_FRAME: u32 = 0x804;

    /// `sceOpen(name, mode)` (cdecl): `"/ffx_ps2/"` and `name + 7` (past
    /// `cdrom0:`) at `ebp - 0x404` (the prefix's ten bytes, the rest of
    /// the 0x400 zeroed by `memset`, the name's bytes and terminator
    /// `memcpy`d over the prefix's terminator), `fiosUnifyFilename` into
    /// `ebp - 0x804`, and when `fiosIsFileExist`, `fiosOpen(path, mode)`;
    /// returns the file or 0. `ebp` is the original's frame
    /// ([`SCE_OPEN_FRAME`] bytes below it; the cookie at `ebp - 4` is not
    /// written).
    pub fn sce_open(h: &mut impl Host, ebp: u32, name: u32, mode: u32) -> u32 {
        let (path, unified) = (ebp.wrapping_sub(0x404), ebp.wrapping_sub(0x804));
        let prefix = h.rebase(PS2_PREFIX);
        let head: u32 = h.read(prefix);
        let tail: u32 = h.read(prefix.wrapping_add(4));
        let last: u16 = h.read(prefix.wrapping_add(8));
        h.write(path, head);
        h.write(path.wrapping_add(4), tail);
        h.write(path.wrapping_add(8), last);
        h.call_sym(MEMSET, Conv::Cdecl, &[path.wrapping_add(10), 0, 0x3f6]);
        let s = name.wrapping_add(7);
        let mut len = 0u32;
        while h.read::<u8>(s.wrapping_add(len)) != 0 {
            len = len.wrapping_add(1);
        }
        h.call_sym(
            MEMCPY,
            Conv::Cdecl,
            &[path.wrapping_add(9), s, len.wrapping_add(1)],
        );
        h.call_sym(UNIFY_FILENAME, Conv::Cdecl, &[path, unified, 0x400]);
        if h.call_sym(FILE_EXISTS, Conv::Cdecl, &[unified]) == 0 {
            return 0;
        }
        h.call_sym(FIOS_OPEN, Conv::Cdecl, &[unified, mode])
    }

    /// `Sg_GetDispMode` (cdecl).
    pub const GET_DISP_MODE: u32 = 0x0082_0550;
    /// The emulated GS's `PMODE` register (absolute: no relocation).
    pub const GS_PMODE: u32 = 0x1200_0000;

    /// `TOSetEnvMovieMessage()` (cdecl): with a display mode, the GS
    /// `PMODE` register (the 64-bit word at the absolute `0x12000000`) =
    /// 0x42 (circuit 2 on, alpha from `ALP`), returning 0x42; else the
    /// mode's 0.
    pub fn set_env_movie_message(h: &mut impl Host) -> u32 {
        if h.call_sym(GET_DISP_MODE, Conv::Cdecl, &[]) == 0 {
            return 0;
        }
        h.write(GS_PMODE, 0x42u32);
        h.write(GS_PMODE + 4, 0u32);
        0x42
    }

    /// `TOGetFFXLang` (cdecl).
    pub const GET_FFX_LANG: u32 = 0x0064_10e0;

    /// `TOGetFFXLangJmp()` (cdecl): a tail jump to `TOGetFFXLang`.
    pub fn get_ffx_lang_jmp(h: &mut impl Host) -> u32 {
        h.call_sym(GET_FFX_LANG, Conv::Cdecl, &[])
    }

    /// The copyright notice's fade (`0x018716a4`, a word 0..0x80).
    pub const COPYRIGHT_ALPHA: u32 = 0x0187_16a4;
    /// The notice's two `f32`s: `g_AlphaScaleExtended` and the one below.
    pub const COPYRIGHT_W: u32 = 0x00b4_1d9c;
    pub const COPYRIGHT_H: u32 = 0x00b4_1d98;
    /// The notice's texture names, Japanese and other languages.
    pub const COPYRIGHT_NAME_JP: u32 = 0x00c5_9ed0;
    pub const COPYRIGHT_NAME: u32 = 0x00c5_9ed4;
    /// `graphicDrawUIElement(quad, name, screen, kind, vertex_kind)`.
    pub const DRAW_UI_ELEMENT: u32 = 0x0063_eee0;
    /// `ToDrawEternalCalmCopyRight`'s quad: `ebp - 0x9c`, 0x98 bytes.
    pub const COPYRIGHT_FRAME: u32 = 0x9c;

    /// `ToDrawEternalCalmCopyRight(frame)` (cdecl): the fade (`frame`
    /// signed: below 0x20 up by 4 to 0x80, below 0x110 0x80, else down by
    /// 4 to 0), then the quad at `ebp - 0x9c` (corners 0.0, the colour
    /// 0x80 0x80 0x80 and the fade twice, the size from `0x00b41d9c` /
    /// `0x00b41d98`, scale 1.0 twice, `+0x80` = 1000, `+0x90` and `+0x94`
    /// 0; the rest the original never writes: junk read, 0 here, brief
    /// 2026-10-24) drawn with the language's texture name
    /// (`TOGetFFXLangJmp` 0: Japanese). Returns the draw's `eax`.
    pub fn draw_copyright(h: &mut impl Host, ebp: u32, frame: u32) -> u32 {
        let cell = h.rebase(COPYRIGHT_ALPHA);
        let frame = frame.cast_signed();
        let alpha = if frame < 0x20 {
            let a = h.read::<u32>(cell).wrapping_add(4);
            h.write(cell, a);
            if a.cast_signed() > 0x80 {
                h.write(cell, 0x80u32);
                0x80
            } else {
                a
            }
        } else if frame < 0x110 {
            h.write(cell, 0x80u32);
            0x80
        } else {
            let a = h.read::<u32>(cell).wrapping_sub(4);
            let a = if a.cast_signed() < 0 { 0 } else { a };
            h.write(cell, a);
            a
        };
        let q = ebp.wrapping_sub(COPYRIGHT_FRAME);
        let one = 1.0f32.to_bits();
        let w: u32 = h.read(h.rebase(COPYRIGHT_W));
        let hh: u32 = h.read(h.rebase(COPYRIGHT_H));
        for (off, v) in [
            (0x00u32, 0u32),
            (0x04, 0),
            (0x08, 0),
            (0x0c, 0),
            (0x10, 0x80),
            (0x14, 0x80),
            (0x18, 0x80),
            (0x1c, alpha),
            (0x20, x87::store(x87::load(w))),
            (0x24, x87::store(x87::load(hh))),
            (0x28, one),
            (0x2c, one),
            (0x30, 0x80),
            (0x34, 0x80),
            (0x38, 0x80),
            (0x3c, alpha),
            (0x80, 0x3e8),
            (0x90, 0),
            (0x94, 0),
        ] {
            h.write(q.wrapping_add(off), v);
        }
        let name_at = if get_ffx_lang_jmp(h) == 0 {
            COPYRIGHT_NAME_JP
        } else {
            COPYRIGHT_NAME
        };
        let name: u32 = h.read(h.rebase(name_at));
        h.call_sym(DRAW_UI_ELEMENT, Conv::Cdecl, &[q, name, 1, 0, 0])
    }

    /// `KERNEL32` import slots: `WaitForSingleObject`, `ReleaseSemaphore`
    /// (stdcall).
    pub const WAIT_FOR_SINGLE_OBJECT_IAT: u32 = 0x00b0_c0f8;
    pub const RELEASE_SEMAPHORE_IAT: u32 = 0x00b0_c0f4;
    /// The cloth job scheduler (`0x00cdede4`).
    pub const CLOTH_SCHEDULER: u32 = 0x00cd_ede4;

    /// `JobSchedule_JobScheduler_waitForComplete(this; queue)` (thiscall,
    /// `ret 4`): queue `this + queue * 0x1e8`; unless its count (`+0x1f8`,
    /// signed) is 8 or more, its semaphore (`+0x234`) waited for 8 times
    /// (`INFINITE`) and released 8 times by 1. `eax` junk to the callers.
    pub fn wait_for_complete(h: &mut impl Host, this: u32, queue: u32) {
        let q = this.wrapping_add(queue.wrapping_mul(0x1e8));
        if h.read::<u32>(q.wrapping_add(0x1f8)).cast_signed() >= 8 {
            return;
        }
        for _ in 0..8 {
            let sem: u32 = h.read(q.wrapping_add(0x234));
            import(
                h,
                WAIT_FOR_SINGLE_OBJECT_IAT,
                Conv::Stdcall,
                &[sem, u32::MAX],
            );
        }
        for _ in 0..8 {
            let sem: u32 = h.read(q.wrapping_add(0x234));
            import(h, RELEASE_SEMAPHORE_IAT, Conv::Stdcall, &[sem, 1, 0]);
        }
    }

    /// `JobScheduleWait_Ch_Cloth_PerCharacter()` (cdecl): queue 0 of the
    /// cloth scheduler.
    pub fn wait_cloth(h: &mut impl Host) {
        let s: u32 = h.read(h.rebase(CLOTH_SCHEDULER));
        wait_for_complete(h, s, 0);
    }

    /// `ClusterManager_GetInstance` (cdecl) and
    /// `ClusterManager_releasePCluster(this; cluster)` (thiscall, `ret 4`).
    pub const CLUSTER_MANAGER: u32 = 0x0069_a1a0;
    pub const RELEASE_CLUSTER: u32 = 0x0069_bdb0;
    /// `Phyre_PCluster_dtor_PCluster` (thiscall) and `Phyre_PhyreFree`
    /// (cdecl).
    pub const CLUSTER_DTOR: u32 = 0x0043_fa30;
    pub const PHYRE_FREE: u32 = 0x0042_fb20;

    /// `LoadingScreen_unloadTextures(this)` (thiscall): the 20 clusters
    /// of the array at `+0x1c`, then the one at `+0x28` when set, handed
    /// back to the cluster manager. Returns the last release's `eax`, or
    /// 0 without a `+0x28` cluster.
    pub fn unload_textures(h: &mut impl Host, this: u32) -> u32 {
        for i in 0..20u32 {
            let array: u32 = h.read(this.wrapping_add(0x1c));
            let cluster: u32 = h.read(array.wrapping_add(4 * i));
            let m = h.call_sym(CLUSTER_MANAGER, Conv::Cdecl, &[]);
            h.call_sym(RELEASE_CLUSTER, Conv::Thiscall(m), &[cluster]);
        }
        let last: u32 = h.read(this.wrapping_add(0x28));
        if last == 0 {
            return 0;
        }
        let m = h.call_sym(CLUSTER_MANAGER, Conv::Cdecl, &[]);
        h.call_sym(RELEASE_CLUSTER, Conv::Thiscall(m), &[last])
    }

    /// `LoadingScreen_terminate(this)` (thiscall): the cluster at `+0x34`
    /// destroyed and freed when set, `+0x34` = 0, then (a tail jump)
    /// [`unload_textures`]; its `eax`.
    pub fn terminate(h: &mut impl Host, this: u32) -> u32 {
        let c: u32 = h.read(this.wrapping_add(0x34));
        if c != 0 {
            h.call_sym(CLUSTER_DTOR, Conv::Thiscall(c), &[]);
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[c]);
        }
        h.write(this.wrapping_add(0x34), 0u32);
        unload_textures(h, this)
    }

    /// `0x00634790(this)` (thiscall): the loading screen's destructor:
    /// [`terminate`], then its two arrays (`+0x20` count / `+0x24` data,
    /// `+0x18` / `+0x1c`): the data freed (`PhyreFree`) unless the count
    /// is negative (signed: a borrowed buffer), both words cleared.
    /// `eax` junk to the callers.
    pub fn loading_screen_dtor(h: &mut impl Host, this: u32) {
        terminate(h, this);
        let at = |off: u32| this.wrapping_add(off);
        if h.read::<u32>(at(0x20)).cast_signed() >= 0 {
            let p: u32 = h.read(at(0x24));
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[p]);
        }
        h.write(at(0x24), 0u32);
        h.write(at(0x20), 0u32);
        if h.read::<u32>(at(0x18)).cast_signed() >= 0 {
            let p: u32 = h.read(at(0x1c));
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[p]);
        }
        h.write(at(0x18), 0u32);
        h.write(at(0x1c), 0u32);
    }

    /// `CDataFrameQueue_vftable` and `VbfMkvReader_vftable`, and the
    /// base's (`0x0041e6a0` stores it).
    pub const DATA_FRAME_QUEUE_VTABLE: u32 = 0x00b7_a9e4;
    pub const VBF_MKV_READER_VTABLE: u32 = 0x00b7_a9fc;
    pub const MKV_READER_BASE_VTABLE: u32 = 0x00b7_a9ec;

    /// `CDataFrameQueue_dtor(this)` (thiscall, 7 bytes): the vtable.
    pub fn data_frame_queue_dtor(h: &mut impl Host, this: u32) {
        let vt = h.rebase(DATA_FRAME_QUEUE_VTABLE);
        h.write(this, vt);
    }

    /// `VbfMkvReader_dtor(this)` (thiscall): its vtable, the file at
    /// `+0x10` deleted when set (its bare-`ret` destructor not called) and
    /// cleared, then the base destructor `0x0041e6a0` (inlined: the base
    /// vtable). `eax` junk to the callers.
    pub fn vbf_mkv_reader_dtor(h: &mut impl Host, this: u32) {
        let vt = h.rebase(VBF_MKV_READER_VTABLE);
        h.write(this, vt);
        let file: u32 = h.read(this.wrapping_add(0x10));
        if file != 0 {
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[file]);
            h.write(this.wrapping_add(0x10), 0u32);
        }
        let vt = h.rebase(MKV_READER_BASE_VTABLE);
        h.write(this, vt);
    }

    /// `Phyre_PRedBlackTree_remove(this; node)` (thiscall, `ret 4`),
    /// `Phyre_PRedBlackTree_clear(this)`, `Phyre_PFreeList_reset(this)`,
    /// `Phyre_PFreeList_dtor_PFreeList(this)` (thiscall).
    pub const TREE_REMOVE: u32 = 0x0057_5000;
    pub const TREE_CLEAR: u32 = 0x0057_4760;
    pub const FREE_LIST_RESET: u32 = 0x0049_cc10;
    pub const FREE_LIST_DTOR: u32 = 0x0049_c5b0;
    /// `user_free_thunk` (cdecl).
    pub const USER_FREE_THUNK: u32 = 0x0068_7010;

    /// `0x00633fe0(this)` (thiscall): a `PRedBlackTree` of buffers
    /// destroyed: while the first node (`+0xc`) is neither the tree itself
    /// nor null, it is removed and its buffer (`+0x18`) freed when set;
    /// then the node free list at `+0x18` reset and destroyed, and the
    /// tree cleared twice (the derived and the base destructor). `eax`
    /// junk to the callers.
    pub fn buffer_tree_dtor(h: &mut impl Host, this: u32) {
        loop {
            let node: u32 = h.read(this.wrapping_add(0xc));
            if node == this || node == 0 {
                break;
            }
            h.call_sym(TREE_REMOVE, Conv::Thiscall(this), &[node]);
            let buf: u32 = h.read(node.wrapping_add(0x18));
            if buf != 0 {
                h.call_sym(USER_FREE_THUNK, Conv::Cdecl, &[buf]);
            }
        }
        let list = this.wrapping_add(0x18);
        h.call_sym(FREE_LIST_RESET, Conv::Thiscall(list), &[]);
        h.call_sym(FREE_LIST_DTOR, Conv::Thiscall(list), &[]);
        h.call_sym(TREE_CLEAR, Conv::Thiscall(this), &[]);
        h.call_sym(TREE_CLEAR, Conv::Thiscall(this), &[]);
    }

    /// `0x00634220(this)` (thiscall): a `std::list` (`+0` the sentinel,
    /// `+4` the size) and a `std::vector` (`+8` begin, `+0xc` end, `+0x10`
    /// capacity end) destroyed: the vector's block deleted and its three
    /// words cleared when set; the list unlinked (the sentinel pointing at
    /// itself, the size 0), each node deleted in order, then the sentinel.
    /// Returns the last `operator delete`'s `eax`.
    pub fn list_vector_dtor(h: &mut impl Host, this: u32) -> u32 {
        let block: u32 = h.read(this.wrapping_add(8));
        if block != 0 {
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[block]);
            for off in [8u32, 0xc, 0x10] {
                h.write(this.wrapping_add(off), 0u32);
            }
        }
        let head: u32 = h.read(this);
        let mut node: u32 = h.read(head);
        h.write(head, head);
        let head: u32 = h.read(this);
        h.write(head.wrapping_add(4), head);
        h.write(this.wrapping_add(4), 0u32);
        while node != h.read::<u32>(this) {
            let next: u32 = h.read(node);
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[node]);
            node = next;
        }
        let head: u32 = h.read(this);
        h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[head])
    }

    /// MSVCR110 import slots: `vsprintf`, `__iob_func`, `fprintf`.
    pub const VSPRINTF_IAT: u32 = 0x00b0_c430;
    pub const IOB_FUNC_IAT: u32 = 0x00b0_c350;
    pub const FPRINTF_IAT: u32 = 0x00b0_c38c;
    /// `"%s"`.
    pub const FMT_S: u32 = 0x00b0_e160;
    /// `SaveDataTtyWrite2`'s frame: the 0x400-byte line at `ebp - 0x404`.
    pub const TTY_FRAME: u32 = 0x404;

    /// `SaveDataTtyWrite2(fd, fmt, ...)` (cdecl, varargs: `args` is the
    /// address of the first variable argument, the caller's stack): the
    /// line at `ebp - 0x404` zeroed (`memset`), `vsprintf(line, fmt,
    /// args)`, `fprintf(stdout, "%s", line)` (`__iob_func() + 0x20`).
    /// Returns 0.
    pub fn tty_write2(h: &mut impl Host, ebp: u32, fmt: u32, args: u32) -> u32 {
        let line = ebp.wrapping_sub(TTY_FRAME);
        h.call_sym(MEMSET, Conv::Cdecl, &[line, 0, 0x400]);
        import(h, VSPRINTF_IAT, Conv::Cdecl, &[line, fmt, args]);
        let s = h.rebase(FMT_S);
        let iob = import(h, IOB_FUNC_IAT, Conv::Cdecl, &[]);
        import(
            h,
            FPRINTF_IAT,
            Conv::Cdecl,
            &[iob.wrapping_add(0x20), s, line],
        );
        0
    }

    /// `Phyre_FogEffectManager_reset(this)` (thiscall).
    pub const FOG_RESET: u32 = 0x006e_34f0;

    /// `0x006e3f10(this)` (thiscall): `FogEffectManager::reset`, then the
    /// words `+0xa0`, `+0xa4`, `+0xa8` cleared. Returns the reset's `eax`.
    pub fn fog_reset_all(h: &mut impl Host, this: u32) -> u32 {
        let r = h.call_sym(FOG_RESET, Conv::Thiscall(this), &[]);
        for off in [0xa0u32, 0xa4, 0xa8] {
            h.write(this.wrapping_add(off), 0u32);
        }
        r
    }

    /// `g_AsyncLoadManager`, `SignalSema(&sema)`, `SleepThread()`.
    pub const ASYNC_LOAD_MANAGER: u32 = 0x00cc_c88c;
    pub const SIGNAL_SEMA: u32 = 0x006f_2ce0;
    pub const SLEEP_THREAD: u32 = 0x006f_2d10;

    /// `0x00691b00()` (cdecl): the async loader's shutdown: with a
    /// manager, its run flag (`+0x14`) cleared, its semaphore (`+8`)
    /// signalled, `SleepThread()`, the manager (read again) deleted
    /// (`vtable[0](1)`) and the global cleared. Returns 0 without a
    /// manager, else the last call's `eax`.
    pub fn async_load_shutdown(h: &mut impl Host) -> u32 {
        let cell = h.rebase(ASYNC_LOAD_MANAGER);
        let m: u32 = h.read(cell);
        if m == 0 {
            return 0;
        }
        h.write(m.wrapping_add(0x14), 0u32);
        let m: u32 = h.read(cell);
        let sema: u32 = h.read(m.wrapping_add(8));
        h.call_sym(SIGNAL_SEMA, Conv::Cdecl, &[sema]);
        let mut r = h.call_sym(SLEEP_THREAD, Conv::Cdecl, &[]);
        let m: u32 = h.read(cell);
        if m != 0 {
            let vtable: u32 = h.read(m);
            let dtor: u32 = h.read(vtable);
            r = h.call(dtor, Conv::Thiscall(m), &[1]);
        }
        h.write(cell, 0u32);
        r
    }

    /// `g_ClusterManager` and its destructor `0x00699d60` (thiscall).
    pub const CLUSTER_MANAGER_CELL: u32 = 0x00cc_ca4c;
    pub const CLUSTER_MANAGER_DTOR: u32 = 0x0069_9d60;

    /// `0x0069a0e0()` (cdecl): the cluster manager, when there is one,
    /// destroyed, deleted and the global cleared. `eax` junk to the
    /// callers (the caller's without a manager).
    pub fn cluster_manager_shutdown(h: &mut impl Host) {
        let cell = h.rebase(CLUSTER_MANAGER_CELL);
        let m: u32 = h.read(cell);
        if m == 0 {
            return;
        }
        h.call_sym(CLUSTER_MANAGER_DTOR, Conv::Thiscall(m), &[]);
        h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[m]);
        h.write(cell, 0u32);
    }

    /// `user_aligned_free` (cdecl).
    pub const USER_ALIGNED_FREE: u32 = 0x006f_b6c0;

    /// `code_00686fe0(p)` (cdecl, a 5-byte `jmp user_aligned_free`).
    pub fn aligned_free_thunk(h: &mut impl Host, p: u32) -> u32 {
        h.call_sym(USER_ALIGNED_FREE, Conv::Cdecl, &[p])
    }

    /// `KERNEL32!ReleaseMutex` (stdcall).
    pub const RELEASE_MUTEX_IAT: u32 = 0x00b0_c1c0;

    /// Whether an MSVC `_Tree` node is the head (its nil byte `+0xd`).
    fn is_nil(h: &impl Host, node: u32) -> bool {
        h.read::<u8>(node.wrapping_add(0xd)) != 0
    }

    /// `0x0069b890(this; id)` (thiscall, `ret 4`): under the mutex at
    /// `[this + 0x18]` (`WaitForSingleObject(INFINITE)` ..
    /// `ReleaseMutex`), whether the `std::map` at `+0x10` (MSVC `_Tree`:
    /// left `+0`, parent `+4`, right `+8`, nil byte `+0xd`, the value
    /// pointer `+0x14`) holds a value whose word 0 is `id` and word 1 is
    /// 1 or more (signed), walked in order from the leftmost node (the
    /// head's left). Returns `eax`: `al` the answer, the upper bytes
    /// `ReleaseMutex`'s.
    pub fn map_has_active(h: &mut impl Host, this: u32, id: u32) -> u32 {
        let cell: u32 = h.read(this.wrapping_add(0x18));
        let handle: u32 = h.read(cell);
        import(
            h,
            WAIT_FOR_SINGLE_OBJECT_IAT,
            Conv::Stdcall,
            &[handle, u32::MAX],
        );
        let head: u32 = h.read(this.wrapping_add(0x10));
        let mut node: u32 = h.read(head);
        let mut found = false;
        while node != head {
            let value: u32 = h.read(node.wrapping_add(0x14));
            if h.read::<u32>(value) == id && h.read::<u32>(value.wrapping_add(4)).cast_signed() >= 1
            {
                found = true;
                break;
            }
            // ++iterator
            if !is_nil(h, node) {
                let right: u32 = h.read(node.wrapping_add(8));
                if is_nil(h, right) {
                    let mut parent: u32 = h.read(node.wrapping_add(4));
                    while !is_nil(h, parent) && node == h.read::<u32>(parent.wrapping_add(8)) {
                        node = parent;
                        parent = h.read(parent.wrapping_add(4));
                    }
                    node = parent;
                } else {
                    node = right;
                    loop {
                        let left: u32 = h.read(node);
                        if is_nil(h, left) {
                            break;
                        }
                        node = left;
                    }
                }
            }
        }
        let cell: u32 = h.read(this.wrapping_add(0x18));
        let handle: u32 = h.read(cell);
        let r = import(h, RELEASE_MUTEX_IAT, Conv::Stdcall, &[handle]);
        (r & !0xff) | u32::from(found)
    }

    /// `Phyre_PRendering_PSamplerStateD3D11_dtor` (thiscall).
    pub const SAMPLER_STATE_DTOR: u32 = 0x0058_5230;

    /// `0x00703070(this)` (thiscall): the byte `+0` and the words `+0x24`,
    /// `+0x2c`, `+0x64`, `+0x68`, `+0x6c`, `+0x20`, `+0x28`, `+0x54`..`+0x60`
    /// cleared (in that order), then a tail jump to the sampler state's
    /// destructor at `+0x30`; its `eax`.
    pub fn clear_sampler_holder(h: &mut impl Host, this: u32) -> u32 {
        h.write(this, 0u8);
        for off in [
            0x24u32, 0x2c, 0x64, 0x68, 0x6c, 0x20, 0x28, 0x54, 0x58, 0x5c, 0x60,
        ] {
            h.write(this.wrapping_add(off), 0u32);
        }
        h.call_sym(
            SAMPLER_STATE_DTOR,
            Conv::Thiscall(this.wrapping_add(0x30)),
            &[],
        )
    }

    /// `g_LocalizationManager` and its destructor `0x006daf30`.
    pub const LOCALIZATION_MANAGER: u32 = 0x00cd_ed58;
    pub const LOCALIZATION_DTOR: u32 = 0x006d_af30;

    /// A global object at `cell`, when set, destroyed (`dtor`, thiscall),
    /// deleted and the global cleared (`0x0069a0e0`, `0x006db040`).
    fn delete_global(h: &mut impl Host, cell: u32, dtor: u32) {
        let cell = h.rebase(cell);
        let m: u32 = h.read(cell);
        if m == 0 {
            return;
        }
        h.call_sym(dtor, Conv::Thiscall(m), &[]);
        h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[m]);
        h.write(cell, 0u32);
    }

    /// `0x006db040()` (cdecl): the localization manager deleted
    /// ([`delete_global`]). `eax` junk to the callers.
    pub fn localization_shutdown(h: &mut impl Host) {
        delete_global(h, LOCALIZATION_MANAGER, LOCALIZATION_DTOR);
    }

    /// `0x0061b690()` (cdecl), fios's resource teardown.
    pub const FIOS_RESOURCES_TERM: u32 = 0x0061_b690;

    /// `fiosFinalize()` (cdecl): [`localization_shutdown`], `0x0061b690`,
    /// and `0x006f9d10` (a bare `ret`, not called); 0.
    pub fn fios_finalize(h: &mut impl Host) -> u32 {
        localization_shutdown(h);
        h.call_sym(FIOS_RESOURCES_TERM, Conv::Cdecl, &[]);
        0
    }

    /// The object at `0x00ce9014` and its destructor `code_006fa8c0`.
    pub const SYNC_MANAGER: u32 = 0x00ce_9014;
    pub const SYNC_MANAGER_DTOR: u32 = 0x006f_a8c0;

    /// `0x0067a900()` (cdecl): the object at `0x00ce9014`, when set,
    /// destroyed (`code_006fa8c0`) and deleted; the global cleared either
    /// way. Returns 0.
    pub fn sync_manager_shutdown(h: &mut impl Host) -> u32 {
        let cell = h.rebase(SYNC_MANAGER);
        let m: u32 = h.read(cell);
        if m != 0 {
            h.call_sym(SYNC_MANAGER_DTOR, Conv::Thiscall(m), &[]);
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[m]);
        }
        h.write(cell, 0u32);
        0
    }

    /// The block at `0x00ccc7cc`.
    pub const BLOCK_CCC7CC: u32 = 0x00cc_c7cc;

    /// `0x0067ac40()` (cdecl): the block at `0x00ccc7cc` deleted and the
    /// global cleared when set. Returns 0 without one, else `operator
    /// delete`'s `eax`.
    pub fn free_block_ccc7cc(h: &mut impl Host) -> u32 {
        let cell = h.rebase(BLOCK_CCC7CC);
        let p: u32 = h.read(cell);
        if p == 0 {
            return 0;
        }
        let r = h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[p]);
        h.write(cell, 0u32);
        r
    }

    /// `PSamplerStateD3D11_release_0059c770` (thiscall) and the two
    /// sampler states it releases at shutdown.
    pub const SAMPLER_RELEASE: u32 = 0x0059_c770;
    pub const SAMPLER_A: u32 = 0x00ce_8208;
    pub const SAMPLER_B: u32 = 0x00ce_822c;

    /// `0x006f5450()` (cdecl): the sampler states at `0x00ce8208` and
    /// `0x00ce822c` released (the second a tail jump); its `eax`.
    pub fn release_samplers(h: &mut impl Host) -> u32 {
        let a = h.rebase(SAMPLER_A);
        h.call_sym(SAMPLER_RELEASE, Conv::Thiscall(a), &[]);
        let b = h.rebase(SAMPLER_B);
        h.call_sym(SAMPLER_RELEASE, Conv::Thiscall(b), &[])
    }

    /// `i % 8` of the ring index after `i + 7` (signed: the original's
    /// `and 0x80000007` / sign fix), the slot before `i`.
    fn ring_prev(i: u32) -> u32 {
        (i.wrapping_add(7).cast_signed() % 8).cast_unsigned()
    }

    /// `0x006dbd10(this; job, queue)` (thiscall, `ret 8`): a job moved
    /// from queue `queue`'s free ring to its run ring. The queue (`this +
    /// queue * 0x1e8`) is written into the `queue` argument slot
    /// (`queue_slot`, when the caller has one: the original reuses it).
    /// `WaitForSingleObject` on the semaphore `+0x234` and the lock
    /// `+0x1fc`; the free ring's index `+0x1f0` stepped back
    /// (`(i + 7) % 8`), its count `+0x1f8` down, the entry at
    /// `+0x1d0 + 4 i` taken; the lock released
    /// (`ReleaseSemaphore(.., 1, 0)`); the entry's job `+0` and `+0x10` =
    /// 0; under the run lock `+0x22c`, the run ring's index `+0x224`
    /// stepped back the same way and the entry stored at `+0x200 + 4 j`,
    /// its count `+0x228` up; the run lock and the run semaphore `+0x230`
    /// released. Returns the last release's `eax`.
    pub fn push_job(
        h: &mut impl Host,
        this: u32,
        job: u32,
        queue: u32,
        queue_slot: Option<u32>,
    ) -> u32 {
        let q = this.wrapping_add(queue.wrapping_mul(0x1e8));
        let at = |off: u32| q.wrapping_add(off);
        let sem: u32 = h.read(at(0x234));
        if let Some(slot) = queue_slot {
            h.write(slot, q);
        }
        import(
            h,
            WAIT_FOR_SINGLE_OBJECT_IAT,
            Conv::Stdcall,
            &[sem, u32::MAX],
        );
        let lock: u32 = h.read(at(0x1fc));
        import(
            h,
            WAIT_FOR_SINGLE_OBJECT_IAT,
            Conv::Stdcall,
            &[lock, u32::MAX],
        );
        let free = ring_prev(h.read(at(0x1f0)));
        let count: u32 = h.read(at(0x1f8));
        h.write(at(0x1f8), count.wrapping_sub(1));
        h.write(at(0x1f0), free);
        let entry: u32 = h.read(at(0x1d0).wrapping_add(free.wrapping_mul(4)));
        let lock: u32 = h.read(at(0x1fc));
        import(h, RELEASE_SEMAPHORE_IAT, Conv::Stdcall, &[lock, 1, 0]);
        h.write(entry, job);
        // the queue read back from the argument slot
        let q = queue_slot.map_or(q, |slot| h.read(slot));
        let at = |off: u32| q.wrapping_add(off);
        h.write(entry.wrapping_add(0x10), 0u32);
        let run_lock: u32 = h.read(at(0x22c));
        import(
            h,
            WAIT_FOR_SINGLE_OBJECT_IAT,
            Conv::Stdcall,
            &[run_lock, u32::MAX],
        );
        let run = ring_prev(h.read(at(0x224)));
        h.write(at(0x224), run);
        h.write(at(0x200).wrapping_add(run.wrapping_mul(4)), entry);
        let queued: u32 = h.read(at(0x228));
        h.write(at(0x228), queued.wrapping_add(1));
        let run_lock: u32 = h.read(at(0x22c));
        import(h, RELEASE_SEMAPHORE_IAT, Conv::Stdcall, &[run_lock, 1, 0]);
        let run_sem: u32 = h.read(at(0x230));
        import(h, RELEASE_SEMAPHORE_IAT, Conv::Stdcall, &[run_sem, 1, 0])
    }

    /// The scheduler's exit job (`0x006dbdf0`) and `Thread_JoinClose`
    /// (thiscall).
    pub const EXIT_JOB: u32 = 0x006d_bdf0;
    pub const THREAD_JOIN_CLOSE: u32 = 0x0070_3a90;

    /// `0x006dbe80(this)` (thiscall, from `graphicFinalize`): the job
    /// scheduler stopped: four exit jobs on queue 0 ([`push_job`]), the
    /// four worker threads (`+0`, 0x14 bytes apart) joined and closed;
    /// queue 0's free ring (`+0x1f8`, `+0x1f4`, `+0x1f0`) and run ring
    /// (`+0x228`, `+0x224`, `+0x220`) reset, each under its lock. Returns
    /// the last release's `eax`.
    pub fn scheduler_shutdown(h: &mut impl Host, this: u32) -> u32 {
        let exit = h.rebase(EXIT_JOB);
        for _ in 0..4 {
            push_job(h, this, exit, 0, None);
        }
        for k in 0..4u32 {
            h.call_sym(
                THREAD_JOIN_CLOSE,
                Conv::Thiscall(this.wrapping_add(0x14 * k)),
                &[],
            );
        }
        let at = |off: u32| this.wrapping_add(off);
        let mut r = 0;
        for (lock, words) in [
            (0x1fcu32, [0x1f8u32, 0x1f4, 0x1f0]),
            (0x22c, [0x228, 0x224, 0x220]),
        ] {
            let l: u32 = h.read(at(lock));
            import(h, WAIT_FOR_SINGLE_OBJECT_IAT, Conv::Stdcall, &[l, u32::MAX]);
            for w in words {
                h.write(at(w), 0u32);
            }
            let l: u32 = h.read(at(lock));
            r = import(h, RELEASE_SEMAPHORE_IAT, Conv::Stdcall, &[l, 1, 0]);
        }
        r
    }

    /// `KERNEL32!InterlockedCompareExchange` (stdcall).
    pub const COMPARE_EXCHANGE_IAT: u32 = 0x00b0_c12c;

    /// `0x0061b4a0(this; timeout)` (thiscall, `ret 4`): a free stream
    /// slot claimed. `WaitForSingleObject([this], timeout)`; signalled (0):
    /// for each of the `[this + 4]` slots of 0x60 bytes at `[this + 8]`,
    /// `InterlockedCompareExchange(slot + 0x58, 1, 0)` (atomic, through the
    /// import); the first that was 0 is returned. Else 0.
    pub fn claim_slot(h: &mut impl Host, this: u32, timeout: u32) -> u32 {
        let event: u32 = h.read(this);
        if import(
            h,
            WAIT_FOR_SINGLE_OBJECT_IAT,
            Conv::Stdcall,
            &[event, timeout],
        ) != 0
        {
            return 0;
        }
        let mut i = 0u32;
        while i < h.read::<u32>(this.wrapping_add(4)) {
            let base: u32 = h.read(this.wrapping_add(8));
            let flag = base.wrapping_add(0x58).wrapping_add(i.wrapping_mul(0x60));
            if import(h, COMPARE_EXCHANGE_IAT, Conv::Stdcall, &[flag, 1, 0]) == 0 {
                let base: u32 = h.read(this.wrapping_add(8));
                return base.wrapping_add(i.wrapping_mul(0x60));
            }
            i += 1;
        }
        0
    }

    /// `0x0061d5c0(this)` (thiscall): a `PRedBlackTree` destroyed: every
    /// node removed from the front (`+0xc`, until the tree itself or
    /// null), the node free list `+0x18` reset and destroyed, the tree
    /// cleared twice. As [`buffer_tree_dtor`] without the buffers. `eax`
    /// junk to the callers.
    pub fn tree_dtor(h: &mut impl Host, this: u32) {
        loop {
            let node: u32 = h.read(this.wrapping_add(0xc));
            if node == this || node == 0 {
                break;
            }
            h.call_sym(TREE_REMOVE, Conv::Thiscall(this), &[node]);
        }
        let list = this.wrapping_add(0x18);
        h.call_sym(FREE_LIST_RESET, Conv::Thiscall(list), &[]);
        h.call_sym(FREE_LIST_DTOR, Conv::Thiscall(list), &[]);
        h.call_sym(TREE_CLEAR, Conv::Thiscall(this), &[]);
        h.call_sym(TREE_CLEAR, Conv::Thiscall(this), &[]);
    }

    /// `operator delete[]` (`0x00630540`, cdecl) and
    /// `eh_vector_destructor_iterator(p, size, count, dtor)` (stdcall).
    pub const OPERATOR_DELETE_ARRAY: u32 = 0x0063_0540;
    pub const EH_VECTOR_DTOR: u32 = 0x0094_96d2;
    /// `VFile_dtor_VFile` and the stream slot's destructor `0x0061b0c0`.
    pub const VFILE_DTOR: u32 = 0x0061_a960;
    pub const SLOT_DTOR: u32 = 0x0061_b0c0;

    /// `delete[] p` of an array of `size`-byte objects with a count
    /// header (`p - 4`) and destructor `dtor`: the CRT's vector
    /// destructor, then `operator delete[](p - 4)`.
    fn delete_vector(h: &mut impl Host, p: u32, size: u32, dtor: u32) {
        let count: u32 = h.read(p.wrapping_sub(4));
        let dtor = h.rebase(dtor);
        h.call_sym(EH_VECTOR_DTOR, Conv::Stdcall, &[p, size, count, dtor]);
        h.call_sym(OPERATOR_DELETE_ARRAY, Conv::Cdecl, &[p.wrapping_sub(4)]);
    }

    /// `BigFileHandle_deinitialize(this)` (thiscall): the blocks `+0x40`,
    /// `+0` and `+0x3c` (`delete[]`, then cleared, when set), and the
    /// `VFile` array `+0x5c` (0x20 bytes each, `delete[]` with its
    /// destructors). `eax` junk to the callers.
    pub fn big_file_deinitialize(h: &mut impl Host, this: u32) {
        for off in [0x40u32, 0, 0x3c] {
            let p: u32 = h.read(this.wrapping_add(off));
            if p != 0 {
                h.call_sym(OPERATOR_DELETE_ARRAY, Conv::Cdecl, &[p]);
                h.write(this.wrapping_add(off), 0u32);
            }
        }
        let files: u32 = h.read(this.wrapping_add(0x5c));
        if files != 0 {
            delete_vector(h, files, 0x20, VFILE_DTOR);
            h.write(this.wrapping_add(0x5c), 0u32);
        }
    }

    /// `KERNEL32` import slots: `DeleteCriticalSection`, `CloseHandle`;
    /// `ADVAPI32!CryptReleaseContext` (stdcall).
    pub const DELETE_CRITICAL_SECTION_IAT: u32 = 0x00b0_c15c;
    pub const CLOSE_HANDLE_IAT: u32 = 0x00b0_c0dc;
    pub const CRYPT_RELEASE_CONTEXT_IAT: u32 = 0x00b0_c014;

    /// `BigFileHandle_dtor_BigFileHandle(this)` (thiscall): the critical
    /// section `+0x44` deleted, [`big_file_deinitialize`], the tree at
    /// `+4` destroyed ([`tree_dtor`]). `eax` junk to the callers.
    pub fn big_file_handle_dtor(h: &mut impl Host, this: u32) {
        import(
            h,
            DELETE_CRITICAL_SECTION_IAT,
            Conv::Stdcall,
            &[this.wrapping_add(0x44)],
        );
        big_file_deinitialize(h, this);
        tree_dtor(h, this.wrapping_add(4));
    }

    /// `0x0061b620(this)` (thiscall): the big-file stream's teardown: the
    /// block `+0x40` (`delete[]`, cleared), the five handles `+0x10..+0x20`
    /// (destroyed, deleted, cleared when set), `CryptReleaseContext([this],
    /// 0)`, the critical section `+0x24` deleted; its `eax`.
    pub fn big_file_stream_term(h: &mut impl Host, this: u32) -> u32 {
        let p: u32 = h.read(this.wrapping_add(0x40));
        if p != 0 {
            h.call_sym(OPERATOR_DELETE_ARRAY, Conv::Cdecl, &[p]);
            h.write(this.wrapping_add(0x40), 0u32);
        }
        for k in 0..5u32 {
            let slot = this.wrapping_add(0x10 + 4 * k);
            let handle: u32 = h.read(slot);
            if handle != 0 {
                big_file_handle_dtor(h, handle);
                h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[handle]);
                h.write(slot, 0u32);
            }
        }
        let provider: u32 = h.read(this);
        import(h, CRYPT_RELEASE_CONTEXT_IAT, Conv::Stdcall, &[provider, 0]);
        import(
            h,
            DELETE_CRITICAL_SECTION_IAT,
            Conv::Stdcall,
            &[this.wrapping_add(0x24)],
        )
    }

    /// `BigFileStream_sBigFileStreamInstacne`.
    pub const BIG_FILE_STREAM: u32 = 0x00cc_9c48;

    /// `0x0061b690()` (cdecl, from `fiosFinalize`): the big-file stream,
    /// when there is one: [`big_file_stream_term`], its slot array `+0xc`
    /// (0x60 bytes each, `delete[]` with destructors, cleared),
    /// `CloseHandle([+4])`, deleted, the global cleared. `eax` junk to the
    /// caller (it returns 0 itself).
    pub fn big_file_stream_shutdown(h: &mut impl Host) {
        let cell = h.rebase(BIG_FILE_STREAM);
        let s: u32 = h.read(cell);
        if s == 0 {
            return;
        }
        big_file_stream_term(h, s);
        let slots: u32 = h.read(s.wrapping_add(0xc));
        if slots != 0 {
            delete_vector(h, slots, 0x60, SLOT_DTOR);
            h.write(s.wrapping_add(0xc), 0u32);
        }
        let event: u32 = h.read(s.wrapping_add(4));
        import(h, CLOSE_HANDLE_IAT, Conv::Stdcall, &[event]);
        h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[s]);
        h.write(cell, 0u32);
    }

    /// `user_free_thunk([p_cell])` when the word is set, then the word at
    /// `clear` = 0.
    fn free_word(h: &mut impl Host, p_cell: u32, clear: u32) {
        let p: u32 = h.read(p_cell);
        if p != 0 {
            h.call_sym(USER_FREE_THUNK, Conv::Cdecl, &[p]);
            h.write(clear, 0u32);
        }
    }

    /// `SyncManager_releaseData(this)` (thiscall) -> 0: the buffer `+0`,
    /// the 0x80 words of the table `[+4]`, the table `+4`, then `+0x24`,
    /// `+0x28`, `+0x2c`, `+0x30`, `+0x34`, `+0x38`, `+0x3c`, `+0x40`
    /// (`user_free_thunk` when set). Each is cleared after its free, but
    /// `+0x34` and `+0x38` clear `+0x30` instead (the original's slip:
    /// they keep their freed pointers). Then `+8..+0x1c` = 0, `+0x20` = 1.
    pub fn sync_release_data(h: &mut impl Host, this: u32) -> u32 {
        let at = |off: u32| this.wrapping_add(off);
        free_word(h, this, this);
        for i in 0..0x80u32 {
            let table: u32 = h.read(at(4));
            let cell = table.wrapping_add(4 * i);
            let p: u32 = h.read(cell);
            if p != 0 {
                h.call_sym(USER_FREE_THUNK, Conv::Cdecl, &[p]);
                // the table read again after the call
                let table: u32 = h.read(at(4));
                h.write(table.wrapping_add(4 * i), 0u32);
            }
        }
        free_word(h, at(4), at(4));
        for (p, clear) in [
            (0x24u32, 0x24u32),
            (0x28, 0x28),
            (0x2c, 0x2c),
            (0x30, 0x30),
            (0x34, 0x30),
            (0x38, 0x30),
            (0x3c, 0x3c),
            (0x40, 0x40),
        ] {
            free_word(h, at(p), at(clear));
        }
        for off in [8u32, 0xc, 0x10, 0x14, 0x18, 0x1c] {
            h.write(at(off), 0u32);
        }
        h.write(at(0x20), 1u32);
        0
    }

    /// `code_006fa8c0(this)` (thiscall, a 5-byte `jmp
    /// SyncManager_releaseData`).
    pub fn sync_release_thunk(h: &mut impl Host, this: u32) -> u32 {
        sync_release_data(h, this)
    }

    /// `PInputMapper_setKey(this; key, v, kbd)` (thiscall, `ret 0xc`):
    /// keyboard `kbd` (0..7, unsigned) of the mapper at `[this + 0x2b8]`
    /// (its `+0x80` table), when set, gets byte `v` at `+0x18 + key -
    /// 0x1c` for keys 0x1c..0x11c. Returns `eax`: the mapper, its low byte
    /// `v` when stored.
    pub fn set_key(h: &mut impl Host, this: u32, key: u32, v: u32, kbd: u32) -> u32 {
        let mapper: u32 = h.read(this.wrapping_add(0x2b8));
        if kbd >= 7 {
            return mapper;
        }
        let board: u32 = h.read(mapper.wrapping_add(0x80).wrapping_add(4 * kbd));
        let k = key.wrapping_sub(0x1c);
        if board == 0 || k >= 0x100 {
            return mapper;
        }
        h.write(board.wrapping_add(k).wrapping_add(0x18), v.to_le_bytes()[0]);
        (mapper & !0xff) | (v & 0xff)
    }

    /// `Phyre_PTimerWin32_GetTime` (cdecl, `ST0` double) and
    /// `g_AppStartTime` (f64).
    pub const GET_TIME: u32 = 0x0042_fd80;
    pub const APP_START_TIME: u32 = 0x00c9_0278;
    /// `inputGetCurTime` (cdecl, `ST0` float).
    pub const CUR_TIME: u32 = 0x0063_0c60;

    /// `inputGetCurTime()` (cdecl) -> `ST0`: `GetTime() - g_AppStartTime`
    /// (`fsub qword`) rounded to `f32` (`fstp dword` / `fld dword`).
    pub fn cur_time(h: &mut impl Host) -> f32 {
        let now = h.call_sym_st0_f64(GET_TIME, Conv::Cdecl, &[]);
        let start: f64 = h.read(h.rebase(APP_START_TIME));
        f32::from_bits(x87::store(x87::sub(now, start)))
    }

    /// `InputManager_ClearHoldTimers(this)` (thiscall): the 29 hold timers
    /// `+8..+0x78` = 0, `+0x88` = `inputGetCurTime()`. `eax` junk to the
    /// caller.
    pub fn clear_hold_timers(h: &mut impl Host, this: u32) {
        for i in 0..29u32 {
            h.write(this.wrapping_add(8 + 4 * i), 0u32);
        }
        let now = h.call_sym_st0(CUR_TIME, Conv::Cdecl, &[]);
        h.write(this.wrapping_add(0x88), now.to_bits());
    }

    /// `InputManager_UpdateHoldTimers(this)` (thiscall): `d = now - +0x88`
    /// (rounded to `f32`); each of the 29 timers `+8..+0x78` grows by `d`
    /// while its bit (1 << i) is held in the pad word `+0`, else is 0
    /// (the previous word `+4` makes no difference); `+0x88` = now.
    /// Returns `eax` = 1 << 29 (the rotated bit).
    pub fn update_hold_timers(h: &mut impl Host, this: u32) -> u32 {
        let now = h.call_sym_st0(CUR_TIME, Conv::Cdecl, &[]);
        let now = x87::load(now.to_bits());
        let last: u32 = h.read(this.wrapping_add(0x88));
        let d = x87::load(x87::store(x87::sub(now, x87::load(last))));
        let mut bit = 1u32;
        for i in 0..29u32 {
            let cell = this.wrapping_add(8 + 4 * i);
            let held: u32 = h.read(this);
            if held & bit != 0 {
                let t: u32 = h.read(cell);
                h.write(cell, x87::store(x87::add(x87::load(t), d)));
            } else {
                h.write(cell, 0u32);
            }
            bit = bit.rotate_left(1);
        }
        h.write(this.wrapping_add(0x88), x87::store(now));
        bit
    }

    /// `Virtuos_InputManager_getInstance` (cdecl),
    /// `Virtuos_InputManager_updateInput` (`ecx` = the manager, no stack
    /// words), `IggyMenuManager_dispatchMouse_00a26ce0` (thiscall),
    /// `g_IggyMenuManager` and the flag `0x0133c930`.
    pub const INPUT_MANAGER: u32 = 0x0063_1300;
    pub const UPDATE_INPUT: u32 = 0x0063_2a30;
    pub const DISPATCH_MOUSE: u32 = 0x00a2_6ce0;
    pub const IGGY_MENU_MANAGER: u32 = 0x0198_4c6c;
    pub const INPUT_READY: u32 = 0x0133_c930;

    /// `inputODUpdate()` (cdecl): while `0x0133c930` is set, the input
    /// manager's `updateInput`, then (a tail jump) the Iggy menu
    /// manager's mouse dispatch; its `eax`. Else 0 (the original leaves
    /// its caller's `eax`; its one caller, a tail jump from
    /// `FFXApplication_pollInputAsync`, returns nothing).
    pub fn od_update(h: &mut impl Host) -> u32 {
        if h.read::<u32>(h.rebase(INPUT_READY)) == 0 {
            return 0;
        }
        let m = h.call_sym(INPUT_MANAGER, Conv::Cdecl, &[]);
        h.call_sym(UPDATE_INPUT, Conv::Thiscall(m), &[]);
        let menus: u32 = h.read(h.rebase(IGGY_MENU_MANAGER));
        h.call_sym(DISPATCH_MOUSE, Conv::Thiscall(menus), &[])
    }

    /// `user_malloc` (cdecl).
    pub const USER_MALLOC: u32 = 0x0068_7020;

    /// `0x00631450(this; n)` (thiscall, `ret 4`): the capacity of an array
    /// of 8-byte pairs (`+0` count, `+4` capacity, `+8` data) set to `n`.
    /// No data yet: `+8` = `user_malloc(8 n)`, `+4` = `n`; returns the
    /// block. Else a new block when `n > 0` (signed; 0 otherwise or when
    /// the allocation fails) with the `count` old pairs copied
    /// (`memcpy`), the old block freed (`user_free_thunk`, its `eax`
    /// returned), `+4` = `n`, `+8` = the new block.
    pub fn pair_array_reserve(h: &mut impl Host, this: u32, n: u32) -> u32 {
        let data: u32 = h.read(this.wrapping_add(8));
        if data == 0 {
            let p = h.call_sym(USER_MALLOC, Conv::Cdecl, &[n.wrapping_mul(8)]);
            h.write(this.wrapping_add(4), n);
            h.write(this.wrapping_add(8), p);
            return p;
        }
        let mut p = 0;
        if n.cast_signed() > 0 {
            p = h.call_sym(USER_MALLOC, Conv::Cdecl, &[n.wrapping_mul(8)]);
            if p != 0 {
                let count: u32 = h.read(this);
                let old: u32 = h.read(this.wrapping_add(8));
                h.call_sym(MEMCPY, Conv::Cdecl, &[p, old, count.wrapping_shl(3)]);
            }
        }
        let old: u32 = h.read(this.wrapping_add(8));
        let r = h.call_sym(USER_FREE_THUNK, Conv::Cdecl, &[old]);
        h.write(this.wrapping_add(4), n);
        h.write(this.wrapping_add(8), p);
        r
    }

    /// `0x00631110(this; other)` (thiscall, `ret 4`): the pair array
    /// assigned from `other` unless it is `this`: count 0, the capacity
    /// set to `other`'s count ([`pair_array_reserve`]), each pair copied
    /// (to a non-null slot; the count read again each step, signed), the
    /// count copied. Returns `this`.
    pub fn pair_array_assign(h: &mut impl Host, this: u32, other: u32) -> u32 {
        if other == this {
            return this;
        }
        h.write(this, 0u32);
        let n: u32 = h.read(other);
        pair_array_reserve(h, this, n);
        let mut i = 0u32;
        while i.cast_signed() < h.read::<u32>(other).cast_signed() {
            let dst = h
                .read::<u32>(this.wrapping_add(8))
                .wrapping_add(i.wrapping_mul(8));
            if dst != 0 {
                let src: u32 = h.read(other.wrapping_add(8));
                let first: u32 = h.read(src.wrapping_add(i.wrapping_mul(8)));
                h.write(dst, first);
                let second: u32 = h.read(src.wrapping_add(i.wrapping_mul(8)).wrapping_add(4));
                h.write(dst.wrapping_add(4), second);
            }
            i = i.wrapping_add(1);
        }
        let n: u32 = h.read(other);
        h.write(this, n);
        this
    }

    /// `SaveDataManager_initSaveDataSys(this; buffer, size)` (thiscall,
    /// `ret 8`), `g_SaveFileBuffer`, `TrophyManager_Instance` (cdecl) and
    /// `SetFieldConst0_0067ac70` (thiscall).
    pub const SAVE_INIT_SYS: u32 = 0x006f_1fa0;
    pub const SAVE_FILE_BUFFER: u32 = 0x0159_7f70;
    pub const TROPHY_MANAGER: u32 = 0x0067_ac90;
    pub const TROPHY_CLEAR: u32 = 0x0067_ac70;

    /// `SaveData_Init()` (cdecl): `initSaveDataSys(manager;
    /// g_SaveFileBuffer, 0x6900)`, then (a tail jump)
    /// `SetFieldConst0_0067ac70(TrophyManager_Instance())`; its `eax`.
    pub fn save_data_init(h: &mut impl Host) -> u32 {
        let m = h.call_sym(SAVE_MANAGER, Conv::Cdecl, &[]);
        let buf = h.rebase(SAVE_FILE_BUFFER);
        h.call_sym(SAVE_INIT_SYS, Conv::Thiscall(m), &[buf, 0x6900]);
        let t = h.call_sym(TROPHY_MANAGER, Conv::Cdecl, &[]);
        h.call_sym(TROPHY_CLEAR, Conv::Thiscall(t), &[])
    }

    /// `0x0067ac10()` (cdecl): the trophy manager (3 bytes, zeroed) at
    /// `0x00ccc7cc` ([`BLOCK_CCC7CC`]), or 0 when `operator new` fails;
    /// returns it.
    pub fn trophy_manager_create(h: &mut impl Host) -> u32 {
        let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[3]);
        if p != 0 {
            h.write(p, 0u16);
            h.write(p.wrapping_add(2), 0u8);
        }
        h.write(h.rebase(BLOCK_CCC7CC), p);
        p
    }

    /// `g_SaveDataManager` and its constructor `0x006f1980` (thiscall).
    pub const SAVE_MANAGER_CELL: u32 = 0x00ce_81f4;
    pub const SAVE_MANAGER_CTOR: u32 = 0x006f_1980;

    /// `0x006f1c10()` (cdecl): `g_SaveDataManager` = a new 0x488-byte
    /// manager (`operator new`, the constructor's `eax`), or 0 when the
    /// allocation fails; returns it.
    pub fn save_manager_create(h: &mut impl Host) -> u32 {
        let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x488]);
        let m = if p == 0 {
            0
        } else {
            h.call_sym(SAVE_MANAGER_CTOR, Conv::Thiscall(p), &[])
        };
        h.write(h.rebase(SAVE_MANAGER_CELL), m);
        m
    }

    /// `StatsAndAchievements_ctor` (thiscall) and `g_StatsAndAchievements`.
    pub const STATS_CTOR: u32 = 0x006f_11a0;
    pub const STATS_CELL: u32 = 0x00ce_81a8;

    /// `StatsAndAchievements_Create()` (cdecl): `g_StatsAndAchievements`
    /// = a new 0x50-byte object built by its constructor (its `eax`), or
    /// 0 when `operator new` fails. Returns the constructor's `eax`, or
    /// `new`'s 0.
    pub fn stats_create(h: &mut impl Host) -> u32 {
        let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x50]);
        if p == 0 {
            h.write(h.rebase(STATS_CELL), 0u32);
            return 0;
        }
        let s = h.call_sym(STATS_CTOR, Conv::Thiscall(p), &[]);
        h.write(h.rebase(STATS_CELL), s);
        s
    }

    /// `MSVCR110!strncpy` (cdecl), the save manager's default strings,
    /// `AddrOfGlobal0159e468_008b3630` and `TOClearBlock_008b3610` (cdecl).
    pub const STRNCPY_IAT: u32 = 0x00b0_c368;
    pub const SAVE_STRINGS: [(u32, u32, u32); 5] = [
        (0x10, 0x00b0_dab8, 0x40),
        (0x50, 0x00b4_bb60, 0x80),
        (0xd0, 0x00b4_bb78, 0x200),
        (0x2d0, 0x00b4_bb94, 0x40),
        (0x310, 0x00b4_bba0, 0x40),
    ];
    pub const TO_BLOCK: u32 = 0x008b_3630;
    pub const TO_CLEAR_BLOCK: u32 = 0x008b_3610;

    /// `0x006f1980(this)` (thiscall, the save manager's constructor):
    /// `+0` = 1, `+4` = 0, `+8`/`+0xc` = -1, `+0x358`/`+0x360`/`+0x368` =
    /// 0, `+0x390` = -1, `+0x398`/`+0x39c` = 0; five default strings
    /// (`strncpy` into `+0x10`, `+0x50`, `+0xd0`, `+0x2d0`, `+0x310`);
    /// `+0x35c` = the TO block's address, `TOClearBlock`; `+0x3a8` =
    /// 0x101 (word), `+0x3aa` = 0, `+0x3ac` = 0. Returns `this`.
    pub fn save_manager_ctor(h: &mut impl Host, this: u32) -> u32 {
        let at = |off: u32| this.wrapping_add(off);
        for (off, v) in [
            (0u32, 1u32),
            (4, 0),
            (8, u32::MAX),
            (0xc, u32::MAX),
            (0x358, 0),
            (0x360, 0),
            (0x368, 0),
            (0x390, u32::MAX),
            (0x398, 0),
            (0x39c, 0),
        ] {
            h.write(at(off), v);
        }
        for (off, s, n) in SAVE_STRINGS {
            let s = h.rebase(s);
            import(h, STRNCPY_IAT, Conv::Cdecl, &[at(off), s, n]);
        }
        let block = h.call_sym(TO_BLOCK, Conv::Cdecl, &[]);
        h.write(at(0x35c), block);
        h.call_sym(TO_CLEAR_BLOCK, Conv::Cdecl, &[]);
        h.write(at(0x3a8), 0x101u16);
        h.write(at(0x3aa), 0u8);
        h.write(at(0x3ac), 0u32);
        this
    }

    /// `0x0061d8e0(this)` (thiscall, from `BigFileHandle_initialize`): a
    /// `PRedBlackTree` emptied: every front node removed (as
    /// [`tree_dtor`]), then (a tail jump) its free list `+0x18` reset; its
    /// `eax`.
    pub fn tree_reset(h: &mut impl Host, this: u32) -> u32 {
        loop {
            let node: u32 = h.read(this.wrapping_add(0xc));
            if node == this || node == 0 {
                break;
            }
            h.call_sym(TREE_REMOVE, Conv::Thiscall(this), &[node]);
        }
        h.call_sym(
            FREE_LIST_RESET,
            Conv::Thiscall(this.wrapping_add(0x18)),
            &[],
        )
    }

    /// `MSVCR110!fopen` (cdecl) and `"rb"`.
    pub const FOPEN_IAT: u32 = 0x00b0_c3f0;
    pub const MODE_RB_A: u32 = 0x00b3_f69c;

    /// `VbfStream_Open(this; path, size)` (thiscall, `ret 8`): `+4` =
    /// `fopen(path, "rb")`; `+8` = `size` when not 0. Returns `size`.
    pub fn vbf_stream_open(h: &mut impl Host, this: u32, path: u32, size: u32) -> u32 {
        let mode = h.rebase(MODE_RB_A);
        let f = import(h, FOPEN_IAT, Conv::Cdecl, &[path, mode]);
        h.write(this.wrapping_add(4), f);
        if size != 0 {
            h.write(this.wrapping_add(8), size);
        }
        size
    }

    /// `SyncManager_SyncManager` (thiscall).
    pub const SYNC_MANAGER_CTOR: u32 = 0x006f_a8b0;

    /// `iSyncInitialize()` (cdecl) -> 0: the `SyncManager` at `0x00ce9014`
    /// created once (`operator new(0x44)`, its constructor; 0 when `new`
    /// fails).
    pub fn sync_initialize(h: &mut impl Host) -> u32 {
        let cell = h.rebase(SYNC_MANAGER);
        if h.read::<u32>(cell) != 0 {
            return 0;
        }
        let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x44]);
        let m = if p == 0 {
            0
        } else {
            h.call_sym(SYNC_MANAGER_CTOR, Conv::Thiscall(p), &[])
        };
        h.write(cell, m);
        0
    }

    /// The localization manager's constructor `0x006daee0` (thiscall).
    pub const LOCALIZATION_CTOR: u32 = 0x006d_aee0;

    /// `0x006dafc0()` (cdecl): `g_LocalizationManager` created once
    /// (`operator new(0x1c)`, its constructor; 0 when `new` fails).
    /// Returns the constructor's `eax`, or 0 when `new` fails; 0 when the
    /// manager already exists (the original's `eax` is the security
    /// cookie's xor there, junk to its one caller).
    pub fn localization_create(h: &mut impl Host) -> u32 {
        let cell = h.rebase(LOCALIZATION_MANAGER);
        if h.read::<u32>(cell) != 0 {
            return 0;
        }
        let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x1c]);
        let m = if p == 0 {
            0
        } else {
            h.call_sym(LOCALIZATION_CTOR, Conv::Thiscall(p), &[])
        };
        h.write(cell, m);
        m
    }

    /// `Vbf_MapLookup(this; &digest)` (thiscall, `ret 4`).
    pub const VBF_MAP_LOOKUP: u32 = 0x0061_d140;
    /// `Vbf_Contains`' frame: the digest copy at `ebp - 0x10`.
    pub const CONTAINS_FRAME: u32 = 0x10;

    /// `Vbf_Contains(this; &digest)` (thiscall, `ret 4`): the 16-byte
    /// digest copied to `ebp - 0x10`, `Vbf_MapLookup(this + 4; &copy)`;
    /// `al` = whether a node came back and its entry (`node + 0x20`) is
    /// not null (wrapping). Returns `eax`: the node (+ 0x20 when not 0),
    /// `al` the answer.
    pub fn vbf_contains(h: &mut impl Host, ebp: u32, this: u32, digest: u32) -> u32 {
        let copy = ebp.wrapping_sub(CONTAINS_FRAME);
        for k in 0..4u32 {
            let w: u32 = h.read(digest.wrapping_add(4 * k));
            h.write(copy.wrapping_add(4 * k), w);
        }
        let node = h.call_sym(
            VBF_MAP_LOOKUP,
            Conv::Thiscall(this.wrapping_add(4)),
            &[copy],
        );
        if node == 0 {
            return 0;
        }
        let entry = node.wrapping_add(0x20);
        if entry == 0 { 0 } else { (entry & !0xff) | 1 }
    }

    /// A function-local static (`0x00cbaa78`, its guard bit 0 of
    /// `0x00cbaa7c`) holding `Phyre_PClassLayoutInterface_vftable`, its
    /// destructor registered with `atexit` (`0x00b03db0`).
    pub const LAYOUT_STATIC: u32 = 0x00cb_aa78;
    pub const LAYOUT_GUARD: u32 = 0x00cb_aa7c;
    pub const LAYOUT_VTABLE: u32 = 0x00b1_3468;
    pub const LAYOUT_DTOR: u32 = 0x00b0_3db0;
    pub const ATEXIT: u32 = 0x0094_9207;

    /// `0x0059e530()` (cdecl): the static layout interface, built on the
    /// first call (guard bit set, the vtable stored, `atexit(dtor)`).
    /// Returns its address.
    pub fn layout_interface(h: &mut impl Host) -> u32 {
        let guard = h.rebase(LAYOUT_GUARD);
        let g: u32 = h.read(guard);
        if g & 1 == 0 {
            h.write(guard, g | 1);
            let vt = h.rebase(LAYOUT_VTABLE);
            h.write(h.rebase(LAYOUT_STATIC), vt);
            let dtor = h.rebase(LAYOUT_DTOR);
            h.call_sym(ATEXIT, Conv::Cdecl, &[dtor]);
        }
        h.rebase(LAYOUT_STATIC)
    }

    /// `0x0061c2c0(this; n)` (thiscall, `ret 4`): the stream slots' setup.
    pub const STREAM_SLOTS_INIT: u32 = 0x0061_c2c0;
    /// `ADVAPI32!CryptAcquireContextA` and
    /// `KERNEL32!InitializeCriticalSection` (stdcall).
    pub const CRYPT_ACQUIRE_IAT: u32 = 0x00b0_c018;
    pub const INIT_CRITICAL_SECTION_IAT: u32 = 0x00b0_c150;

    /// `BigFileStream_ctor(this)` (thiscall): `+0x3c`/`+0x40` = 0,
    /// `0x0061c2c0(this + 4; 0x1e)`, the five handles `+0x10..+0x20` and
    /// `+0` = 0, `CryptAcquireContextA(this, 0, 0, PROV_RSA_FULL,
    /// CRYPT_VERIFYCONTEXT)`, the critical section `+0x24` initialised;
    /// its `eax`.
    pub fn big_file_stream_ctor(h: &mut impl Host, this: u32) -> u32 {
        let at = |off: u32| this.wrapping_add(off);
        h.write(at(0x3c), 0u32);
        h.write(at(0x40), 0u32);
        h.call_sym(STREAM_SLOTS_INIT, Conv::Thiscall(at(4)), &[0x1e]);
        for off in [0x10u32, 0x14, 0x18, 0x1c, 0x20, 0] {
            h.write(at(off), 0u32);
        }
        import(
            h,
            CRYPT_ACQUIRE_IAT,
            Conv::Stdcall,
            &[this, 0, 0, 1, 0xf000_0000],
        );
        import(h, INIT_CRITICAL_SECTION_IAT, Conv::Stdcall, &[at(0x24)])
    }

    /// `operator new[]` (`0x006304c0`, cdecl),
    /// `eh_vector_constructor_iterator(p, size, count, ctor, dtor)`
    /// (stdcall), the stream slot's constructor `0x0061af20`,
    /// `KERNEL32!CreateSemaphoreA` (stdcall).
    pub const OPERATOR_NEW_ARRAY: u32 = 0x0063_04c0;
    pub const EH_VECTOR_CTOR: u32 = 0x0094_966a;
    pub const SLOT_CTOR: u32 = 0x0061_af20;
    pub const CREATE_SEMAPHORE_IAT: u32 = 0x00b0_c11c;

    /// `0x0061c2c0(this; n)` (thiscall, `ret 4`): the stream slots
    /// rebuilt: `+4` = `n`; the old array `+8` (0x60-byte slots with a
    /// count header) destroyed and freed when set; a new one of `n` slots
    /// (`operator new[](n * 0x60 + 4)`, saturating to 0xffffffff on
    /// overflow as the original's `seto`/`setb`; the count written, the
    /// slots constructed; null when `new` fails) at `+8`; the old
    /// semaphore `+0` closed and cleared when set, `+0` =
    /// `CreateSemaphoreA(0, n, n, 0)`, its `eax` returned.
    pub fn stream_slots_init(h: &mut impl Host, this: u32, n: u32) -> u32 {
        h.write(this.wrapping_add(4), n);
        let old: u32 = h.read(this.wrapping_add(8));
        if old != 0 {
            delete_vector(h, old, 0x60, SLOT_DTOR);
        }
        let n: u32 = h.read(this.wrapping_add(4));
        let bytes = n
            .checked_mul(0x60)
            .and_then(|b| b.checked_add(4))
            .unwrap_or(u32::MAX);
        let block = h.call_sym(OPERATOR_NEW_ARRAY, Conv::Cdecl, &[bytes]);
        let slots = if block == 0 {
            0
        } else {
            h.write(block, n);
            let ctor = h.rebase(SLOT_CTOR);
            let dtor = h.rebase(SLOT_DTOR);
            let first = block.wrapping_add(4);
            h.call_sym(EH_VECTOR_CTOR, Conv::Stdcall, &[first, 0x60, n, ctor, dtor]);
            first
        };
        h.write(this.wrapping_add(8), slots);
        let sem: u32 = h.read(this);
        if sem != 0 {
            import(h, CLOSE_HANDLE_IAT, Conv::Stdcall, &[sem]);
            h.write(this, 0u32);
        }
        let n: u32 = h.read(this.wrapping_add(4));
        let s = import(h, CREATE_SEMAPHORE_IAT, Conv::Stdcall, &[0, n, n, 0]);
        h.write(this, s);
        s
    }

    /// `BigFileStream_createInstance()` (cdecl): the big-file stream
    /// created once (`operator new(0x44)`, `+4`/`+8`/`+0xc` = 0,
    /// `BigFileStream_ctor`; 0 when `new` fails) at
    /// `BigFileStream_sBigFileStreamInstacne`. Returns the constructor's
    /// `eax`, 0 when `new` fails, 0 when the stream exists (the
    /// original's `eax` is the cookie's xor there, junk to its caller).
    pub fn big_file_stream_create(h: &mut impl Host) -> u32 {
        let cell = h.rebase(BIG_FILE_STREAM);
        if h.read::<u32>(cell) != 0 {
            return 0;
        }
        let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x44]);
        let r = if p == 0 {
            0
        } else {
            for off in [4u32, 8, 0xc] {
                h.write(p.wrapping_add(off), 0u32);
            }
            big_file_stream_ctor(h, p)
        };
        h.write(cell, p);
        r
    }

    /// The key order of the VBF digest tree: `(+0x14 signed, +0x10
    /// unsigned)`, then `(+0x1c signed, +0x18 unsigned)`.
    fn digest_less(h: &impl Host, a: u32, b: u32) -> bool {
        let key = |n: u32, off: u32| -> (i32, u32) {
            (
                h.read::<u32>(n.wrapping_add(off + 4)).cast_signed(),
                h.read::<u32>(n.wrapping_add(off)),
            )
        };
        (key(a, 0x10), key(a, 0x18)) < (key(b, 0x10), key(b, 0x18))
    }

    /// `0x0061d210(this; node)` (thiscall, `ret 4`): `node` linked into
    /// the digest tree (`PRedBlackTree` layout: the head is the tree,
    /// root `+0xc`; a node's children `+0`/`+4` (less/greater; the head
    /// ends a branch), its parent and colour bit `+8`). The node's colour
    /// bit is set first (`+8 += 1 - (+8 & 1)`) and both children point at
    /// the head; walking from the root by [`digest_less`], an equal key
    /// returns that node (the new one left half set up). Else the node is
    /// hung under the last node visited (or as the root: parent `this +
    /// 0xc`), `+8` = that parent plus the colour bit, and 0 is returned.
    pub fn digest_tree_link(h: &mut impl Host, this: u32, node: u32) -> u32 {
        let flags: u32 = h.read(node.wrapping_add(8));
        let bit = u32::from(h.read::<u8>(node.wrapping_add(8)) & 1);
        h.write(node.wrapping_add(8), flags.wrapping_add(1 - bit));
        h.write(node, this);
        h.write(node.wrapping_add(4), this);
        let mut parent = this.wrapping_add(0xc);
        let mut side = 0u32;
        let mut cur: u32 = h.read(parent);
        if cur != this {
            loop {
                parent = cur;
                if digest_less(h, cur, node) {
                    side = 1;
                } else if digest_less(h, node, cur) {
                    side = 0;
                } else {
                    return cur;
                }
                cur = h.read(cur.wrapping_add(4 * side));
                if cur == this {
                    break;
                }
            }
        }
        let bit = u32::from(h.read::<u8>(node.wrapping_add(8)) & 1);
        h.write(node.wrapping_add(8), bit.wrapping_add(parent));
        h.write(parent.wrapping_add(4 * side), node);
        0
    }

    /// `Phyre_PFreeList_allocateNewBlock(this; size)`,
    /// `Phyre_PFreeList_deallocate(this; p)` and
    /// `Phyre_PRedBlackTree_postInsertFinalize(this; node)` (thiscall,
    /// `ret 4`).
    pub const FREE_LIST_NEW_BLOCK: u32 = 0x0049_c8d0;
    pub const FREE_LIST_DEALLOCATE: u32 = 0x0049_c980;
    pub const TREE_POST_INSERT: u32 = 0x0057_4980;

    /// `Vbf_InsertEntry(this; &digest, &entry)` (thiscall, `ret 8`): a
    /// node from the free list `+0x18` (a new block of `[+0x20]` when it
    /// is empty; none: 0xe), self-linked, the 16-byte digest at `+0x10`,
    /// the entry word at `+0x20`, linked ([`digest_tree_link`]); a
    /// duplicate: the node back to the free list, 0x12; else the tree's
    /// insert fix-up, 0.
    pub fn vbf_insert_entry(h: &mut impl Host, this: u32, digest: u32, entry: u32) -> u32 {
        let list = this.wrapping_add(0x18);
        let mut node: u32 = h.read(list);
        if node == 0 {
            let size: u32 = h.read(this.wrapping_add(0x20));
            h.call_sym(FREE_LIST_NEW_BLOCK, Conv::Thiscall(list), &[size]);
            node = h.read(list);
            if node == 0 {
                return 0xe;
            }
        }
        let next: u32 = h.read(node);
        h.write(list, next);
        for off in [0u32, 4, 8] {
            h.write(node.wrapping_add(off), node);
        }
        for k in 0..4u32 {
            let w: u32 = h.read(digest.wrapping_add(4 * k));
            h.write(node.wrapping_add(0x10 + 4 * k), w);
        }
        let e: u32 = h.read(entry);
        h.write(node.wrapping_add(0x20), e);
        if digest_tree_link(h, this, node) != 0 {
            h.call_sym(FREE_LIST_DEALLOCATE, Conv::Thiscall(list), &[node]);
            return 0x12;
        }
        h.call_sym(TREE_POST_INSERT, Conv::Thiscall(this), &[node]);
        0
    }
}
