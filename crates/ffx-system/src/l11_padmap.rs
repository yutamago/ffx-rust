//! M5 round 11 (`tail11`): `Input_BuildPadMap` (`0x006315c0`, thiscall
//! on the input manager), the key bindings paired with their PS2 pad bits.
//!
//! The manager's pair array at `+0x9c` (`+0` count, `+4` capacity, `+8`
//! data, 8-byte pairs) is emptied (`PairArray_assign` from an empty
//! array), reserved to 75, then 75 pairs pushed: a word of the settings
//! object (`*g_Settings`: the FFX bindings `+0x60..`, the FFX-2 ones
//! `+0x124..`, a few others) or 0, and the pad bit it stands for. Each push
//! grows the array (`PairArray_reserve(count + 2 + count / 8)`) when the
//! new count passes the capacity.

/// The original as a bridge over [`ffx_graphic::host::Host`].
pub mod bridge {
    use ffx_graphic::host::{Conv, Host};

    /// `g_Settings` (`0x022fb504`): the settings object.
    pub const SETTINGS: u32 = 0x022f_b504;
    /// `PairArray_assign_00631110(this; other)` (thiscall, `ret 4`).
    pub const ASSIGN: u32 = 0x0063_1110;
    /// `PairArray_reserve_00631450(this; n)` (thiscall, `ret 4`).
    pub const RESERVE: u32 = 0x0063_1450;
    /// Bytes of [`build_pad_map`]'s frame: the empty array (three words at
    /// the original's `ebp - 0xc`) handed to `PairArray_assign`.
    pub const PAD_MAP_FRAME: u32 = 0xc;

    /// The pairs in push order: the settings word's offset (or `None`: 0)
    /// and the pad bit, as the original's immediates.
    #[allow(clippy::unreadable_literal)] // the immediates as in the original
    pub const PAIRS: [(Option<u32>, u32); 75] = [
        (Some(0x060), 0x1000),
        (Some(0x064), 0x4000),
        (Some(0x068), 0x8000),
        (Some(0x06c), 0x2000),
        (Some(0x070), 0x20),
        (Some(0x074), 0x40),
        (Some(0x078), 0x10),
        (Some(0x07c), 0x80),
        (Some(0x080), 0x4),
        (Some(0x084), 0x1),
        (Some(0x088), 0x8),
        (Some(0x08c), 0x2),
        (Some(0x090), 0x800),
        (Some(0x094), 0x100),
        (Some(0x098), 0x1000),
        (Some(0x09c), 0x4000),
        (Some(0x0a0), 0x8000),
        (Some(0x0a4), 0x2000),
        (Some(0x0a8), 0x20),
        (Some(0x0ac), 0x40),
        (Some(0x0b0), 0x10),
        (Some(0x0b4), 0x80),
        (Some(0x0b8), 0x8000004),
        (Some(0x0bc), 0x10000001),
        (Some(0x0c0), 0x2000008),
        (Some(0x0c4), 0x4000002),
        (Some(0x0c8), 0x800),
        (Some(0x0cc), 0x100),
        (Some(0x0d0), 0x10000),
        (Some(0x0d4), 0x20000),
        (None, 0x40000),
        (Some(0x220), 0x80000),
        (None, 0x100000),
        (Some(0x0e4), 0x200000),
        (Some(0x0e8), 0x400000),
        (Some(0x0ec), 0x0),
        (Some(0x0f0), 0x2000000),
        (Some(0x0f4), 0x4000000),
        (Some(0x0f8), 0x8000000),
        (Some(0x0fc), 0x10000000),
        (Some(0x100), 0x1000000),
        (Some(0x38c), 0x0),
        (Some(0x390), 0x0),
        (Some(0x394), 0x0),
        (Some(0x398), 0x0),
        (Some(0x39c), 0x0),
        (Some(0x3a0), 0x0),
        (Some(0x3a4), 0x0),
        (Some(0x3a8), 0x0),
        (Some(0x124), 0x1000),
        (Some(0x128), 0x4000),
        (Some(0x12c), 0x8000),
        (Some(0x130), 0x2000),
        (Some(0x134), 0x20),
        (Some(0x138), 0x40),
        (Some(0x13c), 0x10),
        (Some(0x140), 0x80),
        (Some(0x144), 0x4),
        (Some(0x148), 0x1),
        (Some(0x14c), 0x8),
        (Some(0x150), 0x2),
        (Some(0x154), 0x800),
        (Some(0x158), 0x100),
        (Some(0x15c), 0x10000),
        (Some(0x160), 0x20000),
        (None, 0x40000),
        (Some(0x168), 0x80000),
        (None, 0x100000),
        (Some(0x170), 0x200000),
        (Some(0x174), 0x400000),
        (Some(0x178), 0x0),
        (Some(0x17c), 0x2000000),
        (Some(0x180), 0x4000000),
        (Some(0x184), 0x8000000),
        (Some(0x188), 0x10000000),
    ];

    /// `Input_BuildPadMap(manager)` (`0x006315c0`, thiscall) -> the last
    /// pair's address (`eax` as the original leaves it; 0 when that slot
    /// would be the null address). `frame` see [`PAD_MAP_FRAME`].
    pub fn build_pad_map(h: &mut impl Host, frame: u32, manager: u32) -> u32 {
        let array = manager.wrapping_add(0x9c);
        for k in 0..3 {
            h.write(frame + 4 * k, 0u32);
        }
        h.call_sym(ASSIGN, Conv::Thiscall(array), &[frame]);
        if h.read::<i32>(array.wrapping_add(4)) < 0x4b {
            h.call_sym(RESERVE, Conv::Thiscall(array), &[0x4b]);
        }
        let mut last = 0;
        for (from, bit) in PAIRS {
            let value = match from {
                Some(off) => {
                    let settings: u32 = h.read(h.rebase(SETTINGS));
                    h.read::<u32>(settings.wrapping_add(off))
                }
                None => 0,
            };
            let count = h.read::<i32>(array).wrapping_add(1);
            h.write(array, count);
            if count > h.read::<i32>(array.wrapping_add(4)) {
                let n = count.wrapping_add(2).wrapping_add(count / 8);
                h.call_sym(RESERVE, Conv::Thiscall(array), &[n.cast_unsigned()]);
            }
            let count: u32 = h.read(array);
            let data: u32 = h.read(array.wrapping_add(8));
            last = data.wrapping_add(count.wrapping_mul(8)).wrapping_sub(8);
            if last != 0 {
                h.write(last, value);
                h.write(last.wrapping_add(4), bit);
            }
        }
        last
    }
}
