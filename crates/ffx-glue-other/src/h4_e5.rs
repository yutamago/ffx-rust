//! Continuation round 3 of list `h4-glue-other`, batch 5: the FMOD
//! music's start, read, cross-play, data send and background-music
//! switch.
//!
//! As in [`crate::h4_e2`]: the x87 at 53 bits, its comparisons as plain
//! `f64` ones, FMOD through its import thunks (and the event project's
//! vtable) with `ERRCHECK`.

use ffx_bridge::Global;
use ffx_bridge::x87::load;
use ffx_graphic::host::{Conv, Host};

use crate::frames::{ERRCHECK, FMOD_GET_STATE};
use crate::h4_c3::EVENT_SET_CALLBACK;
use crate::h4_e1::pass;
use crate::h4_e2::{MUSIC_FADE_OUT, MUSIC_FILE, MUSIC_VOLUME_DIRECT, TRACK, TRACKS};
use crate::h4_e3::{
    EVENT_CALLBACK, EVENT_START, FULL_VOLUME, MUSIC_CLEAR, MUSIC_STOP, MUSIC_VOLUME_TRANSLATE,
};
use crate::sound::EVENT_SET_PAUSED;

/// Track `i`'s record (`[this + 0x10] + 0x3c i`, read again each time).
fn track(h: &impl Host, this: u32, i: u32) -> u32 {
    h.read::<u32>(this.wrapping_add(0x10))
        .wrapping_add(i.wrapping_mul(TRACK))
}

fn errcheck(h: &mut impl Host, r: u32, line: u32) {
    let file = h.rebase(MUSIC_FILE);
    h.call_sym(ERRCHECK, Conv::Cdecl, &[r, file, line]);
}

/// Clears track `i`'s fade (`+0x28` 0, `+0x2c`/`+0x30` 0.0, `+0x24` 0).
fn clear_fade(h: &mut impl Host, this: u32, i: u32) {
    for off in [0x28, 0x2c, 0x30, 0x24] {
        let e = track(h, this, i);
        h.write(e.wrapping_add(off), 0u32);
    }
}

/// After a (re)start: muted (`this + 0x24 == 1`), a queued fade
/// (`+0x18`, `+0x1c`) is dropped and the volume set to 0; a queued fade
/// still there starts from 0 (`volumeDirect(i, 0, 1)`,
/// `volumeTranslate(i, frames, 0x7f)`) and is cleared.
fn queued_fade(h: &mut impl Host, this: u32, i: u32) {
    if h.read::<u32>(this.wrapping_add(0x24)) == 1 {
        let e = track(h, this, i);
        h.write(e.wrapping_add(0x18), 0u32);
        let e = track(h, this, i);
        h.write(e.wrapping_add(0x1c), 0u32);
        h.call_sym(MUSIC_VOLUME_DIRECT, Conv::Thiscall(this), &[i, 0, 0]);
    }
    let e = track(h, this, i);
    if h.read::<u32>(e.wrapping_add(0x18)) != 1 || h.read::<u32>(e.wrapping_add(0x1c)) == 0 {
        return;
    }
    h.call_sym(MUSIC_VOLUME_DIRECT, Conv::Thiscall(this), &[i, 0, 1]);
    let frames: u32 = h.read(track(h, this, i).wrapping_add(0x1c));
    h.call_sym(
        MUSIC_VOLUME_TRANSLATE,
        Conv::Thiscall(this),
        &[i, frames, 0x7f],
    );
    let e = track(h, this, i);
    h.write(e.wrapping_add(0x18), 0u32);
    let e = track(h, this, i);
    h.write(e.wrapping_add(0x1c), 0u32);
}

/// Gives track `i`'s event its callback (user data its `+0x20`) and
/// starts it (`ERRCHECK` lines `lines`), then `+0x34` the full volume,
/// `+0x20` 1.
fn start_event(h: &mut impl Host, this: u32, i: u32, lines: [u32; 2]) {
    let e = track(h, this, i);
    let ev: u32 = h.read(e);
    let callback = h.rebase(EVENT_CALLBACK);
    let r = h.call_sym(
        EVENT_SET_CALLBACK,
        Conv::Stdcall,
        &[ev, callback, e.wrapping_add(0x20)],
    );
    errcheck(h, r, lines[0]);
    let ev: u32 = h.read(track(h, this, i));
    let r = h.call_sym(EVENT_START, Conv::Stdcall, &[ev]);
    errcheck(h, r, lines[1]);
    let full = pass(h.read(h.rebase(FULL_VOLUME)));
    let e = track(h, this, i);
    h.write(e.wrapping_add(0x34), full);
    let e = track(h, this, i);
    h.write(e.wrapping_add(0x20), 1u32);
}

/// `FmodMusic_releaseOldMusic(this; keep)` (thiscall).
pub const RELEASE_OLD: u32 = 0x0070_91b0;
/// Bytes of [`music_start`]'s frame: its argument word, where the state
/// query writes.
pub const START_FRAME: u32 = 4;

/// `FmodMusic_start(this; i)` (thiscall, ret 4; `frame` the argument
/// word, holding `i` until the state query writes it): a loaded track
/// `i` (`<= 0xb5`; else `this + 0x20` is cleared) becomes the current one
/// (`this + 0x18`) after the others are released
/// (`FmodMusic_releaseOldMusic`) and its fade cleared. A playing track
/// (`+0x20 == 1`) only gets [`queued_fade`]. Else its state (line 0x1a1)
/// decides: 1 starts it (lines 0x1a5, 0x1a8; `this + 0x20` 0), bit 8 in
/// music mode 0x79 stops and starts it again (lines 0x1c2, 0x1c5; `+4`
/// 0), then [`queued_fade`]; anything else sets `this + 0x1c` (start
/// pending). Returns 0.
pub fn music_start(h: &mut impl Host, frame: u32, this: u32, i: u32) -> u32 {
    h.write(frame, i);
    if i > 0xb5 || h.read::<u32>(track(h, this, i)) == 0 {
        h.write(this.wrapping_add(0x20), 0u32);
        return 0;
    }
    h.call_sym(RELEASE_OLD, Conv::Thiscall(this), &[i]);
    h.write(this.wrapping_add(0x18), i);
    let e = track(h, this, i);
    if h.read::<u32>(e.wrapping_add(0x24)) == 1 {
        clear_fade(h, this, i);
    }
    let e = track(h, this, i);
    if h.read::<u32>(e.wrapping_add(0x20)) == 1 {
        queued_fade(h, this, i);
        return 0;
    }
    let ev: u32 = h.read(e);
    let r = h.call_sym(FMOD_GET_STATE, Conv::Stdcall, &[ev, frame]);
    errcheck(h, r, 0x1a1);
    let state: u32 = h.read(frame);
    if state == 1 {
        start_event(h, this, i, [0x1a5, 0x1a8]);
        h.write(this.wrapping_add(0x20), 0u32);
    } else if state & 8 != 0 && h.read::<u32>(this.wrapping_add(0x28)) == 0x79 {
        h.call_sym(MUSIC_STOP, Conv::Thiscall(this), &[i]);
        start_event(h, this, i, [0x1c2, 0x1c5]);
        let e = track(h, this, i);
        h.write(e.wrapping_add(4), 0u32);
    } else {
        h.write(this.wrapping_add(0x1c), 1u32);
        return 0;
    }
    queued_fade(h, this, i);
    0
}

/// `g_FmodMusicNoLoad` (`0x00cec174`): 1 skips the event loads.
pub const NO_LOAD: Global<u32> = Global::new(0x00ce_c174);
/// `FmodMusic_getState(i)` (cdecl): the event group index of track `i`.
pub const MUSIC_GROUP: u32 = 0x0070_8be0;
/// `__iob_func`'s import slot.
pub const IOB_FUNC: Global<u32> = Global::new(0x00b0_c350);
/// `fprintf`'s import slot.
pub const FPRINTF: Global<u32> = Global::new(0x00b0_c38c);
/// The message `FmodMusic::read: mEvent[%d] not find! musicIdx = %d\n`.
pub const NOT_FOUND: u32 = 0x00b4_f5ac;
/// FMOD's "event not found" result.
pub const FMOD_ERR_EVENT_NOTFOUND: u32 = 0x59;

/// The event project to load from: `this + 4` in music mode 1 (`this +
/// 0x2c`), else `this + 0`.
fn project(h: &impl Host, this: u32) -> u32 {
    if h.read::<u32>(this.wrapping_add(0x2c)) == 1 {
        h.read(this.wrapping_add(4))
    } else {
        h.read(this)
    }
}

/// Whether an event load may run: the slot is empty, `flag` set, a
/// project there and the no-load switch (`0x00cec174`) not 1.
fn may_load(h: &impl Host, slot: u32, flag: u32, project: u32) -> bool {
    h.read::<u32>(slot) == 0 && flag != 0 && project != 0 && NO_LOAD.get(h) != 1
}

/// Loads event `group` of `project` into `slot` (the project's vtable
/// `+0x24`: `(project, group, 0, &slot)`, line 0x114); a missing event
/// (0x59) is reported on `stderr` with `i`. Then `this + 0x1c` 0.
fn load_event(h: &mut impl Host, this: u32, project: u32, group: u32, slot: u32, i: u32) {
    let vtable: u32 = h.read(project);
    let f: u32 = h.read(vtable.wrapping_add(0x24));
    let r = h.call(f, Conv::Stdcall, &[project, group, 0, slot]);
    errcheck(h, r, 0x114);
    if r == FMOD_ERR_EVENT_NOTFOUND {
        let iob = IOB_FUNC.get(h);
        let stderr = h.call(iob, Conv::Cdecl, &[]).wrapping_add(0x20);
        let f = FPRINTF.get(h);
        let fmt = h.rebase(NOT_FOUND);
        h.call(f, Conv::Cdecl, &[stderr, fmt, i, group]);
    }
    h.write(this.wrapping_add(0x1c), 0u32);
}

/// Drops track `i`'s event when it is to be dropped (`+8 == 1`, cleared):
/// `FmodMusic_stop`, `FmodMusic_clear`, its fade cleared.
fn drop_track(h: &mut impl Host, this: u32, i: u32) {
    let e = track(h, this, i);
    h.write(e.wrapping_add(8), 0u32);
    h.call_sym(MUSIC_STOP, Conv::Thiscall(this), &[i]);
    h.call_sym(MUSIC_CLEAR, Conv::Thiscall(this), &[i]);
    clear_fade(h, this, i);
}

/// `FmodMusic_read(this; i, flag)` (thiscall, ret 8): for track `i`
/// (`<= 0xb5`), a track to drop (`+8 == 1`) is dropped, except track 0xc
/// with an event in music mode 0xd0 (then nothing happens); `+0x38`
/// becomes 1.0, and an empty track loads its event group
/// (`FmodMusic_getState(i)`, `<= 0xb5`) when [`may_load`]. Returns 0.
pub fn music_read(h: &mut impl Host, this: u32, i: u32, flag: u32) -> u32 {
    if i > 0xb5 {
        return 0;
    }
    let e = track(h, this, i);
    if h.read::<u32>(e.wrapping_add(8)) == 1 {
        if h.read::<u32>(e) != 0 && h.read::<u32>(this.wrapping_add(0x28)) == 0xd0 && i == 0xc {
            return 0;
        }
        drop_track(h, this, i);
    }
    let e = track(h, this, i);
    h.write(e.wrapping_add(0x38), 1f32.to_bits());
    let proj = project(h, this);
    let slot = track(h, this, i);
    if !may_load(h, slot, flag, proj) {
        return 0;
    }
    let group = h.call_sym(MUSIC_GROUP, Conv::Cdecl, &[i]);
    if group > 0xb5 {
        return 0;
    }
    load_event(h, this, proj, group, slot, i);
    0
}

/// `FmodMusic_dataSend(this; a, flag)` (thiscall, ret 8; `a` unused):
/// [`music_read`] for track 0x10 with event group 6. Returns 0.
pub fn music_data_send(h: &mut impl Host, this: u32, flag: u32) -> u32 {
    let e = track(h, this, 0x10);
    if h.read::<u32>(e.wrapping_add(8)) == 1 {
        drop_track(h, this, 0x10);
    }
    let e = track(h, this, 0x10);
    h.write(e.wrapping_add(0x38), 1f32.to_bits());
    let proj = project(h, this);
    let slot = track(h, this, 0x10);
    if may_load(h, slot, flag, proj) {
        load_event(h, this, proj, 6, slot, 0x10);
    }
    0
}

/// `FmodMusic_read` (thiscall).
pub const MUSIC_READ: u32 = 0x0070_9080;
/// `FmodMusic_start` (thiscall).
pub const MUSIC_START: u32 = 0x0070_96f0;

/// `FmodMusic_crossPlay(this; i, frames, flag)` (thiscall, ret 0xc): for
/// track `i` (`<= 0xb5`): a held track (`+4 == 1`) without an event is
/// released (`+4` 0); with one, the tracks 0x10 and 0x8a fade out (0x12
/// frames), the others are released (`FmodMusic_releaseOldMusic`), it is
/// unpaused (line 0x278), becomes the current one (`this + 0x18`),
/// playing (`+0x20` 1, `+4` 0), and fades in from 0 to 0x7f over 0x12
/// frames. Any other track not current queues the fade (`+0x18` 1,
/// `+0x1c` frames), fades the current one out (0x3c frames), and is read
/// (`FmodMusic_read(i, flag)`) and started. Returns 0.
pub fn music_cross_play(h: &mut impl Host, this: u32, i: u32, frames: u32, flag: u32) -> u32 {
    if i > 0xb5 {
        return 0;
    }
    let e = track(h, this, i);
    if h.read::<u32>(e.wrapping_add(4)) == 1 {
        if h.read::<u32>(e) == 0 {
            h.write(e.wrapping_add(4), 0u32);
            return 0;
        }
        h.call_sym(MUSIC_FADE_OUT, Conv::Thiscall(this), &[0x10, 0x12]);
        h.call_sym(MUSIC_FADE_OUT, Conv::Thiscall(this), &[0x8a, 0x12]);
        h.call_sym(RELEASE_OLD, Conv::Thiscall(this), &[i]);
        let ev: u32 = h.read(track(h, this, i));
        let r = h.call_sym(EVENT_SET_PAUSED, Conv::Stdcall, &[ev, 0]);
        errcheck(h, r, 0x278);
        let e = track(h, this, i);
        h.write(this.wrapping_add(0x18), i);
        h.write(e.wrapping_add(0x20), 1u32);
        let e = track(h, this, i);
        h.write(e.wrapping_add(4), 0u32);
        h.call_sym(MUSIC_VOLUME_DIRECT, Conv::Thiscall(this), &[i, 0, 1]);
        h.call_sym(
            MUSIC_VOLUME_TRANSLATE,
            Conv::Thiscall(this),
            &[i, 0x12, 0x7f],
        );
        return 0;
    }
    if h.read::<u32>(this.wrapping_add(0x18)) == i {
        return 0;
    }
    h.write(e.wrapping_add(0x18), 1u32);
    let e = track(h, this, i);
    h.write(e.wrapping_add(0x1c), frames);
    let current: u32 = h.read(this.wrapping_add(0x18));
    h.call_sym(MUSIC_FADE_OUT, Conv::Thiscall(this), &[current, 0x3c]);
    h.call_sym(MUSIC_READ, Conv::Thiscall(this), &[i, flag]);
    h.call_sym(MUSIC_START, Conv::Thiscall(this), &[i]);
    0
}

/// `operator_new_006304c0(size)` (cdecl).
pub const OPERATOR_NEW: u32 = 0x0063_04c0;
/// `operator_delete_00630540(p)` (cdecl).
pub const OPERATOR_DELETE_ARRAY: u32 = 0x0063_0540;
/// `FmodMusic_stopAll` (thiscall).
pub const MUSIC_STOP_ALL: u32 = 0x0070_9b10;
/// `FmodMusic_forceClearAll` (thiscall).
pub const MUSIC_FORCE_CLEAR_ALL: u32 = 0x0070_8af0;

/// `FmodMusic_switchBGM(this; bgm, flag)` (thiscall, ret 8): to a new
/// background music set (`this + 0x2c`): the loaded tracks and the
/// audible playing ones not held (`+0x20` set, `+0x34` above 0, `+8`
/// 0) are listed (two 0x2d8-byte arrays from `operator new`), everything
/// stops (`FmodMusic_stopAll`, `FmodMusic_forceClearAll`), the set
/// changes, the listed tracks are read again (`FmodMusic_read(i, flag)`)
/// and the playing ones started; the lists are freed. The original's
/// `eax` is a call's: none read.
pub fn music_switch_bgm(h: &mut impl Host, this: u32, bgm: u32, flag: u32) {
    if h.read::<u32>(this.wrapping_add(0x2c)) == bgm {
        return;
    }
    let loaded = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x2d8]);
    let playing = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x2d8]);
    let (mut n1, mut n2) = (0u32, 0u32);
    for i in 0..TRACKS {
        let e = track(h, this, i);
        if h.read::<u32>(e) != 0 {
            h.write(loaded.wrapping_add(4 * n1), i);
            n1 += 1;
        }
        let e = track(h, this, i);
        if h.read::<u32>(e.wrapping_add(0x20)) != 0
            && 0.0 < load(h.read(e.wrapping_add(0x34)))
            && h.read::<u32>(track(h, this, i).wrapping_add(8)) == 0
        {
            h.write(playing.wrapping_add(4 * n2), i);
            n2 += 1;
        }
    }
    h.call_sym(MUSIC_STOP_ALL, Conv::Thiscall(this), &[]);
    h.call_sym(MUSIC_FORCE_CLEAR_ALL, Conv::Thiscall(this), &[]);
    h.write(this.wrapping_add(0x2c), bgm);
    for k in 0..n1 {
        let i: u32 = h.read(loaded.wrapping_add(4 * k));
        h.call_sym(MUSIC_READ, Conv::Thiscall(this), &[i, flag]);
    }
    for k in 0..n2 {
        let i: u32 = h.read(playing.wrapping_add(4 * k));
        h.call_sym(MUSIC_START, Conv::Thiscall(this), &[i]);
    }
    h.call_sym(OPERATOR_DELETE_ARRAY, Conv::Cdecl, &[loaded]);
    h.call_sym(OPERATOR_DELETE_ARRAY, Conv::Cdecl, &[playing]);
}
