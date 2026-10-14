//! Checks every shipped message table (event dialogue and battle texts, all
//! languages). Needs `FFX_GAME_DIR`; skipped otherwise.

use std::path::PathBuf;

use ffx_text::messages::MessageTable;
use ffx_text::{Charset, Language};
use ffx_vbf::Archive;

#[test]
fn every_message_table_round_trips() {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return;
    };
    let archive = Archive::open(PathBuf::from(dir).join("data/FFX_Data.vbf")).unwrap();
    let read = |path: &str| archive.read(archive.find(path).unwrap()).unwrap();
    let mut tables = 0;
    let mut messages = 0;
    for language in Language::ALL {
        let charset = Charset::from_table(&read(&language.charset_path()), language).unwrap();
        let prefix = format!("ffx_ps2/ffx/master/new_{}pc/", language.code());
        for entry in archive.entries() {
            let Some(rest) = entry.path.strip_prefix(&prefix) else {
                continue;
            };
            let is_table = (rest.starts_with("event/obj_ps3/") || rest.starts_with("battle/btl/"))
                && rest.ends_with(".bin");
            if !is_table {
                continue;
            }
            let original = archive.read(entry).unwrap();
            let table =
                MessageTable::parse(&original).unwrap_or_else(|e| panic!("{}: {e}", entry.path));
            assert_eq!(table.write().unwrap(), original, "{}", entry.path);
            let mut copy = table.clone();
            copy.import(&table.export(&charset).unwrap(), &charset)
                .unwrap_or_else(|e| panic!("{}: {e}", entry.path));
            assert_eq!(copy, table, "{}: TOML round trip", entry.path);
            tables += 1;
            messages += table.messages.len();
        }
    }
    eprintln!("{tables} message tables, {messages} messages, round-tripped");
}
