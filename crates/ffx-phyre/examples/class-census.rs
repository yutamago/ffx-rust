//! Census of the classes the player's clusters name (M5 scope,
//! `tools/phyre-classes.py`).
//!
//! ```text
//! cargo run --release -p ffx-phyre --example class-census -- OUT.tsv VBF...
//! ```
//!
//! Every archive entry that starts with `RYHP` is parsed. One row per class
//! name: its base in the first cluster that names it, the number of clusters
//! whose namespace names it, the number with an instance list of it, the
//! objects in those lists, and the first cluster naming it. Rows with
//! `kind` `fixup` are user fixup strings whose type names a class or type
//! descriptor (`PClassDescriptor`, `PType`): classes the loader looks up by
//! name at run time.

use std::collections::BTreeMap;
use std::io::Write;

use ffx_phyre::cluster::Cluster;
use ffx_vbf::Archive;

#[derive(Default)]
struct Row {
    base: String,
    ns_files: u32,
    inst_files: u32,
    objects: u64,
    first: String,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some((out, vbfs)) = args.split_first() else {
        eprintln!("usage: class-census OUT.tsv VBF...");
        std::process::exit(2);
    };
    let mut classes: BTreeMap<String, Row> = BTreeMap::new();
    let mut fixups: BTreeMap<(String, String), Row> = BTreeMap::new();
    let (mut parsed, mut failed, mut big_endian) = (0u32, 0u32, 0u32);
    for vbf in vbfs {
        let archive = Archive::open(vbf).unwrap_or_else(|e| panic!("{vbf}: {e}"));
        for entry in archive.entries().iter().filter(|e| e.size >= 0x54) {
            let Ok(bytes) = archive.read(entry) else {
                continue;
            };
            match bytes.get(..4) {
                Some(b"RYHP") => {}
                Some(b"PHYR") => {
                    big_endian += 1;
                    continue;
                }
                _ => continue,
            }
            let cluster = match Cluster::parse(&bytes) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("{}: {e}", entry.path);
                    failed += 1;
                    continue;
                }
            };
            parsed += 1;
            let mut inst: BTreeMap<usize, u64> = BTreeMap::new();
            for list in &cluster.lists {
                *inst.entry(list.class).or_default() += u64::from(list.count);
            }
            for (i, class) in cluster.classes.iter().enumerate() {
                let row = classes.entry(class.name.clone()).or_default();
                if row.ns_files == 0 {
                    row.first.clone_from(&entry.path);
                    row.base = class
                        .base
                        .and_then(|b| cluster.classes.get(b))
                        .map_or_else(String::new, |b| b.name.clone());
                }
                row.ns_files += 1;
                if let Some(n) = inst.get(&i) {
                    row.inst_files += 1;
                    row.objects += n;
                }
            }
            for fix in &cluster.user_fixups {
                let ty = cluster
                    .types
                    .get(fix.type_index as usize)
                    .cloned()
                    .unwrap_or_default();
                if !(ty.contains("ClassDescriptor") || ty.starts_with("PType")) {
                    continue;
                }
                let row = fixups.entry((ty, fix.text.clone())).or_default();
                if row.ns_files == 0 {
                    row.first.clone_from(&entry.path);
                }
                row.ns_files += 1;
            }
        }
    }
    let mut f = std::fs::File::create(out).unwrap_or_else(|e| panic!("{out}: {e}"));
    writeln!(
        f,
        "# kind\tname\tbase\tnamespace_files\tinstance_files\tobjects\tfirst_file"
    )
    .unwrap();
    for (name, r) in &classes {
        writeln!(
            f,
            "class\t{name}\t{}\t{}\t{}\t{}\t{}",
            r.base, r.ns_files, r.inst_files, r.objects, r.first
        )
        .unwrap();
    }
    for ((ty, text), r) in &fixups {
        writeln!(f, "fixup\t{text}\t{ty}\t{}\t0\t0\t{}", r.ns_files, r.first).unwrap();
    }
    eprintln!(
        "{parsed} clusters parsed, {failed} failed, {big_endian} big-endian skipped; {} classes, {} class-name fixups",
        classes.len(),
        fixups.len()
    );
}
