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
// small round 1: the CTB window's frame (MsGetCTBlist's count word in stack scratch)
harvest_test!(
    to_btl_ctrl_ctb_window,
    0x0089_ece0,
    args = 2,
    |m: &mut Oracle, s| {
        let local = m.stack_range().start + 0x100;
        ffx_system::btl::bridge::ctrl_ctb_window(
            m,
            s.arg(1),
            local,
            |m, f, a| {
                let f = m.rebase(f);
                m.call(f, ffx_oracle::Conv::Cdecl, a).unwrap().eax
            },
            |m, f, a| m.call(f, ffx_oracle::Conv::Cdecl, a).unwrap().eax,
        );
    }
);
// small round 2
harvest_test!(
    to_wait_btl_win_main,
    0x008a_8a70,
    args = 1,
    |m: &mut Oracle, s| {
        ffx_system::btl::bridge::to_wait_btl_win_main(
            m,
            s.arg(0),
            &mut |m: &mut Oracle, f, a: &[u32]| m.call(f, ffx_oracle::Conv::Cdecl, a).unwrap().eax,
        )
    }
);
// small round 5
harvest_test!(cd_read_sync, 0x0076_c490, args = 1, |m: &mut Oracle, s| {
    ffx_system::cd::bridge::read_sync(m, s.arg(0), |m, f, a| {
        let f = m.rebase(f);
        m.call(f, ffx_oracle::Conv::Cdecl, a).unwrap().eax
    })
});
harvest_test!(
    to_control_ms_set_menu_command,
    0x0089_afc0,
    args = 0,
    |m: &mut Oracle, _| {
        ffx_system::tolib::bridge::to_control_ms_set_menu_command(m, |m, f, a| {
            let f = m.rebase(f);
            m.call(f, ffx_oracle::Conv::Cdecl, a).unwrap().eax
        })
    }
);
// small round 6
harvest_test!(
    menu_scene_track_step,
    0x00a7_b510,
    args = 5,
    |m: &mut Oracle, s| {
        let args = [s.arg(0), s.arg(1), s.arg(2), s.arg(3), s.arg(4)];
        ffx_system::scene::bridge::menu_scene_track_step(m, args, |m, f, a| {
            let f = m.rebase(f);
            m.call(f, ffx_oracle::Conv::Cdecl, a).unwrap().eax
        })
    }
);
// pc round 6
harvest_test!(
    to_control_btl_window,
    0x0089_ae20,
    args = 0,
    |m: &mut Oracle, _s| {
        ffx_system::btl::bridge::to_control_btl_window(
            m,
            |m, f, a| {
                let f = m.rebase(f);
                m.call(f, ffx_oracle::Conv::Cdecl, a).unwrap().eax
            },
            |m, f, a| m.call(f, ffx_oracle::Conv::Cdecl, a).unwrap().eax,
            |m, f, a| {
                let f = m.rebase(f);
                m.call_f64(f, a).unwrap()
            },
        );
    }
);
