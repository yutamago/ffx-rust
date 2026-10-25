//! The glue outside rendering (`ffx_glue_other`, M4 port round 1, list
//! `h4-glue-other`) against the original: on the image alone and on a
//! snapshot (`FFX_SNAPSHOT`). Results and every byte written must agree.
//! Needs `FFX_GAME_DIR`.
//!
//! Callees that stay original are replaced in both oracles alike by
//! recording stubs: each call appends its tag, `ecx` (for the methods: a
//! cdecl callee's `ecx` is whatever the caller left) and its stack words
//! to a log in memory and returns the word in its own result cell, so the
//! diff checks that both made the same calls, with the same `this` and
//! arguments, in the same order. Function pointers the originals call
//! point at such stubs too.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::many_single_char_names,
    clippy::too_many_lines
)]

use std::collections::HashMap;
use std::path::PathBuf;

use ffx_bridge::Mem;
use ffx_glue_other::{effect as e, menu as mn, misc as m, movie as mv, sound as s};
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};

/// `graphicFontDraw`'s body: no test here calls it, so the recording
/// stubs are written over it.
const CAVE: u32 = 0x0064_0660;
const CAVE_LEN: u32 = 1768;

/// Deterministic xorshift stream.
struct Rng(u32);

impl Rng {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }
    fn below(&mut self, n: u32) -> u32 {
        self.next() % n
    }
    /// A word: small, signed edge values or random.
    fn word(&mut self) -> u32 {
        match self.below(4) {
            0 => self.below(8),
            1 => [0, 1, u32::MAX, 0x8000_0000, 0x7fff_ffff, 0xb5, 0xb6, 0x100]
                [self.below(8) as usize],
            _ => self.next(),
        }
    }
    /// Float bits that are not a signalling NaN (the emulator keeps
    /// those; the hardware quiets them).
    fn float(&mut self) -> u32 {
        loop {
            let bits = match self.below(3) {
                0 => self.next(),
                1 => (self.below(2000) as f32 / 7.0 - 100.0).to_bits(),
                _ => [
                    0,
                    0x8000_0000,
                    0x7f80_0000,
                    0xff80_0000,
                    0x7fc0_0000,
                    0xffc0_1234,
                    1,
                    0x7f7f_ffff,
                ][self.below(8) as usize],
            };
            let nan = bits & 0x7f80_0000 == 0x7f80_0000 && bits & 0x007f_ffff != 0;
            if !(nan && bits & 0x0040_0000 == 0) {
                return bits;
            }
        }
    }
}

/// The bridge's view of an oracle: its memory, and its emulator for the
/// original callees.
struct G<'a>(&'a mut Oracle);

impl Mem for G<'_> {
    fn read_bytes(&self, addr: u32, out: &mut [u8]) {
        self.0.read_bytes(addr, out);
    }
    fn write_bytes(&mut self, addr: u32, data: &[u8]) {
        Mem::write_bytes(self.0, addr, data);
    }
    fn image_base(&self) -> u32 {
        self.0.image_base()
    }
}

impl Host for G<'_> {
    fn call(&mut self, addr: u32, conv: host::Conv, args: &[u32]) -> u32 {
        ffx_oracle::check_width("Host::call", addr, args.len(), host::max_words(conv, false));
        let conv = match conv {
            host::Conv::Cdecl => Conv::Cdecl,
            host::Conv::Stdcall => Conv::Stdcall,
            host::Conv::Thiscall(t) => Conv::Thiscall(t),
            host::Conv::Fastcall => Conv::Fastcall,
        };
        self.0
            .call(addr, conv, args)
            .unwrap_or_else(|e| panic!("callee {addr:#x}: {e}"))
            .eax
    }
    fn scratch(&mut self) -> u32 {
        self.0.stack_range().start + 0x100
    }
}

fn game_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("FFX_GAME_DIR").map(PathBuf::from);
    if dir.is_none() {
        eprintln!("FFX_GAME_DIR is not set; skipping");
    }
    dir
}

/// What the original returns, to compare.
#[derive(Debug, Clone, Copy)]
enum Out {
    /// Nothing the callers read.
    None,
    /// `eax`.
    U32(u32),
}

struct Twins {
    a: Oracle,
    b: Oracle,
    /// The call log: a count, then entries of 16 words (tag, `ecx`, up to 14
    /// stack words).
    log: u32,
    cave: u32,
    cave_next: u32,
    /// The result cell of each stub, by tag.
    cells: HashMap<u32, u32>,
}

impl Twins {
    fn new(mut a: Oracle, mut b: Oracle) -> Self {
        a.set_fpu_control(0x027f).unwrap();
        b.set_fpu_control(0x027f).unwrap();
        let cave = a.rebase(CAVE);
        let mut t = Self {
            a,
            b,
            log: 0,
            cave,
            cave_next: cave,
            cells: HashMap::new(),
        };
        t.log = t.zeros(0x4_0000);
        t
    }

    /// The image, then the snapshot when `FFX_SNAPSHOT` is set.
    fn all() -> Vec<(&'static str, Self)> {
        let Some(dir) = game_dir() else {
            return vec![];
        };
        let exe = dir.join("FFX.exe");
        let mut out = vec![(
            "image",
            Self::new(Oracle::load(&exe).unwrap(), Oracle::load(&exe).unwrap()),
        )];
        if let Some(snap) = std::env::var_os("FFX_SNAPSHOT") {
            let pages = Pages::read(PathBuf::from(snap)).unwrap();
            out.push((
                "snapshot",
                Self::new(
                    Oracle::from_snapshot(&exe, &pages).unwrap(),
                    Oracle::from_snapshot(&exe, &pages).unwrap(),
                ),
            ));
        } else {
            eprintln!("FFX_SNAPSHOT is not set; image only");
        }
        out
    }

    fn rebase(&self, sym: u32) -> u32 {
        self.a.rebase(sym)
    }

    fn alloc(&mut self, bytes: &[u8]) -> u32 {
        let x = self.a.alloc_bytes(bytes).unwrap();
        assert_eq!(self.b.alloc_bytes(bytes).unwrap(), x);
        x
    }

    fn zeros(&mut self, n: usize) -> u32 {
        self.alloc(&vec![0; n])
    }

    /// `n` random bytes.
    fn random(&mut self, rng: &mut Rng, n: usize) -> u32 {
        let v: Vec<u8> = (0..n).map(|_| rng.next() as u8).collect();
        self.alloc(&v)
    }

    fn write(&mut self, at: u32, bytes: &[u8]) {
        self.a.write(at, bytes).unwrap();
        self.b.write(at, bytes).unwrap();
    }

    fn w32(&mut self, at: u32, v: u32) {
        self.write(at, &v.to_le_bytes());
    }

    fn w8(&mut self, at: u32, v: u8) {
        self.write(at, &[v]);
    }

    /// Writes global `sym` (an `ffx.sym` address).
    fn g32(&mut self, sym: u32, v: u32) {
        let at = self.rebase(sym);
        self.w32(at, v);
    }

    /// A recording stub at the original `sym` (a `jmp` there): logs `sym`,
    /// `ecx` when `this`, and `n` stack words; pops `pops` words; returns
    /// its cell ([`Twins::ret`]).
    fn stub(&mut self, sym: u32, this: bool, n: u32, pops: u32) {
        let at = self.rebase(sym);
        let addr = self.stub_code(sym, this, n, pops);
        let mut j = vec![0xe9];
        j.extend(addr.wrapping_sub(at + 5).to_le_bytes());
        self.write(at, &j);
    }

    /// The stub of [`Twins::stub`] in the cave only (for function
    /// pointers), `tag` its log tag. Returns its address.
    fn stub_code(&mut self, tag: u32, this: bool, n: u32, pops: u32) -> u32 {
        assert!(n <= 14);
        let cell = self.zeros(4);
        self.cells.insert(tag, cell);
        let log = self.log;
        let mut c = vec![0xa1];
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.extend([0xc1, 0xe0, 0x06]); // shl eax, 6
        c.push(0x05);
        c.extend((log + 4).to_le_bytes()); // add eax, log + 4
        c.extend([0xc7, 0x00]);
        c.extend(tag.to_le_bytes()); // mov [eax], tag
        if this {
            c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
        }
        for i in 0..n {
            c.extend([0x8b, 0x54, 0x24, (4 + 4 * i) as u8]); // mov edx, [esp+4+4i]
            c.extend([0x89, 0x50, (8 + 4 * i) as u8]); // mov [eax+8+4i], edx
        }
        c.extend([0xff, 0x05]);
        c.extend(log.to_le_bytes()); // inc dword [log]
        c.push(0xa1);
        c.extend(cell.to_le_bytes()); // mov eax, [cell]
        if pops == 0 {
            c.push(0xc3);
        } else {
            c.push(0xc2);
            c.extend(((pops * 4) as u16).to_le_bytes());
        }
        self.code(&c)
    }

    /// Machine code in the cave; its address.
    fn code(&mut self, c: &[u8]) -> u32 {
        let addr = self.cave_next;
        assert!(addr + c.len() as u32 <= self.cave + CAVE_LEN, "cave full");
        self.write(addr, c);
        self.cave_next = addr + c.len() as u32;
        addr
    }

    /// Sets the result of the stub tagged `tag`.
    fn ret(&mut self, tag: u32, v: u32) {
        let cell = self.cells[&tag];
        self.w32(cell, v);
    }

    fn log_entries(&self) -> u32 {
        Mem::read(&self.b, self.log)
    }

    fn diff(&mut self, what: &str) {
        let d = self.a.diff(&mut self.b).unwrap();
        assert!(
            d.is_empty(),
            "{what}: memory differs at {} bytes, first {:x?}",
            d.len(),
            &d[..d.len().min(8)]
        );
        self.a.clear_dirty();
        self.b.clear_dirty();
    }

    /// Runs the original at `sym` in `a` and `port` on `b`; compares the
    /// result as `port` says and every byte written. Returns the
    /// original's `eax`.
    fn check(
        &mut self,
        what: &str,
        sym: u32,
        conv: Conv,
        args: &[u32],
        port: impl FnOnce(&mut G) -> Out,
    ) -> u32 {
        let addr = self.rebase(sym);
        let ra = self
            .a
            .call(addr, conv, args)
            .unwrap_or_else(|e| panic!("{what}: the original: {e}"));
        let out = port(&mut G(&mut self.b));
        assert!(
            ra.st0_raw.is_none(),
            "{what}: the original leaves ST0 set ({:?})",
            ra.st0
        );
        match out {
            Out::None => {}
            Out::U32(v) => assert_eq!(ra.eax, v, "{what}: eax"),
        }
        self.diff(what);
        ra.eax
    }
}

fn u(v: u32) -> Out {
    Out::U32(v)
}

const PUSH_COMMAND: u32 = ffx_sound::fmod::PUSH_COMMAND;
const REGIST_COMMAND_SYNC: u32 = ffx_sound::fmod::REGIST_COMMAND_SYNC;
const FMOD_MANAGER: u32 = ffx_sound::fmod::FMOD_MANAGER;

/// The `i*` sound entry points: queue and sync commands, `iSync*`,
/// `iVoiceStart`'s signal.
#[test]
fn sound_entry_points() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_0001);
        t.stub(PUSH_COMMAND, true, 5, 5);
        t.stub(REGIST_COMMAND_SYNC, true, 5, 5);
        t.stub(s::SIGNAL_VOICE_START, true, 0, 0);
        t.stub(s::SYNC_INIT, true, 0, 0);
        t.stub(s::SYNC_DATA_CHANGE, true, 1, 1);
        t.stub(s::SYNC_GET_DATA, true, 3, 3);
        for i in 0..24 {
            let (x, y, z) = (rng.word(), rng.word(), rng.word());
            t.g32(FMOD_MANAGER, rng.next());
            t.g32(0x00ce_9014, rng.next());
            for tag in [
                PUSH_COMMAND,
                REGIST_COMMAND_SYNC,
                s::SIGNAL_VOICE_START,
                s::SYNC_INIT,
                s::SYNC_DATA_CHANGE,
                s::SYNC_GET_DATA,
            ] {
                let v = rng.next();
                t.ret(tag, v);
            }
            let w = format!("{name} {i}");
            let c = Conv::Cdecl;
            t.check(&format!("{w} iSoundInit"), 0x0067_a500, c, &[], |g| {
                u(s::i_sound_init(g))
            });
            t.check(
                &format!("{w} iSoundStageFlush"),
                0x0067_a5d0,
                c,
                &[x],
                |g| u(s::i_sound_stage_flush(g, x)),
            );
            t.check(
                &format!("{w} iSfxUnloadAllData"),
                0x0067_a4d0,
                c,
                &[],
                |g| u(s::i_sfx_unload_all_data(g)),
            );
            t.check(&format!("{w} iSfxStop"), 0x0067_a460, c, &[x], |g| {
                u(s::i_sfx_stop(g, x))
            });
            t.check(
                &format!("{w} iSfxStopNonBlockingSound"),
                0x0067_a4a0,
                c,
                &[x],
                |g| u(s::i_sfx_stop_non_blocking_sound(g, x)),
            );
            t.check(
                &format!("{w} iSfxStopAllNonBlockingSound"),
                0x0067_a480,
                c,
                &[x],
                |g| u(s::i_sfx_stop_all_non_blocking_sound(g, x)),
            );
            t.check(
                &format!("{w} iSfxSetVolume"),
                0x0067_a400,
                c,
                &[x, y],
                |g| u(s::i_sfx_set_volume(g, x, y)),
            );
            t.check(
                &format!("{w} iSfxSetVolumeTranslate"),
                0x0067_a420,
                c,
                &[x, y, z],
                |g| u(s::i_sfx_set_volume_translate(g, x, y, z)),
            );
            t.check(&format!("{w} iSfxStart"), 0x0067_a440, c, &[x, y, z], |g| {
                u(s::i_sfx_start(g, x, y, z))
            });
            t.check(&format!("{w} iVoiceStop"), 0x0067_a7a0, c, &[], |g| {
                u(s::i_voice_stop(g))
            });
            t.check(&format!("{w} iVoiceRead"), 0x0067_a720, c, &[x], |g| {
                u(s::i_voice_read(g, x))
            });
            t.check(&format!("{w} iVoiceSetPaused"), 0x0067_a750, c, &[x], |g| {
                u(s::i_voice_set_paused(g, x))
            });
            t.check(
                &format!("{w} iVoiceDataChange"),
                0x0067_a6a0,
                c,
                &[x],
                |g| u(s::i_voice_data_change(g, x)),
            );
            t.check(&format!("{w} iVoiceStart"), 0x0067_a770, c, &[x, y], |g| {
                u(s::i_voice_start(g, x, y))
            });
            t.check(&format!("{w} iVoicePlayStatus"), 0x0067_a710, c, &[], |g| {
                u(s::i_voice_play_status(g))
            });
            t.check(&format!("{w} iVoiceIsPaused"), 0x0067_a6c0, c, &[], |g| {
                u(s::i_voice_is_paused(g))
            });
            t.check(&format!("{w} iSyncInit"), 0x0067_a950, c, &[], |g| {
                u(s::i_sync_init(g))
            });
            t.check(&format!("{w} iSyncDataChange"), 0x0067_a8e0, c, &[x], |g| {
                u(s::i_sync_data_change(g, x))
            });
            t.check(
                &format!("{w} iSyncGetData"),
                0x0067_a930,
                c,
                &[x, y, z],
                |g| u(s::i_sync_get_data(g, x, y, z)),
            );
        }
        assert!(t.log_entries() >= 24 * 20, "{name}: the stubs ran");
    }
}

/// `iVoiceStart` reads `g_FmodManager` again after the signal: a signal
/// stub that changes it.
#[test]
fn voice_start_rereads_the_manager() {
    for (name, mut t) in Twins::all() {
        t.stub(PUSH_COMMAND, true, 5, 5);
        let cell = t.rebase(FMOD_MANAGER);
        // mov dword [g_FmodManager], 0x12345678; ret
        let mut c = vec![0xc7, 0x05];
        c.extend(cell.to_le_bytes());
        c.extend(0x1234_5678u32.to_le_bytes());
        c.push(0xc3);
        let f = t.code(&c);
        let at = t.rebase(s::SIGNAL_VOICE_START);
        let mut j = vec![0xe9];
        j.extend(f.wrapping_sub(at + 5).to_le_bytes());
        t.write(at, &j);
        t.g32(FMOD_MANAGER, 0x0bad_0000);
        t.check(
            &format!("{name} iVoiceStart"),
            0x0067_a770,
            Conv::Cdecl,
            &[3, 4],
            |g| u(s::i_voice_start(g, 3, 4)),
        );
    }
}

impl Twins {
    /// A recording stub at `sym` that logs `[ecx]` (the object's first
    /// word) where [`Twins::stub`] logs `ecx`, then `n` stack words; pops
    /// `pops`; returns its cell.
    fn stub_deref(&mut self, sym: u32, n: u32, pops: u32) {
        let cell = self.zeros(4);
        self.cells.insert(sym, cell);
        let log = self.log;
        let mut c = vec![0xa1];
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.extend([0xc1, 0xe0, 0x06]); // shl eax, 6
        c.push(0x05);
        c.extend((log + 4).to_le_bytes()); // add eax, log + 4
        c.extend([0xc7, 0x00]);
        c.extend(sym.to_le_bytes()); // mov [eax], sym
        c.extend([0x8b, 0x11]); // mov edx, [ecx]
        c.extend([0x89, 0x50, 0x04]); // mov [eax+4], edx
        for i in 0..n {
            c.extend([0x8b, 0x54, 0x24, (4 + 4 * i) as u8]); // mov edx, [esp+4+4i]
            c.extend([0x89, 0x50, (8 + 4 * i) as u8]); // mov [eax+8+4i], edx
        }
        c.extend([0xff, 0x05]);
        c.extend(log.to_le_bytes()); // inc dword [log]
        c.push(0xa1);
        c.extend(cell.to_le_bytes()); // mov eax, [cell]
        c.push(0xc2);
        c.extend(((pops * 4) as u16).to_le_bytes());
        let f = self.code(&c);
        let at = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(f.wrapping_sub(at + 5).to_le_bytes());
        self.write(at, &j);
    }

    /// A stand-in at `sym` that stores `value` at global `global`, logs
    /// its call as [`Twins::stub`] would (tag `sym`, `ecx` when `this`, no
    /// words) and
    /// returns `ret`.
    fn stub_store(&mut self, sym: u32, this: bool, global: u32, value: u32, ret: u32) {
        let cell = self.rebase(global);
        let log = self.log;
        let mut c = vec![0xc7, 0x05];
        c.extend(cell.to_le_bytes());
        c.extend(value.to_le_bytes()); // mov dword [global], value
        c.push(0xa1);
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.extend([0xc1, 0xe0, 0x06]); // shl eax, 6
        c.push(0x05);
        c.extend((log + 4).to_le_bytes()); // add eax, log + 4
        c.extend([0xc7, 0x00]);
        c.extend(sym.to_le_bytes()); // mov [eax], sym
        if this {
            c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
        }
        c.extend([0xff, 0x05]);
        c.extend(log.to_le_bytes()); // inc dword [log]
        c.push(0xb8);
        c.extend(ret.to_le_bytes()); // mov eax, ret
        c.push(0xc3);
        let f = self.code(&c);
        let at = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(f.wrapping_sub(at + 5).to_le_bytes());
        self.write(at, &j);
    }
}

/// Globals read again after a callee: `GetInstance` after its message
/// (a print that creates the instance), `graphicResetSplitScreen` after
/// the state.
#[test]
fn globals_read_again() {
    for (name, mut t) in Twins::all() {
        t.stub_store(
            e::PHYRE_PRINTF,
            false,
            e::BLUR_EFFECT.global,
            0x1357_9bdf,
            7,
        );
        t.g32(e::BLUR_EFFECT.global, 0);
        t.check(
            &format!("{name} GetInstance"),
            e::BLUR_EFFECT.get,
            Conv::Cdecl,
            &[],
            |g| u(e::get_instance(g, e::BLUR_EFFECT)),
        );
        t.stub_store(m::GET_SPLIT_SCREEN, true, 0x00cc_b9d8, 0x0246_8ace, 1);
        t.stub(m::SET_SPLIT_SCREEN, true, 1, 1);
        t.ret(m::SET_SPLIT_SCREEN, 0x55);
        t.g32(0x00cc_b9d8, 0x1111_1111);
        t.check(
            &format!("{name} resetSplitScreen"),
            0x0063_8d20,
            Conv::Cdecl,
            &[],
            |g| u(m::reset_split_screen(g)),
        );
    }
}

/// `FmodManager_signalVoiceStart`, `SyncManager_SyncManager`,
/// `FmodMusic_getState`, `FmodMemory_free`, `PVideoPlaybackWin32_setVolume`,
/// `iMusicRefreshBgmType`.
#[test]
fn sound_methods_and_callbacks() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_0002);
        t.stub(s::VOICE_SET_START_SIGNAL, true, 1, 1);
        t.stub(s::SYNC_INIT_DATA, true, 0, 0);
        t.stub(s::USER_FREE, false, 1, 0);
        t.stub(s::CHANNEL_SET_VOLUME, false, 2, 2);
        t.stub(s::SAVE_EVENT_ADDRESS, false, 0, 0);
        t.stub(s::SET_BGM_TYPE, false, 1, 0);
        let save = t.random(&mut rng, 0x600);
        for i in 0..40 {
            let w = format!("{name} {i}");
            for tag in [
                s::VOICE_SET_START_SIGNAL,
                s::SYNC_INIT_DATA,
                s::USER_FREE,
                s::SET_BGM_TYPE,
            ] {
                let v = rng.next();
                t.ret(tag, v);
            }
            let set_volume = [0, 1, rng.next()][rng.below(3) as usize];
            t.ret(s::CHANNEL_SET_VOLUME, set_volume);
            t.ret(s::SAVE_EVENT_ADDRESS, save);
            let b = rng.next() as u8;
            t.w8(save + s::SAVE_BGM_TYPE, b);
            let obj = t.random(&mut rng, 0x400);
            t.check(
                &format!("{w} signalVoiceStart"),
                s::SIGNAL_VOICE_START,
                Conv::Thiscall(obj),
                &[],
                |g| u(s::fmod_manager_signal_voice_start(g, obj)),
            );
            t.check(
                &format!("{w} SyncManager"),
                0x006f_a8b0,
                Conv::Thiscall(obj),
                &[],
                |g| u(s::sync_manager_ctor(g, obj)),
            );
            let id = [rng.word(), rng.below(0xb6), 0xb5, 0xb6][rng.below(4) as usize];
            t.check(
                &format!("{w} getState {id:#x}"),
                0x0070_8be0,
                Conv::Cdecl,
                &[id],
                |g| u(s::fmod_music_get_state(g, id)),
            );
            // a block with its offset byte in front; null; offset 0
            let p = if rng.below(4) == 0 {
                0
            } else {
                obj + 0x100 + rng.below(0x100)
            };
            let (k, src) = (rng.next(), rng.next());
            t.check(
                &format!("{w} FmodMemory_free {p:#x}"),
                0x0070_6710,
                Conv::Stdcall,
                &[p, k, src],
                |g| {
                    s::fmod_memory_free(g, p);
                    Out::None
                },
            );
            let v = rng.float();
            t.check(
                &format!("{w} setVolume"),
                0x00a2_8330,
                Conv::Thiscall(obj),
                &[v],
                |g| u(s::video_set_volume(g, obj, v)),
            );
            t.check(
                &format!("{w} iMusicRefreshBgmType"),
                0x008c_c170,
                Conv::Cdecl,
                &[],
                |g| u(s::i_music_refresh_bgm_type(g)),
            );
        }
    }
}

/// The singletons, `resize`, the opacity, the broken-screen init, the
/// delay-release step, the shader-cluster callbacks.
#[test]
fn effects() {
    let singletons = [
        e::DISTORTION,
        e::BLUR_EFFECT,
        e::BLUR_EFFECT_EX,
        e::MOTION_BLUR,
        e::BROKEN_SCREEN,
        e::FMV_PLAYER_MANAGER,
    ];
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_0003);
        t.stub(e::PHYRE_PRINTF, false, 2, 0);
        t.stub(e::TARGET_RESIZE, false, 4, 0);
        t.stub(e::ASYNC_LOAD_MANAGER, false, 0, 0);
        t.stub(e::CLUSTER_MANAGER, false, 0, 0);
        t.stub(e::DELAY_RELEASE_STEP, true, 1, 1);
        t.stub(e::BROKEN_SCREEN_INIT, true, 2, 2);
        t.stub(e::BLUR_ON_LOADED, true, 0, 0);
        t.stub(e::FMV_ON_LOADED, true, 0, 0);
        for i in 0..30 {
            let w = format!("{name} {i}");
            for tag in [
                e::PHYRE_PRINTF,
                e::TARGET_RESIZE,
                e::ASYNC_LOAD_MANAGER,
                e::CLUSTER_MANAGER,
                e::DELAY_RELEASE_STEP,
                e::BROKEN_SCREEN_INIT,
                e::BLUR_ON_LOADED,
                e::FMV_ON_LOADED,
            ] {
                let v = rng.next();
                t.ret(tag, v);
            }
            for s in singletons {
                let v = if rng.below(2) == 0 { 0 } else { rng.next() };
                t.g32(s.global, v);
                t.check(
                    &format!("{w} GetInstance {:#x}", s.get),
                    s.get,
                    Conv::Cdecl,
                    &[],
                    |g| u(e::get_instance(g, s)),
                );
            }
            let obj = t.random(&mut rng, 0x400);
            let (x, y) = (rng.word(), rng.word());
            for (sym, at) in [(0x006d_59c0, 0), (0x006c_2fd0, 0x28), (0x0070_35f0, 0x68)] {
                t.check(
                    &format!("{w} resize {sym:#x}"),
                    sym,
                    Conv::Thiscall(obj),
                    &[x, y],
                    |g| u(e::resize(g, obj, at, x, y)),
                );
            }
            let f = rng.float();
            t.check(
                &format!("{w} setOpacity"),
                0x006d_59e0,
                Conv::Thiscall(obj),
                &[f],
                |g| {
                    e::broken_screen_set_opacity(g, obj, f);
                    Out::None
                },
            );
            t.check(
                &format!("{w} delayReleaseStep"),
                0x0064_42d0,
                Conv::Cdecl,
                &[f],
                |g| u(e::chr_delay_release_step(g, f)),
            );
            let holder = t.zeros(4);
            t.w32(holder, obj);
            t.check(
                &format!("{w} blur loaded"),
                0x006c_3950,
                Conv::Cdecl,
                &[x, holder],
                |g| {
                    u(e::on_shader_cluster_loaded(
                        g,
                        x,
                        holder,
                        e::BLUR_CLUSTER,
                        e::BLUR_ON_LOADED,
                    ))
                },
            );
            t.check(
                &format!("{w} fmv loaded"),
                0x006d_a460,
                Conv::Cdecl,
                &[y, holder],
                |g| {
                    u(e::on_shader_cluster_loaded(
                        g,
                        y,
                        holder,
                        e::FMV_CLUSTER,
                        e::FMV_ON_LOADED,
                    ))
                },
            );
        }
        // the getter is ported here too: stubbed, as a callee
        t.stub(e::BROKEN_SCREEN.get, false, 0, 0);
        for i in 0..12 {
            let inst = rng.next();
            t.ret(e::BROKEN_SCREEN.get, inst);
            let v = rng.next();
            t.ret(e::BROKEN_SCREEN_INIT, v);
            let (x, y) = (rng.word(), rng.word());
            t.check(
                &format!("{name} {i} brokenScreenInit"),
                0x0063_9450,
                Conv::Cdecl,
                &[x, y],
                |g| u(e::graphic_broken_screen_init(g, x, y)),
            );
        }
    }
}

/// The forwarders and the menu, Iggy, text and PS2 helpers.
#[test]
fn misc_helpers() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_0004);
        let stubs: &[(u32, bool, u32, u32)] = &[
            (m::UNLOAD_PRX, false, 1, 0),
            (m::LOAD_PCLUSTER, true, 1, 1),
            (m::SAVE_ADD_AP, false, 2, 0),
            (m::SET_VIEWPORT, true, 2, 2),
            (m::CLOSE_PKTBUFF, false, 0, 0),
            (m::DRAW_KICK, false, 0, 0),
            (m::DBG_PRINTF, false, 1, 0),
            (m::DMA_SYNC, false, 3, 0),
            (m::IGGY_SET_ENABLE, true, 1, 1),
            (m::INPUT_MANAGER, false, 0, 0),
            (m::VIBRATION_FLAG, true, 0, 0),
            (m::BUILD_RESOLUTION_LIST, true, 0, 0),
            (m::INIT_VIDEO_PAGE, true, 0, 0),
            (m::SET_MESH_VISIBLE, true, 2, 2),
            (m::OPERATOR_NEW, false, 1, 0),
            (m::ADD_CALLBACK, true, 3, 3),
            (m::PSTRING_ASSIGN, true, 1, 1),
            (m::TEXT_RENDER, true, 1, 1),
            (m::BASE_DIC_TYPE, false, 0, 0),
            (m::FONT_WIDTH, false, 4, 0),
            (m::GET_SPLIT_SCREEN, true, 0, 0),
            (m::SET_SPLIT_SCREEN, true, 1, 1),
            (m::IDENTITY, false, 1, 0),
            (m::CTRL_PKT, false, 0, 0),
        ];
        for &(sym, this, n, pops) in stubs {
            t.stub(sym, this, n, pops);
        }
        // function pointers: cdecl one word, thiscall one word, the import
        let cdecl1 = t.stub_code(0xc0de_0001, false, 1, 0);
        let this1 = t.stub_code(0xc0de_0002, true, 1, 1);
        let free = t.stub_code(0xc0de_0003, false, 2, 2);
        let slot = t.rebase(0x00b0_c820);
        t.w32(slot, free);
        for i in 0..40 {
            let w = format!("{name} {i}");
            for &(sym, ..) in stubs {
                let v = rng.next();
                t.ret(sym, v);
            }
            for tag in [0xc0de_0001, 0xc0de_0002, 0xc0de_0003] {
                let v = rng.next();
                t.ret(tag, v);
            }
            // the split-screen state is 1 half the time
            let stat = if rng.below(2) == 0 { 1 } else { rng.word() };
            t.ret(m::GET_SPLIT_SCREEN, stat);
            let (x, y, z) = (rng.word(), rng.word(), rng.word());
            let obj = t.random(&mut rng, 0x6000);
            let c = Conv::Cdecl;
            let th = Conv::Thiscall(obj);
            t.check(
                &format!("{w} UnloadMagicfilePrx"),
                0x0067_94f0,
                c,
                &[x],
                |g| u(m::unload_magicfile_prx(g, x)),
            );
            t.check(
                &format!("{w} loadPClusterJump"),
                0x0069_bb00,
                th,
                &[x],
                |g| u(m::load_pcluster_jump(g, obj, x)),
            );
            t.check(
                &format!("{w} SaveAddAPJump"),
                0x008b_a390,
                c,
                &[x, y],
                |g| u(m::save_add_ap_jump(g, x, y)),
            );
            t.check(
                &format!("{w} SetViewportJump"),
                0x00a2_6960,
                th,
                &[x, y],
                |g| u(m::set_viewport_jump(g, obj, x, y)),
            );
            t.w32(obj + 4, cdecl1);
            t.check(
                &format!("{w} RegistTaskRequest_run"),
                0x0069_1350,
                th,
                &[],
                |g| u(m::regist_task_request_run(g, obj)),
            );
            t.check(
                &format!("{w} TOMenuDrawKickTmp"),
                0x008b_ea20,
                c,
                &[],
                |g| u(m::menu_draw_kick_tmp(g)),
            );
            for (sym, msg) in [
                (0x0076_efa0, m::MOVIE_ERR),
                (0x0076_eff0, m::MOVIE_INIT),
                (0x0076_f080, m::MOVIE_START_PROG),
                (0x0076_f090, m::MOVIE_STOP_PROG),
            ] {
                t.check(&format!("{w} movie {sym:#x}"), sym, c, &[], |g| {
                    m::movie_message(g, msg);
                    Out::None
                });
            }
            let sink = rng.next() & !0x100;
            t.g32(0x0230_56e8, sink);
            t.check(&format!("{w} MFIFOSpadSync"), 0x0084_32d0, c, &[], |g| {
                u(m::mfifo_spad_sync(g))
            });
            t.g32(0x023c_bbf4, rng.next());
            t.check(&format!("{w} GsSync"), 0x0084_1d60, c, &[], |g| {
                u(m::gs_sync(g))
            });
            t.g32(0x00ce_b248, rng.next());
            t.check(&format!("{w} saveMenuDisable"), 0x0070_2a70, c, &[], |g| {
                u(m::save_menu_disable(g))
            });
            t.check(
                &format!("{w} callCdecl"),
                0x00a2_8c00,
                c,
                &[x, cdecl1, y],
                |g| u(m::iggy_call_cdecl(g, cdecl1, y)),
            );
            t.check(
                &format!("{w} callMember"),
                0x0068_dfa0,
                c,
                &[x, this1, y],
                |g| u(m::esc_menu_call_member(g, x, this1, y)),
            );
            t.check(&format!("{w} controlMachine"), 0x006e_6450, c, &[], |g| {
                u(m::current_control_machine(g))
            });
            t.check(&format!("{w} setEnabled"), 0x006f_68f0, th, &[x], |g| {
                m::text_set_enabled(g, obj, x);
                Out::None
            });
            t.check(&format!("{w} rebuildVideo"), 0x006f_f020, th, &[], |g| {
                u(m::rebuild_video_setting_page(g, obj))
            });
            let f = if rng.below(3) == 0 { 0 } else { cdecl1 };
            t.w32(obj + 0x10, f);
            t.check(&format!("{w} crossThread"), 0x006f_29f0, th, &[], |g| {
                u(m::cross_thread_run(g, obj))
            });
            let flag = [0, 1, rng.next() as u8][rng.below(3) as usize];
            t.w8(obj, flag);
            t.check(
                &format!("{w} IggyValueRef_free"),
                0x00a2_8a20,
                th,
                &[],
                |g| {
                    m::iggy_value_ref_free(g, obj);
                    Out::None
                },
            );
            let mesh = if rng.below(3) == 0 { 0 } else { rng.next() };
            t.w32(obj + m::GEOMETRY_MESH, mesh);
            t.g32(0x00cc_b9d8, rng.next());
            t.check(&format!("{w} hide"), 0x006a_2370, th, &[], |g| {
                u(m::geometry_set_visible(g, obj, 0))
            });
            t.check(&format!("{w} show"), 0x006a_49e0, th, &[], |g| {
                u(m::geometry_set_visible(g, obj, 1))
            });
            let alt = if rng.below(2) == 0 { 0 } else { rng.next() };
            t.w32(obj + m::MENU_ALT, alt);
            t.check(&format!("{w} GetMenuCount"), 0x0067_b970, th, &[], |g| {
                u(m::menu_count(g, obj))
            });
            let value = if rng.below(4) == 0 { 0 } else { obj + 0x40 };
            t.check(&format!("{w} setType9"), 0x00a2_8dd0, c, &[value, y], |g| {
                m::iggy_value_set_type9(g, value, y);
                Out::None
            });
            let once = if rng.below(2) == 0 { 0 } else { rng.next() };
            t.g32(0x00ce_5194, once);
            t.check(&format!("{w} allocOnce"), 0x006e_5490, c, &[], |g| {
                m::alloc_once(g);
                Out::None
            });
            let (fx, fy) = (rng.float(), rng.float());
            t.check(
                &format!("{w} setPosition"),
                0x006f_6910,
                th,
                &[fx, fy],
                |g| {
                    m::text_set_position(g, obj, fx, fy);
                    Out::None
                },
            );
            t.g32(0x0198_4c6c, rng.next());
            t.check(&format!("{w} addCallback"), 0x00a2_8e90, th, &[x, y], |g| {
                u(m::iggy_menu_add_callback(g, obj, x, y))
            });
            t.check(&format!("{w} setTitleText"), 0x006f_6930, th, &[x], |g| {
                u(m::text_set_title(g, obj, x))
            });
            let (b4, b5) = (
                [0, 1, rng.next() as u8][rng.below(3) as usize],
                [0, 1, rng.next() as u8][rng.below(3) as usize],
            );
            t.w8(obj + 4, b4);
            t.w8(obj + 5, b5);
            t.check(
                &format!("{w} videoTextRender"),
                0x006d_deb0,
                th,
                &[x],
                |g| {
                    m::video_text_render(g, obj, x);
                    Out::None
                },
            );
            t.check(&format!("{w} fontWidth"), 0x008b_7be0, c, &[x, y, z], |g| {
                u(m::font_width_inter_mode(g, x, y, z))
            });
            t.check(&format!("{w} resetSplitScreen"), 0x0063_8d20, c, &[], |g| {
                u(m::reset_split_screen(g))
            });
            t.g32(0x0187_16cc, rng.next());
            t.check(&format!("{w} setEnvSubFont"), 0x0090_3300, c, &[], |g| {
                u(m::set_env_sub_font(g))
            });
        }
    }
}

/// `AsyncLoadManager_waitTasks`: a `SleepThread` stand-in that counts its
/// calls and clears `+0x14` on the `n`th, so the loop ends.
#[test]
fn wait_tasks() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_0005);
        for i in 0..24 {
            let obj = t.zeros(0x40);
            let n = t.zeros(4);
            let calls = t.zeros(4);
            let mut c = vec![0xff, 0x05];
            c.extend(calls.to_le_bytes()); // inc dword [calls]
            c.extend([0xff, 0x0d]);
            c.extend(n.to_le_bytes()); // dec dword [n]
            c.extend([0x75, 0x0a]); // jnz +10
            c.extend([0xc7, 0x05]);
            c.extend((obj + 0x14).to_le_bytes());
            c.extend(0u32.to_le_bytes()); // mov dword [obj+0x14], 0
            c.push(0xc3);
            let f = t.code(&c);
            let at = t.rebase(m::SLEEP_THREAD);
            let mut j = vec![0xe9];
            j.extend(f.wrapping_sub(at + 5).to_le_bytes());
            t.write(at, &j);
            t.w32(n, 1 + rng.below(4));
            for off in [0x14, 0x20, 0x2c] {
                let v = if rng.below(3) == 0 { 0 } else { rng.next() };
                t.w32(obj + off, v);
            }
            t.check(
                &format!("{name} {i} waitTasks"),
                0x0069_1d40,
                Conv::Thiscall(obj),
                &[],
                |g| {
                    m::wait_tasks(g, obj);
                    Out::None
                },
            );
            t.cave_next = t.cave;
        }
    }
}

// ---- batch 2

/// FMOD's event and file callbacks, the FIOS seek, the sfx event flags.
#[test]
fn fmod_callbacks() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_0101);
        t.stub(s::STREAM_IS_OPEN, true, 0, 0);
        t.stub(s::STREAM_SEEK, true, 2, 2);
        t.stub(s::STREAM_CLOSE, true, 0, 0);
        t.stub(s::STREAM_RELEASE, true, 0, 0);
        t.stub(s::OPERATOR_DELETE, false, 1, 0);
        for i in 0..40 {
            let w = format!("{name} {i}");
            for tag in [
                s::STREAM_IS_OPEN,
                s::STREAM_SEEK,
                s::STREAM_CLOSE,
                s::STREAM_RELEASE,
                s::OPERATOR_DELETE,
            ] {
                let v = rng.next();
                t.ret(tag, v);
            }
            // isOpen: al 0 or not, the rest random
            let open = (rng.next() & !0xff) | [0, 1, rng.below(256)][rng.below(3) as usize];
            t.ret(s::STREAM_IS_OPEN, open);
            let out = t.random(&mut rng, 8);
            let kind = [rng.word(), 2, 4, 3, 0][rng.below(5) as usize];
            let (x, y, z) = (rng.word(), rng.word(), rng.word());
            t.check(
                &format!("{w} eventcallback {kind}"),
                0x0070_7a50,
                Conv::Stdcall,
                &[x, kind, y, z, out],
                |g| u(s::event_callback(g, kind, out)),
            );
            let obj = t.random(&mut rng, 0x40);
            t.check(
                &format!("{w} fios seek"),
                s::FIOS_SEEK,
                Conv::Thiscall(obj),
                &[x, y],
                |g| u(s::fios_seek(g, obj, x, y)),
            );
            let handle = if rng.below(4) == 0 { 0 } else { obj };
            // FmodFile_seek calls the FIOS seek, ported here too: the real one, over the stubs
            t.check(
                &format!("{w} FmodFile_seek"),
                0x0070_85e0,
                Conv::Stdcall,
                &[handle, x, y],
                |g| u(s::fmod_file_seek(g, handle, x)),
            );
            t.check(
                &format!("{w} FmodFile_close"),
                0x0070_84d0,
                Conv::Stdcall,
                &[handle, y],
                |g| u(s::fmod_file_close(g, handle)),
            );
            // the event list: count (signed, small or negative), entries
            let n = [0, 1, 3, 7, u32::MAX, 0x8000_0000][rng.below(6) as usize];
            let entries = t.zeros(0x40);
            for k in 0..7 {
                let flag = if rng.below(4) == 0 {
                    rng.next() as u8 | 1
                } else {
                    0
                };
                t.w8(entries + 8 * k + 4, flag);
            }
            let list = t.zeros(0x10);
            t.w32(list, n);
            t.w32(list + 8, entries);
            let ra = t.check(
                &format!("{w} anyEventFlag"),
                0x0070_ed30,
                Conv::Thiscall(list),
                &[],
                |_| Out::None,
            );
            let mine = s::sfx_any_event_flag(&G(&mut t.b), list);
            assert_eq!(ra & 0xff, u32::from(mine), "{w} anyEventFlag al");
        }
    }
}

/// The movie manager and the PS2 movie layer's tests.
#[test]
fn movie_manager() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_0102);
        t.stub(mv::IN_LUCA_THEATER, false, 0, 0);
        t.stub(e::PHYRE_PRINTF, false, 2, 0);
        t.stub(mv::VIDEO_TERMINATE, true, 0, 0);
        t.stub(mv::VIDEO_TOGGLE_PAUSE, true, 0, 0);
        t.stub(mv::OPERATOR_NEW, false, 1, 0);
        let bad_alloc = t.stub_code(0xc0de_0101, false, 0, 0);
        t.g32(0x00b0_c2a0, bad_alloc);
        for i in 0..40 {
            let w = format!("{name} {i}");
            for tag in [
                mv::VIDEO_TERMINATE,
                mv::VIDEO_TOGGLE_PAUSE,
                e::PHYRE_PRINTF,
                0xc0de_0101,
            ] {
                let v = rng.next();
                t.ret(tag, v);
            }
            let theatre = if rng.below(3) == 0 { rng.next() } else { 0 };
            t.ret(mv::IN_LUCA_THEATER, theatre);
            let event = [0x48, 0x46, 0x2e, 0x20, 0x47, rng.word()][rng.below(6) as usize];
            t.g32(0x00c4_0e28, event);
            t.check(
                &format!("{w} with3DScene"),
                0x0076_f0c0,
                Conv::Cdecl,
                &[],
                |g| u(mv::with_3d_scene(g)),
            );
            t.check(
                &format!("{w} have_camera"),
                0x0076_efc0,
                Conv::Cdecl,
                &[],
                |g| u(mv::have_camera(g)),
            );
            let mgr = t.random(&mut rng, 0x800);
            let g = if rng.below(3) == 0 { 0 } else { mgr };
            t.g32(e::FMV_PLAYER_MANAGER.global, g);
            t.check(
                &format!("{w} terminate"),
                0x006d_9180,
                Conv::Thiscall(mgr),
                &[],
                |h| u(mv::fmv_terminate(h, mgr)),
            );
            let loaded = [0, 1, rng.next() as u8][rng.below(3) as usize];
            t.w8(mgr + mv::FMV_LOADED, loaded);
            let pause = [1, 0, rng.next()][rng.below(3) as usize];
            t.g32(0x0112_a008, pause);
            let paused = [0, 1, rng.next() as u8][rng.below(3) as usize];
            t.w8(mgr + mv::FMV_PAUSED, paused);
            t.check(
                &format!("{w} pause"),
                0x006d_a4b0,
                Conv::Thiscall(mgr),
                &[],
                |h| {
                    mv::fmv_pause(h, mgr);
                    Out::None
                },
            );
            if g != 0 {
                t.check(&format!("{w} resume"), 0x006d_8f80, Conv::Cdecl, &[], |h| {
                    u(mv::fmv_resume(h))
                });
            }
            let head = if rng.below(4) == 0 {
                0
            } else {
                t.random(&mut rng, 0x2c)
            };
            t.ret(mv::OPERATOR_NEW, head);
            for sym in [0x006d_78c0, 0x006a_0360] {
                t.check(
                    &format!("{w} tree head {sym:#x}"),
                    sym,
                    Conv::Cdecl,
                    &[],
                    |h| u(mv::tree_head_new(h)),
                );
            }
        }
        // resume with no manager: the message, then the global read again
        let created = t.zeros(0x800);
        t.stub_store(
            e::PHYRE_PRINTF,
            false,
            e::FMV_PLAYER_MANAGER.global,
            created,
            0,
        );
        t.g32(e::FMV_PLAYER_MANAGER.global, 0);
        t.check(
            &format!("{name} resume created"),
            0x006d_8f80,
            Conv::Cdecl,
            &[],
            |h| u(mv::fmv_resume(h)),
        );
    }
}

/// The menus', text layer's and loaders' glue.
#[test]
fn menu_glue() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_0103);
        let stubs: &[(u32, bool, u32, u32)] = &[
            (mn::ADD_CALL, true, 2, 2),
            (mn::QUEUE_DRAW, true, 1, 1),
            (mn::CLUSTER_CREATE, true, 3, 3),
            (mn::CLUSTER_MANAGER, false, 0, 0),
            (mn::LOAD_PCLUSTER, true, 1, 1),
            (mn::ASYNC_LOAD_MANAGER, false, 0, 0),
            (mn::MOTION_BLUR_ON_LOADED, true, 2, 2),
            (mn::VFX_RENDER_DATA, false, 2, 0),
            (mn::TEXTURE_VIDEO_PLAY, false, 1, 0),
            (mn::FORCE_PLAY_CLICK, false, 1, 0),
            (mn::TEXT_ALL_LOADED, true, 0, 0),
            (m::OPERATOR_NEW, false, 1, 0),
            (mn::ICON_PANEL_CTOR, true, 0, 0),
            (m::PSTRING_ASSIGN, true, 1, 1),
            (mn::IGGY_MANAGER_CTOR, true, 0, 0),
            (mn::POST_EFFECT.2, true, 0, 0),
            (mn::REMAP_BUTTON_ICON.2, true, 0, 0),
            (m::IGGY_SET_ENABLE, true, 1, 1),
            (mn::IGGY_MENU_DTOR, true, 0, 0),
            (s::OPERATOR_DELETE, false, 1, 0),
            (mn::IS_PRESS_BUTTON, false, 1, 0),
            (mn::DATA_VALUE_SET, false, 2, 0),
            (mn::PHYRE_FREE, false, 1, 0),
        ];
        for &(sym, this, n, pops) in stubs {
            t.stub(sym, this, n, pops);
        }
        let done = t.stub_code(0xc0de_0201, false, 2, 0);
        let iob = t.stub_code(0xc0de_0202, false, 0, 0);
        let fprintf = t.stub_code(0xc0de_0203, false, 2, 0);
        t.g32(0x00b0_c350, iob);
        t.g32(0x00b0_c38c, fprintf);
        let vslot = t.stub_code(0xc0de_0204, true, 1, 1);
        let vtable = t.zeros(4);
        t.w32(vtable, vslot);
        for i in 0..40 {
            let w = format!("{name} {i}");
            for &(sym, ..) in stubs {
                let v = rng.next();
                t.ret(sym, v);
            }
            for tag in [0xc0de_0201, 0xc0de_0202, 0xc0de_0203, 0xc0de_0204] {
                let v = rng.next();
                t.ret(tag, v);
            }
            let (x, y, z) = (rng.word(), rng.word(), rng.word());
            let obj = t.random(&mut rng, 0x400);
            let th = Conv::Thiscall(obj);
            let c = Conv::Cdecl;
            let idx = rng.below(16);
            t.check(&format!("{w} addHandler"), 0x0070_2570, th, &[idx], |g| {
                u(mn::esc_add_handler(g, obj, idx))
            });
            // the mesh's +0x10 may be the geometry's +0xc (the original
            // reads +0xc again after that store): then the buffer is the
            // mesh the second store goes to
            let alias = rng.below(4) == 0;
            let mesh = if alias {
                obj - 4
            } else {
                t.random(&mut rng, 0x40)
            };
            t.w32(obj + 0xc, mesh);
            let buffer = if alias {
                t.random(&mut rng, 0x40)
            } else if rng.below(2) == 0 {
                0
            } else {
                rng.next()
            };
            t.w32(obj + 0x14, buffer);
            t.check(&format!("{w} applyBuffer"), 0x006a_46c0, th, &[], |g| {
                u(mn::geometry_apply_buffer(g, obj))
            });
            for off in [0x68, 0x6c] {
                let v = if rng.below(3) == 0 { 0 } else { rng.next() };
                t.w32(obj + off, v);
            }
            let on = [0, 1, rng.next() as u8][rng.below(3) as usize];
            t.w8(obj + 0x70, on);
            t.check(&format!("{w} IggyMenu_Draw"), 0x00a2_8eb0, th, &[], |g| {
                u(mn::iggy_menu_draw(g, obj))
            });
            let created = [0, 4, rng.next()][rng.below(3) as usize];
            t.ret(mn::CLUSTER_CREATE, created);
            let ecx = rng.next();
            t.check(
                &format!("{w} createInRoot"),
                0x006f_7a00,
                Conv::Thiscall(ecx),
                &[x],
                |g| u(mn::cluster_create_in_root(g, ecx, x)),
            );
            let f = if rng.below(3) == 0 { 0 } else { done };
            t.w32(obj + 8, f);
            t.check(
                &format!("{w} LoadClusterRequest"),
                0x0069_1320,
                th,
                &[],
                |g| u(mn::load_cluster_request_run(g, obj)),
            );
            let args = t.zeros(0xc);
            let blur = t.random(&mut rng, 0x60);
            t.w32(args, blur);
            t.w32(args + 4, y);
            t.w32(args + 8, z);
            t.check(
                &format!("{w} motionBlur loaded"),
                0x006c_53e0,
                c,
                &[x, args],
                |g| u(mn::motion_blur_on_loaded(g, x, args)),
            );
            let data = if rng.below(3) == 0 { 0 } else { rng.next() };
            t.ret(mn::VFX_RENDER_DATA, data);
            let ready = if rng.below(2) == 0 { 0 } else { rng.next() };
            t.g32(0x00cc_b9cc, ready);
            t.check(&format!("{w} videoPlayIfReady"), 0x0071_85e0, c, &[], |g| {
                u(mn::video_play_if_ready(g))
            });
            let played = [0, 1, rng.next() as u8][rng.below(3) as usize];
            t.w8(obj + mn::ESC_SOUND_PLAYED, played);
            t.check(&format!("{w} playMenuSound"), 0x0068_f9b0, th, &[x], |g| {
                mn::esc_play_menu_sound(g, obj, x);
                Out::None
            });
            let slot = t.zeros(8);
            t.w32(slot, obj);
            let k = [0, 1, 2, rng.below(8)][rng.below(4) as usize];
            t.w32(slot + 4, k);
            t.check(
                &format!("{w} assetLoaded {k}"),
                0x006f_63f0,
                c,
                &[x, slot],
                |g| u(mn::text_on_asset_loaded(g, x, slot)),
            );
            let p = if rng.below(4) == 0 { 0 } else { rng.next() };
            t.ret(m::OPERATOR_NEW, p);
            t.check(&format!("{w} IconPanel create"), 0x006f_77f0, c, &[], |g| {
                u(mn::icon_panel_create(g))
            });
            let count = [0, 1, u32::MAX, 0x8000_0000, rng.word()][rng.below(5) as usize];
            t.g32(0x00c4_21dc, count);
            t.check(
                &format!("{w} chrLoadCountdown {count:#x}"),
                0x0078_2fa0,
                c,
                &[],
                |g| {
                    mn::chr_load_countdown(g);
                    Out::None
                },
            );
            let lines = t.zeros(0x40);
            t.w32(obj + 0x78, lines);
            let n = [0, 3, 0x8000_0003, rng.word()][rng.below(4) as usize];
            t.w32(obj + 0x74, n);
            let li = [0, 2, 3, rng.word()][rng.below(4) as usize];
            t.check(&format!("{w} changeLine"), 0x006f_6070, th, &[li, y], |g| {
                u(mn::text_change_line(g, obj, li, y))
            });
            for (sym, inst) in [
                (0x00a2_6240, mn::IGGY_MENU_MANAGER_NEW),
                (0x006e_76c0, mn::REMAP_BUTTON_ICON),
                (0x006c_5790, mn::POST_EFFECT),
            ] {
                let gv = if rng.below(2) == 0 { 0 } else { rng.next() };
                let at = inst.0.ptr(&t.b).addr;
                t.w32(at, gv);
                t.check(&format!("{w} createInstance {sym:#x}"), sym, c, &[], |g| {
                    mn::create_instance(g, inst);
                    Out::None
                });
            }
            for k in 0..7 {
                let icon = if rng.below(3) == 0 { 0 } else { rng.next() };
                t.w32(obj + 4 * k, icon);
            }
            t.check(&format!("{w} setVisible"), 0x006f_7940, th, &[x], |g| {
                mn::icon_panel_set_visible(g, obj, x);
                Out::None
            });
            t.check(&format!("{w} hideAll"), 0x006f_78d0, th, &[], |g| {
                mn::icon_panel_hide_all(g, obj);
                Out::None
            });
            let arr = t.random(&mut rng, 0x10);
            let data = t.random(&mut rng, 8);
            let n = [0, 1, 2, 0x8000_0000, 0x8000_0001, rng.next()][rng.below(6) as usize];
            t.w32(arr, n);
            t.w32(arr + 4, data);
            t.w32(obj + 0x14, arr);
            t.check(
                &format!("{w} resetMaterials {n:#x}"),
                0x006a_2c60,
                th,
                &[],
                |g| u(mn::reset_materials(g, obj)),
            );
            let table = t.zeros(0x20);
            for k in 0..8 {
                let e = t.random(&mut rng, 8);
                t.w32(table + 4 * k, e);
            }
            t.w32(obj + 0x38, table);
            let (ei, ek) = (
                rng.below(8),
                [1, 2, 0, 3, rng.next()][rng.below(5) as usize],
            );
            t.check(
                &format!("{w} entryWord {ek}"),
                0x006d_5190,
                th,
                &[ei, ek],
                |g| u(mn::broken_screen_entry_word(g, obj, ei, ek)),
            );
            let pressed = if rng.below(2) == 0 { 0 } else { rng.next() };
            t.ret(mn::IS_PRESS_BUTTON, pressed);
            t.check(
                &format!("{w} CheckEncounterKey"),
                0x006f_75c0,
                c,
                &[],
                |g| u(mn::check_encounter_key(g)),
            );
            let out = t.zeros(0x10);
            t.check(
                &format!("{w} LocString_ToValue"),
                0x0068_e270,
                th,
                &[out],
                |g| u(mn::loc_string_to_value(g, obj, out)),
            );
            for (sym, stride) in [(0x006b_01f0, 0x20), (0x006b_0230, 0x40)] {
                let n = rng.below(4);
                let array = if rng.below(4) == 0 { 0 } else { t.zeros(0x100) };
                if array != 0 {
                    for k in 0..n {
                        t.w32(array + stride * k, vtable);
                    }
                }
                t.check(&format!("{w} destroy {sym:#x}"), sym, c, &[array, n], |g| {
                    mn::parray_destroy(g, array, n, stride);
                    Out::None
                });
            }
            let once = if rng.below(2) == 0 { 0 } else { rng.next() };
            t.g32(0x00ce_63c4, once);
            let p = if rng.below(4) == 0 {
                0
            } else {
                t.random(&mut rng, 4)
            };
            t.ret(m::OPERATOR_NEW, p);
            t.check(&format!("{w} allocOnce2"), 0x006e_56a0, c, &[], |g| {
                mn::alloc_once_2(g);
                Out::None
            });
        }
    }
}

/// The PS2 packet helpers.
#[test]
fn ps2_packets() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_0104);
        let stubs: &[(u32, bool, u32, u32)] = &[
            (m::SET_SCREEN_WH, false, 2, 0),
            (m::DMA_GET_CHAN, false, 1, 0),
            (m::ADD_PKT_MENU_ICON_RGBA, false, 8, 0),
            (m::EASY_SHAPE_XYWHC2, false, 8, 0),
            (m::FLUSH_CACHE, false, 1, 0),
            (m::DMA_SEND, false, 2, 0),
            (m::DMA_SYNC, false, 3, 0),
            (m::GET_SPAD_PTR, false, 0, 0),
            (m::SEND_SPAD, false, 1, 0),
            (m::DMA_LOSS_REPORT, false, 0, 0),
            (m::CTRL_PKT, false, 0, 0),
        ];
        for &(sym, this, n, pops) in stubs {
            t.stub(sym, this, n, pops);
        }
        for i in 0..30 {
            let w = format!("{name} {i}");
            for &(sym, ..) in stubs {
                let v = rng.next();
                t.ret(sym, v);
            }
            let spad = t.random(&mut rng, 0x20);
            t.ret(m::GET_SPAD_PTR, spad);
            let c = Conv::Cdecl;
            let (a, b, cc, d, e5) = (rng.word(), rng.word(), rng.word(), rng.word(), rng.word());
            t.check(&format!("{w} TOInitScreen"), 0x008f_ae20, c, &[], |g| {
                u(m::to_init_screen(g))
            });
            t.check(
                &format!("{w} AddPktMenuIcon"),
                0x008e_b240,
                c,
                &[a, b, cc, d],
                |g| u(m::add_pkt_menu_icon(g, a, b, cc, d)),
            );
            t.g32(0x0187_16cc, rng.next());
            t.check(
                &format!("{w} ShapeXYC2"),
                0x0090_3a50,
                c,
                &[a, b, cc, d, e5],
                |g| u(m::mkp_shape_xyc2(g, a, b, cc, d, e5)),
            );
            let packet = t.random(&mut rng, 4);
            t.check(
                &format!("{w} rcPacketDataDraw"),
                0x0092_7600,
                c,
                &[packet],
                |g| u(m::rc_packet_data_draw(g, packet)),
            );
            t.check(&format!("{w} MFIFOEnd"), 0x0084_3260, c, &[], |g| {
                u(m::mfifo_end(g))
            });
        }
    }
}

/// The exact copies, each against its own original.
#[test]
fn copies_of_ports() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_0105);
        // PhyreCursor_IsValid: a list of nodes linked at +0x40, the cursor
        for i in 0..60 {
            let list = t.random(&mut rng, 0x50);
            let nodes: Vec<u32> = (0..=rng.below(4)).map(|_| t.zeros(0x50)).collect();
            for w in nodes.windows(2) {
                t.w32(w[0] + 0x40, w[1]);
            }
            let first = if rng.below(6) == 0 { 0 } else { nodes[0] };
            t.w32(list + 0x2c, first);
            t.w32(list + 0x40, rng.below(2));
            let cur = t.zeros(0x20);
            let l = if rng.below(6) == 0 { 0 } else { list };
            t.w32(cur + 0x18, l);
            let at = if rng.below(2) == 0 {
                nodes[rng.below(nodes.len() as u32) as usize]
            } else {
                rng.next()
            };
            t.w32(cur + 0x1c, at);
            for sym in [0x0063_7f80, 0x006f_63b0] {
                let ra = t.check(
                    &format!("{name} {i} cursor {sym:#x}"),
                    sym,
                    Conv::Thiscall(cur),
                    &[],
                    |_| Out::None,
                );
                let mine = ffx_graphic::managers::bridge::cursor_valid(&G(&mut t.b), cur);
                assert_eq!(ra & 0xff, u32::from(mine), "{name} {i} cursor {sym:#x} al");
            }
        }
        // TaggedTreeInsert: one tree grown in both
        let head = t.zeros(0x10);
        t.w32(head + 0xc, head);
        for i in 0..30 {
            let node = t.random(&mut rng, 0x10);
            t.w32(node + 0xc, rng.below(40));
            t.check(
                &format!("{name} {i} tagged insert"),
                0x0069_dae0,
                Conv::Thiscall(head),
                &[node],
                |g| u(ffx_system::stl::bridge::tagged_tree_insert(g, head, node)),
            );
        }
        // PArray_PushInto
        t.stub(mn::PHYRE_FREE, false, 1, 0);
        for i in 0..24 {
            for sym in [0x006a_2670, 0x006a_2700] {
                let arr = t.zeros(8);
                let n = rng.below(6);
                let old = t.random(&mut rng, 0x20);
                let flag = [0, 0x8000_0000][rng.below(2) as usize];
                t.w32(arr, n | flag);
                t.w32(arr + 4, old);
                let dst = [0, t.zeros(0x20), old][rng.below(3) as usize];
                let value = t.random(&mut rng, 4);
                t.check(
                    &format!("{name} {i} push {sym:#x}"),
                    sym,
                    Conv::Thiscall(arr),
                    &[dst, value],
                    |g| {
                        u(ffx_system::container::bridge::parray_push_into(
                            g, arr, dst, value, c_free,
                        ))
                    },
                );
            }
        }
        // vfxIntTree_Insert copies, then SfxEventTree_Find copies on the trees
        for sym in [0x0070_bb20, 0x0070_bba0, 0x0071_0540] {
            let tree = t.zeros(0x10);
            t.w32(tree + 0xc, tree);
            let count = 1 + rng.below(0x30);
            let nodes = t.zeros(0x20 * count as usize);
            for k in 0..count {
                let node = nodes + 0x20 * k;
                let fill = t.random(&mut rng, 0x10);
                let bytes = Oracle::read(&mut t.a, fill, 0x10).unwrap();
                t.write(node, &bytes);
                t.w32(node + 0xc, (rng.below(0x20) as i32 - 0x10) as u32);
                t.check(
                    &format!("{name} {sym:#x} insert {k}"),
                    sym,
                    Conv::Thiscall(tree),
                    &[node],
                    |g| u(ffx_vfx::tree::bridge::vfx_int_tree_insert(g, tree, node)),
                );
            }
            let key = t.zeros(4);
            for k in 0..12 {
                let id = (rng.below(0x24) as i32 - 0x12) as u32;
                t.w32(key, id);
                for find in [0x0070_ba20, 0x0071_04c0] {
                    t.check(
                        &format!("{name} {sym:#x} find {find:#x} {k}"),
                        find,
                        Conv::Thiscall(tree),
                        &[key],
                        |g| u(ffx_sound::ffxfmod::bridge::sfx_find(g, tree, id)),
                    );
                }
            }
        }
    }
}

/// The `PArray_PushInto` copies' free, as the shim calls it.
fn c_free(g: &mut G, p: u32) {
    ffx_glue_other::copies::phyre_free(g, p);
}

// ---- batch 3

/// FMOD's alloc and read callbacks, the event pause loop.
#[test]
fn fmod_alloc_read_pause() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_0201);
        t.stub(s::USER_MALLOC, false, 1, 0);
        t.stub(s::STREAM_READ, true, 2, 2);
        t.stub(s::EVENT_SET_PAUSED, false, 2, 2);
        t.stub(s::ERRCHECK, false, 3, 0);
        for i in 0..40 {
            let w = format!("{name} {i}");
            for tag in [s::STREAM_READ, s::EVENT_SET_PAUSED, s::ERRCHECK] {
                let v = rng.next();
                t.ret(tag, v);
            }
            // a block at each alignment
            let block = t.zeros(0x60) + rng.below(16);
            t.ret(s::USER_MALLOC, block);
            let size = [0, 1, 0x20, rng.next()][rng.below(4) as usize];
            let (k, src) = (rng.next(), rng.next());
            t.check(
                &format!("{w} alloc {size:#x} at {block:#x}"),
                0x0070_66d0,
                Conv::Stdcall,
                &[size, k, src],
                |g| u(s::fmod_memory_alloc(g, size)),
            );
            let handle = if rng.below(4) == 0 { 0 } else { rng.next() };
            let read = if rng.below(4) == 0 {
                0
            } else {
                t.random(&mut rng, 4)
            };
            let size = [0, 0x10, 0x100, rng.next()][rng.below(4) as usize];
            let n = [size, size.wrapping_sub(1), 0, rng.next()][rng.below(4) as usize];
            t.ret(s::STREAM_READ, n);
            let buf = rng.next();
            t.check(
                &format!("{w} read"),
                0x0070_85a0,
                Conv::Stdcall,
                &[handle, buf, size, read, k],
                |g| u(s::fmod_file_read(g, handle, buf, size, read)),
            );
            let events = t.random(&mut rng, 0x40);
            let list = t.zeros(0x10);
            let count = [0, 1, 3, 8, u32::MAX][rng.below(5) as usize];
            t.w32(list, count);
            t.w32(list + 8, events);
            let paused = rng.word();
            t.check(
                &format!("{w} setPaused {count}"),
                0x0070_f100,
                Conv::Thiscall(list),
                &[paused],
                |g| {
                    s::sfx_event_set_paused(g, list, paused);
                    Out::None
                },
            );
        }
    }
}

/// The FMV manager's state checks, volume and the map node.
#[test]
fn movie_state_volume() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_0202);
        t.stub(mv::VIDEO_SET_VOLUME, true, 1, 1);
        t.stub(mv::OPERATOR_NEW, false, 1, 0);
        let bad_alloc = t.stub_code(0xc0de_0301, false, 0, 0);
        t.g32(0x00b0_c2a0, bad_alloc);
        for i in 0..50 {
            let w = format!("{name} {i}");
            let v = rng.next();
            t.ret(mv::VIDEO_SET_VOLUME, v);
            let mgr = t.random(&mut rng, 0x800);
            let states = t.zeros(0x20);
            for k in 0..8 {
                let s = [0, 1, 2, rng.word()][rng.below(4) as usize];
                t.w32(states + 4 * k, s);
            }
            t.w32(mgr + mv::FMV_STATES, states);
            let n = [0, 1, 3, 8][rng.below(4) as usize];
            t.w32(mgr + mv::FMV_STATE_COUNT, n);
            let pending = if rng.below(2) == 0 { 0 } else { rng.next() };
            t.w32(mgr + mv::FMV_PENDING, pending);
            let want = [0, 1, 2, rng.word()][rng.below(4) as usize];
            t.check(
                &format!("{w} hasState"),
                0x006d_a1f0,
                Conv::Thiscall(mgr),
                &[want],
                |g| u(mv::fmv_has_state(g, mgr, want)),
            );
            let ra = t.check(
                &format!("{w} StateCheck"),
                0x006d_a2a0,
                Conv::Thiscall(mgr),
                &[],
                |_| Out::None,
            );
            assert_eq!(
                ra & 0xff,
                u32::from(mv::fmv_state_check(&G(&mut t.b), mgr)),
                "{w} StateCheck al"
            );
            let g = if rng.below(3) == 0 { 0 } else { mgr };
            t.g32(e::FMV_PLAYER_MANAGER.global, g);
            let loaded = [0, 1, rng.next() as u8][rng.below(3) as usize];
            t.w8(mgr + mv::FMV_LOADED, loaded);
            let pause = [1, 0, rng.next()][rng.below(3) as usize];
            t.g32(0x0112_a008, pause);
            let vol = rng.float();
            t.check(
                &format!("{w} SetVolume"),
                0x006d_abf0,
                Conv::Cdecl,
                &[vol],
                |h| {
                    mv::fmv_set_volume(h, vol);
                    Out::None
                },
            );
            let p = if rng.below(5) == 0 {
                0
            } else {
                t.random(&mut rng, 0x14)
            };
            t.ret(mv::OPERATOR_NEW, p);
            if p != 0 {
                let left = if rng.below(2) == 0 { 0 } else { rng.next() };
                let right = rng.next();
                t.check(
                    &format!("{w} node"),
                    0x006d_78f0,
                    Conv::Stdcall,
                    &[left, right],
                    |h| u(mv::tree_node_new(h, left, right)),
                );
            }
        }
    }
}

/// The menus' records and updates, Iggy references and handler records,
/// the button remap, map heads and nodes, the icon panel.
#[test]
fn menu_records_and_iggy() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_0203);
        let stubs: &[(u32, bool, u32, u32)] = &[
            (mn::IGGY_MENU_DTOR, true, 0, 0),
            (s::OPERATOR_DELETE, false, 1, 0),
            (mn::REMAP_UI_BUTTON_DATA, true, 4, 4),
            (mn::REMAP_MESSAGE_INPUT_DATA, true, 4, 4),
            (mn::SWAP_MENU_BUFFER_CHAIN, true, 0, 0),
            (mn::SET_UP_ALL_MENU_INSTANCE, true, 0, 0),
            (mn::MAP_NODE_NEW, true, 0, 0),
            (m::OPERATOR_NEW, false, 1, 0),
        ];
        for &(sym, this, n, pops) in stubs {
            t.stub(sym, this, n, pops);
        }
        let from_temp = t.stub_code(0xc0de_0401, false, 3, 3);
        let path = t.stub_code(0xc0de_0402, false, 3, 3);
        t.g32(0x00b0_c828, from_temp);
        t.g32(0x00b0_c82c, path);
        let invoker = t.stub_code(0xc0de_0403, false, 3, 0);
        let bad_alloc = t.stub_code(0xc0de_0404, false, 0, 0);
        t.g32(0x00b0_c2a0, bad_alloc);
        let scene = t.random(&mut rng, 0x10000);
        for i in 0..40 {
            let w = format!("{name} {i}");
            for &(sym, ..) in stubs {
                let v = rng.next();
                t.ret(sym, v);
            }
            for tag in [0xc0de_0401, 0xc0de_0402, 0xc0de_0403] {
                let v = rng.next();
                t.ret(tag, v);
            }
            let obj = t.random(&mut rng, 0x6000);
            let th = Conv::Thiscall(obj);
            let (x, y) = (rng.word(), rng.word());
            for k in 0..7 {
                let icon = if rng.below(3) == 0 { 0 } else { rng.next() };
                t.w32(obj + 4 * k, icon);
            }
            // negative indices read below the panel (signed compare): -1, -2
            let idx = [0, 3, 6, 7, u32::MAX, 0xffff_fffe, 0x7fff_ffff][rng.below(7) as usize];
            t.check(
                &format!("{w} hideIcon {idx:#x}"),
                0x006f_7900,
                th,
                &[idx],
                |g| {
                    mn::icon_panel_hide_icon(g, obj, idx);
                    Out::None
                },
            );
            t.check(&format!("{w} valueRef"), 0x00a2_88b0, th, &[x, y], |g| {
                u(mn::iggy_value_ref_create(g, obj, x, y))
            });
            for k in 0..5 {
                let rec = obj + 4 + 0x14 * k;
                let flag = [0, 1, rng.next() as u8][rng.below(3) as usize];
                t.w8(rec, flag);
                t.w32(rec + 0x10, invoker);
            }
            t.check(&format!("{w} flush"), 0x00a2_8ff0, th, &[], |g| {
                mn::iggy_handlers_flush(g, obj);
                Out::None
            });
            let args: [u32; 5] = std::array::from_fn(|_| rng.word());
            t.check(&format!("{w} remap"), 0x006e_69e0, th, &args, |g| {
                u(mn::remap_button_icon(g, obj, args))
            });
            t.g32(0x00cc_b9d8, scene);
            for (at, v) in [
                (scene + 0xfa9d, rng.below(3)),
                (scene + 0xfa9c, rng.below(3)),
                (obj + 0x5b9c, rng.below(3)),
            ] {
                let b = if v == 0 { 0 } else { rng.next() as u8 | 1 };
                t.w8(at, b);
            }
            t.check(&format!("{w} UpdateMenu"), 0x0067_faa0, th, &[], |g| {
                u(mn::menu_update(g, obj))
            });
            let ready = [0, 1, rng.next() as u8][rng.below(3) as usize];
            t.w8(obj + 0x5b9d, ready);
            let alt = if rng.below(2) == 0 { 0 } else { rng.next() };
            t.w32(obj + 0x5b7c, alt);
            let mi = rng.below(16);
            for table in [0x15f0, 0x4248] {
                let rec = t.random(&mut rng, 4);
                t.w32(obj + table + 4 * mi, rec);
            }
            t.check(&format!("{w} GetMenuRecord"), 0x0067_b990, th, &[mi], |g| {
                u(mn::menu_record(g, obj, mi))
            });
            let slots = t.random(&mut rng, 0x20);
            let n = [0, 1, 8, 0x8000_0008][rng.below(4) as usize];
            t.w32(obj + 8, n);
            t.w32(obj + 0xc, slots);
            let menu = match rng.below(3) {
                0 => 0,
                1 => rng.next(),
                _ => {
                    // sometimes twice: only the first is cleared
                    let v = rng.next() | 1;
                    t.w32(slots + 4 * rng.below(8), v);
                    t.w32(slots + 4 * rng.below(8), v);
                    v
                }
            };
            t.check(&format!("{w} removeMenu"), 0x00a2_6c50, th, &[menu], |g| {
                mn::iggy_manager_remove_menu(g, obj, menu);
                Out::None
            });
            let p = t.random(&mut rng, 0x30);
            t.ret(m::OPERATOR_NEW, p);
            t.check(&format!("{w} headCopy"), 0x0069_a1e0, th, &[], |g| {
                u(mn::tree_head_copy(g, obj))
            });
            t.ret(mn::MAP_NODE_NEW, p);
            let key = t.random(&mut rng, 0x10);
            t.check(&format!("{w} nodeWithKey"), 0x0069_c580, th, &[key], |g| {
                u(mn::tree_node_new_with_key(g, obj, key))
            });
        }
    }
}

/// The post-effect setters and constructor, the scene colour, the PS2
/// shape, the thread call, the text animation.
#[test]
fn effects_threads_text() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_0204);
        t.stub(e::FIND_PARAM_BY_NAME, true, 1, 1);
        t.stub(e::SET_PARAM_VALUE, true, 2, 2);
        t.stub(e::MEMSET, false, 3, 0);
        t.stub(m::EASY_SHAPE_XYWHC2, false, 8, 0);
        t.stub(m::CTRL_PKT, false, 0, 0);
        let fns: Vec<u32> = (0..4)
            .map(|k| t.stub_code(0xc0de_0500 + k, k == 0, k, 0))
            .collect();
        let big = t.random(&mut rng, 0x1_2400);
        for i in 0..60 {
            let w = format!("{name} {i}");
            for tag in [
                e::FIND_PARAM_BY_NAME,
                e::SET_PARAM_VALUE,
                e::MEMSET,
                m::EASY_SHAPE_XYWHC2,
                m::CTRL_PKT,
            ] {
                let v = rng.next();
                t.ret(tag, v);
            }
            for k in 0..4 {
                let v = rng.next();
                t.ret(0xc0de_0500 + k, v);
            }
            let off = [0, 1, rng.next() as u8][rng.below(3) as usize];
            t.w8(big + 0x1_23a0, off);
            let v = t.zeros(0x10);
            for k in 0..4 {
                let f = rng.float();
                t.w32(v + 4 * k, f);
            }
            t.check(
                &format!("{w} setColor4"),
                0x0063_8840,
                Conv::Thiscall(big),
                &[v],
                |g| {
                    e::scene_set_color4(g, big, v);
                    Out::None
                },
            );
            let obj = t.random(&mut rng, 0x800);
            let th = Conv::Thiscall(obj);
            let params = if rng.below(4) == 0 { 0 } else { rng.next() };
            t.w32(obj + 0x64, params);
            let param = if rng.below(4) == 0 { 0 } else { rng.next() };
            t.ret(e::FIND_PARAM_BY_NAME, param);
            let buffer = if rng.below(4) == 0 {
                0
            } else {
                t.random(&mut rng, 8)
            };
            t.w32(obj + 0x60, buffer);
            let wt = rng.float();
            t.check(&format!("{w} setWeight"), 0x006c_5600, th, &[wt], |g| {
                e::motion_blur_set_weight(g, obj, wt);
                Out::None
            });
            t.check(&format!("{w} PostEffect ctor"), 0x006c_5670, th, &[], |g| {
                u(e::post_effect_ctor(g, obj))
            });
            t.g32(0x0187_16cc, rng.next());
            let (a, b, c) = (rng.word(), rng.word(), rng.word());
            t.check(
                &format!("{w} ShapeXY"),
                0x0090_3a10,
                Conv::Cdecl,
                &[a, b, c],
                |g| u(m::mkp_shape_xy(g, a, b, c)),
            );
            let rec = t.random(&mut rng, 0x20);
            for k in 0..4 {
                t.w32(rec + 4 * k, fns[k as usize]);
            }
            let kind = [0, 1, 2, 3, 4, rng.next()][rng.below(6) as usize];
            t.w32(rec + 0x10, kind);
            t.check(
                &format!("{w} threadCall {kind:#x}"),
                0x0070_3910,
                Conv::Thiscall(rec),
                &[],
                |g| u(m::thread_call_run(g, rec)),
            );
            // the timer near, at and past 300, NaNs too
            let timer = [
                rng.float(),
                299.5f32.to_bits(),
                300.0f32.to_bits(),
                (rng.below(400) as f32).to_bits(),
                0x7fc0_0000,
            ][rng.below(5) as usize];
            t.w32(obj + 0x40, timer);
            let frame =
                [0, 15, 0x7fff_ffff, u32::MAX, 0x8000_000f, rng.next()][rng.below(6) as usize];
            t.w32(obj + 0x44, frame);
            let dt = [rng.float(), 0.5f32.to_bits(), 0, 1.0f32.to_bits()][rng.below(4) as usize];
            t.check(&format!("{w} animate"), 0x006f_69f0, th, &[dt], |g| {
                m::text_animate(g, obj, dt);
                Out::None
            });
        }
    }
}

// ---- batch 4

/// The volume scale (`ST0`), the music data, the word arrays.
#[test]
fn fmod_volume_music_arrays() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_0301);
        t.stub(s::PRINT_TO_OUTPUT, false, 2, 0);
        t.stub(s::LOAD_EVENT_PROJECT, true, 4, 4);
        // the reserves log the count they see (`[ecx]`) in the `ecx` slot
        t.stub_deref(0x0070_d3a0, 1, 1);
        t.stub_deref(0x0066_7e00, 1, 1);
        let at = t.rebase(0x0070_ea20);
        for v in [
            0u32,
            1,
            63,
            64,
            126,
            127,
            128,
            200,
            u32::MAX,
            0x8000_0000,
            0x7fff_ffff,
            1000,
        ]
        .into_iter()
        .chain((0..40).map(|_| rng.word()))
        {
            let ra = t.a.call(at, Conv::Stdcall, &[v]).unwrap();
            let mine = s::volume_from_127(&G(&mut t.b), v);
            let theirs = ra.st0_f32().expect("ST0");
            assert_eq!(theirs.to_bits(), mine.to_bits(), "{name} volume {v:#x}");
            t.diff(&format!("{name} volume {v:#x}"));
        }
        for i in 0..40 {
            let w = format!("{name} {i}");
            for tag in [
                s::PRINT_TO_OUTPUT,
                s::LOAD_EVENT_PROJECT,
                0x0070_d3a0,
                0x0066_7e00,
            ] {
                let v = rng.next();
                t.ret(tag, v);
            }
            let obj = t.random(&mut rng, 0x40);
            let system = if rng.below(4) == 0 { 0 } else { rng.next() };
            t.check(
                &format!("{w} initData"),
                0x0070_8c00,
                Conv::Thiscall(obj),
                &[system],
                |g| u(s::fmod_music_init_data(g, obj, system)),
            );
            for (sym, reserve) in [(0x0070_c5c0, 0x0070_d3a0), (0x0071_09e0, 0x0066_7e00)] {
                let dst = t.random(&mut rng, 0x10);
                let storage = t.zeros(0x40);
                // a null storage faults past the first word, in both
                t.w32(dst + 8, storage);
                let src = if rng.below(5) == 0 {
                    dst
                } else {
                    t.random(&mut rng, 0x10)
                };
                if src != dst {
                    let words = t.random(&mut rng, 0x40);
                    t.w32(src + 8, words);
                    let n = [0, 1, 5, 16, u32::MAX][rng.below(5) as usize];
                    t.w32(src, n);
                }
                t.check(
                    &format!("{w} assign {sym:#x}"),
                    sym,
                    Conv::Thiscall(dst),
                    &[src],
                    |g| u(s::word_array_assign(g, dst, src, reserve)),
                );
            }
        }
    }
}

/// The FMV initialize, `IggyMenu`'s destructor, the escape menu's resume
/// and dialog text, the save pop-up, array copies, the video text size.
#[test]
fn menu_lifecycle() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_0302);
        let stubs: &[(u32, bool, u32, u32)] = &[
            (e::PHYRE_PRINTF, false, 2, 0),
            (mv::FMOD_MANAGER_WORD, true, 0, 0),
            (mv::VIDEO_INITIALIZE, true, 3, 3),
            (0x00a2_6c50, true, 1, 1),
            (mn::PLAYER_RELEASE, true, 1, 1),
            (mn::PLAYER_DTOR, true, 0, 0),
            (mn::PHYRE_FREE, false, 1, 0),
            (mn::SFX_PAUSE, false, 1, 0),
            (mn::VOICE_SET_PAUSED, false, 1, 0),
            (mn::VIDEO_PAUSE, false, 0, 0),
            (mn::PHYRE_MALLOC, false, 2, 0),
            (mn::TEXT_RESIZE, true, 2, 2),
            (mn::LOC_TEXT, false, 3, 0),
            (mn::SAVE_DATA_LOC, false, 2, 0),
            (mn::TEXT_CONVERT, false, 3, 0),
            (mn::DATA_VALUE_SET, false, 2, 0),
            (mn::CALL_AS3, true, 5, 5),
        ];
        for &(sym, this, n, pops) in stubs {
            t.stub(sym, this, n, pops);
        }
        let sprintf = t.stub_code(0xc0de_0601, false, 2, 0);
        t.g32(0x00b0_c3dc, sprintf);
        let dialog = t.random(&mut rng, 0x300);
        t.g32(0x00cc_c870, dialog);
        for i in 0..40 {
            let w = format!("{name} {i}");
            for &(sym, ..) in stubs {
                let v = rng.next();
                t.ret(sym, v);
            }
            let v = rng.next();
            t.ret(0xc0de_0601, v);
            let mgr = t.random(&mut rng, 0x800);
            let g = if rng.below(3) == 0 { 0 } else { mgr };
            t.g32(e::FMV_PLAYER_MANAGER.global, g);
            t.g32(ffx_sound::fmod::FMOD_MANAGER, rng.next());
            let x = rng.word();
            if g != 0 {
                t.check(
                    &format!("{w} fmvInitialize"),
                    0x006d_8f30,
                    Conv::Stdcall,
                    &[x],
                    |h| u(mv::fmv_initialize(h, x)),
                );
            }
            let obj = t.random(&mut rng, 0x400);
            let th = Conv::Thiscall(obj);
            let player = if rng.below(3) == 0 { 0 } else { rng.next() };
            t.w32(obj + 0x6c, player);
            t.g32(0x0198_4c6c, rng.next());
            t.check(&format!("{w} IggyMenu dtor"), 0x00a2_8a40, th, &[], |h| {
                mn::iggy_menu_dtor(h, obj);
                Out::None
            });
            for off in [0x114, 0x115, 0x116] {
                let b = [0, 1, rng.next() as u8][rng.below(3) as usize];
                t.w8(obj + off, b);
            }
            t.check(&format!("{w} unpause"), 0x0068_ff60, th, &[], |h| {
                mn::esc_unpause(h, obj);
                Out::None
            });
            let n = [0, 1, 4, 9][rng.below(4) as usize];
            let src = t.random(&mut rng, 0x40);
            let block = if rng.below(4) == 0 { 0 } else { t.zeros(0x40) };
            t.ret(mn::PHYRE_MALLOC, block);
            for sym in [0x006a_ffc0, 0x006b_0010] {
                t.check(
                    &format!("{w} copyNew {sym:#x}"),
                    sym,
                    Conv::Cdecl,
                    &[n, src],
                    |h| u(mn::parray_copy_new(h, n, src)),
                );
            }
            for off in [4, 5] {
                let b = [0, 1, rng.next() as u8][rng.below(3) as usize];
                t.w8(obj + off, b);
            }
            // the text the real TextUtil_setPosition writes (it leaves eax)
            let text = t.random(&mut rng, 0x100);
            t.w32(obj, text);
            let (sw, sh) = (rng.word(), rng.word());
            t.check(
                &format!("{w} videoTextSize"),
                0x006d_ded0,
                th,
                &[sw, sh],
                |h| u(mn::video_text_set_size(h, obj, sw, sh)),
            );
            let id = if rng.below(3) == 0 {
                0
            } else {
                t.random(&mut rng, 4)
            };
            t.check(
                &format!("{w} dialogText"),
                0x0068_e560,
                Conv::Cdecl,
                &[id],
                |h| u(mn::esc_dialog_text(h, id)),
            );
            t.g32(0x00ce_b248, rng.next());
            t.check(
                &format!("{w} showPopUp"),
                0x0070_2e90,
                Conv::Cdecl,
                &[x],
                |h| u(mn::save_show_popup(h, x)),
            );
        }
        // the manager missing: the message, the global read again
        let created = t.zeros(0x800);
        t.stub_store(
            e::PHYRE_PRINTF,
            false,
            e::FMV_PLAYER_MANAGER.global,
            created,
            0,
        );
        t.g32(e::FMV_PLAYER_MANAGER.global, 0);
        t.check(
            &format!("{name} fmvInitialize created"),
            0x006d_8f30,
            Conv::Stdcall,
            &[7],
            |h| u(mv::fmv_initialize(h, 7)),
        );
    }
}

/// The post effects' reset, initialize, update and resize, the shader
/// parameter writes, the scene cursor.
#[test]
fn effects_lifecycle() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_0303);
        let stubs: &[(u32, bool, u32, u32)] = &[
            (e::PIECE_RELEASE, true, 0, 0),
            (e::PARAM_WRITE, false, 3, 0),
            (e::TARGET_RESIZE, false, 4, 0),
            (e::RADIAL_BLUR_RESIZE, true, 2, 2),
            (e::RADIAL_LINE_RESIZE, true, 2, 2),
            (e::BLUR_EX_POST_INITIALIZE, true, 0, 0),
            (e::BLUR_IS_ACTIVATED, true, 0, 0),
            (e::PHYRE_PRINTF, false, 2, 0),
            (m::CURSOR_BEGIN, true, 0, 0),
        ];
        for &(sym, this, n, pops) in stubs {
            t.stub(sym, this, n, pops);
        }
        let blur = t.random(&mut rng, 0x40);
        for i in 0..40 {
            let w = format!("{name} {i}");
            for &(sym, ..) in stubs {
                let v = rng.next();
                t.ret(sym, v);
            }
            let flag = (rng.next() & !0xff) | [0, 1][rng.below(2) as usize];
            t.ret(e::BLUR_IS_ACTIVATED, flag);
            let posted = [0, 1][rng.below(2) as usize];
            t.ret(e::BLUR_EX_POST_INITIALIZE, posted);
            let obj = t.random(&mut rng, 0x100);
            let th = Conv::Thiscall(obj);
            // the broken screen's pieces: entries with halves, some null
            let n = [0, 1, 3, 6, u32::MAX][rng.below(5) as usize];
            let table = t.zeros(0x20);
            for k in 0..6 {
                let entry = if rng.below(4) == 0 { 0 } else { t.zeros(8) };
                if entry != 0 {
                    for half in [0, 4] {
                        let v = if rng.below(3) == 0 { 0 } else { rng.next() };
                        t.w32(entry + half, v);
                    }
                }
                t.w32(table + 4 * k, entry);
            }
            t.w32(obj + 0x34, n);
            t.w32(obj + 0x38, if rng.below(4) == 0 { 0 } else { table });
            t.check(&format!("{w} reset"), 0x006d_5970, th, &[], |g| {
                e::broken_screen_reset(g, obj);
                Out::None
            });
            let params = if rng.below(4) == 0 { 0 } else { t.zeros(8) };
            if params != 0 {
                let items = t.random(&mut rng, 0x420);
                t.w32(params, [0, 1, 5, 0x8000_0003, 0x103][rng.below(5) as usize]);
                t.w32(params + 4, items);
            }
            let (a, b) = (rng.word(), rng.word());
            t.check(
                &format!("{w} paramWrite"),
                0x0064_26e0,
                Conv::Cdecl,
                &[params, a, b],
                |g| u(e::shader_param_write(g, params, a, b)),
            );
            // a null BlurEffect would fault in both (its fields are read)
            t.g32(e::BLUR_EFFECT.global, blur);
            let args: [u32; 5] = std::array::from_fn(|_| rng.word());
            let old = rng.next();
            t.w32(obj + 0x38, old);
            t.check(&format!("{w} blurEx init"), 0x006c_4b00, th, &args, |g| {
                u(e::blur_ex_initialize(g, obj, args[0], args[3], args[4]))
            });
            t.w32(obj + 0x10, if rng.below(2) == 0 { 0 } else { rng.next() });
            t.w8(obj + 0x28, [0, 1][rng.below(2) as usize]);
            t.check(&format!("{w} blurEx update"), 0x006c_4e00, th, &[a], |g| {
                u(e::blur_ex_update(g, obj))
            });
            t.check(&format!("{w} resizeAll"), 0x006c_3c40, th, &[a, b], |g| {
                u(e::radial_blur_resize_all(g, obj, a, b))
            });
            let world = t.random(&mut rng, 0x40);
            if rng.below(3) == 0 {
                t.w32(world + 0x1c, world + 0x1c);
            }
            let cursor = t.random(&mut rng, 0x40);
            t.check(
                &format!("{w} cursor ctor"),
                0x0063_3880,
                Conv::Thiscall(cursor),
                &[world],
                |g| u(m::scene_cursor_ctor(g, cursor, world)),
            );
        }
    }
}

// ---- batch 5

/// `SfxEvent` volumes, FMOD event info, the shader-parameter copies, the
/// broken screen's light parameters.
#[test]
fn volumes_params_light() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_0401);
        t.stub(s::EVENT_SET_VOLUME, false, 2, 2);
        t.stub(s::ERRCHECK, false, 3, 0);
        t.stub(e::MEMCPY, false, 3, 0);
        for i in 0..40 {
            let w = format!("{name} {i}");
            for tag in [s::EVENT_SET_VOLUME, s::ERRCHECK, e::MEMCPY] {
                let v = rng.next();
                t.ret(tag, v);
            }
            let list = t.random(&mut rng, 0x30);
            let events = t.zeros(0x40);
            for k in 0..8 {
                let ev = rng.next();
                t.w32(events + 8 * k, ev);
                let on = if rng.below(3) == 0 {
                    0
                } else {
                    rng.next() as u8 | 1
                };
                t.w8(events + 8 * k + 4, on);
            }
            t.w32(list, [0, 1, 4, 8, u32::MAX][rng.below(5) as usize]);
            t.w32(list + 8, events);
            let scale = rng.float();
            t.w32(list + 0x24, scale);
            let vol = rng.float();
            t.check(
                &format!("{w} setVolume"),
                0x0070_f140,
                Conv::Thiscall(list),
                &[vol],
                |g| {
                    s::sfx_event_set_volume(g, list, vol);
                    Out::None
                },
            );
            let info = t.random(&mut rng, 0x40);
            t.check(
                &format!("{w} eventInfo"),
                0x0070_a140,
                Conv::Thiscall(info),
                &[],
                |g| u(s::fmod_event_info_init(g, info)),
            );
            let buffer = t.zeros(8);
            t.w32(
                buffer,
                [0, 3, 4, 0x20, 0x100, rng.next()][rng.below(6) as usize],
            );
            let param = t.random(&mut rng, 0x10);
            let off = [0u16, 3, 4, 8, 0x1c, rng.next() as u16][rng.below(6) as usize];
            let size = [0u16, 4, 0x10, 0x2010, 0xe004, rng.next() as u16][rng.below(6) as usize];
            t.write(param + 8, &off.to_le_bytes());
            t.write(param + 0xa, &size.to_le_bytes());
            let src = if rng.below(5) == 0 { 0 } else { rng.next() };
            for sym in [0x0063_86c0, 0x0063_8720, 0x0063_8780, 0x0069_8960] {
                t.check(
                    &format!("{w} paramCopy {sym:#x}"),
                    sym,
                    Conv::Thiscall(buffer),
                    &[param, src],
                    |g| u(e::shader_param_copy(g, buffer, param, src)),
                );
            }
            let obj = t.random(&mut rng, 0x40);
            let p: [u32; 7] = std::array::from_fn(|_| {
                if rng.below(3) == 0 {
                    0
                } else {
                    let at = t.zeros(4);
                    let f = rng.float();
                    t.w32(at, f);
                    at
                }
            });
            t.check(
                &format!("{w} light"),
                0x006d_5a70,
                Conv::Thiscall(obj),
                &p,
                |g| u(e::broken_screen_update_light(g, obj, p)),
            );
        }
    }
}

impl Twins {
    /// A stand-in at the import slot `slot` (stdcall, `pops` words) that
    /// returns 0 for its first `fails` calls, then runs `then` (a stub's
    /// code: logs and returns its cell).
    fn failing_then(&mut self, slot: u32, fails: u32, pops: u32, then: u32) {
        let cnt = self.zeros(4);
        self.w32(cnt, fails);
        let mut c = vec![0xff, 0x0d];
        c.extend(cnt.to_le_bytes()); // dec dword [cnt]
        c.extend([0x78, 0x05]); // js +5
        c.extend([0x31, 0xc0]); // xor eax, eax
        c.push(0xc2);
        c.extend(((pops * 4) as u16).to_le_bytes()); // ret pops*4
        c.push(0xe9);
        let at = self.cave_next + c.len() as u32 + 4;
        c.extend(then.wrapping_sub(at).to_le_bytes()); // jmp then
        let f = self.code(&c);
        let s = self.rebase(slot);
        self.w32(s, f);
    }
}

/// Iggy calls, the viewport rect, the language suffix, the audio preview
/// callbacks, the Iggy heap.
#[test]
fn iggy_calls_and_heap() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_0402);
        t.stub(mn::APPLY_VIEWPORT, true, 0, 0);
        t.stub(mn::LOCALIZATION, false, 0, 0);
        t.stub(mn::DEREF, true, 0, 0);
        t.stub(mn::ADD_CALLBACK_MENU, true, 2, 2);
        let call_fn = t.stub_code(0xc0de_0701, false, 5, 5);
        t.g32(0x00b0_c854, call_fn);
        let sleep = t.stub_code(0xc0de_0702, false, 1, 1);
        t.g32(0x00b0_c100, sleep);
        let path = t.stub_code(0xc0de_0703, false, 1, 1);
        t.g32(0x00b0_c818, path);
        let set_string = t.stub_code(0xc0de_0704, false, 5, 5);
        t.g32(0x00b0_c81c, set_string);
        let fast = t.stub_code(0xc0de_0705, false, 3, 3);
        let loc = t.random(&mut rng, 0x10);
        for i in 0..30 {
            let w = format!("{name} {i}");
            for tag in [
                mn::APPLY_VIEWPORT,
                mn::DEREF,
                mn::ADD_CALLBACK_MENU,
                0xc0de_0701,
                0xc0de_0702,
                0xc0de_0703,
                0xc0de_0704,
            ] {
                let v = rng.next();
                t.ret(tag, v);
            }
            let v = rng.next() | 1;
            t.ret(0xc0de_0705, v);
            // the fast name fails 0..2 times first
            t.failing_then(0x00b0_c850, rng.below(3), 3, fast);
            let obj = t.random(&mut rng, 0x100);
            let holder = t.random(&mut rng, 4);
            t.w32(obj + 0x6c, holder);
            let text = t.alloc(b"showPopUpStd\0");
            let args = [rng.word(), text, rng.word(), rng.word()];
            t.check(
                &format!("{w} AS3"),
                0x00a2_8ba0,
                Conv::Thiscall(obj),
                &[args[0], args[1], args[2], args[3], 0],
                |g| u(mn::iggy_call_as3(g, obj, args)),
            );
            let r: [u32; 6] = std::array::from_fn(|_| rng.word());
            if rng.below(2) == 0 {
                for k in 0..6 {
                    t.w32(obj + 0x18 + 4 * k, r[k as usize]);
                }
                if rng.below(2) == 0 {
                    let k = rng.below(6);
                    t.w32(obj + 0x18 + 4 * k, r[k as usize] ^ 1);
                }
            }
            t.check(
                &format!("{w} viewportRect"),
                0x00a2_6bd0,
                Conv::Thiscall(obj),
                &r,
                |g| u(mn::iggy_set_viewport_rect(g, obj, r)),
            );
            t.w32(loc + 4, [9, 10, 0, 11, rng.next()][rng.below(5) as usize]);
            t.ret(mn::LOCALIZATION, loc);
            let cb = rng.next();
            t.check(
                &format!("{w} suffix"),
                0x006e_9510,
                Conv::Cdecl,
                &[cb],
                |g| u(mn::iggy_language_suffix(g, cb)),
            );
            t.check(
                &format!("{w} audioPreview"),
                0x006f_fa20,
                Conv::Thiscall(obj),
                &[],
                |g| u(mn::esc_register_audio_preview(g, obj)),
            );
            // the heap: a free ring with 0..3 blocks, the owners' used rings
            let heap = t.zeros(0x40);
            let free_ring = heap + 0x10;
            let owner = t.zeros(0x200);
            let used = owner + 0x160;
            let other = t.zeros(0x40);
            t.w32(used + 0x1c, other);
            t.w32(used + 0x20, other);
            t.w32(other + 0x1c, used);
            t.w32(other + 0x20, used);
            let blocks: Vec<u32> = (0..rng.below(4))
                .map(|_| t.random(&mut rng, 0x40))
                .collect();
            let mut prev = free_ring;
            for &b in &blocks {
                t.w32(prev + 0x1c, b);
                t.w32(b + 0x20, prev);
                t.w32(b + 0x18, owner);
                prev = b;
            }
            t.w32(prev + 0x1c, free_ring);
            t.w32(free_ring + 0x20, prev);
            t.check(
                &format!("{w} heapTake"),
                0x00a3_9a30,
                Conv::Cdecl,
                &[heap],
                |g| u(mn::iggy_heap_take_free(g, heap)),
            );
        }
    }
}

/// The PS2 accumulation frames.
#[test]
fn accumulation_frames() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_0403);
        let stubs: &[(u32, bool, u32, u32)] = &[
            (m::GET_SPAD_PTR, false, 0, 0),
            (m::GS_SET_PACKET, false, 1, 0),
            (m::GS_START_VIF, false, 0, 0),
            (m::GS_TEST, false, 8, 0),
            (m::GS_ZBUF, false, 1, 0),
            (m::ACC_ALPHA, false, 1, 0),
            (m::GS_END2, false, 0, 0),
            (m::SEND_SPAD, false, 1, 0),
            (m::GS_FRAME_16, false, 2, 0),
            (m::GS_FRAME2_16, false, 2, 0),
            (m::DRAW_SET_FRAME, false, 0, 0),
        ];
        for &(sym, this, n, pops) in stubs {
            t.stub(sym, this, n, pops);
        }
        for i in 0..20 {
            for &(sym, ..) in stubs {
                let v = rng.next();
                t.ret(sym, v);
            }
            let on = [0, 1, rng.next() as u8][rng.below(3) as usize];
            t.write(t.rebase(0x0130_0776), &[on]);
            let on2 = [0, 1, rng.next() as u8][rng.below(3) as usize];
            t.write(t.rebase(0x0130_0778), &[on2]);
            t.check(
                &format!("{name} {i} frameMap {on}"),
                0x0082_bc50,
                Conv::Cdecl,
                &[],
                |g| u(m::acc_set_frame_map(g)),
            );
            t.check(
                &format!("{name} {i} frameChr {on2}"),
                0x0082_be20,
                Conv::Cdecl,
                &[],
                |g| u(m::acc_set_frame_chr(g)),
            );
        }
    }
}

// ---- batch 6

/// `SfxEvent`'s 0..127 volume and pan, the FMV manager's creation, the
/// escape menu's pause.
#[test]
fn sfx_scales_fmv_pause() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_0501);
        let stubs: &[(u32, bool, u32, u32)] = &[
            (s::APPLY_VOLUME, true, 1, 1),
            (s::APPLY_PAN, true, 1, 1),
            (mv::OPERATOR_NEW, false, 1, 0),
            (mv::FMV_CTOR, true, 0, 0),
            (mn::SFX_IS_PAUSED, false, 0, 0),
            (mn::VOICE_IS_PAUSED, false, 0, 0),
            (mn::IS_PLAYING_VIDEO, false, 0, 0),
            (mn::SFX_PAUSE, false, 1, 0),
            (mn::VOICE_SET_PAUSED, false, 1, 0),
            (mn::VIDEO_PAUSE, false, 0, 0),
        ];
        for &(sym, this, n, pops) in stubs {
            t.stub(sym, this, n, pops);
        }
        let mut values = vec![
            0u32,
            1,
            63,
            64,
            126,
            127,
            128,
            200,
            u32::MAX,
            0x8000_0000,
            0x7fff_ffff,
            0xffff_ff81,
            0xffff_ff80,
        ];
        values.extend((0..40).map(|_| rng.word()));
        for (i, v) in values.into_iter().enumerate() {
            let w = format!("{name} {i}");
            for &(sym, ..) in stubs {
                let r = rng.next();
                t.ret(sym, r);
            }
            for sym in [mn::SFX_IS_PAUSED, mn::VOICE_IS_PAUSED, mn::IS_PLAYING_VIDEO] {
                let r = if rng.below(2) == 0 { 0 } else { rng.next() };
                t.ret(sym, r);
            }
            let event = rng.next();
            t.check(
                &format!("{w} setVolume {v:#x}"),
                0x0070_f1a0,
                Conv::Thiscall(event),
                &[v],
                |g| u(s::sfx_event_set_volume_127(g, event, v)),
            );
            t.check(
                &format!("{w} setPan {v:#x}"),
                0x0070_ef80,
                Conv::Thiscall(event),
                &[v],
                |g| u(s::sfx_event_set_pan(g, event, v)),
            );
            let g0 = if rng.below(2) == 0 { 0 } else { rng.next() };
            t.g32(e::FMV_PLAYER_MANAGER.global, g0);
            let p = if rng.below(4) == 0 { 0 } else { rng.next() };
            t.ret(mv::OPERATOR_NEW, p);
            t.check(
                &format!("{w} CreateInstance"),
                0x006d_7130,
                Conv::Cdecl,
                &[],
                |g| {
                    mv::fmv_create_instance(g);
                    Out::None
                },
            );
            let obj = t.random(&mut rng, 0x200);
            t.check(
                &format!("{w} pause"),
                0x0068_f940,
                Conv::Thiscall(obj),
                &[],
                |g| u(mn::esc_pause(g, obj)),
            );
        }
    }
}

/// Array and character list clean-ups, the scene cursors, the second
/// save pop-up, the prototype lookup, the string move.
#[test]
fn lists_cursors_strings() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_0502);
        let stubs: &[(u32, bool, u32, u32)] = &[
            (mn::PHYRE_FREE, false, 1, 0),
            (mn::CURSOR_SET_NODE, true, 1, 1),
            (mn::SAVE_DATA_LOC, false, 2, 0),
            (mn::DATA_VALUE_SET, false, 2, 0),
            (mn::CALL_AS3, true, 5, 5),
        ];
        for &(sym, this, n, pops) in stubs {
            t.stub(sym, this, n, pops);
        }
        let vslot = t.stub_code(0xc0de_0801, true, 1, 1);
        let vtable = t.zeros(4);
        t.w32(vtable, vslot);
        let sprintf = t.stub_code(0xc0de_0802, false, 3, 0);
        t.g32(0x00b0_c3dc, sprintf);
        let wait = t.stub_code(0xc0de_0803, false, 2, 2);
        t.g32(0x00b0_c0f8, wait);
        let release = t.stub_code(0xc0de_0804, false, 1, 1);
        t.g32(0x00b0_c1c0, release);
        let memmove = t.stub_code(0xc0de_0805, false, 3, 0);
        t.g32(0x00b0_c34c, memmove);
        for i in 0..40 {
            let w = format!("{name} {i}");
            for &(sym, ..) in stubs {
                let r = rng.next();
                t.ret(sym, r);
            }
            for tag in 0xc0de_0801..=0xc0de_0805 {
                let r = rng.next();
                t.ret(tag, r);
            }
            // an array of objects
            let arr = t.zeros(8);
            let n = rng.below(4);
            let data = if rng.below(4) == 0 { 0 } else { t.zeros(0x100) };
            for k in 0..n {
                if data != 0 {
                    t.w32(data + 0x40 * k, vtable);
                }
            }
            t.w32(arr, n | [0, 0x8000_0000][rng.below(2) as usize]);
            t.w32(arr + 4, data);
            t.check(
                &format!("{w} destroyObjects"),
                0x006a_f630,
                Conv::Thiscall(arr),
                &[],
                |g| {
                    mn::parray_destroy_objects(g, arr);
                    Out::None
                },
            );
            // a scene cursor over a ring of nodes (link at +0) with the
            // list head at +0x14 and nodes the validity check accepts
            let cursor = t.random(&mut rng, 0x20);
            let list = t.zeros(0x50);
            let count = 1 + rng.below(4);
            let nodes: Vec<u32> = (0..count).map(|_| t.zeros(0x50)).collect();
            for k in 0..nodes.len() {
                let next = if k + 1 < nodes.len() {
                    nodes[k + 1]
                } else {
                    list
                };
                t.w32(nodes[k], next);
            }
            t.w32(list, nodes[0]);
            t.w32(cursor + 0x14, list);
            // the validity checks walk the list's chain at +0x2c/+0x40
            t.w32(list + 0x2c, if rng.below(3) == 0 { 0 } else { nodes[0] });
            for k in 0..nodes.len() {
                let next = if k + 1 < nodes.len() { nodes[k + 1] } else { 0 };
                t.w32(nodes[k] + 0x40, next);
            }
            let target = nodes[rng.below(count) as usize];
            t.w32(cursor + 0x1c, target);
            let start = [0, nodes[0], nodes[(count - 1) as usize]][rng.below(3) as usize];
            // sometimes the start is valid already: its chain (+0x2c)
            // holds the cursor's target and its +0x40 is set
            if start != 0 && rng.below(2) == 0 {
                t.w32(start + 0x2c, target);
                t.w32(start + 0x40, 1 + rng.below(8));
            }
            for (sym, check) in [
                (0x006a_2260, 0x006a_29c0),
                (0x006f_6300, 0x006f_63b0),
                (0x006a_22d0, 0x006a_29c0),
            ] {
                t.w32(cursor + 0x18, start);
                t.check(
                    &format!("{w} cursor {sym:#x}"),
                    sym,
                    Conv::Thiscall(cursor),
                    &[],
                    |g| {
                        if sym == 0x006a_22d0 {
                            u(mn::cursor_next(g, cursor, check))
                        } else {
                            u(mn::cursor_begin(g, cursor, check))
                        }
                    },
                );
            }
            let (a, b) = (rng.word(), rng.word());
            t.g32(0x00ce_b248, rng.next());
            t.check(
                &format!("{w} popup2"),
                0x0070_2d20,
                Conv::Cdecl,
                &[a, b],
                |g| u(mn::save_show_popup2(g, a, b)),
            );
            // a character vector
            let chars = t.random(&mut rng, 0x20);
            let len = rng.below(9);
            let holder = t.zeros(0x10);
            t.w32(holder + 8, chars);
            t.w32(holder + 0xc, chars + 4 * len);
            let chr = if len > 0 && rng.below(2) == 0 {
                Mem::read(&t.b, chars + 4 * rng.below(len))
            } else {
                rng.next()
            };
            t.check(
                &format!("{w} removeInstance"),
                0x006a_2b90,
                Conv::Thiscall(holder),
                &[chr],
                |g| u(mn::character_remove_instance(g, holder, chr)),
            );
            // std::string moves: short and long
            let src = t.random(&mut rng, 0x20);
            let cap = [0xf, 0x10, 0x20, rng.next()][rng.below(4) as usize];
            t.w32(src + 0x14, cap);
            t.w32(src + 0x10, [0, 5, 15, u32::MAX][rng.below(4) as usize]);
            let dst = t.random(&mut rng, 0x20);
            let unused = rng.next();
            t.check(
                &format!("{w} string move"),
                0x0069_c270,
                Conv::Thiscall(dst),
                &[src, unused],
                |g| u(mn::string_move_ctor(g, dst, src)),
            );
        }
        // the prototype table: a small red-black-like tree of nil-marked nodes
        for i in 0..30 {
            let head = t.zeros(0x30);
            t.w8(head + 0xd, 1);
            let n = rng.below(6);
            let nodes: Vec<u32> = (0..n).map(|_| t.random(&mut rng, 0x30)).collect();
            // a right-leaning chain: each node's right child the next
            for (k, &nd) in nodes.iter().enumerate() {
                t.w8(nd + 0xd, 0);
                t.w32(nd, head);
                let parent = if k == 0 { head } else { nodes[k - 1] };
                t.w32(nd + 4, parent);
                let right = if k + 1 < nodes.len() {
                    nodes[k + 1]
                } else {
                    head
                };
                t.w32(nd + 8, right);
                t.w32(nd + 0x28, rng.below(8));
            }
            t.w32(head, if nodes.is_empty() { head } else { nodes[0] });
            let table = t.zeros(4);
            t.w32(table, head);
            let id = rng.below(8);
            let ra = t.check(
                &format!("{name} {i} existPrototype"),
                0x006a_1e80,
                Conv::Thiscall(table),
                &[id],
                |_| Out::None,
            );
            assert_eq!(
                ra & 0xff,
                u32::from(mn::exist_prototype(&G(&mut t.b), table, id)),
                "{name} {i} existPrototype al"
            );
        }
    }
}

/// PS2 frame and store helpers, the particle fill buffers.
#[test]
fn ps2_bg_store_fill() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_0503);
        let stubs: &[(u32, bool, u32, u32)] = &[
            (m::GET_SPAD_PTR, false, 0, 0),
            (m::GS_SET_PACKET, false, 1, 0),
            (m::GS_START_VIF, false, 0, 0),
            (m::GS_END2, false, 0, 0),
            (m::SEND_SPAD, false, 1, 0),
            (m::GS_FRAME_16, false, 2, 0),
            (m::GS_FRAME2_16, false, 2, 0),
            (m::DRAW_SET_FRAME, false, 0, 0),
            (m::OPERATOR_NEW_2, false, 1, 0),
            (m::GS_SET_DEF_STORE_IMAGE, false, 8, 0),
            (m::GS_SYNC_PATH, false, 2, 0),
            // FlushCache stays the bare `ret` it is: it leaves eax
            (m::GS_EXEC_STORE_IMAGE, false, 2, 0),
        ];
        for &(sym, this, n, pops) in stubs {
            t.stub(sym, this, n, pops);
        }
        for i in 0..20 {
            for &(sym, ..) in stubs {
                let r = rng.next();
                t.ret(sym, r);
            }
            let on = [0, 1, rng.next() as u8][rng.below(3) as usize];
            t.write(t.rebase(0x0130_0777), &[on]);
            t.check(
                &format!("{name} {i} drawBG"),
                0x0082_be80,
                Conv::Cdecl,
                &[],
                |g| u(m::acc_draw_bg(g)),
            );
            t.check(
                &format!("{name} {i} fillBuffers"),
                0x0072_8750,
                Conv::Cdecl,
                &[],
                |g| u(m::init_fill_buffer_request(g)),
            );
            let rect = t.random(&mut rng, 0x10);
            let dst = rng.next();
            t.check(
                &format!("{name} {i} gsStore"),
                0x007e_4540,
                Conv::Cdecl,
                &[rect, dst],
                |g| u(m::gs_store(g, rect, dst)),
            );
        }
    }
}
