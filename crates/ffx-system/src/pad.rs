//! The pad words the `TO` menus read (`TOMenuGetControlPad` and friends)
//! and the dummy analog sticks.

/// The pad word `TOMenuGetControlPad`/`Trg`/`Rep` return: the first set
/// when the dummy controller is on (`TkMenuGetDummyControlerFlag` = 1) or
/// its held word has a bit of `0xf000`, else the second.
#[must_use]
pub const fn pick(dummy_flag: u32, first_held: u16, first: u16, second: u16) -> u16 {
    if dummy_flag == 1 || first_held & 0xf000 != 0 {
        first
    } else {
        second
    }
}

/// `TODebugAddPadPrintFlag`: the next of the 4 pad-print modes (the
/// byte plus one, signed, remainder by 4 toward zero, as the original
/// computes it).
#[must_use]
pub const fn next_print_flag(v: u8) -> u8 {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // the low byte
    let r = (v.wrapping_add(1).cast_signed() as i32 % 4) as u8;
    r
}

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    /// The first pad words: held, triggered, repeated.
    pub const FIRST: Global<[u16; 3]> = Global::new(0x025d_09d2);
    /// `TOMenuGetControlPad2`'s word.
    pub const PAD2: Global<u16> = Global::new(0x025d_09d8);
    /// The second pad words: held, triggered, repeated.
    pub const SECOND: Global<[u16; 3]> = Global::new(0x025d_09e2);
    /// `SetTOPadGetCtrl`'s byte.
    pub const GET_CTRL: Global<u8> = Global::new(0x025d_09c0);
    /// The dummy analog stick bytes `TOMenuGetDummyAnalogControler` returns
    /// for axes 2, 3, 0, 1 (in address order).
    pub const DUMMY_ANALOG: Global<[u8; 8]> = Global::new(0x0186_a428);
    /// `TODebugAddPadPrintFlag`'s mode.
    pub const PRINT_FLAG: Global<u8> = Global::new(0x0184_1d43);

    /// `TOMenuGetControlPad()` (`0x008be410`, `which` 0), `...Trg`
    /// (`0x008be4b0`, 1) and `...Rep` (`0x008be470`, 2), given
    /// `TkMenuGetDummyControlerFlag()`.
    pub fn control_pad(mem: &impl Mem, which: usize, dummy_flag: u32) -> u16 {
        let first = FIRST.get(mem);
        super::pick(dummy_flag, first[0], first[which], SECOND.get(mem)[which])
    }

    /// `TOMenuGetControlPad2()` (`0x008be440`).
    pub fn control_pad2(mem: &impl Mem) -> u16 {
        PAD2.get(mem)
    }

    /// `TOMenuGetDummyAnalogControler(axis)` (`0x008ca320`): 0x80 (centred)
    /// past axis 3.
    pub fn dummy_analog(mem: &impl Mem, axis: u32) -> u8 {
        let a = DUMMY_ANALOG.get(mem);
        match axis {
            0 => a[4],
            1 => a[6],
            2 => a[0],
            3 => a[2],
            _ => 0x80,
        }
    }

    /// `SetTOPadGetCtrl(v)` (`0x008ab580`).
    pub fn set_get_ctrl(mem: &mut impl Mem, v: u8) {
        GET_CTRL.set(mem, v);
    }

    /// `TODebugAddPadPrintFlag()` (`0x008abd00`).
    pub fn add_print_flag(mem: &mut impl Mem) {
        let v = super::next_print_flag(PRINT_FLAG.get(mem));
        PRINT_FLAG.set(mem, v);
    }

    /// `0x00689a10(base, n)`: `n` touch records of 0x150 bytes from `base`
    /// reset (a null record skipped): `+0` u16 0, `+4` -1, `+8` 0, 32
    /// ten-byte points from `+0xc` (byte, then words at `+2` and `+6`)
    /// zeroed, `+0x14c` 0. Callers are the touch manager's (near
    /// `TouchManager_GetInstance`).
    pub fn touch_records_init(mem: &mut impl Mem, base: u32, n: u32) {
        let mut rec = base;
        for _ in 0..n {
            if rec != 0 {
                mem.write(rec, 0u16);
                mem.write(rec.wrapping_add(4), u32::MAX);
                mem.write(rec.wrapping_add(8), 0u32);
                for k in 0..32u32 {
                    let p = rec.wrapping_add(0xc + 10 * k);
                    mem.write(p, 0u8);
                    mem.write(p.wrapping_add(2), 0u32);
                    mem.write(p.wrapping_add(6), 0u32);
                }
                mem.write(rec.wrapping_add(0x14c), 0u32);
            }
            rec = rec.wrapping_add(0x150);
        }
    }

    /// `0x006898d0(base, n)`: `n` touch tracks of 0x1c8 bytes from `base`
    /// reset (a null one skipped): `+0` u16, eight 0x38-byte points from
    /// `+4` (bytes `+0`, `+0x14` and the floats `+4..+0x14`) and `+0x1c4`
    /// zeroed. Returns the last track's address (what `eax` ends as; for
    /// `n` 0 the original leaves the caller's).
    pub fn touch_tracks_init(mem: &mut impl Mem, base: u32, n: u32) -> u32 {
        let mut rec = base;
        let mut last = 0;
        for _ in 0..n {
            last = rec;
            if rec != 0 {
                mem.write(rec, 0u16);
                for k in 0..8u32 {
                    let p = rec.wrapping_add(4 + 0x38 * k);
                    mem.write(p, 0u8);
                    mem.write(p.wrapping_add(0x14), 0u8);
                    mem.write(p.wrapping_add(4), [0u32; 4]);
                }
                mem.write(rec.wrapping_add(0x1c4), 0u32);
            }
            rec = rec.wrapping_add(0x1c8);
        }
        last
    }

    /// `Input_KeyHeld(binding, device)` (`0x00629d50`, `thiscall` on the
    /// input devices, `ret 8`): for a device below 7 with a state block
    /// (`*(this + 0x2b8) + 0x80 + 4 * device`) and a binding in `28..284`,
    /// whether its key byte (`block + 0x18 + binding - 28`) is set.
    pub fn key_held(mem: &impl Mem, this: u32, binding: u32, device: u32) -> bool {
        let devices: u32 = mem.read(this.wrapping_add(0x2b8));
        if device >= 7 {
            return false;
        }
        let block: u32 = mem.read(devices.wrapping_add(0x80 + 4 * device));
        if block == 0 {
            return false;
        }
        let key = binding.wrapping_sub(0x1c);
        key < 0x100 && mem.read::<u8>(block.wrapping_add(0x18).wrapping_add(key)) != 0
    }

    /// `Virtuos::InputManager::isHoldButton(mask)` (`0x00631340`,
    /// `thiscall`, `ret 4`): any of `mask` down now (`+0`).
    pub fn is_hold_button(mem: &impl Mem, this: u32, mask: u32) -> u32 {
        u32::from(mem.read::<u32>(this) & mask != 0)
    }

    /// `Virtuos::InputManager::isPressButton(mask)` (`0x00631360`): none of
    /// `mask` down before (`+4`) and any down now.
    pub fn is_press_button(mem: &impl Mem, this: u32, mask: u32) -> bool {
        mem.read::<u32>(this.wrapping_add(4)) & mask == 0 && mem.read::<u32>(this) & mask != 0
    }

    /// `0x00631380(mask)`: any of `mask` down before and none now (the
    /// release counterpart of [`is_press_button`]).
    pub fn is_release_button(mem: &impl Mem, this: u32, mask: u32) -> bool {
        mem.read::<u32>(this.wrapping_add(4)) & mask != 0 && mem.read::<u32>(this) & mask == 0
    }

    /// `Virtuos::InputManager::isRepButton(mask)` (`0x006313a0`, `thiscall`,
    /// `ret 4`): a fresh press (none of `mask` before, any now) is true;
    /// otherwise the first of the 29 buttons in `mask` whose held time (f32
    /// at `+8 + 4 i`) is past the double at `0x00b41380` (0.2333) restarts
    /// at the float `0x00b41378` (0.1333) and counts as a repeat.
    pub fn is_rep_button(mem: &mut impl Mem, this: u32, mask: u32) -> bool {
        if mem.read::<u32>(this.wrapping_add(4)) & mask == 0 && mem.read::<u32>(this) & mask != 0 {
            return true;
        }
        let limit = Global::<f64>::new(0x00b4_1380).get(mem);
        for i in 0..0x1d {
            if mask & (1 << i) == 0 {
                continue;
            }
            let at = this.wrapping_add(8 + 4 * i);
            if f64::from(mem.read::<f32>(at)) > limit {
                let restart = Global::<f32>::new(0x00b4_1378).get(mem);
                mem.write(at, ffx_bridge::x87::round_trip(restart));
                return true;
            }
        }
        false
    }

    /// `0x00689510()` (`thiscall`): one touch record reset (as
    /// [`touch_records_init`] does each); returns it.
    pub fn touch_record_reset(mem: &mut impl Mem, rec: u32) -> u32 {
        mem.write(rec, 0u16);
        mem.write(rec.wrapping_add(4), u32::MAX);
        mem.write(rec.wrapping_add(8), 0u32);
        for k in 0..32u32 {
            let p = rec.wrapping_add(0xc + 10 * k);
            mem.write(p, 0u8);
            mem.write(p.wrapping_add(2), 0u32);
            mem.write(p.wrapping_add(6), 0u32);
        }
        mem.write(rec.wrapping_add(0x14c), 0u32);
        rec
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_flag_cycles_and_keeps_the_sign() {
        assert_eq!(next_print_flag(0), 1);
        assert_eq!(next_print_flag(3), 0);
        // 0x7f + 1 = -128: -128 % 4 = 0; 0xfe + 1 = -1: -1 % 4 = -1
        assert_eq!(next_print_flag(0x7f), 0);
        assert_eq!(next_print_flag(0xfe), 0xff);
    }
}
