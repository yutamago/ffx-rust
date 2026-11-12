//! `ffx-host` on the player's `FFX.exe`, on Linux (docs/host.md, "The
//! Linux check"). Needs `FFX_GAME_DIR`; skips without it.

use std::path::PathBuf;
use std::time::Duration;

use ffx_host::exec::{Exec, Stop};
use ffx_host::table::Table;

fn exe() -> Option<PathBuf> {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return None;
    };
    Some(PathBuf::from(dir).join("FFX.exe"))
}

fn syms() -> ffx_host::report::Symbols {
    ffx_host::report::Symbols::read(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../symbols/ffx.sym"),
    )
}

/// A fresh, empty documents folder for one test.
fn documents(name: &str) -> PathBuf {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("host-docs-{name}"));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
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

/// Code in the arena that calls `MsCheckRange(x, lo, hi)` the way the
/// game does (cdecl), returning its `eax`.
fn caller(exec: &mut Exec, x: u32, lo: u32, hi: u32) -> u32 {
    let at = exec.alloc_code(&[0; 32]).unwrap();
    let mut code = Vec::new();
    for v in [hi, lo, x] {
        code.push(0x68); // push imm32
        code.extend_from_slice(&v.to_le_bytes());
    }
    let target = 0x0079_a0c0u32;
    code.push(0xe8); // call rel32
    let next = at + u32::try_from(code.len()).unwrap() + 4;
    code.extend_from_slice(&target.wrapping_sub(next).to_le_bytes());
    code.extend_from_slice(&[0x83, 0xc4, 0x0c, 0xc3]); // add esp, 12; ret
    exec.write_code(at, &code);
    exec.call_cdecl(at, &[]).unwrap()
}

#[test]
fn unicorn_code_enters_a_port_and_matches_the_original() {
    let Some(exe) = exe() else { return };
    let mut ported = ffx_host::boot(&exe).unwrap();
    let mut original = ffx_host::boot_with(&exe, Table::new()).unwrap();
    for (x, lo, hi) in [(9, 1, 5), (0, 1, 5), (3, 1, 5), (u32::MAX, 0, 7), (8, 8, 8)] {
        let a = caller(&mut ported, x, lo, hi);
        let b = caller(&mut original, x, lo, hi);
        assert_eq!(a, b, "MsCheckRange({x}, {lo}, {hi})");
    }
    assert_eq!(ported.stats.ports.get(&0x0079_a0c0), Some(&5));
    assert!(original.stats.ports.is_empty());
}

/// Two game threads, each started through `FFX.exe`'s own `CreateThread`
/// import, add to one counter `N` times under one critical section with
/// a plain (non-atomic) `inc`, and the main thread waits for both handles.
/// Each thread is its own Unicorn instance on its own host thread, so the
/// count is only right if `EnterCriticalSection` excludes for real.
#[test]
fn game_threads_run_in_parallel_under_the_games_own_critical_section() {
    const N: u32 = 3000;
    let Some(exe) = exe() else { return };
    let mut exec = ffx_host::boot(&exe).unwrap();
    exec.set_budget(Duration::from_secs(60));
    let k = "KERNEL32.dll";
    let (create, wait) = (
        slot(&exec, k, "CreateThread"),
        slot(&exec, k, "WaitForSingleObject"),
    );
    let (init, enter, leave) = (
        slot(&exec, k, "InitializeCriticalSection"),
        slot(&exec, k, "EnterCriticalSection"),
        slot(&exec, k, "LeaveCriticalSection"),
    );
    let call = |c: &mut Vec<u8>, at: u32| {
        c.extend_from_slice(&[0xff, 0x15]); // call [at]
        c.extend_from_slice(&at.to_le_bytes());
    };
    // the counter at +0, the CRITICAL_SECTION at +8
    let data = exec.alloc_code(&[0; 0x40]).unwrap();
    // routine(param): N times { Enter(cs); inc [counter]; Leave(cs) }
    let mut r = vec![0x53, 0x56, 0x8b, 0x74, 0x24, 0x0c]; // push ebx; push esi; mov esi, [esp+12]
    r.push(0xbb); // mov ebx, N
    r.extend_from_slice(&N.to_le_bytes());
    let top = r.len();
    r.extend_from_slice(&[0x8d, 0x46, 0x08, 0x50]); // lea eax, [esi+8]; push eax
    call(&mut r, enter);
    r.extend_from_slice(&[0xff, 0x06]); // inc dword [esi]
    r.extend_from_slice(&[0x8d, 0x46, 0x08, 0x50]);
    call(&mut r, leave);
    r.push(0x4b); // dec ebx
    let back = u8::try_from(r.len() + 2 - top).unwrap().wrapping_neg();
    r.extend_from_slice(&[0x75, back]); // jnz top
    r.extend_from_slice(&[0x33, 0xc0, 0x5e, 0x5b, 0xc2, 0x04, 0x00]); // xor eax, eax; pop esi; pop ebx; ret 4
    let routine = exec.alloc_code(&r).unwrap();
    // main: Init(cs); two CreateThread; WaitForSingleObject on both
    let mut m = vec![0x56, 0x57]; // push esi; push edi
    m.push(0x68);
    m.extend_from_slice(&(data + 8).to_le_bytes());
    call(&mut m, init);
    for reg in [0xf8u8, 0xf0] {
        // push 0 (&id); push 0 (flags); push data; push routine; push 0; push 0
        m.extend_from_slice(&[0x6a, 0, 0x6a, 0, 0x68]);
        m.extend_from_slice(&data.to_le_bytes());
        m.push(0x68);
        m.extend_from_slice(&routine.to_le_bytes());
        m.extend_from_slice(&[0x6a, 0, 0x6a, 0]);
        call(&mut m, create);
        m.extend_from_slice(&[0x8b, reg]); // mov edi/esi, eax
    }
    for reg in [0x57u8, 0x56] {
        m.extend_from_slice(&[0x6a, 0xff, reg]); // push INFINITE; push edi/esi
        call(&mut m, wait);
    }
    m.extend_from_slice(&[0xa1]); // mov eax, [counter]
    m.extend_from_slice(&data.to_le_bytes());
    m.extend_from_slice(&[0x5f, 0x5e, 0xc3]); // pop edi; pop esi; ret
    let main = exec.alloc_code(&m).unwrap();
    let count = exec.call_cdecl(main, &[]).unwrap();
    let threads = exec.shared().threads.lock().unwrap().clone();
    assert_eq!(count, 2 * N, "{threads:#?}");
    assert_eq!(threads.len(), 2);
    for t in &threads {
        assert!(t.started);
        assert_eq!(t.outcome.as_deref(), Some("returned 0x0"), "{t:?}");
        let s = t.stats.as_ref().unwrap();
        // N, and the DLLs' own sections in DllMain(DLL_THREAD_ATTACH)
        assert!(s.imports["KERNEL32.dll!EnterCriticalSection"] >= u64::from(N));
    }
}

#[test]
fn boots_past_the_d3d11_device_to_the_first_frames() {
    let Some(exe) = exe() else { return };
    let opts = ffx_host::Options {
        documents: Some(documents("boot")),
        gfx: ffx_host::gfx::Config {
            backend: ffx_host::gfx::Choice::Null,
            stop_after_presents: Some(3),
            ..ffx_host::gfx::Config::default()
        },
        ..ffx_host::Options::default()
    };
    let mut exec = ffx_host::boot_opts(&exe, ffx_host::ports::table(), &opts).unwrap();
    exec.cover().unwrap();
    exec.set_budget(Duration::from_secs(120));
    let stop = exec.run_entry().unwrap();
    exec.join_threads(Duration::from_secs(5));
    let syms = syms();
    let report = ffx_host::report::text(&exec, &syms, &format!("{stop}"));
    eprintln!("{report}");
    // the player's C runtime DLLs ran their DllMain and serve the STL
    assert_eq!(exec.shared().dlls.len(), 2, "{report}");
    // the static initialisers ran (each registers its destructor, through
    // `atexit`'s host port: `ffx_host::crt`)
    assert!(
        exec.shared().at_exit.lock().unwrap().len() > 400,
        "{report}"
    );
    assert!(exec.stats.ports[&ffx_host::crt::ATEXIT] > 400, "{report}");
    // GameSetting.ini: none at the start, so the game wrote its default
    let ini = opts
        .documents
        .as_ref()
        .unwrap()
        .join("Square Enix/FINAL FANTASY X&X-2 HD Remaster/GameSetting.ini");
    assert!(ini.is_file(), "{report}");
    // the window, the feature level probe, the device and swap chain on
    // the recording null device, then three frames presented
    assert_eq!(
        exec.stats.milestones,
        [
            "first window (CreateWindowExA)",
            "D3D11 feature level probe (11_0)",
            "D3D11 device and swap chain (null (recording), 1920 x 1080)"
        ],
        "{report}"
    );
    assert_eq!(stop, Stop::Milestone("Present 3".into()), "{report}");
    {
        let slot = exec.shared().gfx.lock();
        let g = slot.as_ref().unwrap();
        assert_eq!(g.presents, 3, "{report}");
        // the renderer's start-up made its shaders, states and targets
        // through the device, with nothing the layer doesn't answer
        assert!(g.counts["ID3D11Device::CreateVertexShader"] > 0, "{report}");
        assert!(g.counts["ID3D11Device::CreateBlendState"] > 0, "{report}");
        assert_eq!(g.counts["IDXGISwapChain::GetBuffer"], 1, "{report}");
        assert!(g.unhandled.is_empty(), "{:?}", g.unhandled);
        // the stream reads back as the captures do: objects, then frames
        let msgs = ffx_gfx::read_all(&g.stream()).unwrap();
        let frames = msgs
            .iter()
            .filter(|m| matches!(m, ffx_gfx::Msg::Frame(_)))
            .count();
        assert_eq!(frames, 3);
        assert!(
            msgs.iter()
                .any(|m| matches!(m, ffx_gfx::Msg::Shader { kind: b'v', .. }))
        );
    }
    let s = ffx_host::report::summary(&exec, &syms);
    // ported functions run in Rust now, the rest in Unicorn
    assert!(
        s.unicorn_functions.len() + s.rust_functions > 1500,
        "{report}"
    );
    assert!(!s.dll_functions.is_empty(), "{report}");
    // the profile (`--profile`) leaves the ports' int3 blocks out: a
    // table entry's row holds only the original's body (a wrapper's
    // trampoline jumping back in), never an entry of its own
    for f in s.functions.iter().filter(|f| f.module == "FFX.exe") {
        assert!(
            f.calls == 0 || exec.table().get(f.start).is_none(),
            "{} {:#x}: {} calls counted at a port's int3",
            f.name,
            f.start,
            f.calls
        );
    }
    // the TSV tools/unicorn-report.py reads
    let tsv = ffx_host::report::profile_tsv(&s);
    assert!(tsv.starts_with("module\tstart\tname\tcalls\tblocks\tsamples_ms\n"));
    assert_eq!(tsv.lines().count(), s.functions.len() + 1);
    // the profile is every thread's, also read from another thread while
    // the game's threads may still run (the harness's `quit` writes it)
    let shared = std::sync::Arc::clone(exec.shared());
    let blocks = std::thread::spawn(move || shared.profile().blocks.len())
        .join()
        .unwrap();
    assert!(blocks >= s.unicorn_blocks, "{report}");
    // the heap's decommit thread started and ended with the run
    let threads = exec.shared().threads.lock().unwrap().clone();
    assert!(
        threads
            .iter()
            .any(|t| t.entry == 0x0094_8c30 && t.started && t.outcome.is_some()),
        "{report}"
    );
}

/// Without the player's C runtime DLLs the run stops at the first C
/// runtime function with no Rust side yet: since the STL's users were
/// ported (WP6, 2026-11-12) an `MSVCR110` import, no longer an `MSVCP110`
/// one; `fwrite` in `Settings_Save` until the no-DLL stdio
/// (`crate::crt_stdio`, wp5b), `qsort` after it. The wall moves as the
/// C runtime's Rust sides grow (WP5).
#[test]
fn without_the_dlls_the_c_runtime_is_the_wall() {
    let Some(exe) = exe() else { return };
    let opts = ffx_host::Options {
        no_dlls: true,
        documents: Some(documents("no-dlls")),
        ..ffx_host::Options::default()
    };
    let mut exec = ffx_host::boot_opts(&exe, ffx_host::ports::table(), &opts).unwrap();
    exec.set_budget(Duration::from_secs(60));
    let stop = exec.run_entry().unwrap();
    match &stop {
        Stop::Failed(why, _) => assert!(
            why.starts_with("unstubbed import MSVCR110.dll!")
                && !why.starts_with("unstubbed import MSVCR110.dll!fwrite "),
            "{why}"
        ),
        other => panic!("unexpected stop {other:?}"),
    }
}

/// A port calling `wsprintfW` through its IAT slot (Rust → import, as
/// `SaveDataMachine_WriteFile`'s port names the save file): the varargs
/// are read through `slots`, so the words must be on the game stack. They
/// were not, and the save's path came out "000".
#[test]
fn a_varargs_import_called_from_rust_reads_its_words() {
    use ffx_bridge::Mem as _;
    let Some(exe) = exe() else { return };
    let mut exec = ffx_host::boot(&exe).unwrap();
    let stub: u32 = exec.mem().read(slot(&exec, "USER32.dll", "wsprintfW"));
    let wide = |s: &str| -> Vec<u8> {
        s.encode_utf16()
            .chain([0])
            .flat_map(u16::to_le_bytes)
            .collect()
    };
    let put = |exec: &mut Exec, bytes: &[u8]| -> u32 {
        let at = exec.alloc(u32::try_from(bytes.len()).unwrap());
        exec.mem().write_bytes(at, bytes);
        at
    };
    let fmt = put(&mut exec, &wide("%s%s%03d"));
    let dir = put(&mut exec, &wide("C:\\saves\\"));
    let prefix = put(&mut exec, &wide("ffx_"));
    let out = put(&mut exec, &[0; 256]);
    let n = exec.call_cdecl(stub, &[out, fmt, dir, prefix, 7]).unwrap();
    let mut got = vec![0u8; 2 * n as usize];
    exec.mem().read_bytes(out, &mut got);
    let text: Vec<u16> = got
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&c| u16::from_le_bytes(c))
        .collect();
    assert_eq!(String::from_utf16_lossy(&text), "C:\\saves\\ffx_007");
}

/// `FFX.exe`'s import thunks (`jmp [IAT slot]`): a port calling
/// `crt_memcpy_thunk` (95 M calls in gate run 1) or `crt_memset_thunk`
/// runs the import's Rust side with no emulator start, and Unicorn code
/// calling one (patched to `int3`) gets the same result. Thunks the scan
/// leaves alone (`0x009498c2`, after `alloca_probe_8`'s `jmp`) still
/// resolve for a call from Rust.
#[test]
fn import_thunks_run_their_import_without_unicorn() {
    use ffx_bridge::Mem as _;
    const MEMCPY_THUNK: u32 = 0x0094_9256;
    const MEMSET_THUNK: u32 = 0x0094_964c;
    let Some(exe) = exe() else { return };
    let mut exec = ffx_host::boot(&exe).unwrap();
    let memcpy_stub: u32 = exec.mem().read(slot(&exec, "MSVCR110.dll", "memcpy"));
    assert_eq!(exec.shared().thunk(MEMCPY_THUNK), Some(memcpy_stub));
    assert!(exec.shared().is_patched_thunk(MEMCPY_THUNK));
    let memset_stub: u32 = exec.mem().read(slot(&exec, "MSVCR110.dll", "memset"));
    assert_eq!(exec.shared().thunk(MEMSET_THUNK), Some(memset_stub));
    // 0x009498c2: jmp [0x00b0c3b4], right after alloca_probe_8's jmp
    let late: u32 = exec.mem().read(0x00b0_c3b4);
    assert!(!exec.shared().is_patched_thunk(0x0094_98c2));
    assert_eq!(exec.shared().thunk(0x0094_98c2), Some(late));
    // a thunk's neighbour, an ordinary function, is none
    assert_eq!(exec.shared().thunk(0x0094_9620), None);
    let src = exec.alloc(64);
    let dst = exec.alloc(64);
    let bytes: Vec<u8> = (0u8..64).collect();
    exec.mem().write_bytes(src, &bytes);
    // from Rust: the Rust side, no Unicorn
    let before = (exec.stats.unicorn_calls, exec.stats.starts);
    let r = exec.call_cdecl(MEMCPY_THUNK, &[dst, src, 40]).unwrap();
    assert_eq!(r, dst);
    assert_eq!((exec.stats.unicorn_calls, exec.stats.starts), before);
    let mut got = vec![0u8; 64];
    exec.mem().read_bytes(dst, &mut got);
    assert_eq!(&got[..40], &bytes[..40]);
    assert_eq!(&got[40..], &[0u8; 24]);
    exec.call_cdecl(MEMSET_THUNK, &[dst, 0xab, 8]).unwrap();
    exec.mem().read_bytes(dst, &mut got);
    assert_eq!(&got[..8], &[0xab; 8]);
    assert_eq!(&got[8..40], &bytes[8..40]);
    // from Unicorn: push 16; push src; push dst; call thunk; add esp, 12; ret
    let at = exec.alloc_code(&[0; 32]).unwrap();
    let mut code = vec![0x6a, 16, 0x68];
    code.extend_from_slice(&src.to_le_bytes());
    code.push(0x68);
    code.extend_from_slice(&(dst + 48).to_le_bytes());
    code.push(0xe8);
    let next = at + u32::try_from(code.len()).unwrap() + 4;
    code.extend_from_slice(&MEMCPY_THUNK.wrapping_sub(next).to_le_bytes());
    code.extend_from_slice(&[0x83, 0xc4, 0x0c, 0xc3]);
    exec.write_code(at, &code);
    let r = exec.call_cdecl(at, &[]).unwrap();
    assert_eq!(r, dst + 48);
    exec.mem().read_bytes(dst, &mut got);
    assert_eq!(&got[48..], &bytes[..16]);
    assert_eq!(exec.stats.imports["MSVCR110.dll!memcpy"], 2);
}
