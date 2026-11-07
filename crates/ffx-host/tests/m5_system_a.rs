//! The `host_only!` entries of M5 list `system-a` (`code_0062f640`,
//! `code_0062f980`, `code_0062fb00`, and tail9's
//! `YiWindowHandler_idle_00907e50`: `xor eax, eax; ret`, three bytes each;
//! docs/porting.md, "Functions under five bytes") through ffx-host's
//! function table: the 32-bit game has no hook for them, so this is where
//! their table entries run. Each is called from Unicorn code the way its
//! callers call it (cdecl, no arguments, `call`), once with the ported
//! table (the entry must run in Rust) and once with no table (the
//! original's bytes); `eax`, the caller's `ecx`/`edx` after the call and
//! its stack must agree. Needs `FFX_GAME_DIR`; skips without it.

use std::path::PathBuf;

use ffx_bridge::Mem as _;
use ffx_host::exec::Exec;
use ffx_host::table::{Conv, Table};

const ZEROS: [u32; 4] = [0x0062_f640, 0x0062_f980, 0x0062_fb00, 0x0090_7e50];

fn exe() -> Option<PathBuf> {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return None;
    };
    Some(PathBuf::from(dir).join("FFX.exe"))
}

/// Calls `f` from code in the arena with `eax`, `ecx` and `edx` preset;
/// after the call `ecx`, `edx`, `esp` go to `out`, `out + 4`, `out + 16`
/// (`out + 20`: `esp` before). Returns `eax`.
fn call(exec: &mut Exec, f: u32, out: u32) -> u32 {
    let at = exec.alloc_code(&[0; 64]).unwrap();
    let mut c = vec![0x89, 0x25]; // mov [out + 20], esp
    c.extend((out + 20).to_le_bytes());
    c.push(0xb8); // mov eax, imm32
    c.extend(0xa5a5_a5a5u32.to_le_bytes());
    c.push(0xba); // mov edx, imm32
    c.extend(0x5a5a_5a5au32.to_le_bytes());
    c.push(0xb9); // mov ecx, imm32
    c.extend(0x3c3c_3c3cu32.to_le_bytes());
    c.push(0xe8); // call rel32
    let next = at + u32::try_from(c.len()).unwrap() + 4;
    c.extend(f.wrapping_sub(next).to_le_bytes());
    c.extend([0x89, 0x0d]); // mov [out], ecx
    c.extend(out.to_le_bytes());
    c.extend([0x89, 0x15]); // mov [out + 4], edx
    c.extend((out + 4).to_le_bytes());
    c.extend([0x89, 0x25]); // mov [out + 16], esp
    c.extend((out + 16).to_le_bytes());
    c.push(0xc3);
    exec.write_code(at, &c);
    match exec.call_cdecl(at, &[]) {
        Ok(eax) => eax,
        Err(e) => panic!("{f:#x}: {e} ({:?})", exec.stopped()),
    }
}

fn read(exec: &Exec, at: u32, n: usize) -> Vec<u8> {
    let mut b = vec![0; n];
    exec.mem().read_bytes(at, &mut b);
    b
}

#[test]
#[allow(clippy::many_single_char_names)] // the two runs and their results
fn m5_system_a_zero_stubs_run_from_the_table_as_the_original() {
    let Some(exe) = exe() else { return };
    let mut ported = ffx_host::boot_with(&exe, ffx_host::ports::ported()).unwrap();
    let mut original = ffx_host::boot_with(&exe, Table::new()).unwrap();
    let pout = ported.alloc_code(&[0; 32]).unwrap();
    let oout = original.alloc_code(&[0; 32]).unwrap();
    assert_eq!(pout, oout, "the two arenas allocate alike");
    for f in ZEROS {
        let e = ported
            .table()
            .get(f)
            .copied()
            .unwrap_or_else(|| panic!("{f:#x} not in the table"));
        assert_eq!((e.conv, e.words), (Conv::Cdecl, 0), "{}", e.name);
        let before = ported.stats.ports.get(&f).copied().unwrap_or(0);
        let a = call(&mut ported, f, pout);
        let b = call(&mut original, f, oout);
        assert_eq!(ported.stopped(), None);
        assert_eq!(original.stopped(), None);
        assert_eq!(
            ported.stats.ports.get(&f).copied().unwrap_or(0),
            before + 1,
            "{f:#x} ran in Rust"
        );
        assert_eq!((a, b), (0, 0), "{f:#x}: eax");
        let (x, y) = (read(&ported, pout, 32), read(&original, oout, 32));
        assert_eq!(x, y, "{f:#x}: ecx, edx, esp");
        let esp = |v: &[u8], at: usize| u32::from_le_bytes(v[at..at + 4].try_into().unwrap());
        assert_eq!(esp(&x, 16), esp(&x, 20), "{f:#x}: the caller's esp");
    }
    assert!(original.stats.ports.is_empty());
}
