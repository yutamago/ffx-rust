//! Ports against the original on harvested call states (`ffx_oracle::harvest`,
//! docs/hooking.md "Harvested call states"): the movie (ffx-movie) tests.
//! Every state the drivers reached on the base snapshots (`tools/harvest.sh`);
//! each test skips without `FFX_GAME_DIR` or stored states. New lines go at
//! the end of this file.

use ffx_oracle::{Oracle, OracleHost, harvest_test};

// junk round 1: the movie camera for the current frame (its frame in stack
// scratch, the never-written words 0 as the hook has them); eax unchecked:
// off the camera path the original leaves its stack cookie there, and
// neither caller (Sg_MainLoop, Native_b009_MovieCameraWait_Result) reads it
harvest_test!(
    movie_camera_frame,
    0x0076_edc0,
    args = 0,
    |m: &mut Oracle, _s| {
        let work = m.stack_range().start + 0x400;
        let _ = ffx_movie::bridge::movie_camera_frame(&mut OracleHost(m), work);
    }
);
