//! The battle screens' moves and links (`docs/rewrite-plan.md`, M2).
//!
//! A screen is `0x1c0` bytes from `0x01137000` ([`crate::camera::bridge::SCREENS`],
//! three of them). Each of its two sides (`side & 1`) has a position at
//! `+0x40` + 16 side, a saved one at `+0x80` + 16 side and the move's start
//! at `+0xa0` + 16 side, and four move records of 0x10 bytes at `+0xdc` +
//! 0x40 side, one per lane: `+0` the kind (2 accelerated), `+4` the frame,
//! `+6`, `+8`, `+0xa` the accelerating, cruising and braking frames,
//! `+0xc` the total (0 idle), `+0xe` cleared. Bytes from `+0xc2` link
//! screens.

/// The game's memory, through [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    use crate::camera::bridge::{EE_MOVE, SCREEN_SIZE, SCREENS, ms_camera_get_id};
    use crate::camera::x87_copy;

    /// `g_BtlScreen` (`0x023114c0`): the screen in use.
    pub const BTL_SCREEN: Global<u32> = Global::new(0x0231_14c0);

    /// Screen `i`.
    fn screen(mem: &impl Mem, i: u32) -> u32 {
        SCREENS
            .ptr(mem)
            .addr
            .wrapping_add(i.wrapping_mul(SCREEN_SIZE))
    }

    /// Screen `i`'s side `side`'s first move record.
    fn moves(mem: &impl Mem, i: u32, side: u32) -> u32 {
        screen(mem, i) + 0xdc + 0x40 * side
    }

    /// The side's position into the EE's move register (`0x00c8f588`) and
    /// the move's start (`+0xa0` + 16 side), as the PS2 code's `lq`/`sq`.
    fn mark_start(mem: &mut impl Mem, i: u32, side: u32) {
        let s = screen(mem, i);
        let pos: [u32; 4] = mem.read(s + 0x40 + 0x10 * side);
        EE_MOVE.set(mem, pos);
        mem.write(s + 0xa0 + 0x10 * side, pos);
    }

    /// `MsScreenLinkSet(i, k, v)` (`0x007bbdd0`): byte `+0xc2 + k` of
    /// screen `i` (below 3) = `v`.
    pub fn ms_screen_link_set(mem: &mut impl Mem, i: u32, k: u32, v: u8) {
        if i <= 2 {
            let at = screen(mem, i).wrapping_add(0xc2).wrapping_add(k);
            mem.write(at, v);
        }
    }

    /// `MsScreenOffsetID(id, x, y)` (`0x007bc0e0`): when `id` is its
    /// camera's current request ([`ms_camera_get_id`]), the screen in use
    /// gets the offset (`+0x30`, `+0x34`, `f32`). Returns `eax` as the
    /// original leaves it: `g_BtlScreen`, or the current request.
    pub fn ms_screen_offset_id(mem: &mut impl Mem, id: u32, x: u32, y: u32) -> u32 {
        let current = ms_camera_get_id(mem, id);
        if current != id {
            return current;
        }
        let s = BTL_SCREEN.get(mem);
        mem.write(s.wrapping_add(0x30), x87_copy(x));
        let s = BTL_SCREEN.get(mem);
        mem.write(s.wrapping_add(0x34), x87_copy(y));
        s
    }

    /// `MsScreenMoveCheck(i, side)` (`0x007bbfc0`): how many of the side's
    /// four lanes still move (total not 0); 0 for `i` above 2.
    pub fn ms_screen_move_check(mem: &impl Mem, i: u32, side: u32) -> u32 {
        if i > 2 {
            return 0;
        }
        let m = moves(mem, i, side & 1);
        (0..4)
            .map(|k| u32::from(mem.read::<u16>(m + 0x10 * k + 0xc) != 0))
            .sum()
    }

    /// `MsScreenMoveReset(i, side)` (`0x007bc010`): stops the side's four
    /// lanes (kind and total 0) and puts its saved position (`+0x80`) back
    /// from the position (`+0x40`), lane by lane.
    pub fn ms_screen_move_reset(mem: &mut impl Mem, i: u32, side: u32) {
        if i > 2 {
            return;
        }
        let side = side & 1;
        let s = screen(mem, i);
        let m = moves(mem, i, side);
        for k in 0..4 {
            mem.write(m + 0x10 * k, 0u16);
            mem.write(m + 0x10 * k + 0xc, 0u32);
            let v: u32 = mem.read(s + 0x40 + 0x10 * side + 4 * k);
            mem.write(s + 0x80 + 0x10 * side + 4 * k, x87_copy(v));
        }
    }

    /// `MsScreenMove(i, side, total, kind)` (`0x007bbdf0`): every lane of
    /// the side moves `kind` over `total` frames (the frame kept, `+0xe`
    /// cleared), from the side's position now.
    pub fn ms_screen_move(mem: &mut impl Mem, i: u32, side: u32, total: u16, kind: u16) {
        if i > 2 {
            return;
        }
        let side = side & 1;
        let m = moves(mem, i, side);
        for k in 0..4 {
            mem.write(m + 0x10 * k + 0xc, total);
            mem.write(m + 0x10 * k, kind);
            mem.write(m + 0x10 * k + 0xe, 0u16);
        }
        mark_start(mem, i, side);
    }

    /// `MsScreenMoveAcc(i, side, accel, cruise, brake)` (`0x007bbeb0`):
    /// every lane of the side moves accelerated (kind 2) from frame 0 over
    /// `accel + cruise + brake` frames (`i16` sums), from the side's
    /// position now.
    pub fn ms_screen_move_acc(
        mem: &mut impl Mem,
        i: u32,
        side: u32,
        accel: u16,
        cruise: u16,
        brake: u16,
    ) {
        if i > 2 {
            return;
        }
        let side = side & 1;
        let m = moves(mem, i, side);
        let total = brake.wrapping_add(cruise).wrapping_add(accel);
        for k in 0..4 {
            let r = m + 0x10 * k;
            mem.write(r, 2u16);
            mem.write(r + 4, 0u16);
            mem.write(r + 6, accel);
            mem.write(r + 8, cruise);
            mem.write(r + 0xa, brake);
            mem.write(r + 0xc, total);
            mem.write(r + 0xe, 0u16);
        }
        mark_start(mem, i, side);
    }

    /// What [`ms_screen_process`] calls outside the module: the C runtime
    /// on doubles (`MsCameraMoveCalc`'s cosine, `MsCameraShakeProcess`),
    /// `_CIcos`/`_CIsin` (`MsCameraMatrix`), and the renderer's and other
    /// modules' functions through the game.
    pub struct ScreenEnv<'e, M> {
        pub trig: &'e dyn Fn(crate::pos::bridge::Trig, u32) -> u32,
        pub trig64: &'e dyn Fn(crate::pos::bridge::Trig, f64) -> u32,
        pub cos: &'e dyn Fn(f64) -> f64,
        /// An original of this build (cdecl, word arguments): its `eax`.
        pub call: &'e mut dyn FnMut(&mut M, u32, &[u32]) -> u32,
        /// An original returning an `f32` on the x87 stack: its bits.
        pub fcall: &'e mut dyn FnMut(&mut M, u32, &[u32]) -> u32,
    }

    /// `graphicLevelGetNameHash()` (render).
    pub const LEVEL_NAME_HASH: u32 = 0x0064_1da0;
    /// `MagicFile_GetMagicId()` (main).
    pub const MAGIC_ID: u32 = 0x009d_a3b0;
    /// `graphicCameraSetWorldMatrixSub(a, b)` (render).
    pub const SET_WORLD_MATRIX_SUB: u32 = 0x0063_9b30;
    /// `getNearPlane()` (render; an `f32` in ST0).
    pub const GET_NEAR_PLANE: u32 = 0x0063_8e70;
    /// `setNearPlane(f32)` (render).
    pub const SET_NEAR_PLANE: u32 = 0x0064_5f00;
    /// `FFXVu0InnerProduct(a, b)` (an `f32` in ST0).
    pub const INNER_PRODUCT: u32 = 0x0062_fce0;
    /// `graphicScreenSetOffset(x, y)` (render, two `f32`).
    pub const SET_SCREEN_OFFSET: u32 = 0x0064_2d60;
    /// `graphicScreenSetDttOffset(x, y)` (render, two `f32`).
    pub const SET_DTT_OFFSET: u32 = 0x0064_2d40;
    /// `MagicFile_NeedRestoreCamera()` (main).
    pub const NEED_RESTORE_CAMERA: u32 = 0x009d_a700;
    /// `Copy_009da750()` (main): a magic's saved matrices into the view.
    pub const MAGIC_COPY_MATRICES: u32 = 0x009d_a750;
    /// `Cam_Apply(camera)` (`0x007bc980`): the view into the renderer.
    pub const CAM_APPLY: u32 = 0x007b_c980;
    /// `MagicFile_RestoreScreenSetup()` (main).
    pub const RESTORE_SCREEN_SETUP: u32 = 0x009d_a780;
    /// `Sg_SetFocus(k, v)` (main).
    pub const SG_SET_FOCUS: u32 = 0x0082_ce20;

    /// 2.0 (`0x00b92270`).
    pub const TWO: Global<f64> = Global::new(0x00b9_2270);
    /// 416.0 (`0x00b41d30`), the PS2 screen's height in the offset.
    pub const OFFSET_HEIGHT: Global<f64> = Global::new(0x00b4_1d30);
    /// 1/512 (`0x00b41cc0`), the offset's x scale.
    pub const OFFSET_X_SCALE: Global<f64> = Global::new(0x00b4_1cc0);
    /// 0.01 (`0x00b1f2b0`, `f32`) and 1e9 (`0x00b55170`): the bounds the
    /// eye's and look-at's squared lengths must lie within.
    pub const VIEW_MIN: Global<u32> = Global::new(0x00b1_f2b0);
    pub const VIEW_MAX: Global<f64> = Global::new(0x00b5_5170);
    /// `0x00c44238`: the near plane is the default one (cleared when map
    /// 0xd0 of magic 0x21c pushed it 2.0 out).
    pub const NEAR_DEFAULT: Global<u8> = Global::new(0x00c4_4238);
    /// `g_GraphicMapId` (`0x01330844`).
    pub const GRAPHIC_MAP_ID: Global<u32> = Global::new(0x0133_0844);
    /// `g_BtlEventEffectClear` (`0x00c421e4`): with `0x00c421e8` both
    /// negative, the levels named 0x1505, 0x822 and 0x1883 build the camera
    /// matrix only from sane points.
    pub const BTL_EVENT_EFFECT_CLEAR: Global<i32> = Global::new(0x00c4_21e4);

    /// `MsScreenProcess(camera)` (`0x007bc110`): the screen `camera`'s
    /// frame. `g_CameraSelected` is -1, and stays so returning 1 for a
    /// screen past 2 (signed); magics 0x98 and 0x259 on screen 1 while
    /// screen 1 is off reset the renderer's world matrix. An off screen
    /// returns -1. Else its camera becomes `g_Camera` (and
    /// `g_CameraSelected`), the screen `g_BtlScreen`; for each of its two
    /// sides the position (`+0x40`) is the saved one (`+0x80`, through
    /// `g_Ee_GprMove`) and the centre (`+0x60`) half the size (`+0x58`
    /// counting down) less the display's; links (`+0xc2`) 1 take the size
    /// into `+0x48`, 2 the other side's corner as the position (centre
    /// 0), then four lanes move (`MsCameraMoveCalc` from `+0xa0`, records
    /// `+0xdc`), and link 3 centres the position. Two shakes (`+0x16c`,
    /// angle `+0x15c` stepped by `+0x164`) give `+0x30`; the positions and
    /// centres are truncated to integers. The view: the override camera's
    /// (`MsScreenMatrix`, `MsCameraMatrix` from the override's eye,
    /// look-at, up and roll), or with the camera matrices on only the
    /// renderer's screen offset, or the camera's (the near plane for
    /// magic 0x21c on map 0xd0, the matrix from the camera's eye and
    /// look-at unless the three special levels' points are out of
    /// bounds, the screen offset, `Cam_Apply` around a magic's saved
    /// matrices). A camera with focus (`+0xca8`) whose focus words
    /// (`+0xcac`..) differ from the screen's (`+0xd0`..) hands them to the
    /// scene graph (`Sg_SetFocus` 0 and 1, `Sg_EnFocus`). 0.
    #[allow(clippy::too_many_lines)]
    pub fn ms_screen_process<M: Mem>(mem: &mut M, env: &mut ScreenEnv<'_, M>, camera: i32) -> i32 {
        use crate::camera::bridge as cam;
        mem.write(cam::SELECTED.ptr(mem).addr, u32::MAX);
        if camera >= 3 {
            return 1;
        }
        let hash = (env.call)(mem, game(mem, LEVEL_NAME_HASH), &[]);
        let magic = (env.call)(mem, game(mem, MAGIC_ID), &[]);
        let ms_magic = magic == 0x98 || (env.call)(mem, game(mem, MAGIC_ID), &[]) == 0x259;
        if ms_magic && camera == 1 && mem.read::<u8>(screen(mem, 1) + 0xc0) == 0 {
            (env.call)(mem, game(mem, SET_WORLD_MATRIX_SUB), &[0, 0]);
        }
        let s = screen(mem, camera.cast_unsigned());
        if mem.read::<u8>(s + 0xc0) == 0 {
            return -1;
        }
        let c = u32::from(mem.read::<u8>(s + 0xc1));
        mem.write(cam::SELECTED.ptr(mem).addr, c);
        cam::ms_camera_get_num(mem, c);
        let camera_at = cam::CAMERAS
            .ptr(mem)
            .addr
            .wrapping_add(c.wrapping_mul(cam::CAMERA_SIZE));
        BTL_SCREEN.set(mem, s);
        cam::CAMERA.set(mem, camera_at);
        sides(mem, env, s);
        shakes(mem, env, s);
        truncate(mem, s);
        if cam::OVERRIDE_ON.get(mem) != 0 {
            crate::view::bridge::ms_screen_matrix(mem, cam::SCREEN_MATRIX.ptr(mem).addr, s);
            let roll = x87_copy(cam::OVERRIDE_ROLL.get(mem));
            let [eye, look_at, up] = [
                cam::OVERRIDE_EYE.ptr(mem).addr,
                cam::OVERRIDE_LOOK_AT.ptr(mem).addr,
                cam::OVERRIDE_UP.ptr(mem).addr,
            ];
            let out = cam::CAMERA_MATRIX.ptr(mem).addr;
            crate::view::bridge::ms_camera_matrix(mem, env.trig, out, eye, look_at, up, roll);
        } else if cam::MATRIX_ON.get(mem) == 0 {
            camera_view(mem, env, s, hash);
            offsets(mem, env, Some(s));
            let restore = (env.call)(mem, game(mem, NEED_RESTORE_CAMERA), &[]);
            if restore != 0 {
                (env.call)(mem, game(mem, MAGIC_COPY_MATRICES), &[]);
            }
            (env.call)(mem, game(mem, CAM_APPLY), &[camera.cast_unsigned()]);
            if restore != 0 {
                (env.call)(mem, game(mem, RESTORE_SCREEN_SETUP), &[]);
            }
        } else {
            offsets(mem, env, None);
        }
        focus(mem, env);
        0
    }

    /// `f`, an address of this build, where the game is loaded.
    fn game(mem: &impl Mem, f: u32) -> u32 {
        f.wrapping_sub(ffx_bridge::IMAGE_BASE)
            .wrapping_add(mem.image_base())
    }

    /// [`ms_screen_process`]'s two sides of screen `s`.
    fn sides<M: Mem>(mem: &mut M, env: &mut ScreenEnv<'_, M>, scr: u32) {
        use ffx_bridge::x87::{add, load, mul, store, sub};
        let half = crate::target::bridge::HALF.get(mem);
        let size = |mem: &M, k: usize| {
            let v = crate::camera::bridge::DISPLAY_SIZE.get(mem)[k];
            load(store(f64::from(v)))
        };
        for j in 0..2u32 {
            let pos = scr + 0x40 + 0x10 * j;
            let corner = scr + 0x50 - 0x10 * j;
            let saved: [u32; 4] = mem.read(scr + 0x80 + 0x10 * j);
            EE_MOVE.set(mem, saved);
            mem.write(pos, saved);
            let centre_x = store(mul(sub(load(mem.read(corner + 8)), size(mem, 0)), half));
            mem.write(pos + 0x20, centre_x);
            let centre_y = store(mul(half, sub(load(mem.read(corner + 0xc)), size(mem, 1))));
            mem.write(pos + 0x24, centre_y);
            let link = scr + 0xc2 + j;
            match mem.read::<u8>(link) {
                1 => {
                    let width = x87_copy(mem.read(corner + 8));
                    mem.write(pos + 8, width);
                    let height = x87_copy(mem.read(corner + 0xc));
                    mem.write(pos + 0xc, height);
                }
                2 => {
                    mem.write(pos + 0x20, [0.0f32.to_bits(); 2]);
                    let p: [u32; 4] = mem.read(corner);
                    EE_MOVE.set(mem, p);
                    mem.write(pos, p);
                }
                _ => {}
            }
            for i in 0..4 {
                let start = x87_copy(mem.read(pos + 0x60 + 4 * i));
                let lane = scr + 0xdc + 0x40 * j + 0x10 * i;
                crate::camera::bridge::ms_camera_move_calc(
                    mem,
                    env.cos,
                    0,
                    pos + 4 * i,
                    start,
                    lane,
                );
            }
            if mem.read::<u8>(link) == 3 {
                let left = store(add(
                    mul(sub(size(mem, 0), load(mem.read(pos + 8))), half),
                    load(mem.read(pos)),
                ));
                mem.write(pos, left);
                let top = store(add(
                    mul(sub(size(mem, 1), load(mem.read(pos + 0xc))), half),
                    load(mem.read(pos + 4)),
                ));
                mem.write(pos + 4, top);
            }
        }
    }

    /// [`ms_screen_process`]'s two shakes of screen `s` (records `+0x16c`,
    /// angles `+0x15c` stepped by `+0x164`, results `+0x30`).
    fn shakes<M: Mem>(mem: &mut M, env: &mut ScreenEnv<'_, M>, s: u32) {
        use ffx_bridge::x87::{add, load, store};
        for i in 0..2 {
            let rec = s + 0x16c + 0x24 * i;
            let angle = s + 0x15c + 4 * i;
            if mem.read::<u8>(rec) != 0 {
                let a = x87_copy(mem.read(angle));
                crate::shake::bridge::ms_camera_shake_process(
                    mem,
                    env.trig64,
                    rec,
                    a,
                    s + 0x30 + 4 * i,
                );
                let a = store(add(load(mem.read(angle + 8)), load(mem.read(angle))));
                mem.write(angle, a);
            }
        }
    }

    /// [`ms_screen_process`]'s truncation of each side's position and
    /// size (`+0x40` + 16 side, four words) and centre (`+0x60`, two) to
    /// integers (`_ftol2_sse`, back as `f32`).
    fn truncate(mem: &mut impl Mem, s: u32) {
        use ffx_bridge::x87::{ftol, load, store};
        for j in 0..2u32 {
            for o in [0x40, 0x44, 0x48, 0x4c, 0x60, 0x64] {
                let at = s + o + 0x10 * j;
                let n = ftol(mem, load(mem.read(at))).cast_signed();
                let v = store(f64::from(n));
                mem.write(at, v);
            }
        }
    }

    /// [`ms_screen_process`]'s view from the camera (`g_Camera`): the near
    /// plane for magic 0x21c on map 0xd0 (2.0 further once, until another
    /// magic or map restores 1.0), the screen and camera matrices (for the
    /// levels named 0x1505, 0x822 and 0x1883, with `0x00c421e8` and
    /// `0x00c421e4` negative, only when the eye's and look-at's squared
    /// lengths lie strictly between 0.01 and 1e9).
    fn camera_view<M: Mem>(mem: &mut M, env: &mut ScreenEnv<'_, M>, s: u32, hash: u32) {
        use crate::camera::bridge as cam;
        use ffx_bridge::x87::{add, load, store};
        crate::view::bridge::ms_screen_matrix(mem, cam::SCREEN_MATRIX.ptr(mem).addr, s);
        let magic = (env.call)(mem, game(mem, MAGIC_ID), &[]);
        if magic == 0x21c && GRAPHIC_MAP_ID.get(mem) == 0xd0 {
            if NEAR_DEFAULT.get(mem) != 0 {
                let near = (env.fcall)(mem, game(mem, GET_NEAR_PLANE), &[]);
                let near = store(add(load(near), TWO.get(mem)));
                (env.call)(mem, game(mem, SET_NEAR_PLANE), &[near]);
                NEAR_DEFAULT.set(mem, 0);
            }
        } else if NEAR_DEFAULT.get(mem) == 0 {
            (env.call)(mem, game(mem, SET_NEAR_PLANE), &[1.0f32.to_bits()]);
            NEAR_DEFAULT.set(mem, 1);
        }
        let matrix = |mem: &mut M, env: &mut ScreenEnv<'_, M>| {
            let c = cam::CAMERA.get(mem);
            let roll = x87_copy(mem.read(c + 0xc90));
            let out = cam::CAMERA_MATRIX.ptr(mem).addr;
            crate::view::bridge::ms_camera_matrix(mem, env.trig, out, c + 0x65c, c + 0x40, c, roll);
        };
        if !matches!(hash, 0x1505 | 0x822 | 0x1883) {
            matrix(mem, env);
            return;
        }
        let read_clear = crate::parse::bridge::EFFECT_READ_CLEAR
            .get(mem)
            .cast_signed();
        if read_clear >= 0 || BTL_EVENT_EFFECT_CLEAR.get(mem) >= 0 {
            return;
        }
        let c = cam::CAMERA.get(mem);
        let eye = (env.fcall)(mem, game(mem, INNER_PRODUCT), &[c + 0x65c, c + 0x65c]);
        let c = cam::CAMERA.get(mem);
        let look_at = (env.fcall)(mem, game(mem, INNER_PRODUCT), &[c + 0x40, c + 0x40]);
        let (lo, hi) = (load(VIEW_MIN.get(mem)), VIEW_MAX.get(mem));
        let inside = |v: u32| load(v) > lo && hi > load(v);
        if inside(eye) && inside(look_at) {
            matrix(mem, env);
        }
    }

    /// [`ms_screen_process`]'s renderer screen offset: `x = (screen +0x70 +
    /// +0x30 − +0x50 + offset x) · 2 / 512`, `y = (+0x74 − +0x34 − +0x54 +
    /// offset y) · 2 / 416` and the dither offset (`+0x30`, `+0x34`); with
    /// no screen (the camera matrices on) the offset alone.
    fn offsets<M: Mem>(mem: &mut M, env: &mut ScreenEnv<'_, M>, s: Option<u32>) {
        use ffx_bridge::x87::{add, div, load, mul, store, sub};
        let two = TWO.get(mem);
        let [off_x, off_y] = crate::misc::bridge::SCREEN_OFFSET.get(mem);
        let f = |mem: &M, at: u32| load(mem.read(at));
        let (xs, ys) = match s {
            Some(s) => (
                add(
                    sub(add(f(mem, s + 0x70), f(mem, s + 0x30)), f(mem, s + 0x50)),
                    load(off_x),
                ),
                add(
                    sub(sub(f(mem, s + 0x74), f(mem, s + 0x34)), f(mem, s + 0x54)),
                    load(off_y),
                ),
            ),
            None => (load(off_x), load(off_y)),
        };
        let y = store(div(mul(ys, two), OFFSET_HEIGHT.get(mem)));
        let x = store(mul(mul(two, xs), OFFSET_X_SCALE.get(mem)));
        (env.call)(mem, game(mem, SET_SCREEN_OFFSET), &[x, y]);
        if let Some(s) = s {
            let dx = x87_copy(mem.read(s + 0x30));
            let dy = x87_copy(mem.read(s + 0x34));
            (env.call)(mem, game(mem, SET_DTT_OFFSET), &[dx, dy]);
        }
    }

    /// [`ms_screen_process`]'s focus: a camera with focus (`+0xca8`) whose
    /// words (`+0xcac`..`+0xcb4`) differ from `g_BtlScreen`'s (`+0xd0`..)
    /// gives them to it and to the scene graph.
    fn focus<M: Mem>(mem: &mut M, env: &mut ScreenEnv<'_, M>) {
        use crate::camera::bridge as cam;
        let c = cam::CAMERA.get(mem);
        if mem.read::<u32>(c + 0xca8) == 0 {
            return;
        }
        let s = BTL_SCREEN.get(mem);
        let want: [u32; 3] = mem.read(c + 0xcac);
        if mem.read::<[u32; 3]>(s + 0xd0) == want {
            return;
        }
        mem.write(s + 0xd0, want[0]);
        let v: u32 = mem.read(cam::CAMERA.get(mem) + 0xcb0);
        mem.write(BTL_SCREEN.get(mem) + 0xd4, v);
        let v: u32 = mem.read(cam::CAMERA.get(mem) + 0xcb4);
        mem.write(BTL_SCREEN.get(mem) + 0xd8, v);
        let v: u32 = mem.read(BTL_SCREEN.get(mem) + 0xd4);
        (env.call)(mem, game(mem, SG_SET_FOCUS), &[0, v]);
        let v: u32 = mem.read(BTL_SCREEN.get(mem) + 0xd8);
        (env.call)(mem, game(mem, SG_SET_FOCUS), &[1, v]);
        let v: u32 = mem.read(BTL_SCREEN.get(mem) + 0xd0);
        let f = crate::chr_damage::bridge::SG_EN_FOCUS;
        (env.call)(mem, game(mem, f), &[v]);
    }
}
