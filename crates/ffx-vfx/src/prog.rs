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
    /// `KeThResHd_Init(at, count, size)` (the map module's).
    ThRes(u32, u8, u8),
    /// `KeThCp_Init(at)`.
    ThCp(u32),
    /// `KeLnsFls_Init(at)` / `KeLnsCrn_Init(at)` (one body).
    Lns(u32),
    /// `KeLnsClm_Init(at)`.
    LnsClm(u32),
}

/// `f32` bits of 1.0.
pub const ONE: u32 = 0x3f80_0000;

/// The set-ups and steps where the game keeps their objects.
pub mod bridge {
    use ffx_bridge::{Global, Mem, Ptr};
    use ffx_system::stub;

    use super::Init::{self, Copy4, Lns, LnsClm, Null, ThCp, ThRes, W8, W16, W32};
    use super::ONE;
    use crate::part::bridge as part;
    use crate::ppp::bridge::{COLOR_CON_SHIFT, R0, STOP};
    use crate::x87;
    use ffx_field::ke::bridge as ke;

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
                ThRes(at, count, size) => {
                    ke::ke_th_res_hd_init(mem, w.wrapping_add(at), count, size);
                }
                ThCp(at) => ke::ke_th_cp_init(mem, w.wrapping_add(at)),
                Lns(at) => ke::ke_lns_init(mem, w.wrapping_add(at)),
                LnsClm(at) => ke::ke_lns_clm_init(mem, w.wrapping_add(at)),
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
        /// `pppKeThRes128Con` (`0x00736690`).
        ppp_ke_th_res128_con: 0, [ThRes(0xa0, 1, 128)];
        /// `pppKeThRes128x4Con` (`0x007366c0`).
        ppp_ke_th_res128x4_con: 0, [ThRes(0xa0, 4, 128)];
        /// `pppKeThRes128x8Con` (`0x007366f0`).
        ppp_ke_th_res128x8_con: 0, [ThRes(0xa0, 8, 128)];
        /// `pppKeThRes16Con` (`0x00736720`).
        ppp_ke_th_res16_con: 0, [ThRes(0xa0, 1, 16)];
        /// `pppKeThRes16x16Con` (`0x00736750`).
        ppp_ke_th_res16x16_con: 0, [ThRes(0xa0, 16, 16)];
        /// `pppKeThRes16x24Con` (`0x00736780`).
        ppp_ke_th_res16x24_con: 0, [ThRes(0xa0, 24, 16)];
        /// `pppKeThRes16x4Con` (`0x007367b0`).
        ppp_ke_th_res16x4_con: 0, [ThRes(0xa0, 4, 16)];
        /// `pppKeThRes16x64Con` (`0x007367e0`).
        ppp_ke_th_res16x64_con: 0, [ThRes(0xa0, 64, 16)];
        /// `pppKeThRes16x8Con` (`0x00736810`).
        ppp_ke_th_res16x8_con: 0, [ThRes(0xa0, 8, 16)];
        /// `pppKeThRes24Con` (`0x00736840`).
        ppp_ke_th_res24_con: 0, [ThRes(0xa0, 1, 24)];
        /// `pppKeThRes24x16Con` (`0x00736870`).
        ppp_ke_th_res24x16_con: 0, [ThRes(0xa0, 16, 24)];
        /// `pppKeThRes24x4Con` (`0x007368a0`).
        ppp_ke_th_res24x4_con: 0, [ThRes(0xa0, 4, 24)];
        /// `pppKeThRes24x8Con` (`0x007368d0`).
        ppp_ke_th_res24x8_con: 0, [ThRes(0xa0, 8, 24)];
        /// `pppKeThRes255Con` (`0x00736900`).
        ppp_ke_th_res255_con: 0, [ThRes(0xa0, 1, 255)];
        /// `pppKeThRes255x4Con` (`0x00736930`).
        ppp_ke_th_res255x4_con: 0, [ThRes(0xa0, 4, 255)];
        /// `pppKeThRes32Con` (`0x00736960`).
        ppp_ke_th_res32_con: 0, [ThRes(0xa0, 1, 32)];
        /// `pppKeThRes32x16Con` (`0x00736990`).
        ppp_ke_th_res32x16_con: 0, [ThRes(0xa0, 16, 32)];
        /// `pppKeThRes32x24Con` (`0x007369c0`).
        ppp_ke_th_res32x24_con: 0, [ThRes(0xa0, 24, 32)];
        /// `pppKeThRes32x32Con` (`0x007369f0`).
        ppp_ke_th_res32x32_con: 0, [ThRes(0xa0, 32, 32)];
        /// `pppKeThRes32x4Con` (`0x00736a20`).
        ppp_ke_th_res32x4_con: 0, [ThRes(0xa0, 4, 32)];
        /// `pppKeThRes32x8Con` (`0x00736a50`).
        ppp_ke_th_res32x8_con: 0, [ThRes(0xa0, 8, 32)];
        /// `pppKeThRes40Con` (`0x00736a80`).
        ppp_ke_th_res40_con: 0, [ThRes(0xa0, 1, 40)];
        /// `pppKeThRes40x16Con` (`0x00736ab0`).
        ppp_ke_th_res40x16_con: 0, [ThRes(0xa0, 16, 40)];
        /// `pppKeThRes40x4Con` (`0x00736ae0`).
        ppp_ke_th_res40x4_con: 0, [ThRes(0xa0, 4, 40)];
        /// `pppKeThRes40x8Con` (`0x00736b10`).
        ppp_ke_th_res40x8_con: 0, [ThRes(0xa0, 8, 40)];
        /// `pppKeThRes48Con` (`0x00736b40`).
        ppp_ke_th_res48_con: 0, [ThRes(0xa0, 1, 48)];
        /// `pppKeThRes48x16Con` (`0x00736b70`).
        ppp_ke_th_res48x16_con: 0, [ThRes(0xa0, 16, 48)];
        /// `pppKeThRes48x4Con` (`0x00736ba0`).
        ppp_ke_th_res48x4_con: 0, [ThRes(0xa0, 4, 48)];
        /// `pppKeThRes48x8Con` (`0x00736bd0`).
        ppp_ke_th_res48x8_con: 0, [ThRes(0xa0, 8, 48)];
        /// `pppKeThRes64Con` (`0x00736c00`).
        ppp_ke_th_res64_con: 0, [ThRes(0xa0, 1, 64)];
        /// `pppKeThRes64x16Con` (`0x00736c30`).
        ppp_ke_th_res64x16_con: 0, [ThRes(0xa0, 16, 64)];
        /// `pppKeThRes64x4Con` (`0x00736c60`).
        ppp_ke_th_res64x4_con: 0, [ThRes(0xa0, 4, 64)];
        /// `pppKeThRes64x8Con` (`0x00736c90`).
        ppp_ke_th_res64x8_con: 0, [ThRes(0xa0, 8, 64)];
        /// `pppKeThRes8Con` (`0x00736cc0`).
        ppp_ke_th_res8_con: 0, [ThRes(0xa0, 1, 8)];
        /// `pppKeThRes8x128Con` (`0x00736cf0`).
        ppp_ke_th_res8x128_con: 0, [ThRes(0xa0, 128, 8)];
        /// `pppKeThRes8x4Con` (`0x00736d20`).
        ppp_ke_th_res8x4_con: 0, [ThRes(0xa0, 4, 8)];
        /// `pppKeThResCon` (`0x00736d50`).
        ppp_ke_th_res_con: 0, [ThRes(0xa0, 1, 3)];
        /// `pppKeLnsArndCon` (`0x00759fd0`).
        ppp_ke_lns_arnd_con: 0, [Lns(0xa0)];
        /// `pppKeLnsCrnCon` (`0x0075a740`).
        ppp_ke_lns_crn_con: 0, [Lns(0xa0)];
        /// `pppKeLnsClmCon` (`0x0075a370`).
        ppp_ke_lns_clm_con: 0, [LnsClm(0xa0), W8(0xa8, 4)];
        /// `pppKeThCon` (`0x00736460`).
        ppp_ke_th_con: 0, [ThCp(0xa0), W32(0x10c, 0), W32(0x110, 0), W32(0x114, 0), W32(0x118, 0), W32(0x11c, 0), W32(0x120, 0), W32(0x124, 0), W32(0x128, 0), W32(0x12c, 0), W32(0xbc, 0), W32(0xd4, 0), W32(0xdc, 0)];
        /// `pppKeMdlTfdUvCon` (`0x007498f0`).
        ppp_ke_mdl_tfd_uv_con: 2, [W32(0xc4, 0), W32(0xc0, 0), W32(0xbc, 0), W32(0xb8, 0), W32(0xb4, 0), W32(0xb0, 0), W32(0xa8, 0), W32(0xa4, 0), W32(0xa0, 0)];
        /// `pppKeMdlTfdUv2Con` (`0x00749c30`).
        ppp_ke_mdl_tfd_uv2_con: 2, [W32(0xc4, 0), W32(0xc0, 0), W32(0xbc, 0), W32(0xb8, 0), W32(0xb4, 0), W32(0xb0, 0), W32(0xa8, 0), W32(0xa4, 0), W32(0xa0, 0)];
        /// `pppKeMdlTfdUv3Con` (`0x00749f90`).
        ppp_ke_mdl_tfd_uv3_con: 2, [W32(0xc4, 0), W32(0xc0, 0), W32(0xbc, 0), W32(0xb8, 0), W32(0xb4, 0), W32(0xb0, 0), W32(0xa8, 0), W32(0xa4, 0), W32(0xa0, 0)];
        /// `pppKeShpTail3Con2` (`0x007505a0`).
        ppp_ke_shp_tail3_con2: 0, [W8(0x298, 0), W16(0x294, 0), W32(0x290, 0), W32(0xa0, 0), W32(0xa4, 0), W32(0xa8, 0), W32(0xac, 0), W32(0xb0, 0), W32(0xb4, 0), W32(0xb8, 0), W32(0xbc, 0), W32(0xc0, 0), W32(0xc4, 0), W32(0xc8, 0), W32(0xcc, 0)];
        /// `pppKeShpTail3XCon2` (`0x00751f40`).
        ppp_ke_shp_tail3_x_con2: 0, [W8(0x298, 0), W16(0x294, 0), W32(0x290, 0), W32(0xa0, 0), W32(0xa4, 0), W32(0xa8, 0), W32(0xac, 0), W32(0xb0, 0), W32(0xb4, 0), W32(0xb8, 0), W32(0xbc, 0), W32(0xc0, 0), W32(0xc4, 0), W32(0xc8, 0), W32(0xcc, 0)];
        /// `pppKeMdlDttCon` (`0x0072e730`).
        ppp_ke_mdl_dtt_con: 0, [W32(0xa8, 0), W32(0xa0, 0), W32(0xa4, 0)];
        /// `pppKeMdlDttCon2` (`0x0072e760`).
        ppp_ke_mdl_dtt_con2: 0, [W32(0xa8, 0), W32(0xa0, 0), W32(0xa4, 0)];
        /// `pppKeShpTailPhtCon2` (`0x00754260`).
        ppp_ke_shp_tail_pht_con2: 0, [W32(0x288, 0), W8(0x294, 0), W32(0x284, 0), W16(0x290, 0), W32(0x280, 0), W32(0x28c, 0), W32(0xa0, 0), W32(0xa4, 0), W32(0xa8, 0), W32(0xac, 0), W32(0xb0, 0), W32(0xb4, 0), W32(0xb8, 0), W32(0xbc, 0), W32(0xc0, 0), W32(0xc4, 0), W32(0xc8, 0), W32(0xcc, 0)];
        /// `pppDrawMdlInfCon` (`0x0073b7b0`).
        ppp_draw_mdl_inf_con: 2, [Null(0x170), Null(0x174), Null(0x178), Null(0x17c)];
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

    /// `g_pppCurrentPart`: the part `pppRunPart`/`pppStartPart` are
    /// running; `+0x3c` points at its 16-byte object records (the object at
    /// `+4`).
    pub const CURRENT_PART: Global<u32> = Global::new(0x0230_ffe4);

    /// Object `n` of the current part (`get_ptr32_func` of its record's
    /// word at `+4`).
    fn part_object(mem: &impl Mem, n: u32) -> u32 {
        let records = mem.read::<u32>(CURRENT_PART.get(mem).wrapping_add(0x3c));
        stub::ptr32(mem.read(records.wrapping_add(4).wrapping_add(n.wrapping_mul(16))))
    }

    /// A key's target: `g_pppRandTarget` for -1, else field `at` of part
    /// object `n`.
    fn part_target(mem: &impl Mem, n: u32, at: u32) -> u32 {
        if n == u32::MAX {
            crate::rand::bridge::TARGET.ptr(mem).addr
        } else {
            part_object(mem, n).wrapping_add(at).wrapping_add(0xa0)
        }
    }

    /// `pppDrawSprite(obj, key, ctrl)` (`0x007566e0`): remembers the key in
    /// area 1 when it has none.
    pub fn ppp_draw_sprite(mem: &mut impl Mem, obj: u32, key: u32, ctrl: u32) {
        let w1 = area(mem, obj, ctrl, 1).wrapping_add(0xa0);
        if stub::is_ptr32_null(mem.read(w1)) != 0 {
            mem.write(w1, stub::ptr32(key));
        }
    }

    /// `pppKeHitBall(obj, key, ctrl)` (`0x00759830`): area 1 takes a
    /// pointer to area 0 and the key's float at `+4`.
    pub fn ppp_ke_hit_ball(mem: &mut impl Mem, obj: u32, key: u32, ctrl: u32) {
        if !applies(mem, obj, key) {
            return;
        }
        let w1 = area(mem, obj, ctrl, 1);
        let w0 = area(mem, obj, ctrl, 0).wrapping_add(0xa0);
        mem.write(w1.wrapping_add(0xa4), stub::ptr32(w0));
        let v = x87::load(mem.read(key + 4));
        mem.write(w1.wrapping_add(0xa0), v);
    }

    /// `pppNeiPointLight(obj, key, ctrl)` (`0x0075d700`): with its key,
    /// pointers to areas 1 and 2 and the key's three floats added; then the
    /// running sums `w[1] += w[0]`, `w[2] += w[1]`.
    pub fn ppp_nei_point_light(mem: &mut impl Mem, obj: u32, key: u32, ctrl: u32) {
        if STOP.get(mem) != 0 {
            return;
        }
        let w = area(mem, obj, ctrl, 0).wrapping_add(0xa0);
        if applies(mem, obj, key) {
            let a1 = area(mem, obj, ctrl, 1).wrapping_add(0xa0);
            mem.write(w + 0xc, stub::ptr32(a1));
            let a2 = area(mem, obj, ctrl, 2).wrapping_add(0xa0);
            mem.write(w + 0x10, stub::ptr32(a2));
            for i in 0..3 {
                let v = x87::add(f(mem, key + 4 + 4 * i), f(mem, w + 4 * i));
                set_f(mem, w + 4 * i, v);
            }
        }
        let s = x87::add(f(mem, w), f(mem, w + 4));
        set_f(mem, w + 4, s);
        let t = x87::add(s, f(mem, w + 8));
        set_f(mem, w + 8, t);
    }

    /// `pppPObjPoint(obj, key, ctrl)` (`0x0075d280`): with its key, points
    /// area 0's `+0x10` at a part object's field (key `+4` the object, -1
    /// for `g_pppRandTarget`; `+8` the field); then copies the three floats
    /// there into area 0.
    pub fn ppp_p_obj_point(mem: &mut impl Mem, obj: u32, key: u32, ctrl: u32) {
        if STOP.get(mem) != 0 {
            return;
        }
        let w = area(mem, obj, ctrl, 0).wrapping_add(0xa0);
        if applies(mem, obj, key) {
            let t = part_target(mem, mem.read(key + 4), mem.read(key + 8));
            mem.write(w + 0x10, stub::ptr32(t));
        }
        for i in 0..3 {
            let src = stub::ptr32(mem.read(w + 0x10));
            let v = x87::load(mem.read(src.wrapping_add(4 * i)));
            mem.write(w + 4 * i, v);
        }
    }

    /// `pppKeThTp(obj, key, ctrl)` (`0x00736d80`): with its key, three
    /// floats into area 0 (`+0x10` from key `+8`, `+0x18` from `+4`, `+0x14`
    /// from `+0xc`) and the target (key `+0x10` the part object, `+0x14` the
    /// field; none for `g_pppRandTarget`) at `+0x1c`; then, unless stopped,
    /// the object's position (`+0x40`) into area 0 (`KeVu0CopyVecXYZ`).
    pub fn ppp_ke_th_tp(mem: &mut impl Mem, obj: u32, key: u32, ctrl: u32) {
        let w = area(mem, obj, ctrl, 0).wrapping_add(0xa0);
        if applies(mem, obj, key) {
            for (to, from) in [(0x10, 8), (0x18, 4), (0x14, 0xc)] {
                let v = x87::load(mem.read(key + from));
                mem.write(w + to, v);
            }
            let t = stub::ptr32(part_target(mem, mem.read(key + 0x10), mem.read(key + 0x14)));
            mem.write(w + 0x1c, t);
            if stub::ptr32(t) == crate::rand::bridge::TARGET.ptr(mem).addr {
                mem.write(w + 0x1c, stub::ptr32(0));
            }
        }
        if STOP.get(mem) == 0 {
            ffx_field::vu0::bridge::ke_vu0_copy_vec_xyz(mem, w, obj.wrapping_add(0x40));
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

    /// A set-up that takes a CRT `rand()` between its stores: `before`,
    /// then the low half of `rand()` at `rand_at`, then `after`.
    #[allow(clippy::too_many_arguments)]
    fn con_rand<M: Mem>(
        mem: &mut M,
        obj: u32,
        ctrl: u32,
        before: &[Init],
        rand_at: u32,
        after: &[Init],
        rand: &mut impl FnMut(&mut M) -> u32,
    ) {
        con(mem, obj, ctrl, 0, before);
        let r = rand(mem);
        let w = area(mem, obj, ctrl, 0);
        #[allow(clippy::cast_possible_truncation)] // the low half
        mem.write(w.wrapping_add(rand_at), r as u16);
        con(mem, obj, ctrl, 0, after);
    }

    const TAIL3_ZEROS: [Init; 12] = [
        W32(0xa0, 0),
        W32(0xa4, 0),
        W32(0xa8, 0),
        W32(0xac, 0),
        W32(0xb0, 0),
        W32(0xb4, 0),
        W32(0xb8, 0),
        W32(0xbc, 0),
        W32(0xc0, 0),
        W32(0xc4, 0),
        W32(0xc8, 0),
        W32(0xcc, 0),
    ];

    /// `pppKeShpTail3Con` (`0x00750510`) and `pppKeShpTail3XCon`
    /// (`0x00751eb0`): the ring's index cleared, a random seed at `+0x296`,
    /// the motion cleared.
    pub fn ppp_ke_shp_tail3_con<M: Mem>(
        mem: &mut M,
        obj: u32,
        ctrl: u32,
        rand: &mut impl FnMut(&mut M) -> u32,
    ) {
        let before = [W8(0x298, 0), W16(0x294, 0), W32(0x290, 0)];
        con_rand(mem, obj, ctrl, &before, 0x296, &TAIL3_ZEROS, rand);
    }

    /// `pppKeShpTailPhtCon` (`0x00754180`): as the `Tail3` set-up, the
    /// ring's index at `+0x294`, the seed at `+0x292`, and the float motion
    /// at `+0x280` cleared too.
    pub fn ppp_ke_shp_tail_pht_con<M: Mem>(
        mem: &mut M,
        obj: u32,
        ctrl: u32,
        rand: &mut impl FnMut(&mut M) -> u32,
    ) {
        let before = [W8(0x294, 0), W16(0x290, 0), W32(0x28c, 0)];
        let after = [
            W32(0x288, 0),
            W32(0x284, 0),
            W32(0xa0, 0),
            W32(0x280, 0),
            W32(0xa4, 0),
            W32(0xa8, 0),
            W32(0xac, 0),
            W32(0xb0, 0),
            W32(0xb4, 0),
            W32(0xb8, 0),
            W32(0xbc, 0),
            W32(0xc0, 0),
            W32(0xc4, 0),
            W32(0xc8, 0),
            W32(0xcc, 0),
        ];
        con_rand(mem, obj, ctrl, &before, 0x292, &after, rand);
    }

    /// The object's position (`+0x40`, `w` = 1.0) into `count` ring slots
    /// from `first`, 16 bytes apart (at frame 0).
    fn fill_ring(mem: &mut impl Mem, obj: u32, first: u32, count: u32) {
        let mut v: [u32; 4] = mem.read(obj.wrapping_add(0x40));
        v[3] = ONE;
        for k in 0..count {
            mem.write(first.wrapping_add(16 * k), v);
        }
    }

    /// `pppKeShpTail` (`0x0074c490`), `pppKeShpTailX` (`0x0074d490`),
    /// `pppKeShpTail2` (`0x0074e490`) and `pppKeShpTail2X` (`0x0074f4d0`)
    /// `(obj, key, ctrl)`: area 0 is a ring of positions (`+0` the length,
    /// `+1` the index, the slots from `+0x10`); the object's position goes
    /// into the next slot (filled everywhere at frame 0).
    pub fn ppp_ke_shp_tail(mem: &mut impl Mem, obj: u32, ctrl: u32) {
        if STOP.get(mem) != 0 {
            return;
        }
        let p = area(mem, obj, ctrl, 0).wrapping_add(0xa0);
        if mem.read::<u32>(obj.wrapping_add(0xc)) == 0 {
            let n = u32::from(mem.read::<u8>(p));
            fill_ring(mem, obj, p.wrapping_add(0x10), n);
        }
        if mem.read::<u8>(p + 1) == 0 {
            let n: u8 = mem.read(p);
            mem.write(p + 1, n);
        }
        let i = mem.read::<u8>(p + 1).wrapping_sub(1);
        mem.write(p + 1, i);
        let slot = p.wrapping_add((u32::from(i) + 1) * 16);
        ffx_field::vu0::bridge::ke_vu0_copy_vec_xyz(mem, slot, obj.wrapping_add(0x40));
    }

    /// The forms of the `pppKeShpTail3` steps.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Tail3 {
        /// `pppKeShpTail3`: position += velocity, then velocity +=
        /// acceleration.
        Plain,
        /// `pppKeShpTail3X`: velocity first.
        X,
        /// `pppKeShpTailPht`: as `X` with 27 slots, and a float motion
        /// (`+0x1e0` position, `+0x1e4` velocity, `+0x1e8` acceleration).
        Pht,
    }

    /// `pppKeShpTail3` (`0x007502c0`), `pppKeShpTail3X` (`0x00751ca0`) and
    /// `pppKeShpTailPht` (`0x00753f10`) `(obj, key, ctrl)`: a ring of 28
    /// (27) positions from `+0x30` (index byte at `+0x1f8`, `+0x1f4`), and
    /// two `i16` motions (position, velocity and acceleration: halves 0..4,
    /// 8..12, 12..16 and 4..8, 16..20, 20..24) the key adds to.
    #[allow(clippy::many_single_char_names)] // p, h, i, x, v, a
    pub fn ppp_ke_shp_tail3(mem: &mut impl Mem, obj: u32, key: u32, ctrl: u32, form: Tail3) {
        if STOP.get(mem) != 0 {
            return;
        }
        let p = area(mem, obj, ctrl, 0).wrapping_add(0xa0);
        let (slots, index_at) = if form == Tail3::Pht {
            (0x1b, 0x1f4)
        } else {
            (0x1c, 0x1f8)
        };
        if mem.read::<u32>(obj.wrapping_add(0xc)) == 0 {
            fill_ring(mem, obj, p.wrapping_add(0x30), slots);
        }
        let index = p.wrapping_add(index_at);
        if mem.read::<u8>(index) == 0 {
            #[allow(clippy::cast_possible_truncation)] // 27 or 28
            mem.write(index, slots as u8);
        }
        let i = mem.read::<u8>(index).wrapping_sub(1);
        mem.write(index, i);
        let slot = p.wrapping_add((u32::from(i) + 3) * 16);
        ffx_field::vu0::bridge::ke_vu0_copy_vec_xyz(mem, slot, obj.wrapping_add(0x40));
        let h = |i: u32| p.wrapping_add(2 * i);
        for (pos, vel, acc) in [(0, 8, 12), (4, 16, 20)] {
            for j in 0..4 {
                let (x, v, a) = (h(pos + j), h(vel + j), h(acc + j));
                if form == Tail3::Plain {
                    add_i16(mem, x, v);
                    add_i16(mem, v, a);
                } else {
                    add_i16(mem, v, a);
                    add_i16(mem, x, v);
                }
            }
        }
        if form == Tail3::Pht {
            let a = f32::from_bits(mem.read(p + 0x1e8));
            let v = x87::add(a, f32::from_bits(mem.read(p + 0x1e4)));
            mem.write(p + 0x1e4, v.to_bits());
            let x = x87::add(v, f32::from_bits(mem.read(p + 0x1e0)));
            mem.write(p + 0x1e0, x.to_bits());
        }
        if !applies(mem, obj, key) {
            return;
        }
        let k = if form == Tail3::Pht { 0x30 } else { 0x20 };
        for (half, from) in [
            (0, k),
            (8, k + 0x10),
            (12, k + 0x18),
            (4, k + 8),
            (16, k + 0x20),
            (20, k + 0x28),
        ] {
            for j in 0..4 {
                add_i16(mem, h(half + j), key + from + 2 * j);
            }
        }
        if form == Tail3::Pht {
            for (at, from) in [(0x1e0, 0x1c), (0x1e4, 0x20)] {
                let v = x87::add(
                    f32::from_bits(mem.read(p + at)),
                    f32::from_bits(mem.read(key + from)),
                );
                mem.write(p + at, v.to_bits());
            }
            let v = x87::add(
                f32::from_bits(mem.read(key + 0x24)),
                f32::from_bits(mem.read(p + 0x1e8)),
            );
            mem.write(p + 0x1e8, v.to_bits());
        }
    }

    fn add_i16(mem: &mut impl Mem, at: u32, from: u32) {
        let v = mem.read::<u16>(at).wrapping_add(mem.read(from));
        mem.write(at, v);
    }

    /// `pppKeThSft(obj, key, ctrl)` (`0x00736e90`): with its key, area 1's
    /// byte `+0xf0` from the key's `+0x38`, area 0's four floats (`+0xc0`,
    /// `+0xc4`, `+0xc8`, `+0xcc`) and sixteen `i16` (`+0xa0`) += the key's
    /// (`+0x28`, `+0x2c`, `+0x30`, `+0x34`; `+8`); then, except at frame 0,
    /// area 1's floats (`+0xbc`, `+0xd4`, `+0x10c`, `+0xdc`) and `i16`
    /// (`+0x110`) += area 0's.
    pub fn ppp_ke_th_sft(mem: &mut impl Mem, obj: u32, key: u32, ctrl: u32) {
        if STOP.get(mem) != 0 {
            return;
        }
        let w0 = area(mem, obj, ctrl, 0);
        let w1 = area(mem, obj, ctrl, 1);
        if applies(mem, obj, key) {
            let b: u8 = mem.read(key + 0x38);
            mem.write(w1.wrapping_add(0xf0), b);
            for (at, from) in [(0xc8, 0x30), (0xc0, 0x28), (0xc4, 0x2c), (0xcc, 0x34)] {
                let v = x87::add(f(mem, key + from), f(mem, w0.wrapping_add(at)));
                set_f(mem, w0.wrapping_add(at), v);
            }
            for j in 0..16 {
                add_i16(mem, w0.wrapping_add(0xa0 + 2 * j), key + 8 + 2 * j);
            }
            if mem.read::<u32>(obj.wrapping_add(0xc)) == 0 {
                return;
            }
        }
        for (at, from) in [(0x10c, 0xc8), (0xbc, 0xc0), (0xd4, 0xc4), (0xdc, 0xcc)] {
            let v = x87::add(f(mem, w0.wrapping_add(from)), f(mem, w1.wrapping_add(at)));
            set_f(mem, w1.wrapping_add(at), v);
        }
        for j in 0..16 {
            add_i16(
                mem,
                w1.wrapping_add(0x110 + 2 * j),
                w0.wrapping_add(0xa0 + 2 * j),
            );
        }
    }

    /// `pppKeLnsFlsCon(obj, ctrl)` (`0x0075ab30`): through `0x0076a640`,
    /// area 0's `+0xa0` points at `g_pppRandTarget`, `+0xa4` is null.
    pub fn ppp_ke_lns_fls_con(mem: &mut impl Mem, obj: u32, ctrl: u32) {
        let w = area(mem, obj, ctrl, 0).wrapping_add(0xa0);
        let target = crate::rand::bridge::TARGET.ptr(mem).addr;
        mem.write(w, stub::ptr32(target));
        mem.write(w + 4, stub::ptr32(0));
    }

    /// `pppVtMimeDes(obj, ctrl)` (`0x0072ede0`): area 0's buffer (`+0xac`)
    /// back to the current environment's heaps, the pointer cleared.
    pub fn ppp_vt_mime_des(mem: &mut impl Mem, obj: u32, ctrl: u32) {
        let w = area(mem, obj, ctrl, 0);
        let data: u32 = mem.read(w.wrapping_add(0xac));
        if data != 0 {
            let env: u32 = mem.read(crate::heap::bridge::ENV_SLOT.get(mem));
            crate::heap::bridge::ppp_mem_free(mem, env, env.wrapping_add(0x10), data);
            mem.write(w.wrapping_add(0xac), 0u32);
        }
    }

    /// The common part of `pppKeLnsArndUpdate`, `pppKeLnsFlsUpdate` and
    /// `pppKeLnsCrnUpdate(obj, key, ctrl)`: at the key's frame (`key[0]` =
    /// `obj +0xc`), area 0 (`obj + 0xa0 + ctrl.data[0]`) points at area 1
    /// (`+ data[1]`) and, for the key's shape `key[1]` (0xffff: none), at
    /// entry `key[1]` of the environment's shape table (`[g_pppEnvSlot]
    /// +0x24`) unless it already does. Returns the area and the shape
    /// entry to set, or the `eax` the original leaves when it stops.
    #[allow(clippy::many_single_char_names)]
    fn lns_update(mem: &mut impl Mem, obj: u32, key: u32, ctrl: u32) -> Result<(u32, u32), u32> {
        let d: u32 = mem.read(ctrl.wrapping_add(0xc));
        let base = obj.wrapping_add(0xa0);
        let w = base.wrapping_add(mem.read(d));
        let k: u32 = mem.read(key);
        if k != mem.read::<u32>(obj.wrapping_add(0xc)) {
            return Err(k);
        }
        let a1 = base.wrapping_add(mem.read(d.wrapping_add(4)));
        mem.write(w, stub::ptr32(a1));
        let n: u32 = mem.read(key.wrapping_add(4));
        if n == 0xffff {
            mem.write(w.wrapping_add(4), stub::ptr32(0));
            return Err(0);
        }
        let env: u32 = mem.read(crate::heap::bridge::ENV_SLOT.get(mem));
        let table: u32 = mem.read(env.wrapping_add(0x24));
        let e = table.wrapping_add(n.wrapping_mul(4));
        let cur = stub::ptr32(mem.read(w.wrapping_add(4)));
        if e == cur {
            return Err(cur);
        }
        Ok((w, e))
    }

    /// `pppKeLnsArndUpdate(obj, key, ctrl)` (`0x0075a2d0`) and
    /// `pppKeLnsFlsUpdate` (`0x0075ae90`, the same code): see
    /// `lns_update`; the new shape entry goes to area 0's `+4`, its
    /// shape's `u16 +0x12` to `+8`, and `+0xa` is cleared. Returns the
    /// `eax` the original leaves (the shape's high half over that word).
    pub fn ppp_ke_lns_arnd_update(mem: &mut impl Mem, obj: u32, key: u32, ctrl: u32) -> u32 {
        let (w, e) = match lns_update(mem, obj, key, ctrl) {
            Ok(v) => v,
            Err(eax) => return eax,
        };
        mem.write(w.wrapping_add(4), stub::ptr32(e));
        let shape = stub::ptr32(mem.read(stub::ptr32(e)));
        let k: u16 = mem.read(shape.wrapping_add(0x12));
        mem.write(w.wrapping_add(8), k);
        mem.write(w.wrapping_add(0xa), 0u8);
        (shape & 0xffff_0000) | u32::from(k)
    }

    /// `pppKeLnsCrnUpdate(obj, key, ctrl)` (`0x0075aab0`): see
    /// `lns_update`; the new shape entry goes through `KeLnsCrn_SetShp(area,
    /// entry)` (`0x0076a1a0`, through `call`), whose result it returns.
    pub fn ppp_ke_lns_crn_update<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        ctrl: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let (w, e) = match lns_update(mem, obj, key, ctrl) {
            Ok(v) => v,
            Err(eax) => return eax,
        };
        let f = Global::<u32>::new(0x0076_a1a0).ptr(mem).addr;
        call(mem, f, &[w, e])
    }

    /// `pppKeLnsLpCon(obj, ctrl)` (`0x0075af30`): `KeLnsLp_Init` on area 0
    /// (from `+0xa0`), then `+0x9c` and `+0xa8` of it 0.0.
    pub fn ppp_ke_lns_lp_con(mem: &mut impl Mem, obj: u32, ctrl: u32) {
        let w = area(mem, obj, ctrl, 0).wrapping_add(0xa0);
        ke::ke_lns_lp_init(mem, w);
        mem.write(w.wrapping_add(0x9c), 0u32);
        mem.write(w.wrapping_add(0xa8), 0u32);
    }

    /// `g_pppOtList` (`0x0230fff0`): the list of draws the `...T` programs
    /// register their work areas in (`pppOtFunc_Reg`/`pppOtFunc_Lev`).
    pub const OT_LIST: Global<u32> = Global::new(0x0230_fff0);
    /// `g_KeLnsLpTCount`: `pppKeLnsLpTCon` numbers its work areas with it.
    pub const LNS_LP_T_COUNT: Global<u8> = Global::new(0x00d4_b46c);

    /// The `pppKeLns...T` programs: lines drawn from the draw list.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum LnsT {
        /// `pppKeLnsArndT`.
        Arnd,
        /// `pppKeLnsClmT`.
        Clm,
        /// `pppKeLnsCrnT`.
        Crn,
        /// `pppKeLnsFlsT`.
        Fls,
    }

    impl LnsT {
        /// The offsets in the work area of the program byte and the list
        /// node, and the draw the node calls.
        const fn layout(self) -> (u32, u32, u32) {
            match self {
                Self::Arnd => (0xb, 0x24, 0x0075_a250),
                Self::Clm => (0xf, 0x40, 0x0075_a610),
                Self::Crn => (0xb, 0x1c, 0x0075_a9e0),
                Self::Fls => (0xb, 0x20, 0x0075_ade0),
            }
        }
    }

    /// `pppKeLnsArndTCon` (`0x0075a0f0`), `pppKeLnsClmTCon` (`0x0075a4a0`),
    /// `pppKeLnsCrnTCon` (`0x0075a870`) and `pppKeLnsFlsTCon`
    /// (`0x0075ac60`) `(obj, ctrl)`: area 0 (from `+0xa0`) set up as its
    /// `KeLns..._Init` does (`Arnd` with `KeLnsFls_Init`, also storing
    /// `set_ptr32(obj)` at `+0x10`; `Clm` then `+8` = 4; `Fls` as
    /// `pppKeLnsFlsCon` after `set_ptr32(obj)` at `+0xc`), the current
    /// program's low byte stored, and its node registered on `g_pppOtList`
    /// with the program's draw and the area; returns the node
    /// (`pppOtFunc_Reg`'s `eax`, which the original leaves).
    pub fn ppp_ke_lns_t_con(mem: &mut impl Mem, kind: LnsT, obj: u32, ctrl: u32) -> u32 {
        let w = area(mem, obj, ctrl, 0).wrapping_add(0xa0);
        match kind {
            LnsT::Arnd => {
                mem.write(w.wrapping_add(0x10), stub::ptr32(obj));
                ke::ke_lns_init(mem, w);
            }
            LnsT::Clm => {
                ke::ke_lns_clm_init(mem, w);
                mem.write(w.wrapping_add(8), 4u8);
            }
            LnsT::Crn => ke::ke_lns_init(mem, w),
            LnsT::Fls => {
                mem.write(w.wrapping_add(0xc), stub::ptr32(obj));
                let target = crate::rand::bridge::TARGET.ptr(mem).addr;
                mem.write(w, stub::ptr32(target));
                mem.write(w + 4, stub::ptr32(0));
            }
        }
        let (byte, node, draw) = kind.layout();
        let program = part::CURRENT_PROGRAM.get(mem);
        mem.write(w.wrapping_add(byte), program.to_le_bytes()[0]);
        let list = OT_LIST.ptr(mem).addr;
        let draw = Global::<u32>::new(draw).ptr(mem).addr;
        part::ppp_ot_func_reg(mem, list, w.wrapping_add(node), draw, w);
        w.wrapping_add(node)
    }

    /// `pppKeLnsArndTDes` (`0x0075a140`), `pppKeLnsClmTDes` (`0x0075a4f0`),
    /// `pppKeLnsCrnTDes` (`0x0075a8b0`) and `pppKeLnsFlsTDes`
    /// (`0x0075acb0`) `(obj, ctrl)`: `graphicVFXDestroy` (through `call`)
    /// of the instance area:program byte (sign-extended into the high
    /// word), then the node off `g_pppOtList`; returns the list (left in
    /// `eax` by `pppOtFunc_Lev`).
    pub fn ppp_ke_lns_t_des<M: Mem>(
        mem: &mut M,
        kind: LnsT,
        obj: u32,
        ctrl: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let w = area(mem, obj, ctrl, 0).wrapping_add(0xa0);
        let (byte, node, _) = kind.layout();
        let i = i32::from(mem.read::<i8>(w.wrapping_add(byte)));
        let destroy = part::GRAPHIC_VFX_DESTROY.ptr(mem).addr;
        let hi = w.wrapping_add((i >> 31).cast_unsigned());
        call(mem, destroy, &[i.cast_unsigned(), hi]);
        let list = OT_LIST.ptr(mem).addr;
        part::ppp_ot_func_lev(mem, list, w.wrapping_add(node));
        list
    }

    /// `pppKeLnsLpTCon(obj, ctrl)` (`0x0075b090`): as `pppKeLnsLpCon`, and
    /// `+0x8c` the next number of `g_KeLnsLpTCount`, `+0x8d` 2, `+0xd0` 0,
    /// the node `+0xb0` registered on `g_pppOtList` (draw `0x0075b230`)
    /// and `+0xcc` the complement of the list's `+0xe`.
    pub fn ppp_ke_lns_lp_t_con(mem: &mut impl Mem, obj: u32, ctrl: u32) {
        ppp_ke_lns_lp_con(mem, obj, ctrl);
        let w = area(mem, obj, ctrl, 0).wrapping_add(0xa0);
        let n = LNS_LP_T_COUNT.get(mem);
        mem.write(w.wrapping_add(0x8c), n);
        mem.write(w.wrapping_add(0x8d), 2u8);
        mem.write(w.wrapping_add(0xd0), 0u16);
        let list = OT_LIST.ptr(mem).addr;
        let draw = Global::<u32>::new(0x0075_b230).ptr(mem).addr;
        part::ppp_ot_func_reg(mem, list, w.wrapping_add(0xb0), draw, w);
        LNS_LP_T_COUNT.set(mem, n.wrapping_add(1));
        let e = mem.read::<u16>(list.wrapping_add(0xe));
        mem.write(w.wrapping_add(0xcc), !e);
    }

    /// `pppKeLnsLpTDes(obj, ctrl)` (`0x0075b130`): area 0's node (`+0x150`)
    /// off `g_pppOtList`; returns the list.
    pub fn ppp_ke_lns_lp_t_des(mem: &mut impl Mem, obj: u32, ctrl: u32) -> u32 {
        let w = area(mem, obj, ctrl, 0).wrapping_add(0xa0);
        let list = OT_LIST.ptr(mem).addr;
        part::ppp_ot_func_lev(mem, list, w.wrapping_add(0xb0));
        list
    }

    /// The current environment and its spare heap (`pppMemFree`'s first
    /// two arguments).
    fn env_heaps(mem: &impl Mem) -> (u32, u32) {
        let env: u32 = mem.read(crate::heap::bridge::ENV_SLOT.get(mem));
        (env, env.wrapping_add(0x10))
    }

    /// `pppMemFree` of the block a pointer field holds, when it holds one.
    fn free_field(mem: &mut impl Mem, at: u32) {
        let data = stub::ptr32(mem.read(at));
        if data != 0 {
            let (env, spare) = env_heaps(mem);
            crate::heap::bridge::ppp_mem_free(mem, env, spare, data);
        }
    }

    /// `pppDrawMdlSeaDes(obj, ctrl)` (`0x0073ab70`): area 2's blocks at
    /// `+0x108` and `+0x10c` back to the environment's heaps (the fields
    /// kept).
    pub fn ppp_draw_mdl_sea_des(mem: &mut impl Mem, obj: u32, ctrl: u32) {
        let w = area(mem, obj, ctrl, 2);
        free_field(mem, w.wrapping_add(0x108));
        free_field(mem, w.wrapping_add(0x10c));
    }

    /// `g_pppDrawMdlInfActive` (`0x00d4b464`): `pppDrawMdlInfDes` clears it.
    pub const MDL_INF_ACTIVE: Global<u32> = Global::new(0x00d4_b464);

    /// `pppDrawMdlInfDes(obj, ctrl)` (`0x0073b830`): with a saved vertex
    /// copy at area 2 `+0xa0+0xd0`, its `i16` count (the shape's `+0x12`)
    /// of 6-byte vertices goes back into the shape (the record at `+0xdc`
    /// names it by its `+4` index into the environment's shape blocks,
    /// `+0x1c` of 0x20-byte entries at env `+0x20`; vertices at shape +
    /// shape `+8`); then the blocks at `+0xd0`, `+0xd4`, `+0xd8` are freed
    /// and cleared, and `g_pppDrawMdlInfActive` 0.
    pub fn ppp_draw_mdl_inf_des(mem: &mut impl Mem, obj: u32, ctrl: u32) {
        let w = area(mem, obj, ctrl, 2).wrapping_add(0xa0);
        if stub::is_ptr32_null(mem.read(w.wrapping_add(0xd0))) == 0 {
            let rec = stub::ptr32(mem.read(w.wrapping_add(0xdc)));
            let index: u32 = mem.read(rec.wrapping_add(4));
            let (env, _) = env_heaps(mem);
            let blocks: u32 = mem.read(env.wrapping_add(0x20));
            let shape = stub::ptr32(mem.read(blocks.wrapping_add(index << 5).wrapping_add(0x1c)));
            let verts = shape.wrapping_add(mem.read(shape.wrapping_add(8)));
            let mut k = 0i32;
            while k < i32::from(mem.read::<i16>(shape.wrapping_add(0x12))) {
                let off = 6 * k.cast_unsigned();
                for h in [0u32, 2, 4] {
                    let saved = stub::ptr32(mem.read(w.wrapping_add(0xd0)));
                    let v: u16 = mem.read(saved.wrapping_add(off + h));
                    mem.write(verts.wrapping_add(off + h), v);
                }
                k += 1;
            }
        }
        for f in [0xd0u32, 0xd4, 0xd8] {
            free_field(mem, w.wrapping_add(f));
            mem.write(w.wrapping_add(f), stub::ptr32(0));
        }
        MDL_INF_ACTIVE.set(mem, 0);
    }

    /// `rcbgPSimClearOMD(sim)` (`0x0091f9b0`), called through `call`.
    const PSIM_CLEAR_OMD: u32 = 0x0091_f9b0;
    /// `0x0091fd40(sim)` (releases the simulation and `rcFree`s it),
    /// called through `call`.
    const PSIM_FREE: u32 = 0x0091_fd40;

    /// `pppDrawMdlPSimDes(obj, ctrl)` (`0x0073d010`): with a simulation at
    /// area 2 `+0xa0`, `rcbgPSimClearOMD(sim)` and `0x0091fd40(sim)`
    /// (through `call`), then the field cleared.
    pub fn ppp_draw_mdl_psim_des<M: Mem>(
        mem: &mut M,
        obj: u32,
        ctrl: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let at = area(mem, obj, ctrl, 2).wrapping_add(0xa0);
        if stub::is_ptr32_null(mem.read(at)) != 0 {
            return;
        }
        for f in [PSIM_CLEAR_OMD, PSIM_FREE] {
            let sim = stub::ptr32(mem.read(at));
            let f = Global::<u32>::new(f).ptr(mem).addr;
            call(mem, f, &[sim]);
        }
        mem.write(at, stub::ptr32(0));
    }

    /// `KeVu0RndI()` (`0x0075f920`, the map module's), called through
    /// `call`.
    const KE_VU0_RND_I: u32 = 0x0075_f920;
    /// `KeBornRnd2(obj, key, born, area, shape, random, count)`
    /// (`0x00769230`, the map module's), called through `call`.
    const KE_BORN_RND2: u32 = 0x0076_9230;

    /// The `pppKeBornRnd2/3/5/6` steps: where their keys keep the fields,
    /// and the child-making function each passes `KeBornRnd2`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum BornRnd {
        /// `pppKeBornRnd2` (`0x007587b0`): a random mask at `+0xd`.
        Rnd2,
        /// `pppKeBornRnd3` (`0x00758960`): the mask at `+7`.
        Rnd3,
        /// `pppKeBornRnd5` (`0x00758f20`): a countdown from `+0xd`.
        Rnd5,
        /// `pppKeBornRnd6` (`0x00759200`): the countdown from `+7`.
        Rnd6,
    }

    impl BornRnd {
        /// The key offsets of the mask/period, the two definitions that
        /// must not be -1, the shape (`u16`), the random byte and the
        /// count byte; and the born function.
        const fn layout(self) -> ([u32; 6], u32) {
            match self {
                Self::Rnd2 => ([0xd, 0x10, 0x18, 4, 0xe, 0xc], 0x0075_8840),
                Self::Rnd3 => ([7, 0xc, 0x14, 4, 8, 6], 0x0075_89f0),
                Self::Rnd5 => ([0xd, 0x10, 0x18, 4, 0xe, 0xc], 0x0075_8fc0),
                Self::Rnd6 => ([7, 0xc, 0x14, 4, 8, 6], 0x0075_92a0),
            }
        }
    }

    /// `pppKeBornRnd2/3/5/6(obj, key, ctrl)`: unless stopped, children
    /// through `KeBornRnd2(obj, key, born, area 0 + 0xa0, shape, random,
    /// count)` (through `call`). `Rnd2`/`Rnd3` skip when the mask byte is
    /// 0xff or shares a bit with bits 8.. of `KeVu0RndI()`; `Rnd5`/`Rnd6`
    /// count down area byte `+2` from the key's period (0 never), making
    /// children when it is reloaded and leaving it reloaded only when
    /// `KeBornRnd2` makes none. Both need the two definitions not -1.
    pub fn ppp_ke_born_rnd<M: Mem>(
        mem: &mut M,
        kind: BornRnd,
        obj: u32,
        key: u32,
        ctrl: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        if STOP.get(mem) != 0 {
            return;
        }
        let ([m, d1, d2, shape, random, count], born) = kind.layout();
        let mask: u8 = mem.read(key.wrapping_add(m));
        let w = area(mem, obj, ctrl, 0).wrapping_add(0xa0);
        let countdown = matches!(kind, BornRnd::Rnd5 | BornRnd::Rnd6);
        if countdown {
            if mask == 0 {
                return;
            }
            if mem.read::<u8>(w.wrapping_add(2)) != 0 {
                let c: u8 = mem.read(w.wrapping_add(2));
                mem.write(w.wrapping_add(2), c.wrapping_sub(1));
                return;
            }
            mem.write(w.wrapping_add(2), mask);
        } else {
            if mask == 0xff {
                return;
            }
            let f = Global::<u32>::new(KE_VU0_RND_I).ptr(mem).addr;
            let r = call(mem, f, &[]);
            if mask & (r.cast_signed() >> 8).to_le_bytes()[0] != 0 {
                return;
            }
        }
        if mem.read::<u32>(key.wrapping_add(d1)) == u32::MAX
            || mem.read::<u32>(key.wrapping_add(d2)) == u32::MAX
        {
            return;
        }
        let args = [
            obj,
            key,
            Global::<u32>::new(born).ptr(mem).addr,
            w,
            u32::from(mem.read::<u16>(key.wrapping_add(shape))),
            u32::from(mem.read::<u8>(key.wrapping_add(random))),
            u32::from(mem.read::<u8>(key.wrapping_add(count))),
        ];
        let f = Global::<u32>::new(KE_BORN_RND2).ptr(mem).addr;
        let made = call(mem, f, &args);
        if countdown && made != 0 {
            let c: u8 = mem.read(w.wrapping_add(2));
            mem.write(w.wrapping_add(2), c.wrapping_sub(1));
        }
    }

    /// `pppKeBornRnd2_Born(obj, def, shape, out)` (`0x00758840`, and its
    /// copy `pppKeBornRnd5_Born`, `0x00758fc0`): the child `KeBornRnd2`
    /// makes, unless the magic file is 0x2a9 (`MagicFile_GetMagicId`): the
    /// current part's object record `def +0x10` (16 bytes each, from part
    /// `+0x3c`) gets a new object (`pppCreatePObject` through `call`; none:
    /// `*out` = 0 and done) into `*out`, its `+4` = `obj`. Then the `i16`
    /// point `shape[0..3]` (as floats, `w` 1.0) times `obj`'s matrix
    /// (`+0x10`) is the child's area `def +0x14` position (`+0xa0`, `xyz`),
    /// and that minus `obj +0x40`, normalised and scaled by `f32 def +8`,
    /// its area `def +0x1c` velocity. A record address of 0 skips the
    /// object but not the rest (on the child at 0, as the original does).
    /// `frame` is the original's 16-byte local vector.
    pub fn ppp_ke_born_rnd2_born<M: Mem>(
        mem: &mut M,
        obj: u32,
        def: u32,
        shape: u32,
        out: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use ffx_field::vu0::bridge::ke_vu0_copy_vec_xyz;
        use ffx_ps2::vu0::bridge as vu0;
        if ffx_system::instance::bridge::magic_id(mem) == 0x2a9 {
            return;
        }
        let part = CURRENT_PART.get(mem);
        let records: u32 = mem.read(part.wrapping_add(0x3c));
        let rec = mem
            .read::<u32>(def.wrapping_add(0x10))
            .wrapping_shl(4)
            .wrapping_add(records);
        if rec == 0 {
            mem.write(out, 0u32);
        } else {
            let create = Global::<u32>::new(0x0071_6770).ptr(mem).addr;
            let child = call(mem, create, &[part, rec]);
            mem.write(out, child);
            if child == 0 {
                return;
            }
            mem.write(child.wrapping_add(4), stub::ptr32(obj));
        }
        let child: u32 = mem.read(out);
        let at = |k: u32| -> u32 {
            let area: u32 = mem.read(def.wrapping_add(k));
            area.wrapping_add(0xa0).wrapping_add(child)
        };
        let (pos, vel) = (at(0x14), at(0x1c));
        let p: [i16; 3] = mem.read(shape);
        let v = p.map(f32::from);
        mem.write(frame, [v[0], v[1], v[2], 1.0]);
        vu0::apply_matrix(mem, frame, obj.wrapping_add(0x10), frame);
        ke_vu0_copy_vec_xyz(mem, pos, frame);
        vu0::sub_vector(mem, frame, frame, obj.wrapping_add(0x40));
        vu0::normalize(mem, frame, frame);
        let speed = x87::load_f32(mem.read(def.wrapping_add(8)));
        vu0::scale_vector(mem, frame, frame, speed);
        ke_vu0_copy_vec_xyz(mem, vel, frame);
    }

    /// `g_pppChrScale` (`0x0230fd50`): the character's scale (`x`, `y`,
    /// `z`) the `pppChr*Scl*Matrix` steps scale by.
    pub const CHR_SCALE: Global<[f32; 3]> = Global::new(0x0230_fd50);

    /// The `pppChr*Scl*Matrix` steps (`0x00734910`..`0x00734af0`).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ChrScl {
        /// `pppChrSclXYZMatrix` (`0x00734910`): x scale on x and z, y on y.
        Xyz,
        /// `pppChrSclXZMatrix` (`0x007349b0`): x scale on x and z.
        Xz,
        /// `pppChrSclYMatrix` (`0x00734a30`): y scale on y.
        Y,
        /// `pppChrYSclXYZMatrix` (`0x00734aa0`): reads its own unit
        /// matrix's z (1.0) for all three, so it multiplies by the unit
        /// matrix.
        YXyz,
        /// `pppChrXSclXYZMatrix` (`0x00734af0`): x scale on all three.
        XXyz,
    }

    /// The `pppChr*Scl*Matrix(obj, key)` steps: `obj +0x10` =
    /// `FFXVu0MulMatrix(scale, obj +0x10)` (through `call`; `frame`, 16
    /// words, is the original's local) with a diagonal scale (see
    /// [`ChrScl`]) of `g_pppChrScale` component `c` grown by `key +4`:
    /// `f32(c + c * k)` (`Xyz`, the product with `k` as memory operand) or
    /// `f32(k * c + c)` (the others), capped at `key +8` when that is
    /// below it. (`YXyz` reads no key: `key` is unused there.)
    pub fn ppp_chr_scl_matrix<M: Mem>(
        mem: &mut M,
        kind: ChrScl,
        obj: u32,
        key: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use x87::{ADD, MUL, Reg};
        ffx_ps2::vu0::bridge::matrix_identity(mem, frame);
        let scale = |mem: &M, axis: u32| -> f32 {
            let c = mem.read::<f32>(CHR_SCALE.ptr(mem).addr.wrapping_add(4 * axis));
            let grow: f32 = mem.read(key.wrapping_add(4));
            let gc = Reg::load_f32(c);
            let v = if kind == ChrScl::Xyz {
                gc.reg(ADD, gc.mem(MUL, grow))
            } else {
                Reg::load_f32(grow).reg(MUL, gc).reg(ADD, gc)
            }
            .store();
            let cap: f32 = mem.read(key.wrapping_add(8));
            if f64::from(cap) < f64::from(v) {
                x87::load_f32(cap)
            } else {
                v
            }
        };
        let diag = |i: u32| frame.wrapping_add(0x14 * i);
        match kind {
            ChrScl::Xyz => {
                let (x, y) = (scale(mem, 0), scale(mem, 1));
                mem.write(diag(0), x);
                mem.write(diag(2), x);
                mem.write(diag(1), y);
            }
            ChrScl::Xz => {
                let x = scale(mem, 0);
                mem.write(diag(0), x);
                mem.write(diag(2), x);
            }
            ChrScl::Y => {
                let y = scale(mem, 1);
                mem.write(diag(1), y);
            }
            ChrScl::YXyz => {}
            ChrScl::XXyz => {
                let x = scale(mem, 0);
                for i in 0..3 {
                    mem.write(diag(i), x);
                }
            }
        }
        let m = obj.wrapping_add(0x10);
        let mul = Global::<u32>::new(0x006e_d5c0).ptr(mem).addr;
        call(mem, mul, &[m, frame, m]);
    }

    /// `pppKeOfsPt(obj, key, ctrl)` (`0x0075cd30`): unless stopped, on the
    /// key's frame: at frame 0, with a shape (key `+0x2c`, not 0xffff; the
    /// environment's shape blocks) `0x0075c660(area 0 + 0xb0, shape)`
    /// through `call`; then area 0 `+0xa0..+0xa8` += key `+0x10..+0x18`
    /// (`f32`) and `+0xc0..+0xc8` = key `+0x20..+0x28`.
    pub fn ppp_ke_ofs_pt<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        ctrl: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let w = area(mem, obj, ctrl, 0);
        if STOP.get(mem) != 0 || !applies(mem, obj, key) {
            return;
        }
        if mem.read::<u32>(obj.wrapping_add(0xc)) == 0 {
            let s: u32 = mem.read(key.wrapping_add(0x2c));
            if s == 0xffff {
                return;
            }
            let (env, _) = env_heaps(mem);
            let blocks: u32 = mem.read(env.wrapping_add(0x20));
            let shape = stub::ptr32(mem.read(blocks.wrapping_add(s << 5).wrapping_add(0x1c)));
            let f = Global::<u32>::new(0x0075_c660).ptr(mem).addr;
            call(mem, f, &[w.wrapping_add(0xb0), shape]);
        }
        for i in 0..3 {
            let at = w.wrapping_add(0xa0 + 4 * i);
            let v = x87::add(mem.read(key.wrapping_add(0x10 + 4 * i)), mem.read(at));
            mem.write(at, v);
        }
        for i in 0..3 {
            let v = x87::load(mem.read(key.wrapping_add(0x20 + 4 * i)));
            mem.write(w.wrapping_add(0xc0 + 4 * i), v);
        }
    }

    /// `pppKeAcmCic(obj, key)` (`0x0075b690`): unless the key's byte `+4`
    /// is 2, `0x00644be0(0, 1)` (the renderer's) through `call`.
    pub fn ppp_ke_acm_cic<M: Mem>(
        mem: &mut M,
        key: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        if mem.read::<u8>(key.wrapping_add(4)) != 2 {
            let f = Global::<u32>::new(0x0064_4be0).ptr(mem).addr;
            call(mem, f, &[0, 1]);
        }
    }
}
