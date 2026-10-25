//! `census <capture dir> <snapshot> [frame]` with `FFX_GAME_DIR` set: every
//! draw of the frame with its producer (`docs/formats/effects-draw.md`),
//! then the game's VFX lists and draw events as the snapshot holds them.
use std::path::Path;

use ffx_fxdraw::{
    capture::Capture,
    census,
    game::{Overlay, State},
    mem::Mem,
    shaders::Families,
};

#[allow(clippy::too_many_lines)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let cap = args
        .next()
        .ok_or("usage: census <capture> <snapshot> [frame]")?;
    let snap = args.next().ok_or("snapshot")?;
    let frame: usize = args.next().map_or(Ok(0), |f| f.parse())?;
    let game = std::env::var_os("FFX_GAME_DIR").ok_or("FFX_GAME_DIR")?;
    let fam = Families::from_game(Path::new(&game))?;
    let cap_path = cap;
    let cap = Capture::read(Path::new(&cap_path))?;
    let m = Mem::read(Path::new(&snap))?;
    let overlay = Overlay::from_game(&m, Path::new(&game));
    println!(
        "scene {} image base {:#x} overlay {:?}",
        cap.scene,
        m.image_base,
        overlay.as_ref().map(|o| (o.id, o.base))
    );
    let c = census::census(&cap, frame, &m, &fam, overlay.as_ref());
    let state = State::read(&m).unwrap_or_default();
    print!("{}", c.report(&state, &m));
    println!(
        "gate: {}",
        if c.traced() {
            "every effect draw traced"
        } else {
            "FAIL"
        }
    );
    // states and constants per VSP flags and pixel shader
    let sums = ffx_fxdraw::states::read(Path::new(&cap_path), frame)?;
    let mut table: std::collections::BTreeMap<(u32, String), std::collections::BTreeSet<String>> =
        std::collections::BTreeMap::new();
    for l in &c.lines {
        if let census::Producer::Vfx(v) = &l.producer
            && let Some(s) = sums.get(l.n)
        {
            table
                .entry((v.flags, l.ps_family.clone()))
                .or_default()
                .insert(format!("{} | {}", s.state, s.constants.join(" ")));
        }
    }
    // FFX_FXDRAW_GEOM=1: each effect draw's state with its geometry's
    // words +0x30..+0x60 (to find what picks the blend)
    if std::env::var_os("FFX_FXDRAW_GEOM").is_some() {
        for l in &c.lines {
            if let census::Producer::Vfx(v) = &l.producer
                && let Some(s) = sums.get(l.n)
            {
                let words: Vec<String> = (0..12)
                    .map(|i| format!("{:08x}", m.u32(v.geometry + 0x30 + 4 * i).unwrap_or(0)))
                    .collect();
                println!(
                    "geom {:4} {:#x} {} | {}",
                    l.n,
                    v.geometry,
                    s.state,
                    words.join(" ")
                );
            }
        }
    }
    for ((flags, ps), states) in &table {
        for s in states {
            println!("state {flags:#x} {ps}: {s}");
        }
    }
    if std::env::var_os("FFX_FXDRAW_STATE").is_some()
        && let Some(s) = State::read(&m)
    {
        println!("\n== PhyreScene {:#x} ==", s.scene);
        for (l, v) in &s.lists {
            println!("{} {}: {:x?}", l.name(), v.len(), &v[..v.len().min(64)]);
        }
        for e in &s.events {
            println!("event {e:?}");
        }
        println!("geometries {}", s.geometries.len());
        for g in &s.geometries {
            let segs: Vec<String> = g
                .segments
                .iter()
                .map(|x| {
                    let b: Vec<String> = x
                        .buffers
                        .iter()
                        .map(|a| format!("{}/{}@{:x}", a.vertices, a.indices, a.positions))
                        .collect();
                    b.join(",")
                })
                .collect();
            println!(
                "  geom {:#x} vt {:#x} key {:#018x} mi {:#x} in {:?} flags {:#x} segs [{}]",
                g.addr,
                g.vtable,
                g.key,
                g.mesh_instance,
                s.find_instance(g.mesh_instance),
                m.u32(g.addr + 0x3c).unwrap_or(0),
                segs.join(" ")
            );
        }
    }
    Ok(())
}
