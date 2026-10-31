//! The spell overlays' Rust ports in `ffx-host` (docs/host.md,
//! "Overlays"): the same DLL loaded twice through `FFX.exe`'s own
//! `LoadLibraryA`, once all in Unicorn and once with the ports bound by
//! code identity (`ffx-magic`), gives the same results, and the calls are
//! counted on the side that ran them. Needs `FFX_GAME_DIR`; skips without
//! it.

use std::path::PathBuf;

use ffx_bridge::Mem as _;
use ffx_host::exec::Exec;

/// `g_MagicImports`, the table `MagicFile_Start` hands `InitMagicPRX`.
const MAGIC_IMPORTS: u32 = 0x00c6_4ce8;

fn exe() -> Option<PathBuf> {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return None;
    };
    Some(PathBuf::from(dir).join("FFX.exe"))
}

/// The import `KERNEL32.dll!name`'s stub, as `FFX.exe`'s IAT holds it.
fn kernel32(exec: &Exec, name: &str) -> u32 {
    let slot = exec
        .image()
        .imports
        .iter()
        .find(|i| i.dll.eq_ignore_ascii_case("KERNEL32.dll") && i.name == name)
        .unwrap_or_else(|| panic!("{name} not imported"))
        .slot;
    exec.mem().read(slot)
}

fn put_str(exec: &mut Exec, s: &str) -> u32 {
    let at = exec.alloc(u32::try_from(s.len() + 1).unwrap());
    let mut bytes = s.as_bytes().to_vec();
    bytes.push(0);
    exec.mem().write_bytes(at, &bytes);
    at
}

/// `GetProcAddress(module, name)` through the game's import.
fn export(exec: &mut Exec, module: u32, name: &str) -> u32 {
    let gpa = kernel32(exec, "GetProcAddress");
    let at = put_str(exec, name);
    exec.call_stdcall(gpa, &[module, at]).unwrap()
}

/// Code in the arena that calls `f(arg)` (cdecl) the way the game does,
/// so the call enters the overlay from Unicorn.
fn call_from_code(exec: &mut Exec, f: u32, arg: u32) -> u32 {
    let at = exec.alloc_code(&[0; 32]).unwrap();
    let mut code = vec![0x68]; // push imm32
    code.extend_from_slice(&arg.to_le_bytes());
    code.push(0xb8); // mov eax, f
    code.extend_from_slice(&f.to_le_bytes());
    code.extend_from_slice(&[0xff, 0xd0, 0x83, 0xc4, 0x04, 0xc3]); // call eax; add esp, 4; ret
    exec.write_code(at, &code);
    exec.call_cdecl(at, &[]).unwrap()
}

/// What one load of the overlay gives: `InitMagicPRX`'s `eax` (from Rust
/// and from emulated code), `GetEffectOverlayTable(0)` relative to the
/// base and `GetEffectOverlayTable(1)`; then the base.
fn run(exec: &mut Exec, dll: &str) -> ([u32; 4], u32) {
    let lla = kernel32(exec, "LoadLibraryA");
    let path = put_str(exec, &format!("magicFiles\\FFX\\{dll}"));
    let base = exec.call_stdcall(lla, &[path]).unwrap();
    assert_ne!(base, 0, "LoadLibraryA({dll})");
    let init = export(exec, base, "InitMagicPRX");
    let table = export(exec, base, "GetEffectOverlayTable");
    assert!(init != 0 && table != 0, "{dll}'s exports");
    let a = exec.call_cdecl(init, &[MAGIC_IMPORTS]).unwrap();
    let b = call_from_code(exec, init, MAGIC_IMPORTS);
    let c = exec.call_cdecl(table, &[0]).unwrap().wrapping_sub(base);
    let d = call_from_code(exec, table, 1);
    ([a, b, c, d], base)
}

#[test]
fn an_overlay_runs_its_rust_ports_and_counts_both_sides() {
    let Some(exe) = exe() else { return };
    let mut exec = ffx_host::boot(&exe).unwrap();
    // magic_0604: fully ported (docs/formats/magic.md)
    ffx_host::library::select_rust(Some(&[]));
    let (emulated, base_u) = run(&mut exec, "magic_0604.dll");
    ffx_host::library::select_rust(None);
    let (ported, base_r) = run(&mut exec, "magic_0604.dll");
    assert_ne!(base_u, base_r, "each load gets a fresh range");
    assert_eq!(emulated, ported, "InitMagicPRX, GetEffectOverlayTable");
    assert_eq!(emulated[3], 0, "GetEffectOverlayTable(1)");
    // all Unicorn: DllMain, 2 x InitMagicPRX, 2 x GetEffectOverlayTable,
    // and the CRT's initialisers DllMain reaches through its tables
    let (rust_u, emu_u) = ffx_host::library::calls(base_u).unwrap();
    assert_eq!(rust_u, 0, "the all-Unicorn load");
    assert!(emu_u >= 5, "the all-Unicorn load: {emu_u} calls");
    for line in ffx_host::library::summary() {
        eprintln!("{line}");
    }
    // the ports: the same five calls in Rust, none left to Unicorn
    let (rust_r, emu_r) = ffx_host::library::calls(base_r).unwrap();
    assert_eq!((rust_r, emu_r), (5, 0), "the ported load");
    // every section but the code (the globals InitMagicPRX filled): the
    // same words, the relocated ones relative to each base. The ported
    // DllMain keeps none of the CRT's state (docs/formats/magic.md, "The
    // CRT"): the stack cookie keeps its default and is left out.
    let dir = PathBuf::from(std::env::var_os("FFX_GAME_DIR").unwrap());
    let file = std::fs::read(dir.join("magicFiles/FFX/magic_0604.dll")).unwrap();
    let pe = ffx_magic::pe::Pe::parse(&file).unwrap();
    let relocs: std::collections::HashSet<u32> = pe.relocs.iter().copied().collect();
    let rvas: Vec<u32> = pe
        .sections
        .iter()
        .filter(|s| !s.exec)
        .flat_map(|s| (0..s.size / 4).map(move |i| s.rva + 4 * i))
        .collect();
    let words = |base: u32| -> Vec<u32> {
        rvas.iter()
            .map(|&rva| {
                let w = exec.mem().read::<u32>(base + rva);
                if relocs.contains(&rva) {
                    w.wrapping_sub(base)
                } else {
                    w
                }
            })
            .collect()
    };
    // what the loader left there (relocated words relative to the base)
    let loaded: Vec<u32> = rvas
        .iter()
        .map(|&rva| {
            let s = pe
                .sections
                .iter()
                .find(|s| rva >= s.rva && rva < s.rva + s.size)
                .unwrap();
            let off = rva - s.rva;
            let w = if off + 4 <= s.raw_size {
                let at = (s.raw + off) as usize;
                u32::from_le_bytes(file[at..at + 4].try_into().unwrap())
            } else {
                0
            };
            if relocs.contains(&rva) {
                w.wrapping_sub(pe.preferred_base)
            } else {
                w
            }
        })
        .collect();
    let (u, r) = (words(base_u), words(base_r));
    // the CRT's words: the original's DllMain wrote them (the cookie, the
    // attach count, the onexit table), the port left them as loaded
    let crt: Vec<usize> = (0..u.len())
        .filter(|&i| u[i] != r[i] && r[i] == loaded[i])
        .collect();
    for &i in &crt {
        eprintln!("CRT word +{:#x}: {:#x} unicorn", rvas[i], u[i]);
    }
    assert!(crt.len() <= 8, "{} words left to the CRT", crt.len());
    let changed: Vec<usize> = (0..u.len())
        .filter(|&i| u[i] != r[i] && r[i] != loaded[i])
        .collect();
    for &i in &changed {
        eprintln!(
            "data at +{:#x}: {:#x} unicorn, {:#x} rust",
            rvas[i], u[i], r[i]
        );
    }
    assert!(changed.is_empty(), "the DLL's data");
    // InitMagicPRX's 33 stores reached both
    let written = (0..u.len())
        .filter(|&i| r[i] != loaded[i] && u[i] == r[i])
        .count();
    assert!(written >= 33, "{written} words written alike");
}
