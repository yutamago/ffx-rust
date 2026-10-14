//! The archive browser: list, extract, add files to a mod.

use std::path::Path;

use anyhow::{Context, Result};
use serde_json::{Value, json};

use crate::game::{Game, safe_relative, write_mod_file};

/// The folders and files directly under `prefix` (a path ending in `/`, or
/// empty for the top).
pub fn list(game: &Game, prefix: &str) -> Value {
    let mut dirs = std::collections::BTreeMap::<String, usize>::new();
    let mut files = Vec::new();
    for e in game.archive.entries() {
        let Some(rest) = e.path.strip_prefix(prefix) else {
            continue;
        };
        match rest.split_once('/') {
            Some((dir, _)) => *dirs.entry(dir.to_owned()).or_default() += 1,
            None => files.push(json!({"path": e.path, "name": rest, "size": e.size})),
        }
        if files.len() > 5000 {
            break;
        }
    }
    let dirs: Vec<Value> = dirs
        .into_iter()
        .map(|(d, n)| json!({"path": format!("{prefix}{d}/"), "name": d, "files": n}))
        .collect();
    json!({"prefix": prefix, "dirs": dirs, "files": files})
}

/// Writes a file (the active mod's copy if any) into the exports folder.
pub fn extract(game: &Game, overlay: Option<&Path>, exports: &Path, path: &str) -> Result<Value> {
    let bytes = game.read(overlay, path)?;
    let dest = exports.join(safe_relative(path)?);
    if let Some(dir) = dest.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(&dest, bytes)?;
    Ok(json!({"file": dest}))
}

/// Copies a file from disk into the active mod as archive path `path`.
pub fn add(mod_dir: &Path, path: &str, file: &str) -> Result<Value> {
    let bytes = std::fs::read(file).with_context(|| format!("reading {file}"))?;
    let dest = write_mod_file(mod_dir, path, &bytes)?;
    Ok(json!({"file": dest}))
}

/// Decodes base64 (standard alphabet, padding optional, whitespace ignored).
///
/// # Errors
/// Fails on characters outside the alphabet.
pub fn base64(text: &str) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len() * 3 / 4);
    let (mut acc, mut bits) = (0u32, 0);
    for c in text.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            b'=' | b'\n' | b'\r' | b' ' => continue,
            _ => anyhow::bail!("not base64"),
        };
        acc = acc << 6 | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits).to_le_bytes()[0]);
        }
    }
    Ok(out)
}
