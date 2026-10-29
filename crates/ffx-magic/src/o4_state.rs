//! The effect objects' state block reset ([`crate::m0032::reset_state`])
//! in the copies of the own work list (round 4): internal functions
//! (`ecx` = the block), the same code with other constants for `+0x114`
//! and `+0x11c`, read from each copy's `mov dword [ecx + d], imm32`.
//!
//! Three layouts:
//! - [`reset_state_arg`] (273 bytes, `magic_0182` 0x10001a50; 9
//!   copies): a frame, `+0x100` = the second stack word, the two
//!   constants at `+0x12` and `+0x1c`;
//! - [`reset_state_bare`] (270 bytes, `magic_0245` 0x100023f0; 5
//!   copies): no frame and no argument (`+0x100` = 0), the constants at
//!   `+0x06` and `+0x10`;
//! - [`reset_state_edx`] (269 bytes, `magic_0108` 0x10001af0; 3 copies):
//!   `+0x114` = `edx`, `+0x100` = the second stack word, `+0x11c` at
//!   `+0x18` (`magic_0074`'s shape with another count).
//!
//! Each leaves `eax` = 0x8000 (the halfword it stores); the callers seen
//! do not read it.

use crate::m0032::reset_state;
use crate::{Image, Mem, Sig};

/// The 273-byte reset at `f`: `this` = `ecx`, `arg` = the second stack
/// word.
pub fn reset_state_arg(g: &mut impl Mem, img: &Image, f: u32, this: u32, arg: u32) {
    reset_state(g, this, arg, img.word(f + 0x12), img.word(f + 0x1c));
}

/// The 270-byte reset at `f`: `this` = `ecx`, no argument.
pub fn reset_state_bare(g: &mut impl Mem, img: &Image, f: u32, this: u32) {
    reset_state(g, this, 0, img.word(f + 0x06), img.word(f + 0x10));
}

/// The 269-byte reset at `f`: `this` = `ecx`, `x114` = `edx`, `arg` =
/// the second stack word.
pub fn reset_state_edx(g: &mut impl Mem, img: &Image, f: u32, this: u32, x114: u32, arg: u32) {
    reset_state(g, this, arg, x114, img.word(f + 0x18));
}

/// [`reset_state_arg`] in `magic_0182` (0x10001a50, 273 bytes).
pub const RESET_STATE_ARG_0182_1A50: Sig = Sig {
    len: 273,
    hash: 0xee62_eea3_e062_d6d3,
    calls: &[],
    imports: &[],
};

/// [`reset_state_arg`] in `magic_0197` (0x100019b0, 273 bytes).
pub const RESET_STATE_ARG_0197_19B0: Sig = Sig {
    len: 273,
    hash: 0xc348_9da7_f5fb_f821,
    calls: &[],
    imports: &[],
};

/// [`reset_state_arg`] in `magic_0199` (0x10002b20, 273 bytes).
pub const RESET_STATE_ARG_0199_2B20: Sig = Sig {
    len: 273,
    hash: 0x77aa_a6e6_5f59_7bcb,
    calls: &[],
    imports: &[],
};

/// [`reset_state_arg`] in `magic_0252` (0x10001ee0, 273 bytes).
pub const RESET_STATE_ARG_0252_1EE0: Sig = Sig {
    len: 273,
    hash: 0x7cc5_f427_efee_8991,
    calls: &[],
    imports: &[],
};

/// [`reset_state_arg`] in `magic_0266` (0x10003900, 273 bytes).
pub const RESET_STATE_ARG_0266_3900: Sig = Sig {
    len: 273,
    hash: 0xf24e_80c8_e2c2_8e82,
    calls: &[],
    imports: &[],
};

/// [`reset_state_arg`] in `magic_0311` (0x10002b40, 273 bytes).
pub const RESET_STATE_ARG_0311_2B40: Sig = Sig {
    len: 273,
    hash: 0xdfdc_495d_1991_21a3,
    calls: &[],
    imports: &[],
};

/// [`reset_state_arg`] in `magic_0365` (0x10001560, 273 bytes).
pub const RESET_STATE_ARG_0365_1560: Sig = Sig {
    len: 273,
    hash: 0xfe8a_6033_2921_3f5f,
    calls: &[],
    imports: &[],
};

/// [`reset_state_arg`] in `magic_0447` (0x10003200, 273 bytes).
pub const RESET_STATE_ARG_0447_3200: Sig = Sig {
    len: 273,
    hash: 0xb585_2af9_8611_3535,
    calls: &[],
    imports: &[],
};

/// [`reset_state_arg`] in `magic_0475` (0x100026c0, 273 bytes).
pub const RESET_STATE_ARG_0475_26C0: Sig = Sig {
    len: 273,
    hash: 0x058a_c3e5_2367_efd9,
    calls: &[],
    imports: &[],
};

/// [`reset_state_bare`] in `magic_0245` (0x100023f0, 270 bytes).
pub const RESET_STATE_BARE_0245_23F0: Sig = Sig {
    len: 270,
    hash: 0x55ae_3e63_8e0b_6790,
    calls: &[],
    imports: &[],
};

/// [`reset_state_bare`] in `magic_0298` (0x10002ce0, 270 bytes).
pub const RESET_STATE_BARE_0298_2CE0: Sig = Sig {
    len: 270,
    hash: 0x8b9b_6ad7_d37d_69f0,
    calls: &[],
    imports: &[],
};

/// [`reset_state_bare`] in `magic_0308` (0x10001ec0, 270 bytes).
pub const RESET_STATE_BARE_0308_1EC0: Sig = Sig {
    len: 270,
    hash: 0x369b_8bfb_54e2_78cd,
    calls: &[],
    imports: &[],
};

/// [`reset_state_bare`] in `magic_0435` (0x10002790, 270 bytes).
pub const RESET_STATE_BARE_0435_2790: Sig = Sig {
    len: 270,
    hash: 0x8cbc_24ed_8e1a_fad8,
    calls: &[],
    imports: &[],
};

/// [`reset_state_bare`] in `magic_0671` (0x10002b50, 270 bytes).
pub const RESET_STATE_BARE_0671_2B50: Sig = Sig {
    len: 270,
    hash: 0xe65f_8b07_610e_cd6c,
    calls: &[],
    imports: &[],
};

/// [`reset_state_edx`] in `magic_0108` (0x10001af0, 269 bytes).
pub const RESET_STATE_EDX_0108_1AF0: Sig = Sig {
    len: 269,
    hash: 0x779f_b3e8_b937_23a3,
    calls: &[],
    imports: &[],
};

/// [`reset_state_edx`] in `magic_0221` (0x10001af0, 269 bytes).
pub const RESET_STATE_EDX_0221_1AF0: Sig = Sig {
    len: 269,
    hash: 0xc9f2_c1e8_d4dc_15c2,
    calls: &[],
    imports: &[],
};

/// [`reset_state_edx`] in `magic_0419` (0x100037c0, 269 bytes).
pub const RESET_STATE_EDX_0419_37C0: Sig = Sig {
    len: 269,
    hash: 0xac25_1b24_fe06_3cab,
    calls: &[],
    imports: &[],
};
