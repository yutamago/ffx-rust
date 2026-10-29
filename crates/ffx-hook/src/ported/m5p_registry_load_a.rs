//! M5 stage 1, list `phyre-registry-load-a`: `PhyreEngine`'s type
//! registry and loading code (`ffx_pload::m5p_registry_load_a*`). The
//! parity tests are `crates/ffx-oracle/tests/m5p_registry_load_a.rs`.
//!
//! Threads: the conversion kernels run where clusters load (the loader
//! thread and the main thread); no shim keeps state, each works on the
//! game's memory through [`mem`].

use ffx_pload::m5p_registry_load_a_convert as cv;
use ffx_pload::m5p_registry_load_a_ptype::{Kind, bridge as ptb};

use super::{mem, ported};

/// Runs kernel row `i` of [`cv::KERNELS`] (each shim passes its own row):
/// where the source and destination pointers end.
fn convert(i: usize, src: u32, dst: u32, desc: u32, ctx: u32) -> (u32, u32) {
    match cv::KERNELS.get(i) {
        Some(&(_, _, _, elem)) => cv::bridge::convert(&mut mem(), elem, src, dst, desc, ctx),
        None => (src, dst),
    }
}

ported! {
    // ---- the data-conversion kernels (batch 1) ----
    /// `PChar` to `PChar` (`Int(U8, B8)`).
    Phyre_PSerialization_convert_PChar_to_PChar @ 0x0047_19d0 => extern "C" fn k_004719d0(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(0, src, dst, desc, ctx);
    }
    /// `PChar` to `PUInt8` (`Int(U8, B8)`).
    Phyre_PSerialization_convert_PChar_to_PUInt8 @ 0x0047_1a20 => extern "C" fn k_00471a20(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(1, src, dst, desc, ctx);
    }
    /// `PChar` to `PInt16` (`Int(I8, B16)`).
    Phyre_PSerialization_convert_PChar_to_PInt16 @ 0x0047_1a70 => extern "C" fn k_00471a70(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(2, src, dst, desc, ctx);
    }
    /// `PChar` to `PUInt16` (`Int(I8, B16)`).
    Phyre_PSerialization_convert_PChar_to_PUInt16 @ 0x0047_1ad0 => extern "C" fn k_00471ad0(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(3, src, dst, desc, ctx);
    }
    /// `PChar` to `PInt32` (`Int(I8, B32)`).
    Phyre_PSerialization_convert_PChar_to_PInt32 @ 0x0047_1b30 => extern "C" fn k_00471b30(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(4, src, dst, desc, ctx);
    }
    /// `PChar` to `PUInt32` (`Int(I8, B32)`).
    Phyre_PSerialization_convert_PChar_to_PUInt32 @ 0x0047_1b90 => extern "C" fn k_00471b90(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(5, src, dst, desc, ctx);
    }
    /// `PChar` to `Float` (`ToF32(I8)`).
    Phyre_PSerialization_convert_PChar_to_Float @ 0x0047_1bf0 => extern "C" fn k_00471bf0(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(6, src, dst, desc, ctx);
    }
    /// `PChar` to `Double` (`ToF64(I8)`).
    Phyre_PSerialization_convert_PChar_to_Double @ 0x0047_1cb0 => extern "C" fn k_00471cb0(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(7, src, dst, desc, ctx);
    }
    /// `PChar` to `PUInt128` (`Int(I8, B128)`).
    Phyre_PSerialization_convert_PChar_to_PUInt128 @ 0x0047_1d70 => extern "C" fn k_00471d70(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(8, src, dst, desc, ctx);
    }
    /// `PChar` to `PInt64` (`Int(I8, B64)`).
    Phyre_PSerialization_convert_PChar_to_PInt64 @ 0x0047_1de0 => extern "C" fn k_00471de0(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(9, src, dst, desc, ctx);
    }
    /// `PChar` to `PUInt64` (`Int(I8, B64)`).
    Phyre_PSerialization_convert_PChar_to_PUInt64 @ 0x0047_1e40 => extern "C" fn k_00471e40(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(10, src, dst, desc, ctx);
    }
    /// `PChar` to `Bool` (`NonZero(U8)`).
    Phyre_PSerialization_convert_PChar_to_Bool @ 0x0047_1ea0 => extern "C" fn k_00471ea0(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(11, src, dst, desc, ctx);
    }
    /// `PUInt8` to `PChar` (`Int(U8, B8)`).
    Phyre_PSerialization_convert_PUInt8_to_PChar @ 0x0047_1f00 => extern "C" fn k_00471f00(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(12, src, dst, desc, ctx);
    }
    /// `PUInt8` to `PUInt8` (`Int(U8, B8)`).
    Phyre_PSerialization_convert_PUInt8_to_PUInt8 @ 0x0047_1f50 => extern "C" fn k_00471f50(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(13, src, dst, desc, ctx);
    }
    /// `PUInt8` to `PInt16` (`Int(U8, B16)`).
    Phyre_PSerialization_convert_PUInt8_to_PInt16 @ 0x0047_1fa0 => extern "C" fn k_00471fa0(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(14, src, dst, desc, ctx);
    }
    /// `PUInt8` to `PUInt16` (`Int(U8, B16)`).
    Phyre_PSerialization_convert_PUInt8_to_PUInt16 @ 0x0047_2000 => extern "C" fn k_00472000(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(15, src, dst, desc, ctx);
    }
    /// `PUInt8` to `PInt32` (`Int(U8, B32)`).
    Phyre_PSerialization_convert_PUInt8_to_PInt32 @ 0x0047_2060 => extern "C" fn k_00472060(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(16, src, dst, desc, ctx);
    }
    /// `PUInt8` to `PUInt32` (`Int(U8, B32)`).
    Phyre_PSerialization_convert_PUInt8_to_PUInt32 @ 0x0047_20c0 => extern "C" fn k_004720c0(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(17, src, dst, desc, ctx);
    }
    /// `PUInt8` to `Float` (`ToF32(U8)`).
    Phyre_PSerialization_convert_PUInt8_to_Float @ 0x0047_2120 => extern "C" fn k_00472120(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(18, src, dst, desc, ctx);
    }
    /// `PUInt8` to `Double` (`ToF64(U8)`).
    Phyre_PSerialization_convert_PUInt8_to_Double @ 0x0047_21e0 => extern "C" fn k_004721e0(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(19, src, dst, desc, ctx);
    }
    /// `PUInt8` to `PUInt128` (`Int(U8, B128)`).
    Phyre_PSerialization_convert_PUInt8_to_PUInt128 @ 0x0047_22a0 => extern "C" fn k_004722a0(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(20, src, dst, desc, ctx);
    }
    /// `PUInt8` to `PInt64` (`Int(U8, B64)`).
    Phyre_PSerialization_convert_PUInt8_to_PInt64 @ 0x0047_2310 => extern "C" fn k_00472310(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(21, src, dst, desc, ctx);
    }
    /// `PUInt8` to `PUInt64` (`Int(U8, B64)`).
    Phyre_PSerialization_convert_PUInt8_to_PUInt64 @ 0x0047_2370 => extern "C" fn k_00472370(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(22, src, dst, desc, ctx);
    }
    /// `PUInt8` to `Bool` (`NonZero(U8)`).
    Phyre_PSerialization_convert_PUInt8_to_Bool @ 0x0047_23d0 => extern "C" fn k_004723d0(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(23, src, dst, desc, ctx);
    }
    /// `PInt16` to `PChar` (`Int(U8, B8)`).
    Phyre_PSerialization_convert_PInt16_to_PChar @ 0x0047_2430 => extern "C" fn k_00472430(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(24, src, dst, desc, ctx);
    }
    /// `PInt16` to `PUInt8` (`Int(U8, B8)`).
    Phyre_PSerialization_convert_PInt16_to_PUInt8 @ 0x0047_2480 => extern "C" fn k_00472480(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(25, src, dst, desc, ctx);
    }
    /// `PInt16` to `PInt16` (`Int(U16, B16)`).
    Phyre_PSerialization_convert_PInt16_to_PInt16 @ 0x0047_24d0 => extern "C" fn k_004724d0(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(26, src, dst, desc, ctx);
    }
    /// `PInt16` to `PUInt16` (`Int(U16, B16)`).
    Phyre_PSerialization_convert_PInt16_to_PUInt16 @ 0x0047_2530 => extern "C" fn k_00472530(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(27, src, dst, desc, ctx);
    }
    /// `PInt16` to `PInt32` (`Int(I16, B32)`).
    Phyre_PSerialization_convert_PInt16_to_PInt32 @ 0x0047_2590 => extern "C" fn k_00472590(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(28, src, dst, desc, ctx);
    }
    /// `PInt16` to `PUInt32` (`Int(I16, B32)`).
    Phyre_PSerialization_convert_PInt16_to_PUInt32 @ 0x0047_25f0 => extern "C" fn k_004725f0(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(29, src, dst, desc, ctx);
    }
    /// `PInt16` to `Float` (`ToF32(I16)`).
    Phyre_PSerialization_convert_PInt16_to_Float @ 0x0047_2650 => extern "C" fn k_00472650(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(30, src, dst, desc, ctx);
    }
    /// `PInt16` to `Double` (`ToF64(I16)`).
    Phyre_PSerialization_convert_PInt16_to_Double @ 0x0047_2710 => extern "C" fn k_00472710(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(31, src, dst, desc, ctx);
    }
    /// `PInt16` to `PUInt128` (`Int(I16, B128)`).
    Phyre_PSerialization_convert_PInt16_to_PUInt128 @ 0x0047_27d0 => extern "C" fn k_004727d0(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(32, src, dst, desc, ctx);
    }
    /// `PInt16` to `PInt64` (`Int(I16, B64)`).
    Phyre_PSerialization_convert_PInt16_to_PInt64 @ 0x0047_2840 => extern "C" fn k_00472840(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(33, src, dst, desc, ctx);
    }
    /// `PInt16` to `PUInt64` (`Int(I16, B64)`).
    Phyre_PSerialization_convert_PInt16_to_PUInt64 @ 0x0047_28a0 => extern "C" fn k_004728a0(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(34, src, dst, desc, ctx);
    }
    /// `PInt16` to `Bool` (`NonZero(U16)`).
    Phyre_PSerialization_convert_PInt16_to_Bool @ 0x0047_2900 => extern "C" fn k_00472900(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(35, src, dst, desc, ctx);
    }
    /// `PUInt16` to `PChar` (`Int(U8, B8)`).
    Phyre_PSerialization_convert_PUInt16_to_PChar @ 0x0047_2960 => extern "C" fn k_00472960(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(36, src, dst, desc, ctx);
    }
    /// `PUInt16` to `PUInt8` (`Int(U8, B8)`).
    Phyre_PSerialization_convert_PUInt16_to_PUInt8 @ 0x0047_29b0 => extern "C" fn k_004729b0(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(37, src, dst, desc, ctx);
    }
    /// `PUInt16` to `PInt16` (`Int(U16, B16)`).
    Phyre_PSerialization_convert_PUInt16_to_PInt16 @ 0x0047_2a00 => extern "C" fn k_00472a00(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(38, src, dst, desc, ctx);
    }
    /// `PUInt16` to `PUInt16` (`Int(U16, B16)`).
    Phyre_PSerialization_convert_PUInt16_to_PUInt16 @ 0x0047_2a60 => extern "C" fn k_00472a60(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(39, src, dst, desc, ctx);
    }
    /// `PUInt16` to `PInt32` (`Int(U16, B32)`).
    Phyre_PSerialization_convert_PUInt16_to_PInt32 @ 0x0047_2ac0 => extern "C" fn k_00472ac0(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(40, src, dst, desc, ctx);
    }
    /// `PUInt16` to `PUInt32` (`Int(U16, B32)`).
    Phyre_PSerialization_convert_PUInt16_to_PUInt32 @ 0x0047_2b20 => extern "C" fn k_00472b20(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(41, src, dst, desc, ctx);
    }
    /// `PUInt16` to `Float` (`ToF32(U16)`).
    Phyre_PSerialization_convert_PUInt16_to_Float @ 0x0047_2b80 => extern "C" fn k_00472b80(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(42, src, dst, desc, ctx);
    }
    /// `PUInt16` to `Double` (`ToF64(U16)`).
    Phyre_PSerialization_convert_PUInt16_to_Double @ 0x0047_2c40 => extern "C" fn k_00472c40(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(43, src, dst, desc, ctx);
    }
    /// `PUInt16` to `PUInt128` (`Int(U16, B128)`).
    Phyre_PSerialization_convert_PUInt16_to_PUInt128 @ 0x0047_2d00 => extern "C" fn k_00472d00(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(44, src, dst, desc, ctx);
    }
    /// `PUInt16` to `PInt64` (`Int(U16, B64)`).
    Phyre_PSerialization_convert_PUInt16_to_PInt64 @ 0x0047_2d70 => extern "C" fn k_00472d70(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(45, src, dst, desc, ctx);
    }
    /// `PUInt16` to `PUInt64` (`Int(U16, B64)`).
    Phyre_PSerialization_convert_PUInt16_to_PUInt64 @ 0x0047_2dd0 => extern "C" fn k_00472dd0(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(46, src, dst, desc, ctx);
    }
    /// `PUInt16` to `Bool` (`NonZero(U16)`).
    Phyre_PSerialization_convert_PUInt16_to_Bool @ 0x0047_2e30 => extern "C" fn k_00472e30(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(47, src, dst, desc, ctx);
    }
    /// `PInt32` to `PChar` (`Int(U8, B8)`).
    Phyre_PSerialization_convert_PInt32_to_PChar @ 0x0047_2e90 => extern "C" fn k_00472e90(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(48, src, dst, desc, ctx);
    }
    /// `PInt32` to `PUInt8` (`Int(U8, B8)`).
    Phyre_PSerialization_convert_PInt32_to_PUInt8 @ 0x0047_2ee0 => extern "C" fn k_00472ee0(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(49, src, dst, desc, ctx);
    }
    /// `PInt32` to `PInt16` (`Int(U16, B16)`).
    Phyre_PSerialization_convert_PInt32_to_PInt16 @ 0x0047_2f30 => extern "C" fn k_00472f30(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(50, src, dst, desc, ctx);
    }
    /// `PInt32` to `PUInt16` (`Int(U16, B16)`).
    Phyre_PSerialization_convert_PInt32_to_PUInt16 @ 0x0047_2f90 => extern "C" fn k_00472f90(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(51, src, dst, desc, ctx);
    }
    /// `PInt32` to `PInt32` (`Int(U32, B32)`).
    Phyre_PSerialization_convert_PInt32_to_PInt32 @ 0x0047_2ff0 => extern "C" fn k_00472ff0(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(52, src, dst, desc, ctx);
    }
    /// `PInt32` to `PUInt32` (`Int(U32, B32)`).
    Phyre_PSerialization_convert_PInt32_to_PUInt32 @ 0x0047_3040 => extern "C" fn k_00473040(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(53, src, dst, desc, ctx);
    }
    /// `PInt32` to `Float` (`ToF32(I32)`); `eax`: where the source ends.
    Phyre_PSerialization_convert_PInt32_to_Float @ 0x0047_3090 => extern "C" fn k_00473090(src: u32, dst: u32, desc: u32, ctx: u32) -> u32 {
        convert(54, src, dst, desc, ctx).0
    }
    /// `PInt32` to `Double` (`ToF64(I32)`); `eax`: where the source ends.
    Phyre_PSerialization_convert_PInt32_to_Double @ 0x0047_3130 => extern "C" fn k_00473130(src: u32, dst: u32, desc: u32, ctx: u32) -> u32 {
        convert(55, src, dst, desc, ctx).0
    }
    /// `PInt32` to `PUInt128` (`Int(I32, B128)`).
    Phyre_PSerialization_convert_PInt32_to_PUInt128 @ 0x0047_31d0 => extern "C" fn k_004731d0(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(56, src, dst, desc, ctx);
    }
    /// `PInt32` to `PInt64` (`Int(I32, B64)`).
    Phyre_PSerialization_convert_PInt32_to_PInt64 @ 0x0047_3240 => extern "C" fn k_00473240(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(57, src, dst, desc, ctx);
    }
    /// `PInt32` to `PUInt64` (`Int(I32, B64)`).
    Phyre_PSerialization_convert_PInt32_to_PUInt64 @ 0x0047_32a0 => extern "C" fn k_004732a0(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(58, src, dst, desc, ctx);
    }
    /// `PInt32` to `Bool` (`NonZero(U32)`).
    Phyre_PSerialization_convert_PInt32_to_Bool @ 0x0047_3300 => extern "C" fn k_00473300(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(59, src, dst, desc, ctx);
    }
    /// `PUInt32` to `PChar` (`Int(U8, B8)`).
    Phyre_PSerialization_convert_PUInt32_to_PChar @ 0x0047_3360 => extern "C" fn k_00473360(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(60, src, dst, desc, ctx);
    }
    /// `PUInt32` to `PUInt8` (`Int(U8, B8)`).
    Phyre_PSerialization_convert_PUInt32_to_PUInt8 @ 0x0047_33b0 => extern "C" fn k_004733b0(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(61, src, dst, desc, ctx);
    }
    /// `PUInt32` to `PInt16` (`Int(U16, B16)`).
    Phyre_PSerialization_convert_PUInt32_to_PInt16 @ 0x0047_3400 => extern "C" fn k_00473400(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(62, src, dst, desc, ctx);
    }
    /// `PUInt32` to `PUInt16` (`Int(U16, B16)`).
    Phyre_PSerialization_convert_PUInt32_to_PUInt16 @ 0x0047_3460 => extern "C" fn k_00473460(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(63, src, dst, desc, ctx);
    }
    /// `PUInt32` to `PInt32` (`Int(U32, B32)`).
    Phyre_PSerialization_convert_PUInt32_to_PInt32 @ 0x0047_34c0 => extern "C" fn k_004734c0(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(64, src, dst, desc, ctx);
    }
    /// `PUInt32` to `PUInt32` (`Int(U32, B32)`).
    Phyre_PSerialization_convert_PUInt32_to_PUInt32 @ 0x0047_3510 => extern "C" fn k_00473510(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(65, src, dst, desc, ctx);
    }
    /// `PUInt32` to `Float` (`ToF32(U32)`).
    Phyre_PSerialization_convert_PUInt32_to_Float @ 0x0047_3560 => extern "C" fn k_00473560(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(66, src, dst, desc, ctx);
    }
    /// `PUInt32` to `Double` (`ToF64(U32)`).
    Phyre_PSerialization_convert_PUInt32_to_Double @ 0x0047_3630 => extern "C" fn k_00473630(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(67, src, dst, desc, ctx);
    }
    /// `PUInt32` to `PUInt128` (`Int(U32, B128)`); `eax`: where the destination ends.
    Phyre_PSerialization_convert_PUInt32_to_PUInt128 @ 0x0047_3700 => extern "C" fn k_00473700(src: u32, dst: u32, desc: u32, ctx: u32) -> u32 {
        convert(68, src, dst, desc, ctx).1
    }
    /// `PUInt32` to `PInt64` (`Int(U32, B64)`); `eax`: where the destination ends.
    Phyre_PSerialization_convert_PUInt32_to_PInt64 @ 0x0047_3770 => extern "C" fn k_00473770(src: u32, dst: u32, desc: u32, ctx: u32) -> u32 {
        convert(69, src, dst, desc, ctx).1
    }
    /// `PUInt32` to `PUInt64` (`Int(U32, B64)`); `eax`: where the destination ends.
    Phyre_PSerialization_convert_PUInt32_to_PUInt64 @ 0x0047_37d0 => extern "C" fn k_004737d0(src: u32, dst: u32, desc: u32, ctx: u32) -> u32 {
        convert(70, src, dst, desc, ctx).1
    }
    /// `PUInt32` to `Bool` (`NonZero(U32)`).
    Phyre_PSerialization_convert_PUInt32_to_Bool @ 0x0047_3830 => extern "C" fn k_00473830(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(71, src, dst, desc, ctx);
    }
    /// `Float` to `PChar` (`FromF32(Ftol(1))`).
    Phyre_PSerialization_convert_Float_to_PChar @ 0x0047_3890 => extern "C" fn k_00473890(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(72, src, dst, desc, ctx);
    }
    /// `Float` to `PUInt8` (`FromF32(Chop32(1))`).
    Phyre_PSerialization_convert_Float_to_PUInt8 @ 0x0047_38f0 => extern "C" fn k_004738f0(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(73, src, dst, desc, ctx);
    }
    /// `Float` to `PInt16` (`FromF32(Ftol(2))`).
    Phyre_PSerialization_convert_Float_to_PInt16 @ 0x0047_3960 => extern "C" fn k_00473960(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(74, src, dst, desc, ctx);
    }
    /// `Float` to `PUInt16` (`FromF32(Chop32(2))`).
    Phyre_PSerialization_convert_Float_to_PUInt16 @ 0x0047_39c0 => extern "C" fn k_004739c0(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(75, src, dst, desc, ctx);
    }
    /// `Float` to `PInt32` (`FromF32(Ftol(4))`).
    Phyre_PSerialization_convert_Float_to_PInt32 @ 0x0047_3a30 => extern "C" fn k_00473a30(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(76, src, dst, desc, ctx);
    }
    /// `Float` to `PUInt32` (`FromF32(Chop64Low)`).
    Phyre_PSerialization_convert_Float_to_PUInt32 @ 0x0047_3a90 => extern "C" fn k_00473a90(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(77, src, dst, desc, ctx);
    }
    /// `Float` to `Float` (`F32ToF32`); `eax`: where the source ends.
    Phyre_PSerialization_convert_Float_to_Float @ 0x0047_3b00 => extern "C" fn k_00473b00(src: u32, dst: u32, desc: u32, ctx: u32) -> u32 {
        convert(78, src, dst, desc, ctx).0
    }
    /// `Float` to `Double` (`F32ToF64`); `eax`: where the source ends.
    Phyre_PSerialization_convert_Float_to_Double @ 0x0047_3ba0 => extern "C" fn k_00473ba0(src: u32, dst: u32, desc: u32, ctx: u32) -> u32 {
        convert(79, src, dst, desc, ctx).0
    }
    /// `Float` to `PUInt128` (`FromF32(Unsigned64(true))`).
    Phyre_PSerialization_convert_Float_to_PUInt128 @ 0x0047_3c40 => extern "C" fn k_00473c40(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(80, src, dst, desc, ctx);
    }
    /// `Float` to `PInt64` (`FromF32(Ftol64)`).
    Phyre_PSerialization_convert_Float_to_PInt64 @ 0x0047_3d00 => extern "C" fn k_00473d00(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(81, src, dst, desc, ctx);
    }
    /// `Float` to `PUInt64` (`FromF32(Unsigned64(false))`).
    Phyre_PSerialization_convert_Float_to_PUInt64 @ 0x0047_3d60 => extern "C" fn k_00473d60(src: u32, dst: u32, desc: u32, ctx: u32) {
        convert(82, src, dst, desc, ctx);
    }

    // ---- PType and PTypeDefault<T> (batch 2) ----
    /// `PType::~PType()`: the vtable only; `eax` left alone.
    Phyre_PType_dtor_00437700 @ 0x0043_7700 => extern "thiscall" fn pt_dtor(this: u32) {
        ptb::ptype_dtor(&mut mem(), this);
    }
    /// `PType`'s scalar deleting destructor.
    Phyre_PType_deletingDtor_00437710 @ 0x0043_7710 => extern "thiscall" fn pt_delete(this: u32, flags: u32) -> u32 {
        ptb::delete_type(&mut mem(), this, flags)
    }
    /// `PType::PType(a, b, c)`.
    Phyre_PType_ctor_00438cc0 @ 0x0043_8cc0 => extern "thiscall" fn pt_ctor(this: u32, a: u32, b: u32, c: u32) -> u32 {
        ptb::construct(&mut mem(), this, a, b, c)
    }
    /// `cdecl(name, seed, len)`: the type-name hash.
    Phyre_PType_nameHash_004377d0 @ 0x0043_77d0 => extern "C" fn pt_hash(name: u32, seed: u32, len: u32) -> u32 {
        ptb::name_hash(&mem(), name, seed, len)
    }
    /// `PType::GetFixupForPTypePointer(type, &name, &len)`.
    Phyre_PType_GetFixupForPTypePointer @ 0x0043_7740 => extern "C" fn pt_get_fixup(ty: u32, name: u32, len: u32) -> u32 {
        ptb::get_fixup(&mut mem(), ty, name, len)
    }
    /// `PType::ResolveFixupForPTypePointer(out, name)`.
    Phyre_PType_ResolveFixupForPTypePointer @ 0x0043_7830 => extern "C" fn pt_resolve(out: u32, name: u32) -> u32 {
        ptb::resolve_fixup(&mut mem(), out, name)
    }
    /// `cdecl(lo, hi)`: the 64-bit byte swap (`PInt64`).
    Phyre_swapBytes64_00439230 @ 0x0043_9230 => extern "C" fn pt_swap64_s(lo: u32, hi: u32) -> u64 {
        ptb::swap64(lo, hi)
    }
    /// `cdecl(lo, hi)`: the 64-bit byte swap (`PUInt64`).
    Phyre_swapBytes64_004392c0 @ 0x0043_92c0 => extern "C" fn pt_swap64_u(lo: u32, hi: u32) -> u64 {
        ptb::swap64(lo, hi)
    }
    /// `PTypeDefault<char>`'s scalar deleting destructor.
    Phyre_PTypeDefault_char_deletingDtor @ 0x0043_8e00 => extern "thiscall" fn d_00438e00(this: u32, flags: u32) -> u32 {
        ptb::delete_type(&mut mem(), this, flags)
    }
    /// `PTypeDefault<char>::getValueFromScriptContext(L, out)`.
    Phyre_PTypeDefault_char_getValueFromScriptContext @ 0x0043_9670 => extern "stdcall" fn g_00439670(l: u32, out: u32) -> u32 {
        ptb::get_from_script(&mut mem(), Kind::I8, l, out)
    }
    /// `PTypeDefault<char>::putValueToScriptContext(L, value)`.
    Phyre_PTypeDefault_char__putValueToScriptContext @ 0x0043_9910 => extern "stdcall" fn p_00439910(l: u32, value: u32) -> u32 {
        ptb::put_to_script(&mut mem(), Kind::I8, l, value)
    }
    /// `PTypeDefault<char>::swapEndian(p, n)`.
    Phyre_PTypeDefault_char_swapEndian @ 0x0043_9bd0 => extern "stdcall" fn s_00439bd0(p: u32, _n: u32) -> u32 {
        ptb::swap_endian(&mut mem(), Kind::I8, p)
    }
    /// `PTypeDefault<PUInt8>`'s scalar deleting destructor.
    Phyre_PTypeDefault_PUInt8_deletingDtor @ 0x0043_8e30 => extern "thiscall" fn d_00438e30(this: u32, flags: u32) -> u32 {
        ptb::delete_type(&mut mem(), this, flags)
    }
    /// `PTypeDefault<PUInt8>::getValueFromScriptContext(L, out)`.
    Phyre_PTypeDefault_PUInt8_getValueFromScriptContext @ 0x0043_96a0 => extern "stdcall" fn g_004396a0(l: u32, out: u32) -> u32 {
        ptb::get_from_script(&mut mem(), Kind::U8, l, out)
    }
    /// `PTypeDefault<PUInt8>::putValueToScriptContext(L, value)`.
    Phyre_PTypeDefault_unsigned_char__putValueToScriptContext @ 0x0043_9940 => extern "stdcall" fn p_00439940(l: u32, value: u32) -> u32 {
        ptb::put_to_script(&mut mem(), Kind::U8, l, value)
    }
    /// `PTypeDefault<PUInt8>::swapEndian(p, n)`.
    Phyre_PTypeDefault_PUInt8_swapEndian @ 0x0043_9be0 => extern "stdcall" fn s_00439be0(p: u32, _n: u32) -> u32 {
        ptb::swap_endian(&mut mem(), Kind::U8, p)
    }
    /// `PTypeDefault<PInt16>`'s scalar deleting destructor.
    Phyre_PTypeDefault_PInt16_deletingDtor @ 0x0043_8e60 => extern "thiscall" fn d_00438e60(this: u32, flags: u32) -> u32 {
        ptb::delete_type(&mut mem(), this, flags)
    }
    /// `PTypeDefault<PInt16>::getValueFromScriptContext(L, out)`.
    Phyre_PTypeDefault_PInt16_getValueFromScriptContext @ 0x0043_96d0 => extern "stdcall" fn g_004396d0(l: u32, out: u32) -> u32 {
        ptb::get_from_script(&mut mem(), Kind::I16, l, out)
    }
    /// `PTypeDefault<PInt16>::putValueToScriptContext(L, value)`.
    Phyre_PTypeDefault_short__putValueToScriptContext @ 0x0043_9970 => extern "stdcall" fn p_00439970(l: u32, value: u32) -> u32 {
        ptb::put_to_script(&mut mem(), Kind::I16, l, value)
    }
    /// `PTypeDefault<PInt16>::swapEndian(p, n)`.
    Phyre_PTypeDefault_PInt16_swapEndian @ 0x0043_9bf0 => extern "stdcall" fn s_00439bf0(p: u32, _n: u32) -> u32 {
        ptb::swap_endian(&mut mem(), Kind::I16, p)
    }
    /// `PTypeDefault<PUInt16>`'s scalar deleting destructor.
    Phyre_PTypeDefault_PUInt16_deletingDtor @ 0x0043_8e90 => extern "thiscall" fn d_00438e90(this: u32, flags: u32) -> u32 {
        ptb::delete_type(&mut mem(), this, flags)
    }
    /// `PTypeDefault<PUInt16>::getValueFromScriptContext(L, out)`.
    Phyre_PTypeDefault_PUInt16_getValueFromScriptContext @ 0x0043_9700 => extern "stdcall" fn g_00439700(l: u32, out: u32) -> u32 {
        ptb::get_from_script(&mut mem(), Kind::U16, l, out)
    }
    /// `PTypeDefault<PUInt16>::putValueToScriptContext(L, value)`.
    Phyre_PTypeDefault_unsigned_short__putValueToScriptContext @ 0x0043_99a0 => extern "stdcall" fn p_004399a0(l: u32, value: u32) -> u32 {
        ptb::put_to_script(&mut mem(), Kind::U16, l, value)
    }
    /// `PTypeDefault<PUInt16>::swapEndian(p, n)`.
    Phyre_PTypeDefault_PUInt16_swapEndian @ 0x0043_9c20 => extern "stdcall" fn s_00439c20(p: u32, _n: u32) -> u32 {
        ptb::swap_endian(&mut mem(), Kind::U16, p)
    }
    /// `PTypeDefault<PInt32>`'s scalar deleting destructor.
    Phyre_PTypeDefault_PInt32_deletingDtor @ 0x0043_8ec0 => extern "thiscall" fn d_00438ec0(this: u32, flags: u32) -> u32 {
        ptb::delete_type(&mut mem(), this, flags)
    }
    /// `PTypeDefault<PInt32>::getValueFromScriptContext(L, out)`.
    Phyre_PTypeDefault_PInt32_getValueFromScriptContext @ 0x0043_9730 => extern "stdcall" fn g_00439730(l: u32, out: u32) -> u32 {
        ptb::get_from_script(&mut mem(), Kind::I32, l, out)
    }
    /// `PTypeDefault<PInt32>::putValueToScriptContext(L, value)`.
    Phyre_PTypeDefault_PInt32_putValueToScriptContext @ 0x0043_99d0 => extern "stdcall" fn p_004399d0(l: u32, value: u32) -> u32 {
        ptb::put_to_script(&mut mem(), Kind::I32, l, value)
    }
    /// `PTypeDefault<PInt32>::swapEndian(p, n)`.
    Phyre_PTypeDefault_PInt32_swapEndian @ 0x0043_9c40 => extern "stdcall" fn s_00439c40(p: u32, _n: u32) -> u32 {
        ptb::swap_endian(&mut mem(), Kind::I32, p)
    }
    /// `PTypeDefault<PUInt32>`'s scalar deleting destructor.
    Phyre_PTypeDefault_PUInt32_deletingDtor @ 0x0043_8ef0 => extern "thiscall" fn d_00438ef0(this: u32, flags: u32) -> u32 {
        ptb::delete_type(&mut mem(), this, flags)
    }
    /// `PTypeDefault<PUInt32>::getValueFromScriptContext(L, out)`.
    Phyre_PTypeDefault_PUInt32_getValueFromScriptContext @ 0x0043_9760 => extern "stdcall" fn g_00439760(l: u32, out: u32) -> u32 {
        ptb::get_from_script(&mut mem(), Kind::U32, l, out)
    }
    /// `PTypeDefault<PUInt32>::putValueToScriptContext(L, value)`.
    Phyre_PTypeDefault_PUInt32_putValueToScriptContext @ 0x0043_9a00 => extern "stdcall" fn p_00439a00(l: u32, value: u32) -> u32 {
        ptb::put_to_script(&mut mem(), Kind::U32, l, value)
    }
    /// `PTypeDefault<PUInt32>::swapEndian(p, n)`.
    Phyre_PTypeDefault_PUInt32_swapEndian @ 0x0043_9c80 => extern "stdcall" fn s_00439c80(p: u32, _n: u32) -> u32 {
        ptb::swap_endian(&mut mem(), Kind::U32, p)
    }
    /// `PTypeDefault<float>`'s scalar deleting destructor.
    Phyre_PTypeDefault_float_deletingDtor @ 0x0043_8f20 => extern "thiscall" fn d_00438f20(this: u32, flags: u32) -> u32 {
        ptb::delete_type(&mut mem(), this, flags)
    }
    /// `PTypeDefault<float>::getValueFromScriptContext(L, out)`.
    Phyre_PTypeDefault_float_getValueFromScriptContext @ 0x0043_9790 => extern "stdcall" fn g_00439790(l: u32, out: u32) -> u32 {
        ptb::get_from_script(&mut mem(), Kind::F32, l, out)
    }
    /// `PTypeDefault<float>::putValueToScriptContext(L, value)`.
    Phyre_PTypeDefault_float__putValueToScriptContext @ 0x0043_9a30 => extern "stdcall" fn p_00439a30(l: u32, value: u32) -> u32 {
        ptb::put_to_script(&mut mem(), Kind::F32, l, value)
    }
    /// `PTypeDefault<float>::swapEndian(p, n)`.
    Phyre_PTypeDefault_float_swapEndian @ 0x0043_9cc0 => extern "stdcall" fn s_00439cc0(p: u32, _n: u32) -> u32 {
        ptb::swap_endian(&mut mem(), Kind::F32, p)
    }
    /// `PTypeDefault<double>`'s scalar deleting destructor.
    Phyre_PTypeDefault_double_deletingDtor @ 0x0043_8f50 => extern "thiscall" fn d_00438f50(this: u32, flags: u32) -> u32 {
        ptb::delete_type(&mut mem(), this, flags)
    }
    /// `PTypeDefault<double>::getValueFromScriptContext(L, out)`.
    Phyre_PTypeDefault_double_getValueFromScriptContext @ 0x0043_97c0 => extern "stdcall" fn g_004397c0(l: u32, out: u32) -> u32 {
        ptb::get_from_script(&mut mem(), Kind::F64, l, out)
    }
    /// `PTypeDefault<double>::putValueToScriptContext(L, value)`.
    Phyre_PTypeDefault_double__putValueToScriptContext @ 0x0043_9a60 => extern "stdcall" fn p_00439a60(l: u32, value: u32) -> u32 {
        ptb::put_to_script(&mut mem(), Kind::F64, l, value)
    }
    /// `PTypeDefault<double>::swapEndian(p, n)`.
    Phyre_PTypeDefault_double_swapEndian @ 0x0043_9d00 => extern "stdcall" fn s_00439d00(p: u32, _n: u32) -> u32 {
        ptb::swap_endian(&mut mem(), Kind::F64, p)
    }
    /// `PTypeDefault<PQWord>`'s scalar deleting destructor.
    Phyre_PTypeDefault_PQWord_deletingDtor @ 0x0043_8f80 => extern "thiscall" fn d_00438f80(this: u32, flags: u32) -> u32 {
        ptb::delete_type(&mut mem(), this, flags)
    }
    /// `PTypeDefault<PQWord>::getValueFromScriptContext(L, out)`.
    Phyre_PTypeDefault_PQWord_getValueFromScriptContext @ 0x0043_97f0 => extern "stdcall" fn g_004397f0(l: u32, out: u32) -> u32 {
        ptb::get_from_script(&mut mem(), Kind::QWord, l, out)
    }
    /// `PTypeDefault<PQWord>::putValueToScriptContext(L, value)`.
    Phyre_PTypeDefault_PQWord_putValueToScriptContext @ 0x0043_9a90 => extern "stdcall" fn p_00439a90(l: u32, value: u32) -> u32 {
        ptb::put_to_script(&mut mem(), Kind::QWord, l, value)
    }
    /// `PTypeDefault<PQWord>::swapEndian(p, n)`.
    Phyre_PTypeDefault_PQWord_swapEndian @ 0x0043_9d30 => extern "stdcall" fn s_00439d30(p: u32, _n: u32) -> u32 {
        ptb::swap_endian(&mut mem(), Kind::QWord, p)
    }
    /// `PTypeDefault<PInt64>`'s scalar deleting destructor.
    Phyre_PTypeDefault_PInt64_deletingDtor @ 0x0043_8fb0 => extern "thiscall" fn d_00438fb0(this: u32, flags: u32) -> u32 {
        ptb::delete_type(&mut mem(), this, flags)
    }
    /// `PTypeDefault<PInt64>::getValueFromScriptContext(L, out)`.
    Phyre_PTypeDefault_PInt64_getValueFromScriptContext @ 0x0043_9840 => extern "stdcall" fn g_00439840(l: u32, out: u32) -> u32 {
        ptb::get_from_script(&mut mem(), Kind::I64, l, out)
    }
    /// `PTypeDefault<PInt64>::putValueToScriptContext(L, value)`.
    Phyre_PTypeDefault_PInt64_putValueToScriptContext @ 0x0043_9ac0 => extern "stdcall" fn p_00439ac0(l: u32, value: u32) -> u32 {
        ptb::put_to_script(&mut mem(), Kind::I64, l, value)
    }
    /// `PTypeDefault<PInt64>::swapEndian(p, n)`.
    Phyre_PTypeDefault_PInt64_swapEndian @ 0x0043_9d70 => extern "stdcall" fn s_00439d70(p: u32, _n: u32) -> u32 {
        ptb::swap_endian(&mut mem(), Kind::I64, p)
    }
    /// `PTypeDefault<PUInt64>`'s scalar deleting destructor.
    Phyre_PTypeDefault_PUInt64_deletingDtor @ 0x0043_8fe0 => extern "thiscall" fn d_00438fe0(this: u32, flags: u32) -> u32 {
        ptb::delete_type(&mut mem(), this, flags)
    }
    /// `PTypeDefault<PUInt64>::getValueFromScriptContext(L, out)`.
    Phyre_PTypeDefault_PUInt64_getValueFromScriptContext @ 0x0043_9880 => extern "stdcall" fn g_00439880(l: u32, out: u32) -> u32 {
        ptb::get_from_script(&mut mem(), Kind::U64, l, out)
    }
    /// `PTypeDefault<PUInt64>::putValueToScriptContext(L, value)`.
    Phyre_PTypeDefault_PUInt64_putValueToScriptContext @ 0x0043_9af0 => extern "stdcall" fn p_00439af0(l: u32, value: u32) -> u32 {
        ptb::put_to_script(&mut mem(), Kind::U64, l, value)
    }
    /// `PTypeDefault<PUInt64>::swapEndian(p, n)`.
    Phyre_PTypeDefault_PUInt64_swapEndian @ 0x0043_9d90 => extern "stdcall" fn s_00439d90(p: u32, _n: u32) -> u32 {
        ptb::swap_endian(&mut mem(), Kind::U64, p)
    }
    /// `PTypeDefault<bool>`'s scalar deleting destructor.
    Phyre_PTypeDefault_bool_deletingDtor @ 0x0043_9010 => extern "thiscall" fn d_00439010(this: u32, flags: u32) -> u32 {
        ptb::delete_type(&mut mem(), this, flags)
    }
    /// `PTypeDefault<bool>::getValueFromScriptContext(L, out)`.
    Phyre_PTypeDefault_bool__getValueFromScriptContext @ 0x0043_98c0 => extern "stdcall" fn g_004398c0(l: u32, out: u32) -> u32 {
        ptb::get_from_script(&mut mem(), Kind::Bool, l, out)
    }
    /// `PTypeDefault<bool>::putValueToScriptContext(L, value)`.
    Phyre_PTypeDefault_bool__putValueToScriptContext @ 0x0043_9b20 => extern "stdcall" fn p_00439b20(l: u32, value: u32) -> u32 {
        ptb::put_to_script(&mut mem(), Kind::Bool, l, value)
    }
    /// `PTypeDefault<bool>::swapEndian(p, n)`.
    Phyre_PTypeDefault_bool_swapEndian @ 0x0043_9db0 => extern "stdcall" fn s_00439db0(p: u32, _n: u32) -> u32 {
        ptb::swap_endian(&mut mem(), Kind::Bool, p)
    }
}
