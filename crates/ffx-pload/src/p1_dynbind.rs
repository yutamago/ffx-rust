//! `PClassDescriptorDynamic::Bind` (`0x00447da0`, cdecl `()`, `eax` from
//! `addToNamespace`): registers the dynamic class descriptor's type, its
//! eight data members (`m_size` ... `m_nameAsString`) and its three
//! `PArray<PUInt8>` buffers (`m_defaultBufferStorage`,
//! `m_validationBufferStorage`, `m_writeMaskBufferStorage`).
//!
//! Each buffer member is an array member (`PClassDataMemberArray`,
//! `0x00575ef0`, seven words) whose `PArray<PUInt8>` descriptor
//! (`0x00c915c8`) is built inline once under its own guard (`0x00c9165c`
//! bit 0: `0x004473f0`, flag `+0x90` bit 1 cleared, its vtable, `atexit`).
//! Before each buffer, `0x00437d30()` and `0x00448450(name)` with a
//! global's word, results dropped.
//!
//! That is [`ffx_prender::h1_bind`]'s format, so the function is the table
//! [`DYNAMIC_BIND`] run by [`ffx_prender::h1_bind::bridge::bind`]. Read off
//! the machine code with h1's generator (`~/ffx-work/m4/port/p2c/gen.py`)
//! and checked step by step against the disassembly.

use ffx_prender::h1_bind::{Bind, Step, Ty, Val};

/// The original.
pub const ADDR: u32 = 0x0044_7da0;

/// The guard word of the members.
pub const GUARD: u32 = 0x00c9_13f0;
/// The guard word of the inline `PArray<PUInt8>` descriptor.
pub const ARRAY_GUARD: u32 = 0x00c9_165c;
/// The inline `PArray<PUInt8>` descriptor.
pub const ARRAY_DESC: u32 = 0x00c9_15c8;

/// One buffer member: `0x00437d30()`, `0x00448450([name_word])`, then
/// under `bit` the inline descriptor and the array member.
macro_rules! buffer {
    ($word:expr, $bit:expr, $member:expr, $name:expr, $offset:expr, $dtor:expr) => {
        [
            Step::Call(0x0043_7d30, None),
            Step::Call(0x0044_8450, Some($word)),
            Step::Guard(
                GUARD,
                $bit,
                &[
                    Step::Guard(
                        ARRAY_GUARD,
                        0x1,
                        &[
                            Step::Descriptor {
                                obj: ARRAY_DESC,
                                ctor: 0x0044_73f0,
                            },
                            Step::And(0x00c9_1658, 0xffff_fffd),
                            Step::Store(ARRAY_DESC, Val::Addr(0x00b0_fcf4)),
                            Step::AtExit(0x00af_a420),
                        ],
                    ),
                    Step::Array {
                        member: $member,
                        ty: Ty::Addr(ARRAY_DESC),
                        name: $name,
                        words: [$offset, 0x0, 0x3, 0x1],
                    },
                    Step::Store($member, Val::Addr(0x00b0_fbe0)),
                    Step::AtExit($dtor),
                ],
            ),
        ]
    };
}

/// A data member under bit `bit`.
macro_rules! member {
    ($bit:expr, $member:expr, $ty:expr, $name:expr, $w:expr, $dtor:expr) => {
        Step::Guard(
            GUARD,
            $bit,
            &[
                Step::Member {
                    ctor: 0x0057_52a0,
                    member: $member,
                    ty: $ty,
                    name: $name,
                    words: $w,
                },
                Step::AtExit($dtor),
            ],
        )
    };
}

const B1: [Step; 3] = buffer!(
    0x00c0_b480,
    0x100,
    0x00c9_1528,
    0x00b0_fc0c,
    0x9c,
    0x00af_a470
);
const B2: [Step; 3] = buffer!(
    0x00c0_b484,
    0x200,
    0x00c9_155c,
    0x00b0_fc24,
    0xa4,
    0x00af_a4f0
);
const B3: [Step; 3] = buffer!(
    0x00c0_b488,
    0x400,
    0x00c9_1590,
    0x00b0_fc40,
    0xac,
    0x00af_a500
);

/// `Phyre_PClassDescriptorDynamic_Bind`'s steps.
pub const DYNAMIC_BIND: Bind = Bind {
    d: 0x00c9_1330,
    steps: &[
        // m_size: GetType<unsigned int>
        member!(
            0x1,
            0x00c9_13c4,
            Ty::Call(0x0043_7fc0),
            0x00b0_fb4c,
            [0x1c, 0, 0],
            0x00af_a4e0
        ),
        // m_parent: the class descriptor's type
        member!(
            0x2,
            0x00c9_13f4,
            Ty::Addr(0x00c9_0760),
            0x00b0_fb54,
            [0x40, 2, 0],
            0x00af_a4d0
        ),
        // m_offsetFromParent, m_offsetToBase, m_offsetToBaseInAllocatedBlock
        member!(
            0x4,
            0x00c9_1420,
            Ty::Call(0x0043_7f10),
            0x00b0_fb60,
            [0x84, 0, 0],
            0x00af_a4a0
        ),
        member!(
            0x8,
            0x00c9_144c,
            Ty::Call(0x0043_7f10),
            0x00b0_fb74,
            [0x88, 0, 0],
            0x00af_a4b0
        ),
        member!(
            0x10,
            0x00c9_1478,
            Ty::Call(0x0043_7f10),
            0x00b0_fb84,
            [0x8c, 0, 0],
            0x00af_a4c0
        ),
        // m_currentMemberCount
        member!(
            0x20,
            0x00c9_14a4,
            Ty::Call(0x0043_7e70),
            0x00b0_fba4,
            [0x96, 0, 0],
            0x00af_a460
        ),
        // m_actualSize: GetType<unsigned int>
        member!(
            0x40,
            0x00c9_14d0,
            Ty::Call(0x0043_7fc0),
            0x00b0_fbbc,
            [0x98, 0, 0],
            0x00af_a440
        ),
        // m_nameAsString
        member!(
            0x80,
            0x00c9_14fc,
            Ty::Addr(0x00c9_1048),
            0x00b0_f9a0,
            [0xb4, 0, 0],
            0x00af_a490
        ),
        B1[0],
        B1[1],
        B1[2],
        B2[0],
        B2[1],
        B2[2],
        B3[0],
        B3[1],
        B3[2],
    ],
};
