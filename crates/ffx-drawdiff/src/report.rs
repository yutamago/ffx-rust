//! The readable report (`docs/drawdiff.md`, "Report").

use std::collections::BTreeSet;
use std::fmt::Write as _;

use crate::capture::Capture;
use crate::diff::{Comparison, Options, Part};
use crate::{Class, Difference, Value};

/// How much to print.
#[derive(Debug, Clone, Copy)]
pub struct Style {
    /// Field lines per pair (the rest is counted).
    pub limit: usize,
    /// Print allowed differences too (they are always counted).
    pub show_allowed: bool,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            limit: 12,
            show_allowed: false,
        }
    }
}

fn values(d: &Difference) -> String {
    let mut s = format!("{} | {}", d.va, d.vb);
    if let (Value::F(a), Value::F(b)) = (&d.va, &d.vb)
        && let Some(u) = crate::ulps(*a, *b)
    {
        let _ = write!(s, ", {u} ulp");
    }
    s
}

fn idx(side: &str, i: Option<usize>) -> String {
    i.map_or_else(String::new, |i| format!("{side}#{i:04}"))
}

/// The report of a comparison.
#[must_use]
#[allow(clippy::too_many_lines)] // the header, the frames, the lines, the total
pub fn render(
    names: (&str, &str),
    a: &Capture,
    b: &Capture,
    cmp: &Comparison,
    opts: &Options,
    style: Style,
) -> String {
    let mut out = String::new();
    let part = match opts.part {
        Part::All => "all",
        Part::Scene => "scene",
    };
    let _ = writeln!(
        out,
        "drawdiff\n  A {} ({} frames)\n  B {} ({} frames)\n  part {part}, {} tolerance rules, {} allow rules",
        names.0,
        a.frames.len(),
        names.1,
        b.frames.len(),
        opts.tolerances.len(),
        opts.allow.len(),
    );
    let mut total = 0;
    let mut allowed_total = 0;
    for (n, f) in cmp.frames.iter().enumerate() {
        let items_a = a.frames.get(n).map_or(&[][..], |x| &x.items[..]);
        let items_b = b.frames.get(n).map_or(&[][..], |x| &x.items[..]);
        let scene = if f.scene.is_empty() { "?" } else { &f.scene };
        let _ = writeln!(
            out,
            "\nframe {n} (scene {scene}): A {} items, B {} items; {} pairs, {} equal",
            f.a_items, f.b_items, f.pairs, f.equal
        );
        let mut counts = String::new();
        for c in Class::ALL {
            let k = f.diffs.iter().filter(|d| d.class == c).count();
            let _ = write!(counts, "{c} {k}, ");
        }
        let allowed = f.diffs.len() - f.unallowed();
        let _ = writeln!(
            out,
            "  {counts}allowed {allowed}, within tolerance {}",
            f.tolerated
        );
        total += f.unallowed();
        allowed_total += allowed;
        let mut shown_in_pair = 0;
        let mut hidden_in_pair = 0;
        let mut last_pair = None;
        let flush = |out: &mut String, hidden: &mut usize| {
            if *hidden > 0 {
                let _ = writeln!(out, "      ... and {hidden} more in this pair");
                *hidden = 0;
            }
        };
        for d in &f.diffs {
            if d.allowed.is_some() && !style.show_allowed {
                continue;
            }
            let tag = d
                .allowed
                .map(|r| format!("  [allowed: {}]", opts.allow[r].at))
                .unwrap_or_default();
            let label = |side_a: bool, i: Option<usize>| {
                let items = if side_a { items_a } else { items_b };
                i.and_then(|i| items.get(i))
                    .map_or_else(String::new, |it| it.label.clone())
            };
            match d.class {
                Class::Missing | Class::Extra | Class::Moved => {
                    flush(&mut out, &mut hidden_in_pair);
                    last_pair = None;
                    let place = match d.class {
                        Class::Missing => idx("A", d.a),
                        Class::Extra => idx("B", d.b),
                        _ => format!("{} -> {}", idx("A", d.a), idx("B", d.b)),
                    };
                    let side_a = d.a.is_some();
                    let _ = writeln!(
                        out,
                        "  {:<8} {place} {}{tag}",
                        d.class.name(),
                        label(side_a, d.a.or(d.b))
                    );
                }
                _ => {
                    let pair = (d.a, d.b);
                    if last_pair != Some(pair) {
                        flush(&mut out, &mut hidden_in_pair);
                        last_pair = Some(pair);
                        shown_in_pair = 0;
                        let _ = writeln!(
                            out,
                            "  changed  {} {} {}",
                            idx("A", d.a),
                            idx("B", d.b),
                            label(true, d.a)
                        );
                    }
                    if shown_in_pair < style.limit {
                        shown_in_pair += 1;
                        let _ = writeln!(
                            out,
                            "    {:<8} {}: {}{tag}",
                            d.class.name(),
                            d.field,
                            values(d)
                        );
                    } else {
                        hidden_in_pair += 1;
                    }
                }
            }
        }
        flush(&mut out, &mut hidden_in_pair);
    }
    let _ = writeln!(
        out,
        "\nresult: {total} differences ({allowed_total} more allowed)"
    );
    out
}

/// Allow-list lines that would cover the comparison's unallowed
/// differences, one per kind of difference, each still needing its cause
/// (`cause: TODO` is refused when read back).
#[must_use]
pub fn suggest_allow(cmp: &Comparison) -> String {
    let mut lines = BTreeSet::new();
    for f in &cmp.frames {
        for d in f.diffs.iter().filter(|d| d.allowed.is_none()) {
            let shaders = if d.kind == crate::ItemKind::Draw {
                format!(" vs={:016x} ps={:016x}", d.vs, d.ps)
            } else {
                String::new()
            };
            let line = match d.class {
                Class::Constant => {
                    format!("constant{shaders} var={}", d.var.as_deref().unwrap_or("*"))
                }
                Class::State | Class::Resource => {
                    format!("{}{shaders} field={}", d.class.name(), d.field)
                }
                _ => format!("{} kind={}{shaders}", d.class.name(), d.kind.name()),
            };
            lines.insert(format!("[{}]\n{line} cause: TODO", f.scene));
        }
    }
    let mut out = String::new();
    let mut scene = String::new();
    for l in lines {
        let (s, rule) = l.split_once('\n').unwrap_or(("", &l));
        if s != scene {
            let _ = writeln!(out, "{s}");
            s.clone_into(&mut scene);
        }
        let _ = writeln!(out, "{rule}");
    }
    out
}
