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
    /// `0x00ae_6290`: the render pass type `BlackInside` linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6290 @ 0x00ae_6290 => extern "C" fn init_00ae6290() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6290)
    }
    /// `0x00ae_62d0`: the render pass type `CaptureEmissiveSurfaces` linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae62d0 @ 0x00ae_62d0 => extern "C" fn init_00ae62d0() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_62d0)
    }
    /// `0x00ae_6310`: the render pass type `CaptureTriangleGeometry` linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6310 @ 0x00ae_6310 => extern "C" fn init_00ae6310() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6310)
    }
    /// `0x00ae_6350`: the render pass type `DeferredRender` linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6350 @ 0x00ae_6350 => extern "C" fn init_00ae6350() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6350)
    }
    /// `0x00ae_6390`: the render pass type `DepthTestDisableWithOptimize` linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6390 @ 0x00ae_6390 => extern "C" fn init_00ae6390() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6390)
    }
    /// `0x00ae_63d0`: the render pass type `DepthTestDisable` linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae63d0 @ 0x00ae_63d0 => extern "C" fn init_00ae63d0() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_63d0)
    }
    /// `0x00ae_6410`: the render pass type `DressUpFast` linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6410 @ 0x00ae_6410 => extern "C" fn init_00ae6410() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6410)
    }
    /// `0x00ae_6450`: the render pass type `DressUpMelt` linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6450 @ 0x00ae_6450 => extern "C" fn init_00ae6450() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6450)
    }
    /// `0x00ae_6490`: the render pass type `EnableStencilTest` linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6490 @ 0x00ae_6490 => extern "C" fn init_00ae6490() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6490)
    }
    /// `0x00ae_64d0`: the render pass type `FakeTransparent` linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae64d0 @ 0x00ae_64d0 => extern "C" fn init_00ae64d0() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_64d0)
    }
    /// `0x00ae_6510`: the render pass type `HSVPass` linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6510 @ 0x00ae_6510 => extern "C" fn init_00ae6510() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6510)
    }
    /// `0x00ae_6550`: the render pass type `NULL` linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6550 @ 0x00ae_6550 => extern "C" fn init_00ae6550() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6550)
    }
    /// `0x00ae_6590`: the render pass type `OpaqueNoMSAA` linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6590 @ 0x00ae_6590 => extern "C" fn init_00ae6590() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6590)
    }
    /// `0x00ae_65d0`: the render pass type `Opaque` linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae65d0 @ 0x00ae_65d0 => extern "C" fn init_00ae65d0() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_65d0)
    }
    /// `0x00ae_6610`: the render pass type `OptimizedTrans` linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6610 @ 0x00ae_6610 => extern "C" fn init_00ae6610() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6610)
    }
    /// `0x00ae_6650`: the render pass type `PS2Lighting` linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6650 @ 0x00ae_6650 => extern "C" fn init_00ae6650() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6650)
    }
    /// `0x00ae_6690`: the render pass type `SetStencilFlag` linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6690 @ 0x00ae_6690 => extern "C" fn init_00ae6690() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6690)
    }
    /// `0x00ae_66d0`: the render pass type `ShadowTransparent` linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae66d0 @ 0x00ae_66d0 => extern "C" fn init_00ae66d0() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_66d0)
    }
    /// `0x00ae_6710`: the render pass type `Shadow` linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6710 @ 0x00ae_6710 => extern "C" fn init_00ae6710() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6710)
    }
    /// `0x00ae_6750`: the render pass type `SpecularPass_akaza` linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6750 @ 0x00ae_6750 => extern "C" fn init_00ae6750() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6750)
    }
    /// `0x00ae_6790`: the render pass type `SpecularPass` linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6790 @ 0x00ae_6790 => extern "C" fn init_00ae6790() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6790)
    }
    /// `0x00ae_67d0`: the render pass type `SpecularPass_nolight_akaza1` linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae67d0 @ 0x00ae_67d0 => extern "C" fn init_00ae67d0() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_67d0)
    }
    /// `0x00ae_6810`: the render pass type `SpecularPass_nolight_akaza2` linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6810 @ 0x00ae_6810 => extern "C" fn init_00ae6810() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6810)
    }
    /// `0x00ae_6850`: the render pass type `SpecularPass_nolight` linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6850 @ 0x00ae_6850 => extern "C" fn init_00ae6850() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6850)
    }
    /// `0x00ae_6890`: the render pass type `SpecularPass_nolight_kohno1` linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6890 @ 0x00ae_6890 => extern "C" fn init_00ae6890() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6890)
    }
    /// `0x00ae_68d0`: the render pass type `TransparentClipY` linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae68d0 @ 0x00ae_68d0 => extern "C" fn init_00ae68d0() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_68d0)
    }
    /// `0x00ae_6910`: the render pass type `TransparentNega` linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6910 @ 0x00ae_6910 => extern "C" fn init_00ae6910() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6910)
    }
    /// `0x00ae_6950`: the render pass type `TransparentNoMSAA` linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6950 @ 0x00ae_6950 => extern "C" fn init_00ae6950() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6950)
    }
    /// `0x00ae_6990`: the render pass type `Transparent` linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6990 @ 0x00ae_6990 => extern "C" fn init_00ae6990() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6990)
    }
    /// `0x00ae_69d0`: the render pass type `ZPrePass` linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae69d0 @ 0x00ae_69d0 => extern "C" fn init_00ae69d0() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_69d0)
    }
    /// `0x00ae_6a10`: the render pass type `postOpaque` linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6a10 @ 0x00ae_6a10 => extern "C" fn init_00ae6a10() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6a10)
    }
    /// `0x00ae_6a50`: the render pass type `postTransparent` linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6a50 @ 0x00ae_6a50 => extern "C" fn init_00ae6a50() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6a50)
    }
    /// `0x00ae_6a90`: the render pass type `preOpaque` linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6a90 @ 0x00ae_6a90 => extern "C" fn init_00ae6a90() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6a90)
    }
    /// `0x00ae_6ad0`: the render pass type `preTransparentATC` linked into its list unless it is there.
    StaticInit_SceneRenderPassType_00ae6ad0 @ 0x00ae_6ad0 => extern "C" fn init_00ae6ad0() -> u32 {
        sa::find_link(&mut mem(), 0x00ae_6ad0)
    }
    /// `0x00ae_6b10`: the render pass type `preTransparent` linked into its list unless it is there.
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
    /// `0x00ae_49e0`: `Ctor8(0x00ca_3094, ...)` with the word `A16`, then `atexit(0x00af_e050)`.
    StaticInit_Ctor8_00ae49e0 @ 0x00ae_49e0 => extern "C" fn init_00ae49e0() -> u32 {
        sa::ctor8(&mut mem(), 0x00ae_49e0)
    }
    /// `0x00ae_4a10`: `Ctor8(0x00ca_315c, ...)` with the word `A2RGB10`, then `atexit(0x00af_e060)`.
    StaticInit_Ctor8_00ae4a10 @ 0x00ae_4a10 => extern "C" fn init_00ae4a10() -> u32 {
        sa::ctor8(&mut mem(), 0x00ae_4a10)
    }
    /// `0x00ae_4a40`: `Ctor8(0x00ca_2ff4, ...)` with the word `A8`, then `atexit(0x00af_e070)`.
    StaticInit_Ctor8_00ae4a40 @ 0x00ae_4a40 => extern "C" fn init_00ae4a40() -> u32 {
        sa::ctor8(&mut mem(), 0x00ae_4a40)
    }
    /// `0x00ae_4a70`: `Ctor8(0x00ca_3134, ...)` with the word `ARGB8`, then `atexit(0x00af_e080)`.
    StaticInit_Ctor8_00ae4a70 @ 0x00ae_4a70 => extern "C" fn init_00ae4a70() -> u32 {
        sa::ctor8(&mut mem(), 0x00ae_4a70)
    }
    /// `0x00ae_4aa0`: `Ctor8(0x00ca_3224, ...)` with the word `BC4`, then `atexit(0x00af_e090)`.
    StaticInit_Ctor8_00ae4aa0 @ 0x00ae_4aa0 => extern "C" fn init_00ae4aa0() -> u32 {
        sa::ctor8(&mut mem(), 0x00ae_4aa0)
    }
    /// `0x00ae_4ad0`: `Ctor8(0x00ca_324c, ...)` with the word `BC5`, then `atexit(0x00af_e0a0)`.
    StaticInit_Ctor8_00ae4ad0 @ 0x00ae_4ad0 => extern "C" fn init_00ae4ad0() -> u32 {
        sa::ctor8(&mut mem(), 0x00ae_4ad0)
    }
    /// `0x00ae_4b00`: `Ctor8(0x00ca_342c, ...)` with the word `DEPTH16`, then `atexit(0x00af_e0b0)`.
    StaticInit_Ctor8_00ae4b00 @ 0x00ae_4b00 => extern "C" fn init_00ae4b00() -> u32 {
        sa::ctor8(&mut mem(), 0x00ae_4b00)
    }
    /// `0x00ae_4b30`: `Ctor8(0x00ca_347c, ...)` with the word `DEPTH24S8`, then `atexit(0x00af_e0c0)`.
    StaticInit_Ctor8_00ae4b30 @ 0x00ae_4b30 => extern "C" fn init_00ae4b30() -> u32 {
        sa::ctor8(&mut mem(), 0x00ae_4b30)
    }
    /// `0x00ae_4b60`: `Ctor8(0x00ca_3454, ...)` with the word `DEPTH24`, then `atexit(0x00af_e0d0)`.
    StaticInit_Ctor8_00ae4b60 @ 0x00ae_4b60 => extern "C" fn init_00ae4b60() -> u32 {
        sa::ctor8(&mut mem(), 0x00ae_4b60)
    }
    /// `0x00ae_4b90`: `Ctor8(0x00ca_34a4, ...)` with the word `DEPTH32`, then `atexit(0x00af_e0e0)`.
    StaticInit_Ctor8_00ae4b90 @ 0x00ae_4b90 => extern "C" fn init_00ae4b90() -> u32 {
        sa::ctor8(&mut mem(), 0x00ae_4b90)
    }
    /// `0x00ae_4bc0`: `Ctor8(0x00ca_31ac, ...)` with the word `DXT1`, then `atexit(0x00af_e0f0)`.
    StaticInit_Ctor8_00ae4bc0 @ 0x00ae_4bc0 => extern "C" fn init_00ae4bc0() -> u32 {
        sa::ctor8(&mut mem(), 0x00ae_4bc0)
    }
    /// `0x00ae_4bf0`: `Ctor8(0x00ca_31d4, ...)` with the word `DXT3`, then `atexit(0x00af_e100)`.
    StaticInit_Ctor8_00ae4bf0 @ 0x00ae_4bf0 => extern "C" fn init_00ae4bf0() -> u32 {
        sa::ctor8(&mut mem(), 0x00ae_4bf0)
    }
    /// `0x00ae_4c20`: `Ctor8(0x00ca_31fc, ...)` with the word `DXT5`, then `atexit(0x00af_e110)`.
    StaticInit_Ctor8_00ae4c20 @ 0x00ae_4c20 => extern "C" fn init_00ae4c20() -> u32 {
        sa::ctor8(&mut mem(), 0x00ae_4c20)
    }
    /// `0x00ae_4c50`: `Ctor8(0x00ca_32ec, ...)` with the word `L16F`, then `atexit(0x00af_e120)`.
    StaticInit_Ctor8_00ae4c50 @ 0x00ae_4c50 => extern "C" fn init_00ae4c50() -> u32 {
        sa::ctor8(&mut mem(), 0x00ae_4c50)
    }
    /// `0x00ae_4c80`: `Ctor8(0x00ca_306c, ...)` with the word `L16`, then `atexit(0x00af_e130)`.
    StaticInit_Ctor8_00ae4c80 @ 0x00ae_4c80 => extern "C" fn init_00ae4c80() -> u32 {
        sa::ctor8(&mut mem(), 0x00ae_4c80)
    }
    /// `0x00ae_4cb0`: `Ctor8(0x00ca_333c, ...)` with the word `L32F`, then `atexit(0x00af_e140)`.
    StaticInit_Ctor8_00ae4cb0 @ 0x00ae_4cb0 => extern "C" fn init_00ae4cb0() -> u32 {
        sa::ctor8(&mut mem(), 0x00ae_4cb0)
    }
    /// `0x00ae_4ce0`: `Ctor8(0x00ca_2fcc, ...)` with the word `L8`, then `atexit(0x00af_e150)`.
    StaticInit_Ctor8_00ae4ce0 @ 0x00ae_4ce0 => extern "C" fn init_00ae4ce0() -> u32 {
        sa::ctor8(&mut mem(), 0x00ae_4ce0)
    }
    /// `0x00ae_4d10`: `Ctor8(0x00ca_338c, ...)` with the word `LA16F`, then `atexit(0x00af_e160)`.
    StaticInit_Ctor8_00ae4d10 @ 0x00ae_4d10 => extern "C" fn init_00ae4d10() -> u32 {
        sa::ctor8(&mut mem(), 0x00ae_4d10)
    }
    /// `0x00ae_4d40`: `Ctor8(0x00ca_30bc, ...)` with the word `LA16`, then `atexit(0x00af_e170)`.
    StaticInit_Ctor8_00ae4d40 @ 0x00ae_4d40 => extern "C" fn init_00ae4d40() -> u32 {
        sa::ctor8(&mut mem(), 0x00ae_4d40)
    }
    /// `0x00ae_4d70`: `Ctor8(0x00ca_33dc, ...)` with the word `LA32F`, then `atexit(0x00af_e180)`.
    StaticInit_Ctor8_00ae4d70 @ 0x00ae_4d70 => extern "C" fn init_00ae4d70() -> u32 {
        sa::ctor8(&mut mem(), 0x00ae_4d70)
    }
    /// `0x00ae_4da0`: `Ctor8(0x00ca_301c, ...)` with the word `LA8`, then `atexit(0x00af_e190)`.
    StaticInit_Ctor8_00ae4da0 @ 0x00ae_4da0 => extern "C" fn init_00ae4da0() -> u32 {
        sa::ctor8(&mut mem(), 0x00ae_4da0)
    }
    /// `0x00ae_4dd0`: `Ctor8(0x00ca_32c4, ...)` with the word `R16F`, then `atexit(0x00af_e1a0)`.
    StaticInit_Ctor8_00ae4dd0 @ 0x00ae_4dd0 => extern "C" fn init_00ae4dd0() -> u32 {
        sa::ctor8(&mut mem(), 0x00ae_4dd0)
    }
    /// `0x00ae_4e00`: `Ctor8(0x00ca_3314, ...)` with the word `R32F`, then `atexit(0x00af_e1b0)`.
    StaticInit_Ctor8_00ae4e00 @ 0x00ae_4e00 => extern "C" fn init_00ae4e00() -> u32 {
        sa::ctor8(&mut mem(), 0x00ae_4e00)
    }
    /// `0x00ae_4e30`: `Ctor8(0x00ca_3404, ...)` with the word `R32`, then `atexit(0x00af_e1c0)`.
    StaticInit_Ctor8_00ae4e30 @ 0x00ae_4e30 => extern "C" fn init_00ae4e30() -> u32 {
        sa::ctor8(&mut mem(), 0x00ae_4e30)
    }
    /// `0x00ae_4e60`: `Ctor8(0x00ca_3364, ...)` with the word `RG16F`, then `atexit(0x00af_e1d0)`.
    StaticInit_Ctor8_00ae4e60 @ 0x00ae_4e60 => extern "C" fn init_00ae4e60() -> u32 {
        sa::ctor8(&mut mem(), 0x00ae_4e60)
    }
    /// `0x00ae_4e90`: `Ctor8(0x00ca_30e4, ...)` with the word `RG16`, then `atexit(0x00af_e1e0)`.
    StaticInit_Ctor8_00ae4e90 @ 0x00ae_4e90 => extern "C" fn init_00ae4e90() -> u32 {
        sa::ctor8(&mut mem(), 0x00ae_4e90)
    }
    /// `0x00ae_4ec0`: `Ctor8(0x00ca_33b4, ...)` with the word `RG32F`, then `atexit(0x00af_e1f0)`.
    StaticInit_Ctor8_00ae4ec0 @ 0x00ae_4ec0 => extern "C" fn init_00ae4ec0() -> u32 {
        sa::ctor8(&mut mem(), 0x00ae_4ec0)
    }
    /// `0x00ae_4ef0`: `Ctor8(0x00ca_3044, ...)` with the word `RG8`, then `atexit(0x00af_e200)`.
    StaticInit_Ctor8_00ae4ef0 @ 0x00ae_4ef0 => extern "C" fn init_00ae4ef0() -> u32 {
        sa::ctor8(&mut mem(), 0x00ae_4ef0)
    }
    /// `0x00ae_4f20`: `Ctor8(0x00ca_3274, ...)` with the word `RGBA16F`, then `atexit(0x00af_e210)`.
    StaticInit_Ctor8_00ae4f20 @ 0x00ae_4f20 => extern "C" fn init_00ae4f20() -> u32 {
        sa::ctor8(&mut mem(), 0x00ae_4f20)
    }
    /// `0x00ae_4f50`: `Ctor8(0x00ca_3184, ...)` with the word `RGBA16`, then `atexit(0x00af_e220)`.
    StaticInit_Ctor8_00ae4f50 @ 0x00ae_4f50 => extern "C" fn init_00ae4f50() -> u32 {
        sa::ctor8(&mut mem(), 0x00ae_4f50)
    }
    /// `0x00ae_4f80`: `Ctor8(0x00ca_329c, ...)` with the word `RGBA32F`, then `atexit(0x00af_e230)`.
    StaticInit_Ctor8_00ae4f80 @ 0x00ae_4f80 => extern "C" fn init_00ae4f80() -> u32 {
        sa::ctor8(&mut mem(), 0x00ae_4f80)
    }
    /// `0x00ae_4fb0`: `Ctor8(0x00ca_310c, ...)` with the word `RGBA8`, then `atexit(0x00af_e240)`.
    StaticInit_Ctor8_00ae4fb0 @ 0x00ae_4fb0 => extern "C" fn init_00ae4fb0() -> u32 {
        sa::ctor8(&mut mem(), 0x00ae_4fb0)
    }
    /// `0x00ae_5000`: `Ctor8(0x00ca_34cc, ...)` with the word `UNKNOWN`, then `atexit(0x00af_e250)`.
    StaticInit_Ctor8_00ae5000 @ 0x00ae_5000 => extern "C" fn init_00ae5000() -> u32 {
        sa::ctor8(&mut mem(), 0x00ae_5000)
    }
    /// `0x00ae_0c30`: the class descriptor of `PClassDataMemberDynamic` in `0x00c9_1180`, then `atexit(0x00af_a3a0)`.
    StaticInit_ClassDesc_PClassDataMemberDynamic_00ae0c30 @ 0x00ae_0c30 => extern "C" fn init_00ae0c30() -> u32 {
        sa::class_desc(&mut mem(), 0x00ae_0c30)
    }
    /// `0x00ae_0cf0`: the class descriptor of `PClassDescriptorDynamic` in `0x00c9_1330`, then `atexit(0x00af_a3f0)`.
    StaticInit_ClassDesc_PClassDescriptorDynamic_00ae0cf0 @ 0x00ae_0cf0 => extern "C" fn init_00ae0cf0() -> u32 {
        sa::class_desc(&mut mem(), 0x00ae_0cf0)
    }
    /// `0x00ae_13e0`: the class descriptor of `PNameComponent` in `0x00c9_3e38`, then `atexit(0x00af_aeb0)`.
    StaticInit_ClassDesc_PNameComponent_00ae13e0 @ 0x00ae_13e0 => extern "C" fn init_00ae13e0() -> u32 {
        sa::class_desc(&mut mem(), 0x00ae_13e0)
    }
    /// `0x00ae_31e0`: the class descriptor of `PShaderComputeProgram` in `0x00c9_ca40`, then `atexit(0x00af_d3e0)`.
    StaticInit_ClassDesc_PShaderComputeProgram_00ae31e0 @ 0x00ae_31e0 => extern "C" fn init_00ae31e0() -> u32 {
        sa::class_desc(&mut mem(), 0x00ae_31e0)
    }
    /// `0x00ae_32a0`: the class descriptor of `PShaderFragmentProgram` in `0x00c9_c910`, then `atexit(0x00af_d400)`.
    StaticInit_ClassDesc_PShaderFragmentProgram_00ae32a0 @ 0x00ae_32a0 => extern "C" fn init_00ae32a0() -> u32 {
        sa::class_desc(&mut mem(), 0x00ae_32a0)
    }
    /// `0x00ae_3360`: the class descriptor of `PShaderGeometryProgram` in `0x00c9_c9a8`, then `atexit(0x00af_d420)`.
    StaticInit_ClassDesc_PShaderGeometryProgram_00ae3360 @ 0x00ae_3360 => extern "C" fn init_00ae3360() -> u32 {
        sa::class_desc(&mut mem(), 0x00ae_3360)
    }
    /// `0x00ae_4260`: the class descriptor of `PShaderVertexProgram` in `0x00c9_c878`, then `atexit(0x00af_d6a0)`.
    StaticInit_ClassDesc_PShaderVertexProgram_00ae4260 @ 0x00ae_4260 => extern "C" fn init_00ae4260() -> u32 {
        sa::class_desc(&mut mem(), 0x00ae_4260)
    }
    /// `0x00ae_5570`: the class descriptor of `PRenderTarget` in `0x00ca_4200`, then `atexit(0x00af_e440)`.
    StaticInit_ClassDesc_PRenderTarget_00ae5570 @ 0x00ae_5570 => extern "C" fn init_00ae5570() -> u32 {
        sa::class_desc(&mut mem(), 0x00ae_5570)
    }
    /// `0x00ae_6f40`: the class descriptor of `PModifierNetworkDynamicMeshInstance` in `0x00ca_8020`, then `atexit(0x00af_f800)`.
    StaticInit_ClassDesc_PModifierNetworkDynamicMeshInstance_00ae6f40 @ 0x00ae_6f40 => extern "C" fn init_00ae6f40() -> u32 {
        sa::class_desc(&mut mem(), 0x00ae_6f40)
    }
    /// `0x00ae_a920`: the class descriptor of `PShaderComputeProgramD3D11` in `0x00cb_3908`, then `atexit(0x00b0_3790)`.
    StaticInit_ClassDesc_PShaderComputeProgramD3D11_00aea920 @ 0x00ae_a920 => extern "C" fn init_00aea920() -> u32 {
        sa::class_desc(&mut mem(), 0x00ae_a920)
    }
    /// `0x00ae_a9e0`: the class descriptor of `PShaderFragmentProgramD3D11` in `0x00cb_37d8`, then `atexit(0x00b0_37b0)`.
    StaticInit_ClassDesc_PShaderFragmentProgramD3D11_00aea9e0 @ 0x00ae_a9e0 => extern "C" fn init_00aea9e0() -> u32 {
        sa::class_desc(&mut mem(), 0x00ae_a9e0)
    }
    /// `0x00ae_aaa0`: the class descriptor of `PShaderGeometryProgramD3D11` in `0x00cb_3870`, then `atexit(0x00b0_37d0)`.
    StaticInit_ClassDesc_PShaderGeometryProgramD3D11_00aeaaa0 @ 0x00ae_aaa0 => extern "C" fn init_00aeaaa0() -> u32 {
        sa::class_desc(&mut mem(), 0x00ae_aaa0)
    }
    /// `0x00ae_ace0`: the class descriptor of `PShaderProgramD3D11` in `0x00cb_36a8`, then `atexit(0x00b0_3830)`.
    StaticInit_ClassDesc_PShaderProgramD3D11_00aeace0 @ 0x00ae_ace0 => extern "C" fn init_00aeace0() -> u32 {
        sa::class_desc(&mut mem(), 0x00ae_ace0)
    }
    /// `0x00ae_ada0`: the class descriptor of `PShaderVertexProgramD3D11` in `0x00cb_3740`, then `atexit(0x00b0_3850)`.
    StaticInit_ClassDesc_PShaderVertexProgramD3D11_00aeada0 @ 0x00ae_ada0 => extern "C" fn init_00aeada0() -> u32 {
        sa::class_desc(&mut mem(), 0x00ae_ada0)
    }
    /// `0x00ae_10f0`: the class descriptor of `PCameraProjection` in `0x00c9_2358`, then `atexit(0x00af_aa20)`.
    StaticInit_ClassDesc_PCameraProjection_00ae10f0 @ 0x00ae_10f0 => extern "C" fn init_00ae10f0() -> u32 {
        sa::class_desc(&mut mem(), 0x00ae_10f0)
    }
    /// `0x00ae_8770`: the class descriptor of `PAnimationDataSource` in `0x00ca_bd28`, then `atexit(0x00b0_0c50)`.
    StaticInit_ClassDesc_PAnimationDataSource_00ae8770 @ 0x00ae_8770 => extern "C" fn init_00ae8770() -> u32 {
        sa::class_desc(&mut mem(), 0x00ae_8770)
    }
    /// `0x00ae_8b20`: the class descriptor of `PAnimationBlenderController` in `0x00ca_c618`, then `atexit(0x00b0_0e30)`.
    StaticInit_ClassDesc_PAnimationBlenderController_00ae8b20 @ 0x00ae_8b20 => extern "C" fn init_00ae8b20() -> u32 {
        sa::class_desc(&mut mem(), 0x00ae_8b20)
    }
    /// `0x00ae_97f0`: the class descriptor of `PScriptableComponent` in `0x00ca_f780`, then `atexit(0x00b0_20e0)`.
    StaticInit_ClassDesc_PScriptableComponent_00ae97f0 @ 0x00ae_97f0 => extern "C" fn init_00ae97f0() -> u32 {
        sa::class_desc(&mut mem(), 0x00ae_97f0)
    }
    /// `0x00ae_a7b0`: the class descriptor of `PRenderTargetD3D11` in `0x00cb_3b68`, then `atexit(0x00b0_3750)`.
    StaticInit_ClassDesc_PRenderTargetD3D11_00aea7b0 @ 0x00ae_a7b0 => extern "C" fn init_00aea7b0() -> u32 {
        sa::class_desc(&mut mem(), 0x00ae_a7b0)
    }
    /// `0x00ae_1a10`: `PhyreStaticEntry(0x00c9_8668, ...)` with the word `Binormal`, then `atexit(0x00af_c2e0)`.
    StaticInit_StaticEntry_00ae1a10 @ 0x00ae_1a10 => extern "C" fn init_00ae1a10() -> u32 {
        sa::static_entry(&mut mem(), 0x00ae_1a10)
    }
    /// `0x00ae_1a40`: `PhyreStaticEntry(0x00c9_86a4, ...)` with the word `BoundaryTriangleVertexIndex`, then `atexit(0x00af_c2f0)`.
    StaticInit_StaticEntry_00ae1a40 @ 0x00ae_1a40 => extern "C" fn init_00ae1a40() -> u32 {
        sa::static_entry(&mut mem(), 0x00ae_1a40)
    }
    /// `0x00ae_1a70`: `PhyreStaticEntry(0x00c9_8650, ...)` with the word `Color`, then `atexit(0x00af_c300)`.
    StaticInit_StaticEntry_00ae1a70 @ 0x00ae_1a70 => extern "C" fn init_00ae1a70() -> u32 {
        sa::static_entry(&mut mem(), 0x00ae_1a70)
    }
    /// `0x00ae_1aa0`: `PhyreStaticEntry(0x00c9_8698, ...)` with the word `EdgeFaceIndex`, then `atexit(0x00af_c310)`.
    StaticInit_StaticEntry_00ae1aa0 @ 0x00ae_1aa0 => extern "C" fn init_00ae1aa0() -> u32 {
        sa::static_entry(&mut mem(), 0x00ae_1aa0)
    }
    /// `0x00ae_1ad0`: `PhyreStaticEntry(0x00c9_868c, ...)` with the word `EdgeVertexIndex`, then `atexit(0x00af_c320)`.
    StaticInit_StaticEntry_00ae1ad0 @ 0x00ae_1ad0 => extern "C" fn init_00ae1ad0() -> u32 {
        sa::static_entry(&mut mem(), 0x00ae_1ad0)
    }
    /// `0x00ae_1b00`: `PhyreStaticEntry(0x00c9_86c8, ...)` with the word `InstanceTransform0`, then `atexit(0x00af_c330)`.
    StaticInit_StaticEntry_00ae1b00 @ 0x00ae_1b00 => extern "C" fn init_00ae1b00() -> u32 {
        sa::static_entry(&mut mem(), 0x00ae_1b00)
    }
    /// `0x00ae_1b30`: `PhyreStaticEntry(0x00c9_86d4, ...)` with the word `InstanceTransform1`, then `atexit(0x00af_c340)`.
    StaticInit_StaticEntry_00ae1b30 @ 0x00ae_1b30 => extern "C" fn init_00ae1b30() -> u32 {
        sa::static_entry(&mut mem(), 0x00ae_1b30)
    }
    /// `0x00ae_1b60`: `PhyreStaticEntry(0x00c9_86e0, ...)` with the word `InstanceTransform2`, then `atexit(0x00af_c350)`.
    StaticInit_StaticEntry_00ae1b60 @ 0x00ae_1b60 => extern "C" fn init_00ae1b60() -> u32 {
        sa::static_entry(&mut mem(), 0x00ae_1b60)
    }
    /// `0x00ae_1b90`: `PhyreStaticEntry(0x00c9_8644, ...)` with the word `LightMapST`, then `atexit(0x00af_c360)`.
    StaticInit_StaticEntry_00ae1b90 @ 0x00ae_1b90 => extern "C" fn init_00ae1b90() -> u32 {
        sa::static_entry(&mut mem(), 0x00ae_1b90)
    }
    /// `0x00ae_1bc0`: `PhyreStaticEntry(0x00c9_86ec, ...)` with the word `NULL`, then `atexit(0x00af_c370)`.
    StaticInit_StaticEntry_00ae1bc0 @ 0x00ae_1bc0 => extern "C" fn init_00ae1bc0() -> u32 {
        sa::static_entry(&mut mem(), 0x00ae_1bc0)
    }
    /// `0x00ae_1bf0`: `PhyreStaticEntry(0x00c9_862c, ...)` with the word `Normal`, then `atexit(0x00af_c380)`.
    StaticInit_StaticEntry_00ae1bf0 @ 0x00ae_1bf0 => extern "C" fn init_00ae1bf0() -> u32 {
        sa::static_entry(&mut mem(), 0x00ae_1bf0)
    }
    /// `0x00ae_1c20`: `PhyreStaticEntry(0x00c9_8638, ...)` with the word `ST`, then `atexit(0x00af_c390)`.
    StaticInit_StaticEntry_00ae1c20 @ 0x00ae_1c20 => extern "C" fn init_00ae1c20() -> u32 {
        sa::static_entry(&mut mem(), 0x00ae_1c20)
    }
    /// `0x00ae_1c50`: `PhyreStaticEntry(0x00c9_85b4, ...)` with the word `ScreenVertex`, then `atexit(0x00af_c3a0)`.
    StaticInit_StaticEntry_00ae1c50 @ 0x00ae_1c50 => extern "C" fn init_00ae1c50() -> u32 {
        sa::static_entry(&mut mem(), 0x00ae_1c50)
    }
    /// `0x00ae_1c80`: `PhyreStaticEntry(0x00c9_8614, ...)` with the word `SkinIndices`, then `atexit(0x00af_c3b0)`.
    StaticInit_StaticEntry_00ae1c80 @ 0x00ae_1c80 => extern "C" fn init_00ae1c80() -> u32 {
        sa::static_entry(&mut mem(), 0x00ae_1c80)
    }
    /// `0x00ae_1cb0`: `PhyreStaticEntry(0x00c9_8608, ...)` with the word `SkinWeights`, then `atexit(0x00af_c3c0)`.
    StaticInit_StaticEntry_00ae1cb0 @ 0x00ae_1cb0 => extern "C" fn init_00ae1cb0() -> u32 {
        sa::static_entry(&mut mem(), 0x00ae_1cb0)
    }
    /// `0x00ae_1ce0`: `PhyreStaticEntry(0x00c9_85fc, ...)` with the word `SkinnableBinormal`, then `atexit(0x00af_c3d0)`.
    StaticInit_StaticEntry_00ae1ce0 @ 0x00ae_1ce0 => extern "C" fn init_00ae1ce0() -> u32 {
        sa::static_entry(&mut mem(), 0x00ae_1ce0)
    }
    /// `0x00ae_1d10`: `PhyreStaticEntry(0x00c9_85e4, ...)` with the word `SkinnableNormal`, then `atexit(0x00af_c3e0)`.
    StaticInit_StaticEntry_00ae1d10 @ 0x00ae_1d10 => extern "C" fn init_00ae1d10() -> u32 {
        sa::static_entry(&mut mem(), 0x00ae_1d10)
    }
    /// `0x00ae_1d40`: `PhyreStaticEntry(0x00c9_85f0, ...)` with the word `SkinnableTangent`, then `atexit(0x00af_c3f0)`.
    StaticInit_StaticEntry_00ae1d40 @ 0x00ae_1d40 => extern "C" fn init_00ae1d40() -> u32 {
        sa::static_entry(&mut mem(), 0x00ae_1d40)
    }
    /// `0x00ae_1d70`: `PhyreStaticEntry(0x00c9_85d8, ...)` with the word `SkinnableVertex`, then `atexit(0x00af_c400)`.
    StaticInit_StaticEntry_00ae1d70 @ 0x00ae_1d70 => extern "C" fn init_00ae1d70() -> u32 {
        sa::static_entry(&mut mem(), 0x00ae_1d70)
    }
    /// `0x00ae_1da0`: `PhyreStaticEntry(0x00c9_865c, ...)` with the word `Tangent`, then `atexit(0x00af_c410)`.
    StaticInit_StaticEntry_00ae1da0 @ 0x00ae_1da0 => extern "C" fn init_00ae1da0() -> u32 {
        sa::static_entry(&mut mem(), 0x00ae_1da0)
    }
    /// `0x00ae_1dd0`: `PhyreStaticEntry(0x00c9_8680, ...)` with the word `TriangleEdgeIndex`, then `atexit(0x00af_c420)`.
    StaticInit_StaticEntry_00ae1dd0 @ 0x00ae_1dd0 => extern "C" fn init_00ae1dd0() -> u32 {
        sa::static_entry(&mut mem(), 0x00ae_1dd0)
    }
    /// `0x00ae_1e00`: `PhyreStaticEntry(0x00c9_8674, ...)` with the word `TriangleVertexIndex`, then `atexit(0x00af_c430)`.
    StaticInit_StaticEntry_00ae1e00 @ 0x00ae_1e00 => extern "C" fn init_00ae1e00() -> u32 {
        sa::static_entry(&mut mem(), 0x00ae_1e00)
    }
    /// `0x00ae_1e30`: `PhyreStaticEntry(0x00c9_86bc, ...)` with the word `VertexIndexWithRestart`, then `atexit(0x00af_c440)`.
    StaticInit_StaticEntry_00ae1e30 @ 0x00ae_1e30 => extern "C" fn init_00ae1e30() -> u32 {
        sa::static_entry(&mut mem(), 0x00ae_1e30)
    }
    /// `0x00ae_1e60`: `PhyreStaticEntry(0x00c9_86b0, ...)` with the word `VertexIndex`, then `atexit(0x00af_c450)`.
    StaticInit_StaticEntry_00ae1e60 @ 0x00ae_1e60 => extern "C" fn init_00ae1e60() -> u32 {
        sa::static_entry(&mut mem(), 0x00ae_1e60)
    }
    /// `0x00ae_1e90`: `PhyreStaticEntry(0x00c9_8620, ...)` with the word `Vertex`, then `atexit(0x00af_c460)`.
    StaticInit_StaticEntry_00ae1e90 @ 0x00ae_1e90 => extern "C" fn init_00ae1e90() -> u32 {
        sa::static_entry(&mut mem(), 0x00ae_1e90)
    }
    /// `0x00ae_1ec0`: `PhyreStaticEntry(0x00c9_85cc, ...)` with the word `WorldNormal`, then `atexit(0x00af_c470)`.
    StaticInit_StaticEntry_00ae1ec0 @ 0x00ae_1ec0 => extern "C" fn init_00ae1ec0() -> u32 {
        sa::static_entry(&mut mem(), 0x00ae_1ec0)
    }
    /// `0x00ae_1ef0`: `PhyreStaticEntry(0x00c9_85c0, ...)` with the word `WorldVertex`, then `atexit(0x00af_c480)`.
    StaticInit_StaticEntry_00ae1ef0 @ 0x00ae_1ef0 => extern "C" fn init_00ae1ef0() -> u32 {
        sa::static_entry(&mut mem(), 0x00ae_1ef0)
    }
}
