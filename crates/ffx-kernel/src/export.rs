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

use crate::record::Layout;
use crate::schema::Schema;
use crate::{Error, Result, Table};

fn layout(schema: &Schema) -> Layout<'_> {
    Layout {
        size: usize::from(schema.record_size),
        prefix: schema.strings_len(),
        fields: schema.fields,
    }
}

/// Exports every record as TOML.
///
/// # Errors
/// Fails if the table's records do not have the schema's size.
pub fn export(table: &Table, schema: &Schema, charset: &Charset) -> Result<String> {
    let layout = layout(schema);
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
        layout.export(record, &mut t);
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
    let layout = layout(schema);
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
            } else {
                let record = table.record_mut(id).expect("checked");
                match layout.import(record, key, value) {
                    Some(result) => result.map_err(|e| at(&e))?,
                    None => return Err(at("unknown key")),
                }
            }
        }
    }
    Ok(())
}
