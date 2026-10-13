//! Round-trips every kernel table of every language through the reader,
//! the text codec and TOML. Needs `FFX_GAME_DIR`; skipped otherwise.

use std::path::PathBuf;

use ffx_kernel::{Schema, Table, export, import};
use ffx_text::{Charset, Language};
use ffx_vbf::Archive;

fn archive() -> Option<Archive> {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return None;
    };
    Some(Archive::open(PathBuf::from(dir).join("data/FFX_Data.vbf")).unwrap())
}

fn read(archive: &Archive, path: &str) -> Option<Vec<u8>> {
    archive.find(path).map(|e| archive.read(e).unwrap())
}

#[test]
fn every_table_round_trips_in_every_language() {
    let Some(archive) = archive() else { return };
    let mut tables = 0;
    let mut strings = 0;
    for language in Language::ALL {
        let charset =
            Charset::from_table(&read(&archive, &language.charset_path()).unwrap(), language)
                .unwrap();
        for schema in ffx_kernel::SCHEMAS {
            let path = format!("{}{}.bin", language.kernel_dir(), schema.file);
            let original = read(&archive, &path).unwrap_or_else(|| panic!("{path} missing"));
            let table = Table::parse(&original).unwrap_or_else(|e| panic!("{path}: {e}"));
            assert_eq!(
                table.to_bytes().unwrap(),
                original,
                "{path}: container round trip"
            );

            for id in table.ids() {
                assert_eq!(
                    table.record(id).unwrap().len(),
                    usize::from(schema.record_size),
                    "{path}"
                );
                for field in 0..schema.strings.len() {
                    let bytes = table.string(id, field).unwrap();
                    let text = charset.decode(bytes);
                    assert_eq!(
                        charset.encode(&text).unwrap(),
                        bytes,
                        "{path} #{id} string {field}: {text:?}"
                    );
                    strings += 1;
                }
            }

            let toml = export(&table, schema, &charset).unwrap();
            let mut reimported = table.clone();
            import(&mut reimported, schema, &charset, &toml)
                .unwrap_or_else(|e| panic!("{path}: {e}"));
            assert_eq!(
                reimported.to_bytes().unwrap(),
                original,
                "{path}: TOML round trip"
            );
            tables += 1;
        }
    }
    assert_eq!(tables, 8 * ffx_kernel::SCHEMAS.len());
    eprintln!("{tables} tables, {strings} strings round-tripped");
}

#[test]
fn edits_land_where_the_game_reads_them() {
    let Some(archive) = archive() else { return };
    let language = Language::English;
    let charset =
        Charset::from_table(&read(&archive, &language.charset_path()).unwrap(), language).unwrap();
    let schema = Schema::by_file("item").unwrap();
    let original = read(&archive, &format!("{}item.bin", language.kernel_dir())).unwrap();
    let mut table = Table::parse(&original).unwrap();

    let potion = export(&table, schema, &charset).unwrap();
    assert!(potion.contains("name = \"Potion\""), "{}", &potion[..400]);
    assert!(potion.contains("description = \"Restores 200 HP of one character.\""));

    import(
        &mut table,
        schema,
        &charset,
        "[[record]]\nid = 0\nname = \"Mega Potion\"\npower = 20\n",
    )
    .unwrap();
    let edited = Table::parse(&table.to_bytes().unwrap()).unwrap();
    assert_eq!(edited.record(0).unwrap()[0x2a], 20);
    assert_eq!(charset.decode(edited.string(0, 0).unwrap()), "Mega Potion");
    // Everything else is untouched.
    assert_eq!(edited.record(1), Table::parse(&original).unwrap().record(1));
}
