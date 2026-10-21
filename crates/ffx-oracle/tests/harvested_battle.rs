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

// battle round 33: the cursors and the overdrive bars (the frame locals in
// stack scratch; the projection and graphicUiRemapY2 return in ST0, as
// fcall's f32 bits; the live actors' positions take no sine or cosine)
fn r33_call(m: &mut Oracle, f: u32, a: &[u32]) -> u32 {
    m.call(f, ffx_oracle::Conv::Cdecl, a).unwrap().eax
}
fn r33_fcall(m: &mut Oracle, f: u32, a: &[u32]) -> u32 {
    let r = m.call(f, ffx_oracle::Conv::Cdecl, a).unwrap();
    r.st0_f32().unwrap().to_bits()
}
fn r33_trig(_: ffx_battle::pos::bridge::Trig, _: u32) -> u32 {
    panic!("a harvested battle position took a sine or cosine")
}
harvest_test!(
    ms_get_chr_limit_bar_flag,
    0x0079_5250,
    args = 1,
    |m: &mut Oracle, s| {
        let local = m.stack_range().start + 0x100;
        ffx_battle::cursor::bridge::ms_get_chr_limit_bar_flag(
            m,
            &r33_trig,
            &mut r33_call,
            &mut r33_fcall,
            s.arg(0),
            local,
        )
    }
);
harvest_test!(
    ms_calc_cursor_pos,
    0x0079_f380,
    args = 0,
    |m: &mut Oracle, _s| {
        let local = m.stack_range().start + 0x100;
        ffx_battle::cursor::bridge::ms_calc_cursor_pos(
            m,
            &r33_trig,
            &mut r33_call,
            &mut r33_fcall,
            local,
        );
    }
);
harvest_test!(
    ms_battle_cursor_calc,
    0x0078_0cc0,
    args = 0,
    |m: &mut Oracle, _s| {
        let local = m.stack_range().start + 0x100;
        ffx_battle::cursor::bridge::ms_battle_cursor_calc(
            m,
            &r33_trig,
            &mut r33_call,
            &mut r33_fcall,
            local,
        );
    }
);
