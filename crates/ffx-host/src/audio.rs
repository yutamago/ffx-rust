//! FMOD's 40 imports (`fmod_event.dll` 17, `fmodex.dll` 23) on the Rust
//! mixer (docs/host.md, "FMOD"): the same engine the hooked game's
//! `FFX_RUST_AUDIO=1` drives (`crates/ffx-hook/src/rust_audio.rs`,
//! [`ffx_sound::replay::Engine`], checked sample for sample against FMOD's
//! captures by the audio gate), with the same calls.
//!
//! - The objects the game calls through vtables (`EventProject`,
//!   `EventGroup`) are 8-byte blocks on the arena heap whose vtables hold
//!   traps (`Shared::add_trap`); event, channel and sound handles are
//!   numbers the game never reads through.
//! - Files are read through the game's own file callbacks
//!   (`System::setFileSystem`, run in Unicorn), so the VBF archive and mods
//!   work as with FMOD.
//! - Each `EventSystem::update` mixes one 1,600-frame block (one FMOD tick
//!   of game time: the clock is virtual, `crate::clock`) and raises the end
//!   callbacks where FMOD does.
//! - The output: none (the null output: the blocks are counted), or with
//!   `--audio-wav FILE` (`Options::audio_wav`) the mix in FMOD's WAV writer
//!   format, as the audio gate's captures (`FFX_AUDIO_WAV`).
//! - The movie's user stream (`createSound` with `FMOD_OPENUSER`) is
//!   mixed from the game's read callback ([`movie`]).

use std::cell::Cell;
use std::collections::HashMap;
use std::io::Write as _;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use ffx_bridge::Mem as _;
use ffx_sound::mixer::BLOCK;
use ffx_sound::replay::{Call, Engine, Files};

use crate::exec::{Exec, Trap};

mod movie;

/// The mix on the default output device (WASAPI, shared mode): the hook's
/// `rust_audio_out.rs`, the same file, compiled here too. Off in harness
/// games (their clock is virtual, as the hook keeps it off under the
/// virtual clock) unless `FFX_RUST_AUDIO_OUT=1`; off with `--audio-wav`.
#[path = "../../ffx-hook/src/rust_audio_out.rs"]
#[allow(dead_code)] // `on` is the hook's; off Windows only `start` and `push`
#[cfg_attr(not(windows), allow(unused_imports))]
mod device;

/// Starts the device output once, as [`device`] says.
fn start_device(wav: bool) {
    let env = |n: &str| std::env::var(n).unwrap_or_default();
    let harness = !env("FFX_HARNESS").is_empty();
    if wav || (harness && env("FFX_RUST_AUDIO_OUT") != "1") {
        return;
    }
    device::start();
}
use crate::imports::{ImportFn, a};
use crate::table::{Args, Conv, Ret};

const OK: u32 = 0;
/// `FMOD_ERR_INVALID_PARAM`.
const ERR_PARAM: u32 = 0x25;
/// `FMOD_ERR_FILE_NOTFOUND`.
const ERR_FILE_NOT_FOUND: u32 = 0x17;
/// Where the game's sound files are, as it names them to FMOD.
const ROOT: &str = "../../../FFX_Data/GameData/PS3Data/Sound_PC/";

thread_local! {
    /// The executor of the import running on this thread, for the file
    /// callbacks the engine reaches while it runs ([`with_exec`]).
    static EXEC: Cell<*mut Exec> = const { Cell::new(std::ptr::null_mut()) };
}

/// Runs `f` with `e` reachable from [`GameFiles`] on this thread.
fn with_exec<T>(e: &mut Exec, f: impl FnOnce() -> T) -> T {
    let before = EXEC.with(|c| c.replace(std::ptr::from_mut(e)));
    let r = f();
    EXEC.with(|c| c.set(before));
    r
}

/// The game's file callbacks (`FmodFile_open/close/read/seek`, stdcall).
#[derive(Debug, Clone, Copy, Default)]
struct GameFiles {
    open: u32,
    close: u32,
    read: u32,
    seek: u32,
}

impl GameFiles {
    /// `len` bytes from `at` (all from `at` with `None`) of a file below
    /// `Sound_PC/`, through the callbacks.
    fn range(self, path: &str, at: usize, len: Option<usize>) -> Option<Vec<u8>> {
        if self.open == 0 {
            return None;
        }
        let p = EXEC.with(Cell::get);
        if p.is_null() {
            return None;
        }
        // SAFETY: set by `with_exec` from the `&mut Exec` of the import
        // running on this thread, which does not touch it until the engine
        // call that got here returns.
        let e = unsafe { &mut *p };
        let name = format!("{ROOT}{path}\0");
        let scratch = e.alloc(u32::try_from(name.len()).ok()? + 16);
        if scratch == 0 {
            return None;
        }
        let out = scratch + u32::try_from(name.len()).ok()?.next_multiple_of(4);
        let mut mem = e.mem();
        mem.write_bytes(scratch, name.as_bytes());
        mem.write_bytes(out, &[0u8; 12]);
        let r = e.call_stdcall(self.open, &[scratch, 0, out, out + 4, out + 8]);
        let (size, handle, user): (u32, u32, u32) =
            (mem.read(out), mem.read(out + 4), mem.read(out + 8));
        if r.ok() != Some(0) {
            e.release(scratch);
            return None;
        }
        let len = len.unwrap_or((size as usize).saturating_sub(at));
        let mut buf = vec![0u8; len];
        let chunk = 1u32 << 20;
        let block = e.alloc(chunk + 4);
        let mut done = 0usize;
        let mut ok = block != 0
            && e.call_stdcall(self.seek, &[handle, u32::try_from(at).ok()?, user])
                .ok()
                == Some(0);
        while ok && done < len {
            let want = u32::try_from((len - done).min(chunk as usize)).unwrap_or(0);
            mem.write(block + chunk, 0u32);
            let r = e
                .call_stdcall(self.read, &[handle, block, want, block + chunk, user])
                .unwrap_or(u32::MAX);
            let got = mem.read::<u32>(block + chunk).min(want) as usize;
            mem.read_bytes(block, &mut buf[done..done + got]);
            done += got;
            ok = (r == 0 || r == 0x16) && got > 0;
        }
        let _ = e.call_stdcall(self.close, &[handle, user]);
        if block != 0 {
            e.release(block);
        }
        e.release(scratch);
        buf.truncate(done);
        Some(buf)
    }
}

/// The engine's files: the game's callbacks.
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

struct State {
    engine: Engine<SharedFiles>,
    files: Arc<Mutex<GameFiles>>,
    next: u32,
    callbacks: HashMap<u32, (u32, u32)>,
    sounds: HashMap<u32, String>,
    subsounds: HashMap<u32, (u32, u32)>,
    movie: Option<movie::Movie>,
    /// Movie chunks to read after the block just mixed.
    reads: usize,
    wav: Option<std::fs::File>,
    /// Blocks mixed.
    updates: u64,
    /// The project and group vtables (arena), once made.
    project_vt: u32,
    group_vt: u32,
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
        self.engine.call(&Call {
            frame: 0,
            function: function.to_owned(),
            args: args.to_vec(),
            result: 0,
            outs: outs.to_vec(),
            strings: strings.to_vec(),
        });
    }
}

/// The mixer's state (`Shared::audio`).
#[derive(Default)]
pub struct Audio {
    state: Mutex<Option<State>>,
    /// Where the mix goes (`--audio-wav`), set before the run.
    pub wav: Mutex<Option<std::path::PathBuf>>,
    /// `<wav>.marks` ([`Audio::capture`]): where the harness's
    /// `audiomark`s fall in the WAV.
    marks: Mutex<Option<std::fs::File>>,
    /// Blocks mixed ([`BLOCK`] frames each): the mixer's sample clock is
    /// this times [`BLOCK`], FMOD's `System::getDSPClock` after as many
    /// `EventSystem::update`s.
    blocks: AtomicU64,
}

impl std::fmt::Debug for Audio {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("audio::Audio")
    }
}

impl Audio {
    /// The audio capture (docs/harness.md, "Audio capture"): the mix goes
    /// to the WAV file `wav` (FMOD's WAV writer format, as the hook's
    /// `FFX_AUDIO_WAV` capture writes it) and `<wav>.marks` beside it
    /// gets the format line and, later, one line per [`Audio::mark`].
    /// Call before the run.
    ///
    /// # Errors
    /// When the marks file can't be made.
    pub fn capture(&self, wav: &std::path::Path) -> std::io::Result<()> {
        if let Ok(mut w) = self.wav.lock() {
            *w = Some(wav.to_owned());
        }
        let mut name = wav.as_os_str().to_owned();
        name.push(".marks");
        let mut f = std::fs::File::create(std::path::PathBuf::from(name))?;
        f.write_all(marks_header(&wav.display().to_string()).as_bytes())?;
        if let Ok(mut m) = self.marks.lock() {
            *m = Some(f);
        }
        Ok(())
    }

    /// The harness's `audiomark <name>` at `frame` (the main thread held at
    /// that boundary, so the `FmodManager` thread waits for its turn and
    /// the mixer's clock is the same in every run): the hook's marks line,
    /// written to the marks file when there is one, and its reply.
    /// `turns`: FMOD ticks in turns ([`crate::clock::Clock::fmod_turn_state`]),
    /// `late`: turns the boundary gave up waiting for.
    pub fn mark(&self, name: &str, frame: u32, turns: bool, late: u32) -> String {
        let blocks = self.blocks.load(Ordering::Acquire);
        self.mark_line(&mark_line(name, frame, blocks, late, turns));
        mark_reply(frame, blocks, late)
    }

    /// A turn the boundary gave up waiting for (the hook's `late` line).
    pub fn note_late(&self, frame: u32) {
        self.mark_line(&format!("late frame {frame}"));
    }

    fn mark_line(&self, line: &str) {
        if let Ok(mut m) = self.marks.lock()
            && let Some(f) = m.as_mut()
        {
            let _ = writeln!(f, "{line}");
            let _ = f.flush();
        }
    }

    /// Blocks mixed and calls the engine did not follow.
    #[must_use]
    pub fn summary(&self) -> Option<(u64, Vec<(String, u32)>)> {
        let s = self.state.lock().ok()?;
        let s = s.as_ref()?;
        let mut ignored: Vec<_> = s
            .engine
            .ignored
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        ignored.sort();
        Some((s.updates, ignored))
    }
}

/// Runs `f` on the state (made on first use).
fn with<T>(e: &mut Exec, f: impl FnOnce(&mut State) -> T) -> Option<T> {
    let shared = Arc::clone(e.shared());
    let mut guard = shared.audio.state.lock().ok()?;
    if guard.is_none() {
        let files = Arc::new(Mutex::new(GameFiles::default()));
        let wav = shared
            .audio
            .wav
            .lock()
            .ok()
            .and_then(|w| w.clone())
            .and_then(|p| wav_file(&p));
        start_device(wav.is_some());
        *guard = Some(State {
            engine: Engine::new(SharedFiles(Arc::clone(&files))),
            files,
            next: 0x100,
            callbacks: HashMap::new(),
            sounds: HashMap::new(),
            subsounds: HashMap::new(),
            movie: None,
            reads: 0,
            wav,
            updates: 0,
            project_vt: 0,
            group_vt: 0,
        });
    }
    let s = guard.as_mut()?;
    Some(with_exec(e, || f(s)))
}

/// The marks file's first lines: the hook's header and the format FMOD's
/// WAV writer runs with under the capture (48 kHz, `PCMFLOAT` (5), 2
/// channels, 32 bits, blocks of [`BLOCK`], output `WAVWRITER_NRT` (5)).
fn marks_header(wav: &str) -> String {
    format!(
        "# audiomarks 1 {wav}\nformat rate 48000 format 5 channels 2 bits 32 block {BLOCK} output 5 init 0x0\n"
    )
}

/// One `audiomark` line, as the hook writes it (`free`: the hook's free
/// FMOD ticks; the clock here gives none).
fn mark_line(name: &str, frame: u32, blocks: u64, late: u32, turns: bool) -> String {
    format!(
        "mark {name} frame {frame} sample {} updates {blocks} free 0 late {late} turns {turns}",
        blocks * BLOCK as u64
    )
}

/// The reply to `audiomark`, as the hook's.
fn mark_reply(frame: u32, blocks: u64, late: u32) -> String {
    format!(
        "ok frame {frame} sample {} updates {blocks} free 0 late {late}",
        blocks * BLOCK as u64
    )
}

/// A WAV file in FMOD's writer format: float, 2 channels, 48 kHz, sizes 0.
fn wav_file(p: &std::path::Path) -> Option<std::fs::File> {
    let mut f = std::fs::File::create(p).ok()?;
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
}

fn put<T: ffx_bridge::Value>(e: &Exec, p: u32, v: T) {
    if p != 0 {
        e.mem().write(p, v);
    }
}

fn cstr(e: &Exec, p: u32) -> Option<String> {
    if p == 0 {
        return None;
    }
    let mem = e.mem();
    let mut b = Vec::new();
    let mut at = p;
    while b.len() < 4096 {
        let c: u8 = mem.read(at);
        if c == 0 {
            break;
        }
        b.push(c);
        at += 1;
    }
    Some(String::from_utf8_lossy(&b).into_owned())
}

// ---- the objects (vtables of traps) ----------------------------------------

/// Project and group vtables: slot count, and each slot's stack words
/// (`this` included); the unlisted slots take `this` only and return OK.
const PROJECT_SLOTS: usize = 14;
const GROUP_SLOTS: usize = 18;
const KIND_PROJECT: u32 = 1;
const KIND_GROUP: u32 = 2;

fn slot_words(kind: u32, slot: usize) -> usize {
    match (kind, slot) {
        (KIND_PROJECT, 1 | 9) => 2,
        (KIND_PROJECT, 2) | (KIND_GROUP, 8 | 9) => 4,
        (KIND_GROUP, 1 | 2) => 3,
        _ => 1,
    }
}

/// Makes the two vtables in the arena: a slot table and an `int3` per
/// slot, each a trap running [`object_method`].
fn vtables(e: &mut Exec) -> (u32, u32) {
    let Ok(at) = e.map_region(0x1_0000, "FMOD vtables") else {
        return (0, 0);
    };
    let arena = Arc::clone(e.arena());
    let mut mem = e.mem();
    let mut stub = at + 0x400;
    let mut out = [0u32; 2];
    for (i, (kind, n)) in [(KIND_PROJECT, PROJECT_SLOTS), (KIND_GROUP, GROUP_SLOTS)]
        .into_iter()
        .enumerate()
    {
        let table = at + 0x100 * u32::try_from(i).unwrap_or(0);
        out[i] = table;
        for slot in 0..n {
            mem.write(table + 4 * u32::try_from(slot).unwrap_or(0), stub);
            mem.write(stub, 0xccu8);
            e.shared().add_trap(
                stub,
                Trap {
                    run: object_method,
                    tag: (kind << 8) | u32::try_from(slot).unwrap_or(0),
                    conv: Conv::Stdcall,
                    words: slot_words(kind, slot),
                },
            );
            stub += 1;
        }
    }
    drop(arena);
    (out[0], out[1])
}

/// A project or group object: `{vtable, project}` on the heap.
fn object(e: &mut Exec, kind: u32, project: u32) -> u32 {
    let vt = with(e, |s| {
        if kind == KIND_PROJECT {
            s.project_vt
        } else {
            s.group_vt
        }
    })
    .unwrap_or(0);
    let vt = if vt == 0 {
        let (p, g) = vtables(e);
        with(e, |s| {
            s.project_vt = p;
            s.group_vt = g;
        });
        if kind == KIND_PROJECT { p } else { g }
    } else {
        vt
    };
    let o = e.alloc(8);
    if o != 0 {
        let mut mem = e.mem();
        mem.write(o, vt);
        mem.write(o + 4, project);
    }
    o
}

fn object_method(e: &mut Exec, tag: u32, args: &Args) -> Ret {
    let w = |i: usize| args.stack.get(i).copied().unwrap_or(0);
    let this = w(0);
    let r = match (tag >> 8, (tag & 0xff) as usize) {
        (KIND_PROJECT, 0) => {
            with(e, |s| s.call("EventProjectI::release", &[this], &[], &[]));
            OK
        }
        (KIND_PROJECT, 1) => {
            let n = with(e, |s| s.engine.playing_events(this)).unwrap_or(0);
            put(e, w(1) + 0x114, n);
            OK
        }
        (KIND_PROJECT, 2) => {
            let Some(name) = cstr(e, w(1)) else {
                return Ret::eax(ERR_PARAM);
            };
            let g = object(e, KIND_GROUP, this);
            with(e, |s| {
                s.call("EventProjectI::getGroup", &[this], &[(3, g)], &[(1, name)]);
            });
            put(e, w(3), g);
            OK
        }
        (KIND_PROJECT, 9) => {
            with(e, |s| {
                s.call("EventProjectI::stopAllEvents", &[this], &[], &[]);
            });
            OK
        }
        (KIND_GROUP, 2) => {
            let ev = w(1);
            with(e, |s| s.call("Event::stop", &[ev, 1], &[], &[]));
            OK
        }
        (KIND_GROUP, 8) => {
            let Some(name) = cstr(e, w(1)) else {
                return Ret::eax(ERR_PARAM);
            };
            let h = with(e, |s| {
                let h = s.handle();
                s.call("EventGroupI::getEvent", &[this], &[(3, h)], &[(1, name)]);
                h
            });
            put(e, w(3), h.unwrap_or(0));
            OK
        }
        (KIND_GROUP, 9) => {
            let index = w(1);
            let h = with(e, |s| {
                let h = s.handle();
                s.call(
                    "EventGroupI::getEventByIndex",
                    &[this, index],
                    &[(3, h)],
                    &[],
                );
                h
            });
            put(e, w(3), h.unwrap_or(0));
            OK
        }
        _ => OK,
    };
    Ret::eax(r)
}

// ---- fmod_event.dll ----------------------------------------------------------

fn event_system_create(e: &mut Exec, args: &Args) -> Ret {
    put(e, a(args, 0), 0x00e5_0001u32);
    Ret::eax(OK)
}

fn event_system_get_system(e: &mut Exec, args: &Args) -> Ret {
    put(e, a(args, 1), 0x00e5_0002u32);
    Ret::eax(OK)
}

fn ok(_: &mut Exec, _: &Args) -> Ret {
    Ret::eax(OK)
}

/// `EventSystem::update`: one block, then the end callbacks (outside the
/// state's lock: the game's callback may call FMOD again).
fn event_system_update(e: &mut Exec, _: &Args) -> Ret {
    let due = with(e, |s| {
        s.engine.update_frames(BLOCK);
        let mut block = std::mem::take(&mut s.engine.out);
        s.reads = s.movie.as_mut().map_or(0, |m| m.mix(&mut block));
        if let Some(f) = s.wav.as_mut() {
            let bytes: Vec<u8> = block.iter().flat_map(|x| x.to_le_bytes()).collect();
            let _ = f.write_all(&bytes);
        } else {
            device::push(&block);
        }
        s.updates += 1;
        s.engine
            .callbacks()
            .iter()
            .filter_map(|&(ev, kind)| s.callbacks.get(&ev).map(|&(f, u)| (ev, kind, f, u)))
            .collect::<Vec<_>>()
    });
    if due.is_some() {
        e.shared().audio.blocks.fetch_add(1, Ordering::AcqRel);
    }
    let due = due.unwrap_or_default();
    let reads = with(e, |s| std::mem::take(&mut s.reads)).unwrap_or(0);
    movie_reads(e, reads);
    for (ev, kind, f, user) in due {
        callback(e, ev, kind, f, user);
    }
    Ret::eax(OK)
}

/// The game's `FMOD_EVENT_CALLBACK` (`eventcallback` `0x00707a50`,
/// stdcall: event, type, param1, param2, userdata).
fn callback(e: &mut Exec, event: u32, kind: u32, f: u32, user: u32) {
    if f != 0 {
        let _ = e.call_stdcall(f, &[event, kind, 0, 0, user]);
    }
}

fn event_system_load(e: &mut Exec, args: &Args) -> Ret {
    let Some(name) = cstr(e, a(args, 1)) else {
        return Ret::eax(ERR_PARAM);
    };
    let p = object(e, KIND_PROJECT, 0);
    let loaded = with(e, |s| {
        s.call("EventSystem::load", &[0], &[(3, p)], &[(1, name.clone())]);
        s.engine.has_project(p)
    })
    .unwrap_or(false);
    crate::log::note(&format!("fmod: EventSystem::load {name}: {loaded}"));
    if !loaded {
        return Ret::eax(ERR_FILE_NOT_FOUND);
    }
    put(e, a(args, 3), p);
    Ret::eax(OK)
}

fn event_start(e: &mut Exec, args: &Args) -> Ret {
    let ev = a(args, 0);
    with(e, |s| s.call("Event::start", &[ev], &[], &[]));
    Ret::eax(OK)
}

fn event_stop(e: &mut Exec, args: &Args) -> Ret {
    let (ev, immediate) = (a(args, 0), a(args, 1) & 0xff);
    let cb = with(e, |s| {
        let playing = s.engine.state(ev) != ffx_sound::replay::STATE_READY;
        s.call("Event::stop", &[ev, immediate], &[], &[]);
        if playing {
            s.callbacks.get(&ev).copied()
        } else {
            None
        }
    })
    .flatten();
    // measured: stop calls EVENTFINISHED at once
    if let Some((f, u)) = cb {
        callback(e, ev, ffx_sound::replay::CALLBACK_EVENTFINISHED, f, u);
    }
    Ret::eax(OK)
}

fn event_get_state(e: &mut Exec, args: &Args) -> Ret {
    let ev = a(args, 0);
    let st = with(e, |s| s.engine.state(ev)).unwrap_or(1);
    put(e, a(args, 1), st);
    Ret::eax(OK)
}

fn event_set_callback(e: &mut Exec, args: &Args) -> Ret {
    let (ev, f, user) = (a(args, 0), a(args, 1), a(args, 2));
    with(e, |s| {
        if f == 0 {
            s.callbacks.remove(&ev);
        } else {
            s.callbacks.insert(ev, (f, user));
        }
    });
    Ret::eax(OK)
}

fn event_set_volume(e: &mut Exec, args: &Args) -> Ret {
    let (ev, v) = (a(args, 0), a(args, 1));
    with(e, |s| s.call("Event::setVolume", &[ev, v], &[], &[]));
    Ret::eax(OK)
}

fn event_get_volume(e: &mut Exec, args: &Args) -> Ret {
    let ev = a(args, 0);
    let v = with(e, |s| s.engine.paused_volume(ev).1).unwrap_or(0.0);
    put(e, a(args, 1), v);
    Ret::eax(OK)
}

fn event_set_paused(e: &mut Exec, args: &Args) -> Ret {
    let (ev, p) = (a(args, 0), a(args, 1) & 0xff);
    with(e, |s| s.call("Event::setPaused", &[ev, p], &[], &[]));
    Ret::eax(OK)
}

fn event_get_paused(e: &mut Exec, args: &Args) -> Ret {
    let ev = a(args, 0);
    let p = with(e, |s| s.engine.paused_volume(ev).0).unwrap_or(false);
    put(e, a(args, 1), u8::from(p));
    Ret::eax(OK)
}

fn event_get_info(e: &mut Exec, args: &Args) -> Ret {
    let (ev, index, name, info) = (a(args, 0), a(args, 1), a(args, 2), a(args, 3));
    put(e, index, 0u32);
    put(e, name, 0u32);
    if info != 0 {
        let (ms, playing) = with(e, |s| s.engine.info(ev))
            .flatten()
            .unwrap_or((0, false));
        put(e, info + 8, ms);
        put(e, info + 0x28, u32::from(playing));
    }
    Ret::eax(OK)
}

fn event_get_channel_group(e: &mut Exec, args: &Args) -> Ret {
    let ev = a(args, 0);
    let g = ev | 2;
    with(e, |s| {
        s.call("Event::getChannelGroup", &[ev], &[(1, g)], &[]);
    });
    put(e, a(args, 1), g);
    Ret::eax(OK)
}

// ---- fmodex.dll --------------------------------------------------------------

fn memory_get_stats(e: &mut Exec, args: &Args) -> Ret {
    put(e, a(args, 0), 0u32);
    put(e, a(args, 1), 0u32);
    Ret::eax(OK)
}

fn system_set_file_system(e: &mut Exec, args: &Args) -> Ret {
    let f = GameFiles {
        open: a(args, 1),
        close: a(args, 2),
        read: a(args, 3),
        seek: a(args, 4),
    };
    with(e, |s| {
        if let Ok(mut g) = s.files.lock() {
            *g = f;
        }
    });
    Ret::eax(OK)
}

fn system_create_sound(e: &mut Exec, args: &Args) -> Ret {
    let (name, mode, exinfo, out) = (a(args, 1), a(args, 2), a(args, 3), a(args, 4));
    let movie = name == 0 && (mode & 0x8 != 0 || exinfo != 0);
    let text = cstr(e, name);
    let r = with(e, |s| {
        let h = s.handle();
        if movie {
            // the movie's user stream: made below, read twice
            s.movie = None;
            return Some(h);
        }
        let name = text?;
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
    if movie && let Some(h) = r {
        let m = movie::Movie::new(e, h, exinfo);
        with(e, |s| s.movie = Some(m));
        movie_reads(e, 2);
    }
    match r {
        Some(h) => {
            put(e, out, h);
            Ret::eax(OK)
        }
        None => Ret::eax(ERR_FILE_NOT_FOUND),
    }
}

/// Runs the movie's read callback `n` times (outside the state's lock:
/// the callback asks for the sound's user data) and queues the chunks.
fn movie_reads(e: &mut Exec, n: usize) {
    for _ in 0..n {
        let Some(req) = with(e, |s| s.movie.as_ref().and_then(movie::Movie::request)).flatten()
        else {
            return;
        };
        let samples = movie::read(e, req);
        with(e, |s| {
            if let Some(m) = s.movie.as_mut().filter(|m| m.sound == req.1) {
                m.push(&samples);
            }
        });
    }
}

fn is_movie_channel(s: &State, ch: u32) -> bool {
    ch != 0 && s.movie.as_ref().is_some_and(|m| m.channel == ch)
}

fn system_play_sound(e: &mut Exec, args: &Args) -> Ret {
    let (sound, paused, out) = (a(args, 2), a(args, 3) & 0xff, a(args, 4));
    let old: u32 = if out == 0 { 0 } else { e.mem().read(out) };
    let ch = with(e, |s| {
        if let Some(m) = s.movie.as_mut().filter(|m| m.sound == sound) {
            m.channel = sound | 4;
            m.paused = paused != 0;
            return m.channel;
        }
        // CHANNEL_REUSE: the channel the game passes back, else a new one
        let ch = if old != 0 && old & 0xff == 5 {
            old
        } else {
            (s.handle() & !0xff) | 5
        };
        s.call("System::playSound", &[0, 0, sound, paused], &[(4, ch)], &[]);
        ch
    });
    put(e, out, ch.unwrap_or(0));
    Ret::eax(OK)
}

fn sound_release(e: &mut Exec, args: &Args) -> Ret {
    let sound = a(args, 0);
    with(e, |s| {
        if s.movie.as_ref().is_some_and(|m| m.sound == sound)
            && let Some(m) = s.movie.take()
        {
            crate::log::note(&format!(
                "fmod: movie stream released: {} frames read, {} played",
                m.fetched, m.mixed
            ));
        }
    });
    Ret::eax(OK)
}

fn sound_get_sub_sound(e: &mut Exec, args: &Args) -> Ret {
    let (sound, index) = (a(args, 0), a(args, 1));
    let h = with(e, |s| {
        let h = s.handle();
        s.subsounds.insert(h, (sound, index));
        s.call("Sound::getSubSound", &[sound, index], &[(2, h)], &[]);
        h
    });
    put(e, a(args, 2), h.unwrap_or(0));
    Ret::eax(OK)
}

fn sound_get_name(e: &mut Exec, args: &Args) -> Ret {
    let (sound, buf, len) = (a(args, 0), a(args, 1), a(args, 2).cast_signed());
    let name = with(e, |s| {
        let (parent, i) = s.subsounds.get(&sound).copied()?;
        let bank = s.sounds.get(&parent)?.clone();
        s.engine.sample_names(&bank).get(i as usize).cloned()
    })
    .flatten()
    .unwrap_or_default();
    if buf != 0 && len > 0 {
        let n = name.len().min(usize::try_from(len).unwrap_or(1) - 1);
        let mut mem = e.mem();
        mem.write_bytes(buf, &name.as_bytes()[..n]);
        mem.write(buf + u32::try_from(n).unwrap_or(0), 0u8);
    }
    Ret::eax(OK)
}

fn sound_get_num_sub_sounds(e: &mut Exec, args: &Args) -> Ret {
    let sound = a(args, 0);
    let n = with(e, |s| {
        let bank = s.sounds.get(&sound)?.clone();
        Some(s.engine.sample_names(&bank).len())
    })
    .flatten()
    .unwrap_or(0);
    put(e, a(args, 1), u32::try_from(n).unwrap_or(0));
    Ret::eax(OK)
}

fn sound_get_open_state(e: &mut Exec, args: &Args) -> Ret {
    put(e, a(args, 1), 0u32); // FMOD_OPENSTATE_READY
    put(e, a(args, 2), 100u32);
    put(e, a(args, 3), 0u8);
    put(e, a(args, 4), 0u8);
    Ret::eax(OK)
}

/// `Sound::setUserData` / `getUserData`: one word per sound.
fn sound_set_user_data(e: &mut Exec, args: &Args) -> Ret {
    let (sound, data) = (a(args, 0), a(args, 1));
    with(e, |s| {
        s.callbacks.insert(sound | 0x8000_0000, (data, 0));
    });
    Ret::eax(OK)
}

fn sound_get_user_data(e: &mut Exec, args: &Args) -> Ret {
    let sound = a(args, 0);
    let d = with(e, |s| s.callbacks.get(&(sound | 0x8000_0000)).map(|x| x.0))
        .flatten()
        .unwrap_or(0);
    put(e, a(args, 1), d);
    Ret::eax(OK)
}

fn channel_set_volume(e: &mut Exec, args: &Args) -> Ret {
    let (ch, v) = (a(args, 0), a(args, 1));
    with(e, |s| {
        if is_movie_channel(s, ch) {
            if let Some(m) = s.movie.as_mut() {
                m.volume = f32::from_bits(v);
            }
        } else {
            s.call("Channel::setVolume", &[ch, v], &[], &[]);
        }
    });
    Ret::eax(OK)
}

fn channel_get_volume(e: &mut Exec, args: &Args) -> Ret {
    put(e, a(args, 1), 1.0f32);
    Ret::eax(OK)
}

fn channel_set_pan(e: &mut Exec, args: &Args) -> Ret {
    let (ch, p) = (a(args, 0), a(args, 1));
    with(e, |s| s.call("Channel::setPan", &[ch, p], &[], &[]));
    Ret::eax(OK)
}

fn channel_get_pan(e: &mut Exec, args: &Args) -> Ret {
    put(e, a(args, 1), 0.0f32);
    Ret::eax(OK)
}

fn channel_get_loop_count(e: &mut Exec, args: &Args) -> Ret {
    let ch = a(args, 0);
    let looping = with(e, |s| s.engine.looping(ch & !0xff | 1)).unwrap_or(false);
    put(e, a(args, 1), if looping { u32::MAX } else { 0 });
    Ret::eax(OK)
}

fn channel_set_paused(e: &mut Exec, args: &Args) -> Ret {
    let (ch, p) = (a(args, 0), a(args, 1) & 0xff);
    with(e, |s| {
        if is_movie_channel(s, ch) {
            if let Some(m) = s.movie.as_mut() {
                m.paused = p != 0;
            }
        } else {
            s.call("Channel::setPaused", &[ch, p], &[], &[]);
        }
    });
    Ret::eax(OK)
}

fn channel_group_get_num_channels(e: &mut Exec, args: &Args) -> Ret {
    put(e, a(args, 1), 1u32);
    Ret::eax(OK)
}

fn channel_group_get_channel(e: &mut Exec, args: &Args) -> Ret {
    let (g, index) = (a(args, 0), a(args, 1));
    let ch = (g & !0xff) | 4;
    with(e, |s| {
        s.call("ChannelGroup::getChannel", &[g, index], &[(2, ch)], &[]);
    });
    put(e, a(args, 2), ch);
    Ret::eax(OK)
}

const EV: &str = "fmod_event.dll";
const EX: &str = "fmodex.dll";

/// The 40 imports: DLL, decorated name, convention (`QAG`: stdcall with
/// `this` on the stack), stack words, Rust side.
pub const TABLE: &[(&str, &str, Conv, usize, ImportFn)] = &[
    (
        EV,
        "?getChannelGroup@Event@FMOD@@QAG?AW4FMOD_RESULT@@PAPAVChannelGroup@2@@Z",
        Conv::Stdcall,
        2,
        event_get_channel_group,
    ),
    (
        EV,
        "?getInfo@Event@FMOD@@QAG?AW4FMOD_RESULT@@PAHPAPADPAUFMOD_EVENT_INFO@@@Z",
        Conv::Stdcall,
        4,
        event_get_info,
    ),
    (
        EV,
        "?getPaused@Event@FMOD@@QAG?AW4FMOD_RESULT@@PA_N@Z",
        Conv::Stdcall,
        2,
        event_get_paused,
    ),
    (
        EV,
        "?setPaused@Event@FMOD@@QAG?AW4FMOD_RESULT@@_N@Z",
        Conv::Stdcall,
        2,
        event_set_paused,
    ),
    (
        EV,
        "?getVolume@Event@FMOD@@QAG?AW4FMOD_RESULT@@PAM@Z",
        Conv::Stdcall,
        2,
        event_get_volume,
    ),
    (
        EV,
        "?setVolume@Event@FMOD@@QAG?AW4FMOD_RESULT@@M@Z",
        Conv::Stdcall,
        2,
        event_set_volume,
    ),
    (
        EV,
        "?setCallback@Event@FMOD@@QAG?AW4FMOD_RESULT@@P6G?AW43@PAUFMOD_EVENT@@W4FMOD_EVENT_CALLBACKTYPE@@PAX22@Z2@Z",
        Conv::Stdcall,
        3,
        event_set_callback,
    ),
    (
        EV,
        "?getState@Event@FMOD@@QAG?AW4FMOD_RESULT@@PAI@Z",
        Conv::Stdcall,
        2,
        event_get_state,
    ),
    (
        EV,
        "?stop@Event@FMOD@@QAG?AW4FMOD_RESULT@@_N@Z",
        Conv::Stdcall,
        2,
        event_stop,
    ),
    (
        EV,
        "?start@Event@FMOD@@QAG?AW4FMOD_RESULT@@XZ",
        Conv::Stdcall,
        1,
        event_start,
    ),
    (
        EV,
        "?load@EventSystem@FMOD@@QAG?AW4FMOD_RESULT@@PBDPAUFMOD_EVENT_LOADINFO@@PAPAVEventProject@2@@Z",
        Conv::Stdcall,
        4,
        event_system_load,
    ),
    (
        EV,
        "?unload@EventSystem@FMOD@@QAG?AW4FMOD_RESULT@@XZ",
        Conv::Stdcall,
        1,
        ok,
    ),
    (
        EV,
        "?getSystemObject@EventSystem@FMOD@@QAG?AW4FMOD_RESULT@@PAPAVSystem@2@@Z",
        Conv::Stdcall,
        2,
        event_system_get_system,
    ),
    (
        EV,
        "?update@EventSystem@FMOD@@QAG?AW4FMOD_RESULT@@XZ",
        Conv::Stdcall,
        1,
        event_system_update,
    ),
    (
        EV,
        "?release@EventSystem@FMOD@@QAG?AW4FMOD_RESULT@@XZ",
        Conv::Stdcall,
        1,
        ok,
    ),
    (
        EV,
        "?init@EventSystem@FMOD@@QAG?AW4FMOD_RESULT@@HIPAXI@Z",
        Conv::Stdcall,
        5,
        ok,
    ),
    (
        EV,
        "FMOD_EventSystem_Create",
        Conv::Stdcall,
        1,
        event_system_create,
    ),
    (
        EX,
        "?setPriority@Channel@FMOD@@QAG?AW4FMOD_RESULT@@H@Z",
        Conv::Stdcall,
        2,
        ok,
    ),
    (
        EX,
        "?setPaused@Channel@FMOD@@QAG?AW4FMOD_RESULT@@_N@Z",
        Conv::Stdcall,
        2,
        channel_set_paused,
    ),
    (
        EX,
        "?stop@Channel@FMOD@@QAG?AW4FMOD_RESULT@@XZ",
        Conv::Stdcall,
        1,
        ok,
    ),
    (
        EX,
        "?getUserData@Sound@FMOD@@QAG?AW4FMOD_RESULT@@PAPAX@Z",
        Conv::Stdcall,
        2,
        sound_get_user_data,
    ),
    (
        EX,
        "?setUserData@Sound@FMOD@@QAG?AW4FMOD_RESULT@@PAX@Z",
        Conv::Stdcall,
        2,
        sound_set_user_data,
    ),
    (
        EX,
        "?getOpenState@Sound@FMOD@@QAG?AW4FMOD_RESULT@@PAW4FMOD_OPENSTATE@@PAIPA_N2@Z",
        Conv::Stdcall,
        5,
        sound_get_open_state,
    ),
    (
        EX,
        "FMOD_Memory_GetStats",
        Conv::Stdcall,
        3,
        memory_get_stats,
    ),
    (
        EX,
        "?setFileSystem@System@FMOD@@QAG?AW4FMOD_RESULT@@P6G?AW43@PBDHPAIPAPAX2@ZP6G?AW43@PAX4@ZP6G?AW43@44I14@ZP6G?AW43@4I4@ZP6G?AW43@PAUFMOD_ASYNCREADINFO@@4@Z5H@Z",
        Conv::Stdcall,
        8,
        system_set_file_system,
    ),
    (
        EX,
        "?setStreamBufferSize@System@FMOD@@QAG?AW4FMOD_RESULT@@II@Z",
        Conv::Stdcall,
        3,
        ok,
    ),
    (
        EX,
        "?createSound@System@FMOD@@QAG?AW4FMOD_RESULT@@PBDIPAUFMOD_CREATESOUNDEXINFO@@PAPAVSound@2@@Z",
        Conv::Stdcall,
        5,
        system_create_sound,
    ),
    (
        EX,
        "?playSound@System@FMOD@@QAG?AW4FMOD_RESULT@@W4FMOD_CHANNELINDEX@@PAVSound@2@_NPAPAVChannel@2@@Z",
        Conv::Stdcall,
        5,
        system_play_sound,
    ),
    (
        EX,
        "?release@Sound@FMOD@@QAG?AW4FMOD_RESULT@@XZ",
        Conv::Stdcall,
        1,
        sound_release,
    ),
    (
        EX,
        "?getSubSound@Sound@FMOD@@QAG?AW4FMOD_RESULT@@HPAPAV12@@Z",
        Conv::Stdcall,
        3,
        sound_get_sub_sound,
    ),
    (
        EX,
        "?getName@Sound@FMOD@@QAG?AW4FMOD_RESULT@@PADH@Z",
        Conv::Stdcall,
        3,
        sound_get_name,
    ),
    (
        EX,
        "?getNumSubSounds@Sound@FMOD@@QAG?AW4FMOD_RESULT@@PAH@Z",
        Conv::Stdcall,
        2,
        sound_get_num_sub_sounds,
    ),
    (
        EX,
        "?setVolume@Channel@FMOD@@QAG?AW4FMOD_RESULT@@M@Z",
        Conv::Stdcall,
        2,
        channel_set_volume,
    ),
    (
        EX,
        "?getVolume@Channel@FMOD@@QAG?AW4FMOD_RESULT@@PAM@Z",
        Conv::Stdcall,
        2,
        channel_get_volume,
    ),
    (
        EX,
        "?setPan@Channel@FMOD@@QAG?AW4FMOD_RESULT@@M@Z",
        Conv::Stdcall,
        2,
        channel_set_pan,
    ),
    (
        EX,
        "?getPan@Channel@FMOD@@QAG?AW4FMOD_RESULT@@PAM@Z",
        Conv::Stdcall,
        2,
        channel_get_pan,
    ),
    (EX, "FMOD_Memory_Initialize", Conv::Stdcall, 6, ok),
    (
        EX,
        "?getLoopCount@Channel@FMOD@@QAG?AW4FMOD_RESULT@@PAH@Z",
        Conv::Stdcall,
        2,
        channel_get_loop_count,
    ),
    (
        EX,
        "?getChannel@ChannelGroup@FMOD@@QAG?AW4FMOD_RESULT@@HPAPAVChannel@2@@Z",
        Conv::Stdcall,
        3,
        channel_group_get_channel,
    ),
    (
        EX,
        "?getNumChannels@ChannelGroup@FMOD@@QAG?AW4FMOD_RESULT@@PAH@Z",
        Conv::Stdcall,
        2,
        channel_group_get_num_channels,
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    /// The lines of a reference capture's marks file (battle-attack, the
    /// hook's `FFX_AUDIO_WAV` on the original code), field for field.
    #[test]
    fn marks_lines_are_the_hooks() {
        assert_eq!(
            marks_header("C:\\t\\a.wav"),
            "# audiomarks 1 C:\\t\\a.wav\n\
             format rate 48000 format 5 channels 2 bits 32 block 1600 output 5 init 0x0\n"
        );
        assert_eq!(
            mark_line("start", 192, 189, 0, true),
            "mark start frame 192 sample 302400 updates 189 free 0 late 0 turns true"
        );
        assert_eq!(
            mark_reply(1315, 1179, 0),
            "ok frame 1315 sample 1886400 updates 1179 free 0 late 0"
        );
    }

    #[test]
    fn a_capture_writes_its_marks_beside_the_wav() {
        let dir = std::env::temp_dir().join(format!("ffx-host-marks-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let wav = dir.join("x.wav");
        let a = Audio::default();
        a.capture(&wav).unwrap();
        assert_eq!(a.wav.lock().unwrap().as_deref(), Some(wav.as_path()));
        a.blocks.store(3, Ordering::Release);
        assert_eq!(
            a.mark("start", 5, true, 0),
            "ok frame 5 sample 4800 updates 3 free 0 late 0"
        );
        a.note_late(6);
        let text = std::fs::read_to_string(dir.join("x.wav.marks")).unwrap();
        let lines: Vec<_> = text.lines().collect();
        assert_eq!(lines.len(), 4);
        assert!(lines[0].starts_with("# audiomarks 1 "));
        assert!(lines[1].starts_with("format rate 48000 "));
        assert_eq!(
            lines[2],
            "mark start frame 5 sample 4800 updates 3 free 0 late 0 turns true"
        );
        assert_eq!(lines[3], "late frame 6");
        // the WAV itself: FMOD's writer header, 0x44 bytes before the data
        drop(wav_file(&wav).unwrap());
        let h = std::fs::read(&wav).unwrap();
        assert_eq!(h.len(), 0x44);
        assert_eq!(&h[0x3c..0x40], b"data");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn without_a_capture_a_mark_still_answers() {
        let a = Audio::default();
        assert_eq!(
            a.mark("end", 9, false, 0),
            "ok frame 9 sample 0 updates 0 free 0 late 0"
        );
    }
}
