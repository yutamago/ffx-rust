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
}
