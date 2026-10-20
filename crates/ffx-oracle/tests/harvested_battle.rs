//! Ports against the original on harvested call states (`ffx_oracle::harvest`,
//! docs/hooking.md "Harvested call states"): the battle (ffx-battle) tests. Every state
//! the drivers reached on the base snapshots (`tools/harvest.sh`); each test
//! skips without `FFX_GAME_DIR` or stored states. New lines go at the end of
//! this file. Shared helpers: `tests/common/mod.rs` (`mod common` below).

#![allow(clippy::cast_possible_wrap)] // stack words to the ports' i32 arguments

mod common;

use ffx_oracle::{Oracle, harvest_test};

// battle
harvest_test!(
    ms_get_screen_data,
    0x007b_bd00,
    args = 4,
    |m: &mut Oracle, s| {
        ffx_battle::flow::bridge::ms_get_screen_data(m, [s.arg(0), s.arg(1), s.arg(2), s.arg(3)])
    }
);

// battle round 29: small leaves the drivers reach
harvest_test!(
    ms_battle_draw,
    0x0078_0d00,
    args = 0,
    |m: &mut Oracle, _s| {
        ffx_battle::minor::bridge::ms_battle_draw(m, &mut |m, f, a| {
            m.call(f, ffx_oracle::Conv::Cdecl, a).unwrap().eax
        })
    }
);
harvest_test!(
    ms_battle_encount_flag,
    0x0078_0fe0,
    args = 1,
    |m: &mut Oracle, s| {
        ffx_battle::misc::bridge::ms_battle_encount_flag(m, s.arg(0));
    }
);
harvest_test!(
    ms_get_chr_head_pos,
    0x0079_50b0,
    args = 2,
    |m: &mut Oracle, s| {
        ffx_battle::minor::bridge::ms_get_chr_head_pos(m, s.arg(0), s.arg(1));
    }
);
harvest_test!(
    ms_add_next_ctb,
    0x0079_a020,
    args = 5,
    |m: &mut Oracle, s| {
        ffx_battle::minor::bridge::ms_add_next_ctb(
            m,
            s.arg(0),
            s.arg(1),
            s.arg(2),
            s.arg(3),
            s.arg(4),
        );
    }
);
harvest_test!(
    ms_get_ctb_list,
    0x0079_a200,
    args = 1,
    |m: &mut Oracle, s| {
        // the CTBs and counters in stack scratch
        let frame = m.stack_range().start + 0x100;
        ffx_battle::turn::bridge::ms_get_ctb_list(m, s.arg(0), frame)
    }
);
harvest_test!(
    ms_wait_motion_pick,
    0x007a_8c50,
    args = 3,
    |m: &mut Oracle, s| {
        ffx_battle::minor::bridge::ms_wait_motion_pick(m, s.arg(0), s.arg(1), s.arg(2))
    }
);

// battle round 30
harvest_test!(
    ms_walk_ability_process,
    0x0078_2d60,
    args = 1,
    |m: &mut Oracle, s| {
        ffx_battle::minor::bridge::ms_walk_ability_process(m, s.arg(0));
    }
);
