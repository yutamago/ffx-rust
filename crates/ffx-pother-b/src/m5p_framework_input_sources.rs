//! `PInputs::PInputSource*` (list `phyre-framework-input`, batch 2): the
//! sources' `clone` methods and the two constructors that look up their
//! device.
//!
//! A source is `vtable`, `+4` its class's type record, `+8` its name,
//! `+0xc` the device id word, `+0x10` a flag byte, `+0x14` the scale
//! (`f32`), `+0x18` the device's kind or value, `+0x1c` the mapped
//! device, and for the two-key sources `+0x20`/`+0x24` the keys.
//!
//! `clone` comes in three shapes:
//! - [`PLAIN_CLONES`] (sixteen copies of one body): `PhyreMalloc(0x18)`,
//!   the five words copied with the copy's own `+4`/`+8`/vtable;
//! - [`PAIR_CLONES`] (four): `PhyreMalloc(0x28)`, the same plus
//!   `+0x20`/`+0x24`, then the class's semantic check on the copy;
//! - [`CTOR_CLONES`] (two): `PhyreMalloc(0x20)` and the class's
//!   constructor with the source's map, id, flag and scale.
//!
//! The originals guard the allocation with an SEH frame (the constructor
//! may throw): no port can install one, and nothing in the game throws
//! there; the frame leaves nothing behind when no exception is raised.

use ffx_bridge::Mem;
use ffx_graphic::host::{Conv, Host};

use crate::m5p_framework_input_small::float_copy;
pub use ffx_pload::m5p_tmpl_fortype::Lazy;

/// `Phyre::PhyreMalloc(size)` (cdecl).
pub const PHYRE_MALLOC: u32 = 0x0042_fb40;
/// The joypad-axis source's map getter (`[ecx + 0xc]`, thiscall).
pub const AXIS_MAP_GETTER: u32 = 0x00a2_e0c0;
/// The default device object a mouse-button source falls back on.
pub const DEFAULT_MOUSE_DEVICE: u32 = 0x00c6_c3cc;
/// `PInputSourceJoypadAxis`'s vtable.
pub const JOYPAD_AXIS_VTABLE: u32 = 0x00b7_0c28;
/// `PInputSourceMouseButton`'s vtable.
pub const MOUSE_BUTTON_VTABLE: u32 = 0x00b7_0d28;
/// The joypad-axis constructor's `+4` and `+8`.
pub const JOYPAD_AXIS_TYPE: [u32; 2] = [0x0194_0c98, 0x00c6_8240];
/// The mouse-button constructor's `+4` and `+8`.
pub const MOUSE_BUTTON_TYPE: [u32; 2] = [0x0194_0d30, 0x00c6_824c];

/// A clone copy's constants (image addresses, relocated).
#[derive(Debug, Clone, Copy)]
pub struct CloneKind {
    /// `+4`: the class's type record.
    pub type_record: u32,
    /// `+8`: the class's name.
    pub name: u32,
    pub vtable: u32,
    /// The semantic check run on a pair clone (thiscall), or 0.
    pub check: u32,
}

const fn plain(type_record: u32, name: u32, vtable: u32) -> CloneKind {
    CloneKind {
        type_record,
        name,
        vtable,
        check: 0,
    }
}

/// The sixteen copies of the plain `clone` (motion and touch sources).
pub const PLAIN_CLONES: &[(u32, CloneKind)] = &[
    (0x009e_b980, plain(0x0194_14e8, 0x00c6_82e8, 0x00b7_1a38)),
    (0x009e_b9d0, plain(0x0194_1580, 0x00c6_82f4, 0x00b7_1b40)),
    (0x009e_ba20, plain(0x0194_1618, 0x00c6_8300, 0x00b7_1c48)),
    (0x009e_ba70, plain(0x0194_16b0, 0x00c6_830c, 0x00b7_1d50)),
    (0x009e_bac0, plain(0x0194_1748, 0x00c6_8318, 0x00b7_1e60)),
    (0x009e_bb10, plain(0x0194_17e0, 0x00c6_8324, 0x00b7_1f70)),
    (0x009e_bb60, plain(0x0194_1450, 0x00c6_82dc, 0x00b7_1938)),
    (0x009e_bbb0, plain(0x0194_1288, 0x00c6_82b8, 0x00b7_1638)),
    (0x009e_bc00, plain(0x0194_1320, 0x00c6_82c4, 0x00b7_1738)),
    (0x009e_bc50, plain(0x0194_13b8, 0x00c6_82d0, 0x00b7_1838)),
    (0x009e_be80, plain(0x0194_1028, 0x00c6_8270, 0x00b7_1438)),
    (0x009e_bed0, plain(0x0194_10c0, 0x00c6_827c, 0x00b7_1538)),
    (0x009e_bf20, plain(0x0194_0f90, 0x00c6_82a0, 0x00b7_1128)),
    (0x009e_bf70, plain(0x0194_0ef8, 0x00c6_82ac, 0x00b7_1028)),
    (0x009e_bfd0, plain(0x0194_1158, 0x00c6_8288, 0x00b7_1228)),
    (0x009e_c020, plain(0x0194_11f0, 0x00c6_8294, 0x00b7_1330)),
];

/// The four copies of the two-key `clone` (joypad button, key, mouse
/// deltas).
pub const PAIR_CLONES: &[(u32, CloneKind)] = &[
    (
        0x009e_b820,
        CloneKind {
            type_record: 0x0194_0c00,
            name: 0x00c6_8234,
            vtable: 0x00b7_0b28,
            check: 0x009f_0350,
        },
    ),
    (
        0x009e_b8d0,
        CloneKind {
            type_record: 0x0194_0b68,
            name: 0x00c6_8228,
            vtable: 0x00b7_0a30,
            check: 0x009f_03c0,
        },
    ),
    (
        0x009e_bd20,
        CloneKind {
            type_record: 0x0194_0dc8,
            name: 0x00c6_8258,
            vtable: 0x00b7_0e28,
            check: 0x009f_0470,
        },
    ),
    (
        0x009e_bdd0,
        CloneKind {
            type_record: 0x0194_0e60,
            name: 0x00c6_8264,
            vtable: 0x00b7_0f28,
            check: 0x009f_04f0,
        },
    ),
];

/// The two `clone`s through a constructor: `(function, constructor)`.
pub const CTOR_CLONES: &[(u32, u32)] = &[(0x009e_b7a0, 0x009e_1420), (0x009e_bca0, 0x009e_1910)];

/// `PArray<PInputMap*>`'s lazily built swap helpers (the template of list
/// tmpl-fortype, ported generically by
/// `ffx_pload::m5p_tmpl_fortype::bridge::swap_lazy`): `(function, copy)`.
/// Both share one static descriptor and differ in the flag.
pub const SWAP_LAZY: &[(u32, Lazy)] = &[
    (0x00a3_2c60, input_map_lazy(1)),
    (0x00a3_2d10, input_map_lazy(0)),
];

const fn input_map_lazy(flag: u32) -> Lazy {
    Lazy {
        guard: 0x01a8_5744,
        desc: 0x01a8_56b0,
        ctor: 0x00a3_2ef0,
        flags: 0x01a8_5740,
        vtable: 0x00b7_a294,
        at_exit: 0x00b0_b860,
        flag,
    }
}

/// The [`SWAP_LAZY`] entry of copy `f`.
///
/// # Panics
/// At compile time when `f` is not in the table.
#[must_use]
pub const fn lazy_of(f: u32) -> Lazy {
    let mut i = 0;
    while i < SWAP_LAZY.len() {
        if SWAP_LAZY[i].0 == f {
            return SWAP_LAZY[i].1;
        }
        i += 1;
    }
    panic!("not a lazy swap of the list");
}

/// The copy of `f` in `table`.
///
/// # Panics
/// At compile time when `f` is not in the table.
#[must_use]
pub const fn clone_of(table: &[(u32, CloneKind)], f: u32) -> CloneKind {
    let mut i = 0;
    while i < table.len() {
        if table[i].0 == f {
            return table[i].1;
        }
        i += 1;
    }
    panic!("not a clone of the table");
}

/// The constructor `f` of [`CTOR_CLONES`] calls.
///
/// # Panics
/// At compile time when `f` is not in the table.
#[must_use]
pub const fn ctor_of(f: u32) -> u32 {
    let mut i = 0;
    while i < CTOR_CLONES.len() {
        if CTOR_CLONES[i].0 == f {
            return CTOR_CLONES[i].1;
        }
        i += 1;
    }
    panic!("not a constructor clone");
}

/// Whether a device kind is a mouse button (0xb, 0xc, 0xd).
#[must_use]
pub const fn is_mouse_kind(kind: u32) -> bool {
    matches!(kind, 0xb..=0xd)
}

pub mod bridge {
    use super::{
        AXIS_MAP_GETTER, CloneKind, Conv, DEFAULT_MOUSE_DEVICE, Host, JOYPAD_AXIS_TYPE,
        JOYPAD_AXIS_VTABLE, MOUSE_BUTTON_TYPE, MOUSE_BUTTON_VTABLE, Mem, PHYRE_MALLOC, float_copy,
        is_mouse_kind,
    };

    /// The common head: `+4`, `+8` (relocated), `+0xc`, the byte `+0x10`,
    /// the scale `+0x14` (through `fld`/`fstp`), the vtable.
    fn copy_head(h: &mut impl Mem, k: &CloneKind, src: u32, dst: u32) {
        let id: u32 = h.read(src + 0xc);
        let flag: u8 = h.read(src + 0x10);
        let scale: u32 = h.read(src + 0x14);
        let tr = h.rebase(k.type_record);
        let name = h.rebase(k.name);
        let vt = h.rebase(k.vtable);
        h.write(dst + 0x14, float_copy(scale));
        h.write(dst + 4, tr);
        h.write(dst + 8, name);
        h.write(dst + 0xc, id);
        h.write(dst + 0x10, flag);
        h.write(dst, vt);
    }

    /// A plain `clone` (thiscall, no stack words): the copy, or 0 when the
    /// allocation fails.
    pub fn plain_clone(h: &mut impl Host, k: CloneKind, this: u32) -> u32 {
        let copy = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[0x18]);
        if copy == 0 {
            return 0;
        }
        copy_head(h, &k, this, copy);
        copy
    }

    /// A two-key `clone`: the head, `+0x20`/`+0x24`, then the class's
    /// semantic check on the copy; the copy, or 0.
    pub fn pair_clone(h: &mut impl Host, k: CloneKind, this: u32) -> u32 {
        let copy = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[0x28]);
        if copy == 0 {
            return 0;
        }
        let a: u32 = h.read(this + 0x20);
        let b: u32 = h.read(this + 0x24);
        copy_head(h, &k, this, copy);
        h.write(copy + 0x20, a);
        h.write(copy + 0x24, b);
        h.call_sym(k.check, Conv::Thiscall(copy), &[]);
        copy
    }

    /// A `clone` through the class's constructor `(map +0x1c, id +0xc,
    /// flag +0x10 zero-extended, scale +0x14)`: its `eax`, or 0 when the
    /// allocation fails.
    pub fn ctor_clone(h: &mut impl Host, ctor: u32, this: u32) -> u32 {
        let mem = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[0x20]);
        if mem == 0 {
            return 0;
        }
        let map: u32 = h.read(this + 0x1c);
        let id: u32 = h.read(this + 0xc);
        let flag: u8 = h.read(this + 0x10);
        let scale: u32 = h.read(this + 0x14);
        h.call_sym(
            ctor,
            Conv::Thiscall(mem),
            &[map, id, u32::from(flag), float_copy(scale)],
        )
    }

    /// The constructors' common head (`+0xc`, `+0x14`, `+4`, `+8`,
    /// `+0x10`, the vtable, `+0x1c` = `map`).
    fn ctor_head(h: &mut impl Mem, ty: [u32; 2], vtable: u32, this: u32, args: [u32; 4]) {
        let [map, id, flag, scale] = args;
        h.write(this + 0xc, id);
        h.write(this + 0x14, float_copy(scale));
        let tr = h.rebase(ty[0]);
        let name = h.rebase(ty[1]);
        h.write(this + 4, tr);
        h.write(this + 8, name);
        h.write(this + 0x10, flag as u8);
        let vt = h.rebase(vtable);
        h.write(this, vt);
        h.write(this + 0x1c, map);
    }

    /// `PInputSourceJoypadAxis::PInputSourceJoypadAxis(map, id, flag,
    /// scale)` (`0x009e1420`, `ret 0x10`): the head, `+0x18` = the map's
    /// getter (0 without a map); `this`.
    pub fn joypad_axis_ctor(h: &mut impl Host, this: u32, args: [u32; 4]) -> u32 {
        ctor_head(h, JOYPAD_AXIS_TYPE, JOYPAD_AXIS_VTABLE, this, args);
        let map = args[0];
        let v = if map == 0 {
            0
        } else {
            h.call_sym(AXIS_MAP_GETTER, Conv::Thiscall(map), &[])
        };
        h.write(this + 0x18, v);
        this
    }

    /// The mouse-button rule (`0x009f0430` and the end of `0x009e1910`):
    /// a device at `+0x1c` that is not a mouse button (kind by its
    /// vtable slot 1) is replaced by [`DEFAULT_MOUSE_DEVICE`]; `+0x18` =
    /// the device's kind. Returns that kind (the original's `eax`).
    pub fn mouse_button_refresh(h: &mut impl Host, this: u32) -> u32 {
        let dev: u32 = h.read(this + 0x1c);
        let keep = dev != 0 && is_mouse_kind(h.call_virtual(dev, 1, &[]));
        if !keep {
            let d = h.rebase(DEFAULT_MOUSE_DEVICE);
            h.write(this + 0x1c, d);
        }
        // never 0 here (the default is an image address), so the
        // original's `+0x18 = 0` branch cannot run
        let dev: u32 = h.read(this + 0x1c);
        let kind = h.call_virtual(dev, 1, &[]);
        h.write(this + 0x18, kind);
        kind
    }

    /// `PInputSourceMouseButton::PInputSourceMouseButton(map, id, flag,
    /// scale)` (`0x009e1910`, `ret 0x10`): the head, then
    /// [`mouse_button_refresh`]; `this`.
    pub fn mouse_button_ctor(h: &mut impl Host, this: u32, args: [u32; 4]) -> u32 {
        ctor_head(h, MOUSE_BUTTON_TYPE, MOUSE_BUTTON_VTABLE, this, args);
        mouse_button_refresh(h, this);
        this
    }
}
