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

/// The string functions (`_stricmp`, `strstr`, `strchr`, `strncpy`,
/// `memchr`) through the import against the DLL's exports: `eax` and the
/// bytes written, on strings with case, high bytes, empty ones and
/// matches at the start, the end and nowhere.
#[test]
fn string_functions_match_the_dll() {
    let Some(exe) = exe() else { return };
    let mut e = ffx_host::boot(&exe).unwrap();
    let strings: [&[u8]; 9] = [
        b"",
        b"a",
        b"Hello World",
        b"hello world",
        b"HELLO",
        b"world",
        b"\xe9t\xc9\x80z",
        b"abcabcabd",
        b"abd",
    ];
    let put = |e: &mut Exec, s: &[u8]| -> u32 {
        let at = e.alloc(u32::try_from(s.len()).unwrap() + 16);
        let mut v = s.to_vec();
        v.extend_from_slice(&[0, 0x5a, 0x5a, 0x5a]);
        e.mem().write_bytes(at, &v);
        at
    };
    let at: Vec<u32> = strings.iter().map(|s| put(&mut e, s)).collect();
    let call = |e: &mut Exec, f: u32, words: &[u32]| -> u32 { e.call_cdecl(f, words).unwrap() };
    let both = |e: &Exec, name: &str| -> (u32, u32) {
        (
            e.mem().read(slot(e, "MSVCR110.dll", name)),
            export(e, "MSVCR110.dll", name),
        )
    };
    let mut checked = 0;
    for name in ["_stricmp", "strstr"] {
        let (rust, dll) = both(&e, name);
        for &x in &at {
            for &y in &at {
                assert_eq!(
                    call(&mut e, rust, &[x, y]),
                    call(&mut e, dll, &[x, y]),
                    "{name}"
                );
                checked += 1;
            }
        }
    }
    let (rust, dll) = both(&e, "strchr");
    for &x in &at {
        for c in [
            0u32,
            u32::from(b'o'),
            u32::from(b'H'),
            0xc9,
            0x1_0000 + u32::from(b'l'),
        ] {
            assert_eq!(
                call(&mut e, rust, &[x, c]),
                call(&mut e, dll, &[x, c]),
                "strchr {c:#x}"
            );
        }
    }
    let (rust, dll) = both(&e, "memchr");
    for &x in &at {
        for (c, n) in [
            (0u32, 4u32),
            (u32::from(b'o'), 11),
            (0x15a, 20),
            (u32::from(b'd'), 0),
        ] {
            assert_eq!(
                call(&mut e, rust, &[x, c, n]),
                call(&mut e, dll, &[x, c, n]),
                "memchr"
            );
        }
    }
    let (rust, dll) = both(&e, "strncpy");
    for &x in &at {
        for n in [0u32, 1, 3, 11, 24] {
            let mut out = Vec::new();
            for f in [rust, dll] {
                let dst = e.alloc(40);
                e.mem().write_bytes(dst, &[0xee; 40]);
                let r = call(&mut e, f, &[dst, x, n]);
                let mut b = vec![0u8; 40];
                e.mem().read_bytes(dst, &mut b);
                out.push((r - dst, b));
            }
            assert_eq!(out[0], out[1], "strncpy {n}");
        }
    }
    assert!(checked > 100);
}

/// Words of a `printf` call's variable part for one conversion.
fn conv_args(conv: &str, size: &str, v: u64, strings: &[u32]) -> Vec<u32> {
    #[allow(clippy::cast_possible_truncation)] // the words the caller pushes
    match conv {
        "s" => vec![strings[(v % strings.len() as u64) as usize]],
        _ if size == "ll" || size == "I64" => vec![v as u32, (v >> 32) as u32],
        _ => vec![v as u32],
    }
}

/// The `sprintf` family's Rust sides (`ffx_host::crt_format`) against the
/// DLL's exports, through the imports: `eax` and the buffer (with junk
/// after the output) over every integer, character and string conversion
/// with flag sets, widths and precisions (digits and `*`, negative ones),
/// sizes, values at the edges and NULL strings; the Rust side must take
/// them without the emulator. Floats, wide strings, positional and
/// unknown conversions go to the DLL with the caller's words: compared
/// too. `sprintf_s`, `_snprintf`, `vsprintf`, `_vsnprintf` and
/// `_vsnprintf_s` at the size limits (fitting, exactly full, short).
#[test]
#[allow(clippy::too_many_lines, clippy::many_single_char_names)]
fn sprintf_family_matches_the_dll() {
    let Some(exe) = exe() else { return };
    let mut e = ffx_host::boot(&exe).unwrap();
    let put = |e: &mut Exec, s: &[u8]| -> u32 {
        let at = e.alloc(u32::try_from(s.len()).unwrap() + 16);
        let mut v = s.to_vec();
        v.extend_from_slice(&[0, 0x5a, 0x5a, 0x5a]);
        e.mem().write_bytes(at, &v);
        at
    };
    let strings = [
        0,
        put(&mut e, b""),
        put(&mut e, b"abc"),
        put(&mut e, b"Hello, World \xe9"),
    ];
    let buf = e.alloc(0x400);
    let both = |e: &Exec, name: &str| -> (u32, u32) {
        (
            e.mem().read(slot(e, "MSVCR110.dll", name)),
            export(e, "MSVCR110.dll", name),
        )
    };
    // one call each way: (eax, the buffer)
    let run = |e: &mut Exec, f: u32, words: &[u32]| -> (u32, Vec<u8>) {
        e.mem().write_bytes(buf, &[0xee; 0x400]);
        let r = e.call_cdecl(f, words).unwrap();
        let mut b = vec![0u8; 0x400];
        e.mem().read_bytes(buf, &mut b);
        (r, b)
    };
    let (sprintf, sprintf_dll) = both(&e, "sprintf");
    let values: [u64; 12] = [
        0,
        1,
        42,
        0xffff_ffff,
        0x8000_0000,
        0x7fff_ffff,
        0xffff_8000,
        0x0001_2345,
        0xffff_ff85,
        0x8000_0000_0000_0000,
        0xffff_ffff_ffff_fffe,
        0x0000_0012_3456_789a,
    ];
    let mut fast = 0;
    let mut checked = 0;
    let flagsets = ["", "-", "+", " ", "#", "0", "-0", "+0", "#0", " -", "+ #0"];
    let widths = ["", "1", "6", "13", "*"];
    let precisions = ["", ".", ".0", ".2", ".9", ".*"];
    let fmt_at = e.alloc(64);
    let mut k = 0usize;
    for conv in ["d", "i", "u", "o", "x", "X", "c", "s", "p"] {
        let sizes: &[&str] = match conv {
            "c" | "s" => &["", "h"],
            "p" => &[""],
            _ => &["", "h", "l", "ll", "I64", "I32", "I"],
        };
        for size in sizes {
            for flags in flagsets {
                for width in widths {
                    for precision in precisions {
                        k += 1;
                        let v = values[(k * 7) % values.len()];
                        let fmt = format!("[%{flags}{width}{precision}{size}{conv}]");
                        let mut cfmt = fmt.clone().into_bytes();
                        cfmt.push(0);
                        e.mem().write_bytes(fmt_at, &cfmt);
                        let mut words = vec![buf, fmt_at];
                        if width == "*" {
                            words.push([7u32, (-9i32).cast_unsigned(), 0][k % 3]);
                        }
                        if precision == ".*" {
                            words.push([3u32, (-1i32).cast_unsigned(), 0][k % 3]);
                        }
                        words.extend(conv_args(conv, size, v, &strings));
                        let before = e.stats.unicorn_calls;
                        let rust = run(&mut e, sprintf, &words);
                        if e.stats.unicorn_calls == before {
                            fast += 1;
                        }
                        let dll = run(&mut e, sprintf_dll, &words);
                        assert_eq!(rust, dll, "sprintf {fmt} {v:#x}");
                        checked += 1;
                    }
                }
            }
        }
    }
    assert!(fast == checked, "{fast} of {checked} in Rust");
    // what the Rust side hands on, with words after the declared ones
    let wide = e.alloc(16);
    e.mem().write_bytes(wide, &[b'w', 0, b'i', 0, 0, 0]);
    let pi = std::f64::consts::PI.to_bits();
    #[allow(clippy::cast_possible_truncation)]
    let (lo, hi) = (pi as u32, (pi >> 32) as u32);
    let s = strings[3];
    for (fmt, words) in [
        ("%f|%d|%s", vec![lo, hi, 7, s]),
        ("%-9.3e%5s", vec![lo, hi, s]),
        ("%g", vec![lo, hi]),
        ("%ls|%S", vec![wide, wide]),
        ("%lc%d", vec![0x41, 5]),
        ("%5%|%d", vec![3]),
        ("%k%d", vec![3]),
        ("%I%d", vec![3]),
        ("%hhd", vec![0x1ff]),
        ("%zd", vec![4]),
    ] {
        let mut c = fmt.as_bytes().to_vec();
        c.push(0);
        e.mem().write_bytes(fmt_at, &c);
        let mut w = vec![buf, fmt_at];
        w.extend(words);
        let rust = run(&mut e, sprintf, &w);
        let dll = run(&mut e, sprintf_dll, &w);
        assert_eq!(rust, dll, "sprintf {fmt} (handed on)");
    }
    // the sized ones at their limits: "x=%d" of 12345 is 7 characters
    let f = put(&mut e, b"x=%d");
    for name in ["sprintf_s", "_snprintf"] {
        let (rust, dll) = both(&e, name);
        let sizes: &[u32] = if name == "sprintf_s" {
            &[8, 9, 100]
        } else {
            &[1, 6, 7, 8, 100]
        };
        for &n in sizes {
            let w = [buf, n, f, 12345];
            assert_eq!(run(&mut e, rust, &w), run(&mut e, dll, &w), "{name} {n}");
        }
    }
    let va = e.alloc(16);
    e.mem().write(va, 12345u32);
    let (rust, dll) = both(&e, "vsprintf");
    let w = [buf, f, va];
    assert_eq!(run(&mut e, rust, &w), run(&mut e, dll, &w), "vsprintf");
    let (rust, dll) = both(&e, "_vsnprintf");
    for n in [1, 6, 7, 8, 100] {
        let w = [buf, n, f, va];
        assert_eq!(
            run(&mut e, rust, &w),
            run(&mut e, dll, &w),
            "_vsnprintf {n}"
        );
    }
    let (rust, dll) = both(&e, "_vsnprintf_s");
    for (size, count) in [
        (100, u32::MAX),
        (8, u32::MAX),
        (7, u32::MAX),
        (3, u32::MAX),
        (100, 7),
        (100, 8),
        (100, 5),
        (8, 8),
        (8, 100),
        (20, 10),
    ] {
        let w = [buf, size, count, f, va];
        assert_eq!(
            run(&mut e, rust, &w),
            run(&mut e, dll, &w),
            "_vsnprintf_s {size} {count}"
        );
    }
    assert!(checked > 3000, "{checked}");
}
