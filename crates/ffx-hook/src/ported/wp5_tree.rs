//! M5 finish, WP5 part 2 (`ffx_system::wp5_tree`): the cluster map's
//! `std::_Tree` insert and its iterator's `--` (a copy of
//! `ffx_glue_other::m5v_a`'s). Parity tests:
//! `crates/ffx-oracle/tests/wp5_string.rs` (`wp5b_tree_*`).
//!
//! Threads: the main and loader threads (cluster loads). The insert's
//! frame word is its own `StackFrame`.

use ffx_glue_other::m5v_a::bridge as gv;
use ffx_system::stl::bridge as stl;
use ffx_system::wp5_tree::bridge as b;

use super::{StackFrame, mem, ported};

ported! {
    /// `std_Tree_insertNohint_006995e0`: `node` hung in by its `u32` key, or freed for an equal one; `out`.
    std_Tree_insertNohint_006995e0 @ 0x0069_95e0 => extern "thiscall" fn wtr_006995e0(this: u32, out: u32, leftish: u32, value: u32, node: u32) -> u32 {
        let mut frame = StackFrame::<1>::new();
        b::insert_nohint(&mut mem(), this, out, leftish, value, node, frame.at())
    }
    /// `std_TreeIterator_dec_00699f50`: `--it`; `it`.
    std_TreeIterator_dec_00699f50 @ 0x0069_9f50 => extern "thiscall" fn wtr_00699f50(it: u32) -> u32 {
        gv::tree_iterator_dec(&mut mem(), it)
    }
    /// `std_Tree_Lrotate_copy_006a0890`: a copy of `std_Tree_Lrotate_00a41930`.
    std_Tree_Lrotate_copy_006a0890 @ 0x006a_0890 => extern "thiscall" fn wtr_006a0890(this: u32, node: u32) {
        stl::tree_rotate_left(&mut mem(), this, node);
    }
    /// `std_Tree_Rrotate_copy_006a0b50`: a copy of `std_Tree_Rrotate_00a41b40`.
    std_Tree_Rrotate_copy_006a0b50 @ 0x006a_0b50 => extern "thiscall" fn wtr_006a0b50(this: u32, node: u32) {
        stl::tree_rotate_right(&mut mem(), this, node);
    }
    /// `std_Tree_lower_bound_string_006e9920`: the first node whose string key is not below `key`.
    std_Tree_lower_bound_string_006e9920 @ 0x006e_9920 => extern "thiscall" fn wtr_006e9920(this: u32, key: u32) -> u32 {
        b::lower_bound_string(&mut mem(), this, key)
    }
    /// `std_Tree_buyHeadNode_006a0390`: a head node (itself all round, black, nil).
    std_Tree_buyHeadNode_006a0390 @ 0x006a_0390 => extern "C" fn wtr_006a0390() -> u32 {
        b::buy_head_node(&mut mem())
    }
    /// `std_Tree_buyNode2c_006a03c0`: a 0x2c-byte node hung on the head.
    std_Tree_buyNode2c_006a03c0 @ 0x006a_03c0 => extern "thiscall" fn wtr_006a03c0(this: u32) -> u32 {
        b::buy_node(&mut mem(), this, 0x2c)
    }
    /// `std_Tree_buyNode20_006a0400`: a 0x20-byte node hung on the head.
    std_Tree_buyNode20_006a0400 @ 0x006a_0400 => extern "thiscall" fn wtr_006a0400(this: u32) -> u32 {
        b::buy_node(&mut mem(), this, 0x20)
    }
    /// `std_Tree_buyNode2c_006d7930`: a copy of `0x006a03c0`.
    std_Tree_buyNode2c_006d7930 @ 0x006d_7930 => extern "thiscall" fn wtr_006d7930(this: u32) -> u32 {
        b::buy_node(&mut mem(), this, 0x2c)
    }
    /// `std_Tree_buyNodeValue12_0069c5f0`: a node with a 12-byte value.
    std_Tree_buyNodeValue12_0069c5f0 @ 0x0069_c5f0 => extern "thiscall" fn wtr_0069c5f0(this: u32, value: u32) -> u32 {
        b::buy_node_value12(&mut mem(), this, value)
    }
    /// `std_Tree_buyNodeValue12_0069c630`: the same (a copy).
    std_Tree_buyNodeValue12_0069c630 @ 0x0069_c630 => extern "thiscall" fn wtr_0069c630(this: u32, value: u32) -> u32 {
        b::buy_node_value12(&mut mem(), this, value)
    }
    /// `std_Tree_buyNodePrototype_0069c5c0`: a prototype table node, its value moved in.
    std_Tree_buyNodePrototype_0069c5c0 @ 0x0069_c5c0 => extern "thiscall" fn wtr_0069c5c0(this: u32, value: u32) -> u32 {
        b::buy_node_prototype(&mut mem(), this, value)
    }
    /// `VideoIndexMap_buyNode_006d5ca0`: a node of (id, the path as a `std::string`).
    VideoIndexMap_buyNode_006d5ca0 @ 0x006d_5ca0 => extern "thiscall" fn wtr_006d5ca0(this: u32, value: u32) -> u32 {
        b::video_buy_node(&mut mem(), this, value)
    }
    /// `std_Tree_max_0069a450`: the subtree's last node.
    std_Tree_max_0069a450 @ 0x0069_a450 => extern "C" fn wtr_0069a450(node: u32) -> u32 {
        b::tree_max(&mut mem(), node)
    }
    /// `std_Tree_min_0069a470`: the subtree's first node.
    std_Tree_min_0069a470 @ 0x0069_a470 => extern "C" fn wtr_0069a470(node: u32) -> u32 {
        b::tree_min(&mut mem(), node)
    }
    /// `std_Tree_Lrotate_0069a3f0`: the same code as `std_Tree_Lrotate_copy_006a0890`.
    std_Tree_Lrotate_0069a3f0 @ 0x0069_a3f0 => extern "thiscall" fn wtr_0069a3f0(this: u32, node: u32) {
        stl::tree_rotate_left(&mut mem(), this, node);
    }
    /// `std_Tree_Rrotate_0069a4f0`: the same code as `std_Tree_Rrotate_copy_006a0b50`.
    std_Tree_Rrotate_0069a4f0 @ 0x0069_a4f0 => extern "thiscall" fn wtr_0069a4f0(this: u32, node: u32) {
        stl::tree_rotate_right(&mut mem(), this, node);
    }
    /// `VideoIndexMap_insertNode_006d5ff0`: a node hung in by its `i32` id, or freed with its path for an equal one; `out`.
    VideoIndexMap_insertNode_006d5ff0 @ 0x006d_5ff0 => extern "thiscall" fn wtr_006d5ff0(this: u32, out: u32, leftish: u32, value: u32, node: u32) -> u32 {
        let mut frame = StackFrame::<1>::new();
        b::video_insert_node(&mut mem(), this, out, leftish, value, node, frame.at())
    }
}
