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
use ffx_glue_other::{effect as e, misc as m, sound as s};
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
    /// The call log: a count, then entries of 8 words (tag, `ecx`, six
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
        t.log = t.zeros(0x4000);
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
        assert!(n <= 6);
        let cell = self.zeros(4);
        self.cells.insert(tag, cell);
        let log = self.log;
        let mut c = vec![0xa1];
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.extend([0xc1, 0xe0, 0x05]); // shl eax, 5
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
    /// A stand-in at `sym` that stores `value` at global `global`, logs
    /// its call as [`Twins::stub`] would (tag `sym`, `ecx`, no words) and
    /// returns `ret`.
    fn stub_store(&mut self, sym: u32, global: u32, value: u32, ret: u32) {
        let cell = self.rebase(global);
        let log = self.log;
        let mut c = vec![0xc7, 0x05];
        c.extend(cell.to_le_bytes());
        c.extend(value.to_le_bytes()); // mov dword [global], value
        c.push(0xa1);
        c.extend(log.to_le_bytes()); // mov eax, [log]
        c.extend([0xc1, 0xe0, 0x05]); // shl eax, 5
        c.push(0x05);
        c.extend((log + 4).to_le_bytes()); // add eax, log + 4
        c.extend([0xc7, 0x00]);
        c.extend(sym.to_le_bytes()); // mov [eax], sym
        c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
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
        t.stub_store(e::PHYRE_PRINTF, e::BLUR_EFFECT.global, 0x1357_9bdf, 7);
        t.g32(e::BLUR_EFFECT.global, 0);
        t.check(
            &format!("{name} GetInstance"),
            e::BLUR_EFFECT.get,
            Conv::Cdecl,
            &[],
            |g| u(e::get_instance(g, e::BLUR_EFFECT)),
        );
        t.stub_store(m::GET_SPLIT_SCREEN, 0x00cc_b9d8, 0x0246_8ace, 1);
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
