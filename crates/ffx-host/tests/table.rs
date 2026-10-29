//! The `ported!` lists in ffx-host's function table (docs/host.md, "One
//! source of truth"), on the player's `FFX.exe` on Linux. Needs
//! `FFX_GAME_DIR`; skips without it.

use std::path::PathBuf;
use std::time::Duration;

use ffx_bridge::Mem as _;
use ffx_host::exec::{Exec, Stop};
use ffx_host::table::{Conv, Port, Table};

fn exe() -> Option<PathBuf> {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return None;
    };
    Some(PathBuf::from(dir).join("FFX.exe"))
}

#[test]
fn every_ported_line_is_an_entry() {
    let all = ffx_host_ports::hooks();
    let (kept, held) = ffx_host_ports::table_hooks();
    let table = ffx_host::ports::ported();
    eprintln!(
        "ported! entries: {}, in the table: {}, held (host addresses): {held}",
        all.len(),
        table.len()
    );
    assert!(all.len() > 15_000);
    assert_eq!(kept.len() + held, all.len());
    // a few conventions and word counts, as the lines declare them
    let e = table.get(0x0079_a0c0).expect("MsCheckRange");
    assert_eq!((e.name, e.conv, e.words), ("MsCheckRange", Conv::Cdecl, 3));
    let e = table.get(0x006e_cc40).expect("Matrix__copy");
    assert_eq!((e.conv, e.words), (Conv::Thiscall, 1));
    assert!(matches!(e.port, Port::Routed(_)));
}

/// What a parity case compares besides the memory.
#[derive(Clone, Copy, Debug)]
enum Res {
    Eax,
    St0,
}

/// One call: the original's address, convention, `ecx`, stack words
/// (buffer indices `B(i)` are replaced by the buffers' addresses).
struct Case {
    name: &'static str,
    addr: u32,
    conv: Conv,
    ecx: Arg,
    words: &'static [Arg],
    res: Res,
}

#[derive(Clone, Copy)]
enum Arg {
    W(u32),
    B(usize),
}

const BUFS: usize = 4;
const BUF: u32 = 0x40;

/// Code in the arena that calls `case` the way the game does, from
/// Unicorn: the words pushed, `ecx` set, the call, the stack popped for
/// cdecl, a float result stored to `out`; returns `eax`.
fn caller(exec: &mut Exec, case: &Case, bufs: &[u32; BUFS], out: u32) -> u32 {
    let arg = |a: Arg| match a {
        Arg::W(w) => w,
        Arg::B(i) => bufs[i],
    };
    let at = exec.alloc_code(&[0; 96]).unwrap();
    let mut code = Vec::new();
    for &w in case.words.iter().rev() {
        code.push(0x68);
        code.extend_from_slice(&arg(w).to_le_bytes());
    }
    code.push(0xb9); // mov ecx, imm32
    code.extend_from_slice(&arg(case.ecx).to_le_bytes());
    code.push(0xe8); // call rel32
    let next = at + u32::try_from(code.len()).unwrap() + 4;
    code.extend_from_slice(&case.addr.wrapping_sub(next).to_le_bytes());
    if !case.conv.callee_pops() && !case.words.is_empty() {
        code.extend_from_slice(&[0x83, 0xc4, u8::try_from(4 * case.words.len()).unwrap()]);
    }
    if matches!(case.res, Res::St0) {
        code.extend_from_slice(&[0xdd, 0x1d]); // fstp qword [out]
        code.extend_from_slice(&out.to_le_bytes());
    }
    code.push(0xc3);
    exec.write_code(at, &code);
    match exec.call_cdecl(at, &[]) {
        Ok(eax) => eax,
        Err(e) => panic!("{}: {e} ({:?})", case.name, exec.stopped()),
    }
}

/// The buffers, filled with the same floats in both executors.
fn buffers(exec: &mut Exec, seed: u32) -> ([u32; BUFS], u32) {
    let block = exec.alloc_code(&[0; (BUF as usize) * BUFS + 16]).unwrap();
    let mut mem = exec.mem();
    let mut bufs = [0; BUFS];
    for (i, b) in bufs.iter_mut().enumerate() {
        *b = block + BUF * u32::try_from(i).unwrap();
        for k in 0..BUF / 4 {
            #[allow(clippy::cast_precision_loss)]
            let v = ((seed * 7 + k * 3 + u32::try_from(i).unwrap() * 11) % 17) as f32 * 0.375 - 2.5;
            mem.write(*b + 4 * k, v);
        }
    }
    (bufs, block + BUF * u32::try_from(BUFS).unwrap())
}

fn snapshot(exec: &Exec, bufs: &[u32; BUFS]) -> Vec<u8> {
    let mem = exec.mem();
    let mut out = vec![0; (BUF as usize) * BUFS];
    mem.read_bytes(bufs[0], &mut out);
    out
}

#[test]
#[allow(clippy::too_many_lines)] // the cases
fn ported_functions_match_unicorn_through_the_table() {
    use Arg::{B, W};
    const HALF: u32 = 0x3f00_0000; // 0.5
    let Some(exe) = exe() else { return };
    let cases = [
        Case {
            name: "MsCheckRange",
            addr: 0x0079_a0c0,
            conv: Conv::Cdecl,
            ecx: W(0),
            words: &[W(9), W(1), W(5)],
            res: Res::Eax,
        },
        Case {
            name: "FFXVu0CopyVector",
            addr: 0x006e_d370,
            conv: Conv::Cdecl,
            ecx: W(0),
            words: &[B(0), B(1)],
            res: Res::Eax,
        },
        Case {
            name: "FFXVu0AddVector",
            addr: 0x006e_d290,
            conv: Conv::Cdecl,
            ecx: W(0),
            words: &[B(0), B(1), B(2)],
            res: Res::Eax,
        },
        Case {
            name: "FFXVu0SubVector",
            addr: 0x006e_d940,
            conv: Conv::Cdecl,
            ecx: W(0),
            words: &[B(0), B(1), B(2)],
            res: Res::Eax,
        },
        Case {
            name: "FFXVu0MulVector",
            addr: 0x006e_d5d0,
            conv: Conv::Cdecl,
            ecx: W(0),
            words: &[B(0), B(1), B(2)],
            res: Res::Eax,
        },
        Case {
            name: "FFXVu0ScaleVector",
            addr: 0x006e_d8f0,
            conv: Conv::Cdecl,
            ecx: W(0),
            words: &[B(0), B(1), W(HALF)],
            res: Res::Eax,
        },
        Case {
            name: "FFXVu0InnerProduct",
            addr: 0x0062_fce0,
            conv: Conv::Cdecl,
            ecx: W(0),
            words: &[B(1), B(2)],
            res: Res::St0,
        },
        Case {
            name: "FFXVu0OuterProduct",
            addr: 0x006e_d720,
            conv: Conv::Cdecl,
            ecx: W(0),
            words: &[B(0), B(1), B(2)],
            res: Res::Eax,
        },
        Case {
            name: "FFXVu0InterVector",
            addr: 0x0062_fd10,
            conv: Conv::Cdecl,
            ecx: W(0),
            words: &[B(0), B(1), B(2), W(HALF)],
            res: Res::Eax,
        },
        Case {
            name: "FFXVu0ApplyMatrix",
            addr: 0x006e_d2e0,
            conv: Conv::Cdecl,
            ecx: W(0),
            words: &[B(0), B(1), B(2)],
            res: Res::Eax,
        },
        Case {
            name: "FFXVu0UnitMatrix",
            addr: 0x006e_d9a0,
            conv: Conv::Cdecl,
            ecx: W(0),
            words: &[B(0)],
            res: Res::Eax,
        },
        Case {
            name: "FFXVu0DivVectorXYZ",
            addr: 0x006e_d420,
            conv: Conv::Cdecl,
            ecx: W(0),
            words: &[B(0), B(1), W(HALF)],
            res: Res::Eax,
        },
        Case {
            name: "FFXVu0CopyMatrix",
            addr: 0x006e_d360,
            conv: Conv::Cdecl,
            ecx: W(0),
            words: &[B(0), B(1)],
            res: Res::Eax,
        },
        Case {
            name: "Matrix_Identity",
            addr: 0x006e_ce60,
            conv: Conv::Thiscall,
            ecx: B(0),
            words: &[],
            res: Res::Eax,
        },
        Case {
            name: "Matrix__copy",
            addr: 0x006e_cc40,
            conv: Conv::Thiscall,
            ecx: B(0),
            words: &[B(1)],
            res: Res::Eax,
        },
    ];
    let mut ported = ffx_host::boot_with(&exe, ffx_host::ports::ported()).unwrap();
    let mut original = ffx_host::boot_with(&exe, Table::new()).unwrap();
    let mut checked = 0;
    for (seed, case) in (1u32..).zip(&cases) {
        assert!(
            ported.table().get(case.addr).is_some(),
            "{} not in the table",
            case.name
        );
        let (pb, pout) = buffers(&mut ported, seed);
        let (ob, oout) = buffers(&mut original, seed);
        assert_eq!(pb, ob, "the two arenas allocate alike");
        let before = ported.stats.ports.get(&case.addr).copied().unwrap_or(0);
        let a = caller(&mut ported, case, &pb, pout);
        let b = caller(&mut original, case, &ob, oout);
        assert_eq!(ported.stopped(), None, "{}", case.name);
        assert_eq!(original.stopped(), None, "{}", case.name);
        assert_eq!(
            ported.stats.ports.get(&case.addr).copied().unwrap_or(0),
            before + 1,
            "{} ran in Rust",
            case.name
        );
        match case.res {
            Res::Eax => assert_eq!(a, b, "{}: eax", case.name),
            Res::St0 => {
                let (x, y) = (
                    ported.mem().read::<u64>(pout),
                    original.mem().read::<u64>(oout),
                );
                assert_eq!(x, y, "{}: ST0", case.name);
            }
        }
        assert_eq!(
            snapshot(&ported, &pb),
            snapshot(&original, &ob),
            "{}: memory",
            case.name
        );
        checked += 1;
    }
    assert!(original.stats.ports.is_empty());
    eprintln!("{checked} ported functions match Unicorn through the table");
}

/// A fresh, empty documents folder for one test.
fn documents(name: &str) -> PathBuf {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("table-docs-{name}"));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn boot_runs_ports_from_the_table() {
    let Some(exe) = exe() else { return };
    let opts = ffx_host::Options {
        documents: Some(documents("ports")),
        gfx: ffx_host::gfx::Config {
            backend: ffx_host::gfx::Choice::Null,
            stop_after_presents: Some(3),
        },
        ..ffx_host::Options::default()
    };
    let mut exec = ffx_host::boot_opts(&exe, ffx_host::ports::table(), &opts).unwrap();
    exec.set_budget(Duration::from_secs(120));
    let stop = exec.run_entry().unwrap();
    exec.join_threads(Duration::from_secs(5));
    let syms = ffx_host::report::Symbols::read(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../symbols/ffx.sym"),
    );
    let s = ffx_host::report::summary(&exec, &syms);
    let report = ffx_host::report::text(&exec, &syms, &format!("{stop}"));
    eprintln!("{report}");
    eprintln!(
        "table: {} ports; ran in Rust: {} functions, {} calls; stop: {stop}",
        exec.table().len(),
        s.rust_functions,
        s.rust_calls
    );
    assert!(exec.table().len() > 14_000, "{report}");
    assert!(s.rust_functions > 0, "no port ran\n{report}");
    assert_eq!(stop, Stop::Milestone("Present 3".into()), "{report}");
}

/// Every arena region but the stacks, the stub pages and the thread's
/// block (its TEB holds stack addresses such as the SEH chain's head, its
/// stub page the router's x87 stubs), with the `int3`s of the ported entry
/// points put back to the original's bytes.
fn state(exec: &Exec, original: &Exec) -> Vec<(String, u32, Vec<u8>)> {
    let mut out = Vec::new();
    for r in exec.arena().regions() {
        if r.name.contains("stack") || r.name.contains("stubs") || r.name.starts_with("thread ") {
            continue;
        }
        let mut a = vec![0; r.size as usize];
        exec.mem().read_bytes(r.start, &mut a);
        for addr in exec.table().addresses() {
            if addr >= r.start && u64::from(addr) < r.end() {
                let mut b = [0u8];
                original.mem().read_bytes(addr, &mut b);
                a[(addr - r.start) as usize] = b[0];
            }
        }
        out.push((r.name.clone(), r.start, a));
    }
    out
}

#[test]
fn boot_with_the_table_ends_where_unicorn_alone_ends() {
    let Some(exe) = exe() else { return };
    // without the C runtime DLLs the boot ends at the first MSVCP110
    // import (the STL is the wall)
    let opts = |name| ffx_host::Options {
        no_dlls: true,
        documents: Some(documents(name)),
        ..ffx_host::Options::default()
    };
    let mut ported = ffx_host::boot_opts(&exe, ffx_host::ports::table(), &opts("same-a")).unwrap();
    // the all-Unicorn side keeps the clock's ports and wrappers, which the
    // full table has over the ported! entries
    let mut clock = Table::new();
    ffx_host::clock::register(&mut clock);
    let mut original = ffx_host::boot_opts(&exe, clock, &opts("same-b")).unwrap();
    // and before the first game thread starts: the threads run beside the
    // main thread, so the memory past that point differs from run to run,
    // Unicorn alone against itself included
    for e in [&mut ported, &mut original] {
        e.stop_at("KERNEL32.dll!CreateThread");
        e.set_budget(Duration::from_secs(60));
    }
    let a = ported.run_entry().unwrap();
    let b = original.run_entry().unwrap();
    ported.join_threads(Duration::from_secs(5));
    original.join_threads(Duration::from_secs(5));
    // the same stop: the same import, called from the same original
    let at = |s: &Stop| match s {
        Stop::Failed(why, _) => why.clone(),
        other => format!("{other}"),
    };
    assert_eq!(at(&a), at(&b));
    // the same imports, as often, but the C runtime's memory functions:
    // the ports fill and copy in Rust where the originals call them
    let crt = |e: &Exec| {
        let mut m = e.stats.imports.clone();
        m.retain(|k, _| {
            !["!memset", "!memcpy", "!memmove"]
                .iter()
                .any(|f| k.ends_with(f))
        });
        m
    };
    assert_eq!(crt(&ported), crt(&original), "the same imports, as often");
    let threads = |e: &Exec| {
        let l = e.shared().threads.lock().unwrap();
        l.iter().map(|t| (t.entry, t.started)).collect::<Vec<_>>()
    };
    assert_eq!(threads(&ported), threads(&original), "the same threads");
    let (sa, sb) = (state(&ported, &original), state(&original, &original));
    assert_eq!(sa.len(), sb.len());
    for ((name, start, x), (_, _, y)) in sa.iter().zip(&sb) {
        if let Some(i) = x.iter().zip(y).position(|(p, q)| p != q) {
            panic!(
                "{name}: first difference at {:#010x} (ported {:#04x}, original {:#04x})",
                start + u32::try_from(i).unwrap(),
                x[i],
                y[i]
            );
        }
    }
    eprintln!(
        "boot: {} ported functions ran in Rust ({} calls); memory, imports and the stop match the all-Unicorn run",
        ported.stats.ports.len(),
        ported.stats.ports.values().sum::<u64>()
    );
}
