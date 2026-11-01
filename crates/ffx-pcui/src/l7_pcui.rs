//! M5 stage 1, round 7 (list `left7`, from `pc-ui.list`): the sphere
//! grid's moving token (`AbmapUpdateMovingChrPos`): a point carried from
//! one node to the next along an arc around the grid's centre.
//!
//! Floats follow the x87 at 53 bits: f64 in the original's order, rounded
//! to f32 at each of its stores (the emulated VU0 registers and the frame's
//! spills included); `_CIatan2` and `_CIsqrt` stay calls to the CRT.

/// The originals over game memory.
#[allow(clippy::many_single_char_names)] // the original's quantities
pub mod bridge {
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// The CRT intrinsics `_CIatan2` (`ST1`, `ST0`) and `_CIsqrt`.
    pub const CI_ATAN2: u32 = 0x0094_98bc;
    pub const CI_SQRT: u32 = 0x0094_97b8;
    /// `g_Vu0_Vf00` (x, y, z, w), the emulated VU0 `vf` cells the
    /// original computes in (0x00c8f810.., 0x00c8f820.., 0x00c8f830..) and
    /// `Q`; `g_pppSinTable` (4096 f32 over a turn).
    pub const VU0_VF00: u32 = 0x00c0_a004;
    pub const VU0_A: u32 = 0x00c8_f810;
    pub const VU0_B: u32 = 0x00c8_f820;
    pub const VU0_C: u32 = 0x00c8_f830;
    pub const VU0_Q: u32 = 0x00c8_f788;
    pub const SIN_TABLE: u32 = 0x00c4_4bf0;
    /// f64 pi, 2 pi, -pi (each the f32 value) and 32768.
    pub const PI: u32 = 0x00b9_2288;
    pub const TWO_PI: u32 = 0x00b3_9378;
    pub const MINUS_PI: u32 = 0x00b5_c008;
    pub const HALF_TURN: u32 = 0x00b5_1588;

    fn f(h: &impl Host, p: u32) -> f64 {
        x87::load(h.read(p))
    }

    fn k64(h: &impl Host, sym: u32) -> f64 {
        f64::from_bits(h.read(h.rebase(sym)))
    }

    /// `fstp dword` of `x` to `p`, the bits.
    fn put(h: &mut impl Host, p: u32, x: f64) -> u32 {
        let b = x87::store(x);
        h.write(p, b);
        b
    }

    /// The emulated VU0 length of the vector in `VU0_C` (its x, y, z
    /// cells already hold the vector): the squares, their sum through
    /// `VU0_B`'s y and z, `_CIsqrt` of its magnitude (each step stored as
    /// f32).
    fn vu0_length(h: &mut impl Host, x: u32, y: u32) -> u32 {
        let c = h.rebase(VU0_C);
        let b = h.rebase(VU0_B);
        // the squares: x by itself, y by its cell, z (0) by 0
        put(h, c, x87::mul(x87::load(x), x87::load(x)));
        let yy = x87::mul(x87::load(y), f(h, c + 4));
        put(h, c + 4, yy);
        let zz = x87::mul(f(h, c + 8), 0.0);
        put(h, c + 8, zz);
        let s = x87::add(f(h, c + 4), f(h, c));
        put(h, b + 4, s);
        let s = x87::add(f(h, c + 8), f(h, b + 4));
        put(h, b + 8, s);
        let m = x87::store(f(h, b + 8).abs());
        let sqrt = h.rebase(CI_SQRT);
        h.call_intrinsic(sqrt, x87::load(m)).to_bits()
    }

    /// `AbmapUpdateMovingChrPos(out, from, to, centre, t)`: `out` (x, y,
    /// z, w) a point between `from` and `to` on the arc around `centre`:
    /// the angles of `from - centre` and `to - centre` (`_CIatan2`), their
    /// difference brought into -pi..pi (`+-2 pi` once), the distances
    /// through the emulated VU0 (whose cells, `Q` and the normalised
    /// `to - centre` are left as the original leaves them), both
    /// interpolated by `t`; the angle as a 16-bit turn (`ftol(a * 32768 /
    /// pi)`) into the sine table (`(u + 0x4000) >> 4` its cosine, `u >> 4`
    /// its sine); z the centre's, w 1. Returns the sine's index (the
    /// `eax` the original leaves).
    pub fn update_moving_chr_pos(
        h: &mut impl Host,
        out: u32,
        from: u32,
        to: u32,
        centre: u32,
        t: u32,
    ) -> u32 {
        let (cx, cy) = (f(h, centre), f(h, centre + 4));
        let d1x = x87::store(x87::sub(f(h, from), cx));
        let d1y = x87::store(x87::sub(f(h, from + 4), cy));
        let atan2 = h.rebase(CI_ATAN2);
        let ang1 = h
            .call_intrinsic2(atan2, x87::load(d1y), x87::load(d1x))
            .to_bits();
        let d2x = x87::store(x87::sub(f(h, to), cx));
        let d2y = x87::store(x87::sub(f(h, to + 4), cy));
        let ang2 = h
            .call_intrinsic2(atan2, x87::load(d2y), x87::load(d2x))
            .to_bits();
        let one = 1.0_f32.to_bits();
        let c = h.rebase(VU0_C);
        let v0 = h.rebase(VU0_VF00);
        for (o, v) in [(0, d1x), (4, d1y), (8, 0), (0xc, one)] {
            h.write(c + o, v);
        }
        let r1 = vu0_length(h, d1x, d1y);
        for (o, v) in [(0, d2x), (4, d2y), (8, 0), (0xc, one)] {
            h.write(c + o, v);
        }
        let len1 = x87::load(x87::store(x87::add(x87::load(r1), f(h, v0))));
        // to - centre plus vf00 into the first cell
        let a = h.rebase(VU0_A);
        put(h, a, x87::add(f(h, v0), x87::load(d2x)));
        put(h, a + 4, x87::add(x87::load(d2y), f(h, v0 + 4)));
        put(h, a + 8, x87::add(f(h, v0 + 8), 0.0));
        let r2 = vu0_length(h, d2x, d2y);
        let len2 = put(h, c, x87::add(x87::load(r2), f(h, v0)));
        let q = put(
            h,
            h.rebase(VU0_Q),
            x87::div(f(h, v0 + 0xc), x87::load(len2)),
        );
        let q = x87::load(q);
        let b = h.rebase(VU0_B);
        put(h, b, x87::mul(f(h, a), q));
        put(h, b + 4, x87::mul(f(h, a + 4), q));
        put(h, b + 8, x87::mul(q, f(h, a + 8)));
        // the turn: the difference into -pi..pi
        let (a1, a2) = (x87::load(ang1), x87::load(ang2));
        let pi = k64(h, PI);
        let mut d = x87::load(x87::store(x87::sub(a2, a1)));
        if a1 <= a2 {
            if d > pi {
                d = x87::load(x87::store(x87::sub(d, k64(h, TWO_PI))));
            }
        } else if d < k64(h, MINUS_PI) {
            d = x87::load(x87::store(x87::add(d, k64(h, TWO_PI))));
        }
        let (len1, len2) = (len1, x87::load(len2));
        let t = x87::load(t);
        let radius = x87::store(x87::add(len1, x87::mul(x87::sub(len2, len1), t)));
        let angle = x87::store(x87::add(a1, x87::mul(d, t)));
        let turn = x87::div(x87::mul(x87::load(angle), k64(h, HALF_TURN)), pi);
        let u = x87::ftol(h, turn) & 0xffff;
        let table = h.rebase(SIN_TABLE);
        let cos = f(h, table + 4 * (((u + 0x4000) >> 4) & 0xfff));
        let sin = f(h, table + 4 * (u >> 4));
        let r = x87::load(radius);
        // the centre read again (fld [edi], fadd [edi+4])
        let cx = f(h, centre);
        put(h, out, x87::add(x87::mul(cos, r), cx));
        let cy = f(h, centre + 4);
        put(h, out + 4, x87::add(x87::mul(r, sin), cy));
        let z = f(h, centre + 8);
        put(h, out + 8, z);
        h.write(out + 12, one);
        u >> 4
    }

    /// The camera test window's callees: `Gu_WindowSetTitle(window,
    /// text)`, `Gu_WindowAddButton(window, id, text, x, y, w, h)`,
    /// `AtelGetNowCtrlTopId()`, `AtelGetEventActorPos(id, &x, &y, &z)`,
    /// `spline_get_near_point_init(spline, x, y, z, cubic)`,
    /// `spline_get_near_point(spline, x, y, z, &px, &py, &pz, cubic)`
    /// (`ST0`), `TkMtCalcSpline(out, points, n, t)`,
    /// `TkMtCalcSpline3(out, points, n, t)`, `MsCameraSetRoll(cam, r)`,
    /// `MsCameraSetRect(cam, side, &xyz)`, `dbgPrintf(fmt)`, the three
    /// path players (`CameraPath_*`).
    pub const SET_TITLE: u32 = 0x0084_b6a0;
    pub const ADD_BUTTON: u32 = 0x0084_a470;
    pub const NOW_CTRL_TOP_ID: u32 = 0x0086_c1a0;
    pub const EVENT_ACTOR_POS: u32 = 0x0086_b1e0;
    pub const NEAR_POINT_INIT: u32 = 0x00a4_4420;
    pub const NEAR_POINT: u32 = 0x00a4_4120;
    pub const CALC_SPLINE: u32 = 0x0088_a4a0;
    pub const CALC_SPLINE3: u32 = 0x0088_a4d0;
    pub const CAMERA_SET_ROLL: u32 = 0x007b_fa80;
    pub const CAMERA_SET_RECT: u32 = 0x007b_f940;
    pub const DBG_PRINTF: u32 = 0x0062_f500;
    pub const PATH_PLAYERS: [u32; 3] = [0x0094_02c0, 0x0094_0380, 0x0094_0480];
    /// The test's state (on, mode, time, step: 0x0193c378..84), the
    /// near point's copy (0x0193c174, 0x0193c388, 0x0193c38c), the
    /// window holder byte (0x025d5f30), the spline (cubic flag 0x025d5f31,
    /// points 0x025d5f4c, count 0x025d5f50, its node block 0x025d5f54,
    /// control points 0x025d5f58), the camera handle (0x025d5ee0); f32
    /// 0.001.
    pub const CT_ON: u32 = 0x0193_c378;
    pub const CT_MODE: u32 = 0x0193_c37c;
    pub const CT_TIME: u32 = 0x0193_c380;
    pub const CT_STEP: u32 = 0x0193_c384;
    pub const NEAR_X: u32 = 0x0193_c174;
    pub const NEAR_Y: u32 = 0x0193_c388;
    pub const NEAR_Z: u32 = 0x0193_c38c;
    pub const CT_HOLDER: u32 = 0x025d_5f30;
    pub const SPLINE_CUBIC: u32 = 0x025d_5f31;
    pub const SPLINE_POINTS: u32 = 0x025d_5f4c;
    pub const SPLINE_COUNT: u32 = 0x025d_5f50;
    pub const SPLINE_NODES: u32 = 0x025d_5f54;
    pub const SPLINE_CONTROL: u32 = 0x025d_5f58;
    pub const CAMERA: u32 = 0x025d_5ee0;
    pub const STEP: u32 = 0x00b3_68a4;
    /// The window's title and buttons ("PLAY", "PLAY2", "PLAY3", "SPLINE
    /// CAM"), "no chr !!!!".
    pub const CT_TITLE: u32 = 0x00b6_8fec;
    pub const CT_BUTTONS: [u32; 4] = [0x00b6_8ff8, 0x00b6_9000, 0x00b6_9008, 0x00b6_9010];
    pub const NO_CHR: u32 = 0x00b6_901c;
    /// `Gui_CameraTestProc_0093f900`'s frame.
    pub const CAMERA_TEST_FRAME: u32 = 0x40;

    fn g(h: &impl Host, sym: u32) -> u32 {
        h.read(h.rebase(sym))
    }

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// The controlled actor's position into the frame (`ebp - 0x28`,
    /// `- 0x30`, `- 0x2c`: x, y, z) and the spline's node block reset
    /// (its header four words before it: the points, 0, 0, 0); `None`
    /// without an actor (the "no chr" print).
    fn spline_start(h: &mut impl Host, ebp: u32) -> Option<(u32, [u32; 3], u32)> {
        let id = cdecl(h, NOW_CTRL_TOP_ID, &[]);
        if id == 0xffff {
            let s = h.rebase(NO_CHR);
            cdecl(h, DBG_PRINTF, &[s]);
            return None;
        }
        let (x, y, z) = (ebp - 0x28, ebp - 0x30, ebp - 0x2c);
        cdecl(h, EVENT_ACTOR_POS, &[id, x, y, z]);
        let points = g(h, SPLINE_POINTS);
        h.write(g(h, SPLINE_NODES).wrapping_sub(0x10), points);
        for o in [0xc, 8, 4] {
            h.write(g(h, SPLINE_NODES).wrapping_sub(o), 0_u32);
        }
        // fld dword / fstp dword of each coordinate onto the stack
        let pos = [x, y, z].map(|p| x87::store(x87::load(h.read(p))));
        let cubic = i32::from(h.read::<i8>(h.rebase(SPLINE_CUBIC))).cast_unsigned();
        Some((g(h, SPLINE_NODES).wrapping_sub(0x10), pos, cubic))
    }

    /// `Gui_CameraTestProc_0093f900(window, msg, param)` -> 0: the camera
    /// test window. 0x8001 its title and four buttons, the test off;
    /// 0x8003 (closed) clears its holder byte; 0x8006 buttons 1..3 start
    /// a path player (mode 0..2, time 0, step 0.001), 4 the spline camera
    /// (mode 3, with a spline: its nearest point to the controlled actor
    /// initialised); 0x8011 while on runs the mode's player, or for the
    /// spline camera the nearest point on the spline (copied to
    /// 0x0193c174/88/8c and the frame), the spline evaluated there
    /// (`TkMtCalcSpline` or `TkMtCalcSpline3` by the cubic flag) and the
    /// camera's roll, target and position set. `ebp` is the original's
    /// frame (the actor at `ebp - 0x30..`, the near point's outs at `-
    /// 0x40..`, the spline result at `- 0x24`, the target at `- 0x14`).
    #[allow(clippy::too_many_lines)]
    pub fn camera_test_proc(h: &mut impl Host, ebp: u32, window: u32, msg: u32, param: u32) -> u32 {
        let fr = |o: u32| ebp.wrapping_sub(o);
        let have_spline = |h: &mut _| g(h, SPLINE_POINTS) != 0 && g(h, SPLINE_COUNT) != 0;
        match msg {
            0x8001 => {
                let t = h.rebase(CT_TITLE);
                cdecl(h, SET_TITLE, &[window, t]);
                for (id, s, y, w) in [
                    (1, CT_BUTTONS[0], 2, 0x40),
                    (2, CT_BUTTONS[1], 0xe, 0x46),
                    (3, CT_BUTTONS[2], 0x1a, 0x46),
                    (4, CT_BUTTONS[3], 0x26, 100),
                ] {
                    let s = h.rebase(s);
                    cdecl(h, ADD_BUTTON, &[window, id, s, 8, y, w, 10]);
                }
                h.write(h.rebase(CT_ON), 0_u32);
            }
            0x8003 => h.write(h.rebase(CT_HOLDER), 0_u8),
            0x8006 => match param {
                1..=3 => {
                    h.write(h.rebase(CT_ON), 1_u32);
                    h.write(h.rebase(CT_TIME), 0_u32);
                    h.write(h.rebase(CT_MODE), param - 1);
                    let step: u32 = g(h, STEP);
                    h.write(h.rebase(CT_STEP), x87::store(x87::load(step)));
                }
                4 if have_spline(h) => {
                    h.write(h.rebase(CT_ON), 1_u32);
                    h.write(h.rebase(CT_MODE), 3_u32);
                    if let Some((spline, [x, y, z], cubic)) = spline_start(h, ebp) {
                        cdecl(h, NEAR_POINT_INIT, &[spline, x, y, z, cubic]);
                    }
                }
                _ => {}
            },
            0x8011 if g(h, CT_ON) != 0 => match g(h, CT_MODE) {
                m @ 0..=2 => {
                    cdecl(h, PATH_PLAYERS[m as usize], &[]);
                }
                3 if have_spline(h) => {
                    let Some((spline, [x, y, z], cubic)) = spline_start(h, ebp) else {
                        return 0;
                    };
                    let (px, py, pz) = (fr(0x38), fr(0x3c), fr(0x40));
                    let t = x87::store(h.call_sym_st0_f64(
                        NEAR_POINT,
                        Conv::Cdecl,
                        &[spline, x, y, z, px, py, pz, cubic],
                    ));
                    h.write(fr(0x34), t);
                    for (src, global, dst) in
                        [(px, NEAR_X, 0x14), (py, NEAR_Y, 0x10), (pz, NEAR_Z, 0xc)]
                    {
                        let v = x87::store(x87::load(h.read(src)));
                        h.write(h.rebase(global), v);
                        h.write(fr(dst), v);
                    }
                    let f = if h.read::<u8>(h.rebase(SPLINE_CUBIC)) == 0 {
                        CALC_SPLINE
                    } else {
                        CALC_SPLINE3
                    };
                    let (ctl, n) = (g(h, SPLINE_CONTROL), g(h, SPLINE_COUNT));
                    let t = x87::store(x87::load(t));
                    cdecl(h, f, &[fr(0x24), ctl, n, t]);
                    let roll = x87::store(x87::load(h.read(fr(0x18))));
                    let cam = g(h, CAMERA);
                    cdecl(h, CAMERA_SET_ROLL, &[cam, roll]);
                    let cam = g(h, CAMERA);
                    cdecl(h, CAMERA_SET_RECT, &[cam, 0, fr(0x14)]);
                    let cam = g(h, CAMERA);
                    cdecl(h, CAMERA_SET_RECT, &[cam, 1, fr(0x24)]);
                }
                _ => {}
            },
            _ => {}
        }
        0
    }
}
