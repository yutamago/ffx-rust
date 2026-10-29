//! `PClassDescriptorForType<T>`'s slots (M5 stage 1, lists
//! phyre-tmpl-fortype-a and -b): one generic port per slot shape, the
//! constants of each of the template's copies in the generated tables
//! (`m5p_tmpl_fortype_table.rs.in`, by the copy's address). The shims and
//! the oracle tests read them from here, so every copy runs its own
//! original in the tests.
//!
//! The slots (Fahrenheit's Switch symbols name them; offsets in the
//! descriptor vtables) are virtual methods of the descriptor, `thiscall`,
//! most of them without reading `this`:
//!
//! - `+0x00` `swapEndianness(p, toNative)` (`ret 8`): hands `p` to
//!   `PClassDescriptor::swapEndiannessOfObject` (`0x0043d6a0`, thiscall on
//!   the class's descriptor, `(p, flag)`), [`SWAP`]; or `p`'s `PBase`
//!   part, `p + offset` unless `p` is 0 ([`SWAP_BASE`]); or the
//!   descriptor a getter returns ([`SWAP_GET`]). `PArray<T>`'s pick one of
//!   two cdecl helpers by the flag ([`SWAP_PAIR`]), which build a static
//!   descriptor on first use ([`SWAP_LAZY`]). `al = 1` over the callee's
//!   `eax`.
//! - `+0x1c` the scalar deleting destructor `(flags)` (`ret 4`): the
//!   copy's vtable, `PClassDescriptor`'s destructor (`0x0043b3e0`),
//!   `PhyreFree` when `flags & 1`; `eax = this` ([`DELETE`]).
//! - `+0x2c` `destroyInstance(p)` (`ret 4`): nothing ([`TRUE`]), the
//!   virtual deleting destructor of the object at `p - adjust`
//!   ([`DESTROY_VIRTUAL`]), the class's destructor ([`DESTROY_CALL`],
//!   [`DESTROY_PAIR`]) or an array release; `al = 1` over the last
//!   callee's `eax`.
//! - `+0x0c`..`+0x18`, the script accessors ([`SCRIPT_VALUE`],
//!   [`GET_POINTER`], [`PUT_POINTER`]), `+0x24` the namespace unlink and
//!   `+0x34` (`-1`): single functions folded across many vtables.
//!
//! A slot's `eax` is what the original leaves: `al = 1` written over the
//! last callee's `eax` ([`al_one`]), as the vtable callers may read it.

use ffx_graphic::host::{Conv, Host};

/// `PClassDescriptor::swapEndiannessOfObject(p, flag)` (thiscall on the
/// descriptor, `ret 8`; Fahrenheit's Switch symbols).
pub const SWAP_ENDIANNESS_OF_OBJECT: u32 = 0x0043_d6a0;
/// `Phyre_PClassDescriptor_Destruct` (thiscall).
pub const DESTRUCT: u32 = 0x0043_b3e0;
/// `Phyre_PhyreFree` (cdecl).
pub const PHYRE_FREE: u32 = 0x0042_fb20;
/// The CRT's `atexit` (cdecl).
pub const ATEXIT: u32 = 0x0094_9207;
/// `Phyre_PhyrePrintf(level, format, ...)` (cdecl).
pub const PHYRE_PRINTF: u32 = 0x0043_52f0;
/// The type name [`SCRIPT_VALUE`] prints when the type has none.
pub const NO_NAME: u32 = 0x00b0_e588;
/// The script layer's object push `(L, object, class)` (cdecl).
pub const PUSH_OBJECT: u32 = 0x0043_6e80;
/// The script layer's `(L, 1)` and `(L)` calls that push nil (cdecl).
pub const PUSH_NIL: [u32; 2] = [0x0094_ac00, 0x0094_b730];
/// `PClassDescriptor`'s own descriptor (`PClassDescriptorForClassDescriptor`).
pub const CLASS_DESCRIPTOR: u32 = 0x00c9_0760;
/// The two visitors `swapEndiannessOfObject` runs (thiscall on the
/// descriptor, `(visitor)`): flag set, flag clear.
pub const SWAP_VISITORS: [u32; 2] = [0x0043_ab60, 0x0043_ad10];
/// `Phyre_PNamespace_releaseOwned_0043dde0` (thiscall).
pub const RELEASE_OWNED: u32 = 0x0043_dde0;
/// `0x004415a0`'s callees: the destructor before, the owner's release
/// (thiscall on the owner, `(p)`), the destructor after.
pub const WITH_OWNER: [u32; 3] = [0x0044_0eb0, 0x0044_3b70, 0x0057_6640];
/// `0x004d7140`'s element release `(data, count)` (cdecl).
pub const RELEASE_ELEMENTS: u32 = 0x004a_b4d0;
/// `0x005dd0a0`'s callees: `PBase::GetClassDescriptor`, the namespace
/// getter, the base constructor (thiscall, 6 words), `setDefaultBuffer`,
/// `setValidationBuffer`, `setWriteMaskBuffer`.
pub const RIGID_BODY_SHAPES_CALLS: [u32; 6] = [
    0x0043_5740,
    0x0043_e2f0,
    0x0043_b0a0,
    0x0043_d510,
    0x0043_d5b0,
    0x0043_d5c0,
];
/// `0x005dd0a0`'s data: the name pointer's cell, the vtable, the three
/// buffers.
pub const RIGID_BODY_SHAPES_DATA: [u32; 5] = [
    0x00cc_77fc,
    0x00b3_c764,
    0x00cc_9670,
    0x00cc_9680,
    0x00cc_9690,
];

/// A static descriptor built on first use ([`SWAP_LAZY`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lazy {
    /// The guard word (bit 0: built).
    pub guard: u32,
    /// The descriptor.
    pub desc: u32,
    /// Its constructor (thiscall).
    pub ctor: u32,
    /// The flags word whose bit 1 is cleared.
    pub flags: u32,
    /// The vtable stored.
    pub vtable: u32,
    /// The destructor handed to `atexit`.
    pub at_exit: u32,
    /// The flag passed to `swapEndiannessOfObject`.
    pub flag: u32,
}

include!("m5p_tmpl_fortype_table.rs.in");

/// The word of copy `f` in `table` (a compile error when `f` is missing).
#[must_use]
pub const fn word_of(table: &[(u32, u32)], f: u32) -> u32 {
    let mut i = 0;
    while i < table.len() {
        if table[i].0 == f {
            return table[i].1;
        }
        i += 1;
    }
    panic!("not in the table");
}

/// The two words of copy `f` in `table`.
#[must_use]
pub const fn pair_of(table: &[(u32, [u32; 2])], f: u32) -> [u32; 2] {
    let mut i = 0;
    while i < table.len() {
        if table[i].0 == f {
            return table[i].1;
        }
        i += 1;
    }
    panic!("not in the table");
}

/// The [`SWAP_LAZY`] entry of copy `f`.
#[must_use]
pub const fn lazy_of(f: u32) -> Lazy {
    let mut i = 0;
    while i < SWAP_LAZY.len() {
        if SWAP_LAZY[i].0 == f {
            return SWAP_LAZY[i].1;
        }
        i += 1;
    }
    panic!("not in the table");
}

/// Checks that `f` is in `table`.
pub const fn listed(table: &[u32], f: u32) {
    let mut i = 0;
    while i < table.len() {
        if table[i] == f {
            return;
        }
        i += 1;
    }
    panic!("not in the table");
}

/// `mov al, 1` over `eax`.
#[must_use]
pub const fn al_one(eax: u32) -> u32 {
    eax & !0xff | 1
}

pub mod bridge {
    //! The originals over game memory.

    use super::{
        ATEXIT, CLASS_DESCRIPTOR, Conv, DESTRUCT, Host, Lazy, NO_NAME, PHYRE_FREE, PHYRE_PRINTF,
        PUSH_NIL, PUSH_OBJECT, RELEASE_ELEMENTS, RELEASE_OWNED, RIGID_BODY_SHAPES_CALLS,
        RIGID_BODY_SHAPES_DATA, SWAP_ENDIANNESS_OF_OBJECT, SWAP_VISITORS, WITH_OWNER, al_one,
    };

    /// The scalar deleting destructor `(this, flags)`: `vtable`, the base
    /// destructor, `PhyreFree(this)` when `flags & 1`; `this`.
    pub fn delete(h: &mut impl Host, this: u32, flags: u32, vtable: u32) -> u32 {
        let vt = h.rebase(vtable);
        h.write(this, vt);
        h.call_sym(DESTRUCT, Conv::Thiscall(this), &[]);
        if flags & 1 != 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[this]);
        }
        this
    }

    /// `swapEndiannessOfObject(p, flag byte != 0)` on the descriptor at
    /// `desc` (already where it is); `al = 1` over its `eax`.
    fn swap_on(h: &mut impl Host, desc: u32, p: u32, flag: u32) -> u32 {
        let on = u32::from(flag & 0xff != 0);
        al_one(h.call_sym(SWAP_ENDIANNESS_OF_OBJECT, Conv::Thiscall(desc), &[p, on]))
    }

    /// `swapEndianness(p, flag)`: [`SWAP_ENDIANNESS_OF_OBJECT`] on the
    /// class descriptor `desc`.
    pub fn swap(h: &mut impl Host, desc: u32, p: u32, flag: u32) -> u32 {
        let d = h.rebase(desc);
        swap_on(h, d, p, flag)
    }

    /// `swapEndianness(p, flag)` of a class whose `PBase` is at `offset`:
    /// `p + offset` (0 for 0) on `desc`.
    pub fn swap_base(h: &mut impl Host, [desc, offset]: [u32; 2], p: u32, flag: u32) -> u32 {
        let q = if p == 0 { 0 } else { p.wrapping_add(offset) };
        swap(h, desc, q, flag)
    }

    /// `swapEndianness(p, flag)` on the descriptor `getter()` (cdecl)
    /// returns.
    pub fn swap_get(h: &mut impl Host, getter: u32, p: u32, flag: u32) -> u32 {
        let d = h.call_sym(getter, Conv::Cdecl, &[]);
        swap_on(h, d, p, flag)
    }

    /// `swapEndianness(p, flag)`: `yes(p)` when the flag byte is set, else
    /// `no(p)` (cdecl); `al = 1` over its `eax`.
    pub fn swap_pair(h: &mut impl Host, [yes, no]: [u32; 2], p: u32, flag: u32) -> u32 {
        let f = if flag & 0xff != 0 { yes } else { no };
        al_one(h.call_sym(f, Conv::Cdecl, &[p]))
    }

    /// The [`super::SWAP_LAZY`] helper `(p)`: builds the static descriptor
    /// on first use (guard bit 0; constructor, flags bit 1 cleared, the
    /// vtable, its destructor registered with `atexit`), then
    /// `swapEndiannessOfObject(p, flag)` on it; its `eax`.
    pub fn swap_lazy(h: &mut impl Host, l: &Lazy, p: u32) -> u32 {
        let guard = h.rebase(l.guard);
        let d = h.rebase(l.desc);
        let g: u32 = h.read(guard);
        if g & 1 == 0 {
            h.write(guard, g | 1);
            h.call_sym(l.ctor, Conv::Thiscall(d), &[]);
            let flags = h.rebase(l.flags);
            let v: u32 = h.read(flags);
            h.write(flags, v & !2);
            let vt = h.rebase(l.vtable);
            h.write(d, vt);
            let at_exit = h.rebase(l.at_exit);
            h.call_sym(ATEXIT, Conv::Cdecl, &[at_exit]);
        }
        h.call_sym(SWAP_ENDIANNESS_OF_OBJECT, Conv::Thiscall(d), &[p, l.flag])
    }

    /// `destroyInstance(p)`: the object at `p - adjust` (0 when `p` is 0,
    /// as the original, which then reads address 0) runs the virtual
    /// method at `vtable + slot` (a deleting destructor) with 0; `al = 1`
    /// over its `eax`.
    pub fn destroy_virtual(h: &mut impl Host, [adjust, slot]: [u32; 2], p: u32) -> u32 {
        let object = if p == 0 { 0 } else { p.wrapping_sub(adjust) };
        let vtable: u32 = h.read(object);
        let f: u32 = h.read(vtable.wrapping_add(slot));
        al_one(h.call(f, Conv::Thiscall(object), &[0]))
    }

    /// `getPointerFromScriptContext(L, out)`: `*out = get(L)` (cdecl);
    /// that value.
    pub fn get_pointer(h: &mut impl Host, get: u32, l: u32, out: u32) -> u32 {
        let v = h.call_sym(get, Conv::Cdecl, &[l]);
        h.write(out, v);
        v
    }

    /// `destroyInstance(p)`: `dtor` (thiscall) on `p + member`, then on
    /// `p`; `al = 1` over its `eax`.
    pub fn destroy_pair(h: &mut impl Host, [dtor, member]: [u32; 2], p: u32) -> u32 {
        h.call_sym(dtor, Conv::Thiscall(p.wrapping_add(member)), &[]);
        al_one(h.call_sym(dtor, Conv::Thiscall(p), &[]))
    }

    /// `destroyInstance(p)`: `dtor` (thiscall) on `p`; `al = 1` over its
    /// `eax`.
    pub fn destroy_call(h: &mut impl Host, dtor: u32, p: u32) -> u32 {
        al_one(h.call_sym(dtor, Conv::Thiscall(p), &[]))
    }

    /// The "not implemented" accessors `(this; L, value)`:
    /// `PhyrePrintf(2, format, name)`, the type's name at `this + 0x18`
    /// or [`NO_NAME`]; its `eax`.
    pub fn script_value(h: &mut impl Host, this: u32, format: u32) -> u32 {
        let name: u32 = h.read(this.wrapping_add(0x18));
        let name = if name == 0 { h.rebase(NO_NAME) } else { name };
        let format = h.rebase(format);
        h.call_sym(PHYRE_PRINTF, Conv::Cdecl, &[2, format, name])
    }

    /// `putPointerToScriptContext(L, pp)`: the object `*pp` (its `PBase`
    /// at `+offset`) pushed with `class`, or nil when `*pp` is 0; the
    /// last call's `eax`.
    pub fn put_pointer(h: &mut impl Host, [offset, class]: [u32; 2], l: u32, pp: u32) -> u32 {
        let object: u32 = h.read(pp);
        if object != 0 {
            let class = h.rebase(class);
            h.call_sym(
                PUSH_OBJECT,
                Conv::Cdecl,
                &[l, object.wrapping_add(offset), class],
            )
        } else {
            h.call_sym(PUSH_NIL[0], Conv::Cdecl, &[l, 1]);
            h.call_sym(PUSH_NIL[1], Conv::Cdecl, &[l])
        }
    }

    /// `destroyInstance(p)` of an owned name: `PhyreFree([p + 0xc])` when
    /// `[p + 0x10]` bit 0 is set; `al = 1` over that call's `eax`, or over
    /// `p`.
    pub fn destroy_owned_name(h: &mut impl Host, p: u32) -> u32 {
        let flags: u8 = h.read(p.wrapping_add(0x10));
        if flags & 1 == 0 {
            return al_one(p);
        }
        let name: u32 = h.read(p.wrapping_add(0xc));
        al_one(h.call_sym(PHYRE_FREE, Conv::Cdecl, &[name]))
    }

    /// Slot 13: -1.
    #[must_use]
    pub const fn minus_one() -> u32 {
        0xffff_ffff
    }

    /// Unlinks the list node at `p + 4` (`[next, prev]`) and points it at
    /// itself; the old `prev`.
    pub fn unlink(h: &mut impl Host, p: u32) -> u32 {
        let node = p.wrapping_add(4);
        let next: u32 = h.read(node);
        let prev: u32 = h.read(node.wrapping_add(4));
        h.write(next.wrapping_add(4), prev);
        h.write(prev, next);
        h.write(node, node);
        h.write(node.wrapping_add(4), node);
        prev
    }

    /// `swapEndianness(p, flag)` of `PClassDescriptor` objects:
    /// `swapEndiannessOfObject` inlined on [`CLASS_DESCRIPTOR`], its
    /// visitor `{0, [descriptor + 0x1c], p + 0x18 (0 for 0), flag}` in
    /// scratch memory, run by the flag's visitor; `al = 1` over its `eax`.
    pub fn swap_class_descriptor(h: &mut impl Host, p: u32, flag: u32) -> u32 {
        let on = flag & 0xff != 0;
        let object = if p == 0 { 0 } else { p.wrapping_add(0x18) };
        let d = h.rebase(CLASS_DESCRIPTOR);
        let members: u32 = h.read(d.wrapping_add(0x1c));
        let visitor = h.scratch();
        h.write(visitor, 0u32);
        h.write(visitor + 4, members);
        h.write(visitor + 8, object);
        // junk read: [ebp-0x3..-0x1] (the flag word's top bytes) never
        // written; 0 (brief 2026-10-24)
        h.write(visitor + 12, u32::from(on));
        let f = if on {
            SWAP_VISITORS[0]
        } else {
            SWAP_VISITORS[1]
        };
        al_one(h.call_sym(f, Conv::Thiscall(d), &[visitor]))
    }

    /// `destroyInstance(p)` of a `PNamespace` (at `p - 8`, 0 for 0): its
    /// owned descriptors released; `al = 1` over that `eax`.
    pub fn destroy_namespace(h: &mut impl Host, p: u32) -> u32 {
        let ns = if p == 0 { 0 } else { p.wrapping_sub(8) };
        al_one(h.call_sym(RELEASE_OWNED, Conv::Thiscall(ns), &[]))
    }

    /// `destroyInstance(p)` (`0x004415a0`): the first destructor, the
    /// owner at `+0x40` releases `p` (and the field is cleared) when set,
    /// the second destructor; `al = 1` over its `eax`.
    pub fn destroy_with_owner(h: &mut impl Host, p: u32) -> u32 {
        h.call_sym(WITH_OWNER[0], Conv::Thiscall(p), &[]);
        let owner: u32 = h.read(p.wrapping_add(0x40));
        if owner != 0 {
            h.call_sym(WITH_OWNER[1], Conv::Thiscall(owner), &[p]);
            h.write(p.wrapping_add(0x40), 0u32);
        }
        al_one(h.call_sym(WITH_OWNER[2], Conv::Thiscall(p), &[]))
    }

    /// `destroyInstance(p)` (`0x004d7140`): an owned array (`[p + 8]`
    /// bit 31 clear) released by [`RELEASE_ELEMENTS`]`(data, count)`, then
    /// both words zeroed; `al = 1` (over the release's `eax`).
    pub fn destroy_array_elements(h: &mut impl Host, p: u32) -> u32 {
        let count: u32 = h.read(p.wrapping_add(8));
        let eax = if count >> 31 == 0 {
            let data: u32 = h.read(p.wrapping_add(0xc));
            h.call_sym(RELEASE_ELEMENTS, Conv::Cdecl, &[data, count & 0x7fff_ffff])
        } else {
            0
        };
        h.write(p.wrapping_add(0xc), 0u32);
        h.write(p.wrapping_add(8), 0u32);
        al_one(eax)
    }

    /// `destroyInstance(p)` (`0x005f8960`): the array at `p - 1` (0 for
    /// 0): its block at `+0x20` freed when owned (`[+0x1c]` bit 31 clear)
    /// and not 0, both words zeroed; `al = 1` (over free's `eax`).
    pub fn destroy_shape_array(h: &mut impl Host, p: u32) -> u32 {
        let a = if p == 0 { 0 } else { p.wrapping_sub(1) };
        let count: u32 = h.read(a.wrapping_add(0x1c));
        let mut eax = 0;
        if count >> 31 == 0 {
            let data: u32 = h.read(a.wrapping_add(0x20));
            if data != 0 {
                eax = h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
            }
        }
        h.write(a.wrapping_add(0x20), 0u32);
        h.write(a.wrapping_add(0x1c), 0u32);
        al_one(eax)
    }

    /// The descriptor constructor `0x005dd0a0` (`this`): the base
    /// constructor with the namespace, the name, size 8, alignment 4,
    /// `PBase`'s descriptor as the parent and 0; flags bit 1; the vtable;
    /// the default, validation and write-mask buffers; `this`.
    pub fn construct_rigid_body_shapes(h: &mut impl Host, this: u32) -> u32 {
        let [
            base_descriptor,
            namespace,
            ctor,
            defaults,
            validation,
            write_mask,
        ] = RIGID_BODY_SHAPES_CALLS;
        let [
            name_cell,
            vtable,
            buf_default,
            buf_validation,
            buf_write_mask,
        ] = RIGID_BODY_SHAPES_DATA;
        let parent = h.call_sym(base_descriptor, Conv::Cdecl, &[]);
        let name_cell = h.rebase(name_cell);
        let name: u32 = h.read(name_cell);
        let ns = h.call_sym(namespace, Conv::Cdecl, &[]);
        h.call_sym(ctor, Conv::Thiscall(this), &[ns, name, 8, 4, parent, 0]);
        let flags: u32 = h.read(this.wrapping_add(0x90));
        h.write(this.wrapping_add(0x90), flags | 2);
        let vt = h.rebase(vtable);
        h.write(this, vt);
        for (f, buffer) in [
            (defaults, buf_default),
            (validation, buf_validation),
            (write_mask, buf_write_mask),
        ] {
            let b = h.rebase(buffer);
            h.call_sym(f, Conv::Thiscall(this), &[b]);
        }
        this
    }
}
