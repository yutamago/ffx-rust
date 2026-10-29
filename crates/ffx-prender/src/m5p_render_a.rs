//! M5 stage 1, `PhyreEngine` list `render-a`, batch 1: the descriptor
//! destructors and the small helpers of the render classes (resources,
//! shaders, buffers, sprites, geometry utilities) that the class-gated
//! model reaches and no coverage run ran.
//!
//! - [`FORTYPE_DTORS`]: `PClassDescriptorForType<T>`'s destructor body,
//!   one copy per class: `*this = vtable`, a tail jump to
//!   `Phyre_PClassDescriptor_Destruct` (`0x0043b3e0`), whose `eax` it
//!   returns (`m5r_f::bridge::descriptor_destruct`). Only the atexit
//!   teardown of the descriptor globals and their deleting destructors
//!   run them.
//! - The owned-array resets (`PArray<T>` / `PSharray<T>` headers `[count |
//!   0x80000000 when the block is external, data]`), in four shapes by
//!   what they check before freeing; each returns the `eax` its last
//!   branch leaves (`0xfe` when the block is external: `shr eax, 31; not
//!   al`; `0xff` for a count of at most 1 in the sharray shapes; 0 for a
//!   null block; else the free's).
//! - The element helpers (copy, zero, the word-array allocator), the
//!   `PUtility` and `PContextSwitch` deleting destructors with their list
//!   unlinks, setters, getters and tail-jump forwarders.
//!
//! Floats: the setters move a float argument with `fld dword` / `fstp
//! dword`, which only quiets a signalling NaN ([`f32_move`]).

use ffx_bridge::x87;
use ffx_graphic::host::{Conv, Host};

/// The f64 degrees-to-radians factor `FUN_0049f190` multiplies by.
pub const DEG_TO_RAD: u32 = 0x00b9_2220;
/// `Phyre_PhyreFree(p)` (cdecl).
pub const PHYRE_FREE: u32 = 0x0042_fb20;
/// `operator_delete(p)` (cdecl).
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
/// `PhyreMallocAligned(size, align)` (cdecl).
pub const PHYRE_MALLOC_ALIGNED: u32 = 0x0042_fb80;
/// `Phyre_PString_Assign(this; text)` (`ret 4`).
pub const PSTRING_ASSIGN: u32 = 0x0044_5cf0;
/// `PMatrix4_Copy_00453340(out; m)` (`ret 4`).
pub const PMATRIX4_COPY: u32 = 0x0045_3340;
/// `Phyre_PClusterBase_clear(this)`.
pub const CLUSTER_CLEAR: u32 = 0x0044_0eb0;
/// `PRecursiveLock_release_004319d0(lock)`.
pub const RECURSIVE_LOCK_RELEASE: u32 = 0x0043_19d0;
/// `PSpriteArrays_release_00495a10(this)`.
pub const SPRITE_ARRAYS_RELEASE: u32 = 0x0049_5a10;
/// `PMatrix4_toWorldMatrix3x4(dst; m)` (`ret 4`).
pub const MATRIX_TO_WORLD_3X4: u32 = 0x0045_1190;
/// `0x00507410(this; arg)` (`ret 4`, render-a).
pub const SCENE_00507410: u32 = 0x0050_7410;
/// `0x00574760(this)`: the red-black tree pair half destructor.
pub const TREE_HALF_DTOR: u32 = 0x0057_4760;

/// `PUtility`'s vtable and the list of live utilities (next at `+4`).
pub const PUTILITY_VTABLE: u32 = 0x00b0_e078;
pub const PUTILITY_LIST: u32 = 0x00c9_01a8;
/// `PContextSwitch`'s vtable, its list (next at `+8`) and the total of
/// their sizes (`+0x10`).
pub const CONTEXT_SWITCH_VTABLE: u32 = 0x00b1_f7ac;
pub const CONTEXT_SWITCH_LIST: u32 = 0x00ca_4e18;
pub const CONTEXT_SWITCH_TOTAL: u32 = 0x00ca_4e20;
/// `FUN_00432760(item)`: unlinks from the context-switch list.
pub const CONTEXT_SWITCH_UNLINK: u32 = 0x0043_2760;
/// `PIChildClassVisitor`'s vtable.
pub const CHILD_VISITOR_VTABLE: u32 = 0x00b1_3484;
/// `PNetworkOptimizeBufferAllocator`'s and its `PBufferAcceptance`'s
/// vtables.
pub const BUFFER_ALLOCATOR_VTABLE: u32 = 0x00b3_f174;
pub const BUFFER_ACCEPTANCE_VTABLE: u32 = 0x00b3_f168;

/// The `PClassDescriptorForType<T>` destructor bodies: (address, the
/// vtable they store); `T` in the comment. Generated from `FFX.exe`
/// (`mov dword [ecx], vtable; jmp 0x0043b3e0`, the vtable's RTTI name).
pub const FORTYPE_DTORS: &[(u32, u32)] = &[
    (0x0048_0430, 0x00b1_3dbc), // PVertexStream
    (0x0048_1880, 0x00b1_436c), // PArray<PVertexStream>
    (0x0048_1890, 0x00b1_407c), // PDataBlock
    (0x0048_18a0, 0x00b1_3f94), // PDataBlockBase
    (0x0048_4150, 0x00b1_4574), // PDynamicDataBlock
    (0x0048_5030, 0x00b1_481c), // PIndexDataBlock
    (0x0048_5040, 0x00b1_4734), // PIndexDataBlockBase
    (0x0048_7270, 0x00b1_50f4), // PArray<PSkinBoneRemap>
    (0x0048_a5d0, 0x00b1_5594), // PSharray<PMaterial *>
    (0x0048_a5e0, 0x00b1_52fc), // PMaterialSet
    (0x0048_f380, 0x00b1_5d44), // PArray<PInt32>
    (0x0048_f390, 0x00b1_5f54), // PArray<PMatrix4>
    (0x0048_f3a0, 0x00b1_6164), // PArray<PMeshSegment>
    (0x0048_f3b0, 0x00b1_605c), // PArray<PSkeletonJointBounds>
    (0x0048_f3c0, 0x00b1_5e4c), // PArray<PString>
    (0x0049_69c0, 0x00b1_6ab4), // PArray<PDynamicSegmentDesc>
    (0x0049_69d0, 0x00b1_6704), // PDynamicMesh
    (0x0049_69e0, 0x00b1_6794), // PDynamicSegmentDesc
    (0x0049_f9f0, 0x00b1_75f4), // PConstantBuffer
    (0x0049_fa00, 0x00b1_750c), // PConstantBufferBase
    (0x004a_8640, 0x00b1_b90c), // PArray<PShaderParameterDefinition>
    (0x004a_8650, 0x00b1_ba14), // PArray<PShaderStreamDefinition>
    (0x004a_8660, 0x00b1_a274), // PShaderComputeProgram
    (0x004a_8670, 0x00b1_a094), // PShaderFragmentProgram
    (0x004a_8680, 0x00b1_a184), // PShaderGeometryProgram
    (0x004a_8690, 0x00b1_97cc), // PShaderParameterCaptureBufferByteAddressBuffer
    (0x004a_86a0, 0x00b1_9adc), // PShaderParameterCaptureBufferDataBlock
    (0x004a_86b0, 0x00b1_9bdc), // PShaderParameterCaptureBufferIndexDataBlock
    (0x004a_86c0, 0x00b1_98d4), // PShaderParameterCaptureBufferRWByteAddressBuffer
    (0x004a_86d0, 0x00b1_96c4), // PShaderParameterCaptureBufferRWStructuredBuffer
    (0x004a_86e0, 0x00b1_92bc), // PShaderParameterCaptureBufferRWTexture2D
    (0x004a_86f0, 0x00b1_93bc), // PShaderParameterCaptureBufferRWTexture3D
    (0x004a_8700, 0x00b1_94bc), // PShaderParameterCaptureBufferSampler
    (0x004a_8710, 0x00b1_8dc4), // PShaderParameterCaptureBufferStream
    (0x004a_8720, 0x00b1_95bc), // PShaderParameterCaptureBufferStructuredBuffer
    (0x004a_8730, 0x00b1_8fbc), // PShaderParameterCaptureBufferTexture2D
    (0x004a_8740, 0x00b1_90bc), // PShaderParameterCaptureBufferTexture3D
    (0x004a_8750, 0x00b1_8ebc), // PShaderParameterCaptureBufferTextureBase
    (0x004a_8760, 0x00b1_91bc), // PShaderParameterCaptureBufferTextureCubeMap
    (0x004a_8770, 0x00b1_99dc), // PShaderParameterCaptureConstantBuffer
    (0x004a_8780, 0x00b1_9ebc), // PShaderProgramBase
    (0x004a_8790, 0x00b1_9dc4), // PShaderProgramParamsAndStreams
    (0x004a_87a0, 0x00b1_9cdc), // PShaderSource
    (0x004a_87b0, 0x00b1_8cd4), // PShaderStreamDefinition
    (0x004a_87c0, 0x00b1_9fa4), // PShaderVertexProgram
    (0x004b_5350, 0x00b1_cc6c), // PArray<PShaderParameterCaptureBufferLocation>
    (0x004b_5360, 0x00b1_ca5c), // PArray<PShaderParameterCaptureBufferLocationType>
    (0x004b_5370, 0x00b1_cb64), // PArray<PShaderParameterCaptureBufferLocationTypeConstantBuffer>
    (0x004b_5380, 0x00b1_c214), // PShaderPass
    (0x004b_5390, 0x00b1_c12c), // PShaderPassBase
    (0x004b_53a0, 0x00b1_be3c), // PShaderPassParameterLocationTypes
    (0x004b_53b0, 0x00b1_bd3c), // PShaderPassParameterLocationTypesBase
    (0x004b_53c0, 0x00b1_bf34), // PShaderPassParameterLocationTypesConstantBuffer
    (0x004b_53d0, 0x00b1_c03c), // PShaderPassStateBase
    (0x004b_bf00, 0x00b1_d394), // PArray<PShaderPass>
    (0x004b_bf10, 0x00b1_d124), // PShader
    (0x004b_e8f0, 0x00b1_d714), // PSamplerState
    (0x004c_1dd0, 0x00b1_dc4c), // PTextureCommonBase
    (0x004c_2ff0, 0x00b1_dfa4), // PTexture2D
    (0x004c_3000, 0x00b1_debc), // PTexture2DBase
    (0x004c_4810, 0x00b1_e314), // PTexture3D
    (0x004c_4820, 0x00b1_e22c), // PTexture3DBase
    (0x004c_60d0, 0x00b1_e684), // PTextureCubeMap
    (0x004c_60e0, 0x00b1_e59c), // PTextureCubeMapBase
    (0x004c_79b0, 0x00b1_e9bc), // PRenderTarget
    (0x004c_79c0, 0x00b1_e91c), // PRenderTargetBase
    (0x004d_6260, 0x00b1_fb7c), // PParameterBuffer
    (0x004d_6270, 0x00b1_fad4), // PParameterBufferBase
    (0x004d_7ba0, 0x00b1_fe94), // PMaterial
    (0x004d_7bb0, 0x00b1_fdac), // PMaterialSwitch
    (0x004e_a270, 0x00b2_22fc), // PArray<PEffectVariant *>
    (0x004e_a280, 0x00b2_1fe4), // PArray<PContextSwitch *>
    (0x004e_a290, 0x00b2_21f4), // PArray<PLightType *>
    (0x004e_a2a0, 0x00b2_20ec), // PArray<PShadowCasterType *>
    (0x004e_a2b0, 0x00b2_1edc), // PArray<PNodeContext>
    (0x004e_a2c0, 0x00b2_1ac4), // PEffect
    (0x004f_a450, 0x00b2_362c), // PDynamicMeshInstance
    (0x004f_b4a0, 0x00b2_39a4), // PArray<PModifierNetworkInstance *>
    (0x004f_b4b0, 0x00b2_37ac), // PModifierNetworkDynamicMeshInstance
    (0x0050_1cb0, 0x00b2_4624), // PStructuredBuffer
    (0x0050_1cc0, 0x00b2_4534), // PStructuredBufferBase
    (0x0050_3230, 0x00b2_49bc), // PIndirectArgsBuffer
    (0x0050_3240, 0x00b2_48cc), // PIndirectArgsBufferBase
    (0x0050_4f70, 0x00b2_4c94), // PNode
    (0x0056_7440, 0x00b3_024c), // PArray<PSubTextureInfo>
    (0x0056_7450, 0x00b2_f594), // PSpriteAnimationInfo
    (0x0056_7460, 0x00b2_f684), // PSpriteAnimationInfoChar
    (0x0056_7470, 0x00b2_f774), // PSpriteAttributes
    (0x0056_7480, 0x00b2_f85c), // PSpriteCollection
    (0x0056_7490, 0x00b2_f3c4), // PSubTextureInfo
    (0x0056_74a0, 0x00b2_f4ac), // PTextureAtlasInfo
    (0x0056_f8d0, 0x00b3_0504), // PSpriteAnimationInfoInstance
    (0x0058_4800, 0x00b3_3c5c), // PArray<PStreamInputDescD3D11>
    (0x0058_4810, 0x00b3_1494), // PConstantBufferD3D11
    (0x0058_4820, 0x00b3_23a4), // PIndirectArgsBufferD3D11
    (0x0058_4830, 0x00b3_2214), // PRenderTargetD3D11
    (0x0058_4840, 0x00b3_1e6c), // PSamplerStateD3D11
    (0x0058_4850, 0x00b3_1b1c), // PShaderComputeProgramD3D11
    (0x0058_4860, 0x00b3_193c), // PShaderFragmentProgramD3D11
    (0x0058_4870, 0x00b3_1a2c), // PShaderGeometryProgramD3D11
    (0x0058_4880, 0x00b3_1d84), // PShaderPassD3D11
    (0x0058_4890, 0x00b3_1c94), // PShaderPassStateD3D11
    (0x0058_48a0, 0x00b3_1764), // PShaderProgramD3D11
    (0x0058_48b0, 0x00b3_184c), // PShaderVertexProgramD3D11
    (0x0058_48c0, 0x00b3_1584), // PStreamInputDescD3D11
    (0x0058_48d0, 0x00b3_1674), // PStreamInputLayoutD3D11
    (0x0058_48e0, 0x00b3_22b4), // PStructuredBufferD3D11
    (0x0058_48f0, 0x00b3_1f54), // PTexture2DD3D11
    (0x0058_4900, 0x00b3_203c), // PTexture3DD3D11
    (0x0058_4910, 0x00b3_2124), // PTextureCubeMapD3D11
];

/// The vtable a [`FORTYPE_DTORS`] copy stores (0 for an address not in
/// the table; the shims name constant addresses only).
#[must_use]
pub const fn fortype_vtable(addr: u32) -> u32 {
    let mut i = 0;
    while i < FORTYPE_DTORS.len() {
        if FORTYPE_DTORS[i].0 == addr {
            return FORTYPE_DTORS[i].1;
        }
        i += 1;
    }
    0
}

/// The owned-array resets whose block is freed with `Phyre_PhyreFree`
/// whenever it is owned ([`bridge::reset_owned`]): (address, header
/// offset).
pub const RESET_OWNED: &[(u32, u32)] = &[
    (0x004e_a010, 0),
    (0x004e_a040, 0),
    (0x004e_a070, 0),
    (0x004e_a0a0, 0),
    (0x0058_44f0, 0),
    (0x0058_4520, 0),
];

/// The resets that free an owned block only when it is not null
/// ([`bridge::reset_owned_nonnull`]): (address, header offset).
pub const RESET_NONNULL: &[(u32, u32)] = &[
    (0x0048_17d0, 0),
    (0x004d_b6e0, 0),
    (0x0056_72d0, 0),
    (0x0058_4590, 0),
    (0x0043_3d10, 8),
    (0x0048_1910, 8),
    (0x004b_53f0, 0x10),
    (0x004b_5450, 0x10),
];

/// The `PSharray<T>` resets: an owned block of more than one element,
/// not null ([`bridge::reset_shared`], `check_null` true), or without the
/// null check (`0x00433ab0`).
pub const RESET_SHARED: &[(u32, bool)] = &[
    (0x0043_3ab0, false),
    (0x0048_72b0, true),
    (0x0048_f400, true),
    (0x0058_4c70, true),
    (0x0058_4cb0, true),
    (0x0058_4cf0, true),
    (0x0058_4d30, true),
    (0x0058_4d70, true),
];

/// The resets that hand an owned block to an element release `(data,
/// count)` (cdecl) first ([`bridge::reset_release`]): (address, the
/// release).
pub const RESET_RELEASE: &[(u32, u32)] = &[
    (0x0043_3a30, 0x0043_4360),
    (0x004a_82c0, 0x004a_b4d0),
    (0x004d_b770, 0x004d_d770),
    (0x0058_5770, 0x0058_8190),
];

/// The word-array allocators ([`bridge::word_array_alloc`]).
pub const WORD_ARRAY_ALLOCS: &[u32] = &[
    0x004e_4160,
    0x004e_b0a0,
    0x004e_b110,
    0x004e_b180,
    0x004e_b1f0,
    0x004f_b850,
];

/// The element copies ([`bridge::copy_elements`]): (address, words per
/// element).
pub const COPY_ELEMENTS: &[(u32, u32)] = &[
    (0x0048_0b60, 5),
    (0x0048_4a30, 5),
    (0x004d_89e0, 5),
    (0x0049_5bc0, 4),
    (0x0054_65f0, 4),
];

/// The `PUtility` scalar deleting destructors ([`bridge::utility_delete`]).
pub const UTILITY_DELETES: &[u32] = &[0x0043_2250, 0x0043_25d0, 0x0050_3e70];

/// The `PContextSwitch` scalar deleting destructors (one per derived
/// class, the same code: [`bridge::context_switch_delete`]).
pub const CONTEXT_SWITCH_DELETES: &[u32] = &[
    0x004d_34d0,
    0x004d_3510,
    0x004d_3550,
    0x004d_3590,
    0x004d_35d0,
    0x004d_3610,
    0x004d_3650,
];

/// The list unlinks `(item)` (cdecl, [`bridge::list_unlink`]): (address,
/// the list head, the link's offset in a node).
pub const LIST_UNLINKS: &[(u32, u32, u32)] = &[
    (0x0043_2760, 0x00ca_4e18, 8),
    (0x0043_27b0, 0x00ca_54f8, 4),
    (0x005c_9930, 0x00cb_d348, 4),
];

/// The zeroing of 20-byte elements `(data, count)` (cdecl).
pub const ZERO_ELEMENTS20: &[u32] = &[0x0056_0df0, 0x0056_0e40];

/// The tree-pair destructors: `TREE_HALF_DTOR(this)` twice.
pub const TREE_PAIR_DTORS: &[u32] = &[0x0058_49d0, 0x0058_4a30];

/// The `setToExternalBuffer` copies `(this; buffer, count)` (`ret 8`):
/// (address, whether a null old block is skipped).
pub const SET_EXTERNAL: &[(u32, bool)] = &[(0x004e_7900, false), (0x004e_15a0, true)];

/// A float argument moved with `fld dword` / `fstp dword`: the same bits,
/// a signalling NaN quieted (the hardware's `fld`).
#[must_use]
pub const fn f32_move(bits: u32) -> u32 {
    if bits & 0x7f80_0000 == 0x7f80_0000 && bits & 0x007f_ffff != 0 {
        bits | 0x0040_0000
    } else {
        bits
    }
}

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;

    fn cdecl(h: &mut impl Host, f: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Cdecl, args)
    }

    /// `eax` after `mov eax, header; shr eax, 31; not al` for an external
    /// block (bit 31 set): the branch the resets take to skip the free.
    const EXTERNAL_EAX: u32 = 0xfe;

    /// `PClassDescriptorForType<T>`'s destructor body `(this)`.
    pub fn fortype_dtor(h: &mut impl Host, this: u32, vtable: u32) -> u32 {
        crate::m5r_f::bridge::descriptor_destruct(h, this, vtable)
    }

    /// An owned-array reset `(this)` at `this + off`: an owned block
    /// (bit 31 clear) freed, null or not; the header zeroed.
    pub fn reset_owned(h: &mut impl Host, this: u32, off: u32) -> u32 {
        let a = this + off;
        let count: u32 = h.read(a);
        let eax = if count & 0x8000_0000 == 0 {
            let data: u32 = h.read(a + 4);
            cdecl(h, PHYRE_FREE, &[data])
        } else {
            EXTERNAL_EAX
        };
        h.write(a, 0u32);
        h.write(a + 4, 0u32);
        eax
    }

    /// An owned-array reset `(this)` at `this + off` that frees an owned
    /// block only when it is not null.
    pub fn reset_owned_nonnull(h: &mut impl Host, this: u32, off: u32) -> u32 {
        let a = this + off;
        let count: u32 = h.read(a);
        let eax = if count & 0x8000_0000 == 0 {
            let data: u32 = h.read(a + 4);
            if data == 0 {
                0
            } else {
                cdecl(h, PHYRE_FREE, &[data])
            }
        } else {
            EXTERNAL_EAX
        };
        h.write(a + 4, 0u32);
        h.write(a, 0u32);
        eax
    }

    /// A `PSharray<T>` reset `(this)`: a block of more than one element
    /// (one is stored inline at `+4`) freed when owned (and, with
    /// `check_null`, not null).
    pub fn reset_shared(h: &mut impl Host, this: u32, check_null: bool) -> u32 {
        let count: u32 = h.read(this);
        let eax = if count & 0x8000_0000 != 0 {
            EXTERNAL_EAX
        } else if count & 0x7fff_ffff <= 1 {
            0xff
        } else {
            let data: u32 = h.read(this + 4);
            if check_null && data == 0 {
                0
            } else {
                cdecl(h, PHYRE_FREE, &[data])
            }
        };
        h.write(this, 0u32);
        h.write(this + 4, 0u32);
        eax
    }

    /// An owned-array reset `(this)` whose owned block goes to the
    /// element release `release(data, count)` (cdecl), which frees it.
    pub fn reset_release(h: &mut impl Host, this: u32, release: u32) -> u32 {
        let count: u32 = h.read(this);
        let eax = if count & 0x8000_0000 == 0 {
            let data: u32 = h.read(this + 4);
            cdecl(h, release, &[data, count & 0x7fff_ffff])
        } else {
            EXTERNAL_EAX
        };
        h.write(this, 0u32);
        h.write(this + 4, 0u32);
        eax
    }

    /// The word-array allocator `(n, src, count)` (cdecl) -> the block:
    /// `n` words (`PhyreMallocAligned(4n, 4)`), the first `count` copied
    /// from `src`, the rest zeroed; 0 for `n` 0 or a failed allocation.
    /// (`count` above `n` makes the original's `rep stosd` run off the
    /// block; callers never pass one.)
    pub fn word_array_alloc(h: &mut impl Host, n: u32, src: u32, count: u32) -> u32 {
        if n == 0 {
            return 0;
        }
        let block = cdecl(h, PHYRE_MALLOC_ALIGNED, &[n << 2, 4]);
        if block == 0 {
            return 0;
        }
        let mut s = src;
        for i in 0..count {
            let d = block.wrapping_add(4 * i);
            if d != 0 {
                let v: u32 = h.read(s);
                h.write(d, v);
                s = s.wrapping_add(4);
            }
        }
        let rest = n.wrapping_sub(count);
        for i in 0..rest {
            h.write(block.wrapping_add(4 * (count.wrapping_add(i))), 0u32);
        }
        block
    }

    /// The element copy `(dst, src, count)` (cdecl): `count` elements of
    /// `words` words, a destination slot at address 0 skipped without
    /// moving the source.
    pub fn copy_elements(h: &mut impl Host, dst: u32, src: u32, count: u32, words: u32) {
        let mut s = src;
        for i in 0..count {
            let d = dst.wrapping_add(4 * words * i);
            if d != 0 {
                for w in 0..words {
                    let v: u32 = h.read(s + 4 * w);
                    h.write(d + 4 * w, v);
                }
                s = s.wrapping_add(4 * words);
            }
        }
    }

    /// The zeroing of `count` 20-byte elements at `data` (cdecl), one at
    /// address 0 skipped.
    pub fn zero_elements20(h: &mut impl Host, data: u32, count: u32) {
        for i in 0..count {
            let d = data.wrapping_add(20 * i);
            if d != 0 {
                for w in 0..5 {
                    h.write(d + 4 * w, 0u32);
                }
            }
        }
    }

    /// Unlinks `node` from the list at `head` (next pointer at `+4`), as
    /// `PUtility`'s destructor does: the walk stops at `node` or at the
    /// last node; a found node's link takes its next, its next is cleared.
    fn utility_unlink(h: &mut impl Host, head: u32, node: u32) {
        let mut link = head;
        if h.read::<u32>(head) == 0 {
            return;
        }
        loop {
            let cur: u32 = h.read(link);
            if cur == node {
                break;
            }
            link = cur + 4;
            if h.read::<u32>(link) == 0 {
                break;
            }
        }
        if h.read::<u32>(link) != 0 {
            let next: u32 = h.read(node + 4);
            h.write(link, next);
            h.write(node + 4, 0u32);
        }
    }

    /// `PUtility`'s scalar deleting destructor `(this; flags)` (`ret 4`)
    /// -> `this`: its vtable, unlinked from the utility list, and with
    /// bit 0 of the flags `operator_delete(this)`.
    pub fn utility_delete(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        let vt = h.rebase(PUTILITY_VTABLE);
        h.write(this, vt);
        let head = h.rebase(PUTILITY_LIST);
        utility_unlink(h, head, this);
        if flags & 1 != 0 {
            cdecl(h, OPERATOR_DELETE, &[this]);
        }
        this
    }

    /// The list unlink `(item)` (cdecl) -> 0: `item` taken out of the
    /// list at `head` whose nodes link through `+link`.
    pub fn list_unlink(h: &mut impl Host, item: u32, head: u32, link: u32) -> u32 {
        let head = h.rebase(head);
        let mut prev = head;
        let mut node: u32 = h.read(head);
        while node != 0 {
            if node == item {
                let next: u32 = h.read(node + link);
                h.write(prev, next);
                h.write(node + link, 0u32);
                return 0;
            }
            prev = node + link;
            node = h.read(prev);
        }
        0
    }

    /// `PContextSwitch`'s scalar deleting destructor `(this; flags)`
    /// (`ret 4`) -> `this`: its vtable, `FUN_00432760(this)` (the list
    /// unlink), its size `+0x10` taken from the total, and with bit 0 of
    /// the flags `operator_delete(this)`.
    pub fn context_switch_delete(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        let vt = h.rebase(CONTEXT_SWITCH_VTABLE);
        h.write(this, vt);
        cdecl(h, CONTEXT_SWITCH_UNLINK, &[this]);
        let size: u32 = h.read(this + 0x10);
        let total = h.rebase(CONTEXT_SWITCH_TOTAL);
        let t: u32 = h.read(total);
        h.write(total, t.wrapping_sub(size));
        if flags & 1 != 0 {
            cdecl(h, OPERATOR_DELETE, &[this]);
        }
        this
    }

    /// A scalar deleting destructor `(this; flags)` (`ret 4`) -> `this`:
    /// `dtor(this)` (thiscall), then with bit 0 of the flags `free(this)`
    /// (`operator_delete` or `Phyre_PhyreFree`).
    pub fn delete_with(h: &mut impl Host, this: u32, flags: u32, dtor: u32, free: u32) -> u32 {
        h.call_sym(dtor, Conv::Thiscall(this), &[]);
        if flags & 1 != 0 {
            cdecl(h, free, &[this]);
        }
        this
    }

    /// `PIChildClassVisitor`'s scalar deleting destructor `(this; flags)`
    /// (`ret 4`) -> `this`.
    pub fn visitor_delete(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        let vt = h.rebase(CHILD_VISITOR_VTABLE);
        h.write(this, vt);
        if flags & 1 != 0 {
            cdecl(h, OPERATOR_DELETE, &[this]);
        }
        this
    }

    /// A destructor body that only stores its class's vtable `(this)`
    /// (`eax` left alone).
    pub fn store_vtable(h: &mut impl Host, this: u32, vtable: u32) {
        let vt = h.rebase(vtable);
        h.write(this, vt);
    }

    /// `PPair`'s constructor `(this)` -> `this`: two zero words.
    pub fn pair_init(h: &mut impl Host, this: u32) -> u32 {
        h.write(this, 0u32);
        h.write(this + 4, 0u32);
        this
    }

    /// The string-with-length getter `(sp, out, len)` (cdecl) -> 0:
    /// `*out = *sp`, `*len = strlen(*sp) + 1`.
    pub fn string_with_length(h: &mut impl Host, sp: u32, out: u32, len: u32) -> u32 {
        let s: u32 = h.read(sp);
        let mut n = 0u32;
        while h.read::<u8>(s.wrapping_add(n)) != 0 {
            n += 1;
        }
        h.write(len, n + 1);
        h.write(out, s);
        0
    }

    /// `PArray<T>::setToExternalBuffer(this; buffer, count)` (`ret 8`) ->
    /// 0: nothing for a negative count; else an owned old block other
    /// than `buffer` freed (with `skip_null`, only when not null), then
    /// `[count | 0x80000000, buffer]`.
    pub fn set_external(
        h: &mut impl Host,
        this: u32,
        buffer: u32,
        count: u32,
        skip_null: bool,
    ) -> u32 {
        if count & 0x8000_0000 != 0 {
            return 0;
        }
        let old: u32 = h.read(this + 4);
        let header: u32 = h.read(this);
        if old != buffer && header & 0x8000_0000 == 0 && !(skip_null && old == 0) {
            cdecl(h, PHYRE_FREE, &[old]);
        }
        h.write(this + 4, buffer);
        h.write(this, count | 0x8000_0000);
        0
    }

    /// The red-black tree lookup `(this; &key)` (`ret 4`) -> the node
    /// whose key `+0xc` equals `*key` (unsigned), or 0; the root at
    /// `+0xc`, `this` the sentinel, children left `+0` (key above) and
    /// right `+4`.
    pub fn tree_find(h: &mut impl Host, this: u32, key: u32) -> u32 {
        let k: u32 = h.read(key);
        let mut node: u32 = h.read(this + 0xc);
        while node != this {
            let nk: u32 = h.read(node + 0xc);
            if nk == k {
                return node;
            }
            node = h.read(node + if nk > k { 0 } else { 4 });
        }
        0
    }

    /// The tree-pair destructor `(this)`: `TREE_HALF_DTOR(this)` twice
    /// (thiscall), the second's `eax`.
    pub fn tree_pair_dtor(h: &mut impl Host, this: u32) -> u32 {
        h.call_sym(TREE_HALF_DTOR, Conv::Thiscall(this), &[]);
        h.call_sym(TREE_HALF_DTOR, Conv::Thiscall(this), &[])
    }

    /// A tail-jump forwarder `(this; arg)` to `target(this + off; arg)`
    /// (`ret 4`), its `eax`.
    pub fn forward1(h: &mut impl Host, this: u32, off: u32, target: u32, arg: u32) -> u32 {
        h.call_sym(target, Conv::Thiscall(this.wrapping_add(off)), &[arg])
    }

    /// A tail-jump forwarder `(this)` to `target(this)`, its `eax`.
    pub fn forward0(h: &mut impl Host, this: u32, target: u32) -> u32 {
        h.call_sym(target, Conv::Thiscall(this), &[])
    }

    /// `FUN_00463e60(this)`: `PRecursiveLock_release(*this)`'s `eax`.
    pub fn release_held_lock(h: &mut impl Host, this: u32) -> u32 {
        let lock: u32 = h.read(this);
        h.call_sym(RECURSIVE_LOCK_RELEASE, Conv::Thiscall(lock), &[])
    }

    /// `Phyre_PSprite_PUtilitySprite_terminate(this)` -> 0: the sprite
    /// arrays at `+0x10` released.
    pub fn sprite_terminate(h: &mut impl Host, this: u32) -> u32 {
        h.call_sym(SPRITE_ARRAYS_RELEASE, Conv::Thiscall(this + 0x10), &[]);
        0
    }

    /// The `cdecl` forwarder `FUN_005c9920(item)` to `FUN_005c9930`.
    pub fn forward_cdecl1(h: &mut impl Host, target: u32, arg: u32) -> u32 {
        cdecl(h, target, &[arg])
    }

    /// Float setters `(this; v...)`: `v[i]` at `this + off + 4i`.
    pub fn set_floats(h: &mut impl Host, this: u32, off: u32, v: &[u32]) {
        for (i, &x) in (0u32..).zip(v) {
            h.write(this + off + 4 * i, f32_move(x));
        }
    }

    /// `FUN_0049f190(this; a, b)` (`ret 8`, float bits): `x = a *
    /// 0.017453292519943295` (the f64 at `0x00b92220`) and `y = K * b`,
    /// each stored to f32 (in the original over its own argument slots),
    /// then the range in order: `y > 0` gives `+0x1c = x`, `+0x20 = x + y`;
    /// otherwise (`y` at most 0, or NaN) `+0x1c = x + y`, `+0x20 = x`.
    pub fn set_angle_range(h: &mut impl Host, this: u32, a: u32, b: u32) {
        angle_range(h, this, a, b);
    }

    /// `FUN_0049f1e0(this; p)` (`ret 4`): `FUN_0049f190(this; p[0], p[1])`
    /// for a non-null `p`.
    pub fn set_angle_range_from(h: &mut impl Host, this: u32, p: u32) {
        if p == 0 {
            return;
        }
        let a: u32 = h.read(p);
        let b: u32 = h.read(p + 4);
        angle_range(h, this, a, b);
    }

    fn angle_range(h: &mut impl Host, this: u32, a: u32, b: u32) {
        let at = h.rebase(DEG_TO_RAD);
        let k = f64::from_bits(h.read(at));
        let x = x87::store(x87::mul(x87::load(a), k));
        let y = x87::store(x87::mul(k, x87::load(b)));
        let (x64, y64) = (x87::load(x), x87::load(y));
        let sum = x87::store(x87::add(y64, x64));
        if y64 > 0.0 {
            h.write(this + 0x1c, x);
            h.write(this + 0x20, sum);
        } else {
            h.write(this + 0x1c, sum);
            h.write(this + 0x20, x);
        }
    }

    /// The word setter `(this; v)` at `this + off` -> `v` (`eax`).
    pub fn set_word(h: &mut impl Host, this: u32, off: u32, v: u32) -> u32 {
        h.write(this + off, v);
        v
    }

    /// `FUN_0055b620(this; b)` -> `al` = b: the byte at `+0x68`.
    pub fn set_byte68(h: &mut impl Host, this: u32, b: u32) -> u8 {
        h.write(this + 0x68, b as u8);
        b as u8
    }

    /// `FUN_0055b560(this; out)` (`ret 4`) -> `out`: the two words at
    /// `+0x4c` copied to `out`.
    pub fn get_pair4c(h: &mut impl Host, this: u32, out: u32) -> u32 {
        let a: u32 = h.read(this + 0x4c);
        let b: u32 = h.read(this + 0x50);
        h.write(out, a);
        h.write(out + 4, b);
        out
    }

    /// `PNetworkOptimizeBufferAllocator`'s constructor `(this)` -> `this`.
    pub fn buffer_allocator_ctor(h: &mut impl Host, this: u32) -> u32 {
        let vt = h.rebase(BUFFER_ALLOCATOR_VTABLE);
        h.write(this, vt);
        h.write(this + 4, 0u32);
        h.write(this + 8, 0u32);
        h.write(this + 0xc, 0xffff_ffff_u32);
        this
    }

    /// `PNetworkOptimizeBufferAllocator`'s destructor `(this)`: its
    /// vtable, the block `+4` freed when not null (`eax` the free's, or
    /// 0).
    pub fn buffer_allocator_dtor(h: &mut impl Host, this: u32) -> u32 {
        let block: u32 = h.read(this + 4);
        let vt = h.rebase(BUFFER_ALLOCATOR_VTABLE);
        h.write(this, vt);
        if block == 0 {
            0
        } else {
            cdecl(h, PHYRE_FREE, &[block])
        }
    }

    /// `FUN_00616310(this; i)` (`ret 4`) -> `i`: the byte at `[this + 4] +
    /// 8i` cleared.
    pub fn clear_slot_flag(h: &mut impl Host, this: u32, i: u32) -> u32 {
        let base: u32 = h.read(this + 4);
        h.write(base.wrapping_add(i.wrapping_mul(8)), 0u8);
        i
    }

    /// `FUN_0056dc70(this; i)` (`ret 4`): `setb al` of `[+0xc8][i] <
    /// [+0x20]` over the loaded word, whose upper bytes stay in `eax`.
    pub fn frame_below(h: &mut impl Host, this: u32, i: u32) -> u32 {
        let table: u32 = h.read(this + 0xc8);
        let v: u32 = h.read(table.wrapping_add(i.wrapping_mul(4)));
        let limit: u32 = h.read(this + 0x20);
        (v & !0xff) | u32::from(v < limit)
    }

    /// `FUN_0056dca0(this)` -> 0: `+0xb8` takes `+0xd8` when 0.
    pub fn default_b8(h: &mut impl Host, this: u32) -> u32 {
        if h.read::<u32>(this + 0xb8) == 0 {
            let v: u32 = h.read(this + 0xd8);
            h.write(this + 0xb8, v);
        }
        0
    }

    /// `FUN_0055b380(this; i)` (`ret 4`): word `i` of the `PSharray` at
    /// `+4` (count, data or the inline element at `+8`); an empty array
    /// reads address `4i`, as the original does.
    pub fn sharray_word(h: &mut impl Host, this: u32, i: u32) -> u32 {
        let count = h.read::<u32>(this + 4) & 0x7fff_ffff;
        let base = if count > 1 {
            h.read(this + 8)
        } else if count != 0 {
            this + 8
        } else {
            0
        };
        h.read(base.wrapping_add(i.wrapping_mul(4)))
    }

    /// `FUN_0055b780(this; arg)` (`ret 4`): with a `+0x58` object,
    /// `FUN_00507410(that; arg)`; else with `[+4] + 4` set,
    /// `PMatrix4_toWorldMatrix3x4(that; arg)`; else 0 (the `+0x58` word).
    pub fn forward_58(h: &mut impl Host, this: u32, arg: u32) -> u32 {
        let o: u32 = h.read(this + 0x58);
        if o != 0 {
            return h.call_sym(SCENE_00507410, Conv::Thiscall(o), &[arg]);
        }
        let p: u32 = h.read(this + 4);
        let m = if p == 0 { 0 } else { h.read(p + 4) };
        if m != 0 {
            return h.call_sym(MATRIX_TO_WORLD_3X4, Conv::Thiscall(m), &[arg]);
        }
        0
    }
}
