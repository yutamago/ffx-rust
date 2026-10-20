//! Ports against the original on harvested call states (`ffx_oracle::harvest`,
//! docs/hooking.md "Harvested call states"): the system (ffx-system) tests. Every state
//! the drivers reached on the base snapshots (`tools/harvest.sh`); each test
//! skips without `FFX_GAME_DIR` or stored states. New lines go at the end of
//! this file. Shared helpers: `tests/common/mod.rs` (`mod common` below).

#![allow(clippy::cast_possible_wrap)] // stack words to the ports' i32 arguments

mod common;

use ffx_oracle::{Oracle, harvest_test};

// system: the per-frame magic file check (Sg_MainLoop)
harvest_test!(
    magic_file_update,
    0x009d_ac60,
    args = 0,
    |m: &mut Oracle, _s| {
        ffx_system::menu::bridge::magic_file_update(m, |m, f, a| {
            let f = m.rebase(f);
            m.call(f, ffx_oracle::Conv::Cdecl, a).unwrap().eax
        })
    }
);
