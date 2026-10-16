//! The oracle's native CRT and Win32 stubs (`crt` in the library, the
//! [`Runtime::Native`] oracle): each called through the import it is bound
//! to, the results checked against what MSVCR110 gives. `_CIsqrt` and
//! `_CIfmod` are checked through x87 code that calls them like the game
//! does. The recorded MSVCR110 output is also checked against the real
//! DLL in the oracle ([`Runtime::Dll`]). Needs `FFX_GAME_DIR`.

#![allow(clippy::many_single_char_names)] // o the oracle, a/b/s strings, n counts

use std::path::PathBuf;

use ffx_oracle::{Conv, Oracle, Runtime};

fn oracle_with(runtime: Runtime) -> Option<Oracle> {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return None;
    };
    let mut o = Oracle::load_with(PathBuf::from(dir).join("FFX.exe"), runtime).unwrap();
    o.set_fpu_control(0x027f).unwrap();
    Some(o)
}

/// The oracle with native stubs only.
fn oracle() -> Option<Oracle> {
    oracle_with(Runtime::Native)
}

/// Both: native stubs, then the real DLL.
fn both() -> Vec<(&'static str, Oracle)> {
    [("native", Runtime::Native), ("dll", Runtime::Dll)]
        .into_iter()
        .filter_map(|(n, r)| Some((n, oracle_with(r)?)))
        .collect()
}

fn stub(o: &Oracle, name: &str) -> u32 {
    o.import_stub(name)
        .unwrap_or_else(|| panic!("no import {name}"))
}

fn string(o: &mut Oracle, s: &[u8]) -> u32 {
    let mut v = s.to_vec();
    v.push(0);
    o.alloc_bytes(&v).unwrap()
}

fn read_cstr(o: &mut Oracle, p: u32) -> Vec<u8> {
    let b = o.read(p, 512).unwrap();
    b[..b.iter().position(|&c| c == 0).unwrap()].to_vec()
}

/// An argument list: words, and doubles as two words.
enum A<'a> {
    I(u32),
    S(&'a [u8]),
    D(f64),
}

fn sprintf(o: &mut Oracle, fmt: &[u8], args: &[A]) -> Vec<u8> {
    let buf = o.alloc(512).unwrap();
    let f = string(o, fmt);
    let mut words = vec![buf, f];
    for a in args {
        match a {
            A::I(v) => words.push(*v),
            A::S(s) => words.push(string(o, s)),
            A::D(d) => {
                let b = d.to_bits();
                #[allow(clippy::cast_possible_truncation)]
                words.extend([b as u32, (b >> 32) as u32]);
            }
        }
    }
    let at = stub(o, "sprintf");
    let n = o.call(at, Conv::Cdecl, &words).unwrap().eax;
    let out = read_cstr(o, buf);
    assert_eq!(n as usize, out.len(), "sprintf's count");
    out
}

#[test]
#[allow(clippy::too_many_lines)]
fn sprintf_formats_as_msvcr110() {
    let Some(mut o) = oracle() else { return };
    let cases: Vec<(&[u8], Vec<A>, &[u8])> = vec![
        (
            b"%s:%s:%d\n",
            vec![A::S(b"c001"), A::S(b"chr.c"), A::I(2898)],
            b"c001:chr.c:2898\n",
        ),
        (
            b"%d|%i|%u",
            vec![A::I(u32::MAX), A::I(7), A::I(u32::MAX)],
            b"-1|7|4294967295",
        ),
        (
            b"%5d|%-5d|%05d|%+d|% d",
            vec![A::I(42), A::I(42), A::I(42), A::I(5), A::I(5)],
            b"   42|42   |00042|+5| 5",
        ),
        (
            b"%x %X %#x %#o %o",
            vec![A::I(255), A::I(255), A::I(255), A::I(8), A::I(0)],
            b"ff FF 0xff 010 0",
        ),
        (
            b"%08x %.3d %.0d|",
            vec![A::I(0xbeef), A::I(7), A::I(0)],
            b"0000beef 007 |",
        ),
        (
            b"%c%c %5s|%-5s|%.2s",
            vec![
                A::I(u32::from(b'A')),
                A::I(u32::from(b'z')),
                A::S(b"ab"),
                A::S(b"ab"),
                A::S(b"abc"),
            ],
            b"Az    ab|ab   |ab",
        ),
        (b"%s", vec![A::I(0)], b"(null)"),
        (b"%p %%", vec![A::I(0x1234)], b"00001234 %"),
        (
            b"%hd %hu",
            vec![A::I(0x1_8000), A::I(0x1_8000)],
            b"-32768 32768",
        ),
        (
            b"%lld %I64x",
            vec![A::I(0xffff_fffe), A::I(0xffff_ffff), A::I(0), A::I(1)],
            b"-2 100000000",
        ),
        (b"%f", vec![A::D(1.5)], b"1.500000"),
        (
            b"%5.2f|%-8.3f|%08.3f",
            vec![A::D(3.142_59), A::D(2.5), A::D(-1.5)],
            b" 3.14|2.500   |-001.500",
        ),
        (
            b"%.0f %.0f %.0f",
            vec![A::D(0.5), A::D(2.5), A::D(9.5)],
            b"1 3 10",
        ),
        (b"%.1f", vec![A::D(0.05)], b"0.1"),
        (
            b"%e|%E|%.2e",
            vec![A::D(12345.678), A::D(0.000_123), A::D(-1.0)],
            b"1.234568e+004|1.230000E-004|-1.00e+000",
        ),
        (
            b"%g|%g|%g|%g|%G",
            vec![
                A::D(0.0001),
                A::D(0.00001),
                A::D(100_000.0),
                A::D(1_000_000.0),
                A::D(1e-10),
            ],
            b"0.0001|1e-005|100000|1e+006|1E-010",
        ),
        (
            b"%g|%#g|%.3g",
            vec![A::D(1.5), A::D(1.5), A::D(3.142_59)],
            b"1.5|1.50000|3.14",
        ),
        (b"%.20f", vec![A::D(0.1)], b"0.10000000000000001000"),
        (b"%f", vec![A::D(1e20)], b"100000000000000000000.000000"),
        (
            b"%.1f",
            vec![A::D(123_456_789_012_345_678_901.0)],
            b"123456789012345680000.0",
        ),
        (
            b"%f|%.2f|%f",
            vec![
                A::D(f64::INFINITY),
                A::D(f64::INFINITY),
                A::D(f64::NEG_INFINITY),
            ],
            b"1.#INF00|1.#J|-1.#INF00",
        ),
        (
            b"%f|%f",
            vec![A::D(f64::NAN), A::D(f64::from_bits(0xfff8_0000_0000_0000))],
            b"1.#QNAN0|-1.#IND00",
        ),
        (
            b"%*d|%-*d|%.*f",
            vec![A::I(4), A::I(7), A::I(3), A::I(7), A::I(2), A::D(1.0)],
            b"   7|7  |1.00",
        ),
        (
            b"radian value error =%f",
            vec![A::D(70.0)],
            b"radian value error =70.000000",
        ),
    ];
    for (fmt, args, want) in cases {
        let got = sprintf(&mut o, fmt, &args);
        assert_eq!(
            String::from_utf8_lossy(&got),
            String::from_utf8_lossy(want),
            "{}",
            String::from_utf8_lossy(fmt)
        );
    }
}

#[test]
fn counted_and_va_list_printf() {
    let Some(mut o) = oracle() else { return };
    let buf = o.alloc(32).unwrap();
    let fmt = string(&mut o, b"%d-%d");
    let snp = stub(&o, "_snprintf");
    // fits with room: NUL; exactly: no NUL; too long: -1
    o.write(buf, &[0xaa; 16]).unwrap();
    assert_eq!(
        o.call(snp, Conv::Cdecl, &[buf, 8, fmt, 12, 34])
            .unwrap()
            .eax,
        5
    );
    assert_eq!(o.read(buf, 6).unwrap(), b"12-34\0");
    o.write(buf, &[0xaa; 16]).unwrap();
    assert_eq!(
        o.call(snp, Conv::Cdecl, &[buf, 5, fmt, 12, 34])
            .unwrap()
            .eax,
        5
    );
    assert_eq!(o.read(buf, 6).unwrap(), b"12-34\xaa");
    o.write(buf, &[0xaa; 16]).unwrap();
    assert_eq!(
        o.call(snp, Conv::Cdecl, &[buf, 3, fmt, 12, 34])
            .unwrap()
            .eax,
        u32::MAX
    );
    assert_eq!(o.read(buf, 4).unwrap(), b"12-\xaa");
    // va_list forms
    let va = o
        .alloc_bytes(&[12u32.to_le_bytes(), 34u32.to_le_bytes()].concat())
        .unwrap();
    let vsp = stub(&o, "vsprintf");
    assert_eq!(o.call(vsp, Conv::Cdecl, &[buf, fmt, va]).unwrap().eax, 5);
    assert_eq!(read_cstr(&mut o, buf), b"12-34");
    let vsnprintf = stub(&o, "_vsnprintf");
    assert_eq!(
        o.call(vsnprintf, Conv::Cdecl, &[buf, 3, fmt, va])
            .unwrap()
            .eax,
        u32::MAX
    );
    let vsnps = stub(&o, "_vsnprintf_s");
    assert_eq!(
        o.call(vsnps, Conv::Cdecl, &[buf, 4, u32::MAX, fmt, va])
            .unwrap()
            .eax,
        u32::MAX
    );
    assert_eq!(read_cstr(&mut o, buf), b"12-");
    let sps = stub(&o, "sprintf_s");
    assert_eq!(
        o.call(sps, Conv::Cdecl, &[buf, 32, fmt, 1, 2]).unwrap().eax,
        3
    );
    assert_eq!(read_cstr(&mut o, buf), b"1-2");
}

#[test]
fn string_functions() {
    let Some(mut o) = oracle() else { return };
    let hay = string(&mut o, b"..\\proj\\src\\chr\\chr.c");
    let needle = string(&mut o, b"chr");
    let none = string(&mut o, b"xyz");
    let empty = string(&mut o, b"");
    let call = |o: &mut Oracle, name: &str, args: &[u32]| {
        let at = stub(o, name);
        o.call(at, Conv::Cdecl, args).unwrap().eax
    };
    assert_eq!(call(&mut o, "strstr", &[hay, needle]), hay + 12);
    assert_eq!(call(&mut o, "strstr", &[hay, none]), 0);
    assert_eq!(call(&mut o, "strstr", &[hay, empty]), hay);
    assert_eq!(call(&mut o, "strchr", &[hay, u32::from(b'c')]), hay + 10);
    assert_eq!(call(&mut o, "strrchr", &[hay, u32::from(b'c')]), hay + 20);
    assert_eq!(call(&mut o, "strchr", &[hay, 0]), hay + 21);
    assert_eq!(call(&mut o, "strpbrk", &[hay, needle]), hay + 4);
    let (a, b) = (string(&mut o, b"Abc"), string(&mut o, b"abD"));
    assert_eq!(call(&mut o, "_stricmp", &[a, b]).cast_signed(), -1);
    assert_eq!(call(&mut o, "_strnicmp", &[a, b, 2]), 0);
    assert_eq!(call(&mut o, "strncmp", &[a, b, 1]).cast_signed(), -1);
    assert_eq!(call(&mut o, "strncmp", &[b, b, 9]), 0);
    let d = call(&mut o, "_strdup", &[hay]);
    assert_ne!(d, hay);
    assert_eq!(read_cstr(&mut o, d), b"..\\proj\\src\\chr\\chr.c");
    let buf = o.alloc(16).unwrap();
    o.write(buf, &[0xaa; 16]).unwrap();
    assert_eq!(call(&mut o, "strncpy", &[buf, a, 6]), buf);
    assert_eq!(o.read(buf, 7).unwrap(), b"Abc\0\0\0\xaa");
    assert_eq!(call(&mut o, "strncat", &[buf, b, 2]), buf);
    assert_eq!(read_cstr(&mut o, buf), b"Abcab");
    assert_eq!(call(&mut o, "_strlwr", &[a]), a);
    assert_eq!(read_cstr(&mut o, a), b"abc");
    assert_eq!(call(&mut o, "memchr", &[hay, u32::from(b'p'), 21]), hay + 3);
    assert_eq!(call(&mut o, "memchr", &[hay, u32::from(b'p'), 3]), 0);
    assert_eq!(call(&mut o, "tolower", &[u32::from(b'Q')]), u32::from(b'q'));
    assert_eq!(call(&mut o, "toupper", &[u32::from(b'q')]), u32::from(b'Q'));
    assert_eq!(call(&mut o, "toupper", &[0xe9]), 0xe9);
    assert_eq!(call(&mut o, "isalpha", &[u32::from(b'a')]), 2);
    assert_eq!(call(&mut o, "isdigit", &[u32::from(b'5')]), 4);
    assert_eq!(call(&mut o, "isspace", &[u32::from(b'\t')]), 8);
    let num = string(&mut o, b"  -0x1Fz");
    let end = o.alloc(4).unwrap();
    assert_eq!(call(&mut o, "strtol", &[num, end, 0]), 0xffff_ffe1);
    assert_eq!(o.read(end, 4).unwrap(), (num + 7).to_le_bytes());
    let n2 = string(&mut o, b"123abc");
    assert_eq!(call(&mut o, "atoi", &[n2]), 123);
    assert_eq!(call(&mut o, "abs", &[(-5i32).cast_unsigned()]), 5);
    let s = string(&mut o, b"x");
    let dst = o.alloc(4).unwrap();
    assert_eq!(call(&mut o, "strcpy_s", &[dst, 4, s]), 0);
    assert_eq!(call(&mut o, "strcat_s", &[dst, 4, s]), 0);
    assert_eq!(read_cstr(&mut o, dst), b"xx");
    assert_eq!(call(&mut o, "strcat_s", &[dst, 3, s]), 34);
}

#[test]
fn float_functions() {
    let Some(mut o) = oracle() else { return };
    let words = |d: f64| {
        let b = d.to_bits();
        #[allow(clippy::cast_possible_truncation)]
        [b as u32, (b >> 32) as u32]
    };
    for x in [0.0, -0.0, 1.5, -1.5, 2.0, -2.5, 1e300, f64::INFINITY, 0.1] {
        for (name, f) in [("floor", f64::floor as fn(f64) -> f64), ("ceil", f64::ceil)] {
            let at = stub(&o, name);
            let r = o.call(at, Conv::Cdecl, &words(x)).unwrap();
            assert_eq!(r.st0.unwrap().to_bits(), f(x).to_bits(), "{name}({x})");
        }
    }
    for (x, fin, nan) in [(1.0, 1, 0), (f64::INFINITY, 0, 0), (f64::NAN, 0, 1)] {
        let at = stub(&o, "_finite");
        assert_eq!(o.call(at, Conv::Cdecl, &words(x)).unwrap().eax, fin);
        let at = stub(&o, "_isnan");
        assert_eq!(o.call(at, Conv::Cdecl, &words(x)).unwrap().eax, nan);
    }
    let atof = stub(&o, "atof");
    let s = string(&mut o, b" -12.5e-1xyz");
    assert_eq!(o.call(atof, Conv::Cdecl, &[s]).unwrap().st0, Some(-1.25));
    // _CIsqrt through the game's sqrtf wrapper (0x00452940: fld, call
    // _CIsqrt, fstp f32): fsqrt at 53 bits, then the f32 store
    for bits in [
        0x4000_0000u32,
        0x3f80_0000,
        0x4049_0fdb,
        0x0000_0001,
        0x7f7f_ffff,
        0x4110_0000,
    ] {
        let r = o.call(0x0045_2940, Conv::Cdecl, &[bits]).unwrap();
        #[allow(clippy::cast_possible_truncation)]
        let want = f64::from(f32::from_bits(bits)).sqrt() as f32;
        assert_eq!(r.st0.unwrap(), f64::from(want), "sqrtf({bits:#x})");
    }
    // _CIfmod (x in ST1, y in ST0) through a few bytes of x87 code put
    // in padding of the image: fld x; fld y; call 0x009498e0; ret
    let cave = 0x0083_9d61;
    let mut code = vec![0xd9, 0x44, 0x24, 0x04, 0xd9, 0x44, 0x24, 0x08, 0xe8];
    let rel = 0x0094_98e0u32.wrapping_sub(cave + 13);
    code.extend(rel.to_le_bytes());
    code.push(0xc3);
    o.write(cave, &code).unwrap();
    for (x, y) in [
        (7.5f32, 2.0f32),
        (-7.5, 2.0),
        (1e30, 3.0),
        (5.0, 0.0),
        (0.0, 1.0),
    ] {
        let r = o
            .call(cave, Conv::Cdecl, &[x.to_bits(), y.to_bits()])
            .unwrap();
        let want = f64::from(x) % f64::from(y);
        let got = r.st0.unwrap();
        assert!(
            got.to_bits() == want.to_bits() || (got.is_nan() && want.is_nan()),
            "fmod({x}, {y}) = {got}"
        );
    }
}

#[test]
fn rand_and_locks() {
    let Some(mut o) = oracle() else { return };
    let rand = stub(&o, "rand");
    let srand = stub(&o, "srand");
    let seq: Vec<u32> = (0..3)
        .map(|_| o.call(rand, Conv::Cdecl, &[]).unwrap().eax)
        .collect();
    assert_eq!(seq, [41, 18467, 6334]);
    o.call(srand, Conv::Cdecl, &[1]).unwrap();
    assert_eq!(o.call(rand, Conv::Cdecl, &[]).unwrap().eax, 41);
    let cs = o.alloc(24).unwrap();
    for name in [
        "InitializeCriticalSection",
        "EnterCriticalSection",
        "LeaveCriticalSection",
        "DeleteCriticalSection",
    ] {
        let at = stub(&o, name);
        o.call(at, Conv::Stdcall, &[cs]).unwrap();
    }
    let at = stub(&o, "InitializeCriticalSectionEx");
    assert_eq!(o.call(at, Conv::Stdcall, &[cs, 0, 0]).unwrap().eax, 1);
    let at = stub(&o, "GetCurrentThreadId");
    assert_eq!(o.call(at, Conv::Stdcall, &[]).unwrap().eax, 1);
}

#[test]
fn missing_imports_by_callers() {
    let Some(o) = oracle() else { return };
    let missing = o.missing_imports();
    assert!(
        !missing
            .iter()
            .any(|m| m.ends_with("!sprintf") || m.ends_with("!strstr"))
    );
    eprintln!("{} imports have no stub", missing.len());
}

/// Every line of `data/msvcr110-sprintf.tsv` (what the game's own
/// `msvcr110.dll` printed, run on Windows) through the `sprintf` stub.
#[test]
fn sprintf_matches_recorded_msvcr110_output() {
    for (runtime, mut o) in both() {
        sprintf_recorded(runtime, &mut o);
    }
}

fn sprintf_recorded(runtime: &str, o: &mut Oracle) {
    let sp = stub(o, "sprintf");
    let buf = o.alloc(1024).unwrap();
    let abc = string(o, b"abc");
    let mut n = 0;
    for line in include_str!("data/msvcr110-sprintf.tsv").lines() {
        if line.starts_with('#') {
            continue;
        }
        let p: Vec<&str> = line.splitn(3, '\t').collect();
        let (fmt, arg, want) = (p[0], p[1], p[2]);
        let f = string(o, fmt.as_bytes());
        let args: Vec<u32> = match arg {
            "abc" => vec![buf, f, abc],
            "null" => vec![buf, f, 0],
            "-" => vec![buf, f, 0xffff_fffe, 0xffff_ffff, 0, 1],
            a if a.len() == 16 => {
                let b = u64::from_str_radix(a, 16).unwrap();
                #[allow(clippy::cast_possible_truncation)]
                let w = [buf, f, b as u32, (b >> 32) as u32];
                w.to_vec()
            }
            a => vec![buf, f, u32::from_str_radix(a, 16).unwrap()],
        };
        o.call(sp, Conv::Cdecl, &args).unwrap();
        let got = read_cstr(o, buf);
        assert_eq!(
            String::from_utf8_lossy(&got),
            want,
            "{runtime}: {fmt} of {arg}"
        );
        n += 1;
    }
    assert!(n > 700);
}

/// `atof` and `strtol` against values the game's `msvcr110.dll` returned.
#[test]
fn numbers_match_recorded_msvcr110_output() {
    for (runtime, mut o) in both() {
        numbers_recorded(runtime, &mut o);
    }
}

fn numbers_recorded(runtime: &str, o: &mut Oracle) {
    let atof = stub(o, "atof");
    for (text, bits) in [
        ("1.5", 0x3ff8_0000_0000_0000u64),
        (" -12.5e-1xyz", 0xbff4_0000_0000_0000),
        ("0.1", 0x3fb9_9999_9999_999a),
        ("3.14159265358979323846", 0x4009_21fb_5444_2d18),
        ("1e308", 0x7fe1_ccf3_85eb_c8a0),
        ("1e309", 0x7ff0_0000_0000_0000),
        ("2.2250738585072011e-308", 0x000f_ffff_ffff_ffff),
        ("4.9e-324", 0x0000_0000_0000_0001),
        ("123456789012345678901234567890", 0x45f8_ee90_ff6c_373e),
        ("0.30000000000000004", 0x3fd3_3333_3333_3334),
        ("9007199254740993", 0x4340_0000_0000_0000),
        ("1.7976931348623158e308", 0x7fef_ffff_ffff_ffff),
        ("abc", 0),
        ("  +.5", 0x3fe0_0000_0000_0000),
        ("5.", 0x4014_0000_0000_0000),
        ("1e-400", 0),
        ("0x10", 0),
    ] {
        let s = string(o, text.as_bytes());
        let r = o.call(atof, Conv::Cdecl, &[s]).unwrap();
        assert_eq!(r.st0.unwrap().to_bits(), bits, "{runtime}: atof({text})");
    }
    let strtol = stub(o, "strtol");
    let end = o.alloc(4).unwrap();
    for (text, base, v, used) in [
        ("  -0x1Fz", 0, -31, 7),
        ("077", 0, 63, 3),
        ("99999999999", 10, 2_147_483_647, 11),
        ("-99999999999", 10, -2_147_483_648, 12),
        ("x", 10, 0, 0),
        ("0x", 16, 0, 0),
        ("0xg", 0, 0, 0),
        ("  +12", 10, 12, 5),
        ("z", 36, 35, 1),
        ("-2147483648", 10, -2_147_483_648, 11),
    ] {
        let s = string(o, text.as_bytes());
        let r = o.call(strtol, Conv::Cdecl, &[s, end, base]).unwrap();
        assert_eq!(r.eax.cast_signed(), v, "{runtime}: strtol({text}, {base})");
        let e: u32 = u32::from_le_bytes(o.read(end, 4).unwrap().try_into().unwrap());
        assert_eq!(e - s, used, "strtol({text}, {base}) end");
    }
}

/// KERNEL32's threading imports on one thread: each through its import,
/// stdcall (`Oracle::call` checks the callee popped its arguments).
#[test]
fn threading_stubs() {
    let Some(mut o) = oracle() else { return };
    let call = |o: &mut Oracle, name: &str, args: &[u32]| {
        let at = stub(o, name);
        o.call(at, Conv::Stdcall, args).unwrap().eax
    };
    // fresh, distinct handles
    let m = call(&mut o, "CreateMutexW", &[0, 0, 0]);
    let e = call(&mut o, "CreateEventW", &[0, 1, 0, 0]);
    let ea = call(&mut o, "CreateEventA", &[0, 0, 0, 0]);
    let s = call(&mut o, "CreateSemaphoreW", &[0, 2, 10, 0]);
    let sa = call(&mut o, "CreateSemaphoreA", &[0, 0, 1, 0]);
    let handles = [m, e, ea, s, sa];
    for (k, h) in handles.iter().enumerate() {
        assert_ne!(*h, 0);
        assert!(!handles[..k].contains(h), "{h:#x} handed out twice");
    }
    // waits are signalled, releases and event changes succeed
    assert_eq!(call(&mut o, "WaitForSingleObject", &[m, u32::MAX]), 0);
    assert_eq!(call(&mut o, "WaitForSingleObjectEx", &[e, 0, 1]), 0);
    assert_eq!(call(&mut o, "ReleaseMutex", &[m]), 1);
    assert_eq!(call(&mut o, "SetEvent", &[e]), 1);
    assert_eq!(call(&mut o, "ResetEvent", &[e]), 1);
    // the semaphore keeps its count: 2, a wait takes one, release adds 3
    let prev = o.alloc(4).unwrap();
    assert_eq!(call(&mut o, "WaitForSingleObject", &[s, 0]), 0);
    assert_eq!(call(&mut o, "ReleaseSemaphore", &[s, 3, prev]), 1);
    assert_eq!(ffx_bridge::Mem::read::<u32>(&o, prev), 1);
    assert_eq!(call(&mut o, "ReleaseSemaphore", &[s, 1, 0]), 1);
    assert_eq!(call(&mut o, "ReleaseSemaphore", &[s, 1, prev]), 1);
    assert_eq!(ffx_bridge::Mem::read::<u32>(&o, prev), 5);
    for h in handles {
        assert_eq!(call(&mut o, "CloseHandle", &[h]), 1);
    }
    // interlocked read-modify-writes return what the API returns
    let v = o.alloc(4).unwrap();
    ffx_bridge::Mem::write(&mut o, v, 5u32);
    assert_eq!(call(&mut o, "InterlockedIncrement", &[v]), 6);
    assert_eq!(call(&mut o, "InterlockedDecrement", &[v]), 5);
    assert_eq!(call(&mut o, "InterlockedExchangeAdd", &[v, 10]), 5);
    assert_eq!(ffx_bridge::Mem::read::<u32>(&o, v), 15);
    assert_eq!(call(&mut o, "InterlockedCompareExchange", &[v, 1, 7]), 15);
    assert_eq!(ffx_bridge::Mem::read::<u32>(&o, v), 15);
    assert_eq!(call(&mut o, "InterlockedCompareExchange", &[v, 1, 15]), 15);
    assert_eq!(ffx_bridge::Mem::read::<u32>(&o, v), 1);
    ffx_bridge::Mem::write(&mut o, v, 0u32);
    assert_eq!(call(&mut o, "InterlockedDecrement", &[v]), u32::MAX);
    // sleeps return, the last error is kept
    call(&mut o, "Sleep", &[1000]);
    assert_eq!(call(&mut o, "SleepEx", &[0, 1]), 0);
    assert_eq!(call(&mut o, "GetLastError", &[]), 0);
    call(&mut o, "SetLastError", &[183]);
    assert_eq!(call(&mut o, "GetLastError", &[]), 183);
    assert_eq!(call(&mut o, "GetCurrentThreadId", &[]), 1);
    // none of these is missing any more
    for name in [
        "WaitForSingleObject",
        "WaitForSingleObjectEx",
        "ReleaseMutex",
        "CreateMutexW",
        "CloseHandle",
        "InterlockedCompareExchange",
        "Sleep",
        "SetLastError",
    ] {
        assert!(
            !o.missing_imports()
                .iter()
                .any(|m| m.ends_with(&format!("!{name}"))),
            "{name} still missing"
        );
    }
}

/// `sprintf_s` into too small a buffer (the oracle's invalid-parameter
/// handler returns; the game has none and would end): MSVCR110 writes what
/// fits and a NUL, then clears the first byte, and returns -1.
#[test]
fn sprintf_s_overflow_as_msvcr110() {
    for (runtime, mut o) in both() {
        let f = string(&mut o, b"%s%s");
        let x = string(&mut o, b"ab");
        let y = string(&mut o, b"cdefghijk");
        let at = stub(&o, "sprintf_s");
        for (size, ret, want) in [
            (8u32, -1, &b"\0bcdefg\0\x55\x55"[..]),
            (1, -1, b"\0\x55\x55\x55\x55\x55\x55\x55\x55\x55"),
            (11, -1, b"\0bcdefghij\0"),
            (12, 11, b"abcdefghijk\0"),
        ] {
            let buf = o.alloc(16).unwrap();
            o.write(buf, &[0x55; 16]).unwrap();
            let r = o.call(at, Conv::Cdecl, &[buf, size, f, x, y]).unwrap().eax;
            assert_eq!(r.cast_signed(), ret, "{runtime}: size {size}");
            assert_eq!(
                o.read(buf, want.len()).unwrap(),
                want,
                "{runtime}: size {size}"
            );
        }
    }
}
