//! M5 finish, WP5 part 4 (wp5e, the `FFX.exe` frontier continued): the
//! no-Unicorn boot's next stops (`crates/ffx-host/tests/no_unicorn.rs`)
//! and their callees. Each follows its original's machine code; `eax` is
//! the original's.

use ffx_bridge::Mem;
use ffx_graphic::host::{Conv, Host};

use crate::wp5d::{BUY_NODE_2C, OPERATOR_DELETE, OPERATOR_NEW, STRING_ASSIGN_SUBSTR, string_parts};

/// `std::string::compare(pos, n, p, len)` (`0x006590d0`, thiscall, `ret
/// 0x10`).
pub const STRING_COMPARE: u32 = 0x0065_90d0;
/// `std::string` `<` (`0x0064f550`, stdcall `(a, b)`, the answer in `al`).
pub const STRING_LESS: u32 = 0x0064_f550;
/// The string map's iterator `--` (`0x006e91f0`, thiscall on the
/// iterator's slot) and `++` (`0x006352b0`, fastcall, the same).
pub const ITERATOR_DEC: u32 = 0x006e_91f0;
pub const ITERATOR_INC: u32 = 0x0063_52b0;
/// The string map's `_Insert_at(this; &out, add_left, at, &value, node)`
/// (`0x006e8a50`) and `_Insert_nohint(this; &out, leftish, &value, node)`
/// (`0x006e8c80`).
pub const INSERT_AT: u32 = 0x006e_8a50;
pub const INSERT_NOHINT: u32 = 0x006e_8c80;
/// `std::map<std::string, u32>::lower_bound(this; &key)` (`0x006e9920`).
pub const LOWER_BOUND: u32 = 0x006e_9920;
/// The move of a `{std::string, u32}` pair (`0x006f33d0`, thiscall
/// `(src, unused)`, `ret 8`; the code of `StdString_moveWithWord_0069c200`).
pub const PAIR_MOVE: u32 = 0x006f_33d0;
/// The symbol map's `_Buynode(&pair)` (`0x006f34f0`) and
/// `_Insert_hint(&out, where, &value, node)` (`0x006f35b0`).
pub const SYMBOL_BUY_NODE: u32 = 0x006f_34f0;
pub const SYMBOL_INSERT_HINT: u32 = 0x006f_35b0;
/// The symbol map's `operator[](&key)` (`0x006f3ae0`).
pub const SYMBOL_INDEX: u32 = 0x006f_3ae0;

/// `std::map<std::string, u32>::_Buynode(this; &pair)` (`0x006f34f0`,
/// thiscall on the tree, `ret 4`; the symbol map's, `0x006f3ae0`'s
/// callee): a 0x2c-byte node ([`BUY_NODE_2C`]), its colour and nil bytes
/// (the word at `+0xc`) 0, the pair moved into `+0x10` by [`PAIR_MOVE`]
/// (the source emptied). The node.
pub fn symbol_buy_node(h: &mut impl Host, this: u32, pair: u32) -> u32 {
    buy_node_moved(h, this, pair, BUY_NODE_2C, PAIR_MOVE)
}

/// A 0x2c-byte node by `buy` (thiscall on the tree), its colour and nil
/// bytes 0, `pair` moved into `+0x10` by `mv` (thiscall `(src, 0)`, `ret
/// 8`) unless that address is 0. The node.
fn buy_node_moved(h: &mut impl Host, this: u32, pair: u32, buy: u32, mv: u32) -> u32 {
    let node = h.call_sym(buy, Conv::Thiscall(this), &[]);
    h.write(node.wrapping_add(0xc), 0u16);
    let at = node.wrapping_add(0x10);
    if at != 0 {
        h.call_sym(mv, Conv::Thiscall(at), &[pair, 0]);
    }
    node
}

/// `std::string::compare` of the string at `s` (all of it) with the
/// string at `t`: `s.compare(0, s.size, t.data, t.size)` through the
/// original ([`STRING_COMPARE`]); negative when `s` orders first.
fn compare(h: &mut impl Host, s: u32, t: u32) -> i32 {
    let (p, n) = string_parts(h, t);
    let size: u32 = h.read(s.wrapping_add(0x10));
    h.call_sym(STRING_COMPARE, Conv::Thiscall(s), &[0, size, p, n])
        .cast_signed()
}

/// `std::map<std::string, u32>::_Insert_hint(this; &out, where, &value,
/// node)` (`0x006f35b0`, thiscall, `ret 0x10`; MSVC 2012): `node` (whose
/// key is the string at `value`) hung next to `where` when the hint
/// fits, else by `_Insert_nohint(&tmp, 0, value, node)` and `*out =
/// tmp`:
/// - an empty map: `_Insert_at(out, left, head, ..)`;
/// - `where` the leftmost: left of it when the value orders first;
/// - `where` the head (end): right of the rightmost when it orders first;
/// - else, the value before `where` and after its predecessor: right of
///   the predecessor when that has no right child, else left of `where`;
///   the value after `where` and before its successor (or no successor):
///   right of `where` when that has no right child, else left of the
///   successor.
///
/// `slot`: two words of game memory, the iterator for `--`/`++` (the
/// original uses its `value` argument slot, `ebp + 0x10`) and
/// `_Insert_nohint`'s result pair (the original's `ebp - 0x18`). Returns
/// `out`.
pub fn symbol_insert_hint<H: Host>(
    h: &mut H,
    this: u32,
    out: u32,
    at: u32,
    value: u32,
    node: u32,
    slot: u32,
) -> u32 {
    let lt = |h: &mut H, a: u32, b: u32| compare(h, a, b) < 0;
    insert_hint_with(h, this, out, at, value, node, slot, &SYMBOL_HINT, lt)
}

/// A map's `_Insert_hint` callees: `_Insert_at`, `_Insert_nohint`, the
/// iterator `--`/`++` (thiscall on a slot) and the key `<` (stdcall `(a,
/// b)`, `al`).
pub struct HintOps {
    pub insert_at: u32,
    pub nohint: u32,
    pub dec: u32,
    pub inc: u32,
    pub less: u32,
}

/// The symbol map's ([`symbol_insert_hint`]).
pub const SYMBOL_HINT: HintOps = HintOps {
    insert_at: INSERT_AT,
    nohint: INSERT_NOHINT,
    dec: ITERATOR_DEC,
    inc: ITERATOR_INC,
    less: STRING_LESS,
};

/// The VFX render data table's `std::map<u64, ..>` ([`vfx_insert_hint`]).
pub const VFX_HINT: HintOps = HintOps {
    insert_at: VFX_INSERT_AT,
    nohint: 0x0069_cfe0,
    dec: VFX_ITERATOR_DEC,
    inc: 0x0069_ebd0,
    less: 0x0069_eda0,
};

/// The VFX render data table's `_Insert_hint(this; &out, where, &value,
/// node)` (`0x0069caf0`, thiscall, `ret 0x10`;
/// `VfxRenderDataMap_eraseRange_0069d990`'s): [`symbol_insert_hint`]'s
/// code with `u64` keys compared inline (high word first) and
/// [`VFX_HINT`]'s callees; the iterator in a local (`ebp - 0x18`) and
/// `_Insert_nohint`'s pair at `ebp - 0x28` (`slot`: two words). `out`.
pub fn vfx_insert_hint<H: Host>(
    h: &mut H,
    this: u32,
    out: u32,
    at: u32,
    value: u32,
    node: u32,
    slot: u32,
) -> u32 {
    let lt = |h: &mut H, a: u32, b: u32| key64_less(h, a, b);
    insert_hint_with(h, this, out, at, value, node, slot, &VFX_HINT, lt)
}

/// MSVC 2012's `std::_Tree::_Insert_hint` over `ops` and the key order
/// `lt` (the original's inline comparisons; the `<` calls go through
/// `ops.less`). See [`symbol_insert_hint`].
#[allow(clippy::too_many_arguments)]
fn insert_hint_with<H: Host>(
    h: &mut H,
    this: u32,
    out: u32,
    at: u32,
    value: u32,
    node: u32,
    slot: u32,
    ops: &HintOps,
    lt: impl Fn(&mut H, u32, u32) -> bool,
) -> u32 {
    let hang = |h: &mut H, left: bool, place: u32| {
        h.call_sym(
            ops.insert_at,
            Conv::Thiscall(this),
            &[out, u32::from(left), place, value, node],
        );
        out
    };
    let less = |h: &mut H, a: u32, b: u32| h.call_sym(ops.less, Conv::Stdcall, &[a, b]) & 0xff != 0;
    let head: u32 = h.read(this);
    if h.read::<u32>(this.wrapping_add(4)) == 0 {
        return hang(h, true, head);
    }
    let key = |n: u32| n.wrapping_add(0x10);
    if at == h.read::<u32>(head) {
        // the leftmost
        if lt(h, value, key(at)) {
            return hang(h, true, at);
        }
    } else if at == head {
        let last: u32 = h.read(head.wrapping_add(8));
        if lt(h, key(last), value) {
            return hang(h, false, last);
        }
    } else {
        if lt(h, value, key(at)) {
            h.write(slot, at);
            h.call_sym(ops.dec, Conv::Thiscall(slot), &[]);
            let prev: u32 = h.read(slot);
            if less(h, key(prev), value) {
                return if right_is_nil(h, prev) {
                    hang(h, false, prev)
                } else {
                    hang(h, true, at)
                };
            }
        }
        if less(h, key(at), value) {
            h.write(slot, at);
            h.call_sym(ops.inc, Conv::Thiscall(slot), &[]);
            let next: u32 = h.read(slot);
            if next == h.read::<u32>(this) || less(h, value, key(next)) {
                return if right_is_nil(h, at) {
                    hang(h, false, at)
                } else {
                    hang(h, true, next)
                };
            }
        }
    }
    let r = h.call_sym(ops.nohint, Conv::Thiscall(this), &[slot, 0, value, node]);
    let it: u32 = h.read(r);
    h.write(out, it);
    out
}

/// Whether `n`'s right child is a nil node.
fn right_is_nil(h: &impl Host, n: u32) -> bool {
    let r: u32 = h.read(n.wrapping_add(8));
    h.read::<u8>(r.wrapping_add(0xd)) != 0
}

/// `std::map<std::string, u32>::operator[](this; &key)` (`0x006f3ae0`,
/// thiscall, `ret 4`; `AssetListReader_getSymbolId`'s and
/// `AssetListReader_readAssetHeader`'s): `lower_bound(key)`
/// ([`LOWER_BOUND`]); when that is the head or the key orders before its
/// key, a pair (a copy of the key by `assign(key, 0, npos)`, the value 0)
/// in `frame`, a node made of it ([`SYMBOL_BUY_NODE`], which moves the
/// pair out) and hung by [`SYMBOL_INSERT_HINT`] at the bound (the pair's
/// string freed when it still holds a heap block). The value's address
/// (the node's `+0x28`).
///
/// `frame`: 0x20 bytes of game memory, the original's `ebp - 0x2c` (the
/// pair: its string, the value word at `+0x18`) and the insert's result
/// iterator at `+0x1c` (the original's `ebp - 0x30`).
pub fn symbol_map_index(h: &mut impl Host, this: u32, key: u32, frame: u32) -> u32 {
    let it = h.call_sym(LOWER_BOUND, Conv::Thiscall(this), &[key]);
    if it != h.read::<u32>(this) && compare(h, key, it.wrapping_add(0x10)) >= 0 {
        return it.wrapping_add(0x28);
    }
    let s = frame;
    h.write(s + 0x14, 0xfu32);
    h.write(s + 0x10, 0u32);
    h.write(s, 0u8);
    h.call_sym(
        STRING_ASSIGN_SUBSTR,
        Conv::Thiscall(s),
        &[key, 0, 0xffff_ffff],
    );
    h.write(s + 0x18, 0u32);
    let node = h.call_sym(SYMBOL_BUY_NODE, Conv::Thiscall(this), &[s]);
    let out = frame + 0x1c;
    h.call_sym(
        SYMBOL_INSERT_HINT,
        Conv::Thiscall(this),
        &[out, it, node.wrapping_add(0x10), node],
    );
    if h.read::<u32>(s + 0x14) >= 0x10 {
        let p: u32 = h.read(s);
        h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[p]);
    }
    let found: u32 = h.read(out);
    found.wrapping_add(0x28)
}

/// `g_TipSystem` (`TipSystem_Get`'s global).
pub const TIP_SYSTEM: u32 = 0x00ce_6ea4;
/// `LocalizationManager_GetInstance` (cdecl; the language at `+4`).
pub const LOCALIZATION_MANAGER: u32 = 0x006d_b070;
/// MSVCR110's `sprintf` import slot.
pub const SPRINTF_SLOT: u32 = 0x00b0_c3dc;
/// `"/FFX_Data/GameData/PS3Data/flash/BOX%s.swf"` and the suffixes
/// `"_KR"` (language 9), `"_CH"` (10), `""`.
pub const TIP_PATH_FORMAT: u32 = 0x00b4_b4a4;
pub const SUFFIX_KR: u32 = 0x00b4_b49c;
pub const SUFFIX_CH: u32 = 0x00b4_b4a0;
pub const EMPTY: u32 = 0x00b8_fbeb;
/// `IggyPlayerOpenScaled_006e9250(path, x, y, w, h)` (cdecl).
pub const OPEN_SCALED: u32 = 0x006e_9250;
/// `IggyMenu_addCallback_00a28e90(this; name, f)` (thiscall `ret 8`),
/// `"Hide"`, and the callback [`TIP_HIDE`] (`0x006e8760`).
pub const ADD_CALLBACK: u32 = 0x00a2_8e90;
pub const HIDE_NAME: u32 = 0x00b4_b4d0;
pub const TIP_HIDE: u32 = 0x006e_8760;
/// `IggyMenu_setEnable(this; on)` (thiscall `ret 4`).
pub const SET_ENABLE: u32 = 0x00a2_8ef0;
/// `IggyMenu_dtor_IggyMenu(this)` (thiscall).
pub const MENU_DTOR: u32 = 0x00a2_8a40;
/// `TipSystem::TipSystem(this)` (`0x006e85f0`).
pub const TIP_CTOR: u32 = 0x006e_85f0;
/// `TipSystem::CreateInstance()` (`0x006e8690`) and `DestroyInstance()`
/// (`0x006e8700`).
pub const TIP_CREATE: u32 = 0x006e_8690;
pub const TIP_DESTROY: u32 = 0x006e_8700;

/// `TipSystem::TipSystem(this)` (`0x006e85f0`, thiscall; `buf` 0x104
/// bytes of game memory, the original's `ebp - 0x108`): the tips' movie,
/// `BOX<suffix>.swf` by the language (9 Korean, 10 Chinese), opened at
/// (0, 0x2a3) size 0x3ae x 0x2d into `this[0]`, its `"Hide"` callback
/// [`TIP_HIDE`], and disabled. `this`.
pub fn tip_system_ctor(h: &mut impl Host, this: u32, buf: u32) -> u32 {
    let lm = h.call_sym(LOCALIZATION_MANAGER, Conv::Cdecl, &[]);
    let lang: u32 = h.read(lm.wrapping_add(4));
    let suffix = match lang {
        9 => SUFFIX_KR,
        10 => SUFFIX_CH,
        _ => EMPTY,
    };
    let sprintf: u32 = h.read(h.rebase(SPRINTF_SLOT));
    let fmt = h.rebase(TIP_PATH_FORMAT);
    let suffix = h.rebase(suffix);
    h.call(sprintf, Conv::Cdecl, &[buf, fmt, suffix]);
    let menu = h.call_sym(OPEN_SCALED, Conv::Cdecl, &[buf, 0, 0x2a3, 0x3ae, 0x2d]);
    h.write(this, menu);
    let name = h.rebase(HIDE_NAME);
    let f = h.rebase(TIP_HIDE);
    h.call_sym(ADD_CALLBACK, Conv::Thiscall(menu), &[name, f]);
    let menu: u32 = h.read(this);
    h.call_sym(SET_ENABLE, Conv::Thiscall(menu), &[0]);
    this
}

/// `TipSystem::CreateInstance()` (`0x006e8690`, cdecl): `operator
/// new(4)`, [`TIP_CTOR`] on it, into `g_TipSystem` (0 when the
/// allocation failed). The global's new value.
pub fn tip_system_create(h: &mut impl Host) -> u32 {
    let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[4]);
    let g = if p == 0 {
        0
    } else {
        h.call_sym(TIP_CTOR, Conv::Thiscall(p), &[])
    };
    let at = h.rebase(TIP_SYSTEM);
    h.write(at, g);
    g
}

/// `TipSystem::DestroyInstance()` (`0x006e8700`, cdecl): with a
/// `g_TipSystem`, its menu (word 0) destroyed and deleted, the word
/// cleared, the object deleted and the global cleared. `eax`: 0 without
/// one, else the last `operator delete`'s.
pub fn tip_system_destroy(h: &mut impl Host) -> u32 {
    let at = h.rebase(TIP_SYSTEM);
    let g: u32 = h.read(at);
    if g == 0 {
        return 0;
    }
    let menu: u32 = h.read(g);
    if menu != 0 {
        h.call_sym(MENU_DTOR, Conv::Thiscall(menu), &[]);
        h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[menu]);
    }
    let g: u32 = h.read(at);
    h.write(g, 0u32);
    let g: u32 = h.read(at);
    let r = h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[g]);
    h.write(at, 0u32);
    r
}

/// The tips' `"Hide"` callback (`0x006e8760`, cdecl, no words read):
/// `IggyMenu_setEnable(g_TipSystem[0]; 0)`. Its `eax`.
pub fn tip_hide(h: &mut impl Host) -> u32 {
    let g: u32 = h.read(h.rebase(TIP_SYSTEM));
    let menu: u32 = h.read(g);
    h.call_sym(SET_ENABLE, Conv::Thiscall(menu), &[0])
}

/// The copies of `PArray_Data_004d45a0` (`ffx_pcore::small`'s
/// `array_data`: `PSharray<T>`'s elements, the count word's top bit
/// masked: more than 1 the pointer at `+4`, 1 the inline element at `+4`,
/// 0 null; thiscall, no stack words): the same 27 bytes, found by a byte
/// search of `.text`, each a `PClassDataMemberArrayDynamic`'s array-start
/// callback (`Cluster_ApplyPointerFixups` calls them through the member's
/// `+0x38`).
pub const SHARRAY_DATA_COPIES: [u32; 51] = [
    0x0043_47a0,
    0x0043_47c0,
    0x0044_c350,
    0x0044_c370,
    0x0044_c530,
    0x0045_de30,
    0x0045_de50,
    0x0048_32f0,
    0x0048_5db0,
    0x0048_c010,
    0x0048_c030,
    0x0048_c260,
    0x004a_0600,
    0x004a_0620,
    0x004c_3de0,
    0x004c_3e00,
    0x004c_5680,
    0x004c_56a0,
    0x004c_6fd0,
    0x004c_6ff0,
    0x004d_0650,
    0x004d_0670,
    0x004d_0820,
    0x004d_4030,
    0x004f_7b10,
    0x004f_7b30,
    0x004f_7b50,
    0x004f_7b70,
    0x004f_7f50,
    0x004f_8140,
    0x0050_2960,
    0x0050_2980,
    0x0051_cde0,
    0x0051_ce00,
    0x0051_d220,
    0x0054_ffe0,
    0x0055_0000,
    0x0055_02b0,
    0x0055_b300,
    0x0056_2a40,
    0x0056_2a60,
    0x0057_b130,
    0x005a_7030,
    0x005a_7050,
    0x005a_7070,
    0x005a_7090,
    0x005a_7590,
    0x005f_c7e0,
    0x005f_c800,
    0x005f_e400,
    0x0062_80f0,
];

/// `std_Tree_BuyHeadNode_00654ea0()` (cdecl: a 0x2c-byte head node).
pub const BUY_HEAD_NODE: u32 = 0x0065_4ea0;
/// The asset list's `std::map<std::string, u32>` copy (`0x00649bd0`),
/// its `_Copy` (`0x00649cb0`) and `_Buynode(&pair)` (`0x00649a60`).
pub const MAP_COPY: u32 = 0x0064_9bd0;
pub const TREE_COPY: u32 = 0x0064_9cb0;
pub const MAP_BUY_NODE: u32 = 0x0064_9a60;
/// `std::vector<u32>`'s `_Buy` (`StdVector_allocate_00654d40`) and the
/// words' `_Uninit_copy` (`0x0064a850`; the same bytes at `0x0064a880`).
pub const VECTOR_BUY_4: u32 = 0x0065_4d40;
pub const WORDS_COPY: u32 = 0x0064_a850;
/// `std::vector<std::string>`'s `_Buy` (`0x00654e20`) and the strings'
/// `_Uninit_copy` (`0x0064a8b0`).
pub const VECTOR_BUY_24: u32 = 0x0065_4e20;
pub const STRINGS_COPY: u32 = 0x0064_a8b0;
/// MSVCP110's `_Xlength_error` and `_Xbad_alloc` import slots, and
/// `"vector<T> too long"`.
pub const XLENGTH_ERROR_SLOT: u32 = 0x00b0_c29c;
pub const XBAD_ALLOC_SLOT: u32 = 0x00b0_c2a0;
pub const VECTOR_TOO_LONG: u32 = 0x00b3_4498;

/// Calls the import in `slot` (cdecl).
fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
    let f: u32 = h.read(h.rebase(slot));
    h.call(f, Conv::Cdecl, args)
}

/// `std::map<std::string, u32>`'s copy constructor (`0x0064d050`,
/// thiscall `(this; &src, alloc)`, `ret 8`; `AssetListReader_copy_0064d9e0`'s
/// `+0`): the two words cleared, a head node ([`BUY_HEAD_NODE`]) at
/// `+0`, the tree copied by [`MAP_COPY`]. The allocator word the original
/// passes on is its `ebp - 0x14` with only the low byte written (0), which
/// no callee reads: 0 here (junk read: `[ebp-0x14]`'s upper bytes never
/// written; 0, brief 2026-10-24). `this`.
pub fn map_copy_ctor(h: &mut impl Host, this: u32, src: u32) -> u32 {
    h.write(this, 0u32);
    h.write(this.wrapping_add(4), 0u32);
    let head = h.call_sym(BUY_HEAD_NODE, Conv::Cdecl, &[]);
    h.write(this, head);
    h.call_sym(MAP_COPY, Conv::Thiscall(this), &[src, 0]);
    this
}

/// The map's `_Copy(&src)` (`0x00649bd0`, thiscall `(this; &src, tag)`,
/// `ret 8`; `tag`, an allocator word, is only passed on and never read):
/// the root copied under the head ([`tree_copy`]), the size taken, the head's leftmost and rightmost set (itself for
/// an empty tree). `eax`: the head for an empty tree, else the nil node
/// the rightmost walk ended on.
pub fn map_copy(h: &mut impl Host, this: u32, src: u32) -> u32 {
    let head: u32 = h.read(this);
    let src_head: u32 = h.read(src);
    let src_root: u32 = h.read(src_head.wrapping_add(4));
    let root = tree_copy(h, this, src_root, head);
    h.write(head.wrapping_add(4), root);
    let size: u32 = h.read(src.wrapping_add(4));
    h.write(this.wrapping_add(4), size);
    let head: u32 = h.read(this);
    let root: u32 = h.read(head.wrapping_add(4));
    let nil = |h: &mut _, n: u32| Mem::read::<u8>(h, n.wrapping_add(0xd)) != 0;
    if nil(h, root) {
        h.write(head, head);
        let head: u32 = h.read(this);
        h.write(head.wrapping_add(8), head);
        return head;
    }
    let mut n = root;
    loop {
        let l: u32 = h.read(n);
        if nil(h, l) {
            break;
        }
        n = l;
    }
    h.write(head, n);
    let head: u32 = h.read(this);
    let mut n: u32 = h.read(head.wrapping_add(4));
    let end = loop {
        let r: u32 = h.read(n.wrapping_add(8));
        if nil(h, r) {
            break r;
        }
        n = r;
    };
    h.write(head.wrapping_add(8), n);
    end
}

/// The map's recursive `_Copy(node, where)` (`0x00649cb0`, thiscall
/// `(this; node, where, tag)`, `ret 0xc`; `tag` only passed on): for a non-nil `node`, a node
/// made by [`MAP_BUY_NODE`] from its pair, its parent `where`, its
/// colour, its children the copies of `node`'s (left first). The copy of
/// `node` (the first one made), or the head for a nil one.
pub fn tree_copy(h: &mut impl Host, this: u32, node: u32, at: u32) -> u32 {
    let mut first: u32 = h.read(this);
    if h.read::<u8>(node.wrapping_add(0xd)) != 0 {
        return first;
    }
    let copy = h.call_sym(
        MAP_BUY_NODE,
        Conv::Thiscall(this),
        &[node.wrapping_add(0x10)],
    );
    h.write(copy.wrapping_add(4), at);
    let colour: u8 = h.read(node.wrapping_add(0xc));
    h.write(copy.wrapping_add(0xc), colour);
    if h.read::<u8>(first.wrapping_add(0xd)) != 0 {
        first = copy;
    }
    let left: u32 = h.read(node);
    let l = tree_copy(h, this, left, copy);
    h.write(copy, l);
    let right: u32 = h.read(node.wrapping_add(8));
    let r = tree_copy(h, this, right, copy);
    h.write(copy.wrapping_add(8), r);
    first
}

/// The map's `_Buynode(&pair)` (`0x00649a60`, thiscall on the tree,
/// `ret 4`): a 0x2c-byte node ([`BUY_NODE_2C`]), its colour and nil bytes
/// 0, the key copied (`+0x10` emptied, then `assign(&pair, 0, npos)`)
/// and the value word (`pair + 0x18`) to `+0x28`. The node.
pub fn map_buy_node(h: &mut impl Host, this: u32, pair: u32) -> u32 {
    let node = h.call_sym(BUY_NODE_2C, Conv::Thiscall(this), &[]);
    h.write(node.wrapping_add(0xc), 0u16);
    let s = node.wrapping_add(0x10);
    if s != 0 {
        h.write(s + 0x14, 0xfu32);
        h.write(s + 0x10, 0u32);
        h.write(s, 0u8);
        h.call_sym(
            STRING_ASSIGN_SUBSTR,
            Conv::Thiscall(s),
            &[pair, 0, 0xffff_ffff],
        );
        let v: u32 = h.read(pair.wrapping_add(0x18));
        h.write(s + 0x18, v);
    }
    node
}

/// `std::vector<T>`'s `_Buy(n)` for `size`-byte elements (`0x00654e20`:
/// 0x18, at most `max`; thiscall, `ret 4`): the three words cleared; `n`
/// 0: `al` 0 (`eax` 0); above `max` `_Xlength_error("vector<T> too
/// long")`; else `operator new(n * size)` (`_Xbad_alloc` on NULL), the
/// words set: `eax` the end of the storage with `al` 1.
pub fn vector_buy(h: &mut impl Host, this: u32, n: u32, size: u32, max: u32) -> u32 {
    h.write(this, 0u32);
    h.write(this.wrapping_add(4), 0u32);
    h.write(this.wrapping_add(8), 0u32);
    if n == 0 {
        return 0;
    }
    if n > max {
        let text = h.rebase(VECTOR_TOO_LONG);
        return import(h, XLENGTH_ERROR_SLOT, &[text]);
    }
    let bytes = n.wrapping_mul(size);
    let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[bytes]);
    if p == 0 {
        return import(h, XBAD_ALLOC_SLOT, &[]);
    }
    h.write(this, p);
    h.write(this.wrapping_add(4), p);
    let end = p.wrapping_add(bytes);
    h.write(this.wrapping_add(8), end);
    (end & !0xff) | 1
}

/// `std::_Uninitialized_copy` of words (`0x0064a850`, `0x0064a880`;
/// cdecl `(first, last, dst, ..)`): each word copied, a NULL `dst`
/// skipped but advanced. The end of `dst`.
pub fn words_copy(h: &mut impl Host, first: u32, last: u32, dst: u32) -> u32 {
    let (mut s, mut d) = (first, dst);
    while s != last {
        if d != 0 {
            let w: u32 = h.read(s);
            h.write(d, w);
        }
        s = s.wrapping_add(4);
        d = d.wrapping_add(4);
    }
    d
}

/// `std::_Uninitialized_copy` of `std::string`s (`0x0064a8b0`, cdecl
/// `(first, last, dst, ..)`): each target emptied, then `assign(src, 0,
/// npos)`; a NULL `dst` skipped but advanced. The end of `dst`.
pub fn strings_copy(h: &mut impl Host, first: u32, last: u32, dst: u32) -> u32 {
    let (mut s, mut d) = (first, dst);
    while s != last {
        if d != 0 {
            h.write(d + 0x14, 0xfu32);
            h.write(d + 0x10, 0u32);
            h.write(d, 0u8);
            h.call_sym(
                STRING_ASSIGN_SUBSTR,
                Conv::Thiscall(d),
                &[s, 0, 0xffff_ffff],
            );
        }
        s = s.wrapping_add(0x18);
        d = d.wrapping_add(0x18);
    }
    d
}

/// A `std::vector`'s copy constructor (`0x0064d570`: `u32`s, `_Buy`
/// [`VECTOR_BUY_4`], [`WORDS_COPY`]; `0x0064d730`: `std::string`s,
/// [`VECTOR_BUY_24`], [`STRINGS_COPY`]; thiscall `(this; &src)`, `ret
/// 4`): the words cleared, `_Buy(src.size())` (the byte difference
/// shifted right 2 for words, divided by `size` (signed) otherwise), and when it bought storage, the elements
/// copied and the end stored at `+4`. The copy's other words (an
/// allocator's address and `src`) are not read by either copy: 0 here.
/// `this`.
pub fn vector_copy_ctor(
    h: &mut impl Host,
    this: u32,
    src: u32,
    size: u32,
    buy: u32,
    copy: u32,
) -> u32 {
    h.write(this, 0u32);
    h.write(this.wrapping_add(4), 0u32);
    h.write(this.wrapping_add(8), 0u32);
    let first: u32 = h.read(src);
    let last: u32 = h.read(src.wrapping_add(4));
    let diff = last.wrapping_sub(first).cast_signed();
    // `sar 2` for words (rounding down), a signed division otherwise
    let n = if size == 4 {
        diff >> 2
    } else {
        diff / size.cast_signed()
    };
    if h.call_sym(buy, Conv::Thiscall(this), &[n.cast_unsigned()]) & 0xff != 0 {
        let first: u32 = h.read(src);
        let last: u32 = h.read(src.wrapping_add(4));
        let dst: u32 = h.read(this);
        let end = h.call_sym(copy, Conv::Cdecl, &[first, last, dst, 0, 0]);
        h.write(this.wrapping_add(4), end);
    }
    this
}

/// A `std::string` emptied: its heap block freed (a capacity of 16 or
/// more), capacity 15, size 0, the first byte 0.
fn string_tidy(h: &mut impl Host, s: u32) {
    if h.read::<u32>(s.wrapping_add(0x14)) >= 0x10 {
        let p: u32 = h.read(s);
        h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[p]);
    }
    h.write(s.wrapping_add(0x14), 0xfu32);
    h.write(s.wrapping_add(0x10), 0u32);
    h.write(s, 0u8);
}

/// `std::_Tree<.., std::string ..>::_Erase(this; node)` (`0x00635ce0`,
/// `0x006a04e0` with the key at `+0x10`, `0x006d79c0` at `+0x14`; the
/// code of `StdTree_eraseStrings_00635c80`, thiscall, `ret 4`; the tree
/// itself is not read): until a nil node (byte `+0xd`), the right subtree
/// (the original calls itself), the node's key emptied and the node
/// deleted, then on to its left child (read before).
pub fn erase_string_subtree(h: &mut impl Host, node: u32, key: u32) {
    let mut n = node;
    while h.read::<u8>(n.wrapping_add(0xd)) == 0 {
        let right: u32 = h.read(n.wrapping_add(8));
        erase_string_subtree(h, right, key);
        let left: u32 = h.read(n);
        string_tidy(h, n.wrapping_add(key));
        h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[n]);
        n = left;
    }
}

/// A string map's `erase(iterator)` and `erase(first, last)` copies and
/// their callees.
#[derive(Clone, Copy)]
pub struct StringMapErase {
    /// `erase(&out, where)` (thiscall, `ret 8`).
    pub erase: u32,
    /// `erase(&out, first, last)` (thiscall, `ret 0xc`).
    pub erase_range: u32,
    /// `_Erase(node)`, the subtree free (thiscall, `ret 4`).
    pub subtree: u32,
    /// The erase's rotations left and right (thiscall `(this; node)`),
    /// `_Min` and `_Max` (cdecl `(node)`), and the key's offset (the
    /// `std::string` the erase frees before the node): the fields of
    /// `ffx_system::m5b_stl::EraseCalls`.
    pub rotate_left: u32,
    pub rotate_right: u32,
    pub min: u32,
    pub max: u32,
    pub key: u32,
}

/// The four string maps' erase copies (`tools/dupes.py` near families
/// of `std_Tree_erase_range_00636d60` and `0x00636e20`): the asset
/// list's symbol map, two more string maps, and the movie player's
/// (`std_Tree_erase_006d8a00`, its key at `+0x14`).
pub const STRING_MAP_ERASES: [StringMapErase; 4] = [
    erase_copy(
        [0x0063_6e20, 0x0063_6d60, 0x0063_5c80],
        [0x0063_5ec0, 0x0063_6140, 0x0063_5fc0, 0x0063_5f80],
        0x10,
    ),
    erase_copy(
        [0x0063_7190, 0x0063_70d0, 0x0063_5ce0],
        [0x0063_5f20, 0x0063_61a0, 0x0063_5fe0, 0x0063_5fa0],
        0x10,
    ),
    erase_copy(
        [0x006a_1720, 0x006a_1660, 0x006a_04e0],
        [0x006a_0830, 0x006a_0af0, 0x0063_6000, 0x006a_0900],
        0x10,
    ),
    erase_copy(
        [0x006d_8a00, 0x006d_8940, 0x006d_79c0],
        [0x006d_7b80, 0x006d_7cc0, 0x006d_7c00, 0x006d_7be0],
        0x14,
    ),
];

/// One [`STRING_MAP_ERASES`] row: (erase, range erase, subtree free),
/// (rotation left, rotation right, `_Min`, `_Max`), the key's offset.
const fn erase_copy(at: [u32; 3], calls: [u32; 4], key: u32) -> StringMapErase {
    StringMapErase {
        erase: at[0],
        erase_range: at[1],
        subtree: at[2],
        rotate_left: calls[0],
        rotate_right: calls[1],
        min: calls[2],
        max: calls[3],
        key,
    }
}

/// The in-order successor of the non-nil `node`.
fn successor(h: &impl Host, node: u32) -> u32 {
    let nil = |n: u32| h.read::<u8>(n.wrapping_add(0xd)) != 0;
    let right: u32 = h.read(node.wrapping_add(8));
    if nil(right) {
        let mut cur = node;
        let mut p: u32 = h.read(node.wrapping_add(4));
        while !nil(p) && cur == h.read::<u32>(p.wrapping_add(8)) {
            cur = p;
            p = h.read(p.wrapping_add(4));
        }
        p
    } else {
        let mut n = right;
        loop {
            let l: u32 = h.read(n);
            if nil(l) {
                return n;
            }
            n = l;
        }
    }
}

/// A string map's `erase(&out, first, last)` (`0x00636d60` and its
/// copies, [`STRING_MAP_ERASES`]; thiscall, `ret 0xc`): the whole tree
/// (`first` the leftmost, `last` the head): `_Erase(root)` (the copy's
/// subtree free), the head's links itself, the size 0, `out` = the
/// head's left link; else node by node, each successor found first (in
/// the original's own `first` slot; a nil node keeps it, and its erase
/// throws), `erase(&tmp, node)` (`frame`: the original's `ebp - 4`), and
/// `out` = `last`. Returns `out`.
pub fn string_map_erase_range(
    h: &mut impl Host,
    this: u32,
    out: u32,
    first: u32,
    last: u32,
    copy: &StringMapErase,
    frame: u32,
) -> u32 {
    let head: u32 = h.read(this);
    if first == h.read::<u32>(head) && last == head {
        let root: u32 = h.read(head.wrapping_add(4));
        h.call_sym(copy.subtree, Conv::Thiscall(this), &[root]);
        for off in [4u32, 0, 8] {
            let head: u32 = h.read(this);
            h.write(head.wrapping_add(off), head);
        }
        let head: u32 = h.read(this);
        h.write(this.wrapping_add(4), 0u32);
        let l: u32 = h.read(head);
        h.write(out, l);
        return out;
    }
    let mut at = first;
    while at != last {
        let node = at;
        if h.read::<u8>(node.wrapping_add(0xd)) == 0 {
            at = successor(h, node);
        }
        h.call_sym(copy.erase, Conv::Thiscall(this), &[frame, node]);
    }
    h.write(out, at);
    out
}

/// The byte flag `0x00c58e89` (`Native_01c6_GetGlobal00c58e89`'s).
pub const FLAG_C58E89: u32 = 0x00c5_8e89;

/// `0x0088e5e0` (cdecl, no words; `Native_01c6_GetGlobal00c58e89`'s tail
/// jump): the flag byte `0x00c58e89` zero-extended, and the flag cleared.
pub fn take_flag_c58e89(h: &mut impl Host) -> u32 {
    let at = h.rebase(FLAG_C58E89);
    let v: u8 = h.read(at);
    h.write(at, 0u8);
    u32::from(v)
}

/// `0x0088e5f0` (cdecl, no words): the flag byte `0x00c58e89`
/// zero-extended.
pub fn peek_flag_c58e89(h: &mut impl Host) -> u32 {
    u32::from(h.read::<u8>(h.rebase(FLAG_C58E89)))
}

/// `std_string_grow_00404480(this; n, trim)`, `std_string_append_substr`
/// `(this; &s, pos, n)` and `std_string_append_ptr(this; p, n)`
/// (thiscall).
pub const STRING_GROW: u32 = 0x0040_4480;
pub const STRING_APPEND_SUBSTR: u32 = 0x0065_75a0;
pub const STRING_APPEND_PTR: u32 = 0x0065_76d0;

/// The length of the C string at `p`.
fn c_len(h: &impl Host, p: u32) -> u32 {
    let mut n = 0u32;
    while h.read::<u8>(p.wrapping_add(n)) != 0 {
        n += 1;
    }
    n
}

/// `operator+(const std::string& a, const char* b)` (`0x006495b0`, cdecl
/// `(out, &a, b)`): `out` made empty, room for `a.size() + strlen(b)`
/// (`_Grow(n, 1)` unless that is already the capacity; when it grew, the
/// size set back to 0 and the text terminated), then `append(a, 0, npos)`
/// and `append(b, strlen(b))`. `out`.
pub fn string_plus_cstr(h: &mut impl Host, out: u32, a: u32, b: u32) -> u32 {
    h.write(out.wrapping_add(0x14), 0xfu32);
    h.write(out.wrapping_add(0x10), 0u32);
    h.write(out, 0u8);
    let a_size: u32 = h.read(a.wrapping_add(0x10));
    let need = c_len(h, b).wrapping_add(a_size);
    let size: u32 = h.read(out.wrapping_add(0x10));
    if size <= need
        && h.read::<u32>(out.wrapping_add(0x14)) != need
        && h.call_sym(STRING_GROW, Conv::Thiscall(out), &[need, 1]) & 0xff != 0
    {
        let at = if h.read::<u32>(out.wrapping_add(0x14)) >= 0x10 {
            h.read(out)
        } else {
            out
        };
        h.write(out.wrapping_add(0x10), size);
        h.write(at.wrapping_add(size), 0u8);
    }
    h.call_sym(
        STRING_APPEND_SUBSTR,
        Conv::Thiscall(out),
        &[a, 0, 0xffff_ffff],
    );
    let n = c_len(h, b);
    h.call_sym(STRING_APPEND_PTR, Conv::Thiscall(out), &[b, n]);
    out
}

/// `NamedValueMap::_Buynode(&pair)` (`0x00649b30`, thiscall on the tree,
/// `ret 4`; `PhyreScene_doOnCharacterAllAssetsLoaded`'s): the code of
/// [`symbol_buy_node`] with `std_Tree_buyNode2c_00654f80` and the move
/// `NamedValue_moveCtor_006494d0`. The node.
pub fn named_value_buy_node(h: &mut impl Host, this: u32, pair: u32) -> u32 {
    buy_node_moved(h, this, pair, 0x0065_4f80, 0x0064_94d0)
}

/// `std::_Distance2` over a map's nodes (`0x006f3520`, and the same bytes
/// at `0x0061ec20`, `0x00699340`; cdecl `(first, last, &count, ..)`):
/// `*count` (read once) counted up once per node from `first` to `last` (in-order
/// successors; a nil node is not advanced from, so the original loops
/// forever there, and so does this). `eax`: `last`.
pub fn tree_distance(h: &mut impl Host, first: u32, last: u32, count: u32) -> u32 {
    let mut at = first;
    if at == last {
        return at;
    }
    // read once, then counted in a register
    let mut n: u32 = h.read(count);
    while at != last {
        n = n.wrapping_add(1);
        h.write(count, n);
        if h.read::<u8>(at.wrapping_add(0xd)) == 0 {
            at = successor(h, at);
        }
    }
    at
}

/// The VFX render data table's (`std::map<u64, ..>`) `_Insert_at`
/// (`0x0069c8e0`) and iterator `--` (`std_TreeIterator_dec_0069ed00`,
/// thiscall on the iterator's slot).
pub const VFX_INSERT_AT: u32 = 0x0069_c8e0;
pub const VFX_ITERATOR_DEC: u32 = 0x0069_ed00;

/// `a < b` for the `u64` keys at `a` and `b` (low word first).
fn key64_less(h: &impl Host, a: u32, b: u32) -> bool {
    let (ah, bh): (u32, u32) = (h.read(a.wrapping_add(4)), h.read(b.wrapping_add(4)));
    if ah != bh {
        return ah < bh;
    }
    h.read::<u32>(a) < h.read::<u32>(b)
}

/// The VFX render data table's `_Insert_nohint(this; &out, leftish,
/// &value, node)` (`0x0069cfe0`, thiscall, `ret 0x10`;
/// `ClassVFXRenderDataTable_Insert`'s): down from the root by the `u64`
/// key (the value's first two words, the node's `+0x10`), left while the
/// value orders before the node's key (with `leftish`: at most it); at
/// the leftmost place `_Insert_at(&it, 1, place, value, node)` at once,
/// else the predecessor (`--` on `slot`, the original's own `leftish`
/// argument slot, which also takes `_Insert_at`'s iterator) compared:
/// below the value, `_Insert_at(&it, left, place, ..)` and `out` = (it,
/// 1); else `node` deleted and `out` = (the predecessor, 0). The original
/// pushes its `left` byte as a word whose upper bytes it never wrote
/// (`[ebp-0x14]`); `_Insert_at` reads only the byte, so the port passes
/// 0 or 1 (junk read: `[ebp-0x13..0x11]` never written; 0, brief
/// 2026-10-24). Returns `out`.
pub fn vfx_insert_nohint(
    h: &mut impl Host,
    this: u32,
    out: u32,
    leftish: u32,
    value: u32,
    node: u32,
    slot: u32,
) -> u32 {
    let head: u32 = h.read(this);
    let mut add_left = true;
    let mut place = head;
    let mut at: u32 = h.read(head.wrapping_add(4));
    while h.read::<u8>(at.wrapping_add(0xd)) == 0 {
        place = at;
        let key = at.wrapping_add(0x10);
        add_left = if leftish & 0xff != 0 {
            !key64_less(h, key, value)
        } else {
            key64_less(h, value, key)
        };
        at = h.read(at.wrapping_add(if add_left { 0 } else { 8 }));
    }
    h.write(slot, place);
    if add_left {
        if place == h.read::<u32>(head) {
            return vfx_hang(h, this, out, slot, true, place, value, node);
        }
        h.call_sym(VFX_ITERATOR_DEC, Conv::Thiscall(slot), &[]);
    }
    let before: u32 = h.read(slot);
    if key64_less(h, before.wrapping_add(0x10), value) {
        return vfx_hang(h, this, out, slot, add_left, place, value, node);
    }
    h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[node]);
    h.write(out, before);
    h.write(out.wrapping_add(4), 0u8);
    out
}

/// `_Insert_at(&slot, left, place, value, node)` and `out` = (it, 1).
#[allow(clippy::too_many_arguments)]
fn vfx_hang(
    h: &mut impl Host,
    this: u32,
    out: u32,
    slot: u32,
    left: bool,
    place: u32,
    value: u32,
    node: u32,
) -> u32 {
    let it = h.call_sym(
        VFX_INSERT_AT,
        Conv::Thiscall(this),
        &[slot, u32::from(left), place, value, node],
    );
    let it: u32 = h.read(it);
    h.write(out, it);
    h.write(out.wrapping_add(4), 1u8);
    out
}

/// `PMatrix4_RowTimes_004c9c20(m; &out, &v)` and
/// `PMatrix4_FromRows_00450d40(m; r0, r1, r2, r3)` (thiscall).
pub const ROW_TIMES: u32 = 0x004c_9c20;
pub const FROM_ROWS: u32 = 0x0045_0d40;

/// A 4x4 matrix product (`0x005e4400`, cdecl `(out, &a, &b)`;
/// `PhyreScene_setObjectRotation`'s): each row of `b` (from the last,
/// `+0x30`, to the first) times `a` by [`ROW_TIMES`] into `frame` (the
/// original's `ebp - 0x40`: row 3 at `+0x30` .. row 0 at `+0`), then
/// `out` = [`FROM_ROWS`] of the four. `out`.
pub fn matrix_mul(h: &mut impl Host, out: u32, a: u32, b: u32, frame: u32) -> u32 {
    let mut rows = [0u32; 4];
    for i in (0..4u32).rev() {
        rows[i as usize] = h.call_sym(
            ROW_TIMES,
            Conv::Thiscall(a),
            &[frame + 0x10 * i, b + 0x10 * i],
        );
    }
    h.call_sym(FROM_ROWS, Conv::Thiscall(out), &rows);
    out
}

/// `IggyMenu_callIggyAS3External(this; target, name, n, values, unused)`
/// (thiscall `ret 0x14`), `IggyMenu_replay(this)`,
/// `iggyDataValueSet_00a28d70(v, utf8)` (cdecl: a string value) and
/// `IggyMenu_callFlashFunction_1(menu, name, value by value)` (cdecl, six
/// words).
pub const CALL_AS3: u32 = 0x00a2_8ba0;
pub const MENU_REPLAY: u32 = 0x00a2_8ee0;
pub const VALUE_SET_UTF8: u32 = 0x00a2_8d70;
pub const VALUE_SET_INT: u32 = 0x00a2_8d30;
pub const CALL_FLASH_1: u32 = 0x0068_dd40;
/// `"OnPlay"`.
pub const ON_PLAY: u32 = 0x00b4_b4d8;
/// The pause overlay's menu (`OverlayMenu_create_006f3130`'s global) and
/// its texts: `"showPause"`, `"hidePause"`, the argument `"pausa"`
/// (languages 3 and 5) or `"pause"`.
pub const PAUSE_MENU: u32 = 0x00ce_81fc;
pub const SHOW_PAUSE: u32 = 0x00b4_bc14;
pub const HIDE_PAUSE: u32 = 0x00b4_bc28;
pub const PAUSA: u32 = 0x00b4_bc0c;
pub const PAUSE: u32 = 0x00b4_bc20;
/// `Pause_ShowScreen_006f3240` (thiscall on the menu).
pub const PAUSE_SHOW: u32 = 0x006f_3240;

/// `TipSystem::PlayTip(this; text)` (`0x006e8810`, thiscall, `ret 4`):
/// without a movie (`this[0]`) `al` 0 (the port: `eax` 0); else the menu
/// enabled, two Iggy values in `frame` (0x20 bytes, the original's `ebp -
/// 0x20`: the integer 0, then the string `text`), the movie's AS3
/// `"OnPlay"(0, text)`, `IggyMenu_replay`; `al` 1 over the replay's
/// `eax`.
pub fn tip_play(h: &mut impl Host, this: u32, text: u32, frame: u32) -> u32 {
    let menu: u32 = h.read(this);
    if menu == 0 {
        return 0;
    }
    h.call_sym(SET_ENABLE, Conv::Thiscall(menu), &[1]);
    h.call_sym(VALUE_SET_INT, Conv::Cdecl, &[frame, 0]);
    h.call_sym(VALUE_SET_UTF8, Conv::Cdecl, &[frame + 0x10, text]);
    let menu: u32 = h.read(this);
    let name = h.rebase(ON_PLAY);
    h.call_sym(CALL_AS3, Conv::Thiscall(menu), &[0, name, 2, frame, 1]);
    let menu: u32 = h.read(this);
    let r = h.call_sym(MENU_REPLAY, Conv::Thiscall(menu), &[]);
    (r & !0xff) | 1
}

/// The pause overlay's `"showPause"` (`0x006f3240`, thiscall on the menu):
/// the menu enabled, a string value (`"pausa"` for the languages 3 and 5,
/// else `"pause"`) in `frame` (0x10 bytes; the original's `ebp - 0x20` or
/// `ebp - 0x10` by the branch), then `IggyMenu_callFlashFunction_1(menu,
/// "showPause", the value's four words)`. Its `eax`.
pub fn pause_show(h: &mut impl Host, menu: u32, frame: u32) -> u32 {
    h.call_sym(SET_ENABLE, Conv::Thiscall(menu), &[1]);
    let lm = h.call_sym(LOCALIZATION_MANAGER, Conv::Cdecl, &[]);
    let lang: u32 = h.read(lm.wrapping_add(4));
    let text = h.rebase(if lang == 3 || lang == 5 { PAUSA } else { PAUSE });
    h.call_sym(VALUE_SET_UTF8, Conv::Cdecl, &[frame, text]);
    let v: [u32; 4] = std::array::from_fn(|i| h.read(frame + 4 * i as u32));
    let name = h.rebase(SHOW_PAUSE);
    h.call_sym(
        CALL_FLASH_1,
        Conv::Cdecl,
        &[menu, name, v[0], v[1], v[2], v[3]],
    )
}

/// `0x006f3300` (cdecl): [`pause_show`] on the pause overlay's menu (a
/// tail jump with `ecx` the global).
pub fn pause_show_global(h: &mut impl Host) -> u32 {
    let menu: u32 = h.read(h.rebase(PAUSE_MENU));
    h.call_sym(PAUSE_SHOW, Conv::Thiscall(menu), &[])
}

/// `0x006f32d0` (cdecl): the pause overlay's menu disabled
/// (`IggyMenu_setEnable(menu; 0)`). Its `eax`.
pub fn pause_disable(h: &mut impl Host) -> u32 {
    let menu: u32 = h.read(h.rebase(PAUSE_MENU));
    h.call_sym(SET_ENABLE, Conv::Thiscall(menu), &[0])
}

/// `Pause_HideScreen` (`0x006f32e0`, cdecl): the pause overlay's AS3
/// `"hidePause"` with no values. Its `eax`.
pub fn pause_hide(h: &mut impl Host) -> u32 {
    let menu: u32 = h.read(h.rebase(PAUSE_MENU));
    let name = h.rebase(HIDE_PAUSE);
    h.call_sym(CALL_AS3, Conv::Thiscall(menu), &[0, name, 0, 0, 1])
}

/// `std::_Tree::_Erase(this; node)` for trees whose nodes need no
/// destruction (`0x0069a2d0`, `0x006a0540`, `0x00625960`, `0x00806b70`,
/// `0x00806bb0`, `0x00a417d0`: the same code, each calling itself;
/// thiscall, `ret 4`; the tree is not read): until a nil node, the right
/// subtree (recursively), then the node deleted and on to its left child
/// (read before).
pub fn free_subtree(h: &mut impl Host, node: u32) {
    let mut n = node;
    while h.read::<u8>(n.wrapping_add(0xd)) == 0 {
        let right: u32 = h.read(n.wrapping_add(8));
        free_subtree(h, right);
        let left: u32 = h.read(n);
        h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[n]);
        n = left;
    }
}

/// The copies of [`free_subtree`].
pub const FREE_SUBTREE_COPIES: [u32; 6] = [
    0x0062_5960,
    0x0069_a2d0,
    0x006a_0540,
    0x0080_6b70,
    0x0080_6bb0,
    0x00a4_17d0,
];

/// `Phyre_PhyreMalloc_0042fb80(size, align)` (cdecl).
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;

/// A word array's copy into a new block (`0x00635630` and its 20
/// byte-identical copies, [`WORDS_DUP_COPIES`]; cdecl `(n, src)`):
/// `n` 0: 0; else `PhyreMalloc(4 n, 4)` and, when it gave a block, the
/// `n` words copied into it (a slot at address 0 skipped without reading
/// the source on: never with a real block). The block.
pub fn words_dup(h: &mut impl Host, n: u32, src: u32) -> u32 {
    if n == 0 {
        return 0;
    }
    let block = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[n.wrapping_mul(4), 4]);
    if block == 0 {
        return 0;
    }
    let (mut d, mut s) = (block, src);
    for _ in 0..n {
        let at = d;
        d = d.wrapping_add(4);
        if at != 0 {
            let w: u32 = h.read(s);
            h.write(at, w);
            s = s.wrapping_add(4);
        }
    }
    block
}

/// The addresses of [`words_dup`]'s code (`tools/dupes.py` exact family).
pub const WORDS_DUP_COPIES: [u32; 21] = [
    0x0048_ff70,
    0x004d_c730,
    0x004e_4090,
    0x004e_af20,
    0x004e_af70,
    0x004e_afc0,
    0x004e_b010,
    0x004f_b800,
    0x0051_3af0,
    0x0052_81e0,
    0x0052_bbf0,
    0x0053_5a10,
    0x005a_aee0,
    0x005b_12d0,
    0x0063_5630,
    0x006a_ffc0,
    0x006b_0010,
    0x009e_42b0,
    0x009e_4300,
    0x00a0_33e0,
    0x00a3_3480,
];

/// `MsBattleCheck` (cdecl) and `graphicPushTipByLocId(id)` (cdecl), and
/// the auto-battle flag (`0x0133d6e0`).
pub const MS_BATTLE_CHECK: u32 = 0x0078_0c90;
pub const PUSH_TIP_BY_LOC_ID: u32 = 0x0064_2930;
pub const AUTO_BATTLE: u32 = 0x0133_d6e0;

/// `forceAutoBattleOff` (`0x007871f0`, cdecl): with the auto-battle flag
/// 1 and `MsBattleCheck()` true, the flag cleared and the tip 0x37d
/// pushed (`graphicPushTipByLocId`). `eax`: the tip's, `MsBattleCheck`'s
/// 0, or (the flag not 1) the caller's, here 0.
pub fn force_auto_battle_off(h: &mut impl Host) -> u32 {
    let at = h.rebase(AUTO_BATTLE);
    if h.read::<u32>(at) != 1 {
        return 0;
    }
    let r = h.call_sym(MS_BATTLE_CHECK, Conv::Cdecl, &[]);
    if r == 0 {
        return 0;
    }
    h.write(at, 0u32);
    h.call_sym(PUSH_TIP_BY_LOC_ID, Conv::Cdecl, &[0x37d])
}

/// `SaveDataGetLoc(id, out)` (cdecl: the localized text `id` into `out`).
pub const SAVE_DATA_GET_LOC: u32 = 0x0064_12d0;

/// `TipSystem::PushTip(this; id)` (`0x006e8770`, thiscall, `ret 4`):
/// the text `id` into a 0x400-byte buffer (`SaveDataGetLoc`, before the
/// movie check), then [`tip_play`] with it. `frame`: 0x420 bytes, the
/// original's `ebp - 0x424` (the two Iggy values at `+0`, the text at
/// `+0x20`).
pub fn tip_push(h: &mut impl Host, this: u32, id: u32, frame: u32) -> u32 {
    let text = frame + 0x20;
    h.call_sym(SAVE_DATA_GET_LOC, Conv::Cdecl, &[id, text]);
    tip_play(h, this, text, frame)
}

/// `0x00821cc0(on)` (cdecl: the free camera's pause), the scene camera's
/// `Phyre_PCamera_assign_004533b0(this; src)` (thiscall) and the free
/// camera controller's `0x0067fda0(this; camera)` (thiscall).
pub const SET_PAUSE: u32 = 0x0082_1cc0;
pub const CAMERA_ASSIGN: u32 = 0x0045_33b0;
pub const FREE_CAMERA_TAKE: u32 = 0x0067_fda0;

/// An `f32` copied through the x87 (`fld dword` / `fstp dword`): the
/// same bits, a signalling NaN quieted.
fn x87_copy(bits: u32) -> u32 {
    if bits & 0x7f80_0000 == 0x7f80_0000 && bits & 0x007f_ffff != 0 {
        bits | 0x0040_0000
    } else {
        bits
    }
}

/// The free camera takes the scene camera (`FreeCamera_setFreePause`'s and
/// `_setFreeCameraMode`'s tail): `Phyre_PCamera_assign(this + 0x10;
/// scene camera + 4, or 0 without one)`, its floats `+0x150`/`+0x154` to
/// `this + 0xc + ..`, then `0x0067fda0(this + 0x2bc; this + 0xc)`. Its
/// `eax`.
fn free_camera_take_scene(h: &mut impl Host, this: u32) -> u32 {
    let scene: u32 = h.read(h.rebase(PHYRE_SCENE));
    let cam: u32 = h.read(scene.wrapping_add(0xdc));
    let own = this.wrapping_add(0xc);
    let src = if cam == 0 { 0 } else { cam.wrapping_add(4) };
    h.call_sym(CAMERA_ASSIGN, Conv::Thiscall(own.wrapping_add(4)), &[src]);
    for off in [0x150u32, 0x154] {
        let v: u32 = h.read(cam.wrapping_add(off));
        h.write(own.wrapping_add(off), x87_copy(v));
    }
    h.call_sym(
        FREE_CAMERA_TAKE,
        Conv::Thiscall(this.wrapping_add(0x2bc)),
        &[own],
    )
}

/// `g_PhyreScene`.
pub const PHYRE_SCENE: u32 = 0x00cc_b9d8;

/// `FreeCamera::setFreePause(this)` (`0x00680320`, thiscall): the pause
/// byte `+0x338` toggled and passed to `0x00821cc0`; when it is now set
/// and the word `+0` is 0, the free camera takes the scene camera. `eax`:
/// the last call's.
pub fn free_camera_set_free_pause(h: &mut impl Host, this: u32) -> u32 {
    let at = this.wrapping_add(0x338);
    let on = h.read::<u8>(at) == 0;
    h.write(at, u8::from(on));
    let r = h.call_sym(SET_PAUSE, Conv::Cdecl, &[u32::from(on)]);
    if h.read::<u8>(at) == 0 || h.read::<u32>(this) != 0 {
        return r;
    }
    free_camera_take_scene(h, this)
}

/// `FreeCamera::setFreeCameraMode(this; mode)` (`0x006803a0`, thiscall,
/// `ret 4`): the byte `+0x33c` = `mode != 0`; when set and the word `+0`
/// is 0, the free camera takes the scene camera. `eax`: that call's, else
/// `al` the byte (the upper bytes the caller's: 0 here).
pub fn free_camera_set_mode(h: &mut impl Host, this: u32, mode: u32) -> u32 {
    let on = mode != 0;
    h.write(this.wrapping_add(0x33c), u8::from(on));
    if !on || h.read::<u32>(this) != 0 {
        return u32::from(on);
    }
    free_camera_take_scene(h, this)
}

/// `Ch_GetId(chr)` (cdecl), `Phyre_PRendering_PRendererBase_renderMeshInstance`
/// (thiscall `(renderer; instance, contexts, x, y)`, `ret 0x10`),
/// `PRendererBase_pushCommand20_005bb2e0(renderer; arg)` (thiscall) and
/// `PhyreScene_DrawMeshMatrices(scene; on)` (thiscall).
pub const CH_GET_ID: u32 = 0x0082_6b80;
pub const RENDER_MESH_INSTANCE: u32 = 0x005b_9a90;
pub const PUSH_COMMAND_20: u32 = 0x005b_b2e0;
pub const DRAW_MESH_MATRICES: u32 = 0x0066_b360;
/// The render pass names the renderer's `+0x48` points at: `"Opaque"`,
/// `"Transparent"`, `"postTransparent"`, `"TransparentClipY"` (their
/// table entries).
pub const PASS_OPAQUE: u32 = 0x00c1_69b4;
pub const PASS_TRANSPARENT: u32 = 0x00c1_69d4;
pub const PASS_POST_TRANSPARENT: u32 = 0x00c1_6a44;
pub const PASS_TRANSPARENT_CLIP_Y: u32 = 0x00c1_6b54;

/// Every mesh instance of the event's list (`ev[0]` the count, top bit
/// masked and read again after each, `ev[1]` the array) rendered:
/// `renderMeshInstance(scene's renderer; mesh, mesh + 0x10's word, 0,
/// renderer + 0x13c's word)`, the renderer read again for each.
fn render_event_meshes(h: &mut impl Host, scene: u32, ev: u32) {
    let mut i = 0u32;
    while i < h.read::<u32>(ev) & 0x7fff_ffff {
        let r: u32 = h.read(scene.wrapping_add(0xd0));
        let list: u32 = h.read(ev.wrapping_add(4));
        let mesh: u32 = h.read(list.wrapping_add(4 * i));
        let contexts: u32 = h.read(mesh.wrapping_add(0x10));
        let y: u32 = h.read(r.wrapping_add(0x13c));
        h.call_sym(
            RENDER_MESH_INSTANCE,
            Conv::Thiscall(r),
            &[mesh, contexts, 0, y],
        );
        i += 1;
    }
}

/// The renderer's pass name (`+0x48`) set to the table entry `pass`.
fn set_pass(h: &mut impl Host, scene: u32, pass: u32) {
    let r: u32 = h.read(scene.wrapping_add(0xd0));
    let p = h.rebase(pass);
    h.write(r.wrapping_add(0x48), p);
}

/// `PhyreScene_processVFXDrawEvent`'s mesh pass (`0x006673b0`, thiscall
/// `(scene; event, clip)`, `ret 8`): the event's character (`+0x18`)
/// one of 0x10fb..0x10fd or 0x1100 draws the scene's mesh matrices
/// (`PhyreScene_DrawMeshMatrices(scene; 0)`) in place of the event's
/// meshes; the event's callback (`+0x1c`, cdecl `(event)`) first. With
/// `clip` (its low byte): command 2, the pass `"TransparentClipY"` and a
/// draw, command 1, the same pass and a draw again; else the passes
/// `"Opaque"` and `"Transparent"` with a draw each, then (not for the
/// special characters) `"postTransparent"` and one more. The original
/// keeps its flag in its own `event` slot's top byte (dead after). 0.
pub fn vfx_event_mesh_pass<H: Host>(h: &mut H, scene: u32, ev: u32, clip: u32) -> u32 {
    let chr: u32 = h.read(ev.wrapping_add(0x18));
    let id = h.call_sym(CH_GET_ID, Conv::Cdecl, &[chr]);
    let special = matches!(id, 0x10fb | 0x10fc | 0x10fd | 0x1100);
    let cb: u32 = h.read(ev.wrapping_add(0x1c));
    if cb != 0 {
        h.call(cb, Conv::Cdecl, &[ev]);
    }
    let draw = |h: &mut H| {
        if special {
            h.call_sym(DRAW_MESH_MATRICES, Conv::Thiscall(scene), &[0]);
        } else {
            render_event_meshes(h, scene, ev);
        }
    };
    if clip & 0xff != 0 {
        for command in [2u32, 1] {
            let r: u32 = h.read(scene.wrapping_add(0xd0));
            h.call_sym(PUSH_COMMAND_20, Conv::Thiscall(r), &[command]);
            set_pass(h, scene, PASS_TRANSPARENT_CLIP_Y);
            draw(h);
        }
        return 0;
    }
    set_pass(h, scene, PASS_OPAQUE);
    draw(h);
    set_pass(h, scene, PASS_TRANSPARENT);
    draw(h);
    if !special {
        set_pass(h, scene, PASS_POST_TRANSPARENT);
        render_event_meshes(h, scene, ev);
    }
    0
}

/// `std::_Uninitialized_fill_n` of words (`0x0064a970`, cdecl `(dst, n,
/// &value, ..)`): `n` slots from `dst` each get `*value` (read again for
/// each; a NULL slot skipped but advanced). `eax`: the end; for `n` 0
/// the original leaves the caller's `eax` (here `dst`).
pub fn words_fill(h: &mut impl Host, dst: u32, n: u32, value: u32) -> u32 {
    let mut d = dst;
    for _ in 0..n {
        if d != 0 {
            let v: u32 = h.read(value);
            h.write(d, v);
        }
        d = d.wrapping_add(4);
    }
    d
}

/// MSVCR110's `sprintf_s` import slot and the map-name tweak's texts:
/// `"init failed"`, the cursor `">"` / `" "`, the formats `"%s%s%s"`,
/// `"%s%s%d"`, `"%s%s"`, and `"true"` / `"false"`.
pub const SPRINTF_S_SLOT: u32 = 0x00b0_c394;
pub const INIT_FAILED: u32 = 0x00b4_8244;
pub const CURSOR_ON: u32 = 0x00b4_8250;
pub const CURSOR_OFF: u32 = 0x00b2_0b30;
pub const FMT_SSS: u32 = 0x00b4_2a3c;
pub const FMT_SSD: u32 = 0x00b4_88e0;
pub const FMT_SS: u32 = 0x00b4_0eb0;
pub const TRUE_TEXT: u32 = 0x00b3_9230;
pub const FALSE_TEXT: u32 = 0x00b3_9238;

/// The map-name tweak's `getLineText(this; n, buf)` (`0x006b6ef0`,
/// thiscall, `ret 8`; `MapNameText_Show_006bb480`'s): without the byte
/// `+0x110`, `"init failed"`; else line `n` (0..5, the cursor `">"` on
/// the line `+0x1c0`, else `" "`) by `sprintf_s(buf, 0x100, ..)`: 0 the
/// label `+0x1a4` and the text `+8`, 1 and 2 the labels `+0x1a8`/`+0x1ac`
/// and the numbers `+0`/`+4`, 3 the label `+0x1b0` and `+0x10c` as
/// true/false, 4 the label `+0x1b4` alone, 5 the label `+0x1b8` and
/// `+0x108` as true/false. `eax`: `sprintf_s`'s, or `n` past 5.
pub fn map_name_line_text<H: Host>(h: &mut H, this: u32, n: u32, buf: u32) -> u32 {
    let sprintf_s: u32 = h.read(h.rebase(SPRINTF_S_SLOT));
    let at = |o: u32| this.wrapping_add(o);
    if h.read::<u8>(at(0x110)) == 0 {
        let text = h.rebase(INIT_FAILED);
        return h.call(sprintf_s, Conv::Cdecl, &[buf, 0x100, text]);
    }
    let mark = h.rebase(if h.read::<u32>(at(0x1c0)) == n {
        CURSOR_ON
    } else {
        CURSOR_OFF
    });
    let flag = |h: &mut H, o: u32| {
        let v: u32 = h.read(at(o));
        h.rebase(if v == 0 { FALSE_TEXT } else { TRUE_TEXT })
    };
    let word = |h: &mut H, o: u32| -> u32 { h.read(at(o)) };
    let (fmt, label, last) = match n {
        0 => (FMT_SSS, word(h, 0x1a4), Some(at(8))),
        1 => (FMT_SSD, word(h, 0x1a8), Some(word(h, 0))),
        2 => (FMT_SSD, word(h, 0x1ac), Some(word(h, 4))),
        3 => (FMT_SSS, word(h, 0x1b0), Some(flag(h, 0x10c))),
        4 => (FMT_SS, word(h, 0x1b4), None),
        5 => (FMT_SSS, word(h, 0x1b8), Some(flag(h, 0x108))),
        _ => return n,
    };
    let fmt = h.rebase(fmt);
    match last {
        Some(x) => h.call(sprintf_s, Conv::Cdecl, &[buf, 0x100, fmt, mark, label, x]),
        None => h.call(sprintf_s, Conv::Cdecl, &[buf, 0x100, fmt, mark, label]),
    }
}

/// `LocText_ConvertLineBreaks_006e9700(src, dst, n)` (cdecl),
/// `GetDerefU32_00a28d10(this)` (thiscall: `this[0]`), and Iggy's
/// `IggyPlayerCallbackResultPath(player)` and `IggyValueSetStringUTF8RS(
/// path, a, b, text, len)` import slots (stdcall).
pub const CONVERT_LINE_BREAKS: u32 = 0x006e_9700;
pub const GET_DEREF: u32 = 0x00a2_8d10;
pub const IGGY_RESULT_PATH_SLOT: u32 = 0x00b0_c818;
pub const IGGY_SET_STRING_SLOT: u32 = 0x00b0_c81c;

/// `GuideMap_setLocationText_006e9870(menu, loc)` (cdecl;
/// `rcbgGuideMapSave`'s tail target and `IggyCallback_lookupName`'s): the
/// text `loc` (`SaveDataGetLoc`) into `frame` (0x800 bytes, the original's
/// `ebp - 0x804`), its line breaks converted into `frame + 0x400` (at most
/// 0x3ff bytes), and the Iggy callback's result set to it
/// (`IggyValueSetStringUTF8RS(IggyPlayerCallbackResultPath(menu[0]), 0,
/// 0, text, strlen(text))`). That call's `eax`.
pub fn guide_map_set_location_text(h: &mut impl Host, menu: u32, loc: u32, frame: u32) -> u32 {
    let raw = frame;
    let text = frame + 0x400;
    h.call_sym(SAVE_DATA_GET_LOC, Conv::Cdecl, &[loc, raw]);
    h.call_sym(CONVERT_LINE_BREAKS, Conv::Cdecl, &[raw, text, 0x3ff]);
    let player = h.call_sym(GET_DEREF, Conv::Thiscall(menu), &[]);
    let result_path: u32 = h.read(h.rebase(IGGY_RESULT_PATH_SLOT));
    let path = h.call(result_path, Conv::Stdcall, &[player]);
    let len = c_len(h, text);
    let set_string: u32 = h.read(h.rebase(IGGY_SET_STRING_SLOT));
    h.call(set_string, Conv::Stdcall, &[path, 0, 0, text, len])
}
