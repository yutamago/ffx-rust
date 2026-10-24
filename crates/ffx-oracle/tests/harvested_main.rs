//! Ports against the original on harvested call states (`ffx_oracle::harvest`,
//! docs/hooking.md "Harvested call states"): the main (ffx-sg) tests. Every state
//! the drivers reached on the base snapshots (`tools/harvest.sh`); each test
//! skips without `FFX_GAME_DIR` or stored states. New lines go at the end of
//! this file. Shared helpers: `tests/common/mod.rs` (`mod common` below).

#![allow(clippy::cast_possible_wrap)] // stack words to the ports' i32 arguments

mod common;

use common::{crt_atan, crt_vector_trig, int};
use ffx_oracle::{Conv, Oracle, OracleHost, harvest_test};

// main (ffx-sg): the sequence step of a character, the Z sort
harvest_test!(sg_seq_step, 0x0083_7900, args = 1, |m: &mut Oracle, s| {
    ffx_sg::seq::bridge::seq_step(&mut OracleHost(m), s.arg(0))
});
harvest_test!(
    sg_chr_sort_by_z,
    0x0083_eb30,
    args = 1,
    |m: &mut Oracle, s| {
        ffx_sg::state::bridge::chr_sort_by_z(m, int(s, 0));
    }
);
harvest_test!(
    sg_camera_put_rects,
    0x0083_f130,
    args = 0,
    |m: &mut Oracle, _s| { ffx_sg::camera::bridge::sg_camera_put_rects(&mut OracleHost(m)) }
);
harvest_test!(cam_apply, 0x007b_c980, args = 1, |m: &mut Oracle, s| {
    let work = m.stack_range().start + 0x1000;
    let call = &mut |m: &mut Oracle, f: u32, a: &[u32]| m.call(f, Conv::Cdecl, a).unwrap().eax;
    ffx_sg::camera::bridge::cam_apply(m, s.arg(0), work, call, crt_atan);
});

// main round 16: the free camera, the effect read poll, the id map move,
// the motion update
harvest_test!(
    sg_camera_set_player,
    0x0083_f2e0,
    args = 0,
    |m: &mut Oracle, _s| {
        ffx_sg::state::bridge::camera_set_player(&mut OracleHost(m), &crt_vector_trig)
    }
);
harvest_test!(
    sg_effect_read_poll,
    0x0082_2250,
    args = 0,
    |m: &mut Oracle, _s| { ffx_sg::state::bridge::effect_read_poll(&mut OracleHost(m)) }
);
harvest_test!(
    sg_id_chr_move,
    0x0083_e5f0,
    args = 1,
    |m: &mut Oracle, s| {
        let point = m.stack_range().start + 0x1000;
        let fcall = &mut |h: &mut OracleHost<'_>, f: u32, a: &[u32]| {
            // the next call starts with an empty x87 stack (`Oracle::call`)
            h.0.call_f64(f, a).unwrap()
        };
        ffx_sg::state::bridge::id_chr_move(&mut OracleHost(m), s.arg(0), point, fcall);
    }
);
harvest_test!(
    sg_update_motion,
    0x0083_8c90,
    args = 2,
    |m: &mut Oracle, s| {
        ffx_sg::seq::bridge::update_motion(&mut OracleHost(m), s.arg(0), s.arg(1));
    }
);

// main round 17: the main loop's frame sync
harvest_test!(sg_frame_sync, 0x0082_1e80, args = 1, |m: &mut Oracle, s| {
    let at = m.rebase(ffx_sg::state::bridge::TIME_US);
    let time = &mut |h: &mut OracleHost<'_>| {
        let r = h.0.call(at, Conv::Cdecl, &[]).unwrap();
        u64::from(r.edx) << 32 | u64::from(r.eax)
    };
    ffx_sg::state::bridge::frame_sync(&mut OracleHost(m), s.arg(0), time)
});

// pc round 9 a: the main loop's menu check
harvest_test!(
    sg_main_check_menu,
    0x0082_0750,
    args = 0,
    |m: &mut Oracle, _s| { ffx_sg::state::bridge::main_check_menu(&mut OracleHost(m)) }
);
