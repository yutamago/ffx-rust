//! M5 stage 1, list `vfx`, batch 5: code the Ghidra dump has no function
//! for: the bind-flag setters on `g_EtBindDefault` that the Atel natives
//! 0x0245..0x0249 call, the effect debug's flag clear, and five lone `jmp`
//! thunks (`op_gs1_init`, `opus_change_chrhd_ffx_to_battle`,
//! `op_oef2_tex0_get_007e5260`, `op_et_bindeff_draw_off`/`_on`) that hand
//! their caller's words on and return the target's `eax`.

use ffx_bridge::Global;

/// `g_EtBindDefault`: the u16 bind flags.
pub const BIND_DEFAULT: Global<u16> = Global::new(0x011a_0080);
/// The byte after `g_OpDebugMode` (`0x02322791`) the debug reset clears.
pub const OP_DEBUG_BYTE: Global<u8> = Global::new(0x0232_2791);

/// The thunks: `(address, target, words the target reads)`.
pub const THUNKS: [(u32, u32, usize); 5] = [
    (0x007e_b570, 0x007e_b760, 2), // op_gs1_init(work, flags)
    (0x007f_c290, 0x007f_e200, 0), // opus_change_chrhd_ffx_to_battle()
    (0x0080_8a90, 0x007e_5260, 3), // op_oef2_tex0_get_007e5260(oef, no, out)
    (0x0092_d860, 0x007f_ba40, 1), // op_et_bindeff_draw_off(h)
    (0x0092_d870, 0x007f_bab0, 1), // op_et_bindeff_draw_on(h)
];

/// The target of the thunk at `at`, 0 for none.
#[must_use]
pub const fn target(at: u32) -> u32 {
    let mut i = 0;
    while i < THUNKS.len() {
        if THUNKS[i].0 == at {
            return THUNKS[i].1;
        }
        i += 1;
    }
    0
}

pub mod bridge {
    use super::{BIND_DEFAULT, OP_DEBUG_BYTE};
    use ffx_graphic::host::{Conv, Host};

    /// The bind-flag setters: `g_EtBindDefault |= mask` (`set`) or
    /// `&= mask` -> `mask` (the original's `eax`).
    pub fn bind_flags(h: &mut impl Host, mask: u16, set: bool) -> u32 {
        let v = BIND_DEFAULT.get(h);
        BIND_DEFAULT.set(h, if set { v | mask } else { v & mask });
        u32::from(mask)
    }

    /// `op_debug_reset_008005f0()` -> 0: the byte at `0x02322791` cleared.
    pub fn op_debug_reset(h: &mut impl Host) -> u32 {
        OP_DEBUG_BYTE.set(h, 0);
        0
    }

    /// A thunk to `target` (an `ffx.sym` address): the target called with
    /// the caller's words; its `eax`.
    pub fn jmp_thunk(h: &mut impl Host, target: u32, args: &[u32]) -> u32 {
        h.call_sym(target, Conv::Cdecl, args)
    }
}
