//! The battle menu's lookups (`docs/rewrite-plan.md`, M2): status and
//! weapon names from the kernel tables, the characters' saved names, the
//! AP to the next level, the skip command, how a battle ended, and the
//! lists the menu's windows show (`MsGetSaveWindow`).

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

    /// The lists [`ms_get_save_window`] builds (`u16` entries, 0xff after
    /// the last): the summons, every actor in, the living party first, the
    /// party's reserve first, the living party able to switch, the living
    /// monsters; and the one-entry list of kinds 0x15 and 0x16.
    pub const SUMMON_LIST: u32 = 0x0112_c268;
    pub const ACTOR_LIST: u32 = 0x0112_c27c;
    pub const LIVING_LIST: u32 = 0x0112_c2ba;
    pub const SWITCH_LIST: u32 = 0x0112_c2f8;
    pub const RESERVE_LIST: u32 = 0x0112_c336;
    pub const MONSTER_LIST: u32 = 0x0112_c376;
    pub const ONE_ENTRY_LIST: u32 = 0x0112_c374;
    /// The battle's item windows (`MsSetItemWindow`) and the save's item
    /// IDs.
    pub const ITEM_WINDOW: u32 = 0x0112_c6b6;
    pub const ITEM_WINDOW2: u32 = 0x0112_c796;
    pub const SAVE_ITEM_IDS: u32 = 0x0113_095c;
    /// Each character's block of menu lists in `g_BtlItemMem2`.
    pub const CHR_LISTS_SIZE: u32 = 0x478;

    /// `MsGetChrStatusBits_0078f930(chr)`: actor `chr`'s (its low byte)
    /// status bits (`u16` `+0x606`). The original loads them into `ax`, so
    /// the upper half of `eax` is the actor pointer's.
    pub fn ms_get_chr_status_bits(mem: &impl Mem, chr: u32) -> u32 {
        let a = ms_get_chr(mem, chr & 0xff);
        (a & 0xffff_0000) | u32::from(mem.read::<u16>(a.wrapping_add(0x606)))
    }

    /// An actor is in the battle (`+0x10`).
    fn present(mem: &impl Mem, a: u32) -> bool {
        mem.read::<u8>(a.wrapping_add(0x10)) != 0
    }

    /// An actor in the battle, not out of it (`+0xdcd`) and alive or able
    /// to act (`+0xdc8`, `+0xdca`).
    fn living(mem: &impl Mem, a: u32) -> bool {
        let byte = |o: u32| mem.read::<u8>(a.wrapping_add(o));
        byte(0x10) != 0 && byte(0xdcd) == 0 && (byte(0xdc8) != 0 || byte(0xdca) != 0)
    }

    /// The list at `list` (an image address) from `entries`, then 0xff up
    /// to `end` entries.
    fn write_list(mem: &mut impl Mem, list: u32, entries: &[u16], end: usize) {
        let at = mem.rebase(list);
        let mut k = 0u32;
        for &v in entries {
            mem.write(at.wrapping_add(2 * k), v);
            k += 1;
        }
        for _ in entries.len()..end {
            mem.write(at.wrapping_add(2 * k), 0xffu16);
            k += 1;
        }
    }

    /// `slots` (actor indices, 0xff empty) that `keep` takes, then the
    /// actors `0..others` it takes that are not listed yet.
    fn party_first(
        mem: &impl Mem,
        slots: &[u8],
        others: u32,
        keep: impl Fn(&dyn Fn(u32) -> u8) -> bool,
    ) -> Vec<u16> {
        let take = |i: u32| {
            let a = ms_get_chr(mem, i);
            keep(&|o| mem.read::<u8>(a.wrapping_add(o)))
        };
        let mut list: Vec<u16> = slots
            .iter()
            .filter(|&&s| s != 0xff && take(s.into()))
            .map(|&s| u16::from(s))
            .collect();
        for i in 0..others {
            #[allow(clippy::cast_possible_truncation)] // below 0x12
            let id = i as u16;
            if take(i) && !list.contains(&id) {
                list.push(id);
            }
        }
        list
    }

    /// `MsGetSaveWindow(chr, kind, &count)` (`0x00785410`): the entries a
    /// menu window lists and how many it shows (into `count` unless null).
    /// Kinds 1-4, 7 and 0xc-0xf are character `chr`'s own lists in
    /// `g_BtlItemMem2` (its low byte, character 0 when that is no actor);
    /// 6, 0x11 and 0x14 the battle's item windows and the save's items;
    /// 0x15 and 0x16 a one-entry list of command 0x3121 or 0x3122. The
    /// others build their list first, 0xff after: 5 the summons (the
    /// actors in that are aeons; at the first of the sisters 0x30ff and no
    /// more), 8 every actor in, 9 the living party slots then the other
    /// living characters, 0x12 the same among those able to switch
    /// (`+0xdd8`), 0xa the party's reserve (in, not out, `+0xdc8` clear)
    /// then the first eight characters alike, 0xb the living monsters
    /// (0x14 on). Any other kind: the character's first list, 0x14.
    #[allow(clippy::too_many_lines)]
    pub fn ms_get_save_window(mem: &mut impl Mem, chr: u32, kind: u32, count: u32) -> u32 {
        use crate::damage::bridge::ms_get_ram_chr_summon;
        let chr = chr & 0xff;
        let chr = if ms_btl_chr_num_check(chr) == 0 {
            0
        } else {
            chr
        };
        let own = crate::misc::bridge::ITEM_MEM2
            .get(mem)
            .wrapping_add(chr.wrapping_mul(CHR_LISTS_SIZE));
        let (list, n) = match kind {
            1 => (own.wrapping_add(0x78), 0x18),
            2 => (own.wrapping_add(0x48), 0x18),
            3 => (own.wrapping_add(0xa8), 0x20),
            4 => (own.wrapping_add(0x128), 0x18),
            7 => (own.wrapping_add(0x158), 200),
            0xc => (own.wrapping_add(0x28), 8),
            0xd => (own.wrapping_add(0x38), 8),
            0xe => (own.wrapping_add(0xe8), 0x20),
            0xf => (own.wrapping_add(0x2e8), 200),
            6 => (mem.rebase(ITEM_WINDOW), 0x14),
            0x11 => (mem.rebase(ITEM_WINDOW2), 0x32),
            0x14 => (mem.rebase(SAVE_ITEM_IDS), 0x70),
            0x15 | 0x16 => {
                let command: u16 = if kind == 0x15 { 0x3121 } else { 0x3122 };
                let at = mem.rebase(ONE_ENTRY_LIST);
                mem.write(at, command);
                (at, 1)
            }
            5 => {
                let mut list = Vec::new();
                for i in 0..0x12 {
                    let a = ms_get_chr(mem, i);
                    if !present(mem, a) || ms_get_ram_chr_summon(mem, i) == 0 {
                        continue;
                    }
                    if crate::actor::bridge::ms_get_ram_chr_megas(mem, i) != 0 {
                        // the original sets its counter to 0x30ff: the
                        // loop ends after this entry
                        list.push(0x30ff);
                        break;
                    }
                    #[allow(clippy::cast_possible_truncation)] // below 0x12
                    list.push(i as u16);
                }
                write_list(mem, SUMMON_LIST, &list, 0x1f);
                (mem.rebase(SUMMON_LIST), 10)
            }
            8 => {
                let list: Vec<u16> = (0..0x12u16)
                    .filter(|&i| present(mem, ms_get_chr(mem, i.into())))
                    .collect();
                write_list(mem, ACTOR_LIST, &list, 0x1f);
                (mem.rebase(ACTOR_LIST), 0x1f)
            }
            9 | 0x12 => {
                let party = PARTY.get(mem);
                let switch = kind == 0x12;
                let list = party_first(mem, &party, 0x12, |byte| {
                    (!switch || byte(0xdd8) != 0)
                        && byte(0x10) != 0
                        && byte(0xdcd) == 0
                        && (byte(0xdc8) != 0 || byte(0xdca) != 0)
                });
                let at = if switch { SWITCH_LIST } else { LIVING_LIST };
                write_list(mem, at, &list, 0x1f);
                (mem.rebase(at), 0x1f)
            }
            0xa => {
                let reserve = crate::ability::bridge::PARTY_EXTRA.get(mem);
                let list = party_first(mem, &reserve, 8, |byte| {
                    byte(0x10) != 0 && byte(0xdc8) == 0 && byte(0xdcd) == 0
                });
                write_list(mem, RESERVE_LIST, &list, 0x1f);
                (mem.rebase(RESERVE_LIST), 0x1f)
            }
            0xb => {
                let list: Vec<u16> = (0..8u16)
                    .filter(|&i| living(mem, ms_get_mon(mem, i.into())))
                    .map(|i| i + 0x14)
                    .collect();
                write_list(mem, MONSTER_LIST, &list, 8);
                (mem.rebase(MONSTER_LIST), 8)
            }
            _ => (own, 0x14),
        };
        if count != 0 {
            mem.write(count, n);
        }
        list
    }

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
