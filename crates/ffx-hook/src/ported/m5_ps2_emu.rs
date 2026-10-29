//! M5 stage 1, list `ps2-emu` (`ffx_ps2::m5_ps2`): the PS2 layer's
//! jump thunks to the `cdc` matrix code, the allocator and `memcpy`, a
//! getter, the input arrays' class descriptor singletons, and the Ogg
//! reader's end-of-page check.
//!
//! Threads: whichever thread calls them (the matrix thunks run on the job
//! workers too); the shims keep no state.

use ffx_bridge::RawMem;
use ffx_ps2::m5_ps2::bridge as b;

use super::{mem, ported};

/// The game as the bridges' host (`ffx_graphic::host::Host`).
fn game() -> RawMem {
    mem()
}

ported! {
    /// `FFXVu0MulMatrix(dst, a, b)`: `jmp cdc_Mul4x4`.
    FFXVu0MulMatrix @ 0x006e_d5c0 => extern "C" fn ffx_vu0_mul_matrix(dst: u32, a: u32, bb: u32) -> u32 {
        b::jmp_thunk(&mut game(), b::CDC_MUL4X4, &[dst, a, bb])
    }
    /// `FFXVu0InversMatrix(dst, src)`: `jmp cdc_OrthonormalInverse3x4`.
    FFXVu0InversMatrix @ 0x006e_d450 => extern "C" fn ffx_vu0_invers_matrix(dst: u32, src: u32) -> u32 {
        b::jmp_thunk(&mut game(), b::CDC_ORTHONORMAL_INVERSE, &[dst, src])
    }
    /// `jmp cdc_Transpose(dst, src)`.
    cdc_Transpose_thunk_006ed990 @ 0x006e_d990 => extern "C" fn transpose_thunk(dst: u32, src: u32) -> u32 {
        b::jmp_thunk(&mut game(), b::CDC_TRANSPOSE, &[dst, src])
    }
    /// `FFXVu0MulMatrix_thunk(dst, a, b)`: `jmp FFXVu0MulMatrix`.
    FFXVu0MulMatrix_thunk @ 0x009d_ba10 => extern "C" fn ffx_vu0_mul_matrix_thunk(dst: u32, a: u32, bb: u32) -> u32 {
        b::jmp_thunk(&mut game(), b::FFX_VU0_MUL_MATRIX, &[dst, a, bb])
    }
    /// `jmp memcpy(dst, src, n)`.
    memcpy_thunk_009db180 @ 0x009d_b180 => extern "C" fn memcpy_thunk(dst: u32, src: u32, n: u32) -> u32 {
        b::jmp_thunk(&mut game(), b::MEMCPY, &[dst, src, n])
    }
    /// `jmp code_00686fe0(p)` (`user_aligned_free`).
    AlignedFree_thunk_009dbe80 @ 0x009d_be80 => extern "C" fn aligned_free_thunk(p: u32) -> u32 {
        b::jmp_thunk(&mut game(), b::ALIGNED_FREE_THUNK, &[p])
    }
    /// `user_aligned_malloc(size, align)` behind an empty frame.
    AlignedMalloc_thunk_009dbe90 @ 0x009d_be90 => extern "C" fn aligned_malloc_thunk(size: u32, align: u32) -> u32 {
        b::jmp_thunk(&mut game(), b::USER_ALIGNED_MALLOC, &[size, align])
    }
    /// `code_009db170()`: the word at `0x012fbc40`.
    code_009db170 @ 0x009d_b170 => extern "C" fn get_word_012fbc40() -> u32 {
        b::get_word_012fbc40(&mut game())
    }
    /// `PClassDescriptorConcrete<PArray<PInputAction>>`'s accessor.
    PArrayInputAction_descriptor_009dc580 @ 0x009d_c580 => extern "C" fn input_action_descriptor() -> u32 {
        b::descriptor(&mut game(), b::INPUT_ACTION_ARRAY)
    }
    /// `PClassDescriptorConcrete<PArray<PInputSource>>`'s accessor.
    PArrayInputSource_descriptor_009dc610 @ 0x009d_c610 => extern "C" fn input_source_descriptor() -> u32 {
        b::descriptor(&mut game(), b::INPUT_SOURCE_ARRAY)
    }
    /// `0x0042d370(state)` (`ecx`).
    OggPage_end_0042d370 @ 0x0042_d370 => extern "thiscall" fn ogg_page_end(this: u32) -> u32 {
        b::ogg_page_end(&mut game(), this)
    }
}
