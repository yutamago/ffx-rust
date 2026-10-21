//! Ports against the original on harvested call states (`ffx_oracle::harvest`,
//! docs/hooking.md "Harvested call states"): the script (ffx-script) tests. Every
//! state the drivers reached on the base snapshots (`tools/harvest.sh`); each test
//! skips without `FFX_GAME_DIR` or stored states. New lines go at the end of
//! this file. Shared helpers: `tests/common/mod.rs`.

use ffx_oracle::{Oracle, OracleHost, harvest_test};
use ffx_script::vm::bridge::helpers as hp;

// script round 23 (this machine): worker positions, the guide map angle,
// the effect poll
harvest_test!(
    atel_get_actor_start_pos,
    0x0086_a720,
    args = 4,
    |m: &mut Oracle, s| { hp::atel_get_actor_start_pos(m, s.arg(0), s.arg(1), s.arg(2), s.arg(3)) }
);
harvest_test!(
    atel_get_actor_end_pos,
    0x0086_a390,
    args = 4,
    |m: &mut Oracle, s| { hp::atel_get_actor_end_pos(m, s.arg(0), s.arg(1), s.arg(2), s.arg(3)) }
);
harvest_test!(
    script_get_worker_pos,
    0x0086_a2f0,
    args = 4,
    |m: &mut Oracle, s| { hp::script_get_worker_pos(m, s.arg(0), s.arg(1), s.arg(2), s.arg(3)) }
);
harvest_test!(
    atel_get_guide_map_ang_ex,
    0x0086_b9a0,
    args = 1,
    |m: &mut Oracle, s| { hp::atel_get_guide_map_ang_ex(m, s.arg(0)) }
);
harvest_test!(
    atel_effect_idle_poll,
    0x0086_33a0,
    args = 0,
    |m: &mut Oracle, _s| { hp::atel_effect_idle_poll(&mut OracleHost(m)) }
);
