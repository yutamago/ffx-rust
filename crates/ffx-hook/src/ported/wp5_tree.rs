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
}
