//! The movie player's glue: `PhyFMVPlayerManager`'s pause, resume and
//! terminate, the PS2 movie layer's scene tests, the small tree heads the
//! managers allocate.

use ffx_bridge::{Global, Mem};
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

/// `PVideoPlaybackWin32_setVolume(video; f32)` (thiscall).
pub const VIDEO_SET_VOLUME: u32 = 0x00a2_8330;
/// The movie volume `PhyFMVPlayerManager_SetVolume_006dabf0` keeps
/// (`0x00c34420`, f32).
pub const MOVIE_VOLUME: Global<u32> = Global::new(0x00c3_4420);
/// `PhyFMVPlayerManager +4`: the state count; `+0xc` the states.
pub const FMV_STATE_COUNT: u32 = 0x4;
/// `PhyFMVPlayerManager +0xc`.
pub const FMV_STATES: u32 = 0xc;
/// `PhyFMVPlayerManager +0x704`.
pub const FMV_PENDING: u32 = 0x704;

/// Whether one of the manager's `count` (`+4`, unsigned) state words
/// (`+0xc`) is `v`: the loop of `PhyFMVPlayerManager_hasState_006da1f0`
/// and `_StateCheck_006da2a0` (it stops reading at the first match).
fn has_state_word(h: &impl Mem, this: u32, v: u32) -> bool {
    let n = h.read::<u32>(this + FMV_STATE_COUNT);
    for i in 0..n {
        let states = h.read::<u32>(this + FMV_STATES);
        if h.read::<u32>(states.wrapping_add(4 * i)) == v {
            return true;
        }
    }
    false
}

/// `PhyFMVPlayerManager_hasState_006da1f0(this; v)` (thiscall, from
/// `PhyreScene_render`): 1 when a state word is `v`, else 0.
pub fn fmv_has_state(h: &impl Mem, this: u32, v: u32) -> u32 {
    u32::from(has_state_word(h, this, v))
}

/// `PhyFMVPlayerManager_StateCheck_006da2a0(this)` (thiscall, `al`): 1
/// when a state word is 1 or nothing is pending (`+0x704`), else 0.
pub fn fmv_state_check(h: &impl Mem, this: u32) -> u8 {
    if has_state_word(h, this, 1) {
        return 1;
    }
    u8::from(h.read::<u32>(this + FMV_PENDING) == 0)
}

/// `PhyFMVPlayerManager_SetVolume_006dabf0(volume)` (cdecl f32): the
/// volume kept (through `fld`/`fst`); with a manager that has a movie
/// loaded and the pause word 1, `PVideoPlaybackWin32_setVolume(manager +
/// 0x38; volume)`. Callers read no result.
pub fn fmv_set_volume(h: &mut impl Host, volume: u32) {
    let v = ffx_bridge::x87::round_trip(f32::from_bits(volume)).to_bits();
    let m = Global::<u32>::new(FMV_PLAYER_MANAGER.global).get(h);
    MOVIE_VOLUME.set(h, v);
    if m == 0 || h.read::<u8>(m + FMV_LOADED) == 0 || PAUSE_ENABLE.get(h) != 1 {
        return;
    }
    h.call_sym(VIDEO_SET_VOLUME, Conv::Thiscall(m + FMV_VIDEO), &[v]);
}

/// `TreeNode_new_006d78f0(left, right)` (stdcall): a `0x14`-byte node from
/// `operator_new` (`std::_Xbad_alloc` called when it fails) with its two
/// links: `(left, right)`, or itself twice for a null `left`; returns it.
pub fn tree_node_new(h: &mut impl Host, left: u32, right: u32) -> u32 {
    let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x14]);
    if p == 0 {
        let f = XBAD_ALLOC.get(h);
        h.call(f, Conv::Cdecl, &[]);
    }
    let (a, b) = if left == 0 { (p, p) } else { (left, right) };
    h.write(p, a);
    if p.wrapping_add(4) != 0 {
        h.write(p.wrapping_add(4), b);
    }
    p
}
