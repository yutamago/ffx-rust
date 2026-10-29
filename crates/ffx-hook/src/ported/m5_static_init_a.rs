//! M5 stage 1, list `static-init-a` (`ffx_static_a`): the compiler-generated
//! constructors of globals and the `atexit` destructors they register. The CRT
//! runs them before `WinMain`, from its initialiser tables. Each shim passes
//! its own address; the crate looks up its row (`ffx_static_a::rows`).
//!
//! Threads: the main thread, at start-up; the shims keep no state.

use super::{mem, ported};
use ffx_static_a as sa;

ported! {
    /// `0x00ae_03b0`: `atexit(0x00af_9b20)`; returns the `atexit` result.
    StaticInit_AtExit_00ae03b0 @ 0x00ae_03b0 => extern "C" fn init_00ae03b0() -> u32 {
        sa::dtor_only(&mut mem(), 0x00ae_03b0)
    }
    /// `0x00ae_03c0`: `atexit(0x00af_9b10)`; returns the `atexit` result.
    StaticInit_AtExit_00ae03c0 @ 0x00ae_03c0 => extern "C" fn init_00ae03c0() -> u32 {
        sa::dtor_only(&mut mem(), 0x00ae_03c0)
    }
    /// `0x00ae_03d0`: `atexit(0x00af_9b00)`; returns the `atexit` result.
    StaticInit_AtExit_00ae03d0 @ 0x00ae_03d0 => extern "C" fn init_00ae03d0() -> u32 {
        sa::dtor_only(&mut mem(), 0x00ae_03d0)
    }
    /// `0x00ae_0ff0`: `atexit(0x00af_a690)`; returns the `atexit` result.
    StaticInit_AtExit_00ae0ff0 @ 0x00ae_0ff0 => extern "C" fn init_00ae0ff0() -> u32 {
        sa::dtor_only(&mut mem(), 0x00ae_0ff0)
    }
    /// `0x00ae_1a00`: `atexit(0x00af_b980)`; returns the `atexit` result.
    StaticInit_AtExit_00ae1a00 @ 0x00ae_1a00 => extern "C" fn init_00ae1a00() -> u32 {
        sa::dtor_only(&mut mem(), 0x00ae_1a00)
    }
    /// `0x00ae_5ce0`: `atexit(0x00af_e870)`; returns the `atexit` result.
    StaticInit_AtExit_00ae5ce0 @ 0x00ae_5ce0 => extern "C" fn init_00ae5ce0() -> u32 {
        sa::dtor_only(&mut mem(), 0x00ae_5ce0)
    }
    /// `0x00ae_9080`: `atexit(0x00b0_0fd0)`; returns the `atexit` result.
    StaticInit_AtExit_00ae9080 @ 0x00ae_9080 => extern "C" fn init_00ae9080() -> u32 {
        sa::dtor_only(&mut mem(), 0x00ae_9080)
    }
    /// `0x00ae_a4d0`: `atexit(0x00b0_3570)`; returns the `atexit` result.
    StaticInit_AtExit_00aea4d0 @ 0x00ae_a4d0 => extern "C" fn init_00aea4d0() -> u32 {
        sa::dtor_only(&mut mem(), 0x00ae_a4d0)
    }
    /// `0x00ae_a4e0`: `atexit(0x00b0_3580)`; returns the `atexit` result.
    StaticInit_AtExit_00aea4e0 @ 0x00ae_a4e0 => extern "C" fn init_00aea4e0() -> u32 {
        sa::dtor_only(&mut mem(), 0x00ae_a4e0)
    }
    /// `0x00ae_a4f0`: `atexit(0x00b0_3590)`; returns the `atexit` result.
    StaticInit_AtExit_00aea4f0 @ 0x00ae_a4f0 => extern "C" fn init_00aea4f0() -> u32 {
        sa::dtor_only(&mut mem(), 0x00ae_a4f0)
    }
    /// `0x00ae_bef0`: `atexit(0x00b0_4760)`; returns the `atexit` result.
    StaticInit_AtExit_00aebef0 @ 0x00ae_bef0 => extern "C" fn init_00aebef0() -> u32 {
        sa::dtor_only(&mut mem(), 0x00ae_bef0)
    }
    /// `0x00ae_bf00`: `atexit(0x00b0_4770)`; returns the `atexit` result.
    StaticInit_AtExit_00aebf00 @ 0x00ae_bf00 => extern "C" fn init_00aebf00() -> u32 {
        sa::dtor_only(&mut mem(), 0x00ae_bf00)
    }
    /// `0x00ae_bf10`: `atexit(0x00b0_4780)`; returns the `atexit` result.
    StaticInit_AtExit_00aebf10 @ 0x00ae_bf10 => extern "C" fn init_00aebf10() -> u32 {
        sa::dtor_only(&mut mem(), 0x00ae_bf10)
    }
    /// `0x00ae_bf20`: `atexit(0x00b0_4790)`; returns the `atexit` result.
    StaticInit_AtExit_00aebf20 @ 0x00ae_bf20 => extern "C" fn init_00aebf20() -> u32 {
        sa::dtor_only(&mut mem(), 0x00ae_bf20)
    }
    /// `0x00ae_bf30`: `atexit(0x00b0_47a0)`; returns the `atexit` result.
    StaticInit_AtExit_00aebf30 @ 0x00ae_bf30 => extern "C" fn init_00aebf30() -> u32 {
        sa::dtor_only(&mut mem(), 0x00ae_bf30)
    }
    /// `0x00ae_c9b0`: `atexit(0x00b0_4b40)`; returns the `atexit` result.
    StaticInit_AtExit_00aec9b0 @ 0x00ae_c9b0 => extern "C" fn init_00aec9b0() -> u32 {
        sa::dtor_only(&mut mem(), 0x00ae_c9b0)
    }
    /// `0x00ae_7790`: `*0x00c1_a9d0 = 0x00ca_9738++`, then `atexit(0x00af_ff40)`.
    StaticInit_NextId_00ae7790 @ 0x00ae_7790 => extern "C" fn init_00ae7790() -> u32 {
        sa::next_id(&mut mem(), 0x00ae_7790)
    }
    /// `0x00ae_77b0`: `*0x00c1_aa00 = 0x00ca_9738++`, then `atexit(0x00af_ff50)`.
    StaticInit_NextId_00ae77b0 @ 0x00ae_77b0 => extern "C" fn init_00ae77b0() -> u32 {
        sa::next_id(&mut mem(), 0x00ae_77b0)
    }
    /// `0x00ae_77d0`: `*0x00c1_a9f0 = 0x00ca_9738++`, then `atexit(0x00af_ff60)`.
    StaticInit_NextId_00ae77d0 @ 0x00ae_77d0 => extern "C" fn init_00ae77d0() -> u32 {
        sa::next_id(&mut mem(), 0x00ae_77d0)
    }
    /// `0x00ae_77f0`: `*0x00c1_a9e0 = 0x00ca_9738++`, then `atexit(0x00af_ff70)`.
    StaticInit_NextId_00ae77f0 @ 0x00ae_77f0 => extern "C" fn init_00ae77f0() -> u32 {
        sa::next_id(&mut mem(), 0x00ae_77f0)
    }
    /// `0x00ae_7810`: `*0x00c1_a9a0 = 0x00ca_9738++`, then `atexit(0x00af_ff80)`.
    StaticInit_NextId_00ae7810 @ 0x00ae_7810 => extern "C" fn init_00ae7810() -> u32 {
        sa::next_id(&mut mem(), 0x00ae_7810)
    }
    /// `0x00ae_7830`: `*0x00c1_a9c0 = 0x00ca_9738++`, then `atexit(0x00af_ff90)`.
    StaticInit_NextId_00ae7830 @ 0x00ae_7830 => extern "C" fn init_00ae7830() -> u32 {
        sa::next_id(&mut mem(), 0x00ae_7830)
    }
    /// `0x00ae_7850`: `*0x00c1_a9b0 = 0x00ca_9738++`, then `atexit(0x00af_ffa0)`.
    StaticInit_NextId_00ae7850 @ 0x00ae_7850 => extern "C" fn init_00ae7850() -> u32 {
        sa::next_id(&mut mem(), 0x00ae_7850)
    }
    /// `0x00ae_7870`: `*0x00c1_aa10 = 0x00ca_9738++`, then `atexit(0x00af_ffb0)`.
    StaticInit_NextId_00ae7870 @ 0x00ae_7870 => extern "C" fn init_00ae7870() -> u32 {
        sa::next_id(&mut mem(), 0x00ae_7870)
    }
    /// `0x00ae_7890`: `*0x00c1_a930 = 0x00ca_9738++`, then `atexit(0x00af_ffc0)`.
    StaticInit_NextId_00ae7890 @ 0x00ae_7890 => extern "C" fn init_00ae7890() -> u32 {
        sa::next_id(&mut mem(), 0x00ae_7890)
    }
    /// `0x00ae_78b0`: `*0x00c1_a950 = 0x00ca_9738++`, then `atexit(0x00af_ffd0)`.
    StaticInit_NextId_00ae78b0 @ 0x00ae_78b0 => extern "C" fn init_00ae78b0() -> u32 {
        sa::next_id(&mut mem(), 0x00ae_78b0)
    }
    /// `0x00ae_78d0`: `*0x00c1_a990 = 0x00ca_9738++`, then `atexit(0x00af_ffe0)`.
    StaticInit_NextId_00ae78d0 @ 0x00ae_78d0 => extern "C" fn init_00ae78d0() -> u32 {
        sa::next_id(&mut mem(), 0x00ae_78d0)
    }
    /// `0x00ae_78f0`: `*0x00c1_a980 = 0x00ca_9738++`, then `atexit(0x00af_fff0)`.
    StaticInit_NextId_00ae78f0 @ 0x00ae_78f0 => extern "C" fn init_00ae78f0() -> u32 {
        sa::next_id(&mut mem(), 0x00ae_78f0)
    }
    /// `0x00ae_7910`: `*0x00c1_a970 = 0x00ca_9738++`, then `atexit(0x00b0_0000)`.
    StaticInit_NextId_00ae7910 @ 0x00ae_7910 => extern "C" fn init_00ae7910() -> u32 {
        sa::next_id(&mut mem(), 0x00ae_7910)
    }
    /// `0x00ae_7930`: `*0x00c1_a960 = 0x00ca_9738++`, then `atexit(0x00b0_0010)`.
    StaticInit_NextId_00ae7930 @ 0x00ae_7930 => extern "C" fn init_00ae7930() -> u32 {
        sa::next_id(&mut mem(), 0x00ae_7930)
    }
    /// `0x00ae_7950`: `*0x00c1_a940 = 0x00ca_9738++`, then `atexit(0x00b0_0020)`.
    StaticInit_NextId_00ae7950 @ 0x00ae_7950 => extern "C" fn init_00ae7950() -> u32 {
        sa::next_id(&mut mem(), 0x00ae_7950)
    }
    /// `0x00ae_7970`: `*0x00c1_aa20 = 0x00ca_9738++`, then `atexit(0x00b0_0030)`.
    StaticInit_NextId_00ae7970 @ 0x00ae_7970 => extern "C" fn init_00ae7970() -> u32 {
        sa::next_id(&mut mem(), 0x00ae_7970)
    }
    /// `0x00ae_7990`: `*0x00c1_aa40 = 0x00ca_9738++`, then `atexit(0x00b0_0040)`.
    StaticInit_NextId_00ae7990 @ 0x00ae_7990 => extern "C" fn init_00ae7990() -> u32 {
        sa::next_id(&mut mem(), 0x00ae_7990)
    }
    /// `0x00ae_79b0`: `*0x00c1_aa30 = 0x00ca_9738++`, then `atexit(0x00b0_0050)`.
    StaticInit_NextId_00ae79b0 @ 0x00ae_79b0 => extern "C" fn init_00ae79b0() -> u32 {
        sa::next_id(&mut mem(), 0x00ae_79b0)
    }
    /// `0x00ae_79d0`: `*0x00c1_aa50 = 0x00ca_9738++`, then `atexit(0x00b0_0060)`.
    StaticInit_NextId_00ae79d0 @ 0x00ae_79d0 => extern "C" fn init_00ae79d0() -> u32 {
        sa::next_id(&mut mem(), 0x00ae_79d0)
    }
    /// `0x00ae_03e0`: `*0x022f_b538 = 0`.
    StaticInit_ZeroWord_00ae03e0 @ 0x00ae_03e0 => extern "C" fn init_00ae03e0() {
        sa::zero_word(&mut mem(), 0x00ae_03e0);
    }
    /// `0x00ae_03f0`: `*0x022f_b534 = 0`.
    StaticInit_ZeroWord_00ae03f0 @ 0x00ae_03f0 => extern "C" fn init_00ae03f0() {
        sa::zero_word(&mut mem(), 0x00ae_03f0);
    }
    /// `0x00ae_0400`: `*0x022f_b510 = *0x00b0_c468`.
    StaticInit_CopyWord_00ae0400 @ 0x00ae_0400 => extern "C" fn init_00ae0400() -> u32 {
        sa::copy_word(&mut mem(), 0x00ae_0400)
    }
    /// `0x00ae_0410`: `*0x022f_b514 = *0x00b0_c370`.
    StaticInit_CopyWord_00ae0410 @ 0x00ae_0410 => extern "C" fn init_00ae0410() -> u32 {
        sa::copy_word(&mut mem(), 0x00ae_0410)
    }
    /// `0x00ae_6290`: the render pass type "BlackInside" linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6290 @ 0x00ae_6290 => extern "C" fn init_00ae6290() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6290)
    }
    /// `0x00ae_62d0`: the render pass type "CaptureEmissiveSurfaces" linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae62d0 @ 0x00ae_62d0 => extern "C" fn init_00ae62d0() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_62d0)
    }
    /// `0x00ae_6310`: the render pass type "CaptureTriangleGeometry" linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6310 @ 0x00ae_6310 => extern "C" fn init_00ae6310() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6310)
    }
    /// `0x00ae_6350`: the render pass type "DeferredRender" linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6350 @ 0x00ae_6350 => extern "C" fn init_00ae6350() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6350)
    }
    /// `0x00ae_6390`: the render pass type "DepthTestDisableWithOptimize" linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6390 @ 0x00ae_6390 => extern "C" fn init_00ae6390() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6390)
    }
    /// `0x00ae_63d0`: the render pass type "DepthTestDisable" linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae63d0 @ 0x00ae_63d0 => extern "C" fn init_00ae63d0() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_63d0)
    }
    /// `0x00ae_6410`: the render pass type "DressUpFast" linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6410 @ 0x00ae_6410 => extern "C" fn init_00ae6410() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6410)
    }
    /// `0x00ae_6450`: the render pass type "DressUpMelt" linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6450 @ 0x00ae_6450 => extern "C" fn init_00ae6450() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6450)
    }
    /// `0x00ae_6490`: the render pass type "EnableStencilTest" linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6490 @ 0x00ae_6490 => extern "C" fn init_00ae6490() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6490)
    }
    /// `0x00ae_64d0`: the render pass type "FakeTransparent" linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae64d0 @ 0x00ae_64d0 => extern "C" fn init_00ae64d0() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_64d0)
    }
    /// `0x00ae_6510`: the render pass type "HSVPass" linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6510 @ 0x00ae_6510 => extern "C" fn init_00ae6510() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6510)
    }
    /// `0x00ae_6550`: the render pass type "NULL" linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6550 @ 0x00ae_6550 => extern "C" fn init_00ae6550() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6550)
    }
    /// `0x00ae_6590`: the render pass type "OpaqueNoMSAA" linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6590 @ 0x00ae_6590 => extern "C" fn init_00ae6590() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6590)
    }
    /// `0x00ae_65d0`: the render pass type "Opaque" linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae65d0 @ 0x00ae_65d0 => extern "C" fn init_00ae65d0() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_65d0)
    }
    /// `0x00ae_6610`: the render pass type "OptimizedTrans" linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6610 @ 0x00ae_6610 => extern "C" fn init_00ae6610() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6610)
    }
    /// `0x00ae_6650`: the render pass type "PS2Lighting" linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6650 @ 0x00ae_6650 => extern "C" fn init_00ae6650() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6650)
    }
    /// `0x00ae_6690`: the render pass type "SetStencilFlag" linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6690 @ 0x00ae_6690 => extern "C" fn init_00ae6690() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6690)
    }
    /// `0x00ae_66d0`: the render pass type "ShadowTransparent" linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae66d0 @ 0x00ae_66d0 => extern "C" fn init_00ae66d0() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_66d0)
    }
    /// `0x00ae_6710`: the render pass type "Shadow" linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6710 @ 0x00ae_6710 => extern "C" fn init_00ae6710() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6710)
    }
    /// `0x00ae_6750`: the render pass type "SpecularPass_akaza" linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6750 @ 0x00ae_6750 => extern "C" fn init_00ae6750() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6750)
    }
    /// `0x00ae_6790`: the render pass type "SpecularPass" linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6790 @ 0x00ae_6790 => extern "C" fn init_00ae6790() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6790)
    }
    /// `0x00ae_67d0`: the render pass type "SpecularPass_nolight_akaza1" linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae67d0 @ 0x00ae_67d0 => extern "C" fn init_00ae67d0() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_67d0)
    }
    /// `0x00ae_6810`: the render pass type "SpecularPass_nolight_akaza2" linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6810 @ 0x00ae_6810 => extern "C" fn init_00ae6810() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6810)
    }
    /// `0x00ae_6850`: the render pass type "SpecularPass_nolight" linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6850 @ 0x00ae_6850 => extern "C" fn init_00ae6850() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6850)
    }
    /// `0x00ae_6890`: the render pass type "SpecularPass_nolight_kohno1" linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6890 @ 0x00ae_6890 => extern "C" fn init_00ae6890() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6890)
    }
    /// `0x00ae_68d0`: the render pass type "TransparentClipY" linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae68d0 @ 0x00ae_68d0 => extern "C" fn init_00ae68d0() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_68d0)
    }
    /// `0x00ae_6910`: the render pass type "TransparentNega" linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6910 @ 0x00ae_6910 => extern "C" fn init_00ae6910() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6910)
    }
    /// `0x00ae_6950`: the render pass type "TransparentNoMSAA" linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6950 @ 0x00ae_6950 => extern "C" fn init_00ae6950() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6950)
    }
    /// `0x00ae_6990`: the render pass type "Transparent" linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6990 @ 0x00ae_6990 => extern "C" fn init_00ae6990() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6990)
    }
    /// `0x00ae_69d0`: the render pass type "ZPrePass" linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae69d0 @ 0x00ae_69d0 => extern "C" fn init_00ae69d0() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_69d0)
    }
    /// `0x00ae_6a10`: the render pass type "postOpaque" linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6a10 @ 0x00ae_6a10 => extern "C" fn init_00ae6a10() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6a10)
    }
    /// `0x00ae_6a50`: the render pass type "postTransparent" linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6a50 @ 0x00ae_6a50 => extern "C" fn init_00ae6a50() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6a50)
    }
    /// `0x00ae_6a90`: the render pass type "preOpaque" linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6a90 @ 0x00ae_6a90 => extern "C" fn init_00ae6a90() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6a90)
    }
    /// `0x00ae_6ad0`: the render pass type "preTransparentATC" linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6ad0 @ 0x00ae_6ad0 => extern "C" fn init_00ae6ad0() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6ad0)
    }
    /// `0x00ae_6b10`: the render pass type "preTransparent" linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6b10 @ 0x00ae_6b10 => extern "C" fn init_00ae6b10() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6b10)
    }
    /// `0x00ae_0510`: `FFXApplication_FFXApplication(0x00c8_fb78)`, then `atexit(0x00af_9ba0)`.
    StaticInit_FFXApplication_FFXApplication_00ae0510 @ 0x00ae_0510 => extern "C" fn init_00ae0510() -> u32 {
        sa::call_dtor(&mut mem(), 0x00ae_0510)
    }
    /// `0x00ae_0710`: `Phyre_PSprite_PUtilitySprite_PUtilitySprite(0x00c9_0288)`, then `atexit(0x00af_9e10)`.
    StaticInit_Phyre_PSprite_PUtilitySprite_PUtilitySprite_00ae0710 @ 0x00ae_0710 => extern "C" fn init_00ae0710() -> u32 {
        sa::call_dtor(&mut mem(), 0x00ae_0710)
    }
    /// `0x00ae_0730`: `Phyre_PClassDescriptorBase_PClassDescriptorBase_00570670(0x00c9_02e8)`, then `atexit(0x00af_9f00)`.
    StaticInit_Phyre_PClassDescriptorBase_PClassDescriptorBase_00570670_00ae0730 @ 0x00ae_0730 => extern "C" fn init_00ae0730() -> u32 {
        sa::call_dtor(&mut mem(), 0x00ae_0730)
    }
    /// `0x00ae_0870`: `Phyre_Pair_set_00575870(0x00c9_07f4, 0x00b0_ec30)`, then `atexit(0x00af_a060)`.
    StaticInit_Phyre_Pair_set_00575870_00ae0870 @ 0x00ae_0870 => extern "C" fn init_00ae0870() -> u32 {
        sa::call_dtor(&mut mem(), 0x00ae_0870)
    }
    /// `0x00ae_1620`: `Phyre_PCriticalSectionSimple_PCriticalSectionSimple(0x00c9_40c4)`, then `atexit(0x00af_af60)`.
    StaticInit_Phyre_PCriticalSectionSimple_PCriticalSectionSimple_00ae1620 @ 0x00ae_1620 => extern "C" fn init_00ae1620() -> u32 {
        sa::call_dtor(&mut mem(), 0x00ae_1620)
    }
    /// `0x00ae_5ba0`: `Phyre_PRendering_PContextSwitchLights_PContextSwitchLights(0x00ca_4ec4)`, then `atexit(0x00af_e7a0)`.
    StaticInit_Phyre_PRendering_PContextSwitchLights_PContextSwitchLights_00ae5ba0 @ 0x00ae_5ba0 => extern "C" fn init_00ae5ba0() -> u32 {
        sa::call_dtor(&mut mem(), 0x00ae_5ba0)
    }
    /// `0x00ae_7170`: `PZeroQuad_init_005b3620(0x00ca_8998)`, then `atexit(0x00af_f950)`.
    StaticInit_PZeroQuad_init_005b3620_00ae7170 @ 0x00ae_7170 => extern "C" fn init_00ae7170() -> u32 {
        sa::call_dtor(&mut mem(), 0x00ae_7170)
    }
    /// `0x00ae_7190`: `PZeroQuad_init_005b3620(0x00ca_89a8)`, then `atexit(0x00af_f960)`.
    StaticInit_PZeroQuad_init_005b3620_00ae7190 @ 0x00ae_7190 => extern "C" fn init_00ae7190() -> u32 {
        sa::call_dtor(&mut mem(), 0x00ae_7190)
    }
    /// `0x00ae_a5b0`: `PhyreCtor_eventOwner_004315e0(0x00cb_3c94)`, then `atexit(0x00b0_3680)`.
    StaticInit_PhyreCtor_eventOwner_004315e0_00aea5b0 @ 0x00ae_a5b0 => extern "C" fn init_00aea5b0() -> u32 {
        sa::call_dtor(&mut mem(), 0x00ae_a5b0)
    }
}
