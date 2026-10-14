//! Mod folders: `<game>/mods/<id>/<archive paths>` plus a `mod.toml` with
//! the name, description and whether the mod is on (the hook skips mods
//! with `enabled = false`).

use std::path::Path;

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

fn read_meta(dir: &Path) -> toml::Table {
    std::fs::read_to_string(dir.join("mod.toml"))
        .ok()
        .and_then(|t| t.parse().ok())
        .unwrap_or_default()
}

fn write_meta(dir: &Path, meta: &toml::Table) -> Result<()> {
    std::fs::write(dir.join("mod.toml"), toml::to_string(meta)?)?;
    Ok(())
}

/// The mods, in the order the game applies them.
#[must_use]
pub fn list(root: &Path) -> Vec<Value> {
    let mut dirs: Vec<_> = std::fs::read_dir(root)
        .map(|r| {
            r.flatten()
                .map(|e| e.path())
                .filter(|p| p.is_dir())
                .collect()
        })
        .unwrap_or_default();
    dirs.sort();
    dirs.iter()
        .map(|d| {
            let meta = read_meta(d);
            let id = d
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            json!({
                "id": id,
                "name": meta.get("name").and_then(|v| v.as_str()).unwrap_or(&id),
                "description": meta.get("description").and_then(|v| v.as_str()).unwrap_or(""),
                "enabled": meta.get("enabled").and_then(toml::Value::as_bool).unwrap_or(true),
                "files": files(d).len(),
            })
        })
        .collect()
}

/// Makes a mod folder for `name`; returns its id (the folder name).
///
/// # Errors
/// Fails if the folder cannot be made.
pub fn create(root: &Path, name: &str, description: &str) -> Result<String> {
    let name = name.trim();
    if name.is_empty() {
        bail!("give the mod a name");
    }
    let base: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let base = if base.is_empty() {
        "mod".to_owned()
    } else {
        base
    };
    let mut id = base.clone();
    let mut n = 2;
    while root.join(&id).exists() {
        id = format!("{base}-{n}");
        n += 1;
    }
    let dir = root.join(&id);
    std::fs::create_dir_all(&dir)?;
    let mut meta = toml::Table::new();
    meta.insert("name".into(), name.into());
    meta.insert("description".into(), description.into());
    meta.insert("enabled".into(), true.into());
    write_meta(&dir, &meta)?;
    Ok(id)
}

/// Changes a mod's `name`, `description` or `enabled` (whichever `args` has).
///
/// # Errors
/// Fails on a missing mod.
pub fn update(root: &Path, id: &str, args: &Value) -> Result<()> {
    let dir = root.join(crate::game::safe_relative(id)?);
    if !dir.is_dir() {
        bail!("there is no mod {id}");
    }
    let mut meta = read_meta(&dir);
    for key in ["name", "description"] {
        if let Some(v) = args.get(key).and_then(Value::as_str) {
            meta.insert(key.into(), v.into());
        }
    }
    if let Some(v) = args.get("enabled").and_then(Value::as_bool) {
        meta.insert("enabled".into(), v.into());
    }
    write_meta(&dir, &meta)
}

/// The files a mod replaces or adds, as archive paths.
#[must_use]
pub fn files(dir: &Path) -> Vec<String> {
    fn walk(base: &Path, dir: &Path, out: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(base, &p, out);
            } else if let Ok(rel) = p.strip_prefix(base) {
                let rel = rel.to_string_lossy().replace('\\', "/");
                if rel != "mod.toml" {
                    out.push(rel);
                }
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort();
    out
}

/// Takes a file out of a mod (the game's own copy is used again).
///
/// # Errors
/// Fails on paths outside the mod.
pub fn remove_file(dir: &Path, path: &str) -> Result<()> {
    let file = dir.join(crate::game::safe_relative(path)?);
    std::fs::remove_file(&file).with_context(|| format!("removing {path}"))?;
    Ok(())
}
