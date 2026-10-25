//! `PhyreEngine`'s class registration for the render classes (M4 port
//! round 1, h1-render-a): the class descriptors' constructors, the type
//! singletons (`PTypeDispenser::GetType<T>`), and the `Bind` functions
//! that also construct a member descriptor.
//!
//! - A descriptor's constructor (thiscall `(this)` -> `this`):
//!   `PClassDescriptor::PClassDescriptor(this, ns, name, 8, 4, parent, 0)`
//!   (`0x0043b0a0`, `ret 0x18`) with `ns` from
//!   `PNamespace::GetGlobalNamespace()` (`0x0043e2f0`, cdecl, no
//!   arguments: the original pushes the other five words before calling
//!   it), the name from the class's name global and `parent` from
//!   `PBase::GetClassDescriptor()` (`0x00435740`); then flags `+0x90 |=
//!   2`, its vtable at `+0`, and `setDefaultBuffer`,
//!   `setValidationBuffer`, `setWriteMaskBuffer` with the class's three
//!   buffers ([`DESCRIPTOR_CTOR`]: name global, vtable, buffers).
//! - `GetType<T>()` (cdecl) -> the type object: on the first call (bit 0
//!   of its guard word) the object's constructor (thiscall `(name, 0x14,
//!   4, f1, f2)`), its vtable, and its destructor registered with
//!   `atexit` ([`TYPE_SINGLETON`]: guard, object, constructor, name, f1,
//!   f2, vtable, destructor).
//! - `T::Bind()` with a member (cdecl) -> `addToNamespace`'s `eax`:
//!   `RegisterType(d)`, then on the first call the member descriptor's
//!   constructor (thiscall `(d, other, name, size, kind, 0)`) and its
//!   destructor with `atexit`, then `addToNamespace(d)` ([`BIND_MEMBER`]:
//!   `d`, then guard, member, constructor, other, name, size, kind,
//!   destructor).

use ffx_graphic::host::{Conv, Host};

use crate::descriptor::{ADD_TO_NAMESPACE, REGISTER_TYPE};

/// `PBase::GetClassDescriptor()` (cdecl).
pub const PBASE_DESCRIPTOR: u32 = 0x0043_5740;
/// `PNamespace::GetGlobalNamespace()` (cdecl).
pub const GLOBAL_NAMESPACE: u32 = 0x0043_e2f0;
/// `PClassDescriptor::PClassDescriptor` (thiscall, six words).
pub const DESCRIPTOR_BASE_CTOR: u32 = 0x0043_b0a0;
/// `PClassDescriptor::setDefaultBuffer` (thiscall, one word).
pub const SET_DEFAULT_BUFFER: u32 = 0x0043_d510;
/// `PClassDescriptor::setValidationBuffer` (thiscall, one word).
pub const SET_VALIDATION_BUFFER: u32 = 0x0043_d5b0;
/// `PClassDescriptor::setWriteMaskBuffer` (thiscall, one word).
pub const SET_WRITE_MASK_BUFFER: u32 = 0x0043_d5c0;
/// The CRT's `atexit` (cdecl).
pub const ATEXIT: u32 = 0x0094_9207;

/// Descriptor constructors: `[name global, vtable, default buffer,
/// validation buffer, write-mask buffer]`.
pub const DESCRIPTOR_CTOR: &[(u32, [u32; 5])] = &[
    (
        0x0045_77e0,
        [
            0x00c9_3dd4,
            0x00b1_15bc,
            0x00c9_3e10,
            0x00c9_3e20,
            0x00c9_3e30,
        ],
    ),
    (
        0x0048_1470,
        [
            0x00c9_8ce0,
            0x00b1_43b4,
            0x00c9_8cf0,
            0x00c9_8d00,
            0x00c9_8d10,
        ],
    ),
    (
        0x0048_6c20,
        [
            0x00c9_98bc,
            0x00b1_513c,
            0x00c9_98c0,
            0x00c9_98d0,
            0x00c9_98e0,
        ],
    ),
    (
        0x0048_a310,
        [
            0x00c9_9b98,
            0x00b1_55dc,
            0x00c9_9bc0,
            0x00c9_9bd0,
            0x00c9_9be0,
        ],
    ),
    (
        0x0048_e7e0,
        [
            0x00c9_a628,
            0x00b1_5d8c,
            0x00c9_a650,
            0x00c9_a660,
            0x00c9_a670,
        ],
    ),
    (
        0x0048_e880,
        [
            0x00c9_a630,
            0x00b1_5f9c,
            0x00c9_a6b0,
            0x00c9_a6c0,
            0x00c9_a6d0,
        ],
    ),
    (
        0x0048_e920,
        [
            0x00c9_a638,
            0x00b1_61ac,
            0x00c9_a710,
            0x00c9_a720,
            0x00c9_a730,
        ],
    ),
    (
        0x0048_e9c0,
        [
            0x00c9_a634,
            0x00b1_60a4,
            0x00c9_a6e0,
            0x00c9_a6f0,
            0x00c9_a700,
        ],
    ),
    (
        0x0048_ea60,
        [
            0x00c9_a62c,
            0x00b1_5e94,
            0x00c9_a680,
            0x00c9_a690,
            0x00c9_a6a0,
        ],
    ),
    (
        0x0049_6610,
        [
            0x00c9_ab60,
            0x00b1_6afc,
            0x00c9_ab70,
            0x00c9_ab80,
            0x00c9_ab90,
        ],
    ),
    (
        0x0049_9b00,
        [
            0x00c9_aecc,
            0x00b1_6f24,
            0x00c9_aed0,
            0x00c9_aee0,
            0x00c9_aef0,
        ],
    ),
    (
        0x004a_6730,
        [
            0x00ca_12d4,
            0x00b1_b954,
            0x00ca_12e0,
            0x00ca_12f0,
            0x00ca_1300,
        ],
    ),
    (
        0x004a_67d0,
        [
            0x00ca_12d8,
            0x00b1_ba5c,
            0x00ca_1310,
            0x00ca_1320,
            0x00ca_1330,
        ],
    ),
    (
        0x004b_4790,
        [
            0x00ca_24bc,
            0x00b1_ccb4,
            0x00ca_2520,
            0x00ca_2530,
            0x00ca_2540,
        ],
    ),
    (
        0x004b_4830,
        [
            0x00ca_24b4,
            0x00b1_caa4,
            0x00ca_24c0,
            0x00ca_24d0,
            0x00ca_24e0,
        ],
    ),
    (
        0x004b_48d0,
        [
            0x00ca_24b8,
            0x00b1_cbac,
            0x00ca_24f0,
            0x00ca_2500,
            0x00ca_2510,
        ],
    ),
    (
        0x004b_b9f0,
        [
            0x00ca_2860,
            0x00b1_d3dc,
            0x00ca_2870,
            0x00ca_2880,
            0x00ca_2890,
        ],
    ),
    (
        0x004c_9020,
        [
            0x00ca_4b90,
            0x00b1_f0d4,
            0x00ca_4ba0,
            0x00ca_4bb0,
            0x00ca_4bc0,
        ],
    ),
    (
        0x004c_f1d0,
        [
            0x00ca_4de8,
            0x00b1_f534,
            0x00ca_4df0,
            0x00ca_4e00,
            0x00ca_4e10,
        ],
    ),
];

/// `GetType<T>` copies: `[guard, object, constructor, name, f1, f2,
/// vtable, destructor]`.
pub const TYPE_SINGLETON: &[(u32, [u32; 8])] = &[
    (
        0x0049_d050,
        [
            0x00c9_b048,
            0x00c9_b014,
            0x005a_0200,
            0x00b1_71cc,
            0x0049_e550,
            0x0049_e900,
            0x00b1_729c,
            0x00af_cf00,
        ],
    ),
    (
        0x0049_d100,
        [
            0x00c9_b048,
            0x00c9_b014,
            0x005a_0200,
            0x00b1_71cc,
            0x0049_e550,
            0x0049_e900,
            0x00b1_729c,
            0x00af_cf00,
        ],
    ),
    (
        0x004c_0b10,
        [
            0x00ca_3528,
            0x00ca_34f4,
            0x005a_0200,
            0x00b1_db24,
            0x004c_0f80,
            0x004c_13f0,
            0x00b1_db3c,
            0x00af_e260,
        ],
    ),
    (
        0x004c_8a70,
        [
            0x00ca_4590,
            0x00ca_455c,
            0x005a_0200,
            0x00b1_ed64,
            0x004c_b910,
            0x004c_c030,
            0x00b1_ef38,
            0x00af_e610,
        ],
    ),
    (
        0x004c_8ba0,
        [
            0x00ca_4590,
            0x00ca_455c,
            0x005a_0200,
            0x00b1_ed64,
            0x004c_b910,
            0x004c_c030,
            0x00b1_ef38,
            0x00af_e610,
        ],
    ),
];

/// `Bind` copies with a member: `(d, [guard, member, constructor, other,
/// name, size, kind, destructor])`.
pub const BIND_MEMBER: &[(u32, (u32, [u32; 8]))] = &[
    (
        0x004a_9ec0,
        (
            0x00c9_c3b8,
            [
                0x00c9_ce14,
                0x00c9_cde8,
                0x0057_52a0,
                0x00c9_8948,
                0x00b1_b780,
                0xc,
                0x12,
                0x00af_d790,
            ],
        ),
    ),
    (
        0x004a_9f60,
        (
            0x00c9_c450,
            [
                0x00c9_ce44,
                0x00c9_ce18,
                0x0057_52a0,
                0x00c9_8f60,
                0x00b1_b7cc,
                0xc,
                0x12,
                0x00af_d800,
            ],
        ),
    ),
    (
        0x004a_a210,
        (
            0x00c9_c4e8,
            [
                0x00c9_ce74,
                0x00c9_ce48,
                0x0057_52a0,
                0x00ca_37d0,
                0x00b1_b750,
                0xc,
                0x12,
                0x00af_d900,
            ],
        ),
    ),
    (
        0x004a_a2b0,
        (
            0x00c9_c580,
            [
                0x00c9_cea4,
                0x00c9_ce78,
                0x0057_52a0,
                0x00ca_3b60,
                0x00b1_b750,
                0xc,
                0x12,
                0x00af_d910,
            ],
        ),
    ),
    (
        0x004a_a350,
        (
            0x00c9_c0c0,
            [
                0x00c9_cc48,
                0x00c9_cc1c,
                0x0057_52a0,
                0x00ca_3530,
                0x00b1_b75c,
                0xc,
                0x12,
                0x00af_d960,
            ],
        ),
    ),
    (
        0x004a_a4f0,
        (
            0x00c9_bef8,
            [
                0x00c9_cbb8,
                0x00c9_cb8c,
                0x0057_52a0,
                0x00ca_37d0,
                0x00b1_b750,
                0xc,
                0x12,
                0x00af_d920,
            ],
        ),
    ),
    (
        0x004a_a590,
        (
            0x00c9_bf90,
            [
                0x00c9_cbe8,
                0x00c9_cbbc,
                0x0057_52a0,
                0x00ca_3b60,
                0x00b1_b750,
                0xc,
                0x12,
                0x00af_d930,
            ],
        ),
    ),
    (
        0x004a_a760,
        (
            0x00c9_c028,
            [
                0x00c9_cc18,
                0x00c9_cbec,
                0x0057_52a0,
                0x00ca_3ef0,
                0x00b1_b750,
                0xc,
                0x12,
                0x00af_d940,
            ],
        ),
    ),
    (
        0x004a_a800,
        (
            0x00c9_be60,
            [
                0x00c9_cb88,
                0x00c9_cb5c,
                0x0057_52a0,
                0x00c9_b560,
                0x00b1_b73c,
                0xc,
                0x12,
                0x00af_d750,
            ],
        ),
    ),
];

/// `Bind` copies with a member whose type comes from a `GetType` call:
/// `(d, [guard, member, constructor, type getter, name, size, kind,
/// destructor])`.
pub const BIND_MEMBER_TYPED: &[(u32, (u32, [u32; 8]))] = &[
    (
        0x0049_fe10,
        (
            0x00c9_b4c8,
            [
                0x00c9_b620,
                0x00c9_b5f4,
                0x0057_52a0,
                0x0043_7fc0,
                0x00b0_fb4c,
                0x0,
                0x10,
                0x00af_d270,
            ],
        ),
    ),
    (
        0x004b_6870,
        (
            0x00ca_1630,
            [
                0x00ca_18bc,
                0x00ca_1890,
                0x0057_52a0,
                0x0043_7fc0,
                0x00b1_ca40,
                0x0,
                0x10,
                0x00af_db50,
            ],
        ),
    ),
    (
        0x004a_1b40,
        (
            0x00c9_b720,
            [
                0x00c9_ba40,
                0x00c9_ba14,
                0x0057_52a0,
                0x0043_7e70,
                0x00b0_f994,
                0x0,
                0x0,
                0x00af_d380,
            ],
        ),
    ),
    (
        0x004d_65b0,
        (
            0x00ca_4fa8,
            [
                0x00ca_5104,
                0x00ca_50d8,
                0x0057_52a0,
                0x0043_7fc0,
                0x00b1_d350,
                0x0,
                0x0,
                0x00af_e910,
            ],
        ),
    ),
    (
        0x004a_1be0,
        (
            0x00c9_b7b8,
            [
                0x00c9_ba70,
                0x00c9_ba44,
                0x0057_52a0,
                0x0043_7e70,
                0x00b0_fb4c,
                0x2,
                0x0,
                0x00af_d3a0,
            ],
        ),
    ),
    (
        0x004a_1c80,
        (
            0x00c9_b850,
            [
                0x00c9_baa0,
                0x00c9_ba74,
                0x0057_52a0,
                0x0043_7d30,
                0x00b0_f98c,
                0x2,
                0x0,
                0x00af_d3c0,
            ],
        ),
    ),
];

/// The constants of [`BIND_MEMBER_TYPED`] copy `f` (compile time).
#[must_use]
pub const fn bind_typed_of(f: u32) -> (u32, [u32; 8]) {
    let mut i = 0;
    while i < BIND_MEMBER_TYPED.len() {
        if BIND_MEMBER_TYPED[i].0 == f {
            return BIND_MEMBER_TYPED[i].1;
        }
        i += 1;
    }
    panic!("not in the table");
}

/// A member descriptor: `[member, other, name, size, kind, destructor]`.
pub type Member = [u32; 6];

/// A two-member `Bind`'s constants: `(d, guard, first, second)`.
pub type TwoMembers = (u32, u32, Member, Member);

/// `Bind` copies with two members.
pub const BIND_TWO_MEMBERS: &[(u32, TwoMembers)] = &[
    (
        0x004a_9de0,
        (
            0x00c9_c288,
            0x00c9_cd30,
            [
                0x00c9_cd04,
                0x00ca_8dc8,
                0x00b1_b76c,
                0xc,
                0x12,
                0x00af_d8e0,
            ],
            [
                0x00c9_cd34,
                0x00c9_8948,
                0x00b1_b780,
                0x10,
                0x12,
                0x00af_d780,
            ],
        ),
    ),
    (
        0x004a_a130,
        (
            0x00c9_c1f0,
            0x00c9_ccd4,
            [
                0x00c9_cca8,
                0x00ca_8dc8,
                0x00b1_b78c,
                0xc,
                0x12,
                0x00af_d8b0,
            ],
            [
                0x00c9_ccd8,
                0x00c9_8948,
                0x00b1_b7a4,
                0x10,
                0x12,
                0x00af_d880,
            ],
        ),
    ),
    (
        0x004a_a410,
        (
            0x00c9_c158,
            0x00c9_cc78,
            [
                0x00c9_cc4c,
                0x00ca_8dc8,
                0x00b1_b76c,
                0xc,
                0x12,
                0x00af_d8f0,
            ],
            [
                0x00c9_cc7c,
                0x00c9_8948,
                0x00b1_b780,
                0x10,
                0x12,
                0x00af_d7a0,
            ],
        ),
    ),
];

/// The constants of [`BIND_TWO_MEMBERS`] copy `f` (compile time).
#[must_use]
pub const fn bind_two_of(f: u32) -> TwoMembers {
    let mut i = 0;
    while i < BIND_TWO_MEMBERS.len() {
        if BIND_TWO_MEMBERS[i].0 == f {
            return BIND_TWO_MEMBERS[i].1;
        }
        i += 1;
    }
    panic!("not in the table");
}

/// The constants of [`DESCRIPTOR_CTOR`] copy `f` (compile time).
#[must_use]
pub const fn five_of(table: &[(u32, [u32; 5])], f: u32) -> [u32; 5] {
    let mut i = 0;
    while i < table.len() {
        if table[i].0 == f {
            return table[i].1;
        }
        i += 1;
    }
    panic!("not in the table");
}

/// The constants of [`TYPE_SINGLETON`] copy `f` (compile time).
#[must_use]
pub const fn singleton_of(f: u32) -> [u32; 8] {
    let mut i = 0;
    while i < TYPE_SINGLETON.len() {
        if TYPE_SINGLETON[i].0 == f {
            return TYPE_SINGLETON[i].1;
        }
        i += 1;
    }
    panic!("not in the table");
}

/// The constants of [`BIND_MEMBER`] copy `f` (compile time).
#[must_use]
pub const fn bind_member_of(f: u32) -> (u32, [u32; 8]) {
    let mut i = 0;
    while i < BIND_MEMBER.len() {
        if BIND_MEMBER[i].0 == f {
            return BIND_MEMBER[i].1;
        }
        i += 1;
    }
    panic!("not in the table");
}

/// `PTypeDispenser::GetType<T>()` (cdecl) of `PArray<T>`, `0x0043_7fc0`'s
/// for `unsigned int`.
pub const TYPE_U32: u32 = 0x0043_7fc0;
/// `PClassDataMember::PClassDataMember` (thiscall, six words).
pub const DATA_MEMBER_CTOR: u32 = 0x0057_52a0;
/// `PAnnotation::PAnnotation` (thiscall, three words).
pub const ANNOTATION_CTOR: u32 = 0x0057_5840;
/// `PAnnotatable::addAnnotation` (thiscall, one word).
pub const ADD_ANNOTATION: u32 = 0x0057_5c10;
/// `PClassDataMemberArray::PClassDataMemberArray` (thiscall, seven words).
pub const DATA_MEMBER_ARRAY_CTOR: u32 = 0x0057_5ef0;

/// The constants of an array type's `GetType` (globals, functions and
/// strings of the image; [`bridge::array_type`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(missing_docs)] // the fields are the bridge's steps, documented there
pub struct ArrayType {
    pub guard: u32,
    pub obj: u32,
    pub desc_ctor: u32,
    pub vtable: u32,
    pub atexit1: u32,
    pub name_global: u32,
    pub member: u32,
    pub member_name: u32,
    pub atexit2: u32,
    pub annotation_arg: u32,
    pub annotation: u32,
    pub annotation_vtable: u32,
    pub atexit3: u32,
    pub array_member: u32,
    pub array_name: u32,
    pub array_other: u32,
    pub array_vtable: u32,
    pub count_fn: u32,
    pub set_fn: u32,
    pub atexit4: u32,
    /// The element size (the array member's last argument and `+0x2c`).
    pub elem: u32,
    /// The element type's getter (cdecl) when the type is not the
    /// constant [`ArrayType::array_other`], else 0.
    pub other_fn: u32,
}

/// The array types' `GetType` copies.
pub const ARRAY_TYPE: &[(u32, ArrayType)] = &[
    (
        0x0048_2170,
        ArrayType {
            guard: 0x00c9_8b2c,
            obj: 0x00c9_8a98,
            desc_ctor: 0x0048_1470,
            vtable: 0x00b1_43fc,
            atexit1: 0x00af_c550,
            name_global: 0x00c9_8ce0,
            member: 0x00c9_8b30,
            member_name: 0x00b0_fd34,
            atexit2: 0x00af_c570,
            annotation_arg: 0x00c9_07f4,
            annotation: 0x00c9_8b5c,
            annotation_vtable: 0x00b0_fd40,
            atexit3: 0x00af_c540,
            array_member: 0x00c9_8b78,
            array_name: 0x00b0_fd78,
            array_other: 0x00c9_8730,
            array_vtable: 0x00b1_4440,
            count_fn: 0x0043_20c0,
            set_fn: 0x0048_39d0,
            atexit4: 0x00af_c590,
            elem: 4,
            other_fn: 0,
        },
    ),
    (
        0x0049_0d70,
        ArrayType {
            guard: 0x00c9_a484,
            obj: 0x00c9_a3f0,
            desc_ctor: 0x0048_e920,
            vtable: 0x00b1_61f4,
            atexit1: 0x00af_ca20,
            name_global: 0x00c9_a638,
            member: 0x00c9_a488,
            member_name: 0x00b0_fd34,
            atexit2: 0x00af_cae0,
            annotation_arg: 0x00c9_07f4,
            annotation: 0x00c9_a4b4,
            annotation_vtable: 0x00b0_fd40,
            atexit3: 0x00af_c9b0,
            array_member: 0x00c9_a4d0,
            array_name: 0x00b0_fd78,
            array_other: 0x00c9_9330,
            array_vtable: 0x00b1_6238,
            count_fn: 0x0043_20c0,
            set_fn: 0x0049_54e0,
            atexit4: 0x00af_cb50,
            elem: 4,
            other_fn: 0,
        },
    ),
    (
        0x0049_0f80,
        ArrayType {
            guard: 0x00c9_a364,
            obj: 0x00c9_a2d0,
            desc_ctor: 0x0048_e9c0,
            vtable: 0x00b1_60ec,
            atexit1: 0x00af_ca40,
            name_global: 0x00c9_a634,
            member: 0x00c9_a368,
            member_name: 0x00b0_fd34,
            atexit2: 0x00af_caf0,
            annotation_arg: 0x00c9_07f4,
            annotation: 0x00c9_a394,
            annotation_vtable: 0x00b0_fd40,
            atexit3: 0x00af_c9c0,
            array_member: 0x00c9_a3b0,
            array_name: 0x00b0_fd78,
            array_other: 0x00c9_9be8,
            array_vtable: 0x00b1_6130,
            count_fn: 0x0043_20c0,
            set_fn: 0x0049_5530,
            atexit4: 0x00af_cb60,
            elem: 4,
            other_fn: 0,
        },
    ),
    (
        0x0049_1190,
        ArrayType {
            guard: 0x00c9_a124,
            obj: 0x00c9_a090,
            desc_ctor: 0x0048_ea60,
            vtable: 0x00b1_5edc,
            atexit1: 0x00af_ca60,
            name_global: 0x00c9_a62c,
            member: 0x00c9_a128,
            member_name: 0x00b0_fd34,
            atexit2: 0x00af_cb00,
            annotation_arg: 0x00c9_07f4,
            annotation: 0x00c9_a154,
            annotation_vtable: 0x00b0_fd40,
            atexit3: 0x00af_c9d0,
            array_member: 0x00c9_a170,
            array_name: 0x00b0_fd78,
            array_other: 0x00c9_1048,
            array_vtable: 0x00b1_5f20,
            count_fn: 0x0043_20c0,
            set_fn: 0x0049_5580,
            atexit4: 0x00af_cb70,
            elem: 4,
            other_fn: 0,
        },
    ),
    (
        0x0049_7260,
        ArrayType {
            guard: 0x00c9_aaa4,
            obj: 0x00c9_aa10,
            desc_ctor: 0x0049_6610,
            vtable: 0x00b1_6b44,
            atexit1: 0x00af_cc60,
            name_global: 0x00c9_ab60,
            member: 0x00c9_aaa8,
            member_name: 0x00b0_fd34,
            atexit2: 0x00af_cc80,
            annotation_arg: 0x00c9_07f4,
            annotation: 0x00c9_aad4,
            annotation_vtable: 0x00b0_fd40,
            atexit3: 0x00af_cc50,
            array_member: 0x00c9_aaf0,
            array_name: 0x00b0_fd78,
            array_other: 0x00c9_a738,
            array_vtable: 0x00b1_6b88,
            count_fn: 0x0043_20c0,
            set_fn: 0x0049_9220,
            atexit4: 0x00af_ccb0,
            elem: 4,
            other_fn: 0,
        },
    ),
    (
        0x004a_b0b0,
        ArrayType {
            guard: 0x00c9_d174,
            obj: 0x00c9_d0e0,
            desc_ctor: 0x004a_6730,
            vtable: 0x00b1_b99c,
            atexit1: 0x00af_d6e0,
            name_global: 0x00ca_12d4,
            member: 0x00c9_d178,
            member_name: 0x00b0_fd34,
            atexit2: 0x00af_d760,
            annotation_arg: 0x00c9_07f4,
            annotation: 0x00c9_d1a4,
            annotation_vtable: 0x00b0_fd40,
            atexit3: 0x00af_d6c0,
            array_member: 0x00c9_d1c0,
            array_name: 0x00b0_fd78,
            array_other: 0x00c9_b980,
            array_vtable: 0x00b1_b9e0,
            count_fn: 0x0043_20c0,
            set_fn: 0x004b_2660,
            atexit4: 0x00af_d7c0,
            elem: 4,
            other_fn: 0,
        },
    ),
    (
        0x004a_b2c0,
        ArrayType {
            guard: 0x00c9_d294,
            obj: 0x00c9_d200,
            desc_ctor: 0x004a_67d0,
            vtable: 0x00b1_baa4,
            atexit1: 0x00af_d700,
            name_global: 0x00ca_12d8,
            member: 0x00c9_d298,
            member_name: 0x00b0_fd34,
            atexit2: 0x00af_d770,
            annotation_arg: 0x00c9_07f4,
            annotation: 0x00c9_d2c4,
            annotation_vtable: 0x00b0_fd40,
            atexit3: 0x00af_d6d0,
            array_member: 0x00c9_d2e0,
            array_name: 0x00b0_fd78,
            array_other: 0x00c9_c7e0,
            array_vtable: 0x00b1_bae8,
            count_fn: 0x0043_20c0,
            set_fn: 0x004b_26b0,
            atexit4: 0x00af_d7d0,
            elem: 4,
            other_fn: 0,
        },
    ),
    (
        0x004b_6d30,
        ArrayType {
            guard: 0x00ca_1a74,
            obj: 0x00ca_19e0,
            desc_ctor: 0x004b_48d0,
            vtable: 0x00b1_cbf4,
            atexit1: 0x00af_daa0,
            name_global: 0x00ca_24b8,
            member: 0x00ca_1a78,
            member_name: 0x00b0_fd34,
            atexit2: 0x00af_daf0,
            annotation_arg: 0x00c9_07f4,
            annotation: 0x00ca_1aa4,
            annotation_vtable: 0x00b0_fd40,
            atexit3: 0x00af_da50,
            array_member: 0x00ca_1ac0,
            array_name: 0x00b0_fd78,
            array_other: 0x00c9_b8e8,
            array_vtable: 0x00b1_cc38,
            count_fn: 0x0043_20c0,
            set_fn: 0x004b_ab10,
            atexit4: 0x00af_db20,
            elem: 4,
            other_fn: 0,
        },
    ),
    (
        0x004b_c640,
        ArrayType {
            guard: 0x00ca_276c,
            obj: 0x00ca_26d8,
            desc_ctor: 0x004b_b9f0,
            vtable: 0x00b1_d424,
            atexit1: 0x00af_dbf0,
            name_global: 0x00ca_2860,
            member: 0x00ca_2770,
            member_name: 0x00b0_fd34,
            atexit2: 0x00af_dc10,
            annotation_arg: 0x00c9_07f4,
            annotation: 0x00ca_279c,
            annotation_vtable: 0x00b0_fd40,
            atexit3: 0x00af_dbe0,
            array_member: 0x00ca_27b8,
            array_name: 0x00b0_fd78,
            array_other: 0x00ca_1598,
            array_vtable: 0x00b1_d468,
            count_fn: 0x0043_20c0,
            set_fn: 0x004b_dde0,
            atexit4: 0x00af_dc20,
            elem: 4,
            other_fn: 0,
        },
    ),
    (
        0x004c_ae60,
        ArrayType {
            guard: 0x00ca_4714,
            obj: 0x00ca_4680,
            desc_ctor: 0x004c_9020,
            vtable: 0x00b1_f11c,
            atexit1: 0x00af_e5f0,
            name_global: 0x00ca_4b90,
            member: 0x00ca_4718,
            member_name: 0x00b0_fd34,
            atexit2: 0x00af_e620,
            annotation_arg: 0x00c9_07f4,
            annotation: 0x00ca_4744,
            annotation_vtable: 0x00b0_fd40,
            atexit3: 0x00af_e5e0,
            array_member: 0x00ca_4760,
            array_name: 0x00b0_fd78,
            array_other: 0x00ca_4430,
            array_vtable: 0x00b1_f160,
            count_fn: 0x0043_20c0,
            set_fn: 0x004c_d600,
            atexit4: 0x00af_e630,
            elem: 4,
            other_fn: 0,
        },
    ),
    (
        0x0048_7d80,
        ArrayType {
            guard: 0x00c9_9624,
            obj: 0x00c9_9590,
            desc_ctor: 0x0048_6c20,
            vtable: 0x00b1_5184,
            atexit1: 0x00af_c730,
            name_global: 0x00c9_98bc,
            member: 0x00c9_9628,
            member_name: 0x00b0_fd34,
            atexit2: 0x00af_c770,
            annotation_arg: 0x00c9_07f4,
            annotation: 0x00c9_9654,
            annotation_vtable: 0x00b0_fd40,
            atexit3: 0x00af_c720,
            array_member: 0x00c9_9670,
            array_name: 0x00b0_fd78,
            array_other: 0x00c9_9200,
            array_vtable: 0x00b1_51c8,
            count_fn: 0x0043_20c0,
            set_fn: 0x0048_9880,
            atexit4: 0x00af_c780,
            elem: 2,
            other_fn: 0,
        },
    ),
    (
        0x004b_6910,
        ArrayType {
            guard: 0x00ca_1b94,
            obj: 0x00ca_1b00,
            desc_ctor: 0x004b_4790,
            vtable: 0x00b1_ccfc,
            atexit1: 0x00af_da60,
            name_global: 0x00ca_24bc,
            member: 0x00ca_1b98,
            member_name: 0x00b0_fd34,
            atexit2: 0x00af_dad0,
            annotation_arg: 0x00c9_07f4,
            annotation: 0x00ca_1bc4,
            annotation_vtable: 0x00b0_fd40,
            atexit3: 0x00af_da30,
            array_member: 0x00ca_1be0,
            array_name: 0x00b0_fd78,
            array_other: 0x00c9_b720,
            array_vtable: 0x00b1_cd40,
            count_fn: 0x0043_20c0,
            set_fn: 0x004b_aa70,
            atexit4: 0x00af_db00,
            elem: 2,
            other_fn: 0,
        },
    ),
    (
        0x004b_6b20,
        ArrayType {
            guard: 0x00ca_1954,
            obj: 0x00ca_18c0,
            desc_ctor: 0x004b_4830,
            vtable: 0x00b1_caec,
            atexit1: 0x00af_da80,
            name_global: 0x00ca_24b4,
            member: 0x00ca_1958,
            member_name: 0x00b0_fd34,
            atexit2: 0x00af_dae0,
            annotation_arg: 0x00c9_07f4,
            annotation: 0x00ca_1984,
            annotation_vtable: 0x00b0_fd40,
            atexit3: 0x00af_da40,
            array_member: 0x00ca_19a0,
            array_name: 0x00b0_fd78,
            array_other: 0x00c9_b850,
            array_vtable: 0x00b1_cb30,
            count_fn: 0x0043_20c0,
            set_fn: 0x004b_aac0,
            atexit4: 0x00af_db10,
            elem: 2,
            other_fn: 0,
        },
    ),
    (
        0x0049_0950,
        ArrayType {
            guard: 0x00c9_a004,
            obj: 0x00c9_9f70,
            desc_ctor: 0x0048_e7e0,
            vtable: 0x00b1_5dd4,
            atexit1: 0x00af_c9e0,
            name_global: 0x00c9_a628,
            member: 0x00c9_a008,
            member_name: 0x00b0_fd34,
            atexit2: 0x00af_cac0,
            annotation_arg: 0x00c9_07f4,
            annotation: 0x00c9_a034,
            annotation_vtable: 0x00b0_fd40,
            atexit3: 0x00af_c990,
            array_member: 0x00c9_a050,
            array_name: 0x00b0_fd78,
            array_other: 0x0000_0000,
            array_vtable: 0x00b1_5e18,
            count_fn: 0x0043_20c0,
            set_fn: 0x0049_5440,
            atexit4: 0x00af_cb30,
            elem: 4,
            other_fn: 0x0043_7f10,
        },
    ),
    (
        0x0049_0b60,
        ArrayType {
            guard: 0x00c9_a244,
            obj: 0x00c9_a1b0,
            desc_ctor: 0x0048_e880,
            vtable: 0x00b1_5fe4,
            atexit1: 0x00af_ca00,
            name_global: 0x00c9_a630,
            member: 0x00c9_a248,
            member_name: 0x00b0_fd34,
            atexit2: 0x00af_cad0,
            annotation_arg: 0x00c9_07f4,
            annotation: 0x00c9_a274,
            annotation_vtable: 0x00b0_fd40,
            atexit3: 0x00af_c9a0,
            array_member: 0x00c9_a290,
            array_name: 0x00b0_fd78,
            array_other: 0x0000_0000,
            array_vtable: 0x00b1_6028,
            count_fn: 0x0043_20c0,
            set_fn: 0x0049_5490,
            atexit4: 0x00af_cb40,
            elem: 4,
            other_fn: 0x0045_f3f0,
        },
    ),
];

/// The constants of [`ARRAY_TYPE`] copy `f` (compile time).
#[must_use]
pub const fn array_type_of(f: u32) -> ArrayType {
    let mut i = 0;
    while i < ARRAY_TYPE.len() {
        if ARRAY_TYPE[i].0 == f {
            return ARRAY_TYPE[i].1;
        }
        i += 1;
    }
    panic!("not in the table");
}

pub mod bridge {
    //! The originals over game memory.

    use super::{
        ADD_ANNOTATION, ADD_TO_NAMESPACE, ANNOTATION_CTOR, ATEXIT, ArrayType, Conv,
        DATA_MEMBER_ARRAY_CTOR, DATA_MEMBER_CTOR, DESCRIPTOR_BASE_CTOR, GLOBAL_NAMESPACE, Host,
        Member, PBASE_DESCRIPTOR, REGISTER_TYPE, SET_DEFAULT_BUFFER, SET_VALIDATION_BUFFER,
        SET_WRITE_MASK_BUFFER, TYPE_U32,
    };

    /// A descriptor's constructor on `this`; `this`.
    pub fn descriptor_ctor(
        h: &mut impl Host,
        this: u32,
        [name, vtable, default, validation, mask]: [u32; 5],
    ) -> u32 {
        let parent = h.call_sym(PBASE_DESCRIPTOR, Conv::Cdecl, &[]);
        let name_global = h.rebase(name);
        let name: u32 = h.read(name_global);
        let ns = h.call_sym(GLOBAL_NAMESPACE, Conv::Cdecl, &[]);
        h.call_sym(
            DESCRIPTOR_BASE_CTOR,
            Conv::Thiscall(this),
            &[ns, name, 8, 4, parent, 0],
        );
        let flags: u32 = h.read(this + 0x90);
        h.write(this + 0x90, flags | 2);
        let vtable = h.rebase(vtable);
        h.write(this, vtable);
        for (f, buffer) in [
            (SET_DEFAULT_BUFFER, default),
            (SET_VALIDATION_BUFFER, validation),
            (SET_WRITE_MASK_BUFFER, mask),
        ] {
            let buffer = h.rebase(buffer);
            h.call_sym(f, Conv::Thiscall(this), &[buffer]);
        }
        this
    }

    /// Bit 0 of the guard word at `guard` (a global): set it and true when
    /// it was clear (the first call).
    fn first_call(h: &mut impl Host, guard: u32) -> bool {
        let at = h.rebase(guard);
        let flags: u32 = h.read(at);
        if flags & 1 != 0 {
            return false;
        }
        h.write(at, flags | 1);
        true
    }

    /// `GetType<T>()`: the type object, made on the first call.
    pub fn type_singleton(
        h: &mut impl Host,
        [guard, object, ctor, name, f1, f2, vtable, dtor]: [u32; 8],
    ) -> u32 {
        let obj = h.rebase(object);
        if first_call(h, guard) {
            let args = [h.rebase(name), 0x14, 4, h.rebase(f1), h.rebase(f2)];
            h.call_sym(ctor, Conv::Thiscall(obj), &args);
            let vtable = h.rebase(vtable);
            h.write(obj, vtable);
            let dtor = h.rebase(dtor);
            h.call_sym(ATEXIT, Conv::Cdecl, &[dtor]);
        }
        obj
    }

    /// `T::Bind()` with a member descriptor: `addToNamespace`'s `eax`.
    pub fn bind_member(
        h: &mut impl Host,
        d: u32,
        [guard, member, ctor, other, name, size, kind, dtor]: [u32; 8],
    ) -> u32 {
        let d = h.rebase(d);
        h.call_sym(REGISTER_TYPE, Conv::Cdecl, &[d]);
        if first_call(h, guard) {
            let member = h.rebase(member);
            let args = [d, h.rebase(other), h.rebase(name), size, kind, 0];
            h.call_sym(ctor, Conv::Thiscall(member), &args);
            let dtor = h.rebase(dtor);
            h.call_sym(ATEXIT, Conv::Cdecl, &[dtor]);
        }
        h.call_sym(ADD_TO_NAMESPACE, Conv::Thiscall(d), &[])
    }

    /// Registers `f` (an image address) with `atexit`.
    fn at_exit(h: &mut impl Host, f: u32) {
        let f = h.rebase(f);
        h.call_sym(ATEXIT, Conv::Cdecl, &[f]);
    }

    /// `GetType<PArray<T>>(name)` -> the descriptor `k.obj`, built in
    /// stages, each once (bits 0, 1, 2, 3 of `k.guard`): the descriptor
    /// (its constructor, flags `+0x90` bit 1 cleared, its vtable); then,
    /// given a `name` while the descriptor has none (`+0x18`): the name
    /// (the class's name global first, set to `name` when empty), the data
    /// member (`GetType<unsigned int>()`), its annotation (vtable, `+0x18`
    /// 0x7fffffff), added to the member when the annotation's list is
    /// empty (`+4` pointing at itself), the array member (vtable, getCount
    /// and setToExternalBuffer at `+0x34`, `+0x3c`); then the array
    /// member's `+0x2c` = 4, `RegisterType` and `addToNamespace`.
    pub fn array_type(h: &mut impl Host, k: &ArrayType, name: u32) -> u32 {
        let guard = h.rebase(k.guard);
        let obj = h.rebase(k.obj);
        let mut flags: u32 = h.read(guard);
        if flags & 1 == 0 {
            h.write(guard, flags | 1);
            h.call_sym(k.desc_ctor, Conv::Thiscall(obj), &[]);
            let f: u32 = h.read(obj + 0x90);
            h.write(obj + 0x90, f & !2);
            let vtable = h.rebase(k.vtable);
            h.write(obj, vtable);
            at_exit(h, k.atexit1);
            flags = h.read(guard);
        }
        if name == 0 || h.read::<u32>(obj + 0x18) != 0 {
            return obj;
        }
        let name_global = h.rebase(k.name_global);
        let mut class_name: u32 = h.read(name_global);
        if class_name == 0 {
            class_name = name;
            h.write(name_global, name);
        }
        h.write(obj + 0x18, class_name);
        let member = h.rebase(k.member);
        if flags & 2 == 0 {
            h.write(guard, flags | 2);
            let u32_type = h.call_sym(TYPE_U32, Conv::Cdecl, &[]);
            let member_name = h.rebase(k.member_name);
            h.call_sym(
                DATA_MEMBER_CTOR,
                Conv::Thiscall(member),
                &[obj, u32_type, member_name, 0, 0, 0],
            );
            at_exit(h, k.atexit2);
            flags = h.read(guard);
        }
        let annotation = h.rebase(k.annotation);
        if flags & 4 == 0 {
            h.write(guard, flags | 4);
            let u32_type = h.call_sym(TYPE_U32, Conv::Cdecl, &[]);
            let arg = h.rebase(k.annotation_arg);
            h.call_sym(
                ANNOTATION_CTOR,
                Conv::Thiscall(annotation),
                &[arg, u32_type, 0],
            );
            let vtable = h.rebase(k.annotation_vtable);
            h.write(annotation, vtable);
            h.write(annotation + 0x18, 0x7fff_ffffu32);
            at_exit(h, k.atexit3);
            flags = h.read(guard);
        }
        if h.read::<u32>(annotation + 4) == annotation + 4 {
            h.call_sym(ADD_ANNOTATION, Conv::Thiscall(member + 0x1c), &[annotation]);
            flags = h.read(guard);
        }
        let array = h.rebase(k.array_member);
        if flags & 8 == 0 {
            h.write(guard, flags | 8);
            let other = if k.other_fn == 0 {
                h.rebase(k.array_other)
            } else {
                h.call_sym(k.other_fn, Conv::Cdecl, &[])
            };
            let args = [obj, other, h.rebase(k.array_name), 4, 0, 2, k.elem];
            h.call_sym(DATA_MEMBER_ARRAY_CTOR, Conv::Thiscall(array), &args);
            let vtable = h.rebase(k.array_vtable);
            h.write(array, vtable);
            let count = h.rebase(k.count_fn);
            h.write(array + 0x34, count);
            h.write(array + 0x38, 0u32);
            let set = h.rebase(k.set_fn);
            h.write(array + 0x3c, set);
            at_exit(h, k.atexit4);
        }
        h.write(array + 0x2c, k.elem);
        h.call_sym(REGISTER_TYPE, Conv::Cdecl, &[obj]);
        h.call_sym(ADD_TO_NAMESPACE, Conv::Thiscall(obj), &[]);
        obj
    }

    /// `T::Bind()` with a member whose type is `type_of()` (cdecl):
    /// `addToNamespace`'s `eax`.
    pub fn bind_member_typed(
        h: &mut impl Host,
        d: u32,
        [guard, member, ctor, type_of, name, size, kind, dtor]: [u32; 8],
    ) -> u32 {
        let d = h.rebase(d);
        h.call_sym(REGISTER_TYPE, Conv::Cdecl, &[d]);
        if first_call(h, guard) {
            let t = h.call_sym(type_of, Conv::Cdecl, &[]);
            let member = h.rebase(member);
            let args = [d, t, h.rebase(name), size, kind, 0];
            h.call_sym(ctor, Conv::Thiscall(member), &args);
            let dtor = h.rebase(dtor);
            h.call_sym(ATEXIT, Conv::Cdecl, &[dtor]);
        }
        h.call_sym(ADD_TO_NAMESPACE, Conv::Thiscall(d), &[])
    }

    /// One member descriptor: its constructor (`PClassDataMember`, on
    /// `member`: `d`, `other`, `name`, `size`, `kind`, 0) and `atexit`.
    fn make_member(h: &mut impl Host, d: u32, [member, other, name, size, kind, dtor]: Member) {
        let member = h.rebase(member);
        let args = [d, h.rebase(other), h.rebase(name), size, kind, 0];
        h.call_sym(DATA_MEMBER_CTOR, Conv::Thiscall(member), &args);
        let dtor = h.rebase(dtor);
        h.call_sym(ATEXIT, Conv::Cdecl, &[dtor]);
    }

    /// `T::Bind()` with two members (guard bits 0 and 1 of `guard`):
    /// `addToNamespace`'s `eax`.
    pub fn bind_two_members(
        h: &mut impl Host,
        (d, guard, first, second): (u32, u32, Member, Member),
    ) -> u32 {
        let d = h.rebase(d);
        h.call_sym(REGISTER_TYPE, Conv::Cdecl, &[d]);
        let at = h.rebase(guard);
        let mut flags: u32 = h.read(at);
        if flags & 1 == 0 {
            h.write(at, flags | 1);
            make_member(h, d, first);
            flags = h.read(at);
        }
        if flags & 2 == 0 {
            h.write(at, flags | 2);
            make_member(h, d, second);
        }
        h.call_sym(ADD_TO_NAMESPACE, Conv::Thiscall(d), &[])
    }
}
