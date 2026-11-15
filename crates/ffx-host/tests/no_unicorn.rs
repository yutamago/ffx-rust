//! `ffx-host` without the emulator (docs/host.md, "Without Unicorn"): the
//! release's executor, built with `--no-default-features`. The function
//! table, the imports' Rust sides and the traps run as with Unicorn; a
//! call to any other address is a hard stop that names the address, its
//! `ffx.sym` name and the callers. The boot runs as far as the Rust side
//! reaches and reports the first unported address. Needs `FFX_GAME_DIR`;
//! skips without it.
//!
//! `FFX_HOST_UNPORTED_OUT=<file>` writes what the boot found as TSV: the
//! unported addresses it stopped at, and every C runtime import without a
//! Rust side (`kind`, `address or import`, `name`, `detail`).
//! `FFX_NOU_PRESENTS=<n>` (default 3) and `FFX_NOU_BUDGET=<seconds>`
//! (default 120) let the boot run further.

#![cfg(not(feature = "dev-unicorn"))]

use std::fmt::Write as _;
use std::path::PathBuf;
use std::time::Duration;

use ffx_host::exec::{Exec, Route, Stop, Trap};
use ffx_host::table::{Args, Conv, Ret};

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

/// `MsCheckRange`, a port (cdecl `(x, lo, hi)`: `x` clamped).
const MS_CHECK_RANGE: u32 = 0x0079_a0c0;
/// An address with no Rust side: `lua_getfield` (`0x0094b030`, the
/// statically linked Lua VM, which nothing ports yet; the no-Unicorn boot
/// no longer stops anywhere after m5-wp5e, and the entry point, `Game_Main`,
/// `GDraw`'s trampoline `0x00a34b00`, the symbol map's `operator[]`
/// `0x006f3ae0` and the array-start callback `0x0048c260` that this test
/// used are ported now).
const ENTRY: u32 = 0x0094_b030;

#[test]
fn a_port_called_from_rust_runs_without_the_emulator() {
    let Some(exe) = exe() else { return };
    let mut exec = ffx_host::boot(&exe).unwrap();
    // no C runtime DLL: their code can't run
    assert!(exec.shared().dlls.is_empty());
    for (x, lo, hi, want) in [(9, 1, 5, 5), (0, 1, 5, 1), (3, 1, 5, 3), (8, 8, 8, 8)] {
        assert_eq!(exec.call_cdecl(MS_CHECK_RANGE, &[x, lo, hi]).unwrap(), want);
    }
    assert_eq!(exec.stats.ports.get(&MS_CHECK_RANGE), Some(&4));
    assert!(exec.stopped().is_none());
}

/// A trap standing for a port that calls an address with no Rust side.
fn calls_the_entry(e: &mut Exec, _: u32, _: &Args) -> Ret {
    let r = e.call_stdcall(ENTRY, &[]);
    assert!(r.is_err(), "an unported call must not return");
    Ret::eax(7)
}

#[test]
fn an_unported_address_is_a_hard_stop_that_names_it_and_its_callers() {
    let Some(exe) = exe() else { return };
    let mut exec = ffx_host::boot(&exe).unwrap();
    let at = exec.alloc_code(&[0xcc]).unwrap();
    exec.shared().add_trap(
        at,
        Trap {
            run: calls_the_entry,
            tag: 0,
            conv: Conv::Cdecl,
            words: 0,
        },
    );
    // the trap runs; its call stops the thread, so the caller sees a stop
    assert!(exec.call_cdecl(at, &[]).is_err());
    let Some(Stop::Failed(why, addr)) = exec.stopped().cloned() else {
        panic!("no stop: {:?}", exec.stopped());
    };
    assert_eq!(addr, ENTRY, "{why}");
    assert!(
        why.starts_with(&format!("unported address {ENTRY:#010x} (")),
        "{why}"
    );
    assert!(
        why.contains(&format!("called from trap {at:#010x}")),
        "{why}"
    );
    assert_eq!(exec.stats.unported.len(), 1);
    assert_eq!(exec.stats.unported[0].0, ENTRY);
    // every later call unwinds at once
    assert!(exec.call_cdecl(MS_CHECK_RANGE, &[1, 0, 2]).is_err());
}

#[test]
fn a_thread_pool_work_item_runs_its_callback_without_a_trampoline() {
    let Some(exe) = exe() else { return };
    let mut exec = ffx_host::boot(&exe).unwrap();
    exec.set_budget(Duration::from_secs(30));
    // the callback: a port (MsCheckRange(0, context, work) is context
    // clamped to 0..work), here only to see that it ran with the words
    let work = 0x0000_0100;
    let context = 0x40;
    exec.shared()
        .pool_work
        .lock()
        .unwrap()
        .insert(work, (MS_CHECK_RANGE, context));
    exec.submit_work(work);
    exec.join_threads(Duration::from_secs(10));
    let threads = exec.shared().threads.lock().unwrap().clone();
    assert_eq!(threads.len(), 1);
    let t = &threads[0];
    assert_eq!(t.outcome.as_deref(), Some("returned 0x0"), "{t:?}");
    let s = t.stats.as_ref().unwrap();
    assert_eq!(s.ports.get(&MS_CHECK_RANGE), Some(&1), "{t:?}");
}

/// The events [`a_threads_failure_ends_the_main_threads_wait_on_it`] uses:
/// the thread signals the first, the main thread the second.
const EVENT: u32 = 0x7e00;
const GO: u32 = 0x7e04;

/// A game thread's routine: signals [`EVENT`] once, waits for [`GO`], then
/// calls an address with no Rust side, which stops the thread.
fn signals_then_fails(e: &mut Exec, _: u32, _: &Args) -> Ret {
    let id = e.id;
    e.shared().sync.set_event(EVENT, true, id);
    e.shared().sync.wait(id, &[GO], false, 10_000);
    let _ = e.call_stdcall(ENTRY, &[]);
    Ret::eax(0)
}

#[test]
fn a_threads_failure_ends_the_main_threads_wait_on_it() {
    let Some(exe) = exe() else { return };
    let mut exec = ffx_host::boot(&exe).unwrap();
    exec.set_budget(Duration::from_secs(60));
    for h in [EVENT, GO] {
        exec.shared().sync.insert(
            h,
            ffx_host::sync::Obj::Event {
                manual: false,
                set: false,
            },
        );
    }
    let at = exec.alloc_code(&[0xcc]).unwrap();
    exec.shared().add_trap(
        at,
        Trap {
            run: signals_then_fails,
            tag: 0,
            conv: Conv::Stdcall,
            words: 1,
        },
    );
    let (_, id) = exec.spawn_thread(at, 0, false, "CreateThread");
    let main = ffx_host::exec::MAIN_THREAD;
    let sync = &exec.shared().sync;
    // the first signal arrives; the second never will: the thread fails
    assert_eq!(sync.wait(main, &[EVENT], false, 10_000), 0);
    sync.set_event(GO, true, main);
    assert_eq!(
        sync.wait(main, &[EVENT], false, ffx_host::sync::INFINITE),
        ffx_host::sync::WAIT_FAILED
    );
    let dead = sync.take_dead().expect("the stop for the dead wait");
    assert_eq!((dead.thread, dead.eip), (id, ENTRY));
    assert!(
        dead.why
            .starts_with(&format!("unported address {ENTRY:#010x} (")),
        "{}",
        dead.why
    );
    assert!(
        dead.why.contains(&format!(
            "on thread {id:#x}; the main thread waits for event {EVENT:#x}, which that thread \
             signals"
        )),
        "{}",
        dead.why
    );
    exec.join_threads(Duration::from_secs(10));
}

/// A game thread's routine that stops at once at an unported address.
fn fails(e: &mut Exec, _: u32, _: &Args) -> Ret {
    let _ = e.call_stdcall(ENTRY, &[]);
    Ret::eax(0)
}

/// The watchdog's hang rule (docs/host.md, "Hangs"): a main thread that
/// enters no frame boundary for 30 s after a game thread failed (here it
/// only polls) ends the run, with the failure as the outcome. Takes 30 s.
#[test]
fn a_main_thread_that_makes_no_progress_after_a_failure_ends_the_run() {
    let Some(exe) = exe() else { return };
    let mut exec = ffx_host::boot(&exe).unwrap();
    exec.set_budget(Duration::from_secs(120));
    let at = exec.alloc_code(&[0xcc]).unwrap();
    exec.shared().add_trap(
        at,
        Trap {
            run: fails,
            tag: 0,
            conv: Conv::Stdcall,
            words: 1,
        },
    );
    let (_, id) = exec.spawn_thread(at, 0, false, "CreateThread");
    let start = std::time::Instant::now();
    while exec.shared().stopped().is_none() && start.elapsed() < Duration::from_secs(60) {
        std::thread::sleep(Duration::from_millis(100));
    }
    let waited = start.elapsed();
    let Some(Stop::Failed(why, addr)) = exec.shared().stopped() else {
        panic!("no stop: {:?}", exec.shared().stopped());
    };
    assert!(waited >= Duration::from_secs(29), "{waited:?}");
    assert_eq!(addr, ENTRY, "{why}");
    assert!(
        why.starts_with(&format!("unported address {ENTRY:#010x} (")),
        "{why}"
    );
    assert!(
        why.contains(&format!(
            "on thread {id:#x}; the main thread entered no frame boundary for 30 s"
        )),
        "{why}"
    );
    exec.join_threads(Duration::from_secs(10));
}

/// One TSV row.
fn row(out: &mut String, kind: &str, what: &str, name: &str, detail: &str) {
    let _ = writeln!(out, "{kind}\t{what}\t{name}\t{detail}");
}

/// A number from the environment variable `name`, else `default`.
fn env_or(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// The speed line (m5-perf1): updates and flips per host second.
fn speed(exec: &Exec, wall: Duration) {
    let updates = exec.shared().clock.frame();
    let flips = exec.shared().clock.flips();
    let s = wall.as_secs_f64();
    eprintln!(
        "speed: {updates} updates, {flips} flips in {s:.1} s: {:.2} updates/s, {:.2} flips/s",
        f64::from(updates) / s,
        f64::from(flips) / s
    );
}

#[test]
fn boots_until_the_first_unported_address() {
    let Some(exe) = exe() else { return };
    let presents = u32::try_from(env_or("FFX_NOU_PRESENTS", 3)).unwrap_or(3);
    let opts = ffx_host::Options {
        documents: Some(documents("no-unicorn")),
        gfx: ffx_host::gfx::Config {
            backend: ffx_host::gfx::Choice::Null,
            stop_after_presents: Some(presents),
            // as ffx-rust.exe without --gfx-out (the speed line measures
            // the release's work); FFX_NOU_RECORD=1 keeps the recording
            no_record: !std::env::var("FFX_NOU_RECORD").is_ok_and(|v| v == "1"),
            ..ffx_host::gfx::Config::default()
        },
        ..ffx_host::Options::default()
    };
    let mut exec = ffx_host::boot_opts(&exe, ffx_host::ports::table(), &opts).unwrap();
    exec.set_budget(Duration::from_secs(env_or("FFX_NOU_BUDGET", 120)));
    let sampler = ffx_host::sample::start(exec.shared());
    let started = std::time::Instant::now();
    let stop = exec.run_entry().unwrap();
    let wall = started.elapsed();
    exec.join_threads(Duration::from_secs(5));
    if let Some(s) = sampler {
        s.finish(exec.shared());
    }
    speed(&exec, wall);
    let syms = syms();
    let report = ffx_host::report::text(&exec, &syms, &format!("{stop}"));
    eprintln!("{report}");
    assert!(exec.shared().dlls.is_empty());
    let mut all = exec.stats.unported.clone();
    for t in exec.shared().threads.lock().unwrap().iter() {
        for u in t.stats.iter().flat_map(|s| s.unported.iter()) {
            if !all.iter().any(|a| a.0 == u.0) {
                all.push(u.clone());
            }
        }
    }
    // the run ends at an unported address, at a C runtime import without
    // a Rust side (the boot got past the code: wp5b), once the table
    // reaches that far at the presents' milestone, or at the budget (when
    // another thread stopped at an unported address, the main thread
    // waits for the loader's work then)
    let first = match &stop {
        Stop::Failed(why, _) if why.starts_with("unstubbed import ") => {
            eprintln!("the boot's first missing import: {why}");
            None
        }
        Stop::Failed(why, addr) => {
            assert!(why.starts_with("unported address "), "{why}");
            assert_eq!(exec.stats.unported.first().map(|u| u.0), Some(*addr));
            Some(*addr)
        }
        Stop::Milestone(m) => {
            assert_eq!(m, &format!("Present {presents}"), "{report}");
            None
        }
        // (a long run, FFX_NOU_BUDGET, may also just use its budget up
        // with no stop at all: m5-wp5e's boot plays the field's updates)
        Stop::Budget(_) => all.first().map(|u| u.0),
        other => panic!("unexpected stop {other:?}\n{report}"),
    };
    let mut out = String::from("# kind\taddress or import\tname\tdetail\n");
    for (a, what) in &all {
        let kind = if Some(*a) == first {
            "boot-first"
        } else {
            "boot-thread"
        };
        // " (name) called from chain"; the name by the repository's
        // ffx.sym (the executor's is found from the working directory)
        let chain = what
            .split_once(" called from ")
            .map_or(what.as_str(), |w| w.1);
        row(
            &mut out,
            kind,
            &format!("{a:#010x}"),
            &syms.describe(*a),
            chain,
        );
    }
    // the C runtime imports without a Rust side: without the DLLs every
    // one stops the run when called
    let missing: Vec<&String> = exec
        .shared()
        .routes
        .iter()
        .filter(|(n, r)| {
            *r == Route::Missing && (n.starts_with("MSVCR110") || n.starts_with("MSVCP110"))
        })
        .map(|(n, _)| n)
        .collect();
    let called = &exec.stats.missing_imports;
    for n in &missing {
        let detail = if called.iter().any(|c| c.starts_with(n.as_str())) {
            "called in this boot"
        } else {
            "imported by FFX.exe"
        };
        row(&mut out, "crt-import", n, "", detail);
    }
    eprintln!(
        "first unported: {first:x?}; {} C runtime imports without a Rust side",
        missing.len()
    );
    if let Some(path) = std::env::var_os("FFX_HOST_UNPORTED_OUT") {
        std::fs::write(path, &out).unwrap();
    }
    // every C runtime import FFX.exe has is either Rust or missing: no DLL
    assert!(
        !exec.shared().routes.iter().any(|(_, r)| *r == Route::Dll),
        "{report}"
    );
}

/// The real clock (`Options::real_clock`, m5-perf2) paces the game as
/// `FFX.exe` does: `Game_Main`'s frame limiter sleeps until the next
/// frame's time, so the boot plays 30 updates a wall second (two vsyncs a
/// frame at 59.94 Hz: 29.97), not as many as the host can run (the
/// virtual clock's). Measured over updates 100..600 of the boot (the
/// logos and the title) in windows of 50, on the null device; the host
/// must be able to run faster than 30 a second there (headless it runs
/// 100 or more).
#[test]
fn the_real_clock_paces_the_updates_as_the_original() {
    let Some(exe) = exe() else { return };
    let opts = ffx_host::Options {
        documents: Some(documents("real-clock")),
        gfx: ffx_host::gfx::Config {
            backend: ffx_host::gfx::Choice::Null,
            no_record: true,
            ..ffx_host::gfx::Config::default()
        },
        real_clock: true,
        ..ffx_host::Options::default()
    };
    let mut exec = ffx_host::boot_opts(&exe, ffx_host::ports::table(), &opts).unwrap();
    exec.set_budget(Duration::from_secs(env_or("FFX_NOU_BUDGET", 120)));
    let shared = std::sync::Arc::clone(exec.shared());
    let watch = std::thread::spawn(move || {
        let at = |n: u32| {
            while shared.clock.frame() < n {
                if shared.stopped().is_some() {
                    return None;
                }
                std::thread::sleep(Duration::from_micros(200));
            }
            Some(std::time::Instant::now())
        };
        let mut last = at(100)?;
        let mut rates = Vec::new();
        for n in (150..=600).step_by(50) {
            let t = at(n)?;
            rates.push(50.0 / t.duration_since(last).as_secs_f64());
            last = t;
        }
        // the run ends here
        shared.stop_all(Stop::Milestone("updates measured".into()));
        Some(rates)
    });
    let stop = exec.run_entry().unwrap();
    exec.join_threads(Duration::from_secs(5));
    let mut rates = watch
        .join()
        .unwrap()
        .unwrap_or_else(|| panic!("stopped first: {stop}"));
    eprintln!("real clock: updates/s per 50 updates from 100: {rates:.2?}");
    rates.sort_by(f64::total_cmp);
    // a window with a load in it runs longer (as in the original): the
    // median window is the limiter's rate
    let median = rates[rates.len() / 2];
    assert!(
        (29.7..=30.2).contains(&median),
        "median {median:.2} updates/s"
    );
    // never faster than the limiter (the virtual clock ran the field at
    // 34 a second on Windows), but for its catch-up: after a late frame
    // the limiter's last time steps by at most one interval
    // (`now - min(over, interval)`), so the window after a slow one may
    // run one update ahead (50 in 49 frame times: 30.6)
    assert!(rates.iter().all(|&r| r < 30.7), "{rates:.2?}");
}
