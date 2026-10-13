//! TOML export and import.
//!
//! ```toml
//! [[record]]
//! id = 0
//! name = "Potion"
//! description = "Restores 200 HP to one character."
//! formula = 6
//! power = 4
//! bytes_10 = "1c 00 00 00 ..."   # bytes no field covers, keyed by offset
//! ```
//!
//! Import applies a file onto the original table: records and keys that are
//! absent keep their original values, so a mod can list only what it
//! changes. A string that decodes to the same text keeps its original bytes.

use std::fmt::Write;

use ffx_text::Charset;
use toml::{Table as TomlTable, Value};

use crate::schema::{Kind, Schema};
use crate::{Error, Result, Table};

/// Unnamed byte runs of a schema: (offset, length).
fn unnamed(schema: &Schema) -> Vec<(usize, usize)> {
    let mut used = vec![false; usize::from(schema.record_size)];
    used[..schema.strings_len()].fill(true);
    for f in schema.fields {
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

fn read_int(record: &[u8], offset: usize, kind: Kind) -> i64 {
    let b = &record[offset..offset + kind.len()];
    match kind {
        Kind::U8 => i64::from(b[0]),
        Kind::U16 => i64::from(u16::from_le_bytes([b[0], b[1]])),
        Kind::U32 => i64::from(u32::from_le_bytes([b[0], b[1], b[2], b[3]])),
    }
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 3);
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 {
            s.push(' ');
        }
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// Exports every record as TOML.
///
/// # Errors
/// Fails if the table's records do not have the schema's size.
pub fn export(table: &Table, schema: &Schema, charset: &Charset) -> Result<String> {
    let runs = unnamed(schema);
    let mut records = Vec::new();
    for id in table.ids() {
        let record = table.record(id).expect("listed ID");
        if record.len() != usize::from(schema.record_size) {
            return Err(Error::WrongSchema(schema.file));
        }
        let mut t = TomlTable::new();
        t.insert("id".into(), Value::Integer(i64::from(id)));
        for (i, name) in schema.strings.iter().enumerate() {
            let text = charset.decode(table.string(id, i).unwrap_or_default());
            t.insert((*name).into(), Value::String(text));
        }
        for f in schema.fields {
            t.insert(
                f.name.into(),
                Value::Integer(read_int(record, f.offset, f.kind)),
            );
        }
        for &(offset, len) in &runs {
            t.insert(
                format!("bytes_{offset:02x}"),
                Value::String(hex(&record[offset..offset + len])),
            );
        }
        records.push(Value::Table(t));
    }
    let mut doc = TomlTable::new();
    doc.insert("record".into(), Value::Array(records));
    let mut out = format!("# battle/kernel/{}.bin\n", schema.file);
    for f in schema.fields {
        let _ = writeln!(out, "# {}: {}", f.name, f.doc);
    }
    out.push('\n');
    out.push_str(&toml::to_string(&doc).map_err(|e| Error::Toml(e.to_string()))?);
    Ok(out)
}

/// Applies a TOML file onto `table`.
///
/// # Errors
/// Fails on unknown IDs or keys, values out of range, malformed hex, and
/// text the charset cannot encode.
pub fn import(
    table: &mut Table,
    schema: &Schema,
    charset: &Charset,
    toml_text: &str,
) -> Result<()> {
    let doc: TomlTable = toml_text
        .parse()
        .map_err(|e: toml::de::Error| Error::Toml(e.to_string()))?;
    let Some(records) = doc.get("record") else {
        return Ok(());
    };
    let records = records
        .as_array()
        .ok_or(Error::Toml("`record` must be an array of tables".into()))?;
    let runs = unnamed(schema);
    for entry in records {
        let entry = entry
            .as_table()
            .ok_or(Error::Toml("each record must be a table".into()))?;
        let id = entry
            .get("id")
            .and_then(Value::as_integer)
            .and_then(|i| u16::try_from(i).ok())
            .ok_or(Error::Toml("record without a valid `id`".into()))?;
        if table.record(id).is_none() {
            return Err(Error::NoRecord(id));
        }
        for (key, value) in entry {
            if key == "id" {
                continue;
            }
            let at = |what: &str| Error::Value {
                id,
                key: key.clone(),
                problem: what.to_string(),
            };
            if let Some(i) = schema.strings.iter().position(|s| s == key) {
                let text = value.as_str().ok_or_else(|| at("expected a string"))?;
                if charset.decode(table.string(id, i).unwrap_or_default()) != text {
                    let bytes = charset.encode(text).map_err(|e| at(&e.to_string()))?;
                    table.set_string(id, i, &bytes)?;
                }
            } else if let Some(f) = schema.fields.iter().find(|f| f.name == key) {
                let v = value
                    .as_integer()
                    .ok_or_else(|| at("expected an integer"))?;
                let bytes: Vec<u8> = match f.kind {
                    Kind::U8 => vec![u8::try_from(v).map_err(|_| at("out of range for u8"))?],
                    Kind::U16 => u16::try_from(v)
                        .map_err(|_| at("out of range for u16"))?
                        .to_le_bytes()
                        .to_vec(),
                    Kind::U32 => u32::try_from(v)
                        .map_err(|_| at("out of range for u32"))?
                        .to_le_bytes()
                        .to_vec(),
                };
                table.record_mut(id).expect("checked")[f.offset..f.offset + bytes.len()]
                    .copy_from_slice(&bytes);
            } else if let Some(&(offset, len)) = key
                .strip_prefix("bytes_")
                .and_then(|o| usize::from_str_radix(o, 16).ok())
                .and_then(|o| runs.iter().find(|r| r.0 == o))
            {
                let text = value.as_str().ok_or_else(|| at("expected a hex string"))?;
                let digits: String = text.chars().filter(|c| !c.is_whitespace()).collect();
                let bytes: Vec<u8> = (0..digits.len())
                    .step_by(2)
                    .map(|i| {
                        digits
                            .get(i..i + 2)
                            .and_then(|d| u8::from_str_radix(d, 16).ok())
                    })
                    .collect::<Option<_>>()
                    .ok_or_else(|| at("malformed hex"))?;
                if bytes.len() != len {
                    return Err(at(&format!("expected {len} bytes, got {}", bytes.len())));
                }
                table.record_mut(id).expect("checked")[offset..offset + len]
                    .copy_from_slice(&bytes);
            } else {
                return Err(at("unknown key"));
            }
        }
    }
    Ok(())
}
