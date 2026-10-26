//! List `p1-load`, continuation round, batch 12: the classes' static
//! registration (`Bind`, and the class-descriptor getters built the same
//! way), as data: each original is a fixed sequence of steps on static
//! objects, and [`STEPS`] holds them.
//!
//! A step that builds a static member runs once (a bit of the class's
//! guard word), and registers the member's destructor with `atexit`.
//! Method members (`PClassMethod`-like wrappers: a `PClassMember`, then a
//! vtable and a function pointer) are linked into the descriptor's method
//! list unless one of that name is there already; their vtable and
//! function words are written on every call.

use ffx_graphic::host::{Conv, Host};

use crate::descriptor::{GLOBAL_NAMESPACE, REGISTER_TYPE};
use crate::types::ATEXIT;

/// `PClassMember::PClassMember(a, name, flags)` (thiscall).
const MEMBER_CTOR: u32 = 0x0043_a080;
/// `PClassDescriptor::addToNamespace` (thiscall).
const ADD_TO_NAMESPACE: u32 = 0x0043_c140;
/// `PIClassDataMemberDynamicType::PIClassDataMemberDynamicType(member)`
/// (thiscall).
const DYNAMIC_TYPE_CTOR: u32 = 0x0057_5440;
/// `PType::setFixupFunctions(a, b)` (thiscall).
const SET_FIXUPS: u32 = 0x0043_79b0;

/// Where a data member's type comes from.
#[derive(Debug, Clone, Copy)]
pub enum Ty {
    /// A `GetType<T>()` getter (cdecl).
    Get(u32),
    /// A static descriptor.
    Fixed(u32),
}

/// One step.
#[derive(Debug, Clone, Copy)]
pub enum Step {
    /// `PTypeDispenser::RegisterType(desc)`.
    Register,
    /// `setFixupFunctions(a, b)` on the descriptor (two functions).
    Fixups(u32, u32),
    /// A member built by `ctor` (thiscall on `at`, `(desc, type, name, x,
    /// y, z)`) under guard bit `bit`, destroyed by `dtor` at exit.
    Member {
        /// Guard bit.
        bit: u32,
        /// The member's static.
        at: u32,
        /// Its constructor (`0x005752a0` data, `0x00575f30` fixed array).
        ctor: u32,
        /// Its type.
        ty: Ty,
        /// Its name.
        name: u32,
        /// The constructor's last three words.
        rest: [u32; 3],
        /// Its `atexit` destructor.
        dtor: u32,
    },
    /// A method member at `at` under guard bit `bit`: `PClassMember(desc,
    /// name, 0)`, `+0x14` = 0, linked at the tail of the descriptor's list
    /// at `list` unless `finder` (thiscall on the descriptor, `(name)`)
    /// knows the name; `atexit(dtor)`. Then, on every call, `+0x18` =
    /// `vtable`, the function at `+0x1c` (`+0x20` and a zero `+0x24` when
    /// `wide`), `+0x14` = `at + 0x18`.
    Method {
        /// Guard bit.
        bit: u32,
        /// The member's static.
        at: u32,
        /// Its name.
        name: u32,
        /// The descriptor's list it joins (`+0x4c` or `+0x54`).
        list: u32,
        /// The lookup by name of that list (`0x004359d0` or `0x004306b0`).
        finder: u32,
        /// Its `atexit` destructor.
        dtor: u32,
        /// The wrapper's vtable.
        vtable: u32,
        /// The function.
        func: u32,
        /// The function at `+0x20` (with a zero word after it).
        wide: bool,
    },
    /// A dynamic-type handler at `at` under guard bit `bit`:
    /// `PIClassDataMemberDynamicType(at; member)` (`0x00575440`), its
    /// vtable, the handler function at `+8` and a zero `+0xc`,
    /// `atexit(dtor)`. Then, on every call, the member's `+0x28` = `at`.
    DynamicType {
        /// Guard bit.
        bit: u32,
        /// The handler's static.
        at: u32,
        /// The data member it serves.
        member: u32,
        /// Its vtable.
        vtable: u32,
        /// Its function.
        func: u32,
        /// Its `atexit` destructor.
        dtor: u32,
    },
}

/// A registration.
#[derive(Debug, Clone, Copy)]
pub struct Bind {
    /// The original.
    pub addr: u32,
    /// The class descriptor.
    pub desc: u32,
    /// The guard word.
    pub guard: u32,
    /// Its steps.
    pub steps: &'static [Step],
    /// How it ends.
    pub end: End,
}

/// How a registration ends.
#[derive(Debug, Clone, Copy)]
pub enum End {
    /// `addToNamespace(desc)`; its `eax`.
    Add,
    /// `addToNamespace`'s body inline: `PNamespace::add(desc->+0x3c,
    /// desc)`, flags bit 30.
    AddInline,
    /// A class-descriptor getter: the steps (after the descriptor's own,
    /// guard bit 0: `0x0043afb0(GetGlobalNamespace(), name, size,
    /// align)`, `vtable`, `atexit(dtor)`) run until the "bound" byte is
    /// set, then `addToNamespace` and the byte; returns the descriptor.
    Getter {
        /// The bound byte.
        bound: u32,
        /// The class name.
        name: u32,
        /// Its size.
        size: u32,
        /// Its alignment.
        align: u32,
        /// The descriptor's vtable.
        vtable: u32,
        /// The descriptor's `atexit` destructor.
        dtor: u32,
    },
}

const DATA: u32 = 0x0057_52a0;
const GET_UINT: u32 = 0x0043_7fc0;

/// The registrations.
pub const STEPS: [Bind; 17] = [
    // PClassMember::Bind
    Bind {
        addr: 0x0043_a1e0,
        desc: 0x00c9_0610,
        guard: 0x00c9_06d0,
        steps: &[
            Step::Register,
            Step::Member {
                bit: 1,
                at: 0x00c9_06a4,
                ctor: DATA,
                ty: Ty::Get(GET_UINT),
                name: 0x00b0_ea78,
                rest: [0x10, 0, 0],
                dtor: 0x00af_a030,
            },
            Step::Member {
                bit: 2,
                at: 0x00c9_06d4,
                ctor: DATA,
                ty: Ty::Fixed(0x00c9_0760),
                name: 0x00b0_ea80,
                rest: [8, 2, 0],
                dtor: 0x00af_a020,
            },
        ],
        end: End::Add,
    },
    // PTypedObject::Bind
    Bind {
        addr: 0x0044_43f0,
        desc: 0x00c9_0f30,
        guard: 0x00c9_0ff0,
        steps: &[
            Step::Register,
            Step::Member {
                bit: 1,
                at: 0x00c9_0fc4,
                ctor: DATA,
                ty: Ty::Get(0x0043_5740),
                name: 0x00b0_f5f8,
                rest: [0, 0x12, 0],
                dtor: 0x00af_a320,
            },
            Step::Member {
                bit: 2,
                at: 0x00c9_0ff4,
                ctor: DATA,
                ty: Ty::Fixed(0x00c9_0760),
                name: 0x00b0_ea80,
                rest: [4, 0x12, 0],
                dtor: 0x00af_a310,
            },
        ],
        end: End::Add,
    },
    // PAssetReferenceImport::Bind
    Bind {
        addr: 0x0045_c890,
        desc: 0x00c9_4030,
        guard: 0x00c9_421c,
        steps: &[
            Step::Register,
            Step::Fixups(0x0045_d300, 0x0045_d9b0),
            Step::Member {
                bit: 1,
                at: 0x00c9_41f0,
                ctor: DATA,
                ty: Ty::Fixed(0x00c9_1048),
                name: 0x00b1_1c88,
                rest: [4, 0x10, 0],
                dtor: 0x00af_b090,
            },
            Step::Member {
                bit: 2,
                at: 0x00c9_4220,
                ctor: DATA,
                ty: Ty::Fixed(0x00c9_0760),
                name: 0x00b1_1cd4,
                rest: [0, 0x12, 0],
                dtor: 0x00af_b0a0,
            },
        ],
        end: End::Add,
    },
    // PClassDataMemberDynamic::Bind
    Bind {
        addr: 0x0044_6490,
        desc: 0x00c9_1180,
        guard: 0x00c9_1240,
        steps: &[
            Step::Register,
            Step::Member {
                bit: 1,
                at: 0x00c9_1214,
                ctor: DATA,
                ty: Ty::Get(0x0043_74d0),
                name: 0x00b0_f98c,
                rest: [0x18, 2, 0],
                dtor: 0x00af_a3e0,
            },
            Step::Member {
                bit: 2,
                at: 0x00c9_1244,
                ctor: DATA,
                ty: Ty::Get(GET_UINT),
                name: 0x00b0_f994,
                rest: [0x24, 0, 0],
                dtor: 0x00af_a3d0,
            },
            Step::Member {
                bit: 4,
                at: 0x00c9_1270,
                ctor: DATA,
                ty: Ty::Fixed(0x00c9_1048),
                name: 0x00b0_f9a0,
                rest: [0x2c, 0, 0],
                dtor: 0x00af_a3c0,
            },
        ],
        end: End::Add,
    },
    // PClusterHeaderNVN::Bind
    Bind {
        addr: 0x0046_9a70,
        desc: 0x00c9_70f8,
        guard: 0x00c9_71b8,
        steps: &[
            Step::Register,
            Step::Member {
                bit: 1,
                at: 0x00c9_718c,
                ctor: DATA,
                ty: Ty::Get(GET_UINT),
                name: 0x00b1_3200,
                rest: [0x48, 0, 0],
                dtor: 0x00af_b8a0,
            },
            Step::Member {
                bit: 2,
                at: 0x00c9_71bc,
                ctor: DATA,
                ty: Ty::Get(GET_UINT),
                name: 0x00b1_3214,
                rest: [0x4c, 0, 0],
                dtor: 0x00af_b8c0,
            },
            Step::Member {
                bit: 4,
                at: 0x00c9_71e8,
                ctor: DATA,
                ty: Ty::Get(GET_UINT),
                name: 0x00b1_3228,
                rest: [0x50, 0, 0],
                dtor: 0x00af_b8b0,
            },
        ],
        end: End::Add,
    },
    // PNamespace::Bind
    Bind {
        addr: 0x0043_e000,
        desc: 0x00c9_0a18,
        guard: 0x00c9_0ad4,
        steps: &[
            Step::Register,
            Step::Method {
                bit: 1,
                at: 0x00c9_0aac,
                name: 0x00b0_ef48,
                list: 0x54,
                finder: 0x0043_06b0,
                dtor: 0x00af_a0d0,
                vtable: 0x00b0_eff4,
                func: 0x0043_e2f0,
                wide: false,
            },
            Step::Method {
                bit: 2,
                at: 0x00c9_0ad8,
                name: 0x00b0_ef5c,
                list: 0x4c,
                finder: 0x0043_59d0,
                dtor: 0x00af_a110,
                vtable: 0x00b0_f00c,
                func: 0x0043_e950,
                wide: true,
            },
        ],
        end: End::Add,
    },
    // PClassDescriptor::Bind
    Bind {
        addr: 0x0043_b8b0,
        desc: 0x00c9_0760,
        guard: 0x00c9_0824,
        steps: &[
            Step::Register,
            Step::Fixups(0x0043_bc10, 0x0043_be30),
            Step::Method {
                bit: 1,
                at: 0x00c9_07fc,
                name: 0x00b0_ec24,
                list: 0x4c,
                finder: 0x0043_59d0,
                dtor: 0x00af_a070,
                vtable: 0x00b0_edd8,
                func: 0x0043_1420,
                wide: true,
            },
        ],
        end: End::AddInline,
    },
    // PTypeDispenser::GetClassDescriptor<PTimer>
    Bind {
        addr: 0x0043_0340,
        desc: 0x00c9_01b0,
        guard: 0x00c9_0244,
        steps: &[Step::Method {
            bit: 2,
            at: 0x00c9_024c,
            name: 0x00b0_de34,
            list: 0x54,
            finder: 0x0043_06b0,
            dtor: 0x00af_9bd0,
            vtable: 0x00b0_df9c,
            func: 0x0042_f330,
            wide: false,
        }],
        end: End::Getter {
            bound: 0x00c9_0248,
            name: 0x00b0_dbd0,
            size: 1,
            align: 1,
            vtable: 0x00b0_ddf4,
            dtor: 0x00af_9c10,
        },
    },
    // PTypeDispenser::GetClassDescriptor<PMatrix4x3>
    Bind {
        addr: 0x0046_2aa0,
        desc: 0x00c9_4dd8,
        guard: 0x00c9_4e6c,
        steps: &[
            Step::Member {
                bit: 2,
                at: 0x00c9_4e74,
                ctor: 0x0057_5f30,
                ty: Ty::Get(0x0043_8060),
                name: 0x00b1_1fec,
                rest: [0, 0, 0xc],
                dtor: 0x00af_b6b0,
            },
            Step::Method {
                bit: 4,
                at: 0x00c9_4eac,
                name: 0x00b1_2514,
                list: 0x4c,
                finder: 0x0043_59d0,
                dtor: 0x00af_b6c0,
                vtable: 0x00b1_2528,
                func: 0x0046_32f0,
                wide: false,
            },
        ],
        end: End::Getter {
            bound: 0x00c9_4e70,
            name: 0x00b1_247c,
            size: 0x30,
            align: 4,
            vtable: 0x00b1_24d4,
            dtor: 0x00af_b690,
        },
    },
    // PCluster::Bind
    Bind {
        addr: 0x0043_ffe0,
        desc: 0x00c9_0b90,
        guard: 0x00c9_0c7c,
        steps: &[
            Step::Register,
            Step::Method {
                bit: 1,
                at: 0x00c9_0c54,
                name: 0x00b0_f12c,
                list: 0x4c,
                finder: 0x0043_59d0,
                dtor: 0x00af_a1c0,
                vtable: 0x00b0_f2a4,
                func: 0x0044_1120,
                wide: true,
            },
            Step::Method {
                bit: 2,
                at: 0x00c9_0c80,
                name: 0x00b0_f134,
                list: 0x4c,
                finder: 0x0043_59d0,
                dtor: 0x00af_a240,
                vtable: 0x00b0_f2bc,
                func: 0x0044_1380,
                wide: true,
            },
            Step::Method {
                bit: 4,
                at: 0x00c9_0ca8,
                name: 0x00b0_f144,
                list: 0x4c,
                finder: 0x0043_59d0,
                dtor: 0x00af_a200,
                vtable: 0x00b0_f2d4,
                func: 0x0044_1180,
                wide: true,
            },
            Step::Method {
                bit: 8,
                at: 0x00c9_0cd0,
                name: 0x00b0_f15c,
                list: 0x4c,
                finder: 0x0043_59d0,
                dtor: 0x00af_a180,
                vtable: 0x00b0_f2ec,
                func: 0x0044_09e0,
                wide: true,
            },
        ],
        end: End::Add,
    },
    // GetClassDescriptor<Vector2>
    Bind {
        addr: 0x0045_fb40,
        desc: 0x00c9_4448,
        guard: 0x00c9_44dc,
        steps: &[
            Step::Member {
                bit: 2,
                at: 0x00c9_44e4,
                ctor: 0x0057_5f30,
                ty: Ty::Get(0x0043_8060),
                name: 0x00b1_1fec,
                rest: [0, 0, 2],
                dtor: 0x00af_b260,
            },
            Step::Method {
                bit: 4,
                at: 0x00c9_451c,
                name: 0x00b1_1ff8,
                list: 0x4c,
                finder: 0x0043_59d0,
                dtor: 0x00af_b390,
                vtable: 0x00b1_2408,
                func: 0x0046_2640,
                wide: false,
            },
            Step::Method {
                bit: 8,
                at: 0x00c9_4544,
                name: 0x00b1_2000,
                list: 0x4c,
                finder: 0x0043_59d0,
                dtor: 0x00af_b4d0,
                vtable: 0x00b1_2408,
                func: 0x0046_2680,
                wide: false,
            },
        ],
        end: End::Getter {
            bound: 0x00c9_44e0,
            name: 0x00b1_1f18,
            size: 8,
            align: 4,
            vtable: 0x00b1_1fac,
            dtor: 0x00af_b1b0,
        },
    },
    // GetClassDescriptor<Point3>
    Bind {
        addr: 0x0045_f4e0,
        desc: 0x00c9_46c0,
        guard: 0x00c9_4754,
        steps: &[
            Step::Member {
                bit: 2,
                at: 0x00c9_475c,
                ctor: 0x0057_5f30,
                ty: Ty::Get(0x0043_8060),
                name: 0x00b1_1fec,
                rest: [0, 0, 4],
                dtor: 0x00af_b240,
            },
            Step::Method {
                bit: 4,
                at: 0x00c9_4794,
                name: 0x00b1_1ff8,
                list: 0x4c,
                finder: 0x0043_59d0,
                dtor: 0x00af_b310,
                vtable: 0x00b1_2438,
                func: 0x0046_2620,
                wide: false,
            },
            Step::Method {
                bit: 8,
                at: 0x00c9_47bc,
                name: 0x00b1_2000,
                list: 0x4c,
                finder: 0x0043_59d0,
                dtor: 0x00af_b450,
                vtable: 0x00b1_2438,
                func: 0x0046_2660,
                wide: false,
            },
            Step::Method {
                bit: 16,
                at: 0x00c9_47e4,
                name: 0x00b1_2094,
                list: 0x4c,
                finder: 0x0043_59d0,
                dtor: 0x00af_b590,
                vtable: 0x00b1_2438,
                func: 0x0046_26a0,
                wide: false,
            },
        ],
        end: End::Getter {
            bound: 0x00c9_4758,
            name: 0x00b1_1f38,
            size: 0x10,
            align: 4,
            vtable: 0x00b1_20ec,
            dtor: 0x00af_b170,
        },
    },
    // GetClassDescriptor<Vector3>
    Bind {
        addr: 0x0045_fd90,
        desc: 0x00c9_4570,
        guard: 0x00c9_4604,
        steps: &[
            Step::Member {
                bit: 2,
                at: 0x00c9_460c,
                ctor: 0x0057_5f30,
                ty: Ty::Get(0x0043_8060),
                name: 0x00b1_1fec,
                rest: [0, 0, 4],
                dtor: 0x00af_b270,
            },
            Step::Method {
                bit: 4,
                at: 0x00c9_4644,
                name: 0x00b1_1ff8,
                list: 0x4c,
                finder: 0x0043_59d0,
                dtor: 0x00af_b3d0,
                vtable: 0x00b1_2420,
                func: 0x0046_2650,
                wide: false,
            },
            Step::Method {
                bit: 8,
                at: 0x00c9_466c,
                name: 0x00b1_2000,
                list: 0x4c,
                finder: 0x0043_59d0,
                dtor: 0x00af_b510,
                vtable: 0x00b1_2420,
                func: 0x0046_2690,
                wide: false,
            },
            Step::Method {
                bit: 16,
                at: 0x00c9_4694,
                name: 0x00b1_2094,
                list: 0x4c,
                finder: 0x0043_59d0,
                dtor: 0x00af_b610,
                vtable: 0x00b1_2420,
                func: 0x0046_26c0,
                wide: false,
            },
        ],
        end: End::Getter {
            bound: 0x00c9_4608,
            name: 0x00b1_1f20,
            size: 0x10,
            align: 4,
            vtable: 0x00b1_2054,
            dtor: 0x00af_b1d0,
        },
    },
    // GetClassDescriptor<Quat>
    Bind {
        addr: 0x0045_f7c0,
        desc: 0x00c9_4988,
        guard: 0x00c9_4a1c,
        steps: &[
            Step::Member {
                bit: 2,
                at: 0x00c9_4a24,
                ctor: 0x0057_5f30,
                ty: Ty::Get(0x0043_8060),
                name: 0x00b1_1fec,
                rest: [0, 0, 4],
                dtor: 0x00af_b250,
            },
            Step::Method {
                bit: 4,
                at: 0x00c9_4a5c,
                name: 0x00b1_1ff8,
                list: 0x4c,
                finder: 0x0043_59d0,
                dtor: 0x00af_b350,
                vtable: 0x00b1_2468,
                func: 0x0046_2630,
                wide: false,
            },
            Step::Method {
                bit: 8,
                at: 0x00c9_4a84,
                name: 0x00b1_2000,
                list: 0x4c,
                finder: 0x0043_59d0,
                dtor: 0x00af_b490,
                vtable: 0x00b1_2468,
                func: 0x0046_2670,
                wide: false,
            },
            Step::Method {
                bit: 16,
                at: 0x00c9_4aac,
                name: 0x00b1_2094,
                list: 0x4c,
                finder: 0x0043_59d0,
                dtor: 0x00af_b5d0,
                vtable: 0x00b1_2468,
                func: 0x0046_26b0,
                wide: false,
            },
            Step::Method {
                bit: 32,
                at: 0x00c9_4ad4,
                name: 0x00b1_21bc,
                list: 0x4c,
                finder: 0x0043_59d0,
                dtor: 0x00af_b290,
                vtable: 0x00b1_2468,
                func: 0x0046_2610,
                wide: false,
            },
        ],
        end: End::Getter {
            bound: 0x00c9_4a20,
            name: 0x00b1_1f30,
            size: 0x10,
            align: 4,
            vtable: 0x00b1_2214,
            dtor: 0x00af_b190,
        },
    },
    // GetClassDescriptor<Vector4>
    Bind {
        addr: 0x0046_0070,
        desc: 0x00c9_4810,
        guard: 0x00c9_48a4,
        steps: &[
            Step::Member {
                bit: 2,
                at: 0x00c9_48ac,
                ctor: 0x0057_5f30,
                ty: Ty::Get(0x0043_8060),
                name: 0x00b1_1fec,
                rest: [0, 0, 4],
                dtor: 0x00af_b280,
            },
            Step::Method {
                bit: 4,
                at: 0x00c9_48e4,
                name: 0x00b1_1ff8,
                list: 0x4c,
                finder: 0x0043_59d0,
                dtor: 0x00af_b410,
                vtable: 0x00b1_2450,
                func: 0x0045_4e50,
                wide: false,
            },
            Step::Method {
                bit: 8,
                at: 0x00c9_490c,
                name: 0x00b1_2000,
                list: 0x4c,
                finder: 0x0043_59d0,
                dtor: 0x00af_b550,
                vtable: 0x00b1_2450,
                func: 0x0045_4e60,
                wide: false,
            },
            Step::Method {
                bit: 16,
                at: 0x00c9_4934,
                name: 0x00b1_2094,
                list: 0x4c,
                finder: 0x0043_59d0,
                dtor: 0x00af_b650,
                vtable: 0x00b1_2450,
                func: 0x0045_4e70,
                wide: false,
            },
            Step::Method {
                bit: 32,
                at: 0x00c9_495c,
                name: 0x00b1_21bc,
                list: 0x4c,
                finder: 0x0043_59d0,
                dtor: 0x00af_b2d0,
                vtable: 0x00b1_2450,
                func: 0x0045_2790,
                wide: false,
            },
        ],
        end: End::Getter {
            bound: 0x00c9_48a8,
            name: 0x00b1_1f28,
            size: 0x10,
            align: 4,
            vtable: 0x00b1_217c,
            dtor: 0x00af_b1f0,
        },
    },
    // PClusterHeaderBase::Bind
    Bind {
        addr: 0x0046_8890,
        desc: 0x00c9_6aa8,
        guard: 0x00c9_6c00,
        steps: &[
            Step::Register,
            Step::Member {
                bit: 1,
                at: 0x00c9_6bd4,
                ctor: 0x0057_52a0,
                ty: Ty::Get(0x0043_7fc0),
                name: 0x00b1_2ebc,
                rest: [0x0, 0x8, 0x0],
                dtor: 0x00af_b7c0,
            },
            Step::Member {
                bit: 2,
                at: 0x00c9_6c04,
                ctor: 0x0057_52a0,
                ty: Ty::Get(0x0043_7fc0),
                name: 0x00b0_fb4c,
                rest: [0x4, 0x8, 0x0],
                dtor: 0x00af_b840,
            },
            Step::Member {
                bit: 4,
                at: 0x00c9_6c30,
                ctor: 0x0057_52a0,
                ty: Ty::Get(0x0043_7fc0),
                name: 0x00b1_2ecc,
                rest: [0x10, 0x8, 0x0],
                dtor: 0x00af_b7a0,
            },
            Step::Member {
                bit: 8,
                at: 0x00c9_6c5c,
                ctor: 0x0057_52a0,
                ty: Ty::Get(0x0043_7fc0),
                name: 0x00b1_2ee0,
                rest: [0x8, 0x8, 0x0],
                dtor: 0x00af_b7b0,
            },
            Step::Member {
                bit: 16,
                at: 0x00c9_6c88,
                ctor: 0x0057_52a0,
                ty: Ty::Get(0x0043_7fc0),
                name: 0x00b1_2ef8,
                rest: [0x14, 0x8, 0x0],
                dtor: 0x00af_b770,
            },
            Step::Member {
                bit: 32,
                at: 0x00c9_6cb4,
                ctor: 0x0057_52a0,
                ty: Ty::Get(0x0043_7fc0),
                name: 0x00b1_2f0c,
                rest: [0x18, 0x8, 0x0],
                dtor: 0x00af_b760,
            },
            Step::Member {
                bit: 64,
                at: 0x00c9_6ce0,
                ctor: 0x0057_52a0,
                ty: Ty::Get(0x0043_7fc0),
                name: 0x00b1_2f20,
                rest: [0x1c, 0x8, 0x0],
                dtor: 0x00af_b820,
            },
            Step::Member {
                bit: 128,
                at: 0x00c9_6d0c,
                ctor: 0x0057_52a0,
                ty: Ty::Get(0x0043_7fc0),
                name: 0x00b1_2f34,
                rest: [0x20, 0x8, 0x0],
                dtor: 0x00af_b810,
            },
            Step::Member {
                bit: 0x100,
                at: 0x00c9_6d38,
                ctor: 0x0057_52a0,
                ty: Ty::Get(0x0043_7fc0),
                name: 0x00b1_2f48,
                rest: [0x24, 0x8, 0x0],
                dtor: 0x00af_b800,
            },
            Step::Member {
                bit: 0x200,
                at: 0x00c9_6d64,
                ctor: 0x0057_52a0,
                ty: Ty::Get(0x0043_7fc0),
                name: 0x00b1_2f60,
                rest: [0x28, 0x8, 0x0],
                dtor: 0x00af_b7f0,
            },
            Step::Member {
                bit: 0x400,
                at: 0x00c9_6d90,
                ctor: 0x0057_52a0,
                ty: Ty::Get(0x0043_7fc0),
                name: 0x00b1_2f7c,
                rest: [0x2c, 0x8, 0x0],
                dtor: 0x00af_b830,
            },
            Step::Member {
                bit: 0x800,
                at: 0x00c9_6dbc,
                ctor: 0x0057_52a0,
                ty: Ty::Get(0x0043_7fc0),
                name: 0x00b1_2f94,
                rest: [0x30, 0x8, 0x0],
                dtor: 0x00af_b860,
            },
            Step::Member {
                bit: 0x1000,
                at: 0x00c9_6de8,
                ctor: 0x0057_52a0,
                ty: Ty::Get(0x0043_7fc0),
                name: 0x00b1_2fa8,
                rest: [0x34, 0x8, 0x0],
                dtor: 0x00af_b870,
            },
            Step::Member {
                bit: 0x2000,
                at: 0x00c9_6e14,
                ctor: 0x0057_52a0,
                ty: Ty::Get(0x0043_7fc0),
                name: 0x00b1_2fbc,
                rest: [0x38, 0x8, 0x0],
                dtor: 0x00af_b850,
            },
            Step::Member {
                bit: 0x4000,
                at: 0x00c9_6e40,
                ctor: 0x0057_52a0,
                ty: Ty::Get(0x0043_7fc0),
                name: 0x00b1_2fcc,
                rest: [0x3c, 0x8, 0x0],
                dtor: 0x00af_b790,
            },
            Step::Member {
                bit: 0x8000,
                at: 0x00c9_6e6c,
                ctor: 0x0057_52a0,
                ty: Ty::Get(0x0043_7fc0),
                name: 0x00b1_2fe8,
                rest: [0x40, 0x8, 0x0],
                dtor: 0x00af_b780,
            },
            Step::Member {
                bit: 0x10000,
                at: 0x00c9_6e98,
                ctor: 0x0057_52a0,
                ty: Ty::Get(0x0043_7fc0),
                name: 0x00b1_3000,
                rest: [0xc, 0x8, 0x0],
                dtor: 0x00af_b7e0,
            },
            Step::Member {
                bit: 0x20000,
                at: 0x00c9_6ec4,
                ctor: 0x0057_52a0,
                ty: Ty::Get(0x0043_7fc0),
                name: 0x00b1_3010,
                rest: [0x44, 0x8, 0x0],
                dtor: 0x00af_b7d0,
            },
        ],
        end: End::Add,
    },
    // PAssetReference::Bind
    Bind {
        addr: 0x0045_c500,
        desc: 0x00c9_3f98,
        guard: 0x00c9_4108,
        steps: &[
            Step::Register,
            Step::Member {
                bit: 1,
                at: 0x00c9_40dc,
                ctor: DATA,
                ty: Ty::Fixed(0x00c9_1048),
                name: 0x00b1_1c88,
                rest: [0x18, 0x10, 0],
                dtor: 0x00af_b080,
            },
            Step::Member {
                bit: 2,
                at: 0x00c9_410c,
                ctor: DATA,
                ty: Ty::Get(0x0043_5740),
                name: 0x00b1_1c90,
                rest: [0x1c, 0x12, 0],
                dtor: 0x00af_b040,
            },
            Step::Member {
                bit: 4,
                at: 0x00c9_4138,
                ctor: DATA,
                ty: Ty::Fixed(0x00c9_0760),
                name: 0x00b1_1c98,
                rest: [0x20, 0x12, 0],
                dtor: 0x00af_b070,
            },
            Step::DynamicType {
                bit: 8,
                at: 0x00c9_4168,
                member: 0x00c9_410c,
                vtable: 0x00b1_1ca8,
                func: 0x0045_de70,
                dtor: 0x00af_b050,
            },
            Step::Method {
                bit: 0x10,
                at: 0x00c9_4178,
                name: 0x00b1_1cb0,
                list: 0x4c,
                finder: 0x0043_59d0,
                dtor: 0x00af_b000,
                vtable: 0x00b1_1d54,
                func: 0x0045_ded0,
                wide: true,
            },
            Step::Method {
                bit: 0x20,
                at: 0x00c9_41a0,
                name: 0x00b1_1cb8,
                list: 0x4c,
                finder: 0x0043_59d0,
                dtor: 0x00af_af80,
                vtable: 0x00b1_1d6c,
                func: 0x0045_de80,
                wide: true,
            },
            Step::Method {
                bit: 0x40,
                at: 0x00c9_41c8,
                name: 0x00b1_1cc4,
                list: 0x4c,
                finder: 0x0043_59d0,
                dtor: 0x00af_afc0,
                vtable: 0x00b1_1d84,
                func: 0x0044_19d0,
                wide: true,
            },
        ],
        end: End::Add,
    },
];

/// The registration at `addr`.
#[must_use]
pub fn bind(addr: u32) -> Option<&'static Bind> {
    STEPS.iter().find(|b| b.addr == addr)
}

/// The ports over game memory.
pub mod bridge {
    use super::{
        ADD_TO_NAMESPACE, ATEXIT, Bind, Conv, DYNAMIC_TYPE_CTOR, End, GLOBAL_NAMESPACE, Host,
        MEMBER_CTOR, REGISTER_TYPE, SET_FIXUPS, Step, Ty,
    };

    /// Sets `bit` in the guard word and says whether it was clear.
    fn first_time(h: &mut impl Host, guard: u32, bit: u32) -> bool {
        let g: u32 = h.read(guard);
        if g & bit != 0 {
            return false;
        }
        h.write(guard, g | bit);
        true
    }

    /// One step; the `eax` of its last call.
    fn step(h: &mut impl Host, desc: u32, guard: u32, s: &Step) -> u32 {
        match *s {
            Step::Register => h.call_sym(REGISTER_TYPE, Conv::Cdecl, &[desc]),
            Step::Fixups(a, b) => {
                let (a, b) = (h.rebase(a), h.rebase(b));
                h.call_sym(SET_FIXUPS, Conv::Thiscall(desc), &[a, b])
            }
            Step::Member {
                bit,
                at,
                ctor,
                ty,
                name,
                rest,
                dtor,
            } => {
                if !first_time(h, guard, bit) {
                    return 0;
                }
                let name = h.rebase(name);
                let ty = match ty {
                    Ty::Get(f) => h.call_sym(f, Conv::Cdecl, &[]),
                    Ty::Fixed(t) => h.rebase(t),
                };
                let at = h.rebase(at);
                h.call_sym(
                    ctor,
                    Conv::Thiscall(at),
                    &[desc, ty, name, rest[0], rest[1], rest[2]],
                );
                let dtor = h.rebase(dtor);
                h.call_sym(ATEXIT, Conv::Cdecl, &[dtor])
            }
            Step::Method {
                bit,
                at,
                name,
                list,
                finder,
                dtor,
                vtable,
                func,
                wide,
            } => {
                let at = h.rebase(at);
                if first_time(h, guard, bit) {
                    let name = h.rebase(name);
                    h.call_sym(MEMBER_CTOR, Conv::Thiscall(at), &[desc, name, 0]);
                    let owner: u32 = h.read(at + 8);
                    let mname: u32 = h.read(at + 0xc);
                    h.write(at + 0x14, 0u32);
                    let head = owner.wrapping_add(list);
                    if mname != 0 && h.call_sym(finder, Conv::Thiscall(owner), &[mname]) == 0 {
                        let tail: u32 = h.read(head + 4);
                        h.write(at, [head, tail]);
                        h.write(tail, at);
                        h.write(head + 4, at);
                    }
                    let dtor = h.rebase(dtor);
                    h.call_sym(ATEXIT, Conv::Cdecl, &[dtor]);
                }
                let vt = h.rebase(vtable);
                let func = h.rebase(func);
                h.write(at + 0x18, vt);
                if wide {
                    h.write(at + 0x20, [func, 0]);
                } else {
                    h.write(at + 0x1c, func);
                }
                h.write(at + 0x14, at + 0x18);
                0
            }
            Step::DynamicType {
                bit,
                at,
                member,
                vtable,
                func,
                dtor,
            } => {
                let at = h.rebase(at);
                let member = h.rebase(member);
                if first_time(h, guard, bit) {
                    h.call_sym(DYNAMIC_TYPE_CTOR, Conv::Thiscall(at), &[member]);
                    let vt = h.rebase(vtable);
                    let func = h.rebase(func);
                    h.write(at, vt);
                    h.write(at + 8, [func, 0]);
                    let dtor = h.rebase(dtor);
                    h.call_sym(ATEXIT, Conv::Cdecl, &[dtor]);
                }
                h.write(member + 0x28, at);
                0
            }
        }
    }

    /// A registration ([`super::STEPS`]): its steps, then its end. A
    /// getter returns the descriptor; the others the last call's `eax`.
    pub fn run(h: &mut impl Host, b: &Bind) -> u32 {
        let desc = h.rebase(b.desc);
        let guard = h.rebase(b.guard);
        match b.end {
            End::Getter {
                bound,
                name,
                size,
                align,
                vtable,
                dtor,
            } => {
                if first_time(h, guard, 1) {
                    let name = h.rebase(name);
                    let ns = h.call_sym(GLOBAL_NAMESPACE, Conv::Cdecl, &[]);
                    h.call_sym(0x0043_afb0, Conv::Thiscall(desc), &[ns, name, size, align]);
                    let vt = h.rebase(vtable);
                    h.write(desc, vt);
                    let dtor = h.rebase(dtor);
                    h.call_sym(ATEXIT, Conv::Cdecl, &[dtor]);
                }
                let bound = h.rebase(bound);
                if h.read::<u8>(bound) == 0 {
                    for s in b.steps {
                        step(h, desc, guard, s);
                    }
                    h.call_sym(ADD_TO_NAMESPACE, Conv::Thiscall(desc), &[]);
                    h.write(bound, 1u8);
                }
                desc
            }
            End::Add => {
                for s in b.steps {
                    step(h, desc, guard, s);
                }
                h.call_sym(ADD_TO_NAMESPACE, Conv::Thiscall(desc), &[])
            }
            End::AddInline => {
                for s in b.steps {
                    step(h, desc, guard, s);
                }
                let ns: u32 = h.read(desc + 0x3c);
                let r = h.call_sym(0x0043_e660, Conv::Thiscall(ns), &[desc]);
                let flags: u32 = h.read(desc + 0x90);
                h.write(desc + 0x90, flags | 0x4000_0000);
                r
            }
        }
    }

    /// `PClusterDependencyList::Bind()` (`0x0044b2b0`):
    /// `RegisterType(0x00c91958)`, `0x0044b3b0(*0x00c0b7a0)`; once (guard
    /// `0x00c91a20` bit 0) its member: the `PSharray<PString>` descriptor
    /// `0x00c91a28` once (its own guard `0x00c91abc`: `0x0044ac10`, flags
    /// bit 1 cleared, vtable `0x00b100fc`, `atexit`), then
    /// `PClassDataMemberArray` `0x00c919ec` (`0x00575ef0`) with vtable
    /// `0x00b10024` and `atexit`; then the member's flags bit 3 and
    /// `addToNamespace`; its `eax`.
    pub fn dependency_list_bind(h: &mut impl Host) -> u32 {
        let desc = h.rebase(0x00c9_1958);
        h.call_sym(REGISTER_TYPE, Conv::Cdecl, &[desc]);
        let arg: u32 = h.read(h.rebase(0x00c0_b7a0));
        h.call_sym(0x0044_b3b0, Conv::Cdecl, &[arg]);
        let guard = h.rebase(0x00c9_1a20);
        let member = h.rebase(0x00c9_19ec);
        if first_time(h, guard, 1) {
            let array = h.rebase(0x00c9_1a28);
            let inner = h.rebase(0x00c9_1abc);
            if first_time(h, inner, 1) {
                h.call_sym(0x0044_ac10, Conv::Thiscall(array), &[]);
                let flags: u32 = h.read(array + 0x90);
                h.write(array + 0x90, flags & !2);
                let vt = h.rebase(0x00b1_00fc);
                h.write(array, vt);
                let dtor = h.rebase(0x00af_a540);
                h.call_sym(ATEXIT, Conv::Cdecl, &[dtor]);
            }
            let name = h.rebase(0x00b1_0050);
            h.call_sym(
                0x0057_5ef0,
                Conv::Thiscall(member),
                &[desc, array, name, 0, 0, 4, 4],
            );
            let vt = h.rebase(0x00b1_0024);
            h.write(member, vt);
            let dtor = h.rebase(0x00af_a570);
            h.call_sym(ATEXIT, Conv::Cdecl, &[dtor]);
        }
        let flags: u32 = h.read(member + 0x14);
        h.write(member + 0x14, flags | 8);
        h.call_sym(0x0043_c140, Conv::Thiscall(desc), &[])
    }
}
