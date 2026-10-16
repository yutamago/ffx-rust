//! Counts the native function calls (`func` and `func_facc`) in every
//! shipped script, most used first, with the handler addresses:
//! `FFX_GAME_DIR=... cargo run -p ffx-script --example native_counts`.
use std::collections::BTreeMap;
use std::path::PathBuf;

fn main() {
    let dir = PathBuf::from(std::env::var_os("FFX_GAME_DIR").expect("FFX_GAME_DIR"));
    let archive = ffx_vbf::Archive::open(dir.join("data/FFX_Data.vbf")).unwrap();
    let natives =
        ffx_script::natives::Natives::from_exe(std::fs::read(dir.join("FFX.exe")).unwrap())
            .expect("a PE32 image");
    let mut counts: BTreeMap<u16, (u32, u32)> = BTreeMap::new();
    for e in archive.entries() {
        let p = &e.path;
        if !(p.ends_with(".ebp")
            || (p.starts_with("ffx_ps2/ffx/master/jppc/battle/mon/") && p.ends_with(".bin")))
        {
            continue;
        }
        let bytes = archive.read(archive.find(p).unwrap()).unwrap();
        let Ok(file) = ffx_script::Container::parse(bytes) else {
            continue;
        };
        let script = &file.script;
        let mut seen = std::collections::BTreeSet::new();
        for w in script.workers() {
            for i in script.decode(w.range.clone()).unwrap_or_default() {
                if matches!(i.opcode, 0x35 | 0x58) {
                    let id = i.operand.unwrap_or(0);
                    counts.entry(id).or_default().0 += 1;
                    if seen.insert(id) {
                        counts.entry(id).or_default().1 += 1;
                    }
                }
            }
        }
    }
    let mut v: Vec<_> = counts.into_iter().collect();
    v.sort_by_key(|&(id, (n, _))| (std::cmp::Reverse(n), id));
    println!("id\tcalls\tscripts\tstart\tpoll\tfloat\tint");
    for (id, (n, files)) in v {
        let h = natives.slots(id).unwrap_or([None; 4]);
        let f = |x: Option<u32>| x.map_or("-".to_string(), |a| format!("{a:#010x}"));
        println!(
            "{id:#06x}\t{n}\t{files}\t{}\t{}\t{}\t{}",
            f(h[0]),
            f(h[1]),
            f(h[2]),
            f(h[3])
        );
    }
}
