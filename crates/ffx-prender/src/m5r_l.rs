//! M5 stage 1, list `render-pcui` (round 3), batch 1: the render side's
//! jump thunks, the FMV manager's small accessors, the application's
//! float getters, the post-process manager's flag and a float swap.
//!
//! The thunks are `jmp` (or `mov ecx, [g_PhyreScene]; jmp`) to functions
//! the game already runs as ports; the bridges call them by address, so
//! the call reaches the port through the hook (or the function table).

use crate::m5r_c::quiet;
use ffx_bridge::Global;

/// `g_PhyreScene`.
pub const SCENE: Global<u32> = Global::new(0x00cc_b9d8);
/// `g_PostProcessManager`.
pub const POST_PROCESS: Global<u32> = Global::new(0x00cc_c8a0);

pub const RADIAL_LINE_DESTROY: u32 = 0x006f_7b20;
pub const MS_GET_ENCOUNT_TYPE: u32 = 0x0078_2a10;
pub const MS_BATTLE_CHECK: u32 = 0x0078_0c90;
pub const ATTACH_CHR_CLUSTER: u32 = 0x0065_78e0;
pub const CHARACTER_UNLOAD_TASK: u32 = 0x0063_7980;
pub const SEGMENTS_ALL_READY: u32 = 0x0065_22b0;
pub const SCENE_UPDATE_MAP_YIQ: u32 = 0x0067_1da0;
pub const BROKEN_SCREEN_RELEASE: u32 = 0x006d_3e00;
pub const FMV_MANAGER: u32 = 0x006d_73a0;
pub const FMV_END_TIME_MS: u32 = 0x006d_91d0;
pub const FMV_AT_END_FRAME: u32 = 0x006d_a240;
pub const GET_APPLICATION: u32 = 0x0062_3b90;

/// The FMV manager's "playing" byte (`+0x6d0`) and the byte the state
/// getter reads while it is set (`+0x6e4`).
pub const FMV_PLAYING: u32 = 0x6d0;
pub const FMV_STATE: u32 = 0x6e4;
/// The post-process manager's byte `graphicPostProcessSetFlag_00699080`
/// sets.
pub const POST_FLAG: u32 = 0x1_23c4;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, f: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Cdecl, args)
    }

    /// A `jmp target` thunk: `target` with the thunk's own arguments
    /// (`conv` as the target takes them) -> its `eax`.
    pub fn forward(h: &mut impl Host, target: u32, conv: Conv, args: &[u32]) -> u32 {
        h.call_sym(target, conv, args)
    }

    /// `mov ecx, [g_PhyreScene]; jmp target`: the scene method `target`
    /// with no stack words -> its `eax`.
    pub fn scene_forward(h: &mut impl Host, target: u32) -> u32 {
        let scene = SCENE.get(h);
        h.call_sym(target, Conv::Thiscall(scene), &[])
    }

    /// `graphicGetVideoEndTime()`: `PhyFMVPlayerManager_getEndTimeMs` on
    /// the FMV manager (a tail jump) -> its `eax`.
    pub fn video_end_time(h: &mut impl Host) -> u32 {
        let m = cdecl(h, FMV_MANAGER, &[]);
        h.call_sym(FMV_END_TIME_MS, Conv::Thiscall(m), &[])
    }

    /// `graphicVideoClearPlaying_00643e50()`: the FMV manager's byte
    /// `+0x6d0` cleared (the inlined body of `0x006dac40`) -> the manager.
    pub fn video_clear_playing(h: &mut impl Host) -> u32 {
        let m = cdecl(h, FMV_MANAGER, &[]);
        h.write(m.wrapping_add(FMV_PLAYING), 0_u8);
        m
    }

    /// `graphicVideoState_00645dc0()` -> 1 while the manager's `+0x6d0` is
    /// clear, else its byte `+0x6e4` (zero-extended).
    pub fn video_state(h: &mut impl Host) -> u32 {
        let m = cdecl(h, FMV_MANAGER, &[]);
        if h.read::<u8>(m.wrapping_add(FMV_PLAYING)) == 0 {
            return 1;
        }
        u32::from(h.read::<u8>(m.wrapping_add(FMV_STATE)))
    }

    /// `graphicisVideoAtEndFrame()`: `PhyFMVPlayerManager_isVideoAtEndFrame`
    /// on the manager, its `al` zero-extended.
    pub fn video_at_end_frame(h: &mut impl Host) -> u32 {
        let m = cdecl(h, FMV_MANAGER, &[]);
        h.call_sym(FMV_AT_END_FRAME, Conv::Thiscall(m), &[]) & 0xff
    }

    /// `graphicVideoIsPlaying_00660410()` -> `al` the manager's byte
    /// `+0x6d0`; the rest of `eax` is the manager's address (`mov al`).
    pub fn video_is_playing(h: &mut impl Host) -> u32 {
        let m = cdecl(h, FMV_MANAGER, &[]);
        let b = h.read::<u8>(m.wrapping_add(FMV_PLAYING));
        (m & 0xffff_ff00) | u32::from(b)
    }

    /// The application's float getters (`0x006411a0`, `0x00641270` ..
    /// `0x006412a0`): `GetApplication()`'s float at `off`, as `fld dword`
    /// leaves it.
    pub fn app_float(h: &mut impl Host, off: u32) -> f32 {
        let app = cdecl(h, GET_APPLICATION, &[]);
        f32::from_bits(h.read(app.wrapping_add(off)))
    }

    /// `graphicPostProcessSetFlag_00699080()`: the post-process manager's
    /// byte `+0x123c4` set -> the manager.
    pub fn post_set_flag(h: &mut impl Host) -> u32 {
        let p = POST_PROCESS.get(h);
        h.write(p.wrapping_add(POST_FLAG), 1_u8);
        p
    }

    /// `FloatSwap_006c7c80(a, b)` (`slots`: the argument slots; the
    /// original parks `*a` in `a`'s slot): the floats at `a` and `b`
    /// swapped through the x87 (signalling NaNs come out quiet) -> `b`.
    pub fn float_swap(h: &mut impl Host, slots: u32, a: u32, b: u32) -> u32 {
        let x = quiet(h.read(a));
        h.write(slots, x);
        let y = quiet(h.read(b));
        h.write(a, y);
        h.write(b, x);
        b
    }
}
