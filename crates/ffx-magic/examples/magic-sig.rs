//! Signatures for new ports, and what binds where.
//!
//! ```text
//! magic-sig <dll> <addr> <len> [rel32 offset...]   the masked hash
//! magic-sig --bind <dll> <addr>...                 the port each address binds to
//! magic-sig --covered <dll> <addr>...              the port or internal function covering it
//! ```
//! `<dll>` is a path to `magic_NNNN.dll`; addresses are the DLL's own
//! (preferred base), hex.

use ffx_magic::{Image, PORTS, bind, covered};

fn hex(s: &str) -> u32 {
    u32::from_str_radix(s.trim_start_matches("0x"), 16).unwrap_or_else(|_| panic!("not hex: {s}"))
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (mode, rest) = match args.first().map(String::as_str) {
        Some("--bind") => (1, &args[1..]),
        Some("--covered") => (2, &args[1..]),
        _ => (0, &args[..]),
    };
    let path = &rest[0];
    let file = std::fs::read(path).expect("read the DLL");
    let pe = ffx_magic::pe::Pe::parse(&file).expect("a PE image");
    let img = Image::new(path, &file, pe.preferred_base).expect("an overlay");
    if mode != 0 {
        for a in &rest[1..] {
            let addr = hex(a);
            let name = if mode == 1 {
                bind(&img, addr).map(|i| PORTS[i].0)
            } else {
                covered(&img, addr)
            };
            println!("{addr:#010x} {}", name.unwrap_or("-"));
        }
        return;
    }
    let addr = hex(&rest[1]);
    let len: u32 = rest[2].parse().expect("a length");
    let zero: Vec<u32> = rest[3..].iter().map(|s| hex(s)).collect();
    let h = img.masked_hash(addr, len, &zero).expect("inside the image");
    println!("{h:#018x}");
}
