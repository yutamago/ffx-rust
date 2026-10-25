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
//! Arrays (`PArray<T>`, `PSharray<T>`: a header `[count | 0x80000000 when
//! the block is external, data]`): their `destroyInstance` releases an
//! owned block ([`ARRAY_DESTROY`], [`Release`]) and zeroes the header;
//! `PArray<T>::setToExternalBuffer(buffer, count)` ([`SET_EXTERNAL`]);
//! the member descriptors `PClassDataMemberArrayDynamic<T>` forward
//! `getCount` (slot `0x10`), `getArrayStart` (`0x18`) and
//! `setToExternalBuffer` (`0x20`) to the member functions they store at
//! `+0x34`, `+0x38`, `+0x3c`. Classes without a default constructor
//! copy the descriptor's default object ([`CONSTRUCT_DEFAULT`]).
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
    (0x0048_4830, &[0; 24]),
    (0x0049_8160, &[0; 16]),
    (0x004b_0290, &[0; 16]),
    (0x004b_96e0, &[0; 24]),
    (0x004b_9720, &[0; 24]),
    (0x004b_97b0, &[0; 24]),
    (
        0x0048_0a00,
        &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x34, 0xff],
    ),
    (0x0049_3da0, &[0; 32]),
    (0x004a_0530, &[0; 9]),
    (
        0x004a_ffc0,
        &[
            0x4b, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ],
    ),
    (
        0x004b_0000,
        &[
            0xde, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ],
    ),
    (
        0x004b_0030,
        &[
            0xe2, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ],
    ),
    (
        0x004b_0060,
        &[
            0x4c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ],
    ),
    (
        0x004b_00a0,
        &[
            0x49, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ],
    ),
    (
        0x004b_00e0,
        &[
            0x4d, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ],
    ),
    (
        0x004b_0110,
        &[
            0x4e, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ],
    ),
    (
        0x004b_0140,
        &[
            0x47, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ],
    ),
    (
        0x004b_0180,
        &[
            0x48, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ],
    ),
    (
        0x004b_01c0,
        &[
            0x42, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ],
    ),
    (
        0x004b_01f0,
        &[
            0x43, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ],
    ),
    (
        0x004b_0220,
        &[
            0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ],
    ),
    (
        0x004b_0250,
        &[
            0x4a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ],
    ),
    (
        0x004b_02c0,
        &[
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x34, 0x00,
        ],
    ),
    (
        0x004b_0440,
        &[
            0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ],
    ),
    (0x004b_9770, &[0; 16]),
    (0x0048_3170, &[0; 22]),
    (
        0x004b_d5a0,
        &[
            0x0f, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0,
        ],
    ),
    (
        0x004a_2ec0,
        &[0x00, 0x00, 0x46, 0x34, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
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
    (0x0048_3120, 0x0055_dc40),
    (0x0048_5c80, 0x0055_dd30),
    (0x0048_8ef0, 0x0043_37d0),
    (0x004a_04e0, 0x0058_3240),
    (0x004b_9690, 0x0058_3c00),
    (0x004c_0140, 0x0058_3b20),
    (0x004c_3b00, 0x0058_4240),
    (0x004c_5360, 0x0058_42c0),
    (0x004c_6b90, 0x0058_4400),
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

/// `PhyreFree` (cdecl `(block)`).
pub const FREE: u32 = 0x0042_fb20;

/// How an owned array block is released: the `PArray` / `PSharray`
/// header is `[count | 0x80000000 when external, data]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Release {
    /// `PhyreFree(data)` unless `data` is 0.
    Free,
    /// `PhyreFree(data)` when the count is more than 1 (a `PSharray`'s
    /// single element lives in the header), unless `data` is 0 when
    /// `check_null`.
    FreeShared {
        /// Whether a 0 `data` is skipped.
        check_null: bool,
    },
    /// `f(data, count)` (cdecl): destroys the elements and frees the
    /// block.
    Elements(u32),
}

/// `destroyInstance` copies of arrays (and classes holding one at an
/// offset): the array's offset in `p` and how its block is released.
pub const ARRAY_DESTROY: &[(u32, (u32, Release))] = &[
    (0x0048_31b0, (0x0, Release::Free)),
    (0x0048_3260, (0x8, Release::Free)),
    (0x0048_8fb0, (0x0, Release::Free)),
    (0x0048_9080, (0x8, Release::Free)),
    (
        0x0048_be50,
        (0x0, Release::FreeShared { check_null: false }),
    ),
    (
        0x0048_be90,
        (0x0, Release::FreeShared { check_null: false }),
    ),
    (0x0049_3de0, (0x0, Release::Free)),
    (0x0049_3e20, (0x0, Release::Free)),
    (0x0049_3e60, (0x0, Release::Elements(0x0043_4210))),
    (0x0049_3ea0, (0x0, Release::Free)),
    (0x0049_3ee0, (0x0, Release::Elements(0x0043_4360))),
    (0x0049_8480, (0x0, Release::Free)),
    (0x004b_0530, (0x0, Release::Elements(0x004a_b4d0))),
    (0x004b_0570, (0x0, Release::Elements(0x004a_b510))),
    (0x004b_9820, (0x0, Release::Free)),
    (0x004b_9860, (0x0, Release::Free)),
    (0x004b_98a0, (0x0, Release::Free)),
    (0x004b_9900, (0x10, Release::Free)),
    (0x004b_9940, (0x10, Release::Free)),
    (0x004b_9990, (0x10, Release::Free)),
    (0x004d_0590, (0x0, Release::FreeShared { check_null: true })),
    (0x004d_05e0, (0x0, Release::FreeShared { check_null: true })),
];

/// `destroyInstance` copies that free the untagged block at `p + off`
/// (a pointer whose bit 0 marks it as not owned), by offset.
pub const TAGGED_FREE: &[(u32, u32)] = &[
    (0x0045_4980, 0xcc),
    (0x0045_dd10, 0x4),
    (0x004a_2f60, 0x4),
    (0x004b_0750, 0x4),
    (0x004d_86c0, 0x10),
];

/// `destroyInstance` copies that call the object's virtual destructor
/// through `p - 4` (the descriptor's `p` is past a vtable pointer).
pub const VIRTUAL_DELETE: &[u32] = &[0x0045_5b70, 0x0045_6950];

/// `destroyInstance` copies: `[check, release, destructor]`, thiscall on
/// `p`; the release runs when the check's `al` is set.
pub const DESTROY_CHECKED: &[(u32, [u32; 3])] = &[
    (0x0048_31f0, [0x0056_2da0, 0x0056_46d0, 0x0055_dfa0]),
    (0x0048_5d10, [0x0056_2db0, 0x0056_48c0, 0x0055_e060]),
    (0x004a_0560, [0x0059_5f70, 0x0059_c540, 0x0058_4dc0]),
    (0x004c_3bb0, [0x0059_5f90, 0x0059_cc40, 0x0058_5840]),
    (0x004c_5420, [0x0059_5fa0, 0x0059_cf00, 0x0058_58d0]),
    (0x004c_6c40, [0x0059_5fb0, 0x0059_d0f0, 0x0058_5960]),
];

/// `constructInstance` copies of classes without a default constructor:
/// `[descriptor, its default-object getter, copy constructor]`.
pub const CONSTRUCT_DEFAULT: &[(u32, [u32; 3])] = &[
    (0x004b_02f0, [0x00c9_1048, 0x0043_cbb0, 0x0058_3b40]),
    (0x004b_0360, [0x00c9_1048, 0x0043_cbb0, 0x0058_3bc0]),
    (0x004b_03d0, [0x00c9_1048, 0x0043_cbb0, 0x0058_3be0]),
    (0x004b_04c0, [0x00c9_1048, 0x0043_cbb0, 0x0058_4060]),
    (0x004c_8130, [0x00ca_37d0, 0x0043_cbb0, 0x0058_3a60]),
    (0x004d_70d0, [0x00ca_5ff0, 0x0043_cbb0, 0x004d_6180]),
];

/// `PArray::setToExternalBuffer` copies and how each releases the old
/// block.
pub const SET_EXTERNAL: &[(u32, Release)] = &[
    (0x0048_39d0, Release::Free),
    (0x0048_9880, Release::Free),
    (0x0049_5490, Release::Free),
    (0x0049_54e0, Release::Elements(0x0043_4210)),
    (0x0049_5530, Release::Free),
    (0x0049_5580, Release::Elements(0x0043_4360)),
    (0x0049_9220, Release::Free),
    (0x004b_2660, Release::Elements(0x004a_b4d0)),
    (0x004b_26b0, Release::Elements(0x004a_b510)),
    (0x004b_aa70, Release::Free),
    (0x004b_ab10, Release::Free),
    (0x004b_dde0, Release::Elements(0x004b_c850)),
    (0x0049_5440, Release::Free),
];

/// Element releases: each 16-byte element's untagged `+4` block, then the
/// array's.
pub const RELEASE_TAGGED: &[u32] = &[0x004a_b4d0, 0x004a_b510];

/// `PClassDataMemberArrayDynamic<T>::setToExternalBuffer` copies.
pub const MEMBER_SET_EXTERNAL: &[u32] = &[
    0x0048_3a20,
    0x0048_98d0,
    0x0048_c730,
    0x0049_55d0,
    0x0049_5600,
    0x0049_5630,
    0x0049_5660,
    0x0049_5690,
    0x0049_9270,
    0x004b_2700,
    0x004b_2730,
    0x004b_ab60,
    0x004b_abc0,
    0x004b_de30,
];

/// `PClassDataMemberArrayDynamic<T>::getCount` copies.
pub const MEMBER_COUNT: &[u32] = &[0x0048_c140, 0x004d_0780];

/// The array offset and release of [`ARRAY_DESTROY`] copy `f`.
#[must_use]
pub const fn array_of(f: u32) -> (u32, Release) {
    let mut i = 0;
    while i < ARRAY_DESTROY.len() {
        if ARRAY_DESTROY[i].0 == f {
            return ARRAY_DESTROY[i].1;
        }
        i += 1;
    }
    panic!("not in the table");
}

/// The release of [`SET_EXTERNAL`] copy `f`.
#[must_use]
pub const fn release_of(table: &[(u32, Release)], f: u32) -> Release {
    let mut i = 0;
    while i < table.len() {
        if table[i].0 == f {
            return table[i].1;
        }
        i += 1;
    }
    panic!("not in the table");
}

/// The three callees of copy `f` in `table`.
#[must_use]
pub const fn three_of(table: &[(u32, [u32; 3])], f: u32) -> [u32; 3] {
    let mut i = 0;
    while i < table.len() {
        if table[i].0 == f {
            return table[i].1;
        }
        i += 1;
    }
    panic!("not in the table");
}

/// Whether an array header's count word says the block is the array's
/// own (no external flag).
#[must_use]
pub const fn owned(count: u32) -> bool {
    count & 0x8000_0000 == 0
}

/// `(offset, bytes)` runs of default fields.
pub type Spans = &'static [(u32, &'static [u8])];

/// `constructInstance` copies whose default fields leave gaps: `(offset,
/// bytes)` runs.
pub const INIT_SPANS: &[(u32, Spans)] = &[
    (
        0x0048_5cd0,
        &[
            (
                0x0,
                &[
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x34,
                    0x00, 0x00,
                ],
            ),
            (0x10, &[0; 4]),
        ],
    ),
    (
        0x004a_2ea0,
        &[
            (0x0, &[0; 2]),
            (0x4, &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x34]),
        ],
    ),
    (
        0x0048_8f40,
        &[
            (
                0x0,
                &[
                    0, 0, 0, 0, 0xff, 0xff, 0xff, 0xff, 0, 0, 0, 0, 0, 0, 0, 0, 0x06, 0, 0, 0, 0,
                ],
            ),
            (
                0x18,
                &[
                    0, 0, 0, 0, 0, 0, 0, 0, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                ],
            ),
        ],
    ),
];

/// Callbacks that hand `p` to a cdecl function of the class and return its
/// `eax`.
pub const FORWARD: &[(u32, u32)] = &[
    (0x0045_dcd0, 0x0045_c9e0),
    (0x0045_dd70, 0x0045_cd60),
    (0x004d_7230, 0x004d_66c0),
];

/// `constructInstance` copies that make an empty `PWorld` (a list head).
pub const LIST_HEAD: &[u32] = &[0x0044_3400];

/// `destroyInstance` copies with `PTextureCommonBase`'s destructor inlined.
pub const TEXTURE_DESTROY: &[u32] = &[0x004c_26e0];

/// The runs of [`INIT_SPANS`] copy `f`.
#[must_use]
pub const fn spans_of(f: u32) -> Spans {
    let mut i = 0;
    while i < INIT_SPANS.len() {
        if INIT_SPANS[i].0 == f {
            return INIT_SPANS[i].1;
        }
        i += 1;
    }
    panic!("not in the table");
}

/// `PCameraOrthographic`'s constructInstance pieces: the base at `p + 4`
/// (`0x004530a0`, thiscall), its vtable, its setup (`0x00455fe0`, thiscall).
pub const CAMERA_ORTHO: [u32; 3] = [0x0045_30a0, 0x00b1_1040, 0x0045_5fe0];
/// `PShaderSource`'s default object: the
/// descriptor and its getter (constructInstance `0x004b0470`).
pub const SHADER_SOURCE_DEFAULT: [u32; 2] = [0x00c9_1048, 0x0043_cbb0];

/// `PLight`'s constructInstance constants: the image floats at `+0`..`+8`,
/// `+0x20`, `+0x2c` and the pointer at `+0x18`.
pub const LIGHT: [u32; 4] = [0x00b9_2194, 0x00b1_7500, 0x00b1_7504, 0x00c1_0814];
/// `PBase::GetClassDescriptor()` (cdecl).
pub const PBASE_DESCRIPTOR: u32 = 0x0043_5740;

pub mod bridge {
    //! The originals over game memory.

    use super::{
        ADD_TO_NAMESPACE, Conv, FREE, Host, PBASE_DESCRIPTOR, REGISTER_TYPE, Release, owned,
    };
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

    /// Releases an owned block as `how` says (`count` the header word).
    fn release(h: &mut impl Host, how: Release, count: u32, data: u32) {
        let n = count & 0x7fff_ffff;
        match how {
            Release::Free => {
                if data != 0 {
                    h.call_sym(FREE, Conv::Cdecl, &[data]);
                }
            }
            Release::FreeShared { check_null } => {
                if n > 1 && (data != 0 || !check_null) {
                    h.call_sym(FREE, Conv::Cdecl, &[data]);
                }
            }
            Release::Elements(f) => {
                h.call_sym(f, Conv::Cdecl, &[data, n]);
            }
        }
    }

    /// `destroyInstance(p)` of an array at `p + off`: its block released
    /// when owned, then the header zeroed; true.
    pub fn destroy_array(h: &mut impl Host, p: u32, off: u32, how: Release) -> bool {
        let a = p.wrapping_add(off);
        let count: u32 = h.read(a);
        if owned(count) {
            let data: u32 = h.read(a + 4);
            release(h, how, count, data);
        }
        h.write(a + 4, 0u32);
        h.write(a, 0u32);
        true
    }

    /// `destroyInstance(p)`: frees the block at `p + off` unless its bit
    /// 0 is set (a tagged pointer); true.
    pub fn destroy_tagged(h: &mut impl Host, p: u32, off: u32) -> bool {
        let block: u32 = h.read(p.wrapping_add(off));
        if block & 1 == 0 {
            h.call_sym(FREE, Conv::Cdecl, &[block]);
        }
        true
    }

    /// `destroyInstance(p)`: the object at `p - 4` (0 when `p` is 0, as
    /// the original) runs its virtual destructor (slot 0, flags 0); true.
    pub fn destroy_virtual(h: &mut impl Host, p: u32) -> bool {
        let object = if p == 0 { 0 } else { p.wrapping_sub(4) };
        let vtable: u32 = h.read(object);
        let f: u32 = h.read(vtable);
        h.call(f, Conv::Thiscall(object), &[0]);
        true
    }

    /// `destroyInstance(p)`: `release(p)` when `check(p)` sets `al`, then
    /// `dtor(p)` (thiscall); true.
    pub fn destroy_checked(h: &mut impl Host, [check, release, dtor]: [u32; 3], p: u32) -> bool {
        if h.call_sym(check, Conv::Thiscall(p), &[]) & 0xff != 0 {
            h.call_sym(release, Conv::Thiscall(p), &[]);
        }
        h.call_sym(dtor, Conv::Thiscall(p), &[]);
        true
    }

    /// `constructInstance(p)` without a default constructor: the default
    /// object from `get(descriptor)`; 0 when there is none. Else the copy
    /// constructor `ctor(p, default)` unless `p` is 0, and `eax` with
    /// `al = 1` over the last call's.
    pub fn construct_default(h: &mut impl Host, [descriptor, get, ctor]: [u32; 3], p: u32) -> u32 {
        let d = h.rebase(descriptor);
        let default = h.call_sym(get, Conv::Thiscall(d), &[]);
        if default == 0 {
            return 0;
        }
        let eax = if p == 0 {
            default
        } else {
            h.call_sym(ctor, Conv::Thiscall(p), &[default])
        };
        (eax & !0xff) | 1
    }

    /// `PArray::setToExternalBuffer(array, buffer, count)`: for `count >=
    /// 0`, an owned old block other than `buffer` released as `how`, then
    /// `buffer` with `count` marked external; 0.
    pub fn set_external(
        h: &mut impl Host,
        array: u32,
        buffer: u32,
        count: u32,
        how: Release,
    ) -> u32 {
        if (count as i32) < 0 {
            return 0;
        }
        let old: u32 = h.read(array);
        let data: u32 = h.read(array + 4);
        if data != buffer && owned(old) {
            release(h, how, old, data);
        }
        h.write(array + 4, buffer);
        h.write(array, count | 0x8000_0000);
        0
    }

    /// `(data, count)`: unless `data` is 0, frees each 16-byte element's
    /// `+4` block whose bit 0 is clear, then `data`.
    pub fn release_tagged(h: &mut impl Host, data: u32, count: u32) {
        if data == 0 {
            return;
        }
        let mut e = data.wrapping_add(4);
        for _ in 0..count {
            let block: u32 = h.read(e);
            if block & 1 == 0 {
                h.call_sym(FREE, Conv::Cdecl, &[block]);
            }
            e = e.wrapping_add(16);
        }
        h.call_sym(FREE, Conv::Cdecl, &[data]);
    }

    /// `PClassDataMemberArrayDynamic::setToExternalBuffer(member, object,
    /// buffer, count)`: the member's setter (`member +0x3c`, thiscall on
    /// `object`), or 0x15 without one.
    pub fn member_set_external(
        h: &mut impl Host,
        member: u32,
        object: u32,
        buffer: u32,
        count: u32,
    ) -> u32 {
        let f: u32 = h.read(member + 0x3c);
        if f == 0 {
            return 0x15;
        }
        h.call(f, Conv::Thiscall(object), &[buffer, count])
    }

    /// `PClassDataMemberArrayDynamic::getCount(member, object)`: the
    /// member's getter (`member +0x34`), or 0 without one.
    pub fn member_count(h: &mut impl Host, member: u32, object: u32) -> u32 {
        let f: u32 = h.read(member + 0x34);
        if f == 0 {
            return 0;
        }
        h.call(f, Conv::Thiscall(object), &[])
    }

    /// `PClassDataMemberArrayDynamic::getArrayStart(member, object)`: the
    /// member's getter (`member +0x38`), or the word at the member's
    /// offset in `object` (`member +0x24` less its class's base, `member
    /// +0xc` then `+0x88`).
    pub fn member_start(h: &mut impl Host, member: u32, object: u32) -> u32 {
        let f: u32 = h.read(member + 0x38);
        if f != 0 {
            return h.call(f, Conv::Thiscall(object), &[]);
        }
        let class: u32 = h.read(member + 0xc);
        let off = h
            .read::<u32>(member + 0x24)
            .wrapping_sub(h.read(class + 0x88));
        h.read(off.wrapping_add(object))
    }

    /// `constructInstance(p)`: each `(offset, bytes)` run at `p` unless `p`
    /// is 0; true.
    pub fn construct_spans(m: &mut impl Mem, p: u32, spans: &[(u32, &[u8])]) -> bool {
        if p != 0 {
            for &(off, bytes) in spans {
                m.write_bytes(p + off, bytes);
            }
        }
        true
    }

    /// A callback that hands `p` to the class's cdecl function `f`: its
    /// `eax`.
    pub fn forward(h: &mut impl Host, f: u32, p: u32) -> u32 {
        h.call_sym(f, Conv::Cdecl, &[p])
    }

    /// `constructInstance(p)` of `PWorld`: `PWorld::PWorld` in place
    /// unless `p` is 0; true.
    pub fn construct_list_head(m: &mut impl Mem, p: u32) -> bool {
        if p != 0 {
            crate::basic::bridge::world_ctor(m, p);
        }
        true
    }

    /// `destroyInstance(p)` of `PTextureCommonBase`: its destructor's body
    /// on `p`; true.
    pub fn destroy_texture(h: &mut impl Host, p: u32) -> bool {
        crate::texture::bridge::common_dtor(h, p);
        true
    }

    /// `constructInstance(p)` of `PTextureCommonBase`: its constructor in
    /// place unless `p` is 0; true.
    pub fn construct_texture(m: &mut impl Mem, p: u32) -> bool {
        if p != 0 {
            crate::texture::bridge::common_ctor(m, p);
        }
        true
    }

    /// `constructInstance(p)` of `PSamplerStateBase`: its constructor in
    /// place unless `p` is 0; true.
    pub fn construct_sampler(m: &mut impl Mem, p: u32) -> bool {
        if p != 0 {
            crate::texture::bridge::sampler_ctor(m, p);
        }
        true
    }

    /// `constructInstance(p)` of `PCameraOrthographic` unless `p` is 0:
    /// the base at `p + 4`, 1.0 at `+0x150` and `+0x154`, its vtable, its
    /// setup; true.
    pub fn construct_camera_ortho(
        h: &mut impl Host,
        [base, vtable, setup]: [u32; 3],
        p: u32,
    ) -> bool {
        if p != 0 {
            h.call_sym(base, Conv::Thiscall(p + 4), &[]);
            h.write(p + 0x150, 1.0f32);
            h.write(p + 0x154, 1.0f32);
            let vtable = h.rebase(vtable);
            h.write(p, vtable);
            h.call_sym(setup, Conv::Thiscall(p), &[]);
        }
        true
    }

    /// `constructInstance(p)` from the descriptor's default object
    /// pointer: 0 without one; else unless `p` is 0 the object at `+0`
    /// and four words 0, and `eax` = `p` with `al = 1`.
    pub fn construct_default_fields(h: &mut impl Host, [descriptor, get]: [u32; 2], p: u32) -> u32 {
        let d = h.rebase(descriptor);
        let default = h.call_sym(get, Conv::Thiscall(d), &[]);
        if default == 0 {
            return 0;
        }
        if p != 0 {
            h.write(p, default);
            for i in 1..5 {
                h.write(p + 4 * i, 0u32);
            }
        }
        // `al = 1` over `p`, which the original has in `eax` by then
        (p & !0xff) | 1
    }

    /// `constructInstance(p)` of `PTexture2DBase`: its constructor in place
    /// unless `p` is 0; true.
    pub fn construct_texture_2d(m: &mut impl Mem, p: u32) -> bool {
        if p != 0 {
            crate::texture::bridge::ctor_2d(m, p);
        }
        true
    }

    /// `constructInstance(p)` of `PTextureCubeMapBase`: its constructor in
    /// place unless `p` is 0; true.
    pub fn construct_texture_cube(m: &mut impl Mem, p: u32) -> bool {
        if p != 0 {
            crate::texture::bridge::ctor_cube(m, p);
        }
        true
    }

    /// `constructInstance(p)` of `PLight` unless `p` is 0: the image's
    /// float three times, 1.0, 0, 0, a pointer, 0.0, a float, 1.0, 0.0, a
    /// float, 0.0 (each through the x87, as `fld`/`fst` copies them); true.
    pub fn construct_light(m: &mut impl Mem, [a, b, c, ptr]: [u32; 4], p: u32) -> bool {
        use ffx_bridge::x87;
        if p != 0 {
            let copy = |m: &mut _, at: u32| -> u32 {
                let at = Mem::rebase(m, at);
                x87::store(x87::load(Mem::read(m, at)))
            };
            let a = copy(m, a);
            let b = copy(m, b);
            let c = copy(m, c);
            let one = 1.0f32.to_bits();
            let ptr = m.rebase(ptr);
            for (off, v) in [
                (0, a),
                (4, a),
                (8, a),
                (0xc, one),
                (0x10, 0),
                (0x14, 0),
                (0x18, ptr),
                (0x1c, 0),
                (0x20, b),
                (0x24, one),
                (0x28, 0),
                (0x2c, c),
                (0x30, 0),
            ] {
                m.write(p + off, v);
            }
        }
        true
    }

    /// `constructInstance(p)` of `PAssetReferenceImport` unless `p` is 0:
    /// `PBase::GetClassDescriptor()` at `+0`, two words 0; true.
    pub fn construct_asset_import(h: &mut impl Host, p: u32) -> bool {
        if p != 0 {
            let d = h.call_sym(PBASE_DESCRIPTOR, Conv::Cdecl, &[]);
            h.write(p, d);
            h.write(p + 4, 0u32);
            h.write(p + 8, 0u32);
        }
        true
    }

    /// `constructInstance(p)` of `PTexture3DBase`: its constructor in place
    /// unless `p` is 0; true.
    pub fn construct_texture_3d(m: &mut impl Mem, p: u32) -> bool {
        if p != 0 {
            crate::texture::bridge::ctor_3d(m, p);
        }
        true
    }
}
