//! M5 stage 1, list `tmpl-concrete`: the slots of the descriptor
//! templates that build instances (`PClassDescriptorConcrete<T>`,
//! `...WithoutDefaultConstructor<T>`, `...Wrapper<T>`, `...Header<T>`) and
//! of the named-semantic type descriptors
//! (`PNamedSemanticDescriptorForType<T>` / `...ForDynamicType<T>`, and the
//! `PType` slots they inherit).
//!
//! All are thiscall on the descriptor; the shims declare the stack words
//! each copy pops.
//!
//! - slot 7 (`+0x1c`), the scalar deleting destructor, `(flags)`, `ret 4`:
//!   the `PClassDescriptorAbstract<T>` shape
//!   ([`crate::m5p_tmpl_abstract_ab::bridge::scalar_deleting_dtor`], the
//!   vtable it stores in [`DTORS`]); the named-semantic descriptors store
//!   none and run `PNamedSemanticDescriptor`'s destructor
//!   ([`NAMED_DTORS`], [`bridge::dtor_without_vtable`]). `eax = this`.
//! - slot 10 (`+0x28`), `constructInstance(p)`, `ret 4`: `al = 1` and
//!   nothing else for the wrappers ([`RETURN_TRUE`]); the class's
//!   constructor on `p` (thiscall), then the fields the derived class sets
//!   ([`CONSTRUCT`], [`bridge::construct_with`]); for the classes without
//!   a default constructor, a copy built from the descriptor's default
//!   instance ([`bridge::construct_from_default`],
//!   [`bridge::construct_default_pair`]).
//! - slot 12 (`+0x30`) of the wrappers, `bind()`: `RegisterType(this)`
//!   ([`WRAPPER_BIND`]).
//! - `PNamedSemanticDescriptorForType<T>` slot 8 (`(entry)`, the entry's
//!   name word `+4`), slot 9 (`(name)`, the class's cdecl lookup,
//!   [`NAMED_FIND`]) and slot 10 (`(array)`, the class's registered
//!   entries copied into a `PArray`, [`NAMED_VALUES`]); slot 0 of the
//!   `PType` family returns `al = 0`.
//! - `PType` slots 5 and 6 (`get/putPointerFromScriptContext`) print an
//!   error naming the type; `PClassDescriptorConcrete<PEntity>`'s slot 3
//!   copies the script value's seven words.
//!
//! The constant of each copy is in the generated tables
//! (`m5p_tmpl_concrete_table.rs.in`, by the copy's address); the shims and
//! the oracle tests read them from here. Image addresses in the tables
//! are rebased before use.

use ffx_bridge::Mem;
use ffx_graphic::host::{Conv, Host};

/// `PhyreFree(block)` (cdecl).
pub const FREE: u32 = 0x0042_fb20;
/// `PhyreMalloc(size, alignment)` (cdecl).
pub const MALLOC: u32 = 0x0042_fb80;
/// `PTypeDispenser::RegisterType(descriptor)` (cdecl).
pub const REGISTER_TYPE: u32 = 0x0043_95b0;
/// `PhyrePrintf(level, format, ...)` (cdecl, varargs).
pub const PRINTF: u32 = 0x0043_52f0;
/// The name `PType`'s script slots print for a type without one.
pub const UNNAMED: u32 = 0x00b0_e588;
/// `PClassDescriptor::getDefaultInstance` (thiscall, no stack words).
pub const GET_DEFAULT: u32 = 0x0043_cbb0;
/// `PObjectAccessor<PEntity &>::Get(state)` (cdecl).
pub const ENTITY_GET: u32 = 0x0044_f7f0;
/// `PClassDescriptorConcrete<PEntity>`'s slot 3.
pub const ENTITY_GET_VALUE: u32 = 0x0045_0350;
/// `PType`'s slot 0 (`xor al, al`, `ret 8`).
pub const TYPE_SLOT0: u32 = 0x0043_79d0;

/// A word a constructor stores: `(offset, value, value is an image
/// address)`.
pub type Store = (u32, u32, bool);

include!("m5p_tmpl_concrete_table.rs.in");

/// The constant of copy `f` in `table` (compile time: a missing copy is a
/// build error).
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

/// Whether `f` is in `table` (compile time, as [`word_of`]).
#[must_use]
pub const fn listed(table: &[u32], f: u32) -> bool {
    let mut i = 0;
    while i < table.len() {
        if table[i] == f {
            return true;
        }
        i += 1;
    }
    panic!("not in the table");
}

/// The constructor and stores of [`CONSTRUCT`] copy `f`.
#[must_use]
pub const fn construct_of(f: u32) -> (u32, &'static [Store]) {
    let mut i = 0;
    while i < CONSTRUCT.len() {
        if CONSTRUCT[i].0 == f {
            return CONSTRUCT[i].1;
        }
        i += 1;
    }
    panic!("not in the table");
}

/// The message of [`SCRIPT_POINTER`] copy `f`.
#[must_use]
pub const fn message_of(f: u32) -> u32 {
    word_of(SCRIPT_POINTER, f)
}

/// The default-instance descriptor and the constructor of
/// [`FROM_DEFAULT`] copy `f`.
#[must_use]
pub const fn from_default_of(f: u32) -> (u32, u32) {
    let mut i = 0;
    while i < FROM_DEFAULT.len() {
        if FROM_DEFAULT[i].0 == f {
            return FROM_DEFAULT[i].1;
        }
        i += 1;
    }
    panic!("not in the table");
}

pub mod bridge {
    //! The generic ports over game memory.

    use super::{
        Conv, ENTITY_GET, FREE, GET_DEFAULT, Host, MALLOC, Mem, PRINTF, REGISTER_TYPE, Store,
        UNNAMED,
    };

    pub use crate::m5p_tmpl_abstract_ab::bridge::scalar_deleting_dtor;

    /// Slot 7 of a descriptor that stores no vtable first: its base
    /// destructor `destruct` (thiscall, no stack words), then `PhyreFree`
    /// when `flags` bit 0 is set; `this`.
    pub fn dtor_without_vtable(h: &mut impl Host, this: u32, flags: u32, destruct: u32) -> u32 {
        h.call_sym(destruct, Conv::Thiscall(this), &[]);
        if flags & 1 != 0 {
            h.call_sym(FREE, Conv::Cdecl, &[this]);
        }
        this
    }

    /// `constructInstance` of a class with nothing to build: true.
    #[must_use]
    pub const fn constructed() -> bool {
        true
    }

    /// `PType`'s slot 0: false.
    #[must_use]
    pub const fn not_matched() -> bool {
        false
    }

    /// A wrapper's `bind()`: `RegisterType(this)`; its `eax`.
    pub fn register_self(h: &mut impl Host, this: u32) -> u32 {
        h.call_sym(REGISTER_TYPE, Conv::Cdecl, &[this])
    }

    /// `constructInstance(p)`: unless `p` is 0, the constructor `ctor`
    /// (thiscall, no stack words) on `p`, then `stores` in order; true.
    pub fn construct_with(h: &mut impl Host, p: u32, ctor: u32, stores: &[Store]) -> bool {
        if p != 0 {
            h.call_sym(ctor, Conv::Thiscall(p), &[]);
            for &(off, value, image) in stores {
                let v = if image { h.rebase(value) } else { value };
                h.write(p.wrapping_add(off), v);
            }
        }
        true
    }

    /// `constructInstance(p)` of a class without a default constructor:
    /// the default instance of the descriptor `desc` (an image global)
    /// less 4 (false when there is none, or it is 4); unless `p` is 0, the
    /// copy constructor `ctor(p; default - 4)` (thiscall, `ret 4`); true.
    pub fn construct_from_default(h: &mut impl Host, p: u32, desc: u32, ctor: u32) -> bool {
        let d = h.rebase(desc);
        let def = h.call_sym(GET_DEFAULT, Conv::Thiscall(d), &[]);
        if def == 0 {
            return false;
        }
        let source = def.wrapping_sub(4);
        if source == 0 {
            return false;
        }
        if p != 0 {
            h.call_sym(ctor, Conv::Thiscall(p), &[source]);
        }
        true
    }

    /// `constructInstance(p)` of a two-word class without a default
    /// constructor: false without the default instance of `desc` (an
    /// image global); else unless `p` is 0, `p = [default, 0]`; true.
    pub fn construct_default_pair(h: &mut impl Host, p: u32, desc: u32) -> bool {
        let d = h.rebase(desc);
        let def = h.call_sym(GET_DEFAULT, Conv::Thiscall(d), &[]);
        if def == 0 {
            return false;
        }
        if p != 0 {
            h.write(p, def);
            h.write(p.wrapping_add(4), 0u32);
        }
        true
    }

    /// `PType`'s `get/putPointerFromScriptContext`: `PhyrePrintf(2,
    /// message, the type's name +0x18 or the unnamed text)`; its `eax`.
    pub fn script_pointer_unsupported(h: &mut impl Host, this: u32, message: u32) -> u32 {
        let name: u32 = h.read(this.wrapping_add(0x18));
        let name = if name == 0 { h.rebase(UNNAMED) } else { name };
        let message = h.rebase(message);
        h.call_sym(PRINTF, Conv::Cdecl, &[2, message, name])
    }

    /// `PClassDescriptorConcrete<PEntity>`'s slot 3
    /// (`getValueFromScriptContext(state, out)`): the seven words the
    /// accessor returns for `state` copied to `out`, in the original's
    /// order; the last word.
    pub fn entity_get_value(h: &mut impl Host, state: u32, out: u32) -> u32 {
        let src = h.call_sym(ENTITY_GET, Conv::Cdecl, &[state]);
        let word = |i: u32| src.wrapping_add(4 * i);
        let at = |i: u32| out.wrapping_add(4 * i);
        let a: u32 = h.read(word(0));
        h.write(at(0), a);
        let a: u32 = h.read(word(1));
        h.write(at(1), a);
        let a: u32 = h.read(word(2));
        let b: u32 = h.read(word(3));
        let c: u32 = h.read(word(4));
        h.write(at(2), a);
        h.write(at(3), b);
        h.write(at(4), c);
        let a: u32 = h.read(word(5));
        h.write(at(5), a);
        let a: u32 = h.read(word(6));
        h.write(at(6), a);
        a
    }

    /// `PNamedSemanticDescriptorForType<T>` slot 8 (`(entry)`): the
    /// entry's name (`+4`), 0 for no entry.
    pub fn semantic_name(m: &impl Mem, entry: u32) -> u32 {
        if entry == 0 {
            return 0;
        }
        m.read(entry.wrapping_add(4))
    }

    /// `PNamedSemanticDescriptorForType<T>` slot 9 (`(name)`): the class's
    /// lookup `find` (cdecl) for a name, 0 for none.
    pub fn semantic_find(h: &mut impl Host, find: u32, name: u32) -> u32 {
        if name == 0 {
            return 0;
        }
        h.call_sym(find, Conv::Cdecl, &[name])
    }

    /// `PNamedSemanticDescriptorForType<T>` slot 10 (`(array)`): the
    /// entries of the class's list (head in the image global `head`,
    /// linked through `+8`) into the `PArray` at `array` (`[count | bit 31
    /// for an external block, data]`). A count that differs reallocates
    /// the block (`PhyreMalloc(4n, 4)`, zeroed; 0xd when that fails), and
    /// frees an owned old block other than the new one. 0.
    pub fn semantic_values(h: &mut impl Host, head: u32, array: u32) -> u32 {
        let head = h.rebase(head);
        let mut first: u32 = h.read(head);
        let mut n = 0u32;
        let mut e = first;
        while e != 0 {
            e = h.read(e.wrapping_add(8));
            n = n.wrapping_add(1);
        }
        let count: u32 = h.read(array);
        if n != count {
            let mut data = 0u32;
            if n != 0 {
                data = h.call_sym(MALLOC, Conv::Cdecl, &[n.wrapping_mul(4), 4]);
                if data == 0 {
                    return 0xd;
                }
                for i in 0..n {
                    h.write(data.wrapping_add(4 * i), 0u32);
                }
            }
            let old: u32 = h.read(array.wrapping_add(4));
            if old != data && count >> 31 == 0 {
                h.call_sym(FREE, Conv::Cdecl, &[old]);
            }
            h.write(array.wrapping_add(4), data);
            h.write(array, n);
            first = h.read(head);
        }
        let mut out: u32 = h.read(array.wrapping_add(4));
        let mut e = first;
        while e != 0 {
            h.write(out, e);
            e = h.read(e.wrapping_add(8));
            out = out.wrapping_add(4);
        }
        0
    }
}
