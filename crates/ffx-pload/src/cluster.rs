//! `PCluster`, `PNamespace` and the `PBinary` cluster headers.
//!
//! Layouts:
//! - `PNamespace` (`0x0043dd80`): three intrusive list heads (`+0`, `+8`,
//!   `+0x14`, each `next, prev` pointing at itself when empty) and a count
//!   at `+0x10`;
//! - `PCluster` (`0x0043f800`): a `PClusterBase`, then a list head at
//!   `+0x48`;
//! - `PClusterHeaderBase` (`0x00468670`): `"RYHP"`, 0x54, zeroes;
//!   `PClusterHeader` (`0x00469980`) adds three zero words at
//!   `+0x48..+0x54`.

use ffx_bridge::Mem;
use ffx_graphic::host::{Conv, Host};

/// `PClusterBase::PClusterBase` (thiscall).
pub const CLUSTER_BASE_CTOR: u32 = 0x0043_f820;
/// `PClusterHeaderBase::PClusterHeaderBase` (thiscall).
pub const HEADER_BASE_CTOR: u32 = 0x0046_8670;

/// Pure parts: the empty intrusive list.
pub mod core {
    /// The two words of an empty list head at `at` (`next`, `prev`).
    #[must_use]
    pub const fn empty_list(at: u32) -> [u32; 2] {
        [at, at]
    }
}

/// The ports over game memory.
pub mod bridge {
    use super::{CLUSTER_BASE_CTOR, Conv, HEADER_BASE_CTOR, Host, Mem, core};
    use crate::descriptor::FREE;

    fn init_list(m: &mut impl Mem, at: u32) {
        m.write(at, core::empty_list(at));
    }

    /// `PNamespace::PNamespace(ns)` (`0x0043dd80`, thiscall): three empty
    /// lists, count 0; returns `ns`.
    pub fn namespace_ctor(m: &mut impl Mem, ns: u32) -> u32 {
        init_list(m, ns);
        init_list(m, ns + 8);
        m.write(ns + 0x10, 0u32);
        init_list(m, ns + 0x14);
        ns
    }

    /// `0x00576630` (thiscall): `PClusterBase::PClusterBase(this)`;
    /// returns `this`.
    pub fn cluster_base_init(h: &mut impl Host, this: u32) -> u32 {
        h.call_sym(CLUSTER_BASE_CTOR, Conv::Thiscall(this), &[]);
        this
    }

    /// `PCluster::PCluster(cluster)` (`0x0043f800`, thiscall): the base,
    /// then an empty list at `+0x48`; returns `cluster`.
    pub fn cluster_ctor(h: &mut impl Host, cluster: u32) -> u32 {
        cluster_base_init(h, cluster);
        init_list(h, cluster + 0x48);
        cluster
    }

    /// `PClusterHeader::PClusterHeader(header)` (`0x00469980`,
    /// thiscall): the base header, then three zero words; returns it.
    pub fn header_ctor(h: &mut impl Host, header: u32) -> u32 {
        h.call_sym(HEADER_BASE_CTOR, Conv::Thiscall(header), &[]);
        h.write(header + 0x48, [0u32; 3]);
        header
    }

    /// `0x004787e0` (thiscall `(a, b, c)`): `0x004786a0(this, a, b, 1,
    /// c)`; returns `this`.
    pub fn mapping_init(h: &mut impl Host, this: u32, a: u32, b: u32, c: u32) -> u32 {
        h.call_sym(0x0047_86a0, Conv::Thiscall(this), &[a, b, 1, c]);
        this
    }

    /// `0x00465aa0` (thiscall `(a, b, c)`): a tail call of
    /// `PBinary::loadClusterTextures` (`0x00465ab0`) with the same
    /// arguments; its `eax`.
    pub fn load_cluster_textures(h: &mut impl Host, this: u32, a: u32, b: u32, c: u32) -> u32 {
        h.call_sym(0x0046_5ab0, Conv::Thiscall(this), &[a, b, c])
    }

    /// `0x00593bd0` (thiscall): the low 31 bits of `+0x20`, or 1 for 0.
    pub fn count_at_least_one(m: &impl Mem, this: u32) -> u32 {
        let v: u32 = m.read(this + 0x20);
        let v = v & 0x7fff_ffff;
        if v == 0 { 1 } else { v }
    }

    /// `0x00595fa0` / `0x00595fb0` (thiscall): whether `+0x64` is set.
    pub fn has_word_64(m: &impl Mem, this: u32) -> u32 {
        let v: u32 = m.read(this + 0x64);
        u32::from(v != 0)
    }

    /// `PCluster::allocateInstanceList(cluster, n, desc, c)`
    /// (`0x00440b70`, thiscall): 0 for none, else
    /// `allocateInstanceListForSize(n * desc->size, n, desc, c)`
    /// (`0x00440ba0`); its `eax`.
    pub fn allocate_instance_list(
        h: &mut impl Host,
        cluster: u32,
        n: u32,
        desc: u32,
        c: u32,
    ) -> u32 {
        if n == 0 {
            return 0;
        }
        let size: u32 = h.read(desc + 0x1c);
        h.call_sym(
            0x0044_0ba0,
            Conv::Thiscall(cluster),
            &[size.wrapping_mul(n), n, desc, c],
        )
    }

    /// `0x00440cf0` (thiscall `(size, align)` on a cluster): carves
    /// `size` bytes, aligned to `align` (a power of two), from the
    /// cluster's arena (`+0x24`: base `+4`, end `+8`, used `+0xc`); the
    /// address, or 0 without an arena or room.
    pub fn arena_alloc(m: &mut impl Mem, cluster: u32, size: u32, align: u32) -> u32 {
        let arena: u32 = m.read(cluster + 0x24);
        if arena == 0 {
            return 0;
        }
        let used: u32 = m.read(arena + 0xc);
        let mask = align.wrapping_sub(1);
        let offset = used.wrapping_sub(1).wrapping_add(align) & !mask;
        let end = size.wrapping_add(offset);
        let limit: u32 = m.read(arena + 8);
        if end > limit {
            return 0;
        }
        let base: u32 = m.read(arena + 4);
        m.write(arena + 0xc, end);
        base.wrapping_add(offset)
    }

    /// An intrusive list's entry (`link` less 4) unless `link` is the
    /// head or 0.
    fn entry_of(link: u32, head: u32) -> u32 {
        if link == head || link == 0 {
            0
        } else {
            link.wrapping_sub(4)
        }
    }

    /// `0x00575d40` (thiscall `(key)` on a list head): the entry whose
    /// `+0xc` is `key` (entries linked through `+4`), or 0.
    pub fn find_by_key(m: &impl Mem, head: u32, key: u32) -> u32 {
        let first: u32 = m.read(head);
        let mut e = entry_of(first, head);
        while e != 0 {
            let k: u32 = m.read(e + 0xc);
            if k == key {
                return e;
            }
            let link: u32 = m.read(e + 4);
            e = entry_of(link, head);
        }
        0
    }

    /// `0x005758e0` (thiscall on a list head): unlinks every entry
    /// (`PArray_blockHeader(head, head)` gives the next), leaving each
    /// one's link empty; 0.
    pub fn clear_list(m: &mut impl Mem, head: u32) -> u32 {
        loop {
            let n = crate::descriptor::bridge::block_header(m, head, head);
            if n == 0 {
                return 0;
            }
            let link = n + 4;
            let prev: u32 = m.read(link);
            let next: u32 = m.read(link + 4);
            m.write(prev + 4, next);
            m.write(next, prev);
            m.write(link, [link, link]);
        }
    }

    /// `0x0047f9d0` (thiscall on a cluster header): whether its magic is
    /// `0x01020304` and its size word (`+4`) matches the size its counts
    /// give. `al` (all of `eax` when the magic matches).
    pub fn header_is_valid(m: &impl Mem, header: u32) -> u8 {
        let w: [u32; 8] = m.read(header);
        if w[0] != 0x0102_0304 {
            return 0;
        }
        let units = w[2]
            .wrapping_add(w[3].wrapping_mul(9))
            .wrapping_add(w[4].wrapping_mul(6));
        let size = w[7]
            .wrapping_add(0x20)
            .wrapping_add(units.wrapping_mul(4))
            .wrapping_add(w[5]);
        u8::from(w[1] == size)
    }

    /// `0x004647a0` (thiscall `(a, header, c, d)`): copies the header's
    /// `+0x48` and `+0x4c` (and `+0x50` when set) to `+0x18`, `+0x14` and
    /// `+0x1c` unless all three are 0; 0.
    pub fn copy_header_words(m: &mut impl Mem, this: u32, header: u32) -> u32 {
        let [a, b, c]: [u32; 3] = m.read(header + 0x48);
        if a | b | c == 0 {
            return 0;
        }
        m.write(this + 0x18, a);
        m.write(this + 0x14, b);
        if c != 0 {
            m.write(this + 0x1c, c);
        }
        0
    }

    /// The red-black tree lookups `0x00576ef0` and `0x00581620`
    /// (thiscall `(&key)` on the tree, whose header is the nil node): the
    /// node whose key (`+0xc`, unsigned) is `*key`, or 0.
    pub fn tree_find(m: &impl Mem, tree: u32, key: u32) -> u32 {
        let k: u32 = m.read(key);
        let mut node: u32 = m.read(tree + 0xc);
        while node != tree {
            let nk: u32 = m.read(node + 0xc);
            if nk == k {
                return node;
            }
            // left (+0) for a greater node key, right (+4) for a smaller
            let side = u32::from(nk < k);
            node = m.read(node + 4 * side);
        }
        0
    }

    /// `PArray<T>::setData(array, data, count)` (`0x0044a320`'s
    /// neighbours `0x0044a2d0`, `0x00a2db70`; thiscall): nothing for a
    /// negative count; else frees the old data when the array owns it
    /// (bit 31 of the count clear, not 0, not `data`), then stores `data`
    /// and the count with bit 31 (not owned); 0.
    pub fn array_set_data(h: &mut impl Host, array: u32, data: u32, count: i32) -> u32 {
        if count < 0 {
            return 0;
        }
        let old: u32 = h.read(array + 4);
        let word: u32 = h.read(array);
        if old != data && word >> 31 == 0 && old != 0 {
            h.call_sym(FREE, Conv::Cdecl, &[old]);
        }
        h.write(array + 4, data);
        h.write(array, count as u32 | 0x8000_0000);
        0
    }

    /// A `PArray<T>` descriptor's destructInstance slot (`0x00449540`,
    /// `0x00459670`, `0x005cbaa0`, `0x00a2d5d0`; stdcall `(array)`): frees
    /// the data it owns, then zeroes it; `al` = 1.
    pub fn array_destroy(h: &mut impl Host, array: u32) -> u8 {
        let word: u32 = h.read(array);
        let data: u32 = h.read(array + 4);
        if word >> 31 == 0 && data != 0 {
            h.call_sym(FREE, Conv::Cdecl, &[data]);
        }
        h.write(array + 4, 0u32);
        h.write(array, 0u32);
        1
    }

    /// `PClassDescriptorConcrete<PBitmapFontCharInfo>::constructInstance(p)`
    /// (`0x00a2d520`, stdcall): zeroes its words, floats (`fldz`) and the
    /// byte at `+0x2c` when `p` is not 0; `p` with `al` = 1.
    pub fn char_info_ctor(m: &mut impl Mem, p: u32) -> u32 {
        if p != 0 {
            m.write(p, [0u32; 11]);
            m.write(p + 0x2c, 0u8);
        }
        (p & !0xff) | 1
    }

    /// `PDepthOfField::PDepthOfField(this)` (`0x009fbd40`, thiscall): the
    /// base (`0x009fbd90`), four zero words at `+0xfb0`, its vtable and
    /// `+4` = `0x019438e0`; returns `this`.
    pub fn depth_of_field_ctor(h: &mut impl Host, this: u32) -> u32 {
        h.call_sym(0x009f_bd90, Conv::Thiscall(this), &[]);
        h.write(this + 0xfb0, [0u32; 4]);
        let vt = h.rebase(0x00b7_4698);
        h.write(this, vt);
        let p = h.rebase(0x0194_38e0);
        h.write(this + 4, p);
        this
    }

    /// `0x0058dba0` (thiscall): when the first of its data words (one
    /// inline at `+0x24` for a count of 1, else the array `+0x24` points
    /// at; nothing for 0) is 0, `0x00590800(this; 0, 1, 0, 0)`; then
    /// `+0x64` = 1; 0. A count of 0 reads address 0 (as the original).
    pub fn ensure_created(h: &mut impl Host, this: u32) -> u32 {
        let n: u32 = h.read(this + 0x20);
        let n = n & 0x7fff_ffff;
        let words = if n > 1 {
            h.read(this + 0x24)
        } else if n != 0 {
            this + 0x24
        } else {
            0
        };
        let first: u32 = h.read(words);
        if first == 0 {
            h.call_sym(0x0059_0800, Conv::Thiscall(this), &[0, 1, 0, 0]);
        }
        h.write(this + 0x64, 1u32);
        0
    }
}
