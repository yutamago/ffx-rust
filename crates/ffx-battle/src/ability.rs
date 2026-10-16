//! The commands an actor may use (`docs/rewrite-plan.md`, M2): the save's
//! learned bits per character (`ply_save` `+0x3e`, the shared ones at
//! `0x011307fc`), copied to the actor's `+0x664` (`+0x670` shared) and
//! completed by what the inventory allows, and the battle menu's command
//! lists (`g_BtlItemMem2`, 0x478 bytes per character) sorted by the
//! commands' `+0x5c`.

/// The game's memory, through [`ffx_bridge::Mem`].
#[allow(clippy::many_single_char_names)] // words, bits, keys
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    use crate::actor::bridge::ms_get_chr;
    use crate::misc::bridge::ITEM_MEM2;
    use crate::save::bridge::{EQUIPMENT, ITEM_COUNTS, ITEM_IDS, PLY_SAVE, SHARED_COMMANDS};

    /// The aeons' save records (10 of 0x94 bytes; bit 0 of the first byte:
    /// the aeon is available).
    pub const SUMMON_SAVE: Global<u8> = Global::new(0x0113_2528);
    /// `MsGetBattleList`'s list (31 bytes, 0xff after the actors).
    pub const BATTLE_LIST: Global<u8> = Global::new(0x0112_c876);
    /// The party's battle slots.
    pub const PARTY: Global<[u8; 7]> = Global::new(0x0112_c895);
    /// The bytes after the party (17).
    pub const PARTY_EXTRA: Global<[u8; 17]> = Global::new(0x0112_c8a3);
    /// `0x0112c9e9`: a party slot `MsGetBattleList` treats apart (signed).
    pub const LIST_SLOT: Global<i8> = Global::new(0x0112_c9e9);
    /// [`ms_sort_command`]'s sort keys (inside `g_BtlItemMem2`).
    pub const SORT_KEYS: u32 = 0x5070;

    fn rom_command(mem: &mut impl Mem, i: u32) -> u32 {
        ffx_kernel::bridge::rom(mem, ffx_kernel::bridge::COMMAND, i, 0)
    }

    /// The word and bit of a command index (12 bits) in a bit set.
    fn bit(id: u32) -> (u32, u16) {
        let i = id & 0xfff;
        (i >> 4, 1 << (i & 0xf))
    }

    /// `MsGetChrCommandFlag(chr, id)` (`0x0079ad30`): bit 0 when actor
    /// `chr` may use command `id` (`+0x664` bit set), bit 1 its `+0x690`
    /// set.
    pub fn ms_get_chr_command_flag(mem: &impl Mem, chr: u32, id: u32) -> u32 {
        let a = ms_get_chr(mem, chr);
        let (w, b) = bit(id);
        let get = |o: u32| mem.read::<u16>(a.wrapping_add(o + 2 * w)) & b != 0;
        u32::from(get(0x664)) | (u32::from(get(0x690)) << 1)
    }

    /// `MsSetRamChrCommand(chr, id, on)` (`0x0079c070`): sets or clears
    /// actor `chr`'s `+0x664` bit of command `id`.
    pub fn ms_set_chr_command_flag(mem: &mut impl Mem, chr: u32, id: u32, on: u32) {
        let a = ms_get_chr(mem, chr);
        let (w, b) = bit(id);
        let at = a.wrapping_add(0x664 + 2 * w);
        let v = mem.read::<u16>(at);
        mem.write(at, if on != 0 { v | b } else { v & !b });
    }

    /// `MsSortCommandList(list, keys, n)` (`0x007ad970`): sorts `n`
    /// command IDs (`u16`) by their records' `+0x5c` (0x100 for 0xff and
    /// non-command IDs), keys kept in `keys`; a bubble sort, so equal keys
    /// keep their order.
    pub fn ms_sort_command_list(mem: &mut impl Mem, list: u32, keys: u32, n: i32) {
        let n = n.max(0).cast_unsigned();
        for i in 0..n {
            let id = u32::from(mem.read::<u16>(list.wrapping_add(2 * i)));
            let key = if id != 0xff && id & 0xffff_f000 == 0x3000 {
                let r = rom_command(mem, id);
                u16::from(mem.read::<u8>(r.wrapping_add(0x5c)))
            } else {
                0x100
            };
            mem.write(keys.wrapping_add(2 * i), key);
        }
        if n < 2 {
            return;
        }
        loop {
            let mut swapped = false;
            for i in 0..n - 1 {
                let (ka, kb) = (keys.wrapping_add(2 * i), keys.wrapping_add(2 * i + 2));
                let (x, y) = (mem.read::<u16>(ka), mem.read::<u16>(kb));
                if x > y {
                    mem.write(ka, y);
                    mem.write(kb, x);
                    let (la, lb) = (list.wrapping_add(2 * i), list.wrapping_add(2 * i + 2));
                    let (p, q) = (mem.read::<u16>(la), mem.read::<u16>(lb));
                    mem.write(lb, p);
                    mem.write(la, q);
                    swapped = true;
                }
            }
            if !swapped {
                return;
            }
        }
    }

    /// The command lists of a character's 0x478 bytes: offset and length.
    pub const LISTS: [(u32, i32); 8] = [
        (0, 0x14),
        (0x28, 8),
        (0x38, 8),
        (0x48, 0x18),
        (0x78, 0x18),
        (0xa8, 0x20),
        (0xe8, 0x20),
        (0x128, 0x18),
    ];

    /// `MsSortCommand(lists)` (`0x007aefb0`): sorts a character's eight
    /// command lists ([`LISTS`]).
    pub fn ms_sort_command(mem: &mut impl Mem, lists: u32) {
        let keys = ITEM_MEM2.get(mem).wrapping_add(SORT_KEYS);
        for (o, n) in LISTS {
            ms_sort_command_list(mem, lists.wrapping_add(o), keys.wrapping_add(o), n);
        }
    }

    /// Present for the lists: `+0x10`, not `+0xdcd`, and `+0xdc8` or
    /// `+0xdca`.
    fn present(mem: &impl Mem, a: u32) -> bool {
        let b = |o: u32| mem.read::<u8>(a.wrapping_add(o)) != 0;
        b(0x10) && !b(0xdcd) && (b(0xdc8) || b(0xdca))
    }

    /// `MsGetBattleList(kind, &count)` (`0x007b6c50`): a list of actor
    /// indices ([`BATTLE_LIST`], filled up with 0xff to 31): 5 the party
    /// actors (`+0x10`) that are aeons, 8 all party actors, 9 the present
    /// party by battle slot then the other present ones (and [`LIST_SLOT`]),
    /// 0x10 those and the present monsters, 0xa the extra slots' present
    /// actors but [`LIST_SLOT`] then the first 8, 0xb the present
    /// monsters, 0x13 the monsters with an ID and the cursor flag; others
    /// empty. The count into `count` when not null.
    #[allow(clippy::too_many_lines)]
    pub fn ms_get_battle_list<M: Mem>(mem: &mut M, kind: i32, count: u32) -> u32 {
        use crate::actor::bridge::ms_get_mon;
        use crate::damage::bridge::ms_get_ram_chr_summon;
        let list = BATTLE_LIST.ptr(mem).addr;
        let mut n = 0u32;
        let push = |mem: &mut M, n: &mut u32, v: u32| {
            mem.write(list.wrapping_add(*n), v.to_le_bytes()[0]);
            *n += 1;
        };
        let listed = |mem: &M, n: u32, v: u32| {
            (0..n.cast_signed().max(0).cast_unsigned())
                .any(|k| u32::from(mem.read::<u8>(list.wrapping_add(k))) == v)
        };
        let slot = i32::from(LIST_SLOT.get(mem));
        let monsters = |mem: &mut M, n: &mut u32| {
            for i in 0..8 {
                let a = ms_get_mon(mem, i);
                if present(mem, a) {
                    push(mem, n, i + 0x14);
                }
            }
        };
        match kind {
            5 | 8 => {
                for i in 0..0x12 {
                    let a = ms_get_chr(mem, i);
                    if mem.read::<u8>(a.wrapping_add(0x10)) != 0
                        && (kind == 8 || ms_get_ram_chr_summon(mem, i) != 0)
                    {
                        push(mem, &mut n, i);
                    }
                }
            }
            9 | 0x10 => {
                let party = PARTY.get(mem);
                for c in party {
                    let c = u32::from(c);
                    let a = ms_get_chr(mem, c);
                    if c != 0xff && present(mem, a) {
                        push(mem, &mut n, c);
                    }
                }
                for i in 0..0x12 {
                    let a = ms_get_chr(mem, i);
                    if (present(mem, a) || slot == i.cast_signed()) && !listed(mem, n, i) {
                        push(mem, &mut n, i);
                    }
                }
                if kind == 0x10 {
                    monsters(mem, &mut n);
                }
            }
            0xa => {
                let extra = PARTY_EXTRA.get(mem);
                for (k, c) in (0i32..).zip(extra) {
                    let c = u32::from(c);
                    let a = ms_get_chr(mem, c);
                    if c != 0xff && present(mem, a) && slot != k {
                        push(mem, &mut n, c);
                    }
                }
                for i in 0..8 {
                    let a = ms_get_chr(mem, i);
                    if present(mem, a) && slot != i.cast_signed() && !listed(mem, n, i) {
                        push(mem, &mut n, i);
                    }
                }
            }
            0xb => monsters(mem, &mut n),
            0x13 => {
                for i in 0..8 {
                    let a = ms_get_mon(mem, i);
                    if mem.read::<u16>(a.wrapping_add(0xe)) != 0xffff
                        && mem.read::<u8>(a.wrapping_add(0xdd9)) != 0
                    {
                        push(mem, &mut n, i + 0x14);
                    }
                }
            }
            _ => {}
        }
        for k in n..0x1f {
            mem.write(list.wrapping_add(k), 0xffu8);
        }
        if count != 0 {
            mem.write(count, n);
        }
        list
    }

    /// Whether the inventory allows a command group (a command's `+0x17`
    /// when its `+0x16` & 0xf8 is 8): 5 an aeon available, 6 and 0x11 an
    /// item owned, 7 and 0xf a weapon (`+5` 0) or armor (`+5` 1) of the
    /// character's in use and not `+3` bit 2, 0xa anyone in the battle
    /// list of kind 0xa; else `learned`.
    fn group_allowed(mem: &mut impl Mem, chr: u32, group: u32, learned: bool) -> bool {
        match group {
            5 => {
                (0..10).any(|i| {
                    let at = SUMMON_SAVE.ptr(mem).addr.wrapping_add(i * 0x94);
                    mem.read::<u8>(at) & 1 != 0
                }) || learned
            }
            6 | 0x11 => {
                let ids = ITEM_IDS.ptr(mem).addr;
                let counts = ITEM_COUNTS.ptr(mem).addr;
                learned
                    || (0..0x70).any(|i| {
                        mem.read::<u16>(ids.wrapping_add(2 * i)) != 0
                            && mem.read::<u8>(counts.wrapping_add(i)) != 0
                    })
            }
            7 | 0xf => {
                let kind = u8::from(group == 0xf);
                let base = EQUIPMENT.ptr(mem).addr;
                (0..200).any(|i| {
                    let s = base.wrapping_add(i * 0x16);
                    let b = |o: u32| mem.read::<u8>(s.wrapping_add(o));
                    b(2) != 0 && b(5) == kind && u32::from(b(4)) == chr && b(3) & 2 == 0
                }) || learned
            }
            0xa => {
                let mut f = ffx_bridge::frame::Frame::new(mem, ABILITY_FRAME, 4);
                let at = f.at(0);
                ms_get_battle_list(&mut f, 0xa, at);
                f.read::<u32>(at) != 0
            }
            _ => learned,
        }
    }

    /// [`group_allowed`]'s count (on the original's stack).
    pub const ABILITY_FRAME: u32 = 0xff60_0000;

    /// `MsSetRamChrAbility(chr, actor)` (`0x0079bb50`): the actor's usable
    /// commands from character `chr`'s save (`ply_save` `+0x3e`, 6 words,
    /// to `+0x664`; the shared 16 to `+0x670`). A group command (`+0x16`
    /// & 0xf8 == 8) is usable when a learned member (`+0x16` & 7 == 0,
    /// `+0x18` its group) or the inventory allows its group; a command of
    /// another character (`+0x19`, 0xff: anyone but `chr` 0xe and the
    /// sisters) is not. Then each of the 0x140 commands enters or leaves
    /// the character's menu list for its kind (`g_BtlItemMem2`, the
    /// command's `+0x1c` bits 0x1000/0x800 or `+0x18`) by its usability,
    /// the lists are sorted, and the actor's `+0x6cb` extra commands
    /// (`+0x71a`) are inserted at their positions (`+0x722`) into the
    /// second list.
    #[allow(clippy::too_many_lines)]
    pub fn ms_set_ram_chr_ability(mem: &mut impl Mem, chr: u32, actor: u32) {
        let save = PLY_SAVE.ptr(mem).addr.wrapping_add(chr.wrapping_mul(0x94));
        for k in 0..6 {
            let v = mem.read::<u16>(save.wrapping_add(0x3e + 2 * k));
            mem.write(actor.wrapping_add(0x664 + 2 * k), v);
        }
        let shared = SHARED_COMMANDS.ptr(mem).addr;
        for k in 0..16 {
            let v = mem.read::<u16>(shared.wrapping_add(2 * k));
            mem.write(actor.wrapping_add(0x670 + 2 * k), v);
        }
        let rikku_like = chr == 0xe;
        let sister = matches!(chr, 0xf..=0x11);
        for i in 0..0x140 {
            let r = rom_command(mem, i);
            if mem.read::<u8>(r.wrapping_add(0x16)) & 0xf8 == 8 {
                let group = mem.read::<u8>(r.wrapping_add(0x17));
                let mut learned = false;
                for j in 0..0x140 {
                    let m = rom_command(mem, j);
                    if mem.read::<u8>(m.wrapping_add(0x16)).trailing_zeros() >= 3
                        && mem.read::<u8>(m.wrapping_add(0x18)) == group
                        && ms_get_chr_command_flag(mem, chr, j) & 1 != 0
                    {
                        learned = true;
                        break;
                    }
                }
                let on = group_allowed(mem, chr, u32::from(group), learned);
                ms_set_chr_command_flag(mem, chr, i, u32::from(on));
            }
            let owner = mem.read::<u8>(r.wrapping_add(0x19));
            let other = if owner == 0xff {
                rikku_like || sister
            } else {
                u32::from(owner) != chr
            };
            if other {
                ms_set_chr_command_flag(mem, chr, i, 0);
            }
        }
        let base = ITEM_MEM2.get(mem);
        let own = chr.wrapping_mul(0x478);
        for i in 0..0x140u32 {
            let id = 0x3000 + i;
            let r = rom_command(mem, i);
            let flags: u32 = mem.read(r.wrapping_add(0x1c));
            let (list, n) = if mem.read::<u8>(r.wrapping_add(0x16)) & 7 != 0 {
                if flags & 0x1000 != 0 {
                    (base.wrapping_add(0x28).wrapping_add(own), 8)
                } else if flags & 0x800 != 0 {
                    (base.wrapping_add(0x38).wrapping_add(own), 8)
                } else {
                    (own.wrapping_add(base), 0x14)
                }
            } else {
                match mem.read::<u8>(r.wrapping_add(0x18)) {
                    1 => (base.wrapping_add(0x78).wrapping_add(own), 0x18),
                    2 => (base.wrapping_add(0x48).wrapping_add(own), 0x18),
                    3 => (base.wrapping_add(0xa8).wrapping_add(own), 0x20),
                    4 => (base.wrapping_add(0x128).wrapping_add(own), 0x18),
                    0xe => (base.wrapping_add(0xe8).wrapping_add(own), 0x20),
                    _ => (0, 0),
                }
            };
            let mut usable = (ms_get_chr_command_flag(mem, chr, i) & 1).cast_signed();
            if list == 0 {
                continue;
            }
            let mut free = -1i32;
            for k in 0..n {
                let at = list.wrapping_add(2 * k);
                let v = u32::from(mem.read::<u16>(at));
                if v == 0xff && free < 0 {
                    free = k.cast_signed();
                }
                if v == id {
                    if usable == 0 {
                        mem.write(at, 0xffu16);
                    } else {
                        usable = -1;
                    }
                }
            }
            if usable > 0 && free >= 0 {
                #[allow(clippy::cast_possible_truncation)] // below 0x3140
                mem.write(list.wrapping_add(2 * free.cast_unsigned()), id as u16);
            }
        }
        ms_sort_command(mem, base.wrapping_add(own));
        let extra = u32::from(mem.read::<u8>(actor.wrapping_add(0x6cb)));
        for c in 0..extra {
            let pos = u32::from(mem.read::<u8>(actor.wrapping_add(0x722 + c)));
            let v = mem.read::<u16>(actor.wrapping_add(0x71a + 2 * c));
            if pos < 7 {
                let base = ITEM_MEM2.get(mem);
                for k in (pos..7).rev() {
                    let from = base.wrapping_add(own).wrapping_add(0x28 + 2 * k);
                    let w = mem.read::<u16>(from);
                    mem.write(from.wrapping_add(2), w);
                }
            }
            let base = ITEM_MEM2.get(mem);
            mem.write(base.wrapping_add(own).wrapping_add(0x28 + 2 * pos), v);
        }
    }

    /// `MsSetSaveCommand(chr, id, on)` (`0x00785c50`): learns (or forgets)
    /// command `id` (0x3000-0x3fff; 1, else 0) in character `chr`'s save
    /// bits, or in the shared ones from 0x3060, then refreshes the actor
    /// ([`ms_set_ram_chr_ability`]) when a battle is running.
    pub fn ms_set_save_command(mem: &mut impl Mem, chr: u32, id: u32, on: u32) -> u32 {
        if id & 0xffff_f000 != 0x3000 {
            return 0;
        }
        let (bits, i) = if id.cast_signed() < 0x3060 {
            let save = PLY_SAVE.ptr(mem).addr.wrapping_add(chr.wrapping_mul(0x94));
            (save.wrapping_add(0x3e), id & 0xfff)
        } else {
            (SHARED_COMMANDS.ptr(mem).addr, id.wrapping_sub(0x60) & 0xfff)
        };
        let (w, b) = bit(i);
        let at = bits.wrapping_add(2 * w);
        let v = mem.read::<u16>(at) & !b;
        mem.write(at, if on != 0 { v | b } else { v });
        if crate::actor::bridge::ms_get_chr_top(mem) != 0 {
            let a = ms_get_chr(mem, chr);
            ms_set_ram_chr_ability(mem, chr, a);
        }
        1
    }
}
