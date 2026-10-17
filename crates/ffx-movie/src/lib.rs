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

/// The originals, where the game keeps their state (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::{Host, Mem};

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

    /// `FFXSamp0UnitMatrix(m)` (`0x0093d900`): the identity into the 16
    /// floats at `m` (`fld1`/`fldz` stored row by row: +0.0, never -0.0).
    /// The row counter is left in `eax`: 4.
    pub fn samp0_unit_matrix(mem: &mut impl Mem, m: u32) -> u32 {
        mem.write(m, super::unit_matrix());
        4
    }
}
