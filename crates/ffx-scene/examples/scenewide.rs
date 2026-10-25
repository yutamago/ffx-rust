//! Lists the scene-wide parameters a map's shaders read: `scenewide
//! <model path>...` (archive paths), with `FFX_GAME_DIR` set. For each
//! scene-wide location table entry a program uses: its offset and size in
//! the scene-wide buffer and the `$Globals` variable it lands in (from the
//! program's reflection).
use std::collections::BTreeMap;

use ffx_scene::build::Clusters;
use ffx_scene::effect::Source;

fn name_at(code: &[u8], slot: u32, target: u32) -> String {
    let Ok(c) = ffx_dxbc::Container::parse(code) else {
        return "?".into();
    };
    let Ok(Some(r)) = c.reflection() else {
        return "?".into();
    };
    let Some(b) = r.buffer_at(slot) else {
        return "?".into();
    };
    for v in &b.variables {
        if target >= v.offset && target < v.offset + v.size {
            let mut n = format!("{}+{}", v.name, target - v.offset);
            for m in &v.members {
                if target - v.offset >= m.offset {
                    n = format!("{}.{}+{}", v.name, m.name, target - v.offset - m.offset);
                }
            }
            return n;
        }
    }
    "?".into()
}

fn main() {
    let game = std::env::var("FFX_GAME_DIR").expect("FFX_GAME_DIR");
    let archive = ffx_vbf::Archive::open(std::path::Path::new(&game).join("data/FFX_Data.vbf"))
        .expect("archive");
    let mut seen: BTreeMap<(u16, u32, u8), BTreeMap<String, usize>> = BTreeMap::new();
    for model in std::env::args().skip(1) {
        let c = Clusters::load(
            |p| archive.find(p).and_then(|e| archive.read(e).ok()),
            &model,
        )
        .expect("clusters");
        for v in c.effects.values() {
            for rp in &v.passes {
                for sh in &rp.shaders {
                    for sp in &sh.passes {
                        for (prog, table) in [(&sp.vs, &sp.vs_params), (&sp.ps, &sp.ps_params)] {
                            for (src, loc) in table.sourced() {
                                if src != Source::SceneWide || loc.target == u32::MAX {
                                    continue;
                                }
                                let n = name_at(&prog.code, prog.global_cb, loc.target);
                                *seen
                                    .entry((loc.offset, loc.size, loc.kind))
                                    .or_default()
                                    .entry(n)
                                    .or_default() += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    for ((o, s, k), names) in seen {
        println!("{o:#06x} {s:3} kind {k:#04x} {names:?}");
    }
}
