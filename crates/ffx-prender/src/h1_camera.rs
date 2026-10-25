//! Camera projections and a class test (M4 port round 3, h1-render-a):
//! the perspective and orthographic projection-matrix builders, the two
//! cameras' `updateProjection` methods and the perspective camera's
//! constructor, which hand the builders a matrix in their own frame, and
//! the parameter buffer's "is it of this class" test, which hands a
//! virtual method two of its locals.
//!
//! The builders keep their `f32` temporaries in their own argument slots;
//! the bridges return those slots' last values for the shims to write
//! back (`ported!`'s `slots`).

use ffx_bridge::Mem;
use ffx_bridge::x87::{Reg, round_trip};
use ffx_graphic::host::{Conv, Host};

/// `_CItan`'s import thunk: `ST0` in and out.
pub const CI_TAN: u32 = 0x0094_97be;
/// `f64` 0.5 (`fmul qword`).
pub const HALF: u32 = 0x00b9_2220;
/// `f64` pi/2 as an `f32` (`k_HalfPiF64`).
pub const HALF_PI: u32 = 0x00b1_1338;
/// `f32` -1.0 (`g_ChrClothFlip`): the perspective matrix's `[11]`.
pub const MINUS_ONE: u32 = 0x00b9_2340;
/// `f32` pi/4: the perspective camera's default field of view.
pub const DEFAULT_FOV: u32 = 0x00b1_1328;
/// `PMatrix4_Copy_00453340(out, m)` (thiscall, `ret 4`) -> `out`.
pub const MATRIX4_COPY: u32 = 0x0045_3340;
/// `0x004530a0(this)` (thiscall): the camera base's constructor, on
/// `this + 4`.
pub const CAMERA_BASE_CTOR: u32 = 0x0045_30a0;
/// `Phyre::PCameraPerspective::vftable`.
pub const PERSPECTIVE_VTABLE: u32 = 0x00b1_1240;
/// The class descriptor `0x004d5f30` looks for in a class's parent chain.
pub const WANTED_CLASS: u32 = 0x00c9_b980;

/// The camera's fields the projections read.
pub mod camera {
    /// The near plane (`f32`).
    pub const NEAR: u32 = 0xb8;
    /// The far plane (`f32`).
    pub const FAR: u32 = 0xbc;
    /// The aspect ratio (`f32`), or the orthographic half width's factor.
    pub const ASPECT: u32 = 0x150;
    /// The field of view (`f32`, radians), or the orthographic half height.
    pub const FOV: u32 = 0x154;
    /// The projection matrix (16 `f32`s).
    pub const PROJECTION: u32 = 0x34;
}

/// Bytes of the cameras' frames: the matrix (`ebp-0x40`), and for the
/// orthographic camera the half width below it (`ebp-0x44` matrix, `ebp-4`
/// the half width).
pub const PERSPECTIVE_FRAME: u32 = 0x40;
/// The orthographic camera's frame.
pub const ORTHO_FRAME: u32 = 0x44;
/// `0x004d5f30`'s frame: the two words the virtual method fills
/// (`ebp-8`, `ebp-4`).
pub const IS_A_FRAME: u32 = 8;

fn f(bits: u32) -> Reg {
    Reg::load(f32::from_bits(bits))
}

fn bits(r: Reg) -> u32 {
    r.store().to_bits()
}

pub mod bridge {
    //! The originals over game memory.

    use super::{
        CAMERA_BASE_CTOR, CI_TAN, Conv, DEFAULT_FOV, HALF, HALF_PI, Host, MATRIX4_COPY, MINUS_ONE,
        Mem, PERSPECTIVE_VTABLE, Reg, WANTED_CLASS, bits, camera, f, round_trip,
    };

    /// The original's `f64` constant at `sym`.
    fn constant(m: &impl Mem, sym: u32) -> Reg {
        let at = m.rebase(sym);
        Reg::constant(m.read::<f64>(at))
    }

    /// `0x00456b80(out, fov, aspect, near, far)` (cdecl) -> `out`: the
    /// perspective matrix, `c = cot(fov / 2)` (as `tan(pi/2 - fov * 0.5)`,
    /// `_CItan`) and `r = 1 / (near - far)`:
    /// `[c / aspect, 0, 0, 0, 0, c, 0, 0, 0, 0, (near + far) r, -1, 0, 0,
    /// 2 near far r, 0]`.
    ///
    /// The second result: the last values of the four float argument slots
    /// (the original's temporaries; `aspect`'s is unchanged).
    pub fn perspective(
        h: &mut impl Host,
        out: u32,
        fov: u32,
        aspect: u32,
        near: u32,
        far: u32,
    ) -> (u32, [u32; 4]) {
        let half_angle = constant(h, HALF_PI) - f(fov) * constant(h, HALF);
        let angle = bits(half_angle);
        let tan = h.rebase(CI_TAN);
        let cot = h.call_intrinsic(tan, f(angle).value()).to_bits();
        let (n, fr) = (f(near), f(far));
        let r = bits(Reg::ONE / (n - fr));
        let nfr = n * fr * f(r);
        let depth_scale = bits(nfr + nfr);
        let depth_sum = bits((fr + n) * f(r));
        let x_scale = bits(f(cot).div_m32(f32::from_bits(aspect)));
        h.write(out, round_trip(f32::from_bits(x_scale)));
        let zero = 0f32;
        for k in [1, 2, 3, 4, 6, 7, 8, 9, 12, 13, 15] {
            h.write(out + 4 * k, zero);
        }
        h.write(out + 0x14, f(cot).store());
        h.write(out + 0x28, round_trip(f32::from_bits(depth_sum)));
        let minus_one: f32 = h.read(h.rebase(MINUS_ONE));
        h.write(out + 0x2c, round_trip(minus_one));
        h.write(out + 0x38, round_trip(f32::from_bits(depth_scale)));
        (out, [x_scale, aspect, depth_sum, depth_scale])
    }

    /// `0x00455dc0(out, left, right, bottom, top, near, far)` (cdecl) ->
    /// `out`: the orthographic matrix, with `a = 1 / (right - left)`,
    /// `b = 1 / (top - bottom)`, `c = 1 / (near - far)` (each rounded to
    /// `f32`): `[2a, 0, 0, 0, 0, 2b, 0, 0, 0, 0, 2c, 0, -(right + left) a,
    /// -(top + bottom) b, (far + near) c, 1]`.
    ///
    /// The second result: the last values of the six float argument slots.
    pub fn orthographic(m: &mut impl Mem, out: u32, args: [u32; 6]) -> (u32, [u32; 6]) {
        let [left, right, bottom, top, near, far] = args.map(f);
        let a = bits(Reg::ONE / (right - left));
        let b = bits(Reg::ONE / (top - bottom));
        let c = bits(Reg::ONE / (near - far));
        let x_sum = bits(right + left);
        let x_offset = bits(-f(x_sum) * f(a));
        let y_sum = bits(top + bottom);
        let y_offset = bits(-f(y_sum) * f(b));
        let z_sum = bits(far + near);
        let z_offset = bits(f(z_sum) * f(c));
        let z_scale = bits(f(c) + f(c));
        let y_scale = bits(f(b) + f(b));
        let x_scale = bits(f(a) + f(a));
        let copy = |x: u32| round_trip(f32::from_bits(x));
        m.write(out, copy(x_scale));
        for k in [1, 2, 3, 4, 6, 7, 8, 9, 11] {
            m.write(out + 4 * k, 0f32);
        }
        m.write(out + 0x14, copy(y_scale));
        m.write(out + 0x28, copy(z_scale));
        m.write(out + 0x30, copy(x_offset));
        m.write(out + 0x34, copy(y_offset));
        m.write(out + 0x38, copy(z_offset));
        m.write(out + 0x3c, 1f32);
        (
            out,
            [z_scale, y_scale, z_offset, y_offset, x_scale, x_offset],
        )
    }

    /// The camera's projection copied in: `PMatrix4_Copy(this + 0x34,
    /// matrix)`'s `eax`.
    fn copy_projection(h: &mut impl Host, this: u32, matrix: u32) -> u32 {
        h.call_sym(
            MATRIX4_COPY,
            Conv::Thiscall(this + camera::PROJECTION),
            &[matrix],
        )
    }

    /// A float field as the original pushes it (`fld`/`fstp`).
    fn field(h: &impl Host, at: u32) -> u32 {
        round_trip(h.read::<f32>(at)).to_bits()
    }

    /// `PCameraPerspective::updateProjection(this)` (`0x00456d10`, slot 1
    /// of the vtable): [`perspective`] of the camera's field of view,
    /// aspect, near and far into `frame` (the original's `ebp-0x40`), then
    /// copied to the camera's matrix; `eax` = the copy's.
    pub fn update_perspective(h: &mut impl Host, frame: u32, this: u32) -> u32 {
        let far = field(h, this + camera::FAR);
        let near = field(h, this + camera::NEAR);
        let aspect = field(h, this + camera::ASPECT);
        let fov = field(h, this + camera::FOV);
        perspective(h, frame, fov, aspect, near, far);
        copy_projection(h, this, frame)
    }

    /// `PCameraOrthographic::updateProjection(this)` (`0x00455fe0`, slot 1
    /// of the vtable): `w = aspect * height` (`f32`, the original's
    /// `ebp-4`, `frame + 0x40`), [`orthographic`] of `(-w, w, -height,
    /// height, near, far)` into `frame` (`ebp-0x44`), then copied to the
    /// camera's matrix; `eax` = the copy's.
    pub fn update_orthographic(h: &mut impl Host, frame: u32, this: u32) -> u32 {
        let aspect: f32 = h.read(this + camera::ASPECT);
        let height: f32 = h.read(this + camera::FOV);
        let w = Reg::load(aspect).mul_m32(height).store();
        h.write(frame + 0x40, w);
        let far = field(h, this + camera::FAR);
        let near = field(h, this + camera::NEAR);
        let top = field(h, this + camera::FOV);
        let bottom = bits(-f(field(h, this + camera::FOV)));
        let w = Reg::load(h.read::<f32>(frame + 0x40));
        let right = bits(w);
        let left = bits(-w);
        orthographic(h, frame, [left, right, bottom, top, near, far]);
        copy_projection(h, this, frame)
    }

    /// `PCameraPerspective::PCameraPerspective(this)` (`0x00456370`) ->
    /// `this`: the base constructor on `this + 4`, aspect 1, field of view
    /// pi/4, the vtable, then the projection as
    /// [`update_perspective`]'s.
    pub fn construct_perspective(h: &mut impl Host, frame: u32, this: u32) -> u32 {
        h.call_sym(CAMERA_BASE_CTOR, Conv::Thiscall(this + 4), &[]);
        let fov: f32 = h.read(h.rebase(DEFAULT_FOV));
        h.write(this + camera::ASPECT, 1f32);
        h.write(this + camera::FOV, round_trip(fov));
        let vtable = h.rebase(PERSPECTIVE_VTABLE);
        h.write(this, vtable);
        let far = field(h, this + camera::FAR);
        let near = field(h, this + camera::NEAR);
        let fov = round_trip(fov).to_bits();
        perspective(h, frame, fov, 1f32.to_bits(), near, far);
        copy_projection(h, this, frame);
        this
    }

    /// `0x004d5f30(this, x)` (thiscall, `ret 4`): virtual method 4 of
    /// `this` fills two words (`frame`, `frame + 4`, the original's
    /// `ebp-8` and `ebp-4`, zeroed first) from `x`: an object and its
    /// class. The object when that class (the class descriptor of `+0x40`
    /// links, a null class taken as [`WANTED_CLASS`]) is or derives from
    /// [`WANTED_CLASS`]; else, or without an object, 0.
    pub fn find_of_class(h: &mut impl Host, frame: u32, this: u32, x: u32) -> u32 {
        h.write(frame, 0u32);
        h.write(frame + 4, 0u32);
        h.call_virtual(this, 4, &[x, frame, frame + 4]);
        let object: u32 = h.read(frame);
        if object == 0 {
            return 0;
        }
        let wanted = h.rebase(WANTED_CLASS);
        let mut class: u32 = h.read(frame + 4);
        if class == 0 {
            class = wanted;
            h.write(frame + 4, class);
        }
        loop {
            if class == wanted {
                return object;
            }
            class = h.read(class + 0x40);
            if class == 0 {
                return 0;
            }
        }
    }
}
