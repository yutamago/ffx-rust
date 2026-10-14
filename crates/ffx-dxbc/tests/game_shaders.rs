//! Every shader of the game decodes and translates to WGSL that naga
//! accepts. Needs `FFX_GAME_DIR`; skipped otherwise.

use std::collections::BTreeSet;
use std::path::PathBuf;

use ffx_dxbc::{Container, wgsl};

fn game_shaders() -> Option<Vec<Vec<u8>>> {
    let dir = PathBuf::from(std::env::var_os("FFX_GAME_DIR")?);
    let archive = ffx_vbf::Archive::open(dir.join("data/FFX_Data.vbf")).ok()?;
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for e in archive.entries() {
        if !e.path.contains("/shaders/d3d11/") || !e.path.ends_with(".phyre") {
            continue;
        }
        let bytes = archive.read(e).ok()?;
        for (_, blob) in ffx_dxbc::find_all(&bytes) {
            if seen.insert(ffx_dxbc::hash(blob)) {
                out.push(blob.to_vec());
            }
        }
    }
    Some(out)
}

fn validate(source: &str) -> Result<(), String> {
    let module = naga::front::wgsl::parse_str(source).map_err(|e| e.emit_to_string(source))?;
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .map(|_| ())
    .map_err(|e| format!("{e:?}"))
}

#[test]
fn every_game_shader_translates() {
    let Some(shaders) = game_shaders() else {
        eprintln!("FFX_GAME_DIR not set; skipped");
        return;
    };
    assert!(shaders.len() > 2000, "{} shaders", shaders.len());
    let mut failures = Vec::new();
    for blob in &shaders {
        let c = Container::parse(blob).expect("container");
        let vertex = c.program().is_some_and(|p| p.0 == "vs");
        let varyings = if vertex {
            wgsl::own_varyings(&c)
        } else {
            wgsl::ps_varyings(&c)
        }
        .expect("signature");
        let result = wgsl::translate(&c, &varyings, u32::from(!vertex))
            .map_err(|e| e.to_string())
            .and_then(|s| validate(&s.wgsl).map_err(|e| format!("{e}\n{}", s.wgsl)));
        if let Err(e) = result {
            failures.push(format!("{:016x}: {e}", ffx_dxbc::hash(blob)));
        }
    }
    let mut kinds = std::collections::BTreeMap::new();
    for f in &failures {
        let first: String = f.lines().take(3).collect::<Vec<_>>().join(" | ");
        let key: String = first.chars().skip(18).take(90).collect();
        *kinds.entry(key).or_insert(0) += 1;
    }
    for (k, n) in &kinds {
        eprintln!("{n:5} {k}");
    }
    if let Some(f) = failures.first() {
        eprintln!("{}", f.chars().take(4000).collect::<String>());
    }
    assert!(
        failures.is_empty(),
        "{} of {} failed",
        failures.len(),
        shaders.len()
    );
}
