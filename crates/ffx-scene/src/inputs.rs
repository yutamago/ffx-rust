//! The scene builder's inputs: what the hook read from the game's memory
//! at each map `renderWorld` call of a capture (the `SceneInputs`
//! messages of `capture.ffxgfx`, [`ffx_gfx::scene_inputs`]), and what the
//! draws derive from them (`docs/formats/scene.md`, "Scene inputs").
//!
//! Matrices come in two forms here: as `PhyreEngine` keeps them in memory
//! ([`from_3x4`]: column vectors, column-major, translation in elements
//! 12-14) and as the shaders get them ([`Mat`], the census's form:
//! translation in elements 3, 7 and 11), one the transpose of the other.
//! The derived matrices use the census's arithmetic ([`census::mul`],
//! [`census::mul_f64`]), which reproduces the captured constants bit for
//! bit.

use ffx_gfx::Msg;
pub use ffx_gfx::scene_inputs::{
    Bounds, Cluster, DepthBias, Instance, Material, MaterialTexture, Matrix3x4, SceneInputs,
    Segment, Wire,
};

use crate::capture::Error;
use crate::census;
pub use crate::census::Mat;

/// The `SceneInputs` records of a capture file, in stream order.
///
/// # Errors
/// On a wrong magic or a malformed stream.
pub fn read(bytes: &[u8]) -> Result<Vec<SceneInputs>, Error> {
    let rest = bytes
        .strip_prefix(&ffx_gfx::CAPTURE_MAGIC[..])
        .ok_or(Error::Magic)?;
    let msgs = ffx_gfx::read_all(rest).map_err(|e| Error::Decode(e.0))?;
    Ok(msgs
        .into_iter()
        .filter_map(|m| match m {
            Msg::SceneInputs(s) => Some(*s),
            _ => None,
        })
        .collect())
}

/// The `SceneInputs` records of a capture file with their [`Wire`]
/// blocks (`None` in captures taken before the block existed, e.g.
/// `r2a-a`), in stream order.
///
/// # Errors
/// On a wrong magic or a malformed stream.
pub fn read_wired(bytes: &[u8]) -> Result<Vec<(SceneInputs, Option<Wire>)>, Error> {
    let mut rest = bytes
        .strip_prefix(&ffx_gfx::CAPTURE_MAGIC[..])
        .ok_or(Error::Magic)?;
    let mut out = Vec::new();
    while rest.len() >= 8 {
        let tag = u32::from_le_bytes([rest[0], rest[1], rest[2], rest[3]]);
        let len = u32::from_le_bytes([rest[4], rest[5], rest[6], rest[7]]) as usize;
        let payload = rest
            .get(8..8 + len)
            .ok_or(Error::Decode("truncated message"))?;
        if tag == ffx_gfx::scene_inputs::MSG_TAG {
            out.push(
                ffx_gfx::scene_inputs::read_extensions(payload).map_err(|e| Error::Decode(e.0))?,
            );
        }
        rest = &rest[8 + len..];
    }
    Ok(out)
}

/// `PMatrix4_FromMatrix3x4` (`0x00451080`): the 4x4 of a 3x4, in memory
/// order. Its first column is the 3x4's words 3, 7 and 11, the others
/// words 0-2, 4-6 and 8-10 (the translation), with 0 0 0 1 below.
#[must_use]
pub fn from_3x4(m: &Matrix3x4) -> Mat {
    [
        m[3], m[7], m[11], 0.0, //
        m[0], m[1], m[2], 0.0, //
        m[4], m[5], m[6], 0.0, //
        m[8], m[9], m[10], 1.0,
    ]
}

/// The transpose (memory order to the shaders' order and back).
#[must_use]
pub fn transpose(m: &Mat) -> Mat {
    std::array::from_fn(|i| m[(i % 4) * 4 + i / 4])
}

/// The identity, for bounds without a world matrix.
pub const IDENTITY: Mat = [
    1.0, 0.0, 0.0, 0.0, //
    0.0, 1.0, 0.0, 0.0, //
    0.0, 0.0, 1.0, 0.0, //
    0.0, 0.0, 0.0, 1.0,
];

fn f32_at(bytes: &[u8], at: usize) -> Option<f32> {
    let b = bytes.get(at..at + 4)?;
    Some(f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

fn floats<const N: usize>(bytes: &[u8], at: usize) -> Option<[f32; N]> {
    let mut out = [0.0; N];
    for (k, v) in out.iter_mut().enumerate() {
        *v = f32_at(bytes, at + 4 * k)?;
    }
    Some(out)
}

/// Offsets in [`SceneInputs::camera`], the camera `renderWorld` gets.
/// That pointer is 4 bytes past the object `PhyreScene +0xdc` points at,
/// so near, far, aspect and FOV sit 4 lower here than from there
/// (`docs/formats/render.md`: `+0xbc` far, `+0x150` aspect, `+0x154`
/// FOV).
pub mod camera {
    /// The view (3x4), the projection (4x4, depth -w..w), the culling
    /// view-projection (4x4, depth -w..w).
    pub const VIEW: usize = 0;
    pub const PROJECTION: usize = 0x30;
    pub const CULL_VIEW_PROJECTION: usize = 0x70;
    pub const NEAR: usize = 0xb4;
    pub const FAR: usize = 0xb8;
    pub const ASPECT: usize = 0x14c;
    pub const FOV: usize = 0x150;
}

/// `g_PhyreApplication` offsets (absolute; [`app_f32`] takes them).
pub mod app {
    pub const AMBIENT: usize = 0x50;
    pub const FOG_COLOUR: usize = 0x60;
    pub const FOG_NEAR: usize = 0x70;
    pub const FOG_FAR: usize = 0x74;
    pub const FOG_CURVE: usize = 0x78;
    pub const FOG_LIMIT: usize = 0x7c;
    pub const BACKGROUND: usize = 0x80;
    pub const VIEW_PROJECTION2: usize = 0x90;
    pub const GLOBAL_TIME: usize = 0x120;
}

/// A float of `g_PhyreApplication` at `offset` (absolute, e.g.
/// [`app::FOG_NEAR`]), when the record holds it.
#[must_use]
pub fn app_f32(s: &SceneInputs, offset: usize) -> Option<f32> {
    f32_at(&s.app, offset.checked_sub(ffx_gfx::scene_inputs::APP_FROM)?)
}

/// The camera's near and far planes.
#[must_use]
pub fn near_far(s: &SceneInputs) -> Option<(f32, f32)> {
    Some((
        f32_at(&s.camera, camera::NEAR)?,
        f32_at(&s.camera, camera::FAR)?,
    ))
}

/// `scene.View`: the camera's view (`PCamera +0`) through
/// [`from_3x4`], in the shaders' order.
#[must_use]
pub fn view(s: &SceneInputs) -> Option<Mat> {
    Some(transpose(&from_3x4(&floats(&s.camera, camera::VIEW)?)))
}

/// `PCamera_ConvertProjectionMatrixDepthRangeToZeroOne` (`0x00454260`) on
/// a projection in memory order: with element 15 zero (perspective)
/// `[10] = f / (n - f)` and `[14] = n f / (n - f)`, else `[10] = 1 / (n -
/// f)` and `[14] = n / (n - f)`; the x87 computes in 53 bits and stores
/// f32.
#[must_use]
#[allow(clippy::cast_possible_truncation)] // the f32 stores
pub fn depth_zero_one(p: &Mat, near: f32, far: f32) -> Mat {
    let (n, f) = (f64::from(near), f64::from(far));
    let mut out = *p;
    if p[15] == 0.0 {
        out[10] = (f / (n - f)) as f32;
        out[14] = (n * f / (n - f)) as f32;
    } else {
        out[10] = (1.0 / (n - f)) as f32;
        out[14] = (n / (n - f)) as f32;
    }
    out
}

/// `scene.Projection`: `PCamera +0x30` with depth taken to 0..1, in the
/// shaders' order.
#[must_use]
pub fn projection(s: &SceneInputs) -> Option<Mat> {
    let p: Mat = floats(&s.camera, camera::PROJECTION)?;
    let (n, f) = near_far(s)?;
    Some(transpose(&depth_zero_one(&p, n, f)))
}

/// `scene.ViewProjection` = Projection . View (f32 sums).
#[must_use]
pub fn view_projection(s: &SceneInputs) -> Option<Mat> {
    Some(census::mul(&projection(s)?, &view(s)?))
}

/// The culling view-projection (`PCamera +0x70`, depth -w..w), in memory
/// order, as `PVisibilityCheck` keeps it.
#[must_use]
pub fn cull_view_projection(s: &SceneInputs) -> Option<Mat> {
    floats(&s.camera, camera::CULL_VIEW_PROJECTION)
}

/// A world matrix in the shaders' order (identity for `None`).
#[must_use]
pub fn world(m: Option<&Matrix3x4>) -> Mat {
    m.map_or(IDENTITY, |m| transpose(&from_3x4(m)))
}

/// `WorldView` = View . World (f32 sums).
#[must_use]
pub fn world_view(view: &Mat, world: &Mat) -> Mat {
    census::mul(view, world)
}

/// `WorldViewProjection` = `ViewProjection` . World (f64 sums, one f32
/// rounding).
#[must_use]
pub fn world_view_projection(view_projection: &Mat, world: &Mat) -> Mat {
    census::mul_f64(view_projection, world)
}

/// The instance a bounds object points at.
#[must_use]
pub fn bounds_instance<'a>(s: &'a SceneInputs, b: &Bounds) -> Option<&'a Instance> {
    s.instances.get(b.instance? as usize)
}

/// The material a segment of an instance uses (`None`: the null material
/// that ends the instance's draws).
#[must_use]
pub fn segment_material<'a>(
    s: &'a SceneInputs,
    i: &Instance,
    seg: &Segment,
) -> Option<&'a Material> {
    let m = (*i.materials.get(seg.material as usize)?)?;
    s.materials.get(m as usize)
}

/// A parameter's bytes in a material's buffer: `size` bytes at `offset`,
/// when the game would write them (offset at least 4 and inside the
/// buffer's size word, `PParameterCaptureBuffer_setValue` `0x004fedc0`).
#[must_use]
pub fn parameter(m: &Material, offset: u32, size: u32) -> Option<&[u8]> {
    let total = u32::from_le_bytes(m.parameters.get(..4)?.try_into().ok()?);
    (offset >= 4 && offset + size <= total)
        .then(|| m.parameters.get(offset as usize..(offset + size) as usize))
        .flatten()
}

/// The depth bias of a bounds object (`g_DepthBias`: the first entry
/// naming it, `PCheckVisibleBoundsJob_execute` `0x009f3160`).
#[must_use]
pub fn depth_bias(s: &SceneInputs, bounds: u32) -> Option<(f32, f32)> {
    s.depth_bias
        .iter()
        .find(|d| d.bounds == Some(bounds))
        .map(|d| (d.scale, d.bias))
}

/// The bounds of instance `slot` of cluster `cluster` (index into
/// [`SceneInputs::clusters`]).
pub fn bounds_of(s: &SceneInputs, cluster: u32, slot: u32) -> impl Iterator<Item = &Bounds> {
    s.bounds.iter().filter(move |b| {
        bounds_instance(s, b).is_some_and(|i| i.cluster == cluster && i.slot == slot)
    })
}

/// The cluster named `name` (the map model's symbol, e.g. `cdsp00`), as
/// an index into [`SceneInputs::clusters`].
#[must_use]
pub fn cluster_named(s: &SceneInputs, name: &str) -> Option<u32> {
    s.clusters
        .iter()
        .position(|c| c.name == name)
        .and_then(|p| u32::try_from(p).ok())
}

/// A 4x4 constant of a draw's vertex shader by name (`World`,
/// `scene.View`, …; a struct member as `struct.member`), used or not, as
/// the draw's constant buffer holds it.
#[must_use]
pub fn draw_matrix(cap: &crate::capture::Capture, d: &ffx_gfx::Draw, name: &str) -> Option<Mat> {
    let c = ffx_dxbc::Container::parse(cap.shader(d.vs)?).ok()?;
    let r = c.reflection().ok()??;
    for b in &r.buffers {
        let slot = r
            .bindings
            .iter()
            .find(|x| x.kind == ffx_dxbc::BindKind::ConstantBuffer && x.name == b.name)
            .map_or(0, |x| u8::try_from(x.slot).unwrap_or(u8::MAX));
        for v in &b.variables {
            let offset = if v.name == name {
                Some(v.offset)
            } else {
                v.members
                    .iter()
                    .find(|m| format!("{}.{}", v.name, m.name) == name)
                    .map(|m| v.offset + m.offset)
            };
            if let Some(offset) = offset {
                let data = &d.vs_cb.iter().find(|x| x.0 == slot)?.1;
                return floats(data, offset as usize);
            }
        }
    }
    None
}

/// One line per record: pass, bounds, the instances and materials.
#[must_use]
pub fn summary(s: &SceneInputs) -> String {
    use std::fmt::Write as _;
    let mut out = format!(
        "frame {} update {} thread {} world {:#x} pass {} dir {}: {} bounds, {} instances, {} materials, {} clusters {:?}, depth bias {}, group mask {:#x}, shadow solution {}, alpha fast {}, layers {} (group {})",
        s.frame,
        s.update,
        s.thread,
        s.world,
        s.pass,
        s.dir,
        s.bounds.len(),
        s.instances.len(),
        s.materials.len(),
        s.clusters.len(),
        s.clusters
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>(),
        s.depth_bias_count,
        s.group_mask,
        s.shadow_solution,
        s.alpha_fast,
        s.layers.len(),
        s.layer_group,
    );
    for (k, b) in s.bounds.iter().enumerate() {
        let i = bounds_instance(s, b);
        let _ = write!(
            out,
            "\n  bounds {k} slot {} order {} instance {:?} group {:?} kind/flag/obj {:x?} segments {} hidden {:?} world {:?}",
            b.slot,
            b.render_order,
            i.map(|i| (i.cluster, i.slot)),
            i.map(|i| i.group_id),
            i.map(|i| (i.dobj_kind, i.dobj_flag, i.obj_id)),
            i.map_or(0, |i| i.segments.len()),
            i.map(|i| i.segments.iter().filter(|s| s.hidden != 0).count()),
            b.world,
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrices_take_the_game_layout() {
        // A 3x4 as the 3D map load writes it: 10 x unit with translation.
        let m: Matrix3x4 = [
            0.0, 0.0, 0.0, 10.0, 10.0, 0.0, 0.0, 0.0, 0.0, 10.0, 0.0, 0.0,
        ];
        let m = [
            m[0], m[1], m[2], m[3], m[4], m[5], m[6], m[7], 1.0, 2.0, 3.0, 0.0,
        ];
        let w = world(Some(&m));
        // Translation in elements 3, 7, 11 of the shaders' form.
        assert_eq!([w[3], w[7], w[11], w[15]], [1.0, 2.0, 3.0, 1.0]);
        assert_eq!(transpose(&transpose(&w)), w);
        assert_eq!(world(None), IDENTITY);
    }

    #[test]
    #[allow(clippy::cast_possible_truncation)] // the f32 stores
    fn depth_range_follows_the_original() {
        let mut p = IDENTITY;
        p[11] = -1.0;
        p[15] = 0.0;
        let z = depth_zero_one(&p, 0.5, 1000.0);
        assert_eq!(z[10], (1000.0f64 / (0.5 - 1000.0)) as f32);
        assert_eq!(z[14], (500.0f64 / (0.5 - 1000.0)) as f32);
        let o = depth_zero_one(&IDENTITY, 1.0, 3.0);
        assert_eq!([o[10], o[14]], [-0.5, -0.5]);
    }

    #[test]
    fn parameters_respect_the_size_word() {
        let m = Material {
            parameters: vec![12, 0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8],
            ..Material::default()
        };
        assert_eq!(parameter(&m, 4, 4), Some(&[1, 2, 3, 4][..]));
        assert_eq!(parameter(&m, 8, 8), None);
        assert_eq!(parameter(&m, 0, 4), None);
    }
}
