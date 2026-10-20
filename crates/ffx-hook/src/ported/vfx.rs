//! `ffx-vfx`: particle and effect functions. All cdecl; functions whose
//! original leaves a result in `eax` return the full `u32`.

use super::{mem, ported};
use ffx_vfx::mat::{self, order};
use ffx_vfx::rand::bridge::{Kind, Spread};
use ffx_vfx::{
    anmm, draw, et, fp, heap, lns, md, mdl, op, opd, opus, part, ppp, prog, rand, slots, small,
    tree, vec, wave,
};

/// MSVCR110's `srand`, through the game's import slot (`0x00b0c484`), so the
/// CRT's state moves as it does with the originals.
fn crt_srand(seed: u32) {
    // SAFETY: the slot holds the address of the CRT's cdecl `srand(seed)`.
    unsafe {
        let slot = crate::rebase(0x00b0_c484) as *const usize;
        let f: unsafe extern "C" fn(u32) = std::mem::transmute(slot.read());
        f(seed);
    }
}

/// MSVCR110's `_CIatan` (`ST0` in and out) through the game's import slot
/// (`0x00b0c450`, behind the thunk `0x00949d5c`), on an x87 value the
/// caller left unrounded (a 53-bit quotient, exactly an `f64`).
fn crt_atan(x: f64) -> f64 {
    let slot = crate::rebase(0x00b0_c450) as *const usize;
    let mut out = 0f64;
    // SAFETY: the slot holds the CRT's `_CIatan`, which takes its operand
    // in ST0 and leaves the result there; the asm loads it, calls it and
    // stores the result through edi, leaving the x87 stack empty.
    unsafe {
        let f = slot.read();
        std::arch::asm!(
            "fld qword ptr [{x}]",
            "call {f}",
            "fstp qword ptr [edi]",
            x = in(reg) &raw const x,
            // edi: the callee keeps it (eax, ecx and edx it may not)
            in("edi") &raw mut out,
            f = in(reg) f,
            clobber_abi("C"),
        );
    }
    out
}

/// MSVCR110's `_CIatan2` (the x87 form: `ST1` = y, `ST0` = x, the result in
/// `ST0`), through the game's import slot (`0x00b0c3b0`).
fn crt_atan2(y: f64, x: f64) -> f64 {
    let slot = crate::rebase(0x00b0_c3b0) as *const usize;
    let mut out = 0f64;
    // SAFETY: the slot holds the CRT's `_CIatan2`, which takes its two
    // operands on the x87 stack and leaves the result there; the asm loads
    // them, calls it and stores the result, leaving the x87 stack empty.
    unsafe {
        let f = slot.read();
        std::arch::asm!(
            "fld qword ptr [{y}]",
            "fld qword ptr [{x}]",
            "call {f}",
            "fstp qword ptr [edi]",
            y = in(reg) &raw const y,
            x = in(reg) &raw const x,
            // edi: the callee keeps it (eax, ecx and edx it may not)
            in("edi") &raw mut out,
            f = in(reg) f,
            clobber_abi("C"),
        );
    }
    out
}

/// MSVCR110's `_CIcos` (`cos`) or `_CIsin` through the game's import slot
/// (`0x00b0c3b4`, `0x00b0c3b8`): the angle loaded as `fld dword`, the
/// result stored as `fstp dword` through `edi`.
fn crt_trig(cos: bool, x: f32) -> f32 {
    let slot = crate::rebase(if cos { 0x00b0_c3b4 } else { 0x00b0_c3b8 }) as *const usize;
    let mut out = 0f32;
    // SAFETY: the slot holds the CRT's intrinsic, which takes its operand
    // in ST0 and leaves the result there; the asm loads it, calls it and
    // stores the result through edi (which the callee keeps), leaving the
    // x87 stack empty.
    unsafe {
        let f = slot.read();
        std::arch::asm!(
            "fld dword ptr [{x}]",
            "call {f}",
            "fstp dword ptr [edi]",
            x = in(reg) &raw const x,
            f = in(reg) f,
            in("edi") &raw mut out,
            clobber_abi("C"),
        );
    }
    out
}

/// MSVCR110's `_CItan` through the game's import slot (`0x00b0c3a0`): the
/// operand loaded as `fld qword`, the result stored as `fstp qword`
/// through `edi`.
fn crt_tan(x: f64) -> f64 {
    let slot = crate::rebase(0x00b0_c3a0) as *const usize;
    let mut out = 0f64;
    // SAFETY: the slot holds the CRT's intrinsic, which takes its operand
    // in ST0 and leaves the result there; the asm loads it, calls it and
    // stores the result through edi (which the callee keeps), leaving the
    // x87 stack empty.
    unsafe {
        let f = slot.read();
        std::arch::asm!(
            "fld qword ptr [{x}]",
            "call {f}",
            "fstp qword ptr [edi]",
            x = in(reg) &raw const x,
            f = in(reg) f,
            in("edi") &raw mut out,
            clobber_abi("C"),
        );
    }
    out
}

/// MSVCR110's `rand()`, through the game's import slot (`0x00b0c420`).
fn crt_rand() -> u32 {
    // SAFETY: the slot holds the address of the CRT's cdecl `rand()`.
    unsafe {
        let slot = crate::rebase(0x00b0_c420) as *const usize;
        let f: unsafe extern "C" fn() -> u32 = std::mem::transmute(slot.read());
        f()
    }
}

/// A function the originals call through a pointer (the program table's
/// `+4`..`+0x24`, `TOGetFFXLang`, `graphicVFXDestroy`, ...): cdecl, up to twelve
/// arguments; returns `eax`.
fn call_ptr(f: u32, args: &[u32]) -> u32 {
    super::call_cdecl(f, args)
}

/// An original cdecl function of up to 12 word arguments that returns a
/// float in `ST0` (`MsGetChrWidth`, `Ch_GetThickness`): the value at the
/// x87's precision (the declared `f64` return pops `ST0`).
#[allow(clippy::many_single_char_names)] // the arguments, in order
fn call_ptr_f64(f: u32, a: &[u32]) -> f64 {
    type F12 =
        unsafe extern "C" fn(u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32) -> f64;
    let mut w = [0u32; 12];
    let n = a.len().min(12);
    w[..n].copy_from_slice(&a[..n]);
    // SAFETY: a cdecl function of at most 12 word arguments returning in
    // ST0; the extra zero words sit on the caller's side of the stack.
    unsafe {
        let g: F12 = std::mem::transmute::<usize, F12>(f as usize);
        g(
            w[0], w[1], w[2], w[3], w[4], w[5], w[6], w[7], w[8], w[9], w[10], w[11],
        )
    }
}

/// A rand program with the CRT's `srand`.
fn rand_program(obj: u32, key: u32, ctrl: u32, spread: Spread, kind: Kind) {
    rand::bridge::ppp_rand(&mut mem(), obj, key, ctrl, spread, kind, &mut |_, seed| {
        crt_srand(seed);
    });
}

/// A seeded-per-axis rand program with the CRT's `srand`.
fn srand_program(obj: u32, key: u32, ctrl: u32, spread: Spread, kind: Kind) {
    rand::bridge::ppp_srand(&mut mem(), obj, key, ctrl, spread, kind, &mut |_, seed| {
        crt_srand(seed);
    });
}

ported! {
    // ---- rand ----
    /// `op_random(x)`: a random value in `[-x, x)` from VU0's `R`.
    op_random @ 0x007e_6700 => extern "C" fn op_random(x: f32) -> f32 {
        rand::bridge::op_random(&mut mem(), x)
    }
    /// `op_random_p(x)`: a random value in `[0, x)`.
    op_random_p @ 0x007e_6780 => extern "C" fn op_random_p(x: f32) -> f32 {
        rand::bridge::op_random_p(&mut mem(), x)
    }

    // ---- rand programs ----
    /// `pppVu0RNext(out, _)`: `vrnext` into `out`.
    pppVu0RNext @ 0x0072_f420 => extern "C" fn ppp_vu0_rnext(out: u32, r: u32) {
        let _ = r;
        rand::bridge::ppp_vu0_rnext(&mut mem(), out);
    }

    /// `pppRandFloat(obj, key, ctrl)`.
    pppRandFloat @ 0x0072_f4b0 => extern "C" fn ppp_rand_float(obj: u32, key: u32, ctrl: u32) {
        rand_program(obj, key, ctrl, Spread::Plain, Kind::Float);
    }
    /// `pppRandUpFloat(obj, key, ctrl)`.
    pppRandUpFloat @ 0x0072_f680 => extern "C" fn ppp_rand_up_float(obj: u32, key: u32, ctrl: u32) {
        rand_program(obj, key, ctrl, Spread::Up, Kind::Float);
    }
    /// `pppRandDownFloat(obj, key, ctrl)`.
    pppRandDownFloat @ 0x0072_f850 => extern "C" fn ppp_rand_down_float(obj: u32, key: u32, ctrl: u32) {
        rand_program(obj, key, ctrl, Spread::Down, Kind::Float);
    }
    /// `pppRandChar(obj, key, ctrl)`.
    pppRandChar @ 0x0072_fa20 => extern "C" fn ppp_rand_char(obj: u32, key: u32, ctrl: u32) {
        rand_program(obj, key, ctrl, Spread::Plain, Kind::Char);
    }
    /// `pppRandUpChar(obj, key, ctrl)`.
    pppRandUpChar @ 0x0072_fbf0 => extern "C" fn ppp_rand_up_char(obj: u32, key: u32, ctrl: u32) {
        rand_program(obj, key, ctrl, Spread::Up, Kind::Char);
    }
    /// `pppRandDownChar(obj, key, ctrl)`.
    pppRandDownChar @ 0x0072_fdc0 => extern "C" fn ppp_rand_down_char(obj: u32, key: u32, ctrl: u32) {
        rand_program(obj, key, ctrl, Spread::Down, Kind::Char);
    }
    /// `pppRandShort(obj, key, ctrl)`.
    pppRandShort @ 0x0072_ffa0 => extern "C" fn ppp_rand_short(obj: u32, key: u32, ctrl: u32) {
        rand_program(obj, key, ctrl, Spread::Plain, Kind::Short);
    }
    /// `pppRandUpShort(obj, key, ctrl)`.
    pppRandUpShort @ 0x0073_0170 => extern "C" fn ppp_rand_up_short(obj: u32, key: u32, ctrl: u32) {
        rand_program(obj, key, ctrl, Spread::Up, Kind::Short);
    }
    /// `pppRandDownShort(obj, key, ctrl)`.
    pppRandDownShort @ 0x0073_0340 => extern "C" fn ppp_rand_down_short(obj: u32, key: u32, ctrl: u32) {
        rand_program(obj, key, ctrl, Spread::Down, Kind::Short);
    }
    /// `pppRandInt(obj, key, ctrl)`.
    pppRandInt @ 0x0073_0520 => extern "C" fn ppp_rand_int(obj: u32, key: u32, ctrl: u32) {
        rand_program(obj, key, ctrl, Spread::Plain, Kind::Int);
    }
    /// `pppRandUpInt(obj, key, ctrl)`.
    pppRandUpInt @ 0x0073_0700 => extern "C" fn ppp_rand_up_int(obj: u32, key: u32, ctrl: u32) {
        rand_program(obj, key, ctrl, Spread::Up, Kind::Int);
    }
    /// `pppRandDownInt(obj, key, ctrl)`.
    pppRandDownInt @ 0x0073_08d0 => extern "C" fn ppp_rand_down_int(obj: u32, key: u32, ctrl: u32) {
        rand_program(obj, key, ctrl, Spread::Down, Kind::Int);
    }
    /// `pppRandFV(obj, key, ctrl)`.
    pppRandFV @ 0x0073_0ab0 => extern "C" fn ppp_rand_fv(obj: u32, key: u32, ctrl: u32) {
        rand_program(obj, key, ctrl, Spread::Plain, Kind::FloatVec);
    }
    /// `pppRandUpFV(obj, key, ctrl)`.
    pppRandUpFV @ 0x0073_0ce0 => extern "C" fn ppp_rand_up_fv(obj: u32, key: u32, ctrl: u32) {
        rand_program(obj, key, ctrl, Spread::Up, Kind::FloatVec);
    }
    /// `pppRandDownFV(obj, key, ctrl)`.
    pppRandDownFV @ 0x0073_0ef0 => extern "C" fn ppp_rand_down_fv(obj: u32, key: u32, ctrl: u32) {
        rand_program(obj, key, ctrl, Spread::Down, Kind::FloatVec);
    }
    /// `pppRandIV(obj, key, ctrl)`.
    pppRandIV @ 0x0073_1100 => extern "C" fn ppp_rand_iv(obj: u32, key: u32, ctrl: u32) {
        rand_program(obj, key, ctrl, Spread::Plain, Kind::IntVec);
    }
    /// `pppRandUpIV(obj, key, ctrl)`.
    pppRandUpIV @ 0x0073_1350 => extern "C" fn ppp_rand_up_iv(obj: u32, key: u32, ctrl: u32) {
        rand_program(obj, key, ctrl, Spread::Up, Kind::IntVec);
    }
    /// `pppRandDownIV(obj, key, ctrl)`.
    pppRandDownIV @ 0x0073_1560 => extern "C" fn ppp_rand_down_iv(obj: u32, key: u32, ctrl: u32) {
        rand_program(obj, key, ctrl, Spread::Down, Kind::IntVec);
    }
    /// `pppRandCV(obj, key, ctrl)`.
    pppRandCV @ 0x0073_1780 => extern "C" fn ppp_rand_cv(obj: u32, key: u32, ctrl: u32) {
        rand_program(obj, key, ctrl, Spread::Plain, Kind::CharVec);
    }
    /// `pppRandUpCV(obj, key, ctrl)`.
    pppRandUpCV @ 0x0073_19f0 => extern "C" fn ppp_rand_up_cv(obj: u32, key: u32, ctrl: u32) {
        rand_program(obj, key, ctrl, Spread::Up, Kind::CharVec);
    }
    /// `pppRandDownCV(obj, key, ctrl)`.
    pppRandDownCV @ 0x0073_1c50 => extern "C" fn ppp_rand_down_cv(obj: u32, key: u32, ctrl: u32) {
        rand_program(obj, key, ctrl, Spread::Down, Kind::CharVec);
    }
    /// `pppRandHCV(obj, key, ctrl)`.
    pppRandHCV @ 0x0073_1ec0 => extern "C" fn ppp_rand_hcv(obj: u32, key: u32, ctrl: u32) {
        rand_program(obj, key, ctrl, Spread::Plain, Kind::ShortVec);
    }
    /// `pppRandUpHCV(obj, key, ctrl)`.
    pppRandUpHCV @ 0x0073_2130 => extern "C" fn ppp_rand_up_hcv(obj: u32, key: u32, ctrl: u32) {
        rand_program(obj, key, ctrl, Spread::Up, Kind::ShortVec);
    }
    /// `pppRandDownHCV(obj, key, ctrl)`.
    pppRandDownHCV @ 0x0073_2390 => extern "C" fn ppp_rand_down_hcv(obj: u32, key: u32, ctrl: u32) {
        rand_program(obj, key, ctrl, Spread::Down, Kind::ShortVec);
    }
    /// `pppSRandFV(obj, key, ctrl)`.
    pppSRandFV @ 0x0073_25f0 => extern "C" fn ppp_srand_fv(obj: u32, key: u32, ctrl: u32) {
        srand_program(obj, key, ctrl, Spread::Plain, Kind::FloatVec);
    }
    /// `pppSRandUpFV(obj, key, ctrl)`.
    pppSRandUpFV @ 0x0073_28b0 => extern "C" fn ppp_srand_up_fv(obj: u32, key: u32, ctrl: u32) {
        srand_program(obj, key, ctrl, Spread::Up, Kind::FloatVec);
    }
    /// `pppSRandDownFV(obj, key, ctrl)`.
    pppSRandDownFV @ 0x0073_2b40 => extern "C" fn ppp_srand_down_fv(obj: u32, key: u32, ctrl: u32) {
        srand_program(obj, key, ctrl, Spread::Down, Kind::FloatVec);
    }
    /// `pppSRandCV(obj, key, ctrl)`.
    pppSRandCV @ 0x0073_2dd0 => extern "C" fn ppp_srand_cv(obj: u32, key: u32, ctrl: u32) {
        srand_program(obj, key, ctrl, Spread::Plain, Kind::CharVec);
    }
    /// `pppSRandUpCV(obj, key, ctrl)`.
    pppSRandUpCV @ 0x0073_30e0 => extern "C" fn ppp_srand_up_cv(obj: u32, key: u32, ctrl: u32) {
        srand_program(obj, key, ctrl, Spread::Up, Kind::CharVec);
    }
    /// `pppSRandDownCV(obj, key, ctrl)`.
    pppSRandDownCV @ 0x0073_33e0 => extern "C" fn ppp_srand_down_cv(obj: u32, key: u32, ctrl: u32) {
        srand_program(obj, key, ctrl, Spread::Down, Kind::CharVec);
    }
    /// `pppSRandHCV(obj, key, ctrl)`.
    pppSRandHCV @ 0x0073_36e0 => extern "C" fn ppp_srand_hcv(obj: u32, key: u32, ctrl: u32) {
        srand_program(obj, key, ctrl, Spread::Plain, Kind::ShortVec);
    }
    /// `pppSRandUpHCV(obj, key, ctrl)`.
    pppSRandUpHCV @ 0x0073_3a00 => extern "C" fn ppp_srand_up_hcv(obj: u32, key: u32, ctrl: u32) {
        srand_program(obj, key, ctrl, Spread::Up, Kind::ShortVec);
    }
    /// `pppSRandDownHCV(obj, key, ctrl)`.
    pppSRandDownHCV @ 0x0073_3d10 => extern "C" fn ppp_srand_down_hcv(obj: u32, key: u32, ctrl: u32) {
        srand_program(obj, key, ctrl, Spread::Down, Kind::ShortVec);
    }
    /// `pppSRandFV_Draw(two)`: one spread draw, in ST0.
    pppSRandFV_Draw @ 0x0073_26e0 => extern "C" fn ppp_srand_fv_draw(two: u32) -> f32 {
        rand::bridge::ppp_srand_draw(&mut mem(), Spread::Plain, two & 0xff != 0, &mut |_, seed| {
            crt_srand(seed);
        })
    }
    /// `pppSRandUpFV_Draw(two)`: one spread draw, in ST0.
    pppSRandUpFV_Draw @ 0x0073_2970 => extern "C" fn ppp_srand_up_fv_draw(two: u32) -> f32 {
        rand::bridge::ppp_srand_draw(&mut mem(), Spread::Up, two & 0xff != 0, &mut |_, seed| {
            crt_srand(seed);
        })
    }
    /// `pppSRandDownFV_Draw(two)`: one spread draw, in ST0.
    pppSRandDownFV_Draw @ 0x0073_2c00 => extern "C" fn ppp_srand_down_fv_draw(two: u32) -> f32 {
        rand::bridge::ppp_srand_draw(&mut mem(), Spread::Down, two & 0xff != 0, &mut |_, seed| {
            crt_srand(seed);
        })
    }
    /// `pppSRandCV_Draw(two)`: one spread draw, in ST0.
    pppSRandCV_Draw @ 0x0073_2f30 => extern "C" fn ppp_srand_cv_draw(two: u32) -> f32 {
        rand::bridge::ppp_srand_draw(&mut mem(), Spread::Plain, two & 0xff != 0, &mut |_, seed| {
            crt_srand(seed);
        })
    }
    /// `pppSRandUpCV_Draw(two)`: one spread draw, in ST0 (the same code as `pppSRandUpFV_Draw`, tools/dupes.py).
    pppSRandUpCV_Draw @ 0x0073_3230 => extern "C" fn ppp_srand_up_cv_draw(two: u32) -> f32 {
        rand::bridge::ppp_srand_draw(&mut mem(), Spread::Up, two & 0xff != 0, &mut |_, seed| {
            crt_srand(seed);
        })
    }
    /// `pppSRandDownCV_Draw(two)`: one spread draw, in ST0 (the same code as `pppSRandDownFV_Draw`, tools/dupes.py).
    pppSRandDownCV_Draw @ 0x0073_3530 => extern "C" fn ppp_srand_down_cv_draw(two: u32) -> f32 {
        rand::bridge::ppp_srand_draw(&mut mem(), Spread::Down, two & 0xff != 0, &mut |_, seed| {
            crt_srand(seed);
        })
    }
    /// `pppSRandHCV_Draw(two)`: one spread draw, in ST0 (the same code as `pppSRandFV_Draw`, tools/dupes.py).
    pppSRandHCV_Draw @ 0x0073_3820 => extern "C" fn ppp_srand_hcv_draw(two: u32) -> f32 {
        rand::bridge::ppp_srand_draw(&mut mem(), Spread::Plain, two & 0xff != 0, &mut |_, seed| {
            crt_srand(seed);
        })
    }
    /// `pppSRandUpHCV_Draw(two)`: one spread draw, in ST0 (the same code as `pppSRandUpFV_Draw`, tools/dupes.py).
    pppSRandUpHCV_Draw @ 0x0073_3b30 => extern "C" fn ppp_srand_up_hcv_draw(two: u32) -> f32 {
        rand::bridge::ppp_srand_draw(&mut mem(), Spread::Up, two & 0xff != 0, &mut |_, seed| {
            crt_srand(seed);
        })
    }
    /// `pppSRandDownHCV_Draw(two)`: one spread draw, in ST0 (the same code as `pppSRandDownFV_Draw`, tools/dupes.py).
    pppSRandDownHCV_Draw @ 0x0073_3e40 => extern "C" fn ppp_srand_down_hcv_draw(two: u32) -> f32 {
        rand::bridge::ppp_srand_draw(&mut mem(), Spread::Down, two & 0xff != 0, &mut |_, seed| {
            crt_srand(seed);
        })
    }
    // ---- ppp programs ----
    /// `pppKeDrctCon(obj, ctrl)`.
    pppKeDrctCon @ 0x0075_e470 => extern "C" fn ppp_ke_drct_con(obj: u32, ctrl: u32) {
        ppp::bridge::ppp_ke_drct_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeDrct(obj, key, ctrl)`.
    pppKeDrct @ 0x0075_e430 => extern "C" fn ppp_ke_drct(obj: u32, key: u32, ctrl: u32) {
        ppp::bridge::ppp_ke_drct(&mut mem(), obj, key, ctrl);
    }
    /// `pppKeMatSNCon(obj, ctrl)`.
    pppKeMatSNCon @ 0x0073_40f0 => extern "C" fn ppp_ke_mat_sn_con(obj: u32, ctrl: u32) {
        ppp::bridge::ppp_ke_mat_sn_con(&mut mem(), obj, ctrl);
    }
    /// `pppVtMimeCon(obj, ctrl)`.
    pppVtMimeCon @ 0x0072_ed70 => extern "C" fn ppp_vt_mime_con(obj: u32, ctrl: u32) {
        ppp::bridge::ppp_vt_mime_con(&mut mem(), obj, ctrl);
    }
    /// `pppNeiLightEikyoCon(obj, ctrl)`.
    pppNeiLightEikyoCon @ 0x0075_d6c0 => extern "C" fn ppp_nei_light_eikyo_con(obj: u32, ctrl: u32) {
        ppp::bridge::ppp_nei_light_eikyo_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeThCon2(obj, ctrl)`.
    pppKeThCon2 @ 0x0073_64e0 => extern "C" fn ppp_ke_th_con2(obj: u32, ctrl: u32) {
        ppp::bridge::ppp_ke_th_con2(&mut mem(), obj, ctrl);
    }
    /// `pppKeThSftCon(obj, ctrl)`.
    pppKeThSftCon @ 0x0073_7130 => extern "C" fn ppp_ke_th_sft_con(obj: u32, ctrl: u32) {
        ppp::bridge::ppp_ke_th_sft_con(&mut mem(), obj, ctrl);
    }
    /// `pppColorCon(obj, ctrl)`.
    pppColorCon @ 0x0075_d240 => extern "C" fn ppp_color_con(obj: u32, ctrl: u32) {
        ppp::bridge::ppp_color_con(&mut mem(), obj, ctrl);
    }
    /// `pppColor(obj, key, ctrl)`.
    pppColor @ 0x0075_d1f0 => extern "C" fn ppp_color(obj: u32, key: u32, ctrl: u32) {
        ppp::bridge::ppp_color(&mut mem(), obj, key, ctrl);
    }
    /// `pppAngle(obj, key, ctrl)`.
    pppAngle @ 0x0075_ce30 => extern "C" fn ppp_angle(obj: u32, key: u32, ctrl: u32) {
        ppp::bridge::ppp_angle(&mut mem(), obj, key, ctrl);
    }
    /// `pppNeiLightEikyo(obj, key, ctrl)`.
    pppNeiLightEikyo @ 0x0075_d5d0 => extern "C" fn ppp_nei_light_eikyo(obj: u32, key: u32, ctrl: u32) {
        ppp::bridge::ppp_nei_light_eikyo(&mut mem(), obj, key, ctrl);
    }
    /// `pppKeLnsLpSft(obj, key, ctrl)`.
    pppKeLnsLpSft @ 0x0075_9f30 => extern "C" fn ppp_ke_lns_lp_sft(obj: u32, key: u32, ctrl: u32) {
        ppp::bridge::ppp_ke_lns_lp_sft(&mut mem(), obj, key, ctrl);
    }
    /// `pppClearWait()`.
    pppClearWait @ 0x0076_b200 => extern "C" fn ppp_clear_wait() {
        ppp::bridge::ppp_clear_wait(&mut mem());
    }
    /// `pppGetRotMatrixX(out, angle)`.
    pppGetRotMatrixX @ 0x0072_c730 => extern "C" fn ppp_get_rot_matrix_x(out: u32, angle: i32) {
        ppp::bridge::ppp_get_rot_matrix_x(&mut mem(), out, angle);
    }
    /// `pppGetRotMatrixY(out, angle)`.
    pppGetRotMatrixY @ 0x0072_c920 => extern "C" fn ppp_get_rot_matrix_y(out: u32, angle: i32) {
        ppp::bridge::ppp_get_rot_matrix_y(&mut mem(), out, angle);
    }
    /// `pppGetRotMatrixZ(out, angle)`.
    pppGetRotMatrixZ @ 0x0072_cb10 => extern "C" fn ppp_get_rot_matrix_z(out: u32, angle: i32) {
        ppp::bridge::ppp_get_rot_matrix_z(&mut mem(), out, angle);
    }

    // ---- heaps ----
    /// `pppCreateHeap(heap, memory, size)`.
    pppCreateHeap @ 0x0072_c480 => extern "C" fn ppp_create_heap(h: u32, memory: u32, size: u32) {
        heap::bridge::ppp_create_heap(&mut mem(), h, memory, size);
    }
    /// `pppAlloc(heap, size)`.
    pppAlloc @ 0x0072_c3d0 => extern "C" fn ppp_alloc(h: u32, size: u32) -> u32 {
        heap::bridge::ppp_alloc(&mut mem(), h, size)
    }
    /// `pppFree(heap, data)`.
    pppFree @ 0x0072_c510 => extern "C" fn ppp_free(h: u32, data: u32) {
        heap::bridge::ppp_free(&mut mem(), h, data);
    }
    /// `pppFpGetHeapSize()`.
    pppFpGetHeapSize @ 0x0072_9290 => extern "C" fn ppp_fp_get_heap_size() -> u32 {
        heap::bridge::ppp_fp_get_heap_size(&mem())
    }

    // ---- field effects ----
    /// `Fp_Restart(id)`.
    Fp_Restart @ 0x0072_9700 => extern "C" fn fp_restart(id: u32) {
        fp::bridge::fp_restart(&mut mem(), id);
    }
    /// `Fp_RestartMode2(id)`.
    Fp_RestartMode2 @ 0x0072_9810 => extern "C" fn fp_restart_mode2(id: u32) {
        fp::bridge::fp_restart_mode2(&mut mem(), id);
    }
    /// `Fp_End(id)`.
    Fp_End @ 0x0072_9660 => extern "C" fn fp_end(id: u32) {
        fp::bridge::fp_end(&mut mem(), id);
    }
    /// `pppFpIDdispOff(id)`.
    pppFpIDdispOff @ 0x0072_9570 => extern "C" fn ppp_fp_id_disp_off(id: u32) {
        fp::bridge::ppp_fp_id_disp_off(&mut mem(), id);
    }
    /// `pppFpIDdispOn(id)`.
    pppFpIDdispOn @ 0x0072_95b0 => extern "C" fn ppp_fp_id_disp_on(id: u32) {
        fp::bridge::ppp_fp_id_disp_on(&mut mem(), id);
    }
    /// `pppFpGetMngPtr(id, &run, &disp)`.
    pppFpGetMngPtr @ 0x0072_92f0 => extern "C" fn ppp_fp_get_mng_ptr(id: u32, run: u32, disp: u32) {
        fp::bridge::ppp_fp_get_mng_ptr(&mut mem(), id, run, disp);
    }
    /// `pppFpResetStick(id)`.
    pppFpResetStick @ 0x0072_a120 => extern "C" fn ppp_fp_reset_stick(id: u32) {
        fp::bridge::ppp_fp_reset_stick(&mut mem(), id);
    }
    /// `pppFpSetStickMap(id, target)`.
    pppFpSetStickMap @ 0x0072_a590 => extern "C" fn ppp_fp_set_stick_map(id: u32, target: u32) {
        fp::bridge::ppp_fp_set_stick_map(&mut mem(), id, target);
    }
    /// `pppFpSetStickMapDObj(id, target)`.
    pppFpSetStickMapDObj @ 0x0072_a5e0 => extern "C" fn ppp_fp_set_stick_map_dobj(id: u32, target: u32) {
        fp::bridge::ppp_fp_set_stick_map_dobj(&mut mem(), id, target);
    }
    /// `pppFpStopStatus(stop)`.
    pppFpStopStatus @ 0x0072_a750 => extern "C" fn ppp_fp_stop_status(stop: u32) {
        fp::bridge::ppp_fp_stop_status(&mut mem(), stop);
    }
    /// `pppFpPointLightInit()`.
    pppFpPointLightInit @ 0x0072_9f70 => extern "C" fn ppp_fp_point_light_init() {
        fp::bridge::ppp_fp_point_light_init(&mut mem());
    }
    /// `pppFpSaveStat(out)`.
    pppFpSaveStat @ 0x0072_a180 => extern "C" fn ppp_fp_save_stat(out: u32) {
        fp::bridge::ppp_fp_save_stat(&mut mem(), out);
    }

    // ---- op layer ----
    /// `op_spr_alloc(n)`.
    op_spr_alloc @ 0x007e_6a00 => extern "C" fn op_spr_alloc(n: u32) -> u32 {
        op::bridge::op_spr_alloc(&mut mem(), n)
    }
    /// `op_spr_free()`.
    op_spr_free @ 0x007e_6a60 => extern "C" fn op_spr_free() {
        op::bridge::op_spr_free(&mut mem());
    }
    /// `op_m_copy(dst, src)`.
    op_m_copy @ 0x007e_8010 => extern "C" fn op_m_copy(dst: u32, src: u32) {
        op::bridge::op_m_copy(&mut mem(), dst, src);
    }
    /// `op_unit_m_scale(m, s)`.
    op_unit_m_scale @ 0x007e_a700 => extern "C" fn op_unit_m_scale(m: u32, s: f32) {
        op::bridge::op_unit_m_scale(&mut mem(), m, s);
    }
    /// `op_func_ot_set(func, a, b, c, list)`.
    op_func_ot_set @ 0x007e_b500 => extern "C" fn op_func_ot_set(func: u32, a: u32, b: u32, c: u32, list: u32) -> i32 {
        op::bridge::op_func_ot_set(&mut mem(), func, a, b, c, list)
    }
    /// `op_func_ot_free(slot, list)`.
    op_func_ot_free @ 0x007e_b4a0 => extern "C" fn op_func_ot_free(slot: u32, list: u32) {
        op::bridge::op_func_ot_free(&mut mem(), slot, list);
    }
    /// `op_wot_triger(on)`.
    op_wot_triger @ 0x007e_c7d0 => extern "C" fn op_wot_triger(on: u32) {
        op::bridge::op_wot_triger(&mut mem(), on);
    }
    /// `op_pray_effect_check()`.
    op_pray_effect_check @ 0x007f_ce10 => extern "C" fn op_pray_effect_check() -> i32 {
        op::bridge::op_pray_effect_check(&mem())
    }
    /// `op_pad_set(pad, _, input)`.
    op_pad_set @ 0x007e_6610 => extern "C" fn op_pad_set(pad: u32, unused: u32, input: u32) {
        let _ = unused;
        op::bridge::op_pad_set(&mut mem(), pad, input);
    }
    /// `op_mdc_mem_transfer_cue_free(key)`.
    op_mdc_mem_transfer_cue_free @ 0x0080_3730 => extern "C" fn op_mdc_mem_transfer_cue_free(key: u32) {
        op::bridge::op_mdc_mem_transfer_cue_free(&mut mem(), key);
    }
    /// `op_mdc_mem_transfer_cue_free_all()`.
    op_mdc_mem_transfer_cue_free_all @ 0x0080_3760 => extern "C" fn op_mdc_mem_transfer_cue_free_all() {
        op::bridge::op_mdc_mem_transfer_cue_free_all(&mut mem());
    }
    /// `op_oef2_tex_clut_ovl_clear(mask)`.
    op_oef2_tex_clut_ovl_clear @ 0x007e_4e40 => extern "C" fn op_oef2_tex_clut_ovl_clear(mask: u32) -> u32 {
        op::bridge::op_oef2_tex_clut_ovl_clear(&mut mem(), mask)
    }
    /// `op_oef2_tex_clut_cue_check(mask)`.
    op_oef2_tex_clut_cue_check @ 0x007e_46d0 => extern "C" fn op_oef2_tex_clut_cue_check(mask: u32) -> i32 {
        op::bridge::op_oef2_tex_clut_cue_check(&mem(), mask)
    }
    /// `op_oef2_clut_change(clut, v)`.
    op_oef2_clut_change @ 0x007e_45c0 => extern "C" fn op_oef2_clut_change(clut: u32, v: u32) -> u32 {
        op::bridge::op_oef2_clut_change(&mut mem(), clut, v)
    }
    /// `op_oef2_obj_a_get(base, part, n)`.
    op_oef2_obj_a_get @ 0x007e_3b40 => extern "C" fn op_oef2_obj_a_get(base: u32, part: u32, n: u32) -> u32 {
        op::bridge::op_oef2_obj_a_get(&mem(), base, part, n)
    }
    /// `op_oef2_stream_cue(cue, channel)`.
    op_oef2_stream_cue @ 0x0080_0390 => extern "C" fn op_oef2_stream_cue(cue: u32, channel: u32) -> i32 {
        op::bridge::op_oef2_stream_cue(&mut mem(), cue, channel)
    }

    // ---- et tables ----
    /// `op_get_chr_status_stone(h)`.
    op_get_chr_status_stone @ 0x007f_cc80 => extern "C" fn op_get_chr_status_stone(h: u32) -> u32 {
        et::bridge::op_get_chr_status_stone(&mem(), h)
    }
    /// `op_get_chrhd_by_chrno(no)`.
    op_get_chrhd_by_chrno @ 0x007f_cd10 => extern "C" fn op_get_chrhd_by_chrno(no: u32) -> u32 {
        et::bridge::op_get_chrhd_by_chrno(&mem(), no)
    }
    /// `op_get_chrhd_all_list(out)`.
    op_get_chrhd_all_list @ 0x007f_ccc0 => extern "C" fn op_get_chrhd_all_list(out: u32) -> u32 {
        et::bridge::op_get_chrhd_all_list(&mut mem(), out)
    }
    /// `op_et_battle_genko_counter_get()`.
    op_et_battle_genko_counter_get @ 0x007f_b240 => extern "C" fn op_et_battle_genko_counter_get() -> u32 {
        et::bridge::op_et_battle_genko_counter_get(&mem())
    }
    /// `op_et_bindeff_draw_on(h)`.
    op_et_bindeff_draw_on @ 0x007f_bab0 => extern "C" fn op_et_bindeff_draw_on(h: u32) -> u32 {
        et::bridge::op_et_bindeff_draw_on(&mut mem(), h)
    }
    /// `op_et_bindeff_draw_off(h)`.
    op_et_bindeff_draw_off @ 0x007f_ba40 => extern "C" fn op_et_bindeff_draw_off(h: u32) -> u32 {
        et::bridge::op_et_bindeff_draw_off(&mut mem(), h)
    }
    /// `op_et_bindeff2_draw_on(h)`.
    op_et_bindeff2_draw_on @ 0x007f_b5a0 => extern "C" fn op_et_bindeff2_draw_on(h: u32) -> u32 {
        et::bridge::op_et_bindeff2_draw_on(&mut mem(), h)
    }
    /// `op_et_bindeff2_walk_on(h)`.
    op_et_bindeff2_walk_on @ 0x007f_b830 => extern "C" fn op_et_bindeff2_walk_on(h: u32) {
        et::bridge::op_et_bindeff2_walk_on(&mut mem(), h);
    }
    /// `op_et_bindeff2_water_on(h)`.
    op_et_bindeff2_water_on @ 0x007f_b8e0 => extern "C" fn op_et_bindeff2_water_on(h: u32) {
        et::bridge::op_et_bindeff2_water_on(&mut mem(), h);
    }
    /// `op_et_bindeff2_breath_on(h)`.
    op_et_bindeff2_breath_on @ 0x007f_b4c0 => extern "C" fn op_et_bindeff2_breath_on(h: u32) {
        et::bridge::op_et_bindeff2_breath_on(&mut mem(), h);
    }
    /// `op_et_bindeff_off_signal(h)`.
    op_et_bindeff_off_signal @ 0x007f_bcd0 => extern "C" fn op_et_bindeff_off_signal(h: u32) {
        et::bridge::op_et_bindeff_off_signal(&mut mem(), h);
    }
    /// `Fp_EndGroup(group)`.
    Fp_EndGroup @ 0x0072_94f0 => extern "C" fn fp_end_group(group: u32) {
        fp::bridge::fp_end_group(&mut mem(), group);
    }
    /// `Fp_RestartGroup(group)`.
    Fp_RestartGroup @ 0x0072_9530 => extern "C" fn fp_restart_group(group: u32) {
        fp::bridge::fp_restart_group(&mut mem(), group);
    }
    /// `op_et_bindeff2_breath_off(h)`.
    op_et_bindeff2_breath_off @ 0x007f_b470 => extern "C" fn op_et_bindeff2_breath_off(h: u32) {
        et::bridge::op_et_bindeff2_breath_off(&mut mem(), h);
    }
    /// `op_et_bindeff2_draw_off(h)`.
    op_et_bindeff2_draw_off @ 0x007f_b530 => extern "C" fn op_et_bindeff2_draw_off(h: u32) -> u32 {
        et::bridge::op_et_bindeff2_draw_off(&mut mem(), h)
    }
    /// `op_et_bindeff2_draw_set(h, hide)`.
    op_et_bindeff2_draw_set @ 0x007f_b610 => extern "C" fn op_et_bindeff2_draw_set(h: u32, hide: u32) -> u32 {
        et::bridge::op_et_bindeff2_draw_set(&mut mem(), h, hide)
    }
    /// `op_et_bindeff2_walk_off(h)`.
    op_et_bindeff2_walk_off @ 0x007f_b7f0 => extern "C" fn op_et_bindeff2_walk_off(h: u32) {
        et::bridge::op_et_bindeff2_walk_off(&mut mem(), h);
    }
    /// `op_et_bind_default_water()`; returns the word it computed.
    op_et_bind_default_water @ 0x007f_b870 => extern "C" fn op_et_bind_default_water() -> u32 {
        et::bridge::op_et_bind_default_water(&mut mem())
    }
    /// `op_et_bindeff2_water_off(h)`.
    op_et_bindeff2_water_off @ 0x007f_b890 => extern "C" fn op_et_bindeff2_water_off(h: u32) {
        et::bridge::op_et_bindeff2_water_off(&mut mem(), h);
    }
    /// `op_et_bindeff_draw_set(h, hide)`.
    op_et_bindeff_draw_set @ 0x007f_bb20 => extern "C" fn op_et_bindeff_draw_set(h: u32, hide: u32) -> u32 {
        et::bridge::op_et_bindeff_draw_set(&mut mem(), h, hide)
    }
    /// `op_et_chr_draw_off(h)`.
    op_et_chr_draw_off @ 0x007f_c000 => extern "C" fn op_et_chr_draw_off(h: u32) -> u32 {
        et::bridge::op_et_chr_draw_off(&mut mem(), h)
    }
    /// `op_et_chr_draw_on(h)`.
    op_et_chr_draw_on @ 0x007f_c060 => extern "C" fn op_et_chr_draw_on(h: u32) -> u32 {
        et::bridge::op_et_chr_draw_on(&mut mem(), h)
    }
    /// `op_et_chr_draw_set(h, hide)`.
    op_et_chr_draw_set @ 0x007f_c0c0 => extern "C" fn op_et_chr_draw_set(h: u32, hide: u32) -> u32 {
        et::bridge::op_et_chr_draw_set(&mut mem(), h, hide)
    }
    /// `op_et_draw_flags_switch(h, restore)`.
    op_et_draw_flags_switch @ 0x007f_c200 => extern "C" fn op_et_draw_flags_switch(h: u32, restore: u32) -> u32 {
        et::bridge::op_et_draw_flags_switch(&mut mem(), h, restore)
    }
    /// `op_et_oef2_free(oef2)`.
    op_et_oef2_free @ 0x007f_c2a0 => extern "C" fn op_et_oef2_free(oef2: u32) -> u32 {
        et::bridge::op_et_oef2_free(&mut mem(), oef2, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opu_et_oef2_free(oef2)`.
    opu_et_oef2_free @ 0x007f_da60 => extern "C" fn opu_et_oef2_free(oef2: u32) -> u32 {
        et::bridge::op_et_oef2_free(&mut mem(), oef2, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_et_oef2_start(oef2)`.
    op_et_oef2_start @ 0x007f_c2c0 => extern "C" fn op_et_oef2_start(oef2: u32) -> u32 {
        et::bridge::op_et_oef2_start(&mut mem(), oef2, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_et_oef2_start2(oef2)`: the same body.
    op_et_oef2_start2 @ 0x007f_c970 => extern "C" fn op_et_oef2_start2(oef2: u32) -> u32 {
        et::bridge::op_et_oef2_start(&mut mem(), oef2, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_et_eff_signal_and(id, mask)`.
    op_et_eff_signal_and @ 0x007f_c490 => extern "C" fn op_et_eff_signal_and(id: u32, mask: u32) {
        et::bridge::op_et_eff_signal_and(&mut mem(), id, mask);
    }
    /// `op_et_eff_flags_and(id, mask)`.
    op_et_eff_flags_and @ 0x007f_c4d0 => extern "C" fn op_et_eff_flags_and(id: u32, mask: u32) {
        et::bridge::op_et_eff_flags_and(&mut mem(), id, mask);
    }
    /// `op_et_eff_flags_or(id, bits)`.
    op_et_eff_flags_or @ 0x007f_c510 => extern "C" fn op_et_eff_flags_or(id: u32, bits: u32) {
        et::bridge::op_et_eff_flags_or(&mut mem(), id, bits);
    }
    /// `op_et_eff_signal_or(id, bits)`.
    op_et_eff_signal_or @ 0x007f_c550 => extern "C" fn op_et_eff_signal_or(id: u32, bits: u32) {
        et::bridge::op_et_eff_signal_or(&mut mem(), id, bits);
    }
    /// `op_et_encount_effect_free()`.
    op_et_encount_effect_free @ 0x007f_c5f0 => extern "C" fn op_et_encount_effect_free() -> u32 {
        et::bridge::op_et_encount_effect_free(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_et_monster_add(oef2)`.
    op_et_monster_add @ 0x007f_c8a0 => extern "C" fn op_et_monster_add(oef2: u32) -> u32 {
        et::bridge::op_et_monster_add(&mut mem(), oef2, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opus_chr_draw_index_nth(n)`.
    opus_chr_draw_index_nth @ 0x007f_cf10 => extern "C" fn opus_chr_draw_index_nth(n: u32) -> u32 {
        et::bridge::opus_chr_draw_index_nth(&mut mem(), n, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opus_chr_handle_count()`.
    opus_chr_handle_count @ 0x007f_cf60 => extern "C" fn opus_chr_handle_count() -> u32 {
        et::bridge::opus_chr_handle_count(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `opus_chr_handle_nth(n)`.
    opus_chr_handle_nth @ 0x007f_cfb0 => extern "C" fn opus_chr_handle_nth(n: u32) -> u32 {
        et::bridge::opus_chr_handle_nth(&mut mem(), n, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_et_eff_init_all()`.
    op_et_eff_init_all @ 0x007f_ef70 => extern "C" fn op_et_eff_init_all() {
        et::bridge::op_et_eff_init_all(&mut mem());
    }
    /// `opus_get_weapon_by_no(no)`.
    opus_get_weapon_by_no @ 0x0080_0fd0 => extern "C" fn opus_get_weapon_by_no(no: u32) -> u32 {
        opus::bridge::opus_get_weapon_by_no(&mem(), no)
    }
    /// `opus_get_weapon_index_by_no(no)`.
    opus_get_weapon_index_by_no @ 0x0080_1030 => extern "C" fn opus_get_weapon_index_by_no(no: u32) -> i32 {
        opus::bridge::opus_get_weapon_index_by_no(&mem(), no)
    }
    /// `vfx_vec_w_add(dst, v, s)`.
    vfx_vec_w_add @ 0x0081_7cc0 => extern "C" fn vfx_vec_w_add(dst: u32, v: u32, s: f32) {
        mat::bridge::vfx_vec_w_add(&mut mem(), dst, v, s);
    }
    /// `vfx_vec3_scale(dst, v, s)`.
    vfx_vec3_scale @ 0x0075_bb00 => extern "C" fn vfx_vec3_scale(dst: u32, v: u32, s: f32) {
        mat::bridge::vfx_vec3_scale(&mut mem(), dst, v, s);
    }
    /// `vfx_vec4_min(dst, v, s)`.
    vfx_vec4_min @ 0x0080_8c10 => extern "C" fn vfx_vec4_min(dst: u32, v: u32, s: f32) {
        mat::bridge::vfx_vec4_min(&mut mem(), dst, v, s);
    }
    /// `vfxIntTree_Find(this, key)`.
    vfxIntTree_Find @ 0x0070_fd20 => extern "thiscall" fn vfx_int_tree_find(this: u32, key: u32) -> u32 {
        tree::bridge::vfx_int_tree_find(&mem(), this, key)
    }
    /// `vfxIntTree_Insert(this, node)`.
    vfxIntTree_Insert @ 0x0070_fda0 => extern "thiscall" fn vfx_int_tree_insert(this: u32, node: u32) -> u32 {
        tree::bridge::vfx_int_tree_insert(&mut mem(), this, node)
    }
    /// `opusTree_Next(it)`.
    opusTree_Next @ 0x0080_6a50 => extern "thiscall" fn opus_tree_next(it: u32) -> u32 {
        tree::bridge::opus_tree_next(&mut mem(), it)
    }
    /// `opusTree_Next2(it)`: the same body.
    opusTree_Next2 @ 0x0080_6aa0 => extern "thiscall" fn opus_tree_next2(it: u32) -> u32 {
        tree::bridge::opus_tree_next(&mut mem(), it)
    }
    /// `op_et_bindeff2_off(h)`.
    op_et_bindeff2_off @ 0x007f_b6d0 => extern "C" fn op_et_bindeff2_off(h: u32) -> u32 {
        et::bridge::op_et_bindeff2_off(&mut mem(), h, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opu_et_unit_free_by_id(id)`.
    opu_et_unit_free_by_id @ 0x007f_da40 => extern "C" fn opu_et_unit_free_by_id(id: u32) -> u32 {
        et::bridge::opu_et_unit_free_by_id(&mut mem(), id, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opu_et_unit_ofs(oef2, a, b, c)`.
    opu_et_unit_ofs @ 0x007f_da30 => extern "C" fn opu_et_unit_ofs(o: u32, a: u32, b: u32, c: u32) -> u32 {
        et::bridge::opu_et_unit_ofs(&mut mem(), [o, a, b, c], &mut |_, f, x| call_ptr(f, x))
    }
    /// `pppEePextlh(dst, rs, rt)`.
    pppEePextlh @ 0x0073_7450 => extern "C" fn ppp_ee_pextlh(dst: u32, rs: u32, rt: u32) {
        mat::bridge::ppp_ee_pextlh(&mut mem(), dst, rs, rt);
    }
    /// `pppEePpach(dst, rs, rt)`.
    pppEePpach @ 0x0073_76a0 => extern "C" fn ppp_ee_ppach(dst: u32, rs: u32, rt: u32) {
        mat::bridge::ppp_ee_ppach(&mut mem(), dst, rs, rt);
    }
    /// `pppEePsrlw(dst, src, sa)`.
    pppEePsrlw @ 0x0073_7700 => extern "C" fn ppp_ee_psrlw(dst: u32, src: u32, sa: u32) {
        mat::bridge::ppp_ee_psrlw(&mut mem(), dst, src, sa);
    }
    /// `pppColorShiftLoad(dst, base)`.
    pppColorShiftLoad @ 0x0073_7420 => extern "C" fn ppp_color_shift_load(dst: u32, base: u32) {
        mat::bridge::ppp_color_shift_load(&mut mem(), dst, base);
    }
    /// `pppColorShiftStore(src, base)`.
    pppColorShiftStore @ 0x0073_7750 => extern "C" fn ppp_color_shift_store(src: u32, base: u32) {
        mat::bridge::ppp_color_shift_store(&mut mem(), src, base);
    }
    /// `pppVu0MoveVia(dst, src)`.
    pppVu0MoveVia @ 0x0073_ce30 => extern "C" fn ppp_vu0_move_via(dst: u32, src: u32) {
        mat::bridge::ppp_vu0_move_via(&mut mem(), dst, src);
    }
    /// `pppVu0LoadVf00(dst)`.
    pppVu0LoadVf00 @ 0x0073_ce80 => extern "C" fn ppp_vu0_load_vf00(dst: u32) {
        mat::bridge::ppp_vu0_load_vf00(&mut mem(), dst);
    }
    /// `pppVu0LoadVf00_2(dst)`.
    pppVu0LoadVf00_2 @ 0x0073_e110 => extern "C" fn ppp_vu0_load_vf00_2(dst: u32) {
        mat::bridge::ppp_vu0_load_vf00(&mut mem(), dst);
    }
    /// `pppVu0Length(v)`.
    pppVu0Length @ 0x0073_ceb0 => extern "C" fn ppp_vu0_length(v: u32) -> f32 {
        mat::bridge::ppp_vu0_length(&mut mem(), v)
    }
    /// `pppVu0Length_2(v)`.
    pppVu0Length_2 @ 0x0073_d7c0 => extern "C" fn ppp_vu0_length_2(v: u32) -> f32 {
        mat::bridge::ppp_vu0_length(&mut mem(), v)
    }
    /// `pppVu0Length_3(v)`.
    pppVu0Length_3 @ 0x0073_e140 => extern "C" fn ppp_vu0_length_3(v: u32) -> f32 {
        mat::bridge::ppp_vu0_length(&mut mem(), v)
    }
    /// `pppShapeIterInit(shape, it)`.
    pppShapeIterInit @ 0x0073_e990 => extern "C" fn ppp_shape_iter_init(shape: u32, it: u32) {
        mat::bridge::ppp_shape_iter_init(&mut mem(), shape, it);
    }
    /// `pppShapeIterInit_2(shape, it)`.
    pppShapeIterInit_2 @ 0x0073_f180 => extern "C" fn ppp_shape_iter_init_2(shape: u32, it: u32) {
        mat::bridge::ppp_shape_iter_init(&mut mem(), shape, it);
    }
    /// `pppShapeIterInit_3(shape, it)`.
    pppShapeIterInit_3 @ 0x0073_f9a0 => extern "C" fn ppp_shape_iter_init_3(shape: u32, it: u32) {
        mat::bridge::ppp_shape_iter_init(&mut mem(), shape, it);
    }
    /// `pppShapeRandFlip(def, out)`.
    pppShapeRandFlip @ 0x0074_19e0 => extern "C" fn ppp_shape_rand_flip(def: u32, out: u32) {
        mat::bridge::ppp_shape_rand_flip(&mut mem(), def, out);
    }
    /// `pppDrawMdlSeaDes(obj, ctrl)`.
    pppDrawMdlSeaDes @ 0x0073_ab70 => extern "C" fn ppp_draw_mdl_sea_des(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_mdl_sea_des(&mut mem(), obj, ctrl);
    }
    /// `pppDrawMdlInfDes(obj, ctrl)`.
    pppDrawMdlInfDes @ 0x0073_b830 => extern "C" fn ppp_draw_mdl_inf_des(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_mdl_inf_des(&mut mem(), obj, ctrl);
    }
    /// `pppDrawMdlPSimDes(obj, ctrl)`.
    pppDrawMdlPSimDes @ 0x0073_d010 => extern "C" fn ppp_draw_mdl_psim_des(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_mdl_psim_des(&mut mem(), obj, ctrl, &mut |_, f, a| call_ptr(f, a));
    }
    /// `pppKeBornRnd2(obj, key, ctrl)`.
    pppKeBornRnd2 @ 0x0075_87b0 => extern "C" fn ppp_ke_born_rnd2(obj: u32, key: u32, ctrl: u32) {
        let kind = prog::bridge::BornRnd::Rnd2;
        prog::bridge::ppp_ke_born_rnd(&mut mem(), kind, obj, key, ctrl, &mut |_, f, a| call_ptr(f, a));
    }
    /// `pppKeBornRnd3(obj, key, ctrl)`.
    pppKeBornRnd3 @ 0x0075_8960 => extern "C" fn ppp_ke_born_rnd3(obj: u32, key: u32, ctrl: u32) {
        let kind = prog::bridge::BornRnd::Rnd3;
        prog::bridge::ppp_ke_born_rnd(&mut mem(), kind, obj, key, ctrl, &mut |_, f, a| call_ptr(f, a));
    }
    /// `pppKeBornRnd5(obj, key, ctrl)`.
    pppKeBornRnd5 @ 0x0075_8f20 => extern "C" fn ppp_ke_born_rnd5(obj: u32, key: u32, ctrl: u32) {
        let kind = prog::bridge::BornRnd::Rnd5;
        prog::bridge::ppp_ke_born_rnd(&mut mem(), kind, obj, key, ctrl, &mut |_, f, a| call_ptr(f, a));
    }
    /// `pppKeBornRnd6(obj, key, ctrl)`.
    pppKeBornRnd6 @ 0x0075_9200 => extern "C" fn ppp_ke_born_rnd6(obj: u32, key: u32, ctrl: u32) {
        let kind = prog::bridge::BornRnd::Rnd6;
        prog::bridge::ppp_ke_born_rnd(&mut mem(), kind, obj, key, ctrl, &mut |_, f, a| call_ptr(f, a));
    }
    /// `pppKeBornRnd2_Born(obj, def, shape, out)`.
    pppKeBornRnd2_Born @ 0x0075_8840 => extern "C" fn ppp_ke_born_rnd2_born(obj: u32, def: u32, shape: u32, out: u32) {
        // the original's local vector
        let mut frame = super::StackFrame::<4>::new();
        let at = frame.at();
        prog::bridge::ppp_ke_born_rnd2_born(&mut mem(), obj, def, shape, out, at, &mut |_, f, a| call_ptr(f, a));
    }
    /// `pppKeBornRnd5_Born(obj, def, shape, out)`.
    pppKeBornRnd5_Born @ 0x0075_8fc0 => extern "C" fn ppp_ke_born_rnd5_born(obj: u32, def: u32, shape: u32, out: u32) {
        // the original's local vector
        let mut frame = super::StackFrame::<4>::new();
        let at = frame.at();
        prog::bridge::ppp_ke_born_rnd2_born(&mut mem(), obj, def, shape, out, at, &mut |_, f, a| call_ptr(f, a));
    }
    /// `pppChrSclXYZMatrix(obj, key)`.
    pppChrSclXYZMatrix @ 0x0073_4910 => extern "C" fn ppp_chr_scl_xyz_matrix(obj: u32, key: u32) {
        // the original's local scale matrix
        let mut frame = super::StackFrame::<16>::new();
        let at = frame.at();
        let kind = prog::bridge::ChrScl::Xyz;
        prog::bridge::ppp_chr_scl_matrix(&mut mem(), kind, obj, key, at, &mut |_, f, a| call_ptr(f, a));
    }
    /// `pppChrSclXZMatrix(obj, key)`.
    pppChrSclXZMatrix @ 0x0073_49b0 => extern "C" fn ppp_chr_scl_xz_matrix(obj: u32, key: u32) {
        // the original's local scale matrix
        let mut frame = super::StackFrame::<16>::new();
        let at = frame.at();
        let kind = prog::bridge::ChrScl::Xz;
        prog::bridge::ppp_chr_scl_matrix(&mut mem(), kind, obj, key, at, &mut |_, f, a| call_ptr(f, a));
    }
    /// `pppChrSclYMatrix(obj, key)`.
    pppChrSclYMatrix @ 0x0073_4a30 => extern "C" fn ppp_chr_scl_y_matrix(obj: u32, key: u32) {
        // the original's local scale matrix
        let mut frame = super::StackFrame::<16>::new();
        let at = frame.at();
        let kind = prog::bridge::ChrScl::Y;
        prog::bridge::ppp_chr_scl_matrix(&mut mem(), kind, obj, key, at, &mut |_, f, a| call_ptr(f, a));
    }
    /// `pppChrYSclXYZMatrix(obj)`: it reads no key.
    pppChrYSclXYZMatrix @ 0x0073_4aa0 => extern "C" fn ppp_chr_y_scl_xyz_matrix(obj: u32) {
        // the original's local scale matrix
        let mut frame = super::StackFrame::<16>::new();
        let at = frame.at();
        let kind = prog::bridge::ChrScl::YXyz;
        prog::bridge::ppp_chr_scl_matrix(&mut mem(), kind, obj, 0, at, &mut |_, f, a| call_ptr(f, a));
    }
    /// `pppChrXSclXYZMatrix(obj, key)`.
    pppChrXSclXYZMatrix @ 0x0073_4af0 => extern "C" fn ppp_chr_x_scl_xyz_matrix(obj: u32, key: u32) {
        // the original's local scale matrix
        let mut frame = super::StackFrame::<16>::new();
        let at = frame.at();
        let kind = prog::bridge::ChrScl::XXyz;
        prog::bridge::ppp_chr_scl_matrix(&mut mem(), kind, obj, key, at, &mut |_, f, a| call_ptr(f, a));
    }
    /// `op_oef2_se_on(cue, pan, vol)`.
    op_oef2_se_on @ 0x007f_fe10 => extern "C" fn op_oef2_se_on(cue: u32, pan: u32, vol: u32) -> u32 {
        op::bridge::op_oef2_se_on(&mut mem(), cue, pan, vol, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_oef2_se_volume(cue, vol)`.
    op_oef2_se_volume @ 0x007f_fde0 => extern "C" fn op_oef2_se_volume(cue: u32, vol: u32) -> u32 {
        op::bridge::op_oef2_se_volume(&mut mem(), cue, vol, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_oef2_sound_cue_push(cue, arg)`.
    op_oef2_sound_cue_push @ 0x007f_feb0 => extern "C" fn op_oef2_sound_cue_push(cue: u32, arg: u32) -> u32 {
        op::bridge::op_oef2_sound_cue_push(&mut mem(), cue, arg, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_oef2_sound_cue_multi(oef2, from, to, arg)`.
    op_oef2_sound_cue_multi @ 0x0080_02c0 => extern "C" fn op_oef2_sound_cue_multi(oef2: u32, from: u32, to: u32, arg: u32) -> u32 {
        op::bridge::op_oef2_sound_cue_multi(&mut mem(), oef2, from, to, arg, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_oef2_sound_cue_check()`.
    op_oef2_sound_cue_check @ 0x0080_00f0 => extern "C" fn op_oef2_sound_cue_check() -> u32 {
        op::bridge::op_oef2_sound_cue_check(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_et_bindeff(h)`; returns what the original leaves in `eax`.
    op_et_bindeff @ 0x007f_b290 => extern "C" fn op_et_bindeff(h: u32) -> u32 {
        et::bridge::op_et_bindeff(&mut mem(), h, &mut |_, f, a| call_ptr(f, a)).unwrap_or(0)
    }
    /// `op_et_bindeff_off(h)`.
    op_et_bindeff_off @ 0x007f_bb90 => extern "C" fn op_et_bindeff_off(h: u32) {
        et::bridge::op_et_bindeff_off(&mut mem(), h, &mut |_, f, a| call_ptr(f, a));
    }
    /// `op_et_weapon_unit(h)`.
    op_et_weapon_unit @ 0x007f_c9b0 => extern "C" fn op_et_weapon_unit(h: u32) -> u32 {
        et::bridge::op_et_weapon_unit(&mut mem(), h, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_chara_hsv_size_get(chr)`.
    op_chara_hsv_size_get @ 0x0080_7b20 => extern "C" fn op_chara_hsv_size_get(chr: u32) -> u32 {
        op::bridge::op_chara_hsv_size_get(&mut mem(), chr, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_chara_hsv_make(chr, buf)`.
    op_chara_hsv_make @ 0x0080_79f0 => extern "C" fn op_chara_hsv_make(chr: u32, buf: u32) -> u32 {
        op::bridge::op_chara_hsv_make(&mut mem(), chr, buf, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_chara_hsv(chr, buf, hue, sat, val, mix)`.
    op_chara_hsv @ 0x0080_7870 => extern "C" fn op_chara_hsv(chr: u32, buf: u32, hue: i32, sat: i32, val: i32, mix: i32) -> u32 {
        // the original's local: the shade Ch_CalcLight reads
        let mut frame = super::StackFrame::<1>::new();
        let at = frame.at();
        op::bridge::op_chara_hsv(&mut mem(), chr, buf, [hue, sat, val, mix], at, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_chara_packet_size_get(chr, mode, visible)`.
    op_chara_packet_size_get @ 0x0080_37c0 => extern "C" fn op_chara_packet_size_get(chr: u32, mode: u32, visible: u32) -> u32 {
        op::bridge::op_chara_packet_size_get(&mut mem(), chr, mode, visible, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppKeOfsPt(obj, key, ctrl)`.
    pppKeOfsPt @ 0x0075_cd30 => extern "C" fn ppp_ke_ofs_pt(obj: u32, key: u32, ctrl: u32) {
        prog::bridge::ppp_ke_ofs_pt(&mut mem(), obj, key, ctrl, &mut |_, f, a| call_ptr(f, a));
    }
    /// `pppKeAcmCic(obj, key)`.
    pppKeAcmCic @ 0x0075_b690 => extern "C" fn ppp_ke_acm_cic(obj: u32, key: u32) {
        let _ = obj;
        prog::bridge::ppp_ke_acm_cic(&mut mem(), key, &mut |_, f, a| call_ptr(f, a));
    }
    /// `op_apply_m_w1(dst, m, v)`.
    op_apply_m_w1 @ 0x007e_79a0 => extern "C" fn op_apply_m_w1(dst: u32, m: u32, v: u32) {
        op::bridge::op_apply_m_w1(&mut mem(), dst, m, v);
    }
    /// `op_length_get(a, b)`.
    op_length_get @ 0x007e_7fc0 => extern "C" fn op_length_get(a: u32, b: u32) -> f32 {
        op::bridge::op_length_get(&mem(), a, b)
    }
    /// `op_deg_get(x0, y0, x1, y1, len)`.
    op_deg_get @ 0x007e_7ed0 => extern "C" fn op_deg_get(x0: f32, y0: f32, x1: f32, y1: f32, len: u32) -> f32 {
        op::bridge::op_deg_get(&mut mem(), [x0, y0, x1, y1], len, &mut |_, y, x| crt_atan2(y, x))
    }
    /// `op_3d_vol_pan_get(pos, vol, out)`.
    op_3d_vol_pan_get @ 0x007f_fa90 => extern "C" fn op_3d_vol_pan_get(pos: u32, vol: u32, out: u32) -> u32 {
        op::bridge::op_3d_vol_pan_get(&mut mem(), pos, vol, out, &mut |_, y, x| crt_atan2(y, x))
    }
    /// `op_oef2_se_3d_on(cue, pos, vol)`.
    op_oef2_se_3d_on @ 0x007f_fd20 => extern "C" fn op_oef2_se_3d_on(cue: u32, pos: u32, vol: u32) -> u32 {
        op::bridge::op_oef2_se_3d_on(&mut mem(), cue, pos, vol, &mut |_, f, a| call_ptr(f, a), &mut |_, y, x| crt_atan2(y, x))
    }
    /// `op_oef2_se_3d_move(cue, pos, vol)`.
    op_oef2_se_3d_move @ 0x007f_fc60 => extern "C" fn op_oef2_se_3d_move(cue: u32, pos: u32, vol: u32) -> u32 {
        op::bridge::op_oef2_se_3d_move(&mut mem(), cue, pos, vol, &mut |_, f, a| call_ptr(f, a), &mut |_, y, x| crt_atan2(y, x))
    }
    /// `pppFpGetPacketSize()`.
    pppFpGetPacketSize @ 0x0072_93e0 => extern "C" fn ppp_fp_get_packet_size() -> u32 {
        fp::bridge::ppp_fp_stat(&mem(), 0)
    }
    /// `pppFpGetPacketUsed()`.
    pppFpGetPacketUsed @ 0x0072_93f0 => extern "C" fn ppp_fp_get_packet_used() -> u32 {
        fp::bridge::ppp_fp_stat(&mem(), 1)
    }
    /// `pppFpGetPacketPeak()`.
    pppFpGetPacketPeak @ 0x0072_93d0 => extern "C" fn ppp_fp_get_packet_peak() -> u32 {
        fp::bridge::ppp_fp_stat(&mem(), 2)
    }
    /// `pppFpGetHeapUsed()`.
    pppFpGetHeapUsed @ 0x0072_92d0 => extern "C" fn ppp_fp_get_heap_used() -> u32 {
        fp::bridge::ppp_fp_stat(&mem(), 3)
    }
    /// `pppFpGetHeapMaxUseSize()`.
    pppFpGetHeapMaxUseSize @ 0x0072_9280 => extern "C" fn ppp_fp_get_heap_peak() -> u32 {
        fp::bridge::ppp_fp_stat(&mem(), 4)
    }
    /// `MsSet01130838(v)`; returns `v` (left in `eax`).
    MsSet01130838 @ 0x007b_2b40 => extern "C" fn ms_set01130838(v: u32) -> u32 {
        op::bridge::set_word(&mut mem(), 0, v);
        v
    }
    /// `MsSetVoiceId(v)`; returns `v` (left in `eax`).
    MsSetVoiceId @ 0x007b_2b50 => extern "C" fn ms_set_voice_id(v: u32) -> u32 {
        op::bridge::set_word(&mut mem(), 1, v);
        v
    }
    /// `MsSetVoiceWait(v)`; returns `v` (left in `eax`).
    MsSetVoiceWait @ 0x007b_2b60 => extern "C" fn ms_set_voice_wait(v: u32) -> u32 {
        op::bridge::set_word(&mut mem(), 2, v);
        v
    }
    /// `op_tex_vdisp_load_triger(on)`.
    op_tex_vdisp_load_triger @ 0x007e_5210 => extern "C" fn op_tex_vdisp_load_triger(on: u32) {
        op::bridge::op_tex_vdisp_load_triger(&mut mem(), on);
    }
    /// `op_word_copy(dst, src)`.
    op_word_copy @ 0x0075_e4b0 => extern "C" fn op_word_copy(dst: u32, src: u32) {
        op::bridge::op_word_copy(&mut mem(), dst, src);
    }
    /// `op_word_copy2(dst, src)`: the same body.
    op_word_copy2 @ 0x007e_6c60 => extern "C" fn op_word_copy2(dst: u32, src: u32) {
        op::bridge::op_word_copy(&mut mem(), dst, src);
    }
    /// `op_vec_w_scale(dst, v, s)`.
    op_vec_w_scale @ 0x007e_cec0 => extern "C" fn op_vec_w_scale(dst: u32, v: u32, s: f32) {
        op::bridge::op_vec_w_scale(&mut mem(), dst, v, s);
    }
    /// `op_add64(dst, src, n)`.
    op_add64 @ 0x0081_7c30 => extern "C" fn op_add64(dst: u32, src: u32, n: u32) {
        op::bridge::op_add64(&mut mem(), dst, src, n);
    }
    /// `op_sum_words(p, n)`.
    op_sum_words @ 0x0076_b1e0 => extern "C" fn op_sum_words(p: u32, n: u32) -> u32 {
        op::bridge::op_sum_words(&mem(), p, n)
    }
    /// `opus_chr_handles_clear()`.
    opus_chr_handles_clear @ 0x007f_edf0 => extern "C" fn opus_chr_handles_clear() {
        et::bridge::opus_chr_handles_clear(&mut mem());
    }
    /// `opus_monster_index(obj)`.
    opus_monster_index @ 0x007f_e540 => extern "C" fn opus_monster_index(obj: u32) -> i32 {
        opus::bridge::opus_monster_index(&mem(), obj)
    }
    /// `opus_weapon_index(obj)`: the `opus_get_weapon_index` body.
    opus_weapon_index @ 0x007f_e570 => extern "C" fn opus_weapon_index(obj: u32) -> i32 {
        opus::bridge::opus_get_weapon_index(&mem(), obj)
    }
    /// `opusTree_Min(node)`.
    opusTree_Min @ 0x0080_6db0 => extern "C" fn opus_tree_min(node: u32) -> u32 {
        tree::bridge::opus_tree_min(&mem(), node)
    }
    /// `opusTree_Max(node)`.
    opusTree_Max @ 0x0080_6d70 => extern "C" fn opus_tree_max(node: u32) -> u32 {
        tree::bridge::opus_tree_max(&mem(), node)
    }
    /// `pppFpGroupDispOff(group)`.
    pppFpGroupDispOff @ 0x0072_9400 => extern "C" fn ppp_fp_group_disp_off(group: u32) {
        fp::bridge::ppp_fp_group_disp_off(&mut mem(), group);
    }
    /// `pppFpGroupDispOn(group)`.
    pppFpGroupDispOn @ 0x0072_9440 => extern "C" fn ppp_fp_group_disp_on(group: u32) {
        fp::bridge::ppp_fp_group_disp_on(&mut mem(), group);
    }
    /// `vfx_vec3_scale3(dst, v, sx, sy, sz)`.
    vfx_vec3_scale3 @ 0x0072_8a60 => extern "C" fn vfx_vec3_scale3(dst: u32, v: u32, sx: f32, sy: f32, sz: f32) {
        op::bridge::vfx_vec3_scale3(&mut mem(), dst, v, [sx, sy, sz]);
    }
    /// `pppVec4Copy(dst, src)`.
    pppVec4Copy @ 0x0075_0020 => extern "C" fn ppp_vec4_copy(dst: u32, src: u32) {
        op::bridge::ppp_vec4_copy(&mut mem(), dst, src);
    }
    /// `op_color_shift_load16(dst, base)`.
    op_color_shift_load16 @ 0x007e_ca60 => extern "C" fn op_color_shift_load16(dst: u32, base: u32) {
        op::bridge::op_color_shift_load16(&mut mem(), dst, base);
    }
    /// `op_oef2_clut_change_monster(rec, pos)`.
    op_oef2_clut_change_monster @ 0x007e_45e0 => extern "C" fn op_oef2_clut_change_monster(rec: u32, pos: u32) -> u32 {
        op::bridge::op_oef2_clut_change_monster(&mut mem(), rec, pos)
    }
    /// `op_oef2_tex_pos_set(rec, pos)`.
    op_oef2_tex_pos_set @ 0x007e_4ff0 => extern "C" fn op_oef2_tex_pos_set(rec: u32, pos: u32) -> u32 {
        op::bridge::op_oef2_tex_pos_set(&mut mem(), rec, pos)
    }
    /// `opus_vec3_scale(dst, v, _, _, _, s)`.
    opus_vec3_scale @ 0x0080_8cb0 => extern "C" fn opus_vec3_scale(dst: u32, v: u32, a: u32, b: u32, c: u32, s: f32) {
        let _ = (a, b, c);
        op::bridge::opus_vec3_scale(&mut mem(), dst, v, s);
    }
    /// `opus_monster_clut_pos(slot)`.
    opus_monster_clut_pos @ 0x0080_0e20 => extern "C" fn opus_monster_clut_pos(slot: u32) -> u32 {
        opus::bridge::opus_monster_clut_pos(&mem(), slot)
    }
    /// `opus_monster_tex_pos(slot)`.
    opus_monster_tex_pos @ 0x0080_0e50 => extern "C" fn opus_monster_tex_pos(slot: u32) -> u32 {
        opus::bridge::opus_monster_tex_pos(&mem(), slot)
    }
    /// `opus_osu_init_group(i, n)`.
    opus_osu_init_group @ 0x0080_1480 => extern "C" fn opus_osu_init_group(i: u32, n: u32) {
        opus::bridge::opus_osu_init_group(&mut mem(), i, n);
    }
    /// `op_oef2_sound_wait()`.
    op_oef2_sound_wait @ 0x0080_0330 => extern "C" fn op_oef2_sound_wait() -> u32 {
        op::bridge::op_oef2_sound_wait(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_et_flag1_on()`.
    op_et_flag1_on @ 0x007f_c990 => extern "C" fn op_et_flag1_on() {
        slots::bridge::op_et_flag(&mut mem(), 1, true);
    }
    /// `op_et_flag1_off()`.
    op_et_flag1_off @ 0x007f_c9a0 => extern "C" fn op_et_flag1_off() {
        slots::bridge::op_et_flag(&mut mem(), 1, false);
    }
    /// `op_et_flag2_on()`.
    op_et_flag2_on @ 0x007f_c900 => extern "C" fn op_et_flag2_on() {
        slots::bridge::op_et_flag(&mut mem(), 2, true);
    }
    /// `op_et_flag2_off()`.
    op_et_flag2_off @ 0x007f_c910 => extern "C" fn op_et_flag2_off() {
        slots::bridge::op_et_flag(&mut mem(), 2, false);
    }
    /// `TOBtlLibraSetState6()`.
    TOBtlLibraSetState6 @ 0x0089_0ec0 => extern "C" fn to_btl_libra_set_state6() -> u32 {
        slots::bridge::to_btl_libra_set_state6(&mut mem())
    }
    /// `TOBtlLibraGetState()`.
    TOBtlLibraGetState @ 0x0089_7770 => extern "C" fn to_btl_libra_get_state() -> u32 {
        slots::bridge::to_btl_libra_get_state(&mem())
    }
    /// `rcefGetFloat25d5ea4()`.
    rcefGetFloat25d5ea4 @ 0x0092_49a0 => extern "C" fn rcef_get_float_25d5ea4() -> f32 {
        slots::bridge::get_float(&mem(), 0x025d_5ea4)
    }
    /// `TOGetFloat133f790()`.
    TOGetFloat133f790 @ 0x0089_7780 => extern "C" fn to_get_float_133f790() -> f32 {
        slots::bridge::get_float(&mem(), 0x0133_f790)
    }
    /// `MsGetBattle112be90()`.
    MsGetBattle112be90 @ 0x007b_7f20 => extern "C" fn ms_get_battle_112be90() -> u32 {
        slots::bridge::ms_get_battle_112be90(&mem())
    }
    /// `rcefReverseList()`.
    rcefReverseList @ 0x0092_e7a0 => extern "C" fn rcef_reverse_list()  {
        slots::bridge::rcef_reverse_list(&mut mem());
    }
    /// `rcefWorkCall_00925530()`.
    rcefWorkCall_00925530 @ 0x0092_e770 => extern "C" fn rcef_work_call_00925530() -> u32 {
        slots::bridge::rcef_work_call(&mut mem(), 0x0092_5530, &mut |_, f, a| call_ptr(f, a))
    }
    /// `rcefWorkCall_00924c70()`.
    rcefWorkCall_00924c70 @ 0x0092_5130 => extern "C" fn rcef_work_call_00924c70() -> u32 {
        slots::bridge::rcef_work_call(&mut mem(), 0x0092_4c70, &mut |_, f, a| call_ptr(f, a))
    }
    /// `rcefEffectEndMotion()`.
    rcefEffectEndMotion @ 0x0092_df90 => extern "C" fn rcef_effect_end_motion() -> u32 {
        slots::bridge::rcef_effect_end_motion(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `rcefChrAlloc()`.
    rcefChrAlloc @ 0x0093_0800 => extern "C" fn rcef_chr_alloc() -> u32 {
        slots::bridge::rcef_chr_alloc(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `msVoiceCheckStanbyOn()`.
    msVoiceCheckStanbyOn @ 0x007b_2b70 => extern "C" fn ms_voice_check_stanby_on() -> u32 {
        slots::bridge::ms_voice_check_stanby_on(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `msVoiceCheckStanbyOff()`.
    msVoiceCheckStanbyOff @ 0x007b_2ba0 => extern "C" fn ms_voice_check_stanby_off() -> u32 {
        slots::bridge::ms_voice_check_stanby_off(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `msVoiceIsStreaming()`.
    msVoiceIsStreaming @ 0x007b_2be0 => extern "C" fn ms_voice_is_streaming() -> u32 {
        slots::bridge::ms_voice_is_streaming(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `TOSepStop4e()`.
    TOSepStop4e @ 0x0089_0e70 => extern "C" fn to_sep_stop_4e() -> u32 {
        slots::bridge::to_sep_stop_4e(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `TOBwakkStop()`.
    TOBwakkStop @ 0x0089_0f10 => extern "C" fn to_bwakk_stop() -> u32 {
        slots::bridge::to_bwakk_stop(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `TOBwakkCheckTerm()`.
    TOBwakkCheckTerm @ 0x0089_77c0 => extern "C" fn to_bwakk_check_term() -> u32 {
        slots::bridge::to_bwakk_check_term(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `TOBgoodCheckTerm()`.
    TOBgoodCheckTerm @ 0x0089_77e0 => extern "C" fn to_bgood_check_term() -> u32 {
        slots::bridge::to_bgood_check_term(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `TODum225GetNum()`.
    TODum225GetNum @ 0x0089_8230 => extern "C" fn to_dum225_get_num() -> u32 {
        slots::bridge::to_dum225_get_num(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `TOBwakkStart()`.
    TOBwakkStart @ 0x0089_8dc0 => extern "C" fn to_bwakk_start() -> u32 {
        slots::bridge::to_bwakk_start(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `TOBgoodStart()`.
    TOBgoodStart @ 0x0089_ab80 => extern "C" fn to_bgood_start() -> u32 {
        slots::bridge::to_bgood_start(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `opu_debug_reload()`.
    opu_debug_reload @ 0x007f_dc80 => extern "C" fn opu_debug_reload() {
        slots::bridge::opu_debug_reload(&mut mem(), &mut |_, f, a| call_ptr(f, a));
    }
    /// `opp_draw_part_a74d30(obj, cmd)`: its stack locals in a 16-aligned
    /// buffer on this shim's stack (the frame pointer at its end).
    opp_draw_part_a74d30 @ 0x00a7_4d30 => extern "C" fn opp_draw_part_a74d30(obj: u32, cmd: u32) -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0xa0]);
        let mut locals = Locals([0; 0xa0]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0xa0;
        slots::bridge::opp_draw_part(&mut mem(), obj, cmd, frame, &mut |_, f, a| call_ptr(f, a))
    }
    /// `TOBtlLibraGetRects()`.
    TOBtlLibraGetRects @ 0x0089_7dc0 => extern "C" fn to_btl_libra_get_rects() -> u32 {
        slots::bridge::to_btl_libra_get_rects(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `opp_part_a76720(obj, cmd)`.
    opp_part_a76720 @ 0x00a7_6720 => extern "C" fn opp_part_a76720(obj: u32, cmd: u32) -> u32 {
        slots::bridge::opp_part_a76720(&mut mem(), obj, cmd, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opp_map_draw_a74c60(obj, cmd)`.
    opp_map_draw_a74c60 @ 0x00a7_4c60 => extern "C" fn opp_map_draw_a74c60(obj: u32, cmd: u32) -> u32 {
        slots::bridge::opp_map_draw_a74c60(&mut mem(), obj, cmd, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opp_md_break_init_a74410(obj, cmd)`.
    opp_md_break_init_a74410 @ 0x00a7_4410 => extern "C" fn opp_md_break_init_a74410(obj: u32, cmd: u32) -> u32 {
        slots::bridge::opp_md_break_init_a74410(&mut mem(), obj, cmd, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opus_wave_cmd_7f5f70(obj, cmd)`: its stack locals in a buffer on
    /// this shim's stack (the frame pointer at its end).
    opus_wave_cmd_7f5f70 @ 0x007f_5f70 => extern "C" fn opus_wave_cmd_7f5f70(obj: u32, cmd: u32) -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0x40]);
        let mut locals = Locals([0; 0x40]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0x40;
        slots::bridge::opus_wave_cmd_7f5f70(&mut mem(), obj, cmd, frame, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opp_chr_center_a756e0(obj, cmd)`: its stack locals in a buffer on
    /// this shim's stack (the frame pointer at its end).
    opp_chr_center_a756e0 @ 0x00a7_56e0 => extern "C" fn opp_chr_center_a756e0(obj: u32, cmd: u32) -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0x60]);
        let mut locals = Locals([0; 0x60]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0x60;
        slots::bridge::opp_chr_center_a756e0(&mut mem(), obj, cmd, frame, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opp_chr_fade_center_a76b60(obj, cmd)`: its stack locals in a
    /// buffer on this shim's stack (the frame pointer at its end).
    opp_chr_fade_center_a76b60 @ 0x00a7_6b60 => extern "C" fn opp_chr_fade_center_a76b60(obj: u32, cmd: u32) -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0x1f0]);
        let mut locals = Locals([0; 0x1f0]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0x1f0;
        slots::bridge::opp_chr_fade_center_a76b60(&mut mem(), obj, cmd, frame, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opp2_optic_laser(obj, list)`: its stack locals in a buffer on
    /// this shim's stack (the frame pointer at its end).
    opp2_optic_laser @ 0x00a7_3810 => extern "C" fn opp2_optic_laser(obj: u32, list: u32) -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0x30]);
        let mut locals = Locals([0; 0x30]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0x30;
        slots::bridge::opp2_optic_laser(&mut mem(), obj, list, frame, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opp2_md_break(obj, list)`: its stack locals in a buffer on this
    /// shim's stack (the frame pointer at its end).
    opp2_md_break @ 0x00a7_3d90 => extern "C" fn opp2_md_break(obj: u32, list: u32) -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0x90]);
        let mut locals = Locals([0; 0x90]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0x90;
        slots::bridge::opp2_md_break(&mut mem(), obj, list, frame, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opp_0582_cmd_a70910(obj, cmd)`.
    opp_0582_cmd_a70910 @ 0x00a7_0910 => extern "C" fn opp_0582_cmd_a70910(obj: u32, cmd: u32) -> u32 {
        slots::bridge::opp_0582_cmd_a70910(&mut mem(), obj, cmd, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opp_0437_cmd_a6dbe0(obj, cmd)`: its stack locals in a buffer on
    /// this shim's stack (the frame pointer at its end).
    opp_0437_cmd_a6dbe0 @ 0x00a6_dbe0 => extern "C" fn opp_0437_cmd_a6dbe0(obj: u32, cmd: u32) -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0xf0]);
        let mut locals = Locals([0; 0xf0]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0xf0;
        slots::bridge::opp_0437_cmd_a6dbe0(&mut mem(), obj, cmd, frame, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opp_mdc_draw_a6f4d0(obj, cmd)`: its stack locals in a buffer on
    /// this shim's stack (the frame pointer at its end).
    opp_mdc_draw_a6f4d0 @ 0x00a6_f4d0 => extern "C" fn opp_mdc_draw_a6f4d0(obj: u32, cmd: u32) -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0x3b0]);
        let mut locals = Locals([0; 0x3b0]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0x3b0;
        slots::bridge::opp_mdc_draw_a6f4d0(&mut mem(), obj, cmd, frame, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opus_0582_dot_draw(b, rec, m, hold)`: its stack locals in a
    /// buffer on this shim's stack (the frame pointer at its end).
    opus_0582_dot_draw @ 0x00a7_2b80 => extern "C" fn opus_0582_dot_draw(b: u32, rec: u32, m: u32, hold: u32) {
        #[repr(C, align(16))]
        struct Locals([u8; 0x3a0]);
        let mut locals = Locals([0; 0x3a0]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0x3a0;
        slots::bridge::opus_0582_dot_draw(&mut mem(), b, rec, m, hold, frame, &mut |_, f, a| call_ptr(f, a));
    }
    /// `opus_no_get(list, unit, no)`.
    opus_no_get @ 0x0080_1060 => extern "C" fn opus_no_get(list: u32, unit: u32, no: u32) -> u32 {
        opus::bridge::opus_no_get(&mem(), list, unit, no)
    }
    /// `op_mul_m(dst, b, c)`.
    op_mul_m @ 0x007e_83b0 => extern "C" fn op_mul_m(dst: u32, b: u32, c: u32) {
        op::bridge::op_mul_m(&mut mem(), dst, b, c);
    }
    /// `Vec3AddScalars_00735100(dst, src, x, y, z, w)`.
    Vec3AddScalars_00735100 @ 0x0073_5100 => extern "C" fn vec3_add_scalars(dst: u32, src: u32, x: u32, y: u32, z: u32) -> u32 {
        let k = [x, y, z].map(f32::from_bits);
        vec::bridge::vec3_add_scalars(&mut mem(), dst, src, k)
    }
    /// `Vec3SubScalars_0074b670(dst, src, x, y, z)`.
    Vec3SubScalars_0074b670 @ 0x0074_b670 => extern "C" fn vec3_sub_scalars(dst: u32, src: u32, x: u32, y: u32, z: u32) {
        let k = [x, y, z].map(f32::from_bits);
        vec::bridge::vec3_sub_scalars(&mut mem(), dst, src, k);
    }
    /// `Vec2SubScalarsFlags_0075e870(dst, src, x, y)`.
    Vec2SubScalarsFlags_0075e870 @ 0x0075_e870 => extern "C" fn vec2_sub_scalars_flags(dst: u32, src: u32, x: u32, y: u32) {
        vec::bridge::vec2_sub_scalars_flags(&mut mem(), dst, src, f32::from_bits(x), f32::from_bits(y));
    }
    /// `opus_get_chr_status_index(chr, no, add)`.
    opus_get_chr_status_index @ 0x007f_e970 => extern "C" fn opus_get_chr_status_index(chr: u32, no: u32, add: u32) -> i32 {
        et::bridge::opus_get_chr_status_index(&mut mem(), chr, no, add, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_xyz_scale(dst, src, s)`.
    op_xyz_scale @ 0x007e_a850 => extern "C" fn op_xyz_scale(dst: u32, src: u32, s: u32) -> u32 {
        op::bridge::op_xyz_scale(&mut mem(), dst, src, s)
    }
    /// `op_heap_display_clear(flags)`.
    op_heap_display_clear @ 0x007f_f260 => extern "C" fn op_heap_display_clear(flags: u32) {
        op::bridge::op_heap_display_clear(&mut mem(), flags);
    }
    /// `op_oef2_sep_free_all(mask)`.
    op_oef2_sep_free_all @ 0x007f_ff30 => extern "C" fn op_oef2_sep_free_all(mask: u32) -> u32 {
        op::bridge::op_oef2_sep_free_all(&mut mem(), mask, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppFpSetStickChr(id, chr, bone)`.
    pppFpSetStickChr @ 0x0072_a530 => extern "C" fn ppp_fp_set_stick_chr(id: u32, chr: u32, bone: u32) -> u32 {
        fp::bridge::ppp_fp_set_stick_chr(&mut mem(), id, chr, bone, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppFpSetStickPos(id, pos)`.
    pppFpSetStickPos @ 0x0072_a630 => extern "C" fn ppp_fp_set_stick_pos(id: u32, pos: u32) {
        fp::bridge::ppp_fp_set_stick_pos(&mut mem(), id, pos, &mut |_, f, a| call_ptr(f, a));
    }
    /// `opus_parent_get(obj, unit)`.
    opus_parent_get @ 0x0080_1f00 => extern "C" fn opus_parent_get(obj: u32, unit: u32) -> u32 {
        opus::bridge::opus_parent_get(&mem(), obj, unit)
    }
    /// `op_free(heap, p)`.
    op_free @ 0x007f_f1d0 => extern "C" fn op_free(heap: u32, p: u32) {
        op::bridge::op_free(&mut mem(), heap, p, &mut |_, f, a| call_ptr(f, a));
    }
    /// `op_rot_m(dst, src, angles)`.
    op_rot_m @ 0x007e_9530 => extern "C" fn op_rot_m(dst: u32, src: u32, angles: u32) {
        op::bridge::op_rot_m(&mut mem(), dst, src, angles);
    }
    /// `op_rt_m(dst, src, angles, t)`.
    op_rt_m @ 0x007e_9850 => extern "C" fn op_rt_m(dst: u32, src: u32, angles: u32, t: u32) {
        op::bridge::op_rt_m_zyx(&mut mem(), dst, src, angles, t);
    }
    /// `opus_prog_off(list, no)`.
    opus_prog_off @ 0x0080_14c0 => extern "C" fn opus_prog_off(list: u32, no: u32) {
        op::bridge::opus_prog_off(&mut mem(), list, no, &mut |_, f, a| call_ptr(f, a));
    }
    /// `op_irandom_core_007e35c0(n, b, c, d)`.
    op_irandom_core_007e35c0 @ 0x007e_35c0 => extern "C" fn op_irandom_core(n: u32, b: u32, c: u32, d: u32) -> u32 {
        rand::bridge::op_irandom_core(&mut mem(), [n, b, c, d])
    }
    /// `op_random_per(n, b, c, d)`.
    op_random_per @ 0x007e_33e0 => extern "C" fn op_random_per(n: u32, b: u32, c: u32, d: u32) -> u32 {
        rand::bridge::op_random_per(&mut mem(), [n, b, c, d])
    }
    /// `opu_unit_free_by_id(list, id)`.
    opu_unit_free_by_id @ 0x007f_de30 => extern "C" fn opu_unit_free_by_id(list: u32, id: u32) {
        et::bridge::opu_unit_free_by_id(&mut mem(), list, id, &mut |_, f, a| call_ptr(f, a));
    }
    /// `opu_unit_free_by_oef2(list, oef)`.
    opu_unit_free_by_oef2 @ 0x007f_df70 => extern "C" fn opu_unit_free_by_oef2(list: u32, oef: u32) {
        et::bridge::opu_unit_free_by_oef2(&mut mem(), list, oef, &mut |_, f, a| call_ptr(f, a));
    }
    /// `op_oef2_a_set(oef, kind, i, v)`.
    op_oef2_a_set @ 0x007e_38a0 => extern "C" fn op_oef2_a_set(oef: u32, kind: u32, i: u32, v: u32) -> u32 {
        op::bridge::op_oef2_a_set(&mut mem(), oef, kind, i, v, &mut |_, f, a| call_ptr(f, a))
    }
    /// `Vec4Max_00808b70(dst, src, x, y, z, w)`.
    Vec4Max_00808b70 @ 0x0080_8b70 => extern "C" fn vec4_max(dst: u32, src: u32, x: u32, y: u32, z: u32, w: u32) {
        vec::bridge::vec4_max(&mut mem(), dst, src, [x, y, z, w]);
    }
    /// `op_random_per_call_007e5d40(n, b, c, d)`: a jump to `op_random_per`.
    op_random_per_call_007e5d40 @ 0x007e_5d40 => extern "C" fn op_random_per_call(n: u32, b: u32, c: u32, d: u32) -> u32 {
        rand::bridge::op_random_per(&mut mem(), [n, b, c, d])
    }
    /// `op_irandom_p(n, b, c, d)`: a jump to `op_irandom_core_007e35c0`.
    op_irandom_p @ 0x007e_5d50 => extern "C" fn op_irandom_p(n: u32, b: u32, c: u32, d: u32) -> u32 {
        rand::bridge::op_irandom_core(&mut mem(), [n, b, c, d])
    }
    // ---- round 33 a ----
    /// `ClassDynamicGeometry_setupDynMesh(this)`.
    ClassDynamicGeometry_setupDynMesh @ 0x006a_3b30 => extern "thiscall" fn class_dynamic_geometry_setup_dyn_mesh(this: u32) -> u32 {
        small::bridge::setup_dyn_mesh(&mut mem(), this, &mut |_, f, this, a: &[u32]| {
            // SAFETY: the engine's thiscall function of one stack word (`ret 4`).
            unsafe {
                let g: unsafe extern "thiscall" fn(u32, u32) -> u32 = std::mem::transmute(f as usize);
                g(this, a.first().copied().unwrap_or(0))
            }
        })
    }
    /// `vfxIntTree_InsertFinalize_0070fd70(this, node)`.
    vfxIntTree_InsertFinalize_0070fd70 @ 0x0070_fd70 => extern "thiscall" fn vfx_int_tree_insert_finalize(this: u32, node: u32) -> u32 {
        tree::bridge::vfx_int_tree_insert_finalize(&mut mem(), this, node, &mut |_, f, this, a: &[u32]| {
            // SAFETY: the engine's thiscall function of one stack word (`ret 4`).
            unsafe {
                let g: unsafe extern "thiscall" fn(u32, u32) -> u32 = std::mem::transmute(f as usize);
                g(this, a.first().copied().unwrap_or(0))
            }
        })
    }
    /// `pppKeBornPtCmpl(pc, pd, prog)`.
    pppKeBornPtCmpl @ 0x0075_95e0 => extern "C" fn ppp_ke_born_pt_cmpl(pc: u32, pd: u32, prog: u32) {
        // the original's one-word local
        let mut frame = super::StackFrame::<1>::new();
        let at = frame.at();
        prog::bridge::ppp_ke_born_pt_cmpl(&mut mem(), pc, pd, prog, at, &mut |_, f, a| call_ptr(f, a));
    }
    /// `opus_view_screen_matrix_00801ea0(m)`.
    opus_view_screen_matrix_00801ea0 @ 0x0080_1ea0 => extern "C" fn opus_view_screen_matrix(m: u32) -> u32 {
        small::bridge::opus_view_screen_matrix(&mut mem(), m, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opus_colour_scale2_0080a470(r, g, b, a)`.
    opus_colour_scale2_0080a470 @ 0x0080_a470 => extern "C" fn opus_colour_scale2(r: u32, g: u32, b: u32, a: u32) -> u32 {
        small::bridge::opus_colour_scale2(&mut mem(), [r, g, b, a], &mut |_, f, a| call_ptr(f, a))
    }
    /// `opu_prog_clear_0080adc0(host, cmd)`.
    opu_prog_clear_0080adc0 @ 0x0080_adc0 => extern "C" fn opu_prog_clear(host: u32, cmd: u32) -> u32 {
        opus::bridge::opu_prog_clear(&mut mem(), host, cmd)
    }
    /// `opu_end_no_0080c650(host, cmd)`.
    opu_end_no_0080c650 @ 0x0080_c650 => extern "C" fn opu_end_no(host: u32, cmd: u32) -> u32 {
        opus::bridge::opu_end_no(&mut mem(), host, cmd, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opp_anmm_frame_random_00810dd0(host, set, cmd, work)`.
    opp_anmm_frame_random_00810dd0 @ 0x0081_0dd0 => extern "C" fn opp_anmm_frame_random(host: u32, set: u32, cmd: u32, work: u32) -> u32 {
        let _ = (host, set);
        anmm::bridge::frame_random(&mut mem(), cmd, work, [0; 3])
    }
    /// `opp_anmm_scale_y_00810ff0(host, set, cmd, work)`.
    opp_anmm_scale_y_00810ff0 @ 0x0081_0ff0 => extern "C" fn opp_anmm_scale_y(host: u32, set: u32, cmd: u32, work: u32) -> u32 {
        let _ = set;
        anmm::bridge::scale_y(&mut mem(), host, cmd, work, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opp_anmm_width_scale_008110d0(host, set, cmd, work)`.
    opp_anmm_width_scale_008110d0 @ 0x0081_10d0 => extern "C" fn opp_anmm_width_scale(host: u32, set: u32, cmd: u32, work: u32) -> u32 {
        anmm::bridge::width_scale(&mut mem(), host, set, cmd, work, &mut |_, f, a| call_ptr_f64(f, a))
    }
    /// `opp_et_ovl(host)`.
    opp_et_ovl @ 0x0081_3460 => extern "C" fn opp_et_ovl(host: u32) -> u32 {
        opus::bridge::opp_et_ovl(&mem(), host)
    }
    /// `opu_vibration(host, cmd)`.
    opu_vibration @ 0x0081_43e0 => extern "C" fn opu_vibration(host: u32, cmd: u32) -> u32 {
        opus::bridge::opu_vibration(&mut mem(), host, cmd, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opu_stone_00814440(host, cmd)`.
    opu_stone_00814440 @ 0x0081_4440 => extern "C" fn opu_stone(host: u32, cmd: u32) -> u32 {
        opus::bridge::opu_stone(&mut mem(), host, cmd, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opu_chr_switch_008144b0(host, cmd)`.
    opu_chr_switch_008144b0 @ 0x0081_44b0 => extern "C" fn opu_chr_switch(host: u32, cmd: u32) -> u32 {
        opus::bridge::opu_chr_switch(&mut mem(), host, cmd, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opu_kind8_00815710(host, cmd)`.
    opu_kind8_00815710 @ 0x0081_5710 => extern "C" fn opu_kind8(host: u32, cmd: u32) -> u32 {
        opus::bridge::opu_kind8(&mut mem(), host, cmd)
    }
    /// `opu_call_00815760(host, cmd)`.
    opu_call_00815760 @ 0x0081_5760 => extern "C" fn opu_call(host: u32, cmd: u32) -> u32 {
        opus::bridge::opu_call(&mut mem(), host, cmd)
    }
    /// `opu_return_00817280(host)`.
    opu_return_00817280 @ 0x0081_7280 => extern "C" fn opu_return(host: u32) -> u32 {
        opus::bridge::opu_return(&mem(), host)
    }
    /// `opu_signal(host, cmd)`.
    opu_signal @ 0x0081_72b0 => extern "C" fn opu_signal(host: u32, cmd: u32) -> u32 {
        opus::bridge::opu_signal(&mut mem(), host, cmd, &mut |_, f, a| call_ptr(f, a))
    }
    // ---- round 33 b ----
    /// `graphicVFXAddScreenFadeEvent(r, g, b, a, alpha)`.
    graphicVFXAddScreenFadeEvent @ 0x0064_4d40 => extern "C" fn graphic_vfx_add_screen_fade_event(r: u32, g: u32, b: u32, a: u32, alpha: u32) -> u32 {
        // the original's event local
        let mut frame = super::StackFrame::<4>::new();
        let at = frame.at();
        small::bridge::graphic_vfx_add_screen_fade_event(&mut mem(), [r, g, b, a], alpha, at, &mut |_, f, this, a: &[u32]| {
            // SAFETY: the engine's thiscall function of one stack word (`ret 4`).
            unsafe {
                let g: unsafe extern "thiscall" fn(u32, u32) -> u32 = std::mem::transmute(f as usize);
                g(this, a.first().copied().unwrap_or(0))
            }
        })
    }
    /// `pppNoiseBlurDraw_007116d0(a0..a6)`.
    pppNoiseBlurDraw_007116d0 @ 0x0071_16d0 => extern "C" fn ppp_noise_blur_draw(a0: u32, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32) {
        small::bridge::ppp_noise_blur_draw(&mut mem(), [a0, a1, a2, a3, a4, a5, a6], &mut |_, f, a| call_ptr(f, a));
    }
    /// `pppAkazaScrFltDraw_00711750(lo, hi, a, b)`.
    pppAkazaScrFltDraw_00711750 @ 0x0071_1750 => extern "C" fn ppp_akaza_scr_flt_draw(lo: u32, hi: u32, a: u32, b: u32) {
        small::bridge::ppp_akaza_scr_flt_draw(&mut mem(), lo, hi, a, b, &mut |_, f, a| call_ptr(f, a));
    }
    /// `pppKeParMatR(obj)`.
    pppKeParMatR @ 0x0073_4880 => extern "C" fn ppp_ke_par_mat_r(obj: u32) {
        // the original's local matrix and vector
        let mut frame = super::StackFrame::<20>::new();
        let at = frame.at();
        mat::bridge::ppp_ke_par_mat_r(&mut mem(), obj, at, &mut |_, f, a| call_ptr(f, a));
    }
    /// `pppKeLnsClmTOtDraw(area)`: its stack locals in a buffer on this shim's
    /// stack (the frame pointer at its end).
    pppKeLnsClmTOtDraw @ 0x0075_a610 => extern "C" fn ppp_ke_lns_clm_t_ot_draw(area: u32) -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0x30]);
        let mut locals = Locals([0; 0x30]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0x30;
        lns::bridge::ppp_ke_lns_clm_t_ot_draw(&mut mem(), area, frame, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_part_fp_init(part)`.
    op_part_fp_init @ 0x0080_0700 => extern "C" fn op_part_fp_init(part: u32) -> u32 {
        opus::bridge::op_part_fp_init(&mut mem(), part, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_part_free(part)`.
    op_part_free @ 0x0080_09c0 => extern "C" fn op_part_free(part: u32) -> u32 {
        opus::bridge::op_part_free(&mut mem(), part, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opu_motion_etc(host, cmd)`.
    opu_motion_etc @ 0x0080_47b0 => extern "C" fn opu_motion_etc(host: u32, cmd: u32) -> u32 {
        opus::bridge::opu_motion_etc(&mut mem(), host, cmd, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opu_blur_00807f90(host, cmd)`.
    opu_blur_00807f90 @ 0x0080_7f90 => extern "C" fn opu_blur(host: u32, cmd: u32) -> u32 {
        opus::bridge::opu_blur(&mut mem(), host, cmd, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opu_et_battle_0080aca0(host, cmd)`.
    opu_et_battle_0080aca0 @ 0x0080_aca0 => extern "C" fn opu_et_battle(host: u32, cmd: u32) -> u32 {
        opus::bridge::opu_et_battle(&mut mem(), host, cmd, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opu_eff_end_0080c490(host, cmd)`.
    opu_eff_end_0080c490 @ 0x0080_c490 => extern "C" fn opu_eff_end(host: u32, cmd: u32) -> u32 {
        opus::bridge::opu_eff_end(&mut mem(), host, cmd, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opp_anmm_weapon_count_00811050(host, set, cmd, work)`.
    opp_anmm_weapon_count_00811050 @ 0x0081_1050 => extern "C" fn opp_anmm_weapon_count(host: u32, set: u32, cmd: u32, work: u32) -> u32 {
        anmm::bridge::weapon_count(&mut mem(), host, set, cmd, work)
    }
    /// `opp_flash(host)`.
    opp_flash @ 0x0081_2fa0 => extern "C" fn opp_flash(host: u32) -> u32 {
        opus::bridge::opp_flash(&mut mem(), host, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opu_et_ovl(host, cmd)`.
    opu_et_ovl @ 0x0081_4790 => extern "C" fn opu_et_ovl(host: u32, cmd: u32) -> u32 {
        opus::bridge::opu_et_ovl(&mut mem(), host, cmd)
    }
    /// `opu_damage(host, cmd)`.
    opu_damage @ 0x0081_5d90 => extern "C" fn opu_damage(host: u32, cmd: u32) -> u32 {
        opus::bridge::opu_damage(&mut mem(), host, cmd, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opu_damage_motion(host, cmd)`.
    opu_damage_motion @ 0x0081_5e10 => extern "C" fn opu_damage_motion(host: u32, cmd: u32) -> u32 {
        opus::bridge::opu_damage_motion(&mut mem(), host, cmd, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opu_ground_branch_00817e60(host, cmd)`.
    opu_ground_branch_00817e60 @ 0x0081_7e60 => extern "C" fn opu_ground_branch(host: u32, cmd: u32) -> u32 {
        opus::bridge::opu_ground_branch(&mut mem(), host, cmd, &mut |_, f, a| call_ptr_f64(f, a))
    }
    /// `opu_length_00817ee0(host, cmd)`.
    opu_length_00817ee0 @ 0x0081_7ee0 => extern "C" fn opu_length(host: u32, cmd: u32) -> u32 {
        opus::bridge::opu_length(&mut mem(), host, cmd)
    }
    /// `opu_kind2_0081c460(host, cmd)`.
    opu_kind2_0081c460 @ 0x0081_c460 => extern "C" fn opu_kind2(host: u32, cmd: u32) -> u32 {
        opus::bridge::opu_kind2(&mut mem(), host, cmd)
    }
    // ---- round 33 c ----
    /// `pppExSendPacket(rpc, tag, data, n)`.
    pppExSendPacket @ 0x0076_b210 => extern "C" fn ppp_ex_send_packet(rpc: u32, tag: u32, data: u32, n: u32) -> u32 {
        ppp::bridge::ppp_ex_send_packet(&mut mem(), rpc, tag, data, n, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_part_run(part)`.
    op_part_run @ 0x0080_08b0 => extern "C" fn op_part_run(part: u32) -> u32 {
        opus::bridge::op_part_run(&mut mem(), part, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opu_prog_set_0080ad20(host, cmd)`.
    opu_prog_set_0080ad20 @ 0x0080_ad20 => extern "C" fn opu_prog_set(host: u32, cmd: u32) -> u32 {
        opus::bridge::opu_prog_set(&mut mem(), host, cmd)
    }
    /// `opu_osu_load_pos_0080b460(host, cmd)`.
    opu_osu_load_pos_0080b460 @ 0x0080_b460 => extern "C" fn opu_osu_load_pos(host: u32, cmd: u32) -> u32 {
        opus::bridge::opu_osu_load_pos(&mut mem(), host, cmd, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opp_anmm_fin_en_bone5(host, set, cmd, work)`.
    opp_anmm_fin_en_bone5 @ 0x0081_0e20 => extern "C" fn opp_anmm_fin_en_bone5(host: u32, set: u32, cmd: u32, work: u32) -> u32 {
        let _ = host;
        anmm::bridge::fin_en_bone5(&mut mem(), set, cmd, work, [0; 3], &mut |_, f, a| call_ptr(f, a), &mut |_| crt_rand())
    }
    /// `DynGeo_ResetCurrentBuffer_0069fcb0(this)`.
    DynGeo_ResetCurrentBuffer_0069fcb0 @ 0x0069_fcb0 => extern "thiscall" fn dyn_geo_reset_current_buffer(this: u32) -> u32 {
        small::bridge::dyn_geo_reset_current_buffer(&mut mem(), this, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppKeLnsClmTDraw(obj, key, prog)`.
    pppKeLnsClmTDraw @ 0x0075_a540 => extern "C" fn ppp_ke_lns_clm_t_draw(obj: u32, key: u32, prog: u32) -> u32 {
        lns::bridge::ppp_ke_lns_clm_t_draw(&mut mem(), obj, key, prog, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppKeLnsLpTDraw(obj, key, prog)`: its stack locals (and
    /// pppKeLnsLpDraw's) in a buffer on this shim's stack (the frame
    /// pointer at its end).
    pppKeLnsLpTDraw @ 0x0075_b160 => extern "C" fn ppp_ke_lns_lp_t_draw(obj: u32, key: u32, prog: u32) -> u32 {
        // its vector at frame - 0x14, pppKeLnsLpDraw's 0x30 bytes below
        // (frame - 0x40) & !0xf
        #[repr(C, align(16))]
        struct Locals([u8; 0x80]);
        let mut locals = Locals([0; 0x80]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0x80;
        lns::bridge::ppp_ke_lns_lp_t_draw(&mut mem(), obj, key, prog, frame, &mut |_, f, a| call_ptr(f, a))
    }
    /// `create0706NoiseRenderDataBuffer(lo, hi)`.
    create0706NoiseRenderDataBuffer @ 0x007f_a510 => extern "C" fn create_0706_noise_render_data_buffer(lo: u32, hi: u32) -> u32 {
        // the original's locals: three words and a 0x100-byte name
        let mut frame = super::StackFrame::<0x43>::new();
        let at = frame.at();
        small::bridge::create_0706_noise_render_data_buffer(&mut mem(), lo, hi, at, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opu_en_etc(host, cmd)`.
    opu_en_etc @ 0x0081_5f60 => extern "C" fn opu_en_etc(host: u32, cmd: u32) -> u32 {
        opus::bridge::opu_en_etc(&mut mem(), host, cmd, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opu_fin_type(host, cmd)`.
    opu_fin_type @ 0x0081_67b0 => extern "C" fn opu_fin_type(host: u32, cmd: u32) -> u32 {
        opus::bridge::opu_fin_type(&mut mem(), host, cmd, &mut |_, y, x| crt_atan2(y, x))
    }
    // ---- round 33 d ----
    /// `VfxGeometry_Register_006a52f0(this, kind)`.
    VfxGeometry_Register_006a52f0 @ 0x006a_52f0 => extern "thiscall" fn vfx_geometry_register(this: u32, kind: u32) -> u32 {
        small::bridge::vfx_geometry_register(&mut mem(), this, kind, &mut |_, f, this, a: &[u32]| {
            // SAFETY: the engine's thiscall functions of one or two stack
            // words (the bridge asks for no other count).
            unsafe {
                match *a {
                    [x] => {
                        let g: unsafe extern "thiscall" fn(u32, u32) -> u32 = std::mem::transmute(f as usize);
                        g(this, x)
                    }
                    [x, y] => {
                        let g: unsafe extern "thiscall" fn(u32, u32, u32) -> u32 = std::mem::transmute(f as usize);
                        g(this, x, y)
                    }
                    _ => 0,
                }
            }
        })
    }
    /// `pppKaGra001Draw_00711830(hi, a, rec)`.
    pppKaGra001Draw_00711830 @ 0x0071_1830 => extern "C" fn ppp_ka_gra001_draw(hi: u32, a: u32, rec: u32) {
        small::bridge::ppp_ka_gra001_draw(&mut mem(), hi, a, rec, &mut |_, f, a| call_ptr(f, a));
    }
    /// `weaponShadowDrawInit(lo, hi, _, mode)`.
    weaponShadowDrawInit @ 0x0081_4070 => extern "C" fn weapon_shadow_draw_init(lo: u32, hi: u32, unused: u32, mode: u32) -> u32 {
        let _ = unused;
        // the original's locals: three words and a 0x100-byte name
        let mut frame = super::StackFrame::<0x43>::new();
        let at = frame.at();
        small::bridge::weapon_shadow_draw_init(&mut mem(), lo, hi, mode, at, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opu_flag_etc_008176f0(host, cmd)`.
    opu_flag_etc_008176f0 @ 0x0081_76f0 => extern "C" fn opu_flag_etc(host: u32, cmd: u32) -> u32 {
        opus::bridge::opu_flag_etc(&mut mem(), opus::bridge::FlagWord::Host, host, cmd)
    }
    /// `opu_v_get(host, cmd)`.
    opu_v_get @ 0x0081_9940 => extern "C" fn opu_v_get(host: u32, cmd: u32) -> u32 {
        opus::bridge::opu_v_get(&mut mem(), host, cmd)
    }
    /// `createZBufClearRenderData_00716110(lo, hi)`.
    createZBufClearRenderData_00716110 @ 0x0071_6110 => extern "C" fn create_zbuf_clear_render_data(lo: u32, hi: u32) -> u32 {
        // the original's locals: ten words and a 0x100-byte name
        let mut frame = super::StackFrame::<0x4a>::new();
        let at = frame.at();
        small::bridge::create_zbuf_clear_render_data(&mut mem(), lo, hi, at, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opu_mem(host, cmd)`.
    opu_mem @ 0x0081_70c0 => extern "C" fn opu_mem(host: u32, cmd: u32) -> u32 {
        opus::bridge::opu_mem(&mut mem(), host, cmd, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opu_signal_etc_008177f0(host, cmd)`.
    opu_signal_etc_008177f0 @ 0x0081_77f0 => extern "C" fn opu_signal_etc(host: u32, cmd: u32) -> u32 {
        opus::bridge::opu_flag_etc(&mut mem(), opus::bridge::FlagWord::Unit, host, cmd)
    }
    /// `opu_r_etc(host, cmd)`.
    opu_r_etc @ 0x0081_8110 => extern "C" fn opu_r_etc(host: u32, cmd: u32) -> u32 {
        opus::bridge::opu_r_etc(&mut mem(), host, cmd)
    }
    /// `opu_r_u(host, cmd)`.
    opu_r_u @ 0x0081_8200 => extern "C" fn opu_r_u(host: u32, cmd: u32) -> u32 {
        opus::bridge::opu_r_u(&mut mem(), host, cmd, &mut |_, y, x| crt_atan2(y, x))
    }
    /// `opu_end(obj, cmd)`.
    opu_end @ 0x0080_c3f0 => extern "C" fn opu_end(obj: u32, cmd: u32) -> u32 {
        opus::bridge::opu_end(&mut mem(), obj, cmd, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opus_chr_width_00800cd0(chr, battle)`: a float in `ST0`.
    opus_chr_width_00800cd0 @ 0x0080_0cd0 => extern "C" fn opus_chr_width(chr: u32, battle: u32) -> f32 {
        opus::bridge::opus_chr_width(&mut mem(), chr, battle, &mut |_, f, a| call_ptr_f64(f, a))
    }
    /// `op_ovl_effect_init()`.
    op_ovl_effect_init @ 0x007f_cdd0 => extern "C" fn op_ovl_effect_init() -> u32 {
        op::bridge::op_ovl_effect_init(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `opus_clip_off_all_in_battle()`.
    opus_clip_off_all_in_battle @ 0x0080_0bb0 => extern "C" fn opus_clip_off_all_in_battle() -> u32 {
        opus::bridge::opus_clip_off_all_in_battle(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_ivec4_to_fvec4_007e3100(dst, src)`.
    op_ivec4_to_fvec4_007e3100 @ 0x007e_3100 => extern "C" fn op_ivec4_to_fvec4(dst: u32, src: u32) {
        vec::bridge::ivec4_to_fvec4(&mut mem(), dst, src);
    }
    /// `op_num_xy_set(x, y)`.
    op_num_xy_set @ 0x007e_65e0 => extern "C" fn op_num_xy_set(x: u32, y: u32) {
        op::bridge::op_num_xy_set(&mut mem(), x, y, &mut |_, f, a| call_ptr(f, a));
    }
    /// `op_heap_init(heap, size)`.
    op_heap_init @ 0x007f_f2d0 => extern "C" fn op_heap_init(heap: u32, size: u32) -> u32 {
        op::bridge::op_heap_init(&mut mem(), heap, size)
    }
    /// `op_malloc(heap, n)`.
    op_malloc @ 0x007f_f780 => extern "C" fn op_malloc(heap: u32, n: u32) -> u32 {
        op::bridge::op_malloc(&mut mem(), heap, n, &mut |_, f, a| call_ptr(f, a))
    }
    /// `Vec4AddScalars_0074bc50(dst, src, x, y, z, w)`.
    Vec4AddScalars_0074bc50 @ 0x0074_bc50 => extern "C" fn vec4_add_scalars(dst: u32, src: u32, x: u32, y: u32, z: u32, w: u32) {
        vec::bridge::vec4_add_scalars(&mut mem(), dst, src, [x, y, z, w].map(f32::from_bits));
    }
    /// `op_trans_m(dst, src, v)`.
    op_trans_m @ 0x007e_a550 => extern "C" fn op_trans_m(dst: u32, src: u32, v: u32) -> u32 {
        op::bridge::op_trans_m(&mut mem(), dst, src, v)
    }
    /// `op_rtp_w1(dst, m, v)`.
    op_rtp_w1 @ 0x007e_9b50 => extern "C" fn op_rtp_w1(dst: u32, m: u32, v: u32) -> u32 {
        op::bridge::op_rtp_w1(&mut mem(), dst, m, v)
    }
    /// `op_vector_m(dst, t, v)`: its stack locals in a buffer on this
    /// shim's stack (the frame pointer at its end).
    op_vector_m @ 0x007e_a760 => extern "C" fn op_vector_m(dst: u32, t: u32, v: u32) -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0x60]);
        let mut locals = Locals([0; 0x60]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0x60;
        op::bridge::op_vector_m(&mut mem(), dst, t, v, frame, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_matrix_light4_vector(dst, m, colours)`: its stack locals in a
    /// buffer on this shim's stack (the frame pointer at its end).
    op_matrix_light4_vector @ 0x00a6_cd80 => extern "C" fn op_matrix_light4_vector(dst: u32, m: u32, colours: u32) -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0x90]);
        let mut locals = Locals([0; 0x90]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0x90;
        op::bridge::op_matrix_light4_vector(&mut mem(), dst, m, colours, frame, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_oef2_tex_clut_ovl_add_007e56d0(rec, mask, flag)`.
    op_oef2_tex_clut_ovl_add_007e56d0 @ 0x007e_56d0 => extern "C" fn op_oef2_tex_clut_ovl_add(rec: u32, mask: u32, flag: u32) -> i32 {
        op::bridge::op_oef2_tex_clut_ovl_add(&mut mem(), rec, mask, flag)
    }
    /// `pppKeLnsArndUpdate(obj, key, ctrl)`.
    pppKeLnsArndUpdate @ 0x0075_a2d0 => extern "C" fn ppp_ke_lns_arnd_update(obj: u32, key: u32, ctrl: u32) -> u32 {
        prog::bridge::ppp_ke_lns_arnd_update(&mut mem(), obj, key, ctrl)
    }
    /// `pppKeLnsFlsUpdate(obj, key, ctrl)`: the same code.
    pppKeLnsFlsUpdate @ 0x0075_ae90 => extern "C" fn ppp_ke_lns_fls_update(obj: u32, key: u32, ctrl: u32) -> u32 {
        prog::bridge::ppp_ke_lns_arnd_update(&mut mem(), obj, key, ctrl)
    }
    /// `pppKeLnsCrnUpdate(obj, key, ctrl)`.
    pppKeLnsCrnUpdate @ 0x0075_aab0 => extern "C" fn ppp_ke_lns_crn_update(obj: u32, key: u32, ctrl: u32) -> u32 {
        prog::bridge::ppp_ke_lns_crn_update(&mut mem(), obj, key, ctrl, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_unit_m_scale2(m, v)`.
    op_unit_m_scale2 @ 0x007e_a730 => extern "C" fn op_unit_m_scale2(m: u32, v: u32) -> u32 {
        op::bridge::op_unit_m_scale2(&mut mem(), m, v)
    }
    /// `op_fp_effect_free()`.
    op_fp_effect_free @ 0x007f_cc40 => extern "C" fn op_fp_effect_free() -> u32 {
        op::bridge::op_fp_effect_reset(&mut mem(), 0x00b5_84e4, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_fp_effect_init()`.
    op_fp_effect_init @ 0x007f_cc60 => extern "C" fn op_fp_effect_init() -> u32 {
        op::bridge::op_fp_effect_reset(&mut mem(), 0x00b5_84c8, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_et_prayeff_send_read_through(id)`.
    op_et_prayeff_send_read_through @ 0x007f_c920 => extern "C" fn op_et_prayeff_send_read_through(id: u32) -> u32 {
        op::bridge::op_et_prayeff_send_read_through(&mut mem(), id, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_deg_in_check(a, b, c)`.
    op_deg_in_check @ 0x007e_7f60 => extern "C" fn op_deg_in_check(a: u32, b: u32, c: u32) -> u32 {
        op::bridge::op_deg_in_check(&mem(), f32::from_bits(a), f32::from_bits(b), f32::from_bits(c))
    }
    /// `op_str_equal_007e2600(a, b, nocase)`.
    op_str_equal_007e2600 @ 0x007e_2600 => extern "C" fn op_str_equal(a: u32, b: u32, nocase: u32) -> u32 {
        op::bridge::op_str_equal(&mem(), a, b, nocase)
    }
    /// `op_pair_index_00a70ce0(a, b)`.
    op_pair_index_00a70ce0 @ 0x00a7_0ce0 => extern "C" fn op_pair_index(a: u32, b: u32) -> u32 {
        op::bridge::op_pair_index(&mut mem(), a, b)
    }
    /// `opus_cscale_get(chr, battle)`: a float in `ST0`.
    opus_cscale_get @ 0x0080_0c10 => extern "C" fn opus_cscale_get(chr: u32, battle: u32) -> f32 {
        opus::bridge::opus_cscale_get(&mut mem(), chr, battle, &mut |_, f, a| call_ptr_f64(f, a))
    }
    /// `op_fog_param_set()`: its local in a buffer on this shim's stack.
    op_fog_param_set @ 0x007f_f100 => extern "C" fn op_fog_param_set() {
        #[repr(C, align(16))]
        struct Locals([u8; 0x20]);
        let mut locals = Locals([0; 0x20]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0x20;
        op::bridge::op_fog_param_set(&mut mem(), frame, &mut |_, f, a| call_ptr(f, a));
    }
    /// `op_oef2_sep_wave_load(cue, mask)`.
    op_oef2_sep_wave_load @ 0x007f_ffa0 => extern "C" fn op_oef2_sep_wave_load(cue: u32, mask: u32) -> u32 {
        op::bridge::op_oef2_sep_wave_load(&mut mem(), cue, mask, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_ffxlight_to_omdlight(colours, dirs)`: its local in a buffer on
    /// this shim's stack.
    op_ffxlight_to_omdlight @ 0x00a6_90c0 => extern "C" fn op_ffxlight_to_omdlight(colours: u32, dirs: u32) -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0x20]);
        let mut locals = Locals([0; 0x20]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0x20;
        op::bridge::op_ffxlight_to_omdlight(&mut mem(), colours, dirs, frame, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_md_vn_index_sub(tri, table)`.
    op_md_vn_index_sub @ 0x00a6_9fe0 => extern "C" fn op_md_vn_index_sub(tri: u32, table: u32) -> u32 {
        op::bridge::op_md_vn_index_sub(&mut mem(), tri, table)
    }
    /// `op_error_007e1450(code, arg)`.
    op_error_007e1450 @ 0x007e_1450 => extern "C" fn op_error(code: u32, arg: u32) -> u32 {
        op::bridge::op_error(&mut mem(), code, arg, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppVu0Length_copy_0072ab20(v)`: a float in `ST0`.
    pppVu0Length_copy_0072ab20 @ 0x0072_ab20 => extern "C" fn ppp_vu0_length_copy(v: u32) -> f32 {
        vec::bridge::ppp_vu0_length(&mut mem(), v)
    }
    /// `op_spline(t, y, x, d, n)`: a float in `ST0`.
    op_spline @ 0x007e_9e20 => extern "C" fn op_spline(t: f32, y: u32, x: u32, d: u32, n: u32) -> f32 {
        op::bridge::op_spline(&mem(), t, y, x, d, n)
    }
    /// `op_spline_table(d, x, y, n)`.
    op_spline_table @ 0x007e_9f20 => extern "C" fn op_spline_table(d: u32, x: u32, y: u32, n: u32) {
        op::bridge::op_spline_table(&mut mem(), d, x, y, n);
    }
    /// `opd_num_init(x)`.
    opd_num_init @ 0x00a5_dda0 => extern "C" fn opd_num_init(x: u32) -> u32 {
        op::bridge::opd_num_init(&mut mem(), x, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_spline_xyz(out, t, pts, x, n)`.
    op_spline_xyz @ 0x007e_a120 => extern "C" fn op_spline_xyz(out: u32, t: f32, pts: u32, x: u32, n: u32) {
        op::bridge::op_spline_xyz(&mut mem(), out, t, pts, x, n);
    }
    /// `Vec4SubScalars_0074be90(dst, src, x, y, z, w)`.
    Vec4SubScalars_0074be90 @ 0x0074_be90 => extern "C" fn vec4_sub_scalars(dst: u32, src: u32, x: u32, y: u32, z: u32, w: u32) {
        vec::bridge::vec4_sub_scalars(&mut mem(), dst, src, [x, y, z, w].map(f32::from_bits));
    }
    /// `op_func_ot(list)`.
    op_func_ot @ 0x007e_b430 => extern "C" fn op_func_ot(list: u32) {
        op::bridge::op_func_ot(&mut mem(), list, &mut |_, f, a| call_ptr(f, a));
    }
    /// `op1_md_draw_eiabm_sphe(obj, part, n, e)`: its stack locals in a
    /// buffer on this shim's stack (the frame pointer at its end).
    op1_md_draw_eiabm_sphe @ 0x00a6_8140 => extern "C" fn op1_md_draw_eiabm_sphe(obj: u32, part: u32, n: u32, e: u32) -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0xb0]);
        let mut locals = Locals([0; 0xb0]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0xb0;
        md::bridge::op1_md_draw_eiabm_sphe(&mut mem(), obj, part, n, e, frame, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_rot_point_007ea9d0(x0, y0, ang, x1, y1, ox, oy)`.
    op_rot_point_007ea9d0 @ 0x007e_a9d0 => extern "C" fn op_rot_point(x0: f32, y0: f32, ang: f32, x1: f32, y1: f32, ox: u32, oy: u32) -> u32 {
        vec::bridge::op_rot_point(&mut mem(), x0, y0, ang, [x1, y1], [ox, oy], &mut |_, c, x| crt_trig(c, x))
    }
    /// `op_token_007e1f80(src, out, pos, max, eol)`.
    op_token_007e1f80 @ 0x007e_1f80 => extern "C" fn op_token(src: u32, out: u32, pos: u32, max: u32, eol: u32) -> u32 {
        op::bridge::op_token(&mut mem(), src, out, pos, max, eol, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op1_md_draw_eiabm(obj, part, slot)`: its stack locals in a buffer
    /// on this shim's stack (the frame pointer at its end).
    op1_md_draw_eiabm @ 0x00a6_57c0 => extern "C" fn op1_md_draw_eiabm(obj: u32, part: u32, slot: u32) -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0x450]);
        let mut locals = Locals([0; 0x450]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0x450;
        md::bridge::op1_md_draw_eiabm(&mut mem(), obj, part, slot, frame, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_md_spr_init_x64(obj, init, work)`.
    op_md_spr_init_x64 @ 0x00a6_9940 => extern "C" fn op_md_spr_init_x64(obj: u32, init: u32, work: u32) -> u32 {
        md::bridge::op_md_spr_init_x64(&mut mem(), obj, init, work)
    }
    /// `0x0069ebd0(it)`: `opusTree_Next`'s code again (`ecx` the iterator).
    TreeNext_0069ebd0 @ 0x0069_ebd0 => extern "thiscall" fn tree_next_0069ebd0(it: u32) -> u32 {
        tree::bridge::opus_tree_next(&mut mem(), it)
    }
    /// `opus_0437_fade_step_00a6e260(b)`.
    opus_0437_fade_step_00a6e260 @ 0x00a6_e260 => extern "C" fn opus_0437_fade_step(b: u32) -> u32 {
        small::bridge::opus_0437_fade_step(&mut mem(), b)
    }
    /// `op_texels_halve_v_00a38560(dst, dst_step, src, src_step, n, size)`.
    op_texels_halve_v_00a38560 @ 0x00a3_8560 => extern "C" fn op_texels_halve_v(dst: u32, dst_step: u32, src: u32, src_step: u32, n: u32, size: u32) -> u32 {
        small::bridge::op_texels_halve_v(&mut mem(), dst, dst_step, src, src_step, n, size)
    }
    /// `op_texels_halve_00a385f0(dst, src, n, size)`.
    op_texels_halve_00a385f0 @ 0x00a3_85f0 => extern "C" fn op_texels_halve(dst: u32, src: u32, n: u32, size: u32) -> u32 {
        small::bridge::op_texels_halve(&mut mem(), dst, src, n, size)
    }
    /// `op_texels_box_00a38670(dst, row_a, row_b, n, size)`.
    op_texels_box_00a38670 @ 0x00a3_8670 => extern "C" fn op_texels_box(dst: u32, row_a: u32, row_b: u32, n: u32, size: u32) -> u32 {
        small::bridge::op_texels_box(&mut mem(), dst, row_a, row_b, n, size)
    }
    /// `op_node_requeue_all_00a39f60(set)`: returns `set + 0x80`.
    op_node_requeue_all_00a39f60 @ 0x00a3_9f60 => extern "C" fn op_node_requeue_all(set: u32) -> u32 {
        small::bridge::op_node_requeue_all(&mut mem(), set)
    }
    /// `op_node_requeue_00a39f00(n)`.
    op_node_requeue_00a39f00 @ 0x00a3_9f00 => extern "C" fn op_node_requeue(n: u32) -> u32 {
        small::bridge::op_node_requeue(&mut mem(), n)
    }
    /// `opus_osu_init(out)`.
    opus_osu_init @ 0x0080_1200 => extern "C" fn opus_osu_init(out: u32) -> u32 {
        opus::bridge::opus_osu_init(&mut mem(), out)
    }
    /// `opus_osu_copy_00800eb0(set, dst, src)`.
    opus_osu_copy_00800eb0 @ 0x0080_0eb0 => extern "C" fn opus_osu_copy(set: u32, dst: u32, src: u32) -> u32 {
        opus::bridge::opus_osu_copy(&mut mem(), set, dst, src)
    }
    /// `op_rgb_to_hsv_00807e90(out, src, n)`.
    op_rgb_to_hsv_00807e90 @ 0x0080_7e90 => extern "C" fn op_rgb_to_hsv(out: u32, src: u32, n: u32) -> u32 {
        small::bridge::op_rgb_to_hsv(&mut mem(), out, src, n)
    }
    /// `op_node_pool_init_00a39bc0(set, n, v)`.
    op_node_pool_init_00a39bc0 @ 0x00a3_9bc0 => extern "C" fn op_node_pool_init(set: u32, n: u32, v: u32) -> u32 {
        small::bridge::op_node_pool_init(&mut mem(), set, n, v)
    }
    /// `op_slot_bytes_reset_008888e0(obj)`.
    op_slot_bytes_reset_008888e0 @ 0x0088_88e0 => extern "C" fn op_slot_bytes_reset(obj: u32) -> u32 {
        small::bridge::op_slot_bytes_reset(&mut mem(), obj)
    }
    /// `Fp_IdState_0072a760(id)`.
    Fp_IdState_0072a760 @ 0x0072_a760 => extern "C" fn fp_id_state(id: u32) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // the i16 argument
        fp::bridge::fp_id_state(&mem(), id as u16)
    }
    /// `op_md_packet_size_00a698c0(packet)`.
    op_md_packet_size_00a698c0 @ 0x00a6_98c0 => extern "C" fn op_md_packet_size(packet: u32) -> u32 {
        op::bridge::op_md_packet_size(&mem(), packet)
    }
    /// `op_read_line_007e1cc0(src, out, max)`.
    op_read_line_007e1cc0 @ 0x007e_1cc0 => extern "C" fn op_read_line(src: u32, out: u32, max: u32) -> u32 {
        op::bridge::op_read_line(&mut mem(), src, out, max)
    }
    /// `op_line_length_007e2660(src, lines)`.
    op_line_length_007e2660 @ 0x007e_2660 => extern "C" fn op_line_length(src: u32, lines: u32) -> u32 {
        op::bridge::op_line_length(&mut mem(), src, lines)
    }
    /// `op_counter_add_00888890(n)`.
    op_counter_add_00888890 @ 0x0088_8890 => extern "C" fn op_counter_add(n: u32) -> u32 {
        small::bridge::op_counter_add(&mut mem(), n)
    }
    /// `getAlphaForShpIgnoreAnFormatBaseOnSh(shp)`.
    getAlphaForShpIgnoreAnFormatBaseOnSh @ 0x0071_6550 => extern "C" fn shp_alpha(shp: u32) -> u32 {
        small::bridge::shp_alpha(&mem(), shp)
    }
    /// `op_node_slot_00a3ba10(node)`.
    op_node_slot_00a3ba10 @ 0x00a3_ba10 => extern "C" fn op_node_slot(node: u32) -> u32 {
        small::bridge::op_node_slot(&mem(), node)
    }
    /// `op_mip_bytes_00a3c0f0(w, h, levels)`.
    op_mip_bytes_00a3c0f0 @ 0x00a3_c0f0 => extern "C" fn op_mip_bytes(w: u32, h: u32, levels: u32) -> u32 {
        small::bridge::op_mip_bytes(w, h, levels)
    }
    /// `op_md_white_vectors_00a6cf70(obj, _, work)`.
    op_md_white_vectors_00a6cf70 @ 0x00a6_cf70 => extern "C" fn op_md_white_vectors(obj: u32, _unused: u32, work: u32) -> u32 {
        md::bridge::op_md_white_vectors(&mut mem(), obj, work)
    }
    /// `op_oef2_tex_clut_init_007e5680()`.
    op_oef2_tex_clut_init_007e5680 @ 0x007e_5680 => extern "C" fn op_oef2_tex_clut_init() -> u32 {
        op::bridge::op_oef2_tex_clut_init(&mut mem())
    }
    /// `op_oef2_sound_init_00800440()`.
    op_oef2_sound_init_00800440 @ 0x0080_0440 => extern "C" fn op_oef2_sound_init() -> u32 {
        op::bridge::op_oef2_sound_init(&mut mem())
    }
    /// `pppVu0MaddW_copy_0074bde0(dst, v, _, _, _, s)`: `pppVu0MaddW`'s
    /// code again; returns `dst`.
    pppVu0MaddW_copy_0074bde0 @ 0x0074_bde0 => extern "C" fn ppp_vu0_madd_w_copy(dst: u32, v: u32, a: u32, b: u32, c: u32, s: f32) -> u32 {
        let _ = (a, b, c);
        mat::bridge::ppp_vu0_madd(&mut mem(), dst, v, s);
        dst
    }
    /// `op_ivec4_shr7_00749050(out, v)`.
    op_ivec4_shr7_00749050 @ 0x0074_9050 => extern "C" fn ivec4_shr7_to_fvec4(out: u32, v: u32) -> u32 {
        vec::bridge::ivec4_shr7_to_fvec4(&mut mem(), out, v)
    }
    /// `op_et_chr_slot_007fead0(chr)`.
    op_et_chr_slot_007fead0 @ 0x007f_ead0 => extern "C" fn op_et_chr_slot(chr: u32) -> u32 {
        et::bridge::op_et_chr_slot(&mem(), chr)
    }
    // opp_anmm's sprite commands (host, set, cmd, work) -> next command.
    // The random ones' extra callee stack words (the original's saved
    // registers and return address) only reach VU0 scratch: zeros here.
    /// `opp_anmm(host, set)`.
    opp_anmm @ 0x0081_24b0 => extern "C" fn opp_anmm(host: u32, set: u32) -> u32 {
        // the original's local at [ebp - 0xc] (set_ptr32's result)
        let mut frame = super::StackFrame::<1>::new();
        let at = frame.at();
        anmm::bridge::opp_anmm(&mut mem(), host, set, at, &mut |_, f, a| call_ptr(f, a), &mut |_, f, a| call_ptr_f64(f, a))
    }
    /// `opp_anmm_wait_random(host, set, cmd, work)`.
    opp_anmm_wait_random @ 0x0081_06e0 => extern "C" fn opp_anmm_wait_random(host: u32, set: u32, cmd: u32, work: u32) -> u32 {
        let _ = (set, work);
        anmm::bridge::wait_random(&mut mem(), host, cmd, [0; 3])
    }
    /// `opp_anmm_sprite_set(host, set, cmd, work)`.
    opp_anmm_sprite_set @ 0x0081_0720 => extern "C" fn opp_anmm_sprite_set(host: u32, set: u32, cmd: u32, work: u32) -> u32 {
        let _ = host;
        anmm::bridge::sprite_set(&mut mem(), set, cmd, work)
    }
    /// `opp_anmm_flag_on(host, set, cmd, work)`.
    opp_anmm_flag_on @ 0x0081_0790 => extern "C" fn opp_anmm_flag_on(host: u32, set: u32, cmd: u32, work: u32) -> u32 {
        let _ = set;
        anmm::bridge::flag_on(&mut mem(), host, cmd, work)
    }
    /// `opp_anmm_vec_set(host, set, cmd, work)`.
    opp_anmm_vec_set @ 0x0081_07c0 => extern "C" fn opp_anmm_vec_set(host: u32, set: u32, cmd: u32, work: u32) -> u32 {
        let _ = set;
        anmm::bridge::vec_set(&mut mem(), host, cmd, work)
    }
    /// `opp_anmm_vec_add_random(host, set, cmd, work)`.
    opp_anmm_vec_add_random @ 0x0081_0890 => extern "C" fn opp_anmm_vec_add_random(host: u32, set: u32, cmd: u32, work: u32) -> u32 {
        let _ = set;
        anmm::bridge::vec_add_random(&mut mem(), host, cmd, work)
    }
    /// `opp_anmm_vec_add(host, set, cmd, work)`.
    opp_anmm_vec_add @ 0x0081_0a90 => extern "C" fn opp_anmm_vec_add(host: u32, set: u32, cmd: u32, work: u32) -> u32 {
        let _ = set;
        anmm::bridge::vec_add(&mut mem(), host, cmd, work)
    }
    /// `opp_anmm_short_add_random(host, set, cmd, work)`.
    opp_anmm_short_add_random @ 0x0081_0ba0 => extern "C" fn opp_anmm_short_add_random(host: u32, set: u32, cmd: u32, work: u32) -> u32 {
        let _ = set;
        anmm::bridge::short_add_random(&mut mem(), host, cmd, work, [0; 3])
    }
    /// `opp_anmm_pair_set(host, set, cmd, work)`.
    opp_anmm_pair_set @ 0x0081_0be0 => extern "C" fn opp_anmm_pair_set(host: u32, set: u32, cmd: u32, work: u32) -> u32 {
        let _ = set;
        anmm::bridge::pair_set(&mut mem(), host, cmd, work)
    }
    /// `opp_anmm_mode_set(host, set, cmd, work)`.
    opp_anmm_mode_set @ 0x0081_0c20 => extern "C" fn opp_anmm_mode_set(host: u32, set: u32, cmd: u32, work: u32) -> u32 {
        let _ = set;
        anmm::bridge::mode_set(&mut mem(), host, cmd, work)
    }
    /// `opp_anmm_rgba_set(host, set, cmd, work)`.
    opp_anmm_rgba_set @ 0x0081_0c60 => extern "C" fn opp_anmm_rgba_set(host: u32, set: u32, cmd: u32, work: u32) -> u32 {
        let _ = set;
        anmm::bridge::rgba_set(&mut mem(), host, cmd, work)
    }
    /// `opp_anmm_loop(host, set, cmd, work)`.
    opp_anmm_loop @ 0x0081_0cf0 => extern "C" fn opp_anmm_loop(host: u32, set: u32, cmd: u32, work: u32) -> u32 {
        let _ = (host, set);
        anmm::bridge::loop_back(&mut mem(), cmd, work)
    }
    /// `opp_anmm_branch_random(host, set, cmd, work)`.
    opp_anmm_branch_random @ 0x0081_0d10 => extern "C" fn opp_anmm_branch_random(host: u32, set: u32, cmd: u32, work: u32) -> u32 {
        let _ = (host, set, work);
        anmm::bridge::branch_random(&mut mem(), cmd, [0; 3])
    }
    /// `opp_anmm_fade(host, set, cmd, work)`.
    opp_anmm_fade @ 0x0081_0d50 => extern "C" fn opp_anmm_fade(host: u32, set: u32, cmd: u32, work: u32) -> u32 {
        let _ = set;
        anmm::bridge::fade(&mut mem(), host, cmd, work)
    }
    /// `opp_anmm_part_float(host, set, cmd, work)`.
    opp_anmm_part_float @ 0x0081_0da0 => extern "C" fn opp_anmm_part_float(host: u32, set: u32, cmd: u32, work: u32) -> u32 {
        let _ = host;
        anmm::bridge::part_float(&mut mem(), set, cmd, work)
    }
    /// `opus_encount0_init(obj)`.
    opus_encount0_init @ 0x007f_8480 => extern "C" fn opus_encount0_init(obj: u32) -> u32 {
        opus::bridge::opus_encount0_init(&mut mem(), obj)
    }
    /// `opu_part_kill(host, cmd)`.
    opu_part_kill @ 0x0080_b7d0 => extern "C" fn opu_part_kill(host: u32, cmd: u32) -> u32 {
        opus::bridge::opu_part_kill(&mut mem(), host, cmd, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opu_local_link(host, cmd)`.
    opu_local_link @ 0x0080_1a50 => extern "C" fn opu_local_link(host: u32, cmd: u32) -> u32 {
        opus::bridge::opu_local_link(&mut mem(), host, cmd)
    }
    /// `opu_etc(host, cmd)`.
    opu_etc @ 0x0081_6610 => extern "C" fn opu_etc(host: u32, cmd: u32) -> u32 {
        opus::bridge::opu_etc(&mut mem(), host, cmd)
    }
    /// `opu_jloop(host, cmd)`.
    opu_jloop @ 0x0081_6870 => extern "C" fn opu_jloop(host: u32, cmd: u32) -> u32 {
        opus::bridge::opu_jloop(&mut mem(), host, cmd)
    }
    /// `opu_unit_max(host, cmd)`.
    opu_unit_max @ 0x0081_7530 => extern "C" fn opu_unit_max(host: u32, cmd: u32) -> u32 {
        opus::bridge::opu_unit_max(&mut mem(), host, cmd)
    }
    /// `opus_copy_1k_00800ab0(dst, src)`.
    opus_copy_1k_00800ab0 @ 0x0080_0ab0 => extern "C" fn opus_copy_1k(dst: u32, src: u32) -> u32 {
        opus::bridge::copy_1k(&mut mem(), dst, src)
    }
    /// `opus_copy_1k_00800af0(src, dst)`.
    opus_copy_1k_00800af0 @ 0x0080_0af0 => extern "C" fn opus_copy_1k_rev(src: u32, dst: u32) -> u32 {
        opus::bridge::copy_1k(&mut mem(), dst, src)
    }
    /// `opf_cir(host, ptr)`.
    opf_cir @ 0x0081_4c90 => extern "C" fn opf_cir(host: u32, ptr: u32) -> u32 {
        opus::bridge::opf_cir(&mut mem(), host, ptr)
    }
    /// `opf_cir2(host, ptr)`.
    opf_cir2 @ 0x0081_4d20 => extern "C" fn opf_cir2(host: u32, ptr: u32) -> u32 {
        opus::bridge::opf_cir2(&mut mem(), host, ptr)
    }
    /// `opf_cir_gw(host, ptr)`.
    opf_cir_gw @ 0x0081_4e50 => extern "C" fn opf_cir_gw(host: u32, ptr: u32) -> u32 {
        opus::bridge::opf_cir_gw(&mut mem(), host, ptr)
    }
    /// `opf_cir_y(host, ptr)`.
    opf_cir_y @ 0x0081_4f00 => extern "C" fn opf_cir_y(host: u32, ptr: u32) -> u32 {
        opus::bridge::opf_cir_y(&mut mem(), host, ptr)
    }
    /// `opf_cirz_gw(host, ptr)`.
    opf_cirz_gw @ 0x0081_4fa0 => extern "C" fn opf_cirz_gw(host: u32, ptr: u32) -> u32 {
        opus::bridge::opf_cirz_gw(&mut mem(), host, ptr)
    }
    /// `opf_sph(host, ptr)`.
    opf_sph @ 0x0081_55b0 => extern "C" fn opf_sph(host: u32, ptr: u32) -> u32 {
        opus::bridge::opf_sph(&mut mem(), host, ptr)
    }
    /// `opf_spl_gw(host, ptr)`.
    opf_spl_gw @ 0x0081_5670 => extern "C" fn opf_spl_gw(host: u32, ptr: u32) -> u32 {
        opus::bridge::opf_spl_gw(&mut mem(), host, ptr)
    }
    /// `opf_rel(host, ptr)`.
    opf_rel @ 0x0081_5120 => extern "C" fn opf_rel(host: u32, ptr: u32) -> u32 {
        opus::bridge::opf_rel(&mut mem(), host, ptr)
    }
    /// `opf_rel2(host, ptr)`.
    opf_rel2 @ 0x0081_5350 => extern "C" fn opf_rel2(host: u32, ptr: u32) -> u32 {
        opus::bridge::opf_rel2(&mut mem(), host, ptr)
    }
    /// `opu_vzero(host, cmd)`.
    opu_vzero @ 0x0081_afa0 => extern "C" fn opu_vzero(host: u32, cmd: u32) -> u32 {
        opus::bridge::opu_vzero(&mut mem(), host, cmd, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opu_spl(host, cmd)`.
    opu_spl @ 0x0081_b560 => extern "C" fn opu_spl(host: u32, cmd: u32) -> u32 {
        opus::bridge::opu_spl(&mut mem(), host, cmd, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opd_exit_from_ovl()`.
    opd_exit_from_ovl @ 0x007f_f8e0 => extern "C" fn opd_exit_from_ovl() -> u32 {
        op::bridge::opd_exit_from_ovl(&mut mem())
    }
    /// `op_heap_display_set(a, b, c)`: `eax` is left alone outside debug
    /// mode; 0 here.
    op_heap_display_set @ 0x007f_f290 => extern "C" fn op_heap_display_set(a: u32, b: u32, c: u32) -> u32 {
        op::bridge::op_heap_display_set(&mut mem(), [a, b, c], &mut |_, f, a| call_ptr(f, a)).unwrap_or(0)
    }
    /// `op_num_vdisp()`.
    op_num_vdisp @ 0x007e_6560 => extern "C" fn op_num_vdisp() -> u32 {
        op::bridge::op_num_vdisp(&mut mem(), &mut |_, f, a| call_ptr(f, a)).unwrap_or(0)
    }
    /// `op_num_display(x)`.
    op_num_display @ 0x007e_6440 => extern "C" fn op_num_display(x: u32) -> u32 {
        op::bridge::op_num_display(&mut mem(), x, &mut |_, f, a| call_ptr(f, a)).unwrap_or(0)
    }
    /// `ops_init_spr()`.
    ops_init_spr @ 0x007e_6c30 => extern "C" fn ops_init_spr() -> u32 {
        op::bridge::ops_init_spr(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_rt_m_xyz(m, src, angles, t)`.
    op_rt_m_xyz @ 0x007e_9970 => extern "C" fn op_rt_m_xyz(m: u32, src: u32, angles: u32, t: u32) -> u32 {
        op::bridge::op_rt_m_xyz(&mut mem(), m, src, angles, t)
    }
    /// `opus_get_bone_vertex(out, chr, bone)`.
    opus_get_bone_vertex @ 0x0080_54d0 => extern "C" fn opus_get_bone_vertex(out: u32, chr: u32, bone: u32) -> u32 {
        opus::bridge::opus_get_bone_vertex(&mut mem(), out, chr, bone, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opus_cheight_get(chr, battle)`: returns in `ST0`.
    opus_cheight_get @ 0x0080_0b70 => extern "C" fn opus_cheight_get(chr: u32, battle: u32) -> f32 {
        opus::bridge::opus_cheight_get(&mut mem(), chr, battle, &mut |_, f, a| call_ptr_f64(f, a))
    }
    /// `opus_get_chr_pos(chr, out)`.
    opus_get_chr_pos @ 0x0081_b1e0 => extern "C" fn opus_get_chr_pos(chr: u32, out: u32) -> u32 {
        opus::bridge::opus_get_chr_pos(&mut mem(), chr, out, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_symbol_find_007e11a0(name, list)`.
    op_symbol_find_007e11a0 @ 0x007e_11a0 => extern "C" fn op_symbol_find(name: u32, list: u32) -> u32 {
        op::bridge::op_symbol_find(&mem(), name, list)
    }
    /// `op_token_get_007e1d50(file, src, out, pos, max, eol, flags)`.
    op_token_get_007e1d50 @ 0x007e_1d50 => extern "C" fn op_token_get(file: u32, src: u32, out: u32, pos: u32, max: u32, eol: u32, flags: u32) -> u32 {
        op::bridge::op_token_get(&mut mem(), file, src, out, pos, max, eol, flags, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_oef2_tex0_get_007e5260(oef, no, out)`.
    op_oef2_tex0_get_007e5260 @ 0x007e_5260 => extern "C" fn op_oef2_tex0_get(oef: u32, no: u32, out: u32) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // the u16 argument
        op::bridge::op_oef2_tex0_get(&mut mem(), oef, no as u16, out)
    }
    /// `opus_mdc_head_set_00802450(head, mats, set, part, tex)`.
    opus_mdc_head_set_00802450 @ 0x0080_2450 => extern "C" fn opus_mdc_head_set(head: u32, mats: u32, set: u32, part: u32, tex: u32) -> u32 {
        opus::bridge::opus_mdc_head_set(&mut mem(), head, mats, set, part, tex)
    }
    /// `pppSendPacket(rpc, tag, data, n)`.
    pppSendPacket @ 0x0076_b3d0 => extern "C" fn ppp_send_packet(rpc: u32, tag: u32, data: u32, n: u32) -> u32 {
        ppp::bridge::ppp_send_packet(&mut mem(), rpc, tag, data, n, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opus_chara_status_rgb_set(chr, weapons)`.
    opus_chara_status_rgb_set @ 0x0080_aa00 => extern "C" fn opus_chara_status_rgb_set(chr: u32, weapons: u32) {
        opus::bridge::opus_chara_status_rgb_set(&mut mem(), chr, weapons, &mut |_, f, a| call_ptr(f, a));
    }
    /// `opus_chara_rgba_set(chr, rgba, weapons)`.
    opus_chara_rgba_set @ 0x0080_a560 => extern "C" fn opus_chara_rgba_set(chr: u32, rgba: u32, weapons: u32) {
        opus::bridge::opus_chara_rgba_set(&mut mem(), chr, rgba, weapons, &mut |_, f, a| call_ptr(f, a));
    }
    /// `opus_mdc_lights_get_00802bd0(head, _, list, unit, nos)`.
    opus_mdc_lights_get_00802bd0 @ 0x0080_2bd0 => extern "C" fn opus_mdc_lights_get(head: u32, unused: u32, list: u32, unit: u32, nos: u32) {
        let _ = unused;
        opus::bridge::opus_mdc_lights_get(&mut mem(), head, list, unit, nos);
    }
    /// `opus_hsv_rgba_00807b70(out, src, n, time, sat, val, mix)`.
    opus_hsv_rgba_00807b70 @ 0x0080_7b70 => extern "C" fn opus_hsv_rgba(out: u32, src: u32, n: i32, time: i32, sat: i32, val: i32, mix: i32) -> u32 {
        opus::bridge::opus_hsv_rgba(&mut mem(), out, src, n, time, sat, val, mix)
    }
    /// `pppSetFpPdt(data)`.
    pppSetFpPdt @ 0x0072_a7f0 => extern "C" fn ppp_set_fp_pdt(data: u32) {
        fp::bridge::ppp_set_fp_pdt(&mut mem(), data, &mut |_, f, a| call_ptr(f, a));
    }
    /// `pppFpFree()`.
    pppFpFree @ 0x0072_91c0 => extern "C" fn ppp_fp_free() {
        fp::bridge::ppp_fp_free(&mut mem(), &mut |_, f, a| call_ptr(f, a));
    }
    /// `pppFpIDfree_007295f0(id)`.
    pppFpIDfree_007295f0 @ 0x0072_95f0 => extern "C" fn ppp_fp_id_free(id: u32) {
        #[allow(clippy::cast_possible_truncation)] // a u16 argument
        fp::bridge::ppp_fp_id_free(&mut mem(), id as u16, &mut |_, f, a| call_ptr(f, a));
    }
    /// `pppFpLoadStat(stat)`.
    pppFpLoadStat @ 0x0072_9a10 => extern "C" fn ppp_fp_load_stat(stat: u32) {
        fp::bridge::ppp_fp_load_stat(&mut mem(), stat);
    }
    /// `pppSetFpTex(oef)`.
    pppSetFpTex @ 0x0072_aa40 => extern "C" fn ppp_set_fp_tex(oef: u32) {
        fp::bridge::ppp_set_fp_tex(&mut mem(), oef, &mut |_, f, a| call_ptr(f, a));
    }
    /// `pppShapeFieldRenderOn_00743030()`.
    pppShapeFieldRenderOn_00743030 @ 0x0074_3030 => extern "C" fn shape_field_render_on() -> u32 {
        draw::bridge::shape_field_render_on(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_et_weapon_free(obj)`.
    op_et_weapon_free @ 0x007f_ca90 => extern "C" fn op_et_weapon_free(obj: u32) {
        et::bridge::op_et_weapon_free(&mut mem(), obj, &mut |_, f, a| call_ptr(f, a));
    }
    /// `op_et_weapon_free_all()`.
    op_et_weapon_free_all @ 0x007f_caf0 => extern "C" fn op_et_weapon_free_all() {
        et::bridge::op_et_weapon_free_all(&mut mem(), &mut |_, f, a| call_ptr(f, a));
    }
    /// `op_fvec4_to_ivec4_00808b00(dst, src)`.
    op_fvec4_to_ivec4_00808b00 @ 0x0080_8b00 => extern "C" fn fvec4_to_ivec4(dst: u32, src: u32) {
        vec::bridge::fvec4_to_ivec4(&mut mem(), dst, src);
    }
    /// `op_ot_draw()`.
    op_ot_draw @ 0x007e_c470 => extern "C" fn op_ot_draw() {
        op::bridge::op_ot_draw(&mut mem(), &mut |_, f, a| call_ptr(f, a));
    }
    /// `op_oef2_tex_clut_ovl_trans(mask)`.
    op_oef2_tex_clut_ovl_trans @ 0x007e_4e80 => extern "C" fn op_oef2_tex_clut_ovl_trans(mask: u32) {
        op::bridge::op_oef2_tex_clut_ovl_trans(&mut mem(), mask, &mut |_, f, a| call_ptr(f, a));
    }
    /// `opus_mdc_free(oef, chr, heap, second)`.
    opus_mdc_free @ 0x0080_61a0 => extern "C" fn opus_mdc_free(oef: u32, chr: u32, heap: u32, second: u32) {
        opus::bridge::opus_mdc_free(&mut mem(), oef, chr, heap, second, &mut |_, f, a| call_ptr(f, a));
    }
    /// `op_et_astatus_effect_off(chr)`.
    op_et_astatus_effect_off @ 0x007f_b000 => extern "C" fn op_et_astatus_effect_off(chr: u32) {
        et::bridge::op_et_astatus_effect_off(&mut mem(), chr, &mut |_, f, a| call_ptr(f, a));
    }
    /// `pppKeLnsClmUpdate(obj, key, prog)`.
    pppKeLnsClmUpdate @ 0x0075_a6a0 => extern "C" fn ppp_ke_lns_clm_update(obj: u32, key: u32, prog: u32) {
        lns::bridge::ppp_ke_lns_clm_update(&mut mem(), obj, key, prog, &mut |_, f, a| call_ptr(f, a));
    }
    /// `opus_change_chr_status_en()`.
    opus_change_chr_status_en @ 0x007f_e0f0 => extern "C" fn opus_change_chr_status_en() -> u32 {
        // the original's locals: the index map and both flag copies
        let mut frame = super::StackFrame::<0xf0>::new();
        let at = frame.at();
        et::bridge::opus_change_chr_status_en(&mut mem(), at)
    }
    /// `opus_change_eff_status_en_007fe280()`.
    opus_change_eff_status_en_007fe280 @ 0x007f_e280 => extern "C" fn opus_change_eff_status_en() -> u32 {
        // the original's local index map
        let mut frame = super::StackFrame::<0xf0>::new();
        let at = frame.at();
        et::bridge::opus_change_eff_status_en(&mut mem(), at)
    }
    /// `opus_change_osu_en_by_chr_status(list)`.
    opus_change_osu_en_by_chr_status @ 0x007f_e340 => extern "C" fn opus_change_osu_en_by_chr_status(list: u32) -> u32 {
        // the original's local index map
        let mut frame = super::StackFrame::<0xf0>::new();
        let at = frame.at();
        et::bridge::opus_change_osu_en_by_chr_status(&mut mem(), list, at)
    }
    /// `pppCreateFrameBufferData_00715a40(lo, hi)`.
    pppCreateFrameBufferData_00715a40 @ 0x0071_5a40 => extern "C" fn create_frame_buffer_data(lo: u32, hi: u32) -> u32 {
        // the original's locals: count, sizes and name
        let mut frame = super::StackFrame::<0x43>::new();
        let at = frame.at();
        draw::bridge::create_frame_buffer_data(&mut mem(), lo, hi, at, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppCreateD3D11FrameBufferData_00716050(lo, hi, n)`.
    pppCreateD3D11FrameBufferData_00716050 @ 0x0071_6050 => extern "C" fn create_d3d11_frame_buffer_data(lo: u32, hi: u32, n: u32) -> u32 {
        // the original's locals: count, sizes and name
        let mut frame = super::StackFrame::<0x43>::new();
        let at = frame.at();
        draw::bridge::create_d3d11_frame_buffer_data(&mut mem(), lo, hi, n, at, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_et_eff_target_center_0081b3e0(eff, out, filter)`.
    op_et_eff_target_center_0081b3e0 @ 0x0081_b3e0 => extern "C" fn op_et_eff_target_center(eff: u32, out: u32, filter: u32) {
        et::bridge::op_et_eff_target_center(&mut mem(), eff, out, filter, &mut |_, f, a| call_ptr(f, a));
    }
    /// `op_rot_m_y_apply_007e7be0(dst, src, angle)`.
    op_rot_m_y_apply_007e7be0 @ 0x007e_7be0 => extern "C" fn op_rot_m_y_apply(dst: u32, src: u32, angle: u32) {
        // the original's local matrix
        let mut frame = super::StackFrame::<16>::new();
        let at = frame.at();
        op::bridge::op_rot_m_y_apply(&mut mem(), dst, src, angle, at, &mut |_, f, a| call_ptr(f, a));
    }
    /// `pppFpSetWoodMatrix()`.
    pppFpSetWoodMatrix @ 0x0072_a690 => extern "C" fn ppp_fp_set_wood_matrix() -> u32 {
        // the original's three local vectors
        let mut frame = super::StackFrame::<12>::new();
        let at = frame.at();
        fp::bridge::ppp_fp_set_wood_matrix(&mut mem(), at)
    }
    /// `pppFpSetRotMatrix(part, mode)`.
    pppFpSetRotMatrix @ 0x0072_a2e0 => extern "C" fn ppp_fp_set_rot_matrix(part: u32, mode: u32) -> u32 {
        mat::bridge::ppp_fp_set_rot_matrix(&mut mem(), part, mode)
    }
    /// `opus_osu_set_0080b520(set, src, ptr, list, _)`.
    opus_osu_set_0080b520 @ 0x0080_b520 => extern "C" fn opus_osu_set(set: u32, src: u32, ptr: u32, list: u32, unused: u32) -> u32 {
        let _ = unused;
        opus::bridge::opus_osu_set(&mut mem(), set, src, ptr, list, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_token_expr_007e2140(src, out, pos, max, eol)`.
    op_token_expr_007e2140 @ 0x007e_2140 => extern "C" fn op_token_expr(src: u32, out: u32, pos: u32, max: u32, eol: u32) -> u32 {
        op::bridge::op_token_expr(&mut mem(), src, out, pos, max, eol, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opu_osu_load_0080ae30(host, cmd)`.
    opu_osu_load_0080ae30 @ 0x0080_ae30 => extern "C" fn opu_osu_load(host: u32, cmd: u32) -> u32 {
        opus::bridge::opu_osu_load(&mut mem(), host, cmd, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_oef2_sound_cue_loop()`.
    op_oef2_sound_cue_loop @ 0x0080_0170 => extern "C" fn op_oef2_sound_cue_loop() {
        op::bridge::op_oef2_sound_cue_loop(&mut mem(), &mut |_, f, a| call_ptr(f, a));
    }
    /// `op_spline_draw_path3(scale, spread)`.
    op_spline_draw_path3 @ 0x007e_67f0 => extern "C" fn op_spline_draw_path3(scale: f32, spread: f32) -> f32 {
        rand::bridge::op_spline_draw_path3(&mut mem(), scale, spread)
    }
    /// `op_spline_xyz_table(x, pts, n)`.
    op_spline_xyz_table @ 0x007e_a2d0 => extern "C" fn op_spline_xyz_table(x: u32, pts: u32, n: u32) {
        op::bridge::op_spline_xyz_table(&mut mem(), x, pts, n);
    }
    /// `op_anm_tex0_reset()`.
    op_anm_tex0_reset @ 0x007e_cf30 => extern "C" fn op_anm_tex0_reset() {
        op::bridge::op_anm_tex0_reset(&mut mem(), &mut |_, f, a| call_ptr(f, a));
    }
    /// `op_event_effect_init()`.
    op_event_effect_init @ 0x007f_cc20 => extern "C" fn op_event_effect_init() {
        op::bridge::op_event_effect_init(&mut mem(), &mut |_, f, a| call_ptr(f, a));
    }
    /// `op_event_effect_free()`.
    op_event_effect_free @ 0x007f_cbf0 => extern "C" fn op_event_effect_free() {
        op::bridge::op_event_effect_free(&mut mem(), &mut |_, f, a| call_ptr(f, a));
    }
    /// `op_oef2_tex_clut_reset2(oef)`.
    op_oef2_tex_clut_reset2 @ 0x007e_3e70 => extern "C" fn op_oef2_tex_clut_reset2(oef: u32) {
        op::bridge::op_oef2_tex_clut_reset2(&mut mem(), oef, &mut |_, f, a| call_ptr(f, a));
    }
    /// `op_num_pal_default()`.
    op_num_pal_default @ 0x007e_64b0 => extern "C" fn op_num_pal_default() {
        op::bridge::op_num_pal_default(&mut mem(), &mut |_, f, a| call_ptr(f, a));
    }
    /// `op_num_init(x)`.
    op_num_init @ 0x007e_6480 => extern "C" fn op_num_init(x: u32) {
        op::bridge::op_num_forward(&mut mem(), 0x00a5_dda0, x, &mut |_, f, a| call_ptr(f, a));
    }
    /// `op_num_pal_set(x)`.
    op_num_pal_set @ 0x007e_64d0 => extern "C" fn op_num_pal_set(x: u32) {
        op::bridge::op_num_forward(&mut mem(), 0x00a5_de60, x, &mut |_, f, a| call_ptr(f, a));
    }
    /// `opus_hd_ptr_get_00803200(base, i)`.
    opus_hd_ptr_get_00803200 @ 0x0080_3200 => extern "C" fn opus_hd_ptr_get(base: u32, i: u32) -> u32 {
        opus::bridge::opus_hd_ptr_get(&mut mem(), base, i, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_part_init_check(part)`.
    op_part_init_check @ 0x0080_b650 => extern "C" fn op_part_init_check(part: u32) -> u32 {
        opus::bridge::op_part_init_check(&mut mem(), part, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opus_ffxfocus_off()`.
    opus_ffxfocus_off @ 0x0080_abe0 => extern "C" fn opus_ffxfocus_off() -> u32 {
        opus::bridge::opus_ffxfocus_off(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `opus_ffxfocus_free()`.
    opus_ffxfocus_free @ 0x0080_abb0 => extern "C" fn opus_ffxfocus_free() {
        opus::bridge::opus_ffxfocus_free(&mut mem(), &mut |_, f, a| call_ptr(f, a));
    }
    /// `opus_chr_org_set_008053a0(chr, m)`.
    opus_chr_org_set_008053a0 @ 0x0080_53a0 => extern "C" fn opus_chr_org_set(chr: u32, m: u32) {
        opus::bridge::opus_chr_org_set(&mut mem(), chr, m, &mut |_, f, a| call_ptr(f, a));
    }
    /// `opus_copy_buffer_event_00808a60()`.
    opus_copy_buffer_event_00808a60 @ 0x0080_8a60 => extern "C" fn opus_copy_buffer_event() {
        opus::bridge::opus_copy_buffer_event(&mut mem(), &mut |_, f, a| call_ptr(f, a));
    }
    /// `opus_all_clut_dispose()`.
    opus_all_clut_dispose @ 0x0080_8a20 => extern "C" fn opus_all_clut_dispose() {
        opus::bridge::opus_all_clut_dispose(&mut mem(), &mut |_, f, a| call_ptr(f, a));
    }
    /// `opus_clip_on_all_in_battle()`.
    opus_clip_on_all_in_battle @ 0x0080_0be0 => extern "C" fn opus_clip_on_all_in_battle() {
        opus::bridge::opus_clip_on_all_in_battle(&mut mem(), &mut |_, f, a| call_ptr(f, a));
    }
    /// `opus_chr_thickness_00800c90(chr, battle)`.
    opus_chr_thickness_00800c90 @ 0x0080_0c90 => extern "C" fn opus_chr_thickness(chr: u32, battle: u32) -> f32 {
        opus::bridge::opus_chr_thickness(&mut mem(), chr, battle, &mut |_, f, a| call_ptr_f64(f, a))
    }
    /// `opus_ffxdeg_to_normaldeg(x)`.
    opus_ffxdeg_to_normaldeg @ 0x0081_b170 => extern "C" fn opus_ffxdeg_to_normaldeg(x: f32) -> f32 {
        opus::bridge::opus_ffxdeg_to_normaldeg(&mem(), x)
    }
    /// `opus_normaldeg_to_ffxdeg_0081b230(x)`.
    opus_normaldeg_to_ffxdeg_0081b230 @ 0x0081_b230 => extern "C" fn opus_normaldeg_to_ffxdeg(x: f32) -> f32 {
        opus::bridge::opus_normaldeg_to_ffxdeg(&mem(), x)
    }
    /// `opus_set_chr_dirf(chr, dir)`.
    opus_set_chr_dirf @ 0x0081_b270 => extern "C" fn opus_set_chr_dirf(chr: u32, dir: u32) -> u32 {
        opus::bridge::opus_set_chr_dirf(&mut mem(), chr, dir, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opus_set_chr_pos(chr, pos)`.
    opus_set_chr_pos @ 0x0081_b2b0 => extern "C" fn opus_set_chr_pos(chr: u32, pos: u32) -> u32 {
        opus::bridge::opus_set_chr_pos(&mut mem(), chr, pos, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opus_encount4_size_get(oef, first, n)`.
    opus_encount4_size_get @ 0x007f_8e30 => extern "C" fn opus_encount4_size_get(oef: u32, first: u32, n: i32) -> u32 {
        opus::bridge::opus_encount4_size_get(&mem(), oef, first, n)
    }
    /// `pppSetPartMatrix(camera, screen)`.
    pppSetPartMatrix @ 0x0076_2550 => extern "C" fn ppp_set_part_matrix(camera: u32, screen: u32) -> u32 {
        part::bridge::ppp_set_part_matrix(&mut mem(), camera, screen)
    }
    /// `pppGetPartMatrix(camera, screen)`.
    pppGetPartMatrix @ 0x0076_1c40 => extern "C" fn ppp_get_part_matrix(camera: u32, screen: u32) -> u32 {
        part::bridge::ppp_get_part_matrix(&mut mem(), camera, screen, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppOtFunc_InitEnv(list)`.
    pppOtFunc_InitEnv @ 0x0071_6b70 => extern "C" fn ppp_ot_func_init_env(list: u32) -> u32 {
        part::bridge::ppp_ot_func_init_env(&mut mem(), list, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppFpInitHeapDisplay(heap)`.
    pppFpInitHeapDisplay @ 0x0072_99e0 => extern "C" fn ppp_fp_init_heap_display(heap: u32) {
        fp::bridge::ppp_fp_init_heap_display(&mut mem(), heap);
    }
    /// `Vec4SubScalars_0074bed0(dst, src, x, y, z, w)`: the same code as
    /// `Vec4SubScalars_0074be90`.
    Vec4SubScalars_0074bed0 @ 0x0074_bed0 => extern "C" fn vec4_sub_scalars_2(dst: u32, src: u32, x: f32, y: f32, z: f32, w: f32) {
        vec::bridge::vec4_sub_scalars(&mut mem(), dst, src, [x, y, z, w]);
    }
    /// `op_chr_status_reset_exception(h)`.
    op_chr_status_reset_exception @ 0x007f_ae40 => extern "C" fn op_chr_status_reset_exception(h: u32) {
        et::bridge::op_chr_status_reset_exception(&mut mem(), h, &mut |_, f, a| call_ptr(f, a));
    }
    /// `opus_check_forced_draw()`.
    opus_check_forced_draw @ 0x007f_e4e0 => extern "C" fn opus_check_forced_draw() -> u32 {
        et::bridge::opus_check_forced_draw(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `opus_change_chrhd_battle_to_ffx()`.
    opus_change_chrhd_battle_to_ffx @ 0x007f_e1a0 => extern "C" fn opus_change_chrhd_battle_to_ffx() -> u32 {
        // the three remaps' index map and flag copies
        let mut frame = super::StackFrame::<0xf0>::new();
        let at = frame.at();
        et::bridge::opus_change_chrhd_battle_to_ffx(&mut mem(), at)
    }
    /// `op_effect_run_before()`.
    op_effect_run_before @ 0x0080_0610 => extern "C" fn op_effect_run_before() -> u32 {
        op::bridge::op_effect_run_before(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_anm_tex0_set_007ecf50(_, a, b)`.
    op_anm_tex0_set_007ecf50 @ 0x007e_cf50 => extern "C" fn op_anm_tex0_set(unused: u32, a: u32, b: u32) -> u32 {
        let _ = unused;
        op::bridge::op_anm_tex0_set(&mut mem(), a, b, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_anm_tex0_set_007ecfa0(_, a, b)`: the same code.
    op_anm_tex0_set_007ecfa0 @ 0x007e_cfa0 => extern "C" fn op_anm_tex0_set_2(unused: u32, a: u32, b: u32) -> u32 {
        let _ = unused;
        op::bridge::op_anm_tex0_set(&mut mem(), a, b, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_rt_m_xzy(m, src, angles, t)`.
    op_rt_m_xzy @ 0x007e_99d0 => extern "C" fn op_rt_m_xzy(m: u32, src: u32, angles: u32, t: u32) -> u32 {
        op::bridge::op_rt_m_order(&mut mem(), m, src, angles, t, [mat::Axis::X, mat::Axis::Z, mat::Axis::Y])
    }
    /// `op_rt_m_yzx(m, src, angles, t)`.
    op_rt_m_yzx @ 0x007e_9a90 => extern "C" fn op_rt_m_yzx(m: u32, src: u32, angles: u32, t: u32) -> u32 {
        op::bridge::op_rt_m_order(&mut mem(), m, src, angles, t, [mat::Axis::Y, mat::Axis::Z, mat::Axis::X])
    }
    /// `op_ivec2_scale_007ecc40(_, v)`.
    op_ivec2_scale_007ecc40 @ 0x007e_cc40 => extern "C" fn op_ivec2_scale(unused: u32, v: u32) {
        let _ = unused;
        op::bridge::op_ivec2_scale(&mut mem(), v, 0x00b5_0190);
    }
    /// `op_ivec2_scale_007ecca0(_, v)`.
    op_ivec2_scale_007ecca0 @ 0x007e_cca0 => extern "C" fn op_ivec2_scale_2(unused: u32, v: u32) {
        let _ = unused;
        op::bridge::op_ivec2_scale(&mut mem(), v, 0x00b9_21e0);
    }
    /// `op_view_screen_set_007ec990()`.
    op_view_screen_set_007ec990 @ 0x007e_c990 => extern "C" fn op_view_screen_set() {
        op::bridge::op_view_screen_set(&mut mem(), &mut |_, f, a| call_ptr(f, a));
    }
    /// `opus_mdc_rec_init_00813950(rec, n, a, b)`.
    opus_mdc_rec_init_00813950 @ 0x0081_3950 => extern "C" fn opus_mdc_rec_init(rec: u32, n: u32, a: u32, b: u32) -> u32 {
        opus::bridge::opus_mdc_rec_init(&mut mem(), rec, n, a, b, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_chr_status_reset_exception_wrap_007fae90(h)`.
    op_chr_status_reset_exception_wrap_007fae90 @ 0x007f_ae90 => extern "C" fn op_chr_status_reset_exception_wrap(h: u32) {
        et::bridge::op_chr_status_reset_exception_wrap(&mut mem(), h, &mut |_, f, a| call_ptr(f, a));
    }
    /// `pppFpGrpForceOff(group)`.
    pppFpGrpForceOff @ 0x0072_9480 => extern "C" fn ppp_fp_grp_force_off(group: u32) {
        #[allow(clippy::cast_possible_truncation)] // a u16 argument
        fp::bridge::ppp_fp_grp_force_off(&mut mem(), group as u16, &mut |_, f, a| call_ptr(f, a));
    }
    /// `opu_mdc_0x1000_hide()`.
    opu_mdc_0x1000_hide @ 0x0080_4150 => extern "C" fn opu_mdc_0x1000_hide() -> u32 {
        opus::bridge::opu_mdc_0x1000_hide(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `opus_change_chrhd_ffx_to_battle()`.
    opus_change_chrhd_ffx_to_battle @ 0x007f_e200 => extern "C" fn opus_change_chrhd_ffx_to_battle() -> u32 {
        // the three remaps' index map and flag copies
        let mut frame = super::StackFrame::<0xf0>::new();
        let at = frame.at();
        et::bridge::opus_change_chrhd_ffx_to_battle(&mut mem(), at, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_rt_m_zxy(m, src, angles, t)`.
    op_rt_m_zxy @ 0x007e_9af0 => extern "C" fn op_rt_m_zxy(m: u32, src: u32, angles: u32, t: u32) -> u32 {
        op::bridge::op_rt_m_order(&mut mem(), m, src, angles, t, [mat::Axis::Z, mat::Axis::X, mat::Axis::Y])
    }
    /// `opus_map_rgba_free()`.
    opus_map_rgba_free @ 0x0080_ac00 => extern "C" fn opus_map_rgba_free() {
        // the original's local vector, 16-byte aligned as in its frame
        #[repr(C, align(16))]
        struct Locals([u32; 4]);
        let mut locals = Locals([0; 4]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let at = locals.0.as_mut_ptr() as u32;
        opus::bridge::opus_map_rgba_free(&mut mem(), at, &mut |_, f, a| call_ptr(f, a));
        std::hint::black_box(&mut locals);
    }
    /// `op_vec3_max_007ecd00(out, v, a)`, `a` by value.
    op_vec3_max_007ecd00 @ 0x007e_cd00 => extern "C" fn op_vec3_max(out: u32, v: u32, a0: u32, a1: u32, a2: u32, a3: u32) {
        op::bridge::op_vec3_clamp(&mut mem(), out, v, [a0, a1, a2, a3], true);
    }
    /// `op_vec3_min_007ecdc0(out, v, a)`, `a` by value.
    op_vec3_min_007ecdc0 @ 0x007e_cdc0 => extern "C" fn op_vec3_min(out: u32, v: u32, a0: u32, a1: u32, a2: u32, a3: u32) {
        op::bridge::op_vec3_clamp(&mut mem(), out, v, [a0, a1, a2, a3], false);
    }
    /// `op_file_puts_007e2fb0(file, s)`: the caller's eax for an empty
    /// string is not kept (0); nothing reads it there.
    op_file_puts_007e2fb0 @ 0x007e_2fb0 => extern "C" fn op_file_puts(file: u32, s: u32) -> u32 {
        op::bridge::op_file_puts(&mut mem(), file, s, &mut |_, f, a| call_ptr(f, a)).unwrap_or(0)
    }
    /// `op_effect_init()`.
    op_effect_init @ 0x0080_0580 => extern "C" fn op_effect_init() -> u32 {
        op::bridge::op_effect_init(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `ppp_mdl_rev_random_flags_00740960(rec, out)`.
    ppp_mdl_rev_random_flags_00740960 @ 0x0074_0960 => extern "C" fn ppp_mdl_rev_random_flags(rec: u32, out: u32) {
        mdl::bridge::ppp_mdl_rev_random_flags(&mut mem(), rec, out, &mut |_, f, a| call_ptr_f64(f, a));
    }
    /// `ppp_screen_to_world_007626d0(out, p)`.
    ppp_screen_to_world_007626d0 @ 0x0076_26d0 => extern "C" fn ppp_screen_to_world(out: u32, p: u32) {
        // the original's inverse camera matrix and view point
        let mut frame = super::StackFrame::<0x14>::new();
        let at = frame.at();
        part::bridge::ppp_screen_to_world(&mut mem(), out, p, at, &mut |_, f, a| call_ptr(f, a));
    }
    /// `op_rt_m_6(m, src, angles, t, order)`.
    op_rt_m_6 @ 0x007e_98b0 => extern "C" fn op_rt_m_6(m: u32, src: u32, angles: u32, t: u32, order: u32) -> u32 {
        op::bridge::op_rt_m_6(&mut mem(), [m, src, angles, t], order, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_ovl_effect_free()`.
    op_ovl_effect_free @ 0x007f_cd60 => extern "C" fn op_ovl_effect_free() -> u32 {
        op::bridge::op_ovl_effect_free(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_oef2_clut_cue(rec, mask)`.
    op_oef2_clut_cue @ 0x007e_4610 => extern "C" fn op_oef2_clut_cue(rec: u32, mask: u32) -> u32 {
        op::bridge::op_oef2_clut_cue(&mut mem(), rec, mask, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_et_chr_reset_all_007fee10()`.
    op_et_chr_reset_all_007fee10 @ 0x007f_ee10 => extern "C" fn op_et_chr_reset_all() -> u32 {
        et::bridge::op_et_chr_reset_all(&mut mem())
    }
    /// `op_et_chr_reset_007feec0(i)`: past 0xf0 the caller's eax is not
    /// kept (0); its only caller returns right after with it unused.
    op_et_chr_reset_007feec0 @ 0x007f_eec0 => extern "C" fn op_et_chr_reset(i: u32) -> u32 {
        et::bridge::op_et_chr_reset(&mut mem(), i).unwrap_or(0)
    }
    /// `op_et_astatus_effect_off_all()`.
    op_et_astatus_effect_off_all @ 0x007f_b0a0 => extern "C" fn op_et_astatus_effect_off_all() {
        et::bridge::op_et_astatus_effect_off_all(&mut mem(), &mut |_, f, a| call_ptr(f, a));
    }
    /// `op_et_bindeff2_walk(h, speed)`: without event effects or a handle
    /// the caller's eax is not kept (0).
    op_et_bindeff2_walk @ 0x007f_b740 => extern "C" fn op_et_bindeff2_walk(h: u32, speed: u32) -> u32 {
        et::bridge::op_et_bindeff2_walk(&mut mem(), h, speed, &mut |_, f, a| call_ptr(f, a)).unwrap_or(0)
    }
    /// `opu_mdc_0x2000_ambient(rec)`.
    opu_mdc_0x2000_ambient @ 0x0080_41c0 => extern "C" fn opu_mdc_0x2000_ambient(rec: u32) -> u32 {
        opus::bridge::opu_mdc_0x2000_ambient(&mut mem(), rec, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_oef2_tex_cue_007e4f20(rec, mask)`.
    op_oef2_tex_cue_007e4f20 @ 0x007e_4f20 => extern "C" fn op_oef2_tex_cue(rec: u32, mask: u32) -> u32 {
        op::bridge::op_oef2_tex_cue(&mut mem(), rec, mask, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_token_line_007e1c30(file, src, out, pos, max, eol, flags)`.
    op_token_line_007e1c30 @ 0x007e_1c30 => extern "C" fn op_token_line(file: u32, src: u32, out: u32, pos: u32, max: u32, eol: u32, flags: u32) -> u32 {
        op::bridge::op_token_line(&mut mem(), [file, src, out, pos, max, eol, flags], &mut |_, f, a| call_ptr(f, a))
    }
    /// `opus_get_eff_status_index(id, kind, create)`.
    opus_get_eff_status_index @ 0x007f_eba0 => extern "C" fn opus_get_eff_status_index(id: u32, kind: u32, create: u32) -> u32 {
        // the original's count local
        let mut frame = super::StackFrame::<1>::new();
        let at = frame.at();
        #[allow(clippy::cast_possible_truncation)] // a byte argument
        et::bridge::opus_get_eff_status_index(&mut mem(), id, kind as u8, create, at, &mut |_, f, a| call_ptr(f, a))
    }
    /// `vu_vec4_add_0074bf10(out, a, b)`.
    vu_vec4_add_0074bf10 @ 0x0074_bf10 => extern "C" fn vu_vec4_add_0074bf10(out: u32, a: u32, b: u32) -> u32 {
        vec::bridge::vu_vec4_add(&mut mem(), out, a, b)
    }
    /// `vu_vec4_add_0074cef0(out, a, b)`.
    vu_vec4_add_0074cef0 @ 0x0074_cef0 => extern "C" fn vu_vec4_add_0074cef0(out: u32, a: u32, b: u32) -> u32 {
        vec::bridge::vu_vec4_add(&mut mem(), out, a, b)
    }
    /// `vu_vec4_add_0074def0(out, a, b)`.
    vu_vec4_add_0074def0 @ 0x0074_def0 => extern "C" fn vu_vec4_add_0074def0(out: u32, a: u32, b: u32) -> u32 {
        vec::bridge::vu_vec4_add(&mut mem(), out, a, b)
    }
    /// `vu_vec4_add_0074ef50(out, a, b)`.
    vu_vec4_add_0074ef50 @ 0x0074_ef50 => extern "C" fn vu_vec4_add_0074ef50(out: u32, a: u32, b: u32) -> u32 {
        vec::bridge::vu_vec4_add(&mut mem(), out, a, b)
    }
    /// `vu_vec4_add_00753230(out, a, b)`.
    vu_vec4_add_00753230 @ 0x0075_3230 => extern "C" fn vu_vec4_add_00753230(out: u32, a: u32, b: u32) -> u32 {
        vec::bridge::vu_vec4_add(&mut mem(), out, a, b)
    }
    /// `vu_vec4_sub_0074c3b0(out, a, b)`.
    vu_vec4_sub_0074c3b0 @ 0x0074_c3b0 => extern "C" fn vu_vec4_sub_0074c3b0(out: u32, a: u32, b: u32) -> u32 {
        vec::bridge::vu_vec4_sub(&mut mem(), out, a, b)
    }
    /// `vu_vec4_sub_0074d3b0(out, a, b)`.
    vu_vec4_sub_0074d3b0 @ 0x0074_d3b0 => extern "C" fn vu_vec4_sub_0074d3b0(out: u32, a: u32, b: u32) -> u32 {
        vec::bridge::vu_vec4_sub(&mut mem(), out, a, b)
    }
    /// `vu_vec4_sub_0074e3b0(out, a, b)`.
    vu_vec4_sub_0074e3b0 @ 0x0074_e3b0 => extern "C" fn vu_vec4_sub_0074e3b0(out: u32, a: u32, b: u32) -> u32 {
        vec::bridge::vu_vec4_sub(&mut mem(), out, a, b)
    }
    /// `vu_vec4_sub_0074f3f0(out, a, b)`.
    vu_vec4_sub_0074f3f0 @ 0x0074_f3f0 => extern "C" fn vu_vec4_sub_0074f3f0(out: u32, a: u32, b: u32) -> u32 {
        vec::bridge::vu_vec4_sub(&mut mem(), out, a, b)
    }
    /// `vu_vec4_sub_007501d0(out, a, b)`.
    vu_vec4_sub_007501d0 @ 0x0075_01d0 => extern "C" fn vu_vec4_sub_007501d0(out: u32, a: u32, b: u32) -> u32 {
        vec::bridge::vu_vec4_sub(&mut mem(), out, a, b)
    }
    /// `vu_vec4_sub_007519d0(out, a, b)`.
    vu_vec4_sub_007519d0 @ 0x0075_19d0 => extern "C" fn vu_vec4_sub_007519d0(out: u32, a: u32, b: u32) -> u32 {
        vec::bridge::vu_vec4_sub(&mut mem(), out, a, b)
    }
    /// `vu_vec4_sub_00753db0(out, a, b)`.
    vu_vec4_sub_00753db0 @ 0x0075_3db0 => extern "C" fn vu_vec4_sub_00753db0(out: u32, a: u32, b: u32) -> u32 {
        vec::bridge::vu_vec4_sub(&mut mem(), out, a, b)
    }
    /// `ppp_obj_rot_pos_0074a760(rot, pos, r)`.
    ppp_obj_rot_pos_0074a760 @ 0x0074_a760 => extern "C" fn ppp_obj_rot_pos_0074a760(rot: u32, pos: u32, r: u32) -> u32 {
        part::bridge::ppp_obj_rot_pos(&mut mem(), rot, pos, r)
    }
    /// `ppp_obj_rot_pos_0074ad80(rot, pos, r)`.
    ppp_obj_rot_pos_0074ad80 @ 0x0074_ad80 => extern "C" fn ppp_obj_rot_pos_0074ad80(rot: u32, pos: u32, r: u32) -> u32 {
        part::bridge::ppp_obj_rot_pos(&mut mem(), rot, pos, r)
    }
    /// `ppp_obj_rot_pos_0074b540(rot, pos, r)`.
    ppp_obj_rot_pos_0074b540 @ 0x0074_b540 => extern "C" fn ppp_obj_rot_pos_0074b540(rot: u32, pos: u32, r: u32) -> u32 {
        part::bridge::ppp_obj_rot_pos(&mut mem(), rot, pos, r)
    }
    /// `hideDisplayOffParticle(part)`.
    hideDisplayOffParticle @ 0x0072_8e40 => extern "C" fn hide_display_off_particle(part: u32) {
        part::bridge::hide_display_off_particle(&mut mem(), part, &mut |_, f, a| call_ptr(f, a));
    }
    /// `op_et_battle_target_center_0081b300(out)`.
    op_et_battle_target_center_0081b300 @ 0x0081_b300 => extern "C" fn op_et_battle_target_center(out: u32) {
        // the original's count local
        let mut frame = super::StackFrame::<1>::new();
        let at = frame.at();
        et::bridge::op_et_battle_target_center(&mut mem(), out, at, &mut |_, f, a| call_ptr(f, a));
    }
    /// `op_et_battle_effect_init()`.
    op_et_battle_effect_init @ 0x007f_b1e0 => extern "C" fn op_et_battle_effect_init() -> u32 {
        et::bridge::op_et_battle_effect_init(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_et_monster_free(h)`.
    op_et_monster_free @ 0x007f_c650 => extern "C" fn op_et_monster_free(h: u32) {
        et::bridge::op_et_monster_free(&mut mem(), h, &mut |_, f, a| call_ptr(f, a));
    }
    /// `op_mdc_mem_transfer()`.
    op_mdc_mem_transfer @ 0x0080_3610 => extern "C" fn op_mdc_mem_transfer() {
        op::bridge::op_mdc_mem_transfer(&mut mem(), &mut |_, f, a| call_ptr(f, a));
    }
    /// `op_oef2_tex_clut_cue_loop()`.
    op_oef2_tex_clut_cue_loop @ 0x007e_4720 => extern "C" fn op_oef2_tex_clut_cue_loop() {
        op::bridge::op_oef2_tex_clut_cue_loop(&mut mem(), &mut |_, f, a| call_ptr(f, a));
    }
    /// `op_oef2_tex_clut_cue_multi(oef, first, last, mask)`.
    op_oef2_tex_clut_cue_multi @ 0x007e_4830 => extern "C" fn op_oef2_tex_clut_cue_multi(oef: u32, first: u32, last: u32, mask: u32) -> u32 {
        op::bridge::op_oef2_tex_clut_cue_multi(&mut mem(), oef, first, last, mask, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opu_et_unit(oef, group, part, id)`.
    opu_et_unit @ 0x007f_d7f0 => extern "C" fn opu_et_unit(oef: u32, group: u32, part: u32, id: u32) -> u32 {
        et::bridge::opu_et_unit(&mut mem(), oef, group, part, id, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppFpPointLightProc()`.
    pppFpPointLightProc @ 0x0072_9f90 => extern "C" fn ppp_fp_point_light_proc() -> u32 {
        fp::bridge::ppp_fp_point_light_proc(&mut mem(), &mut |_, f, a| call_ptr(f, a), &mut |_, f, a| call_ptr_f64(f, a))
    }
    /// `op_loop_ffx_after()`.
    op_loop_ffx_after @ 0x007f_f680 => extern "C" fn op_loop_ffx_after() {
        // the original's locals (the frame number, then the GS environment
        // at a 16-byte boundary, as its aligned frame has them): with an
        // unaligned environment the game crashed in the replays
        #[repr(C, align(16))]
        struct Locals([u32; 0x80]);
        let mut locals = Locals([0; 0x80]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let at = locals.0.as_mut_ptr() as u32 + 0xc;
        op::bridge::op_loop_ffx_after(&mut mem(), at, &mut |_, f, a| call_ptr(f, a));
        std::hint::black_box(&mut locals);
    }
    /// `opf_normal(host, ptr)`.
    opf_normal @ 0x0081_5050 => extern "C" fn opf_normal(host: u32, ptr: u32) -> u32 {
        opus::bridge::opf_normal(&mut mem(), host, ptr)
    }
    /// `opu_eff_flag_008178f0(host, cmd)`.
    opu_eff_flag_008178f0 @ 0x0081_78f0 => extern "C" fn opu_eff_flag(host: u32, cmd: u32) -> u32 {
        opus::bridge::opu_eff_flag(&mut mem(), host, cmd)
    }
    /// `opu_v_rev(host, cmd)`.
    opu_v_rev @ 0x0081_9a20 => extern "C" fn opu_v_rev(host: u32, cmd: u32) -> u32 {
        opus::bridge::opu_v_rev(&mut mem(), host, cmd)
    }
    /// `opu_v_stop_008182f0(host, cmd)`.
    opu_v_stop_008182f0 @ 0x0081_82f0 => extern "C" fn opu_v_stop(host: u32, cmd: u32) -> u32 {
        opus::bridge::opu_v_stop(&mut mem(), host, cmd)
    }
    /// `opu_motion_cb_0081b4d0(host, cmd)`.
    opu_motion_cb_0081b4d0 @ 0x0081_b4d0 => extern "C" fn opu_motion_cb(host: u32, cmd: u32) -> u32 {
        opus::bridge::opu_motion_cb(&mut mem(), host, cmd)
    }
    /// `opus_prog_on(set, list)`.
    opus_prog_on @ 0x0080_1700 => extern "C" fn opus_prog_on(set: u32, list: u32) -> u32 {
        opus::bridge::opus_prog_on(&mut mem(), set, list)
    }
    /// `opus_list_add_00801780(set, id, list)`.
    opus_list_add_00801780 @ 0x0080_1780 => extern "C" fn opus_list_add(set: u32, id: u32, list: u32) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // the u16 argument
        opus::bridge::opus_list_add(&mut mem(), set, id as u16, list)
    }
    /// `opu_main_host(battle, list)`.
    opu_main_host @ 0x0080_cc00 => extern "C" fn opu_main_host(battle: u32, list: u32) -> u32 {
        opus::bridge::opu_main_host(&mut mem(), battle, list, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opus_wave_omd_make(obj, wave, uv, omd, view, light)`: its stack
    /// locals in a buffer on this shim's stack (the frame pointer at its
    /// end).
    opus_wave_omd_make @ 0x007f_94e0 => extern "C" fn opus_wave_omd_make(obj: u32, wv: u32, uv: u32, omd: u32, view: u32, light: u32) -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0x90]);
        let mut locals = Locals([0; 0x90]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0x90;
        wave::bridge::opus_wave_omd_make(&mut mem(), obj, wv, uv, omd, view, light, frame, &mut |_, f, a| call_ptr(f, a), &mut |_, f, a| call_ptr_f64(f, a))
    }
    /// `opus_0154_wave_omd_make(obj, wave, uv, omd, view, proj)`: its
    /// stack locals in a buffer on this shim's stack (the frame pointer at
    /// its end).
    opus_0154_wave_omd_make @ 0x007f_67c0 => extern "C" fn opus_0154_wave_omd_make(obj: u32, wv: u32, uv: u32, omd: u32, view: u32, proj: u32) -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0xf0]);
        let mut locals = Locals([0; 0xf0]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0xf0;
        wave::bridge::opus_0154_wave_omd_make(&mut mem(), obj, wv, uv, omd, view, proj, frame, &mut |_, f, a| call_ptr(f, a), &mut |_, f, a| call_ptr_f64(f, a), &mut |_, x| crt_tan(x))
    }
    /// `pppDrawShapeFieldCon(obj, prog)`.
    pppDrawShapeFieldCon @ 0x0074_2b50 => extern "C" fn ppp_draw_shape_field_con(obj: u32, prog: u32) -> u32 {
        draw::bridge::shape_con(&mut mem(), obj, prog, draw::bridge::FIELD_CON, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppDrawShapeFieldRevCon(obj, prog)`.
    pppDrawShapeFieldRevCon @ 0x0074_3f00 => extern "C" fn ppp_draw_shape_field_rev_con(obj: u32, prog: u32) -> u32 {
        draw::bridge::shape_con(&mut mem(), obj, prog, draw::bridge::FIELD_REV_CON, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppDrawShapeFieldSpdCon(obj, prog)`.
    pppDrawShapeFieldSpdCon @ 0x0074_50a0 => extern "C" fn ppp_draw_shape_field_spd_con(obj: u32, prog: u32) -> u32 {
        draw::bridge::shape_con(&mut mem(), obj, prog, draw::bridge::FIELD_SPD_CON, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppDrawShapeFieldGlobalCon(obj, prog)`.
    pppDrawShapeFieldGlobalCon @ 0x0074_6200 => extern "C" fn ppp_draw_shape_field_global_con(obj: u32, prog: u32) -> u32 {
        draw::bridge::shape_con(&mut mem(), obj, prog, draw::bridge::FIELD_GLOBAL_CON, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppDrawShapeCameraCon(obj, prog)`.
    pppDrawShapeCameraCon @ 0x0074_7410 => extern "C" fn ppp_draw_shape_camera_con(obj: u32, prog: u32) -> u32 {
        draw::bridge::shape_con(&mut mem(), obj, prog, draw::bridge::CAMERA_CON, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppDrawShapeCameraDisPosCon(obj, prog)`.
    pppDrawShapeCameraDisPosCon @ 0x0074_8660 => extern "C" fn ppp_draw_shape_camera_dis_pos_con(obj: u32, prog: u32) -> u32 {
        draw::bridge::shape_con(&mut mem(), obj, prog, draw::bridge::CAMERA_CON, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppDrawShapeFieldDes(obj, prog)`.
    pppDrawShapeFieldDes @ 0x0074_2bd0 => extern "C" fn ppp_draw_shape_field_des(obj: u32, prog: u32) -> u32 {
        draw::bridge::ppp_des(&mut mem(), obj, prog, draw::bridge::FIELD_DES, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppDrawShapeFieldRevDes(obj, prog)`.
    pppDrawShapeFieldRevDes @ 0x0074_3f80 => extern "C" fn ppp_draw_shape_field_rev_des(obj: u32, prog: u32) -> u32 {
        draw::bridge::ppp_des(&mut mem(), obj, prog, draw::bridge::FIELD_REV_DES, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppDrawShapeFieldSpdDes(obj, prog)`.
    pppDrawShapeFieldSpdDes @ 0x0074_5130 => extern "C" fn ppp_draw_shape_field_spd_des(obj: u32, prog: u32) -> u32 {
        draw::bridge::ppp_des(&mut mem(), obj, prog, draw::bridge::FIELD_SPD_DES, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppDrawShapeFieldGlobalDes(obj, prog)`.
    pppDrawShapeFieldGlobalDes @ 0x0074_6280 => extern "C" fn ppp_draw_shape_field_global_des(obj: u32, prog: u32) -> u32 {
        draw::bridge::ppp_des(&mut mem(), obj, prog, draw::bridge::FIELD_GLOBAL_DES, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppDrawShapeCameraDes(obj, prog)`.
    pppDrawShapeCameraDes @ 0x0074_7490 => extern "C" fn ppp_draw_shape_camera_des(obj: u32, prog: u32) -> u32 {
        draw::bridge::ppp_des(&mut mem(), obj, prog, draw::bridge::CAMERA_DES, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppDrawShapeCameraDisPosDes(obj, prog)`.
    pppDrawShapeCameraDisPosDes @ 0x0074_86e0 => extern "C" fn ppp_draw_shape_camera_dis_pos_des(obj: u32, prog: u32) -> u32 {
        draw::bridge::ppp_des(&mut mem(), obj, prog, draw::bridge::CAMERA_DES, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppDrawShapeFieldAlloc_00742f20(obj, key, prog)`.
    pppDrawShapeFieldAlloc_00742f20 @ 0x0074_2f20 => extern "C" fn ppp_draw_shape_field_alloc(obj: u32, key: u32, prog: u32) -> u32 {
        draw::bridge::shape_alloc(&mut mem(), obj, key, prog, draw::bridge::FIELD_ALLOC, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppDrawShapeFieldRevAlloc_00744260(obj, key, prog)`.
    pppDrawShapeFieldRevAlloc_00744260 @ 0x0074_4260 => extern "C" fn ppp_draw_shape_field_rev_alloc(obj: u32, key: u32, prog: u32) -> u32 {
        draw::bridge::shape_alloc(&mut mem(), obj, key, prog, draw::bridge::FIELD_ALLOC, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppDrawShapeFieldGlobalAlloc_00746540(obj, key, prog)`.
    pppDrawShapeFieldGlobalAlloc_00746540 @ 0x0074_6540 => extern "C" fn ppp_draw_shape_field_global_alloc(obj: u32, key: u32, prog: u32) -> u32 {
        draw::bridge::shape_alloc(&mut mem(), obj, key, prog, draw::bridge::FIELD_GLOBAL_ALLOC, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppDrawShapeCameraAlloc_00747760(obj, key, prog)`.
    pppDrawShapeCameraAlloc_00747760 @ 0x0074_7760 => extern "C" fn ppp_draw_shape_camera_alloc(obj: u32, key: u32, prog: u32) -> u32 {
        draw::bridge::shape_alloc(&mut mem(), obj, key, prog, draw::bridge::CAMERA_ALLOC, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppDrawShapeCameraDisPosAlloc_007489b0(obj, key, prog)`.
    pppDrawShapeCameraDisPosAlloc_007489b0 @ 0x0074_89b0 => extern "C" fn ppp_draw_shape_camera_dis_pos_alloc(obj: u32, key: u32, prog: u32) -> u32 {
        draw::bridge::shape_alloc(&mut mem(), obj, key, prog, draw::bridge::CAMERA_ALLOC, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_token_paren_007e1e90(file, src, out, pos, max, eol)`.
    op_token_paren_007e1e90 @ 0x007e_1e90 => extern "C" fn op_token_paren(file: u32, src: u32, out: u32, pos: u32, max: u32, eol: u32) -> u32 {
        op::bridge::op_token_paren(&mut mem(), [file, src, out, pos, max, eol], &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_init_ffx_state_007ff960()`.
    op_init_ffx_state_007ff960 @ 0x007f_f960 => extern "C" fn op_init_ffx_state() {
        op::bridge::op_init_ffx_state(&mut mem(), &mut |_, f, a| call_ptr(f, a));
    }
    /// `op_xyzw_scale(dst, src, s)`.
    op_xyzw_scale @ 0x007e_a8f0 => extern "C" fn op_xyzw_scale(dst: u32, src: u32, s: u32) -> u32 {
        op::bridge::op_xyzw_scale(&mut mem(), dst, src, s)
    }
    /// `ppp_screen_to_fixed_00761170(out, v)`.
    ppp_screen_to_fixed_00761170 @ 0x0076_1170 => extern "C" fn ppp_screen_to_fixed(out: u32, v: u32) -> u32 {
        part::bridge::ppp_screen_to_fixed(&mut mem(), out, v)
    }
    /// `pppDrawMdlCameraLoopInit_007400f0(rec, area)`.
    pppDrawMdlCameraLoopInit_007400f0 @ 0x0074_00f0 => extern "C" fn ppp_draw_mdl_camera_loop_init(rec: u32, area: u32) {
        mdl::bridge::ppp_draw_mdl_camera_loop_init(&mut mem(), rec, area, &mut |_, f, a| call_ptr(f, a));
    }
    /// `opus_mdc_break_make_VitaSafe(mdc, out, params, view)`.
    opus_mdc_break_make_VitaSafe @ 0x0080_59c0 => extern "C" fn opus_mdc_break_make(mdc: u32, out: u32, params: u32, view: u32) {
        // the original's matrices and image-size locals (0x180 bytes)
        let mut frame = super::StackFrame::<0x60>::new();
        let at = frame.at();
        // the caller's saved edi and esi only reach op_irandom_p's VU0
        // scratch (as anmm's junk words)
        opus::bridge::opus_mdc_break_make(&mut mem(), [mdc, out, params, view], at, [0; 2], &mut |_, f, a| call_ptr(f, a), &mut |_, f, a| call_ptr_f64(f, a));
    }
    /// `pppDrawRainDes(obj, prog)`.
    pppDrawRainDes @ 0x0075_6270 => extern "C" fn ppp_draw_rain_des(obj: u32, prog: u32) -> u32 {
        draw::bridge::ppp_des(&mut mem(), obj, prog, draw::bridge::RAIN_DES, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppDrawMdlDraw(obj, key, prog)`: its stack locals in a buffer on
    /// this shim's stack (the frame pointer at its end).
    pppDrawMdlDraw @ 0x0073_7770 => extern "C" fn ppp_draw_mdl_draw(obj: u32, key: u32, prog: u32) -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0xa0]);
        let mut locals = Locals([0; 0xa0]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0xa0;
        mdl::bridge::ppp_draw_mdl_draw(&mut mem(), obj, key, prog, frame, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppDrawMdlSemiDraw(obj, key, prog)`: its stack locals in a buffer on this
    /// shim's stack (the frame pointer at its end).
    pppDrawMdlSemiDraw @ 0x0073_7b20 => extern "C" fn ppp_draw_mdl_semi_draw(obj: u32, key: u32, prog: u32) -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0xa0]);
        let mut locals = Locals([0; 0xa0]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0xa0;
        mdl::bridge::ppp_draw_mdl_semi_draw(&mut mem(), obj, key, prog, frame, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppDrawMdlTsDraw(obj, key, prog)`: its stack locals in a buffer on this
    /// shim's stack (the frame pointer at its end).
    pppDrawMdlTsDraw @ 0x0073_7f40 => extern "C" fn ppp_draw_mdl_ts_draw(obj: u32, key: u32, prog: u32) -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0xa0]);
        let mut locals = Locals([0; 0xa0]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0xa0;
        mdl::bridge::ppp_draw_mdl_ts_draw(&mut mem(), obj, key, prog, frame, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppDrawMdl2Draw(obj, key, prog)`: its stack locals in a buffer on this
    /// shim's stack (the frame pointer at its end).
    pppDrawMdl2Draw @ 0x0073_8530 => extern "C" fn ppp_draw_mdl_2_draw(obj: u32, key: u32, prog: u32) -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0xa0]);
        let mut locals = Locals([0; 0xa0]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0xa0;
        mdl::bridge::ppp_draw_mdl_2_draw(&mut mem(), obj, key, prog, frame, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppDrawMdlSemi2Draw(obj, key, prog)`: its stack locals in a buffer on this
    /// shim's stack (the frame pointer at its end).
    pppDrawMdlSemi2Draw @ 0x0073_8a40 => extern "C" fn ppp_draw_mdl_semi2_draw(obj: u32, key: u32, prog: u32) -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0xa0]);
        let mut locals = Locals([0; 0xa0]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0xa0;
        mdl::bridge::ppp_draw_mdl_semi2_draw(&mut mem(), obj, key, prog, frame, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppDrawMdlTs2Draw(obj, key, prog)`: its stack locals in a buffer on this
    /// shim's stack (the frame pointer at its end).
    pppDrawMdlTs2Draw @ 0x0073_8ec0 => extern "C" fn ppp_draw_mdl_ts2_draw(obj: u32, key: u32, prog: u32) -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0xa0]);
        let mut locals = Locals([0; 0xa0]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0xa0;
        mdl::bridge::ppp_draw_mdl_ts2_draw(&mut mem(), obj, key, prog, frame, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppDrawMdl3Draw(obj, key, prog)`: its stack locals in a buffer on this
    /// shim's stack (the frame pointer at its end).
    pppDrawMdl3Draw @ 0x0073_94c0 => extern "C" fn ppp_draw_mdl_3_draw(obj: u32, key: u32, prog: u32) -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0x120]);
        let mut locals = Locals([0; 0x120]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0x120;
        mdl::bridge::ppp_draw_mdl_3_draw(&mut mem(), obj, key, prog, frame, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppDrawMdlSemi3Draw(obj, key, prog)`: its stack locals in a buffer on this
    /// shim's stack (the frame pointer at its end).
    pppDrawMdlSemi3Draw @ 0x0073_9d30 => extern "C" fn ppp_draw_mdl_semi3_draw(obj: u32, key: u32, prog: u32) -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0x120]);
        let mut locals = Locals([0; 0x120]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0x120;
        mdl::bridge::ppp_draw_mdl_semi3_draw(&mut mem(), obj, key, prog, frame, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppDrawMdlTs3Draw(obj, key, prog)`: its stack locals in a buffer on this
    /// shim's stack (the frame pointer at its end).
    pppDrawMdlTs3Draw @ 0x0073_a370 => extern "C" fn ppp_draw_mdl_ts3_draw(obj: u32, key: u32, prog: u32) -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0xe0]);
        let mut locals = Locals([0; 0xe0]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0xe0;
        mdl::bridge::ppp_draw_mdl_ts3_draw(&mut mem(), obj, key, prog, frame, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppDrawMdlLoopDraw(obj, key, prog)`: its stack locals in a buffer on this
    /// shim's stack (the frame pointer at its end).
    pppDrawMdlLoopDraw @ 0x0073_e2b0 => extern "C" fn ppp_draw_mdl_loop_draw(obj: u32, key: u32, prog: u32) -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0xe0]);
        let mut locals = Locals([0; 0xe0]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0xe0;
        mdl::bridge::ppp_draw_mdl_loop_draw(&mut mem(), obj, key, prog, frame, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppDrawMdlLoopZDraw(obj, key, prog)`: its stack locals in a buffer on this
    /// shim's stack (the frame pointer at its end).
    pppDrawMdlLoopZDraw @ 0x0073_ea40 => extern "C" fn ppp_draw_mdl_loop_z_draw(obj: u32, key: u32, prog: u32) -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0xe0]);
        let mut locals = Locals([0; 0xe0]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0xe0;
        mdl::bridge::ppp_draw_mdl_loop_z_draw(&mut mem(), obj, key, prog, frame, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppDrawMdlLoopDisPosDraw(obj, key, prog)`: its stack locals in a buffer on this
    /// shim's stack (the frame pointer at its end).
    pppDrawMdlLoopDisPosDraw @ 0x0073_f230 => extern "C" fn ppp_draw_mdl_loop_dis_pos_draw(obj: u32, key: u32, prog: u32) -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0x1a0]);
        let mut locals = Locals([0; 0x1a0]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0x1a0;
        mdl::bridge::ppp_draw_mdl_loop_dis_pos_draw(&mut mem(), obj, key, prog, frame, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppKeLnsArndDraw(obj, key, prog)`: its stack locals in a buffer on this shim's
    /// stack (the frame pointer at its end).
    pppKeLnsArndDraw @ 0x0075_9ff0 => extern "C" fn ppp_ke_lns_arnd_draw(obj: u32, key: u32, prog: u32) -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0x30]);
        let mut locals = Locals([0; 0x30]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0x30;
        lns::bridge::ppp_ke_lns_draw(&mut mem(), lns::bridge::Lns::Arnd, obj, key, prog, frame, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppKeLnsArndTDraw(obj, key, prog)`.
    pppKeLnsArndTDraw @ 0x0075_a190 => extern "C" fn ppp_ke_lns_arnd_t_draw(obj: u32, key: u32, prog: u32) -> u32 {
        lns::bridge::ppp_ke_lns_t_draw(&mut mem(), lns::bridge::Lns::Arnd, obj, key, prog, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppKeLnsArndTOtDraw(area)`: its stack locals in a buffer on this shim's
    /// stack (the frame pointer at its end).
    pppKeLnsArndTOtDraw @ 0x0075_a250 => extern "C" fn ppp_ke_lns_arnd_t_ot_draw(area: u32) -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0x30]);
        let mut locals = Locals([0; 0x30]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0x30;
        lns::bridge::ppp_ke_lns_t_ot_draw(&mut mem(), lns::bridge::Lns::Arnd, area, frame, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppKeLnsCrnDraw(obj, key, prog)`: its stack locals in a buffer on this shim's
    /// stack (the frame pointer at its end).
    pppKeLnsCrnDraw @ 0x0075_a760 => extern "C" fn ppp_ke_lns_crn_draw(obj: u32, key: u32, prog: u32) -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0x30]);
        let mut locals = Locals([0; 0x30]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0x30;
        lns::bridge::ppp_ke_lns_draw(&mut mem(), lns::bridge::Lns::Crn, obj, key, prog, frame, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppKeLnsCrnTDraw(obj, key, prog)`.
    pppKeLnsCrnTDraw @ 0x0075_a900 => extern "C" fn ppp_ke_lns_crn_t_draw(obj: u32, key: u32, prog: u32) -> u32 {
        lns::bridge::ppp_ke_lns_t_draw(&mut mem(), lns::bridge::Lns::Crn, obj, key, prog, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppKeLnsCrnTOtDraw(area)`: its stack locals in a buffer on this shim's
    /// stack (the frame pointer at its end).
    pppKeLnsCrnTOtDraw @ 0x0075_a9e0 => extern "C" fn ppp_ke_lns_crn_t_ot_draw(area: u32) -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0x30]);
        let mut locals = Locals([0; 0x30]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0x30;
        lns::bridge::ppp_ke_lns_t_ot_draw(&mut mem(), lns::bridge::Lns::Crn, area, frame, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppKeLnsFlsDraw(obj, key, prog)`: its stack locals in a buffer on this shim's
    /// stack (the frame pointer at its end).
    pppKeLnsFlsDraw @ 0x0075_ab50 => extern "C" fn ppp_ke_lns_fls_draw(obj: u32, key: u32, prog: u32) -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0x30]);
        let mut locals = Locals([0; 0x30]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0x30;
        lns::bridge::ppp_ke_lns_draw(&mut mem(), lns::bridge::Lns::Fls, obj, key, prog, frame, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppKeLnsFlsTDraw(obj, key, prog)`.
    pppKeLnsFlsTDraw @ 0x0075_ad00 => extern "C" fn ppp_ke_lns_fls_t_draw(obj: u32, key: u32, prog: u32) -> u32 {
        lns::bridge::ppp_ke_lns_t_draw(&mut mem(), lns::bridge::Lns::Fls, obj, key, prog, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppKeLnsFlsTOtDraw(area)`: its stack locals in a buffer on this shim's
    /// stack (the frame pointer at its end).
    pppKeLnsFlsTOtDraw @ 0x0075_ade0 => extern "C" fn ppp_ke_lns_fls_t_ot_draw(area: u32) -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0x30]);
        let mut locals = Locals([0; 0x30]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0x30;
        lns::bridge::ppp_ke_lns_t_ot_draw(&mut mem(), lns::bridge::Lns::Fls, area, frame, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppKeLnsLpDraw(obj, key, prog)`: its stack locals in a buffer on this shim's
    /// stack (the frame pointer at its end).
    pppKeLnsLpDraw @ 0x0075_af80 => extern "C" fn ppp_ke_lns_lp_draw(obj: u32, key: u32, prog: u32) -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0x30]);
        let mut locals = Locals([0; 0x30]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0x30;
        lns::bridge::ppp_ke_lns_lp_draw(&mut mem(), obj, key, prog, frame, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppKeMdlTfdDraw(obj, key, prog)`.
    pppKeMdlTfdDraw @ 0x0074_9810 => extern "C" fn ppp_ke_mdl_tfd_draw(obj: u32, key: u32, prog: u32) -> u32 {
        mdl::bridge::ppp_ke_mdl_tfd_draw(&mut mem(), mdl::bridge::Tfd::One, false, obj, key, prog, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppKeMdlTfd2Draw(obj, key, prog)`.
    pppKeMdlTfd2Draw @ 0x0074_9b40 => extern "C" fn ppp_ke_mdl_tfd2_draw(obj: u32, key: u32, prog: u32) -> u32 {
        mdl::bridge::ppp_ke_mdl_tfd_draw(&mut mem(), mdl::bridge::Tfd::Two, false, obj, key, prog, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppKeMdlTfd3Draw(obj, key, prog)`.
    pppKeMdlTfd3Draw @ 0x0074_9e90 => extern "C" fn ppp_ke_mdl_tfd3_draw(obj: u32, key: u32, prog: u32) -> u32 {
        mdl::bridge::ppp_ke_mdl_tfd_draw(&mut mem(), mdl::bridge::Tfd::Three, false, obj, key, prog, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppKeMdlTfdUvDraw(obj, key, prog)`.
    pppKeMdlTfdUvDraw @ 0x0074_9950 => extern "C" fn ppp_ke_mdl_tfd_uv_draw(obj: u32, key: u32, prog: u32) -> u32 {
        mdl::bridge::ppp_ke_mdl_tfd_draw(&mut mem(), mdl::bridge::Tfd::One, true, obj, key, prog, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppKeMdlTfdUv2Draw(obj, key, prog)`.
    pppKeMdlTfdUv2Draw @ 0x0074_9c90 => extern "C" fn ppp_ke_mdl_tfd_uv2_draw(obj: u32, key: u32, prog: u32) -> u32 {
        mdl::bridge::ppp_ke_mdl_tfd_draw(&mut mem(), mdl::bridge::Tfd::Two, true, obj, key, prog, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppKeMdlTfdUv3Draw(obj, key, prog)`.
    pppKeMdlTfdUv3Draw @ 0x0074_9ff0 => extern "C" fn ppp_ke_mdl_tfd_uv3_draw(obj: u32, key: u32, prog: u32) -> u32 {
        mdl::bridge::ppp_ke_mdl_tfd_draw(&mut mem(), mdl::bridge::Tfd::Three, true, obj, key, prog, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppFpSetDrawMatrix(mode)`.
    pppFpSetDrawMatrix @ 0x0072_a260 => extern "C" fn ppp_fp_set_draw_matrix(mode: u32) -> u32 {
        fp::bridge::ppp_fp_set_draw_matrix(&mut mem(), mode)
    }
    /// `pppDrawMatrixWoodSta()`: its three vectors in a buffer on this
    /// shim's stack (the frame pointer at its end).
    pppDrawMatrixWoodSta @ 0x0073_4d90 => extern "C" fn ppp_draw_matrix_wood_sta() -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0x40]);
        let mut locals = Locals([0; 0x40]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0x40;
        mat::bridge::ppp_draw_matrix_wood_sta(&mut mem(), frame)
    }
    /// `pppKeTkFadeDraw(obj, key, prog)`: its stack locals in a buffer on
    /// this shim's stack (the frame pointer at its end).
    pppKeTkFadeDraw @ 0x0072_ee50 => extern "C" fn ppp_ke_tk_fade_draw(obj: u32, key: u32, prog: u32) -> u32 {
        #[repr(C, align(16))]
        struct Locals([u8; 0xc0]);
        let mut locals = Locals([0; 0xc0]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0xc0;
        draw::bridge::ppp_ke_tk_fade_draw(&mut mem(), obj, key, prog, frame, &mut |_, f, a| call_ptr(f, a))
    }
    // pppKeLnsLpTOtDraw (0x0075b230) is ported (draw::bridge::
    // ppp_ke_lns_lp_t_ot_draw, oracle parity) but deliberately not hooked:
    // on PC its result comes from stale memory (docs/formats/vfx.md,
    // "To observe").
    /// `opu2_part(obj, cmd)`.
    opu2_part @ 0x00a7_4a00 => extern "C" fn opu2_part(obj: u32, cmd: u32) -> u32 {
        slots::bridge::opu2_part(&mut mem(), obj, cmd, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_char_table_set`.
    op_char_table_set @ 0x007e_13e0 => extern "C" fn op_char_table_set(table: u32, chars: u32, v: u32)  {
        small::bridge::op_char_table_set(&mut mem(), table, chars, v);
    }
    /// `op_str_unquote`.
    op_str_unquote @ 0x007e_1b50 => extern "C" fn op_str_unquote(s: u32)  {
        small::bridge::op_str_unquote(&mut mem(), s);
    }
    /// `op_and64`.
    op_and64 @ 0x007e_3040 => extern "C" fn op_and64(dst: u32, a: u32, b: u32)  {
        small::bridge::op_logic64(&mut mem(), dst, a, b, false);
    }
    /// `op_or64`.
    op_or64 @ 0x007e_6c80 => extern "C" fn op_or64(dst: u32, a: u32, b: u32)  {
        small::bridge::op_logic64(&mut mem(), dst, a, b, true);
    }
    /// `op_word_copy3`.
    op_word_copy3 @ 0x007e_3060 => extern "C" fn op_word_copy3(dst: u32, src: u32)  {
        op::bridge::op_word_copy(&mut mem(), dst, src);
    }
    /// `op_word_copy4`.
    op_word_copy4 @ 0x007e_6c70 => extern "C" fn op_word_copy4(dst: u32, src: u32)  {
        op::bridge::op_word_copy(&mut mem(), dst, src);
    }
    /// `op_float_scale`.
    op_float_scale @ 0x007e_3070 => extern "C" fn op_float_scale(dst: u32, v: u32, s: f32)  {
        small::bridge::op_float_step(&mut mem(), dst, v, s, 0, '*');
    }
    /// `op_float_scale2`.
    op_float_scale2 @ 0x0080_2080 => extern "C" fn op_float_scale2(dst: u32, v: u32, s: f32)  {
        small::bridge::op_float_step(&mut mem(), dst, v, s, 0, '*');
    }
    /// `op_float_add`.
    op_float_add @ 0x007e_5950 => extern "C" fn op_float_add(dst: u32, v: u32, s: f32)  {
        small::bridge::op_float_step(&mut mem(), dst, v, s, 0, '+');
    }
    /// `op_float_sub`.
    op_float_sub @ 0x007e_59a0 => extern "C" fn op_float_sub(dst: u32, v: u32, s: f32)  {
        small::bridge::op_float_step(&mut mem(), dst, v, s, 0, '-');
    }
    /// `op_vec_w_sub`.
    op_vec_w_sub @ 0x007e_31c0 => extern "C" fn op_vec_w_sub(dst: u32, v: u32, s: f32)  {
        small::bridge::op_float_step(&mut mem(), dst, v, s, 0xc, '-');
    }
    /// `op_vec_z_scale`.
    op_vec_z_scale @ 0x007e_cea0 => extern "C" fn op_vec_z_scale(dst: u32, v: u32, s: f32)  {
        small::bridge::op_float_step(&mut mem(), dst, v, s, 8, '*');
    }
    /// `op_vec_w_add2`.
    op_vec_w_add2 @ 0x007e_cc00 => extern "C" fn op_vec_w_add2(dst: u32, v: u32, a: u32, b: u32, c: u32, s: f32)  {
        let _ = (a, b, c);
        small::bridge::op_float_step(&mut mem(), dst, v, s, 0xc, '+');
    }
    /// `op_or16_64`.
    op_or16_64 @ 0x007e_30a0 => extern "C" fn op_or16_64(dst: u32, src: u32, v: u32)  {
        small::bridge::op_or16_64(&mut mem(), dst, src, v);
    }
    /// `op_mask16_64`.
    op_mask16_64 @ 0x007e_ca00 => extern "C" fn op_mask16_64(dst: u32, a: u32, b: u32)  {
        small::bridge::op_mask16_64(&mut mem(), dst, a, b);
    }
    /// `op_add64_2`.
    op_add64_2 @ 0x007e_ca20 => extern "C" fn op_add64_2(dst: u32, src: u32, n: u32)  {
        op::bridge::op_add64(&mut mem(), dst, src, n);
    }
    /// `op_color_shift_store_word`.
    op_color_shift_store_word @ 0x007e_30c0 => extern "C" fn op_color_shift_store_word(src: u32, base: u32)  {
        small::bridge::op_color_shift(&mut mem(), src, base, false, false);
    }
    /// `op_color_shift_store_half`.
    op_color_shift_store_half @ 0x007e_aa60 => extern "C" fn op_color_shift_store_half(src: u32, base: u32)  {
        small::bridge::op_color_shift(&mut mem(), src, base, true, false);
    }
    /// `op_color_shift_load_word`.
    op_color_shift_load_word @ 0x007e_cac0 => extern "C" fn op_color_shift_load_word(dst: u32, base: u32)  {
        small::bridge::op_color_shift(&mut mem(), dst, base, false, true);
    }
    /// `op_color_shift_load16_2`.
    op_color_shift_load16_2 @ 0x007e_ca90 => extern "C" fn op_color_shift_load16_2(dst: u32, base: u32)  {
        op::bridge::op_color_shift_load16(&mut mem(), dst, base);
    }
    /// `op_pow2`.
    op_pow2 @ 0x007e_31e0 => extern "C" fn op_pow2(n: u32) -> u32 {
        small::bridge::op_pow2(n)
    }
    /// `op_log2_ceil`.
    op_log2_ceil @ 0x007e_aa90 => extern "C" fn op_log2_ceil(n: u32) -> u32 {
        small::bridge::op_log2_ceil(n)
    }
    /// `op_fabs`.
    op_fabs @ 0x007e_3360 => extern "C" fn op_fabs(x: f32) -> f32 {
        small::bridge::op_fabs(x)
    }
    /// `op_table2_get`.
    op_table2_get @ 0x007e_37b0 => extern "C" fn op_table2_get(t: u32, i: u32, j: u32) -> u32 {
        small::bridge::op_table2_get(&mem(), t, i, j)
    }
    /// `op_vec_splat`.
    op_vec_splat @ 0x007e_37f0 => extern "C" fn op_vec_splat(dst: u32, v: f32)  {
        small::bridge::op_vec_splat(&mut mem(), dst, v);
    }
    /// `op_vec4_move`.
    op_vec4_move @ 0x007e_3f60 => extern "C" fn op_vec4_move(dst: u32, src: u32)  {
        small::bridge::op_vec4_move(&mut mem(), dst, src);
    }
    /// `op_oef2_tex_pos_replace`.
    op_oef2_tex_pos_replace @ 0x007e_4fc0 => extern "C" fn op_oef2_tex_pos_replace(rec: u32, pos: u32) -> u32 {
        small::bridge::op_oef2_tex_pos_replace(&mut mem(), rec, pos)
    }
    /// `op_vec_zero`.
    op_vec_zero @ 0x007e_5660 => extern "C" fn op_vec_zero(dst: u32)  {
        small::bridge::op_vec_zero(&mut mem(), dst);
    }
    /// `op_vec_xz_add`.
    op_vec_xz_add @ 0x007e_6ca0 => extern "C" fn op_vec_xz_add(dst: u32, v: u32, x: f32, y: u32, z: f32)  {
        let _ = y;
        small::bridge::op_vec_xz_add(&mut mem(), dst, v, x, z);
    }
    /// `op_unit_m`.
    op_unit_m @ 0x007e_a6d0 => extern "C" fn op_unit_m(dst: u32)  {
        small::bridge::op_unit_m(&mut mem(), dst);
    }
    /// `op_word_copy_z`.
    op_word_copy_z @ 0x007e_ce80 => extern "C" fn op_word_copy_z(dst: u32, src: u32)  {
        small::bridge::op_word_copy_at(&mut mem(), dst, src, 8);
    }
    /// `op_spr_cmd_size`.
    op_spr_cmd_size @ 0x007f_5ed0 => extern "C" fn op_spr_cmd_size(cmd: u32) -> u32 {
        small::bridge::op_spr_cmd_size(&mem(), cmd)
    }
    /// `op_is_not_0119fe90`.
    op_is_not_0119fe90 @ 0x007f_e430 => extern "C" fn op_is_not_0119fe90(x: u32) -> u32 {
        small::bridge::op_is_not(&mem(), x, 0x0119_fe90)
    }
    /// `op_is_not_encount`.
    op_is_not_encount @ 0x007f_e4c0 => extern "C" fn op_is_not_encount(x: u32) -> u32 {
        small::bridge::op_is_not(&mem(), x, 0x0119_fe98)
    }
    /// `opus_draw_slot_clear`.
    opus_draw_slot_clear @ 0x007f_e690 => extern "C" fn opus_draw_slot_clear(obj: u32, v: u32)  {
        small::bridge::opus_draw_slot_clear(&mut mem(), obj, v);
    }
    /// `op_et_eff_index`.
    op_et_eff_index @ 0x007f_ed60 => extern "C" fn op_et_eff_index(id: u32) -> i32 {
        small::bridge::op_et_eff_index(&mem(), id)
    }
    /// `op_oef2_sep_number_get`.
    op_oef2_sep_number_get @ 0x007f_ff80 => extern "C" fn op_oef2_sep_number_get(cue: u32) -> u32 {
        small::bridge::op_oef2_sep_number_get(&mem(), cue)
    }
    /// `opus_part_kind`.
    opus_part_kind @ 0x0080_0410 => extern "C" fn opus_part_kind(p: u32) -> u32 {
        small::bridge::opus_part_kind(&mem(), p)
    }
    /// `opusIdentity_00806450`.
    opusIdentity_00806450 @ 0x0080_6450 => extern "C" fn opus_identity_00806450(v: u32) -> u32 {
        v
    }
    /// `opusIdentity_00806460`.
    opusIdentity_00806460 @ 0x0080_6460 => extern "C" fn opus_identity_00806460(v: u32) -> u32 {
        v
    }
    /// `opusIdentity_00806470`.
    opusIdentity_00806470 @ 0x0080_6470 => extern "C" fn opus_identity_00806470(v: u32) -> u32 {
        v
    }
    /// `opusIdentity_00806480`.
    opusIdentity_00806480 @ 0x0080_6480 => extern "C" fn opus_identity_00806480(v: u32) -> u32 {
        v
    }
    /// `opusIdentity_00806610`.
    opusIdentity_00806610 @ 0x0080_6610 => extern "C" fn opus_identity_00806610(v: u32) -> u32 {
        v
    }
    /// `opusIdentity_00806620`.
    opusIdentity_00806620 @ 0x0080_6620 => extern "C" fn opus_identity_00806620(v: u32) -> u32 {
        v
    }
    /// `opusCopyIf_00806490`.
    opusCopyIf_00806490 @ 0x0080_6490 => extern "stdcall" fn opus_copy_if_00806490(dst: u32, src: u32)  {
        small::bridge::copy_if(&mut mem(), dst, src);
    }
    /// `opusCopyIf_008064b0`.
    opusCopyIf_008064b0 @ 0x0080_64b0 => extern "stdcall" fn opus_copy_if_008064b0(dst: u32, src: u32)  {
        small::bridge::copy_if(&mut mem(), dst, src);
    }
    /// `opusCopyIf_008064f0`.
    opusCopyIf_008064f0 @ 0x0080_64f0 => extern "stdcall" fn opus_copy_if_008064f0(dst: u32, src: u32)  {
        small::bridge::copy_if(&mut mem(), dst, src);
    }
    /// `opusCopyIf_00806510`.
    opusCopyIf_00806510 @ 0x0080_6510 => extern "stdcall" fn opus_copy_if_00806510(dst: u32, src: u32)  {
        small::bridge::copy_if(&mut mem(), dst, src);
    }
    /// `opusCopyIf_008064d0`.
    opusCopyIf_008064d0 @ 0x0080_64d0 => extern "C" fn opus_copy_if_008064d0(al: u32, dst: u32, src: u32)  {
        let _ = al;
        small::bridge::copy_if(&mut mem(), dst, src);
    }
    /// `opusCopyIf_00806530`.
    opusCopyIf_00806530 @ 0x0080_6530 => extern "C" fn opus_copy_if_00806530(al: u32, dst: u32, src: u32)  {
        let _ = al;
        small::bridge::copy_if(&mut mem(), dst, src);
    }
    /// `opusSetThis_00806730`.
    opusSetThis_00806730 @ 0x0080_6730 => extern "thiscall" fn opus_set_this_00806730(this: u32, v: u32, x: u32) -> u32 {
        let _ = x;
        small::bridge::set_this(&mut mem(), this, v)
    }
    /// `opusSetThis_00806740`.
    opusSetThis_00806740 @ 0x0080_6740 => extern "thiscall" fn opus_set_this_00806740(this: u32, v: u32, x: u32) -> u32 {
        let _ = x;
        small::bridge::set_this(&mut mem(), this, v)
    }
    /// `opusSetThis_00806750`.
    opusSetThis_00806750 @ 0x0080_6750 => extern "thiscall" fn opus_set_this_00806750(this: u32, v: u32, x: u32) -> u32 {
        let _ = x;
        small::bridge::set_this(&mut mem(), this, v)
    }
    /// `opusSetThis_00806760`.
    opusSetThis_00806760 @ 0x0080_6760 => extern "thiscall" fn opus_set_this_00806760(this: u32, v: u32, x: u32) -> u32 {
        let _ = x;
        small::bridge::set_this(&mut mem(), this, v)
    }
    /// `opusSetThis_00806770`.
    opusSetThis_00806770 @ 0x0080_6770 => extern "thiscall" fn opus_set_this_00806770(this: u32, v: u32, x: u32) -> u32 {
        let _ = x;
        small::bridge::set_this(&mut mem(), this, v)
    }
    /// `opusSetThis_00806780`.
    opusSetThis_00806780 @ 0x0080_6780 => extern "thiscall" fn opus_set_this_00806780(this: u32, v: u32, x: u32) -> u32 {
        let _ = x;
        small::bridge::set_this(&mut mem(), this, v)
    }
    /// `opusIterEq_00806970`.
    opusIterEq_00806970 @ 0x0080_6970 => extern "thiscall" fn opus_iter_eq_00806970(this: u32, other: u32) -> u32 {
        small::bridge::iter_eq(&mem(), this, other, false)
    }
    /// `opusIterEq_00806990`.
    opusIterEq_00806990 @ 0x0080_6990 => extern "thiscall" fn opus_iter_eq_00806990(this: u32, other: u32) -> u32 {
        small::bridge::iter_eq(&mem(), this, other, false)
    }
    /// `opusIterNe_008069b0`.
    opusIterNe_008069b0 @ 0x0080_69b0 => extern "thiscall" fn opus_iter_ne_008069b0(this: u32, other: u32) -> u32 {
        small::bridge::iter_eq(&mem(), this, other, true)
    }
    /// `opusIterNe_008069d0`.
    opusIterNe_008069d0 @ 0x0080_69d0 => extern "thiscall" fn opus_iter_ne_008069d0(this: u32, other: u32) -> u32 {
        small::bridge::iter_eq(&mem(), this, other, true)
    }
    /// `opusPlus0c`.
    opusPlus0c @ 0x0080_6b50 => extern "C" fn opus_plus0c(p: u32) -> u32 {
        p.wrapping_add(0xc)
    }
    /// `op_et_bindeff_send_common_signal(h, bits)`.
    op_et_bindeff_send_common_signal @ 0x007f_bdf0 => extern "C" fn op_et_bindeff_send_common_signal(h: u32, bits: u32) -> u32 {
        et::bridge::op_et_bindeff_send_common_signal(&mut mem(), h, bits)
    }
    /// `op_et_bindeff_send_signal(h, bits)`.
    op_et_bindeff_send_signal @ 0x007f_bef0 => extern "C" fn op_et_bindeff_send_signal(h: u32, bits: u32) -> u32 {
        et::bridge::op_et_bindeff_send_signal(&mut mem(), h, bits)
    }
    /// `op_et_eff_off_signal(id)`.
    op_et_eff_off_signal @ 0x007f_c450 => extern "C" fn op_et_eff_off_signal(id: u32) {
        et::bridge::op_et_eff_off_signal(&mut mem(), id);
    }
    /// `op_et_weapon_afterimage_fade(id)`.
    op_et_weapon_afterimage_fade @ 0x007f_ca40 => extern "C" fn op_et_weapon_afterimage_fade(id: u32) {
        et::bridge::op_et_weapon_afterimage_fade(&mut mem(), id);
    }

    // ---- opus ----
    /// `opus_get_bone_matrix(out, chr, bone)`.
    opus_get_bone_matrix @ 0x0080_5430 => extern "C" fn opus_get_bone_matrix(out: u32, chr: u32, bone: u32) -> u32 {
        // the original's 16-byte local for Ch_OptposGetElement
        #[repr(C, align(16))]
        struct Locals([u8; 0x10]);
        let mut locals = Locals([0; 0x10]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32;
        opus::bridge::opus_get_bone_matrix(&mut mem(), out, chr, bone, frame, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opus_mimic_check(chr)`.
    opus_mimic_check @ 0x0080_f630 => extern "C" fn opus_mimic_check(chr: u32) -> u32 {
        opus::bridge::opus_mimic_check(&mut mem(), chr)
    }
    /// `opus_chara_root_m_get(out, chr)`.
    opus_chara_root_m_get @ 0x0080_5400 => extern "C" fn opus_chara_root_m_get(out: u32, chr: u32) -> u32 {
        opus::bridge::opus_chara_root_m_get(&mut mem(), out, chr, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opus_rt_m_chara(m, chr, k, angles, trans, battle)`.
    opus_rt_m_chara @ 0x0080_62c0 => extern "C" fn opus_rt_m_chara(m: u32, chr: u32, k: u32, angles: u32, trans: u32, battle: u32) {
        // the original's 16-byte scale local, at [ebp - 0x20]
        #[repr(C, align(16))]
        struct Locals([u8; 0x10]);
        let mut locals = Locals([0; 0x10]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = (locals.0.as_mut_ptr() as u32).wrapping_add(0x20);
        opus::bridge::opus_rt_m_chara(&mut mem(), m, chr, f32::from_bits(k), angles, trans, battle, frame, &mut |_, f, a| call_ptr(f, a), &mut |_, f, a| call_ptr_f64(f, a));
    }
    /// `opus_mdc_packet_set_0080e160(pk, mats, obj, unit, tex)`.
    opus_mdc_packet_set_0080e160 @ 0x0080_e160 => extern "C" fn opus_mdc_packet_set_0080e160(pk: u32, mats: u32, obj: u32, unit: u32, tex: u32) -> u32 {
        opus::bridge::opus_mdc_packet_set(&mut mem(), pk, mats, obj, unit, tex, true)
    }
    /// `opus_mdc_packet_set_0080e230(pk, mats, obj, unit, tex)`.
    opus_mdc_packet_set_0080e230 @ 0x0080_e230 => extern "C" fn opus_mdc_packet_set_0080e230(pk: u32, mats: u32, obj: u32, unit: u32, tex: u32) -> u32 {
        opus::bridge::opus_mdc_packet_set(&mut mem(), pk, mats, obj, unit, tex, false)
    }
    /// `opus_mdc_light_set_0080e900(pk, mats, obj, unit, nos)`.
    opus_mdc_light_set_0080e900 @ 0x0080_e900 => extern "C" fn opus_mdc_light_set_0080e900(pk: u32, mats: u32, obj: u32, unit: u32, nos: u32) {
        let _ = mats;
        opus::bridge::opus_mdc_light_set(&mut mem(), pk, obj, unit, nos, &mut |_, f, a| call_ptr(f, a));
    }
    /// `opus_get_weapon_index(obj)`.
    opus_get_weapon_index @ 0x0080_1000 => extern "C" fn opus_get_weapon_index(obj: u32) -> i32 {
        opus::bridge::opus_get_weapon_index(&mem(), obj)
    }
    /// `opus_get_monster_index(obj)`.
    opus_get_monster_index @ 0x0080_0e80 => extern "C" fn opus_get_monster_index(obj: u32) -> i32 {
        opus::bridge::opus_get_monster_index(&mem(), obj)
    }
    /// `opus_draw_off(obj, part)`.
    opus_draw_off @ 0x0080_0d10 => extern "C" fn opus_draw_off(obj: u32, part: u32) {
        opus::bridge::opus_draw_off(&mut mem(), obj, part);
    }
    /// `opus_get_osu_by_draw(obj, like, kind, any)`.
    opus_get_osu_by_draw @ 0x0080_0f60 => extern "C" fn opus_get_osu_by_draw(obj: u32, like: u32, kind: u32, any: u32) -> u32 {
        opus::bridge::opus_get_osu_by_draw(&mem(), obj, like, kind, any)
    }
    /// `opus_fullauto_no_get(obj)`.
    opus_fullauto_no_get @ 0x0080_0df0 => extern "C" fn opus_fullauto_no_get(obj: u32) -> u32 {
        opus::bridge::opus_fullauto_no_get(&mut mem(), obj)
    }
    /// `opu_wait(obj, cmd)`.
    opu_wait @ 0x0080_c500 => extern "C" fn opu_wait(obj: u32, cmd: u32) -> u32 {
        opus::bridge::opu_wait(&mut mem(), obj, cmd)
    }
    /// `opu_et_no(obj, cmd)`.
    opu_et_no @ 0x0081_65d0 => extern "C" fn opu_et_no(obj: u32, cmd: u32) -> u32 {
        opus::bridge::opu_et_no(&mut mem(), obj, cmd)
    }
    /// `opp_anmm_fin_en_bone3x(_, _, cmd, part)`.
    opp_anmm_fin_en_bone3x @ 0x0081_0ed0 => extern "C" fn opp_anmm_fin_en_bone3x(a: u32, b: u32, cmd: u32, part: u32) -> u32 {
        let _ = (a, b);
        opus::bridge::opp_anmm_fin_en_bone3x(&mut mem(), cmd, part)
    }

    // ---- opd ----
    /// `opd_num_pal_default()`.
    opd_num_pal_default @ 0x00a5_de50 => extern "C" fn opd_num_pal_default() {
        opd::bridge::opd_num_pal_default(&mut mem());
    }
    /// `opd_num_pal_set(pal)`.
    opd_num_pal_set @ 0x00a5_de60 => extern "C" fn opd_num_pal_set(pal: u32) {
        opd::bridge::opd_num_pal_set(&mut mem(), pal);
    }
    /// `opd_num_display(mode)`.
    opd_num_display @ 0x00a5_dd90 => extern "C" fn opd_num_display(mode: u32) {
        opd::bridge::opd_num_display(&mut mem(), mode);
    }
    /// `opd_num_xy_set(x, y)`.
    opd_num_xy_set @ 0x00a5_e2b0 => extern "C" fn opd_num_xy_set(x: u32, y: u32) {
        opd::bridge::opd_num_xy_set(&mut mem(), x, y);
    }
    /// `opd_num_vdisp()`.
    opd_num_vdisp @ 0x00a5_dfb0 => extern "C" fn opd_num_vdisp() {
        opd::bridge::opd_num_vdisp(&mut mem());
    }
    /// `opd_heap_display_clear(n)`.
    opd_heap_display_clear @ 0x00a6_49c0 => extern "C" fn opd_heap_display_clear(n: u32) {
        opd::bridge::opd_heap_display_clear(&mut mem(), n);
    }
    /// `opd_heap_display_init()`.
    opd_heap_display_init @ 0x00a6_4a00 => extern "C" fn opd_heap_display_init() {
        opd::bridge::opd_heap_display_init(&mut mem());
    }
    /// `opd_heap_display_set(used, total, flags)`.
    opd_heap_display_set @ 0x00a6_4b20 => extern "C" fn opd_heap_display_set(used: u32, total: u32, flags: u32) {
        opd::bridge::opd_heap_display_set(&mut mem(), used, total, flags);
    }
    /// `opd_heap_display_set_par(heap, flags)`.
    opd_heap_display_set_par @ 0x00a6_4b60 => extern "C" fn opd_heap_display_set_par(h: u32, flags: u32) {
        opd::bridge::opd_heap_display_set_par(&mut mem(), h, flags);
    }

    // ---- op ordering-table wrappers ----
    /// `op_func_before_ot_set(func, a, b, c)`.
    op_func_before_ot_set @ 0x007e_b370 => extern "C" fn op_func_before_ot_set(func: u32, a: u32, b: u32, c: u32) -> i32 {
        op::bridge::op_func_before_ot_set(&mut mem(), func, a, b, c)
    }
    /// `op_func_after_ot3d_set(func, a, b, c)`.
    op_func_after_ot3d_set @ 0x007e_b320 => extern "C" fn op_func_after_ot3d_set(func: u32, a: u32, b: u32, c: u32) -> i32 {
        op::bridge::op_func_after_ot3d_set(&mut mem(), func, a, b, c)
    }
    /// `op_func_before_wot_set(func, a, b, c)`.
    op_func_before_wot_set @ 0x007e_b3c0 => extern "C" fn op_func_before_wot_set(func: u32, a: u32, b: u32, c: u32) -> i32 {
        op::bridge::op_func_before_wot_set(&mut mem(), func, a, b, c)
    }
    /// `op_func_final_set(func, a, b, c)`.
    op_func_final_set @ 0x007e_b410 => extern "C" fn op_func_final_set(func: u32, a: u32, b: u32, c: u32) -> i32 {
        op::bridge::op_func_final_set(&mut mem(), func, a, b, c)
    }
    /// `op_func_before_ot_free(slot)`.
    op_func_before_ot_free @ 0x007e_b350 => extern "C" fn op_func_before_ot_free(slot: u32) {
        op::bridge::op_func_before_ot_free(&mut mem(), slot);
    }
    /// `op_func_after_ot3d_free(slot)`.
    op_func_after_ot3d_free @ 0x007e_b300 => extern "C" fn op_func_after_ot3d_free(slot: u32) {
        op::bridge::op_func_after_ot3d_free(&mut mem(), slot);
    }
    /// `op_func_before_wot_free(slot)`.
    op_func_before_wot_free @ 0x007e_b3a0 => extern "C" fn op_func_before_wot_free(slot: u32) {
        op::bridge::op_func_before_wot_free(&mut mem(), slot);
    }
    /// `op_heap_display_set_par(heap, flags)`.
    op_heap_display_set_par @ 0x007f_f2b0 => extern "C" fn op_heap_display_set_par(h: u32, flags: u32) {
        op::bridge::op_heap_display_set_par(&mut mem(), h, flags);
    }

    // ---- particle program table ----
    /// `pppDrawShapeCon(obj, ctrl)`.
    pppDrawShapeCon @ 0x0074_0eb0 => extern "C" fn ppp_draw_shape_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_shape_con(&mut mem(), obj, ctrl);
    }
    /// `pppDrawShapeXCon(obj, ctrl)`.
    pppDrawShapeXCon @ 0x0074_1450 => extern "C" fn ppp_draw_shape_x_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_shape_x_con(&mut mem(), obj, ctrl);
    }
    /// `pppNeiDrawShapePointLightCon(obj, ctrl)`.
    pppNeiDrawShapePointLightCon @ 0x0074_b9a0 => extern "C" fn ppp_nei_draw_shape_point_light_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_nei_draw_shape_point_light_con(&mut mem(), obj, ctrl);
    }
    /// `pppVertexApCon(obj, ctrl)`.
    pppVertexApCon @ 0x0075_7b30 => extern "C" fn ppp_vertex_ap_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_vertex_ap_con(&mut mem(), obj, ctrl);
    }
    /// `pppVertexApDisPosCon(obj, ctrl)`.
    pppVertexApDisPosCon @ 0x0075_7e90 => extern "C" fn ppp_vertex_ap_dis_pos_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_vertex_ap_dis_pos_con(&mut mem(), obj, ctrl);
    }
    /// `pppVertexApLcCon(obj, ctrl)`.
    pppVertexApLcCon @ 0x0075_8250 => extern "C" fn ppp_vertex_ap_lc_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_vertex_ap_lc_con(&mut mem(), obj, ctrl);
    }
    /// `pppVertexApAtCon(obj, ctrl)`.
    pppVertexApAtCon @ 0x0075_84b0 => extern "C" fn ppp_vertex_ap_at_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_vertex_ap_at_con(&mut mem(), obj, ctrl);
    }
    /// `pppPointApCon(obj, ctrl)`.
    pppPointApCon @ 0x0075_7470 => extern "C" fn ppp_point_ap_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_point_ap_con(&mut mem(), obj, ctrl);
    }
    /// `pppPointRApCon(obj, ctrl)`.
    pppPointRApCon @ 0x0075_7750 => extern "C" fn ppp_point_r_ap_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_point_r_ap_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeHitBornCon(obj, ctrl)`.
    pppKeHitBornCon @ 0x0075_9990 => extern "C" fn ppp_ke_hit_born_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_hit_born_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeThHitBornCon(obj, ctrl)`.
    pppKeThHitBornCon @ 0x0075_9f10 => extern "C" fn ppp_ke_th_hit_born_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_th_hit_born_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeBornRndCon(obj, ctrl)`.
    pppKeBornRndCon @ 0x0075_8790 => extern "C" fn ppp_ke_born_rnd_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_born_rnd_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeBornRnd2Con(obj, ctrl)`.
    pppKeBornRnd2Con @ 0x0075_8820 => extern "C" fn ppp_ke_born_rnd2_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_born_rnd2_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeBornRnd3Con(obj, ctrl)`.
    pppKeBornRnd3Con @ 0x0075_89d0 => extern "C" fn ppp_ke_born_rnd3_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_born_rnd3_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeBornRnd4Con(obj, ctrl)`.
    pppKeBornRnd4Con @ 0x0075_8dd0 => extern "C" fn ppp_ke_born_rnd4_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_born_rnd4_con(&mut mem(), obj, ctrl);
    }
    /// `pppDrawMdlRevCon(obj, ctrl)`.
    pppDrawMdlRevCon @ 0x0074_03f0 => extern "C" fn ppp_draw_mdl_rev_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_mdl_rev_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeLnsLpSftCon(obj, ctrl)`.
    pppKeLnsLpSftCon @ 0x0075_9fb0 => extern "C" fn ppp_ke_lns_lp_sft_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_lns_lp_sft_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeLnsLpCon2(obj, ctrl)`.
    pppKeLnsLpCon2 @ 0x0075_af60 => extern "C" fn ppp_ke_lns_lp_con2(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_lns_lp_con2(&mut mem(), obj, ctrl);
    }
    /// `pppKeLnsLpTCon2(obj, ctrl)`.
    pppKeLnsLpTCon2 @ 0x0075_b110 => extern "C" fn ppp_ke_lns_lp_t_con2(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_lns_lp_t_con2(&mut mem(), obj, ctrl);
    }
    /// `pppKeBornRnd5Con(obj, ctrl)`.
    pppKeBornRnd5Con @ 0x0075_8f90 => extern "C" fn ppp_ke_born_rnd5_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_born_rnd5_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeBornRnd6Con(obj, ctrl)`.
    pppKeBornRnd6Con @ 0x0075_9270 => extern "C" fn ppp_ke_born_rnd6_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_born_rnd6_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeHitChkPxBCon(obj, ctrl)`.
    pppKeHitChkPxBCon @ 0x0075_9bc0 => extern "C" fn ppp_ke_hit_chk_px_b_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_hit_chk_px_b_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeShpTailCon(obj, ctrl)`.
    pppKeShpTailCon @ 0x0074_c540 => extern "C" fn ppp_ke_shp_tail_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_shp_tail_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeShpTailXCon(obj, ctrl)`.
    pppKeShpTailXCon @ 0x0074_d540 => extern "C" fn ppp_ke_shp_tail_x_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_shp_tail_x_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeShpTail2Con(obj, ctrl)`.
    pppKeShpTail2Con @ 0x0074_e540 => extern "C" fn ppp_ke_shp_tail2_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_shp_tail2_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeShpTail2XCon(obj, ctrl)`.
    pppKeShpTail2XCon @ 0x0074_f580 => extern "C" fn ppp_ke_shp_tail2_x_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_shp_tail2_x_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeShpTailLcCon(obj, ctrl)`.
    pppKeShpTailLcCon @ 0x0075_4fc0 => extern "C" fn ppp_ke_shp_tail_lc_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_shp_tail_lc_con(&mut mem(), obj, ctrl);
    }
    /// `pppDrawShapeRevCon(obj, ctrl)`.
    pppDrawShapeRevCon @ 0x0074_1570 => extern "C" fn ppp_draw_shape_rev_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_shape_rev_con(&mut mem(), obj, ctrl);
    }
    /// `pppVtMimeCon2(obj, ctrl)`.
    pppVtMimeCon2 @ 0x0072_edb0 => extern "C" fn ppp_vt_mime_con2(obj: u32, ctrl: u32) {
        prog::bridge::ppp_vt_mime_con2(&mut mem(), obj, ctrl);
    }
    /// `pppNeiPointLightCon(obj, ctrl)`.
    pppNeiPointLightCon @ 0x0075_d7c0 => extern "C" fn ppp_nei_point_light_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_nei_point_light_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeTkFadeCon(obj, ctrl)`.
    pppKeTkFadeCon @ 0x0072_ee20 => extern "C" fn ppp_ke_tk_fade_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_tk_fade_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeMdlTfdCon(obj, ctrl)`.
    pppKeMdlTfdCon @ 0x0074_97e0 => extern "C" fn ppp_ke_mdl_tfd_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_mdl_tfd_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeMdlTfd2Con(obj, ctrl)`.
    pppKeMdlTfd2Con @ 0x0074_9b10 => extern "C" fn ppp_ke_mdl_tfd2_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_mdl_tfd2_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeMdlTfd3Con(obj, ctrl)`.
    pppKeMdlTfd3Con @ 0x0074_9e60 => extern "C" fn ppp_ke_mdl_tfd3_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_mdl_tfd3_con(&mut mem(), obj, ctrl);
    }
    /// `pppMoveLoopCon(obj, ctrl)`.
    pppMoveLoopCon @ 0x0075_c150 => extern "C" fn ppp_move_loop_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_move_loop_con(&mut mem(), obj, ctrl);
    }
    /// `pppMoveLoopCon2(obj, ctrl)`.
    pppMoveLoopCon2 @ 0x0075_c180 => extern "C" fn ppp_move_loop_con2(obj: u32, ctrl: u32) {
        prog::bridge::ppp_move_loop_con2(&mut mem(), obj, ctrl);
    }
    /// `pppSclMoveLoopCon(obj, ctrl)`.
    pppSclMoveLoopCon @ 0x0075_c330 => extern "C" fn ppp_scl_move_loop_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_scl_move_loop_con(&mut mem(), obj, ctrl);
    }
    /// `pppSclMoveLoopCon2(obj, ctrl)`.
    pppSclMoveLoopCon2 @ 0x0075_c360 => extern "C" fn ppp_scl_move_loop_con2(obj: u32, ctrl: u32) {
        prog::bridge::ppp_scl_move_loop_con2(&mut mem(), obj, ctrl);
    }
    /// `pppAngMoveLoopCon(obj, ctrl)`.
    pppAngMoveLoopCon @ 0x0075_c210 => extern "C" fn ppp_ang_move_loop_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ang_move_loop_con(&mut mem(), obj, ctrl);
    }
    /// `pppAngMoveLoopCon2(obj, ctrl)`.
    pppAngMoveLoopCon2 @ 0x0075_c250 => extern "C" fn ppp_ang_move_loop_con2(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ang_move_loop_con2(&mut mem(), obj, ctrl);
    }
    /// `pppKeOfsPtCon(obj, ctrl)`.
    pppKeOfsPtCon @ 0x0075_cdf0 => extern "C" fn ppp_ke_ofs_pt_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_ofs_pt_con(&mut mem(), obj, ctrl);
    }
    /// `pppDrawMdlSeaCon2(obj, ctrl)`.
    pppDrawMdlSeaCon2 @ 0x0073_ab40 => extern "C" fn ppp_draw_mdl_sea_con2(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_mdl_sea_con2(&mut mem(), obj, ctrl);
    }
    /// `pppDrawMdlInfCon2(obj, ctrl)`.
    pppDrawMdlInfCon2 @ 0x0073_b800 => extern "C" fn ppp_draw_mdl_inf_con2(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_mdl_inf_con2(&mut mem(), obj, ctrl);
    }
    /// `pppDrawMdlPSimCon2(obj, ctrl)`.
    pppDrawMdlPSimCon2 @ 0x0073_cfe0 => extern "C" fn ppp_draw_mdl_p_sim_con2(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_mdl_p_sim_con2(&mut mem(), obj, ctrl);
    }
    /// `pppDrawMdlLoopCon2(obj, ctrl)`.
    pppDrawMdlLoopCon2 @ 0x0073_e270 => extern "C" fn ppp_draw_mdl_loop_con2(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_mdl_loop_con2(&mut mem(), obj, ctrl);
    }
    /// `pppDrawMdlLoopZCon2(obj, ctrl)`.
    pppDrawMdlLoopZCon2 @ 0x0073_ea00 => extern "C" fn ppp_draw_mdl_loop_z_con2(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_mdl_loop_z_con2(&mut mem(), obj, ctrl);
    }
    /// `pppDrawMdlLoopDisPosCon2(obj, ctrl)`.
    pppDrawMdlLoopDisPosCon2 @ 0x0073_f1f0 => extern "C" fn ppp_draw_mdl_loop_dis_pos_con2(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_mdl_loop_dis_pos_con2(&mut mem(), obj, ctrl);
    }
    /// `pppDrawMdlCameraLoopCon2(obj, ctrl)`.
    pppDrawMdlCameraLoopCon2 @ 0x0073_fa10 => extern "C" fn ppp_draw_mdl_camera_loop_con2(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_mdl_camera_loop_con2(&mut mem(), obj, ctrl);
    }
    /// `pppDrawMdlCameraCon2(obj, ctrl)`.
    pppDrawMdlCameraCon2 @ 0x0073_d8f0 => extern "C" fn ppp_draw_mdl_camera_con2(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_mdl_camera_con2(&mut mem(), obj, ctrl);
    }
    /// `pppKeShpDttCon(obj, ctrl)`.
    pppKeShpDttCon @ 0x0075_5010 => extern "C" fn ppp_ke_shp_dtt_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_shp_dtt_con(&mut mem(), obj, ctrl);
    }
    /// `pppEiWindFunCon(obj, ctrl)`.
    pppEiWindFunCon @ 0x0075_d590 => extern "C" fn ppp_ei_wind_fun_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ei_wind_fun_con(&mut mem(), obj, ctrl);
    }
    /// `pppDrawMdlTsCon(obj, ctrl)`.
    pppDrawMdlTsCon @ 0x0073_84f0 => extern "C" fn ppp_draw_mdl_ts_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_mdl_ts_con(&mut mem(), obj, ctrl);
    }
    /// `pppDrawMdlTs2Con(obj, ctrl)`.
    pppDrawMdlTs2Con @ 0x0073_9480 => extern "C" fn ppp_draw_mdl_ts2_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_mdl_ts2_con(&mut mem(), obj, ctrl);
    }
    /// `pppDrawMdlTs3Con(obj, ctrl)`.
    pppDrawMdlTs3Con @ 0x0073_aac0 => extern "C" fn ppp_draw_mdl_ts3_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_mdl_ts3_con(&mut mem(), obj, ctrl);
    }
    /// `pppNeiDrawMdlTsPointLightCon(obj, ctrl)`.
    pppNeiDrawMdlTsPointLightCon @ 0x0074_b500 => extern "C" fn ppp_nei_draw_mdl_ts_point_light_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_nei_draw_mdl_ts_point_light_con(&mut mem(), obj, ctrl);
    }
    /// `pppPointCon(obj, ctrl)`.
    pppPointCon @ 0x0075_c4c0 => extern "C" fn ppp_point_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_point_con(&mut mem(), obj, ctrl);
    }
    /// `pppScaleCon(obj, ctrl)`.
    pppScaleCon @ 0x0075_d050 => extern "C" fn ppp_scale_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_scale_con(&mut mem(), obj, ctrl);
    }
    /// `pppAngleCon(obj, ctrl)`.
    pppAngleCon @ 0x0075_ce80 => extern "C" fn ppp_angle_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_angle_con(&mut mem(), obj, ctrl);
    }
    /// `pppAcceleCon(obj, ctrl)`.
    pppAcceleCon @ 0x0075_b810 => extern "C" fn ppp_accele_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_accele_con(&mut mem(), obj, ctrl);
    }
    /// `pppSclAcceleCon(obj, ctrl)`.
    pppSclAcceleCon @ 0x0075_b9d0 => extern "C" fn ppp_scl_accele_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_scl_accele_con(&mut mem(), obj, ctrl);
    }
    /// `pppMoveCon(obj, ctrl)`.
    pppMoveCon @ 0x0075_beb0 => extern "C" fn ppp_move_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_move_con(&mut mem(), obj, ctrl);
    }
    /// `pppSclMoveCon(obj, ctrl)`.
    pppSclMoveCon @ 0x0075_c070 => extern "C" fn ppp_scl_move_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_scl_move_con(&mut mem(), obj, ctrl);
    }
    /// `pppAngAcceleCon(obj, ctrl)`.
    pppAngAcceleCon @ 0x0075_b8c0 => extern "C" fn ppp_ang_accele_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ang_accele_con(&mut mem(), obj, ctrl);
    }
    /// `pppAngMoveCon(obj, ctrl)`.
    pppAngMoveCon @ 0x0075_bf60 => extern "C" fn ppp_ang_move_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ang_move_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeBornPtCmplCon(obj, ctrl)`.
    pppKeBornPtCmplCon @ 0x0075_9620 => extern "C" fn ppp_ke_born_pt_cmpl_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_born_pt_cmpl_con(&mut mem(), obj, ctrl);
    }
    /// `pppPointLoopCon(obj, ctrl)`.
    pppPointLoopCon @ 0x0075_c5a0 => extern "C" fn ppp_point_loop_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_point_loop_con(&mut mem(), obj, ctrl);
    }
    /// `pppAngleLoopCon(obj, ctrl)`.
    pppAngleLoopCon @ 0x0075_cf40 => extern "C" fn ppp_angle_loop_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_angle_loop_con(&mut mem(), obj, ctrl);
    }
    /// `pppScaleLoopCon(obj, ctrl)`.
    pppScaleLoopCon @ 0x0075_d130 => extern "C" fn ppp_scale_loop_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_scale_loop_con(&mut mem(), obj, ctrl);
    }
    /// `pppDrawMdlCameraCon(obj, ctrl)`.
    pppDrawMdlCameraCon @ 0x0073_d8c0 => extern "C" fn ppp_draw_mdl_camera_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_mdl_camera_con(&mut mem(), obj, ctrl);
    }
    /// `pppDrawMdlLoopCon(obj, ctrl)`.
    pppDrawMdlLoopCon @ 0x0073_e240 => extern "C" fn ppp_draw_mdl_loop_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_mdl_loop_con(&mut mem(), obj, ctrl);
    }
    /// `pppDrawMdlLoopZCon(obj, ctrl)`.
    pppDrawMdlLoopZCon @ 0x0073_e9d0 => extern "C" fn ppp_draw_mdl_loop_z_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_mdl_loop_z_con(&mut mem(), obj, ctrl);
    }
    /// `pppDrawMdlLoopDisPosCon(obj, ctrl)`.
    pppDrawMdlLoopDisPosCon @ 0x0073_f1c0 => extern "C" fn ppp_draw_mdl_loop_dis_pos_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_mdl_loop_dis_pos_con(&mut mem(), obj, ctrl);
    }
    /// `pppDrawMdlCameraLoopCon(obj, ctrl)`.
    pppDrawMdlCameraLoopCon @ 0x0073_f9e0 => extern "C" fn ppp_draw_mdl_camera_loop_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_mdl_camera_loop_con(&mut mem(), obj, ctrl);
    }
    /// `pppDrawSpriteCon(obj, ctrl)`.
    pppDrawSpriteCon @ 0x0075_66a0 => extern "C" fn ppp_draw_sprite_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_sprite_con(&mut mem(), obj, ctrl);
    }
    /// `pppDrawFilterCon(obj, ctrl)`.
    pppDrawFilterCon @ 0x0075_7210 => extern "C" fn ppp_draw_filter_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_filter_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeGrvTgtCon(obj, ctrl)`.
    pppKeGrvTgtCon @ 0x0075_9800 => extern "C" fn ppp_ke_grv_tgt_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_grv_tgt_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeThTp2Con(obj, ctrl)`.
    pppKeThTp2Con @ 0x0073_6e30 => extern "C" fn ppp_ke_th_tp2_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_th_tp2_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeThTpCon(obj, ctrl)`.
    pppKeThTpCon @ 0x0073_6e60 => extern "C" fn ppp_ke_th_tp_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_th_tp_con(&mut mem(), obj, ctrl);
    }
    /// `pppDrawMdlPSimCon(obj, ctrl)`.
    pppDrawMdlPSimCon @ 0x0073_cfb0 => extern "C" fn ppp_draw_mdl_p_sim_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_mdl_p_sim_con(&mut mem(), obj, ctrl);
    }
    /// `pppDrawRainCon(obj, ctrl)`.
    pppDrawRainCon @ 0x0075_6240 => extern "C" fn ppp_draw_rain_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_rain_con(&mut mem(), obj, ctrl);
    }
    /// `pppDrawMdlSeaCon(obj, ctrl)`.
    pppDrawMdlSeaCon @ 0x0073_ab00 => extern "C" fn ppp_draw_mdl_sea_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_mdl_sea_con(&mut mem(), obj, ctrl);
    }
    /// `pppColAcceleCon(obj, ctrl)`.
    pppColAcceleCon @ 0x0075_bac0 => extern "C" fn ppp_col_accele_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_col_accele_con(&mut mem(), obj, ctrl);
    }
    /// `pppColMoveCon(obj, ctrl)`.
    pppColMoveCon @ 0x0075_c410 => extern "C" fn ppp_col_move_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_col_accele_con(&mut mem(), obj, ctrl);
    }
    /// `pppAccele(obj, key, ctrl)`.
    pppAccele @ 0x0075_b740 => extern "C" fn ppp_accele(obj: u32, key: u32, ctrl: u32) {
        prog::bridge::ppp_accele(&mut mem(), obj, key, ctrl);
    }
    /// `pppSclAccele(obj, key, ctrl)`.
    pppSclAccele @ 0x0075_b900 => extern "C" fn ppp_scl_accele(obj: u32, key: u32, ctrl: u32) {
        prog::bridge::ppp_accele(&mut mem(), obj, key, ctrl);
    }
    /// `pppMove(obj, key, ctrl)`.
    pppMove @ 0x0075_bde0 => extern "C" fn ppp_move(obj: u32, key: u32, ctrl: u32) {
        prog::bridge::ppp_accele(&mut mem(), obj, key, ctrl);
    }
    /// `pppSclMove(obj, key, ctrl)`.
    pppSclMove @ 0x0075_bfa0 => extern "C" fn ppp_scl_move(obj: u32, key: u32, ctrl: u32) {
        prog::bridge::ppp_accele(&mut mem(), obj, key, ctrl);
    }
    /// `pppAngAccele(obj, key, ctrl)`.
    pppAngAccele @ 0x0075_b850 => extern "C" fn ppp_ang_accele(obj: u32, key: u32, ctrl: u32) {
        prog::bridge::ppp_ang_accele(&mut mem(), obj, key, ctrl);
    }
    /// `pppAngMove(obj, key, ctrl)`.
    pppAngMove @ 0x0075_bef0 => extern "C" fn ppp_ang_move(obj: u32, key: u32, ctrl: u32) {
        prog::bridge::ppp_ang_accele(&mut mem(), obj, key, ctrl);
    }
    /// `pppColAccele(obj, key, ctrl)`.
    pppColAccele @ 0x0075_ba40 => extern "C" fn ppp_col_accele(obj: u32, key: u32, ctrl: u32) {
        prog::bridge::ppp_col_accele(&mut mem(), obj, key, ctrl);
    }
    /// `pppColMove(obj, key, ctrl)`.
    pppColMove @ 0x0075_c390 => extern "C" fn ppp_col_move(obj: u32, key: u32, ctrl: u32) {
        prog::bridge::ppp_col_accele(&mut mem(), obj, key, ctrl);
    }
    /// `pppMoveLoop(obj, key, ctrl)`.
    pppMoveLoop @ 0x0075_c0b0 => extern "C" fn ppp_move_loop(obj: u32, key: u32, ctrl: u32) {
        prog::bridge::ppp_move_loop(&mut mem(), obj, key, ctrl);
    }
    /// `pppSclMoveLoop(obj, key, ctrl)`.
    pppSclMoveLoop @ 0x0075_c290 => extern "C" fn ppp_scl_move_loop(obj: u32, key: u32, ctrl: u32) {
        prog::bridge::ppp_move_loop(&mut mem(), obj, key, ctrl);
    }
    /// `pppAngMoveLoop(obj, key, ctrl)`.
    pppAngMoveLoop @ 0x0075_c1b0 => extern "C" fn ppp_ang_move_loop(obj: u32, key: u32, ctrl: u32) {
        prog::bridge::ppp_ang_move_loop(&mut mem(), obj, key, ctrl);
    }
    /// `pppPoint(obj, key, ctrl)`.
    pppPoint @ 0x0075_c450 => extern "C" fn ppp_point(obj: u32, key: u32, ctrl: u32) {
        prog::bridge::ppp_point(&mut mem(), obj, key, ctrl);
    }
    /// `pppScale(obj, key, ctrl)`.
    pppScale @ 0x0075_cfe0 => extern "C" fn ppp_scale(obj: u32, key: u32, ctrl: u32) {
        prog::bridge::ppp_point(&mut mem(), obj, key, ctrl);
    }
    /// `pppPointLoop(obj, key, ctrl)`.
    pppPointLoop @ 0x0075_c500 => extern "C" fn ppp_point_loop(obj: u32, key: u32, ctrl: u32) {
        prog::bridge::ppp_point_loop(&mut mem(), obj, key, ctrl);
    }
    /// `pppScaleLoop(obj, key, ctrl)`.
    pppScaleLoop @ 0x0075_d090 => extern "C" fn ppp_scale_loop(obj: u32, key: u32, ctrl: u32) {
        prog::bridge::ppp_point_loop(&mut mem(), obj, key, ctrl);
    }
    /// `pppAngleLoop(obj, key, ctrl)`.
    pppAngleLoop @ 0x0075_cec0 => extern "C" fn ppp_angle_loop(obj: u32, key: u32, ctrl: u32) {
        prog::bridge::ppp_angle_loop(&mut mem(), obj, key, ctrl);
    }
    /// `pppPointLoopCon2(obj, ctrl)`.
    pppPointLoopCon2 @ 0x0075_c5d0 => extern "C" fn ppp_point_loop_con2(obj: u32, ctrl: u32) {
        prog::bridge::ppp_point_loop_con2(&mut mem(), obj, ctrl);
    }
    /// `pppScaleLoopCon2(obj, ctrl)`.
    pppScaleLoopCon2 @ 0x0075_d160 => extern "C" fn ppp_scale_loop_con2(obj: u32, ctrl: u32) {
        prog::bridge::ppp_point_loop_con2(&mut mem(), obj, ctrl);
    }
    /// `pppAngleLoopCon2(obj, ctrl)`.
    pppAngleLoopCon2 @ 0x0075_cf70 => extern "C" fn ppp_angle_loop_con2(obj: u32, ctrl: u32) {
        prog::bridge::ppp_angle_loop_con2(&mut mem(), obj, ctrl);
    }
    /// `pppKeGrvTgt(obj, key, ctrl)`.
    pppKeGrvTgt @ 0x0075_97b0 => extern "C" fn ppp_ke_grv_tgt(obj: u32, key: u32, ctrl: u32) {
        prog::bridge::ppp_ke_grv_tgt(&mut mem(), obj, key, ctrl);
    }
    /// `pppKeThRes128Con(obj, ctrl)`.
    pppKeThRes128Con @ 0x0073_6690 => extern "C" fn ppp_ke_th_res128_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_th_res128_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeThRes128x4Con(obj, ctrl)`.
    pppKeThRes128x4Con @ 0x0073_66c0 => extern "C" fn ppp_ke_th_res128x4_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_th_res128x4_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeThRes128x8Con(obj, ctrl)`.
    pppKeThRes128x8Con @ 0x0073_66f0 => extern "C" fn ppp_ke_th_res128x8_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_th_res128x8_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeThRes16Con(obj, ctrl)`.
    pppKeThRes16Con @ 0x0073_6720 => extern "C" fn ppp_ke_th_res16_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_th_res16_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeThRes16x16Con(obj, ctrl)`.
    pppKeThRes16x16Con @ 0x0073_6750 => extern "C" fn ppp_ke_th_res16x16_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_th_res16x16_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeThRes16x24Con(obj, ctrl)`.
    pppKeThRes16x24Con @ 0x0073_6780 => extern "C" fn ppp_ke_th_res16x24_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_th_res16x24_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeThRes16x4Con(obj, ctrl)`.
    pppKeThRes16x4Con @ 0x0073_67b0 => extern "C" fn ppp_ke_th_res16x4_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_th_res16x4_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeThRes16x64Con(obj, ctrl)`.
    pppKeThRes16x64Con @ 0x0073_67e0 => extern "C" fn ppp_ke_th_res16x64_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_th_res16x64_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeThRes16x8Con(obj, ctrl)`.
    pppKeThRes16x8Con @ 0x0073_6810 => extern "C" fn ppp_ke_th_res16x8_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_th_res16x8_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeThRes24Con(obj, ctrl)`.
    pppKeThRes24Con @ 0x0073_6840 => extern "C" fn ppp_ke_th_res24_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_th_res24_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeThRes24x16Con(obj, ctrl)`.
    pppKeThRes24x16Con @ 0x0073_6870 => extern "C" fn ppp_ke_th_res24x16_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_th_res24x16_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeThRes24x4Con(obj, ctrl)`.
    pppKeThRes24x4Con @ 0x0073_68a0 => extern "C" fn ppp_ke_th_res24x4_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_th_res24x4_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeThRes24x8Con(obj, ctrl)`.
    pppKeThRes24x8Con @ 0x0073_68d0 => extern "C" fn ppp_ke_th_res24x8_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_th_res24x8_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeThRes255Con(obj, ctrl)`.
    pppKeThRes255Con @ 0x0073_6900 => extern "C" fn ppp_ke_th_res255_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_th_res255_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeThRes255x4Con(obj, ctrl)`.
    pppKeThRes255x4Con @ 0x0073_6930 => extern "C" fn ppp_ke_th_res255x4_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_th_res255x4_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeThRes32Con(obj, ctrl)`.
    pppKeThRes32Con @ 0x0073_6960 => extern "C" fn ppp_ke_th_res32_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_th_res32_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeThRes32x16Con(obj, ctrl)`.
    pppKeThRes32x16Con @ 0x0073_6990 => extern "C" fn ppp_ke_th_res32x16_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_th_res32x16_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeThRes32x24Con(obj, ctrl)`.
    pppKeThRes32x24Con @ 0x0073_69c0 => extern "C" fn ppp_ke_th_res32x24_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_th_res32x24_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeThRes32x32Con(obj, ctrl)`.
    pppKeThRes32x32Con @ 0x0073_69f0 => extern "C" fn ppp_ke_th_res32x32_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_th_res32x32_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeThRes32x4Con(obj, ctrl)`.
    pppKeThRes32x4Con @ 0x0073_6a20 => extern "C" fn ppp_ke_th_res32x4_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_th_res32x4_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeThRes32x8Con(obj, ctrl)`.
    pppKeThRes32x8Con @ 0x0073_6a50 => extern "C" fn ppp_ke_th_res32x8_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_th_res32x8_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeThRes40Con(obj, ctrl)`.
    pppKeThRes40Con @ 0x0073_6a80 => extern "C" fn ppp_ke_th_res40_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_th_res40_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeThRes40x16Con(obj, ctrl)`.
    pppKeThRes40x16Con @ 0x0073_6ab0 => extern "C" fn ppp_ke_th_res40x16_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_th_res40x16_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeThRes40x4Con(obj, ctrl)`.
    pppKeThRes40x4Con @ 0x0073_6ae0 => extern "C" fn ppp_ke_th_res40x4_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_th_res40x4_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeThRes40x8Con(obj, ctrl)`.
    pppKeThRes40x8Con @ 0x0073_6b10 => extern "C" fn ppp_ke_th_res40x8_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_th_res40x8_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeThRes48Con(obj, ctrl)`.
    pppKeThRes48Con @ 0x0073_6b40 => extern "C" fn ppp_ke_th_res48_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_th_res48_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeThRes48x16Con(obj, ctrl)`.
    pppKeThRes48x16Con @ 0x0073_6b70 => extern "C" fn ppp_ke_th_res48x16_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_th_res48x16_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeThRes48x4Con(obj, ctrl)`.
    pppKeThRes48x4Con @ 0x0073_6ba0 => extern "C" fn ppp_ke_th_res48x4_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_th_res48x4_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeThRes48x8Con(obj, ctrl)`.
    pppKeThRes48x8Con @ 0x0073_6bd0 => extern "C" fn ppp_ke_th_res48x8_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_th_res48x8_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeThRes64Con(obj, ctrl)`.
    pppKeThRes64Con @ 0x0073_6c00 => extern "C" fn ppp_ke_th_res64_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_th_res64_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeThRes64x16Con(obj, ctrl)`.
    pppKeThRes64x16Con @ 0x0073_6c30 => extern "C" fn ppp_ke_th_res64x16_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_th_res64x16_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeThRes64x4Con(obj, ctrl)`.
    pppKeThRes64x4Con @ 0x0073_6c60 => extern "C" fn ppp_ke_th_res64x4_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_th_res64x4_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeThRes64x8Con(obj, ctrl)`.
    pppKeThRes64x8Con @ 0x0073_6c90 => extern "C" fn ppp_ke_th_res64x8_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_th_res64x8_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeThRes8Con(obj, ctrl)`.
    pppKeThRes8Con @ 0x0073_6cc0 => extern "C" fn ppp_ke_th_res8_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_th_res8_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeThRes8x128Con(obj, ctrl)`.
    pppKeThRes8x128Con @ 0x0073_6cf0 => extern "C" fn ppp_ke_th_res8x128_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_th_res8x128_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeThRes8x4Con(obj, ctrl)`.
    pppKeThRes8x4Con @ 0x0073_6d20 => extern "C" fn ppp_ke_th_res8x4_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_th_res8x4_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeThResCon(obj, ctrl)`.
    pppKeThResCon @ 0x0073_6d50 => extern "C" fn ppp_ke_th_res_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_th_res_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeLnsArndCon(obj, ctrl)`.
    pppKeLnsArndCon @ 0x0075_9fd0 => extern "C" fn ppp_ke_lns_arnd_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_lns_arnd_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeLnsCrnCon(obj, ctrl)`.
    pppKeLnsCrnCon @ 0x0075_a740 => extern "C" fn ppp_ke_lns_crn_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_lns_crn_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeLnsClmCon(obj, ctrl)`.
    pppKeLnsClmCon @ 0x0075_a370 => extern "C" fn ppp_ke_lns_clm_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_lns_clm_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeThCon(obj, ctrl)`.
    pppKeThCon @ 0x0073_6460 => extern "C" fn ppp_ke_th_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_th_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeMdlTfdUvCon(obj, ctrl)`.
    pppKeMdlTfdUvCon @ 0x0074_98f0 => extern "C" fn ppp_ke_mdl_tfd_uv_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_mdl_tfd_uv_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeMdlTfdUv2Con(obj, ctrl)`.
    pppKeMdlTfdUv2Con @ 0x0074_9c30 => extern "C" fn ppp_ke_mdl_tfd_uv2_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_mdl_tfd_uv2_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeMdlTfdUv3Con(obj, ctrl)`.
    pppKeMdlTfdUv3Con @ 0x0074_9f90 => extern "C" fn ppp_ke_mdl_tfd_uv3_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_mdl_tfd_uv3_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeShpTail3Con2(obj, ctrl)`.
    pppKeShpTail3Con2 @ 0x0075_05a0 => extern "C" fn ppp_ke_shp_tail3_con2(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_shp_tail3_con2(&mut mem(), obj, ctrl);
    }
    /// `pppKeShpTail3XCon2(obj, ctrl)`.
    pppKeShpTail3XCon2 @ 0x0075_1f40 => extern "C" fn ppp_ke_shp_tail3_x_con2(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_shp_tail3_x_con2(&mut mem(), obj, ctrl);
    }
    /// `pppDrawMdlInfCon(obj, ctrl)`.
    pppDrawMdlInfCon @ 0x0073_b7b0 => extern "C" fn ppp_draw_mdl_inf_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_mdl_inf_con(&mut mem(), obj, ctrl);
    }
    /// `pppDrawSprite(obj, key, ctrl)`.
    pppDrawSprite @ 0x0075_66e0 => extern "C" fn ppp_draw_sprite(obj: u32, key: u32, ctrl: u32) {
        prog::bridge::ppp_draw_sprite(&mut mem(), obj, key, ctrl);
    }
    /// `pppKeHitBall(obj, key, ctrl)`.
    pppKeHitBall @ 0x0075_9830 => extern "C" fn ppp_ke_hit_ball(obj: u32, key: u32, ctrl: u32) {
        prog::bridge::ppp_ke_hit_ball(&mut mem(), obj, key, ctrl);
    }
    /// `pppNeiPointLight(obj, key, ctrl)`.
    pppNeiPointLight @ 0x0075_d700 => extern "C" fn ppp_nei_point_light(obj: u32, key: u32, ctrl: u32) {
        prog::bridge::ppp_nei_point_light(&mut mem(), obj, key, ctrl);
    }
    /// `pppPObjPoint(obj, key, ctrl)`.
    pppPObjPoint @ 0x0075_d280 => extern "C" fn ppp_p_obj_point(obj: u32, key: u32, ctrl: u32) {
        prog::bridge::ppp_p_obj_point(&mut mem(), obj, key, ctrl);
    }
    /// `pppKeThTp(obj, key, ctrl)`.
    pppKeThTp @ 0x0073_6d80 => extern "C" fn ppp_ke_th_tp(obj: u32, key: u32, ctrl: u32) {
        prog::bridge::ppp_ke_th_tp(&mut mem(), obj, key, ctrl);
    }

    // ---- matrix programs ----
    /// `pppGetRotMatrixXYZ(out, angles)`.
    pppGetRotMatrixXYZ @ 0x0072_c800 => extern "C" fn ppp_get_rot_matrix_xyz(out: u32, angles: u32) {
        mat::bridge::ppp_get_rot_matrix(&mut mem(), out, angles, order::XYZ);
    }
    /// `pppGetRotMatrixXZY(out, angles)`.
    pppGetRotMatrixXZY @ 0x0072_c890 => extern "C" fn ppp_get_rot_matrix_xzy(out: u32, angles: u32) {
        mat::bridge::ppp_get_rot_matrix(&mut mem(), out, angles, order::XZY);
    }
    /// `pppGetRotMatrixYXZ(out, angles)`.
    pppGetRotMatrixYXZ @ 0x0072_c9f0 => extern "C" fn ppp_get_rot_matrix_yxz(out: u32, angles: u32) {
        mat::bridge::ppp_get_rot_matrix(&mut mem(), out, angles, order::YXZ);
    }
    /// `pppGetRotMatrixYZX(out, angles)`.
    pppGetRotMatrixYZX @ 0x0072_ca80 => extern "C" fn ppp_get_rot_matrix_yzx(out: u32, angles: u32) {
        mat::bridge::ppp_get_rot_matrix(&mut mem(), out, angles, order::YZX);
    }
    /// `pppGetRotMatrixZXY(out, angles)`.
    pppGetRotMatrixZXY @ 0x0072_cbe0 => extern "C" fn ppp_get_rot_matrix_zxy(out: u32, angles: u32) {
        mat::bridge::ppp_get_rot_matrix(&mut mem(), out, angles, order::ZXY);
    }
    /// `pppGetRotMatrixZYX(out, angles)`.
    pppGetRotMatrixZYX @ 0x0072_cc70 => extern "C" fn ppp_get_rot_matrix_zyx(out: u32, angles: u32) {
        mat::bridge::ppp_get_rot_matrix(&mut mem(), out, angles, order::ZYX);
    }
    /// `pppMatrixLoop(obj, key, ctrl)`.
    pppMatrixLoop @ 0x0072_d1b0 => extern "C" fn ppp_matrix_loop(obj: u32, key: u32, ctrl: u32) {
        let _ = key;
        mat::bridge::ppp_matrix(&mut mem(), obj, ctrl, order::XYZ);
    }
    /// `pppMatrixXYZ(obj, key, ctrl)`.
    pppMatrixXYZ @ 0x0072_d390 => extern "C" fn ppp_matrix_xyz(obj: u32, key: u32, ctrl: u32) {
        let _ = key;
        mat::bridge::ppp_matrix(&mut mem(), obj, ctrl, order::XYZ);
    }
    /// `pppMatrixXZY(obj, key, ctrl)`.
    pppMatrixXZY @ 0x0072_d570 => extern "C" fn ppp_matrix_xzy(obj: u32, key: u32, ctrl: u32) {
        let _ = key;
        mat::bridge::ppp_matrix(&mut mem(), obj, ctrl, order::XZY);
    }
    /// `pppMatrixYXZ(obj, key, ctrl)`.
    pppMatrixYXZ @ 0x0072_d750 => extern "C" fn ppp_matrix_yxz(obj: u32, key: u32, ctrl: u32) {
        let _ = key;
        mat::bridge::ppp_matrix(&mut mem(), obj, ctrl, order::YXZ);
    }
    /// `pppMatrixYZX(obj, key, ctrl)`.
    pppMatrixYZX @ 0x0072_d930 => extern "C" fn ppp_matrix_yzx(obj: u32, key: u32, ctrl: u32) {
        let _ = key;
        mat::bridge::ppp_matrix(&mut mem(), obj, ctrl, order::YZX);
    }
    /// `pppMatrixZXY(obj, key, ctrl)`.
    pppMatrixZXY @ 0x0072_db10 => extern "C" fn ppp_matrix_zxy(obj: u32, key: u32, ctrl: u32) {
        let _ = key;
        mat::bridge::ppp_matrix(&mut mem(), obj, ctrl, order::ZXY);
    }
    /// `pppMatrixZYX(obj, key, ctrl)`.
    pppMatrixZYX @ 0x0072_dcf0 => extern "C" fn ppp_matrix_zyx(obj: u32, key: u32, ctrl: u32) {
        let _ = key;
        mat::bridge::ppp_matrix(&mut mem(), obj, ctrl, order::ZYX);
    }
    /// `pppMatrixLoc(obj, key, ctrl)`.
    pppMatrixLoc @ 0x0073_4130 => extern "C" fn ppp_matrix_loc(obj: u32, key: u32, ctrl: u32) {
        let _ = key;
        mat::bridge::ppp_matrix_loc(&mut mem(), obj, ctrl);
    }
    /// `pppMatrixScl(obj, key, ctrl)`.
    pppMatrixScl @ 0x0073_4170 => extern "C" fn ppp_matrix_scl(obj: u32, key: u32, ctrl: u32) {
        let _ = key;
        mat::bridge::ppp_matrix_scl(&mut mem(), obj, ctrl);
    }
    /// `pppKeMatSN(obj, key, ctrl)`.
    pppKeMatSN @ 0x0073_4020 => extern "C" fn ppp_ke_mat_sn(obj: u32, key: u32, ctrl: u32) {
        let _ = key;
        mat::bridge::ppp_ke_mat_sn(&mut mem(), obj, ctrl);
    }
    /// `pppKeMatPht(obj, key, ctrl)`.
    pppKeMatPht @ 0x0073_41e0 => extern "C" fn ppp_ke_mat_pht(obj: u32, key: u32, ctrl: u32) {
        let _ = key;
        mat::bridge::ppp_ke_mat_pht(&mut mem(), obj, ctrl);
    }
    /// `pppKeOfsMatXYZ(obj, key, ctrl)`.
    pppKeOfsMatXYZ @ 0x0073_4400 => extern "C" fn ppp_ke_ofs_mat_xyz(obj: u32, key: u32, ctrl: u32) {
        let _ = key;
        mat::bridge::ppp_ke_ofs_mat_xyz(&mut mem(), obj, ctrl);
    }
    /// `pppParMatrix(obj, key)`.
    pppParMatrix @ 0x0073_4640 => extern "C" fn ppp_par_matrix(obj: u32, key: u32) {
        mat::bridge::ppp_par_matrix(&mut mem(), obj, key);
    }
    /// `pppDrawMatrix(obj)`.
    pppDrawMatrix @ 0x0073_4b70 => extern "C" fn ppp_draw_matrix(obj: u32) {
        mat::bridge::ppp_draw_matrix(&mut mem(), obj);
    }
    /// `pppDrawMatrixLoop(obj)`.
    pppDrawMatrixLoop @ 0x0073_4ba0 => extern "C" fn ppp_draw_matrix_loop(obj: u32) {
        mat::bridge::ppp_draw_matrix(&mut mem(), obj);
    }
    /// `pppDrawMatrixFront(obj)`.
    pppDrawMatrixFront @ 0x0073_4bd0 => extern "C" fn ppp_draw_matrix_front(obj: u32) {
        mat::bridge::ppp_draw_matrix_front(&mut mem(), obj);
    }
    /// `pppDrawMatrixNoRot(obj)`.
    pppDrawMatrixNoRot @ 0x0073_4c50 => extern "C" fn ppp_draw_matrix_no_rot(obj: u32) {
        mat::bridge::ppp_draw_matrix_no_rot(&mut mem(), obj);
    }
    /// `pppDrawMatrixWood(obj)`.
    pppDrawMatrixWood @ 0x0073_4cf0 => extern "C" fn ppp_draw_matrix_wood(obj: u32) {
        mat::bridge::ppp_draw_matrix_wood(&mut mem(), obj);
    }
    /// `pppKeZCrctShpDraw(obj, key, ctrl)`: its stack matrix in a buffer
    /// on this shim's stack (the frame pointer at its end; a mode the
    /// original does not know leaves row 3 zero here, stale stack there).
    pppKeZCrctShpDraw @ 0x0073_54c0 => extern "C" fn ppp_ke_z_crct_shp_draw(obj: u32, key: u32, ctrl: u32) {
        #[repr(C, align(16))]
        struct Locals([u8; 0x50]);
        let mut locals = Locals([0; 0x50]);
        #[allow(clippy::cast_possible_truncation)] // a 32-bit process
        let frame = locals.0.as_mut_ptr() as u32 + 0x50;
        mat::bridge::ppp_ke_z_crct_shp_draw(&mut mem(), obj, key, ctrl, frame, &mut |_, f, a| call_ptr(f, a));
    }
    /// `pppKeDMatDraw(obj, key, ctrl)`.
    pppKeDMatDraw @ 0x0073_4eb0 => extern "C" fn ppp_ke_d_mat_draw(obj: u32, key: u32, ctrl: u32) {
        let _ = key;
        mat::bridge::ppp_ke_d_mat_draw(&mut mem(), obj, ctrl);
    }
    /// `pppKeDMatFrDraw(obj, key, ctrl)`.
    pppKeDMatFrDraw @ 0x0073_4ef0 => extern "C" fn ppp_ke_d_mat_fr_draw(obj: u32, key: u32, ctrl: u32) {
        let _ = key;
        mat::bridge::ppp_ke_d_mat_fr_draw(&mut mem(), obj, ctrl);
    }
    /// `pppKeDMatPhtDraw(obj, key, ctrl)`.
    pppKeDMatPhtDraw @ 0x0073_4f80 => extern "C" fn ppp_ke_d_mat_pht_draw(obj: u32, key: u32, ctrl: u32) {
        let _ = key;
        mat::bridge::ppp_ke_d_mat_pht_draw(&mut mem(), obj, ctrl);
    }
    /// `pppKeDMatPhtFrDraw(obj, key, ctrl)`.
    pppKeDMatPhtFrDraw @ 0x0073_5020 => extern "C" fn ppp_ke_d_mat_pht_fr_draw(obj: u32, key: u32, ctrl: u32) {
        let _ = key;
        mat::bridge::ppp_ke_d_mat_pht_fr_draw(&mut mem(), obj, ctrl);
    }
    // ---- tails ----
    /// `pppKeThSft(obj, key, ctrl)`.
    pppKeThSft @ 0x0073_6e90 => extern "C" fn ppp_ke_th_sft(obj: u32, key: u32, ctrl: u32) {
        prog::bridge::ppp_ke_th_sft(&mut mem(), obj, key, ctrl);
    }
    /// `pppKeShpTail(obj, key, ctrl)`.
    pppKeShpTail @ 0x0074_c490 => extern "C" fn ppp_ke_shp_tail(obj: u32, key: u32, ctrl: u32) {
        let _ = key;
        prog::bridge::ppp_ke_shp_tail(&mut mem(), obj, ctrl);
    }
    /// `pppKeShpTailX(obj, key, ctrl)`.
    pppKeShpTailX @ 0x0074_d490 => extern "C" fn ppp_ke_shp_tail_x(obj: u32, key: u32, ctrl: u32) {
        let _ = key;
        prog::bridge::ppp_ke_shp_tail(&mut mem(), obj, ctrl);
    }
    /// `pppKeShpTail2(obj, key, ctrl)`.
    pppKeShpTail2 @ 0x0074_e490 => extern "C" fn ppp_ke_shp_tail2(obj: u32, key: u32, ctrl: u32) {
        let _ = key;
        prog::bridge::ppp_ke_shp_tail(&mut mem(), obj, ctrl);
    }
    /// `pppKeShpTail2X(obj, key, ctrl)`.
    pppKeShpTail2X @ 0x0074_f4d0 => extern "C" fn ppp_ke_shp_tail2_x(obj: u32, key: u32, ctrl: u32) {
        let _ = key;
        prog::bridge::ppp_ke_shp_tail(&mut mem(), obj, ctrl);
    }
    /// `pppKeShpTail3(obj, key, ctrl)`.
    pppKeShpTail3 @ 0x0075_02c0 => extern "C" fn ppp_ke_shp_tail3(obj: u32, key: u32, ctrl: u32) {
        prog::bridge::ppp_ke_shp_tail3(&mut mem(), obj, key, ctrl, prog::bridge::Tail3::Plain);
    }
    /// `pppKeShpTail3X(obj, key, ctrl)`.
    pppKeShpTail3X @ 0x0075_1ca0 => extern "C" fn ppp_ke_shp_tail3_x(obj: u32, key: u32, ctrl: u32) {
        prog::bridge::ppp_ke_shp_tail3(&mut mem(), obj, key, ctrl, prog::bridge::Tail3::X);
    }
    /// `pppKeShpTailPht(obj, key, ctrl)`.
    pppKeShpTailPht @ 0x0075_3f10 => extern "C" fn ppp_ke_shp_tail_pht(obj: u32, key: u32, ctrl: u32) {
        prog::bridge::ppp_ke_shp_tail3(&mut mem(), obj, key, ctrl, prog::bridge::Tail3::Pht);
    }
    /// `pppKeMdlDttCon(obj, ctrl)`.
    pppKeMdlDttCon @ 0x0072_e730 => extern "C" fn ppp_ke_mdl_dtt_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_mdl_dtt_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeShpTailPhtCon2(obj, ctrl)`.
    pppKeShpTailPhtCon2 @ 0x0075_4260 => extern "C" fn ppp_ke_shp_tail_pht_con2(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_shp_tail_pht_con2(&mut mem(), obj, ctrl);
    }
    /// `pppKeShpTail3Con(obj, ctrl)`.
    pppKeShpTail3Con @ 0x0075_0510 => extern "C" fn ppp_ke_shp_tail3_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_shp_tail3_con(&mut mem(), obj, ctrl, &mut |_| crt_rand());
    }
    /// `pppKeShpTail3XCon(obj, ctrl)`.
    pppKeShpTail3XCon @ 0x0075_1eb0 => extern "C" fn ppp_ke_shp_tail3_x_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_shp_tail3_con(&mut mem(), obj, ctrl, &mut |_| crt_rand());
    }
    /// `pppKeShpTailPhtCon(obj, ctrl)`.
    pppKeShpTailPhtCon @ 0x0075_4180 => extern "C" fn ppp_ke_shp_tail_pht_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_shp_tail_pht_con(&mut mem(), obj, ctrl, &mut |_| crt_rand());
    }
    /// `pppKeMdlDttCon2(obj, ctrl)`.
    pppKeMdlDttCon2 @ 0x0072_e760 => extern "C" fn ppp_ke_mdl_dtt_con2(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_mdl_dtt_con2(&mut mem(), obj, ctrl);
    }
    /// `pppKeLnsFlsCon(obj, ctrl)`.
    pppKeLnsFlsCon @ 0x0075_ab30 => extern "C" fn ppp_ke_lns_fls_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_lns_fls_con(&mut mem(), obj, ctrl);
    }

    // ---- oef2 blocks, environments, draw flags ----
    /// `op_oef2_a_get(oef, kind, n)`.
    op_oef2_a_get @ 0x007e_3810 => extern "C" fn op_oef2_a_get(oef: u32, kind: u32, n: u32) -> u32 {
        op::bridge::op_oef2_a_get(&mem(), oef, kind, n)
    }
    /// `op_oef2_set(oef, data)`.
    op_oef2_set @ 0x007e_3b70 => extern "C" fn op_oef2_set(oef: u32, data: u32) {
        op::bridge::op_oef2_set(&mut mem(), oef, data);
    }
    /// `op_oef2_set2(oef)`.
    op_oef2_set2 @ 0x007e_3e30 => extern "C" fn op_oef2_set2(oef: u32) {
        op::bridge::op_oef2_set2(&mut mem(), oef);
    }
    /// `op_oef2_clut_load(clut)`: only a debug message (`dbgPrintf` is
    /// empty) for a null one.
    op_oef2_clut_load @ 0x007e_46b0 => extern "C" fn op_oef2_clut_load(clut: u32) {
        let _ = clut;
    }
    /// `pppInitEnv(env, data, memory, size)`.
    pppInitEnv @ 0x0071_69d0 => extern "C" fn ppp_init_env(env: u32, data: u32, memory: u32, size: u32) {
        heap::bridge::ppp_init_env(&mut mem(), env, data, memory, size);
    }
    /// `pppMemAlloc(heap, spare, size)`.
    pppMemAlloc @ 0x0071_6a30 => extern "C" fn ppp_mem_alloc(heap: u32, spare: u32, size: u32) -> u32 {
        heap::bridge::ppp_mem_alloc(&mut mem(), heap, spare, size)
    }
    /// `op_et_draw_flags_restore(h)`.
    op_et_draw_flags_restore @ 0x007f_c130 => extern "C" fn op_et_draw_flags_restore(h: u32) -> u32 {
        et::bridge::op_et_draw_flags_restore(&mut mem(), h)
    }
    /// `op_et_draw_flags_save(h)`.
    op_et_draw_flags_save @ 0x007f_c190 => extern "C" fn op_et_draw_flags_save(h: u32) -> u32 {
        et::bridge::op_et_draw_flags_save(&mut mem(), h)
    }
    /// `op_et_chr_show(on)`.
    op_et_chr_show @ 0x007f_f090 => extern "C" fn op_et_chr_show(on: u32) {
        et::bridge::op_et_chr_show(&mut mem(), on);
    }

    // ---- small helpers ----
    /// `VFX_Copy16From(dst, src)`: returns the last word.
    VFX_Copy16From @ 0x0071_0e30 => extern "C" fn vfx_copy16_from(dst: u32, src: u32) -> u32 {
        op::bridge::copy16(&mut mem(), dst, src)
    }
    /// `VFX_Copy16To(src, dst)`: returns the last word.
    VFX_Copy16To @ 0x0071_0e70 => extern "C" fn vfx_copy16_to(src: u32, dst: u32) -> u32 {
        op::bridge::copy16(&mut mem(), dst, src)
    }
    /// `VFX_Copy16To_0072f3c0(src, dst)`: the same code.
    VFX_Copy16To_0072f3c0 @ 0x0072_f3c0 => extern "C" fn vfx_copy16_to_0072f3c0(src: u32, dst: u32) -> u32 {
        op::bridge::copy16(&mut mem(), dst, src)
    }
    /// `VFX_ScaleVec3(dst, v, s)`: returns `dst`.
    VFX_ScaleVec3 @ 0x0071_0ea0 => extern "C" fn vfx_scale_vec3(dst: u32, v: u32, s: f32) -> u32 {
        op::bridge::vfx_scale_vec3(&mut mem(), dst, v, s)
    }
    /// `DJB_hash(s)`.
    DJB_hash @ 0x0067_b7d0 => extern "C" fn djb_hash(s: u32) -> u32 {
        op::bridge::djb_hash(&mem(), s)
    }
    /// `opu_id_list_add(unit, id)`: returns the index.
    opu_id_list_add @ 0x0080_0d60 => extern "C" fn opu_id_list_add(unit: u32, id: u32) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // a short argument
        op::bridge::opu_id_list_add(&mut mem(), unit, id as u16)
    }
    /// `op_et_chr_count_same_no_007feb20(h)`.
    op_et_chr_count_same_no_007feb20 @ 0x007f_eb20 => extern "C" fn op_et_chr_count_same_no(h: u32) -> u32 {
        et::bridge::op_et_chr_count_same_no(&mem(), h)
    }
    /// `op_et_chr_index_by_no(no)`.
    op_et_chr_index_by_no @ 0x007f_eaa0 => extern "C" fn op_et_chr_index_by_no(no: u32) -> i32 {
        et::bridge::op_et_chr_index_by_no(&mem(), no)
    }
    /// `opu_eff_part_index(unit)`.
    opu_eff_part_index @ 0x0080_0da0 => extern "C" fn opu_eff_part_index(unit: u32) -> u32 {
        et::bridge::opu_eff_part_index(&mem(), unit)
    }

    /// `pppViewOutCode(x, y)`.
    pppViewOutCode @ 0x0072_8dd0 => extern "C" fn ppp_view_out_code(x: f32, y: f32) -> u32 {
        small::ppp_view_out_code(x, y)
    }
    /// `pppCheckViewCylinder(m, v, far, radius, height)`.
    pppCheckViewCylinder @ 0x0072_8f10 => extern "C" fn ppp_check_view_cylinder(m: u32, v: u32, far: f32, radius: f32, height: f32) -> u32 {
        small::bridge::ppp_check_view_cylinder(&mem(), m, v, far, radius, height)
    }
    /// `pppKeAngleXY_007590e0(v, out)`: `_CIatan` through its import slot.
    pppKeAngleXY_007590e0 @ 0x0075_90e0 => extern "C" fn ppp_ke_angle_xy_007590e0(v: u32, out: u32) -> u32 {
        small::bridge::ppp_angle_xy(&mut mem(), v, out, &mut |_, t| crt_atan(t))
    }
    /// `pppKeAngleXY_0075e1f0(v, out)`: `_CIatan` through its import slot.
    pppKeAngleXY_0075e1f0 @ 0x0075_e1f0 => extern "C" fn ppp_ke_angle_xy_0075e1f0(v: u32, out: u32) -> u32 {
        small::bridge::ppp_angle_xy(&mut mem(), v, out, &mut |_, t| crt_atan(t))
    }
    /// `pppShapeCameraStep_007472b0(obj, key)`.
    pppShapeCameraStep_007472b0 @ 0x0074_72b0 => extern "C" fn ppp_shape_step_007472b0(obj: u32, key: u32) {
        draw::bridge::ppp_shape_step(&mut mem(), draw::bridge::ShapeStep::Camera, obj, key);
    }
    /// `pppShapeCameraDisPosStep_00748500(obj, key)`.
    pppShapeCameraDisPosStep_00748500 @ 0x0074_8500 => extern "C" fn ppp_shape_step_00748500(obj: u32, key: u32) {
        draw::bridge::ppp_shape_step(&mut mem(), draw::bridge::ShapeStep::Camera, obj, key);
    }
    /// `pppShapeFieldRevStep_00743da0(obj, key)`.
    pppShapeFieldRevStep_00743da0 @ 0x0074_3da0 => extern "C" fn ppp_shape_step_00743da0(obj: u32, key: u32) {
        draw::bridge::ppp_shape_step(&mut mem(), draw::bridge::ShapeStep::FieldRev, obj, key);
    }
    /// `pppShapeCameraEmit_00747090(obj, key, ctrl)`.
    pppShapeCameraEmit_00747090 @ 0x0074_7090 => extern "C" fn ppp_shape_camera_emit_00747090(obj: u32, key: u32, ctrl: u32) {
        // the original's local: the inverse camera matrix
        let mut frame = super::StackFrame::<16>::new();
        let at = frame.at();
        draw::bridge::ppp_shape_camera_emit(&mut mem(), obj, key, ctrl, at, &mut |_, f, a| call_ptr(f, a));
    }
    /// `pppShapeCameraDisPosEmit_007482e0(obj, key, ctrl)`.
    pppShapeCameraDisPosEmit_007482e0 @ 0x0074_82e0 => extern "C" fn ppp_shape_camera_emit_007482e0(obj: u32, key: u32, ctrl: u32) {
        // the original's local: the inverse camera matrix
        let mut frame = super::StackFrame::<16>::new();
        let at = frame.at();
        draw::bridge::ppp_shape_camera_emit(&mut mem(), obj, key, ctrl, at, &mut |_, f, a| call_ptr(f, a));
    }
    /// `hashU(s)`.
    hashU @ 0x0071_6570 => extern "C" fn hash_u(s: u32) -> u32 {
        small::bridge::hash_u_at(&mem(), s)
    }
    /// `pppFtoiRgba_0074c280(dst, src)`.
    pppFtoiRgba_0074c280 @ 0x0074_c280 => extern "C" fn ppp_ftoi_rgba_0074c280(dst: u32, src: u32) -> u32 {
        small::bridge::ppp_ftoi_rgba(&mut mem(), dst, src)
    }
    /// `pppFtoiRgba_0074d280(dst, src)`.
    pppFtoiRgba_0074d280 @ 0x0074_d280 => extern "C" fn ppp_ftoi_rgba_0074d280(dst: u32, src: u32) -> u32 {
        small::bridge::ppp_ftoi_rgba(&mut mem(), dst, src)
    }
    /// `pppFtoiRgba_0074e280(dst, src)`.
    pppFtoiRgba_0074e280 @ 0x0074_e280 => extern "C" fn ppp_ftoi_rgba_0074e280(dst: u32, src: u32) -> u32 {
        small::bridge::ppp_ftoi_rgba(&mut mem(), dst, src)
    }
    /// `pppFtoiRgba_0074f2c0(dst, src)`.
    pppFtoiRgba_0074f2c0 @ 0x0074_f2c0 => extern "C" fn ppp_ftoi_rgba_0074f2c0(dst: u32, src: u32) -> u32 {
        small::bridge::ppp_ftoi_rgba(&mut mem(), dst, src)
    }
    /// `pppFtoiRgba_00750130(dst, src)`.
    pppFtoiRgba_00750130 @ 0x0075_0130 => extern "C" fn ppp_ftoi_rgba_00750130(dst: u32, src: u32) -> u32 {
        small::bridge::ppp_ftoi_rgba(&mut mem(), dst, src)
    }
    /// `pppFtoiRgba_00751930(dst, src)`.
    pppFtoiRgba_00751930 @ 0x0075_1930 => extern "C" fn ppp_ftoi_rgba_00751930(dst: u32, src: u32) -> u32 {
        small::bridge::ppp_ftoi_rgba(&mut mem(), dst, src)
    }
    /// `pppFtoiRgba_007535a0(dst, src)`.
    pppFtoiRgba_007535a0 @ 0x0075_35a0 => extern "C" fn ppp_ftoi_rgba_007535a0(dst: u32, src: u32) -> u32 {
        small::bridge::ppp_ftoi_rgba(&mut mem(), dst, src)
    }
    /// `pppKeShpTail3VfCopy_00751680(dst)`.
    pppKeShpTail3VfCopy_00751680 @ 0x0075_1680 => extern "C" fn ppp_ke_shp_tail3_vf_copy(dst: u32) -> u32 {
        small::bridge::ppp_ke_shp_tail3_vf_copy(&mut mem(), dst)
    }
    /// `caculateVFXRenderDataShapeFieldBufferSize(lo, hi, shape, n)`.
    caculateVFXRenderDataShapeFieldBufferSize @ 0x0071_2bd0 => extern "C" fn caculate_vfx_render_data_shape_field_buffer_size(lo: u32, hi: u32, shape: u32, n: u32) -> u32 {
        draw::bridge::caculate_vfx_render_data_shape_field_buffer_size(&mut mem(), [lo, hi, shape, n], &mut |_, f, a| call_ptr(f, a))
    }
    /// `clearVFXRenderDataShp(lo, hi)`.
    clearVFXRenderDataShp @ 0x0071_3290 => extern "C" fn clear_vfx_render_data_shp(lo: u32, hi: u32) -> u32 {
        draw::bridge::clear_vfx_render_data_shp(&mut mem(), lo, hi, &mut |_, f, a| call_ptr(f, a))
    }
    /// `clearVFXRenderDataShpWithMemset(lo, hi)`.
    clearVFXRenderDataShpWithMemset @ 0x0071_3320 => extern "C" fn clear_vfx_render_data_shp_with_memset(lo: u32, hi: u32) -> u32 {
        draw::bridge::clear_vfx_render_data_shp_with_memset(&mut mem(), lo, hi, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppKeShpTail3XVfCopy_00753080(dst)`.
    pppKeShpTail3XVfCopy_00753080 @ 0x0075_3080 => extern "C" fn ppp_ke_shp_tail3x_vf_copy(dst: u32) -> u32 {
        small::bridge::ppp_ke_shp_tail3x_vf_copy(&mut mem(), dst)
    }
    /// `pppKeShpTail3VfLoad_00751740(src)`.
    pppKeShpTail3VfLoad_00751740 @ 0x0075_1740 => extern "C" fn ppp_ke_shp_tail3_vf_load(src: u32) -> u32 {
        small::bridge::ppp_ke_shp_tail3_vf_load(&mut mem(), src)
    }
    /// `pppKeShpTail3XVfLoad_00751ab0(src)`: the same code.
    pppKeShpTail3XVfLoad_00751ab0 @ 0x0075_1ab0 => extern "C" fn ppp_ke_shp_tail3x_vf_load(src: u32) -> u32 {
        small::bridge::ppp_ke_shp_tail3_vf_load(&mut mem(), src)
    }
    /// `pppDataLastPair_00761250(data, a, b)`.
    pppDataLastPair_00761250 @ 0x0076_1250 => extern "C" fn ppp_data_last_pair(data: u32, a: u32, b: u32) -> u32 {
        small::bridge::ppp_data_last_pair(&mut mem(), data, a, b)
    }
    /// `pppDataSetTex_007628c0(data, base, a, b)`.
    pppDataSetTex_007628c0 @ 0x0076_28c0 => extern "C" fn ppp_data_set_tex(data: u32, base: u32, a: u32, b: u32) -> u32 {
        small::bridge::ppp_data_set_tex(&mut mem(), data, base, a, b)
    }
    // ---- VU0 macro helpers ----
    /// `pppVu0MaddaY(_, v, _, s)`: returns `v`.
    pppVu0MaddaY @ 0x0072_c590 => extern "C" fn ppp_vu0_madda_y(a: u32, v: u32, b: u32, s: f32) -> u32 {
        let _ = (a, b);
        mat::bridge::ppp_vu0_madda(&mut mem(), v, s);
        v
    }
    /// `pppVu0MaddaZ(_, v, _, _, s)`: returns `v`.
    pppVu0MaddaZ @ 0x0072_c5f0 => extern "C" fn ppp_vu0_madda_z(a: u32, v: u32, b: u32, c: u32, s: f32) -> u32 {
        let _ = (a, b, c);
        mat::bridge::ppp_vu0_madda(&mut mem(), v, s);
        v
    }
    /// `pppVu0MaddaY_copy_0075e670(_, v, _, s)`: `pppVu0MaddaY`'s code
    /// again; returns `v`.
    pppVu0MaddaY_copy_0075e670 @ 0x0075_e670 => extern "C" fn ppp_vu0_madda_y_copy(a: u32, v: u32, b: u32, s: f32) -> u32 {
        let _ = (a, b);
        mat::bridge::ppp_vu0_madda(&mut mem(), v, s);
        v
    }
    /// `pppVu0MaddaZ_copy_0075e6d0(_, v, _, _, s)`: `pppVu0MaddaZ`'s code
    /// again; returns `v`.
    pppVu0MaddaZ_copy_0075e6d0 @ 0x0075_e6d0 => extern "C" fn ppp_vu0_madda_z_copy(a: u32, v: u32, b: u32, c: u32, s: f32) -> u32 {
        let _ = (a, b, c);
        mat::bridge::ppp_vu0_madda(&mut mem(), v, s);
        v
    }
    /// `pppVu0MaddW(dst, v, _, _, _, s)`: returns `dst`.
    pppVu0MaddW @ 0x0072_c650 => extern "C" fn ppp_vu0_madd_w(dst: u32, v: u32, a: u32, b: u32, c: u32, s: f32) -> u32 {
        let _ = (a, b, c);
        mat::bridge::ppp_vu0_madd(&mut mem(), dst, v, s);
        dst
    }
    /// `pppVu0MulX(dst, v, s)`: returns `dst`.
    pppVu0MulX @ 0x0072_c6a0 => extern "C" fn ppp_vu0_mul_x(dst: u32, v: u32, s: f32) -> u32 {
        mat::bridge::ppp_vu0_mul(&mut mem(), dst, v, s);
        dst
    }
    /// `pppVu0MulX_007500d0(dst, v, s)`: returns `v`.
    pppVu0MulX_007500d0 @ 0x0075_00d0 => extern "C" fn ppp_vu0_mul_x_007500d0(dst: u32, v: u32, s: f32) -> u32 {
        mat::bridge::ppp_vu0_mul(&mut mem(), dst, v, s);
        v
    }
    /// `pppVu0AddY(dst, v, _, s)`: returns `dst`.
    pppVu0AddY @ 0x0074_b630 => extern "C" fn ppp_vu0_add_y(dst: u32, v: u32, a: u32, s: f32) -> u32 {
        let _ = a;
        mat::bridge::ppp_vu0_add_x(&mut mem(), dst, v, s);
        dst
    }
    /// `pppVu0AddZ(dst, v, _, _, s)`: returns `dst`.
    pppVu0AddZ @ 0x0074_b650 => extern "C" fn ppp_vu0_add_z(dst: u32, v: u32, a: u32, b: u32, s: f32) -> u32 {
        let _ = (a, b);
        mat::bridge::ppp_vu0_add_x(&mut mem(), dst, v, s);
        dst
    }
    /// `op_strlen(s)`.
    op_strlen @ 0x007e_2250 => extern "C" fn op_strlen(s: u32) -> u32 {
        op::bridge::op_strlen(&mem(), s)
    }
    /// `lmemcpy(dst, src, n)`: returns the end of `dst`.
    lmemcpy @ 0x0076_b0f0 => extern "C" fn lmemcpy(dst: u32, src: u32, n: u32) -> u32 {
        op::bridge::lmemcpy(&mut mem(), dst, src, n)
    }
    /// `pppFpSetDisp(on)`.
    pppFpSetDisp @ 0x0072_9050 => extern "C" fn ppp_fp_set_disp(on: u32) -> u32 {
        fp::bridge::ppp_fp_set_disp(&mut mem(), on)
    }
    /// `setVfxShapeEnlargeBufferFactor(n)`.
    setVfxShapeEnlargeBufferFactor @ 0x0071_7610 => extern "C" fn set_vfx_shape_enlarge_buffer_factor(n: u32) -> u32 {
        op::bridge::set_vfx_shape_enlarge_buffer_factor(&mut mem(), n)
    }
    /// `resetVfxShapeEnlargeBufferFactor()`.
    resetVfxShapeEnlargeBufferFactor @ 0x0071_7600 => extern "C" fn reset_vfx_shape_enlarge_buffer_factor() {
        op::bridge::set_vfx_shape_enlarge_buffer_factor(&mut mem(), 1);
    }
    /// `opu_eff_free_by_id(id)`.
    opu_eff_free_by_id @ 0x007f_e920 => extern "C" fn opu_eff_free_by_id(id: u32) {
        et::bridge::opu_eff_free_by_id(&mut mem(), id);
    }
    /// `opus_chr_index_map(out)`: how many moved.
    opus_chr_index_map @ 0x007f_e450 => extern "C" fn opus_chr_index_map(out: u32) -> u32 {
        et::bridge::opus_chr_index_map(&mut mem(), out)
    }

    // ---- round 5: renderer flags, EE/VU emulation, rotations, packets ----
    /// `calculateVFXSP_ZMat(z)`.
    calculateVFXSP_ZMat @ 0x0071_2d00 => extern "C" fn calculate_vfx_sp_zmat(z: u32) -> u32 {
        op::bridge::calculate_vfx_sp_zmat(z)
    }
    /// `calculateVFXSP_AlphaFunction(f)`.
    calculateVFXSP_AlphaFunction @ 0x0071_2c30 => extern "C" fn calculate_vfx_sp_alpha_function(f: u32) -> u32 {
        op::bridge::calculate_vfx_sp_alpha_function(f)
    }
    /// `graphicVFXGetRenderDataByGeometry(g)`.
    graphicVFXGetRenderDataByGeometry @ 0x0064_53b0 => extern "C" fn graphic_vfx_get_render_data_by_geometry(g: u32) -> u32 {
        op::bridge::graphic_vfx_get_render_data_by_geometry(&mem(), g)
    }
    /// `graphicVFXSetCurrentVFXType(type)`.
    graphicVFXSetCurrentVFXType @ 0x0064_5670 => extern "C" fn graphic_vfx_set_current_vfx_type(kind: u32) {
        #[allow(clippy::cast_possible_truncation)] // a byte argument
        op::bridge::CURRENT_TYPE.set(&mut mem(), kind as u8);
    }
    /// `pppEePmulth(dst, a, b)`.
    pppEePmulth @ 0x0073_74e0 => extern "C" fn ppp_ee_pmulth(dst: u32, a: u32, b: u32) {
        mat::bridge::ppp_ee_pmulth(&mut mem(), dst, a, b);
    }
    /// `pppEePpacb(dst, a, b)`.
    pppEePpacb @ 0x0073_75d0 => extern "C" fn ppp_ee_ppacb(dst: u32, a: u32, b: u32) {
        mat::bridge::ppp_ee_ppacb(&mut mem(), dst, a, b);
    }
    /// `pppMemFree(heap, spare, data)`.
    pppMemFree @ 0x0071_6a90 => extern "C" fn ppp_mem_free(heap: u32, spare: u32, data: u32) {
        heap::bridge::ppp_mem_free(&mut mem(), heap, spare, data);
    }
    /// `pppVtMimeDes(obj, ctrl)`.
    pppVtMimeDes @ 0x0072_ede0 => extern "C" fn ppp_vt_mime_des(obj: u32, ctrl: u32) {
        prog::bridge::ppp_vt_mime_des(&mut mem(), obj, ctrl);
    }
    /// `pppKeLnsLpCon(obj, ctrl)`.
    pppKeLnsLpCon @ 0x0075_af30 => extern "C" fn ppp_ke_lns_lp_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_lns_lp_con(&mut mem(), obj, ctrl);
    }
    /// `op_rot_m_x(dst, src, angle)`.
    op_rot_m_x @ 0x007e_9580 => extern "C" fn op_rot_m_x(dst: u32, src: u32, angle: f32) {
        op::bridge::op_rt_m(&mut mem(), dst, src, angle, mat::Axis::X);
    }
    /// `op_rot_m_y(dst, src, angle)`.
    op_rot_m_y @ 0x007e_9670 => extern "C" fn op_rot_m_y(dst: u32, src: u32, angle: f32) {
        op::bridge::op_rt_m(&mut mem(), dst, src, angle, mat::Axis::Y);
    }
    /// `op_rot_m_z(dst, src, angle)`.
    op_rot_m_z @ 0x007e_9760 => extern "C" fn op_rot_m_z(dst: u32, src: u32, angle: f32) {
        op::bridge::op_rt_m(&mut mem(), dst, src, angle, mat::Axis::Z);
    }
    /// `op_rt_m_yxz(dst, src, angles, t)`.
    op_rt_m_yxz @ 0x007e_9a30 => extern "C" fn op_rt_m_yxz(dst: u32, src: u32, angles: u32, t: u32) {
        op::bridge::op_rt_m_yxz(&mut mem(), dst, src, angles, t);
    }
    /// `op_md_tex0_change(packet, tex0)`.
    op_md_tex0_change @ 0x00a6_9c80 => extern "C" fn op_md_tex0_change(packet: u32, _tex0: u32) -> u32 {
        op::bridge::op_md_tex0_change(&mut mem(), packet)
    }
    /// `op_md_tex0_change_limit(packet, tex0, lo, hi)`.
    op_md_tex0_change_limit @ 0x00a6_9ca0 => extern "C" fn op_md_tex0_change_limit(packet: u32, _tex0: u32, lo: u32, hi: u32) -> u32 {
        op::bridge::op_md_tex0_change_limit(&mut mem(), packet, lo, hi)
    }
    /// `op_md_packet_set_addr(packet, a, b)`.
    op_md_packet_set_addr @ 0x00a6_9de0 => extern "C" fn op_md_packet_set_addr(packet: u32, a: u32, b: u32) {
        op::bridge::op_md_packet_set_addr(&mut mem(), packet, a, b);
    }
    /// `pppSetMatrix(part)`.
    pppSetMatrix @ 0x0071_7410 => extern "C" fn ppp_set_matrix(part: u32) {
        mat::bridge::ppp_set_matrix(&mut mem(), part);
    }
    /// `pppVertexAttend(obj, key, ctrl)`.
    pppVertexAttend @ 0x0075_d340 => extern "C" fn ppp_vertex_attend(obj: u32, key: u32, ctrl: u32) {
        mat::bridge::ppp_vertex_attend(&mut mem(), obj, key, ctrl);
    }

    // ---- parts and objects ----
    /// `pppInitPdt(pdt, programs)`.
    pppInitPdt @ 0x0071_20f0 => extern "C" fn ppp_init_pdt(pdt: u32, programs: u32) {
        part::bridge::ppp_init_pdt(&mut mem(), pdt, programs);
    }
    /// `pppInitData(data, programs, _)`.
    pppInitData @ 0x0071_1fa0 => extern "C" fn ppp_init_data(data: u32, programs: u32, x: u32) {
        let _ = x;
        part::bridge::ppp_init_data(&mut mem(), data, programs);
    }
    /// `pppOtFunc_Reg(list, node, a, b)`; returns `node`.
    pppOtFunc_Reg @ 0x0071_6be0 => extern "C" fn ppp_ot_func_reg(list: u32, node: u32, a: u32, b: u32) -> u32 {
        part::bridge::ppp_ot_func_reg(&mut mem(), list, node, a, b);
        // the original leaves set_ptr32(node) in eax (the TCon set-ups return it)
        node
    }
    /// `pppPObjectCon(obj)`.
    pppPObjectCon @ 0x0071_2d90 => extern "C" fn ppp_pobject_con(obj: u32) {
        part::bridge::ppp_pobject_con(&mut mem(), obj, 0x1c, &mut |_, f, a| call_ptr(f, a));
    }
    /// `pppPObjectCon2(obj)`.
    pppPObjectCon2 @ 0x0071_2d20 => extern "C" fn ppp_pobject_con2(obj: u32) {
        part::bridge::ppp_pobject_con(&mut mem(), obj, 0x20, &mut |_, f, a| call_ptr(f, a));
    }
    /// `pppPartCall10(part)`.
    pppPartCall10 @ 0x0071_2e40 => extern "C" fn ppp_part_call10(p: u32) {
        part::bridge::ppp_part_call(&mut mem(), p, 0x10, &mut |_, f, a| call_ptr(f, a));
    }
    /// `pppPartCall14(part)`.
    pppPartCall14 @ 0x0071_2e80 => extern "C" fn ppp_part_call14(p: u32) {
        part::bridge::ppp_part_call(&mut mem(), p, 0x14, &mut |_, f, a| call_ptr(f, a));
    }
    /// `pppPartCall18(part)`.
    pppPartCall18 @ 0x0071_2e00 => extern "C" fn ppp_part_call18(p: u32) {
        part::bridge::ppp_part_call(&mut mem(), p, 0x18, &mut |_, f, a| call_ptr(f, a));
    }
    /// `pppCreatePObject(part, def)`: the object, or 0.
    pppCreatePObject @ 0x0071_6770 => extern "C" fn ppp_create_pobject(p: u32, def: u32) -> u32 {
        part::bridge::ppp_create_pobject(&mut mem(), p, def, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppBornPObjects(part)`.
    pppBornPObjects @ 0x0071_2b80 => extern "C" fn ppp_born_pobjects(p: u32) {
        part::bridge::ppp_born_pobjects(&mut mem(), p, &mut |_, f, a| call_ptr(f, a));
    }
    /// `pppDeletePObject(obj, _)`.
    pppDeletePObject @ 0x0071_68f0 => extern "C" fn ppp_delete_pobject(obj: u32, x: u32) {
        let _ = x;
        part::bridge::ppp_delete_pobject(&mut mem(), obj, &mut |_, f, a| call_ptr(f, a));
    }
    /// `pppDrawShape(def, prog, entry, c)`.
    pppDrawShape @ 0x0074_09e0 => extern "C" fn ppp_draw_shape(def: u32, prog: u32, e: u32, c: u32) -> u32 {
        let draw = part::bridge::DRAW_SHAPE0;
        part::bridge::ppp_draw_shape(&mut mem(), draw, def, prog, e, c, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppDrawShapeX(def, prog, entry, c)`.
    pppDrawShapeX @ 0x0074_0ed0 => extern "C" fn ppp_draw_shape_x(def: u32, prog: u32, e: u32, c: u32) -> u32 {
        let draw = part::bridge::DRAW_SHAPE_X0;
        part::bridge::ppp_draw_shape(&mut mem(), draw, def, prog, e, c, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppOtFunc_Control(list)`.
    pppOtFunc_Control @ 0x0071_6ac0 => extern "C" fn ppp_ot_func_control(list: u32) {
        part::bridge::ppp_ot_func_control(&mut mem(), list, &mut |_, f, a| call_ptr(f, a));
    }
    /// `pppOtFunc_Lev(list, node)`; returns `list` as the original leaves it.
    pppOtFunc_Lev @ 0x0071_6ba0 => extern "C" fn ppp_ot_func_lev(list: u32, node: u32) -> u32 {
        part::bridge::ppp_ot_func_lev(&mut mem(), list, node);
        list
    }
    /// `pppKeLnsArndTCon(obj, ctrl)`.
    pppKeLnsArndTCon @ 0x0075_a0f0 => extern "C" fn ppp_ke_lns_arnd_t_con(obj: u32, ctrl: u32) -> u32 {
        prog::bridge::ppp_ke_lns_t_con(&mut mem(), prog::bridge::LnsT::Arnd, obj, ctrl)
    }
    /// `pppKeLnsClmTCon(obj, ctrl)`.
    pppKeLnsClmTCon @ 0x0075_a4a0 => extern "C" fn ppp_ke_lns_clm_t_con(obj: u32, ctrl: u32) -> u32 {
        prog::bridge::ppp_ke_lns_t_con(&mut mem(), prog::bridge::LnsT::Clm, obj, ctrl)
    }
    /// `pppKeLnsCrnTCon(obj, ctrl)`.
    pppKeLnsCrnTCon @ 0x0075_a870 => extern "C" fn ppp_ke_lns_crn_t_con(obj: u32, ctrl: u32) -> u32 {
        prog::bridge::ppp_ke_lns_t_con(&mut mem(), prog::bridge::LnsT::Crn, obj, ctrl)
    }
    /// `pppKeLnsFlsTCon(obj, ctrl)`.
    pppKeLnsFlsTCon @ 0x0075_ac60 => extern "C" fn ppp_ke_lns_fls_t_con(obj: u32, ctrl: u32) -> u32 {
        prog::bridge::ppp_ke_lns_t_con(&mut mem(), prog::bridge::LnsT::Fls, obj, ctrl)
    }
    /// `pppKeLnsArndTDes(obj, ctrl)`.
    pppKeLnsArndTDes @ 0x0075_a140 => extern "C" fn ppp_ke_lns_arnd_t_des(obj: u32, ctrl: u32) -> u32 {
        let kind = prog::bridge::LnsT::Arnd;
        prog::bridge::ppp_ke_lns_t_des(&mut mem(), kind, obj, ctrl, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppKeLnsClmTDes(obj, ctrl)`.
    pppKeLnsClmTDes @ 0x0075_a4f0 => extern "C" fn ppp_ke_lns_clm_t_des(obj: u32, ctrl: u32) -> u32 {
        let kind = prog::bridge::LnsT::Clm;
        prog::bridge::ppp_ke_lns_t_des(&mut mem(), kind, obj, ctrl, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppKeLnsCrnTDes(obj, ctrl)`.
    pppKeLnsCrnTDes @ 0x0075_a8b0 => extern "C" fn ppp_ke_lns_crn_t_des(obj: u32, ctrl: u32) -> u32 {
        let kind = prog::bridge::LnsT::Crn;
        prog::bridge::ppp_ke_lns_t_des(&mut mem(), kind, obj, ctrl, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppKeLnsFlsTDes(obj, ctrl)`.
    pppKeLnsFlsTDes @ 0x0075_acb0 => extern "C" fn ppp_ke_lns_fls_t_des(obj: u32, ctrl: u32) -> u32 {
        let kind = prog::bridge::LnsT::Fls;
        prog::bridge::ppp_ke_lns_t_des(&mut mem(), kind, obj, ctrl, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppKeLnsLpTCon(obj, ctrl)`.
    pppKeLnsLpTCon @ 0x0075_b090 => extern "C" fn ppp_ke_lns_lp_t_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_lns_lp_t_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeLnsLpTDes(obj, ctrl)`.
    pppKeLnsLpTDes @ 0x0075_b130 => extern "C" fn ppp_ke_lns_lp_t_des(obj: u32, ctrl: u32) -> u32 {
        prog::bridge::ppp_ke_lns_lp_t_des(&mut mem(), obj, ctrl)
    }
    /// `pppAllFreePObject(part)`.
    pppAllFreePObject @ 0x0071_1f20 => extern "C" fn ppp_all_free_pobject(p: u32) {
        part::bridge::ppp_all_free_pobject(&mut mem(), p, &mut |_, f, a| call_ptr(f, a));
    }
    /// `pppRunPartStd(part)`.
    pppRunPartStd @ 0x0071_6c40 => extern "C" fn ppp_run_part_std(p: u32) {
        part::bridge::ppp_run_part_std(&mut mem(), p, &mut |_, f, a| call_ptr(f, a));
    }
    /// `pppRunPart(part, flags)`: 1 when the part has ended.
    pppRunPart @ 0x0071_2250 => extern "C" fn ppp_run_part(p: u32, flags: u32) -> u32 {
        part::bridge::ppp_run_part(&mut mem(), p, flags & 0xff, &mut |_, f, a| call_ptr(f, a))
    }
    /// `pppStartPart(part, rate, data, call_set_ups)`.
    pppStartPart @ 0x0071_23c0 => extern "C" fn ppp_start_part(p: u32, rate: u32, data: u32, set_ups: u32) {
        part::bridge::ppp_start_part(&mut mem(), p, rate, data, set_ups, &mut |_, f, a| call_ptr(f, a));
    }
    /// `pppCallUsrFunc(part, table, arg)`.
    pppCallUsrFunc @ 0x0071_6710 => extern "C" fn ppp_call_usr_func(p: u32, table: u32, arg: u32) {
        part::bridge::ppp_call_usr_func(&mut mem(), p, table, arg, &mut |_, f, a| call_ptr(f, a));
    }
    /// `pppDrawHookDraw(obj, key, ctrl)`.
    pppDrawHookDraw @ 0x0075_7370 => extern "C" fn ppp_draw_hook(obj: u32, key: u32, ctrl: u32) {
        part::bridge::ppp_draw_hook(&mut mem(), obj, key, ctrl, &mut |_, f, a| call_ptr(f, a));
    }
    /// `pppPointAp(obj, key, ctrl)`.
    pppPointAp @ 0x0075_73c0 => extern "C" fn ppp_point_ap(obj: u32, key: u32, ctrl: u32) {
        part::bridge::ppp_point_ap(&mut mem(), obj, key, ctrl, &mut |_, f, a| call_ptr(f, a));
    }
    /// `pppBornShort(obj, key, v)`.
    pppBornShort @ 0x0075_8270 => extern "C" fn ppp_born_short(obj: u32, key: u32, v: u32) {
        #[allow(clippy::cast_possible_truncation)] // a short argument
        part::bridge::ppp_born_short(&mut mem(), obj, key, v as u16, &mut |_, f, a| call_ptr(f, a));
    }
    /// `pppBornVertex(obj, key, v)`.
    pppBornVertex @ 0x0075_7770 => extern "C" fn ppp_born_vertex(obj: u32, key: u32, v: u32) {
        part::bridge::ppp_born_vertex(&mut mem(), obj, key, v, false, &mut |_, f, a| call_ptr(f, a));
    }
    /// `pppBornVertexRel(obj, key, v)`.
    pppBornVertexRel @ 0x0075_7b50 => extern "C" fn ppp_born_vertex_rel(obj: u32, key: u32, v: u32) {
        part::bridge::ppp_born_vertex(&mut mem(), obj, key, v, true, &mut |_, f, a| call_ptr(f, a));
    }
    /// `pppPointRAp(obj, key, ctrl)`.
    pppPointRAp @ 0x0075_7490 => extern "C" fn ppp_point_r_ap(obj: u32, key: u32, ctrl: u32) {
        part::bridge::ppp_point_r_ap(
            &mut mem(),
            obj,
            key,
            ctrl,
            &mut |_, f, a| call_ptr(f, a),
            &mut |_, seed| crt_srand(seed),
        );
    }
    /// `pppKeHitBorn(obj, key, ctrl)`.
    pppKeHitBorn @ 0x0075_9d80 => extern "C" fn ppp_ke_hit_born(obj: u32, key: u32, ctrl: u32) {
        part::bridge::ppp_ke_hit_born(&mut mem(), obj, key, ctrl, &mut |_, f, a| call_ptr(f, a));
    }
    /// `pppBornVertexLc(obj, key, v)`.
    pppBornVertexLc @ 0x0075_7ed0 => extern "C" fn ppp_born_vertex_lc(obj: u32, key: u32, v: u32) {
        part::bridge::ppp_born_vertex_lc(&mut mem(), obj, key, v, &mut |_, f, a| call_ptr(f, a));
    }
    /// `pppVertexAp(obj, key, ctrl)`.
    pppVertexAp @ 0x0075_7840 => extern "C" fn ppp_vertex_ap(obj: u32, key: u32, ctrl: u32) {
        part::bridge::ppp_vertex_ap(
            &mut mem(),
            obj,
            key,
            ctrl,
            part::bridge::VertexAp::Ap,
            &mut |_, f, a| call_ptr(f, a),
            &mut |_, seed| crt_srand(seed),
        );
    }
    /// `pppVertexApDisPos(obj, key, ctrl)`.
    pppVertexApDisPos @ 0x0075_7c30 => extern "C" fn ppp_vertex_ap_dis_pos(obj: u32, key: u32, ctrl: u32) {
        part::bridge::ppp_vertex_ap(
            &mut mem(),
            obj,
            key,
            ctrl,
            part::bridge::VertexAp::DisPos,
            &mut |_, f, a| call_ptr(f, a),
            &mut |_, seed| crt_srand(seed),
        );
    }
    /// `pppVertexApLc(obj, key, ctrl)`.
    pppVertexApLc @ 0x0075_7ff0 => extern "C" fn ppp_vertex_ap_lc(obj: u32, key: u32, ctrl: u32) {
        part::bridge::ppp_vertex_ap(
            &mut mem(),
            obj,
            key,
            ctrl,
            part::bridge::VertexAp::Lc,
            &mut |_, f, a| call_ptr(f, a),
            &mut |_, seed| crt_srand(seed),
        );
    }
    /// `pppVertexApAt(obj, key, ctrl)`.
    pppVertexApAt @ 0x0075_82e0 => extern "C" fn ppp_vertex_ap_at(obj: u32, key: u32, ctrl: u32) {
        part::bridge::ppp_vertex_ap(
            &mut mem(),
            obj,
            key,
            ctrl,
            part::bridge::VertexAp::At,
            &mut |_, f, a| call_ptr(f, a),
            &mut |_, seed| crt_srand(seed),
        );
    }
    /// `op_et_ffx_init(oef2)`.
    op_et_ffx_init @ 0x007f_c620 => extern "C" fn op_et_ffx_init(oef2: u32) -> u32 {
        et::bridge::op_et_ffx_init(&mut mem(), oef2, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_et_battle_init(oef2)`.
    op_et_battle_init @ 0x007f_b250 => extern "C" fn op_et_battle_init(oef2: u32) -> u32 {
        et::bridge::op_et_battle_init(&mut mem(), oef2, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_et_encount_effect_on(part, id)`.
    op_et_encount_effect_on @ 0x007f_c590 => extern "C" fn op_et_encount_effect_on(part: u32, id: u32) -> u32 {
        et::bridge::op_et_encount_effect_on(&mut mem(), part, id, &mut |_, f, a| call_ptr(f, a)).unwrap_or(0)
    }
    /// `op_et_battle_effect_free()`.
    op_et_battle_effect_free @ 0x007f_b170 => extern "C" fn op_et_battle_effect_free() -> u32 {
        et::bridge::op_et_battle_effect_free(&mut mem(), &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_et_weapon_init(oef2, no)`.
    op_et_weapon_init @ 0x007f_cb60 => extern "C" fn op_et_weapon_init(oef2: u32, no: u32) -> u32 {
        et::bridge::op_et_weapon_init(&mut mem(), oef2, no, &mut |_, f, a| call_ptr(f, a))
    }
    /// `op_et_bindeff2(h)`.
    op_et_bindeff2 @ 0x007f_b360 => extern "C" fn op_et_bindeff2(h: u32) -> u32 {
        et::bridge::op_et_bindeff2(&mut mem(), h, &mut |_, f, a| call_ptr(f, a)).unwrap_or(0)
    }
    /// `op_et_monster_init(h, no)`.
    op_et_monster_init @ 0x007f_c780 => extern "C" fn op_et_monster_init(h: u32, no: u32) -> u32 {
        et::bridge::op_et_monster_init(&mut mem(), h, no, &mut |_, f, a| call_ptr(f, a))
    }
    /// `opus_end_all_sub(k)`.
    opus_end_all_sub @ 0x007f_e6c0 => extern "C" fn opus_end_all_sub(k: u32) {
        et::bridge::opus_end_all_sub(&mut mem(), k, &mut |_, f, a| call_ptr(f, a));
    }
    /// `op_segment_circle_cross_007e8fd0(x0, y0, r, x1, y1, x2, y2, out, out2)`.
    op_segment_circle_cross_007e8fd0 @ 0x007e_8fd0 => extern "C" fn op_segment_circle_cross(x0: u32, y0: u32, r: u32, x1: u32, y1: u32, x2: u32, y2: u32, out: u32, out2: u32) -> u32 {
        // the original's locals: the crossings and the three lengths
        let mut frame = super::StackFrame::<8>::new();
        let at = frame.at();
        let p = [x0, y0, r, x1, y1, x2, y2].map(f32::from_bits);
        op::bridge::op_segment_circle_cross(&mut mem(), p, out, out2, at, &mut |_, f, a| call_ptr(f, a), &mut |_, f, a| call_ptr_f64(f, a))
    }
    /// `vu_apply_matrix_0074bff0(out, m, v)`.
    vu_apply_matrix_0074bff0 @ 0x0074_bff0 => extern "C" fn vu_apply_matrix_0074bff0(out: u32, m: u32, v: u32) -> u32 {
        vec::bridge::vu_apply_matrix(&mut mem(), out, m, v)
    }
    /// `vu_apply_matrix_0074cff0(out, m, v)`.
    vu_apply_matrix_0074cff0 @ 0x0074_cff0 => extern "C" fn vu_apply_matrix_0074cff0(out: u32, m: u32, v: u32) -> u32 {
        vec::bridge::vu_apply_matrix(&mut mem(), out, m, v)
    }
    /// `vu_apply_matrix_0074dff0(out, m, v)`.
    vu_apply_matrix_0074dff0 @ 0x0074_dff0 => extern "C" fn vu_apply_matrix_0074dff0(out: u32, m: u32, v: u32) -> u32 {
        vec::bridge::vu_apply_matrix(&mut mem(), out, m, v)
    }
    /// `vu_apply_matrix_0074f030(out, m, v)`.
    vu_apply_matrix_0074f030 @ 0x0074_f030 => extern "C" fn vu_apply_matrix_0074f030(out: u32, m: u32, v: u32) -> u32 {
        vec::bridge::vu_apply_matrix(&mut mem(), out, m, v)
    }
    /// `vu_apply_matrix_00753310(out, m, v)`.
    vu_apply_matrix_00753310 @ 0x0075_3310 => extern "C" fn vu_apply_matrix_00753310(out: u32, m: u32, v: u32) -> u32 {
        vec::bridge::vu_apply_matrix(&mut mem(), out, m, v)
    }
    /// `op_et_astatus_effect_change(h, status)`.
    op_et_astatus_effect_change @ 0x007f_aed0 => extern "C" fn op_et_astatus_effect_change(h: u32, status: u32) -> u32 {
        et::bridge::op_et_astatus_effect_change(&mut mem(), h, status, &mut |_, f, a| call_ptr(f, a)).unwrap_or(0)
    }
    /// `ops_init_draw_work()`.
    ops_init_draw_work @ 0x007e_c890 => extern "C" fn ops_init_draw_work() -> u32 {
        op::bridge::ops_init_draw_work(&mut mem(), &mut |_, f, a| call_ptr(f, a)).unwrap_or(0)
    }
    /// `pppStartPartFromYonish(part, rate, data, set_ups)`.
    pppStartPartFromYonish @ 0x0071_2560 => extern "C" fn ppp_start_part_from_yonish(p: u32, rate: u32, data: u32, set_ups: u32) {
        part::bridge::ppp_start_part_from_yonish(&mut mem(), p, rate, data, set_ups, &mut |_, f, a| call_ptr(f, a));
    }
}
