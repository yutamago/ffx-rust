//! The battle scripts' camera natives (`docs/rewrite-plan.md`, M2): each
//! takes the worker's camera (its camera work word, got once from
//! `MsAtelGetCameraID`), pops its arguments off the script stack and hands
//! them, the last popped first, to one of the camera's setters. The
//! script's functions and the setters are called through the game (`call`
//! for the ones that return in `EAX`, `fcall` for `AtelPopStackFloat`'s
//! `f32` in ST0, as bits).

/// The game's memory, through [`ffx_bridge::Mem`].
// the popped arguments are a, b, ... (a popped first), c the calls
#[allow(clippy::many_single_char_names)]
pub mod bridge {
    use ffx_bridge::Mem;

    use crate::exec::bridge::Call;

    /// `AtelGetCameraWorkAdrs(worker)` (script): the worker's camera word.
    pub const ATEL_GET_CAMERA_WORK_ADRS: u32 = 0x0086_ae50;
    /// `AtelGetEventMapCameraId()` (script).
    pub const ATEL_GET_EVENT_MAP_CAMERA_ID: u32 = 0x0086_b5d0;
    /// `AtelPopStackFloat(worker, stack)` (script): an `f32` in ST0.
    pub const ATEL_POP_STACK_FLOAT: u32 = 0x0086_de50;
    /// `AtelGetRealVarAdrs(worker, word)` (script).
    pub const ATEL_GET_REAL_VAR_ADRS: u32 = 0x0086_c2e0;
    /// The setters the natives call: `MsCameraMoveAcc`, `MsCameraShake`,
    /// `MsScreenShake`, `MsCameraAddLineFilter_007bf0b0`,
    /// `MsCameraMoveDivide`, `MsCameraSetAimPolar_007bf330`,
    /// `MsCameraSetAimPolar2_007bf420`, `MsCameraSetAtelFocus_007beeb0`,
    /// `MsCameraSetAtelYaw_007befc0`, `MsCameraSetSpline`, `MsCameraSetBtl`,
    /// `MsCameraRefAddCircleArea_007bf1f0`.
    pub const MS_CAMERA_MOVE_ACC: u32 = 0x007b_d860;
    pub const MS_CAMERA_SHAKE: u32 = 0x007c_4fc0;
    pub const MS_SCREEN_SHAKE: u32 = 0x007c_56f0;
    pub const MS_CAMERA_ADD_LINE_FILTER: u32 = 0x007b_f0b0;
    pub const MS_CAMERA_MOVE_DIVIDE: u32 = 0x007b_dde0;
    pub const MS_CAMERA_SET_AIM_POLAR: u32 = 0x007b_f330;
    pub const MS_CAMERA_SET_AIM_POLAR2: u32 = 0x007b_f420;
    pub const MS_CAMERA_SET_ATEL_FOCUS: u32 = 0x007b_eeb0;
    pub const MS_CAMERA_SET_ATEL_YAW: u32 = 0x007b_efc0;
    pub const MS_CAMERA_SET_SPLINE: u32 = 0x007b_fca0;
    pub const MS_CAMERA_SET_BTL: u32 = 0x007c_5c60;
    pub const MS_CAMERA_REF_ADD_CIRCLE_AREA: u32 = 0x007b_f1f0;
    /// The two built-in splines (`MsSplineRegist`): counts at `0x01136b60`,
    /// records of 0x210 bytes from `0x01136b68`.
    pub const SPLINE_COUNTS: u32 = 0x0113_6b60;
    pub const SPLINE_RECORDS: u32 = 0x0113_6b68;
    pub const SPLINE_SIZE: u32 = 0x210;
    /// `g_PiF64` and 180.0 (degrees to radians).
    pub const PI: u32 = 0x00b9_2288;
    pub const HALF_TURN_DEG: u32 = 0x00b4_8378;
    /// 100.0, the battle camera's lift scale (`0x007bad10`).
    pub const HUNDRED: u32 = 0x00b9_22e0;

    /// What the natives call in the game: `EAX` results and `f32` ones.
    pub struct Calls<'a, M> {
        pub call: Call<'a, M>,
        pub fcall: Call<'a, M>,
    }

    impl<M: Mem> Calls<'_, M> {
        fn game(&mut self, mem: &mut M, sym: u32, args: &[u32]) -> u32 {
            let f = mem.rebase(sym);
            (self.call)(mem, f, args)
        }

        /// `AtelPopStackInteger(worker, stack)`.
        fn int(&mut self, mem: &mut M, w: u32, s: u32) -> u32 {
            self.game(mem, crate::motion::bridge::ATEL_POP_STACK_INTEGER, &[w, s])
        }

        /// `AtelPopStackFloat(worker, stack)`, rounded to `f32` as the
        /// natives store it.
        fn float(&mut self, mem: &mut M, w: u32, s: u32) -> u32 {
            let f = mem.rebase(ATEL_POP_STACK_FLOAT);
            (self.fcall)(mem, f, &[w, s])
        }

        /// `n` floats, in the order popped.
        fn floats<const N: usize>(&mut self, mem: &mut M, w: u32, s: u32) -> [u32; N] {
            let mut v = [0; N];
            for x in &mut v {
                *x = self.float(mem, w, s);
            }
            v
        }
    }

    fn f64_at(mem: &impl Mem, sym: u32) -> f64 {
        f64::from_bits(mem.read(mem.rebase(sym)))
    }

    /// Degrees (an `f32`'s bits) to radians as the natives turn them:
    /// `* g_PiF64 / 180.0` on the x87, rounded to `f32`.
    fn radians(mem: &impl Mem, deg: u32) -> u32 {
        use ffx_bridge::x87::{div, load, mul, store};
        store(div(
            mul(load(deg), f64_at(mem, PI)),
            f64_at(mem, HALF_TURN_DEG),
        ))
    }

    /// `MsAtelGetCamera_007baa50(worker, kind, cam)` (`0x007baa50`): the
    /// worker's camera (`AtelGetCameraWorkAdrs`): kept, or got now and
    /// kept: `MsAtelGetCameraID(5, kind, cam)`, a `kind` of 0 becoming 1
    /// when the worker's actor (`AtelGetActorId`) is the event map's camera
    /// (`AtelGetEventMapCameraId`). `EAX` the camera.
    pub fn ms_atel_get_camera<M: Mem>(
        mem: &mut M,
        c: &mut Calls<'_, M>,
        w: u32,
        kind: u32,
        cam: u32,
    ) -> u32 {
        let work = c.game(mem, ATEL_GET_CAMERA_WORK_ADRS, &[w]);
        let id: u32 = mem.read(work);
        if id != 0 {
            return id;
        }
        let mut kind = kind;
        if kind == 0 {
            let actor = c.game(mem, crate::atel::bridge::ATEL_GET_ACTOR_ID, &[w]);
            if c.game(mem, ATEL_GET_EVENT_MAP_CAMERA_ID, &[]) == actor {
                kind = 1;
            }
        }
        let id = crate::camera::bridge::ms_atel_get_camera_id(mem, 5, kind, cam);
        mem.write(work, id);
        id
    }

    /// The natives' own camera: [`ms_atel_get_camera`] with kind 0 (the
    /// event map's camera's actor 1) and camera 0.
    fn camera<M: Mem>(mem: &mut M, c: &mut Calls<'_, M>, w: u32) -> u32 {
        ms_atel_get_camera(mem, c, w, 0, 0)
    }

    /// `MsAtelCameraMoveAcc_007ba5b0(worker, _, stack, side, mode)`
    /// (`0x007ba5b0`): four integers popped (`a`..`d`) into
    /// `MsCameraMoveAcc(camera, side, mode, d, c, b, a)`; its result.
    pub fn ms_atel_camera_move_acc<M: Mem>(
        mem: &mut M,
        c: &mut Calls<'_, M>,
        w: u32,
        s: u32,
        side: u32,
        mode: u32,
    ) -> u32 {
        let cam = camera(mem, c, w);
        let [a, b, x, d] = [0; 4].map(|_| c.int(mem, w, s));
        c.game(mem, MS_CAMERA_MOVE_ACC, &[cam, side, mode, d, x, b, a])
    }

    /// `MsAtelCameraShake(worker, _, stack, side, kind)` (`0x007bb8c0`):
    /// three integers (`a`..`c`), a float `f` and an integer `which` popped
    /// into `MsCameraShake(camera, side, which, kind, f, c, b, a)`; the
    /// camera.
    pub fn ms_atel_camera_shake<M: Mem>(
        mem: &mut M,
        c: &mut Calls<'_, M>,
        w: u32,
        s: u32,
        side: u32,
        kind: u32,
    ) -> u32 {
        let cam = camera(mem, c, w);
        let [a, b, x] = [0; 3].map(|_| c.int(mem, w, s));
        let f = c.float(mem, w, s);
        let which = c.int(mem, w, s);
        c.game(mem, MS_CAMERA_SHAKE, &[cam, side, which, kind, f, x, b, a]);
        cam
    }

    /// `MsAtelScreenShake(worker, _, stack, kind)` (`0x007bb840`): three
    /// integers (`a`..`c`), a float `f`, then `which` and the screen popped
    /// into `MsScreenShake(screen, which, kind, f, c, b, a)`; 0.
    pub fn ms_atel_screen_shake<M: Mem>(
        mem: &mut M,
        c: &mut Calls<'_, M>,
        w: u32,
        s: u32,
        kind: u32,
    ) -> u32 {
        let [a, b, x] = [0; 3].map(|_| c.int(mem, w, s));
        let f = c.float(mem, w, s);
        let which = c.int(mem, w, s);
        let screen = c.int(mem, w, s);
        c.game(mem, MS_SCREEN_SHAKE, &[screen, which, kind, f, x, b, a]);
        0
    }

    /// `MsAtelCameraLineFilter_007bb2b0(worker, _, stack, side, kind)`
    /// (`0x007bb2b0`): five floats popped into
    /// `MsCameraAddLineFilter_007bf0b0(camera, side, kind, e, d, c, b, a)`;
    /// its `EAX`.
    pub fn ms_atel_camera_line_filter<M: Mem>(
        mem: &mut M,
        c: &mut Calls<'_, M>,
        w: u32,
        s: u32,
        side: u32,
        kind: u32,
    ) -> u32 {
        let cam = camera(mem, c, w);
        let [a, b, x, d, e] = c.floats::<5>(mem, w, s);
        c.game(
            mem,
            MS_CAMERA_ADD_LINE_FILTER,
            &[cam, side, kind, e, d, x, b, a],
        )
    }

    /// `MsAtelCameraMoveDivide_007bb520(worker, _, stack, side)`
    /// (`0x007bb520`): four floats popped into `MsCameraMoveDivide(camera,
    /// side, d, c, b, a)`; its `EAX`.
    pub fn ms_atel_camera_move_divide<M: Mem>(
        mem: &mut M,
        c: &mut Calls<'_, M>,
        w: u32,
        s: u32,
        side: u32,
    ) -> u32 {
        let cam = camera(mem, c, w);
        let [a, b, x, d] = c.floats::<4>(mem, w, s);
        c.game(mem, MS_CAMERA_MOVE_DIVIDE, &[cam, side, d, x, b, a])
    }

    /// `MsAtelCameraEyeAddArea_007bb410(worker, _, stack, side, short)`
    /// (`0x007bb410`): six floats popped (`a`..`f`); with a negative
    /// `short`, `MsCameraSetAimPolar_007bf330(camera, side, f, e, rad(d), c,
    /// b, a)` (`d` from degrees), else `MsCameraSetAimPolar2_007bf420(camera,
    /// side, short, f, e, d, c, b, a)`; its `EAX`.
    pub fn ms_atel_camera_eye_add_area<M: Mem>(
        mem: &mut M,
        c: &mut Calls<'_, M>,
        w: u32,
        s: u32,
        side: u32,
        short: u32,
    ) -> u32 {
        let cam = camera(mem, c, w);
        let [a, b, x, d, e, f] = c.floats::<6>(mem, w, s);
        if short.cast_signed() < 0 {
            let d = radians(mem, d);
            c.game(mem, MS_CAMERA_SET_AIM_POLAR, &[cam, side, f, e, d, x, b, a])
        } else {
            c.game(
                mem,
                MS_CAMERA_SET_AIM_POLAR2,
                &[cam, side, short, f, e, d, x, b, a],
            )
        }
    }

    /// `MsAtelCameraFocus_007bab90(worker, _, stack, side, local)`
    /// (`0x007bab90`): a float `y` popped into the vector (0, `y`, 0) at
    /// `local` (the original's frame; 12 bytes), then two integers `a`,
    /// `b` into `MsCameraSetAtelFocus_007beeb0(camera, side, b, a, local)`;
    /// its `EAX`.
    pub fn ms_atel_camera_focus<M: Mem>(
        mem: &mut M,
        c: &mut Calls<'_, M>,
        w: u32,
        s: u32,
        side: u32,
        local: u32,
    ) -> u32 {
        let cam = camera(mem, c, w);
        mem.write(local, 0u32);
        let y = c.float(mem, w, s);
        mem.write(local + 4, y);
        mem.write(local + 8, 0u32);
        let a = c.int(mem, w, s);
        let b = c.int(mem, w, s);
        c.game(mem, MS_CAMERA_SET_ATEL_FOCUS, &[cam, side, b, a, local])
    }

    /// `MsAtelCameraYaw_007bac40(worker, _, stack, side)` (`0x007bac40`):
    /// four floats popped, the last two from degrees, into
    /// `MsCameraSetAtelYaw_007befc0(camera, side, rad(d), rad(c), b, a)`;
    /// its `EAX`.
    pub fn ms_atel_camera_yaw<M: Mem>(
        mem: &mut M,
        c: &mut Calls<'_, M>,
        w: u32,
        s: u32,
        side: u32,
    ) -> u32 {
        let cam = camera(mem, c, w);
        let a = c.float(mem, w, s);
        let b = c.float(mem, w, s);
        let x = c.float(mem, w, s);
        let x = radians(mem, x);
        let d = c.float(mem, w, s);
        let d = radians(mem, d);
        c.game(mem, MS_CAMERA_SET_ATEL_YAW, &[cam, side, d, x, b, a])
    }

    /// `MsAtelCameraSpline_007bb970(worker, _, stack, side)` (`0x007bb970`):
    /// an integer `flags`, floats `speed` and `t`, then the spline popped:
    /// 0 and 1 the built-in ones ([`SPLINE_RECORDS`], their count copied to
    /// their first word, used from 3 points on), others a script variable
    /// (`AtelGetRealVarAdrs`, null skips); then `MsCameraSetSpline(camera,
    /// side, spline, t, speed, flags)`. The camera.
    pub fn ms_atel_camera_spline<M: Mem>(
        mem: &mut M,
        c: &mut Calls<'_, M>,
        w: u32,
        s: u32,
        side: u32,
    ) -> u32 {
        let cam = camera(mem, c, w);
        let flags = c.int(mem, w, s);
        let speed = c.float(mem, w, s);
        let t = c.float(mem, w, s);
        let k = c.int(mem, w, s);
        let spline = if k <= 1 {
            let n: i32 = mem.read(mem.rebase(SPLINE_COUNTS) + 4 * k);
            if n < 3 {
                return cam;
            }
            let rec = mem.rebase(SPLINE_RECORDS) + SPLINE_SIZE * k;
            mem.write(rec, n);
            rec
        } else {
            c.game(mem, ATEL_GET_REAL_VAR_ADRS, &[w, k])
        };
        if spline != 0 {
            c.game(
                mem,
                MS_CAMERA_SET_SPLINE,
                &[cam, side, spline, t, speed, flags],
            );
        }
        cam
    }

    /// `MsAtelCameraSetBtl_007bad10(worker, _, stack, side)` (`0x007bad10`):
    /// the worker's actor byte (`AtelGetBattleIdAdrs2`) and camera
    /// ([`ms_atel_get_camera`]'s store, kind 2 without the actor check), a
    /// float `lift` (over 100.0) and integers `b`, `a` popped into
    /// `MsCameraSetBtl(camera, side, actor, 0, a, b, lift, 0.0, 0.0, 0.0)`.
    /// The camera.
    pub fn ms_atel_camera_set_btl<M: Mem>(
        mem: &mut M,
        c: &mut Calls<'_, M>,
        w: u32,
        s: u32,
        side: u32,
    ) -> u32 {
        use ffx_bridge::x87::{div, load, store};
        let actor = c.game(mem, crate::motion::bridge::ATEL_GET_BATTLE_ID_ADRS2, &[w]);
        let work = c.game(mem, ATEL_GET_CAMERA_WORK_ADRS, &[w]);
        let mut cam: u32 = mem.read(work);
        if cam == 0 {
            cam = crate::camera::bridge::ms_atel_get_camera_id(mem, 5, 2, 0);
            mem.write(work, cam);
        }
        let lift = c.float(mem, w, s);
        let lift = store(div(load(lift), f64_at(mem, HUNDRED)));
        let b = c.int(mem, w, s);
        let a = c.int(mem, w, s);
        let chr = u32::from(mem.read::<u8>(actor));
        c.game(
            mem,
            MS_CAMERA_SET_BTL,
            &[cam, side, chr, 0, a, b, lift, 0, 0, 0],
        );
        cam
    }

    /// `MsAtelCameraRefAddCircleArea_007bb370(worker, _, stack, side,
    /// kind)` (`0x007bb370`): three floats popped into
    /// `MsCameraRefAddCircleArea_007bf1f0(camera, side, kind, c, b, a)`; its
    /// `EAX`.
    pub fn ms_atel_camera_ref_add_circle_area<M: Mem>(
        mem: &mut M,
        c: &mut Calls<'_, M>,
        w: u32,
        s: u32,
        side: u32,
        kind: u32,
    ) -> u32 {
        let cam = camera(mem, c, w);
        let [a, b, x] = c.floats::<3>(mem, w, s);
        c.game(
            mem,
            MS_CAMERA_REF_ADD_CIRCLE_AREA,
            &[cam, side, kind, x, b, a],
        )
    }
}
