//! The `host_only!` entry of M5 list `phyre-physics`
//! (`Phyre_PPhysics_PUtilityPhysics_slot2`, `0x00605720`, `xor eax, eax;
//! ret`; docs/porting.md, "Functions under five bytes") through ffx-host's
//! function table: the 32-bit game has no hook for it, so this is where
//! its table entry runs. It is called from Unicorn code the way
//! `Phyre_PhyreExit` calls it (`ecx` = the utility, `call`), once with the
//! ported table (the entry must run in Rust) and once with no table (the
//! original's bytes); `eax`, the caller's `ecx`/`edx` after the call and
//! its stack must agree. Needs `FFX_GAME_DIR`; skips without it.

use std::path::PathBuf;

use ffx_bridge::Mem as _;
use ffx_host::exec::Exec;
use ffx_host::table::{Conv, Table};

const SLOT2: u32 = 0x0060_5720;

fn exe() -> Option<PathBuf> {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return None;
    };
    Some(PathBuf::from(dir).join("FFX.exe"))
}

/// Calls the entry on `this` from code in the arena with `eax` and `edx`
/// preset; after the call `ecx`, `edx`, `esp` go to `out`, `out + 4`,
/// `out + 16` (`out + 20`: `esp` before). Returns `eax`.
fn call(exec: &mut Exec, this: u32, out: u32) -> u32 {
    let at = exec.alloc_code(&[0; 64]).unwrap();
    let mut c = vec![0x89, 0x25]; // mov [out + 20], esp
    c.extend((out + 20).to_le_bytes());
    c.push(0xb8); // mov eax, imm32
    c.extend(0xa5a5_a5a5u32.to_le_bytes());
    c.push(0xba); // mov edx, imm32
    c.extend(0x5a5a_5a5au32.to_le_bytes());
    c.push(0xb9); // mov ecx, this
    c.extend(this.to_le_bytes());
    c.push(0xe8); // call rel32
    let next = at + u32::try_from(c.len()).unwrap() + 4;
    c.extend(SLOT2.wrapping_sub(next).to_le_bytes());
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
        Err(e) => panic!("{SLOT2:#x}: {e} ({:?})", exec.stopped()),
    }
}

fn read(exec: &Exec, at: u32, n: usize) -> Vec<u8> {
    let mut b = vec![0; n];
    exec.mem().read_bytes(at, &mut b);
    b
}

#[test]
#[allow(clippy::many_single_char_names)] // the two runs and their results
fn m5p_physics_slot2_runs_from_the_table_as_the_original() {
    let Some(exe) = exe() else { return };
    let mut ported = ffx_host::boot_with(&exe, ffx_host::ports::ported()).unwrap();
    let mut original = ffx_host::boot_with(&exe, Table::new()).unwrap();
    let pout = ported.alloc_code(&[0; 32]).unwrap();
    let oout = original.alloc_code(&[0; 32]).unwrap();
    assert_eq!(pout, oout, "the two arenas allocate alike");
    let e = ported
        .table()
        .get(SLOT2)
        .copied()
        .unwrap_or_else(|| panic!("{SLOT2:#x} not in the table"));
    assert_eq!((e.conv, e.words), (Conv::Thiscall, 0), "{}", e.name);
    for this in [0, 0x00c2_a0f4, u32::MAX] {
        let before = ported.stats.ports.get(&SLOT2).copied().unwrap_or(0);
        let a = call(&mut ported, this, pout);
        let b = call(&mut original, this, oout);
        assert_eq!(ported.stopped(), None);
        assert_eq!(original.stopped(), None);
        assert_eq!(
            ported.stats.ports.get(&SLOT2).copied().unwrap_or(0),
            before + 1,
            "ran in Rust"
        );
        assert_eq!((a, b), (0, 0), "eax");
        let (x, y) = (read(&ported, pout, 32), read(&original, oout, 32));
        assert_eq!(x, y, "this {this:#x}: ecx, edx, esp");
        let esp = |v: &[u8], at: usize| u32::from_le_bytes(v[at..at + 4].try_into().unwrap());
        assert_eq!(esp(&x, 16), esp(&x, 20), "the caller's esp");
    }
    assert!(original.stats.ports.is_empty());
}
