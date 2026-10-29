//! `ffx-ps2`: the VU0 vector and matrix library and the emulated
//! registers' setters. The "void" originals here leave a value in `eax`
//! (the last word they copied, or a pointer); the shims return it.

use ffx_ps2::{ee::bridge as ee, vu0::bridge as vu0};

use super::{mem, ported};
use ffx_ps2::vu0::{Axis, Trig};

/// MSVCR110's `_CIcos` or `_CIsin` through the game's import slot
/// (`0x00b0c3b4`, `0x00b0c3b8`): the angle in `ST0` as `fld dword` loads
/// it, the result stored as `fstp dword` stores it.
fn crt_trig(t: Trig, x: f32) -> f32 {
    let slot = match t {
        Trig::Cos => 0x00b0_c3b4,
        Trig::Sin => 0x00b0_c3b8,
    };
    // the slot holds the address of the CRT's intrinsic, which takes its
    // argument in ST0 (`fld dword`) and returns in ST0 (`fstp dword`)
    ffx_bridge::route::x87(super::import_slot(slot), &[f64::from(x)])
}

/// MSVCR110's `srand`, through the game's import slot (`0x00b0c484`).
fn crt_srand(seed: u32) {
    // the import slot holds the address of MSVCR110's cdecl `srand(seed)`
    ffx_bridge::route::cdecl::<()>(super::import_slot(0x00b0_c484), &[seed]);
}

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
    /// `AsmEmuInit()`.
    AsmEmuInit @ 0x0042_ee40 => extern "C" fn asm_emu_init() {
        ee::asm_emu_init(&mut mem());
    }
    /// `restoreVf00Register()`.
    restoreVf00Register @ 0x0042_ef00 => extern "C" fn restore_vf00_register() {
        ee::restore_vf00(&mut mem());
    }
    /// `setZSFlag()`.
    setZSFlag @ 0x0042_ef30 => extern "C" fn set_zs_flag() {
        ee::set_zs_flag(&mut mem());
    }
    /// `FFXVu0Normalize(dst, src)`.
    FFXVu0Normalize @ 0x006e_d6f0 => extern "C" fn ffx_vu0_normalize(dst: u32, src: u32) {
        vu0::normalize(&mut mem(), dst, src);
    }
    /// `FFXVu0RotMatrixX(dst, src, angle)`.
    FFXVu0RotMatrixX @ 0x006e_d780 => extern "C" fn ffx_vu0_rot_matrix_x(dst: u32, src: u32, angle: f32) -> u32 {
        vu0::rot_matrix(&mut mem(), Axis::X, dst, src, angle, &mut crt_trig)
    }
    /// `FFXVu0RotMatrixY(dst, src, angle)`.
    FFXVu0RotMatrixY @ 0x006e_d7c0 => extern "C" fn ffx_vu0_rot_matrix_y(dst: u32, src: u32, angle: f32) -> u32 {
        vu0::rot_matrix(&mut mem(), Axis::Y, dst, src, angle, &mut crt_trig)
    }
    /// `FFXVu0RotMatrixZ(dst, src, angle)`.
    FFXVu0RotMatrixZ @ 0x006e_d800 => extern "C" fn ffx_vu0_rot_matrix_z(dst: u32, src: u32, angle: f32) -> u32 {
        vu0::rot_matrix(&mut mem(), Axis::Z, dst, src, angle, &mut crt_trig)
    }
    /// `FFXVu0ClipScreen()`.
    FFXVu0ClipScreen @ 0x0083_5b90 => extern "C" fn ffx_vu0_clip_screen() -> u32 {
        vu0::clip_screen(&mem())
    }
    /// `MagicNormalizeLen_009dbaf0(v, out, len)`.
    MagicNormalizeLen_009dbaf0 @ 0x009d_baf0 => extern "C" fn magic_normalize_len(v: u32, out: u32, len: u32) -> u32 {
        vu0::normalize_len(&mut mem(), v, out, len)
    }
    /// `MagicRandFloat_009dbb40(out)`.
    MagicRandFloat_009dbb40 @ 0x009d_bb40 => extern "C" fn magic_rand_float(out: u32) -> u32 {
        vu0::rand_float(&mut mem(), out, |m| {
            ffx_vfx::rand::bridge::draw(m, &mut |_, seed| crt_srand(seed))
        })
    }
    /// `setSSFlag()`.
    setSSFlag @ 0x0042_ef20 => extern "C" fn set_ss_flag() {
        ee::set_ss_flag(&mut mem());
    }
}
