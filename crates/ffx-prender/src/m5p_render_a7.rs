//! M5 stage 1, `PhyreEngine` list `render-a`, batch 7: the scene
//! utilities' world matrices (`PUtilityScene`: a node's matrix composed
//! up its parent chain, the locator's world matrix, the gameplay
//! object's update). Each original keeps matrices and vectors in its
//! stack frame and hands their addresses to the `Vectormath` routines;
//! the bridges take that frame (`frame`, laid out as the original's
//! `ebp - size`).

use ffx_graphic::host::{Conv, Host};

use crate::m5p_render_a::f32_move;

/// The class descriptors `FUN_0055b670` tests the object's class chain
/// for (as `FUN_0055b490`).
pub use crate::m5p_render_a4::MATRIX_SOURCE_CLASSES;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;

    fn this_call(h: &mut impl Host, f: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Thiscall(this), args)
    }

    /// `FUN_00506660(this; out)` (`ret 4`) -> out, with `frame` its
    /// 0x100-byte frame: `out` = the node's matrix `+0x10`; then for each
    /// parent (`+4` chain) the matrix rebuilt from `out`'s rows through
    /// the parent's matrix (the translation by `0x004c9eb0`, the three
    /// axes by `0x004ca140`, all four composed by `0x00450de0(r0, r1, r2,
    /// t)` into the frame's matrix at `ebp-0x80`) and copied back to
    /// `out`.
    pub fn node_world_matrix(h: &mut impl Host, this: u32, out: u32, frame: u32) -> u32 {
        // ebp - n
        let at = |n: u32| frame + 0x100 - n;
        for i in 0..16 {
            let v: u32 = h.read(this + 0x10 + 4 * i);
            h.write(out + 4 * i, v);
        }
        let mut node: u32 = h.read(this + 4);
        while node != 0 {
            // rows 3, 2, 1, 0: the raw words at ebp-0x30.., -0x20.., -0x10..,
            // -0x40.., their x, y, z moved to the vectors at ebp-0xc0, -0xb0,
            // -0x90, -0xa0
            for (row, raw, vec) in [
                (3u32, 0x30u32, 0xc0u32),
                (2, 0x20, 0xb0),
                (1, 0x10, 0x90),
                (0, 0x40, 0xa0),
            ] {
                for k in 0..4 {
                    let w: u32 = h.read(out + 16 * row + 4 * k);
                    h.write(at(raw) + 4 * k, w);
                    if k < 3 {
                        h.write(at(vec) + 4 * k, f32_move(w));
                    }
                }
            }
            let m = node + 0x10;
            // the transformed rows, pushed one by one for the final call
            let r1 = this_call(h, 0x004c_9eb0, m, &[at(0x100), at(0xc0)]);
            let r2 = this_call(h, 0x004c_a140, m, &[at(0xd0), at(0xb0)]);
            let r3 = this_call(h, 0x004c_a140, m, &[at(0xf0), at(0x90)]);
            let r4 = this_call(h, 0x004c_a140, m, &[at(0xe0), at(0xa0)]);
            this_call(h, 0x0045_0de0, at(0x80), &[r4, r3, r2, r1]);
            for i in 0..16 {
                let v: u32 = h.read(at(0x80) + 4 * i);
                h.write(out + 4 * i, f32_move(v));
            }
            node = h.read(node + 4);
        }
        out
    }

    /// `FUN_00507410(this; out)` (`ret 4`), with `frame` its 0x100-byte
    /// frame: the local matrix `+0x10` takes `out` relative to the
    /// parent (`+4`): the parent's own matrix object `+0xc` (copied to
    /// the frame, `0x00451080`) or its composed world matrix
    /// (`FUN_00506660`), inverted (`0x004cc0c0`), and multiplied with `out`
    /// (`0x004c9430`, `0x004c9da0`); no parent: `out` itself
    /// (`PMatrix4_Copy`); then the object `+0xc` gets `out` as its world
    /// matrix (`PMatrix4_toWorldMatrix3x4`). The last call's `eax`.
    pub fn set_world_matrix(h: &mut impl Host, this: u32, out: u32, frame: u32) -> u32 {
        let at = |n: u32| frame + 0x100 - n;
        let parent: u32 = h.read(this + 4);
        let src = if parent == 0 {
            out
        } else {
            let own: u32 = h.read(parent + 0xc);
            let (m, tmp) = if own != 0 {
                this_call(h, 0x0045_1080, at(0x40), &[own]);
                (at(0x40), at(0xc0))
            } else {
                let w = this_call(h, 0x0050_6660, parent, &[at(0xc0)]);
                (w, at(0x100))
            };
            let inv = h.call_sym(0x004c_c0c0, Conv::Cdecl, &[at(0x80), m]);
            this_call(h, 0x004c_9430, at(0x40), &[inv]);
            this_call(h, 0x004c_9da0, at(0x40), &[tmp, out])
        };
        let mut r = this_call(h, 0x0045_3340, this + 0x10, &[src]);
        let obj: u32 = h.read(this + 0xc);
        if obj != 0 {
            r = this_call(h, 0x0045_1190, obj, &[out]);
        }
        r
    }

    /// Whether the class chain from `desc` (parents `+0x40`) holds
    /// `class`.
    fn is_a(h: &mut impl Host, desc: u32, class: u32) -> bool {
        let class = h.rebase(class);
        let mut d = desc;
        while d != 0 {
            if d == class {
                return true;
            }
            d = h.read(d + 0x40);
        }
        false
    }

    /// `FUN_0055b670(this)`, with `frame` its 0x80-byte frame: for an
    /// enabled (`+0x68`) object with a target (`[+4] + 4`, or `+0x58`) and
    /// a source `+0x60`: the source's matrix by its class (a matrix
    /// holder copied, `0x00451080`; a node asked, `0x004f71e0(node; m,
    /// [+0x5c])`; else the scene node `0x005337d0(this + 0x60)` composed,
    /// `FUN_00506660`), times the offset `+0xc` (`0x004c9da0`), handed on:
    /// for a matrix holder to `FUN_00507410([+0x58]; m)` or
    /// `PMatrix4_toWorldMatrix3x4([[+4] + 4]; m)`, else to
    /// `PGameplay_forward58_0055b780(this; m)`. `eax` unused.
    pub fn update_object(h: &mut impl Host, this: u32, frame: u32) {
        let at = |n: u32| frame + 0x80 - n;
        if h.read::<u8>(this + 0x68) == 0 {
            return;
        }
        let target: u32 = h.read(this + 4);
        if (target == 0 || h.read::<u32>(target + 4) == 0) && h.read::<u32>(this + 0x58) == 0 {
            return;
        }
        let obj: u32 = h.read(this + 0x60);
        if obj == 0 {
            return;
        }
        let desc: u32 = h.read(this + 0x64);
        if is_a(h, desc, MATRIX_SOURCE_CLASSES[0]) {
            this_call(h, 0x0045_1080, at(0x40), &[obj]);
            let m = this_call(h, 0x004c_9da0, at(0x40), &[at(0x80), this + 0xc]);
            let scene: u32 = h.read(this + 0x58);
            if scene != 0 {
                this_call(h, 0x0050_7410, scene, &[m]);
                return;
            }
            let t: u32 = h.read(this + 4);
            let w = if t == 0 { 0 } else { h.read(t + 4) };
            if w != 0 {
                this_call(h, 0x0045_1190, w, &[m]);
            }
            return;
        }
        if is_a(h, desc, MATRIX_SOURCE_CLASSES[1]) {
            let k: u32 = h.read(this + 0x5c);
            this_call(h, 0x004f_71e0, obj, &[at(0x40), k]);
        } else {
            let node = this_call(h, 0x0053_37d0, this + 0x60, &[]);
            if node == 0 {
                return;
            }
            this_call(h, 0x0050_6660, node, &[at(0x40)]);
        }
        let m = this_call(h, 0x004c_9da0, at(0x40), &[at(0x80), this + 0xc]);
        this_call(h, 0x0055_b780, this, &[m]);
    }
}
