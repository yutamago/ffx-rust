//! `PTypeDispenser::GetType<T>`: the function-local statics that describe
//! the engine's primitive and enum types, built on first use.
//!
//! Each getter tests bit 0 of its guard word, and the first time sets it,
//! fills in the type object and registers the object's destructor with
//! `atexit` (`0x00949207`). It returns the object every time. The getters
//! differ only in their constants, listed in [`PRIMITIVES`] and [`ENUMS`]
//! (read from each getter's machine code; the oracle tests compare every
//! row with its original).
//!
//! A primitive type (`PType`, 0x30 bytes, guard at `+0x2c`):
//!
//! | offset | field |
//! |---|---|
//! | `+0` | vtable |
//! | `+4..+0xc` | a list head (three words, all `+4`) |
//! | `+0x10`, `+0x14` | 0 |
//! | `+0x18` | name |
//! | `+0x1c`, `+0x20` | size, alignment |
//! | `+0x24`, `+0x28` | the fixup functions |
//!
//! An enum type is a `PType` built by `0x005a0200` (thiscall: name, size,
//! alignment, fixups; it also links the type into the global type list),
//! then given its own vtable. Its guard is at `+0x34`.

use ffx_graphic::host::{Conv, Host};

/// The CRT's `atexit` (cdecl, one word).
pub const ATEXIT: u32 = 0x0094_9207;
/// The enum types' `PType` constructor (thiscall, five words).
pub const ENUM_TYPE_CTOR: u32 = 0x005a_0200;

/// One primitive type's getter.
#[derive(Debug, Clone, Copy)]
pub struct PrimitiveType {
    /// The getter's address.
    pub getter: u32,
    /// The static type object.
    pub object: u32,
    /// Its vtable.
    pub vtable: u32,
    /// Its name (a C string in `.rdata`).
    pub name: u32,
    /// The size of a value.
    pub size: u32,
    /// The alignment of a value.
    pub align: u32,
    /// The fixup functions (0 for none).
    pub fixups: [u32; 2],
    /// The destructor given to `atexit`.
    pub dtor: u32,
}

/// One enum type's getter.
#[derive(Debug, Clone, Copy)]
pub struct EnumType {
    /// The getter's address.
    pub getter: u32,
    /// The static type object.
    pub object: u32,
    /// Its vtable, stored after the constructor.
    pub vtable: u32,
    /// Its name.
    pub name: u32,
    /// The size of a value.
    pub size: u32,
    /// The alignment of a value.
    pub align: u32,
    /// The fixup functions: the enum's value table walk and its parser.
    pub fixups: [u32; 2],
    /// The destructor given to `atexit`.
    pub dtor: u32,
}

/// The primitive types' getters.
pub const PRIMITIVES: [PrimitiveType; 13] = [
    PrimitiveType {
        getter: 0x0043_74d0,
        object: 0x00c9_0380,
        vtable: 0x00b0_e568,
        name: 0x00b0_e694,
        size: 0x2c,
        align: 0x4,
        fixups: [0x0043_7740, 0x0043_7830],
        dtor: 0x00af_9f20,
    },
    PrimitiveType {
        getter: 0x0043_7c90,
        object: 0x00c9_03e0,
        vtable: 0x00b0_e6f0,
        name: 0x00b0_e710,
        size: 0x1,
        align: 0x1,
        fixups: [0x0000_0000, 0x0000_0000],
        dtor: 0x00af_9f40,
    },
    PrimitiveType {
        getter: 0x0043_7d30,
        object: 0x00c9_0410,
        vtable: 0x00b0_e71c,
        name: 0x00b0_e73c,
        size: 0x1,
        align: 0x1,
        fixups: [0x0000_0000, 0x0000_0000],
        dtor: 0x00af_9f50,
    },
    PrimitiveType {
        getter: 0x0043_7dd0,
        object: 0x00c9_0440,
        vtable: 0x00b0_e748,
        name: 0x00b0_e768,
        size: 0x2,
        align: 0x2,
        fixups: [0x0000_0000, 0x0000_0000],
        dtor: 0x00af_9f60,
    },
    PrimitiveType {
        getter: 0x0043_7e70,
        object: 0x00c9_0470,
        vtable: 0x00b0_e774,
        name: 0x00b0_e794,
        size: 0x2,
        align: 0x2,
        fixups: [0x0000_0000, 0x0000_0000],
        dtor: 0x00af_9f70,
    },
    PrimitiveType {
        getter: 0x0043_7f10,
        object: 0x00c9_04a0,
        vtable: 0x00b0_e7a0,
        name: 0x00b0_e7c0,
        size: 0x4,
        align: 0x4,
        fixups: [0x0000_0000, 0x0000_0000],
        dtor: 0x00af_9f80,
    },
    PrimitiveType {
        getter: 0x0043_7fc0,
        object: 0x00c9_04d0,
        vtable: 0x00b0_e6a0,
        name: 0x00b0_e7c8,
        size: 0x4,
        align: 0x4,
        fixups: [0x0000_0000, 0x0000_0000],
        dtor: 0x00af_9f90,
    },
    PrimitiveType {
        getter: 0x0043_8060,
        object: 0x00c9_0560,
        vtable: 0x00b0_e82c,
        name: 0x00b0_e84c,
        size: 0x4,
        align: 0x4,
        fixups: [0x0000_0000, 0x0000_0000],
        dtor: 0x00af_9fa0,
    },
    PrimitiveType {
        getter: 0x0043_8100,
        object: 0x00c9_0590,
        vtable: 0x00b0_e858,
        name: 0x00b0_e878,
        size: 0x8,
        align: 0x8,
        fixups: [0x0000_0000, 0x0000_0000],
        dtor: 0x00af_9fb0,
    },
    PrimitiveType {
        getter: 0x0043_81b0,
        object: 0x00c9_05c0,
        vtable: 0x00b0_e884,
        name: 0x00b0_e8a4,
        size: 0x10,
        align: 0x10,
        fixups: [0x0000_0000, 0x0000_0000],
        dtor: 0x00af_9fc0,
    },
    PrimitiveType {
        getter: 0x0043_8250,
        object: 0x00c9_0530,
        vtable: 0x00b0_e800,
        name: 0x00b0_e820,
        size: 0x8,
        align: 0x8,
        fixups: [0x0000_0000, 0x0000_0000],
        dtor: 0x00af_9fd0,
    },
    PrimitiveType {
        getter: 0x0043_82f0,
        object: 0x00c9_0500,
        vtable: 0x00b0_e7d4,
        name: 0x00b0_e7f4,
        size: 0x8,
        align: 0x8,
        fixups: [0x0000_0000, 0x0000_0000],
        dtor: 0x00af_9fe0,
    },
    PrimitiveType {
        getter: 0x0043_8390,
        object: 0x00c9_03b0,
        vtable: 0x00b0_e6c4,
        name: 0x00b0_e6e4,
        size: 0x1,
        align: 0x1,
        fixups: [0x0000_0000, 0x0000_0000],
        dtor: 0x00af_9ff0,
    },
];

/// The enum types' getters.
pub const ENUMS: [EnumType; 9] = [
    EnumType {
        getter: 0x0050_78c0,
        object: 0x00ca_973c,
        vtable: 0x00b2_4fdc,
        name: 0x00b2_4fc0,
        size: 0x10,
        align: 0x4,
        fixups: [0x0050_7aa0, 0x0050_7b50],
        dtor: 0x00b0_0070,
    },
    EnumType {
        getter: 0x005c_bc80,
        object: 0x00cb_d90c,
        vtable: 0x00b3_85a0,
        name: 0x00b3_855c,
        size: 0xc,
        align: 0x4,
        fixups: [0x005c_beb0, 0x005c_bf20],
        dtor: 0x00b0_48d0,
    },
    EnumType {
        getter: 0x005c_c0e0,
        object: 0x00cb_d948,
        vtable: 0x00b3_85f4,
        name: 0x00b3_85d4,
        size: 0xc,
        align: 0x4,
        fixups: [0x005c_c310, 0x005c_c380],
        dtor: 0x00b0_4920,
    },
    EnumType {
        getter: 0x009f_4730,
        object: 0x0194_35a0,
        vtable: 0x00b7_3f28,
        name: 0x00b7_3da8,
        size: 0xc,
        align: 0x4,
        fixups: [0x009f_4960, 0x009f_49d0],
        dtor: 0x00b0_8640,
    },
    EnumType {
        getter: 0x00a2_dce0,
        object: 0x01a8_531c,
        vtable: 0x00b7_944c,
        name: 0x00b7_93e4,
        size: 0x10,
        align: 0x4,
        fixups: [0x00a2_df10, 0x00a2_df80],
        dtor: 0x00b0_9360,
    },
    EnumType {
        getter: 0x00a2_e150,
        object: 0x01a8_5358,
        vtable: 0x00b7_96cc,
        name: 0x00b7_9484,
        size: 0x10,
        align: 0x4,
        fixups: [0x00a2_e380, 0x00a2_e3f0],
        dtor: 0x00b0_98f0,
    },
    EnumType {
        getter: 0x00a2_e5c0,
        object: 0x01a8_5394,
        vtable: 0x00b7_9b68,
        name: 0x00b7_9704,
        size: 0x10,
        align: 0x4,
        fixups: [0x00a2_e7f0, 0x00a2_e860],
        dtor: 0x00b0_a520,
    },
    EnumType {
        getter: 0x00a2_ea30,
        object: 0x01a8_53d0,
        vtable: 0x00b7_9bf0,
        name: 0x00b7_9ba0,
        size: 0x10,
        align: 0x4,
        fixups: [0x00a2_ec60, 0x00a2_ecd0],
        dtor: 0x00b0_a5b0,
    },
    EnumType {
        getter: 0x00a3_23a0,
        object: 0x01a8_557c,
        vtable: 0x00b7_9fd8,
        name: 0x00b7_9fbc,
        size: 0x10,
        align: 0x4,
        fixups: [0x00a3_25d0, 0x00a3_2640],
        dtor: 0x00b0_b820,
    },
];

/// `PTypeDispenser`'s own static (`0x00439040`): six list words, all
/// pointing at the object; guard at `+0x18`.
pub const DISPENSER: u32 = 0x00c9_05f0;
/// Its destructor given to `atexit`.
pub const DISPENSER_DTOR: u32 = 0x00af_9f30;
/// The asset tree's registry static (`0x0045d420`), the same shape.
pub const ASSET_REGISTRY: u32 = 0x00c9_424c;
/// Its destructor given to `atexit`.
pub const ASSET_REGISTRY_DTOR: u32 = 0x00af_af70;

/// The getter at `getter`'s primitive row.
#[must_use]
pub fn primitive(getter: u32) -> Option<&'static PrimitiveType> {
    PRIMITIVES.iter().find(|t| t.getter == getter)
}

/// The getter at `getter`'s enum row.
#[must_use]
pub fn enum_type(getter: u32) -> Option<&'static EnumType> {
    ENUMS.iter().find(|t| t.getter == getter)
}

/// The ports over game memory.
pub mod bridge {
    use super::{
        ATEXIT, Conv, DISPENSER, DISPENSER_DTOR, ENUM_TYPE_CTOR, EnumType, Host, PrimitiveType,
    };

    /// An image address where the image is (0 stays 0: no fixup).
    fn at(h: &impl Host, sym: u32) -> u32 {
        if sym == 0 { 0 } else { h.rebase(sym) }
    }

    /// Sets bit 0 of the guard at `guard`; whether it was clear (the
    /// first call).
    fn first_time(h: &mut impl Host, guard: u32) -> bool {
        let g: u32 = h.read(guard);
        if g & 1 != 0 {
            return false;
        }
        h.write(guard, g | 1);
        true
    }

    /// `PTypeDispenser::GetType<T>()` for a primitive type: builds the
    /// object the first time; returns it.
    pub fn primitive(h: &mut impl Host, t: &PrimitiveType) -> u32 {
        let o = h.rebase(t.object);
        if first_time(h, o + 0x2c) {
            let list = o + 4;
            h.write(list, [list, list, list, 0, 0]);
            let words = [
                at(h, t.name),
                t.size,
                t.align,
                at(h, t.fixups[0]),
                at(h, t.fixups[1]),
            ];
            h.write(o + 0x18, words);
            let vt = h.rebase(t.vtable);
            h.write(o, vt);
            let dtor = h.rebase(t.dtor);
            h.call_sym(ATEXIT, Conv::Cdecl, &[dtor]);
        }
        o
    }

    /// `PTypeDispenser::GetType<T>()` for an enum type: builds the object
    /// the first time (`0x005a0200`, then its vtable); returns it.
    pub fn enum_type(h: &mut impl Host, t: &EnumType) -> u32 {
        let o = h.rebase(t.object);
        if first_time(h, o + 0x34) {
            let args = [
                at(h, t.name),
                t.size,
                t.align,
                at(h, t.fixups[0]),
                at(h, t.fixups[1]),
            ];
            h.call_sym(ENUM_TYPE_CTOR, Conv::Thiscall(o), &args);
            let vt = h.rebase(t.vtable);
            h.write(o, vt);
            let dtor = h.rebase(t.dtor);
            h.call_sym(ATEXIT, Conv::Cdecl, &[dtor]);
        }
        o
    }

    /// A static of six list words that all point at it, guard at `+0x18`
    /// (the type dispenser `0x00439040`: [`DISPENSER`]; the asset
    /// registry `0x0045d420`: [`super::ASSET_REGISTRY`]): the words the
    /// first time; returns it.
    pub fn list_static(h: &mut impl Host, object: u32, dtor: u32) -> u32 {
        let o = h.rebase(object);
        if first_time(h, o + 0x18) {
            h.write(o, [o; 6]);
            let dtor = h.rebase(dtor);
            h.call_sym(ATEXIT, Conv::Cdecl, &[dtor]);
        }
        o
    }

    /// The type dispenser's static (`0x00439040`).
    pub fn dispenser(h: &mut impl Host) -> u32 {
        list_static(h, DISPENSER, DISPENSER_DTOR)
    }
}
