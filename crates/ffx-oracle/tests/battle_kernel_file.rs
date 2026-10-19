//! The kernel file reader (`ffx_battle::init`, round 23:
//! `getExcelBinSize`) against the original, on the image alone and on
//! both snapshots. The language, file layer and heap callees are logging
//! stubs in both oracles (`TOGetFFXLang` the language, `sceOpen` a handle
//! or 0, `sceLseek` the size, `TkMalloc` a block of random bytes); the
//! original's `sprintf` and `memcpy` run as the game's. The path the
//! original builds on its stack is compared with the port's (in scratch,
//! as the hook's frame), the records of the calls that get it made alike.
//! Names stay short enough for the original's 128-byte path.

// a and b are the two oracles throughout
#![allow(clippy::many_single_char_names)]

mod battle_common;

use battle_common::*;
use ffx_battle::init::bridge as ini;
use ffx_oracle::{Conv, Oracle};

/// `Ms_Gui_BattleChr2Proc` (`0x007de790`), code no test runs: the stubs.
const GUI_STUBS: u32 = 0x007d_e790;

/// The stubbed callees and their argument counts.
const CALLEES: [(u32, u8); 8] = [
    (ini::TO_GET_FFX_LANG_FN, 0),
    (ini::SCE_OPEN, 2),
    (ini::SCE_LSEEK, 3),
    (ini::SCE_READ, 3),
    (ini::SCE_CLOSE, 1),
    (ini::TK_MALLOC, 1),
    (ini::TK_FREE, 1),
    (ini::DBG_PRINTF, 2),
];
const LANG: u32 = 0;
const OPEN: u32 = 1;
const LSEEK: u32 = 2;
const MALLOC: u32 = 5;
const PRINTF: u32 = 7;
const REC: u32 = 0x100;

fn game_call(m: &mut Oracle, f: u32, args: &[u32]) -> u32 {
    m.call(f, Conv::Cdecl, args)
        .unwrap_or_else(|e| panic!("{f:#x} {args:x?}: {e}"))
        .eax
}

fn word(o: &Oracle, at: u32) -> u32 {
    ffx_bridge::Mem::read(o, at)
}

fn check(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    let recs = scratch(a, b, (REC as usize) * CALLEES.len());
    let rec = |k: u32| recs + REC * k;
    for o in [&mut *a, &mut *b] {
        for (k, &(f, n)) in (0u32..).zip(&CALLEES) {
            logging_stub(o, f, GUI_STUBS + 0x40 * k, rec(k), n, 0);
        }
    }
    let name = scratch(a, b, 0x60);
    let path = scratch(a, b, ini::KERNEL_PATH_SIZE as usize);
    let block = scratch(a, b, 0x400);
    let buffer = scratch(a, b, 0x400);
    for _ in 0..1500 {
        let n = rng.below(0x50) as usize;
        let mut s: Vec<u8> = (0..n)
            .map(|_| b"abcdefghijklmnopqrstuvwxyz_0123456789"[rng.below(37) as usize])
            .collect();
        s.push(0);
        poke(a, b, name, &s);
        fill(a, b, rng, block, 0x400);
        let lang = [0, 1, 2, 3, 4, 5, 6, 9, 10, 11, rng.next()][rng.below(11) as usize];
        let fd = if rng.below(5) == 0 { 0 } else { rng.next() | 1 };
        let size = [
            0,
            1,
            15,
            16,
            0x3f1,
            rng.below(0x3f2),
            (-15i32).cast_unsigned(),
        ][rng.below(7) as usize];
        for o in [&mut *a, &mut *b] {
            for k in 0..u32::try_from(CALLEES.len()).unwrap() {
                o.write(rec(k), &[0; 4]).unwrap();
            }
            o.write(rec(LANG) + 4, &lang.to_le_bytes()).unwrap();
            o.write(rec(OPEN) + 4, &fd.to_le_bytes()).unwrap();
            o.write(rec(LSEEK) + 4, &size.to_le_bytes()).unwrap();
            o.write(rec(MALLOC) + 4, &block.to_le_bytes()).unwrap();
        }
        twin(
            "getExcelBinSize",
            a,
            b,
            0x0078_2df0,
            &[name, buffer],
            ALL,
            |m| {
                let mut call = game_call;
                ini::get_excel_bin_size(m, &mut call, name, buffer, path)
            },
        );
        // the path: the original's on its stack, the port's in `path`
        let theirs = word(a, rec(OPEN) + 8);
        let len = ini::KERNEL_PATH_SIZE as usize;
        let built = a.read(theirs, len).unwrap();
        let ours = b.read(path, len).unwrap();
        let end = built.iter().position(|&c| c == 0).unwrap();
        assert_eq!(built[..=end], ours[..=end], "getExcelBinSize: path");
        poke_one(b, path, &vec![0; len]);
        poke_one(b, rec(OPEN) + 8, &theirs.to_le_bytes());
        if fd == 0 {
            let p = word(a, rec(PRINTF) + 0xc);
            poke_one(b, rec(PRINTF) + 0xc, &p.to_le_bytes());
        }
        same("getExcelBinSize", a, b);
    }
}

fn poke_one(o: &mut Oracle, at: u32, bytes: &[u8]) {
    o.write(at, bytes).unwrap();
}

#[test]
fn kernel_files_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x23b1_0001));
}

#[test]
fn kernel_files_match_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x23b1_0002));
}
