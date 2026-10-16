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

/// `MsGetNowSelectedCamraLevel`: the highest level (with its bonus) among
/// the live slots, 0 when none is above 0.
#[must_use]
pub fn best_level(slots: [Slot; SLOTS]) -> i32 {
    let mut best = 0;
    for s in slots {
        let l = s.level();
        if l > best && s.off == 0 && s.done == 0 {
            best = l;
        }
    }
    best
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

    use super::{SLOTS, Slot, best_level, current, id_check, x87_copy};

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

    /// `MsCameraGetID(cam)` (`0x007bd300`): the request ID of camera
    /// `cam & 0xff`'s current slot ([`ms_camera_get_num`]), 0 when that slot
    /// is off or done.
    pub fn ms_camera_get_id(mem: &impl Mem, cam: u32) -> u32 {
        let cam = cam & 0xff;
        let n = ms_camera_get_num(mem, cam);
        let at = SLOTS_AT
            .ptr(mem)
            .addr
            .wrapping_add(cam.wrapping_mul(CAMERA_SIZE))
            .wrapping_add(n.wrapping_mul(8));
        let b: [u8; 6] = mem.read(at);
        if b[4] != 0 || b[5] != 0 {
            0
        } else {
            u32::from_le_bytes([b[0], b[1], b[2], b[3]])
        }
    }
    /// The camera `g_Cameras` puts first (`0x00c4423c`, a byte):
    /// `MsGetNowSelectedCamraLevel` looks at its slots.
    pub const SELECTED: Global<u8> = Global::new(0x00c4_423c);

    /// `MsGetNowSelectedCamraLevel` (`0x007bbc30`): [`best_level`] of the
    /// camera [`SELECTED`].
    pub fn ms_get_now_selected_camra_level(mem: &impl Mem) -> i32 {
        best_level(slots(mem, u32::from(SELECTED.get(mem))))
    }

    /// `MsCameraFree(id)` (`0x007bd000`): clears (ID, flags, bonus) every
    /// slot of camera `id & 0xff` whose ID equals `id` but for bits 20-23;
    /// how many it cleared.
    pub fn ms_camera_free(mem: &mut impl Mem, id: u32) -> u32 {
        let base = SLOTS_AT.ptr(mem).addr + (id & 0xff) * CAMERA_SIZE;
        let key = id & 0xff0f_ffff;
        let mut n = 0;
        for i in 0u32..4 {
            let at = base + 8 * i;
            if mem.read::<u32>(at) & 0xff0f_ffff == key {
                mem.write(at, 0u32);
                mem.write(at + 4, 0u16);
                mem.write(at + 6, 0u8);
                n += 1;
            }
        }
        n
    }

    /// The override camera (`MsEffectCameraSet`): screen depth, roll, eye,
    /// look-at and up (vec4 each), and whether it is on.
    pub const OVERRIDE_DEPTH: Global<u32> = Global::new(0x0231_1428);
    pub const OVERRIDE_ROLL: Global<u32> = Global::new(0x0113_6fc4);
    pub const OVERRIDE_EYE: Global<[u32; 4]> = Global::new(0x0113_6fc8);
    pub const OVERRIDE_LOOK_AT: Global<[u32; 4]> = Global::new(0x0113_6fd8);
    pub const OVERRIDE_UP: Global<[u32; 4]> = Global::new(0x0113_6fe8);
    pub const OVERRIDE_ON: Global<u32> = Global::new(0x0231_142c);
    /// The default screen depth (1024.0, `0x00b55178`) and -1.0
    /// (`0x00b92340`), what `MsEffectCameraInit` resets to.
    pub const DEFAULT_DEPTH: Global<u32> = Global::new(0x00b5_5178);
    pub const MINUS_ONE: Global<u32> = Global::new(0x00b9_2340);
    /// The emulated EE register `r0` (`g_Ee_Gpr`, always zero): the
    /// override camera's vectors reset to it.
    pub const EE_R0: Global<[u32; 4]> = Global::new(0x00c8_f508);
    /// The EE register the PS2 code moved `MsEffectCameraSet`'s vectors
    /// through (`0x00c8f588`).
    pub const EE_MOVE: Global<[u32; 4]> = Global::new(0x00c8_f588);

    /// `MsEffectCameraInit` (`0x007bba70`): override off, depth 1024, roll
    /// 0, eye and look-at `r0`, up `r0` with y -1.0; EAX `r0`'s w.
    pub fn ms_effect_camera_init(mem: &mut impl Mem) -> u32 {
        let r0 = EE_R0.get(mem);
        let depth = x87_copy(DEFAULT_DEPTH.get(mem));
        OVERRIDE_DEPTH.set(mem, depth);
        OVERRIDE_ROLL.set(mem, 0);
        OVERRIDE_LOOK_AT.set(mem, r0);
        OVERRIDE_EYE.set(mem, r0);
        OVERRIDE_UP.set(mem, [r0[0], x87_copy(MINUS_ONE.get(mem)), r0[2], r0[3]]);
        OVERRIDE_ON.set(mem, 0);
        r0[3]
    }

    /// `MsEffectCameraSet(look_at, eye, depth, roll)` (`0x007bbb10`):
    /// override on with these (null vectors kept); EAX `eye`'s w, or null.
    pub fn ms_effect_camera_set(
        mem: &mut impl Mem,
        look_at: u32,
        eye: u32,
        depth: u32,
        roll: u32,
    ) -> u32 {
        OVERRIDE_ON.set(mem, 1);
        if look_at != 0 {
            let v: [u32; 4] = mem.read(look_at);
            EE_MOVE.set(mem, v);
            OVERRIDE_LOOK_AT.set(mem, v);
        }
        let mut eax = eye;
        if eye != 0 {
            let v: [u32; 4] = mem.read(eye);
            EE_MOVE.set(mem, v);
            OVERRIDE_EYE.set(mem, v);
            eax = v[3];
        }
        OVERRIDE_DEPTH.set(mem, x87_copy(depth));
        OVERRIDE_ROLL.set(mem, x87_copy(roll));
        eax
    }

    /// `MsSetScreen(i, a, b, c, d, e, near, far, z0, z1)` (`0x007bc7f0`):
    /// screen `i` (signed, below 3): `+0x1c` = a, `+0x10` b, `+0x14` c,
    /// `+0x20` d, `+0x24` e, `+0` near, `+4` far, `+8` z0, `+0xc` z1, and
    /// the depth mapping `(far − near)·(z0·z1) / (z1 − z0)` at `+0x18`,
    /// `(near·z1 − far·z0) / (z1 − z0)` at `+0x28` (x87 at 53 bits, each
    /// stored as `f32`).
    #[allow(clippy::many_single_char_names)]
    pub fn ms_set_screen(mem: &mut impl Mem, i: i32, f: [u32; 9]) {
        use ffx_bridge::x87::{div, load, mul, store, sub};
        if i >= 3 {
            return;
        }
        let s = SCREENS
            .ptr(mem)
            .addr
            .wrapping_add(i.cast_unsigned().wrapping_mul(SCREEN_SIZE));
        let [a, b, c, d, e, near, far, z0, z1] = f;
        let (n, fa, za, zb) = (load(near), load(far), load(z0), load(z1));
        let dz = sub(zb, za);
        let k = div(mul(sub(fa, n), mul(za, zb)), dz);
        let o = div(sub(mul(n, zb), mul(fa, za)), dz);
        for (off, v) in [
            (0x1c, x87_copy(a)),
            (0x10, x87_copy(b)),
            (0x14, x87_copy(c)),
            (0x18, store(k)),
            (0x20, x87_copy(d)),
            (0x24, x87_copy(e)),
            (0x28, store(o)),
            (0, x87_copy(near)),
            (4, x87_copy(far)),
            (8, x87_copy(z0)),
            (0xc, x87_copy(z1)),
        ] {
            mem.write(s.wrapping_add(off), v);
        }
    }
    /// `g_Cameras` (`0x01137540`), [`CAMERA_SIZE`] bytes each.
    pub const CAMERAS: Global<u8> = Global::new(0x0113_7540);
    /// The camera `MsDebugGetCameraMode` looks at (a byte of the first
    /// screen, `0x011370c1`).
    pub const DEBUG_CAMERA: Global<u8> = Global::new(0x0113_70c1);

    fn camera_base(mem: &impl Mem, cam: u32) -> u32 {
        CAMERAS.ptr(mem).addr + (cam & 0xff) * CAMERA_SIZE
    }

    /// `MsDebugGetCameraMode(side)` (`0x007cfb20`): the mode byte (`+0xf2`
    /// of side `side`, 0x61c bytes each, channel the current slot, 0x130
    /// each) of camera [`DEBUG_CAMERA`].
    pub fn ms_debug_get_camera_mode(mem: &impl Mem, side: i32) -> u32 {
        let cam = u32::from(DEBUG_CAMERA.get(mem));
        let n = ms_camera_get_num(mem, cam);
        let at = camera_base(mem, cam)
            .wrapping_add(0xf2)
            .wrapping_add(n.wrapping_mul(0x130))
            .wrapping_add(side.cast_unsigned().wrapping_mul(0x61c));
        u32::from(mem.read::<u8>(at))
    }

    /// `MsCopyCameraMatrix(out)` (`0x007c0070`): [`CAMERA_MATRIX`] into
    /// `out` (`FFXVu0CopyMatrix`).
    pub fn ms_copy_camera_matrix(mem: &mut impl Mem, out: u32) -> u32 {
        let src = CAMERA_MATRIX.ptr(mem).addr;
        ffx_ps2::vu0::bridge::matrix_copy(mem, out, src)
    }

    /// `MsCopyScreenMatrix(out)` (`0x007bba50`): [`SCREEN_MATRIX`] into
    /// `out`.
    pub fn ms_copy_screen_matrix(mem: &mut impl Mem, out: u32) -> u32 {
        let src = SCREEN_MATRIX.ptr(mem).addr;
        ffx_ps2::vu0::bridge::matrix_copy(mem, out, src)
    }

    /// A camera parameter by request (`MsCameraSetRoll`, `MsCameraSetScrDpt`):
    /// for a request `id` some live slot holds ([`ms_camera_id_check`]),
    /// channel `id >> 8 & 0xff` of camera `id & 0xff` takes `v` (at
    /// `chan`, 0x30 bytes per channel; its `+0x713` flag, 0x130 per
    /// channel, cleared); when `id` is the current slot's request, `v` is
    /// the camera's target (`target`) too, and with no move running
    /// (`u16` at `lock`) its value now (`now`) and the channel's second
    /// copy (`chan2`).
    #[allow(clippy::too_many_arguments)]
    fn set_param(
        mem: &mut impl Mem,
        id: u32,
        v: u32,
        chan: u32,
        target: u32,
        lock: u32,
        now: u32,
        chan2: u32,
    ) {
        if ms_camera_id_check(mem, id) == 0 {
            return;
        }
        let sub = (id.cast_signed() >> 8).cast_unsigned() & 0xff;
        let cam = id & 0xff;
        let base = camera_base(mem, cam);
        let v = x87_copy(v);
        mem.write(base + 0x713 + sub * 0x130, 0u8);
        mem.write(base + chan + sub * 0x30, v);
        let n = ms_camera_get_num(mem, cam);
        let slot = SLOTS_AT.ptr(mem).addr + cam * CAMERA_SIZE + 8 * n;
        let b: [u8; 6] = mem.read(slot);
        let cur = if b[4] == 0 && b[5] == 0 {
            u32::from_le_bytes([b[0], b[1], b[2], b[3]])
        } else {
            0
        };
        if cur != id {
            return;
        }
        mem.write(base + target, v);
        if mem.read::<u16>(base + lock) == 0 {
            mem.write(base + now, v);
            mem.write(base + chan2 + sub * 0x30, v);
        }
    }

    /// `MsCameraSetRoll(id, roll)` (`0x007bfa80`).
    pub fn ms_camera_set_roll(mem: &mut impl Mem, id: u32, roll: u32) {
        set_param(mem, id, roll, 0xcb8, 0xc94, 0xd84, 0xc90, 0xcd4);
    }

    /// `MsCameraSetScrDpt(id, depth)` (`0x007bfbd0`).
    pub fn ms_camera_set_scr_dpt(mem: &mut impl Mem, id: u32, depth: u32) {
        set_param(mem, id, depth, 0xcc0, 0xca0, 0xd94, 0xc9c, 0xcd8);
    }
    /// The camera matrices' source (`MsSetCameraMatrix`): on, the roll,
    /// and two vectors (the look-at and the eye).
    pub const MATRIX_ON: Global<u32> = Global::new(0x0231_14c4);
    pub const MATRIX_ROLL: Global<u32> = Global::new(0x0113_9e20);
    pub const MATRIX_LOOK_AT: Global<[u32; 4]> = Global::new(0x0113_9e24);
    pub const MATRIX_EYE: Global<[u32; 4]> = Global::new(0x0113_9e34);

    /// `MsSetCameraMatrix(on, screen, camera, look_at, eye, roll)`
    /// (`0x007c06d0`): with `on` (bit 0), the screen and camera matrices
    /// from `screen` and `camera` and the vectors and roll kept; EAX `eye`'s
    /// w, else 0.
    pub fn ms_set_camera_matrix(
        mem: &mut impl Mem,
        on: u32,
        screen: u32,
        camera: u32,
        look_at: u32,
        eye: u32,
        roll: u32,
    ) -> u32 {
        let on = on & 1;
        MATRIX_ON.set(mem, on);
        if on == 0 {
            return 0;
        }
        let s = SCREEN_MATRIX.ptr(mem).addr;
        ffx_ps2::vu0::bridge::matrix_copy(mem, s, screen);
        let c = CAMERA_MATRIX.ptr(mem).addr;
        ffx_ps2::vu0::bridge::matrix_copy(mem, c, camera);
        MATRIX_ROLL.set(mem, x87_copy(roll));
        let v: [u32; 4] = mem.read(look_at);
        EE_MOVE.set(mem, v);
        MATRIX_LOOK_AT.set(mem, v);
        let v: [u32; 4] = mem.read(eye);
        EE_MOVE.set(mem, v);
        MATRIX_EYE.set(mem, v);
        v[3]
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
