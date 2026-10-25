//! `PType` and `PClassDescriptor`: the type system the `PBinary` loader
//! reads clusters through.
//!
//! A class descriptor (`PClassDescriptor`, from `0x0043afb0`'s
//! constructor) holds, among others:
//!
//! | offset | field |
//! |---|---|
//! | `+0x18` | the descriptor's dynamic type (read by `getDynamicType`) |
//! | `+0x24`, `+0x28` | the fixup functions (`PType::setFixupFunctions`) |
//! | `+0x3c` | the namespace it belongs to |
//! | `+0x40` | the parent class descriptor (0 at the root) |
//! | `+0x64` | the default buffer |
//! | `+0x68` | the validation buffer |
//! | `+0x6c` | the write-mask buffer |
//! | `+0x74`, `+0x78` | the over-allocation (count, size) |
//! | `+0x90` | flags: bit 31 the buffers are computed, bit 30 added to its namespace |
//!
//! The per-class descriptor templates (`PClassDescriptorForType<T>`,
//! `...Concrete<T>`, `...Abstract<T>`, `...WithoutDefaultConstructor<T>`)
//! share vtable slots: 10 (`+0x28`) constructs an instance in place,
//! 11 (`+0x2c`) destroys one, 14 (`+0x38`) is a hook most classes leave
//! returning 0. Many of their bodies are the same few bytes; each copy is
//! its own original and gets its own hook.

use ffx_bridge::{Global, Mem};
use ffx_graphic::host::{Conv, Host};

/// A class descriptor's flags word.
pub const FLAGS: u32 = 0x90;
/// Flag: the default, validation and write-mask buffers are computed.
pub const COMPUTED: u32 = 0x8000_0000;
/// Flag: the descriptor was added to its namespace.
pub const IN_NAMESPACE: u32 = 0x4000_0000;
/// The parent class descriptor.
pub const PARENT: u32 = 0x40;
/// The namespace.
pub const NAMESPACE: u32 = 0x3c;
/// The default buffer.
pub const DEFAULT_BUFFER: u32 = 0x64;

/// `PNamespace::add(namespace, descriptor)` (thiscall, one word).
pub const NAMESPACE_ADD: u32 = 0x0043_e660;
/// `PClassDescriptor::calculateDefaultBuffer` (thiscall).
pub const CALCULATE_DEFAULT: u32 = 0x0043_c420;
/// `PClassDescriptor::calculateValidationBuffer` (thiscall).
pub const CALCULATE_VALIDATION: u32 = 0x0043_c630;
/// `PTypeDispenser::RegisterType(type)` (cdecl).
pub const REGISTER_TYPE: u32 = 0x0043_95b0;
/// `PhyreEngine`'s free (cdecl, one word).
pub const FREE: u32 = 0x0042_fb20;
/// `PClassDataMemberDynamic`'s default instance marker
/// (`IsDefaultInstance` compares a member's `+0x10` with it).
pub const DEFAULT_INSTANCE: Global<u32> = Global::new(0x00c0_b238);

/// Pure parts.
pub mod core {
    /// What the "construct in place" slot 10 of a class without a
    /// default constructor returns (`xor al, al; ret 4`): `al` = false.
    pub const CONSTRUCT_REFUSED: u8 = 0;
    /// What a slot 11 with nothing to destroy returns (`mov al, 1; ret
    /// 4`): `al` = true.
    pub const DESTRUCT_DONE: u8 = 1;
    /// What slot 14 returns where a class leaves it (`xor eax, eax; ret
    /// 4`).
    pub const SLOT14: u32 = 0;

    /// Whether the descriptor at `desc` was added to its namespace
    /// (`0x0043cfb0`): flags bit 30.
    #[must_use]
    pub const fn in_namespace(flags: u32) -> u32 {
        (flags >> 30) & 1
    }

    /// What a "construct two words in place" slot leaves in `eax`: the
    /// instance pointer with `al` = 1 (`mov eax, [p]; ...; mov al, 1`).
    #[must_use]
    pub const fn pointer_with_true(p: u32) -> u32 {
        (p & !0xff) | 1
    }

    /// `0x00507b50`'s result: 0 when the lookup found the value, else
    /// [`crate::error::NOT_FOUND`].
    #[must_use]
    pub const fn found_or_error(value: u32) -> u32 {
        if value == 0 {
            crate::error::NOT_FOUND
        } else {
            0
        }
    }
}

/// The ports over game memory.
pub mod bridge {
    use super::{
        CALCULATE_DEFAULT, CALCULATE_VALIDATION, COMPUTED, Conv, DEFAULT_BUFFER, DEFAULT_INSTANCE,
        FLAGS, FREE, Host, IN_NAMESPACE, Mem, NAMESPACE, NAMESPACE_ADD, PARENT, REGISTER_TYPE,
        core,
    };

    /// `PClassDescriptor::isInNamespace(desc)` (`0x0043cfb0`, thiscall):
    /// flags bit 30, all of `eax`.
    pub fn in_namespace(m: &impl Mem, desc: u32) -> u32 {
        core::in_namespace(m.read(desc + FLAGS))
    }

    /// `PClassDescriptor::addToNamespace(desc)` (`0x0043c140`,
    /// thiscall): `PNamespace::add` on its namespace, then the flag;
    /// returns `add`'s `eax`.
    pub fn add_to_namespace(h: &mut impl Host, desc: u32) -> u32 {
        let ns: u32 = h.read(desc + NAMESPACE);
        let r = h.call_sym(NAMESPACE_ADD, Conv::Thiscall(ns), &[desc]);
        let flags: u32 = h.read(desc + FLAGS);
        h.write(desc + FLAGS, flags | IN_NAMESPACE);
        r
    }

    /// `PClassDescriptor::getDefaultBuffer(desc)` (`0x0043cb80`,
    /// thiscall): computes the buffers the first time (slot 12 first when
    /// the descriptor is not in its namespace yet), returns the default
    /// buffer.
    pub fn default_buffer(h: &mut impl Host, desc: u32) -> u32 {
        let flags: u32 = h.read(desc + FLAGS);
        if flags & COMPUTED == 0 {
            if flags & IN_NAMESPACE == 0 {
                h.call_virtual(desc, 12, &[]);
            }
            h.call_sym(CALCULATE_DEFAULT, Conv::Thiscall(desc), &[]);
            h.call_sym(CALCULATE_VALIDATION, Conv::Thiscall(desc), &[]);
        }
        h.read(desc + DEFAULT_BUFFER)
    }

    /// `PClassDescriptor::inheritsFrom(desc, other)` (`0x00431420`,
    /// thiscall): whether `other` is `desc` or one of its parents. `al`;
    /// the rest of `eax` holds `other`'s upper bytes (`mov eax, [ebp+8]`)
    /// unless `desc` is 0, where the original leaves the caller's.
    pub fn inherits_from(m: &impl Mem, desc: u32, other: u32) -> u32 {
        if desc == 0 {
            return 0;
        }
        let mut d = desc;
        while d != 0 {
            if d == other {
                return (other & !0xff) | 1;
            }
            d = m.read(d + PARENT);
        }
        other & !0xff
    }

    /// `PClassDescriptorDynamic` slot 12 (`0x00449380`, thiscall): when
    /// its base class descriptor (`+0x40`) is not in its namespace yet,
    /// that descriptor's slot 12 (tail call; its `eax`). Else the flag
    /// test's `eax` (1), or 0 without a base (the original leaves the
    /// caller's `eax` there).
    pub fn bind_base(h: &mut impl Host, desc: u32) -> u32 {
        let base: u32 = h.read(desc + PARENT);
        if base == 0 {
            return 0;
        }
        let r = in_namespace(h, base);
        if r != 0 {
            return r;
        }
        h.call_virtual(base, 12, &[])
    }

    /// `PClassDescriptorDynamic`'s scalar deleting destructor
    /// (`0x00447cb0`, thiscall `(flags)`): the destructor `0x00447880`,
    /// then frees the descriptor when bit 0 is set; returns it.
    pub fn delete_dynamic(h: &mut impl Host, desc: u32, flags: u32) -> u32 {
        h.call_sym(0x0044_7880, Conv::Thiscall(desc), &[]);
        if flags & 1 != 0 {
            h.call_sym(FREE, Conv::Cdecl, &[desc]);
        }
        desc
    }

    /// `PClassDataMember::getDynamicType(member, x)` (`0x00575690`,
    /// thiscall): the member's dynamic resolver's slot 1 when it has one
    /// (`+0x28`, a tail call passing `x`), else its type (`+0x18`).
    pub fn dynamic_type(h: &mut impl Host, member: u32, x: u32) -> u32 {
        let resolver: u32 = h.read(member + 0x28);
        if resolver == 0 {
            return h.read(member + 0x18);
        }
        h.call_virtual(resolver, 1, &[x])
    }

    /// `PClassDataMemberDynamic::IsDefaultInstance(member)`
    /// (`0x00446870`, cdecl): whether its `+0x10` is the default marker.
    pub fn is_default_instance(m: &impl Mem, member: u32) -> u32 {
        let v: u32 = m.read(member + 0x10);
        u32::from(v == DEFAULT_INSTANCE.get(m))
    }

    /// `PType::resolveFixupForAddress(type, a, b, c, d)` (`0x00437980`,
    /// thiscall): the type's fixup function (`+0x28`, cdecl) or
    /// [`crate::error::UNSUPPORTED`].
    pub fn resolve_fixup(h: &mut impl Host, ty: u32, args: [u32; 4]) -> u32 {
        let f: u32 = h.read(ty + 0x28);
        if f == 0 {
            return crate::error::UNSUPPORTED;
        }
        h.call(f, Conv::Cdecl, &args)
    }

    /// `PClassDataMemberArrayDynamic<T>` slot 8 (`0x0044a320`,
    /// `0x00a2dbc0`, thiscall `(array, a, b)`): the member's resizer
    /// (`+0x3c`, thiscall on `array` with `a`, `b`) or
    /// [`crate::error::UNSUPPORTED`].
    pub fn resize_array(h: &mut impl Host, member: u32, array: u32, a: u32, b: u32) -> u32 {
        let f: u32 = h.read(member + 0x3c);
        if f == 0 {
            return crate::error::UNSUPPORTED;
        }
        h.call(f, Conv::Thiscall(array), &[a, b])
    }

    /// A field setter: stores `values` from `this + offset` on and
    /// returns the last one (the original's `eax`).
    pub fn set_fields(m: &mut impl Mem, this: u32, offset: u32, values: &[u32]) -> u32 {
        let mut last = 0;
        for (i, &v) in values.iter().enumerate() {
            m.write(this + offset + 4 * i as u32, v);
            last = v;
        }
        last
    }

    /// The descriptors' "construct two words" slot 10 (`0x00449400`
    /// `PArray<u8>`, `0x004595e0` `PArray<PTypedObject>`, `0x005cba80`,
    /// `0x00a2d500`; stdcall `(p)`): zeroes the two words at `p` when it
    /// is not 0. [`core::pointer_with_true`] in `eax`.
    pub fn construct_zero_pair(m: &mut impl Mem, p: u32) -> u32 {
        if p != 0 {
            m.write(p, 0u32);
            m.write(p + 4, 0u32);
        }
        core::pointer_with_true(p)
    }

    /// A descriptor's "construct in place" slot 10 that runs a
    /// constructor (thiscall, at `ctor`) on `p` when it is not 0
    /// (`0x004694b0`, `0x00506b30`); `al` = 1.
    pub fn construct_with(h: &mut impl Host, ctor: u32, p: u32) -> u8 {
        if p != 0 {
            h.call_sym(ctor, Conv::Thiscall(p), &[]);
        }
        1
    }

    /// A descriptor's slot 11 that runs a destructor (thiscall at `dtor`)
    /// on the object `adjust` bytes before `p`, or on 0 for 0
    /// (`0x00506b50` `PNode`); `al` = 1.
    pub fn destroy_with(h: &mut impl Host, dtor: u32, adjust: u32, p: u32) -> u8 {
        let this = if p == 0 { 0 } else { p.wrapping_sub(adjust) };
        h.call_sym(dtor, Conv::Thiscall(this), &[]);
        1
    }

    /// A descriptor's slot 11 that calls virtual `slot` of the object
    /// `adjust` bytes before `p` (0 for 0) with one word 0
    /// (`0x004469a0`, `0x00449580`: their deleting destructors); `al` = 1.
    pub fn destroy_virtual(h: &mut impl Host, slot: u32, adjust: u32, p: u32) -> u8 {
        let this = if p == 0 { 0 } else { p.wrapping_sub(adjust) };
        h.call_virtual(this, slot, &[0]);
        1
    }

    /// `PClassDescriptorWithoutDefaultConstructor<PClassDataMemberDynamic>::constructInstance(p)`
    /// (`0x00446980`): `0x004465c0(p)` (cdecl), its `eax`.
    pub fn construct_member_dynamic(h: &mut impl Host, p: u32) -> u32 {
        h.call_sym(0x0044_65c0, Conv::Cdecl, &[p])
    }

    /// `0x00575d70` (thiscall `(x)`): the array of `this`'s first word
    /// seen as a counted block: 0 when it is 0 or `x`, else its address
    /// less 4 (the block's header).
    pub fn block_header(m: &impl Mem, this: u32, x: u32) -> u32 {
        let p: u32 = m.read(this);
        if p == x || p == 0 {
            0
        } else {
            p.wrapping_sub(4)
        }
    }

    /// The `PClassDataMember` walkers `0x00465830`..`0x00465a40` (twelve
    /// copies, thiscall): whether the member's class (`+0x18`) lists the
    /// class `+0x1c` (its `+0x2c` chain through `+0x40`) and its count
    /// (`+0x40`) is not 0; 1 without a class. `al` only.
    pub fn member_lists_class(m: &impl Mem, this: u32) -> u8 {
        let class: u32 = m.read(this + 0x18);
        if class == 0 {
            return 1;
        }
        let want: u32 = m.read(this + 0x1c);
        let mut e: u32 = m.read(class + 0x2c);
        while e != 0 {
            if e == want {
                let n: u32 = m.read(class + 0x40);
                return u8::from(n != 0);
            }
            e = m.read(e + 0x40);
        }
        0
    }

    /// `0x00468870` (cdecl): registers the type at `0x00c96b40` and adds
    /// it to its namespace; `addToNamespace`'s `eax`.
    pub fn register_cluster_header(h: &mut impl Host) -> u32 {
        let ty = h.rebase(0x00c9_6b40);
        h.call(h.rebase(REGISTER_TYPE), Conv::Cdecl, &[ty]);
        add_to_namespace(h, ty)
    }

    /// `PAnimationKeyDataType`'s parser (`0x00507b50`, cdecl `(out,
    /// name)`): looks `name` up (`0x00507a30`), stores the value at `out`;
    /// 0 when found, else [`crate::error::NOT_FOUND`].
    pub fn parse_key_data_type(h: &mut impl Host, out: u32, name: u32) -> u32 {
        let v = h.call_sym(0x0050_7a30, Conv::Cdecl, &[name]);
        h.write(out, v);
        core::found_or_error(v)
    }
}
