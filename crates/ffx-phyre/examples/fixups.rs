//! Lists the fixups of a cluster with member names:
//! `fixups <file.phyre> [class ...]` (only lists of those classes, and
//! pointers into them).
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let bytes = std::fs::read(&args[1]).expect("read");
    let e = ffx_phyre::edit::ClusterEdit::parse(&bytes).expect("parse");
    let c = &e.cluster;
    let wanted = |i: usize| args.len() < 3 || args[2..].contains(&c.classes[e.lists[i].class].name);
    let field = |class: usize, m: u32| -> String {
        if m & 0x8000_0000 != 0 {
            format!("+{:#x}", m & 0x7fff_ffff)
        } else {
            c.member_by_index(m).map_or(format!("#{m}"), |(k, mm)| {
                format!("{}.{}", c.classes[k].name, mm.name)
            })
        }
        .replace(&format!("{}.", c.classes[class].name), "")
    };
    for (i, l) in e.lists.iter().enumerate() {
        let name = &c.classes[l.class].name;
        for f in &l.pointer_fixups {
            let target = f.0[3] as usize;
            if !(wanted(i)
                || (f.user_fixup().is_none() && target < e.lists.len() && wanted(target)))
            {
                continue;
            }
            match f.user_fixup() {
                Some(u) => println!(
                    "P {i}:{name}[{}].{} = user {u} {:?}",
                    f.0[1],
                    field(l.class, f.0[0]),
                    c.user_fixups[u as usize].text
                ),
                None => println!(
                    "P {i}:{name}[{}].{} -> {target}:{}[{}] +{:#x} x{}",
                    f.0[1],
                    field(l.class, f.0[0]),
                    c.classes[e.lists[target].class].name,
                    f.0[2],
                    f.0[4],
                    f.0[5]
                ),
            }
        }
        if !wanted(i) {
            continue;
        }
        for f in &l.array_fixups {
            println!(
                "A {i}:{name}[{}].{} w2={} at={:#x}",
                f.0[1],
                field(l.class, f.0[0]),
                f.0[2],
                f.0[3]
            );
        }
        for f in &l.pointer_array_fixups {
            println!(
                "PA {i}:{name}[{}].{} n={} at={:#x}",
                f.0[1],
                field(l.class, f.0[0]),
                f.0[2],
                f.0[3]
            );
        }
    }
}
