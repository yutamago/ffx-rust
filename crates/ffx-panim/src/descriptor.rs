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

/// `Phyre::PTypeDispenser::RegisterType(desc)` (cdecl).
pub const REGISTER_TYPE: u32 = 0x0043_95b0;
/// `Phyre::PClassDescriptor::addToNamespace()` (`thiscall`, no stack
/// arguments).
pub const ADD_TO_NAMESPACE: u32 = 0x0043_c140;

/// `Phyre::PBase::GetClassDescriptor()` (cdecl; returns `PBase`'s
/// descriptor), which the channel-target construct callbacks store first.
pub const GET_CLASS_DESCRIPTOR: u32 = 0x0043_5740;
/// `PAnimationChannelBase`'s constructor (`thiscall(this)`).
pub const CHANNEL_BASE_CTOR: u32 = 0x0050_d150;
/// `PAnimationChannelBase`'s destructor (`thiscall(this)`).
pub const CHANNEL_BASE_DTOR: u32 = 0x0050_d1d0;

/// `Phyre::PNamespace::GetGlobalNamespace()` (cdecl, no arguments).
pub const GET_GLOBAL_NAMESPACE: u32 = 0x0043_e2f0;
/// `Phyre::PClassDescriptor::PClassDescriptor(ns, name, size, align, base,
/// 0)` (`thiscall`, pops six words).
pub const CLASS_DESCRIPTOR_CTOR: u32 = 0x0043_b0a0;
/// `PClassDescriptor::setDefaultBuffer`, `setValidationBuffer`,
/// `setWriteMaskBuffer` (`thiscall(this, buffer)`).
pub const SET_BUFFERS: [u32; 3] = [0x0043_d510, 0x0043_d5b0, 0x0043_d5c0];

/// Where a class descriptor's name comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Name {
    /// The string pointer held in this global (the `PArray` templates).
    Global(u32),
    /// This string in the image.
    Literal(u32),
}

/// What a `PClassDescriptorAbstract<T>` constructor (`0x00509630` shape)
/// fills in: `ffx.sym` addresses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AbstractDescriptor {
    pub name: Name,
    /// The instance size (8 for the `PArray`s).
    pub size: u32,
    /// `PClassDescriptorAbstract<T>`'s vtable.
    pub vtable: u32,
    /// The default, validation and write-mask buffers.
    pub buffers: [u32; 3],
}

/// The top bit of a `PArray` count: the elements are not the array's.
pub(crate) const NOT_OWNED: u32 = 0x8000_0000;

/// `Phyre::PhyreFree(p)`; returns its `eax`.
pub(crate) fn free(h: &mut impl Host, p: u32) -> u32 {
    h.call_sym(PHYRE_FREE, Conv::Cdecl, &[p])
}

pub mod bridge {
    use super::{
        ADD_TO_NAMESPACE, AbstractDescriptor, CHANNEL_BASE_CTOR, CHANNEL_BASE_DTOR,
        CLASS_DESCRIPTOR_CTOR, Conv, GET_CLASS_DESCRIPTOR, GET_GLOBAL_NAMESPACE, Host, MINUS_ONE,
        Mem, NOT_OWNED, Name, REGISTER_TYPE, SET_BUFFERS, free,
    };
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

    /// The destruct callback of a class with a virtual destructor
    /// (`0x005220a0` and its copies): the instance pointer is `p`, the
    /// object starts 4 bytes before it (`p - 4`, or null for a null `p`,
    /// which the original then reads through as well); calls its vtable's
    /// slot 0 (the deleting destructor) with flags 0, as a `thiscall`.
    /// Returns `eax`: the destructor's with `al = 1`.
    pub fn destroy_virtual(h: &mut impl Host, p: u32) -> u32 {
        let obj = if p == 0 { 0 } else { p.wrapping_sub(4) };
        let vtable: u32 = h.read(obj);
        let dtor: u32 = h.read(vtable);
        al_one(h.call(dtor, Conv::Thiscall(obj), &[0]))
    }

    /// A callback that runs the class's destructor `method` (an `ffx.sym`
    /// address, `thiscall(this)`) on `p` (destruct: `0x0050f710` and
    /// others). Returns `eax`: the method's with `al = 1`.
    pub fn call_on(h: &mut impl Host, p: u32, method: u32) -> u32 {
        al_one(h.call_sym(method, Conv::Thiscall(p), &[]))
    }

    /// As [`call_on`], but nothing for a null `p` (construct: `0x0053a700`,
    /// `0x0053a720` run the constructor). Returns `true` in `al` (the rest
    /// of `eax` is the caller's for a null `p`).
    pub fn call_on_nonnull(h: &mut impl Host, p: u32, method: u32) -> bool {
        if p != 0 {
            h.call_sym(method, Conv::Thiscall(p), &[]);
        }
        true
    }

    /// A class's registration (`0x00508450` and its copies):
    /// `PTypeDispenser::RegisterType(desc)`, then tail-calls
    /// `desc->addToNamespace()`. `desc` is the descriptor's `ffx.sym`
    /// address. Returns `addToNamespace`'s `eax`.
    pub fn register(h: &mut impl Host, desc: u32) -> u32 {
        let d = h.rebase(desc);
        h.call_sym(REGISTER_TYPE, Conv::Cdecl, &[d]);
        h.call_sym(ADD_TO_NAMESPACE, Conv::Thiscall(d), &[])
    }

    /// A construct callback whose class starts with a descriptor pointer
    /// (`0x0050c510`: `PAnimationChannelTarget`, six more words;
    /// `0x0050d6b0`: `PAnimationChannelBase`, eight): when `p` is not
    /// null, `GetClassDescriptor()` at `+0`, then `words` zero words.
    /// Returns `true` in `al` (callers test `al`).
    pub fn construct_described(h: &mut impl Host, p: u32, words: u32) -> bool {
        if p != 0 {
            let d = h.call_sym(GET_CLASS_DESCRIPTOR, Conv::Cdecl, &[]);
            h.write(p, d);
            for i in 1..=words {
                h.write(p + 4 * i, 0u32);
            }
        }
        true
    }

    /// The construct callback of a channel class (`0x0050e5e0`:
    /// `PAnimationChannel`; `0x0050f6b0`: `PAnimationConstantChannel`,
    /// whose four words are `0.0f`): `PAnimationChannelBase`'s constructor
    /// on `p`, then four zero words at `+0x24`. Nothing for a null `p`.
    /// Returns `true` in `al`.
    pub fn construct_channel(h: &mut impl Host, p: u32) -> bool {
        if p != 0 {
            h.call_sym(CHANNEL_BASE_CTOR, Conv::Thiscall(p), &[]);
            for off in [0x24, 0x28, 0x2c, 0x30] {
                h.write(p + off, 0u32);
            }
        }
        true
    }

    /// `PAnimationChannel`'s destruct callback (`0x0050e650`): zeroes
    /// `+0x24` and `+0x30`, frees the owned `PArray` at `+0x28`, then runs
    /// `PAnimationChannelBase`'s destructor. Returns `eax`: the
    /// destructor's with `al = 1`.
    pub fn destroy_channel(h: &mut impl Host, p: u32) -> u32 {
        h.write(p + 0x24, 0u32);
        h.write(p + 0x30, 0u32);
        free_owned(h, p + 0x28, true);
        al_one(h.call_sym(CHANNEL_BASE_DTOR, Conv::Thiscall(p), &[]))
    }

    /// A `PClassDescriptorAbstract<T>` constructor (`thiscall(this)` ->
    /// `this`; `0x00509630` and its copies): `PClassDescriptor`'s
    /// constructor with the global namespace, the name, the size, align 4
    /// and `PBase`'s descriptor as the base; sets flag 2 at `+0x90`, the
    /// vtable, and the three buffers. The original pushes the
    /// constructor's arguments before it calls `GetGlobalNamespace` (which
    /// takes none). SEH frame dropped (nothing here throws).
    pub fn abstract_descriptor_ctor(h: &mut impl Host, this: u32, d: &AbstractDescriptor) -> u32 {
        let base = h.call_sym(GET_CLASS_DESCRIPTOR, Conv::Cdecl, &[]);
        let name = match d.name {
            Name::Global(g) => {
                let at = h.rebase(g);
                h.read(at)
            }
            Name::Literal(s) => h.rebase(s),
        };
        let ns = h.call_sym(GET_GLOBAL_NAMESPACE, Conv::Cdecl, &[]);
        h.call_sym(
            CLASS_DESCRIPTOR_CTOR,
            Conv::Thiscall(this),
            &[ns, name, d.size, 4, base, 0],
        );
        let flags: u32 = h.read(this + 0x90);
        h.write(this + 0x90, flags | 2);
        let vt = h.rebase(d.vtable);
        h.write(this, vt);
        for (f, b) in SET_BUFFERS.into_iter().zip(d.buffers) {
            let b = h.rebase(b);
            h.call_sym(f, Conv::Thiscall(this), &[b]);
        }
        this
    }
}
