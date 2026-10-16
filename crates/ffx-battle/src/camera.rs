//! The battle camera's lookups (`docs/rewrite-plan.md`, M2).
//!
//! Each camera (`g_Cameras`, 0xda0 bytes apiece) has four request slots of
//! 8 bytes at `+0xc68` (`0x011381a8` for the first): a `u32` request ID
//! whose byte 2 is the slot's level, an `i8` level bonus at `+6`, and two
//! flags at `+4` and `+5` that take the slot out of the running.
//! `g_Camera` (`0x02311430`) points at the camera in use: two sides of
//! 0x61c bytes (0 the look-at, 1 the eye), positions at `+0x50`, the screen
//! depth at `+0xc9c`.

/// Request slots per camera.
pub const SLOTS: usize = 4;

/// A camera's request slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Slot {
    /// `+0`: the request ID; byte 2 the level.
    pub id: u32,
    /// `+4`: set, the slot does not count.
    pub off: u8,
    /// `+5`: set, the slot does not count (or is done).
    pub done: u8,
    /// `+6`: added to the level.
    pub bonus: i8,
}

impl Slot {
    /// From the slot's first seven bytes.
    #[must_use]
    pub const fn from_bytes(b: [u8; 7]) -> Self {
        Self {
            id: u32::from_le_bytes([b[0], b[1], b[2], b[3]]),
            off: b[4],
            done: b[5],
            bonus: b[6].cast_signed(),
        }
    }

    /// Its level with the bonus.
    #[must_use]
    pub fn level(self) -> i32 {
        i32::from(self.id.to_le_bytes()[2]) + i32::from(self.bonus)
    }
}

/// `MsCameraGetNum`: the live slot with the highest level (above 0; the
/// first of equals), 0 when none is.
#[must_use]
pub fn current(slots: [Slot; SLOTS]) -> u32 {
    let (mut best, mut at) = (0, 0);
    for (i, s) in (0u32..).zip(slots) {
        let l = s.level();
        if l > best && s.off == 0 && s.done == 0 {
            best = l;
            at = i;
        }
    }
    at
}

/// `MsCameraIDcheck(id)`: with a level in `id` (byte 2), -1 when a slot
/// that is not done has exactly `id`; without, 0 when some slot with a
/// level is not done, else -1.
#[must_use]
pub fn id_check(slots: [Slot; SLOTS], id: u32) -> i32 {
    if id & 0xff_0000 == 0 {
        if slots
            .iter()
            .any(|s| (s.id >> 16) & 0xff != 0 && s.done == 0)
        {
            0
        } else {
            -1
        }
    } else if slots.iter().any(|s| s.id == id && s.done == 0) {
        -1
    } else {
        0
    }
}

/// A float as the x87 copies it (`flds`/`fstps`): a signalling NaN comes
/// out quiet.
#[must_use]
pub const fn x87_copy(bits: u32) -> u32 {
    if bits & 0x7f80_0000 == 0x7f80_0000 && bits & 0x007f_ffff != 0 {
        bits | 0x0040_0000
    } else {
        bits
    }
}

/// The cameras where the game keeps them, through [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    use super::{SLOTS, Slot, current, id_check, x87_copy};

    /// The first camera's request slots.
    pub const SLOTS_AT: Global<u8> = Global::new(0x0113_81a8);
    /// Bytes per camera.
    pub const CAMERA_SIZE: u32 = 0xda0;
    /// `g_Camera`: the camera in use.
    pub const CAMERA: Global<u32> = Global::new(0x0231_1430);
    /// The camera matrix (`MsGetCameraMatrix`).
    pub const CAMERA_MATRIX: Global<u8> = Global::new(0x0231_1440);
    /// The screen matrix (`MsGetScreenMatrix`).
    pub const SCREEN_MATRIX: Global<u8> = Global::new(0x0231_1480);
    /// Not 0: `MsCameraPriority` changes nothing (`MsCameraPriorityMask`).
    pub const PRIORITY_MASK: Global<u32> = Global::new(0x0231_1424);
    /// The three screens (`MsScreenResetShake`), 0x1c0 bytes each, right
    /// before `g_Cameras`: shake offsets `+0x30`/`+0x34` (`f32`), shake
    /// flags `+0x16c` and `+0x190`.
    pub const SCREENS: Global<u8> = Global::new(0x0113_7000);
    /// Bytes per screen.
    pub const SCREEN_SIZE: u32 = 0x1c0;

    fn slots(mem: &impl Mem, cam: u32) -> [Slot; SLOTS] {
        let base = SLOTS_AT.ptr(mem).addr + (cam & 0xff) * CAMERA_SIZE;
        #[allow(clippy::cast_possible_truncation)] // 0..4
        std::array::from_fn(|i| Slot::from_bytes(mem.read(base + 8 * i as u32)))
    }

    /// `MsCameraGetNum(cam)` (`0x007bd340`): camera `cam & 0xff`'s
    /// current slot.
    pub fn ms_camera_get_num(mem: &impl Mem, cam: u32) -> u32 {
        current(slots(mem, cam))
    }

    /// `MsCameraIDcheck(id)` (`0x007bd560`): camera `id & 0xff`, see
    /// [`id_check`].
    pub fn ms_camera_id_check(mem: &impl Mem, id: u32) -> i32 {
        id_check(slots(mem, id), id)
    }

    /// `MsCameraGetPosNow(side, out)` (`0x007bd470`): side `side & 1`'s
    /// position (3 floats) into `out` when not null.
    pub fn ms_camera_get_pos_now(mem: &mut impl Mem, side: u32, out: u32) {
        let at = CAMERA
            .get(mem)
            .wrapping_add((side & 1) * 0x61c)
            .wrapping_add(0x50);
        if out != 0 {
            for i in 0..3 {
                let v: u32 = mem.read(at.wrapping_add(4 * i));
                mem.write(out.wrapping_add(4 * i), x87_copy(v));
            }
        }
    }

    /// `MsCameraGetScrDpt` (`0x007bd510`): the screen depth (returned on
    /// the x87 stack).
    pub fn ms_camera_get_scr_dpt(mem: &impl Mem) -> f32 {
        mem.read(CAMERA.get(mem).wrapping_add(0xc9c))
    }

    /// `MsCameraPriority(id, bonus)` (`0x007be0a0`): unless masked, every
    /// slot of camera `id & 0xff` with exactly request `id` gets level
    /// bonus `bonus`.
    pub fn ms_camera_priority(mem: &mut impl Mem, id: u32, bonus: u8) {
        if PRIORITY_MASK.get(mem) != 0 {
            return;
        }
        let base = SLOTS_AT.ptr(mem).addr + (id & 0xff) * CAMERA_SIZE;
        for i in 0u32..4 {
            let at = base + 8 * i;
            if mem.read::<u32>(at) == id {
                mem.write(at + 6, bonus);
            }
        }
    }

    /// `MsCameraPriorityMask(on)` (`0x007be100`).
    pub fn ms_camera_priority_mask(mem: &mut impl Mem, on: u32) {
        PRIORITY_MASK.set(mem, on);
    }

    /// `MsScreenResetShake(screen)` (`0x007c56c0`): stops screen `screen`'s
    /// shake (below 3, unsigned).
    pub fn ms_screen_reset_shake(mem: &mut impl Mem, screen: u32) {
        if screen > 2 {
            return;
        }
        let s = SCREENS.ptr(mem).addr + screen * SCREEN_SIZE;
        mem.write(s + 0x16c, 0u8);
        mem.write(s + 0x30, 0.0f32);
        mem.write(s + 0x190, 0u8);
        mem.write(s + 0x34, 0.0f32);
    }

    /// `MsGetScreenMatrix` (`0x007bbd70`).
    pub fn ms_get_screen_matrix(mem: &impl Mem) -> u32 {
        SCREEN_MATRIX.ptr(mem).addr
    }

    /// `MsGetCameraMatrix` (`0x007c04b0`).
    pub fn ms_get_camera_matrix(mem: &impl Mem) -> u32 {
        CAMERA_MATRIX.ptr(mem).addr
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highest_live_level_wins() {
        let s = |level: u32, bonus: i8, done: u8| Slot {
            id: level << 16 | 7,
            off: 0,
            done,
            bonus,
        };
        assert_eq!(current([s(1, 0, 0), s(3, 0, 0), s(3, 0, 0), s(2, 1, 0)]), 1);
        assert_eq!(
            current([s(0, 0, 0), s(1, -1, 0), s(5, 0, 1), s(0, 0, 0)]),
            0
        );
        assert_eq!(
            id_check([s(0, 0, 0), s(2, 0, 1), s(0, 0, 0), s(0, 0, 0)], 0),
            -1
        );
        assert_eq!(
            id_check([s(0, 0, 0), s(2, 0, 0), s(0, 0, 0), s(0, 0, 0)], 0),
            0
        );
        assert_eq!(x87_copy(0x7f80_0001), 0x7fc0_0001);
        assert_eq!(x87_copy(0x7f80_0000), 0x7f80_0000);
    }
}
