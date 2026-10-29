//! M5 stage 1, list `scene`, batch 5: array and `PSharray` resizes of the
//! render and gameplay classes, the LOD group's external buffer, the
//! gameplay utility's clean-up and its type registry, the tree-with-free-
//! list destructors, the shadow caster's split distances and the world
//! cursor's advance. Conventions as in [`crate::m5p_scene_b`].
//!
//! Image addresses are rebased before use. Nothing here keeps state of
//! its own (worker threads: per-call scratch only).

use ffx_bridge::{Mem, x87};
use ffx_graphic::host::{Conv, Host};

pub use crate::m5p_scene_b::PHYRE_FREE;
pub use crate::m5p_scene_c::PHYRE_MALLOC;
pub use crate::m5p_scene_d::{FREE_LIST_DTOR, FREE_LIST_RESET, TREE_CLEAR, TREE_ORPHAN};

/// `Phyre_PRedBlackTree_remove` (thiscall `(tree; node)`, `ret 4`).
pub const TREE_REMOVE: u32 = 0x0057_5000;
/// The gameplay type list (`{name, next, registered}` entries) and the two
/// free lists `0x004334e0` resets and `0x0054d1e0` allocates from (entry,
/// then name; each `{head, ?, block size}`).
pub const GAMEPLAY_TYPES: u32 = 0x00ca_ece8;
pub const GAMEPLAY_TYPE_POOL: u32 = 0x00ca_ecec;
pub const GAMEPLAY_NAME_POOL: u32 = 0x00ca_ed04;
/// `Phyre_PFreeList_allocateNewBlock` (thiscall `(list; size)`, `ret 4`).
pub const FREE_LIST_GROW: u32 = 0x0049_c8d0;
/// The CRT's `memcpy` (its thunk, cdecl).
pub const MEMCPY: u32 = 0x0094_9256;
/// `Phyre_PhyrePrintf(level, format, ...)` (cdecl).
pub const PRINTF: u32 = 0x0043_52f0;
/// The message `0x0054d1e0` prints for a new gameplay type.
pub const NEW_TYPE_MESSAGE: u32 = 0x00b1_fa50;
/// `0x0054d190`, the gameplay type lookup (ported in batch 2).
pub const FIND_TYPE: u32 = 0x0054_d190;
/// `0x0054d040`, the gameplay type constructor (ported in batch 2).
pub const TYPE_CTOR: u32 = 0x0054_d040;
/// `Phyre_PLOD_element_dtor_005a29d0` (thiscall): a `PLODLevel`'s
/// destructor (0x24 bytes).
pub const LOD_LEVEL_DTOR: u32 = 0x005a_29d0;
/// The `PSharray<T*>` buffer constructors the resizes and the copy
/// constructor call (thiscall on the header): with a fill (`(n, data,
/// keep)`, `ret 0xc`) and without (`(n, data)`, `ret 8`).
pub const STREAM_BINDINGS_FILL: u32 = 0x004f_6e10;
pub const RECEIVERS_FILL: u32 = 0x0054_fa00;
pub const SEGMENT_CONTEXTS_COPY: u32 = 0x004f_6db0;
/// `PArray<PSceneRenderPass>`: the copy of kept elements `(dst, src, n)`
/// (cdecl), the constructor of new ones (`0x004e6250`, cdecl `(items,
/// n)`) and the storage set (thiscall `(data, n)`, `ret 8`).
pub const RENDER_PASS_COPY: u32 = 0x004e_1ae0;
pub const RENDER_PASS_CONSTRUCT: u32 = 0x004e_6250;
pub const RENDER_PASS_SET_STORAGE: u32 = 0x004e_7780;
/// `PArray<PMaterialSwitch>`: the copy of kept elements `(dst, src, n)`,
/// the constructor of new ones (`0x004e61d0`) and the release of the old
/// buffer (`Phyre_FreePairs_004e4df0`, `(items, n)`) (all cdecl).
pub const MATERIAL_COPY: u32 = 0x004e_19c0;
pub const MATERIAL_CONSTRUCT: u32 = 0x004e_61d0;
pub const MATERIAL_RELEASE: u32 = 0x004e_4df0;
/// The default split distance (a float).
pub const DEFAULT_SPLIT: u32 = 0x00b1_0f50;
/// `PWorldCursor_skipEmpty_009f3930` (thiscall).
pub const CURSOR_SKIP_EMPTY: u32 = 0x009f_3930;
/// `0x00577da0`, the second tree-with-free-list destructor (ported here).
pub const TREE_DTOR_REMOVING: u32 = 0x0057_7da0;

pub mod bridge {
    //! The ports over game memory.

    use super::{
        CURSOR_SKIP_EMPTY, Conv, DEFAULT_SPLIT, FIND_TYPE, FREE_LIST_DTOR, FREE_LIST_GROW,
        FREE_LIST_RESET, GAMEPLAY_NAME_POOL, GAMEPLAY_TYPE_POOL, GAMEPLAY_TYPES, Host,
        LOD_LEVEL_DTOR, MATERIAL_CONSTRUCT, MATERIAL_COPY, MATERIAL_RELEASE, MEMCPY, Mem,
        NEW_TYPE_MESSAGE, PHYRE_FREE, PHYRE_MALLOC, PRINTF, RENDER_PASS_CONSTRUCT,
        RENDER_PASS_COPY, RENDER_PASS_SET_STORAGE, TREE_CLEAR, TREE_ORPHAN, TREE_REMOVE, TYPE_CTOR,
        x87,
    };

    const EXTERNAL: u32 = 0x8000_0000;

    /// The data of a `PSharray` header at `h` (none, its own word, the
    /// buffer).
    fn sharray_data(m: &impl Mem, h: u32) -> u32 {
        let count: u32 = m.read(h);
        match count & !EXTERNAL {
            0 => 0,
            1 => h + 4,
            _ => m.read(h + 4),
        }
    }

    /// `0x004334e0` (cdecl): the gameplay utility's clean-up: each
    /// registered type (`+8` set) unlinked from the type list, then both
    /// free lists reset; the second reset's `eax`.
    pub fn gameplay_cleanup(h: &mut impl Host) -> u32 {
        let head = h.rebase(GAMEPLAY_TYPES);
        let mut first: u32 = h.read(head);
        let mut e = first;
        while e != 0 {
            let registered: u8 = h.read(e + 8);
            if registered == 0 {
                e = h.read(e + 4);
                continue;
            }
            let link = e + 4;
            let next: u32 = h.read(link);
            let mut slot = head;
            let mut p = first;
            while p != 0 {
                if p + 4 == link {
                    let after: u32 = h.read(p + 4);
                    h.write(slot, after);
                    h.write(p + 4, 0u32);
                    first = h.read(head);
                    break;
                }
                slot = p + 4;
                p = h.read(slot);
            }
            e = next;
        }
        let types = h.rebase(GAMEPLAY_TYPE_POOL);
        h.call_sym(FREE_LIST_RESET, Conv::Thiscall(types), &[]);
        let names = h.rebase(GAMEPLAY_NAME_POOL);
        h.call_sym(FREE_LIST_RESET, Conv::Thiscall(names), &[])
    }

    /// A node from a free list (`{head, ?, block size}`): its head, after
    /// a new block when it is empty; 0 when none could be had.
    fn pool_take(h: &mut impl Host, pool: u32) -> u32 {
        let mut node: u32 = h.read(pool);
        if node == 0 {
            let size: u32 = h.read(pool + 8);
            h.call_sym(FREE_LIST_GROW, Conv::Thiscall(pool), &[size]);
            node = h.read(pool);
            if node == 0 {
                return 0;
            }
        }
        let next: u32 = h.read(node);
        h.write(pool, next);
        node
    }

    /// `0x0054d1e0` (cdecl `(name)`): the gameplay type of `name`: the
    /// listed one, or a new one (a message printed; an entry and a copy of
    /// the name from the two free lists, constructed and listed by
    /// [`TYPE_CTOR`]). 0 for a null name; the entry without a name when
    /// the name list is out of nodes; 0 when the entry list is.
    pub fn gameplay_type(h: &mut impl Host, name: u32) -> u32 {
        let found = h.call_sym(FIND_TYPE, Conv::Cdecl, &[name]);
        if name == 0 || found != 0 {
            return found;
        }
        let msg = h.rebase(NEW_TYPE_MESSAGE);
        h.call_sym(PRINTF, Conv::Cdecl, &[1, msg, name]);
        let types = h.rebase(GAMEPLAY_TYPE_POOL);
        let entry = pool_take(h, types);
        let names = h.rebase(GAMEPLAY_NAME_POOL);
        let copy = pool_take(h, names);
        if entry == 0 || copy == 0 {
            return entry;
        }
        let mut n = 0u32;
        loop {
            let c: u8 = h.read(name.wrapping_add(n));
            if c == 0 {
                break;
            }
            n += 1;
        }
        h.call_sym(MEMCPY, Conv::Cdecl, &[copy, name, n + 1]);
        h.call_sym(TYPE_CTOR, Conv::Thiscall(entry), &[0, copy])
    }

    /// `0x005a7d90` (thiscall `(buffer, n)`, `ret 8`): the LOD group's
    /// `PArray<PLODLevel>::setToExternalBuffer`: an owned, non-null old
    /// buffer other than `buffer` has its levels destroyed (0x24 bytes
    /// each) and is freed; then the header is `(n, buffer)`. `n`.
    pub fn lod_levels_set_external(h: &mut impl Host, this: u32, buffer: u32, n: u32) -> u32 {
        let header: u32 = h.read(this);
        let old: u32 = h.read(this + 4);
        if old != buffer && header & EXTERNAL == 0 && old != 0 {
            let mut p = old;
            for _ in 0..header & !EXTERNAL {
                h.call_sym(LOD_LEVEL_DTOR, Conv::Thiscall(p), &[]);
                p = p.wrapping_add(0x24);
            }
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
        }
        h.write(this + 4, buffer);
        h.write(this, n);
        n
    }

    /// `0x004f8a40`, `0x00550530` (thiscall `(n)`, `ret 4`): a
    /// `PSharray<T*>` resized to `n`: a new buffer with the kept elements
    /// and zeros (`fill`, thiscall on the header `(n, data, keep)`), the
    /// old data freed when owned and neither the new data nor the
    /// header's own word, the buffer word set unless the data is the
    /// header's own. 0, 0xd when the fill gave none; nothing changes when
    /// the header word is `n`.
    pub fn sharray_resize(h: &mut impl Host, this: u32, n: u32, fill: u32) -> u32 {
        let header: u32 = h.read(this);
        if n == header {
            return 0;
        }
        let count = header & !EXTERNAL;
        let data = sharray_data(h, this);
        let keep = if n < count { n } else { count };
        let new = h.call_sym(fill, Conv::Thiscall(this), &[n, data, keep]);
        if new == 0 && n != 0 {
            return 0xd;
        }
        let own = this + 4;
        if data != new && data != own {
            let header: u32 = h.read(this);
            if header & EXTERNAL == 0 {
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
            }
        }
        if new != own {
            h.write(own, new);
        }
        h.write(this, n);
        0
    }

    /// `0x004e75e0` (thiscall `(n)`, `ret 4`): a `PArray<PSceneRenderPass>`
    /// (0x28-byte elements) resized to `n`: a new buffer
    /// (`PhyreMalloc(0x28 n, 4)`, none for 0) with the kept elements
    /// copied and the rest constructed, handed to the array's storage set.
    /// 0, 0xd when the allocation failed; nothing changes when the header
    /// word is `n`.
    pub fn render_pass_array_resize(h: &mut impl Host, this: u32, n: u32) -> u32 {
        let header: u32 = h.read(this);
        if n == header {
            return 0;
        }
        let count = header & !EXTERNAL;
        let keep = if n < count { n } else { count };
        let old: u32 = h.read(this + 4);
        let mut new = 0;
        if n != 0 {
            new = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[n.wrapping_mul(0x28), 4]);
            if new == 0 {
                return 0xd;
            }
            h.call_sym(RENDER_PASS_COPY, Conv::Cdecl, &[new, old, keep]);
            let rest = new.wrapping_add(keep.wrapping_mul(0x28));
            h.call_sym(
                RENDER_PASS_CONSTRUCT,
                Conv::Cdecl,
                &[rest, n.wrapping_sub(keep)],
            );
        }
        h.call_sym(RENDER_PASS_SET_STORAGE, Conv::Thiscall(this), &[new, n]);
        0
    }

    /// `0x004e7530` (thiscall `(n)`, `ret 4`): a `PArray<PMaterialSwitch>`
    /// (8-byte elements) resized to `n`: a new buffer (none for 0) with
    /// the kept elements copied and the rest constructed; an owned old
    /// buffer other than it released (`(buffer, count)`). 0, 0xd when the
    /// allocation failed; nothing changes when the header word is `n`.
    pub fn material_array_resize(h: &mut impl Host, this: u32, n: u32) -> u32 {
        let header: u32 = h.read(this);
        if n == header {
            return 0;
        }
        let count = header & !EXTERNAL;
        let keep = if n < count { n } else { count };
        let old: u32 = h.read(this + 4);
        let mut new = 0;
        if n != 0 {
            new = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[n.wrapping_mul(8), 4]);
            if new == 0 {
                return 0xd;
            }
            h.call_sym(MATERIAL_COPY, Conv::Cdecl, &[new, old, keep]);
            let rest = new.wrapping_add(keep.wrapping_mul(8));
            h.call_sym(
                MATERIAL_CONSTRUCT,
                Conv::Cdecl,
                &[rest, n.wrapping_sub(keep)],
            );
        }
        let cur: u32 = h.read(this + 4);
        if cur != new {
            let header: u32 = h.read(this);
            if header & EXTERNAL == 0 {
                h.call_sym(MATERIAL_RELEASE, Conv::Cdecl, &[cur, header & !EXTERNAL]);
            }
        }
        h.write(this + 4, new);
        h.write(this, n);
        0
    }

    /// The free list and tree teardown both tree destructors end with:
    /// the free list (`+0x18`) reset and destroyed, the tree cleared
    /// twice; the last call's `eax`.
    fn tree_teardown(h: &mut impl Host, this: u32) -> u32 {
        h.call_sym(FREE_LIST_RESET, Conv::Thiscall(this + 0x18), &[]);
        h.call_sym(FREE_LIST_DTOR, Conv::Thiscall(this + 0x18), &[]);
        h.call_sym(TREE_CLEAR, Conv::Thiscall(this), &[]);
        h.call_sym(TREE_CLEAR, Conv::Thiscall(this), &[])
    }

    /// `0x00577da0` (thiscall): a tree with a free list destroyed by
    /// removing its root until it is empty (or null), then the teardown.
    pub fn tree_dtor_removing(h: &mut impl Host, this: u32) -> u32 {
        loop {
            let root: u32 = h.read(this + 0xc);
            if root == this || root == 0 {
                break;
            }
            h.call_sym(TREE_REMOVE, Conv::Thiscall(this), &[root]);
        }
        tree_teardown(h, this)
    }

    /// `0x00578030` (thiscall): an owner of three trees with free lists
    /// destroyed: `+0x64` by [`tree_dtor_removing`], `+0x34` and `+4`
    /// orphaned, then torn down; the last call's `eax`.
    pub fn three_trees_dtor(h: &mut impl Host, this: u32) -> u32 {
        tree_dtor_removing(h, this + 0x64);
        h.call_sym(TREE_ORPHAN, Conv::Thiscall(this + 0x34), &[]);
        tree_teardown(h, this + 0x34);
        h.call_sym(TREE_ORPHAN, Conv::Thiscall(this + 4), &[]);
        tree_teardown(h, this + 4)
    }

    /// `PShadowCaster_splitDistances` (`0x005b8c70`, thiscall `(out)`,
    /// `ret 4`): the four cascade split distances (the cascades' `+0xc4`
    /// of each 0xe0-byte cascade at `+0xc`, count `+8`) into `out`, the
    /// default for each cascade that doesn't exist. `out`.
    pub fn split_distances(m: &mut impl Mem, this: u32, out: u32) -> u32 {
        let count: u32 = m.read(this + 8);
        let count = count & !EXTERNAL;
        let at = m.rebase(DEFAULT_SPLIT);
        let default: f32 = m.read(at);
        let cascades: u32 = m.read(this + 0xc);
        for i in 0..4u32 {
            let v = if count > i {
                m.read(cascades + 0xc4 + 0xe0 * i)
            } else {
                default
            };
            m.write(out + 4 * i, x87::round_trip(v));
        }
        out
    }

    /// `0x009f3e50` (thiscall `(n)`, `ret 4`): the world cursor moved on
    /// by `n` entries, block by block: each block takes what is left of it
    /// (`(end - start) / stride` while `start < end`, else the cursor's
    /// remaining count `+0x18`), at most `n`; stepping `start` by the
    /// stride, the remaining count down, and onto the next block
    /// (`*end`) when `start` reaches `end`; then empty blocks skipped
    /// (`PWorldCursor_skipEmpty`). The last skip's `eax` (0 for `n = 0`).
    /// A zero stride faults in the original (a divide error); the port
    /// takes 0 entries there.
    pub fn cursor_advance(h: &mut impl Host, this: u32, n: u32) -> u32 {
        let mut left = n;
        let mut eax = 0;
        while left != 0 {
            let remaining: u32 = h.read(this + 0x18);
            let start: u32 = h.read(this + 0xc);
            let mut end: u32 = h.read(this + 0x10);
            let mut take = remaining;
            if start < end {
                let stride: u32 = h.read(this + 0x14);
                take = (end - start).checked_div(stride).unwrap_or(0);
            }
            if left < take {
                take = left;
            }
            if take == remaining && end == 0 {
                h.write(this + 0x18, 0u32);
            } else if take != 0 {
                let stride: u32 = h.read(this + 0x14);
                let mut cur = start;
                let mut rem = remaining;
                let mut steps = take;
                'outer: loop {
                    steps -= 1;
                    loop {
                        cur = cur.wrapping_add(stride);
                        rem = rem.wrapping_sub(1);
                        h.write(this + 0xc, cur);
                        h.write(this + 0x18, rem);
                        if end != cur {
                            break;
                        }
                        end = h.read(end);
                        h.write(this + 0x10, end);
                        if rem == 0 {
                            break;
                        }
                    }
                    if steps == 0 {
                        break 'outer;
                    }
                }
            }
            left = left.wrapping_sub(take);
            eax = h.call_sym(CURSOR_SKIP_EMPTY, Conv::Thiscall(this), &[]);
        }
        eax
    }
}
