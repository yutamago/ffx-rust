//! What a run reached: ports vs Unicorn functions (with their counts and
//! time), imports, threads, the stop.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use crate::exec::{Exec, Profile, Route, Shared};

/// Function starts and names from `symbols/ffx.sym`.
#[derive(Debug, Default, Clone)]
pub struct Symbols {
    names: BTreeMap<u32, String>,
}

impl Symbols {
    /// Reads `ffx.sym` (missing file: no names).
    #[must_use]
    pub fn read(path: &Path) -> Self {
        let mut names = BTreeMap::new();
        if let Ok(text) = std::fs::read_to_string(path) {
            for line in text.lines() {
                let mut it = line.split_whitespace();
                let (Some(addr), Some("function"), Some(name)) = (it.next(), it.next(), it.next())
                else {
                    continue;
                };
                if let Some(a) = addr
                    .strip_prefix("0x")
                    .and_then(|h| u32::from_str_radix(h, 16).ok())
                {
                    names.insert(a, name.to_string());
                }
            }
        }
        Self { names }
    }

    /// Where `ffx.sym` is: `FFX_HOST_SYM`, else `symbols/ffx.sym` from the
    /// current directory, else `ffx.sym` beside the executable (the first
    /// that exists).
    #[must_use]
    pub fn locate() -> Option<PathBuf> {
        let mut paths: Vec<PathBuf> = Vec::new();
        if let Some(p) = std::env::var_os("FFX_HOST_SYM") {
            paths.push(p.into());
        }
        paths.push("symbols/ffx.sym".into());
        if let Some(d) = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(PathBuf::from))
        {
            paths.push(d.join("ffx.sym"));
        }
        paths.into_iter().find(|p| p.is_file())
    }

    /// The function containing `addr` (the nearest start at or below it).
    #[must_use]
    pub fn containing(&self, addr: u32) -> Option<(u32, &str)> {
        self.names
            .range(..=addr)
            .next_back()
            .map(|(a, n)| (*a, n.as_str()))
    }

    /// `name+off (addr)` for `addr`.
    #[must_use]
    pub fn describe(&self, addr: u32) -> String {
        match self.containing(addr) {
            Some((a, n)) if a == addr => format!("{n} ({addr:#010x})"),
            Some((a, n)) => format!("{n}+{:#x} ({addr:#010x})", addr - a),
            None => format!("{addr:#010x}"),
        }
    }
}

/// One function that ran in Unicorn.
#[derive(Debug, Clone, Default)]
pub struct Function {
    /// Its start (`FFX.exe`: the `ffx.sym` address; a DLL: its export).
    pub start: u32,
    /// Its name.
    pub name: String,
    /// Where: `FFX.exe` or the DLL.
    pub module: String,
    /// Entries (runs of the block at its start).
    pub calls: u64,
    /// Basic blocks run.
    pub blocks: u64,
    /// Clock samples (about a millisecond each).
    pub samples: u64,
}

/// The run in numbers.
#[derive(Debug, Clone, Default)]
pub struct Summary {
    /// Distinct functions entered as Rust ports.
    pub rust_functions: usize,
    /// Port entries.
    pub rust_calls: u64,
    /// Distinct `FFX.exe` functions run in Unicorn (by their basic blocks).
    pub unicorn_functions: BTreeSet<u32>,
    /// Distinct DLL functions run in Unicorn.
    pub dll_functions: BTreeSet<u32>,
    /// Distinct basic blocks run in Unicorn.
    pub unicorn_blocks: usize,
    /// Every Unicorn function, most samples first.
    pub functions: Vec<Function>,
}

/// Counts what `exec` (and the threads that ended) ran.
#[must_use]
pub fn summary(exec: &Exec, syms: &Symbols) -> Summary {
    let shared = exec.shared();
    let profile = shared.profile();
    let functions = functions(shared, &profile, syms);
    let mut stats = exec.stats.clone();
    for t in shared.threads.lock().map(|t| t.clone()).unwrap_or_default() {
        if let Some(s) = &t.stats {
            stats.merge(s);
        }
    }
    Summary {
        rust_functions: stats.ports.len(),
        rust_calls: stats.ports.values().sum(),
        unicorn_functions: functions
            .iter()
            .filter(|f| f.module == "FFX.exe")
            .map(|f| f.start)
            .collect(),
        dll_functions: functions
            .iter()
            .filter(|f| f.module != "FFX.exe")
            .map(|f| f.start)
            .collect(),
        unicorn_blocks: profile.blocks.len(),
        functions,
    }
}

/// Every function `profile`'s blocks ran in, most samples first:
/// `FFX.exe` by `syms` (a block before the first symbol is its own
/// function), the C runtime DLLs by their nearest export, the spell
/// overlays by their nearest entry ([`crate::library::function_at`]).
/// Blocks at a port's `int3` (a table entry, a trap, a bound overlay
/// entry) are left out: that is Rust running, not the original. So are
/// the stub page's (stubs, sentinels, trampolines: no module).
#[must_use]
pub fn functions(shared: &Shared, profile: &Profile, syms: &Symbols) -> Vec<Function> {
    let text = shared.image().text().map(|t| (t.start, t.start + t.size));
    let bound = crate::library::bound_entries();
    let mut funcs: BTreeMap<(String, u32), Function> = BTreeMap::new();
    for (&block, &n) in &profile.blocks {
        if shared.table().get(block).is_some()
            || shared.trap(block).is_some()
            || shared.is_patched_thunk(block)
            || bound.contains(&block)
        {
            continue; // a port's (or an import thunk's) int3
        }
        let samples = profile.samples.get(&block).copied().unwrap_or(0);
        let (module, start, name) = if text.is_some_and(|(lo, hi)| block >= lo && block < hi) {
            match syms.containing(block) {
                Some((a, name)) => ("FFX.exe".to_string(), a, name.to_string()),
                None => ("FFX.exe".to_string(), block, format!("{block:#010x}")),
            }
        } else if let Some(d) = shared.dlls.iter().find(|d| d.contains(block)) {
            let desc = d.describe(block);
            let name = desc.split('+').next().unwrap_or(&desc).to_string();
            let start = name
                .split_once('!')
                .and_then(|(_, n)| d.exports.get(n).copied())
                .unwrap_or(d.base);
            (d.name.clone(), start, name)
        } else if let Some((file, base, start)) = crate::library::function_at(block) {
            let name = format!("{file}+{:#x}", start - base);
            (file, start, name)
        } else {
            continue; // stubs, trampolines
        };
        let f = funcs.entry((module.clone(), start)).or_insert(Function {
            start,
            name,
            module,
            ..Function::default()
        });
        f.blocks += n;
        f.samples += samples;
        if block == start {
            f.calls += n;
        }
    }
    let mut functions: Vec<Function> = funcs.into_values().collect();
    functions.sort_by(|a, b| b.samples.cmp(&a.samples).then(b.blocks.cmp(&a.blocks)));
    functions
}

/// Every port that ran (all threads) as tab-separated lines (address,
/// name, calls), for `--ports FILE`: the list to bisect with
/// `FFX_HOST_DISABLE`/`FFX_HOST_ONLY` (`crate::ports::ported`).
#[must_use]
pub fn ports_tsv(exec: &Exec) -> String {
    let mut stats = exec.stats.clone();
    for t in exec
        .shared()
        .threads
        .lock()
        .map(|t| t.clone())
        .unwrap_or_default()
    {
        if let Some(s) = &t.stats {
            stats.merge(s);
        }
    }
    let mut out = String::from("address\tname\tcalls\n");
    for (&addr, &n) in &stats.ports {
        let name = exec.table().get(addr).map_or("?", |e| e.name);
        let _ = writeln!(out, "{addr:#010x}\t{name}\t{n}");
    }
    out
}

/// Every Unicorn function as tab-separated lines (module, start, name,
/// calls, blocks, samples), for `--profile FILE`.
#[must_use]
pub fn profile_tsv(s: &Summary) -> String {
    functions_tsv(&s.functions)
}

/// `functions` as [`profile_tsv`] writes them (a header line, then one
/// line each; `tools/unicorn-report.py` reads it).
#[must_use]
pub fn functions_tsv(functions: &[Function]) -> String {
    let mut out = String::from("module\tstart\tname\tcalls\tblocks\tsamples_ms\n");
    for f in functions {
        let _ = writeln!(
            out,
            "{}\t{:#010x}\t{}\t{}\t{}\t{}",
            f.module, f.start, f.name, f.calls, f.blocks, f.samples
        );
    }
    out
}

/// The Rust → Unicorn calls by target ([`Profile::entries`]) as
/// tab-separated lines (module, target, name, calls), most first: the
/// originals ports and imports still call, named as [`functions`] names
/// its rows, `FFX.exe`'s by the `ffx.sym` function starting there (else
/// `<containing>+0x<offset>`, code no symbol starts). Written beside the
/// profile (`entries_path`).
#[must_use]
pub fn entries_tsv(shared: &Shared, profile: &Profile, syms: &Symbols) -> String {
    let text = shared.image().text().map(|t| (t.start, t.start + t.size));
    let mut rows: Vec<(u32, u64)> = profile.entries.iter().map(|(&a, &n)| (a, n)).collect();
    rows.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let mut out = String::from("module\ttarget\tname\tcalls\n");
    for (a, n) in rows {
        let (module, name) = if text.is_some_and(|(lo, hi)| a >= lo && a < hi) {
            let name = match syms.containing(a) {
                Some((s, name)) if s == a => name.to_string(),
                Some((s, name)) => format!("{name}+{:#x}", a - s),
                None => format!("{a:#010x}"),
            };
            ("FFX.exe".to_string(), name)
        } else if let Some(d) = shared.dlls.iter().find(|d| d.contains(a)) {
            (d.name.clone(), d.describe(a))
        } else if let Some((file, base, start)) = crate::library::function_at(a) {
            (file, format!("{:#x}", start - base))
        } else {
            ("other".to_string(), format!("{a:#010x}"))
        };
        let _ = writeln!(out, "{module}\t{a:#010x}\t{name}\t{n}");
    }
    out
}

/// Where [`entries_tsv`] goes beside the profile at `profile`:
/// `x.profile.tsv` → `x.entries.tsv`, else `<profile>.entries.tsv`.
#[must_use]
pub fn entries_path(profile: &std::path::Path) -> std::path::PathBuf {
    let s = profile.to_string_lossy();
    match s.strip_suffix(".profile.tsv") {
        Some(stem) => std::path::PathBuf::from(format!("{stem}.entries.tsv")),
        None => std::path::PathBuf::from(format!("{s}.entries.tsv")),
    }
}

/// The report as text.
#[must_use]
#[allow(clippy::too_many_lines)] // one section after the other
pub fn text(exec: &Exec, syms: &Symbols, outcome: &str) -> String {
    let s = summary(exec, syms);
    let shared = exec.shared();
    let threads = shared.threads.lock().map(|t| t.clone()).unwrap_or_default();
    let mut st = exec.stats.clone();
    st.instructions = exec.instructions();
    st.x87 = exec.x87_steps();
    st.exceptions = exec.exceptions_raised();
    for t in &threads {
        if let Some(ts) = &t.stats {
            st.merge(ts);
        }
    }
    let (stubbed, imports) = exec.imports().coverage();
    let mut out = String::new();
    let _ = writeln!(out, "outcome: {outcome}");
    if !exec.trace.is_empty() {
        let _ = writeln!(out, "  at: {}", describe_trace(exec, syms, &exec.trace));
    }
    if !exec.recent.is_empty() {
        let _ = writeln!(
            out,
            "  last blocks: {}",
            describe_trace(exec, syms, &exec.recent)
        );
    }
    let _ = writeln!(out, "milestones: {:?}", st.milestones);
    let clock = &shared.clock;
    let (turns, freed) = clock.loader_turns();
    let _ = writeln!(
        out,
        "clock: {} updates, {} flips, game time {:.3} s past the start, {} loader turns ({} freed inside an update), {} FMOD turns",
        clock.frame(),
        clock.flips(),
        clock.now() - crate::clock::VIRTUAL_START,
        turns,
        freed,
        clock.fmod_turns()
    );
    if let Ok(s) = clock.script.lock()
        && let Some(s) = s.as_ref()
    {
        let _ = writeln!(
            out,
            "replay: {} commands done{}, shots {:?}",
            s.replies.len(),
            if s.done() { ", finished" } else { "" },
            s.shots
        );
    }
    if let Some((blocks, ignored)) = shared.audio.summary() {
        let _ = writeln!(
            out,
            "audio: {blocks} blocks mixed ({:.3} s), calls the engine did not follow: {ignored:?}",
            f64::from(u32::try_from(blocks).unwrap_or(u32::MAX)) / 30.0
        );
    }
    let players = shared.iggy.players();
    if !players.is_empty() {
        let _ = writeln!(
            out,
            "iggy: players (handle, movie bytes, ticks, draws): {players:x?}; images composited {}, draws without one {}; calls: {:?}",
            shared
                .iggy
                .composited
                .load(std::sync::atomic::Ordering::Relaxed),
            shared
                .iggy
                .not_composited
                .load(std::sync::atomic::Ordering::Relaxed),
            shared
                .iggy
                .calls
                .lock()
                .map(|c| c.clone())
                .unwrap_or_default()
        );
    }
    let notes = crate::log::notes();
    if !notes.is_empty() {
        let _ = writeln!(out, "notes:");
        for n in notes.iter().take(80) {
            let _ = writeln!(out, "  {n}");
        }
    }
    for line in crate::library::summary() {
        let _ = writeln!(out, "{line}");
    }
    let _ = writeln!(out, "message boxes: {:?}", st.messages);
    let _ = writeln!(out, "files tried: {:?}", exec.files().log);
    let dlls: Vec<String> = shared
        .dlls
        .iter()
        .map(|d| format!("{} at {:#010x}", d.name, d.base))
        .collect();
    let _ = writeln!(out, "DLLs in Unicorn: {dlls:?}");
    let _ = writeln!(
        out,
        "functions: {} in Rust ({} calls), {} of FFX.exe and {} of the DLLs in Unicorn ({} blocks)",
        s.rust_functions,
        s.rust_calls,
        s.unicorn_functions.len(),
        s.dll_functions.len(),
        s.unicorn_blocks
    );
    let rate = if st.instructions > 0 && !st.emulated.is_zero() {
        #[allow(clippy::cast_precision_loss)] // a rate
        let per_s = st.instructions as f64 / st.emulated.as_secs_f64() / 1e6;
        format!(", {} instructions, {per_s:.1} M/s", st.instructions)
    } else {
        String::new()
    };
    let _ = writeln!(
        out,
        "executor (all threads): {} emulator starts, {} Rust->Unicorn calls ({} more to one-instruction functions run in Rust), {:.3} s emulated{rate}, {} exact x87 instructions, {} exceptions dispatched",
        st.starts,
        st.unicorn_calls,
        st.leaf_calls,
        st.emulated.as_secs_f64(),
        st.x87,
        st.exceptions
    );
    let count = |r: Route| shared.routes.iter().filter(|(_, x)| *x == r).count();
    let _ = writeln!(
        out,
        "imports: {} to Rust, {} to the DLLs, {} missing ({stubbed} of {imports} stubs have a Rust side)",
        count(Route::Rust),
        count(Route::Dll),
        count(Route::Missing)
    );
    let _ = writeln!(
        out,
        "missing imports called (first called first): {:?}",
        st.missing_imports
    );
    let _ = writeln!(out, "threads: {}", threads.len());
    for t in &threads {
        let (em, starts) = t
            .stats
            .as_ref()
            .map_or((0.0, 0), |s| (s.emulated.as_secs_f64(), s.starts));
        let _ = writeln!(
            out,
            "  {:#x} {} param {:#x} via {} at {:.3} s: {} ({em:.3} s emulated, {starts} starts)",
            t.id,
            syms.describe(t.entry),
            t.param,
            t.via,
            t.at.as_secs_f64(),
            match (&t.outcome, t.started) {
                (Some(o), _) => o.clone(),
                (None, true) => "still running".into(),
                (None, false) => "never resumed".into(),
            }
        );
        if let Some(s) = &t.stats
            && !s.imports.is_empty()
        {
            let calls: Vec<String> = s
                .imports
                .iter()
                .map(|(n, k)| format!("{} x{k}", n.split('!').nth(1).unwrap_or(n)))
                .collect();
            let _ = writeln!(out, "    imports: {}", calls.join(", "));
        }
    }
    for t in threads.iter().filter(|t| !t.trace.is_empty()) {
        let _ = writeln!(
            out,
            "  thread {:#x} at: {}",
            t.id,
            describe_trace(exec, syms, &t.trace)
        );
    }
    let _ = writeln!(
        out,
        "unported functions by time (samples ~1 ms, calls, blocks), top 40 of {}:",
        s.functions.len()
    );
    for f in s.functions.iter().take(40) {
        let _ = writeln!(
            out,
            "  {:>6} {:>9} {:>11}  {} ({:#010x}, {})",
            f.samples, f.calls, f.blocks, f.name, f.start, f.module
        );
    }
    let _ = writeln!(out, "imports called:");
    for (name, n) in &st.imports {
        let _ = writeln!(out, "  {n:>8}  {name}");
    }
    let _ = writeln!(out, "ports entered:");
    for (addr, n) in &st.ports {
        let name = exec.table().get(*addr).map_or("?", |e| e.name);
        let _ = writeln!(out, "  {n:>8}  {name} ({addr:#010x})");
    }
    let (bad, first) = crate::log::bad_accesses();
    if bad > 0 {
        let _ = writeln!(out, "accesses outside the arena: {bad}, first {first:x?}");
    }
    out
}

/// Where each import went (`DLL!name<TAB>rust|dll|missing`), for the docs.
#[must_use]
pub fn routes(exec: &Exec) -> String {
    let mut out = String::new();
    for (name, r) in &exec.shared().routes {
        let r = match r {
            Route::Rust => "rust",
            Route::Dll => "dll",
            Route::Missing => "missing",
        };
        let _ = writeln!(out, "{name}\t{r}");
    }
    out
}

/// Return addresses as names (`FFX.exe` by `ffx.sym`, the DLLs by their
/// nearest export).
fn describe_trace(exec: &Exec, syms: &Symbols, trace: &[u32]) -> String {
    trace
        .iter()
        .map(
            |&a| match exec.shared().dlls.iter().find(|d| d.contains(a)) {
                Some(d) => d.describe(a),
                None => syms.describe(a),
            },
        )
        .collect::<Vec<_>>()
        .join(" <- ")
}
