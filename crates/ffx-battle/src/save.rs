//! The save block as the battle code (`Ms*`) reads and writes it
//! (`docs/rewrite-plan.md`, M2).
//!
//! `g_SaveData` (`0x0112ca90`, 0x68c0 bytes) is what a save file holds; the
//! battle code reaches into it through small accessors:
//! - flag arrays of `u16` words, flag `n` in word `n >> 4`, bit `n & 0xf`
//!   (commands learnt and used, key items, items and monsters seen);
//! - the inventory: 0x70 item IDs (`u16`) with their counts (`u8`), 200
//!   equipment slots of 22 bytes (`docs/formats/kernel.md`);
//! - `ply_save`, 18 character records of 0x94 bytes (`0x0113205c`): `+0x2c`
//!   join flags, `+0x3b` SP, `+0x3e` the commands learnt;
//! - after them, 18 × 0x16 words of commands used (`0x01132ac4`).
//!
//! The core is the flag and inventory logic on plain values; [`bridge`]
//! applies it where the game keeps the block.

/// `ply_save` records.
pub const PLAYERS: u32 = 0x12;
/// Bytes per `ply_save` record.
pub const PLY_SAVE_SIZE: u32 = 0x94;
/// Item slots in the inventory.
pub const ITEM_SLOTS: u32 = 0x70;
/// Equipment slots in the inventory.
pub const EQUIPMENT_SLOTS: u32 = 200;
/// Bytes per equipment slot.
pub const EQUIPMENT_SIZE: u32 = 22;
/// Gil never exceeds this (`MsDebugAddGill`).
pub const GIL_MAX: u32 = 999_999_899;
/// What `MsDebugFullGill` sets.
pub const GIL_DEBUG_FULL: u32 = 999_999_999;
/// A monster is captured at most this many times (`AddMonsterCapture`).
pub const CAPTURE_MAX: i32 = 10;
/// Equipment list entries are `0x5000 + slot` (`MsGetSaveWeaponList`).
pub const EQUIPMENT_ID_BASE: u16 = 0x5000;

/// Word and bit of flag `n` in a `u16` flag array: the original divides
/// signed (`n >> 4` rounding toward zero), so callers mask `n` first.
#[must_use]
pub const fn flag(n: u32) -> (u32, u32) {
    (n >> 4, n & 0xf)
}

/// Whether bit `bit` of `word` is set, as 0 or 1.
#[must_use]
pub fn bit(word: u16, bit: u32) -> u32 {
    (u32::from(word) >> bit) & 1
}

/// Where the flag of a command (`0x3000`-`0x3fff`) is kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandFlag {
    /// Commands below `0x3060`: in the character's `ply_save` record
    /// (`+0x3e`), word and bit.
    Character(u32, u32),
    /// The rest: one array for everybody (`0x011307fc`), word and bit of
    /// `cmd - 0x60`.
    Shared(u32, u32),
}

/// The flag of command `cmd`, if it is a command ID (`0x3000`-`0x3fff`).
#[must_use]
pub const fn command_flag(cmd: u32) -> Option<CommandFlag> {
    if cmd & 0xffff_f000 != 0x3000 {
        return None;
    }
    if cmd < 0x3060 {
        let (w, b) = flag(cmd & 0xfff);
        Some(CommandFlag::Character(w, b))
    } else {
        let (w, b) = flag(cmd.wrapping_sub(0x60) & 0xfff);
        Some(CommandFlag::Shared(w, b))
    }
}

/// How many of `item` the inventory holds (`MsGetSaveItemNum`): the count
/// of the first slot with that ID and a count, one less while that item is
/// being used (`in_use`), 0 when no slot has it.
#[must_use]
pub fn item_count(ids: &[u16], counts: &[u8], item: u32, in_use: u16) -> u32 {
    let Some(i) = ids
        .iter()
        .zip(counts)
        .position(|(&id, &n)| u32::from(id) == item && n != 0)
    else {
        return 0;
    };
    let mut n = i32::from(counts[i]);
    if item == u32::from(in_use) {
        n -= 1;
    }
    n.max(0).cast_unsigned()
}

/// The new capture count from `current` and `delta` (`AddMonsterCapture`):
/// 0 when it would go negative, at most [`CAPTURE_MAX`].
#[must_use]
pub fn add_capture(current: u8, delta: i32) -> i32 {
    i32::from(current).wrapping_add(delta).clamp(0, CAPTURE_MAX)
}

/// Gil after adding `n` (`MsDebugAddGill`): unsigned, at most [`GIL_MAX`].
#[must_use]
pub const fn add_gil(gil: u32, n: i32) -> u32 {
    let g = gil.wrapping_add(n.cast_unsigned());
    if g > GIL_MAX { GIL_MAX } else { g }
}

/// The equipment slot fields `MsGetSaveWeaponList` and
/// `MsGetSaveWeaponAmount` look at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Slot {
    /// `+0x02`: the slot holds equipment.
    pub in_use: u8,
    /// `+0x03`: bit `0x2`, not counted as owned.
    pub flags: u8,
    /// `+0x04`: whose it is.
    pub character: u8,
    /// `+0x05`: 0 weapon, 1 armor.
    pub armor: u8,
    /// `+0x06`: who wears it, `0xff` nobody.
    pub equipped_by: u8,
}

impl Slot {
    /// From the slot's bytes `+0x02..+0x07`.
    #[must_use]
    pub const fn from_bytes(b: [u8; 5]) -> Self {
        Self {
            in_use: b[0],
            flags: b[1],
            character: b[2],
            armor: b[3],
            equipped_by: b[4],
        }
    }
}

/// Whether `MsGetSaveWeaponList(character, kind, unequipped)` lists a slot.
/// Like the original, a slot without equipment is always listed; a hidden
/// one (flag `0x2`) never for the party (`character` below 8); `character`
/// below 0 takes anybody's; `kind` 0 or 1 asks for weapons or armor (any
/// other kind both); `unequipped` only what nobody wears.
#[must_use]
pub fn weapon_listed(s: Slot, character: i32, kind: i32, unequipped: bool) -> bool {
    if s.in_use == 0 {
        return true;
    }
    if s.flags & 2 != 0 && character < 8 {
        return false;
    }
    let mut ok = i32::from(s.character) == character || character < 0;
    if unequipped {
        ok = ok && s.equipped_by == 0xff;
    }
    match kind {
        0 if s.armor != 0 => false,
        1 if s.armor != 1 => false,
        _ => ok,
    }
}

/// `(owned, free)` from the slots (`MsGetSaveWeaponAmount`): slots in use
/// without flag `0x2`, and 200 less the slots in use.
#[must_use]
pub fn weapon_amount(slots: impl IntoIterator<Item = Slot>) -> (u32, u32) {
    let (mut used, mut owned) = (0u32, 0u32);
    for s in slots {
        if s.in_use != 0 {
            used += 1;
            if s.flags & 2 == 0 {
                owned += 1;
            }
        }
    }
    (owned, EQUIPMENT_SLOTS.wrapping_sub(used))
}

/// The ability slots of an equipment after `MsCalcWeaponCap`: at least as
/// many as its abilities (neither 0 nor `0x00ff`, empty).
#[must_use]
pub fn ability_slots(abilities: [u16; 4], slots: u8) -> u8 {
    let n = abilities.iter().filter(|&&a| a != 0 && a != 0xff).count();
    #[allow(clippy::cast_possible_truncation)] // at most 4
    let n = n as u8;
    slots.max(n)
}

/// The save block where the game keeps it, through [`ffx_bridge::Mem`].
/// Pointer results are the game's addresses (rebased).
pub mod bridge {
    use ffx_bridge::{Global, Mem, Ptr};

    use super::{
        CommandFlag, EQUIPMENT_ID_BASE, EQUIPMENT_SIZE, EQUIPMENT_SLOTS, GIL_DEBUG_FULL,
        ITEM_SLOTS, PLAYERS, PLY_SAVE_SIZE, Slot, ability_slots, add_capture, add_gil, bit,
        command_flag, flag, item_count, weapon_amount, weapon_listed,
    };

    /// `g_SaveData`: the save block.
    pub const SAVE_DATA: Global<u8> = Global::new(0x0112_ca90);
    /// Story progress (`getScenerioFlag`), `u16` at save `+0xbec`.
    pub const SCENARIO: Global<u16> = Global::new(0x0112_d67c);
    /// Valefor's intermission flag (`getInterValefor`), save `+0xc89`.
    pub const INTER_VALEFOR: Global<u8> = Global::new(0x0112_d719);
    /// The ability map (sphere grid) block, save `+0x21ec`.
    pub const ABILITY_MAP: Global<u8> = Global::new(0x0112_ec7c);
    /// The walk map (minimap fog of war), save `+0x350c`.
    pub const WALK_MAP: Global<u8> = Global::new(0x0112_ff9c);
    /// Menu settings word (`MsGetSaveMenu`): bit 1 cursor memory, bit 3
    /// hiragana; save `+0x3d0c`.
    pub const MENU: Global<u32> = Global::new(0x0113_079c);
    /// Battles fought, save `+0x3d14`.
    pub const BATTLE_COUNT: Global<u32> = Global::new(0x0113_07a4);
    /// Yojimbo's command (`MsGetYoujinbouCom`), save `+0x3da4`.
    pub const YOUJINBOU_COM: Global<u32> = Global::new(0x0113_0834);
    /// Gil, save `+0x3d48`.
    pub const GIL: Global<u32> = Global::new(0x0113_07d8);
    /// The three party members in battle (`ply_save` indices).
    pub const IN_PARTY: Global<u8> = Global::new(0x0113_07e8);
    /// The 0x11 reserve members.
    pub const OUT_PARTY: Global<u8> = Global::new(0x0113_07eb);
    /// Command flags of commands `0x3060` and up, for everybody.
    pub const SHARED_COMMANDS: Global<u16> = Global::new(0x0113_07fc);
    /// Event flags (`MsGetSaveEventFlag`).
    pub const EVENT_FLAGS: Global<u8> = Global::new(0x0113_085c);
    /// Inventory item IDs, [`ITEM_SLOTS`] × `u16`.
    pub const ITEM_IDS: Global<u16> = Global::new(0x0113_095c);
    /// Inventory item counts, [`ITEM_SLOTS`] × `u8`.
    pub const ITEM_COUNTS: Global<u8> = Global::new(0x0113_0b5c);
    /// Item flag words (`MsGetItemCheck`).
    pub const ITEM_CHECK: Global<u16> = Global::new(0x0113_0c5c);
    /// Item used flag words (`MsGetSaveItemUse`).
    pub const ITEM_USE: Global<u16> = Global::new(0x0113_0c7c);
    /// Times each monster was captured, by monster ID (`u8`).
    pub const CAPTURES: Global<u8> = Global::new(0x0113_0c9c);
    /// Monsters met, flag words.
    pub const MONSTER_MEET: Global<u16> = Global::new(0x0113_0e9c);
    /// Monsters defeated, flag words.
    pub const MONSTER_DEFEAT: Global<u16> = Global::new(0x0113_0edc);
    /// Key items held, 8 flag words (IDs below 0x80).
    pub const IMPORTANT: Global<u16> = Global::new(0x0113_0f1c);
    /// `g_Equipment`: [`EQUIPMENT_SLOTS`] slots of [`EQUIPMENT_SIZE`] bytes.
    pub const EQUIPMENT: Global<u8> = Global::new(0x0113_0f2c);
    /// `g_PlySave`: [`PLAYERS`] records of [`PLY_SAVE_SIZE`] bytes.
    pub const PLY_SAVE: Global<u8> = Global::new(0x0113_205c);
    /// Commands used, [`PLAYERS`] × 0x16 flag words.
    pub const COMMAND_USE: Global<u16> = Global::new(0x0113_2ac4);
    /// Affection, `u32` by character; `MsAddLoveParam` takes 1-7.
    pub const LOVE: Global<u32> = Global::new(0x0112_cabc);

    /// The item being used in battle (`MsSetUseItem`), not counted by
    /// `MsGetSaveItemNum`.
    pub const USE_ITEM: Global<u16> = Global::new(0x0112_c948);
    /// The battle's party: two `u8` slots then the third
    /// (`MsSetSaveParty` copies them into [`IN_PARTY`]).
    pub const BATTLE_PARTY: Global<u16> = Global::new(0x0112_c895);
    /// The third battle party slot.
    pub const BATTLE_PARTY3: Global<u8> = Global::new(0x0112_c897);
    /// `MsGetSaveWeaponList`'s result buffer (`u16` entries).
    pub const WEAPON_LIST: Global<u16> = Global::new(0x0112_c526);
    /// The key item kernel table (`important.bin`).
    pub const KERNEL_IMPORTANT: Global<u32> = Global::new(0x0113_34c0);
    /// The character kernel table (`ply_rom`).
    pub const KERNEL_PLY_ROM: Global<u32> = Global::new(0x0112_a938);
    /// The default character names, 0x14 bytes each.
    pub const DEFAULT_NAMES: Global<u8> = Global::new(0x0231_10a0);

    fn word(mem: &impl Mem, base: u32, index: u32) -> u16 {
        Ptr::<u16>::new(base).at(index).get(mem)
    }

    /// `MsGetSaveCommand(chr, cmd)` (`0x00785020`): 1 when character `chr`
    /// (not checked) has learnt command `cmd`.
    pub fn ms_get_save_command(mem: &mut impl Mem, chr: i32, cmd: u32) -> u32 {
        match command_flag(cmd) {
            None => 0,
            Some(CommandFlag::Character(w, b)) => {
                let base = PLY_SAVE.ptr(mem).addr.wrapping_add(0x3e);
                let index = chr.cast_unsigned().wrapping_mul(PLY_SAVE_SIZE / 2);
                bit(word(mem, base, index.wrapping_add(w)), b)
            }
            Some(CommandFlag::Shared(w, b)) => bit(word(mem, SHARED_COMMANDS.ptr(mem).addr, w), b),
        }
    }

    /// `MsGetSaveCommandUse(chr, cmd)` (`0x00785140`): 1 when character
    /// `chr` (below 0x12, signed) has used command `cmd`.
    pub fn ms_get_save_command_use(mem: &mut impl Mem, chr: i32, cmd: u32) -> u32 {
        if cmd & 0xffff_f000 != 0x3000 || chr >= 0x12 {
            return 0;
        }
        let (w, b) = flag(cmd & 0xfff);
        let index = chr.cast_unsigned().wrapping_mul(0x16).wrapping_add(w);
        bit(word(mem, COMMAND_USE.ptr(mem).addr, index), b)
    }

    /// `MsGetSaveEventAddress` (`0x00785240`): `&g_SaveData`.
    pub fn ms_get_save_event_address(mem: &impl Mem) -> u32 {
        SAVE_DATA.ptr(mem).addr
    }

    /// `MsGetSaveMenu` (`0x00785290`): the menu settings word's address.
    pub fn ms_get_save_menu(mem: &impl Mem) -> u32 {
        MENU.ptr(mem).addr
    }

    /// `MsGetSaveConfigCursorSave` (`0x007851c0`): menu bit 1.
    pub fn ms_get_save_config_cursor_save(mem: &impl Mem) -> u32 {
        (MENU.get(mem) >> 1) & 1
    }

    /// `MsGetSaveConfigHiragana` (`0x007851f0`): menu bit 3.
    pub fn ms_get_save_config_hiragana(mem: &impl Mem) -> u32 {
        (MENU.get(mem) >> 3) & 1
    }

    /// `MsGetSavePlayerPtr(chr)` (`0x00785330`): the `ply_save` record of
    /// `chr & 0xff`, null from 0x12 on.
    pub fn ms_get_save_player_ptr(mem: &impl Mem, chr: u32) -> u32 {
        let chr = chr & 0xff;
        if chr >= PLAYERS {
            return 0;
        }
        PLY_SAVE.ptr(mem).addr + chr * PLY_SAVE_SIZE
    }

    fn ply_byte(mem: &impl Mem, chr: u32, offset: u32) -> u8 {
        let at = PLY_SAVE.ptr(mem).addr + (chr & 0xff) * PLY_SAVE_SIZE + offset;
        mem.read(at)
    }

    /// `MsGetSavePlyJoin(chr)` (`0x00785380`): `ply_save` `+0x2c` bit 0
    /// (the character can be in the party).
    pub fn ms_get_save_ply_join(mem: &impl Mem, chr: u32) -> u32 {
        u32::from(ply_byte(mem, chr, 0x2c) & 1)
    }

    /// `MsGetSavePlyJoined(chr)` (`0x007853a0`): `ply_save` `+0x2c` bit 4.
    pub fn ms_get_save_ply_joined(mem: &impl Mem, chr: u32) -> u32 {
        u32::from((ply_byte(mem, chr, 0x2c) >> 4) & 1)
    }

    /// `MsGetSaveSP(chr)` (`0x007853e0`): `ply_save` `+0x3b`.
    pub fn ms_get_save_sp(mem: &impl Mem, chr: u32) -> u32 {
        u32::from(ply_byte(mem, chr, 0x3b))
    }

    /// `getScenerioFlag` (`0x00787360`): the story progress.
    pub fn get_scenerio_flag(mem: &impl Mem) -> u16 {
        SCENARIO.get(mem)
    }

    /// `getInterValefor` (`0x00787340`).
    pub fn get_inter_valefor(mem: &impl Mem) -> u8 {
        INTER_VALEFOR.get(mem)
    }

    /// `getDefaultChrName(chr)` (`0x00787320`): character `chr`'s default
    /// name (not checked).
    pub fn get_default_chr_name(mem: &impl Mem, chr: u32) -> u32 {
        DEFAULT_NAMES
            .ptr(mem)
            .addr
            .wrapping_add(chr.wrapping_mul(0x14))
    }

    /// `MsGetSaveItemNum(item)` (`0x007904b0`): how many of `item` the
    /// party has, less the one being used.
    pub fn ms_get_save_item_num(mem: &mut impl Mem, item: u32) -> u32 {
        let ids: [u16; ITEM_SLOTS as usize] = ITEM_IDS.ptr(mem).cast().get(mem);
        let counts: [u8; ITEM_SLOTS as usize] = ITEM_COUNTS.ptr(mem).cast().get(mem);
        item_count(&ids, &counts, item, USE_ITEM.get(mem))
    }

    /// `MsSetUseItem(item)` (`0x00790790`).
    pub fn ms_set_use_item(mem: &mut impl Mem, item: u32) {
        #[allow(clippy::cast_possible_truncation)] // the original stores 16 bits
        USE_ITEM.set(mem, item as u16);
    }

    fn count_and(mem: &mut impl Mem, count: u32, n: u32, at: u32) -> u32 {
        if count != 0 {
            mem.write(count, n);
        }
        at
    }

    /// `MsSaveItemTypeBuffer(&count)` (`0x00790530`): the item IDs, their
    /// number (0x70) in `count` when not null.
    pub fn ms_save_item_type_buffer(mem: &mut impl Mem, count: u32) -> u32 {
        let at = ITEM_IDS.ptr(mem).addr;
        count_and(mem, count, ITEM_SLOTS, at)
    }

    /// `MsSaveItemNumBuffer(&count)` (`0x00790510`): the item counts.
    pub fn ms_save_item_num_buffer(mem: &mut impl Mem, count: u32) -> u32 {
        let at = ITEM_COUNTS.ptr(mem).addr;
        count_and(mem, count, ITEM_SLOTS, at)
    }

    /// `MsGetSaveInParty(&count)` (`0x00785270`): the battle party (3).
    pub fn ms_get_save_in_party(mem: &mut impl Mem, count: u32) -> u32 {
        let at = IN_PARTY.ptr(mem).addr;
        count_and(mem, count, 3, at)
    }

    /// `MsGetSaveOutParty(&count)` (`0x007852d0`): the reserve (0x11).
    pub fn ms_get_save_out_party(mem: &mut impl Mem, count: u32) -> u32 {
        let at = OUT_PARTY.ptr(mem).addr;
        count_and(mem, count, 0x11, at)
    }

    /// `MsGetSavePartyMember(&a, &b, &c)` (`0x007852f0`): the battle party,
    /// each into its pointer when not null.
    pub fn ms_get_save_party_member(mem: &mut impl Mem, a: u32, b: u32, c: u32) {
        for (i, out) in [a, b, c].into_iter().enumerate() {
            if out != 0 {
                #[allow(clippy::cast_possible_truncation)] // 0..3
                let v = IN_PARTY.ptr(mem).at(i as u32).get(mem);
                mem.write(out, u32::from(v));
            }
        }
    }

    /// `MsSetSaveParty` (`0x00786930`): the battle's party into the save.
    pub fn ms_set_save_party(mem: &mut impl Mem) {
        let ab = BATTLE_PARTY.get(mem);
        IN_PARTY.ptr(mem).cast::<u16>().set(mem, ab);
        let c = BATTLE_PARTY3.get(mem);
        IN_PARTY.ptr(mem).at(2).set(mem, c);
    }

    /// `MsGetGIL` (`0x00784e80`).
    pub fn ms_get_gil(mem: &impl Mem) -> u32 {
        GIL.get(mem)
    }

    /// `MsGetYoujinbouCom` (`0x007b2b20`).
    pub fn ms_get_youjinbou_com(mem: &impl Mem) -> u32 {
        YOUJINBOU_COM.get(mem)
    }

    /// `MsDebugFullGill` (`0x007849c0`).
    pub fn ms_debug_full_gill(mem: &mut impl Mem) {
        GIL.set(mem, GIL_DEBUG_FULL);
    }

    /// `MsDebugAddGill(n)` (`0x00784830`).
    pub fn ms_debug_add_gill(mem: &mut impl Mem, n: i32) {
        let g = add_gil(GIL.get(mem), n);
        GIL.set(mem, g);
    }

    /// `MsGetSaveBattleCount` (`0x00784f50`).
    pub fn ms_get_save_battle_count(mem: &impl Mem) -> u32 {
        BATTLE_COUNT.get(mem)
    }

    /// `MsGetMonsterCapture(id)` (`0x00790af0`): times monster
    /// `id & 0xfff` was captured.
    pub fn ms_get_monster_capture(mem: &impl Mem, id: u32) -> u32 {
        u32::from(CAPTURES.ptr(mem).at(id & 0xfff).get(mem))
    }

    /// `AddMonsterCapture(id, n)` (`0x00790b90`): adds `n` captures of
    /// monster `id & 0xfff` (0 to 10); ID 0xff is nobody. Returns the new
    /// count.
    pub fn add_monster_capture(mem: &mut impl Mem, id: u32, n: i32) -> i32 {
        let id = id & 0xfff;
        if id == 0xff {
            return 0;
        }
        let slot = CAPTURES.ptr(mem).at(id);
        let v = add_capture(slot.get(mem), n);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 0..=10
        slot.set(mem, v as u8);
        v
    }

    /// `MsImportantGet(id)` (`0x00790800`): 1 when key item `id & 0xfff`
    /// (below 0x80) is held.
    pub fn ms_important_get(mem: &impl Mem, id: u32) -> u32 {
        let id = id & 0xfff;
        if id >= 0x80 {
            return 0;
        }
        let (w, b) = flag(id);
        bit(word(mem, IMPORTANT.ptr(mem).addr, w), b)
    }

    /// `MsImportantSet(id)` (`0x00790930`): marks key item `id & 0xfff`
    /// (below 0x80) held.
    pub fn ms_important_set(mem: &mut impl Mem, id: u32) {
        let id = id & 0xfff;
        if id < 0x80 {
            let table = IMPORTANT.ptr(mem).addr;
            set_flag(mem, id, table);
        }
    }

    /// `MsImportantPtr(id, strings)` (`0x007908a0`): key item `id & 0xfff`'s
    /// kernel record (`MsGetExcelData`, which also returns the strings).
    pub fn ms_important_ptr(mem: &mut impl Mem, id: u32, strings: u32) -> u32 {
        let t = KERNEL_IMPORTANT.get(mem);
        #[allow(clippy::cast_possible_wrap)] // 12 bits
        ffx_kernel::bridge::ms_get_excel_data(mem, (id & 0xfff) as i32, t, strings)
    }

    fn set_flag(mem: &mut impl Mem, id: u32, table: u32) {
        let (w, b) = flag(id & 0xfff);
        let p = Ptr::<u16>::new(table).at(w);
        #[allow(clippy::cast_possible_truncation)] // the original shifts 16 bits
        let v = p.get(mem) | (1u32 << b) as u16;
        p.set(mem, v);
    }

    /// `MsSetItemCheck(id, table)` (`0x00790670`): sets flag `id & 0xfff`
    /// in the flag words at `table`.
    pub fn ms_set_item_check(mem: &mut impl Mem, id: u32, table: u32) {
        set_flag(mem, id, table);
    }

    /// `MsSetMonsterCheck(id, table)` (`0x00790be0`): the same for monsters.
    pub fn ms_set_monster_check(mem: &mut impl Mem, id: u32, table: u32) {
        set_flag(mem, id, table);
    }

    /// `MsGetSaveAbilityMap` (`0x00784f40`).
    pub fn ms_get_save_ability_map(mem: &impl Mem) -> u32 {
        ABILITY_MAP.ptr(mem).addr
    }

    /// `MsGetSaveWalkMap` (`0x00785400`).
    pub fn ms_get_save_walk_map(mem: &impl Mem) -> u32 {
        WALK_MAP.ptr(mem).addr
    }

    /// `MsGetSaveEventFlag` (`0x00785250`).
    pub fn ms_get_save_event_flag(mem: &impl Mem) -> u32 {
        EVENT_FLAGS.ptr(mem).addr
    }

    /// `MsGetItemCheck` (`0x007904a0`).
    pub fn ms_get_item_check(mem: &impl Mem) -> u32 {
        ITEM_CHECK.ptr(mem).addr
    }

    /// `MsGetSaveItemUse` (`0x00790500`).
    pub fn ms_get_save_item_use(mem: &impl Mem) -> u32 {
        ITEM_USE.ptr(mem).addr
    }

    /// `MsGetSaveMonsterMeet` (`0x00790b20`).
    pub fn ms_get_save_monster_meet(mem: &impl Mem) -> u32 {
        MONSTER_MEET.ptr(mem).addr
    }

    /// `MsGetSaveMonsterDefeat` (`0x00790b10`).
    pub fn ms_get_save_monster_defeat(mem: &impl Mem) -> u32 {
        MONSTER_DEFEAT.ptr(mem).addr
    }

    fn slot(mem: &impl Mem, i: u32) -> Slot {
        let at = EQUIPMENT.ptr(mem).addr + i * EQUIPMENT_SIZE + 2;
        Slot::from_bytes(mem.read(at))
    }

    /// `MsGetSaveWeaponList(character, kind, unequipped, &count)`
    /// (`0x007abd10`): fills the list buffer with `0x5000 + slot` for every
    /// slot [`weapon_listed`]; their number into `count` when not null.
    /// Returns the buffer.
    pub fn ms_get_save_weapon_list(
        mem: &mut impl Mem,
        character: i32,
        kind: i32,
        unequipped: i32,
        count: u32,
    ) -> u32 {
        let list = WEAPON_LIST.ptr(mem);
        let mut n = 0u32;
        for i in 0..EQUIPMENT_SLOTS {
            if weapon_listed(slot(mem, i), character, kind, unequipped != 0) {
                #[allow(clippy::cast_possible_truncation)] // below 200
                list.at(n).set(mem, EQUIPMENT_ID_BASE + i as u16);
                n += 1;
            }
        }
        if count != 0 {
            mem.write(count, n);
        }
        list.addr
    }

    /// `MsGetSaveWeaponAmount(&free)` (`0x007abc60`): the equipment owned;
    /// the free slots into `free` when not null.
    pub fn ms_get_save_weapon_amount(mem: &mut impl Mem, free: u32) -> u32 {
        let slots: Vec<Slot> = (0..EQUIPMENT_SLOTS).map(|i| slot(mem, i)).collect();
        let (owned, empty) = weapon_amount(slots);
        if free != 0 {
            mem.write(free, empty);
        }
        owned
    }

    /// `MsCalcWeaponCap(equipment)` (`0x00798eb0`): raises the slot count
    /// (`+0x0b`) of the equipment at `equipment` to its ability count
    /// (`+0x0e`, four `u16`); writes only when it grows.
    pub fn ms_calc_weapon_cap(mem: &mut impl Mem, equipment: u32) {
        let p = Ptr::<u8>::new(equipment);
        let slots = p.field::<u8>(0x0b).get(mem);
        let n = ability_slots(p.field::<[u16; 4]>(0x0e).get(mem), slots);
        if n > slots {
            p.field::<u8>(0x0b).set(mem, n);
        }
    }

    fn ply_rom_flag(mem: &mut impl Mem, chr: u32, shift: u32) -> u32 {
        let t = KERNEL_PLY_ROM.get(mem);
        let r = ffx_kernel::bridge::ms_get_excel_data(mem, chr.cast_signed(), t, 0);
        u32::from(mem.read::<u8>(r.wrapping_add(0x10)) >> shift) & 1
    }

    /// `MsGetSaveChrMan(chr)` (`0x00784f90`): `ply_rom` `+0x10` bit 0.
    pub fn ms_get_save_chr_man(mem: &mut impl Mem, chr: u32) -> u32 {
        ply_rom_flag(mem, chr, 0)
    }

    /// `MsGetSaveChrFemale(chr)` (`0x00784f60`): `ply_rom` `+0x10` bit 1.
    pub fn ms_get_save_chr_female(mem: &mut impl Mem, chr: u32) -> u32 {
        ply_rom_flag(mem, chr, 1)
    }

    /// `MsGetSaveChrSummon(chr)` (`0x00784ff0`): `ply_rom` `+0x10` bit 2
    /// (an aeon).
    pub fn ms_get_save_chr_summon(mem: &mut impl Mem, chr: u32) -> u32 {
        ply_rom_flag(mem, chr, 2)
    }

    /// `MsAddLoveParam(chr, n)` (`0x007845e0`): adds `n` to character
    /// `chr`'s (1-7) affection unless it would pass `0x0ffffffe`; returns
    /// the sum (0 for another `chr`), stored or not.
    pub fn ms_add_love_param(mem: &mut impl Mem, chr: i32, n: i32) -> u32 {
        if chr.cast_unsigned().wrapping_sub(1) > 6 {
            return 0;
        }
        let p = LOVE.ptr(mem).at(chr.cast_unsigned());
        let v = p.get(mem).wrapping_add(n.cast_unsigned());
        if v <= 0x0fff_fffe {
            p.set(mem, v);
        }
        v
    }

    /// Item list A (`MsSetItemWindow`: usable items without flag 0x400),
    /// 0x70 `u16`, `0xff` after the last.
    pub const ITEM_WINDOW: Global<[u16; 0x70]> = Global::new(0x0112_c6b6);
    /// Item list B (usable items with flag 0x400).
    pub const ITEM_WINDOW2: Global<[u16; 0x70]> = Global::new(0x0112_c796);
    /// Items stolen so far (`Battle_Steal`, achievement 4 at 200).
    pub const STEAL_COUNT: Global<u32> = Global::new(0x0113_0840);
    /// No steals in this battle when 2 (`Battle_Steal`).
    pub const STEAL_RULE: Global<u8> = Global::new(0x0112_c9e5);
    /// Debug: steals take the rare item (`Battle_Steal`).
    pub const DEBUG_RARE_STEAL: Global<u8> = Global::new(0x0112_a911);

    /// `MsSetItemWindow` (`0x007906a0`): the battle item lists, in
    /// inventory order: items usable in battle (kernel flag 2) into list A,
    /// or list B with flag 0x400; the rest `0xff`.
    pub fn ms_set_item_window(mem: &mut impl Mem) {
        let ids: [u16; ITEM_SLOTS as usize] = ITEM_IDS.ptr(mem).cast().get(mem);
        let (mut a, mut b) = ([0xffu16; 0x70], [0xffu16; 0x70]);
        let (mut na, mut nb) = (0, 0);
        for id in ids {
            if id == 0xff {
                continue;
            }
            let rec = ffx_kernel::bridge::rom(mem, ffx_kernel::bridge::ITEM, u32::from(id), 0);
            let flags: u32 = mem.read(rec.wrapping_add(0x1c));
            if flags & 2 == 0 {
                continue;
            }
            if flags & 0x400 == 0 {
                a[na] = id;
                na += 1;
            } else {
                b[nb] = id;
                nb += 1;
            }
        }
        ITEM_WINDOW.set(mem, a);
        ITEM_WINDOW2.set(mem, b);
    }

    /// `MsSaveItemUse(item, n)` (`0x00790550`): adds `n` (negative: uses)
    /// of `item` (`0x2xxx`) to the inventory, 0-99 a slot, into the first
    /// free slot when it has none; marks it got or used (`MsSetItemCheck`)
    /// when the count changes; rebuilds the battle lists. 0, or -1 when
    /// nothing changed hands.
    pub fn ms_save_item_use(mem: &mut impl Mem, item: u32, n: i32) -> i32 {
        use ffx_kernel::bridge::ms_check_range;
        if item & 0xffff_f000 != 0x2000 || n == 0 {
            return -1;
        }
        let ids = ITEM_IDS.ptr(mem);
        let counts = ITEM_COUNTS.ptr(mem);
        let mut free = -1i32;
        for i in 0..ITEM_SLOTS {
            let id = ids.at(i).get(mem);
            let c = counts.at(i).get(mem);
            if u32::from(id) == item && c != 0 && (n >= 0 || i32::from(c) + n >= 0) {
                let v = ms_check_range(i32::from(c) + n, 0, 99);
                counts.at(i).set(mem, v.to_le_bytes()[0]);
                if v != i32::from(c) {
                    let table = if n > 0 {
                        ms_get_item_check(mem)
                    } else {
                        ms_get_save_item_use(mem)
                    };
                    ms_set_item_check(mem, item, table);
                }
                if v == 0 {
                    ids.at(i).set(mem, 0xff);
                }
                ms_set_item_window(mem);
                return 0;
            }
            if (id == 0 || c == 0) && free < 0 {
                free = i.cast_signed();
            }
        }
        if free >= 0 && n > 0 {
            let f = free.cast_unsigned();
            ids.at(f).set(
                mem,
                item.to_le_bytes()[..2]
                    .try_into()
                    .map_or(0, u16::from_le_bytes),
            );
            counts
                .at(f)
                .set(mem, ms_check_range(n, 0, 99).to_le_bytes()[0]);
            ms_set_item_window(mem);
            return 0;
        }
        -1
    }

    /// `Battle_Steal(user, target, spoils, info, fail)` (`0x0078b760`): a
    /// steal from a monster's spoils (`+0xa` chance /255, items `+0x24` /
    /// `+0x26`, counts `+0x28` / `+0x29`) with generator 10; the rare item
    /// with generator 11 below 32 (128 with the user's `+0x6be` bit 0x80,
    /// 256 with 0x100). A steal halves the chance, counts at the target's
    /// `+0xdeb`, and puts the item in the inventory; `info` `+8`/`+0xa` say
    /// what (`0xffff` = something to steal, `+7` set). In the Korean game
    /// item 0x2065 is 2 × 0x206d. `lang` is `TOGetFFXLang`, `achievement`
    /// `achievementUnlockAchievement` (both other modules').
    #[allow(clippy::too_many_arguments)]
    pub fn battle_steal<M: Mem>(
        mem: &mut M,
        user: u32,
        target: u32,
        spoils: u32,
        info: u32,
        fail: i32,
        lang: impl FnOnce(&mut M) -> u32,
        achievement: impl FnOnce(&mut M, u32),
    ) -> u32 {
        use crate::damage::bridge::DEBUG_ALWAYS_HIT;
        use crate::rng::bridge::brnd;
        if spoils == 0 {
            return 0;
        }
        let u8_at = |mem: &M, o: u32| mem.read::<u8>(spoils.wrapping_add(o));
        let u16_at = |mem: &M, o: u32| mem.read::<u16>(spoils.wrapping_add(o));
        let mut chance = i32::from(u8_at(mem, 0xa));
        let something = chance != 0
            && u8_at(mem, 0x28) != 0
            && u16_at(mem, 0x24) & 0xf000 == 0x2000
            && STEAL_RULE.get(mem) != 2;
        if !something {
            chance = 0;
        }
        mem.write(info.wrapping_add(8), if something { 0xffffu16 } else { 0 });
        mem.write(info.wrapping_add(7), 1u8);
        mem.write(info.wrapping_add(0xa), 0u16);
        let r = brnd(mem, 10).cast_signed() % 0xff;
        let k: u16 = mem.read(user.wrapping_add(0x6be));
        let rare = if k & 0x100 != 0 {
            0x100
        } else if k & 0x80 != 0 {
            0x80
        } else {
            0x20
        };
        if !((r < chance || DEBUG_ALWAYS_HIT.get(mem) != 0) && fail == 0) {
            return 0;
        }
        let c = u8_at(mem, 0xa);
        let c = (u32::from(c) * 50 / 100).to_le_bytes()[0];
        mem.write(spoils.wrapping_add(0xa), c.max(1));
        let r = brnd(mem, 11) & 0xff;
        let which = u32::from(
            (r < rare || DEBUG_RARE_STEAL.get(mem) != 0)
                && u8_at(mem, 0x29) != 0
                && u16_at(mem, 0x26) & 0xf000 == 0x2000,
        );
        let mut count = u16::from(u8_at(mem, 0x28 + which));
        let mut item = u16_at(mem, 0x24 + 2 * which);
        mem.write(info.wrapping_add(8), count);
        mem.write(info.wrapping_add(0xa), item);
        if lang(mem) == 0xb && item == 0x2065 {
            mem.write(spoils.wrapping_add(0x24 + 2 * which), 0x206du16);
            mem.write(spoils.wrapping_add(0x28 + which), 2u8);
            item = 0x206d;
            count = 2;
            mem.write(info.wrapping_add(0xa), item);
            mem.write(info.wrapping_add(8), count);
        }
        if target != 0 {
            let s: u8 = mem.read(target.wrapping_add(0xdeb));
            if s != 0xff {
                mem.write(target.wrapping_add(0xdeb), s + 1);
            }
        }
        if mem.read::<u8>(target.wrapping_add(0xded)) == 6 {
            let n = STEAL_COUNT.get(mem).wrapping_add(1);
            STEAL_COUNT.set(mem, n);
            if n.cast_signed() >= 200 {
                achievement(mem, 4);
            }
        }
        ms_save_item_use(mem, u32::from(item), i32::from(count.cast_signed()));
        0
    }

    /// `Battle_StealGil(user, target, spoils, info, fail)` (`0x0078b920`):
    /// Mug's gil: with the spoils' gil (`+0x113`) and the target's chance
    /// (`+0x712`, /255, halved by each success), `(100 + roll % 101) · gil
    /// · 100 / 200 · chance / 255` gil into `info` `+0x10` (-1 for none to
    /// take); generator 10.
    pub fn battle_steal_gil(
        mem: &mut impl Mem,
        target: u32,
        spoils: u32,
        info: u32,
        fail: i32,
    ) -> u32 {
        use crate::damage::bridge::DEBUG_ALWAYS_HIT;
        use crate::rng::bridge::brnd;
        if spoils == 0 {
            return 0;
        }
        mem.write(info.wrapping_add(0xc), 1u8);
        let gil: u8 = mem.read(spoils.wrapping_add(0x113));
        if gil == 0 || STEAL_RULE.get(mem) == 2 {
            mem.write(info.wrapping_add(0x10), u32::MAX);
            return 0;
        }
        mem.write(info.wrapping_add(0x10), 0u32);
        let chance = i32::from(mem.read::<i16>(target.wrapping_add(0x712)));
        let r = brnd(mem, 10).cast_signed() % 0xff;
        if (r < chance || DEBUG_ALWAYS_HIT.get(mem) != 0) && fail == 0 {
            let r = brnd(mem, 10).cast_signed() % 0x65;
            let x = (r + 100).wrapping_mul(i32::from(gil)).wrapping_mul(100) / 200;
            let now = i32::from(mem.read::<i16>(target.wrapping_add(0x712)));
            mem.write(info.wrapping_add(0x10), x.wrapping_mul(now) / 0xff);
            let c = (chance * 50 / 100).max(1);
            mem.write(
                target.wrapping_add(0x712),
                c.to_le_bytes()[..2]
                    .try_into()
                    .map_or(0, u16::from_le_bytes),
            );
        }
        0
    }

    /// `MsGetActionBuffer(actor)` (`0x0078d040`): the first of the actor's
    /// two 0x2d8-byte action buffers (`+0x774`) not in use (`+2` 0xff),
    /// cleared; null when both are.
    pub fn ms_get_action_buffer(mem: &mut impl Mem, actor: u32) -> u32 {
        for i in 0..2 {
            let p = actor.wrapping_add(0x774 + i * 0x2d8);
            if mem.read::<u8>(p.wrapping_add(2)) == 0xff {
                crate::util::bridge::ms_struct_clear(mem, p, 0x2d8);
                return p;
            }
        }
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_flags_split_at_0x3060() {
        assert_eq!(command_flag(0x2fff), None);
        assert_eq!(command_flag(0x3011), Some(CommandFlag::Character(1, 1)));
        assert_eq!(command_flag(0x3060), Some(CommandFlag::Shared(0, 0)));
        assert_eq!(command_flag(0x3fff), Some(CommandFlag::Shared(0xf9, 0xf)));
    }

    #[test]
    fn item_count_skips_the_item_in_use() {
        let ids = [3u16, 7, 7];
        let counts = [1u8, 0, 5];
        assert_eq!(item_count(&ids, &counts, 7, 0), 5);
        assert_eq!(item_count(&ids, &counts, 7, 7), 4);
        assert_eq!(item_count(&ids, &counts, 9, 0), 0);
        assert_eq!(add_capture(9, 3), 10);
        assert_eq!(add_capture(1, -3), 0);
        assert_eq!(add_gil(GIL_MAX, 1), GIL_MAX);
    }
}
