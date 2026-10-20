//! Ports against the original on harvested call states (`ffx_oracle::harvest`,
//! docs/hooking.md "Harvested call states"): the kernel (ffx-kernel) tests. Every state
//! the drivers reached on the base snapshots (`tools/harvest.sh`); each test
//! skips without `FFX_GAME_DIR` or stored states. New lines go at the end of
//! this file. Shared helpers: `tests/common/mod.rs` (`mod common` below).

#![allow(clippy::cast_possible_wrap)] // stack words to the ports' i32 arguments

mod common;

use common::int;
use ffx_oracle::{Oracle, harvest_test};

// kernel: a record of a loaded kernel table
harvest_test!(
    ms_get_excel_data,
    0x007a_b870,
    args = 3,
    |m: &mut Oracle, s| { ffx_kernel::bridge::ms_get_excel_data(m, int(s, 0), s.arg(1), s.arg(2)) }
);
