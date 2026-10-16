//! The game's own `MSVCR110.dll` running in the oracle ([`Runtime::Dll`]):
//! its math functions (the cdecl exports, the `_CI*` x87 intrinsics and the
//! `_libm_sse2_*_precise` SSE2 ones), `rand` and `qsort` (with a comparator
//! from `FFX.exe`) give exactly what the same DLL gave on Windows
//! (`data/msvcr110-math.tsv`). Needs `FFX_GAME_DIR`.

#![allow(clippy::many_single_char_names)] // o the oracle, p fields, n counts

use std::path::PathBuf;

use ffx_oracle::{Conv, Oracle, Pages, Runtime};

fn oracle() -> Option<Oracle> {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return None;
    };
    let o = Oracle::load_with(PathBuf::from(dir).join("FFX.exe"), Runtime::Dll).unwrap();
    assert!(o.runtime_base().is_some(), "MSVCR110.dll is beside FFX.exe");
    Some(o)
}

/// The image-only oracle, and one on `FFX_SNAPSHOT` when it is set (the
/// runtime loaded where the snapshot leaves room).
fn oracles() -> Vec<(&'static str, Oracle)> {
    let Some(o) = oracle() else { return Vec::new() };
    let mut out = vec![("image", o)];
    if let Some(snap) = std::env::var_os("FFX_SNAPSHOT").filter(|s| !s.is_empty()) {
        let exe = PathBuf::from(std::env::var_os("FFX_GAME_DIR").unwrap()).join("FFX.exe");
        let pages = Pages::read(PathBuf::from(snap)).unwrap();
        let o = Oracle::from_snapshot_with(&exe, &pages, Runtime::Dll).unwrap();
        assert!(o.runtime_base().is_some());
        out.push(("snapshot", o));
    }
    out
}

/// What `fstp qword` stores for an x87 register: the f64 bits, NaN
/// payloads kept.
fn st0_bits(raw: [u8; 10]) -> u64 {
    let mant = u64::from_le_bytes(raw[..8].try_into().unwrap());
    let se = u16::from_le_bytes([raw[8], raw[9]]);
    let sign = u64::from(se >> 15) << 63;
    if se & 0x7fff == 0x7fff {
        // infinities and NaNs: the top 52 fraction bits
        return sign | (0x7ff << 52) | ((mant << 1) >> 12);
    }
    let r = ffx_oracle::f80_to_f64(&raw);
    r.to_bits()
}

fn words(x: f64) -> [u32; 2] {
    let b = x.to_bits();
    #[allow(clippy::cast_possible_truncation)]
    [b as u32, (b >> 32) as u32]
}

fn hex(s: &str) -> u64 {
    u64::from_str_radix(s, 16).unwrap()
}

/// `fld qword [esp+4]; (fld qword [esp+12];) mov eax, f; call eax; ret`
fn x87_thunk(o: &mut Oracle, f: u32, two: bool) -> u32 {
    let mut c = vec![0xdd, 0x44, 0x24, 0x04];
    if two {
        c.extend([0xdd, 0x44, 0x24, 0x0c]);
    }
    c.push(0xb8);
    c.extend(f.to_le_bytes());
    c.extend([0xff, 0xd0, 0xc3]);
    o.alloc_code(&c).unwrap()
}

/// `movsd xmm0, [esp+4]; movsd xmm1, [esp+12]; mov eax, f; call eax;
/// mov ecx, [esp+20]; movsd [ecx], xmm0; ret`
fn sse2_thunk(o: &mut Oracle, f: u32) -> u32 {
    let mut c = vec![
        0xf2, 0x0f, 0x10, 0x44, 0x24, 0x04, 0xf2, 0x0f, 0x10, 0x4c, 0x24, 0x0c, 0xb8,
    ];
    c.extend(f.to_le_bytes());
    c.extend([
        0xff, 0xd0, 0x8b, 0x4c, 0x24, 0x14, 0xf2, 0x0f, 0x11, 0x01, 0xc3,
    ]);
    o.alloc_code(&c).unwrap()
}

#[test]
fn math_matches_recorded_msvcr110_output() {
    for (_, mut o) in oracles() {
        math_matches_recorded_msvcr110_output_on(&mut o);
    }
}

fn math_matches_recorded_msvcr110_output_on(o: &mut Oracle) {
    let out = o.alloc(8).unwrap();
    let mut thunks = std::collections::HashMap::new();
    let mut n = 0;
    let mut bad = Vec::new();
    for line in include_str!("data/msvcr110-math.tsv").lines() {
        if line.starts_with('#') {
            continue;
        }
        let p: Vec<&str> = line.split('\t').collect();
        let name = p[0];
        if name == "rand" || name == "qsort" {
            continue;
        }
        let two = p.len() == 4;
        let x = f64::from_bits(hex(p[1]));
        let y = if two { f64::from_bits(hex(p[2])) } else { 0.0 };
        let want = hex(p[p.len() - 1]);
        let f = o
            .runtime_export(name)
            .unwrap_or_else(|| panic!("no {name}"));
        let mut args = words(x).to_vec();
        if two {
            args.extend(words(y));
        }
        let got = if name.starts_with("_libm_sse2") {
            let t = *thunks.entry(name).or_insert_with(|| sse2_thunk(o, f));
            if !two {
                args.extend([0, 0]);
            }
            args.push(out);
            o.call(t, Conv::Cdecl, &args).unwrap();
            u64::from_le_bytes(o.read(out, 8).unwrap().try_into().unwrap())
        } else {
            let at = if name.starts_with("_CI") {
                *thunks.entry(name).or_insert_with(|| x87_thunk(o, f, two))
            } else {
                f
            };
            let r = o.call(at, Conv::Cdecl, &args).unwrap();
            st0_bits(r.st0_raw.expect("a result in ST0"))
        };
        if got != want {
            bad.push(format!(
                "{name}({}) = {:e} ({got:016x}), Windows gave {:e} ({want:016x})",
                p[1..p.len() - 1].join(", "),
                f64::from_bits(got),
                f64::from_bits(want)
            ));
        }
        n += 1;
    }
    assert!(n > 1200, "{n} cases");
    assert!(
        bad.is_empty(),
        "{} of {n} differ:\n{}",
        bad.len(),
        bad.join("\n")
    );
}

#[test]
fn rand_matches_recorded_msvcr110_output() {
    for (_, mut o) in oracles() {
        rand_matches_recorded_msvcr110_output_on(&mut o);
    }
}

fn rand_matches_recorded_msvcr110_output_on(o: &mut Oracle) {
    let rand = o.import_stub("rand").unwrap();
    let srand = o.import_stub("srand").unwrap();
    assert_eq!(Some(rand), o.runtime_export("rand"), "bound to the DLL");
    for line in include_str!("data/msvcr110-math.tsv").lines() {
        let p: Vec<&str> = line.split('\t').collect();
        if p[0] != "rand" {
            continue;
        }
        let seed: u32 = p[1].parse().unwrap();
        if seed != 1 {
            o.call(srand, Conv::Cdecl, &[seed]).unwrap();
        }
        let got: Vec<String> = (0..12)
            .map(|_| o.call(rand, Conv::Cdecl, &[]).unwrap().eax.to_string())
            .collect();
        assert_eq!(got.join(" "), p[2], "rand after seed {seed}");
    }
}

/// `qsort` with `FFX.exe`'s int comparator `FUN_00529e50` (`*a - *b`) over
/// (key, id) pairs: equal keys come out in the order MSVCR110's algorithm
/// leaves them.
#[test]
fn qsort_matches_recorded_msvcr110_output() {
    for (_, mut o) in oracles() {
        qsort_matches_recorded_msvcr110_output_on(&mut o);
    }
}

fn qsort_matches_recorded_msvcr110_output_on(o: &mut Oracle) {
    let qsort = o.import_stub("qsort").unwrap();
    assert_eq!(Some(qsort), o.runtime_export("qsort"), "bound to the DLL");
    let compare = o.rebase(0x0052_9e50);
    let mut cases = 0;
    for line in include_str!("data/msvcr110-math.tsv").lines() {
        let p: Vec<&str> = line.split('\t').collect();
        if p[0] != "qsort" {
            continue;
        }
        let (n, keys, seed): (u32, u32, u32) = (
            p[1].parse().unwrap(),
            p[2].parse().unwrap(),
            p[3].parse().unwrap(),
        );
        let mut s = seed.wrapping_mul(0x9e37_79b9) | 1;
        let mut data = Vec::new();
        for i in 0..n {
            s ^= s << 13;
            s ^= s >> 17;
            s ^= s << 5;
            data.extend((s % keys).to_le_bytes());
            data.extend(i.to_le_bytes());
        }
        let at = o.alloc_bytes(&data).unwrap();
        o.call(qsort, Conv::Cdecl, &[at, n, 8, compare]).unwrap();
        let sorted = o.read(at, 8 * n as usize).unwrap();
        let ids: Vec<String> = sorted
            .chunks(8)
            .map(|e| u32::from_le_bytes(e[4..8].try_into().unwrap()).to_string())
            .collect();
        assert_eq!(ids.join(" "), p[4].trim_end(), "qsort of {n}");
        cases += 1;
    }
    assert_eq!(cases, 5);
}

fn string(o: &mut Oracle, s: &str) -> u32 {
    let mut v = s.as_bytes().to_vec();
    v.push(0);
    o.alloc_bytes(&v).unwrap()
}

/// The imports the game calls most that no native stub covered: stdio,
/// time, environment through the DLL; Win32 waits, locks and handles as
/// single-threaded success.
#[test]
fn common_imports_run() {
    for (_, mut o) in oracles() {
        common_imports_run_on(&mut o);
    }
}

fn common_imports_run_on(o: &mut Oracle) {
    let call = |o: &mut Oracle, name: &str, conv, args: &[u32]| {
        let at = o.import_stub(name).unwrap_or_else(|| panic!("no {name}"));
        o.call(at, conv, args).unwrap().eax
    };
    // printf and fprintf(stdout) count what they write (the output is
    // discarded)
    let fmt = string(o, "%s=%d\n");
    let s = string(o, "hp");
    assert_eq!(call(o, "printf", Conv::Cdecl, &[fmt, s, 1234]), 8);
    let iob = call(o, "__iob_func", Conv::Cdecl, &[]);
    let stdout = iob + 0x20; // _iob[1], 32-byte FILEs
    assert_eq!(call(o, "fprintf", Conv::Cdecl, &[stdout, fmt, s, 5]), 5);
    assert_eq!(call(o, "fflush", Conv::Cdecl, &[stdout]), 0);
    // the fixed clock: 2013-12-26 00:00 UTC
    let t = call(o, "_time64", Conv::Cdecl, &[0]);
    assert_eq!(t, 1_388_016_000);
    let name = string(o, "PATH");
    assert_eq!(call(o, "getenv", Conv::Cdecl, &[name]), 0);
    let m = call(o, "CreateMutexW", Conv::Stdcall, &[0, 0, 0]);
    assert_ne!(m, 0);
    assert_eq!(
        call(o, "WaitForSingleObject", Conv::Stdcall, &[m, u32::MAX]),
        0
    );
    assert_eq!(call(o, "ReleaseMutex", Conv::Stdcall, &[m]), 1);
    assert_eq!(call(o, "CloseHandle", Conv::Stdcall, &[m]), 1);
    let v = o.alloc(4).unwrap();
    o.write(v, &5u32.to_le_bytes()).unwrap();
    assert_eq!(call(o, "InterlockedExchangeAdd", Conv::Stdcall, &[v, 3]), 5);
    assert_eq!(
        call(o, "InterlockedCompareExchange", Conv::Stdcall, &[v, 1, 8]),
        8
    );
    assert_eq!(o.read(v, 4).unwrap(), 1u32.to_le_bytes());
    assert_eq!(call(o, "GetLastError", Conv::Stdcall, &[]), 0);
    assert_eq!(call(o, "Sleep", Conv::Stdcall, &[10]), 0);
}

/// A failed assertion stops the call with its message.
#[test]
fn wassert_stops_the_call() {
    let Some(mut o) = oracle() else { return };
    let w = |s: &str| -> Vec<u8> {
        s.encode_utf16()
            .chain([0])
            .flat_map(u16::to_le_bytes)
            .collect()
    };
    let msg = o.alloc_bytes(&w("p != NULL")).unwrap();
    let file = o.alloc_bytes(&w("chr.c")).unwrap();
    let at = o.import_stub("_wassert").unwrap();
    let e = o.call(at, Conv::Cdecl, &[msg, file, 42]).unwrap_err();
    assert_eq!(
        e.to_string(),
        "assertion failed in the emulated code: p != NULL (chr.c:42)"
    );
}

/// The imports still without anything to run, for the report.
#[test]
fn missing_imports() {
    let Some(o) = oracle() else { return };
    let missing = o.missing_imports();
    assert!(
        !missing
            .iter()
            .any(|m| m.to_ascii_lowercase().starts_with("msvcr110"))
    );
    for m in missing {
        println!("missing\t{m}");
    }
}
