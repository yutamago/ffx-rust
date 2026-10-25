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
    let list = ffx_scene::inputs::read(&std::fs::read(path)?)?;
    for s in &list {
        println!("{}", ffx_scene::inputs::summary(s));
        if args.iter().any(|a| a == "--camera") {
            println!("  camera:{}", floats(&s.camera, 0));
        }
        if args.iter().any(|a| a == "--app") {
            println!("  app:{}", floats(&s.app, ffx_gfx::scene_inputs::APP_FROM));
        }
    }
    Ok(())
}
