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
}
