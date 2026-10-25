//! h2-render-b round 3: the larger render methods the earlier rounds left
//! (spin waits on job counters, free-list and red-black-tree inserts,
//! world cursors, constructors and destructors of the D3D11 objects,
//! array resizes, the sampler filter map, constant copies through a local
//! descriptor).
//!
//! As in [`crate::small`], bridges of originals that hand the address of
//! a local to a callee take that frame (`frame`: the original's `ebp -
//! size`, laid out at the original's offsets).

use ffx_graphic::host::{Conv, Host};

use crate::array::{BORROWED, COUNT};
use crate::descriptor::PHYRE_FREE;

/// `Phyre::PhyreOS::Sleep(ms)` (cdecl).
pub const SLEEP: u32 = 0x0043_0030;
/// `Phyre::PhyreMalloc(size, align)` (cdecl).
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
/// The CRT's `memset` thunk (cdecl).
pub const MEMSET: u32 = 0x0094_964c;
/// `PWorldCursor_nextList` (fastcall on the cursor).
pub const CURSOR_NEXT_LIST: u32 = 0x0044_3ef0;
/// `PWorldCursor_skipEmpty` (fastcall on the cursor).
pub const CURSOR_SKIP_EMPTY: u32 = 0x0050_09f0;

/// Releases the COM object in `slot` (method 2, `ecx` the object) and
/// clears the slot, when it is set.
fn release(h: &mut impl Host, slot: u32) {
    let obj: u32 = h.read(slot);
    if obj != 0 {
        let vt: u32 = h.read(obj);
        let f: u32 = h.read(vt + 8);
        h.call(f, Conv::Thiscall(obj), &[obj]);
        h.write(slot, 0u32);
    }
}

pub mod bridge {
    //! The originals over game memory.

    use super::{
        BORROWED, COUNT, CURSOR_NEXT_LIST, CURSOR_SKIP_EMPTY, Conv, Host, MEMSET, PHYRE_FREE,
        PHYRE_MALLOC, SLEEP, release,
    };

    /// `0x00564630`/`0x00564680(this)`: for each of the `count_off`
    /// array's elements (count word masked to 31 bits, read again after
    /// each), `Sleep(1)` while the counter `get(this; i)` (thiscall)
    /// points at is positive (signed). Returns the last count read (the
    /// original's `eax`; with no elements it leaves `eax` as it was).
    pub fn spin_wait(h: &mut impl Host, this: u32, count_off: u32, get: u32) -> u32 {
        let mut i = 0u32;
        let mut n = h.read::<u32>(this + count_off) & COUNT;
        while i < n {
            loop {
                let p = h.call_sym(get, Conv::Thiscall(this), &[i]);
                if h.read::<i32>(p) <= 0 {
                    break;
                }
                h.call_sym(SLEEP, Conv::Cdecl, &[1]);
            }
            n = h.read::<u32>(this + count_off) & COUNT;
            i += 1;
        }
        n
    }

    /// `destructInstance(p)` of a class whose members have destructors:
    /// `dtor(p + off)` (thiscall) for each, in order (no null check);
    /// `al = 1`.
    pub fn destruct_members(h: &mut impl Host, p: u32, members: &[(u32, u32)]) -> u8 {
        for &(off, dtor) in members {
            h.call_sym(dtor, Conv::Thiscall(p.wrapping_add(off)), &[]);
        }
        1
    }

    /// `PInstanceList::PInstanceList(this; type, owner)`: an empty list
    /// head at `+0` and `+0x20`, the free list at `+8` built by
    /// `PFreeList::PFreeList(+8; type +0x1c, 0, type +0x20, type +0x18)`,
    /// the owner and type at `+0x28`/`+0x2c`, six zero words and the
    /// half-word `0x101` at `+0x48`; returns `this`.
    pub fn instance_list_ctor(h: &mut impl Host, this: u32, ty: u32, owner: u32) -> u32 {
        h.write(this, this);
        h.write(this + 4, this);
        let args = [
            h.read(ty + 0x1c),
            0,
            h.read::<u32>(ty + 0x20),
            h.read::<u32>(ty + 0x18),
        ];
        h.call_sym(0x0049_c480, Conv::Thiscall(this + 8), &args);
        h.write(this + 0x20, this + 0x20);
        h.write(this + 0x24, this + 0x20);
        h.write(this + 0x28, owner);
        h.write(this + 0x2c, ty);
        for off in (0x30..=0x44).step_by(4) {
            h.write(this + off, 0u32);
        }
        h.write(this + 0x48, 0x101u16);
        this
    }

    /// `0x00505010(this)`: `0x00507360([this+4]; this)` when `+4` is set;
    /// then each node of the singly linked list at `+8` gets its `+4`
    /// cleared and is unlinked (its link cleared); then the word at
    /// `+0x50` is freed unless its bit 0 is set.
    pub fn detach_all(h: &mut impl Host, this: u32) {
        let o: u32 = h.read(this + 4);
        if o != 0 {
            h.call_sym(0x0050_7360, Conv::Thiscall(o), &[this]);
        }
        let mut c: u32 = h.read(this + 8);
        while c != 0 {
            let next: u32 = h.read(c);
            h.write(c + 4, 0u32);
            let mut prev = this + 8;
            let mut x: u32 = h.read(prev);
            while x != 0 {
                if x == c {
                    let link: u32 = h.read(x);
                    h.write(prev, link);
                    h.write(x, 0u32);
                    break;
                }
                prev = x;
                x = h.read(x);
            }
            c = next;
        }
        if h.read::<u8>(this + 0x50) & 1 == 0 {
            let p: u32 = h.read(this + 0x50);
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[p]);
        }
    }

    /// `constructInstance(p)` of a class whose constructor takes its
    /// type: the registry's lookup `0x0043cbb0` (`al = 0` when null), then
    /// `ctor(p; type)` (thiscall) when `p` is not null; `al = 1`.
    pub fn construct_with_type(h: &mut impl Host, p: u32, registry: u32, ctor: u32) -> u8 {
        let r = h.rebase(registry);
        let t = h.call_sym(crate::descriptor::REGISTRY_LOOKUP, Conv::Thiscall(r), &[]);
        if t == 0 {
            return 0;
        }
        if p != 0 {
            h.call_sym(ctor, Conv::Thiscall(p), &[t]);
        }
        1
    }

    /// `0x0056e600(buffer; desc, src)`: a local descriptor `(0x47, 0,
    /// src, 0)` at `frame`; 3 unless the parameter (offset `desc +8`,
    /// size `desc +0xa` low 13 bits) fits the buffer's size word and lies
    /// at offset 4 or more; else `copy(buffer + offset, &local, size)`
    /// (cdecl, the CRT import at `0x00949256`) and 0.
    pub fn set_value_local(h: &mut impl Host, buffer: u32, desc: u32, src: u32, frame: u32) -> u32 {
        h.write(frame + 8, src);
        let off = u32::from(h.read::<u16>(desc + 8));
        let size = u32::from(h.read::<u16>(desc + 0xa) & 0x1fff);
        h.write(frame, 0x47u32);
        h.write(frame + 4, 0u32);
        h.write(frame + 0xc, 0u32);
        if off + size > h.read::<u32>(buffer) || off < 4 {
            return 3;
        }
        h.call_sym(
            0x0094_9256,
            Conv::Cdecl,
            &[buffer.wrapping_add(off), frame, size],
        );
        0
    }

    /// `0x0056e670(this; desc, tex)`: 5 unless the parameter's type byte
    /// (`desc +2`) is one of the texture types; when `tex`, its `+8` and
    /// `+0xc` are set, `0x0059e520([tex+8]; [tex+0xc])` first, whose
    /// non-zero result is returned; else `0x004d57d0(this; desc, tex)`.
    pub fn set_texture_checked(h: &mut impl Host, this: u32, desc: u32, tex: u32) -> u32 {
        let t: u8 = h.read(desc + 2);
        if !matches!(t, 0x42 | 0xd9 | 0xda | 0xdb | 0xdc | 0x12) {
            return 5;
        }
        if tex != 0 {
            let o: u32 = h.read(tex + 8);
            if o != 0 {
                let a: u32 = h.read(tex + 0xc);
                if a != 0 {
                    let r = h.call_sym(0x0059_e520, Conv::Thiscall(o), &[a]);
                    if r != 0 {
                        return r;
                    }
                }
            }
        }
        h.call_sym(0x004d_57d0, Conv::Thiscall(this), &[desc, tex])
    }

    /// A function-local static ([`crate::small::bridge::static_once`]),
    /// then `then(object)` (thiscall); returns its `eax`.
    pub fn static_once_then(
        h: &mut impl Host,
        guard: u32,
        object: u32,
        ctor: u32,
        dtor: u32,
        then: u32,
    ) -> u32 {
        let o = crate::small::bridge::static_once(h, guard, object, ctor, dtor);
        h.call_sym(then, Conv::Thiscall(o), &[])
    }

    /// `0x00595ee0(this; key)`: a node from the free list at `+0x18`
    /// (`PFreeList::allocateNewBlock(+0x18; [+0x20])` when it is empty;
    /// 0xe when still empty), its three links at itself and `[key]` at
    /// `+0xc`; `0x005817d0(this; node)` inserts it: when that finds the
    /// key taken, the node goes back (`PFreeList::deallocate`) and 0x12;
    /// else `PRedBlackTree::postInsertFinalize(this; node)` and 0.
    pub fn tree_add(h: &mut impl Host, this: u32, key: u32) -> u32 {
        let mut node: u32 = h.read(this + 0x18);
        if node == 0 {
            let a: u32 = h.read(this + 0x20);
            h.call_sym(0x0049_c8d0, Conv::Thiscall(this + 0x18), &[a]);
            node = h.read(this + 0x18);
            if node == 0 {
                return 0xe;
            }
        }
        let next: u32 = h.read(node);
        h.write(this + 0x18, next);
        h.write(node, node);
        h.write(node + 4, node);
        h.write(node + 8, node);
        let k: u32 = h.read(key);
        h.write(node + 0xc, k);
        if h.call_sym(0x0058_17d0, Conv::Thiscall(this), &[node]) != 0 {
            h.call_sym(0x0049_c980, Conv::Thiscall(this + 0x18), &[node]);
            return 0x12;
        }
        h.call_sym(0x0057_4980, Conv::Thiscall(this), &[node]);
        0
    }

    /// `(items, n)` (cdecl): `f(items + i * stride)` (thiscall) for each
    /// of the `n` elements whose address is not null.
    pub fn for_each(h: &mut impl Host, items: u32, n: u32, stride: u32, f: u32) {
        for i in 0..n {
            let p = items.wrapping_add(i.wrapping_mul(stride));
            if p != 0 {
                h.call_sym(f, Conv::Thiscall(p), &[]);
            }
        }
    }

    /// `PWorldCursor_skipEmpty(cursor)` (fastcall): while the cursor has
    /// no objects left (`+0x18`) and a list (`+8`), the next list
    /// (`PWorldCursor_nextList`) and its words (stride `+0x14`, slot
    /// `+0xc`, count `+0x18`, class `+0x1c`, free slot `+0x10`); free
    /// slots are passed (the slot equals the next free one: step, count
    /// down, follow the free link).
    pub fn skip_empty(h: &mut impl Host, c: u32) {
        if h.read::<u32>(c + 0x18) != 0 {
            return;
        }
        loop {
            if h.read::<u32>(c + 8) == 0 {
                return;
            }
            h.call_sym(CURSOR_NEXT_LIST, Conv::Thiscall(c), &[]);
            let list: u32 = h.read(c + 8);
            if list == 0 {
                return;
            }
            let class: u32 = h.read(list + 0xc);
            let w: u32 = h.read(class + 0x1c);
            h.write(c + 0x14, w);
            let w: u32 = h.read(list + 0x14);
            h.write(c + 0xc, w);
            let w: u32 = h.read(list + 0x1c);
            h.write(c + 0x18, w);
            let w: u32 = h.read(class + 0x8c);
            h.write(c + 0x1c, w);
            let w: u32 = h.read(list.wrapping_sub(0x18));
            h.write(c + 0x10, w);
            if h.read::<u32>(c + 0x18) == 0 {
                continue;
            }
            loop {
                let free: u32 = h.read(c + 0x10);
                let slot: u32 = h.read(c + 0xc);
                if free != slot {
                    break;
                }
                let stride: u32 = h.read(c + 0x14);
                let n = h.read::<u32>(c + 0x18).wrapping_sub(1);
                h.write(c + 0x18, n);
                h.write(c + 0xc, stride.wrapping_add(slot));
                let link: u32 = h.read(free);
                h.write(c + 0x10, link);
                if n == 0 {
                    break;
                }
            }
            if h.read::<u32>(c + 0x18) != 0 {
                return;
            }
        }
    }

    /// `0x0058aa10(mode, a, b)` (cdecl): the D3D11 filter of a sampler
    /// mode (0..5; others 0) and whether `a` is set; the anisotropic
    /// filter `0x55` when the anisotropy at `0x00c25370` is above 1
    /// (ordered); `| 0x80` (comparison) when the byte `b` is set.
    pub fn filter(h: &mut impl Host, mode: u32, a: u32, b: u32) -> u32 {
        let set = u32::from(a != 0);
        let mut f = match mode {
            0 | 2 => 4 * set,
            1 | 3 => 0x10 + 4 * set,
            4 => 1 + 4 * set,
            5 => 0x11 + 4 * set,
            _ => 0,
        };
        let aniso = f32::from_bits(h.read(h.rebase(0x00c2_5370)));
        if 1.0 < aniso {
            f = 0x55;
        }
        if b as u8 != 0 {
            f |= 0x80;
        }
        f
    }

    /// `0x00566680(cluster; a)`: `PCluster::allocateInstanceList(cluster;
    /// 1, type 0x00ca7228, 0)`; when that gives a list whose `+0x34` is
    /// set, `0x004f27e0([list+0x34]; a)` and its `eax`; else 0.
    pub fn instance_list_call(h: &mut impl Host, cluster: u32, a: u32) -> u32 {
        let t = h.rebase(0x00ca_7228);
        let list = h.call_sym(0x0044_0b70, Conv::Thiscall(cluster), &[1, t, 0]);
        if list == 0 {
            return 0;
        }
        let o: u32 = h.read(list + 0x34);
        if o == 0 {
            return 0;
        }
        h.call_sym(0x004f_27e0, Conv::Thiscall(o), &[a])
    }

    /// `0x00583320(this)`: words `+0x40..+0x5c` zero, `0x00583e20(+0x60)`
    /// (thiscall), words `+0..+0x3c` zero; returns `this`.
    pub fn ctor_583320(h: &mut impl Host, this: u32) -> u32 {
        for off in (0x40..=0x5c).step_by(4) {
            h.write(this + off, 0u32);
        }
        h.call_sym(0x0058_3e20, Conv::Thiscall(this + 0x60), &[]);
        for off in (0..=0x3c).step_by(4) {
            h.write(this + off, 0u32);
        }
        this
    }

    /// `0x0059a200(this; color)`: the four words of `color` into
    /// `+0x2a8..`; with no color, those of a default `PRenderSurface`-like
    /// local at `frame` (`0x005b3620(&local)` gives the words,
    /// `0x005b3640(&local)` destroys it). Returns 0.
    pub fn set_color(h: &mut impl Host, this: u32, color: u32, frame: u32) -> u32 {
        if color != 0 {
            for i in 0..4 {
                let w: u32 = h.read(color + 4 * i);
                h.write(this + 0x2a8 + 4 * i, w);
            }
            return 0;
        }
        let p = h.call_sym(0x005b_3620, Conv::Thiscall(frame), &[]);
        for i in 0..4 {
            let w: u32 = h.read(p.wrapping_add(4 * i));
            h.write(this + 0x2a8 + 4 * i, w);
        }
        h.call_sym(0x005b_3640, Conv::Thiscall(frame), &[]);
        0
    }

    /// `0x0059c9c0(this)`: releases the objects at `+0x4ec` and `+0`, frees
    /// the owned byte array at `+4` (data `+8`) and clears it when its
    /// count is not 0, then releases `+0x4e8`.
    pub fn release_program(h: &mut impl Host, this: u32) {
        release(h, this + 0x4ec);
        release(h, this);
        let n: u32 = h.read(this + 4);
        if n != 0 {
            let data: u32 = h.read(this + 8);
            if data != 0 && n & BORROWED == 0 {
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
            }
            h.write(this + 8, 0u32);
            h.write(this + 4, 0u32);
        }
        release(h, this + 0x4e8);
    }

    /// `PArray<u32>::resize(this; n)`: nothing when the count word is `n`
    /// already; else new zeroed storage (`PhyreMalloc(4n, 4)`, none for 0;
    /// 0xd when it fails), the old storage freed when owned and not the
    /// new one, and `n` (owned) stored. Returns 0.
    pub fn resize_words(h: &mut impl Host, this: u32, n: u32) -> u32 {
        if n == h.read::<u32>(this) {
            return 0;
        }
        let mut data = 0;
        if n != 0 {
            data = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[n.wrapping_mul(4), 4]);
            if data == 0 {
                return 0xd;
            }
            for i in 0..n {
                h.write(data.wrapping_add(4 * i), 0u32);
            }
        }
        let old: u32 = h.read(this + 4);
        if old != data && h.read::<u32>(this) & BORROWED == 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
        }
        h.write(this + 4, data);
        h.write(this, n);
        0
    }

    /// `PWorldCursor<PMeshInstanceBounds>(cursor; world)`: the world's
    /// class entry (`0x004437e0(world; 0x00ca89b8)`) at `+0` and `+8`;
    /// when found, its count (`+0x10`) at `+4` and its first list
    /// (`+0x14`) at `+0`, else `+4 = -1`; `PWorldCursor_nextList`; the
    /// scene cursor at `+0xc` zeroed and set to the entry's node
    /// (`entry - 0x20`, `PhyreSceneCursor_SetNode`) when there is one;
    /// `PWorldCursor_skipEmpty`. Returns `cursor`.
    pub fn bounds_cursor(h: &mut impl Host, c: u32, world: u32) -> u32 {
        let class = h.rebase(0x00ca_89b8);
        let e = h.call_sym(0x0044_37e0, Conv::Thiscall(world), &[class]);
        h.write(c, e);
        h.write(c + 8, e);
        if e == 0 {
            h.write(c + 4, u32::MAX);
        } else {
            let n: u32 = h.read(e + 0x10);
            h.write(c + 4, n);
            h.write(c, e + 0x14);
        }
        h.call_sym(CURSOR_NEXT_LIST, Conv::Thiscall(c), &[]);
        for i in 0..5 {
            h.write(c + 0xc + 4 * i, 0u32);
        }
        let e: u32 = h.read(c + 8);
        if e != 0 {
            h.call_sym(
                0x0044_1dc0,
                Conv::Thiscall(c + 0xc),
                &[e.wrapping_sub(0x20)],
            );
        }
        h.call_sym(CURSOR_SKIP_EMPTY, Conv::Thiscall(c), &[]);
        c
    }

    /// `PTexture2DD3D11::PTexture2DD3D11(this)`: the base constructor
    /// `0x004c2f90`, `+0x24 = 0`, the list at `+0x28` set up
    /// (`0x0058b070(+0x28, 1)`, cdecl) and `+0x24 = 1` (unless `+0x28` is
    /// null), `+0x70 = 0`, `memset(+0x44, 0, 0x2c)`. Returns `this`.
    pub fn texture2d_ctor(h: &mut impl Host, this: u32) -> u32 {
        h.call_sym(0x004c_2f90, Conv::Thiscall(this), &[]);
        let list = this.wrapping_add(0x28);
        h.write(this + 0x24, 0u32);
        if list != 0 {
            h.call_sym(0x0058_b070, Conv::Cdecl, &[list, 1]);
            h.write(this + 0x24, 1u32);
        }
        h.write(this + 0x70, 0u32);
        h.call_sym(MEMSET, Conv::Cdecl, &[this + 0x44, 0, 0x2c]);
        this
    }

    /// `PMeshInstance::computeJointMatrices(this; joint)`: the root matrix
    /// (`PMatrix4_FromMatrix3x4` of `*(this +4)` into `frame`, or the
    /// identity at `frame + 0x40` when there is none or bit 0 of `+0x6c`
    /// is set) copied below `joint` (`PMatrix4_Copy(joint - 0x40; m)`),
    /// then the tail call `computeJointMatrix(this; joint)`; returns its
    /// `eax`. `frame` is the original's `ebp - 0x80`.
    pub fn compute_joint_matrices(h: &mut impl Host, this: u32, joint: u32, frame: u32) -> u32 {
        let src: u32 = if h.read::<u8>(this + 0x6c) & 1 != 0 {
            0
        } else {
            h.read(this + 4)
        };
        let m = if src != 0 {
            h.call_sym(0x0045_1080, Conv::Thiscall(frame), &[src]);
            frame
        } else {
            let m = frame + 0x40;
            for i in 0..16 {
                let v: u32 = if i % 5 == 0 { 0x3f80_0000 } else { 0 };
                h.write(m + 4 * i, v);
            }
            m
        };
        h.call_sym(0x0045_3340, Conv::Thiscall(joint.wrapping_sub(0x40)), &[m]);
        h.call_sym(0x004f_6fc0, Conv::Thiscall(this), &[joint])
    }

    /// `0x005817d0(tree; node)`: the red-black tree insert: the node's
    /// links at the header, its parent word's colour bit set; down from
    /// the root (`+0xc`) by key (`+0xc`, unsigned; greater goes left):
    /// a node with the same key is returned; else the node hangs under
    /// the last one (parent word: colour bit + parent link) and 0.
    pub fn tree_insert(h: &mut impl Host, tree: u32, node: u32) -> u32 {
        let f: u32 = h.read(node + 8);
        let bit = u32::from(h.read::<u8>(node + 8) & 1);
        h.write(node + 8, f.wrapping_add(1 - bit));
        h.write(node, tree);
        h.write(node + 4, tree);
        let mut parent = tree + 0xc;
        let mut x: u32 = h.read(parent);
        let mut right = 0u32;
        if x != tree {
            let k: u32 = h.read(node + 0xc);
            loop {
                let v: u32 = h.read(x + 0xc);
                parent = x;
                if v == k {
                    return x;
                }
                right = u32::from(v < k);
                x = h.read(x + 4 * right);
                if x == tree {
                    break;
                }
            }
        }
        let bit = u32::from(h.read::<u8>(node + 8) & 1);
        h.write(node + 8, bit.wrapping_add(parent));
        h.write(parent + 4 * right, node);
        0
    }

    /// `0x0055dd30(this)`: the zeroed words, the half-word `0x34` at
    /// `+0xc`, an empty block at `+0x18` (5 words) with `+0x14 = 1`
    /// (unless `+0x18` is null), four zero words at `+0x2c`; returns
    /// `this`.
    pub fn ctor_55dd30(h: &mut impl Host, this: u32) -> u32 {
        for off in [0, 4, 8] {
            h.write(this + off, 0u32);
        }
        h.write(this + 0xc, 0x34u16);
        h.write(this + 0xe, 0u8);
        h.write(this + 0x10, 0u32);
        h.write(this + 0x14, 0u32);
        let b = this.wrapping_add(0x18);
        if b != 0 {
            for i in 0..5 {
                h.write(b + 4 * i, 0u32);
            }
            h.write(this + 0x14, 1u32);
        }
        for off in [0x2c, 0x30, 0x34, 0x38] {
            h.write(this + off, 0u32);
        }
        this
    }

    /// A function-local static built by `ctor(object; args)` (thiscall;
    /// the words marked `true` are image addresses, rebased) whose
    /// vtable is then `vtable`; on first use only (guard bit 0), with
    /// `atexit(dtor)`. Returns the object.
    pub fn static_once_args(
        h: &mut impl Host,
        guard: u32,
        object: u32,
        ctor: u32,
        args: &[(u32, bool)],
        vtable: u32,
        dtor: u32,
    ) -> u32 {
        let g = h.rebase(guard);
        let o = h.rebase(object);
        let v: u32 = h.read(g);
        if v & 1 == 0 {
            h.write(g, v | 1);
            let words: Vec<u32> = args
                .iter()
                .map(|&(w, image)| if image { h.rebase(w) } else { w })
                .collect();
            h.call_sym(ctor, Conv::Thiscall(o), &words); // call width: 5 at most
            let vt = h.rebase(vtable);
            h.write(o, vt);
            let d = h.rebase(dtor);
            h.call_sym(crate::declare::ATEXIT, Conv::Cdecl, &[d]);
        }
        o
    }
}
