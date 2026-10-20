//! Ports against the original on harvested call states (`ffx_oracle::harvest`,
//! docs/hooking.md "Harvested call states"): the chara (ffx-chara) tests. Every state
//! the drivers reached on the base snapshots (`tools/harvest.sh`); each test
//! skips without `FFX_GAME_DIR` or stored states. New lines go at the end of
//! this file. Shared helpers: `tests/common/mod.rs` (`mod common` below).

#![allow(clippy::cast_possible_wrap)] // stack words to the ports' i32 arguments

mod common;

use ffx_oracle::{Oracle, harvest_test};

// chara
harvest_test!(
    ch_set_sequence,
    0x0083_7ac0,
    args = 2,
    |m: &mut Oracle, s| { ffx_chara::motion::bridge::ch_set_sequence(m, s.arg(0), s.arg(1)) }
);
