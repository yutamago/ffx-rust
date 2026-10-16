//! Trivial `FFX.exe` functions ported automatically (`tools/autoport.py`,
//! `docs/porting.md`, "Trivial functions").
//!
//! The generator evaluates each unported game function symbolically over a
//! small, exact set of integer instructions; a function that is straight-line
//! code made of those alone becomes a [`Row`]: its loads and stores in
//! program order and what it leaves in `eax`. [`run`] and [`call`] replay a
//! row over game memory ([`Mem`]); `crates/ffx-hook/src/ported/auto.rs`
//! installs them and `crates/ffx-oracle/tests/auto.rs` checks every row
//! against the original machine code. Nothing here is written by hand but
//! the interpreter: [`table`] is generated.

use ffx_bridge::Mem;

pub mod table;

pub use table::ROWS;

/// How a function takes its arguments ([`Row::args`] lists them in that
/// order: `this` first for thiscall, `ecx` and `edx` first for fastcall).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Conv {
    /// Stack arguments, the caller pops.
    Cdecl,
    /// Stack arguments, the callee pops.
    Stdcall,
    /// `this` in `ecx`, the rest on the stack, the callee pops.
    Thiscall,
    /// The first two in `ecx` and `edx`, the rest on the stack, the callee pops.
    Fastcall,
}

/// What an argument or a loaded value is to the function, for the parity
/// test's inputs (the interpreter does not use it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Any number.
    Value,
    /// An address the function reads or writes through.
    Pointer,
    /// A small number scaled into an address.
    Index,
}

/// Binary operations on 32-bit values (comparisons give 0 or 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bin {
    Add,
    Sub,
    Mul,
    And,
    Or,
    Xor,
    /// Shift counts are taken modulo 32, as x86 does.
    Shl,
    Shr,
    Sar,
    Eq,
    Ne,
    Ltu,
    Leu,
    Gtu,
    Geu,
    Lt,
    Le,
    Gt,
    Ge,
}

/// Unary operations on 32-bit values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Un {
    Not,
    Neg,
    /// The low byte, sign-extended.
    Sx8,
    /// The low half, sign-extended.
    Sx16,
}

/// A 32-bit value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Val {
    Const(u32),
    /// The function's argument (in [`Row::args`] order).
    Arg(u8),
    /// An address in the image (`symbols/ffx.sym` base, rebased when used).
    Image(u32),
    /// A value loaded by an earlier [`Step::Load`] (zero-extended).
    Tmp(u8),
    Bin(Bin, &'static Val, &'static Val),
    Un(Un, &'static Val),
}

/// One memory access, in the original's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// `tmp = size bytes at addr`, zero-extended.
    Load {
        tmp: u8,
        size: u8,
        addr: Val,
        kind: Kind,
    },
    /// The low `size` bytes of `value` stored at `addr`.
    Store { size: u8, addr: Val, value: Val },
}

/// One ported function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row {
    /// Its address (`symbols/ffx.sym` base).
    pub address: u32,
    pub conv: Conv,
    /// Its arguments, in calling-convention order.
    pub args: &'static [Kind],
    pub steps: &'static [Step],
    /// What it leaves in `eax`, all 32 bits; `None` when it leaves `eax`
    /// alone.
    pub result: Option<Val>,
}

/// The most loads a row has.
pub const MAX_TMPS: usize = 64;

/// A value of a row, given its arguments and the loads so far.
#[must_use]
pub fn eval(mem: &impl Mem, v: &Val, args: &[u32], tmps: &[u32]) -> u32 {
    match *v {
        Val::Const(c) => c,
        Val::Arg(i) => args.get(usize::from(i)).copied().unwrap_or(0),
        Val::Image(a) => mem.rebase(a),
        Val::Tmp(t) => tmps.get(usize::from(t)).copied().unwrap_or(0),
        Val::Un(op, a) => {
            let a = eval(mem, a, args, tmps);
            #[allow(clippy::cast_possible_truncation)] // the low byte or half
            match op {
                Un::Not => !a,
                Un::Neg => a.wrapping_neg(),
                Un::Sx8 => i32::from((a as u8).cast_signed()).cast_unsigned(),
                Un::Sx16 => i32::from((a as u16).cast_signed()).cast_unsigned(),
            }
        }
        Val::Bin(op, a, b) => {
            let (a, b) = (eval(mem, a, args, tmps), eval(mem, b, args, tmps));
            let (sa, sb) = (a.cast_signed(), b.cast_signed());
            match op {
                Bin::Add => a.wrapping_add(b),
                Bin::Sub => a.wrapping_sub(b),
                Bin::Mul => a.wrapping_mul(b),
                Bin::And => a & b,
                Bin::Or => a | b,
                Bin::Xor => a ^ b,
                Bin::Shl => a << (b & 31),
                Bin::Shr => a >> (b & 31),
                Bin::Sar => (sa >> (b & 31)).cast_unsigned(),
                Bin::Eq => u32::from(a == b),
                Bin::Ne => u32::from(a != b),
                Bin::Ltu => u32::from(a < b),
                Bin::Leu => u32::from(a <= b),
                Bin::Gtu => u32::from(a > b),
                Bin::Geu => u32::from(a >= b),
                Bin::Lt => u32::from(sa < sb),
                Bin::Le => u32::from(sa <= sb),
                Bin::Gt => u32::from(sa > sb),
                Bin::Ge => u32::from(sa >= sb),
            }
        }
    }
}

/// Runs a row's loads and stores; returns the loaded values.
fn steps(mem: &mut impl Mem, row: &Row, args: &[u32]) -> [u32; MAX_TMPS] {
    let mut tmps = [0u32; MAX_TMPS];
    for s in row.steps {
        match *s {
            Step::Load {
                tmp, size, addr, ..
            } => {
                let a = eval(mem, &addr, args, &tmps);
                let v = match size {
                    1 => u32::from(mem.read::<u8>(a)),
                    2 => u32::from(mem.read::<u16>(a)),
                    _ => mem.read::<u32>(a),
                };
                // (the generator keeps tmp below MAX_TMPS: `rows_are_well_formed`)
                if let Some(t) = tmps.get_mut(usize::from(tmp)) {
                    *t = v;
                }
            }
            Step::Store { size, addr, value } => {
                let a = eval(mem, &addr, args, &tmps);
                let v = eval(mem, &value, args, &tmps);
                #[allow(clippy::cast_possible_truncation)] // the stored width
                match size {
                    1 => mem.write(a, v as u8),
                    2 => mem.write(a, v as u16),
                    _ => mem.write(a, v),
                }
            }
        }
    }
    tmps
}

/// A row whose function leaves `eax` alone: its stores.
pub fn run(mem: &mut impl Mem, row: &Row, args: &[u32]) {
    steps(mem, row, args);
}

/// A row: its stores, then what the function leaves in `eax` (0 for a row
/// without a result, whose shim returns nothing).
pub fn call(mem: &mut impl Mem, row: &Row, args: &[u32]) -> u32 {
    let tmps = steps(mem, row, args);
    row.result.as_ref().map_or(0, |r| eval(mem, r, args, &tmps))
}

/// The row of a function, by address.
#[must_use]
pub fn find(address: u32) -> Option<&'static Row> {
    ROWS.binary_search_by_key(&address, |r| r.address)
        .ok()
        .and_then(|i| ROWS.get(i).copied())
}

#[cfg(test)]
mod tests {
    use super::{MAX_TMPS, ROWS, Step, Val};

    /// The interpreter's invariants, which the generator keeps: arguments
    /// and loads in range, each load read only after it happens, sizes of
    /// 1, 2 or 4 bytes, rows sorted by address.
    #[test]
    fn rows_are_well_formed() {
        fn check(v: &Val, args: usize, loaded: &[bool], at: u32) {
            match *v {
                Val::Arg(i) => assert!(usize::from(i) < args, "{at:#x}: argument {i}"),
                Val::Tmp(t) => assert!(loaded[usize::from(t)], "{at:#x}: t{t} before its load"),
                Val::Bin(_, a, b) => {
                    check(a, args, loaded, at);
                    check(b, args, loaded, at);
                }
                Val::Un(_, a) => check(a, args, loaded, at),
                Val::Const(_) | Val::Image(_) => {}
            }
        }
        for w in ROWS.windows(2) {
            assert!(w[0].address < w[1].address);
        }
        for row in ROWS {
            let mut loaded = [false; MAX_TMPS];
            let n = row.args.len();
            for s in row.steps {
                match s {
                    Step::Load {
                        tmp, size, addr, ..
                    } => {
                        assert!([1, 2, 4].contains(size));
                        check(addr, n, &loaded, row.address);
                        assert!(usize::from(*tmp) < MAX_TMPS);
                        loaded[usize::from(*tmp)] = true;
                    }
                    Step::Store { size, addr, value } => {
                        assert!([1, 2, 4].contains(size));
                        check(addr, n, &loaded, row.address);
                        check(value, n, &loaded, row.address);
                    }
                }
            }
            if let Some(r) = &row.result {
                check(r, n, &loaded, row.address);
            }
        }
    }
}
