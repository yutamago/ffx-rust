//! The gate of the audio census: every FMOD import `FFX.exe` uses is in
//! `docs/formats/audio-platform.md` with its callers, and so is every
//! function a call log saw.
//!
//! ```bash
//! FFX_GAME_DIR=~/steam-games/FFX-X2-HD \
//!   FFX_FMOD_LOGS=~/ffx-work/m4/captures/audio/battle-attack-a/fmod.log,... \
//!   cargo test --release -p ffx-audio-census
//! ```
//!
//! Each test skips (passes with a note) when its variable is unset.

use std::path::PathBuf;

use ffx_audio_census::{boundary, demangle, fmodlog::Log};

fn doc() -> String {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/formats/audio-platform.md");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn imports() -> Option<Vec<boundary::Import>> {
    let Ok(dir) = std::env::var("FFX_GAME_DIR") else {
        eprintln!("skipped: FFX_GAME_DIR is not set");
        return None;
    };
    let exe = std::fs::read(PathBuf::from(dir).join("FFX.exe")).expect("FFX.exe");
    Some(boundary::imports(&exe).expect("a PE32 image"))
}

#[test]
fn every_import_is_documented_with_its_callers() {
    let Some(imports) = imports() else { return };
    let doc = doc();
    assert_eq!(
        imports.len(),
        40,
        "17 fmod_event.dll + 23 fmodex.dll imports"
    );
    let mut missing = Vec::new();
    for i in &imports {
        let name = demangle::short(&i.name);
        assert!(!i.thunks.is_empty(), "{name}: no thunk");
        assert!(!i.calls.is_empty(), "{name}: never called");
        if !doc.contains(&format!("`{name}`")) {
            missing.push(name.clone());
        }
        for c in &i.calls {
            if !doc.contains(&format!("{c:08x}")) {
                missing.push(format!("{name} call {c:#010x}"));
            }
        }
    }
    assert!(missing.is_empty(), "not in the doc: {missing:#?}");
}

fn logs() -> Option<Vec<(String, Log)>> {
    let Ok(list) = std::env::var("FFX_FMOD_LOGS") else {
        eprintln!("skipped: FFX_FMOD_LOGS is not set");
        return None;
    };
    Some(
        list.split(',')
            .filter(|p| !p.is_empty())
            .map(|p| {
                let text = std::fs::read_to_string(p).unwrap_or_else(|e| panic!("{p}: {e}"));
                (
                    p.to_owned(),
                    Log::parse(&text).unwrap_or_else(|e| panic!("{p}: {e}")),
                )
            })
            .collect(),
    )
}

#[test]
fn every_logged_call_is_known() {
    let Some(logs) = logs() else { return };
    let doc = doc();
    let imports = imports();
    let mut missing = Vec::new();
    for (path, log) in &logs {
        assert!(!log.calls.is_empty(), "{path}: no calls");
        for call in &log.calls {
            let f = log.function(call).expect("a call of a listed function");
            let name = demangle::short(&f.name);
            if !doc.contains(&format!("`{name}`")) && !missing.contains(&name) {
                missing.push(name.clone());
            }
            // an import call comes from one of the image's call sites
            if let (true, Some(imports)) = (f.is_import(), &imports) {
                let site = call.direct_site();
                assert!(
                    imports
                        .iter()
                        .any(|i| i.name == f.name && i.calls.contains(&site)),
                    "{path}: {name} called from {site:#010x}, not a static call site"
                );
            }
        }
    }
    assert!(
        missing.is_empty(),
        "logged but not in the doc: {missing:#?}"
    );
}
