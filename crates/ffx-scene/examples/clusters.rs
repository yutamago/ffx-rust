//! Dumps the namespace and instance lists of archive clusters:
//! `clusters <path filter> [--objects <class>]`, with `FFX_GAME_DIR` set.
//! A tool for reading `PhyreEngine` classes the scene builder decodes.
use std::fmt::Write as _;

fn main() {
    let mut args = std::env::args().skip(1);
    let filter = args.next().expect("usage: clusters <filter> [class...]");
    let want: Vec<String> = args.collect();
    let game = std::env::var("FFX_GAME_DIR").expect("FFX_GAME_DIR");
    let archive = ffx_vbf::Archive::open(std::path::Path::new(&game).join("data/FFX_Data.vbf"))
        .expect("archive");
    for e in archive.entries() {
        if !e.path.contains(filter.as_str()) || !e.path.ends_with(".phyre") {
            continue;
        }
        let bytes = archive.read(e).expect("read");
        let c = ffx_phyre::cluster::Cluster::parse(&bytes).expect("cluster");
        println!("== {} ({} bytes)", e.path, bytes.len());
        if want.iter().any(|w| w == "classes") {
            for cl in &c.classes {
                println!("class {} size {} base {:?}", cl.name, cl.size, cl.base);
                for m in &cl.members {
                    let ty = if (m.type_index as usize) < c.types.len() {
                        c.types[m.type_index as usize].clone()
                    } else {
                        c.classes
                            .get(m.type_index as usize - c.types.len() - 1)
                            .map_or("?".into(), |x| x.name.clone())
                    };
                    println!("    +{:#x} {} size {} : {ty}", m.offset, m.name, m.size);
                }
            }
        }
        for l in &c.lists {
            let class = &c.classes[l.class];
            println!(
                "list {} x{} size {} arrays {} at {:#x}",
                class.name,
                l.count,
                l.object_size(),
                l.arrays_size,
                l.offset
            );
            if want.iter().any(|w| w == "strings") {
                let arr = &bytes[l.arrays()];
                let words: Vec<String> = arr
                    .split(|&b| b == 0)
                    .filter(|s| s.len() > 2 && s.iter().all(|b| (0x20..0x7f).contains(b)))
                    .map(|s| String::from_utf8_lossy(s).into_owned())
                    .collect();
                println!("  strings: {words:?}");
            }
            if !want.iter().any(|w| *w == class.name || w == "all") {
                continue;
            }
            let mut chain = Some(l.class);
            while let Some(k) = chain {
                let cl = &c.classes[k];
                for m in &cl.members {
                    let ty = if (m.type_index as usize) < c.types.len() {
                        c.types[m.type_index as usize].clone()
                    } else {
                        c.classes
                            .get(m.type_index as usize - c.types.len() - 1)
                            .map_or("?".into(), |x| x.name.clone())
                    };
                    println!(
                        "    {}::{} +{:#x} size {} : {ty}",
                        cl.name, m.name, m.offset, m.size
                    );
                }
                chain = cl.base;
            }
            for i in 0..l.count.min(16) {
                let at = l.object(i);
                let mut s = String::new();
                for w in bytes[at..at + l.object_size()].chunks(4) {
                    let mut x = [0u8; 4];
                    x[..w.len()].copy_from_slice(w);
                    let _ = write!(s, "{:08x} ", u32::from_le_bytes(x));
                }
                println!("  [{i}] {s}");
            }
            let arr = l.arrays();
            if arr.len() <= 4096 {
                println!("  arrays: {:02x?}", &bytes[arr]);
            }
        }
        for u in &c.user_fixups {
            println!(
                "user {} {:?} {:02x?}",
                c.types
                    .get(u.type_index as usize)
                    .map_or("?", String::as_str),
                u.text,
                &u.data[..u.data.len().min(32)]
            );
        }
    }
}
