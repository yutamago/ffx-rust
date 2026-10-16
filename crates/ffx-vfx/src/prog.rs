//! The particle program table's set-ups and per-frame steps.
//!
//! `FFX.exe` lists its particle programs in a table of 0x28-byte records at
//! `0x00c3a508` (418 of them, some programs twice): the name at `+0`, the
//! per-frame step at `+8`, the draw function at `+0xc`, the set-up (`Con`)
//! at `+0x1c`, a second set-up (`Con2`) at `+0x20` and the destructor at
//! `+0x24`. The functions here are named after their records.
//!
//! Programs work as in [`crate::ppp`]: work area `i` of an object is at
//! `obj + offsets[i]` (fields from `+0xa0`), `offsets` being what
//! `ctrl + 0xc` points at; a step applies its key when the key's frame is
//! the object's and does nothing while `g_pppStopStatus` is set.
//!
//! - Most set-ups store constants ([`Init`]): zeros and ones, the emulated
//!   `r0` or `vf00` (four words), or `set_ptr32(0)`.
//! - The `Accele`/`Move` steps add the key to work area 1 and then, every
//!   frame, area 1 to area 0: `f32` (`pppAccele`, `pppSclAccele`, `pppMove`,
//!   `pppSclMove`, all one body), `i32` (`pppAngAccele`, `pppAngMove`) and
//!   `i16` (`pppColAccele`, `pppColMove`), four values; the `MoveLoop`
//!   ones three.
//! - `pppPoint` and `pppScale` add the key to area 0; their `Loop` forms
//!   first restart (area cleared, the key remembered at `+0xb0`) when no key
//!   is remembered, and their `Con2` take the remembered key back off.

/// One store of a set-up, at an offset in its work area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Init {
    /// A word (floats as their bits: 0.0, 1.0).
    W32(u32, u32),
    /// A half word.
    W16(u32, u16),
    /// A byte.
    W8(u32, u8),
    /// The four words of a global (the emulated `r0` or `vf00`).
    Copy4(u32, u32),
    /// `set_ptr32(0)`.
    Null(u32),
}

/// `f32` bits of 1.0.
pub const ONE: u32 = 0x3f80_0000;

/// The set-ups and steps where the game keeps their objects.
pub mod bridge {
    use ffx_bridge::{Global, Mem, Ptr};
    use ffx_system::stub;

    use super::Init::{self, Copy4, Null, W8, W16, W32};
    use super::ONE;
    use crate::ppp::bridge::{COLOR_CON_SHIFT, R0, STOP};
    use crate::x87;

    /// `g_Vu0_Vf00`: the emulated VU0 `vf00` (0, 0, 0, 1.0).
    pub const VF00: Global<[u32; 4]> = Global::new(0x00c0_a004);

    /// Work area `i` of `obj` (its offset from `ctrl`'s offsets).
    fn area(mem: &impl Mem, obj: u32, ctrl: u32, i: u32) -> u32 {
        let offsets = mem.read::<u32>(ctrl.wrapping_add(0xc));
        obj.wrapping_add(mem.read(offsets.wrapping_add(4 * i)))
    }

    fn applies(mem: &impl Mem, obj: u32, key: u32) -> bool {
        mem.read::<u32>(key) == mem.read::<u32>(obj.wrapping_add(0xc))
    }

    /// A set-up: `init` into work area `i`, in order.
    pub fn con(mem: &mut impl Mem, obj: u32, ctrl: u32, i: u32, init: &[Init]) {
        let w = area(mem, obj, ctrl, i);
        for &s in init {
            match s {
                W32(at, v) => mem.write(w.wrapping_add(at), v),
                W16(at, v) => mem.write(w.wrapping_add(at), v),
                W8(at, v) => mem.write(w.wrapping_add(at), v),
                Copy4(at, sym) => {
                    let v = Global::<[u32; 4]>::new(sym).get(mem);
                    mem.write(w.wrapping_add(at), v);
                }
                Null(at) => mem.write(w.wrapping_add(at), stub::ptr32(0)),
            }
        }
    }

    macro_rules! cons {
        ($( $(#[$doc:meta])* $f:ident: $i:literal, [$($init:expr),* $(,)?]; )*) => {
            $(
                $(#[$doc])*
                pub fn $f(mem: &mut impl Mem, obj: u32, ctrl: u32) {
                    con(mem, obj, ctrl, $i, &[$($init),*]);
                }
            )*
        };
    }

    const R0_: u32 = R0.sym;
    const VF00_: u32 = VF00.sym;

    cons! {
        /// `pppDrawShapeCon` (`0x00740eb0`).
        ppp_draw_shape_con: 0, [W32(0xa0, 0)];
        /// `pppDrawShapeXCon` (`0x00741450`).
        ppp_draw_shape_x_con: 0, [W32(0xa0, 0)];
        /// `pppNeiDrawShapePointLightCon` (`0x0074b9a0`).
        ppp_nei_draw_shape_point_light_con: 0, [W32(0xa0, 0)];
        /// `pppVertexApCon` (`0x00757b30`).
        ppp_vertex_ap_con: 0, [W32(0xa0, 0)];
        /// `pppVertexApDisPosCon` (`0x00757e90`).
        ppp_vertex_ap_dis_pos_con: 0, [W32(0xa0, 0)];
        /// `pppVertexApLcCon` (`0x00758250`).
        ppp_vertex_ap_lc_con: 0, [W32(0xa0, 0)];
        /// `pppVertexApAtCon` (`0x007584b0`).
        ppp_vertex_ap_at_con: 0, [W32(0xa0, 0)];
        /// `pppPointApCon` (`0x00757470`).
        ppp_point_ap_con: 1, [W8(0xa1, 0)];
        /// `pppPointRApCon` (`0x00757750`).
        ppp_point_r_ap_con: 1, [W8(0xa1, 0)];
        /// `pppKeHitBornCon` (`0x00759990`).
        ppp_ke_hit_born_con: 2, [W8(0xa0, 0)];
        /// `pppKeThHitBornCon` (`0x00759f10`).
        ppp_ke_th_hit_born_con: 1, [W8(0xa0, 0)];
        /// `pppKeBornRndCon` (`0x00758790`).
        ppp_ke_born_rnd_con: 0, [W16(0xa0, 0)];
        /// `pppKeBornRnd2Con` (`0x00758820`).
        ppp_ke_born_rnd2_con: 0, [W16(0xa0, 0)];
        /// `pppKeBornRnd3Con` (`0x007589d0`).
        ppp_ke_born_rnd3_con: 0, [W16(0xa0, 0)];
        /// `pppKeBornRnd4Con` (`0x00758dd0`).
        ppp_ke_born_rnd4_con: 0, [W16(0xa0, 0)];
        /// `pppDrawMdlRevCon` (`0x007403f0`): work area 5.
        ppp_draw_mdl_rev_con: 5, [W32(0xa0, 0)];
        /// `pppKeLnsLpSftCon` (`0x00759fb0`).
        ppp_ke_lns_lp_sft_con: 0, [W32(0xa0, 0), W32(0xa4, 0)];
        /// `pppKeLnsLpCon2` (`0x0075af60`).
        ppp_ke_lns_lp_con2: 0, [W32(0x13c, 0), W32(0x148, 0)];
        /// `pppKeLnsLpTCon2` (`0x0075b110`).
        ppp_ke_lns_lp_t_con2: 0, [W32(0x13c, 0), W32(0x148, 0)];
        /// `pppKeBornRnd5Con` (`0x00758f90`).
        ppp_ke_born_rnd5_con: 0, [W16(0xa0, 0), W8(0xa2, 0)];
        /// `pppKeBornRnd6Con` (`0x00759270`).
        ppp_ke_born_rnd6_con: 0, [W16(0xa0, 0), W8(0xa2, 0)];
        /// `pppKeHitChkPxBCon` (`0x00759bc0`).
        ppp_ke_hit_chk_px_b_con: 2, [W8(0xb0, 0), W32(0xac, ONE)];
        /// `pppKeShpTailCon` (`0x0074c540`).
        ppp_ke_shp_tail_con: 0, [W32(0xa2, 0), W16(0xa0, 0x1f)];
        /// `pppKeShpTailXCon` (`0x0074d540`).
        ppp_ke_shp_tail_x_con: 0, [W32(0xa2, 0), W16(0xa0, 0x1f)];
        /// `pppKeShpTail2Con` (`0x0074e540`).
        ppp_ke_shp_tail2_con: 0, [W32(0xa2, 0), W16(0xa0, 0x1f)];
        /// `pppKeShpTail2XCon` (`0x0074f580`).
        ppp_ke_shp_tail2_x_con: 0, [W32(0xa2, 0), W16(0xa0, 0x1f)];
        /// `pppKeShpTailLcCon` (`0x00754fc0`).
        ppp_ke_shp_tail_lc_con: 0, [W32(0xa2, 0), W16(0xa0, 0x1f), W8(0xa6, 0)];
        /// `pppDrawShapeRevCon` (`0x00741570`).
        ppp_draw_shape_rev_con: 0, [W32(0xa0, 0), W16(0xa4, 0)];
        /// `pppVtMimeCon2` (`0x0072edb0`).
        ppp_vt_mime_con2: 0, [W32(0xa8, 0), W32(0xa4, 0), W32(0xa0, 0)];
        /// `pppNeiPointLightCon` (`0x0075d7c0`).
        ppp_nei_point_light_con: 0, [W32(0xa8, 0), W32(0xa4, 0), W32(0xa0, 0)];
        /// `pppKeTkFadeCon` (`0x0072ee20`).
        ppp_ke_tk_fade_con: 0, [W32(0xa0, 0), W32(0xa4, 0), W32(0xa8, 0)];
        /// `pppKeMdlTfdCon` (`0x007497e0`).
        ppp_ke_mdl_tfd_con: 2, [W32(0xa0, 0), W32(0xa4, 0), W32(0xa8, 0)];
        /// `pppKeMdlTfd2Con` (`0x00749b10`).
        ppp_ke_mdl_tfd2_con: 2, [W32(0xa0, 0), W32(0xa4, 0), W32(0xa8, 0)];
        /// `pppKeMdlTfd3Con` (`0x00749e60`).
        ppp_ke_mdl_tfd3_con: 2, [W32(0xa0, 0), W32(0xa4, 0), W32(0xa8, 0)];
        /// `pppMoveLoopCon` (`0x0075c150`).
        ppp_move_loop_con: 1, [W32(0xa8, 0), W32(0xa4, 0), W32(0xa0, 0)];
        /// `pppMoveLoopCon2` (`0x0075c180`).
        ppp_move_loop_con2: 1, [W32(0xa8, 0), W32(0xa4, 0), W32(0xa0, 0)];
        /// `pppSclMoveLoopCon` (`0x0075c330`).
        ppp_scl_move_loop_con: 1, [W32(0xa8, 0), W32(0xa4, 0), W32(0xa0, 0)];
        /// `pppSclMoveLoopCon2` (`0x0075c360`).
        ppp_scl_move_loop_con2: 1, [W32(0xa8, 0), W32(0xa4, 0), W32(0xa0, 0)];
        /// `pppAngMoveLoopCon` (`0x0075c210`).
        ppp_ang_move_loop_con: 1, [W32(0xa8, 0), W32(0xa4, 0), W32(0xa0, 0)];
        /// `pppAngMoveLoopCon2` (`0x0075c250`).
        ppp_ang_move_loop_con2: 1, [W32(0xa8, 0), W32(0xa4, 0), W32(0xa0, 0)];
        /// `pppKeOfsPtCon` (`0x0075cdf0`).
        ppp_ke_ofs_pt_con: 0, [W32(0xa8, 0), W32(0xa4, 0), W32(0xa0, 0), W32(0xcc, 0)];
        /// `pppDrawMdlSeaCon2` (`0x0073ab40`).
        ppp_draw_mdl_sea_con2: 2, [W32(0x104, 0), W32(0x100, 0), W32(0xf8, 0), W32(0xf4, 0)];
        /// `pppDrawMdlInfCon2` (`0x0073b800`).
        ppp_draw_mdl_inf_con2: 2, [W32(0x16c, 0), W32(0x168, 0), W32(0x160, 0), W32(0x15c, 0)];
        /// `pppDrawMdlPSimCon2` (`0x0073cfe0`).
        ppp_draw_mdl_p_sim_con2: 2, [W32(0xb8, 0), W32(0xb4, 0), W32(0xac, 0), W32(0xa8, 0)];
        /// `pppDrawMdlLoopCon2` (`0x0073e270`).
        ppp_draw_mdl_loop_con2: 2, [W32(0xb8, 0), W32(0xb4, 0), W32(0xac, 0), W32(0xa8, 0)];
        /// `pppDrawMdlLoopZCon2` (`0x0073ea00`).
        ppp_draw_mdl_loop_z_con2: 2, [W32(0xb8, 0), W32(0xb4, 0), W32(0xac, 0), W32(0xa8, 0)];
        /// `pppDrawMdlLoopDisPosCon2` (`0x0073f1f0`).
        ppp_draw_mdl_loop_dis_pos_con2: 2, [W32(0xb8, 0), W32(0xb4, 0), W32(0xac, 0), W32(0xa8, 0)];
        /// `pppDrawMdlCameraLoopCon2` (`0x0073fa10`).
        ppp_draw_mdl_camera_loop_con2: 2, [W32(0xb8, 0), W32(0xb4, 0), W32(0xac, 0), W32(0xa8, 0)];
        /// `pppDrawMdlCameraCon2` (`0x0073d8f0`).
        ppp_draw_mdl_camera_con2: 2, [W32(0x124, 0), W32(0x120, 0), W32(0x118, 0), W32(0x114, 0)];
        /// `pppKeShpDttCon` (`0x00755010`).
        ppp_ke_shp_dtt_con: 0, [W32(0xa8, 0), W32(0xa0, 0), W32(0xa4, 0)];
        /// `pppEiWindFunCon` (`0x0075d590`).
        ppp_ei_wind_fun_con: 0, [W32(0xb0, 0), W32(0xb4, 0), W32(0xc4, 0), W32(0xc0, 0), W32(0xbc, 0)];
        /// `pppDrawMdlTsCon` (`0x007384f0`).
        ppp_draw_mdl_ts_con: 2, [W32(0xb4, 0), W32(0xb0, 0), W32(0xac, 0), W32(0xa8, 0), W32(0xa4, 0), W32(0xa0, 0)];
        /// `pppDrawMdlTs2Con` (`0x00739480`).
        ppp_draw_mdl_ts2_con: 2, [W32(0xb4, 0), W32(0xb0, 0), W32(0xac, 0), W32(0xa8, 0), W32(0xa4, 0), W32(0xa0, 0)];
        /// `pppDrawMdlTs3Con` (`0x0073aac0`).
        ppp_draw_mdl_ts3_con: 2, [W32(0xb4, 0), W32(0xb0, 0), W32(0xac, 0), W32(0xa8, 0), W32(0xa4, 0), W32(0xa0, 0)];
        /// `pppNeiDrawMdlTsPointLightCon` (`0x0074b500`).
        ppp_nei_draw_mdl_ts_point_light_con: 2, [W32(0xb4, 0), W32(0xb0, 0), W32(0xac, 0), W32(0xa8, 0), W32(0xa4, 0), W32(0xa0, 0)];
        /// `pppPointCon` (`0x0075c4c0`): `vf00`.
        ppp_point_con: 0, [Copy4(0xa0, VF00_)];
        /// `pppScaleCon` (`0x0075d050`): `vf00`.
        ppp_scale_con: 0, [Copy4(0xa0, VF00_)];
        /// `pppAngleCon` (`0x0075ce80`): `r0`.
        ppp_angle_con: 0, [Copy4(0xa0, R0_)];
        /// `pppAcceleCon` (`0x0075b810`).
        ppp_accele_con: 1, [Copy4(0xa0, VF00_)];
        /// `pppSclAcceleCon` (`0x0075b9d0`).
        ppp_scl_accele_con: 1, [Copy4(0xa0, VF00_)];
        /// `pppMoveCon` (`0x0075beb0`).
        ppp_move_con: 1, [Copy4(0xa0, VF00_)];
        /// `pppSclMoveCon` (`0x0075c070`).
        ppp_scl_move_con: 1, [Copy4(0xa0, VF00_)];
        /// `pppAngAcceleCon` (`0x0075b8c0`).
        ppp_ang_accele_con: 1, [Copy4(0xa0, R0_)];
        /// `pppAngMoveCon` (`0x0075bf60`).
        ppp_ang_move_con: 1, [Copy4(0xa0, R0_)];
        /// `pppKeBornPtCmplCon` (`0x00759620`).
        ppp_ke_born_pt_cmpl_con: 0, [Null(0xa0)];
        /// `pppPointLoopCon` (`0x0075c5a0`).
        ppp_point_loop_con: 0, [Null(0xb0)];
        /// `pppAngleLoopCon` (`0x0075cf40`).
        ppp_angle_loop_con: 0, [Null(0xb0)];
        /// `pppScaleLoopCon` (`0x0075d130`).
        ppp_scale_loop_con: 0, [Null(0xb0)];
        /// `pppDrawMdlCameraCon` (`0x0073d8c0`).
        ppp_draw_mdl_camera_con: 2, [Null(0x128)];
        /// `pppDrawMdlLoopCon` (`0x0073e240`).
        ppp_draw_mdl_loop_con: 2, [Null(0xa0)];
        /// `pppDrawMdlLoopZCon` (`0x0073e9d0`).
        ppp_draw_mdl_loop_z_con: 2, [Null(0xa0)];
        /// `pppDrawMdlLoopDisPosCon` (`0x0073f1c0`).
        ppp_draw_mdl_loop_dis_pos_con: 2, [Null(0xa0)];
        /// `pppDrawMdlCameraLoopCon` (`0x0073f9e0`).
        ppp_draw_mdl_camera_loop_con: 2, [Null(0xa0)];
        /// `pppDrawSpriteCon` (`0x007566a0`).
        ppp_draw_sprite_con: 1, [Null(0xa0)];
        /// `pppDrawFilterCon` (`0x00757210`).
        ppp_draw_filter_con: 1, [Null(0xa0)];
        /// `pppKeGrvTgtCon` (`0x00759800`).
        ppp_ke_grv_tgt_con: 1, [Null(0xa8)];
        /// `pppKeThTp2Con` (`0x00736e30`).
        ppp_ke_th_tp2_con: 0, [Null(0xbc), W32(0xac, ONE)];
        /// `pppKeThTpCon` (`0x00736e60`).
        ppp_ke_th_tp_con: 0, [Null(0xbc), W32(0xac, 0)];
        /// `pppDrawMdlPSimCon` (`0x0073cfb0`).
        ppp_draw_mdl_p_sim_con: 2, [Null(0xa0), Null(0xbc)];
        /// `pppDrawRainCon` (`0x00756240`).
        ppp_draw_rain_con: 1, [Null(0xa0), Null(0xa4)];
        /// `pppDrawMdlSeaCon` (`0x0073ab00`).
        ppp_draw_mdl_sea_con: 2, [Null(0x108), Null(0x10c), Null(0x110)];
    }

    /// `pppColAcceleCon` (`0x0075bac0`) and `pppColMoveCon` (`0x0075c410`):
    /// work area 1 (shifted by `g_pppColorConShift`, which they clear) from
    /// the low eight bytes of `r0`, as `pppColorCon` does for area 0.
    pub fn ppp_col_accele_con(mem: &mut impl Mem, obj: u32, ctrl: u32) {
        let shift = COLOR_CON_SHIFT.get(mem);
        let at = area(mem, obj, ctrl, 1)
            .wrapping_add(shift)
            .wrapping_add(0xa0);
        let r0 = R0.get(mem);
        mem.write(at, r0[0]);
        mem.write(at.wrapping_add(4), r0[1]);
        COLOR_CON_SHIFT.set(mem, 0);
    }

    fn f(mem: &impl Mem, at: u32) -> f32 {
        f32::from_bits(mem.read(at))
    }

    fn set_f(mem: &mut impl Mem, at: u32, v: f32) {
        mem.write(at, v.to_bits());
    }

    /// The `f32` `Accele`/`Move` step over `n` values: area 1 += key
    /// (`+0x10`), then area 0 += area 1.
    fn accumulate_f32(mem: &mut impl Mem, obj: u32, key: u32, ctrl: u32, n: u32) {
        if STOP.get(mem) != 0 {
            return;
        }
        let w0 = area(mem, obj, ctrl, 0).wrapping_add(0xa0);
        let w1 = area(mem, obj, ctrl, 1).wrapping_add(0xa0);
        if applies(mem, obj, key) {
            for i in 0..n {
                let v = x87::add(f(mem, key + 0x10 + 4 * i), f(mem, w1 + 4 * i));
                set_f(mem, w1 + 4 * i, v);
            }
        }
        for i in 0..n {
            let v = x87::add(f(mem, w1 + 4 * i), f(mem, w0 + 4 * i));
            set_f(mem, w0 + 4 * i, v);
        }
    }

    /// The `i32` form.
    fn accumulate_i32(mem: &mut impl Mem, obj: u32, key: u32, ctrl: u32, n: u32) {
        if STOP.get(mem) != 0 {
            return;
        }
        let w0 = area(mem, obj, ctrl, 0).wrapping_add(0xa0);
        let w1 = area(mem, obj, ctrl, 1).wrapping_add(0xa0);
        if applies(mem, obj, key) {
            for i in 0..n {
                let p = Ptr::<u32>::new(w1 + 4 * i);
                let v = p.get(mem).wrapping_add(mem.read(key + 0x10 + 4 * i));
                p.set(mem, v);
            }
        }
        for i in 0..n {
            let p = Ptr::<u32>::new(w0 + 4 * i);
            let v = p.get(mem).wrapping_add(mem.read(w1 + 4 * i));
            p.set(mem, v);
        }
    }

    /// The `i16` form (the key's values at `+8`).
    fn accumulate_i16(mem: &mut impl Mem, obj: u32, key: u32, ctrl: u32) {
        if STOP.get(mem) != 0 {
            return;
        }
        let w0 = area(mem, obj, ctrl, 0).wrapping_add(0xa0);
        let w1 = area(mem, obj, ctrl, 1).wrapping_add(0xa0);
        if applies(mem, obj, key) {
            for i in 0..4 {
                let p = Ptr::<u16>::new(w1 + 2 * i);
                let v = p.get(mem).wrapping_add(mem.read(key + 8 + 2 * i));
                p.set(mem, v);
            }
        }
        for i in 0..4 {
            let p = Ptr::<u16>::new(w0 + 2 * i);
            let v = p.get(mem).wrapping_add(mem.read(w1 + 2 * i));
            p.set(mem, v);
        }
    }

    /// `pppAccele` (`0x0075b740`), `pppSclAccele` (`0x0075b900`), `pppMove`
    /// (`0x0075bde0`), `pppSclMove` (`0x0075bfa0`): one body.
    pub fn ppp_accele(mem: &mut impl Mem, obj: u32, key: u32, ctrl: u32) {
        accumulate_f32(mem, obj, key, ctrl, 4);
    }

    /// `pppAngAccele` (`0x0075b850`) and `pppAngMove` (`0x0075bef0`).
    pub fn ppp_ang_accele(mem: &mut impl Mem, obj: u32, key: u32, ctrl: u32) {
        accumulate_i32(mem, obj, key, ctrl, 4);
    }

    /// `pppColAccele` (`0x0075ba40`) and `pppColMove` (`0x0075c390`).
    pub fn ppp_col_accele(mem: &mut impl Mem, obj: u32, key: u32, ctrl: u32) {
        accumulate_i16(mem, obj, key, ctrl);
    }

    /// `pppMoveLoop` (`0x0075c0b0`) and `pppSclMoveLoop` (`0x0075c290`).
    pub fn ppp_move_loop(mem: &mut impl Mem, obj: u32, key: u32, ctrl: u32) {
        accumulate_f32(mem, obj, key, ctrl, 3);
    }

    /// `pppAngMoveLoop` (`0x0075c1b0`).
    pub fn ppp_ang_move_loop(mem: &mut impl Mem, obj: u32, key: u32, ctrl: u32) {
        accumulate_i32(mem, obj, key, ctrl, 3);
    }

    /// `pppPoint` (`0x0075c450`) and `pppScale` (`0x0075cfe0`): area 0 +=
    /// the key's four floats.
    pub fn ppp_point(mem: &mut impl Mem, obj: u32, key: u32, ctrl: u32) {
        if STOP.get(mem) != 0 || !applies(mem, obj, key) {
            return;
        }
        let w0 = area(mem, obj, ctrl, 0).wrapping_add(0xa0);
        for i in 0..4 {
            let v = x87::add(f(mem, key + 0x10 + 4 * i), f(mem, w0 + 4 * i));
            set_f(mem, w0 + 4 * i, v);
        }
    }

    /// `pppPointLoop` (`0x0075c500`) and `pppScaleLoop` (`0x0075d090`):
    /// with no key remembered, clears area 0 and remembers this one; then
    /// adds the key's three floats.
    pub fn ppp_point_loop(mem: &mut impl Mem, obj: u32, key: u32, ctrl: u32) {
        if STOP.get(mem) != 0 {
            return;
        }
        let w0 = area(mem, obj, ctrl, 0).wrapping_add(0xa0);
        if stub::is_ptr32_null(mem.read(w0 + 0x10)) != 0 {
            for at in [8, 4, 0] {
                mem.write(w0 + at, 0u32);
            }
            mem.write(w0 + 0x10, stub::ptr32(key));
        }
        if applies(mem, obj, key) {
            for i in 0..3 {
                let v = x87::add(f(mem, key + 0x10 + 4 * i), f(mem, w0 + 4 * i));
                set_f(mem, w0 + 4 * i, v);
            }
        }
    }

    /// `pppAngleLoop` (`0x0075cec0`): the `i32` form.
    pub fn ppp_angle_loop(mem: &mut impl Mem, obj: u32, key: u32, ctrl: u32) {
        let w0 = area(mem, obj, ctrl, 0).wrapping_add(0xa0);
        if STOP.get(mem) != 0 {
            return;
        }
        if stub::is_ptr32_null(mem.read(w0 + 0x10)) != 0 {
            for at in [8, 4, 0] {
                mem.write(w0 + at, 0u32);
            }
            mem.write(w0 + 0x10, stub::ptr32(key));
        }
        if applies(mem, obj, key) {
            for i in 0..3 {
                let p = Ptr::<u32>::new(w0 + 4 * i);
                let v = p.get(mem).wrapping_add(mem.read(key + 0x10 + 4 * i));
                p.set(mem, v);
            }
        }
    }

    /// `pppPointLoopCon2` (`0x0075c5d0`) and `pppScaleLoopCon2`
    /// (`0x0075d160`): take the remembered key's three floats back off.
    pub fn ppp_point_loop_con2(mem: &mut impl Mem, obj: u32, ctrl: u32) {
        let w0 = area(mem, obj, ctrl, 0).wrapping_add(0xa0);
        let k = stub::ptr32(mem.read(w0 + 0x10));
        for i in 0..3 {
            let v = x87::sub(f(mem, w0 + 4 * i), f(mem, k.wrapping_add(0x10 + 4 * i)));
            set_f(mem, w0 + 4 * i, v);
        }
    }

    /// `pppAngleLoopCon2` (`0x0075cf70`): the `i32` form.
    pub fn ppp_angle_loop_con2(mem: &mut impl Mem, obj: u32, ctrl: u32) {
        let w0 = area(mem, obj, ctrl, 0).wrapping_add(0xa0);
        let k = stub::ptr32(mem.read(w0 + 0x10));
        for i in 0..3 {
            let p = Ptr::<u32>::new(w0 + 4 * i);
            let v = p
                .get(mem)
                .wrapping_sub(mem.read(k.wrapping_add(0x10 + 4 * i)));
            p.set(mem, v);
        }
    }

    /// `pppKeGrvTgt(obj, key, ctrl)` (`0x007597b0`): area 1 takes the key's
    /// two floats at `+4` and `set_ptr32` of area 0's fields (no stop
    /// check).
    pub fn ppp_ke_grv_tgt(mem: &mut impl Mem, obj: u32, key: u32, ctrl: u32) {
        if !applies(mem, obj, key) {
            return;
        }
        let w1 = area(mem, obj, ctrl, 1).wrapping_add(0xa0);
        let a = x87::load(mem.read(key + 4));
        mem.write(w1, a);
        let b = x87::load(mem.read(key + 8));
        mem.write(w1 + 4, b);
        let w0 = area(mem, obj, ctrl, 0).wrapping_add(0xa0);
        mem.write(w1 + 8, stub::ptr32(w0));
    }
}
