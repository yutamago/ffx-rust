//! M5 finish, WP5 part 2: the class descriptor constructors of one
//! template (`0x005dd540` and 132 copies found in `FFX.exe` by their
//! instruction sequence, `~/ffx-work/m5/finish/wp5b/descr_scan.py`): the
//! no-Unicorn boot reached them from the static initialisers. Each, thiscall
//! on the descriptor:
//!
//! `PBase`'s descriptor as the parent (`Phyre_PBase_GetClassDescriptor`),
//! `PClassDescriptor`'s constructor `(the global namespace, the name, the
//! size, the alignment, the parent, the flags)`, the flags word `+0x90`
//! with bit 1, the vtable, then the default, validation and write-mask
//! buffers (`setDefaultBuffer`, `setValidationBuffer`,
//! `setWriteMaskBuffer`); `this`. The per-copy constants are in
//! [`DESCRIPTORS`] (generated, `wp5_descr_table.rs.in`).

use ffx_graphic::host::{Conv, Host};

/// One copy's constants (image addresses, rebased before use).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Descriptor {
    pub name: u32,
    pub size: u32,
    pub align: u32,
    pub flags: u32,
    pub vtable: u32,
    pub default: u32,
    pub validation: u32,
    pub write_mask: u32,
}

include!("wp5_descr_table.rs.in");

/// The callees: `Phyre_PBase_GetClassDescriptor`,
/// `Phyre_PNamespace_GetGlobalNamespace` (cdecl), the base constructor and
/// the three buffer setters (thiscall).
pub const BASE_DESCRIPTOR: u32 = 0x0043_5740;
pub const GLOBAL_NAMESPACE: u32 = 0x0043_e2f0;
pub const DESCRIPTOR_CTOR: u32 = 0x0043_b0a0;
pub const SET_DEFAULT: u32 = 0x0043_d510;
pub const SET_VALIDATION: u32 = 0x0043_d5b0;
pub const SET_WRITE_MASK: u32 = 0x0043_d5c0;

/// The [`DESCRIPTORS`] row of `f`.
#[must_use]
pub const fn descriptor_of(f: u32) -> Descriptor {
    let mut i = 0;
    while i < DESCRIPTORS.len() {
        if DESCRIPTORS[i].0 == f {
            return DESCRIPTORS[i].1;
        }
        i += 1;
    }
    panic!("not in the table");
}

/// The constructor of `d` on `this`. `this`.
pub fn construct(h: &mut impl Host, this: u32, d: &Descriptor) -> u32 {
    let parent = h.call_sym(BASE_DESCRIPTOR, Conv::Cdecl, &[]);
    let ns = h.call_sym(GLOBAL_NAMESPACE, Conv::Cdecl, &[]);
    let name = h.rebase(d.name);
    h.call_sym(
        DESCRIPTOR_CTOR,
        Conv::Thiscall(this),
        &[ns, name, d.size, d.align, parent, d.flags],
    );
    let flags: u32 = h.read(this.wrapping_add(0x90));
    h.write(this.wrapping_add(0x90), flags | 2);
    let vt = h.rebase(d.vtable);
    h.write(this, vt);
    for (f, b) in [
        (SET_DEFAULT, d.default),
        (SET_VALIDATION, d.validation),
        (SET_WRITE_MASK, d.write_mask),
    ] {
        let b = h.rebase(b);
        h.call_sym(f, Conv::Thiscall(this), &[b]);
    }
    this
}
