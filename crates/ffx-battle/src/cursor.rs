//! Where the battle's cursors and overdrive bars go on the screen
//! (`docs/rewrite-plan.md`, M2): `MsCalcCursorPos` (`0x0079f380`) projects
//! every shown actor's cursor points each frame (`MsBattleCursorCalc`),
//! `MsGetChrLimitBarFlag` (`0x00795250`) places an actor's overdrive bar
//! and keeps it on the screen, `getSparedLmtPos` (`0x00796c30`) projects
//! the bar's spare position.
//!
//! The screen is 512 x 416 at the PS2's size: a projected point `(x, y)`
//! (`graphicProject3DPointOntoScreenConsider2DScene`, whole pixels of the
//! window, from its centre) is `(x / sx + 256, y / sy + 208)`, truncated,
//! with `sx = width / 512`, `sy = height / 416`
//! (`graphicFontGetScreenWH`). The projection and `graphicUiRemapY2` return
//! in `ST0`: they go through `fcall` (the `f32`'s bits); the screen size
//! and `MagicFile_GetMagicId` through `call`. The originals' frame locals
//! the callees write are `local` (see each function).

/// The game's memory, through [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::Mem;
    use ffx_bridge::x87::{add, div, ftol, load, mul, store, sub};

    use crate::exec::bridge::Call;
    use crate::pos::bridge::Trig;

    /// `graphicFontGetScreenWH(&w, &h)` (graphics), the projection
    /// (`graphicProject3DPointOntoScreenConsider2DScene(out, p)`, `1 / w`
    /// in `ST0`), `graphicUiRemapY2(y)` (`ST0`) and
    /// `MagicFile_GetMagicId()` (system).
    pub const FONT_GET_SCREEN_WH: u32 = 0x0064_0db0;
    pub const PROJECT_POINT: u32 = 0x0064_28f0;
    pub const UI_REMAP_Y2: u32 = 0x0064_4820;
    pub const MAGIC_FILE_GET_MAGIC_ID: u32 = 0x009d_a3b0;
    /// `MsNumberDrawProcess()` (`0x0079f660`).
    pub const MS_NUMBER_DRAW_PROCESS: u32 = 0x0079_f660;
    /// The two magic files (by ID) during which no overdrive bar shows.
    pub const NO_BAR_MAGIC: [u32; 2] = [0x130, 0x254];

    /// The screen's scales `(width / 512, height / 416)` as `f32` bits
    /// (`graphicFontGetScreenWH` into `wh`, two words).
    fn screen_scale<M: Mem>(mem: &mut M, call: Call<'_, M>, wh: u32) -> (u32, u32) {
        let f = mem.rebase(FONT_GET_SCREEN_WH);
        call(mem, f, &[wh, wh.wrapping_add(4)]);
        let w: i32 = mem.read(wh);
        let h: i32 = mem.read(wh.wrapping_add(4));
        let sx = store(mul(load(store(f64::from(w))), 1.0 / 512.0));
        let sy = store(div(load(store(f64::from(h))), 416.0));
        (sx, sy)
    }

    /// A projected coordinate `v` to the screen: `v / scale + centre`
    /// (`v` through an `f32`), truncated.
    fn to_screen(mem: &impl Mem, v: u32, scale: u32, centre: f64) -> u32 {
        let v = load(store(f64::from(v.cast_signed())));
        ftol(mem, add(div(v, load(scale)), centre))
    }

    /// The projection of `p` into `out` (two words); its `ST0` (`f32`
    /// bits).
    fn project<M: Mem>(mem: &mut M, fcall: Call<'_, M>, out: u32, p: u32) -> u32 {
        let f = mem.rebase(PROJECT_POINT);
        fcall(mem, f, &[out, p])
    }

    /// `getSparedLmtPos(i, x, y)` (`0x00796c30`) without its out words:
    /// actor `i & 0xff`'s position (`MsGetChrPos`) at its ground
    /// (`MsGetChrGround`) less its [`crate::minor::bridge::CHR_EXTRA`]
    /// slot's spare height (`+8`, [`ms_set_chr_extra_slot`]), projected to
    /// the screen. `local` is the original's frame: the screen size (+0,
    /// two words), the projection (+8, two words), the position (+0x10,
    /// four).
    ///
    /// [`ms_set_chr_extra_slot`]: crate::minor::bridge::ms_set_chr_extra_slot
    pub fn spared_lmt_pos<M: Mem>(
        mem: &mut M,
        trig: &dyn Fn(Trig, u32) -> u32,
        call: Call<'_, M>,
        fcall: Call<'_, M>,
        i: u32,
        local: u32,
    ) -> (u32, u32) {
        let i = i & 0xff;
        let (out, p) = (local.wrapping_add(8), local.wrapping_add(0x10));
        let (sx, sy) = screen_scale(mem, call, local);
        crate::pos::bridge::ms_get_chr_pos(mem, trig, i, p);
        let ground = crate::body::bridge::ms_get_chr_ground(mem, i);
        let slot = crate::minor::bridge::ms_set_chr_extra_slot(mem, i);
        let extra = crate::minor::bridge::CHR_EXTRA.get(mem);
        let spare: u32 = mem.read(extra.wrapping_add(slot.wrapping_mul(0xc)).wrapping_add(8));
        let y = store(sub(f64::from(ground), load(spare)));
        mem.write(p.wrapping_add(4), y);
        project(mem, fcall, out, p);
        let ox: u32 = mem.read(out);
        let oy: u32 = mem.read(out.wrapping_add(4));
        (to_screen(mem, ox, sx, 256.0), to_screen(mem, oy, sy, 208.0))
    }

    /// `getSparedLmtPos(i, x, y)` (`0x00796c30`): [`spared_lmt_pos`] into
    /// `*x`, `*y`; `eax` the y.
    pub fn get_spared_lmt_pos<M: Mem>(
        mem: &mut M,
        trig: &dyn Fn(Trig, u32) -> u32,
        [call, fcall]: [Call<'_, M>; 2],
        [i, x, y]: [u32; 3],
        local: u32,
    ) -> u32 {
        let (sx, sy) = spared_lmt_pos(mem, trig, call, fcall, i, local);
        mem.write(x, sx);
        mem.write(y, sy);
        sy
    }

    /// The overdrive bar's y limits at the PS2's size (`graphicUiRemapY2`
    /// of them): the bar moves to its spare position past the lower one
    /// (pulled 10 up) or the upper one (10 down).
    pub const BAR_LOW: f32 = 760.0;
    pub const BAR_HIGH: f32 = 160.0;

    /// Bytes of [`ms_get_chr_limit_bar_flag`]'s `local`.
    pub const LIMIT_BAR_LOCAL: u32 = 0x60;

    /// `MsGetChrLimitBarFlag(i)` (`0x00795250`): where actor `i & 0xff`'s
    /// overdrive bar goes (`+0xf54` x, `+0xf58` y) and whether it shows (the
    /// result). Not during the two [`NO_BAR_MAGIC`] files, nor for an actor
    /// without its bar (`+0x718`, `+0x719`), not shown (`+0x15`) or with a
    /// zero `+0x40` (0, nothing written). Else its position (`MsGetChrPos`)
    /// at its ground less `+0x4ec`, projected to the screen: behind the
    /// camera (`1 / w` not above 0) at x -0x200, hidden; off the screen
    /// (x above 0x200 unsigned, y outside 10..=0x196) hidden; else shown,
    /// and while the camera level is 5 or below a y past
    /// `graphicUiRemapY2(760)` moves to the spare position
    /// ([`spared_lmt_pos`], 10 up) when the actor is in
    /// [`crate::minor::bridge::CHR_EXTRA`] and it is not past it, else
    /// after `MesSetChrTargetPosToLimitSparePosy` to the spare position
    /// again (10 up, at most the limit, truncated); the same for a y
    /// before `graphicUiRemapY2(160)` (10 down, at least the limit).
    /// `local` is the original's frame: the screen size (+0), the
    /// projection (+8), the position (+0x10), then [`spared_lmt_pos`]'s
    /// (+0x20, 0x20 bytes) and `MesSetChrTargetPosToLimitSparePosy`'s
    /// (+0x40, 0x10 bytes); [`LIMIT_BAR_LOCAL`] bytes.
    #[allow(clippy::too_many_lines)]
    pub fn ms_get_chr_limit_bar_flag<M: Mem>(
        mem: &mut M,
        trig: &dyn Fn(Trig, u32) -> u32,
        call: Call<'_, M>,
        fcall: Call<'_, M>,
        i: u32,
        local: u32,
    ) -> u32 {
        use crate::minor::bridge::{mes_set_chr_target_pos_to_limit_spare_posy, ms_chr_in_extra};
        use core::cmp::Ordering;
        let i = i & 0xff;
        let (proj, pos) = (local.wrapping_add(8), local.wrapping_add(0x10));
        let (spare_local, mes_local) = (local.wrapping_add(0x20), local.wrapping_add(0x40));
        let (sx, sy) = screen_scale(mem, &mut *call, local);
        let actor = crate::actor::bridge::ms_get_chr(mem, i);
        let magic = mem.rebase(MAGIC_FILE_GET_MAGIC_ID);
        if call(mem, magic, &[]) == NO_BAR_MAGIC[0] || call(mem, magic, &[]) == NO_BAR_MAGIC[1] {
            return 0;
        }
        let set = |mem: &M, o: u32| mem.read::<u8>(actor.wrapping_add(o)) != 0;
        if !set(mem, 0x718) || !set(mem, 0x719) || !set(mem, 0x15) {
            return 0;
        }
        if load(mem.read(actor.wrapping_add(0x40))) == 0.0 {
            return 0;
        }
        crate::pos::bridge::ms_get_chr_pos(mem, trig, i, pos);
        let ground = crate::body::bridge::ms_get_chr_ground(mem, i);
        let lift: u32 = mem.read(actor.wrapping_add(0x4ec));
        mem.write(
            pos.wrapping_add(4),
            store(sub(f64::from(ground), load(lift))),
        );
        let w = project(mem, &mut *fcall, proj, pos);
        let ox: u32 = mem.read(proj);
        let oy: u32 = mem.read(proj.wrapping_add(4));
        let mut x = to_screen(mem, ox, sx, 256.0);
        let mut y = to_screen(mem, oy, sy, 208.0);
        let shown = if !(load(w) > 0.0 || load(w).is_nan()) {
            x = 0xffff_fe00;
            0
        } else if x > 0x200 || y.cast_signed() < 10 || y.cast_signed() > 0x196 {
            0
        } else {
            if crate::camera::bridge::ms_get_now_selected_camra_level(mem) <= 5 {
                let remap = mem.rebase(UI_REMAP_Y2);
                let yf = |v: u32| load(store(f64::from(v.cast_signed())));
                // past the lower limit: (y as f32) at or past it
                let low = load(fcall(mem, remap, &[BAR_LOW.to_bits()]));
                if low <= yf(y) {
                    let mut moved = false;
                    if ms_chr_in_extra(mem, i) != 0 {
                        (x, y) = spared_lmt_pos(mem, trig, &mut *call, &mut *fcall, i, spare_local);
                        y = y.wrapping_sub(10);
                    }
                    if ms_chr_in_extra(mem, i) != 0
                        && !matches!(
                            low.partial_cmp(&yf(y)),
                            Some(Ordering::Less | Ordering::Equal)
                        )
                    {
                        moved = true;
                    }
                    if !moved {
                        mes_set_chr_target_pos_to_limit_spare_posy(mem, i, mes_local);
                        (x, y) = spared_lmt_pos(mem, trig, &mut *call, &mut *fcall, i, spare_local);
                        y = y.wrapping_sub(10);
                        if low < yf(y) {
                            y = ftol(mem, low);
                        }
                    }
                }
                // before the upper limit
                let high = load(fcall(mem, remap, &[BAR_HIGH.to_bits()]));
                if !(high < yf(y) || high.is_nan()) {
                    let mut kept = false;
                    if ms_chr_in_extra(mem, i) != 0 {
                        (x, y) = spared_lmt_pos(mem, trig, &mut *call, &mut *fcall, i, spare_local);
                        y = y.wrapping_add(10);
                    }
                    if ms_chr_in_extra(mem, i) != 0 && (high < yf(y) || high.is_nan()) {
                        kept = true;
                    }
                    if !kept {
                        mes_set_chr_target_pos_to_limit_spare_posy(mem, i, mes_local);
                        (x, y) = spared_lmt_pos(mem, trig, &mut *call, &mut *fcall, i, spare_local);
                        y = y.wrapping_add(10);
                        if high > yf(y) {
                            y = ftol(mem, high);
                        }
                    }
                }
            }
            1
        };
        mem.write(actor.wrapping_add(0xf54), x);
        mem.write(actor.wrapping_add(0xf58), y);
        shown
    }

    /// Bytes of [`ms_calc_cursor_pos`]'s `local`.
    pub const CURSOR_LOCAL: u32 = 0x78;

    /// `MsCalcCursorPos()` (`0x0079f380`): each of the 31 shown actors'
    /// (`+0x10`) cursor points on the screen. The battle's camera and
    /// screen matrices are multiplied (`MsMulMatrix(local + 0x10, screen,
    /// camera)`, the product unused). The target point, the character's
    /// optional position `(i8) +0xdf9` (`Ch_OptposGet`, its centre without
    /// a character, `MsGetChrCenterPos`), is projected: `+0xf34`, `+0xf38`
    /// the projection, `+0xf3c` its screen x (-0x200 behind the camera:
    /// `1 / w` not above 0), `+0xf40` its y. The camera point, optional
    /// position `+0x442` (unless 0xff; `MsGetChrCameraPos` without one or
    /// a character) less `+0x4e8` in y (the point, unlowered, also into
    /// the EE register move words, [`EE_GPR_MOVE`]), to the screen at
    /// `+0xf44`, `+0xf48`, and clamped to 0x1b..=0x1e5, 0x22..=0x187 at
    /// `+0xf4c`, `+0xf50`. `local` is the original's frame: the screen size
    /// (+0), the product (+0x10, 0x40 bytes), the projection (+0x50), the
    /// point (+0x58, four words) and the camera point (+0x68 is the
    /// original's -0x34: `local + 0x60`, four words); [`CURSOR_LOCAL`]
    /// bytes.
    ///
    /// [`EE_GPR_MOVE`]: crate::after_damage::bridge::EE_GPR_MOVE
    pub fn ms_calc_cursor_pos<M: Mem>(
        mem: &mut M,
        trig: &dyn Fn(Trig, u32) -> u32,
        call: Call<'_, M>,
        fcall: Call<'_, M>,
        local: u32,
    ) {
        use crate::target::bridge::{ms_get_chr_camera_pos, ms_get_chr_center_pos};
        use ffx_chara::attach::bridge::ch_optpos_get;
        let (product, out) = (local.wrapping_add(0x10), local.wrapping_add(0x50));
        let (target, eye) = (local.wrapping_add(0x58), local.wrapping_add(0x68));
        let (sx, sy) = screen_scale(mem, call, local);
        let camera = crate::camera::bridge::ms_get_camera_matrix(mem);
        let screen = crate::camera::bridge::ms_get_screen_matrix(mem);
        crate::vector::bridge::ms_mul_matrix(mem, product, screen, camera);
        for k in 0..crate::actor::ACTORS {
            let actor = crate::actor::bridge::ms_get_chr(mem, k);
            if mem.read::<u8>(actor.wrapping_add(0x10)) == 0 {
                continue;
            }
            let chr: u32 = mem.read(actor);
            if chr == 0 {
                ms_get_chr_center_pos(mem, trig, k, target);
            } else {
                let opt = i32::from(mem.read::<i8>(actor.wrapping_add(0xdf9)));
                ch_optpos_get(mem, chr, opt.cast_unsigned(), target);
            }
            let w = project(mem, &mut *fcall, out, target);
            let ox: u32 = mem.read(out);
            let oy: u32 = mem.read(out.wrapping_add(4));
            mem.write(actor.wrapping_add(0xf34), ox);
            mem.write(actor.wrapping_add(0xf38), oy);
            let x = to_screen(mem, ox, sx, 256.0);
            let y = to_screen(mem, oy, sy, 208.0);
            let w = load(w);
            let x = if w > 0.0 || w.is_nan() {
                x
            } else {
                0xffff_fe00
            };
            mem.write(actor.wrapping_add(0xf3c), x);
            mem.write(actor.wrapping_add(0xf40), y);
            let opt = mem.read::<u8>(actor.wrapping_add(0x442));
            if opt == 0xff || chr == 0 {
                ms_get_chr_camera_pos(mem, trig, k, eye);
            } else {
                ch_optpos_get(mem, chr, u32::from(opt), eye);
            }
            let v: [u32; 4] = mem.read(eye);
            let lift: u32 = mem.read(actor.wrapping_add(0x4e8));
            crate::after_damage::bridge::EE_GPR_MOVE.set(mem, v);
            let y = store(sub(load(v[1]), load(store(load(lift)))));
            mem.write(target, [v[0], y, v[2], v[3]]);
            project(mem, &mut *fcall, out, target);
            let ox: u32 = mem.read(out);
            let oy: u32 = mem.read(out.wrapping_add(4));
            let x = to_screen(mem, ox, sx, 256.0);
            let y = to_screen(mem, oy, sy, 208.0);
            mem.write(actor.wrapping_add(0xf44), x);
            mem.write(actor.wrapping_add(0xf48), y);
            let x = x.cast_signed().clamp(0x1b, 0x1e5);
            let y = y.cast_signed().clamp(0x22, 0x187);
            mem.write(actor.wrapping_add(0xf4c), x);
            mem.write(actor.wrapping_add(0xf50), y);
        }
    }

    /// `MsBattleCursorCalc()` (`0x00780cc0`): during a battle
    /// (`MsBattleGetStatus`) the cursors ([`ms_calc_cursor_pos`]) and the
    /// damage numbers (`MsNumberDrawProcess`, which the original jumps
    /// to; through `call`).
    pub fn ms_battle_cursor_calc<M: Mem>(
        mem: &mut M,
        trig: &dyn Fn(Trig, u32) -> u32,
        call: Call<'_, M>,
        fcall: Call<'_, M>,
        local: u32,
    ) {
        if crate::state::bridge::ms_battle_get_status(mem) == 0 {
            return;
        }
        ms_calc_cursor_pos(mem, trig, &mut *call, fcall, local);
        let f = mem.rebase(MS_NUMBER_DRAW_PROCESS);
        call(mem, f, &[]);
    }
}
