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

/// The tables shared by all languages round-trip, and their data fit what
/// the field names say.
#[test]
fn shared_tables_round_trip_and_fit_their_fields() {
    let Some(archive) = archive() else { return };
    let language = Language::English;
    let charset =
        Charset::from_table(&read(&archive, &language.charset_path()).unwrap(), language).unwrap();
    for schema in ffx_kernel::SHARED_SCHEMAS {
        let path = schema.path(language);
        let original = read(&archive, &path).unwrap_or_else(|| panic!("{path} missing"));
        let table = Table::parse(&original).unwrap();
        let toml = export(&table, schema, &charset).unwrap();
        let mut reimported = table.clone();
        import(&mut reimported, schema, &charset, &toml).unwrap();
        assert_eq!(
            reimported.to_bytes().unwrap(),
            original,
            "{path}: TOML round trip"
        );
    }
    let field = |schema: &Schema, record: &[u8], name: &str| -> u32 {
        let f = schema.fields.iter().find(|f| f.name == name).unwrap();
        record[f.offset..f.offset + f.kind.len()]
            .iter()
            .rev()
            .fold(0, |v, &b| v << 8 | u32::from(b))
    };
    let table = |name: &str| {
        let schema = Schema::by_file(name).unwrap();
        (
            schema,
            Table::parse(&read(&archive, &schema.path(language)).unwrap()).unwrap(),
        )
    };
    // Equipment: a ply_save character (0-17), weapon or armor, abilities 0x8000 + n or empty.
    for name in ["weapon", "shop_arms", "buki_get"] {
        let (schema, t) = table(name);
        for id in t.ids() {
            let r = t.record(id).unwrap();
            if name != "buki_get" && field(schema, r, "in_use") == 0 {
                continue;
            }
            assert!(field(schema, r, "character") < 18, "{name} #{id}");
            assert!(field(schema, r, "armor") < 2, "{name} #{id}");
            for a in ["ability_1", "ability_2", "ability_3", "ability_4"] {
                let v = field(schema, r, a);
                assert!(
                    v == 0 || v == 0xff || v >> 12 == 8,
                    "{name} #{id} {a} {v:#x}"
                );
            }
        }
    }
    // Treasure: kinds 0, 2, 5, 10; equipment points into buki_get, items
    // and key items carry their kind's ID prefix.
    let (schema, takara) = table("takara");
    let (_, buki) = table("buki_get");
    for id in takara.ids() {
        let r = takara.record(id).unwrap();
        let item = field(schema, r, "item");
        match field(schema, r, "kind") {
            0 => {}
            2 => assert_eq!(item >> 12, 2, "takara #{id}"),
            5 => assert!(
                buki.record(u16::try_from(item).unwrap()).is_some(),
                "takara #{id}"
            ),
            10 => assert_eq!(item >> 12, 0xa, "takara #{id}"),
            k => panic!("takara #{id}: kind {k}"),
        }
    }
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
