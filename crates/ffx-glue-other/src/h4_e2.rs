//! Continuation round 3 of list `h4-glue-other`, batch 2: the FMOD
//! volume code: the music's direct volume, its whole-volume scale, its
//! fade setup and the old tracks' fade-out, the voice's loop fade, the
//! sound events' pan and stop, the sync manager's data read.
//!
//! Floats follow the x87 at 53 bits (`ffx_bridge::x87`). The x87
//! comparisons (`fcom`/`fucom` + `fnstsw` + `test ah`) come out as plain
//! `f64` comparisons: each tested pattern is false for NaN, as Rust's
//! comparisons are. FMOD's API is called through its import thunks
//! (stdcall), each result through `ERRCHECK(result, file, line)`.
//!
//! Bridges that hand callees a local take `frame`, the block laid out as
//! the original's frame from its lowest local up.

// `fcom`/`fucom` compare exactly, as `==` does.
#![allow(clippy::float_cmp)]

use ffx_bridge::x87::{add, div, ftol, load, mul, store, sub};
use ffx_graphic::host::{Conv, Host};

use crate::frames::{ERRCHECK, FMOD_GET_STATE, FMOD_SET_VOLUME, FMOD_VOICE_FILE, OPERATOR_DELETE};
use crate::h4_c2::SLEEP_MS;
use crate::h4_d1::{
    EVENT_GET_CHANNEL_GROUP, GROUP_GET_CHANNEL, GROUP_NUM_CHANNELS, SFX_EVENT_FILE,
};
use crate::h4_e1::pass;

/// The file name `FmodMusic` passes to `ERRCHECK` (`0x00b4f4a0`).
pub const MUSIC_FILE: u32 = 0x00b4_f4a0;
/// The divisor the volumes are scaled by (`0x00b922e8`, f64).
pub const VOLUME_DIVISOR: u32 = 0x00b9_22e8;
/// The music tracks: 0xb6 records of 0x3c bytes from `+0x10`.
pub const TRACKS: u32 = 0xb6;
/// A track record's size.
pub const TRACK: u32 = 0x3c;

fn divisor(h: &impl Host) -> f64 {
    h.read(h.rebase(VOLUME_DIVISOR))
}

/// Track `i`'s record (`[this + 0x10] + 0x3c i`, read again each time as
/// the original does).
fn track(h: &impl Host, this: u32, i: u32) -> u32 {
    h.read::<u32>(this.wrapping_add(0x10))
        .wrapping_add(i.wrapping_mul(TRACK))
}

// ---- the music's volume

/// `FmodMusic_volumeDirect(this; i, volume, keep)` (thiscall, ret 0xc):
/// for a loaded track `i` (`<= 0xb5`, event `+0`): while it does not play
/// (`+0x20 != 1`) the volume is kept for later (`+0xc` 1, `+0x10` the
/// volume, `+0x14` keep). Else `keep == 1` stores the volume at `+0x34`;
/// the mute flag (`this + 0x24 == 1`) makes it 0; a pending volume
/// (`+0xc == 1`) is cleared (`+0x10` 0.0, `+0x14` 0); `+0x38` becomes
/// `volume / divisor` and the event's volume `+0x38 * this + 0x30`
/// (`ERRCHECK` line 0x33a). Returns 0.
pub fn music_volume_direct(h: &mut impl Host, this: u32, i: u32, volume: u32, keep: u32) -> u32 {
    if i > 0xb5 {
        return 0;
    }
    let e = track(h, this, i);
    if h.read::<u32>(e) == 0 {
        return 0;
    }
    let v = load(volume);
    if h.read::<u32>(e.wrapping_add(0x20)) != 1 {
        h.write(e.wrapping_add(0xc), 1u32);
        let e = track(h, this, i);
        h.write(e.wrapping_add(0x10), store(v));
        let e = track(h, this, i);
        h.write(e.wrapping_add(0x14), keep);
        return 0;
    }
    if keep == 1 {
        h.write(e.wrapping_add(0x34), store(v));
    }
    let value = if h.read::<u32>(this.wrapping_add(0x24)) == 1 {
        0.0
    } else {
        v
    };
    let e = track(h, this, i);
    if h.read::<u32>(e.wrapping_add(0xc)) == 1 {
        h.write(e.wrapping_add(0xc), 0u32);
        let e = track(h, this, i);
        h.write(e.wrapping_add(0x10), 0u32);
        let e = track(h, this, i);
        h.write(e.wrapping_add(0x14), 0u32);
    }
    let w = store(div(value, divisor(h)));
    let e = track(h, this, i);
    h.write(e.wrapping_add(0x38), w);
    let e = track(h, this, i);
    let scale: u32 = h.read(this.wrapping_add(0x30));
    let x = store(mul(load(h.read(e.wrapping_add(0x38))), load(scale)));
    let event: u32 = h.read(e);
    let r = h.call_sym(FMOD_SET_VOLUME, Conv::Stdcall, &[event, x]);
    let file = h.rebase(MUSIC_FILE);
    h.call_sym(ERRCHECK, Conv::Cdecl, &[r, file, 0x33a]);
    0
}

/// `FmodMusic_setEntireVolume(this; volume)` (thiscall, ret 4): the
/// volume clamped to 0..1; when it differs from the whole-volume scale
/// (`+0x30`, `!=` with NaN unequal) it becomes the scale, and every
/// loaded track (`+0` set) is set again: a playing one (`+0x20 == 1`) as
/// [`music_volume_direct`] does with its kept volume `+0x34` (0 when
/// muted, a pending volume cleared), a stopped one gets its kept volume
/// pending (`+0xc` 1, `+0x10` it, `+0x14` 0). The original's `eax` is its
/// status word: none read.
pub fn music_set_entire_volume(h: &mut impl Host, this: u32, volume: u32) {
    let v = load(volume);
    let volume = if v < 0.0 {
        0
    } else if v > 1.0 {
        1f32.to_bits()
    } else {
        pass(volume)
    };
    let cur = load(h.read(this.wrapping_add(0x30)));
    if cur == load(volume) {
        return;
    }
    h.write(this.wrapping_add(0x30), volume);
    let k = divisor(h);
    for i in 0..TRACKS {
        let e = track(h, this, i);
        let mut kept = pass(h.read(e.wrapping_add(0x34)));
        if h.read::<u32>(e) == 0 {
            continue;
        }
        if h.read::<u32>(e.wrapping_add(0x20)) != 1 {
            h.write(e.wrapping_add(0xc), 1u32);
            let e = track(h, this, i);
            h.write(e.wrapping_add(0x10), kept);
            let e = track(h, this, i);
            h.write(e.wrapping_add(0x14), 0u32);
            continue;
        }
        if h.read::<u32>(this.wrapping_add(0x24)) == 1 {
            kept = 0;
        }
        if h.read::<u32>(e.wrapping_add(0xc)) == 1 {
            h.write(e.wrapping_add(0xc), 0u32);
            let e = track(h, this, i);
            h.write(e.wrapping_add(0x10), 0u32);
            let e = track(h, this, i);
            h.write(e.wrapping_add(0x14), 0u32);
        }
        let w = store(div(load(kept), k));
        let e = track(h, this, i);
        h.write(e.wrapping_add(0x38), w);
        let e = track(h, this, i);
        let scale: u32 = h.read(this.wrapping_add(0x30));
        let x = store(mul(load(h.read(e.wrapping_add(0x38))), load(scale)));
        let event: u32 = h.read(e);
        let r = h.call_sym(FMOD_SET_VOLUME, Conv::Stdcall, &[event, x]);
        let file = h.rebase(MUSIC_FILE);
        h.call_sym(ERRCHECK, Conv::Cdecl, &[r, file, 0x33a]);
    }
}

/// `FmodMusic_volumeDirect` (thiscall).
pub const MUSIC_VOLUME_DIRECT: u32 = 0x0070_9dd0;
/// Bytes of [`music_volume_translate`]'s frame (the state word).
pub const TRANSLATE_FRAME: u32 = 4;

/// `FmodMusic_volumeTranslate(this; i, frames, target)` (thiscall, ret
/// 0xc; `frame` its state word, which starts as `this`, `push ecx`): for
/// a loaded track `i` (`<= 0xb5`): with no frames, the fade is cleared
/// (`+0x28` 0, `+0x2c`/`+0x30` 0.0, `+0x24` 0) and the volume set at once
/// (`FmodMusic_volumeDirect(i, (float)target, 1)`); else, when the
/// event's state (line 0x2f2) has bit 8, a fade of `frames` steps (at
/// most 0x78; up to 0x168 for track 0x82 in mode 0x84 and 0xb4 in mode
/// 0x15d when `+0x1e80` of the tracks is 1) toward `(float)target`: `+0x28`
/// the steps, `+0x30` the target, `+0x2c` the step `(target - +0x38 *
/// divisor) / steps`, `+0x24` 1. Returns 0.
pub fn music_volume_translate(
    h: &mut impl Host,
    frame: u32,
    this: u32,
    i: u32,
    frames: u32,
    target: u32,
) -> u32 {
    h.write(frame, this);
    if i > 0xb5 {
        return 0;
    }
    let e = track(h, this, i);
    let event: u32 = h.read(e);
    if event == 0 {
        return 0;
    }
    if frames == 0 {
        h.write(e.wrapping_add(0x28), 0u32);
        let e = track(h, this, i);
        h.write(e.wrapping_add(0x2c), 0u32);
        let e = track(h, this, i);
        h.write(e.wrapping_add(0x30), 0u32);
        let e = track(h, this, i);
        h.write(e.wrapping_add(0x24), 0u32);
        let t = pass(store(f64::from(target.cast_signed())));
        h.call_sym(MUSIC_VOLUME_DIRECT, Conv::Thiscall(this), &[i, t, 1]);
        return 0;
    }
    let r = h.call_sym(FMOD_GET_STATE, Conv::Stdcall, &[event, frame]);
    let file = h.rebase(MUSIC_FILE);
    h.call_sym(ERRCHECK, Conv::Cdecl, &[r, file, 0x2f2]);
    if h.read::<u8>(frame) & 8 == 0 {
        return 0;
    }
    let mode: u32 = h.read(this.wrapping_add(0x28));
    let f = frames.cast_signed();
    let steps = if mode == 0x84 && i == 0x82 && h_flag(h, this) && f > 0x168 {
        0x168
    } else if mode == 0x15d && i == 0x82 && h_flag(h, this) && f > 0xb4 {
        i + 0x32
    } else if f > 0x78 {
        0x78
    } else {
        frames
    };
    let e = track(h, this, i);
    h.write(e.wrapping_add(0x28), steps);
    let t = pass(store(f64::from(target.cast_signed())));
    let e = track(h, this, i);
    h.write(e.wrapping_add(0x30), t);
    let e = track(h, this, i);
    let cur = mul(load(h.read(e.wrapping_add(0x38))), divisor(h));
    let n = store(f64::from(steps.cast_signed()));
    let step = store(div(sub(load(t), cur), load(n)));
    h.write(e.wrapping_add(0x2c), step);
    let e = track(h, this, i);
    h.write(e.wrapping_add(0x24), 1u32);
    0
}

/// Whether `+0x1e80` of the tracks (track 0x82's `+0x20`) is 1.
fn h_flag(h: &mut impl Host, this: u32) -> bool {
    let tracks: u32 = h.read(this.wrapping_add(0x10));
    h.read::<u32>(tracks.wrapping_add(0x1e80)) == 1
}

/// The event's volume getter (`0x00948cb8`, stdcall `(event, &volume)`).
pub const EVENT_GET_VOLUME: u32 = 0x0094_8cb8;
/// The event's paused getter (`0x00948cc4`, stdcall `(event, &paused)`).
pub const EVENT_GET_PAUSED: u32 = 0x0094_8cc4;
/// `FmodMusic_volumeFadeOut(this; i, frames)` (thiscall).
pub const MUSIC_FADE_OUT: u32 = 0x0070_9eb0;
/// Bytes of [`release_old_music`]'s frame (the volume, the paused byte at
/// `+7`).
pub const RELEASE_OLD_FRAME: u32 = 8;

/// `FmodMusic_releaseOldMusic(this; keep)` (thiscall, ret 4; `frame` its
/// volume at `ebp - 8` and paused byte at `ebp - 1`): with `keep <=
/// 0xb5`, every other loaded track not held (`+8 != 1`) is asked its
/// volume and paused state (lines 0x259, 0x25b); a playing one (`+0x20
/// == 1`) that is audible (volume above 0) and not paused fades out over
/// 0x12 frames (`FmodMusic_volumeFadeOut`). The original's `eax` is a
/// call's: none read.
pub fn release_old_music(h: &mut impl Host, frame: u32, this: u32, keep: u32) {
    if keep > 0xb5 {
        return;
    }
    let file = h.rebase(MUSIC_FILE);
    let (volume, paused) = (frame, frame.wrapping_add(7));
    for i in 0..TRACKS {
        let e = track(h, this, i);
        let event: u32 = h.read(e);
        if event == 0 || keep == i || h.read::<u32>(e.wrapping_add(8)) == 1 {
            continue;
        }
        let r = h.call_sym(EVENT_GET_VOLUME, Conv::Stdcall, &[event, volume]);
        h.call_sym(ERRCHECK, Conv::Cdecl, &[r, file, 0x259]);
        let e = track(h, this, i);
        let event: u32 = h.read(e);
        let r = h.call_sym(EVENT_GET_PAUSED, Conv::Stdcall, &[event, paused]);
        h.call_sym(ERRCHECK, Conv::Cdecl, &[r, file, 0x25b]);
        let e = track(h, this, i);
        if h.read::<u32>(e.wrapping_add(0x20)) == 1
            && load(h.read(volume)) > 0.0
            && h.read::<u8>(paused) == 0
        {
            h.call_sym(MUSIC_FADE_OUT, Conv::Thiscall(this), &[i, 0x12]);
        }
    }
}

// ---- the voice

/// `FmodVoice_volumeDirect(this; volume, keep)` (thiscall).
pub const VOICE_VOLUME_DIRECT: u32 = 0x0070_b7f0;
/// The snap distance of the voice's fade (`0x00b92188`, f64).
pub const VOICE_SNAP: u32 = 0x00b9_2188;
/// Bytes of [`voice_loop_volume`]'s frame (a float, the state word).
pub const VOICE_LOOP_FRAME: u32 = 8;

/// `FmodVoice_loopVolumeTranslate(this)` (thiscall; `frame` its locals
/// from `ebp - 8`, the state word at `+4`): one step of the voice's fade
/// while it runs (byte `+0x2c`): the step count `+0x30` counts down; past
/// the end the target (`+0x3c`) is set (`FmodVoice_volumeDirect(target,
/// 1)`) and the fade cleared (`+0x30` 0, `+0x34`/`+0x38`/`+0x3c` 0.0).
/// Else, with a channel (`+0x14`) in state bit 8 (line 0x2d1), the
/// volume `+0x38` becomes `+0x38 * divisor + step (+0x34)`, snapped to
/// the target when within `0x00b92188` of it, is set, and is kept
/// divided again. Returns 0.
pub fn voice_loop_volume(h: &mut impl Host, frame: u32, this: u32) -> u32 {
    if h.read::<u8>(this.wrapping_add(0x2c)) == 0 {
        return 0;
    }
    let n: u32 = h.read(this.wrapping_add(0x30));
    h.write(this.wrapping_add(0x30), n.wrapping_sub(1));
    if n.cast_signed() < 0 {
        let target = pass(h.read(this.wrapping_add(0x3c)));
        h.call_sym(VOICE_VOLUME_DIRECT, Conv::Thiscall(this), &[target, 1]);
        h.write(this.wrapping_add(0x30), 0u32);
        for off in [0x34, 0x38, 0x3c] {
            h.write(this.wrapping_add(off), 0u32);
        }
        return 0;
    }
    let channel: u32 = h.read(this.wrapping_add(0x14));
    if channel == 0 {
        return 0;
    }
    let state = frame.wrapping_add(4);
    let r = h.call_sym(FMOD_GET_STATE, Conv::Stdcall, &[channel, state]);
    let file = h.rebase(FMOD_VOICE_FILE);
    h.call_sym(ERRCHECK, Conv::Cdecl, &[r, file, 0x2d1]);
    if h.read::<u8>(state) & 8 == 0 {
        return 0;
    }
    let k = divisor(h);
    let step: u32 = h.read(this.wrapping_add(0x34));
    let v = store(add(
        mul(load(h.read(this.wrapping_add(0x38))), k),
        load(step),
    ));
    h.write(this.wrapping_add(0x38), v);
    let target: u32 = h.read(this.wrapping_add(0x3c));
    let d = store(sub(load(v), load(target)));
    let d = store(load(d).abs());
    h.write(frame, d);
    let snap: f64 = h.read(h.rebase(VOICE_SNAP));
    if load(d) < snap {
        let target = pass(h.read(this.wrapping_add(0x3c)));
        h.write(this.wrapping_add(0x38), target);
    }
    let v = pass(h.read(this.wrapping_add(0x38)));
    h.call_sym(VOICE_VOLUME_DIRECT, Conv::Thiscall(this), &[v, 1]);
    let v = store(div(load(h.read(this.wrapping_add(0x38))), k));
    h.write(this.wrapping_add(0x38), v);
    0
}

// ---- sound events

/// The channel's pan setter (`0x00948d1e`, stdcall `(channel, pan)`).
pub const CHANNEL_SET_PAN: u32 = 0x0094_8d1e;
/// Bytes of [`sfx_event_apply_pan`]'s frame (`this`, the channel, the
/// channel count, the group).
pub const APPLY_PAN_FRAME: u32 = 0x10;

/// `SfxEvent_applyPan(this; pan)` (thiscall, ret 4; `frame` its locals
/// from `ebp - 0x10`: the channel at `+4`, the count at `+8`, the group
/// at `+0xc`): for each of the `+0` (signed) events (`+8`, 8 bytes:
/// handle, live byte `+4`) that is live and has a channel group, every
/// channel's pan is set (lines 0x13f, 0x144, 0x146). The original's
/// `eax` is a call's: none read.
pub fn sfx_event_apply_pan(h: &mut impl Host, frame: u32, this: u32, pan: u32) {
    let (channel, count, group) = (frame + 4, frame + 8, frame + 0xc);
    let file = h.rebase(SFX_EVENT_FILE);
    let n: u32 = h.read(this);
    let mut i = 0u32;
    while i.cast_signed() < n.cast_signed() {
        let events: u32 = h.read(this.wrapping_add(8));
        let ev = events.wrapping_add(8 * i);
        if h.read::<u8>(ev.wrapping_add(4)) != 0 {
            let handle: u32 = h.read(ev);
            if h.call_sym(EVENT_GET_CHANNEL_GROUP, Conv::Stdcall, &[handle, group]) == 0 {
                let g: u32 = h.read(group);
                let r = h.call_sym(GROUP_NUM_CHANNELS, Conv::Stdcall, &[g, count]);
                h.call_sym(ERRCHECK, Conv::Cdecl, &[r, file, 0x13f]);
                let mut j = 0u32;
                while j.cast_signed() < h.read::<u32>(count).cast_signed() {
                    let g: u32 = h.read(group);
                    let r = h.call_sym(GROUP_GET_CHANNEL, Conv::Stdcall, &[g, j, channel]);
                    h.call_sym(ERRCHECK, Conv::Cdecl, &[r, file, 0x144]);
                    let c: u32 = h.read(channel);
                    let r = h.call_sym(CHANNEL_SET_PAN, Conv::Stdcall, &[c, pass(pan)]);
                    h.call_sym(ERRCHECK, Conv::Cdecl, &[r, file, 0x146]);
                    j += 1;
                }
            }
        }
        i += 1;
    }
}

/// The event's stop (`0x00948ca0`, stdcall `(event, immediate)`).
pub const EVENT_STOP: u32 = 0x0094_8ca0;
/// Bytes of [`sfx_event_stop`]'s frame (a float, the whole volume, the
/// volume).
pub const STOP_FRAME: u32 = 0xc;

/// `SfxEvent_stop(this; immediate)` (thiscall, ret 4; `frame` its locals
/// from `ebp - 0xc`, the volume at `+8`): the first event's volume (0.0
/// unless it is live, line 0x10f) truncated; every event (`+0` of them,
/// signed) is stopped, at once when `immediate` (byte) is set or the
/// volume truncates to 0 (line 0xac), else with its fade (line 0xb1).
/// Then `+0xc` 0.0, `+0x10`/`+0x14` 0, and the buffers `+0x18`/`+0x1c`
/// deleted and cleared. The original's `eax` is a call's: none read.
pub fn sfx_event_stop(h: &mut impl Host, frame: u32, this: u32, immediate: u32) {
    let volume = frame.wrapping_add(8);
    h.write(volume, 0u32);
    let file = h.rebase(SFX_EVENT_FILE);
    let first: u32 = h.read(this.wrapping_add(8));
    if h.read::<u8>(first.wrapping_add(4)) != 0 {
        let handle: u32 = h.read(first);
        let r = h.call_sym(EVENT_GET_VOLUME, Conv::Stdcall, &[handle, volume]);
        h.call_sym(ERRCHECK, Conv::Cdecl, &[r, file, 0x10f]);
    }
    let whole = ftol(h, load(h.read(volume)));
    h.write(frame.wrapping_add(4), whole);
    let n: u32 = h.read(this);
    let mut i = 0u32;
    while i.cast_signed() < n.cast_signed() {
        let at_once = if immediate as u8 == 0 {
            let f = store(f64::from(whole.cast_signed()));
            h.write(frame, f);
            load(f) == 0.0
        } else {
            true
        };
        let (mode, line) = if at_once { (1, 0xac) } else { (0, 0xb1) };
        let events: u32 = h.read(this.wrapping_add(8));
        let ev: u32 = h.read(events.wrapping_add(8 * i));
        let r = h.call_sym(EVENT_STOP, Conv::Stdcall, &[ev, mode]);
        h.call_sym(ERRCHECK, Conv::Cdecl, &[r, file, line]);
        i += 1;
    }
    h.write(this.wrapping_add(0xc), 0u32);
    h.write(this.wrapping_add(0x10), 0u32);
    h.write(this.wrapping_add(0x14), 0u32);
    for off in [0x18, 0x1c] {
        let p: u32 = h.read(this.wrapping_add(off));
        if p != 0 {
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[p]);
            h.write(this.wrapping_add(off), 0u32);
        }
    }
}

// ---- the sync manager

/// `SyncManager_syncGetData(this; &x, &y, &id)` (thiscall, ret 0xc): with
/// a read cursor (`+8`) and data (`+0x1c`), once loaded (`+0x20`, waited
/// for in 5 ms sleeps), the next record's two floats and id are read
/// (the cursor moves 4 bytes each). Returns 0, or 0x3e7 (and the cursor
/// cleared) at the end record (x and y 1.0, id 0x3e7) and without data.
pub fn sync_get_data(h: &mut impl Host, this: u32, x: u32, y: u32, id: u32) -> u32 {
    if h.read::<u32>(this.wrapping_add(8)) == 0 || h.read::<u32>(this.wrapping_add(0x1c)) == 0 {
        return 0x3e7;
    }
    while h.read::<u32>(this.wrapping_add(0x20)) == 0 {
        h.call_sym(SLEEP_MS, Conv::Cdecl, &[5]);
    }
    for (out, float) in [(x, true), (y, true), (id, false)] {
        let p: u32 = h.read(this.wrapping_add(8));
        let v: u32 = h.read(p);
        h.write(out, if float { pass(v) } else { v });
        h.write(
            this.wrapping_add(8),
            h.read::<u32>(this.wrapping_add(8)).wrapping_add(4),
        );
    }
    if load(h.read(x)) != 1.0 || load(h.read(y)) != 1.0 || h.read::<u32>(id) != 0x3e7 {
        return 0;
    }
    h.write(this.wrapping_add(8), 0u32);
    0x3e7
}
