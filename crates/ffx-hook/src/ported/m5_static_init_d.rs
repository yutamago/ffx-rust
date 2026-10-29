//! M5 stage 1, list `static-init-d`: the destructors the static
//! initialisers register with `atexit`. Each shim calls its row of
//! `ffx_pload::m5_static_init_d` (the family tables), so one shim per
//! address; the parity tests are `crates/ffx-oracle/tests/m5_static_init_d.rs`.

use ffx_pload::m5_static_init_d as si;

use super::{StackFrame, mem, ported};

ported! {
    /// `StaticDtor_Tail_00b073c0`: `ecx = 0x00cd_ece0`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b073c0 @ 0x00b0_73c0 => extern "C" fn s_00b073c0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[0])
    }
    /// `StaticDtor_Tail_00b073d0`: `ecx = 0x00cd_ecc0`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b073d0 @ 0x00b0_73d0 => extern "C" fn s_00b073d0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[1])
    }
    /// `StaticDtor_Tail_00b073e0`: `ecx = 0x00cd_eca0`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b073e0 @ 0x00b0_73e0 => extern "C" fn s_00b073e0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[2])
    }
    /// `StaticDtor_Tail_00b073f0`: `ecx = 0x00cd_ed20`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b073f0 @ 0x00b0_73f0 => extern "C" fn s_00b073f0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[3])
    }
    /// `StaticDtor_Tail_00b07430`: `ecx = 0x00cd_ed44`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b07430 @ 0x00b0_7430 => extern "C" fn s_00b07430() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[4])
    }
    /// `StaticDtor_Tail_00b074d0`: `ecx = 0x00cd_ee20`, then a tail-jump to `Phyre_PCluster_dtor_PCluster`.
    StaticDtor_Tail_00b074d0 @ 0x00b0_74d0 => extern "C" fn s_00b074d0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[5])
    }
    /// `StaticDtor_PoolTree_00b074e0`: the pooled tree at `0x00c3_4b4c` (orphan all, the pool's reset and destructor, clear twice).
    StaticDtor_PoolTree_00b074e0 @ 0x00b0_74e0 => extern "C" fn s_00b074e0() -> u32 {
        si::pool_tree_dtor(&mut mem(), &si::POOL_TREE_DTORS[0])
    }
    /// `StaticDtor_PoolTree_00b07560`: the pooled tree at `0x00c3_4b1c` (orphan all, the pool's reset and destructor, clear twice).
    StaticDtor_PoolTree_00b07560 @ 0x00b0_7560 => extern "C" fn s_00b07560() -> u32 {
        si::pool_tree_dtor(&mut mem(), &si::POOL_TREE_DTORS[1])
    }
    /// `StaticDtor_OwnedBuffer_00b07640`: the buffer `{flags, data}` at `0x00cd_efa4`: owned data freed, both words cleared.
    StaticDtor_OwnedBuffer_00b07640 @ 0x00b0_7640 => extern "C" fn s_00b07640() -> u32 {
        si::owned_buffer(&mut mem(), &si::OWNED_BUFFERS[0])
    }
    /// `StaticDtor_Tail_00b07740`: `ecx = 0x00ce_6d70`, then a tail-jump to `Phyre_PCluster_dtor_PCluster`.
    StaticDtor_Tail_00b07740 @ 0x00b0_7740 => extern "C" fn s_00b07740() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[6])
    }
    /// `StaticDtor_PoolTree_00b07750`: the pooled tree at `0x00c3_4ffc` (orphan all, the pool's reset and destructor, clear twice).
    StaticDtor_PoolTree_00b07750 @ 0x00b0_7750 => extern "C" fn s_00b07750() -> u32 {
        si::pool_tree_dtor(&mut mem(), &si::POOL_TREE_DTORS[2])
    }
    /// `StaticDtor_PoolTree_00b077d0`: the pooled tree at `0x00c3_4fcc` (orphan all, the pool's reset and destructor, clear twice).
    StaticDtor_PoolTree_00b077d0 @ 0x00b0_77d0 => extern "C" fn s_00b077d0() -> u32 {
        si::pool_tree_dtor(&mut mem(), &si::POOL_TREE_DTORS[3])
    }
    /// `StaticDtor_Tail_00b078b0`: `ecx = 0x00ce_6e94`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b078b0 @ 0x00b0_78b0 => extern "C" fn s_00b078b0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[7])
    }
    /// `StaticDtor_Map_00b07920`: the `std::map` at `0x00ce_6eb0` erased (`0x0063_6d60`), its head deleted; the erase's out iterator in the shim's frame.
    StaticDtor_Map_00b07920 @ 0x00b0_7920 => extern "C" fn s_00b07920() -> u32 {
        let mut f = StackFrame::<1>::new();
        si::map_dtor(&mut mem(), &si::MAP_DTORS[0], f.at())
    }
    /// `StaticDtor_Tail_00b07a00`: `ecx = 0x00ce_8208`, then a tail-jump to `Phyre_PRendering_PSamplerStateD3D11_dtor`.
    StaticDtor_Tail_00b07a00 @ 0x00b0_7a00 => extern "C" fn s_00b07a00() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[8])
    }
    /// `StaticDtor_Tail_00b07a10`: `ecx = 0x00ce_822c`, then a tail-jump to `Phyre_PRendering_PSamplerStateD3D11_dtor`.
    StaticDtor_Tail_00b07a10 @ 0x00b0_7a10 => extern "C" fn s_00b07a10() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[9])
    }
    /// `StaticDtor_Map_00b07c20`: the `std::map` at `0x012f_4104` erased (`0x0080_7510`), its head deleted; the erase's out iterator in the shim's frame.
    StaticDtor_Map_00b07c20 @ 0x00b0_7c20 => extern "C" fn s_00b07c20() -> u32 {
        let mut f = StackFrame::<1>::new();
        si::map_dtor(&mut mem(), &si::MAP_DTORS[1], f.at())
    }
    /// `StaticDtor_Map_00b07ca0`: the `std::map` at `0x012f_40f8` erased (`0x0080_7210`), its head deleted; the erase's out iterator in the shim's frame.
    StaticDtor_Map_00b07ca0 @ 0x00b0_7ca0 => extern "C" fn s_00b07ca0() -> u32 {
        let mut f = StackFrame::<1>::new();
        si::map_dtor(&mut mem(), &si::MAP_DTORS[2], f.at())
    }
    /// `StaticDtor_CloseHandle_00b07d20`: `CloseHandle` of the handle at `0x012f_410c`, through its import slot.
    StaticDtor_CloseHandle_00b07d20 @ 0x00b0_7d20 => extern "C" fn s_00b07d20() -> u32 {
        si::import_call(&mut mem(), &si::IMPORT_CALLS[0])
    }
    /// `StaticDtor_CondTail_00b07d30`: `ecx = [0x0193_cd2c]` (`g_MemSystem`); a tail-jump to `0x0094_5b80` when it is not 0.
    StaticDtor_CondTail_00b07d30 @ 0x00b0_7d30 => extern "C" fn s_00b07d30() -> u32 {
        si::cond_tail_load(&mut mem(), &si::COND_TAIL_LOADS[0])
    }
    /// `StaticDtor_Tail_00b07d3f`: `ecx = 0x0193_ed2c`, then a tail-jump to `0x0094_a641`.
    StaticDtor_Tail_00b07d3f @ 0x00b0_7d3f => extern "C" fn s_00b07d3f() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[10])
    }
    /// `StaticDtor_ChainUnlink_00b07d50`: the object at `0x00c6_13f0` gets `Phyre_PUtility_vftable` back and leaves the chain at `0x00c9_01a8`.
    StaticDtor_ChainUnlink_00b07d50 @ 0x00b0_7d50 => extern "C" fn s_00b07d50() -> u32 {
        si::chain_unlink(&mut mem(), &si::CHAIN_UNLINKS[0])
    }
    /// `StaticDtor_Tail_00b07dba`: `ecx = 0x0193_fcc4`, then a tail-jump to `0x009d_9cc1`.
    StaticDtor_Tail_00b07dba @ 0x00b0_7dba => extern "C" fn s_00b07dba() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[11])
    }
    /// `StaticDtor_Tail_00b07dc4`: `ecx = 0x0193_fcfc`, then a tail-jump to `0x009d_a05c`.
    StaticDtor_Tail_00b07dc4 @ 0x00b0_7dc4 => extern "C" fn s_00b07dc4() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[12])
    }
    /// `StaticDtor_StoreVt_00b07dce`: `[0x00c6_4bf0] = 0x00b7_02b4`.
    StaticDtor_StoreVt_00b07dce @ 0x00b0_7dce => extern "C" fn s_00b07dce() {
        si::store_vt(&mut mem(), &si::STORE_VTS[0]);
    }
    /// `StaticDtor_ChainUnlink_00b08330`: the object at `0x00c6_81ec` gets `Phyre_PUtility_vftable` back and leaves the chain at `0x00c9_01a8`.
    StaticDtor_ChainUnlink_00b08330 @ 0x00b0_8330 => extern "C" fn s_00b08330() -> u32 {
        si::chain_unlink(&mut mem(), &si::CHAIN_UNLINKS[1])
    }
    /// `StaticDtor_PInputTypeSemantic_00b08380`: the `PInputTypeSemantic` value at `0x00c6_8240` gets its vtable back, then `0x009f_49f0(obj)` unlinks it.
    StaticDtor_PInputTypeSemantic_00b08380 @ 0x00b0_8380 => extern "C" fn s_00b08380() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[0])
    }
    /// `StaticDtor_PInputTypeSemantic_00b083a0`: the `PInputTypeSemantic` value at `0x00c6_8234` gets its vtable back, then `0x009f_49f0(obj)` unlinks it.
    StaticDtor_PInputTypeSemantic_00b083a0 @ 0x00b0_83a0 => extern "C" fn s_00b083a0() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[1])
    }
    /// `StaticDtor_PInputTypeSemantic_00b083c0`: the `PInputTypeSemantic` value at `0x00c6_8228` gets its vtable back, then `0x009f_49f0(obj)` unlinks it.
    StaticDtor_PInputTypeSemantic_00b083c0 @ 0x00b0_83c0 => extern "C" fn s_00b083c0() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[2])
    }
    /// `StaticDtor_PInputTypeSemantic_00b083e0`: the `PInputTypeSemantic` value at `0x00c6_82e8` gets its vtable back, then `0x009f_49f0(obj)` unlinks it.
    StaticDtor_PInputTypeSemantic_00b083e0 @ 0x00b0_83e0 => extern "C" fn s_00b083e0() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[3])
    }
    /// `StaticDtor_PInputTypeSemantic_00b08400`: the `PInputTypeSemantic` value at `0x00c6_82f4` gets its vtable back, then `0x009f_49f0(obj)` unlinks it.
    StaticDtor_PInputTypeSemantic_00b08400 @ 0x00b0_8400 => extern "C" fn s_00b08400() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[4])
    }
    /// `StaticDtor_PInputTypeSemantic_00b08420`: the `PInputTypeSemantic` value at `0x00c6_8300` gets its vtable back, then `0x009f_49f0(obj)` unlinks it.
    StaticDtor_PInputTypeSemantic_00b08420 @ 0x00b0_8420 => extern "C" fn s_00b08420() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[5])
    }
    /// `StaticDtor_PInputTypeSemantic_00b08440`: the `PInputTypeSemantic` value at `0x00c6_830c` gets its vtable back, then `0x009f_49f0(obj)` unlinks it.
    StaticDtor_PInputTypeSemantic_00b08440 @ 0x00b0_8440 => extern "C" fn s_00b08440() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[6])
    }
    /// `StaticDtor_PInputTypeSemantic_00b08460`: the `PInputTypeSemantic` value at `0x00c6_8318` gets its vtable back, then `0x009f_49f0(obj)` unlinks it.
    StaticDtor_PInputTypeSemantic_00b08460 @ 0x00b0_8460 => extern "C" fn s_00b08460() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[7])
    }
    /// `StaticDtor_PInputTypeSemantic_00b08480`: the `PInputTypeSemantic` value at `0x00c6_8324` gets its vtable back, then `0x009f_49f0(obj)` unlinks it.
    StaticDtor_PInputTypeSemantic_00b08480 @ 0x00b0_8480 => extern "C" fn s_00b08480() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[8])
    }
    /// `StaticDtor_PInputTypeSemantic_00b084a0`: the `PInputTypeSemantic` value at `0x00c6_82dc` gets its vtable back, then `0x009f_49f0(obj)` unlinks it.
    StaticDtor_PInputTypeSemantic_00b084a0 @ 0x00b0_84a0 => extern "C" fn s_00b084a0() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[9])
    }
    /// `StaticDtor_PInputTypeSemantic_00b084c0`: the `PInputTypeSemantic` value at `0x00c6_82b8` gets its vtable back, then `0x009f_49f0(obj)` unlinks it.
    StaticDtor_PInputTypeSemantic_00b084c0 @ 0x00b0_84c0 => extern "C" fn s_00b084c0() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[10])
    }
    /// `StaticDtor_PInputTypeSemantic_00b084e0`: the `PInputTypeSemantic` value at `0x00c6_82c4` gets its vtable back, then `0x009f_49f0(obj)` unlinks it.
    StaticDtor_PInputTypeSemantic_00b084e0 @ 0x00b0_84e0 => extern "C" fn s_00b084e0() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[11])
    }
    /// `StaticDtor_PInputTypeSemantic_00b08500`: the `PInputTypeSemantic` value at `0x00c6_82d0` gets its vtable back, then `0x009f_49f0(obj)` unlinks it.
    StaticDtor_PInputTypeSemantic_00b08500 @ 0x00b0_8500 => extern "C" fn s_00b08500() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[12])
    }
    /// `StaticDtor_PInputTypeSemantic_00b08520`: the `PInputTypeSemantic` value at `0x00c6_824c` gets its vtable back, then `0x009f_49f0(obj)` unlinks it.
    StaticDtor_PInputTypeSemantic_00b08520 @ 0x00b0_8520 => extern "C" fn s_00b08520() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[13])
    }
    /// `StaticDtor_PInputTypeSemantic_00b08540`: the `PInputTypeSemantic` value at `0x00c6_8258` gets its vtable back, then `0x009f_49f0(obj)` unlinks it.
    StaticDtor_PInputTypeSemantic_00b08540 @ 0x00b0_8540 => extern "C" fn s_00b08540() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[14])
    }
    /// `StaticDtor_PInputTypeSemantic_00b08560`: the `PInputTypeSemantic` value at `0x00c6_8264` gets its vtable back, then `0x009f_49f0(obj)` unlinks it.
    StaticDtor_PInputTypeSemantic_00b08560 @ 0x00b0_8560 => extern "C" fn s_00b08560() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[15])
    }
    /// `StaticDtor_PInputTypeSemantic_00b08580`: the `PInputTypeSemantic` value at `0x00c6_8270` gets its vtable back, then `0x009f_49f0(obj)` unlinks it.
    StaticDtor_PInputTypeSemantic_00b08580 @ 0x00b0_8580 => extern "C" fn s_00b08580() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[16])
    }
    /// `StaticDtor_PInputTypeSemantic_00b085a0`: the `PInputTypeSemantic` value at `0x00c6_827c` gets its vtable back, then `0x009f_49f0(obj)` unlinks it.
    StaticDtor_PInputTypeSemantic_00b085a0 @ 0x00b0_85a0 => extern "C" fn s_00b085a0() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[17])
    }
    /// `StaticDtor_PInputTypeSemantic_00b085c0`: the `PInputTypeSemantic` value at `0x00c6_82a0` gets its vtable back, then `0x009f_49f0(obj)` unlinks it.
    StaticDtor_PInputTypeSemantic_00b085c0 @ 0x00b0_85c0 => extern "C" fn s_00b085c0() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[18])
    }
    /// `StaticDtor_PInputTypeSemantic_00b085e0`: the `PInputTypeSemantic` value at `0x00c6_82ac` gets its vtable back, then `0x009f_49f0(obj)` unlinks it.
    StaticDtor_PInputTypeSemantic_00b085e0 @ 0x00b0_85e0 => extern "C" fn s_00b085e0() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[19])
    }
    /// `StaticDtor_PInputTypeSemantic_00b08600`: the `PInputTypeSemantic` value at `0x00c6_8288` gets its vtable back, then `0x009f_49f0(obj)` unlinks it.
    StaticDtor_PInputTypeSemantic_00b08600 @ 0x00b0_8600 => extern "C" fn s_00b08600() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[20])
    }
    /// `StaticDtor_PInputTypeSemantic_00b08620`: the `PInputTypeSemantic` value at `0x00c6_8294` gets its vtable back, then `0x009f_49f0(obj)` unlinks it.
    StaticDtor_PInputTypeSemantic_00b08620 @ 0x00b0_8620 => extern "C" fn s_00b08620() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[21])
    }
    /// `StaticDtor_ChainUnlink_00b08650`: the object at `0x00c6_845c` gets `Phyre_PUtility_vftable` back and leaves the chain at `0x00c9_01a8`.
    StaticDtor_ChainUnlink_00b08650 @ 0x00b0_8650 => extern "C" fn s_00b08650() -> u32 {
        si::chain_unlink(&mut mem(), &si::CHAIN_UNLINKS[2])
    }
    /// `StaticDtor_Tail_00b086a0`: `ecx = 0x0194_3da0`, then a tail-jump to `Phyre_PCluster_dtor_PCluster`.
    StaticDtor_Tail_00b086a0 @ 0x00b0_86a0 => extern "C" fn s_00b086a0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[13])
    }
}
