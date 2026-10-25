//! `peek <snapshot> <addr> [len]` or `peek <snapshot> find <hex word>...`:
//! words at an address of a snapshot (as u32 and f32), or the addresses
//! holding a word.
use std::path::Path;

use ffx_fxdraw::mem::Mem;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let m = Mem::read(Path::new(&args.next().ok_or("snapshot")?))?;
    let a = args.next().ok_or("addr")?;
    if a == "find" {
        for w in args {
            let v = u32::from_str_radix(w.trim_start_matches("0x"), 16)?;
            println!("{v:#x}: {:x?}", m.find(&v.to_le_bytes(), 4, 64));
        }
        return Ok(());
    }
    let addr = u32::from_str_radix(a.trim_start_matches("0x"), 16)?;
    let len = match args.next() {
        Some(l) => u32::from_str_radix(l.trim_start_matches("0x"), 16)?,
        None => 0x100,
    };
    println!("image base {:#x}", m.image_base);
    for row in (0..len).step_by(16) {
        let ws: Vec<String> = (0..4)
            .map(|i| {
                m.u32(addr + row + 4 * i).map_or("--------".into(), |w| {
                    let f = f32::from_bits(w);
                    if f.is_finite() && f.abs() > 1e-6 && f.abs() < 1e7 {
                        format!("{w:08x}({f})")
                    } else {
                        format!("{w:08x}")
                    }
                })
            })
            .collect();
        println!("{:08x}: {}", addr + row, ws.join(" "));
    }
    Ok(())
}
