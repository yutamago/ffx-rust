//! `Snd3dFootRequest` (`0x00876950`): a positioned sound (the 3D
//! footsteps and the event sounds `AtelCalcSound`/`AtelExecSound` place)
//! turned into a volume and a pan for the separate-voice API.
//!
//! The PC port kept the PS2's VU0 code as C over the emulated registers
//! (`0x00c8f790` the accumulator, `0x00c8f890` the camera matrix,
//! `0x00c8f910`.. the work vectors); every one of those writes is
//! reproduced, in the original's order and x87 rounding.

// x, y, z, w, p, h, t as in the formulas; the x87 compares are exact
#![allow(clippy::many_single_char_names, clippy::float_cmp)]

/// The originals over game memory.
pub mod bridge {
    use ffx_bridge::x87::{self, Reg};
    use ffx_bridge::{Global, Mem};

    use crate::fmod::{Conv, FmodHost as Host};

    /// `g_AtelCameraMatrixFlag` (`0x01328308`): which camera matrix the
    /// VU0 matrix register holds (1 the one at [`CAMERA_MATRIX_4`], 0 the
    /// one at [`CAMERA_MATRIX`], -1 cleared).
    pub const CAMERA_MATRIX_FLAG: Global<u32> = Global::new(0x0132_8308);
    /// The camera matrices (16 words each) a request's flag 4 picks.
    pub const CAMERA_MATRIX_4: u32 = 0x0132_8288;
    pub const CAMERA_MATRIX: u32 = 0x0132_82c8;
    /// The start of the segment `Math_SegmentNearestT` measures from
    /// (`AtelInitCalcSound` copies vf00 there).
    pub const SEGMENT_START: u32 = 0x0132_8278;
    /// `g_AtelSnd3dVolumeTotalRatio` (`f32`).
    pub const VOLUME_RATIO: u32 = 0x0132_830c;
    /// The emulated VU0 registers: Q, the accumulator, vf00 (its `x`
    /// is added), the matrix and the work vectors (position, direction,
    /// squares).
    pub const VU0_Q: u32 = 0x00c8_f788;
    pub const VU0_ACC: u32 = 0x00c8_f790;
    pub const VU0_VF00_X: u32 = 0x00c0_a004;
    pub const VU0_MATRIX: u32 = 0x00c8_f890;
    pub const VU0_POS: u32 = 0x00c8_f910;
    pub const VU0_DIR: u32 = 0x00c8_f920;
    pub const VU0_SQUARES: u32 = 0x00c8_f930;

    /// `AtelGetEventActorPos(i, &x, &y, &z)`, `AtelGetEventActorType(i)`,
    /// `Atel_GetEventActorRot_0086b270(i, &x, &y, &z)`.
    pub const ACTOR_POS: u32 = 0x0086_b1e0;
    pub const ACTOR_TYPE: u32 = 0x0086_b320;
    pub const ACTOR_ROT: u32 = 0x0086_b270;
    /// `pppVu0MaddaY(acc, m, v)`, `pppVu0MaddaZ(acc, m, v)`,
    /// `pppVu0MaddW(dst, m, v)` (`v` four words by value).
    pub const MADDA_Y: u32 = 0x0072_c590;
    pub const MADDA_Z: u32 = 0x0072_c5f0;
    pub const MADD_W: u32 = 0x0072_c650;
    /// `Math_SegmentNearestT(a, p, b)` and `Math_Atan2XZ(x0, z0, x1, z1)`
    /// (both in `ST0`).
    pub const SEGMENT_NEAREST_T: u32 = 0x0088_a040;
    pub const ATAN2_XZ: u32 = 0x0088_b430;
    /// `SndSepVolumeDirect(id, v)`, `SndSepPanDirect(id, pan)`,
    /// `SndSepPlay(id, pan, v)`, `graphicLevelGetNameHash()`,
    /// `getScenerioFlag()`.
    pub const SEP_VOLUME_DIRECT: u32 = 0x0088_6f60;
    pub const SEP_PAN_DIRECT: u32 = 0x0088_6df0;
    pub const SEP_PLAY: u32 = 0x0088_6e50;
    pub const LEVEL_NAME_HASH: u32 = 0x0064_1da0;
    pub const SCENARIO_FLAG: u32 = 0x0078_7360;

    /// `.rdata`: the angle folds (`f64`s of `f32` pi/2, -pi/2, pi, -pi),
    /// 127 and 2 pi for the pan; the three `x` positions of the special
    /// case.
    const HALF_PI: u32 = 0x00b1_1338;
    const MINUS_HALF_PI: u32 = 0x00b3_ebb0;
    const PI: u32 = 0x00b9_2288;
    const MINUS_PI: u32 = 0x00b5_c008;
    const K127: u32 = 0x00b9_22e8;
    const TWO_PI: u32 = 0x00b3_9378;
    const SPECIAL_X: [u32; 3] = [0x00b5_bffc, 0x00b5_c004, 0x00b5_c000];
    /// The level (`graphicLevelGetNameHash`), scenario flags and sound ids
    /// (exclusive bounds) of the special case.
    const SPECIAL_LEVEL: u32 = 0x76f;
    const SPECIAL_SCENARIO: [u16; 2] = [2000, 0x7c6];
    const SPECIAL_IDS: (u32, u32) = (0x0001_1df8, 0x0001_1dfd);

    /// The frame of [`snd3d_foot_request`], laid out as the original's
    /// (`ebp - 0x24` on): `+0` the direction (`x, y, z`, `+0xc` 1.0 once
    /// written), `+0x10` the position (`x, y, z`, `+0x1c` its `w`, 1.0).
    pub const WORK: u32 = 0x20;

    fn rd(mem: &impl Mem, at: u32) -> u32 {
        mem.read(mem.rebase(at))
    }

    fn rd_f32(mem: &impl Mem, at: u32) -> f32 {
        f32::from_bits(rd(mem, at))
    }

    fn rdata_f64(mem: &impl Mem, at: u32) -> f64 {
        mem.read(mem.rebase(at))
    }

    fn wr(mem: &mut impl Mem, at: u32, v: u32) {
        mem.write(mem.rebase(at), v);
    }

    fn wr_f32(mem: &mut impl Mem, at: u32, v: f32) {
        wr(mem, at, v.to_bits());
    }

    fn call(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// `ax` of a returned `eax`.
    fn lo16(v: u32) -> u16 {
        let [a, b, _, _] = v.to_le_bytes();
        u16::from_le_bytes([a, b])
    }

    /// Element `i` of the VU0 matrix register.
    fn m(mem: &impl Mem, i: u32) -> f32 {
        rd_f32(mem, VU0_MATRIX + 4 * i)
    }

    /// Loads the camera matrix the request asks for into the VU0 matrix
    /// register, unless [`CAMERA_MATRIX_FLAG`] says it is there already.
    fn select_camera(mem: &mut impl Mem, flag4: bool) {
        let (want, src) = if flag4 {
            (1, CAMERA_MATRIX_4)
        } else {
            (0, CAMERA_MATRIX)
        };
        if CAMERA_MATRIX_FLAG.get(mem) != want {
            for i in 0..16 {
                let w = rd(mem, src + 4 * i);
                wr(mem, VU0_MATRIX + 4 * i, w);
            }
        }
        CAMERA_MATRIX_FLAG.set(mem, want);
    }

    /// The position through the camera matrix: row 0 times `x` from the
    /// registers into the accumulator, rows 1 and 2 times `y` and `z`
    /// added there, row 3 times `w` (1.0) added into [`VU0_POS`]. Returns
    /// row 0, which the direction's transform reuses from the registers.
    fn transform_position(mem: &mut impl Mem, work: u32) -> [f32; 4] {
        let [x, y, z]: [u32; 3] = mem.read(work + 0x10);
        let w: u32 = mem.read(work + 0x1c);
        for (i, v) in (0..).zip([x, y, z, w]) {
            wr(mem, VU0_POS + 4 * i, v);
        }
        let row0 = [m(mem, 0), m(mem, 1), m(mem, 2), m(mem, 3)];
        let x = Reg::load(f32::from_bits(x));
        for (i, &r) in (0..).zip(&row0) {
            wr_f32(mem, VU0_ACC + 4 * i, (x * Reg::load(r)).store());
        }
        for (row, v) in [(1, y), (2, z)] {
            let v = Reg::load(f32::from_bits(v));
            for i in 0..4 {
                let acc = rd_f32(mem, VU0_ACC + 4 * i);
                let sum = v.mul_m32(m(mem, 4 * row + i)).add_m32(acc);
                wr_f32(mem, VU0_ACC + 4 * i, sum.store());
            }
        }
        let w = Reg::load(f32::from_bits(w));
        for i in 0..4 {
            let acc = rd_f32(mem, VU0_ACC + 4 * i);
            let sum = w.mul_m32(m(mem, 12 + i)).add_m32(acc);
            wr_f32(mem, VU0_POS + 4 * i, sum.store());
        }
        row0
    }

    /// The direction through the camera matrix (flag 1): its `x` times
    /// row 0 into the accumulator, then `pppVu0MaddaY`, `pppVu0MaddaZ`
    /// and `pppVu0MaddW` into [`VU0_DIR`]; position and direction back
    /// into `work`; returns `Math_SegmentNearestT(SEGMENT_START, position,
    /// direction)` as an `f32`.
    fn transform_direction(h: &mut impl Host, work: u32, row0: [f32; 4]) -> f32 {
        let [dx, dy, dz]: [u32; 3] = h.read(work);
        h.write(work + 0xc, 1f32.to_bits());
        let x = Reg::load(f32::from_bits(dx));
        for (i, &r) in (0..).zip(&row0) {
            wr_f32(h, VU0_ACC + 4 * i, (x * Reg::load(r)).store());
        }
        let one = 1f32.to_bits();
        for (i, v) in (0..).zip([dx, dy, dz, one]) {
            wr(h, VU0_DIR + 4 * i, v);
        }
        let (acc, dir) = (h.rebase(VU0_ACC), h.rebase(VU0_DIR));
        let row = h.rebase(VU0_MATRIX + 0x10);
        call(h, MADDA_Y, &[acc, row, dx, dy, dz, one]);
        for (f, dst, row) in [(MADDA_Z, acc, 0x20), (MADD_W, dir, 0x30)] {
            let v: [u32; 4] = h.read(dir);
            let row = h.rebase(VU0_MATRIX + row);
            call(h, f, &[dst, row, v[0], v[1], v[2], v[3]]);
        }
        let pos: [u32; 4] = h.read(h.rebase(VU0_POS));
        h.write(work + 0x10, pos);
        let d: [u32; 4] = h.read(dir);
        h.write(work, d);
        let start = h.rebase(SEGMENT_START);
        h.call_sym_st0(SEGMENT_NEAREST_T, Conv::Cdecl, &[start, work + 0x10, work])
    }

    /// The angle folded into [-pi/2, pi/2] (the `f64` constants are `f32`
    /// pi and pi/2): above pi/2 to `pi - a`, below -pi/2 to `-pi - a`,
    /// each stored as an `f32`.
    fn fold(mem: &impl Mem, a: f32) -> f32 {
        let mut a = a;
        let pi = rdata_f64(mem, PI);
        if f64::from(a) > rdata_f64(mem, HALF_PI) {
            a = (Reg::constant(pi) - Reg::load(a)).store();
        }
        if f64::from(a) < rdata_f64(mem, MINUS_HALF_PI) {
            a = (Reg::constant(rdata_f64(mem, MINUS_PI)) - Reg::load(a)).store();
        }
        a
    }

    /// Where `dist` lies between `near` and `far`, as an `f32` in [0, 1]:
    /// 0 up to `near`, 1 from `far` on or when they are equal, else the
    /// ratio clamped; unordered compares fall through to the ratio, and a
    /// NaN ratio is kept.
    fn ratio(dist: f32, near: f32, far: f32) -> f32 {
        if near >= dist {
            return 0.0;
        }
        if far <= dist || near == far {
            return 1.0;
        }
        let q = (Reg::load(dist).sub_m32(near) / Reg::load(far).sub_m32(near)).store();
        q.clamp(0.0, 1.0)
    }

    /// The volume for `dist`: byte `+3` (as an `f32`) nearer than `+8`,
    /// byte `+2` from `+0xc` on or when the two are equal, else the bytes
    /// mixed by where `dist` lies (`(1 - q) b3 + q b2`, stored as an `f32`);
    /// times [`VOLUME_RATIO`], `ftol`, and only the mix clamped to 0..=127.
    fn volume(mem: &impl Mem, p: u32, dist: f32) -> u32 {
        let (near, far): (f32, f32) = (mem.read(p + 8), mem.read(p + 0xc));
        let byte = |o: u32| f32::from(mem.read::<u8>(p + o));
        let ratio_m = rd_f32(mem, VOLUME_RATIO);
        let fixed = if near > dist {
            Some(byte(3))
        } else if far < dist || near == far {
            Some(byte(2))
        } else {
            None
        };
        if let Some(v) = fixed {
            return x87::ftol(mem, Reg::load(v).mul_m32(ratio_m).value());
        }
        let q = (Reg::load(dist).sub_m32(near) / Reg::load(far).sub_m32(near)).store();
        let q = Reg::load(q);
        let mix = (Reg::ONE - q) * Reg::constant(f64::from(byte(3)))
            + q * Reg::constant(f64::from(byte(2)));
        let v = x87::ftol(mem, Reg::load(mix.store()).mul_m32(ratio_m).value());
        v.cast_signed().clamp(0, 0x7f).cast_unsigned()
    }

    /// `Snd3dFootRequest(req, play, count)` (`0x00876950`, from
    /// `SndKickFoot`, `AtelCalcSound`, `AtelExecSound`, `0x008682f0`):
    /// nothing without a sound id (`req +0`) or parameters (`req +4`:
    /// `u16` flags, bytes `+2`/`+3` the far and near volumes, `f32` the
    /// pan scale `+4`, the volume's near/far distances `+8`/`+0xc`, the
    /// pan's `+0x10`/`+0x14`). The camera matrix by flag 4; the position
    /// from `req +8` (flag 2: event actor `u16 req +8`'s, its rotation as
    /// the direction for actor types 2 and 5, which sets flag 1) or
    /// `+8..+0x10` (and the direction `+0x14..+0x1c` with flag 1); both
    /// into the camera's space ([`transform_position`]), the position
    /// moved toward the direction's nearest point by `t`
    /// ([`transform_direction`]; 0 without flag 1); the distance (without
    /// `y` for flag 0x8000) through `_CIsqrt` into Q, the angle
    /// `Math_Atan2XZ(0, 0, z, x)` folded ([`fold`]) times the pan scale
    /// and [`ratio`] of the pan distances, the pan `ftol((pi + angle) *
    /// 127 / 2 pi)` clamped to 0..=127, the [`volume`] (0 for `count` up to
    /// 2). Then `SndSepVolumeDirect` and `SndSepPanDirect` (`play` 0) or
    /// `SndSepPlay` (in level 0x76f at scenario 2000 or 0x7c6, sounds
    /// 0x11df9..=0x11dfc only from three exact `x` positions), and the pan
    /// and volume into bytes `req +0x20`/`+0x21`. `work`: the original's
    /// frame ([`WORK`]); with flag 2 for an actor of another type and flag
    /// 1 in the request the direction is what `work` held (the original
    /// reads its uninitialised frame). Returns the pan (the original's
    /// `eax`; 0 when it does nothing).
    #[allow(clippy::too_many_lines)] // one request, in the original's order
    pub fn snd3d_foot_request(
        h: &mut impl Host,
        req: u32,
        play: u32,
        count: i32,
        work: u32,
    ) -> u32 {
        let id: u32 = h.read(req);
        let p: u32 = h.read(req + 4);
        if id == 0 || p == 0 {
            return 0;
        }
        h.write(work + 0x1c, 1f32.to_bits());
        let mut flags = u32::from(h.read::<u16>(p));
        select_camera(h, flags & 4 != 0);
        if flags & 2 == 0 {
            for i in 0..3 {
                let v: f32 = h.read(req + 8 + 4 * i);
                h.write(work + 0x10 + 4 * i, x87::round_trip(v));
            }
            if flags & 1 != 0 {
                for i in 0..3 {
                    let v: f32 = h.read(req + 0x14 + 4 * i);
                    h.write(work + 4 * i, x87::round_trip(v));
                }
            }
        } else {
            let actor = u32::from(h.read::<u16>(req + 8));
            call(
                h,
                ACTOR_POS,
                &[actor, work + 0x10, work + 0x14, work + 0x18],
            );
            let kind = call(h, ACTOR_TYPE, &[actor]);
            if kind == 2 || kind == 5 {
                flags |= 1;
                call(h, ACTOR_ROT, &[actor, work, work + 4, work + 8]);
            }
        }
        let row0 = transform_position(h, work);
        let t = if flags & 1 == 0 {
            let v = rd_f32(h, VU0_VF00_X);
            for i in 0..4 {
                let p = rd_f32(h, VU0_POS + 4 * i);
                wr_f32(h, VU0_DIR + 4 * i, (Reg::load(p) + Reg::load(v)).store());
            }
            0.0
        } else {
            transform_direction(h, work, row0)
        };
        // the position moved by t toward the direction's point
        let pos: [f32; 4] = h.read(h.rebase(VU0_POS));
        let u = (Reg::ONE - Reg::load(t)).store();
        wr_f32(h, VU0_SQUARES, u);
        wr_f32(h, VU0_SQUARES + 4, t);
        let (u, t) = (Reg::load(u), Reg::load(t));
        for (i, &c) in (0..).zip(&pos[..3]) {
            wr_f32(h, VU0_ACC + 4 * i, (Reg::load(c) * u).store());
        }
        for i in 0..2 {
            let d = rd_f32(h, VU0_DIR + 4 * i);
            let acc = rd_f32(h, VU0_ACC + 4 * i);
            wr_f32(h, VU0_POS + 4 * i, (Reg::load(d) * t).add_m32(acc).store());
        }
        let d2 = rd_f32(h, VU0_DIR + 8);
        let acc2 = rd_f32(h, VU0_ACC + 8);
        wr_f32(h, VU0_POS + 8, t.mul_m32(d2).add_m32(acc2).store());
        let moved: [u32; 4] = h.read(h.rebase(VU0_POS));
        h.write(work + 0x10, moved);
        // the squares, the distance
        for i in 0..3 {
            let c = Reg::load(rd_f32(h, VU0_POS + 4 * i));
            wr_f32(h, VU0_SQUARES + 4 * i, (c * c).store());
        }
        let w = Reg::load(pos[3]);
        wr_f32(h, VU0_SQUARES + 12, (w * w).store());
        let terms: &[u32] = if flags & 0x8000 == 0 { &[4, 8] } else { &[8] };
        for &o in terms {
            let sq = rd_f32(h, VU0_SQUARES + o);
            let sum = Reg::load(sq).add_m32(rd_f32(h, VU0_SQUARES));
            wr_f32(h, VU0_SQUARES, sum.store());
        }
        let s = rd_f32(h, VU0_SQUARES).abs();
        let q = Reg::load(s).sqrt().store();
        wr_f32(h, VU0_Q, q);
        let dist = Reg::load(rd_f32(h, VU0_VF00_X)).add_m32(q).store();
        wr_f32(h, VU0_SQUARES, dist);
        // the pan
        let [x, _, z, _]: [f32; 4] = h.read(work + 0x10);
        let (x, z) = (x87::round_trip(x), x87::round_trip(z));
        let angle = h.call_sym_st0(ATAN2_XZ, Conv::Cdecl, &[0, 0, z.to_bits(), x.to_bits()]);
        let angle = fold(h, angle);
        let (pan_near, pan_far): (f32, f32) = (h.read(p + 0x10), h.read(p + 0x14));
        let r = ratio(dist, pan_near, pan_far);
        let scale = Reg::load(h.read(p + 4)).mul_m32(r).store();
        let angle = (Reg::load(angle) * Reg::load(scale)).store();
        let vol = volume(h, p, dist);
        let turn = Reg::constant(rdata_f64(h, PI)).add_m32(angle);
        let turn = turn * Reg::constant(rdata_f64(h, K127)) / Reg::constant(rdata_f64(h, TWO_PI));
        let pan = x87::ftol(h, turn.value())
            .cast_signed()
            .clamp(0, 0x7f)
            .cast_unsigned();
        let vol = if count <= 2 { 0 } else { vol };
        if play == 0 {
            let id: u32 = h.read(req);
            call(h, SEP_VOLUME_DIRECT, &[id, vol]);
            let id: u32 = h.read(req);
            call(h, SEP_PAN_DIRECT, &[id, pan]);
        } else {
            let special = call(h, LEVEL_NAME_HASH, &[]) == SPECIAL_LEVEL
                && SPECIAL_SCENARIO
                    .iter()
                    .any(|&s| lo16(call(h, SCENARIO_FLAG, &[])) == s);
            let id: u32 = h.read(req);
            let plays = if special && SPECIAL_IDS.0 < id && id < SPECIAL_IDS.1 {
                let x: f32 = h.read(work + 0x10);
                SPECIAL_X.iter().any(|&c| rd_f32(h, c) == x)
            } else {
                true
            };
            if plays {
                call(h, SEP_PLAY, &[id, pan, vol]);
            }
        }
        h.write(req + 0x20, pan.to_le_bytes()[0]);
        h.write(req + 0x21, vol.to_le_bytes()[0]);
        pan
    }
}
