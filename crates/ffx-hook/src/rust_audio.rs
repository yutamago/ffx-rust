//! The Rust sound engine in place of FMOD (`FFX_RUST_AUDIO=1`; M5 stage 2,
//! `docs/formats/audio-platform.md`, "The Rust engine in the game").
//!
//! Every one of `FFX.exe`'s 40 imports of `fmod_event.dll` and `fmodex.dll`
//! is pointed at a stub here, so no FMOD function runs. The objects the
//! game calls through vtables (`EventProject`, `EventGroup`) are ours, with
//! our vtables; event, channel and sound handles are our own numbers (the
//! game never reads through them). The stubs drive
//! [`ffx_sound::replay::Engine`], the engine the offline gate checks sample
//! for sample against FMOD's captures, with the same calls the game's log
//! shows. Files are read through the game's own file callbacks
//! (`System::setFileSystem`), so the VBF archive and mods work as with FMOD.
//!
//! Each `EventSystem::update` mixes one 1,600-frame block and raises the
//! end callbacks at the updates FMOD does (`SOUNDDEF_END` one update after
//! the block with the wave's last frame, EVENTFINISHED one later; `stop`
//! calls EVENTFINISHED at once), and `Event::getState` answers as FMOD does
//! (0x18 playing, 0x08 ending, 1 ready). With `FFX_AUDIO_WAV` the blocks go
//! into the file in FMOD's WAV writer format (`audio_wav.rs` keeps the
//! marks and the turns). Without a file nothing is played yet: the real
//! output device is still to come.
//!
//! The movie's user stream (`createSound` with `FMOD_OPENUSER`) is mixed
//! from the game's PCM read callback at the stream's rate, ramped and
//! interpolated like every voice; it is not measured against FMOD yet.

use std::collections::HashMap;
use std::ffi::{CStr, c_void};
use std::io::Write as _;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use ffx_sound::mixer::{BLOCK, Voice, Wave};
use ffx_sound::replay::{Call, Engine, Files};

use crate::log::log;

static ON: AtomicBool = AtomicBool::new(false);
/// Frames mixed so far (the DSP clock the marks read).
static CLOCK: AtomicU64 = AtomicU64::new(0);
static STATE: Mutex<Option<State>> = Mutex::new(None);

/// The Rust engine replaces FMOD in this process.
#[must_use]
pub fn on() -> bool {
    ON.load(Ordering::Acquire)
}

/// The mixer's sample clock after the last update (`System::getDSPClock`).
#[must_use]
pub fn clock() -> u64 {
    CLOCK.load(Ordering::Acquire)
}

/// The game's file callbacks (`FmodFile_open/close/read/seek`).
#[derive(Clone, Copy, Default)]
struct GameFiles {
    open: usize,
    close: usize,
    read: usize,
    seek: usize,
}

const ROOT: &str = "../../../FFX_Data/GameData/PS3Data/Sound_PC/";

type Open = unsafe extern "system" fn(*const u8, i32, *mut u32, *mut usize, *mut usize) -> u32;
type Close = unsafe extern "system" fn(usize, usize) -> u32;
type Read = unsafe extern "system" fn(usize, *mut u8, u32, *mut u32, usize) -> u32;
type Seek = unsafe extern "system" fn(usize, u32, usize) -> u32;

impl GameFiles {
    fn range(&self, path: &str, at: usize, len: Option<usize>) -> Option<Vec<u8>> {
        if self.open == 0 {
            return None;
        }
        let name = std::ffi::CString::new(format!("{ROOT}{path}")).ok()?;
        let (mut size, mut handle, mut user) = (0u32, 0usize, 0usize);
        // SAFETY: the game's FMOD file callbacks with their FMOD signatures
        // (stdcall), the pointers to locals.
        unsafe {
            let open: Open = std::mem::transmute(self.open);
            if open(
                name.as_ptr().cast(),
                0,
                &raw mut size,
                &raw mut handle,
                &raw mut user,
            ) != 0
            {
                return None;
            }
            let len = len.unwrap_or((size as usize).saturating_sub(at));
            let mut buf = vec![0u8; len];
            let seek: Seek = std::mem::transmute(self.seek);
            let read: Read = std::mem::transmute(self.read);
            let mut ok = seek(handle, u32::try_from(at).unwrap_or(u32::MAX), user) == 0;
            let mut done = 0usize;
            while ok && done < len {
                let mut got = 0u32;
                let want = u32::try_from((len - done).min(1 << 20)).unwrap_or(0);
                let r = read(handle, buf.as_mut_ptr().add(done), want, &raw mut got, user);
                done += got as usize;
                ok = (r == 0 || r == 0x16) && got > 0;
            }
            let close: Close = std::mem::transmute(self.close);
            close(handle, user);
            buf.truncate(done);
            Some(buf)
        }
    }
}

impl Files for GameFiles {
    fn read(&mut self, path: &str) -> Option<Arc<Vec<u8>>> {
        self.range(path, 0, None).map(Arc::new)
    }
    fn read_range(&mut self, path: &str, at: usize, len: usize) -> Option<Vec<u8>> {
        self.range(path, at, Some(len)).filter(|b| b.len() == len)
    }
}

/// The movie's user stream.
struct Movie {
    sound: u32,
    channel: u32,
    read: usize,
    channels: u16,
    rate: u32,
    voice: Option<Voice>,
    pcm: Vec<i16>,
    volume: f32,
    paused: bool,
}

struct State {
    engine: Engine<SharedFiles>,
    files: Arc<Mutex<GameFiles>>,
    next: u32,
    /// The end callback and its user data per event instance.
    callbacks: HashMap<u32, (usize, usize)>,
    /// Shout sounds: the bank path of each sound handle, the subsound index
    /// of each subsound handle.
    sounds: HashMap<u32, String>,
    subsounds: HashMap<u32, (u32, u32)>,
    movie: Option<Movie>,
    wav: Option<std::fs::File>,
}

/// The game's file callbacks, shared with the engine.
struct SharedFiles(Arc<Mutex<GameFiles>>);

impl Files for SharedFiles {
    fn read(&mut self, path: &str) -> Option<Arc<Vec<u8>>> {
        let f = *self.0.lock().ok()?;
        f.range(path, 0, None).map(Arc::new)
    }
    fn read_range(&mut self, path: &str, at: usize, len: usize) -> Option<Vec<u8>> {
        let f = *self.0.lock().ok()?;
        f.range(path, at, Some(len)).filter(|b| b.len() == len)
    }
}

impl State {
    fn handle(&mut self) -> u32 {
        self.next += 1;
        (self.next << 8) | 1
    }
    fn call(
        &mut self,
        function: &str,
        args: &[u32],
        outs: &[(usize, u32)],
        strings: &[(usize, String)],
    ) {
        let c = Call {
            frame: 0,
            function: function.to_owned(),
            args: args.to_vec(),
            result: 0,
            outs: outs.to_vec(),
            strings: strings.to_vec(),
        };
        self.engine.call(&c);
    }
}

fn with<T>(f: impl FnOnce(&mut State) -> T) -> Option<T> {
    let mut s = STATE.lock().ok()?;
    s.as_mut().map(f)
}

const OK: u32 = 0;
/// `FMOD_ERR_INVALID_PARAM`.
const ERR_PARAM: u32 = 0x25;

fn put<T>(p: usize, v: T) {
    if p != 0 {
        // SAFETY: an out-pointer the game passed for this value.
        unsafe { (p as *mut T).write_unaligned(v) }
    }
}

fn cstr(p: usize) -> Option<String> {
    if p == 0 {
        return None;
    }
    // SAFETY: a NUL-terminated string the game passed.
    Some(
        unsafe { CStr::from_ptr(p as *const i8) }
            .to_string_lossy()
            .into_owned(),
    )
}

fn addr(p: *const ()) -> u32 {
    p as usize as u32
}

// ---- fake objects ------------------------------------------------------

/// A project or group object: its vtable and its handle for the engine.
#[repr(C)]
struct Object {
    vtable: *const usize,
    project: u32,
}

static mut PROJECT_VT: [usize; 14] = [0; 14]; // check-ported: read-only table (filled at install)
static mut GROUP_VT: [usize; 18] = [0; 18]; // check-ported: read-only table (filled at install)

fn object(vtable: *const usize, project: u32) -> u32 {
    let o = Box::leak(Box::new(Object { vtable, project }));
    addr(std::ptr::from_ref::<Object>(o).cast())
}

unsafe extern "system" fn unhandled0(_this: usize) -> u32 {
    OK
}

unsafe extern "system" fn project_release(this: usize) -> u32 {
    with(|s| s.call("EventProjectI::release", &[this as u32], &[], &[]));
    OK
}

unsafe extern "system" fn project_get_info(this: usize, info: usize) -> u32 {
    let n = with(|s| s.engine.playing_events(this as u32)).unwrap_or(0);
    put(info + 0x114, n);
    OK
}

unsafe extern "system" fn project_get_group(
    this: usize,
    name: usize,
    _cache: u32,
    out: usize,
) -> u32 {
    let Some(name) = cstr(name) else {
        return ERR_PARAM;
    };
    // GROUP_VT is filled once at install, before the game runs.
    let g = object((&raw const GROUP_VT).cast(), this as u32);
    with(|s| {
        s.call(
            "EventProjectI::getGroup",
            &[this as u32],
            &[(3, g)],
            &[(1, name)],
        );
    });
    put(out, g);
    OK
}

unsafe extern "system" fn project_stop_all(this: usize, _immediate: u32) -> u32 {
    with(|s| s.call("EventProjectI::stopAllEvents", &[this as u32], &[], &[]));
    OK
}

unsafe extern "system" fn project_cancel(_this: usize) -> u32 {
    OK
}

unsafe extern "system" fn group_load_data(_this: usize, _resource: u32, _mode: u32) -> u32 {
    OK
}

unsafe extern "system" fn group_free_data(_this: usize, event: usize, _wait: u32) -> u32 {
    with(|s| s.call("Event::stop", &[event as u32, 1], &[], &[]));
    OK
}

unsafe extern "system" fn group_get_event(this: usize, name: usize, _mode: u32, out: usize) -> u32 {
    let Some(name) = cstr(name) else {
        return ERR_PARAM;
    };
    let h = with(|s| {
        let h = s.handle();
        s.call(
            "EventGroupI::getEvent",
            &[this as u32],
            &[(3, h)],
            &[(1, name)],
        );
        h
    });
    put(out, h.unwrap_or(0));
    OK
}

unsafe extern "system" fn group_get_event_by_index(
    this: usize,
    index: u32,
    _mode: u32,
    out: usize,
) -> u32 {
    let h = with(|s| {
        let h = s.handle();
        s.call(
            "EventGroupI::getEventByIndex",
            &[this as u32, index],
            &[(3, h)],
            &[],
        );
        h
    });
    put(out, h.unwrap_or(0));
    OK
}

// ---- fmod_event.dll ----------------------------------------------------

unsafe extern "system" fn event_system_create(out: usize) -> u32 {
    put(out, 0x00e5_0001u32);
    OK
}
unsafe extern "system" fn event_system_init(
    _this: usize,
    _ch: i32,
    _flags: u32,
    _extra: usize,
    _ev: u32,
) -> u32 {
    OK
}
unsafe extern "system" fn event_system_get_system(_this: usize, out: usize) -> u32 {
    put(out, 0x00e5_0002u32);
    OK
}
unsafe extern "system" fn event_system_update(_this: usize) -> u32 {
    let due = with(|s| {
        s.engine.update();
        let mut block = std::mem::take(&mut s.engine.out);
        if let Some(m) = s.movie.as_mut() {
            mix_movie(m, &mut block);
        }
        if let Some(f) = s.wav.as_mut() {
            let bytes: Vec<u8> = block.iter().flat_map(|x| x.to_le_bytes()).collect();
            let _ = f.write_all(&bytes);
        }
        CLOCK.fetch_add(BLOCK as u64, Ordering::AcqRel);
        s.engine
            .callbacks()
            .iter()
            .filter_map(|&(e, kind)| s.callbacks.get(&e).map(|&(f, u)| (e, kind, f, u)))
            .collect::<Vec<_>>()
    })
    .unwrap_or_default();
    // outside the lock: the game's callback may call FMOD again
    for (e, kind, f, user) in due {
        callback(e, kind, f, user);
    }
    OK
}

fn callback(event: u32, kind: u32, f: usize, user: usize) {
    if f == 0 {
        return;
    }
    // SAFETY: the game's FMOD_EVENT_CALLBACK (`eventcallback` 0x00707a50,
    // stdcall: event, type, param1, param2, userdata).
    unsafe {
        let f: unsafe extern "system" fn(u32, u32, usize, usize, usize) -> u32 =
            std::mem::transmute(f);
        f(event, kind, 0, 0, user);
    }
}

unsafe extern "system" fn event_system_release(_this: usize) -> u32 {
    OK
}
unsafe extern "system" fn event_system_unload(_this: usize) -> u32 {
    OK
}
unsafe extern "system" fn event_system_load(
    _this: usize,
    name: usize,
    _info: usize,
    out: usize,
) -> u32 {
    let Some(name) = cstr(name) else {
        return ERR_PARAM;
    };
    // PROJECT_VT is filled once at install.
    let p = object((&raw const PROJECT_VT).cast(), 0);
    let loaded = with(|s| {
        s.call("EventSystem::load", &[0], &[(3, p)], &[(1, name.clone())]);
        s.engine.has_project(p)
    })
    .unwrap_or(false);
    if !loaded {
        // FMOD_ERR_FILE_NOTFOUND, as FMOD answers for a missing project
        return 0x17;
    }
    put(out, p);
    OK
}
unsafe extern "system" fn event_start(e: usize) -> u32 {
    with(|s| s.call("Event::start", &[e as u32], &[], &[]));
    OK
}
unsafe extern "system" fn event_stop(e: usize, immediate: u32) -> u32 {
    let cb = with(|s| {
        let playing = s.engine.state(e as u32) != ffx_sound::replay::STATE_READY;
        s.call("Event::stop", &[e as u32, immediate & 0xff], &[], &[]);
        if playing {
            s.callbacks.get(&(e as u32)).copied()
        } else {
            None
        }
    })
    .flatten();
    // measured: stop calls EVENTFINISHED at once
    if let Some((f, u)) = cb {
        callback(e as u32, ffx_sound::replay::CALLBACK_EVENTFINISHED, f, u);
    }
    OK
}
unsafe extern "system" fn event_get_state(e: usize, out: usize) -> u32 {
    put(out, with(|s| s.engine.state(e as u32)).unwrap_or(1));
    OK
}
unsafe extern "system" fn event_set_callback(e: usize, f: usize, user: usize) -> u32 {
    with(|s| {
        if f == 0 {
            s.callbacks.remove(&(e as u32));
        } else {
            s.callbacks.insert(e as u32, (f, user));
        }
    });
    OK
}
unsafe extern "system" fn event_set_volume(e: usize, v: u32) -> u32 {
    with(|s| s.call("Event::setVolume", &[e as u32, v], &[], &[]));
    OK
}
unsafe extern "system" fn event_get_volume(e: usize, out: usize) -> u32 {
    put(
        out,
        with(|s| s.engine.paused_volume(e as u32).1).unwrap_or(0.0),
    );
    OK
}
unsafe extern "system" fn event_set_paused(e: usize, p: u32) -> u32 {
    with(|s| s.call("Event::setPaused", &[e as u32, p & 0xff], &[], &[]));
    OK
}
unsafe extern "system" fn event_get_paused(e: usize, out: usize) -> u32 {
    put(
        out,
        u8::from(with(|s| s.engine.paused_volume(e as u32).0).unwrap_or(false)),
    );
    OK
}
unsafe extern "system" fn event_get_info(e: usize, index: usize, name: usize, info: usize) -> u32 {
    put(index, 0u32);
    put(name, 0usize);
    if info != 0 {
        let (ms, playing) = with(|s| s.engine.info(e as u32))
            .flatten()
            .unwrap_or((0, false));
        put(info + 8, ms);
        put(info + 0x28, u32::from(playing));
    }
    OK
}
unsafe extern "system" fn event_get_channel_group(e: usize, out: usize) -> u32 {
    let g = (e as u32) | 2;
    with(|s| s.call("Event::getChannelGroup", &[e as u32], &[(1, g)], &[]));
    put(out, g);
    OK
}

// ---- fmodex.dll --------------------------------------------------------

unsafe extern "system" fn memory_initialize(
    _a: usize,
    _b: usize,
    _c: usize,
    _d: usize,
    _e: usize,
    _f: usize,
) -> u32 {
    OK
}
unsafe extern "system" fn memory_get_stats(cur: usize, max: usize, _blocking: u32) -> u32 {
    put(cur, 0u32);
    put(max, 0u32);
    OK
}
unsafe extern "system" fn system_set_file_system(
    _this: usize,
    open: usize,
    close: usize,
    read: usize,
    seek: usize,
    _aread: usize,
    _acancel: usize,
    _align: i32,
) -> u32 {
    with(|s| {
        if let Ok(mut f) = s.files.lock() {
            *f = GameFiles {
                open,
                close,
                read,
                seek,
            };
        }
    });
    OK
}
unsafe extern "system" fn system_set_stream_buffer_size(
    _this: usize,
    _size: u32,
    _unit: u32,
) -> u32 {
    OK
}
unsafe extern "system" fn system_create_sound(
    _this: usize,
    name: usize,
    mode: u32,
    exinfo: usize,
    out: usize,
) -> u32 {
    let r = with(|s| {
        let h = s.handle();
        if name == 0 && (mode & 0x8 != 0 || exinfo != 0) {
            // the movie's user stream (FMOD_OPENUSER, CREATESTREAM)
            // SAFETY: FMOD_CREATESOUNDEXINFO as the movie player fills it.
            let (channels, rate, read) = unsafe {
                (
                    ((exinfo + 0xc) as *const i32).read_unaligned(),
                    ((exinfo + 0x10) as *const i32).read_unaligned(),
                    ((exinfo + 0x2c) as *const usize).read_unaligned(),
                )
            };
            s.movie = Some(Movie {
                sound: h,
                channel: 0,
                read,
                channels: u16::try_from(channels).unwrap_or(2),
                rate: u32::try_from(rate).unwrap_or(48_000),
                voice: None,
                pcm: Vec::new(),
                volume: 1.0,
                paused: false,
            });
            return Some(h);
        }
        let name = cstr(name)?;
        s.sounds.insert(h, Engine::<SharedFiles>::bank_path(&name));
        s.call(
            "System::createSound",
            &[0, 0, mode],
            &[(4, h)],
            &[(1, name)],
        );
        Some(h)
    })
    .flatten();
    match r {
        Some(h) => {
            put(out, h);
            OK
        }
        None => 0x17,
    }
}
unsafe extern "system" fn system_play_sound(
    _this: usize,
    _index: i32,
    sound: usize,
    paused: u32,
    out: usize,
) -> u32 {
    let ch = with(|s| {
        if let Some(m) = s.movie.as_mut()
            && m.sound == sound as u32
        {
            m.channel = (m.sound) | 4;
            m.paused = paused & 0xff != 0;
            return m.channel;
        }
        // CHANNEL_REUSE: the channel the game passes back, else a new one
        // SAFETY: the game's out-pointer, read before it is written.
        let old = if out == 0 {
            0
        } else {
            unsafe { (out as *const u32).read_unaligned() }
        };
        let ch = if old != 0 && old & 0xff == 5 {
            old
        } else {
            (s.handle() & !0xff) | 5
        };
        s.call(
            "System::playSound",
            &[0, 0, sound as u32, paused & 0xff],
            &[(4, ch)],
            &[],
        );
        ch
    });
    put(out, ch.unwrap_or(0));
    OK
}
unsafe extern "system" fn sound_release(sound: usize) -> u32 {
    with(|s| {
        if s.movie.as_ref().is_some_and(|m| m.sound == sound as u32) {
            s.movie = None;
        }
    });
    OK
}
unsafe extern "system" fn sound_get_sub_sound(sound: usize, index: i32, out: usize) -> u32 {
    let h = with(|s| {
        let h = s.handle();
        s.subsounds
            .insert(h, (sound as u32, u32::try_from(index).unwrap_or(0)));
        s.call(
            "Sound::getSubSound",
            &[sound as u32, u32::try_from(index).unwrap_or(0)],
            &[(2, h)],
            &[],
        );
        h
    });
    put(out, h.unwrap_or(0));
    OK
}
unsafe extern "system" fn sound_get_name(sound: usize, buf: usize, len: i32) -> u32 {
    let name = with(|s| {
        let (parent, i) = s.subsounds.get(&(sound as u32)).copied()?;
        let bank = s.sounds.get(&parent)?.clone();
        s.engine.sample_names(&bank).get(i as usize).cloned()
    })
    .flatten()
    .unwrap_or_default();
    if buf != 0 && len > 0 {
        let n = name.len().min(usize::try_from(len).unwrap_or(1) - 1);
        // SAFETY: the game's buffer of `len` bytes.
        unsafe {
            std::ptr::copy_nonoverlapping(name.as_ptr(), buf as *mut u8, n);
            ((buf + n) as *mut u8).write(0);
        }
    }
    OK
}
unsafe extern "system" fn sound_get_num_sub_sounds(sound: usize, out: usize) -> u32 {
    let n = with(|s| {
        let bank = s.sounds.get(&(sound as u32))?.clone();
        Some(s.engine.sample_names(&bank).len())
    })
    .flatten()
    .unwrap_or(0);
    put(out, i32::try_from(n).unwrap_or(0));
    OK
}
unsafe extern "system" fn sound_get_open_state(
    _sound: usize,
    state: usize,
    pct: usize,
    starving: usize,
    busy: usize,
) -> u32 {
    put(state, 0u32); // FMOD_OPENSTATE_READY
    put(pct, 100u32);
    put(starving, 0u8);
    put(busy, 0u8);
    OK
}
unsafe extern "system" fn sound_set_user_data(_sound: usize, _data: usize) -> u32 {
    OK
}
unsafe extern "system" fn sound_get_user_data(_sound: usize, _out: usize) -> u32 {
    OK
}
fn movie_channel(s: &mut State, ch: u32) -> Option<&mut Movie> {
    s.movie.as_mut().filter(|m| m.channel == ch && ch != 0)
}
unsafe extern "system" fn channel_set_volume(ch: usize, v: u32) -> u32 {
    with(|s| {
        if let Some(m) = movie_channel(s, ch as u32) {
            m.volume = f32::from_bits(v);
        } else {
            s.call("Channel::setVolume", &[ch as u32, v], &[], &[]);
        }
    });
    OK
}
unsafe extern "system" fn channel_get_volume(_ch: usize, out: usize) -> u32 {
    put(out, 1.0f32);
    OK
}
unsafe extern "system" fn channel_set_pan(ch: usize, p: u32) -> u32 {
    with(|s| s.call("Channel::setPan", &[ch as u32, p], &[], &[]));
    OK
}
unsafe extern "system" fn channel_get_pan(_ch: usize, out: usize) -> u32 {
    put(out, 0.0f32);
    OK
}
unsafe extern "system" fn channel_get_loop_count(ch: usize, out: usize) -> u32 {
    let looping = with(|s| s.engine.looping(ch as u32 & !0xff | 1)).unwrap_or(false);
    put(out, if looping { -1i32 } else { 0 });
    OK
}
unsafe extern "system" fn channel_set_paused(ch: usize, p: u32) -> u32 {
    with(|s| {
        if let Some(m) = movie_channel(s, ch as u32) {
            m.paused = p & 0xff != 0;
        } else {
            s.call("Channel::setPaused", &[ch as u32, p & 0xff], &[], &[]);
        }
    });
    OK
}
unsafe extern "system" fn channel_stop(ch: usize) -> u32 {
    with(|s| {
        if let Some(m) = movie_channel(s, ch as u32) {
            m.channel = 0;
            m.voice = None;
        }
    });
    OK
}
unsafe extern "system" fn channel_set_priority(_ch: usize, _p: i32) -> u32 {
    OK
}
unsafe extern "system" fn channel_group_get_num_channels(_g: usize, out: usize) -> u32 {
    put(out, 1i32);
    OK
}
unsafe extern "system" fn channel_group_get_channel(g: usize, index: i32, out: usize) -> u32 {
    let ch = (g as u32 & !0xff) | 4;
    with(|s| {
        s.call(
            "ChannelGroup::getChannel",
            &[g as u32, u32::try_from(index).unwrap_or(0)],
            &[(2, ch)],
            &[],
        );
    });
    put(out, ch);
    OK
}

/// Mixes the movie stream into a block: PCM pulled from the game's read
/// callback 0x1000 frames at a time (what it always writes), interpolated
/// to 48 kHz and ramped like every voice.
fn mix_movie(m: &mut Movie, block: &mut [f32]) {
    if m.channel == 0 || m.paused || m.read == 0 {
        return;
    }
    let need = (BLOCK as u64 * u64::from(m.rate)).div_ceil(48_000) as usize + 2;
    let ch = usize::from(m.channels.max(1));
    let have = m.voice.as_ref().map_or(0, |v| {
        let used = (v.pos >> 32) as usize;
        m.pcm.len() / ch - used.min(m.pcm.len() / ch)
    });
    if have < need {
        let mut buf = vec![0i16; 0x1000 * ch];
        // SAFETY: the game's FMOD_SOUND_PCMREADCALLBACK (stdcall: sound,
        // data, length in bytes), with a buffer of the 0x1000 frames it writes.
        unsafe {
            let f: unsafe extern "system" fn(u32, *mut c_void, u32) -> u32 =
                std::mem::transmute(m.read);
            f(
                m.sound,
                buf.as_mut_ptr().cast(),
                u32::try_from(buf.len() * 2).unwrap_or(0),
            );
        }
        m.pcm.extend_from_slice(&buf);
    }
    let frames = u32::try_from(m.pcm.len() / ch).unwrap_or(0);
    let wave = Arc::new(Wave {
        pcm: m.pcm.clone(),
        channels: m.channels,
        rate: m.rate,
        frames,
        looped: None,
    });
    let mut v = match m.voice.take() {
        Some(mut v) => {
            v.wave = wave;
            v
        }
        None => Voice::new(wave, false),
    };
    v.volume = m.volume;
    v.mix(block);
    v.ended = false;
    m.voice = Some(v);
}

/// Points every FMOD import of `FFX.exe` at the stubs when
/// `FFX_RUST_AUDIO=1`.
///
/// # Safety
/// `base` must be the main module; called before other threads run game
/// code, before the other FMOD patches (`fmod_log`, `audio_wav`).
#[allow(clippy::too_many_lines)] // the table of the 40 imports
pub unsafe fn install(base: usize) {
    if std::env::var("FFX_RUST_AUDIO").as_deref() != Ok("1") {
        return;
    }
    // SAFETY: written once here, before the game runs.
    unsafe {
        let p = &raw mut PROJECT_VT;
        for (i, f) in [
            project_release as *const () as usize,
            project_get_info as *const () as usize,
            project_get_group as *const () as usize,
        ]
        .into_iter()
        .enumerate()
        {
            (*p)[i] = f;
        }
        for i in 3..14 {
            (*p)[i] = unhandled0 as *const () as usize;
        }
        (*p)[0x24 / 4] = project_stop_all as *const () as usize;
        (*p)[0x28 / 4] = project_cancel as *const () as usize;
        let g = &raw mut GROUP_VT;
        for i in 0..18 {
            (*g)[i] = unhandled0 as *const () as usize;
        }
        (*g)[1] = group_load_data as *const () as usize;
        (*g)[2] = group_free_data as *const () as usize;
        (*g)[0x20 / 4] = group_get_event as *const () as usize;
        (*g)[0x24 / 4] = group_get_event_by_index as *const () as usize;
    }
    let files = Arc::new(Mutex::new(GameFiles::default()));
    let wav = std::env::var("FFX_AUDIO_WAV")
        .ok()
        .filter(|p| !p.is_empty())
        .and_then(|p| {
            let mut f = std::fs::File::create(&p).ok()?;
            // FMOD's WAV writer header: float, 2 channels, 48 kHz, sizes 0
            let mut h = Vec::new();
            h.extend_from_slice(b"RIFF");
            h.extend_from_slice(&0x38u32.to_le_bytes());
            h.extend_from_slice(b"WAVEfmt ");
            h.extend_from_slice(&0x28u32.to_le_bytes());
            h.extend_from_slice(&3u16.to_le_bytes());
            h.extend_from_slice(&2u16.to_le_bytes());
            h.extend_from_slice(&48_000u32.to_le_bytes());
            h.extend_from_slice(&(48_000u32 * 8).to_le_bytes());
            h.extend_from_slice(&8u16.to_le_bytes());
            h.extend_from_slice(&32u16.to_le_bytes());
            h.extend_from_slice(&[0u8; 24]);
            h.extend_from_slice(b"data");
            h.extend_from_slice(&0u32.to_le_bytes());
            f.write_all(&h).ok()?;
            Some(f)
        });
    if let Ok(mut s) = STATE.lock() {
        *s = Some(State {
            engine: Engine::new(SharedFiles(files.clone())),
            files,
            next: 0x100,
            callbacks: HashMap::new(),
            sounds: HashMap::new(),
            subsounds: HashMap::new(),
            movie: None,
            wav,
        });
    }
    let ev = "fmod_event.dll";
    let ex = "fmodex.dll";
    let stubs: [(&str, &str, usize); 40] = [
        (
            ev,
            "?getChannelGroup@Event@FMOD@@QAG?AW4FMOD_RESULT@@PAPAVChannelGroup@2@@Z",
            event_get_channel_group as *const () as usize,
        ),
        (
            ev,
            "?getInfo@Event@FMOD@@QAG?AW4FMOD_RESULT@@PAHPAPADPAUFMOD_EVENT_INFO@@@Z",
            event_get_info as *const () as usize,
        ),
        (
            ev,
            "?getPaused@Event@FMOD@@QAG?AW4FMOD_RESULT@@PA_N@Z",
            event_get_paused as *const () as usize,
        ),
        (
            ev,
            "?setPaused@Event@FMOD@@QAG?AW4FMOD_RESULT@@_N@Z",
            event_set_paused as *const () as usize,
        ),
        (
            ev,
            "?getVolume@Event@FMOD@@QAG?AW4FMOD_RESULT@@PAM@Z",
            event_get_volume as *const () as usize,
        ),
        (
            ev,
            "?setVolume@Event@FMOD@@QAG?AW4FMOD_RESULT@@M@Z",
            event_set_volume as *const () as usize,
        ),
        (
            ev,
            "?setCallback@Event@FMOD@@QAG?AW4FMOD_RESULT@@P6G?AW43@PAUFMOD_EVENT@@W4FMOD_EVENT_CALLBACKTYPE@@PAX22@Z2@Z",
            event_set_callback as *const () as usize,
        ),
        (
            ev,
            "?getState@Event@FMOD@@QAG?AW4FMOD_RESULT@@PAI@Z",
            event_get_state as *const () as usize,
        ),
        (
            ev,
            "?stop@Event@FMOD@@QAG?AW4FMOD_RESULT@@_N@Z",
            event_stop as *const () as usize,
        ),
        (
            ev,
            "?start@Event@FMOD@@QAG?AW4FMOD_RESULT@@XZ",
            event_start as *const () as usize,
        ),
        (
            ev,
            "?load@EventSystem@FMOD@@QAG?AW4FMOD_RESULT@@PBDPAUFMOD_EVENT_LOADINFO@@PAPAVEventProject@2@@Z",
            event_system_load as *const () as usize,
        ),
        (
            ev,
            "?unload@EventSystem@FMOD@@QAG?AW4FMOD_RESULT@@XZ",
            event_system_unload as *const () as usize,
        ),
        (
            ev,
            "?getSystemObject@EventSystem@FMOD@@QAG?AW4FMOD_RESULT@@PAPAVSystem@2@@Z",
            event_system_get_system as *const () as usize,
        ),
        (
            ev,
            "?update@EventSystem@FMOD@@QAG?AW4FMOD_RESULT@@XZ",
            event_system_update as *const () as usize,
        ),
        (
            ev,
            "?release@EventSystem@FMOD@@QAG?AW4FMOD_RESULT@@XZ",
            event_system_release as *const () as usize,
        ),
        (
            ev,
            "?init@EventSystem@FMOD@@QAG?AW4FMOD_RESULT@@HIPAXI@Z",
            event_system_init as *const () as usize,
        ),
        (
            ev,
            "FMOD_EventSystem_Create",
            event_system_create as *const () as usize,
        ),
        (
            ex,
            "?setPriority@Channel@FMOD@@QAG?AW4FMOD_RESULT@@H@Z",
            channel_set_priority as *const () as usize,
        ),
        (
            ex,
            "?setPaused@Channel@FMOD@@QAG?AW4FMOD_RESULT@@_N@Z",
            channel_set_paused as *const () as usize,
        ),
        (
            ex,
            "?stop@Channel@FMOD@@QAG?AW4FMOD_RESULT@@XZ",
            channel_stop as *const () as usize,
        ),
        (
            ex,
            "?getUserData@Sound@FMOD@@QAG?AW4FMOD_RESULT@@PAPAX@Z",
            sound_get_user_data as *const () as usize,
        ),
        (
            ex,
            "?setUserData@Sound@FMOD@@QAG?AW4FMOD_RESULT@@PAX@Z",
            sound_set_user_data as *const () as usize,
        ),
        (
            ex,
            "?getOpenState@Sound@FMOD@@QAG?AW4FMOD_RESULT@@PAW4FMOD_OPENSTATE@@PAIPA_N2@Z",
            sound_get_open_state as *const () as usize,
        ),
        (
            ex,
            "FMOD_Memory_GetStats",
            memory_get_stats as *const () as usize,
        ),
        (
            ex,
            "?setFileSystem@System@FMOD@@QAG?AW4FMOD_RESULT@@P6G?AW43@PBDHPAIPAPAX2@ZP6G?AW43@PAX4@ZP6G?AW43@44I14@ZP6G?AW43@4I4@ZP6G?AW43@PAUFMOD_ASYNCREADINFO@@4@Z5H@Z",
            system_set_file_system as *const () as usize,
        ),
        (
            ex,
            "?setStreamBufferSize@System@FMOD@@QAG?AW4FMOD_RESULT@@II@Z",
            system_set_stream_buffer_size as *const () as usize,
        ),
        (
            ex,
            "?createSound@System@FMOD@@QAG?AW4FMOD_RESULT@@PBDIPAUFMOD_CREATESOUNDEXINFO@@PAPAVSound@2@@Z",
            system_create_sound as *const () as usize,
        ),
        (
            ex,
            "?playSound@System@FMOD@@QAG?AW4FMOD_RESULT@@W4FMOD_CHANNELINDEX@@PAVSound@2@_NPAPAVChannel@2@@Z",
            system_play_sound as *const () as usize,
        ),
        (
            ex,
            "?release@Sound@FMOD@@QAG?AW4FMOD_RESULT@@XZ",
            sound_release as *const () as usize,
        ),
        (
            ex,
            "?getSubSound@Sound@FMOD@@QAG?AW4FMOD_RESULT@@HPAPAV12@@Z",
            sound_get_sub_sound as *const () as usize,
        ),
        (
            ex,
            "?getName@Sound@FMOD@@QAG?AW4FMOD_RESULT@@PADH@Z",
            sound_get_name as *const () as usize,
        ),
        (
            ex,
            "?getNumSubSounds@Sound@FMOD@@QAG?AW4FMOD_RESULT@@PAH@Z",
            sound_get_num_sub_sounds as *const () as usize,
        ),
        (
            ex,
            "?setVolume@Channel@FMOD@@QAG?AW4FMOD_RESULT@@M@Z",
            channel_set_volume as *const () as usize,
        ),
        (
            ex,
            "?getVolume@Channel@FMOD@@QAG?AW4FMOD_RESULT@@PAM@Z",
            channel_get_volume as *const () as usize,
        ),
        (
            ex,
            "?setPan@Channel@FMOD@@QAG?AW4FMOD_RESULT@@M@Z",
            channel_set_pan as *const () as usize,
        ),
        (
            ex,
            "?getPan@Channel@FMOD@@QAG?AW4FMOD_RESULT@@PAM@Z",
            channel_get_pan as *const () as usize,
        ),
        (
            ex,
            "FMOD_Memory_Initialize",
            memory_initialize as *const () as usize,
        ),
        (
            ex,
            "?getLoopCount@Channel@FMOD@@QAG?AW4FMOD_RESULT@@PAH@Z",
            channel_get_loop_count as *const () as usize,
        ),
        (
            ex,
            "?getChannel@ChannelGroup@FMOD@@QAG?AW4FMOD_RESULT@@HPAPAVChannel@2@@Z",
            channel_group_get_channel as *const () as usize,
        ),
        (
            ex,
            "?getNumChannels@ChannelGroup@FMOD@@QAG?AW4FMOD_RESULT@@PAH@Z",
            channel_group_get_num_channels as *const () as usize,
        ),
    ];
    let mut done = 0;
    for (dll, name, f) in stubs {
        // SAFETY: forwarded from the caller; an import slot of the image.
        if unsafe { crate::iat::patch(base, dll, name, f) }.is_some() {
            done += 1;
        } else {
            log(&format!("  rust audio: {dll} {name} not imported"));
        }
    }
    ON.store(true, Ordering::Release);
    log(&format!(
        "  rust audio: {done} of 40 FMOD imports replaced by the Rust engine"
    ));
}
