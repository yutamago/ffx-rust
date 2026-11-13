//! M5 finish, WP5 part 4 (wp5e, `ffx_prender::wp5e`): the `FFX.exe`
//! frontier of the no-Unicorn boot, continued. Parity tests:
//! `crates/ffx-oracle/tests/wp5e.rs`.
//!
//! Threads: the main thread (the tips) and the loader thread (the symbol
//! map). No shim keeps state; frames are `StackFrame`s.

use ffx_prender::wp5e as e;
use ffx_system::m5b_stl::bridge as st;
use ffx_system::wp5_tree::bridge as tb;

use super::{StackFrame, mem, ported};
use ffx_system::stl::bridge as stl;

/// A [`e::StringMapErase`] row's callees as `m5b_stl`'s erase takes them.
fn erase_calls(row: &e::StringMapErase) -> ffx_system::m5b_stl::EraseCalls {
    ffx_system::m5b_stl::EraseCalls {
        rotate_left: row.rotate_left,
        rotate_right: row.rotate_right,
        min: row.min,
        max: row.max,
        string_key: Some(row.key),
    }
}

ported! {
    /// `SymbolMap_pairMove_006f33d0`: the code of `StdString_moveWithWord_0069c200`; `this`.
    SymbolMap_pairMove_006f33d0 @ 0x006f_33d0 => extern "thiscall" fn e_006f33d0(this: u32, src: u32, _unused: u32) -> u32 {
        st::string_move_with_word(&mut mem(), this, src)
    }
    /// `SymbolMap_buyNode_006f34f0`: a node with the pair moved in.
    SymbolMap_buyNode_006f34f0 @ 0x006f_34f0 => extern "thiscall" fn e_006f34f0(this: u32, pair: u32) -> u32 {
        e::symbol_buy_node(&mut mem(), this, pair)
    }
    /// `SymbolMap_insertHint_006f35b0`: `node` hung by the hint or by `_Insert_nohint`; `out`.
    SymbolMap_insertHint_006f35b0 @ 0x006f_35b0 => extern "thiscall" fn e_006f35b0(this: u32, out: u32, at: u32, value: u32, node: u32) -> u32 {
        let mut frame = StackFrame::<2>::new();
        e::symbol_insert_hint(&mut mem(), this, out, at, value, node, frame.at())
    }
    /// `SymbolMap_index_006f3ae0`: `&map[key]`, inserted with 0 when missing.
    SymbolMap_index_006f3ae0 @ 0x006f_3ae0 => extern "thiscall" fn e_006f3ae0(this: u32, key: u32) -> u32 {
        let mut frame = StackFrame::<8>::new();
        e::symbol_map_index(&mut mem(), this, key, frame.at())
    }
    /// `std_Tree_lower_bound_string_006557a0`: the code of `std_Tree_lower_bound_string_006e9920`.
    std_Tree_lower_bound_string_006557a0 @ 0x0065_57a0 => extern "thiscall" fn e_006557a0(this: u32, key: u32) -> u32 {
        tb::lower_bound_string(&mut mem(), this, key)
    }
    /// `std_Tree_lower_bound_string_006a06f0`: the code of `std_Tree_lower_bound_string_006e9920`.
    std_Tree_lower_bound_string_006a06f0 @ 0x006a_06f0 => extern "thiscall" fn e_006a06f0(this: u32, key: u32) -> u32 {
        tb::lower_bound_string(&mut mem(), this, key)
    }
    /// `TipSystem_ctor_006e85f0`: the tips' movie opened, its callback, disabled; `this`.
    TipSystem_ctor_006e85f0 @ 0x006e_85f0 => extern "thiscall" fn e_006e85f0(this: u32) -> u32 {
        let mut buf = StackFrame::<0x41>::new();
        e::tip_system_ctor(&mut mem(), this, buf.at())
    }
    /// `TipSystem_CreateInstance_006e8690`: `g_TipSystem` made.
    TipSystem_CreateInstance_006e8690 @ 0x006e_8690 => extern "C" fn e_006e8690() -> u32 {
        e::tip_system_create(&mut mem())
    }
    /// `TipSystem_DestroyInstance`: `g_TipSystem` and its menu freed.
    TipSystem_DestroyInstance @ 0x006e_8700 => extern "C" fn e_006e8700() -> u32 {
        e::tip_system_destroy(&mut mem())
    }
    /// `TipSystem_onHide_006e8760`: the tips' menu disabled.
    TipSystem_onHide_006e8760 @ 0x006e_8760 => extern "C" fn e_006e8760() -> u32 {
        e::tip_hide(&mut mem())
    }
    /// `AssetMap_copyCtor_0064d050(this; &src, alloc)`: the map copied; `this`.
    AssetMap_copyCtor_0064d050 @ 0x0064_d050 => extern "thiscall" fn e_0064d050(this: u32, src: u32, _alloc: u32) -> u32 {
        e::map_copy_ctor(&mut mem(), this, src)
    }
    /// `AssetMap_copy_00649bd0(this; &src, tag)`: the tree, size and extremes copied.
    AssetMap_copy_00649bd0 @ 0x0064_9bd0 => extern "thiscall" fn e_00649bd0(this: u32, src: u32, _tag: u32) -> u32 {
        e::map_copy(&mut mem(), this, src)
    }
    /// `AssetMap_treeCopy_00649cb0(this; node, where, tag)`: the subtree copied; its copy.
    AssetMap_treeCopy_00649cb0 @ 0x0064_9cb0 => extern "thiscall" fn e_00649cb0(this: u32, node: u32, at: u32, _tag: u32) -> u32 {
        e::tree_copy(&mut mem(), this, node, at)
    }
    /// `AssetMap_buyNode_00649a60(this; &pair)`: a node with the pair copied in.
    AssetMap_buyNode_00649a60 @ 0x0064_9a60 => extern "thiscall" fn e_00649a60(this: u32, pair: u32) -> u32 {
        e::map_buy_node(&mut mem(), this, pair)
    }
    /// `StdVectorString_buy_00654e20(this; n)`: storage for `n` strings (`al`).
    StdVectorString_buy_00654e20 @ 0x0065_4e20 => extern "thiscall" fn e_00654e20(this: u32, n: u32) -> u32 {
        e::vector_buy(&mut mem(), this, n, 0x18, 0x0aaa_aaaa)
    }
    /// `StdWords_uninitCopy_0064a850(first, last, dst)`: the words copied; the end.
    StdWords_uninitCopy_0064a850 @ 0x0064_a850 => extern "C" fn e_0064a850(first: u32, last: u32, dst: u32) -> u32 {
        e::words_copy(&mut mem(), first, last, dst)
    }
    /// `StdWords_uninitCopy_0064a880`: the same bytes as `0x0064a850`.
    StdWords_uninitCopy_0064a880 @ 0x0064_a880 => extern "C" fn e_0064a880(first: u32, last: u32, dst: u32) -> u32 {
        e::words_copy(&mut mem(), first, last, dst)
    }
    /// `StdStrings_uninitCopy_0064a8b0(first, last, dst)`: the strings copied; the end.
    StdStrings_uninitCopy_0064a8b0 @ 0x0064_a8b0 => extern "C" fn e_0064a8b0(first: u32, last: u32, dst: u32) -> u32 {
        e::strings_copy(&mut mem(), first, last, dst)
    }
    /// `StdVectorU32_copyCtor_0064d570(this; &src)`: the vector copied; `this`.
    StdVectorU32_copyCtor_0064d570 @ 0x0064_d570 => extern "thiscall" fn e_0064d570(this: u32, src: u32) -> u32 {
        e::vector_copy_ctor(&mut mem(), this, src, 4, e::VECTOR_BUY_4, e::WORDS_COPY)
    }
    /// `StdVectorString_copyCtor_0064d730(this; &src)`: the vector copied; `this`.
    StdVectorString_copyCtor_0064d730 @ 0x0064_d730 => extern "thiscall" fn e_0064d730(this: u32, src: u32) -> u32 {
        e::vector_copy_ctor(&mut mem(), this, src, 0x18, e::VECTOR_BUY_24, e::STRINGS_COPY)
    }
    /// `StringMap_erase_00636e20(this; &out, where)`: one node erased (its key freed); `out` = the successor.
    StringMap_erase_00636e20 @ 0x0063_6e20 => extern "thiscall" fn e_00636e20(this: u32, out: u32, _at: u32; slots: u32) -> u32 {
        let calls = erase_calls(&e::STRING_MAP_ERASES[0]);
        st::tree_erase_with(&mut mem(), this, out, slots.wrapping_add(4), &calls)
    }
    /// `std_Tree_erase_range_00636d60(this; &out, first, last)`: the range erased; `out`.
    std_Tree_erase_range_00636d60 @ 0x0063_6d60 => extern "thiscall" fn e_00636d60(this: u32, out: u32, first: u32, last: u32) -> u32 {
        let mut frame = StackFrame::<1>::new();
        e::string_map_erase_range(&mut mem(), this, out, first, last, &e::STRING_MAP_ERASES[0], frame.at())
    }
    /// `StringMap_erase_00637190(this; &out, where)`: one node erased (its key freed); `out` = the successor.
    StringMap_erase_00637190 @ 0x0063_7190 => extern "thiscall" fn e_00637190(this: u32, out: u32, _at: u32; slots: u32) -> u32 {
        let calls = erase_calls(&e::STRING_MAP_ERASES[1]);
        st::tree_erase_with(&mut mem(), this, out, slots.wrapping_add(4), &calls)
    }
    /// `StringMap_eraseRange_006370d0(this; &out, first, last)`: the range erased; `out`.
    StringMap_eraseRange_006370d0 @ 0x0063_70d0 => extern "thiscall" fn e_006370d0(this: u32, out: u32, first: u32, last: u32) -> u32 {
        let mut frame = StackFrame::<1>::new();
        e::string_map_erase_range(&mut mem(), this, out, first, last, &e::STRING_MAP_ERASES[1], frame.at())
    }
    /// `StdTree_eraseStrings_00635ce0(this; node)`: the code of `StdTree_eraseStrings_00635c80` (key at `+0x10`).
    StdTree_eraseStrings_00635ce0 @ 0x0063_5ce0 => extern "thiscall" fn e_00635ce0(_this: u32, node: u32) {
        e::erase_string_subtree(&mut mem(), node, 0x10);
    }
    /// `StringMap_erase_006a1720(this; &out, where)`: one node erased (its key freed); `out` = the successor.
    StringMap_erase_006a1720 @ 0x006a_1720 => extern "thiscall" fn e_006a1720(this: u32, out: u32, _at: u32; slots: u32) -> u32 {
        let calls = erase_calls(&e::STRING_MAP_ERASES[2]);
        st::tree_erase_with(&mut mem(), this, out, slots.wrapping_add(4), &calls)
    }
    /// `StringMap_eraseRange_006a1660(this; &out, first, last)`: the range erased; `out`.
    StringMap_eraseRange_006a1660 @ 0x006a_1660 => extern "thiscall" fn e_006a1660(this: u32, out: u32, first: u32, last: u32) -> u32 {
        let mut frame = StackFrame::<1>::new();
        e::string_map_erase_range(&mut mem(), this, out, first, last, &e::STRING_MAP_ERASES[2], frame.at())
    }
    /// `StdTree_eraseStrings_006a04e0(this; node)`: the code of `StdTree_eraseStrings_00635c80` (key at `+0x10`).
    StdTree_eraseStrings_006a04e0 @ 0x006a_04e0 => extern "thiscall" fn e_006a04e0(_this: u32, node: u32) {
        e::erase_string_subtree(&mut mem(), node, 0x10);
    }
    /// `std_Tree_erase_006d8a00(this; &out, where)`: one node erased (its key freed); `out` = the successor.
    std_Tree_erase_006d8a00 @ 0x006d_8a00 => extern "thiscall" fn e_006d8a00(this: u32, out: u32, _at: u32; slots: u32) -> u32 {
        let calls = erase_calls(&e::STRING_MAP_ERASES[3]);
        st::tree_erase_with(&mut mem(), this, out, slots.wrapping_add(4), &calls)
    }
    /// `std_Tree_erase_range_006d8940(this; &out, first, last)`: the range erased; `out`.
    std_Tree_erase_range_006d8940 @ 0x006d_8940 => extern "thiscall" fn e_006d8940(this: u32, out: u32, first: u32, last: u32) -> u32 {
        let mut frame = StackFrame::<1>::new();
        e::string_map_erase_range(&mut mem(), this, out, first, last, &e::STRING_MAP_ERASES[3], frame.at())
    }
    /// `StdTree_eraseStrings_006d79c0(this; node)`: the code of `StdTree_eraseStrings_00635c80` (key at `+0x14`).
    StdTree_eraseStrings_006d79c0 @ 0x006d_79c0 => extern "thiscall" fn e_006d79c0(_this: u32, node: u32) {
        e::erase_string_subtree(&mut mem(), node, 0x14);
    }
    /// `std_Tree_Lrotate_00625a60`: the code of `std_Tree_Lrotate_0069a3f0`.
    std_Tree_Lrotate_00625a60 @ 0x0062_5a60 => extern "thiscall" fn e_00625a60(this: u32, node: u32) {
        stl::tree_rotate_left(&mut mem(), this, node);
    }
    /// `std_Tree_Lrotate_00635ec0`: the code of `std_Tree_Lrotate_0069a3f0`.
    std_Tree_Lrotate_00635ec0 @ 0x0063_5ec0 => extern "thiscall" fn e_00635ec0(this: u32, node: u32) {
        stl::tree_rotate_left(&mut mem(), this, node);
    }
    /// `std_Tree_Lrotate_00635f20`: the code of `std_Tree_Lrotate_0069a3f0`.
    std_Tree_Lrotate_00635f20 @ 0x0063_5f20 => extern "thiscall" fn e_00635f20(this: u32, node: u32) {
        stl::tree_rotate_left(&mut mem(), this, node);
    }
    /// `std_Tree_Lrotate_006a0830`: the code of `std_Tree_Lrotate_0069a3f0`.
    std_Tree_Lrotate_006a0830 @ 0x006a_0830 => extern "thiscall" fn e_006a0830(this: u32, node: u32) {
        stl::tree_rotate_left(&mut mem(), this, node);
    }
    /// `std_Tree_Lrotate_copy_006d7b80`: the code of `std_Tree_Lrotate_0069a3f0`.
    std_Tree_Lrotate_copy_006d7b80 @ 0x006d_7b80 => extern "thiscall" fn e_006d7b80(this: u32, node: u32) {
        stl::tree_rotate_left(&mut mem(), this, node);
    }
    /// `std_Tree_Lrotate_copy_00806cb0`: the code of `std_Tree_Lrotate_0069a3f0`.
    std_Tree_Lrotate_copy_00806cb0 @ 0x0080_6cb0 => extern "thiscall" fn e_00806cb0(this: u32, node: u32) {
        stl::tree_rotate_left(&mut mem(), this, node);
    }
    /// `std_Tree_Lrotate_copy_00806d10`: the code of `std_Tree_Lrotate_0069a3f0`.
    std_Tree_Lrotate_copy_00806d10 @ 0x0080_6d10 => extern "thiscall" fn e_00806d10(this: u32, node: u32) {
        stl::tree_rotate_left(&mut mem(), this, node);
    }
    /// `std_Tree_Rrotate_00625b60`: the code of `std_Tree_Rrotate_0069a4f0`.
    std_Tree_Rrotate_00625b60 @ 0x0062_5b60 => extern "thiscall" fn e_00625b60(this: u32, node: u32) {
        stl::tree_rotate_right(&mut mem(), this, node);
    }
    /// `std_Tree_Rrotate_00636140`: the code of `std_Tree_Rrotate_0069a4f0`.
    std_Tree_Rrotate_00636140 @ 0x0063_6140 => extern "thiscall" fn e_00636140(this: u32, node: u32) {
        stl::tree_rotate_right(&mut mem(), this, node);
    }
    /// `std_Tree_Rrotate_006361a0`: the code of `std_Tree_Rrotate_0069a4f0`.
    std_Tree_Rrotate_006361a0 @ 0x0063_61a0 => extern "thiscall" fn e_006361a0(this: u32, node: u32) {
        stl::tree_rotate_right(&mut mem(), this, node);
    }
    /// `std_Tree_Rrotate_006a0af0`: the code of `std_Tree_Rrotate_0069a4f0`.
    std_Tree_Rrotate_006a0af0 @ 0x006a_0af0 => extern "thiscall" fn e_006a0af0(this: u32, node: u32) {
        stl::tree_rotate_right(&mut mem(), this, node);
    }
    /// `std_Tree_Rrotate_copy_006d7cc0`: the code of `std_Tree_Rrotate_0069a4f0`.
    std_Tree_Rrotate_copy_006d7cc0 @ 0x006d_7cc0 => extern "thiscall" fn e_006d7cc0(this: u32, node: u32) {
        stl::tree_rotate_right(&mut mem(), this, node);
    }
    /// `std_Tree_Rrotate_copy_00806eb0`: the code of `std_Tree_Rrotate_0069a4f0`.
    std_Tree_Rrotate_copy_00806eb0 @ 0x0080_6eb0 => extern "thiscall" fn e_00806eb0(this: u32, node: u32) {
        stl::tree_rotate_right(&mut mem(), this, node);
    }
    /// `std_Tree_Rrotate_copy_00806f10`: the code of `std_Tree_Rrotate_0069a4f0`.
    std_Tree_Rrotate_copy_00806f10 @ 0x0080_6f10 => extern "thiscall" fn e_00806f10(this: u32, node: u32) {
        stl::tree_rotate_right(&mut mem(), this, node);
    }
    /// `std_Tree_max_00625ac0`: the code of `std_Tree_max_0069a450`.
    std_Tree_max_00625ac0 @ 0x0062_5ac0 => extern "C" fn e_00625ac0(node: u32) -> u32 {
        tb::tree_max(&mut mem(), node)
    }
    /// `std_Tree_max_00635f80`: the code of `std_Tree_max_0069a450`.
    std_Tree_max_00635f80 @ 0x0063_5f80 => extern "C" fn e_00635f80(node: u32) -> u32 {
        tb::tree_max(&mut mem(), node)
    }
    /// `std_Tree_max_00635fa0`: the code of `std_Tree_max_0069a450`.
    std_Tree_max_00635fa0 @ 0x0063_5fa0 => extern "C" fn e_00635fa0(node: u32) -> u32 {
        tb::tree_max(&mut mem(), node)
    }
    /// `std_Tree_max_006a0900`: the code of `std_Tree_max_0069a450`.
    std_Tree_max_006a0900 @ 0x006a_0900 => extern "C" fn e_006a0900(node: u32) -> u32 {
        tb::tree_max(&mut mem(), node)
    }
    /// `std_Tree_max_006a0920`: the code of `std_Tree_max_0069a450`.
    std_Tree_max_006a0920 @ 0x006a_0920 => extern "C" fn e_006a0920(node: u32) -> u32 {
        tb::tree_max(&mut mem(), node)
    }
    /// `std_Tree_max_006d7be0`: the code of `std_Tree_max_0069a450`.
    std_Tree_max_006d7be0 @ 0x006d_7be0 => extern "C" fn e_006d7be0(node: u32) -> u32 {
        tb::tree_max(&mut mem(), node)
    }
    /// `std_Tree_max_00806d90`: the code of `std_Tree_max_0069a450`.
    std_Tree_max_00806d90 @ 0x0080_6d90 => extern "C" fn e_00806d90(node: u32) -> u32 {
        tb::tree_max(&mut mem(), node)
    }
    /// `std_Tree_min_00625ae0`: the code of `std_Tree_min_0069a470`.
    std_Tree_min_00625ae0 @ 0x0062_5ae0 => extern "C" fn e_00625ae0(node: u32) -> u32 {
        tb::tree_min(&mut mem(), node)
    }
    /// `std_Tree_min_00635fc0`: the code of `std_Tree_min_0069a470`.
    std_Tree_min_00635fc0 @ 0x0063_5fc0 => extern "C" fn e_00635fc0(node: u32) -> u32 {
        tb::tree_min(&mut mem(), node)
    }
    /// `std_Tree_min_00635fe0`: the code of `std_Tree_min_0069a470`.
    std_Tree_min_00635fe0 @ 0x0063_5fe0 => extern "C" fn e_00635fe0(node: u32) -> u32 {
        tb::tree_min(&mut mem(), node)
    }
    /// `std_Tree_min_00636000`: the code of `std_Tree_min_0069a470`.
    std_Tree_min_00636000 @ 0x0063_6000 => extern "C" fn e_00636000(node: u32) -> u32 {
        tb::tree_min(&mut mem(), node)
    }
    /// `std_Tree_min_006a0940`: the code of `std_Tree_min_0069a470`.
    std_Tree_min_006a0940 @ 0x006a_0940 => extern "C" fn e_006a0940(node: u32) -> u32 {
        tb::tree_min(&mut mem(), node)
    }
    /// `std_Tree_min_006d7c00`: the code of `std_Tree_min_0069a470`.
    std_Tree_min_006d7c00 @ 0x006d_7c00 => extern "C" fn e_006d7c00(node: u32) -> u32 {
        tb::tree_min(&mut mem(), node)
    }
    /// `std_Tree_min_00806dd0`: the code of `std_Tree_min_0069a470`.
    std_Tree_min_00806dd0 @ 0x0080_6dd0 => extern "C" fn e_00806dd0(node: u32) -> u32 {
        tb::tree_min(&mut mem(), node)
    }
    /// `TkFlag_take_0088e5e0()`: the flag byte `0x00c58e89`, cleared.
    TkFlag_take_0088e5e0 @ 0x0088_e5e0 => extern "C" fn e_0088e5e0() -> u32 {
        e::take_flag_c58e89(&mut mem())
    }
    /// `TkFlag_peek_0088e5f0()`: the flag byte `0x00c58e89`.
    TkFlag_peek_0088e5f0 @ 0x0088_e5f0 => extern "C" fn e_0088e5f0() -> u32 {
        e::peek_flag_c58e89(&mut mem())
    }
    /// `StdVector_reallocate_00655920`: the code of `StdVector_reallocate_006f3ee0`; the new end.
    StdVector_reallocate_00655920 @ 0x0065_5920 => extern "thiscall" fn e_00655920(this: u32, capacity: u32) -> u32 {
        st::vector_reallocate(&mut mem(), this, capacity)
    }
    /// `StdVector_reallocate_006559a0`: the code of `StdVector_reallocate_006f3ee0`; the new end.
    StdVector_reallocate_006559a0 @ 0x0065_59a0 => extern "thiscall" fn e_006559a0(this: u32, capacity: u32) -> u32 {
        st::vector_reallocate(&mut mem(), this, capacity)
    }
    /// `StdVector_reallocate_006a09b0`: the code of `StdVector_reallocate_006f3ee0`; the new end.
    StdVector_reallocate_006a09b0 @ 0x006a_09b0 => extern "thiscall" fn e_006a09b0(this: u32, capacity: u32) -> u32 {
        st::vector_reallocate(&mut mem(), this, capacity)
    }
    /// `StdVector_reserve_00655c60`: the code of `StdVector_reserve_006f3f60` (its own `_Reallocate` copy, the same code).
    StdVector_reserve_00655c60 @ 0x0065_5c60 => extern "thiscall" fn e_00655c60(this: u32, more: u32) -> u32 {
        st::vector_reserve(&mut mem(), this, more)
    }
    /// `StdVector_reserve_00655cd0`: the code of `StdVector_reserve_006f3f60` (its own `_Reallocate` copy, the same code).
    StdVector_reserve_00655cd0 @ 0x0065_5cd0 => extern "thiscall" fn e_00655cd0(this: u32, more: u32) -> u32 {
        st::vector_reserve(&mut mem(), this, more)
    }
    /// `StdVector_reserve_006a0a30`: the code of `StdVector_reserve_006f3f60` (its own `_Reallocate` copy, the same code).
    StdVector_reserve_006a0a30 @ 0x006a_0a30 => extern "thiscall" fn e_006a0a30(this: u32, more: u32) -> u32 {
        st::vector_reserve(&mut mem(), this, more)
    }
    /// `NamedValueMap_buyNode_00649b30(this; &pair)`: a node with the pair moved in.
    NamedValueMap_buyNode_00649b30 @ 0x0064_9b30 => extern "thiscall" fn e_00649b30(this: u32, pair: u32) -> u32 {
        e::named_value_buy_node(&mut mem(), this, pair)
    }
    /// `std_string_plusCstr_006495b0(out, &a, b)`: `out` = `a + b`; `out`.
    std_string_plusCstr_006495b0 @ 0x0064_95b0 => extern "C" fn e_006495b0(out: u32, a: u32, b: u32) -> u32 {
        e::string_plus_cstr(&mut mem(), out, a, b)
    }
    /// `dlmalloc_internal_memalign_006ea5f0(m, alignment, bytes)`: an aligned block.
    dlmalloc_internal_memalign_006ea5f0 @ 0x006e_a5f0 => extern "C" fn e_006ea5f0(m: u32, alignment: u32, bytes: u32) -> u32 {
        let mut spins = StackFrame::<1>::new();
        ffx_sg::m5main4_lib::bridge::internal_memalign(&mut mem(), m, alignment, bytes, spins.at())
    }
    /// `std_Tree_distance_0061ec20(first, last, &count)`: the nodes counted; `last`.
    std_Tree_distance_0061ec20 @ 0x0061_ec20 => extern "C" fn e_0061ec20(first: u32, last: u32, count: u32) -> u32 {
        e::tree_distance(&mut mem(), first, last, count)
    }
    /// `std_Tree_distance_00699340(first, last, &count)`: the nodes counted; `last`.
    std_Tree_distance_00699340 @ 0x0069_9340 => extern "C" fn e_00699340(first: u32, last: u32, count: u32) -> u32 {
        e::tree_distance(&mut mem(), first, last, count)
    }
    /// `std_Tree_distance_006f3520(first, last, &count)`: the nodes counted; `last`.
    std_Tree_distance_006f3520 @ 0x006f_3520 => extern "C" fn e_006f3520(first: u32, last: u32, count: u32) -> u32 {
        e::tree_distance(&mut mem(), first, last, count)
    }
    /// `PSharray_data_004347a0`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_004347a0 @ 0x0043_47a0 => extern "thiscall" fn e_004347a0(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_004347c0`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_004347c0 @ 0x0043_47c0 => extern "thiscall" fn e_004347c0(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_0044c350`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_0044c350 @ 0x0044_c350 => extern "thiscall" fn e_0044c350(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_0044c370`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_0044c370 @ 0x0044_c370 => extern "thiscall" fn e_0044c370(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_0044c530`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_0044c530 @ 0x0044_c530 => extern "thiscall" fn e_0044c530(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_0045de30`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_0045de30 @ 0x0045_de30 => extern "thiscall" fn e_0045de30(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_0045de50`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_0045de50 @ 0x0045_de50 => extern "thiscall" fn e_0045de50(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_004832f0`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_004832f0 @ 0x0048_32f0 => extern "thiscall" fn e_004832f0(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_00485db0`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_00485db0 @ 0x0048_5db0 => extern "thiscall" fn e_00485db0(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_0048c010`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_0048c010 @ 0x0048_c010 => extern "thiscall" fn e_0048c010(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_0048c030`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_0048c030 @ 0x0048_c030 => extern "thiscall" fn e_0048c030(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_0048c260`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_0048c260 @ 0x0048_c260 => extern "thiscall" fn e_0048c260(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_004a0600`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_004a0600 @ 0x004a_0600 => extern "thiscall" fn e_004a0600(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_004a0620`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_004a0620 @ 0x004a_0620 => extern "thiscall" fn e_004a0620(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_004c3de0`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_004c3de0 @ 0x004c_3de0 => extern "thiscall" fn e_004c3de0(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_004c3e00`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_004c3e00 @ 0x004c_3e00 => extern "thiscall" fn e_004c3e00(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_004c5680`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_004c5680 @ 0x004c_5680 => extern "thiscall" fn e_004c5680(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_004c56a0`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_004c56a0 @ 0x004c_56a0 => extern "thiscall" fn e_004c56a0(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_004c6fd0`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_004c6fd0 @ 0x004c_6fd0 => extern "thiscall" fn e_004c6fd0(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_004c6ff0`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_004c6ff0 @ 0x004c_6ff0 => extern "thiscall" fn e_004c6ff0(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_004d0650`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_004d0650 @ 0x004d_0650 => extern "thiscall" fn e_004d0650(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_004d0670`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_004d0670 @ 0x004d_0670 => extern "thiscall" fn e_004d0670(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_004d0820`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_004d0820 @ 0x004d_0820 => extern "thiscall" fn e_004d0820(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_004d4030`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_004d4030 @ 0x004d_4030 => extern "thiscall" fn e_004d4030(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_004f7b10`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_004f7b10 @ 0x004f_7b10 => extern "thiscall" fn e_004f7b10(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_004f7b30`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_004f7b30 @ 0x004f_7b30 => extern "thiscall" fn e_004f7b30(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_004f7b50`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_004f7b50 @ 0x004f_7b50 => extern "thiscall" fn e_004f7b50(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_004f7b70`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_004f7b70 @ 0x004f_7b70 => extern "thiscall" fn e_004f7b70(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_004f7f50`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_004f7f50 @ 0x004f_7f50 => extern "thiscall" fn e_004f7f50(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_004f8140`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_004f8140 @ 0x004f_8140 => extern "thiscall" fn e_004f8140(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_00502960`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_00502960 @ 0x0050_2960 => extern "thiscall" fn e_00502960(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_00502980`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_00502980 @ 0x0050_2980 => extern "thiscall" fn e_00502980(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_0051cde0`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_0051cde0 @ 0x0051_cde0 => extern "thiscall" fn e_0051cde0(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_0051ce00`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_0051ce00 @ 0x0051_ce00 => extern "thiscall" fn e_0051ce00(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_0051d220`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_0051d220 @ 0x0051_d220 => extern "thiscall" fn e_0051d220(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_0054ffe0`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_0054ffe0 @ 0x0054_ffe0 => extern "thiscall" fn e_0054ffe0(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_00550000`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_00550000 @ 0x0055_0000 => extern "thiscall" fn e_00550000(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_005502b0`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_005502b0 @ 0x0055_02b0 => extern "thiscall" fn e_005502b0(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_0055b300`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_0055b300 @ 0x0055_b300 => extern "thiscall" fn e_0055b300(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_00562a40`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_00562a40 @ 0x0056_2a40 => extern "thiscall" fn e_00562a40(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_00562a60`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_00562a60 @ 0x0056_2a60 => extern "thiscall" fn e_00562a60(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_0057b130`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_0057b130 @ 0x0057_b130 => extern "thiscall" fn e_0057b130(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_005a7030`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_005a7030 @ 0x005a_7030 => extern "thiscall" fn e_005a7030(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_005a7050`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_005a7050 @ 0x005a_7050 => extern "thiscall" fn e_005a7050(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_005a7070`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_005a7070 @ 0x005a_7070 => extern "thiscall" fn e_005a7070(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_005a7090`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_005a7090 @ 0x005a_7090 => extern "thiscall" fn e_005a7090(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_005a7590`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_005a7590 @ 0x005a_7590 => extern "thiscall" fn e_005a7590(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_005fc7e0`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_005fc7e0 @ 0x005f_c7e0 => extern "thiscall" fn e_005fc7e0(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_005fc800`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_005fc800 @ 0x005f_c800 => extern "thiscall" fn e_005fc800(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_005fe400`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_005fe400 @ 0x005f_e400 => extern "thiscall" fn e_005fe400(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
    /// `PSharray_data_006280f0`: the code of `PArray_Data_004d45a0`; the elements.
    PSharray_data_006280f0 @ 0x0062_80f0 => extern "thiscall" fn e_006280f0(this: u32) -> u32 {
        ffx_pcore::small::bridge::array_data(&mut mem(), this)
    }
}
