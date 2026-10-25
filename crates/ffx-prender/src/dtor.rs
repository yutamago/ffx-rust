//! Destructors and storage setters of the render classes (h2-render-b
//! round 2): the D3D11 shader objects' releases, the destructors that
//! chain to their members' and bases', and `PArray<T>`'s storage setter
//! for element types with a destructor.
//!
//! Several are identical copies at different addresses (one per class);
//! the bridges take what differs. SEH frames only unwind the chained
//! destructors; dropped, as nothing here throws.

use ffx_graphic::host::{Conv, Host};

use crate::array::{BORROWED, COUNT};
use crate::descriptor::PHYRE_FREE;

pub mod bridge {
    //! The originals over game memory.

    use super::{BORROWED, COUNT, Conv, Host, PHYRE_FREE};

    /// A destructor that runs `first(this)` and then `second(this)`
    /// (thiscall); returns `second`'s `eax`, as the original leaves it.
    pub fn chain2(h: &mut impl Host, this: u32, first: u32, second: u32) -> u32 {
        h.call_sym(first, Conv::Thiscall(this), &[]);
        h.call_sym(second, Conv::Thiscall(this), &[])
    }

    /// `IUnknown::Release` of the object at `slot` when not null (the
    /// original's `push obj; call [vtable + 8]` with `ecx = obj`), then
    /// null in `slot`.
    fn release(h: &mut impl Host, slot: u32) {
        let obj: u32 = h.read(slot);
        if obj != 0 {
            let vt: u32 = h.read(obj);
            let f: u32 = h.read(vt + 8);
            h.call(f, Conv::Thiscall(obj), &[obj]);
            h.write(slot, 0u32);
        }
    }

    /// The D3D11 shader's destructor `(this)` (three identical copies):
    /// releases the objects at `+0x4e0` and `+0`, then frees the owned
    /// byte array at `+4` (count, data at `+8`) and clears it when its
    /// count word is not 0.
    pub fn release_shader(h: &mut impl Host, this: u32) {
        release(h, this + 0x4e0);
        release(h, this);
        let n: u32 = h.read(this + 4);
        if n != 0 {
            let data: u32 = h.read(this + 8);
            if data != 0 && n & BORROWED == 0 {
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
            }
            h.write(this + 8, 0u32);
            h.write(this + 4, 0u32);
        }
    }

    /// A destructor with a `PSharray` member at `this + off`: `first(this)`
    /// (thiscall), the member's storage freed when owned, holding more
    /// than one element and not null, the member cleared, then the base's
    /// `base(this)`; returns `base`'s `eax`.
    pub fn with_sharray(h: &mut impl Host, this: u32, first: u32, off: u32, base: u32) -> u32 {
        h.call_sym(first, Conv::Thiscall(this), &[]);
        let n: u32 = h.read(this + off);
        if n & BORROWED == 0 && n & COUNT > 1 {
            let data: u32 = h.read(this + off + 4);
            if data != 0 {
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
            }
        }
        h.write(this + off + 4, 0u32);
        h.write(this + off, 0u32);
        h.call_sym(base, Conv::Thiscall(this), &[])
    }

    /// `PArray<T>::setStorage(array; data, count)` for elements with a
    /// destructor (thiscall, two words): when the array owns non-null
    /// storage other than `data`, each old element is destroyed
    /// (`dtor(element)`, `stride` apart) and the storage freed; then
    /// `array = (count, data)`. Returns `count`.
    pub fn set_storage_dtor(
        h: &mut impl Host,
        array: u32,
        data: u32,
        count: u32,
        dtor: u32,
        stride: u32,
    ) -> u32 {
        let n: u32 = h.read(array);
        let old: u32 = h.read(array + 4);
        if old != data && n & BORROWED == 0 && old != 0 {
            let f = h.rebase(dtor);
            let mut e = old;
            for _ in 0..n & COUNT {
                h.call(f, Conv::Thiscall(e), &[]);
                e = e.wrapping_add(stride);
            }
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
        }
        h.write(array + 4, data);
        h.write(array, count);
        count
    }
}
