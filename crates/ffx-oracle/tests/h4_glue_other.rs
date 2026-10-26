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
use ffx_glue_other::{
    effect as e, frames as fr, h4_c1 as c1, h4_c2 as c2, h4_c3 as c3, h4_c4 as c4, h4_d1 as d1,
    menu as mn, misc as m, movie as mv, r3, sound as s,
};
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
    /// `al` (the rest of `eax` is the caller's).
    U8(u8),
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
            Out::U8(v) => assert_eq!(ra.eax & 0xff, u32::from(v), "{what}: al"),
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

// ---- batch 7

/// A binary search tree over sorted `keys` (`nodes[lo..hi]`, the middle
/// one the root; leaves link to `head`); its root.
fn build(t: &mut Twins, nodes: &[u32], keys: &[i32], head: u32, lo: usize, hi: usize) -> u32 {
    if lo >= hi {
        return head;
    }
    let mid = usize::midpoint(lo, hi);
    let nd = nodes[mid];
    t.w8(nd + 0xd, 0);
    t.w32(nd + 0x10, keys[mid] as u32);
    let l = build(t, nodes, keys, head, lo, mid);
    let r = build(t, nodes, keys, head, mid + 1, hi);
    t.w32(nd, l);
    t.w32(nd + 8, r);
    nd
}

/// The sfx id array's reserve, `FmodMusic`'s and the button remap's
/// constructors, the movie names.
#[test]
fn reserve_ctors_names() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_0601);
        let stubs: &[(u32, bool, u32, u32)] = &[
            (s::USER_MALLOC, false, 1, 0),
            (s::USER_FREE, false, 1, 0),
            (e::MEMCPY, false, 3, 0),
            (e::MEMSET, false, 3, 0),
            (mn::REMAP_INIT_MESSAGE, true, 0, 0),
            (mn::REMAP_INIT_UI, true, 0, 0),
        ];
        for &(sym, this, n, pops) in stubs {
            t.stub(sym, this, n, pops);
        }
        let sprintf_s = t.stub_code(0xc0de_0901, false, 4, 0);
        t.g32(0x00b0_c394, sprintf_s);
        let mut found = 0u32;
        for i in 0..40 {
            let w = format!("{name} {i}");
            for &(sym, ..) in stubs {
                let r = rng.next();
                t.ret(sym, r);
            }
            let r = rng.next();
            t.ret(0xc0de_0901, r);
            let malloc = if rng.below(4) == 0 { 0 } else { rng.next() };
            t.ret(s::USER_MALLOC, malloc);
            let arr = t.random(&mut rng, 0x10);
            if rng.below(3) == 0 {
                t.w32(arr + 8, 0);
            }
            let n = [0, 1, 4, u32::MAX, 0x8000_0000, rng.below(64)][rng.below(6) as usize];
            t.check(
                &format!("{w} reserve {n:#x}"),
                0x0070_d3a0,
                Conv::Thiscall(arr),
                &[n],
                |g| u(s::sfx_id_array_reserve(g, arr, n)),
            );
            let obj = t.random(&mut rng, 0x200);
            t.check(
                &format!("{w} FmodMusic"),
                0x0070_8630,
                Conv::Thiscall(obj),
                &[],
                |g| u(s::fmod_music_ctor(g, obj)),
            );
            t.check(
                &format!("{w} RemapButtonIcon"),
                0x006e_6330,
                Conv::Thiscall(obj),
                &[],
                |g| u(mn::remap_button_icon_ctor(g, obj)),
            );
            // the movie map: a head, a balanced tree of 0..6 keyed nodes
            let head = t.zeros(0x40);
            t.w8(head + 0xd, 1);
            let n_nodes = rng.below(7);
            let mut keys: Vec<i32> = (0..n_nodes).map(|_| rng.below(20) as i32 - 5).collect();
            keys.sort_unstable();
            keys.dedup();
            let nodes: Vec<u32> = keys.iter().map(|_| t.random(&mut rng, 0x40)).collect();
            let root = build(&mut t, &nodes, &keys, head, 0, nodes.len());
            t.w32(head + 4, root);
            // short and long names
            for &nd in &nodes {
                let cap = [0xf, 0x10, 0x20][rng.below(3) as usize];
                t.w32(nd + 0x14 + 0x14, cap);
            }
            let mgr = t.zeros(0x700);
            t.w32(mgr + 0x6e8, head);
            let id = (rng.below(24) as i32 - 6) as u32;
            let out = rng.next();
            // the id is one of the keys half the time
            let id = if keys.is_empty() || rng.below(2) == 0 {
                id
            } else {
                keys[rng.below(keys.len() as u32) as usize] as u32
            };
            t.check(
                &format!("{w} videoName {id:#x}"),
                0x006d_93e0,
                Conv::Thiscall(mgr),
                &[id, out],
                |g| {
                    let r = mv::fmv_video_name(g, mgr, id, out);
                    found += u32::from(r);
                    Out::U8(r)
                },
            );
        }
        assert!(found >= 10, "{name}: names found {found}");
    }
}

/// The Iggy heap's block move, the menu's VRAM block transfers.
#[test]
fn heap_move_vram_blocks() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_0602);
        let stubs: &[(u32, bool, u32, u32)] = &[
            (m::GS_SET_DEF_LOAD_IMAGE, false, 8, 0),
            (m::GS_SET_DEF_STORE_IMAGE, false, 8, 0),
            (m::GS_EXEC_LOAD_IMAGE, false, 2, 0),
            (m::GS_EXEC_STORE_IMAGE, false, 2, 0),
            (m::GS_SYNC_PATH, false, 2, 0),
        ];
        for &(sym, this, n, pops) in stubs {
            t.stub(sym, this, n, pops);
        }
        for i in 0..40 {
            let w = format!("{name} {i}");
            for &(sym, ..) in stubs {
                let r = rng.next();
                t.ret(sym, r);
            }
            let (buffer, address) = (
                rng.next(),
                [0, 63, 64, u32::MAX, 0xffff_ffc0, rng.next()][rng.below(6) as usize],
            );
            let size =
                [0, 3, 4, 2047, 2048, u32::MAX, 0xffff_f000, rng.next()][rng.below(8) as usize];
            for (sym, load, frame) in [
                (0x008a_a710, true, false),
                (0x008a_ac10, false, false),
                (0x008a_acf0, false, true),
            ] {
                t.check(
                    &format!("{w} vram {sym:#x} {size:#x}"),
                    sym,
                    Conv::Cdecl,
                    &[buffer, address, size],
                    |g| u(m::menu_vram_block(g, load, frame, buffer, address, size)),
                );
            }
            // the heap: rings at heap + 0x10 + 56 k, a block in ring 0
            let heap = t.zeros(0x200);
            t.w32(heap, rng.next());
            for k in 0..4 {
                let block_ring = heap + 0x10 + 56 * k;
                t.w32(block_ring + 0x1c, block_ring);
                t.w32(block_ring + 0x20, block_ring);
            }
            let block = t.random(&mut rng, 0x40);
            t.w32(block + 0x18, heap);
            let ring0 = heap + 0x10;
            t.w32(ring0 + 0x1c, block);
            t.w32(ring0 + 0x20, block);
            t.w32(block + 0x1c, ring0);
            t.w32(block + 0x20, ring0);
            let (size, owner, kind) = (rng.next(), rng.next(), rng.below(4));
            t.check(
                &format!("{w} moveBlock {kind}"),
                0x00a3_9a90,
                Conv::Cdecl,
                &[block, size, owner, kind],
                |g| u(mn::iggy_heap_move_block(g, block, size, owner, kind)),
            );
        }
    }
}

/// The cluster slots.
#[test]
fn cluster_slots() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_0603);
        t.stub(m::OPERATOR_NEW, false, 1, 0);
        let iob = t.stub_code(0xc0de_0a01, false, 0, 0);
        let fprintf = t.stub_code(0xc0de_0a02, false, 2, 0);
        t.g32(0x00b0_c350, iob);
        t.g32(0x00b0_c38c, fprintf);
        let slots = t.zeros(0x6420);
        for i in 0..40 {
            for tag in [m::OPERATOR_NEW, 0xc0de_0a01, 0xc0de_0a02] {
                let r = rng.next();
                t.ret(tag, r);
            }
            // a few busy runs, the hint anywhere
            for _ in 0..4 {
                let at = rng.below(0x400);
                for k in at..(at + rng.below(6)).min(0x400) {
                    t.w8(slots + 0x601c + k, 1);
                }
            }
            let hint = [0, 0x3ff, 0x400, 0x401, rng.below(0x400)][rng.below(5) as usize];
            t.w32(slots + 0x641c, hint);
            t.check(
                &format!("{name} {i} take {hint:#x}"),
                0x0069_aba0,
                Conv::Thiscall(slots),
                &[],
                |g| u(mn::cluster_slots_take(g, slots)),
            );
        }
    }
}

/// `BlurEffectEx_CreateInstance`, the character cache's delayed release.
#[test]
fn blur_ex_create_and_release_step() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_0604);
        t.stub(e::BLUR_EFFECT.get, false, 0, 0);
        t.stub(e::PHYRE_PRINTF, false, 2, 0);
        t.stub(m::OPERATOR_NEW, false, 1, 0);
        t.stub(e::BLUR_EX_CTOR, true, 0, 0);
        t.stub(e::APPLY_DELAYED_RELEASE, true, 0, 0);
        let iob = t.stub_code(0xc0de_0b01, false, 0, 0);
        let fprintf = t.stub_code(0xc0de_0b02, false, 2, 0);
        t.g32(0x00b0_c350, iob);
        t.g32(0x00b0_c38c, fprintf);
        for i in 0..60 {
            let w = format!("{name} {i}");
            for tag in [
                e::PHYRE_PRINTF,
                e::BLUR_EX_CTOR,
                e::APPLY_DELAYED_RELEASE,
                0xc0de_0b01,
                0xc0de_0b02,
            ] {
                let r = rng.next();
                t.ret(tag, r);
            }
            let blur = if rng.below(3) == 0 { 0 } else { rng.next() };
            t.ret(e::BLUR_EFFECT.get, blur);
            let p = if rng.below(4) == 0 { 0 } else { rng.next() };
            t.ret(m::OPERATOR_NEW, p);
            t.g32(
                e::BLUR_EFFECT_EX.global,
                if rng.below(2) == 0 { 0 } else { rng.next() },
            );
            t.check(
                &format!("{w} BlurEx create"),
                0x006c_49a0,
                Conv::Cdecl,
                &[],
                |g| {
                    e::blur_ex_create_instance(g);
                    Out::None
                },
            );
            let timer = t.zeros(8);
            t.w32(timer, [0, 1, 2, u32::MAX][rng.below(4) as usize]);
            let time = [
                rng.float(),
                2.9f32.to_bits(),
                3.0f32.to_bits(),
                11.9f32.to_bits(),
                12.0f32.to_bits(),
                0x7fc0_0000,
            ][rng.below(6) as usize];
            t.w32(timer + 4, time);
            let dt = [0, 0x3c88_8889, 1.0f32.to_bits(), rng.float()][rng.below(4) as usize];
            t.check(
                &format!("{w} releaseStep"),
                0x0069_c010,
                Conv::Thiscall(timer),
                &[dt],
                |g| {
                    e::delay_release_step(g, timer, dt);
                    Out::None
                },
            );
        }
        // exactly at the limits: not past them
        for (state, time) in [(0u32, 12.0f32), (1, 3.0), (0, 11.0), (1, 2.0)] {
            let timer = t.zeros(8);
            t.w32(timer, state);
            t.w32(timer + 4, time.to_bits());
            let dt = if time < 3.5 || (11.5..12.0).contains(&time) {
                1.0f32.to_bits()
            } else {
                0
            };
            t.check(
                &format!("{name} release at {time}"),
                0x0069_c010,
                Conv::Thiscall(timer),
                &[dt],
                |g| {
                    e::delay_release_step(g, timer, dt);
                    Out::None
                },
            );
        }
    }
}

// ---- batch 8

/// FMOD voice clear, the post-effect parameter block, the third save
/// pop-up, the escape menu's resume, text scrolling, the MFIFO reset.
#[test]
fn voice_params_resume_scroll() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_0701);
        let stubs: &[(u32, bool, u32, u32)] = &[
            (s::ERRCHECK, false, 3, 0),
            (e::SAMPLER_STATE_CTOR, true, 0, 0),
            (mn::SAVE_DATA_LOC, false, 2, 0),
            (mn::DATA_VALUE_SET, false, 2, 0),
            (mn::DATA_VALUE_SET_BOOL, false, 2, 0),
            (mn::CALL_AS3, true, 5, 5),
            (m::IGGY_SET_ENABLE, true, 1, 1),
            (mn::SFX_PAUSE, false, 1, 0),
            (mn::VOICE_SET_PAUSED, false, 1, 0),
            (mn::VIDEO_PAUSE, false, 0, 0),
            (mn::SETTINGS_SAVE, true, 0, 0),
            (mn::SAVE_MENU_ENABLE, false, 1, 0),
            (mn::ESC_PAGE_REFRESH, true, 1, 1),
            (mn::TO_WORLD_3X4, true, 1, 1),
            (m::SET_GLOBALS_8431D0, false, 1, 0),
        ];
        for &(sym, this, n, pops) in stubs {
            t.stub(sym, this, n, pops);
        }
        let release = t.stub_code(0xc0de_0c01, false, 3, 3);
        let vtable = t.zeros(0x10);
        t.w32(vtable + 8, release);
        let show_cursor = t.stub_code(0xc0de_0c02, false, 1, 1);
        t.g32(0x00b0_c688, show_cursor);
        let dialog = t.random(&mut rng, 0x200);
        t.g32(0x00cc_c870, dialog);
        for i in 0..40 {
            let w = format!("{name} {i}");
            for &(sym, ..) in stubs {
                let r = rng.next();
                t.ret(sym, r);
            }
            let r = if rng.below(3) == 0 { 0 } else { rng.next() };
            t.ret(0xc0de_0c01, r);
            let v = rng.next();
            t.ret(0xc0de_0c02, v);
            let voice = t.random(&mut rng, 0x80);
            let group = t.zeros(4);
            t.w32(group, vtable);
            t.w32(voice + 0x10, if rng.below(4) == 0 { 0 } else { group });
            t.w32(voice + 0x14, if rng.below(4) == 0 { 0 } else { rng.next() });
            t.check(
                &format!("{w} voiceClear"),
                0x0070_a590,
                Conv::Thiscall(voice),
                &[],
                |g| u(s::fmod_voice_clear(g, voice)),
            );
            let params = t.random(&mut rng, 0x80);
            t.check(
                &format!("{w} params ctor"),
                0x0070_2ff0,
                Conv::Thiscall(params),
                &[],
                |g| u(e::post_effect_params_ctor(g, params)),
            );
            t.g32(0x00ce_b248, rng.next());
            let id = rng.word();
            t.check(
                &format!("{w} popup4"),
                0x0070_2ef0,
                Conv::Cdecl,
                &[id],
                |g| u(mn::save_show_popup4(g, id)),
            );
            let menu = t.random(&mut rng, 0x200);
            t.w8(menu + 0x130, [0, 1, 2][rng.below(3) as usize]);
            for off in [0x114, 0x115, 0x116] {
                let b = [0, 1, rng.next() as u8][rng.below(3) as usize];
                t.w8(dialog + off, b);
            }
            t.g32(0x022f_b504, rng.next());
            t.g32(0x00ce_7310, if rng.below(2) == 0 { 0 } else { rng.next() });
            t.check(
                &format!("{w} resume"),
                0x0068_fe60,
                Conv::Cdecl,
                &[menu],
                |g| {
                    mn::esc_resume(g, menu);
                    Out::None
                },
            );
            let text = t.zeros(0x80);
            for off in [0x50, 0x54, 0x58, 0x60, 0x64, 0x68] {
                let f = rng.float();
                t.w32(text + off, f);
            }
            t.w32(text + 0x10, if rng.below(3) == 0 { 0 } else { rng.next() });
            let steps =
                [0, 1, u32::MAX, 0x7fff_ffff, 0x0100_0001, rng.word()][rng.below(6) as usize];
            t.check(
                &format!("{w} scroll {steps:#x}"),
                0x006f_6860,
                Conv::Thiscall(text),
                &[steps],
                |g| {
                    mn::text_scroll(g, text, steps);
                    Out::None
                },
            );
            let sink = rng.next() & !0x100;
            t.g32(0x0230_56e8, sink);
            t.check(
                &format!("{w} mfifoReset"),
                0x0084_32e0,
                Conv::Cdecl,
                &[],
                |g| u(m::mfifo_reset(g)),
            );
        }
    }
}

// ---- round 2: originals that hand callees their own frame ----

/// What a [`Twins::stub_out`] stub writes before it logs: the word in
/// `cell` at `offset` past its `ecx` (`arg` `None`) or past its stack
/// argument `arg`.
struct Write {
    arg: Option<u8>,
    offset: u32,
    cell: u32,
}

impl Twins {
    /// A stub like [`Twins::stub_code`] that first stores cells through
    /// pointers it is handed (a callee filling the caller's locals).
    fn stub_out(&mut self, tag: u32, this: bool, n: u32, pops: u32, writes: &[Write]) -> u32 {
        let mut c = Vec::new();
        for w in writes {
            match w.arg {
                None => c.extend([0x89, 0xc8]),                     // mov eax, ecx
                Some(i) => c.extend([0x8b, 0x44, 0x24, 4 + 4 * i]), // mov eax, [esp+4+4i]
            }
            c.extend([0x8b, 0x15]);
            c.extend(w.cell.to_le_bytes()); // mov edx, [cell]
            c.extend([0x89, 0x90]);
            c.extend(w.offset.to_le_bytes()); // mov [eax+off], edx
        }
        let rest = self.stub_code(tag, this, n, pops);
        c.push(0xe9);
        let at = self.cave_next;
        c.extend(rest.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes()); // jmp rest
        self.code(&c)
    }

    /// [`Twins::stub_out`] jumped to from the original `sym`; its address.
    fn stub_at(&mut self, sym: u32, this: bool, n: u32, pops: u32, writes: &[Write]) -> u32 {
        let addr = self.stub_out(sym, this, n, pops, writes);
        self.jump(sym, addr);
        addr
    }

    /// A `jmp` from the original `sym` to `addr` (switching between a
    /// stub that writes and one that does not).
    fn jump(&mut self, sym: u32, addr: u32) {
        let at = self.rebase(sym);
        let mut j = vec![0xe9];
        j.extend(addr.wrapping_sub(at + 5).to_le_bytes());
        self.write(at, &j);
    }

    /// Where the original's `ebp` lands for a call with `words` stack
    /// arguments ([`Oracle::call`] pushes them from the top of the stack
    /// less `0x100`, then the return address; the original pushes `ebp`).
    fn ebp_for(&self, words: u32) -> u32 {
        assert_eq!(self.a.stack_range(), self.b.stack_range());
        self.a.stack_range().end - 0x100 - 4 * words - 8
    }

    /// [`Twins::check`] for an original whose frame (`frame`, `len` bytes)
    /// callees see: the port's oracle gets the same block, its own calls
    /// below it; afterwards the bytes of `compare` (offsets and lengths in
    /// the frame) must agree.
    #[allow(clippy::too_many_arguments)]
    fn check_framed(
        &mut self,
        what: &str,
        sym: u32,
        conv: Conv,
        args: &[u32],
        frame: u32,
        compare: &[(u32, usize)],
        port: impl FnOnce(&mut G) -> Out,
    ) -> u32 {
        // the same junk in both frames first: a word the port forgets to
        // write then differs
        for &(off, len) in compare {
            let junk: Vec<u8> = (0..len).map(|k| 0xa5 ^ k as u8).collect();
            self.write(frame + off, &junk);
        }
        self.b.set_call_top(Some(frame - 0x100));
        let eax = self.check(what, sym, conv, args, port);
        self.b.set_call_top(None);
        for &(off, len) in compare {
            let mut x = vec![0; len];
            let mut y = vec![0; len];
            self.a.read_bytes(frame + off, &mut x);
            self.b.read_bytes(frame + off, &mut y);
            assert_eq!(x, y, "{what}: frame +{off:#x}");
        }
        eax
    }
}

/// The `IggyMenu_callFlashFunction_<n>` family and
/// `IggyMenu_getArrayS32_00a28b20` (its value written over its own
/// argument).
#[test]
fn frames_iggy_flash_calls_and_array() {
    const FLASH: [u32; 5] = [
        0x0068_dd40,
        0x0068_dd80,
        0x0068_ddd0,
        0x0068_de50,
        0x0068_def0,
    ];
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_2b01);
        t.stub(mn::CALL_AS3, true, 5, 5);
        let value = t.zeros(4);
        let make = t.stub_code(fr::IGGY_PATH_MAKE_ARRAY_REF, false, 3, 3);
        let get = t.stub_out(
            fr::IGGY_VALUE_GET_S32,
            false,
            4,
            4,
            &[Write {
                arg: Some(3),
                offset: 0,
                cell: value,
            }],
        );
        t.g32(fr::IGGY_PATH_MAKE_ARRAY_REF, make);
        // a failing get leaves the argument slot as it was
        let get_quiet = t.stub_code(fr::IGGY_VALUE_GET_S32 + 1, false, 4, 4);
        for i in 0..16 {
            let w = format!("{name} {i}");
            for (k, &sym) in FLASH.iter().enumerate() {
                let n = k as u32 + 1;
                let words: Vec<u32> = (0..2 + 4 * n).map(|_| rng.word()).collect();
                let args = t.alloc(
                    &words
                        .iter()
                        .flat_map(|v| v.to_le_bytes())
                        .collect::<Vec<_>>(),
                );
                let r = rng.next();
                t.ret(mn::CALL_AS3, r);
                let frame = t.ebp_for(2 + 4 * n) - fr::flash_values_below_ebp(n);
                t.check_framed(
                    &format!("{w} callFlashFunction {n}"),
                    sym,
                    Conv::Cdecl,
                    &words,
                    frame,
                    &[(0, 16 * n as usize)],
                    |g| u(fr::iggy_call_flash(g, frame, n, args)),
                );
            }
            let this = rng.word();
            let x = rng.word();
            let v = rng.next();
            t.w32(value, v);
            let quiet = rng.below(3) == 0;
            t.g32(fr::IGGY_VALUE_GET_S32, if quiet { get_quiet } else { get });
            let frame = t.ebp_for(1) - 0x18;
            t.check_framed(
                &format!("{w} getArrayS32 (quiet {quiet})"),
                0x00a2_8b20,
                Conv::Thiscall(this),
                &[x],
                frame,
                &[(0x20, 4)],
                |g| u(fr::iggy_array_s32(g, frame, this, x)),
            );
        }
        assert!(t.log_entries() > 0);
    }
}

/// `FmodEventTree_getEvent_00710290`, `Distortion_resetRenderTarget_006c24b0`,
/// `PostProcessManager_findClusterData_00698a30`.
#[test]
fn frames_tree_event_distortion_cluster_cursor() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_2b02);
        // the event tree: a project whose vtable +0x24 fills the result
        let result = t.zeros(4);
        let method = t.stub_out(
            0x24,
            false,
            4,
            4,
            &[Write {
                arg: Some(3),
                offset: 0,
                cell: result,
            }],
        );
        let vtable = t.zeros(0x40);
        t.w32(vtable + 0x24, method);
        let project = t.zeros(0x10);
        t.w32(project, vtable);
        t.stub(fr::INT_TREE_FIND, true, 1, 1);
        // the distortion's callees
        t.stub(fr::RENDER_TARGET_RELEASE, true, 0, 0);
        t.stub(fr::SHADER_PARAMS_FIND, true, 1, 1);
        t.stub(fr::SHADER_PARAMS_SET, true, 2, 2);
        // the cursor's step fills +0, +0xc, +0x10
        let cells = [t.zeros(4), t.zeros(4), t.zeros(4)];
        let cursor_quiet = t.stub_code(fr::CLUSTER_CURSOR_NEXT + 1, true, 0, 0);
        let cursor = t.stub_at(
            fr::CLUSTER_CURSOR_NEXT,
            true,
            0,
            0,
            &[
                Write {
                    arg: None,
                    offset: 0,
                    cell: cells[0],
                },
                Write {
                    arg: None,
                    offset: 0xc,
                    cell: cells[1],
                },
                Write {
                    arg: None,
                    offset: 0x10,
                    cell: cells[2],
                },
            ],
        );
        for i in 0..24 {
            let w = format!("{name} {i}");
            // getEvent
            let tree = t.zeros(0x10);
            t.w32(tree, if rng.below(4) == 0 { 0 } else { project });
            let node = match rng.below(4) {
                0 => 0,
                1 => 0xffff_fff0,
                _ => {
                    let n = t.zeros(0x20);
                    t.w32(n + 0x10, rng.word());
                    n
                }
            };
            t.ret(fr::INT_TREE_FIND, node);
            let r = rng.next();
            t.w32(result, r);
            t.ret(0x24, rng.next());
            let key = rng.word();
            let frame = t.ebp_for(1) - 4;
            t.check_framed(
                &format!("{w} getEvent"),
                0x0071_0290,
                Conv::Thiscall(tree),
                &[key],
                frame,
                &[(0, 4)],
                |g| u(fr::fmod_tree_get_event(g, frame, tree, key)),
            );
            // resetRenderTarget
            let this = t.random(&mut rng, 0x80);
            let shader = if rng.below(3) == 0 { 0 } else { rng.word() };
            t.w32(this + 0x2c, shader);
            let owner = t.random(&mut rng, 8);
            t.w32(this + 0x30, owner);
            let found = if rng.below(3) == 0 { 0 } else { rng.next() };
            t.ret(fr::SHADER_PARAMS_FIND, found);
            for tag in [fr::RENDER_TARGET_RELEASE, fr::SHADER_PARAMS_SET] {
                let v = rng.next();
                t.ret(tag, v);
            }
            let frame = t.ebp_for(0) - fr::DISTORTION_FRAME;
            t.check_framed(
                &format!("{w} resetRenderTarget"),
                0x006c_24b0,
                Conv::Thiscall(this),
                &[],
                frame,
                &if found != 0 && shader != 0 {
                    vec![(0, 16)]
                } else {
                    vec![]
                },
                |g| u(fr::distortion_reset_target(g, frame, this)),
            );
            // findClusterData
            let k = rng.below(4);
            let manager = t.zeros(0x20);
            let cluster = match rng.below(4) {
                0 => 0,
                _ => {
                    let c = t.zeros(0x40);
                    let head = match rng.below(3) {
                        0 => c + 0x1c,
                        1 => 0,
                        _ => rng.word(),
                    };
                    t.w32(c + 0x1c, head);
                    c
                }
            };
            t.w32(manager + 4 + 4 * k, cluster);
            let quiet = rng.below(4) == 0;
            t.jump(
                fr::CLUSTER_CURSOR_NEXT,
                if quiet { cursor_quiet } else { cursor },
            );
            for &c in &cells {
                let v = if rng.below(3) == 0 { 0 } else { rng.next() };
                t.w32(c, v);
            }
            let frame = t.ebp_for(1) - fr::CLUSTER_CURSOR_FRAME;
            t.check_framed(
                &format!("{w} findClusterData"),
                0x0069_8a30,
                Conv::Thiscall(manager),
                &[k],
                frame,
                &if cluster == 0 {
                    vec![]
                } else {
                    vec![(0, 0x20)]
                },
                |g| u(fr::post_find_cluster_data(g, frame, manager, k)),
            );
        }
    }
}

/// `ScreenFadeDelegations_registerDelegation`, the two Iggy `GDraw` D3D11
/// helpers, `LocString_Load_0068e180`, `getFullFilename`,
/// `ClassLoadedCharacterTable_makeEntry_0069da20`.
#[test]
fn frames_fade_gdraw_strings() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_2b03);
        t.stub(fr::GET_APPLICATION, false, 0, 0);
        t.stub(fr::REGISTER_DELEGATION, true, 4, 4);
        // GDraw's context: Map fills pData
        let mapped = t.zeros(4);
        let vtable = t.zeros(0x80);
        let map = t.stub_out(
            0x38,
            false,
            6,
            6,
            &[Write {
                arg: Some(5),
                offset: 0,
                cell: mapped,
            }],
        );
        t.w32(vtable + 0x38, map);
        for (slot, n) in [(0x20, 4), (0x48, 6), (0x4c, 4)] {
            let f = t.stub_code(slot, false, n, n);
            t.w32(vtable + slot, f);
        }
        let ctx = t.zeros(8);
        t.w32(ctx, vtable);
        let state = t.zeros(8);
        t.w32(state + 4, ctx);
        t.g32(fr::GDRAW_STATE, state);
        // the out-of-memory warning passes 4 words, the other 5 (a stub
        // logging 5 would log the caller's stack junk after the 4)
        let warn4 = t.stub_code(fr::GDRAW_SEND_WARNING + 4, false, 4, 0);
        let warn5 = t.stub_code(fr::GDRAW_SEND_WARNING, false, 5, 0);
        // strings
        let text = t.zeros(4);
        t.stub_at(
            mn::LOC_TEXT,
            false,
            3,
            0,
            &[Write {
                arg: Some(1),
                offset: 0,
                cell: text,
            }],
        );
        t.stub(mn::TEXT_CONVERT, false, 3, 0);
        t.stub(fr::MEMSET, false, 3, 0);
        let dir = t.zeros(4);
        t.stub_at(
            fr::DATA_DIRECTORY,
            false,
            3,
            0,
            &[Write {
                arg: Some(1),
                offset: 4,
                cell: dir,
            }],
        );
        let sprintf = t.stub_code(0x00b0_c3dc, false, 4, 0);
        t.g32(0x00b0_c3dc, sprintf);
        let (cap, ptr) = (t.zeros(4), t.zeros(4));
        // an assign that keeps the empty string's capacity 0xf
        let assign_quiet = t.stub_code(fr::STRING_ASSIGN + 1, true, 3, 3);
        let assign = t.stub_at(
            fr::STRING_ASSIGN,
            true,
            3,
            3,
            &[
                Write {
                    arg: None,
                    offset: 0x14,
                    cell: cap,
                },
                Write {
                    arg: None,
                    offset: 0,
                    cell: ptr,
                },
            ],
        );
        t.stub(fr::STRING_MOVE_CTOR, true, 2, 2);
        t.stub(fr::OPERATOR_DELETE, false, 1, 0);
        for i in 0..24 {
            let w = format!("{name} {i}");
            for tag in [
                fr::GET_APPLICATION,
                fr::REGISTER_DELEGATION,
                0x20,
                0x48,
                0x4c,
                fr::GDRAW_SEND_WARNING,
                mn::LOC_TEXT,
                mn::TEXT_CONVERT,
                fr::MEMSET,
                fr::DATA_DIRECTORY,
                0x00b0_c3dc,
                fr::STRING_ASSIGN,
                fr::STRING_MOVE_CTOR,
            ] {
                let v = rng.next();
                t.ret(tag, v);
            }
            // registerDelegation
            let scene = t.zeros(0x100);
            t.w32(scene + 0xdc, if rng.below(3) == 0 { 0 } else { rng.word() });
            t.g32(fr::PHYRE_SCENE, scene);
            t.g32(fr::RENDER_DELEGATION_HELPER, rng.word());
            let colour = [rng.float(), rng.float(), rng.float(), rng.float()];
            let (x, y) = (rng.word(), rng.word());
            let args = [colour[0], colour[1], colour[2], colour[3], x, y];
            let frame = t.ebp_for(6) - fr::FADE_RECORD_FRAME;
            t.check_framed(
                &format!("{w} registerDelegation"),
                0x006a_e650,
                Conv::Cdecl,
                &args,
                frame,
                &[(0, 0x20)],
                |g| u(fr::screen_fade_register(g, frame, colour, x, y)),
            );
            // gdraw
            let hr = [0, 1, fr::E_OUTOFMEMORY, 0x8000_4005, rng.next()][rng.below(5) as usize];
            t.ret(0x38, hr);
            t.g32(
                fr::GDRAW_SEND_WARNING,
                if hr == fr::E_OUTOFMEMORY {
                    warn4
                } else {
                    warn5
                },
            );
            let p = rng.next();
            t.w32(mapped, p);
            let resource = rng.word();
            let frame = t.ebp_for(1) - fr::MAPPED_FRAME;
            t.check_framed(
                &format!("{w} mapBuffer {hr:#x}"),
                0x00a3_cec0,
                Conv::Cdecl,
                &[resource],
                frame,
                &[(0, 4)],
                |g| u(fr::gdraw_map_buffer(g, frame, resource)),
            );
            let frame = t.ebp_for(0) - fr::CLEAR_BINDINGS_FRAME;
            t.check_framed(
                &format!("{w} clearBindings"),
                0x00a3_d280,
                Conv::Cdecl,
                &[],
                frame,
                &[(0, 8)],
                |g| u(fr::gdraw_clear_bindings(g, frame)),
            );
            // LocString_Load
            let v = rng.next();
            t.w32(text, v);
            let (this, id) = (rng.word(), rng.word());
            let frame = t.ebp_for(1) - fr::LOC_STRING_FRAME;
            t.check_framed(
                &format!("{w} LocString_Load"),
                0x0068_e180,
                Conv::Thiscall(this),
                &[id],
                frame,
                &[(0, 4)],
                |g| u(fr::loc_string_load(g, frame, this, id)),
            );
            // getFullFilename
            let v = rng.next();
            t.w32(dir, v);
            let (out, file) = (rng.word(), rng.word());
            let frame = t.ebp_for(2) - fr::FULL_FILENAME_FRAME;
            t.check_framed(
                &format!("{w} getFullFilename"),
                0x0070_7aa0,
                Conv::Cdecl,
                &[out, file],
                frame,
                &[(0, 1), (4, 4)],
                |g| u(fr::full_filename(g, frame, out, file)),
            );
            // makeEntry
            let c = [0, 0xf, 0x10, 0x11, rng.next()][rng.below(5) as usize];
            t.w32(cap, c);
            let quiet = rng.below(4) == 0;
            t.jump(fr::STRING_ASSIGN, if quiet { assign_quiet } else { assign });
            let v = rng.next();
            t.w32(ptr, v);
            let value = t.random(&mut rng, 4);
            let (out, src) = (rng.word(), rng.word());
            let frame = t.ebp_for(3) - fr::PROTOTYPE_ENTRY_FRAME;
            t.check_framed(
                &format!("{w} makeEntry cap {c:#x}"),
                0x0069_da20,
                Conv::Cdecl,
                &[out, src, value],
                frame,
                &[(0, 8), (0x14, 0xc)],
                |g| u(fr::prototype_entry(g, frame, out, src, value)),
            );
        }
    }
}

/// Writes `words` at `at` in the port's oracle (where the original's
/// caller pushed them).
fn put_args(g: &mut G, at: u32, words: &[u32]) {
    let bytes: Vec<u8> = words.iter().flat_map(|v| v.to_le_bytes()).collect();
    Mem::write_bytes(g, at, &bytes);
}

/// `printToOutput` (varargs from the caller's arguments), then `ERRCHECK`
/// (its peak written over its own argument) with `printToOutput` stubbed.
#[test]
fn frames_errcheck_print() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_2b11);
        let vsprintf = t.stub_code(fr::VSPRINTF, false, 3, 0);
        t.g32(fr::VSPRINTF, vsprintf);
        let sprintf = t.stub_code(0x00b0_c3dc, false, 4, 0);
        t.g32(0x00b0_c3dc, sprintf);
        t.stub(fr::PHYRE_PRINTF, false, 2, 0);
        for i in 0..16 {
            let v = rng.next();
            t.ret(fr::PHYRE_PRINTF, v);
            let n = 2 + rng.below(5);
            let words: Vec<u32> = (0..n).map(|_| rng.word()).collect();
            let ebp = t.ebp_for(n);
            let frame = ebp - fr::PRINT_FRAME;
            t.check_framed(
                &format!("{name} {i} printToOutput {n}"),
                fr::PRINT_TO_OUTPUT,
                Conv::Cdecl,
                &words,
                frame,
                &[],
                |g| {
                    put_args(g, ebp + 8, &words);
                    u(fr::print_to_output(g, frame, ebp + 8))
                },
            );
        }
        t.stub(fr::FMOD_ERROR_STRING, false, 1, 0);
        // the print logs 6 words when only the error is printed, 3 when the
        // memory report (3 words) follows
        let print6 = t.stub_code(fr::PRINT_TO_OUTPUT, false, 6, 0);
        let print3 = t.stub_code(fr::PRINT_TO_OUTPUT + 1, false, 3, 0);
        let (current, peak) = (t.zeros(4), t.zeros(4));
        let stats = t.stub_out(
            fr::FMOD_MEMORY_GET_STATS,
            false,
            3,
            3,
            &[
                Write {
                    arg: Some(0),
                    offset: 0,
                    cell: current,
                },
                Write {
                    arg: Some(1),
                    offset: 0,
                    cell: peak,
                },
            ],
        );
        let stats_quiet = t.stub_code(fr::FMOD_MEMORY_GET_STATS + 1, false, 3, 3);
        for i in 0..32 {
            let w = format!("{name} {i}");
            let v = rng.next();
            t.ret(fr::FMOD_ERROR_STRING, v);
            let result = [0, 0, 1, rng.word()][rng.below(4) as usize];
            let known =
                [0, 1, u32::MAX, 0x7fff_ffff, 0x8000_0000, rng.next()][rng.below(6) as usize];
            let p = match rng.below(4) {
                0 => known,
                1 => known.wrapping_add(1),
                2 => known.wrapping_sub(1),
                _ => rng.next(),
            };
            t.w32(peak, p);
            let c = rng.next();
            t.w32(current, c);
            t.g32(fr::FMOD_MAX_MEMORY, known);
            let quiet = rng.below(5) == 0;
            t.jump(
                fr::FMOD_MEMORY_GET_STATS,
                if quiet { stats_quiet } else { stats },
            );
            let reported = if quiet { 0 } else { p };
            let both = (reported as i32) > (known as i32);
            t.jump(fr::PRINT_TO_OUTPUT, if both { print3 } else { print6 });
            let args = [result, rng.word(), rng.word()];
            let ebp = t.ebp_for(3);
            t.check_framed(
                &format!("{w} ERRCHECK {result:#x} peak {p:#x} known {known:#x} quiet {quiet}"),
                fr::ERRCHECK,
                Conv::Cdecl,
                &args,
                ebp - 4,
                &[(0, 4), (0xc, 12)],
                |g| {
                    put_args(g, ebp + 8, &args);
                    u(fr::errcheck(g, ebp - 4, ebp + 8))
                },
            );
        }
    }
}

/// `SfxBank_getNumPlayingEvents`, `FmodVoice_volumeDirect`,
/// `FmodSfx_isPlaying`, `FmodShout_setVolume`.
#[test]
fn frames_fmod_bank_voice_sfx_shout() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_2b12);
        t.stub(fr::ERRCHECK, false, 3, 0);
        // the bank's info method fills the playing count
        let playing = t.zeros(4);
        let info = t.stub_out(
            0x104,
            false,
            2,
            2,
            &[Write {
                arg: Some(1),
                offset: 0x114,
                cell: playing,
            }],
        );
        let bank_vtable = t.zeros(8);
        t.w32(bank_vtable + 4, info);
        // the channel's state
        let state = t.zeros(4);
        t.stub_at(
            fr::FMOD_GET_STATE,
            false,
            2,
            2,
            &[Write {
                arg: Some(1),
                offset: 0,
                cell: state,
            }],
        );
        t.stub(fr::FMOD_SET_VOLUME, false, 2, 2);
        // the sfx mutex, tree and per-event flag (each event's own first
        // word, so the loop can stop anywhere)
        let wait = t.stub_code(fr::WAIT_FOR_SINGLE_OBJECT, false, 2, 2);
        t.g32(fr::WAIT_FOR_SINGLE_OBJECT, wait);
        let release = t.stub_code(fr::RELEASE_MUTEX, false, 1, 1);
        t.g32(fr::RELEASE_MUTEX, release);
        t.stub(fr::SFX_EVENT_TREE_FIND, true, 1, 1);
        let flag_log = t.stub_code(fr::SFX_ANY_EVENT_FLAG, true, 0, 0);
        let flag_cell = t.cells[&fr::SFX_ANY_EVENT_FLAG];
        let mut c = vec![0x8b, 0x01, 0xa3]; // mov eax, [ecx]; mov [cell], eax
        c.extend(flag_cell.to_le_bytes());
        c.push(0xe9);
        let at = t.cave_next;
        c.extend(flag_log.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes());
        let flag = t.code(&c);
        t.jump(fr::SFX_ANY_EVENT_FLAG, flag);
        // the shout
        t.stub(fr::FMOD_SHOUT_GET, false, 2, 2);
        t.stub(fr::FMOD_SHOUT_SET, false, 2, 2);
        for i in 0..32 {
            let w = format!("{name} {i}");
            for tag in [
                fr::ERRCHECK,
                0x104,
                fr::FMOD_GET_STATE,
                fr::FMOD_SET_VOLUME,
                fr::WAIT_FOR_SINGLE_OBJECT,
                fr::RELEASE_MUTEX,
                fr::FMOD_SHOUT_SET,
            ] {
                let v = rng.next();
                t.ret(tag, v);
            }
            // getNumPlayingEvents
            let this = t.zeros(4);
            if rng.below(4) != 0 {
                let bank = t.zeros(8);
                t.w32(bank, bank_vtable);
                t.w32(this, bank);
            }
            let v = rng.next();
            t.w32(playing, v);
            let frame = t.ebp_for(0) - fr::BANK_INFO_FRAME;
            t.check_framed(
                &format!("{w} getNumPlayingEvents"),
                0x0070_fb20,
                Conv::Thiscall(this),
                &[],
                frame,
                &[(0, 4), (0x104, 0x18)],
                |g| u(fr::sfx_bank_playing_events(g, frame, this)),
            );
            // volumeDirect
            let voice = t.random(&mut rng, 0x90);
            if rng.below(5) == 0 {
                t.w32(voice + 0x14, 0);
            }
            t.w32(voice + 0x80, rng.below(3));
            let scale = rng.float();
            t.w32(voice + 0x88, scale);
            let s = [0, 8, 0xff, 0xf7, rng.next()][rng.below(5) as usize];
            t.w32(state, s);
            let volume = rng.float();
            let keep = rng.below(3);
            let frame = t.ebp_for(2) - 4;
            t.check_framed(
                &format!("{w} volumeDirect state {s:#x} keep {keep}"),
                0x0070_b7f0,
                Conv::Thiscall(voice),
                &[volume, keep],
                frame,
                &[(0, 4)],
                |g| u(fr::voice_volume_direct(g, frame, voice, volume, keep)),
            );
            // isPlaying
            let sfx = t.random(&mut rng, 0xd0);
            let node = match rng.below(5) {
                0 => 0,
                1 => 0xffff_fff0,
                _ => {
                    let n = rng.below(6);
                    let events = t.zeros(4 * n as usize);
                    for k in 0..n {
                        let e = match rng.below(3) {
                            0 => 0,
                            _ => {
                                let e = t.zeros(4);
                                let f = [0, 0x100, 1, rng.next()][rng.below(4) as usize];
                                t.w32(e, f);
                                e
                            }
                        };
                        t.w32(events + 4 * k, e);
                    }
                    let node = t.zeros(0x20);
                    t.w32(node + 0x10, n);
                    t.w32(node + 0x18, events);
                    node
                }
            };
            t.ret(fr::SFX_EVENT_TREE_FIND, node);
            let id = rng.word();
            let frame = t.ebp_for(1) - 4;
            t.check_framed(
                &format!("{w} isPlaying"),
                0x0070_ce90,
                Conv::Thiscall(sfx),
                &[id],
                frame,
                &[(0, 4)],
                |g| u(fr::sfx_is_playing(g, frame, sfx, id)),
            );
            // setVolume
            let shout = t.random(&mut rng, 0x10);
            if rng.below(4) == 0 {
                t.w32(shout + 4, 0);
            }
            let got = if rng.below(3) == 0 { rng.next() } else { 0 };
            t.ret(fr::FMOD_SHOUT_GET, got);
            let volume = rng.float();
            let frame = t.ebp_for(1) - 4;
            t.check_framed(
                &format!("{w} shout setVolume"),
                0x0070_e400,
                Conv::Thiscall(shout),
                &[volume],
                frame,
                &[(0, 4)],
                |g| {
                    fr::shout_set_volume(g, frame, shout, volume);
                    Out::None
                },
            );
        }
    }
}

/// The `GDraw` target setter and resource map, `BlurEffect_initialize_006c38b0`.
#[test]
fn frames_gdraw_target_map_blur() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_2b13);
        let dim = t.zeros(4);
        let desc = t.stub_out(
            0x20,
            false,
            2,
            2,
            &[Write {
                arg: Some(1),
                offset: 4,
                cell: dim,
            }],
        );
        let view_vtable = t.zeros(0x40);
        t.w32(view_vtable + 0x20, desc);
        let mapped = t.zeros(4);
        let map = t.stub_out(
            0x38,
            false,
            6,
            6,
            &[Write {
                arg: Some(5),
                offset: 0,
                cell: mapped,
            }],
        );
        let ctx_vtable = t.zeros(0x40);
        t.w32(ctx_vtable + 0x38, map);
        let warn4 = t.stub_code(fr::GDRAW_SEND_WARNING + 4, false, 4, 0);
        let warn5 = t.stub_code(fr::GDRAW_SEND_WARNING, false, 5, 0);
        let path = t.zeros(4);
        t.stub_at(
            fr::FIOS_UNIFY,
            false,
            3,
            0,
            &[Write {
                arg: Some(1),
                offset: 0,
                cell: path,
            }],
        );
        t.stub(fr::ASYNC_GET_INSTANCE, false, 0, 0);
        t.stub(fr::ASYNC_LOAD_CLUSTER, false, 7, 0);
        for i in 0..32 {
            let w = format!("{name} {i}");
            for tag in [0x20, fr::ASYNC_GET_INSTANCE, fr::ASYNC_LOAD_CLUSTER] {
                let v = rng.next();
                t.ret(tag, v);
            }
            // setRenderTargetView
            let state = t.random(&mut rng, 0xb00);
            if rng.below(2) == 0 {
                t.w32(state + 0xab8, 0);
            }
            t.w32(
                state + 0xac0,
                [0, u32::MAX, rng.next()][rng.below(3) as usize],
            );
            t.g32(fr::GDRAW_STATE, state);
            let view = t.zeros(8);
            t.w32(view, view_vtable);
            let d = [6, 5, 7, rng.next()][rng.below(4) as usize];
            t.w32(dim, d);
            let args = [rng.word(), rng.word(), rng.word(), rng.word()];
            let frame = t.ebp_for(5) - fr::VIEW_DESC_FRAME;
            t.check_framed(
                &format!("{w} setRenderTargetView dim {d}"),
                0x00a3_8300,
                Conv::Cdecl,
                &[view, args[0], args[1], args[2], args[3]],
                frame,
                &[(4, 4)],
                |g| u(fr::gdraw_set_target(g, frame, view, args)),
            );
            // mapResource
            let ctx = t.zeros(8);
            t.w32(ctx, ctx_vtable);
            let hr = [0, 1, fr::E_OUTOFMEMORY, 0x8000_4005, rng.next()][rng.below(5) as usize];
            t.ret(0x38, hr);
            t.g32(
                fr::GDRAW_SEND_WARNING,
                if hr == fr::E_OUTOFMEMORY {
                    warn4
                } else {
                    warn5
                },
            );
            let p = rng.next();
            t.w32(mapped, p);
            let resource = rng.word();
            let discard = [0, 1, 0x100, rng.next()][rng.below(4) as usize];
            let frame = t.ebp_for(3) - fr::MAPPED_FRAME;
            t.check_framed(
                &format!("{w} mapResource {hr:#x} discard {discard:#x}"),
                0x00a3_c070,
                Conv::Cdecl,
                &[ctx, resource, discard],
                frame,
                &[(0, 4)],
                |g| u(fr::gdraw_map_resource(g, frame, ctx, resource, discard)),
            );
            // BlurEffect_initialize
            let blur = t.random(&mut rng, 0x40);
            t.g32(fr::BLUR_CLUSTER_NAME, rng.word());
            let v = rng.next();
            t.w32(path, v);
            let args = [rng.word(), rng.word(), rng.word(), rng.word(), rng.word()];
            let frame = t.ebp_for(5) - fr::BLUR_PATH_FRAME;
            t.check_framed(
                &format!("{w} BlurEffect_initialize"),
                0x006c_38b0,
                Conv::Thiscall(blur),
                &args,
                frame,
                &[(0, 4)],
                |g| u(fr::blur_initialize(g, frame, blur, args)),
            );
        }
    }
}

/// The post-effects' vector setters (a copy from their own argument
/// words), `gdraw_newPool_00a3bcd0` (its argument overwritten),
/// `Distortion_onShaderClusterLoaded_006c28b0` (its argument's address
/// handed on).
#[test]
fn frames_params_pool_cluster() {
    const SETTERS: [(u32, fr::ParamSlot); 4] = [
        (0x006c_45c0, fr::PARAM_48),
        (0x006c_4680, fr::PARAM_4C),
        (0x006c_4730, fr::PARAM_50),
        (0x0070_36f0, fr::PARAM_54),
    ];
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_2b21);
        t.stub(fr::MEMCPY, false, 3, 0);
        let malloc = t.stub_code(fr::GDRAW_MALLOC, false, 3, 3);
        t.g32(fr::GDRAW_MALLOC, malloc);
        t.stub(fr::NODE_POOL_INIT, false, 3, 0);
        t.stub(fr::ASYNC_GET_INSTANCE, false, 0, 0);
        t.stub(fr::GET_APPLICATION, false, 0, 0);
        t.stub(fr::FIXUP_CLUSTERS, false, 2, 0);
        t.stub(fr::DISTORTION_ON_CLUSTER, true, 0, 0);
        let table = t.rebase(fr::GDRAW_POOL_SIZES);
        for i in 0..32 {
            let w = format!("{name} {i}");
            for tag in [
                fr::MEMCPY,
                fr::NODE_POOL_INIT,
                fr::ASYNC_GET_INSTANCE,
                fr::GET_APPLICATION,
                fr::FIXUP_CLUSTERS,
                fr::DISTORTION_ON_CLUSTER,
            ] {
                let v = rng.next();
                t.ret(tag, v);
            }
            // the setters
            for &(sym, slot) in &SETTERS {
                let this = t.zeros(0x60);
                let param = t.zeros(0x10);
                let offset = [0, 3, 4, 8, 0x40, rng.below(0x1_0000)][rng.below(6) as usize];
                let size = [0, 0x10, 0x1fff, rng.next() & 0xffff][rng.below(4) as usize];
                t.write(param + 8, &(offset as u16).to_le_bytes());
                t.write(param + 0xa, &(size as u16).to_le_bytes());
                let buffer = t.zeros(4);
                let limit = offset + (size & 0x1fff);
                let have = match rng.below(3) {
                    0 => limit,
                    1 => limit.wrapping_sub(1),
                    _ => rng.next(),
                };
                t.w32(buffer, have);
                let holder = t.zeros(8);
                t.w32(holder + 4, buffer);
                t.w32(this + slot.holder, holder);
                t.w32(this + slot.param, param);
                if rng.below(4) == 0 {
                    t.w32(this + slot.guard, 0);
                } else if slot.guard != slot.param {
                    t.w32(this + slot.guard, rng.next() | 1);
                }
                let args = [rng.word(), rng.word(), rng.word(), rng.word()];
                let ebp = t.ebp_for(4);
                t.check_framed(
                    &format!("{w} setParam {sym:#x} offset {offset:#x} size {size:#x}"),
                    sym,
                    Conv::Thiscall(this),
                    &args,
                    ebp + 8,
                    &[],
                    |g| {
                        put_args(g, ebp + 8, &args);
                        u(fr::post_set_param(g, slot, this, ebp + 8))
                    },
                );
            }
            // newPool
            let kind = rng.below(4);
            let (n, v) = (rng.below(0x100), rng.next());
            t.w32(table + 8 * kind, n);
            t.w32(table + 8 * kind + 4, v);
            let pool = if rng.below(4) == 0 {
                0
            } else {
                t.random(&mut rng, 0x10)
            };
            t.ret(fr::GDRAW_MALLOC, pool);
            let ebp = t.ebp_for(1);
            t.check_framed(
                &format!("{w} newPool kind {kind}"),
                0x00a3_bcd0,
                Conv::Cdecl,
                &[kind],
                ebp + 8,
                &[(0, 4)],
                |g| {
                    put_args(g, ebp + 8, &[kind]);
                    u(fr::gdraw_new_pool(g, ebp + 8))
                },
            );
            // onShaderClusterLoaded
            let distortion = t.zeros(0x30);
            let holder = t.zeros(4);
            t.w32(holder, distortion);
            let cluster = rng.word();
            let ebp = t.ebp_for(2);
            t.check_framed(
                &format!("{w} distortion cluster loaded"),
                0x006c_28b0,
                Conv::Cdecl,
                &[cluster, holder],
                ebp + 8,
                &[(0, 8)],
                |g| {
                    put_args(g, ebp + 8, &[cluster, holder]);
                    u(fr::distortion_on_cluster_loaded(g, ebp + 8))
                },
            );
        }
    }
}

/// `LoadingScreen_initialize`, `SaveDataMachine_setSaveLoadFocus_00702cd0`,
/// `ClusterSlots_give_0069ae20`, `IggyMenuManager_callback_00a260f0`.
#[test]
fn frames_loading_focus_slots_callback() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_2b22);
        t.stub(m::LOADING_LOAD_TEXTURES, true, 0, 0);
        t.stub(m::POST_PROCESS_INSTANCE, false, 0, 0);
        t.stub(m::POST_PROCESS_LOOKUP, true, 1, 1);
        t.stub(m::LOADING_PREPARE_MESH, true, 0, 0);
        let sprintf = t.stub_code(0x00b0_c3dc, false, 3, 0);
        t.g32(0x00b0_c3dc, sprintf);
        t.stub(m::IGGY_DATA_VALUE_SET, false, 2, 0);
        t.stub(m::CALL_IGGY_AS3, true, 5, 5);
        t.stub(s::OPERATOR_DELETE, false, 1, 0);
        t.stub(m::IGGY_MANAGER_HANDLE, true, 3, 3);
        for i in 0..32 {
            let w = format!("{name} {i}");
            for tag in [
                m::LOADING_LOAD_TEXTURES,
                m::POST_PROCESS_INSTANCE,
                m::POST_PROCESS_LOOKUP,
                m::LOADING_PREPARE_MESH,
                0x00b0_c3dc,
                m::IGGY_DATA_VALUE_SET,
                m::CALL_IGGY_AS3,
                s::OPERATOR_DELETE,
                m::IGGY_MANAGER_HANDLE,
            ] {
                let v = rng.next();
                t.ret(tag, v);
            }
            let screen = t.random(&mut rng, 0x40);
            t.check(
                &format!("{w} LoadingScreen_initialize"),
                0x006e_4720,
                Conv::Thiscall(screen),
                &[],
                |g| u(m::loading_screen_initialize(g, screen)),
            );
            t.g32(0x00ce_b248, rng.word());
            let x = rng.word();
            t.check(
                &format!("{w} setSaveLoadFocus"),
                0x0070_2cd0,
                Conv::Cdecl,
                &[x],
                |g| u(m::save_set_focus(g, x)),
            );
            // the pool: inside (first, last, past the end), before it, after
            let slots = t.random(&mut rng, 0x6420);
            let hint = [0, 0x3ff, 0x400, u32::MAX, rng.below(0x400)][rng.below(5) as usize];
            t.w32(slots + 0x641c, hint);
            let p = match rng.below(6) {
                0 => slots + 0x1c,
                1 => slots + 0x1c + 0x18 * 0x3ff,
                2 => slots + 0x601c,
                3 => slots + 0x1b,
                4 => rng.word(),
                _ => slots + 0x1c + rng.below(0x6000),
            };
            t.check(
                &format!(
                    "{w} ClusterSlots_give {:#x} hint {hint:#x}",
                    p.wrapping_sub(slots)
                ),
                0x0069_ae20,
                Conv::Thiscall(slots),
                &[p],
                |g| u(m::cluster_slots_give(g, slots, p)),
            );
            t.g32(0x0198_4c6c, rng.word());
            let (a, b, c) = (rng.word(), rng.word(), rng.word());
            t.check(
                &format!("{w} IggyMenuManager callback"),
                0x00a2_60f0,
                Conv::Stdcall,
                &[a, b, c],
                |g| u(m::iggy_manager_handle_callback(g, a, b, c)),
            );
        }
    }
}

/// A search tree over `nodes` in order (the middle one the root, parents
/// linked, leaves pointing at `head`); its root.
fn tree(t: &mut Twins, nodes: &[u32], head: u32, parent: u32) -> u32 {
    if nodes.is_empty() {
        return head;
    }
    let mid = nodes.len() / 2;
    let nd = nodes[mid];
    t.w8(nd + 0xd, 0);
    t.w32(nd + 4, parent);
    let l = tree(t, &nodes[..mid], head, nd);
    let r = tree(t, &nodes[mid + 1..], head, nd);
    t.w32(nd, l);
    t.w32(nd + 8, r);
    nd
}

/// Round 3, batch 1: the wide calls (`TextUtil_init`,
/// `TOMkpCrossEasyStrFontSClut`), four singletons, two reserves, the
/// 0x40-element assign, the tree erase, `create_mspace_with_base`, the
/// escape menu's cancel input, the wave textures, the opus size sum,
/// `rcbgRestoreReg`, `MenuUnloadTexture`.
#[test]
fn r3_wide_singletons_arrays_frames() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_3b01);
        let stubs: [(u32, bool, u32, u32); 30] = [
            (r3::ASYNC_LOAD_GET, false, 0, 0),
            (r3::ASYNC_LOAD_CLUSTER, false, 9, 0),
            (r3::CROSS_EASY_RGBA, false, 10, 0),
            (m::OPERATOR_NEW, false, 1, 0),
            (r3::ASYNC_LOAD_MANAGER.2, true, 0, 0),
            (r3::POST_PROCESS_MANAGER.2, true, 0, 0),
            (r3::POST_PROCESS_TWEAK.2, true, 0, 0),
            (r3::TEXTURE_ANIMATION_CTOR, true, 0, 0),
            (r3::TEXTURE_ANIMATION_SETUP, true, 0, 0),
            (s::USER_MALLOC, false, 1, 0),
            (s::USER_FREE, false, 1, 0),
            (fr::MEMCPY, false, 3, 0),
            (r3::ALIGNED_ALLOC, false, 2, 0),
            (r3::COPY_ELEMENTS40, false, 3, 0),
            (r3::DESTROY_ELEMENTS40, false, 2, 0),
            (r3::INIT_MPARAMS, false, 0, 0),
            (r3::INIT_USER_MSTATE, false, 2, 0),
            (mn::CALL_AS3, true, 5, 5),
            (r3::LOAD_MENU_TEXTURE, true, 1, 1),
            (r3::ACC_HEADER_SIZE, false, 1, 0),
            (r3::OPUS_ENTRY, false, 3, 0),
            (r3::ACC_ENTRY_SIZE, false, 1, 0),
            (r3::RC_PACKET_SET_BUFFER, false, 2, 0),
            (r3::RC_IDENTITY, false, 1, 0),
            (r3::RC_PACKET_STEP, false, 1, 0),
            (r3::RC_PACKET_SEND, false, 1, 0),
            (r3::UI_RELEASE_TEXTURE, true, 1, 1),
            (r3::SET_TEXTURE_UNLOADED, true, 1, 1),
            (r3::SCENE_UNLOAD_TEXTURE, true, 1, 1),
            (r3::MAKE_EVENT, false, 4, 0),
        ];
        for &(sym, this, n, pops) in &stubs {
            t.stub(sym, this, n, pops);
        }
        // callees that write through the pointers they get
        let erase_cell = t.zeros(4);
        t.stub_at(
            r3::TREE_ERASE,
            true,
            2,
            2,
            &[Write {
                arg: Some(0),
                offset: 0,
                cell: erase_cell,
            }],
        );
        let packet_cell = t.zeros(4);
        t.stub_at(
            r3::RC_PACKET_INIT,
            false,
            1,
            0,
            &[Write {
                arg: Some(0),
                offset: 4,
                cell: packet_cell,
            }],
        );
        let name_cells = [t.zeros(4), t.zeros(4)];
        t.stub_at(
            r3::TEXTURE_NAME,
            false,
            3,
            0,
            &[
                Write {
                    arg: Some(1),
                    offset: 0,
                    cell: name_cells[0],
                },
                Write {
                    arg: Some(1),
                    offset: 0xfc,
                    cell: name_cells[1],
                },
            ],
        );
        let wide = |h: &mut G, f: u32, a: &[u32]| h.0.call(f, Conv::Cdecl, a).unwrap().eax;
        for i in 0..32 {
            let w = format!("{name} {i}");
            let log = t.log;
            t.w32(log, 0);
            for &(sym, ..) in &stubs {
                let v = rng.word();
                t.ret(sym, v);
            }
            for c in [erase_cell, packet_cell, name_cells[0], name_cells[1]] {
                let v = rng.next();
                t.w32(c, v);
            }
            for tag in [r3::TREE_ERASE, r3::RC_PACKET_INIT, r3::TEXTURE_NAME] {
                let v = rng.next();
                t.ret(tag, v);
            }
            // TextUtil_init, the font call
            let table = t.rebase(r3::TEXT_CLUSTERS);
            let (x, y) = (rng.next(), rng.next());
            t.w32(table, x);
            t.w32(table + 4, y);
            let n = rng.word();
            t.check(
                &format!("{w} TextUtil_init {n:#x}"),
                0x006f_6370,
                Conv::Thiscall(n),
                &[],
                |g| u(r3::text_util_init(g, n, &mut { wide })),
            );
            let args = [
                rng.word(),
                rng.float(),
                rng.float(),
                rng.word(),
                rng.float(),
                rng.float(),
            ];
            t.check(
                &format!("{w} TOMkpCrossEasyStrFontSClut"),
                0x0090_1660,
                Conv::Cdecl,
                &args,
                |g| u(r3::mkp_cross_easy(g, args, &mut { wide })),
            );
            // the singletons
            for (sym, inst) in [
                (0x0069_1a80, r3::ASYNC_LOAD_MANAGER),
                (0x0069_2a90, r3::POST_PROCESS_MANAGER),
                (0x0069_2b10, r3::POST_PROCESS_TWEAK),
            ] {
                let gv = if rng.below(2) == 0 { 0 } else { rng.next() };
                t.g32(inst.0.sym, gv);
                if rng.below(3) == 0 {
                    t.ret(m::OPERATOR_NEW, 0);
                }
                t.check(
                    &format!("{w} createInstance {sym:#x}"),
                    sym,
                    Conv::Cdecl,
                    &[],
                    |g| {
                        mn::create_instance(g, inst);
                        Out::None
                    },
                );
            }
            let gv = if rng.below(2) == 0 { 0 } else { rng.next() };
            t.g32(r3::TEXTURE_ANIMATION_MANAGER.sym, gv);
            if rng.below(3) == 0 {
                t.ret(m::OPERATOR_NEW, 0);
            }
            t.check(
                &format!("{w} TextureAnimationManager create"),
                0x006b_00a0,
                Conv::Cdecl,
                &[],
                |g| {
                    r3::texture_animation_create(g);
                    Out::None
                },
            );
            // the reserves
            for (sym, scale) in [(0x0070_edb0u32, 8u32), (0x0070_fbf0, 4)] {
                let arr = t.zeros(0xc);
                let count = rng.below(5);
                t.w32(arr, count);
                let cap = rng.next();
                t.w32(arr + 4, cap);
                let storage = if rng.below(3) == 0 { 0 } else { rng.next() };
                t.w32(arr + 8, storage);
                let n = [0, 1, 7, 0x8000_0000, u32::MAX, rng.next()][rng.below(6) as usize];
                if rng.below(3) == 0 {
                    t.ret(s::USER_MALLOC, 0);
                }
                t.check(
                    &format!("{w} reserve {sym:#x} storage {storage:#x} n {n:#x}"),
                    sym,
                    Conv::Thiscall(arr),
                    &[n],
                    |g| {
                        u(if scale == 8 {
                            r3::array8_reserve(g, arr, n)
                        } else {
                            s::sfx_id_array_reserve(g, arr, n)
                        })
                    },
                );
            }
            // the 0x40-element assign
            let this = t.zeros(8);
            let old = [0, rng.next()][rng.below(2) as usize];
            let word = [rng.below(4), 0x8000_0003, 0x4000_0002, rng.next()][rng.below(4) as usize];
            t.w32(this, word);
            t.w32(this + 4, old);
            let src = t.zeros(8);
            let count = [0, 0x8000_0000, 2, 0x8000_0002, rng.next()][rng.below(5) as usize];
            t.w32(src, count);
            let data = rng.next();
            t.w32(src + 4, data);
            match rng.below(3) {
                0 => t.ret(r3::ALIGNED_ALLOC, 0),
                1 => t.ret(r3::ALIGNED_ALLOC, old),
                _ => {}
            }
            let ebp = t.ebp_for(1);
            t.check_framed(
                &format!("{w} PArray40 assign"),
                0x006a_fa70,
                Conv::Thiscall(this),
                &[src],
                ebp + 8,
                &[(0, 4)],
                |g| {
                    put_args(g, ebp + 8, &[src]);
                    u(r3::parray40_assign(g, this, ebp + 8))
                },
            );
            // the tree erase: 0..7 nodes, the value one of theirs or not
            let head = t.zeros(0x30);
            t.w8(head + 0xd, 1);
            let count = rng.below(8) as usize;
            let nodes: Vec<u32> = (0..count).map(|_| t.zeros(0x30)).collect();
            let values: Vec<u32> = (0..count).map(|_| rng.below(6)).collect();
            for (&nd, &v) in nodes.iter().zip(&values) {
                t.w32(nd + 0x28, v);
            }
            let root = tree(&mut t, &nodes, head, head);
            t.w32(head + 4, root);
            t.w32(head, nodes.first().copied().unwrap_or(head));
            t.w32(head + 8, nodes.last().copied().unwrap_or(head));
            let map = t.zeros(4);
            t.w32(map, head);
            let value = rng.below(7);
            let ebp = t.ebp_for(1);
            t.check_framed(
                &format!("{w} tree erase {value} of {values:?}"),
                0x006a_1d10,
                Conv::Thiscall(map),
                &[value],
                ebp + 8,
                &[(0, 4)],
                |g| {
                    put_args(g, ebp + 8, &[value]);
                    u(r3::tree_erase_value(g, map, ebp + 8))
                },
            );
            // create_mspace_with_base
            let magic = rng.below(2);
            t.g32(r3::MPARAMS_MAGIC, magic);
            let page = [0x1000, 0, rng.next()][rng.below(3) as usize];
            t.g32(r3::MPARAMS_PAGE_SIZE, page);
            let m_state = t.random(&mut rng, 0x200);
            t.ret(r3::INIT_USER_MSTATE, m_state);
            let capacity = [
                0x210,
                0x211,
                0xffff_fdf0u32.wrapping_sub(page),
                0xffff_fdefu32.wrapping_sub(page),
                rng.next(),
                0x10_0000,
            ][rng.below(6) as usize];
            let (base, locked) = (rng.next(), rng.word());
            t.check(
                &format!("{w} create_mspace_with_base {capacity:#x} page {page:#x}"),
                0x006e_bd30,
                Conv::Cdecl,
                &[base, capacity, locked],
                |g| u(r3::create_mspace_with_base(g, base, capacity, locked)),
            );
            // the escape menu's cancel input
            let menu = t.random(&mut rng, 0x100);
            let flag = [0, 1, 0x80][rng.below(3) as usize];
            t.w8(menu + 0xfc, flag);
            let input = t.zeros(4);
            let kind = rng.below(10);
            t.w32(input, kind);
            let b = rng.next();
            let ebp = t.ebp_for(2);
            t.check_framed(
                &format!("{w} esc cancel input {flag} {kind}"),
                0x0070_18a0,
                Conv::Thiscall(menu),
                &[input, b],
                ebp - 0xc,
                &[],
                |g| Out::U8(r3::esc_on_cancel_input(g, ebp - 0xc, menu, input)),
            );
            // the wave textures
            let scene = rng.next();
            t.g32(r3::PHYRE_SCENE.sym, scene);
            t.check(
                &format!("{w} wave textures"),
                0x0067_cba0,
                Conv::Cdecl,
                &[],
                |g| u(r3::load_main_menu_wave_texture(g)),
            );
            // the opus size sum
            let rec = t.random(&mut rng, 8);
            let n = rng.below(5) as u16;
            t.write(rec + 6, &n.to_le_bytes());
            let entry = t.random(&mut rng, 0x20);
            t.ret(r3::OPUS_ENTRY, entry);
            let out = t.random(&mut rng, 4);
            let base = rng.next();
            t.check(
                &format!("{w} opus_0706_acc_draw {n}"),
                0x007f_7360,
                Conv::Cdecl,
                &[rec, base, out],
                |g| u(r3::opus_0706_acc_draw(g, rec, base, out)),
            );
            // rcbgRestoreReg
            let ebp = t.ebp_for(0);
            t.check_framed(
                &format!("{w} rcbgRestoreReg"),
                0x0092_0250,
                Conv::Cdecl,
                &[],
                ebp - 8,
                &[(0, 8)],
                |g| u(r3::rcbg_restore_reg(g, ebp - 8)),
            );
            // MenuUnloadTexture
            let ui = t.zeros(0x2c00);
            let holder = t.zeros(4);
            let geometry = rng.next();
            t.w32(holder, geometry);
            t.w32(ui + 0x2bd4, holder);
            t.g32(r3::GRAPHIC_UI.sym, ui);
            let id = if rng.below(4) == 0 { 0 } else { rng.next() };
            let ebp = t.ebp_for(1);
            t.check_framed(
                &format!("{w} MenuUnloadTexture {id:#x}"),
                0x0067_dbf0,
                Conv::Stdcall,
                &[id],
                ebp - 0x104,
                &[(0, 0x100)],
                |g| {
                    let r = r3::menu_unload_texture(g, ebp - 0x104, id);
                    // for id 0 the original's eax is its cookie arithmetic
                    if id == 0 { Out::None } else { u(r) }
                },
            );
        }
    }
}

/// Round 3, batch 2: three more singletons, `InitVFXRenderDataLine`, the
/// two tree adds, `MotionBlur_initialize` (eleven words), the escape
/// menu's enable flags (junk words), the `GDraw` dynamic buffer lock, the
/// button icon UVs.
#[test]
fn r3_singletons_trees_frames() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_3b02);
        let mut stubs: Vec<(u32, bool, u32, u32)> = vec![
            (m::OPERATOR_NEW, false, 1, 0),
            (r3::BLUR_EFFECT_NEW.2, true, 0, 0),
            (r3::MOTION_BLUR_NEW.2, true, 0, 0),
            (r3::ABMAP_CTOR, true, 1, 1),
            (r3::TAGGED_TREE_INSERT, true, 1, 1),
            (r3::VFX_INT_TREE_INSERT, true, 1, 1),
            (r3::POOL_GIVE, true, 1, 1),
            (r3::TREE_FIXUP, true, 1, 1),
            (r3::ASYNC_LOAD_GET, false, 0, 0),
            (r3::ASYNC_LOAD_CLUSTER, false, 11, 0),
            (r3::CALL_FLASH_1, false, 6, 0),
            (r3::GDRAW_MAP_RESOURCE, false, 3, 0),
            (r3::REMAP_GET, false, 0, 0),
            (r3::SET_QUAD_UV, true, 5, 5),
        ];
        for (k, &sym) in r3::VFX_LINE_CALLS.iter().enumerate() {
            stubs.push((sym, false, [4, 2, 2, 3, 2, 2][k], 0));
        }
        for &(sym, this, n, pops) in &stubs {
            t.stub(sym, this, n, pops);
        }
        let warn = t.stub_code(fr::GDRAW_SEND_WARNING, false, 4, 0);
        t.g32(fr::GDRAW_SEND_WARNING, warn);
        let mut cells = vec![];
        let mut cell = |t: &mut Twins| {
            let c = t.zeros(4);
            cells.push(c);
            c
        };
        let refill = cell(&mut t);
        t.stub_at(
            r3::POOL_REFILL,
            true,
            1,
            1,
            &[Write {
                arg: None,
                offset: 0,
                cell: refill,
            }],
        );
        let path = cell(&mut t);
        t.stub_at(
            r3::TEXTURE_NAME,
            false,
            3,
            0,
            &[Write {
                arg: Some(1),
                offset: 0,
                cell: path,
            }],
        );
        let (kind, flag) = (cell(&mut t), cell(&mut t));
        t.stub_at(
            r3::DATA_VALUE_SET_BOOL,
            false,
            2,
            0,
            &[
                Write {
                    arg: Some(0),
                    offset: 0,
                    cell: kind,
                },
                Write {
                    arg: Some(0),
                    offset: 8,
                    cell: flag,
                },
            ],
        );
        let uv = [cell(&mut t), cell(&mut t), cell(&mut t), cell(&mut t)];
        for (k, sym) in [r3::REMAP_PC_UV, r3::REMAP_PAD_UV].into_iter().enumerate() {
            t.stub_at(
                sym,
                true,
                3,
                3,
                &[
                    Write {
                        arg: Some(1),
                        offset: 0,
                        cell: uv[2 * k],
                    },
                    Write {
                        arg: Some(2),
                        offset: 0,
                        cell: uv[2 * k + 1],
                    },
                ],
            );
        }
        let wide = |h: &mut G, f: u32, a: &[u32]| h.0.call(f, Conv::Cdecl, a).unwrap().eax;
        let state = t.zeros(0xa00);
        t.g32(r3::GDRAW_STATE, state);
        for i in 0..32 {
            let w = format!("{name} {i}");
            let log = t.log;
            t.w32(log, 0);
            for &(sym, ..) in &stubs {
                let v = rng.word();
                t.ret(sym, v);
            }
            for tag in [
                r3::POOL_REFILL,
                r3::TEXTURE_NAME,
                r3::DATA_VALUE_SET_BOOL,
                r3::REMAP_PC_UV,
                r3::REMAP_PAD_UV,
                fr::GDRAW_SEND_WARNING,
            ] {
                let v = rng.next();
                t.ret(tag, v);
            }
            for &c in &cells {
                let v = rng.float();
                t.w32(c, v);
            }
            // the singletons
            for (sym, inst) in [
                (0x006c_34d0, r3::BLUR_EFFECT_NEW),
                (0x006c_5030, r3::MOTION_BLUR_NEW),
            ] {
                let gv = if rng.below(2) == 0 { 0 } else { rng.next() };
                t.g32(inst.0.sym, gv);
                if rng.below(3) == 0 {
                    t.ret(m::OPERATOR_NEW, 0);
                }
                t.check(
                    &format!("{w} createInstance {sym:#x}"),
                    sym,
                    Conv::Cdecl,
                    &[],
                    |g| {
                        mn::create_instance(g, inst);
                        Out::None
                    },
                );
            }
            let gv = if rng.below(2) == 0 { 0 } else { rng.next() };
            t.g32(r3::GRAPHIC_ABMAP.sym, gv);
            if rng.below(3) == 0 {
                t.ret(m::OPERATOR_NEW, 0);
            }
            let arg = rng.next();
            t.check(
                &format!("{w} abmap create"),
                0x0068_4a60,
                Conv::Cdecl,
                &[arg],
                |g| {
                    r3::abmap_create(g, arg);
                    Out::None
                },
            );
            // InitVFXRenderDataLine
            for &sym in &[r3::VFX_LINE_CALLS[0], r3::VFX_LINE_CALLS[2]] {
                if rng.below(3) == 0 {
                    t.ret(sym, 0);
                }
            }
            if rng.below(3) == 0 {
                t.ret(r3::VFX_LINE_CALLS[1], 0);
            } else {
                let data = t.random(&mut rng, 0xc0);
                t.ret(r3::VFX_LINE_CALLS[1], data);
            }
            let args = [rng.next(), rng.next(), rng.next(), rng.next()];
            t.check(
                &format!("{w} InitVFXRenderDataLine"),
                0x0071_12d0,
                Conv::Cdecl,
                &args,
                |g| {
                    u(r3::init_vfx_render_data_line(
                        g, args[0], args[1], args[2], args[3],
                    ))
                },
            );
            // the tree adds
            for (sym, insert) in [
                (0x006a_2830, r3::TAGGED_TREE_INSERT),
                (0x0070_cd60, r3::VFX_INT_TREE_INSERT),
            ] {
                let this = t.random(&mut rng, 0x24);
                let node = t.random(&mut rng, 0x14);
                match rng.below(3) {
                    0 => t.w32(this + 0x18, node),
                    1 => {
                        t.w32(this + 0x18, 0);
                        t.w32(refill, node);
                    }
                    _ => {
                        t.w32(this + 0x18, 0);
                        t.w32(refill, 0);
                    }
                }
                if rng.below(2) == 0 {
                    t.ret(insert, 0);
                }
                let (key, value) = (t.random(&mut rng, 4), t.random(&mut rng, 4));
                t.check(
                    &format!("{w} tree add {sym:#x}"),
                    sym,
                    Conv::Thiscall(this),
                    &[key, value],
                    |g| u(r3::tree_add(g, insert, this, key, value)),
                );
            }
            // MotionBlur_initialize
            let this = rng.word();
            let abc = [rng.next(), rng.word(), rng.word()];
            let ebp = t.ebp_for(3);
            t.check_framed(
                &format!("{w} MotionBlur_initialize"),
                0x006c_5360,
                Conv::Thiscall(this),
                &abc,
                ebp - 0x104,
                &[(0, 0x100)],
                |g| {
                    u(r3::motion_blur_initialize(
                        g,
                        ebp - 0x104,
                        this,
                        [abc[1], abc[2]],
                        &mut { wide },
                    ))
                },
            );
            // the escape menu's enable flags: the value's never-written
            // words are 0 on both sides
            let menu = rng.next();
            let on = rng.word();
            let ebp = t.ebp_for(1);
            let frame = ebp - 0x10;
            t.w32(frame + 4, 0);
            t.w32(frame + 0xc, 0);
            t.check_framed(
                &format!("{w} esc enable flags"),
                0x006f_ca50,
                Conv::Thiscall(menu),
                &[on],
                frame,
                &[(0, 4), (8, 4)],
                |g| u(r3::esc_set_enable_para_and_special(g, frame, menu, on)),
            );
            // the GDraw dynamic buffer lock
            let buffer = t.zeros(0x10);
            let size = [0, 0x100, rng.next()][rng.below(3) as usize];
            let cursor = [0, 0x80, size, rng.next()][rng.below(4) as usize];
            let n = [0, 0x80, 0x81, size, size.wrapping_add(1), rng.next()][rng.below(6) as usize];
            let words = [rng.next(), size, rng.next(), cursor];
            for (k, &v) in words.iter().enumerate() {
                t.w32(buffer + 4 * k as u32, v);
            }
            let peak = [0, n, rng.next()][rng.below(3) as usize];
            t.w32(state + 0x9b8, peak);
            let ctx = rng.next();
            t.w32(state + 4, ctx);
            if rng.below(3) == 0 {
                t.ret(r3::GDRAW_MAP_RESOURCE, 0);
            }
            t.check(
                &format!("{w} gdraw lock {n:#x} size {size:#x} cursor {cursor:#x}"),
                0x00a3_cf40,
                Conv::Cdecl,
                &[buffer, n],
                |g| u(r3::gdraw_lock_dynamic(g, buffer, n)),
            );
            // the button icon UVs
            let this = t.zeros(0x60);
            let id = rng.word();
            let ebp = t.ebp_for(1);
            t.check_framed(
                &format!("{w} button icon uvs"),
                0x006e_8020,
                Conv::Thiscall(this),
                &[id],
                ebp - 8,
                &[(0, 8)],
                |g| u(r3::button_icon_set_uvs(g, ebp - 8, this, id)),
            );
        }
    }
}

/// `MotionBlur_ctor_006c4ec0` (its own oracles: the singleton test stubs
/// it).
#[test]
fn r3_motion_blur_ctor() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_3b03);
        t.stub(r3::POST_EFFECT_BASE_CTOR, true, 0, 0);
        t.stub(r3::MEMBER68_CTOR, true, 0, 0);
        for i in 0..16 {
            let w = format!("{name} {i}");
            for tag in [r3::POST_EFFECT_BASE_CTOR, r3::MEMBER68_CTOR] {
                let v = rng.next();
                t.ret(tag, v);
            }
            let this = t.random(&mut rng, 0x90);
            t.check(
                &format!("{w} MotionBlur ctor"),
                0x006c_4ec0,
                Conv::Thiscall(this),
                &[],
                |g| u(r3::motion_blur_ctor(g, this)),
            );
        }
    }
}

/// Round 3, batch 2: `ClusterManager_getPClusterByName` (its iterator in
/// the argument slot), `FmodFile_open`, `SfxBank_destroyEvents`, the map
/// range erase, `PArray_appendInto`, `MenuManager_TakeMultiRecord`,
/// `MenuDynamicGeometry_setMeshWorldMatrix`.
#[test]
fn r3_cluster_map_file_records() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_3b04);
        let stubs: [(u32, bool, u32, u32); 14] = [
            (r3::DJB_HASH, false, 1, 0),
            (m::OPERATOR_NEW, false, 1, 0),
            (r3::PSTREAM_FILE_CTOR, true, 5, 5),
            (r3::PSTREAM_FILE_IS_OPEN, true, 0, 0),
            (r3::PSTREAM_FILE_SIZE, true, 0, 0),
            (r3::PSTREAM_FILE_DTOR, true, 0, 0),
            (s::OPERATOR_DELETE, false, 1, 0),
            (s::USER_FREE, false, 1, 0),
            (r3::POOL_RELEASE, true, 0, 0),
            (r3::LIST_TEARDOWN, true, 0, 0),
            (r3::TREE_NODE_NEW, true, 1, 1),
            (r3::ALIGNED_FREE, false, 1, 0),
            (r3::SET_MESH_WORLD, true, 2, 2),
            (r3::SET_MIN_AND_MAX, true, 2, 2),
        ];
        for &(sym, this, n, pops) in &stubs {
            t.stub(sym, this, n, pops);
        }
        let wait = t.stub_code(mn::WAIT_FOR_SINGLE_OBJECT.sym, false, 2, 2);
        t.g32(mn::WAIT_FOR_SINGLE_OBJECT.sym, wait);
        let release = t.stub_code(mn::RELEASE_MUTEX.sym, false, 1, 1);
        t.g32(mn::RELEASE_MUTEX.sym, release);
        let erase_cell = t.zeros(4);
        t.stub_at(
            r3::MAP_ERASE_KEY,
            true,
            4,
            4,
            &[Write {
                arg: Some(0),
                offset: 0,
                cell: erase_cell,
            }],
        );
        // the unlink makes the node's next the list's first
        let log_unlink = t.stub_code(r3::LIST_UNLINK, true, 1, 1);
        let mut c = vec![0x8b, 0x44, 0x24, 0x04, 0x8b, 0x10, 0x89, 0x51, 0x0c, 0xe9];
        let at = t.cave_next;
        c.extend(log_unlink.wrapping_sub(at + 14).to_le_bytes());
        let unlink = t.code(&c);
        t.jump(r3::LIST_UNLINK, unlink);
        for i in 0..32 {
            let w = format!("{name} {i}");
            let log = t.log;
            t.w32(log, 0);
            for &(sym, ..) in &stubs {
                let v = rng.word();
                t.ret(sym, v);
            }
            for tag in [
                mn::WAIT_FOR_SINGLE_OBJECT.sym,
                mn::RELEASE_MUTEX.sym,
                r3::MAP_ERASE_KEY,
                r3::LIST_UNLINK,
            ] {
                let v = rng.next();
                t.ret(tag, v);
            }
            let v = rng.next();
            t.w32(erase_cell, v);
            // a map of 0..7 sorted keys
            let head = t.zeros(0x20);
            t.w8(head + 0xd, 1);
            let count = rng.below(8) as usize;
            let nodes: Vec<u32> = (0..count).map(|_| t.zeros(0x20)).collect();
            let mut keys: Vec<u32> = (0..count).map(|_| rng.below(20)).collect();
            keys.sort_unstable();
            for (&nd, &k) in nodes.iter().zip(&keys) {
                t.w32(nd + 0x10, k);
                let entry = t.zeros(8);
                let first = rng.next();
                t.w32(entry, first);
                let n = [0, 1, 2, u32::MAX][rng.below(4) as usize];
                t.w32(entry + 4, n);
                t.w32(nd + 0x14, entry);
            }
            let root = tree(&mut t, &nodes, head, head);
            t.w32(head + 4, root);
            t.w32(head, nodes.first().copied().unwrap_or(head));
            t.w32(head + 8, nodes.last().copied().unwrap_or(head));
            // getPClusterByName
            let mgr = t.zeros(0x20);
            t.w32(mgr + 0x10, head);
            let holder = t.zeros(4);
            let mutex = rng.next();
            t.w32(holder, mutex);
            t.w32(mgr + 0x18, holder);
            let hash = rng.below(22);
            t.ret(r3::DJB_HASH, hash);
            let text = rng.next();
            let ebp = t.ebp_for(1);
            t.check_framed(
                &format!("{w} getPClusterByName {hash} of {keys:?}"),
                0x0069_b4b0,
                Conv::Thiscall(mgr),
                &[text],
                ebp + 8,
                &[(0, 4)],
                |g| {
                    put_args(g, ebp + 8, &[text]);
                    u(r3::get_pcluster_by_name(g, mgr, ebp + 8))
                },
            );
            // the range erase, from an in-order node to a later one or the end
            if !nodes.is_empty() {
                let from = rng.below(count as u32) as usize;
                let to = from + rng.below((count - from) as u32 + 1) as usize;
                let first = nodes[from];
                let last = nodes.get(to).copied().unwrap_or(head);
                let node = t.random(&mut rng, 0x20);
                t.ret(r3::TREE_NODE_NEW, node);
                let map = t.zeros(4);
                t.w32(map, head);
                let ebp = t.ebp_for(2);
                t.check_framed(
                    &format!("{w} erase range {from}..{to} of {count}"),
                    0x0069_d990,
                    Conv::Thiscall(map),
                    &[first, last],
                    ebp - 4,
                    &[(0, 4), (0xc, 8)],
                    |g| {
                        put_args(g, ebp + 8, &[first, last]);
                        u(r3::vfx_map_erase_range(g, ebp - 4, map, ebp + 8))
                    },
                );
            }
            // FmodFile_open
            let prefix = rng.next();
            t.g32(r3::FMOD_FILE_PREFIX, prefix);
            if rng.below(3) == 0 {
                t.ret(m::OPERATOR_NEW, 0);
            }
            let open = [0, 1, 0x100, 0x1_0001][rng.below(4) as usize];
            t.ret(r3::PSTREAM_FILE_IS_OPEN, open);
            let (size, handle) = (t.zeros(4), t.zeros(4));
            let file = if rng.below(5) == 0 { 0 } else { rng.next() };
            let (unicode, user) = (rng.next(), rng.next());
            t.check(
                &format!("{w} FmodFile_open {open:#x}"),
                0x0070_8510,
                Conv::Stdcall,
                &[file, unicode, size, handle, user],
                |g| u(r3::fmod_file_open(g, file, size, handle)),
            );
            // SfxBank_destroyEvents
            let bank = t.zeros(0x30);
            let list = bank + 4;
            let n = rng.below(4);
            let events: Vec<u32> = (0..n).map(|_| t.zeros(0x20)).collect();
            let end = if rng.below(2) == 0 { 0 } else { list };
            for (k, &e) in events.iter().enumerate() {
                let next = events.get(k + 1).copied().unwrap_or(end);
                t.w32(e, next);
                let data = if rng.below(3) == 0 { 0 } else { rng.next() };
                t.w32(e + 0x18, data);
            }
            t.w32(list + 0xc, events.first().copied().unwrap_or(end));
            t.check(
                &format!("{w} SfxBank destroy events {n}"),
                0x0071_0960,
                Conv::Thiscall(bank),
                &[],
                |g| u(r3::sfx_bank_destroy_events(g, bank)),
            );
            // PArray_appendInto
            let arr = t.zeros(8);
            let n = rng.below(4);
            let word = [n, n | 0x8000_0000][rng.below(2) as usize];
            t.w32(arr, word);
            let block = t.random(&mut rng, 0x10);
            // no storage only for an empty array (the copy reads it)
            let old = if n == 0 && rng.below(2) == 0 {
                0
            } else {
                block
            };
            t.w32(arr + 4, old);
            let dst = match rng.below(4) {
                0 => 0,
                1 => old,
                _ => t.zeros(0x14),
            };
            let value = t.random(&mut rng, 4);
            t.check(
                &format!("{w} appendInto {word:#x} old {old:#x} dst {dst:#x}"),
                0x006a_25e0,
                Conv::Thiscall(arr),
                &[dst, value],
                |g| u(r3::parray_append_into(g, arr, dst, value)),
            );
            // MenuManager_TakeMultiRecord
            let mm = t.zeros(0x5b80);
            for (cap, pool, used, count) in [
                (0x15e8, 0x15ec, 0x2bfc, 0x2c50),
                (0x4240, 0x4244, 0x583c, 0x5840),
            ] {
                let records = t.random(&mut rng, 0x10);
                t.w32(mm + pool, records);
                let c = [0, 3, 0x8000_0004, 4][rng.below(4) as usize];
                t.w32(mm + cap, c);
                let taken = [3, 4, rng.below(5)][rng.below(3) as usize];
                t.w32(mm + used, taken);
                t.w32(mm + count, rng.below(8));
            }
            t.w8(mm + 0x5b74, 1);
            let chain = rng.below(3);
            t.check(
                &format!("{w} TakeMultiRecord {chain}"),
                0x0067_c170,
                Conv::Thiscall(mm),
                &[chain],
                |g| u(r3::take_multi_record(g, mm, chain)),
            );
            // MenuDynamicGeometry_setMeshWorldMatrix
            let geometry = t.zeros(0x28);
            let instance = t.zeros(0xb0);
            for k in 0..6 {
                let v = rng.float();
                t.w32(instance + 0x98 + 4 * k, v);
            }
            let mesh = t.zeros(0x20);
            let bounds = if rng.below(4) == 0 { 0 } else { rng.next() };
            t.w32(mesh + 0x1c, bounds);
            t.w32(
                geometry + 0x24,
                [0, instance, instance][rng.below(3) as usize],
            );
            t.w32(geometry + 0xc, [0, mesh, mesh][rng.below(3) as usize]);
            let scene = rng.next();
            t.g32(r3::PHYRE_SCENE.sym, scene);
            let ebp = t.ebp_for(0);
            t.check_framed(
                &format!("{w} setMeshWorldMatrix"),
                0x006a_2e60,
                Conv::Thiscall(geometry),
                &[],
                ebp - 0x28,
                &[(0, 12), (0x10, 12), (0x20, 8)],
                |g| u(r3::set_mesh_world_matrix(g, ebp - 0x28, geometry)),
            );
        }
    }
}

// ---- continuation round, batch 1 (ffx_glue_other::h4_c1)

/// A stub at `sym` that flips bit 0 of its result cell before each call
/// (so two calls in a row answer differently), else as [`Twins::stub`].
fn stub_toggle(t: &mut Twins, sym: u32, this: bool, n: u32, pops: u32) {
    let rest = t.stub_code(sym, this, n, pops);
    let cell = t.cells[&sym];
    let mut c = vec![0x83, 0x35];
    c.extend(cell.to_le_bytes());
    c.push(0x01); // xor dword [cell], 1
    c.push(0xe9);
    let at = t.cave_next;
    c.extend(rest.wrapping_sub(at + 12).to_le_bytes()); // jmp rest
    let f = t.code(&c);
    t.jump(sym, f);
}

/// The accumulation quads, the single-record take, the name lookup, the
/// two array helpers, the mip dispatcher, the texture-animation records,
/// the icon packet, the escape menu's page state.
#[test]
fn c1_gs_menu_arrays_records() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_c101);
        let stubs: [(u32, bool, u32, u32); 32] = [
            (c1::GS_TEXFLUSH, false, 0, 0),
            (c1::GS_GET, false, 0, 0),
            (c1::GS_INC, false, 0, 0),
            (c1::GS_TEX_HOKAN, false, 1, 0),
            (c1::GS_ALPHA, false, 5, 0),
            (c1::GS_PRIM, false, 1, 0),
            (c1::GS_UV, false, 2, 0),
            (c1::GS_XYZF2, false, 2, 0),
            (c1::GS_UV2, false, 2, 0),
            (c1::GS_RGBAQ, false, 4, 0),
            (c1::GS_XYZF2V, false, 3, 0),
            (c1::GS_SET_PACKET, false, 1, 0),
            (c1::GS_START, false, 0, 0),
            (c1::GS_FRAME_16, false, 2, 0),
            (c1::GS_FRAME, false, 2, 0),
            (c1::GS_END, false, 0, 0),
            (mn::PHYRE_FREE, false, 1, 0),
            (r3::ALIGNED_ALLOC, false, 2, 0),
            (c1::TEXELS_HALVE, false, 4, 0),
            (c1::TEXELS_HALVE_V, false, 6, 0),
            (c1::TEXELS_BOX, false, 5, 0),
            (fr::OPERATOR_DELETE, false, 1, 0),
            (c1::SAMPLER_STATE_CTOR, true, 0, 0),
            (c1::DESTROY_OBJECTS, true, 0, 0),
            (c1::COPY_NEW_A, false, 2, 0),
            (c1::COPY_NEW_B, false, 2, 0),
            (c1::PARRAY40_ASSIGN, true, 1, 1),
            (c1::PSTRING_ASSIGN, true, 1, 1),
            (c1::ADD_PKT_MENU_ICON, false, 4, 0),
            (c1::ADD_PKT_BASE_MES_FONT, false, 12, 0),
            (c1::GET_SCENARIO_FLAG, false, 0, 0),
            (c1::SET_ENABLE_PARA_AND_SPECIAL, true, 1, 1),
        ];
        for &(sym, this, n, pops) in &stubs {
            t.stub(sym, this, n, pops);
        }
        let strings_cell = t.zeros(4);
        t.stub_at(
            c1::MS_GET_ROM_ITEM,
            false,
            2,
            0,
            &[Write {
                arg: Some(1),
                offset: 0,
                cell: strings_cell,
            }],
        );
        stub_toggle(&mut t, c1::IS_ETERNAL_CALM_SCENE, false, 1, 0);
        t.stub(r3::CALL_FLASH_1, false, 6, 0);
        let wide = |h: &mut G, f: u32, a: &[u32]| h.0.call(f, Conv::Cdecl, a).unwrap().eax;
        let table = t.rebase(c1::MENU_NAMES);
        let texts: Vec<u32> = ["", "a", "ab", "menu", "menv", "aenu", "menu2"]
            .iter()
            .map(|s| {
                let mut b = s.as_bytes().to_vec();
                b.push(0);
                t.alloc(&b)
            })
            .collect();
        for i in 0..40 {
            let w = format!("{name} {i}");
            let log = t.log;
            t.w32(log, 0);
            for &(sym, ..) in &stubs {
                let v = rng.next();
                t.ret(sym, v);
            }
            let v = rng.next();
            t.ret(c1::MS_GET_ROM_ITEM, v);
            let v = rng.next();
            t.ret(r3::CALL_FLASH_1, v);
            // the GS quads
            let packet = t.random(&mut rng, 0x20);
            t.ret(c1::GS_GET, packet);
            let fbp = rng.next();
            t.g32(c1::DRAW_FBP.sym, fbp);
            let fix = rng.word();
            t.check(
                &format!("{w} alpha quad"),
                0x0082_baf0,
                Conv::Cdecl,
                &[fix],
                |g| u(c1::acc_draw_alpha_quad(g, fix)),
            );
            let z = rng.word();
            t.check(
                &format!("{w} filter quad"),
                0x0082_bb90,
                Conv::Cdecl,
                &[z],
                |g| u(c1::draw_filter_quad(g, z)),
            );
            let request = [0, 1, rng.next()][rng.below(3) as usize];
            t.g32(c1::ACC_CURRENT_REQUEST.sym, request);
            t.check(
                &format!("{w} set current {request:#x}"),
                0x0082_ce90,
                Conv::Cdecl,
                &[],
                |g| {
                    c1::acc_set_current(g);
                    Out::None
                },
            );
            // MenuManager_TakeSingleRecord
            let mm = t.zeros(0x5b80);
            for (cap, pool, used, count) in [
                (0x15e0, 0x15e4, 0x2bf8, 0x2c50),
                (0x4238, 0x423c, 0x5838, 0x5840),
            ] {
                let records = t.random(&mut rng, 0x14);
                t.w32(mm + pool, records);
                let c = [0, 3, 0x8000_0004, 4][rng.below(4) as usize];
                t.w32(mm + cap, c);
                let taken = [3, 4, rng.below(5)][rng.below(3) as usize];
                t.w32(mm + used, taken);
                t.w32(mm + count, rng.below(8));
            }
            let mut chain = rng.below(3);
            if rng.below(4) == 0 {
                // the list store lands on the used word: the record is read
                // again through the stored value (small, to stay in the pool)
                chain = 0;
                let records = t.zeros(0x14);
                for k in 0..5 {
                    let v = rng.below(5);
                    t.w32(records + 4 * k, v);
                }
                t.w32(mm + 0x15e4, records);
                t.w32(mm + 0x15e0, 4);
                t.w32(mm + 0x2bf8, rng.below(4));
                t.w32(mm + 0x2c50, 0x2bf8 / 4);
            }
            t.check(
                &format!("{w} TakeSingleRecord {chain}"),
                0x0067_c210,
                Conv::Thiscall(mm),
                &[chain],
                |g| u(c1::take_single_record(g, mm, chain)),
            );
            // MenuManager_findNameIndex
            let n = rng.below(7);
            for k in 0..n {
                let s = texts[rng.below(texts.len() as u32) as usize];
                t.w32(table + 4 * k, s);
            }
            let this = t.zeros(0x5848);
            t.w32(this + 0x5844, n);
            let wanted = texts[rng.below(texts.len() as u32) as usize];
            t.check(
                &format!("{w} findNameIndex of {n}"),
                0x0067_e310,
                Conv::Thiscall(this),
                &[wanted],
                |g| u(c1::find_name_index(g, this, wanted)),
            );
            // PArray_reset16
            let arr = t.zeros(8);
            let count = [0x10, 0, 3, 0x8000_0003, 0x8000_0010, 17][rng.below(6) as usize];
            t.w32(arr, count);
            let old = t.random(&mut rng, 0x60);
            t.w32(arr + 4, old);
            let block = match rng.below(4) {
                0 => 0,
                1 => old,
                _ => t.random(&mut rng, 0x40),
            };
            t.ret(r3::ALIGNED_ALLOC, block);
            t.check(
                &format!("{w} reset16 {count:#x} block {block:#x}"),
                0x0068_bc60,
                Conv::Thiscall(arr),
                &[],
                |g| {
                    c1::parray_reset16(g, arr);
                    Out::None
                },
            );
            // PArray8_appendInto
            let arr = t.zeros(8);
            let n = rng.below(4);
            let word = [n, n | 0x8000_0000][rng.below(2) as usize];
            t.w32(arr, word);
            let block = t.random(&mut rng, 0x20);
            let old = if n == 0 && rng.below(2) == 0 {
                0
            } else {
                block
            };
            t.w32(arr + 4, old);
            let dst = match rng.below(4) {
                0 => 0,
                1 => old,
                _ => t.zeros(0x28),
            };
            let value = t.random(&mut rng, 8);
            t.check(
                &format!("{w} append8 {word:#x} old {old:#x} dst {dst:#x}"),
                0x006a_2790,
                Conv::Thiscall(arr),
                &[dst, value],
                |g| u(c1::parray8_append_into(g, arr, dst, value)),
            );
            // op_texels_mip
            let mut a: [u32; 7] = std::array::from_fn(|_| rng.next());
            a[2] = [0, 1, 2, rng.next()][rng.below(4) as usize];
            a[3] = [0, 1, 2, 3][rng.below(4) as usize];
            t.check(
                &format!("{w} texels mip {}x{}", a[2], a[3]),
                0x00a3_84d0,
                Conv::Cdecl,
                &a,
                |g| {
                    c1::texels_mip(g, a);
                    Out::None
                },
            );
            // the texture-animation records
            let info = t.random(&mut rng, 0x24);
            if rng.below(3) == 0 {
                t.w32(info + 0xc, 0);
            }
            let flags = rng.word();
            t.check(
                &format!("{w} info dtor {flags:#x}"),
                0x006a_fd60,
                Conv::Thiscall(info),
                &[flags],
                |g| u(c1::texture_animation_info_dtor(g, info, flags)),
            );
            let info = t.random(&mut rng, 0x24);
            if rng.below(3) == 0 {
                t.w32(info + 0xc, 0);
            }
            t.check(
                &format!("{w} info char dtor"),
                0x006a_f7f0,
                Conv::Thiscall(info),
                &[],
                |g| {
                    c1::texture_animation_info_char_dtor(g, info);
                    Out::None
                },
            );
            let manager = t.random(&mut rng, 0x84);
            t.check(
                &format!("{w} manager ctor"),
                0x006a_f370,
                Conv::Thiscall(manager),
                &[],
                |g| u(c1::texture_animation_manager_ctor(g, manager)),
            );
            let material = t.random(&mut rng, 0x20);
            t.check(
                &format!("{w} material dtor"),
                0x006a_f690,
                Conv::Thiscall(material),
                &[],
                |g| {
                    c1::texture_animation_material_dtor(g, material);
                    Out::None
                },
            );
            let material = t.random(&mut rng, 0x20);
            let src = if rng.below(4) == 0 {
                material
            } else {
                t.random(&mut rng, 0x20)
            };
            let copy = [0, rng.next(), rng.next()][rng.below(3) as usize];
            t.ret(c1::COPY_NEW_A, copy);
            if rng.below(3) == 0 {
                let old: u32 = Mem::read(&t.b, material + 0x14);
                t.ret(c1::COPY_NEW_B, old);
            }
            t.check(
                &format!("{w} material assign"),
                0x006a_faf0,
                Conv::Thiscall(material),
                &[src],
                |g| u(c1::texture_animation_material_assign(g, material, src)),
            );
            // TOAdpMenuIconItemAbmap
            let record = t.random(&mut rng, 0x18);
            t.ret(c1::MS_GET_ROM_ITEM, record);
            let strings = rng.next();
            t.w32(strings_cell, strings);
            let args: [u32; 5] = std::array::from_fn(|_| rng.word());
            let ebp = t.ebp_for(5);
            t.check_framed(
                &format!("{w} icon item"),
                0x008f_0700,
                Conv::Cdecl,
                &args,
                ebp - 4,
                &[(0, 4)],
                |g| u(c1::adp_menu_icon_item(g, ebp - 4, args, &mut { wide })),
            );
            // FFEscMenu_refreshParameterAndSpecialPageEnableState
            let menu = rng.next();
            let fading = [0, 1, rng.next()][rng.below(3) as usize];
            t.g32(c1::MOVIE_FADING.sym, fading);
            let map = rng.word();
            t.g32(c1::CURRENT_MAP_NO.sym, map);
            let flag = [0, 1, 0x1_0000, rng.next()][rng.below(4) as usize];
            t.ret(c1::GET_SCENARIO_FLAG, flag);
            let calm = [0, 1, 2][rng.below(3) as usize];
            t.ret(c1::IS_ETERNAL_CALM_SCENE, calm);
            let ebp = t.ebp_for(0);
            let frame = ebp - 0x10;
            t.w32(frame + 4, 0);
            t.w32(frame + 0xc, 0);
            t.check_framed(
                &format!("{w} esc refresh {fading:#x} {flag:#x} {calm}"),
                0x006f_c9b0,
                Conv::Thiscall(menu),
                &[],
                frame,
                &[(0, 4), (8, 4)],
                |g| u(c1::esc_refresh_page_state(g, frame, menu)),
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

/// A word array (`+0` count, `+4` capacity, `+8` storage of 16 words)
/// with a small count and capacity (-1 too); `null` allows an empty one
/// without storage.
fn word_array(t: &mut Twins, rng: &mut Rng, null: bool) -> u32 {
    let out = t.zeros(0xc);
    let empty = null && rng.below(3) == 0;
    let n = if empty { 0 } else { rng.below(4) };
    t.w32(out, n);
    t.w32(out + 4, [0, 1, 3, 8, u32::MAX][rng.below(5) as usize]);
    let data = if empty { 0 } else { t.zeros(0x40) };
    t.w32(out + 8, data);
    out
}

/// The FMOD event collectors, the character cache's delayed-release
/// flags and the escape menu's Iggy string arrays.
#[test]
fn c1_events_cache_strings() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_c102);
        let stubs: [(u32, bool, u32, u32); 2] = [
            (c1::WORD_ARRAY_RESERVE, true, 1, 1),
            (c1::EVENT_TREE_GET_EVENT, true, 1, 1),
        ];
        for &(sym, this, n, pops) in &stubs {
            t.stub(sym, this, n, pops);
        }
        // the tree find and the project's event getter: logging their
        // pointers for the framed test of the group, quiet for the bank
        // (whose group call runs in a frame of its own)
        let find_log = t.stub_code(c1::SFX_EVENT_TREE_FIND, true, 1, 1);
        let find_quiet = t.stub_code(c1::SFX_EVENT_TREE_FIND + 1, true, 0, 1);
        let event_cell = t.zeros(4);
        let write = [Write {
            arg: Some(3),
            offset: 0,
            cell: event_cell,
        }];
        let get_log = t.stub_out(0x0bad_0024, false, 4, 4, &write);
        let get_quiet = t.stub_out(0x0bad_0025, false, 3, 4, &write);
        let vtable = t.zeros(0x30);
        let wait = t.stub_code(fr::WAIT_FOR_SINGLE_OBJECT, false, 2, 2);
        t.g32(fr::WAIT_FOR_SINGLE_OBJECT, wait);
        let release = t.stub_code(fr::RELEASE_MUTEX, false, 1, 1);
        t.g32(fr::RELEASE_MUTEX, release);
        let iggy: [(u32, u32); 5] = [
            (c1::IGGY_REF_CREATE_ARRAY, 2),
            (c1::IGGY_PATH_FROM_REF, 3),
            (c1::IGGY_PATH_MAKE_ARRAY_REF, 3),
            (c1::IGGY_PATH_SET_ARRAY_INDEX, 2),
            (c1::IGGY_SET_STRING_UTF8, 5),
        ];
        for &(slot, n) in &iggy {
            let f = t.stub_code(slot, false, n, n);
            t.g32(slot, f);
        }
        let texts: Vec<u32> = ["", "x", "Video", "Brightness"]
            .iter()
            .map(|s| {
                let mut b = s.as_bytes().to_vec();
                b.push(0);
                t.alloc(&b)
            })
            .collect();
        for i in 0..40 {
            let w = format!("{name} {i}");
            let log = t.log;
            t.w32(log, 0);
            for &(sym, ..) in &stubs {
                let v = rng.word();
                t.ret(sym, v);
            }
            for tag in [
                c1::SFX_EVENT_TREE_FIND,
                c1::SFX_EVENT_TREE_FIND + 1,
                0x0bad_0024,
                0x0bad_0025,
                fr::WAIT_FOR_SINGLE_OBJECT,
                fr::RELEASE_MUTEX,
            ] {
                let v = rng.next();
                t.ret(tag, v);
            }
            for &(slot, _) in &iggy {
                let v = rng.next();
                t.ret(slot, v);
            }
            let ev = rng.next();
            t.w32(event_cell, ev);
            // a group: a project, its node of ids, an out array
            let project = t.zeros(4);
            t.w32(project, vtable);
            let group = t.zeros(0x10);
            t.w32(group, [0, project, project, project][rng.below(4) as usize]);
            let node = t.zeros(0x20);
            let count = [0, 1, 2, 4][rng.below(4) as usize];
            t.w32(node + 0x10, count);
            let ids = t.random(&mut rng, 0x10);
            t.w32(node + 0x18, ids);
            let found = [0, node, node, 0xffff_fff0][rng.below(4) as usize];
            // no storage only where at most one word lands in slot 0
            let out = word_array(&mut t, &mut rng, count <= 1);
            // SfxEventGroup_appendEvents, framed
            t.jump(c1::SFX_EVENT_TREE_FIND, find_log);
            t.w32(vtable + 0x24, get_log);
            t.ret(c1::SFX_EVENT_TREE_FIND, found);
            let key = rng.word();
            let ebp = t.ebp_for(2);
            t.check_framed(
                &format!("{w} group events {count} found {found:#x}"),
                0x0071_0ae0,
                Conv::Thiscall(group),
                &[key, out],
                ebp + 8,
                &[(0, 8)],
                |g| {
                    put_args(g, ebp + 8, &[key, out]);
                    Out::U8(c1::append_group_events(g, group, ebp + 8))
                },
            );
            // SfxBank_appendEventById: its own event, else the group's
            t.jump(c1::SFX_EVENT_TREE_FIND, find_quiet);
            t.w32(vtable + 0x24, get_quiet);
            t.ret(c1::SFX_EVENT_TREE_FIND + 1, found);
            let bank = t.zeros(0xc);
            let tree_word = [0, rng.next()][rng.below(2) as usize];
            t.w32(bank + 4, tree_word);
            let fallback = [0, group][rng.below(2) as usize];
            t.w32(bank + 8, fallback);
            let out = word_array(&mut t, &mut rng, fallback == 0);
            let event = [0, rng.next()][rng.below(2) as usize];
            t.ret(c1::EVENT_TREE_GET_EVENT, event);
            let id = rng.word();
            t.check(
                &format!("{w} bank event {tree_word:#x} {event:#x}"),
                0x0070_fa80,
                Conv::Thiscall(bank),
                &[id, out],
                |g| Out::U8(c1::append_event_by_id(g, bank, id, out)),
            );
            // ChrResourceCacheManager_enableChrDelayRelease
            let head = t.zeros(0x20);
            t.w8(head + 0xd, 1);
            let n = rng.below(6) as usize;
            let nodes: Vec<u32> = (0..n).map(|_| t.zeros(0x20)).collect();
            for &nd in &nodes {
                let entry = t.zeros(0x14);
                let v = rng.below(2);
                t.w32(entry + 8, v);
                let v = rng.next();
                t.w32(entry + 0xc, v);
                let v = rng.below(2);
                t.w32(entry + 0x10, v);
                t.w32(nd + 0x14, entry);
            }
            let root = tree(&mut t, &nodes, head, head);
            t.w32(head + 4, root);
            t.w32(head, nodes.first().copied().unwrap_or(head));
            t.w32(head + 8, nodes.last().copied().unwrap_or(head));
            let mgr = t.zeros(0x20);
            t.w32(mgr + 0x10, head);
            let holder = t.zeros(4);
            let mutex = rng.next();
            t.w32(holder, mutex);
            t.w32(mgr + 0x18, holder);
            t.g32(c1::CLUSTER_MANAGER.sym, mgr);
            let cache = t.zeros(4);
            let state = [2, 2, 0, 1][rng.below(4) as usize];
            t.w32(cache, state);
            t.check(
                &format!("{w} delay release {state} of {n}"),
                0x0069_ae80,
                Conv::Thiscall(cache),
                &[],
                // in another state the original leaves the caller's eax
                // (its one caller's is `this`, which the port returns)
                |g| match c1::enable_chr_delay_release(g, cache) {
                    r if state == 2 => u(r),
                    r => {
                        assert_eq!(r, cache);
                        Out::None
                    }
                },
            );
            // IggyStringArray_ctor
            let n = [0, 1, 2, 3, u32::MAX][rng.below(5) as usize];
            let names = t.zeros(0x10);
            for k in 0..4 {
                let s = texts[rng.below(texts.len() as u32) as usize];
                t.w32(names + 4 * k, s);
            }
            let this = t.random(&mut rng, 8);
            let root = rng.next();
            let ebp = t.ebp_for(3);
            t.check_framed(
                &format!("{w} string array {n:#x}"),
                0x006f_cbc0,
                Conv::Thiscall(this),
                &[root, names, n],
                ebp - 0x34,
                &[(0x30, 4)],
                |g| {
                    u(c1::iggy_string_array_ctor(
                        g,
                        ebp - 0x34,
                        this,
                        [root, names, n],
                    ))
                },
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

// ---- continuation round, batch 2 (ffx_glue_other::h4_c2)

/// A cluster map of `keys` (sorted) under a fresh head, each node's entry
/// (`+0x14`) with a random cluster, references, pending flag and lock;
/// the head and the nodes in order.
fn cluster_map(t: &mut Twins, rng: &mut Rng, keys: &[u32]) -> (u32, Vec<u32>) {
    let head = t.zeros(0x20);
    t.w8(head + 0xd, 1);
    let nodes: Vec<u32> = keys.iter().map(|_| t.zeros(0x20)).collect();
    for (&nd, &k) in nodes.iter().zip(keys) {
        t.w32(nd + 0x10, k);
        let entry = t.zeros(0x18);
        let cluster = [0, rng.next(), rng.next()][rng.below(3) as usize];
        t.w32(entry, cluster);
        let refs = [0, 1, 2, u32::MAX][rng.below(4) as usize];
        t.w32(entry + 4, refs);
        let pending = [0, 0, 1][rng.below(3) as usize];
        t.w32(entry + 0xc, pending);
        let locked = [0, 0, 1][rng.below(3) as usize];
        t.w8(entry + 0x14, locked);
        t.w32(nd + 0x14, entry);
    }
    let root = tree(t, &nodes, head, head);
    t.w32(head + 4, root);
    t.w32(head, nodes.first().copied().unwrap_or(head));
    t.w32(head + 8, nodes.last().copied().unwrap_or(head));
    (head, nodes)
}

/// The cluster manager's three releases, its map's range erase, the
/// named-pair append and the `PArray<PInputAction>` descriptor.
#[test]
fn c2_clusters_maps_pairs() {
    use ffx_prender::registry;
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_c201);
        let stubs: [(u32, bool, u32, u32); 15] = [
            (r3::DJB_HASH, false, 1, 0),
            (c2::PCLUSTER_DTOR, true, 0, 0),
            (mn::PHYRE_FREE, false, 1, 0),
            (c2::CLUSTER_SLOTS_GIVE, true, 1, 1),
            (c2::CLUSTER_MAP_ERASE, true, 2, 2),
            (c2::CLUSTER_MAP_CLEAR, true, 0, 0),
            (c2::COPY_NAMED_PAIRS, false, 3, 0),
            (c2::NAMED_PAIR_COPY, true, 1, 1),
            (c2::FREE_OWNED_ARRAY, false, 2, 0),
            (registry::PBASE_DESCRIPTOR, false, 0, 0),
            (registry::GLOBAL_NAMESPACE, false, 0, 0),
            (registry::DESCRIPTOR_BASE_CTOR, true, 6, 6),
            (registry::SET_DEFAULT_BUFFER, true, 1, 1),
            (registry::SET_VALIDATION_BUFFER, true, 1, 1),
            (registry::SET_WRITE_MASK_BUFFER, true, 1, 1),
        ];
        for &(sym, this, n, pops) in &stubs {
            t.stub(sym, this, n, pops);
        }
        let wait = t.stub_code(mn::WAIT_FOR_SINGLE_OBJECT.sym, false, 2, 2);
        t.g32(mn::WAIT_FOR_SINGLE_OBJECT.sym, wait);
        let release = t.stub_code(mn::RELEASE_MUTEX.sym, false, 1, 1);
        t.g32(mn::RELEASE_MUTEX.sym, release);
        for i in 0..40 {
            let w = format!("{name} {i}");
            let log = t.log;
            t.w32(log, 0);
            for &(sym, ..) in &stubs {
                let v = rng.next();
                t.ret(sym, v);
            }
            for tag in [mn::WAIT_FOR_SINGLE_OBJECT.sym, mn::RELEASE_MUTEX.sym] {
                let v = rng.next();
                t.ret(tag, v);
            }
            let count = rng.below(7) as usize;
            let mut keys: Vec<u32> = (0..count).map(|_| rng.below(20)).collect();
            keys.sort_unstable();
            let (head, nodes) = cluster_map(&mut t, &mut rng, &keys);
            let mgr = t.zeros(0x20);
            t.w32(mgr + 0x10, head);
            let holder = t.zeros(4);
            let mutex = rng.next();
            t.w32(holder, mutex);
            t.w32(mgr + 0x18, holder);
            // the two releases by name
            for (sym, force) in [(0x0069_b310u32, true), (0x0069_be90, false)] {
                // mostly a key of the map
                let hash = if keys.is_empty() || rng.below(4) == 0 {
                    rng.below(22)
                } else {
                    keys[rng.below(count as u32) as usize]
                };
                t.ret(r3::DJB_HASH, hash);
                let text = rng.next();
                let ebp = t.ebp_for(1);
                t.check_framed(
                    &format!("{w} release {sym:#x} {hash} of {keys:?}"),
                    sym,
                    Conv::Thiscall(mgr),
                    &[text],
                    ebp + 8,
                    &[(0, 4)],
                    |g| {
                        put_args(g, ebp + 8, &[text]);
                        u(if force {
                            c2::force_release_pcluster(g, mgr, ebp + 8)
                        } else {
                            c2::release_pcluster_by_name(g, mgr, ebp + 8)
                        })
                    },
                );
            }
            // the release by cluster: one of the map's, or another
            let cluster = if nodes.is_empty() || rng.below(4) == 0 {
                rng.next()
            } else {
                let nd = nodes[rng.below(count as u32) as usize];
                let entry: u32 = Mem::read(&t.b, nd + 0x14);
                Mem::read(&t.b, entry)
            };
            let ebp = t.ebp_for(1);
            t.check_framed(
                &format!("{w} release cluster {cluster:#x} of {count}"),
                0x0069_bdb0,
                Conv::Thiscall(mgr),
                &[cluster],
                ebp + 8,
                &[(0, 4)],
                |g| {
                    put_args(g, ebp + 8, &[cluster]);
                    u(c2::release_pcluster(g, mgr, ebp + 8))
                },
            );
            // the range erase: all, or an in-order range
            let map = t.zeros(4);
            t.w32(map, head);
            let (first, last) = if nodes.is_empty() || rng.below(4) == 0 {
                (nodes.first().copied().unwrap_or(head), head)
            } else {
                let from = rng.below(count as u32) as usize;
                let to = from + rng.below((count - from) as u32 + 1) as usize;
                (nodes[from], nodes.get(to).copied().unwrap_or(head))
            };
            let out = t.zeros(4);
            let ebp = t.ebp_for(3);
            t.check_framed(
                &format!("{w} erase range {first:#x}..{last:#x}"),
                0x0069_af70,
                Conv::Thiscall(map),
                &[out, first, last],
                ebp - 4,
                &[(0, 4)],
                |g| {
                    u(c2::cluster_map_erase_range(
                        g,
                        ebp - 4,
                        map,
                        [out, first, last],
                    ))
                },
            );
            // the named-pair append
            let arr = t.zeros(8);
            let n = rng.below(4);
            let word = [n, n | 0x8000_0000][rng.below(2) as usize];
            t.w32(arr, word);
            let old = t.random(&mut rng, 0x20);
            t.w32(arr + 4, old);
            let dst = match rng.below(4) {
                0 => 0,
                1 => old,
                _ => t.zeros(0x28),
            };
            let value = rng.next();
            t.check(
                &format!("{w} named pairs {word:#x} dst {dst:#x}"),
                0x0069_b6e0,
                Conv::Thiscall(arr),
                &[dst, value],
                |g| u(c2::named_pair_append_into(g, arr, dst, value)),
            );
            // the descriptor
            let this = t.random(&mut rng, 0x98);
            t.check(
                &format!("{w} input action descriptor"),
                0x009d_fb30,
                Conv::Thiscall(this),
                &[],
                |g| {
                    u(registry::bridge::descriptor_ctor(
                        g,
                        this,
                        c2::INPUT_ACTION_ARRAY_DESCRIPTOR,
                    ))
                },
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

/// A stub at `sym` (thiscall, one word, ret 4) that answers its node
/// argument's `+0x1c` word: the sound tree's successor.
fn stub_successor(t: &mut Twins, sym: u32) {
    let rest = t.stub_code(sym, true, 1, 1);
    let cell = t.cells[&sym];
    let mut c = vec![0x8b, 0x44, 0x24, 0x04, 0x8b, 0x40, 0x1c, 0xa3];
    c.extend(cell.to_le_bytes()); // mov eax, [esp+4]; mov eax, [eax+0x1c]; mov [cell], eax
    c.push(0xe9);
    let at = t.cave_next;
    c.extend(rest.wrapping_sub(at + 17).to_le_bytes()); // jmp rest
    let f = t.code(&c);
    t.jump(sym, f);
}

/// The FMOD bank's destructor and its event map's, the two id-array
/// tree inserts, `FmodSfx`'s stop-all and pause walks.
#[test]
fn c2_fmod_banks_walks() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_c202);
        let stubs: [(u32, bool, u32, u32); 21] = [
            (fr::PRINT_TO_OUTPUT, false, 3, 0),
            (fr::ERRCHECK, false, 3, 0),
            (c2::SLEEP_MS, false, 1, 0),
            (c2::DESTROY_EVENTS, true, 0, 0),
            (fr::OPERATOR_DELETE, false, 1, 0),
            (c2::TREE_ORPHAN_ALL, true, 0, 0),
            (c2::FREE_LIST_RESET, true, 0, 0),
            (c2::FREE_LIST_DTOR, true, 0, 0),
            (c2::TREE_CLEAR, true, 0, 0),
            (c2::FREE_LIST_DEALLOCATE, true, 1, 1),
            (c2::TREE_POST_INSERT, true, 1, 1),
            (c2::USER_FREE_THUNK, false, 1, 0),
            (c2::SFX_ID_TREE[0], true, 1, 1),
            (c2::SFX_ID_TREE[1], true, 1, 1),
            (c2::ID_TREE[0], true, 1, 1),
            (c2::ID_TREE[1], true, 1, 1),
            (c2::SFX_EVENT_STOP, true, 1, 1),
            (c2::SFX_EVENT_SET_PAUSED, true, 1, 1),
            (0x0bad_0028, false, 1, 1),
            (0x0bad_0000, false, 1, 1),
            (c2::FREE_LIST_ALLOCATE_BLOCK, true, 1, 1),
        ];
        let mut project_methods = [0; 2];
        for (k, &(sym, this, n, pops)) in stubs.iter().enumerate() {
            if (18..20).contains(&k) {
                project_methods[k - 18] = t.stub_code(sym, this, n, pops);
            } else if k == 20 {
                // the refill puts a node on the free list
                let cell = t.zeros(4);
                t.cells.insert(0x0bad_f00d, cell);
                t.stub_at(
                    sym,
                    this,
                    n,
                    pops,
                    &[Write {
                        arg: None,
                        offset: 0,
                        cell,
                    }],
                );
            } else {
                t.stub(sym, this, n, pops);
            }
        }
        stub_successor(&mut t, c2::TREE_SUCCESSOR);
        let vtable = t.zeros(0x30);
        t.w32(vtable + 0x28, project_methods[0]);
        t.w32(vtable, project_methods[1]);
        for i in 0..40 {
            let w = format!("{name} {i}");
            let log = t.log;
            t.w32(log, 0);
            for &(sym, ..) in &stubs {
                let v = rng.next();
                t.ret(sym, v);
            }
            // the bank and its event map
            let bank = t.zeros(0xc);
            let project = t.zeros(4);
            t.w32(project, vtable);
            t.w32(bank, [0, project, project][rng.below(3) as usize]);
            let map = t.random(&mut rng, 0x40);
            t.w32(bank + 4, [0, map][rng.below(2) as usize]);
            t.w32(bank + 8, [0, rng.next()][rng.below(2) as usize]);
            t.check(
                &format!("{w} bank dtor"),
                0x0070_f930,
                Conv::Thiscall(bank),
                &[],
                |g| {
                    c2::sfx_bank_dtor(g, bank);
                    Out::None
                },
            );
            let map = t.random(&mut rng, 0x40);
            t.check(
                &format!("{w} event map dtor"),
                0x0071_0180,
                Conv::Thiscall(map),
                &[],
                |g| {
                    c2::sfx_event_map_dtor(g, map);
                    Out::None
                },
            );
            // the two tree inserts
            for (sym, callees) in [
                (0x0070_cde0u32, c2::SFX_ID_TREE),
                (0x0071_0ba0, c2::ID_TREE),
            ] {
                let tree = t.random(&mut rng, 0x24);
                let node = t.random(&mut rng, 0x20);
                let free = [0, node, node][rng.below(3) as usize];
                t.w32(tree + 0x18, free);
                let refill = [0, t.random(&mut rng, 0x20)][rng.below(2) as usize];
                let cell = t.cells[&0x0bad_f00d];
                t.w32(cell, refill);
                let dup = [0, 1, 0x100][rng.below(3) as usize];
                t.ret(callees[1], dup);
                let key = t.random(&mut rng, 4);
                let array = rng.next();
                t.check(
                    &format!("{w} tree insert {sym:#x} free {free:#x} dup {dup}"),
                    sym,
                    Conv::Thiscall(tree),
                    &[key, array],
                    |g| u(c2::id_tree_insert_array(g, callees, tree, key, array)),
                );
            }
            // the sound walks
            let this = t.zeros(0xd0);
            let tree = this + 0x64;
            let n = rng.below(5) as usize;
            let nodes: Vec<u32> = (0..n).map(|_| t.zeros(0x20)).collect();
            for (k, &nd) in nodes.iter().enumerate() {
                let id = [0x7f, 0x80, 0x100, 5, u32::MAX, 0x81][rng.below(6) as usize];
                t.w32(nd + 0xc, id);
                let events = t.zeros(0x10);
                for e in 0..4 {
                    let v = [0, rng.next()][rng.below(2) as usize];
                    t.w32(events + 4 * e, v);
                }
                t.w32(nd + 0x10, rng.below(5));
                t.w32(nd + 0x18, events);
                t.w32(nd + 0x1c, nodes.get(k + 1).copied().unwrap_or(tree));
            }
            match nodes.first() {
                Some(&first) => {
                    // the leftmost chain: one node above the first
                    if rng.below(2) == 0 {
                        t.w32(tree + 0xc, first);
                        t.w32(first, tree);
                    } else {
                        let top = t.zeros(0x20);
                        t.w32(tree + 0xc, top);
                        t.w32(top, first);
                        t.w32(first, tree);
                    }
                }
                None => t.w32(tree + 0xc, tree),
            }
            let keep = t.zeros(0x10);
            for k in 0..3 {
                let v = [0x80, 0x100, 0x81, 5][rng.below(4) as usize];
                t.w32(keep + 4 * k, v);
            }
            t.w32(keep + 4 * rng.below(4), u32::MAX);
            t.w32(keep + 0xc, u32::MAX);
            let immediate = rng.word();
            t.check(
                &format!("{w} stop all of {n}"),
                0x0070_d930,
                Conv::Thiscall(this),
                &[keep, immediate],
                |g| {
                    c2::stop_all_non_blocking(g, this, keep, immediate);
                    Out::None
                },
            );
            let paused = [0, 1, 0x100, 0x101][rng.below(4) as usize];
            t.w8(this, [0, 1][rng.below(2) as usize]);
            t.w32(this + 0xc8, [0, rng.next()][rng.below(2) as usize]);
            t.check(
                &format!("{w} pause {paused:#x} of {n}"),
                0x0070_d0c0,
                Conv::Thiscall(this),
                &[paused],
                |g| {
                    c2::sfx_pause(g, this, paused);
                    Out::None
                },
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

// ---- continuation round, batch 3 (ffx_glue_other::h4_c3)

/// `FmodVoice`'s stop, data release and state update, the sync manager's
/// asynchronous change, the shape-field effect's initialisation, the menu
/// geometry's texture-unload flag.
#[test]
fn c3_voice_sync_effects_menu() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_c301);
        let stubs: [(u32, bool, u32, u32); 15] = [
            (c3::EVENT_SET_CALLBACK, false, 3, 3),
            (c3::EVENT_STOP, false, 2, 2),
            (fr::ERRCHECK, false, 3, 0),
            (c3::VOICE_CLEAR, true, 0, 0),
            (c3::VOICE_UPDATE_PLAY, true, 1, 1),
            (c3::SYNC_RELEASE_DATA, true, 0, 0),
            (r3::ASYNC_LOAD_GET, false, 0, 0),
            (c3::REGIST_ASYNC_TASK, false, 7, 0),
            (c3::CREATE_SHAPE_FIELD, false, 5, 0),
            (c3::VFX_SETUP, false, 2, 0),
            (c3::VFX_RENDER_DATA, false, 2, 0),
            (c3::VFX_DESTROY, false, 2, 0),
            (r3::DJB_HASH, false, 1, 0),
            (0x0bad_0128, false, 1, 1),
            (0x0bad_0100, false, 1, 1),
        ];
        let mut methods = [0; 2];
        for (k, &(sym, this, n, pops)) in stubs.iter().enumerate() {
            if k >= 13 {
                methods[k - 13] = t.stub_code(sym, this, n, pops);
            } else {
                t.stub(sym, this, n, pops);
            }
        }
        // the event's state: logged with its pointer for the framed stop,
        // without it inside the release (its frame is the original's own)
        let state_cell = t.zeros(4);
        let write = [Write {
            arg: Some(1),
            offset: 0,
            cell: state_cell,
        }];
        let state_log = t.stub_out(fr::FMOD_GET_STATE, false, 2, 2, &write);
        let state_quiet = t.stub_out(fr::FMOD_GET_STATE + 1, false, 1, 2, &write);
        // the clock: moves 0x5000 us (20 ms) on each call, edx:eax
        let clock = t.stub_code(c3::TIME_US, false, 0, 0);
        let lo = t.cells[&c3::TIME_US];
        let hi = t.zeros(4);
        let mut c = vec![0x81, 0x05];
        c.extend(lo.to_le_bytes());
        c.extend(0x5000u32.to_le_bytes()); // add dword [lo], 0x5000
        c.extend([0x83, 0x15]);
        c.extend(hi.to_le_bytes());
        c.push(0); // adc dword [hi], 0
        c.extend([0x8b, 0x15]);
        c.extend(hi.to_le_bytes()); // mov edx, [hi]
        c.push(0xe9);
        let at = t.cave_next;
        c.extend(clock.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes());
        let f = t.code(&c);
        t.jump(c3::TIME_US, f);
        let time = |g: &mut G, f: u32| {
            let r = g.0.call(f, Conv::Cdecl, &[]).unwrap();
            u64::from(r.edx) << 32 | u64::from(r.eax)
        };
        // the sync manager at a fixed place, so the sleep can set its flag
        let sync = t.zeros(0x48);
        let sleep = t.stub_code(c3::SLEEP_MS, false, 1, 0);
        let mut c = vec![0xc7, 0x05];
        c.extend((sync + 0x20).to_le_bytes());
        c.extend(1u32.to_le_bytes()); // mov dword [sync+0x20], 1
        c.push(0xe9);
        let at = t.cave_next;
        c.extend(sleep.wrapping_sub(at + 15).to_le_bytes());
        let f = t.code(&c);
        t.jump(c3::SLEEP_MS, f);
        stub_successor(&mut t, c3::TREE_SUCCESSOR);
        // a manager above 2^31 (FFX.exe is large-address-aware): its
        // address goes to the loader sign-extended
        let high = 0x9000_0000u32;
        t.a.map_zeroed(high, 0x1000).unwrap();
        t.b.map_zeroed(high, 0x1000).unwrap();
        let vtable = t.zeros(0x30);
        t.w32(vtable + 0x28, methods[0]);
        t.w32(vtable, methods[1]);
        let tree = t.rebase(c3::MENU_TEXTURE_TREE);
        for i in 0..40 {
            let w = format!("{name} {i}");
            let log = t.log;
            t.w32(log, 0);
            for &(sym, ..) in &stubs {
                let v = rng.next();
                t.ret(sym, v);
            }
            for tag in [fr::FMOD_GET_STATE, fr::FMOD_GET_STATE + 1, c3::SLEEP_MS] {
                let v = rng.next();
                t.ret(tag, v);
            }
            let state = [0, 8, 0xff, 7, 0x108][rng.below(5) as usize];
            t.w32(state_cell, state);
            let now = rng.next();
            t.w32(lo, now);
            let now_hi = rng.below(3);
            t.w32(hi, now_hi);
            // FmodVoice_stop, framed
            let voice = t.random(&mut rng, 0x90);
            if rng.below(4) == 0 {
                t.w32(voice + 0x14, 0);
            }
            t.jump(fr::FMOD_GET_STATE, state_log);
            let ebp = t.ebp_for(0);
            t.check_framed(
                &format!("{w} voice stop {state:#x}"),
                0x0070_b6b0,
                Conv::Thiscall(voice),
                &[],
                ebp - 4,
                &[(0, 4)],
                |g| u(c3::voice_stop(g, ebp - 4, voice)),
            );
            // FmodVoice_releaseData (the original FmodVoice_stop inside)
            t.jump(fr::FMOD_GET_STATE, state_quiet);
            let voice = t.random(&mut rng, 0x90);
            if rng.below(3) == 0 {
                t.w32(voice + 0x14, 0);
            }
            let projects = t.zeros(4 * 21);
            for k in 0..21 {
                if rng.below(2) == 0 {
                    let p = t.zeros(4);
                    t.w32(p, vtable);
                    t.w32(projects + 4 * k, p);
                }
            }
            t.w32(voice + 0x18, projects);
            let words = t.random(&mut rng, 4 * 21);
            t.w32(voice + 0xc, words);
            t.check(
                &format!("{w} voice release"),
                0x0070_b0b0,
                Conv::Thiscall(voice),
                &[],
                |g| u(c3::voice_release_data(g, voice)),
            );
            // FmodVoice_updateState, several cases
            for _ in 0..6 {
                let voice = t.random(&mut rng, 0x90);
                if rng.below(5) == 0 {
                    t.w32(voice + 0x14, 0);
                }
                let v = [0, 1, 1, 2][rng.below(4) as usize];
                t.w32(voice + 0x40, v);
                let v = [0, 1, 0][rng.below(3) as usize];
                t.w32(voice + 0x48, v);
                let v = [0, 1, 1, 2][rng.below(4) as usize];
                t.w32(voice + 0x54, v);
                let v = [0, rng.below(300), rng.below(300), u32::MAX][rng.below(4) as usize];
                t.w32(voice + 0x6c, v);
                if rng.below(3) == 0 {
                    t.w32(voice + 0x70, 0);
                    t.w32(voice + 0x74, [0, 0, 1][rng.below(3) as usize]);
                } else {
                    // a start shortly before now
                    let back = rng.below(0x3_0000);
                    t.w32(voice + 0x70, now.wrapping_sub(back));
                    t.w32(voice + 0x74, now_hi);
                }
                if rng.below(4) == 0 {
                    // a playing voice with no start yet and a negative
                    // limit: an elapsed time of 0 is not past it
                    t.w32(voice + 0x40, 1);
                    t.w32(voice + 0x54, 0);
                    t.w32(voice + 0x6c, u32::MAX);
                    t.w32(voice + 0x70, 0);
                    t.w32(voice + 0x74, 0);
                }
                let arg = rng.word();
                t.check(
                    &format!("{w} voice update"),
                    0x0070_b750,
                    Conv::Thiscall(voice),
                    &[arg],
                    |g| u(c3::voice_update_state(g, voice, arg, &mut { time })),
                );
            }
            // SyncManager_syncDataChangeAsync
            let sync = if rng.below(3) == 0 {
                t.w32(high + 0x20, 1);
                high
            } else {
                sync
            };
            let id = [0, 1, 7, 500, 501, u32::MAX, 3][rng.below(7) as usize];
            let current = [0, id, 3][rng.below(3) as usize];
            t.w32(sync + 0xc, current);
            let n = [0, 1, 3, u32::MAX][rng.below(4) as usize];
            t.w32(sync + 0x18, n);
            let list = t.zeros(0x10);
            for k in 0..3 {
                let v = [7, 3, 1, 9][rng.below(4) as usize];
                t.w32(list + 4 * k, v);
            }
            t.w32(sync + 0x40, list);
            let (id, n) = if rng.below(2) == 0 {
                // a listed id: the change goes through
                let id = [1, 7, 500][rng.below(3) as usize];
                t.w32(list, id);
                // a negative count lists nothing
                let n = [1, u32::MAX][rng.below(2) as usize];
                t.w32(sync + 0x18, n);
                t.w32(sync + 0xc, 0);
                (id, n)
            } else {
                (id, n)
            };
            // the sleep sets the low manager's flag; the high one never waits
            let v = if sync == high { 1 } else { rng.below(2) };
            t.w32(sync + 0x20, v);
            let v = rng.next();
            t.w32(sync + 8, v);
            t.check(
                &format!("{w} sync change {id} of {n:#x}"),
                0x006f_b080,
                Conv::Thiscall(sync),
                &[id],
                |g| u(c3::sync_data_change_async(g, sync, id)),
            );
            // InitVFXRenderDataShapeField
            let data = t.zeros(0x98);
            let n = [0, 1, 2, 3, u32::MAX][rng.below(5) as usize];
            t.w32(data + 0x90, n);
            let records = t.random(&mut rng, 3 * 0x6c);
            t.w32(data + 0x94, records);
            let rd = [0, data, data][rng.below(3) as usize];
            t.ret(c3::VFX_RENDER_DATA, rd);
            let ok = [0, 1, 1][rng.below(3) as usize];
            t.ret(c3::VFX_SETUP, ok);
            let made = [0, 1, 1, 1][rng.below(4) as usize];
            t.ret(c3::CREATE_SHAPE_FIELD, made);
            let args: [u32; 5] = std::array::from_fn(|_| rng.word());
            t.check(
                &format!("{w} shape field {made} {ok} {rd:#x} {n}"),
                0x0071_13d0,
                Conv::Cdecl,
                &args,
                |g| u(c3::init_vfx_render_data_shape_field(g, args)),
            );
            // MenuDynamicGeometry_setTextureInMaterialUnloaded
            let n = rng.below(5) as usize;
            let nodes: Vec<u32> = (0..n).map(|_| t.zeros(0x20)).collect();
            for (k, &nd) in nodes.iter().enumerate() {
                let record = t.zeros(8);
                let v = rng.below(6);
                t.w32(record, v);
                t.w32(record + 4, 1);
                t.w32(nd + 0xc, record);
                t.w32(nd + 0x1c, nodes.get(k + 1).copied().unwrap_or(tree));
            }
            match nodes.first() {
                Some(&first) => {
                    let top = t.zeros(0x20);
                    t.w32(tree + 0xc, [first, top][rng.below(2) as usize]);
                    t.w32(top, first);
                    t.w32(first, tree);
                }
                None => t.w32(tree + 0xc, tree),
            }
            let hash = rng.below(6);
            t.ret(r3::DJB_HASH, hash);
            let text = rng.next();
            t.check(
                &format!("{w} texture unloaded {hash} of {n}"),
                0x006a_3740,
                Conv::Stdcall,
                &[text],
                |g| u(c3::set_texture_in_material_unloaded(g, text)),
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

/// A `std::string` holding `text` (inline below 16 bytes of capacity,
/// else in a block); its address.
fn std_string(t: &mut Twins, text: &[u8], heap: bool) -> u32 {
    let s = t.zeros(0x18);
    let mut bytes = text.to_vec();
    bytes.push(0);
    if heap {
        let p = t.alloc(&bytes);
        t.w32(s, p);
        t.w32(s + 0x14, 0x1f);
    } else {
        t.write(s, &bytes);
        t.w32(s + 0x14, 0xf);
    }
    t.w32(s + 0x10, text.len() as u32);
    s
}

/// The FMOD bank's two event-group constructors, the Iggy callback
/// dispatch and `std::string::rfind`.
#[test]
fn c3_groups_callbacks_strings() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_c302);
        let stubs: [(u32, bool, u32, u32); 7] = [
            (c3::FREE_LIST_CTOR, true, 4, 4),
            (c3::GROUP_LOAD, true, 1, 1),
            (c3::TREE_LOAD, true, 1, 1),
            (c3::TREE_ORPHAN_ALL, true, 0, 0),
            (c3::FREE_LIST_RESET, true, 0, 0),
            (c3::GET_DEREF_U32, true, 0, 0),
            (0x0bad_0300, false, 2, 0),
        ];
        let mut handler = 0;
        for &(sym, this, n, pops) in &stubs {
            if sym == 0x0bad_0300 {
                handler = t.stub_code(sym, this, n, pops);
            } else {
                t.stub(sym, this, n, pops);
            }
        }
        // std_string_begin and memcmp, computed and logged
        let begin = t.stub_code(c3::STRING_BEGIN, true, 0, 0);
        let cell = t.cells[&c3::STRING_BEGIN];
        let mut c = vec![
            0x83, 0x79, 0x14, 0x10, 0x89, 0xc8, 0x72, 0x02, 0x8b, 0x01, 0xa3,
        ];
        c.extend(cell.to_le_bytes()); // cmp [ecx+0x14], 16; mov eax, ecx; jb; mov eax, [ecx]; mov [cell], eax
        c.push(0xe9);
        let at = t.cave_next;
        c.extend(begin.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes());
        let f = t.code(&c);
        t.jump(c3::STRING_BEGIN, f);
        let cmp = t.stub_code(c3::MEMCMP, false, 3, 0);
        let cell = t.cells[&c3::MEMCMP];
        let mut c = vec![
            0x56, 0x57, 0x8b, 0x74, 0x24, 0x0c, 0x8b, 0x7c, 0x24, 0x10, 0x8b, 0x4c, 0x24, 0x14,
            0x31, 0xc0, 0xe3, 0x05, 0xf3, 0xa6, 0x0f, 0x95, 0xc0, 0xa3,
        ];
        c.extend(cell.to_le_bytes()); // repe cmpsb; setne al; mov [cell], eax
        c.extend([0x5f, 0x5e, 0xe9]);
        let at = t.cave_next;
        c.extend(cmp.wrapping_sub(at + c.len() as u32 + 4).to_le_bytes());
        let f = t.code(&c);
        t.jump(c3::MEMCMP, f);
        let group_cell = t.zeros(4);
        let get_group = t.stub_out(
            0x0bad_0308,
            false,
            4,
            4,
            &[Write {
                arg: Some(3),
                offset: 0,
                cell: group_cell,
            }],
        );
        let vtable = t.zeros(0x10);
        t.w32(vtable + 8, get_group);
        let names: Vec<u32> = [&b""[..], b"a", b"onPress", b"onRelease", b"\xe9t\xe9"]
            .iter()
            .map(|s| {
                let mut b = s.to_vec();
                b.push(0);
                t.alloc(&b)
            })
            .collect();
        for i in 0..40 {
            let w = format!("{name} {i}");
            let log = t.log;
            t.w32(log, 0);
            for &(sym, ..) in &stubs {
                let v = rng.next();
                t.ret(sym, v);
            }
            let r = [0, 0, 1, 0x80][rng.below(4) as usize];
            t.ret(0x0bad_0308, r);
            let group = rng.next();
            t.w32(group_cell, group);
            // the two group constructors
            for (sym, loop_group) in [(0x0071_0780u32, true), (0x0070_ff50, false)] {
                let this = t.random(&mut rng, 0x30);
                let project = if rng.below(4) == 0 {
                    0
                } else {
                    let p = t.zeros(4);
                    t.w32(p, vtable);
                    p
                };
                let arg = rng.word();
                t.check(
                    &format!("{w} group ctor {sym:#x} {project:#x} {r:#x}"),
                    sym,
                    Conv::Thiscall(this),
                    &[project, arg],
                    |g| {
                        u(if loop_group {
                            c3::sfx_event_group_ctor(g, this, project, arg)
                        } else {
                            c3::fmod_event_tree_ctor(g, this, project, arg)
                        })
                    },
                );
            }
            // the Iggy callback dispatch, several cases
            for _ in 0..6 {
                let player = rng.below(3);
                t.ret(c3::GET_DEREF_U32, [player, 9][rng.below(2) as usize]);
                let text = names[rng.below(names.len() as u32) as usize];
                let record = t.zeros(0x20);
                t.w32(record, text);
                let n = rng.below(4);
                let handlers = t.zeros(0x40);
                for k in 0..4 {
                    let at = handlers + 0x10 * k;
                    let v = [0, 1, 1][rng.below(3) as usize];
                    t.w8(at, v);
                    // the hash of one of the names, or another
                    let hashes = [
                        0x1505u32,
                        0x0002_b606,
                        0x6abf_52af,
                        0x13f6_e4a3,
                        0x0b86_5feb,
                        rng.next(),
                    ];
                    let v = hashes[rng.below(6) as usize];
                    t.w32(at + 4, v);
                    t.w32(at + 8, handler);
                    let v = rng.next();
                    t.w32(at + 0xc, v);
                }
                let manager = t.zeros(0x18);
                t.w32(manager + 0x10, [n, n | 0x8000_0000][rng.below(2) as usize]);
                t.w32(manager + 0x14, handlers);
                let a = rng.next();
                t.check(
                    &format!("{w} dispatch {n}"),
                    0x00a2_6100,
                    Conv::Thiscall(manager),
                    &[a, player, record],
                    |g| u(c3::dispatch_callback(g, manager, player, record)),
                );
            }
            // std::string::rfind
            let hay: &[&[u8]] = &[
                b"",
                b"a",
                b"abcab",
                b"chr_c001_ab_c001",
                b"xxxxxxxxxxxxxxxxxxxxab",
            ];
            let text = hay[rng.below(hay.len() as u32) as usize];
            let heap = text.len() >= 16 || rng.below(2) == 0;
            let s = std_string(&mut t, text, heap);
            let needles: &[&[u8]] = &[b"", b"a", b"ab", b"c001", b"zz", b"abcab"];
            let needle = needles[rng.below(needles.len() as u32) as usize];
            let mut nb = needle.to_vec();
            nb.push(0);
            let np = t.alloc(&nb);
            let pos = [0, 1, 3, 8, 30, u32::MAX][rng.below(6) as usize];
            let n = needle.len() as u32;
            t.check(
                &format!("{w} rfind {needle:?} in {text:?} at {pos:#x}"),
                0x0063_8340,
                Conv::Thiscall(s),
                &[np, pos, n],
                |g| u(c3::string_rfind(g, s, [np, pos, n])),
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

/// `FmodMusic`'s three slot clears.
#[test]
fn c3_music_clears() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_c303);
        t.stub(fr::ERRCHECK, false, 3, 0);
        let free = t.stub_code(0x0bad_0388, false, 3, 3);
        let vtable = t.zeros(0x10);
        t.w32(vtable + 8, free);
        for i in 0..24 {
            let w = format!("{name} {i}");
            let log = t.log;
            t.w32(log, 0);
            for tag in [fr::ERRCHECK, 0x0bad_0388] {
                let v = rng.next();
                t.ret(tag, v);
            }
            let music = t.random(&mut rng, 0x30);
            let which = rng.below(3);
            t.w32(music + 0x2c, which);
            for off in [0, 4] {
                let project = if rng.below(5) == 0 {
                    0
                } else {
                    let p = t.zeros(4);
                    t.w32(p, vtable);
                    p
                };
                t.w32(music + off, project);
            }
            // one slot past the end too, which slot 0xb6 must not touch
            let slots = t.random(&mut rng, ((c3::MUSIC_SLOTS + 1) * 0x3c) as usize);
            for k in 0..=c3::MUSIC_SLOTS {
                let at = slots + 0x3c * k;
                let v = [0, rng.next(), rng.next()][rng.below(3) as usize];
                t.w32(at, v);
                let v = [0, 0, 1][rng.below(3) as usize];
                t.w32(at + 4, v);
                let v = [0, 1, 2][rng.below(3) as usize];
                t.w32(at + 8, v);
            }
            t.w32(music + 0x10, slots);
            let slot = [0, 1, 0xb5, 0xb6, rng.below(0xb6), u32::MAX][rng.below(6) as usize];
            t.check(
                &format!("{w} music clear {slot:#x}"),
                0x0070_8720,
                Conv::Thiscall(music),
                &[slot],
                |g| u(c3::music_clear(g, music, slot)),
            );
            t.check(
                &format!("{w} music clear all"),
                0x0070_8810,
                Conv::Thiscall(music),
                &[],
                |g| u(c3::music_clear_all(g, music)),
            );
            for k in 0..c3::MUSIC_SLOTS {
                let v = [0, rng.next()][rng.below(2) as usize];
                t.w32(slots + 0x3c * k, v);
            }
            t.check(
                &format!("{w} music force clear all"),
                0x0070_8af0,
                Conv::Thiscall(music),
                &[],
                |g| {
                    c3::music_force_clear_all(g, music);
                    Out::None
                },
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

// ---- continuation round, batch 4 (ffx_glue_other::h4_c4)

/// The extended blur's render, the voice's event check, the six-value
/// Flash call, the string replace, the wave-texture unload and the
/// vertex-info move.
#[test]
fn c4_blur_voice_menu() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_c401);
        let stubs: [(u32, bool, u32, u32); 14] = [
            (m::POST_PROCESS_INSTANCE, false, 0, 0),
            (c4::COPY_BUFFER, true, 6, 6),
            (c4::BLUR_PASS, true, 2, 2),
            (fr::ERRCHECK, false, 3, 0),
            (c4::VOICE_READ, true, 2, 2),
            (c4::CALL_AS3, true, 5, 5),
            (fr::MEMCPY, false, 3, 0),
            (fr::MEMSET, false, 3, 0),
            (c4::SET_UI_UNLOADED, true, 1, 1),
            (c4::UNIFY_FILENAME, false, 3, 0),
            (c4::SET_MATERIAL_UNLOADED, false, 1, 1),
            (c4::FORCE_UNLOAD_MENU_TEXTURE, true, 1, 1),
            (0x0bad_0408, false, 3, 3),
            (0x0bad_0478, false, 2, 0),
        ];
        let mut codes = [0; 2];
        for (k, &(sym, this, n, pops)) in stubs.iter().enumerate() {
            match k {
                12 | 13 => codes[k - 12] = t.stub_code(sym, this, n, pops),
                _ => t.stub(sym, this, n, pops),
            }
        }
        let [free_event, strstr] = codes;
        t.g32(c4::STRSTR.sym, strstr);
        let group_vtable = t.zeros(0x10);
        t.w32(group_vtable + 8, free_event);
        let state_cell = t.zeros(4);
        t.stub_at(
            fr::FMOD_GET_STATE,
            false,
            2,
            2,
            &[Write {
                arg: Some(1),
                offset: 0,
                cell: state_cell,
            }],
        );
        let table = t.rebase(c4::WAVE_TEXTURES);
        for i in 0..40 {
            let w = format!("{name} {i}");
            let log = t.log;
            t.w32(log, 0);
            for &(sym, ..) in &stubs {
                let v = rng.next();
                t.ret(sym, v);
            }
            // BlurEffectEx_render
            let blur = t.random(&mut rng, 0x40);
            t.w8(blur + 0x28, [0, 1, 1, 1][rng.below(4) as usize]);
            t.w32(blur + 0x2c, [0, 1, 2][rng.below(3) as usize]);
            t.w32(blur + 0x3c, [0, 1, 2][rng.below(3) as usize]);
            for off in [0xc, 0x18] {
                let rt = t.random(&mut rng, 8);
                t.w8(rt + 4, [0, 1][rng.below(2) as usize]);
                t.w32(blur + off, rt);
            }
            t.check(
                &format!("{w} blur render"),
                0x006c_4cf0,
                Conv::Thiscall(blur),
                &[],
                |g| u(c4::blur_ex_render(g, blur)),
            );
            // FmodVoice_checkEvent
            let voice = t.random(&mut rng, 0x80);
            if rng.below(4) == 0 {
                t.w32(voice + 0x14, 0);
            }
            if rng.below(3) != 0 {
                let group = t.zeros(4);
                t.w32(group, group_vtable);
                t.w32(voice + 0x10, group);
            } else {
                t.w32(voice + 0x10, 0);
            }
            t.w32(voice + 0x60, [0, 1, 1, 2][rng.below(4) as usize]);
            t.w32(voice + 0x5c, [0, rng.next()][rng.below(2) as usize]);
            let r = [0, 0x1e][rng.below(2) as usize];
            t.ret(fr::FMOD_GET_STATE, r);
            let v = rng.next();
            t.w32(state_cell, v);
            let system = rng.next();
            let ebp = t.ebp_for(1);
            t.check_framed(
                &format!("{w} voice check {r:#x}"),
                0x0070_a4a0,
                Conv::Thiscall(voice),
                &[system],
                ebp - 4,
                &[(0, 4)],
                |g| {
                    c4::voice_check_event(g, ebp - 4, voice, system);
                    Out::None
                },
            );
            // the six-value Flash call
            let args: [u32; 26] = std::array::from_fn(|_| rng.next());
            let values: [u32; 24] = std::array::from_fn(|k| args[k + 2]);
            let ebp = t.ebp_for(26);
            t.check_framed(
                &format!("{w} flash 6"),
                0x006f_caf0,
                Conv::Cdecl,
                &args,
                ebp - 0x64,
                &[(0, 0x60)],
                |g| u(c4::call_flash_6(g, ebp - 0x64, args[0], args[1], &values)),
            );
            // the string replace
            let texts: &[&[u8]] = &[b"Video %s here", b"%s", b"none", b"a%sb%s", b"yes"];
            let text = texts[rng.below(texts.len() as u32) as usize];
            // junk after the terminator: a moved terminator must be written
            let mut buf = text.to_vec();
            buf.push(0);
            buf.resize(0x40, 0xaa);
            let tp = t.alloc(&buf);
            let froms: &[&[u8]] = &[b"%s", b"s", b"e"];
            let pattern = froms[rng.below(froms.len() as u32) as usize];
            let mut fb = pattern.to_vec();
            fb.push(0);
            let from = t.alloc(&fb);
            let tos: &[&[u8]] = &[b"\0", b"x\0", b"ab\0", b"long text\0"];
            let to = t.alloc(tos[rng.below(tos.len() as u32) as usize]);
            let found = text.windows(pattern.len()).position(|x| x == pattern);
            t.ret(0x0bad_0478, found.map_or(0, |k| tp + k as u32));
            t.check(
                &format!("{w} replace in {text:?}"),
                0x0070_2590,
                Conv::Cdecl,
                &[tp, from, to],
                |g| Out::U8(c4::string_replace_first(g, tp, from, to)),
            );
            // the wave-texture unload
            for k in 0..70u32 {
                let v = [0, rng.next()][rng.below(4).min(1) as usize];
                t.w32(table + 4 * k, v);
            }
            // the original reads **(ui + 0x2bd4) for the geometry's ecx
            let ui = t.zeros(0x2bd8);
            let geometry = t.random(&mut rng, 4);
            t.w32(ui + 0x2bd4, geometry);
            t.g32(c4::GRAPHIC_UI.sym, ui);
            let scene = rng.next();
            t.g32(c4::PHYRE_SCENE.sym, scene);
            let ebp = t.ebp_for(0);
            t.check_framed(
                &format!("{w} wave unload"),
                0x0067_efa0,
                Conv::Cdecl,
                &[],
                ebp - 0x104,
                &[(0, 0x100)],
                |g| {
                    c4::unload_main_menu_wave_texture(g, ebp - 0x104);
                    Out::None
                },
            );
            // the vertex-info move
            let record = |t: &mut Twins, rng: &mut Rng| {
                let r = t.zeros(0x98);
                let info = t.random(rng, 0x24);
                for (off, n) in [(0x14, 16), (0x1c, 3), (0x10, 12), (0x18, 8), (0xc, 12)] {
                    let a = t.random(rng, 4 * n);
                    t.w32(info + off, a);
                }
                t.w32(r + 0x94, info);
                r
            };
            let from = record(&mut t, &mut rng);
            let to = record(&mut t, &mut rng);
            let v = rng.next();
            t.g32(c4::VERTEX_INFO_DEFAULT.sym, v);
            t.check(
                &format!("{w} vertex move"),
                0x0067_dc70,
                Conv::Stdcall,
                &[from, to],
                |g| u(c4::vertex_info_move(g, from, to)),
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

// ---- continuation round 2, batch 1 (ffx_glue_other::h4_d1)

/// Resets the log and sets every listed stub's result to a random word.
fn fresh_round(t: &mut Twins, rng: &mut Rng, tags: &[u32]) {
    let log = t.log;
    t.w32(log, 0);
    for &tag in tags {
        let v = rng.next();
        t.ret(tag, v);
    }
}

/// Mostly 0, else a random word: results that let a caller go on.
fn mostly_zero(rng: &mut Rng) -> u32 {
    [0, 0, 0, rng.next()][rng.below(4) as usize]
}

/// The particle fill-buffer jobs, the Iggy manager's render and menu
/// insert, `GDraw`'s render-thread setup.
#[test]
fn d1_jobs_iggy() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_d101);
        let mut stubs: Vec<(u32, bool, u32, u32)> =
            d1::FILL_PART.iter().map(|&f| (f, false, 1, 0)).collect();
        stubs.extend([
            (d1::QUEUE_JOB, true, 5, 5),
            (d1::SET_COLOR_TARGET, true, 2, 2),
            (d1::SET_DEPTH_TARGET, true, 1, 1),
            (d1::BEGIN_SCENE, true, 1, 1),
            (d1::END_SCENE, true, 1, 1),
            (d1::MENU_DRAW, true, 0, 0),
            (d1::END_DRAW, true, 0, 0),
            (d1::MENU_SET_MANAGER, true, 1, 1),
            (d1::MENU_SET_VIEWPORT, true, 4, 4),
            (d1::PHYRE_MALLOC, false, 2, 0),
            (d1::MENU_ARRAY_APPEND, true, 2, 2),
            (d1::SYNC_RENDER, true, 0, 0),
        ]);
        for &(sym, this, n, pops) in &stubs {
            t.stub(sym, this, n, pops);
        }
        let mp3 = t.stub_code(0x0bad_c864, false, 0, 0);
        let sound = t.stub_code(0x0bad_c860, false, 0, 0);
        let flush = t.stub_code(0x0bad_0008, true, 0, 0);
        t.g32(d1::IGGY_INSTALL_MP3.sym, mp3);
        t.g32(d1::IGGY_USE_DIRECT_SOUND.sym, sound);
        let result_cell = t.zeros(4);
        // the queued callback writes the record's result word, or (a
        // second stub) leaves the 0 the setup put there
        let writer = t.stub_at(
            d1::QUEUE_CALLBACK,
            true,
            2,
            2,
            &[Write {
                arg: Some(1),
                offset: 8,
                cell: result_cell,
            }],
        );
        let plain = t.stub_code(0x0bad_5880, true, 2, 2);
        let mut tags: Vec<u32> = stubs.iter().map(|s| s.0).collect();
        tags.extend([0x0bad_c864, 0x0bad_c860, 0x0bad_0008, d1::QUEUE_CALLBACK]);
        tags.push(0x0bad_5880);
        // six part arrays of 64 parts, kinds 0..6 and some outside
        let mut buffers = [0u32; 6];
        for b in &mut buffers {
            *b = t.zeros(64 * d1::PART_SIZE as usize);
        }
        for (k, &b) in buffers.iter().enumerate() {
            t.g32(d1::FILL_BUFFERS + 4 * k as u32, b);
        }
        let renderer_vtable = t.zeros(0xc);
        t.w32(renderer_vtable + 8, flush);
        for i in 0..150 {
            let w = format!("{name} {i}");
            fresh_round(&mut t, &mut rng, &tags);
            for &b in &buffers {
                for p in 0..64 {
                    let kind = [0, 1, 2, 3, 4, 5, 6, 7, 0xff, 0x80][rng.below(10) as usize];
                    t.w8(b + p * d1::PART_SIZE, kind);
                }
            }
            // fillBuffer
            let start = t.zeros(4);
            let end = t.zeros(4);
            let s = [0, 1, 3, u32::MAX][rng.below(4) as usize];
            let e = [0, 1, 5, 9, 2][rng.below(5) as usize];
            t.w32(start, s);
            t.w32(end, e);
            let parts = buffers[rng.below(6) as usize] + d1::PART_SIZE;
            t.check(
                &format!("{w} fill {s} {e}"),
                d1::FILL_BUFFER,
                Conv::Cdecl,
                &[parts, start, end],
                |g| u(d1::fill_buffer(g, parts, start, end)),
            );
            // the kick
            // totals at the limits: weights 0 / 499 / 500 / negative,
            // requests 49 / 50 / negative (an empty fill)
            let request_total = [0, 3, 49, 50, 0x8000_0000][rng.below(5) as usize];
            let weight_total = [0, 0, 1, 499, 500, 0x8000_0000][rng.below(6) as usize];
            for k in 0..6u32 {
                let (r, wt) = if k == 5 {
                    (request_total, weight_total)
                } else {
                    (0, 0)
                };
                let (r, wt) = if k == 4 && request_total == 50 {
                    (20, wt)
                } else if k == 5 && request_total == 50 {
                    (30, wt)
                } else {
                    (r, wt)
                };
                t.g32(d1::FILL_REQUESTS + 4 * k, r);
                t.g32(d1::FILL_WEIGHTS + 4 * k, wt);
            }
            let sched = rng.next();
            t.g32(d1::JOB_SCHEDULER.sym, sched);
            let inline = [0, 1, rng.next()][rng.below(3) as usize];
            let ebp = t.ebp_for(1);
            // the start word is the original's push-ecx slot: the
            // caller's ecx on the job path, 0 before the inline fills (a
            // port that leaves the junk there fills other parts)
            t.w32(ebp - 4, 0xa5a4_a7a6);
            t.check_framed(
                &format!("{w} kick {inline:#x}"),
                0x0071_edb0,
                Conv::Cdecl,
                &[inline],
                ebp - 4,
                &[],
                |g| u(d1::fill_buffer_kick(g, ebp - 4, inline)),
            );
            // the Iggy manager's render
            let this = t.random(&mut rng, 0x40);
            let on = [0, 1, 0x80, 1][rng.below(4) as usize];
            t.w8(this + 0x30, on);
            if rng.below(4) == 0 {
                t.w32(this + 4, 0);
            }
            let n = rng.below(5);
            let menus = t.zeros(4 * 6);
            for k in 0..n {
                if rng.below(4) != 0 {
                    let menu = t.zeros(0xa0);
                    t.w32(menu + 0x98, rng.below(4));
                    t.w32(menus + 4 * k, menu);
                }
            }
            t.w32(this + 8, n | (rng.below(2) << 31));
            t.w32(this + 0xc, menus);
            t.check(
                &format!("{w} render"),
                0x00a2_67f0,
                Conv::Thiscall(this),
                &[],
                |g| {
                    let r = d1::iggy_manager_render(g, this);
                    if on == 0 { Out::None } else { u(r) }
                },
            );
            // the menu insert
            let menu = [0, rng.next()][rng.below(5).min(1) as usize];
            let this = t.random(&mut rng, 0x30);
            let n = rng.below(4);
            let slots = t.zeros(4 * 5);
            for k in 0..n {
                if rng.below(3) != 0 {
                    let v = rng.next() | 1;
                    t.w32(slots + 4 * k, v);
                }
            }
            t.w32(this + 8, n | (rng.below(2) << 31));
            t.w32(this + 0xc, slots);
            let ebp = t.ebp_for(1);
            t.check_framed(
                &format!("{w} add menu"),
                0x00a2_65f0,
                Conv::Thiscall(this),
                &[menu],
                ebp + 8,
                &[(0, 4)],
                |g| {
                    let r = d1::iggy_manager_add_menu(g, ebp + 8, this, menu);
                    if menu == 0 { Out::None } else { u(r) }
                },
            );
            // GDraw's render setup
            let this = t.random(&mut rng, 8);
            let renderer = t.zeros(4);
            t.w32(renderer, renderer_vtable);
            let audio = t.zeros(4);
            t.w8(audio, [0, 1, 0x80][rng.below(3) as usize]);
            for tag in [0x0bad_0008, d1::SYNC_RENDER, d1::QUEUE_CALLBACK] {
                let v = mostly_zero(&mut rng);
                t.ret(tag, v);
            }
            let v = mostly_zero(&mut rng);
            t.ret(0x0bad_5880, v);
            t.jump(d1::QUEUE_CALLBACK, [writer, plain][rng.below(2) as usize]);
            let v = rng.next();
            t.w32(result_cell, v);
            let arg = rng.next();
            let ebp = t.ebp_for(3);
            t.check_framed(
                &format!("{w} gdraw setup"),
                0x00a3_ba50,
                Conv::Thiscall(this),
                &[renderer, audio, arg],
                ebp - 0xc,
                &[(0, 0xc)],
                |g| {
                    u(d1::gdraw_render_setup(
                        g,
                        ebp - 0xc,
                        this,
                        renderer,
                        audio,
                        arg,
                    ))
                },
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

/// The sync file read and the data directory.
#[test]
fn d1_files() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_d102);
        let stubs: [(u32, bool, u32, u32); 11] = [
            (d1::DBG_PRINTF, false, 1, 0),
            (d1::FIOS_EXISTS, false, 1, 0),
            (d1::FIOS_OPEN, false, 2, 0),
            (d1::FIOS_SEEK, false, 3, 0),
            (d1::ALIGNED_MALLOC, false, 2, 0),
            (d1::FIOS_READ, false, 3, 0),
            (d1::FIOS_CLOSE, false, 1, 0),
            (d1::MEMSET, false, 3, 0),
            (d1::FIOS_UNIFY, false, 3, 0),
            (d1::STREAM_PREFIX, false, 0, 0),
            (d1::PHYRE_PRINTF, false, 3, 0),
        ];
        for &(sym, this, n, pops) in &stubs {
            t.stub(sym, this, n, pops);
        }
        let sprintf = t.stub_code(0x0bad_c394, false, 4, 0);
        t.g32(d1::SPRINTF_S.sym, sprintf);
        let mut tags: Vec<u32> = stubs.iter().map(|s| s.0).collect();
        tags.push(0x0bad_c394);
        for i in 0..40 {
            let w = format!("{name} {i}");
            fresh_round(&mut t, &mut rng, &tags);
            // readSyncFile
            for tag in [d1::FIOS_EXISTS, d1::FIOS_OPEN] {
                let v = [0, rng.next()][rng.below(4).min(1) as usize];
                t.ret(tag, v);
            }
            let path = [0, rng.next()][rng.below(5).min(1) as usize];
            t.check(
                &format!("{w} read sync"),
                0x006f_acf0,
                Conv::Stdcall,
                &[path],
                |g| u(d1::read_sync_file(g, path)),
            );
            // the data directory
            let names: &[&[u8]] = &[b"\0", b"a\0", b"sound/fmod\0", b"data/x.bin\0"];
            let nm_text = names[rng.below(4) as usize];
            let nm = t.alloc(nm_text);
            let prefixes: &[&[u8]] = &[b"\0", b"C:/game/\0", b"pfx\0"];
            let prefix_text = prefixes[rng.below(3) as usize];
            let prefix = t.alloc(prefix_text);
            t.ret(d1::STREAM_PREFIX, prefix);
            let out = rng.next();
            // around the length the warning compares with, and signed
            let len = (nm_text.len() + prefix_text.len() - 2) as u32;
            let size = [
                len.wrapping_sub(1),
                len,
                len + 1,
                0x100,
                u32::MAX,
                0x8000_0000,
            ][rng.below(6) as usize];
            let ebp = t.ebp_for(3);
            t.check_framed(
                &format!("{w} data directory {size:#x}"),
                0x0067_9a10,
                Conv::Cdecl,
                &[nm, out, size],
                ebp - 0x104,
                &[(0, 0x100)],
                |g| u(d1::data_directory(g, ebp - 0x104, nm, out, size)),
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

/// The menu textures' kind dispatch and the Sa sub-menu preload.
#[test]
fn d1_menu_textures() {
    // (routine, arguments) of both tables
    const ROUTINES: [(u32, u32); 26] = [
        (0x0067_f610, 0),
        (0x0067_f230, 0),
        (0x0067_f3c0, 0),
        (0x0067_f9a0, 0),
        (0x0067_fa20, 0),
        (0x0067_ecf0, 0),
        (0x0067_f920, 0),
        (0x0067_ee70, 0),
        (0x0067_efa0, 0),
        (0x0067_f3e0, 0),
        (0x0067_ebc0, 0),
        (0x0067_edc0, 1),
        (0x0067_ee40, 0),
        (0x0067_d370, 1),
        (0x0067_ce50, 1),
        (0x0067_d0b0, 1),
        (0x0067_d670, 0),
        (0x0067_d690, 0),
        (0x0067_c6f0, 0),
        (0x0067_d650, 0),
        (0x0067_cb30, 1),
        (0x0067_cba0, 0),
        (0x0067_d180, 1),
        (0x0067_c8c0, 0),
        (0x0067_c720, 1),
        (0x0067_c8f0, 0),
    ];
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_d103);
        let mut tags = Vec::new();
        for &(sym, n) in &ROUTINES {
            t.stub(sym, true, n, n);
            tags.push(sym);
        }
        for (sym, this, n, pops) in [
            (d1::MEMSET, false, 3, 0),
            (d1::SA_TEXTURE_NAME, false, 2, 0),
            (d1::LOAD_MENU_TEXTURE, true, 1, 1),
        ] {
            t.stub(sym, this, n, pops);
            tags.push(sym);
        }
        for i in 0..60 {
            let w = format!("{name} {i}");
            fresh_round(&mut t, &mut rng, &tags);
            let this = rng.next();
            let kind = rng.below(20).wrapping_sub(2);
            let [a, b] = [rng.next(), rng.next()];
            t.check(
                &format!("{w} unload {kind}"),
                0x0067_f830,
                Conv::Thiscall(this),
                &[kind],
                |g| u(d1::unload_special_menu_texture(g, this, kind)),
            );
            t.check(
                &format!("{w} load {kind}"),
                0x0067_d550,
                Conv::Thiscall(this),
                &[kind, a, b],
                |g| u(d1::load_special_menu_texture(g, this, kind, a, b)),
            );
            if i % 6 == 0 {
                let scene = rng.next();
                t.g32(d1::PHYRE_SCENE.sym, scene);
                let ebp = t.ebp_for(1);
                t.check_framed(
                    &format!("{w} preload"),
                    0x0067_dd70,
                    Conv::Stdcall,
                    &[a],
                    ebp - 0x134,
                    &[(0, 0x130)],
                    |g| u(d1::preload_sa_sub_menu(g, ebp - 0x134)),
                );
            }
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}

/// The distortion target, the extended blur's parameter, the event's
/// play test, the Atel shape's VFX.
#[test]
fn d1_effects_sound_vfx() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_d104);
        let stubs: [(u32, bool, u32, u32); 17] = [
            (d1::DISTORTION_RESET, true, 0, 0),
            (d1::CLUSTER_CREATE, true, 3, 3),
            (d1::SET_DIMENSIONS, true, 4, 4),
            (d1::UPDATE_MAX_LEVEL, true, 0, 0),
            (d1::CREATE_TARGET, true, 1, 1),
            (d1::FIND_PARAM, true, 1, 1),
            (d1::SET_TEXTURE, true, 2, 2),
            (d1::BIND_TEXTURE, true, 1, 1),
            (d1::MEMCPY, false, 3, 0),
            (fr::ERRCHECK, false, 3, 0),
            (d1::EVENT_INFO_INIT, true, 0, 0),
            (d1::CREATE_ATEL_SHP, false, 4, 0),
            (d1::VFX_RENDER_DATA, false, 2, 0),
            (d1::VFX_2D_SETUP, false, 2, 0),
            (d1::UPDATE_ATEL_SHP, false, 5, 0),
            (d1::VFX_2D_UPDATE, false, 2, 0),
            (d1::VFX_DESTROY, false, 2, 0),
        ];
        for &(sym, this, n, pops) in &stubs {
            t.stub(sym, this, n, pops);
        }
        let cells: [u32; 5] = std::array::from_fn(|_| t.zeros(4));
        let [group_cell, n_cell, channel_cell, loops_cell, state_cell] = cells;
        for (sym, n, arg, offset, cell) in [
            (d1::EVENT_GET_CHANNEL_GROUP, 2, 1, 0, group_cell),
            (d1::GROUP_NUM_CHANNELS, 2, 1, 0, n_cell),
            (d1::GROUP_GET_CHANNEL, 3, 2, 0, channel_cell),
            (d1::CHANNEL_LOOP_COUNT, 2, 1, 0, loops_cell),
            (d1::EVENT_GET_INFO, 4, 3, 0x28, state_cell),
        ] {
            t.stub_at(
                sym,
                false,
                n,
                n,
                &[Write {
                    arg: Some(arg),
                    offset,
                    cell,
                }],
            );
        }
        let mut tags: Vec<u32> = stubs.iter().map(|s| s.0).collect();
        tags.extend([d1::GROUP_GET_CHANNEL, d1::GROUP_NUM_CHANNELS]);
        tags.extend([d1::CHANNEL_LOOP_COUNT, d1::EVENT_GET_INFO]);
        for i in 0..150 {
            let w = format!("{name} {i}");
            fresh_round(&mut t, &mut rng, &tags);
            // the distortion target
            let this = t.random(&mut rng, 0x80);
            let owner = t.random(&mut rng, 8);
            t.w32(this + 0x30, owner);
            if rng.below(3) == 0 {
                t.w32(this + 0x2c, 0);
            }
            let tex = t.random(&mut rng, 0x20);
            let tex = [0, tex][rng.below(5).min(1) as usize];
            t.ret(d1::CLUSTER_CREATE, tex);
            let r = mostly_zero(&mut rng);
            t.ret(d1::SET_DIMENSIONS, r);
            let target = t.random(&mut rng, 0x10);
            let target = [0, target][rng.below(5).min(1) as usize];
            t.ret(d1::CREATE_TARGET, target);
            let r = [0, rng.next()][rng.below(3).min(1) as usize];
            t.ret(d1::FIND_PARAM, r);
            let ebp = t.ebp_for(0);
            t.check_framed(
                &format!("{w} distortion target"),
                0x006c_2350,
                Conv::Thiscall(this),
                &[],
                ebp - 0x10,
                &[(0, 0x10)],
                |g| u(d1::distortion_create_target(g, ebp - 0x10, this)),
            );
            // the blur parameter
            let this = t.random(&mut rng, 0x58);
            if rng.below(6) == 0 {
                t.w32(this + 0x54, 0);
            } else {
                let record = t.random(&mut rng, 0xc);
                let kind = [0x42, 0xd9, 0xda, 0xdb, 0xdc, 0x12, 0x43, 0xd8, 0xdd, 0]
                    [rng.below(10) as usize];
                t.w8(record + 2, kind);
                let off = [0, 3, 4, 8, 0x20][rng.below(5) as usize];
                t.write(record + 8, &(off as u16).to_le_bytes());
                let len = [4, 0x10, 0x20][rng.below(3) as usize] | (rng.below(8) << 13);
                t.write(record + 0xa, &(len as u16).to_le_bytes());
                t.w32(this + 0x54, record);
            }
            let holder = t.zeros(8);
            let block = t.random(&mut rng, 0x40);
            t.w32(block, [0x10, 0x14, 0x24, 0x40][rng.below(4) as usize]);
            t.w32(holder + 4, block);
            t.w32(this + 0x20, holder);
            let r = [0, rng.next() | 1][rng.below(2) as usize];
            t.ret(d1::BIND_TEXTURE, r);
            let sampler = [0, rng.next()][rng.below(4).min(1) as usize];
            let texture = [0, rng.next()][rng.below(4).min(1) as usize];
            let ebp = t.ebp_for(2);
            t.check_framed(
                &format!("{w} blur param"),
                0x006c_44e0,
                Conv::Thiscall(this),
                &[sampler, texture],
                ebp - 0x10,
                &[(0, 0x10)],
                |g| {
                    u(d1::blur_set_texture_param(
                        g,
                        ebp - 0x10,
                        this,
                        sampler,
                        texture,
                    ))
                },
            );
            // SfxEvent_isPlaying
            let this = t.random(&mut rng, 0xc);
            t.w32(this, [0, 1, 2, u32::MAX, 1, 1][rng.below(6) as usize]);
            let event = t.random(&mut rng, 8);
            t.w8(event + 4, [0, 1][rng.below(6).min(1) as usize]);
            t.w32(this + 8, event);
            let r = mostly_zero(&mut rng);
            t.ret(d1::EVENT_GET_CHANNEL_GROUP, r);
            for (cell, v) in [
                (group_cell, rng.next()),
                (
                    n_cell,
                    [0, 1, 3, u32::MAX, 2, 0x8000_0001][rng.below(6) as usize],
                ),
                (channel_cell, rng.next()),
                (
                    loops_cell,
                    [u32::MAX, 0, 2, u32::MAX][rng.below(4) as usize],
                ),
                (state_cell, [0, 1, 2][rng.below(3) as usize]),
            ] {
                t.w32(cell, v);
            }
            let ebp = t.ebp_for(0);
            t.check_framed(
                &format!("{w} is playing"),
                0x0070_ec40,
                Conv::Thiscall(this),
                &[],
                ebp - 0x44,
                &[(0, 0x44)],
                |g| Out::U8(d1::sfx_event_is_playing(g, ebp - 0x44, this)),
            );
            // the Atel shape's VFX
            let args: [u32; 5] = std::array::from_fn(|k| match k {
                0 => [rng.next(), 0xffff_ff00, 5][rng.below(3) as usize],
                _ => rng.next(),
            });
            let r = [0, 1, rng.next()][rng.below(3) as usize];
            t.ret(d1::CREATE_ATEL_SHP, r);
            let data = t.zeros(0x98);
            let inner = t.random(&mut rng, 0x28);
            t.w32(data + 0x94, inner);
            let data = [0, data, data][rng.below(3) as usize];
            t.ret(d1::VFX_RENDER_DATA, data);
            let r = [0, 1, 1, 3][rng.below(4) as usize];
            t.ret(d1::VFX_2D_SETUP, r);
            t.check(
                &format!("{w} atel shp"),
                0x0071_11e0,
                Conv::Cdecl,
                &args,
                |g| u(d1::init_vfx_render_data_atel_shp(g, args)),
            );
        }
        assert!(t.log_entries() > 0, "{name}: the stubs ran");
    }
}
