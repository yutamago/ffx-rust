//! `PhyreEngine`'s class-descriptor callbacks and class registration for
//! the render classes (M4 port round 1, list h1-render-a).
//!
//! Every serialisable class `T` has descriptor objects whose vtables
//! (`PClassDescriptorForType<T>`, `PClassDescriptorAbstract<T>`,
//! `PClassDescriptorConcrete<T>` or `...WithoutDefaultConstructor<T>`)
//! hold `T`'s own callbacks. Names follow Fahrenheit's Switch symbols for
//! the slots; all are thiscall on the descriptor with one stack word
//! (`ret 4`) and never read the descriptor:
//!
//! - slot `0x28` `constructInstance(p)`: builds a `T` at `p`. Classes with
//!   plain fields store their defaults when `p` is not 0 ([`INIT`]); the
//!   others call `T`'s constructor on `p` when `p` is not 0
//!   ([`CONSTRUCT`]), or do nothing ([`NOTHING`]). `al = 1`.
//! - slot `0x2c` `destroyInstance(p)`: calls `T`'s destructor on `p`
//!   (thiscall, no stack words, `p` not checked: [`DESTROY`]) or does
//!   nothing for a `T` without one ([`NOTHING`]). `al = 1`.
//! - slot `0x38` `fixObjectsAfterLoading(p)`: nothing to fix for these
//!   classes: `eax = 0` ([`FIX_NONE`]).
//!
//! `T::Bind()` (cdecl, no arguments): `PTypeDispenser::RegisterType(d)`
//! (`0x004395b0`, cdecl) then a tail call of
//! `PClassDescriptor::addToNamespace` (`0x0043c140`, thiscall on `d`, no
//! stack words), whose `eax` it returns; `d` is `T`'s
//! `PClassDescriptor` global ([`BIND`]).
//!
//! The constant of each copy (the init bytes, the callee, the descriptor)
//! is in the tables below, by the copy's address; the shims and the
//! oracle tests both read them from here.

use ffx_graphic::host::{Conv, Host};

/// `PTypeDispenser::RegisterType(descriptor)` (cdecl, `al`).
pub const REGISTER_TYPE: u32 = 0x0043_95b0;
/// `PClassDescriptor::addToNamespace` (thiscall on the descriptor).
pub const ADD_TO_NAMESPACE: u32 = 0x0043_c140;

/// `constructInstance` copies that store default bytes at `p`.
pub const INIT: &[(u32, &[u8])] = &[
    (
        0x0048_3100,
        &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    ),
    (
        0x0048_8ed0,
        &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    ),
    (
        0x0048_bd90,
        &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    ),
    (
        0x0048_bdb0,
        &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    ),
    (
        0x0049_3ce0,
        &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    ),
    (
        0x0049_3d00,
        &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    ),
    (
        0x0049_3d20,
        &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    ),
    (
        0x0049_3d40,
        &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    ),
    (
        0x0049_3d60,
        &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    ),
    (
        0x0049_8140,
        &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    ),
    (0x004a_2e40, &[0x00, 0x00]),
    (0x004a_2e60, &[0x00, 0x00, 0x00, 0x00]),
    (0x004a_2e80, &[0x00, 0x00, 0x34]),
    (
        0x004a_ff80,
        &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    ),
    (
        0x004a_ffa0,
        &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    ),
    (
        0x004b_9630,
        &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    ),
    (
        0x004b_9650,
        &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    ),
    (
        0x004b_9670,
        &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    ),
    (0x004b_9800, &[0x00, 0x00, 0x00, 0x00]),
    (
        0x004b_d580,
        &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    ),
    (
        0x004d_0550,
        &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    ),
    (
        0x004d_0570,
        &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    ),
    (
        0x004d_8610,
        &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    ),
];

/// `destroyInstance` copies and the destructor each calls.
pub const DESTROY: &[(u32, u32)] = &[
    (0x0044_3430, 0x0044_2a80),
    (0x0045_dcf0, 0x0045_c0f0),
    (0x0048_4870, 0x0048_4160),
    (0x0049_3f20, 0x0043_3af0),
    (0x004b_05b0, 0x0058_5290),
    (0x004b_05d0, 0x0058_5300),
    (0x004b_05f0, 0x0058_5360),
    (0x004b_0710, 0x004a_8860),
    (0x004b_0730, 0x004a_8910),
    (0x004b_0770, 0x0058_56b0),
    (0x004b_98e0, 0x0058_53c0),
    (0x004b_d5f0, 0x004b_be40),
    (0x004b_d610, 0x004b_bf50),
    (0x004c_01d0, 0x0058_5230),
    (0x004c_3c20, 0x004c_1df0),
    (0x004c_5490, 0x004c_1df0),
    (0x004c_6cb0, 0x004c_1df0),
    (0x004c_81a0, 0x0058_5180),
];

/// `constructInstance` copies and the constructor each calls.
pub const CONSTRUCT: &[(u32, u32)] = &[
    (0x0045_4960, 0x0045_30a0),
    (0x0045_6930, 0x0045_6370),
    (0x0049_3d80, 0x0043_3700),
];

/// `Bind` functions and the descriptor each registers.
pub const BIND: &[(u32, u32)] = &[
    (0x0048_1f60, 0x00c9_8948),
    (0x0048_5450, 0x00c9_8f60),
    (0x0048_7970, 0x00c9_9330),
    (0x0049_fdf0, 0x00c9_b560),
    (0x004a_9d80, 0x00c9_ca40),
    (0x004a_9da0, 0x00c9_c910),
    (0x004a_9dc0, 0x00c9_c9a8),
    (0x004a_a3f0, 0x00c9_bd30),
    (0x004a_a8a0, 0x00c9_c748),
    (0x004a_ae60, 0x00c9_c878),
    (0x004b_6330, 0x00ca_1598),
    (0x004b_ed40, 0x00ca_2930),
    (0x004c_33e0, 0x00ca_37d0),
    (0x004c_4c20, 0x00ca_3b60),
    (0x004c_64e0, 0x00ca_3ef0),
    (0x004c_7b50, 0x00ca_4200),
    (0x004d_36e0, 0x00ca_4e28),
];

/// `constructInstance` / `destroyInstance` copies that only return true.
pub const NOTHING: &[u32] = &[
    0x0046_a920,
    0x0046_a930,
    0x0048_0a30,
    0x0048_5d80,
    0x0048_8fa0,
    0x0048_90c0,
    0x0049_3f40,
    0x0049_84f0,
    0x0049_ec20,
    0x004a_05d0,
    0x004a_2f20,
    0x004a_2f30,
    0x004a_2f40,
    0x004a_2f50,
    0x004b_0170,
    0x004b_0280,
    0x004b_0610,
    0x004b_0620,
    0x004b_0630,
    0x004b_0640,
    0x004b_0650,
    0x004b_0660,
    0x004b_0670,
    0x004b_0680,
    0x004b_0690,
    0x004b_06a0,
    0x004b_06b0,
    0x004b_06c0,
    0x004b_06d0,
    0x004b_06e0,
    0x004b_06f0,
    0x004b_0700,
    0x004b_9980,
    0x004b_99d0,
    0x004c_01f0,
];

/// `fixObjectsAfterLoading` copies that only return 0.
pub const FIX_NONE: &[u32] = &[
    0x0045_5ba0,
    0x0045_6980,
    0x0048_0a40,
    0x0048_32b0,
    0x0048_4890,
    0x0048_91e0,
    0x0048_9200,
    0x0049_4020,
    0x0049_4030,
    0x0049_8640,
    0x004a_2f80,
    0x004a_2fb0,
    0x004a_2fc0,
    0x004b_07c0,
    0x004b_0900,
    0x004b_0910,
    0x004b_9a10,
    0x004b_d7c0,
    0x004c_0200,
    0x004c_3c40,
    0x004c_6cd0,
    0x004d_0640,
];

/// The constant of copy `f` in `table` (compile time: a missing copy is a
/// build error, not a run-time one).
#[must_use]
pub const fn callee_of(table: &[(u32, u32)], f: u32) -> u32 {
    let mut i = 0;
    while i < table.len() {
        if table[i].0 == f {
            return table[i].1;
        }
        i += 1;
    }
    panic!("not in the table");
}

/// The default bytes of `constructInstance` copy `f` ([`INIT`]).
#[must_use]
pub const fn init_of(f: u32) -> &'static [u8] {
    let mut i = 0;
    while i < INIT.len() {
        if INIT[i].0 == f {
            return INIT[i].1;
        }
        i += 1;
    }
    panic!("not in the table");
}

/// Whether `f` is in `table` (compile time, as [`callee_of`]).
#[must_use]
pub const fn listed(table: &[u32], f: u32) -> bool {
    let mut i = 0;
    while i < table.len() {
        if table[i] == f {
            return true;
        }
        i += 1;
    }
    panic!("not in the table");
}

pub mod bridge {
    //! The originals over game memory.

    use super::{ADD_TO_NAMESPACE, Conv, Host, REGISTER_TYPE};
    use ffx_bridge::Mem;

    /// `constructInstance` / `destroyInstance` of a class with nothing
    /// to do: true.
    #[must_use]
    pub const fn nothing_to_do() -> bool {
        true
    }

    /// `fixObjectsAfterLoading` of a class with nothing to fix: 0.
    #[must_use]
    pub const fn no_objects_to_fix() -> u32 {
        0
    }

    /// `constructInstance(p)`: `init` at `p` unless `p` is 0; true.
    pub fn construct_init(m: &mut impl Mem, p: u32, init: &[u8]) -> bool {
        if p != 0 {
            m.write_bytes(p, init);
        }
        true
    }

    /// `constructInstance(p)`: the constructor `ctor` (thiscall) on `p`
    /// unless `p` is 0; true.
    pub fn construct(h: &mut impl Host, ctor: u32, p: u32) -> bool {
        if p != 0 {
            h.call_sym(ctor, Conv::Thiscall(p), &[]);
        }
        true
    }

    /// `destroyInstance(p)`: the destructor `dtor` (thiscall) on `p`; true.
    pub fn destroy(h: &mut impl Host, dtor: u32, p: u32) -> bool {
        h.call_sym(dtor, Conv::Thiscall(p), &[]);
        true
    }

    /// `T::Bind()`: registers the descriptor `d` (a global) with the type
    /// dispenser, then adds it to its namespace; that call's `eax`.
    pub fn bind(h: &mut impl Host, d: u32) -> u32 {
        let d = h.rebase(d);
        h.call_sym(REGISTER_TYPE, Conv::Cdecl, &[d]);
        h.call_sym(ADD_TO_NAMESPACE, Conv::Thiscall(d), &[])
    }
}
