//! The movie player's glue: `PhyFMVPlayerManager`'s pause, resume and
//! terminate, the PS2 movie layer's scene tests, the small tree heads the
//! managers allocate.

use ffx_bridge::Global;
use ffx_graphic::host::{Conv, Host};

use crate::effect::{FMV_PLAYER_MANAGER, PHYRE_PRINTF};

/// `graphicIsInLucaTheater()`.
pub const IN_LUCA_THEATER: u32 = 0x0064_1c10;
/// The event the movie tests look at (`0x00c40e28`).
pub const MOVIE_EVENT: Global<u32> = Global::new(0x00c4_0e28);
/// `PVideoPlaybackWin32_terminate(video)` (thiscall).
pub const VIDEO_TERMINATE: u32 = 0x00a2_8430;
/// `PVideoPlaybackWin32_togglePause(video)` (thiscall).
pub const VIDEO_TOGGLE_PAUSE: u32 = 0x00a2_7e90;
/// `PhyFMVPlayerManager +0x38`: its `PVideoPlaybackWin32`.
pub const FMV_VIDEO: u32 = 0x38;
/// `PhyFMVPlayerManager +0x6d0`: a movie is loaded (byte).
pub const FMV_LOADED: u32 = 0x6d0;
/// `PhyFMVPlayerManager +0x6d2`: playing (byte).
pub const FMV_PLAYING: u32 = 0x6d2;
/// `PhyFMVPlayerManager +0x6fc`: paused (byte).
pub const FMV_PAUSED: u32 = 0x6fc;
/// The word `pauseMovie` needs to be 1 (`0x0112a008`).
pub const PAUSE_ENABLE: Global<u32> = Global::new(0x0112_a008);
/// `operator_new(n)`.
pub const OPERATOR_NEW: u32 = 0x0063_0310;
/// The import slot of `std::_Xbad_alloc` (`0x00b0c2a0`).
pub const XBAD_ALLOC: Global<u32> = Global::new(0x00b0_c2a0);

/// `movie_with3DScene()` (0x0076f0c0): 1 outside the Luca theatre when
/// the event is `0x48` or `0x46`, else 0.
pub fn with_3d_scene(h: &mut impl Host) -> u32 {
    if h.call_sym(IN_LUCA_THEATER, Conv::Cdecl, &[]) != 0 {
        return 0;
    }
    u32::from(matches!(MOVIE_EVENT.get(h), 0x48 | 0x46))
}

/// `movie_have_camera()` (0x0076efc0): 1 outside the Luca theatre when
/// the event is `0x2e`, `0x48`, `0x20` or `0x46`, else 0.
pub fn have_camera(h: &mut impl Host) -> u32 {
    if h.call_sym(IN_LUCA_THEATER, Conv::Cdecl, &[]) != 0 {
        return 0;
    }
    u32::from(matches!(MOVIE_EVENT.get(h), 0x2e | 0x48 | 0x20 | 0x46))
}

fn print_missing(h: &mut impl Host) {
    let message = h.rebase(FMV_PLAYER_MANAGER.message);
    h.call_sym(PHYRE_PRINTF, Conv::Cdecl, &[1, message]);
}

/// `PhyFMVPlayerManager_fmvPlayer_terminate(this)` (0x006d9180,
/// thiscall): the singleton message when the global is null (the method
/// still runs on `this`), `PVideoPlaybackWin32_terminate(this + 0x38)`,
/// playing = 0; the terminate's `eax`.
pub fn fmv_terminate(h: &mut impl Host, this: u32) -> u32 {
    if Global::<u32>::new(FMV_PLAYER_MANAGER.global).get(h) == 0 {
        print_missing(h);
    }
    let r = h.call_sym(VIDEO_TERMINATE, Conv::Thiscall(this + FMV_VIDEO), &[]);
    h.write(this + FMV_PLAYING, 0u8);
    r
}

/// `PhyFMVPlayerManager_pauseMovie(this)` (0x006da4b0, thiscall): with a
/// movie loaded and the pause word 1, `togglePause` and the paused byte
/// flipped. Callers read no result.
pub fn fmv_pause(h: &mut impl Host, this: u32) {
    if h.read::<u8>(this + FMV_LOADED) == 0 || PAUSE_ENABLE.get(h) != 1 {
        return;
    }
    h.call_sym(VIDEO_TOGGLE_PAUSE, Conv::Thiscall(this + FMV_VIDEO), &[]);
    let paused = h.read::<u8>(this + FMV_PAUSED);
    h.write(this + FMV_PAUSED, u8::from(paused == 0));
}

/// `PhyFMVPlayerManager_resumeMovie()` (0x006d8f80, on the singleton,
/// message and global read again when it is null): `togglePause`, paused
/// = 0; the toggle's `eax`.
pub fn fmv_resume(h: &mut impl Host) -> u32 {
    let g = Global::<u32>::new(FMV_PLAYER_MANAGER.global);
    let mut m = g.get(h);
    if m == 0 {
        print_missing(h);
        m = g.get(h);
    }
    let r = h.call_sym(VIDEO_TOGGLE_PAUSE, Conv::Thiscall(m + FMV_VIDEO), &[]);
    h.write(m + FMV_PAUSED, 0u8);
    r
}

/// `TreeHead_new_006d78c0` / `TreeHead_new_006a0360`: a `0x2c`-byte
/// `std::map` head from `operator_new`, its three links to itself and
/// the colour/nil bytes `0x101` at `+0xc`; returns it. On a null
/// allocation, a tail jump to `std::_Xbad_alloc` (which throws).
pub fn tree_head_new(h: &mut impl Host) -> u32 {
    let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x2c]);
    if p == 0 {
        let f = XBAD_ALLOC.get(h);
        return h.call(f, Conv::Cdecl, &[]);
    }
    h.write(p, p);
    // the original tests each link's address for null (`lea`, `test`):
    // never null past a non-null p in the game's address space
    if p.wrapping_add(4) != 0 {
        h.write(p + 4, p);
    }
    if p.wrapping_add(8) != 0 {
        h.write(p + 8, p);
    }
    h.write(p + 0xc, 0x0101u16);
    p
}
