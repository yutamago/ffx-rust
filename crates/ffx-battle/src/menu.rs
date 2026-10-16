//! The battle menu's lookups (`docs/rewrite-plan.md`, M2): status and
//! weapon names from the kernel tables, the characters' saved names, the
//! AP to the next level, the skip command, and how a battle ended.

/// `MsGetNextAP`'s curve: the AP from level `n` to `n + 1` (`n` at most
/// 100) with a character's three factors (`+0x11` cubic, `+0x12`
/// quadratic, `+0x13` linear); integer divisions truncate.
#[must_use]
pub fn next_ap(n: i32, cubic: i32, quadratic: i32, linear: i32) -> i32 {
    linear * (n + 1) + cubic * n * n * n / 100 + quadratic * n * n / 10
}

/// The game's memory, through [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::{Global, Mem};
    use ffx_kernel::bridge::{excel_data, ms_get_excel_data};

    use crate::actor::bridge::{ms_btl_chr_num_check, ms_get_chr, ms_get_mon};

    /// The characters' kernel table (`ply_rom`).
    pub const PLY_ROM: Global<u32> = Global::new(0x0112_a938);
    /// The status kernel table `MsMenuStatusNum` reads.
    pub const STATUS_ROM: Global<u32> = Global::new(0x0112_a958);
    /// The weapon name table `MsWeaponName` reads.
    pub const WEAPON_NAMES: Global<u32> = Global::new(0x0113_63b4);
    /// The characters' names, 0x14 bytes each.
    pub const SAVE_NAMES: Global<u8> = Global::new(0x0113_2ddc);
    /// Set: the menu skips every command (`MsMenuGetChrSkipCommand`: 0xff).
    pub const SKIP_ALL: Global<u8> = Global::new(0x0112_a922);
    /// Set: each actor's skip command (`+0x6c8`) counts.
    pub const SKIP_ON: Global<u8> = Global::new(0x0112_ca00);
    /// The party's seven battle slots (actor indices, 0xff empty).
    pub const PARTY: Global<[u8; 7]> = Global::new(0x0112_c895);
    /// Set: winning ends as escape kind 2 rather than 4
    /// (`MsGetBattleEndStatus`).
    pub const END_ALT: Global<u8> = Global::new(0x0112_c9eb);
    /// Forces `MsGetBattleEndStatus`'s result when not 0.
    pub const END_FORCED: Global<u8> = Global::new(0x0112_c9f1);

    /// `MsGetNextAP(chr)` (`0x00784e90`): [`super::next_ap`] at the
    /// character's level (the sum of its `g_PlySave` bytes `+0x3b` and
    /// `+0x3c`), or its record's
    /// `+0x14` past 100.
    pub fn ms_get_next_ap(mem: &mut impl Mem, chr: i32) -> i32 {
        let table = PLY_ROM.get(mem);
        let r = ms_get_excel_data(mem, chr, table, 0);
        let save = crate::save::bridge::PLY_SAVE
            .ptr(mem)
            .addr
            .wrapping_add(chr.cast_unsigned().wrapping_mul(0x94));
        let n = i32::from(mem.read::<u8>(save.wrapping_add(0x3c)))
            + i32::from(mem.read::<u8>(save.wrapping_add(0x3b)));
        if n > 100 {
            return mem.read(r.wrapping_add(0x14));
        }
        let f = |o: u32| i32::from(mem.read::<u8>(r.wrapping_add(o)));
        super::next_ap(n, f(0x11), f(0x12), f(0x13))
    }

    /// `MsGetSaveChrName(chr)` (`0x00784fb0`): character `chr`'s name
    /// (`chr` whole, after checking its low byte), null past the actors.
    pub fn ms_get_save_chr_name(mem: &impl Mem, chr: u32) -> u32 {
        if ms_btl_chr_num_check(chr) == 0 {
            return 0;
        }
        SAVE_NAMES
            .ptr(mem)
            .addr
            .wrapping_add(chr.wrapping_mul(0x14))
    }

    /// `MsMenuStatusNum(id, strings)` (`0x00790200`): the status record.
    pub fn ms_menu_status_num(mem: &mut impl Mem, id: i32, strings: u32) -> u32 {
        let t = STATUS_ROM.get(mem);
        ms_get_excel_data(mem, id, t, strings)
    }

    /// `MsWeaponName(id, k, second, &icon)` (`0x007a0c50`): weapon `id`'s
    /// (12 bits) `k`-th (a byte) name: the strings plus the `u16` at
    /// `+4k`, or `+0x1c + 4k` for `second`; its icon (`+0x38 + 2k`) into
    /// `icon` when not null.
    pub fn ms_weapon_name(mem: &mut impl Mem, id: u32, k: u32, second: u32, icon: u32) -> u32 {
        let k = k & 0xff;
        let t = WEAPON_NAMES.get(mem);
        #[allow(clippy::cast_possible_wrap)] // 12 bits
        let (r, strings) = excel_data(mem, (id & 0xfff) as i32, t);
        if icon != 0 {
            let v: u16 = mem.read(r.wrapping_add(0x38 + 2 * k));
            mem.write(icon, v);
        }
        let o = if second != 0 { 0x1c + 4 * k } else { 4 * k };
        u32::from(mem.read::<u16>(r.wrapping_add(o))).wrapping_add(strings)
    }

    /// `MsMenuGetChrSkipCommand(i)` (`0x0078fb80`): 0xff when everything is
    /// skipped, the actor's skip command when skips count, else 0.
    pub fn ms_menu_get_chr_skip_command(mem: &impl Mem, i: u32) -> u32 {
        if SKIP_ALL.get(mem) != 0 {
            return 0xff;
        }
        if SKIP_ON.get(mem) == 0 {
            return 0;
        }
        let a = ms_get_chr(mem, i);
        u32::from(mem.read::<u16>(a.wrapping_add(0x6c8)))
    }

    /// `MsGetCheckEnglish()` (`0x00784e60`): 0 when the language
    /// (`TkGetLang`, from the caller) is 1, else -1.
    #[must_use]
    pub fn ms_get_check_english(lang: i32) -> i32 {
        if lang == 1 { 0 } else { -1 }
    }

    /// `MsGetBattleEndStatus()` (`0x007928d0`): 1 when no party member
    /// still fights (`+0xdd2`, `+0xdd3`, HP `+0x5d0` above 0, not out
    /// `+0xdcc`/`+0xdce`), 3 when none of those can act (`+0xdc8`), 4 (2
    /// with [`END_ALT`]) when every monster that can act and is in is
    /// gone (`+0xdce`), else 0; [`END_FORCED`] wins when set.
    pub fn ms_get_battle_end_status(mem: &impl Mem) -> u32 {
        let b = |a: u32, o: u32| mem.read::<u8>(a.wrapping_add(o)) != 0;
        let (mut fighting, mut acting) = (0, 0);
        for i in PARTY.get(mem) {
            if i == 0xff {
                continue;
            }
            let a = ms_get_chr(mem, u32::from(i));
            if b(a, 0xdd2)
                && b(a, 0xdd3)
                && mem.read::<i32>(a.wrapping_add(0x5d0)) > 0
                && !b(a, 0xdcc)
                && !b(a, 0xdce)
            {
                fighting += 1;
                if b(a, 0xdc8) {
                    acting += 1;
                }
            }
        }
        let r = if fighting == 0 {
            1
        } else if acting == 0 {
            3
        } else {
            let mut gone = true;
            for k in 0..8 {
                let m = ms_get_mon(mem, k);
                if b(m, 0xdc8) && b(m, 0xdd2) && !b(m, 0xdcc) {
                    gone &= b(m, 0xdce);
                }
            }
            if !gone {
                0
            } else if END_ALT.get(mem) != 0 {
                2
            } else {
                4
            }
        };
        match END_FORCED.get(mem) {
            0 => r,
            f => u32::from(f),
        }
    }
}
