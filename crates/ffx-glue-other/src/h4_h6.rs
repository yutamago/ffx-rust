//! Continuation round 6 of list `h4-glue-other`, batch 6: the movie
//! player manager's play request, its tear-down and its skip input.
//!
//! The player manager (`g_FMVPlayerManager`, `0x00cded3c`) keeps the
//! movie's name at `+0x4f0` (empty: no movie), the three plane textures
//! at `+0x38c`/`+0x400`/`+0x474`, the playback object at `+0x38` and a
//! row of flag bytes from `+0x6d0`.

use ffx_bridge::Global;
use ffx_graphic::host::{Conv, Host};

use crate::h4_e1::pass;
use crate::h4_h3::differs;

/// `""` (`0x00b8fbeb`): what an empty movie name is compared with.
pub const EMPTY: u32 = 0x00b8_fbeb;
/// `PhyFMVPlayerManager_resumeMovie_006d8f80(this)` (thiscall).
pub const RESUME_MOVIE: u32 = 0x006d_8f80;
/// `g_FMVPlayerManager` (`0x00cded3c`).
pub const FMV_MANAGER: Global<u32> = Global::new(0x00cd_ed3c);
/// `g_MovieVolume` (`0x00c34420`, f32).
pub const MOVIE_VOLUME: Global<u32> = Global::new(0x00c3_4420);
/// `Phyre_PhyrePrintf(level, fmt, ...)` (cdecl).
pub const PHYRE_PRINTF: u32 = 0x0043_52f0;
/// `"sFMVPlayerManager singleton is not created yet."` (`0x00b49b98`).
pub const NOT_CREATED: u32 = 0x00b4_9b98;
/// `PVideoPlaybackWin32_setVolume(playback; volume)` (thiscall, ret 4).
pub const SET_VOLUME: u32 = 0x00a2_8330;
/// `fiosUnifyFilename(name, out, n)` (cdecl).
pub const FIOS_UNIFY: u32 = crate::h4_h2::FIOS_UNIFY;
/// `"start to play video:%s\n"` (`0x00b49df8`).
pub const START_TO_PLAY: u32 = 0x00b4_9df8;
/// `PhyFMVPlayerManager_fmvPlayer_play(this; name, loop, flag)`
/// (thiscall, ret 0xc).
pub const FMV_PLAY: u32 = 0x006d_8fc0;
/// Bytes of [`play_video`]'s frame: the unified name (`ebp - 0x104`).
pub const PLAY_FRAME: u32 = 0x100;

/// `PhyFMVPlayerManager_PlayVideo(this; flag)` (`0x006d7430`, thiscall,
/// ret 4; `frame` its 0x100-byte name buffer): unless byte `+0x6fd` is
/// set, it takes `flag`'s low byte. With `+0x6fd` set (before or now): a
/// movie already up (`+0x6e4`) and paused (`+0x6fc`) is resumed at the
/// global volume (the manager's playback `+0x38`; a missing manager is
/// reported first); without one, the named movie (`+0x4f0`, unless empty)
/// is unified into the frame, reported and played
/// (`fmvPlayer_play(name, byte +0x6fe, flag)`). With it clear, the named
/// movie is played the same way. Returns 0.
pub fn play_video(h: &mut impl Host, frame: u32, this: u32, flag: u32) -> u32 {
    let flag8 = flag & 0xff;
    let started = if h.read::<u8>(this.wrapping_add(0x6fd)) == 0 {
        h.write(this.wrapping_add(0x6fd), flag8 as u8);
        flag8 != 0
    } else {
        true
    };
    if started && h.read::<u8>(this.wrapping_add(0x6e4)) != 0 {
        if h.read::<u8>(this.wrapping_add(0x6fc)) != 0 {
            h.call_sym(RESUME_MOVIE, Conv::Thiscall(this), &[]);
            if FMV_MANAGER.get(h) == 0 {
                let text = h.rebase(NOT_CREATED);
                h.call_sym(PHYRE_PRINTF, Conv::Cdecl, &[1, text]);
            }
            let playback = FMV_MANAGER.get(h).wrapping_add(0x38);
            let volume = pass(MOVIE_VOLUME.get(h));
            h.call_sym(SET_VOLUME, Conv::Thiscall(playback), &[volume]);
        }
        return 0;
    }
    let name = this.wrapping_add(0x4f0);
    let empty = h.rebase(EMPTY);
    if differs(h, name, empty) {
        h.call_sym(FIOS_UNIFY, Conv::Cdecl, &[name, frame, 0x100]);
        let text = h.rebase(START_TO_PLAY);
        h.call_sym(PHYRE_PRINTF, Conv::Cdecl, &[2, text, name]);
        let looped = u32::from(h.read::<u8>(this.wrapping_add(0x6fe)));
        h.call_sym(FMV_PLAY, Conv::Thiscall(this), &[name, looped, flag]);
    }
    0
}

/// The import slot of the CRT's stream table getter (`__iob_func`,
/// cdecl), `0x00b0c350`.
pub const IOB_FUNC: Global<u32> = Global::new(0x00b0_c350);
/// The import slot of `fprintf(stream, fmt, ...)` (cdecl), `0x00b0c38c`.
pub const FPRINTF: Global<u32> = Global::new(0x00b0_c38c);
/// `"DestroyFMVPlayer when updateTextureCallback in progress..."`
/// (`0x00b49db8`).
pub const DESTROY_WARNING: u32 = 0x00b4_9db8;
/// `isEternalCalmScene(map)` (cdecl).
pub const IS_ETERNAL_CALM: u32 = 0x0067_ab50;
/// `g_GraphicMapId` (`0x01330844`).
pub const MAP_ID: Global<u32> = Global::new(0x0133_0844);
/// `Phyre_PhyreOS_Sleep(ms)` (cdecl).
pub const SLEEP: u32 = 0x0043_0030;
/// `Phyre_PRendering_PTexture2DD3D11_isBound(texture)` (thiscall; `al`).
pub const IS_BOUND: u32 = 0x0059_5f90;
/// `Phyre_PRendering_PTexture2DD3D11_unbind(texture)` (thiscall).
pub const UNBIND: u32 = 0x0059_cc40;
/// `PSpriteArrays_release_00495a10(mesh)` (thiscall).
pub const MESH_RELEASE: u32 = 0x0049_5a10;
/// `RenderTarget_release_00440ea0(cluster)` (thiscall).
pub const CLUSTER_CLEAR: u32 = crate::h4_h2::CLUSTER_CLEAR;
/// `ClusterManager_GetInstance()` (cdecl).
pub const CLUSTER_MANAGER: u32 = 0x0069_a1a0;
/// `ClusterManager_releasePCluster(manager; cluster)` (thiscall, ret 4).
pub const RELEASE_PCLUSTER: u32 = 0x0069_bdb0;
/// `Phyre_PhyreFree(block)` (cdecl).
pub const PHYRE_FREE: u32 = 0x0042_fb20;
/// `PhyFMVPlayerManager_fmvPlayer_terminate(this)` (thiscall).
pub const FMV_TERMINATE: u32 = 0x006d_9180;
/// `PSamplerStateD3D11_release_0059c770(sampler)` (thiscall).
pub const SAMPLER_RELEASE: u32 = 0x0059_c770;
/// The plane textures' offsets.
pub const PLANES: [u32; 3] = [0x38c, 0x400, 0x474];

/// `PhyFMVPlayerManager_DestroyFMVPlayer(this)` (`0x006d71b0`, thiscall):
/// unless byte `+0x6d1` is set: the half-word `+0x6d0` 0x100, byte
/// `+0x6e5` 0; with a texture update in progress (`+0x6f0`) a warning on
/// stderr (`fprintf(&__iob_func()[1], ...)`), and in the Eternal Calm
/// scene up to 60 1 ms sleeps while it lasts. With a movie named
/// (`+0x4f0`): each bound plane texture unbound (a non-zero result is
/// returned), the full-screen mesh `+0x388` released and cleared, the
/// cluster `+0x310` cleared, the shader cluster `+0x360` released, the
/// small array `+0x4e8` emptied, the player terminated and the sampler
/// `+0x364` released. Returns 0 (or the unbind's result).
pub fn destroy_fmv_player(h: &mut impl Host, this: u32) -> u32 {
    if h.read::<u8>(this.wrapping_add(0x6d1)) != 0 {
        return 0;
    }
    h.write(this.wrapping_add(0x6d0), 0x100u16);
    h.write(this.wrapping_add(0x6e5), 0u8);
    let text = h.rebase(DESTROY_WARNING);
    if h.read::<u32>(this.wrapping_add(0x6f0)) != 0 {
        let iob = IOB_FUNC.get(h);
        let streams = h.call(iob, Conv::Cdecl, &[text]);
        let fprintf = FPRINTF.get(h);
        h.call(fprintf, Conv::Cdecl, &[streams.wrapping_add(0x20), text]);
    }
    let map = MAP_ID.get(h);
    if h.call_sym(IS_ETERNAL_CALM, Conv::Cdecl, &[map]) != 0 {
        let mut waited = 0;
        while h.read::<u32>(this.wrapping_add(0x6f0)) != 0 && waited < 0x3c {
            h.call_sym(SLEEP, Conv::Cdecl, &[1]);
            waited += 1;
        }
    }
    let empty = h.rebase(EMPTY);
    if !differs(h, this.wrapping_add(0x4f0), empty) {
        return 0;
    }
    for off in PLANES {
        let texture = this.wrapping_add(off);
        if h.call_sym(IS_BOUND, Conv::Thiscall(texture), &[]) & 0xff != 0 {
            let r = h.call_sym(UNBIND, Conv::Thiscall(texture), &[]);
            if r != 0 {
                return r;
            }
        }
    }
    let mesh: u32 = h.read(this.wrapping_add(0x388));
    if mesh != 0 {
        let sprites: u32 = h.read(mesh);
        h.call_sym(MESH_RELEASE, Conv::Thiscall(sprites), &[]);
        h.write(this.wrapping_add(0x388), 0u32);
    }
    h.call_sym(CLUSTER_CLEAR, Conv::Thiscall(this.wrapping_add(0x310)), &[]);
    let shaders: u32 = h.read(this.wrapping_add(0x360));
    let manager = h.call_sym(CLUSTER_MANAGER, Conv::Cdecl, &[]);
    h.call_sym(RELEASE_PCLUSTER, Conv::Thiscall(manager), &[shaders]);
    let count: u32 = h.read(this.wrapping_add(0x4e8));
    if count != 0 {
        let inline = this.wrapping_add(0x4ec);
        let n = count & 0x7fff_ffff;
        let data = if n > 1 {
            h.read(inline)
        } else if n != 0 {
            inline
        } else {
            0
        };
        if data != inline && count >> 31 == 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
        }
        h.write(this.wrapping_add(0x4e8), 0u32);
    }
    h.call_sym(FMV_TERMINATE, Conv::Thiscall(this), &[]);
    h.call_sym(
        SAMPLER_RELEASE,
        Conv::Thiscall(this.wrapping_add(0x364)),
        &[],
    );
    0
}

/// `g_WaitFramesByTime` (`0x0112a008`).
pub const WAIT_FRAMES_BY_TIME: Global<u32> = Global::new(0x0112_a008);
/// `g_PhyreScene` (`0x00ccb9d8`).
pub const PHYRE_SCENE: Global<u32> = Global::new(0x00cc_b9d8);
/// `g_FMVSkipped` (`0x00cded31`, a byte).
pub const FMV_SKIPPED: Global<u8> = Global::new(0x00cd_ed31);
/// `inputIsPressButton(mask)` (cdecl).
pub const IS_PRESS_BUTTON: u32 = 0x0063_0cf0;
/// `0x006e78a0(skip)` (thiscall): shows the skip prompt.
pub const SKIP_PROMPT_SHOW: u32 = 0x006e_78a0;
/// `0x006e78c0(skip)` (thiscall): hides it.
pub const SKIP_PROMPT_HIDE: u32 = 0x006e_78c0;
/// `Phyre_PVideo_PVideoPlaybackWin32_getEndTime(playback)` (thiscall;
/// `edx:eax`).
pub const GET_END_TIME: u32 = 0x00a2_7c40;
/// `PVideoPlaybackWin32_seek(playback; lo, hi)` (thiscall, ret 8).
pub const SEEK: u32 = 0x00a2_7e10;

/// `PhyFMVPlayerManager_handleInput(this; unused)` (`0x006d9460`,
/// thiscall, ret 4; from `graphicVideoUpdate`): while a skippable movie
/// plays (bytes `+0x6d2` set, `+0x74e` clear, `+0x6d0` set, frame waits
/// by time): a first press of button 0x800 (with `+0x74d`) arms the skip
/// (`+0x74c`) and shows the prompt (the scene's `+0x10de0`); once armed,
/// button 0x80 skips: the prompt hidden and its `+4` 0.0, `+0x6e0` and
/// `+0x6d8` 0xfffe, the playback (`+0x38`) seeked to its end,
/// `g_FMVSkipped` and byte `+0x710` set, `+0x6d0` cleared unless `+0x720`,
/// the skip disarmed. The original's `eax` is whatever the last call left
/// (no caller reads it).
pub fn handle_input(h: &mut impl Host, this: u32) {
    if h.read::<u8>(this.wrapping_add(0x6d2)) == 0
        || h.read::<u8>(this.wrapping_add(0x74e)) != 0
        || h.read::<u8>(this.wrapping_add(0x6d0)) == 0
        || WAIT_FRAMES_BY_TIME.get(h) != 1
    {
        return;
    }
    if h.read::<u8>(this.wrapping_add(0x74c)) == 0 {
        if h.read::<u8>(this.wrapping_add(0x74d)) != 0 {
            let armed = h.call_sym(IS_PRESS_BUTTON, Conv::Cdecl, &[0x800]) != 0;
            h.write(this.wrapping_add(0x74c), u8::from(armed));
            if armed {
                let prompt = skip_prompt(h);
                h.call_sym(SKIP_PROMPT_SHOW, Conv::Thiscall(prompt), &[]);
            }
        }
        if h.read::<u8>(this.wrapping_add(0x74c)) == 0 {
            return;
        }
    }
    if h.call_sym(IS_PRESS_BUTTON, Conv::Cdecl, &[0x80]) == 0 {
        return;
    }
    let prompt = skip_prompt(h);
    h.call_sym(SKIP_PROMPT_HIDE, Conv::Thiscall(prompt), &[]);
    let prompt = skip_prompt(h);
    h.write(prompt.wrapping_add(4), 0u32);
    h.write(this.wrapping_add(0x6e0), 0xfffeu32);
    h.write(this.wrapping_add(0x6d8), 0xfffeu32);
    let playback = this.wrapping_add(0x38);
    let end = h.call_sym_u64(GET_END_TIME, Conv::Thiscall(playback), &[]);
    #[allow(clippy::cast_possible_truncation)]
    let (lo, hi) = (end as u32, (end >> 32) as u32);
    h.call_sym(SEEK, Conv::Thiscall(playback), &[lo, hi]);
    FMV_SKIPPED.set(h, 1);
    h.write(this.wrapping_add(0x710), 1u8);
    if h.read::<u8>(this.wrapping_add(0x720)) == 0 {
        h.write(this.wrapping_add(0x6d0), 0u8);
    }
    h.write(this.wrapping_add(0x74c), 0u8);
}

/// The scene's skip prompt (`[g_PhyreScene + 0x10de0]`).
fn skip_prompt(h: &impl Host) -> u32 {
    let scene = PHYRE_SCENE.get(h);
    h.read(scene.wrapping_add(0x1_0de0))
}
