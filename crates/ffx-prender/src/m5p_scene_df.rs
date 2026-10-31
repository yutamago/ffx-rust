//! List `phyre-scene`'s varargs forwarder (M5 stage 1, round
//! "design-frames"): `0x0044e9e0(member, object, ...)`, the value of a
//! class member read through its accessor (slot 2 of the object at
//! `member + 0x14`) into a 16-byte aligned buffer of its frame, handed the
//! address of a `va_list` over its own variable arguments
//! (`Phyre_PEntity_lookup_0044f6d0` asks it for `getLocalToWorldMatrix`).
//!
//! The `va_list` points into the caller's argument words, which the shim
//! has as `slots` (docs/porting.md, "Run-time frames and varargs"); the
//! bridge takes it and the two frame words by address.

/// The type the accessor is asked for: `0x00c92090` with bit 0 set.
pub const VALUE_TYPE: u32 = 0x00c9_2090;
/// The bytes of the aligned value buffer the shim provides (the original
/// keeps 24 bytes at `ebp - 0x1c` and rounds their start up to 16).
pub const VALUE_BYTES: u32 = 0x18;

/// The originals over game memory.
pub mod bridge {
    use super::VALUE_TYPE;
    use ffx_graphic::host::{Conv, Host};

    /// `0x0044e9e0(member, object, ...)` (cdecl) -> `eax`, with `va` the
    /// address of its first variable argument (the caller's third word),
    /// `cell` its `va_list` word (`ebp - 0x20`) and `value` its aligned
    /// buffer: `cell = va`; with an accessor at `member + 0x14`, its slot
    /// 2 `(object, VALUE_TYPE | 1, value, &cell)` (thiscall, ret 0x10); the
    /// buffer's second word.
    pub fn member_value(
        h: &mut impl Host,
        member: u32,
        object: u32,
        va: u32,
        cell: u32,
        value: u32,
    ) -> u32 {
        h.write(cell, va);
        let accessor: u32 = h.read(member.wrapping_add(0x14));
        if accessor != 0 {
            let vt: u32 = h.read(accessor);
            let f: u32 = h.read(vt.wrapping_add(8));
            let ty = h.rebase(VALUE_TYPE) | 1;
            h.call(f, Conv::Thiscall(accessor), &[object, ty, value, cell]);
        }
        // junk read: the buffer is never written without an accessor; 0
        // there (brief 2026-10-24: the frame starts zeroed)
        h.read(value.wrapping_add(4))
    }
}
