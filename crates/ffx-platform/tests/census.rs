//! The gate of `docs/formats/platform.md`: every import of the platform
//! DLLs, and every import the movie and codec code calls, is listed there
//! with each function that calls it. Needs `FFX_GAME_DIR`; skips without.

use std::path::Path;

use ffx_platform::{Image, MOVIE_CODE, PLATFORM_DLLS, callers_by_function, movie_range, parse_sym};

fn setup() -> Option<(Image, std::collections::BTreeMap<u32, String>, String)> {
    let Some(game) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR unset: skipped");
        return None;
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let exe = std::fs::read(Path::new(&game).join("FFX.exe")).expect("FFX.exe");
    let mut image = Image::parse(exe).expect("PE");
    let names = parse_sym(&std::fs::read_to_string(root.join("symbols/ffx.sym")).expect("sym"));
    image.add_starts(names.keys().copied());
    let doc = std::fs::read_to_string(root.join("docs/formats/platform.md")).expect("doc");
    Some((image, names, doc))
}

#[test]
fn every_platform_import_and_caller_is_documented() {
    let Some((image, names, doc)) = setup() else {
        return;
    };
    let mut missing = Vec::new();
    let mut imports = 0;
    for dll in PLATFORM_DLLS {
        let mut n = 0;
        for i in image.imports_of(dll) {
            n += 1;
            if !doc.contains(&format!("`{}`", i.name)) {
                missing.push(format!("{dll}!{}", i.name));
            }
            for c in callers_by_function(&image, &names, i.slot) {
                if !doc.contains(&format!("0x{:08x}", c.function)) {
                    missing.push(format!("{dll}!{} caller 0x{:08x}", i.name, c.function));
                }
            }
        }
        assert!(n > 0, "{dll}: no imports");
        imports += n;
    }
    assert_eq!(imports, 69, "imports of the platform DLLs");
    assert!(missing.is_empty(), "not in platform.md: {missing:#?}");
}

#[test]
fn every_import_of_the_movie_code_is_documented() {
    let Some((image, names, doc)) = setup() else {
        return;
    };
    let mut missing = Vec::new();
    let mut found = 0;
    for i in &image.imports {
        for c in callers_by_function(&image, &names, i.slot) {
            if c.sites.iter().all(|s| movie_range(s.at).is_none()) {
                continue;
            }
            found += 1;
            if !doc.contains(&format!("`{}`", i.name)) {
                missing.push(format!("{}!{}", i.dll, i.name));
            }
            if !doc.contains(&format!("0x{:08x}", c.function)) {
                missing.push(format!("{}!{} caller 0x{:08x}", i.dll, i.name, c.function));
            }
        }
    }
    assert!(found > 0, "no imports called from {MOVIE_CODE:x?}");
    assert!(missing.is_empty(), "not in platform.md: {missing:#?}");
}

#[test]
fn import_sites_decode() {
    let Some((image, _, _)) = setup() else {
        return;
    };
    // CreateWindowExA: one `call [slot]` and its own thunk.
    let slot = image
        .imports_of("USER32.dll")
        .find(|i| i.name == "CreateWindowExA")
        .expect("CreateWindowExA")
        .slot;
    assert_eq!(slot, 0x00b0_c6ec);
    let callers = image.callers(slot);
    assert_eq!(callers.len(), 1);
    assert_eq!(callers[0].at, 0x0062_6f35);
    // XInputGetState: only through its thunk.
    let slot = image
        .imports_of("XINPUT9_1_0.dll")
        .find(|i| i.name == "XInputGetState")
        .expect("XInputGetState")
        .slot;
    let callers = image.callers(slot);
    assert_eq!(callers.len(), 1);
    assert_eq!(callers[0].thunk, Some(0x009d_a25a));
}
