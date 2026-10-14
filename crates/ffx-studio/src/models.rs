//! Models and textures: export to glTF, PNG and DDS; import edited glTF and
//! DDS into the active mod.

use std::path::Path;

use anyhow::{Context, Result, bail};
use ffx_phyre::{Model, Texture};
use serde_json::{Value, json};

use crate::game::{Game, safe_relative, write_mod_file};

/// What kind of model a path is, for grouping.
fn group(path: &str) -> &'static str {
    let p = path
        .strip_prefix("ffx_data/gamedata/ps3data/")
        .unwrap_or(path);
    match p.split('/').take(2).collect::<Vec<_>>()[..] {
        ["chr", "pc"] => "Party",
        ["chr", "mon"] => "Monsters",
        ["chr", "npc"] => "People",
        ["chr", "sum"] => "Aeons",
        ["chr", "wep"] => "Weapons",
        ["chr", _] => "Objects",
        ["map", _] => "Maps",
        ["btlmap", _] => "Battle maps",
        _ => "Other",
    }
}

/// Models (and with `textures` in the filter, textures) whose path
/// contains `filter`.
pub fn list(game: &Game, filter: &str) -> Value {
    let filter = filter.to_ascii_lowercase();
    let rows: Vec<Value> = game
        .archive
        .entries()
        .iter()
        .filter(|e| {
            e.size > 0 && (e.path.ends_with(".dae.phyre") || e.path.ends_with(".dds.phyre"))
        })
        .filter(|e| e.path.contains("/d3d11/") && e.path.contains(&filter))
        .map(|e| {
            let file = e.path.rsplit('/').next().unwrap_or(&e.path);
            let texture = e.path.ends_with(".dds.phyre");
            json!({
                "id": e.path,
                "label": file.trim_end_matches(".phyre"),
                "group": if texture { "Textures" } else { group(&e.path) },
                "detail": e.path.trim_start_matches("ffx_data/gamedata/ps3data/"),
                "texture": texture,
            })
        })
        .collect();
    json!({"rows": rows})
}

fn png_of(texture: &Texture) -> Vec<u8> {
    ffx_phyre::png(texture.width, texture.height, &texture.to_rgba())
}

/// Exports a model as `.glb` (base color textures embedded) into the
/// exports folder; returns the file.
pub fn export(game: &Game, overlay: Option<&Path>, exports: &Path, path: &str) -> Result<Value> {
    let model = Model::parse(&game.read(overlay, path)?).with_context(|| path.to_owned())?;
    let y_down = path.contains("/ps3data/chr/");
    let glb = model.to_glb_with(
        y_down,
        |t| {
            let bytes = game.read(overlay, &t.archive_path()).ok()?;
            Texture::parse(bytes).ok().map(|t| png_of(&t))
        },
        &[],
    );
    let dest = exports.join(safe_relative(&path.replace(".dae.phyre", ".glb"))?);
    if let Some(dir) = dest.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(&dest, glb)?;
    Ok(json!({"file": dest, "segments": model.segments.len()}))
}

/// Puts an edited `.glb` back into the model, in the active mod. New
/// textures are added to the character's asset list there too.
pub fn import(game: &Game, mod_dir: &Path, path: &str, file: &str) -> Result<Value> {
    let overlay = Some(mod_dir);
    let original = game.read(overlay, path)?;
    let before = Model::parse(&original)?.segments.len();
    let glb = std::fs::read(file).with_context(|| format!("reading {file}"))?;
    let imported = ffx_phyre::import_into_cluster(&original, &glb)?;
    write_mod_file(mod_dir, path, &imported.bytes)?;
    let mut notes = Vec::new();
    if !imported.new_textures.is_empty() {
        let entries: Vec<String> = imported
            .new_textures
            .iter()
            .map(|t| ffx_phyre::assets::texture_entry(t))
            .collect();
        let list = path.rsplit_once("/mdl/").map(|(dir, f)| {
            let name = f
                .rsplit('/')
                .next()
                .unwrap_or(f)
                .trim_end_matches(".dae.phyre");
            format!("{dir}/{name}.ahwin32")
        });
        match list.filter(|l| game.archive.find(l).is_some()) {
            Some(l) => {
                let updated = ffx_phyre::assets::add_files(&game.read(overlay, &l)?, &entries)?;
                write_mod_file(mod_dir, &l, &updated)?;
                notes.push(format!(
                    "added {} new texture(s) to the asset list",
                    entries.len()
                ));
            }
            None => notes.push(format!(
                "new textures {}: no asset list found to add them to",
                entries.join(", ")
            )),
        }
    }
    let after = Model::parse(&imported.bytes)?.segments.len();
    Ok(json!({
        "segments": imported.segments.len(),
        "new_segments": after.saturating_sub(before),
        "notes": notes,
    }))
}

/// Exports a texture as PNG (for viewing and painting) and DDS (for
/// importing back) into the exports folder.
pub fn export_texture(
    game: &Game,
    overlay: Option<&Path>,
    exports: &Path,
    path: &str,
) -> Result<Value> {
    let texture = Texture::parse(game.read(overlay, path)?).with_context(|| path.to_owned())?;
    let base = exports.join(safe_relative(
        path.trim_end_matches(".phyre").trim_end_matches(".dds"),
    )?);
    if let Some(dir) = base.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let png = base.with_extension("png");
    let dds = base.with_extension("dds");
    std::fs::write(&png, png_of(&texture))?;
    std::fs::write(&dds, texture.to_dds())?;
    Ok(json!({
        "png": png, "dds": dds,
        "width": texture.width, "height": texture.height,
        "format": format!("{:?}", texture.format),
    }))
}

/// Replaces a texture's pixels with a DDS of the same size and format, in
/// the active mod.
pub fn import_texture(game: &Game, mod_dir: &Path, path: &str, file: &str) -> Result<Value> {
    let overlay = Some(mod_dir);
    let texture = Texture::parse(game.read(overlay, path)?)?;
    if !file.to_ascii_lowercase().ends_with(".dds") {
        bail!("choose a .dds file of the same size and format (PNG export is for viewing)");
    }
    let dds = std::fs::read(file).with_context(|| format!("reading {file}"))?;
    let replaced = texture.with_dds(&dds)?;
    write_mod_file(mod_dir, path, replaced.bytes())?;
    Ok(json!({"saved": path}))
}
