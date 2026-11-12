//! The C runtime DLLs' Rust sides (`ffx_host::crt_dll`) against the
//! player's DLLs in Unicorn: each called as `FFX.exe` and the ports call
//! it, and the DLL's own export called directly. Needs `FFX_GAME_DIR`;
//! skips without it.

use std::path::PathBuf;

use ffx_bridge::Mem as _;
use ffx_bridge::route::Router as _;
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

/// A xorshift stream of unit values for the argument spreads.
struct Spread(u32);

impl Spread {
    fn unit(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        f64::from(self.0) / f64::from(u32::MAX)
    }
}

/// Edge arguments every one-operand intrinsic gets: tiny, normal, the
/// band edges of the SSE2 paths, large, signs, zeros, infinities, a NaN.
const EDGES: [f64; 30] = [
    0.0,
    -0.0,
    1e-300,
    -1e-310,
    5e-324,
    1e-80,
    1e-20,
    1e-9,
    0.5,
    1.0,
    -1.0,
    std::f64::consts::PI,
    std::f64::consts::FRAC_PI_2,
    -std::f64::consts::FRAC_PI_4,
    100.0,
    -12_345.678,
    8191.9,
    8192.1,
    1e8,
    3e8,
    1e9,
    1e15,
    1e19,
    1e300,
    f64::INFINITY,
    f64::NEG_INFINITY,
    f64::MAX,
    f64::NAN,
    2.0,
    4.0,
];

/// One-operand arguments: the edges and `n` from a spread (the game's
/// f32 radians, a wide range, tiny ones, near the poles of tan).
#[allow(clippy::cast_possible_truncation)] // the game's f32 angles
fn one_operand(n: usize, seed: u32) -> Vec<Vec<f64>> {
    let mut s = Spread(seed);
    let mut xs: Vec<f64> = EDGES.to_vec();
    for i in 0..n {
        let u = s.unit();
        xs.push(match i % 6 {
            0 => (u - 0.5) * 20.0,
            1 => f64::from(((u - 0.5) * 7.0) as f32),
            2 => (u - 0.5) * 2e6,
            3 => u * 1e-6,
            4 => std::f64::consts::FRAC_PI_2 * f64::from((u * 40.0) as i32 - 20) + (u - 0.5) * 1e-9,
            _ => (u - 0.5) * 2.0,
        });
    }
    xs.into_iter().map(|x| vec![x]).collect()
}

/// Two-operand arguments: every pair of a set of edges (zeros,
/// denormals, infinities, NaNs, signs) and `n` spread pairs.
#[allow(clippy::cast_possible_truncation)]
fn two_operands(n: usize, seed: u32) -> Vec<Vec<f64>> {
    let edges = [
        0.0,
        -0.0,
        1e-310,
        -5e-324,
        1e-300,
        1.0,
        -1.0,
        0.5,
        3.0,
        -7.25,
        1e300,
        -1e300,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        -f64::NAN,
    ];
    let mut out = Vec::new();
    for &a in &edges {
        for &b in &edges {
            out.push(vec![a, b]);
        }
    }
    let mut s = Spread(seed);
    for i in 0..n {
        let (u, v) = (s.unit(), s.unit());
        out.push(match i % 4 {
            0 => vec![(u - 0.5) * 20.0, (v - 0.5) * 20.0],
            1 => vec![
                f64::from(((u - 0.5) * 7.0) as f32),
                f64::from(((v - 0.5) * 7.0) as f32),
            ],
            2 => vec![(u - 0.5) * 1e6, (v - 0.5) * 1e-3],
            _ => vec![(u - 0.5) * 1e-3, (v - 0.5) * 1e5],
        });
    }
    out
}

/// The bits a result stores, NaNs compared by their bits too.
fn same(a: f64, b: f64) -> bool {
    a.to_bits() == b.to_bits()
}

/// The x87 intrinsics through the ports' thunks (`call_x87`, as
/// `ffx_chara`'s `CI_COS`) and from emulated code through the IAT,
/// against the DLL's exports, bit for bit, stored as doubles and (one
/// rounding from the 80 bits) as floats: `_CIsin`, `_CIcos`, `_CItan`
/// (SSE2 paths and the x87 ones for large arguments), `_CIsqrt`,
/// `_CIatan2` and `_CIfmod` (x87, two operands). `min_rust` is how many
/// calls each must take in Rust.
#[test]
#[allow(clippy::too_many_lines)]
fn x87_intrinsics_match_the_dll() {
    use ffx_bridge::route::Float;
    let Some(exe) = exe() else { return };
    let mut e = ffx_host::boot(&exe).unwrap();
    let cases: [(&str, u32, Vec<Vec<f64>>, usize); 6] = [
        ("_CIsin", 0x0094_98c8, one_operand(3000, 0x1234_5678), 2200),
        ("_CIcos", 0x0094_98c2, one_operand(3000, 0x2345_6789), 2200),
        ("_CItan", 0x0094_97be, one_operand(3000, 0x3456_789a), 2900),
        ("_CIsqrt", 0x0094_97b8, one_operand(3000, 0x4567_89ab), 1500),
        (
            "_CIatan2",
            0x0094_98bc,
            two_operands(3000, 0x5678_9abc),
            3100,
        ),
        (
            "_CIfmod",
            0x0094_98e0,
            two_operands(3000, 0x6789_abcd),
            3100,
        ),
    ];
    for (name, thunk, args, min_rust) in cases {
        let dll = export(&e, "MSVCR110.dll", name);
        let iat = slot(&e, "MSVCR110.dll", name);
        let n = args[0].len();
        // emulated code: fld qword [cell + 8i] for each; call [iat];
        // fstp qword [out]; ret
        let cell = e.alloc(32);
        let out = cell + 16;
        let caller = {
            let mut c = Vec::new();
            for i in 0..u32::try_from(n).unwrap() {
                c.extend_from_slice(&[0xdd, 0x05]);
                c.extend_from_slice(&(cell + 8 * i).to_le_bytes());
            }
            c.extend_from_slice(&[0xff, 0x15]);
            c.extend_from_slice(&iat.to_le_bytes());
            c.extend_from_slice(&[0xdd, 0x1d]);
            c.extend_from_slice(&out.to_le_bytes());
            c.push(0xc3);
            e.alloc_code(&c).unwrap()
        };
        let mut fast = 0;
        for st in &args {
            if name == "_CIsqrt" && !(st[0] >= 0.0) {
                continue; // the DLL's error path (errno, the matherr hook)
            }
            for float in [Float::F64, Float::F32] {
                let before = e.stats.leaf_calls;
                let rust = e.call_x87(thunk, st, float);
                if float == Float::F64 {
                    fast += e.stats.leaf_calls - before;
                }
                let want = e.call_x87(dll, st, float);
                assert!(
                    same(rust, want),
                    "{name}({st:?}) as {float:?} = {rust:e} ({:#x}), the DLL's {want:e} ({:#x})",
                    rust.to_bits(),
                    want.to_bits()
                );
            }
            let want = e.call_x87(dll, st, Float::F64);
            for (i, &v) in (0u32..).zip(st) {
                e.mem().write(cell + 8 * i, v);
            }
            e.call_cdecl(caller, &[]).unwrap();
            let from_unicorn: f64 = e.mem().read(out);
            assert!(
                same(from_unicorn, want),
                "{name}({st:?}) from emulated code = {from_unicorn:e}, the DLL's {want:e}"
            );
        }
        assert!(
            fast >= min_rust as u64,
            "{name}: {fast} of {} in Rust",
            args.len()
        );
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

/// The DLL's own `memcpy`/`memmove` entries are traps to Rust
/// (`crt_dll::dll_copy`): called directly and from the DLL's code
/// (`memcpy_s`, which calls `memcpy`), forward, backward over an overlap
/// and past a 64 KB chunk, the bytes must be `memmove`'s (Rust's
/// `copy_within`) and `eax` the destination (`memcpy_s`: 0).
#[test]
#[allow(clippy::cast_possible_truncation)] // bytes of a pattern
fn the_dlls_copies_are_memmove() {
    let Some(exe) = exe() else { return };
    let mut e = ffx_host::boot(&exe).unwrap();
    let len = 0x3_0000u32;
    let at = e.alloc(len);
    let fresh: Vec<u8> = (0..len).map(|i| ((i * 7 + i / 251) % 256) as u8).collect();
    let memcpy = export(&e, "MSVCR110.dll", "memcpy");
    let memmove = export(&e, "MSVCR110.dll", "memmove");
    let memcpy_s = export(&e, "MSVCR110.dll", "memcpy_s");
    for (dst, src, n) in [
        (0u32, 0x100u32, 0x80u32),
        (0x100, 0, 0x80),
        (0x10, 0, 0x2_2345),
        (0, 0x10, 0x2_2345),
        (0x1_0000, 0x1_0007, 0x1_8000),
        (0x5, 0x5, 0x40),
        (0x40, 0, 0),
    ] {
        for f in [memcpy, memmove, memcpy_s] {
            e.mem().write_bytes(at, &fresh);
            let words = if f == memcpy_s {
                vec![at + dst, len - dst, at + src, n]
            } else {
                vec![at + dst, at + src, n]
            };
            let r = e.call_cdecl(f, &words).unwrap();
            let mut want = fresh.clone();
            want.copy_within(src as usize..(src + n) as usize, dst as usize);
            let mut got = vec![0u8; len as usize];
            e.mem().read_bytes(at, &mut got);
            assert!(got == want, "{f:#x} ({dst:#x}, {src:#x}, {n:#x})");
            assert_eq!(r, if f == memcpy_s { 0 } else { at + dst });
        }
    }
}

/// `atoi` and `atol` through the imports against the DLL's exports: signs,
/// spaces of every kind, junk after the digits, no digits, the edges of
/// `long` (the overflowing ones are handed on: compared too).
#[test]
fn atoi_matches_the_dll() {
    let Some(exe) = exe() else { return };
    let mut e = ffx_host::boot(&exe).unwrap();
    let cases: [&[u8]; 18] = [
        b"0",
        b"42",
        b"-17",
        b"+8x",
        b"  \t\n\x0b\x0c\r123",
        b"",
        b"abc",
        b"-",
        b"+-3",
        b" - 5",
        b"2147483647",
        b"-2147483648",
        b"2147483648",
        b"-2147483649",
        b"99999999999999",
        b"007",
        b"12 34",
        b"\xa012",
    ];
    for name in ["atoi", "atol"] {
        let rust: u32 = e.mem().read(slot(&e, "MSVCR110.dll", name));
        let dll = export(&e, "MSVCR110.dll", name);
        for c in cases {
            let at = e.alloc(32);
            let mut v = c.to_vec();
            v.push(0);
            e.mem().write_bytes(at, &v);
            let r = e.call_cdecl(rust, &[at]).unwrap();
            let d = e.call_cdecl(dll, &[at]).unwrap();
            assert_eq!(r, d, "{name}({:?})", String::from_utf8_lossy(c));
        }
    }
}

/// `qsort` (Rust side) against the DLL's: arrays of 0 to 120 records of
/// 8 and 12 bytes (few distinct keys, so equal ones meet, and many), the
/// comparator x86 code that logs both records' ids and compares their
/// first words signed; the array and the log must agree.
#[test]
#[allow(clippy::many_single_char_names)]
fn qsort_matches_the_dll() {
    let Some(exe) = exe() else { return };
    let mut e = ffx_host::boot(&exe).unwrap();
    let log = e.alloc(4 + 8 * 4096);
    // mov edx,[log]; and edx,0xfff; mov eax,[esp+4]; mov ecx,[esp+8];
    // push esi; mov esi,[eax+4]; mov [log+4+edx*8],esi; mov esi,[ecx+4];
    // mov [log+8+edx*8],esi; pop esi; inc dword [log];
    // mov eax,[eax]; sub eax,[ecx]  (the key difference: <0, 0, >0); ret
    let mut c = vec![0x8b, 0x15];
    c.extend(log.to_le_bytes());
    c.extend([
        0x81, 0xe2, 0xff, 0x0f, 0, 0, 0x8b, 0x44, 0x24, 0x04, 0x8b, 0x4c, 0x24, 0x08, 0x56,
    ]);
    c.extend([0x8b, 0x70, 0x04, 0x89, 0x34, 0xd5]);
    c.extend((log + 4).to_le_bytes());
    c.extend([0x8b, 0x71, 0x04, 0x89, 0x34, 0xd5]);
    c.extend((log + 8).to_le_bytes());
    c.extend([0x5e, 0xff, 0x05]);
    c.extend(log.to_le_bytes());
    c.extend([0x8b, 0x00, 0x2b, 0x01, 0xc3]);
    let comp = e.alloc_code(&c).unwrap();
    let rust: u32 = e.mem().read(slot(&e, "MSVCR110.dll", "qsort"));
    let dll = export(&e, "MSVCR110.dll", "qsort");
    let mut s = 0x2468_ace1u32;
    for round in 0..60u32 {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        let n: u32 = [0, 1, 2, 7, 8, 9, 17, 50, 120][(round % 9) as usize];
        let width = if round % 2 == 0 { 8u32 } else { 12 };
        let keys = if round % 3 == 0 { 4 } else { 1000 };
        let mut bytes = Vec::new();
        let mut r = s;
        for i in 0..n {
            r ^= r << 13;
            r ^= r >> 17;
            r ^= r << 5;
            bytes.extend((r % keys).to_le_bytes());
            bytes.extend((0x7700u32 + i).to_le_bytes());
            if width == 12 {
                bytes.extend(0x5a5a_5a5au32.to_le_bytes());
            }
        }
        let base = e.alloc(n * width + 4);
        let mut out = Vec::new();
        for f in [rust, dll] {
            e.mem().write_bytes(base, &bytes);
            e.mem().write(log, 0u32);
            e.call_cdecl(f, &[base, n, width, comp]).unwrap();
            let mut arr = vec![0u8; (n * width) as usize];
            e.mem().read_bytes(base, &mut arr);
            let count: u32 = e.mem().read(log);
            let mut calls = vec![0u8; 8 * count.min(4096) as usize];
            e.mem().read_bytes(log + 4, &mut calls);
            out.push((arr, count, calls));
        }
        assert!(
            out[0] == out[1],
            "qsort round {round} ({n} x {width}, {keys} keys)"
        );
    }
}
