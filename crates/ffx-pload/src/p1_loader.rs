//! List `p1-load`, continuation round, batch 11: the cluster's instance
//! lists (allocation, deletion, clearing), the namespace mapping, and the
//! asset references' registry (`PAssetReferenceImport`).
//!
//! Layouts found here:
//! - `PCluster`: its instance lists (`+0x1c`, a list head; each list
//!   linked through `+0`/`+4`), its arenas (`+0x24`: next, base `+4`,
//!   size `+0xc`), the instance lists' pool (`+0x28`, a free list:
//!   first free `+0`, block size `+8`), its world (`+0x40`).
//! - `PInstanceList`: its descriptor `+0x2c`, the buffer `+0x34`, the
//!   count `+0x3c`, `+0x40` (when set the list stays), `+0x49` set once
//!   it is in its cluster.
//! - The asset registry: the tree `0x00c9424c` (from `0x0045d420`)
//!   guarded by the critical section `0x00c940c4`
//!   (`0x00431920`/`0x00431a80`).
//! - `PAssetReference` (`0x0045e280`): its class descriptor `+0`, its
//!   name `+4` (bit 0 not owned), the chain of words to fix (`+8`, each
//!   word holding the next one's address).

use ffx_graphic::host::{Conv, Host};

use crate::descriptor::{ALLOC, FREE};
use crate::p1_misc::PRINTF;

/// The asset registry's critical section.
pub const ASSET_LOCK: u32 = 0x00c9_40c4;
/// Its lock (thiscall).
pub const LOCK: u32 = 0x0043_1920;
/// Its unlock (thiscall).
pub const UNLOCK: u32 = 0x0043_1a80;
/// The asset tree's getter (cdecl; its one argument unused).
pub const ASSET_TREE: u32 = 0x0045_d420;

/// The ports over game memory.
pub mod bridge {
    use super::{ALLOC, ASSET_LOCK, ASSET_TREE, Conv, FREE, Host, LOCK, PRINTF, UNLOCK};

    /// An instance list's release, as `deleteObject` and `clear` do it:
    /// out of the world (`0x00443c20`) when there is one, its buffer
    /// (`0x005764a0`), its free list (`0x0049c5b0` on `+8`), back to the
    /// cluster's pool (`0x0049c980`).
    fn release_list(h: &mut impl Host, cluster: u32, list: u32) {
        let world: u32 = h.read(cluster + 0x40);
        if world != 0 {
            h.call_sym(0x0044_3c20, Conv::Thiscall(world), &[list]);
        }
        h.call_sym(0x0057_64a0, Conv::Thiscall(list), &[]);
        h.call_sym(0x0049_c5b0, Conv::Thiscall(list + 8), &[]);
        h.call_sym(0x0049_c980, Conv::Thiscall(cluster + 0x28), &[list]);
    }

    /// Unlinks `node` from its list and leaves it pointing at itself.
    fn unlink(h: &mut impl Host, node: u32) {
        let [next, prev]: [u32; 2] = h.read(node);
        h.write(next + 4, prev);
        h.write(prev, next);
        h.write(node, [node, node]);
    }

    /// `PCluster::allocateInstanceListForSize(cluster, size, n, desc,
    /// extra)` (`0x00440ba0`, thiscall): 0 for no size or no count, for
    /// an abstract class (flags bit 1, reported) or when `n` instances
    /// need more than `size` bytes (reported); else a list from the
    /// pool (`0x0049c8d0` grows it when empty; 0 if still none),
    /// constructed (`0x00576270`, thiscall on it, `(desc, cluster)`),
    /// given `size + extra` bytes of the arena (`0x00440cf0`, the class's
    /// alignment) as its buffer (`0x005763b0`; back to the pool,
    /// `0x0043f3c0`, and 0 when that fails), linked at the cluster's tail
    /// and added to its world (`0x00443320`). Returns the list.
    pub fn allocate_instance_list(
        h: &mut impl Host,
        cluster: u32,
        size: u32,
        n: u32,
        desc: u32,
        extra: u32,
    ) -> u32 {
        if size == 0 || n == 0 {
            return 0;
        }
        let flags: u8 = h.read(desc + 0x90);
        if flags & 2 != 0 {
            let name: u32 = h.read(desc + 0x18);
            let msg = h.rebase(0x00b0_f1a8);
            h.call_sym(PRINTF, Conv::Cdecl, &[1, msg, name]);
            return 0;
        }
        let elem: u32 = h.read(desc + 0x1c);
        let need = elem.wrapping_mul(n);
        if size < need {
            let name: u32 = h.read(desc + 0x18);
            let msg = h.rebase(0x00b0_f1f0);
            h.call_sym(PRINTF, Conv::Cdecl, &[1, msg, n, name, size, need]);
            return 0;
        }
        let pool = cluster + 0x28;
        let mut block: u32 = h.read(pool);
        if block == 0 {
            let grow: u32 = h.read(pool + 8);
            h.call_sym(0x0049_c8d0, Conv::Thiscall(pool), &[grow]);
            block = h.read(pool);
        }
        if block != 0 {
            let next: u32 = h.read(block);
            h.write(pool, next);
        }
        if block == 0 {
            return 0;
        }
        let total = size.wrapping_add(extra);
        let list = h.call_sym(0x0057_6270, Conv::Thiscall(block), &[desc, cluster]);
        let align: u32 = h.read(desc + 0x20);
        let buffer = h.call_sym(0x0044_0cf0, Conv::Thiscall(cluster), &[total, align]);
        let ok = h.call_sym(0x0057_63b0, Conv::Thiscall(list), &[n, total, buffer]);
        if ok.trailing_zeros() >= 8 {
            h.call_sym(0x0043_f3c0, Conv::Thiscall(pool), &[list]);
            return 0;
        }
        h.write(list + 0x49, 1u8);
        let head = cluster + 0x1c;
        let tail: u32 = h.read(cluster + 0x20);
        h.write(list, [head, tail]);
        h.write(tail, list);
        h.write(head + 4, list);
        let world: u32 = h.read(cluster + 0x40);
        if world != 0 {
            h.call_sym(0x0044_3320, Conv::Thiscall(world), &[list]);
        }
        list
    }

    /// `PCluster::deleteObject(cluster, obj)` (`0x00441240`, thiscall):
    /// 0x13 unless one of the cluster's instance lists holds `obj` (its
    /// buffer up to `count` instances); else that list's `freeObject`
    /// (`0x00576530`), whose error is returned; then a list that does
    /// not stay (`+0x40`) and is not in one of the cluster's arenas (or
    /// whose class has flags bit 0) is unlinked and released. 0.
    pub fn delete_object(h: &mut impl Host, cluster: u32, obj: u32) -> u32 {
        let head = cluster + 0x1c;
        let first: u32 = h.read(head);
        let mut list = if first == head { 0 } else { first };
        let (buffer, desc) = loop {
            if list == 0 {
                return 0x13;
            }
            let buffer: u32 = h.read(list + 0x34);
            let desc: u32 = h.read(list + 0x2c);
            if obj >= buffer {
                let elem: u32 = h.read(desc + 0x1c);
                let count: u32 = h.read(list + 0x3c);
                if obj < elem.wrapping_mul(count).wrapping_add(buffer) {
                    break (buffer, desc);
                }
            }
            let next: u32 = h.read(list);
            list = if next == head { 0 } else { next };
        };
        let r = h.call_sym(0x0057_6530, Conv::Thiscall(list), &[obj]);
        if r != 0 {
            return r;
        }
        let mut in_arena = false;
        let mut arena: u32 = h.read(cluster + 0x24);
        while arena != 0 {
            let base: u32 = h.read(arena + 4);
            let size: u32 = h.read(arena + 0xc);
            in_arena |= buffer >= base && buffer < size.wrapping_add(base);
            arena = h.read(arena);
        }
        let flags: u8 = h.read(desc + 0x90);
        let stays: u32 = h.read(list + 0x40);
        if stays == 0 && (flags & 1 != 0 || !in_arena) {
            unlink(h, list);
            release_list(h, cluster, list);
        }
        0
    }

    /// `PClusterBase::clear(cluster)` (`0x00440eb0`, thiscall): every
    /// instance list unlinked and released, except the lists of
    /// `0x00c91180` and `0x00c91330`, which move to a local list (`local`,
    /// two words: the original's frame) at its head and are released
    /// after the others, from its head; then the pool's reset
    /// (`0x0049cc10`) and the arenas freed.
    pub fn clear(h: &mut impl Host, local: u32, cluster: u32) {
        let head = cluster + 0x1c;
        h.write(local, [local, local]);
        let late = [h.rebase(0x00c9_1180), h.rebase(0x00c9_1330)];
        while h.read::<u32>(head) != head {
            let first: u32 = h.read(head);
            let list = if first == head { 0 } else { first };
            let desc: u32 = h.read(list + 0x2c);
            unlink(h, list);
            if late.contains(&desc) {
                let old: u32 = h.read(local);
                h.write(list, [old, local]);
                h.write(local, list);
                h.write(old + 4, list);
            } else {
                release_list(h, cluster, list);
            }
        }
        while h.read::<u32>(local) != local {
            let list: u32 = h.read(local);
            unlink(h, list);
            release_list(h, cluster, list);
        }
        h.call_sym(0x0049_cc10, Conv::Thiscall(cluster + 0x28), &[]);
        loop {
            let arena: u32 = h.read(cluster + 0x24);
            if arena == 0 {
                break;
            }
            let next: u32 = h.read(arena);
            h.write(cluster + 0x24, next);
            h.call_sym(FREE, Conv::Cdecl, &[arena]);
        }
    }

    /// `PNamespaceMapping::PNamespaceMapping(map, ns, a, b, c)`
    /// (`0x004786a0`, thiscall): three arrays sized from the packed
    /// namespace's counts (`+8` entries of 12 bytes; `+0xc` and `+0x10`
    /// of 32, initialised by `0x0047bb00` and `0x0047baa0`), each count
    /// kept only when its block was allocated, then `build`
    /// (`0x0047c8b0`, `(a, 0, b, c)`). Returns `map`.
    pub fn mapping_ctor(h: &mut impl Host, map: u32, args: [u32; 4]) -> u32 {
        let [ns, a, b, c] = args;
        h.write(map, ns);
        let n: u32 = h.read(ns + 8);
        h.write(map + 4, 0u32);
        let block = if n == 0 {
            0
        } else {
            h.call_sym(ALLOC, Conv::Cdecl, &[n.wrapping_mul(12), 4])
        };
        h.write(map + 8, block);
        h.write(map + 4, if block == 0 { 0 } else { n });
        for (count_at, at, init) in [(0xc, 0xc, 0x0047_bb00), (0x10, 0x14, 0x0047_baa0)] {
            let n: u32 = h.read(ns + count_at);
            h.write(map + at, 0u32);
            let mut block = 0;
            if n != 0 {
                block = h.call_sym(ALLOC, Conv::Cdecl, &[n << 5, 4]);
                if block != 0 {
                    h.call_sym(init, Conv::Cdecl, &[block, n]);
                }
            }
            h.write(map + at + 4, block);
            h.write(map + at, if block == 0 { 0 } else { n });
        }
        h.write(map + 0x1c, 0u32);
        h.call_sym(0x0047_c8b0, Conv::Thiscall(map), &[a, 0, b, c]);
        map
    }

    /// `0x0045e1a0` (thiscall on a `PAssetReferenceImport`): 0x15 without
    /// a name (`+0x18`, less bit 0); else its node emptied, its key (the
    /// name's hash `0x0045d520(name, 0x7b5)` and the name), and under the
    /// registry's lock its insert (`0x0045b9d0`): 0x12 for a duplicate,
    /// else `postInsertFinalize` (`0x00574980`) and 0. `local` is the
    /// word the original hands the tree's getter.
    pub fn import_register(h: &mut impl Host, local: u32, import: u32) -> u32 {
        let word: u32 = h.read(import + 0x18);
        let name = word.wrapping_sub(word & 1);
        if name == 0 {
            return 0x15;
        }
        let node = import + 4;
        h.write(node, [node, node, node]);
        let hash = h.call_sym(0x0045_d520, Conv::Cdecl, &[name, 0x7b5]);
        h.write(import + 0x10, [hash, name]);
        let lock = h.rebase(ASSET_LOCK);
        h.write(local, lock);
        h.call_sym(LOCK, Conv::Thiscall(lock), &[]);
        let tree = h.call_sym(ASSET_TREE, Conv::Cdecl, &[local]);
        let r = if h.call_sym(0x0045_b9d0, Conv::Thiscall(tree), &[import]) == 0 {
            h.call_sym(0x0057_4980, Conv::Thiscall(tree), &[node]);
            0
        } else {
            0x12
        };
        h.call_sym(UNLOCK, Conv::Thiscall(lock), &[]);
        r
    }

    /// `PAssetReferenceImport::resolve(reference)` (`0x0045e280`,
    /// thiscall): nothing without a chain (`+8`); else, under the
    /// registry's lock, the import of that name (`key`, two words: the
    /// name's hash and the name, for `0x0045b8f0`); none: a warning and
    /// the class's default instance (`0x0043cbb0`); one of another class
    /// (its `+0x20` and parents): a warning, its object anyway (`+0x1c`).
    /// Every word of the chain gets the object less the class's `+0x88`;
    /// the chain is cleared. 0.
    pub fn resolve(h: &mut impl Host, key: u32, reference: u32) -> u32 {
        let mut link: u32 = h.read(reference + 8);
        if link == 0 {
            return 0;
        }
        let class: u32 = h.read(reference);
        let word: u32 = h.read(reference + 4);
        let name = word.wrapping_sub(word & 1);
        let lock = h.rebase(ASSET_LOCK);
        h.call_sym(LOCK, Conv::Thiscall(lock), &[]);
        let tree = h.call_sym(ASSET_TREE, Conv::Cdecl, &[key + 4]);
        h.write(key + 4, name);
        let hash = h.call_sym(0x0045_d520, Conv::Cdecl, &[name, 0x7b5]);
        h.write(key, hash);
        let found = h.call_sym(0x0045_b8f0, Conv::Thiscall(tree), &[key]);
        h.write(key + 4, found);
        h.call_sym(UNLOCK, Conv::Thiscall(lock), &[]);
        let object = if found == 0 {
            let word: u32 = h.read(reference + 4);
            let cname: u32 = h.read(class + 0x18);
            let msg = h.rebase(0x00b1_1ce8);
            h.call_sym(
                PRINTF,
                Conv::Cdecl,
                &[2, msg, word.wrapping_sub(word & 1), cname],
            );
            h.call_sym(0x0043_cbb0, Conv::Thiscall(class), &[])
        } else {
            let got: u32 = h.read(found + 0x20);
            let mut c = got;
            while c != 0 && c != class {
                c = h.read(c + 0x40);
            }
            if c == 0 {
                let gname: u32 = h.read(got + 0x18);
                let cname: u32 = h.read(class + 0x18);
                let word: u32 = h.read(reference + 4);
                let msg = h.rebase(0x00b1_1d1c);
                h.call_sym(
                    PRINTF,
                    Conv::Cdecl,
                    &[2, msg, word.wrapping_sub(word & 1), cname, gname],
                );
            }
            let found: u32 = h.read(key + 4);
            h.read(found + 0x1c)
        };
        let base: u32 = h.read(class + 0x88);
        let value = object.wrapping_sub(base);
        loop {
            let next: u32 = h.read(link);
            h.write(link, value);
            link = next;
            if next == 0 {
                break;
            }
        }
        h.write(reference + 8, 0u32);
        0
    }

    /// `PAssetReferenceImport`'s destructor (`0x0045c0f0`, thiscall):
    /// when it has a key (`+0x14`) and is in the tree (its parent word
    /// `+0xc`, less bit 0, is not its own node), out of the tree
    /// (`0x00575000`) under the registry's lock; then its name (`+0x18`)
    /// freed unless bit 0 says it is not owned. `local` is the word the
    /// original hands the tree's getter.
    pub fn import_dtor(h: &mut impl Host, local: u32, import: u32) {
        let key: u32 = h.read(import + 0x14);
        let parent: u32 = h.read(import + 0xc);
        let node = import + 4;
        if key != 0 && parent.wrapping_sub(parent & 1) != node {
            let lock = h.rebase(ASSET_LOCK);
            h.write(local, lock);
            h.call_sym(LOCK, Conv::Thiscall(lock), &[]);
            let tree = h.call_sym(ASSET_TREE, Conv::Cdecl, &[local]);
            h.call_sym(0x0057_5000, Conv::Thiscall(tree), &[node]);
            h.call_sym(UNLOCK, Conv::Thiscall(lock), &[]);
        }
        let name: u32 = h.read(import + 0x18);
        if name & 1 == 0 {
            h.call_sym(FREE, Conv::Cdecl, &[name]);
        }
    }
}
