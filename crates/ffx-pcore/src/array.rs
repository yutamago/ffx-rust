//! `PArray` / `PSharray` helpers: assignment, comparison, bit fields, and
//! the small forwarders around them.
//!
//! `PArray`: `+0` the count (top bit: borrowed storage, never freed), `+4`
//! the elements: a pointer for more than one, the element itself for
//! one (`PArray_Data`).

/// `0x004cb090` (cdecl `(x)` -> object), `0x004cc030`'s.
pub const MAKE_4CB090: u32 = 0x004c_b090;
/// `0x004d3830` (cdecl `(x)` -> object), `0x004d3d10`'s.
pub const MAKE_4D3830: u32 = 0x004d_3830;
/// The elements' destructor `0x004f90b0` calls (cdecl `(elements, count)`).
pub const DESTROY_4F4FC0: u32 = 0x004f_4fc0;
/// The object whose virtual slot 1 `0x004f9100` asks (an image global).
pub const OBJECT_CA70F8: u32 = 0x00ca_70f8;
/// What `0x004f9100` hands a one-element array to (thiscall
/// `(object, elements, 1)`).
pub const STORE_43C160: u32 = 0x0043_c160;
/// The thread pool's stack size (`PThreadPool::SetStackSize`'s word).
pub const STACK_SIZE: u32 = 0x00c0_b954;

pub mod bridge {
    //! The originals over game memory.

    use ffx_graphic::host::{Conv, Host};

    use super::{DESTROY_4F4FC0, OBJECT_CA70F8, STACK_SIZE, STORE_43C160};
    use crate::{phyre_free, phyre_malloc, small};

    /// `0x004cc030` / `0x004d3d10(out, x)`: `*out = make(x)`; 0, or `0x13`
    /// when that gave null.
    pub fn make_into(h: &mut impl Host, make: u32, out: u32, x: u32) -> u32 {
        let r = h.call_sym(make, Conv::Cdecl, &[x]);
        h.write(out, r);
        if r == 0 { 0x13 } else { 0 }
    }

    /// `0x004d55d0(words, value, bit, count)`: `count` bits of `value`
    /// into the bit array `*words` from bit `bit`, low bits first, at most
    /// one word per step (shifts by 32 do what the x86 does with them:
    /// nothing).
    pub fn set_bits(h: &mut impl Host, this: u32, value: u32, bit: u32, count: u32) {
        let mut value = value;
        let mut done = 0u32;
        while done < count {
            let pos = bit.wrapping_add(done);
            let word = pos >> 5;
            let shift = pos & 0x1f;
            let n = count.wrapping_sub(done).wrapping_add(shift).min(0x20) - shift;
            let mask = (1u32 << (n & 0x1f)).wrapping_sub(1);
            let at = h.read::<u32>(this).wrapping_add(word.wrapping_mul(4));
            let w: u32 = h.read(at);
            h.write(at, w & !(mask << shift));
            let at = h.read::<u32>(this).wrapping_add(word.wrapping_mul(4));
            let w: u32 = h.read(at);
            h.write(at, w | ((mask & value) << shift));
            value >>= n & 0x1f;
            done = done.wrapping_add(n);
        }
    }

    /// `PSharray_compareWords` `0x004eea10(array, words, count)`: -1, 0 or
    /// 1 by the count, then the first differing word (unsigned).
    pub fn compare_words(h: &mut impl Host, this: u32, words: u32, count: u32) -> u32 {
        let n = h.read::<u32>(this) & 0x7fff_ffff;
        if n < count {
            return u32::MAX;
        }
        if n > count {
            return 1;
        }
        let data = small::bridge::array_data(h, this);
        for i in 0..n {
            let a: u32 = h.read(data.wrapping_add(4 * i));
            let b: u32 = h.read(words.wrapping_add(4 * i));
            if a < b {
                return u32::MAX;
            }
            if a > b {
                return 1;
            }
        }
        0
    }

    /// `0x004f9020(array, src, n)`: the array takes `n` elements at `src`
    /// (borrowed; one element is copied in place), the old owned buffer
    /// freed. 0, or 5 for a negative `n`.
    pub fn assign(h: &mut impl Host, this: u32, src: u32, n: u32) -> u32 {
        if (n as i32) < 0 {
            return 5;
        }
        let old = small::bridge::array_data(h, this);
        let inline = this + 4;
        let (data, count) = if n == 1 {
            let v: u32 = h.read(src);
            h.write(inline, v);
            (inline, 1)
        } else {
            (src, n | 0x8000_0000)
        };
        if old != data && old != inline && h.read::<u32>(this) >> 31 == 0 {
            phyre_free(h, old);
        }
        if data != inline {
            h.write(inline, data);
        }
        h.write(this, count);
        0
    }

    /// `0x004f90b0(array, elements, n)`: the array borrows `n` elements,
    /// the old owned ones destroyed (`0x004f4fc0(old, count)`) unless they
    /// are the same. 0.
    pub fn borrow(h: &mut impl Host, this: u32, elements: u32, n: u32) -> u32 {
        if (n as i32) < 0 {
            return 0;
        }
        let old: u32 = h.read(this + 4);
        if old != elements {
            let count: u32 = h.read(this);
            if count >> 31 == 0 {
                h.call_sym(DESTROY_4F4FC0, Conv::Cdecl, &[old, count & 0x7fff_ffff]);
            }
        }
        h.write(this + 4, elements);
        h.write(this, n | 0x8000_0000);
        0
    }

    /// `0x004f9100(array, src, n)`: asks the object at `0x00ca70f8`
    /// (virtual slot 1); with one, a one-element self-assignment goes to
    /// `0x0043c160(object; elements, 1)`, else [`assign`]. 0.
    pub fn assign_or_store(h: &mut impl Host, this: u32, src: u32, n: u32) -> u32 {
        let g = h.rebase(OBJECT_CA70F8);
        let object = h.call_virtual(g, 1, &[]);
        let inline = this + 4;
        if src == inline && n == 1 && object != 0 {
            h.call_sym(STORE_43C160, Conv::Thiscall(object), &[inline, 1]);
            return 0;
        }
        assign(h, this, src, n);
        0
    }

    /// `0x0057cfe0(this, object)`: `object`'s virtual slot 1 with
    /// `(this +0xc, this +0x10, 0)`; its result.
    pub fn forward_57cfe0(h: &mut impl Host, this: u32, object: u32) -> u32 {
        let a: u32 = h.read(this + 0xc);
        let b: u32 = h.read(this + 0x10);
        h.call_virtual(object, 1, &[a, b, 0])
    }

    /// `0x0057d000(this)`: a stack of the pool's stack size
    /// (`PhyreMalloc(size, 0x80)` into `+0xc`, the size into `+0x10`); 0,
    /// or `0xd` without memory.
    pub fn alloc_stack(h: &mut impl Host, this: u32) -> u32 {
        let at = h.rebase(STACK_SIZE);
        let mut size: u32 = h.read(at);
        if size != 0 {
            let p = phyre_malloc(h, size, 0x80);
            h.write(this + 0xc, p);
            if p == 0 {
                return 0xd;
            }
            size = h.read(at);
        }
        h.write(this + 0x10, size);
        0
    }
}
