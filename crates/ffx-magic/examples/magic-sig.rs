//! Signatures for new ports, and what binds where.
//!
//! ```text
//! magic-sig <dll> <addr> <len> [rel32 offset...]   the masked hash
//! magic-sig --bind <dll> <addr>...                 the port each address binds to
//! magic-sig --covered <dll> <addr>...              the port or internal function covering it
//! magic-sig --batch <dll>                          code identities, for tools/magic-worklist.py
//! ```
//! `--batch` reads `<addr> <len> [rel32 offset...]` lines (hex addresses
//! and offsets, decimal lengths) and prints one line each: the address,
//! the masked hash, the imports its relocated words name (`call [IAT]`,
//! comma-separated, `-` for none) and per `rel32` field its target (hex)
//! or `imp:<name>` when the target is an import thunk (`jmp [IAT]`).
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
        Some("--batch") => (3, &args[1..]),
        _ => (0, &args[..]),
    };
    let path = &rest[0];
    let file = std::fs::read(path).expect("read the DLL");
    let pe = ffx_magic::pe::Pe::parse(&file).expect("a PE image");
    let img = Image::new(path, &file, pe.preferred_base).expect("an overlay");
    if mode == 3 {
        batch(&img);
        return;
    }
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

/// `--batch`: see the module docs.
fn batch(img: &Image) {
    use std::io::{BufRead, Write};
    let stdin = std::io::stdin();
    let mut out = std::io::BufWriter::new(std::io::stdout());
    for line in stdin.lock().lines() {
        let line = line.expect("stdin");
        let mut it = line.split_whitespace();
        let (Some(a), Some(n)) = (it.next(), it.next()) else {
            continue;
        };
        let addr = hex(a);
        let len: u32 = n.parse().expect("a length");
        let offs: Vec<u32> = it.map(hex).collect();
        let h = img.masked_hash(addr, len, &offs).unwrap_or(0);
        let imports: Vec<&str> = img
            .operands(addr, len)
            .into_iter()
            .filter_map(|w| img.import_at(w))
            .collect();
        let imports = if imports.is_empty() {
            "-".to_string()
        } else {
            imports.join(",")
        };
        let callees: Vec<String> = offs
            .iter()
            .map(|&o| {
                let t = img.rel32_target(addr + o);
                let thunk = img.word(t).to_le_bytes()[..2] == [0xff, 0x25];
                match thunk.then(|| img.import_at(img.word(t + 2))).flatten() {
                    Some(name) => format!("imp:{name}"),
                    None => format!("{t:#x}"),
                }
            })
            .collect();
        let _ = writeln!(out, "{addr:#x} {h:#018x} {imports} {}", callees.join(" "));
    }
}
