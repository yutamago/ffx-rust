//! The cloth step (`0x00822de0`): each cloth strand of a character's
//! models moved by the wind and the character's own motion, held by its
//! anchors and rods, pushed out of the collision shapes and the floor,
//! then written back to the model's vertices.
//!
//! The original keeps its vectors and matrices on its stack and hands
//! their addresses to the vector and matrix helpers; the bridge keeps them
//! in a [`Frame`](ffx_bridge::frame::Frame) and calls the same helpers'
//! bridges. All x87 at 53 bits, each value stored as `f32` where the
//! original stores it.

/// The original, over game memory.
#[allow(clippy::many_single_char_names)] // vectors and their components
pub mod bridge {
    use ffx_bridge::frame::Frame;
    use ffx_bridge::x87::{Reg, round_trip};
    use ffx_bridge::{Global, Host, Mem};
    use ffx_ps2::vu0::bridge as vu0;
    use ffx_system::math::bridge as mx;

    use crate::quat::Crt;
    use crate::vec::bridge as v;

    /// The wind: strength, heading, elevation (`Sg_GlobalSetWind`'s
    /// globals, read as three floats).
    pub const WIND: Global<[f32; 3]> = Global::new(0x012f_bc44);
    /// The current map number.
    pub const MAP: Global<u32> = Global::new(0x012f_bc40);
    /// 399 (`f32`): the wind strength above which the field's 0x15d/0x84
    /// cloth stays still early in the story.
    pub const WIND_STILL: Global<f32> = Global::new(0x00b5_91a8);
    /// 2000 (`f32`): the wind strength on map 0xc0.
    pub const WIND_MAP_C0: Global<f32> = Global::new(0x00b4_5bb8);
    /// 17 (`f32`): the reach in battle.
    pub const REACH_BATTLE: Global<f32> = Global::new(0x00b4_a64c);
    /// 0.1 (`f64`): below this the character counts as still.
    pub const STILL: Global<f64> = Global::new(0x00b9_21f0);
    /// The reach when still, `+0x820` bit 1 set / clear (0.15, 0.3).
    pub const REACH_STILL: [Global<f32>; 2] = [Global::new(0x00b4_7c60), Global::new(0x00b3_eb44)];
    /// The reach when moving, bit 1 set / clear (1.7, 3.4).
    pub const REACH_MOVING: [Global<f32>; 2] = [Global::new(0x00b5_918c), Global::new(0x00b5_9190)];
    /// 1000 (`f64`): bone translations are scaled by it around the matrix
    /// products.
    pub const THOUSAND: Global<f64> = Global::new(0x00b0_dba0);
    /// 1.7 (`f64`): the follow distance's unit.
    pub const FOLLOW_UNIT: Global<f64> = Global::new(0x00b5_83a0);
    /// 100 (`f64`): the lift of a strand far from its group.
    pub const FAR_LIFT: Global<f64> = Global::new(0x00b9_22e0);
    /// 2700 and 1600 (`f64`): the lifts of map 0x107's two points.
    pub const LIFT_107: [Global<f64>; 2] = [Global::new(0x00b5_91a0), Global::new(0x00b5_9198)];
    /// 1.2 (`f64`): the push out of a plane or the floor.
    pub const PUSH: Global<f64> = Global::new(0x00b4_8340);
    /// The C runtime's `rand` (import slot).
    pub const RAND_SLOT: u32 = 0x00b0_c420;
    /// `getScenerioFlag()`.
    pub const GET_SCENERIO_FLAG: u32 = 0x0078_7360;
    /// `MsBattleCheck()`.
    pub const MS_BATTLE_CHECK: u32 = 0x0078_0c90;
    /// `MagicFile_GetMagicId()`.
    pub const MAGIC_FILE_GET_MAGIC_ID: u32 = 0x009d_a3b0;

    const CLOTH_FRAME: u32 = 0xfff8_0000;
    // the locals
    const WINDV: u32 = 0x00;
    const PU: u32 = 0x10;
    const FV: u32 = 0x20;
    const P0: u32 = 0x30;
    const NV: u32 = 0x40;
    const QV: u32 = 0x50;
    const MM: u32 = 0x60;
    const MINV: u32 = 0xa0;
    const WM: u32 = 0xe0;
    const BINV: u32 = 0x120;
    const TV: u32 = 0x160;
    const RECA: u32 = 0x170;
    const RECB: u32 = 0x180;
    const LOC: u32 = 0x190;
    const AV: u32 = 0x1a0;
    const DOT: u32 = 0x1b0;
    const DOT2: u32 = 0x1b4;
    const FRAME_LEN: usize = 0x1c0;

    fn ld(x: f32) -> Reg {
        Reg::load(x)
    }

    fn g32(m: &impl Mem, a: u32) -> f32 {
        m.read(a)
    }

    fn put(m: &mut impl Mem, a: u32, x: f32) {
        m.write(a, x);
    }

    fn copy16(m: &mut impl Mem, dst: u32, src: u32) {
        let w: [u32; 4] = m.read(src);
        m.write(dst, w);
    }

    /// A bone's translation (`+0xb8..0xc4` of its 0x160 bytes, the
    /// matrix at `+0x88`) times 1000 (`up`) or over 1000.
    fn scale_bone(m: &mut impl Mem, bone: u32, k: Reg, up: bool) {
        for o in [0xb8, 0xbc, 0xc0] {
            let x = ld(g32(m, bone + o));
            put(m, bone + o, if up { x * k } else { x / k }.store());
        }
    }

    /// `FFXVu0MulMatrix(dst, a, bone)` with the bone's translation in
    /// thousandths.
    fn mul_bone(m: &mut impl Mem, dst: u32, a: u32, bone: u32, k: Reg) {
        scale_bone(m, bone, k, true);
        mx::mul4x4(m, dst, a, bone + 0x88);
        scale_bone(m, bone, k, false);
    }

    fn bone_at(m: &impl Mem, chr: u32, b: i32) -> u32 {
        #[allow(clippy::cast_sign_loss)] // two's complement offsets
        let o = b.wrapping_mul(0x160) as u32;
        m.read::<u32>(chr + 0x328).wrapping_add(o)
    }

    fn i16_at(m: &impl Mem, a: u32) -> i32 {
        i32::from(m.read::<i16>(a))
    }

    #[allow(clippy::cast_sign_loss)]
    fn off(i: i32, size: i32) -> u32 {
        i.wrapping_mul(size) as u32
    }

    /// The floor (`P0` the point, `N` the normal): a point below it is
    /// pushed back up by 1.2 times its depth.
    fn floor_push(f: &mut impl Mem, p: u32) {
        let (p0, n) = (CLOTH_FRAME + P0, CLOTH_FRAME + NV);
        let t = CLOTH_FRAME + TV;
        for k in 0..3 {
            let d = ld(g32(f, p + 4 * k)).sub_m32(g32(f, p0 + 4 * k)).store();
            put(f, t + 4 * k, d);
        }
        let [dx, dy, dz] = [0, 1, 2].map(|k| g32(f, t + 4 * k));
        let [nx, ny, nz] = [0, 1, 2].map(|k| g32(f, n + 4 * k));
        let dd = ((ld(dx) * ld(nx) + ld(dy).mul_m32(ny)) + ld(dz).mul_m32(nz)).store();
        if 0.0 > ld(dd).value() {
            let c = ld((ld(dd) * Reg::constant(PUSH.get(f))).store());
            let x = (ld(g32(f, p)) - ld(nx) * c).store();
            put(f, p, x);
            let y = (ld(g32(f, p + 4)) - ld(ny) * c).store();
            put(f, p + 4, y);
            let z = (ld(nz) * c).subr_m32(g32(f, p + 8)).store();
            put(f, p + 8, z);
        }
    }

    /// A point done: `w = 1`, copied to the model vertex `idx` points at,
    /// its velocity (`stride` bytes on) the damped change from the
    /// previous position (`2 stride` on), which becomes the point.
    fn settle(f: &mut impl Mem, p: u32, idx: u32, verts: u32, stride: u32, damp: Reg) {
        put(f, p + 12, 1.0);
        let i = u32::from(f.read::<u16>(idx));
        copy16(f, verts.wrapping_add(16 * i), p);
        let (vel, prev) = (p.wrapping_add(stride), p.wrapping_add(2 * stride));
        for k in 0..3 {
            let x = (ld(g32(f, p + 4 * k)).sub_m32(g32(f, prev + 4 * k)) * damp).store();
            put(f, vel + 4 * k, x);
        }
        copy16(f, prev, p);
    }

    /// The collision shapes of strand `s` against point `p`: spheres
    /// (kind 0), capsules (2) and planes (1), their records at
    /// `[chr+0x1c0]+0x30` (radius at `+0`, reach `+4`, kind `+8`, axis
    /// scale `+0xc`), their positions and axes at `[chr+0x32c]` (0x60
    /// each).
    fn collide(f: &mut impl Mem, chr: u32, s: u32, p: u32, g: u32, k: u32) {
        let (t, a) = (CLOTH_FRAME + TV, CLOTH_FRAME + AV);
        let (dot, dot2) = (CLOTH_FRAME + DOT, CLOTH_FRAME + DOT2);
        let mut c = 0;
        while c < i32::from(f.read::<i8>(s + 0x12)) {
            let list: u32 = f.read(s + 0x14);
            let ci = i16_at(f, list.wrapping_add(off(c, 2)));
            let model: u32 = f.read(chr + 0x1c0);
            let rec = f.read::<u32>(model + 0x30).wrapping_add(off(ci, 0x40));
            let cp = f.read::<u32>(chr + 0x32c).wrapping_add(off(ci, 0x60));
            let r = ld(round_trip(
                ffx_chara::cloth::bridge::ch_use_large_collision_rad(f, chr, g, k),
            ));
            let rad = (ld(g32(f, rec)) * r).store();
            let reach = (r * (ld(g32(f, rec + 4)) * r)).store();
            match f.read::<u16>(rec + 8) {
                0 => {
                    v::sub(f, t, p, cp);
                    v::dot(f, t, t, dot);
                    let dd = g32(f, dot);
                    if ld(reach).value() > ld(dd).value() {
                        let l = ld(ld(dd).sqrt().store());
                        for q in 0..3 {
                            let x = (ld(g32(f, t + 4 * q)) / l).store();
                            put(f, t + 4 * q, x);
                        }
                        let e = l.subr_m32(rad);
                        let x = (ld(g32(f, t)) * e).store();
                        put(f, t, x);
                        let y = (ld(g32(f, t + 4)) * e).store();
                        put(f, t + 4, y);
                        let z = e.mul_m32(g32(f, t + 8)).store();
                        put(f, t + 8, z);
                        for q in 0..3 {
                            let x = ld(g32(f, p + 4 * q)).add_m32(g32(f, t + 4 * q)).store();
                            put(f, p + 4 * q, x);
                        }
                    }
                }
                2 => {
                    v::sub(f, a, p, cp);
                    v::dot(f, a, cp + 0x10, dot2);
                    let along = g32(f, dot2);
                    v::scale(f, t, cp + 0x10, round_trip(along));
                    v::sub(f, t, a, t);
                    v::dot(f, t, t, dot);
                    let side = g32(f, dot);
                    let along = ld(g32(f, rec + 0xc)).mul_m32(along).store();
                    let dd = (ld(along) * ld(along)).add_m32(side).store();
                    if ld(reach).value() > ld(dd).value() {
                        let l = ld(dd).sqrt().store();
                        let q = (ld(rad) / ld(l)).store();
                        v::scale(f, t, a, q);
                        v::add(f, p, cp, t);
                    }
                }
                1 => {
                    v::sub(f, t, p, cp);
                    v::dot(f, t, cp + 0x10, dot);
                    let d = g32(f, dot);
                    if 0.0 > ld(d).value() {
                        let q = (ld(d) * Reg::constant(PUSH.get(f))).store();
                        v::scale(f, t, cp + 0x10, q);
                        v::sub(f, p, p, t);
                    }
                }
                _ => {}
            }
            c += 1;
        }
    }

    /// `Sg_ClothStep(chr)` (`0x00822de0`): see the module. Runs when
    /// `+0x194` has bits 21 and 22.
    ///
    /// The step repeats `iters + 1` times (`iters` from `+0x812`, doubled
    /// for characters 6 and 7 on maps 0x15d and 0x84), over every model
    /// group with strands; afterwards `+0x194` bit 23 and `+0x810` are
    /// cleared. `crt` gives `_CIsin`/`_CIcos` for the wind direction
    /// (`Sg_CreateVectorFromHV`). `Ch_ShouldDisableClothForMotion`, whose
    /// answer the original drops, is not called.
    #[allow(clippy::too_many_lines, clippy::float_cmp)]
    pub fn cloth_step(h: &mut impl Host, chr: u32, crt: &mut impl FnMut(Crt, f64) -> f32) {
        let fl: u32 = h.read(chr + 0x194);
        if fl & 0x20_0000 == 0 || fl & 0x40_0000 == 0 {
            return;
        }
        let scale = round_trip(g32(h, h.read::<u32>(chr + 0x1c4) + 0x3c));
        let f820: u32 = h.read(chr + 0x820);
        if f820 & 8 != 0 {
            h.write(chr + 0x820, f820 & !4);
        }
        let floor = h.read::<u32>(chr + 0x820) >> 3 & 1 != 0;
        let (mut wind, hd, el) = if h.read::<u8>(chr + 0x198) & 2 != 0 {
            (
                g32(h, chr + 0x80c),
                g32(h, chr + 0x804),
                g32(h, chr + 0x808),
            )
        } else {
            let w = WIND.get(h);
            (w[0], w[1], w[2])
        };
        wind = ld(wind).mul_m32(scale).store();
        let mut f = Frame::new(h, CLOTH_FRAME, FRAME_LEN);
        let at = |o: u32| CLOTH_FRAME + o;
        crate::quat::bridge::create_vector_from_hv(
            &mut f,
            at(WINDV),
            round_trip(hd),
            round_trip(el),
            crt,
        );
        let id: u16 = f.read(chr);
        let map = MAP.get(&f);
        if id != 0x4001 && id != 1 && (map == 0x15d || map == 0x84) {
            let fa = f.rebase(GET_SCENERIO_FLAG);
            #[allow(clippy::cast_possible_truncation)] // ax
            let sf = f.inner().call(fa, &[]) as u16;
            if sf < 2 && ld(WIND_STILL.get(&f)).value() < ld(wind).value() {
                return;
            }
        }
        if MAP.get(&f) == 0xc0 {
            wind = round_trip(WIND_MAP_C0.get(&f));
        }
        let ma = f.rebase(MAGIC_FILE_GET_MAGIC_ID);
        if f.inner().call(ma, &[]) == 0x1b4 && (id == 0x1120 || id == 0x1154) {
            let motion: u32 = f.read(chr + 0x72c);
            if motion == 0x3008_101a || motion == 0x3008_101b {
                return;
            }
        }
        let mut ka = round_trip(g32(&f, chr + 0x814));
        let mut kb = round_trip(g32(&f, chr + 0x818));
        v::sub(&mut f, at(PU), chr + 0x684, chr + 0x694);
        let mut dy = ld(g32(&f, chr + 0x10)).sub_m32(g32(&f, chr + 0x20)).store();
        v::dot(&mut f, at(PU), at(PU), at(DOT));
        let mut dist = ld(g32(&f, at(DOT))).sqrt().store();
        let mut extra = 0.0f32;
        let ba = f.rebase(MS_BATTLE_CHECK);
        let reach = if f.inner().call(ba, &[]) == 0 {
            let bit = usize::from(f.read::<u8>(chr + 0x820) & 2 == 0);
            // the moving reach only when ordered above 0.1 (a NaN is still)
            let moving = ld(dist).value() > STILL.get(&f);
            round_trip(
                if moving {
                    REACH_MOVING[bit]
                } else {
                    REACH_STILL[bit]
                }
                .get(&f),
            )
        } else {
            let r = round_trip(REACH_BATTLE.get(&f));
            let x: u32 = f.read(chr + 0x820);
            f.write(chr + 0x820, x & !4);
            dist = 0.0;
            dy = 0.0;
            let motion: u32 = f.read(chr + 0x72c);
            if f.read::<u16>(chr) == 3 && (motion == 0x310b0 || motion == 0x310b1) {
                ka = 0.0;
                kb = 0.0;
            }
            let e = g32(&f, chr + 0x81c);
            if ld(e).value() != 0.0 {
                extra = round_trip(e);
            }
            r
        };
        let x = f.read::<u32>(chr + 0x820) & !2;
        f.write(chr + 0x820, x);
        let s2 = (ld(scale) * ld(scale)).store();
        dy = ld(s2).mul_m32(dy).store();
        let mut iters = 0i32;
        let mut high = false;
        let far = ld(reach).value() < ld(dist).value();
        let f810: u16 = f.read(chr + 0x810);
        let id6 = matches!(f.read::<u16>(chr), 6 | 7) && matches!(MAP.get(&f), 0x15d | 0x84);
        if id6 && (f810 != 0 || far || x & 4 != 0) {
            iters = i16_at(&f, chr + 0x812).wrapping_mul(2);
        } else if f810 != 0 || far || x & 4 != 0 {
            iters = i16_at(&f, chr + 0x812);
            high = true;
        }
        f.write(chr + 0x820, x & !4);
        if floor {
            put(&mut f, chr + 0x18, 1.0);
            f.write(at(P0), [0.0f32, 0.0, 0.0, 1.0]);
            mx::transpose(&mut f, at(BINV), chr + 0x210);
            v::matrix_apply_vec3(&mut f, at(NV), at(BINV), chr + 0x7c);
            vu0::apply_matrix(&mut f, at(QV), chr + 0x1d0, at(P0));
        }
        let k = Reg::constant(THOUSAND.get(&f));
        loop {
            let mut g = 0u32;
            while g < u32::from(f.read::<u16>(f.read::<u32>(chr + 0x1c0) + 6)) {
                let groups: u32 = f.read(chr + 0x1bc);
                let grp = groups.wrapping_add(0x38 * g);
                let hdr: u32 = f.read(grp);
                if f.read::<u16>(hdr + 0x18) != 0 {
                    group(
                        &mut f,
                        chr,
                        g,
                        grp,
                        hdr,
                        &mut Step {
                            k,
                            s2,
                            scale,
                            reach,
                            extra,
                            dy,
                            ka,
                            kb,
                            wind,
                            floor,
                            high: &mut high,
                            iters: &mut iters,
                        },
                    );
                }
                g += 1;
            }
            let e = iters;
            iters = iters.wrapping_sub(1);
            high = false;
            if e == 0 {
                break;
            }
        }
        let x: u32 = f.read(chr + 0x194);
        f.write(chr + 0x194, x & !0x80_0000);
        f.write(chr + 0x810, 0u16);
    }

    /// The step's values that a group reads (and the two it may change).
    struct Step<'a> {
        k: Reg,
        s2: f32,
        scale: f32,
        reach: f32,
        extra: f32,
        dy: f32,
        ka: f32,
        kb: f32,
        wind: f32,
        floor: bool,
        /// Set when a group is out of reach: the strands are lifted.
        high: &'a mut bool,
        /// The repeats left; out of reach sets them from `+0x812`.
        iters: &'a mut i32,
    }

    #[allow(clippy::too_many_lines)]
    fn group<H: Host>(
        f: &mut Frame<'_, H>,
        chr: u32,
        g: u32,
        grp: u32,
        hdr: u32,
        st: &mut Step<'_>,
    ) {
        let at = |o: u32| CLOTH_FRAME + o;
        let k = st.k;
        let root = bone_at(f, chr, i32::from(f.read::<u16>(hdr)));
        mul_bone(f, at(MM), chr + 0x1d0, root, k);
        let tx = g32(f, at(MM) + 0x30);
        let mut free = f.read::<u16>(chr + 0x810) == 0;
        if f.read::<u32>(chr + 0x194) & 0x80_0000 != 0 {
            f.write(at(FV), [0.0f32; 3]);
        } else {
            let x = ld(tx).subr_m32(g32(f, grp + 0x1c)).store();
            put(f, at(FV), x);
            let y = ld(g32(f, grp + 0x20))
                .sub_m32(g32(f, at(MM) + 0x34))
                .store();
            put(f, at(FV) + 4, y);
            let z = ld(g32(f, grp + 0x24))
                .sub_m32(g32(f, at(MM) + 0x38))
                .store();
            put(f, at(FV) + 8, z);
            v::dot(f, at(FV), at(FV), at(DOT));
            let len = ld(g32(f, at(DOT))).sqrt().store();
            if ld(st.reach).value() < ld(len).value() {
                free = false;
                f.write(at(FV), [0.0f32; 3]);
                *st.high = true;
                *st.iters = i16_at(f, chr + 0x812);
            } else {
                let s2 = ld(st.s2);
                let x = (ld(g32(f, at(FV))) * s2).store();
                put(f, at(FV), x);
                let y = (ld(g32(f, at(FV) + 4)) * s2).store();
                put(f, at(FV) + 4, y);
                let z = s2.mul_m32(g32(f, at(FV) + 8)).store();
                put(f, at(FV) + 8, z);
                let mut q = (ld(len) / Reg::constant(FOLLOW_UNIT.get(f))).store();
                q = (ld(q) * ld(q)).store();
                if ld(st.extra).value() != 0.0 {
                    let e = g32(f, chr + 0x81c);
                    if ld(e).value() < ld(q).value() {
                        q = round_trip(e);
                    }
                }
                let y = if f.read::<u8>(chr + 0x820) & 1 != 0 {
                    let t = ld(g32(f, at(FV) + 4)).add_m32(st.dy).store();
                    ld(t).mul_m32(st.ka).store()
                } else {
                    0.0
                };
                put(f, at(FV) + 4, y);
                let qk = ld(q).mul_m32(st.kb);
                let x = (ld(g32(f, at(FV))) * qk).store();
                put(f, at(FV), x);
                let z = qk.mul_m32(g32(f, at(FV) + 8)).store();
                put(f, at(FV) + 8, z);
            }
        }
        put(f, grp + 0x1c, round_trip(tx));
        let y = round_trip(g32(f, at(MM) + 0x34));
        put(f, grp + 0x20, y);
        let z = round_trip(g32(f, at(MM) + 0x38));
        put(f, grp + 0x24, z);
        mx::orthonormal_inverse3x4(f, at(MINV), at(MM));
        let s2 = ld(st.s2);
        let mut i = 0u32;
        while i < u32::from(f.read::<u16>(f.read::<u32>(chr + 0x1c0) + 0x2c)) {
            let pt = f.read::<u32>(chr + 0x32c).wrapping_add(0x60 * i);
            let model: u32 = f.read(chr + 0x1c0);
            let rec = f.read::<u32>(model + 0x30).wrapping_add(0x40 * i);
            let bone = bone_at(f, chr, i16_at(f, rec + 0xa));
            mul_bone(f, at(WM), chr + 0x1d0, bone, k);
            copy16(f, at(RECA), rec + 0x20);
            copy16(f, at(RECB), rec + 0x10);
            if matches!(f.read::<u16>(rec + 8), 1 | 2) {
                let bone = bone_at(f, chr, i16_at(f, rec + 0xa));
                v::matrix_apply_vec3(f, pt + 0x10, bone + 0x88, at(RECA));
            }
            vu0::apply_matrix(f, at(TV), at(WM), at(RECB));
            vu0::apply_matrix(f, pt, at(MINV), at(TV));
            let x = (ld(g32(f, pt)) * s2).store();
            put(f, pt, x);
            let y = s2.mul_m32(g32(f, pt + 4)).store();
            put(f, pt + 4, y);
            let z = s2.mul_m32(g32(f, pt + 8)).store();
            put(f, pt + 8, z);
            i += 1;
        }
        if st.floor {
            vu0::apply_matrix(f, at(P0), at(MINV), at(QV));
            let x = (ld(g32(f, at(P0))) * s2).store();
            put(f, at(P0), x);
            let y = s2.mul_m32(g32(f, at(P0) + 4)).store();
            put(f, at(P0) + 4, y);
            let z = s2.mul_m32(g32(f, at(P0) + 8)).store();
            put(f, at(P0) + 8, z);
        }
        let root = bone_at(f, chr, i32::from(f.read::<u16>(hdr)));
        scale_bone(f, root, k, true);
        mx::orthonormal_inverse3x4(f, at(BINV), root + 0x88);
        scale_bone(f, root, k, false);
        let mut vp: u32 = f.read(grp + 0xc);
        let mut kk = 0u32;
        while kk < u32::from(f.read::<u16>(hdr + 0x18)) {
            let s = f.read::<u32>(hdr + 0x1c).wrapping_add(0x24 * kk);
            let tc = strand(f, chr, g, kk, grp, s, vp, free, st);
            vp = vp.wrapping_add(off(tc, 0x30));
            kk += 1;
        }
        f.write(grp + 0x34, 1u32);
    }

    /// One strand (0x24 bytes at `s`) of `vp`'s points; returns its point
    /// count (fixed, moving and anchors).
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    fn strand<H: Host>(
        f: &mut Frame<'_, H>,
        chr: u32,
        g: u32,
        kk: u32,
        grp: u32,
        s: u32,
        vp: u32,
        free: bool,
        st: &Step<'_>,
    ) -> i32 {
        let at = |o: u32| CLOTH_FRAME + o;
        let k = st.k;
        let damp = if free {
            round_trip(g32(f, s + 0x18))
        } else {
            0.0
        };
        if f.read::<u32>(grp + 0x34) == 0 {
            let mut q = 0;
            while q < i16_at(f, s + 0x1c) {
                let list: u32 = f.read(s + 8);
                let i = i16_at(f, list.wrapping_add(off(q, 2)));
                let marks: u32 = f.read(grp + 0x30);
                f.write(marks.wrapping_add(off(i, 1)), 1u8);
                q += 1;
            }
        }
        let slot = f.rebase(RAND_SLOT);
        let rand: u32 = f.read(slot);
        #[allow(clippy::cast_possible_wrap)]
        let r = (f.inner().call(rand, &[]) as i32).wrapping_rem(1000);
        #[allow(clippy::cast_precision_loss)] // |r| < 1000
        let w = ld(r as f32).mul_m32(st.wind).store();
        copy16(f, at(PU), at(WINDV));
        let w = ld(w);
        let x = (ld(g32(f, at(PU))) * w).store();
        put(f, at(PU), x);
        let y = (ld(g32(f, at(PU) + 4)) * w).store();
        put(f, at(PU) + 4, y);
        let z = w.mul_m32(g32(f, at(PU) + 8)).store();
        put(f, at(PU) + 8, z);
        for q in 0..3 {
            let x = ld(g32(f, at(PU) + 4 * q))
                .add_m32(g32(f, at(FV) + 4 * q))
                .store();
            put(f, at(PU) + 4 * q, x);
        }
        let y = if *st.high {
            (ld(st.scale) * Reg::constant(FAR_LIFT.get(f)) * k).store()
        } else {
            ld(g32(f, s + 4))
                .mul_m32(st.scale)
                .add_m32(g32(f, at(PU) + 4))
                .store()
        };
        put(f, at(PU) + 4, y);
        let y = ld(g32(f, s + 4)).mul_m32(st.scale).add_m32(y).store();
        put(f, at(PU) + 4, y);
        v::matrix_apply_vec3(f, at(PU), at(MINV), at(PU));

        // the anchors, from their bones
        let mut j = 0;
        while j < i32::from(f.read::<i8>(s + 0x13)) {
            let a = f.read::<u32>(s + 0x20).wrapping_add(off(j, 0x20));
            let bone = bone_at(f, chr, i16_at(f, a + 0x10));
            mul_bone(f, at(WM), at(BINV), bone, k);
            let a = f.read::<u32>(s + 0x20).wrapping_add(off(j, 0x20));
            copy16(f, at(LOC), a);
            vu0::apply_matrix(f, at(LOC), at(WM), at(LOC));
            let i = i16_at(f, s + 0x1c).wrapping_add(j);
            copy16(f, vp.wrapping_add(off(i, 16)), at(LOC));
            j += 1;
        }
        let tc = i16_at(f, s + 0x1c).wrapping_add(i32::from(f.read::<i8>(s + 0x13)));
        let mut idx: u32 = f.read(s + 8);
        let mut q = 0;
        while q < i16_at(f, s) {
            let verts: u32 = f.read(grp + 8);
            let i = u32::from(f.read::<u16>(idx));
            copy16(f, vp.wrapping_add(off(q, 16)), verts.wrapping_add(16 * i));
            idx = idx.wrapping_add(2);
            q += 1;
        }

        // the wind and the motion
        let pu: [f32; 3] = f.read(at(PU));
        let mut q = i16_at(f, s);
        while q < i16_at(f, s + 0x1c) {
            let p = vp.wrapping_add(off(q, 16));
            let o = vp.wrapping_add(off(tc.wrapping_add(q), 16));
            for c in 0..3 {
                let x = if free {
                    (ld(g32(f, o + 4 * c)) + ld(pu[c as usize])).add_m32(g32(f, p + 4 * c))
                } else {
                    ld(g32(f, p + 4 * c)) + ld(pu[c as usize])
                };
                put(f, p + 4 * c, x.store());
            }
            q += 1;
        }

        // the rods
        let mut r: u32 = f.read(s + 0xc);
        let mut kind = i16_at(f, r);
        while kind != 0 {
            let n = i16_at(f, r + 2);
            let rods = r.wrapping_add(0xc);
            if (1..=6).contains(&kind) {
                #[allow(clippy::cast_sign_loss)]
                v::cloth_rods(f, kind as u32, rods, n as u32, vp);
            }
            r = rods.wrapping_add(off(n, 12));
            kind = i16_at(f, r);
        }

        // two points lifted on map 0x107 (character 2, group 2, strand 2)
        if MAP.get(f) == 0x107 {
            let fa = f.rebase(GET_SCENERIO_FLAG);
            #[allow(clippy::cast_possible_truncation)]
            if f.inner().call(fa, &[]) as u16 >= 0x532 {
                #[allow(clippy::cast_possible_truncation)]
                let s2 = f.inner().call(fa, &[]) as u16;
                if s2 <= 0x53c && f.read::<u16>(chr) == 2 && g == 2 && kk == 2 {
                    for (o, lift) in [(0, LIFT_107[0]), (2, LIFT_107[1])] {
                        let list: u32 = f.read(s + 0x14);
                        let c = i16_at(f, list.wrapping_add(o));
                        let y = f
                            .read::<u32>(chr + 0x32c)
                            .wrapping_add(off(c, 0x60))
                            .wrapping_add(4);
                        let v = (ld(g32(f, y)) + Reg::constant(lift.get(f))).store();
                        put(f, y, v);
                    }
                }
            }
        }

        // the moving points: collisions, the floor, written back
        let damp = ld(damp);
        let stride = off(tc, 16);
        let mut q = i16_at(f, s);
        while q < i16_at(f, s + 0x1c).wrapping_sub(i16_at(f, s + 0x1e)) {
            let p = vp.wrapping_add(off(q, 16));
            collide(f, chr, s, p, g, kk);
            if st.floor {
                floor_push(f, p);
            }
            let verts: u32 = f.read(grp + 8);
            settle(f, p, idx, verts, stride, damp);
            idx = idx.wrapping_add(2);
            q += 1;
        }
        // the free ends: the floor only
        while q < i16_at(f, s + 0x1c) {
            let p = vp.wrapping_add(off(q, 16));
            if st.floor {
                floor_push(f, p);
            }
            let verts: u32 = f.read(grp + 8);
            settle(f, p, idx, verts, stride, damp);
            idx = idx.wrapping_add(2);
            q += 1;
        }
        tc
    }
}
