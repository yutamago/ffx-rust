//! `ffx-vfx`: particle and effect functions. All cdecl; functions whose
//! original leaves a result in `eax` return the full `u32`.

use super::{mem, ported};
use ffx_vfx::mat::{self, order};
use ffx_vfx::rand::bridge::{Kind, Spread};
use ffx_vfx::{et, fp, heap, op, opd, opus, part, ppp, prog, rand};

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
/// `+4`..`+0x24`, `TOGetFFXLang`, `graphicVFXDestroy`): cdecl, up to four
/// arguments; returns `eax`.
#[allow(clippy::many_single_char_names)] // the arguments
fn call_ptr(f: u32, args: &[u32]) -> u32 {
    // SAFETY: `f` is a function the game stored in its particle program
    // table, or one of its own; the originals call it as cdecl with these
    // arguments.
    unsafe {
        match *args {
            [] => {
                let f: unsafe extern "C" fn() -> u32 = std::mem::transmute(f as usize);
                f()
            }
            [a] => {
                let f: unsafe extern "C" fn(u32) -> u32 = std::mem::transmute(f as usize);
                f(a)
            }
            [a, b] => {
                let f: unsafe extern "C" fn(u32, u32) -> u32 = std::mem::transmute(f as usize);
                f(a, b)
            }
            [a, b, c] => {
                let f: unsafe extern "C" fn(u32, u32, u32) -> u32 = std::mem::transmute(f as usize);
                f(a, b, c)
            }
            [a, b, c, d] => {
                let f: unsafe extern "C" fn(u32, u32, u32, u32) -> u32 =
                    std::mem::transmute(f as usize);
                f(a, b, c, d)
            }
            _ => unreachable!("program functions take up to four arguments"),
        }
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
    /// `op_et_eff_off_signal(id)`.
    op_et_eff_off_signal @ 0x007f_c450 => extern "C" fn op_et_eff_off_signal(id: u32) {
        et::bridge::op_et_eff_off_signal(&mut mem(), id);
    }
    /// `op_et_weapon_afterimage_fade(id)`.
    op_et_weapon_afterimage_fade @ 0x007f_ca40 => extern "C" fn op_et_weapon_afterimage_fade(id: u32) {
        et::bridge::op_et_weapon_afterimage_fade(&mut mem(), id);
    }

    // ---- opus ----
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
    /// `op_et_chr_index_by_no(no)`.
    op_et_chr_index_by_no @ 0x007f_eaa0 => extern "C" fn op_et_chr_index_by_no(no: u32) -> i32 {
        et::bridge::op_et_chr_index_by_no(&mem(), no)
    }
    /// `opu_eff_part_index(unit)`.
    opu_eff_part_index @ 0x0080_0da0 => extern "C" fn opu_eff_part_index(unit: u32) -> u32 {
        et::bridge::opu_eff_part_index(&mem(), unit)
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
    /// `op_rt_m_x(dst, src, angle)`.
    op_rt_m_x @ 0x007e_9580 => extern "C" fn op_rt_m_x(dst: u32, src: u32, angle: f32) {
        op::bridge::op_rt_m(&mut mem(), dst, src, angle, mat::Axis::X);
    }
    /// `op_rt_m_y(dst, src, angle)`.
    op_rt_m_y @ 0x007e_9670 => extern "C" fn op_rt_m_y(dst: u32, src: u32, angle: f32) {
        op::bridge::op_rt_m(&mut mem(), dst, src, angle, mat::Axis::Y);
    }
    /// `op_rt_m_z(dst, src, angle)`.
    op_rt_m_z @ 0x007e_9760 => extern "C" fn op_rt_m_z(dst: u32, src: u32, angle: f32) {
        op::bridge::op_rt_m(&mut mem(), dst, src, angle, mat::Axis::Z);
    }
    /// `op_rt_m_yxz(dst, src, angles, t)`.
    op_rt_m_yxz @ 0x007e_9a30 => extern "C" fn op_rt_m_yxz(dst: u32, src: u32, angles: u32, t: u32) {
        op::bridge::op_rt_m_yxz(&mut mem(), dst, src, angles, t);
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
    /// `pppOtFunc_Reg(list, node, a, b)`.
    pppOtFunc_Reg @ 0x0071_6be0 => extern "C" fn ppp_ot_func_reg(list: u32, node: u32, a: u32, b: u32) {
        part::bridge::ppp_ot_func_reg(&mut mem(), list, node, a, b);
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
}
