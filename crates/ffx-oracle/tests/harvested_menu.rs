//! Ports against the original on harvested call states (`ffx_oracle::harvest`,
//! docs/hooking.md "Harvested call states"): the menu (ffx-menu) tests. Every state
//! the drivers reached on the base snapshots (`tools/harvest.sh`); each test
//! skips without `FFX_GAME_DIR` or stored states. New lines go at the end of
//! this file. Shared helpers: `tests/common/mod.rs` (`mod common` below).

#![allow(clippy::cast_possible_wrap)] // stack words to the ports' i32 arguments

mod common;

use common::int;
use ffx_oracle::{Oracle, harvest_test};

// menu (pad history)
harvest_test!(
    tk_get_controler_analog_hist,
    0x0088_8c20,
    args = 4,
    |m: &mut Oracle, s| {
        ffx_menu::tk::bridge::tk_get_controler_analog_hist(
            m,
            int(s, 0),
            int(s, 1),
            s.arg(2),
            int(s, 3),
        )
    }
);
// small round 1
harvest_test!(
    tk_draw_no_trans,
    0x0088_d4f0,
    args = 0,
    |m: &mut Oracle, _s| {
        ffx_menu::tk::bridge::tk_draw_no_trans(m, &mut |m, f, a| {
            m.call(f, ffx_oracle::Conv::Cdecl, a).unwrap().eax
        })
    }
);
