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

/// Without the player's C runtime DLLs the run (here with Unicorn for
/// the code without a port) no longer stops at the C runtime: every
/// `MSVCR110`/`MSVCP110` import the boot calls has a Rust side (m5-wp5c;
/// the wall was `fwrite` in `Settings_Save` before wp5b's stdio, `qsort`
/// after it, then the maths). The run uses its whole budget, and no C
/// runtime import without a Rust side was called.
#[test]
fn without_the_dlls_the_c_runtime_is_no_wall() {
    let Some(exe) = exe() else { return };
    let opts = ffx_host::Options {
        no_dlls: true,
        documents: Some(documents("no-dlls")),
        ..ffx_host::Options::default()
    };
    let mut exec = ffx_host::boot_opts(&exe, ffx_host::ports::table(), &opts).unwrap();
    exec.set_budget(Duration::from_secs(60));
    let stop = exec.run_entry().unwrap();
    if let Stop::Failed(why, _) = &stop {
        assert!(
            !why.contains("MSVCR110.dll!") && !why.contains("MSVCP110.dll!"),
            "{why}"
        );
    }
    let crt_missing: Vec<&String> = exec
        .stats
        .missing_imports
        .iter()
        .filter(|n| n.starts_with("MSVCR110") || n.starts_with("MSVCP110"))
        .collect();
    assert!(crt_missing.is_empty(), "{crt_missing:?}");
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

/// On the real clock the start-up logos (`OpeningScreen.swf`) end with the
/// movie's `Finish` callback, which a build without Ruffle (no `swf`
/// feature) never sends: the start-up loop then never ends (0 updates,
/// flips at 30 a second). `Options::skip_opening` ends them on their first
/// frame, as the virtual clock does, and the main loop's updates begin.
#[test]
#[cfg(not(feature = "swf"))]
fn the_real_clock_skips_the_opening_logos_when_asked() {
    let Some(exe) = exe() else { return };
    let opts = ffx_host::Options {
        documents: Some(documents("skip-opening")),
        gfx: ffx_host::gfx::Config {
            backend: ffx_host::gfx::Choice::Null,
            no_record: true,
            ..ffx_host::gfx::Config::default()
        },
        real_clock: true,
        skip_opening: true,
        ..ffx_host::Options::default()
    };
    let mut exec = ffx_host::boot_opts(&exe, ffx_host::ports::table(), &opts).unwrap();
    exec.set_budget(Duration::from_secs(120));
    let shared = std::sync::Arc::clone(exec.shared());
    let watch = std::thread::spawn(move || {
        while shared.clock.frame() < 30 && shared.stopped().is_none() {
            std::thread::sleep(Duration::from_millis(5));
        }
        shared.stop_all(Stop::Milestone("30 updates".into()));
    });
    let stop = exec.run_entry().unwrap();
    exec.join_threads(Duration::from_secs(5));
    watch.join().unwrap();
    assert_eq!(stop, Stop::Milestone("30 updates".into()));
    assert!(exec.shared().clock.frame() >= 30);
}

/// The game-state API (`ffx_host::observe`, `ffx_state`): an observer gets a
/// snapshot at every update boundary. In the opening (map 0x15d: the party
/// at the campfire) it sees the camera and the party's characters, which
/// move and animate from update to update.
#[test]
fn the_state_api_sees_the_opening() {
    use std::sync::{Arc, Mutex};
    let Some(exe) = exe() else { return };
    let opts = ffx_host::Options {
        documents: Some(documents("state-api")),
        gfx: ffx_host::gfx::Config {
            backend: ffx_host::gfx::Choice::Null,
            no_record: true,
            ..ffx_host::gfx::Config::default()
        },
        ..ffx_host::Options::default()
    };
    let mut exec = ffx_host::boot_opts(&exe, ffx_host::ports::table(), &opts).unwrap();
    exec.set_budget(Duration::from_secs(300));
    // the snapshots from 120 updates into the opening on
    let seen: Arc<Mutex<Vec<Arc<ffx_state::Snapshot>>>> = Arc::default();
    let opening_at: Arc<Mutex<Option<u32>>> = Arc::default();
    let id = {
        let (seen, opening_at) = (Arc::clone(&seen), Arc::clone(&opening_at));
        let shared = Arc::downgrade(exec.shared());
        exec.shared().observers.add(Arc::new(move |s| {
            if s.world.map != 0x15d {
                return;
            }
            let Ok(mut at) = opening_at.lock() else {
                return;
            };
            let start = *at.get_or_insert(s.update);
            if s.world.event_frame >= 900
                && s.update >= start + 120
                && let Ok(mut v) = seen.lock()
            {
                v.push(Arc::clone(s));
                if v.len() >= 60
                    && let Some(shared) = shared.upgrade()
                {
                    shared.stop_all(Stop::Milestone("60 snapshots".into()));
                }
            }
        }))
    };
    let stop = exec.run_entry().unwrap();
    exec.join_threads(Duration::from_secs(5));
    exec.shared().observers.remove(id);
    assert!(!exec.shared().observers.any());
    assert_eq!(stop, Stop::Milestone("60 snapshots".into()));
    let seen = seen.lock().unwrap();
    let (first, last) = (&seen[0], &seen[seen.len() - 1]);
    for a in &last.actors {
        eprintln!(
            "slot {} id {:#06x} {} hidden {:#x} motion {:#x} frame {:.1} bones {}",
            a.slot,
            a.id,
            a.model,
            a.hidden,
            a.motion,
            a.motion_frame,
            a.bones.len()
        );
    }
    eprintln!("camera {:?}", last.camera);
    eprintln!("world {:?}", last.world);
    // one snapshot an update, in order
    assert!(seen.windows(2).all(|w| w[1].update == w[0].update + 1));
    assert!(last.camera.valid);
    assert!(
        last.camera.fov_y > 0.1 && last.camera.fov_y < 2.0,
        "{:?}",
        last.camera
    );
    assert_eq!(last.world.event, "lo/loopdemo", "{:?}", last.world);
    // the renderer's level: the opening's map model
    assert_eq!(last.world.level, "/map/znkd/znkd06/", "{:?}", last.world);
    // the party at the fire: party characters (kind 0) shown, posed by
    // their skeletons (the event's motions place them: their records stay
    // at the origin), animating
    let party: Vec<_> = last
        .actors
        .iter()
        .filter(|a| a.kind == 0 && a.hidden == 0)
        .collect();
    assert!(party.len() >= 3, "{:?}", last.actors);
    assert!(party.iter().all(|a| a.bones.len() > 50));
    let animating = party
        .iter()
        .filter(|a| {
            first
                .actors
                .iter()
                .any(|b| b.slot == a.slot && (b.motion_frame - a.motion_frame).abs() > 0.01)
        })
        .count();
    assert!(animating >= 1, "no party member's motion advanced");
    // the renderer's camera: a world matrix with the eye in it
    assert!(last.camera.world[8..11].iter().any(|v| v.abs() > 1.0));
    assert_eq!(&last.encode()[0..4], b"FFXS");
}

/// A frontend's request (`ffx_host::requests`): in the opening, developer
/// save 7 (Besaid, scenario 118: the beach at the save sphere) loads from
/// the player's archive without the harness, and the beach comes up with
/// the control: the level `/map/bsil/bsil00/`, the player character shown.
#[test]
fn a_developer_save_loads_on_request() {
    use std::sync::{Arc, Mutex};
    let Some(exe) = exe() else { return };
    let opts = ffx_host::Options {
        documents: Some(documents("devsave-request")),
        gfx: ffx_host::gfx::Config {
            backend: ffx_host::gfx::Choice::Null,
            no_record: true,
            ..ffx_host::gfx::Config::default()
        },
        ..ffx_host::Options::default()
    };
    let mut exec = ffx_host::boot_opts(&exe, ffx_host::ports::table(), &opts).unwrap();
    exec.set_budget(Duration::from_secs(300));
    let reply: Arc<Mutex<Option<Result<String, String>>>> = Arc::default();
    let besaid: Arc<Mutex<Option<Arc<ffx_state::Snapshot>>>> = Arc::default();
    let id = {
        let (reply, besaid) = (Arc::clone(&reply), Arc::clone(&besaid));
        let shared = Arc::downgrade(exec.shared());
        let asked = std::sync::atomic::AtomicBool::new(false);
        exec.shared().observers.add(Arc::new(move |s| {
            let Some(shared) = shared.upgrade() else {
                return;
            };
            if s.world.map == 0x15d
                && s.world.event_frame > 30
                && !asked.swap(true, std::sync::atomic::Ordering::Relaxed)
            {
                let reply = Arc::clone(&reply);
                shared.requests.push(
                    ffx_host::requests::Request::DevSave(7),
                    Box::new(move |r| {
                        if let Ok(mut x) = reply.lock() {
                            *x = Some(r);
                        }
                    }),
                );
            }
            let player = s.actors.iter().any(|a| a.player && a.hidden == 0);
            if s.world.map == 0x46 && s.world.player_control && player {
                if let Ok(mut b) = besaid.lock() {
                    *b = Some(Arc::clone(s));
                }
                shared.stop_all(Stop::Milestone("Besaid with the control".into()));
            }
        }))
    };
    let stop = exec.run_entry().unwrap();
    exec.join_threads(Duration::from_secs(5));
    exec.shared().observers.remove(id);
    let reply = reply.lock().unwrap().clone();
    eprintln!("reply: {reply:?}");
    let r = reply
        .expect("the request was served")
        .expect("the save loaded");
    assert!(r.starts_with("developer save 7 map 70 "), "{r}");
    assert_eq!(stop, Stop::Milestone("Besaid with the control".into()));
    let s = besaid.lock().unwrap().clone().unwrap();
    eprintln!("world {:?}", s.world);
    assert_eq!(s.world.level, "/map/bsil/bsil00/", "{:?}", s.world);
    assert_eq!(s.world.story, 118);
    let p = s.actors.iter().find(|a| a.player).unwrap();
    assert!(p.bones.len() > 50, "{p:?}");
    eprintln!("player {} at {:?}", p.model, p.position);
}

/// The effect draws' capture (`ffx_host::vfx`, docs/state-api.md "Effect
/// draws"): on Besaid at the save sphere (developer save 7) the frame of
/// update `n`, read as `PhyreScene_DrawMostVFX` draws, comes before the
/// snapshot of update `n`; it draws the beach's field effects with the
/// map's own textures, and the sphere's orb at the save sphere `f001`'s
/// render point (its field position with x and y negated). The blob
/// decodes to the same frame.
#[test]
fn the_effect_draws_are_captured_at_the_save_sphere() {
    use std::sync::{Arc, Mutex};
    type Got = Option<(Arc<ffx_state::Snapshot>, Arc<ffx_host::vfx::Frame>)>;
    let Some(exe) = exe() else { return };
    let opts = ffx_host::Options {
        documents: Some(documents("vfx-capture")),
        gfx: ffx_host::gfx::Config {
            backend: ffx_host::gfx::Choice::Null,
            no_record: true,
            ..ffx_host::gfx::Config::default()
        },
        ..ffx_host::Options::default()
    };
    let mut exec = ffx_host::boot_opts(&exe, ffx_host::ports::table(), &opts).unwrap();
    exec.set_budget(Duration::from_secs(300));
    assert!(!exec.shared().vfx.enabled(), "off unless switched on");
    let got: Arc<Mutex<Got>> = Arc::default();
    let id = {
        let got = Arc::clone(&got);
        let shared = Arc::downgrade(exec.shared());
        let asked = std::sync::atomic::AtomicBool::new(false);
        let on_at = std::sync::atomic::AtomicU32::new(u32::MAX);
        exec.shared().observers.add(Arc::new(move |s| {
            use std::sync::atomic::Ordering::Relaxed;
            let Some(shared) = shared.upgrade() else {
                return;
            };
            if s.world.map == 0x15d && s.world.event_frame > 30 && !asked.swap(true, Relaxed) {
                shared
                    .requests
                    .push(ffx_host::requests::Request::DevSave(7), Box::new(|_| {}));
            }
            let player = s.actors.iter().any(|a| a.player && a.hidden == 0);
            if s.world.map == 0x46 && s.world.player_control && player {
                if on_at.load(Relaxed) == u32::MAX {
                    on_at.store(s.update, Relaxed);
                    shared.vfx.set_enabled(true);
                } else if s.update >= on_at.load(Relaxed) + 10 {
                    if let (Some(f), Ok(mut g)) = (shared.vfx.frame(s.update), got.lock()) {
                        *g = Some((Arc::clone(s), f));
                    }
                    shared.stop_all(Stop::Milestone("effect draws captured".into()));
                }
            }
        }))
    };
    let stop = exec.run_entry().unwrap();
    exec.join_threads(Duration::from_secs(5));
    exec.shared().observers.remove(id);
    assert_eq!(stop, Stop::Milestone("effect draws captured".into()));
    let (s, f) = got
        .lock()
        .unwrap()
        .clone()
        .expect("a frame of the snapshot's update");
    eprintln!(
        "update {}: {} draws, {} events, {} unresolved",
        f.update,
        f.draws.len(),
        f.events.len(),
        f.unresolved
    );
    assert_eq!(f.update, s.update);
    assert!(f.draws.len() > 10, "{}", f.draws.len());
    assert_eq!(f.unresolved, 0);
    for d in &f.draws {
        assert!(!d.texture.is_empty(), "{d:?}");
        assert_eq!(d.colours.len(), d.positions.len());
        assert_eq!(d.uvs.len(), d.positions.len());
        assert!(
            d.indices
                .iter()
                .all(|&i| usize::from(i) < d.positions.len())
        );
    }
    let map_tex = "/PS3Data/map/bsil/bsil00/fp/tex/D3D11/";
    assert!(f.draws.iter().any(|d| d.texture.contains(map_tex)));
    let sphere = s
        .actors
        .iter()
        .find(|a| a.model == "f001")
        .expect("the save sphere");
    let at = [-sphere.position[0], -sphere.position[1], sphere.position[2]];
    // the orb: textured draws whose vertices lie within a few units of the
    // sphere, above it
    let orb: Vec<_> = f
        .draws
        .iter()
        .filter(|d| {
            d.texture.contains(map_tex)
                && d.positions.iter().all(|p| {
                    (p[0] - at[0]).abs() < 6.0
                        && (p[2] - at[2]).abs() < 6.0
                        && (0.0..12.0).contains(&(p[1] - at[1]))
                })
        })
        .collect();
    eprintln!("save sphere at {at:?}: {} orb draws", orb.len());
    assert!(orb.len() >= 2, "{at:?}");
    check_save_sphere_blends(&f.draws, &orb);
    let decoded = ffx_host::vfx::Frame::decode(&f.encode()).unwrap();
    assert_eq!(&decoded, &*f);
}

/// The save sphere's blends (docs/formats/effects-draw.md, "Blend"): every
/// draw's is read, one shader pass each; the orb's modulated draws use the
/// Transparent technique (one, inverse source alpha), the sparkle quads
/// (flags 0xa, a `bat_eff` texture) one of each technique.
fn check_save_sphere_blends(draws: &[ffx_host::vfx::Draw], orb: &[&ffx_host::vfx::Draw]) {
    use ffx_host::vfx::{BlendFactor as F, BlendOp, Technique};
    assert!(
        draws
            .iter()
            .all(|d| d.technique.is_some() && d.blends.len() == 1),
        "{draws:?}"
    );
    for d in orb.iter().filter(|d| d.flags == 8) {
        let b = d.blends[0];
        assert_eq!(
            (d.technique, b.src, b.dest, b.op),
            (
                Some(Technique::Transparent),
                F::One,
                F::InvSrcAlpha,
                BlendOp::Add
            ),
            "{d:?}"
        );
    }
    let mut sparkles: Vec<_> = draws
        .iter()
        .filter(|d| d.texture.contains("/bat_eff/") && d.flags == 0xa)
        .map(|d| {
            (
                d.technique.map(|t| t as u8),
                d.blends[0].src,
                d.blends[0].dest,
            )
        })
        .collect();
    sparkles.sort_by_key(|b| b.0);
    assert_eq!(
        sparkles,
        [
            (Some(Technique::Transparent as u8), F::One, F::One),
            (Some(Technique::OptimizedTrans as u8), F::SrcAlpha, F::One),
        ]
    );
}

/// The game-state API's message windows (API 3): developer save 6 (the
/// arrival off Besaid, scenario 111) plays its cut scene, whose first
/// subtitle opens a message window with its text.
#[test]
fn the_state_api_sees_a_subtitle() {
    use std::sync::{Arc, Mutex};
    let Some(exe) = exe() else { return };
    let opts = ffx_host::Options {
        documents: Some(documents("subtitle")),
        gfx: ffx_host::gfx::Config {
            backend: ffx_host::gfx::Choice::Null,
            no_record: true,
            ..ffx_host::gfx::Config::default()
        },
        ..ffx_host::Options::default()
    };
    let mut exec = ffx_host::boot_opts(&exe, ffx_host::ports::table(), &opts).unwrap();
    exec.set_budget(Duration::from_secs(300));
    let seen: Arc<Mutex<Option<ffx_state::Message>>> = Arc::default();
    let language: Arc<Mutex<Option<u8>>> = Arc::default();
    let id = {
        let seen = Arc::clone(&seen);
        let language = Arc::clone(&language);
        let shared = Arc::downgrade(exec.shared());
        let asked = std::sync::atomic::AtomicBool::new(false);
        exec.shared().observers.add(Arc::new(move |s| {
            let Some(shared) = shared.upgrade() else {
                return;
            };
            if s.world.map == 0x15d
                && s.world.event_frame > 30
                && !asked.swap(true, std::sync::atomic::Ordering::Relaxed)
            {
                shared
                    .requests
                    .push(ffx_host::requests::Request::DevSave(6), Box::new(|_| {}));
            }
            if s.world.story != 111 {
                return;
            }
            if let Some(m) = s
                .messages
                .iter()
                .find(|m| m.state == 2 && !m.text.is_empty())
            {
                if let Ok(mut l) = language.lock() {
                    *l = Some(s.world.language);
                }
                if let Ok(mut x) = seen.lock() {
                    *x = Some(m.clone());
                }
                shared.stop_all(Stop::Milestone("a subtitle".into()));
            }
        }))
    };
    let stop = exec.run_entry().unwrap();
    exec.join_threads(Duration::from_secs(5));
    exec.shared().observers.remove(id);
    assert_eq!(stop, Stop::Milestone("a subtitle".into()));
    let m = seen.lock().unwrap().clone().unwrap();
    // decoded with the character set of the game's language (a fresh
    // GameSetting.ini: Japanese): the first line of the arrival, Tidus's
    // "Rikku!?" ("リュック！？")
    let language = language.lock().unwrap().expect("the language was read");
    // a fresh GameSetting.ini: Japanese
    assert_eq!(language, 0);
    let lang = ffx_text::Language::Japanese;
    let game = exe.parent().unwrap();
    let archive = ffx_vbf::Archive::open(game.join("data/FFX_Data.vbf")).unwrap();
    let table = archive
        .read(archive.find(&lang.charset_path()).unwrap())
        .unwrap();
    let text = ffx_text::Charset::from_table(&table, lang)
        .unwrap()
        .decode(&m.text);
    eprintln!("message {m:?} (language {language}, {lang:?}): {text:?}");
    assert!(text.contains("リュック"), "{text:?}");
}

/// A frontend's battle (`Request::Battle`): from Besaid (developer save
/// 7), battle field 3 (`cdsp00`), group 0, formation 0 starts as a random
/// encounter would, and the snapshots show the battle: `World.battle`,
/// the characters, and later a command window open.
#[test]
fn a_battle_starts_on_request() {
    use std::sync::{Arc, Mutex};
    let Some(exe) = exe() else { return };
    let opts = ffx_host::Options {
        documents: Some(documents("battle-request")),
        gfx: ffx_host::gfx::Config {
            backend: ffx_host::gfx::Choice::Null,
            no_record: true,
            ..ffx_host::gfx::Config::default()
        },
        ..ffx_host::Options::default()
    };
    let mut exec = ffx_host::boot_opts(&exe, ffx_host::ports::table(), &opts).unwrap();
    exec.set_budget(Duration::from_secs(300));
    let reply: Arc<Mutex<Option<Result<String, String>>>> = Arc::default();
    let window: Arc<Mutex<Option<Arc<ffx_state::Snapshot>>>> = Arc::default();
    let id = {
        let (reply, window) = (Arc::clone(&reply), Arc::clone(&window));
        let shared = Arc::downgrade(exec.shared());
        let step = std::sync::atomic::AtomicU32::new(0);
        exec.shared().observers.add(Arc::new(move |s| {
            use std::sync::atomic::Ordering::Relaxed;
            let Some(shared) = shared.upgrade() else {
                return;
            };
            match step.load(Relaxed) {
                0 if s.world.map == 0x15d && s.world.event_frame > 30 => {
                    shared
                        .requests
                        .push(ffx_host::requests::Request::DevSave(7), Box::new(|_| {}));
                    step.store(1, Relaxed);
                }
                1 if s.world.map == 0x46 && s.world.player_control => {
                    let reply = Arc::clone(&reply);
                    shared.requests.push(
                        ffx_host::requests::Request::Battle {
                            field: 3,
                            group: 0,
                            formation: 0,
                        },
                        Box::new(move |r| {
                            if let Ok(mut x) = reply.lock() {
                                *x = Some(r);
                            }
                        }),
                    );
                    step.store(2, Relaxed);
                }
                2 if s.world.battle && s.menus.battle_window != 0 => {
                    if let Ok(mut w) = window.lock() {
                        *w = Some(Arc::clone(s));
                    }
                    shared.stop_all(Stop::Milestone("a command window".into()));
                }
                _ => {}
            }
        }))
    };
    let stop = exec.run_entry().unwrap();
    exec.join_threads(Duration::from_secs(5));
    exec.shared().observers.remove(id);
    let r = reply.lock().unwrap().clone();
    eprintln!("reply: {r:?}");
    assert!(r.expect("served").is_ok());
    assert_eq!(stop, Stop::Milestone("a command window".into()));
    let s = window.lock().unwrap().clone().unwrap();
    eprintln!(
        "battle: level {} menus {:?} actors {}",
        s.world.level,
        s.menus,
        s.actors.len()
    );
    for a in &s.actors {
        eprintln!(
            "  {} {:#06x} hidden {:#x} bones {}",
            a.model,
            a.id,
            a.hidden,
            a.bones.len()
        );
    }
    assert!(s.world.battle);
    assert!(s.menus.battle_depth >= 0);
    // the battlers: Tidus and Yuna, the two monsters, alive, with HP
    eprintln!("battlers: {:?}", s.battlers);
    assert!(s.battlers.iter().any(|b| b.index == 0
        && b.hp > 0
        && b.hp <= b.max_hp
        && !b.dead
        && b.exists));
    assert!(
        s.battlers
            .iter()
            .filter(|b| b.index >= 0x14 && b.max_hp > 0 && !b.dead)
            .count()
            >= 2
    );
    assert!(s.menus.battle_ids.len() >= 2, "{:?}", s.menus);
}

/// The main menu's options (API 4): on Besaid (developer save 7) Triangle
/// opens the main menu; the snapshots list its options with their names
/// as the game shows them (`MsMenuGetText(9, id)`, decoded with the
/// game's character set: Japanese for a fresh `GameSetting.ini`).
#[test]
fn the_state_api_names_the_main_menu() {
    use std::sync::atomic::{AtomicU32, Ordering::Relaxed};
    use std::sync::{Arc, Mutex};
    let Some(exe) = exe() else { return };
    let opts = ffx_host::Options {
        documents: Some(documents("main-menu")),
        gfx: ffx_host::gfx::Config {
            backend: ffx_host::gfx::Choice::Null,
            no_record: true,
            ..ffx_host::gfx::Config::default()
        },
        ..ffx_host::Options::default()
    };
    let mut exec = ffx_host::boot_opts(&exe, ffx_host::ports::table(), &opts).unwrap();
    exec.set_budget(Duration::from_secs(300));
    let seen: Arc<Mutex<Option<ffx_state::Menus>>> = Arc::default();
    let language: Arc<Mutex<Option<u8>>> = Arc::default();
    let id = {
        let seen = Arc::clone(&seen);
        let language = Arc::clone(&language);
        let shared = Arc::downgrade(exec.shared());
        let step = AtomicU32::new(0);
        let at = AtomicU32::new(0);
        exec.shared().observers.add(Arc::new(move |s| {
            let Some(shared) = shared.upgrade() else {
                return;
            };
            match step.load(Relaxed) {
                0 if s.world.map == 0x15d && s.world.event_frame > 30 => {
                    shared
                        .requests
                        .push(ffx_host::requests::Request::DevSave(7), Box::new(|_| {}));
                    step.store(1, Relaxed);
                }
                // Triangle (the menu) held for a few updates, once the
                // beach has the control
                1 if s.world.map == 0x46 && s.world.player_control => {
                    shared.clock.buttons.store(0x10, Relaxed);
                    at.store(s.update, Relaxed);
                    step.store(2, Relaxed);
                }
                2 if s.update > at.load(Relaxed) + 4 => {
                    shared.clock.buttons.store(0, Relaxed);
                    step.store(3, Relaxed);
                }
                3 if s.menus.main_mask & 0x2 != 0
                    && s.menus.main_option_list.len() >= 4
                    && s.menus.main_option_list.iter().all(|o| !o.name.is_empty()) =>
                {
                    if let Ok(mut x) = seen.lock() {
                        *x = Some(s.menus.clone());
                    }
                    if let Ok(mut l) = language.lock() {
                        *l = Some(s.world.language);
                    }
                    shared.stop_all(Stop::Milestone("the main menu".into()));
                }
                _ => {}
            }
        }))
    };
    let stop = exec.run_entry().unwrap();
    exec.join_threads(Duration::from_secs(5));
    exec.shared().observers.remove(id);
    assert_eq!(stop, Stop::Milestone("the main menu".into()));
    let menus = seen.lock().unwrap().clone().unwrap();
    let game = exe.parent().unwrap();
    let archive = ffx_vbf::Archive::open(game.join("data/FFX_Data.vbf")).unwrap();
    // the snapshot's language: Japanese for a fresh GameSetting.ini
    assert_eq!(language.lock().unwrap().unwrap(), 0);
    let lang = ffx_text::Language::Japanese;
    let table = archive
        .read(archive.find(&lang.charset_path()).unwrap())
        .unwrap();
    let cs = ffx_text::Charset::from_table(&table, lang).unwrap();
    let names: Vec<String> = menus
        .main_option_list
        .iter()
        .map(|o| cs.decode(&o.name))
        .collect();
    eprintln!("main menu: cursor {} options {names:?}", menus.main_cursor);
    assert_eq!(
        menus.main_option_list.len(),
        usize::try_from(menus.main_options).unwrap()
    );
    // the Japanese main menu: スフィア盤 (Sphere Grid), アイテム, アビリティ (Abilities), ...
    assert!(names.iter().any(|n| n.contains("アビリティ")), "{names:?}");
}

/// The effects (API 5): in the deck's battle (Besaid, then `Battle { 3, 0,
/// 0 }`; Attack chosen, Circle twice), the snapshots show the effect
/// records in use, there from its start (`0x50001800`/`01` targeting the
/// party's characters 0 and 4, `0x40000405`/`06` the monsters' 20 and 21),
/// the monsters' each targeting its own character; the battle music; the
/// five screen filter slots.
#[test]
fn the_state_api_sees_a_battles_effects() {
    use std::sync::atomic::{AtomicU32, Ordering::Relaxed};
    use std::sync::{Arc, Mutex};
    let Some(exe) = exe() else { return };
    let opts = ffx_host::Options {
        documents: Some(documents("effects")),
        gfx: ffx_host::gfx::Config {
            backend: ffx_host::gfx::Choice::Null,
            no_record: true,
            ..ffx_host::gfx::Config::default()
        },
        ..ffx_host::Options::default()
    };
    let mut exec = ffx_host::boot_opts(&exe, ffx_host::ports::table(), &opts).unwrap();
    exec.set_budget(Duration::from_secs(300));
    let seen: Arc<Mutex<Option<ffx_state::Effects>>> = Arc::default();
    let id = {
        let seen = Arc::clone(&seen);
        let shared = Arc::downgrade(exec.shared());
        let step = AtomicU32::new(0);
        let at = AtomicU32::new(0);
        exec.shared().observers.add(Arc::new(move |s| {
            let Some(shared) = shared.upgrade() else {
                return;
            };
            let since = s.update.saturating_sub(at.load(Relaxed));
            match step.load(Relaxed) {
                0 if s.world.map == 0x15d && s.world.event_frame > 30 => {
                    shared
                        .requests
                        .push(ffx_host::requests::Request::DevSave(7), Box::new(|_| {}));
                    step.store(1, Relaxed);
                }
                1 if s.world.map == 0x46 && s.world.player_control => {
                    shared.requests.push(
                        ffx_host::requests::Request::Battle {
                            field: 3,
                            group: 0,
                            formation: 0,
                        },
                        Box::new(|_| {}),
                    );
                    step.store(2, Relaxed);
                }
                // the command window: Circle (Attack), then Circle (the
                // target), each a few updates
                2 if s.world.battle && s.menus.battle_window != 0 => {
                    shared.clock.buttons.store(0x20, Relaxed);
                    at.store(s.update, Relaxed);
                    step.store(3, Relaxed);
                }
                3 if since > 4 => {
                    shared.clock.buttons.store(0, Relaxed);
                    at.store(s.update, Relaxed);
                    step.store(4, Relaxed);
                }
                4 if since > 30 => {
                    shared.clock.buttons.store(0x20, Relaxed);
                    at.store(s.update, Relaxed);
                    step.store(5, Relaxed);
                }
                5 if since > 4 => {
                    shared.clock.buttons.store(0, Relaxed);
                    step.store(6, Relaxed);
                }
                6 if !s.effects.running.is_empty() => {
                    if let Ok(mut x) = seen.lock() {
                        *x = Some(s.effects.clone());
                    }
                    shared.stop_all(Stop::Milestone("effects".into()));
                }
                _ => {}
            }
        }))
    };
    let stop = exec.run_entry().unwrap();
    exec.join_threads(Duration::from_secs(5));
    exec.shared().observers.remove(id);
    assert_eq!(stop, Stop::Milestone("effects".into()));
    let e = seen.lock().unwrap().clone().unwrap();
    eprintln!("effects: {e:?}");
    assert!(e.running.iter().all(|r| r.id != 0 && r.id != u32::MAX));
    // the monsters' (characters 20, 21) bind effects, each on its own one
    for chr in [20u8, 21] {
        assert!(
            e.running
                .iter()
                .any(|r| r.id & 0xf000_0000 == 0x4000_0000 && r.targets == [chr]),
            "{e:?}"
        );
    }
    assert_eq!(e.filters.len(), 5);
    assert_ne!(e.music, -1, "{e:?}");
}

/// API 10 at Besaid (developer save 7): the shaders' global time advances
/// with the updates; the save sphere `f001` is drawn, its parts unskinned
/// with their base elements; every drawn character has a loaded model; the
/// map's display objects carry their morph values; bsil00's 2D level is
/// loaded but switched off. Then Triangle opens the main menu, the pad
/// moves to the config option (menu 8, slot 10) and Circle opens it: the config
/// rows with their titles and options in the game's encoding.
#[test]
#[allow(clippy::too_many_lines)] // one walk through the game, step by step
fn the_state_api_sees_the_beach_and_the_config_menu() {
    use std::sync::atomic::{AtomicU32, Ordering::Relaxed};
    use std::sync::{Arc, Mutex};
    type Seen = Vec<Arc<ffx_state::Snapshot>>;
    let Some(exe) = exe() else { return };
    let opts = ffx_host::Options {
        documents: Some(documents("beach-config")),
        gfx: ffx_host::gfx::Config {
            backend: ffx_host::gfx::Choice::Null,
            no_record: true,
            ..ffx_host::gfx::Config::default()
        },
        ..ffx_host::Options::default()
    };
    let mut exec = ffx_host::boot_opts(&exe, ffx_host::ports::table(), &opts).unwrap();
    exec.set_budget(Duration::from_secs(300));
    // the beach at the control, 60 updates on, and the config menu open
    let seen: Arc<Mutex<Seen>> = Arc::default();
    let last: Arc<Mutex<Option<Arc<ffx_state::Snapshot>>>> = Arc::default();
    let id = {
        let (seen, last) = (Arc::clone(&seen), Arc::clone(&last));
        let shared = Arc::downgrade(exec.shared());
        let step = AtomicU32::new(0);
        let at = AtomicU32::new(0);
        exec.shared().observers.add(Arc::new(move |s| {
            let Some(shared) = shared.upgrade() else {
                return;
            };
            if let Ok(mut l) = last.lock() {
                *l = Some(Arc::clone(s));
            }
            let since = s.update.saturating_sub(at.load(Relaxed));
            let press = |b: u32, next: u32| {
                shared.clock.buttons.store(b, Relaxed);
                at.store(s.update, Relaxed);
                step.store(next, Relaxed);
            };
            let keep = || {
                if let Ok(mut v) = seen.lock() {
                    v.push(Arc::clone(s));
                }
            };
            let player = s.actors.iter().any(|a| a.player && a.hidden == 0);
            match step.load(Relaxed) {
                0 if s.world.map == 0x15d && s.world.event_frame > 30 => {
                    shared
                        .requests
                        .push(ffx_host::requests::Request::DevSave(7), Box::new(|_| {}));
                    step.store(1, Relaxed);
                }
                1 if s.world.map == 0x46 && s.world.player_control && player => {
                    keep();
                    press(0, 2);
                }
                // 60 updates on: the second beach snapshot, then Triangle
                2 if since >= 60 => {
                    keep();
                    press(0x10, 3);
                }
                3 if since > 4 => press(0, 4),
                // the main menu's options: down to menu 8 (`TkMenuMainOpenOption`: slot 10), then Circle
                4 if s.menus.main_mask & 0x2 != 0 && s.menus.main_option_list.len() >= 4 => {
                    let target = s.menus.main_option_list.iter().position(|o| o.id == 8);
                    match target {
                        Some(t) if usize::try_from(s.menus.main_cursor) == Ok(t) => {
                            press(0x20, 7);
                        }
                        Some(_) => press(0x4000, 5),
                        None => {}
                    }
                }
                5 if since > 3 => press(0, 6),
                6 if since > 8 => step.store(4, Relaxed),
                7 if since > 3 => press(0, 8),
                8 if s.menus.config.open && !s.menus.config.rows.is_empty() && since > 30 => {
                    keep();
                    shared.stop_all(Stop::Milestone("the config menu".into()));
                }
                _ => {}
            }
        }))
    };
    let stop = exec.run_entry().unwrap();
    exec.join_threads(Duration::from_secs(5));
    exec.shared().observers.remove(id);
    let seen = seen.lock().unwrap().clone();
    if stop != Stop::Milestone("the config menu".into())
        && let Some(l) = last.lock().unwrap().as_ref()
    {
        eprintln!("stopped: {stop:?}; menus {:?}", l.menus);
    }
    assert_eq!(stop, Stop::Milestone("the config menu".into()));
    let (first, beach, config) = (&seen[0], &seen[1], &seen[2]);
    // the global time: about 60 updates of 1/30 s apart
    eprintln!(
        "global time {} -> {} over {} updates",
        first.look.global_time,
        beach.look.global_time,
        beach.update - first.update
    );
    let dt = beach.look.global_time - first.look.global_time;
    assert!((1.5..2.5).contains(&dt), "{dt}");
    // GlobalTime restarts at the map load, the game time does not (64 s or
    // more by now): the read is the scene clock, not the game time
    assert!(
        beach.look.global_time < 30.0 && f64::from(beach.look.global_time) < beach.game_time,
        "global time {} game time {}",
        beach.look.global_time,
        beach.game_time
    );
    for a in &beach.actors {
        eprintln!(
            "slot {} {:#06x} {} hidden {:#x} alpha {} drawn {} flags {:#x} flags2 {:#x} models {:?}",
            a.slot, a.id, a.model, a.hidden, a.alpha, a.drawn, a.flags, a.flags2, a.models
        );
    }
    let sphere = beach
        .actors
        .iter()
        .find(|a| a.model == "f001")
        .expect("the save sphere");
    assert!(sphere.drawn);
    assert!(sphere.models.len() >= 2, "{:?}", sphere.models);
    assert!(
        sphere
            .models
            .iter()
            .all(|m| !m.skinned && usize::from(m.base) < sphere.bones.len())
    );
    // the player's character is drawn; nothing hidden or purged is
    assert!(beach.actors.iter().any(|a| a.player && a.drawn));
    assert!(
        beach
            .actors
            .iter()
            .filter(|a| a.drawn)
            .all(|a| a.hidden == 0 && a.flags2 & 0x20 == 0)
    );
    // the display objects
    let dobjs: Vec<_> = beach.objects.iter().filter(|o| o.dobj).collect();
    for o in &beach.objects {
        let before = first.objects.iter().find(|p| p.id == o.id);
        eprintln!(
            "object {} dobj {} morph {} {} (before {:?})",
            o.id,
            o.dobj,
            o.morph,
            o.morph_v,
            before.map(|p| (p.morph, p.morph_v))
        );
    }
    assert!(!dobjs.is_empty(), "{:?}", beach.objects);
    // bsil00: the 2D level loaded, its pass off (its one layer at depth
    // 0.01)
    eprintln!("layers {:?}", beach.layers);
    assert!(beach.layers.valid);
    assert!(beach.layers.loaded && beach.layers.off && !beach.layers.drawn);
    // the config menu
    let c = &config.menus.config;
    eprintln!(
        "config: cursor {} pc row {} rows {}",
        c.cursor,
        c.pc_row,
        c.rows.len()
    );
    let game = exe.parent().unwrap();
    let archive = ffx_vbf::Archive::open(game.join("data/FFX_Data.vbf")).unwrap();
    let lang = ffx_text::Language::Japanese;
    let table = archive
        .read(archive.find(&lang.charset_path()).unwrap())
        .unwrap();
    let cs = ffx_text::Charset::from_table(&table, lang).unwrap();
    for (i, r) in c.rows.iter().enumerate() {
        let options: Vec<String> = r.options.iter().map(|o| cs.decode(&o.text)).collect();
        eprintln!(
            "  row {i}: {:#x} {:?} value {} enabled {} options {:?} ids {:?}",
            r.title_id,
            cs.decode(&r.title),
            r.value,
            r.enabled,
            options,
            r.options.iter().map(|o| o.id).collect::<Vec<_>>()
        );
    }
    assert_eq!(config.menus.main_mask & 0x400, 0x400);
    assert!(c.rows.len() >= 4);
    for (i, r) in c.rows.iter().enumerate() {
        assert!(!r.options.is_empty(), "{r:?}");
        assert!(r.value < u32::try_from(r.options.len()).unwrap());
        if i64::from(c.pc_row) != i64::try_from(i).unwrap() {
            assert!(!r.title.is_empty() && r.options.iter().all(|o| !o.text.is_empty()));
        }
    }
    assert!(!beach.menus.config.open && !beach.menus.battle_end.open);
}

/// API 10's battle results: the deck's battle (Besaid, then `Battle { 3,
/// 0, 0 }`) won by pressing Circle (Attack on the first target), the
/// results screen runs (`main_mask` bit `0x20000`): its state, the gil
/// won, the rows of the party with the AP still to count.
#[test]
#[allow(clippy::too_many_lines)] // one walk through the game, step by step
fn the_state_api_sees_the_battle_results() {
    use std::sync::atomic::{AtomicU32, Ordering::Relaxed};
    use std::sync::{Arc, Mutex};
    type Seen = Vec<Arc<ffx_state::Snapshot>>;
    let Some(exe) = exe() else { return };
    let opts = ffx_host::Options {
        documents: Some(documents("battle-results")),
        gfx: ffx_host::gfx::Config {
            backend: ffx_host::gfx::Choice::Null,
            no_record: true,
            ..ffx_host::gfx::Config::default()
        },
        ..ffx_host::Options::default()
    };
    let mut exec = ffx_host::boot_opts(&exe, ffx_host::ports::table(), &opts).unwrap();
    exec.set_budget(Duration::from_secs(400));
    // the results screen's snapshots
    let seen: Arc<Mutex<Seen>> = Arc::default();
    let id = {
        let seen = Arc::clone(&seen);
        let shared = Arc::downgrade(exec.shared());
        let step = AtomicU32::new(0);
        exec.shared().observers.add(Arc::new(move |s| {
            let Some(shared) = shared.upgrade() else {
                return;
            };
            match step.load(Relaxed) {
                0 if s.world.map == 0x15d && s.world.event_frame > 30 => {
                    shared
                        .requests
                        .push(ffx_host::requests::Request::DevSave(7), Box::new(|_| {}));
                    step.store(1, Relaxed);
                }
                1 if s.world.map == 0x46 && s.world.player_control => {
                    shared.requests.push(
                        ffx_host::requests::Request::Battle {
                            field: 3,
                            group: 0,
                            formation: 0,
                        },
                        Box::new(|_| {}),
                    );
                    step.store(2, Relaxed);
                }
                // in battle until the results: Circle 3 updates of 10
                2 if s.menus.battle_end.open => {
                    shared.clock.buttons.store(0, Relaxed);
                    step.store(3, Relaxed);
                }
                2 if s.world.battle => {
                    let b = if s.update % 10 < 3 { 0x20 } else { 0 };
                    shared.clock.buttons.store(b, Relaxed);
                }
                // the results, every update for 4 seconds
                3 => {
                    if let Ok(mut v) = seen.lock() {
                        v.push(Arc::clone(s));
                        if v.len() >= 120 {
                            shared.stop_all(Stop::Milestone("the results".into()));
                        }
                    }
                }
                _ => {}
            }
        }))
    };
    let stop = exec.run_entry().unwrap();
    exec.join_threads(Duration::from_secs(5));
    exec.shared().observers.remove(id);
    assert_eq!(stop, Stop::Milestone("the results".into()));
    let seen = seen.lock().unwrap().clone();
    let ap = |e: &ffx_state::BattleEnd, chr: usize| {
        u32::from_le_bytes(e.ap[4 * chr..4 * chr + 4].try_into().unwrap())
    };
    for s in seen.iter().step_by(10) {
        let e = &s.menus.battle_end;
        eprintln!(
            "update {} state {:#x} gil won {} left {} step {} party gil {} rows {:?} ap {:?} items {:?} gear {:?} side {} cursors {} {} top {} list {} full {} done {}",
            s.update,
            e.state,
            e.gil_won,
            e.gil_left,
            e.gil_step,
            s.save.gil,
            e.rows,
            (0..18).map(|c| ap(e, c)).collect::<Vec<_>>(),
            e.items,
            e.gear,
            e.side,
            e.cursor,
            e.cursor_won,
            e.top,
            e.gear_list,
            e.gear_full,
            e.done
        );
    }
    let first = &seen[0].menus.battle_end;
    assert!(first.open);
    assert_eq!(first.ap.len(), 72);
    assert!(first.gil_won > 0, "{first:?}");
    assert!(!first.rows.is_empty(), "{first:?}");
    assert!(first.rows.iter().all(|r| (0..18).contains(&r.character)));
    // the counts run down: AP and gil left at the end no more than at the
    // start
    let last = &seen[seen.len() - 1].menus.battle_end;
    assert!(last.gil_left <= first.gil_left);
    assert!((0..18).all(|c| ap(last, c) <= ap(first, c)));
    assert!(seen.iter().any(|s| s.menus.battle_end.state != first.state));
}

/// API 11 at Besaid (developer save 7): the map change is seen before the
/// beach starts; the beach is a 3D map (no squeeze, no offset) with the
/// boosters as a new game has them; Circle at the save sphere opens its
/// two-way choice, whose cursor the snapshots follow before the confirm
/// (Down, Up, Down) and whose pick they show after it. Then developer save
/// 12 (the S.S. Liki's corridor, `slik04`), a 2D-background room: the
/// camera keeps the PS2's 4/3 and gets the squeeze, its offset `1/3 - 2
/// (map_pan + pan)`.
#[test]
#[allow(clippy::too_many_lines)] // one walk through the game, step by step
fn the_state_api_follows_a_choice_and_a_2d_rooms_camera() {
    use std::sync::atomic::{AtomicU32, Ordering::Relaxed};
    use std::sync::{Arc, Mutex};
    type Seen = Vec<(&'static str, Arc<ffx_state::Snapshot>)>;
    let Some(exe) = exe() else { return };
    let opts = ffx_host::Options {
        documents: Some(documents("choice-2d")),
        gfx: ffx_host::gfx::Config {
            backend: ffx_host::gfx::Choice::Null,
            no_record: true,
            ..ffx_host::gfx::Config::default()
        },
        ..ffx_host::Options::default()
    };
    let mut exec = ffx_host::boot_opts(&exe, ffx_host::ports::table(), &opts).unwrap();
    exec.set_budget(Duration::from_secs(400));
    let seen: Arc<Mutex<Seen>> = Arc::default();
    let changes: Arc<Mutex<Vec<(u32, u16)>>> = Arc::default();
    let id = {
        let (seen, changes) = (Arc::clone(&seen), Arc::clone(&changes));
        let shared = Arc::downgrade(exec.shared());
        let step = AtomicU32::new(0);
        let at = AtomicU32::new(0);
        exec.shared().observers.add(Arc::new(move |s| {
            let Some(shared) = shared.upgrade() else {
                return;
            };
            if s.world.map_change
                && let Ok(mut c) = changes.lock()
            {
                c.push((s.update, s.world.map));
            }
            let since = s.update.saturating_sub(at.load(Relaxed));
            let keep = |what: &'static str| {
                if let Ok(mut v) = seen.lock() {
                    v.push((what, Arc::clone(s)));
                }
            };
            let press = |b: u32, next: u32| {
                shared.clock.buttons.store(b, Relaxed);
                at.store(s.update, Relaxed);
                step.store(next, Relaxed);
            };
            let choice = s.messages.iter().find(|m| m.mode & 0x02 != 0);
            match step.load(Relaxed) {
                0 if s.world.map == 0x15d && s.world.event_frame > 30 => {
                    shared
                        .requests
                        .push(ffx_host::requests::Request::DevSave(7), Box::new(|_| {}));
                    step.store(1, Relaxed);
                }
                // the beach at the control, the fade-in over: Circle
                1 if s.world.map == 0x46
                    && s.world.player_control
                    && s.effects.filters.iter().all(|f| !f.drawn) =>
                {
                    keep("beach");
                    press(0x20, 2);
                }
                2 if since > 3 => press(0, 3),
                // the choice open (state 2): Down, Up, Down, each kept
                3 if choice.is_some_and(|m| m.state == 2) && since > 20 => {
                    keep("open");
                    press(0x4000, 4);
                }
                4 if since > 3 => press(0, 5),
                5 if since > 10 => {
                    keep("down");
                    press(0x1000, 6);
                }
                6 if since > 3 => press(0, 7),
                7 if since > 10 => {
                    keep("up");
                    press(0x4000, 8);
                }
                8 if since > 3 => press(0, 9),
                9 if since > 10 => {
                    keep("down again");
                    press(0x20, 10);
                }
                10 if since > 3 => press(0, 11),
                11 if choice.is_some_and(|m| m.picked >= 0) => {
                    keep("picked");
                    step.store(12, Relaxed);
                }
                // the choice gone and the control back: the corridor
                12 if s.messages.is_empty() && s.world.player_control => {
                    shared
                        .requests
                        .push(ffx_host::requests::Request::DevSave(12), Box::new(|_| {}));
                    step.store(13, Relaxed);
                }
                13 if s.world.map == 0x94 && s.world.player_control && !s.world.scene_loading => {
                    keep("corridor");
                    shared.stop_all(Stop::Milestone("the corridor".into()));
                }
                _ => {}
            }
        }))
    };
    let stop = exec.run_entry().unwrap();
    exec.join_threads(Duration::from_secs(5));
    exec.shared().observers.remove(id);
    let seen = seen.lock().unwrap().clone();
    let changes = changes.lock().unwrap().clone();
    for (what, s) in &seen {
        eprintln!(
            "{what}: update {} map {:#x} camera {:?} boosters {:?} messages {:?}",
            s.update,
            s.world.map,
            (
                s.camera.aspect,
                s.camera.shift_x,
                s.camera.shift_y,
                s.camera.pan,
                s.camera.map_pan,
                s.camera.squeezed,
                s.camera.scene_3d,
                s.camera.offset_y
            ),
            s.boosters,
            s.messages
                .iter()
                .map(|m| (m.window, m.state, m.mode, m.choice, m.picked, m.vars.len()))
                .collect::<Vec<_>>()
        );
    }
    eprintln!("map changes seen: {changes:?}");
    assert_eq!(stop, Stop::Milestone("the corridor".into()));
    let get = |what: &str| &seen.iter().find(|(w, _)| *w == what).unwrap().1;
    // a map change seen at a boundary before the beach (a developer save
    // served at a boundary loads its map within the next update, so only
    // the script's own jumps are seen)
    let (beach, corridor) = (get("beach"), get("corridor"));
    assert!(
        changes.iter().any(|&(u, _)| u < beach.update),
        "{changes:?}"
    );
    assert!(!beach.world.map_change && !beach.world.scene_loading);
    // Besaid: a 3D map, no squeeze; a new game's boosters
    let c = &beach.camera;
    assert!(c.scene_3d && !c.squeezed && c.offset_y == 0.0, "{c:?}");
    assert!((c.aspect - 16.0 / 9.0).abs() < 1e-4);
    let b = &beach.boosters;
    assert_eq!(
        (
            b.speed_level,
            b.speeding,
            b.powerup,
            b.encounters,
            b.auto_battle
        ),
        (0, false, false, 1, false)
    );
    assert!((b.speed - 1.0).abs() < f32::EPSILON);
    // the choice: the cursor before the confirm, the pick after it
    let choice = |what: &str| {
        let m = get(what)
            .messages
            .iter()
            .find(|m| m.mode & 0x02 != 0)
            .unwrap();
        (m.choice, m.picked)
    };
    assert_eq!(choice("open"), (0, -1));
    assert_eq!(choice("down"), (1, -1));
    assert_eq!(choice("up"), (0, -1));
    assert_eq!(choice("down again"), (1, -1));
    assert_eq!(choice("picked"), (1, 1));
    // the corridor: a 2D room, the PS2's aspect, squeezed, `d` as the game
    // computes it
    let c = &corridor.camera;
    assert!(!c.scene_3d && c.squeezed, "{c:?}");
    assert!((c.aspect - 4.0 / 3.0).abs() < 1e-4);
    let d = 0.333_333_37 - 2.0 * (f64::from(c.map_pan) + f64::from(c.pan));
    assert!((f64::from(c.offset_y) - d).abs() < 1e-6, "{c:?}");
    assert!((0.034..=0.296).contains(&c.pan), "{c:?}");
    assert_eq!(corridor.layers.pan.to_bits(), c.pan.to_bits());
}

/// API 11 in the deck's battle (Besaid, then `Battle { 3, 0, 0 }`): the
/// command list puts its hand cursor on the screen; Circle (Attack) opens
/// the target selection on the first monster (character 0x14, its bit in
/// the mask, two targets on the monsters' side), Right moves it to the
/// other and back (wrapping), Cross closes it.
#[test]
#[allow(clippy::too_many_lines)] // one walk through the game, step by step
fn the_state_api_sees_the_battle_target_selection() {
    use std::sync::atomic::{AtomicU32, Ordering::Relaxed};
    use std::sync::{Arc, Mutex};
    type Seen = Vec<(&'static str, Arc<ffx_state::Snapshot>)>;
    let Some(exe) = exe() else { return };
    let opts = ffx_host::Options {
        documents: Some(documents("battle-target")),
        gfx: ffx_host::gfx::Config {
            backend: ffx_host::gfx::Choice::Null,
            no_record: true,
            ..ffx_host::gfx::Config::default()
        },
        ..ffx_host::Options::default()
    };
    let mut exec = ffx_host::boot_opts(&exe, ffx_host::ports::table(), &opts).unwrap();
    exec.set_budget(Duration::from_secs(400));
    let seen: Arc<Mutex<Seen>> = Arc::default();
    let id = {
        let seen = Arc::clone(&seen);
        let shared = Arc::downgrade(exec.shared());
        let step = AtomicU32::new(0);
        let at = AtomicU32::new(0);
        exec.shared().observers.add(Arc::new(move |s| {
            let Some(shared) = shared.upgrade() else {
                return;
            };
            let since = s.update.saturating_sub(at.load(Relaxed));
            let keep = |what: &'static str| {
                if let Ok(mut v) = seen.lock() {
                    v.push((what, Arc::clone(s)));
                }
            };
            let press = |b: u32, next: u32| {
                shared.clock.buttons.store(b, Relaxed);
                at.store(s.update, Relaxed);
                step.store(next, Relaxed);
            };
            let t = &s.menus.battle_target;
            match step.load(Relaxed) {
                0 if s.world.map == 0x15d && s.world.event_frame > 30 => {
                    shared
                        .requests
                        .push(ffx_host::requests::Request::DevSave(7), Box::new(|_| {}));
                    step.store(1, Relaxed);
                }
                1 if s.world.map == 0x46 && s.world.player_control => {
                    shared.requests.push(
                        ffx_host::requests::Request::Battle {
                            field: 3,
                            group: 0,
                            formation: 0,
                        },
                        Box::new(|_| {}),
                    );
                    step.store(2, Relaxed);
                }
                // the command list, its hand drawn; Circle once the window
                // takes input
                2 if s.world.battle && s.menus.battle_window != 0 && !t.hands.is_empty() => {
                    keep("commands");
                    press(0, 20);
                }
                20 if since > 30 => press(0x20, 3),
                3 if since > 3 => press(0, 4),
                4 if t.open && since > 20 => {
                    keep("target");
                    press(0x2000, 5);
                }
                5 if since > 3 => press(0, 6),
                6 if since > 20 => {
                    keep("right");
                    press(0x2000, 7);
                }
                7 if since > 3 => press(0, 8),
                8 if since > 20 => {
                    keep("right again");
                    press(0x40, 9);
                }
                9 if since > 3 => press(0, 10),
                10 if since > 20 => {
                    keep("cancelled");
                    shared.stop_all(Stop::Milestone("the target selection".into()));
                }
                _ => {}
            }
        }))
    };
    let stop = exec.run_entry().unwrap();
    exec.join_threads(Duration::from_secs(5));
    exec.shared().observers.remove(id);
    let seen = seen.lock().unwrap().clone();
    for (what, s) in &seen {
        eprintln!(
            "{what}: update {} depth {} {:?}",
            s.update, s.menus.battle_depth, s.menus.battle_target
        );
    }
    assert_eq!(stop, Stop::Milestone("the target selection".into()));
    let get = |what: &str| {
        &seen
            .iter()
            .find(|(w, _)| *w == what)
            .unwrap()
            .1
            .menus
            .battle_target
    };
    // the command list: kind 0, its hand on the PS2 screen
    let c = get("commands");
    assert!(!c.open && c.kind == 0, "{c:?}");
    assert!(
        c.hands
            .iter()
            .all(|h| (0..512).contains(&h.x) && (0..448).contains(&h.y) && (h.x, h.y) != (0, 0)),
        "{c:?}"
    );
    // the target: one monster, its bit, two on its side
    let first = get("target");
    assert!(first.open && first.kind == 1, "{first:?}");
    assert!((0x14..0x1c).contains(&first.character), "{first:?}");
    assert_eq!(first.mask, 1 << first.character);
    assert_eq!(first.count, 2);
    // Right: the other monster, then back (the list wraps)
    let right = get("right");
    assert!(
        right.open && right.character != first.character,
        "{right:?}"
    );
    assert_eq!(right.mask, 1 << right.character);
    assert_eq!(right.cursor, (first.cursor + 1) % 2);
    let again = get("right again");
    assert_eq!(
        (again.character, again.cursor),
        (first.character, first.cursor)
    );
    // Cross: back to the command list
    assert!(!get("cancelled").open);
}

/// The real clock's game speed (`Options::speed`, `Clock::set_speed`): at
/// 0.5 the limiter holds 15 updates a second and game time runs at half
/// the host's; set to 0.25 at run time, 7.5 a second, the game time going
/// on from where it was.
#[test]
fn the_real_clock_runs_at_the_game_speed() {
    use std::time::Instant;
    let Some(exe) = exe() else { return };
    let opts = ffx_host::Options {
        documents: Some(documents("game-speed")),
        gfx: ffx_host::gfx::Config {
            backend: ffx_host::gfx::Choice::Null,
            no_record: true,
            ..ffx_host::gfx::Config::default()
        },
        real_clock: true,
        skip_opening: true,
        speed: Some(0.5),
        ..ffx_host::Options::default()
    };
    let mut exec = ffx_host::boot_opts(&exe, ffx_host::ports::table(), &opts).unwrap();
    exec.set_budget(Duration::from_secs(300));
    let shared = std::sync::Arc::clone(exec.shared());
    let watch = std::thread::spawn(move || {
        let at = |n: u32| {
            while shared.clock.frame() < n {
                if shared.stopped().is_some() {
                    return None;
                }
                std::thread::sleep(Duration::from_micros(200));
            }
            Some((Instant::now(), shared.clock.now()))
        };
        // windows of 30 updates: their rate (a second's updates) and the
        // game time over the host's
        let window = |from: u32, to: u32| -> Option<Vec<(f64, f64)>> {
            let mut last = at(from)?;
            let mut out = Vec::new();
            for n in (from + 30..=to).step_by(30) {
                let t = at(n)?;
                let host = t.0.duration_since(last.0).as_secs_f64();
                out.push((30.0 / host, (t.1 - last.1) / host));
                last = t;
            }
            Some(out)
        };
        let half = window(90, 240)?;
        let before = shared.clock.now();
        shared.clock.set_speed(0.25);
        let after = shared.clock.now();
        let quarter = window(shared.clock.frame() + 15, shared.clock.frame() + 15 + 90)?;
        shared.stop_all(Stop::Milestone("speeds measured".into()));
        Some((half, quarter, after - before))
    });
    let stop = exec.run_entry().unwrap();
    exec.join_threads(Duration::from_secs(5));
    let (half, quarter, jump) = watch
        .join()
        .unwrap()
        .unwrap_or_else(|| panic!("stopped first: {stop}"));
    eprintln!(
        "speed 0.5: (updates/s, game s/host s) {half:.3?}; 0.25: {quarter:.3?}; set_speed moved the clock by {jump:.6} s"
    );
    let median = |v: &[(f64, f64)], k: usize| {
        let mut x: Vec<f64> = v.iter().map(|p| if k == 0 { p.0 } else { p.1 }).collect();
        x.sort_by(f64::total_cmp);
        x[x.len() / 2]
    };
    assert!((14.7..=15.2).contains(&median(&half, 0)), "{half:.3?}");
    assert!((0.49..=0.51).contains(&median(&half, 1)), "{half:.3?}");
    assert!((7.3..=7.6).contains(&median(&quarter, 0)), "{quarter:.3?}");
    assert!(
        (0.245..=0.255).contains(&median(&quarter, 1)),
        "{quarter:.3?}"
    );
    // no jump at the change
    assert!((0.0..0.01).contains(&jump), "{jump}");
}
