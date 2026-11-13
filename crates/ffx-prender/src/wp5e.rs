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
    let node = h.call_sym(BUY_NODE_2C, Conv::Thiscall(this), &[]);
    h.write(node.wrapping_add(0xc), 0u16);
    let at = node.wrapping_add(0x10);
    if at != 0 {
        h.call_sym(PAIR_MOVE, Conv::Thiscall(at), &[pair, 0]);
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

/// `a < b` for two strings through [`STRING_LESS`] (`al`).
fn less(h: &mut impl Host, a: u32, b: u32) -> bool {
    h.call_sym(STRING_LESS, Conv::Stdcall, &[a, b]) & 0xff != 0
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
    let hang = |h: &mut H, left: bool, place: u32| {
        h_insert_at(h, this, out, left, place, value, node);
        out
    };
    let head: u32 = h.read(this);
    if h.read::<u32>(this.wrapping_add(4)) == 0 {
        return hang(h, true, head);
    }
    let key = |n: u32| n.wrapping_add(0x10);
    if at == h.read::<u32>(head) {
        // the leftmost
        if compare(h, value, key(at)) < 0 {
            return hang(h, true, at);
        }
    } else if at == head {
        let last: u32 = h.read(head.wrapping_add(8));
        if compare(h, key(last), value) < 0 {
            return hang(h, false, last);
        }
    } else {
        if compare(h, value, key(at)) < 0 {
            h.write(slot, at);
            h.call_sym(ITERATOR_DEC, Conv::Thiscall(slot), &[]);
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
            h.call_sym(ITERATOR_INC, Conv::Thiscall(slot), &[]);
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
    let r = h.call_sym(INSERT_NOHINT, Conv::Thiscall(this), &[slot, 0, value, node]);
    let it: u32 = h.read(r);
    h.write(out, it);
    out
}

/// Whether `n`'s right child is a nil node.
fn right_is_nil(h: &impl Host, n: u32) -> bool {
    let r: u32 = h.read(n.wrapping_add(8));
    h.read::<u8>(r.wrapping_add(0xd)) != 0
}

/// `_Insert_at(this; out, left, place, value, node)` ([`INSERT_AT`]).
fn h_insert_at(
    h: &mut impl Host,
    this: u32,
    out: u32,
    left: bool,
    place: u32,
    value: u32,
    node: u32,
) -> u32 {
    h.call_sym(
        INSERT_AT,
        Conv::Thiscall(this),
        &[out, u32::from(left), place, value, node],
    )
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
