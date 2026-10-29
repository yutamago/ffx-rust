//! M5 stage 1, lists `tmpl-members-phyrearray-a` and `-b`: the vtable
//! slots of the member descriptors for `PArray<T>` / `PSharray<T>` members
//! (`PClassDataMemberPhyreArray<T>`, `PClassDataMemberPhyreSharray<T>`),
//! and the array methods they reach.
//!
//! A member descriptor locates its array in an object at `object +
//! [this+0x24] - [[this+0xc]+0x88]` (the member's offset less its class's
//! base; [`bridge::member_array`]). The array header is `[count | bit 31
//! for an external block, data]`; a `PSharray` keeps a single element in
//! the data word itself. All slots are thiscall on the member
//! descriptor, at the slots of `PClassDataMemberArrayDynamic`
//! (`m5p_tmpl_members_dynamic`):
//!
//! - slot 0, the scalar deleting destructor `(flags)`, `ret 4`:
//!   `PClassDataMemberArray`'s destructor `0x00575f70`, `PhyreFree` when
//!   `flags` bit 0 is set; `this` ([`DTORS`]).
//! - slot 4 `getCount(object)`: the count without bit 31 ([`GET_COUNT`]).
//! - slot 5 `setCount(object, count)`, `ret 8`: the array's own resize
//!   (thiscall on the array, `ret 4`), its `eax` ([`SET_COUNT`]).
//! - slots 6 and 7 `getArrayStart(object)`: the data word
//!   ([`GET_START`]); a `PSharray`'s inline element for a count of 1, 0
//!   for none ([`SHARRAY_START`]).
//! - slot 8 `setToExternalBuffer(object, buffer, count)`, `ret 0xc`: 5
//!   for a negative count; else an owned old block other than `buffer` is
//!   released ([`Release`]), then `[count | bit 31, buffer]`; 0
//!   ([`SET_EXTERNAL`]). Some copies forward to the array's own method
//!   instead ([`SET_EXTERNAL_FORWARD`], [`SET_EXTERNAL_MARKED`]).
//! - slot 9, the element type: a descriptor global ([`ELEMENT_TYPE`]) or
//!   a tail jump to the type's `GetType` (cdecl, no arguments;
//!   [`ELEMENT_TYPE_JUMP`]).
//!
//! The constant of each copy is in the generated tables
//! (`m5p_tmpl_members_phyrearray_table.rs.in`, by the copy's address);
//! image addresses there are rebased before use.

use ffx_bridge::Mem;
use ffx_graphic::host::{Conv, Host};

/// `PhyreFree(block)` (cdecl).
pub const FREE: u32 = 0x0042_fb20;
/// `PClassDataMemberArray`'s destructor (thiscall, no stack words).
pub const MEMBER_ARRAY_DESTRUCT: u32 = 0x0057_5f70;

/// How an owned array block is released by `setToExternalBuffer`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Release {
    /// `PhyreFree(data)`, skipped for a 0 `data` when `check_null`.
    Free {
        /// Whether a 0 `data` is skipped.
        check_null: bool,
    },
    /// `f(data, count)` (cdecl): destroys the elements, frees the block.
    Elements(u32),
}

include!("m5p_tmpl_members_phyrearray_table.rs.in");

pub use crate::m5p_tmpl_concrete::{listed, word_of};

/// The release of [`SET_EXTERNAL`] copy `f`.
#[must_use]
pub const fn release_of(f: u32) -> Release {
    let mut i = 0;
    while i < SET_EXTERNAL.len() {
        if SET_EXTERNAL[i].0 == f {
            return SET_EXTERNAL[i].1;
        }
        i += 1;
    }
    panic!("not in the table");
}

/// Whether the array header word `count` owns its block (bit 31 clear).
#[must_use]
pub const fn owned(count: u32) -> bool {
    count >> 31 == 0
}

pub mod bridge {
    //! The generic ports over game memory.

    use super::{Conv, FREE, Host, Mem, Release, owned};

    pub use crate::m5p_tmpl_concrete::bridge::dtor_without_vtable;

    /// The array a member descriptor `this` locates in `object`: `object +
    /// [this+0x24] - [[this+0xc]+0x88]`.
    pub fn member_array(m: &impl Mem, this: u32, object: u32) -> u32 {
        let class: u32 = m.read(this.wrapping_add(0xc));
        let off: u32 = m.read(this.wrapping_add(0x24));
        let base: u32 = m.read(class.wrapping_add(0x88));
        off.wrapping_sub(base).wrapping_add(object)
    }

    /// Slots 6/7 `getArrayStart(object)`: the array's data word.
    pub fn array_start(m: &impl Mem, this: u32, object: u32) -> u32 {
        let a = member_array(m, this, object);
        m.read(a.wrapping_add(4))
    }

    /// Slot 4 `getCount(object)`: the count without bit 31.
    pub fn array_count(m: &impl Mem, this: u32, object: u32) -> u32 {
        let a = member_array(m, this, object);
        m.read::<u32>(a) & 0x7fff_ffff
    }

    /// Slots 6/7 of a `PSharray`: the data word for a count above 1, the
    /// inline element (`array + 4`) for 1, 0 for none.
    pub fn sharray_start(m: &impl Mem, this: u32, object: u32) -> u32 {
        let a = member_array(m, this, object);
        let count = m.read::<u32>(a) & 0x7fff_ffff;
        if count > 1 {
            m.read(a.wrapping_add(4))
        } else if count != 0 {
            a.wrapping_add(4)
        } else {
            0
        }
    }

    /// Slot 5 `setCount(object, count)`: the array's resize `resize`
    /// (thiscall on the array, one stack word); its `eax`.
    pub fn array_set_count(
        h: &mut impl Host,
        this: u32,
        object: u32,
        count: u32,
        resize: u32,
    ) -> u32 {
        let a = member_array(h, this, object);
        h.call_sym(resize, Conv::Thiscall(a), &[count])
    }

    /// Slot 8 `setToExternalBuffer(object, buffer, count)`: 5 for a
    /// negative count; else an owned old block other than `buffer`
    /// released as `how`, then `buffer` with `count | bit 31`; 0.
    pub fn set_external(
        h: &mut impl Host,
        this: u32,
        object: u32,
        buffer: u32,
        count: u32,
        how: Release,
    ) -> u32 {
        let a = member_array(h, this, object);
        if (count as i32) < 0 {
            return 5;
        }
        let data: u32 = h.read(a.wrapping_add(4));
        let word: u32 = h.read(a);
        if data != buffer && owned(word) {
            match how {
                Release::Free { check_null } => {
                    if data != 0 || !check_null {
                        h.call_sym(FREE, Conv::Cdecl, &[data]);
                    }
                }
                Release::Elements(f) => {
                    h.call_sym(f, Conv::Cdecl, &[data, word & 0x7fff_ffff]);
                }
            }
        }
        h.write(a.wrapping_add(4), buffer);
        h.write(a, count | 0x8000_0000);
        0
    }

    /// Slot 8 forwarding to the array's own `setToExternalBuffer` `f`
    /// (thiscall on the array, `(buffer, count)`); its `eax`.
    pub fn set_external_forward(
        h: &mut impl Host,
        this: u32,
        object: u32,
        buffer: u32,
        count: u32,
        f: u32,
    ) -> u32 {
        let a = member_array(h, this, object);
        h.call_sym(f, Conv::Thiscall(a), &[buffer, count])
    }

    /// Slot 8 through the array's buffer setter `f` (thiscall on the
    /// array, `(buffer, count | bit 31)`): 5 for a negative count, else 0.
    pub fn set_external_marked(
        h: &mut impl Host,
        this: u32,
        object: u32,
        buffer: u32,
        count: u32,
        f: u32,
    ) -> u32 {
        if (count as i32) < 0 {
            return 5;
        }
        let word = count | 0x8000_0000;
        let a = member_array(h, this, object);
        h.call_sym(f, Conv::Thiscall(a), &[buffer, word]);
        0
    }

    /// Slot 9: the element type's descriptor global `d` (rebased).
    pub fn element_type(h: &impl Host, d: u32) -> u32 {
        h.rebase(d)
    }

    /// Slot 9 as a tail jump to the type's `GetType` `f` (cdecl, no
    /// arguments); its `eax`.
    pub fn element_type_jump(h: &mut impl Host, f: u32) -> u32 {
        h.call_sym(f, Conv::Cdecl, &[])
    }
}
