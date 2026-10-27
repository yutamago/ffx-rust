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
}
