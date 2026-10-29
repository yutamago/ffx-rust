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
//! The array methods (module [`arrays`]) are the resizes those slots
//! call (thiscall on the array, `(count)`, `ret 4`): 0 when the header word
//! already equals `count`; else the block is reallocated keeping
//! `min(count, old count)` elements (by a reallocating helper,
//! [`RESIZE_REALLOC`], or `PhyreMalloc` plus a copy and an element
//! constructor, [`RESIZE_MALLOC`]), 0xd when that fails; an owned old block
//! other than the new one is released; then `[count, block]`; 0. Plus the
//! `PSharray` versions (a single element inline), the element
//! constructors (cdecl `(elements, n)`, [`ELEMENTS`]) and the two slots of
//! `PClassDataMemberDynamicType<PAssetReference>`.
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

/// `PhyreMalloc(size, alignment)` (cdecl).
pub const MALLOC: u32 = 0x0042_fb80;
/// `PBase::GetClassDescriptor()` (cdecl).
pub const PBASE_DESCRIPTOR: u32 = 0x0043_5740;

/// A resize that allocates with `PhyreMalloc`: the element size, the
/// cdecl `copy(new, old, kept)`, the cdecl element constructor
/// `ctor(first new element, n)` (0 for none) and how the old block is
/// released.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Grow {
    /// Bytes per element.
    pub size: u32,
    /// `copy(new, old, kept)` (cdecl).
    pub copy: u32,
    /// `ctor(first, n)` (cdecl) for the elements past the kept ones, or 0.
    pub ctor: u32,
    /// How an owned old block is released.
    pub release: Release,
}

/// `(offset, word)` stores of an element constructor.
pub type Words = &'static [(u32, u32)];

/// `PSharray<PString>::resize` (`0x0044c790`).
pub const SHARRAY_STRING_RESIZE: u32 = 0x0044_c790;
/// `PSharray<PString>`'s block helper (`0x0044c220`, thiscall `(n, old,
/// kept)`, `ret 0xc`).
pub const SHARRAY_STRING_ALLOC: u32 = 0x0044_c220;
/// The cdecl copy `0x0044c220` hands the kept elements to.
pub const SHARRAY_STRING_COPY: u32 = 0x0044_a470;
/// The cdecl release of a `PSharray<PString>` block `(data, count)`.
pub const STRING_RELEASE: u32 = 0x0043_4360;
/// `PSharray<PPhysicsShape *>`'s block helper (copies inline).
pub const SHARRAY_ALLOC_INLINE: u32 = 0x005f_3f40;
/// `PSharray<PMeshInstance *>::setToExternalBuffer(buffer, count)`.
pub const SHARRAY_SET_EXTERNAL: u32 = 0x005a_8060;
/// Its override that keeps a single inline element through the global
/// object `0x00ca7228`'s slot 1 and `0x0043c160`.
pub const SHARRAY_SET_EXTERNAL_LOD: u32 = 0x005a_8250;
/// The global object `0x005a8250` asks (slot 1, thiscall, no stack
/// words).
pub const LOD_OBJECT: u32 = 0x00ca_7228;
/// The thiscall `(element, 1)` `0x005a8250` calls on that object's
/// answer (`ret 8`).
pub const LOD_SET_ONE: u32 = 0x0043_c160;
/// `PClassDataMemberDynamicType<PAssetReference>`'s slot 0.
pub const DYNAMIC_TYPE_DTOR: u32 = 0x0045_c2c0;
/// Its vtable, base destructor (thiscall) and free (cdecl).
pub const DYNAMIC_TYPE_DTOR_PARTS: [u32; 3] = [0x00b1_1ca8, 0x0057_54f0, 0x0063_0390];
/// `PClassDataMemberDynamicType<PAssetReference>`'s slot 1.
pub const DYNAMIC_TYPE_SLOT1: u32 = 0x0045_dea0;
/// The `PTypedObject` element constructor (`[e] = 0`, `[e+4] =
/// PBase::GetClassDescriptor()`, 8 bytes each).
pub const TYPED_OBJECT_ELEMENTS: u32 = 0x0045_90f0;
/// The `PDataBlockD3D11` resize (its block goes to a setter, not into
/// the header).
pub const DATA_BLOCK_RESIZE: u32 = 0x0056_3d60;
/// Its size, copy, element constructor and the setter (thiscall on the
/// array, `(block, count)`, `ret 8`).
pub const DATA_BLOCK_RESIZE_PARTS: [u32; 4] = [0x40, 0x0048_6120, 0x0043_43c0, 0x0043_4e40];

include!("m5p_tmpl_members_phyrearray_table.rs.in");

/// The helper and release of [`RESIZE_REALLOC`] copy `f`.
#[must_use]
pub const fn realloc_of(f: u32) -> (u32, Release) {
    let mut i = 0;
    while i < RESIZE_REALLOC.len() {
        if RESIZE_REALLOC[i].0 == f {
            return RESIZE_REALLOC[i].1;
        }
        i += 1;
    }
    panic!("not in the table");
}

/// The parts of [`RESIZE_MALLOC`] copy `f`.
#[must_use]
pub const fn grow_of(f: u32) -> Grow {
    let mut i = 0;
    while i < RESIZE_MALLOC.len() {
        if RESIZE_MALLOC[i].0 == f {
            return RESIZE_MALLOC[i].1;
        }
        i += 1;
    }
    panic!("not in the table");
}

/// The stride and stores of [`ELEMENTS`] copy `f`.
#[must_use]
pub const fn elements_of(f: u32) -> (u32, Words) {
    let mut i = 0;
    while i < ELEMENTS.len() {
        if ELEMENTS[i].0 == f {
            return ELEMENTS[i].1;
        }
        i += 1;
    }
    panic!("not in the table");
}

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

pub mod arrays {
    //! The array methods: resizes, block helpers, element constructors.

    use super::{
        Conv, FREE, Grow, Host, LOD_OBJECT, LOD_SET_ONE, MALLOC, Mem, PBASE_DESCRIPTOR, Release,
        SHARRAY_SET_EXTERNAL, SHARRAY_STRING_ALLOC, SHARRAY_STRING_COPY, STRING_RELEASE, owned,
    };

    const MASK: u32 = 0x7fff_ffff;

    /// Releases the owned old block `data` of header word `word` as `how`.
    fn release(h: &mut impl Host, how: Release, data: u32, word: u32) {
        match how {
            Release::Free { check_null } => {
                if data != 0 || !check_null {
                    h.call_sym(FREE, Conv::Cdecl, &[data]);
                }
            }
            Release::Elements(f) => {
                h.call_sym(f, Conv::Cdecl, &[data, word & MASK]);
            }
        }
    }

    /// The new block replaces the old one: an owned old block other than
    /// `block` released, then `[count, block]`; 0.
    fn replace(h: &mut impl Host, array: u32, count: u32, block: u32, how: Release) -> u32 {
        let data: u32 = h.read(array.wrapping_add(4));
        let word: u32 = h.read(array);
        if data != block && owned(word) {
            release(h, how, data, word);
        }
        h.write(array.wrapping_add(4), block);
        h.write(array, count);
        0
    }

    /// `PArray<T>::resize(count)` through a reallocating helper
    /// `realloc(count, data, kept)` (cdecl, the new block); 0, 0xd when
    /// it gives none for a count other than 0.
    pub fn resize_realloc(
        h: &mut impl Host,
        array: u32,
        count: u32,
        realloc: u32,
        how: Release,
    ) -> u32 {
        let word: u32 = h.read(array);
        if count == word {
            return 0;
        }
        let kept = count.min(word & MASK);
        let data: u32 = h.read(array.wrapping_add(4));
        let block = h.call_sym(realloc, Conv::Cdecl, &[count, data, kept]);
        if block == 0 && count != 0 {
            return 0xd;
        }
        replace(h, array, count, block, how)
    }

    /// The new block of a `PhyreMalloc` resize: none for a count of 0;
    /// else `PhyreMalloc(count * size, 4)` (`None` when that fails), the
    /// kept elements copied in and the rest constructed.
    fn grow_block(
        h: &mut impl Host,
        array: u32,
        count: u32,
        size: u32,
        copy: u32,
        ctor: u32,
    ) -> Option<u32> {
        let word: u32 = h.read(array);
        let kept = count.min(word & MASK);
        let data: u32 = h.read(array.wrapping_add(4));
        if count == 0 {
            return Some(0);
        }
        let block = h.call_sym(MALLOC, Conv::Cdecl, &[count.wrapping_mul(size), 4]);
        if block == 0 {
            return None;
        }
        h.call_sym(copy, Conv::Cdecl, &[block, data, kept]);
        if ctor != 0 {
            let first = block.wrapping_add(kept.wrapping_mul(size));
            h.call_sym(ctor, Conv::Cdecl, &[first, count.wrapping_sub(kept)]);
        }
        Some(block)
    }

    /// `PArray<T>::resize(count)` with `PhyreMalloc` ([`Grow`]); 0, 0xd
    /// when the allocation fails.
    pub fn resize_malloc(h: &mut impl Host, array: u32, count: u32, grow: Grow) -> u32 {
        let word: u32 = h.read(array);
        if count == word {
            return 0;
        }
        let Some(block) = grow_block(h, array, count, grow.size, grow.copy, grow.ctor) else {
            return 0xd;
        };
        replace(h, array, count, block, grow.release)
    }

    /// `PArray<PDataBlockD3D11>::resize(count)`: the `PhyreMalloc` resize
    /// whose new block goes to `setter(array; block, count)` (thiscall);
    /// 0, 0xd when the allocation fails.
    pub fn resize_malloc_set(
        h: &mut impl Host,
        array: u32,
        count: u32,
        [size, copy, ctor, setter]: [u32; 4],
    ) -> u32 {
        let word: u32 = h.read(array);
        if count == word {
            return 0;
        }
        let Some(block) = grow_block(h, array, count, size, copy, ctor) else {
            return 0xd;
        };
        h.call_sym(setter, Conv::Thiscall(array), &[block, count]);
        0
    }

    /// A `PSharray`'s elements: the block for a count above 1, the inline
    /// word (`array + 4`) for 1, 0 for none.
    fn sharray_data(m: &impl Mem, array: u32, word: u32) -> u32 {
        let n = word & MASK;
        if n > 1 {
            m.read(array.wrapping_add(4))
        } else if n != 0 {
            array.wrapping_add(4)
        } else {
            0
        }
    }

    /// `PSharray<T *>::resize(count)` through the block helper `alloc(array;
    /// count, data, kept)` (thiscall, `ret 0xc`): 0, 0xd when it gives no
    /// block for a count other than 0; an owned old block other than the
    /// new one and the inline word freed; the new block stored unless it
    /// is the inline word.
    pub fn sharray_resize(h: &mut impl Host, array: u32, count: u32, alloc: u32) -> u32 {
        let word: u32 = h.read(array);
        if count == word {
            return 0;
        }
        let data = sharray_data(h, array, word);
        let kept = count.min(word & MASK);
        let block = h.call_sym(alloc, Conv::Thiscall(array), &[count, data, kept]);
        if block == 0 && count != 0 {
            return 0xd;
        }
        let inline = array.wrapping_add(4);
        if data != block && data != inline && owned(h.read(array)) {
            h.call_sym(FREE, Conv::Cdecl, &[data]);
        }
        if block != inline {
            h.write(inline, block);
        }
        h.write(array, count);
        0
    }

    /// `PSharray<PString>::resize(count)`: as [`sharray_resize`] with the
    /// strings' helper (`0x0044c220`); an old inline string frees its
    /// block when the block's bit 0 is clear, an old owned block goes to
    /// the strings' release `(data, old count)`.
    pub fn sharray_string_resize(h: &mut impl Host, array: u32, count: u32) -> u32 {
        let word: u32 = h.read(array);
        let n = word & MASK;
        if count == word {
            return 0;
        }
        let data = sharray_data(h, array, word);
        let kept = count.min(n);
        let block = h.call_sym(
            SHARRAY_STRING_ALLOC,
            Conv::Thiscall(array),
            &[count, data, kept],
        );
        if block == 0 && count != 0 {
            return 0xd;
        }
        let inline = array.wrapping_add(4);
        if data != block {
            if data == inline {
                let string: u32 = h.read(data);
                if string & 1 == 0 {
                    h.call_sym(FREE, Conv::Cdecl, &[string]);
                }
            } else if owned(h.read(array)) {
                h.call_sym(STRING_RELEASE, Conv::Cdecl, &[data, n]);
            }
        }
        if block != inline {
            h.write(inline, block);
        }
        h.write(array, count);
        0
    }

    /// `PSharray<PString>`'s block helper `(count, data, kept)` on
    /// `array`: the inline word for a count up to 1, else `PhyreMalloc(4
    /// count, 4)` (0 when that fails); the kept elements copied in
    /// (`0x0044a470(block, data, kept)`, cdecl), the rest zeroed; the
    /// block.
    pub fn sharray_string_alloc(
        h: &mut impl Host,
        array: u32,
        count: u32,
        data: u32,
        kept: u32,
    ) -> u32 {
        let block = if count > 1 {
            h.call_sym(MALLOC, Conv::Cdecl, &[count.wrapping_mul(4), 4])
        } else {
            array.wrapping_add(4)
        };
        if block == 0 {
            return 0;
        }
        h.call_sym(SHARRAY_STRING_COPY, Conv::Cdecl, &[block, data, kept]);
        let mut left = count.wrapping_sub(kept);
        let mut e = block.wrapping_add(kept.wrapping_mul(4));
        while left != 0 {
            if e != 0 {
                h.write(e, 0u32);
            }
            e = e.wrapping_add(4);
            left = left.wrapping_sub(1);
        }
        block
    }

    /// `PSharray<PPhysicsShape *>`'s block helper `(count, data, kept)`:
    /// as [`sharray_string_alloc`] with the kept words copied inline.
    pub fn sharray_alloc_inline(
        h: &mut impl Host,
        array: u32,
        count: u32,
        data: u32,
        kept: u32,
    ) -> u32 {
        let block = if count > 1 {
            h.call_sym(MALLOC, Conv::Cdecl, &[count.wrapping_mul(4), 4])
        } else {
            array.wrapping_add(4)
        };
        if block == 0 {
            return 0;
        }
        let mut src = data;
        let mut dst = block;
        for _ in 0..kept {
            if dst != 0 {
                let w: u32 = h.read(src);
                h.write(dst, w);
                src = src.wrapping_add(4);
            }
            dst = dst.wrapping_add(4);
        }
        let rest = count.wrapping_sub(kept);
        let mut e = block.wrapping_add(kept.wrapping_mul(4));
        for _ in 0..rest {
            h.write(e, 0u32);
            e = e.wrapping_add(4);
        }
        block
    }

    /// `PSharray<PMeshInstance *>::setToExternalBuffer(buffer, count)`: 5
    /// for a negative count; a count of 1 keeps `buffer`'s word inline
    /// (header 1), any other uses `buffer` (header `count | bit 31`); an
    /// owned old block other than the new one and the inline word is
    /// freed; 0.
    pub fn sharray_set_external(h: &mut impl Host, array: u32, buffer: u32, count: u32) -> u32 {
        if (count as i32) < 0 {
            return 5;
        }
        let word: u32 = h.read(array);
        let data = sharray_data(h, array, word);
        let inline = array.wrapping_add(4);
        let (block, header) = if count == 1 {
            if inline != 0 {
                let w: u32 = h.read(buffer);
                h.write(inline, w);
            }
            (inline, 1)
        } else {
            (buffer, count | 0x8000_0000)
        };
        if data != block && data != inline && owned(h.read(array)) {
            h.call_sym(FREE, Conv::Cdecl, &[data]);
        }
        if block != inline {
            h.write(inline, block);
        }
        h.write(array, header);
        0
    }

    /// The override of [`sharray_set_external`] on `array`: the global
    /// object `0x00ca7228`'s slot 1 (thiscall, no stack words); for
    /// `buffer` the inline word, a count of 1 and an answer, `0x0043c160(
    /// answer; inline, 1)` (thiscall); else `0x005a8060(array; buffer,
    /// count)`; 0.
    pub fn sharray_set_external_lod(h: &mut impl Host, array: u32, buffer: u32, count: u32) -> u32 {
        let object = h.rebase(LOD_OBJECT);
        let vtable: u32 = h.read(object);
        let f: u32 = h.read(vtable.wrapping_add(4));
        let answer = h.call(f, Conv::Thiscall(object), &[]);
        let inline = array.wrapping_add(4);
        if buffer == inline && count == 1 && answer != 0 {
            h.call_sym(LOD_SET_ONE, Conv::Thiscall(answer), &[inline, 1]);
        } else {
            h.call_sym(
                SHARRAY_SET_EXTERNAL,
                Conv::Thiscall(array),
                &[buffer, count],
            );
        }
        0
    }

    /// An element constructor `(elements, n)` (cdecl): `words` stored in
    /// each `stride`-byte element, skipping an element at address 0.
    pub fn construct_elements(
        m: &mut impl Mem,
        elements: u32,
        n: u32,
        stride: u32,
        words: &[(u32, u32)],
    ) {
        let mut e = elements;
        for _ in 0..n {
            if e != 0 {
                for &(off, w) in words {
                    m.write(e.wrapping_add(off), w);
                }
            }
            e = e.wrapping_add(stride);
        }
    }

    /// The `PTypedObject` element constructor `(elements, n)`: `[e] = 0`,
    /// `[e+4] = PBase::GetClassDescriptor()` in each 8-byte element
    /// (skipping one at address 0).
    pub fn construct_typed_objects(h: &mut impl Host, elements: u32, n: u32) {
        for i in 0..n {
            let e = elements.wrapping_add(i.wrapping_mul(8));
            if e != 0 {
                h.write(e, 0u32);
                let d = h.call_sym(PBASE_DESCRIPTOR, Conv::Cdecl, &[]);
                h.write(e.wrapping_add(4), d);
            }
        }
    }

    /// `PClassDataMemberDynamicType<PAssetReference>`'s scalar deleting
    /// destructor: its vtable, its base destructor (thiscall), its free
    /// (cdecl) when `flags` bit 0 is set; `this`.
    pub fn dtor_with(
        h: &mut impl Host,
        this: u32,
        flags: u32,
        [vtable, destruct, free]: [u32; 3],
    ) -> u32 {
        let vt = h.rebase(vtable);
        h.write(this, vt);
        h.call_sym(destruct, Conv::Thiscall(this), &[]);
        if flags & 1 != 0 {
            h.call_sym(free, Conv::Cdecl, &[this]);
        }
        this
    }

    /// `PClassDataMemberDynamicType<PAssetReference>`'s slot 1
    /// `(object)`: the getter at `+8` (thiscall on `object + [this+0xc]`)
    /// when set and its answer is not 0; else `[[this+4]+0x18]`.
    pub fn dynamic_type_descriptor(h: &mut impl Host, this: u32, object: u32) -> u32 {
        let f: u32 = h.read(this.wrapping_add(8));
        if f != 0 {
            let off: u32 = h.read(this.wrapping_add(0xc));
            let r = h.call(f, Conv::Thiscall(off.wrapping_add(object)), &[]);
            if r != 0 {
                return r;
            }
        }
        let t: u32 = h.read(this.wrapping_add(4));
        h.read(t.wrapping_add(0x18))
    }
}
