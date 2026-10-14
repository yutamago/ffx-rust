//! The dialogue editor: each event's and battle's message table in the
//! chosen language (`ffx_text::messages`).

use std::path::Path;

use anyhow::{Context, Result};
use ffx_text::messages::MessageTable;
use serde_json::{Value, json};

use crate::game::{Game, write_mod_file};

/// The archive path of an event's (`cdsp0000`) or a battle's
/// (`cdsp00_00`) messages.
fn path(game: &Game, name: &str) -> String {
    if name.contains('_') {
        game.language.battle_messages_path(name)
    } else {
        game.language.event_messages_path(name)
    }
}

/// Every event and battle with messages in the language, with a preview of
/// its first line.
pub fn list(game: &Game) -> Value {
    let prefix = format!("ffx_ps2/ffx/master/new_{}pc/", game.language.code());
    let mut rows = Vec::new();
    for e in game.archive.entries() {
        let Some(rest) = e.path.strip_prefix(&prefix) else {
            continue;
        };
        let (kind, name) = if let Some(r) = rest.strip_prefix("event/obj_ps3/") {
            ("Scene", r.rsplit('/').next().unwrap_or(r))
        } else if let Some(r) = rest.strip_prefix("battle/btl/") {
            ("Battle", r.rsplit('/').next().unwrap_or(r))
        } else {
            continue;
        };
        let Some(name) = name.strip_suffix(".bin") else {
            continue;
        };
        let Some(table) = game
            .archive
            .read(e)
            .ok()
            .and_then(|b| MessageTable::parse(&b).ok())
        else {
            continue;
        };
        let count = table.messages.len();
        let preview = table
            .messages
            .iter()
            .find_map(|m| {
                let plain = strip_codes(&game.charset.decode(m.text.as_ref()?));
                (!plain.trim().is_empty())
                    .then(|| plain.split_whitespace().collect::<Vec<_>>().join(" "))
            })
            .unwrap_or_default();
        if count == 0 {
            continue;
        }
        rows.push(json!({
            "id": name,
            "label": name,
            "group": kind,
            "detail": format!("{count} lines · {}", preview.chars().take(60).collect::<String>()),
        }));
    }
    rows.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    json!({"rows": rows})
}

/// Text without `{…}` control codes, for previews.
fn strip_codes(text: &str) -> String {
    let mut out = String::new();
    let mut depth = 0;
    for c in text.chars() {
        match c {
            '{' => depth += 1,
            '}' if depth > 0 => depth -= 1,
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    out
}

/// One table's messages: index, text (with `{…}` codes), flags.
pub fn get(game: &Game, overlay: Option<&Path>, name: &str) -> Result<Value> {
    let table = MessageTable::parse(&game.read(overlay, &path(game, name))?)?;
    let messages: Vec<Value> = table
        .messages
        .iter()
        .enumerate()
        .map(|(i, m)| {
            json!({
                "index": i,
                "text": m.text.as_ref().map(|t| game.charset.decode(t)),
                "flags": m.flags,
                "choices": m.flags >> 8,
            })
        })
        .collect();
    Ok(json!({"name": name, "path": path(game, name), "messages": messages}))
}

/// Writes changed messages: `[{index, text}]`.
pub fn save(game: &Game, mod_dir: &Path, name: &str, messages: &Value) -> Result<Value> {
    let overlay = Some(mod_dir);
    let archive_path = path(game, name);
    let mut table = MessageTable::parse(&game.read(overlay, &archive_path)?)?;
    let mut list = Vec::new();
    for m in messages.as_array().context("messages must be a list")? {
        let mut t = toml::Table::new();
        t.insert(
            "index".into(),
            toml::Value::Integer(m["index"].as_i64().context("index")?),
        );
        t.insert("text".into(), m["text"].as_str().context("text")?.into());
        list.push(toml::Value::Table(t));
    }
    let mut doc = toml::Table::new();
    doc.insert("message".into(), toml::Value::Array(list));
    table.import(&toml::to_string(&doc)?, &game.charset)?;
    write_mod_file(mod_dir, &archive_path, &table.write()?)?;
    get(game, overlay, name)
}
