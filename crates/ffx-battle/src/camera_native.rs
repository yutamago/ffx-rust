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

    /// `MsAtelCameraDirVector_007bb210(worker, _, stack, side)`
    /// (`0x007bb210`, name ours; natives 0x6036 and 0x6018): three floats
    /// popped (`a`, `b`, `c`) into
    /// `MsCameraSetDirVector_007bce80(camera, side, c, b, a)`.
    ///
    /// The callee's scale words past the first are this function's `esi`
    /// (`stack`), its `ebp` (`frame`: the original's frame pointer) and
    /// the return address into it (see
    /// `ffx_battle::camera::bridge::ms_camera_set_dir_vector`).
    pub fn ms_atel_camera_dir_vector<M: Mem>(
        mem: &mut M,
        c: &mut Calls<'_, M>,
        w: u32,
        s: u32,
        side: u32,
        frame: u32,
    ) {
        let cam = camera(mem, c, w);
        let [a, b, x] = c.floats::<3>(mem, w, s);
        let ret = mem.rebase(DIR_VECTOR_RETURN);
        crate::camera::bridge::ms_camera_set_dir_vector(mem, cam, side, x, b, a, [s, frame, ret]);
    }

    /// Where `MsCameraSetDirVector_007bce80` returns to in
    /// [`ms_atel_camera_dir_vector`]'s original (`0x007bb29e`): a word of
    /// the scale vector it leaves in VU register 2.
    pub const DIR_VECTOR_RETURN: u32 = 0x007b_b29e;

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

    /// `MsGetBattleScene` (battle), `MagicFile_GetMagicId` and
    /// `MagicFile_GetHandle` (system), `graphicLevelGetNameHash`
    /// (render), `Ch_GetId` and `Ch_SetHideBattle` (chara): the callees
    /// of [`ms_atel_camera_set_btl_pc`] it reaches through the game.
    pub const MS_GET_BATTLE_SCENE: u32 = 0x0078_2970;
    pub const MAGIC_FILE_GET_MAGIC_ID: u32 = 0x009d_a3b0;
    pub const MAGIC_FILE_GET_HANDLE: u32 = 0x009d_a390;
    pub const GRAPHIC_LEVEL_GET_NAME_HASH: u32 = 0x0064_1da0;
    pub const CH_GET_ID: u32 = 0x0082_6b80;
    pub const CH_SET_HIDE_BATTLE: u32 = 0x0082_b260;

    /// [`ms_atel_camera_set_btl_pc`]'s constants (`f64`s in the image but
    /// for the last two, `f32`s): the products it compares with, its
    /// steps, and the 0x92 fix's camera.
    const C_MINUS_150: u32 = 0x00b5_5150;
    const C_89: u32 = 0x00b5_5128;
    const C_229: u32 = 0x00b5_5130;
    const C_0_9: u32 = 0x00b4_9958;
    const C_4: u32 = 0x00b5_5108;
    const C_1_5_F32: u32 = 0x00b3_eb58;
    const C_3: u32 = 0x00b2_c5c8;
    const C_HALF: u32 = 0x00b9_2220;
    const C_30_49: u32 = 0x00b5_5120;
    const C_TINY: u32 = 0x00b3_eb50;
    const C_29_49: u32 = 0x00b5_5118;
    const C_3_5: u32 = 0x00b5_5100;
    const C_10_09: u32 = 0x00b5_5110;
    const C_0_1: u32 = 0x00b9_21f0;
    const C_0X92_X_F32: u32 = 0x00b5_50fc;
    const C_0X92_DIST_F32: u32 = 0x00b5_5140;

    /// `MsAtelCameraSetBtlPc_007badb0(worker, _, stack, side, mode)`
    /// (`0x007badb0`, name ours; natives 0x6040, 0x604c, 0x604d): as
    /// [`ms_atel_camera_set_btl`] with six values popped: `dist` (over
    /// 100.0), yaw and pitch (degrees to radians), `x` (over 100.0) and
    /// integers `b`, `a`, into `MsCameraSetBtl(camera, side, actor, mode,
    /// a, b, x, pitch, yaw, dist)`, after the PC build's fixes for single
    /// battles (scene `MsGetBattleScene`, magic `MagicFile_GetMagicId`):
    ///
    /// - scene 0xd40000, level hash 0x200f, magic 0xfb: with `100 dist`
    ///   -150.0 and `100 x` 88.99999237, the hidden characters
    ///   ([`HIDE_QUEUE`] from its last down to 1) shown
    ///   (`Ch_SetHideBattle(chr, 1)`) and [`HIDE_MASK`] 0;
    /// - scene 0x1a30004, magic 0x58: `100 dist` 228.99996948, `dist` less
    ///   0.9;
    /// - scene 0x1f60004, magic 0xc9: `100 x` 3.99999905, `x` plus 0.9;
    /// - scene 0x25c0013, side 1, mode 2, magic 0x4c or 0x137, the command
    ///   actor's character `Ch_GetId` 0x10c1: `dist` 1.5 tripled;
    /// - magic 0xa3, scene 0x1590000, side 1, mode 2: `100 dist + 0.5`
    ///   truncated 520, `dist` less 0.5;
    /// - magic 0x2c1 with a magic file (`MagicFile_GetHandle` not -1):
    ///   `|trunc(dist - 30.49)|` or else `|trunc(dist - 29.49)|` (as
    ///   `f32`s) below 1e-4, `dist` less 3.5;
    /// - magic 0x2c0 with a magic file: `|trunc(dist - 10.09)|` below
    ///   1e-4, `x` plus 0.1;
    /// - magic 0x92 with a magic file in scene 0x1ae000a: `100 x` and
    ///   `100 dist` truncated 0x12 and 1000, `x` 0.2221 and `dist` 9.0.
    ///
    /// The products are the x87's at the game's 53-bit precision, where
    /// no `f32` times 100 makes 88.99999237, 228.99996948 or 3.99999905
    /// (they are `f32` products, single precision's): those three fixes
    /// never apply. The camera. (The equalities are the x87's `fucom`
    /// tests: exact on purpose.)
    #[allow(clippy::too_many_lines, clippy::float_cmp)]
    pub fn ms_atel_camera_set_btl_pc<M: Mem>(
        mem: &mut M,
        c: &mut Calls<'_, M>,
        w: u32,
        s: u32,
        side: u32,
        mode: u32,
    ) -> u32 {
        use crate::visible::bridge::{HIDE_MASK, HIDE_QUEUE, HIDE_QUEUE_LAST};
        use ffx_bridge::x87::{add, div, ftol, load, mul, store, sub};
        let scene = c.game(mem, MS_GET_BATTLE_SCENE, &[]);
        let magic = c.game(mem, MAGIC_FILE_GET_MAGIC_ID, &[]);
        let actor = c.game(mem, crate::motion::bridge::ATEL_GET_BATTLE_ID_ADRS2, &[w]);
        let work = c.game(mem, ATEL_GET_CAMERA_WORK_ADRS, &[w]);
        let mut cam: u32 = mem.read(work);
        if cam == 0 {
            cam = crate::camera::bridge::ms_atel_get_camera_id(mem, 5, 2, 0);
            mem.write(work, cam);
        }
        let hundred = f64_at(mem, HUNDRED);
        let d = c.float(mem, w, s);
        let mut dist = store(div(load(d), hundred));
        let yaw = c.float(mem, w, s);
        let pitch = c.float(mem, w, s);
        let x = c.float(mem, w, s);
        let mut x = store(div(load(x), hundred));
        let b = c.int(mem, w, s);
        let a = c.int(mem, w, s);
        let yaw = radians(mem, yaw);
        let pitch = radians(mem, pitch);
        let k = |mem: &M, sym: u32| f64_at(mem, sym);
        let hundredth = |v: u32| mul(load(v), hundred);
        match scene {
            0x00d4_0000 => {
                if c.game(mem, GRAPHIC_LEVEL_GET_NAME_HASH, &[]) == 0x200f
                    && magic == 0xfb
                    && hundredth(dist) == k(mem, C_MINUS_150)
                    && hundredth(x) == k(mem, C_89)
                {
                    let mut last = HIDE_QUEUE_LAST.get(mem);
                    while last != 0 {
                        let at = HIDE_QUEUE
                            .ptr(mem)
                            .addr
                            .wrapping_add(last.cast_unsigned().wrapping_mul(4));
                        let chr: u32 = mem.read(at);
                        c.game(mem, CH_SET_HIDE_BATTLE, &[chr, 1]);
                        last = HIDE_QUEUE_LAST.get(mem).wrapping_sub(1);
                        HIDE_QUEUE_LAST.set(mem, last);
                    }
                    HIDE_MASK.set(mem, 0);
                }
            }
            0x01a3_0004 => {
                if c.game(mem, MAGIC_FILE_GET_MAGIC_ID, &[]) == 0x58
                    && hundredth(dist) == k(mem, C_229)
                {
                    dist = store(sub(load(dist), k(mem, C_0_9)));
                }
            }
            0x01f6_0004 => {
                if c.game(mem, MAGIC_FILE_GET_MAGIC_ID, &[]) == 0xc9 && hundredth(x) == k(mem, C_4)
                {
                    x = store(add(load(x), k(mem, C_0_9)));
                }
            }
            0x025c_0013 if side == 1 && mode == 2 && (magic == 0x4c || magic == 0x137) => {
                let who = crate::actor::bridge::COMMAND_ACTOR.get(mem);
                if who != 0xff {
                    let chr: u32 = mem.read(crate::actor::bridge::ms_get_chr(mem, u32::from(who)));
                    if chr != u32::MAX
                        && c.game(mem, CH_GET_ID, &[chr]) == 0x10c1
                        && load(mem.read(mem.rebase(C_1_5_F32))) == load(dist)
                    {
                        dist = store(mul(load(dist), k(mem, C_3)));
                    }
                }
            }
            _ => {}
        }
        // |trunc(dist - by)| as an f32
        let off = |mem: &M, dist: u32, by: u32| {
            let t = ftol(mem, sub(load(dist), k(mem, by)))
                .cast_signed()
                .wrapping_abs();
            store(f64::from(t))
        };
        match magic {
            0xa3 => {
                if scene == 0x0159_0000
                    && side == 1
                    && mode == 2
                    && ftol(mem, add(hundredth(dist), k(mem, C_HALF))) == 0x208
                {
                    dist = store(sub(load(dist), k(mem, C_HALF)));
                }
            }
            0x2c1 => {
                if c.game(mem, MAGIC_FILE_GET_HANDLE, &[]) != u32::MAX {
                    let tiny = k(mem, C_TINY);
                    if tiny > load(off(mem, dist, C_30_49)) || load(off(mem, dist, C_29_49)) < tiny
                    {
                        dist = store(sub(load(dist), k(mem, C_3_5)));
                    }
                }
            }
            0x2c0 => {
                if c.game(mem, MAGIC_FILE_GET_HANDLE, &[]) != u32::MAX
                    && load(off(mem, dist, C_10_09)) < k(mem, C_TINY)
                {
                    x = store(add(load(x), k(mem, C_0_1)));
                }
            }
            0x92 if c.game(mem, MAGIC_FILE_GET_HANDLE, &[]) != u32::MAX
                && scene == 0x01ae_000a
                && ftol(mem, hundredth(x)) == 0x12
                && ftol(mem, hundredth(dist)) == 1000 =>
            {
                x = mem.read(mem.rebase(C_0X92_X_F32));
                dist = mem.read(mem.rebase(C_0X92_DIST_F32));
            }
            _ => {}
        }
        let chr = u32::from(mem.read::<u8>(actor));
        c.game(
            mem,
            MS_CAMERA_SET_BTL,
            &[cam, side, chr, mode, a, b, x, pitch, yaw, dist],
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

    /// `MsBindCamera(worker, kind, cam)` (`0x007ba9a0`): for kinds 0-3,
    /// [`ms_atel_get_camera`] (the original jumps to it); 0 for the others.
    pub fn ms_bind_camera<M: Mem>(
        mem: &mut M,
        c: &mut Calls<'_, M>,
        w: u32,
        kind: u32,
        cam: u32,
    ) -> u32 {
        if kind > 3 {
            return 0;
        }
        ms_atel_get_camera(mem, c, w, kind, cam)
    }

    /// `MsSplineRegist(cam, which)` (`0x007baac0`): for camera `cam` (as
    /// `MsCameraGetID` gives it back), a point appended to built-in spline
    /// `which & 1` (eye 1, focus 0; at most 0x20 points: the count at
    /// [`SPLINE_COUNTS`] grows first, the point at `+0x10 * count` of the
    /// spline's record): the camera side's position (`g_Cameras + 0xda0 *
    /// (cam & 0xff) + 0x61c * which + 0x80`, three words through
    /// `fld`/`fstp`) and the camera's `+0xc94` angle near 0
    /// (`MsNearRag(0, angle)`). Its `eax` is never read (the eye and focus
    /// path natives drop it).
    pub fn ms_spline_regist(mem: &mut impl Mem, cam: u32, which: u32) {
        use ffx_bridge::x87::round_trip;
        if crate::camera::bridge::ms_camera_get_id(mem, cam) != cam {
            return;
        }
        let which = which & 1;
        let at = mem.rebase(SPLINE_COUNTS) + 4 * which;
        let n: i32 = mem.read(at);
        if n >= 0x20 {
            return;
        }
        let n = n + 1;
        mem.write(at, n);
        let camera = crate::camera::bridge::CAMERAS
            .ptr(mem)
            .addr
            .wrapping_add((cam & 0xff) * 0xda0);
        let side = camera.wrapping_add(which * 0x61c);
        let point = mem.rebase(SPLINE_RECORDS).wrapping_add(
            (which * 0x21)
                .wrapping_add(n.cast_unsigned())
                .wrapping_mul(0x10),
        );
        for k in 0..3 {
            let v: f32 = mem.read(side.wrapping_add(0x80 + 4 * k));
            mem.write(point.wrapping_add(4 * k), round_trip(v));
        }
        let angle: f32 = mem.read(camera.wrapping_add(0xc94));
        let near = crate::vector::bridge::ms_near_rag(mem, 0, round_trip(angle).to_bits());
        mem.write(point.wrapping_add(0xc), near);
    }

    /// `Cam_SetPolar(id, side, a, b, c)` and `Cam_AddPolar` (the camera's
    /// polar aim, set or added).
    pub const CAM_SET_POLAR: u32 = 0x007b_f700;
    pub const CAM_ADD_POLAR: u32 = 0x007b_f7f0;

    /// `MsAtelCameraPolar_007bb5d0(worker, _, stack, side, add)`
    /// (`0x007bb5d0`, name ours; callers `Native_CamSetPolar`,
    /// `Native_Cam_6005`, `Native_6022_CamRefSetPolar`, `Native_6023_CamRefAddPolar`): three floats
    /// popped (`r`, then the degrees `b` and `a`) to the worker's camera's
    /// polar aim on `side`: `Cam_SetPolar(camera, side, rad(a), rad(b), r)`,
    /// or with `add` `Cam_AddPolar`, whose `eax` it returns.
    pub fn ms_atel_camera_polar<M: Mem>(
        mem: &mut M,
        c: &mut Calls<'_, M>,
        [w, s, side, add]: [u32; 4],
    ) -> u32 {
        let id = camera(mem, c, w);
        let [r, b, a] = c.floats::<3>(mem, w, s);
        let b = radians(mem, b);
        let a = radians(mem, a);
        let f = if add == 0 {
            CAM_SET_POLAR
        } else {
            CAM_ADD_POLAR
        };
        c.game(mem, f, &[id, side, a, b, r])
    }

    /// `g_GraphicMapId` (graphics) and the special cases of
    /// [`set_camera_matrix_to_phyre_scene`]: two battle scenes
    /// (`MsGetBattleScene`), their levels (`graphicLevelGetNameHash`), the
    /// story step (`g_SaveScenario`) and the magic file's ID.
    pub const GRAPHIC_MAP_ID: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x0133_0844);
    pub const SCENE_D4: u32 = 0x00d4_0000;
    pub const SCENE_139: u32 = 0x0139_0000;
    /// The three positions the special cases look for, `(x, y, z)` as
    /// `f32` bits (the first's z the `f64` 480.0 at `0x00b55138`), and the
    /// x the last one gets.
    pub const RESET_POS: [u32; 3] = [0xc3c3_0000, 0xc2a4_0000, 0x43f0_0000];
    pub const HIDE_POS: [u32; 3] = [0x4324_afc0, 0xc214_28f6, 0x4242_ad72];
    pub const FIXED_X: u32 = 0xc288_1cb3;
    /// `MsResetSceneObjects_0079e2a0()` (its shim makes its frame) and
    /// `MsCameraSetRect(id, side, p)`.
    pub const MS_RESET_SCENE_OBJECTS: u32 = 0x0079_e2a0;
    pub const MS_CAMERA_SET_RECT: u32 = 0x007b_f940;

    /// `setCameraMatrixToPhyreScene(worker, _, stack, side)` (`0x007bb6a0`,
    /// natives `Native_CamSetPos`, `Native_CamRefSetPos`): the worker's
    /// camera's side `side` placed (`MsCameraSetRect`) at the three floats
    /// popped (`z`, `y`, `x`; into `local`, the original's frame, whose
    /// fourth word it leaves as it is). Three fixes: at (-390, -82, 480) in
    /// level 0x6e1 at story step 0x104 the scene objects are reset
    /// (`MsResetSceneObjects_0079e2a0`, through the game); at [`HIDE_POS`] in battle scene
    /// [`SCENE_D4`], level 0x200f, magic file 0xfa the hide mask is set
    /// ([`crate::visible::bridge::HIDE_MASK`] 1); side 1 on map 0xf8 in
    /// scene [`SCENE_139`] with `ftol(x * 1000)` -71056 moves to x
    /// [`FIXED_X`]. `eax` `MsCameraSetRect`'s (the battle scene, the level,
    /// the magic file and `MsCameraSetRect` through the game).
    pub fn set_camera_matrix_to_phyre_scene<M: Mem>(
        mem: &mut M,
        c: &mut Calls<'_, M>,
        [w, s, side]: [u32; 3],
        local: u32,
    ) -> u32 {
        use ffx_bridge::x87::{ftol, load, mul};
        let id = camera(mem, c, w);
        let [z, y, x] = c.floats::<3>(mem, w, s);
        mem.write(local, [x, y, z]);
        // exactly, as the original's fucompp
        #[allow(clippy::float_cmp)]
        let at = |v: [u32; 3]| [x, y, z].iter().zip(v).all(|(&a, b)| load(a) == load(b));
        if at(RESET_POS)
            && c.game(mem, GRAPHIC_LEVEL_GET_NAME_HASH, &[]) == 0x6e1
            && crate::save::bridge::SCENARIO.get(mem) == 0x104
        {
            c.game(mem, MS_RESET_SCENE_OBJECTS, &[]);
        }
        if c.game(mem, MS_GET_BATTLE_SCENE, &[]) == SCENE_D4
            && c.game(mem, GRAPHIC_LEVEL_GET_NAME_HASH, &[]) == 0x200f
            && c.game(mem, MAGIC_FILE_GET_MAGIC_ID, &[]) == 0xfa
            && at(HIDE_POS)
        {
            crate::visible::bridge::HIDE_MASK.set(mem, 1);
        }
        if side == 1
            && GRAPHIC_MAP_ID.get(mem) == 0xf8
            && c.game(mem, MS_GET_BATTLE_SCENE, &[]) == SCENE_139
            && ftol(mem, mul(load(x), 1000.0)) == 0xfffe_ea70
        {
            mem.write(local, FIXED_X);
        }
        c.game(mem, MS_CAMERA_SET_RECT, &[id, side, local])
    }
}
