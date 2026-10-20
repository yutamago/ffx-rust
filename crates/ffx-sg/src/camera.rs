//! The main module's camera and screen set-up: the screen matrix (through
//! the battle camera's `MsSetScreen`), the view clip matrix, the camera
//! position, reference point and look vector, the screen centre, and two
//! accessors of the battle cameras (`Cam_*`).

/// The originals over game memory (`ffx_bridge::Mem`).
#[allow(clippy::many_single_char_names)] // matrix entries and coordinates
pub mod bridge {
    use ffx_battle::camera::bridge as battle;
    use ffx_bridge::x87::{self, Reg};
    use ffx_bridge::{Global, Host, Mem};

    /// The screen centre, x and y (`f32`, `Sg_SetScreenCenter`).
    pub const SCREEN_CENTER: Global<[f32; 2]> = Global::new(0x0130_1bbc);
    /// The camera position (`Sg_CameraGetPos` fills it from the battle camera).
    pub const CAMERA_POS: Global<[f32; 3]> = Global::new(0x00c4_dec0);
    /// The camera's reference (look-at) point (`Sg_CameraGetRef`).
    pub const CAMERA_REF: Global<[f32; 3]> = Global::new(0x00c4_ded0);
    /// The look vector, reference minus position, and a fourth word
    /// (`Sg_GetLookVector`).
    pub const LOOK: Global<[u32; 4]> = Global::new(0x0130_1bd0);
    /// `Sg_SetScreenCenter`'s offset (`f64` 2048.0 in `.rdata`).
    pub const CENTER_BASE: Global<f64> = Global::new(0x00b5_1bc0);
    /// `Sg_CalcScreenMatrix`'s constants (`f32` in `.rdata`), in
    /// `MsSetScreen`'s argument order after the depth and the centre.
    pub const SCREEN_ARGS: [Global<f32>; 4] = [
        Global::new(0x00b3_4254),
        Global::new(0x00b5_5198),
        Global::new(0x00b5_519c),
        Global::new(0x00b4_3ddc),
    ];
    /// `Sg_CalcViewClipMatrix`'s half extents: without and with its flag
    /// (`f32` in `.rdata`).
    pub const CLIP_EXTENT: [Global<f32>; 2] = [Global::new(0x00b4_998c), Global::new(0x00b4_9994)];
    /// `Sg_CalcViewClipMatrix`'s numerator (`f64` in `.rdata`).
    pub const CLIP_SCALE: Global<f64> = Global::new(0x00b9_2270);
    /// `Sg_CalcViewClipMatrix`'s `m[2][2]` and `m[3][2]` (`f32` in `.rdata`).
    pub const CLIP_Z: [Global<f32>; 2] = [Global::new(0x00b5_a470), Global::new(0x00b5_a498)];
    /// The battle cameras' positions: `g_Cameras + 0x50`, a camera every
    /// `0xda0` bytes, a side every `0x61c`.
    pub const CAMERA_POS_NOW: Global<f32> = Global::new(0x0113_7590);
    /// The battle cameras' channel table: four `(id, suspended)` pairs of
    /// 8 bytes per camera (`Cam_SetChannelSuspended`).
    pub const CHANNELS: Global<u32> = Global::new(0x0113_81a8);

    /// `g_Screens`: three screens of 0x1c0 bytes; `+0xc0` (a byte) says
    /// whether the screen views one of the battle cameras, `+0xc1`
    /// (`g_DebugCamera` for the first) which.
    pub const SCREENS: u32 = 0x0113_7000;
    pub const SCREEN_SIZE: u32 = 0x1c0;
    pub const SCREEN_CAMERA_ON: u32 = 0xc0;
    pub const SCREEN_CAMERA: u32 = 0xc1;

    /// `Cam_SetScreenCameraOn(screen, on)` (`0x007bc930`): the screen's
    /// camera byte (`+0xc0`) for screens 0..=2, the others ignored.
    pub fn set_screen_camera_on(mem: &mut impl Mem, screen: u32, on: u8) {
        if screen <= 2 {
            let at = mem.rebase(SCREENS) + SCREEN_SIZE * screen + SCREEN_CAMERA_ON;
            mem.write(at, on);
        }
    }

    /// `Cam_SetScreenCamera(screen, on, camera)` (`0x007bc950`): the
    /// screen's camera byte (`+0xc0`) and camera (`+0xc1`) for screens
    /// 0..=2, the others ignored.
    pub fn set_screen_camera(mem: &mut impl Mem, screen: u32, on: u8, camera: u8) {
        if screen <= 2 {
            let at = mem.rebase(SCREENS) + SCREEN_SIZE * screen;
            mem.write(at + SCREEN_CAMERA_ON, on);
            mem.write(at + SCREEN_CAMERA, camera);
        }
    }

    /// The EE register (`g_Ee_GprMove`, four words) the PS2 code moved
    /// vectors through; the PC keeps the stores.
    pub const EE_GPR_MOVE: u32 = 0x00c8_f588;
    /// A screen's vector (four `f32`) [`set_screen_vector`] sets, and the
    /// copy it keeps at `+0x40`; its word at `+0xc4`.
    pub const SCREEN_VECTOR: u32 = 0x80;
    pub const SCREEN_VECTOR_COPY: u32 = 0x40;
    pub const SCREEN_VECTOR_WORD: u32 = 0xc4;

    /// `Cam_SetScreenVector(screen, x, y, z, w, v)` (`0x007bc890`, name
    /// ours; `Native_604e` calls it): for screens 0..=2 (unsigned), the
    /// four `i32`s as `f32` (`fild`, `fstp dword`) into the screen's
    /// `+0x80`, copied through [`EE_GPR_MOVE`] and into `+0x40`, and `v`
    /// into `+0xc4`. `eax` is `v` then; for another screen the original
    /// leaves the caller's `eax` (0 here; no caller reads it).
    pub fn set_screen_vector(mem: &mut impl Mem, screen: u32, xyzw: [i32; 4], v: u32) -> u32 {
        if screen > 2 {
            return 0;
        }
        let at = mem.rebase(SCREENS) + SCREEN_SIZE * screen;
        let vector = xyzw.map(|c| x87::store(f64::from(c)));
        mem.write(at + SCREEN_VECTOR, vector);
        let vector: [u32; 4] = mem.read(at + SCREEN_VECTOR);
        let ee = mem.rebase(EE_GPR_MOVE);
        mem.write(ee, vector);
        mem.write(at + SCREEN_VECTOR_COPY, vector);
        mem.write(at + SCREEN_VECTOR_WORD, v);
        v
    }

    fn bits(x: f32) -> u32 {
        x87::round_trip(x).to_bits()
    }

    /// `Sg_CalcScreenMatrix()` (`0x0083f160`): `MsSetScreen(0, depth, ...)`
    /// with the battle camera's screen depth, the screen centre and fixed
    /// planes.
    pub fn calc_screen_matrix(mem: &mut impl Mem) {
        let [a, b, c, d] = SCREEN_ARGS.map(|g| bits(g.get(mem)));
        let [cx, cy] = SCREEN_CENTER.get(mem).map(bits);
        let one = 1.0f32.to_bits();
        let depth = bits(battle::ms_camera_get_scr_dpt(mem));
        battle::ms_set_screen(mem, 0, [depth, a, b, cx, cy, one, c, one, d]);
    }

    /// `Sg_CalcViewClipMatrix(m, wide)` (`0x0083f1c0`): a unit matrix with
    /// `m[0][0]` and `m[1][1]` the clip scale over twice the extent seen at
    /// the screen depth, `m[2][2]`, `m[2][3]`, `m[3][2]`, `m[3][3]` fixed;
    /// returns `FFXVu0UnitMatrix`'s result.
    pub fn calc_view_clip_matrix(mem: &mut impl Mem, m: u32, wide: u32) -> u32 {
        let depth = x87::round_trip(battle::ms_camera_get_scr_dpt(mem));
        let extent = CLIP_EXTENT[usize::from(wide != 0)].get(mem);
        let r = ffx_ps2::vu0::bridge::matrix_identity(mem, m);
        let ax = (Reg::load(extent) / Reg::load(depth)).store();
        let ay = Reg::load(depth).divr_m32(extent).store();
        let k = Reg::constant(CLIP_SCALE.get(mem));
        let twice = |a: f32| {
            let a = Reg::load(a);
            a - (-a)
        };
        mem.write(m, (k / twice(ax)).store());
        mem.write(m.wrapping_add(0x14), (k / twice(ay)).store());
        let [z0, z1] = CLIP_Z.map(|g| x87::round_trip(g.get(mem)));
        mem.write(m.wrapping_add(0x28), z0);
        mem.write(m.wrapping_add(0x38), z1);
        mem.write(m.wrapping_add(0x2c), 1.0f32);
        mem.write(m.wrapping_add(0x3c), 0.0f32);
        r
    }

    /// `Sg_CameraGetPos(&v)` (`0x0083f260`): when `v` is not null, the
    /// battle camera's current position (`MsCameraGetPosNow(1, ...)`) into
    /// [`CAMERA_POS`] and `v`; [`CAMERA_POS`]'s address.
    pub fn camera_get_pos(mem: &mut impl Mem, v: u32) -> u32 {
        let at = CAMERA_POS.ptr(mem).addr;
        if v != 0 {
            battle::ms_camera_get_pos_now(mem, 1, at);
            copy3(mem, at, v);
        }
        at
    }

    /// `Sg_CameraGetYaw()` (`0x0083f2d0`, a `jmp` to the battle module's
    /// `MsCameraGetYaw`, `0x007bcf20`): the active camera's (`g_Camera`)
    /// yaw, `MsVectorRect2Polar(cam + 0x50, cam + 0x66c, &yaw, 0, 0)` from
    /// the eye to the look-at point, as `ST0` holds it (`fld dword`). The
    /// yaw goes through a stack local ([`crate::scratch`]); `trig` is the
    /// CRT's `_CIatan`.
    pub fn camera_get_yaw(
        mem: &mut impl Mem,
        trig: &dyn Fn(ffx_battle::pos::bridge::Trig, u32) -> u32,
    ) -> f32 {
        use crate::scratch::{SCRATCH, Scratch};
        let cam = battle::CAMERA.get(mem);
        let mut s = Scratch::new(mem);
        ffx_battle::vector::bridge::ms_vector_rect2_polar(
            &mut s,
            trig,
            cam.wrapping_add(0x50),
            cam.wrapping_add(0x66c),
            SCRATCH,
            0,
            0,
        );
        s.read(SCRATCH)
    }

    /// `Sg_CameraGetRef(&v)` (`0x0083f2a0`); [`CAMERA_REF`]'s address.
    pub fn camera_get_ref(mem: &mut impl Mem, v: u32) -> u32 {
        let at = CAMERA_REF.ptr(mem).addr;
        if v != 0 {
            copy3(mem, at, v);
        }
        at
    }

    fn copy3(mem: &mut impl Mem, from: u32, to: u32) {
        for k in 0..3 {
            let x: f32 = mem.read(from.wrapping_add(4 * k));
            mem.write(to.wrapping_add(4 * k), x87::round_trip(x));
        }
    }

    /// `Sg_GetLookVector(&v)` (`0x0083f710`): reference minus position into
    /// [`LOOK`], copied (four words) to `v` when not null; [`LOOK`]'s
    /// address.
    pub fn get_look_vector(mem: &mut impl Mem, v: u32) -> u32 {
        let r = CAMERA_REF.get(mem);
        let p = CAMERA_POS.get(mem);
        let look = LOOK.ptr(mem).addr;
        for (k, (r, p)) in (0u32..).zip(r.into_iter().zip(p)) {
            let d = Reg::load(r).sub_m32(p).store();
            mem.write(look.wrapping_add(4 * k), d);
        }
        if v != 0 {
            let w = LOOK.get(mem);
            mem.write(v, w);
        }
        look
    }

    /// `Sg_SetScreenCenter(x, y)` (`0x0083f7a0`): `2048 + (x - 256)` and
    /// `2048 + (y - 208)` (each difference rounded to `f32` first); `EAX`
    /// ends as `y - 208`.
    pub fn set_screen_center(mem: &mut impl Mem, x: i32, y: i32) -> i32 {
        let base = Reg::constant(CENTER_BASE.get(mem));
        let dx = f32::from_bits(x87::store(f64::from(x.wrapping_sub(0x100))));
        let dy = y.wrapping_sub(0xd0);
        let fy = f32::from_bits(x87::store(f64::from(dy)));
        let cx = (Reg::load(dx) + base).store();
        let cy = base.add_m32(fy).store();
        SCREEN_CENTER.set(mem, [cx, cy]);
        dy
    }

    /// `Cam_GetPos(cam, side, &x, &y, &z)` (`0x007bd420`): battle camera
    /// `cam` (a byte), `side` (bit 0); null outputs are skipped. `EAX` ends
    /// as the side's offset.
    pub fn cam_get_pos(mem: &mut impl Mem, cam: u32, side: u32, out: [u32; 3]) -> u32 {
        let off = (side & 1) * 0x61c;
        let base = CAMERA_POS_NOW
            .ptr(mem)
            .addr
            .wrapping_add((cam & 0xff) * 0xda0)
            .wrapping_add(off);
        for (k, o) in (0..).zip(out) {
            if o != 0 {
                let x: f32 = mem.read(base.wrapping_add(4 * k));
                mem.write(o, x87::round_trip(x));
            }
        }
        off
    }

    /// `Cam_SetChannelSuspended(id, on)` (`0x007c0010`): every one of the
    /// four channels of camera `id & 0xff` whose id is `id` gets bit 0 of
    /// `on` as its suspended byte.
    pub fn cam_set_channel_suspended(mem: &mut impl Mem, id: u32, on: u32) {
        let base = CHANNELS.ptr(mem).addr.wrapping_add((id & 0xff) * 0xda0);
        for k in 0..4 {
            let e = base.wrapping_add(8 * k);
            if mem.read::<u32>(e) == id {
                mem.write(e.wrapping_add(4), crate::lo8(on) & 1);
            }
        }
    }

    /// `Cam_IsMoving(cam, side)` (`0x007bdd20`): what of battle camera
    /// `cam`'s side is moving: bit 0 a channel with a running key (the
    /// side's three at `+0x58c` from the side, and for side 1 the camera's
    /// two at `+0xd84`), bit 1 the side's byte at `+2`, bit 2 the current
    /// channel (`MsCameraGetNum`) in state `0xe`.
    pub fn cam_is_moving(mem: &impl Mem, cam: u32, side: u32) -> u32 {
        let cam = cam & 0xff;
        let side = side & 1;
        let base = battle::CAMERAS.ptr(mem).addr.wrapping_add(cam * 0xda0);
        let side_base = base.wrapping_add(side * 0x61c).wrapping_add(0x30);
        let n = battle::ms_camera_get_num(mem, cam);
        let ch = side_base
            .wrapping_add(0xc0)
            .wrapping_add(n.wrapping_mul(0x130));
        let busy =
            |at: u32, count: u32| (0..count).any(|k| mem.read::<u16>(at.wrapping_add(16 * k)) != 0);
        let mut r = 0;
        if side == 1 && busy(base.wrapping_add(0xd84), 2) {
            r = 1;
        }
        if busy(side_base.wrapping_add(0x58c), 3) {
            r = 1;
        }
        if mem.read::<u8>(side_base.wrapping_add(2)) != 0 {
            r |= 2;
        }
        if mem.read::<u8>(ch.wrapping_add(2)) == 0xe {
            r |= 4;
        }
        r
    }

    /// `MsCameraGet(owner, camera, type)`.
    pub const MS_CAMERA_GET: u32 = 0x007b_d110;
    /// `MsCameraSetRect(handle, side, &xyz)`.
    pub const MS_CAMERA_SET_RECT: u32 = 0x007b_f940;

    /// `Sg_CameraPutRects()` (`0x0083f130`, name ours; from `Sg_MainLoop`
    /// and `Sg_CameraSetPlayer`): the battle camera `MsCameraGet(6, 0, 0)`
    /// gets side 0 from [`CAMERA_REF`] and side 1 from [`CAMERA_POS`]
    /// (`MsCameraSetRect`); returns the last call's `eax`.
    pub fn sg_camera_put_rects(h: &mut impl Host) -> u32 {
        let f = h.rebase(MS_CAMERA_GET);
        let cam = h.call(f, &[6, 0, 0]);
        let f = h.rebase(MS_CAMERA_SET_RECT);
        let at = h.rebase(0x00c4_ded0);
        h.call(f, &[cam, 0, at]);
        let at = h.rebase(0x00c4_dec0);
        h.call(f, &[cam, 1, at])
    }

    /// `MsCameraIDcheck(id)`.
    pub const ID_CHECK: u32 = 0x007b_d560;
    /// `MsCameraGetNum(cam)`.
    pub const GET_NUM: u32 = 0x007b_d340;
    /// `MsSetCheckChange(id, side, step, mode)`.
    pub const SET_CHECK_CHANGE: u32 = 0x007c_0790;
    /// `MsVectorRect2Polar(v, centre, yaw, pitch, len)`.
    pub const RECT_TO_POLAR: u32 = 0x007c_4990;
    /// `MsCameraFocusWeights_007c0ae0(side, request)`.
    pub const FOCUS_WEIGHTS: u32 = 0x007c_0ae0;
    /// The cameras' sides (`g_Cameras + 0x30`): 0xda0 bytes a camera,
    /// 0x61c a side; requests of 0x130 bytes from `+0xc0` of a side.
    pub const CAMERA_SIDES: u32 = 0x0113_7570;
    /// `g_CameraSlots`: 8-byte request slots, 0x1b4 a camera.
    pub const CAMERA_SLOTS: u32 = 0x0113_81a8;

    /// A request id's camera (`id & 0xff`), its side record and the
    /// request's 0x130-byte block (`(id >> 8) & 0xff` from the side).
    fn request(h: &impl Host, id: u32, side: u32) -> (u32, u32, u32) {
        let cam = id & 0xff;
        let base = h
            .rebase(CAMERA_SIDES)
            .wrapping_add(side.wrapping_mul(0x61c))
            .wrapping_add(cam.wrapping_mul(0xda0));
        let rec = base.wrapping_add(((id >> 8) & 0xff).wrapping_mul(0x130));
        (cam, base, rec)
    }

    /// The camera's running request: the id in its slot `MsCameraGetNum`
    /// unless the slot's off or done byte (`+4`, `+5`) is set; else 0.
    fn running(h: &mut impl Host, cam: u32) -> u32 {
        let f = h.rebase(GET_NUM);
        let n = h.call(f, &[cam]);
        let slot = h
            .rebase(CAMERA_SLOTS)
            .wrapping_add(cam.wrapping_mul(0x1b4).wrapping_add(n).wrapping_mul(8));
        if h.read::<u8>(slot.wrapping_add(4)) == 0 && h.read::<u8>(slot.wrapping_add(5)) == 0 {
            h.read(slot)
        } else {
            0
        }
    }

    /// `fld m32`, `fstp m32`.
    fn copy(v: u32) -> u32 {
        x87::store(x87::load(v))
    }

    /// Applies polar `(a, b, c)` to the side now (`+0x60..+0x6c`, the
    /// last 0) and `MsSetCheckChange(id, side, 1, 0)`.
    fn apply_polar(h: &mut impl Host, id: u32, side: u32, base: u32, [a, b, c]: [u32; 3]) -> u32 {
        h.write(base.wrapping_add(0x6c), 0u32);
        h.write(base.wrapping_add(0x60), copy(a));
        h.write(base.wrapping_add(0x64), copy(b));
        h.write(base.wrapping_add(0x68), copy(c));
        let f = h.rebase(SET_CHECK_CHANGE);
        h.call(f, &[id, side, 1, 0])
    }

    /// `Cam_SetPolar(id, side, theta, phi, len)` (`0x007bf700`): request
    /// `id` of camera side `side & 1` becomes mode 2 (`+0xc0` 1, `+0xc2`
    /// 2, `+0xc7` 0) with the polar position at `+0x100..+0x108`; when it
    /// is the camera's running request, also the side's now and
    /// `MsSetCheckChange`'s `eax` returned, else the running id. 0 for an
    /// id `MsCameraIDcheck` rejects.
    pub fn cam_set_polar(h: &mut impl Host, id: u32, side: u32, polar: [u32; 3]) -> u32 {
        let f = h.rebase(ID_CHECK);
        if h.call(f, &[id]) == 0 {
            return 0;
        }
        let side = side & 1;
        let (cam, base, rec) = request(h, id, side);
        h.write(rec.wrapping_add(0xc7), 0u8);
        h.write(rec.wrapping_add(0xc0), 1u8);
        h.write(rec.wrapping_add(0x100), copy(polar[0]));
        h.write(rec.wrapping_add(0xc2), 2u8);
        h.write(rec.wrapping_add(0x104), copy(polar[1]));
        h.write(rec.wrapping_add(0x108), copy(polar[2]));
        let now = running(h, cam);
        if now != id {
            return now;
        }
        apply_polar(h, id, side, base, polar)
    }

    /// `Cam_AddPolar(id, side, dtheta, dphi, dlen)` (`0x007bf7f0`): as
    /// [`cam_set_polar`] in mode 7 with the polar position moved by the
    /// deltas; a request not yet set (`+0xc0` 0) starts from the side's
    /// position (`+0x50`) about its centre (`[+0x8c]`) through
    /// `MsVectorRect2Polar`. Returns the id itself when it is not the
    /// running request.
    pub fn cam_add_polar(h: &mut impl Host, id: u32, side: u32, delta: [u32; 3]) -> u32 {
        let f = h.rebase(ID_CHECK);
        if h.call(f, &[id]) == 0 {
            return 0;
        }
        let side = side & 1;
        let (cam, base, rec) = request(h, id, side);
        let set = h.read::<u8>(rec.wrapping_add(0xc0));
        h.write(rec.wrapping_add(0xc7), 0u8);
        let polar = [0x100u32, 0x104, 0x108].map(|o| rec.wrapping_add(o));
        if set == 0 {
            let f = h.rebase(RECT_TO_POLAR);
            let centre: u32 = h.read(base.wrapping_add(0x8c));
            h.call(
                f,
                &[
                    base.wrapping_add(0x50),
                    centre,
                    polar[0],
                    polar[1],
                    polar[2],
                ],
            );
        }
        for (at, d) in polar.into_iter().zip(delta) {
            let v = x87::add(x87::load(d), x87::load(h.read(at)));
            h.write(at, x87::store(v));
            if at == polar[0] {
                h.write(rec.wrapping_add(0xc0), 1u8);
                h.write(rec.wrapping_add(0xc2), 7u8);
            }
        }
        let now = running(h, cam);
        if now != id {
            return id;
        }
        let now = polar.map(|at| h.read(at));
        apply_polar(h, id, side, base, now)
    }

    /// `MsCameraSetTargetPolar_007bf600(id, side, flags, target, d, a,
    /// b)` (name ours; from `Native_Cam_601d`/`601f`): request `id`
    /// becomes mode 9 (`+0xc0` 1, `+0xc2` 9, `+0xc7` 0) following `target`
    /// (`+0xc8`, its byte `+4` OR the low byte of `flags` at `+0xc3`), `a`,
    /// `b` at `+0xf0`, `+0xf8` and `d` at `+0x110`. When it is the running
    /// request: `MsVectorRect2Polar(+0xf0, [side +0x8c], +0x100, 0, 0)`,
    /// `+0x100 += d`, `MsCameraFocusWeights(side, request +0xc0)` and
    /// `MsSetCheckChange(id, side, 1, 0)`'s `eax`; else the running id; 0
    /// for an id `MsCameraIDcheck` rejects.
    #[allow(clippy::many_single_char_names)] // h, d, a, b, f: as documented
    pub fn cam_set_target_polar(
        h: &mut impl Host,
        id: u32,
        side: u32,
        flags: u32,
        target: u32,
        [d, a, b]: [u32; 3],
    ) -> u32 {
        let f = h.rebase(ID_CHECK);
        if h.call(f, &[id]) == 0 {
            return 0;
        }
        let side = side & 1;
        let (cam, base, rec) = request(h, id, side);
        let rec = rec.wrapping_add(0xc0);
        h.write(rec.wrapping_add(7), 0u8);
        h.write(rec, 1u8);
        h.write(rec.wrapping_add(2), 9u8);
        let mode = h.read::<u8>(target.wrapping_add(4)) | crate::lo8(flags);
        h.write(rec.wrapping_add(8), target);
        h.write(rec.wrapping_add(3), mode);
        h.write(rec.wrapping_add(0x30), copy(a));
        h.write(rec.wrapping_add(0x38), copy(b));
        h.write(rec.wrapping_add(0x50), copy(d));
        let now = running(h, cam);
        if now != id {
            return now;
        }
        let f = h.rebase(RECT_TO_POLAR);
        let centre: u32 = h.read(base.wrapping_add(0x8c));
        h.call(
            f,
            &[rec.wrapping_add(0x30), centre, rec.wrapping_add(0x40), 0, 0],
        );
        let yaw = rec.wrapping_add(0x40);
        let v = x87::add(
            x87::load(h.read(rec.wrapping_add(0x50))),
            x87::load(h.read(yaw)),
        );
        h.write(yaw, x87::store(v));
        let f = h.rebase(FOCUS_WEIGHTS);
        h.call(f, &[base, rec]);
        let f = h.rebase(SET_CHECK_CHANGE);
        h.call(f, &[id, side, 1, 0])
    }

    /// The screens (`0x01137010`, 0x1c0 bytes each): `+0` and `+4` the
    /// depths `Cam_Apply` turns into the field of view, `+0xc` its
    /// divisor, `+0x78` 256 for a split screen.
    pub const APPLY_SCREENS: u32 = 0x0113_7010;
    /// 2 when the field of view uses 3/4 of the angle (`0x01136fc0`).
    pub const APPLY_NARROW: Global<u32> = Global::new(0x0113_6fc0);
    /// 1 when camera 1 draws through the sub matrix (`0x01137280`).
    pub const APPLY_SUB: Global<u8> = Global::new(0x0113_7280);
    /// `g_CameraMatrix` (`0x02311440`).
    pub const CAMERA_MATRIX: u32 = 0x0231_1440;
    /// The callees: `graphicCameraSetFOVy(&f)`, `graphicCameraSetAspectRatio
    /// (&f)`, `MagicFile_GetMagicId()`, `graphicCameraSetWorldMatrixSub(m,
    /// cam)`, `SplitScreenStart(0)`, `SplitScreenSetCamera(m)`,
    /// `0x00638d20()`, `graphicCameraSetWorldMatrix(m)`.
    pub const SET_FOVY: u32 = 0x0063_95e0;
    pub const SET_ASPECT: u32 = 0x0063_95c0;
    pub const MAGIC_ID: u32 = 0x009d_a3b0;
    pub const SET_WORLD_SUB: u32 = 0x0063_9b30;
    pub const SPLIT_START: u32 = 0x0063_8d40;
    pub const SPLIT_SET_CAMERA: u32 = 0x0063_8aa0;
    pub const SPLIT_END: u32 = 0x0063_8d20;
    pub const SET_WORLD: u32 = 0x0063_9860;
    /// [`cam_apply`]'s work: the field of view and the aspect ratio
    /// (`f32` each) the renderer reads through pointers.
    pub const APPLY_WORK: u32 = 8;
    /// `.rdata` `f64`: 2, 512, 448, 0.75, 256.
    const K2: u32 = 0x00b9_2270;
    const K512: u32 = 0x00b4_1d38;
    const K448: u32 = 0x00b5_0210;
    const K075: u32 = 0x00b5_1bd0;
    const K256: u32 = 0x00b9_22f0;

    /// `Cam_Apply(camera)` (`0x007bc980`; from `MsScreenProcess` and
    /// `0x0076edc0`): the renderer's camera from screen `camera`
    /// ([`APPLY_SCREENS`]): with `h = f32(512 / (2 a))` and `w = f32(448 /
    /// (2 b))` (`a`, `b` its `+0`, `+4`), the field of view `f32(2 f32(atan(w
    /// / c)))` (`c` its `+0xc`; `atan(0.75 w / c)` when [`APPLY_NARROW`] is
    /// 2) and the aspect ratio `f32(h / w)` go to `graphicCameraSetFOVy`
    /// and `graphicCameraSetAspectRatio` through `work` ([`APPLY_WORK`]).
    /// Then the view `g_CameraMatrix`: for magic 0x98 or 0x259 on camera 1
    /// with [`APPLY_SUB`] set through `graphicCameraSetWorldMatrixSub(m,
    /// 1)`; else on a split screen (`+0x78` exactly 256) `SplitScreenStart
    /// (0)` and, for camera 1, `SplitScreenSetCamera(m)`; else (and for the
    /// other split cameras) `graphicCameraSetWorldMatrix(m)`, after
    /// `0x00638d20()` when not split. `atan` stands for `_CIatan` (its
    /// result stored as `f32` bits).
    pub fn cam_apply<M, C>(
        mem: &mut M,
        cam: u32,
        work: u32,
        call: &mut C,
        mut atan: impl FnMut(&mut M, f64) -> u32,
    ) where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        let k = |mem: &M, a: u32| mem.read::<f64>(mem.rebase(a));
        let screen = mem
            .rebase(APPLY_SCREENS)
            .wrapping_add(cam.wrapping_mul(0x1c0));
        let field = |mem: &M, o: u32| x87::load(mem.read(screen.wrapping_add(o)));
        let two = k(mem, K2);
        let height = x87::store(x87::div(k(mem, K512), x87::mul(field(mem, 0), two)));
        let width = x87::store(x87::div(k(mem, K448), x87::mul(two, field(mem, 4))));
        let depth = x87::load(x87::store(field(mem, 0xc)));
        let q = x87::div(x87::load(width), depth);
        let mut fov = x87::store(x87::mul(x87::load(atan(mem, q)), two));
        if APPLY_NARROW.get(mem) == 2 {
            let narrow = atan(mem, x87::mul(q, k(mem, K075)));
            fov = x87::store(x87::mul(x87::load(narrow), two));
        }
        let aspect = x87::store(x87::div(x87::load(height), x87::load(width)));
        mem.write(work, fov);
        mem.write(work + 4, aspect);
        let f = mem.rebase(SET_FOVY);
        call(mem, f, &[work]);
        let f = mem.rebase(SET_ASPECT);
        call(mem, f, &[work + 4]);
        let f = mem.rebase(MAGIC_ID);
        let special = call(mem, f, &[]) == 0x98 || call(mem, f, &[]) == 0x259;
        let m = mem.rebase(CAMERA_MATRIX);
        if special && cam == 1 && APPLY_SUB.get(mem) == 1 {
            let f = mem.rebase(SET_WORLD_SUB);
            call(mem, f, &[m, cam]);
            return;
        }
        // fucompp: equal and ordered
        #[allow(clippy::float_cmp)] // the exact compare the original makes
        let split = field(mem, 0x78) == k(mem, K256);
        if split {
            let f = mem.rebase(SPLIT_START);
            call(mem, f, &[0]);
            if cam == 1 {
                let f = mem.rebase(SPLIT_SET_CAMERA);
                call(mem, f, &[m]);
                return;
            }
        } else {
            let f = mem.rebase(SPLIT_END);
            call(mem, f, &[]);
        }
        let f = mem.rebase(SET_WORLD);
        call(mem, f, &[m]);
    }
}
