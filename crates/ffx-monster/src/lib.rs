//! FFX's monster files, `ffx_ps2/ffx/master/jppc/battle/mon/_mNNN/mNNN.bin`
//! (one per monster, shared by all languages).
//!
//! A file is a table of sections (`Mon_AttachData`, `0x00783b00`):
//!
//! ```text
//! 0x00  u32       section count (8 in every file)
//! 0x04  u32 * 8   section offsets from the file start; 0 = absent
//! ```
//!
//! | Section | Use |
//! |---|---|
//! | 0 | battle AI script → actor `+0xf78` |
//! | 1 | script dispatch table (`Mon_FindHandler`, `0x00797410`) → `+0xf7c` |
//! | 2 | stat record (128 bytes, then Japanese strings) → `+0xf80`, copied into the actor by `Mon_InitActor` (`0x0079b4f0`) |
//! | 4 | rewards, steal and bribe (`0x118` bytes) → copied per enemy slot, `+0xf88` |
//! | 5 | read by `0x007a01b0` when the actor asks for it |
//! | 6 | a second copy of the stat record → `+0xf84` |
//!
//! Edits are made in place, so the layout never changes. The format is
//! described in `docs/formats/monster.md`.

use ffx_kernel::record::Layout;
use ffx_kernel::schema::{Field, Kind, MONSTER_FIELDS};
use toml::{Table as TomlTable, Value};

/// Number of section slots in every shipped file.
pub const SECTIONS: usize = 8;
/// Section with the stat record.
pub const STATS: usize = 2;
/// Section with a second copy of the stat record.
pub const STATS_COPY: usize = 6;
/// Section with rewards, steal and bribe.
pub const SPOILS: usize = 4;
/// Size of the stat record.
pub const STATS_LEN: usize = 128;
/// Size of the reward block the battle copies (`Mon_InitActor`).
pub const SPOILS_LEN: usize = 0x118;
/// Bytes of the stat record before the stats: five string references.
const STATS_PREFIX: usize = 20;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("malformed monster file: {0}")]
    Malformed(&'static str),
    #[error("TOML: {0}")]
    Toml(String),
    #[error("[{table}] `{key}`: {problem}")]
    Value {
        table: &'static str,
        key: String,
        problem: String,
    },
}

pub type Result<T> = std::result::Result<T, Error>;

/// Rewards, steal and bribe (`Battle_GiveRewards` `0x007990d0`,
/// `Battle_Steal` `0x0078b760`).
pub const SPOILS_FIELDS: &[Field] = &[
    Field {
        name: "gil",
        offset: 0x00,
        kind: Kind::U16,
        doc: "gil on victory",
    },
    Field {
        name: "ap",
        offset: 0x02,
        kind: Kind::U16,
        doc: "AP on victory",
    },
    Field {
        name: "ap_overkill",
        offset: 0x04,
        kind: Kind::U16,
        doc: "AP on an overkill",
    },
    Field {
        name: "drop_chance_1",
        offset: 0x08,
        kind: Kind::U8,
        doc: "x/255 chance of drop slot 1",
    },
    Field {
        name: "drop_chance_2",
        offset: 0x09,
        kind: Kind::U8,
        doc: "x/255 chance of drop slot 2",
    },
    Field {
        name: "steal_chance",
        offset: 0x0a,
        kind: Kind::U8,
        doc: "x/255; halved after each successful steal",
    },
    Field {
        name: "equipment_chance",
        offset: 0x0b,
        kind: Kind::U8,
        doc: "x/255 chance of an equipment drop (0x00798c10)",
    },
    Field {
        name: "drop_1",
        offset: 0x0c,
        kind: Kind::U16,
        doc: "slot 1 item (0x2000 + item ID)",
    },
    Field {
        name: "drop_1_rare",
        offset: 0x0e,
        kind: Kind::U16,
        doc: "slot 1 rare item (32/256)",
    },
    Field {
        name: "drop_2",
        offset: 0x10,
        kind: Kind::U16,
        doc: "slot 2 item",
    },
    Field {
        name: "drop_2_rare",
        offset: 0x12,
        kind: Kind::U16,
        doc: "slot 2 rare item",
    },
    Field {
        name: "drop_1_count",
        offset: 0x14,
        kind: Kind::U8,
        doc: "",
    },
    Field {
        name: "drop_1_rare_count",
        offset: 0x15,
        kind: Kind::U8,
        doc: "",
    },
    Field {
        name: "drop_2_count",
        offset: 0x16,
        kind: Kind::U8,
        doc: "",
    },
    Field {
        name: "drop_2_rare_count",
        offset: 0x17,
        kind: Kind::U8,
        doc: "",
    },
    Field {
        name: "overkill_drop_1",
        offset: 0x18,
        kind: Kind::U16,
        doc: "slot 1 item on an overkill",
    },
    Field {
        name: "overkill_drop_1_rare",
        offset: 0x1a,
        kind: Kind::U16,
        doc: "",
    },
    Field {
        name: "overkill_drop_2",
        offset: 0x1c,
        kind: Kind::U16,
        doc: "",
    },
    Field {
        name: "overkill_drop_2_rare",
        offset: 0x1e,
        kind: Kind::U16,
        doc: "",
    },
    Field {
        name: "overkill_drop_1_count",
        offset: 0x20,
        kind: Kind::U8,
        doc: "",
    },
    Field {
        name: "overkill_drop_1_rare_count",
        offset: 0x21,
        kind: Kind::U8,
        doc: "",
    },
    Field {
        name: "overkill_drop_2_count",
        offset: 0x22,
        kind: Kind::U8,
        doc: "",
    },
    Field {
        name: "overkill_drop_2_rare_count",
        offset: 0x23,
        kind: Kind::U8,
        doc: "",
    },
    Field {
        name: "steal",
        offset: 0x24,
        kind: Kind::U16,
        doc: "steal item",
    },
    Field {
        name: "steal_rare",
        offset: 0x26,
        kind: Kind::U16,
        doc: "rare steal item (32/256, more with abilities)",
    },
    Field {
        name: "steal_count",
        offset: 0x28,
        kind: Kind::U8,
        doc: "",
    },
    Field {
        name: "steal_rare_count",
        offset: 0x29,
        kind: Kind::U8,
        doc: "",
    },
    Field {
        name: "bribe",
        offset: 0x2a,
        kind: Kind::U16,
        doc: "item a Bribe gives",
    },
];

const STATS_LAYOUT: Layout<'static> = Layout {
    size: STATS_LEN,
    prefix: STATS_PREFIX,
    fields: MONSTER_FIELDS,
};
const SPOILS_LAYOUT: Layout<'static> = Layout {
    size: SPOILS_LEN,
    prefix: 0,
    fields: SPOILS_FIELDS,
};

/// A parsed monster file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonsterFile {
    bytes: Vec<u8>,
    /// (offset, length) of each present section.
    sections: [Option<(usize, usize)>; SECTIONS],
}

impl MonsterFile {
    /// Parses a monster file.
    ///
    /// # Errors
    /// Fails if the header is truncated, or the sections are not in
    /// ascending order inside the file, or the stat and reward sections are
    /// too short.
    pub fn parse(bytes: Vec<u8>) -> Result<Self> {
        let word = |i: usize| -> Result<usize> {
            bytes
                .get(i * 4..i * 4 + 4)
                .map(|b| u32::from_le_bytes(b.try_into().expect("4 bytes")) as usize)
                .ok_or(Error::Malformed("truncated header"))
        };
        if word(0)? != SECTIONS {
            return Err(Error::Malformed("expected 8 sections"));
        }
        let offsets: Vec<usize> = (1..=SECTIONS).map(word).collect::<Result<_>>()?;
        let present: Vec<usize> = offsets.iter().copied().filter(|&o| o != 0).collect();
        if present.windows(2).any(|w| w[0] > w[1]) || present.iter().any(|&o| o > bytes.len()) {
            return Err(Error::Malformed("sections out of order or out of range"));
        }
        let mut sections = [None; SECTIONS];
        for (i, &o) in offsets.iter().enumerate() {
            if o != 0 {
                let end = present
                    .iter()
                    .copied()
                    .find(|&n| n > o)
                    .unwrap_or(bytes.len());
                sections[i] = Some((o, end - o));
            }
        }
        let file = Self { bytes, sections };
        for (s, need) in [
            (STATS, STATS_LEN),
            (STATS_COPY, STATS_LEN),
            (SPOILS, SPOILS_LEN),
        ] {
            if file.section(s).is_some_and(|b| b.len() < need) {
                return Err(Error::Malformed("stat or reward section too short"));
            }
        }
        Ok(file)
    }

    /// The file's bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// The bytes of section `i`, if present.
    #[must_use]
    pub fn section(&self, i: usize) -> Option<&[u8]> {
        let (o, len) = (*self.sections.get(i)?)?;
        Some(&self.bytes[o..o + len])
    }

    fn section_range(&self, i: usize, len: usize) -> Option<std::ops::Range<usize>> {
        let (o, _) = (*self.sections.get(i)?)?;
        Some(o..o + len)
    }

    /// The stat record (section 2).
    #[must_use]
    pub fn stats(&self) -> Option<&[u8]> {
        Some(&self.section(STATS)?[..STATS_LEN])
    }

    /// The reward block (section 4).
    #[must_use]
    pub fn spoils(&self) -> Option<&[u8]> {
        Some(&self.section(SPOILS)?[..SPOILS_LEN])
    }

    /// Exports stats and rewards as TOML (`[stats]`, `[spoils]`).
    ///
    /// # Errors
    /// Fails only if TOML serialization fails.
    pub fn export(&self) -> Result<String> {
        let mut doc = TomlTable::new();
        if let Some(stats) = self.stats() {
            let mut t = TomlTable::new();
            STATS_LAYOUT.export(stats, &mut t);
            doc.insert("stats".into(), Value::Table(t));
        }
        if let Some(spoils) = self.spoils() {
            let mut t = TomlTable::new();
            SPOILS_LAYOUT.export(spoils, &mut t);
            doc.insert("spoils".into(), Value::Table(t));
        }
        toml::to_string(&doc).map_err(|e| Error::Toml(e.to_string()))
    }

    /// Applies TOML onto the file. Keys that are absent keep their values.
    /// Stat edits go to section 2 and to its copy in section 6, which ship
    /// identical in every file.
    ///
    /// # Errors
    /// Fails on unknown tables or keys, values out of range and malformed hex.
    pub fn import(&mut self, toml_text: &str) -> Result<()> {
        let doc: TomlTable = toml_text
            .parse()
            .map_err(|e: toml::de::Error| Error::Toml(e.to_string()))?;
        for (table, value) in &doc {
            let (name, layout, ranges): (&'static str, Layout<'_>, Vec<_>) = match table.as_str() {
                "stats" => (
                    "stats",
                    STATS_LAYOUT,
                    [STATS, STATS_COPY]
                        .iter()
                        .filter_map(|&s| self.section_range(s, STATS_LEN))
                        .collect(),
                ),
                "spoils" => (
                    "spoils",
                    SPOILS_LAYOUT,
                    self.section_range(SPOILS, SPOILS_LEN).into_iter().collect(),
                ),
                other => {
                    return Err(Error::Toml(format!(
                        "unknown table [{other}]; expected [stats] or [spoils]"
                    )));
                }
            };
            let entries = value
                .as_table()
                .ok_or(Error::Toml(format!("[{name}] must be a table")))?;
            for range in ranges {
                for (key, v) in entries {
                    let err = |problem: String| Error::Value {
                        table: name,
                        key: key.clone(),
                        problem,
                    };
                    match layout.import(&mut self.bytes[range.clone()], key, v) {
                        Some(r) => r.map_err(err)?,
                        None => return Err(err("unknown key".into())),
                    }
                }
            }
        }
        Ok(())
    }
}

/// Archive path of monster `id`'s file.
#[must_use]
pub fn archive_path(id: u16) -> String {
    format!("ffx_ps2/ffx/master/jppc/battle/mon/_m{id:03}/m{id:03}.bin")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<u8> {
        let mut f = vec![0u8; 0x30];
        f[0] = 8;
        let offsets = [0x30u32, 0x34, 0x38, 0, 0xb8, 0x1d8, 0x1dc, 0x25c];
        for (i, o) in offsets.iter().enumerate() {
            f[4 + i * 4..8 + i * 4].copy_from_slice(&o.to_le_bytes());
        }
        f.resize(0x25c, 0);
        f[0x38 + 0x14] = 240; // hp
        f[0x1dc + 0x14] = 240;
        f[0xb8] = 42; // gil
        f
    }

    #[test]
    fn exports_and_edits_in_place() {
        let mut m = MonsterFile::parse(sample()).unwrap();
        assert_eq!(m.section(3), None);
        assert_eq!(m.section(7), Some(&[][..]));
        let text = m.export().unwrap();
        assert!(
            text.contains("hp = 240") && text.contains("gil = 42"),
            "{text}"
        );

        let mut same = m.clone();
        same.import(&text).unwrap();
        assert_eq!(same, m);

        m.import("[stats]\nhp = 9999\n[spoils]\ngil = 500\n")
            .unwrap();
        assert_eq!(m.bytes()[0x38 + 0x14..0x38 + 0x16], 9999u16.to_le_bytes());
        assert_eq!(m.bytes()[0x1dc + 0x14..0x1dc + 0x16], 9999u16.to_le_bytes());
        assert_eq!(m.bytes()[0xb8..0xba], 500u16.to_le_bytes());
        assert!(m.import("[stats]\nname = 1\n").is_err());
        assert!(m.import("[loot]\ngil = 1\n").is_err());
    }
}
