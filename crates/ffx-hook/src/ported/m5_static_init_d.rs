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
    /// `StaticDtor_Tail_00b08ab0`: `ecx = 0x0194_4d04`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b08ab0 @ 0x00b0_8ab0 => extern "C" fn s_00b08ab0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[14])
    }
    /// `StaticDtor_Tail_00b08ac0`: `ecx = 0x0194_4d44`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b08ac0 @ 0x00b0_8ac0 => extern "C" fn s_00b08ac0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[15])
    }
    /// `StaticDtor_Tail_00b08ad0`: `ecx = 0x0194_4d14`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b08ad0 @ 0x00b0_8ad0 => extern "C" fn s_00b08ad0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[16])
    }
    /// `StaticDtor_Tail_00b08ae0`: `ecx = 0x0194_4f28`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b08ae0 @ 0x00b0_8ae0 => extern "C" fn s_00b08ae0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[17])
    }
    /// `StaticDtor_Tail_00b08af0`: `ecx = 0x0194_4c0c`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b08af0 @ 0x00b0_8af0 => extern "C" fn s_00b08af0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[18])
    }
    /// `StaticDtor_Tail_00b08b00`: `ecx = 0x0194_4bfc`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b08b00 @ 0x00b0_8b00 => extern "C" fn s_00b08b00() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[19])
    }
    /// `StaticDtor_Tail_00b08b10`: `ecx = 0x0194_4d34`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b08b10 @ 0x00b0_8b10 => extern "C" fn s_00b08b10() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[20])
    }
    /// `StaticDtor_Tail_00b08b20`: `ecx = 0x0194_4b88`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b08b20 @ 0x00b0_8b20 => extern "C" fn s_00b08b20() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[21])
    }
    /// `StaticDtor_Tail_00b08b30`: `ecx = 0x0194_4e7c`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b08b30 @ 0x00b0_8b30 => extern "C" fn s_00b08b30() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[22])
    }
    /// `StaticDtor_Tail_00b08b40`: `ecx = 0x0194_4db4`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b08b40 @ 0x00b0_8b40 => extern "C" fn s_00b08b40() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[23])
    }
    /// `StaticDtor_Tail_00b08b50`: `ecx = 0x0194_4934`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b08b50 @ 0x00b0_8b50 => extern "C" fn s_00b08b50() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[24])
    }
    /// `StaticDtor_Tail_00b08b60`: `ecx = 0x0194_4f18`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b08b60 @ 0x00b0_8b60 => extern "C" fn s_00b08b60() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[25])
    }
    /// `StaticDtor_Tail_00b08b70`: `ecx = 0x0194_4cf4`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b08b70 @ 0x00b0_8b70 => extern "C" fn s_00b08b70() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[26])
    }
    /// `StaticDtor_Tail_00b08b80`: `ecx = 0x0194_4d24`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b08b80 @ 0x00b0_8b80 => extern "C" fn s_00b08b80() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[27])
    }
    /// `StaticDtor_Tail_00b08b90`: `ecx = 0x0194_35d8`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b08b90 @ 0x00b0_8b90 => extern "C" fn s_00b08b90() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[28])
    }
    /// `StaticDtor_Tail_00b08ba0`: `ecx = 0x0194_49b4`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b08ba0 @ 0x00b0_8ba0 => extern "C" fn s_00b08ba0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[29])
    }
    /// `StaticDtor_Tail_00b08bb0`: `ecx = 0x0194_4c4c`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b08bb0 @ 0x00b0_8bb0 => extern "C" fn s_00b08bb0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[30])
    }
    /// `StaticDtor_Tail_00b08bc0`: `ecx = 0x0194_4c2c`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b08bc0 @ 0x00b0_8bc0 => extern "C" fn s_00b08bc0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[31])
    }
    /// `StaticDtor_Tail_00b08bd0`: `ecx = 0x0194_4c3c`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b08bd0 @ 0x00b0_8bd0 => extern "C" fn s_00b08bd0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[32])
    }
    /// `StaticDtor_Tail_00b08be0`: `ecx = 0x0194_4c1c`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b08be0 @ 0x00b0_8be0 => extern "C" fn s_00b08be0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[33])
    }
    /// `StaticDtor_Tail_00b08bf0`: `ecx = 0x0194_4d94`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b08bf0 @ 0x00b0_8bf0 => extern "C" fn s_00b08bf0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[34])
    }
    /// `StaticDtor_Tail_00b08c00`: `ecx = 0x0194_4ce4`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b08c00 @ 0x00b0_8c00 => extern "C" fn s_00b08c00() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[35])
    }
    /// `StaticDtor_Tail_00b08c10`: `ecx = 0x0194_4d84`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b08c10 @ 0x00b0_8c10 => extern "C" fn s_00b08c10() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[36])
    }
    /// `StaticDtor_Tail_00b08c20`: `ecx = 0x0194_4bec`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b08c20 @ 0x00b0_8c20 => extern "C" fn s_00b08c20() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[37])
    }
    /// `StaticDtor_Tail_00b08c30`: `ecx = 0x0194_4954`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b08c30 @ 0x00b0_8c30 => extern "C" fn s_00b08c30() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[38])
    }
    /// `StaticDtor_Tail_00b08c40`: `ecx = 0x0194_4964`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b08c40 @ 0x00b0_8c40 => extern "C" fn s_00b08c40() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[39])
    }
    /// `StaticDtor_Tail_00b08c50`: `ecx = 0x0194_4974`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b08c50 @ 0x00b0_8c50 => extern "C" fn s_00b08c50() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[40])
    }
    /// `StaticDtor_Tail_00b08c60`: `ecx = 0x0194_49c4`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b08c60 @ 0x00b0_8c60 => extern "C" fn s_00b08c60() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[41])
    }
    /// `StaticDtor_Tail_00b08c70`: `ecx = 0x0194_4e4c`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b08c70 @ 0x00b0_8c70 => extern "C" fn s_00b08c70() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[42])
    }
    /// `StaticDtor_Tail_00b08c80`: `ecx = 0x0194_4e9c`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b08c80 @ 0x00b0_8c80 => extern "C" fn s_00b08c80() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[43])
    }
    /// `StaticDtor_Tail_00b08c90`: `ecx = 0x0194_4e8c`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b08c90 @ 0x00b0_8c90 => extern "C" fn s_00b08c90() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[44])
    }
    /// `StaticDtor_Tail_00b08ca0`: `ecx = 0x0194_4e6c`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b08ca0 @ 0x00b0_8ca0 => extern "C" fn s_00b08ca0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[45])
    }
    /// `StaticDtor_Tail_00b08cb0`: `ecx = 0x0194_4e5c`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b08cb0 @ 0x00b0_8cb0 => extern "C" fn s_00b08cb0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[46])
    }
    /// `StaticDtor_Tail_00b08cc0`: `ecx = 0x0194_4994`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b08cc0 @ 0x00b0_8cc0 => extern "C" fn s_00b08cc0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[47])
    }
    /// `StaticDtor_Tail_00b08cd0`: `ecx = 0x0194_4984`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b08cd0 @ 0x00b0_8cd0 => extern "C" fn s_00b08cd0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[48])
    }
    /// `StaticDtor_Tail_00b08ce0`: `ecx = 0x0194_4944`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b08ce0 @ 0x00b0_8ce0 => extern "C" fn s_00b08ce0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[49])
    }
    /// `StaticDtor_Tail_00b08cf0`: `ecx = 0x0194_49a4`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b08cf0 @ 0x00b0_8cf0 => extern "C" fn s_00b08cf0() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[50])
    }
    /// `StaticDtor_Tail_00b08d00`: `ecx = 0x0194_4bcc`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b08d00 @ 0x00b0_8d00 => extern "C" fn s_00b08d00() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[51])
    }
    /// `StaticDtor_Tail_00b08d10`: `ecx = 0x0194_49d4`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b08d10 @ 0x00b0_8d10 => extern "C" fn s_00b08d10() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[52])
    }
    /// `StaticDtor_Tail_00b08d20`: `ecx = 0x0194_49e4`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b08d20 @ 0x00b0_8d20 => extern "C" fn s_00b08d20() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[53])
    }
    /// `StaticDtor_Tail_00b08d30`: `ecx = 0x0194_4bdc`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b08d30 @ 0x00b0_8d30 => extern "C" fn s_00b08d30() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[54])
    }
    /// `StaticDtor_Tail_00b08d40`: `ecx = 0x0194_4da4`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b08d40 @ 0x00b0_8d40 => extern "C" fn s_00b08d40() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[55])
    }
    /// `StaticDtor_Tail_00b08d50`: `ecx = 0x0194_4f08`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b08d50 @ 0x00b0_8d50 => extern "C" fn s_00b08d50() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[56])
    }
    /// `StaticDtor_Tail_00b08d60`: `ecx = 0x0194_4bb8`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b08d60 @ 0x00b0_8d60 => extern "C" fn s_00b08d60() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[57])
    }
    /// `StaticDtor_Tail_00b08d70`: `ecx = 0x0194_4b98`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b08d70 @ 0x00b0_8d70 => extern "C" fn s_00b08d70() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[58])
    }
    /// `StaticDtor_Tail_00b08d80`: `ecx = 0x0194_4ba8`, then a tail-jump to `0x004d_bab0`.
    StaticDtor_Tail_00b08d80 @ 0x00b0_8d80 => extern "C" fn s_00b08d80() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[59])
    }
    /// `StaticDtor_ChainUnlink_00b09010`: the object at `0x00c6_8494` gets `Phyre_PUtility_vftable` back and leaves the chain at `0x00c9_01a8`.
    StaticDtor_ChainUnlink_00b09010 @ 0x00b0_9010 => extern "C" fn s_00b09010() -> u32 {
        si::chain_unlink(&mut mem(), &si::CHAIN_UNLINKS[3])
    }
    /// `StaticDtor_PInputAxisSemantic_00b09220`: the `PInputAxisSemantic` value at `0x00c6_b6b4` gets its vtable back, then `0x00a2_dfa0(obj)` unlinks it.
    StaticDtor_PInputAxisSemantic_00b09220 @ 0x00b0_9220 => extern "C" fn s_00b09220() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[22])
    }
    /// `StaticDtor_PInputAxisSemantic_00b09240`: the `PInputAxisSemantic` value at `0x00c6_b744` gets its vtable back, then `0x00a2_dfa0(obj)` unlinks it.
    StaticDtor_PInputAxisSemantic_00b09240 @ 0x00b0_9240 => extern "C" fn s_00b09240() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[23])
    }
    /// `StaticDtor_PInputAxisSemantic_00b09260`: the `PInputAxisSemantic` value at `0x00c6_b714` gets its vtable back, then `0x00a2_dfa0(obj)` unlinks it.
    StaticDtor_PInputAxisSemantic_00b09260 @ 0x00b0_9260 => extern "C" fn s_00b09260() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[24])
    }
    /// `StaticDtor_PInputAxisSemantic_00b09280`: the `PInputAxisSemantic` value at `0x00c6_b6e4` gets its vtable back, then `0x00a2_dfa0(obj)` unlinks it.
    StaticDtor_PInputAxisSemantic_00b09280 @ 0x00b0_9280 => extern "C" fn s_00b09280() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[25])
    }
    /// `StaticDtor_PInputAxisSemantic_00b092a0`: the `PInputAxisSemantic` value at `0x00c6_b734` gets its vtable back, then `0x00a2_dfa0(obj)` unlinks it.
    StaticDtor_PInputAxisSemantic_00b092a0 @ 0x00b0_92a0 => extern "C" fn s_00b092a0() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[26])
    }
    /// `StaticDtor_PInputAxisSemantic_00b092c0`: the `PInputAxisSemantic` value at `0x00c6_b704` gets its vtable back, then `0x00a2_dfa0(obj)` unlinks it.
    StaticDtor_PInputAxisSemantic_00b092c0 @ 0x00b0_92c0 => extern "C" fn s_00b092c0() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[27])
    }
    /// `StaticDtor_PInputAxisSemantic_00b092e0`: the `PInputAxisSemantic` value at `0x00c6_b6d4` gets its vtable back, then `0x00a2_dfa0(obj)` unlinks it.
    StaticDtor_PInputAxisSemantic_00b092e0 @ 0x00b0_92e0 => extern "C" fn s_00b092e0() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[28])
    }
    /// `StaticDtor_PInputAxisSemantic_00b09300`: the `PInputAxisSemantic` value at `0x00c6_b724` gets its vtable back, then `0x00a2_dfa0(obj)` unlinks it.
    StaticDtor_PInputAxisSemantic_00b09300 @ 0x00b0_9300 => extern "C" fn s_00b09300() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[29])
    }
    /// `StaticDtor_PInputAxisSemantic_00b09320`: the `PInputAxisSemantic` value at `0x00c6_b6f4` gets its vtable back, then `0x00a2_dfa0(obj)` unlinks it.
    StaticDtor_PInputAxisSemantic_00b09320 @ 0x00b0_9320 => extern "C" fn s_00b09320() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[30])
    }
    /// `StaticDtor_PInputAxisSemantic_00b09340`: the `PInputAxisSemantic` value at `0x00c6_b6c4` gets its vtable back, then `0x00a2_dfa0(obj)` unlinks it.
    StaticDtor_PInputAxisSemantic_00b09340 @ 0x00b0_9340 => extern "C" fn s_00b09340() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[31])
    }
    /// `StaticDtor_PInputJoypadButtonSemantic_00b09370`: the `PInputJoypadButtonSemantic` value at `0x00c6_bb34` gets its vtable back, then `0x00a2_e410(obj)` unlinks it.
    StaticDtor_PInputJoypadButtonSemantic_00b09370 @ 0x00b0_9370 => extern "C" fn s_00b09370() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[32])
    }
    /// `StaticDtor_PInputJoypadButtonSemantic_00b09390`: the `PInputJoypadButtonSemantic` value at `0x00c6_b964` gets its vtable back, then `0x00a2_e410(obj)` unlinks it.
    StaticDtor_PInputJoypadButtonSemantic_00b09390 @ 0x00b0_9390 => extern "C" fn s_00b09390() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[33])
    }
    /// `StaticDtor_PInputJoypadButtonSemantic_00b093b0`: the `PInputJoypadButtonSemantic` value at `0x00c6_b934` gets its vtable back, then `0x00a2_e410(obj)` unlinks it.
    StaticDtor_PInputJoypadButtonSemantic_00b093b0 @ 0x00b0_93b0 => extern "C" fn s_00b093b0() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[34])
    }
    /// `StaticDtor_PInputJoypadButtonSemantic_00b093d0`: the `PInputJoypadButtonSemantic` value at `0x00c6_b904` gets its vtable back, then `0x00a2_e410(obj)` unlinks it.
    StaticDtor_PInputJoypadButtonSemantic_00b093d0 @ 0x00b0_93d0 => extern "C" fn s_00b093d0() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[35])
    }
    /// `StaticDtor_PInputJoypadButtonSemantic_00b093f0`: the `PInputJoypadButtonSemantic` value at `0x00c6_b8e4` gets its vtable back, then `0x00a2_e410(obj)` unlinks it.
    StaticDtor_PInputJoypadButtonSemantic_00b093f0 @ 0x00b0_93f0 => extern "C" fn s_00b093f0() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[36])
    }
    /// `StaticDtor_PInputJoypadButtonSemantic_00b09410`: the `PInputJoypadButtonSemantic` value at `0x00c6_b8c4` gets its vtable back, then `0x00a2_e410(obj)` unlinks it.
    StaticDtor_PInputJoypadButtonSemantic_00b09410 @ 0x00b0_9410 => extern "C" fn s_00b09410() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[37])
    }
    /// `StaticDtor_PInputJoypadButtonSemantic_00b09430`: the `PInputJoypadButtonSemantic` value at `0x00c6_b8a4` gets its vtable back, then `0x00a2_e410(obj)` unlinks it.
    StaticDtor_PInputJoypadButtonSemantic_00b09430 @ 0x00b0_9430 => extern "C" fn s_00b09430() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[38])
    }
    /// `StaticDtor_PInputJoypadButtonSemantic_00b09450`: the `PInputJoypadButtonSemantic` value at `0x00c6_bb04` gets its vtable back, then `0x00a2_e410(obj)` unlinks it.
    StaticDtor_PInputJoypadButtonSemantic_00b09450 @ 0x00b0_9450 => extern "C" fn s_00b09450() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[39])
    }
    /// `StaticDtor_PInputJoypadButtonSemantic_00b09470`: the `PInputJoypadButtonSemantic` value at `0x00c6_bad4` gets its vtable back, then `0x00a2_e410(obj)` unlinks it.
    StaticDtor_PInputJoypadButtonSemantic_00b09470 @ 0x00b0_9470 => extern "C" fn s_00b09470() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[40])
    }
    /// `StaticDtor_PInputJoypadButtonSemantic_00b09490`: the `PInputJoypadButtonSemantic` value at `0x00c6_baa4` gets its vtable back, then `0x00a2_e410(obj)` unlinks it.
    StaticDtor_PInputJoypadButtonSemantic_00b09490 @ 0x00b0_9490 => extern "C" fn s_00b09490() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[41])
    }
    /// `StaticDtor_PInputJoypadButtonSemantic_00b094b0`: the `PInputJoypadButtonSemantic` value at `0x00c6_ba74` gets its vtable back, then `0x00a2_e410(obj)` unlinks it.
    StaticDtor_PInputJoypadButtonSemantic_00b094b0 @ 0x00b0_94b0 => extern "C" fn s_00b094b0() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[42])
    }
    /// `StaticDtor_PInputJoypadButtonSemantic_00b094d0`: the `PInputJoypadButtonSemantic` value at `0x00c6_ba44` gets its vtable back, then `0x00a2_e410(obj)` unlinks it.
    StaticDtor_PInputJoypadButtonSemantic_00b094d0 @ 0x00b0_94d0 => extern "C" fn s_00b094d0() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[43])
    }
    /// `StaticDtor_PInputJoypadButtonSemantic_00b094f0`: the `PInputJoypadButtonSemantic` value at `0x00c6_ba14` gets its vtable back, then `0x00a2_e410(obj)` unlinks it.
    StaticDtor_PInputJoypadButtonSemantic_00b094f0 @ 0x00b0_94f0 => extern "C" fn s_00b094f0() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[44])
    }
    /// `StaticDtor_PInputJoypadButtonSemantic_00b09510`: the `PInputJoypadButtonSemantic` value at `0x00c6_b9e4` gets its vtable back, then `0x00a2_e410(obj)` unlinks it.
    StaticDtor_PInputJoypadButtonSemantic_00b09510 @ 0x00b0_9510 => extern "C" fn s_00b09510() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[45])
    }
    /// `StaticDtor_PInputJoypadButtonSemantic_00b09530`: the `PInputJoypadButtonSemantic` value at `0x00c6_b9b4` gets its vtable back, then `0x00a2_e410(obj)` unlinks it.
    StaticDtor_PInputJoypadButtonSemantic_00b09530 @ 0x00b0_9530 => extern "C" fn s_00b09530() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[46])
    }
    /// `StaticDtor_PInputJoypadButtonSemantic_00b09550`: the `PInputJoypadButtonSemantic` value at `0x00c6_b984` gets its vtable back, then `0x00a2_e410(obj)` unlinks it.
    StaticDtor_PInputJoypadButtonSemantic_00b09550 @ 0x00b0_9550 => extern "C" fn s_00b09550() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[47])
    }
    /// `StaticDtor_PInputJoypadButtonSemantic_00b09570`: the `PInputJoypadButtonSemantic` value at `0x00c6_bae4` gets its vtable back, then `0x00a2_e410(obj)` unlinks it.
    StaticDtor_PInputJoypadButtonSemantic_00b09570 @ 0x00b0_9570 => extern "C" fn s_00b09570() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[48])
    }
    /// `StaticDtor_PInputJoypadButtonSemantic_00b09590`: the `PInputJoypadButtonSemantic` value at `0x00c6_bab4` gets its vtable back, then `0x00a2_e410(obj)` unlinks it.
    StaticDtor_PInputJoypadButtonSemantic_00b09590 @ 0x00b0_9590 => extern "C" fn s_00b09590() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[49])
    }
    /// `StaticDtor_PInputJoypadButtonSemantic_00b095b0`: the `PInputJoypadButtonSemantic` value at `0x00c6_b994` gets its vtable back, then `0x00a2_e410(obj)` unlinks it.
    StaticDtor_PInputJoypadButtonSemantic_00b095b0 @ 0x00b0_95b0 => extern "C" fn s_00b095b0() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[50])
    }
    /// `StaticDtor_PInputJoypadButtonSemantic_00b095d0`: the `PInputJoypadButtonSemantic` value at `0x00c6_bac4` gets its vtable back, then `0x00a2_e410(obj)` unlinks it.
    StaticDtor_PInputJoypadButtonSemantic_00b095d0 @ 0x00b0_95d0 => extern "C" fn s_00b095d0() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[51])
    }
    /// `StaticDtor_PInputJoypadButtonSemantic_00b095f0`: the `PInputJoypadButtonSemantic` value at `0x00c6_baf4` gets its vtable back, then `0x00a2_e410(obj)` unlinks it.
    StaticDtor_PInputJoypadButtonSemantic_00b095f0 @ 0x00b0_95f0 => extern "C" fn s_00b095f0() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[52])
    }
    /// `StaticDtor_PInputJoypadButtonSemantic_00b09610`: the `PInputJoypadButtonSemantic` value at `0x00c6_b8b4` gets its vtable back, then `0x00a2_e410(obj)` unlinks it.
    StaticDtor_PInputJoypadButtonSemantic_00b09610 @ 0x00b0_9610 => extern "C" fn s_00b09610() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[53])
    }
    /// `StaticDtor_PInputJoypadButtonSemantic_00b09630`: the `PInputJoypadButtonSemantic` value at `0x00c6_ba64` gets its vtable back, then `0x00a2_e410(obj)` unlinks it.
    StaticDtor_PInputJoypadButtonSemantic_00b09630 @ 0x00b0_9630 => extern "C" fn s_00b09630() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[54])
    }
    /// `StaticDtor_PInputJoypadButtonSemantic_00b09650`: the `PInputJoypadButtonSemantic` value at `0x00c6_ba04` gets its vtable back, then `0x00a2_e410(obj)` unlinks it.
    StaticDtor_PInputJoypadButtonSemantic_00b09650 @ 0x00b0_9650 => extern "C" fn s_00b09650() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[55])
    }
    /// `StaticDtor_PInputJoypadButtonSemantic_00b09670`: the `PInputJoypadButtonSemantic` value at `0x00c6_b954` gets its vtable back, then `0x00a2_e410(obj)` unlinks it.
    StaticDtor_PInputJoypadButtonSemantic_00b09670 @ 0x00b0_9670 => extern "C" fn s_00b09670() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[56])
    }
    /// `StaticDtor_PInputJoypadButtonSemantic_00b09690`: the `PInputJoypadButtonSemantic` value at `0x00c6_ba54` gets its vtable back, then `0x00a2_e410(obj)` unlinks it.
    StaticDtor_PInputJoypadButtonSemantic_00b09690 @ 0x00b0_9690 => extern "C" fn s_00b09690() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[57])
    }
    /// `StaticDtor_PInputJoypadButtonSemantic_00b096b0`: the `PInputJoypadButtonSemantic` value at `0x00c6_b944` gets its vtable back, then `0x00a2_e410(obj)` unlinks it.
    StaticDtor_PInputJoypadButtonSemantic_00b096b0 @ 0x00b0_96b0 => extern "C" fn s_00b096b0() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[58])
    }
    /// `StaticDtor_PInputJoypadButtonSemantic_00b096d0`: the `PInputJoypadButtonSemantic` value at `0x00c6_b9f4` gets its vtable back, then `0x00a2_e410(obj)` unlinks it.
    StaticDtor_PInputJoypadButtonSemantic_00b096d0 @ 0x00b0_96d0 => extern "C" fn s_00b096d0() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[59])
    }
    /// `StaticDtor_PInputJoypadButtonSemantic_00b096f0`: the `PInputJoypadButtonSemantic` value at `0x00c6_b894` gets its vtable back, then `0x00a2_e410(obj)` unlinks it.
    StaticDtor_PInputJoypadButtonSemantic_00b096f0 @ 0x00b0_96f0 => extern "C" fn s_00b096f0() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[60])
    }
    /// `StaticDtor_PInputJoypadButtonSemantic_00b09710`: the `PInputJoypadButtonSemantic` value at `0x00c6_ba34` gets its vtable back, then `0x00a2_e410(obj)` unlinks it.
    StaticDtor_PInputJoypadButtonSemantic_00b09710 @ 0x00b0_9710 => extern "C" fn s_00b09710() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[61])
    }
    /// `StaticDtor_PInputJoypadButtonSemantic_00b09730`: the `PInputJoypadButtonSemantic` value at `0x00c6_b9d4` gets its vtable back, then `0x00a2_e410(obj)` unlinks it.
    StaticDtor_PInputJoypadButtonSemantic_00b09730 @ 0x00b0_9730 => extern "C" fn s_00b09730() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[62])
    }
    /// `StaticDtor_PInputJoypadButtonSemantic_00b09750`: the `PInputJoypadButtonSemantic` value at `0x00c6_b924` gets its vtable back, then `0x00a2_e410(obj)` unlinks it.
    StaticDtor_PInputJoypadButtonSemantic_00b09750 @ 0x00b0_9750 => extern "C" fn s_00b09750() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[63])
    }
    /// `StaticDtor_PInputJoypadButtonSemantic_00b09770`: the `PInputJoypadButtonSemantic` value at `0x00c6_ba24` gets its vtable back, then `0x00a2_e410(obj)` unlinks it.
    StaticDtor_PInputJoypadButtonSemantic_00b09770 @ 0x00b0_9770 => extern "C" fn s_00b09770() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[64])
    }
    /// `StaticDtor_PInputJoypadButtonSemantic_00b09790`: the `PInputJoypadButtonSemantic` value at `0x00c6_b914` gets its vtable back, then `0x00a2_e410(obj)` unlinks it.
    StaticDtor_PInputJoypadButtonSemantic_00b09790 @ 0x00b0_9790 => extern "C" fn s_00b09790() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[65])
    }
    /// `StaticDtor_PInputJoypadButtonSemantic_00b097b0`: the `PInputJoypadButtonSemantic` value at `0x00c6_b9c4` gets its vtable back, then `0x00a2_e410(obj)` unlinks it.
    StaticDtor_PInputJoypadButtonSemantic_00b097b0 @ 0x00b0_97b0 => extern "C" fn s_00b097b0() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[66])
    }
    /// `StaticDtor_PInputJoypadButtonSemantic_00b097d0`: the `PInputJoypadButtonSemantic` value at `0x00c6_b8d4` gets its vtable back, then `0x00a2_e410(obj)` unlinks it.
    StaticDtor_PInputJoypadButtonSemantic_00b097d0 @ 0x00b0_97d0 => extern "C" fn s_00b097d0() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[67])
    }
    /// `StaticDtor_PInputJoypadButtonSemantic_00b097f0`: the `PInputJoypadButtonSemantic` value at `0x00c6_b9a4` gets its vtable back, then `0x00a2_e410(obj)` unlinks it.
    StaticDtor_PInputJoypadButtonSemantic_00b097f0 @ 0x00b0_97f0 => extern "C" fn s_00b097f0() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[68])
    }
    /// `StaticDtor_PInputJoypadButtonSemantic_00b09810`: the `PInputJoypadButtonSemantic` value at `0x00c6_bb24` gets its vtable back, then `0x00a2_e410(obj)` unlinks it.
    StaticDtor_PInputJoypadButtonSemantic_00b09810 @ 0x00b0_9810 => extern "C" fn s_00b09810() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[69])
    }
    /// `StaticDtor_PInputJoypadButtonSemantic_00b09830`: the `PInputJoypadButtonSemantic` value at `0x00c6_b974` gets its vtable back, then `0x00a2_e410(obj)` unlinks it.
    StaticDtor_PInputJoypadButtonSemantic_00b09830 @ 0x00b0_9830 => extern "C" fn s_00b09830() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[70])
    }
    /// `StaticDtor_PInputJoypadButtonSemantic_00b09850`: the `PInputJoypadButtonSemantic` value at `0x00c6_ba94` gets its vtable back, then `0x00a2_e410(obj)` unlinks it.
    StaticDtor_PInputJoypadButtonSemantic_00b09850 @ 0x00b0_9850 => extern "C" fn s_00b09850() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[71])
    }
    /// `StaticDtor_PInputJoypadButtonSemantic_00b09870`: the `PInputJoypadButtonSemantic` value at `0x00c6_b8f4` gets its vtable back, then `0x00a2_e410(obj)` unlinks it.
    StaticDtor_PInputJoypadButtonSemantic_00b09870 @ 0x00b0_9870 => extern "C" fn s_00b09870() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[72])
    }
    /// `StaticDtor_PInputJoypadButtonSemantic_00b09890`: the `PInputJoypadButtonSemantic` value at `0x00c6_bb14` gets its vtable back, then `0x00a2_e410(obj)` unlinks it.
    StaticDtor_PInputJoypadButtonSemantic_00b09890 @ 0x00b0_9890 => extern "C" fn s_00b09890() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[73])
    }
    /// `StaticDtor_PInputJoypadButtonSemantic_00b098b0`: the `PInputJoypadButtonSemantic` value at `0x00c6_ba84` gets its vtable back, then `0x00a2_e410(obj)` unlinks it.
    StaticDtor_PInputJoypadButtonSemantic_00b098b0 @ 0x00b0_98b0 => extern "C" fn s_00b098b0() -> u32 {
        si::enum_value_dtor(&mut mem(), &si::ENUM_VALUE_DTORS[74])
    }
}
