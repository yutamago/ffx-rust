//! `magic_0093.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::m0032::reset_state;
use crate::{Mem, Sig};

/// 0x10001b80 (internal; `ecx` = the object, two stack words of which
/// only the second is read, cdecl): [`reset_state`] with `+0x114` 0 and
/// the count `+0x11c` 0x12. Returns 0x8000 (the flag word it stores).
pub fn reset_state_12(g: &mut impl Mem, this: u32, arg: u32) -> u32 {
    reset_state(g, this, arg, 0, 0x12);
    0x8000
}

/// [`reset_state_12`] (273 bytes, 1 overlay).
pub const RESET_STATE_12: Sig = Sig {
    len: 273,
    hash: 0x4dac_897b_4810_f785,
    calls: &[],
    imports: &[],
};
