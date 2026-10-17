//! Small float helpers of the main module: three-component vector
//! arithmetic (used by the cloth and triangle tests), the global wind,
//! the camera's defaults, a vector's length, radians to degrees.
//!
//! All x87 at the game's 53-bit precision (`ffx_bridge::x87`): each
//! component is computed and stored before the next one is read, so an
//! output that overlaps an input behaves as in the original.

#![allow(clippy::many_single_char_names)] // vectors and their components

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::x87::{self, Reg};
    use ffx_bridge::{Global, Mem};

    /// The global wind: direction x, y, z (`Sg_GlobalSetWind`).
    pub const WIND: Global<[f32; 3]> = Global::new(0x012f_bc44);
    /// The wind's default x (`f32` 0.01 in `.rdata`, `Sg_GlobalInitWind`).
    pub const WIND_DEFAULT: Global<f32> = Global::new(0x00b1_f2b0);
    /// The camera: a word and two floats (`Sg_InitCamera`).
    pub const CAMERA: Global<u32> = Global::new(0x0130_1bb0);
    /// The camera's default (`f32` 50.0 in `.rdata`).
    pub const CAMERA_DEFAULT: Global<f32> = Global::new(0x00b1_f2c8);
    /// 180 (`f64` in `.rdata`).
    pub const DEG_180: Global<f64> = Global::new(0x00b4_8378);
    /// π (`f64` in `.rdata`).
    pub const PI: Global<f64> = Global::new(0x00b5_82f0);

    fn get(mem: &impl Mem, v: u32, k: u32) -> f32 {
        mem.read(v.wrapping_add(4 * k))
    }

    fn put(mem: &mut impl Mem, v: u32, k: u32, x: f32) {
        mem.write(v.wrapping_add(4 * k), x);
    }

    /// `Sg_VecAdd(out, a, b)` (`0x00824ce0`): `out = a + b`.
    pub fn add(mem: &mut impl Mem, out: u32, a: u32, b: u32) {
        for k in 0..3 {
            let x = Reg::load(get(mem, a, k)).add_m32(get(mem, b, k));
            put(mem, out, k, x.store());
        }
    }

    /// `Sg_VecSub(out, a, b)` (`0x00824da0`): `out = a - b`.
    pub fn sub(mem: &mut impl Mem, out: u32, a: u32, b: u32) {
        for k in 0..3 {
            let x = Reg::load(get(mem, a, k)).sub_m32(get(mem, b, k));
            put(mem, out, k, x.store());
        }
    }

    /// `Sg_VecDot(a, b, &out)` (`0x00824d40`): `(a.y b.y + a.x b.x) + a.z b.z`.
    pub fn dot(mem: &mut impl Mem, a: u32, b: u32, out: u32) {
        let [x, y, z] = [0, 1, 2].map(|k| Reg::load(get(mem, a, k)).mul_m32(get(mem, b, k)));
        mem.write(out, ((y + x) + z).store());
    }

    /// `Sg_VecScale(out, v, s)` (`0x00824d70`): `out = v * s` (`x` and `y`
    /// multiply registers, `z` is `s * [v.z]`).
    pub fn scale(mem: &mut impl Mem, out: u32, v: u32, s: f32) {
        let s = Reg::load(s);
        let x = Reg::load(get(mem, v, 0)) * s;
        put(mem, out, 0, x.store());
        let y = Reg::load(get(mem, v, 1)) * s;
        put(mem, out, 1, y.store());
        let z = s.mul_m32(get(mem, v, 2));
        put(mem, out, 2, z.store());
    }

    /// `Sg_VecCopy4(out, v)` (`0x00824d10`): four floats through the x87
    /// (a signalling NaN comes out quiet).
    pub fn copy4(mem: &mut impl Mem, out: u32, v: u32) {
        for k in 0..4 {
            let x = x87::round_trip(get(mem, v, k));
            put(mem, out, k, x);
        }
    }

    /// `Sg_GlobalSetWind(x, y, z)` (`0x00824e30`).
    pub fn global_set_wind(mem: &mut impl Mem, x: f32, y: f32, z: f32) {
        WIND.set(mem, [x, y, z].map(x87::round_trip));
    }

    /// `Sg_GlobalInitWind()` (`0x00824e10`): `(0.01, 0, 0)`.
    pub fn global_init_wind(mem: &mut impl Mem) {
        let x = x87::round_trip(WIND_DEFAULT.get(mem));
        WIND.set(mem, [x, 0.0, 0.0]);
    }

    /// `Sg_InitCamera()` (`0x0083f780`): the word 0, both floats 50.
    pub fn init_camera(mem: &mut impl Mem) {
        let d = x87::round_trip(CAMERA_DEFAULT.get(mem));
        let p = CAMERA.ptr(mem).addr;
        mem.write(p, 0u32);
        mem.write(p.wrapping_add(4), [d, d]);
    }

    /// `Sg_RadToDeg(r)` (`0x008366f0`): `r * 180 / π`, stored as `f32`.
    pub fn rad_to_deg(mem: &impl Mem, r: f32) -> f32 {
        (Reg::load(r) * Reg::constant(DEG_180.get(mem)) / Reg::constant(PI.get(mem))).store()
    }

    /// `Sg_VectorGetLength(v)` (`0x00836670`): `sqrt((x² + y²) + z²)` (the
    /// CRT's `_CIsqrt`, the x87's `fsqrt`), stored as `f32`.
    pub fn vector_get_length(mem: &impl Mem, v: u32) -> f32 {
        let [x, y, z] = [0, 1, 2].map(|k| Reg::load(get(mem, v, k)));
        let s = (x * x + y * y) + z * z;
        // `fsqrt` passes a NaN through (Rust's `sqrt` may not keep its bits).
        let r = if s.value().is_nan() {
            s
        } else {
            Reg::constant(s.value().sqrt())
        };
        r.store()
    }

    /// `Sg_VecCross(out, a, b)` (`0x008295a0`): `out = a × b`, each
    /// component `a.i b.j - a.j b.i` stored before the next is read.
    pub fn cross(mem: &mut impl Mem, out: u32, a: u32, b: u32) {
        for (k, (i, j)) in [(1, 2), (2, 0), (0, 1)].into_iter().enumerate() {
            let p = Reg::load(get(mem, a, i)).mul_m32(get(mem, b, j));
            let q = Reg::load(get(mem, a, j)).mul_m32(get(mem, b, i));
            #[allow(clippy::cast_possible_truncation)] // 0..3
            put(mem, out, k as u32, (p - q).store());
        }
    }

    /// `Sg_GlobalGetWind(&x, &y, &z)` (`0x00824dd0`): null outputs are
    /// skipped.
    pub fn global_get_wind(mem: &mut impl Mem, x: u32, y: u32, z: u32) {
        let w = WIND.get(mem);
        for (o, v) in [(y, w[1]), (z, w[2]), (x, w[0])] {
            if o != 0 {
                mem.write(o, x87::round_trip(v));
            }
        }
    }

    /// `Sg_MatrixApplyVec3(out, m, v)` (`0x00824e60`): `out = (x, y, z)`
    /// times the first three rows of the 4×4 matrix `m` (four results, all
    /// computed before `out` is written). The products pair up as the
    /// original's `fmul`s do: the first column's terms are
    /// `m[0] v.x + v.y m[4]`, the others `m[4+j] v.y + m[j] v.x`.
    pub fn matrix_apply_vec3(mem: &mut impl Mem, out: u32, m: u32, v: u32) {
        let [x, y, z] = [0, 1, 2].map(|k| get(mem, v, k));
        let e = |k: u32| get(mem, m, k);
        let mut r = [0f32; 4];
        r[0] = ((Reg::load(e(0)).mul_m32(x) + Reg::load(y).mul_m32(e(4)))
            + Reg::load(z).mul_m32(e(8)))
        .store();
        for j in 1..4u32 {
            let t = Reg::load(e(4 + j)).mul_m32(y) + Reg::load(e(j)).mul_m32(x);
            r[j as usize] = (t + Reg::load(e(8 + j)).mul_m32(z)).store();
        }
        mem.write(out, r);
    }

    /// `Angle_Wrap(a)` (`0x0082db50`) and its five copies in other files:
    /// `a` brought into `-π ..= π` in steps of `2π`, each step stored as
    /// `f32` (`ffx_chara::chr::wrap_angle`, `Chr_WrapAngle`'s code). It
    /// never ends for an infinity, as the original.
    #[must_use]
    pub fn angle_wrap(a: f32) -> f32 {
        ffx_chara::chr::wrap_angle(a)
    }

    /// `Sg_AngleApproach(cur, target, step)` (`0x00835f80`): `cur` turned
    /// toward `target` by at most `step` the short way round (the
    /// difference wrapped first), wrapped; `target` itself when the
    /// wrapped difference is 0.
    #[must_use]
    pub fn angle_approach(cur: f32, target: f32, step: f32) -> f32 {
        let d = angle_wrap(Reg::load(cur).sub_m32(target).store());
        #[allow(clippy::float_cmp)] // the original's `fucom` with 0
        if d == 0.0 {
            return x87::round_trip(target);
        }
        let r = if d > 0.0 {
            if step > d {
                Reg::load(d).subr_m32(cur)
            } else {
                Reg::load(step).subr_m32(cur)
            }
        } else if -step < d {
            Reg::load((-Reg::load(d)).store()).add_m32(cur)
        } else {
            Reg::load(step).add_m32(cur)
        };
        angle_wrap(r.store())
    }

    /// `SG_RawToRad(x)` (`0x00838c60`): `x π / 2048` (4096 units a turn),
    /// wrapped.
    pub fn raw_to_rad(mem: &impl Mem, x: f32) -> f32 {
        let r = Reg::load(x) * Reg::constant(PI.get(mem)) * Reg::constant(RAW_SCALE.get(mem));
        angle_wrap(r.store())
    }

    /// 1/2048 (`f64` in `.rdata`, `SG_RawToRad`).
    pub const RAW_SCALE: Global<f64> = Global::new(0x00b5_8590);

    fn length_sq(d: [f32; 3]) -> f32 {
        let [x, y, z] = d.map(Reg::load);
        let s = (x * x + y * y) + z * z;
        let r = if s.value().is_nan() {
            s
        } else {
            Reg::constant(s.value().sqrt())
        };
        r.store()
    }

    /// `Sg_VectorGetLength2(a, b)` (`0x008366a0`): the distance from `a` to
    /// `b` (each difference stored as `f32` first).
    pub fn vector_get_length2(mem: &impl Mem, a: u32, b: u32) -> f32 {
        let d = [0, 1, 2].map(|k| Reg::load(get(mem, b, k)).sub_m32(get(mem, a, k)).store());
        length_sq(d)
    }

    /// `Sg_VectorGetLengthXZ(a, b)` (`0x0083e3e0`): the distance from `a` to
    /// `b` in the x-z plane.
    pub fn vector_get_length_xz(mem: &impl Mem, a: u32, b: u32) -> f32 {
        let dx = Reg::load(Reg::load(get(mem, b, 0)).sub_m32(get(mem, a, 0)).store());
        let dz = Reg::load(Reg::load(get(mem, b, 2)).sub_m32(get(mem, a, 2)).store());
        let s = dx * dx + dz * dz;
        let r = if s.value().is_nan() {
            s
        } else {
            Reg::constant(s.value().sqrt())
        };
        r.store()
    }

    /// `Sg_IntVecToFloat(out, v)` (`0x00845d90`): three `i32` as `f32`
    /// (`fild`/`fstp`). Returns `v`'s fourth word (`eax`).
    pub fn int_vec_to_float(mem: &mut impl Mem, out: u32, v: u32) -> u32 {
        let w: [i32; 4] = mem.read(v);
        for (k, x) in (0u32..3).zip(w) {
            put(mem, out, k, f32::from_bits(x87::store(f64::from(x))));
        }
        w[3].cast_unsigned()
    }

    /// `Sg_CalcTriangleNormal(v0, v1, v2, out)` (`0x0083e420`): with `i16`
    /// vertices, the edges `v1 - v0` and `v2 - v0` as `f32`, their outer
    /// product (`FFXVu0OuterProduct(n, e2, e1)`) normalised into `out`
    /// (`FFXVu0Normalize`).
    pub fn calc_triangle_normal(mem: &mut impl Mem, v0: u32, v1: u32, v2: u32, out: u32) {
        use crate::scratch::{SCRATCH, Scratch};
        use ffx_ps2::vu0::bridge as vu0;
        let p: [i16; 3] = mem.read(v0);
        let a: [i16; 3] = mem.read(v1);
        let b: [i16; 3] = mem.read(v2);
        let edge = |q: [i16; 3]| {
            [0, 1, 2].map(|k| x87::store(f64::from(i32::from(q[k]) - i32::from(p[k]))))
        };
        let (e1, e2) = (edge(a), edge(b));
        let mut s = Scratch::new(mem);
        // e1 at +0x20, e2 at +0x10, the product at +0 (w of each 0)
        s.write(SCRATCH + 0x20, e1);
        s.write(SCRATCH + 0x10, e2);
        vu0::outer_product(&mut s, SCRATCH, SCRATCH + 0x10, SCRATCH + 0x20);
        vu0::normalize(&mut s, out, SCRATCH);
    }

    /// `Sg_MatrixFromDir(m, dir, rows)` (`0x008435a0`): an orthonormal
    /// basis around `dir`: `d = |dir|`, `s = |(0, 0, 1) x d|`, `u = |s x
    /// d|` (`FFXVu0Normalize`, `Sg_VecCross`); `m` the identity
    /// (`FFXVu0UnitMatrix`), then the `x, y, z` of `s` into row `rows >> 8
    /// & 15`, of `u` into row `rows >> 4 & 15` and of `d` into row `rows &
    /// 15`, in that order (rows past 3 write beyond the matrix, as the
    /// original). Returns `eax`: twice `u`'s row.
    pub fn matrix_from_dir(mem: &mut impl Mem, m: u32, dir: u32, rows: u32) -> u32 {
        use crate::scratch::{SCRATCH, Scratch};
        use ffx_ps2::vu0::bridge as vu0;
        let (d, z, t, s_, u) = (
            SCRATCH,
            SCRATCH + 0x10,
            SCRATCH + 0x20,
            SCRATCH + 0x30,
            SCRATCH + 0x40,
        );
        let mut sc = Scratch::new(mem);
        sc.write(z, [0f32, 0.0, 1.0]);
        vu0::normalize(&mut sc, d, dir);
        cross(&mut sc, t, z, d);
        vu0::normalize(&mut sc, s_, t);
        cross(&mut sc, t, s_, d);
        vu0::normalize(&mut sc, u, t);
        vu0::matrix_identity(&mut sc, m);
        let row = |r: u32| m.wrapping_add(16 * (r & 0xf));
        for (v, r) in [(s_, rows >> 8), (u, rows >> 4), (d, rows)] {
            let x: [f32; 3] = sc.read(v);
            let at = row(r);
            for (k, c) in (0u32..).zip(x) {
                sc.write(at.wrapping_add(4 * k), x87::round_trip(c));
            }
        }
        ((rows >> 4) & 0xf) * 2
    }

    /// `Sg_ClothRod*`'s threshold for kinds 5 and 6 (`f64` 1e-5) and the
    /// half (`f64` 0.5).
    pub const ROD_EPSILON: Global<f64> = Global::new(0x00b5_01e8);
    pub const ROD_HALF: Global<f64> = Global::new(0x00b9_2220);

    /// The cloth's rod constraints, kind 1-6 of the cloth step
    /// (`0x00822de0`): `n` rods of 12 bytes (`i16` points `a`, `b` into
    /// `points`, 16 bytes each; `f32` length; for kinds 1-4 an `f32` limit
    /// on the squared distance) with `d = b - a` (`Sg_VecSub`), `dd = d.d`
    /// (`Sg_VecDot`), `s = sqrt(dd)` stored as `f32`:
    ///
    /// - 1 `Sg_ClothRodMaxPin` (`0x00822890`), 3 `Sg_ClothRodMinPin`
    ///   (`0x00822a50`): when `dd` is above (1) or below (3) the limit, `b
    ///   = a + d * (len / s)`.
    /// - 2 `Sg_ClothRodMax` (`0x00822960`), 4 `Sg_ClothRodMin`
    ///   (`0x00822b20`): the same tests; both move, `k = (s - len) / s *
    ///   0.5`, `a += d k`, `b -= d k`.
    /// - 5 `Sg_ClothRodFixPin` (`0x00822c10`): as 1 always, but `b = a`
    ///   (`Sg_VecCopy4`) unless `|dd| > 1e-5`.
    /// - 6 `Sg_ClothRodFix` (`0x00822cf0`): as 2 always, but `b = a` unless
    ///   `|s| > 1e-5`.
    ///
    /// A NaN comparison does nothing (1-4) or copies (5, 6). Returns 0
    /// (the count left, `eax`).
    pub fn cloth_rods(mem: &mut impl Mem, kind: u32, rods: u32, n: u32, points: u32) -> u32 {
        use crate::scratch::{SCRATCH, Scratch};
        let (eps, half) = (ROD_EPSILON.get(mem), ROD_HALF.get(mem));
        let mut m = Scratch::new(mem);
        let (d, dd_at) = (SCRATCH, SCRATCH + 0x10);
        for r in 0..n {
            let e = rods.wrapping_add(12 * r);
            let pa = points.wrapping_add((i32::from(m.read::<i16>(e)) << 4).cast_unsigned());
            let pb = points
                .wrapping_add((i32::from(m.read::<i16>(e.wrapping_add(2))) << 4).cast_unsigned());
            let len = x87::load(m.read(e.wrapping_add(4)));
            let lim = x87::load(m.read(e.wrapping_add(8)));
            sub(&mut m, d, pb, pa);
            dot(&mut m, d, d, dd_at);
            let dd = x87::load(m.read(dd_at));
            let s = || x87::load(x87::store(x87::sqrt(dd)));
            let pin = |m: &mut Scratch<'_, _>, s: f64| {
                let k = f32::from_bits(x87::store(x87::div(len, s)));
                scale(m, d, d, k);
                add(m, pb, pa, d);
            };
            let both = |m: &mut Scratch<'_, _>, s: f64| {
                let k = x87::mul(x87::div(x87::sub(s, len), s), half);
                scale(m, d, d, f32::from_bits(x87::store(k)));
                add(m, pa, pa, d);
                sub(m, pb, pb, d);
            };
            match kind {
                1 if dd > lim => pin(&mut m, s()),
                3 if dd < lim => pin(&mut m, s()),
                2 if dd > lim => both(&mut m, s()),
                4 if dd < lim => both(&mut m, s()),
                5 if dd.abs() > eps => pin(&mut m, s()),
                6 if s().abs() > eps => both(&mut m, s()),
                5 | 6 => copy4(&mut m, pb, pa),
                _ => {}
            }
        }
        0
    }

    /// The anchors' default lift (`f32` 1.1 in `.rdata`).
    pub const ANCHOR_LIFT: Global<f32> = Global::new(0x00b4_3d28);
    /// The lift of character 1's group 0, mesh 2 (`f32` 3.0).
    pub const ANCHOR_LIFT_HIGH: Global<f32> = Global::new(0x00b3_eb94);
    /// Below this squared length a matrix row counts as collapsed (`f64`
    /// 2.5e-7).
    pub const ANCHOR_FLAT: Global<f64> = Global::new(0x00b5_9868);
    /// `Cache_Exist_ClothVertexBuff(kind, id)`.
    pub const CACHE_EXIST_CLOTH_VERTEX_BUFF: u32 = 0x0084_0850;
    /// `graphicLevelGetNameHash()`.
    pub const GRAPHIC_LEVEL_GET_NAME_HASH: u32 = 0x0064_1da0;
    /// `Ch_UseAnchorVertex(chr, group, mesh)`.
    pub const CH_USE_ANCHOR_VERTEX: u32 = 0x0083_16d0;
    /// The level whose collapsed models zero their anchors.
    pub const ANCHOR_FLAT_LEVEL: u32 = 0xc58;

    /// `Sg_ClothPlaceAnchors(chr)` (`0x0082dbc0`): puts each cloth's anchor
    /// vertices on the model's triangles. Runs when the character has a
    /// cloth (`+0x834`), a cloth vertex buffer
    /// (`Cache_Exist_ClothVertexBuff(1, chr[0])`) and `+0x194` bit 21;
    /// first sets `cloth+8` to the sign-extended byte `+0x180`.
    ///
    /// For every list `i < cloth+0x14` (`cloth+0x30[i]`, paired with
    /// `vb+4[i]`) and each entry pair `(e, v)` of them with `e+0x4c` (the
    /// output, 16 bytes a vertex) set: the mesh is model group `e+0x40`,
    /// sub-mesh `e+0x42 % 100` (`[chr+0x1bc] + 0x38 g`: `+0` header with
    /// the sub-mesh count at `+0x18`, `+8` the vertices, `+0x2c` the
    /// sub-meshes, each with its triangles at `+0x44`). A missing
    /// sub-mesh only marks `e+0x48`. On level `0xc58` with a collapsed
    /// matrix (any of the rows `+0x1d0`, `+0x1e0`, `+0x1f0` of squared
    /// length below 2.5e-7) every output is zeroed. Otherwise each of the
    /// `e+0x44` vertices (`v+8`: 8 bytes, flags, triangle `u16`, lift
    /// `f32`; `v+0xc`: three weights) not on triangle `0xffff` becomes
    /// the weighted sum of its triangle's corners; when the anchor is in
    /// use (`Ch_UseAnchorVertex(chr, g, k)`) or the vertex's bit 0 is set
    /// it is lifted along the triangle's normal by `lift / |n| *` 1.1 (3.0
    /// for character 1's group 0, sub-mesh 2). `e+0x48` is then set to 1.
    #[allow(clippy::too_many_lines)]
    pub fn cloth_place_anchors(h: &mut impl ffx_bridge::Host, chr: u32) {
        let cloth: u32 = h.read(chr.wrapping_add(0x834));
        let id: u16 = h.read(chr);
        let at = h.rebase(CACHE_EXIST_CLOTH_VERTEX_BUFF);
        let vb = h.call(at, &[1, u32::from(id)]);
        let flags: u32 = h.read(chr.wrapping_add(0x194));
        if cloth == 0 || vb == 0 || flags & 0x20_0000 == 0 {
            return;
        }
        let b: i8 = h.read(chr.wrapping_add(0x180));
        #[allow(clippy::cast_sign_loss)]
        h.write(cloth.wrapping_add(8), i32::from(b) as u32);
        let at = h.rebase(GRAPHIC_LEVEL_GET_NAME_HASH);
        let flat = h.call(at, &[]) == ANCHOR_FLAT_LEVEL && {
            let thr = ANCHOR_FLAT.get(h);
            [0x1d0u32, 0x1e0, 0x1f0].iter().any(|&r| {
                let [x, y, z] = [0, 1, 2].map(|k| Reg::load(get(h, chr.wrapping_add(r), k)));
                Reg::load(((x * x + y * y) + z * z).store()).value() < thr
            })
        };
        let mut i = 0u32;
        while i < h.read::<u32>(cloth.wrapping_add(0x14)) {
            let l: u32 = h.read(cloth.wrapping_add(0x30 + 4 * i));
            let v: u32 = h.read(vb.wrapping_add(4 + 4 * i));
            if l != 0 && v != 0 && h.read::<u32>(l) != 0 {
                let mut j = 0u32;
                while j < h.read::<u32>(l) {
                    let e: u32 = h.read(l.wrapping_add(4 + 4 * j));
                    let ve: u32 = h.read(v.wrapping_add(4 + 4 * j));
                    j += 1;
                    let out: u32 = if e != 0 && ve != 0 {
                        h.read(e.wrapping_add(0x4c))
                    } else {
                        0
                    };
                    if out == 0 {
                        continue;
                    }
                    place(h, chr, e, ve, out, flat);
                    h.write(e.wrapping_add(0x48), 1u32);
                }
            }
            i += 1;
        }
    }

    /// One entry of `cloth_place_anchors`.
    fn place(h: &mut impl ffx_bridge::Host, chr: u32, e: u32, ve: u32, mut out: u32, flat: bool) {
        let g: u16 = h.read(e.wrapping_add(0x40));
        let k: u16 = h.read(e.wrapping_add(0x42));
        let at = h.rebase(CH_USE_ANCHOR_VERTEX);
        let anchor = h.call(at, &[chr, u32::from(g), u32::from(k)]);
        let mut tri: u32 = h.read(ve.wrapping_add(8));
        let mut w: u32 = h.read(ve.wrapping_add(0xc));
        let rem = u32::from(k % 100);
        let grp = h
            .read::<u32>(chr.wrapping_add(0x1bc))
            .wrapping_add(0x38 * u32::from(g));
        let tbl: u32 = h.read(grp.wrapping_add(0x2c));
        if tbl == 0 {
            return;
        }
        let sub: u32 = h.read(tbl.wrapping_add(4 * rem));
        if sub == 0 {
            return;
        }
        let head: u32 = h.read(grp);
        if u32::from(h.read::<u16>(head.wrapping_add(0x18))) <= rem {
            return;
        }
        let verts: u32 = h.read(grp.wrapping_add(8));
        let faces: u32 = h.read(sub.wrapping_add(0x44));
        let count = |h: &mut _| Mem::read::<u32>(h, e.wrapping_add(0x44));
        if flat {
            let mut n = 0;
            while n < count(h) {
                h.write(out, [0.0f32; 3]);
                out = out.wrapping_add(0x10);
                n += 1;
            }
            return;
        }
        let lift = if id_is(h, chr, 1) && g == 0 && rem == 2 {
            ANCHOR_LIFT_HIGH.get(h)
        } else {
            ANCHOR_LIFT.get(h)
        };
        let mut n = 0;
        while n < count(h) {
            let idx: u16 = h.read(tri.wrapping_add(2));
            if idx != 0xffff {
                let f = faces.wrapping_add(6 * u32::from(idx));
                let corner = |h: &mut _, o: u32| {
                    let c: i16 = Mem::read(h, f.wrapping_add(o));
                    #[allow(clippy::cast_sign_loss)]
                    let at = verts.wrapping_add((i32::from(c) * 0x10) as u32);
                    Mem::read::<[f32; 3]>(h, at)
                };
                let (c0, c1, c2) = (corner(h, 0), corner(h, 2), corner(h, 4));
                let wt: [f32; 3] = h.read(w);
                let mut p = [0, 1, 2].map(|q| {
                    ((Reg::load(wt[0]).mul_m32(c0[q]) + Reg::load(c1[q]).mul_m32(wt[1]))
                        + Reg::load(c2[q]).mul_m32(wt[2]))
                    .store()
                });
                let bits: u8 = h.read(tri);
                if anchor != 0 || bits & 1 != 0 {
                    let e1 = [0, 1, 2].map(|q| Reg::load(c1[q]).sub_m32(c0[q]).store());
                    let e2 = [0, 1, 2].map(|q| Reg::load(c2[q]).sub_m32(c0[q]).store());
                    let mut nv = [0.0f32; 3];
                    for (q, (a, b)) in [(1, 2), (2, 0), (0, 1)].into_iter().enumerate() {
                        let x = Reg::load(e1[a]).mul_m32(e2[b]);
                        let y = Reg::load(e1[b]).mul_m32(e2[a]);
                        nv[q] = (x - y).store();
                    }
                    let [dx, dy, dz] = [0, 1, 2].map(|q| Reg::load(nv[q]).mul_m32(nv[q]));
                    let dot = ((dy + dx) + dz).store();
                    let len = Reg::load(dot).sqrt().store();
                    let inv = Reg::load(len).divr_m32(1.0).store();
                    let l: f32 = h.read(tri.wrapping_add(4));
                    let s = Reg::load(l).mul_m32(inv).mul_m32(lift).store();
                    let sr = Reg::load(s);
                    nv[0] = (Reg::load(nv[0]) * sr).store();
                    nv[1] = (Reg::load(nv[1]) * sr).store();
                    nv[2] = sr.mul_m32(nv[2]).store();
                    for q in 0..3 {
                        p[q] = Reg::load(p[q]).add_m32(nv[q]).store();
                    }
                }
                h.write(out, p);
            }
            n += 1;
            tri = tri.wrapping_add(8);
            out = out.wrapping_add(0x10);
            w = w.wrapping_add(0xc);
        }
    }

    fn id_is(h: &impl Mem, chr: u32, id: u16) -> bool {
        h.read::<u16>(chr) == id
    }
}
