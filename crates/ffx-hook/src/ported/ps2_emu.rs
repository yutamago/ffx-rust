//! `ffx-ps2`: the VU0 vector and matrix library and the emulated
//! registers' setters. The "void" originals here leave a value in `eax`
//! (the last word they copied, or a pointer); the shims return it.

use ffx_ps2::{ee::bridge as ee, vu0::bridge as vu0};

use super::{mem, ported};

ported! {
    /// `FFXVu0CopyVector(dst, src)`.
    FFXVu0CopyVector @ 0x006e_d370 => extern "C" fn ffx_vu0_copy_vector(dst: u32, src: u32) -> u32 {
        vu0::copy_vector(&mut mem(), dst, src)
    }
    /// `FFXVu0CopyVectorXYZ(dst, src)`.
    FFXVu0CopyVectorXYZ @ 0x0062_fca0 => extern "C" fn ffx_vu0_copy_vector_xyz(dst: u32, src: u32) -> u32 {
        vu0::copy_vector_xyz(&mut mem(), dst, src)
    }
    /// `FFXVu0AddVector(dst, a, b)`.
    FFXVu0AddVector @ 0x006e_d290 => extern "C" fn ffx_vu0_add_vector(dst: u32, a: u32, b: u32) -> u32 {
        vu0::add_vector(&mut mem(), dst, a, b)
    }
    /// `FFXVu0SubVector(dst, a, b)`.
    FFXVu0SubVector @ 0x006e_d940 => extern "C" fn ffx_vu0_sub_vector(dst: u32, a: u32, b: u32) -> u32 {
        vu0::sub_vector(&mut mem(), dst, a, b)
    }
    /// `FFXVu0MulVector(dst, a, b)`.
    FFXVu0MulVector @ 0x006e_d5d0 => extern "C" fn ffx_vu0_mul_vector(dst: u32, a: u32, b: u32) -> u32 {
        vu0::mul_vector(&mut mem(), dst, a, b)
    }
    /// `FFXVu0ScaleVector(dst, v, s)`.
    FFXVu0ScaleVector @ 0x006e_d8f0 => extern "C" fn ffx_vu0_scale_vector(dst: u32, v: u32, s: f32) -> u32 {
        vu0::scale_vector(&mut mem(), dst, v, s)
    }
    /// `FFXVu0DivVector(dst, v, s)`.
    FFXVu0DivVector @ 0x006e_d3a0 => extern "C" fn ffx_vu0_div_vector(dst: u32, v: u32, s: f32) -> u32 {
        vu0::div_vector(&mut mem(), dst, v, s)
    }
    /// `FFXVu0DivVectorXYZ(dst, v, s)`.
    FFXVu0DivVectorXYZ @ 0x006e_d420 => extern "C" fn ffx_vu0_div_vector_xyz(dst: u32, v: u32, s: f32) -> u32 {
        vu0::div_vector_xyz(&mut mem(), dst, v, s)
    }
    /// `FFXVu0InnerProduct(a, b)`.
    FFXVu0InnerProduct @ 0x0062_fce0 => extern "C" fn ffx_vu0_inner_product(a: u32, b: u32) -> f32 {
        vu0::inner_product(&mem(), a, b)
    }
    /// `FFXVu0OuterProduct(dst, a, b)`.
    FFXVu0OuterProduct @ 0x006e_d720 => extern "C" fn ffx_vu0_outer_product(dst: u32, a: u32, b: u32) -> u32 {
        vu0::outer_product(&mut mem(), dst, a, b)
    }
    /// `FFXVu0InterVector(dst, a, b, t)`.
    FFXVu0InterVector @ 0x0062_fd10 => extern "C" fn ffx_vu0_inter_vector(dst: u32, a: u32, b: u32, t: f32) -> u32 {
        vu0::inter_vector(&mut mem(), dst, a, b, t)
    }
    /// `FFXVu0InterVectorXYZ(dst, a, b, t)`.
    FFXVu0InterVectorXYZ @ 0x0062_fd70 => extern "C" fn ffx_vu0_inter_vector_xyz(dst: u32, a: u32, b: u32, t: f32) -> u32 {
        vu0::inter_vector_xyz(&mut mem(), dst, a, b, t)
    }
    /// `Matrix_Identity()` (`this` in ECX).
    Matrix_Identity @ 0x006e_ce60 => extern "thiscall" fn matrix_identity(this: u32) -> u32 {
        vu0::matrix_identity(&mut mem(), this)
    }
    /// `FFXVu0UnitMatrix(m)`.
    FFXVu0UnitMatrix @ 0x006e_d9a0 => extern "C" fn ffx_vu0_unit_matrix(m: u32) -> u32 {
        vu0::matrix_identity(&mut mem(), m)
    }
    /// `Matrix::copy(src)`.
    Matrix__copy @ 0x006e_cc40 => extern "thiscall" fn matrix_copy(this: u32, src: u32) -> u32 {
        vu0::matrix_copy(&mut mem(), this, src)
    }
    /// `FFXVu0CopyMatrix(dst, src)`.
    FFXVu0CopyMatrix @ 0x006e_d360 => extern "C" fn ffx_vu0_copy_matrix(dst: u32, src: u32) -> u32 {
        vu0::matrix_copy(&mut mem(), dst, src)
    }
    /// `FFXVu0ApplyMatrix(dst, m, v)`.
    FFXVu0ApplyMatrix @ 0x006e_d2e0 => extern "C" fn ffx_vu0_apply_matrix(dst: u32, m: u32, v: u32) -> u32 {
        vu0::apply_matrix(&mut mem(), dst, m, v)
    }
    /// `FFXVu0RotTransPers(dst, m, v, keep)`.
    FFXVu0RotTransPers @ 0x006e_d840 => extern "C" fn ffx_vu0_rot_trans_pers(dst: u32, m: u32, v: u32, keep: u32) -> u32 {
        vu0::rot_trans_pers(&mut mem(), dst, m, v, keep)
    }
    /// `FFXVu0LightColorMatrix(dst, c0, c1, c2, ambient)`.
    FFXVu0LightColorMatrix @ 0x006e_d4f0 => extern "C" fn ffx_vu0_light_color_matrix(dst: u32, c0: u32, c1: u32, c2: u32, ambient: u32) -> u32 {
        vu0::light_color_matrix(&mut mem(), dst, [c0, c1, c2, ambient])
    }
    /// `FFXVu0NormalLightMatrix(dst, l0, l1, l2)`.
    FFXVu0NormalLightMatrix @ 0x006e_d620 => extern "C" fn ffx_vu0_normal_light_matrix(dst: u32, l0: u32, l1: u32, l2: u32) -> u32 {
        vu0::normal_light_matrix(&mut mem(), dst, [l0, l1, l2])
    }

    /// `AsmEmuSetFuncParam0(v)`: `a0`.
    AsmEmuSetFuncParam0 @ 0x0042_eec0 => extern "C" fn asm_emu_set_func_param0(v: u32) -> u32 {
        ee::set_func_param(&mut mem(), 0, v)
    }
    /// `AsmEmuSetFuncParam1(v)`: `a1`.
    AsmEmuSetFuncParam1 @ 0x0042_eed0 => extern "C" fn asm_emu_set_func_param1(v: u32) -> u32 {
        ee::set_func_param(&mut mem(), 1, v)
    }
    /// `AsmEmuSetFuncParam2(v)`: `a2`.
    AsmEmuSetFuncParam2 @ 0x0042_eee0 => extern "C" fn asm_emu_set_func_param2(v: u32) -> u32 {
        ee::set_func_param(&mut mem(), 2, v)
    }
    /// `AsmEmuSetFuncParam3(v)`: `a3`.
    AsmEmuSetFuncParam3 @ 0x0042_eef0 => extern "C" fn asm_emu_set_func_param3(v: u32) -> u32 {
        ee::set_func_param(&mut mem(), 3, v)
    }
    /// `restoreVf00Register()`.
    restoreVf00Register @ 0x0042_ef00 => extern "C" fn restore_vf00_register() {
        ee::restore_vf00(&mut mem());
    }
    /// `setZSFlag()`.
    setZSFlag @ 0x0042_ef30 => extern "C" fn set_zs_flag() {
        ee::set_zs_flag(&mut mem());
    }
    /// `setSSFlag()`.
    setSSFlag @ 0x0042_ef20 => extern "C" fn set_ss_flag() {
        ee::set_ss_flag(&mut mem());
    }
}
