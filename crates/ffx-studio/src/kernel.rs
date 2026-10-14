//! The kernel table editor: items, abilities, key items, equipment,
//! treasure, characters. Fields come from `ffx_kernel::Schema`; values go
//! through the kernel's own TOML export and import.

use std::path::Path;

use anyhow::{Context, Result};
use ffx_kernel::{Kind, Schema, Table};
use serde_json::{Value, json};

use crate::game::{Game, write_mod_file};

/// Fields only advanced mode shows.
const ADVANCED: [&str; 6] = [
    "flags",
    "crit_flags",
    "in_use",
    "name",
    "model",
    "equipped_by",
];

/// Human labels for field keys.
fn label(key: &str) -> String {
    let mut s = key.replace('_', " ");
    if let Some(c) = s.get(..1) {
        s.replace_range(..1, &c.to_uppercase());
    }
    s
}

pub fn schema(table: &str) -> Result<&'static Schema> {
    Schema::by_file(table).with_context(|| format!("unknown table {table}"))
}

pub fn load(game: &Game, overlay: Option<&Path>, schema: &Schema) -> Result<Table> {
    Ok(Table::parse(
        &game.read(overlay, &schema.path(game.language))?,
    )?)
}

/// The TOML export of a table, as JSON records by id.
fn exported(game: &Game, table: &Table, schema: &Schema) -> Result<Vec<Value>> {
    let text = ffx_kernel::export(table, schema, &game.charset)?;
    let doc: toml::Table = text.parse()?;
    let records = doc
        .get("record")
        .and_then(toml::Value::as_array)
        .cloned()
        .unwrap_or_default();
    Ok(records
        .into_iter()
        .map(|r| serde_json::to_value(r).unwrap_or(Value::Null))
        .collect())
}

/// Names for references between tables: items, auto-abilities, key items,
/// characters.
pub struct Names {
    items: Vec<(u16, String)>,
    abilities: Vec<(u16, String)>,
    key_items: Vec<(u16, String)>,
    characters: Vec<(u16, String)>,
}

impl Names {
    pub fn load(game: &Game, overlay: Option<&Path>) -> Self {
        let names = |file: &str| -> Vec<(u16, String)> {
            let Ok(schema) = schema(file) else {
                return Vec::new();
            };
            let Ok(table) = load(game, overlay, schema) else {
                return Vec::new();
            };
            table
                .ids()
                .map(|id| {
                    (
                        id,
                        game.charset.decode(table.string(id, 0).unwrap_or_default()),
                    )
                })
                .collect()
        };
        Self {
            items: names("item"),
            abilities: names("a_ability"),
            key_items: names("important"),
            characters: names("ply_save"),
        }
    }

    fn find(list: &[(u16, String)], id: u16) -> String {
        list.iter().find(|(i, _)| *i == id).map_or_else(
            || format!("#{id}"),
            |(_, n)| {
                if n.is_empty() {
                    format!("#{id}")
                } else {
                    n.clone()
                }
            },
        )
    }

    /// The name of a reference value: `0x2000 + item`, `0x8000 + ability`,
    /// `0xa000 + key item`.
    pub fn reference(&self, v: u64) -> String {
        let id = u16::try_from(v & 0xfff).unwrap_or(0);
        match v >> 12 {
            2 => Self::find(&self.items, id),
            8 => Self::find(&self.abilities, id),
            0xa => Self::find(&self.key_items, id),
            _ => format!("{v:#x}"),
        }
    }

    pub fn character(&self, c: u64) -> String {
        Self::find(&self.characters, u16::try_from(c).unwrap_or(0))
    }

    /// Choices for a reference kind, as (value, label).
    pub fn options(&self, kind: &str) -> Vec<Value> {
        let list = |l: &[(u16, String)], base: u64| -> Vec<Value> {
            l.iter()
                .filter(|(_, n)| !n.is_empty())
                .map(|(i, n)| json!({"value": base + u64::from(*i), "label": n}))
                .collect()
        };
        match kind {
            "item" => list(&self.items, 0x2000),
            "ability" => {
                let mut v = vec![json!({"value": 0xff, "label": "(empty)"})];
                v.extend(list(&self.abilities, 0x8000));
                v
            }
            "ability0" => {
                let mut v = vec![json!({"value": 0, "label": "(none)"})];
                v.extend(list(&self.abilities, 0x8000));
                v
            }
            "key_item" => list(&self.key_items, 0xa000),
            "character" => list(&self.characters, 0),
            _ => Vec::new(),
        }
    }
}

/// How the UI should edit a field: a reference list, a choice, a number.
fn editor(table: &str, key: &str, kind: Kind) -> Value {
    let max = match kind {
        Kind::U8 => 0xff_u64,
        Kind::U16 => 0xffff,
        Kind::U32 => 0xffff_ffff,
    };
    let reference = match (table, key) {
        (_, k) if k.starts_with("ability_") => Some(if table == "buki_get" {
            "ability0"
        } else {
            "ability"
        }),
        (_, "character") => Some("character"),
        _ => None,
    };
    if let Some(r) = reference {
        return json!({"type": "ref", "ref": r});
    }
    match (table, key) {
        (_, "armor") => json!({"type": "choice", "choices": [[0, "Weapon"], [1, "Armor"]]}),
        ("takara", "kind") => {
            json!({"type": "choice", "choices": [[0, "Gil"], [2, "Item"], [5, "Equipment"], [10, "Key item"]]})
        }
        ("takara", "item") => json!({"type": "treasure", "max": max}),
        _ => json!({"type": "int", "min": 0, "max": max}),
    }
}

/// A short description of a record without a name of its own.
fn summary(table: &str, rec: &Value, names: &Names) -> String {
    let get = |k: &str| rec.get(k).and_then(Value::as_u64).unwrap_or(0);
    match table {
        "takara" => match get("kind") {
            0 => format!("{} gil", get("amount") * 100),
            2 => format!("{} × {}", names.reference(get("item")), get("amount")),
            5 => format!("Equipment #{}", get("item")),
            10 => names.reference(get("item")),
            k => format!("kind {k}"),
        },
        "weapon" | "shop_arms" | "buki_get" => {
            if table != "buki_get" && get("in_use") == 0 {
                return "(empty slot)".into();
            }
            let abilities: Vec<String> = (1..=4)
                .map(|i| get(&format!("ability_{i}")))
                .filter(|&a| a != 0 && a != 0xff)
                .map(|a| names.reference(a))
                .collect();
            format!(
                "{} {}{}",
                names.character(get("character")),
                if get("armor") == 1 { "armor" } else { "weapon" },
                if abilities.is_empty() {
                    String::new()
                } else {
                    format!(": {}", abilities.join(", "))
                }
            )
        }
        _ => String::new(),
    }
}

/// The rows of a table: id, label and a summary.
pub fn rows(game: &Game, overlay: Option<&Path>, table: &str) -> Result<Value> {
    let schema = schema(table)?;
    let parsed = load(game, overlay, schema)?;
    let names = Names::load(game, overlay);
    let records = exported(game, &parsed, schema)?;
    let rows: Vec<Value> = records
        .iter()
        .map(|r| {
            let id = r.get("id").and_then(Value::as_u64).unwrap_or(0);
            let name = schema
                .strings
                .first()
                .and_then(|s| r.get(*s))
                .and_then(Value::as_str)
                .unwrap_or("")
                .replace('\n', " ");
            let summary = summary(table, r, &names);
            json!({
                "id": id,
                "label": if name.is_empty() { if summary.is_empty() { format!("#{id}") } else { summary.clone() } } else { name },
                "detail": if schema.strings.is_empty() { String::new() } else { summary },
            })
        })
        .collect();
    Ok(json!({"rows": rows}))
}

/// One record: its fields, with how to edit them, and its values.
pub fn record(game: &Game, overlay: Option<&Path>, table: &str, id: u64) -> Result<Value> {
    let schema = schema(table)?;
    let parsed = load(game, overlay, schema)?;
    let records = exported(game, &parsed, schema)?;
    let rec = records
        .into_iter()
        .find(|r| r.get("id").and_then(Value::as_u64) == Some(id))
        .with_context(|| format!("{table} has no record {id}"))?;
    let names = Names::load(game, overlay);
    let mut fields = Vec::new();
    for (i, s) in schema.strings.iter().enumerate() {
        let main =
            i == 0 || s.starts_with("desc") || s.starts_with("name") || s.starts_with("scan");
        fields.push(json!({
            "key": s, "label": label(s), "doc": "Text (control codes like {09:30} are kept)",
            "editor": {"type": if s.contains("desc") || s.contains("scan") { "textarea" } else { "text" }},
            "advanced": !main,
        }));
    }
    for f in schema.fields {
        fields.push(json!({
            "key": f.name, "label": label(f.name), "doc": f.doc,
            "editor": editor(table, f.name, f.kind),
            "advanced": ADVANCED.contains(&f.name),
        }));
    }
    // Bytes no field names: advanced, as hex.
    if let Some(obj) = rec.as_object() {
        for k in obj.keys().filter(|k| k.starts_with("bytes_")) {
            fields.push(json!({
                "key": k, "label": format!("Bytes at {}", &k[6..]), "doc": "Not identified yet; hexadecimal bytes",
                "editor": {"type": "hex"}, "advanced": true,
            }));
        }
    }
    let mut options = serde_json::Map::new();
    for kind in ["item", "ability", "ability0", "key_item", "character"] {
        options.insert(kind.into(), Value::Array(names.options(kind)));
    }
    Ok(json!({"id": id, "values": rec, "fields": fields, "options": options}))
}

/// JSON values to TOML for an import.
pub fn to_toml(v: &Value) -> Option<toml::Value> {
    Some(match v {
        Value::String(s) => toml::Value::String(s.clone()),
        Value::Bool(b) => toml::Value::Boolean(*b),
        Value::Number(n) => toml::Value::Integer(n.as_i64()?),
        _ => return None,
    })
}

/// Applies changed values to a record and writes the table into the mod.
pub fn save(game: &Game, mod_dir: &Path, table: &str, id: u64, values: &Value) -> Result<Value> {
    let schema = schema(table)?;
    let overlay = Some(mod_dir);
    let mut parsed = load(game, overlay, schema)?;
    let mut rec = toml::Table::new();
    rec.insert("id".into(), toml::Value::Integer(i64::try_from(id)?));
    for (k, v) in values.as_object().context("values must be an object")? {
        if k == "id" {
            continue;
        }
        rec.insert(
            k.clone(),
            to_toml(v).with_context(|| format!("{k}: unsupported value"))?,
        );
    }
    let mut doc = toml::Table::new();
    doc.insert(
        "record".into(),
        toml::Value::Array(vec![toml::Value::Table(rec)]),
    );
    ffx_kernel::import(&mut parsed, schema, &game.charset, &toml::to_string(&doc)?)?;
    let path = schema.path(game.language);
    write_mod_file(mod_dir, &path, &parsed.to_bytes()?)?;
    record(game, overlay, table, id).map(|r| json!({"saved": path, "record": r}))
}
