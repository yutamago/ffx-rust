//! M5 stage 1, list `scene`, batch 7: the fly controller's two
//! integration steps, the context switch's default state and the
//! attachable component's joint target. Conventions as in
//! [`crate::m5p_scene_b`]; floats on the x87 at 53 bits, each store
//! rounded to `f32`; frames as in [`crate::m5p_scene_f`].
//!
//! Image addresses are rebased before use. Nothing here keeps state of
//! its own (worker threads: per-call scratch only).

use ffx_bridge::{Mem, x87};
use ffx_graphic::host::{Conv, Host};

pub use crate::m5p_scene_b::{PHYRE_FREE, TO_WORLD_MATRIX};
pub use crate::m5p_scene_c::PHYRE_MALLOC;
pub use crate::m5p_scene_e::MEMCPY;
pub use crate::m5p_scene_f::FROM_MATRIX3X4;

/// `0x004d3720(items, n)` (cdecl): the context switch state pairs'
/// release.
pub const STATE_RELEASE: u32 = 0x004d_3720;
/// The name fields the joint target points at (`.data` words): a joint's
/// and a node's.
pub const JOINT_NAME: u32 = 0x00ca_7228;
pub const NODE_CLASS: u32 = 0x00c9_2090;

pub mod bridge {
    //! The ports over game memory.

    use super::{
        Conv, FROM_MATRIX3X4, Host, JOINT_NAME, MEMCPY, Mem, NODE_CLASS, PHYRE_FREE, PHYRE_MALLOC,
        STATE_RELEASE, TO_WORLD_MATRIX, x87,
    };

    const EXTERNAL: u32 = 0x8000_0000;

    fn ld(m: &impl Mem, at: u32) -> f64 {
        let bits: u32 = m.read(at);
        x87::load(bits)
    }

    /// The fly controller's 4x4 (`+0x34`) from its 3x4 (`+4`, through the
    /// frame, 16 words).
    fn refresh_matrix(h: &mut impl Host, this: u32, frame: u32) {
        h.call_sym(FROM_MATRIX3X4, Conv::Thiscall(frame), &[this + 4]);
        for i in 0..16 {
            let f: f32 = h.read(frame + 4 * i);
            h.write(this + 0x34 + 4 * i, x87::round_trip(f));
        }
    }

    /// `position += step` at `+0x64`, then the 3x4 rebuilt from the 4x4;
    /// `PMatrix4_toWorldMatrix3x4`'s `eax`.
    fn move_by(h: &mut impl Host, this: u32, step: [u32; 3]) -> u32 {
        let mut p = [0u32; 3];
        for (i, s) in step.into_iter().enumerate() {
            let at = this + 0x64 + 4 * u32::try_from(i).unwrap_or(0);
            let q = x87::store(x87::load(x87::store(ld(h, at))));
            p[i] = x87::store(x87::add(x87::load(q), x87::load(s)));
        }
        for (i, v) in p.into_iter().enumerate() {
            h.write(this + 0x64 + 4 * u32::try_from(i).unwrap_or(0), v);
        }
        h.call_sym(TO_WORLD_MATRIX, Conv::Thiscall(this + 4), &[this + 0x34])
    }

    /// `0x005d7190` (thiscall `(dt)`, `ret 4`): the fly controller moved
    /// back along its forward axis: the 4x4 refreshed, `-row2 * dt`
    /// added to the position.
    pub fn fly_forward(h: &mut impl Host, this: u32, dt: f32, frame: u32) -> u32 {
        refresh_matrix(h, this, frame);
        let dt = x87::load(dt.to_bits());
        let mut step = [0u32; 3];
        for (i, s) in step.iter_mut().enumerate() {
            let at = this + 0x54 + 4 * u32::try_from(i).unwrap_or(0);
            let n = x87::store(-ld(h, at));
            *s = x87::store(x87::mul(x87::load(n), dt));
        }
        move_by(h, this, step)
    }

    /// `0x005d72b0` (thiscall `(a, b)`, `ret 8`): the fly controller moved
    /// in its plane: the 4x4 refreshed, `row0 * a + row1 * b` added to the
    /// position.
    pub fn fly_strafe(h: &mut impl Host, this: u32, a: f32, b: f32, frame: u32) -> u32 {
        refresh_matrix(h, this, frame);
        let a = x87::load(a.to_bits());
        let b = x87::load(b.to_bits());
        let mut step = [0u32; 3];
        for (i, s) in step.iter_mut().enumerate() {
            let i = u32::try_from(i).unwrap_or(0);
            let up = x87::store(x87::mul(ld(h, this + 0x44 + 4 * i), b));
            let side = x87::store(x87::mul(ld(h, this + 0x34 + 4 * i), a));
            *s = x87::store(x87::add(x87::load(side), x87::load(up)));
        }
        move_by(h, this, step)
    }

    /// A `PArray` of one 4-byte element at `pair`: a fresh buffer unless
    /// it holds one already (an owned, non-null old one other than it
    /// freed). False when the allocation failed.
    fn one_word(h: &mut impl Host, pair: u32) -> bool {
        let count: u32 = h.read(pair);
        if count == 1 {
            return true;
        }
        let buf = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[4, 4]);
        if buf == 0 {
            return false;
        }
        let old: u32 = h.read(pair + 4);
        let header: u32 = h.read(pair);
        if old != buf && header & EXTERNAL == 0 && old != 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
        }
        h.write(pair + 4, buf);
        h.write(pair, 1u32);
        true
    }

    /// `0x004d4d50` (thiscall, `(unused, state)`, `ret 8`): a context
    /// switch's default state: `state` made a `PArray` of two pairs (16
    /// bytes, the old ones released when owned), each pair a one-word
    /// array, holding 0 and 1. 0, or 0xd when an allocation failed.
    pub fn switch_default_state(h: &mut impl Host, state: u32) -> u32 {
        let count: u32 = h.read(state);
        if count != 2 {
            let pairs = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[0x10, 4]);
            if pairs == 0 {
                return 0xd;
            }
            for i in 0..4 {
                h.write(pairs + 4 * i, 0u32);
            }
            let old: u32 = h.read(state + 4);
            if old != pairs {
                let header: u32 = h.read(state);
                if header & EXTERNAL == 0 {
                    h.call_sym(STATE_RELEASE, Conv::Cdecl, &[old, header & !EXTERNAL]);
                }
            }
            h.write(state + 4, pairs);
            h.write(state, 2u32);
        }
        let pairs: u32 = h.read(state + 4);
        if !one_word(h, pairs) {
            return 0xd;
        }
        let pairs: u32 = h.read(state + 4);
        if !one_word(h, pairs + 8) {
            return 0xd;
        }
        let pairs: u32 = h.read(state + 4);
        let first: u32 = h.read(pairs + 4);
        h.write(first, 0u32);
        let pairs: u32 = h.read(state + 4);
        let second: u32 = h.read(pairs + 0xc);
        h.write(second, 1u32);
        0
    }

    /// Whether the bytes at `a` and `b` are the same string.
    fn same_string(m: &impl Mem, a: u32, b: u32) -> bool {
        let mut i = 0u32;
        loop {
            let x: u8 = m.read(a.wrapping_add(i));
            let y: u8 = m.read(b.wrapping_add(i));
            if x != y {
                return false;
            }
            if x == 0 {
                return true;
            }
            i += 1;
        }
    }

    /// The old name (`+0x54`) freed unless unowned.
    fn free_name(h: &mut impl Host, this: u32) {
        let lo: u8 = h.read(this + 0x54);
        if lo & 1 == 0 {
            let old: u32 = h.read(this + 0x54);
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
        }
    }

    /// `0x0055a980` (thiscall `(skeleton, name)`, `ret 8`): the
    /// attachable component's joint target: with a name, the joint of that
    /// name (`*skeleton`'s names `+0x24`, count `+0x18`; each with an
    /// ownership bit) and a copy of the name (`PhyreMalloc(len + 1, 2)`;
    /// 0xd when that fails); without, the skeleton's node (`+4`). 0x13
    /// when there is none; else the fields set and 0.
    pub fn attach_joint(h: &mut impl Host, this: u32, skeleton: u32, name: u32) -> u32 {
        let joint_name = h.rebase(JOINT_NAME);
        let named = name != 0 && {
            let c: u8 = h.read(name);
            c != 0
        };
        if !named {
            let node: u32 = h.read(skeleton + 4);
            if node == 0 {
                return 0x13;
            }
            h.write(this + 0x4c, skeleton);
            h.write(this + 0x50, joint_name);
            free_name(h, this);
            h.write(this + 0x54, 0u32);
            h.write(this + 0x5c, u32::MAX);
            h.write(this + 0x60, node);
            let class = h.rebase(NODE_CLASS);
            h.write(this + 0x64, class);
            return 0;
        }
        let obj: u32 = h.read(skeleton);
        let names: u32 = h.read(obj + 0x24);
        let count: u32 = h.read(obj + 0x18);
        let mut found = None;
        for i in 0..count & !EXTERNAL {
            let at = names.wrapping_add(4 * i);
            let lo: u8 = h.read(at);
            let p: u32 = h.read(at);
            let s = p.wrapping_sub(u32::from(lo & 1));
            if s != 0 && same_string(h, s, name) {
                found = Some(i);
                break;
            }
        }
        let Some(index) = found else {
            return 0x13;
        };
        let mut len = 0u32;
        loop {
            let c: u8 = h.read(name.wrapping_add(len));
            if c == 0 {
                break;
            }
            len += 1;
        }
        let copy = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[len + 1, 2]);
        if copy == 0 {
            return 0xd;
        }
        h.call_sym(MEMCPY, Conv::Cdecl, &[copy, name, len + 1]);
        free_name(h, this);
        h.write(this + 0x54, copy);
        h.write(this + 0x5c, index);
        h.write(this + 0x4c, skeleton);
        h.write(this + 0x50, joint_name);
        h.write(this + 0x60, skeleton);
        h.write(this + 0x64, joint_name);
        0
    }
}
