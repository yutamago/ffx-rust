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

// script round 24 (this machine): the movie check, the guide map records
// (the start/end y go to the original's own argument slot), the vibration
// volume
harvest_test!(
    movie_is_busy,
    0x0076_ed10,
    args = 0,
    |m: &mut Oracle, _s| { hp::movie_is_busy(&mut OracleHost(m)) }
);
harvest_test!(
    atel_get_guide_map_actor_info,
    0x0086_b740,
    args = 2,
    |m: &mut Oracle, s| {
        hp::atel_get_guide_map_actor_info(m, s.arg(0), s.arg(1), s.regs.esp + 8)
    }
);
harvest_test!(
    atel_get_gide_map_actor_info,
    0x0086_b700,
    args = 2,
    |m: &mut Oracle, s| {
        hp::atel_get_guide_map_actor_info(m, s.arg(0), s.arg(1), s.regs.esp + 8)
    }
);
harvest_test!(
    vibrate_update_volume,
    0x0087_5b40,
    args = 0,
    |m: &mut Oracle, _s| { hp::vibrate_update_volume(&mut OracleHost(m)) }
);
