//! M5 stage 1, list `scene`, batch 12: the camera controller's look-at
//! update. Conventions as in [`crate::m5p_scene_b`]; frames as in
//! [`crate::m5p_scene_f`].

use ffx_bridge::x87;
use ffx_graphic::host::{Conv, Host};

pub use crate::m5p_scene_b::FLY_SET;
pub use crate::m5p_scene_f::FROM_MATRIX3X4;

/// `Vector3_normalize_004525a0(out, v)` (cdecl).
pub const NORMALIZE: u32 = 0x0045_25a0;
/// `0x006058b0` (thiscall `(&position, &direction, f)`, `ret 0xc`): the
/// look-at helper's set-up.
pub const LOOK_AT_SET: u32 = 0x0060_58b0;

pub mod bridge {
    //! The ports over game memory.

    use super::{Conv, FLY_SET, FROM_MATRIX3X4, Host, LOOK_AT_SET, NORMALIZE, x87};

    /// `0x00558b10` (thiscall `(f)`, `ret 4`): when the controller is on
    /// (`+0xb1`) and follows a node (`+0xb4`): the node's 4x4 into the
    /// frame (28 words, the original's `ebp-0x70`), its position (row 3)
    /// and its forward axis (row 2, normalized) handed with `f` to the
    /// look-at helper (`+0xbc`), then the fly controller (`+0x34`) set to
    /// `+0xfc` and `+0x10c`. Its `eax`: the last call's, 0 for no node;
    /// `None` when off (the original leaves its caller's `eax`).
    pub fn camera_follow(h: &mut impl Host, this: u32, f: f32, frame: u32) -> Option<u32> {
        let on: u8 = h.read(this + 0xb1);
        if on == 0 {
            return None;
        }
        let node: u32 = h.read(this + 0xb4);
        if node == 0 {
            return Some(0);
        }
        h.call_sym(FROM_MATRIX3X4, Conv::Thiscall(frame), &[node]);
        let pos = frame + 0x50;
        let axis = frame + 0x60;
        let dir = frame + 0x40;
        for i in 0..3 {
            let p: f32 = h.read(frame + 0x30 + 4 * i);
            h.write(pos + 4 * i, x87::round_trip(p));
            let a: f32 = h.read(frame + 0x20 + 4 * i);
            h.write(axis + 4 * i, x87::round_trip(a));
        }
        h.call_sym(NORMALIZE, Conv::Cdecl, &[dir, axis]);
        let f = x87::round_trip(f).to_bits();
        h.call_sym(LOOK_AT_SET, Conv::Thiscall(this + 0xbc), &[pos, dir, f]);
        h.call_sym(FLY_SET[1], Conv::Thiscall(this + 0x34), &[this + 0xfc]);
        Some(h.call_sym(FLY_SET[0], Conv::Thiscall(this + 0x34), &[this + 0x10c]))
    }
}
