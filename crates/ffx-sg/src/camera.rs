//! The main module's camera and screen set-up: the screen matrix (through
//! the battle camera's `MsSetScreen`), the view clip matrix, the camera
//! position, reference point and look vector, the screen centre, and two
//! accessors of the battle cameras (`Cam_*`).

/// The originals over game memory (`ffx_bridge::Mem`).
#[allow(clippy::many_single_char_names)] // matrix entries and coordinates
pub mod bridge {
    use ffx_battle::camera::bridge as battle;
    use ffx_bridge::x87::{self, Reg};
    use ffx_bridge::{Global, Mem};

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
}
