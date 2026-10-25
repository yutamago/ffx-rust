//! `PhyreEngine`'s `T::Bind()` functions with several members (M4 port
//! round 2, h1-render-a): each registers its class descriptor `d` and,
//! once each, the descriptors of its members.
//!
//! Every copy has the same frame (cdecl, no arguments, returns
//! `addToNamespace`'s `eax`):
//!
//! - `PTypeDispenser::RegisterType(d)` (`0x004395b0`, cdecl);
//! - the member list, in declaration order. Each member is built on the
//!   first call only: a bit of a guard word (a global; several members
//!   share a word, one bit each) says it was built, and is set before the
//!   build. A built member is registered with `atexit` (its destructor).
//!   The kinds ([`Step`]):
//!   - a data member: a member-descriptor constructor (thiscall, six
//!     words: `d`, the member's type, its name and three words: offset,
//!     kind and a flag/count) on the member global. `PClassDataMember`
//!     (`0x005752a0`) for plain members, `0x00575f30` for fixed-size
//!     arrays and `0x00575370` for one other kind. The type is a global
//!     or comes from a type getter (cdecl, `PTypeDispenser::GetType<T>`,
//!     no arguments or one);
//!   - a class member (`PClassMember::PClassMember`, `0x0043a080`,
//!     thiscall `(d, name, flags)`): after the constructor `+0x14` = 0,
//!     and when it has a name that its owner `[member+8]` does not know
//!     yet (`0x004359d0`, thiscall on the owner, the name), the member goes
//!     to the end of the owner's member list at `owner + 0x4c` (a circular
//!     list: `+0` next, `+4` previous);
//!   - an array member (`PClassDataMemberArray`, `0x00575ef0`, thiscall,
//!     seven words: `d`, the array's class descriptor, name, offset, two
//!     words and the element size). Its array descriptor is built inline
//!     first, under its own guard: [`Step::Descriptor`] (the descriptor's
//!     constructor), flag `+0x90` bit 1 cleared, its vtable, `atexit`;
//! - field stores between them, done on every call (vtables and callbacks
//!   of the members' sub-objects, flag bits);
//! - calls whose result is dropped: the array types' `GetType<PArray<T>>
//!   (name)` (the name a global's word), and a few type getters;
//! - `PClassDescriptor::addToNamespace(d)` (`0x0043c140`, thiscall): its
//!   `eax` is the result.
//!
//! [`BINDS`] holds the copies' constants, generated from the machine code
//! (`~/ffx-work/m4/port/h1r2/gen.py`); [`bridge::bind`] is the function.

use ffx_graphic::host::{Conv, Host};

use crate::class_slots::{ADD_TO_NAMESPACE, REGISTER_TYPE};
use crate::registry::ATEXIT;

/// `PClassMember::PClassMember` (thiscall, three words).
pub const CLASS_MEMBER_CTOR: u32 = 0x0043_a080;
/// `PClassDescriptor::findMember(name)` (thiscall, one word): the member
/// of that name or 0 (`0x004359d0`).
pub const FIND_MEMBER: u32 = 0x0043_59d0;
/// `PClassDataMemberArray::PClassDataMemberArray` (thiscall, seven words).
pub const ARRAY_MEMBER_CTOR: u32 = 0x0057_5ef0;

/// A word of the image: an address (rebased) or a constant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Val {
    /// An address in the image.
    Addr(u32),
    /// A constant.
    Imm(u32),
}

/// A member's type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ty {
    /// The descriptor at this global.
    Addr(u32),
    /// The result of this getter (cdecl, no arguments).
    Call(u32),
    /// The result of this getter (cdecl) of one word.
    CallWith(u32, Val),
}

/// One step of a `Bind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// The steps, when bit `.1` of the guard word at `.0` is clear (set
    /// first).
    Guard(u32, u32, &'static [Step]),
    /// A member descriptor's constructor `ctor` on `member`: `(d, type,
    /// name, words)`.
    Member {
        ctor: u32,
        member: u32,
        ty: Ty,
        name: u32,
        words: [u32; 3],
    },
    /// A class member on `member` (`(d, name, flags)`), linked to its
    /// owner's list when the owner doesn't know its name.
    Class { member: u32, name: u32, flags: u32 },
    /// An array member on `member`: `(d, descriptor, name, words)`.
    Array {
        member: u32,
        ty: Ty,
        name: u32,
        words: [u32; 4],
    },
    /// An array descriptor's constructor `ctor` on `obj` (thiscall, no
    /// arguments).
    Descriptor { obj: u32, ctor: u32 },
    /// `atexit(f)`.
    AtExit(u32),
    /// The word at `.0` = `.1`.
    Store(u32, Val),
    /// The word at `.0` |= `.1`.
    Or(u32, u32),
    /// The word at `.0` &= `.1`.
    And(u32, u32),
    /// `f()` or `f(word at .1)` (cdecl), the result dropped.
    Call(u32, Option<u32>),
}

/// A `Bind` copy: its class descriptor and steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bind {
    /// The class descriptor (a global).
    pub d: u32,
    /// The members and stores between `RegisterType` and
    /// `addToNamespace`.
    pub steps: &'static [Step],
}

/// The copies: `(address, constants)`.
pub const BINDS: &[(u32, Bind)] = &include!("h1_bind_table.rs.in");

/// The constants of copy `f` (compile time).
#[must_use]
pub const fn bind_of(f: u32) -> Bind {
    let mut i = 0;
    while i < BINDS.len() {
        if BINDS[i].0 == f {
            return BINDS[i].1;
        }
        i += 1;
    }
    panic!("not in the table");
}

pub mod bridge {
    //! The originals over game memory.

    use super::{
        ADD_TO_NAMESPACE, ARRAY_MEMBER_CTOR, ATEXIT, Bind, CLASS_MEMBER_CTOR, Conv, FIND_MEMBER,
        Host, REGISTER_TYPE, Step, Ty, Val,
    };

    fn value(h: &mut impl Host, v: Val) -> u32 {
        match v {
            Val::Addr(a) => h.rebase(a),
            Val::Imm(x) => x,
        }
    }

    fn type_of(h: &mut impl Host, ty: Ty) -> u32 {
        match ty {
            Ty::Addr(a) => h.rebase(a),
            Ty::Call(f) => h.call_sym(f, Conv::Cdecl, &[]),
            Ty::CallWith(f, v) => {
                let arg = value(h, v);
                h.call_sym(f, Conv::Cdecl, &[arg])
            }
        }
    }

    /// A class member's construction on `member` for `d`.
    fn class_member(h: &mut impl Host, d: u32, member: u32, name: u32, flags: u32) {
        let name = h.rebase(name);
        h.call_sym(CLASS_MEMBER_CTOR, Conv::Thiscall(member), &[d, name, flags]);
        let owner: u32 = h.read(member + 8);
        let own_name: u32 = h.read(member + 0xc);
        h.write(member + 0x14, 0u32);
        let head = owner.wrapping_add(0x4c);
        if own_name == 0 || h.call_sym(FIND_MEMBER, Conv::Thiscall(owner), &[own_name]) != 0 {
            return;
        }
        let last: u32 = h.read(head + 4);
        h.write(member, head);
        h.write(member + 4, last);
        h.write(last, member);
        h.write(head + 4, member);
    }

    fn run(h: &mut impl Host, d: u32, steps: &[Step]) {
        for &step in steps {
            match step {
                Step::Guard(guard, bit, inner) => {
                    let at = h.rebase(guard);
                    let flags: u32 = h.read(at);
                    if flags & bit == 0 {
                        h.write(at, flags | bit);
                        run(h, d, inner);
                    }
                }
                Step::Member {
                    ctor,
                    member,
                    ty,
                    name,
                    words: [w0, w1, w2],
                } => {
                    let t = type_of(h, ty);
                    let member = h.rebase(member);
                    let name = h.rebase(name);
                    h.call_sym(ctor, Conv::Thiscall(member), &[d, t, name, w0, w1, w2]);
                }
                Step::Class {
                    member,
                    name,
                    flags,
                } => {
                    let member = h.rebase(member);
                    class_member(h, d, member, name, flags);
                }
                Step::Array {
                    member,
                    ty,
                    name,
                    words: [w0, w1, w2, w3],
                } => {
                    let t = type_of(h, ty);
                    let member = h.rebase(member);
                    let name = h.rebase(name);
                    h.call_sym(
                        ARRAY_MEMBER_CTOR,
                        Conv::Thiscall(member),
                        &[d, t, name, w0, w1, w2, w3],
                    );
                }
                Step::Descriptor { obj, ctor } => {
                    let obj = h.rebase(obj);
                    h.call_sym(ctor, Conv::Thiscall(obj), &[]);
                }
                Step::AtExit(f) => {
                    let f = h.rebase(f);
                    h.call_sym(ATEXIT, Conv::Cdecl, &[f]);
                }
                Step::Store(at, v) => {
                    let at = h.rebase(at);
                    let v = value(h, v);
                    h.write(at, v);
                }
                Step::Or(at, bits) => {
                    let at = h.rebase(at);
                    let old: u32 = h.read(at);
                    h.write(at, old | bits);
                }
                Step::And(at, mask) => {
                    let at = h.rebase(at);
                    let old: u32 = h.read(at);
                    h.write(at, old & mask);
                }
                Step::Call(f, arg) => match arg {
                    None => {
                        h.call_sym(f, Conv::Cdecl, &[]);
                    }
                    Some(word) => {
                        let at = h.rebase(word);
                        let arg: u32 = h.read(at);
                        h.call_sym(f, Conv::Cdecl, &[arg]);
                    }
                },
            }
        }
    }

    /// `T::Bind()`: `addToNamespace`'s `eax`.
    pub fn bind(h: &mut impl Host, k: Bind) -> u32 {
        let d = h.rebase(k.d);
        h.call_sym(REGISTER_TYPE, Conv::Cdecl, &[d]);
        run(h, d, k.steps);
        h.call_sym(ADD_TO_NAMESPACE, Conv::Thiscall(d), &[])
    }
}
