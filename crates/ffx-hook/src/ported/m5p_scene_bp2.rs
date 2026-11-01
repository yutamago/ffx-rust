//! M5 stage 1, list `scene`, round big-phyre2 (`ffx_prender::m5p_scene_bp2`):
//! the occluder classes' `Bind()` tables run by `ffx_prender::h1_bind`,
//! and the camera's look-at. Parity tests:
//! `crates/ffx-oracle/tests/m5p_scene.rs` (`m5p_scene_bp2_*`).

use ffx_prender::h1_bind::bridge::bind;
use ffx_prender::m5p_scene_bp2 as t;

use super::{StackFrame, mem, ported};

ported! {
    /// `POccluderGeometryInstance::Bind()`: `addToNamespace`'s `eax`.
    Phyre_POccluderGeometry_POccluderGeometryInstance_Bind @ 0x00a3_0c90 => extern "C" fn sbp2_00a30c90() -> u32 {
        bind(&mut mem(), t::INSTANCE_BIND)
    }
    /// `POccluderGeometryObject::Bind()`: `addToNamespace`'s `eax`.
    Phyre_POccluderGeometry_POccluderGeometryObject_Bind @ 0x00a4_0270 => extern "C" fn sbp2_00a40270() -> u32 {
        bind(&mut mem(), t::OBJECT_BIND)
    }
    /// `Phyre_PGameplay_cameraLookAt_005d6f00(this; target)`: the matrices
    /// and vectors in a frame; `toWorldMatrix3x4`'s `eax`.
    Phyre_PGameplay_cameraLookAt_005d6f00 @ 0x005d_6f00 => extern "thiscall" fn sbp2_005d6f00(this: u32, target: u32) -> u32 {
        let mut f = StackFrame::<{ t::LOOK_AT_FRAME_WORDS as usize }>::new();
        let ebp = f.at() + 4 * t::LOOK_AT_FRAME_WORDS;
        t::look::look_at(&mut mem(), ebp, this, target)
    }
    /// `Phyre_PRendering_PInternal_PCheckVisibleBoundsJob_boxPlanes_009f2a60(first, end, inner, origin, extent, m)`:
    /// the corners in a frame; -1, 0 or 1.
    Phyre_PRendering_PInternal_PCheckVisibleBoundsJob_boxPlanes_009f2a60 @ 0x009f_2a60 => extern "C" fn sbp2_009f2a60(first: u32, end: u32, inner: u32, origin: u32, extent: u32, m: u32) -> u32 {
        let mut f = StackFrame::<{ t::BOX_PLANES_FRAME_WORDS as usize }>::new();
        let ebp = f.at() + 4 * t::BOX_PLANES_FRAME_WORDS;
        t::box_planes::box_planes(&mut mem(), ebp, first, end, inner, origin, extent, m)
    }
    /// `PVisibilityCheck_isBoxOccluded(occluders; origin, extent, m)`: the
    /// corners in a frame; `al` 0 when occluded, else 1.
    PVisibilityCheck_isBoxOccluded @ 0x009f_4100 => extern "thiscall" fn sbp2_009f4100(this: u32, origin: u32, extent: u32, m: u32) -> u8 {
        let mut f = StackFrame::<{ t::BOX_OCCLUDED_FRAME_WORDS as usize }>::new();
        let ebp = f.at() + 4 * t::BOX_OCCLUDED_FRAME_WORDS;
        t::box_occluded::is_box_occluded(&mut mem(), ebp, this, origin, extent, m)
    }
    /// `Phyre_POccluderGeometry_rayBox_00a2fe70(m, lo, hi, p, d, near, far)`:
    /// the distances in a frame; `al` 1 when the ray meets the box.
    Phyre_POccluderGeometry_rayBox_00a2fe70 @ 0x00a2_fe70 => extern "C" fn sbp2_00a2fe70(m: u32, lo: u32, hi: u32, p: u32, d: u32, near: u32, far: u32) -> u8 {
        let mut f = StackFrame::<{ t::RAY_BOX_FRAME_WORDS as usize }>::new();
        let ebp = f.at() + 4 * t::RAY_BOX_FRAME_WORDS;
        t::ray_box::ray_box(&mut mem(), ebp, m, lo, hi, p, d, near, far)
    }
    /// `Phyre_POccluderGeometry_selectOccluders_00a30260(array, items, count, stride, owner, ray)`:
    /// the matrices and the ray in a frame; 0.
    Phyre_POccluderGeometry_selectOccluders_00a30260 @ 0x00a3_0260 => extern "C" fn sbp2_00a30260(array: u32, items: u32, count: u32, stride: u32, owner: u32, ray: u32) -> u32 {
        let mut f = StackFrame::<{ t::SELECT_FRAME_WORDS as usize }>::new();
        let ebp = f.at() + 4 * t::SELECT_FRAME_WORDS;
        t::select::select_occluders(&mut mem(), ebp, array, items, count, stride, owner, ray)
    }
    /// `Phyre_POccluderGeometry_screenCover_00a2ef70(lo, hi, owner, m, inverse)`:
    /// the corners and polygons in a frame; the cover in `ST0`.
    Phyre_POccluderGeometry_screenCover_00a2ef70 @ 0x00a2_ef70 => extern "C" fn sbp2_00a2ef70(lo: u32, hi: u32, owner: u32, m: u32, inverse: u32) -> f32 {
        let mut f = StackFrame::<{ t::COVER_FRAME_WORDS as usize }>::new();
        let ebp = f.at() + 4 * t::COVER_FRAME_WORDS;
        t::cover::screen_cover(&mut mem(), ebp, lo, hi, owner, m, inverse)
    }
}
