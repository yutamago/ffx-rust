//! `PhyreEngine`'s reflection registrations the input and game-settings
//! classes run at start-up (`Phyre::PInputs::*::Bind`,
//! `PAsyncProcessHeader::Bind`, `PGameSettings::Bind`), the `PArray<T*>`
//! class descriptors they declare, and those descriptors' constructors.
//!
//! Each `Bind` registers its class descriptor, builds its member
//! descriptors once (a guard word's bits; the `PClassDataMember` /
//! `PClassDataMemberArray` / `PClassMember` constructors, each with its
//! `atexit` destructor), declares the element classes it needs, and adds
//! the class to its namespace. The tables below are each function's
//! constants, in its order; addresses are `ffx.sym` addresses.

use ffx_graphic::host::{Conv, Host};

/// `Phyre_PTypeDispenser_RegisterType` (cdecl `(desc)`).
pub const REGISTER_TYPE: u32 = 0x0043_95b0;
/// `Phyre_PClassDescriptor_addToNamespace` (thiscall `(desc)`).
pub const ADD_TO_NAMESPACE: u32 = 0x0043_c140;
/// `Phyre_PClassDataMember_PClassDataMember` (thiscall `(member; desc,
/// type, name, offset, kind, 0)`, `ret 0x18`).
pub const DATA_MEMBER_CTOR: u32 = 0x0057_52a0;
/// `Phyre_PClassDataMemberArray_PClassDataMemberArray` (thiscall
/// `(member; desc, element desc, name, offset, a, b, c)`, `ret 0x1c`).
pub const ARRAY_MEMBER_CTOR: u32 = 0x0057_5ef0;
/// `Phyre_PClassMember_PClassMember` (thiscall `(member; desc, name, 0)`,
/// `ret 0xc`).
pub const METHOD_MEMBER_CTOR: u32 = 0x0043_a080;
/// `Phyre_PClassDescriptor_findMember_004359d0` (thiscall `(class,
/// name)`).
pub const FIND_MEMBER: u32 = 0x0043_59d0;
/// `atexit` (cdecl `(fn)`).
pub const ATEXIT: u32 = 0x0094_9207;
/// `Phyre_PBase_GetClassDescriptor` (cdecl, no arguments).
pub const BASE_CLASS_DESCRIPTOR: u32 = 0x0043_5740;
/// `Phyre_PNamespace_GetGlobalNamespace` (cdecl).
pub const GLOBAL_NAMESPACE: u32 = 0x0043_e2f0;
/// `Phyre_PClassDescriptor_PClassDescriptor_0043b0a0` (thiscall `(desc;
/// namespace, name, 8, 4, base, 0)`, `ret 0x18`).
pub const CLASS_DESCRIPTOR_CTOR: u32 = 0x0043_b0a0;
/// `Phyre_PClassDescriptor_setDefaultBuffer` (thiscall `(desc, buf)`).
pub const SET_DEFAULT_BUFFER: u32 = 0x0043_d510;
/// `Phyre_PClassDescriptor_setValidationBuffer` (thiscall `(desc, buf)`).
pub const SET_VALIDATION_BUFFER: u32 = 0x0043_d5b0;
/// `Phyre_PClassDescriptor_setWriteMaskBuffer` (thiscall `(desc, buf)`).
pub const SET_WRITE_MASK_BUFFER: u32 = 0x0043_d5c0;
/// `Phyre_PTypeDispenser_GetType_unsigned_int` (cdecl, no arguments).
pub const TYPE_UINT: u32 = 0x0043_7fc0;
/// `Phyre_PAnnotation_PAnnotation` (thiscall `(annot; name, type, 0)`,
/// `ret 0xc`).
pub const ANNOTATION_CTOR: u32 = 0x0057_5840;
/// `Phyre_PAnnotatable_addAnnotation` (thiscall `(annotatable, annot)`).
pub const ADD_ANNOTATION: u32 = 0x0057_5c10;

/// Where a data member's type comes from.
#[derive(Debug, Clone, Copy)]
pub enum TypeOf {
    /// A `PTypeDispenser::GetType<T>` (cdecl, no arguments).
    Get(u32),
    /// A class descriptor at this address.
    Desc(u32),
}

/// A word a registration stores: an address (rebased) or a plain value.
#[derive(Debug, Clone, Copy)]
pub enum Val {
    /// An `ffx.sym` address.
    Addr(u32),
    /// A plain word.
    Word(u32),
}

/// An element class descriptor built once: the guard is its `+0x94`
/// word's bit 0; its constructor (thiscall `(obj)`), then `+0x90 &= ~2`,
/// its vtable, and `atexit(dtor)`.
#[derive(Debug, Clone, Copy)]
pub struct Inner {
    pub obj: u32,
    pub ctor: u32,
    pub vtable: u32,
    pub dtor: u32,
}

/// One step of a `Bind`.
#[derive(Debug, Clone, Copy)]
pub enum Step {
    /// A data member, built when its guard bit is clear.
    Field {
        bit: u32,
        member: u32,
        ty: TypeOf,
        name: u32,
        offset: u32,
        kind: u32,
        dtor: u32,
    },
    /// An array member: its element descriptor ([`Inner`]), the member,
    /// its vtable, `atexit(dtor)`.
    Array {
        bit: u32,
        inner: Inner,
        member: u32,
        name: u32,
        offset: u32,
        abc: [u32; 3],
        vtable: u32,
        dtor: u32,
    },
    /// A method member: built and linked into its class's member list
    /// (`+0x4c`) unless the class has the name already; then, every time,
    /// its caller's words (`caller`, in order).
    Method {
        bit: u32,
        member: u32,
        name: u32,
        dtor: u32,
        caller: &'static [(u32, Val)],
    },
    /// A class declaration called with a global's value (cdecl).
    Call { f: u32, arg: u32 },
}

/// A `Bind`: its class descriptor, its guard word, its steps.
#[derive(Debug, Clone, Copy)]
pub struct Bind {
    pub desc: u32,
    pub guard: u32,
    pub steps: &'static [Step],
}

/// `Phyre::PInputs::PInputAction::Bind` (`0x009e4430`).
pub const INPUT_ACTION: Bind = Bind {
    desc: 0x0194_1878,
    guard: 0x0194_1c54,
    steps: &[
        Step::Field {
            bit: 1,
            member: 0x0194_1c28,
            ty: TypeOf::Desc(0x00c9_1048),
            name: 0x00b1_0dc0,
            offset: 4,
            kind: 0,
            dtor: 0x00b0_82c0,
        },
        Step::Call {
            f: 0x009e_5220,
            arg: 0x00c6_762c,
        },
        Step::Array {
            bit: 2,
            inner: Inner {
                obj: 0x0194_1d78,
                ctor: 0x009d_fbd0,
                vtable: 0x00b7_3984,
                dtor: 0x00b0_8140,
            },
            member: 0x0194_1c58,
            name: 0x00b7_3868,
            offset: 8,
            abc: [0, 3, 4],
            vtable: 0x00b7_383c,
            dtor: 0x00b0_8240,
        },
        Step::Method {
            bit: 4,
            member: 0x0194_1c8c,
            name: 0x00b7_3878,
            dtor: 0x00b0_8160,
            caller: &[
                (0x0194_1ca4, Val::Addr(0x00b7_39f8)),
                (0x0194_1cac, Val::Addr(0x009e_ca80)),
                (0x0194_1cb0, Val::Word(0)),
                (0x0194_1ca0, Val::Addr(0x0194_1ca4)),
            ],
        },
        Step::Method {
            bit: 8,
            member: 0x0194_1cb4,
            name: 0x00b7_3884,
            dtor: 0x00b0_81a0,
            caller: &[
                (0x0194_1ccc, Val::Addr(0x00b7_3a10)),
                (0x0194_1cd4, Val::Addr(0x009e_caa0)),
                (0x0194_1cd8, Val::Word(0)),
                (0x0194_1cc8, Val::Addr(0x0194_1ccc)),
            ],
        },
    ],
};

/// `Phyre::PInputs::PInputMap::Bind` (`0x009e46d0`).
pub const INPUT_MAP: Bind = Bind {
    desc: 0x0194_1910,
    guard: 0x0194_1d10,
    steps: &[
        Step::Call {
            f: 0x009e_5010,
            arg: 0x00c6_7630,
        },
        Step::Array {
            bit: 1,
            inner: Inner {
                obj: 0x0194_1e98,
                ctor: 0x009d_fb30,
                vtable: 0x00b7_3abc,
                dtor: 0x00b0_8120,
            },
            member: 0x0194_1cdc,
            name: 0x00b7_38dc,
            offset: 0,
            abc: [0, 3, 4],
            vtable: 0x00b7_38b0,
            dtor: 0x00b0_8230,
        },
    ],
};

/// `Phyre::PInputs::PInputSource::Bind` (`0x009e47c0`).
pub const INPUT_SOURCE: Bind = Bind {
    desc: 0x0194_0ad0,
    guard: 0x0194_19d0,
    steps: &[
        Step::Field {
            bit: 1,
            member: 0x0194_19a4,
            ty: TypeOf::Get(0x009f_4730),
            name: 0x00b7_3738,
            offset: 8,
            kind: 2,
            dtor: 0x00b0_8320,
        },
        Step::Field {
            bit: 2,
            member: 0x0194_19d4,
            ty: TypeOf::Get(0x0043_7fc0),
            name: 0x00b1_1c88,
            offset: 0xc,
            kind: 0,
            dtor: 0x00b0_8220,
        },
        Step::Field {
            bit: 4,
            member: 0x0194_1a00,
            ty: TypeOf::Get(0x0043_8390),
            name: 0x00b7_3748,
            offset: 0x10,
            kind: 0,
            dtor: 0x00b0_8250,
        },
        Step::Field {
            bit: 8,
            member: 0x0194_1a2c,
            ty: TypeOf::Get(0x0043_8060),
            name: 0x00b2_87d4,
            offset: 0x14,
            kind: 0,
            dtor: 0x00b0_8310,
        },
    ],
};

/// `Phyre::PInputs::PInputSourceJoypadAxis::Bind` (`0x009e4940`).
pub const JOYPAD_AXIS: Bind = Bind {
    desc: 0x0194_0c98,
    guard: 0x0194_1b3c,
    steps: &[Step::Field {
        bit: 1,
        member: 0x0194_1b10,
        ty: TypeOf::Get(0x00a2_dce0),
        name: 0x00b7_37bc,
        offset: 0x1c,
        kind: 2,
        dtor: 0x00b0_8260,
    }],
};

/// `Phyre::PInputs::PInputSourceJoypadButton::Bind` (`0x009e49e0`).
pub const JOYPAD_BUTTON: Bind = Bind {
    desc: 0x0194_0c00,
    guard: 0x0194_1ae0,
    steps: &[
        Step::Field {
            bit: 1,
            member: 0x0194_1ab4,
            ty: TypeOf::Get(0x00a2_e150),
            name: 0x00b7_3784,
            offset: 0x20,
            kind: 2,
            dtor: 0x00b0_82f0,
        },
        Step::Field {
            bit: 2,
            member: 0x0194_1ae4,
            ty: TypeOf::Get(0x00a2_e150),
            name: 0x00b7_37a0,
            offset: 0x24,
            kind: 2,
            dtor: 0x00b0_82d0,
        },
    ],
};

/// `Phyre::PInputs::PInputSourceKey::Bind` (`0x009e4ac0`).
pub const SOURCE_KEY: Bind = Bind {
    desc: 0x0194_0b68,
    guard: 0x0194_1a84,
    steps: &[
        Step::Field {
            bit: 1,
            member: 0x0194_1a58,
            ty: TypeOf::Get(0x00a2_e5c0),
            name: 0x00b7_3754,
            offset: 0x20,
            kind: 2,
            dtor: 0x00b0_8300,
        },
        Step::Field {
            bit: 2,
            member: 0x0194_1a88,
            ty: TypeOf::Get(0x00a2_e5c0),
            name: 0x00b7_376c,
            offset: 0x24,
            kind: 2,
            dtor: 0x00b0_82e0,
        },
    ],
};

/// `Phyre::PInputs::PInputSourceMouseButton::Bind` (`0x009e4ce0`).
pub const MOUSE_BUTTON: Bind = Bind {
    desc: 0x0194_0d30,
    guard: 0x0194_1b6c,
    steps: &[Step::Field {
        bit: 1,
        member: 0x0194_1b40,
        ty: TypeOf::Get(0x00a2_ea30),
        name: 0x00b7_37d4,
        offset: 0x1c,
        kind: 2,
        dtor: 0x00b0_82b0,
    }],
};

/// `0x009e4d80`: a `PInputSource` subclass binding a mouse-button and a
/// key semantic (`+0x20`, `+0x24`).
pub const MOUSE_BUTTON_KEY: Bind = Bind {
    desc: 0x0194_0dc8,
    guard: 0x0194_1b9c,
    steps: &[
        Step::Field {
            bit: 1,
            member: 0x0194_1b70,
            ty: TypeOf::Get(0x00a2_ea30),
            name: 0x00b7_37ec,
            offset: 0x20,
            kind: 2,
            dtor: 0x00b0_8270,
        },
        Step::Field {
            bit: 2,
            member: 0x0194_1ba0,
            ty: TypeOf::Get(0x00a2_e5c0),
            name: 0x00b7_3808,
            offset: 0x24,
            kind: 2,
            dtor: 0x00b0_8290,
        },
    ],
};

/// `0x009e4e60`: the same members as [`MOUSE_BUTTON_KEY`] for another
/// class.
pub const MOUSE_BUTTON_KEY_2: Bind = Bind {
    desc: 0x0194_0e60,
    guard: 0x0194_1bf8,
    steps: &[
        Step::Field {
            bit: 1,
            member: 0x0194_1bcc,
            ty: TypeOf::Get(0x00a2_ea30),
            name: 0x00b7_37ec,
            offset: 0x20,
            kind: 2,
            dtor: 0x00b0_8280,
        },
        Step::Field {
            bit: 2,
            member: 0x0194_1bfc,
            ty: TypeOf::Get(0x00a2_e5c0),
            name: 0x00b7_3808,
            offset: 0x24,
            kind: 2,
            dtor: 0x00b0_82a0,
        },
    ],
};

/// `Phyre::PGame::PGameSettings::Bind` (`0x00a33540`).
pub const GAME_SETTINGS: Bind = Bind {
    desc: 0x01a8_55b8,
    guard: 0x01a8_5678,
    steps: &[
        Step::Field {
            bit: 1,
            member: 0x01a8_564c,
            ty: TypeOf::Desc(0x0194_1910),
            name: 0x00b4_0c30,
            offset: 8,
            kind: 2,
            dtor: 0x00b0_b8a0,
        },
        Step::Call {
            f: 0x00a3_3680,
            arg: 0x00c6_d25c,
        },
        Step::Array {
            bit: 2,
            inner: Inner {
                obj: 0x01a8_56b0,
                ctor: 0x00a3_2ef0,
                vtable: 0x00b7_a294,
                dtor: 0x00b0_b860,
            },
            member: 0x01a8_567c,
            name: 0x00b7_a1f4,
            offset: 0,
            abc: [0, 3, 4],
            vtable: 0x00b7_a1c8,
            dtor: 0x00b0_b8b0,
        },
    ],
};

/// `Phyre::PScripting::PAsyncProcessHeader::Bind` (`0x00617960`).
pub const ASYNC_PROCESS_HEADER: Bind = Bind {
    desc: 0x00cc_9910,
    guard: 0x00cc_99cc,
    steps: &[Step::Method {
        bit: 1,
        member: 0x00cc_99a4,
        name: 0x00b3_f34c,
        dtor: 0x00b0_5fc0,
        caller: &[
            (0x00cc_99bc, Val::Addr(0x00b3_f360)),
            (0x00cc_99c0, Val::Addr(0x0061_7e40)),
            (0x00cc_99b8, Val::Addr(0x00cc_99bc)),
        ],
    }],
};

/// A `PArray<T*>` class descriptor's declaration (`0x009e5010`,
/// `0x009e5220`): the constants that differ; every other word sits at a
/// fixed offset from `desc`.
#[derive(Debug, Clone, Copy)]
pub struct ArrayDecl {
    pub inner: Inner,
    /// The global holding the first declared name.
    pub name_global: u32,
    pub field_dtor: u32,
    pub annotation_dtor: u32,
    /// The element type's class descriptor.
    pub element: u32,
    pub array_vtable: u32,
    pub array_fn: u32,
    pub array_dtor: u32,
}

/// `0x009e5010`: `PArray<PInputAction*>`'s descriptor.
pub const ARRAY_INPUT_ACTION: ArrayDecl = ArrayDecl {
    inner: Inner {
        obj: 0x0194_1e98,
        ctor: 0x009d_fb30,
        vtable: 0x00b7_3abc,
        dtor: 0x00b0_8120,
    },
    name_global: 0x0194_293c,
    field_dtor: 0x00b0_81e0,
    annotation_dtor: 0x00b0_8100,
    element: 0x0194_1878,
    array_vtable: 0x00b7_3b00,
    array_fn: 0x009e_f5b0,
    array_dtor: 0x00b0_8200,
};

/// `0x009e5220`: `PArray<PInputSource*>`'s descriptor.
pub const ARRAY_INPUT_SOURCE: ArrayDecl = ArrayDecl {
    inner: Inner {
        obj: 0x0194_1d78,
        ctor: 0x009d_fbd0,
        vtable: 0x00b7_3984,
        dtor: 0x00b0_8140,
    },
    name_global: 0x0194_2938,
    field_dtor: 0x00b0_81f0,
    annotation_dtor: 0x00b0_8110,
    element: 0x0194_0ad0,
    array_vtable: 0x00b7_39c8,
    array_fn: 0x009e_f600,
    array_dtor: 0x00b0_8210,
};

/// The constants of a `PClassDescriptorAbstract<PArray<T*>>` constructor
/// (`0x009dfbd0`, `0x00a32ef0`).
#[derive(Debug, Clone, Copy)]
pub struct DescriptorCtor {
    /// The global holding the class name.
    pub name_global: u32,
    pub vtable: u32,
    /// The default, validation and write-mask buffers.
    pub buffers: [u32; 3],
}

/// `0x009dfbd0`: `PArray<PInputSource*>`.
pub const CTOR_INPUT_SOURCE_ARRAY: DescriptorCtor = DescriptorCtor {
    name_global: 0x0194_2938,
    vtable: 0x00b7_393c,
    buffers: [0x0194_2940, 0x0194_2950, 0x0194_2960],
};

/// `0x00a32ef0`: `PArray<PInputMap*>`.
pub const CTOR_INPUT_MAP_ARRAY: DescriptorCtor = DescriptorCtor {
    name_global: 0x01a8_57fc,
    vtable: 0x00b7_a24c,
    buffers: [0x01a8_5800, 0x01a8_5810, 0x01a8_5820],
};

pub mod bridge {
    //! The originals over game memory.

    use super::{
        ADD_ANNOTATION, ADD_TO_NAMESPACE, ANNOTATION_CTOR, ARRAY_MEMBER_CTOR, ATEXIT, ArrayDecl,
        BASE_CLASS_DESCRIPTOR, Bind, CLASS_DESCRIPTOR_CTOR, Conv, DATA_MEMBER_CTOR, DescriptorCtor,
        FIND_MEMBER, GLOBAL_NAMESPACE, Host, Inner, METHOD_MEMBER_CTOR, REGISTER_TYPE,
        SET_DEFAULT_BUFFER, SET_VALIDATION_BUFFER, SET_WRITE_MASK_BUFFER, Step, TYPE_UINT, TypeOf,
        Val,
    };

    fn atexit(h: &mut impl Host, dtor: u32) {
        let f = h.rebase(ATEXIT);
        let d = h.rebase(dtor);
        h.call(f, Conv::Cdecl, &[d]);
    }

    /// Sets `bit` of the guard word at `guard` (rebased) and says whether
    /// it was clear.
    fn first_time(h: &mut impl Host, guard: u32, bit: u32) -> bool {
        let g: u32 = h.read(guard);
        if g & bit != 0 {
            return false;
        }
        h.write(guard, g | bit);
        true
    }

    /// [`Inner`]'s one-time construction.
    fn inner(h: &mut impl Host, i: Inner) {
        let obj = h.rebase(i.obj);
        if !first_time(h, obj + 0x94, 1) {
            return;
        }
        let f = h.rebase(i.ctor);
        h.call(f, Conv::Thiscall(obj), &[]);
        let w: u32 = h.read(obj + 0x90);
        h.write(obj + 0x90, w & !2);
        let vt = h.rebase(i.vtable);
        h.write(obj, vt);
        atexit(h, i.dtor);
    }

    /// A data member: `PClassDataMember(member; desc, type, name, offset,
    /// kind, 0)`, `atexit(dtor)`.
    fn data_member(
        h: &mut impl Host,
        desc: u32,
        member: u32,
        ty: TypeOf,
        name_off_kind: [u32; 3],
        dtor: u32,
    ) {
        let t = match ty {
            TypeOf::Get(f) => {
                let f = h.rebase(f);
                h.call(f, Conv::Cdecl, &[])
            }
            TypeOf::Desc(d) => h.rebase(d),
        };
        let [name, offset, kind] = name_off_kind;
        let name = h.rebase(name);
        let m = h.rebase(member);
        let f = h.rebase(DATA_MEMBER_CTOR);
        h.call(f, Conv::Thiscall(m), &[desc, t, name, offset, kind, 0]);
        atexit(h, dtor);
    }

    /// The `Bind` of `b` -> `addToNamespace`'s `eax`.
    pub fn bind(h: &mut impl Host, b: &Bind) -> u32 {
        let desc = h.rebase(b.desc);
        let guard = h.rebase(b.guard);
        let f = h.rebase(REGISTER_TYPE);
        h.call(f, Conv::Cdecl, &[desc]);
        for step in b.steps {
            match *step {
                Step::Field {
                    bit,
                    member,
                    ty,
                    name,
                    offset,
                    kind,
                    dtor,
                } => {
                    if first_time(h, guard, bit) {
                        data_member(h, desc, member, ty, [name, offset, kind], dtor);
                    }
                }
                Step::Array {
                    bit,
                    inner: i,
                    member,
                    name,
                    offset,
                    abc,
                    vtable,
                    dtor,
                } => {
                    if first_time(h, guard, bit) {
                        inner(h, i);
                        let obj = h.rebase(i.obj);
                        let name = h.rebase(name);
                        let m = h.rebase(member);
                        let f = h.rebase(ARRAY_MEMBER_CTOR);
                        let [a, b2, c] = abc;
                        h.call(f, Conv::Thiscall(m), &[desc, obj, name, offset, a, b2, c]);
                        let vt = h.rebase(vtable);
                        h.write(m, vt);
                        atexit(h, dtor);
                    }
                }
                Step::Method {
                    bit,
                    member,
                    name,
                    dtor,
                    caller,
                } => {
                    let m = h.rebase(member);
                    if first_time(h, guard, bit) {
                        let name = h.rebase(name);
                        let f = h.rebase(METHOD_MEMBER_CTOR);
                        h.call(f, Conv::Thiscall(m), &[desc, name, 0]);
                        method_link(h, m);
                        atexit(h, dtor);
                    }
                    for &(at, v) in caller {
                        let at = h.rebase(at);
                        let v = match v {
                            Val::Addr(a) => h.rebase(a),
                            Val::Word(w) => w,
                        };
                        h.write(at, v);
                    }
                }
                Step::Call { f: declare, arg } => {
                    let v: u32 = h.read(h.rebase(arg));
                    let declare = h.rebase(declare);
                    h.call(declare, Conv::Cdecl, &[v]);
                }
            }
        }
        let f = h.rebase(ADD_TO_NAMESPACE);
        h.call(f, Conv::Thiscall(desc), &[])
    }

    /// A new method member: its caller word `+0x14` cleared; linked at the
    /// tail of its class's (`+8`) member list `+0x4c` when it has a name
    /// (`+0xc`) the class doesn't hold yet.
    fn method_link(h: &mut impl Host, m: u32) {
        let class: u32 = h.read(m + 8);
        let name: u32 = h.read(m + 0xc);
        h.write(m + 0x14, 0u32);
        let list = class.wrapping_add(0x4c);
        if name == 0 {
            return;
        }
        let f = h.rebase(FIND_MEMBER);
        if h.call(f, Conv::Thiscall(class), &[name]) != 0 {
            return;
        }
        let last: u32 = h.read(list + 4);
        h.write(m, list);
        h.write(m + 4, last);
        h.write(last, m);
        h.write(list + 4, m);
    }

    /// A `PArray<T*>` declaration `(name)` (`0x009e5010`, `0x009e5220`,
    /// cdecl) -> the descriptor: its one-time construction; with a name
    /// and no name yet (`+0x18`), the name (the first declared one,
    /// `name_global`, else this one), its count member (`+0x98`), the
    /// count's annotation (`+0xc4`, added to the member's `+0x1c` list when
    /// unlinked), its array member (`+0xe0`), registration and namespace.
    pub fn array_decl(h: &mut impl Host, d: &ArrayDecl, name: u32) -> u32 {
        inner(h, d.inner);
        let desc = h.rebase(d.inner.obj);
        let guard = desc + 0x94;
        if name == 0 || h.read::<u32>(desc + 0x18) != 0 {
            return desc;
        }
        let g = h.rebase(d.name_global);
        let mut first: u32 = h.read(g);
        if first == 0 {
            first = name;
            h.write(g, first);
        }
        h.write(desc + 0x18, first);
        if first_time(h, guard, 2) {
            data_member(
                h,
                desc,
                d.inner.obj + 0x98,
                TypeOf::Get(TYPE_UINT),
                [0x00b0_fd34, 0, 0],
                d.field_dtor,
            );
        }
        let annot = desc + 0xc4;
        if first_time(h, guard, 4) {
            let f = h.rebase(TYPE_UINT);
            let t = h.call(f, Conv::Cdecl, &[]);
            let label = h.rebase(0x00c9_07f4);
            let f = h.rebase(ANNOTATION_CTOR);
            h.call(f, Conv::Thiscall(annot), &[label, t, 0]);
            let vt = h.rebase(0x00b0_fd40);
            h.write(annot, vt);
            h.write(annot + 0x18, 0x7fff_ffffu32);
            atexit(h, d.annotation_dtor);
        }
        if h.read::<u32>(annot + 4) == annot + 4 {
            let f = h.rebase(ADD_ANNOTATION);
            h.call(f, Conv::Thiscall(desc + 0xb4), &[annot]);
        }
        let array = desc + 0xe0;
        if first_time(h, guard, 8) {
            let element = h.rebase(d.element);
            let label = h.rebase(0x00b0_fd78);
            let f = h.rebase(ARRAY_MEMBER_CTOR);
            h.call(
                f,
                Conv::Thiscall(array),
                &[desc, element, label, 4, 2, 2, 4],
            );
            let vt = h.rebase(d.array_vtable);
            h.write(array, vt);
            let f = h.rebase(0x0043_20c0);
            h.write(array + 0x34, f);
            h.write(array + 0x38, 0u32);
            let f = h.rebase(d.array_fn);
            h.write(array + 0x3c, f);
            atexit(h, d.array_dtor);
        }
        h.write(array + 0x2c, 4u32);
        let f = h.rebase(REGISTER_TYPE);
        h.call(f, Conv::Cdecl, &[desc]);
        let f = h.rebase(ADD_TO_NAMESPACE);
        h.call(f, Conv::Thiscall(desc), &[]);
        desc
    }

    /// A `PClassDescriptorAbstract<PArray<T*>>` constructor `(desc)`
    /// (thiscall) -> `desc`: the base class descriptor, the global
    /// namespace, `PClassDescriptor(desc; namespace, name, 8, 4, base, 0)`,
    /// `+0x90 |= 2`, its vtable, then its three buffers.
    pub fn descriptor_ctor(h: &mut impl Host, c: &DescriptorCtor, desc: u32) -> u32 {
        let f = h.rebase(BASE_CLASS_DESCRIPTOR);
        let base = h.call(f, Conv::Cdecl, &[]);
        let name: u32 = h.read(h.rebase(c.name_global));
        let words = [name, 8, 4, base, 0];
        let f = h.rebase(GLOBAL_NAMESPACE);
        let ns = h.call(f, Conv::Cdecl, &words);
        let f = h.rebase(CLASS_DESCRIPTOR_CTOR);
        h.call(f, Conv::Thiscall(desc), &[ns, name, 8, 4, base, 0]);
        let w: u32 = h.read(desc + 0x90);
        h.write(desc + 0x90, w | 2);
        let vt = h.rebase(c.vtable);
        h.write(desc, vt);
        for (setter, buf) in [
            SET_DEFAULT_BUFFER,
            SET_VALIDATION_BUFFER,
            SET_WRITE_MASK_BUFFER,
        ]
        .into_iter()
        .zip(c.buffers)
        {
            let setter = h.rebase(setter);
            let buf = h.rebase(buf);
            h.call(setter, Conv::Thiscall(desc), &[buf]);
        }
        desc
    }
}

/// The `Bind`s `PUtilityInputs::initialize` (`0x009f4650`) runs, in its
/// order.
pub const UTILITY_BINDS: [u32; 25] = [
    0x009e_47c0,
    0x009e_4ac0,
    0x009e_49e0,
    0x009e_4940,
    0x009e_4ce0,
    0x009e_4d80,
    0x009e_4e60,
    0x009e_4fa0,
    0x009e_4f80,
    0x009e_4fc0,
    0x009e_4fe0,
    0x009e_4f40,
    0x009e_4f60,
    0x009e_4c80,
    0x009e_4ca0,
    0x009e_4cc0,
    0x009e_4c60,
    0x009e_4ba0,
    0x009e_4bc0,
    0x009e_4be0,
    0x009e_4c00,
    0x009e_4c20,
    0x009e_4c40,
    0x009e_4430,
    0x009e_46d0,
];
/// The semantic types it registers (`GetType<T>`, cdecl, then
/// `RegisterType`), in its order.
pub const UTILITY_TYPES: [u32; 6] = [
    0x00a2_dce0,
    0x00a3_23a0,
    0x00a2_e150,
    0x00a2_e5c0,
    0x00a2_ea30,
    0x009f_4730,
];
/// `Phyre_PhyreInit` (cdecl, an error code).
pub const PHYRE_INIT: u32 = 0x0043_0dd0;
/// `0x00431170` (cdecl `(1)`), the start-up's first call.
pub const STARTUP_FIRST: u32 = 0x0043_1170;
/// `SetThreadExecutionState`'s import slot (stdcall `(flags)`).
pub const SET_THREAD_EXECUTION_STATE: u32 = 0x00b0_c1ac;
/// `PApplication_InitWindowAndApp_00629ab0` (thiscall `(app, argv, argc)`).
pub const INIT_WINDOW_AND_APP: u32 = 0x0062_9ab0;

pub mod startup {
    //! The originals over game memory.

    use super::{
        Conv, Host, INIT_WINDOW_AND_APP, PHYRE_INIT, REGISTER_TYPE, SET_THREAD_EXECUTION_STATE,
        STARTUP_FIRST, UTILITY_BINDS, UTILITY_TYPES,
    };

    /// `PUtilityInputs::initialize()` (`0x009f4650`) -> 0: every input
    /// class's `Bind`, then the semantic types' registrations.
    pub fn utility_inputs_initialize(h: &mut impl Host) -> u32 {
        for f in UTILITY_BINDS {
            let f = h.rebase(f);
            h.call(f, Conv::Cdecl, &[]);
        }
        let reg = h.rebase(REGISTER_TYPE);
        for f in UTILITY_TYPES {
            let f = h.rebase(f);
            let t = h.call(f, Conv::Cdecl, &[]);
            h.call(reg, Conv::Cdecl, &[t]);
        }
        0
    }

    /// `PApplication_Startup(app, argv, argc)` (`0x00629a40`, `ret 8`) ->
    /// 0 or 9: `0x00431170(1)`, the icon/hook/drag globals cleared,
    /// `SetThreadExecutionState(CONTINUOUS | SYSTEM | DISPLAY)` through its
    /// import slot, `PhyreInit` (9 when it fails), then
    /// `InitWindowAndApp(app, argv, argc)` (9 when it fails).
    pub fn startup(h: &mut impl Host, app: u32, argv: u32, argc: u32) -> u32 {
        let f = h.rebase(STARTUP_FIRST);
        h.call(f, Conv::Cdecl, &[1]);
        let a = h.rebase(0x00cc_9cec);
        h.write(a, 0u32);
        let a = h.rebase(0x00cc_9cf0);
        h.write(a, 0u32);
        let a = h.rebase(0x00cc_9cd3);
        h.write(a, 0u8);
        let f: u32 = h.read(h.rebase(SET_THREAD_EXECUTION_STATE));
        h.call(f, Conv::Stdcall, &[0x8000_0003]);
        let f = h.rebase(PHYRE_INIT);
        if h.call(f, Conv::Cdecl, &[]) != 0 {
            return 9;
        }
        let f = h.rebase(INIT_WINDOW_AND_APP);
        if h.call(f, Conv::Thiscall(app), &[argv, argc]) != 0 {
            9
        } else {
            0
        }
    }
}
