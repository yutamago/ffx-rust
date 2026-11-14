//! M5 finish, WP5 part 6 (wp5g): the functions the replays of the build
//! without Unicorn stopped at, and the frontier's leftovers, against the
//! original, on the image and on the snapshots (`FFX_SNAPSHOT`). Results
//! and every byte written must agree. Needs `FFX_GAME_DIR`.
//!
//! The `FFX.exe` callees are recording stubs over their code.

#![allow(clippy::cast_possible_truncation, clippy::too_many_lines)]

mod m5_main3_common;

use m5_main3_common::{G, Out, Rng, Twins};

use ffx_oracle::Conv;

/// A cdecl call through the port's oracle (the battle bridges' `Call`).
fn cdecl(m: &mut G, f: u32, args: &[u32]) -> u32 {
    m.0.call(f, Conv::Cdecl, args).unwrap().eax
}

#[test]
fn wp5g_reg_motion_read_done_matches_the_original() {
    use ffx_battle::misc::bridge::{REG_MOTION, REG_MOTION_FLAG};
    use ffx_battle::read::bridge as rd;
    Twins::each(|t| {
        let mut rng = Rng(0x0079_cc80);
        for k in 0..8 {
            t.reset();
            t.stub(rd::CHN_ADD_SYSTEM_MGRP, false, 1, 0, 0x4400 + k);
            t.g32(REG_MOTION.sym, rng.next());
            t.g8(REG_MOTION_FLAG.sym, rng.next() as u8);
            t.check(
                &format!("MsRegMotionReadDone {k}"),
                0x0079_cc80,
                Conv::Cdecl,
                &[],
                |m| Out::U32(rd::ms_reg_motion_read_done(m, &mut cdecl)),
            );
            assert_eq!(t.calls(), 1);
        }
    });
}
