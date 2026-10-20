//! Ports against the original on harvested call states (`ffx_oracle::harvest`,
//! docs/hooking.md "Harvested call states"): the map (ffx-field) tests. Every state
//! the drivers reached on the base snapshots (`tools/harvest.sh`); each test
//! skips without `FFX_GAME_DIR` or stored states. New lines go at the end of
//! this file. Shared helpers: `tests/common/mod.rs` (`mod common` below).

#![allow(clippy::cast_possible_wrap)] // stack words to the ports' i32 arguments

mod common;

use ffx_oracle::{Oracle, harvest_test};

// map (ffx-field)
harvest_test!(
    yi_render_setup_fog_param,
    0x0090_d1b0,
    args = 1,
    |m: &mut Oracle, s| {
        ffx_field::yi::bridge::yi_render_setup_fog_param(m, s.arg(0));
    }
);
