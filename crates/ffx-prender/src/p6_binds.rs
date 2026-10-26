//! `Bind()` functions of list p6-render-c (continuation round): their
//! [`Bind`] tables for [`crate::h2_bind::bridge::bind`], drafted from each
//! copy's machine code by h2's `bindgen.py`
//! (`~/ffx-work/m4/port/p6c/gen_binds.py`); the oracle tests compare every
//! copy with its original.

use crate::h2_bind::{Bind, Member, Step, Ty};

/// `Phyre_PDynamicGeometry_PModifierNetworkDynamicMeshSegment_Bind`.
pub const B_005AB400: Bind = Bind {
    class: 0x00cb_b268,
    steps: &[
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_b3c0, 0x1),
            member: 0x00cb_b394,
            ty: Ty::Image(0x00cb_c420),
            name: 0x00b3_5398,
            off: 0x0,
            a: 0x12,
            b: 0x0,
            atexit: 0x00b0_4110,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_b3c0, 0x2),
            member: 0x00cb_b3c4,
            ty: Ty::Get(0x0043_7fc0),
            name: 0x00b3_53ac,
            off: 0x4,
            a: 0x10,
            b: 0x0,
            atexit: 0x00b0_4160,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_b3c0, 0x4),
            member: 0x00cb_b3f0,
            ty: Ty::Get(0x0043_7fc0),
            name: 0x00b3_53c8,
            off: 0x8,
            a: 0x10,
            b: 0x0,
            atexit: 0x00b0_4180,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_b3c0, 0x8),
            member: 0x00cb_b41c,
            ty: Ty::Get(0x0043_7fc0),
            name: 0x00b3_53e4,
            off: 0xc,
            a: 0x10,
            b: 0x0,
            atexit: 0x00b0_4150,
        }),
    ],
};

/// `Phyre_PDynamicGeometry_PModifierNetworkInstanceInput_Bind`.
pub const B_005B1600: Bind = Bind {
    class: 0x00cb_ba08,
    steps: &[
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_bbf8, 0x1),
            member: 0x00cb_bbcc,
            ty: Ty::Get(0x0043_7fc0),
            name: 0x00b2_48b8,
            off: 0x8,
            a: 0x10,
            b: 0x0,
            atexit: 0x00b0_4230,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_bbf8, 0x2),
            member: 0x00cb_bbfc,
            ty: Ty::Image(0x00cb_bff8),
            name: 0x00b3_6160,
            off: 0xc,
            a: 0x10,
            b: 0x0,
            atexit: 0x00b0_4290,
        }),
    ],
};

/// `Phyre_PDynamicGeometry_PModifierNetworkInstancePacketInput_Bind`.
pub const B_005B16E0: Bind = Bind {
    class: 0x00cb_baa0,
    steps: &[
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_bc54, 0x1),
            member: 0x00cb_bc28,
            ty: Ty::Get(0x0043_7fc0),
            name: 0x00b2_4510,
            off: 0x0,
            a: 0x10,
            b: 0x0,
            atexit: 0x00b0_42a0,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_bc54, 0x2),
            member: 0x00cb_bc58,
            ty: Ty::Get(0x0043_7fc0),
            name: 0x00b3_6170,
            off: 0x4,
            a: 0x10,
            b: 0x0,
            atexit: 0x00b0_4250,
        }),
    ],
};

/// `Phyre_PDynamicGeometry_PModifierNetworkBuffer_Bind`.
pub const B_005C0EB0: Bind = Bind {
    class: 0x00cb_c090,
    steps: &[
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_c598, 0x1),
            member: 0x00cb_c56c,
            ty: Ty::Get(0x0047_fae0),
            name: 0x00b0_f98c,
            off: 0x0,
            a: 0x12,
            b: 0x0,
            atexit: 0x00b0_4730,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_c598, 0x2),
            member: 0x00cb_c59c,
            ty: Ty::Get(0x0043_7fc0),
            name: 0x00b3_79a4,
            off: 0x4,
            a: 0x10,
            b: 0x0,
            atexit: 0x00b0_45a0,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_c598, 0x4),
            member: 0x00cb_c5c8,
            ty: Ty::Get(0x0043_7fc0),
            name: 0x00b3_7988,
            off: 0x8,
            a: 0x10,
            b: 0x0,
            atexit: 0x00b0_4690,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_c598, 0x8),
            member: 0x00cb_c5f4,
            ty: Ty::Get(0x0043_7fc0),
            name: 0x00b2_48b8,
            off: 0xc,
            a: 0x10,
            b: 0x0,
            atexit: 0x00b0_44b0,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_c598, 0x10),
            member: 0x00cb_c620,
            ty: Ty::Get(0x0043_7fc0),
            name: 0x00b3_79b0,
            off: 0x10,
            a: 0x10,
            b: 0x0,
            atexit: 0x00b0_4500,
        }),
    ],
};

/// `Phyre_PDynamicGeometry_PModifierNetworkInfoPacket_Buffer_Bind`.
pub const B_005C1390: Bind = Bind {
    class: 0x00cb_c2f0,
    steps: &[Step::Member(Member {
        ctor: 0x0057_52a0,
        guard: (0x00cb_c87c, 0x1),
        member: 0x00cb_c850,
        ty: Ty::Get(0x0043_7e70),
        name: 0x00b2_48b8,
        off: 0x0,
        a: 0x10,
        b: 0x0,
        atexit: 0x00b0_44c0,
    })],
};

/// `Phyre_PDynamicGeometry_PModifierNetworkInfoPacket_ModifierCode_Bind`.
pub const B_005C1430: Bind = Bind {
    class: 0x00cb_c1c0,
    steps: &[
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_c734, 0x1),
            member: 0x00cb_c708,
            ty: Ty::Get(0x005c_7fe0),
            name: 0x00b3_79a4,
            off: 0x0,
            a: 0x12,
            b: 0x0,
            atexit: 0x00b0_45b0,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_c734, 0x2),
            member: 0x00cb_c738,
            ty: Ty::Get(0x0043_7fc0),
            name: 0x00b2_a948,
            off: 0x4,
            a: 0x10,
            b: 0x0,
            atexit: 0x00b0_4530,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_c734, 0x4),
            member: 0x00cb_c764,
            ty: Ty::Get(0x0043_7fc0),
            name: 0x00b3_7a3c,
            off: 0x8,
            a: 0x10,
            b: 0x0,
            atexit: 0x00b0_4600,
        }),
    ],
};

/// `Phyre_PDynamicGeometry_PModifierNetworkInfoPacket_ModifierInstance_Bind`.
pub const B_005C1560: Bind = Bind {
    class: 0x00cb_c258,
    steps: &[
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_c7bc, 0x1),
            member: 0x00cb_c790,
            ty: Ty::Get(0x0043_7fc0),
            name: 0x00b3_79b8,
            off: 0x0,
            a: 0x10,
            b: 0x0,
            atexit: 0x00b0_4630,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_c7bc, 0x2),
            member: 0x00cb_c7c0,
            ty: Ty::Get(0x0043_7fc0),
            name: 0x00b3_79d0,
            off: 0x4,
            a: 0x10,
            b: 0x0,
            atexit: 0x00b0_4680,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_c7bc, 0x4),
            member: 0x00cb_c7ec,
            ty: Ty::Get(0x0043_7d30),
            name: 0x00b3_7a4c,
            off: 0x8,
            a: 0x10,
            b: 0x0,
            atexit: 0x00b0_4460,
        }),
        Step::Member(Member {
            ctor: 0x0057_5f30,
            guard: (0x00cb_c7bc, 0x8),
            member: 0x00cb_c818,
            ty: Ty::Get(0x0043_7d30),
            name: 0x00b3_7a58,
            off: 0x9,
            a: 0x0,
            b: 0x17,
            atexit: 0x00b0_4550,
        }),
        Step::Or {
            at: 0x00cb_c82c,
            bits: 0x10,
        },
    ],
};

/// `Phyre_PDynamicGeometry_PRenderStream_Bind`.
pub const B_005C16E0: Bind = Bind {
    class: 0x00cb_bff8,
    steps: &[
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_c53c, 0x1),
            member: 0x00cb_c510,
            ty: Ty::Image(0x00c9_8d18),
            name: 0x00b1_b780,
            off: 0x0,
            a: 0x12,
            b: 0x0,
            atexit: 0x00b0_44a0,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_c53c, 0x2),
            member: 0x00cb_c540,
            ty: Ty::Image(0x00c9_8730),
            name: 0x00b3_7994,
            off: 0x4,
            a: 0x12,
            b: 0x0,
            atexit: 0x00b0_4740,
        }),
    ],
};

/// `0x5c17c0`.
pub const B_005C17C0: Bind = Bind {
    class: 0x00cb_bf60,
    steps: &[
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_c4e0, 0x1),
            member: 0x00cb_c4b4,
            ty: Ty::Get(0x0043_7f10),
            name: 0x00b2_4510,
            off: 0x0,
            a: 0x10,
            b: 0x0,
            atexit: 0x00b0_4650,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x00cb_c4e0, 0x2),
            member: 0x00cb_c4e4,
            ty: Ty::Get(0x0043_7f10),
            name: 0x00b3_7988,
            off: 0x4,
            a: 0x10,
            b: 0x0,
            atexit: 0x00b0_46a0,
        }),
    ],
};

/// `Phyre_PPostProcessing_PDeferredLightingBase_Bind`.
pub const B_00A03500: Bind = Bind {
    class: 0x0194_3680,
    steps: &[
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x0194_4a7c, 0x1),
            member: 0x0194_4a50,
            ty: Ty::Get(0x0046_0070),
            name: 0x00b1_0db0,
            off: 0x3e90,
            a: 0x0,
            b: 0x0,
            atexit: 0x00b0_8ec0,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x0194_4a7c, 0x2),
            member: 0x0194_4a80,
            ty: Ty::Get(0x0043_8060),
            name: 0x00b7_60dc,
            off: 0x3ec8,
            a: 0x0,
            b: 0x0,
            atexit: 0x00b0_8fb0,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x0194_4a7c, 0x4),
            member: 0x0194_4aac,
            ty: Ty::Get(0x0043_8060),
            name: 0x00b7_60f4,
            off: 0x3ecc,
            a: 0x0,
            b: 0x0,
            atexit: 0x00b0_8fc0,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x0194_4a7c, 0x8),
            member: 0x0194_4ad8,
            ty: Ty::Get(0x0043_8060),
            name: 0x00b7_6118,
            off: 0x3eb0,
            a: 0x0,
            b: 0x0,
            atexit: 0x00b0_8f60,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x0194_4a7c, 0x10),
            member: 0x0194_4b04,
            ty: Ty::Get(0x0043_8060),
            name: 0x00b7_6128,
            off: 0x3eb4,
            a: 0x0,
            b: 0x0,
            atexit: 0x00b0_8f70,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x0194_4a7c, 0x20),
            member: 0x0194_4b30,
            ty: Ty::Get(0x0043_8060),
            name: 0x00b7_613c,
            off: 0x3eb8,
            a: 0x0,
            b: 0x0,
            atexit: 0x00b0_8f40,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x0194_4a7c, 0x40),
            member: 0x0194_4b5c,
            ty: Ty::Get(0x0046_0070),
            name: 0x00b1_0dd4,
            off: 0x3ea0,
            a: 0x0,
            b: 0x0,
            atexit: 0x00b0_8f50,
        }),
    ],
};

/// `Phyre_PPostProcessing_PDepthOfFieldBase_Bind`.
pub const B_00A037C0: Bind = Bind {
    class: 0x0194_3848,
    steps: &[
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x0194_4c88, 0x1),
            member: 0x0194_4c5c,
            ty: Ty::Get(0x0043_8060),
            name: 0x00b7_65e4,
            off: 0x14,
            a: 0x0,
            b: 0x0,
            atexit: 0x00b0_8f20,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x0194_4c88, 0x2),
            member: 0x0194_4c8c,
            ty: Ty::Get(0x0043_8060),
            name: 0x00b7_65fc,
            off: 0x18,
            a: 0x0,
            b: 0x0,
            atexit: 0x00b0_8f30,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x0194_4c88, 0x4),
            member: 0x0194_4cb8,
            ty: Ty::Get(0x0043_8060),
            name: 0x00b7_660c,
            off: 0x1c,
            a: 0x0,
            b: 0x0,
            atexit: 0x00b0_8f10,
        }),
    ],
};

/// `Phyre_PPostProcessing_PGlowBase_Bind`.
pub const B_00A03990: Bind = Bind {
    class: 0x0194_4050,
    steps: &[
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x0194_4df0, 0x1),
            member: 0x0194_4dc4,
            ty: Ty::Get(0x0043_8060),
            name: 0x00b7_69a8,
            off: 0x68,
            a: 0x0,
            b: 0x0,
            atexit: 0x00b0_8f80,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x0194_4df0, 0x2),
            member: 0x0194_4df4,
            ty: Ty::Get(0x0043_8060),
            name: 0x00b7_69bc,
            off: 0x6c,
            a: 0x0,
            b: 0x0,
            atexit: 0x00b0_8fa0,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x0194_4df0, 0x4),
            member: 0x0194_4e20,
            ty: Ty::Get(0x0043_8060),
            name: 0x00b7_69d8,
            off: 0x70,
            a: 0x0,
            b: 0x0,
            atexit: 0x00b0_8f90,
        }),
    ],
};

/// `Phyre_PPostProcessing_PMotionBlurBase_Bind`.
pub const B_00A03C20: Bind = Bind {
    class: 0x0194_3a10,
    steps: &[Step::Member(Member {
        ctor: 0x0057_52a0,
        guard: (0x0194_4d80, 0x1),
        member: 0x0194_4d54,
        ty: Ty::Get(0x0043_8060),
        name: 0x00b7_6790,
        off: 0xcd4,
        a: 0x0,
        b: 0x0,
        atexit: 0x00b0_9000,
    })],
};

/// `Phyre_PPostProcessing_PPostEffectBase_Bind`.
pub const B_00A03CE0: Bind = Bind {
    class: 0x0194_35e8,
    steps: &[
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x0194_4a20, 0x1),
            member: 0x0194_49f4,
            ty: Ty::Image(0x00ca_5268),
            name: 0x00b7_60c8,
            off: 0x8,
            a: 0x2,
            b: 0x0,
            atexit: 0x00b0_8ee0,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x0194_4a20, 0x2),
            member: 0x0194_4a24,
            ty: Ty::Get(0x0043_8390),
            name: 0x00b2_c7c4,
            off: 0x11,
            a: 0x0,
            b: 0x0,
            atexit: 0x00b0_8f00,
        }),
    ],
};

/// `Phyre_PPostProcessing_PScreenSpaceReflectionBase_Bind`.
pub const B_00A041C0: Bind = Bind {
    class: 0x0194_46d8,
    steps: &[
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x0194_4ed8, 0x1),
            member: 0x0194_4eac,
            ty: Ty::Get(0x0043_8060),
            name: 0x00b7_6b6c,
            off: 0x14,
            a: 0x0,
            b: 0x0,
            atexit: 0x00b0_8fe0,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x0194_4ed8, 0x2),
            member: 0x0194_4edc,
            ty: Ty::Get(0x0043_8060),
            name: 0x00b7_6b80,
            off: 0x18,
            a: 0x0,
            b: 0x0,
            atexit: 0x00b0_8fd0,
        }),
    ],
};

/// `Phyre_PText_PBitmapFontCharInfo_Bind`.
pub const B_00A2C350: Bind = Bind {
    class: 0x01a8_4c88,
    steps: &[
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x01a8_4de0, 0x1),
            member: 0x01a8_4db4,
            ty: Ty::Get(0x0043_7f10),
            name: 0x00b7_90d0,
            off: 0x0,
            a: 0x10,
            b: 0x0,
            atexit: 0x00b0_9130,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x01a8_4de0, 0x2),
            member: 0x01a8_4de4,
            ty: Ty::Get(0x0043_7f10),
            name: 0x00b7_90e0,
            off: 0x4,
            a: 0x10,
            b: 0x0,
            atexit: 0x00b0_91b0,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x01a8_4de0, 0x4),
            member: 0x01a8_4e10,
            ty: Ty::Get(0x0043_7f10),
            name: 0x00b7_90ec,
            off: 0x8,
            a: 0x10,
            b: 0x0,
            atexit: 0x00b0_91a0,
        }),
        Step::Member(Member {
            ctor: 0x0057_5f30,
            guard: (0x01a8_4de0, 0x8),
            member: 0x01a8_4e3c,
            ty: Ty::Get(0x0043_8060),
            name: 0x00b7_90fc,
            off: 0xc,
            a: 0x0,
            b: 0x2,
            atexit: 0x00b0_9200,
        }),
        Step::Or {
            at: 0x01a8_4e50,
            bits: 0x10,
        },
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x01a8_4de0, 0x10),
            member: 0x01a8_4e74,
            ty: Ty::Get(0x0043_8060),
            name: 0x00b1_e21c,
            off: 0x14,
            a: 0x10,
            b: 0x0,
            atexit: 0x00b0_9210,
        }),
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x01a8_4de0, 0x20),
            member: 0x01a8_4ea0,
            ty: Ty::Get(0x0043_8060),
            name: 0x00b1_1120,
            off: 0x18,
            a: 0x10,
            b: 0x0,
            atexit: 0x00b0_9180,
        }),
        Step::Member(Member {
            ctor: 0x0057_5f30,
            guard: (0x01a8_4de0, 0x40),
            member: 0x01a8_4ecc,
            ty: Ty::Get(0x0043_8060),
            name: 0x00b0_f994,
            off: 0x1c,
            a: 0x0,
            b: 0x2,
            atexit: 0x00b0_91e0,
        }),
        Step::Or {
            at: 0x01a8_4ee0,
            bits: 0x10,
        },
        Step::Member(Member {
            ctor: 0x0057_5f30,
            guard: (0x01a8_4de0, 0x80),
            member: 0x01a8_4f04,
            ty: Ty::Get(0x0043_8060),
            name: 0x00b7_9104,
            off: 0x24,
            a: 0x0,
            b: 0x2,
            atexit: 0x00b0_9100,
        }),
        Step::Or {
            at: 0x01a8_4f18,
            bits: 0x10,
        },
        Step::Member(Member {
            ctor: 0x0057_52a0,
            guard: (0x01a8_4de0, 0x100),
            member: 0x01a8_4f3c,
            ty: Ty::Get(0x0043_8390),
            name: 0x00b7_9110,
            off: 0x2c,
            a: 0x10,
            b: 0x0,
            atexit: 0x00b0_91f0,
        }),
    ],
};

/// The copies: (original, table).
pub const BINDS: &[(u32, &Bind)] = &[
    (0x005a_b400, &B_005AB400),
    (0x005b_1600, &B_005B1600),
    (0x005b_16e0, &B_005B16E0),
    (0x005c_0eb0, &B_005C0EB0),
    (0x005c_1390, &B_005C1390),
    (0x005c_1430, &B_005C1430),
    (0x005c_1560, &B_005C1560),
    (0x005c_16e0, &B_005C16E0),
    (0x005c_17c0, &B_005C17C0),
    (0x00a0_3500, &B_00A03500),
    (0x00a0_37c0, &B_00A037C0),
    (0x00a0_3990, &B_00A03990),
    (0x00a0_3c20, &B_00A03C20),
    (0x00a0_3ce0, &B_00A03CE0),
    (0x00a0_41c0, &B_00A041C0),
    (0x00a2_c350, &B_00A2C350),
];
