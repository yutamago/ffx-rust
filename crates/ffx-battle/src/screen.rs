//! The battle screens' moves and links (`docs/rewrite-plan.md`, M2).
//!
//! A screen is `0x1c0` bytes from `0x01137000` ([`crate::camera::bridge::SCREENS`],
//! three of them). Each of its two sides (`side & 1`) has a position at
//! `+0x40` + 16 side, a saved one at `+0x80` + 16 side and the move's start
//! at `+0xa0` + 16 side, and four move records of 0x10 bytes at `+0xdc` +
//! 0x40 side, one per lane: `+0` the kind (2 accelerated), `+4` the frame,
//! `+6`, `+8`, `+0xa` the accelerating, cruising and braking frames,
//! `+0xc` the total (0 idle), `+0xe` cleared. Bytes from `+0xc2` link
//! screens.

/// The game's memory, through [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    use crate::camera::bridge::{EE_MOVE, SCREEN_SIZE, SCREENS, ms_camera_get_id};
    use crate::camera::x87_copy;

    /// `g_BtlScreen` (`0x023114c0`): the screen in use.
    pub const BTL_SCREEN: Global<u32> = Global::new(0x0231_14c0);

    /// Screen `i`.
    fn screen(mem: &impl Mem, i: u32) -> u32 {
        SCREENS
            .ptr(mem)
            .addr
            .wrapping_add(i.wrapping_mul(SCREEN_SIZE))
    }

    /// Screen `i`'s side `side`'s first move record.
    fn moves(mem: &impl Mem, i: u32, side: u32) -> u32 {
        screen(mem, i) + 0xdc + 0x40 * side
    }

    /// The side's position into the EE's move register (`0x00c8f588`) and
    /// the move's start (`+0xa0` + 16 side), as the PS2 code's `lq`/`sq`.
    fn mark_start(mem: &mut impl Mem, i: u32, side: u32) {
        let s = screen(mem, i);
        let pos: [u32; 4] = mem.read(s + 0x40 + 0x10 * side);
        EE_MOVE.set(mem, pos);
        mem.write(s + 0xa0 + 0x10 * side, pos);
    }

    /// `MsScreenLinkSet(i, k, v)` (`0x007bbdd0`): byte `+0xc2 + k` of
    /// screen `i` (below 3) = `v`.
    pub fn ms_screen_link_set(mem: &mut impl Mem, i: u32, k: u32, v: u8) {
        if i <= 2 {
            let at = screen(mem, i).wrapping_add(0xc2).wrapping_add(k);
            mem.write(at, v);
        }
    }

    /// `MsScreenOffsetID(id, x, y)` (`0x007bc0e0`): when `id` is its
    /// camera's current request ([`ms_camera_get_id`]), the screen in use
    /// gets the offset (`+0x30`, `+0x34`, `f32`). Returns `eax` as the
    /// original leaves it: `g_BtlScreen`, or the current request.
    pub fn ms_screen_offset_id(mem: &mut impl Mem, id: u32, x: u32, y: u32) -> u32 {
        let current = ms_camera_get_id(mem, id);
        if current != id {
            return current;
        }
        let s = BTL_SCREEN.get(mem);
        mem.write(s.wrapping_add(0x30), x87_copy(x));
        let s = BTL_SCREEN.get(mem);
        mem.write(s.wrapping_add(0x34), x87_copy(y));
        s
    }

    /// `MsScreenMoveCheck(i, side)` (`0x007bbfc0`): how many of the side's
    /// four lanes still move (total not 0); 0 for `i` above 2.
    pub fn ms_screen_move_check(mem: &impl Mem, i: u32, side: u32) -> u32 {
        if i > 2 {
            return 0;
        }
        let m = moves(mem, i, side & 1);
        (0..4)
            .map(|k| u32::from(mem.read::<u16>(m + 0x10 * k + 0xc) != 0))
            .sum()
    }

    /// `MsScreenMoveReset(i, side)` (`0x007bc010`): stops the side's four
    /// lanes (kind and total 0) and puts its saved position (`+0x80`) back
    /// from the position (`+0x40`), lane by lane.
    pub fn ms_screen_move_reset(mem: &mut impl Mem, i: u32, side: u32) {
        if i > 2 {
            return;
        }
        let side = side & 1;
        let s = screen(mem, i);
        let m = moves(mem, i, side);
        for k in 0..4 {
            mem.write(m + 0x10 * k, 0u16);
            mem.write(m + 0x10 * k + 0xc, 0u32);
            let v: u32 = mem.read(s + 0x40 + 0x10 * side + 4 * k);
            mem.write(s + 0x80 + 0x10 * side + 4 * k, x87_copy(v));
        }
    }

    /// `MsScreenMove(i, side, total, kind)` (`0x007bbdf0`): every lane of
    /// the side moves `kind` over `total` frames (the frame kept, `+0xe`
    /// cleared), from the side's position now.
    pub fn ms_screen_move(mem: &mut impl Mem, i: u32, side: u32, total: u16, kind: u16) {
        if i > 2 {
            return;
        }
        let side = side & 1;
        let m = moves(mem, i, side);
        for k in 0..4 {
            mem.write(m + 0x10 * k + 0xc, total);
            mem.write(m + 0x10 * k, kind);
            mem.write(m + 0x10 * k + 0xe, 0u16);
        }
        mark_start(mem, i, side);
    }

    /// `MsScreenMoveAcc(i, side, accel, cruise, brake)` (`0x007bbeb0`):
    /// every lane of the side moves accelerated (kind 2) from frame 0 over
    /// `accel + cruise + brake` frames (`i16` sums), from the side's
    /// position now.
    pub fn ms_screen_move_acc(
        mem: &mut impl Mem,
        i: u32,
        side: u32,
        accel: u16,
        cruise: u16,
        brake: u16,
    ) {
        if i > 2 {
            return;
        }
        let side = side & 1;
        let m = moves(mem, i, side);
        let total = brake.wrapping_add(cruise).wrapping_add(accel);
        for k in 0..4 {
            let r = m + 0x10 * k;
            mem.write(r, 2u16);
            mem.write(r + 4, 0u16);
            mem.write(r + 6, accel);
            mem.write(r + 8, cruise);
            mem.write(r + 0xa, brake);
            mem.write(r + 0xc, total);
            mem.write(r + 0xe, 0u16);
        }
        mark_start(mem, i, side);
    }
}
