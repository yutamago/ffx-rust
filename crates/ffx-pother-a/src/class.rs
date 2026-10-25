//! The type system: lookups by name and offset, and class registration
//! (`Bind`: the class descriptor's buffers, the type dispenser, the
//! namespace).

use crate::{cdecl, str_eq, thiscall, with_al};
use ffx_graphic::host::Host;

/// `Phyre_PTypeDispenser_RegisterType` (cdecl `(descriptor)`).
pub const REGISTER_TYPE: u32 = 0x0043_95b0;
/// `Phyre_PClassDescriptor_addToNamespace` (thiscall `(descriptor)`).
pub const ADD_TO_NAMESPACE: u32 = 0x0043_c140;
/// `Phyre_PClassDescriptor_setDefaultBuffer` (thiscall `(d, buffer)`).
pub const SET_DEFAULT_BUFFER: u32 = 0x0043_d510;
/// `Phyre_PClassDescriptor_setValidationBuffer` (thiscall `(d, buffer)`).
pub const SET_VALIDATION_BUFFER: u32 = 0x0043_d5b0;
/// `Phyre_PClassDescriptor_setWriteMaskBuffer` (thiscall `(d, buffer)`).
pub const SET_WRITE_MASK_BUFFER: u32 = 0x0043_d5c0;
/// `PBase`'s class descriptor.
pub const PBASE_DESCRIPTOR: u32 = 0x00c9_02e8;
/// The head of the list `0x00436350` searches (`{name, next}`).
pub const NAMED_LIST: u32 = 0x00cb_3358;
/// The head of the list `0x00545840` searches (`{?, name, next}`).
pub const NAMED_LIST2: u32 = 0x00ca_e548;
/// `0x0043cbb0` (thiscall `(descriptor)`): the descriptor's type, or 0.
pub const DESCRIPTOR_TYPE: u32 = 0x0043_cbb0;
/// The three descriptors `0x00550ed0` looks up.
pub const CALLBACK_DESCRIPTORS: [u32; 3] = [0x00ca_f388, 0x00ca_ed58, 0x00ca_f540];
/// `Phyre_PNamespace_GetGlobalNamespace` (cdecl `()`).
pub const GLOBAL_NAMESPACE: u32 = 0x0043_e2f0;
/// `Phyre_PClassDescriptor_PClassDescriptor` (thiscall `(d, ns, name, a,
/// b)`).
pub const CLASS_DESCRIPTOR_INIT: u32 = 0x0043_afb0;
/// `Phyre_PClassDescriptorBase_vftable`.
pub const DESCRIPTOR_BASE_VFTABLE: u32 = 0x00b3_072c;
/// The name `0x00570670` gives its descriptor.
pub const BASE_DESCRIPTOR_NAME: u32 = 0x00b3_076c;

/// The class descriptors each `register` function (`RegisterType`, then
/// `addToNamespace`) registers: `(function, descriptor)`.
pub const REGISTERS: [(u32, u32); 11] = [
    (0x0058_7220, 0x00cb_3de0),
    (0x0058_7240, 0x00cb_3d48),
    (0x0058_7260, 0x00cb_3b68),
    (0x0058_7280, 0x00cb_3c00),
    (0x0058_72a0, 0x00cb_3908),
    (0x0058_72c0, 0x00cb_37d8),
    (0x0058_72e0, 0x00cb_3870),
    (0x0058_7cf0, 0x00cb_3cb0),
    (0x0058_7d10, 0x00cb_39a0),
    (0x0058_7d30, 0x00cb_3a38),
    (0x0058_7d50, 0x00cb_3ad0),
];

/// The calls of `0x00464400` (cdecl `()`, each returning a descriptor
/// to register when marked): `(function, register its eax)`.
pub const SHADER_PASS_INIT: [(u32, bool); 25] = [
    (0x0058_7610, false),
    (0x0058_7300, false),
    (0x0058_7a20, false),
    (0x0058_7bf0, false),
    (0x0058_7790, false),
    (0x0058_7980, false),
    (0x0058_72c0, false),
    (0x0058_72e0, false),
    (0x0058_72a0, false),
    (0x0055_e960, false),
    (0x0055_eb10, false),
    (0x0055_eca0, false),
    (0x0058_7280, false),
    (0x0058_7d10, false),
    (0x0058_7d30, false),
    (0x0058_7d50, false),
    (0x0058_7cf0, false),
    (0x0058_7240, false),
    (0x0058_7220, false),
    (0x0057_eac0, true),
    (0x0057_ee70, true),
    (0x0057_e7b0, true),
    (0x0057_f050, true),
    (0x0057_e610, true),
    (0x0058_7260, false),
];

/// `Phyre_PLOD_PUtilityLOD_initialize`'s once flag (byte).
pub const LOD_INITIALIZED: u32 = 0x00c0_a09c;
/// The object whose virtual slot 1 it calls first.
pub const LOD_PRE_INIT: u32 = 0x00c0_a37c;
/// `PTypeDispenser::GetType<PLODMetricType>`, `<PLODBlendType>`.
pub const LOD_TYPES: [u32; 2] = [0x005c_bc80, 0x005c_c0e0];
/// `PLODLevel::Bind`, `PLODGroup::Bind`.
pub const LOD_BINDS: [u32; 2] = [0x005a_3910, 0x005a_34b0];

pub mod bridge {
    //! The originals over game memory.

    use super::{
        ADD_TO_NAMESPACE, BASE_DESCRIPTOR_NAME, CALLBACK_DESCRIPTORS, CLASS_DESCRIPTOR_INIT,
        DESCRIPTOR_BASE_VFTABLE, DESCRIPTOR_TYPE, GLOBAL_NAMESPACE, Host, LOD_BINDS,
        LOD_INITIALIZED, LOD_PRE_INIT, LOD_TYPES, NAMED_LIST, NAMED_LIST2, PBASE_DESCRIPTOR,
        REGISTER_TYPE, SET_DEFAULT_BUFFER, SET_VALIDATION_BUFFER, SET_WRITE_MASK_BUFFER,
        SHADER_PASS_INIT, cdecl, str_eq, thiscall, with_al,
    };

    /// `0x004359d0` (thiscall `(class, name)`): the class's member (a
    /// list at `+0x4c`, entries `{next, ?, ?, name}`) named `name`, or 0.
    pub fn find_member(h: &mut impl Host, class: u32, name: u32) -> u32 {
        if name == 0 {
            return 0;
        }
        let head = class.wrapping_add(0x4c);
        let first: u32 = h.read(head);
        let mut e = if first == head { 0 } else { first };
        while e != 0 {
            let n: u32 = h.read(e + 0xc);
            if n != 0 && str_eq(h, n, name) {
                return e;
            }
            let next: u32 = h.read(e);
            e = if next == head { 0 } else { next };
        }
        0
    }

    /// `0x00436350(name)`: the entry `{name, next}` of the list at
    /// `0x00cb3358` named `name`, or 0.
    pub fn find_named(h: &mut impl Host, name: u32) -> u32 {
        if name == 0 {
            return 0;
        }
        let mut e: u32 = h.read(h.rebase(NAMED_LIST));
        while e != 0 {
            let n: u32 = h.read(e);
            if str_eq(h, n, name) {
                return e;
            }
            e = h.read(e + 4);
        }
        0
    }

    /// `0x00545840(name)`: the entry `{?, name, next}` of the list at
    /// `0x00cae548` named `name`, or 0.
    pub fn find_named2(h: &mut impl Host, name: u32) -> u32 {
        if name == 0 {
            return 0;
        }
        let mut e: u32 = h.read(h.rebase(NAMED_LIST2));
        while e != 0 {
            let n: u32 = h.read(e + 4);
            if str_eq(h, n, name) {
                return e;
            }
            e = h.read(e + 8);
        }
        0
    }

    /// `0x0047d2f0` (thiscall `(table, key)`): the 32-byte entry of the
    /// array `{.., +0xc count | flag, +0x10 data}` whose `+8` is `key`,
    /// or 0.
    pub fn find_by_key(h: &mut impl Host, table: u32, key: u32) -> u32 {
        let base: u32 = h.read(table + 0x10);
        let n: u32 = h.read(table + 0xc);
        let end = (n & 0x7fff_ffff).wrapping_shl(5).wrapping_add(base);
        let mut e = base;
        while e < end {
            if h.read::<u32>(e + 8) == key {
                return e;
            }
            e = e.wrapping_add(0x20);
        }
        0
    }

    /// `0x0047e4e0` (thiscall `(view, offset)`): the 32-byte entry (from
    /// `view +0x10`, `view +4`'s `+0xc` of them) whose member covers
    /// `view +4`'s `+0x14` plus `offset`: `start <= o < start + size *
    /// max(count, 1)` (signed), or 0.
    pub fn find_by_offset(h: &mut impl Host, view: u32, offset: u32) -> u32 {
        let class: u32 = h.read(view + 4);
        let mut e: u32 = h.read(view + 0x10);
        let n: u32 = h.read(class + 0xc);
        let base: u32 = h.read(class + 0x14);
        let o = base.wrapping_add(offset) as i32;
        let mut i = 0u32;
        while i < n {
            let m: u32 = h.read(e + 4);
            let count = h.read::<u32>(m + 0x14).max(1);
            let start: i32 = h.read(m + 8);
            if start <= o {
                let size: u32 = h.read(m + 0xc);
                let end = size.wrapping_mul(count).wrapping_add(start as u32) as i32;
                if end > o {
                    return e;
                }
            }
            i += 1;
            e = e.wrapping_add(0x20);
        }
        0
    }

    /// `0x00435610` `Phyre_PBase_Bind()`: `PBase`'s three buffers, then
    /// `addToNamespace` (its `eax`).
    pub fn pbase_bind(h: &mut impl Host) -> u32 {
        let d = h.rebase(PBASE_DESCRIPTOR);
        let b = h.rebase(0x00c9_037c);
        thiscall(h, SET_DEFAULT_BUFFER, d, &[b]);
        thiscall(h, SET_VALIDATION_BUFFER, d, &[b + 1]);
        thiscall(h, SET_WRITE_MASK_BUFFER, d, &[b + 2]);
        thiscall(h, ADD_TO_NAMESPACE, d, &[])
    }

    /// The `register` functions ([`super::REGISTERS`]):
    /// `RegisterType(d)`, then `addToNamespace(d)` (its `eax`).
    pub fn register(h: &mut impl Host, descriptor: u32) -> u32 {
        let d = h.rebase(descriptor);
        cdecl(h, REGISTER_TYPE, &[d]);
        thiscall(h, ADD_TO_NAMESPACE, d, &[])
    }

    /// `0x00464400()`: the shader pass classes' `Bind`s and
    /// registrations ([`super::SHADER_PASS_INIT`]); 0.
    pub fn shader_pass_init(h: &mut impl Host) -> u32 {
        for &(f, reg) in &SHADER_PASS_INIT {
            let r = cdecl(h, f, &[]);
            if reg {
                cdecl(h, REGISTER_TYPE, &[r]);
            }
        }
        0
    }

    /// `0x00504260` `Phyre_PLOD_PUtilityLOD_initialize()`: the pre-init
    /// object's slot 1 unless already initialised, the two LOD enum types,
    /// the two LOD `Bind`s; 0.
    pub fn lod_initialize(h: &mut impl Host) -> u32 {
        if h.read::<u8>(h.rebase(LOD_INITIALIZED)) == 0 {
            let o = h.rebase(LOD_PRE_INIT);
            h.call_virtual(o, 1, &[]);
        }
        for t in LOD_TYPES {
            let r = cdecl(h, t, &[]);
            cdecl(h, REGISTER_TYPE, &[r]);
        }
        for b in LOD_BINDS {
            cdecl(h, b, &[]);
        }
        0
    }

    /// `0x00550ed0(out)`: the types of the three trigger callback
    /// descriptors; when all exist, `out` (unless null) gets `{first,
    /// first +8, second, third, 0}`. `al` = whether they exist; `eax`'s
    /// rest as the original leaves it.
    pub fn callback_types(h: &mut impl Host, out: u32) -> u32 {
        let [a, b, c] = CALLBACK_DESCRIPTORS.map(|d| {
            let d = h.rebase(d);
            thiscall(h, DESCRIPTOR_TYPE, d, &[])
        });
        if a == 0 || b == 0 || c == 0 {
            return with_al(c, 0);
        }
        if out == 0 {
            return with_al(c, 1);
        }
        let a8: u32 = h.read(a + 8);
        h.write(out, a);
        h.write(out + 4, a8);
        h.write(out + 8, b);
        h.write(out + 0xc, c);
        h.write(out + 0x10, 0u32);
        with_al(a8, 1)
    }

    /// `0x00570670` (thiscall `(descriptor)`): a base class descriptor in
    /// the global namespace (`PClassDescriptor(d, ns, name, 1, 1)`); `this`.
    pub fn base_descriptor_init(h: &mut impl Host, d: u32) -> u32 {
        let ns = cdecl(h, GLOBAL_NAMESPACE, &[]);
        let name = h.rebase(BASE_DESCRIPTOR_NAME);
        thiscall(h, CLASS_DESCRIPTOR_INIT, d, &[ns, name, 1, 1]);
        let vt = h.rebase(DESCRIPTOR_BASE_VFTABLE);
        h.write(d, vt);
        h.write(d + 0x1c, 0u32);
        d
    }
}
