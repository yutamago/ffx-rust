//! M5 stage 1, list phyre-registry-load-b (`ffx_pload::m5p_registry_b`):
//! the conversion kernels and the type system's helpers.
//!
//! Threads: clusters load on the main thread and the loader; the kernels
//! and helpers keep no state.

use super::StackFrame;
#[allow(unused_imports)]
use super::{call_cdecl, mem, ported};
use ffx_pload::m5p_registry_b::contacts as ct;
use ffx_pload::m5p_registry_b::geometry as gm;
use ffx_pload::m5p_registry_b::helpers as h;
use ffx_pload::m5p_registry_b::mapping as mp;
use ffx_pload::m5p_registry_b::more as m;
use ffx_pload::m5p_registry_b::rest as r;
use ffx_pload::m5p_registry_b::swap as s;
use ffx_pload::m5p_registry_b::{self as rb, bridge as b};
use ffx_pload::m5p_tmpl_fortype as ft;

ported! {
    /// `Phyre_PSerialization_convert_float_to_bool(src, dst, &counts, entry)`: the `bool(float)` conversion kernel.
    Phyre_PSerialization_convert_float_to_bool @ 0x0047_3e10 => extern "C" fn k_00473e10(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_3e10) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_double_to_PChar(src, dst, &counts, entry)`: the `PChar(double)` conversion kernel.
    Phyre_PSerialization_convert_double_to_PChar @ 0x0047_3f00 => extern "C" fn k_00473f00(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_3f00) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_double_to_PUInt8(src, dst, &counts, entry)`: the `PUInt8(double)` conversion kernel.
    Phyre_PSerialization_convert_double_to_PUInt8 @ 0x0047_3f60 => extern "C" fn k_00473f60(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_3f60) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_double_to_PInt16(src, dst, &counts, entry)`: the `PInt16(double)` conversion kernel.
    Phyre_PSerialization_convert_double_to_PInt16 @ 0x0047_3fd0 => extern "C" fn k_00473fd0(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_3fd0) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_double_to_PUInt16(src, dst, &counts, entry)`: the `PUInt16(double)` conversion kernel.
    Phyre_PSerialization_convert_double_to_PUInt16 @ 0x0047_4030 => extern "C" fn k_00474030(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_4030) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_double_to_PInt32(src, dst, &counts, entry)`: the `PInt32(double)` conversion kernel.
    Phyre_PSerialization_convert_double_to_PInt32 @ 0x0047_40a0 => extern "C" fn k_004740a0(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_40a0) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_double_to_unsigned_int(src, dst, &counts, entry)`: the `unsigned_int(double)` conversion kernel.
    Phyre_PSerialization_convert_double_to_unsigned_int @ 0x0047_4100 => extern "C" fn k_00474100(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_4100) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_double_to_float(src, dst, &counts, entry)`: the `float(double)` conversion kernel.
    Phyre_PSerialization_convert_double_to_float @ 0x0047_4170 => extern "C" fn k_00474170(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_4170) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_double_to_double(src, dst, &counts, entry)`: the `double(double)` conversion kernel.
    Phyre_PSerialization_convert_double_to_double @ 0x0047_4210 => extern "C" fn k_00474210(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_4210) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_double_to_PUInt128(src, dst, &counts, entry)`: the `PUInt128(double)` conversion kernel.
    Phyre_PSerialization_convert_double_to_PUInt128 @ 0x0047_42b0 => extern "C" fn k_004742b0(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_42b0) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_double_to_PInt64(src, dst, &counts, entry)`: the `PInt64(double)` conversion kernel.
    Phyre_PSerialization_convert_double_to_PInt64 @ 0x0047_4360 => extern "C" fn k_00474360(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_4360) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_double_to_PUInt64(src, dst, &counts, entry)`: the `PUInt64(double)` conversion kernel.
    Phyre_PSerialization_convert_double_to_PUInt64 @ 0x0047_43c0 => extern "C" fn k_004743c0(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_43c0) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_double_to_bool(src, dst, &counts, entry)`: the `bool(double)` conversion kernel.
    Phyre_PSerialization_convert_double_to_bool @ 0x0047_4460 => extern "C" fn k_00474460(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_4460) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_PUInt128_to_PChar(src, dst, &counts, entry)`: the `PChar(PUInt128)` conversion kernel.
    Phyre_PSerialization_convert_PUInt128_to_PChar @ 0x0047_4550 => extern "C" fn k_00474550(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_4550) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_PUInt128_to_PUInt8(src, dst, &counts, entry)`: the `PUInt8(PUInt128)` conversion kernel.
    Phyre_PSerialization_convert_PUInt128_to_PUInt8 @ 0x0047_45a0 => extern "C" fn k_004745a0(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_45a0) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_PUInt128_to_PInt16(src, dst, &counts, entry)`: the `PInt16(PUInt128)` conversion kernel.
    Phyre_PSerialization_convert_PUInt128_to_PInt16 @ 0x0047_45f0 => extern "C" fn k_004745f0(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_45f0) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_PUInt128_to_PUInt16(src, dst, &counts, entry)`: the `PUInt16(PUInt128)` conversion kernel.
    Phyre_PSerialization_convert_PUInt128_to_PUInt16 @ 0x0047_4650 => extern "C" fn k_00474650(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_4650) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_PUInt128_to_PInt32(src, dst, &counts, entry)`: the `PInt32(PUInt128)` conversion kernel.
    Phyre_PSerialization_convert_PUInt128_to_PInt32 @ 0x0047_46b0 => extern "C" fn k_004746b0(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_46b0) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_PUInt128_to_unsigned_int(src, dst, &counts, entry)`: the `unsigned_int(PUInt128)` conversion kernel.
    Phyre_PSerialization_convert_PUInt128_to_unsigned_int @ 0x0047_4700 => extern "C" fn k_00474700(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_4700) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_PUInt128_to_float(src, dst, &counts, entry)`: the `float(PUInt128)` conversion kernel.
    Phyre_PSerialization_convert_PUInt128_to_float @ 0x0047_4750 => extern "C" fn k_00474750(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_4750) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_PUInt128_to_double(src, dst, &counts, entry)`: the `double(PUInt128)` conversion kernel.
    Phyre_PSerialization_convert_PUInt128_to_double @ 0x0047_47f0 => extern "C" fn k_004747f0(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_47f0) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_PUInt128_to_PUInt128(src, dst, &counts, entry)`: the `PUInt128(PUInt128)` conversion kernel.
    Phyre_PSerialization_convert_PUInt128_to_PUInt128 @ 0x0047_4890 => extern "C" fn k_00474890(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_4890) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_PUInt128_to_PInt64(src, dst, &counts, entry)`: the `PInt64(PUInt128)` conversion kernel.
    Phyre_PSerialization_convert_PUInt128_to_PInt64 @ 0x0047_4910 => extern "C" fn k_00474910(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_4910) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_PUInt128_to_PUInt64(src, dst, &counts, entry)`: the `PUInt64(PUInt128)` conversion kernel.
    Phyre_PSerialization_convert_PUInt128_to_PUInt64 @ 0x0047_4970 => extern "C" fn k_00474970(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_4970) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_PUInt128_to_bool(src, dst, &counts, entry)`: the `bool(PUInt128)` conversion kernel.
    Phyre_PSerialization_convert_PUInt128_to_bool @ 0x0047_49d0 => extern "C" fn k_004749d0(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_49d0) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_PInt64_to_PChar(src, dst, &counts, entry)`: the `PChar(PInt64)` conversion kernel.
    Phyre_PSerialization_convert_PInt64_to_PChar @ 0x0047_4a30 => extern "C" fn k_00474a30(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_4a30) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_PInt64_to_PUInt8(src, dst, &counts, entry)`: the `PUInt8(PInt64)` conversion kernel.
    Phyre_PSerialization_convert_PInt64_to_PUInt8 @ 0x0047_4a80 => extern "C" fn k_00474a80(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_4a80) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_PInt64_to_PInt16(src, dst, &counts, entry)`: the `PInt16(PInt64)` conversion kernel.
    Phyre_PSerialization_convert_PInt64_to_PInt16 @ 0x0047_4ad0 => extern "C" fn k_00474ad0(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_4ad0) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_PInt64_to_PUInt16(src, dst, &counts, entry)`: the `PUInt16(PInt64)` conversion kernel.
    Phyre_PSerialization_convert_PInt64_to_PUInt16 @ 0x0047_4b30 => extern "C" fn k_00474b30(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_4b30) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_PInt64_to_PInt32(src, dst, &counts, entry)`: the `PInt32(PInt64)` conversion kernel.
    Phyre_PSerialization_convert_PInt64_to_PInt32 @ 0x0047_4b90 => extern "C" fn k_00474b90(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_4b90) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_PInt64_to_unsigned_int(src, dst, &counts, entry)`: the `unsigned_int(PInt64)` conversion kernel.
    Phyre_PSerialization_convert_PInt64_to_unsigned_int @ 0x0047_4be0 => extern "C" fn k_00474be0(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_4be0) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_PInt64_to_float(src, dst, &counts, entry)`: the `float(PInt64)` conversion kernel.
    Phyre_PSerialization_convert_PInt64_to_float @ 0x0047_4c30 => extern "C" fn k_00474c30(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_4c30) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_PInt64_to_double(src, dst, &counts, entry)`: the `double(PInt64)` conversion kernel.
    Phyre_PSerialization_convert_PInt64_to_double @ 0x0047_4cd0 => extern "C" fn k_00474cd0(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_4cd0) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_PInt64_to_PUInt128(src, dst, &counts, entry)`: the `PUInt128(PInt64)` conversion kernel.
    Phyre_PSerialization_convert_PInt64_to_PUInt128 @ 0x0047_4d70 => extern "C" fn k_00474d70(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_4d70) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_PInt64_to_PInt64(src, dst, &counts, entry)`: the `PInt64(PInt64)` conversion kernel.
    Phyre_PSerialization_convert_PInt64_to_PInt64 @ 0x0047_4de0 => extern "C" fn k_00474de0(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_4de0) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_PInt64_to_PUInt64(src, dst, &counts, entry)`: the `PUInt64(PInt64)` conversion kernel.
    Phyre_PSerialization_convert_PInt64_to_PUInt64 @ 0x0047_4e40 => extern "C" fn k_00474e40(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_4e40) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_PInt64_to_bool(src, dst, &counts, entry)`: the `bool(PInt64)` conversion kernel.
    Phyre_PSerialization_convert_PInt64_to_bool @ 0x0047_4ea0 => extern "C" fn k_00474ea0(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_4ea0) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_PUInt64_to_PChar(src, dst, &counts, entry)`: the `PChar(PUInt64)` conversion kernel.
    Phyre_PSerialization_convert_PUInt64_to_PChar @ 0x0047_4f00 => extern "C" fn k_00474f00(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_4f00) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_PUInt64_to_PUInt8(src, dst, &counts, entry)`: the `PUInt8(PUInt64)` conversion kernel.
    Phyre_PSerialization_convert_PUInt64_to_PUInt8 @ 0x0047_4f50 => extern "C" fn k_00474f50(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_4f50) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_PUInt64_to_PInt16(src, dst, &counts, entry)`: the `PInt16(PUInt64)` conversion kernel.
    Phyre_PSerialization_convert_PUInt64_to_PInt16 @ 0x0047_4fa0 => extern "C" fn k_00474fa0(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_4fa0) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_PUInt64_to_PUInt16(src, dst, &counts, entry)`: the `PUInt16(PUInt64)` conversion kernel.
    Phyre_PSerialization_convert_PUInt64_to_PUInt16 @ 0x0047_5000 => extern "C" fn k_00475000(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_5000) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_PUInt64_to_PInt32(src, dst, &counts, entry)`: the `PInt32(PUInt64)` conversion kernel.
    Phyre_PSerialization_convert_PUInt64_to_PInt32 @ 0x0047_5060 => extern "C" fn k_00475060(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_5060) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_PUInt64_to_unsigned_int(src, dst, &counts, entry)`: the `unsigned_int(PUInt64)` conversion kernel.
    Phyre_PSerialization_convert_PUInt64_to_unsigned_int @ 0x0047_50b0 => extern "C" fn k_004750b0(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_50b0) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_PUInt64_to_float(src, dst, &counts, entry)`: the `float(PUInt64)` conversion kernel.
    Phyre_PSerialization_convert_PUInt64_to_float @ 0x0047_5100 => extern "C" fn k_00475100(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_5100) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_PUInt64_to_double(src, dst, &counts, entry)`: the `double(PUInt64)` conversion kernel.
    Phyre_PSerialization_convert_PUInt64_to_double @ 0x0047_5290 => extern "C" fn k_00475290(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_5290) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_PUInt64_to_PUInt128(src, dst, &counts, entry)`: the `PUInt128(PUInt64)` conversion kernel.
    Phyre_PSerialization_convert_PUInt64_to_PUInt128 @ 0x0047_5420 => extern "C" fn k_00475420(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_5420) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_PUInt64_to_PInt64(src, dst, &counts, entry)`: the `PInt64(PUInt64)` conversion kernel.
    Phyre_PSerialization_convert_PUInt64_to_PInt64 @ 0x0047_5490 => extern "C" fn k_00475490(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_5490) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_PUInt64_to_PUInt64(src, dst, &counts, entry)`: the `PUInt64(PUInt64)` conversion kernel.
    Phyre_PSerialization_convert_PUInt64_to_PUInt64 @ 0x0047_54f0 => extern "C" fn k_004754f0(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_54f0) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_PUInt64_to_bool(src, dst, &counts, entry)`: the `bool(PUInt64)` conversion kernel.
    Phyre_PSerialization_convert_PUInt64_to_bool @ 0x0047_5550 => extern "C" fn k_00475550(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_5550) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_bool_to_PChar(src, dst, &counts, entry)`: the `PChar(bool)` conversion kernel.
    Phyre_PSerialization_convert_bool_to_PChar @ 0x0047_55b0 => extern "C" fn k_004755b0(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_55b0) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_bool_to_PUInt8(src, dst, &counts, entry)`: the `PUInt8(bool)` conversion kernel.
    Phyre_PSerialization_convert_bool_to_PUInt8 @ 0x0047_5600 => extern "C" fn k_00475600(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_5600) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_bool_to_PInt16(src, dst, &counts, entry)`: the `PInt16(bool)` conversion kernel.
    Phyre_PSerialization_convert_bool_to_PInt16 @ 0x0047_5650 => extern "C" fn k_00475650(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_5650) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_bool_to_PUInt16(src, dst, &counts, entry)`: the `PUInt16(bool)` conversion kernel.
    Phyre_PSerialization_convert_bool_to_PUInt16 @ 0x0047_56b0 => extern "C" fn k_004756b0(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_56b0) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_bool_to_PInt32(src, dst, &counts, entry)`: the `PInt32(bool)` conversion kernel.
    Phyre_PSerialization_convert_bool_to_PInt32 @ 0x0047_5710 => extern "C" fn k_00475710(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_5710) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_bool_to_unsigned_int(src, dst, &counts, entry)`: the `unsigned_int(bool)` conversion kernel.
    Phyre_PSerialization_convert_bool_to_unsigned_int @ 0x0047_5770 => extern "C" fn k_00475770(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_5770) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_bool_to_float(src, dst, &counts, entry)`: the `float(bool)` conversion kernel.
    Phyre_PSerialization_convert_bool_to_float @ 0x0047_57d0 => extern "C" fn k_004757d0(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_57d0) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_bool_to_double(src, dst, &counts, entry)`: the `double(bool)` conversion kernel.
    Phyre_PSerialization_convert_bool_to_double @ 0x0047_5890 => extern "C" fn k_00475890(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_5890) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_bool_to_PUInt128(src, dst, &counts, entry)`: the `PUInt128(bool)` conversion kernel.
    Phyre_PSerialization_convert_bool_to_PUInt128 @ 0x0047_5950 => extern "C" fn k_00475950(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_5950) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_bool_to_PInt64(src, dst, &counts, entry)`: the `PInt64(bool)` conversion kernel.
    Phyre_PSerialization_convert_bool_to_PInt64 @ 0x0047_59c0 => extern "C" fn k_004759c0(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_59c0) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_bool_to_PUInt64(src, dst, &counts, entry)`: the `PUInt64(bool)` conversion kernel.
    Phyre_PSerialization_convert_bool_to_PUInt64 @ 0x0047_5a20 => extern "C" fn k_00475a20(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_5a20) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_bool_to_bool(src, dst, &counts, entry)`: the `bool(bool)` conversion kernel.
    Phyre_PSerialization_convert_bool_to_bool @ 0x0047_5a80 => extern "C" fn k_00475a80(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_5a80) }, src, dst, counts, entry);
    }
    /// `Phyre_PSerialization_convert_PChar_to_PChar(src, dst, &counts, entry)`: the `PChar(PChar)` conversion kernel.
    Phyre_PSerialization_convert_PChar_to_PChar @ 0x0047_6c20 => extern "C" fn k_00476c20(src: u32, dst: u32, counts: u32, entry: u32) {
        b::kernel(&mut mem(), const { rb::kernel_of(0x0047_6c20) }, src, dst, counts, entry);
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_unsigned_char_4__swapEndiannessStatic1`: `swapEndianness` of `p` on the static descriptor built on first use (flag 1).
    Phyre_PClassDescriptorForType_Phyre_PArray_unsigned_char_4__swapEndiannessStatic1 @ 0x0049_98a0 => extern "C" fn lazy_004998a0(p: u32) -> u32 {
        ft::bridge::swap_lazy(&mut mem(), &const { h::of(h::LAZY, 0x0049_98a0) }, p)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_unsigned_char_4__swapEndiannessStatic0`: `swapEndianness` of `p` on the static descriptor built on first use (flag 0).
    Phyre_PClassDescriptorForType_Phyre_PArray_unsigned_char_4__swapEndiannessStatic0 @ 0x0049_9950 => extern "C" fn lazy_00499950(p: u32) -> u32 {
        ft::bridge::swap_lazy(&mut mem(), &const { h::of(h::LAZY, 0x0049_9950) }, p)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PRendering_PShadowCaster_PShadowSplit_4__swapEndiannessStatic1`: `swapEndianness` of `p` on the static descriptor built on first use (flag 1).
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PRendering_PShadowCaster_PShadowSplit_4__swapEndiannessStatic1 @ 0x004c_8ce0 => extern "C" fn lazy_004c8ce0(p: u32) -> u32 {
        ft::bridge::swap_lazy(&mut mem(), &const { h::of(h::LAZY, 0x004c_8ce0) }, p)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PRendering_PShadowCaster_PShadowSplit_4__swapEndiannessStatic0`: `swapEndianness` of `p` on the static descriptor built on first use (flag 0).
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PRendering_PShadowCaster_PShadowSplit_4__swapEndiannessStatic0 @ 0x004c_8db0 => extern "C" fn lazy_004c8db0(p: u32) -> u32 {
        ft::bridge::swap_lazy(&mut mem(), &const { h::of(h::LAZY, 0x004c_8db0) }, p)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PSharray_unsigned_int__swapEndiannessStatic1`: `swapEndianness` of `p` on the static descriptor built on first use (flag 1).
    Phyre_PClassDescriptorForType_Phyre_PSharray_unsigned_int__swapEndiannessStatic1 @ 0x004c_ef50 => extern "C" fn lazy_004cef50(p: u32) -> u32 {
        ft::bridge::swap_lazy(&mut mem(), &const { h::of(h::LAZY, 0x004c_ef50) }, p)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PSharray_unsigned_int__swapEndiannessStatic0`: `swapEndianness` of `p` on the static descriptor built on first use (flag 0).
    Phyre_PClassDescriptorForType_Phyre_PSharray_unsigned_int__swapEndiannessStatic0 @ 0x004c_f000 => extern "C" fn lazy_004cf000(p: u32) -> u32 {
        ft::bridge::swap_lazy(&mut mem(), &const { h::of(h::LAZY, 0x004c_f000) }, p)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_float_4__swapEndiannessStatic1`: `swapEndianness` of `p` on the static descriptor built on first use (flag 1).
    Phyre_PClassDescriptorForType_Phyre_PArray_float_4__swapEndiannessStatic1 @ 0x0050_93a0 => extern "C" fn lazy_005093a0(p: u32) -> u32 {
        ft::bridge::swap_lazy(&mut mem(), &const { h::of(h::LAZY, 0x0050_93a0) }, p)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_float_4__swapEndiannessStatic0`: `swapEndianness` of `p` on the static descriptor built on first use (flag 0).
    Phyre_PClassDescriptorForType_Phyre_PArray_float_4__swapEndiannessStatic0 @ 0x0050_9450 => extern "C" fn lazy_00509450(p: u32) -> u32 {
        ft::bridge::swap_lazy(&mut mem(), &const { h::of(h::LAZY, 0x0050_9450) }, p)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PAnimationChannelTarget__swapEndiannessStatic1`: `swapEndianness` of `p` on the static descriptor built on first use (flag 1).
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PAnimationChannelTarget__swapEndiannessStatic1 @ 0x0051_8550 => extern "C" fn lazy_00518550(p: u32) -> u32 {
        ft::bridge::swap_lazy(&mut mem(), &const { h::of(h::LAZY, 0x0051_8550) }, p)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PAnimationSlotListIndex__swapEndiannessStatic1`: `swapEndianness` of `p` on the static descriptor built on first use (flag 1).
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PAnimationSlotListIndex__swapEndiannessStatic1 @ 0x0051_85e0 => extern "C" fn lazy_005185e0(p: u32) -> u32 {
        ft::bridge::swap_lazy(&mut mem(), &const { h::of(h::LAZY, 0x0051_85e0) }, p)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PSharray_Phyre_PAnimation_PAnimationClip_ptr__swapEndiannessStatic1`: `swapEndianness` of `p` on the static descriptor built on first use (flag 1).
    Phyre_PClassDescriptorForType_Phyre_PSharray_Phyre_PAnimation_PAnimationClip_ptr__swapEndiannessStatic1 @ 0x0051_8670 => extern "C" fn lazy_00518670(p: u32) -> u32 {
        ft::bridge::swap_lazy(&mut mem(), &const { h::of(h::LAZY, 0x0051_8670) }, p)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PAnimationChannelTarget__swapEndiannessStatic0`: `swapEndianness` of `p` on the static descriptor built on first use (flag 0).
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PAnimationChannelTarget__swapEndiannessStatic0 @ 0x0051_8720 => extern "C" fn lazy_00518720(p: u32) -> u32 {
        ft::bridge::swap_lazy(&mut mem(), &const { h::of(h::LAZY, 0x0051_8720) }, p)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PAnimationSlotListIndex__swapEndiannessStatic0`: `swapEndianness` of `p` on the static descriptor built on first use (flag 0).
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PAnimationSlotListIndex__swapEndiannessStatic0 @ 0x0051_87b0 => extern "C" fn lazy_005187b0(p: u32) -> u32 {
        ft::bridge::swap_lazy(&mut mem(), &const { h::of(h::LAZY, 0x0051_87b0) }, p)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PSharray_Phyre_PAnimation_PAnimationClip_ptr__swapEndiannessStatic0`: `swapEndianness` of `p` on the static descriptor built on first use (flag 0).
    Phyre_PClassDescriptorForType_Phyre_PSharray_Phyre_PAnimation_PAnimationClip_ptr__swapEndiannessStatic0 @ 0x0051_8840 => extern "C" fn lazy_00518840(p: u32) -> u32 {
        ft::bridge::swap_lazy(&mut mem(), &const { h::of(h::LAZY, 0x0051_8840) }, p)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PAnimationSlotFilterDeferredLoad__swapEndiannessStatic0`: `swapEndianness` of `p` on the static descriptor built on first use (flag 0).
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PAnimation_PAnimationSlotFilterDeferredLoad__swapEndiannessStatic0 @ 0x0052_7240 => extern "C" fn lazy_00527240(p: u32) -> u32 {
        ft::bridge::swap_lazy(&mut mem(), &const { h::of(h::LAZY, 0x0052_7240) }, p)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Vectormath_Aos_Vector3__swapEndiannessStatic1`: `swapEndianness` of `p` on the static descriptor built on first use (flag 1).
    Phyre_PClassDescriptorForType_Phyre_PArray_Vectormath_Aos_Vector3__swapEndiannessStatic1 @ 0x0054_6d20 => extern "C" fn lazy_00546d20(p: u32) -> u32 {
        ft::bridge::swap_lazy(&mut mem(), &const { h::of(h::LAZY, 0x0054_6d20) }, p)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Vectormath_Aos_Vector3__swapEndiannessStatic0`: `swapEndianness` of `p` on the static descriptor built on first use (flag 0).
    Phyre_PClassDescriptorForType_Phyre_PArray_Vectormath_Aos_Vector3__swapEndiannessStatic0 @ 0x0054_6de0 => extern "C" fn lazy_00546de0(p: u32) -> u32 {
        ft::bridge::swap_lazy(&mut mem(), &const { h::of(h::LAZY, 0x0054_6de0) }, p)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PSharray_Phyre_PGameplay_PTriggerReceiverComponent_ptr__swapEndiannessStatic1`: `swapEndianness` of `p` on the static descriptor built on first use (flag 1).
    Phyre_PClassDescriptorForType_Phyre_PSharray_Phyre_PGameplay_PTriggerReceiverComponent_ptr__swapEndiannessStatic1 @ 0x0054_dac0 => extern "C" fn lazy_0054dac0(p: u32) -> u32 {
        ft::bridge::swap_lazy(&mut mem(), &const { h::of(h::LAZY, 0x0054_dac0) }, p)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PSharray_Phyre_PGameplay_PTriggerReceiverComponent_ptr__swapEndiannessStatic0`: `swapEndianness` of `p` on the static descriptor built on first use (flag 0).
    Phyre_PClassDescriptorForType_Phyre_PSharray_Phyre_PGameplay_PTriggerReceiverComponent_ptr__swapEndiannessStatic0 @ 0x0054_db70 => extern "C" fn lazy_0054db70(p: u32) -> u32 {
        ft::bridge::swap_lazy(&mut mem(), &const { h::of(h::LAZY, 0x0054_db70) }, p)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PLOD_PLODLevel__swapEndiannessStatic0`: `swapEndianness` of `p` on the static descriptor built on first use (flag 0).
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PLOD_PLODLevel__swapEndiannessStatic0 @ 0x005a_1bd0 => extern "C" fn lazy_005a1bd0(p: u32) -> u32 {
        ft::bridge::swap_lazy(&mut mem(), &const { h::of(h::LAZY, 0x005a_1bd0) }, p)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PSharray_Phyre_PRendering_PMeshInstance_ptr__swapEndiannessStatic0`: `swapEndianness` of `p` on the static descriptor built on first use (flag 0).
    Phyre_PClassDescriptorForType_Phyre_PSharray_Phyre_PRendering_PMeshInstance_ptr__swapEndiannessStatic0 @ 0x005a_1c60 => extern "C" fn lazy_005a1c60(p: u32) -> u32 {
        ft::bridge::swap_lazy(&mut mem(), &const { h::of(h::LAZY, 0x005a_1c60) }, p)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PBase_ptr__swapEndiannessStatic0`: `swapEndianness` of `p` on the static descriptor built on first use (flag 0).
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PBase_ptr__swapEndiannessStatic0 @ 0x005b_0580 => extern "C" fn lazy_005b0580(p: u32) -> u32 {
        ft::bridge::swap_lazy(&mut mem(), &const { h::of(h::LAZY, 0x005b_0580) }, p)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PGeometry_PMesh__dtor`: the vtable, then `Phyre_PClassDescriptor_Destruct`; its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PGeometry_PMesh__dtor @ 0x0048_f3d0 => extern "thiscall" fn destruct_0048f3d0(this: u32) -> u32 {
        h::bridge::destruct(&mut mem(), this, const { h::of(h::DESTRUCT, 0x0048_f3d0) })
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PGeometry_PSkeletonJointBounds__dtor`: the vtable, then `Phyre_PClassDescriptor_Destruct`; its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PGeometry_PSkeletonJointBounds__dtor @ 0x0048_f3e0 => extern "thiscall" fn destruct_0048f3e0(this: u32) -> u32 {
        h::bridge::destruct(&mut mem(), this, const { h::of(h::DESTRUCT, 0x0048_f3e0) })
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderParameterCaptureBufferLocation__dtor`: the vtable, then `Phyre_PClassDescriptor_Destruct`; its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderParameterCaptureBufferLocation__dtor @ 0x004a_1720 => extern "thiscall" fn destruct_004a1720(this: u32) -> u32 {
        h::bridge::destruct(&mut mem(), this, const { h::of(h::DESTRUCT, 0x004a_1720) })
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderParameterCaptureBufferLocationSize__dtor`: the vtable, then `Phyre_PClassDescriptor_Destruct`; its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderParameterCaptureBufferLocationSize__dtor @ 0x004a_1730 => extern "thiscall" fn destruct_004a1730(this: u32) -> u32 {
        h::bridge::destruct(&mut mem(), this, const { h::of(h::DESTRUCT, 0x004a_1730) })
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderParameterCaptureBufferLocationType__dtor`: the vtable, then `Phyre_PClassDescriptor_Destruct`; its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderParameterCaptureBufferLocationType__dtor @ 0x004a_1740 => extern "thiscall" fn destruct_004a1740(this: u32) -> u32 {
        h::bridge::destruct(&mut mem(), this, const { h::of(h::DESTRUCT, 0x004a_1740) })
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderParameterCaptureBufferLocationTypeConstantBuffer__dtor`: the vtable, then `Phyre_PClassDescriptor_Destruct`; its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderParameterCaptureBufferLocationTypeConstantBuffer__dtor @ 0x004a_1750 => extern "thiscall" fn destruct_004a1750(this: u32) -> u32 {
        h::bridge::destruct(&mut mem(), this, const { h::of(h::DESTRUCT, 0x004a_1750) })
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderParameterDefinition__dtor`: the vtable, then `Phyre_PClassDescriptor_Destruct`; its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderParameterDefinition__dtor @ 0x004a_1760 => extern "thiscall" fn destruct_004a1760(this: u32) -> u32 {
        h::bridge::destruct(&mut mem(), this, const { h::of(h::DESTRUCT, 0x004a_1760) })
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PSharray_unsigned_int__dtor`: the vtable, then `Phyre_PClassDescriptor_Destruct`; its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PSharray_unsigned_int__dtor @ 0x004c_f430 => extern "thiscall" fn destruct_004cf430(this: u32) -> u32 {
        h::bridge::destruct(&mut mem(), this, const { h::of(h::DESTRUCT, 0x004c_f430) })
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PRendering_PNodeContext__dtor`: the vtable, then `Phyre_PClassDescriptor_Destruct`; its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PNodeContext__dtor @ 0x004c_f440 => extern "thiscall" fn destruct_004cf440(this: u32) -> u32 {
        h::bridge::destruct(&mut mem(), this, const { h::of(h::DESTRUCT, 0x004c_f440) })
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_float_4__dtor`: the vtable, then `Phyre_PClassDescriptor_Destruct`; its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PArray_float_4__dtor @ 0x0050_98c0 => extern "thiscall" fn destruct_005098c0(this: u32) -> u32 {
        h::bridge::destruct(&mut mem(), this, const { h::of(h::DESTRUCT, 0x0050_98c0) })
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationChannelTimes__dtor`: the vtable, then `Phyre_PClassDescriptor_Destruct`; its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PAnimation_PAnimationChannelTimes__dtor @ 0x0050_98d0 => extern "thiscall" fn destruct_005098d0(this: u32) -> u32 {
        h::bridge::destruct(&mut mem(), this, const { h::of(h::DESTRUCT, 0x0050_98d0) })
    }
    /// `Phyre_PClassLayoutInterface_scalarDeletingDtor_00479f10`: scalar deleting destructor; `this`.
    Phyre_PClassLayoutInterface_scalarDeletingDtor_00479f10 @ 0x0047_9f10 => extern "thiscall" fn delete_00479f10(this: u32, flags: u32) -> u32 {
        h::bridge::delete(&mut mem(), &const { h::of(h::DELETE, 0x0047_9f10) }, this, flags)
    }
    /// `Phyre_PSerialization_PBinary_PInternal_PFindNestedClassVisitor_scalarDeletingDtor_00479f40`: scalar deleting destructor; `this`.
    Phyre_PSerialization_PBinary_PInternal_PFindNestedClassVisitor_scalarDeletingDtor_00479f40 @ 0x0047_9f40 => extern "thiscall" fn delete_00479f40(this: u32, flags: u32) -> u32 {
        h::bridge::delete(&mut mem(), &const { h::of(h::DELETE, 0x0047_9f40) }, this, flags)
    }
    /// `Phyre_PIChildClassVisitor_scalarDeletingDtor_00479f70`: scalar deleting destructor; `this`.
    Phyre_PIChildClassVisitor_scalarDeletingDtor_00479f70 @ 0x0047_9f70 => extern "thiscall" fn delete_00479f70(this: u32, flags: u32) -> u32 {
        h::bridge::delete(&mut mem(), &const { h::of(h::DELETE, 0x0047_9f70) }, this, flags)
    }
    /// `Phyre_PClassDataMemberArray_scalarDeletingDtor_00575f80`: scalar deleting destructor; `this`.
    Phyre_PClassDataMemberArray_scalarDeletingDtor_00575f80 @ 0x0057_5f80 => extern "thiscall" fn delete_00575f80(this: u32, flags: u32) -> u32 {
        h::bridge::delete(&mut mem(), &const { h::of(h::DELETE, 0x0057_5f80) }, this, flags)
    }
    /// `Phyre_PClassDataMemberArrayFixedSize_scalarDeletingDtor_00575fb0`: scalar deleting destructor; `this`.
    Phyre_PClassDataMemberArrayFixedSize_scalarDeletingDtor_00575fb0 @ 0x0057_5fb0 => extern "thiscall" fn delete_00575fb0(this: u32, flags: u32) -> u32 {
        h::bridge::delete(&mut mem(), &const { h::of(h::DELETE, 0x0057_5fb0) }, this, flags)
    }
    /// `Phyre_PLOD_PLODMetricType_scalarDeletingDtor_005cbe10`: scalar deleting destructor; `this`.
    Phyre_PLOD_PLODMetricType_scalarDeletingDtor_005cbe10 @ 0x005c_be10 => extern "thiscall" fn delete_005cbe10(this: u32, flags: u32) -> u32 {
        h::bridge::delete(&mut mem(), &const { h::of(h::DELETE, 0x005c_be10) }, this, flags)
    }
    /// `Phyre_PLOD_PLODBlendType_scalarDeletingDtor_005cc270`: scalar deleting destructor; `this`.
    Phyre_PLOD_PLODBlendType_scalarDeletingDtor_005cc270 @ 0x005c_c270 => extern "thiscall" fn delete_005cc270(this: u32, flags: u32) -> u32 {
        h::bridge::delete(&mut mem(), &const { h::of(h::DELETE, 0x005c_c270) }, this, flags)
    }
    /// `Phyre_PInputs_PInputTypeSemantic_scalarDeletingDtor_009f48c0`: scalar deleting destructor; `this`.
    Phyre_PInputs_PInputTypeSemantic_scalarDeletingDtor_009f48c0 @ 0x009f_48c0 => extern "thiscall" fn delete_009f48c0(this: u32, flags: u32) -> u32 {
        h::bridge::delete(&mut mem(), &const { h::of(h::DELETE, 0x009f_48c0) }, this, flags)
    }
    /// `Phyre_PInputs_PInputAxisSemantic_scalarDeletingDtor_00a2de70`: scalar deleting destructor; `this`.
    Phyre_PInputs_PInputAxisSemantic_scalarDeletingDtor_00a2de70 @ 0x00a2_de70 => extern "thiscall" fn delete_00a2de70(this: u32, flags: u32) -> u32 {
        h::bridge::delete(&mut mem(), &const { h::of(h::DELETE, 0x00a2_de70) }, this, flags)
    }
    /// `Phyre_PInputs_PInputJoypadButtonSemantic_scalarDeletingDtor_00a2e2e0`: scalar deleting destructor; `this`.
    Phyre_PInputs_PInputJoypadButtonSemantic_scalarDeletingDtor_00a2e2e0 @ 0x00a2_e2e0 => extern "thiscall" fn delete_00a2e2e0(this: u32, flags: u32) -> u32 {
        h::bridge::delete(&mut mem(), &const { h::of(h::DELETE, 0x00a2_e2e0) }, this, flags)
    }
    /// `Phyre_PInputs_PInputKeySemantic_scalarDeletingDtor_00a2e750`: scalar deleting destructor; `this`.
    Phyre_PInputs_PInputKeySemantic_scalarDeletingDtor_00a2e750 @ 0x00a2_e750 => extern "thiscall" fn delete_00a2e750(this: u32, flags: u32) -> u32 {
        h::bridge::delete(&mut mem(), &const { h::of(h::DELETE, 0x00a2_e750) }, this, flags)
    }
    /// `Phyre_PInputs_PInputMouseButtonSemantic_scalarDeletingDtor_00a2ebc0`: scalar deleting destructor; `this`.
    Phyre_PInputs_PInputMouseButtonSemantic_scalarDeletingDtor_00a2ebc0 @ 0x00a2_ebc0 => extern "thiscall" fn delete_00a2ebc0(this: u32, flags: u32) -> u32 {
        h::bridge::delete(&mut mem(), &const { h::of(h::DELETE, 0x00a2_ebc0) }, this, flags)
    }
    /// `Phyre_PInputs_PInputChannelSemantic_scalarDeletingDtor_00a32530`: scalar deleting destructor; `this`.
    Phyre_PInputs_PInputChannelSemantic_scalarDeletingDtor_00a32530 @ 0x00a3_2530 => extern "thiscall" fn delete_00a32530(this: u32, flags: u32) -> u32 {
        h::bridge::delete(&mut mem(), &const { h::of(h::DELETE, 0x00a3_2530) }, this, flags)
    }
    /// `Phyre_PRendering_PLightType_unregister_0049e920`: unlinks the type from its list; 0.
    Phyre_PRendering_PLightType_unregister_0049e920 @ 0x0049_e920 => extern "C" fn unlink_0049e920(ty: u32) -> u32 {
        h::bridge::unlink(&mut mem(), const { h::of(h::UNLINK, 0x0049_e920) }, ty)
    }
    /// `Phyre_PLOD_PLODMetricType_unregister_005cbf40`: unlinks the type from its list; 0.
    Phyre_PLOD_PLODMetricType_unregister_005cbf40 @ 0x005c_bf40 => extern "C" fn unlink_005cbf40(ty: u32) -> u32 {
        h::bridge::unlink(&mut mem(), const { h::of(h::UNLINK, 0x005c_bf40) }, ty)
    }
    /// `Phyre_PLOD_PLODBlendType_unregister_005cc3a0`: unlinks the type from its list; 0.
    Phyre_PLOD_PLODBlendType_unregister_005cc3a0 @ 0x005c_c3a0 => extern "C" fn unlink_005cc3a0(ty: u32) -> u32 {
        h::bridge::unlink(&mut mem(), const { h::of(h::UNLINK, 0x005c_c3a0) }, ty)
    }
    /// `Phyre_PInputs_PInputTypeSemantic_unregister_009f49f0`: unlinks the type from its list; 0.
    Phyre_PInputs_PInputTypeSemantic_unregister_009f49f0 @ 0x009f_49f0 => extern "C" fn unlink_009f49f0(ty: u32) -> u32 {
        h::bridge::unlink(&mut mem(), const { h::of(h::UNLINK, 0x009f_49f0) }, ty)
    }
    /// `Phyre_PInputs_PInputAxisSemantic_unregister_00a2dfa0`: unlinks the type from its list; 0.
    Phyre_PInputs_PInputAxisSemantic_unregister_00a2dfa0 @ 0x00a2_dfa0 => extern "C" fn unlink_00a2dfa0(ty: u32) -> u32 {
        h::bridge::unlink(&mut mem(), const { h::of(h::UNLINK, 0x00a2_dfa0) }, ty)
    }
    /// `Phyre_PInputs_PInputJoypadButtonSemantic_unregister_00a2e410`: unlinks the type from its list; 0.
    Phyre_PInputs_PInputJoypadButtonSemantic_unregister_00a2e410 @ 0x00a2_e410 => extern "C" fn unlink_00a2e410(ty: u32) -> u32 {
        h::bridge::unlink(&mut mem(), const { h::of(h::UNLINK, 0x00a2_e410) }, ty)
    }
    /// `Phyre_PInputs_PInputKeySemantic_unregister_00a2e880`: unlinks the type from its list; 0.
    Phyre_PInputs_PInputKeySemantic_unregister_00a2e880 @ 0x00a2_e880 => extern "C" fn unlink_00a2e880(ty: u32) -> u32 {
        h::bridge::unlink(&mut mem(), const { h::of(h::UNLINK, 0x00a2_e880) }, ty)
    }
    /// `Phyre_PInputs_PInputMouseButtonSemantic_unregister_00a2ecf0`: unlinks the type from its list; 0.
    Phyre_PInputs_PInputMouseButtonSemantic_unregister_00a2ecf0 @ 0x00a2_ecf0 => extern "C" fn unlink_00a2ecf0(ty: u32) -> u32 {
        h::bridge::unlink(&mut mem(), const { h::of(h::UNLINK, 0x00a2_ecf0) }, ty)
    }
    /// `Phyre_PInputs_PInputChannelSemantic_unregister_00a32660`: unlinks the type from its list; 0.
    Phyre_PInputs_PInputChannelSemantic_unregister_00a32660 @ 0x00a3_2660 => extern "C" fn unlink_00a32660(ty: u32) -> u32 {
        h::bridge::unlink(&mut mem(), const { h::of(h::UNLINK, 0x00a3_2660) }, ty)
    }
    /// `Phyre_PGeometry_PUtilityGeometry_typeName_0047fd20`: the name and its size; 0.
    Phyre_PGeometry_PUtilityGeometry_typeName_0047fd20 @ 0x0047_fd20 => extern "C" fn name_0047fd20(object: u32, name: u32, size: u32) -> u32 {
        h::bridge::name(&mut mem(), const { h::of(h::NAME, 0x0047_fd20) }, object, name, size)
    }
    /// `Phyre_PRendering_PUtilityRendering_typeName_004c0f80`: the name and its size; 0.
    Phyre_PRendering_PUtilityRendering_typeName_004c0f80 @ 0x004c_0f80 => extern "C" fn name_004c0f80(object: u32, name: u32, size: u32) -> u32 {
        h::bridge::name(&mut mem(), const { h::of(h::NAME, 0x004c_0f80) }, object, name, size)
    }
    /// `Phyre_PAnimation_PUtilityAnimation_typeName_00507aa0`: the name and its size; 0.
    Phyre_PAnimation_PUtilityAnimation_typeName_00507aa0 @ 0x0050_7aa0 => extern "C" fn name_00507aa0(object: u32, name: u32, size: u32) -> u32 {
        h::bridge::name(&mut mem(), const { h::of(h::NAME, 0x0050_7aa0) }, object, name, size)
    }
    /// `Phyre_PLOD_PUtilityLOD_typeName_005cbeb0`: the name and its size; 0.
    Phyre_PLOD_PUtilityLOD_typeName_005cbeb0 @ 0x005c_beb0 => extern "C" fn name_005cbeb0(object: u32, name: u32, size: u32) -> u32 {
        h::bridge::name(&mut mem(), const { h::of(h::NAME, 0x005c_beb0) }, object, name, size)
    }
    /// `Phyre_PLOD_PUtilityLOD_typeName_005cc310`: the name and its size; 0.
    Phyre_PLOD_PUtilityLOD_typeName_005cc310 @ 0x005c_c310 => extern "C" fn name_005cc310(object: u32, name: u32, size: u32) -> u32 {
        h::bridge::name(&mut mem(), const { h::of(h::NAME, 0x005c_c310) }, object, name, size)
    }
    /// `Phyre_PInputs_PUtilityInputs_typeName_009f4960`: the name and its size; 0.
    Phyre_PInputs_PUtilityInputs_typeName_009f4960 @ 0x009f_4960 => extern "C" fn name_009f4960(object: u32, name: u32, size: u32) -> u32 {
        h::bridge::name(&mut mem(), const { h::of(h::NAME, 0x009f_4960) }, object, name, size)
    }
    /// `Phyre_PInputs_PUtilityInputs_typeName_00a2df10`: the name and its size; 0.
    Phyre_PInputs_PUtilityInputs_typeName_00a2df10 @ 0x00a2_df10 => extern "C" fn name_00a2df10(object: u32, name: u32, size: u32) -> u32 {
        h::bridge::name(&mut mem(), const { h::of(h::NAME, 0x00a2_df10) }, object, name, size)
    }
    /// `Phyre_PInputs_PUtilityInputs_typeName_00a2e380`: the name and its size; 0.
    Phyre_PInputs_PUtilityInputs_typeName_00a2e380 @ 0x00a2_e380 => extern "C" fn name_00a2e380(object: u32, name: u32, size: u32) -> u32 {
        h::bridge::name(&mut mem(), const { h::of(h::NAME, 0x00a2_e380) }, object, name, size)
    }
    /// `Phyre_PInputs_PUtilityInputs_typeName_00a2e7f0`: the name and its size; 0.
    Phyre_PInputs_PUtilityInputs_typeName_00a2e7f0 @ 0x00a2_e7f0 => extern "C" fn name_00a2e7f0(object: u32, name: u32, size: u32) -> u32 {
        h::bridge::name(&mut mem(), const { h::of(h::NAME, 0x00a2_e7f0) }, object, name, size)
    }
    /// `Phyre_PInputs_PUtilityInputs_typeName_00a2ec60`: the name and its size; 0.
    Phyre_PInputs_PUtilityInputs_typeName_00a2ec60 @ 0x00a2_ec60 => extern "C" fn name_00a2ec60(object: u32, name: u32, size: u32) -> u32 {
        h::bridge::name(&mut mem(), const { h::of(h::NAME, 0x00a2_ec60) }, object, name, size)
    }
    /// `Phyre_PInputs_PUtilityInputs_typeName_00a325d0`: the name and its size; 0.
    Phyre_PInputs_PUtilityInputs_typeName_00a325d0 @ 0x00a3_25d0 => extern "C" fn name_00a325d0(object: u32, name: u32, size: u32) -> u32 {
        h::bridge::name(&mut mem(), const { h::of(h::NAME, 0x00a3_25d0) }, object, name, size)
    }
    /// `PTypeEnum_create_005cbf20`: `*out = find(name)`; 0 or 0x13.
    PTypeEnum_create_005cbf20 @ 0x005c_bf20 => extern "C" fn create_005cbf20(out: u32, name: u32) -> u32 {
        ffx_prender::basic::bridge::create_result(&mut mem(), const { h::of(h::CREATE, 0x005c_bf20) }, out, name)
    }
    /// `PTypeEnum_create_005cc380`: `*out = find(name)`; 0 or 0x13.
    PTypeEnum_create_005cc380 @ 0x005c_c380 => extern "C" fn create_005cc380(out: u32, name: u32) -> u32 {
        ffx_prender::basic::bridge::create_result(&mut mem(), const { h::of(h::CREATE, 0x005c_c380) }, out, name)
    }
    /// `PTypeEnum_create_009f49d0`: `*out = find(name)`; 0 or 0x13.
    PTypeEnum_create_009f49d0 @ 0x009f_49d0 => extern "C" fn create_009f49d0(out: u32, name: u32) -> u32 {
        ffx_prender::basic::bridge::create_result(&mut mem(), const { h::of(h::CREATE, 0x009f_49d0) }, out, name)
    }
    /// `PTypeEnum_create_00a2df80`: `*out = find(name)`; 0 or 0x13.
    PTypeEnum_create_00a2df80 @ 0x00a2_df80 => extern "C" fn create_00a2df80(out: u32, name: u32) -> u32 {
        ffx_prender::basic::bridge::create_result(&mut mem(), const { h::of(h::CREATE, 0x00a2_df80) }, out, name)
    }
    /// `PTypeEnum_create_00a2e3f0`: `*out = find(name)`; 0 or 0x13.
    PTypeEnum_create_00a2e3f0 @ 0x00a2_e3f0 => extern "C" fn create_00a2e3f0(out: u32, name: u32) -> u32 {
        ffx_prender::basic::bridge::create_result(&mut mem(), const { h::of(h::CREATE, 0x00a2_e3f0) }, out, name)
    }
    /// `PTypeEnum_create_00a2e860`: `*out = find(name)`; 0 or 0x13.
    PTypeEnum_create_00a2e860 @ 0x00a2_e860 => extern "C" fn create_00a2e860(out: u32, name: u32) -> u32 {
        ffx_prender::basic::bridge::create_result(&mut mem(), const { h::of(h::CREATE, 0x00a2_e860) }, out, name)
    }
    /// `PTypeEnum_create_00a2ecd0`: `*out = find(name)`; 0 or 0x13.
    PTypeEnum_create_00a2ecd0 @ 0x00a2_ecd0 => extern "C" fn create_00a2ecd0(out: u32, name: u32) -> u32 {
        ffx_prender::basic::bridge::create_result(&mut mem(), const { h::of(h::CREATE, 0x00a2_ecd0) }, out, name)
    }
    /// `PTypeEnum_create_00a32640`: `*out = find(name)`; 0 or 0x13.
    PTypeEnum_create_00a32640 @ 0x00a3_2640 => extern "C" fn create_00a32640(out: u32, name: u32) -> u32 {
        ffx_prender::basic::bridge::create_result(&mut mem(), const { h::of(h::CREATE, 0x00a3_2640) }, out, name)
    }
    /// `PArray_Reset_00478d60`: owned array reset.
    PArray_Reset_00478d60 @ 0x0047_8d60 => extern "thiscall" fn reset_00478d60(this: u32) {
        ffx_prender::m5r_g::bridge::owned_reset(&mut mem(), this, false);
    }
    /// `PArray_Reset_00478d90`: owned array reset.
    PArray_Reset_00478d90 @ 0x0047_8d90 => extern "thiscall" fn reset_00478d90(this: u32) {
        ffx_prender::m5r_g::bridge::owned_reset(&mut mem(), this, true);
    }
    /// `PArray_Reset_00478dc0`: owned array reset.
    PArray_Reset_00478dc0 @ 0x0047_8dc0 => extern "thiscall" fn reset_00478dc0(this: u32) {
        ffx_prender::m5r_g::bridge::owned_reset(&mut mem(), this, true);
    }
    /// `PArray_Reset_00478e50`: owned array reset.
    PArray_Reset_00478e50 @ 0x0047_8e50 => extern "thiscall" fn reset_00478e50(this: u32) {
        ffx_prender::m5r_g::bridge::owned_reset(&mut mem(), this, true);
    }
    /// `PArray_Reset_00509830`: owned array reset.
    PArray_Reset_00509830 @ 0x0050_9830 => extern "thiscall" fn reset_00509830(this: u32) {
        ffx_prender::m5r_g::bridge::owned_reset(&mut mem(), this, true);
    }
    /// `PArray_Reset_00577950`: owned array reset.
    PArray_Reset_00577950 @ 0x0057_7950 => extern "thiscall" fn reset_00577950(this: u32) {
        ffx_prender::m5r_g::bridge::owned_reset(&mut mem(), this, true);
    }
    /// `PArray_Reset_00577980`: owned array reset.
    PArray_Reset_00577980 @ 0x0057_7980 => extern "thiscall" fn reset_00577980(this: u32) {
        ffx_prender::m5r_g::bridge::owned_reset(&mut mem(), this, true);
    }
    /// `PArray_Reset_005779b0`: owned array reset.
    PArray_Reset_005779b0 @ 0x0057_79b0 => extern "thiscall" fn reset_005779b0(this: u32) {
        ffx_prender::m5r_g::bridge::owned_reset(&mut mem(), this, true);
    }
    /// `PArray_Reset_00577a40`: owned array reset.
    PArray_Reset_00577a40 @ 0x0057_7a40 => extern "thiscall" fn reset_00577a40(this: u32) {
        ffx_prender::m5r_g::bridge::owned_reset(&mut mem(), this, true);
    }
    /// `PArray_Reset_00577a70`: owned array reset.
    PArray_Reset_00577a70 @ 0x0057_7a70 => extern "thiscall" fn reset_00577a70(this: u32) {
        ffx_prender::m5r_g::bridge::owned_reset(&mut mem(), this, true);
    }
    /// `PArray_Reset_00577aa0`: owned array reset.
    PArray_Reset_00577aa0 @ 0x0057_7aa0 => extern "thiscall" fn reset_00577aa0(this: u32) {
        ffx_prender::m5r_g::bridge::owned_reset(&mut mem(), this, true);
    }
    /// `PArray_Reset_00577ad0`: owned array reset.
    PArray_Reset_00577ad0 @ 0x0057_7ad0 => extern "thiscall" fn reset_00577ad0(this: u32) {
        ffx_prender::m5r_g::bridge::owned_reset(&mut mem(), this, true);
    }
    /// `PArray_Reset_009fd4b0`: owned array reset.
    PArray_Reset_009fd4b0 @ 0x009f_d4b0 => extern "thiscall" fn reset_009fd4b0(this: u32) {
        ffx_prender::m5r_g::bridge::owned_reset(&mut mem(), this, true);
    }
    /// `PSharray_Reset_004a8300`: shared array reset.
    PSharray_Reset_004a8300 @ 0x004a_8300 => extern "thiscall" fn release_004a8300(this: u32) {
        h::bridge::release(&mut mem(), const { h::of(h::RELEASE, 0x004a_8300) }, this);
    }
    /// `PSharray_Reset_004ea0d0`: shared array reset.
    PSharray_Reset_004ea0d0 @ 0x004e_a0d0 => extern "thiscall" fn release_004ea0d0(this: u32) {
        h::bridge::release(&mut mem(), const { h::of(h::RELEASE, 0x004e_a0d0) }, this);
    }
    /// `ReturnConst_00576110`: 0x17.
    ReturnConst_00576110 @ 0x0057_6110 => extern "thiscall" fn konst_00576110(_this: u32, _a0: u32, _a1: u32) -> u32 {
        const { h::of(h::CONSTANT, 0x0057_6110) }
    }
    /// `ReturnConst_00576120`: 0x17.
    ReturnConst_00576120 @ 0x0057_6120 => extern "thiscall" fn konst_00576120(_this: u32, _a0: u32, _a1: u32, _a2: u32) -> u32 {
        const { h::of(h::CONSTANT, 0x0057_6120) }
    }
    /// `ReturnConst_00675a40`: 0x18.
    ReturnConst_00675a40 @ 0x0067_5a40 => extern "thiscall" fn konst_00675a40(_this: u32, _a0: u32) -> u32 {
        const { h::of(h::CONSTANT, 0x0067_5a40) }
    }
    /// `Phyre_PIChildClassVisitor_ctorVtable_00479910`: `[this] = Phyre_PIChildClassVisitor_vftable`.
    Phyre_PIChildClassVisitor_ctorVtable_00479910 @ 0x0047_9910 => extern "thiscall" fn set_vtable_00479910(this: u32) {
        h::bridge::set_vtable(&mut mem(), this, 0x00b1_3484);
    }
    /// `PhyreFreeNonNull_0047a650(p)`: `PhyreFree(p)` unless 0.
    PhyreFreeNonNull_0047a650 @ 0x0047_a650 => extern "C" fn free_some_0047a650(p: u32) -> u32 {
        h::bridge::free_some(&mut mem(), p)
    }
    /// `PhyreFreeUntagged_004a1770(this)`: frees `[this+4]` unless tagged.
    PhyreFreeUntagged_004a1770 @ 0x004a_1770 => extern "thiscall" fn free_untagged_004a1770(this: u32) {
        h::bridge::free_untagged(&mut mem(), this);
    }
    /// `PhyreFreeOwned_00575460(this)`: frees `[this+0xc]` when owned.
    PhyreFreeOwned_00575460 @ 0x0057_5460 => extern "thiscall" fn free_owned_00575460(this: u32) {
        h::bridge::free_owned(&mut mem(), this);
    }
    /// `Phyre_PNamedSemanticDescriptor_dtor_005a0270(this)`: unlinks the node, the `PType` vtable; the old `prev`.
    Phyre_PNamedSemanticDescriptor_dtor_005a0270 @ 0x005a_0270 => extern "thiscall" fn named_semantic_dtor_005a0270(this: u32) -> u32 {
        h::bridge::named_semantic_dtor(&mut mem(), this, [0x00b3_44b0, 0x00b0_e568])
    }
    /// `Phyre_PIClassDataMemberDynamicType_scalarDeletingDtor_00575630(this, flags)`: `this`.
    Phyre_PIClassDataMemberDynamicType_scalarDeletingDtor_00575630 @ 0x0057_5630 => extern "thiscall" fn dynamic_type_delete_00575630(this: u32, flags: u32) -> u32 {
        h::bridge::dynamic_type_delete(&mut mem(), this, flags, 0x00b3_09cc)
    }
    /// `Phyre_PClassDataMemberArrayFixedSize_memberAddress_00576000(this, object)`.
    Phyre_PClassDataMemberArrayFixedSize_memberAddress_00576000 @ 0x0057_6000 => extern "thiscall" fn member_address_00576000(this: u32, object: u32) -> u32 {
        h::bridge::member_address(&mut mem(), this, object)
    }
    /// `Phyre_PClassDataMemberArrayFixedSize_memberAddress_00576020(this, object)`.
    Phyre_PClassDataMemberArrayFixedSize_memberAddress_00576020 @ 0x0057_6020 => extern "thiscall" fn member_address_00576020(this: u32, object: u32) -> u32 {
        h::bridge::member_address(&mut mem(), this, object)
    }
    /// `Phyre_PClassDataMemberArrayFixedSize_getCount_00576040(this, object)`: `[this+0x34]`.
    Phyre_PClassDataMemberArrayFixedSize_getCount_00576040 @ 0x0057_6040 => extern "thiscall" fn get_count_00576040(this: u32, _object: u32) -> u32 {
        h::bridge::word_at(&mut mem(), this, 0x34)
    }
    /// `Phyre_PClassDataMember_typeVirtual_005756b0(this, a, object)`.
    Phyre_PClassDataMember_typeVirtual_005756b0 @ 0x0057_56b0 => extern "thiscall" fn member_virtual_005756b0(this: u32, a: u32, object: u32) -> u32 {
        h::bridge::member_virtual(&mut mem(), [3, 5], this, a, object)
    }
    /// `Phyre_PClassDataMember_typeVirtual_005756f0(this, a, object)`.
    Phyre_PClassDataMember_typeVirtual_005756f0 @ 0x0057_56f0 => extern "thiscall" fn member_virtual_005756f0(this: u32, a: u32, object: u32) -> u32 {
        h::bridge::member_virtual(&mut mem(), [4, 6], this, a, object)
    }
    /// `ClassDynamicMesh_poolInstance_00672b30(this, mesh, existing)`.
    ClassDynamicMesh_poolInstance_00672b30 @ 0x0067_2b30 => extern "thiscall" fn pool_instance_00672b30(this: u32, mesh: u32, existing: u32) -> u32 {
        h::bridge::pool_instance(&mut mem(), this, mesh, existing)
    }
    /// `Phyre_PClassDescriptorConcrete_PNodeContext_allocCopy_004d0470(this, n, src)`.
    Phyre_PClassDescriptorConcrete_PNodeContext_allocCopy_004d0470 @ 0x004d_0470 => extern "thiscall" fn alloc_copy_004d0470(this: u32, n: u32, src: u32) -> u32 {
        h::bridge::alloc_copy(&mut mem(), this, n, src, n)
    }
    /// `Phyre_PClassDataMemberPhyreSharray_PUInt32_allocCopy_004d04d0(this, n, src, count)`.
    Phyre_PClassDataMemberPhyreSharray_PUInt32_allocCopy_004d04d0 @ 0x004d_04d0 => extern "thiscall" fn alloc_copy_004d04d0(this: u32, n: u32, src: u32, count: u32) -> u32 {
        h::bridge::alloc_copy(&mut mem(), this, n, src, count)
    }
    /// `Phyre_PClassDescriptorBase_scalarDeletingDtor_005706a0(this, flags)`: the `0x004355e0` shape.
    Phyre_PClassDescriptorBase_scalarDeletingDtor_005706a0 @ 0x0057_06a0 => extern "thiscall" fn delete_005706a0(this: u32, flags: u32) -> u32 {
        ft::bridge::delete(&mut mem(), this, flags, const { ft::word_of(ft::DELETE, 0x0043_55e0) })
    }
    /// `ClassDynamicMesh_virtualStore_00672f20(this, a0..a3, out)`: the `0x00674650` shape.
    ClassDynamicMesh_virtualStore_00672f20 @ 0x0067_2f20 => extern "thiscall" fn virtual_store_00672f20(this: u32, a0: u32, a1: u32, a2: u32, a3: u32, out: u32) -> u32 {
        ffx_prender::m5r_f::bridge::virtual_store(&mut mem(), this, [a0, a1, a2, a3], out)
    }
    /// `RadialLineDynamicMesh_virtualStore_00676590(this, a0..a3, out)`: the `0x00674650` shape.
    RadialLineDynamicMesh_virtualStore_00676590 @ 0x0067_6590 => extern "thiscall" fn virtual_store_00676590(this: u32, a0: u32, a1: u32, a2: u32, a3: u32, out: u32) -> u32 {
        ffx_prender::m5r_f::bridge::virtual_store(&mut mem(), this, [a0, a1, a2, a3], out)
    }
    /// `BrokenScreenPolygonDynamicMesh_virtualStore_00677bf0(this, a0..a3, out)`: the `0x00674650` shape.
    BrokenScreenPolygonDynamicMesh_virtualStore_00677bf0 @ 0x0067_7bf0 => extern "thiscall" fn virtual_store_00677bf0(this: u32, a0: u32, a1: u32, a2: u32, a3: u32, out: u32) -> u32 {
        ffx_prender::m5r_f::bridge::virtual_store(&mut mem(), this, [a0, a1, a2, a3], out)
    }
    /// `Phyre_PClassDataMember_scalarDeletingDtor_00575510`.
    Phyre_PClassDataMember_scalarDeletingDtor_00575510 @ 0x0057_5510 => extern "thiscall" fn f_00575510(this: u32, flags: u32) -> u32 {
        m::bridge::member_delete(&mut mem(), this, flags)
    }
    /// `Phyre_PClassDataMemberEnum_scalarDeletingDtor_005755a0`.
    Phyre_PClassDataMemberEnum_scalarDeletingDtor_005755a0 @ 0x0057_55a0 => extern "thiscall" fn f_005755a0(this: u32, flags: u32) -> u32 {
        m::bridge::member_delete(&mut mem(), this, flags)
    }
    /// `Phyre_PSharray_PUInt32_setToExternalBuffer_004d0b00`.
    Phyre_PSharray_PUInt32_setToExternalBuffer_004d0b00 @ 0x004d_0b00 => extern "thiscall" fn f_004d0b00(this: u32, data: u32, count: u32) -> u32 {
        m::bridge::sharray_set(&mut mem(), this, data, count)
    }
    /// `Phyre_PSharray_PUInt32_resize_004d0970`.
    Phyre_PSharray_PUInt32_resize_004d0970 @ 0x004d_0970 => extern "thiscall" fn f_004d0970(this: u32, n: u32) -> u32 {
        m::bridge::sharray_resize(&mut mem(), this, n)
    }
    /// `Phyre_PClassDataMemberPhyreSharray_PUInt32_setBuffer_004d0ba0`.
    Phyre_PClassDataMemberPhyreSharray_PUInt32_setBuffer_004d0ba0 @ 0x004d_0ba0 => extern "thiscall" fn f_004d0ba0(this: u32, data: u32, n: u32) -> u32 {
        m::bridge::sharray_member_set(&mut mem(), this, data, n)
    }
    /// `Phyre_PInternal_PDeleteChildrenVisitor_visit_004d7510`.
    Phyre_PInternal_PDeleteChildrenVisitor_visit_004d7510 @ 0x004d_7510 => extern "thiscall" fn f_004d7510(_this: u32, objects: u32, count: u32, member: u32, flags: u32) -> u32 {
        m::bridge::delete_children(&mut mem(), objects, count, member, flags)
    }
    /// `Phyre_PSharray_ptr_indexOf_00518a60`.
    Phyre_PSharray_ptr_indexOf_00518a60 @ 0x0051_8a60 => extern "thiscall" fn f_00518a60(this: u32, value: u32, start: u32) -> u32 {
        m::bridge::index_of(&mut mem(), this, value, start)
    }
    /// `Phyre_PSharray_PString_indexOf_0054dcd0`.
    Phyre_PSharray_PString_indexOf_0054dcd0 @ 0x0054_dcd0 => extern "thiscall" fn f_0054dcd0(this: u32, name: u32, start: u32) -> u32 {
        m::bridge::name_index(&mut mem(), this, name, start)
    }
    /// `Phyre_PRedBlackTree_find_005cc560`.
    Phyre_PRedBlackTree_find_005cc560 @ 0x005c_c560 => extern "thiscall" fn f_005cc560(this: u32, key: u32) -> u32 {
        m::bridge::tree_find(&mut mem(), this, key)
    }
    /// `Phyre_PRedBlackTree_insertNode_005cc5e0`.
    Phyre_PRedBlackTree_insertNode_005cc5e0 @ 0x005c_c5e0 => extern "thiscall" fn f_005cc5e0(this: u32, node: u32) -> u32 {
        m::bridge::tree_insert(&mut mem(), this, node)
    }
    /// `Phyre_PAnimation_PAnimationSlotFilter_test_0052ae40`.
    Phyre_PAnimation_PAnimationSlotFilter_test_0052ae40 @ 0x0052_ae40 => extern "thiscall" fn f_0052ae40(this: u32, object: u32, x: u32) -> u32 {
        m::bridge::slot_filter(&mut mem(), this, object, x)
    }
    /// `Phyre_PAnimation_PAnimationSlotFilter_copyKeys_00529e60`.
    Phyre_PAnimation_PAnimationSlotFilter_copyKeys_00529e60 @ 0x0052_9e60 => extern "C" fn f_00529e60(dst: u32, src: u32, _a2: u32, indices: u32, _a4: u32, range: u32) -> u32 {
        m::bridge::slot_copy(&mut mem(), dst, src, indices, range)
    }
    /// `Phyre_PSerialization_swapWords_0047bf50`.
    Phyre_PSerialization_swapWords_0047bf50 @ 0x0047_bf50 => extern "C" fn f_0047bf50(n: u32, p: u32) {
        m::bridge::swap_words(&mut mem(), n, p);
    }
    /// `Phyre_PNamedSemanticDescriptor_scalarDeletingDtor_005a02a0`.
    Phyre_PNamedSemanticDescriptor_scalarDeletingDtor_005a02a0 @ 0x005a_02a0 => extern "thiscall" fn f_005a02a0(this: u32, flags: u32) -> u32 {
        m::bridge::named_semantic_delete(&mut mem(), this, flags)
    }
    /// `FFXApplication_axisXNeg_00a2f740`.
    FFXApplication_axisXNeg_00a2f740 @ 0x00a2_f740 => extern "C" fn f_00a2f740(xy: u32) -> f32 {
        m::bridge::axis(&mut mem(), xy, 0, true)
    }
    /// `FFXApplication_axisYNeg_00a2f760`.
    FFXApplication_axisYNeg_00a2f760 @ 0x00a2_f760 => extern "C" fn f_00a2f760(xy: u32) -> f32 {
        m::bridge::axis(&mut mem(), xy, 4, true)
    }
    /// `FFXApplication_axisX_00a2f780`.
    FFXApplication_axisX_00a2f780 @ 0x00a2_f780 => extern "C" fn f_00a2f780(xy: u32) -> f32 {
        m::bridge::axis(&mut mem(), xy, 0, false)
    }
    /// `FFXApplication_axisY_00a2f7a0`.
    FFXApplication_axisY_00a2f7a0 @ 0x00a2_f7a0 => extern "C" fn f_00a2f7a0(xy: u32) -> f32 {
        m::bridge::axis(&mut mem(), xy, 4, false)
    }
    /// `Phyre_PLOD_blend4_005cb940`.
    Phyre_PLOD_blend4_005cb940 @ 0x005c_b940 => extern "C" fn f_005cb940(weights: u32, n: u32, arrays: u32, out: u32) {
        m::bridge::blend4(&mut mem(), weights, n, arrays, out);
    }
    /// `Phyre_PPhysics_descriptorPair_005d7980`.
    Phyre_PPhysics_descriptorPair_005d7980 @ 0x005d_7980 => extern "C" fn f_005d7980(a: u32, b: u32) -> u32 {
        m::bridge::pair(&mut mem(), const { h::of(m::PAIR, 0x005d_7980) }, a, b)
    }
    /// `Phyre_PPhysics_descriptorPair_005d79c0`.
    Phyre_PPhysics_descriptorPair_005d79c0 @ 0x005d_79c0 => extern "C" fn f_005d79c0(a: u32, b: u32) -> u32 {
        m::bridge::pair(&mut mem(), const { h::of(m::PAIR, 0x005d_79c0) }, a, b)
    }
    /// `Phyre_PPhysics_descriptorPair_005d7a40`.
    Phyre_PPhysics_descriptorPair_005d7a40 @ 0x005d_7a40 => extern "C" fn f_005d7a40(a: u32, b: u32) -> u32 {
        m::bridge::pair(&mut mem(), const { h::of(m::PAIR, 0x005d_7a40) }, a, b)
    }
    /// `Phyre_PPhysics_copyConstruct_005d78b0`.
    Phyre_PPhysics_copyConstruct_005d78b0 @ 0x005d_78b0 => extern "C" fn f_005d78b0(dst: u32, src: u32, n: u32) {
        m::bridge::copy_construct(&mut mem(), const { h::of(m::COPY, 0x005d_78b0) }, dst, src, n);
    }
    /// `Phyre_PPhysics_copyConstruct_005d78e0`.
    Phyre_PPhysics_copyConstruct_005d78e0 @ 0x005d_78e0 => extern "C" fn f_005d78e0(dst: u32, src: u32, n: u32) {
        m::bridge::copy_construct(&mut mem(), const { h::of(m::COPY, 0x005d_78e0) }, dst, src, n);
    }
    /// `Phyre_PPhysics_copyConstruct_005d7910`.
    Phyre_PPhysics_copyConstruct_005d7910 @ 0x005d_7910 => extern "C" fn f_005d7910(dst: u32, src: u32, n: u32) {
        m::bridge::copy_construct(&mut mem(), const { h::of(m::COPY, 0x005d_7910) }, dst, src, n);
    }
    /// `PRedBlackTreePair_dtor_00479000`.
    PRedBlackTreePair_dtor_00479000 @ 0x0047_9000 => extern "thiscall" fn g_00479000(this: u32) -> u32 {
        ffx_prender::m5r_b::bridge::tree_pair_dtor(&mut mem(), this)
    }
    /// `PRedBlackTreePair_dtor_00577b00`.
    PRedBlackTreePair_dtor_00577b00 @ 0x0057_7b00 => extern "thiscall" fn g_00577b00(this: u32) -> u32 {
        ffx_prender::m5r_b::bridge::tree_pair_dtor(&mut mem(), this)
    }
    /// `PRedBlackTreePair_dtor_00577b60`.
    PRedBlackTreePair_dtor_00577b60 @ 0x0057_7b60 => extern "thiscall" fn g_00577b60(this: u32) -> u32 {
        ffx_prender::m5r_b::bridge::tree_pair_dtor(&mut mem(), this)
    }
    /// `PRedBlackTreePair_dtor_00577bc0`.
    PRedBlackTreePair_dtor_00577bc0 @ 0x0057_7bc0 => extern "thiscall" fn g_00577bc0(this: u32) -> u32 {
        ffx_prender::m5r_b::bridge::tree_pair_dtor(&mut mem(), this)
    }
    /// `PRedBlackTreeFree_dtor_004794e0`.
    PRedBlackTreeFree_dtor_004794e0 @ 0x0047_94e0 => extern "thiscall" fn g_004794e0(this: u32) -> u32 {
        ffx_prender::m5r_b::bridge::tree_free_dtor(&mut mem(), this)
    }
    /// `Phyre_PDynamicGeometry_PModifierNetworkDynamicMesh_newInstance_005adf70`.
    Phyre_PDynamicGeometry_PModifierNetworkDynamicMesh_newInstance_005adf70 @ 0x005a_df70 => extern "thiscall" fn g_005adf70(this: u32, cluster: u32, existing: u32; slots: u32) -> u32 {
        r::bridge::new_instance(&mut mem(), const { h::of(r::NEW_INSTANCE, 0x005a_df70) }, slots, this, cluster, existing)
    }
    /// `DistortionGridDynamicMesh_newInstance_006744c0`.
    DistortionGridDynamicMesh_newInstance_006744c0 @ 0x0067_44c0 => extern "thiscall" fn g_006744c0(this: u32, cluster: u32, existing: u32; slots: u32) -> u32 {
        r::bridge::new_instance(&mut mem(), const { h::of(r::NEW_INSTANCE, 0x0067_44c0) }, slots, this, cluster, existing)
    }
    /// `RadialLineDynamicMesh_newInstance_00676400`.
    RadialLineDynamicMesh_newInstance_00676400 @ 0x0067_6400 => extern "thiscall" fn g_00676400(this: u32, cluster: u32, existing: u32; slots: u32) -> u32 {
        r::bridge::new_instance(&mut mem(), const { h::of(r::NEW_INSTANCE, 0x0067_6400) }, slots, this, cluster, existing)
    }
    /// `BrokenScreenPolygonDynamicMesh_newInstance_00677a60`.
    BrokenScreenPolygonDynamicMesh_newInstance_00677a60 @ 0x0067_7a60 => extern "thiscall" fn g_00677a60(this: u32, cluster: u32, existing: u32; slots: u32) -> u32 {
        r::bridge::new_instance(&mut mem(), const { h::of(r::NEW_INSTANCE, 0x0067_7a60) }, slots, this, cluster, existing)
    }
    /// `Phyre_PSerialization_convert_copy_0047a510`.
    Phyre_PSerialization_convert_copy_0047a510 @ 0x0047_a510 => extern "C" fn g_0047a510(src: u32, dst: u32, counts: u32, entry: u32) {
        r::bridge::copy_kernel(&mut mem(), src, dst, counts, entry);
    }
    /// `Phyre_PGameplay_PScriptedComponent_scaleMatrix_00570c00`.
    Phyre_PGameplay_PScriptedComponent_scaleMatrix_00570c00 @ 0x0057_0c00 => extern "thiscall" fn g_00570c00(this: u32, out: u32, s: u32) -> u32 {
        r::bridge::scale_matrix(&mut mem(), this, out, s)
    }
    /// `Phyre_PScripting_testUserData_00570d50`.
    Phyre_PScripting_testUserData_00570d50 @ 0x0057_0d50 => extern "C" fn g_00570d50(l: u32, idx: u32) -> u32 {
        r::bridge::test_user_data(&mut mem(), l, idx)
    }
    /// `Phyre_PSharray_copyCtor_005a2380`.
    Phyre_PSharray_copyCtor_005a2380 @ 0x005a_2380 => extern "thiscall" fn g_005a2380(this: u32, src: u32) -> u32 {
        r::bridge::sharray_copy(&mut mem(), this, src)
    }
    /// `Phyre_PAnimation_slotMapCtor_005cc670`.
    Phyre_PAnimation_slotMapCtor_005cc670 @ 0x005c_c670 => extern "thiscall" fn g_005cc670(this: u32, src: u32) -> u32 {
        r::bridge::slot_map(&mut mem(), this, src)
    }
    /// `RadialLineDynamicMeshInstance_populatePreprocessData_00675a90`.
    RadialLineDynamicMeshInstance_populatePreprocessData_00675a90 @ 0x0067_5a90 => extern "thiscall" fn g_00675a90(this: u32, out: u32, _x: u32) -> u32 {
        r::bridge::radial_preprocess(&mut mem(), this, out)
    }
    /// `RadialLineDynamicMesh_copyBuffers_006764b0`.
    RadialLineDynamicMesh_copyBuffers_006764b0 @ 0x0067_64b0 => extern "stdcall" fn g_006764b0(_x: u32, src: u32, outs: u32, mesh: u32) -> u32 {
        ffx_pother_b::p4_cloth::bridge::copy_buffers(&mut mem(), src, outs, mesh)
    }
    /// `FFXApplication_releaseClusters_006dfdd0`.
    FFXApplication_releaseClusters_006dfdd0 @ 0x006d_fdd0 => extern "C" fn g_006dfdd0() {
        r::bridge::release_clusters(&mut mem());
    }
    /// `FFXApplication_closeMenu_006f72a0`.
    FFXApplication_closeMenu_006f72a0 @ 0x006f_72a0 => extern "C" fn g_006f72a0() -> u32 {
        r::bridge::close_menu(&mut mem())
    }
    /// `FFXApplication_closeMenus_006f7820`.
    FFXApplication_closeMenus_006f7820 @ 0x006f_7820 => extern "C" fn g_006f7820() {
        r::bridge::close_menus(&mut mem());
    }
    /// `Phyre_PBitmapFontCharInfo_constructArray_00a2d1a0`.
    Phyre_PBitmapFontCharInfo_constructArray_00a2d1a0 @ 0x00a2_d1a0 => extern "C" fn g_00a2d1a0(p: u32, n: u32) {
        r::bridge::font_chars(&mut mem(), p, n);
    }
    /// `SpdCtrl_UpdateSubtitleMute_006f7230`.
    SpdCtrl_UpdateSubtitleMute_006f7230 @ 0x006f_7230 => extern "C" fn g_006f7230() -> u32 {
        r::bridge::subtitle_mute(&mut mem())
    }
    /// `Phyre_PSerialization_swapNamespaceHeader_0047f880`.
    Phyre_PSerialization_swapNamespaceHeader_0047f880 @ 0x0047_f880 => extern "thiscall" fn s_0047f880(this: u32) {
        s::bridge::swap_words(&mut mem(), 8, this);
    }
    /// `Phyre_PSerialization_swapClassRecords_0047bc60`.
    Phyre_PSerialization_swapClassRecords_0047bc60 @ 0x0047_bc60 => extern "C" fn s_0047bc60(n: u32, p: u32) {
        s::bridge::swap_records(&mut mem(), 9, n, p);
    }
    /// `Phyre_PSerialization_swapMemberRecords_0047be20`.
    Phyre_PSerialization_swapMemberRecords_0047be20 @ 0x0047_be20 => extern "C" fn s_0047be20(n: u32, p: u32) {
        s::bridge::swap_records(&mut mem(), 6, n, p);
    }
    /// `Phyre_PSerialization_swapNamespaceBlock_0047d680`.
    Phyre_PSerialization_swapNamespaceBlock_0047d680 @ 0x0047_d680 => extern "thiscall" fn s_0047d680(this: u32) {
        s::bridge::swap_block(&mut mem(), this);
    }
    /// `Phyre_PSerialization_PMappingEntry_needsConversion_0047dc20`.
    Phyre_PSerialization_PMappingEntry_needsConversion_0047dc20 @ 0x0047_dc20 => extern "thiscall" fn s_0047dc20(this: u32) -> u32 {
        s::bridge::needs_conversion(&mut mem(), this)
    }
    /// `Phyre_PSerialization_readNamespaceBlock_00579010`.
    Phyre_PSerialization_readNamespaceBlock_00579010 @ 0x0057_9010 => extern "C" fn s_00579010(reader: u32, swap: u32, block: u32, size: u32) -> u32 {
        s::bridge::read_block(&mut mem(), reader, swap, block, size)
    }
    /// `Phyre_PRendering_bindConstantBuffer_00598d00`.
    Phyre_PRendering_bindConstantBuffer_00598d00 @ 0x0059_8d00 => extern "thiscall" fn s_00598d00(this: u32, array: u32, _slot: u32, index: u32; slots: u32) -> u32 {
        s::bridge::bind_buffer(&mut mem(), slots, this, array, index)
    }
    /// `FFXApplication_vectorMax_00a1b560`.
    FFXApplication_vectorMax_00a1b560 @ 0x00a1_b560 => extern "C" fn s_00a1b560(out: u32, a: u32, b: u32) -> u32 {
        s::bridge::min_max(&mut mem(), true, out, a, b)
    }
    /// `FFXApplication_vectorMin_00a1b620`.
    FFXApplication_vectorMin_00a1b620 @ 0x00a1_b620 => extern "C" fn s_00a1b620(out: u32, a: u32, b: u32) -> u32 {
        s::bridge::min_max(&mut mem(), false, out, a, b)
    }
    /// `Phyre_PSerialization_PNamespaceMapping_updateConversionFlags_0047c080`.
    Phyre_PSerialization_PNamespaceMapping_updateConversionFlags_0047c080 @ 0x0047_c080 => extern "thiscall" fn mp_0047c080(this: u32) {
        mp::bridge::update_flags(&mut mem(), this);
    }
    /// `Phyre_PSerialization_PMappingEntry_assignKernels_0047e180`.
    Phyre_PSerialization_PMappingEntry_assignKernels_0047e180 @ 0x0047_e180 => extern "thiscall" fn mp_0047e180(this: u32, to_runtime: u32) {
        mp::bridge::assign_kernels(&mut mem(), this, to_runtime);
    }
    /// `Phyre_PSerialization_PNamespaceMapping_convert_0047c610`.
    Phyre_PSerialization_PNamespaceMapping_convert_0047c610 @ 0x0047_c610 => extern "thiscall" fn mp_0047c610(this: u32, counts: u32, src: u32, dst: u32) -> u32 {
        mp::bridge::driver(&mut mem(), this, counts, src, dst)
    }
    /// `FFXApplication_boxOutcode_005a4180`.
    FFXApplication_boxOutcode_005a4180 @ 0x005a_4180 => extern "C" fn gm_005a4180(min: u32, max: u32, p: u32) -> u32 {
        gm::bridge::outcode(&mut mem(), min, max, p)
    }
    /// `Phyre_PAnimation_bonePalette_005c8830`.
    Phyre_PAnimation_bonePalette_005c8830 @ 0x005c_8830 => extern "C" fn gm_005c8830(out: u32, skin: u32) -> u32 {
        { let mut f = StackFrame::<0x10>::new(); gm::bridge::palette(&mut mem(), f.at(), out, skin) }
    }
    /// `Phyre_PAnimation_bonePaletteRelative_005c8700`.
    Phyre_PAnimation_bonePaletteRelative_005c8700 @ 0x005c_8700 => extern "C" fn gm_005c8700(out: u32, skin: u32) -> u32 {
        { let mut f = StackFrame::<0x32>::new(); gm::bridge::palette_relative(&mut mem(), f.at(), out, skin) }
    }
    /// `FFXApplication_clipPolygonPlane_00a2f5e0`.
    FFXApplication_clipPolygonPlane_00a2f5e0 @ 0x00a2_f5e0 => extern "C" fn gm_00a2f5e0(input: u32, n: u32, out: u32, count: u32, distance: u32) -> u32 {
        { let mut f = StackFrame::<4>::new(); gm::bridge::clip(&mut mem(), f.at(), input, n, out, count, distance) }
    }
    /// `FFXApplication_pointInBox_00a2fc90`.
    FFXApplication_pointInBox_00a2fc90 @ 0x00a2_fc90 => extern "C" fn ct_00a2fc90(p: u32, min: u32, max: u32) -> u32 {
        ct::bridge::inside(&mut mem(), p, min, max)
    }
    /// `Phyre_PSerialization_convertElements_005788f0`.
    Phyre_PSerialization_convertElements_005788f0 @ 0x0057_88f0 => extern "C" fn ct_005788f0(entry: u32, src: u32, dst: u32, n: u32) -> u32 {
        { let mut f = StackFrame::<4>::new(); ct::bridge::convert_elements(&mut mem(), f.at(), entry, src, dst, n) }
    }
    /// `Phyre_PPhysics_PPhysicsWorld_appendContact_005f36d0`.
    Phyre_PPhysics_PPhysicsWorld_appendContact_005f36d0 @ 0x005f_36d0 => extern "thiscall" fn ct_005f36d0(this: u32, a: u32, b: u32, index: u32, c: u32) -> u32 {
        ct::bridge::append(&mut mem(), this, a, b, index, c)
    }
    /// `Phyre_PPhysics_PPhysicsWorld_addContact_005f31e0`.
    Phyre_PPhysics_PPhysicsWorld_addContact_005f31e0 @ 0x005f_31e0 => extern "thiscall" fn ct_005f31e0(this: u32, a: u32, b: u32, c: u32) -> u32 {
        { let mut f = StackFrame::<1>::new(); ct::bridge::add_contact(&mut mem(), f.at(), this, a, b, c) }
    }
    /// `Phyre_PArray_resize36_00602900`.
    Phyre_PArray_resize36_00602900 @ 0x0060_2900 => extern "thiscall" fn ct_00602900(this: u32, n: u32) -> u32 {
        { let (_, w, copy, ctor) = const { ct::resize_of(0x0060_2900) }; ct::bridge::resize(&mut mem(), w, copy, ctor, this, n) }
    }
    /// `Phyre_PArray_resize40_006029a0`.
    Phyre_PArray_resize40_006029a0 @ 0x0060_29a0 => extern "thiscall" fn ct_006029a0(this: u32, n: u32) -> u32 {
        { let (_, w, copy, ctor) = const { ct::resize_of(0x0060_29a0) }; ct::bridge::resize(&mut mem(), w, copy, ctor, this, n) }
    }
    /// `Phyre_PArray_resize40_00602a60`.
    Phyre_PArray_resize40_00602a60 @ 0x0060_2a60 => extern "thiscall" fn ct_00602a60(this: u32, n: u32) -> u32 {
        { let (_, w, copy, ctor) = const { ct::resize_of(0x0060_2a60) }; ct::bridge::resize(&mut mem(), w, copy, ctor, this, n) }
    }
}
