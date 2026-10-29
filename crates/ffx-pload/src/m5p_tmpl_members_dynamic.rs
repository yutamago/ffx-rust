//! M5 stage 1, list `tmpl-members-dynamic`: the vtable slots of the
//! `PClassDataMemberArrayDynamic<T>` member descriptors (an array member
//! whose class supplies its own accessors), and three slots of their base
//! `PClassDataMemberArray` they share with the `PhyreArray`/`PhyreSharray`
//! member descriptors.
//!
//! All are thiscall on the member descriptor. Fahrenheit's Switch symbols
//! list the class's own virtuals as the destructor, `getCount`,
//! `setCount`, `getArrayStart` (twice) and `setToExternalBuffer`, which
//! puts them at slots 0, 4, 5, 6/7 and 8:
//!
//! - slot 0, the scalar deleting destructor `(flags)`, `ret 4`:
//!   `PClassDataMemberArray`'s destructor (`0x00575f70`, no vtable stored
//!   first), `PhyreFree` when `flags` bit 0 is set; `this`
//!   ([`DTORS`], [`crate::m5p_tmpl_concrete::bridge::dtor_without_vtable`]).
//! - slot 4 `getCount(object)`, `ret 4`: the class's getter stored at
//!   `+0x34` (thiscall on `object`), 0 without one ([`GET_COUNT`]).
//! - slot 5 `setCount(object, count)`, `ret 8`: 0x17, not supported
//!   ([`SET_COUNT`]).
//! - slots 6 and 7 `getArrayStart(object)`, `ret 4`: the getter at `+0x38`,
//!   or the word at the member's offset in `object` ([`GET_START`]).
//! - slot 8 `setToExternalBuffer(object, buffer, count)`, `ret 0xc`: the
//!   setter at `+0x3c` (thiscall on `object`), 0x15 without one
//!   ([`SET_EXTERNAL`]).
//!
//! The three `PClassDataMemberArray` slots: 2 and 3 hand the count, the
//! start and slot 9's result to the script layer's array helpers
//! ([`bridge::script_get_array`], [`bridge::script_put_array`]); 10 is
//! the member's flag bit 1 ([`bridge::flag_bit1`]).
//!
//! The getters reuse the generic ports of M4's list h1-render-a
//! (`ffx_prender::class_slots::bridge`), which hooked a few copies of the
//! same shapes. The generated table (`m5p_tmpl_members_dynamic_table.rs.in`)
//! lists every copy by address; the slots have no per-copy constant but
//! the destructor's callee.

use ffx_bridge::Mem;
use ffx_graphic::host::{Conv, Host};

/// `PClassDataMemberArray`'s destructor (thiscall, no stack words).
pub const MEMBER_ARRAY_DESTRUCT: u32 = 0x0057_5f70;
/// The script helper slot 2 hands the array to (cdecl, four words).
pub const SCRIPT_GET: u32 = 0x0043_63b0;
/// Slot 3's helper for members whose slot 10 is set (cdecl, four words).
pub const SCRIPT_PUT_FLAGGED: u32 = 0x0043_6f20;
/// Slot 3's helper otherwise (cdecl, four words).
pub const SCRIPT_PUT: u32 = 0x0043_6e20;
/// `PClassDataMemberArray` slot 2.
pub const ARRAY_SLOT2: u32 = 0x0057_6060;
/// `PClassDataMemberArray` slot 10.
pub const ARRAY_SLOT10: u32 = 0x0057_60a0;
/// `PClassDataMemberArray` slot 3.
pub const ARRAY_SLOT3: u32 = 0x0057_60b0;

include!("m5p_tmpl_members_dynamic_table.rs.in");

pub use crate::m5p_tmpl_concrete::{listed, word_of};

pub mod bridge {
    //! The generic ports over game memory.

    use super::{Conv, Host, Mem, SCRIPT_GET, SCRIPT_PUT, SCRIPT_PUT_FLAGGED};

    pub use crate::m5p_tmpl_concrete::bridge::dtor_without_vtable;
    pub use ffx_prender::class_slots::bridge::{member_count, member_set_external, member_start};

    /// Slot 5 `setCount(object, count)`: 0x17 (not supported).
    #[must_use]
    pub const fn set_count_unsupported() -> u32 {
        0x17
    }

    /// `PClassDataMemberArray` slot 10: bit 1 of the member's flags
    /// (`+0x14`).
    pub fn flag_bit1(m: &impl Mem, this: u32) -> u32 {
        let flags: u32 = m.read(this.wrapping_add(0x14));
        (flags >> 1) & 1
    }

    /// The member's virtual `slot` (thiscall on the member, `args` on the
    /// stack); its `eax`.
    fn virtual_call(h: &mut impl Host, this: u32, slot: u32, args: &[u32]) -> u32 {
        let vtable: u32 = h.read(this);
        let f: u32 = h.read(vtable.wrapping_add(4 * slot));
        h.call(f, Conv::Thiscall(this), args)
    }

    /// `PClassDataMemberArray` slot 2 `(state, object)`: `0x004363b0(state,
    /// getCount(object), getArrayStart(object) (slot 6), slot 9())`; its
    /// `eax`.
    pub fn script_get_array(h: &mut impl Host, this: u32, state: u32, object: u32) -> u32 {
        let count = virtual_call(h, this, 4, &[object]);
        let start = virtual_call(h, this, 6, &[object]);
        let element = virtual_call(h, this, 9, &[]);
        h.call_sym(SCRIPT_GET, Conv::Cdecl, &[state, count, start, element])
    }

    /// `PClassDataMemberArray` slot 3 `(state, object)`: `getCount(object)`,
    /// `getArrayStart(object)` (slot 7), then by slot 10's `al`
    /// `0x00436f20` or `0x00436e20` `(state, count, start, slot 9())`; its
    /// `eax`.
    pub fn script_put_array(h: &mut impl Host, this: u32, state: u32, object: u32) -> u32 {
        let count = virtual_call(h, this, 4, &[object]);
        let start = virtual_call(h, this, 7, &[object]);
        let flagged = virtual_call(h, this, 10, &[]) & 0xff != 0;
        let element = virtual_call(h, this, 9, &[]);
        let f = if flagged {
            SCRIPT_PUT_FLAGGED
        } else {
            SCRIPT_PUT
        };
        h.call_sym(f, Conv::Cdecl, &[state, count, start, element])
    }
}
