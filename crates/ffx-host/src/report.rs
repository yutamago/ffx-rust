//! What a run reached: ports vs Unicorn functions (with their counts and
//! time), imports, threads, the stop.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::Path;

use crate::exec::{Exec, Route};

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
    let profile = exec.all_profile();
    let shared = exec.shared();
    let text = exec.image().text().map(|t| (t.start, t.start + t.size));
    let mut funcs: BTreeMap<(String, u32), Function> = BTreeMap::new();
    for (&block, &n) in &profile.blocks {
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

/// Every Unicorn function as tab-separated lines (module, start, name,
/// calls, blocks, samples), for `--profile FILE`.
#[must_use]
pub fn profile_tsv(s: &Summary) -> String {
    let mut out = String::from("module\tstart\tname\tcalls\tblocks\tsamples_ms\n");
    for f in &s.functions {
        let _ = writeln!(
            out,
            "{}\t{:#010x}\t{}\t{}\t{}\t{}",
            f.module, f.start, f.name, f.calls, f.blocks, f.samples
        );
    }
    out
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
        "executor (all threads): {} emulator starts, {} Rust->Unicorn calls, {:.3} s emulated{rate}",
        st.starts,
        st.unicorn_calls,
        st.emulated.as_secs_f64()
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
