//! `PhyreEngine`'s arrays as the render classes hold them: `PArray<T>`
//! and `PSharray<T>` (a count word whose bit 31 marks borrowed storage,
//! then the data pointer; a `PSharray` of one element keeps it inline in
//! the data word), and the per-type helpers that set, free and index them.

use ffx_graphic::host::{Conv, Host};

use crate::descriptor::PHYRE_FREE;

/// The count bits of a count word.
pub const COUNT: u32 = 0x7fff_ffff;
/// The borrowed bit of a count word.
pub const BORROWED: u32 = 0x8000_0000;

pub mod bridge {
    //! The originals over game memory.

    use super::{BORROWED, COUNT, Conv, Host, PHYRE_FREE};
    use crate::descriptor::Init;

    /// `PArray<T>::setStorage(array, data, count)` (our name): for a count
    /// with bit 31 clear, the old storage is freed when the array owned it
    /// and it differs from `data` (`free(old, n)` cdecl, or
    /// `Phyre_PhyreFree(old)`), then `array = (count | borrowed, data)`;
    /// returns 0.
    pub fn set_storage(
        h: &mut impl Host,
        array: u32,
        data: u32,
        count: u32,
        free: Option<u32>,
    ) -> u32 {
        if count & BORROWED != 0 {
            return 0;
        }
        let old: u32 = h.read(array + 4);
        if old != data {
            let n: u32 = h.read(array);
            if n & BORROWED == 0 {
                match free {
                    Some(f) => {
                        let f = h.rebase(f);
                        h.call(f, Conv::Cdecl, &[old, n & COUNT]);
                    }
                    None => {
                        let f = h.rebase(PHYRE_FREE);
                        h.call(f, Conv::Cdecl, &[old]);
                    }
                }
            }
        }
        h.write(array + 4, data);
        h.write(array, count | BORROWED);
        0
    }

    /// `PArray<T>`'s clear `(array)`: owned, non-null storage has each of
    /// its elements destroyed (`dtor(element)` thiscall, `stride` apart)
    /// and is freed; then data and count are 0.
    pub fn clear_with_dtor(h: &mut impl Host, array: u32, dtor: u32, stride: u32) {
        let n: u32 = h.read(array);
        let data: u32 = h.read(array + 4);
        if n & BORROWED == 0 && data != 0 {
            let f = h.rebase(dtor);
            let mut e = data;
            for _ in 0..n & COUNT {
                h.call(f, Conv::Thiscall(e), &[]);
                e = e.wrapping_add(stride);
            }
            let free = h.rebase(PHYRE_FREE);
            h.call(free, Conv::Cdecl, &[data]);
        }
        h.write(array + 4, 0u32);
        h.write(array, 0u32);
    }

    /// The free of a `PSharray` at `p + off`: the storage is separate (and
    /// freed) only when owned and holding more than one element (and, when
    /// `null_check`, non-null); then data and count are 0.
    fn free_sharray(h: &mut impl Host, p: u32, null_check: bool) {
        let n: u32 = h.read(p);
        if n & BORROWED == 0 && n & COUNT > 1 {
            let data: u32 = h.read(p + 4);
            if !null_check || data != 0 {
                let f = h.rebase(PHYRE_FREE);
                h.call(f, Conv::Cdecl, &[data]);
            }
        }
        h.write(p + 4, 0u32);
        h.write(p, 0u32);
    }

    /// destructInstance `(p)` of a `PSharray` at `p + off`; `al = 1`.
    pub fn destruct_sharray(h: &mut impl Host, p: u32, off: u32, null_check: bool) -> u8 {
        free_sharray(h, p + off, null_check);
        1
    }

    /// `(items, n)`: the element free of arrays of `stride`-byte items
    /// holding a `PSharray` at `off`, then the items' storage; nothing for
    /// null `items`.
    pub fn free_items(
        h: &mut impl Host,
        items: u32,
        n: u32,
        stride: u32,
        off: u32,
        null_check: bool,
    ) {
        if items == 0 {
            return;
        }
        let mut e = items;
        for _ in 0..n {
            free_sharray(h, e + off, null_check);
            e = e.wrapping_add(stride);
        }
        let f = h.rebase(PHYRE_FREE);
        h.call(f, Conv::Cdecl, &[items]);
    }

    /// `(items, n)`: the default constructor of `n` items `stride` apart
    /// (each whose address is not null).
    pub fn init_items(h: &mut impl Host, items: u32, n: u32, stride: u32, fields: &[Init]) {
        let mut e = items;
        for _ in 0..n {
            crate::descriptor::bridge::construct(h, e, fields);
            e = e.wrapping_add(stride);
        }
    }

    /// The storage of the array at `array`: the data word for more than
    /// one element, the inline word for one, null for none.
    fn storage(m: &mut impl Host, array: u32) -> u32 {
        match m.read::<u32>(array) & COUNT {
            0 => 0,
            1 => array + 4,
            _ => m.read(array + 4),
        }
    }

    /// `PSharray<T>::operator[](array, i)`: the address of element `i`
    /// (`stride` bytes each) of the storage.
    pub fn element(m: &mut impl Host, array: u32, i: u32, stride: u32) -> u32 {
        let s = storage(m, array);
        s.wrapping_add(i.wrapping_mul(stride))
    }

    /// `0x00593af0(this, i)`: for `i` below the count at `this + 0x18`
    /// while `this + 0x34` has bit 0, the first word of the 0x14-byte
    /// element `i`; else 0.
    pub fn bound_element_word(m: &mut impl Host, this: u32, i: u32) -> u32 {
        let n = m.read::<u32>(this + 0x18) & COUNT;
        if i >= n || m.read::<u8>(this + 0x34) & 1 == 0 {
            return 0;
        }
        let e = element(m, this + 0x18, i, 0x14);
        m.read(e)
    }
}
