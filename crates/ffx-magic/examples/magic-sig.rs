//! Signatures for new ports, and what binds where.
//!
//! ```text
//! magic-sig <dll> <addr> <len> [rel32 offset...]   the masked hash
//! magic-sig --bind <dll> <addr>...                 the port each address binds to
//! ```
//! `<dll>` is a path to `magic_NNNN.dll`; addresses are the DLL's own
//! (preferred base), hex.

use ffx_magic::{Image, PORTS, bind};

fn hex(s: &str) -> u32 {
    u32::from_str_radix(s.trim_start_matches("0x"), 16).unwrap_or_else(|_| panic!("not hex: {s}"))
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (binding, rest) = match args.first().map(String::as_str) {
        Some("--bind") => (true, &args[1..]),
        _ => (false, &args[..]),
    };
    let path = &rest[0];
    let file = std::fs::read(path).expect("read the DLL");
    let pe = ffx_magic::pe::Pe::parse(&file).expect("a PE image");
    let img = Image::new(path, &file, pe.preferred_base).expect("an overlay");
    if binding {
        for a in &rest[1..] {
            let addr = hex(a);
            match bind(&img, addr) {
                Some(i) => println!("{addr:#010x} {}", PORTS[i].0),
                None => println!("{addr:#010x} -"),
            }
        }
        return;
    }
    let addr = hex(&rest[1]);
    let len: u32 = rest[2].parse().expect("a length");
    let zero: Vec<u32> = rest[3..].iter().map(|s| hex(s)).collect();
    let h = img.masked_hash(addr, len, &zero).expect("inside the image");
    println!("{h:#018x}");
}
