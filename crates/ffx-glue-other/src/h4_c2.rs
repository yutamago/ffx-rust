//! Continuation round of list `h4-glue-other`, batch 2: the cluster
//! manager's releases and its map's range erase, the FMOD bank's
//! destructor and its tree's, the two id-array tree inserts, the named
//! pair append, `FmodSfx`'s pause and stop-all walks, and the
//! `PArray<PInputAction>` class descriptor's constants.
//!
//! Bridges that hand callees the address of their own argument word take
//! `args`, its address (as in [`crate::r3`]); those that hand callees a
//! local take `frame`.

use ffx_graphic::host::{Conv, Host};

use crate::frames::{ERRCHECK, OPERATOR_DELETE, PRINT_TO_OUTPUT};
use crate::menu::{PHYRE_FREE, RELEASE_MUTEX, WAIT_FOR_SINGLE_OBJECT};
use crate::r3::DJB_HASH;

// ---- the cluster manager

/// `Phyre_PCluster_dtor_PCluster(this)` (thiscall).
pub const PCLUSTER_DTOR: u32 = 0x0043_fa30;
/// `ClusterSlots_give_0069ae20(this; p)` (thiscall, ret 4).
pub const CLUSTER_SLOTS_GIVE: u32 = 0x0069_ae20;
/// `ClusterMap_erase_0069b010(map; &out, node)` (thiscall, ret 8).
pub const CLUSTER_MAP_ERASE: u32 = 0x0069_b010;
/// `ClusterMap_clear_0069a970(map)` (thiscall).
pub const CLUSTER_MAP_CLEAR: u32 = 0x0069_a970;

fn nil(h: &impl Host, node: u32) -> bool {
    h.read::<u8>(node.wrapping_add(0xd)) != 0
}

/// The in-order successor of `node` in an MSVC `std::map` (`+0` left,
/// `+4` parent, `+8` right, `+0xd` nil); a nil node stays.
fn successor(h: &impl Host, node: u32) -> u32 {
    if nil(h, node) {
        return node;
    }
    let right: u32 = h.read(node.wrapping_add(8));
    if nil(h, right) {
        let mut child = node;
        let mut parent: u32 = h.read(node.wrapping_add(4));
        while !nil(h, parent) && child == h.read::<u32>(parent.wrapping_add(8)) {
            child = parent;
            parent = h.read(parent.wrapping_add(4));
        }
        parent
    } else {
        let mut node = right;
        loop {
            let left: u32 = h.read(node);
            if nil(h, left) {
                return node;
            }
            node = left;
        }
    }
}

fn lock(h: &mut impl Host, this: u32) {
    let holder: u32 = h.read(this + 0x18);
    let mutex: u32 = h.read(holder);
    let wait = WAIT_FOR_SINGLE_OBJECT.get(h);
    h.call(wait, Conv::Stdcall, &[mutex, u32::MAX]);
}

fn unlock(h: &mut impl Host, this: u32) -> u32 {
    let holder: u32 = h.read(this + 0x18);
    let mutex: u32 = h.read(holder);
    let release = RELEASE_MUTEX.get(h);
    h.call(release, Conv::Stdcall, &[mutex])
}

/// The node of `hash` in the cluster map at `this + 0x10` (key `+0x10`):
/// the lower bound, the head when it is not an exact match.
fn find_hash(h: &impl Host, this: u32, hash: u32) -> u32 {
    let head: u32 = h.read(this + 0x10);
    let mut bound = head;
    let mut node: u32 = h.read(head + 4);
    while !nil(h, node) {
        if h.read::<u32>(node + 0x10) < hash {
            node = h.read(node + 8);
        } else {
            bound = node;
            node = h.read(node);
        }
    }
    if bound == head || hash < h.read::<u32>(bound + 0x10) {
        head
    } else {
        bound
    }
}

/// Frees the entry of map node `node` (`+0x14`): its cluster `cluster`
/// destroyed and freed when set, the entry given back to the slots, the
/// node erased (`ClusterMap_erase_0069b010(this + 0x10; args, node)`, the
/// iterator landing in the argument slot).
fn drop_entry(h: &mut impl Host, this: u32, args: u32, node: u32, cluster: u32) {
    if cluster != 0 {
        h.call_sym(PCLUSTER_DTOR, Conv::Thiscall(cluster), &[]);
        h.call_sym(PHYRE_FREE, Conv::Cdecl, &[cluster]);
    }
    let entry: u32 = h.read(node + 0x14);
    h.call_sym(CLUSTER_SLOTS_GIVE, Conv::Thiscall(this), &[entry]);
    h.call_sym(
        CLUSTER_MAP_ERASE,
        Conv::Thiscall(this + 0x10),
        &[args, node],
    );
}

/// `ClusterManager_forceReleasePCluster(this; name)` (`0x0069b310`,
/// thiscall, ret 4; `args` the address of its `name` slot): under the
/// mutex, the map node of the name's `DJB_hash` (kept in the `name`
/// slot); unless the entry is locked (byte `+0x14`), its cluster (`+0`)
/// destroyed and freed, the entry given back, the node erased. Returns
/// `ReleaseMutex`'s `eax`.
pub fn force_release_pcluster(h: &mut impl Host, this: u32, args: u32) -> u32 {
    let name: u32 = h.read(args);
    let hash = h.call_sym(DJB_HASH, Conv::Cdecl, &[name]);
    lock(h, this);
    let head: u32 = h.read(this + 0x10);
    let it = find_hash(h, this, hash);
    h.write(args, it);
    let node: u32 = h.read(args);
    if node != head {
        let entry: u32 = h.read(node + 0x14);
        if h.read::<u8>(entry + 0x14) == 0 {
            let cluster: u32 = h.read(entry);
            drop_entry(h, this, args, node, cluster);
        }
    }
    unlock(h, this)
}

/// `ClusterManager_releasePCluster_0069be90(this; name)` (thiscall, ret
/// 4; `args` as [`force_release_pcluster`]): the entry of the name's hash,
/// unless locked, loses a reference (`+4`); with none left (signed) and no
/// pending load (`+0xc`) it is dropped as in [`force_release_pcluster`].
/// Returns `ReleaseMutex`'s `eax`.
pub fn release_pcluster_by_name(h: &mut impl Host, this: u32, args: u32) -> u32 {
    let name: u32 = h.read(args);
    let hash = h.call_sym(DJB_HASH, Conv::Cdecl, &[name]);
    lock(h, this);
    let head: u32 = h.read(this + 0x10);
    let it = find_hash(h, this, hash);
    h.write(args, it);
    let node: u32 = h.read(args);
    if node != head {
        let entry: u32 = h.read(node + 0x14);
        if h.read::<u8>(entry + 0x14) == 0 {
            let refs: u32 = h.read(entry + 4);
            h.write(entry + 4, refs.wrapping_sub(1));
            let entry: u32 = h.read(node + 0x14);
            if h.read::<u32>(entry + 4).cast_signed() <= 0 && h.read::<u32>(entry + 0xc) == 0 {
                let cluster: u32 = h.read(entry);
                drop_entry(h, this, args, node, cluster);
            }
        }
    }
    unlock(h, this)
}

/// `ClusterManager_releasePCluster(this; cluster)` (`0x0069bdb0`,
/// thiscall, ret 4; `args` the address of its `cluster` slot): under the
/// mutex, the first map entry (in order) holding `cluster` (`+0`); unless
/// locked it loses a reference, and with none left (signed) and no pending
/// load `cluster` is destroyed and freed (when set), the entry given back
/// and the node erased (the iterator lands in the `cluster` slot).
/// Returns `ReleaseMutex`'s `eax`.
pub fn release_pcluster(h: &mut impl Host, this: u32, args: u32) -> u32 {
    lock(h, this);
    let head: u32 = h.read(this + 0x10);
    let mut node: u32 = h.read(head);
    let cluster: u32 = h.read(args);
    while node != head {
        let entry: u32 = h.read(node + 0x14);
        if h.read::<u32>(entry) == cluster {
            if h.read::<u8>(entry + 0x14) == 0 {
                let refs: u32 = h.read(entry + 4);
                h.write(entry + 4, refs.wrapping_sub(1));
                let entry: u32 = h.read(node + 0x14);
                if h.read::<u32>(entry + 4).cast_signed() < 1 && h.read::<u32>(entry + 0xc) == 0 {
                    drop_entry(h, this, args, node, cluster);
                }
            }
            break;
        }
        node = successor(h, node);
    }
    unlock(h, this)
}

/// `ClusterMap_eraseRange_0069af70(this; out, first, last)` (thiscall,
/// ret 0xc; `frame` the word at `ebp - 4` the erase writes): the whole map
/// (`first` the head's leftmost, `last` the head) is cleared
/// (`ClusterMap_clear_0069a970`) and `*out` = the head's leftmost;
/// otherwise each node from `first` to `last` erased
/// (`ClusterMap_erase_0069b010(this; frame, node)`, its successor taken
/// first; a nil node stays) and `*out = last`. Returns `out`. The local
/// starts as `this` (the prologue's `push ecx`).
pub fn cluster_map_erase_range(
    h: &mut impl Host,
    frame: u32,
    this: u32,
    [out, first, last]: [u32; 3],
) -> u32 {
    // the local is the `push ecx` slot: it starts as `this`
    h.write(frame, this);
    let head: u32 = h.read(this);
    if first == h.read::<u32>(head) && last == head {
        h.call_sym(CLUSTER_MAP_CLEAR, Conv::Thiscall(this), &[]);
        let head: u32 = h.read(this);
        let begin: u32 = h.read(head);
        h.write(out, begin);
        return out;
    }
    let mut at = first;
    while at != last {
        let next = successor(h, at);
        h.call_sym(CLUSTER_MAP_ERASE, Conv::Thiscall(this), &[frame, at]);
        at = next;
    }
    h.write(out, at);
    out
}

// ---- named pairs

/// `CopyNamedPairs_006991f0(dst, src, n)` (cdecl).
pub const COPY_NAMED_PAIRS: u32 = 0x0069_91f0;
/// The named pair's copy constructor `0x00699b70(this; &src)` (thiscall).
pub const NAMED_PAIR_COPY: u32 = 0x0069_9b70;
/// `FreeOwnedArray_0069a130(items, n)` (cdecl).
pub const FREE_OWNED_ARRAY: u32 = 0x0069_a130;

/// `PArrayNamedPair_appendInto_0069b6e0(this; dst, &value)` (thiscall,
/// ret 8): `dst` 0 gives 0xd; else the count's (`+0`, top bit not owned)
/// pairs of `+4` copied into `dst` (`CopyNamedPairs_006991f0`), `value`
/// copy-constructed after them (unless that address is 0), the old storage
/// freed with its pairs' names (`FreeOwnedArray_0069a130`) unless it is
/// `dst` or not owned; `+0` = count + 1 (top bit dropped), `+4 = dst`.
/// Returns 0. The SEH frame is not kept.
pub fn named_pair_append_into(h: &mut impl Host, this: u32, dst: u32, value: u32) -> u32 {
    if dst == 0 {
        return 0xd;
    }
    let n = h.read::<u32>(this) & 0x7fff_ffff;
    let src: u32 = h.read(this + 4);
    h.call_sym(COPY_NAMED_PAIRS, Conv::Cdecl, &[dst, src, n]);
    let end = dst.wrapping_add(n.wrapping_mul(8));
    if end != 0 {
        h.call_sym(NAMED_PAIR_COPY, Conv::Thiscall(end), &[value]);
    }
    let old: u32 = h.read(this + 4);
    if old != dst {
        let word: u32 = h.read(this);
        if word >> 31 == 0 {
            h.call_sym(FREE_OWNED_ARRAY, Conv::Cdecl, &[old, word & 0x7fff_ffff]);
        }
    }
    h.write(this, n + 1);
    h.write(this + 4, dst);
    0
}

// ---- FMOD banks

/// `printToOutput`'s prefix and format for the bank's release
/// (`0x00b4f190`, `0x00b4fe64`).
pub const BANK_RELEASE_TEXT: [u32; 2] = [0x00b4_f190, 0x00b4_fe64];
/// The file name `SfxBank`'s destructor passes to `ERRCHECK` (`0x00b4fe84`).
pub const BANK_FILE: u32 = 0x00b4_fe84;
/// `SleepThreadInMillisecond(ms)` (cdecl).
pub const SLEEP_MS: u32 = 0x006f_2d20;
/// `SfxEventMap_dtor_00710180(this)` (thiscall).
pub const EVENT_MAP_DTOR: u32 = 0x0071_0180;
/// `SfxBank_destroyEvents_00710960(this)` (thiscall).
pub const DESTROY_EVENTS: u32 = 0x0071_0960;

/// `SfxBank_dtor_SfxBank(this)` (`0x0070f930`, thiscall): with a project
/// (`+0`), a print, its virtual `+0x28` (stdcall, the project) checked
/// (`ERRCHECK(r, file, 0x48)`), 100 ms of sleep, its virtual `+0`
/// checked (line 0x4c); then the project, the event map `+4`
/// (`SfxEventMap_dtor_00710180` and `operator_delete`) and the event
/// list `+8` (`SfxBank_destroyEvents_00710960` and `operator_delete`)
/// cleared. Callers read no result.
pub fn sfx_bank_dtor(h: &mut impl Host, this: u32) {
    let project: u32 = h.read(this);
    if project == 0 {
        return;
    }
    let [prefix, fmt] = BANK_RELEASE_TEXT.map(|a| h.rebase(a));
    h.call_sym(PRINT_TO_OUTPUT, Conv::Cdecl, &[prefix, fmt, this]);
    let file = h.rebase(BANK_FILE);
    for (slot, line) in [(0x28, 0x48), (0, 0x4c)] {
        let project: u32 = h.read(this);
        let vtable: u32 = h.read(project);
        let f: u32 = h.read(vtable.wrapping_add(slot));
        let r = h.call(f, Conv::Stdcall, &[project]);
        h.call_sym(ERRCHECK, Conv::Cdecl, &[r, file, line]);
        if line == 0x48 {
            h.call_sym(SLEEP_MS, Conv::Cdecl, &[100]);
        }
    }
    let map: u32 = h.read(this + 4);
    h.write(this, 0u32);
    if map != 0 {
        h.call_sym(EVENT_MAP_DTOR, Conv::Thiscall(map), &[]);
        h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[map]);
    }
    let list: u32 = h.read(this + 8);
    h.write(this + 4, 0u32);
    if list != 0 {
        h.call_sym(DESTROY_EVENTS, Conv::Thiscall(list), &[]);
        h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[list]);
    }
    h.write(this + 8, 0u32);
}

/// `Phyre_PRedBlackTree_orphanAllContents(this)` (thiscall).
pub const TREE_ORPHAN_ALL: u32 = 0x0057_4970;
/// `Phyre_PFreeList_reset(this)` (thiscall).
pub const FREE_LIST_RESET: u32 = 0x0049_cc10;
/// `Phyre_PFreeList_dtor_PFreeList(this)` (thiscall).
pub const FREE_LIST_DTOR: u32 = 0x0049_c5b0;
/// `Phyre_PRedBlackTree_clear(this)` (thiscall).
pub const TREE_CLEAR: u32 = 0x0057_4760;

/// `SfxEventMap_dtor_00710180(this)` (thiscall): `+0` cleared, then the
/// tree at `+4` destroyed as its inlined destructors do: orphaned, its
/// free list (`+0x1c`) reset, both again, the free list destroyed, the
/// tree cleared twice. The SEH frame is not kept; callers read no result.
pub fn sfx_event_map_dtor(h: &mut impl Host, this: u32) {
    let tree = this + 4;
    h.write(this, 0u32);
    for _ in 0..2 {
        h.call_sym(TREE_ORPHAN_ALL, Conv::Thiscall(tree), &[]);
        h.call_sym(FREE_LIST_RESET, Conv::Thiscall(tree + 0x18), &[]);
    }
    h.call_sym(FREE_LIST_DTOR, Conv::Thiscall(tree + 0x18), &[]);
    h.call_sym(TREE_CLEAR, Conv::Thiscall(tree), &[]);
    h.call_sym(TREE_CLEAR, Conv::Thiscall(tree), &[]);
}

/// `Phyre_PFreeList_allocateNewBlock(this; n)` (thiscall).
pub const FREE_LIST_ALLOCATE_BLOCK: u32 = 0x0049_c8d0;
/// `Phyre_PFreeList_deallocate(this; p)` (thiscall).
pub const FREE_LIST_DEALLOCATE: u32 = 0x0049_c980;
/// `Phyre_PRedBlackTree_postInsertFinalize(this; node)` (thiscall).
pub const TREE_POST_INSERT: u32 = 0x0057_4980;
/// `user_free_thunk(p)` (cdecl).
pub const USER_FREE_THUNK: u32 = 0x0068_7010;
/// `SfxIdArray_assign_0070c5c0` and `vfxIntTree_Insert_0070bba0`, the
/// callees of `SfxIdTree_insertArray_0070cde0`.
pub const SFX_ID_TREE: [u32; 2] = [0x0070_c5c0, 0x0070_bba0];
/// `IdArray_assign_007109e0` and `vfxIntTree_Insert_00710540`, the callees
/// of `IdTree_insertArray_00710ba0`.
pub const ID_TREE: [u32; 2] = [0x0071_09e0, 0x0071_0540];

/// `SfxIdTree_insertArray_0070cde0` and `IdTree_insertArray_00710ba0`
/// (`this; &key, &array`, thiscall, ret 8; `PRedBlackTree_insertArray_0065fca0`
/// with other callees, `[assign, insert]`): a node from the free list
/// `+0x18` (refilled by `allocateNewBlock(+0x20)`; 0xe when none), linked
/// to itself, the key at `+0xc`, an empty id array at `+0x10` assigned from
/// `array`; inserted: a duplicate frees the array's storage and the node
/// (0x12), else the tree is rebalanced (0).
pub fn id_tree_insert_array(
    h: &mut impl Host,
    [assign, insert]: [u32; 2],
    this: u32,
    key: u32,
    array: u32,
) -> u32 {
    let list = this + 0x18;
    let mut node: u32 = h.read(list);
    if node == 0 {
        let n: u32 = h.read(this + 0x20);
        h.call_sym(FREE_LIST_ALLOCATE_BLOCK, Conv::Thiscall(list), &[n]);
        node = h.read(list);
        if node == 0 {
            return 0xe;
        }
    }
    let next: u32 = h.read(node);
    h.write(list, next);
    h.write(node, node);
    h.write(node + 4, node);
    h.write(node + 8, node);
    let k: u32 = h.read(key);
    h.write(node + 0xc, k);
    for off in [0x10, 0x14, 0x18] {
        h.write(node + off, 0u32);
    }
    h.call_sym(assign, Conv::Thiscall(node + 0x10), &[array]);
    if h.call_sym(insert, Conv::Thiscall(this), &[node]) != 0 {
        let data: u32 = h.read(node + 0x18);
        if data != 0 {
            h.call_sym(USER_FREE_THUNK, Conv::Cdecl, &[data]);
        }
        h.call_sym(FREE_LIST_DEALLOCATE, Conv::Thiscall(list), &[node]);
        return 0x12;
    }
    h.call_sym(TREE_POST_INSERT, Conv::Thiscall(this), &[node]);
    0
}

// ---- FmodSfx walks

/// `Phyre_PRedBlackTree_successor(tree; node)` (thiscall, ret 4).
pub const TREE_SUCCESSOR: u32 = 0x0057_5160;
/// `SfxEvent_stop(event; immediate)` (thiscall, ret 4).
pub const SFX_EVENT_STOP: u32 = 0x0070_f490;
/// `SfxEvent_setPaused(event; paused)` (thiscall, ret 4).
pub const SFX_EVENT_SET_PAUSED: u32 = 0x0070_f100;

/// The first node of the `PRedBlackTree` at `tree` (`+0xc` its leftmost
/// entry, `+0` the next link down, the tree itself the end): the last
/// before the tree on that chain; the tree when empty, 0 for a null link.
pub(crate) fn tree_begin(h: &impl Host, tree: u32) -> u32 {
    let mut node: u32 = h.read(tree + 0xc);
    if node == tree || node == 0 {
        return node;
    }
    loop {
        let next: u32 = h.read(node);
        if next == tree {
            return node;
        }
        node = next;
    }
}

/// Walks the sound tree at `this + 0x64` in order (`successor` taken one
/// node ahead, as the originals do), calling `visit` on each node.
fn walk_sounds<H: Host>(h: &mut H, this: u32, visit: impl FnMut(&mut H, u32)) {
    walk_tree(h, this + 0x64, visit);
}

/// Walks the `PRedBlackTree` at `tree` in order (`successor` taken one
/// node ahead, as the originals do), calling `visit` on each node.
pub(crate) fn walk_tree<H: Host>(h: &mut H, tree: u32, mut visit: impl FnMut(&mut H, u32)) {
    let mut node = tree_begin(h, tree);
    if node == tree {
        return;
    }
    let mut next = h.call_sym(TREE_SUCCESSOR, Conv::Thiscall(tree), &[node]);
    while node != tree {
        visit(h, node);
        node = next;
        if next != tree {
            next = h.call_sym(TREE_SUCCESSOR, Conv::Thiscall(tree), &[next]);
        }
    }
}

/// `FmodSfx_stopAllNonBlockingSound(this; keep, immediate)`
/// (`0x0070d930`, thiscall, ret 8): every sound of the tree at `+0x64`
/// whose id (`+0xc`) is above 0x7f (signed) and not in the -1-terminated
/// list `keep` has its events (`+0x18`, count `+0x10`; null ones
/// skipped) stopped (`SfxEvent_stop(event; immediate)`). Its one caller
/// (`FfxFmod_sfxStopAllNonBlockingSound`) reads no result.
pub fn stop_all_non_blocking(h: &mut impl Host, this: u32, keep: u32, immediate: u32) {
    walk_sounds(h, this, |h, node| {
        let id: u32 = h.read(node + 0xc);
        if id.cast_signed() <= 0x7f {
            return;
        }
        let mut at = keep;
        loop {
            let k: u32 = h.read(at);
            if k == u32::MAX {
                break;
            }
            if id == k {
                return;
            }
            at = at.wrapping_add(4);
        }
        let mut i = 0u32;
        while i < h.read::<u32>(node + 0x10) {
            let events: u32 = h.read(node + 0x18);
            let event: u32 = h.read(events.wrapping_add(4 * i));
            if event != 0 {
                h.call_sym(SFX_EVENT_STOP, Conv::Thiscall(event), &[immediate]);
            }
            i += 1;
        }
    });
}

/// `FmodSfx_pause(this; paused)` (`0x0070d0c0`, thiscall, ret 4): a new
/// pause byte (`+0`, the low byte of `paused`) is stored and handed to
/// every event of every sound of the tree at `+0x64`
/// (`SfxEvent_setPaused(event; byte)`, the byte read again each time),
/// then to the stream event `+0xc8` when set. Its one caller
/// (`FfxFmod_sfxPause`) reads no result.
pub fn sfx_pause(h: &mut impl Host, this: u32, paused: u32) {
    let byte = paused as u8;
    if h.read::<u8>(this) == byte {
        return;
    }
    h.write(this, byte);
    walk_sounds(h, this, |h, node| {
        let mut i = 0u32;
        while i < h.read::<u32>(node + 0x10) {
            let events: u32 = h.read(node + 0x18);
            let event: u32 = h.read(events.wrapping_add(4 * i));
            if event != 0 {
                let b = u32::from(h.read::<u8>(this));
                h.call_sym(SFX_EVENT_SET_PAUSED, Conv::Thiscall(event), &[b]);
            }
            i += 1;
        }
    });
    let stream: u32 = h.read(this + 0xc8);
    if stream != 0 {
        let b = u32::from(h.read::<u8>(this));
        h.call_sym(SFX_EVENT_SET_PAUSED, Conv::Thiscall(stream), &[b]);
    }
}

// ---- class descriptors

/// The constants of the `PArray<PInputs::PInputAction>` class
/// descriptor's constructor (`0x009dfb30`), in the order
/// `ffx_prender::registry::bridge::descriptor_ctor` takes them: the name
/// global, the vtable, the default, validation and write-mask buffers.
pub const INPUT_ACTION_ARRAY_DESCRIPTOR: [u32; 5] = [
    0x0194_293c,
    0x00b7_3a74,
    0x0194_2970,
    0x0194_2980,
    0x0194_2990,
];
