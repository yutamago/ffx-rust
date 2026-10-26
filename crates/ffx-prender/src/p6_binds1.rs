//! `Bind()` functions of list p6-render-c (continuation round) with class
//! members or inline array descriptors: their [`Bind`] tables for
//! [`crate::h1_bind::bridge::bind`], generated from each copy's machine
//! code by h1's `gen.py` (`~/ffx-work/m4/port/p6c/h1gen/gen.py`); the
//! oracle tests compare every copy with its original.

#[allow(clippy::wildcard_imports)]
use crate::h1_bind::*;

/// The copies: `(address, constants)`.
pub const BINDS: &[(u32, Bind)] = &include!("p6_binds1_table.rs.in");

/// The [`BINDS`] constants of the copy at `f` (a compile error when there
/// is none: the shims look theirs up in a `const` block).
#[must_use]
pub const fn bind_of(f: u32) -> Bind {
    let mut i = 0;
    while i < BINDS.len() {
        if BINDS[i].0 == f {
            return BINDS[i].1;
        }
        i += 1;
    }
    panic!("no such Bind copy")
}
