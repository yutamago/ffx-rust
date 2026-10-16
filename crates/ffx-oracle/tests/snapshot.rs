//! Runs original functions on a snapshot of the running game
//! (`cargo run -p ffx-harness --example snapshot`). Needs `FFX_GAME_DIR` and
//! `FFX_SNAPSHOT` (a snapshot taken in the field, English); skipped
//! otherwise.

use std::path::PathBuf;
use std::sync::Arc;

use ffx_oracle::{Conv, Oracle, Pages};

/// `MsGetRomItem` (`0x007909f0`): the kernel record of an item ID.
const MS_GET_ROM_ITEM: u32 = 0x0079_09f0;

fn setup() -> Option<(PathBuf, Arc<Pages>)> {
    let (Some(dir), Some(snap)) = (
        std::env::var_os("FFX_GAME_DIR"),
        std::env::var_os("FFX_SNAPSHOT"),
    ) else {
        eprintln!("FFX_GAME_DIR or FFX_SNAPSHOT is not set; skipping");
        return None;
    };
    Some((
        PathBuf::from(&dir).join("FFX.exe"),
        Pages::read(PathBuf::from(snap)).expect("snapshot"),
    ))
}

/// A file as the game loads it: from the last mod that has it (mods load in
/// name order, a later one winning), else from the archive.
fn game_file(exe: &std::path::Path, path: &str) -> Vec<u8> {
    let mods = exe.with_file_name("mods");
    let mut dirs: Vec<_> = std::fs::read_dir(&mods)
        .map(|d| d.filter_map(Result::ok).map(|e| e.path()).collect())
        .unwrap_or_default();
    dirs.sort();
    if let Some(f) = dirs
        .iter()
        .rev()
        .map(|d| d.join(path))
        .find(|f| f.is_file())
    {
        return std::fs::read(f).unwrap();
    }
    let archive = ffx_vbf::Archive::open(exe.with_file_name("data/FFX_Data.vbf")).unwrap();
    archive.read(archive.find(path).unwrap()).unwrap()
}

#[test]
fn snapshot_holds_the_loaded_item_table() {
    let Some((exe, pages)) = setup() else { return };
    let mut o = Oracle::from_snapshot(&exe, &pages).unwrap();
    assert_eq!(o.image_base(), pages.snapshot().image_base);
    // the game thread's control word came along
    assert_eq!(
        pages.snapshot().fpu_control & 0x0300,
        0x0200,
        "53-bit precision"
    );

    let path = ffx_kernel::Schema::by_file("item")
        .unwrap()
        .path(ffx_text::Language::English);
    let table = ffx_kernel::Table::parse(&game_file(&exe, &path)).unwrap();

    for id in [0u16, 1, 0x40, 0x6f] {
        let at = o
            .call(
                o.rebase(MS_GET_ROM_ITEM),
                Conv::Cdecl,
                &[0x2000 | u32::from(id), 0],
            )
            .unwrap()
            .eax;
        let want = table.record(id).unwrap();
        let got = o.read(at, want.len()).unwrap();
        // the string offsets (first fields) are rewritten when the game
        // loads the table; compare what follows them
        assert_eq!(got[8..], want[8..], "item {id:#x} at {at:#x}");
    }
}

#[test]
fn two_runs_on_one_snapshot_write_the_same() {
    let Some((exe, pages)) = setup() else { return };
    let mut a = Oracle::from_snapshot(&exe, &pages).unwrap();
    let mut b = Oracle::from_snapshot(&exe, &pages).unwrap();
    for o in [&mut a, &mut b] {
        let f = o.rebase(MS_GET_ROM_ITEM);
        o.call(f, Conv::Cdecl, &[0x2001, 0]).unwrap();
    }
    assert_eq!(a.diff(&mut b).unwrap(), []);
}
