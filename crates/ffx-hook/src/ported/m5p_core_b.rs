//! M5 stage 1, list `phyre-core`, batch 2 (`ffx_pcore::m5p_core_b`): the
//! containers: the `PArray<T>::resize` copies (one generic port, a table
//! of each copy's allocator, element helpers and release), the element
//! helpers themselves, `PSharray` members, `PStringBuilder`'s constructor
//! and `+= int`, the thread pool's destructor body.
//!
//! Threads: where objects are made, loaded and freed (main thread and the
//! loader). No shim keeps state; the locals callees see are per-call
//! `StackFrame`s.

use super::{StackFrame, mem, ported};
use ffx_pcore::m5p_core_b::{self as c, bridge as b};

ported! {
    /// `Phyre_PArray_resize_0044a080(this; n)` -> 0 or 0xd.
    Phyre_PArray_resize_0044a080 @ 0x0044_a080 => extern "thiscall" fn resize_0044a080(this: u32, n: u32) -> u32 {
        b::resize(&mut mem(), this, n, const { c::resize_of(0x0044_a080) })
    }
    /// `Phyre_PArray_resize_0047ef50(this; n)` -> 0 or 0xd.
    Phyre_PArray_resize_0047ef50 @ 0x0047_ef50 => extern "thiscall" fn resize_0047ef50(this: u32, n: u32) -> u32 {
        b::resize(&mut mem(), this, n, const { c::resize_of(0x0047_ef50) })
    }
    /// `Phyre_PArray_resize_00489740(this; n)` -> 0 or 0xd.
    Phyre_PArray_resize_00489740 @ 0x0048_9740 => extern "thiscall" fn resize_00489740(this: u32, n: u32) -> u32 {
        b::resize(&mut mem(), this, n, const { c::resize_of(0x0048_9740) })
    }
    /// `Phyre_PArray_resize_00499030(this; n)` -> 0 or 0xd.
    Phyre_PArray_resize_00499030 @ 0x0049_9030 => extern "thiscall" fn resize_00499030(this: u32, n: u32) -> u32 {
        b::resize(&mut mem(), this, n, const { c::resize_of(0x0049_9030) })
    }
    /// `Phyre_PArray_resize_004bdc40(this; n)` -> 0 or 0xd.
    Phyre_PArray_resize_004bdc40 @ 0x004b_dc40 => extern "thiscall" fn resize_004bdc40(this: u32, n: u32) -> u32 {
        b::resize(&mut mem(), this, n, const { c::resize_of(0x004b_dc40) })
    }
    /// `Phyre_PArray_resize_004f8980(this; n)` -> 0 or 0xd.
    Phyre_PArray_resize_004f8980 @ 0x004f_8980 => extern "thiscall" fn resize_004f8980(this: u32, n: u32) -> u32 {
        b::resize(&mut mem(), this, n, const { c::resize_of(0x004f_8980) })
    }
    /// `Phyre_PArray_resize_0050ae70(this; n)` -> 0 or 0xd.
    Phyre_PArray_resize_0050ae70 @ 0x0050_ae70 => extern "thiscall" fn resize_0050ae70(this: u32, n: u32) -> u32 {
        b::resize(&mut mem(), this, n, const { c::resize_of(0x0050_ae70) })
    }
    /// `Phyre_PArray_resize_0052a860(this; n)` -> 0 or 0xd.
    Phyre_PArray_resize_0052a860 @ 0x0052_a860 => extern "thiscall" fn resize_0052a860(this: u32, n: u32) -> u32 {
        b::resize(&mut mem(), this, n, const { c::resize_of(0x0052_a860) })
    }
    /// `Phyre_PArray_resize_005993a0(this; n)` -> 0 or 0xd.
    Phyre_PArray_resize_005993a0 @ 0x0059_93a0 => extern "thiscall" fn resize_005993a0(this: u32, n: u32) -> u32 {
        b::resize(&mut mem(), this, n, const { c::resize_of(0x0059_93a0) })
    }
    /// `Phyre_PArray_resize_005b3040(this; n)` -> 0 or 0xd.
    Phyre_PArray_resize_005b3040 @ 0x005b_3040 => extern "thiscall" fn resize_005b3040(this: u32, n: u32) -> u32 {
        b::resize(&mut mem(), this, n, const { c::resize_of(0x005b_3040) })
    }
    /// `Phyre_PArray_resize_00a2d9e0(this; n)` -> 0 or 0xd.
    Phyre_PArray_resize_00a2d9e0 @ 0x00a2_d9e0 => extern "thiscall" fn resize_00a2d9e0(this: u32, n: u32) -> u32 {
        b::resize(&mut mem(), this, n, const { c::resize_of(0x00a2_d9e0) })
    }
    /// `Phyre_PSharray_resize_0048c470(this; n)` -> 0 or 0xd.
    Phyre_PSharray_resize_0048c470 @ 0x0048_c470 => extern "thiscall" fn sharray_resize(this: u32, n: u32) -> u32 {
        b::sharray_resize(&mut mem(), this, n)
    }
    /// `Phyre_PArray_PLODLevel_resize_005a7b60(this; n)` -> 0 or 0xd.
    Phyre_PArray_PLODLevel_resize_005a7b60 @ 0x005a_7b60 => extern "thiscall" fn lod_levels_resize(this: u32, n: u32) -> u32 {
        b::lod_levels_resize(&mut mem(), this, n)
    }
    /// `Phyre_PSharray_element16_00585b50(sharray; i)`: the code of `Phyre_PSharray_element28_00585c90` for 16-byte elements.
    Phyre_PSharray_element16_00585b50 @ 0x0058_5b50 => extern "thiscall" fn element16(this: u32, i: u32) -> u32 {
        ffx_prender::array::bridge::element(&mut mem(), this, i, 0x10)
    }
    /// `PArray12_at_005a7770(owner; i)`: the 12-byte `PSharray` at `+0x14`.
    PArray12_at_005a7770 @ 0x005a_7770 => extern "thiscall" fn element12(this: u32, i: u32) -> u32 {
        ffx_prender::array::bridge::element(&mut mem(), this.wrapping_add(0x14), i, 12)
    }
    /// `Phyre_PStreamInputDescD3D11_copyArray_0057e190(dst, src, n)`.
    Phyre_PStreamInputDescD3D11_copyArray_0057e190 @ 0x0057_e190 => extern "C" fn copy_stream_descs(dst: u32, src: u32, n: u32) {
        b::copy_stream_descs(&mut mem(), dst, src, n);
    }
    /// `Phyre_zeroElements20_0058afd0(dst, n)`.
    Phyre_zeroElements20_0058afd0 @ 0x0058_afd0 => extern "C" fn zero20(dst: u32, n: u32) {
        b::zero20(&mut mem(), dst, n);
    }
    /// `Phyre_PMeshInstanceSegmentContext_copyArray_004f0950(dst, src, n)`.
    Phyre_PMeshInstanceSegmentContext_copyArray_004f0950 @ 0x004f_0950 => extern "C" fn copy_segment_contexts(dst: u32, src: u32, n: u32) {
        b::copy_segment_contexts(&mut mem(), dst, src, n);
    }
    /// `Phyre_PLODLevel_copyArray_005a0f80(dst, src, n)`.
    Phyre_PLODLevel_copyArray_005a0f80 @ 0x005a_0f80 => extern "C" fn copy_lod_levels(dst: u32, src: u32, n: u32) {
        b::copy_lod_levels(&mut mem(), dst, src, n);
    }
    /// `Phyre_PLODLevel_constructArray_005a6280(dst, n)`.
    Phyre_PLODLevel_constructArray_005a6280 @ 0x005a_6280 => extern "C" fn default_lod_levels(dst: u32, n: u32) {
        b::default_lod_levels(&mut mem(), dst, n);
    }
    /// `Phyre_PSharray12_copy_005a2440(this; &src)` -> `this`.
    Phyre_PSharray12_copy_005a2440 @ 0x005a_2440 => extern "thiscall" fn sharray12_copy(this: u32, src: u32) -> u32 {
        b::sharray12_copy(&mut mem(), this, src)
    }
    /// `Phyre_PSharray_PString_setToExternalBuffer_0044c960(this; data, n)` -> 0 or 5.
    Phyre_PSharray_PString_setToExternalBuffer_0044c960 @ 0x0044_c960 => extern "thiscall" fn string_sharray_set(this: u32, data: u32, n: u32) -> u32 {
        b::string_sharray_set(&mut mem(), this, data, n)
    }
    /// `Phyre_PSharray_PString_assign_0044ca80(this; data, n)` -> 0.
    Phyre_PSharray_PString_assign_0044ca80 @ 0x0044_ca80 => extern "thiscall" fn string_sharray_assign(this: u32, data: u32, n: u32) -> u32 {
        b::string_sharray_assign(&mut mem(), this, data, n)
    }
    /// `Phyre_releaseRecordArrays_00431bf0(this)`.
    Phyre_releaseRecordArrays_00431bf0 @ 0x0043_1bf0 => extern "thiscall" fn record_dtor(this: u32) -> u32 {
        b::record_dtor(&mut mem(), this)
    }
    /// `Phyre_constructArray64_004343c0(p, n)`.
    Phyre_constructArray64_004343c0 @ 0x0043_43c0 => extern "C" fn construct64(p: u32, n: u32) {
        b::construct64(&mut mem(), p, n);
    }
    /// `Phyre_PStringBuilder_PStringBuilder_004449e0(this; s)` -> `this`.
    Phyre_PStringBuilder_PStringBuilder_004449e0 @ 0x0044_49e0 => extern "thiscall" fn builder_new(this: u32, s: u32) -> u32 {
        b::builder_new(&mut mem(), this, s)
    }
    /// `Phyre_PStringBuilder_appendInt_00444b90(this; v)` -> the append's `eax`; the text in a per-call frame.
    Phyre_PStringBuilder_appendInt_00444b90 @ 0x0044_4b90 => extern "thiscall" fn builder_append_int(this: u32, v: u32) -> u32 {
        let mut frame = StackFrame::<14>::new();
        b::builder_append_int(&mut mem(), this, v, frame.at())
    }
    /// `Phyre_PThreadPool_dtorBody_0044d070(this)`; the join's exit code in a per-call frame.
    Phyre_PThreadPool_dtorBody_0044d070 @ 0x0044_d070 => extern "thiscall" fn pool_dtor_body(this: u32) -> u32 {
        let mut frame = StackFrame::<1>::new();
        b::pool_dtor_body(&mut mem(), this, frame.at())
    }
    /// `Matrix3x4_Identity_00450ee0(m)` -> `m`; the identity in a per-call frame.
    Matrix3x4_Identity_00450ee0 @ 0x0045_0ee0 => extern "thiscall" fn identity_3x4(this: u32) -> u32 {
        let mut frame = StackFrame::<16>::new();
        b::identity_3x4(&mut mem(), this, frame.at())
    }
}
