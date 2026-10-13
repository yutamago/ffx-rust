//! Fixed-size records as TOML: named integer fields, and every other byte as
//! hex runs keyed by offset (`bytes_2b = "01 00 …"`), so a record survives
//! an export/import round trip exactly. Shared by the kernel tables and the
//! monster files.

use std::fmt::Write;

use toml::{Table as TomlTable, Value};

use crate::schema::{Field, Kind};

/// The fields of a record, after an optional prefix handled elsewhere (the
/// string references of kernel records).
#[derive(Debug, Clone, Copy)]
pub struct Layout<'a> {
    pub size: usize,
    /// Leading bytes this module neither exports nor imports.
    pub prefix: usize,
    pub fields: &'a [Field],
}

impl Layout<'_> {
    /// Byte runs no field covers: (offset, length).
    #[must_use]
    pub fn unnamed(&self) -> Vec<(usize, usize)> {
        let mut used = vec![false; self.size];
        used[..self.prefix].fill(true);
        for f in self.fields {
            used[f.offset..f.offset + f.kind.len()].fill(true);
        }
        let mut runs = Vec::new();
        let mut i = 0;
        while i < used.len() {
            if used[i] {
                i += 1;
                continue;
            }
            let start = i;
            while i < used.len() && !used[i] {
                i += 1;
            }
            runs.push((start, i - start));
        }
        runs
    }

    /// Adds the record's fields and hex runs to `out`.
    pub fn export(&self, record: &[u8], out: &mut TomlTable) {
        for f in self.fields {
            out.insert(
                f.name.into(),
                Value::Integer(read_int(record, f.offset, f.kind)),
            );
        }
        for (offset, len) in self.unnamed() {
            out.insert(
                format!("bytes_{offset:02x}"),
                Value::String(hex(&record[offset..offset + len])),
            );
        }
    }

    /// Applies one key to the record. Returns `None` if the key is neither a
    /// field nor a hex run of this layout.
    pub fn import(
        &self,
        record: &mut [u8],
        key: &str,
        value: &Value,
    ) -> Option<Result<(), String>> {
        if let Some(f) = self.fields.iter().find(|f| f.name == key) {
            return Some(write_int(record, f, value));
        }
        let offset = usize::from_str_radix(key.strip_prefix("bytes_")?, 16).ok()?;
        let &(offset, len) = self.unnamed().iter().find(|r| r.0 == offset)?;
        Some((|| {
            let text = value.as_str().ok_or("expected a hex string")?;
            let bytes = parse_hex(text).ok_or("malformed hex")?;
            if bytes.len() != len {
                return Err(format!("expected {len} bytes, got {}", bytes.len()));
            }
            record[offset..offset + len].copy_from_slice(&bytes);
            Ok(())
        })())
    }
}

fn read_int(record: &[u8], offset: usize, kind: Kind) -> i64 {
    let b = &record[offset..offset + kind.len()];
    match kind {
        Kind::U8 => i64::from(b[0]),
        Kind::U16 => i64::from(u16::from_le_bytes([b[0], b[1]])),
        Kind::U32 => i64::from(u32::from_le_bytes([b[0], b[1], b[2], b[3]])),
    }
}

fn write_int(record: &mut [u8], f: &Field, value: &Value) -> Result<(), String> {
    let v = value.as_integer().ok_or("expected an integer")?;
    let range = |kind| format!("out of range for {kind}");
    let bytes: Vec<u8> = match f.kind {
        Kind::U8 => vec![u8::try_from(v).map_err(|_| range("u8"))?],
        Kind::U16 => u16::try_from(v)
            .map_err(|_| range("u16"))?
            .to_le_bytes()
            .to_vec(),
        Kind::U32 => u32::try_from(v)
            .map_err(|_| range("u32"))?
            .to_le_bytes()
            .to_vec(),
    };
    record[f.offset..f.offset + bytes.len()].copy_from_slice(&bytes);
    Ok(())
}

/// Formats bytes as space-separated lowercase hex.
#[must_use]
pub fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 3);
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 {
            s.push(' ');
        }
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// Parses hex, ignoring whitespace.
#[must_use]
pub fn parse_hex(text: &str) -> Option<Vec<u8>> {
    let digits: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    if !digits.len().is_multiple_of(2) {
        return None;
    }
    (0..digits.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&digits[i..i + 2], 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIELDS: &[Field] = &[Field {
        name: "hp",
        offset: 2,
        kind: Kind::U16,
        doc: "",
    }];

    #[test]
    fn round_trips_and_edits() {
        let layout = Layout {
            size: 6,
            prefix: 1,
            fields: FIELDS,
        };
        assert_eq!(layout.unnamed(), [(1, 1), (4, 2)]);
        let record = [9, 1, 0x34, 0x12, 5, 6];
        let mut t = TomlTable::new();
        layout.export(&record, &mut t);
        assert_eq!(t["hp"].as_integer(), Some(0x1234));
        assert_eq!(t["bytes_04"].as_str(), Some("05 06"));

        let mut edited = record;
        for (k, v) in &t {
            layout.import(&mut edited, k, v).unwrap().unwrap();
        }
        assert_eq!(edited, record);
        layout
            .import(&mut edited, "hp", &Value::Integer(7))
            .unwrap()
            .unwrap();
        assert_eq!(edited[2..4], [7, 0]);
        assert!(
            layout
                .import(&mut edited, "hp", &Value::Integer(70_000))
                .unwrap()
                .is_err()
        );
        assert!(
            layout
                .import(&mut edited, "bytes_04", &Value::String("01".into()))
                .unwrap()
                .is_err()
        );
        assert!(
            layout
                .import(&mut edited, "name", &Value::Integer(1))
                .is_none()
        );
    }
}
