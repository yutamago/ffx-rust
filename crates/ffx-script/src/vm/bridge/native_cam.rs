//! The camera natives in Rust (group 6, `0x6000`-`0x6088`). Two shapes:
//! most hand `(worker, args, stack)` on to a camera helper with one or two
//! constants that pick the half (0 the focus point, 1 the eye) and the mode;
//! the rest take the worker's camera handle ([`cam`]: cached in the
//! worker's `+0x3c`, else asked of `MsAtelGetCameraID`), pop what they need
//! and call the camera module with it.

// w, a, s: the worker, the args words and the stack, as everywhere
#![allow(clippy::many_single_char_names)]

use ffx_bridge::x87;

use super::native::{DEBUG_MODE, Native};
use super::native_map::pop_st0;
use super::run::NativeHost;
use super::{
    atel_get_actor_id, atel_get_basic_work, atel_get_camera_work_adrs,
    atel_get_event_map_camera_id, atel_pop_stack_integer,
};

fn pop(h: &mut impl NativeHost, s: u32) -> u32 {
    atel_pop_stack_integer(h, s)
}

fn call(h: &mut impl NativeHost, sym: u32, args: &[u32]) -> u32 {
    let f = h.rebase(sym);
    h.call(f, args)
}

/// `MsAtelGetCameraID` (`0x007ba780`).
pub const MS_ATEL_GET_CAMERA_ID: u32 = 0x007b_a780;
/// `g_BtlSplines` (`0x01136b60`, `[u32; 2]`), cleared by `0x6048`/`0x604a`.
pub const BTL_SPLINES: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x0113_6b60);
/// The `printf` import (`0x00b0c380`) the debug lines go through.
pub const PRINTF_IAT: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x00b0_c380);
/// `"MsCameraSetScrDpt (%d)\n"` (`0x00b550b0`).
pub const FMT_SET_SCR_DPT: u32 = 0x00b5_50b0;

/// The worker's camera handle: its `+0x3c` slot, filled on first use with
/// `MsAtelGetCameraID(5, worker is the event map's camera actor, 0)`.
pub fn cam(h: &mut impl NativeHost, w: u32) -> u32 {
    let slot = atel_get_camera_work_adrs(w);
    let c = h.read::<u32>(slot);
    if c != 0 {
        return c;
    }
    let id = atel_get_actor_id(h, w);
    let m = atel_get_event_map_camera_id(h);
    let c = call(h, MS_ATEL_GET_CAMERA_ID, &[5, u32::from(m == id), 0]);
    h.write(slot, c);
    c
}

/// `(w, a, s)` on to `f` with the extra words; returns 0.
macro_rules! fwd0 {
    ($($(#[$m:meta])* $name:ident => $f:expr, [$($k:expr),*];)*) => {$(
        $(#[$m])*
        pub fn $name<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
            call(h, $f, &[w, a, s, $($k),*]);
            0
        }
    )*};
}

/// `(w, a, s)` on to `f` with the extra words; returns its `EAX`.
macro_rules! fwde {
    ($($(#[$m:meta])* $name:ident => $f:expr, [$($k:expr),*];)*) => {$(
        $(#[$m])*
        pub fn $name<H: NativeHost>(h: &mut H, w: u32, a: u32, s: u32) -> u32 {
            call(h, $f, &[w, a, s, $($k),*])
        }
    )*};
}

/// `f(cam, k)`; returns the handle or 0.
macro_rules! handle {
    ($($(#[$m:meta])* $name:ident => $f:expr, $k:expr, $ret:expr;)*) => {$(
        $(#[$m])*
        pub fn $name<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
            let c = cam(h, w);
            call(h, $f, &[c, $k]);
            if $ret { c } else { 0 }
        }
    )*};
}

fwd0! {
    /// `Native_CamRefSetPos` (`0x007b9d00`, `0x6020`): `0x007bb6a0(w, a, s, 0)`; 0.
    native_cam_ref_set_pos => 0x007b_b6a0, [0];
    /// `Native_CamSetPolar` (`0x007b92e0`, `0x6004`): `0x007bb5d0(w, a, s, 1, 0)`; 0.
    native_cam_set_polar => 0x007b_b5d0, [1, 0];
    /// `Native_CamSetPos` (`0x007b9220`, `0x6002`): `0x007bb6a0(w, a, s, 1)`; 0.
    native_cam_set_pos => 0x007b_b6a0, [1];
    /// `Native_CamRefSetTarget` (`0x007b9fe0`, `0x6027`): `0x007bab90(w, a, s, 0)`; 0.
    native_cam_ref_set_target => 0x007b_ab90, [0];
    /// `Native_CamFocusTimeEase` (`0x007b8130`, `0x6032`): `0x007ba5b0(w, a, s, 0, 0)`; 0.
    native_cam_focus_time_ease => 0x007b_a5b0, [0, 0];
    /// `Native_CamRefAddLineArea` (`0x007ba1d0`, `0x6029`): `0x007bb2b0(w, a, s, 0, 1)`; 0.
    native_cam_ref_add_line_area => 0x007b_b2b0, [0, 1];
    /// `Native_CamMoveEased` (`0x007b9940`, `0x6014`): `0x007ba5b0(w, a, s, 1, 0)`; 0.
    native_cam_move_eased => 0x007b_a5b0, [1, 0];
    /// `Native_CamMoveEased1` (`0x007b9980`, `0x6015`): `0x007ba5b0(w, a, s, 1, 1)`; 0.
    native_cam_move_eased1 => 0x007b_a5b0, [1, 1];
    /// `Native_CamEyeOrbit` (`0x007b8750`, `0x6044`): `0x007badb0(w, a, s, 1, 2)`; 0.
    native_cam_eye_orbit => 0x007b_adb0, [1, 2];
    /// `Native_CamFocusOrbitScaled` (`0x007b8730`, `0x6041`): `0x007badb0(w, a, s, 0, 1)`; 0.
    native_cam_focus_orbit_scaled => 0x007b_adb0, [0, 1];
    /// `Native_CamFocusOrbit` (`0x007b8770`, `0x6045`): `0x007badb0(w, a, s, 0, 2)`; 0.
    native_cam_focus_orbit => 0x007b_adb0, [0, 2];
    /// `Native_CamEyeOrbitScaled` (`0x007b8710`, `0x6040`): `0x007badb0(w, a, s, 1, 1)`; 0.
    native_cam_eye_orbit_scaled => 0x007b_adb0, [1, 1];
    /// `Native_Cam_604d` (`0x007b87b0`, `0x604d`): `0x007badb0(w, a, s, 1, 4)`; 0.
    native_cam_604d => 0x007b_adb0, [1, 4];
    /// `Native_CamRefAddCircleArea` (`0x007ba2d0`, `0x602b`): `0x007bb370(w, a, s, 0, 1)`; 0.
    native_cam_ref_add_circle_area => 0x007b_b370, [0, 1];
    /// `Native_Cam_6006` (`0x007b9360`, `0x6006`): `0x007bb410(w, a, s, 1, 0xffff_ffff)`; 0.
    native_cam_6006 => 0x007b_b410, [1, 0xffff_ffff];
    /// `Native_Cam_6008` (`0x007b93c0`, `0x6008`): `0x007bb410(w, a, s, 1, 1)`; 0.
    native_cam_6008 => 0x007b_b410, [1, 1];
    /// `Native_Cam_6007` (`0x007b93a0`, `0x6007`): `0x007bb410(w, a, s, 1, 0)`; 0.
    native_cam_6007 => 0x007b_b410, [1, 0];
    /// `Native_Cam_603c` (`0x007b9400`, `0x603c`): `0x007bac40(w, a, s, 1)`; 0.
    native_cam_603c => 0x007b_ac40, [1];
    /// `Native_Cam_6035` (`0x007b8170`, `0x6035`): `0x007bb520(w, a, s, 0)`; 0.
    native_cam_6035 => 0x007b_b520, [0];
    /// `Native_Cam_6017` (`0x007b9a00`, `0x6017`): `0x007bb520(w, a, s, 1)`; 0.
    native_cam_6017 => 0x007b_b520, [1];
    /// `Native_CamGetPos` (`0x007b9260`, `0x6003`): `0x007ba3c0(w, a, s, 1)`; 0.
    native_cam_get_pos => 0x007b_a3c0, [1];
    /// `Native_Cam_6021` (`0x007b9d40`, `0x6021`): `0x007ba3c0(w, a, s, 0)`; 0.
    native_cam_6021 => 0x007b_a3c0, [0];
    /// `Native_Cam_603f` (`0x007b86f0`, `0x603f`): `0x007bad10(w, a, s, 0)`; 0.
    native_cam_603f => 0x007b_ad10, [0];
    /// `Native_Cam_6028` (`0x007ba0e0`, `0x6028`): `0x007bb2b0(w, a, s, 0, 0)`; 0.
    native_cam_6028 => 0x007b_b2b0, [0, 0];
    /// `Native_Cam_6071` (`0x007b9580`, `0x6071`): `0x007bb840(w, a, s, 5)`; 0.
    native_cam_6071 => 0x007b_b840, [5];
    /// `Native_Cam_606b` (`0x007b94a0`, `0x606b`): `0x007bb840(w, a, s, 3)`; 0.
    native_cam_606b => 0x007b_b840, [3];
    /// `Native_Cam_6005` (`0x007b9320`, `0x6005`): `0x007bb5d0(w, a, s, 1, 0xffff_ffff)`; 0.
    native_cam_6005 => 0x007b_b5d0, [1, 0xffff_ffff];
}

fwde! {
    /// `Native_Cam_6046` (`0x007b9d20`, `0x6046`): returns `0x007bb970(w, a, s, 1)`.
    native_cam_6046 => 0x007b_b970, [1];
    /// `Native_Cam_6047` (`0x007b9d60`, `0x6047`): returns `0x007bb970(w, a, s, 0)`.
    native_cam_6047 => 0x007b_b970, [0];
    /// `Native_Cam_605c` (`0x007b8e80`, `0x605c`): returns `0x007bb8c0(w, a, s, 0, 1)`.
    native_cam_605c => 0x007b_b8c0, [0, 1];
    /// `Native_Cam_605d` (`0x007b8f10`, `0x605d`): returns `0x007bb8c0(w, a, s, 1, 1)`.
    native_cam_605d => 0x007b_b8c0, [1, 1];
    /// `Native_Cam_6083` (`0x007b9340`, `0x6083`): returns `0x007bb8c0(w, a, s, 0, 8)`.
    native_cam_6083 => 0x007b_b8c0, [0, 8];
    /// `Native_Cam_6084` (`0x007b9380`, `0x6084`): returns `0x007bb8c0(w, a, s, 1, 8)`.
    native_cam_6084 => 0x007b_b8c0, [1, 8];
}

handle! {
    /// `Native_CamFocusCut` (`0x007b8210`, `0x6034`): `0x007bed60(cam, 0)`; the handle.
    native_cam_focus_cut => 0x007b_ed60, 0, true;
    /// `Native_CamRefClearAreas` (`0x007ba2f0`, `0x602d`): `0x007bed10(cam, 0)`; the handle.
    native_cam_ref_clear_areas => 0x007b_ed10, 0, true;
    /// `Native_Cam_600f` (`0x007b9520`, `0x600f`): `0x007bed10(cam, 1)`; the handle.
    native_cam_600f => 0x007b_ed10, 1, true;
    /// `Native_CamPathAddEye` (`0x007b9e30`, `0x6049`): `0x007baac0(cam, 1)`; the handle.
    native_cam_path_add_eye => 0x007b_aac0, 1, true;
    /// `Native_CamPathAddFocus` (`0x007b9f40`, `0x604b`): `0x007baac0(cam, 0)`; the handle.
    native_cam_path_add_focus => 0x007b_aac0, 0, true;
    /// `Native_Cam_6019` (`0x007b9ac0`, `0x6019`): `0x007bec90(cam, 1)`; the handle.
    native_cam_6019 => 0x007b_ec90, 1, true;
    /// `Native_Cam_6087` (`0x007b8a10`, `0x6087`): `0x007beae0(cam, 0)`; the handle.
    native_cam_6087 => 0x007b_eae0, 0, true;
    /// `Native_Cam_6088` (`0x007b8a70`, `0x6088`): `0x007beae0(cam, 1)`; the handle.
    native_cam_6088 => 0x007b_eae0, 1, true;
    /// `Native_Cam_6060` (`0x007b98e0`, `0x6060`): `0x007c4f50(cam, 1)`; 0.
    native_cam_6060 => 0x007c_4f50, 1, false;
    /// `Native_Cam_605f` (`0x007b97a0`, `0x605f`): `0x007c4f50(cam, 0)`; 0.
    native_cam_605f => 0x007c_4f50, 0, false;
}

/// `Native_Cam_6076` (`0x007b8fe0`): `0x007ba950()`; 0.
pub fn native_cam_6076<H: NativeHost>(h: &mut H, _w: u32, _a: u32, _s: u32) -> u32 {
    call(h, 0x007b_a950, &[]);
    0
}

/// A poll: 5 once `f(cam, k)` is 0, else 0.
fn wait<H: NativeHost>(h: &mut H, w: u32, f: u32, k: u32) -> u32 {
    let c = cam(h, w);
    if call(h, f, &[c, k]) == 0 { 5 } else { 0 }
}

/// `Native_CamFocusWait` (`0x007b8270`, `0x6038`, poll): 5 once
/// `Cam_IsMoving(cam, 0)` is 0.
pub fn native_cam_focus_wait<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    wait(h, w, 0x007b_dd20, 0)
}

/// `Native_Cam_6063` (`0x007b99a0`, poll): 5 once `0x007c4f10(cam, 1)` is 0.
pub fn native_cam_6063<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    wait(h, w, 0x007c_4f10, 1)
}

/// `Native_Cam_6062` (`0x007b9a20`, poll): 5 once `0x007c4f10(cam, 0)` is 0.
pub fn native_cam_6062<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    wait(h, w, 0x007c_4f10, 0)
}

/// `MsCameraGetID(cam)`; when it is the handle itself, clears `g_BtlSplines[i]`.
fn clear_spline<H: NativeHost>(h: &mut H, w: u32, i: u32) -> u32 {
    let c = cam(h, w);
    let r = call(h, 0x007b_d300, &[c]);
    let p = BTL_SPLINES.ptr(h).addr + i * 4;
    let v = h.read::<u32>(p);
    h.write(p, if r == c { 0 } else { v });
    c
}

/// `Native_Cam_6048` (`0x007b9da0`): [`clear_spline`] 1; the handle.
pub fn native_cam_6048<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    clear_spline(h, w, 1)
}

/// `Native_Cam_604a` (`0x007b9eb0`): [`clear_spline`] 0; the handle.
pub fn native_cam_604a<H: NativeHost>(h: &mut H, w: u32, _a: u32, _s: u32) -> u32 {
    clear_spline(h, w, 0)
}

fn suspend<H: NativeHost>(h: &mut H, s: u32, on: u32) -> u32 {
    let i = pop(h, s);
    let o = atel_get_basic_work(h, i.cast_signed());
    let c = cam(h, o);
    call(h, 0x007c_0010, &[c, on]);
    c
}

/// `Native_CamSuspend` (`0x007b8ea0`, `0x6000`): `(worker)`:
/// `Cam_SetChannelSuspended(its camera, 1)`; the handle.
pub fn native_cam_suspend<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    suspend(h, s, 1)
}

/// `Native_CamResume` (`0x007b8f30`, `0x6001`): as [`native_cam_suspend`]
/// with 0.
pub fn native_cam_resume<H: NativeHost>(h: &mut H, _w: u32, _a: u32, s: u32) -> u32 {
    suspend(h, s, 0)
}

fn move_frame<H: NativeHost>(h: &mut H, w: u32, s: u32, side: u32, mode: u32, ty: u32) -> u32 {
    let c = cam(h, w);
    let n = pop(h, s);
    call(h, 0x007b_de50, &[c, side, mode, n, ty]);
    0
}

/// `Native_CamMove` (`0x007b96c0`, `0x6010`): `(frames)`:
/// `MsCameraMoveFrame(cam, 1, 0, frames, 1)`; 0.
pub fn native_cam_move<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    move_frame(h, w, s, 1, 0, 1)
}

/// `Native_CamFocusTimeLinear` (`0x007b7f70`, `0x602e`): `(cam, 0, 0, frames, 1)`.
pub fn native_cam_focus_time_linear<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    move_frame(h, w, s, 0, 0, 1)
}

/// `Native_Cam_6012` (`0x007b9800`): `(cam, 1, 0, frames, 3)`.
pub fn native_cam_6012<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    move_frame(h, w, s, 1, 0, 3)
}

/// `Native_Cam_6030` (`0x007b8050`): `(cam, 0, 0, frames, 3)`.
pub fn native_cam_6030<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    move_frame(h, w, s, 0, 0, 3)
}

/// `Native_Cam_6011` (`0x007b9730`): `(cam, 1, 1, frames, 1)`.
pub fn native_cam_6011<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    move_frame(h, w, s, 1, 1, 1)
}

/// `Native_Cam_6043` (`0x007b8660`): `(v)`: `0x007bdf90(cam, 1, v ^ 0xffff)`;
/// the handle.
pub fn native_cam_6043<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let c = cam(h, w);
    let v = pop(h, s);
    call(h, 0x007b_df90, &[c, 1, v ^ 0xffff]);
    c
}

/// `Native_Cam_607d` (`0x007b8840`): `(v)`: `0x007bcdd0(cam, v)`; the handle.
pub fn native_cam_607d<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let c = cam(h, w);
    let v = pop(h, s);
    call(h, 0x007b_cdd0, &[c, v]);
    c
}

/// `Native_CamSetRoll` (`0x007b9010`, `0x603a`): `(degrees)`:
/// `MsCameraSetRoll(cam, (float)(deg * pi / 180))`, `pi` the `f32` one as a
/// double; the handle.
pub fn native_cam_set_roll<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let c = cam(h, w);
    let d = pop_st0(h, s);
    let pi = h.read::<f64>(h.rebase(0x00b9_2288));
    let k = h.read::<f64>(h.rebase(0x00b4_8378));
    let t = x87::store(x87::div(x87::mul(d, pi), k));
    call(h, 0x007b_fa80, &[c, t]);
    c
}

/// `Native_CamSetScreenDepth` (`0x007b90b0`, `0x603b`): `(depth)`:
/// `MsCameraSetScrDpt(cam, depth)`, printed first in debug mode; the handle.
pub fn native_cam_set_screen_depth<H: NativeHost>(h: &mut H, w: u32, _a: u32, s: u32) -> u32 {
    let c = cam(h, w);
    let d = pop_st0(h, s);
    if DEBUG_MODE.get(h) == 1 {
        let n = x87::ftol(h, d);
        let p = PRINTF_IAT.get(h);
        let fmt = h.rebase(FMT_SET_SCR_DPT);
        h.call(p, &[fmt, n]);
    }
    call(h, 0x007b_fbd0, &[c, x87::store(d)]);
    c
}

/// `Native_Cam_6064` (`0x007b9aa0`, start): `(id)` into the args word; `EAX`
/// is the id.
pub fn native_cam_6064<H: NativeHost>(h: &mut H, _w: u32, a: u32, s: u32) -> u32 {
    let v = pop(h, s);
    h.write(a, v);
    v
}

/// `Native_Cam_6064_Poll` (`0x007b9b20`): 5 once `0x007c5680(id)` is 0.
pub fn native_cam_6064_poll<H: NativeHost>(h: &mut H, _w: u32, a: u32, _s: u32) -> u32 {
    let v = h.read::<u32>(a);
    if call(h, 0x007c_5680, &[v]) == 0 {
        5
    } else {
        0
    }
}

/// The Rust version of a camera handler, if there is one.
#[must_use]
pub fn find<H: NativeHost>(sym: u32) -> Option<Native<H>> {
    Some(Native::Int(match sym {
        0x007b_7f70 => native_cam_focus_time_linear,
        0x007b_8050 => native_cam_6030,
        0x007b_8130 => native_cam_focus_time_ease,
        0x007b_8170 => native_cam_6035,
        0x007b_8210 => native_cam_focus_cut,
        0x007b_8270 => native_cam_focus_wait,
        0x007b_8660 => native_cam_6043,
        0x007b_86f0 => native_cam_603f,
        0x007b_8710 => native_cam_eye_orbit_scaled,
        0x007b_8730 => native_cam_focus_orbit_scaled,
        0x007b_8750 => native_cam_eye_orbit,
        0x007b_8770 => native_cam_focus_orbit,
        0x007b_87b0 => native_cam_604d,
        0x007b_8840 => native_cam_607d,
        0x007b_8a10 => native_cam_6087,
        0x007b_8a70 => native_cam_6088,
        0x007b_8e80 => native_cam_605c,
        0x007b_8ea0 => native_cam_suspend,
        0x007b_8f10 => native_cam_605d,
        0x007b_8f30 => native_cam_resume,
        0x007b_8fe0 => native_cam_6076,
        0x007b_9010 => native_cam_set_roll,
        0x007b_90b0 => native_cam_set_screen_depth,
        0x007b_9220 => native_cam_set_pos,
        0x007b_9260 => native_cam_get_pos,
        0x007b_92e0 => native_cam_set_polar,
        0x007b_9320 => native_cam_6005,
        0x007b_9340 => native_cam_6083,
        0x007b_9360 => native_cam_6006,
        0x007b_9380 => native_cam_6084,
        0x007b_93a0 => native_cam_6007,
        0x007b_93c0 => native_cam_6008,
        0x007b_9400 => native_cam_603c,
        0x007b_94a0 => native_cam_606b,
        0x007b_9520 => native_cam_600f,
        0x007b_9580 => native_cam_6071,
        0x007b_96c0 => native_cam_move,
        0x007b_9730 => native_cam_6011,
        0x007b_97a0 => native_cam_605f,
        0x007b_9800 => native_cam_6012,
        0x007b_98e0 => native_cam_6060,
        0x007b_9940 => native_cam_move_eased,
        0x007b_9980 => native_cam_move_eased1,
        0x007b_99a0 => native_cam_6063,
        0x007b_9a00 => native_cam_6017,
        0x007b_9a20 => native_cam_6062,
        0x007b_9aa0 => native_cam_6064,
        0x007b_9ac0 => native_cam_6019,
        0x007b_9b20 => native_cam_6064_poll,
        0x007b_9d00 => native_cam_ref_set_pos,
        0x007b_9d20 => native_cam_6046,
        0x007b_9d40 => native_cam_6021,
        0x007b_9d60 => native_cam_6047,
        0x007b_9da0 => native_cam_6048,
        0x007b_9e30 => native_cam_path_add_eye,
        0x007b_9eb0 => native_cam_604a,
        0x007b_9f40 => native_cam_path_add_focus,
        0x007b_9fe0 => native_cam_ref_set_target,
        0x007b_a0e0 => native_cam_6028,
        0x007b_a1d0 => native_cam_ref_add_line_area,
        0x007b_a2d0 => native_cam_ref_add_circle_area,
        0x007b_a2f0 => native_cam_ref_clear_areas,
        _ => return super::native_msg::find(sym),
    }))
}

/// Every handler [`find`] has, with whether it returns a float.
pub const PORTED: [(u32, bool); 62] = [
    (0x007b_7f70, false),
    (0x007b_8050, false),
    (0x007b_8130, false),
    (0x007b_8170, false),
    (0x007b_8210, false),
    (0x007b_8270, false),
    (0x007b_8660, false),
    (0x007b_86f0, false),
    (0x007b_8710, false),
    (0x007b_8730, false),
    (0x007b_8750, false),
    (0x007b_8770, false),
    (0x007b_87b0, false),
    (0x007b_8840, false),
    (0x007b_8a10, false),
    (0x007b_8a70, false),
    (0x007b_8e80, false),
    (0x007b_8ea0, false),
    (0x007b_8f10, false),
    (0x007b_8f30, false),
    (0x007b_8fe0, false),
    (0x007b_9010, false),
    (0x007b_90b0, false),
    (0x007b_9220, false),
    (0x007b_9260, false),
    (0x007b_92e0, false),
    (0x007b_9320, false),
    (0x007b_9340, false),
    (0x007b_9360, false),
    (0x007b_9380, false),
    (0x007b_93a0, false),
    (0x007b_93c0, false),
    (0x007b_9400, false),
    (0x007b_94a0, false),
    (0x007b_9520, false),
    (0x007b_9580, false),
    (0x007b_96c0, false),
    (0x007b_9730, false),
    (0x007b_97a0, false),
    (0x007b_9800, false),
    (0x007b_98e0, false),
    (0x007b_9940, false),
    (0x007b_9980, false),
    (0x007b_99a0, false),
    (0x007b_9a00, false),
    (0x007b_9a20, false),
    (0x007b_9aa0, false),
    (0x007b_9ac0, false),
    (0x007b_9b20, false),
    (0x007b_9d00, false),
    (0x007b_9d20, false),
    (0x007b_9d40, false),
    (0x007b_9d60, false),
    (0x007b_9da0, false),
    (0x007b_9e30, false),
    (0x007b_9eb0, false),
    (0x007b_9f40, false),
    (0x007b_9fe0, false),
    (0x007b_a0e0, false),
    (0x007b_a1d0, false),
    (0x007b_a2d0, false),
    (0x007b_a2f0, false),
];
