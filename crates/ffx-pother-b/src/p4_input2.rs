//! The input mapper's per-frame update (`PInputMapper::update`).
//!
//! The mapper (`PInputMapper`): the mouse's x / y mappings (`+4`, `+8`)
//! and its three button mappings (`+0xc..0x14`), the pads (`+0x38 + 4
//! pad`), the button states (`+0xa0..0xa2`) and their latches
//! (`+0xa3..0xa5`), the absolute-mouse byte (`+0xa6`), the activity byte
//! (`+0xa7`), the mouse position (`+0xa8`, `+0xac`) and its deltas
//! (`+0xb0`, `+0xb4`).

use ffx_bridge::x87;
use ffx_graphic::host::{Conv, Host};

use crate::p4_input::stick_axis;

/// `PInputMapper::attachPads` (thiscall `(mapper)`).
pub const ATTACH_PADS: u32 = 0x0062_6aa0;

pub mod bridge {
    //! The originals over game memory.

    use super::{ATTACH_PADS, Conv, Host, stick_axis, x87};

    /// One stick axis of pad 0 as `update` reads it: the axis's mapping
    /// `own` and the cross axis's `other` (`dev + 0x38 + 4 k`). The
    /// deadzone is `own`'s threshold while `other`'s magnitude is under
    /// `other`'s threshold (a NaN is not under), else 0; the value is
    /// `own`'s (0 for none) unless its magnitude is under the deadzone.
    fn axis(h: &mut impl Host, dev: u32, own_k: u32, other_k: u32) -> f32 {
        let own: u32 = h.read(dev + 0x38 + 4 * own_k);
        let other: u32 = h.read(dev + 0x38 + 4 * other_k);
        let v: u32 = h.read(other + 0x1c);
        let t: u32 = h.read(other + 0x18);
        // `fcompp` with `test ah, 0x41`: the own threshold only when
        // `t > |v|`, ordered
        let dz = if x87::load(t) > x87::load(v).abs() {
            h.read(own + 0x18)
        } else {
            0
        };
        // the original tests `own` here only; a null mapping has already
        // been read through as the cross axis (unreachable in the game)
        let value = if own == 0 {
            None
        } else {
            Some(h.read(own + 0x1c))
        };
        stick_axis(dz, value)
    }

    /// One mouse axis: the mapping's value (`+0x1c`) truncated
    /// (`_ftol2_sse`); absolute (`+0xa6` set) it becomes the position and
    /// the delta is its change, else it is the delta and adds to the
    /// position. Returns whether the delta is non-zero.
    fn mouse_axis(h: &mut impl Host, mapper: u32, mapping: u32, pos: u32, delta: u32) -> bool {
        let v: u32 = h.read(mapping + 0x1c);
        let i = x87::ftol(&*h, x87::load(v));
        let absolute: u8 = h.read(mapper + 0xa6);
        let d = if absolute == 0 {
            let p: u32 = h.read(mapper + pos);
            h.write(mapper + pos, p.wrapping_add(i));
            i
        } else {
            let p: u32 = h.read(mapper + pos);
            let d = i.wrapping_sub(p);
            h.write(mapper + pos, i);
            d
        };
        h.write(mapper + delta, d);
        d != 0
    }

    /// `PInputMapper::update(mapper)` (`0x0062e040`, thiscall, no stack
    /// words, nothing in `eax` callers read): attaches new pads
    /// (`0x00626aa0`), clears the activity byte `+0xa7`, then sets it
    /// for a mouse move (x `+4` into `+0xa8`/`+0xb0`, y `+8` into
    /// `+0xac`/`+0xb4`), and latches the mouse buttons (`+0xc`, `+0x10`,
    /// `+0x14`: state byte `+0x24` != 0 into `+0xa0`, `+0xa2`, `+0xa1`,
    /// or-ed into `+0xa3`, `+0xa5`, `+0xa4`). Pad 0's four stick axes
    /// (left x/y against each other, right x/y likewise) set the
    /// activity byte when any is non-zero (NaN included), and so does
    /// pad 0's byte `+0x1c` or `+0x1d`.
    pub fn update(h: &mut impl Host, mapper: u32) {
        h.call_sym(ATTACH_PADS, Conv::Thiscall(mapper), &[]);
        h.write(mapper + 0xa7, 0u8);
        let mx: u32 = h.read(mapper + 4);
        if mx != 0 && mouse_axis(h, mapper, mx, 0xa8, 0xb0) {
            h.write(mapper + 0xa7, 1u8);
        }
        let my: u32 = h.read(mapper + 8);
        if my != 0 && mouse_axis(h, mapper, my, 0xac, 0xb4) {
            h.write(mapper + 0xa7, 1u8);
        }
        for (mapping, state, latch) in [(0xc, 0xa0, 0xa3), (0x10, 0xa2, 0xa5), (0x14, 0xa1, 0xa4)] {
            let m: u32 = h.read(mapper + mapping);
            if m != 0 {
                let on = u8::from(h.read::<u8>(m + 0x24) != 0);
                let l: u8 = h.read(mapper + latch);
                h.write(mapper + latch, l | on);
                h.write(mapper + state, on);
            }
        }
        let dev: u32 = h.read(mapper + 0x38);
        let sticks = if dev == 0 {
            [0.0; 4]
        } else {
            [
                axis(h, dev, 0, 1),
                axis(h, dev, 1, 0),
                axis(h, dev, 2, 3),
                axis(h, dev, 3, 2),
            ]
        };
        // `fucom` against 0: NaN is not equal
        if sticks.iter().any(|&s| s != 0.0) {
            h.write(mapper + 0xa7, 1u8);
        }
        let dev: u32 = h.read(mapper + 0x38);
        if dev != 0 && (h.read::<u8>(dev + 0x1c) != 0 || h.read::<u8>(dev + 0x1d) != 0) {
            h.write(mapper + 0xa7, 1u8);
        }
    }
}
