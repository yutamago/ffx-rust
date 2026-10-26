//! Continuation round 6 of list `h4-glue-other`, batch 3: name lookups
//! through `std::string` keys (the loaded-character table's
//! `getPrototypeByFilename` and `insertPrototype`, an Iggy callback's
//! lookup in the global name map) and the cluster manager's load path
//! (`loadPCluster`, the character resource cache's `onLoadPCluster`).
//!
//! The `std::string`s are the originals' locals (MSVC layout: a 16-byte
//! buffer or a pointer at `+0`, the length `+0x10`, the capacity
//! `+0x14`; heap storage from capacity 0x10): each function takes its
//! frame, laid out at the original's `ebp` offsets, because the callees
//! get pointers into it. `std::string::assign` (`0x00404270`), the tree
//! lookups and `std_string_compare_006590d0` stay original.

use ffx_bridge::Global;
use ffx_graphic::host::{Conv, Host};

/// `std::string::assign(this; text, n)` (thiscall, ret 8).
pub const STRING_ASSIGN: u32 = 0x0040_4270;
/// `std_string_compare_006590d0(this; pos, n, text, len)` (thiscall, ret
/// 0x10).
pub const STRING_COMPARE: u32 = 0x0065_90d0;
/// `operator_delete(block)` (cdecl).
pub const OPERATOR_DELETE: u32 = 0x0063_0390;

/// `strlen` of the C string at `s`, as the originals' inline loops (a
/// first-byte test, then the scan).
fn strlen(h: &impl Host, s: u32) -> u32 {
    let mut n = 0u32;
    while h.read::<u8>(s.wrapping_add(n)) != 0 {
        n = n.wrapping_add(1);
    }
    n
}

/// An empty `std::string` at `s` (capacity 15, length 0, first byte 0)
/// assigned `text`. Returns the assign's `eax`.
fn string_from(h: &mut impl Host, s: u32, text: u32) -> u32 {
    h.write(s.wrapping_add(0x14), 15u32);
    h.write(s.wrapping_add(0x10), 0u32);
    h.write(s, 0u8);
    let n = strlen(h, text);
    h.call_sym(STRING_ASSIGN, Conv::Thiscall(s), &[text, n])
}

/// The `std::string` at `s` freed when on the heap (capacity 0x10 or
/// more). Returns `operator_delete`'s `eax`, or `None` without a call.
fn string_free(h: &mut impl Host, s: u32) -> Option<u32> {
    if h.read::<u32>(s.wrapping_add(0x14)) < 0x10 {
        return None;
    }
    let block: u32 = h.read(s);
    Some(h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[block]))
}

/// The tree lookup of `key` (the `std::string` at `key`) in a map whose
/// head is `head`: `node = lower_bound(key)` (`find`, thiscall on `tree`,
/// ret 4); unless the head, the node's key (`+0x10`, a `std::string`)
/// compared (`key.compare(0, len, node key, node len)`); a negative
/// result or the head gives the head. Returns the node and the last
/// `eax` (the compare's, or the head for the head).
fn find_string(h: &mut impl Host, find: u32, tree: u32, head: u32, key: u32) -> (u32, u32) {
    let node = h.call_sym(find, Conv::Thiscall(tree), &[key]);
    if node == h.read::<u32>(head) {
        let head_node: u32 = h.read(head);
        return (head_node, head_node);
    }
    let text = node.wrapping_add(0x10);
    let text = if h.read::<u32>(node.wrapping_add(0x24)) >= 0x10 {
        h.read(text)
    } else {
        text
    };
    let len: u32 = h.read(node.wrapping_add(0x20));
    let key_len: u32 = h.read(key.wrapping_add(0x10));
    let r = h.call_sym(
        STRING_COMPARE,
        Conv::Thiscall(key),
        &[0, key_len, text, len],
    );
    if r.cast_signed() < 0 {
        let head_node: u32 = h.read(head);
        return (head_node, head_node);
    }
    (node, r)
}

// ---- the loaded-character table

/// `std_Tree_lower_bound_string_006a06f0(table; &key)` (thiscall, ret 4).
pub const TABLE_FIND: u32 = 0x006a_06f0;
/// `ClassLoadedCharacterTable_getPrototypeByFilename(table; name)`.
pub const GET_PROTOTYPE: u32 = 0x006a_2110;
/// `ClassLoadedCharacterTable_makeEntry_0069da20(out, &name, &prototype)`
/// (cdecl).
pub const MAKE_ENTRY: u32 = 0x0069_da20;
/// `0x0069c5c0(table; &entry)` (thiscall, ret 4): a new tree node.
pub const NEW_NODE: u32 = 0x0069_c5c0;
/// `0x0069cd40(table; &out, 0, &node key, node)` (thiscall, ret 0x10): the
/// node's insertion.
pub const INSERT_NODE: u32 = 0x0069_cd40;
/// Bytes of [`get_prototype`]'s frame (from `ebp - 0x2c`): the result
/// slot `+0`, the key string `+4`.
pub const GET_FRAME: u32 = 0x1c;
/// Bytes of [`insert_prototype`]'s frame (from `ebp - 0x4c`): the
/// insertion's out slot `+0`, the prototype `+4`, the entry `+8` (its
/// capacity `+0x1c`), the name string `+0x24`.
pub const INSERT_FRAME: u32 = 0x3c;

/// `ClassLoadedCharacterTable_getPrototypeByFilename(this; name)`
/// (`0x006a2110`, thiscall, ret 4; `frame` from `ebp - 0x2c`): the name
/// as a `std::string`, looked up (`0x006a06f0`, the head `+0`); the
/// found node's prototype (`+0x28`), 0 without one. The key string freed.
pub fn get_prototype(h: &mut impl Host, frame: u32, this: u32, name: u32) -> u32 {
    let key = frame.wrapping_add(4);
    string_from(h, key, name);
    let (node, _) = find_string(h, TABLE_FIND, this, this, key);
    h.write(frame, node);
    let found = if node == h.read::<u32>(this) {
        0
    } else {
        h.read(node.wrapping_add(0x28))
    };
    string_free(h, key);
    found
}

/// `ClassLoadedCharacterTable_insertPrototype(this; name, prototype)`
/// (`0x006a28b0`, thiscall, ret 8; Ghidra's cdecl misses `this`; `frame`
/// from `ebp - 0x4c`): the name as a `std::string`; unless the table has
/// it (`getPrototypeByFilename`), an entry (`makeEntry(&entry, &name,
/// &prototype)`) turned into a node (`0x0069c5c0`) and inserted
/// (`0x0069cd40`), the entry's string freed. The name string freed.
/// Returns the last call's `eax`.
pub fn insert_prototype(
    h: &mut impl Host,
    frame: u32,
    this: u32,
    name: u32,
    prototype: u32,
) -> u32 {
    let slot = frame.wrapping_add(4);
    h.write(slot, prototype);
    let key = frame.wrapping_add(0x24);
    string_from(h, key, name);
    let mut last = h.call_sym(GET_PROTOTYPE, Conv::Thiscall(this), &[name]);
    if last == 0 {
        let entry = frame.wrapping_add(8);
        let made = h.call_sym(MAKE_ENTRY, Conv::Cdecl, &[entry, key, slot]);
        let node = h.call_sym(NEW_NODE, Conv::Thiscall(this), &[made]);
        last = h.call_sym(
            INSERT_NODE,
            Conv::Thiscall(this),
            &[frame, 0, node.wrapping_add(0x10), node],
        );
        if let Some(r) = string_free(h, entry) {
            last = r;
        }
    }
    string_free(h, key).unwrap_or(last)
}

// ---- the Iggy name lookup

/// The global name map (`0x00ce6eb0`: its head node at `+0`).
pub const NAME_MAP: Global<u32> = Global::new(0x00ce_6eb0);
/// `std_Tree_lower_bound_string_006e9920(map; &key)` (thiscall, ret 4).
pub const NAME_MAP_FIND: u32 = 0x006e_9920;
/// `0x006e9870(a, value)` (cdecl): what a found name gets.
pub const NAME_FOUND: u32 = 0x006e_9870;
/// Bytes of [`iggy_lookup`]'s frame (from `ebp - 0x2c`): the result slot
/// `+0`, the key string `+4`.
pub const LOOKUP_FRAME: u32 = 0x1c;

/// `IggyCallback_lookupName_006e9590(a, args)` (cdecl; a callback the
/// Iggy player opens with; `frame` from `ebp - 0x2c`): the name
/// `args + 8` as a `std::string` looked up in the global name map; the
/// string freed and emptied; a found name's value (`+0x28`) handed to
/// `0x006e9870(a, value)`. Returns the last `eax`.
pub fn iggy_lookup(h: &mut impl Host, frame: u32, a: u32, args: u32) -> u32 {
    let key = frame.wrapping_add(4);
    let name: u32 = h.read(args.wrapping_add(8));
    string_from(h, key, name);
    let map = h.rebase(NAME_MAP.sym);
    let (node, mut last) = find_string(h, NAME_MAP_FIND, map, map, key);
    h.write(frame, node);
    if let Some(r) = string_free(h, key) {
        last = r;
    }
    h.write(key.wrapping_add(0x14), 15u32);
    h.write(key.wrapping_add(0x10), 0u32);
    h.write(key, 0u8);
    if node != NAME_MAP.get(h) {
        let value: u32 = h.read(node.wrapping_add(0x28));
        last = h.call_sym(NAME_FOUND, Conv::Cdecl, &[a, value]);
    }
    last
}

// ---- the cluster manager

/// `strstr`'s import slot (cdecl).
pub const STRSTR: Global<u32> = Global::new(0x00b0_c378);
/// `WaitForSingleObject`'s import slot (stdcall, two words).
pub const WAIT_FOR_SINGLE_OBJECT: Global<u32> = Global::new(0x00b0_c0f8);
/// `ReleaseMutex`'s import slot (stdcall, one word).
pub const RELEASE_MUTEX: Global<u32> = Global::new(0x00b0_c1c0);
/// `Phyre_PString_Assign(this; text)` (thiscall, ret 4).
pub const PSTRING_ASSIGN: u32 = 0x0044_5cf0;
/// `Phyre_PhyreMalloc_0042fb80(bytes, align)` (cdecl).
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
/// `Phyre_PhyreFree(block)` (cdecl).
pub const PHYRE_FREE: u32 = 0x0042_fb20;
/// `PArrayNamedPair_appendInto_0069b6e0(array; storage, &pair)`
/// (thiscall, ret 8).
pub const APPEND_PAIR: u32 = 0x0069_b6e0;
/// The patterns of [`on_load_pcluster`]: `".dae"`, `"chr"`, `"/chr/"`,
/// `"PhyreChrLitShader"`, `"PhyreDefaultLitShader"`.
pub const CHR_PATTERNS: [u32; 5] = [
    0x00b4_7d7c,
    0x00b4_7d84,
    0x00b4_7d88,
    0x00b4_7d90,
    0x00b4_7da4,
];
/// The string a character name is compared with (`0x00b8fbeb`, `""`).
pub const EMPTY_NAME: u32 = 0x00b8_fbeb;
/// The pointer to the costume pattern (`0x00c3417c`).
pub const COSTUME_PATTERN: Global<u32> = Global::new(0x00c3_417c);
/// Bytes of [`on_load_pcluster`]'s frame (from `ebp - 0x18`): the pair's
/// key word and its `PString`.
pub const ON_LOAD_FRAME: u32 = 8;

/// `strstr(text, pattern)` through the import slot.
fn strstr(h: &mut impl Host, text: u32, pattern: u32) -> u32 {
    let f = STRSTR.get(h);
    h.call(f, Conv::Cdecl, &[text, pattern])
}

/// Whether the C strings at `a` and `b` differ (the originals' inline
/// `strcmp`, two bytes a step).
fn differs(h: &impl Host, a: u32, b: u32) -> bool {
    let mut k = 0u32;
    loop {
        let x: u8 = h.read(a.wrapping_add(k));
        let y: u8 = h.read(b.wrapping_add(k));
        if x != y {
            return true;
        }
        if x == 0 {
            return false;
        }
        k = k.wrapping_add(1);
    }
}

/// `ChrResourceCacheManager_onLoadPCluster(this; path, hash, record)`
/// (`0x0069bb80`, thiscall, ret 0xc; `frame` from `ebp - 0x18`): a
/// character model (`".dae"` and `"chr"` in the path) whose hash is not
/// in the `(hash, PString)` array `+8`/`+0xc` yet is appended
/// (`PArrayNamedPair_appendInto_0069b6e0` into `PhyreMalloc(8n + 8, 4)`;
/// the pair's string freed unless static). The record's `+8` is 1 for a
/// character path (`"/chr/"`, `"PhyreChrLitShader"`,
/// `"PhyreDefaultLitShader"`), else 0; `+0x10` 1 when that holds, the
/// path is not empty and has the costume pattern. A character path sets
/// `+0` when it is 0. Returns `this` when `+8` is set, else the `+0x10`
/// value (the original's `eax`).
pub fn on_load_pcluster(
    h: &mut impl Host,
    frame: u32,
    this: u32,
    path: u32,
    hash: u32,
    record: u32,
) -> u32 {
    let p = h.rebase(CHR_PATTERNS[0]);
    if strstr(h, path, p) != 0 {
        let p = h.rebase(CHR_PATTERNS[1]);
        if strstr(h, path, p) != 0 && !has_hash(h, this, hash) {
            let pair_name = frame.wrapping_add(4);
            h.write(pair_name, 0u32);
            h.write(frame, hash);
            h.call_sym(PSTRING_ASSIGN, Conv::Thiscall(pair_name), &[path]);
            let array = this.wrapping_add(8);
            let n = h.read::<u32>(array) & 0x7fff_ffff;
            let storage = h.call_sym(
                PHYRE_MALLOC,
                Conv::Cdecl,
                &[n.wrapping_mul(8).wrapping_add(8), 4],
            );
            h.call_sym(APPEND_PAIR, Conv::Thiscall(array), &[storage, frame]);
            let word: u32 = h.read(pair_name);
            if word & 1 == 0 {
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[word]);
            }
        }
    }
    let mut character = 0u32;
    for &pattern in &CHR_PATTERNS[2..] {
        let p = h.rebase(pattern);
        if strstr(h, path, p) != 0 {
            character = 1;
            break;
        }
    }
    h.write(record.wrapping_add(8), character);
    let mut costume = 0u32;
    if h.read::<u32>(record.wrapping_add(8)) != 0 && path != 0 {
        let empty = h.rebase(EMPTY_NAME);
        if differs(h, path, empty) {
            let p = COSTUME_PATTERN.get(h);
            if strstr(h, path, p) != 0 {
                costume = 1;
            }
        }
    }
    h.write(record.wrapping_add(0x10), costume);
    if h.read::<u32>(record.wrapping_add(8)) != 0 {
        if h.read::<u32>(this) == 0 {
            h.write(this, 1u32);
        }
        return this;
    }
    costume
}

/// Whether the `(hash, PString)` array at `this + 8` (count, top bit not
/// owned; block `+0xc`) has `hash`.
fn has_hash(h: &impl Host, this: u32, hash: u32) -> bool {
    let n = h.read::<u32>(this.wrapping_add(8)) & 0x7fff_ffff;
    let mut at: u32 = h.read(this.wrapping_add(0xc));
    let mut found = false;
    for _ in 0..n {
        if h.read::<u32>(at) == hash {
            found = true;
        }
        at = at.wrapping_add(8);
    }
    found
}

/// `fiosUnifyFilename(name, out, n)` (cdecl).
pub const FIOS_UNIFY: u32 = 0x0067_9820;
/// `DJB_hash(text)` (cdecl).
pub const DJB_HASH: u32 = 0x0067_b7d0;
/// `ClusterSlots_take_0069aba0(manager)` (thiscall): a free record.
pub const TAKE_RECORD: u32 = 0x0069_aba0;
/// `ClusterSlots_give_0069ae20(manager; record)` (thiscall, ret 4).
pub const GIVE_RECORD: u32 = 0x0069_ae20;
/// `ChrResourceCacheManager_onLoadPCluster(manager; path, hash, record)`.
pub const ON_LOAD_PCLUSTER: u32 = 0x0069_bb80;
/// `Phyre_PCluster_LoadAssetFile(record, path, hash)` (cdecl).
pub const LOAD_ASSET_FILE: u32 = 0x0044_06c0;
/// `ClusterManager_newEntry_00699300(map; &pair)` (thiscall, ret 4).
pub const NEW_ENTRY: u32 = 0x0069_9300;
/// `0x006995e0(map; &out, 0, &node key, node)` (thiscall, ret 0x10).
pub const INSERT_ENTRY: u32 = 0x0069_95e0;
/// `"/Shaders/"` (`0x00b47d20`).
pub const SHADERS: u32 = 0x00b4_7d20;
/// Bytes of [`load_pcluster`]'s frame (from `ebp - 0x10c`): the pair
/// `(hash, record)` `+0` (its `+4` is first the lookup's node), the
/// unified path `+8` (0x100 bytes).
pub const LOAD_FRAME: u32 = 0x108;

/// The manager's mutex (`[[this + 0x18]]`).
fn mutex(h: &impl Host, this: u32) -> u32 {
    let at: u32 = h.read(this.wrapping_add(0x18));
    h.read(at)
}

/// `ClusterManager_loadPCluster(this; name)` (`0x0069b940`, thiscall, ret
/// 4; `frame` from `ebp - 0x10c`): the name unified into the frame and
/// hashed (`DJB_hash`); under the manager's mutex, the map `+0x10` (a
/// `std::map` by hash: head `[+0x10]`, nodes' key `+0x10`, record
/// `+0x14`) searched. A loaded cluster gets its record's count (`+4`)
/// raised and is returned (record `+0`). Else a record is taken
/// (`0x0069aba0`: `+4` 1, `+8`/`+0xc` 0, byte `+0x14` 1 for a
/// `"/Shaders/"` path), told to the character cache
/// (`onLoadPCluster`), loaded (`Phyre_PCluster_LoadAssetFile`); a failed
/// load gives the record back (`0x0069ae20`) and returns 0, a good one is
/// inserted into the map under the mutex and its cluster returned.
pub fn load_pcluster(h: &mut impl Host, frame: u32, this: u32, name: u32) -> u32 {
    let path = frame.wrapping_add(8);
    h.call_sym(FIOS_UNIFY, Conv::Cdecl, &[name, path, 0x100]);
    let hash = h.call_sym(DJB_HASH, Conv::Cdecl, &[path]);
    let wait = WAIT_FOR_SINGLE_OBJECT.get(h);
    let m = mutex(h, this);
    h.call(wait, Conv::Stdcall, &[m, 0xffff_ffff]);
    let head: u32 = h.read(this.wrapping_add(0x10));
    let mut best = head;
    let mut node: u32 = h.read(head.wrapping_add(4));
    while h.read::<u8>(node.wrapping_add(0xd)) == 0 {
        if h.read::<u32>(node.wrapping_add(0x10)) >= hash {
            best = node;
            node = h.read(node);
        } else {
            node = h.read(node.wrapping_add(8));
        }
    }
    if best == head || hash < h.read::<u32>(best.wrapping_add(0x10)) {
        best = head;
    }
    h.write(frame.wrapping_add(4), best);
    let release = RELEASE_MUTEX.get(h);
    if best != head {
        let record: u32 = h.read(best.wrapping_add(0x14));
        let count: u32 = h.read(record.wrapping_add(4));
        h.write(record.wrapping_add(4), count.wrapping_add(1));
        let record: u32 = h.read(best.wrapping_add(0x14));
        let cluster: u32 = h.read(record);
        let m = mutex(h, this);
        h.call(release, Conv::Stdcall, &[m]);
        return cluster;
    }
    let m = mutex(h, this);
    h.call(release, Conv::Stdcall, &[m]);
    let record = h.call_sym(TAKE_RECORD, Conv::Thiscall(this), &[]);
    h.write(record.wrapping_add(4), 1u32);
    h.write(record.wrapping_add(0xc), 0u32);
    h.write(record.wrapping_add(8), 0u32);
    h.write(record.wrapping_add(0x14), 0u8);
    let shaders = h.rebase(SHADERS);
    if strstr(h, path, shaders) != 0 {
        h.write(record.wrapping_add(0x14), 1u8);
    }
    h.call_sym(
        ON_LOAD_PCLUSTER,
        Conv::Thiscall(this),
        &[path, hash, record],
    );
    let hash2 = h.call_sym(DJB_HASH, Conv::Cdecl, &[path]);
    if h.call_sym(LOAD_ASSET_FILE, Conv::Cdecl, &[record, path, hash2]) != 0 {
        h.call_sym(GIVE_RECORD, Conv::Thiscall(this), &[record]);
        return 0;
    }
    let m = mutex(h, this);
    h.call(wait, Conv::Stdcall, &[m, 0xffff_ffff]);
    h.write(frame, hash);
    h.write(frame.wrapping_add(4), record);
    let map = this.wrapping_add(0x10);
    let node = h.call_sym(NEW_ENTRY, Conv::Thiscall(map), &[frame]);
    h.call_sym(
        INSERT_ENTRY,
        Conv::Thiscall(map),
        &[frame, 0, node.wrapping_add(0x10), node],
    );
    let m = mutex(h, this);
    h.call(release, Conv::Stdcall, &[m]);
    h.read(record)
}
