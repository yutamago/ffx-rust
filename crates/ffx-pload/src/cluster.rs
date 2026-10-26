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
        tree_find_key(m, tree, m.read(key))
    }

    /// [`tree_find`] for the key value `k`.
    pub fn tree_find_key(m: &impl Mem, tree: u32, k: u32) -> u32 {
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

    // ---- the class-member iterators (`PClusterHeader`'s instance walk) ----
    //
    // An iterator: five result words (`+0..+0x10`, filled by `0x00441dc0`),
    // the list's end (`+0x14`), the current element (`+0x18`, linked
    // through its first word) and the class it looks for (`+0x1c`).

    /// One step: the next element, or 0 at the end.
    fn iter_step(m: &mut impl Mem, it: u32) {
        let cur: u32 = m.read(it + 0x18);
        if cur != 0 {
            let next: u32 = m.read(cur);
            let end: u32 = m.read(it + 0x14);
            m.write(it + 0x18, if next == end { 0 } else { next });
        }
    }

    /// The end of a walk: the element's data through `0x00441dc0`
    /// (thiscall, its `eax`), or the results cleared and 0.
    fn iter_finish(h: &mut impl Host, it: u32) -> u32 {
        let cur: u32 = h.read(it + 0x18);
        if cur != 0 {
            return h.call_sym(0x0044_1dc0, Conv::Thiscall(it), &[cur]);
        }
        h.write(it, [0u32; 5]);
        0
    }

    /// The iterators' seek (twelve copies, `0x00464b40`..`0x00465010`,
    /// thiscall): steps until the current element lists the class
    /// (`PClassDataMember_listsClass`), then [`iter_finish`].
    pub fn iter_seek(h: &mut impl Host, it: u32) -> u32 {
        while crate::descriptor::bridge::member_lists_class(h, it) == 0 {
            iter_step(h, it);
        }
        iter_finish(h, it)
    }

    /// The iterators' next (five copies, `0x00465080`..`0x004653e0`,
    /// thiscall): one step at least, then as [`iter_seek`].
    pub fn iter_next(h: &mut impl Host, it: u32) -> u32 {
        loop {
            iter_step(h, it);
            if crate::descriptor::bridge::member_lists_class(h, it) != 0 {
                break;
            }
        }
        iter_finish(h, it)
    }

    /// `0x004638f0` (thiscall `(owner)`): an iterator over `owner`'s list
    /// at `+0x1c` for the class at `0x00cb1120`, sought to its first
    /// element ([`iter_seek`]); returns the iterator.
    pub fn iter_begin(h: &mut impl Host, it: u32, owner: u32) -> u32 {
        h.write(it, [0u32; 5]);
        let end = owner + 0x1c;
        h.write(it + 0x14, end);
        let first: u32 = h.read(end);
        h.write(it + 0x18, if first == end { 0 } else { first });
        let class = h.rebase(0x00cb_1120);
        h.write(it + 0x1c, class);
        iter_seek(h, it);
        it
    }

    /// The `PClusterHeader` descriptor's constructInstance (`0x00469460`,
    /// stdcall `(p)`): [`header_ctor`] when `p` is not 0; `al` = 1.
    pub fn construct_header(h: &mut impl Host, p: u32) -> u8 {
        if p != 0 {
            header_ctor(h, p);
        }
        1
    }

    /// `PApplication::FixupClusters(clusters, n)` (`0x00623580`, cdecl):
    /// `resolveAssetReferences` (`0x00441f20`) on every cluster, then
    /// `fixupInstances` (`0x00441620`), both thiscall; the first error, or
    /// 0. Null entries are skipped.
    pub fn fixup_clusters(h: &mut impl Host, clusters: u32, n: u32) -> u32 {
        let end = clusters.wrapping_add(n.wrapping_mul(4));
        for f in [0x0044_1f20, 0x0044_1620] {
            let mut p = clusters;
            while p < end {
                let c: u32 = h.read(p);
                if c != 0 {
                    let r = h.call_sym(f, Conv::Thiscall(c), &[]);
                    if r != 0 {
                        return r;
                    }
                }
                p += 4;
            }
        }
        0
    }

    /// `0x0045cd00` (cdecl `(&array, id)`): the element of a `PArray` of
    /// pointers (count `+0` with bit 31 masked, data inline at `+4` for
    /// one, else `+4` points at it) whose `+0x2c` is `id`, or 0.
    pub fn find_by_id(m: &impl Mem, array: u32, id: u32) -> u32 {
        let a: u32 = m.read(array);
        if a == 0 {
            return 0;
        }
        let n: u32 = m.read(a);
        let n = n & 0x7fff_ffff;
        for i in 0..n {
            let data = if n > 1 { m.read(a + 4) } else { a + 4 };
            let e: u32 = m.read(data.wrapping_add(4 * i));
            if e != 0 {
                let v: u32 = m.read(e + 0x2c);
                if v == id {
                    return e;
                }
            }
        }
        0
    }

    /// `0x0047e730` (thiscall `(i, &a, &b)` on a mapping): element `i` of
    /// the first table (`+8`, 12-byte entries, count `+4` with bit 31
    /// masked) into `*a`, or of the second one (`+0x10`, 32-byte entries,
    /// from 1 past the first's count) into `*b`; whether `*a` or `*b` is
    /// set (`al`).
    pub fn mapping_entry(m: &mut impl Mem, map: u32, i: u32, a: u32, b: u32) -> u8 {
        let n: u32 = m.read(map + 4);
        let n = n & 0x7fff_ffff;
        if i < n {
            let t: u32 = m.read(map + 8);
            m.write(a, t.wrapping_add(i.wrapping_mul(12)));
        } else {
            let t: u32 = m.read(map + 0x10);
            m.write(b, (i.wrapping_sub(n).wrapping_sub(1) << 5).wrapping_add(t));
        }
        let vb: u32 = m.read(b);
        let va: u32 = m.read(a);
        u8::from(vb != 0 || va != 0)
    }

    /// Zeroes `n` 0x20-byte entries at `at` (`0x0047baa0`, `0x0047bb00`,
    /// cdecl; an entry at address 0 is skipped).
    pub fn zero_entries(m: &mut impl Mem, at: u32, n: u32) {
        let mut e = at;
        for _ in 0..n {
            if e != 0 {
                m.write(e, [0u32; 8]);
            }
            e = e.wrapping_add(0x20);
        }
    }

    /// `0x00577760` (thiscall `(a, b, c, flag)`): stores the three words
    /// and the flag's low byte, zeroes `+0x10..+0x24`; returns `this`.
    pub fn record_init(m: &mut impl Mem, this: u32, a: u32, b: u32, c: u32, flag: u32) -> u32 {
        m.write(this, [a, b, c]);
        m.write(this + 0xc, flag as u8);
        m.write(this + 0x10, [0u32; 6]);
        this
    }

    /// `PCluster`'s teardown (`0x0043faa0`, thiscall): `PClusterBase::clear`
    /// (`0x00440eb0`), `0x0049c5b0` on `+0x28`, then its namespace's owned
    /// descriptors (`0x0043dde0`).
    pub fn cluster_teardown(h: &mut impl Host, cluster: u32) {
        h.call_sym(0x0044_0eb0, Conv::Thiscall(cluster), &[]);
        h.call_sym(0x0049_c5b0, Conv::Thiscall(cluster + 0x28), &[]);
        crate::descriptor::bridge::release_owned(h, cluster);
    }

    /// `PClusterHeaderBase::PClusterHeaderBase(header)` (`0x00468670`,
    /// thiscall): `"RYHP"`, 0x54, sixteen zero words; returns it.
    pub fn header_base_ctor(m: &mut impl Mem, header: u32) -> u32 {
        m.write(header, [0x5048_5952u32, 0x54]);
        m.write(header + 8, [0u32; 16]);
        header
    }

    /// `PCluster::~PCluster(cluster)` (`0x0043fa30`, thiscall):
    /// `PClusterBase::clear`, the owner's release of it (`0x00443b70` on
    /// `+0x40`, then 0), then the base destructor (`0x00576640`).
    pub fn cluster_dtor(h: &mut impl Host, cluster: u32) {
        h.call_sym(0x0044_0eb0, Conv::Thiscall(cluster), &[]);
        let owner: u32 = h.read(cluster + 0x40);
        if owner != 0 {
            h.call_sym(0x0044_3b70, Conv::Thiscall(owner), &[cluster]);
            h.write(cluster + 0x40, 0u32);
        }
        h.call_sym(0x0057_6640, Conv::Thiscall(cluster), &[]);
    }

    /// `PClusterBase::PClusterBase(cluster)` (`0x0043f820`, thiscall): its
    /// namespace ([`namespace_ctor`]), an empty list at `+0x1c`, `+0x24`
    /// = 0, the pool at `+0x28` (`0x0049c480(0x4c, 0x14, 4, 0x00b0f11c)`),
    /// `+0x40` = 0, `+0x44` = 3; returns it.
    pub fn cluster_base_ctor(h: &mut impl Host, cluster: u32) -> u32 {
        namespace_ctor(h, cluster);
        init_list(h, cluster + 0x1c);
        h.write(cluster + 0x24, 0u32);
        let name = h.rebase(0x00b0_f11c);
        h.call_sym(
            0x0049_c480,
            Conv::Thiscall(cluster + 0x28),
            &[0x4c, 0x14, 4, name],
        );
        h.write(cluster + 0x40, [0u32, 3]);
        cluster
    }

    /// `0x004c7260` (thiscall `(a, b, shift)`): 3 when `1 << shift`
    /// exceeds `a` (shift not 0); else, unless all three match `+0x1c`,
    /// `+0`, `+0xc`, stores them and refreshes `+0x10` (`shift`, or
    /// `0x004c2430(a, 0, 0)` when bit 1 of `+0x14` is set); 0.
    pub fn set_layout(h: &mut impl Host, this: u32, a: u32, b: u32, shift: u32) -> u32 {
        if shift != 0 && (1u32 << (shift & 31)) > a {
            return 3;
        }
        let ca: u32 = h.read(this + 0x1c);
        let cb: u32 = h.read(this);
        let cs: u32 = h.read(this + 0xc);
        if a == ca && b == cb && shift == cs {
            return 0;
        }
        h.write(this, b);
        let flags: u32 = h.read(this + 0x14);
        h.write(this + 0x1c, a);
        h.write(this + 0xc, shift);
        let v = if (flags >> 1) & 1 != 0 {
            h.call_sym(0x004c_2430, Conv::Cdecl, &[a, 0, 0])
        } else {
            shift
        };
        let cur: u32 = h.read(this + 0x10);
        if v != cur {
            h.write(this + 0x10, v);
        }
        0
    }

    /// `0x005989b0` (thiscall): frees the four blocks at `+0`, `+0x10`,
    /// `+0x18`, `+0x20` and clears the record's words.
    pub fn release_blocks(h: &mut impl Host, this: u32) {
        let a: u32 = h.read(this);
        h.call_sym(FREE, Conv::Cdecl, &[a]);
        let b: u32 = h.read(this + 0x10);
        h.write(this, [0u32; 4]);
        h.write(this + 0x14, 0u32);
        h.write(this + 0x1c, 0u32);
        h.call_sym(FREE, Conv::Cdecl, &[b]);
        let c: u32 = h.read(this + 0x18);
        h.write(this + 0x10, 0u32);
        h.call_sym(FREE, Conv::Cdecl, &[c]);
        let d: u32 = h.read(this + 0x20);
        h.write(this + 0x18, 0u32);
        h.call_sym(FREE, Conv::Cdecl, &[d]);
        h.write(this + 0x20, [0u32; 2]);
    }

    /// Rounds `x` up to `align` (a power of two; 0 leaves it).
    fn align_up(x: u32, align: u32) -> u32 {
        if align == 0 {
            x
        } else {
            x.wrapping_add(align.wrapping_sub(1) & x.wrapping_neg())
        }
    }

    /// `0x00578300` (cdecl `(&cursor, list)`): places an instance list's
    /// over-allocation (the class's count `+0x74` times the list's count
    /// `+0x3c`, aligned to `+0x78`): at the cursor when it is set
    /// (advancing it), else just past the instances; the address goes to
    /// `+0x38`. Nothing without over-allocation.
    pub fn place_overallocation(m: &mut impl Mem, cursor: u32, list: u32) {
        let class: u32 = m.read(list + 0x2c);
        let count: u32 = m.read(list + 0x3c);
        let over: u32 = m.read(class + 0x74);
        let align: u32 = m.read(class + 0x78);
        if over == 0 {
            return;
        }
        let total = over.wrapping_mul(count);
        let at: u32 = m.read(cursor);
        if at != 0 {
            let at = align_up(at, align);
            m.write(cursor, at);
            m.write(list + 0x38, at);
            m.write(cursor, at.wrapping_add(total));
        } else {
            let size: u32 = m.read(class + 0x1c);
            let extra: u32 = m.read(list + 0x44);
            let base: u32 = m.read(list + 0x34);
            let end = size
                .wrapping_mul(count)
                .wrapping_add(extra)
                .wrapping_add(base)
                .wrapping_sub(total);
            m.write(list + 0x38, align_up(end, align));
        }
    }

    /// The red-black tree insert `0x00581750` (thiscall `(node)`): the
    /// node red with nil children, then placed under its parent by
    /// unsigned key (`+0xc`); the node already holding the key instead
    /// (nothing linked), else 0. The parent pointer shares `+8` with the
    /// colour bit.
    pub fn tree_insert(m: &mut impl Mem, tree: u32, node: u32) -> u32 {
        let pc: u32 = m.read(node + 8);
        m.write(node + 8, pc.wrapping_add(1 - (pc & 1)));
        m.write(node, [tree, tree]);
        let key: u32 = m.read(node + 0xc);
        let mut parent = tree + 0xc;
        let mut side = 0;
        let mut cur: u32 = m.read(parent);
        while cur != tree {
            let k: u32 = m.read(cur + 0xc);
            parent = cur;
            if k == key {
                return cur;
            }
            side = u32::from(k < key);
            cur = m.read(cur + 4 * side);
        }
        let pc: u32 = m.read(node + 8);
        m.write(node + 8, (pc & 1).wrapping_add(parent));
        m.write(parent + 4 * side, node);
        0
    }

    /// `PCluster::create(cluster, n, desc, c)` (`0x00441010`, thiscall):
    /// for an abstract class (flags bit 3) an error report
    /// (`0x004352f0(1, msg, name)`) and 0; else `n` instances in a new
    /// instance list (`0x00440ba0`), each initialised
    /// (`initializeInstance`, `0x0043ce00`); their data, or 0.
    pub fn create(h: &mut impl Host, cluster: u32, n: u32, desc: u32, c: u32) -> u32 {
        let flags: u32 = h.read(desc + 0x90);
        if flags & 8 != 0 {
            let name: u32 = h.read(desc + 0x18);
            let msg = h.rebase(0x00b0_f248);
            h.call_sym(0x0043_52f0, Conv::Cdecl, &[1, msg, name]);
            return 0;
        }
        if n == 0 {
            return 0;
        }
        let size: u32 = h.read(desc + 0x1c);
        let list = h.call_sym(
            0x0044_0ba0,
            Conv::Thiscall(cluster),
            &[size.wrapping_mul(n), n, desc, c],
        );
        if list == 0 {
            return 0;
        }
        let data: u32 = h.read(list + 0x30);
        let mut p = data;
        for _ in 0..n {
            h.call_sym(0x0043_ce00, Conv::Thiscall(desc), &[p]);
            p = p.wrapping_add(size);
        }
        data
    }

    /// `PClusterBase::~PClusterBase(cluster)` (`0x00576640`, thiscall):
    /// the cluster registry static (`0x00c94f00`, built the first time:
    /// `0x004fee70`, `atexit(0x00afb710)`; guard `0x00c9628c`) forgets
    /// it (`0x00598c50`), then [`cluster_teardown`].
    pub fn cluster_base_dtor(h: &mut impl Host, cluster: u32) {
        let guard = h.rebase(0x00c9_628c);
        let registry = h.rebase(0x00c9_4f00);
        let g: u32 = h.read(guard);
        if g & 1 == 0 {
            h.write(guard, g | 1);
            h.call_sym(0x004f_ee70, Conv::Thiscall(registry), &[]);
            let dtor = h.rebase(0x00af_b710);
            h.call_sym(crate::types::ATEXIT, Conv::Cdecl, &[dtor]);
        }
        h.call_sym(0x0059_8c50, Conv::Thiscall(registry), &[cluster]);
        cluster_teardown(h, cluster);
    }

    /// The cluster registry's forget (`0x00598c50`, thiscall `(cluster)`
    /// on the registry `0x00c94f00`): under its mutex (`+0x1374`), the
    /// cluster's entry in the tree at `+0x1344` (keyed by the cluster)
    /// releases its blocks ([`release_blocks`] on `+0x10`), then leaves the
    /// tree (`0x00575000`) and the node pool (`0x0049c980` on `+0x18`); 0.
    pub fn registry_forget(h: &mut impl Host, registry: u32, cluster: u32) -> u32 {
        let mutex = registry + 0x1374;
        crate::stream::bridge::mutex_lock(h, mutex);
        let tree = registry + 0x1344;
        let node = tree_find_key(h, tree, cluster);
        if node != 0 && node + 0x10 != 0 {
            release_blocks(h, node + 0x10);
            let node = tree_find_key(h, tree, cluster);
            if node != 0 {
                h.call_sym(0x0057_5000, Conv::Thiscall(tree), &[node]);
                h.call_sym(0x0049_c980, Conv::Thiscall(tree + 0x18), &[node]);
            }
        }
        crate::stream::bridge::mutex_unlock(h, mutex);
        0
    }

    /// `0x00579090` (thiscall `(&table, &data, extra, align)` on a cluster
    /// reader): sizes the reader's pointer table (`r = +8`: `r->+0x2c`
    /// words after `r->+0x38` bytes rounded to 4) plus `extra`, aligned;
    /// grows the cluster's arena when that is not 0
    /// (`0x00442010(cluster; size, r->+0x38, r->+0x10)`, its error
    /// returned); then the table at the arena's base plus the rounded
    /// size, zeroed (`memset`), and the data past the table; 0.
    pub fn place_tables(
        h: &mut impl Host,
        this: u32,
        table: u32,
        data: u32,
        extra: u32,
        align: u32,
    ) -> u32 {
        let r: u32 = h.read(this + 8);
        let head: u32 = h.read(r + 0x38);
        let words: u32 = h.read(r + 0x2c);
        let start = head.wrapping_add(3) & !3;
        let bytes = words.wrapping_mul(4);
        let mut end = bytes.wrapping_add(start);
        if align != 0 {
            end = end.wrapping_sub(1).wrapping_add(align) & !align.wrapping_sub(1);
        }
        let size = extra.wrapping_add(end);
        if size != 0 {
            let cluster: u32 = h.read(this);
            let flags: u32 = h.read(r + 0x10);
            let e = h.call_sym(0x0044_2010, Conv::Thiscall(cluster), &[size, head, flags]);
            if e != 0 {
                return e;
            }
        }
        let cluster: u32 = h.read(this);
        let arena: u32 = h.read(cluster + 0x24);
        let base: u32 = if arena == 0 { 0 } else { h.read(arena + 4) };
        let at = base.wrapping_add(start);
        h.write(table, at);
        h.call_sym(0x0094_964c, Conv::Cdecl, &[at, 0, bytes]);
        h.write(data, base.wrapping_add(end));
        0
    }

    /// `0x00442010` (thiscall `(size, used, count)` on a cluster): its
    /// arena, once: 5 for a size of 0 or `used` past it,
    /// [`crate::error::ALREADY`] when it has one; else one block
    /// (`0x0042fb80(0x20 + size rounded to 4 + count * 0x4c, 0x20)`,
    /// [`crate::error::NO_BLOCK`] when that fails) holding the arena's
    /// header (next, base, used, 0; linked at `+0x24`), its bytes, then
    /// `count` pool entries handed to the pool at `+0x28`
    /// (`0x0049c7c0`); 0.
    pub fn create_arena(h: &mut impl Host, cluster: u32, size: u32, used: u32, count: u32) -> u32 {
        if size == 0 || used > size {
            return 5;
        }
        let old: u32 = h.read(cluster + 0x24);
        if old != 0 {
            return crate::error::ALREADY;
        }
        let rounded = size.wrapping_add(3) & !3;
        let total = count
            .wrapping_mul(0x4c)
            .wrapping_add(0x20)
            .wrapping_add(rounded);
        let block = h.call_sym(crate::descriptor::ALLOC, Conv::Cdecl, &[total, 0x20]);
        if block == 0 {
            return crate::error::NO_BLOCK;
        }
        let base = block + 0x20;
        h.write(block + 4, [base, used, 0]);
        let next: u32 = h.read(cluster + 0x24);
        h.write(block, next);
        h.write(cluster + 0x24, block);
        h.call_sym(
            0x0049_c7c0,
            Conv::Thiscall(cluster + 0x28),
            &[base.wrapping_add(rounded), count],
        );
        0
    }

    /// The cluster registry's add (`0x00595e50`, thiscall `(&cluster,
    /// &record)` on the registry): a node from the pool at `+0x18` (grown
    /// by `0x0049c8d0(pool; +0x20)` when empty; 0xe when still empty),
    /// keyed by the cluster, holding a copy of the ten-word record, put in
    /// the tree ([`tree_insert`]); a duplicate goes back to the pool
    /// (`0x0049c980`) with [`crate::error::DUPLICATE`], else the tree's
    /// fix-up (`0x00574980`) and 0.
    pub fn registry_add(h: &mut impl Host, registry: u32, key: u32, record: u32) -> u32 {
        let pool = registry + 0x18;
        let mut node: u32 = h.read(pool);
        if node == 0 {
            let grow: u32 = h.read(pool + 8);
            h.call_sym(0x0049_c8d0, Conv::Thiscall(pool), &[grow]);
            node = h.read(pool);
            if node == 0 {
                return 0xe;
            }
        }
        let next: u32 = h.read(node);
        h.write(pool, next);
        let k: u32 = h.read(key);
        h.write(node, [node, node, node, k]);
        let words: [u32; 10] = h.read(record);
        h.write(node + 0x10, words);
        if tree_insert(h, registry, node) != 0 {
            h.call_sym(0x0049_c980, Conv::Thiscall(pool), &[node]);
            return crate::error::DUPLICATE;
        }
        h.call_sym(0x0057_4980, Conv::Thiscall(registry), &[node]);
        0
    }
}
