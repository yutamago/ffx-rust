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
    use ffx_graphic::host::{Conv, Host};

    /// `Phyre_PSerialization_PStreamFile_closeFile` (thiscall) -> `al`.
    pub const CLOSE_FILE: u32 = 0x0060_7d80;
    /// `operator delete` (cdecl).
    pub const OPERATOR_DELETE: u32 = 0x0063_0390;
    /// `fiosClose` (cdecl).
    pub const FIOS_CLOSE: u32 = 0x0067_9510;
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
}
