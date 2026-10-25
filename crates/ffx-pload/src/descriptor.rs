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

    /// `0x0043be30` (cdecl `(out, name)`): the class descriptor named
    /// `name` (`0x00439040()` gives the type dispenser, `0x00437570` looks
    /// the name up, thiscall) into `out`; 0, or
    /// [`crate::error::NOT_FOUND`] for none (and for no name, storing 0).
    pub fn find_class_descriptor(h: &mut impl Host, out: u32, name: u32) -> u32 {
        if name == 0 {
            h.write(out, 0u32);
            return core::found_or_error(0);
        }
        let dispenser = h.call_sym(0x0043_9040, Conv::Cdecl, &[]);
        let v = h.call_sym(0x0043_7570, Conv::Thiscall(dispenser), &[name]);
        h.write(out, v);
        core::found_or_error(v)
    }

    /// An enum value table's registration (`0x00507ad0`, cdecl
    /// `(entry)`): [`crate::error::NO_NAME`] for an entry without a name,
    /// [`crate::error::DUPLICATE`] when the name is registered already
    /// (`0x00507a30`), else pushes it on the list at `0x00ca9734`; 0.
    pub fn register_enum_entry(h: &mut impl Host, entry: u32) -> u32 {
        let name: u32 = h.read(entry);
        if name == 0 {
            return crate::error::NO_NAME;
        }
        if h.call_sym(0x0050_7a30, Conv::Cdecl, &[name]) != 0 {
            return crate::error::DUPLICATE;
        }
        let head = h.rebase(0x00ca_9734);
        let first: u32 = h.read(head);
        h.write(entry + 4, first);
        h.write(head, entry);
        0
    }

    /// Fills `n` 0x14-byte entries at `at` (`0x0043bcf0`, cdecl): two
    /// zero words, three `-1` words each (an entry at address 0 is
    /// skipped, as the original's null test does).
    pub fn init_entries(m: &mut impl Mem, at: u32, n: u32) {
        let mut e = at;
        for _ in 0..n {
            if e != 0 {
                m.write(e, [0u32, 0, u32::MAX, u32::MAX, u32::MAX]);
            }
            e = e.wrapping_add(0x14);
        }
    }

    /// `0x0043dde0` (thiscall on a namespace): for each class descriptor
    /// of its list at `+8` (linked through the descriptors' `+0x2c`)
    /// whose flags have bit 0, slot 7 with 1 (its deleting destructor).
    /// The next one is read before the call.
    pub fn release_owned(h: &mut impl Host, ns: u32) {
        let head = ns + 8;
        let entry = |link: u32| {
            if link == head || link == 0 {
                0
            } else {
                link.wrapping_sub(0x2c)
            }
        };
        let first: u32 = h.read(head);
        let mut d = entry(first);
        while d != 0 {
            let link: u32 = h.read(d + 0x2c);
            let next = entry(link);
            let flags: u32 = h.read(d + FLAGS);
            if flags & 1 != 0 {
                h.call_virtual(d, 7, &[1]);
            }
            d = next;
        }
    }

    /// `PClassDataMemberArray::PClassDataMemberArray(member, a, b, c, d,
    /// e, f, g)` (`0x00575ef0`, thiscall): the `PClassDataMember`
    /// constructor (`0x005752a0`) with `a..f`, then `g`, `f`, its vtable;
    /// returns `member`.
    pub fn member_array_ctor(h: &mut impl Host, member: u32, args: [u32; 7]) -> u32 {
        h.call_sym(0x0057_52a0, Conv::Thiscall(member), &args[..6]);
        h.write(member + 0x2c, args[6]);
        h.write(member + 0x30, args[5]);
        let vt = h.rebase(0x00b3_0a60);
        h.write(member, vt);
        member
    }

    /// `PClassDataMemberArrayFixedSize::PClassDataMemberArrayFixedSize(member,
    /// a, ty, c, d, e, size)` (`0x00575f30`, thiscall): the
    /// `PClassDataMember` constructor with `(a, ty, c, d, e, 1)`, then
    /// `size`, the type's `+0x20`, 1, its vtable; returns `member`.
    pub fn member_array_fixed_ctor(h: &mut impl Host, member: u32, args: [u32; 6]) -> u32 {
        let [name, ty, offset, flags, extra, size] = args;
        let element: u32 = h.read(ty + 0x20);
        h.call_sym(
            0x0057_52a0,
            Conv::Thiscall(member),
            &[name, ty, offset, flags, extra, 1],
        );
        h.write(member + 0x34, size);
        h.write(member + 0x2c, element);
        h.write(member + 0x30, 1u32);
        let vt = h.rebase(0x00b3_0a90);
        h.write(member, vt);
        member
    }

    /// `0x005765e0` (thiscall `(i)` on an instance list): the address of
    /// instance `i`'s data (`+0x34` plus `i` times the class size, then
    /// the class's `+0x8c`), or 0 when `i` is out of range or the instance
    /// is in the list at `+8` (linked through the first word).
    pub fn instance_data(m: &impl Mem, list: u32, i: u32) -> u32 {
        let count: u32 = m.read(list + 0x3c);
        if i >= count {
            return 0;
        }
        let class: u32 = m.read(list + 0x2c);
        let size: u32 = m.read(class + 0x1c);
        let base: u32 = m.read(list + 0x34);
        let at = size.wrapping_mul(i).wrapping_add(base);
        let mut e: u32 = m.read(list + 8);
        while e != 0 {
            if e == at {
                return 0;
            }
            e = m.read(e);
        }
        let offset: u32 = m.read(class + 0x8c);
        offset.wrapping_add(at)
    }

    /// `PAssetReferenceImport::ResolveFixup(slot, id, c, name)`
    /// (`0x0045d9b0`, cdecl): the cluster named `name`
    /// (`0x0045cd00(name, &g_assetTree)`), else
    /// [`crate::error::MISSING_CLUSTER`]; links `slot` into the import
    /// list of the entry the big-endian `u16` at `id` picks (12-byte
    /// entries at the cluster's `+0x30`, list head at `+8`); 0.
    pub fn resolve_asset_fixup(h: &mut impl Host, slot: u32, id: u32, c: u32, name: u32) -> u32 {
        let _ = c;
        let index: u16 = h.read(id);
        let tree = h.rebase(0x00c9_4030);
        let cluster = h.call_sym(0x0045_cd00, Conv::Cdecl, &[name, tree]);
        if cluster == 0 {
            return crate::error::MISSING_CLUSTER;
        }
        let table: u32 = h.read(cluster + 0x30);
        let entry = table.wrapping_add(u32::from(index.swap_bytes()) * 12);
        let first: u32 = h.read(entry + 8);
        h.write(slot, first);
        h.write(entry + 8, slot);
        0
    }
}
