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
