//! The encounter editor: battle fields, their groups (rate, battle map,
//! formation weights) and each formation's monsters.

use std::path::Path;

use anyhow::{Context, Result, bail};
use ffx_battle::{ENCOUNTER_TABLE, EncounterTable, FormationFile, formation_path};
use serde_json::{Value, json};

use crate::game::{Game, write_mod_file};

fn table(game: &Game, overlay: Option<&Path>) -> Result<EncounterTable> {
    Ok(EncounterTable::parse(
        &game.read(overlay, ENCOUNTER_TABLE)?,
    )?)
}

fn monster_names(game: &Game, overlay: Option<&Path>) -> Vec<(u64, String)> {
    crate::monsters::names(game, overlay)
        .ok()
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|v| Some((v["value"].as_u64()?, v["label"].as_str()?.to_owned())))
        .collect()
}

/// The battle fields: name, map number, groups, formations.
pub fn fields(game: &Game, overlay: Option<&Path>) -> Result<Value> {
    let t = table(game, overlay)?;
    let rows: Vec<Value> = t
        .fields
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let groups = &t.blocks[f.block];
            let random = groups.iter().any(|g| g.rate > 0 && g.formations.iter().any(|x| x.weight > 0));
            let shared = t.fields[..i].iter().find(|o| o.block == f.block).map(ffx_battle::Field::name);
            json!({
                "id": i,
                "label": f.name(),
                "detail": format!(
                    "map {} · {} formation{}{}{}",
                    f.map,
                    groups.iter().map(|g| g.formations.len()).sum::<usize>(),
                    if groups.iter().map(|g| g.formations.len()).sum::<usize>() == 1 { "" } else { "s" },
                    if random { " · random encounters" } else { "" },
                    shared.map(|s| format!(" · same as {s}")).unwrap_or_default(),
                ),
                "random": random,
            })
        })
        .collect();
    Ok(json!({"rows": rows}))
}

/// One field: its groups with their formations and monsters.
pub fn field(game: &Game, overlay: Option<&Path>, index: u64) -> Result<Value> {
    let t = table(game, overlay)?;
    let i = usize::try_from(index)?;
    let f = t
        .fields
        .get(i)
        .with_context(|| format!("no battle field {i}"))?;
    let names = monster_names(game, overlay);
    let name_of = |id: u16| {
        names
            .iter()
            .find(|(v, _)| *v == u64::from(id))
            .map_or_else(|| format!("{id:#06x}"), |(_, n)| n.clone())
    };
    let groups: Vec<Value> = t.blocks[f.block]
        .iter()
        .enumerate()
        .map(|(g, group)| {
            let total: u32 = group.formations.iter().map(|x| u32::from(x.weight)).sum();
            let formations: Vec<Value> = group
                .formations
                .iter()
                .enumerate()
                .map(|(k, x)| {
                    let file = t.formation_file(i, g, k).unwrap_or_default();
                    let parsed = game
                        .read(overlay, &formation_path(&file))
                        .ok()
                        .and_then(|b| FormationFile::parse(b).ok());
                    let monsters: Vec<Value> = parsed
                        .as_ref()
                        .and_then(FormationFile::monsters)
                        .map(|m| {
                            m.iter()
                                .map(|s| s.map_or(Value::Null, |id| json!({"id": id, "name": name_of(id)})))
                                .collect()
                        })
                        .unwrap_or_default();
                    let flags = parsed.as_ref().and_then(FormationFile::formation).map(|b| {
                        json!({"footsteps": b[2], "underwater": b[3] == 1})
                    });
                    json!({
                        "position": k,
                        "id": x.id,
                        "file": file,
                        "weight": x.weight,
                        "chance": if total > 0 { f64::from(x.weight) * 100.0 / f64::from(total) } else { 0.0 },
                        "monsters": monsters,
                        "flags": flags,
                    })
                })
                .collect();
            json!({"index": g, "rate": group.rate, "battle_map": group.battle_map, "formations": formations})
        })
        .collect();
    let shared: Vec<String> = t
        .fields
        .iter()
        .enumerate()
        .filter(|(k, o)| *k != i && o.block == f.block)
        .map(|(_, o)| o.name())
        .collect();
    Ok(json!({
        "id": i,
        "name": f.name(),
        "map": f.map,
        "shared_with": shared,
        "groups": groups,
        "monsters": names.iter().map(|(v, n)| json!({"value": v, "label": n})).collect::<Vec<_>>(),
    }))
}

/// Changes a group: `{field, group, rate, battle_map, weights: [..]}`.
pub fn save_group(game: &Game, mod_dir: &Path, args: &Value) -> Result<Value> {
    let overlay = Some(mod_dir);
    let mut t = table(game, overlay)?;
    let i = usize::try_from(args["field"].as_u64().context("field")?)?;
    let g = usize::try_from(args["group"].as_u64().context("group")?)?;
    let block = t.fields.get(i).context("no such field")?.block;
    let group = t.blocks[block].get_mut(g).context("no such group")?;
    if let Some(rate) = args.get("rate").and_then(Value::as_u64) {
        group.rate = u8::try_from(rate).context("rate is 0-255")?;
    }
    if let Some(map) = args.get("battle_map").and_then(Value::as_u64) {
        group.battle_map = u16::try_from(map).context("battle_map")?;
    }
    if let Some(weights) = args.get("weights").and_then(Value::as_array) {
        if weights.len() != group.formations.len() {
            bail!("one weight per formation");
        }
        for (f, w) in group.formations.iter_mut().zip(weights) {
            let w = w.as_u64().context("weights are numbers")?;
            if w > 15 {
                bail!("weights are 0-15");
            }
            f.weight = u8::try_from(w)?;
        }
    }
    write_mod_file(mod_dir, ENCOUNTER_TABLE, &t.write())?;
    field(game, overlay, i as u64)
}

/// Changes a formation: `{file, monsters: [id or null; ≤ 8], underwater,
/// footsteps}`, and returns its field again when `field` is given.
pub fn save_formation(game: &Game, mod_dir: &Path, args: &Value) -> Result<Value> {
    let overlay = Some(mod_dir);
    let name = args["file"].as_str().context("file")?;
    let path = formation_path(name);
    let mut file = FormationFile::parse(game.read(overlay, &path)?)?;
    let mut t = toml::Table::new();
    if let Some(m) = args.get("monsters").and_then(Value::as_array) {
        let list: Vec<toml::Value> = m
            .iter()
            .map(|v| {
                v.as_u64().map_or_else(
                    || "-".into(),
                    |id| toml::Value::Integer(i64::try_from(id).unwrap_or(0)),
                )
            })
            .collect();
        t.insert("monsters".into(), toml::Value::Array(list));
    }
    if let Some(u) = args.get("underwater").and_then(Value::as_bool) {
        t.insert("underwater".into(), u.into());
    }
    if let Some(f) = args.get("footsteps").and_then(Value::as_u64) {
        t.insert("footsteps".into(), toml::Value::Integer(i64::try_from(f)?));
    }
    let mut doc = toml::Table::new();
    doc.insert("formation".into(), toml::Value::Table(t));
    file.import(&toml::to_string(&doc)?)?;
    write_mod_file(mod_dir, &path, file.bytes())?;
    match args.get("field").and_then(Value::as_u64) {
        Some(i) => field(game, overlay, i),
        None => Ok(json!({"saved": path})),
    }
}
