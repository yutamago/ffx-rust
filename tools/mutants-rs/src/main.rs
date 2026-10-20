//! Schema sites for `tools/mutants.py`.
//!
//! Reads `{"src": text, "sites": [{"start", "end", "new"}]}` on stdin: a Rust
//! file and, per mutant, the byte range it changes and the text that replaces
//! it. Writes, per site, every syntax node that could carry the mutant as a
//! schema, smallest first: an expression (`expr`: wrapped in parentheses;
//! `arm`: a match arm's body, wrapped in braces so the arm still needs no
//! comma; `bare`: a statement's own expression, wrapped as is) or a run of
//! whole statements in one block (`stmts`; extended to the block's end when a
//! `let` is involved, so later statements still see the binding). Each
//! candidate covers the changed range and comes with its mutated text (`b`),
//! which must parse as the same kind of node, and the function holding it must
//! still parse with the schema in place (`schema()` below is the shape
//! mutants.py writes). Nodes in const contexts (const and static items, const
//! fns, array lengths, enum discriminants) never qualify: `mutant()` is not a
//! const fn. Whether a candidate type-checks is for the compiler.

use std::io::Read;

use serde::{Deserialize, Serialize};
use syn::spanned::Spanned;
use syn::visit::Visit;

#[derive(Deserialize)]
struct Input {
    src: String,
    sites: Vec<Site>,
}

#[derive(Deserialize)]
struct Site {
    start: usize,
    end: usize,
    new: String,
}

#[derive(Serialize, Clone, PartialEq, Eq)]
struct Candidate {
    kind: &'static str,
    start: usize,
    end: usize,
    b: String,
}

/// Every expression and block of the file, by byte range.
#[derive(Default)]
struct Nodes {
    /// (start, end, is the direct expression of a statement without `;`)
    exprs: Vec<(usize, usize, bool)>,
    blocks: Vec<Block>,
    /// Statements' own expressions, marked in `exprs` once the walk is done.
    bare: Vec<(usize, usize)>,
    /// Match arms' bodies.
    arms: Vec<(usize, usize)>,
    /// Ranges where only const expressions are allowed.
    consts: Vec<(usize, usize)>,
    /// Functions: range and how to parse one on its own.
    fns: Vec<(usize, usize, FnKind)>,
}

#[derive(Clone, Copy)]
enum FnKind {
    Item,
    Impl,
    Trait,
}

struct Block {
    /// The range between the braces.
    inner: (usize, usize),
    /// Each statement's range and whether it is a `let`.
    stmts: Vec<(usize, usize, bool)>,
}

fn range(s: &impl Spanned) -> (usize, usize) {
    let r = s.span().byte_range();
    (r.start, r.end)
}

impl<'ast> Visit<'ast> for Nodes {
    fn visit_expr(&mut self, e: &'ast syn::Expr) {
        let (s, t) = range(e);
        self.exprs.push((s, t, false));
        syn::visit::visit_expr(self, e);
    }

    fn visit_block(&mut self, b: &'ast syn::Block) {
        let open = b.brace_token.span.open().byte_range().end;
        let close = b.brace_token.span.close().byte_range().start;
        let stmts = b
            .stmts
            .iter()
            .map(|st| {
                let (s, t) = range(st);
                if let syn::Stmt::Expr(e, None) = st {
                    self.bare.push(range(e));
                }
                (s, t, matches!(st, syn::Stmt::Local(_)))
            })
            .collect();
        self.blocks.push(Block {
            inner: (open, close),
            stmts,
        });
        syn::visit::visit_block(self, b);
    }

    fn visit_arm(&mut self, a: &'ast syn::Arm) {
        self.arms.push(range(&a.body));
        syn::visit::visit_arm(self, a);
    }

    fn visit_item_fn(&mut self, f: &'ast syn::ItemFn) {
        let r = range(f);
        self.fns.push((r.0, r.1, FnKind::Item));
        if f.sig.constness.is_some() {
            self.consts.push(r);
        }
        syn::visit::visit_item_fn(self, f);
    }

    fn visit_impl_item_fn(&mut self, f: &'ast syn::ImplItemFn) {
        let r = range(f);
        self.fns.push((r.0, r.1, FnKind::Impl));
        if f.sig.constness.is_some() {
            self.consts.push(r);
        }
        syn::visit::visit_impl_item_fn(self, f);
    }

    fn visit_trait_item_fn(&mut self, f: &'ast syn::TraitItemFn) {
        let r = range(f);
        self.fns.push((r.0, r.1, FnKind::Trait));
        if f.sig.constness.is_some() {
            self.consts.push(r);
        }
        syn::visit::visit_trait_item_fn(self, f);
    }

    fn visit_item_const(&mut self, c: &'ast syn::ItemConst) {
        self.consts.push(range(c));
        syn::visit::visit_item_const(self, c);
    }

    fn visit_item_static(&mut self, c: &'ast syn::ItemStatic) {
        self.consts.push(range(c));
        syn::visit::visit_item_static(self, c);
    }

    fn visit_impl_item_const(&mut self, c: &'ast syn::ImplItemConst) {
        self.consts.push(range(c));
        syn::visit::visit_impl_item_const(self, c);
    }

    fn visit_trait_item_const(&mut self, c: &'ast syn::TraitItemConst) {
        self.consts.push(range(c));
        syn::visit::visit_trait_item_const(self, c);
    }

    fn visit_type_array(&mut self, a: &'ast syn::TypeArray) {
        self.consts.push(range(&a.len));
        syn::visit::visit_type_array(self, a);
    }

    fn visit_expr_repeat(&mut self, a: &'ast syn::ExprRepeat) {
        self.consts.push(range(&a.len));
        syn::visit::visit_expr_repeat(self, a);
    }

    fn visit_variant(&mut self, v: &'ast syn::Variant) {
        if let Some((_, d)) = &v.discriminant {
            self.consts.push(range(d));
        }
        syn::visit::visit_variant(self, v);
    }

    fn visit_generic_argument(&mut self, g: &'ast syn::GenericArgument) {
        if let syn::GenericArgument::Const(c) = g {
            self.consts.push(range(c));
        }
        syn::visit::visit_generic_argument(self, g);
    }
}

impl Nodes {
    fn mark_bare(&mut self) {
        let bare = std::mem::take(&mut self.bare);
        for e in &mut self.exprs {
            if bare.contains(&(e.0, e.1)) {
                e.2 = true;
            }
        }
    }

    fn in_const(&self, s: usize, t: usize) -> bool {
        self.consts.iter().any(|&(a, b)| a <= s && t <= b)
    }

    /// Whether the function holding `[s, t)` (or the file) parses with the
    /// schema of `c` in place.
    fn schema_parses(&self, src: &str, cand: &Candidate) -> bool {
        let (start, end) = (cand.start, cand.end);
        let wrapped = schema(cand, &src[start..end]);
        let holder = self
            .fns
            .iter()
            .filter(|f| f.0 <= start && end <= f.1)
            .min_by_key(|f| f.1 - f.0);
        let Some(&(fn_start, fn_end, kind)) = holder else {
            return syn::parse_file(&format!("{}{wrapped}{}", &src[..start], &src[end..])).is_ok();
        };
        let text = format!("{}{wrapped}{}", &src[fn_start..start], &src[end..fn_end]);
        match kind {
            FnKind::Item => syn::parse_str::<syn::Item>(&text).is_ok(),
            FnKind::Impl => syn::parse_str::<syn::ImplItem>(&text).is_ok(),
            FnKind::Trait => syn::parse_str::<syn::TraitItem>(&text).is_ok(),
        }
    }
}

/// The schema text mutants.py writes for one mutant at `c` (`a` is the original).
fn schema(c: &Candidate, a: &str) -> String {
    let (open, close) = match c.kind {
        "expr" => ("(", ")"),
        "arm" => ("{", "}"),
        _ => ("", ""),
    };
    format!(
        "{open}if ::ffx_core::mutant(0) {{\n{}\n}} else {{\n{a}\n}}{close}",
        c.b
    )
}

/// The mutated text of `[s, t)`: the original range with the site's change.
fn mutated(src: &str, site: &Site, s: usize, t: usize) -> String {
    format!("{}{}{}", &src[s..site.start], site.new, &src[site.end..t])
}

fn block_has_let(text: &str) -> Option<bool> {
    let b: syn::Block = syn::parse_str(&format!("{{\n{text}\n}}")).ok()?;
    Some(b.stmts.iter().any(|s| matches!(s, syn::Stmt::Local(_))))
}

fn candidates(src: &str, nodes: &Nodes, site: &Site) -> Vec<Candidate> {
    let (rs, re) = (site.start, site.end);
    let mut out = Vec::new();
    for &(s, t, bare) in &nodes.exprs {
        if s <= rs && re <= t {
            let b = mutated(src, site, s, t);
            if syn::parse_str::<syn::Expr>(&b).is_ok() {
                let kind = if bare {
                    "bare"
                } else if nodes.arms.contains(&(s, t)) {
                    "arm"
                } else {
                    "expr"
                };
                out.push(Candidate {
                    kind,
                    start: s,
                    end: t,
                    b,
                });
            }
        }
    }
    for block in &nodes.blocks {
        let (open, close) = block.inner;
        if !(open <= rs && re <= close) {
            continue;
        }
        let hit: Vec<_> = block
            .stmts
            .iter()
            .filter(|&&(s, t, _)| s < re && t > rs)
            .collect();
        let s = hit.first().map_or(rs, |h| h.0.min(rs));
        let mut t = hit.last().map_or(re, |h| h.1.max(re));
        let lets = hit.iter().any(|h| h.2);
        let Some(b_lets) = block_has_let(&mutated(src, site, s, t)) else {
            continue;
        };
        if lets || b_lets {
            // keep the bindings' scope: the schema runs to the block's end
            t = block.stmts.last().map_or(t, |l| l.1.max(t));
        }
        let b = mutated(src, site, s, t);
        if block_has_let(&b).is_some() {
            out.push(Candidate {
                kind: "stmts",
                start: s,
                end: t,
                b,
            });
        }
    }
    out.retain(|c| !nodes.in_const(c.start, c.end));
    out.sort_by_key(|c| (c.end - c.start, c.start, c.kind == "stmts"));
    out.retain(|c| nodes.schema_parses(src, c));
    out.dedup_by(|a, b| a.start == b.start && a.end == b.end);
    out
}

fn main() {
    let mut text = String::new();
    std::io::stdin()
        .read_to_string(&mut text)
        .expect("read stdin");
    let input: Input = serde_json::from_str(&text).expect("input JSON");
    let file = match syn::parse_file(&input.src) {
        Ok(f) => f,
        Err(e) => {
            // no schemata for an unparsable file: every site falls back
            eprintln!("parse error: {e}");
            let none: Vec<Vec<Candidate>> = input.sites.iter().map(|_| Vec::new()).collect();
            println!("{}", serde_json::to_string(&none).expect("JSON"));
            return;
        }
    };
    let mut nodes = Nodes::default();
    nodes.visit_file(&file);
    nodes.mark_bare();
    let out: Vec<Vec<Candidate>> = input
        .sites
        .iter()
        .map(|site| candidates(&input.src, &nodes, site))
        .collect();
    println!("{}", serde_json::to_string(&out).expect("JSON"));
}
