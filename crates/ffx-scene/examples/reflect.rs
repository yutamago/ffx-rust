//! Scratch: the used constant-buffer variables of a capture's scene shaders.
use std::collections::BTreeMap;

use ffx_scene::capture::Capture;

fn main() {
    let path = std::env::args().nth(1).expect("capture");
    let cap = Capture::read(&std::fs::read(path).expect("read")).expect("capture");
    let mut seen = BTreeMap::new();
    for (_, d) in cap.draws() {
        if Capture::scene_targets(d).is_none() {
            continue;
        }
        for id in [d.vs, d.ps] {
            if let Some(code) = cap.shader(id) {
                *seen
                    .entry((ffx_scene::key::shader(code), id))
                    .or_insert(0u32) += 1;
            }
        }
    }
    for ((k, id), n) in seen {
        let code = cap.shader(id).unwrap();
        let c = ffx_dxbc::Container::parse(code).unwrap();
        let kind = c.program().map_or("?", |p| p.0);
        println!("{kind} {k:016x} draws {n}");
        if let Ok(Some(r)) = c.reflection() {
            for b in &r.bindings {
                println!("  bind {:?} {} slot {}", b.kind, b.name, b.slot);
            }
            for b in &r.buffers {
                for v in b
                    .variables
                    .iter()
                    .filter(|v| v.used && !v.members.is_empty())
                {
                    let m: Vec<String> = v
                        .members
                        .iter()
                        .map(|m| {
                            format!(
                                "{}@{}[{}x{}x{}]",
                                m.name, m.offset, m.rows, m.columns, m.elements
                            )
                        })
                        .collect();
                    println!("  struct {}: {}", v.name, m.join(" "));
                }
                let used: Vec<String> = b
                    .variables
                    .iter()
                    .filter(|v| v.used)
                    .map(|v| format!("{}@{}+{}", v.name, v.offset, v.size))
                    .collect();
                println!(
                    "  cb {} size {} ({} vars) used: {}",
                    b.name,
                    b.size,
                    b.variables.len(),
                    used.join(" ")
                );
            }
        }
    }
}
