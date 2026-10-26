//! List `p1-load`, continuation round (2026-10-26), batch 10: class
//! data members and their descriptors, namespace lookups, UTF-8 name
//! hashes, instance-list buffers, small releases.
//!
//! Layouts found here:
//! - `PClassDataMember` (`0x005752a0`): its vtable (`0x00b309d8`), a
//!   `PClassMember` at `+4` (link, the class descriptor at `+0xc`, the
//!   name at `+0x10`, flags at `+0x14`), its type `+0x18`, an empty list
//!   `+0x1c`, its offset `+0x24`, 0 at `+0x28`. The enum flavour
//!   (`0x00575370`, vtable `0x00b309ec`) adds `+0x2c`; the dynamic one
//!   (`0x004461e0`, vtable `0x00b0f900`) a copy of its default name at
//!   `+0x2c`.
//! - `PInstanceList` (`0x005763b0`): its class descriptor `+0x2c`, the
//!   data `+0x30` (the buffer plus the descriptor's `+0x8c`), the buffer
//!   `+0x34`, the counts `+0x3c`/`+0x40`, the bytes over `+0x44`, owned
//!   `+0x48`.
//! - `PAssetReferenceImport` (`0x0045c9e0`): a tree node at `+4` (its
//!   parent word `+0xc` with the colour in bit 0), the name hash `+0x10`
//!   and the name `+0x14` (the tree key), its name `+0x18`, the class
//!   descriptor `+0x1c`, the type `+0x20`, `+0x24`.

use ffx_bridge::Mem;
use ffx_graphic::host::{Conv, Host};

use crate::descriptor::{ALLOC, FREE, GLOBAL_NAMESPACE};

/// `PBase`'s class descriptor getter (`0x00435740`, cdecl).
pub const BASE_DESCRIPTOR: u32 = 0x0043_5740;
/// `PClassDescriptor::PClassDescriptor(ns, name, size, align, base, c)`
/// (thiscall, six words).
pub const DESCRIPTOR_CTOR6: u32 = 0x0043_b0a0;
/// `PClassMember::PClassMember(a, name, flags)` (thiscall).
pub const MEMBER_CTOR: u32 = 0x0043_a080;
/// `PClassDataMember::PClassDataMember` (thiscall, six words).
pub const DATA_MEMBER_CTOR: u32 = 0x0057_52a0;
/// `PClassDataMemberDynamic::PClassDataMemberDynamic` (thiscall, six
/// words).
pub const DYNAMIC_MEMBER_CTOR: u32 = 0x0044_61e0;
/// `PClassDataMember`'s vtable.
pub const DATA_MEMBER_VTABLE: u32 = 0x00b3_09d8;
/// `PClassDataMemberEnum`'s vtable.
pub const ENUM_MEMBER_VTABLE: u32 = 0x00b3_09ec;
/// `PClassDataMemberDynamic`'s vtable.
pub const DYNAMIC_MEMBER_VTABLE: u32 = 0x00b0_f900;
/// The CRT's `memset` (cdecl).
pub const MEMSET: u32 = 0x0094_964c;
/// `PhyrePrintf(level, format, ...)` (cdecl).
pub const PRINTF: u32 = 0x0043_52f0;
/// The CRT import `_stricmp` (cdecl, its word in the import table).
pub const STRICMP_IMPORT: u32 = 0x00b0_c398;

/// Pure parts.
pub mod core {
    /// The number of continuation bytes a UTF-8 lead byte announces, as
    /// `0x0045d640` and `0x0045d520` count them (0 below `0xc0`, up to 5
    /// from `0xfc`).
    #[must_use]
    pub const fn utf8_tail(lead: u8) -> u32 {
        match lead {
            0..=0xbf => 0,
            0xc0..=0xdf => 1,
            0xe0..=0xef => 2,
            0xf0..=0xf7 => 3,
            0xf8..=0xfb => 4,
            0xfc..=0xff => 5,
        }
    }

    /// The lead byte's payload: all of it without continuation bytes,
    /// else its low `6 - tail` bits.
    #[must_use]
    pub const fn utf8_lead_bits(lead: u8, tail: u32) -> u32 {
        let b = lead as u32;
        if tail == 0 {
            b
        } else {
            b & ((1 << (6 - tail)) - 1)
        }
    }

    /// One continuation byte appended to a code point.
    #[must_use]
    pub const fn utf8_push(cp: u32, byte: u8) -> u32 {
        (cp << 6).wrapping_add((byte & 0x3f) as u32)
    }

    /// `0x0045d520`'s step: the hash times 33 plus the code point's low 5
    /// bits.
    #[must_use]
    pub const fn hash_step(hash: u32, cp: u32) -> u32 {
        hash.wrapping_mul(33).wrapping_add(cp & 0x1f)
    }

    /// The default-buffer merge of `0x0043c160`: the default byte where
    /// the validation mask has a bit, the instance's byte elsewhere.
    #[must_use]
    pub const fn merge_default(default: u8, mask: u8, instance: u8) -> u8 {
        (default & mask) | (!mask & instance)
    }

    /// `al` set over the rest of `eax`.
    #[must_use]
    pub const fn with_al(eax: u32, al: bool) -> u32 {
        (eax & !0xff) | al as u32
    }
}

/// The ports over game memory.
pub mod bridge {
    use super::{
        ALLOC, BASE_DESCRIPTOR, Conv, DATA_MEMBER_CTOR, DATA_MEMBER_VTABLE, DESCRIPTOR_CTOR6,
        DYNAMIC_MEMBER_CTOR, DYNAMIC_MEMBER_VTABLE, ENUM_MEMBER_VTABLE, FREE, GLOBAL_NAMESPACE,
        Host, MEMBER_CTOR, MEMSET, Mem, PRINTF, STRICMP_IMPORT, core,
    };
    use crate::descriptor::DEFAULT_INSTANCE;
    use crate::descriptor::bridge::copy_string;

    /// The abstract class descriptors' constructors (thiscall, no
    /// arguments; `0x004473f0` for `PArray<u8>`, `0x0044ac10` for
    /// `PSharray<PString>`): `PClassDescriptor(GetGlobalNamespace(),
    /// *name, 8, 4, PBase's descriptor, 0)` (`0x0043b0a0`), flags bit 1,
    /// the class's vtable, then its default, validation and write-mask
    /// buffers (`buffers`, `+0x10`, `+0x20`: `0x0043d510`, `0x0043d5b0`,
    /// `0x0043d5c0`); returns `this`.
    pub fn abstract_descriptor_ctor(
        h: &mut impl Host,
        this: u32,
        name: u32,
        vtable: u32,
        buffers: u32,
    ) -> u32 {
        let base = h.call_sym(BASE_DESCRIPTOR, Conv::Cdecl, &[]);
        let name: u32 = h.read(h.rebase(name));
        let ns = h.call_sym(GLOBAL_NAMESPACE, Conv::Cdecl, &[]);
        h.call_sym(
            DESCRIPTOR_CTOR6,
            Conv::Thiscall(this),
            &[ns, name, 8, 4, base, 0],
        );
        let flags: u32 = h.read(this + 0x90);
        h.write(this + 0x90, flags | 2);
        let vt = h.rebase(vtable);
        h.write(this, vt);
        let buffers = h.rebase(buffers);
        h.call_sym(0x0043_d510, Conv::Thiscall(this), &[buffers]);
        h.call_sym(0x0043_d5b0, Conv::Thiscall(this), &[buffers + 0x10]);
        h.call_sym(0x0043_d5c0, Conv::Thiscall(this), &[buffers + 0x20]);
        this
    }

    /// `0x005763b0` (thiscall `(n, size, buffer)` on an instance list):
    /// refuses (`al` 0) when `n` instances of the descriptor's size
    /// (`+0x1c`) need more than `size` bytes, or `n` > 1 of a class with
    /// flags bit 2; without `buffer` allocates `size` bytes (alignment
    /// `+0x20`; `al` 0 when that fails), frees the old buffer
    /// (`0x005764a0`), then stores the buffer, data, counts and the
    /// bytes over; `al` 1. The full `eax`.
    pub fn instance_list_set_buffer(
        h: &mut impl Host,
        list: u32,
        n: u32,
        size: u32,
        buffer: u32,
    ) -> u32 {
        let desc: u32 = h.read(list + 0x2c);
        let elem: u32 = h.read(desc + 0x1c);
        let need = elem.wrapping_mul(n);
        if need > size {
            return core::with_al(desc, false);
        }
        if n > 1 {
            let flags: u8 = h.read(desc + 0x90);
            if flags & 4 != 0 {
                return core::with_al(desc, false);
            }
        }
        let mut block = 0;
        if size != 0 && buffer == 0 {
            let align: u32 = h.read(desc + 0x20);
            block = h.call_sym(ALLOC, Conv::Cdecl, &[size, align]);
            if block == 0 {
                return 0;
            }
        }
        h.call_sym(0x0057_64a0, Conv::Thiscall(list), &[]);
        if buffer != 0 {
            block = buffer;
        }
        h.write(list + 0x34, block);
        let offset: u32 = h.read(desc + 0x8c);
        h.write(list + 0x30, offset.wrapping_add(block));
        h.write(list + 0x3c, [n, n, size.wrapping_sub(need)]);
        h.write(list + 0x48, u8::from(buffer == 0));
        core::with_al(n, true)
    }

    /// `PClassDescriptorConcrete<PAssetReferenceImport>::constructInstance(p)`
    /// (`0x0045c9e0`, cdecl): `al` 0 when the class has no default
    /// instance (`0x0043cbb0` on `PBase`'s descriptor); else, for a `p`,
    /// the import's empty node, zero words, the default instance and
    /// `PBase`'s descriptor, then its registration (`0x0045e1a0`); `al`
    /// 1. The full `eax`.
    pub fn construct_asset_import(h: &mut impl Host, p: u32) -> u32 {
        let base = h.call_sym(BASE_DESCRIPTOR, Conv::Cdecl, &[]);
        let def = h.call_sym(0x0043_cbb0, Conv::Thiscall(base), &[]);
        if def == 0 {
            return 0;
        }
        if p == 0 {
            return core::with_al(def, true);
        }
        let base = h.call_sym(BASE_DESCRIPTOR, Conv::Cdecl, &[]);
        let node = p + 4;
        h.write(node, [node, node, node, 0, 0, 0, def, base, 0]);
        let r = h.call_sym(0x0045_e1a0, Conv::Thiscall(p), &[]);
        core::with_al(r, true)
    }

    /// The UTF-8 decoder `0x0045d640` (cdecl `(&cursor)`): the code point
    /// at the cursor (the lead byte's bits, then 6 bits from each
    /// continuation byte, whatever they hold), the cursor moved past it.
    pub fn utf8_next(m: &mut impl Mem, cursor: u32) -> u32 {
        let mut p: u32 = m.read(cursor);
        let lead: u8 = m.read(p);
        let tail = core::utf8_tail(lead);
        let mut cp = core::utf8_lead_bits(lead, tail);
        p = p.wrapping_add(1);
        for _ in 0..tail {
            cp = core::utf8_push(cp, m.read(p));
            p = p.wrapping_add(1);
        }
        m.write(cursor, p);
        cp
    }

    /// The name hash `0x0045d520` (cdecl `(name, seed)`): `seed` for no
    /// name; else, over each UTF-8 code point up to the terminator,
    /// `hash * 33 + (cp & 0x1f)`.
    pub fn utf8_hash(m: &impl Mem, name: u32, seed: u32) -> u32 {
        let mut hash = seed;
        if name == 0 {
            return hash;
        }
        let mut p = name;
        while m.read::<u8>(p) != 0 {
            let lead: u8 = m.read(p);
            let tail = core::utf8_tail(lead);
            let mut cp = core::utf8_lead_bits(lead, tail);
            p = p.wrapping_add(1);
            for _ in 0..tail {
                cp = core::utf8_push(cp, m.read(p));
                p = p.wrapping_add(1);
            }
            hash = core::hash_step(hash, cp);
        }
        hash
    }

    /// `0x00605660` (thiscall): deletes the three objects at `+0x54`,
    /// `+0x58`, `+0x5c` (virtual slot 0 with 1) and clears them, then
    /// releases the arrays at `+0x80` and `+0x88` (count, data: freed
    /// when both are set and the count's bit 31 is clear); 0.
    pub fn release_parts(h: &mut impl Host, this: u32) -> u32 {
        for at in [0x54, 0x58, 0x5c] {
            let obj: u32 = h.read(this + at);
            if obj != 0 {
                h.call_virtual(obj, 0, &[1]);
            }
            h.write(this + at, 0u32);
        }
        for at in [0x80, 0x88] {
            let count: u32 = h.read(this + at);
            if count != 0 {
                let data: u32 = h.read(this + at + 4);
                if data != 0 && count >> 31 == 0 {
                    h.call_sym(FREE, Conv::Cdecl, &[data]);
                }
                h.write(this + at + 4, 0u32);
                h.write(this + at, 0u32);
            }
        }
        0
    }

    /// `0x0057ccf0` (thiscall `(n)` on a pointer array with one inline
    /// slot: count `+0`, bit 31 not owned; data `+4`, or the element
    /// itself for a count of 1): nothing for the same count; else `n`
    /// zero words (inline for `n` <= 1, else a new block,
    /// [`crate::error::NO_BLOCK`] when that fails), the old block freed
    /// when owned, then the data and the count; 0.
    pub fn inline_array_resize(h: &mut impl Host, array: u32, n: u32) -> u32 {
        let count: u32 = h.read(array);
        if n == count {
            return 0;
        }
        let inline = array + 4;
        let old = match count & 0x7fff_ffff {
            0 => 0,
            1 => inline,
            _ => h.read(inline),
        };
        let block = if n > 1 {
            h.call_sym(ALLOC, Conv::Cdecl, &[n.wrapping_mul(4), 4])
        } else {
            inline
        };
        if block == 0 {
            return crate::error::NO_BLOCK;
        }
        for i in 0..n {
            h.write(block + 4 * i, 0u32);
        }
        if old != block && old != inline {
            let count: u32 = h.read(array);
            if count >> 31 == 0 {
                h.call_sym(FREE, Conv::Cdecl, &[old]);
            }
        }
        if block != inline {
            h.write(inline, block);
        }
        h.write(array, n);
        0
    }

    /// `0x0044a470` (cdecl `(dst, src, n)`): `n` `PString` copies; each
    /// source text (its pointer less bit 0) copied into a new block
    /// (alignment 2), 0 for none. A `dst` of 0 stores nothing and leaves
    /// the source where it is; the destination moves on anyway.
    pub fn copy_strings(h: &mut impl Host, dst: u32, src: u32, n: u32) {
        let mut dst = dst;
        let mut src = src;
        for _ in 0..n {
            if dst != 0 {
                let p: u32 = h.read(src);
                let text = p.wrapping_sub(p & 1);
                let copy = if text == 0 {
                    0
                } else {
                    copy_string(h, text, 2)
                };
                h.write(dst, copy);
                src = src.wrapping_add(4);
            }
            dst = dst.wrapping_add(4);
        }
    }

    /// Frees an owned array (count at `at`, bit 31 not owned; data at
    /// `at + 4`) when `owned_if` says so, then clears both words.
    fn release_array(h: &mut impl Host, at: u32, owned_if: fn(u32, u32) -> bool) {
        let count: u32 = h.read(at);
        let data: u32 = h.read(at + 4);
        if count >> 31 == 0 && owned_if(count, data) {
            h.call_sym(FREE, Conv::Cdecl, &[data]);
        }
        h.write(at + 4, 0u32);
        h.write(at, 0u32);
    }

    /// `0x00479930` (thiscall, a destructor): the arrays at `+0x14`,
    /// `+0xc`, `+4`, each freed when owned and set.
    pub fn release_three(h: &mut impl Host, this: u32) {
        for at in [0x14, 0xc, 4] {
            release_array(h, this + at, |_, data| data != 0);
        }
    }

    /// `0x00577f60` (thiscall, a destructor): the array at `+0x20` freed
    /// when owned; `+0x18` when owned with more than one element (one is
    /// inline); `+0x10` when owned and set.
    pub fn release_three_inline(h: &mut impl Host, this: u32) {
        release_array(h, this + 0x20, |_, _| true);
        release_array(h, this + 0x18, |count, _| count & 0x7fff_ffff > 1);
        release_array(h, this + 0x10, |_, data| data != 0);
    }

    /// The decision the data-member constructors share: 4 (a pointer)
    /// when the member is flagged so (flags bit 1), or when the type the
    /// member's slot 1 returns (`*0x00b309dc`, called directly) has kind
    /// 2 and the member lacks flags bit 6; else the type's size
    /// (`+0x1c`, read first).
    fn member_size(h: &mut impl Host, this: u32) -> u32 {
        let ty: u32 = h.read(this + 0x18);
        let size: u32 = h.read(ty + 0x1c);
        let slot1: u32 = h.read(h.rebase(0x00b3_09dc));
        let r = h.call(slot1, Conv::Thiscall(this), &[]);
        let flags: u32 = h.read(this + 0x14);
        let kind2 = r != 0 && h.read::<u32>(r + 0x30) == 2;
        if ((flags >> 6) & 1 == 0 && kind2) || (flags >> 1) & 1 != 0 {
            4
        } else {
            size
        }
    }

    /// The fields both data-member constructors set after the member.
    fn data_member_fields(h: &mut impl Host, this: u32, ty: u32, offset: u32) {
        h.write(this + 0x18, ty);
        let list = this + 0x1c;
        h.write(list, [list, list]);
        let vt = h.rebase(DATA_MEMBER_VTABLE);
        h.write(this, vt);
        h.write(this + 0x24, offset);
        h.write(this + 0x28, 0u32);
    }

    /// `PClassDataMember::PClassDataMember(this, desc, type, name,
    /// offset, flags, kind)` (`0x005752a0`, thiscall): the member
    /// (`0x0043a080` at `+4`), its fields, then the class descriptor's
    /// `addMember(this, size)` (slot 8): the size is 4 for kind 2, else
    /// [`member_size`]. Returns `this`.
    pub fn data_member_ctor(h: &mut impl Host, this: u32, args: [u32; 6]) -> u32 {
        let [desc, ty, name, offset, flags, kind] = args;
        h.call_sym(MEMBER_CTOR, Conv::Thiscall(this + 4), &[desc, name, flags]);
        data_member_fields(h, this, ty, offset);
        let size = if kind == 2 { 4 } else { member_size(h, this) };
        let desc: u32 = h.read(this + 0xc);
        h.call_virtual(desc, 8, &[this, size]);
        this
    }

    /// `PClassDataMemberEnum::PClassDataMemberEnum(this, desc, type,
    /// name, offset, flags, values)` (`0x00575370`, thiscall): as
    /// [`data_member_ctor`] with flags bit 5 and no kind, then `values`
    /// at `+0x2c` and the enum vtable. Returns `this`.
    pub fn enum_member_ctor(h: &mut impl Host, this: u32, args: [u32; 6]) -> u32 {
        let [desc, ty, name, offset, flags, values] = args;
        h.call_sym(
            MEMBER_CTOR,
            Conv::Thiscall(this + 4),
            &[desc, name, flags | 0x20],
        );
        data_member_fields(h, this, ty, offset);
        let size = member_size(h, this);
        let desc: u32 = h.read(this + 0xc);
        h.call_virtual(desc, 8, &[this, size]);
        h.write(this + 0x2c, values);
        let vt = h.rebase(ENUM_MEMBER_VTABLE);
        h.write(this, vt);
        this
    }

    /// `PClassDataMemberDynamic::PClassDataMemberDynamic(this, desc,
    /// type, name, offset, flags, kind)` (`0x004461e0`, thiscall):
    /// [`data_member_ctor`] (`0x005752a0`, flags bit 0 cleared), its
    /// vtable, a copy of `name` at `+0x2c` (none when `name` is the
    /// default-instance marker `*0x00c0b238`), flags bit 0 back; unless
    /// the marker, the copy (less bit 0) is the member's name. Returns
    /// `this`.
    pub fn dynamic_member_ctor(h: &mut impl Host, this: u32, args: [u32; 6]) -> u32 {
        let [desc, ty, name, offset, flags, kind] = args;
        h.call_sym(
            DATA_MEMBER_CTOR,
            Conv::Thiscall(this),
            &[desc, ty, name, offset, flags & !1, kind],
        );
        let vt = h.rebase(DYNAMIC_MEMBER_VTABLE);
        h.write(this, vt);
        let copy = if name == DEFAULT_INSTANCE.get(h) || name == 0 {
            0
        } else {
            copy_string(h, name, 2)
        };
        h.write(this + 0x2c, copy);
        let old: u32 = h.read(this + 0x14);
        h.write(this + 0x14, old | (flags & 1));
        if name != DEFAULT_INSTANCE.get(h) {
            h.write(this + 0x10, copy.wrapping_sub(copy & 1));
        }
        this
    }

    /// `0x004465c0` (cdecl `(p)`, `PClassDataMemberDynamic`'s
    /// construction in place): `al` 0 unless its descriptor
    /// (`0x00c91330`) has a default instance (`0x0043cbb0`) other than
    /// 0x18; else, for a `p`, [`dynamic_member_ctor`] (`0x004461e0`) with
    /// that instance less 0x18, `PBase`'s descriptor, the marker name and
    /// zeroes; `al` 1. The full `eax`.
    pub fn construct_dynamic_member(h: &mut impl Host, p: u32) -> u32 {
        let desc = h.rebase(0x00c9_1330);
        let def = h.call_sym(0x0043_cbb0, Conv::Thiscall(desc), &[]);
        if def == 0 || def == 0x18 {
            return core::with_al(def, false);
        }
        if p == 0 {
            return core::with_al(def, true);
        }
        let base = h.call_sym(BASE_DESCRIPTOR, Conv::Cdecl, &[]);
        let marker = DEFAULT_INSTANCE.get(h);
        let r = h.call_sym(
            DYNAMIC_MEMBER_CTOR,
            Conv::Thiscall(p),
            &[def - 0x18, base, marker, 0, 0, 0],
        );
        core::with_al(r, true)
    }

    /// `strcmp(a, b) == 0`, as the originals' inline loops compare.
    fn same_c_string(m: &impl Mem, a: u32, b: u32) -> bool {
        let mut i = 0u32;
        loop {
            let x: u8 = m.read(a.wrapping_add(i));
            if x != m.read::<u8>(b.wrapping_add(i)) {
                return false;
            }
            if x == 0 {
                return true;
            }
            i += 1;
        }
    }

    /// `PNamespace::getClassDescriptor(ns, name)` (`0x0043e950`,
    /// thiscall): 0 for no name; the descriptor of `ns`'s list (`+8`, a
    /// descriptor's link at `+0x2c`) whose name (`+0x18`) is `name`; else
    /// the first match of a child namespace (list `+0x14`, linked through
    /// `+0`), looked up through the original's address; else 0.
    pub fn namespace_find(h: &mut impl Host, ns: u32, name: u32) -> u32 {
        if name == 0 {
            return 0;
        }
        let head = ns + 8;
        let mut link: u32 = h.read(head);
        while link != head && link != 0 && link != 0x2c {
            let desc = link - 0x2c;
            let n: u32 = h.read(desc + 0x18);
            if n != 0 && same_c_string(h, n, name) {
                return desc;
            }
            link = h.read(desc + 0x2c);
        }
        let head = ns + 0x14;
        let first: u32 = h.read(head);
        let mut child = if first == head { 0 } else { first };
        while child != 0 {
            let r = h.call_sym(0x0043_e950, Conv::Thiscall(child), &[name]);
            if r != 0 {
                return r;
            }
            let next: u32 = h.read(child);
            child = if next == head { 0 } else { next };
        }
        0
    }

    /// `PNamespace::add(ns, desc)` (`0x0043e660`, thiscall): when a
    /// descriptor of that name is there already, a warning unless it is
    /// `desc`; else, for a namespace in state 1 or 2 (2 warns first),
    /// `updateBaseOffsets` (`0x0043d6e0`), then `desc`'s link goes after
    /// its parent's when the parent is in `ns`, else at the list's head;
    /// a first descriptor puts `ns` on the global namespace's child list
    /// (unless `ns` is the global one).
    pub fn namespace_add(h: &mut impl Host, ns: u32, desc: u32) {
        let name: u32 = h.read(desc + 0x18);
        let found = h.call_sym(0x0043_e950, Conv::Thiscall(ns), &[name]);
        if found != 0 {
            if found != desc {
                let msg = h.rebase(0x00b0_ef70);
                h.call_sym(PRINTF, Conv::Cdecl, &[2, msg, name]);
            }
            return;
        }
        let head = ns + 8;
        let first: u32 = h.read(head);
        let empty = first == head || first == 0 || first == 0x2c;
        let state: u32 = h.read(ns + 0x10);
        if state == 2 {
            let msg = h.rebase(0x00b0_efb8);
            h.call_sym(PRINTF, Conv::Cdecl, &[1, msg, name]);
        }
        if state == 1 || state == 2 {
            h.call_sym(0x0043_d6e0, Conv::Thiscall(desc), &[]);
        }
        let parent: u32 = h.read(desc + 0x40);
        let link = desc + 0x2c;
        let after = if parent != 0 && h.read::<u32>(parent + 0x3c) == ns {
            parent + 0x2c
        } else {
            head
        };
        let next: u32 = h.read(after);
        let tail: u32 = h.read(desc + 0x30);
        h.write(next + 4, tail);
        h.write(tail, next);
        h.write(link + 4, after);
        h.write(after, link);
        if empty {
            let global = h.call_sym(GLOBAL_NAMESPACE, Conv::Cdecl, &[]);
            if ns != global {
                let global = h.call_sym(GLOBAL_NAMESPACE, Conv::Cdecl, &[]);
                let children = global + 0x14;
                let old: u32 = h.read(children);
                h.write(ns, [old, children]);
                h.write(children, ns);
                h.write(old + 4, ns);
            }
        }
    }

    /// `0x0045b9d0` (thiscall `(import)` on the asset tree): links the
    /// import's node (`+4`) under the tree ordered by (hash `+0x10`, name
    /// `+0x14` through `_stricmp`): 0 once inserted (its parent word gets
    /// the parent, keeping the colour bit), or the node already there
    /// with the same key (0 for one at address 0).
    pub fn import_insert(h: &mut impl Host, tree: u32, import: u32) -> u32 {
        let cmp = h.read(h.rebase(STRICMP_IMPORT));
        insert_named(h, tree, import, |h, key, node| {
            h.call(cmp, Conv::Cdecl, &[key, node]) as i32
        })
    }

    /// `strcmp`'s sign as the originals' inline loops give it: -1 when
    /// the first differing byte of `a` is below `b`'s, 1 above, 0 for the
    /// same string.
    pub(crate) fn c_compare(m: &impl Mem, a: u32, b: u32) -> i32 {
        let mut i = 0u32;
        loop {
            let x: u8 = m.read(a.wrapping_add(i));
            let y: u8 = m.read(b.wrapping_add(i));
            if x != y {
                return if x < y { -1 } else { 1 };
            }
            if x == 0 {
                return 0;
            }
            i += 1;
        }
    }

    /// The (hash, name) red-black tree insert `0x0045b9d0` and
    /// `0x00438780` share (thiscall `(item)` on the tree; the item's node
    /// at `+4`, its colour and parent word `+0xc`, its key at
    /// `+0x10`/`+0x14`): the hashes ordered unsigned, equal hashes by
    /// `cmp(key name, node name)`; 0 once linked, else the node already
    /// there.
    pub(crate) fn insert_named<H: Host>(
        h: &mut H,
        tree: u32,
        item: u32,
        mut cmp: impl FnMut(&mut H, u32, u32) -> i32,
    ) -> u32 {
        let node = item + 4;
        let color: u32 = h.read(item + 0xc);
        h.write(item + 0xc, color.wrapping_add(1 - (color & 1)));
        h.write(node, [tree, tree]);
        let mut parent = tree + 0xc;
        let mut left = false;
        let link: u32 = h.read(tree + 0xc);
        let mut cur = if link == 0 { 0 } else { link - 4 };
        loop {
            let at = if cur == 0 { 0 } else { cur + 4 };
            if at == tree {
                let color: u32 = h.read(node + 8);
                h.write(node + 8, (color & 1).wrapping_add(parent));
                h.write(parent + 4 * u32::from(left), node);
                return 0;
            }
            parent = at;
            let key_at = if cur == 0 { 0 } else { cur + 0x10 };
            let nh: u32 = h.read(key_at);
            let hash: u32 = h.read(item + 0x10);
            let order: i32 = match nh.cmp(&hash) {
                std::cmp::Ordering::Greater => 1,
                std::cmp::Ordering::Less => -1,
                std::cmp::Ordering::Equal => {
                    let n: u32 = h.read(key_at + 4);
                    let name: u32 = h.read(item + 0x14);
                    cmp(h, name, n)
                }
            };
            if order == 0 {
                return if cur == 0 { 0 } else { cur + 4 };
            }
            left = order < 0;
            let next: u32 = h.read(cur + 4 + 4 * u32::from(left));
            cur = if next == 0 { 0 } else { next - 4 };
        }
    }

    /// `0x00438780` (thiscall `(type)` on the type dispenser's tree):
    /// [`insert_named`] with the names compared by `strcmp`.
    pub fn dispenser_insert(h: &mut impl Host, tree: u32, ty: u32) -> u32 {
        insert_named(h, tree, ty, |h, key, node| c_compare(h, key, node))
    }

    /// The buffers' computation `0x0043c160` repeats: nothing once
    /// computed (flags bit 31); else slot 12 unless in a namespace (bit
    /// 30), `calculateDefaultBuffer`, `calculateValidationBuffer`. The
    /// `eax` it leaves: the flags, or the last call's.
    fn compute_buffers(h: &mut impl Host, desc: u32) -> u32 {
        let flags: u32 = h.read(desc + 0x90);
        if flags >> 31 != 0 {
            return flags;
        }
        if flags & 0x4000_0000 == 0 {
            h.call_virtual(desc, 12, &[]);
        }
        h.call_sym(0x0043_c420, Conv::Thiscall(desc), &[]);
        h.call_sym(0x0043_c630, Conv::Thiscall(desc), &[])
    }

    /// `PClassDescriptor::applyDefaultBufferWithValidation(desc, p, n)`
    /// (`0x0043c160`, thiscall): `al` 1 at once for flags bit 28; else
    /// (twice, as the original) computes the buffers when not done
    /// (slot 12 unless in a namespace, then `calculateDefaultBuffer` and
    /// `calculateValidationBuffer`); `al` 0 without `p`, a default or a
    /// validation buffer; else merges the default into each of `n`
    /// instances at `p` (less `+0x8c`, `+0x1c` bytes each) where the
    /// validation buffer has bits; `al` 1. The full `eax`.
    pub fn apply_default_buffer(h: &mut impl Host, desc: u32, p: u32, n: u32) -> u32 {
        let flags: u32 = h.read(desc + 0x90);
        if flags & 0x1000_0000 != 0 {
            return core::with_al(flags, true);
        }
        compute_buffers(h, desc);
        let default: u32 = h.read(desc + 0x64);
        let eax = compute_buffers(h, desc);
        let mask: u32 = h.read(desc + 0x68);
        if p == 0 || default == 0 || mask == 0 {
            return core::with_al(eax, false);
        }
        let size: u32 = h.read(desc + 0x1c);
        let offset: u32 = h.read(desc + 0x8c);
        let mut inst = p.wrapping_sub(offset);
        for _ in 0..n {
            for i in 0..size {
                let d: u8 = h.read(default.wrapping_add(i));
                let k: u8 = h.read(mask.wrapping_add(i));
                let at = inst.wrapping_add(i);
                let v: u8 = h.read(at);
                h.write(at, core::merge_default(d, k, v));
            }
            inst = inst.wrapping_add(size);
        }
        1
    }

    /// `0x005cb870` (cdecl `(&w, n, &arrays, &out)`): `n` floats
    /// `a[i] * w[0] + b[i] * w[1]` (x87 at 53 bits, one rounding to
    /// `f32` per store) from the two arrays `arrays` points at into the
    /// array `out` points at. A count of 0 runs 2^32 times, as the
    /// original's loop does.
    pub fn blend2(m: &mut impl Mem, w: u32, n: u32, arrays: u32, out: u32) {
        use ffx_bridge::x87;
        let w0 = x87::load(m.read(w));
        let w1 = x87::load(m.read(w + 4));
        let [a, b]: [u32; 2] = m.read(arrays);
        let dst: u32 = m.read(out);
        let mut i = 0u32;
        let mut left = n;
        loop {
            let x = x87::mul(x87::load(m.read(b.wrapping_add(i))), w1);
            let y = x87::mul(x87::load(m.read(a.wrapping_add(i))), w0);
            m.write(dst.wrapping_add(i), x87::store(x87::add(x, y)));
            i = i.wrapping_add(4);
            left = left.wrapping_sub(1);
            if left == 0 {
                break;
            }
        }
    }

    /// `0x006019b0` (thiscall): for every group of the list at `+0x48`
    /// (linked through `+0`), every entry of its list (`+4`, linked
    /// through `+0`) whose object (`+0xb0`) belongs to this owner's
    /// renderer (`+0x54`, compared with the object's `+8`'s `+0x54`) and
    /// has a resource (`+0xdc`) hands it to the renderer's slot 20; then
    /// the group's `+8` = 0. Returns 0.
    pub fn release_group_resources(h: &mut impl Host, this: u32) -> u32 {
        let mut group: u32 = h.read(this + 0x48);
        while group != 0 {
            let mut e: u32 = h.read(group + 4);
            while e != 0 {
                let obj: u32 = h.read(e + 0xb0);
                let next: u32 = h.read(e);
                if obj != 0 {
                    let o8: u32 = h.read(obj + 8);
                    let owner = if o8 == 0 { 0 } else { h.read(o8 + 0x54) };
                    let renderer: u32 = h.read(this + 0x54);
                    if renderer != 0 && renderer == owner {
                        let res: u32 = h.read(e + 0xdc);
                        if res != 0 {
                            h.call_virtual(renderer, 20, &[res]);
                        }
                    }
                }
                e = next;
            }
            h.write(group + 8, 0u32);
            group = h.read(group);
        }
        0
    }

    /// `0x0058c010` (thiscall, ten words, on the cluster registry's
    /// owner): under the lock at `+0x1374` (`0x00431840`/`0x004319d0`),
    /// `0x00595e50(&key, &record)` on the registry at `+0x1344`, where
    /// the key is the second argument and the record all ten. `frame`
    /// holds the key word, then the record (the original's arguments and
    /// locals). Returns 0.
    pub fn registry_add_locked(h: &mut impl Host, frame: u32, this: u32, args: [u32; 10]) -> u32 {
        let lock = this + 0x1374;
        h.call_sym(0x0043_1840, Conv::Thiscall(lock), &[]);
        h.write(frame, args[1]);
        h.write(frame + 4, args);
        h.call_sym(
            0x0059_5e50,
            Conv::Thiscall(this + 0x1344),
            &[frame, frame + 4],
        );
        h.call_sym(0x0043_19d0, Conv::Thiscall(lock), &[]);
        0
    }

    /// `0x00449e40` (thiscall on `PClassDescriptorDynamic`): when the
    /// size (`+0x1c`) grew past the buffers' count (`+0x9c`, low 31
    /// bits), the three byte arrays at `+0x9c`, `+0xa4`, `+0xac` resized
    /// (`0x0044a080`) and their new tails zeroed; the buffers marked not
    /// computed and pointed at the arrays, `getDefaultBuffer`
    /// (`0x0043cb80`), and when there is a default buffer and the class
    /// derives from `0x00c91c18`, the descriptor stored in its instance's
    /// `+8`.
    pub fn dynamic_refresh(h: &mut impl Host, desc: u32) {
        let had: u32 = h.read(desc + 0x9c);
        let had = had & 0x7fff_ffff;
        let size: u32 = h.read(desc + 0x1c);
        if size != 0 && size > had {
            let grow = size - had;
            h.call_sym(0x0044_a080, Conv::Thiscall(desc + 0x9c), &[size]);
            for at in [0xa4, 0xac] {
                let size: u32 = h.read(desc + 0x1c);
                h.call_sym(0x0044_a080, Conv::Thiscall(desc + at), &[size]);
            }
            for at in [0xa0, 0xa8, 0xb0] {
                let data: u32 = h.read(desc + at);
                h.call_sym(MEMSET, Conv::Cdecl, &[data.wrapping_add(had), 0, grow]);
            }
        }
        let flags: u32 = h.read(desc + 0x90);
        h.write(desc + 0x90, flags & 0x7fff_ffff);
        for (to, from) in [(0x64, 0xa0), (0x68, 0xa8), (0x6c, 0xb0)] {
            let v: u32 = h.read(desc + from);
            h.write(desc + to, v);
        }
        h.call_sym(0x0043_cb80, Conv::Thiscall(desc), &[]);
        let default: u32 = h.read(desc + 0x64);
        if default == 0 {
            return;
        }
        let target = h.rebase(0x00c9_1c18);
        let mut d = desc;
        while d != target {
            d = h.read(d + 0x40);
            if d == 0 {
                return;
            }
        }
        let offset: u32 = h.read(desc + 0x8c);
        h.write(offset.wrapping_add(default).wrapping_add(8), desc);
    }
}
