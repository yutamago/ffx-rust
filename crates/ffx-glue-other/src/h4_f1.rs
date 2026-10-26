//! Continuation round 4 of list `h4-glue-other`, batch 1: the FMOD
//! voice's start and restart, the sound loop, the sound effects' whole
//! volume and project unload, a sound event's volume fade; the menu
//! manager's instance setup, its setup of every instance and its buffer
//! chain swap.
//!
//! As in [`crate::h4_e2`]: the x87 at 53 bits, its comparisons as plain
//! `f64` ones, FMOD through its import thunks with `ERRCHECK`. The clock
//! (`graphicGetTimeInMicroseconds`) and `MenuManager_MenuInstanceSetup`
//! return 64 bits in `edx:eax` ([`Host::call_u64`]).

// `fcom`/`fucom` compare exactly, as `==` does.
#![allow(clippy::float_cmp)]

use ffx_bridge::x87::{div, load, store, sub};
use ffx_graphic::host::{Conv, Host};

use crate::frames::{ERRCHECK, FMOD_GET_STATE, FMOD_VOICE_FILE, OPERATOR_DELETE};
use crate::h4_c2::{
    FREE_LIST_ALLOCATE_BLOCK, FREE_LIST_DEALLOCATE, FREE_LIST_RESET, TREE_ORPHAN_ALL,
    TREE_POST_INSERT, walk_tree,
};
use crate::h4_c3::{EVENT_SET_CALLBACK, TIME_US};
use crate::h4_d2::strcmp;
use crate::h4_e1::pass;
use crate::h4_e2::VOICE_VOLUME_DIRECT;
use crate::h4_e3::{EVENT_CALLBACK, EVENT_START, FULL_VOLUME, SFX_FILE};
use crate::r3::VFX_INT_TREE_INSERT;

fn errcheck(h: &mut impl Host, r: u32, file: u32, line: u32) {
    let file = h.rebase(file);
    h.call_sym(ERRCHECK, Conv::Cdecl, &[r, file, line]);
}

// ---- the voice

/// `0x00cec184`: the voice's loop flag (1: a started voice is not marked
/// playing).
pub const VOICE_NO_PLAY: u32 = 0x00ce_c184;
/// Bytes of [`voice_start`]'s and [`voice_restart`]'s frame: the state
/// word at `ebp - 4`.
pub const VOICE_FRAME: u32 = 4;

/// Hands the voice's event (`+0x14`) the callback (user data `+0x40`) and
/// starts it (`ERRCHECK` lines `lines`), notes the start time (`+0x70`,
/// 64-bit µs), keeps the full volume (`0x00b4f5e4`) in `+0x50` and sets it
/// (`FmodVoice_volumeDirect(volume, 0)`; 0.0 when `+0x80 == 1`).
fn start_event(h: &mut impl Host, this: u32, lines: [u32; 2]) {
    let ev: u32 = h.read(this.wrapping_add(0x14));
    let callback = h.rebase(EVENT_CALLBACK);
    let r = h.call_sym(
        EVENT_SET_CALLBACK,
        Conv::Stdcall,
        &[ev, callback, this.wrapping_add(0x40)],
    );
    errcheck(h, r, FMOD_VOICE_FILE, lines[0]);
    let ev: u32 = h.read(this.wrapping_add(0x14));
    let r = h.call_sym(EVENT_START, Conv::Stdcall, &[ev]);
    errcheck(h, r, FMOD_VOICE_FILE, lines[1]);
    let now = h.call_sym_u64(TIME_US, Conv::Cdecl, &[]);
    let full = pass(h.read(h.rebase(FULL_VOLUME)));
    h.write(this.wrapping_add(0x50), full);
    h.write(this.wrapping_add(0x70), now as u32);
    h.write(this.wrapping_add(0x74), (now >> 32) as u32);
    let volume = if h.read::<u32>(this.wrapping_add(0x80)) == 1 {
        0
    } else {
        full
    };
    h.call_sym(VOICE_VOLUME_DIRECT, Conv::Thiscall(this), &[volume, 0]);
}

/// `FmodVoice_start(this; pan, volume)` (thiscall, ret 8; both arguments
/// unread; `frame` the state word): without an event (`+0x14`) `+0x40`
/// and `+0x44` are cleared. Else its state (line 0x37e) decides: 1 starts
/// it ([`start_event`], lines 0x383, 0x386) and clears `+0x5c`, `+0x60`,
/// `+0x64`; anything else sets `+0x58` (start pending). Then `+0x40` is 1
/// unless the loop flag `0x00cec184` is 1 (0 then) and `+0x44` 0.
/// Returns 0.
pub fn voice_start(h: &mut impl Host, frame: u32, this: u32) -> u32 {
    // the frame word is allocated with `push ecx`
    h.write(frame, this);
    let ev: u32 = h.read(this.wrapping_add(0x14));
    if ev == 0 {
        h.write(this.wrapping_add(0x40), 0u32);
        h.write(this.wrapping_add(0x44), 0u32);
        return 0;
    }
    let r = h.call_sym(FMOD_GET_STATE, Conv::Stdcall, &[ev, frame]);
    errcheck(h, r, FMOD_VOICE_FILE, 0x37e);
    if h.read::<u32>(frame) == 1 {
        start_event(h, this, [0x383, 0x386]);
        for off in [0x5c, 0x60, 0x64] {
            h.write(this.wrapping_add(off), 0u32);
        }
    } else {
        h.write(this.wrapping_add(0x58), 1u32);
    }
    let playing = u32::from(h.read::<u32>(h.rebase(VOICE_NO_PLAY)) != 1);
    h.write(this.wrapping_add(0x40), playing);
    h.write(this.wrapping_add(0x44), 0u32);
    0
}

/// `FmodVoice_restart(this)` (thiscall; `frame` the state word): a voice
/// with an event (`+0x14`) and a pending start (`+0x58`) asks its state
/// (unchecked call, then `ERRCHECK` line 0x334). On an error, from the
/// fifth retry (`+0x64 >= 5`, signed) the event is released (with a group
/// `+0x10`: its vtable `+8` (group, event, 1), stdcall with `ecx` the
/// group too, line 0x33c) and the voice
/// cleared (`+0x14`, `+0x54`, `+0x40`, `+0x44`, `+0x48`, `+0x10`, `+0x6c`,
/// `+0x70`, `+0x74`), then the retry state below. State 1 starts it
/// ([`start_event`], lines 0x354, 0x357) and clears the retry state
/// (`+0x58`, `+0x64`, `+0x60`, `+0x5c`). Returns 0.
pub fn voice_restart(h: &mut impl Host, frame: u32, this: u32) -> u32 {
    // the frame word is allocated with `push ecx`
    h.write(frame, this);
    let ev: u32 = h.read(this.wrapping_add(0x14));
    if ev == 0 || h.read::<u32>(this.wrapping_add(0x58)) == 0 {
        return 0;
    }
    let r = h.call_sym(FMOD_GET_STATE, Conv::Stdcall, &[ev, frame]);
    errcheck(h, r, FMOD_VOICE_FILE, 0x334);
    if r != 0 {
        if h.read::<u32>(this.wrapping_add(0x64)).cast_signed() < 5 {
            return 0;
        }
        let ev: u32 = h.read(this.wrapping_add(0x14));
        let group: u32 = h.read(this.wrapping_add(0x10));
        if ev != 0 && group != 0 {
            let vtable: u32 = h.read(group);
            let release: u32 = h.read(vtable.wrapping_add(8));
            // stdcall with `this` on the stack; `ecx` holds it too
            let r = h.call(release, Conv::Thiscall(group), &[group, ev, 1]);
            errcheck(h, r, FMOD_VOICE_FILE, 0x33c);
        }
        for off in [0x14, 0x54, 0x40, 0x44, 0x48, 0x10, 0x6c, 0x70, 0x74] {
            h.write(this.wrapping_add(off), 0u32);
        }
    } else {
        if h.read::<u32>(frame) != 1 {
            return 0;
        }
        start_event(h, this, [0x354, 0x357]);
    }
    for off in [0x58, 0x64, 0x60, 0x5c] {
        h.write(this.wrapping_add(off), 0u32);
    }
    0
}

// ---- the sound loop

/// `0x00cec168`: the sound loop's last run (64-bit µs).
pub const LAST_LOOP: u32 = 0x00ce_c168;
/// µs per second (`0x00b3ecc0`, f64).
pub const US_PER_S: u32 = 0x00b3_ecc0;
/// `FMOD::EventSystem::update`'s thunk (stdcall).
pub const SYSTEM_UPDATE: u32 = 0x0094_8c82;
/// The file name `FfxFmod` passes to `ERRCHECK` (`0x00b4d268`).
pub const FFX_FMOD_FILE: u32 = 0x00b4_d268;
/// `FmodVoice_updateState(this; system)` (thiscall).
pub const VOICE_UPDATE_STATE: u32 = 0x0070_b750;
/// `FmodMusic_loopVolumeTranslate(this)` (thiscall).
pub const MUSIC_LOOP_VOLUME: u32 = 0x0070_8e50;
/// `FmodVoice_loopVolumeTranslate(this)` (thiscall).
pub const VOICE_LOOP_VOLUME: u32 = 0x0070_acf0;
/// `FmodVoice_restart(this)` (thiscall).
pub const VOICE_RESTART: u32 = 0x0070_b160;
/// `FmodMusic_restart(this; i)` (thiscall).
pub const MUSIC_RESTART: u32 = 0x0070_9270;
/// `FmodSfx_update(this; dt)` (thiscall, `dt` an f32).
pub const SFX_UPDATE: u32 = 0x0070_dec0;

/// `FfxFmod_soundLoop(this)` (thiscall): once more than 33 ms (0x80e8 µs,
/// unsigned 64-bit difference) passed since the last run (`0x00cec168`),
/// the elapsed time becomes seconds (`u64` to the x87, an f32, divided by
/// `0x00b3ecc0`, an f32), the last run is now; then the event system
/// (`+4`) updates (line 0x121) when there is one, the voice (`+0x10`)
/// updates its state with the system, the music (`+0xc`) and the voice
/// run their fades, the voice its restart, the music its restart of the
/// current track (`+0x18`), and the sound effects (`+0x14`) update with
/// the seconds. Returns 0.
pub fn sound_loop(h: &mut impl Host, this: u32) -> u32 {
    let now = h.call_sym_u64(TIME_US, Conv::Cdecl, &[]);
    let at = h.rebase(LAST_LOOP);
    let last = u64::from(h.read::<u32>(at)) | (u64::from(h.read::<u32>(at.wrapping_add(4))) << 32);
    // the original also tests `now < last` when the difference is small:
    // a small unsigned difference means `now >= last`, so that never runs
    let elapsed = now.wrapping_sub(last);
    if elapsed <= 0x80e8 {
        return 0;
    }
    // `fild` of the low 63 bits plus 2^63 for the top bit, rounded once
    // to 53 bits: the `u64`'s nearest double
    #[allow(clippy::cast_precision_loss)]
    let seconds = store(elapsed as f64);
    let us: f64 = h.read(h.rebase(US_PER_S));
    let dt = store(div(load(seconds), us));
    h.write(at, now as u32);
    h.write(at.wrapping_add(4), (now >> 32) as u32);
    let system: u32 = h.read(this.wrapping_add(4));
    if system != 0 {
        let r = h.call_sym(SYSTEM_UPDATE, Conv::Stdcall, &[system]);
        errcheck(h, r, FFX_FMOD_FILE, 0x121);
    }
    let system: u32 = h.read(this.wrapping_add(4));
    let voice: u32 = h.read(this.wrapping_add(0x10));
    h.call_sym(VOICE_UPDATE_STATE, Conv::Thiscall(voice), &[system]);
    let music: u32 = h.read(this.wrapping_add(0xc));
    h.call_sym(MUSIC_LOOP_VOLUME, Conv::Thiscall(music), &[]);
    let voice: u32 = h.read(this.wrapping_add(0x10));
    h.call_sym(VOICE_LOOP_VOLUME, Conv::Thiscall(voice), &[]);
    let voice: u32 = h.read(this.wrapping_add(0x10));
    h.call_sym(VOICE_RESTART, Conv::Thiscall(voice), &[]);
    let music: u32 = h.read(this.wrapping_add(0xc));
    let track: u32 = h.read(music.wrapping_add(0x18));
    h.call_sym(MUSIC_RESTART, Conv::Thiscall(music), &[track]);
    let sfx: u32 = h.read(this.wrapping_add(0x14));
    h.call_sym(SFX_UPDATE, Conv::Thiscall(sfx), &[pass(dt)]);
    0
}

// ---- the sound effects

/// `FmodSfx_anyEventFlag_0070ed30(list)` (thiscall, `al`).
pub const SFX_ANY_EVENT_FLAG: u32 = 0x0070_ed30;
/// `SfxEvent_setEntireVolume_0070ee90(event; volume)` (thiscall, ret 4):
/// `+0x24` = volume, then `SfxEvent_applyVolume(+0x20)`.
pub const SFX_EVENT_ENTIRE_VOLUME: u32 = 0x0070_ee90;

/// Hands `[this + 0xd0]` to `event` when one of its instances is flagged.
fn entire_volume_to(h: &mut impl Host, this: u32, event: u32) {
    if h.call_sym(SFX_ANY_EVENT_FLAG, Conv::Thiscall(event), &[]) as u8 == 0 {
        return;
    }
    let v = pass(h.read(this.wrapping_add(0xd0)));
    h.call_sym(SFX_EVENT_ENTIRE_VOLUME, Conv::Thiscall(event), &[v]);
}

/// `FmodSfx_setEntireVolume(this; volume)` (thiscall, ret 4): `volume`
/// clamped to 0..1 (NaN kept); unchanged from `+0xd0` (equal, not NaN)
/// it stops. Else it is stored and handed to every event (`+0x18`, count
/// `+0x10`, unsigned; null ones skipped) of every sound of the tree at
/// `+0x64` and to the stream event `+0xc8`, each only when one of its
/// instances is flagged (`FmodSfx_anyEventFlag`). Its callers read no
/// result.
pub fn sfx_set_entire_volume(h: &mut impl Host, this: u32, volume: u32) {
    let mut bits = pass(volume);
    let v = load(bits);
    if v < 0.0 {
        bits = 0;
    } else if v > 1.0 {
        bits = 1.0f32.to_bits();
    }
    let cur = load(h.read(this.wrapping_add(0xd0)));
    if load(bits) == cur {
        return;
    }
    h.write(this.wrapping_add(0xd0), bits);
    walk_tree(h, this.wrapping_add(0x64), |h, node| {
        let mut i = 0u32;
        while i < h.read::<u32>(node.wrapping_add(0x10)) {
            let events: u32 = h.read(node.wrapping_add(0x18));
            let event: u32 = h.read(events.wrapping_add(4 * i));
            if event != 0 {
                entire_volume_to(h, this, event);
            }
            i += 1;
        }
    });
    let stream: u32 = h.read(this.wrapping_add(0xc8));
    if stream != 0 {
        entire_volume_to(h, this, stream);
    }
}

/// `SfxBank_dtor_SfxBank(bank)` (thiscall).
pub const SFX_BANK_DTOR: u32 = 0x0070_f930;
/// `SfxBank_getNumPlayingEvents(bank)` (thiscall).
pub const SFX_BANK_PLAYING: u32 = 0x0070_fb20;

/// Destroys and frees the bank of `node` (`+0x10`) when set.
fn delete_bank(h: &mut impl Host, node: u32) {
    let bank: u32 = h.read(node.wrapping_add(0x10));
    if bank != 0 {
        h.call_sym(SFX_BANK_DTOR, Conv::Thiscall(bank), &[]);
        h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[bank]);
    }
}

/// `FmodSfx_unloadProjects(this; all, _)` (thiscall, ret 8): with `all`
/// (its low byte) every bank of the tree at `+4` is destroyed and the
/// tree emptied (`orphanAllContents`, its free list `+0x1c` reset). Then
/// the tree at `+0x34`: up to the first bank with no playing events,
/// which is destroyed, every bank is moved to the tree at `+0x94` (a node
/// from its free list `+0xac`, refilled once by `allocateNewBlock(+0xb4)`
/// when empty, nothing when still empty; key `+0xc` and bank `+0x10`;
/// `vfxIntTree_Insert`, then `postInsertFinalize`, or the node given back
/// when the key is there); after it every bank is moved without the
/// check. The tree at `+0x34` is emptied as the first. Returns the last
/// free-list reset's `eax`.
pub fn sfx_unload_projects(h: &mut impl Host, this: u32, all: u32) -> u32 {
    if all & 0xff != 0 {
        let tree = this.wrapping_add(4);
        walk_tree(h, tree, |h, node| delete_bank(h, node));
        h.call_sym(TREE_ORPHAN_ALL, Conv::Thiscall(tree), &[]);
        h.call_sym(
            FREE_LIST_RESET,
            Conv::Thiscall(tree.wrapping_add(0x18)),
            &[],
        );
    }
    let tree = this.wrapping_add(0x34);
    let mut deleted = false;
    walk_tree(h, tree, |h, node| {
        let bank: u32 = h.read(node.wrapping_add(0x10));
        if !deleted && h.call_sym(SFX_BANK_PLAYING, Conv::Thiscall(bank), &[]) == 0 {
            delete_bank(h, node);
            deleted = true;
            return;
        }
        let kept = this.wrapping_add(0x94);
        let list = kept.wrapping_add(0x18);
        let mut n: u32 = h.read(list);
        if n == 0 {
            let size: u32 = h.read(kept.wrapping_add(0x20));
            h.call_sym(FREE_LIST_ALLOCATE_BLOCK, Conv::Thiscall(list), &[size]);
            n = h.read(this.wrapping_add(0x94 + 0x18));
            if n == 0 {
                return;
            }
        }
        let next: u32 = h.read(n);
        h.write(list, next);
        h.write(n, n);
        h.write(n.wrapping_add(4), n);
        h.write(n.wrapping_add(8), n);
        let key: u32 = h.read(node.wrapping_add(0xc));
        h.write(n.wrapping_add(0xc), key);
        h.write(n.wrapping_add(0x10), bank);
        if h.call_sym(VFX_INT_TREE_INSERT, Conv::Thiscall(kept), &[n]) == 0 {
            h.call_sym(
                TREE_POST_INSERT,
                Conv::Thiscall(this.wrapping_add(0x94)),
                &[n],
            );
        } else {
            h.call_sym(
                FREE_LIST_DEALLOCATE,
                Conv::Thiscall(this.wrapping_add(0xac)),
                &[n],
            );
        }
    });
    h.call_sym(TREE_ORPHAN_ALL, Conv::Thiscall(tree), &[]);
    h.call_sym(
        FREE_LIST_RESET,
        Conv::Thiscall(tree.wrapping_add(0x18)),
        &[],
    )
}

/// `SfxEvent_setVolume(event; volume)` (thiscall, ret 4).
pub const SFX_EVENT_SET_VOLUME: u32 = 0x0070_f1a0;
/// `SfxEvent_volumeFrom127_0070ea20(v)` (ret 4, `ST0`; called with `ecx`
/// the event).
pub const SFX_VOLUME_FROM_127: u32 = 0x0070_ea20;
/// `FMOD::Event::getVolume`'s thunk (stdcall).
pub const EVENT_GET_VOLUME: u32 = 0x0094_8cb8;
/// `operator new` (`0x00630310`).
pub const OPERATOR_NEW: u32 = 0x0063_0310;
/// The frames per second of [`sfx_event_set_volume2`]'s fade (`0x00b922c8`,
/// f64).
pub const FADE_FPS: u32 = 0x00b9_22c8;
/// Bytes of [`sfx_event_set_volume2`]'s frame: the current volume at
/// `ebp - 0x10`.
pub const SET_VOLUME2_FRAME: u32 = 4;

/// `SfxEvent_setVolume2(this; volume, frames)` (thiscall, ret 8; `frame`
/// the current-volume local; without its SEH frame): `frames` 0 is
/// `SfxEvent_setVolume(volume)`, its `eax` returned. Else the old fade
/// (`+0x18`) is freed and a new one of 16 bytes made (`operator new`;
/// none: `+0x18` 0): its target (`volumeFrom127(volume)`, an f32), its
/// start (the current volume of the first instance (`+8`) when its flag
/// byte `+4` is set, line 0x10f; else 0.0), its seconds (`frames`, an
/// f32, over `0x00b922c8`) and its step ((target - start) / seconds).
/// Its callers read no other result.
pub fn sfx_event_set_volume2(
    h: &mut impl Host,
    frame: u32,
    this: u32,
    volume: u32,
    frames: u32,
) -> Option<u32> {
    if frames == 0 {
        return Some(h.call_sym(SFX_EVENT_SET_VOLUME, Conv::Thiscall(this), &[volume]));
    }
    let old: u32 = h.read(this.wrapping_add(0x18));
    if old != 0 {
        h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[old]);
    }
    let fade = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x10]);
    if fade == 0 {
        h.write(this.wrapping_add(0x18), 0u32);
        return Some(0);
    }
    h.write(frame, 0u32);
    let first: u32 = h.read(this.wrapping_add(8));
    if h.read::<u8>(first.wrapping_add(4)) != 0 {
        let ev: u32 = h.read(first);
        let r = h.call_sym(EVENT_GET_VOLUME, Conv::Stdcall, &[ev, frame]);
        errcheck(h, r, SFX_FILE, 0x10f);
    }
    let target = h
        .call_sym_st0(SFX_VOLUME_FROM_127, Conv::Thiscall(this), &[volume])
        .to_bits();
    h.write(fade, target);
    let start = pass(h.read(frame));
    h.write(fade.wrapping_add(4), start);
    let fps: f64 = h.read(h.rebase(FADE_FPS));
    let seconds = store(div(load(store(f64::from(frames.cast_signed()))), fps));
    h.write(fade.wrapping_add(8), seconds);
    let start = load(h.read(frame));
    let step = store(div(sub(load(target), start), load(seconds)));
    h.write(fade.wrapping_add(0xc), step);
    h.write(this.wrapping_add(0x18), fade);
    None
}

// ---- the menu manager

/// `MenuDynamicGeometry_MenuDynamicGeometry(this)` (thiscall).
pub const MENU_GEOMETRY_CTOR: u32 = 0x0069_e320;
/// `MenuDynamicGeometry_setMeshWorldMatrix(this)` (thiscall).
pub const MENU_GEOMETRY_SET_MATRIX: u32 = 0x006a_2e60;
/// `MenuManager_MenuInstanceSetup(this; rec, name, big)` (thiscall).
pub const MENU_INSTANCE_SETUP: u32 = 0x0067_d760;
/// `MenuManager_MenuInstanceUpdateMaterialShader(this; slot, rec)`
/// (thiscall).
pub const MENU_UPDATE_MATERIAL: u32 = 0x0067_d920;
/// `MenuManager_gatherMenuBuffers_0067b530(this; which)` (thiscall).
pub const MENU_GATHER_BUFFERS: u32 = 0x0067_b530;
/// `g_PhyreScene`.
pub const PHYRE_SCENE: u32 = 0x00cc_b9d8;
/// `"RealFrameBuffer"` (`0x00b45b2c`).
pub const REAL_FRAME_BUFFER: u32 = 0x00b4_5b2c;
/// `g_StrNoTexture` (`"NoTexture"`).
pub const NO_TEXTURE: u32 = 0x00b4_1b50;
/// The small and big instances' texture names (`0x00c33610`,
/// `0x00c33614`).
pub const MENU_NAMES: [u32; 2] = [0x00c3_3610, 0x00c3_3614];

/// `MenuManager_MenuInstanceSetup(this; rec, name, big)` (thiscall, ret
/// 0xc; without its SEH frame): the render target by `name`:
/// `"RealFrameBuffer"` the scene's `+0xfa88`; `"NoTexture"` `+0xfa80`,
/// anything else `+0xfa84`, unless the scene's byte `+0xfa9c` (`+0xfa9d`)
/// is set, which skips the material. The next geometry slot (`+0x2ec30`,
/// 0x3c bytes from `+0x5bf0`) is constructed and counted. The record's
/// vertex info (`+0x94`) lends its counts (`+4`, `+8`: 4 and 6, big 0x30
/// and 0x48) while the geometry takes the record (`+0x24`) and, unless
/// skipped, its material (vtable `+4` (target, scene + 0x1284)) and world
/// matrix; `+0xb4` gets the info's `+0x28` (16 bits). Returns the
/// geometry, sign-extended to 64 bits (`cdq`).
pub fn menu_instance_setup(h: &mut impl Host, this: u32, rec: u32, name: u32, big: u32) -> u64 {
    let scene: u32 = h.read(h.rebase(PHYRE_SCENE));
    let mut target = 0u32;
    let skip: u8;
    if strcmp(h, name, h.rebase(REAL_FRAME_BUFFER)) == 0 {
        skip = 0;
        target = h.read(scene.wrapping_add(0xfa88));
    } else if strcmp(h, name, h.rebase(NO_TEXTURE)) == 0 {
        skip = h.read(scene.wrapping_add(0xfa9c));
        if skip == 0 {
            target = h.read(scene.wrapping_add(0xfa80));
        }
    } else {
        skip = h.read(scene.wrapping_add(0xfa9d));
        if skip == 0 {
            target = h.read(scene.wrapping_add(0xfa84));
        }
    }
    let n: u32 = h.read(this.wrapping_add(0x2ec30));
    let slot = this.wrapping_add(n.wrapping_mul(15).wrapping_add(0x16fc).wrapping_mul(4));
    let geom = if slot == 0 {
        0
    } else {
        h.call_sym(MENU_GEOMETRY_CTOR, Conv::Thiscall(slot), &[])
    };
    let n: u32 = h.read(this.wrapping_add(0x2ec30));
    h.write(this.wrapping_add(0x2ec30), n.wrapping_add(1));
    let scene: u32 = h.read(h.rebase(PHYRE_SCENE));
    let info: u32 = h.read(rec.wrapping_add(0x94));
    let count4: u32 = h.read(info.wrapping_add(4));
    let count8: u32 = h.read(info.wrapping_add(8));
    let id: u16 = h.read(info.wrapping_add(0x28));
    h.write(rec.wrapping_add(0xb4), id);
    let (a, b) = if big == 0 { (4u32, 6u32) } else { (0x30, 0x48) };
    h.write(info.wrapping_add(4), a);
    let info: u32 = h.read(rec.wrapping_add(0x94));
    h.write(info.wrapping_add(8), b);
    h.write(geom.wrapping_add(0x24), rec);
    if skip == 0 {
        let vtable: u32 = h.read(geom);
        let set: u32 = h.read(vtable.wrapping_add(4));
        h.call(
            set,
            Conv::Thiscall(geom),
            &[target, scene.wrapping_add(0x1284)],
        );
        h.call_sym(MENU_GEOMETRY_SET_MATRIX, Conv::Thiscall(geom), &[]);
    }
    let info: u32 = h.read(rec.wrapping_add(0x94));
    h.write(info.wrapping_add(4), count4);
    let info: u32 = h.read(rec.wrapping_add(0x94));
    h.write(info.wrapping_add(8), count8);
    i64::from(geom.cast_signed()).cast_unsigned()
}

/// `MenuManager_SetUpAllMenuInstance(this)` (thiscall): the geometry
/// count (`+0x2ec30`) restarts; each of the four instance lists
/// (16-byte records: `+0x2bd4` 0x258 small, `+0x2bdc` 0x320 big,
/// `+0x582c` 0x258 small, `+0x5834` 0x320 big) gets, per record,
/// `MenuInstanceSetup(this; *rec, name, big)` of the list's first
/// record (`*(+0x15e4)` small, `*(+0x15ec)` big) and its name
/// (`0x00c33610`, `0x00c33614`): its 64-bit result (`+0`, `+4`), the
/// name read again (`+8`) and 0 (`+0xc`). Then byte `+0x5b9c` is 1.
/// Returns the last setup's `edx:eax`.
pub fn menu_set_up_all(h: &mut impl Host, this: u32) -> u64 {
    h.write(this.wrapping_add(0x2ec30), 0u32);
    let mut last = 0u64;
    for (list, bytes, big) in [
        (0x2bd4u32, 0x2580u32, 0u32),
        (0x2bdc, 0x3200, 1),
        (0x582c, 0x2580, 0),
        (0x5834, 0x3200, 1),
    ] {
        let first = if big == 0 { 0x15e4u32 } else { 0x15ec };
        let name_at = h.rebase(if big == 0 {
            MENU_NAMES[0]
        } else {
            MENU_NAMES[1]
        });
        let mut k = 0u32;
        while k.cast_signed() < bytes.cast_signed() {
            let recs: u32 = h.read(this.wrapping_add(first));
            let rec: u32 = h.read(recs);
            let name: u32 = h.read(name_at);
            last = h.call_sym_u64(MENU_INSTANCE_SETUP, Conv::Thiscall(this), &[rec, name, big]);
            let e = h.read::<u32>(this.wrapping_add(list)).wrapping_add(k);
            let name: u32 = h.read(name_at);
            h.write(e, last as u32);
            h.write(e.wrapping_add(4), (last >> 32) as u32);
            h.write(e.wrapping_add(8), name);
            h.write(e.wrapping_add(0xc), 0u32);
            k += 0x10;
        }
    }
    h.write(this.wrapping_add(0x5b9c), 1u8);
    last
}

/// `MenuManager_SwapMenuBufferChain(this)` (thiscall): the first chain
/// (`+0x5b7c` 0: count `+0x2c50`, slots from `+0x15f0`, records of
/// `+0x2bd4`/`+0x2bdc`), else the second (count `+0x5840`, slots from
/// `+0x4248`, records of `+0x582c`/`+0x5834`). With a count: a pending
/// gather (`+0x5844`) runs (`gatherMenuBuffers(which)`) and is cleared;
/// then every slot `i` takes the next record of its kind (its record
/// `*(slot - 0x15f0)`'s `+0xb0` 0: small, else big), stored
/// in the slot and handed on (`MenuInstanceUpdateMaterialShader(record,
/// rec)`). Its callers read no result.
pub fn menu_swap_buffer_chain(h: &mut impl Host, this: u32) {
    let second = h.read::<u32>(this.wrapping_add(0x5b7c)) != 0;
    let (count_at, slots, small, big, which) = if second {
        (0x5840u32, 0x4248u32, 0x582cu32, 0x5834u32, 1u32)
    } else {
        (0x2c50, 0x15f0, 0x2bd4, 0x2bdc, 0)
    };
    if h.read::<u32>(this.wrapping_add(count_at)) == 0 {
        return;
    }
    if h.read::<u32>(this.wrapping_add(0x5844)) != 0 {
        h.call_sym(MENU_GATHER_BUFFERS, Conv::Thiscall(this), &[which]);
        h.write(this.wrapping_add(0x5844), 0u32);
    }
    let (mut ks, mut kb) = (0u32, 0u32);
    let mut i = 0u32;
    while i < h.read::<u32>(this.wrapping_add(count_at)) {
        let slot = this.wrapping_add(slots).wrapping_add(4 * i);
        let rec: u32 = h.read(slot.wrapping_sub(0x15f0));
        let record = if h.read::<u32>(rec.wrapping_add(0xb0)) == 0 {
            let r = h.read::<u32>(this.wrapping_add(small)).wrapping_add(ks);
            ks = ks.wrapping_add(0x10);
            r
        } else {
            let r = h.read::<u32>(this.wrapping_add(big)).wrapping_add(kb);
            kb = kb.wrapping_add(0x10);
            r
        };
        h.write(slot, record);
        h.call_sym(MENU_UPDATE_MATERIAL, Conv::Thiscall(this), &[record, rec]);
        i += 1;
    }
}
