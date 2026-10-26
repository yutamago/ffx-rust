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
/// `PhyreEngine`'s allocator (cdecl `(size, align)`).
pub const ALLOC: u32 = 0x0042_fb80;
/// The CRT's `memcpy` (cdecl).
pub const MEMCPY: u32 = 0x0094_9256;
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
        ALLOC, CALCULATE_DEFAULT, CALCULATE_VALIDATION, COMPUTED, Conv, DEFAULT_BUFFER,
        DEFAULT_INSTANCE, FLAGS, FREE, GLOBAL_NAMESPACE, Host, IN_NAMESPACE, MEMCPY, Mem,
        NAMESPACE, NAMESPACE_ADD, PARENT, REGISTER_TYPE, core,
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

    /// The type dispenser's name hash (`0x00437570`, `0x00438630`): from
    /// 0x7b5, `h = h * 33 + (byte & 0x1f)` over the name's bytes; 0x7b5
    /// for no name.
    pub fn name_hash(m: &impl Mem, name: u32) -> u32 {
        let mut h: u32 = 0x7b5;
        if name == 0 {
            return h;
        }
        let mut p = name;
        loop {
            let b: u8 = m.read(p);
            if b == 0 {
                return h;
            }
            h = h.wrapping_add(h << 5).wrapping_add(u32::from(b & 0x1f));
            p += 1;
        }
    }

    /// The dispensers' lookups by name (`0x00437570` with `0x004375c0`,
    /// `0x00438630` with `0x00438680`; thiscall `(name)`): the key `(hash,
    /// name)` in `key` (two words; the original's frame), then
    /// `find(this; &key)`; its `eax`.
    pub fn find_by_name(h: &mut impl Host, find: u32, key: u32, this: u32, name: u32) -> u32 {
        let hash = name_hash(h, name);
        h.write(key, [hash, name]);
        h.call_sym(find, Conv::Thiscall(this), &[key])
    }

    /// `0x0043cbb0` (thiscall): the descriptor's instance at `+0x70`, or
    /// (computing the buffers as [`default_buffer`] does) its default
    /// buffer plus `+0x8c`, flagging `0x20000000`; 0 without either.
    pub fn default_instance(h: &mut impl Host, desc: u32) -> u32 {
        let inst: u32 = h.read(desc + 0x70);
        if inst != 0 {
            return inst;
        }
        let buffer = default_buffer(h, desc);
        if buffer == 0 {
            return h.read(desc + 0x70);
        }
        let offset: u32 = h.read(desc + 0x8c);
        let flags: u32 = h.read(desc + FLAGS);
        h.write(desc + FLAGS, flags | 0x2000_0000);
        offset.wrapping_add(buffer)
    }

    /// `PClassDescriptor<PBitmapFontCharInfo...>` slot 10 (`0x00a2d570`,
    /// stdcall `(p)`): `al` = 0 without the font descriptor's instance
    /// (`0x0043cbb0` on `0x00ca37d0`); else clears `p` (a byte, a word, two
    /// floats, four words) and stores the instance at `+0x20`; `al` = 1.
    pub fn construct_with_default(h: &mut impl Host, p: u32) -> u8 {
        let desc = h.rebase(0x00ca_37d0);
        let inst = default_instance(h, desc);
        if inst == 0 {
            return 0;
        }
        if p != 0 {
            h.write(p, 0u8);
            h.write(p + 4, [0u32; 7]);
            h.write(p + 0x20, inst);
        }
        1
    }

    /// A descriptor's slot 10 for a class holding its type (`0x00444760`,
    /// stdcall `(p)`): `[p] = 0`, `[p + 4] = 0x00435740()` when `p` is not
    /// 0; `al` = 1.
    pub fn construct_typed(h: &mut impl Host, p: u32) -> u8 {
        if p != 0 {
            let ty = h.call_sym(0x0043_5740, Conv::Cdecl, &[]);
            h.write(p, 0u32);
            h.write(p + 4, ty);
        }
        1
    }

    /// `PNamespace::remove(ns, desc)` (`0x0043ec40`, thiscall): unlinks
    /// `desc` (its link at `+0x2c`); when the namespace's list at `+8` is
    /// then empty and it is not the global namespace
    /// (`GetGlobalNamespace`, `0x0043e2f0`, called again before), unlinks
    /// the namespace itself (its link at `+0`). `eax` as the original
    /// leaves it: the first descriptor left, the global namespace, or the
    /// namespace's old `prev`.
    pub fn namespace_remove(h: &mut impl Host, ns: u32, desc: u32) -> u32 {
        unlink(h, desc + 0x2c);
        let head = ns + 8;
        let first: u32 = h.read(head);
        if first != head && first != 0 && first.wrapping_sub(0x2c) != 0 {
            return first.wrapping_sub(0x2c);
        }
        let global = h.call_sym(GLOBAL_NAMESPACE, Conv::Cdecl, &[]);
        if ns == global {
            return global;
        }
        h.call_sym(GLOBAL_NAMESPACE, Conv::Cdecl, &[]);
        unlink(h, ns)
    }

    /// Unlinks the intrusive link at `link` and leaves it empty; returns
    /// its old `prev`.
    fn unlink(m: &mut impl Mem, link: u32) -> u32 {
        let next: u32 = m.read(link);
        let prev: u32 = m.read(link + 4);
        m.write(next + 4, prev);
        m.write(prev, next);
        m.write(link, [link, link]);
        prev
    }

    /// A C string compare, as the originals' inline `strcmp`.
    fn same_name(m: &impl Mem, a: u32, b: u32) -> bool {
        let mut i = 0;
        loop {
            let x: u8 = m.read(a + i);
            let y: u8 = m.read(b + i);
            if x != y {
                return false;
            }
            if x == 0 {
                return true;
            }
            i += 1;
        }
    }

    /// The enum value tables' lookups by name (cdecl `(name)`): the entry
    /// of the list at `head` (a global) whose name (at `name_at`) is
    /// `name`, entries linked through `next_at`; 0 for none or no name.
    /// `0x00507a30` (`0x00ca9734`, name `+0`, next `+4`) and eight
    /// others with name `+4`, next `+8` ([`ENUM_TABLES`]).
    pub fn enum_find(m: &impl Mem, head: u32, name_at: u32, next_at: u32, name: u32) -> u32 {
        if name == 0 {
            return 0;
        }
        let mut e: u32 = m.read(m.rebase(head));
        while e != 0 {
            let n: u32 = m.read(e + name_at);
            if same_name(m, n, name) {
                return e;
            }
            e = m.read(e + next_at);
        }
        0
    }

    /// A `PString`'s release (`0x0044af00`, thiscall): unless it does not
    /// own its text (bit 31 of the length word), frees it: a length over
    /// 1 through `0x00434360(text, length)`, a length of 1 through free
    /// when the pointer's bit 0 is clear; then clears both words.
    pub fn string_release(h: &mut impl Host, s: u32) {
        let word: u32 = h.read(s);
        if word >> 31 == 0 {
            let n = word & 0x7fff_ffff;
            let text: u32 = h.read(s + 4);
            if n > 1 {
                h.call_sym(0x0043_4360, Conv::Cdecl, &[text, n]);
            } else if n == 1 && text & 1 == 0 {
                h.call_sym(FREE, Conv::Cdecl, &[text]);
            }
        }
        h.write(s + 4, 0u32);
        h.write(s, 0u32);
    }

    /// A descriptor's slot 10 for a class built in the global namespace
    /// (`0x00449420`, stdcall `(p)`): when `p` is not 0,
    /// `0x00447690(p; GetGlobalNamespace(), 0, 0x00435740())`; `al` = 1.
    pub fn construct_in_global(h: &mut impl Host, p: u32) -> u8 {
        if p != 0 {
            let ty = h.call_sym(0x0043_5740, Conv::Cdecl, &[]);
            let ns = h.call_sym(GLOBAL_NAMESPACE, Conv::Cdecl, &[]);
            h.call_sym(0x0044_7690, Conv::Thiscall(p), &[ns, 0, ty]);
        }
        1
    }

    /// `PClassMember::PClassMember(member, a, name, flags)`
    /// (`0x0043a080`, thiscall): an empty link, `a`, the name and the
    /// flags; with flags bit 0 the name is copied into a new block
    /// (`0x0042fb80(len, 1)`, then `memcpy`; 0 when it fails). Returns
    /// `member`.
    pub fn member_ctor(h: &mut impl Host, member: u32, a: u32, name: u32, flags: u32) -> u32 {
        h.write(member, [member, member, a, name, flags]);
        if flags & 1 != 0 {
            let mut len = 0u32;
            while h.read::<u8>(name + len) != 0 {
                len += 1;
            }
            let size = len + 1;
            let copy = h.call_sym(0x0042_fb80, Conv::Cdecl, &[size, 1]);
            if copy != 0 {
                h.call_sym(0x0094_9256, Conv::Cdecl, &[copy, name, size]);
            }
            h.write(member + 0xc, copy);
        }
        member
    }

    /// `strlen(s) + 1`, as the originals' inline loops count it.
    pub(crate) fn c_size(m: &impl Mem, s: u32) -> u32 {
        let mut n = 0u32;
        while m.read::<u8>(s + n) != 0 {
            n += 1;
        }
        n + 1
    }

    /// A copy of the C string at `s` in a new block (`0x0042fb80(size,
    /// align)`, then `memcpy`); the block, or 0 when the allocation fails.
    pub(crate) fn copy_string(h: &mut impl Host, s: u32, align: u32) -> u32 {
        let size = c_size(h, s);
        let copy = h.call_sym(ALLOC, Conv::Cdecl, &[size, align]);
        if copy != 0 {
            h.call_sym(MEMCPY, Conv::Cdecl, &[copy, s, size]);
        }
        copy
    }

    /// `PArray<u8>::resize(array, n)` (`0x0044a010`, thiscall): nothing
    /// for the same count; else a new block of `n` bytes (none for 0;
    /// [`crate::error::NO_BLOCK`] when the allocation fails), the old
    /// one freed when owned, then the block and the count; 0.
    pub fn array_resize(h: &mut impl Host, array: u32, n: u32) -> u32 {
        let count: u32 = h.read(array);
        if n == count {
            return 0;
        }
        let mut block = 0;
        if n != 0 {
            block = h.call_sym(ALLOC, Conv::Cdecl, &[n, 1]);
            if block == 0 {
                return crate::error::NO_BLOCK;
            }
        }
        let old: u32 = h.read(array + 4);
        if old != block && count >> 31 == 0 && old != 0 {
            h.call_sym(FREE, Conv::Cdecl, &[old]);
        }
        h.write(array + 4, block);
        h.write(array, n);
        0
    }

    /// `PString::PString(dst, &src)` (`0x0044ae10`, thiscall): a copy of
    /// `src`'s text (its pointer less its bit 0) in a new block
    /// ([`copy_string`], alignment 2), or 0 for no text; returns `dst`.
    pub fn string_copy(h: &mut impl Host, dst: u32, src: u32) -> u32 {
        let p: u32 = h.read(src);
        let text = p.wrapping_sub(p & 1);
        let copy = if text == 0 {
            0
        } else {
            copy_string(h, text, 2)
        };
        h.write(dst, copy);
        dst
    }

    /// `0x0043c0a0` (thiscall `(member, extra)` on a class descriptor):
    /// adds `member` to the descriptor's member list (`+0x44`, the
    /// member's link at `+4`). 5 when the members would not fit
    /// (`+0x8c - +0x88 + member->offset (+0x24) + extra > +0x1c`, signed),
    /// [`crate::error::UNSUPPORTED`] without a name (`+0x10`),
    /// [`crate::error::DUPLICATE`] when `0x00435ca0(desc; name)` (thiscall) finds
    /// one; 0.
    pub fn add_member(h: &mut impl Host, desc: u32, member: u32, extra: u32) -> u32 {
        let end: u32 = h.read(desc + 0x8c);
        let base: u32 = h.read(desc + 0x88);
        let offset: u32 = h.read(member + 0x24);
        let need = end
            .wrapping_sub(base)
            .wrapping_add(offset)
            .wrapping_add(extra);
        let size: u32 = h.read(desc + 0x1c);
        if need as i32 > size as i32 {
            return 5;
        }
        let name: u32 = h.read(member + 0x10);
        if name == 0 {
            return crate::error::UNSUPPORTED;
        }
        if h.call_sym(0x0043_5ca0, Conv::Thiscall(desc), &[name]) != 0 {
            return crate::error::DUPLICATE;
        }
        let head = desc + 0x44;
        let last: u32 = h.read(head + 4);
        let link = member + 4;
        h.write(link, [head, last]);
        h.write(last, link);
        h.write(head + 4, link);
        0
    }

    /// `0x004490a0` (thiscall `(member, extra)` on a dynamic class
    /// descriptor): 0 for the default instance; else flushes pending
    /// changes (`0x00449c80` when the two counts at `+0x94`, `+0x96`
    /// differ), [`add_member`], and on success bumps both counts and
    /// rebuilds (`0x00449e40`). [`add_member`]'s result.
    pub fn add_dynamic_member(h: &mut impl Host, desc: u32, member: u32, extra: u32) -> u32 {
        if is_default_instance(h, member) != 0 {
            return 0;
        }
        let [a, b]: [u16; 2] = h.read(desc + 0x94);
        if a != b {
            h.call_sym(0x0044_9c80, Conv::Thiscall(desc), &[]);
        }
        let r = add_member(h, desc, member, extra);
        if r == 0 {
            let [a, b]: [u16; 2] = h.read(desc + 0x94);
            h.write(desc + 0x94, [a.wrapping_add(1), b.wrapping_add(1)]);
            h.call_sym(0x0044_9e40, Conv::Thiscall(desc), &[]);
        }
        r
    }

    /// `PClassDescriptor::updateBaseOffsets(desc)` (`0x0043d6e0`,
    /// thiscall): `+0x88` = the sizes (`+0x84`) of the descriptor and its
    /// parents up to `0x00435740()`'s; `+0x8c` = the parent's `+0x8c` (0
    /// without one) plus its own size when positive. Returns `+0x8c`.
    pub fn update_base_offsets(h: &mut impl Host, desc: u32) -> u32 {
        let root = h.call_sym(0x0043_5740, Conv::Cdecl, &[]);
        let mut total = 0u32;
        h.write(desc + 0x88, total);
        let mut d = desc;
        while d != root {
            let size: u32 = h.read(d + 0x84);
            total = total.wrapping_add(size);
            h.write(desc + 0x88, total);
            d = h.read(d + PARENT);
            if d == 0 {
                break;
            }
        }
        let parent: u32 = h.read(desc + PARENT);
        let own: u32 = h.read(desc + 0x84);
        let base: u32 = if parent == 0 {
            0
        } else {
            h.read(parent + 0x8c)
        };
        let own = if own as i32 > 0 { own } else { 0 };
        let v = base.wrapping_add(own);
        h.write(desc + 0x8c, v);
        v
    }

    /// `0x0042b7c0` (thiscall on a table of objects, `+0x30` data,
    /// `+0x38` count): each object's slot 0 with 1 (its deleting
    /// destructor; a null one reports through the CRT's assertion import
    /// at `0x00b0c5c0`), then the data freed through the hook at
    /// `0x022fb514`.
    pub fn delete_all(h: &mut impl Host, table: u32) {
        let n: u32 = h.read(table + 0x38);
        if n as i32 <= 0 {
            return;
        }
        let data: u32 = h.read(table + 0x30);
        let end = data.wrapping_add(n.wrapping_mul(4));
        let mut p = data;
        while p != end {
            let o: u32 = h.read(p);
            p = p.wrapping_add(4);
            if o == 0 {
                let assert = h.read(h.rebase(0x00b0_c5c0));
                let (msg, file) = (h.rebase(0x00b9_0514), h.rebase(0x00b9_02f0));
                h.call(assert, Conv::Cdecl, &[msg, file, 0x1aff]);
            } else {
                h.call_virtual(o, 0, &[1]);
            }
        }
        let data: u32 = h.read(table + 0x30);
        if data != 0 {
            let free = h.read(h.rebase(0x022f_b514));
            h.call(free, Conv::Cdecl, &[data]);
        }
    }

    /// The `PArray<PTypedObject>` descriptor's slot 10 (`0x00459600`,
    /// stdcall `(p)`): `0x0044d6a0(p; 0x00c93ae8)`, then zeroes `+0xc`
    /// and `+0x10` when `p` is not 0; `al` = 1.
    pub fn construct_typed_array(h: &mut impl Host, p: u32) -> u8 {
        if p != 0 {
            let ty = h.rebase(0x00c9_3ae8);
            h.call_sym(0x0044_d6a0, Conv::Thiscall(p), &[ty]);
            h.write(p + 0xc, [0u32; 2]);
        }
        1
    }
}

/// `PNamespace::GetGlobalNamespace()` (cdecl).
pub const GLOBAL_NAMESPACE: u32 = 0x0043_e2f0;

/// The enum lookups with name `+4` and next `+8`: (function, list head).
pub const ENUM_TABLES: [(u32, u32); 8] = [
    (0x005c_be40, 0x00cb_d908),
    (0x005c_c2a0, 0x00cb_d944),
    (0x009f_48f0, 0x0194_359c),
    (0x00a2_dea0, 0x01a8_5318),
    (0x00a2_e310, 0x01a8_5354),
    (0x00a2_e780, 0x01a8_5390),
    (0x00a2_ebf0, 0x01a8_53cc),
    (0x00a3_2560, 0x01a8_5578),
];
