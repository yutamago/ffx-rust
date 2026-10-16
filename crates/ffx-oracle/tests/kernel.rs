//! Kernel table lookups (`ffx_kernel::bridge`) against the original, on a
//! snapshot of the running game (the tables are loaded there). Needs
//! `FFX_GAME_DIR` and `FFX_SNAPSHOT`; skipped otherwise.

use std::path::PathBuf;

use ffx_bridge::Global;
use ffx_kernel::bridge as k;
use ffx_oracle::{Conv, Oracle, Pages};

const MS_GET_EXCEL_DATA: u32 = 0x007a_b870;
const MS_GET_EXCEL_DATA_NUM: u32 = 0x007a_b8f0;
const MS_CHECK_RANGE: u32 = 0x0079_a0c0;
const MS_GET_ROM_CTB_BASE: u32 = 0x0079_0990;
const MS_GET_ROM_CTB_COUNT: u32 = 0x0079_09c0;
const MS_GET_ROM_KAIZOU: u32 = 0x0079_0a10;
const MS_GET_ROM_SUMMON_GROW: u32 = 0x0079_0ab0;

/// (original, table) for the `id & 0xfff` lookups.
const ROMS: [(u32, Global<u32>); 5] = [
    (0x0079_0970, k::A_ABILITY),  // MsGetRomAbility
    (0x0079_09f0, k::ITEM),       // MsGetRomItem
    (0x0079_0a50, k::MON_MAGIC1), // MonMagic1
    (0x0079_0a70, k::MON_MAGIC2), // MsGetRomMonCommand2
    (0x0079_0a90, k::COMMAND),    // MsGetRomPlyCommand
];

fn setup() -> Option<(Oracle, Oracle)> {
    let (Some(dir), Some(snap)) = (
        std::env::var_os("FFX_GAME_DIR"),
        std::env::var_os("FFX_SNAPSHOT"),
    ) else {
        eprintln!("FFX_GAME_DIR or FFX_SNAPSHOT is not set; skipping");
        return None;
    };
    let exe = PathBuf::from(dir).join("FFX.exe");
    let pages = Pages::read(PathBuf::from(snap)).unwrap();
    Some((
        Oracle::from_snapshot(&exe, &pages).unwrap(),
        Oracle::from_snapshot(&exe, &pages).unwrap(),
    ))
}

/// A scratch word in emulated memory both oracles allocate alike.
fn scratch(a: &mut Oracle, b: &mut Oracle) -> u32 {
    let x = a.alloc(4).unwrap();
    assert_eq!(b.alloc(4).unwrap(), x);
    x
}

fn ids() -> Vec<i32> {
    let mut v: Vec<i32> = (-3..0x140).collect();
    v.extend([0x1000, 0x2000, 0x2001, 0x6000, 0x7fff, 0xffff, 0x1_0000]);
    v
}

#[test]
fn lookups_match_the_original() {
    let Some((mut a, mut b)) = setup() else {
        return;
    };
    let out = scratch(&mut a, &mut b);
    for &(f, table) in &ROMS {
        for id in ids() {
            #[allow(clippy::cast_sign_loss)]
            let id = id as u32;
            for strings in [0, out] {
                let ra = a
                    .call(a.rebase(f), Conv::Cdecl, &[id, strings])
                    .unwrap()
                    .eax;
                let rb = k::rom(&mut b, table, id, strings);
                assert_eq!(ra, rb, "{f:#x} id {id:#x}");
            }
        }
        assert_eq!(a.diff(&mut b).unwrap(), [], "{f:#x}: memory");
    }
    // MsGetExcelData itself, on every table, signed IDs
    for &(_, table) in &ROMS {
        let t = table.get(&b);
        for id in ids() {
            #[allow(clippy::cast_sign_loss)]
            let raw = id as u32;
            let ra = a
                .call(a.rebase(MS_GET_EXCEL_DATA), Conv::Cdecl, &[raw, t, out])
                .unwrap()
                .eax;
            let rb = k::ms_get_excel_data(&mut b, id, t, out);
            assert_eq!(ra, rb, "MsGetExcelData {id} in {t:#x}");
        }
        for seg in 0..4 {
            let ra = a
                .call(a.rebase(MS_GET_EXCEL_DATA_NUM), Conv::Cdecl, &[t, seg])
                .unwrap()
                .eax;
            #[allow(clippy::cast_possible_wrap)]
            let rb = k::ms_get_excel_data_num(&b, t, seg as i32);
            #[allow(clippy::cast_sign_loss)]
            let rb = rb as u32;
            assert_eq!(ra, rb, "MsGetExcelDataNum seg {seg}");
        }
    }
    assert_eq!(a.diff(&mut b).unwrap(), [], "MsGetExcelData: memory");
}

#[test]
fn battle_tables_match_the_original() {
    let Some((mut a, mut b)) = setup() else {
        return;
    };
    let out = scratch(&mut a, &mut b);
    for (x, lo, hi) in [(0, 1, 255), (300, 1, 255), (-5, -10, 10), (7, 0, 3)] {
        #[allow(clippy::cast_sign_loss)]
        let args = [x as u32, lo as u32, hi as u32];
        let ra = a
            .call(a.rebase(MS_CHECK_RANGE), Conv::Cdecl, &args)
            .unwrap()
            .eax;
        #[allow(clippy::cast_sign_loss)]
        let rb = k::ms_check_range(x, lo, hi) as u32;
        assert_eq!(ra, rb, "MsCheckRange({x}, {lo}, {hi})");
    }
    // the CTB table is loaded only in battle; skip when the snapshot has none
    if k::CTB_BASE.get(&b) != 0 {
        for n in [-1i32, 0, 1, 2, 50, 255, 256, 1000] {
            #[allow(clippy::cast_sign_loss)]
            let arg = n as u32;
            let ra = a
                .call(a.rebase(MS_GET_ROM_CTB_BASE), Conv::Cdecl, &[arg])
                .unwrap()
                .eax;
            assert_eq!(ra, k::ms_get_rom_ctb_base(&mut b, n), "CTBbase {n}");
            let ra = a
                .call(a.rebase(MS_GET_ROM_CTB_COUNT), Conv::Cdecl, &[arg])
                .unwrap()
                .eax
                & 0xff;
            assert_eq!(
                ra,
                u32::from(k::ms_get_rom_ctb_count(&mut b, n)),
                "CTBcount {n}"
            );
        }
    }
    for (f, table) in [
        (MS_GET_ROM_KAIZOU, k::KAIZOU),
        (MS_GET_ROM_SUMMON_GROW, k::SUMMON_GROW),
    ] {
        if table.get(&b) == 0 {
            continue;
        }
        for count in [0, out] {
            let ra = a.call(a.rebase(f), Conv::Cdecl, &[count]).unwrap().eax;
            assert_eq!(ra, k::first_record(&mut b, table, count), "{f:#x}");
        }
    }
    assert_eq!(a.diff(&mut b).unwrap(), [], "memory");
    let _ = b.image_base();
}
