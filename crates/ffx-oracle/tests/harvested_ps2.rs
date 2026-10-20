//! Ports against the original on harvested call states (`ffx_oracle::harvest`,
//! docs/hooking.md "Harvested call states"): the VU0 helpers (ffx-ps2) tests. Every state
//! the drivers reached on the base snapshots (`tools/harvest.sh`); each test
//! skips without `FFX_GAME_DIR` or stored states. New lines go at the end of
//! this file. Shared helpers: `tests/common/mod.rs` (`mod common` below).

#![allow(clippy::cast_possible_wrap)] // stack words to the ports' i32 arguments

mod common;

use ffx_oracle::{Oracle, harvest_test};

use ffx_ps2::vu0::bridge as vu0;

// ps2_emu: VU0 helpers (float math on vectors and matrices)
harvest_test!(
    ffx_vu0_apply_matrix,
    0x006e_d2e0,
    args = 3,
    |m: &mut Oracle, s| { vu0::apply_matrix(m, s.arg(0), s.arg(1), s.arg(2)) }
);
harvest_test!(
    ffx_vu0_rot_trans_pers,
    0x006e_d840,
    args = 4,
    |m: &mut Oracle, s| { vu0::rot_trans_pers(m, s.arg(0), s.arg(1), s.arg(2), s.arg(3)) }
);
