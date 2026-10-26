//! Continuation round 3 of list `h4-glue-other`, batch 3: the FMOD
//! music's stop, stop-all, restart and per-frame fade.
//!
//! As in [`crate::h4_e2`]: the x87 at 53 bits, its comparisons as plain
//! `f64` ones (each tested pattern false for NaN), FMOD through its import
//! thunks with `ERRCHECK`, and `frame` the original's frame from its
//! lowest local up. Where the original hands FMOD the address of its own
//! argument word (the paused byte of `FmodMusic_stop`, the state of
//! `FmodMusic_restart`), the frame reaches up to that word and holds the
//! argument there.

use ffx_bridge::x87::{add, div, load, mul, store, sub};
use ffx_graphic::host::{Conv, Host};

use crate::frames::{ERRCHECK, FMOD_GET_STATE};
use crate::h4_c3::{EVENT_SET_CALLBACK, EVENT_STOP};
use crate::h4_e1::pass;
use crate::h4_e2::{
    EVENT_GET_PAUSED, MUSIC_FILE, MUSIC_VOLUME_DIRECT, TRACK, TRACKS, VOLUME_DIVISOR,
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

/// Clears track `i`'s fade: `+0x28` 0, `+0x2c`/`+0x30` 0.0, `+0x24` 0
/// (the tracks read again for each store).
fn clear_fade(h: &mut impl Host, this: u32, i: u32) {
    for off in [0x28, 0x2c, 0x30, 0x24] {
        let e = track(h, this, i);
        h.write(e.wrapping_add(off), 0u32);
    }
}

/// One track's stop (shared by [`music_stop`] and [`music_stop_all`]),
/// after its checks: a running fade cleared (`+0x24 == 1`), its state
/// asked (line 0x1f1; `state` the word FMOD writes); when it plays (bit
/// 8): `+0x20` and `this + 0x20` cleared, its paused byte asked (line
/// 0x1fa; `paused`); a held track (`+4 == 1`) is paused (line 0x201)
/// unless it already is; any other is unpaused if paused (line 0x20a), its
/// callback removed (line 0x20c) and it is stopped (line 0x20d).
fn stop_track(h: &mut impl Host, this: u32, i: u32, state: u32, paused: u32) {
    let e = track(h, this, i);
    if h.read::<u32>(e.wrapping_add(0x24)) == 1 {
        clear_fade(h, this, i);
    }
    let ev: u32 = h.read(track(h, this, i));
    let r = h.call_sym(FMOD_GET_STATE, Conv::Stdcall, &[ev, state]);
    errcheck(h, r, 0x1f1);
    if h.read::<u8>(state) & 8 == 0 {
        return;
    }
    let e = track(h, this, i);
    h.write(e.wrapping_add(0x20), 0u32);
    h.write(this.wrapping_add(0x20), 0u32);
    let ev: u32 = h.read(track(h, this, i));
    let r = h.call_sym(EVENT_GET_PAUSED, Conv::Stdcall, &[ev, paused]);
    errcheck(h, r, 0x1fa);
    let e = track(h, this, i);
    let is_paused = h.read::<u8>(paused) != 0;
    if h.read::<u32>(e.wrapping_add(4)) == 1 {
        if !is_paused {
            let ev: u32 = h.read(e);
            let r = h.call_sym(EVENT_SET_PAUSED, Conv::Stdcall, &[ev, 1]);
            errcheck(h, r, 0x201);
        }
        return;
    }
    if is_paused {
        let ev: u32 = h.read(e);
        let r = h.call_sym(EVENT_SET_PAUSED, Conv::Stdcall, &[ev, 0]);
        errcheck(h, r, 0x20a);
    }
    let ev: u32 = h.read(track(h, this, i));
    let r = h.call_sym(EVENT_SET_CALLBACK, Conv::Stdcall, &[ev, 0, 0]);
    errcheck(h, r, 0x20c);
    let ev: u32 = h.read(track(h, this, i));
    let r = h.call_sym(EVENT_STOP, Conv::Stdcall, &[ev, 0]);
    errcheck(h, r, 0x20d);
}

/// Bytes of [`music_stop`]'s frame: from the state word at `ebp - 4` up
/// to the argument word at `ebp + 8` (`+0xc`), whose top byte (`+0xf`)
/// FMOD's paused query writes.
pub const STOP_FRAME: u32 = 0x10;

/// `FmodMusic_stop(this; i)` (thiscall, ret 4; `frame` from `ebp - 4`,
/// the state word starting as `this` (`push ecx`), `i` at `+0xc`): a
/// loaded track `i` (`<= 0xb5`) that is not kept (`+8 != 1`) is stopped
/// ([`stop_track`]). Returns 0.
pub fn music_stop(h: &mut impl Host, frame: u32, this: u32, i: u32) -> u32 {
    h.write(frame, this);
    h.write(frame.wrapping_add(0xc), i);
    if i > 0xb5 {
        return 0;
    }
    let e = track(h, this, i);
    if h.read::<u32>(e) == 0 || h.read::<u32>(e.wrapping_add(8)) == 1 {
        return 0;
    }
    stop_track(h, this, i, frame, frame.wrapping_add(0xf));
    0
}

/// Bytes of [`music_stop_all`]'s frame (the state word, the paused byte
/// at `+7`; FMOD writes both before they are read).
pub const STOP_ALL_FRAME: u32 = 8;

/// `FmodMusic_stopAll(this)` (thiscall; `frame` from `ebp - 8`): every
/// loaded track not kept is stopped ([`stop_track`]). Returns 0.
pub fn music_stop_all(h: &mut impl Host, frame: u32, this: u32) -> u32 {
    for i in 0..TRACKS {
        let e = track(h, this, i);
        if h.read::<u32>(e) == 0 || h.read::<u32>(e.wrapping_add(8)) == 1 {
            continue;
        }
        stop_track(h, this, i, frame, frame.wrapping_add(7));
    }
    0
}

/// The event's start (`0x00948c9a`, stdcall `(event)`).
pub const EVENT_START: u32 = 0x0094_8c9a;
/// `eventcallback` (`0x00707a50`), the callback the tracks get.
pub const EVENT_CALLBACK: u32 = 0x0070_7a50;
/// The full volume, `0x00b4f5e4` (f32).
pub const FULL_VOLUME: u32 = 0x00b4_f5e4;
/// `FmodMusic_volumeTranslate(this; i, frames, target)` (thiscall).
pub const MUSIC_VOLUME_TRANSLATE: u32 = 0x0070_9ff0;
/// Bytes of [`music_restart`]'s frame: the argument word, where the
/// state query writes.
pub const RESTART_FRAME: u32 = 4;

/// `FmodMusic_restart(this; i)` (thiscall, ret 4; `frame` the argument
/// word, which holds `i` until the state query writes it): with the
/// restart flag (`+0x1c`), a loaded track `i` (1..=0xb5) not playing
/// (`+0x20 != 1`) whose state (line 0x224) is exactly 1 gets its callback
/// (`eventcallback`, user data its `+0x20`; line 0x229) and is started
/// (line 0x22c): the flag cleared, `+0x34` the full volume, `+0x20` 1,
/// `this + 0x20` 0. Muted (`this + 0x24 == 1`), a queued fade (`+0x18`,
/// `+0x1c`) is dropped and the volume set to 0; a queued fade still there
/// starts from 0 (`FmodMusic_volumeDirect(i, 0, 1)`,
/// `FmodMusic_volumeTranslate(i, frames, 0x7f)`) and is cleared. Returns
/// 0.
pub fn music_restart(h: &mut impl Host, frame: u32, this: u32, i: u32) -> u32 {
    h.write(frame, i);
    if h.read::<u32>(this.wrapping_add(0x1c)) == 0 || i == 0 || i > 0xb5 {
        return 0;
    }
    let e = track(h, this, i);
    if h.read::<u32>(e.wrapping_add(0x20)) == 1 {
        return 0;
    }
    let ev: u32 = h.read(e);
    if ev == 0 {
        return 0;
    }
    let r = h.call_sym(FMOD_GET_STATE, Conv::Stdcall, &[ev, frame]);
    errcheck(h, r, 0x224);
    if h.read::<u32>(frame) != 1 {
        return 0;
    }
    let e = track(h, this, i);
    let ev: u32 = h.read(e);
    let callback = h.rebase(EVENT_CALLBACK);
    let r = h.call_sym(
        EVENT_SET_CALLBACK,
        Conv::Stdcall,
        &[ev, callback, e.wrapping_add(0x20)],
    );
    errcheck(h, r, 0x229);
    let ev: u32 = h.read(track(h, this, i));
    let r = h.call_sym(EVENT_START, Conv::Stdcall, &[ev]);
    errcheck(h, r, 0x22c);
    let full = pass(h.read(h.rebase(FULL_VOLUME)));
    let e = track(h, this, i);
    h.write(this.wrapping_add(0x1c), 0u32);
    h.write(e.wrapping_add(0x34), full);
    let e = track(h, this, i);
    h.write(e.wrapping_add(0x20), 1u32);
    h.write(this.wrapping_add(0x20), 0u32);
    if h.read::<u32>(this.wrapping_add(0x24)) == 1 {
        let e = track(h, this, i);
        h.write(e.wrapping_add(0x18), 0u32);
        let e = track(h, this, i);
        h.write(e.wrapping_add(0x1c), 0u32);
        h.call_sym(MUSIC_VOLUME_DIRECT, Conv::Thiscall(this), &[i, 0, 0]);
    }
    let e = track(h, this, i);
    if h.read::<u32>(e.wrapping_add(0x18)) != 1 || h.read::<u32>(e.wrapping_add(0x1c)) == 0 {
        return 0;
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
    0
}

/// `FmodMusic_volumeReset(this; i, volume, keep)` (thiscall).
pub const MUSIC_VOLUME_RESET: u32 = 0x0070_9f20;
/// `FmodMusic_stop` (thiscall).
pub const MUSIC_STOP: u32 = 0x0070_99a0;
/// `FmodMusic_clear(this; i)` (thiscall).
pub const MUSIC_CLEAR: u32 = 0x0070_8720;
/// Bytes of [`music_loop_volume`]'s frame (the state word, a float).
pub const LOOP_FRAME: u32 = 8;

/// `FmodMusic_loopVolumeTranslate(this)` (thiscall; `frame` from `ebp -
/// 8`: the state word FMOD writes, the volume at `+4`): per track, a
/// pending volume (`+0xc`) is applied (`FmodMusic_volumeReset(i, +0x10,
/// +0x14)`); a running fade (`+0x24`) steps: its count `+0x28` counts
/// down; past the end the target `+0x30` is set (`volumeDirect(i, target,
/// 1)`), a track to drop (`+8 == 1`, cleared) is stopped and cleared
/// (`FmodMusic_stop`, `FmodMusic_clear`), and the fade cleared. Else, for
/// an event whose state (line 0x2b7) has bit 8, the volume `+0x38 *
/// divisor + step (+0x2c)` clamped to 0 below and to the full volume
/// (`0x00b4f5e4`) above the divisor is set (`volumeDirect(i, v, 1)`). A
/// fading track without an event ends the walk. Returns 0.
pub fn music_loop_volume(h: &mut impl Host, frame: u32, this: u32) -> u32 {
    let k: f64 = h.read(h.rebase(VOLUME_DIVISOR));
    for i in 0..TRACKS {
        let e = track(h, this, i);
        if h.read::<u32>(e.wrapping_add(0xc)) != 0 {
            let keep: u32 = h.read(e.wrapping_add(0x14));
            let v = pass(h.read(e.wrapping_add(0x10)));
            h.call_sym(MUSIC_VOLUME_RESET, Conv::Thiscall(this), &[i, v, keep]);
        }
        let e = track(h, this, i);
        if h.read::<u32>(e.wrapping_add(0x24)) == 0 {
            continue;
        }
        let n: u32 = h.read(e.wrapping_add(0x28));
        h.write(e.wrapping_add(0x28), n.wrapping_sub(1));
        let e = track(h, this, i);
        if n.cast_signed() < 0 {
            let target = pass(h.read(e.wrapping_add(0x30)));
            h.call_sym(MUSIC_VOLUME_DIRECT, Conv::Thiscall(this), &[i, target, 1]);
            let e = track(h, this, i);
            if h.read::<u32>(e.wrapping_add(8)) == 1 {
                h.write(e.wrapping_add(8), 0u32);
                h.call_sym(MUSIC_STOP, Conv::Thiscall(this), &[i]);
                h.call_sym(MUSIC_CLEAR, Conv::Thiscall(this), &[i]);
            }
            clear_fade(h, this, i);
            continue;
        }
        let ev: u32 = h.read(e);
        if ev == 0 {
            break;
        }
        let r = h.call_sym(FMOD_GET_STATE, Conv::Stdcall, &[ev, frame]);
        errcheck(h, r, 0x2b7);
        if h.read::<u8>(frame) & 8 == 0 {
            continue;
        }
        let e = track(h, this, i);
        let step: u32 = h.read(e.wrapping_add(0x2c));
        let mut v = store(add(mul(load(h.read(e.wrapping_add(0x38))), k), load(step)));
        if load(v) < 0.0 {
            v = 0;
        }
        if load(v) > k {
            v = pass(h.read(h.rebase(FULL_VOLUME)));
        }
        h.write(frame.wrapping_add(4), v);
        h.call_sym(MUSIC_VOLUME_DIRECT, Conv::Thiscall(this), &[i, pass(v), 1]);
    }
    0
}

// ---- sound events

/// `SfxEvent_applyVolume_0070f140(this; volume)` (thiscall).
pub const SFX_APPLY_VOLUME: u32 = 0x0070_f140;
/// `SfxEvent_applyPan(this; pan)` (thiscall).
pub const SFX_APPLY_PAN: u32 = 0x0070_eeb0;
/// The pan's offset (`0x00b92248`, f64 1.0).
pub const PAN_OFFSET: u32 = 0x00b9_2248;
/// The pan's lower limit (`0x00b92340`, f32 -1.0; `g_ChrClothFlip`).
pub const PAN_MIN: u32 = 0x00b9_2340;
/// `SfxEvent.cpp`'s name for `ERRCHECK`.
pub const SFX_FILE: u32 = 0x00b4_fddc;

/// `SfxEvent_start(this; volume, pan)` (thiscall, ret 8; both 0..127
/// integers): the delay (`+0xc` 0.0, `+0x10`/`+0x14` 0) and the fades
/// (`+0x18`, `+0x1c`, deleted) cleared; every event (`+0`, signed;
/// `+8`, 8 bytes each) marked live (byte `+4`), given the callback (user
/// data its live byte; line 0x87) and started (line 0x88); then its
/// volume `volume / divisor` clamped to 0..1 and its pan
/// `2 pan / divisor - 1` clamped to -1..1 (`0x00b92340`, `0x00b92248`)
/// applied. The
/// original's `eax` is the pan call's: none read.
pub fn sfx_event_start(h: &mut impl Host, this: u32, volume: u32, pan: u32) {
    h.write(this.wrapping_add(0xc), 0u32);
    h.write(this.wrapping_add(0x10), 0u32);
    h.write(this.wrapping_add(0x14), 0u32);
    for off in [0x18, 0x1c] {
        let p: u32 = h.read(this.wrapping_add(off));
        if p != 0 {
            h.call_sym(crate::frames::OPERATOR_DELETE, Conv::Cdecl, &[p]);
            h.write(this.wrapping_add(off), 0u32);
        }
    }
    let file = h.rebase(SFX_FILE);
    let callback = h.rebase(EVENT_CALLBACK);
    let n: u32 = h.read(this);
    let mut i = 0u32;
    while i.cast_signed() < n.cast_signed() {
        let events: u32 = h.read(this.wrapping_add(8));
        h.write(events.wrapping_add(8 * i + 4), 1u8);
        let events: u32 = h.read(this.wrapping_add(8));
        let ev = events.wrapping_add(8 * i);
        let handle: u32 = h.read(ev);
        let r = h.call_sym(
            EVENT_SET_CALLBACK,
            Conv::Stdcall,
            &[handle, callback, ev.wrapping_add(4)],
        );
        h.call_sym(ERRCHECK, Conv::Cdecl, &[r, file, 0x87]);
        let events: u32 = h.read(this.wrapping_add(8));
        let handle: u32 = h.read(events.wrapping_add(8 * i));
        let r = h.call_sym(EVENT_START, Conv::Stdcall, &[handle]);
        h.call_sym(ERRCHECK, Conv::Cdecl, &[r, file, 0x88]);
        i += 1;
    }
    let k: f64 = h.read(h.rebase(VOLUME_DIVISOR));
    let int = |x: u32| store(f64::from(x.cast_signed()));
    let mut v = store(div(load(int(volume)), k));
    if load(v) > 1.0 {
        v = 1f32.to_bits();
    } else if load(v) < 0.0 {
        v = 0;
    }
    h.call_sym(SFX_APPLY_VOLUME, Conv::Thiscall(this), &[pass(v)]);
    let q = div(load(int(pan)), k);
    let offset: f64 = h.read(h.rebase(PAN_OFFSET));
    let p = store(sub(add(q, q), offset));
    let min = pass(h.read(h.rebase(PAN_MIN)));
    let p = if load(p) > 1.0 {
        1f32.to_bits()
    } else if load(min) > load(p) {
        min
    } else {
        pass(p)
    };
    h.call_sym(SFX_APPLY_PAN, Conv::Thiscall(this), &[p]);
}

/// One fade record's step (`+0` its target, `+4` its value, `+8` its
/// time left, `+0xc` its rate): the time less `dt`, the value plus `dt *
/// rate`, snapped to the target once the time runs out (`<= 0`).
fn fade_step(h: &mut impl Host, f: u32, dt: f64) {
    let t = store(sub(load(h.read(f.wrapping_add(8))), dt));
    h.write(f.wrapping_add(8), t);
    let rate: u32 = h.read(f.wrapping_add(0xc));
    let cur: u32 = h.read(f.wrapping_add(4));
    h.write(
        f.wrapping_add(4),
        store(add(mul(dt, load(rate)), load(cur))),
    );
    if load(t) <= 0.0 {
        let target = pass(h.read(f));
        h.write(f.wrapping_add(4), target);
    }
}

/// `SfxEvent_updateFades_0070f5c0(this; dt)` (thiscall, ret 4): a delayed
/// start (`+0xc` above 0) counts down by `dt` and starts the event
/// (`SfxEvent_start(+0x10, +0x14)`, the delay 0.0) once it runs out; the
/// volume fade (`+0x18`) and the pan fade (`+0x1c`) step ([`fade_step`])
/// and are applied (`SfxEvent_applyVolume`/`SfxEvent_applyPan` of the
/// value), each deleted when its time is up (`<= 0`). The original's `eax`
/// is a call's or its status word: none read.
pub fn sfx_event_update_fades(h: &mut impl Host, this: u32, dt: u32) {
    let d = load(dt);
    let delay = load(h.read(this.wrapping_add(0xc)));
    if delay > 0.0 {
        let left = store(sub(delay, d));
        h.write(this.wrapping_add(0xc), left);
        if load(left) <= 0.0 {
            let pan: u32 = h.read(this.wrapping_add(0x14));
            h.write(this.wrapping_add(0xc), 0u32);
            let volume: u32 = h.read(this.wrapping_add(0x10));
            h.call_sym(SFX_START, Conv::Thiscall(this), &[volume, pan]);
        }
    }
    for (off, apply) in [(0x18, SFX_APPLY_VOLUME), (0x1c, SFX_APPLY_PAN)] {
        let f: u32 = h.read(this.wrapping_add(off));
        if f == 0 {
            continue;
        }
        fade_step(h, f, d);
        let f: u32 = h.read(this.wrapping_add(off));
        let v = pass(h.read(f.wrapping_add(4)));
        h.call_sym(apply, Conv::Thiscall(this), &[v]);
        let f: u32 = h.read(this.wrapping_add(off));
        if load(h.read(f.wrapping_add(8))) <= 0.0 {
            h.call_sym(crate::frames::OPERATOR_DELETE, Conv::Cdecl, &[f]);
            h.write(this.wrapping_add(off), 0u32);
        }
    }
}

/// `SfxEvent_start` (thiscall).
pub const SFX_START: u32 = 0x0070_f310;
