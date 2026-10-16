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
}
