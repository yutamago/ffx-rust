//! Schema sites for `tools/mutants.py`.
//!
//! Reads `{"src": text, "sites": [{"start", "end", "new"}]}` on stdin: a Rust
//! file and, per mutant, the byte range it changes and the text that replaces
//! it. Writes, per site, every syntax node that could carry the mutant as a
//! schema, smallest first: an expression (`expr`: wrapped in parentheses;
//! `bare`: a statement's own expression, wrapped as is) or a run of whole
//! statements in one block (`stmts`; extended to the block's end when a `let`
//! is involved, so later statements still see the binding). Each candidate
//! covers the changed range and comes with its mutated text (`b`), which must
//! parse as the same kind of node. Whether it type-checks is for the compiler.

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
                let kind = if bare { "bare" } else { "expr" };
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
    out.sort_by_key(|c| (c.end - c.start, c.start, c.kind == "stmts"));
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
