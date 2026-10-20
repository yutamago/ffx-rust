//! Fills the harvested-state store (`ffx_oracle::harvest`) by running a
//! driver function on a snapshot.
//!
//! ```text
//! cargo run --release -p ffx-oracle --example harvest -- \
//!     --snapshot ~/ffx-work/snapshots/battle.snap --driver 0x00790c10 \
//!     --frames 60 --max 8 [--targets ported|<file>|0xaddr,0xaddr] [--store <dir>]
//! ```
//!
//! `--targets ported` (the default) takes every address in
//! `crates/ffx-hook/src/ported/*.rs`; a file holds one address per line
//! (`#` comments). `--name` sets the store's snapshot name (default: the
//! file stem). `--args` passes comma-separated words to the driver,
//! `--conv cdecl|stdcall|thiscall:<this>|fastcall` its convention.
//! Needs `FFX_GAME_DIR`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use ffx_oracle::{Conv, Driver, Oracle, Pages, Store};

fn parse_u32(s: &str) -> u32 {
    let t = s.trim().replace('_', "");
    let v = t
        .strip_prefix("0x")
        .map_or_else(|| t.parse::<u32>(), |h| u32::from_str_radix(h, 16));
    v.unwrap_or_else(|_| panic!("not a number: {s}"))
}

/// Every `name @ 0x... =>` in the hook table.
fn ported() -> Vec<u32> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../ffx-hook/src/ported");
    let mut out = Vec::new();
    for e in std::fs::read_dir(&dir).expect("the hook table") {
        let p = e.expect("entry").path();
        if p.extension().is_none_or(|x| x != "rs") {
            continue;
        }
        for line in std::fs::read_to_string(&p).expect("a table file").lines() {
            let l = line.trim();
            if let Some((_, rest)) = l.split_once(" @ 0x")
                && let Some((hex, _)) = rest.split_once(" =>")
                && let Ok(v) = u32::from_str_radix(&hex.replace('_', ""), 16)
            {
                out.push(v);
            }
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}

fn targets(spec: &str) -> Vec<u32> {
    if spec == "ported" {
        return ported();
    }
    if Path::new(spec).is_file() {
        return std::fs::read_to_string(spec)
            .expect("targets file")
            .lines()
            .map(|l| l.split('#').next().unwrap_or("").trim())
            .filter(|l| !l.is_empty())
            .map(|l| parse_u32(l.split_whitespace().next().unwrap_or(l)))
            .collect();
    }
    spec.split(',').map(parse_u32).collect()
}

fn conv(s: &str) -> Conv {
    match s {
        "cdecl" => Conv::Cdecl,
        "stdcall" => Conv::Stdcall,
        "fastcall" => Conv::Fastcall,
        t => t.strip_prefix("thiscall:").map_or_else(
            || panic!("unknown convention {t}"),
            |this| Conv::Thiscall(parse_u32(this)),
        ),
    }
}

fn main() {
    let mut opts: BTreeMap<String, String> = BTreeMap::new();
    let mut it = std::env::args().skip(1);
    while let Some(k) = it.next() {
        let k = k.trim_start_matches("--").to_string();
        let v = it.next().unwrap_or_else(|| panic!("--{k} needs a value"));
        opts.insert(k, v);
    }
    let get = |k: &str| opts.get(k).cloned();
    let snapshot = PathBuf::from(get("snapshot").expect("--snapshot <file>"));
    let name = get("name").unwrap_or_else(|| {
        snapshot
            .file_stem()
            .map_or_else(|| "snapshot".into(), |s| s.to_string_lossy().into_owned())
    });
    let driver = Driver {
        addr: parse_u32(&get("driver").expect("--driver <addr>")),
        conv: conv(&get("conv").unwrap_or_else(|| "cdecl".into())),
        args: get("args")
            .map(|a| a.split(',').map(parse_u32).collect())
            .unwrap_or_default(),
        frames: get("frames").map_or(30, |f| parse_u32(&f)),
    };
    let max = get("max").map_or(8, |m| parse_u32(&m) as usize);
    let targets = targets(&get("targets").unwrap_or_else(|| "ported".into()));
    let store = get("store").map_or_else(Store::from_env, Store::at);
    let exe =
        PathBuf::from(std::env::var_os("FFX_GAME_DIR").expect("FFX_GAME_DIR")).join("FFX.exe");

    let t0 = Instant::now();
    let pages = Pages::read(&snapshot).expect("snapshot");
    let mut o = Oracle::from_snapshot(&exe, &pages).expect("oracle");
    // the game's clock reads a system DLL: one frame (1/60 s) per read
    o.fake_clock(1000.0, 1.0 / 60.0).expect("clock");
    eprintln!(
        "{name}: driver {:#010x} x{} frames, {} targets, max {max} each",
        driver.addr,
        driver.frames,
        targets.len()
    );
    let h = o.harvest(&driver, &targets, max).expect("harvest");
    let ran = t0.elapsed();
    let (mut saved, mut bytes) = (0usize, 0u64);
    let mut per: BTreeMap<u32, usize> = BTreeMap::new();
    for s in &h.states {
        if let Some(p) = store.save(&name, &snapshot, s).expect("save") {
            saved += 1;
            bytes += std::fs::metadata(&p).map_or(0, |m| m.len());
            *per.entry(s.target).or_default() += 1;
        }
    }
    eprintln!(
        "{} frames run{}; {} states ({} new, {} KiB) in {:.1?} (+{:.1?} saving)",
        h.frames,
        h.error
            .as_ref()
            .map_or_else(String::new, |e| format!(", then stopped: {e}")),
        h.states.len(),
        saved,
        bytes / 1024,
        ran,
        t0.elapsed().saturating_sub(ran)
    );
    if let Some((eip, top)) = h.stopped_at {
        eprintln!("stopped at eip {eip:#010x}, [esp] {top:#010x}");
        let base = o.image_base();
        let trail: Vec<String> = h
            .trail
            .iter()
            .map(|a| format!("{:#010x}", a.wrapping_sub(base).wrapping_add(0x40_0000)))
            .collect();
        eprintln!("last blocks (ffx.sym addresses): {}", trail.join(" "));
    }
    eprintln!("{} targets reached", h.hits.len());
    for (t, n) in &per {
        println!(
            "{t:#010x}\t{n}\t{}",
            h.hits.get(t).copied().unwrap_or_default()
        );
    }
}
