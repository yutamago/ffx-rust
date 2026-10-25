//! `probe <capture> <snapshot> <draw>`: where a draw's ring bytes sit in
//! the snapshot (every slot, every hit), for working out a producer by
//! hand.
use std::path::Path;

use ffx_fxdraw::{capture::Capture, mem::Mem};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let cap = Capture::read(Path::new(&args.next().ok_or("capture")?))?;
    let m = Mem::read(Path::new(&args.next().ok_or("snapshot")?))?;
    let n: usize = args.next().ok_or("draw")?.parse()?;
    let d = &cap.frames[0].draws[n];
    println!(
        "draw {n}: {:?} count {} start {} base {} indices {:?}",
        d.draw.kind, d.draw.count, d.draw.start, d.draw.base, d.indices
    );
    for s in &d.slots {
        let words: Vec<String> = s
            .bytes
            .as_chunks::<4>()
            .0
            .iter()
            .take(12)
            .map(|c| format!("{:?}", f32::from_le_bytes(*c)))
            .collect();
        println!(
            "slot {} stride {} ring {} first {} bytes {}: {}",
            s.slot,
            s.stride,
            s.ring,
            s.first,
            s.bytes.len(),
            words.join(" ")
        );
        if s.bytes.len() >= 8 {
            let k = s.bytes.len().min(48);
            let hits = m.find(&s.bytes[..k], 4, 16);
            println!("  hits of the first {k} bytes: {hits:x?}");
        }
    }
    for (slot, cb) in &d.draw.vs_cb {
        println!("vs cb{slot}: {} bytes", cb.len());
    }
    Ok(())
}
