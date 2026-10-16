//! Formation files, `battle/btl/<field>_<id>/<field>_<id>.bin`.
//!
//! The same section table as the monster files: `u32` count (8; 7 or 5 in
//! a few), then the section offsets, 0 for absent ones, in ascending order
//! (in 15 files the last ones point 16 bytes past the end: empty sections).
//! `MsBtlReadSetScene` (`0x00783e20`) takes four of them:
//!
//! | Section | Use |
//! |---|---|
//! | 0 | battle event script (`ffx-script` reads it) → `0x0112a9ac` |
//! | 1 | its first byte → `0x0112a9a4` |
//! | 2 | the formation, 28 bytes → `0x0112a9c0` |
//! | 3 | positions: `0x20`-byte records with counts at `+4..+6` → `0x0112a9b0` |
//!
//! The formation (the debug overlay prints bytes 0–2 as `V%dR%dF%d` and the
//! slots as `M%d:%03d`):
//!
//! ```text
//! 0x00  u8       unknown_0: with byte 3 clear, battle signals also go to
//!                the second script set (0x007986a0)
//! 0x01  u8       unknown_1: 1 sends request 0xc with 1 at battle start
//!                (0x0081ec20); 0 or 2 in the shipped files
//! 0x02  u8       footsteps: the footstep sound bank (MsBtlReadScene)
//! 0x03  u8       underwater (MsBattleWater)
//! 0x04  u8 * 8   0 in every file
//! 0x0c  u16 * 8  monster slots: monster id 0x1000 | n for mNNN; 0xffff empty
//!                (MsSetRamChrData, 0x0079c110)
//! ```
//!
//! Edits are made in place, so the layout never changes.

use toml::{Table, Value};

use crate::{Error, Result, int, parse_toml, u16_at, u32_at};

/// The formation section.
pub const FORMATION: usize = 2;
/// Monster slots of a formation.
pub const MONSTER_SLOTS: usize = 8;
const FORMATION_LEN: usize = 0x1c;
const SLOTS_AT: usize = 0x0c;
const EMPTY: u16 = 0xffff;

/// A parsed formation file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormationFile {
    bytes: Vec<u8>,
    /// (offset, length) of each present section.
    sections: Vec<Option<(usize, usize)>>,
}

const BYTES: [(&str, usize); 4] = [
    ("unknown_0", 0),
    ("unknown_1", 1),
    ("footsteps", 2),
    ("underwater", 3),
];

impl FormationFile {
    /// Parses a formation file.
    ///
    /// # Errors
    /// Fails if the section table is truncated or out of order.
    pub fn parse(bytes: Vec<u8>) -> Result<Self> {
        let err = || Error::Malformed("formation file header");
        let count = u32_at(&bytes, 0).ok_or_else(err)?;
        if count == 0 || count > 16 {
            return Err(err());
        }
        let offsets: Vec<usize> = (1..=count)
            .map(|i| u32_at(&bytes, 4 * i).ok_or_else(err))
            .collect::<Result<_>>()?;
        let present: Vec<usize> = offsets.iter().copied().filter(|&o| o != 0).collect();
        if present.windows(2).any(|w| w[0] > w[1]) {
            return Err(Error::Malformed("formation file sections out of order"));
        }
        // Some 7-section files point their last sections past the end of
        // the file (by 16 bytes): those are empty.
        let sections = offsets
            .iter()
            .map(|&o| {
                (o != 0).then(|| {
                    let o = o.min(bytes.len());
                    let end = present
                        .iter()
                        .copied()
                        .find(|&n| n > o)
                        .unwrap_or(bytes.len());
                    (o, end.min(bytes.len()) - o)
                })
            })
            .collect();
        Ok(Self { bytes, sections })
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

    /// Where the formation is, if the file has one: a few old files
    /// (`klyt01_01`…) have a shorter section 2 of another layout.
    fn formation_at(&self) -> Option<usize> {
        let (o, len) = (*self.sections.get(FORMATION)?)?;
        (len >= FORMATION_LEN).then_some(o)
    }

    /// The formation's 28 bytes.
    #[must_use]
    pub fn formation(&self) -> Option<&[u8]> {
        let o = self.formation_at()?;
        Some(&self.bytes[o..o + FORMATION_LEN])
    }

    /// The monster ids of the eight slots (`None`: empty).
    #[must_use]
    pub fn monsters(&self) -> Option<[Option<u16>; MONSTER_SLOTS]> {
        let f = self.formation()?;
        Some(std::array::from_fn(|i| {
            let id = u16_at(f, SLOTS_AT + 2 * i).expect("inside the formation");
            (id != EMPTY).then_some(id)
        }))
    }

    /// Puts monster `id` (`None`: empty) into slot `slot`.
    ///
    /// # Errors
    /// Fails if the file has no formation or the slot does not exist.
    pub fn set_monster(&mut self, slot: usize, id: Option<u16>) -> Result<()> {
        if slot >= MONSTER_SLOTS {
            return Err(Error::Value(format!("slot {slot}: there are 8 slots")));
        }
        let o = self
            .formation_at()
            .ok_or(Error::Malformed("file without a formation"))?
            + SLOTS_AT
            + 2 * slot;
        self.bytes[o..o + 2].copy_from_slice(&id.unwrap_or(EMPTY).to_le_bytes());
        Ok(())
    }

    /// Exports the formation as TOML (`[formation]`); empty for a file
    /// without one.
    ///
    /// # Errors
    /// Fails only if TOML serialization fails.
    pub fn export(&self) -> Result<String> {
        let (Some(f), Some(monsters)) = (self.formation(), self.monsters()) else {
            return Ok(String::new());
        };
        let mut t = Table::new();
        let used = monsters
            .iter()
            .rposition(Option::is_some)
            .map_or(0, |i| i + 1);
        let list = monsters[..used].iter().map(|&m| monster_name(m)).collect();
        t.insert("monsters".into(), Value::Array(list));
        for (key, at) in BYTES {
            let v = f[at];
            if key == "underwater" && v <= 1 {
                t.insert(key.into(), (v == 1).into());
            } else {
                t.insert(key.into(), i64::from(v).into());
            }
        }
        let mut doc = Table::new();
        doc.insert("formation".into(), Value::Table(t));
        let body = toml::to_string(&doc).map_err(|e| Error::Toml(e.to_string()))?;
        Ok(format!(
            "# monsters: up to 8 slots, \"mNNN\" (battle/mon/_mNNN) or \"-\" for an empty slot\n{body}"
        ))
    }

    /// Applies TOML onto the file. Keys that are absent keep their values;
    /// `monsters` sets all eight slots (missing ones become empty).
    ///
    /// # Errors
    /// Fails on unknown tables or keys, values out of range, and files
    /// without a formation.
    pub fn import(&mut self, text: &str) -> Result<()> {
        let doc = parse_toml(text)?;
        for (table, value) in &doc {
            if table != "formation" {
                return Err(Error::Value(format!(
                    "unknown table [{table}]; expected [formation]"
                )));
            }
            let at = self
                .formation_at()
                .ok_or(Error::Malformed("file without a formation"))?;
            let t = value
                .as_table()
                .ok_or_else(|| Error::Value("[formation] must be a table".into()))?;
            for (key, v) in t {
                if key == "monsters" {
                    let list = v
                        .as_array()
                        .filter(|l| l.len() <= MONSTER_SLOTS)
                        .ok_or_else(|| Error::Value("`monsters`: up to 8 entries".into()))?;
                    for slot in 0..MONSTER_SLOTS {
                        let id = list.get(slot).map(monster_id).transpose()?.flatten();
                        self.set_monster(slot, id)?;
                    }
                } else if let Some(&(_, o)) = BYTES.iter().find(|(k, _)| k == key) {
                    let value = match v.as_bool() {
                        Some(b) => u8::from(b),
                        None => int(v, &format!("`{key}`"))?,
                    };
                    self.bytes[at + o] = value;
                } else {
                    return Err(Error::Value(format!("[formation] `{key}`: unknown key")));
                }
            }
        }
        Ok(())
    }
}

fn monster_name(id: Option<u16>) -> Value {
    match id {
        None => "-".into(),
        Some(id) if id >> 12 == 1 && id & 0xfff < 1000 => format!("m{:03}", id & 0xfff).into(),
        Some(id) => i64::from(id).into(),
    }
}

fn monster_id(v: &Value) -> Result<Option<u16>> {
    let bad = || {
        Error::Value(format!(
            "`monsters`: {v} is not \"mNNN\", \"-\" or a number"
        ))
    };
    match v {
        Value::String(s) if s == "-" => Ok(None),
        Value::String(s) => {
            let n: u16 = s
                .strip_prefix('m')
                .filter(|n| n.len() == 3)
                .and_then(|n| n.parse().ok())
                .ok_or_else(bad)?;
            Ok(Some(0x1000 | n))
        }
        Value::Integer(_) => Ok(Some(int(v, "`monsters`")?)),
        _ => Err(bad()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<u8> {
        let mut f = vec![0u8; 0x30];
        f[0] = 8;
        let offsets = [0x30u32, 0x34, 0x38, 0x54, 0, 0, 0, 0x58];
        for (i, o) in offsets.iter().enumerate() {
            f[4 + i * 4..8 + i * 4].copy_from_slice(&o.to_le_bytes());
        }
        f.resize(0x58, 0);
        let formation = [1u8, 0, 16, 1, 0, 0, 0, 0, 0, 0, 0, 0];
        f[0x38..0x44].copy_from_slice(&formation);
        for slot in 0..8 {
            let id: u16 = match slot {
                0 => 0x1044,
                1 => 0x1045,
                _ => 0xffff,
            };
            f[0x44 + 2 * slot..0x46 + 2 * slot].copy_from_slice(&id.to_le_bytes());
        }
        f
    }

    #[test]
    fn exports_and_edits_in_place() {
        let bytes = sample();
        let mut file = FormationFile::parse(bytes.clone()).unwrap();
        assert_eq!(
            file.monsters().unwrap()[..3],
            [Some(0x1044), Some(0x1045), None]
        );
        let text = file.export().unwrap();
        assert!(text.contains(r#"monsters = ["m068", "m069"]"#), "{text}");
        assert!(
            text.contains("underwater = true") && text.contains("footsteps = 16"),
            "{text}"
        );

        let mut same = file.clone();
        same.import(&text).unwrap();
        assert_eq!(same.bytes(), &bytes[..]);

        file.import("[formation]\nmonsters = [\"-\", \"m001\", \"m001\"]\nunderwater = false\n")
            .unwrap();
        assert_eq!(
            file.monsters().unwrap()[..4],
            [None, Some(0x1001), Some(0x1001), None]
        );
        assert_eq!(file.bytes()[0x38 + 3], 0);
        assert_eq!(file.bytes().len(), bytes.len());
        assert!(file.import("[formation]\nmonsters = [\"x1\"]\n").is_err());
        assert!(file.import("[formation]\nboss = 1\n").is_err());
    }
}
