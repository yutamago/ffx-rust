//! The animation classes' `Bind()` functions (cdecl `()`, `eax` from
//! `addToNamespace`): each registers its class descriptor, builds its
//! member descriptors once (guard bits, `atexit` destructors), declares
//! the `PArray<T>` classes it uses, stores its method callers and adds the
//! class to its namespace.
//!
//! No interpreter of its own: the tables are run by one of the two
//! existing ones.
//! - [`BINDS`]: [`ffx_prender::h1_bind`]'s format and
//!   [`ffx_prender::h1_bind::bridge::bind`] (inline array descriptors,
//!   `PArray` declarations called with a global's word, class members
//!   found through `PClassDescriptor::findMember`). Generated from each
//!   copy's machine code by a copy of h1's `gen.py`
//!   (`~/ffx-work/m4/port/p2c/gen.py`).
//! - [`TIME_CONTROLLER`]: `PTimeController::Bind` (`0x00521bb0`), whose
//!   one method member looks for a namesake with `0x004306b0` in the
//!   owner's function list (`+0x54`), which only
//!   [`ffx_pother_a::p3_bind`]'s format has; run by
//!   [`ffx_pother_a::p3_bind::bridge::bind`]. Read off the machine code
//!   with a copy of p3's reading aid (`~/ffx-work/m4/port/p2c/bindx.py`).
//!
//! The oracle tests (`tests/p2_anim_c.rs`) run every table against its
//! original.

#[allow(clippy::wildcard_imports)]
use ffx_prender::h1_bind::*;

use ffx_pother_a::p3_bind::{FIND_FUNCTION, Step as P3};

/// The h1-format copies: `(address, constants)`.
pub const BINDS: &[(u32, Bind)] = &include!("p2_bind_table.rs.in");

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

/// `0x0052_1bb0` `Phyre_PAnimation_PTimeController_Bind`: its method
/// `GetGlobalTime` and the method's caller words.
pub const TIME_CONTROLLER: &[P3] = &[
    P3::Register(0x00ca_b920),
    P3::Guard(0x00ca_b9dc),
    P3::Once(0x1),
    P3::Method {
        member: 0x00ca_b9b4,
        name: 0x00b2_83b8,
        find: FIND_FUNCTION,
    },
    P3::AtExit(0x00b0_0b60),
    P3::End,
    P3::Store(0x00ca_b9cc, 0x00b1_1f00),
    P3::Store(0x00ca_b9d0, 0x0052_1ea0),
    P3::Store(0x00ca_b9c8, 0x00ca_b9cc),
    P3::AddToNamespace,
];
