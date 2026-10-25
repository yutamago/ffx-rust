//! Class-descriptor callbacks of the animation classes.
//!
//! `PhyreEngine`'s type system keeps one `PClassDescriptor` per class;
//! its vtables (`PClassDescriptorForType<T>`, `...Abstract<T>`,
//! `...Concrete<T>`) hold per-class callbacks, each `stdcall(instance)`:
//! - slot `+0x28` constructs an instance in place (zeroes its fields);
//! - slot `+0x2c` destructs one (frees what it owns);
//! - slot `+0x38` fixes objects after loading (nothing to do: 0).
//!
//! The compiler made many identical copies (one per class); each shape
//! below serves every copy. `PArray<T>` is two words: the count, whose top
//! bit says the elements are *not* owned (memory of the loaded cluster),
//! and the element pointer.
//!
//! Results: the construct and destruct callbacks return `true` with
//! `mov al, 1`; the bridges return the whole `eax` the original leaves
//! ([`crate::al_one`] over the pointer or the last callee's `eax`) where
//! that is defined by the arguments.

use ffx_bridge::Mem;
use ffx_graphic::host::{Conv, Host};

/// `Phyre::PhyreFree(ptr)` (cdecl).
pub const PHYRE_FREE: u32 = 0x0042_fb20;
/// `-1.0f` in `.rdata` (`g_ChrClothFlip`), which `PAnimationClip`'s
/// construct callback loads for its end time.
pub const MINUS_ONE: u32 = 0x00b9_2340;

/// The top bit of a `PArray` count: the elements are not the array's.
const NOT_OWNED: u32 = 0x8000_0000;

fn free(h: &mut impl Host, p: u32) -> u32 {
    h.call_sym(PHYRE_FREE, Conv::Cdecl, &[p])
}

pub mod bridge {
    use super::{Host, MINUS_ONE, Mem, NOT_OWNED, free};
    use crate::al_one;

    /// Construct callbacks that store `words` at `p` when `p` is not null
    /// (`0x0050a950` and its copies: two zero words; `0x00508c50`: the
    /// slot-list index `0, 3, 0, 0`). Returns `eax`: `p` with `al = 1`.
    pub fn init_words(m: &mut impl Mem, p: u32, words: &[u32]) -> u32 {
        if p != 0 {
            for (i, &w) in (0u32..).zip(words) {
                m.write(p + 4 * i, w);
            }
        }
        al_one(p)
    }

    /// `PAnimationClip`'s construct callback (`0x00516140`): five zero
    /// words, start time `1.0` (`+0x14`), end time `-1.0` (`+0x18`, copied
    /// from [`MINUS_ONE`]), a zero word at `+0x1c`.
    pub fn clip_init(m: &mut impl Mem, p: u32) -> u32 {
        if p != 0 {
            for off in [0, 4, 8, 0xc, 0x10, 0x1c] {
                m.write(p + off, 0u32);
            }
            m.write(p + 0x14, 1.0f32.to_bits());
            let minus_one: u32 = m.read(m.rebase(MINUS_ONE));
            m.write(p + 0x18, minus_one);
        }
        al_one(p)
    }

    /// A `PArray` destruct callback (`0x0050a9a0` and its copies; with
    /// `array = p + 4`, `0x0050a9e0`): frees the elements when the array
    /// owns them and the pointer is not null (`null_check`; the copies
    /// without the check free a null pointer too), then zeroes both
    /// words. Returns `eax`: `PhyreFree`'s, else 0, with `al = 1`.
    pub fn free_owned(h: &mut impl Host, array: u32, null_check: bool) -> u32 {
        let count: u32 = h.read(array);
        let mut eax = 0;
        if count & NOT_OWNED == 0 {
            let els: u32 = h.read(array + 4);
            if !null_check || els != 0 {
                eax = free(h, els);
            }
        }
        h.write(array + 4, 0u32);
        h.write(array, 0u32);
        al_one(eax)
    }

    /// `PSharray<PAnimationClip>`'s destruct callback (`0x0051c9c0`): as
    /// [`free_owned`], but only an owned array of more than one element is
    /// freed (a single element sits in the pointer word itself).
    pub fn free_owned_shared(h: &mut impl Host, array: u32) -> u32 {
        let count: u32 = h.read(array);
        let mut eax = 0;
        if count & NOT_OWNED == 0 && count & !NOT_OWNED > 1 {
            let els: u32 = h.read(array + 4);
            eax = free(h, els);
        }
        h.write(array + 4, 0u32);
        h.write(array, 0u32);
        al_one(eax)
    }

    /// Frees an array of `n` elements of `stride` bytes whose word at
    /// `off` is a string pointer with an "not owned" flag in bit 0
    /// (`0x0051a890`: `PAnimationChannelTarget`, stride `0x1c`, the name
    /// at `+0x14`; `0x00528a30`: stride 8, at `+4`): each owned string,
    /// then the array. Nothing for a null array.
    ///
    /// Returns the last `PhyreFree`'s `eax`, `None` when nothing was
    /// called (the original leaves `eax` as it found it).
    pub fn free_elements(
        h: &mut impl Host,
        els: u32,
        n: u32,
        stride: u32,
        off: u32,
    ) -> Option<u32> {
        if els == 0 {
            return None;
        }
        let mut at = els + off;
        for _ in 0..n {
            let s: u32 = h.read(at);
            if s & 1 == 0 {
                free(h, s);
            }
            at = at.wrapping_add(stride);
        }
        Some(free(h, els))
    }

    /// The destruct callback of a `PArray` of such elements (`0x0051c940`,
    /// `0x0052a140`): [`free_elements`] when the array owns them, then
    /// zeroes both words. Returns `eax` with `al = 1` (the original's
    /// `0xff` when nothing was called).
    pub fn free_owned_elements(h: &mut impl Host, array: u32, stride: u32, off: u32) -> u32 {
        let count: u32 = h.read(array);
        let mut eax = 0;
        if count & NOT_OWNED == 0 {
            let els: u32 = h.read(array + 4);
            eax = free_elements(h, els, count & !NOT_OWNED, stride, off).unwrap_or(0xff);
        }
        h.write(array + 4, 0u32);
        h.write(array, 0u32);
        al_one(eax)
    }

    /// A destruct callback that frees the string at `p + off` unless its
    /// bit 0 says it is not owned (`0x0050c590`: `+0x14`; `0x0052a1b0`:
    /// `+4`), after zeroing the word at `p + clear` (`0x0050d740`: `+0x1c`,
    /// then the string at `+0x14`). Returns `true` in `al` (the rest of
    /// `eax` is the caller's or `PhyreFree`'s; callers test `al`).
    pub fn free_flagged(h: &mut impl Host, p: u32, off: u32, clear: Option<u32>) -> bool {
        if let Some(c) = clear {
            h.write(p + c, 0u32);
        }
        let s: u32 = h.read(p + off);
        if s & 1 == 0 {
            free(h, s);
        }
        true
    }
}
