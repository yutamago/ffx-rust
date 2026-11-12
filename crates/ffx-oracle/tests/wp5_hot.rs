//! M5 finish, WP5 (`ffx_prender::wp5_hot`): the originals the ports still
//! called in Unicorn, against the original machine code: results and every
//! byte written, on junk-filled memory, on the image alone and over a
//! snapshot (`FFX_SNAPSHOT`). Needs `FFX_GAME_DIR`.

use std::path::PathBuf;

use ffx_bridge::Mem as _;
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_prender::wp5_hot as hot;

/// Two oracles alike: the original runs in `a`, the port over `b`'s memory.
fn twins() -> Vec<(&'static str, Oracle, Oracle)> {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR").map(PathBuf::from) else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return vec![];
    };
    let exe = dir.join("FFX.exe");
    let mut out = vec![(
        "image",
        Oracle::load(&exe).unwrap(),
        Oracle::load(&exe).unwrap(),
    )];
    if let Some(snap) = std::env::var_os("FFX_SNAPSHOT").filter(|s| !s.is_empty()) {
        let pages = Pages::read(PathBuf::from(snap)).unwrap();
        out.push((
            "snapshot",
            Oracle::from_snapshot(&exe, &pages).unwrap(),
            Oracle::from_snapshot(&exe, &pages).unwrap(),
        ));
    }
    for (_, a, b) in &mut out {
        a.set_fpu_control(0x027f).unwrap();
        b.set_fpu_control(0x027f).unwrap();
        a.set_max_instructions(20_000_000);
    }
    out
}

/// `len` bytes of junk at the same address in both (a fresh block of each).
#[allow(clippy::cast_possible_truncation)] // the pattern wraps every 256 bytes on purpose
fn junk(a: &mut Oracle, b: &mut Oracle, len: usize, seed: u8) -> u32 {
    let bytes: Vec<u8> = (0..len)
        .map(|i| {
            seed.wrapping_mul(31)
                .wrapping_add((i as u8).wrapping_mul(7))
                | 1
        })
        .collect();
    let p = a.alloc_bytes(&bytes).unwrap();
    let q = b.alloc_bytes(&bytes).unwrap();
    assert_eq!(p, q, "both oracles allocate alike");
    p
}

fn same(label: &str, a: &mut Oracle, b: &mut Oracle) {
    let d = a.diff(b).unwrap();
    assert!(
        d.is_empty(),
        "{label}: {} bytes differ, first {:x?}",
        d.len(),
        &d[..d.len().min(8)]
    );
    a.clear_dirty();
    b.clear_dirty();
}

#[test]
fn job_list_ctor_matches_the_original() {
    for (label, mut a, mut b) in twins() {
        for seed in [3u8, 0x80] {
            // a list in the middle of junk, its neighbours untouched
            let block = junk(&mut a, &mut b, 0x2100, seed);
            let this = block + 0x40;
            // a snapshot's image sits where the game's process had it
            let at = a.rebase(0x0061_3f70);
            let r = a.call(at, Conv::Thiscall(this), &[]).unwrap();
            let port = hot::job_list_ctor(&mut b, this);
            assert_eq!(r.eax, port, "{label}: eax");
            assert_eq!(port, this);
            same(&format!("{label}: job list {seed}"), &mut a, &mut b);
            // the two bytes after the 16-bit count stay
            let mut kept = [0u8; 2];
            b.read_bytes(this + 2, &mut kept);
            assert_ne!(kept, [0, 0]);
        }
    }
}
