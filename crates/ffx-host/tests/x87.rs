//! The player's `MSVCR110.dll` maths in `ffx-host` (docs/host.md, "x87"):
//! its cdecl exports, the `_CI*` x87 intrinsics and the
//! `_libm_sse2_*_precise` SSE2 ones give exactly what the same DLL gave on
//! an Intel CPU (`ffx-oracle`'s `tests/data/msvcr110-math.tsv`, recorded on
//! Windows at the game's control word `0x027f`). Unicorn's own `fsin`,
//! `fpatan`, `fyl2x`, `fprem`, ... are off in the last bits; `ffx_fpu`
//! computes them. Needs `FFX_GAME_DIR`; skips without it.

use std::path::PathBuf;

use ffx_host::exec::Exec;

const MATH: &str = include_str!("../../ffx-oracle/tests/data/msvcr110-math.tsv");

fn boot() -> Option<Exec> {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return None;
    };
    let exe = PathBuf::from(dir).join("FFX.exe");
    let exec = ffx_host::boot_opts(
        &exe,
        ffx_host::table::Table::new(),
        &ffx_host::Options::default(),
    )
    .unwrap();
    assert!(
        exec.shared().dll("MSVCR110.dll").is_some(),
        "MSVCR110.dll is beside FFX.exe"
    );
    Some(exec)
}

fn hex(s: &str) -> u64 {
    u64::from_str_radix(s, 16).unwrap()
}

/// A cdecl thunk `(x_lo, x_hi, y_lo, y_hi, out)` that calls `f` the way
/// its kind takes doubles and stores its result at `out` as `fstp qword`
/// (or `movsd`) does, NaN payloads kept.
fn thunk(exec: &mut Exec, name: &str, f: u32, two: bool) -> u32 {
    let mut c = Vec::new();
    let call = |c: &mut Vec<u8>| {
        c.push(0xb8); // mov eax, f
        c.extend(f.to_le_bytes());
        c.extend([0xff, 0xd0]); // call eax
    };
    if name.starts_with("_libm_sse2") {
        // movsd xmm0, [esp+4]; movsd xmm1, [esp+12]
        c.extend([0xf2, 0x0f, 0x10, 0x44, 0x24, 0x04]);
        c.extend([0xf2, 0x0f, 0x10, 0x4c, 0x24, 0x0c]);
        call(&mut c);
        // mov ecx, [esp+20]; movsd [ecx], xmm0
        c.extend([0x8b, 0x4c, 0x24, 0x14, 0xf2, 0x0f, 0x11, 0x01]);
    } else if name.starts_with("_CI") {
        // fld qword [esp+4] (and fld qword [esp+12]: ST0 = y, ST1 = x)
        c.extend([0xdd, 0x44, 0x24, 0x04]);
        if two {
            c.extend([0xdd, 0x44, 0x24, 0x0c]);
        }
        call(&mut c);
        c.extend([0x8b, 0x44, 0x24, 0x14, 0xdd, 0x18]); // mov eax, [esp+20]; fstp qword [eax]
    } else {
        // push the four words again, call, pop them
        c.extend([0xff, 0x74, 0x24, 0x10].repeat(4));
        call(&mut c);
        c.extend([0x83, 0xc4, 0x10]);
        c.extend([0x8b, 0x44, 0x24, 0x14, 0xdd, 0x18]);
    }
    c.push(0xc3);
    exec.alloc_code(&c).unwrap()
}

#[test]
#[allow(clippy::many_single_char_names)] // n cases, p fields, x and y
fn msvcr110_maths_match_the_real_cpu() {
    let Some(mut exec) = boot() else { return };
    let exports = exec.shared().dll("MSVCR110.dll").unwrap().exports.clone();
    let out = exec.alloc(8);
    let mut thunks = std::collections::HashMap::new();
    let (mut n, mut bad) = (0, Vec::new());
    for line in MATH.lines().filter(|l| !l.starts_with('#')) {
        let p: Vec<&str> = line.split('\t').collect();
        let name = p[0];
        if name == "rand" || name == "qsort" {
            continue;
        }
        let two = p.len() == 4;
        let x = hex(p[1]);
        let y = if two { hex(p[2]) } else { 0 };
        let want = hex(p[p.len() - 1]);
        let f = *exports.get(name).unwrap_or_else(|| panic!("no {name}"));
        let at = *thunks
            .entry(name)
            .or_insert_with(|| thunk(&mut exec, name, f, two));
        #[allow(clippy::cast_possible_truncation)] // the halves
        let words = [x as u32, (x >> 32) as u32, y as u32, (y >> 32) as u32, out];
        exec.call_cdecl(at, &words).unwrap();
        let got: u64 = ffx_bridge::Mem::read(&exec.mem(), out);
        if got != want {
            bad.push(format!(
                "{name}({}) = {:e} ({got:016x}), the CPU gave {:e} ({want:016x})",
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
    // the exact instructions really ran (the DLL's maths uses them)
    assert!(
        exec.x87_steps() > 300,
        "{} x87 instructions",
        exec.x87_steps()
    );
}

/// Every site `ffx_fpu` lists for the player's builds is a real x87
/// instruction, and `FFX.exe` and `MSVCP110.dll` have none.
#[test]
fn the_players_builds_are_known() {
    let Some(exec) = boot() else { return };
    let image = &exec.image().x87;
    assert!(image.exact(), "FFX.exe is a known build");
    assert_eq!(image.at, vec![]);
    let crt = &exec.shared().dll("MSVCR110.dll").unwrap().x87;
    assert!(crt.exact(), "MSVCR110.dll is a known build");
    assert_eq!(crt.at.len(), 22);
    if let Some(p) = exec.shared().dll("MSVCP110.dll") {
        assert!(p.x87.exact(), "MSVCP110.dll is a known build");
        assert_eq!(p.x87.at, vec![]);
    }
}
