//! Per-function totals of a call log: how often, from where, on which
//! thread, with which arguments and results.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::demangle;
use crate::fmodlog::{Call, Log};

/// Totals for one logged function.
#[derive(Debug, Clone, Default)]
pub struct FunctionTotals {
    /// `Class::method` or the C name.
    pub name: String,
    /// Parameter types (after `this`) when the name is decorated.
    pub params: Vec<String>,
    pub calls: usize,
    /// Return address -> calls.
    pub callers: BTreeMap<u32, usize>,
    /// Thread start -> calls.
    pub threads: BTreeMap<String, usize>,
    /// Per argument word (`this` first for members): value -> calls.
    pub args: Vec<BTreeMap<u32, usize>>,
    /// Result -> calls.
    pub results: BTreeMap<u32, usize>,
    /// Strings seen per argument.
    pub strings: BTreeMap<usize, BTreeMap<String, usize>>,
    pub first_frame: u32,
    pub last_frame: u32,
    /// Stack words popped (with `this`).
    pub words: BTreeMap<u32, usize>,
}

/// Totals per function index, for the functions that were called.
#[must_use]
pub fn totals(log: &Log) -> BTreeMap<u32, FunctionTotals> {
    let mut out: BTreeMap<u32, FunctionTotals> = BTreeMap::new();
    for call in &log.calls {
        let t = out.entry(call.function).or_insert_with(|| {
            let name = log.function(call).map_or("?", |f| f.name.as_str());
            FunctionTotals {
                name: demangle::short(name),
                params: demangle::member(name).map(|m| m.params).unwrap_or_default(),
                first_frame: call.frame,
                ..FunctionTotals::default()
            }
        });
        add(t, log, call);
    }
    out
}

fn add(t: &mut FunctionTotals, log: &Log, call: &Call) {
    t.calls += 1;
    *t.callers.entry(call.caller).or_default() += 1;
    let thread = log
        .threads
        .get(&call.thread)
        .cloned()
        .unwrap_or_else(|| call.thread.to_string());
    *t.threads.entry(thread).or_default() += 1;
    if t.args.len() < call.args.len() {
        t.args.resize(call.args.len(), BTreeMap::new());
    }
    for (i, a) in call.args.iter().enumerate() {
        *t.args[i].entry(*a).or_default() += 1;
    }
    *t.results.entry(call.result).or_default() += 1;
    for (i, s) in &call.strings {
        *t.strings
            .entry(*i)
            .or_default()
            .entry(s.clone())
            .or_default() += 1;
    }
    t.first_frame = t.first_frame.min(call.frame);
    t.last_frame = t.last_frame.max(call.frame);
    *t.words.entry(call.words).or_default() += 1;
}

/// A value as its parameter type reads it: floats and bools decoded.
#[must_use]
pub fn show(value: u32, ty: Option<&str>) -> String {
    match ty {
        Some("float") => format!("{}", f32::from_bits(value)),
        Some("bool") => (value & 0xff != 0).to_string(),
        Some("int") => format!("{}", value.cast_signed()),
        _ => format!("{value:#x}"),
    }
}

/// The few most common values of `counts`, `value xN`; `n distinct` when
/// there are many.
#[must_use]
pub fn common(counts: &BTreeMap<u32, usize>, ty: Option<&str>, keep: usize) -> String {
    let mut v: Vec<(u32, usize)> = counts.iter().map(|(k, n)| (*k, *n)).collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let mut s: Vec<String> = v
        .iter()
        .take(keep)
        .map(|(k, n)| format!("{} x{n}", show(*k, ty)))
        .collect();
    if v.len() > keep {
        s.push(format!("({} distinct)", v.len()));
    }
    s.join(", ")
}

/// A markdown table of the totals, one row per function, most called
/// first: calls, rate per second of the log, callers, threads, and the
/// common arguments.
#[must_use]
pub fn table(log: &Log) -> String {
    let span_s = match (log.calls.first(), log.calls.last()) {
        (Some(a), Some(b)) if b.ms > a.ms => (b.ms - a.ms) / 1000.0,
        _ => 1.0,
    };
    let mut rows: Vec<(u32, FunctionTotals)> = totals(log).into_iter().collect();
    rows.sort_by(|a, b| b.1.calls.cmp(&a.1.calls).then(a.0.cmp(&b.0)));
    let mut out = String::from(
        "| function | calls | /s | frames | callers (return address xN) | threads | arguments (common values) | results |\n|---|---|---|---|---|---|---|---|\n",
    );
    for (_, t) in rows {
        let callers: Vec<String> = t
            .callers
            .iter()
            .map(|(a, n)| format!("{a:08x} x{n}"))
            .collect();
        let threads: Vec<String> = t.threads.iter().map(|(a, n)| format!("{a} x{n}")).collect();
        let member = !t.params.is_empty() || t.name.contains("::");
        let args: Vec<String> = t
            .args
            .iter()
            .enumerate()
            .map(|(i, counts)| {
                let ty = if member {
                    i.checked_sub(1).and_then(|p| t.params.get(p))
                } else {
                    t.params.get(i)
                };
                let label = if member && i == 0 {
                    "this".to_owned()
                } else {
                    ty.cloned().unwrap_or_else(|| format!("a{i}"))
                };
                let strings = t
                    .strings
                    .get(&i)
                    .map(|s| {
                        let v: Vec<String> = s.keys().take(4).map(|k| format!("\"{k}\"")).collect();
                        format!(" {}", v.join(" "))
                    })
                    .unwrap_or_default();
                format!(
                    "{label}: {}{strings}",
                    common(counts, ty.map(String::as_str), 3)
                )
            })
            .collect();
        let _ = writeln!(
            out,
            "| `{}` | {} | {:.1} | {}-{} | {} | {} | {} | {} |",
            t.name,
            t.calls,
            f64::from(u32::try_from(t.calls).unwrap_or(u32::MAX)) / span_s,
            t.first_frame,
            t.last_frame,
            callers.join(", "),
            threads.join(", "),
            args.join("; "),
            common(&t.results, None, 3)
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sums_calls() {
        let text =
            "# fn 0 fmod_event.dll import 1000 ?setVolume@Event@FMOD@@QAG?AW4FMOD_RESULT@@M@Z
# thread 7 006fa700
1 0.0 3 7 00709e80 0 2 a0,3f800000 0
2 500.0 9 7 00709e80 0 2 a0,0 0
3 1000.0 12 7 0070b867 0 2 b0,3f800000 0
";
        let log = Log::parse(text).unwrap();
        let t = &totals(&log)[&0];
        assert_eq!(t.name, "Event::setVolume");
        assert_eq!(t.calls, 3);
        assert_eq!(t.callers[&0x0070_9e80], 2);
        assert_eq!((t.first_frame, t.last_frame), (3, 12));
        assert_eq!(common(&t.args[1], Some("float"), 3), "1 x2, 0 x1");
        let table = table(&log);
        assert!(
            table.contains("| `Event::setVolume` | 3 | 3.0 | 3-12 |"),
            "{table}"
        );
        assert!(
            table.contains("this: 0xa0 x2, 0xb0 x1; float: 1 x2, 0 x1"),
            "{table}"
        );
    }
}
