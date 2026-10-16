//! The map camera: `rcbgSceneSetMatrix` and the projection and view
//! matrices it builds (`rcbgSetProjectionMatrixSub`, `rcbgSetViewMatrix`).
//!
//! The callees outside the module (the movie module's
//! `FFXSamp0ViewScreenMatrix`, the battle module's camera matrices, the
//! renderer's `graphicCamera*`, the CRT's `_CItan`/`_CIatan`) are passed in
//! as callbacks: `call` for word arguments and results, `fcall` for the
//! ones that return in `ST0` ([`FArgs`]).

use ffx_bridge::Global;

/// `_CItan`'s import thunk (`0x009497be`): `ST0` in and out.
pub const CI_TAN: u32 = 0x0094_97be;
/// `_CIatan`'s import thunk (`0x00949d5c`): `ST0` in and out.
pub const CI_ATAN: u32 = 0x0094_9d5c;
/// `FFXSamp0ViewScreenMatrix(out, d, sx, sy, cx, cy, s0, s1, near, far)`
/// (`0x0093d960`).
pub const VIEW_SCREEN_MATRIX: u32 = 0x0093_d960;
/// `MsCameraMatrix(out, eye, at, up, roll)` (`0x007c1be0`).
pub const MS_CAMERA_MATRIX: u32 = 0x007c_1be0;
/// `Sg_GetPlayer()` (`0x0082d860`).
pub const SG_GET_PLAYER: u32 = 0x0082_d860;
/// `Ch_SetClipZ(chr, z)` (`0x0082b150`): returns the old value in `ST0`.
pub const CH_SET_CLIP_Z: u32 = 0x0082_b150;
/// `graphicCameraSetFOVy(&radians)` (`0x006395e0`).
pub const CAMERA_SET_FOVY: u32 = 0x0063_95e0;
/// `graphicCameraSetAspectRatio(&ratio)` (`0x006395c0`).
pub const CAMERA_SET_ASPECT: u32 = 0x0063_95c0;
/// `graphicCameraSetWorldMatrix(matrix)` (`0x00639860`).
pub const CAMERA_SET_WORLD: u32 = 0x0063_9860;
/// `g_CameraMatrix` (`0x02311440`), what `rcbgSceneSetMatrix` hands the
/// renderer.
pub const CAMERA_MATRIX: u32 = 0x0231_1440;

/// The screen words at `0x01137000` (`g_Screens`), `f32` bits.
pub const SCREEN_0: Global<u32> = Global::new(0x0113_7000);
/// `0x01137004`.
pub const SCREEN_1: Global<u32> = Global::new(0x0113_7004);
/// The near plane (`0x01137008`).
pub const NEAR: Global<u32> = Global::new(0x0113_7008);
/// The far plane (`0x0113700c`).
pub const FAR: Global<u32> = Global::new(0x0113_700c);
/// The screen depth (`0x0113701c`).
pub const DEPTH: Global<u32> = Global::new(0x0113_701c);
/// The screen centre's x (`0x01137020`).
pub const CENTER_X: Global<u32> = Global::new(0x0113_7020);
/// The screen centre's y (`0x01137024`).
pub const CENTER_Y: Global<u32> = Global::new(0x0113_7024);
/// `g_BtlScreenOffset` (`0x01136fb8`) and the word after it.
pub const SCREEN_OFFSET: Global<[u32; 2]> = Global::new(0x0113_6fb8);

/// The `.rdata` doubles the camera code loads (`fld qword`).
pub mod rdata {
    use ffx_bridge::Global;
    /// 0.5 (`0x00b92220`).
    pub const HALF: Global<f64> = Global::new(0x00b9_2220);
    /// Degrees to radians, the `f32` value widened (`0x00b11330`).
    pub const DEG: Global<f64> = Global::new(0x00b1_1330);
    /// 2.0 (`0x00b92270`).
    pub const TWO: Global<f64> = Global::new(0x00b9_2270);
    /// 512.0 (`0x00b41d38`), the screen width.
    pub const WIDTH: Global<f64> = Global::new(0x00b4_1d38);
    /// 416.0 (`0x00b41d30`).
    pub const CLIP_HEIGHT: Global<f64> = Global::new(0x00b4_1d30);
    /// 448.0 (`0x00b50210`), the screen height.
    pub const HEIGHT: Global<f64> = Global::new(0x00b5_0210);
    /// 7/6 as an `f32` widened (`0x00b67698`), the pixel aspect.
    pub const ASPECT: Global<f64> = Global::new(0x00b6_7698);
    /// 65536.0 (`0x00b51558`).
    pub const FAR_SCALE: Global<f64> = Global::new(0x00b5_1558);
    /// 10.0 (`0x00b922b0`).
    pub const NEAR_SCALE: Global<f64> = Global::new(0x00b9_22b0);
    /// 256.0 (`0x00b922f0`).
    pub const HALF_WIDTH: Global<f64> = Global::new(0x00b9_22f0);
    /// 1000.0 as an `f32` (`0x00b1f2cc`), the player's clip distance.
    pub const CLIP_Z: Global<u32> = Global::new(0x00b1_f2cc);
}

/// Offsets in the camera block `rcbgSceneSetMatrix` reads.
pub mod view {
    /// `+0x64`: a y offset, `i32`.
    pub const Y_OFFSET: u32 = 0x64;
    /// `+0x68`: the eye.
    pub const EYE: u32 = 0x68;
    /// `+0x78`: the point looked at.
    pub const AT: u32 = 0x78;
    /// `+0x88`: the up vector (passed on to `MsCameraMatrix`).
    pub const UP: u32 = 0x88;
    /// `+0xa8`: the field of view in degrees; 0 leaves the matrices alone.
    pub const FOV: u32 = 0xa8;
    /// `+0xac`: the roll.
    pub const ROLL: u32 = 0xac;
    /// `+0xb0`: the x scale `rcbgSetProjectionMatrixSub` returned.
    pub const SCALE_X: u32 = 0xb0;
    /// `+0xb4`: it times the pixel aspect.
    pub const SCALE_Y: u32 = 0xb4;
    /// `+0xb8`: the screen centre's x offset.
    pub const OFFSET_X: u32 = 0xb8;
    /// `+0xbc`: the y offset added to [`Y_OFFSET`].
    pub const OFFSET_Y: u32 = 0xbc;
}

/// Offsets in the scratch `rcbgSceneSetMatrix` keeps on its stack (the
/// original's frame from `ebp - 0x98`), [`SCRATCH`] bytes.
pub mod scratch {
    /// The aspect ratio handed to the renderer.
    pub const ASPECT: u32 = 0x0;
    /// The vertical field of view handed to the renderer.
    pub const FOVY: u32 = 0x4;
    /// The projection's `256 / tan` (then plus 256: the camera's roll
    /// argument to `MsSetCameraMatrix`).
    pub const DISTANCE: u32 = 0x8;
    /// The projection matrix.
    pub const PROJECTION: u32 = 0x14;
    /// The view matrix.
    pub const VIEW: u32 = 0x54;
}
/// Bytes of [`scratch`].
pub const SCRATCH: u32 = 0x94;
/// Bytes of the work area `rcbgSceneSetMatrix` takes: [`scratch`] and the
/// view's two scaled vectors.
pub const WORK: u32 = SCRATCH + 0x20;

/// How a callee returning in `ST0` takes its arguments.
#[derive(Debug, Clone, Copy)]
pub enum FArgs<'a> {
    /// In `ST0` (the CRT's `_CI*` intrinsics), at full precision.
    St0(f64),
    /// In `ST1` and `ST0` (`_CIfmod(ST1, ST0)`: the first is `ST1`).
    St01(f64, f64),
    /// cdecl words.
    Stack(&'a [u32]),
}

/// [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::Mem;
    use ffx_bridge::x87::Reg;

    use super::{
        CAMERA_MATRIX, CAMERA_SET_ASPECT, CAMERA_SET_FOVY, CAMERA_SET_WORLD, CENTER_X, CENTER_Y,
        CH_SET_CLIP_Z, CI_ATAN, CI_TAN, DEPTH, FAR, FArgs, MS_CAMERA_MATRIX, NEAR, SCREEN_0,
        SCREEN_1, SCREEN_OFFSET, SG_GET_PLAYER, VIEW_SCREEN_MATRIX, rdata, scratch, view,
    };
    use crate::guide::{YN_MAP_SYSTEM, map, system};
    use crate::x87;

    fn f(bits: u32) -> Reg {
        Reg::load(f32::from_bits(bits))
    }

    fn bits(r: Reg) -> u32 {
        r.store().to_bits()
    }

    /// The map's scene the camera code reads: null when there is no map
    /// system or map (the original then reads from null + offset).
    fn scene(mem: &impl Mem) -> u32 {
        let s = YN_MAP_SYSTEM.get(mem);
        if s == 0 {
            return 0;
        }
        let m = mem.read::<u32>(s + system::MAP);
        if m == 0 {
            return 0;
        }
        mem.read::<u32>(m + map::SCENE)
    }

    /// `rcbgSetProjectionMatrixSub(out, fov, dx, dy, unused, distance)`
    /// (`0x009202e0`): `t = tan(fov / 2)` (degrees, `_CItan`), the x scale
    /// `512 / (depth * 2 * t)`, its y scale times 7/6; `FFXSamp0ViewScreenMatrix`
    /// into `out` with the screen centre moved by `dx`, `dy` and the near
    /// and far planes (times 10 and 2 * 65536 when the scene's `+0xfc` is
    /// set); `256 / t` to `distance` when it is not null. Returns the x scale
    /// (`ST0`).
    #[allow(clippy::too_many_arguments)]
    pub fn rcbg_set_projection_matrix_sub<M: Mem>(
        mem: &mut M,
        out: u32,
        fov: u32,
        dx: u32,
        dy: u32,
        distance: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        fcall: &mut impl FnMut(&mut M, u32, FArgs<'_>) -> f64,
    ) -> f32 {
        let s = scene(mem);
        let wide = s != 0 && mem.read::<u32>(s + 0xfc) != 0;
        let a = f(fov) * Reg::constant(rdata::HALF.get(mem)) * Reg::constant(rdata::DEG.get(mem));
        let tan = mem.rebase(CI_TAN);
        let t = Reg::constant(fcall(mem, tan, FArgs::St0(a.value()))).store();
        let depth = DEPTH.get(mem);
        let d2 = bits(f(depth) * Reg::constant(rdata::TWO.get(mem)) * Reg::load(t));
        let sx = bits(Reg::constant(rdata::WIDTH.get(mem)) / f(d2));
        let sy = bits(Reg::constant(rdata::ASPECT.get(mem)) * f(sx));
        let (near, far) = if wide {
            let far = f(FAR.get(mem)) * Reg::constant(rdata::FAR_SCALE.get(mem));
            let far = bits(Reg::constant(rdata::TWO.get(mem)) * far);
            let near = bits(f(NEAR.get(mem)) * Reg::constant(rdata::NEAR_SCALE.get(mem)));
            (near, far)
        } else {
            (x87::copy(NEAR.get(mem)), x87::copy(FAR.get(mem)))
        };
        let s1 = x87::copy(SCREEN_1.get(mem));
        let s0 = x87::copy(SCREEN_0.get(mem));
        let cy = bits(f(CENTER_Y.get(mem)).add_m32(f32::from_bits(dy)));
        let cx = bits(f(CENTER_X.get(mem)).add_m32(f32::from_bits(dx)));
        let m = mem.rebase(VIEW_SCREEN_MATRIX);
        call(
            mem,
            m,
            &[out, x87::copy(depth), sx, sy, cx, cy, s0, s1, near, far],
        );
        if distance != 0 {
            let v = Reg::constant(rdata::HALF_WIDTH.get(mem)) / Reg::load(t);
            mem.write(distance, v.store());
        }
        f32::from_bits(sx)
    }

    /// `rcbgSetViewMatrix(out, eye, at, up, roll)` (`0x009204b0`): `eye`
    /// and `at` scaled by the scene's `+0xf4` (`rcbgScaleVector`, into the
    /// original's stack: here `work`, 0x20 bytes) and `MsCameraMatrix`
    /// into `out`.
    #[allow(clippy::too_many_arguments)]
    pub fn rcbg_set_view_matrix<M: Mem>(
        mem: &mut M,
        out: u32,
        eye: u32,
        at: u32,
        up: u32,
        roll: u32,
        work: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let s = scene(mem);
        let k = x87::copy(mem.read::<u32>(s.wrapping_add(0xf4)));
        let (e, a) = (work, work + 0x10);
        crate::vu0::bridge::rcbg_scale_vector(mem, e, eye, k);
        crate::vu0::bridge::rcbg_scale_vector(mem, a, at, k);
        let f = mem.rebase(MS_CAMERA_MATRIX);
        call(mem, f, &[out, e, a, up, x87::copy(roll)])
    }

    /// `rcbgSceneSetMatrix(v)` (`0x00920c00`): with a field of view, the
    /// player's clip distance to 1000 (`Ch_SetClipZ`), the projection
    /// (`rcbgSetProjectionMatrixSub`) and its scales to `+0xb0`/`+0xb4`,
    /// the battle screen offset, the clip matrix (`rcbgSetClipMatrix`), the
    /// view (`rcbgSetViewMatrix`), both to `MsSetCameraMatrix`, and the
    /// renderer's field of view (`2 atan(448 / (2 sy) / depth)`), aspect
    /// ratio and camera matrix. `work` is the original's stack
    /// ([`super::scratch`], [`super::SCRATCH`] bytes, plus 0x20 for the
    /// view's scaled vectors).
    pub fn rcbg_scene_set_matrix<M: Mem>(
        mem: &mut M,
        v: u32,
        work: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        fcall: &mut impl FnMut(&mut M, u32, FArgs<'_>) -> f64,
    ) {
        let fov = mem.read::<u32>(v + view::FOV);
        if f32::from_bits(fov) == 0.0 {
            return;
        }
        let get = mem.rebase(SG_GET_PLAYER);
        let player = call(mem, get, &[]);
        if player != 0 {
            let clip = x87::copy(rdata::CLIP_Z.get(mem));
            let set = mem.rebase(CH_SET_CLIP_Z);
            fcall(mem, set, FArgs::Stack(&[player, clip]));
        }
        let y = mem.read::<i32>(v + view::Y_OFFSET);
        let y = bits(Reg::constant(f64::from(y)));
        let dy = bits(f(y).add_m32(mem.read(v + view::OFFSET_Y)));
        let dx = x87::copy(mem.read::<u32>(v + view::OFFSET_X));
        let projection = work + scratch::PROJECTION;
        let distance = work + scratch::DISTANCE;
        let sx = rcbg_set_projection_matrix_sub(
            mem,
            projection,
            x87::copy(fov),
            dx,
            dy,
            distance,
            call,
            fcall,
        );
        let offset_x = x87::copy(mem.read::<u32>(v + view::OFFSET_X));
        let y = mem.read::<i32>(v + view::Y_OFFSET);
        let y = bits(Reg::constant(f64::from(y)));
        let offset_y = bits(f(y).add_m32(mem.read(v + view::OFFSET_Y)));
        SCREEN_OFFSET.set(mem, [offset_x, offset_y]);
        mem.write(v + view::SCALE_X, sx);
        let sy = (Reg::constant(rdata::ASPECT.get(mem)) * Reg::load(sx)).store();
        mem.write(v + view::SCALE_Y, sy);
        let far = f32::from_bits(x87::copy(FAR.get(mem)));
        let near = f32::from_bits(x87::copy(NEAR.get(mem)));
        let depth = f32::from_bits(x87::copy(DEPTH.get(mem)));
        let h = (Reg::constant(rdata::CLIP_HEIGHT.get(mem)) / Reg::load(sy)).store();
        let w = (Reg::constant(rdata::WIDTH.get(mem)) / Reg::load(sx)).store();
        crate::geom::bridge::rcbg_set_clip_matrix(mem, w, h, depth, near, far);
        let view_m = work + scratch::VIEW;
        let roll = mem.read::<u32>(v + view::ROLL);
        rcbg_set_view_matrix(
            mem,
            view_m,
            v + view::EYE,
            v + view::AT,
            v + view::UP,
            roll,
            work + super::SCRATCH,
            call,
        );
        let d: f32 = mem.read(distance);
        let d = (Reg::load(d) + Reg::constant(rdata::HALF_WIDTH.get(mem))).store();
        mem.write(distance, d);
        let set = mem.rebase(crate::guide::MS_SET_CAMERA_MATRIX);
        call(
            mem,
            set,
            &[
                1,
                projection,
                view_m,
                v + view::AT,
                v + view::EYE,
                d.to_bits(),
            ],
        );
        let sy: f32 = mem.read(v + view::SCALE_Y);
        let two = Reg::constant(rdata::TWO.get(mem));
        let q = (Reg::constant(rdata::HEIGHT.get(mem)) / (Reg::load(sy) * two)).store();
        let r = Reg::load(q).div_m32(f32::from_bits(DEPTH.get(mem)));
        let atan = mem.rebase(CI_ATAN);
        let a = Reg::constant(fcall(mem, atan, FArgs::St0(r.value()))).store();
        let fovy = (Reg::load(a) * two).store();
        mem.write(work + scratch::FOVY, fovy);
        let sx: f32 = mem.read(v + view::SCALE_X);
        let s = (Reg::constant(rdata::WIDTH.get(mem)) / two.mul_m32(sx)).store();
        let aspect = Reg::load(s).div_m32(q).store();
        mem.write(work + scratch::ASPECT, aspect);
        let f = mem.rebase(CAMERA_SET_FOVY);
        call(mem, f, &[work + scratch::FOVY]);
        let f = mem.rebase(CAMERA_SET_ASPECT);
        call(mem, f, &[work + scratch::ASPECT]);
        let f = mem.rebase(CAMERA_SET_WORLD);
        let m = mem.rebase(CAMERA_MATRIX);
        call(mem, f, &[m]);
    }
}
