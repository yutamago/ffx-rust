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
    /// File name without `.bin`, as passed to `Kernel_LoadFile`.
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
        SCHEMAS.iter().find(|s| s.file == file)
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
        name: "flags",
        offset: 0x1c,
        kind: Kind::U32,
        doc: "bit 0x2: listed for use (Item_BuildLists 0x007906a0); bit 0x400 selects the second list",
    },
    Field {
        name: "formula",
        offset: 0x28,
        kind: Kind::U8,
        doc: "healing formula (Cmd_HealAmount 0x008c6920): 6 = power x 50, 7 = (stat + power) / 2 x power, 8 = max HP x power / 16",
    },
    Field {
        name: "power",
        offset: 0x2a,
        kind: Kind::U8,
        doc: "formula input (Cmd_HealAmount 0x008c6920); Potion 4 = 200 HP",
    },
];

const NAME_HELP: &[&str] = &["name", "text_1", "description", "text_3"];
const FOUR: &[&str] = &["text_0", "text_1", "text_2", "text_3"];
const TWO: &[&str] = &["text_0", "text_1"];

/// Every table in `battle/kernel/`.
pub const SCHEMAS: &[Schema] = &[
    Schema {
        file: "a_ability",
        record_size: 108,
        strings: NAME_HELP,
        fields: &[],
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
        strings: &["name", "text_1", "scan", "text_3", "text_4"],
        fields: &[],
    },
    Schema {
        file: "monster2",
        record_size: 128,
        strings: &["name", "text_1", "scan", "text_3", "text_4"],
        fields: &[],
    },
    Schema {
        file: "monster3",
        record_size: 128,
        strings: &["name", "text_1", "scan", "text_3", "text_4"],
        fields: &[],
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
        for s in SCHEMAS {
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
