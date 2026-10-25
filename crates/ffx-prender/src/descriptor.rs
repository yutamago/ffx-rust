//! `PhyreEngine`'s type system, as the render classes use it: the slots of
//! the class descriptors (`PClassDescriptorConcrete<T>`,
//! `PClassDescriptorForType<T>`, `...Abstract<T>`,
//! `...WithoutDefaultConstructor<T>`) and of the dynamic array members
//! (`PClassDataMemberArrayDynamic<PArray<T>>`), which the serializer calls
//! through their vtables when it builds, frees and fixes up loaded objects.
//!
//! Each class has its own copy of these (one template instance per type);
//! the bridges here take what differs (the field layout, the destructor,
//! the element free) as arguments, and the shims pass each copy's values.
//!
//! Descriptor slots (vtable offsets; Fahrenheit names where they have one):
//! - `+0x28` `constructInstance(p)`: the default constructor in place: the
//!   fields' initial values when `p` is not null; `al = 1`
//!   ([`bridge::construct`]); for classes without a default constructor, a
//!   check of the type registry first ([`bridge::construct_registered`]).
//! - `+0x2c` destructInstance (our name) `(p)`: the destructor in place
//!   ([`bridge::destruct_with`], [`bridge::destruct_virtual`]), for arrays
//!   the storage when the array owns it ([`bridge::free_array`]); `al = 1`.
//! - `+0x38` `fixObjectsAfterLoading(p)`: 0 (nothing to fix) or a call.
//!
//! Array members: `+0x20` `(array, a, b)` forwards to the member's handler
//! at `+0x3c` or fails with `0x15`; `+0x18` `(array, i)`: the handler at
//! `+0x38` or the element word at the member's offset; `+0x10` `(array)`:
//! the handler at `+0x34` or 0 ([`bridge::member_forward`] and the others).
//!
//! An array (`PArray<T>`, `PSharray<T>`) is `count | owned-bit, data`: bit
//! 31 of the count word set means the storage is borrowed (not freed).

use ffx_graphic::host::{Conv, Host};

/// `Phyre::PhyreFree(p)` (cdecl).
pub const PHYRE_FREE: u32 = 0x0042_fb20;

/// The type registry lookup `0x0043cbb0` (thiscall on the registry).
pub const REGISTRY_LOOKUP: u32 = 0x0043_cbb0;

/// An initial value [`bridge::construct`] stores.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Init {
    /// A word at the offset.
    Word(u32, u32),
    /// A byte at the offset.
    Byte(u32, u8),
    /// The `f32` at an image address (its `ffx.sym` address), copied by
    /// `fld`/`fstp` (a signalling NaN comes out quiet).
    Float(u32, u32),
}

pub mod bridge {
    //! The originals over game memory.

    use super::{Conv, Host, Init, PHYRE_FREE, REGISTRY_LOOKUP};
    use ffx_bridge::x87;

    /// `constructInstance(p)`: when `p` is not null, the initial values;
    /// `al = 1`.
    pub fn construct(m: &mut impl Host, p: u32, fields: &[Init]) -> u8 {
        if p != 0 {
            for &f in fields {
                match f {
                    Init::Word(off, v) => m.write(p.wrapping_add(off), v),
                    Init::Byte(off, v) => m.write(p.wrapping_add(off), v),
                    Init::Float(off, sym) => {
                        let bits: u32 = m.read(m.rebase(sym));
                        m.write(p.wrapping_add(off), x87::store(x87::load(bits)));
                    }
                }
            }
        }
        1
    }

    /// `constructInstance(p)` of a class built by a constructor function:
    /// `ctor(p)` (cdecl); returns its `eax`.
    pub fn construct_by(h: &mut impl Host, p: u32, ctor: u32) -> u32 {
        let f = h.rebase(ctor);
        h.call(f, Conv::Cdecl, &[p])
    }

    /// `constructInstance(p)` of the classes without a default
    /// constructor: the registry `registry`'s lookup `0x0043cbb0` first
    /// (`al = 0` when it gives null), then, when `p` is not null, the base
    /// constructor `ctor(p)` (thiscall) and `p + 0x4e0 = 0`; `al = 1`.
    pub fn construct_registered(h: &mut impl Host, p: u32, registry: u32, ctor: u32) -> u8 {
        let f = h.rebase(REGISTRY_LOOKUP);
        let r = h.rebase(registry);
        if h.call(f, Conv::Thiscall(r), &[]) == 0 {
            return 0;
        }
        if p != 0 {
            let f = h.rebase(ctor);
            h.call(f, Conv::Thiscall(p), &[]);
            h.write(p + 0x4e0, 0u32);
        }
        1
    }

    /// `PDynamicMeshInstance`'s `constructInstance(p)`: the registry
    /// `registry`'s lookup gives the owner (null, or 4 as the adjusted
    /// null, gives `al = 0`); then, when `p` is not null, `p = (vtable,
    /// owner - 4, 0)`; `al = 1`.
    pub fn construct_owned(h: &mut impl Host, p: u32, registry: u32, vtable: u32) -> u8 {
        let f = h.rebase(REGISTRY_LOOKUP);
        let r = h.rebase(registry);
        let found = h.call(f, Conv::Thiscall(r), &[]);
        if found == 0 || found == 4 {
            return 0;
        }
        if p != 0 {
            let v = h.rebase(vtable);
            h.write(p, v);
            h.write(p + 4, found.wrapping_sub(4));
            h.write(p + 8, 0u32);
        }
        1
    }

    /// destructInstance `(p)`: the destructor `dtor(p)` (thiscall); `al =
    /// 1` (over the destructor's `eax`, which the callers do not read).
    pub fn destruct_with(h: &mut impl Host, p: u32, dtor: u32) -> u8 {
        let f = h.rebase(dtor);
        h.call(f, Conv::Thiscall(p), &[]);
        1
    }

    /// [`destruct_with`] when `p` is not null.
    pub fn destruct_with_if(h: &mut impl Host, p: u32, dtor: u32) -> u8 {
        if p != 0 {
            destruct_with(h, p, dtor);
        }
        1
    }

    /// destructInstance `(p)` of a class with a virtual destructor: the
    /// object starts 4 bytes before `p` (null stays null), whose first
    /// virtual method is the scalar deleting destructor, called with 0 (no
    /// free); `al = 1`.
    pub fn destruct_virtual(h: &mut impl Host, p: u32) -> u8 {
        let obj = if p == 0 { 0 } else { p.wrapping_sub(4) };
        h.call_virtual(obj, 0, &[0]);
        1
    }

    /// destructInstance `(array)` of a `PArray`: the count word at
    /// `array + count`, the data at `array + count + 4`. When the array
    /// owns its storage (bit 31 clear): `free(data, count)` (cdecl) when
    /// `free` is given, else `Phyre_PhyreFree(data)` (only for non-null
    /// data when `null_check`). Then data and count are 0; `al = 1`.
    pub fn free_array(
        h: &mut impl Host,
        array: u32,
        count: u32,
        free: Option<u32>,
        null_check: bool,
    ) -> u8 {
        let n: u32 = h.read(array + count);
        let data: u32 = h.read(array + count + 4);
        if n & 0x8000_0000 == 0 {
            match free {
                Some(f) => {
                    let f = h.rebase(f);
                    h.call(f, Conv::Cdecl, &[data, n & 0x7fff_ffff]);
                }
                None if null_check && data == 0 => {}
                None => {
                    let f = h.rebase(PHYRE_FREE);
                    h.call(f, Conv::Cdecl, &[data]);
                }
            }
        }
        h.write(array + count + 4, 0u32);
        h.write(array + count, 0u32);
        1
    }

    /// destructInstance `(p)` of a class holding one buffer at `p + off`
    /// that it owns when the word's bit 0 is clear: `Phyre_PhyreFree` of
    /// it; `al = 1`.
    pub fn free_if_owned(h: &mut impl Host, p: u32, off: u32) -> u8 {
        let w: u32 = h.read(p + off);
        if w & 1 == 0 {
            let f = h.rebase(PHYRE_FREE);
            h.call(f, Conv::Cdecl, &[w]);
        }
        1
    }

    /// `(items, n)`: the element free of the arrays of 0x14-byte items
    /// that each hold one buffer (as [`free_if_owned`] at offset 0), then
    /// the items' storage; nothing when `items` is null.
    pub fn free_owned_buffers(h: &mut impl Host, items: u32, n: u32) {
        if items == 0 {
            return;
        }
        let free = h.rebase(PHYRE_FREE);
        let mut e = items;
        for _ in 0..n {
            let w: u32 = h.read(e);
            if w & 1 == 0 {
                h.call(free, Conv::Cdecl, &[w]);
            }
            e = e.wrapping_add(0x14);
        }
        h.call(free, Conv::Cdecl, &[items]);
    }

    /// The array member's `+0x20 (array, a, b)`: its handler at
    /// `member + 0x3c` as `handler(array; a, b)` (thiscall), or `0x15`
    /// without one.
    pub fn member_forward(h: &mut impl Host, member: u32, array: u32, a: u32, b: u32) -> u32 {
        let f: u32 = h.read(member + 0x3c);
        if f == 0 {
            return 0x15;
        }
        h.call(f, Conv::Thiscall(array), &[a, b])
    }

    /// The array member's `+0x18 (i)`: its handler at `member + 0x38` as
    /// `handler(i)` (thiscall), or the word at the member's field offset
    /// (`member + 0x24` less the class's base, `[member + 0xc] + 0x88`)
    /// plus `i`.
    pub fn member_element(h: &mut impl Host, member: u32, i: u32) -> u32 {
        let f: u32 = h.read(member + 0x38);
        if f != 0 {
            return h.call(f, Conv::Thiscall(i), &[]);
        }
        let class: u32 = h.read(member + 0xc);
        let field: u32 = h.read(member + 0x24);
        let base: u32 = h.read(class + 0x88);
        h.read(field.wrapping_sub(base).wrapping_add(i))
    }

    /// The array member's `+0x10 (p)`: its handler at `member + 0x34` as
    /// `handler(p)` (thiscall), or 0.
    pub fn member_handler34(h: &mut impl Host, member: u32, p: u32) -> u32 {
        let f: u32 = h.read(member + 0x34);
        if f == 0 {
            return 0;
        }
        h.call(f, Conv::Thiscall(p), &[])
    }

    /// `PArray<T>`'s set-borrowed helper `(array, data, count)`: for a
    /// count with bit 31 clear, `set(array; data, count | 0x80000000)`
    /// (thiscall, the array's own setter); returns 0.
    pub fn set_borrowed(h: &mut impl Host, array: u32, data: u32, count: u32, set: u32) -> u32 {
        if count & 0x8000_0000 == 0 {
            let f = h.rebase(set);
            h.call(f, Conv::Thiscall(array), &[data, count | 0x8000_0000]);
        }
        0
    }

    /// The type registration helpers `()`:
    /// `Phyre_PTypeDispenser_RegisterType(descriptor)` (cdecl), then
    /// `Phyre_PClassDescriptor_addToNamespace(descriptor)` (thiscall, its
    /// `eax` returned).
    pub fn register_type(h: &mut impl Host, descriptor: u32) -> u32 {
        let d = h.rebase(descriptor);
        let f = h.rebase(super::REGISTER_TYPE);
        h.call(f, Conv::Cdecl, &[d]);
        let f = h.rebase(super::ADD_TO_NAMESPACE);
        h.call(f, Conv::Thiscall(d), &[])
    }
}

/// `Phyre::PTypeDispenser::RegisterType(descriptor)` (cdecl).
pub const REGISTER_TYPE: u32 = 0x0043_95b0;
/// `Phyre::PClassDescriptor::addToNamespace(this)` (thiscall).
pub const ADD_TO_NAMESPACE: u32 = 0x0043_c140;
