//! `ffx-movie`: the PS2 sample library's `FFXSamp0UnitMatrix` and
//! `FFXSamp0ViewScreenMatrix` (the movie player itself stays original:
//! glue).

use ffx_movie::{ViewScreen, bridge as mv};

use super::{mem, ported};

ported! {
    /// `FFXSamp0UnitMatrix(m)`: returns the 4 its loop leaves in `eax`.
    FFXSamp0UnitMatrix @ 0x0093_d900 => extern "C" fn ffx_samp0_unit_matrix(m: u32) -> u32 {
        mv::samp0_unit_matrix(&mut mem(), m)
    }
    /// `FFXSamp0ViewScreenMatrix(m, scrz, ax, ay, cx, cy, zmin, zmax,
    /// nearz, farz)`: the screen matrix in this call's own frame, as the
    /// original keeps it.
    FFXSamp0ViewScreenMatrix @ 0x0093_d960 => extern "C" fn ffx_samp0_view_screen_matrix(
        m: u32, scrz: u32, ax: u32, ay: u32, cx: u32, cy: u32, zmin: u32, zmax: u32, nearz: u32, farz: u32
    ) {
        let f = f32::from_bits;
        let v = ViewScreen {
            scrz: f(scrz),
            ax: f(ax),
            ay: f(ay),
            cx: f(cx),
            cy: f(cy),
            zmin: f(zmin),
            zmax: f(zmax),
            nearz: f(nearz),
            farz: f(farz),
        };
        let mut tmp = [0u32; 16];
        #[allow(clippy::cast_possible_truncation)] // 32-bit process
        let at = tmp.as_mut_ptr() as usize as u32;
        mv::samp0_view_screen_matrix(&mut mem(), m, at, &v);
    }
    /// `MovieCameraFrame_0076edc0()`: the camera record goes into this
    /// call's own frame, as the original keeps it; without records the
    /// port uses 0 for the original's never-written frame (brief
    /// 2026-10-24). Nobody reads its `eax` (`Sg_MainLoop`,
    /// `Native_b009_MovieCameraWait_Result`).
    MovieCameraFrame_0076edc0 @ 0x0076_edc0 => extern "C" fn movie_camera_frame() -> u32 {
        let mut work = super::StackFrame::<{ (mv::CAMERA_WORK / 4) as usize }>::new();
        let at = work.at();
        mv::movie_camera_frame(&mut mem(), at)
    }
}
