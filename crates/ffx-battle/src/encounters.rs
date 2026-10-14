//! The encounter table, `battle/kernel/btl.bin` (`Enc_InitTable`,
//! `0x0079d250`).
//!
//! ```text
//! 0x00  u32  version (2)
//! 0x04  u32  offset of the fields (0x10)
//! 0x08  u32  offset of the encounter blocks = end of the fields
//! 0x0c  u32  end of the file
//! ```
//!
//! A field (`Enc_Field`, `0x0079d190`) is 14 bytes: `i16` map number
//! (`Enc_FieldIndex` finds the field of a map by it), `i16` offset of its
//! block from the start of the blocks, `i16` its first formation's index,
//! `char[8]` its name. Fields with the same block share their encounters.
//!
//! A block (`Enc_FieldGroups`, `0x0079d170`) is `u8` formation count
//! (wrapping), `u8` group count, then the groups (`Enc_Group`,
//! `0x0079d210`): `u8` formation count `n`, `u16` music, `u8` encounter
//! rate, `u8` weight total, then `n` × (`u8` formation id, `u8` weights).
//! The low nibble of the weights is the default weight; `Enc_ResetWeights`
//! (`0x007828a0`) copies it into the high nibble, the weight `Enc_Check`
//! (`0x00780d10`) rolls with, and sums the group's weights into the total.
//!
//! `Battle_Load` (`0x00781020`) reads formation `first + position`, where
//! the position counts the block's formations in order (`Enc_FormationIndex`,
//! `0x0079d0e0`), as PS2 file `0xe:index`: the file
//! `battle/btl/<owner>_<id>`, `<owner>` being the first field with the block.
//! So the formation counts are fixed: one formation more would make every
//! later field load its neighbour's files.

use toml::{Table, Value};

use crate::{Error, Result, int, parse_toml, u16_at, u32_at};

const FIELDS_AT: usize = 0x10;
const FIELD_LEN: usize = 14;
const NAME_LEN: usize = 8;

/// One battle field: a map and the encounters it can start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    /// The map number the field belongs to (0 for the system entry).
    pub map: i16,
    /// The name, as stored (NUL padded).
    pub name: [u8; NAME_LEN],
    /// Index into [`EncounterTable::blocks`].
    pub block: usize,
}

impl Field {
    /// The name without its padding.
    #[must_use]
    pub fn name(&self) -> String {
        let end = self.name.iter().position(|&b| b == 0).unwrap_or(NAME_LEN);
        String::from_utf8_lossy(&self.name[..end]).into_owned()
    }
}

/// A group of formations that can be met together, like those of one
/// region of a map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    /// The battle music: track `& 0x3ff`; `0x400` set for battle music,
    /// clear to keep the field music (the debug overlay's `BM`/`FM`).
    pub music: u16,
    /// Encounter rate: 0 for no random encounters.
    pub rate: u8,
    pub formations: Vec<Formation>,
}

/// One formation of a group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Formation {
    /// The formation's number within its field: file `<field>_<id>`.
    pub id: u8,
    /// Default weight (0–15): the chance of this formation is its weight
    /// over the group's weight total.
    pub weight: u8,
    /// The weight in use (high nibble); 0 in the shipped file, set from
    /// `weight` when the map is entered.
    pub current: u8,
}

/// The parsed table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncounterTable {
    pub version: u32,
    pub fields: Vec<Field>,
    /// The encounter blocks in file order; fields refer to them by index.
    pub blocks: Vec<Vec<Group>>,
}

impl EncounterTable {
    /// Parses `btl.bin`.
    ///
    /// # Errors
    /// Fails on a truncated file or tables outside it.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let err = || Error::Malformed("encounter table");
        let version = u32::try_from(u32_at(bytes, 0).ok_or_else(err)?).map_err(|_| err())?;
        let fields_at = u32_at(bytes, 4).ok_or_else(err)?;
        let blocks_at = u32_at(bytes, 8).ok_or_else(err)?;
        let end = u32_at(bytes, 12).ok_or_else(err)?;
        if fields_at != FIELDS_AT
            || blocks_at < fields_at
            || (blocks_at - fields_at) % FIELD_LEN != 0
            || end > bytes.len()
            || blocks_at > end
        {
            return Err(err());
        }
        let mut offsets: Vec<usize> = Vec::new();
        let mut blocks = Vec::new();
        let mut fields = Vec::new();
        for at in (fields_at..blocks_at).step_by(FIELD_LEN) {
            let map = u16_at(bytes, at).ok_or_else(err)?.cast_signed();
            let offset = usize::from(u16_at(bytes, at + 2).ok_or_else(err)?);
            let name = bytes[at + 6..at + FIELD_LEN].try_into().expect("8 bytes");
            let block = match offsets.iter().position(|&o| o == offset) {
                Some(b) => b,
                None => {
                    blocks.push(parse_block(&bytes[blocks_at..end], offset)?);
                    offsets.push(offset);
                    offsets.len() - 1
                }
            };
            fields.push(Field { map, name, block });
        }
        let table = Self {
            version,
            fields,
            blocks,
        };
        // Blocks follow each other in the order fields first use them, and
        // the first-formation words are running totals; writing relies on
        // both.
        let mut at = 0;
        for (o, block) in offsets.iter().zip(&table.blocks) {
            if *o != at {
                return Err(Error::Malformed("encounter blocks out of order"));
            }
            at += block_len(block);
        }
        if blocks_at + at != end {
            return Err(Error::Malformed("encounter blocks do not fill the table"));
        }
        for (i, f) in table.fields.iter().enumerate() {
            let first = u16_at(bytes, fields_at + i * FIELD_LEN + 4).ok_or_else(err)?;
            if usize::from(first) != table.first_formation(f.block) {
                return Err(Error::Malformed("first formation is not a running total"));
            }
        }
        Ok(table)
    }

    /// The index of block `block`'s first formation among all formations.
    #[must_use]
    pub fn first_formation(&self, block: usize) -> usize {
        self.blocks[..block]
            .iter()
            .map(|b| formation_count(b))
            .sum()
    }

    /// Where block `block` starts, from the start of the blocks (the
    /// field entries' block offset; the game's copy is at `*0x0112a9cc`).
    #[must_use]
    pub fn block_offset(&self, block: usize) -> usize {
        self.blocks[..block].iter().map(|b| block_len(b)).sum()
    }

    /// The name of formation file `position` of field `field` (`cdsp00_00`),
    /// counting the field's formations in order over its groups.
    #[must_use]
    pub fn formation_file(&self, field: usize, group: usize, formation: usize) -> Option<String> {
        let f = self.fields.get(field)?;
        let owner = self.fields.iter().find(|o| o.block == f.block)?;
        let id = self.blocks[f.block]
            .get(group)?
            .formations
            .get(formation)?
            .id;
        Some(format!("{}_{id:02}", owner.name()))
    }

    /// Writes the table back.
    ///
    /// # Panics
    /// If the table outgrows its 16-bit block offsets.
    #[must_use]
    pub fn write(&self) -> Vec<u8> {
        let blocks_at = FIELDS_AT + FIELD_LEN * self.fields.len();
        let mut offsets = Vec::with_capacity(self.blocks.len());
        let mut blocks = Vec::new();
        for block in &self.blocks {
            offsets.push(u16::try_from(blocks.len()).expect("16-bit block offset"));
            let count: usize = formation_count(block);
            blocks.push(count.to_le_bytes()[0]); // wraps, like the shipped 255 of zzzz00
            blocks.push(u8::try_from(block.len()).expect("at most 255 groups"));
            for g in block {
                blocks.push(u8::try_from(g.formations.len()).expect("at most 255 formations"));
                blocks.extend(g.music.to_le_bytes());
                blocks.push(g.rate);
                let total: u32 = g.formations.iter().map(|f| u32::from(f.weight)).sum();
                blocks.push(u8::try_from(total).expect("weights checked on import"));
                for f in &g.formations {
                    blocks.push(f.id);
                    blocks.push(f.current << 4 | f.weight & 0xf);
                }
            }
        }
        let end = blocks_at + blocks.len();
        let mut out = Vec::with_capacity(end);
        for word in [self.version as usize, FIELDS_AT, blocks_at, end] {
            out.extend(u32::try_from(word).expect("32-bit header").to_le_bytes());
        }
        for f in &self.fields {
            out.extend(f.map.to_le_bytes());
            out.extend(offsets[f.block].to_le_bytes());
            let first = u16::try_from(self.first_formation(f.block)).expect("16-bit index");
            out.extend(first.to_le_bytes());
            out.extend(f.name);
        }
        out.extend(blocks);
        out
    }

    /// Exports the table as TOML: one `[[field]]` per field, with its
    /// `[[field.group]]`s, or `groups_of` naming the field it shares them
    /// with.
    ///
    /// # Errors
    /// Fails only if TOML serialization fails.
    pub fn export(&self) -> Result<String> {
        let mut fields = Vec::new();
        for (i, f) in self.fields.iter().enumerate() {
            let mut t = Table::new();
            t.insert("name".into(), f.name().into());
            t.insert("map".into(), i64::from(f.map).into());
            match self.fields[..i].iter().find(|o| o.block == f.block) {
                Some(owner) => {
                    t.insert("groups_of".into(), owner.name().into());
                }
                None => {
                    let groups = self.blocks[f.block].iter().map(group_toml).collect();
                    t.insert("group".into(), Value::Array(groups));
                }
            }
            fields.push(Value::Table(t));
        }
        let mut doc = Table::new();
        doc.insert("field".into(), Value::Array(fields));
        let body = toml::to_string(&doc).map_err(|e| Error::Toml(e.to_string()))?;
        Ok(format!("{HELP}{body}"))
    }

    /// Replaces the table's contents with TOML from [`Self::export`]. The
    /// fields must stay in order, and every field must keep its number of
    /// formations: the game finds formation files by position.
    ///
    /// # Errors
    /// Fails on malformed TOML, unknown keys, values out of range, and
    /// changes to the field count or a field's formation count.
    pub fn import(&mut self, text: &str) -> Result<()> {
        let doc = parse_toml(text)?;
        for key in doc.keys() {
            if key != "field" {
                return Err(Error::Value(format!("unknown table [{key}]")));
            }
        }
        let list = doc
            .get("field")
            .and_then(Value::as_array)
            .ok_or_else(|| Error::Value("expected [[field]] tables".into()))?;
        if list.len() != self.fields.len() {
            return Err(Error::Value(format!(
                "expected {} fields, found {}",
                self.fields.len(),
                list.len()
            )));
        }
        let mut fields: Vec<Field> = Vec::with_capacity(list.len());
        let mut blocks: Vec<Vec<Group>> = Vec::new();
        for (i, v) in list.iter().enumerate() {
            let field = self.field_from_toml(i, v, &fields, &mut blocks)?;
            fields.push(field);
        }
        let new = Self {
            version: self.version,
            fields,
            blocks,
        };
        for (i, (a, b)) in self.fields.iter().zip(&new.fields).enumerate() {
            let (first_a, first_b) = (self.first_formation(a.block), new.first_formation(b.block));
            let (n_a, n_b) = (
                formation_count(&self.blocks[a.block]),
                formation_count(&new.blocks[b.block]),
            );
            if first_a != first_b || n_a != n_b {
                return Err(Error::Value(format!(
                    "field {i} ({}): {n_b} formations from index {first_b} instead of {n_a} \
                     from {first_a}; formation counts are fixed (files are found by position)",
                    b.name()
                )));
            }
        }
        *self = new;
        Ok(())
    }

    /// Field `i` of an import; its groups go to `blocks` unless it shares
    /// those of one of the `earlier` fields.
    fn field_from_toml(
        &self,
        i: usize,
        v: &Value,
        earlier: &[Field],
        blocks: &mut Vec<Vec<Group>>,
    ) -> Result<Field> {
        let t = v
            .as_table()
            .ok_or_else(|| Error::Value(format!("field {i}: expected a table")))?;
        let what = |key: &str| format!("field {i} `{key}`");
        for key in t.keys() {
            if !["name", "map", "group", "groups_of"].contains(&key.as_str()) {
                return Err(Error::Value(format!("{}: unknown key", what(key))));
            }
        }
        let old = &self.fields[i];
        let name = match t.get("name") {
            None => old.name,
            Some(v) => {
                let s = v
                    .as_str()
                    .ok_or_else(|| Error::Value(format!("{}: expected a string", what("name"))))?;
                if s == old.name() {
                    old.name
                } else if s.len() < NAME_LEN && s.is_ascii() {
                    let mut name = [0; NAME_LEN];
                    name[..s.len()].copy_from_slice(s.as_bytes());
                    name
                } else {
                    return Err(Error::Value(format!(
                        "{}: at most 7 ASCII characters",
                        what("name")
                    )));
                }
            }
        };
        let map = match t.get("map") {
            None => old.map,
            Some(v) => int(v, &what("map"))?,
        };
        let block = match (t.get("groups_of"), t.get("group")) {
            (Some(_), Some(_)) => {
                return Err(Error::Value(format!(
                    "field {i}: either `group` or `groups_of`, not both"
                )));
            }
            (Some(owner), None) => {
                let owner = owner.as_str().unwrap_or_default();
                earlier
                    .iter()
                    .find(|f| f.name() == owner)
                    .map(|f| f.block)
                    .ok_or_else(|| {
                        Error::Value(format!(
                            "{}: no earlier field is named {owner:?}",
                            what("groups_of")
                        ))
                    })?
            }
            (None, Some(groups)) => {
                let groups = groups
                    .as_array()
                    .ok_or_else(|| Error::Value(format!("{}: expected tables", what("group"))))?
                    .iter()
                    .enumerate()
                    .map(|(g, v)| group_from_toml(v, &format!("field {i} group {g}")))
                    .collect::<Result<Vec<_>>>()?;
                blocks.push(groups);
                blocks.len() - 1
            }
            (None, None) => {
                return Err(Error::Value(format!(
                    "field {i}: needs `group` tables or `groups_of`"
                )));
            }
        };
        Ok(Field { map, name, block })
    }
}

const HELP: &str = "\
# Battle fields and their encounter groups.
# A group's formations are [id, weight]: id names the formation file
# <field>_<id> (battle/btl/), weight (0-15) its share of the group's
# random encounters. rate 0 means no random encounters. music: track & 0x3ff,
# + 0x400 for battle music. A field's number of formations cannot change.
# A third number in a formation, if present, is its current weight.

";

fn formation_count(block: &[Group]) -> usize {
    block.iter().map(|g| g.formations.len()).sum()
}

fn block_len(block: &[Group]) -> usize {
    2 + block
        .iter()
        .map(|g| 5 + 2 * g.formations.len())
        .sum::<usize>()
}

fn parse_block(blocks: &[u8], at: usize) -> Result<Vec<Group>> {
    let err = || Error::Malformed("encounter block");
    let count = usize::from(*blocks.get(at + 1).ok_or_else(err)?);
    let mut p = at + 2;
    let mut groups = Vec::with_capacity(count);
    for _ in 0..count {
        let head = blocks.get(p..p + 5).ok_or_else(err)?;
        let n = usize::from(head[0]);
        let entries = blocks.get(p + 5..p + 5 + 2 * n).ok_or_else(err)?;
        let formations: Vec<Formation> = entries
            .as_chunks::<2>()
            .0
            .iter()
            .map(|e| Formation {
                id: e[0],
                weight: e[1] & 0xf,
                current: e[1] >> 4,
            })
            .collect();
        let total: u32 = formations.iter().map(|f| u32::from(f.weight)).sum();
        if u32::from(head[4]) != total {
            return Err(Error::Malformed("group weight total"));
        }
        groups.push(Group {
            music: u16::from_le_bytes([head[1], head[2]]),
            rate: head[3],
            formations,
        });
        p += 5 + 2 * n;
    }
    let total: usize = groups.iter().map(|g| g.formations.len()).sum();
    if blocks[at] != total.to_le_bytes()[0] {
        return Err(Error::Malformed("block formation count"));
    }
    Ok(groups)
}

fn group_toml(g: &Group) -> Value {
    let mut t = Table::new();
    t.insert("music".into(), i64::from(g.music).into());
    t.insert("rate".into(), i64::from(g.rate).into());
    let formations = g
        .formations
        .iter()
        .map(|f| {
            let mut v = vec![i64::from(f.id).into(), i64::from(f.weight).into()];
            if f.current != 0 {
                v.push(i64::from(f.current).into());
            }
            Value::Array(v)
        })
        .collect();
    t.insert("formations".into(), Value::Array(formations));
    Value::Table(t)
}

fn group_from_toml(v: &Value, what: &str) -> Result<Group> {
    let t = v
        .as_table()
        .ok_or_else(|| Error::Value(format!("{what}: expected a table")))?;
    for key in t.keys() {
        if !["music", "rate", "formations"].contains(&key.as_str()) {
            return Err(Error::Value(format!("{what} `{key}`: unknown key")));
        }
    }
    let get = |key: &str| {
        t.get(key)
            .ok_or_else(|| Error::Value(format!("{what}: `{key}` is missing")))
    };
    let formations = get("formations")?
        .as_array()
        .ok_or_else(|| Error::Value(format!("{what} `formations`: expected an array")))?
        .iter()
        .map(|f| {
            let what = format!("{what} formation {f}");
            let v = f
                .as_array()
                .filter(|v| (2..=3).contains(&v.len()))
                .ok_or_else(|| Error::Value(format!("{what}: expected [id, weight]")))?;
            let nibble = |v: &Value| -> Result<u8> {
                let n: u8 = int(v, &what)?;
                if n > 15 {
                    return Err(Error::Value(format!("{what}: weights are 0-15")));
                }
                Ok(n)
            };
            Ok(Formation {
                id: int(&v[0], &what)?,
                weight: nibble(&v[1])?,
                current: v.get(2).map_or(Ok(0), nibble)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let total: u32 = formations.iter().map(|f| u32::from(f.weight)).sum();
    if total > 255 {
        return Err(Error::Value(format!(
            "{what}: weights add up to more than 255"
        )));
    }
    Ok(Group {
        music: int(get("music")?, &format!("{what} `music`"))?,
        rate: int(get("rate")?, &format!("{what} `rate`"))?,
        formations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two fields sharing a block, then one with two groups.
    fn sample() -> Vec<u8> {
        let mut blocks = vec![2, 1, 2, 0x02, 0x04, 60, 3, 0, 1, 1, 2];
        blocks.extend([3, 2, 1, 0x07, 0x04, 0, 0, 0, 0]);
        blocks.extend([2, 0x07, 0x00, 50, 5, 20, 4, 21, 1]);
        let fields_end = 0x10 + 3 * 14;
        let end = fields_end + blocks.len();
        let mut f = Vec::new();
        for w in [2, 0x10, fields_end, end].map(|w: usize| u32::try_from(w).unwrap()) {
            f.extend(w.to_le_bytes());
        }
        for (map, off, first, name) in [
            (50i16, 0u16, 0u16, b"cdsp00\0\0"),
            (51, 0, 0, b"cdsp01\0\0"),
            (210, 11, 2, b"mihn00\0\0"),
        ] {
            f.extend(map.to_le_bytes());
            f.extend(off.to_le_bytes());
            f.extend(first.to_le_bytes());
            f.extend(name);
        }
        f.extend(blocks);
        f
    }

    #[test]
    fn parses_writes_and_round_trips_toml() {
        let bytes = sample();
        let table = EncounterTable::parse(&bytes).unwrap();
        assert_eq!(table.blocks.len(), 2);
        assert_eq!(table.fields[1].block, 0);
        assert_eq!(table.first_formation(1), 2);
        assert_eq!(table.block_offset(1), 11);
        assert_eq!(table.blocks[1][1].formations[1].weight, 1);
        assert_eq!(table.formation_file(1, 0, 1).as_deref(), Some("cdsp00_01"));
        assert_eq!(table.formation_file(2, 1, 0).as_deref(), Some("mihn00_20"));
        assert_eq!(table.write(), bytes);

        let text = table.export().unwrap();
        assert!(text.contains("groups_of = \"cdsp00\""), "{text}");
        let mut copy = table.clone();
        copy.import(&text).unwrap();
        assert_eq!(copy, table);

        let edited = text.replace("[20, 4]", "[20, 9]");
        copy.import(&edited).unwrap();
        let out = copy.write();
        assert_eq!(out.len(), bytes.len());
        assert_eq!(
            EncounterTable::parse(&out).unwrap().blocks[1][1].formations[0].weight,
            9
        );
        // the group's weight total follows
        assert_eq!(out[out.len() - 5], 10);

        let grown = text.replace("[20, 4]", "[20, 4], [22, 1]");
        assert!(table.clone().import(&grown).is_err());
        assert!(
            table
                .clone()
                .import(&text.replace("[20, 4]", "[20, 16]"))
                .is_err()
        );
    }
}
