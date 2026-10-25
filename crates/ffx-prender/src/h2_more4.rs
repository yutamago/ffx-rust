//! h2-render-b round 3, batch 5: the render-state block constructor, the
//! constant-buffer slot array resize, another job wait-and-clear.

use ffx_graphic::host::{Conv, Host};

use crate::array::{BORROWED, COUNT};
use crate::descriptor::PHYRE_FREE;

/// The storage of the `PSharray` at `array`: the data word for more than
/// one element, the inline word for one, null for none.
fn storage(h: &mut impl Host, array: u32) -> u32 {
    match h.read::<u32>(array) & COUNT {
        0 => 0,
        1 => array + 4,
        _ => h.read(array + 4),
    }
}

pub mod bridge {
    //! The originals over game memory.

    use super::{BORROWED, COUNT, Conv, Host, PHYRE_FREE, storage};

    /// `0x0055dfa0(this)`: the spin wait `0x00564630(this)`, then
    /// `0x005646d0(this)` when bit 0 of `+0x34` is set; the `PSharray` at
    /// `+0x18` freed when owned, holding more than one element and not
    /// null, the one at `+8` when owned and not null; both cleared.
    pub fn wait_and_clear2(h: &mut impl Host, this: u32) {
        h.call_sym(0x0056_4630, Conv::Thiscall(this), &[]);
        if h.read::<u8>(this + 0x34) & 1 != 0 {
            h.call_sym(0x0056_46d0, Conv::Thiscall(this), &[]);
        }
        let n: u32 = h.read(this + 0x18);
        if n & BORROWED == 0 && n & COUNT > 1 {
            let data: u32 = h.read(this + 0x1c);
            if data != 0 {
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
            }
        }
        h.write(this + 0x1c, 0u32);
        h.write(this + 0x18, 0u32);
        if h.read::<u32>(this + 8) & BORROWED == 0 {
            let data: u32 = h.read(this + 0xc);
            if data != 0 {
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
            }
        }
        h.write(this + 0xc, 0u32);
        h.write(this + 8, 0u32);
    }

    /// `0x00583e20(this)`: the render-state block's defaults (`fldz`
    /// stores at `+0x14`/`+0x18`), the sub-block at `+0x60` built by
    /// `0x00582f50(+0x60; byte 0x00b3144c)`, then `+0x168..+0x178`
    /// cleared and `+8 = +0x24 = 1`; returns `this`.
    pub fn state_block_ctor(h: &mut impl Host, this: u32) -> u32 {
        let words: [(u32, u32); 25] = [
            (0, 0),
            (0x14, 0),
            (0x18, 0),
            (4, 3),
            (8, 3),
            (0xc, 0),
            (0x10, 0),
            (0x1c, 1),
            (0x20, 0),
            (0x24, 0),
            (0x28, 0),
            (0x40, 1),
            (0x50, 1),
            (0x44, 1),
            (0x54, 1),
            (0x48, 1),
            (0x58, 1),
            (0x2c, 1),
            (0x30, 1),
            (0x34, 2),
            (0x38, 0),
            (0x4c, 8),
            (0x5c, 8),
            (0x168, 0),
            (0x16c, 0),
        ];
        for &(off, v) in &words[..23] {
            h.write(this + off, v);
        }
        h.write(this + 0x3c, 0xffffu16);
        let g = h.rebase(0x00b3_144c);
        let b = u32::from(h.read::<u8>(g));
        h.call_sym(0x0058_2f50, Conv::Thiscall(this + 0x60), &[b]);
        for &(off, v) in &words[23..] {
            h.write(this + off, v);
        }
        h.write(this + 0x170, 0u32);
        h.write(this + 0x174, 0u32);
        h.write(this + 0x178, 0u8);
        h.write(this + 8, 1u32);
        h.write(this + 0x24, 1u32);
        this
    }

    /// `0x00599a80(this; n)`: 0x16 while `+0x38` is set; else, unless the
    /// slot array at `+0xc` holds `n` already, `0x00599460(+0xc; n)`
    /// resizes it and each of the `n` 16-byte slots is reset: word 0
    /// cleared, word 1 freed when set, word 3 cleared. Returns 0.
    pub fn resize_slots(h: &mut impl Host, this: u32, n: u32) -> u32 {
        if h.read::<u32>(this + 0x38) != 0 {
            return 0x16;
        }
        let arr = this + 0xc;
        if n == h.read::<u32>(arr) & COUNT {
            return 0;
        }
        h.call_sym(0x0059_9460, Conv::Thiscall(arr), &[n]);
        for i in 0..n {
            let at = 0x10 * i;
            let s = storage(h, arr);
            h.write(s.wrapping_add(at), 0u32);
            let s = storage(h, arr);
            let p: u32 = h.read(s.wrapping_add(at + 4));
            if p != 0 {
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[p]);
            }
            let s = storage(h, arr);
            h.write(s.wrapping_add(at + 0xc), 0u32);
        }
        0
    }
}
