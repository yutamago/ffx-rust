//! M5 stage 1, list `static-init-d`: the destructors the static
//! initialisers register with `atexit` (the CRT runs them at exit, after
//! `WinMain`), and a few initialisers of the same code range.
//!
//! The shapes and their tables follow `m5_static_init_b`: each family is
//! one bridge over its rows, each address one shim in `ffx-hook`
//! (`ported/m5_static_init_d.rs`) that calls its row. The rows are the
//! original's constants, read off its machine code
//! (`~/ffx-work/m5/port/static-init-d-work/gen.py`). An image address is
//! rebased before use; a plain number goes as it is. The callees stay calls
//! to the original code at their addresses (`call_sym`).
//!
//! SEH frames and stack cookies are left out (the pool-tree destructors
//! set one up): nothing on these paths throws, and the cookie is pushed but
//! never checked.
//!
//! Every shim returns what the original leaves in `eax` where the original
//! sets it; the CRT's exit loop reads none of it.

use ffx_graphic::host::{Conv, Host};

pub use crate::m5_static_init_b::{TailThiscall, tail_thiscall};

/// `PhyreFree(p)` (cdecl).
pub const PHYRE_FREE: u32 = 0x0042_fb20;
/// `operator delete(p)` (cdecl).
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
/// The chain of blocks the `ChainAppend` initialisers link into
/// (`m5_static_init_b::CHAIN_HEAD`).
pub const CHAIN_HEAD: u32 = crate::m5_static_init_b::CHAIN_HEAD;
/// `PRedBlackTree::orphanAllContents()` (thiscall).
pub const RBT_ORPHAN_ALL: u32 = 0x0057_4970;
/// `PRedBlackTree::clear()` (thiscall).
pub const RBT_CLEAR: u32 = 0x0057_4760;
/// `PFreeList::reset()` (thiscall).
pub const FREE_LIST_RESET: u32 = 0x0049_cc10;
/// `PFreeList::~PFreeList()` (thiscall).
pub const FREE_LIST_DTOR: u32 = 0x0049_c5b0;

/// An enum value's destructor (`EnumValueDtor`): the value's base vtable
/// goes back into its first word, then `unlink(obj)` (cdecl, one of six
/// list removals, one per semantic class) takes it out of its class's
/// list. The original returns the removal's `eax` (always 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnumValueDtor {
    pub addr: u32,
    pub obj: u32,
    pub vt: u32,
    pub unlink: u32,
}

/// The `EnumValueDtor` shims' body. The original pushes `obj` before it
/// stores the vtable; the store comes first either way.
pub fn enum_value_dtor(h: &mut impl Host, r: &EnumValueDtor) -> u32 {
    let obj = h.rebase(r.obj);
    let vt = h.rebase(r.vt);
    h.write(obj, vt);
    h.call_sym(r.unlink, Conv::Cdecl, &[obj])
}

/// A chain object's destructor (`ChainUnlink`, the `ChainAppend`
/// initialisers' counterpart): the base vtable `vt` into the object, then
/// the walk of the chain at [`CHAIN_HEAD`] (links at `+4`) for the link that
/// holds the object; when one does, it takes the object's own link and the
/// object's link is cleared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChainUnlink {
    pub addr: u32,
    pub obj: u32,
    pub vt: u32,
}

/// The `ChainUnlink` shims' body. Returns the address of the link the walk
/// ended on (`eax`): the head when the chain is empty.
pub fn chain_unlink(h: &mut impl Host, r: &ChainUnlink) -> u32 {
    let head = h.rebase(CHAIN_HEAD);
    // the original tests the head before the vtable store
    let empty = h.read::<u32>(head) == 0;
    let obj = h.rebase(r.obj);
    let vt = h.rebase(r.vt);
    h.write(obj, vt);
    let mut slot = head;
    if empty {
        return slot;
    }
    loop {
        let cur: u32 = h.read(slot);
        if cur == obj {
            break;
        }
        slot = cur.wrapping_add(4);
        if h.read::<u32>(slot) == 0 {
            break;
        }
    }
    if h.read::<u32>(slot) != 0 {
        let link = obj.wrapping_add(4);
        let next: u32 = h.read(link);
        h.write(slot, next);
        h.write(link, 0u32);
    }
    slot
}

/// A pooled red-black tree's destructor (`PoolTreeDtor`, a global tree with
/// its node free list `pool`, at `tree + 0x18`): `orphanAllContents(tree)`,
/// the pool's `reset` and destructor, then `clear(tree)` twice (the body's
/// and the member's).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PoolTreeDtor {
    pub addr: u32,
    pub tree: u32,
    pub pool: u32,
}

/// The `PoolTreeDtor` shims' body; the last `clear`'s `eax`.
pub fn pool_tree_dtor(h: &mut impl Host, r: &PoolTreeDtor) -> u32 {
    let tree = h.rebase(r.tree);
    let pool = h.rebase(r.pool);
    h.call_sym(RBT_ORPHAN_ALL, Conv::Thiscall(tree), &[]);
    h.call_sym(FREE_LIST_RESET, Conv::Thiscall(pool), &[]);
    h.call_sym(FREE_LIST_DTOR, Conv::Thiscall(pool), &[]);
    h.call_sym(RBT_CLEAR, Conv::Thiscall(tree), &[]);
    h.call_sym(RBT_CLEAR, Conv::Thiscall(tree), &[])
}

/// A global `std::map`'s destructor (`MapDtor`): `erase(map; &out,
/// head->left, head)` (thiscall, `ret 0xc`) empties it, then the head node
/// is deleted. The original also calls its allocator's empty methods on a
/// byte of its frame (`mov eax, ecx; ret` and `ret 4`, seven calls); they
/// change nothing but `eax`, which the next call sets, so the port leaves
/// them out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapDtor {
    pub addr: u32,
    pub map: u32,
    pub erase: u32,
}

/// The `MapDtor` shims' body. `out` is the word of the shim's frame the
/// erase writes its result iterator to (the original's `ebp - 8`).
/// Returns `operator delete`'s `eax`.
pub fn map_dtor(h: &mut impl Host, r: &MapDtor, out: u32) -> u32 {
    let map = h.rebase(r.map);
    let head: u32 = h.read(map);
    let first: u32 = h.read(head);
    h.call_sym(r.erase, Conv::Thiscall(map), &[out, first, head]);
    let head: u32 = h.read(map);
    h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[head])
}

/// A global buffer's destructor (`OwnedBuffer`): `{flags, data}` where bit
/// 31 of `flags` says the data is not owned; an owned, non-null `data` is
/// freed (`PhyreFree`), then both words are cleared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OwnedBuffer {
    pub addr: u32,
    pub obj: u32,
}

/// The `OwnedBuffer` shims' body. Returns the original's `eax`: `0xfe`
/// when the data is not owned (`!(flags >> 31)` in `al`), 0 when there is
/// none, else `PhyreFree`'s.
pub fn owned_buffer(h: &mut impl Host, r: &OwnedBuffer) -> u32 {
    let obj = h.rebase(r.obj);
    let flags: u32 = h.read(obj);
    let data_at = obj.wrapping_add(4);
    let mut eax = (flags >> 31) ^ 0xff;
    if eax & 1 != 0 {
        let data: u32 = h.read(data_at);
        eax = data;
        if data != 0 {
            eax = h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
        }
    }
    h.write(data_at, 0u32);
    h.write(obj, 0u32);
    eax
}

/// The argument of an [`ImportCall`]: the word at an image global, or an
/// image address itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportArg {
    Word(u32),
    Addr(u32),
}

/// A one-word stdcall import through its slot `iat` (`CloseHandle` of a
/// global handle, `DeleteCriticalSection` of a global section).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImportCall {
    pub addr: u32,
    pub iat: u32,
    pub arg: ImportArg,
}

/// The `ImportCall` shims' body; the import's `eax`.
pub fn import_call(h: &mut impl Host, r: &ImportCall) -> u32 {
    let arg = match r.arg {
        ImportArg::Word(g) => {
            let g = h.rebase(g);
            h.read::<u32>(g)
        }
        ImportArg::Addr(a) => h.rebase(a),
    };
    let slot = h.rebase(r.iat);
    let f: u32 = h.read(slot);
    h.call(f, Conv::Stdcall, &[arg])
}

/// A conditional tail-jump on a loaded pointer (`CondTailLoad`): `ecx =
/// [ptr]`; when it is not 0 the original jumps to `target` (thiscall, no
/// stack words), which returns to our caller with its `eax`; otherwise it
/// returns with the caller's `eax` (0 here: the CRT's exit loop reads
/// nothing).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CondTailLoad {
    pub addr: u32,
    pub ptr: u32,
    pub target: u32,
}

/// The `CondTailLoad` shims' body.
pub fn cond_tail_load(h: &mut impl Host, r: &CondTailLoad) -> u32 {
    let ptr = h.rebase(r.ptr);
    let this: u32 = h.read(ptr);
    if this == 0 {
        return 0;
    }
    h.call_sym(r.target, Conv::Thiscall(this), &[])
}

/// A vtable stored into a global object (`StoreVt`): `[obj] = vt`, and
/// nothing else (`eax` is the caller's).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoreVt {
    pub addr: u32,
    pub obj: u32,
    pub vt: u32,
}

/// The `StoreVt` shims' body.
pub fn store_vt(h: &mut impl Host, r: &StoreVt) {
    let obj = h.rebase(r.obj);
    let vt = h.rebase(r.vt);
    h.write(obj, vt);
}

// ---- the tables: generated by static-init-d-work/gen.py ----

/// The enum values' destructors (22).
pub const ENUM_VALUE_DTORS: &[EnumValueDtor] = &[
    EnumValueDtor {
        addr: 0x00b0_8380,
        obj: 0x00c6_8240,
        vt: 0x00b7_3da4,
        unlink: 0x009f_49f0,
    },
    EnumValueDtor {
        addr: 0x00b0_83a0,
        obj: 0x00c6_8234,
        vt: 0x00b7_3da4,
        unlink: 0x009f_49f0,
    },
    EnumValueDtor {
        addr: 0x00b0_83c0,
        obj: 0x00c6_8228,
        vt: 0x00b7_3da4,
        unlink: 0x009f_49f0,
    },
    EnumValueDtor {
        addr: 0x00b0_83e0,
        obj: 0x00c6_82e8,
        vt: 0x00b7_3da4,
        unlink: 0x009f_49f0,
    },
    EnumValueDtor {
        addr: 0x00b0_8400,
        obj: 0x00c6_82f4,
        vt: 0x00b7_3da4,
        unlink: 0x009f_49f0,
    },
    EnumValueDtor {
        addr: 0x00b0_8420,
        obj: 0x00c6_8300,
        vt: 0x00b7_3da4,
        unlink: 0x009f_49f0,
    },
    EnumValueDtor {
        addr: 0x00b0_8440,
        obj: 0x00c6_830c,
        vt: 0x00b7_3da4,
        unlink: 0x009f_49f0,
    },
    EnumValueDtor {
        addr: 0x00b0_8460,
        obj: 0x00c6_8318,
        vt: 0x00b7_3da4,
        unlink: 0x009f_49f0,
    },
    EnumValueDtor {
        addr: 0x00b0_8480,
        obj: 0x00c6_8324,
        vt: 0x00b7_3da4,
        unlink: 0x009f_49f0,
    },
    EnumValueDtor {
        addr: 0x00b0_84a0,
        obj: 0x00c6_82dc,
        vt: 0x00b7_3da4,
        unlink: 0x009f_49f0,
    },
    EnumValueDtor {
        addr: 0x00b0_84c0,
        obj: 0x00c6_82b8,
        vt: 0x00b7_3da4,
        unlink: 0x009f_49f0,
    },
    EnumValueDtor {
        addr: 0x00b0_84e0,
        obj: 0x00c6_82c4,
        vt: 0x00b7_3da4,
        unlink: 0x009f_49f0,
    },
    EnumValueDtor {
        addr: 0x00b0_8500,
        obj: 0x00c6_82d0,
        vt: 0x00b7_3da4,
        unlink: 0x009f_49f0,
    },
    EnumValueDtor {
        addr: 0x00b0_8520,
        obj: 0x00c6_824c,
        vt: 0x00b7_3da4,
        unlink: 0x009f_49f0,
    },
    EnumValueDtor {
        addr: 0x00b0_8540,
        obj: 0x00c6_8258,
        vt: 0x00b7_3da4,
        unlink: 0x009f_49f0,
    },
    EnumValueDtor {
        addr: 0x00b0_8560,
        obj: 0x00c6_8264,
        vt: 0x00b7_3da4,
        unlink: 0x009f_49f0,
    },
    EnumValueDtor {
        addr: 0x00b0_8580,
        obj: 0x00c6_8270,
        vt: 0x00b7_3da4,
        unlink: 0x009f_49f0,
    },
    EnumValueDtor {
        addr: 0x00b0_85a0,
        obj: 0x00c6_827c,
        vt: 0x00b7_3da4,
        unlink: 0x009f_49f0,
    },
    EnumValueDtor {
        addr: 0x00b0_85c0,
        obj: 0x00c6_82a0,
        vt: 0x00b7_3da4,
        unlink: 0x009f_49f0,
    },
    EnumValueDtor {
        addr: 0x00b0_85e0,
        obj: 0x00c6_82ac,
        vt: 0x00b7_3da4,
        unlink: 0x009f_49f0,
    },
    EnumValueDtor {
        addr: 0x00b0_8600,
        obj: 0x00c6_8288,
        vt: 0x00b7_3da4,
        unlink: 0x009f_49f0,
    },
    EnumValueDtor {
        addr: 0x00b0_8620,
        obj: 0x00c6_8294,
        vt: 0x00b7_3da4,
        unlink: 0x009f_49f0,
    },
];

/// The thiscall tail-jumps (14).
pub const TAIL_THISCALLS: &[TailThiscall] = &[
    TailThiscall {
        addr: 0x00b0_73c0,
        obj: 0x00cd_ece0,
        target: 0x004d_bab0,
    },
    TailThiscall {
        addr: 0x00b0_73d0,
        obj: 0x00cd_ecc0,
        target: 0x004d_bab0,
    },
    TailThiscall {
        addr: 0x00b0_73e0,
        obj: 0x00cd_eca0,
        target: 0x004d_bab0,
    },
    TailThiscall {
        addr: 0x00b0_73f0,
        obj: 0x00cd_ed20,
        target: 0x004d_bab0,
    },
    TailThiscall {
        addr: 0x00b0_7430,
        obj: 0x00cd_ed44,
        target: 0x004d_bab0,
    },
    TailThiscall {
        addr: 0x00b0_74d0,
        obj: 0x00cd_ee20,
        target: 0x0043_fa30,
    },
    TailThiscall {
        addr: 0x00b0_7740,
        obj: 0x00ce_6d70,
        target: 0x0043_fa30,
    },
    TailThiscall {
        addr: 0x00b0_78b0,
        obj: 0x00ce_6e94,
        target: 0x004d_bab0,
    },
    TailThiscall {
        addr: 0x00b0_7a00,
        obj: 0x00ce_8208,
        target: 0x0058_5230,
    },
    TailThiscall {
        addr: 0x00b0_7a10,
        obj: 0x00ce_822c,
        target: 0x0058_5230,
    },
    TailThiscall {
        addr: 0x00b0_7d3f,
        obj: 0x0193_ed2c,
        target: 0x0094_a641,
    },
    TailThiscall {
        addr: 0x00b0_7dba,
        obj: 0x0193_fcc4,
        target: 0x009d_9cc1,
    },
    TailThiscall {
        addr: 0x00b0_7dc4,
        obj: 0x0193_fcfc,
        target: 0x009d_a05c,
    },
    TailThiscall {
        addr: 0x00b0_86a0,
        obj: 0x0194_3da0,
        target: 0x0043_fa30,
    },
];

/// The chain objects' destructors (3).
pub const CHAIN_UNLINKS: &[ChainUnlink] = &[
    ChainUnlink {
        addr: 0x00b0_7d50,
        obj: 0x00c6_13f0,
        vt: 0x00b0_e078,
    },
    ChainUnlink {
        addr: 0x00b0_8330,
        obj: 0x00c6_81ec,
        vt: 0x00b0_e078,
    },
    ChainUnlink {
        addr: 0x00b0_8650,
        obj: 0x00c6_845c,
        vt: 0x00b0_e078,
    },
];

/// The pooled trees' destructors (4).
pub const POOL_TREE_DTORS: &[PoolTreeDtor] = &[
    PoolTreeDtor {
        addr: 0x00b0_74e0,
        tree: 0x00c3_4b4c,
        pool: 0x00c3_4b64,
    },
    PoolTreeDtor {
        addr: 0x00b0_7560,
        tree: 0x00c3_4b1c,
        pool: 0x00c3_4b34,
    },
    PoolTreeDtor {
        addr: 0x00b0_7750,
        tree: 0x00c3_4ffc,
        pool: 0x00c3_5014,
    },
    PoolTreeDtor {
        addr: 0x00b0_77d0,
        tree: 0x00c3_4fcc,
        pool: 0x00c3_4fe4,
    },
];

/// The maps' destructors (3).
pub const MAP_DTORS: &[MapDtor] = &[
    MapDtor {
        addr: 0x00b0_7920,
        map: 0x00ce_6eb0,
        erase: 0x0063_6d60,
    },
    MapDtor {
        addr: 0x00b0_7c20,
        map: 0x012f_4104,
        erase: 0x0080_7510,
    },
    MapDtor {
        addr: 0x00b0_7ca0,
        map: 0x012f_40f8,
        erase: 0x0080_7210,
    },
];

/// The owned buffers' destructors (1).
pub const OWNED_BUFFERS: &[OwnedBuffer] = &[OwnedBuffer {
    addr: 0x00b0_7640,
    obj: 0x00cd_efa4,
}];

/// The one-word import calls (1).
pub const IMPORT_CALLS: &[ImportCall] = &[ImportCall {
    addr: 0x00b0_7d20,
    iat: 0x00b0_c0dc,
    arg: ImportArg::Word(0x012f_410c),
}];

/// The conditional tail-jumps on a loaded pointer (1).
pub const COND_TAIL_LOADS: &[CondTailLoad] = &[CondTailLoad {
    addr: 0x00b0_7d30,
    ptr: 0x0193_cd2c,
    target: 0x0094_5b80,
}];

/// The vtable stores (1).
pub const STORE_VTS: &[StoreVt] = &[StoreVt {
    addr: 0x00b0_7dce,
    obj: 0x00c6_4bf0,
    vt: 0x00b7_02b4,
}];
