//! Movies (`movie`, `PhyFMV`).
//!
//! The `movie` module of `docs/rewrite-plan.md`: each original function
//! becomes a core (portable, on its own types) and a [`bridge`] function
//! over game memory (`docs/porting.md`).
//!
//! The movie player itself (`PhyFMVPlayerManager`, the `WebM` demuxer and
//! the Vorbis decoder feeding FMOD) is glue and library code and stays
//! original; what the module holds besides is a part of the PS2 sample
//! library the classification files here (`FFXSamp0*`).

/// The 4x4 identity, row-major (`FFXSamp0UnitMatrix`).
#[must_use]
pub fn unit_matrix() -> [f32; 16] {
    std::array::from_fn(|k| if k % 5 == 0 { 1.0 } else { 0.0 })
}

/// `FFXSamp0ViewScreenMatrix`'s arguments (libvu0's
/// `sceVu0ViewScreenMatrix`): the screen distance, the aspect scales, the
/// screen centre, the Z buffer range and the near and far clip planes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewScreen {
    pub scrz: f32,
    pub ax: f32,
    pub ay: f32,
    pub cx: f32,
    pub cy: f32,
    pub zmin: f32,
    pub zmax: f32,
    pub nearz: f32,
    pub farz: f32,
}

/// The two matrices `FFXSamp0ViewScreenMatrix` multiplies, as
/// `rcuiMulMatrix(m, screen, m)` takes them: the projection and the screen
/// transform (`docs/formats/movie.md`). Both start as the identity. The
/// projection: `[0]` and `[5]` = `scrz`, `[10]` and `[15]` = 0, `[11]` and
/// `[14]` = 1. The screen: `[0]` = `ax`, `[5]` = `ay`, `[12]` = `cx`,
/// `[13]` = `cy`, `[10]` = `(zmax - zmin) * (nearz * farz) / depth` and
/// `[14]` = `(farz * zmin - nearz * zmax) / depth`, with `depth` =
/// `farz - nearz`; on the x87 at 53 bits, each stored as f32.
#[must_use]
pub fn view_screen(v: &ViewScreen) -> ([f32; 16], [f32; 16]) {
    use ffx_bridge::x87::{Reg, round_trip};
    let mut proj = unit_matrix();
    proj[0] = round_trip(v.scrz);
    proj[5] = round_trip(v.scrz);
    proj[10] = 0.0;
    proj[15] = 0.0;
    proj[14] = 1.0;
    proj[11] = 1.0;
    let (zmin, zmax) = (Reg::load(v.zmin), Reg::load(v.zmax));
    let (nearz, farz) = (Reg::load(v.nearz), Reg::load(v.farz));
    let depth = farz - nearz;
    let mut screen = unit_matrix();
    screen[0] = round_trip(v.ax);
    screen[5] = round_trip(v.ay);
    screen[10] = ((zmax - zmin) * (nearz * farz) / depth).store();
    screen[12] = round_trip(v.cx);
    screen[13] = round_trip(v.cy);
    screen[14] = ((farz * zmin - nearz * zmax) / depth).store();
    (proj, screen)
}

pub mod api;
pub mod mkv;
pub mod vorbis;
pub mod vp8;
pub mod webm;

/// One Vorbis sample as the movie's FMOD stream gets it
/// (`PVideoPlaybackWin32_AudioRead` `0x00a27170`, on the x87 at 53 bits):
/// `fld f; fmul 32767.0; fadd 0.5; fstp dword`, `floor` of that (stored
/// as f32 again), `ftol2_sse` (truncation; `cvttsd2si`, as the game always
/// has SSE2) and a clamp to `-0x8000..=0x7fff`.
#[must_use]
pub fn vorbis_to_s16(f: f32) -> i16 {
    #[allow(clippy::cast_possible_truncation)] // the fstp dword
    let biased = (f64::from(f) * 32767.0 + 0.5) as f32;
    #[allow(clippy::cast_possible_truncation)] // the second fstp dword
    let floored = f64::from(biased).floor() as f32;
    let v = ffx_bridge::x87::cvttsd2si(f64::from(floored)).cast_signed();
    #[allow(clippy::cast_possible_truncation)] // clamped
    let s = v.clamp(-0x8000, 0x7fff) as i16;
    s
}

/// The originals, where the game keeps their state (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::{Host, Mem, x87};

    use super::ViewScreen;

    /// `rcuiMulMatrix(out, a, b)` (`0x0093d1b0`), left to the caller's
    /// host (ported in `ffx-map`).
    pub const RCUI_MUL_MATRIX: u32 = 0x0093_d1b0;

    /// `FFXSamp0ViewScreenMatrix(m, scrz, ax, ay, cx, cy, zmin, zmax,
    /// nearz, farz)` (`0x0093d960`): the projection into `m`, the screen
    /// transform into `tmp` (64 bytes: the original's stack frame), then
    /// `rcuiMulMatrix(m, tmp, m)`. Its `eax` is `rcuiMulMatrix`'s, which
    /// nothing reads.
    pub fn samp0_view_screen_matrix(h: &mut impl Host, m: u32, tmp: u32, v: &ViewScreen) {
        let (proj, screen) = super::view_screen(v);
        h.write(m, proj);
        h.write(tmp, screen);
        let f = h.rebase(RCUI_MUL_MATRIX);
        h.call(f, &[m, tmp, m]);
    }

    /// `g_WaitFramesByTime` (`0x0112a008`): movie frames count by time.
    pub const WAIT_FRAMES_BY_TIME: u32 = 0x0112_a008;
    /// The movie playing (`0x00c40e28`): 0x2e and 0x46 have frame rules.
    pub const MOVIE_ID: u32 = 0x00c4_0e28;
    /// `g_CameraDataWords` (`0x00c38e18`): the camera records' size in
    /// words, 42 per frame.
    pub const CAMERA_DATA_WORDS: u32 = 0x00c3_8e18;
    /// `g_CameraData` (`0x00ce82ac`): the camera records, 0xa8 bytes per
    /// frame, null when none is loaded.
    pub const CAMERA_DATA: u32 = 0x00ce_82ac;
    /// `movie_have_camera()`, `graphicGetCurrentFrame()`,
    /// `setScreenDataAspectZ(f)`, `MsSetCameraMatrix(on, screen, camera,
    /// look_at, eye, roll)`, `rcbgSceneSetMatrix_Cross()`,
    /// `Cam_Apply(camera)`, `SetByte_012fb8fb(v)`.
    pub const HAVE_CAMERA: u32 = 0x0076_efc0;
    pub const CURRENT_FRAME: u32 = 0x0064_10d0;
    pub const SET_ASPECT_Z: u32 = 0x007c_0e60;
    pub const SET_CAMERA_MATRIX: u32 = 0x007c_06d0;
    pub const SCENE_SET_MATRIX_CROSS: u32 = 0x0092_0e40;
    pub const CAM_APPLY: u32 = 0x007b_c980;
    pub const SET_BYTE_012FB8FB: u32 = 0x0082_1a20;

    /// The frame of [`movie_camera_frame`], laid out as the original's
    /// (`ebp - 0xac` on): `+0` the aspect (`f32`), `+4` the "on" word,
    /// `+8` the camera matrix, `+0x48` the screen matrix, `+0x88` the
    /// look-at and `+0x98` the eye (four words each).
    pub const CAMERA_WORK: u32 = 0xa8;

    /// `fld dword` then `fstp dword` (a signalling NaN comes out quiet).
    fn through_x87(bits: u32) -> u32 {
        x87::store(x87::load(bits))
    }

    fn call_at(h: &mut impl Host, f: u32, args: &[u32]) -> u32 {
        let f = h.rebase(f);
        h.call(f, args)
    }

    /// The camera record for movie frame `frame`, or `None` for a frame
    /// out of range: movie 0x2e plays one frame ahead (except at 0x47b)
    /// and stops at 0x4d8, movie 0x46 stops at 0x3ca; otherwise a negative
    /// frame is out. The bound is `g_CameraDataWords / 42` (signed).
    fn camera_frame(mem: &impl Mem, frame: i32) -> Option<i32> {
        let id: u32 = mem.read(mem.rebase(MOVIE_ID));
        let frame = match id {
            0x2e => {
                let f = if frame == 0x47b {
                    frame
                } else {
                    frame.wrapping_add(1)
                };
                if f >= 0x4d8 { 0x4d8 } else { f }
            }
            0x46 if frame >= 0x3ca => 0x3ca,
            _ => frame,
        };
        if frame < 0 {
            return None;
        }
        let words: i32 = mem.read(mem.rebase(CAMERA_DATA_WORDS));
        (frame < words / 42).then_some(frame)
    }

    /// `MovieCameraFrame_0076edc0()` (from `Sg_MainLoop` and
    /// `Native_b009_MovieCameraWait_Result`): while `g_WaitFramesByTime` is set and
    /// `movie_have_camera()`, the movie camera for the renderer's current
    /// frame ([`camera_frame`]; out of range: `SetByte_012fb8fb(0)`). From
    /// the record (when [`CAMERA_DATA`] is loaded; the original reads
    /// [`CAMERA_DATA`] again after the `ftol`): "on" the `ftol` of its
    /// first float, the screen matrix from `+4`, the camera matrix from
    /// `+0x44`, the look-at and eye from `+0x84`, the aspect `+0xa4`
    /// (through the x87), into `work` ([`CAMERA_WORK`]); without records
    /// "on" is 0 and the rest is the original's never-written frame, which
    /// the port sets to 0 (brief 2026-10-24). Then `setScreenDataAspectZ(aspect)`,
    /// `MsSetCameraMatrix(on, screen, camera, look_at, eye, aspect)` and,
    /// unless `rcbgSceneSetMatrix_Cross()` took the view, `Cam_Apply(0)`.
    /// Returns the last callee's `eax` on the camera path, as the original
    /// (elsewhere the original leaves what its checks left; nothing reads
    /// it).
    pub fn movie_camera_frame(h: &mut impl Host, work: u32) -> u32 {
        // Without records the original reads its frame unwritten:
        // junk read: [ebp-0xac] never written; 0 (brief 2026-10-24)
        // (the aspect)
        h.write(work, [0u32; 2]);
        // junk read: [ebp-0xa4] never written; 0 (brief 2026-10-24)
        // (the camera matrix, to MsSetCameraMatrix by address)
        h.write(work + 8, [0u32; 16]);
        // junk read: [ebp-0x64] never written; 0 (brief 2026-10-24)
        // (the screen matrix, by address)
        h.write(work + 0x48, [0u32; 16]);
        // junk read: [ebp-0x24] never written; 0 (brief 2026-10-24)
        // (the look-at and eye, by address)
        h.write(work + 0x88, [0u32; 8]);
        if h.read::<u32>(h.rebase(WAIT_FRAMES_BY_TIME)) == 0 {
            return 0;
        }
        if call_at(h, HAVE_CAMERA, &[]) == 0 {
            return 0;
        }
        let current = call_at(h, CURRENT_FRAME, &[]).cast_signed();
        let Some(frame) = camera_frame(h, current) else {
            return call_at(h, SET_BYTE_012FB8FB, &[0]);
        };
        let data: u32 = h.read(h.rebase(CAMERA_DATA));
        if data == 0 {
            // the aspect, matrices and vectors stay the 0s written above
            h.write(work + 4, 0u32);
        } else {
            let off = frame.wrapping_mul(0xa8).cast_unsigned();
            let first: u32 = h.read(off.wrapping_add(data));
            let on = x87::ftol(h, x87::load(first));
            let rec = h.read::<u32>(h.rebase(CAMERA_DATA)).wrapping_add(off);
            h.write(work + 4, on);
            let screen: [u32; 16] = h.read(rec.wrapping_add(4));
            h.write(work + 0x48, screen);
            let camera: [u32; 16] = h.read(rec.wrapping_add(0x44));
            h.write(work + 8, camera);
            let vectors: [u32; 8] = h.read(rec.wrapping_add(0x84));
            h.write(work + 0x88, vectors);
            let aspect: u32 = h.read(rec.wrapping_add(0xa4));
            h.write(work, through_x87(aspect));
        }
        let aspect = through_x87(h.read(work));
        call_at(h, SET_ASPECT_Z, &[aspect]);
        let aspect = through_x87(h.read(work));
        let on: u32 = h.read(work + 4);
        call_at(
            h,
            SET_CAMERA_MATRIX,
            &[on, work + 0x48, work + 8, work + 0x88, work + 0x98, aspect],
        );
        let taken = call_at(h, SCENE_SET_MATRIX_CROSS, &[]);
        if taken != 0 {
            return taken;
        }
        call_at(h, CAM_APPLY, &[0])
    }

    /// `FFXSamp0UnitMatrix(m)` (`0x0093d900`): the identity into the 16
    /// floats at `m` (`fld1`/`fldz` stored row by row: +0.0, never -0.0).
    /// The row counter is left in `eax`: 4.
    pub fn samp0_unit_matrix(mem: &mut impl Mem, m: u32) -> u32 {
        mem.write(m, super::unit_matrix());
        4
    }
}
