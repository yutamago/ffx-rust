//! The C runtime DLLs' Rust sides (`ffx_host::crt_dll`) against the
//! player's DLLs in Unicorn: each called as `FFX.exe` and the ports call
//! it, and the DLL's own export called directly. Needs `FFX_GAME_DIR`;
//! skips without it.

// test data: the C names, the spreads' casts
#![allow(
    clippy::many_single_char_names,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

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

/// Arguments for the inverse sines and cosines: the edges and `n` spread
/// over [-1, 1] (denser near ±1 and 0), the band edges' neighbours.
#[allow(clippy::cast_possible_truncation)]
fn unit_operand(n: usize, seed: u32) -> Vec<Vec<f64>> {
    let mut s = Spread(seed);
    let mut xs: Vec<f64> = EDGES.to_vec();
    xs.extend([
        1.0,
        -1.0,
        0.0625,
        -0.0625,
        0.8649,
        0.8651,
        0.9922,
        -0.9922,
        1.0 - 1e-16,
        -1.0 + 1e-16,
        1e-18,
        -8.6e-19,
        1.000_000_1,
    ]);
    for i in 0..n {
        let u = s.unit();
        xs.push(match i % 5 {
            0 => (u - 0.5) * 2.0,
            1 => 1.0 - u * 0.02,
            2 => -(1.0 - u * 0.2),
            3 => (u - 0.5) * 0.15,
            _ => f64::from(((u - 0.5) * 2.0) as f32),
        });
    }
    xs.into_iter().map(|x| vec![x]).collect()
}

/// Positive arguments (for the logarithms): the edges' magnitudes and
/// `n` spread over the whole exponent range, near 1 and denormal.
fn positive_operand(n: usize, seed: u32) -> Vec<Vec<f64>> {
    let mut s = Spread(seed);
    let mut xs: Vec<f64> = EDGES.iter().map(|x| x.abs()).collect();
    xs.extend([
        -1.0,
        -0.0,
        f64::NEG_INFINITY,
        1.0 + 1e-15,
        0.999_999_9,
        1.007_812_5,
    ]);
    for i in 0..n {
        let u = s.unit();
        xs.push(match i % 4 {
            0 => 10f64.powf((u - 0.5) * 600.0),
            1 => 1.0 + (u - 0.5) * 0.05,
            2 => u * 1e-310,
            _ => u * 1000.0,
        });
    }
    xs.into_iter().map(|x| vec![x]).collect()
}

/// Arguments for `exp`: the edges and `n` spread over the range that
/// gives finite results, with the ends where the scaling runs in halves.
fn exp_operand(n: usize, seed: u32) -> Vec<Vec<f64>> {
    let mut s = Spread(seed);
    let mut xs: Vec<f64> = EDGES.to_vec();
    xs.extend([
        709.78, 709.79, -708.39, -708.4, -745.13, -745.14, -744.0, 1e-17, -1e-17,
    ]);
    for i in 0..n {
        let u = s.unit();
        xs.push(match i % 4 {
            0 => (u - 0.5) * 1400.0,
            1 => (u - 0.5) * 20.0,
            2 => 690.0 + u * 20.0,
            _ => -760.0 + u * 60.0,
        });
    }
    xs.into_iter().map(|x| vec![x]).collect()
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
    let cases: [(&str, u32, Vec<Vec<f64>>, usize); 9] = [
        (
            "_CIasin",
            0x0094_98f2,
            unit_operand(3000, 0x9abc_def0),
            2900,
        ),
        (
            "_CIacos",
            0x0094_98da,
            unit_operand(3000, 0x89ab_cdef),
            2900,
        ),
        ("_CIatan", 0x0094_9d5c, one_operand(3000, 0x789a_bcde), 2900),
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
            if name == "_CIsqrt" && (st[0].is_nan() || st[0] < 0.0) {
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

/// `dll!name`'s Rust side (the IAT slot's stub) and the DLL's export.
fn both(e: &Exec, name: &str) -> (u32, u32) {
    (
        e.mem().read(slot(e, "MSVCR110.dll", name)),
        export(e, "MSVCR110.dll", name),
    )
}

/// `s` and a NUL (and junk after it) in fresh memory.
fn put(e: &mut Exec, s: &[u8]) -> u32 {
    let at = e.alloc(u32::try_from(s.len()).unwrap() + 16);
    let mut v = s.to_vec();
    v.extend_from_slice(&[0, 0x5a, 0x5a, 0x5a]);
    e.mem().write_bytes(at, &v);
    at
}

/// The wide string `s` and a NUL in fresh memory.
fn put_w(e: &mut Exec, s: &str) -> u32 {
    let mut v: Vec<u8> = s.encode_utf16().flat_map(u16::to_le_bytes).collect();
    v.extend_from_slice(&[0, 0, 0x5a, 0x5a]);
    let at = e.alloc(u32::try_from(v.len()).unwrap() + 8);
    e.mem().write_bytes(at, &v);
    at
}

const TEXTS: [&[u8]; 12] = [
    b"",
    b"a",
    b"Hello World",
    b"hello world",
    b"HELLO",
    b"world",
    b"\xe9t\xc9\x80z",
    b"abcabcabd",
    b"abd",
    b"  -0x1Fz",
    b"12,34;;56",
    b";,lead",
];

/// The string functions of `crt_str` through the imports against the
/// DLL's exports: `eax` and every byte of a destination buffer (junk
/// around it), over strings with case, high bytes, empty ones, matches at
/// the start, the end and nowhere, and the sized functions at their
/// limits. Each call through the import must stay in Rust.
#[test]
#[allow(clippy::too_many_lines)]
fn crt_str_strings_match_the_dll() {
    let Some(exe) = exe() else { return };
    let mut e = ffx_host::boot(&exe).unwrap();
    let at: Vec<u32> = TEXTS.iter().map(|s| put(&mut e, s)).collect();
    let buf = e.alloc(0x100);
    let mut checked = 0;
    // one call each way: eax and the buffer
    let mut run = |e: &mut Exec, name: &str, words: &dyn Fn(u32) -> Vec<u32>, fill: &[u8]| {
        let (rust, dll) = both(e, name);
        let mut out = Vec::new();
        for f in [rust, dll] {
            let mut b = vec![0xeeu8; 0x100];
            b[..fill.len()].copy_from_slice(fill);
            e.mem().write_bytes(buf, &b);
            let before = e.stats.unicorn_calls;
            let r = e.call_cdecl(f, &words(buf)).unwrap();
            if f == rust {
                assert_eq!(e.stats.unicorn_calls, before, "{name} ran in Unicorn");
            }
            let mut got = vec![0u8; 0x100];
            e.mem().read_bytes(buf, &mut got);
            out.push((r.wrapping_sub(buf), got));
        }
        assert_eq!(out[0], out[1], "{name}");
        checked += 1;
    };
    for &x in &at {
        for &y in &at {
            for name in ["strspn", "strpbrk", "strcoll"] {
                run(&mut e, name, &|_| vec![x, y], &[]);
            }
            for n in [0u32, 1, 3, 5, 100] {
                run(&mut e, "strncmp", &|_| vec![x, y, n], &[]);
                run(&mut e, "_strnicmp", &|_| vec![x, y, n], &[]);
                run(&mut e, "strncat", &|b| vec![b, y, n], b"start\0");
            }
        }
        for c in [
            0u32,
            u32::from(b'o'),
            u32::from(b'l'),
            0xc9,
            0x1_0000 + u32::from(b'd'),
        ] {
            run(&mut e, "strrchr", &|_| vec![x, c], &[]);
        }
        // copies into the buffer: sizes that fit and the exact fit
        let len = u32::try_from(TEXTS[at.iter().position(|&a| a == x).unwrap()].len()).unwrap();
        for size in [len + 1, len + 5, 0x100] {
            run(&mut e, "strcpy_s", &|b| vec![b, size, x], &[]);
            run(&mut e, "strcat_s", &|b| vec![b, size + 4, x], b"abc\0");
        }
        for (size, count) in [
            (0x100, 0),
            (0x100, 3),
            (len + 1, len),
            (0x100, u32::MAX),
            (4, u32::MAX),
            (len + 1, 99),
        ] {
            run(&mut e, "strncpy_s", &|b| vec![b, size, x, count], &[]);
        }
        for (size, count) in [(0x100, 0), (0x100, 5), (8, 8)] {
            run(&mut e, "memcpy_s", &|b| vec![b, size, x, count], &[]);
            run(&mut e, "memmove_s", &|b| vec![b, size, x, count], &[]);
            run(&mut e, "wmemcpy_s", &|b| vec![b, size, x, count / 2], &[]);
        }
        // in place: a copy of the text in the buffer
        let mut text = TEXTS[at.iter().position(|&a| a == x).unwrap()].to_vec();
        text.push(0);
        run(&mut e, "_strlwr", &|b| vec![b], &text);
    }
    // overlapping moves
    run(
        &mut e,
        "memmove_s",
        &|b| vec![b + 3, 0x80, b, 40],
        b"0123456789abcdefghijklmnopqrstuvwxyz0123456789",
    );
    run(
        &mut e,
        "memmove_s",
        &|b| vec![b, 0x80, b + 3, 40],
        b"0123456789abcdefghijklmnopqrstuvwxyz0123456789",
    );
    // strtok over a buffer: the same sequence of tokens, then NULL
    for (text, delims) in [
        (&b"12,34;;56\0"[..], &b",;\0"[..]),
        (b";,lead,x;\0", b";,\0"),
        (b"\0", b",\0"),
        (b",,,\0", b",\0"),
        (b"one\0", b"\0"),
    ] {
        let d = put(&mut e, delims);
        let (rust, dll) = both(&e, "strtok");
        let mut out = Vec::new();
        for f in [rust, dll] {
            e.mem().write_bytes(buf, text);
            let mut seq = Vec::new();
            let mut s = buf;
            for _ in 0..6 {
                let r = e.call_cdecl(f, &[s, d]).unwrap();
                seq.push(r.wrapping_sub(buf));
                s = 0;
            }
            let mut got = vec![0u8; 16];
            e.mem().read_bytes(buf, &mut got);
            out.push((seq, got));
        }
        assert_eq!(out[0], out[1], "strtok {text:?}");
    }
    // the wide ones
    let wide: Vec<u32> = ["", "a", "Hello", "lo", "\u{e9}x\u{3042}", "xyz"]
        .iter()
        .map(|s| put_w(&mut e, s))
        .collect();
    let (rust, dll) = both(&e, "wcsstr");
    for &x in &wide {
        for &y in &wide {
            assert_eq!(
                e.call_cdecl(rust, &[x, y]).unwrap(),
                e.call_cdecl(dll, &[x, y]).unwrap(),
                "wcsstr"
            );
        }
        for size in [12u32, 40] {
            run(
                &mut e,
                "wcscat_s",
                &|b| vec![b, size, x],
                &[b'a', 0, b'b', 0, 0, 0],
            );
        }
    }
    assert!(checked > 2000, "{checked}");
}

/// The character classes (`isalpha` ... `iscntrl`), `toupper`/`tolower`,
/// `abs`, `ldiv`, `strtol`/`strtoul` and `realloc` through the imports
/// against the DLL: every `c` from -128 to 511, numbers in every base
/// with prefixes, signs, junk and overflows.
#[test]
#[allow(clippy::too_many_lines)]
fn crt_str_numbers_and_classes_match_the_dll() {
    let Some(exe) = exe() else { return };
    let mut e = ffx_host::boot(&exe).unwrap();
    for name in [
        "isalpha", "isupper", "islower", "isdigit", "isxdigit", "isspace", "ispunct", "isalnum",
        "isgraph", "iscntrl", "toupper", "tolower", "abs",
    ] {
        let (rust, dll) = both(&e, name);
        for c in (-128i32..512).chain([i32::MIN + 1, -77_777, 70_000, i32::MAX]) {
            if name.starts_with("is") && !(-128..512).contains(&c) {
                continue; // the DLL reads outside its table
            }
            let c = c.cast_unsigned();
            let before = e.stats.unicorn_calls;
            let r = e.call_cdecl(rust, &[c]).unwrap();
            assert_eq!(e.stats.unicorn_calls, before, "{name} ran in Unicorn");
            assert_eq!(r, e.call_cdecl(dll, &[c]).unwrap(), "{name}({c:#x})");
        }
    }
    let (rust, dll) = both(&e, "ldiv");
    for (n, d) in [
        (7i32, 2i32),
        (-7, 2),
        (7, -2),
        (-7, -2),
        (0, 5),
        (i32::MIN, 3),
        (i32::MAX, -1),
    ] {
        let call = |e: &mut Exec, f: u32| {
            let r = ffx_bridge::route::Router::call(
                e,
                f,
                ffx_host::table::Conv::Cdecl,
                &ffx_host::table::Args {
                    stack: vec![n.cast_unsigned(), d.cast_unsigned()],
                    ..Default::default()
                },
                ffx_bridge::route::Float::None,
            );
            (r.eax, r.edx)
        };
        assert_eq!(call(&mut e, rust), call(&mut e, dll), "ldiv({n}, {d})");
    }
    let texts: [&[u8]; 22] = [
        b"0",
        b"42",
        b"-17",
        b"+8x",
        b"  \t\n123",
        b"",
        b"abc",
        b"-",
        b"0x1A",
        b"0X",
        b"0x",
        b"017",
        b"09",
        b"z",
        b"Zz9",
        b"2147483647",
        b"-2147483648",
        b"4294967295",
        b"1y",
        b"  -0x7fffffff",
        b"0b101",
        b"7fffffff",
    ];
    let end = e.alloc(8);
    for name in ["strtol", "strtoul"] {
        let (rust, dll) = both(&e, name);
        for t in texts {
            let s = put(&mut e, t);
            for base in [0u32, 2, 8, 10, 16, 36] {
                let mut out = Vec::new();
                for f in [rust, dll] {
                    e.mem().write(end, 0x1234_5678u32);
                    let r = e.call_cdecl(f, &[s, end, base]).unwrap();
                    let stop: u32 = e.mem().read(end);
                    out.push((r, stop.wrapping_sub(s)));
                }
                assert_eq!(
                    out[0],
                    out[1],
                    "{name}({:?}, {base})",
                    String::from_utf8_lossy(t)
                );
            }
        }
    }
    // realloc: grow, shrink, from NULL, to 0; the bytes kept
    let (rust, _) = both(&e, "realloc");
    let p = e.call_cdecl(rust, &[0, 16]).unwrap();
    e.mem().write_bytes(p, b"0123456789abcdef");
    let q = e.call_cdecl(rust, &[p, 64]).unwrap();
    let mut got = [0u8; 16];
    e.mem().read_bytes(q, &mut got);
    assert_eq!(&got, b"0123456789abcdef");
    let r = e.call_cdecl(rust, &[q, 4]).unwrap();
    let mut got = [0u8; 4];
    e.mem().read_bytes(r, &mut got);
    assert_eq!(&got, b"0123");
    assert_eq!(e.call_cdecl(rust, &[r, 0]).unwrap(), 0);
}

/// The double functions (`floor`, `ceil`, `modf`, `frexp`, `ldexp`,
/// `_finite`, `_isnan`) through the imports against the DLL: `ST0`, `eax`
/// and the output word, over zeros, denormals, halves, large and
/// infinite values (NaNs are handed on: compared too).
#[test]
fn crt_str_doubles_match_the_dll() {
    use ffx_bridge::route::Float;
    let Some(exe) = exe() else { return };
    let mut e = ffx_host::boot(&exe).unwrap();
    let out = e.alloc(16);
    let xs = [
        0.0,
        -0.0,
        5e-324,
        -1e-310,
        0.25,
        0.5,
        -0.5,
        1.5,
        -1.5,
        2.5,
        -2.5,
        3.0,
        -7.99,
        1e15 + 0.5,
        4.5e15,
        -9.1e18,
        1e300,
        f64::MAX,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        123.456,
        -0.001,
    ];
    let call = |e: &mut Exec, f: u32, words: Vec<u32>, float: Float| {
        let r = ffx_bridge::route::Router::call(
            e,
            f,
            ffx_host::table::Conv::Cdecl,
            &ffx_host::table::Args {
                stack: words,
                ..Default::default()
            },
            float,
        );
        (r.eax, r.st0.to_bits())
    };
    #[allow(clippy::cast_possible_truncation)]
    let words = |x: f64| vec![x.to_bits() as u32, (x.to_bits() >> 32) as u32];
    for name in [
        "floor", "ceil", "modf", "frexp", "ldexp", "_finite", "_isnan",
    ] {
        let (rust, dll) = both(&e, name);
        for &x in &xs {
            let extras: Vec<Vec<u32>> = match name {
                "modf" | "frexp" => vec![vec![out]],
                "ldexp" => [0i32, 1, -1, 10, -1074, -1080, 1024, 2000, -2000]
                    .iter()
                    .map(|n| vec![n.cast_unsigned()])
                    .collect(),
                _ => vec![vec![]],
            };
            let float = if name.starts_with('_') {
                Float::None
            } else {
                Float::F64
            };
            for extra in extras {
                let mut w = words(x);
                w.extend(&extra);
                let mut got = Vec::new();
                for f in [rust, dll] {
                    e.mem().write(out, 0x5a5a_5a5a_5a5a_5a5au64);
                    let r = call(&mut e, f, w.clone(), float);
                    let side: u64 = e.mem().read(out);
                    let none = float == Float::None;
                    got.push((if none { r.0 } else { 0 }, if none { 0 } else { r.1 }, side));
                }
                assert_eq!(got[0], got[1], "{name}({x:e}, {extra:?})");
            }
        }
    }
}

/// A `TypeDescriptor` for the decorated `name`.
fn type_descriptor(e: &mut Exec, name: &[u8]) -> u32 {
    let at = e.alloc(8 + u32::try_from(name.len()).unwrap() + 8);
    let mut v = vec![0x11u8, 0x22, 0x33, 0x44, 0, 0, 0, 0];
    v.extend_from_slice(name);
    v.push(0);
    e.mem().write_bytes(at, &v);
    at
}

/// A `BaseClassDescriptor`: (type, contained bases, mdisp, pdisp, vdisp,
/// attributes).
fn base_class(e: &mut Exec, td: u32, n: u32, pmd: (u32, i32, u32), attr: u32) -> u32 {
    let at = e.alloc(0x20);
    let mut mem = e.mem();
    for (i, w) in (0u32..).zip([td, n, pmd.0, pmd.1.cast_unsigned(), pmd.2, attr, 0]) {
        mem.write(at + 4 * i, w);
    }
    at
}

/// An object whose vtable's locator says (`offset`, `cd_offset`, type,
/// hierarchy attributes, the base descriptors); `vfptr_at` is where the
/// vtable pointer sits in the object. The object's address and `p`.
fn rtti_object(e: &mut Exec, offset: u32, attributes: u32, bases: &[u32], td: u32) -> u32 {
    let array = e.alloc(4 * u32::try_from(bases.len()).unwrap() + 4);
    for (i, b) in (0u32..).zip(bases) {
        e.mem().write(array + 4 * i, *b);
    }
    let chd = e.alloc(16);
    let col = e.alloc(20);
    let vtable = e.alloc(16);
    let object = e.alloc(0x40);
    let mut mem = e.mem();
    for (i, w) in (0u32..).zip([0, attributes, u32::try_from(bases.len()).unwrap(), array]) {
        mem.write(chd + 4 * i, w);
    }
    for (i, w) in (0u32..).zip([0, offset, 0, td, chd]) {
        mem.write(col + 4 * i, w);
    }
    mem.write(vtable, col);
    mem.write(vtable + 4, 0x0040_1000u32);
    mem.write_bytes(object, &[0; 0x40]);
    mem.write(object + offset, vtable + 4);
    object
}

/// `__RTDynamicCast` (Rust side) against the DLL's over synthetic RTTI:
/// a single-inheritance chain C : B : A cast from each type to each, to
/// an unrelated type, to a type equal only by name, past a base marked
/// private, through a sub-object pointer (the locator's offset) with
/// `mdisp` offsets, NULL; and a multiple-inheritance hierarchy (handed
/// on).
#[test]
fn rt_dynamic_cast_matches_the_dll() {
    let Some(exe) = exe() else { return };
    let mut e = ffx_host::boot(&exe).unwrap();
    let (rust, dll) = both(&e, "__RTDynamicCast");
    let ta = type_descriptor(&mut e, b".?AVA@@");
    let tb = type_descriptor(&mut e, b".?AVB@@");
    let tc = type_descriptor(&mut e, b".?AVC@@");
    let td = type_descriptor(&mut e, b".?AVD@@");
    let ta2 = type_descriptor(&mut e, b".?AVA@@");
    let bc = base_class(&mut e, tc, 2, (0, -1, 0), 0);
    let bb = base_class(&mut e, tb, 1, (0, -1, 0), 0);
    let ba = base_class(&mut e, ta, 0, (4, -1, 0), 0);
    let hidden = base_class(&mut e, tb, 0, (8, -1, 0), 4);
    let mut cases = Vec::new();
    for (offset, attributes, bases) in [
        (0u32, 0u32, vec![bc, bb, ba]),
        (8, 0, vec![bc, bb, ba]),
        (0, 0, vec![bc, hidden, bb]),
        (0, 1, vec![bc, bb, ba]),
    ] {
        let object = rtti_object(&mut e, offset, attributes, &bases, tc);
        for source in [ta, tb, tc, ta2] {
            for target in [ta, tb, tc, td, ta2] {
                cases.push(vec![object + offset, 0, source, target, 0]);
            }
        }
    }
    cases.push(vec![0, 0, tc, ta, 0]);
    for words in cases {
        let before = e.stats.unicorn_calls;
        let r = e.call_cdecl(rust, &words).unwrap();
        let in_rust = e.stats.unicorn_calls == before;
        assert_eq!(r, e.call_cdecl(dll, &words).unwrap(), "{words:x?}");
        let multiple = {
            let vt: u32 = if words[0] == 0 {
                0
            } else {
                e.mem().read(words[0])
            };
            vt != 0 && {
                let col: u32 = e.mem().read(vt - 4);
                let chd: u32 = e.mem().read(col + 0x10);
                e.mem().read::<u32>(chd + 4) & 1 != 0
            }
        };
        assert_eq!(in_rust, !multiple, "{words:x?}");
    }
}

/// `dll!name`'s slot value with and without the DLLs (two boots): the
/// no-DLL functions' Rust sides against the DLL's own results.
fn no_dll_boot(exe: &std::path::Path) -> Exec {
    let opts = ffx_host::Options {
        no_dlls: true,
        ..ffx_host::Options::default()
    };
    ffx_host::boot_opts(exe, ffx_host::ports::table(), &opts).unwrap()
}

/// The no-DLL environment, locale, error-text and time functions
/// (`crt_os`) against the DLL in a run with it: `strerror` for every
/// number, `setlocale`'s queries, `getenv`/`_dupenv_s` (and, without the
/// DLL only, after `_putenv_s`), the
/// `tm` of `_gmtime64` and `_localtime64` (the run's local time is UTC
/// in both: the stand-ins give no time zone), `_mktime64`'s
/// normalization, `strftime` of every conversion, `system(NULL)`.
#[test]
#[allow(clippy::too_many_lines)]
fn crt_os_matches_the_dll() {
    let Some(exe) = exe() else { return };
    let mut with = ffx_host::boot(&exe).unwrap();
    let mut without = no_dll_boot(&exe);
    assert!(without.shared().dlls.is_empty());
    let call = |e: &mut Exec, name: &str, words: &[u32]| -> u32 {
        let f: u32 = e.mem().read(slot(e, "MSVCR110.dll", name));
        e.call_cdecl(f, words)
            .unwrap_or_else(|_| panic!("{name}: {:?}", e.stopped()))
    };
    let text = |e: &Exec, at: u32| -> Vec<u8> {
        let mut v = Vec::new();
        let mut p = at;
        loop {
            let b: u8 = e.mem().read(p);
            if b == 0 || v.len() > 300 {
                return v;
            }
            v.push(b);
            p += 1;
        }
    };
    for n in (0u32..50).chain([u32::MAX, 0x8000_0000]) {
        let a = call(&mut with, "strerror", &[n]);
        let b = call(&mut without, "strerror", &[n]);
        assert_eq!(text(&with, a), text(&without, b), "strerror({n})");
    }
    for e in [&mut with, &mut without] {
        let c = put(e, b"C");
        let empty = put(e, b"");
        let r = call(e, "setlocale", &[0, 0]);
        assert_eq!(text(e, r), b"C");
        let r = call(e, "setlocale", &[4, c]);
        assert_eq!(text(e, r), b"C");
        let _ = empty;
        let name = put(e, b"FFX_TEST_VAR");
        let lower = put(e, b"ffx_test_var");
        let value = put(e, b"on");
        assert_eq!(call(e, "getenv", &[name]), 0);
        let out = e.alloc(8);
        assert_eq!(call(e, "_dupenv_s", &[out, out + 4, name]), 0);
        assert_eq!(e.mem().read::<u64>(out), 0);
        assert_eq!(call(e, "system", &[0]), 0, "system(NULL)");
        if !e.shared().dlls.is_empty() {
            // the DLL's _putenv_s needs SetEnvironmentVariableA, which
            // has no stand-in: the rest runs without the DLLs only
            continue;
        }
        assert_eq!(call(e, "_putenv_s", &[name, value]), 0);
        let r = call(e, "getenv", &[lower]);
        assert_eq!(text(e, r), b"on");
        let out = e.alloc(8);
        assert_eq!(call(e, "_dupenv_s", &[out, out + 4, name]), 0);
        let p: u32 = e.mem().read(out);
        assert_eq!(text(e, p), b"on");
        assert_eq!(e.mem().read::<u32>(out + 4), 3);
    }
    let time_at = |e: &mut Exec, t: u64| {
        let at = e.alloc(8);
        e.mem().write(at, t);
        at
    };
    for t in [
        0u64,
        86_399,
        951_782_400,
        1_700_000_000,
        4_102_444_800,
        32_535_215_999,
    ] {
        for name in ["_gmtime64", "_localtime64"] {
            let mut got = Vec::new();
            for e in [&mut with, &mut without] {
                let when = time_at(e, t);
                let r = call(e, name, &[when]);
                let mut b = vec![0u8; 36];
                e.mem().read_bytes(r, &mut b);
                got.push(b);
            }
            assert_eq!(got[0], got[1], "{name}({t})");
        }
    }
    // mktime of out-of-range fields, normalized
    for fields in [
        [0i32, 0, 0, 1, 0, 70, 0, 0, 0],
        [75, 61, 25, 32, 13, 99, 0, 0, 0],
        [-1, 0, 0, 1, 0, 100, 0, 0, 0],
        [0, 0, 12, 29, 1, 124, 0, 0, 0],
    ] {
        let mut got = Vec::new();
        for e in [&mut with, &mut without] {
            let tm = e.alloc(36);
            for (i, v) in (0u32..).zip(fields) {
                e.mem().write(tm + 4 * i, v.cast_unsigned());
            }
            let f: u32 = e.mem().read(slot(e, "MSVCR110.dll", "_mktime64"));
            let r = ffx_bridge::route::Router::call(
                e,
                f,
                ffx_host::table::Conv::Cdecl,
                &ffx_host::table::Args {
                    stack: vec![tm],
                    ..Default::default()
                },
                ffx_bridge::route::Float::None,
            );
            let mut b = vec![0u8; 36];
            e.mem().read_bytes(tm, &mut b);
            got.push((r.eax, r.edx, b));
        }
        assert_eq!(got[0], got[1], "_mktime64({fields:?})");
    }
    for fmt in [
        "%a %A %b %B %d %H %I %j %m %M %p %S %U %w %W %y %Y %%",
        "%c|%x|%X",
        "%#d %#j",
        "plain",
        "",
    ] {
        let mut got = Vec::new();
        for e in [&mut with, &mut without] {
            let when = time_at(e, 1_700_000_000);
            let tm = call(e, "_gmtime64", &[when]);
            let f = put(e, fmt.as_bytes());
            let buf = e.alloc(200);
            e.mem().write_bytes(buf, &[0xee; 200]);
            let n = call(e, "strftime", &[buf, 200, f, tm]);
            got.push((n, text(e, buf)));
        }
        assert_eq!(got[0], got[1], "strftime({fmt})");
    }
}

/// A `basic_streambuf<char>` with its six pointer cells, a 16-byte
/// buffer and a vtable whose `overflow`/`underflow`/`uflow`/`xsputn` are
/// x86 stubs returning 0x77, 0x66, 0x55 and 9 (`ret 4`, `ret`, `ret`,
/// `ret 0xc`).
fn streambuf(e: &mut Exec) -> (u32, u32) {
    let this = e.alloc(0x40);
    let cells = e.alloc(0x20);
    let buf = e.alloc(16);
    e.mem().write_bytes(buf, b"abcdefghijklmnop");
    let stubs = e
        .alloc_code(&[
            0xb8, 0x77, 0, 0, 0, 0xc2, 4, 0, // overflow
            0xb8, 0x66, 0, 0, 0, 0xc3, 0x90, 0x90, // underflow
            0xb8, 0x55, 0, 0, 0, 0xc3, 0x90, 0x90, // uflow
            0xb8, 9, 0, 0, 0, 0x31, 0xd2, 0xc2, 0xc, 0, // xsputn: eax 9, edx 0
        ])
        .unwrap();
    let vt = e.alloc(0x40);
    let mut mem = e.mem();
    mem.write_bytes(this, &[0; 0x40]);
    mem.write(this, vt);
    for (i, off) in (0u32..).zip([0xc, 0x10, 0x1c, 0x20, 0x2c, 0x30]) {
        mem.write(this + off, cells + 4 * i);
    }
    mem.write(vt + 0xc, stubs);
    mem.write(vt + 0x18, stubs + 8);
    mem.write(vt + 0x1c, stubs + 16);
    mem.write(vt + 0x24, stubs + 24);
    (this, buf)
}

/// `MSVCP110`'s stream buffer and `ios_base` accessors (`crt_cpp`)
/// against the DLL's: the same object, its cells set the same way, each
/// accessor through the import and the DLL's export: `eax` (and `edx`)
/// and all six cells after, for full, partial, empty and missing buffers
/// (the virtuals' stubs cover the slow paths).
#[test]
#[allow(clippy::too_many_lines)]
fn crt_cpp_accessors_match_the_dll() {
    let Some(exe) = exe() else { return };
    let mut e = ffx_host::boot(&exe).unwrap();
    let (sb, buf) = streambuf(&mut e);
    let sbn =
        |m: &str, sig: &str| format!("?{m}@?$basic_streambuf@DU?$char_traits@D@std@@@std@@{sig}");
    let calls: Vec<(String, Vec<u32>)> = vec![
        (sbn("eback", "IBEPADXZ"), vec![]),
        (sbn("gptr", "IBEPADXZ"), vec![]),
        (sbn("egptr", "IBEPADXZ"), vec![]),
        (sbn("pbase", "IBEPADXZ"), vec![]),
        (sbn("pptr", "IBEPADXZ"), vec![]),
        (sbn("epptr", "IBEPADXZ"), vec![]),
        (sbn("setg", "IAEXPAD00@Z"), vec![buf, buf + 3, buf + 9]),
        (sbn("setp", "IAEXPAD0@Z"), vec![buf + 1, buf + 12]),
        (sbn("setp", "IAEXPAD00@Z"), vec![buf, buf + 5, buf + 6]),
        (sbn("pbump", "IAEXH@Z"), vec![3]),
        (sbn("gbump", "IAEXH@Z"), vec![(-2i32).cast_unsigned()]),
        (sbn("_Pninc", "IAEPADXZ"), vec![]),
        (sbn("sputc", "QAEHD@Z"), vec![0x1e9]),
        (sbn("sgetc", "QAEHXZ"), vec![]),
        (sbn("snextc", "QAEHXZ"), vec![]),
        (sbn("sputn", "QAE_JPBD_J@Z"), vec![buf, 5, 0]),
    ];
    // the cells' starting states: a buffer of 16, one with 1 left, none
    let states: [[u32; 6]; 5] = [
        [buf, buf, buf + 2, buf + 4, 14, 12],
        [buf, buf, buf + 15, buf + 15, 1, 1],
        [buf, buf, buf + 16, buf + 16, 0, 0],
        [0, 0, 0, 0, 0, 0],
        [buf, buf, buf + 3, buf + 3, 2, 0],
    ];
    let ptrs: u32 = e.mem().read(sb + 0xc);
    let mut checked = 0;
    for (name, words) in &calls {
        let (rust, dll) = both_cpp(&e, name);
        for st in states {
            let mut got = Vec::new();
            for f in [rust, dll] {
                for (i, v) in (0u32..).zip(st) {
                    e.mem().write(ptrs + 4 * i, v);
                }
                e.mem().write_bytes(buf, b"abcdefghijklmnop");
                let args = ffx_host::table::Args {
                    ecx: sb,
                    stack: words.clone(),
                    ..Default::default()
                };
                let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    e.call(f, ffx_host::table::Conv::Thiscall, &args)
                }));
                let Ok(Ok(r)) = caught else {
                    panic!("{name} {f:#x} {st:x?}: {:?}", e.stopped());
                };
                let r = (r.eax, r.edx.unwrap_or(0));
                let r = Raw { eax: r.0, edx: r.1 };
                let mut after = vec![0u8; 0x20 + 16];
                e.mem().read_bytes(ptrs, &mut after[..0x18]);
                e.mem().read_bytes(buf, &mut after[0x20..]);
                let edx = if name.contains("_J") { r.edx } else { 0 };
                got.push((r.eax, edx, after));
            }
            assert_eq!(got[0], got[1], "{name} {st:x?}");
            checked += 1;
        }
    }
    // ios_base / basic_ios fields
    let ios = e.alloc(0x50);
    let pattern: Vec<u8> = (0..0x50u8).map(|i| i.wrapping_mul(37) ^ 0x5a).collect();
    let iosn = |m: &str, sig: &str| format!("?{m}@?$basic_ios@DU?$char_traits@D@std@@@std@@{sig}");
    for (name, words) in [
        ("?rdstate@ios_base@std@@QBEHXZ".to_string(), vec![]),
        ("?good@ios_base@std@@QBE_NXZ".to_string(), vec![]),
        ("?eof@ios_base@std@@QBE_NXZ".to_string(), vec![]),
        ("?flags@ios_base@std@@QBEHXZ".to_string(), vec![]),
        ("?width@ios_base@std@@QBE_JXZ".to_string(), vec![]),
        ("?width@ios_base@std@@QAE_J_J@Z".to_string(), vec![7, 0]),
        (iosn("fill", "QBEDXZ"), vec![]),
        (
            iosn("tie", "QBEPAV?$basic_ostream@DU?$char_traits@D@std@@@2@XZ"),
            vec![],
        ),
        (
            iosn(
                "rdbuf",
                "QBEPAV?$basic_streambuf@DU?$char_traits@D@std@@@2@XZ",
            ),
            vec![],
        ),
        (iosn("setstate", "QAEXH_N@Z"), vec![2, 0]),
        (iosn("setstate", "QAEXH_N@Z"), vec![0, 0]),
    ] {
        let (rust, dll) = both_cpp(&e, &name);
        for (state, except, strbuf) in [(0u32, 0u32, 1u32), (1, 0, 1), (6, 0, 0), (0, 0, 0)] {
            let mut got = Vec::new();
            for f in [rust, dll] {
                e.mem().write_bytes(ios, &pattern);
                e.mem().write(ios + 0xc, state);
                e.mem().write(ios + 0x10, except);
                e.mem().write(ios + 0x38, strbuf);
                let r = ffx_bridge::route::Router::call(
                    &mut e,
                    f,
                    ffx_host::table::Conv::Thiscall,
                    &ffx_host::table::Args {
                        ecx: ios,
                        stack: words.clone(),
                        ..Default::default()
                    },
                    ffx_bridge::route::Float::None,
                );
                let mut after = vec![0u8; 0x50];
                e.mem().read_bytes(ios, &mut after);
                let eax = if name.contains("fill") {
                    r.eax & 0xff
                } else {
                    r.eax
                };
                let edx = if name.contains("_J") { r.edx } else { 0 };
                got.push((eax, edx, after));
            }
            assert_eq!(got[0], got[1], "{name} state {state}");
            checked += 1;
        }
    }
    assert!(checked > 100, "{checked}");
}

/// A call's registers.
struct Raw {
    eax: u32,
    edx: u32,
}

/// `MSVCP110`'s import (Rust side) and export.
fn both_cpp(e: &Exec, name: &str) -> (u32, u32) {
    (
        e.mem().read(slot(e, "MSVCP110.dll", name)),
        export(e, "MSVCP110.dll", name),
    )
}

/// Without the DLLs `std::cin.getline` (the auto-test console thread's
/// poll) finds the end of input as the DLL does with the run's empty
/// console: `eax`, the buffer, `_Chcount` and the stream state after one
/// and after a second call, against the DLL's run.
#[test]
fn cin_getline_without_the_dlls_matches_the_dll() {
    const CIN_SLOT: u32 = 0x00b0_c200;
    let Some(exe) = exe() else { return };
    let mut got = Vec::new();
    for dlls in [true, false] {
        let mut e = if dlls {
            ffx_host::boot(&exe).unwrap()
        } else {
            no_dll_boot(&exe)
        };
        let cin: u32 = e.mem().read(CIN_SLOT);
        let at: u32 = e
            .mem()
            .read(slot(&e, "MSVCP110.dll", ffx_host::crt_dll::GETLINE));
        let buffer = e.alloc(100);
        let mut runs = Vec::new();
        for _ in 0..2 {
            e.mem().write_bytes(buffer, &[0x55; 100]);
            let args = ffx_host::table::Args {
                ecx: cin,
                stack: vec![buffer, 99, 0],
                ..Default::default()
            };
            let r = e.call(at, ffx_host::table::Conv::Thiscall, &args).unwrap();
            let vbtable: u32 = e.mem().read(cin);
            let ios = cin + e.mem().read::<u32>(vbtable + 4);
            let state: u32 = e.mem().read(ios + 0xc);
            let count: u64 = e.mem().read(cin + 8);
            let first: u8 = e.mem().read(buffer);
            runs.push((r.eax == cin, state, count, first));
        }
        got.push(runs);
    }
    assert_eq!(got[0], got[1]);
}

/// The SSE2 maths functions (`_libm_sse2_*_precise`, `xmm0` in and out)
/// through `Exec::call_sse` (their Rust sides, as the overlays' ports call
/// them) against the DLL's exports run in Unicorn, bit for bit, over the
/// one-operand spread; `min_rust` calls must take Rust.
#[test]
fn sse2_functions_match_the_dll() {
    let Some(exe) = exe() else { return };
    let mut e = ffx_host::boot(&exe).unwrap();
    let cases: [(&str, u32, usize); 9] = [
        ("_libm_sse2_exp_precise", 0x9100_0009, 2300),
        ("_libm_sse2_log_precise", 0x9100_0008, 3000),
        ("_libm_sse2_asin_precise", 0x9100_0007, 3000),
        ("_libm_sse2_acos_precise", 0x9100_0006, 3000),
        ("_libm_sse2_atan_precise", 0x9100_0005, 3000),
        ("_libm_sse2_sin_precise", 0x9100_0001, 3000),
        ("_libm_sse2_cos_precise", 0x9100_0002, 3000),
        ("_libm_sse2_tan_precise", 0x9100_0003, 3000),
        ("_libm_sse2_sqrt_precise", 0x9100_0004, 1400),
    ];
    for (name, seed, min_rust) in cases {
        let at = export(&e, "MSVCR110.dll", name);
        let mut fast = 0;
        let args = if name.contains("acos") || name.contains("asin") {
            unit_operand(3000, seed)
        } else if name.contains("log") {
            positive_operand(3000, seed)
        } else if name.contains("exp") {
            exp_operand(3000, seed)
        } else {
            one_operand(3000, seed)
        };
        for st in args {
            let x = st[0];
            let before = e.stats.leaf_calls;
            let rust = e.call_sse(at, x);
            fast += e.stats.leaf_calls - before;
            let want = e.call_sse_emulated(at, x);
            assert!(
                same(rust, want),
                "{name}({x:e}) = {rust:e} ({:#x}), the DLL's {want:e} ({:#x})",
                rust.to_bits(),
                want.to_bits()
            );
        }
        assert!(fast >= min_rust as u64, "{name}: {fast} in Rust");
    }
}

/// Decimal texts for the conversions: short ones as the game's files and
/// `%f` write them, signs, exponents (`e`, `E`, `d`), leading spaces,
/// junk after, no digits, and over- and underflow.
fn decimal_texts(n: usize, seed: u32) -> Vec<String> {
    let mut v: Vec<String> = [
        "0",
        "1",
        "-1",
        "+2.5",
        "  3.25x",
        ".5",
        "5.",
        "-.0",
        "1e10",
        "1E-5",
        "2d3",
        "-7.5D-2",
        "1e",
        "1e+",
        "abc",
        "",
        "-",
        ".",
        "1.7976931348623157e308",
        "1e309",
        "-1e400",
        "4.9e-324",
        "1e-330",
        "2.2250738585072014e-308",
        "0.1",
        "0.2",
        "0.3",
        "123456789",
        "3.14159265358979",
        "\t\n 42",
        "0x10",
        "1,5",
        "1..2",
        "nan",
        "inf",
    ]
    .iter()
    .map(ToString::to_string)
    .collect();
    let mut s = Spread(seed);
    for i in 0..n {
        let u = s.unit();
        let w = s.unit();
        #[allow(clippy::cast_possible_truncation)]
        let t = match i % 5 {
            0 => format!("{:.6}", (u - 0.5) * 2000.0),
            1 => format!("{}", ((u - 0.5) * 100.0) as f32),
            2 => format!("{:e}", (u - 0.3) * 10f64.powi((w * 40.0) as i32 - 20)),
            3 => format!("{:.3}", u * 10.0),
            _ => format!("{}.{:05}", (u * 1e6) as u32, (w * 99999.0) as u32),
        };
        v.push(t);
    }
    v
}

/// `strtod` and `atof` (Rust sides) against the DLL's: `ST0`'s bits and
/// the end pointer over the decimal texts.
#[test]
fn strtod_and_atof_match_the_dll() {
    use ffx_bridge::route::Float;
    let Some(exe) = exe() else { return };
    let mut e = ffx_host::boot(&exe).unwrap();
    let end = e.alloc(8);
    let mut fast = 0;
    let texts = decimal_texts(6000, 0x1357_9bdf);
    for t in &texts {
        let s = put(&mut e, t.as_bytes());
        for name in ["strtod", "atof"] {
            let (rust, dll) = both(&e, name);
            let mut got = Vec::new();
            for f in [rust, dll] {
                e.mem().write(end, 0x5a5a_5a5au32);
                let words = if name == "strtod" {
                    vec![s, end]
                } else {
                    vec![s]
                };
                let before = e.stats.unicorn_calls;
                let r = ffx_bridge::route::Router::call(
                    &mut e,
                    f,
                    ffx_host::table::Conv::Cdecl,
                    &ffx_host::table::Args {
                        stack: words,
                        ..Default::default()
                    },
                    Float::F64,
                );
                if f == rust && e.stats.unicorn_calls == before {
                    fast += 1;
                }
                let stop: u32 = e.mem().read(end);
                got.push((r.st0.to_bits(), stop.wrapping_sub(s)));
            }
            assert_eq!(got[0], got[1], "{name}({t:?})");
        }
    }
    // the long mantissas (most of the `{:e}` ones) are handed on
    assert!(fast > texts.len() * 3 / 2, "{fast} in Rust");
}

/// The `sscanf` family (Rust sides) against the DLL's: the game's formats
/// over matching, partly matching and empty inputs, `%f` into floats and
/// `%lf` into doubles from the decimal texts, `%s` into buffers, `_s`
/// sizes, `swscanf_s`'s `VID_%4X`; the count and every output byte.
#[test]
#[allow(clippy::too_many_lines)]
fn sscanf_family_matches_the_dll() {
    let Some(exe) = exe() else { return };
    let mut e = ffx_host::boot(&exe).unwrap();
    let out = e.alloc(0x100);
    let mut cases: Vec<(&str, String, String)> = Vec::new();
    for (fmt, inputs) in [
        (
            "%d",
            vec!["42", " -17", "+8x", "", "abc", "2147483648", "  "],
        ),
        ("%03d", vec!["7", "12345", "-1"]),
        ("%d %d %d %d %d %d", vec!["1 2 3 4 5 6", "1 2 3", "1 2 x 4"]),
        ("%d,%d", vec!["3,4", "3 ,4", "3,", ",4"]),
        ("%d:", vec!["5:", "5", "5;"]),
        ("%d:%d:%d", vec!["12:34:56", "1:2"]),
        ("%d_%d_%d_%d_%d_%d", vec!["1_2_3_4_5_6", "1_2_3"]),
        (
            "%x",
            vec!["ff", "0x1A", "-10", "g", "FFFFFFFF", "123456789"],
        ),
        (":%d", vec![":9", "9", " :9"]),
        (
            "JumpMap %s",
            vec!["JumpMap bsil0300", "JumpMap", "Jump bsil", "JumpMap  a b"],
        ),
        ("SetBattle %s", vec!["SetBattle btl01 x"]),
        (
            "scene%dlayer%dShape",
            vec![
                "scene3layer12Shape",
                "scene3layerShape",
                "scene-1layer0Shape",
            ],
        ),
        ("%i %i %i", vec!["010 0x10 10", "-0x7f +07 9"]),
        ("%u %o %X", vec!["4294967295 777 aBc"]),
        ("%*d %d", vec!["1 2", "1"]),
        ("%2d%3d", vec!["12345678"]),
        ("%c%c", vec!["ab", " a", "a"]),
        ("%5c", vec!["abcdefg", "abc"]),
        ("%lld %I64d %hd", vec!["-5000000000 77 70000"]),
        ("%% %d", vec!["% 5", "%5", "5"]),
    ] {
        for i in inputs {
            cases.push(("sscanf", fmt.to_string(), i.to_string()));
        }
    }
    for t in decimal_texts(600, 0x2468_1357) {
        cases.push(("sscanf", "%f".to_string(), t.clone()));
        cases.push(("sscanf", "%lf".to_string(), t));
    }
    for (fmt, input) in [("%d", "5"), ("%d %d", "6 7"), ("%f", "2.5"), ("%s", "word")] {
        cases.push(("sscanf_s", fmt.to_string(), input.to_string()));
    }
    let mut fast = 0;
    for (name, fmt, input) in &cases {
        let (rust, dll) = both(&e, name);
        let s = put(&mut e, input.as_bytes());
        let f = put(&mut e, fmt.as_bytes());
        // up to six outputs 0x20 apart (and a size word 0x20 for `_s`)
        let mut words = vec![s, f];
        for k in 0..6 {
            words.push(out + 0x20 * k);
            if *name == "sscanf_s" && fmt.contains("%s") {
                words.push(0x20);
            }
        }
        let mut got = Vec::new();
        for g in [rust, dll] {
            e.mem().write_bytes(out, &[0xee; 0x100]);
            let before = e.stats.unicorn_calls;
            let r = e.call_cdecl(g, &words).unwrap();
            if g == rust && e.stats.unicorn_calls == before {
                fast += 1;
            }
            let mut b = vec![0u8; 0x100];
            e.mem().read_bytes(out, &mut b);
            got.push((r, b));
        }
        assert!(
            got[0] == got[1],
            "{name}({input:?}, {fmt:?}): {} vs {}",
            got[0].0,
            got[1].0
        );
    }
    assert!(
        fast + 300 >= cases.len(),
        "{fast} of {} in Rust",
        cases.len()
    );
    // the wide one, as Input_IsXInputDevice reads a device path
    let (rust, dll) = both(&e, "swscanf_s");
    for input in ["VID_045E&PID_028E", "VID_12", "VID_ZZ", "vid_045E"] {
        let s = put_w(&mut e, input);
        let f = put_w(&mut e, "VID_%4X");
        let mut got = Vec::new();
        for g in [rust, dll] {
            e.mem().write_bytes(out, &[0xee; 16]);
            let r = e.call_cdecl(g, &[s, f, out]).unwrap();
            let mut b = vec![0u8; 16];
            e.mem().read_bytes(out, &mut b);
            got.push((r, b));
        }
        assert_eq!(got[0], got[1], "swscanf_s({input:?})");
    }
}
