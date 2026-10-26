//! `PhyreEngine`'s core, list `p5-core` continued (`ffx_pcore::p5_more`).
//!
//! The thread pools' and the preprocess ring buffer's singletons, the
//! `PString` / `PSharray<T>` / `PArray<T *>` registrations (tables run by
//! h1's, p2's and h2's bridges), two `PClassDescriptorAbstract<T>`
//! constructors, `PShadowCaster`'s constructor and descriptor callbacks,
//! `PInputAction`'s destructor and the 4x4 transpose; `PMatrix4_lookAt`
//! and the affine inverses (`ffx_pcore::p5_math`). Nothing keeps state of
//! its own; the locals callees see are per-call `StackFrame`s.

use super::{StackFrame, mem, ported};
use ffx_pcore::p5_math::bridge as math;
use ffx_pcore::p5_more::{self as p5m, bridge as p5b};
use ffx_prender::{declare, h1_bind};

ported! {
    /// `Phyre_PString_Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PString_Bind @ 0x0044_54a0 => extern "C" fn bind_004454a0() -> u32 {
        h1_bind::bridge::bind(&mut mem(), p5m::P5_PSTRING_BIND)
    }
    /// `Phyre_PSharray_Phyre_PString__DeclareClassDescriptor(name)` -> the descriptor.
    Phyre_PSharray_Phyre_PString__DeclareClassDescriptor @ 0x0044_b3b0 => extern "C" fn declare_0044b3b0(name: u32) -> u32 {
        ffx_panim::p2_declare::bridge::declare_sharray(&mut mem(), name, &p5m::P5_PSTRING_SHARRAY)
    }
    /// `Phyre_PSharray_Phyre_PGameplay_PTriggerReceiverComponent__DeclareClassDescriptor(name)` -> the descriptor.
    Phyre_PSharray_Phyre_PGameplay_PTriggerReceiverComponent__DeclareClassDescriptor @ 0x0054_ecb0 => extern "C" fn declare_0054ecb0(name: u32) -> u32 {
        ffx_panim::p2_declare::bridge::declare_sharray(&mut mem(), name, &p5m::P5_TRIGGER_RECEIVER_SHARRAY)
    }
    /// `Phyre_PArray_Phyre_PInputs_PInputMap_8u__DeclareClassDescriptor(name)` -> the descriptor.
    Phyre_PArray_Phyre_PInputs_PInputMap_8u__DeclareClassDescriptor @ 0x00a3_3680 => extern "C" fn declare_00a33680(name: u32) -> u32 {
        declare::bridge::declare(&mut mem(), name, &p5m::P5_INPUT_MAP_ARRAY)
    }
    /// `PClassDescriptorAbstract<PCamera>(this)` -> `this`.
    Phyre_PClassDescriptorAbstract_Phyre_PCamera__PClassDescriptorAbstract @ 0x0045_2e80 => extern "thiscall" fn abstract_00452e80(this: u32) -> u32 {
        declare::bridge::abstract_ctor(&mut mem(), this, &p5m::P5_PCAMERA_ABSTRACT)
    }
    /// `PClassDescriptorAbstract<PShadowCaster::PShadowSplit>(this)` -> `this`.
    Phyre_PClassDescriptorAbstract_Phyre_PRendering_PShadowCaster_PShadowSplit__PClassDescriptorAbstract @ 0x004c_9160 => extern "thiscall" fn abstract_004c9160(this: u32) -> u32 {
        declare::bridge::abstract_ctor(&mut mem(), this, &p5m::P5_PSHADOWSPLIT_ABSTRACT)
    }
    /// `Phyre_PThreadPool_GetSingleton()` -> the pool.
    Phyre_PThreadPool_GetSingleton @ 0x0044_cd30 => extern "C" fn pool_0044cd30() -> u32 {
        p5b::thread_pool(&mut mem())
    }
    /// `Phyre_PThreadPoolImmediate_GetSingleton()` -> the pool.
    Phyre_PThreadPoolImmediate_GetSingleton @ 0x0057_cf50 => extern "C" fn pool_0057cf50() -> u32 {
        p5b::immediate_pool(&mut mem())
    }
    /// `Phyre_PPreprocessRingBuffer_GetSingleton()` -> the ring.
    Phyre_PPreprocessRingBuffer_GetSingleton @ 0x005c_d010 => extern "C" fn ring_005cd010() -> u32 {
        p5b::ring_buffer(&mut mem())
    }
    /// `PShadowCaster::PShadowCaster(this)` -> `this`.
    Phyre_PRendering_PShadowCaster_PShadowCaster_004c9500 @ 0x004c_9500 => extern "thiscall" fn ctor_004c9500(this: u32) -> u32 {
        p5b::shadow_caster_ctor(&mut mem(), this)
    }
    /// `PShadowCaster`'s `constructInstance(p)` -> `al` 1.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PShadowCaster__constructInstance @ 0x004c_c1f0 => extern "stdcall" fn construct_004cc1f0(p: u32) -> u32 {
        p5b::shadow_caster_construct(&mut mem(), p)
    }
    /// `PShadowCaster`'s `destructInstance(p)` -> `al` 1.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShadowCaster__destructInstance @ 0x004c_c2e0 => extern "stdcall" fn destruct_004cc2e0(p: u32) -> u32 {
        p5b::shadow_caster_destruct(&mut mem(), p)
    }
    /// `PInputAction`'s destructor `(this)`.
    Phyre_PInputs_PInputAction_dtor_PInputAction @ 0x009e_2330 => extern "thiscall" fn dtor_009e2330(this: u32) -> u32 {
        p5b::input_action_dtor(&mut mem(), this)
    }
    /// `PMatrix4_Transpose_004f93f0(out, m)` -> `out`.
    PMatrix4_Transpose_004f93f0 @ 0x004f_93f0 => extern "C" fn transpose_004f93f0(out: u32, m: u32) -> u32 {
        p5b::transpose(&mut mem(), out, m)
    }
    /// `Phyre_CharacterLighting_setLightMatrix(this; node, a, b)`.
    Phyre_CharacterLighting_setLightMatrix @ 0x006a_abf0 => extern "thiscall" fn light_006aabf0(this: u32, node: u32, a: u32, b: u32) {
        p5b::set_light_matrix(&mut mem(), this, node, a, b);
    }
    // batch 2: the matrix math (ffx_pcore::p5_math)
    /// `PMatrix4_lookAt(this; p, q, u)` -> `this`; its locals in a per-call frame.
    PMatrix4_lookAt @ 0x0045_1c30 => extern "thiscall" fn look_at_00451c30(this: u32, p: u32, q: u32, u: u32) -> u32 {
        let mut frame = StackFrame::<0x25>::new();
        math::look_at(&mut mem(), this, p, q, u, frame.at())
    }
    /// `Vectormath_Transform3_inverse(out, m)` -> `out`.
    Vectormath_Transform3_inverse @ 0x004c_c890 => extern "C" fn inverse3_004cc890(out: u32, m: u32) -> u32 {
        math::transform3_inverse(&mut mem(), out, m)
    }
    /// `GDraw_releaseTextureViews_00a34bc0(texture)`; its locals in a per-call frame.
    GDraw_releaseTextureViews_00a34bc0 @ 0x00a3_4bc0 => extern "C" fn gdraw_00a34bc0(texture: u32) -> u32 {
        let mut frame = StackFrame::<2>::new();
        ffx_pcore::p5_gdraw::bridge::unbind(&mut mem(), texture, frame.at())
    }
    /// `GDraw_createBlendState_00a35670(device, a, b, c)` -> the state or 0; its locals in a per-call frame.
    GDraw_createBlendState_00a35670 @ 0x00a3_5670 => extern "C" fn gdraw_00a35670(dev: u32, a: u32, b: u32, c: u32) -> u32 {
        let mut frame = StackFrame::<0x44>::new();
        ffx_pcore::p5_gdraw::bridge::create_blend_state(&mut mem(), dev, a, b, c, frame.at())
    }
    /// `PMatrix4_Inverse_004cc0c0(out, m)` -> `out`; its locals in a per-call frame.
    PMatrix4_Inverse_004cc0c0 @ 0x004c_c0c0 => extern "C" fn inverse_004cc0c0(out: u32, m: u32) -> u32 {
        let mut frame = StackFrame::<0x24>::new();
        math::inverse(&mut mem(), out, m, frame.at())
    }
}
