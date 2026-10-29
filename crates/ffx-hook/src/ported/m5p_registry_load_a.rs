//! M5 stage 1, list `phyre-registry-load-a`: `PhyreEngine`'s type
//! registry and loading code (`ffx_pload::m5p_registry_load_a*`). The
//! parity tests are `crates/ffx-oracle/tests/m5p_registry_load_a.rs`.
//!
//! Threads: the conversion kernels run where clusters load (the loader
//! thread and the main thread); no shim keeps state, each works on the
//! game's memory through [`mem`].

use ffx_pload::m5p_registry_load_a_convert as cv;

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
}
