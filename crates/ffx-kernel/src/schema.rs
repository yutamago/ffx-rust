//! What is known about each table's records.
//!
//! Every name here comes from how `FFX.exe` reads the field; see the doc of
//! each [`Field`]. Bytes no field covers are exported as hex and kept as is.

/// The integer type of a named field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    U8,
    U16,
    U32,
}

impl Kind {
    #[must_use]
    pub fn len(self) -> usize {
        match self {
            Self::U8 => 1,
            Self::U16 => 2,
            Self::U32 => 4,
        }
    }

    /// Never true; present for clippy's `len_without_is_empty`.
    #[must_use]
    pub fn is_empty(self) -> bool {
        false
    }
}

/// A named field of a record.
#[derive(Debug, Clone, Copy)]
pub struct Field {
    pub name: &'static str,
    pub offset: usize,
    pub kind: Kind,
    /// Where the meaning comes from.
    pub doc: &'static str,
}

/// The layout of one table's records.
#[derive(Debug, Clone, Copy)]
pub struct Schema {
    /// File name without `.bin`, as passed to `getExcelBinSize`.
    pub file: &'static str,
    pub record_size: u16,
    /// Names of the string references at the start of each record.
    pub strings: &'static [&'static str],
    pub fields: &'static [Field],
}

impl Schema {
    /// Looks up a table by file name, with or without `.bin`.
    #[must_use]
    pub fn by_file(file: &str) -> Option<&'static Self> {
        let file = file.strip_suffix(".bin").unwrap_or(file);
        SCHEMAS
            .iter()
            .chain(SHARED_SCHEMAS)
            .find(|s| s.file == file)
    }

    /// Whether the table is one of [`SHARED_SCHEMAS`]: a single copy in
    /// `jppc` for every language.
    #[must_use]
    pub fn is_shared(&self) -> bool {
        SHARED_SCHEMAS.iter().any(|s| s.file == self.file)
    }

    /// The table's archive path for `language`.
    #[must_use]
    pub fn path(&self, language: ffx_text::Language) -> String {
        if self.is_shared() {
            format!("ffx_ps2/ffx/master/jppc/battle/kernel/{}.bin", self.file)
        } else {
            format!("{}{}.bin", language.kernel_dir(), self.file)
        }
    }

    /// Byte range covered by the string references.
    #[must_use]
    pub fn strings_len(&self) -> usize {
        self.strings.len() * 4
    }
}

/// Battle commands and items share one record layout.
const COMMAND_FIELDS: &[Field] = &[
    Field {
        name: "icon",
        offset: 0x14,
        kind: Kind::U8,
        doc: "the icon drawn before an item's name (MsGetRomItem's record byte +0x14, ffx_menu::tk); name from Fahrenheit",
    },
    Field {
        name: "sub_menu_cat2",
        offset: 0x17,
        kind: Kind::U8,
        doc: "the command group the inventory must allow when +0x16 & 0xf8 is 8: 5 an aeon, 6/0x11 an item, 7/0xf a weapon or armour, 0xa a battle-list kind (ffx_battle::ability); name from Fahrenheit",
    },
    Field {
        name: "flags_target",
        offset: 0x1a,
        kind: Kind::U8,
        doc: "bit 1 targets the other side, bits 2-3 0 for a one-target or self command, bit 6 the dead (ffx_battle::action, ffx_battle::actor); name and the other bits from Fahrenheit",
    },
    Field {
        name: "flags",
        offset: 0x1c,
        kind: Kind::U32,
        doc: "bit 0x2: listed for use (MsSetItemWindow 0x007906a0); bit 0x400 selects the second list; bit 0x40000: formula and power come from the user's weapon (0x007898d0, 0x0078e630)",
    },
    Field {
        name: "crit_flags",
        offset: 0x20,
        kind: Kind::U8,
        doc: "bit 0x4: can be a critical hit; bit 0x8: its crit bonus is the user's equipment's (actor +0x5d8) instead of crit_bonus (crit roll 0x00789690)",
    },
    Field {
        name: "flags_damage_class",
        offset: 0x23,
        kind: Kind::U8,
        doc: "what the command hits as DmgCalc keeps it (DmgCalc 0x0078e630 reads it at 0x0078e7e8); name from Fahrenheit",
    },
    Field {
        name: "mp_cost",
        offset: 0x25,
        kind: Kind::U8,
        doc: "MP cost (MsGetCommandMP 0x0078cf70 reads it at 0x0078cf9c, 0 with the user's STAT_USE_MP0 +0x6ce; the menus' MP column); name from Fahrenheit",
    },
    Field {
        name: "limit_cost",
        offset: 0x26,
        kind: Kind::U8,
        doc: "overdrive gauge cost (MsCalcMagic 0x0078abb7 and MsCheckUseCommand 0x0078c6ad read it; ignored with the debug no-cost flag 0x0112a90c); name from Fahrenheit",
    },
    Field {
        name: "crit_bonus",
        offset: 0x27,
        kind: Kind::U8,
        doc: "added to the critical hit chance, in percent (crit roll 0x00789690)",
    },
    Field {
        name: "formula",
        offset: 0x28,
        kind: Kind::U8,
        doc: "damage or healing formula; healing (Cmd_HealAmount 0x008c6920): 6 = power x 50, 7 = (stat + power) / 2 x power, 8 = max HP x power / 16",
    },
    Field {
        name: "accuracy",
        offset: 0x29,
        kind: Kind::U8,
        doc: "hit chance input (DmgCalc_CheckHit 0x0078a890 reads it at 0x0078a8db); name from Fahrenheit",
    },
    Field {
        name: "power",
        offset: 0x2a,
        kind: Kind::U8,
        doc: "formula input (Cmd_HealAmount 0x008c6920, damage setup 0x0078e630); Potion 4 = 200 HP",
    },
    Field {
        name: "hit_count",
        offset: 0x2b,
        kind: Kind::U8,
        doc: "hits (MsMenuLimitSet's hit counts, ffx_battle::limit); name from Fahrenheit",
    },
    Field {
        name: "shatter_chance",
        offset: 0x2c,
        kind: Kind::U8,
        doc: "a roll % 101 below it shatters a petrified target: KO and bit 0x100 (DmgCalc_InflictStatus_Extra 0x0078b4e0); name from Fahrenheit",
    },
    Field {
        name: "element",
        offset: 0x2d,
        kind: Kind::U8,
        doc: "element bits: 0x1 fire, 0x2 ice, 0x4 thunder, 0x8 water, 0x10 holy (MsGetComInfo 0x0078cd30 reads it); name from Fahrenheit",
    },
    Field {
        name: "flags_status_extra",
        offset: 0x54,
        kind: Kind::U16,
        doc: "the second status word it inflicts or removes (+0x616 bits; DmgCalc_InflictStatus_Extra 0x0078b4e0); the 25 status chances at +0x2e and 13 durations at +0x47 are exported as hex (DmgCalc_InflictStatus 0x0078ae00); name from Fahrenheit",
    },
    Field {
        name: "flags_buffs_stat",
        offset: 0x56,
        kind: Kind::U16,
        doc: "cheer, aim, focus, reflex, luck and jinx bits it raises (DmgCalc_ApplyStatUpBuffs 0x0078d730); name from Fahrenheit",
    },
    Field {
        name: "overdrive_category",
        offset: 0x58,
        kind: Kind::U8,
        doc: "the overdrive kind (MsMenuLimitSet 0x007afe00; 1: the menus draw the user's name, MenuDrawChrNameOrCommand); name from Fahrenheit",
    },
    Field {
        name: "buff_amount",
        offset: 0x59,
        kind: Kind::U8,
        doc: "how many levels +0x56 adds, at least 1, to 5 at most (DmgCalc_ApplyStatUpBuffs); name from Fahrenheit",
    },
    Field {
        name: "ordering_idx",
        offset: 0x5c,
        kind: Kind::U8,
        doc: "the battle menu's sort key (ffx_battle::ability); commands and items only (monmagic records end at 0x5c); name from Fahrenheit",
    },
];

/// An auto-ability slot: 0x8000 + `a_ability` ID; 0x00ff (0 in `buki_get`)
/// for none. Each one's `a_ability` record is applied to the wearer in
/// battle (`MsSetRamChrParam`, `0x0079c5f0`).
const ABILITY_DOC: &str = "auto-ability, 0x8000 + a_ability ID; 0x00ff empty";

/// Auto-abilities (`a_ability`, 108 bytes): what an equipment ability
/// gives its wearer (`MsSetRamChrParam`, `0x0079c5f0`; `MsSetSaveParam`,
/// `0x007860f0`). Field names from Fahrenheit's `AutoAbility`.
const A_ABILITY_FIELDS: &[Field] = &[
    Field {
        name: "is_sos",
        offset: 0x10,
        kind: Kind::U8,
        doc: "bit 0: the auto-statuses at +0x58 apply only at HP weak levels 1 and 2 (SOS: actor +0x636 instead of +0x630; MsSetRamChrParam 0x0079c5f0 tests it at 0x0079c957); name from Fahrenheit",
    },
    Field {
        name: "elem_strike",
        offset: 0x11,
        kind: Kind::U8,
        doc: "element bits ORed into the wearer: weapon element (actor +0x5d9) (MsSetRamChrParam 0x0079c5f0); name from Fahrenheit",
    },
    Field {
        name: "elem_absorb",
        offset: 0x12,
        kind: Kind::U8,
        doc: "element bits ORed into the wearer: absorbed (actor +0x5da) (MsSetRamChrParam 0x0079c5f0); name from Fahrenheit",
    },
    Field {
        name: "elem_ignore",
        offset: 0x13,
        kind: Kind::U8,
        doc: "element bits ORed into the wearer: nullified (actor +0x5db) (MsSetRamChrParam 0x0079c5f0); name from Fahrenheit",
    },
    Field {
        name: "elem_resist",
        offset: 0x14,
        kind: Kind::U8,
        doc: "element bits ORed into the wearer: halved (actor +0x5dc) (MsSetRamChrParam 0x0079c5f0); name from Fahrenheit",
    },
    Field {
        name: "elem_weak",
        offset: 0x15,
        kind: Kind::U8,
        doc: "element bits ORed into the wearer: weak (actor +0x5dd) (MsSetRamChrParam 0x0079c5f0); name from Fahrenheit",
    },
    Field {
        name: "stat_inc_amount",
        offset: 0x55,
        kind: Kind::U8,
        doc: "percentage added to the values +0x56 names (MsSetSaveParam 0x007860f0); name from Fahrenheit",
    },
    Field {
        name: "stat_inc_flags",
        offset: 0x56,
        kind: Kind::U16,
        doc: "bits 0-7 strength, defence, magic, magic defence, agility, luck, evasion, accuracy; 8 HP, 9 MP; 10-13 four damage rates (MsSetSaveParam 0x007860f0); bit names from Fahrenheit",
    },
    Field {
        name: "status_auto_permanent",
        offset: 0x58,
        kind: Kind::U16,
        doc: "auto-statuses, actor +0x606 bits (MsSetRamChrParam 0x0079c5f0); the 25 inflict chances at +0x16, 13 durations at +0x2f and 25 resistances at +0x3c are exported as hex; name from Fahrenheit",
    },
    Field {
        name: "status_auto_temporal",
        offset: 0x5a,
        kind: Kind::U16,
        doc: "auto-statuses, the 13 counters' bits (MsSetRamChrParam 0x0079c5f0); name from Fahrenheit",
    },
    Field {
        name: "status_auto_extra",
        offset: 0x5c,
        kind: Kind::U16,
        doc: "auto-statuses, actor +0x616 bits (MsSetRamChrParam 0x0079c5f0); name from Fahrenheit",
    },
    Field {
        name: "status_inflict_extra",
        offset: 0x5e,
        kind: Kind::U16,
        doc: "+0x616 bits the wearer's attacks inflict (MsSetRamChrParam 0x0079c5f0); name from Fahrenheit",
    },
    Field {
        name: "status_resist_extra",
        offset: 0x60,
        kind: Kind::U16,
        doc: "+0x616 bits the wearer resists, actor +0x65a (MsSetRamChrParam 0x0079c5f0); name from Fahrenheit",
    },
    Field {
        name: "auto_ability_effects",
        offset: 0x62,
        kind: Kind::U16,
        doc: "the first word of the auto-ability effect bits, actor +0x6bc (MsSetRamChrParam 0x0079c5f0; MsSetSaveParam copies +0x62-+0x67 into ply_save +0x4a); name from Fahrenheit",
    },
];

/// Equipment: the starting inventory (`weapon`, `wep_data`, copied into
/// the 200 slots at `0x01130f2c` by `MsSetSaveStartGame`, `0x00786b00`) and the
/// equipment shops sell (`shop_arms`). The inventory slots have the same
/// layout.
const EQUIPMENT_FIELDS: &[Field] = &[
    Field {
        name: "name",
        offset: 0x00,
        kind: Kind::U16,
        doc: "w_name record, chosen from character, kind and abilities when the equipment is created (0x007a0cf0); 0 in the files",
    },
    Field {
        name: "in_use",
        offset: 0x02,
        kind: Kind::U8,
        doc: "nonzero: the slot holds equipment (0x007abc60)",
    },
    Field {
        name: "flags",
        offset: 0x03,
        kind: Kind::U8,
        doc: "bit 0x2: not counted as owned (0x007abc60) nor matched by the equipment checks of 0x0079bb50",
    },
    Field {
        name: "character",
        offset: 0x04,
        kind: Kind::U8,
        doc: "who wears it, a ply_save record: 0 Tidus, 1 Yuna, 2 Auron, 3 Kimahri, 4 Wakka, 5 Lulu, 6 Rikku, 7 Seymour, 8-17 the aeons; only 0-6 get a name and model from w_name (0x00786b00)",
    },
    Field {
        name: "armor",
        offset: 0x05,
        kind: Kind::U8,
        doc: "0 weapon, 1 armor (0x0079bb50 checks; 0x007a0cf0 names weapons and armor apart)",
    },
    Field {
        name: "equipped_by",
        offset: 0x06,
        kind: Kind::U8,
        doc: "character wearing it, 0xff none; rebuilt from ply_save +0x2d/+0x2e (0x007abca0)",
    },
    Field {
        name: "formula",
        offset: 0x08,
        kind: Kind::U8,
        doc: "weapon: damage formula of commands with flag 0x40000 (actor +0x5c1, MsSetRamChrParam 0x0079c5f0)",
    },
    Field {
        name: "power",
        offset: 0x09,
        kind: Kind::U8,
        doc: "weapon: power of commands with flag 0x40000 (actor +0x5c7)",
    },
    Field {
        name: "crit_bonus",
        offset: 0x0a,
        kind: Kind::U8,
        doc: "weapon and armor values add up into actor +0x5d8, the crit bonus of commands with crit flag 0x8",
    },
    Field {
        name: "slots",
        offset: 0x0b,
        kind: Kind::U8,
        doc: "ability slots",
    },
    Field {
        name: "model",
        offset: 0x0c,
        kind: Kind::U16,
        doc: "w_name +0x38 + 2 x character of the name's record (0x007a0c50)",
    },
    Field {
        name: "ability_1",
        offset: 0x0e,
        kind: Kind::U16,
        doc: ABILITY_DOC,
    },
    Field {
        name: "ability_2",
        offset: 0x10,
        kind: Kind::U16,
        doc: ABILITY_DOC,
    },
    Field {
        name: "ability_3",
        offset: 0x12,
        kind: Kind::U16,
        doc: ABILITY_DOC,
    },
    Field {
        name: "ability_4",
        offset: 0x14,
        kind: Kind::U16,
        doc: ABILITY_DOC,
    },
];

/// Treasure: what a chest (or a reward) gives (`msFieldItemGet`,
/// `0x00799420`).
const TAKARA_FIELDS: &[Field] = &[
    Field {
        name: "kind",
        offset: 0,
        kind: Kind::U8,
        doc: "0 gil (amount x 100), 2 item, 5 equipment (a buki_get record), 10 key item",
    },
    Field {
        name: "amount",
        offset: 1,
        kind: Kind::U8,
        doc: "item count (up to 99), or gil / 100",
    },
    Field {
        name: "item",
        offset: 2,
        kind: Kind::U16,
        doc: "what it gives: item (0x2000 + item ID), buki_get record, or key item (0xa000 + important ID)",
    },
];

/// Equipment found in treasure (`msFieldItemGet`, `0x00799420`, builds an
/// equipment slot from it).
const BUKI_GET_FIELDS: &[Field] = &[
    Field {
        name: "flags",
        offset: 0,
        kind: Kind::U8,
        doc: "equipment flags (+0x03)",
    },
    Field {
        name: "character",
        offset: 1,
        kind: Kind::U8,
        doc: "equipment character (+0x04), a ply_save record",
    },
    Field {
        name: "armor",
        offset: 2,
        kind: Kind::U8,
        doc: "0 weapon, 1 armor (+0x05)",
    },
    Field {
        name: "formula",
        offset: 4,
        kind: Kind::U8,
        doc: "equipment formula (+0x08)",
    },
    Field {
        name: "power",
        offset: 5,
        kind: Kind::U8,
        doc: "equipment power (+0x09)",
    },
    Field {
        name: "crit_bonus",
        offset: 6,
        kind: Kind::U8,
        doc: "equipment crit bonus (+0x0a)",
    },
    Field {
        name: "slots",
        offset: 7,
        kind: Kind::U8,
        doc: "ability slots (+0x0b); at least the number of abilities",
    },
    Field {
        name: "ability_1",
        offset: 8,
        kind: Kind::U16,
        doc: "auto-ability, 0x8000 + a_ability ID; 0 none",
    },
    Field {
        name: "ability_2",
        offset: 10,
        kind: Kind::U16,
        doc: "auto-ability; 0 none",
    },
    Field {
        name: "ability_3",
        offset: 12,
        kind: Kind::U16,
        doc: "auto-ability; 0 none",
    },
    Field {
        name: "ability_4",
        offset: 14,
        kind: Kind::U16,
        doc: "auto-ability; 0 none",
    },
];

/// Tables with one copy in `ffx_ps2/ffx/master/jppc/battle/kernel/` for
/// all languages. The game reads them as PS2 files (category `0xd`:
/// `weapon` 5, `takara` 25, `buki_get` 26; category `0x2f`: `shop_arms` 4).
pub const SHARED_SCHEMAS: &[Schema] = &[
    Schema {
        file: "buki_get",
        record_size: 16,
        strings: &[],
        fields: BUKI_GET_FIELDS,
    },
    Schema {
        file: "shop_arms",
        record_size: 22,
        strings: &[],
        fields: EQUIPMENT_FIELDS,
    },
    Schema {
        file: "takara",
        record_size: 4,
        strings: &[],
        fields: TAKARA_FIELDS,
    },
    Schema {
        file: "weapon",
        record_size: 22,
        strings: &[],
        fields: EQUIPMENT_FIELDS,
    },
];

/// Monster stats. The same 128-byte record appears in the kernel
/// `monster1`–`3` tables and in sections 2 and 6 of each monster file; the
/// battle copies section 2 into the enemy actor (`MsSetRamChr`,
/// `0x0079b4f0`; see `docs/formats/monster.md`).
pub const MONSTER_FIELDS: &[Field] = &[
    Field {
        name: "hp",
        offset: 0x14,
        kind: Kind::U32,
        doc: "max HP; actor +0x594/+0x59c",
    },
    Field {
        name: "mp",
        offset: 0x18,
        kind: Kind::U32,
        doc: "max MP; actor +0x598/+0x5a0",
    },
    Field {
        name: "overkill",
        offset: 0x1c,
        kind: Kind::U32,
        doc: "damage needed for an overkill; actor +0x5a4",
    },
    Field {
        name: "strength",
        offset: 0x20,
        kind: Kind::U8,
        doc: "actor +0x5a8; attacker stat of physical damage (MsCalcDamageCommand 0x00789bf0)",
    },
    Field {
        name: "defense",
        offset: 0x21,
        kind: Kind::U8,
        doc: "actor +0x5a9; target stat of physical damage (MsCalcDamageCommand)",
    },
    Field {
        name: "magic",
        offset: 0x22,
        kind: Kind::U8,
        doc: "actor +0x5aa; attacker stat of magic damage (MsCalcDamageCommand)",
    },
    Field {
        name: "magic_defense",
        offset: 0x23,
        kind: Kind::U8,
        doc: "actor +0x5ab; target stat of magic damage (MsCalcDamageCommand)",
    },
    Field {
        name: "stat_24",
        offset: 0x24,
        kind: Kind::U8,
        doc: "actor +0x5ac, clamped 1..255",
    },
    Field {
        name: "stat_25",
        offset: 0x25,
        kind: Kind::U8,
        doc: "actor +0x5ad, clamped 1..255",
    },
    Field {
        name: "stat_26",
        offset: 0x26,
        kind: Kind::U8,
        doc: "actor +0x5ae, clamped 0..255",
    },
    Field {
        name: "stat_27",
        offset: 0x27,
        kind: Kind::U8,
        doc: "actor +0x5af, clamped 1..255",
    },
];

/// Kernel monster text: 0 → actor +0x50 (name), 1 → +0x90, 3 → +0x190
/// (`MsBtlReadMonster`, `0x00783680`).
const MONSTER_STRINGS: &[&str] = &["name", "text_1", "text_2", "scan", "text_4"];

const NAME_HELP: &[&str] = &["name", "text_1", "description", "text_3"];
const FOUR: &[&str] = &["text_0", "text_1", "text_2", "text_3"];
const TWO: &[&str] = &["text_0", "text_1"];

/// Every table in `battle/kernel/`.
pub const SCHEMAS: &[Schema] = &[
    Schema {
        file: "a_ability",
        record_size: 108,
        strings: NAME_HELP,
        fields: A_ABILITY_FIELDS,
    },
    Schema {
        file: "arms_txt",
        record_size: 16,
        strings: FOUR,
        fields: &[],
    },
    Schema {
        file: "btl_txt",
        record_size: 8,
        strings: TWO,
        fields: &[],
    },
    Schema {
        file: "btlend_txt",
        record_size: 16,
        strings: TWO,
        fields: &[],
    },
    Schema {
        file: "build_txt",
        record_size: 16,
        strings: TWO,
        fields: &[],
    },
    Schema {
        file: "command",
        record_size: 96,
        strings: NAME_HELP,
        fields: COMMAND_FIELDS,
    },
    Schema {
        file: "config_txt",
        record_size: 16,
        strings: FOUR,
        fields: &[],
    },
    Schema {
        file: "important",
        record_size: 20,
        strings: NAME_HELP,
        fields: &[],
    },
    Schema {
        file: "item",
        record_size: 96,
        strings: NAME_HELP,
        fields: COMMAND_FIELDS,
    },
    Schema {
        file: "item_txt",
        record_size: 16,
        strings: FOUR,
        fields: &[],
    },
    Schema {
        file: "menu_txt",
        record_size: 16,
        strings: FOUR,
        fields: &[],
    },
    Schema {
        file: "mmain_txt",
        record_size: 16,
        strings: FOUR,
        fields: &[],
    },
    Schema {
        file: "monmagic1",
        record_size: 92,
        strings: NAME_HELP,
        fields: &[],
    },
    Schema {
        file: "monmagic2",
        record_size: 92,
        strings: NAME_HELP,
        fields: &[],
    },
    Schema {
        file: "monster1",
        record_size: 128,
        strings: MONSTER_STRINGS,
        fields: MONSTER_FIELDS,
    },
    Schema {
        file: "monster2",
        record_size: 128,
        strings: MONSTER_STRINGS,
        fields: MONSTER_FIELDS,
    },
    Schema {
        file: "monster3",
        record_size: 128,
        strings: MONSTER_STRINGS,
        fields: MONSTER_FIELDS,
    },
    Schema {
        file: "name_txt",
        record_size: 16,
        strings: TWO,
        fields: &[],
    },
    Schema {
        file: "panel",
        record_size: 24,
        strings: FOUR,
        fields: &[],
    },
    Schema {
        file: "ply_rom",
        record_size: 44,
        strings: FOUR,
        fields: &[],
    },
    Schema {
        file: "ply_save",
        record_size: 148,
        strings: &["name"],
        fields: &[],
    },
    Schema {
        file: "save_txt",
        record_size: 16,
        strings: TWO,
        fields: &[],
    },
    Schema {
        file: "sphere",
        record_size: 16,
        strings: TWO,
        fields: &[],
    },
    Schema {
        file: "status_txt",
        record_size: 16,
        strings: FOUR,
        fields: &[],
    },
    Schema {
        file: "summon_txt",
        record_size: 16,
        strings: FOUR,
        fields: &[],
    },
    Schema {
        file: "w_name",
        record_size: 72,
        strings: &[
            "text_0", "text_1", "text_2", "text_3", "text_4", "text_5", "text_6", "text_7",
            "text_8", "text_9", "text_10", "text_11", "text_12", "text_13",
        ],
        fields: &[],
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields_fit_and_do_not_overlap() {
        for s in SCHEMAS.iter().chain(SHARED_SCHEMAS) {
            let mut used = vec![false; usize::from(s.record_size)];
            for b in &mut used[..s.strings_len()] {
                *b = true;
            }
            for f in s.fields {
                for b in &mut used[f.offset..f.offset + f.kind.len()] {
                    assert!(!*b, "{}.{} overlaps", s.file, f.name);
                    *b = true;
                }
            }
            let mut names: Vec<_> = s
                .strings
                .iter()
                .chain(s.fields.iter().map(|f| &f.name))
                .collect();
            for n in &names {
                assert!(
                    **n != "id" && !n.starts_with("bytes_"),
                    "{}.{n} is a name the export uses itself",
                    s.file
                );
            }
            names.sort_unstable();
            names.dedup();
            assert_eq!(
                names.len(),
                s.strings.len() + s.fields.len(),
                "{} has duplicate names",
                s.file
            );
        }
    }
}
