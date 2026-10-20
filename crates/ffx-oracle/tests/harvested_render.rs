//! Ports against the original on harvested call states (`ffx_oracle::harvest`,
//! docs/hooking.md "Harvested call states"): the render (ffx-graphic) tests. Every state
//! the drivers reached on the base snapshots (`tools/harvest.sh`); each test
//! skips without `FFX_GAME_DIR` or stored states. New lines go at the end of
//! this file. Shared helpers: `tests/common/mod.rs` (`mod common` below).

#![allow(clippy::cast_possible_wrap)] // stack words to the ports' i32 arguments

mod common;

use ffx_oracle::{Oracle, harvest_test};

// render (ffx-graphic): a UI element's texture remap
harvest_test!(
    graphic_ui_element_remap,
    0x0064_4320,
    args = 2,
    |m: &mut Oracle, s| {
        ffx_graphic::ui::bridge::element_remap(m, s.arg(0), s.arg(1));
    }
);
