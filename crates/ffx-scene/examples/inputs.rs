//! Prints a capture's scene inputs (`ffx_scene::inputs`): one block per
//! map `renderWorld` call, with `--camera` the `PCamera` bytes as floats,
//! with `--app` the application's.
//!
//! ```bash
//! cargo run --release -p ffx-scene --example inputs -- <capture.ffxgfx> [--camera] [--app]
//! ```

fn floats(bytes: &[u8], from: usize) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    for (k, c) in bytes.as_chunks::<4>().0.iter().enumerate() {
        if k % 4 == 0 {
            let _ = write!(out, "\n    +{:#05x}", from + 4 * k);
        }
        let f = f32::from_le_bytes(*c);
        let _ = write!(out, " {f:>14.7}");
    }
    out
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let path = args
        .first()
        .ok_or("usage: inputs <capture.ffxgfx> [--camera] [--app]")?;
    let list = ffx_scene::inputs::read_wired(&std::fs::read(path)?)?;
    for (s, wire) in &list {
        println!("{}", ffx_scene::inputs::summary(s));
        if let Some(w) = wire
            && args.iter().any(|a| a == "--wire")
        {
            println!("  wire: camera world {:?}", w.camera_world);
            println!(
                "  texture quality {} anisotropy {}",
                w.texture_quality,
                f32::from_bits(w.anisotropy)
            );
            println!("  clusters {:?}", w.cluster_assets);
            for (m, t) in w.material_textures.iter().enumerate() {
                for t in t {
                    println!(
                        "  material {m} slot {:#x} kind {:#x} set {} {}",
                        t.offset, t.kind, t.set, t.asset
                    );
                }
            }
        }
        if args.iter().any(|a| a == "--camera") {
            println!("  camera:{}", floats(&s.camera, 0));
        }
        if args.iter().any(|a| a == "--app") {
            println!("  app:{}", floats(&s.app, ffx_gfx::scene_inputs::APP_FROM));
        }
    }
    Ok(())
}
