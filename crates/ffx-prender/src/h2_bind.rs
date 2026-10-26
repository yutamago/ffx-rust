//! The render classes' `Bind()` functions (h2-render-b round 3): each
//! registers its class (`registerType`), builds its data members once
//! (function-local statics behind the bits of a guard word, each with its
//! `atexit` destructor), and adds the class to its namespace.
//!
//! Every class has its own copy; [`bridge::bind`] runs a class's
//! [`Bind`] table (the class, its members in order, the base class's
//! `Bind` call, flag stores), which the shims pass. The tables were read
//! from each copy's machine code (`~/ffx-work/m4/port/h2r3/bindgen.py`
//! drafts them; the oracle tests compare every copy with its original).
//!
//! Member kinds:
//! - [`Member`]: `PClassDataMember::PClassDataMember(member; class, type,
//!   name, offset, a, b)` (`0x005752a0`; fixed-size arrays
//!   `0x00575f30`), the type an image address or a
//!   type getter's result (`PTypeDispenser::GetType<T>()`);
//! - [`Array`]: a `PArray<T>` member: the array's class descriptor built
//!   once first (its own guard: constructor, flag bit 1 of `+0x90`
//!   cleared, vtable, `atexit`), then
//!   `PClassDataMemberArrayDynamic(member; class, descriptor, name,
//!   offset, x, y, z)` (`0x00575ef0`) and the member's vtable.

use ffx_graphic::host::{Conv, Host};

use crate::declare::{ADD_TO_NAMESPACE, ATEXIT, MEMBER_ARRAY_CTOR, REGISTER_TYPE};

/// A data member's type.
#[derive(Debug, Clone, Copy)]
pub enum Ty {
    /// A type descriptor in the image.
    Image(u32),
    /// The result of a type getter (cdecl, no arguments), called after
    /// the guard bit is set.
    Get(u32),
    /// The same with one argument (cdecl), which the getter may ignore.
    GetArg(u32, u32),
}

/// A plain data member.
#[derive(Debug, Clone, Copy)]
pub struct Member {
    /// The member's constructor: `PClassDataMember` (`0x005752a0`) or
    /// `PClassDataMemberArrayFixedSize` (`0x00575f30`), same arguments.
    pub ctor: u32,
    /// The guard word and its bit.
    pub guard: (u32, u32),
    pub member: u32,
    pub ty: Ty,
    pub name: u32,
    pub off: u32,
    pub a: u32,
    pub b: u32,
    pub atexit: u32,
}

/// A dynamic-array data member and its array class descriptor.
#[derive(Debug, Clone, Copy)]
pub struct Array {
    pub guard: (u32, u32),
    pub desc_guard: (u32, u32),
    pub desc: u32,
    pub desc_ctor: u32,
    pub desc_vtable: u32,
    pub desc_atexit: u32,
    pub member: u32,
    pub name: u32,
    pub off: u32,
    pub x: u32,
    pub y: u32,
    pub z: u32,
    pub vtable: u32,
    pub atexit: u32,
}

/// One step of a `Bind()`.
#[derive(Debug, Clone, Copy)]
pub enum Step {
    /// The base class's (or a helper's) call `func([global])` (cdecl).
    Base {
        global: u32,
        func: u32,
    },
    Member(Member),
    Array(Array),
    /// `[at] |= bits` (a member's flags word, every call).
    Or {
        at: u32,
        bits: u32,
    },
}

/// A class's `Bind()` table.
#[derive(Debug, Clone, Copy)]
pub struct Bind {
    pub class: u32,
    pub steps: &'static [Step],
}

pub mod bridge {
    //! The originals over game memory.

    use super::{
        ADD_TO_NAMESPACE, ATEXIT, Array, Bind, Conv, Host, MEMBER_ARRAY_CTOR, Member,
        REGISTER_TYPE, Step, Ty,
    };

    /// Sets the guard's bit and says whether it was clear (the static
    /// still to build).
    fn first_time(h: &mut impl Host, (guard, bit): (u32, u32)) -> bool {
        let g = h.rebase(guard);
        let v: u32 = h.read(g);
        if v & bit != 0 {
            return false;
        }
        h.write(g, v | bit);
        true
    }

    fn atexit(h: &mut impl Host, f: u32) {
        let f = h.rebase(f);
        h.call_sym(ATEXIT, Conv::Cdecl, &[f]);
    }

    fn member(h: &mut impl Host, class: u32, m: &Member) {
        if !first_time(h, m.guard) {
            return;
        }
        let ty = match m.ty {
            Ty::Image(t) => h.rebase(t),
            Ty::Get(f) => h.call_sym(f, Conv::Cdecl, &[]),
            Ty::GetArg(f, x) => h.call_sym(f, Conv::Cdecl, &[x]),
        };
        let name = h.rebase(m.name);
        let at = h.rebase(m.member);
        h.call_sym(
            m.ctor,
            Conv::Thiscall(at),
            &[class, ty, name, m.off, m.a, m.b],
        );
        atexit(h, m.atexit);
    }

    fn array(h: &mut impl Host, class: u32, a: &Array) {
        if !first_time(h, a.guard) {
            return;
        }
        let desc = h.rebase(a.desc);
        if first_time(h, a.desc_guard) {
            h.call_sym(a.desc_ctor, Conv::Thiscall(desc), &[]);
            let f: u32 = h.read(desc + 0x90);
            h.write(desc + 0x90, f & !2);
            let vt = h.rebase(a.desc_vtable);
            h.write(desc, vt);
            atexit(h, a.desc_atexit);
        }
        let name = h.rebase(a.name);
        let at = h.rebase(a.member);
        h.call_sym(
            MEMBER_ARRAY_CTOR,
            Conv::Thiscall(at),
            &[class, desc, name, a.off, a.x, a.y, a.z],
        );
        let vt = h.rebase(a.vtable);
        h.write(at, vt);
        atexit(h, a.atexit);
    }

    /// A class's `Bind()`: `registerType(class)`, the steps in order, then
    /// `addToNamespace(class)` (thiscall); returns its `eax`.
    pub fn bind(h: &mut impl Host, b: &Bind) -> u32 {
        let class = h.rebase(b.class);
        h.call_sym(REGISTER_TYPE, Conv::Cdecl, &[class]);
        for step in b.steps {
            match step {
                Step::Base { global, func } => {
                    let g = h.rebase(*global);
                    let w: u32 = h.read(g);
                    h.call_sym(*func, Conv::Cdecl, &[w]);
                }
                Step::Member(m) => member(h, class, m),
                Step::Array(a) => array(h, class, a),
                Step::Or { at, bits } => {
                    let at = h.rebase(*at);
                    let v: u32 = h.read(at);
                    h.write(at, v | bits);
                }
            }
        }
        h.call_sym(ADD_TO_NAMESPACE, Conv::Thiscall(class), &[])
    }
}

// The tables (`bindgen.py` drafts, checked by the oracle tests).

/// `Phyre_PRendering_PMeshInstanceSegmentContext_Bind`.
pub const B_004F4700: Bind = Bind {
    class: 0x00ca_7190,
    steps: &[
        Step::Base {
            global: 0x00c1_8c08,
            func: 0x004f_4d60,
        },
        Step::Array(Array {
            guard: (0x00ca_73d0, 0x1),
            desc_guard: (0x00ca_7a3c, 0x1),
            desc: 0x00ca_79a8,
            desc_ctor: 0x004f_2160,
            desc_vtable: 0x00b2_319c,
            desc_atexit: 0x00af_f370,
            member: 0x00ca_739c,
            name: 0x00b2_2f10,
            off: 0x1c,
            x: 0x0,
            y: 0x4,
            z: 0x4,
            vtable: 0x00b2_2ee4,
            atexit: 0x00af_f6f0,
        }),
        Step::Or {
            at: 0x00ca_73b0,
            bits: 0x8,
        },
    ],
};

/// `Phyre_PRendering_PModifierNetworkDynamicMeshInstance_Bind`.
pub const B_004FB8C0: Bind = Bind {
    class: 0x00ca_8020,
    steps: &[
        Step::Base {
            global: 0x00c1_9870,
            func: 0x004f_ba50,
        },
        Step::Array(Array {
            guard: (0x00ca_80e8, 0x1),
            desc_guard: (0x00ca_8184, 0x1),
            desc: 0x00ca_80f0,
            desc_ctor: 0x004f_b1e0,
            desc_vtable: 0x00b2_3a34,
            desc_atexit: 0x00af_f830,
            member: 0x00ca_80b4,
            name: 0x00b2_3980,
            off: 0xc,
            x: 0x0,
            y: 0x3,
            z: 0x4,
            vtable: 0x00b2_3954,
            atexit: 0x00af_f870,
        }),
        Step::Or {
            at: 0x00ca_80c8,
            bits: 0x8,
        },
    ],
};

/// `Phyre_PRendering_PMeshInstanceAttachPoint_Bind`.
pub const B_00501110: Bind = Bind {
    class: 0x00ca_8bd8,
    steps: &[
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00ca_8c98, 0x1),
            member: 0x00ca_8c6c,
            ty: Ty::Image(0x00c9_2090),
            name: 0x00b2_4500,
            off: 0x0,
            a: 0x2,
            b: 0x0,
            atexit: 0x00af_fa00,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00ca_8c98, 0x2),
            member: 0x00ca_8c9c,
            ty: Ty::Image(0x00ca_7228),
            name: 0x00b2_4510,
            off: 0x4,
            a: 0x2,
            b: 0x0,
            atexit: 0x00af_fa10,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00ca_8c98, 0x4),
            member: 0x00ca_8cc8,
            ty: Ty::Get(0x0043_7fc0),
            name: 0x00b2_451c,
            off: 0x8,
            a: 0x0,
            b: 0x0,
            atexit: 0x00af_fa20,
        }),
    ],
};

/// `Phyre_PSprite_PTextureAtlasInfo_Bind`.
pub const B_0056A390: Bind = Bind {
    class: 0x00cb_1e20,
    steps: &[
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_27c4, 0x1),
            member: 0x00cb_2798,
            ty: Ty::Image(0x00ca_37d0),
            name: 0x00b1_b750,
            off: 0x0,
            a: 0x2,
            b: 0x0,
            atexit: 0x00b0_2ff0,
        }),
        Step::Base {
            global: 0x00c2_4170,
            func: 0x0056_a4d0,
        },
        Step::Array(Array {
            guard: (0x00cb_27c4, 0x2),
            desc_guard: (0x00cb_2af4, 0x1),
            desc: 0x00cb_2a60,
            desc_ctor: 0x0056_6810,
            desc_vtable: 0x00b3_02dc,
            desc_atexit: 0x00b0_2d00,
            member: 0x00cb_27c8,
            name: 0x00b3_00cc,
            off: 0x4,
            x: 0x0,
            y: 0x3,
            z: 0x4,
            vtable: 0x00b3_00a0,
            atexit: 0x00b0_2fe0,
        }),
    ],
};

/// `Phyre_PGeometry_PMeshSegmentNVN_Bind`.
pub const B_0055ECA0: Bind = Bind {
    class: 0x00cb_0ff0,
    steps: &[
        Step::Base {
            global: 0x00c2_34dc,
            func: 0x0055_edf0,
        },
        Step::Array(Array {
            guard: (0x00cb_1318, 0x1),
            desc_guard: (0x00cb_14fc, 0x1),
            desc: 0x00cb_1468,
            desc_ctor: 0x0055_d460,
            desc_vtable: 0x00b2_edd4,
            desc_atexit: 0x00b0_2a70,
            member: 0x00cb_12e4,
            name: 0x00b1_6ea4,
            off: 0x28,
            x: 0x0,
            y: 0x3,
            z: 0x4,
            vtable: 0x00b2_eabc,
            atexit: 0x00b0_2ba0,
        }),
        Step::Or {
            at: 0x00cb_12f8,
            bits: 0x10,
        },
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_1318, 0x2),
            member: 0x00cb_131c,
            ty: Ty::Image(0x00cb_1120),
            name: 0x00b2_eae8,
            off: 0x30,
            a: 0x10,
            b: 0x0,
            atexit: 0x00b0_2b50,
        }),
    ],
};

/// `Phyre_PRendering_PMeshInstanceSegmentStreamBinding_Bind`.
pub const B_004F4800: Bind = Bind {
    class: 0x00ca_70f8,
    steps: &[
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00ca_72e8, 0x1),
            member: 0x00ca_72bc,
            ty: Ty::Get(0x0047_fae0),
            name: 0x00b1_3f6c,
            off: 0x0,
            a: 0xa,
            b: 0x0,
            atexit: 0x00af_f6d0,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00ca_72e8, 0x2),
            member: 0x00ca_72ec,
            ty: Ty::Image(0x00c9_1048),
            name: 0x00b1_0dc0,
            off: 0x4,
            a: 0x8,
            b: 0x0,
            atexit: 0x00af_f6a0,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00ca_72e8, 0x4),
            member: 0x00ca_7318,
            ty: Ty::Get(0x0043_7e70),
            name: 0x00b1_b8f0,
            off: 0x8,
            a: 0x8,
            b: 0x0,
            atexit: 0x00af_f6b0,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00ca_72e8, 0x8),
            member: 0x00ca_7344,
            ty: Ty::Get(0x0043_7d30),
            name: 0x00b1_b8fc,
            off: 0xa,
            a: 0x8,
            b: 0x0,
            atexit: 0x00af_f5d0,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00ca_72e8, 0x10),
            member: 0x00ca_7370,
            ty: Ty::Get(0x0043_7d30),
            name: 0x00b2_2ea4,
            off: 0xb,
            a: 0x8,
            b: 0x0,
            atexit: 0x00af_f5e0,
        }),
    ],
};

/// `Phyre_PRendering_PContextVariantFoldingTable_Bind`.
pub const B_004DC870: Bind = Bind {
    class: 0x00ca_5660,
    steps: &[
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00ca_59e8, 0x1),
            member: 0x00ca_59bc,
            ty: Ty::Get(0x0043_7fc0),
            name: 0x00b2_083c,
            off: 0x0,
            a: 0x0,
            b: 0x0,
            atexit: 0x00af_ed90,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00ca_59e8, 0x2),
            member: 0x00ca_59ec,
            ty: Ty::Get(0x0043_7fc0),
            name: 0x00b2_0854,
            off: 0x4,
            a: 0x0,
            b: 0x0,
            atexit: 0x00af_eda0,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00ca_59e8, 0x4),
            member: 0x00ca_5a18,
            ty: Ty::Get(0x0043_7fc0),
            name: 0x00b2_086c,
            off: 0x8,
            a: 0x0,
            b: 0x0,
            atexit: 0x00af_ed70,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00ca_59e8, 0x8),
            member: 0x00ca_5a44,
            ty: Ty::Get(0x0043_7fc0),
            name: 0x00b2_0884,
            off: 0xc,
            a: 0x0,
            b: 0x0,
            atexit: 0x00af_ed80,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00ca_59e8, 0x10),
            member: 0x00ca_5a70,
            ty: Ty::Get(0x0043_7fc0),
            name: 0x00b2_089c,
            off: 0x10,
            a: 0x0,
            b: 0x0,
            atexit: 0x00af_ed60,
        }),
    ],
};

/// `Phyre_PSprite_PSpriteAnimationInfoInstance_Bind`.
pub const B_0056F990: Bind = Bind {
    class: 0x00cb_3158,
    steps: &[
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_3218, 0x1),
            member: 0x00cb_31ec,
            ty: Ty::Image(0x00c9_2090),
            name: 0x00b1_0d80,
            off: 0x0,
            a: 0x2,
            b: 0x0,
            atexit: 0x00b0_3530,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_3218, 0x2),
            member: 0x00cb_321c,
            ty: Ty::Image(0x00cb_1cf0),
            name: 0x00b3_06d8,
            off: 0x4,
            a: 0x2,
            b: 0x0,
            atexit: 0x00b0_3550,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_3218, 0x4),
            member: 0x00cb_3248,
            ty: Ty::Image(0x00cb_1eb8),
            name: 0x00b3_06ec,
            off: 0x8,
            a: 0x2,
            b: 0x0,
            atexit: 0x00b0_3540,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_3218, 0x8),
            member: 0x00cb_3274,
            ty: Ty::Get(0x0043_8390),
            name: 0x00b3_0700,
            off: 0x20,
            a: 0x0,
            b: 0x0,
            atexit: 0x00b0_3510,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_3218, 0x10),
            member: 0x00cb_32a0,
            ty: Ty::Get(0x0043_8390),
            name: 0x00b3_0708,
            off: 0x21,
            a: 0x0,
            b: 0x0,
            atexit: 0x00b0_3520,
        }),
    ],
};

/// `Phyre_PAnimation_PBlendableAnimationSource_Bind`.
pub const B_00540750: Bind = Bind {
    class: 0x00ca_d9a8,
    steps: &[
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00ca_db00, 0x1),
            member: 0x00ca_dad4,
            ty: Ty::Image(0x00ca_a7e8),
            name: 0x00b2_8d6c,
            off: 0x0,
            a: 0x12,
            b: 0x0,
            atexit: 0x00b0_14d0,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00ca_db00, 0x2),
            member: 0x00ca_db04,
            ty: Ty::Image(0x00ca_ba08),
            name: 0x00b2_b794,
            off: 0xc,
            a: 0x10,
            b: 0x0,
            atexit: 0x00b0_1570,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00ca_db00, 0x4),
            member: 0x00ca_db30,
            ty: Ty::Image(0x00ca_bba8),
            name: 0x00b2_b7b0,
            off: 0x24,
            a: 0x10,
            b: 0x0,
            atexit: 0x00b0_1580,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00ca_db00, 0x8),
            member: 0x00ca_db5c,
            ty: Ty::Image(0x00ca_bf68),
            name: 0x00b2_b7cc,
            off: 0x38,
            a: 0x10,
            b: 0x0,
            atexit: 0x00b0_14f0,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00ca_db00, 0x10),
            member: 0x00ca_db88,
            ty: Ty::Get(0x0043_8060),
            name: 0x00b2_b7e4,
            off: 0x8,
            a: 0x10,
            b: 0x0,
            atexit: 0x00b0_1560,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00ca_db00, 0x20),
            member: 0x00ca_dbb4,
            ty: Ty::Get(0x0043_8060),
            name: 0x00b2_b7ec,
            off: 0x4,
            a: 0x10,
            b: 0x0,
            atexit: 0x00b0_1590,
        }),
    ],
};

/// `Phyre_PRendering_PShaderPassInfo_Bind`.
pub const B_004DCDD0: Bind = Bind {
    class: 0x00ca_55c8,
    steps: &[
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00ca_5884, 0x1),
            member: 0x00ca_5858,
            ty: Ty::Image(0x00c9_1048),
            name: 0x00b2_0798,
            off: 0x0,
            a: 0x0,
            b: 0x0,
            atexit: 0x00af_eeb0,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00ca_5884, 0x2),
            member: 0x00ca_5888,
            ty: Ty::Get(0x0043_7fc0),
            name: 0x00b2_07ac,
            off: 0x10,
            a: 0x0,
            b: 0x0,
            atexit: 0x00af_eec0,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00ca_5884, 0x4),
            member: 0x00ca_58b4,
            ty: Ty::Image(0x00c9_1048),
            name: 0x00b2_07bc,
            off: 0x4,
            a: 0x0,
            b: 0x0,
            atexit: 0x00af_ee20,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00ca_5884, 0x8),
            member: 0x00ca_58e0,
            ty: Ty::Get(0x0043_7fc0),
            name: 0x00b2_07d4,
            off: 0x14,
            a: 0x0,
            b: 0x0,
            atexit: 0x00af_ee30,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00ca_5884, 0x10),
            member: 0x00ca_590c,
            ty: Ty::Image(0x00c9_1048),
            name: 0x00b2_07e8,
            off: 0x8,
            a: 0x0,
            b: 0x0,
            atexit: 0x00af_ee40,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00ca_5884, 0x20),
            member: 0x00ca_5938,
            ty: Ty::Get(0x0043_7fc0),
            name: 0x00b2_0800,
            off: 0x18,
            a: 0x0,
            b: 0x0,
            atexit: 0x00af_ee50,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00ca_5884, 0x40),
            member: 0x00ca_5964,
            ty: Ty::Image(0x00c9_1048),
            name: 0x00b2_0814,
            off: 0xc,
            a: 0x0,
            b: 0x0,
            atexit: 0x00af_ed40,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00ca_5884, 0x80),
            member: 0x00ca_5990,
            ty: Ty::Get(0x0043_7fc0),
            name: 0x00b2_0828,
            off: 0x1c,
            a: 0x0,
            b: 0x0,
            atexit: 0x00af_ed50,
        }),
    ],
};

/// `Phyre_PSprite_PSpriteAttributes_Bind`.
pub const B_00568DC0: Bind = Bind {
    class: 0x00cb_1c58,
    steps: &[
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_2010, 0x1),
            member: 0x00cb_1fe4,
            ty: Ty::Get(0x0043_8060),
            name: 0x00b2_fe30,
            off: 0x0,
            a: 0x0,
            b: 0x0,
            atexit: 0x00b0_2f50,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_2010, 0x2),
            member: 0x00cb_2014,
            ty: Ty::Get(0x0043_8060),
            name: 0x00b2_fe38,
            off: 0x4,
            a: 0x0,
            b: 0x0,
            atexit: 0x00b0_2f60,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_2010, 0x4),
            member: 0x00cb_2040,
            ty: Ty::Get(0x0043_8060),
            name: 0x00b2_fe40,
            off: 0x8,
            a: 0x0,
            b: 0x0,
            atexit: 0x00b0_2f80,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_2010, 0x8),
            member: 0x00cb_206c,
            ty: Ty::Get(0x0043_8060),
            name: 0x00b2_fe4c,
            off: 0xc,
            a: 0x0,
            b: 0x0,
            atexit: 0x00b0_2e60,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_2010, 0x10),
            member: 0x00cb_2098,
            ty: Ty::Get(0x0043_8060),
            name: 0x00b2_fe58,
            off: 0x10,
            a: 0x0,
            b: 0x0,
            atexit: 0x00b0_3070,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_2010, 0x20),
            member: 0x00cb_20c4,
            ty: Ty::Get(0x0043_8060),
            name: 0x00b2_fe64,
            off: 0x14,
            a: 0x0,
            b: 0x0,
            atexit: 0x00b0_3090,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_2010, 0x40),
            member: 0x00cb_20f0,
            ty: Ty::Get(0x0043_8060),
            name: 0x00b2_fe70,
            off: 0x18,
            a: 0x0,
            b: 0x0,
            atexit: 0x00b0_3020,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_2010, 0x80),
            member: 0x00cb_211c,
            ty: Ty::Get(0x0043_8060),
            name: 0x00b2_fe80,
            off: 0x1c,
            a: 0x0,
            b: 0x0,
            atexit: 0x00b0_3040,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_2010, 0x100),
            member: 0x00cb_2148,
            ty: Ty::Get(0x0043_8060),
            name: 0x00b2_fe90,
            off: 0x20,
            a: 0x0,
            b: 0x0,
            atexit: 0x00b0_2fa0,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_2010, 0x200),
            member: 0x00cb_2174,
            ty: Ty::Get(0x0043_8060),
            name: 0x00b2_fea0,
            off: 0x24,
            a: 0x0,
            b: 0x0,
            atexit: 0x00b0_2fb0,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_2010, 0x400),
            member: 0x00cb_21a0,
            ty: Ty::Get(0x0043_8060),
            name: 0x00b1_e58c,
            off: 0x28,
            a: 0x0,
            b: 0x0,
            atexit: 0x00b0_2e90,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_2010, 0x800),
            member: 0x00cb_21cc,
            ty: Ty::Get(0x0043_8060),
            name: 0x00b2_feb0,
            off: 0x2c,
            a: 0x0,
            b: 0x0,
            atexit: 0x00b0_2f40,
        }),
    ],
};

/// `Phyre_PRendering_PSceneRenderPass_Bind`.
pub const B_004DCA40: Bind = Bind {
    class: 0x00ca_5530,
    steps: &[
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00ca_5758, 0x1),
            member: 0x00ca_572c,
            ty: Ty::GetArg(0x004d_9b80, 0x4),
            name: 0x00b2_0658,
            off: 0x0,
            a: 0x2,
            b: 0x0,
            atexit: 0x00af_ee60,
        }),
        Step::Base {
            global: 0x00c1_6f14,
            func: 0x004d_d2d0,
        },
        Step::Array(Array {
            guard: (0x00ca_5758, 0x2),
            desc_guard: (0x00ca_5d74, 0x1),
            desc: 0x00ca_5ce0,
            desc_ctor: 0x004d_a980,
            desc_vtable: 0x00b2_0e24,
            desc_atexit: 0x00af_ecf0,
            member: 0x00ca_575c,
            name: 0x00b2_06a4,
            off: 0x4,
            x: 0x0,
            y: 0x3,
            z: 0x4,
            vtable: 0x00b2_0678,
            atexit: 0x00af_ee90,
        }),
        Step::Base {
            global: 0x00c1_6f18,
            func: 0x004d_d4e0,
        },
        Step::Array(Array {
            guard: (0x00ca_5758, 0x4),
            desc_guard: (0x00ca_5c54, 0x1),
            desc: 0x00ca_5bc0,
            desc_ctor: 0x004d_aa20,
            desc_vtable: 0x00b2_0d1c,
            desc_atexit: 0x00af_ed10,
            member: 0x00ca_5790,
            name: 0x00b2_06f8,
            off: 0xc,
            x: 0x0,
            y: 0x3,
            z: 0x4,
            vtable: 0x00b2_06cc,
            atexit: 0x00af_ee10,
        }),
        Step::Base {
            global: 0x00c1_6f1c,
            func: 0x004d_d0c0,
        },
        Step::Array(Array {
            guard: (0x00ca_5758, 0x8),
            desc_guard: (0x00ca_5b34, 0x1),
            desc: 0x00ca_5aa0,
            desc_ctor: 0x004d_a8e0,
            desc_vtable: 0x00b2_0c14,
            desc_atexit: 0x00af_ecd0,
            member: 0x00ca_57c4,
            name: 0x00b2_075c,
            off: 0x14,
            x: 0x0,
            y: 0x3,
            z: 0x4,
            vtable: 0x00b2_0730,
            atexit: 0x00af_eea0,
        }),
        Step::Base {
            global: 0x00c1_6f20,
            func: 0x0049_1190,
        },
        Step::Array(Array {
            guard: (0x00ca_5758, 0x10),
            desc_guard: (0x00c9_a124, 0x1),
            desc: 0x00c9_a090,
            desc_ctor: 0x0048_ea60,
            desc_vtable: 0x00b1_5edc,
            desc_atexit: 0x00af_ca60,
            member: 0x00ca_57f8,
            name: 0x00b2_0774,
            off: 0x1c,
            x: 0x0,
            y: 0x3,
            z: 0x4,
            vtable: 0x00b1_5c94,
            atexit: 0x00af_ee70,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00ca_5758, 0x20),
            member: 0x00ca_582c,
            ty: Ty::Get(0x0043_8390),
            name: 0x00b2_0780,
            off: 0x24,
            a: 0x0,
            b: 0x0,
            atexit: 0x00af_ee80,
        }),
    ],
};

/// `Phyre_PRendering_PEffectVariant_Bind`.
pub const B_004E4270: Bind = Bind {
    class: 0x00ca_5ff0,
    steps: &[
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00ca_60b0, 0x1),
            member: 0x00ca_6084,
            ty: Ty::Image(0x00ca_66e8),
            name: 0x00b2_12f8,
            off: 0x0,
            a: 0x2,
            b: 0x0,
            atexit: 0x00af_efb0,
        }),
        Step::Base {
            global: 0x00c1_7724,
            func: 0x004e_49c0,
        },
        Step::Array(Array {
            guard: (0x00ca_60b0, 0x2),
            desc_guard: (0x00ca_6514, 0x1),
            desc: 0x00ca_6480,
            desc_ctor: 0x004e_31a0,
            desc_vtable: 0x00b2_174c,
            desc_atexit: 0x00af_ef40,
            member: 0x00ca_60b4,
            name: 0x00b2_134c,
            off: 0x4,
            x: 0x0,
            y: 0x3,
            z: 0x4,
            vtable: 0x00b2_1320,
            atexit: 0x00af_f020,
        }),
        Step::Base {
            global: 0x00c1_7728,
            func: 0x004e_4bd0,
        },
        Step::Array(Array {
            guard: (0x00ca_60b0, 0x4),
            desc_guard: (0x00ca_63f4, 0x1),
            desc: 0x00ca_6360,
            desc_ctor: 0x004e_3240,
            desc_vtable: 0x00b2_1644,
            desc_atexit: 0x00af_ef60,
            member: 0x00ca_60e8,
            name: 0x00b2_13a4,
            off: 0xc,
            x: 0x0,
            y: 0x3,
            z: 0x4,
            vtable: 0x00b2_1378,
            atexit: 0x00af_f010,
        }),
        Step::Base {
            global: 0x00c1_772c,
            func: 0x004e_47b0,
        },
        Step::Array(Array {
            guard: (0x00ca_60b0, 0x8),
            desc_guard: (0x00ca_62d4, 0x1),
            desc: 0x00ca_6240,
            desc_ctor: 0x004e_3100,
            desc_vtable: 0x00b2_153c,
            desc_atexit: 0x00af_ef20,
            member: 0x00ca_611c,
            name: 0x00b2_1404,
            off: 0x14,
            x: 0x0,
            y: 0x3,
            z: 0x4,
            vtable: 0x00b2_13d8,
            atexit: 0x00af_f000,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00ca_60b0, 0x10),
            member: 0x00ca_6150,
            ty: Ty::Get(0x0043_7e70),
            name: 0x00b2_141c,
            off: 0x1c,
            a: 0x0,
            b: 0x0,
            atexit: 0x00af_eff0,
        }),
        Step::Base {
            global: 0x00c1_7730,
            func: 0x004a_b0b0,
        },
        Step::Array(Array {
            guard: (0x00ca_60b0, 0x20),
            desc_guard: (0x00c9_d174, 0x1),
            desc: 0x00c9_d0e0,
            desc_ctor: 0x004a_6730,
            desc_vtable: 0x00b1_b99c,
            desc_atexit: 0x00af_d6e0,
            member: 0x00ca_617c,
            name: 0x00b1_fd48,
            off: 0x20,
            x: 0x0,
            y: 0x3,
            z: 0x4,
            vtable: 0x00b1_b89c,
            atexit: 0x00af_f040,
        }),
        Step::Base {
            global: 0x00c1_7734,
            func: 0x004a_b0b0,
        },
        Step::Array(Array {
            guard: (0x00ca_60b0, 0x40),
            desc_guard: (0x00c9_d174, 0x1),
            desc: 0x00c9_d0e0,
            desc_ctor: 0x004a_6730,
            desc_vtable: 0x00b1_b99c,
            desc_atexit: 0x00af_d6e0,
            member: 0x00ca_61b0,
            name: 0x00b2_1438,
            off: 0x28,
            x: 0x0,
            y: 0x3,
            z: 0x4,
            vtable: 0x00b1_b89c,
            atexit: 0x00af_f060,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00ca_60b0, 0x80),
            member: 0x00ca_61e4,
            ty: Ty::Get(0x0043_7e70),
            name: 0x00b2_1460,
            off: 0x30,
            a: 0x0,
            b: 0x0,
            atexit: 0x00af_f030,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00ca_60b0, 0x100),
            member: 0x00ca_6210,
            ty: Ty::Get(0x0043_7e70),
            name: 0x00b2_1480,
            off: 0x32,
            a: 0x0,
            b: 0x0,
            atexit: 0x00af_f050,
        }),
    ],
};

/// `Phyre_PRendering_PShaderPassNVN_Bind`.
pub const B_00587300: Bind = Bind {
    class: 0x00cb_3448,
    steps: &[
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_3ea0, 0x1),
            member: 0x00cb_3e74,
            ty: Ty::Image(0x00cb_34e0),
            name: 0x00b3_3048,
            off: 0x18,
            a: 0x10,
            b: 0x0,
            atexit: 0x00b0_3d60,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_3ea0, 0x2),
            member: 0x00cb_3ea4,
            ty: Ty::Image(0x00ca_1468),
            name: 0x00b3_3050,
            off: 0x194,
            a: 0x10,
            b: 0x0,
            atexit: 0x00b0_3d90,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_3ea0, 0x4),
            member: 0x00cb_3ed0,
            ty: Ty::Image(0x00ca_1468),
            name: 0x00b3_306c,
            off: 0x1ac,
            a: 0x10,
            b: 0x0,
            atexit: 0x00b0_3ca0,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_3ea0, 0x8),
            member: 0x00cb_3efc,
            ty: Ty::Image(0x00ca_1468),
            name: 0x00b3_3088,
            off: 0x1c4,
            a: 0x10,
            b: 0x0,
            atexit: 0x00b0_3cc0,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_3ea0, 0x10),
            member: 0x00cb_3f28,
            ty: Ty::Image(0x00ca_1468),
            name: 0x00b3_30a4,
            off: 0x1dc,
            a: 0x10,
            b: 0x0,
            atexit: 0x00b0_3c30,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_3ea0, 0x20),
            member: 0x00cb_3f54,
            ty: Ty::Image(0x00ca_1468),
            name: 0x00b3_30c0,
            off: 0x1f4,
            a: 0x10,
            b: 0x0,
            atexit: 0x00b0_3da0,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_3ea0, 0x40),
            member: 0x00cb_3f80,
            ty: Ty::Image(0x00ca_1468),
            name: 0x00b3_30e0,
            off: 0x20c,
            a: 0x10,
            b: 0x0,
            atexit: 0x00b0_3cb0,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_3ea0, 0x80),
            member: 0x00cb_3fac,
            ty: Ty::Image(0x00ca_1468),
            name: 0x00b3_3100,
            off: 0x224,
            a: 0x10,
            b: 0x0,
            atexit: 0x00b0_3cd0,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_3ea0, 0x100),
            member: 0x00cb_3fd8,
            ty: Ty::Image(0x00ca_1468),
            name: 0x00b3_3120,
            off: 0x23c,
            a: 0x10,
            b: 0x0,
            atexit: 0x00b0_3c40,
        }),
    ],
};

/// `Phyre_PRendering_PMeshInstanceBounds_Bind`.
pub const B_004FFA30: Bind = Bind {
    class: 0x00ca_89b8,
    steps: &[
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00ca_8a78, 0x1),
            member: 0x00ca_8a4c,
            ty: Ty::Image(0x00ca_7228),
            name: 0x00b2_437c,
            off: 0x1c,
            a: 0x2,
            b: 0x0,
            atexit: 0x00af_f990,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00ca_8a78, 0x2),
            member: 0x00ca_8a7c,
            ty: Ty::Image(0x00c9_2090),
            name: 0x00b1_069c,
            off: 0xc,
            a: 0x2,
            b: 0x0,
            atexit: 0x00af_f9d0,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00ca_8a78, 0x4),
            member: 0x00ca_8aa8,
            ty: Ty::Get(0x0043_7f10),
            name: 0x00b2_438c,
            off: 0x20,
            a: 0x0,
            b: 0x0,
            atexit: 0x00af_f9b0,
        }),
        Step::Member(Member {
            ctor: 0x0057_5f30,
            guard: (0x00ca_8a78, 0x8),
            member: 0x00ca_8ad4,
            ty: Ty::Get(0x0043_8060),
            name: 0x00b1_5b4c,
            off: 0x0,
            a: 0x0,
            b: 0x3,
            atexit: 0x00af_f9a0,
        }),
        Step::Member(Member {
            ctor: 0x0057_5f30,
            guard: (0x00ca_8a78, 0x10),
            member: 0x00ca_8b0c,
            ty: Ty::Get(0x0043_8060),
            name: 0x00b0_fb4c,
            off: 0x10,
            a: 0x0,
            b: 0x3,
            atexit: 0x00af_f9c0,
        }),
    ],
};

/// `Phyre_Bind_00502100`: three `unsigned int` members (offsets 0, 4, 8).
pub const B_00502100: Bind = Bind {
    class: 0x00ca_8d30,
    steps: &[
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00ca_8e88, 0x1),
            member: 0x00ca_8e5c,
            ty: Ty::Get(0x0043_7fc0),
            name: 0x00b1_4304,
            off: 0x0,
            a: 0x10,
            b: 0x0,
            atexit: 0x00af_fa70,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00ca_8e88, 0x2),
            member: 0x00ca_8e8c,
            ty: Ty::Get(0x0043_7fc0),
            name: 0x00b2_48b8,
            off: 0x4,
            a: 0x10,
            b: 0x0,
            atexit: 0x00af_fa80,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00ca_8e88, 0x4),
            member: 0x00ca_8eb8,
            ty: Ty::Get(0x0043_7fc0),
            name: 0x00b0_ea78,
            off: 0x8,
            a: 0x10,
            b: 0x0,
            atexit: 0x00af_fa90,
        }),
    ],
};
