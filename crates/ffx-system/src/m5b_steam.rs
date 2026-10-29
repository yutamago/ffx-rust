//! M5 `system-b`: the Steam callback objects of `StatsAndAchievements`
//! (`CCallback<StatsAndAchievements, T>`: `+0` the vtable, `+4` the
//! registration flags, `+0xc` the object, `+0x10` the member function)
//! and the stats-received handler that fills the achievement records
//! (`g_AchievementRecords` layout: `ffx_system::m5a_platform`).

/// `steam_api` imports: `SteamAPI_UnregisterCallback(cb)`,
/// `SteamUserStats()` (cdecl); `MSVCR110` `_snprintf` (cdecl).
pub const UNREGISTER_IAT: u32 = 0x00b0_c8a0;
pub const USER_STATS_IAT: u32 = 0x00b0_c898;
pub const SNPRINTF_IAT: u32 = 0x00b0_c42c;
/// `"name"`, `"desc"`, `"%s"`.
pub const NAME: u32 = 0x00b4_ba1c;
pub const DESC: u32 = 0x00b4_ba24;
pub const PERCENT_S: u32 = 0x00b0_e160;
/// The three callbacks' vtables: `UserAchievementStored_t`,
/// `UserStatsReceived_t`, `UserStatsStored_t`.
pub const ACHIEVEMENT_STORED_VFTABLE: u32 = 0x00b4_ba10;
pub const STATS_RECEIVED_VFTABLE: u32 = 0x00b4_b9f0;
pub const STATS_STORED_VFTABLE: u32 = 0x00b4_ba00;

/// The originals over game memory.
pub mod bridge {
    use super::{DESC, NAME, PERCENT_S, SNPRINTF_IAT, UNREGISTER_IAT, USER_STATS_IAT};
    use ffx_graphic::host::{Conv, Host};

    fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, Conv::Cdecl, args)
    }

    /// A `CCallback`'s destructor `(this)` (`0x006f1310`, `0x006f1330`,
    /// `0x006f1350`, thiscall; `vftable` its own): the vtable, and
    /// `SteamAPI_UnregisterCallback(this)` while registered (`+4` bit 0).
    pub fn callback_dtor(h: &mut impl Host, this: u32, vftable: u32) {
        let flags: u8 = h.read(this.wrapping_add(4));
        h.write(this, h.rebase(vftable));
        if flags & 1 != 0 {
            import(h, UNREGISTER_IAT, &[this]);
        }
    }

    /// `CCallback::Run(this; param)` (`0x006f1820`, `0x006f1850`,
    /// `0x006f1880`: a tail jump): the member function at `+0x10` on the
    /// object at `+0xc` with `param` (thiscall, one word); its `eax`.
    pub fn callback_run(h: &mut impl Host, this: u32, param: u32) -> u32 {
        let obj: u32 = h.read(this.wrapping_add(0xc));
        let f: u32 = h.read(this.wrapping_add(0x10));
        h.call(f, Conv::Thiscall(obj), &[param])
    }

    /// `CCallback::Run(this; param, ioFailure, apiCall)` (`0x006f1830`,
    /// `0x006f1860`, `0x006f1890`, `ret 0x10`): as [`callback_run`].
    pub fn callback_run_call(h: &mut impl Host, this: u32, param: u32) -> u32 {
        callback_run(h, this, param)
    }

    /// `StatsAndAchievements_OnUserStatsReceived(this; cb)` (`0x006f15f0`,
    /// thiscall, `ret 4`): for this game (`+0x3c`/`+0x40`) and a result
    /// of 1, `+0x4c` = 1 and each of the `+0x48` records (`+0x44`, 0x190
    /// bytes each: `+4` the API name) gets `GetAchievement(name, +0x188)`
    /// and the display attributes "name" and "desc" copied with
    /// `_snprintf("%s")` into `+8` (0x80) and `+0x88` (0x100).
    pub fn on_user_stats_received(h: &mut impl Host, this: u32, cb: u32) {
        let id_lo: u32 = h.read(this.wrapping_add(0x3c));
        if id_lo != h.read::<u32>(cb) {
            return;
        }
        let id_hi: u32 = h.read(this.wrapping_add(0x40));
        if id_hi != h.read::<u32>(cb.wrapping_add(4)) || h.read::<u32>(cb.wrapping_add(8)) != 1 {
            return;
        }
        h.write(this.wrapping_add(0x4c), 1u8);
        let mut i = 0i32;
        let mut off = 0u32;
        while i < h.read::<u32>(this.wrapping_add(0x48)).cast_signed() {
            let records: u32 = h.read(this.wrapping_add(0x44));
            let rec = records.wrapping_add(off);
            let stats = import(h, USER_STATS_IAT, &[]);
            let api: u32 = h.read(rec.wrapping_add(4));
            h.call_virtual(stats, 6, &[api, rec.wrapping_add(0x188)]);
            for (key, dst, n) in [(NAME, 8u32, 0x80u32), (DESC, 0x88, 0x100)] {
                let stats = import(h, USER_STATS_IAT, &[]);
                let api: u32 = h.read(rec.wrapping_add(4));
                let k = h.rebase(key);
                let text = h.call_virtual(stats, 12, &[api, k]);
                let fmt = h.rebase(PERCENT_S);
                import(h, SNPRINTF_IAT, &[rec.wrapping_add(dst), n, fmt, text]);
            }
            i += 1;
            off = off.wrapping_add(0x190);
        }
    }
}
