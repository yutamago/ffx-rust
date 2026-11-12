//! The C runtime DLLs' Rust sides (`ffx_host::crt_dll`) against the
//! player's DLLs in Unicorn: each called as `FFX.exe` and the ports call
//! it, and the DLL's own export called directly. Needs `FFX_GAME_DIR`;
//! skips without it.

use std::path::PathBuf;

use ffx_bridge::Mem as _;
use ffx_bridge::route::{Float, Router as _};
use ffx_host::exec::Exec;

fn exe() -> Option<PathBuf> {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return None;
    };
    Some(PathBuf::from(dir).join("FFX.exe"))
}

fn export(exec: &Exec, dll: &str, name: &str) -> u32 {
    exec.shared().dll(dll).unwrap().exports[name]
}

/// The IAT slot of `dll!name` in the image.
fn slot(exec: &Exec, dll: &str, name: &str) -> u32 {
    exec.image()
        .imports
        .iter()
        .find(|i| i.dll.eq_ignore_ascii_case(dll) && i.name == name)
        .unwrap_or_else(|| panic!("{dll}!{name} not imported"))
        .slot
}

/// `std::cin`'s `getline(buffer, 99)` called through the import (the
/// Rust side, as the auto-test console thread calls it) and through the
/// DLL's export: twice each on a fresh run, the stream at its end of
/// input after the first. `eax`, the buffer and the whole stream object
/// (the `istream` part and its `basic_ios`) must agree.
#[test]
fn getline_on_a_stream_at_its_end_matches_the_dll() {
    const CIN_SLOT: u32 = 0x00b0_c200;
    let Some(exe) = exe() else { return };
    let mut got = Vec::new();
    for via_import in [false, true] {
        let mut e = ffx_host::boot(&exe).unwrap();
        let cin: u32 = e.mem().read(CIN_SLOT);
        let at = if via_import {
            e.mem()
                .read(slot(&e, "MSVCP110.dll", ffx_host::crt_dll::GETLINE))
        } else {
            export(&e, "MSVCP110.dll", ffx_host::crt_dll::GETLINE)
        };
        let buffer = e.alloc(100);
        let mut runs = Vec::new();
        for n in 0..3 {
            let emulated = e.stats.unicorn_calls;
            e.mem().write_bytes(buffer, &[0x55; 100]);
            let args = ffx_host::table::Args {
                ecx: cin,
                stack: vec![buffer, 99, 0],
                ..Default::default()
            };
            let r = e.call(at, ffx_host::table::Conv::Thiscall, &args).unwrap();
            if via_import && n > 0 {
                // the stream at its end: the Rust side, no DLL
                assert_eq!(e.stats.unicorn_calls, emulated, "call {n}");
            }
            let mut object = vec![0u8; 0x60];
            e.mem().read_bytes(cin, &mut object);
            let mut text = vec![0u8; 100];
            e.mem().read_bytes(buffer, &mut text);
            runs.push((r.eax, object, text));
        }
        got.push(runs);
    }
    assert_eq!(got[0], got[1]);
    // the reads found nothing: the buffer an empty string each time
    for (_, _, text) in &got[1] {
        assert_eq!(text[0], 0);
    }
}

/// The x87 intrinsics: `_CIsin`, `_CIcos` (SSE2 paths) and `_CIsqrt`
/// through the ports' thunks (`call_x87`, as `ffx_chara`'s `CI_COS`), and
/// from emulated code through the IAT, against the DLL's exports, bit for
/// bit, over arguments in every band (tiny, the table's range, the large
/// ones the DLL computes with x87 `fcos`, signs, zeros, infinities).
#[test]
#[allow(clippy::cast_possible_truncation)] // the game's f32 angles
fn x87_intrinsics_match_the_dll() {
    let Some(exe) = exe() else { return };
    let mut e = ffx_host::boot(&exe).unwrap();
    let mut xs = vec![
        0.0,
        -0.0,
        1e-300,
        -1e-310,
        5e-324,
        1e-80,
        1e-20,
        0.5,
        1.0,
        -1.0,
        std::f64::consts::PI,
        std::f64::consts::FRAC_PI_2,
        -std::f64::consts::FRAC_PI_4,
        100.0,
        -12_345.678,
        1e8,
        3e8,
        1e9,
        1e15,
        1e300,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::MAX,
        2.0,
        4.0,
    ];
    // the angles the game feeds them (f32 radians) and a spread
    let mut s = 0x1234_5678u32;
    for i in 0..4000 {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        let unit = f64::from(s) / f64::from(u32::MAX);
        let x = match i % 4 {
            0 => (unit - 0.5) * 20.0,
            1 => f64::from(((unit - 0.5) * 7.0) as f32),
            2 => (unit - 0.5) * 2e6,
            _ => unit * 1e-6,
        };
        xs.push(x);
    }
    // thunks the ports call (`jmp [slot]`): _CIsin, _CIcos, _CIsqrt
    for (name, thunk) in [
        ("_CIsin", 0x0094_98c8u32),
        ("_CIcos", 0x0094_98c2),
        ("_CIsqrt", 0x0094_97b8),
    ] {
        let dll = export(&e, "MSVCR110.dll", name);
        let iat = slot(&e, "MSVCR110.dll", name);
        // emulated code: fld qword [cell]; call [iat]; fstp qword [out]; ret
        let cell = e.alloc(16);
        let code = {
            let mut c = vec![0xdd, 0x05];
            c.extend_from_slice(&cell.to_le_bytes());
            c.extend_from_slice(&[0xff, 0x15]);
            c.extend_from_slice(&iat.to_le_bytes());
            c.extend_from_slice(&[0xdd, 0x1d]);
            c.extend_from_slice(&(cell + 8).to_le_bytes());
            c.push(0xc3);
            c
        };
        let caller = e.alloc_code(&code).unwrap();
        let mut fast = 0;
        for &x in &xs {
            if name == "_CIsqrt" && x < 0.0 {
                continue; // the DLL's error path (errno, the matherr hook)
            }
            let before = e.stats.leaf_calls;
            let rust = e.call_x87(thunk, &[x], Float::F64);
            fast += e.stats.leaf_calls - before;
            let want = e.call_x87(dll, &[x], Float::F64);
            assert!(
                rust.to_bits() == want.to_bits() || (rust.is_nan() && want.is_nan()),
                "{name}({x:e}) = {rust:e}, the DLL's {want:e}"
            );
            e.mem().write(cell, x);
            e.call_cdecl(caller, &[]).unwrap();
            let from_unicorn: f64 = e.mem().read(cell + 8);
            assert!(
                from_unicorn.to_bits() == want.to_bits()
                    || (from_unicorn.is_nan() && want.is_nan()),
                "{name}({x:e}) from emulated code = {from_unicorn:e}, the DLL's {want:e}"
            );
        }
        // the SSE2 paths took Rust for |x| under about 2^16 (all but the
        // quarter around 10^6 and the large ones), sqrt for the
        // non-negative arguments (about 2,500 here)
        let floor = if name == "_CIsqrt" { 2400 } else { 2900 };
        assert!(fast > floor, "{name}: {fast} in Rust");
    }
}
