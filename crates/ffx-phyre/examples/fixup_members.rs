//! For each byte-offset fixup, the class member it falls in:
//! `fixup_members <file.phyre>...`; prints `class member delta kind` counts.
use std::collections::BTreeMap;

fn main() {
    let mut seen: BTreeMap<(String, String, i64, &str), usize> = BTreeMap::new();
    for path in std::env::args().skip(1) {
        let bytes = std::fs::read(&path).unwrap();
        let Ok(c) = ffx_phyre::cluster::Cluster::parse(&bytes) else { continue };
        let (Ok(pointers), Ok(arrays)) = (c.pointer_fixups(&bytes), c.array_fixups(&bytes)) else {
            continue;
        };
        let mut note = |list: usize, member: u32, kind: &'static str| {
            if member & 0x8000_0000 == 0 {
                return;
            }
            let off = i64::from(member & 0x7fff_ffff);
            // Members of the class and its bases.
            let mut k = Some(c.lists[list].class);
            let mut best: Option<(String, i64)> = None;
            while let Some(ci) = k {
                for m in &c.classes[ci].members {
                    let d = off - i64::from(m.offset);
                    if (0..i64::from(m.size.max(1))).contains(&d)
                        || best.as_ref().is_none_or(|(_, b)| d.abs() < b.abs())
                    {
                        if (0..i64::from(m.size.max(1))).contains(&d) {
                            best = Some((m.name.clone(), d));
                            break;
                        }
                        best = Some((m.name.clone(), d));
                    }
                }
                k = c.classes[ci].base;
            }
            let (name, d) = best.unwrap_or_default();
            *seen
                .entry((c.classes[c.lists[list].class].name.clone(), name, d, kind))
                .or_default() += 1;
        };
        for (l, fs) in pointers.iter().enumerate() {
            for f in fs {
                note(l, f.member(), "ptr");
            }
        }
        for (l, fs) in arrays.iter().enumerate() {
            for f in fs {
                note(l, f.member(), "arr");
            }
        }
    }
    for ((class, member, d, kind), n) in seen {
        println!("{n:7} {kind} {class}.{member} {d:+}");
    }
}
