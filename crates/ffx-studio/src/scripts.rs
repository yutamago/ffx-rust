//! The script editor: event and monster AI scripts as assembly
//! (`ffx_script`), annotated with native function names.

use std::path::Path;

use anyhow::Result;
use ffx_script::{Container, Natives, disassemble_annotated};
use serde_json::{Value, json};

use crate::game::{Game, write_mod_file};

const SYMBOLS: &str = include_str!("../../../symbols/ffx.sym");

/// Script files whose path contains `filter`.
pub fn list(game: &Game, filter: &str) -> Value {
    let rows: Vec<Value> = game
        .archive
        .entries()
        .iter()
        .filter(|e| {
            let p = &e.path;
            p.ends_with(".ebp")
                || ((p.contains("/jppc/battle/mon/_m") || p.contains("/jppc/battle/btl/")) && p.ends_with(".bin"))
        })
        .filter(|e| e.path.contains(filter))
        .map(|e| {
            let name = e.path.rsplit('/').next().unwrap_or(&e.path);
            let group = if e.path.contains("/battle/mon/") {
                "Monster AI"
            } else if e.path.contains("/battle/btl/") {
                "Battle events"
            } else {
                "Scenes"
            };
            json!({"id": e.path, "label": name, "group": group, "detail": e.path.trim_start_matches("ffx_ps2/ffx/master/jppc/")})
        })
        .collect();
    json!({"rows": rows})
}

fn names() -> std::collections::HashMap<u32, &'static str> {
    SYMBOLS
        .lines()
        .filter_map(|l| {
            let mut w = l.split('#').next()?.split_whitespace();
            let addr = u32::from_str_radix(w.next()?.trim_start_matches("0x"), 16).ok()?;
            w.next()?;
            Some((addr, w.next()?))
        })
        .collect()
}

/// A script as assembly.
pub fn get(game: &Game, overlay: Option<&Path>, path: &str) -> Result<Value> {
    let container = Container::parse(game.read(overlay, path)?)?;
    let natives = std::fs::read(game.dir.join("FFX.exe"))
        .ok()
        .and_then(Natives::from_exe);
    let names = names();
    let annotate = |id: u16| {
        let handler = natives.as_ref()?.handler(id)?;
        Some(
            names
                .get(&handler)
                .map_or_else(|| format!("0x{handler:08x}"), |n| (*n).to_owned()),
        )
    };
    Ok(json!({"path": path, "source": disassemble_annotated(&container.script, &annotate)?}))
}

/// Assembles edited source onto the original script, into the active mod.
pub fn save(game: &Game, mod_dir: &Path, path: &str, source: &str) -> Result<Value> {
    let original = Container::parse(game.original(path)?)?;
    let script = ffx_script::assemble(&original.script, source)?;
    let rebuilt = original.replace_script(&script)?;
    write_mod_file(mod_dir, path, rebuilt.bytes())?;
    Ok(json!({"saved": path, "size": rebuilt.bytes().len()}))
}
