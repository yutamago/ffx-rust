//! What a run reached: ports vs Unicorn functions, imports, the stop.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::Path;

use crate::exec::Exec;

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

/// The run in numbers.
#[derive(Debug, Clone, Default)]
pub struct Summary {
    /// Distinct functions entered as Rust ports.
    pub rust_functions: usize,
    /// Port entries.
    pub rust_calls: u64,
    /// Distinct functions run in Unicorn (by their basic blocks).
    pub unicorn_functions: BTreeSet<u32>,
    /// Basic blocks run in Unicorn.
    pub unicorn_blocks: usize,
}

/// Counts what `exec` ran.
#[must_use]
pub fn summary(exec: &Exec, syms: &Symbols) -> Summary {
    let unicorn_functions = exec
        .blocks()
        .iter()
        .filter_map(|&b| syms.containing(b).map(|(a, _)| a))
        .collect();
    Summary {
        rust_functions: exec.stats.ports.len(),
        rust_calls: exec.stats.ports.values().sum(),
        unicorn_functions,
        unicorn_blocks: exec.blocks().len(),
    }
}

/// The report as text.
#[must_use]
pub fn text(exec: &Exec, syms: &Symbols, outcome: &str) -> String {
    let s = summary(exec, syms);
    let st = &exec.stats;
    let (stubbed, imports) = exec.imports().coverage();
    let mut out = String::new();
    let _ = writeln!(out, "outcome: {outcome}");
    let _ = writeln!(out, "milestones: {:?}", st.milestones);
    let _ = writeln!(out, "message boxes: {:?}", st.messages);
    let _ = writeln!(out, "files tried: {:?}", exec.files.log);
    let _ = writeln!(
        out,
        "functions: {} in Rust ({} calls), {} in Unicorn ({} blocks)",
        s.rust_functions,
        s.rust_calls,
        s.unicorn_functions.len(),
        s.unicorn_blocks
    );
    let _ = writeln!(
        out,
        "executor: {} emulator starts, {} Rust->Unicorn calls, {:.3} s emulated",
        st.starts,
        st.unicorn_calls,
        st.emulated.as_secs_f64()
    );
    let _ = writeln!(out, "imports with a Rust side: {stubbed} of {imports}");
    let _ = writeln!(
        out,
        "missing imports (first called first): {:?}",
        st.missing_imports
    );
    let _ = writeln!(out, "threads asked for (not run): {}", st.threads.len());
    for (entry, param) in &st.threads {
        let _ = writeln!(out, "  {} param {param:#x}", syms.describe(*entry));
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
