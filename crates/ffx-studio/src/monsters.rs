//! The monster editor: names from the kernel monster tables, stats and
//! rewards from each monster's file (`battle/mon/_mNNN/mNNN.bin`).

use std::path::Path;

use anyhow::{Context, Result};
use ffx_kernel::schema::MONSTER_FIELDS;
use ffx_kernel::{Field, Kind, Table};
use ffx_monster::{MonsterFile, SPOILS_FIELDS, archive_path};
use serde_json::{Value, json};

use crate::game::{Game, write_mod_file};
use crate::kernel::{Names, to_toml};

const TABLES: [&str; 3] = ["monster1", "monster2", "monster3"];

/// The kernel monster table holding `id`, with its file name.
fn kernel_table(game: &Game, overlay: Option<&Path>, id: u16) -> Result<(&'static str, Table)> {
    for name in TABLES {
        let schema = crate::kernel::schema(name)?;
        let table = crate::kernel::load(game, overlay, schema)?;
        if table.ids().any(|i| i == id) {
            return Ok((name, table));
        }
    }
    anyhow::bail!("no kernel record for monster {id}")
}

/// Every monster with a file: id, name, HP.
pub fn rows(game: &Game, overlay: Option<&Path>) -> Result<Value> {
    let mut rows = Vec::new();
    for name in TABLES {
        let table = crate::kernel::load(game, overlay, crate::kernel::schema(name)?)?;
        for id in table.ids() {
            if game.archive.find(&archive_path(id)).is_none() {
                continue;
            }
            let label = game.charset.decode(table.string(id, 0).unwrap_or_default());
            let hp = table.record(id).map_or(0, |r| {
                u32::from_le_bytes(r[0x14..0x18].try_into().expect("4 bytes"))
            });
            rows.push(json!({
                "id": id,
                "label": if label.is_empty() { format!("m{id:03}") } else { label },
                "detail": format!("m{id:03} · {hp} HP"),
            }));
        }
    }
    Ok(json!({"rows": rows}))
}

/// Monster names by monster id (`0x1000 + n` as in formations).
pub fn names(game: &Game, overlay: Option<&Path>) -> Result<Value> {
    let mut out = Vec::new();
    for name in TABLES {
        let table = crate::kernel::load(game, overlay, crate::kernel::schema(name)?)?;
        for id in table.ids() {
            if game.archive.find(&archive_path(id)).is_none() {
                continue;
            }
            let label = game.charset.decode(table.string(id, 0).unwrap_or_default());
            out.push(
                json!({"value": 0x1000 + u32::from(id), "label": format!("{label} (m{id:03})")}),
            );
        }
    }
    Ok(Value::Array(out))
}

fn field_json(group: &str, f: &Field) -> Value {
    let max = match f.kind {
        Kind::U8 => 0xff_u64,
        Kind::U16 => 0xffff,
        Kind::U32 => 0xffff_ffff,
    };
    let is_item = group == "spoils"
        && f.kind == Kind::U16
        && !f.name.ends_with("count")
        && ["drop", "steal", "bribe", "overkill_drop"]
            .iter()
            .any(|p| f.name.starts_with(p));
    json!({
        "key": format!("{group}.{}", f.name),
        "label": pretty(f.name),
        "doc": f.doc,
        "editor": if is_item { json!({"type": "ref", "ref": "item0"}) } else { json!({"type": "int", "min": 0, "max": max}) },
        "advanced": f.name.starts_with("stat_"),
        "group": if group == "stats" { "Stats" } else { "Rewards" },
    })
}

fn pretty(key: &str) -> String {
    let mut s = key.replace('_', " ");
    if let Some(c) = s.get(..1) {
        s.replace_range(..1, &c.to_uppercase());
    }
    s.replace("Hp", "HP")
        .replace("Mp", "MP")
        .replace(" ap", " AP")
        .replace("Ap", "AP")
}

fn load_file(game: &Game, overlay: Option<&Path>, id: u16) -> Result<MonsterFile> {
    Ok(MonsterFile::parse(game.read(overlay, &archive_path(id))?)?)
}

/// One monster: name, scan text, stats and rewards.
pub fn record(game: &Game, overlay: Option<&Path>, id: u64) -> Result<Value> {
    let id = u16::try_from(id)?;
    let file = load_file(game, overlay, id)?;
    let doc: toml::Table = file.export()?.parse()?;
    let mut values = serde_json::Map::new();
    for (group, table) in &doc {
        if let Some(t) = table.as_table() {
            for (k, v) in t {
                values.insert(format!("{group}.{k}"), serde_json::to_value(v)?);
            }
        }
    }
    let (_, kernel) = kernel_table(game, overlay, id)?;
    values.insert(
        "name".into(),
        json!(
            game.charset
                .decode(kernel.string(id, 0).unwrap_or_default())
        ),
    );
    values.insert(
        "scan".into(),
        json!(
            game.charset
                .decode(kernel.string(id, 3).unwrap_or_default())
        ),
    );
    let mut fields = vec![
        json!({"key": "name", "label": "Name", "doc": "Shown in battle and the Scan", "editor": {"type": "text"}, "advanced": false, "group": "Text"}),
        json!({"key": "scan", "label": "Scan text", "doc": "What Scan says", "editor": {"type": "textarea"}, "advanced": false, "group": "Text"}),
    ];
    fields.extend(MONSTER_FIELDS.iter().map(|f| field_json("stats", f)));
    fields.extend(SPOILS_FIELDS.iter().map(|f| field_json("spoils", f)));
    let names = Names::load(game, overlay);
    let mut items = vec![json!({"value": 0, "label": "(none)"})];
    items.extend(names.options("item"));
    Ok(json!({
        "id": id,
        "values": values,
        "fields": fields,
        "options": {"item0": items},
    }))
}

/// Writes changed values: stats and rewards into the monster file, name
/// and scan text into the kernel table.
pub fn save(game: &Game, mod_dir: &Path, id: u64, values: &Value) -> Result<Value> {
    let id16 = u16::try_from(id)?;
    let overlay = Some(mod_dir);
    let values = values.as_object().context("values must be an object")?;
    let mut file = load_file(game, overlay, id16)?;
    let mut doc = toml::Table::new();
    let mut texts = Vec::new();
    for (k, v) in values {
        if let Some((group, key)) = k.split_once('.') {
            let t = doc
                .entry(group.to_owned())
                .or_insert_with(|| toml::Value::Table(toml::Table::new()));
            if let Some(t) = t.as_table_mut() {
                t.insert(
                    key.to_owned(),
                    to_toml(v).with_context(|| format!("{k}: unsupported value"))?,
                );
            }
        } else if k == "name" || k == "scan" {
            texts.push((
                if k == "name" { 0 } else { 3 },
                v.as_str().unwrap_or("").to_owned(),
            ));
        }
    }
    let mut saved = Vec::new();
    if !doc.is_empty() {
        file.import(&toml::to_string(&doc)?)?;
        let path = archive_path(id16);
        write_mod_file(mod_dir, &path, file.bytes())?;
        saved.push(path);
    }
    if !texts.is_empty() {
        let (name, mut table) = kernel_table(game, overlay, id16)?;
        for (field, text) in texts {
            let bytes = game
                .charset
                .encode(&text)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            if table.string(id16, field) != Some(&bytes[..]) {
                table.set_string(id16, field, &bytes)?;
            }
        }
        let path = crate::kernel::schema(name)?.path(game.language);
        write_mod_file(mod_dir, &path, &table.to_bytes()?)?;
        saved.push(path);
    }
    Ok(json!({"saved": saved, "record": record(game, overlay, id)?}))
}
