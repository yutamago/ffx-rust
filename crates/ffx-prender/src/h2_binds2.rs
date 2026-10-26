//! The six large `Bind()`s of h2-render-b (continuation round): the
//! class registrations of `PEffect`, `PMeshInstance`,
//! `PCameraControllerComponent`, `PSpriteAnimationInfo`,
//! `PSpriteAnimationInfoChar` and `PSpriteCollection`.
//!
//! They have the frame of h1-render-a's multi-member `Bind`s
//! ([`crate::h1_bind`]): `RegisterType(d)`, guarded member builds
//! (data members, class members linked into their owner's list through
//! `findMember`, inline array descriptors), unconditional code-address
//! stores into the members' sub-objects, dropped type-getter calls
//! (`0x00528610`, `0x00490b60`, `0x004ebc70` with a global's word) and
//! `addToNamespace(d)` last. Only the guard tests differ in form (bit 31
//! is tested with `test eax, eax; js`), which the steps already express,
//! so these copies run on [`crate::h1_bind::bridge::bind`] with their own
//! table ([`BINDS`], read from the machine code by
//! `~/ffx-work/m4/port/h2c/gen.py`; the oracle tests compare each copy
//! with its original).

use crate::h1_bind::{Bind, Step, Ty, Val};

/// The copies: `(address, constants)`.
pub const BINDS: &[(u32, Bind)] = &include!("h2_binds2_table.rs.in");

/// The constants of copy `f` (compile time).
#[must_use]
pub const fn bind_of(f: u32) -> Bind {
    let mut i = 0;
    while i < BINDS.len() {
        if BINDS[i].0 == f {
            return BINDS[i].1;
        }
        i += 1;
    }
    panic!("not in the table");
}
