//! Readable assembly.
//!
//! ```text
//! worker 1 type 2
//! e0:
//!     pushc c3        ; 1234
//!     popv v0
//! l2:
//!     pushv v0
//!     popa_jz l5
//!     func 0x6012
//! ```
//!
//! - `worker N` starts worker *N*'s code; workers may appear in any order
//!   but each exactly once.
//! - `eN:` and `lN:` place entry point *N* and jump label *N* of the
//!   current worker. Every original entry and label must be placed once.
//! - Operands: `lN` label, `cN` integer constant, `fN` float constant,
//!   `vN` variable, `wN` worker, or a number (`12`, `-3`, `0x6012`). Any
//!   form is accepted for any opcode; it is stored as a `u16`.
//! - `;` starts a comment.
//!
//! Assembly applies onto the original script (see [`Script::rebuild`]):
//! constants, variables and everything outside the code stay as they were.

use std::fmt::Write;

use crate::opcode::{OPCODES, Operand, by_name};
use crate::script::{Instr, Script};
use crate::{Error, Result};

/// Disassembles every worker.
///
/// # Errors
/// Fails only on scripts [`Script::parse`] would have rejected.
pub fn disassemble(script: &Script) -> Result<String> {
    disassemble_annotated(script, &|_| None)
}

/// Disassembles every worker; `annotate` may add a comment to each native
/// function call (for example the handler's address and name).
///
/// # Errors
/// Fails only on scripts [`Script::parse`] would have rejected.
pub fn disassemble_annotated(
    script: &Script,
    annotate: &dyn Fn(u16) -> Option<String>,
) -> Result<String> {
    let mut out = String::new();
    let _ = writeln!(out, "; {} workers", script.workers().len());
    for (w, worker) in script.workers().iter().enumerate() {
        let _ = writeln!(out, "\nworker {w} type {}", worker.kind);
        for instr in script.decode(worker.range.clone())? {
            for (i, _) in worker
                .entries
                .iter()
                .enumerate()
                .filter(|(_, t)| **t == instr.offset)
            {
                let _ = writeln!(out, "e{i}:");
            }
            for (i, _) in worker
                .labels
                .iter()
                .enumerate()
                .filter(|(_, t)| **t == instr.offset)
            {
                let _ = writeln!(out, "l{i}:");
            }
            let mut line = render(script, w, &instr);
            let op = &OPCODES[usize::from(instr.opcode)];
            if let (Operand::Func, Some(id)) = (op.operand, instr.operand)
                && let Some(note) = annotate(id)
            {
                line = format!("{line:<24} ; {note}");
            }
            let _ = writeln!(out, "    {line}");
        }
    }
    Ok(out)
}

fn render(script: &Script, worker: usize, instr: &Instr) -> String {
    let op = &OPCODES[usize::from(instr.opcode)];
    let Some(v) = instr.operand else {
        return op.name.to_string();
    };
    match op.operand {
        Operand::Label => format!("{} l{v}", op.name),
        Operand::Int => match script.int_const(worker, v) {
            Some(c) => format!("{} c{v:<8} ; {c}", op.name),
            None => format!("{} c{v}", op.name),
        },
        Operand::Float => match script.float_const(worker, v) {
            Some(f) => format!("{} f{v:<8} ; {f:?}", op.name),
            None => format!("{} f{v}", op.name),
        },
        Operand::Imm => format!("{} {}", op.name, v.cast_signed()),
        Operand::Var => format!("{} v{v}", op.name),
        Operand::Worker => format!("{} w{v}", op.name),
        Operand::Func | Operand::Raw => format!("{} 0x{v:04x}", op.name),
    }
}

/// Code, entries and labels assembled for one worker.
#[derive(Default)]
struct Assembled {
    seen: bool,
    code: Vec<u8>,
    entries: Vec<Option<u32>>,
    labels: Vec<Option<u32>>,
}

fn parse_number(s: &str) -> Option<i64> {
    let (neg, s) = s.strip_prefix('-').map_or((false, s), |r| (true, r));
    let v = match s.strip_prefix("0x") {
        Some(hex) => i64::from_str_radix(hex, 16).ok()?,
        None => s.parse().ok()?,
    };
    Some(if neg { -v } else { v })
}

fn parse_operand(s: &str) -> Option<u16> {
    let v = match s.as_bytes().first()? {
        b'l' | b'c' | b'f' | b'v' | b'w' if s.len() > 1 => s[1..].parse::<i64>().ok()?,
        _ => parse_number(s)?,
    };
    match v {
        -0x8000..=-1 => Some(i16::try_from(v).ok()?.cast_unsigned()),
        0..=0xffff => u16::try_from(v).ok(),
        _ => None,
    }
}

/// Assembles text onto the original script.
///
/// # Errors
/// Fails on unknown mnemonics or operands, missing or repeated workers,
/// entries and labels, and anything [`Script::rebuild`] refuses.
pub fn assemble(original: &Script, text: &str) -> Result<Script> {
    let workers = original.workers();
    let mut out: Vec<Assembled> = workers
        .iter()
        .map(|w| Assembled {
            entries: vec![None; w.entries.len()],
            labels: vec![None; w.labels.len()],
            ..Assembled::default()
        })
        .collect();
    let mut current: Option<usize> = None;
    for (n, raw) in text.lines().enumerate() {
        let line_no = n + 1;
        let err = |m: String| Error::Syntax {
            line: line_no,
            message: m,
        };
        let line = raw.split(';').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let mut words = line.split_whitespace();
        let first = words.next().expect("non-empty");
        if first == "worker" {
            let w = words
                .next()
                .and_then(|s| s.parse::<usize>().ok())
                .filter(|&w| w < out.len())
                .ok_or_else(|| err(format!("expected a worker number below {}", out.len())))?;
            if std::mem::replace(&mut out[w].seen, true) {
                return Err(err(format!("worker {w} appears twice")));
            }
            current = Some(w);
            continue;
        }
        let w = current.ok_or_else(|| err("code before the first `worker` line".into()))?;
        let a = &mut out[w];
        let here = u32::try_from(a.code.len()).map_err(|_| err("code too large".into()))?;
        if let Some(name) = first.strip_suffix(':') {
            let (table, kind) = match name.as_bytes().first() {
                Some(b'e') => (&mut a.entries, "entry"),
                Some(b'l') => (&mut a.labels, "label"),
                _ => return Err(err(format!("unknown definition {first}"))),
            };
            let i = name[1..]
                .parse::<usize>()
                .ok()
                .filter(|&i| i < table.len())
                .ok_or_else(|| err(format!("worker {w} has no {kind} {}", &name[1..])))?;
            if table[i].replace(here).is_some() {
                return Err(err(format!("{kind} {i} placed twice")));
            }
            continue;
        }
        let opcode = by_name(first).ok_or_else(|| err(format!("unknown instruction {first}")))?;
        let operand = match words.next() {
            Some(s) => Some(parse_operand(s).ok_or_else(|| err(format!("bad operand {s}")))?),
            None => None,
        };
        if let Some(extra) = words.next() {
            return Err(err(format!("unexpected {extra}")));
        }
        Instr {
            offset: here,
            opcode,
            operand,
        }
        .encode(&mut a.code);
    }

    let mut code = Vec::new();
    let mut entries = Vec::new();
    let mut labels = Vec::new();
    for (w, a) in out.into_iter().enumerate() {
        if !a.seen {
            return Err(Error::Shape(format!("worker {w} is missing")));
        }
        let unplaced = |kind: &str, t: &[Option<u32>]| {
            t.iter()
                .position(Option::is_none)
                .map(|i| Error::Shape(format!("worker {w}: {kind} {i} is not placed")))
        };
        if let Some(e) = unplaced("entry", &a.entries).or_else(|| unplaced("label", &a.labels)) {
            return Err(e);
        }
        entries.push(a.entries.into_iter().map(Option::unwrap).collect());
        labels.push(a.labels.into_iter().map(Option::unwrap).collect());
        code.push(a.code);
    }
    original.rebuild(&code, &entries, &labels)
}
