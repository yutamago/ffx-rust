//! The matrix programs: the compound rotations (`pppGetRotMatrixXYZ` and
//! its five siblings), the steps that build an object's matrix at `+0x10`
//! (`pppMatrix*`, `pppMatrixLoc`, `pppMatrixScl`, `pppParMatrix`,
//! `pppKeMatSN`, `pppKeMatPht`, `pppKeOfsMatXYZ`) and the draws that turn
//! it into the view matrix at `+0x50` (`pppDrawMatrix*`, `pppKeDMat*`).
//!
//! An object's matrix is row-major (`ffx_system::math`): rows at `+0x10`,
//! `+0x20`, `+0x30`, the position at `+0x40`. The steps take the position
//! from work area 0, the angles (three `i32`, a turn being `0x10000`) from
//! area 1 and the scale from area 2. The PC port computes as the PS2's VU0
//! did, through emulated VF registers in globals (`0x00c8f820` and on);
//! those writes are reproduced.
//!
//! A compound rotation `pppGetRotMatrixABC(out, angles)` multiplies the
//! axis rotations `C`, then `B`, then `A` with `cdc_Mul4x4` (each rotation
//! from `g_pppSinTable`, filled with `r0` as `pppGetRotMatrixX` builds it).

// m, s, p, q, t as in the formulas
#![allow(clippy::many_single_char_names)]

/// A rotation axis (index of its angle).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    /// `pppGetRotMatrixX`, angle 0.
    X,
    /// `pppGetRotMatrixY`, angle 1.
    Y,
    /// `pppGetRotMatrixZ`, angle 2.
    Z,
}

/// The order the compound rotations build in (first, second, third).
pub mod order {
    use super::Axis::{self, X, Y, Z};

    /// `pppGetRotMatrixXYZ`.
    pub const XYZ: [Axis; 3] = [Z, Y, X];
    /// `pppGetRotMatrixXZY`.
    pub const XZY: [Axis; 3] = [Y, Z, X];
    /// `pppGetRotMatrixYXZ`.
    pub const YXZ: [Axis; 3] = [Z, X, Y];
    /// `pppGetRotMatrixYZX`.
    pub const YZX: [Axis; 3] = [X, Z, Y];
    /// `pppGetRotMatrixZXY`.
    pub const ZXY: [Axis; 3] = [Y, X, Z];
    /// `pppGetRotMatrixZYX`.
    pub const ZYX: [Axis; 3] = [X, Y, Z];
}

/// The matrix programs over game memory.
pub mod bridge {
    use ffx_bridge::x87::{Reg, round_trip};
    use ffx_bridge::{Global, Mem};
    use ffx_ps2::vu0::bridge as vu0;
    use ffx_system::{math, stub};

    use super::Axis;
    use crate::ppp::bridge as ppp;
    use crate::prog::bridge::CURRENT_PART;

    /// The emulated VF register the scale goes through (`0x00c8f820`).
    pub const VF_SCALE: Global<[u32; 4]> = Global::new(0x00c8_f820);
    /// The emulated VF registers the rows go through (`0x00c8f830`, three
    /// of them).
    pub const VF_ROWS: Global<[u32; 12]> = Global::new(0x00c8_f830);
    /// `pppParMatrix`'s result VF register (`0x00c8f930`), then its
    /// vector (`0x00c8f940`) and matrix (`0x00c8f950`).
    pub const VF_PAR: Global<[u32; 4]> = Global::new(0x00c8_f930);
    /// `pppParMatrix`'s vector VF register.
    pub const VF_PAR_V: Global<[u32; 4]> = Global::new(0x00c8_f940);
    /// `pppParMatrix`'s matrix VF registers.
    pub const VF_PAR_M: Global<[u32; 16]> = Global::new(0x00c8_f950);
    /// The emulated VU0 accumulator (`0x00c8f790`).
    pub const ACC: Global<[u32; 4]> = Global::new(0x00c8_f790);
    /// `g_Vu0_Vf00`'s `w` (1.0).
    const VF00_W: Global<u32> = Global::new(0x00c0_a010);
    /// `g_pppViewMatrix`: the effects' view matrix (`g_pppWorldMatrix` ×
    /// `g_pppCameraMatrix`), its translation at `+0x30`.
    pub const VIEW: Global<[f32; 16]> = Global::new(0x0230_ff20);
    /// `g_pppViewMatrixCopy`: a copy of it the draw matrices use.
    pub const VIEW_COPY: Global<[f32; 16]> = Global::new(0x0230_fe20);
    /// `g_pppWoodMatrix`: the billboard rotation `pppDrawMatrixWoodSta`
    /// builds.
    pub const WOOD: Global<[f32; 16]> = Global::new(0x0230_fde0);
    /// `g_pppWoodMatrixWork`: where `pppDrawMatrixWoodSta` builds `WOOD`.
    pub const WOOD_WORK: Global<[f32; 16]> = Global::new(0x0230_fee0);
    /// `pppKeMatSN`'s two words (`0x00c3e6e0`).
    pub const MAT_SN: Global<[u32; 2]> = Global::new(0x00c3_e6e0);

    fn f(mem: &impl Mem, at: u32) -> f32 {
        mem.read(at)
    }

    fn at(p: u32, i: u32) -> u32 {
        p.wrapping_add(4 * i)
    }

    /// Work area `i` of `obj` (`obj + 0xa0 + offsets[i]`).
    fn area(mem: &impl Mem, obj: u32, ctrl: u32, i: u32) -> u32 {
        let offsets = mem.read::<u32>(ctrl.wrapping_add(0xc));
        obj.wrapping_add(0xa0)
            .wrapping_add(mem.read(offsets.wrapping_add(4 * i)))
    }

    fn rotation(mem: &impl Mem, axis: Axis, angle: i32) -> [f32; 16] {
        let f = match axis {
            Axis::X => crate::ppp::rot_x,
            Axis::Y => crate::ppp::rot_y,
            Axis::Z => crate::ppp::rot_z,
        };
        ppp::rotation(mem, angle, f).map(f32::from_bits)
    }

    /// `pppGetRotMatrix{XYZ,...}(out, angles)`: the rotations in `order`
    /// multiplied (`order` from [`super::order`]).
    pub fn ppp_get_rot_matrix(mem: &mut impl Mem, out: u32, angles: u32, order: [Axis; 3]) {
        let angles: [i32; 3] = mem.read(angles);
        let r = matrix_of(mem, angles, order);
        mem.write(out, r);
    }

    /// `s * m` (`fld s; fmul m` or the reverse: the same for numbers and
    /// quiet NaNs).
    fn scaled(s: f32, m: f32) -> f32 {
        Reg::load(s).mul_m32(m).store()
    }

    /// `pppMatrix{XYZ,XZY,YXZ,YZX,ZXY,ZYX}` and `pppMatrixLoop`
    /// `(obj, key, ctrl)`: the rotation of area 1's angles into the
    /// object's matrix, its rows scaled by area 2 (through `VF_SCALE` and
    /// `VF_ROWS`, `w` copied), the position from area 0.
    pub fn ppp_matrix(mem: &mut impl Mem, obj: u32, ctrl: u32, order: [Axis; 3]) {
        let pos = area(mem, obj, ctrl, 0);
        let angles = area(mem, obj, ctrl, 1);
        let scale = area(mem, obj, ctrl, 2);
        let m = obj.wrapping_add(0x10);
        ppp_get_rot_matrix(mem, m, angles, order);
        let s: [u32; 4] = mem.read(scale);
        VF_SCALE.set(mem, s);
        let mut rows: [u32; 12] = mem.read(m);
        VF_ROWS.set(mem, rows);
        for r in 0..3 {
            for c in 0..3 {
                let v = scaled(f32::from_bits(s[r]), f32::from_bits(rows[4 * r + c]));
                rows[4 * r + c] = v.to_bits();
            }
        }
        VF_ROWS.set(mem, rows);
        mem.write(m, rows);
        let x: u32 = mem.read(pos);
        mem.write(obj.wrapping_add(0x40), x);
        let y: u32 = mem.read(at(pos, 1));
        mem.write(obj.wrapping_add(0x44), y);
        let z = round_trip(f(mem, at(pos, 2)));
        mem.write(obj.wrapping_add(0x48), z);
    }

    /// Rows 0..3 of the matrix at `m` scaled by the three floats at
    /// `scale`, one at a time through `VF_SCALE.x` and the first row
    /// register (`pppKeMatPht`, `pppKeOfsMatXYZ`).
    fn scale_rows(mem: &mut impl Mem, m: u32, scale: u32) {
        for r in 0..3 {
            let s = round_trip(f(mem, at(scale, r)));
            mem.write(VF_SCALE.ptr(mem).addr, s);
            let row_at = at(m, 4 * r);
            let mut row: [u32; 4] = mem.read(row_at);
            let vf = VF_ROWS.ptr(mem).addr;
            mem.write(vf, row);
            for w in &mut row[..3] {
                *w = scaled(f32::from_bits(*w), s).to_bits();
            }
            mem.write(vf, row);
            mem.write(row_at, row);
        }
    }

    /// The position of area 0 into `dst` through the FPU.
    fn copy_xyz(mem: &mut impl Mem, dst: u32, src: u32) {
        for i in 0..3 {
            let v = round_trip(f(mem, at(src, i)));
            mem.write(at(dst, i), v);
        }
    }

    /// `pppMatrixLoc(obj, key, ctrl)` (`0x00734130`): the identity, the
    /// position from area 0.
    pub fn ppp_matrix_loc(mem: &mut impl Mem, obj: u32, ctrl: u32) {
        let pos = area(mem, obj, ctrl, 0);
        vu0::matrix_identity(mem, obj.wrapping_add(0x10));
        copy_xyz(mem, obj.wrapping_add(0x40), pos);
    }

    /// `pppMatrixScl(obj, key, ctrl)` (`0x00734170`): the identity scaled
    /// by area 1, the position from area 0.
    pub fn ppp_matrix_scl(mem: &mut impl Mem, obj: u32, ctrl: u32) {
        let pos = area(mem, obj, ctrl, 0);
        let scale = area(mem, obj, ctrl, 1);
        let m = obj.wrapping_add(0x10);
        vu0::matrix_identity(mem, m);
        diagonal(mem, m, scale);
        copy_xyz(mem, obj.wrapping_add(0x40), pos);
    }

    /// The three scale floats at `scale` onto the diagonal of `m`.
    fn diagonal(mem: &mut impl Mem, m: u32, scale: u32) {
        for i in 0..3 {
            let v = round_trip(f(mem, at(scale, i)));
            mem.write(at(m, 5 * i), v);
        }
    }

    /// `pppParMatrix(obj, key)` (`0x00734640`): unless the key's `+4` is
    /// -1, the object's position (`+0x40`, four words) transformed by its
    /// parent's matrix (`get_ptr32(obj + 4) + 0x10`, `w` taken as `vf00.w`),
    /// VU0's `mul`/`madd` sequence through the accumulator.
    pub fn ppp_par_matrix(mem: &mut impl Mem, obj: u32, key: u32) {
        if mem.read::<u32>(key.wrapping_add(4)) == u32::MAX {
            return;
        }
        let parent = stub::ptr32(mem.read(obj.wrapping_add(4)));
        let v: [u32; 4] = mem.read(obj.wrapping_add(0x40));
        VF_PAR_V.set(mem, v);
        let m: [u32; 16] = mem.read(parent.wrapping_add(0x10));
        VF_PAR_M.set(mem, m);
        let v = v.map(f32::from_bits);
        let m = m.map(f32::from_bits);
        let mut acc = [0.0f32; 4];
        for (c, a) in acc.iter_mut().enumerate() {
            *a = scaled(v[0], m[c]);
        }
        ACC.set(mem, acc.map(f32::to_bits));
        for (r, x) in [v[1], v[2]].into_iter().enumerate() {
            for (c, a) in acc.iter_mut().enumerate() {
                *a = Reg::load(x).mul_m32(m[4 * (r + 1) + c]).add_m32(*a).store();
            }
            ACC.set(mem, acc.map(f32::to_bits));
        }
        let w = f32::from_bits(VF00_W.get(mem));
        let out: [f32; 4] =
            std::array::from_fn(|c| Reg::load(w).mul_m32(m[12 + c]).add_m32(acc[c]).store());
        let out = out.map(f32::to_bits);
        VF_PAR.set(mem, out);
        mem.write(obj.wrapping_add(0x40), out);
    }

    /// `pppKeMatSN(obj, key, ctrl)` (`0x00734020`): area 0 keeps the angles
    /// at `+0x80`; it takes area 1's three there and builds the rotation
    /// (`XYZ`) of the old ones minus the new (as integers) into `+0x40`;
    /// `+0x70`/`+0x74` from `MAT_SN`.
    pub fn ppp_ke_mat_sn(mem: &mut impl Mem, obj: u32, ctrl: u32) {
        let w = area(mem, obj, ctrl, 0);
        let src = area(mem, obj, ctrl, 1);
        let old: [i32; 4] = mem.read(w.wrapping_add(0x80));
        for i in 0..3 {
            let v: u32 = mem.read(at(src, i));
            mem.write(at(w.wrapping_add(0x80), i), v);
        }
        let new: [i32; 3] = mem.read(w.wrapping_add(0x80));
        let d: [i32; 3] = std::array::from_fn(|i| old[i].wrapping_sub(new[i]));
        let r = matrix_of(mem, d, super::order::XYZ);
        mem.write(w.wrapping_add(0x40), r);
        let k = MAT_SN.get(mem);
        mem.write(w.wrapping_add(0x70), k[0]);
        mem.write(w.wrapping_add(0x74), k[1]);
    }

    /// The compound rotation of angles held here (not in game memory).
    fn matrix_of(mem: &impl Mem, angles: [i32; 3], order: [Axis; 3]) -> [f32; 16] {
        let a = rotation(mem, order[0], angles[order[0] as usize]);
        let b = rotation(mem, order[1], angles[order[1] as usize]);
        let t = math::mul4x4(&a, &b);
        let c = rotation(mem, order[2], angles[order[2] as usize]);
        math::mul4x4(&t, &c)
    }

    /// `pppKeMatPht(obj, key, ctrl)` (`0x007341e0`): the matrix in area 3
    /// (`ZYX` of area 1, rows scaled by area 2, the position from area 0);
    /// the object's matrix the scale alone with area 3's position.
    pub fn ppp_ke_mat_pht(mem: &mut impl Mem, obj: u32, ctrl: u32) {
        let pos = area(mem, obj, ctrl, 0);
        let angles = area(mem, obj, ctrl, 1);
        let scale = area(mem, obj, ctrl, 2);
        let m = area(mem, obj, ctrl, 3);
        ppp_get_rot_matrix(mem, m, angles, super::order::ZYX);
        scale_rows(mem, m, scale);
        copy_xyz(mem, m.wrapping_add(0x30), pos);
        let own = obj.wrapping_add(0x10);
        vu0::matrix_identity(mem, own);
        diagonal(mem, own, scale);
        vu0::copy_vector(mem, obj.wrapping_add(0x40), m.wrapping_add(0x30));
    }

    /// `pppKeOfsMatXYZ(obj, key, ctrl)` (`0x00734400`): the rotation `XYZ`
    /// of area 1, rows scaled by area 2, about the pivot `p = a0[+0x10] +
    /// a0[+0x20]`: the position is `a0 + p - M p`.
    pub fn ppp_ke_ofs_mat_xyz(mem: &mut impl Mem, obj: u32, ctrl: u32) {
        let pos = area(mem, obj, ctrl, 0);
        let angles = area(mem, obj, ctrl, 1);
        let scale = area(mem, obj, ctrl, 2);
        let m = obj.wrapping_add(0x10);
        ppp_get_rot_matrix(mem, m, angles, super::order::XYZ);
        scale_rows(mem, m, scale);
        let t = obj.wrapping_add(0x40);
        for i in [2, 1, 0] {
            mem.write(at(t, i), 0.0f32);
        }
        mem.write(at(t, 3), 1.0f32);
        let a: [f32; 4] = mem.read(pos.wrapping_add(0x10));
        let b: [f32; 4] = mem.read(pos.wrapping_add(0x20));
        let p: [f32; 4] = std::array::from_fn(|i| ffx_ps2::vu0::add(a[i], b[i]));
        let mat: [f32; 16] = mem.read(m);
        let q: [f32; 4] = std::array::from_fn(|i| {
            let col = [mat[i], mat[4 + i], mat[8 + i], mat[12 + i]];
            #[allow(clippy::cast_possible_truncation)] // 0..4
            math::mul4x4_component(i as u32, p, col)
        });
        for i in 0..3 {
            let v = Reg::load(f(mem, at(pos, i)))
                .add_m32(p[i as usize])
                .sub_m32(q[i as usize])
                .store();
            mem.write(at(t, i), v);
        }
        mem.write(at(t, 3), 1.0f32);
    }

    /// `pppDrawMatrix` (`0x00734b70`) and `pppDrawMatrixLoop`
    /// (`0x00734ba0`) `(obj, ...)`: mode 0 (`+0x9c`), the view matrix
    /// `+0x50` = the object's × `VIEW_COPY`.
    pub fn ppp_draw_matrix(mem: &mut impl Mem, obj: u32) {
        mem.write(obj.wrapping_add(0x9c), 0u8);
        let view = VIEW_COPY.ptr(mem).addr;
        math::bridge::mul4x4(mem, obj.wrapping_add(0x50), view, obj.wrapping_add(0x10));
    }

    /// The rows of the object's matrix scaled by the current part's scale
    /// (`+0x60`) into `+0x50`.
    fn scale_by_part(mem: &mut impl Mem, dst: u32, src: u32) {
        let part = CURRENT_PART.get(mem);
        for i in 0..3 {
            let s = round_trip(f(mem, part.wrapping_add(0x60 + 4 * i)));
            vu0::scale_vector(
                mem,
                dst.wrapping_add(0x10 * i),
                src.wrapping_add(0x10 * i),
                s,
            );
        }
    }

    fn view_position(mem: &mut impl Mem, obj: u32) {
        let view = VIEW.ptr(mem).addr;
        vu0::apply_matrix(mem, obj.wrapping_add(0x80), view, obj.wrapping_add(0x40));
    }

    /// `pppDrawMatrixFront(obj, ...)` (`0x00734bd0`): mode 1, the rows
    /// scaled by the part's scale, the position through `VIEW`.
    pub fn ppp_draw_matrix_front(mem: &mut impl Mem, obj: u32) {
        mem.write(obj.wrapping_add(0x9c), 1u8);
        scale_by_part(mem, obj.wrapping_add(0x50), obj.wrapping_add(0x10));
        view_position(mem, obj);
    }

    /// `pppDrawMatrixNoRot(obj, ...)` (`0x00734c50`): the rows scaled, the
    /// position scaled by the part's scale and moved by `VIEW`'s
    /// translation (no mode byte).
    pub fn ppp_draw_matrix_no_rot(mem: &mut impl Mem, obj: u32) {
        let part = CURRENT_PART.get(mem);
        scale_by_part(mem, obj.wrapping_add(0x50), obj.wrapping_add(0x10));
        let p = obj.wrapping_add(0x80);
        vu0::mul_vector(mem, p, obj.wrapping_add(0x40), part.wrapping_add(0x60));
        let t = VIEW.ptr(mem).addr.wrapping_add(0x30);
        let x = Reg::load(f(mem, t)).add_m32(f(mem, p)).store();
        mem.write(p, x);
        for i in 1..3 {
            let v = Reg::load(f(mem, at(p, i)))
                .add_m32(f(mem, at(t, i)))
                .store();
            mem.write(at(p, i), v);
        }
    }

    /// `pppDrawMatrixWood(obj, ...)` (`0x00734cf0`): mode 2, the rows
    /// scaled and turned by `WOOD`, the position through `VIEW_COPY`.
    pub fn ppp_draw_matrix_wood(mem: &mut impl Mem, obj: u32) {
        mem.write(obj.wrapping_add(0x9c), 2u8);
        let rows = obj.wrapping_add(0x50);
        scale_by_part(mem, rows, obj.wrapping_add(0x10));
        let p = obj.wrapping_add(0x80);
        vu0::copy_vector(mem, p, obj.wrapping_add(0x40));
        let wood = WOOD.ptr(mem).addr;
        math::bridge::mul4x4(mem, rows, wood, rows);
        let view = VIEW_COPY.ptr(mem).addr;
        vu0::apply_matrix(mem, p, view, obj.wrapping_add(0x40));
    }

    /// `pppKeDMatDraw(obj, key, ctrl)` (`0x00734eb0`): `+0x50` = the
    /// object's matrix × `VIEW`, copied into area 1.
    pub fn ppp_ke_d_mat_draw(mem: &mut impl Mem, obj: u32, ctrl: u32) {
        let a1 = area(mem, obj, ctrl, 1);
        let view = VIEW.ptr(mem).addr;
        math::bridge::mul4x4(mem, obj.wrapping_add(0x50), view, obj.wrapping_add(0x10));
        vu0::matrix_copy(mem, a1, obj.wrapping_add(0x50));
    }

    /// `pppKeDMatFrDraw(obj, key, ctrl)` (`0x00734ef0`): as
    /// [`ppp_draw_matrix_front`] without the mode byte, then the view
    /// matrix copied into area 1.
    pub fn ppp_ke_d_mat_fr_draw(mem: &mut impl Mem, obj: u32, ctrl: u32) {
        let a1 = area(mem, obj, ctrl, 1);
        scale_by_part(mem, obj.wrapping_add(0x50), obj.wrapping_add(0x10));
        view_position(mem, obj);
        vu0::matrix_copy(mem, a1, obj.wrapping_add(0x50));
    }

    /// `pppKeDMatPhtDraw(obj, key, ctrl)` (`0x00734f80`): area 1 = area 0's
    /// matrix × `VIEW`; then as [`ppp_ke_d_mat_fr_draw`]'s front part.
    pub fn ppp_ke_d_mat_pht_draw(mem: &mut impl Mem, obj: u32, ctrl: u32) {
        let a0 = area(mem, obj, ctrl, 0);
        let a1 = area(mem, obj, ctrl, 1);
        let view = VIEW.ptr(mem).addr;
        math::bridge::mul4x4(mem, a1, view, a0);
        scale_by_part(mem, obj.wrapping_add(0x50), obj.wrapping_add(0x10));
        view_position(mem, obj);
    }

    /// `pppKeDMatPhtFrDraw(obj, key, ctrl)` (`0x00735020`): the front
    /// treatment of area 0's matrix into area 1, then of the object's.
    pub fn ppp_ke_d_mat_pht_fr_draw(mem: &mut impl Mem, obj: u32, ctrl: u32) {
        let a0 = area(mem, obj, ctrl, 0);
        let a1 = area(mem, obj, ctrl, 1);
        scale_by_part(mem, a1, a0);
        let view = VIEW.ptr(mem).addr;
        vu0::apply_matrix(mem, a1.wrapping_add(0x30), view, a0.wrapping_add(0x30));
        scale_by_part(mem, obj.wrapping_add(0x50), obj.wrapping_add(0x10));
        view_position(mem, obj);
    }

    /// The broadcast factor of a VU0 macro helper: the stack argument at
    /// `ebp + slot` (the shim passes it).
    fn madd(v: f32, s: f32, acc: f32) -> f32 {
        (Reg::load(v) * Reg::load(s) + Reg::load(acc)).store()
    }

    /// `pppVu0MaddaY` (`0x0072c590`) and `pppVu0MaddaZ` (`0x0072c5f0`)
    /// `(_, v, ..s)`: the accumulator `+= v * s` (`vmadda` with a broadcast
    /// factor; `w` with `v`'s element as the memory operand).
    pub fn ppp_vu0_madda(mem: &mut impl Mem, v: u32, s: f32) {
        let acc = ACC.ptr(mem).addr;
        for i in 0..4 {
            let x: f32 = mem.read(at(v, i));
            let a: f32 = mem.read(at(acc, i));
            let r = if i == 3 {
                Reg::load(s).mul_m32(x).add_m32(a).store()
            } else {
                madd(x, s, a)
            };
            mem.write(at(acc, i), r);
        }
    }

    /// `pppVu0MaddW(dst, v, ..s)` (`0x0072c650`): `dst = acc + v * s`
    /// (`vmadd`).
    pub fn ppp_vu0_madd(mem: &mut impl Mem, dst: u32, v: u32, s: f32) {
        let acc = ACC.ptr(mem).addr;
        for i in 0..4 {
            let x: f32 = mem.read(at(v, i));
            let a: f32 = mem.read(at(acc, i));
            let r = if i == 3 {
                Reg::load(s).mul_m32(x).add_m32(a).store()
            } else {
                madd(x, s, a)
            };
            mem.write(at(dst, i), r);
        }
    }

    /// `pppVu0MulX(dst, v, s)` (`0x0072c6a0`) and `0x007500d0`: `dst = v *
    /// s`, element by element (`w` with `v`'s as the memory operand).
    pub fn ppp_vu0_mul(mem: &mut impl Mem, dst: u32, v: u32, s: f32) {
        let k = Reg::load(s);
        for i in 0..3 {
            let x: f32 = mem.read(at(v, i));
            mem.write(at(dst, i), (Reg::load(x) * k).store());
        }
        let w: f32 = mem.read(at(v, 3));
        mem.write(at(dst, 3), k.mul_m32(w).store());
    }

    /// `pppVu0AddY` (`0x0074b630`) and `pppVu0AddZ` (`0x0074b650`)
    /// `(dst, v, ..s)`: `dst.x = v.x + s`.
    pub fn ppp_vu0_add_x(mem: &mut impl Mem, dst: u32, v: u32, s: f32) {
        let x = Reg::load(f(mem, v)).add_m32(s).store();
        mem.write(dst, x);
    }

    /// The emulated R5900 `LO`/`HI` pairs `pppEePmulth` writes, in the
    /// order of its four words (`0x00c8f718`, `0x00c8f708`, `0x00c8f720`,
    /// `0x00c8f710`, the high halves' products 4 bytes after each).
    const PMULTH_LOHI: [u32; 4] = [0x00c8_f718, 0x00c8_f708, 0x00c8_f720, 0x00c8_f710];

    /// `pppEePmulth(dst, a, b)` (`0x007374e0`): the R5900's `PMULTH`:
    /// each word of `dst` the product of the low signed halves of `a`'s and
    /// `b`'s words; the products of both halves into the emulated
    /// `LO`/`HI`.
    pub fn ppp_ee_pmulth(mem: &mut impl Mem, dst: u32, a: u32, b: u32) {
        let x: [u32; 4] = mem.read(a);
        let y: [u32; 4] = mem.read(b);
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)] // the signed halves
        let half = |w: u32, hi: bool| i32::from((if hi { w >> 16 } else { w }) as u16 as i16);
        for i in 0..4 {
            let lo = half(x[i], false)
                .wrapping_mul(half(y[i], false))
                .cast_unsigned();
            let hi = half(x[i], true)
                .wrapping_mul(half(y[i], true))
                .cast_unsigned();
            let g = ffx_bridge::Global::<u32>::new(PMULTH_LOHI[i]).ptr(mem).addr;
            mem.write(g, lo);
            #[allow(clippy::cast_possible_truncation)] // 0..4
            mem.write(at(dst, i as u32), lo);
            mem.write(g.wrapping_add(4), hi);
        }
    }

    /// `pppEePpacb(dst, a, b)` (`0x007375d0`): the R5900's `PPACB`: the
    /// low bytes of `b`'s eight halves, then `a`'s, into `dst`'s 16 bytes.
    pub fn ppp_ee_ppacb(mem: &mut impl Mem, dst: u32, a: u32, b: u32) {
        let x: [u32; 4] = mem.read(a);
        let y: [u32; 4] = mem.read(b);
        let mut out = [0u8; 16];
        for i in 0..4 {
            for (k, w) in [(0, y[i]), (8, x[i])] {
                out[k + 2 * i] = w.to_le_bytes()[0];
                out[k + 2 * i + 1] = w.to_le_bytes()[2];
            }
        }
        mem.write(dst, out);
    }

    /// `g_pppWorldMatrix` (`0x0230ffa0`): the running part's matrix
    /// (`pppSetMatrix`).
    pub const WORLD: Global<[f32; 16]> = Global::new(0x0230_ffa0);

    /// `pppSetMatrix(part)` (`0x00717410`): `g_pppWorldMatrix` = the
    /// `XYZ` rotation of the part's angles (`+0x50`), rows scaled by its
    /// scale (`+0x60`, as `pppKeMatPht` scales), its position (`+0x40`).
    pub fn ppp_set_matrix(mem: &mut impl Mem, part: u32) {
        set_world(mem, part, super::order::XYZ);
    }

    /// [`ppp_set_matrix`] with the rotation order `order`.
    fn set_world(mem: &mut impl Mem, part: u32, order: [super::Axis; 3]) {
        let m = WORLD.ptr(mem).addr;
        ppp_get_rot_matrix(mem, m, part.wrapping_add(0x50), order);
        scale_rows(mem, m, part.wrapping_add(0x60));
        copy_xyz(mem, m.wrapping_add(0x30), part.wrapping_add(0x40));
    }

    /// `pppFpSetRotMatrix(part, mode)` (`0x0072a2e0`): [`ppp_set_matrix`]
    /// with the rotation order the mode's low byte picks (1 `XZY`, 2
    /// `YXZ`, 3 `YZX`, 4 `ZXY`, 5 `ZYX`, else `XYZ`). Returns the
    /// original's eax: the scaled row 2's `z`.
    pub fn ppp_fp_set_rot_matrix(mem: &mut impl Mem, part: u32, mode: u32) -> u32 {
        use super::order;
        let o = match mode & 0xff {
            1 => order::XZY,
            2 => order::YXZ,
            3 => order::YZX,
            4 => order::ZXY,
            5 => order::ZYX,
            _ => order::XYZ,
        };
        set_world(mem, part, o);
        mem.read(WORLD.ptr(mem).addr + 0x28)
    }

    /// `pppVertexAttend(obj, key, ctrl)` (`0x0075d340`): unless the key's
    /// `+4` is -1 or its shape (`i16` at `+0xc`) negative, area 1 takes
    /// vertex `n` of that shape (`n` from the shape's index list at area
    /// 0's `u16`), its three `i16` as floats through the parent's matrix
    /// (`get_ptr32(obj + 4) + 0x10`). The shapes are the current
    /// environment's tables: `+0x28` 8-byte records (`i16` vertex block,
    /// then the index list), `+0x20` 0x20-byte blocks (vertices at
    /// `+0x14`).
    pub fn ppp_vertex_attend(mem: &mut impl Mem, obj: u32, key: u32, ctrl: u32) {
        if mem.read::<u32>(key.wrapping_add(4)) == u32::MAX {
            return;
        }
        let shape = mem.read::<i16>(key.wrapping_add(0xc));
        if shape < 0 {
            return;
        }
        let a0 = area(mem, obj, ctrl, 0);
        let a1 = area(mem, obj, ctrl, 1);
        let parent = stub::ptr32(mem.read(obj.wrapping_add(4)));
        let env: u32 = mem.read(crate::heap::bridge::ENV_SLOT.get(mem));
        let records: u32 = mem.read(env.wrapping_add(0x28));
        let rec = records.wrapping_add((i32::from(shape) * 8).cast_unsigned());
        let block = i32::from(mem.read::<i16>(rec));
        let blocks: u32 = mem.read(env.wrapping_add(0x20));
        let verts = stub::ptr32(
            mem.read(
                blocks
                    .wrapping_add((block * 0x20).cast_unsigned())
                    .wrapping_add(0x14),
            ),
        );
        let list = stub::ptr32(mem.read(rec.wrapping_add(4)));
        let i = u32::from(mem.read::<u16>(a0));
        let n = u32::from(mem.read::<u16>(list.wrapping_add(2 * i)));
        let v: [f32; 4] = [
            f32::from(mem.read::<i16>(verts.wrapping_add(6 * n))),
            f32::from(mem.read::<i16>(verts.wrapping_add(6 * n + 2))),
            f32::from(mem.read::<i16>(verts.wrapping_add(6 * n + 4))),
            1.0,
        ];
        let m = parent.wrapping_add(0x10);
        for c in 0..3 {
            let col = [0, 4, 8, 12].map(|r| f(mem, at(m, r + c)));
            let x = math::mul4x4_component(c, v, col);
            mem.write(a1.wrapping_add(4 * c), x);
        }
    }

    /// `vfx_vec_w_add(dst, v, s)` (`0x00817cc0`): `dst.w = v.w + s`.
    pub fn vfx_vec_w_add(mem: &mut impl Mem, dst: u32, v: u32, s: f32) {
        let w = crate::x87::add(f(mem, v.wrapping_add(0xc)), s);
        mem.write(dst.wrapping_add(0xc), w);
    }

    /// `vfx_vec3_scale(dst, v, s)` (`0x0075bb00`): `dst.xyz = v.xyz * s`
    /// (x and y multiplied in registers, z by the memory operand).
    pub fn vfx_vec3_scale(mem: &mut impl Mem, dst: u32, v: u32, s: f32) {
        use crate::x87::{MUL, Reg};
        let r = Reg::load_f32(s);
        let x = Reg::load_f32(f(mem, v)).reg(MUL, r).store();
        let y = Reg::load_f32(f(mem, v.wrapping_add(4))).reg(MUL, r).store();
        let z = r.mem(MUL, f(mem, v.wrapping_add(8))).store();
        mem.write(dst, x);
        mem.write(dst.wrapping_add(4), y);
        mem.write(dst.wrapping_add(8), z);
    }

    /// `vfx_vec4_min(dst, v, s)` (`0x00808c10`): each component of `dst`
    /// the smaller of `v`'s and `s` (`s` unless `v`'s is less).
    pub fn vfx_vec4_min(mem: &mut impl Mem, dst: u32, v: u32, s: f32) {
        for i in 0..4 {
            let c = f(mem, v.wrapping_add(4 * i));
            let x = if c < s { c } else { crate::x87::load_f32(s) };
            mem.write(dst.wrapping_add(4 * i), x);
        }
    }

    /// `pppEePextlh(dst, rs, rt)` (`0x00737450`): the R5900's `PEXTLH`:
    /// the halves of the low two words of `rt` and `rs` interleaved (`rt`'s
    /// first).
    pub fn ppp_ee_pextlh(mem: &mut impl Mem, dst: u32, rs: u32, rt: u32) {
        let a: [u16; 4] = mem.read(rs);
        let b: [u16; 4] = mem.read(rt);
        let out = [b[0], a[0], b[1], a[1], b[2], a[2], b[3], a[3]];
        mem.write(dst, out);
    }

    /// `pppEePpach(dst, rs, rt)` (`0x007376a0`): the R5900's `PPACH`: the
    /// low halves of `rt`'s four words, then of `rs`'s.
    pub fn ppp_ee_ppach(mem: &mut impl Mem, dst: u32, rs: u32, rt: u32) {
        let a: [u16; 8] = mem.read(rs);
        let b: [u16; 8] = mem.read(rt);
        let out = [b[0], b[2], b[4], b[6], a[0], a[2], a[4], a[6]];
        mem.write(dst, out);
    }

    /// `pppEePsrlw(dst, src, sa)` (`0x00737700`): the R5900's `PSRLW` as
    /// the PC code does it: each word shifted arithmetically by `sa & 31`
    /// and masked with `0x7fffffff >> (sa - 1 & 31)` (so a shift of 0 gives
    /// 0).
    pub fn ppp_ee_psrlw(mem: &mut impl Mem, dst: u32, src: u32, sa: u32) {
        let s = sa & 0xff;
        let mask = 0x7fff_ffff_u32 >> (s.wrapping_sub(1) & 31);
        for i in 0..4 {
            let v = mem.read::<i32>(src.wrapping_add(4 * i));
            mem.write(
                dst.wrapping_add(4 * i),
                (v >> (s & 31)).cast_unsigned() & mask,
            );
        }
    }

    /// The emulated VU0 register `0x00c8f980` `pppVu0MoveVia` goes through.
    const VF_MOVE: Global<[u32; 4]> = Global::new(0x00c8_f980);

    /// `pppVu0MoveVia(dst, src)` (`0x0073ce30`): `src` into the emulated
    /// register at `0x00c8f980` and from there into `dst`.
    pub fn ppp_vu0_move_via(mem: &mut impl Mem, dst: u32, src: u32) {
        let v: [u32; 4] = mem.read(src);
        VF_MOVE.set(mem, v);
        mem.write(dst, v);
    }

    /// `pppVu0LoadVf00(dst)` (`0x0073ce80`, `0x0073e110`): `dst` = `vf00`.
    pub fn ppp_vu0_load_vf00(mem: &mut impl Mem, dst: u32) {
        let v = crate::prog::bridge::VF00.get(mem);
        mem.write(dst, v);
    }

    /// The emulated VU0 registers the length macro uses: `0x00c8f820`
    /// (squares, running sums) and `0x00c8f830` (the vector).
    const VF_LEN_ACC: u32 = 0x00c8_f820;
    const VF_LEN_V: u32 = 0x00c8_f830;
    /// `g_Vu0_Q`.
    const VU0_Q: Global<f32> = Global::new(0x00c8_f788);

    /// `pppVu0Length(v)` (`0x0073ceb0`, `0x0073d7c0`, `0x0073e140`): the
    /// emulated VU0 length macro: the vector into `0x00c8f830`, its squares
    /// there, `0x00c8f820` = (x², x² + y², x² + y² + z²), `Q` = the square
    /// root of the last (its absolute value), `0x00c8f830`'s x then
    /// `vf00.x + Q`, which is returned.
    pub fn ppp_vu0_length(mem: &mut impl Mem, v: u32) -> f32 {
        use crate::x87::{add, mul};
        let x: [f32; 4] = mem.read(v);
        let vr = Global::<[f32; 4]>::new(VF_LEN_V).ptr(mem).addr;
        let acc = Global::<[f32; 4]>::new(VF_LEN_ACC).ptr(mem).addr;
        mem.write(vr, x);
        let sq = [mul(x[0], x[0]), mul(x[1], x[1]), mul(x[2], x[2])];
        let s1 = add(sq[0], sq[1]);
        let s2 = add(sq[2], s1);
        mem.write(acc, sq[0]);
        mem.write(acc + 4, s1);
        mem.write(acc + 8, s2);
        // the square root in 53 bits rounded to 24 is the f32 square root
        let q = s2.abs().sqrt();
        VU0_Q.set(mem, q);
        let vf00x = f32::from_bits(crate::prog::bridge::VF00.get(mem)[0]);
        let r = add(vf00x, q);
        mem.write(vr, r);
        mem.write(vr + 4, sq[1]);
        mem.write(vr + 8, sq[2]);
        r
    }

    /// `pppColorShiftLoad(dst, base)` (`0x00737420`): two words from
    /// `base + g_pppColorConShift` into `dst`; the shift cleared.
    pub fn ppp_color_shift_load(mem: &mut impl Mem, dst: u32, base: u32) {
        let at = base.wrapping_add(crate::ppp::bridge::COLOR_CON_SHIFT.get(mem));
        let v: [u32; 2] = mem.read(at);
        mem.write(dst, v);
        crate::ppp::bridge::COLOR_CON_SHIFT.set(mem, 0);
    }

    /// `pppColorShiftStore(src, base)` (`0x00737750`): a word from `src`
    /// to `base + g_pppColorConShift`; the shift cleared.
    pub fn ppp_color_shift_store(mem: &mut impl Mem, src: u32, base: u32) {
        let at = base.wrapping_add(crate::ppp::bridge::COLOR_CON_SHIFT.get(mem));
        let v: u32 = mem.read(src);
        mem.write(at, v);
        crate::ppp::bridge::COLOR_CON_SHIFT.set(mem, 0);
    }

    /// `pppShapeIterInit(shape, it)` (`0x0073e990`, `0x0073f180`,
    /// `0x0073f9a0`): `it` = (`set_ptr32(shape)`, shape `+8`, 0, 0, shape
    /// `+0x14`, 0, 0).
    pub fn ppp_shape_iter_init(mem: &mut impl Mem, shape: u32, it: u32) {
        let a: u32 = mem.read(shape.wrapping_add(8));
        let b: u32 = mem.read(shape.wrapping_add(0x14));
        mem.write(it.wrapping_add(4), a);
        mem.write(it.wrapping_add(0x10), b);
        for k in [6u32, 5, 3, 2] {
            mem.write(it.wrapping_add(4 * k), 0u32);
        }
        mem.write(it, stub::ptr32(shape));
    }

    /// `pppShapeRandFlip(def, out)` (`0x007419e0`): `out` `u16` 1, then
    /// for bits 0 and 1 of the definition's byte `+0xf`, an `op_random(1.0)`
    /// draw above 0 sets the same bit of `out`'s high byte.
    pub fn ppp_shape_rand_flip(mem: &mut impl Mem, def: u32, out: u32) {
        mem.write(out, 1u16);
        let flags: u8 = mem.read(def.wrapping_add(0xf));
        for bit in [1u8, 2] {
            if flags & bit != 0 && crate::rand::bridge::op_random(mem, 1.0) > 0.0 {
                let b: u8 = mem.read(out.wrapping_add(1));
                mem.write(out.wrapping_add(1), b | bit);
            }
        }
    }

    /// `pppDrawMatrixWoodSta()` (`0x00734d90`): `WOOD`, a billboard
    /// rotation from `VIEW`: row 1 its `+0x10` row (`w` 0) normalised, row
    /// 0 `(y, -x, 0, 0)` of that normalised, row 2 their outer product
    /// normalised, row 3 `VIEW`'s translation; built in `0x0230fee0`, then
    /// copied (`FFXVu0CopyMatrix`, whose eax it returns). The three vectors
    /// are locals below `frame`.
    pub fn ppp_draw_matrix_wood_sta(mem: &mut impl Mem, frame: u32) -> u32 {
        let view = VIEW.ptr(mem).addr;
        let work = WOOD_WORK.ptr(mem).addr;
        let (v, b, c) = (
            frame.wrapping_sub(0x14),
            frame.wrapping_sub(0x24),
            frame.wrapping_sub(0x34),
        );
        for i in 0..3 {
            mem.write(at(v, i), round_trip(f(mem, at(view, 4 + i))));
        }
        mem.write(at(v, 3), 0.0f32);
        vu0::normalize(mem, v, v);
        let row: [u32; 4] = mem.read(v);
        mem.write(at(work, 4), row);
        mem.write(at(b, 0), round_trip(f(mem, at(v, 1))));
        mem.write(at(b, 1), (-Reg::load(f(mem, v))).store());
        mem.write(at(b, 2), 0.0f32);
        mem.write(at(b, 3), 0.0f32);
        vu0::normalize(mem, b, b);
        let row: [u32; 4] = mem.read(b);
        mem.write(work, row);
        vu0::outer_product(mem, c, b, v);
        vu0::normalize(mem, c, c);
        let row: [u32; 4] = mem.read(c);
        mem.write(at(work, 8), row);
        let row: [u32; 4] = mem.read(at(view, 12));
        mem.write(at(work, 12), row);
        vu0::matrix_copy(mem, WOOD.ptr(mem).addr, work)
    }

    /// `pppKeZCrctDraw(obj, key, ctrl)` (`0x00735160`): the model's draw
    /// matrix into work area 0. `0x00c8f820` = the key's `+0x20` (the
    /// scale), `0x00c8f830..0x00c8f85c` = the object's rows (`+0x10`),
    /// their `x, y, z` times the scale's (x for row 0, y row 1, z row 2;
    /// `f32` products); the local matrix at `frame - 0x44` (the original's
    /// frame pointer) gets those rows and row 3 = the object's `+0x40`
    /// (`FFXVu0CopyVector`). By the key's mode (`u8 +0x30`): 0 row 3 plus
    /// the key's `+0x10` (all four), then times `g_pppViewMatrix`
    /// (`FFXVu0MulMatrix(m, view, m)`); 1 times the view first, then plus;
    /// 2 times `g_pppWorldMatrix`, then `0x00c8f830` = the key's `+0x10`,
    /// `0x00c8f840` and `0x00c8f850` = the running part's `+0x60` (its
    /// scale), `0x00c8f830..38` = scale times key, `0x00c8f850..58` and row
    /// 3 `x, y, z` = scale plus that (`w` the scale's), then times
    /// `g_pppCameraMatrix`; others nothing more. Copied with
    /// `FFXVu0CopyMatrix`, whose `eax` it returns. Callees through `call`.
    pub fn ppp_ke_z_crct_draw<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        ctrl: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::{add, load, mul, store};
        let mut go = |mem: &mut M, at: u32, args: &[u32]| {
            let f = Global::<u32>::new(at).ptr(mem).addr;
            call(mem, f, args)
        };
        let g = |mem: &M, a: u32| Global::<u32>::new(a).ptr(mem).addr;
        let dst = area(mem, obj, ctrl, 0);
        let scale: [u32; 4] = mem.read(key.wrapping_add(0x20));
        mem.write(g(mem, 0x00c8_f820), scale);
        let work = g(mem, 0x00c8_f830);
        let mut rows: [u32; 12] = mem.read(obj.wrapping_add(0x10));
        mem.write(work, rows);
        for (r, &sv) in scale.iter().enumerate().take(3) {
            let s = load(sv);
            for c in 0..3 {
                let i = 4 * r + c;
                rows[i] = store(mul(s, load(rows[i])));
            }
        }
        mem.write(work, rows);
        let part = CURRENT_PART.get(mem);
        let m = frame.wrapping_sub(0x44);
        let last_row = m.wrapping_add(0x30);
        mem.write(m, rows);
        go(mem, 0x006e_d370, &[last_row, obj.wrapping_add(0x40)]);
        let mul_matrix = |mem: &mut M, go: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32, by: u32| {
            let by = g(mem, by);
            go(mem, 0x006e_d5c0, &[m, by, m]);
        };
        let key10: [u32; 4] = mem.read(key.wrapping_add(0x10));
        // row 3 plus the key's +0x10 (the x87's NaN choice is symmetric,
        // so the original's operand orders don't matter)
        let plus = |mem: &mut M| {
            let v: [u32; 4] = mem.read(last_row);
            let out: [u32; 4] = std::array::from_fn(|i| store(add(load(v[i]), load(key10[i]))));
            mem.write(last_row, out);
        };
        match mem.read::<u8>(key.wrapping_add(0x30)) {
            0 => {
                plus(mem);
                mul_matrix(mem, &mut go, 0x0230_ff20);
            }
            1 => {
                mul_matrix(mem, &mut go, 0x0230_ff20);
                plus(mem);
            }
            2 => {
                mul_matrix(mem, &mut go, 0x0230_ffa0);
                let s: [u32; 4] = mem.read(part.wrapping_add(0x60));
                let p: [u32; 3] = std::array::from_fn(|c| store(mul(load(s[c]), load(key10[c]))));
                let t: [u32; 3] = std::array::from_fn(|c| store(add(load(s[c]), load(p[c]))));
                mem.write(work, [p[0], p[1], p[2], key10[3]]);
                mem.write(work.wrapping_add(0x10), s);
                mem.write(work.wrapping_add(0x20), [t[0], t[1], t[2], s[3]]);
                mem.write(last_row, [t[0], t[1], t[2], s[3]]);
                mul_matrix(mem, &mut go, 0x0230_ff60);
            }
            _ => {}
        }
        go(mem, 0x006e_d360, &[dst, m])
    }

    /// `pppKeZCrctShpDraw(obj, key, ctrl)` (`0x007354c0`): the shape's
    /// draw matrix into work area 0, built on the emulated VU0 (whose
    /// registers keep the original's final values):
    /// - rows 0..2: the object's rows (`+0x10`) times the running part's
    ///   scale (`+0x60`, through `VF_SCALE`), `w` as is (left in
    ///   `VF_ROWS`, and row 2 stays in `0x00c8f850`);
    /// - `v` = the object's `+0x40` times the key's `+0x20` (xyz, `w` as
    ///   is), in `0x00c8f830`, the key's `+0x20` in `0x00c8f840`;
    /// - row 3 by the key's mode (`u8 +0x30`), the product through VU0's
    ///   `mul`/`madd` sequence ([`apply`]):
    ///   0: `v` plus the key's `+0x10` (all four), times `g_pppViewMatrix`;
    ///   1: `v` times `g_pppViewMatrix`, plus the key's `+0x10`;
    ///   2: `v` times `g_pppWorldMatrix` (into the VU0 registers only),
    ///   and row 3 = the part's scale plus the scale times the key's
    ///   `+0x10` (`w` the scale's), through `0x00c8f830..0x00c8f85c`;
    ///   other modes leave row 3 as the stack had it.
    ///
    /// The matrix is built on the stack at `frame - 0x44` (the original's
    /// frame pointer; row 3 at `frame - 0x14`) and copied with
    /// `FFXVu0CopyMatrix` through `call`.
    pub fn ppp_ke_z_crct_shp_draw<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        ctrl: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let dst = area(mem, obj, ctrl, 0);
        let part = CURRENT_PART.get(mem);
        let words = |mem: &M, p: u32| -> [u32; 4] { mem.read(p) };
        let floats = |w: [u32; 4]| w.map(f32::from_bits);
        let scale = words(mem, part.wrapping_add(0x60));
        VF_SCALE.set(mem, scale);
        // rows 0..2 scaled (x and z by the memory operand, y by the
        // register)
        let mut rows = [0u32; 12];
        for r in 0..3 {
            let s = f32::from_bits(scale[r as usize]);
            let row = floats(words(mem, obj.wrapping_add(0x10 + 0x10 * r)));
            let i = 4 * r as usize;
            rows[i] = scaled(s, row[0]).to_bits();
            rows[i + 1] = scaled(row[1], round_trip(s)).to_bits();
            rows[i + 2] = scaled(s, row[2]).to_bits();
            rows[i + 3] = row[3].to_bits();
        }
        // v = the object's +0x40 times the key's +0x20
        let o = floats(words(mem, obj.wrapping_add(0x40)));
        let k20 = words(mem, key.wrapping_add(0x20));
        let k = floats(k20);
        let v = [
            scaled(k[0], o[0]),
            scaled(k[1], o[1]),
            scaled(k[2], o[2]),
            o[3],
        ];
        let mut vf = rows;
        vf[0..4].copy_from_slice(&v.map(f32::to_bits));
        vf[4..8].copy_from_slice(&k20);
        let k10 = floats(words(mem, key.wrapping_add(0x10)));
        let w = f32::from_bits(VF00_W.get(mem));
        let mut last = None;
        match mem.read::<u8>(key.wrapping_add(0x30)) {
            0 => {
                let u = [
                    Reg::load(v[0]).add_m32(k10[0]).store(),
                    Reg::load(v[1]).add_m32(k10[1]).store(),
                    Reg::load(v[2]).add_m32(k10[2]).store(),
                    Reg::load(k10[3]).add_m32(v[3]).store(),
                ];
                VF_PAR_V.set(mem, u.map(f32::to_bits));
                last = Some(apply(mem, VIEW.get(mem), u, w));
            }
            1 => {
                VF_PAR_V.set(mem, v.map(f32::to_bits));
                let out = apply(mem, VIEW.get(mem), v, w);
                last = Some([
                    Reg::load(out[0]).add_m32(k10[0]).store(),
                    Reg::load(out[1]).add_m32(k10[1]).store(),
                    Reg::load(out[2]).add_m32(k10[2]).store(),
                    Reg::load(k10[3]).add_m32(out[3]).store(),
                ]);
            }
            2 => {
                VF_PAR_V.set(mem, v.map(f32::to_bits));
                apply(mem, WORLD.get(mem), v, w);
                let part = CURRENT_PART.get(mem);
                let sw = words(mem, part.wrapping_add(0x60));
                let s = floats(sw);
                let p: [f32; 3] = std::array::from_fn(|c| scaled(s[c], k10[c]));
                let t: [f32; 3] = std::array::from_fn(|c| Reg::load(s[c]).add_m32(p[c]).store());
                vf[0..4].copy_from_slice(&[p[0], p[1], p[2], k10[3]].map(f32::to_bits));
                vf[4..8].copy_from_slice(&sw);
                let r3 = [t[0], t[1], t[2], s[3]];
                vf[8..12].copy_from_slice(&r3.map(f32::to_bits));
                last = Some(r3);
            }
            _ => {}
        }
        VF_ROWS.set(mem, vf);
        let local = frame.wrapping_sub(0x44);
        mem.write(local, rows);
        if let Some(r3) = last {
            mem.write(local.wrapping_add(0x30), r3);
        }
        let copy = Global::<u32>::new(0x006e_d360).ptr(mem).addr;
        call(mem, copy, &[dst, local]);
    }

    /// The VU0 `mul`/`madd` product of the row vector `v` (with `w` as its
    /// fourth element) and `m`: `m` into `VF_PAR_M`, the partial sums in
    /// `ACC`, the result in `VF_PAR`, which it returns (each product exact,
    /// each sum rounded to `f32`).
    fn apply(mem: &mut impl Mem, m: [f32; 16], v: [f32; 4], w: f32) -> [f32; 4] {
        VF_PAR_M.set(mem, m.map(f32::to_bits));
        let mut acc: [f32; 4] = std::array::from_fn(|c| scaled(m[c], v[0]));
        for (r, x) in [v[1], v[2]].into_iter().enumerate() {
            for (c, a) in acc.iter_mut().enumerate() {
                *a = Reg::load(m[4 * (r + 1) + c]).mul_m32(x).add_m32(*a).store();
            }
        }
        ACC.set(mem, acc.map(f32::to_bits));
        let out: [f32; 4] =
            std::array::from_fn(|c| Reg::load(w).mul_m32(m[12 + c]).add_m32(acc[c]).store());
        VF_PAR.set(mem, out.map(f32::to_bits));
        out
    }

    /// `pppKeParMatR(obj)` (`0x00734880`): the object's matrix `+0x10`
    /// times its parent's rotation: the parent's (`get_ptr32(obj +4) +
    /// 0x10`) rows 0..2 normalised (`FFXVu0Normalize`) into a local matrix
    /// at `frame` (row 3 zero), `obj +0x40` moved by the parent's matrix
    /// (`FFXVu0ApplyMatrix` into `frame + 0x40`, copied back), and `obj
    /// +0x10 = FFXVu0MulMatrix(obj +0x10, local)` (through `call`).
    pub fn ppp_ke_par_mat_r<M: Mem>(
        mem: &mut M,
        obj: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use ffx_ps2::vu0::bridge as vu0;
        let m = ffx_system::stub::ptr32(mem.read(obj.wrapping_add(4))).wrapping_add(0x10);
        for row in 0..3 {
            vu0::normalize(
                mem,
                frame.wrapping_add(0x10 * row),
                m.wrapping_add(0x10 * row),
            );
        }
        mem.write(frame.wrapping_add(0x30), [0f32; 4]);
        let pos = obj.wrapping_add(0x40);
        let out = frame.wrapping_add(0x40);
        vu0::apply_matrix(mem, out, m, pos);
        let mine = obj.wrapping_add(0x10);
        call(
            mem,
            Global::<u32>::new(0x006e_d5c0).ptr(mem).addr,
            &[mine, frame, mine],
        );
        vu0::copy_vector(mem, pos, out);
    }

    /// `0x00741470(obj, _, prog)` (`pppShapeRevMatrix`, ours; the
    /// `pppDrawShapeRevDraw` set-up): with the work areas `w0`, `w1`, `w2`
    /// at `obj + 0xa0 + [[prog + 0xc] + 4k]`, the object's matrix `+0x10` =
    /// the identity (`FFXVu0UnitMatrix`) with the diagonal `w2`'s `x`
    /// (negated when byte `w0 +1` has bit 1), `y` (bit 0) and `z`; its
    /// position `+0x40..+0x4c` = `w1`'s `x, y, z` (`w` kept); the rows
    /// scaled by the current part's `+0x60`, `+0x64`, `+0x68` into `+0x50`,
    /// `+0x60`, `+0x70` (`FFXVu0ScaleVector`); `+0x80` = the position
    /// through `g_pppViewMatrix` (`FFXVu0ApplyMatrix`, whose eax it
    /// returns). Floats through the FPU (a signalling NaN comes out quiet).
    pub fn ppp_shape_rev_matrix<M: Mem>(mem: &mut M, obj: u32, prog: u32) -> u32 {
        use ffx_bridge::x87::{load, store};
        let offs: u32 = mem.read(prog.wrapping_add(0xc));
        let [o0, o1, o2]: [u32; 3] = mem.read(offs);
        let area = |o: u32| obj.wrapping_add(0xa0).wrapping_add(o);
        let (w0, w1, w2) = (area(o0), area(o1), area(o2));
        let part = CURRENT_PART.get(mem);
        let m = obj.wrapping_add(0x10);
        vu0::matrix_identity(mem, m);
        let f = |mem: &M, a: u32| load(mem.read(a));
        let neg = |x: f64, on: bool| if on { -x } else { x };
        let flags = mem.read::<u8>(w0.wrapping_add(1));
        let x = neg(f(mem, w2), flags & 2 != 0);
        mem.write(m, store(x));
        let flags = mem.read::<u8>(w0.wrapping_add(1));
        let y = neg(f(mem, w2.wrapping_add(4)), flags & 1 != 0);
        mem.write(obj.wrapping_add(0x24), store(y));
        let z = f(mem, w2.wrapping_add(8));
        mem.write(obj.wrapping_add(0x38), store(z));
        let pos = obj.wrapping_add(0x40);
        for k in 0..3 {
            let v = f(mem, w1.wrapping_add(4 * k));
            mem.write(pos.wrapping_add(4 * k), store(v));
        }
        for k in 0..3 {
            let s = f32::from_bits(store(load(mem.read(part.wrapping_add(0x60 + 4 * k)))));
            vu0::scale_vector(
                mem,
                obj.wrapping_add(0x50 + 0x10 * k),
                m.wrapping_add(0x10 * k),
                s,
            );
        }
        let view = VIEW.ptr(mem).addr;
        vu0::apply_matrix(mem, obj.wrapping_add(0x80), view, pos)
    }
}
