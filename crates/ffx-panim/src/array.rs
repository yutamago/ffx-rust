//! `PArray` setters and the `PClassDataMemberArrayDynamic` callbacks of
//! the animation classes.
//!
//! A `PArray<T>` is its count (top bit: the elements are not owned) and
//! its element pointer (`crate::descriptor`). The type system's dynamic
//! array members keep function pointers in their descriptor (`+0x34`,
//! `+0x38`, `+0x3c`); their vtable callbacks forward to them.

use ffx_graphic::host::{Conv, Host};

use crate::descriptor::{NOT_OWNED, bridge::free_elements, free};

/// `PAnimationChannel`'s destructor (`thiscall(this)`), which
/// `0x00517270` runs on each element of a `PArray<PAnimationChannel>`.
pub const CHANNEL_DTOR: u32 = 0x0050_d1d0;
/// The size of a `PAnimationChannel`.
pub const CHANNEL_SIZE: u32 = 0x34;

/// `Phyre::PhyreMalloc(size, align)` (cdecl).
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;

/// What a setter does with the elements it replaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Old {
    /// `PhyreFree` them when the pointer is not null (`0x0050aff0`).
    Free,
    /// `PhyreFree` them, null or not (`0x00517380`).
    FreeAlways,
    /// Free each element's owned name, then the array
    /// (`PAnimationChannelTarget`'s, `0x0051db50`:
    /// `crate::descriptor::bridge::free_elements` with stride `0x1c`,
    /// name at `+0x14`).
    Targets,
}

pub mod bridge {
    use super::{
        CHANNEL_DTOR, CHANNEL_SIZE, Conv, Host, NOT_OWNED, Old, PHYRE_MALLOC, free, free_elements,
    };

    /// `PArray<T>::setArray(els, count)` as the dynamic members use it
    /// (`thiscall`; `0x0050aff0` and its copies): for a count with the top
    /// bit clear, frees the old elements when they are other than `els`
    /// and the array owns them (`old`), then stores `els` and `count`
    /// with the "not owned" bit set. A negative count changes nothing.
    /// Returns 0.
    pub fn set_borrowed(h: &mut impl Host, this: u32, els: u32, count: u32, old: Old) -> u32 {
        if count & NOT_OWNED == 0 {
            let prev: u32 = h.read(this + 4);
            let flags: u32 = h.read(this);
            if prev != els && flags & NOT_OWNED == 0 {
                match old {
                    Old::Free if prev != 0 => {
                        free(h, prev);
                    }
                    Old::Free => {}
                    Old::FreeAlways => {
                        free(h, prev);
                    }
                    Old::Targets => {
                        free_elements(h, prev, flags & !NOT_OWNED, 0x1c, 0x14);
                    }
                }
            }
            h.write(this + 4, els);
            h.write(this, count | NOT_OWNED);
        }
        0
    }

    /// `PArray<PAnimationChannel>::assign(els, count)` (`thiscall`,
    /// `0x00517270`): when the old elements are other than `els`, owned
    /// and not null, runs each one's destructor ([`CHANNEL_DTOR`], stride
    /// [`CHANNEL_SIZE`]) and frees them; then stores `els` and `count` as
    /// given. Returns `count`.
    pub fn assign_channels(h: &mut impl Host, this: u32, els: u32, count: u32) -> u32 {
        let flags: u32 = h.read(this);
        let prev: u32 = h.read(this + 4);
        if prev != els && flags & NOT_OWNED == 0 && prev != 0 {
            let mut el = prev;
            for _ in 0..flags & !NOT_OWNED {
                h.call_sym(CHANNEL_DTOR, Conv::Thiscall(el), &[]);
                el = el.wrapping_add(CHANNEL_SIZE);
            }
            free(h, prev);
        }
        h.write(this + 4, els);
        h.write(this, count);
        count
    }

    /// The dynamic member's `setArray` for `PArray<PAnimationChannel>`
    /// (`0x005173d0`; `thiscall`, `this` passed on in `ecx`): a count with
    /// the top bit clear goes to [`assign_channels`] with the bit set.
    /// Returns 0.
    pub fn set_channels(h: &mut impl Host, this: u32, els: u32, count: u32) -> u32 {
        if count & NOT_OWNED == 0 {
            assign_channels(h, this, els, count | NOT_OWNED);
        }
        0
    }

    /// The dynamic member callback that forwards to the descriptor's
    /// function at `+0x3c` (`0x0050b040` and its copies; `thiscall(desc,
    /// object, a, b)`): `f(object; a, b)` as a `thiscall`, or `0x15`
    /// (not supported) without one. Returns its `eax`.
    pub fn forward_3c(h: &mut impl Host, desc: u32, object: u32, a: u32, b: u32) -> u32 {
        let f: u32 = h.read(desc + 0x3c);
        if f == 0 {
            return 0x15;
        }
        h.call(f, Conv::Thiscall(object), &[a, b])
    }

    /// The dynamic member callback for element `i`'s address
    /// (`0x005166f0`, `0x0051cf10`; `thiscall(desc, i)`): the descriptor's
    /// function at `+0x38` with `ecx = i` and `edx = desc`, or else the
    /// word at `[desc + 0x24] - [[desc + 0xc] + 0x88] + i`.
    pub fn forward_38(h: &mut impl Host, desc: u32, i: u32) -> u32 {
        let f: u32 = h.read(desc + 0x38);
        if f != 0 {
            return h.call(f, Conv::Fastcall, &[i, desc]);
        }
        let class: u32 = h.read(desc + 0xc);
        let base: u32 = h.read(class + 0x88);
        let off: u32 = h.read(desc + 0x24);
        h.read(off.wrapping_sub(base).wrapping_add(i))
    }

    /// The dynamic member callback through the descriptor's function at
    /// `+0x34` (`0x0051d0b0`; `thiscall(desc, object)`): `f(object)` as a
    /// `thiscall`, or 0 without one.
    pub fn forward_34(h: &mut impl Host, desc: u32, object: u32) -> u32 {
        let f: u32 = h.read(desc + 0x34);
        if f == 0 {
            return 0;
        }
        h.call(f, Conv::Thiscall(object), &[])
    }

    /// Copies `n` words from `src` into a new array (`0x005281e0`, cdecl;
    /// `PhyreMalloc(4 * n, 4)`). Returns it, or 0 for `n == 0` or a failed
    /// allocation.
    pub fn copy_words(h: &mut impl Host, n: u32, src: u32) -> u32 {
        if n == 0 {
            return 0;
        }
        let out = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[n.wrapping_mul(4), 4]);
        if out == 0 {
            return 0;
        }
        let mut from = src;
        for i in 0..n {
            let to = out.wrapping_add(i.wrapping_mul(4));
            // the original's placement-new check of each element
            if to != 0 {
                let w: u32 = h.read(from);
                h.write(to, w);
                from = from.wrapping_add(4);
            }
        }
        out
    }
}
