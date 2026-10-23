//! The sound layer (`ffx_sound`: the PS2 `Snd*` API, the FMOD queue's
//! `i*` entry points and command helpers) against the original, on two
//! oracles started alike: one runs `FFX.exe`'s code, the other the Rust
//! bridge on the same memory; results and every written byte must agree.
//!
//! The callees left original (the `Sqs*` sequencer layer, the FMOD
//! command queue, a few `Tk*` voice checks) are replaced in both oracles
//! by the same recorders: each logs its arguments (and `this`) to a log
//! in the image's `.reloc` pages and returns a value the case chooses, so
//! the comparison also covers which callees run, in which order, with
//! which arguments. Synthetic cases need `FFX_GAME_DIR`; snapshot cases
//! also `FFX_SNAPSHOT` (both kept snapshots).

// a, b: the two oracles; h, f, s: hosts and bridges, as in the crate
// cases are long lists of calls; test values are cut to bytes on purpose
#![allow(
    clippy::many_single_char_names,
    clippy::too_many_lines,
    clippy::cast_possible_truncation
)]

use std::path::PathBuf;
use std::sync::Arc;

use ffx_bridge::{Global, Mem, Value};
use ffx_graphic::host::{self, Host};
use ffx_oracle::{Conv, Oracle, Pages};
use ffx_sound::ffxfmod::bridge::{self as x, callee as xc};
use ffx_sound::fmod::{self, bridge as f};
use ffx_sound::snd::bridge as s;

fn exe() -> Option<PathBuf> {
    let dir = std::env::var_os("FFX_GAME_DIR").map(PathBuf::from);
    if dir.is_none() {
        eprintln!("FFX_GAME_DIR is not set; skipping");
    }
    Some(dir?.join("FFX.exe"))
}

/// The worlds a test runs in: the image alone, and the snapshot when
/// `FFX_SNAPSHOT` is set.
fn worlds() -> Vec<(&'static str, Oracle, Oracle)> {
    worlds_except(&[])
}

/// [`worlds`], with the originals in `keep` left unrecorded (the ones a
/// test compares).
fn worlds_except(keep: &[u32]) -> Vec<(&'static str, Oracle, Oracle)> {
    let Some(exe) = exe() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut a = Oracle::load(&exe).unwrap();
    let mut b = Oracle::load(&exe).unwrap();
    setup(&mut a, &mut b, false, keep);
    out.push(("image", a, b));
    if let Some(snap) = std::env::var_os("FFX_SNAPSHOT") {
        let pages: Arc<Pages> = Pages::read(PathBuf::from(snap)).unwrap();
        let mut a = Oracle::from_snapshot(&exe, &pages).unwrap();
        let mut b = Oracle::from_snapshot(&exe, &pages).unwrap();
        setup(&mut a, &mut b, true, keep);
        out.push(("snapshot", a, b));
    } else {
        eprintln!("FFX_SNAPSHOT is not set; image cases only");
    }
    out
}

struct Xorshift(u32);

impl Xorshift {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }
    fn below(&mut self, n: u32) -> u32 {
        self.next() % n
    }
    /// Small, edge and random values.
    fn word(&mut self) -> u32 {
        match self.below(6) {
            0 => self.below(4),
            1 => u32::MAX - self.below(3),
            2 => 0x8000_0000 | self.below(0x40),
            3 => self.below(0x200),
            _ => self.next(),
        }
    }
}

// ---- the bridge's view of an oracle

struct Run<'a>(&'a mut Oracle);

impl Mem for Run<'_> {
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

impl Host for Run<'_> {
    fn call(&mut self, addr: u32, conv: host::Conv, args: &[u32]) -> u32 {
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
        // the bottom of the emulated stack: no call reaches that deep, and
        // the diff leaves the stacks out, as the originals' frames are
        self.0.stack_range().start + 0x100
    }
}

// ---- recorders

/// Free image bytes for the recorders: the `.reloc` section, which the
/// code never reads.
const CODE: u32 = 0x0270_0000;
/// A stand-in `FmodManager` -> `FfxFmod` -> `FmodMusic` for the image.
const FAKE_FMOD: u32 = 0x0270_e000;
/// The log's write pointer.
const LOG_PTR: u32 = 0x0270_f000;
/// One return value per recorder.
const RETS: u32 = 0x0270_f100;
/// Out values the stdcall recorders write through a pointer argument.
const OUTS: u32 = 0x0270_f800;
/// The language word the localization recorder points at.
const LANG: u32 = 0x0270_fff0;
/// The log: 64-byte entries `{tag, this, args[14]}`.
const LOG: u32 = 0x0271_0000;
const LOG_ENTRIES: u32 = 256;

/// The Snd callees replaced by recorders: (address, stack words,
/// thiscall).
const SND_STUBS: &[(u32, u32, bool)] = &[
    (s::SQS_DATA_SENDING_COMPLETED, 1, false),
    (s::SQS_SE_FORCE_PLAY, 4, false),
    (s::SQS_EE_SOUND_ERROR_STATUS, 0, false),
    (s::SQS_SE_PLAY_STATE, 0, false),
    (s::SQS_SE_BLOCK_PLAYING, 1, false),
    (s::SQS_SE_SEPARATE_PLAYING, 1, false),
    (s::FOOT3D_REQUEST, 3, false),
    (s::SQS_MUSIC_DATA_READ, 1, false),
    (s::SQS_SE_PLAY, 4, false),
    (s::MS_BATTLE_CHECK, 0, false),
    (s::SQS_REPORT_STANDBY_STATUS, 1, false),
    (s::SQS_SE_PITCH_DIRECT, 4, false),
    (s::SQS_SE_PITCH_TRANSLATE, 5, false),
    (s::SQS_SE_SEPARATE_CLEAR, 1, false),
    (s::SQS_SE_PAN_DIRECT, 3, false),
    (s::SQS_SE_PAN_TRANSLATE, 4, false),
    (SQS_SE_STOP, 2, false),
    (s::SQS_SE_SEPARATE_DATA_SEND, 3, false),
    (s::SQS_SE_VOLUME_DIRECT, 3, false),
    (s::SQS_SE_VOLUME_FADE_OUT, 3, false),
    (s::SQS_SE_VOLUME_TRANSLATE, 4, false),
    (s::SQS_IOP_COMMAND_SEND, 0, false),
    (s::TK_CD_READ_SYNC, 1, false),
    (s::TK_REFRESH_HDD_MODE, 0, false),
    (s::SET_HDD_ERR, 1, false),
    (s::SQS_SET_PLAY_MODE, 1, false),
    (s::SQS_STRAIGHT_DIGITAL_MODE, 1, false),
    (s::SQS_SORT_COMMON_WAVE, 1, false),
    (s::SQS_SET_STRAIGHT_STREAM_AREA_SIZE, 1, false),
    (s::SQS_IOP_SOUND_ERROR_STATUS, 0, false),
    (s::SQS_REPORT_IOP_SOUND_STATUS, 1, false),
    (s::SQS_SHOUT_STREAMING_PLAY_STATE, 1, false),
    (s::SQS_BATTLE_WAVE_DATA_SEND, 2, false),
    (s::SQS_SET_STANDBY_WAVE, 0, false),
    (s::SQS_BATTLE_STREAMING_START, 3, false),
    (s::TK_CHECK_PLAY_VOICE, 0, false),
    (s::SQS_SHOUT_STREAMING_START, 3, false),
    (s::I_VOICE_START, 2, false),
    (f::FMOD_FINALIZE, 0, false),
    (f::MOVIE_SET_VOLUME, 1, false),
    (f::FMOD_INITIALIZE, 0, false),
    (fmod::PUSH_COMMAND, 5, true),
    (fmod::REGIST_COMMAND_SYNC, 5, true),
];

/// `SqsSeStop` (`SndStop` jumps to it).
const SQS_SE_STOP: u32 = 0x0081_e7c0;

/// How a recorded callee takes its arguments.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// The caller pops.
    Cdecl,
    /// `this` in `ecx` (logged), the callee pops.
    This,
    /// The callee pops (FMOD's members: `this` the first word).
    Std,
    /// [`Kind::Std`], argument `k` a pointer the recorder writes its out
    /// value through when it returns 0 (not logged: the original passes
    /// its stack).
    StdOut(u32),
}

/// A recorded callee: its `ffx.sym` address, stack words, convention, and
/// the import slot pointed at the recorder (0: a `jmp` at the address).
#[derive(Clone, Copy)]
struct Stub {
    sym: u32,
    words: u32,
    kind: Kind,
    slot: u32,
}

const fn stub(sym: u32, words: u32, kind: Kind) -> Stub {
    Stub {
        sym,
        words,
        kind,
        slot: 0,
    }
}

/// The FMOD layer's callees left original: the objects' methods, the
/// `FfxFmod` methods that create and release FMOD, the helpers, and the
/// FMOD imports (on their import slots).
const FMOD_STUBS: &[Stub] = &[
    stub(xc::MUSIC_CLEAR, 1, Kind::This),
    stub(xc::MUSIC_CLEAR_ALL, 0, Kind::This),
    stub(xc::MUSIC_CROSS_PLAY, 3, Kind::This),
    stub(xc::MUSIC_DATA_SEND, 2, Kind::This),
    stub(xc::MUSIC_INIT_DATA, 1, Kind::This),
    stub(xc::MUSIC_INIT_TEMP_DATA, 1, Kind::This),
    stub(xc::MUSIC_PAUSE_ALL, 1, Kind::This),
    stub(xc::MUSIC_PLAY_STATE, 1, Kind::This),
    stub(xc::MUSIC_READ, 2, Kind::This),
    stub(xc::MUSIC_SET_ENTIRE_VOLUME, 1, Kind::This),
    stub(xc::MUSIC_SET_MUTE, 1, Kind::This),
    stub(xc::MUSIC_START, 1, Kind::This),
    stub(xc::MUSIC_STOP, 1, Kind::This),
    stub(xc::MUSIC_STOP_ALL, 0, Kind::This),
    stub(xc::MUSIC_SWITCH_BGM, 2, Kind::This),
    stub(xc::MUSIC_SYNC_STANDBY, 0, Kind::This),
    stub(xc::MUSIC_VOLUME_DIRECT, 3, Kind::This),
    stub(xc::MUSIC_VOLUME_FADE_OUT, 2, Kind::This),
    stub(xc::MUSIC_VOLUME_TRANSLATE, 3, Kind::This),
    stub(xc::VOICE_CLEAR, 0, Kind::This),
    stub(xc::VOICE_DATA_CHANGE, 2, Kind::This),
    stub(xc::VOICE_INIT_DATA, 2, Kind::This),
    stub(xc::VOICE_INIT_LIST, 0, Kind::This),
    stub(xc::VOICE_INIT_TEMP_DATA, 2, Kind::This),
    stub(xc::VOICE_READ, 2, Kind::This),
    stub(xc::VOICE_RELEASE_DATA, 0, Kind::This),
    stub(xc::VOICE_SET_ENTIRE_VOLUME, 1, Kind::This),
    stub(xc::VOICE_SET_MUTE, 1, Kind::This),
    stub(xc::VOICE_SET_PAUSED, 1, Kind::This),
    stub(xc::VOICE_SET_LANG, 1, Kind::This),
    stub(xc::VOICE_SHOUT_START, 4, Kind::This),
    stub(xc::VOICE_SOUND_STAGE_FLUSH, 1, Kind::This),
    stub(xc::VOICE_START, 2, Kind::This),
    stub(xc::VOICE_STOP, 0, Kind::This),
    stub(xc::VOICE_VOLUME_DIRECT, 2, Kind::This),
    stub(xc::VOICE_VOLUME_TRANSLATE, 2, Kind::This),
    stub(xc::SHOUT_START, 8, Kind::This),
    stub(xc::SHOUT_SET_VOLUME, 1, Kind::This),
    stub(xc::SFX_CAN_START_NEW_EVENT, 1, Kind::This),
    stub(xc::SFX_CREATE_EVENT, 1, Kind::This),
    stub(xc::SFX_DELAY_START, 4, Kind::This),
    stub(xc::SFX_FORCE_START, 3, Kind::This),
    stub(xc::SFX_IS_PLAYING, 1, Kind::This),
    stub(xc::SFX_LOAD_PROJECT, 3, Kind::This),
    stub(xc::SFX_PAUSE, 1, Kind::This),
    stub(xc::SFX_PLAY_BATTLE_STREAMING, 4, Kind::This),
    stub(xc::SFX_SET_ENTIRE_VOLUME, 1, Kind::This),
    stub(xc::SFX_SET_PAN, 2, Kind::This),
    stub(xc::SFX_SET_PAN_TRANSLATE, 3, Kind::This),
    stub(xc::SFX_SET_VOLUME, 2, Kind::This),
    stub(xc::SFX_SET_VOLUME_TRANSLATE, 3, Kind::This),
    stub(xc::SFX_START, 3, Kind::This),
    stub(xc::SFX_STOP, 1, Kind::This),
    stub(xc::SFX_STOP_ALL_NON_BLOCKING, 2, Kind::This),
    stub(xc::SFX_STOP_ALL_NONLOOP, 0, Kind::This),
    stub(xc::SFX_STOP_NON_BLOCKING, 1, Kind::This),
    stub(xc::SFX_UNLOAD_BY_SEQ, 1, Kind::This),
    stub(xc::SFX_UNLOAD_PROJECTS, 2, Kind::This),
    stub(xc::EVENT_IS_PLAYING, 0, Kind::This),
    stub(xc::EVENT_DELAY_START, 3, Kind::This),
    stub(xc::EVENT_START, 2, Kind::This),
    stub(xc::EVENT_SET_PAN, 1, Kind::This),
    stub(xc::EVENT_SET_PAN_TRANSLATE, 2, Kind::This),
    stub(xc::EVENT_SET_VOLUME, 1, Kind::This),
    stub(xc::EVENT_SET_VOLUME_TRANSLATE, 2, Kind::This),
    stub(xc::EVENT_STOP, 1, Kind::This),
    stub(xc::FFX_INIT_EVENT_SYSTEM, 0, Kind::This),
    stub(xc::FFX_SHUT_DOWN, 0, Kind::This),
    stub(xc::FFX_SOUND_LOOP, 0, Kind::This),
    stub(xc::LOCALIZATION, 0, Kind::Cdecl),
    stub(xc::ERRCHECK, 3, Kind::Cdecl),
    stub(xc::PRINT, 3, Kind::Cdecl),
    Stub {
        sym: x::EVENT_GET_STATE,
        words: 2,
        kind: Kind::StdOut(1),
        slot: 0x00b0_c750,
    },
    Stub {
        sym: x::EVENT_SET_PAUSED,
        words: 2,
        kind: Kind::Std,
        slot: 0x00b0_c740,
    },
    Stub {
        sym: x::EVENT_SYSTEM_UPDATE,
        words: 1,
        kind: Kind::Std,
        slot: 0x00b0_c768,
    },
];

/// Every recorded callee.
fn stubs() -> &'static [Stub] {
    static ALL: std::sync::OnceLock<Vec<Stub>> = std::sync::OnceLock::new();
    ALL.get_or_init(|| {
        let mut all: Vec<Stub> = SND_STUBS
            .iter()
            .map(|&(sym, words, this)| {
                stub(sym, words, if this { Kind::This } else { Kind::Cdecl })
            })
            .collect();
        all.extend_from_slice(FMOD_STUBS);
        all
    })
}

/// x86 for a recorder: logs `tag`, `this` (thiscall) and `words` stack
/// arguments, returns `[ret]`, keeping `ecx` and `edx`; an out argument
/// gets `[out]` when `[ret]` is 0 (FMOD answers only on success).
fn recorder(tag: u32, words: u32, kind: Kind, ptr: u32, ret: u32, out: u32) -> Vec<u8> {
    let mut c = vec![0x51, 0x52, 0x8b, 0x15];
    c.extend_from_slice(&ptr.to_le_bytes());
    c.extend_from_slice(&[0xc7, 0x02]);
    c.extend_from_slice(&tag.to_le_bytes());
    if kind == Kind::This {
        c.extend_from_slice(&[0x89, 0x4a, 0x04]);
    }
    for i in 0..words {
        #[allow(clippy::cast_possible_truncation)] // small offsets
        let (from, to) = ((0x0c + 4 * i) as u8, (8 + 4 * i) as u8);
        c.extend_from_slice(&[0x8b, 0x4c, 0x24, from]);
        if kind == Kind::StdOut(i) {
            // when it returns 0 (FMOD_OK): mov eax, [out]; mov [ecx], eax
            c.extend_from_slice(&[0x83, 0x3d]);
            c.extend_from_slice(&ret.to_le_bytes());
            c.extend_from_slice(&[0x00, 0x75, 0x07, 0xa1]);
            c.extend_from_slice(&out.to_le_bytes());
            c.extend_from_slice(&[0x89, 0x01]);
        } else {
            c.extend_from_slice(&[0x89, 0x4a, to]);
        }
    }
    c.extend_from_slice(&[0x83, 0xc2, 0x40, 0x89, 0x15]);
    c.extend_from_slice(&ptr.to_le_bytes());
    c.extend_from_slice(&[0x5a, 0x59, 0xa1]);
    c.extend_from_slice(&ret.to_le_bytes());
    if kind == Kind::Cdecl {
        c.push(0xc3);
    } else {
        #[allow(clippy::cast_possible_truncation)] // at most 8 words
        c.extend_from_slice(&[0xc2, (4 * words) as u8, 0]);
    }
    c
}

fn put<T: Value>(a: &mut Oracle, b: &mut Oracle, addr: u32, v: T) {
    Mem::write(a, addr, v);
    Mem::write(b, addr, v);
}

fn set<T: Value>(a: &mut Oracle, b: &mut Oracle, g: Global<T>, v: T) {
    g.set(a, v);
    g.set(b, v);
}

fn alloc(a: &mut Oracle, b: &mut Oracle, len: usize) -> u32 {
    let x = a.alloc(len).unwrap();
    assert_eq!(b.alloc(len).unwrap(), x);
    x
}

/// Installs the recorders (but for the originals in `keep`) and, on the
/// image, a fake `FmodManager` chain.
fn setup(a: &mut Oracle, b: &mut Oracle, snapshot: bool, keep: &[u32]) {
    for (k, st) in stubs().iter().enumerate() {
        if keep.contains(&st.sym) {
            continue;
        }
        let k = u32::try_from(k).unwrap();
        let code = a.rebase(CODE) + 0x100 * k;
        assert!(code + 0x100 <= a.rebase(FAKE_FMOD));
        let ret = a.rebase(RETS) + 4 * k;
        let out = a.rebase(OUTS) + 4 * k;
        let rec = recorder(st.sym, st.words, st.kind, a.rebase(LOG_PTR), ret, out);
        assert!(rec.len() <= 0x100);
        a.write(code, &rec).unwrap();
        b.write(code, &rec).unwrap();
        if st.slot != 0 {
            let slot = a.rebase(st.slot);
            put(a, b, slot, code);
            continue;
        }
        let at = a.rebase(st.sym);
        let rel = code.wrapping_sub(at + 5);
        let mut jmp = vec![0xe9];
        jmp.extend_from_slice(&rel.to_le_bytes());
        a.write(at, &jmp).unwrap();
        b.write(at, &jmp).unwrap();
    }
    let lang = a.rebase(LANG);
    stub_ret(a, b, xc::LOCALIZATION, lang);
    if !snapshot || f::MANAGER.get(a) == 0 {
        // in the image, so the cases' heap resets keep it
        let mgr = a.rebase(FAKE_FMOD);
        let (ffx, music) = (mgr + 0x100, mgr + 0x200);
        put(a, b, mgr + f::MANAGER_FMOD, ffx);
        put(a, b, ffx + f::FMOD_MUSIC, music);
        set(a, b, f::MANAGER, mgr);
    }
    reset_log(a, b);
}

/// Sets recorder `sym`'s return value.
fn stub_ret(a: &mut Oracle, b: &mut Oracle, sym: u32, v: u32) {
    let k = stubs()
        .iter()
        .position(|s| s.sym == sym)
        .expect("a recorder");
    let at = a.rebase(RETS) + 4 * u32::try_from(k).unwrap();
    put(a, b, at, v);
}

/// Sets recorder `sym`'s out value.
fn stub_out(a: &mut Oracle, b: &mut Oracle, sym: u32, v: u32) {
    let k = stubs()
        .iter()
        .position(|s| s.sym == sym)
        .expect("a recorder");
    let at = a.rebase(OUTS) + 4 * u32::try_from(k).unwrap();
    put(a, b, at, v);
}

/// Random return and out values for every recorder (the localization
/// manager's stays a pointer).
fn stub_rets(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    for st in stubs() {
        if st.sym == xc::LOCALIZATION {
            continue;
        }
        let v = rng.word();
        stub_ret(a, b, st.sym, v);
        let v = rng.word();
        stub_out(a, b, st.sym, v);
    }
}

fn reset_log(a: &mut Oracle, b: &mut Oracle) {
    let log = a.rebase(LOG);
    put(a, b, a.rebase(LOG_PTR), log);
    let zero = vec![0u8; (LOG_ENTRIES * 64) as usize];
    a.write(log, &zero).unwrap();
    b.write(log, &zero).unwrap();
}

/// How many entries the case logged (the same in both, or the diff fails).
fn logged(a: &Oracle) -> u32 {
    (Mem::read::<u32>(a, a.rebase(LOG_PTR)) - a.rebase(LOG)) / 64
}

/// A `ret` in `FFX.exe`'s code (`FfxFmod::getNowPlayMusic`'s): calling it
/// shows the x87 stack as it is.
const RET: u32 = 0x0070_67d6;
/// `fstp st(0); ret` in `FFX.exe`'s code: pops one x87 register.
const FPU_POP: u32 = 0x0068_ab21;

/// Whether `o`'s x87 stack is empty; when not, it is emptied (what an
/// earlier original left there, which the oracle keeps across calls).
fn fpu_empty(o: &mut Oracle) -> bool {
    let (ret, pop) = (o.rebase(RET), o.rebase(FPU_POP));
    let mut empty = true;
    while o.call(ret, Conv::Cdecl, &[]).unwrap().st0.is_some() {
        empty = false;
        o.call(pop, Conv::Cdecl, &[]).unwrap();
    }
    empty
}

/// Runs the original at `sym` on `a` and `bridge` on `b`; compares the
/// result (when the bridge gives one) and every written byte.
fn check(
    what: &str,
    a: &mut Oracle,
    b: &mut Oracle,
    sym: u32,
    conv: Conv,
    args: &[u32],
    bridge: impl FnOnce(&mut Run) -> Option<u32>,
) {
    check_masked(what, a, b, sym, conv, args, u32::MAX, bridge);
}

/// [`check`], comparing only the bits of `eax` in `mask` (a callee that
/// answers in `al`).
#[allow(clippy::too_many_arguments)] // check's, plus the mask
fn check_masked(
    what: &str,
    a: &mut Oracle,
    b: &mut Oracle,
    sym: u32,
    conv: Conv,
    args: &[u32],
    mask: u32,
    bridge: impl FnOnce(&mut Run) -> Option<u32>,
) {
    reset_log(a, b);
    let at = a.rebase(sym);
    // what earlier originals left (the oracle keeps the x87 stack), gone
    fpu_empty(a);
    fpu_empty(b);
    let ret = a
        .call(at, conv, args)
        .unwrap_or_else(|e| panic!("{what} {args:x?}: {e}"));
    // a float the original leaves in ST0 is a result its callers use: the
    // bridges here all answer in eax (or nothing), so there must be none
    assert!(
        ret.st0.is_none(),
        "{what} {args:x?}: the original returns {:?} in ST0, the port no float",
        ret.st0
    );
    let want = ret.eax;
    let got = bridge(&mut Run(b));
    if let Some(got) = got {
        assert_eq!(got & mask, want & mask, "{what} {args:x?}: eax");
    }
    let d = a.diff(b).unwrap();
    assert!(
        d.is_empty(),
        "{what} {args:x?}: memory {:x?}",
        &d[..d.len().min(16)]
    );
}

fn cdecl(
    what: &str,
    a: &mut Oracle,
    b: &mut Oracle,
    sym: u32,
    args: &[u32],
    bridge: impl FnOnce(&mut Run) -> Option<u32>,
) {
    check(what, a, b, sym, Conv::Cdecl, args, bridge);
}

/// A function's address by its `symbols/ffx.sym` name.
fn sym(name: &str) -> u32 {
    static SYMS: std::sync::OnceLock<std::collections::HashMap<String, u32>> =
        std::sync::OnceLock::new();
    let syms = SYMS.get_or_init(|| {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../symbols/ffx.sym");
        let text = std::fs::read_to_string(path).unwrap();
        let mut out = std::collections::HashMap::new();
        for line in text.lines() {
            let p: Vec<&str> = line
                .split('#')
                .next()
                .unwrap_or("")
                .split_whitespace()
                .collect();
            if p.len() == 3 && p[1] == "function" {
                out.insert(
                    p[2].to_string(),
                    u32::from_str_radix(p[0].trim_start_matches("0x"), 16).unwrap(),
                );
            }
        }
        out
    });
    *syms
        .get(name)
        .unwrap_or_else(|| panic!("{name} not in ffx.sym"))
}

/// One cdecl case per `"Name" => bridge` with the same arguments.
macro_rules! each {
    ($a:ident, $b:ident, $args:expr, $( $name:literal => $f:expr ),* $(,)?) => {
        $( cdecl($name, $a, $b, sym($name), $args, $f); )*
    };
}

// ---- state

/// Random Snd state: the masks, the status words, the volume table, the
/// footstep tables (sound ids), a clear queue; pointers stay valid.
fn scramble(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    for addr in (0x0132_f690..0x0132_ff10).step_by(4) {
        let v = if rng.below(3) == 0 { 0 } else { rng.word() };
        let at = a.rebase(addr);
        put(a, b, at, v);
    }
    for g in [
        s::VOICE_READ,
        s::SHOUT_READ,
        s::VAG_READ,
        s::STANDBY_VOICE,
        s::SENDING_DATA,
        s::STATUS,
        s::VOICE_SECTORS,
        s::FOOT3D_MASK,
        s::FOOT_MASK,
        s::KINU_MASK,
        s::EVENT_MASK,
        s::FOOT_KICK_STAY,
        s::IOP_ERROR,
        s::NOW_PLAY,
        s::STATUS_PREV,
        s::STATUS_FRAME,
        s::SHOUT_PREV,
        s::SHOUT_FRAME,
        s::VAG_PREV,
        s::VAG_FRAME,
        s::BATTLE_MUSIC_SIZE,
        s::LAST_FADE_OUT,
    ] {
        let v = rng.word();
        set(a, b, g, v);
    }
    for g in [0x0132_ff30u32, 0x0132_ff38, 0x0132_ff40, 0x0132_ff48] {
        let v = rng.word();
        let at = a.rebase(g);
        put(a, b, at, v);
    }
    for i in 0..16 {
        let v = rng.word();
        let at = a.rebase(s::PARTY_WORKERS) + 4 * i;
        put(a, b, at, v);
        let at = a.rebase(s::IOP_STATUS) + 4 * i;
        put(a, b, at, v ^ 0x55);
    }
    for i in 0..256 {
        #[allow(clippy::cast_possible_truncation)] // a byte
        let v = rng.next() as u8;
        let at = a.rebase(s::MUSIC_VOLUME) + i;
        put(a, b, at, v);
    }
    for g in [s::MUSIC_MUTE, s::SEP_LOCK, s::KICK_FLAG, s::HDD_ERROR] {
        let v = u8::from(rng.below(3) == 0);
        set(a, b, g, v);
    }
    // the 3D control and the event table, or none
    let ctrl = if rng.below(2) == 0 {
        0
    } else {
        alloc(a, b, 0x40)
    };
    set(a, b, s::FOOT3D_CTRL, ctrl);
    let events = if rng.below(2) == 0 {
        0
    } else {
        let n = rng.below(6);
        let t = alloc(a, b, 4 + 4 * n as usize);
        put(a, b, t, n);
        for i in 0..n {
            let v = rng.word();
            put(a, b, t + 4 + 4 * i, v);
        }
        t
    };
    set(a, b, s::SOUND_EVENT, events);
    clear_queue(a, b, rng);
    stub_rets(a, b, rng);
}

/// A random music clear queue: musics 0..8 and SEs (negative), small
/// frame counts, seps 0..8.
fn clear_queue(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    let n = rng.below(18);
    set(a, b, s::CLEAR_COUNT, n.cast_signed());
    let base = a.rebase(s::CLEAR_QUE);
    for i in 0..17 {
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)] // small values
        let music = if rng.below(4) == 0 {
            -(rng.below(3) as i16) - 1
        } else {
            rng.below(8) as i16
        };
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)] // small values
        let frames = rng.below(6) as i16 - 2;
        let sep = rng.below(8);
        put(a, b, base + 8 * i, music);
        put(a, b, base + 8 * i + 2, frames);
        put(a, b, base + 8 * i + 4, sep);
    }
}

// ---- tests

#[test]
fn ids_and_getters() {
    let mut rng = Xorshift(0x5eed_0001);
    for (world, mut a, mut b) in worlds() {
        let (a, b) = (&mut a, &mut b);
        for _ in 0..40 {
            let id = rng.word();
            cdecl(
                "SndSep_IsValidHandle",
                a,
                b,
                sym("SndSep_IsValidHandle"),
                &[id],
                |_| Some(s::snd_sep_kind(id)),
            );
            cdecl(
                "SndSep_HandleIndex",
                a,
                b,
                sym("SndSep_HandleIndex"),
                &[id],
                |_| Some(s::snd_sep_id(id)),
            );
            let k = rng.below(20).wrapping_sub(2);
            cdecl(
                "SndGetFootMaxVariation",
                a,
                b,
                sym("SndGetFootMaxVariation"),
                &[k],
                |_| Some(s::snd_get_foot_max_variation(k)),
            );
            cdecl(
                "SndGetFootLastVariation",
                a,
                b,
                sym("SndGetFootLastVariation"),
                &[k],
                |_| Some(s::snd_get_foot_last_variation(k)),
            );
        }
        for _ in 0..30 {
            scramble(a, b, &mut rng);
            each!(a, b, &[],
                "SndGetEventMask" => |h| Some(s::snd_get_event_mask(h)),
                "SndGetFoot3dMask" => |h| Some(s::snd_get_foot3d_mask(h)),
                "SndGetFootMask" => |h| Some(s::snd_get_foot_mask(h)),
                "SndGetKinuMask" => |h| Some(s::snd_get_kinu_mask(h)),
                "SndGetNowPlayMusic" => |h| Some(s::snd_get_now_play_music(h)),
                "SndGetSoundType" => |h| Some(s::snd_get_sound_type(h)),
                "SndGetSpuMemorySortSwitch" => |h| Some(s::snd_get_spu_memory_sort_switch(h)),
                "SndIsReadShoutVoiceStatus" => |h| Some(s::snd_is_read_shout_voice_status(h)),
                "SndIsReadVagVoiceStatus" => |h| Some(s::snd_is_read_vag_voice_status(h)),
            );
            let v = rng.below(4) as u8;
            set(a, b, s::SOUND_TYPE, v);
            set(a, b, s::SPU_SORT, v ^ 1);
            cdecl("SndGetSoundType", a, b, sym("SndGetSoundType"), &[], |h| {
                Some(s::snd_get_sound_type(h))
            });
            cdecl(
                "SndGetSpuMemorySortSwitch",
                a,
                b,
                sym("SndGetSpuMemorySortSwitch"),
                &[],
                |h| Some(s::snd_get_spu_memory_sort_switch(h)),
            );
            cdecl(
                "SndGetStreamState",
                a,
                b,
                sym("SndGetStreamState"),
                &[],
                |h| Some(s::snd_get_stream_state(h)),
            );
            let i = rng.below(16);
            cdecl(
                "SndSqsReportIopSoundStatus",
                a,
                b,
                sym("SndSqsReportIopSoundStatus"),
                &[i],
                |h| Some(s::snd_sqs_report_iop_sound_status(h, i)),
            );
            for id in [
                u32::MAX,
                rng.below(256),
                rng.below(256) | 0x100,
                rng.below(256) | 0xab00,
            ] {
                cdecl(
                    "SndGetMusicVolume",
                    a,
                    b,
                    sym("SndGetMusicVolume"),
                    &[id],
                    |h| Some(s::snd_get_music_volume(h, id)),
                );
            }
            let i = rng.below(8);
            cdecl(
                "SndKickFootEvent",
                a,
                b,
                sym("SndKickFootEvent"),
                &[i],
                |h| Some(s::snd_kick_foot_event(h, i)),
            );
            let wave = alloc(a, b, 0x10);
            for k in 0..4 {
                let v = rng.word();
                put(a, b, wave + 4 * k, v);
            }
            cdecl(
                "SndGetSepNumber",
                a,
                b,
                sym("SndGetSepNumber"),
                &[wave],
                |h| Some(s::snd_get_sep_number(h, wave)),
            );
            cdecl("SndGetSepSize", a, b, sym("SndGetSepSize"), &[wave], |h| {
                Some(s::snd_get_sep_size(h, wave))
            });
            cdecl(
                "SndGetWaveSize",
                a,
                b,
                sym("SndGetWaveSize"),
                &[wave],
                |h| Some(s::snd_get_wave_size(h, wave)),
            );
            a.reset_heap();
            b.reset_heap();
        }
        eprintln!("{world}: ids and getters agree");
    }
}

#[test]
fn foot_sep_work() {
    let mut rng = Xorshift(0x5eed_0002);
    for (world, mut a, mut b) in worlds() {
        let (a, b) = (&mut a, &mut b);
        for kind in 0..20u32 {
            for party in [-3i32, -1, 0, 1, 7, 15, 16, 40] {
                for var in [-2i32, -1, 0, 1, 2, 3, 4, 9] {
                    let args = [
                        kind.wrapping_sub(2),
                        party.cast_unsigned(),
                        var.cast_unsigned(),
                    ];
                    cdecl(
                        "SndGetFootSepWork",
                        a,
                        b,
                        sym("SndGetFootSepWork"),
                        &args,
                        |h| Some(s::snd_get_foot_sep_work(h, args[0], party, var)),
                    );
                }
            }
        }
        for _ in 0..60 {
            scramble(a, b, &mut rng);
            let kind = rng.below(18);
            let party = rng.below(20).cast_signed() - 2;
            let var = rng.below(7).cast_signed() - 1;
            let sep = rng.word();
            let args = [kind, party.cast_unsigned(), var.cast_unsigned(), sep];
            cdecl("SndSetFootSep", a, b, sym("SndSetFootSep"), &args, |h| {
                Some(s::snd_set_foot_sep(h, kind, party, var, sep))
            });
            let (p, on) = (rng.below(40), rng.below(3));
            each!(a, b, &[p, on],
                "SndSetEventStatus" => |h| Some(s::snd_set_event_status(h, p, on)),
                "SndSetFoot3dStatus" => |h| Some(s::snd_set_foot3d_status(h, p, on)),
                "SndSetFootStatus" => |h| Some(s::snd_set_foot_status(h, p, on)),
                "SndSetKinuStatus" => |h| Some(s::snd_set_kinu_status(h, p, on)),
            );
            let (p, w) = (rng.below(20).cast_signed() - 2, rng.word());
            cdecl(
                "SndEventSetPcToActorFoot",
                a,
                b,
                sym("SndEventSetPcToActorFoot"),
                &[p.cast_unsigned(), w],
                |h| {
                    s::snd_event_set_pc_to_actor_foot(h, p, w);
                    None
                },
            );
            let v = rng.word();
            cdecl(
                "SndEventSetFoot3dCtrlp",
                a,
                b,
                sym("SndEventSetFoot3dCtrlp"),
                &[v],
                |h| Some(s::snd_event_set_foot3d_ctrlp(h, v)),
            );
            cdecl(
                "SndEventSetSoundEvent",
                a,
                b,
                sym("SndEventSetSoundEvent"),
                &[v],
                |h| Some(s::snd_event_set_sound_event(h, v)),
            );
            each!(a, b, &[],
                "SndPushFootStatus" => |h| Some(s::snd_push_foot_status(h)),
                "SndPopFootStatus" => |h| Some(s::snd_pop_foot_status(h)),
            );
            scramble(a, b, &mut rng);
            each!(a, b, &[],
                "SndPlayFootStatus" => |h| { s::snd_play_foot_status(h); None },
                "SndPushFootStatus" => |h| Some(s::snd_push_foot_status(h)),
                "SndClearFootStatus" => |h| { s::snd_clear_foot_status(h); None },
                "SndPopFootStatus" => |h| Some(s::snd_pop_foot_status(h)),
                "SndEventClearPcToActorFoot" => |h| { s::snd_event_clear_pc_to_actor_foot(h); None },
                "SndInitNowPlayMusic" => |h| { s::snd_init_now_play_music(h); None },
                "SndClearMusicClearQue" => |h| { s::snd_clear_music_clear_que(h); None },
                "SndSetMusicMute" => |h| { s::snd_set_music_mute(h); None },
            );
            scramble(a, b, &mut rng);
            cdecl(
                "SndClearFootSepWork",
                a,
                b,
                sym("SndClearFootSepWork"),
                &[rng.below(2)],
                |h| {
                    s::snd_clear_foot_sep_work(h);
                    None
                },
            );
            scramble(a, b, &mut rng);
            cdecl(
                "SndInitFootSepWork",
                a,
                b,
                sym("SndInitFootSepWork"),
                &[],
                |h| {
                    s::snd_init_foot_sep_work(h);
                    None
                },
            );
            scramble(a, b, &mut rng);
            cdecl(
                "SndRemoveFootSepWork",
                a,
                b,
                sym("SndRemoveFootSepWork"),
                &[],
                |h| {
                    s::snd_remove_foot_sep_work(h);
                    None
                },
            );
            a.reset_heap();
            b.reset_heap();
        }
        eprintln!("{world}: footstep work agrees");
    }
}

#[test]
fn kick_foot() {
    let mut rng = Xorshift(0x5eed_0003);
    for (world, mut a, mut b) in worlds() {
        let (a, b) = (&mut a, &mut b);
        let mut kicked = 0;
        for _ in 0..600 {
            scramble(a, b, &mut rng);
            set(a, b, s::FOOT_KICK_STAY, rng.below(2));
            for g in [s::FOOT3D_MASK, s::FOOT_MASK, s::KINU_MASK, s::EVENT_MASK] {
                let v = if rng.below(3) == 0 {
                    u32::MAX
                } else {
                    rng.next()
                };
                set(a, b, g, v);
            }
            let kind = if rng.below(5) == 0 {
                0xff
            } else {
                rng.below(17)
            };
            let party = rng.below(19);
            let kick = rng.below(7).wrapping_sub(1);
            let var = rng.below(8).cast_signed() - 2;
            let args = [kind, party, kick, var.cast_unsigned()];
            cdecl("SndKickFoot", a, b, sym("SndKickFoot"), &args, |h| {
                s::snd_kick_foot(h, kind, party, kick, var);
                None
            });
            kicked += logged(a);
            a.reset_heap();
            b.reset_heap();
        }
        assert!(kicked > 50, "{world}: few footsteps played ({kicked})");
        eprintln!("{world}: SndKickFoot agrees ({kicked} calls logged)");
    }
}

#[test]
fn music() {
    let mut rng = Xorshift(0x5eed_0004);
    for (world, mut a, mut b) in worlds() {
        let (a, b) = (&mut a, &mut b);
        for _ in 0..150 {
            scramble(a, b, &mut rng);
            let id = match rng.below(5) {
                0 => rng.below(0x120),
                1 => s::NOW_PLAY.get(a),
                2 => s::LAST_FADE_OUT.get(a),
                3 => rng.below(8),
                _ => rng.word(),
            };
            if rng.below(3) == 0 {
                set(a, b, s::NOW_PLAY, u32::MAX);
            }
            if rng.below(3) == 0 {
                set(a, b, s::LAST_FADE_OUT, id);
            }
            let (x, y) = (rng.word(), rng.word());
            let g = sym("SndMusicCrossStart");
            cdecl("SndMusicCrossStart", a, b, g, &[id, x], |h| {
                s::snd_music_cross_start(h, id, x);
                None
            });
            // SndMusicRead waits on the stream words (TkSndUpdateMain clears
            // them; one of the two set exercises the loop)
            set(a, b, s::SHOUT_READ, rng.below(2));
            set(a, b, s::VAG_READ, 0);
            stub_ret(a, b, fmod::REGIST_COMMAND_SYNC, 0);
            stub_ret(a, b, s::SQS_SHOUT_STREAMING_PLAY_STATE, 0);
            each!(a, b, &[id],
                "SndMusicClear" => |h| { s::snd_music_clear(h, id); None },
                "SndMusicMultiStart" => |h| { s::snd_music_multi_start(h, id); None },
                "SndMusicStart" => |h| { s::snd_music_start(h, id); None },
                "SndMusicStop" => |h| { s::snd_music_stop(h, id); None },
                "SndMusicRead" => |h| { s::snd_music_read(h, id); None },
                "SndClearNowPlayMusic" => |h| { s::snd_clear_now_play_music(h, id); None },
                "SndRegistNowPlayMusic" => |h| { s::snd_regist_now_play_music(h, id); None },
                "SndCancelMusicClear" => |h| { s::snd_cancel_music_clear(h, id); None },
            );
            scramble(a, b, &mut rng);
            each!(a, b, &[id, x],
                "SndMusicPhraseStart" => |h| { s::snd_music_phrase_start(h, id, x); None },
                "SndMusicVolumeDirect" => |h| { s::snd_music_volume_direct(h, id, x); None },
                "SndMusicVolumeFadeOut" => |h| { s::snd_music_volume_fade_out(h, id, x); None },
            );
            scramble(a, b, &mut rng);
            cdecl(
                "SndMusicVolumeTranslate",
                a,
                b,
                sym("SndMusicVolumeTranslate"),
                &[id, x, y],
                |h| {
                    s::snd_music_volume_translate(h, id, x, y);
                    None
                },
            );
            cdecl(
                "SndMusicPitchDirect",
                a,
                b,
                sym("SndMusicPitchDirect"),
                &[x, y],
                |h| Some(s::snd_music_pitch_direct(h, x, y)),
            );
            cdecl(
                "SndMusicPitchTranslate",
                a,
                b,
                sym("SndMusicPitchTranslate"),
                &[id, x, y],
                |h| Some(s::snd_music_pitch_translate(h, id, x, y)),
            );
            cdecl(
                "SndMusicTempoDirect",
                a,
                b,
                sym("SndMusicTempoDirect"),
                &[x],
                |h| Some(s::snd_music_tempo_direct(h, x)),
            );
            cdecl(
                "SndMusicTempoTranslate",
                a,
                b,
                sym("SndMusicTempoTranslate"),
                &[id, x],
                |h| Some(s::snd_music_tempo_translate(h, id, x)),
            );
            cdecl(
                "SndMusicStandby",
                a,
                b,
                sym("SndMusicStandby"),
                &[id],
                |h| Some(s::snd_music_standby(h, id)),
            );
            cdecl(
                "SndGetMusicPlayState",
                a,
                b,
                sym("SndGetMusicPlayState"),
                &[id],
                |h| Some(s::snd_get_music_play_state(h, id)),
            );
            a.reset_heap();
            b.reset_heap();
        }
        eprintln!("{world}: music agrees");
    }
}

#[test]
fn music_clear_queue() {
    let mut rng = Xorshift(0x5eed_0005);
    for (world, mut a, mut b) in worlds() {
        let (a, b) = (&mut a, &mut b);
        for _ in 0..400 {
            scramble(a, b, &mut rng);
            let mode = match rng.below(4) {
                0 => 0,
                1 => u32::MAX,
                2 => 1,
                _ => rng.word(),
            };
            let id = if rng.below(5) == 0 {
                rng.word()
            } else {
                rng.below(8)
            };
            cdecl(
                "SndExecMusicClearQue",
                a,
                b,
                sym("SndExecMusicClearQue"),
                &[mode, id],
                |h| {
                    s::snd_exec_music_clear_que(h, mode, id);
                    None
                },
            );
            a.reset_heap();
            b.reset_heap();
        }
        eprintln!("{world}: SndExecMusicClearQue agrees");
    }
}

#[test]
fn sound_effects() {
    let mut rng = Xorshift(0x5eed_0006);
    for (world, mut a, mut b) in worlds() {
        let (a, b) = (&mut a, &mut b);
        for _ in 0..80 {
            scramble(a, b, &mut rng);
            let id = rng.word();
            let (x, y, z) = (rng.word(), rng.word(), rng.word());
            let locked = s::SEP_LOCK.get(a) != 0;
            cdecl(
                "SndSepPlaySimple",
                a,
                b,
                sym("SndSepPlaySimple"),
                &[id],
                |h| {
                    let r = s::snd_sep_play_simple(h, id);
                    // locked, the original leaves its caller's eax
                    (!locked).then_some(r)
                },
            );
            each!(a, b, &[id],
                "SndSepStop" => |h| Some(s::snd_sep_stop(h, id)),
                "SndIsSepPlay" => |h| Some(s::snd_is_sep_play(h, id)),
                "SndForcePlayClick" => |h| Some(s::snd_force_play_click(h, id)),
                "SndPlayClick" => |h| Some(s::snd_play_click(h, id)),
                "SndSepClear" => |h| Some(s::snd_sep_clear(h, id)),
                "SndDataSendSync" => |h| Some(s::snd_data_send_sync(h, id)),
                "SndReportStandbyStatusDirect" => |h| Some(s::snd_report_standby_status_direct(h, id)),
                "SndSortCommonWave" => |h| Some(s::snd_sort_common_wave(h, id)),
                "SndSpuMemorySortSwitch" => |h| Some(s::snd_spu_memory_sort_switch(h, id)),
                "SndSetStraightStreamingDigitalMode" => |h| Some(s::snd_set_straight_streaming_digital_mode(h, id)),
                "SndSepLock" => |h| Some(s::snd_sep_lock(h, id)),
            );
            cdecl(
                "SndSortCommonWave",
                a,
                b,
                sym("SndSortCommonWave"),
                &[0],
                |h| Some(s::snd_sort_common_wave(h, 0)),
            );
            for kind in [s::CLICK_GATE.get(a), 1] {
                set(a, b, s::CLICK_GATE, kind);
                stub_ret(a, b, s::MS_BATTLE_CHECK, rng.below(2));
                cdecl("SndPlayClick", a, b, sym("SndPlayClick"), &[id], |h| {
                    Some(s::snd_play_click(h, id))
                });
            }
            cdecl("SndSepClear", a, b, sym("SndSepClear"), &[u32::MAX], |h| {
                Some(s::snd_sep_clear(h, u32::MAX))
            });
            cdecl("SndSepClearAll", a, b, sym("SndSepClearAll"), &[], |h| {
                Some(s::snd_sep_clear_all(h))
            });
            cdecl(
                "SndGetSePlayState",
                a,
                b,
                sym("SndGetSePlayState"),
                &[],
                |h| Some(s::snd_get_se_play_state(h)),
            );
            let locked = s::SEP_LOCK.get(a) != 0;
            cdecl("SndSepPlay", a, b, sym("SndSepPlay"), &[id, x, y], |h| {
                let r = s::snd_sep_play(h, id, x, y);
                (!locked).then_some(r)
            });
            set(a, b, s::SEP_LOCK, u8::from(rng.below(2) == 0));
            let lock = s::SEP_LOCK.get(a) != 0;
            cdecl("SndPlay", a, b, sym("SndPlay"), &[x, id, y, z], |h| {
                let r = s::snd_play(h, x, id, y, z);
                (!lock).then_some(r)
            });
            each!(a, b, &[id, x],
                "SndSepPanDirect" => |h| Some(s::snd_sep_pan_direct(h, id, x)),
                "SndSepVolumeDirect" => |h| Some(s::snd_sep_volume_direct(h, id, x)),
                "SndSepVolumeFadeOut" => |h| Some(s::snd_sep_volume_fade_out(h, id, x)),
                "SndSepVolumeFadeOut2" => |h| Some(s::snd_sep_volume_fade_out2(h, id, x)),
                "SndSepTrans" => |h| Some(s::snd_sep_trans(h, id, x)),
                "SndSepTransNonblock" => |h| Some(s::snd_sep_trans_nonblock(h, id, x)),
                "SndSepTransSepOnly" => |h| Some(s::snd_sep_trans_sep_only(h, id, x)),
            );
            each!(a, b, &[id, x, y],
                "SndSepPanTranslate" => |h| Some(s::snd_sep_pan_translate(h, id, x, y)),
                "SndSepVolumeTranslate" => |h| Some(s::snd_sep_volume_translate(h, id, x, y)),
                "SndSePitchDirect" => |h| Some(s::snd_se_pitch_direct(h, id, x, y)),
            );
            cdecl(
                "SndSePitchTranslate",
                a,
                b,
                sym("SndSePitchTranslate"),
                &[id, x, y, z],
                |h| Some(s::snd_se_pitch_translate(h, id, x, y, z)),
            );
            a.reset_heap();
            b.reset_heap();
        }
        eprintln!("{world}: sound effects agree");
    }
}

#[test]
fn streams_and_status() {
    let mut rng = Xorshift(0x5eed_0007);
    for (world, mut a, mut b) in worlds() {
        let (a, b) = (&mut a, &mut b);
        for _ in 0..80 {
            scramble(a, b, &mut rng);
            let (x, y, z) = (rng.word(), rng.word(), rng.word());
            stub_ret(a, b, s::TK_CHECK_PLAY_VOICE, rng.below(2));
            each!(a, b, &[],
                "SndStartStream" => |h| Some(s::snd_start_stream(h)),
                "SndStopStream" => |h| Some(s::snd_stop_stream(h)),
                "SndStopBattleStream" => |h| Some(s::snd_stop_battle_stream(h)),
                "SndSqsIopSoundErrorStatus" => |h| Some(s::snd_sqs_iop_sound_error_status(h)),
                "SndSqsShoutStreamingPlayState" => |h| Some(s::snd_sqs_shout_streaming_play_state(h)),
                "SndSqsVagStreamingPlayState" => |h| Some(s::snd_sqs_vag_streaming_play_state(h)),
                "SndSyncMusicStandby" => |_| Some(s::snd_sync_music_standby()),
            );
            for _ in 0..4 {
                scramble(a, b, &mut rng);
                set(a, b, s::CD_WAIT, u8::from(rng.below(2) == 0));
                stub_ret(a, b, s::SQS_EE_SOUND_ERROR_STATUS, rng.below(2) << 31);
                stub_ret(a, b, s::TK_CD_READ_SYNC, rng.below(2));
                each!(a, b, &[],
                    "SndIopEioErrorStatus" => |h| Some(s::snd_iop_eio_error_status(h)),
                    "SndSeqIopCommandSendMain" => |h| { s::snd_seq_iop_command_send_main(h); None },
                    "SndSqsIopFrame" => |h| Some(s::snd_sqs_iop_frame(h)),
                    "SndStatusUpdate" => |h| { s::snd_status_update(h); None },
                    "SndStandbySepWave" => |h| { s::snd_standby_sep_wave(h); None },
                    "SndInitWork" => |h| { s::snd_init_work(h); None },
                );
            }
            cdecl(
                "SndStandbyStream",
                a,
                b,
                sym("SndStandbyStream"),
                &[x],
                |h| {
                    s::snd_standby_stream(h, x);
                    None
                },
            );
            cdecl(
                "SndStartStream2",
                a,
                b,
                sym("SndStartStream2"),
                &[x, y],
                |h| Some(s::snd_start_stream2(h, x, y)),
            );
            cdecl(
                "SndStartBattleStream",
                a,
                b,
                sym("SndStartBattleStream"),
                &[x, y, z],
                |h| Some(s::snd_start_battle_stream(h, x, y, z)),
            );
            cdecl(
                "SndStartShoutStream",
                a,
                b,
                sym("SndStartShoutStream"),
                &[x, y, z],
                |h| Some(s::snd_start_shout_stream(h, x, y, z)),
            );
            cdecl(
                "SndVagStreamVolumeDirect",
                a,
                b,
                sym("SndVagStreamVolumeDirect"),
                &[x],
                |h| Some(s::snd_vag_stream_volume_direct(h, x)),
            );
            cdecl(
                "SndVagStreamVolumeTranslate",
                a,
                b,
                sym("SndVagStreamVolumeTranslate"),
                &[x, y],
                |h| Some(s::snd_vag_stream_volume_translate(h, x, y)),
            );
            for block in [0, 1, x] {
                cdecl("SndSyncTrans", a, b, sym("SndSyncTrans"), &[block], |_| {
                    Some(s::snd_sync_trans(block))
                });
                cdecl(
                    "SndSyncWaveTrans",
                    a,
                    b,
                    sym("SndSyncWaveTrans"),
                    &[block],
                    |_| Some(s::snd_sync_wave_trans(block)),
                );
            }
            for size in [0x2_ffff, 0x3_0000, rng.below(0x40000), y] {
                // the wait loop calls TkSndUpdateMain, which clears the words
                set(a, b, s::SHOUT_READ, rng.below(2));
                set(a, b, s::VAG_READ, 0);
                stub_rets(a, b, &mut rng);
                stub_ret(a, b, fmod::REGIST_COMMAND_SYNC, 0);
                stub_ret(a, b, s::SQS_SHOUT_STREAMING_PLAY_STATE, 0);
                stub_ret(a, b, s::SQS_DATA_SENDING_COMPLETED, 0);
                cdecl(
                    "SndStandbyBattleWave",
                    a,
                    b,
                    sym("SndStandbyBattleWave"),
                    &[x, size],
                    |h| {
                        s::snd_standby_battle_wave(h, x, size.cast_signed());
                        None
                    },
                );
                set(a, b, s::SHOUT_READ, rng.below(2));
                cdecl(
                    "SndStandbyBattleWaveBlock",
                    a,
                    b,
                    sym("SndStandbyBattleWaveBlock"),
                    &[x, size],
                    |h| {
                        s::snd_standby_battle_wave_block(h, x, size.cast_signed());
                        None
                    },
                );
            }
            let mode = rng.below(8).cast_signed() - 3;
            cdecl(
                "SndSetSoundMode",
                a,
                b,
                sym("SndSetSoundMode"),
                &[mode.cast_unsigned()],
                |h| Some(s::snd_set_sound_mode(h, mode)),
            );
            cdecl(
                "SndSetSoundType",
                a,
                b,
                sym("SndSetSoundType"),
                &[mode.cast_unsigned()],
                |h| Some(s::snd_set_sound_type(h, mode)),
            );
            a.reset_heap();
            b.reset_heap();
        }
        eprintln!("{world}: streams and status agree");
    }
}

#[test]
fn waves() {
    let mut rng = Xorshift(0x5eed_0008);
    for (world, mut a, mut b) in worlds() {
        let (a, b) = (&mut a, &mut b);
        for _ in 0..60 {
            scramble(a, b, &mut rng);
            let list = if rng.below(3) == 0 {
                0
            } else {
                let l = alloc(a, b, 32);
                for k in 0..16 {
                    #[allow(clippy::cast_possible_truncation)] // a wave id
                    let w = if rng.below(3) == 0 {
                        -1
                    } else {
                        rng.below(0x900) as i16
                    };
                    put(a, b, l + 2 * k, w);
                }
                l
            };
            cdecl(
                "SndSetWaveData",
                a,
                b,
                sym("SndSetWaveData"),
                &[list],
                |h| {
                    s::snd_set_wave_data(h, list);
                    None
                },
            );
            cdecl(
                "SndSortCommonWaveWithSet",
                a,
                b,
                sym("SndSortCommonWaveWithSet"),
                &[list],
                |h| {
                    s::snd_sort_common_wave_with_set(h, list);
                    None
                },
            );
            a.reset_heap();
            b.reset_heap();
        }
        eprintln!("{world}: waves agree");
    }
}

#[test]
fn voice_wait() {
    let mut rng = Xorshift(0x5eed_0009);
    for (world, mut a, mut b) in worlds() {
        let (a, b) = (&mut a, &mut b);
        for i in 0..400 {
            let v = match i % 4 {
                0 => rng.below(0x2000),
                1 => rng.next(),
                2 => 0x8000_0000 | rng.next(),
                _ => rng.below(64),
            };
            set(a, b, s::VOICE_SECTORS, v);
            #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)] // a mode byte
            let mode = (rng.below(3) as i8) - 1;
            set(a, b, s::VIDEO_MODE, mode);
            cdecl(
                "SndCalcVoiceWait",
                a,
                b,
                sym("SndCalcVoiceWait"),
                &[],
                |h| Some(s::snd_calc_voice_wait(h)),
            );
        }
        eprintln!("{world}: SndCalcVoiceWait agrees");
    }
}

#[test]
fn bgm_type_save() {
    let mut rng = Xorshift(0x5eed_0010);
    for (world, mut a, mut b) in worlds() {
        let (a, b) = (&mut a, &mut b);
        let at = a.rebase(0x0112_ca90 + s::SAVE_BGM_TYPE);
        for i in 0..64u32 {
            #[allow(clippy::cast_possible_truncation)] // a byte
            let old = rng.next() as u8;
            put(a, b, at, old);
            stub_rets(a, b, &mut rng);
            let on = match i % 4 {
                0 => 0,
                1 => 1,
                2 => 0x100,
                _ => rng.word(),
            };
            cdecl(
                "SndSetBgmTypeSave",
                a,
                b,
                sym("SndSetBgmTypeSave"),
                &[on],
                |h| Some(s::snd_set_bgm_type_save(h, on)),
            );
            // iMusicRefreshBgmType queued the type
            assert_eq!(logged(a), 1, "SndSetBgmTypeSave({on:#x})");
            assert_eq!(Mem::read::<u8>(a, at), u8::from(on != 0));
        }
        eprintln!("{world}: SndSetBgmTypeSave agrees");
    }
}

#[test]
fn sound_update() {
    let mut rng = Xorshift(0x5eed_0011);
    for (world, mut a, mut b) in worlds() {
        let (a, b) = (&mut a, &mut b);
        let real = f::SETTINGS.get(a);
        let fake = alloc(a, b, 0x40);
        for i in 0..200u32 {
            stub_rets(a, b, &mut rng);
            if i == 0 && real != 0 {
                // the snapshot's own settings
                f::SETTINGS.set(a, real);
                f::SETTINGS.set(b, real);
            } else {
                set(a, b, f::SETTINGS, fake);
                for k in 0..16 {
                    let v = match rng.below(4) {
                        0 => rng.below(101),
                        1 => 0,
                        _ => rng.word(),
                    };
                    put(a, b, fake + 4 * k, v);
                }
                let mute = if rng.below(2) == 0 { 0 } else { rng.word() };
                put(a, b, fake + f::SETTINGS_MUTE, mute);
            }
            cdecl("iSoundUpdate", a, b, sym("iSoundUpdate"), &[], |h| {
                Some(f::i_sound_update(h))
            });
            // the movie volume and four commands
            assert_eq!(logged(a), 5, "iSoundUpdate");
        }
        eprintln!("{world}: iSoundUpdate agrees");
    }
}

/// Writes a stream position table: groups of entry lists.
fn stream_table(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) -> u32 {
    let t = alloc(a, b, 0x1000);
    let groups = 1 + rng.below(4);
    put(a, b, t, u16::try_from(groups).unwrap());
    let mut at = 2 + 2 * groups;
    for g in 0..groups {
        put(a, b, t + 2 + 2 * g, u16::try_from(at).unwrap());
        let lists = 1 + rng.below(4);
        let group = at;
        put(a, b, t + group, u16::try_from(lists).unwrap());
        at += 2 + 2 * lists;
        for l in 0..lists {
            put(a, b, t + group + 2 + 2 * l, u16::try_from(at).unwrap());
            let n = 1 + rng.below(6);
            put(a, b, t + at, u16::try_from(n).unwrap());
            let mut sector = rng.below(0x1000);
            for k in 0..=n {
                let v = if rng.below(4) == 0 {
                    0xffff
                } else {
                    sector += rng.below(3);
                    sector
                };
                put(a, b, t + at + 2 + 2 * k, u16::try_from(v & 0xffff).unwrap());
            }
            put(a, b, t + at + 4 + 2 * n, 0x10u16);
            at += 6 + 2 * n;
        }
    }
    t
}

#[test]
fn stream_names_and_positions() {
    let mut rng = Xorshift(0x5eed_000a);
    for (world, mut a, mut b) in worlds() {
        let (a, b) = (&mut a, &mut b);
        let (mut found, mut lengths) = (0, 0);
        for _ in 0..200 {
            let t = if rng.below(6) == 0 {
                0
            } else {
                stream_table(a, b, &mut rng)
            };
            set(a, b, s::STREAM_TABLE, t);
            for _ in 0..10 {
                let (d2, d1, d0) = (rng.below(4), rng.below(4), rng.below(6));
                let id = (d2 * 10000 + d1 * 100 + d0) << 12 | rng.below(0x1000);
                let id = if rng.below(8) == 0 { rng.next() } else { id };
                cdecl(
                    "SndStreamDVDPos",
                    a,
                    b,
                    sym("SndStreamDVDPos"),
                    &[id],
                    |h| {
                        let r = s::snd_stream_dvd_pos(h, id);
                        found += u32::from(r != u32::MAX);
                        lengths += u32::from(s::VOICE_SECTORS.get(h) != 0);
                        Some(r)
                    },
                );
                let buf = alloc(a, b, 32);
                #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
                // a language byte
                let lang = rng.below(12) as i8 - 2;
                set(a, b, s::LANG, lang);
                let id = if rng.below(2) == 0 {
                    rng.next()
                } else {
                    rng.below(0x0100_0000)
                };
                cdecl(
                    "SndStreamPCName",
                    a,
                    b,
                    sym("SndStreamPCName"),
                    &[id, buf],
                    |h| {
                        s::snd_stream_pc_name(h, id, buf);
                        None
                    },
                );
                let text: Vec<u8> = (0..rng.below(24)).map(|_| (rng.next() | 1) as u8).collect();
                let p = a.alloc_bytes(&[text.as_slice(), &[0]].concat()).unwrap();
                assert_eq!(b.alloc_bytes(&[text.as_slice(), &[0]].concat()).unwrap(), p);
                cdecl("SndToUpper", a, b, sym("SndToUpper"), &[p], |h| {
                    s::snd_to_upper(h, p);
                    None
                });
            }
            a.reset_heap();
            b.reset_heap();
        }
        assert!(
            found > 100 && lengths > 50,
            "{world}: {found} positions, {lengths} lengths"
        );
        eprintln!(
            "{world}: stream names and positions agree ({found} positions, {lengths} lengths)"
        );
    }
}

/// The `FmodManager` command helpers by argument count: async, then sync.
const HELPERS: [[&str; 4]; 2] = [
    [
        "FmodManager_registCommandAsync1",
        "FmodManager_registCommandAsync",
        "FmodManager_registCommandAsync3",
        "FmodManager_registCommandAsync4",
    ],
    [
        "FmodManager_registCommandSync1",
        "FmodManager_registCommandSync2",
        "FmodManager_registCommandSync3",
        "FmodManager_registCommandSync4",
    ],
];

#[test]
fn fmod_queue() {
    let mut rng = Xorshift(0x5eed_000b);
    for (world, mut a, mut b) in worlds() {
        let (a, b) = (&mut a, &mut b);
        for _ in 0..40 {
            stub_rets(a, b, &mut rng);
            let (x, y, z, w) = (rng.word(), rng.word(), rng.word(), rng.word());
            each!(a, b, &[],
                "iFmodFinalize" => |h| Some(f::i_fmod_finalize(h)),
                "iFmodInitialize" => |h| Some(f::i_fmod_initialize(h)),
                "iMusicClearAll" => |h| Some(f::i_music_clear_all(h)),
                "iMusicReplayPointSet" => |h| Some(f::i_music_replay_point_set(h)),
                "iMusicStopAll" => |h| Some(f::i_music_stop_all(h)),
                "iMusicSyncStandby" => |h| Some(f::i_music_sync_standby(h)),
                "iSfxIsPaused" => |h| Some(f::i_sfx_is_paused(h)),
            );
            each!(a, b, &[x],
                "iMusicClear" => |h| Some(f::i_music_clear(h, x)),
                "iMusicDataSend" => |h| Some(f::i_music_data_send(h, x)),
                "iMusicMultiPlay" => |h| Some(f::i_music_multi_play(h, x)),
                "iMusicPlayFast" => |h| Some(f::i_music_play_fast(h, x)),
                "iMusicPlayState" => |h| Some(f::i_music_play_state(h, x)),
                "iMusicRead" => |h| Some(f::i_music_read(h, x)),
                "iMusicReplayPointClear" => |h| Some(f::i_music_replay_point_clear(h, x)),
                "iMusicSetPaused" => |h| Some(f::i_music_set_paused(h, x)),
                "iMusicStop" => |h| Some(f::i_music_stop(h, x)),
                "iMusicTempoDirect" => |h| Some(f::i_music_tempo_direct(h, x)),
                "iSfxIsPlaying" => |h| Some(f::i_sfx_is_playing(h, x)),
                "iSfxPause" => |h| Some(f::i_sfx_pause(h, x)),
            );
            let t = [rng.below(3), 0xb5, 0xb6, x][rng.below(4) as usize];
            each!(a, b, &[t],
                "iMuiscSetBgmType" => |h| Some(f::i_music_set_bgm_type(h, t)),
                "iMusicStart" => |h| Some(f::i_music_start(h, t)),
            );
            each!(a, b, &[x, y],
                "iMusicCrossPlay" => |h| Some(f::i_music_cross_play(h, x, y)),
                "iMusicMuteSet" => |h| Some(f::i_music_mute_set(h, x, y)),
                "iMusicPhrasePlay" => |h| Some(f::i_music_phrase_play(h, x, y)),
                "iMusicPitchDirect" => |h| Some(f::i_music_pitch_direct(h, x, y)),
                "iMusicTempoTranslate" => |h| Some(f::i_music_tempo_translate(h, x, y)),
                "iMusicVolumeDirect" => |h| Some(f::i_music_volume_direct(h, x, y)),
                "iMusicVolumeFadeOut" => |h| Some(f::i_music_volume_fade_out(h, x, y)),
                "iSfxLoadData" => |h| Some(f::i_sfx_load_data(h, x, y)),
                "iSfxSetPan" => |h| Some(f::i_sfx_set_pan(h, x, y)),
            );
            each!(a, b, &[x, y, z],
                "iMusicPitchTranslate" => |h| Some(f::i_music_pitch_translate(h, x, y, z)),
                "iMusicVolumeTranslate" => |h| Some(f::i_music_volume_translate(h, x, y, z)),
                "iSfxForceStart" => |h| Some(f::i_sfx_force_start(h, x, y, z)),
                "iSfxPlayBattleStreaming" => |h| Some(f::i_sfx_play_battle_streaming(h, x, y, z)),
                "iSfxSetPanTranslate" => |h| Some(f::i_sfx_set_pan_translate(h, x, y, z)),
            );
            cdecl(
                "iSfxDelayStart",
                a,
                b,
                sym("iSfxDelayStart"),
                &[x, y, z, w],
                |h| Some(f::i_sfx_delay_start(h, x, y, z, w)),
            );
            // the helpers, on a manager of the case's choosing
            let mgr = f::MANAGER.get(a) ^ (rng.below(2) * 0x10);
            for (k, names) in HELPERS.iter().enumerate() {
                for (n, name) in names.iter().enumerate() {
                    let args = [x, y, z, w];
                    let g = sym(name);
                    let shim_args: Vec<u32> = args[..=n].to_vec();
                    check(name, a, b, g, Conv::Thiscall(mgr), &shim_args, |h| {
                        let [c, p, q, r] = [args[0], args[1], args[2], args[3]];
                        Some(match (k, n) {
                            (0, 0) => f::push0(h, mgr, c),
                            (0, 1) => f::push1(h, mgr, c, p),
                            (0, 2) => f::push2(h, mgr, c, p, q),
                            (0, 3) => f::push3(h, mgr, c, p, q, r),
                            (_, 0) => f::sync0(h, mgr, c),
                            (_, 1) => f::sync1(h, mgr, c, p),
                            (_, 2) => f::sync2(h, mgr, c, p, q),
                            _ => f::sync3(h, mgr, c, p, q, r),
                        })
                    });
                }
            }
            let mgr = f::MANAGER.get(a);
            let ffx = Mem::read::<u32>(a, mgr + f::MANAGER_FMOD);
            let id = [x, 0xb5, 0xb6, rng.below(0xb6)][rng.below(4) as usize];
            check(
                "FmodManager_signalMusicStart",
                a,
                b,
                sym("FmodManager_signalMusicStart"),
                Conv::Thiscall(mgr),
                &[id],
                |h| Some(f::fmod_manager_signal_music_start(h, mgr, id)),
            );
            check(
                "FfxFmod_musicSeqCheck",
                a,
                b,
                sym("FfxFmod_musicSeqCheck"),
                Conv::Thiscall(ffx),
                &[id],
                |_| Some(f::ffx_fmod_music_seq_check(id)),
            );
            check(
                "FfxFmod_setMusicStartId",
                a,
                b,
                sym("FfxFmod_setMusicStartId"),
                Conv::Thiscall(ffx),
                &[y],
                |h| Some(f::ffx_fmod_set_music_start_id(h, ffx, y)),
            );
            check(
                "FfxFmod_setMusicStartSignal",
                a,
                b,
                sym("FfxFmod_setMusicStartSignal"),
                Conv::Thiscall(ffx),
                &[z],
                |h| Some(f::ffx_fmod_set_music_start_signal(h, ffx, z)),
            );
        }
        eprintln!("{world}: FMOD queue entry points agree");
    }
}

#[test]
fn stream_pc_name_core() {
    let name = |id: u32, english: bool| {
        let mut buf = [0u8; 16];
        for (at, b) in ffx_sound::snd::stream_pc_name(id, english) {
            buf[at] = b;
        }
        let end = buf.iter().position(|&b| b == 0).unwrap();
        String::from_utf8(buf[..end].to_vec()).unwrap()
    };
    // voice 123, letters 'a' + 2 and 'a' + 4
    assert_eq!(name(123 << 12 | 3 << 6 | 5, true), "ve000123ce.vs");
    assert_eq!(name(5 << 12, false), "vj000005``.vs");
}

// ---- the FMOD thread: handlers, FfxFmod, the objects

/// A random volume (f32): in and out of 0..1, signed zeros, a quiet NaN.
fn volume(rng: &mut Xorshift) -> f32 {
    #[allow(clippy::cast_precision_loss)] // small values
    match rng.below(8) {
        0 => 0.0,
        1 => -0.0,
        2 => 1.0,
        3 => 0.5,
        4 => -(rng.below(100) as f32) / 50.0,
        5 => 1.0 + rng.below(100) as f32 / 50.0,
        6 => f32::NAN,
        _ => rng.below(1000) as f32 / 1000.0,
    }
}

/// An argument word: a music id, a small signed number (percentages,
/// frames), or anything.
fn arg_word(rng: &mut Xorshift) -> u32 {
    match rng.below(4) {
        0 => rng.below(0xb8),
        1 => rng.below(300).wrapping_sub(50),
        _ => rng.word(),
    }
}

/// Random words in both oracles.
fn fill(a: &mut Oracle, b: &mut Oracle, at: u32, len: u32, rng: &mut Xorshift) {
    for i in (0..len).step_by(4) {
        let v = rng.word();
        put(a, b, at + i, v);
    }
}

/// An FMOD event pointer or none.
fn event(rng: &mut Xorshift) -> u32 {
    if rng.below(3) == 0 {
        0
    } else {
        0x0400_0000 | rng.below(0x10_0000) << 4
    }
}

/// A random `FmodSfx` event tree at `head` (`FmodSfx +0x64`): sound ids
/// near the stop limit and one, each with a list of events (some none);
/// returns its ids.
fn sfx_tree(a: &mut Oracle, b: &mut Oracle, head: u32, rng: &mut Xorshift) -> Vec<u32> {
    put(a, b, head + 0xc, head);
    let mut ids = Vec::new();
    for _ in 0..rng.below(7) {
        let id = [1, 2, 0x7f, 0x80, 0x81, 0x200, rng.word()][rng.below(7) as usize];
        if ids.contains(&id) {
            continue;
        }
        ids.push(id);
        let node = alloc(a, b, 0x20);
        fill(a, b, node, 0x20, rng);
        put(a, b, node, head);
        put(a, b, node + 4, head);
        put(a, b, node + 0xc, id);
        let n = rng.below(4);
        let events = alloc(a, b, 4 * n as usize + 4);
        for i in 0..n {
            let e = event(rng);
            put(a, b, events + 4 * i, e);
        }
        put(a, b, node + 0x10, n);
        put(a, b, node + 0x18, events);
        // insert: smaller ids to the left
        let mut at = head + 0xc;
        loop {
            let cur = Mem::read::<u32>(a, at);
            if cur == head {
                put(a, b, at, node);
                break;
            }
            let key = Mem::read::<u32>(a, cur + 0xc).cast_signed();
            at = if key > id.cast_signed() { cur } else { cur + 4 };
        }
    }
    ids
}

/// An `FfxFmod` for a case: in a snapshot, now and then the game's own;
/// else a random one with its three objects (the sfx's ids returned too).
fn fake_fmod(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) -> (u32, Vec<u32>) {
    let mgr = f::MANAGER.get(a);
    if mgr != a.rebase(FAKE_FMOD) && mgr != 0 && rng.below(3) == 0 {
        let ffx = Mem::read::<u32>(a, mgr + f::MANAGER_FMOD);
        let objects = [x::MUSIC, x::VOICE, x::SFX].map(|o| Mem::read::<u32>(a, ffx + o));
        if ffx != 0 && objects.iter().all(|&o| o != 0) {
            return (ffx, Vec::new());
        }
    }
    let ffx = alloc(a, b, 0x40);
    fill(a, b, ffx, 0x40, rng);
    let es = event(rng);
    put(a, b, ffx + x::EVENT_SYSTEM, es);
    for v in [
        x::MASTER_VOLUME,
        x::MUSIC_VOLUME,
        x::VOICE_VOLUME,
        x::SFX_VOLUME,
    ] {
        let f = volume(rng);
        put(a, b, ffx + v, f);
    }
    let music = alloc(a, b, 0x40);
    fill(a, b, music, 0x40, rng);
    let records = alloc(a, b, (0xb6 * x::MUSIC_RECORD) as usize);
    fill(a, b, records, 0xb6 * x::MUSIC_RECORD, rng);
    for id in 0..0xb6 {
        let e = event(rng);
        put(a, b, records + id * x::MUSIC_RECORD, e);
    }
    put(a, b, music + x::MUSIC_RECORDS, records);
    let id = rng.below(0xb6);
    put(a, b, music + x::MUSIC_START_ID, id);
    let voice = alloc(a, b, 0x100);
    fill(a, b, voice, 0x100, rng);
    let e = event(rng);
    put(a, b, voice + x::VOICE_EVENT, e);
    let paused = [0, 1, rng.word()][rng.below(3) as usize];
    put(a, b, voice + x::VOICE_PAUSED, paused);
    for v in [x::VOICE_VOLUME_LEVEL, x::VOICE_ENTIRE_VOLUME] {
        let f = volume(rng);
        put(a, b, voice + v, f);
    }
    let sfx = alloc(a, b, 0x80);
    fill(a, b, sfx, 0x80, rng);
    let p = u8::from(rng.below(2) == 0);
    put(a, b, sfx + x::SFX_PAUSED, p);
    let ids = sfx_tree(a, b, sfx + x::SFX_EVENTS, rng);
    put(a, b, ffx + x::MUSIC, music);
    put(a, b, ffx + x::VOICE, voice);
    put(a, b, ffx + x::SFX, sfx);
    (ffx, ids)
}

/// Return values that steer the FMOD paths: `getState`'s result and
/// state, the sfx answers.
fn fmod_rets(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    stub_rets(a, b, rng);
    let r = [0, 0, rng.word()][rng.below(3) as usize];
    stub_ret(a, b, x::EVENT_GET_STATE, r);
    let s = [2, 8, 0, 9, rng.word()][rng.below(5) as usize];
    stub_out(a, b, x::EVENT_GET_STATE, s);
    for sym in [
        xc::SFX_CAN_START_NEW_EVENT,
        xc::EVENT_IS_PLAYING,
        xc::SFX_CREATE_EVENT,
        xc::VOICE_DATA_CHANGE,
    ] {
        let v = [0, 1, 0x100, rng.word()][rng.below(4) as usize];
        stub_ret(a, b, sym, v);
    }
    let lang = rng.below(7);
    let at = a.rebase(LANG);
    put(a, b, at, lang);
}

#[test]
fn fmod_handlers() {
    let mut rng = Xorshift(0x5eed_0021);
    for (world, mut a, mut b) in worlds() {
        let (a, b) = (&mut a, &mut b);
        for _ in 0..10 {
            for (cmd, &at) in x::HANDLERS.iter().enumerate() {
                let cmd = u32::try_from(cmd).unwrap();
                a.reset_heap();
                b.reset_heap();
                let (fmod, _) = fake_fmod(a, b, &mut rng);
                fmod_rets(a, b, &mut rng);
                let args = alloc(a, b, 16);
                for i in 0..4 {
                    let v = arg_word(&mut rng);
                    put(a, b, args + 4 * i, v);
                }
                cdecl(&format!("handler {cmd:#x}"), a, b, at, &[fmod, args], |h| {
                    x::handle(h, cmd, fmod, args)
                });
            }
        }
        eprintln!("{world}: the 70 FMOD command handlers agree");
    }
}

/// One thiscall case per `address name (args) => bridge`.
macro_rules! method {
    ($a:ident, $b:ident, $this:expr, $( $at:literal $name:literal ($($w:expr),*) => $f:expr ),* $(,)?) => {
        $( check($name, $a, $b, $at, Conv::Thiscall($this), &[$($w),*], $f); )*
    };
}

#[test]
fn ffx_fmod_methods() {
    let mut rng = Xorshift(0x5eed_0022);
    for (world, mut a, mut b) in worlds() {
        let (a, b) = (&mut a, &mut b);
        for _ in 0..30 {
            a.reset_heap();
            b.reset_heap();
            let (m, _) = fake_fmod(a, b, &mut rng);
            fmod_rets(a, b, &mut rng);
            let w: [u32; 4] = std::array::from_fn(|_| arg_word(&mut rng));
            let (p, q, r, s4) = (w[0], w[1], w[2], w[3]);
            let id = [rng.below(0xb6), 0xb5, 0xb6, p][rng.below(4) as usize];
            let t = [0, 1, 2, p][rng.below(4) as usize];
            let kind = [0, 1, 2, q][rng.below(4) as usize];
            let v = volume(&mut rng).to_bits();
            method!(a, b, m,
                0x0070_67d0 "FfxFmod::getNowPlayMusic" () => |h| Some(x::get_now_play_music(h, m)),
                0x0070_6920 "FfxFmod::initLoadMasterBank" () => |h| {
                    x::init_load_master_bank(h, m);
                    None
                },
                0x0070_69e0 "FfxFmod::musicClear" (p) => |h| Some(x::music_clear(h, m, p)),
                0x0070_69f0 "FfxFmod::musicClearAll" () => |h| Some(x::music_clear_all(h, m)),
                0x0070_6a00 "FfxFmod::musicCrossPlay" (p, q) => |h| Some(x::music_cross_play(h, m, p, q)),
                0x0070_6a20 "FfxFmod::musicDataSend" (p) => |h| Some(x::music_data_send(h, m, p)),
                0x0070_6a40 "FfxFmod::musicMultiPlay" (id) => |h| Some(x::music_multi_play(h, m, id)),
                0x0070_6a70 "FfxFmod::musicPauseAll" (p) => |h| Some(x::music_pause_all(h, m, p)),
                0x0070_6a80 "FfxFmod::musicPhrasePlay" (id, q) => |h| Some(x::music_multi_play(h, m, id)),
                0x0070_6ad0 "FfxFmod::musicPlayState" (p) => |h| Some(x::music_play_state(h, m, p)),
                0x0070_6ae0 "FfxFmod::musicRead" (p) => |h| Some(x::music_read(h, m, p)),
                0x0070_6b00 "FfxFmod::musicReplayPointClear" (id) => |h| Some(x::music_replay_point_clear(h, m, id)),
                0x0070_6b30 "FfxFmod::musicReplayPointSet" () => |h| Some(x::music_replay_point_set(h, m)),
                0x0070_6b90 "FfxFmod::musicStart" (p) => |h| Some(x::music_start(h, m, p)),
                0x0070_6ba0 "FfxFmod::musicStop" (p) => |h| Some(x::music_stop(h, m, p)),
                0x0070_6bb0 "FfxFmod::musicStopAll" () => |h| Some(x::music_stop_all(h, m)),
                0x0070_6bc0 "FfxFmod::musicSwitchBGM" (t) => |h| Some(x::music_switch_bgm(h, m, t)),
                0x0070_6bf0 "FfxFmod::musicSyncStandby" () => |h| Some(x::music_sync_standby(h, m)),
                0x0070_6c20 "FfxFmod::musicVolumeDirect" (p, v, r) => |h| Some(x::music_volume_direct(h, m, p, v, r)),
                0x0070_6c40 "FfxFmod::musicVolumeFadeOut" (p, q) => |h| Some(x::music_volume_fade_out(h, m, p, q)),
                0x0070_6c50 "FfxFmod::musicVolumeTranslate" (p, q, r) => |h| Some(x::music_volume_translate(h, m, p, q, r)),
                0x0070_6db0 "FfxFmod::sfxDelayStart" (p, q, r, s4) => |h| Some(x::sfx_call(h, m, xc::SFX_DELAY_START, &[p, q, r, s4])),
                0x0070_6dd0 "FfxFmod::sfxForceStart" (p, q, r) => |h| Some(x::sfx_call(h, m, xc::SFX_FORCE_START, &[p, q, r])),
                0x0070_6df0 "FfxFmod::sfxIsPlaying" (p) => |h| Some(x::sfx_is_playing(h, m, p)),
                0x0070_6e10 "FfxFmod::sfxIspaused" () => |h| Some(x::sfx_is_paused(h, m)),
                0x0070_6e20 "FfxFmod::sfxLoad" (p, t) => |h| Some(x::sfx_load(h, m, p, t)),
                0x0070_6e50 "FfxFmod::sfxPause" (t) => |h| Some(x::sfx_call(h, m, xc::SFX_PAUSE, &[t])),
                0x0070_6e70 "FfxFmod::sfxPlayBattleStreaming" (p, q, r) => |h| Some(x::sfx_play_battle_streaming(h, m, p, q, r)),
                0x0070_6e90 "FfxFmod::sfxSetPan" (p, q) => |h| Some(x::sfx_call(h, m, xc::SFX_SET_PAN, &[p, q])),
                0x0070_6eb0 "FfxFmod::sfxSetPan/3" (p, q, r) => |h| Some(x::sfx_call(h, m, xc::SFX_SET_PAN_TRANSLATE, &[p, q, r])),
                0x0070_6ed0 "FfxFmod::sfxSetVolume" (p, q) => |h| Some(x::sfx_call(h, m, xc::SFX_SET_VOLUME, &[p, q])),
                0x0070_6ef0 "FfxFmod::sfxSetVolume/3" (p, q, r) => |h| Some(x::sfx_call(h, m, xc::SFX_SET_VOLUME_TRANSLATE, &[p, q, r])),
                0x0070_6f10 "FfxFmod::sfxStart" (p, q, r) => |h| Some(x::sfx_call(h, m, xc::SFX_START, &[p, q, r])),
                0x0070_6f30 "FfxFmod::sfxStop" (p) => |h| Some(x::sfx_call(h, m, xc::SFX_STOP, &[p])),
                0x0070_6f50 "FfxFmod::sfxStopAllNonBlockingSound" (p) => |h| Some(x::sfx_call(h, m, xc::SFX_STOP_ALL_NON_BLOCKING, &[p, 0])),
                0x0070_6f70 "FfxFmod::sfxStopAllNonloopSound" () => |h| Some(x::sfx_call(h, m, xc::SFX_STOP_ALL_NONLOOP, &[])),
                0x0070_6f80 "FfxFmod::sfxStopNonBlockingSound" (p) => |h| Some(x::sfx_call(h, m, xc::SFX_STOP_NON_BLOCKING, &[p])),
                0x0070_6fa0 "FfxFmod::sfxUnloadAllData" () => |h| Some(x::sfx_call(h, m, xc::SFX_UNLOAD_PROJECTS, &[0, 0])),
                0x0070_6fb0 "FfxFmod::sfxUnloadBySeq" (p) => |h| Some(x::sfx_call(h, m, xc::SFX_UNLOAD_BY_SEQ, &[p])),
                0x0070_6fd0 "FfxFmod::shoutStart" (p, q, r) => |h| Some(x::shout_start(h, m, p, q, r)),
                0x0070_7080 "FfxFmod::soundInit" () => |h| Some(x::sound_init(h, m)),
                0x0070_71c0 "FfxFmod::soundPlayStatus" () => |h| Some(x::sound_play_status(h, m)),
                0x0070_71e0 "FfxFmod::soundSetMasterVolume" (v) => |h| Some(x::sound_set_volume(h, m, x::MASTER_VOLUME, v)),
                0x0070_7280 "FfxFmod::soundSetMusicVolume" (v) => |h| Some(x::sound_set_volume(h, m, x::MUSIC_VOLUME, v)),
                0x0070_72f0 "FfxFmod::soundSetMute" (p, kind) => |h| Some(x::sound_set_mute(h, m, p, kind)),
                0x0070_7330 "FfxFmod::soundSetSfxVolume" (v) => |h| Some(x::sound_set_volume(h, m, x::SFX_VOLUME, v)),
                0x0070_73a0 "FfxFmod::soundSetVoiceVolume" (v) => |h| Some(x::sound_set_volume(h, m, x::VOICE_VOLUME, v)),
                0x0070_7410 "FfxFmod::soundStageFlush" (p) => |h| Some(x::sound_stage_flush(h, m, p)),
                0x0070_7420 "FfxFmod::soundUpdate" () => |h| Some(x::sound_update(h, m)),
                0x0070_7470 "FfxFmod::voiceClear" () => |h| Some(x::voice_clear(h, m)),
                0x0070_7480 "FfxFmod::voiceDataChange" (p) => |h| Some(x::voice_data_change(h, m, p)),
                0x0070_74b0 "FfxFmod::voiceIsPaused" () => |h| Some(x::voice_is_paused(h, m)),
                0x0070_74e0 "FfxFmod::voicePlayStatus" () => |h| Some(x::sound_play_status(h, m)),
                0x0070_74f0 "FfxFmod::voiceRead" (p) => |h| Some(x::voice_read(h, m, p)),
                0x0070_7510 "FfxFmod::voiceReleaseData" () => |h| Some(x::voice_release_data(h, m)),
                0x0070_7520 "FfxFmod::voiceSetPaused" (p) => |h| Some(x::voice_set_paused(h, m, p)),
                0x0070_7530 "FfxFmod::voiceSetStartSignal" (p) => |h| Some(x::voice_set_start_signal(h, m, p)),
                0x0070_7540 "FfxFmod::voiceStart" (p, q) => |h| Some(x::voice_start(h, m, p, q)),
                0x0070_7550 "FfxFmod::voiceStop" () => |h| Some(x::voice_stop(h, m)),
                0x0070_7560 "FfxFmod::voiceVolumeDirect" (v, q) => |h| Some(x::voice_volume_direct(h, m, v, q)),
                0x0070_7580 "FfxFmod::voiceVolumeTranslate" (p, q) => |h| Some(x::voice_volume_translate(h, m, p, q)),
            );
        }
        eprintln!("{world}: FfxFmod methods agree");
    }
}

/// The object methods ported (left unrecorded in `fmod_objects`).
const OBJECTS: &[u32] = &[
    xc::MUSIC_PLAY_STATE,
    xc::MUSIC_SYNC_STANDBY,
    xc::MUSIC_VOLUME_FADE_OUT,
    xc::VOICE_SET_ENTIRE_VOLUME,
    xc::VOICE_SET_MUTE,
    xc::VOICE_SET_PAUSED,
    xc::VOICE_SET_LANG,
    xc::VOICE_SHOUT_START,
    xc::VOICE_SOUND_STAGE_FLUSH,
    xc::SFX_CAN_START_NEW_EVENT,
    xc::SFX_DELAY_START,
    xc::SFX_FORCE_START,
    xc::SFX_START,
    xc::SFX_SET_PAN,
    xc::SFX_SET_PAN_TRANSLATE,
    xc::SFX_SET_VOLUME,
    xc::SFX_SET_VOLUME_TRANSLATE,
    xc::SFX_STOP,
    xc::SFX_STOP_NON_BLOCKING,
];

#[test]
fn fmod_objects() {
    let mut rng = Xorshift(0x5eed_0023);
    for (world, mut a, mut b) in worlds_except(OBJECTS) {
        let (a, b) = (&mut a, &mut b);
        for _ in 0..60 {
            a.reset_heap();
            b.reset_heap();
            let (ffx, ids) = fake_fmod(a, b, &mut rng);
            fmod_rets(a, b, &mut rng);
            let [music, voice, sfx] =
                [x::MUSIC, x::VOICE, x::SFX].map(|o| Mem::read::<u32>(a, ffx + o));
            let w: [u32; 4] = std::array::from_fn(|_| arg_word(&mut rng));
            let (p, q, r, s4) = (w[0], w[1], w[2], w[3]);
            let id = [rng.below(0xb6), 0xb5, 0xb6, p][rng.below(4) as usize];
            let v = volume(&mut rng).to_bits();
            let lang = [0, 1, 2, 3, 4, 5, p][rng.below(7) as usize];
            let flush = [0x189, 0x18e, 0x188, 0x18f, p][rng.below(5) as usize];
            let paused = [0, 1, 2, p][rng.below(4) as usize];
            for g in x::STAGE_FLUSH {
                let v = rng.word();
                let at = a.rebase(g);
                put(a, b, at, v);
            }
            method!(a, b, music,
                0x0070_9060 "FmodMusic::playState" (p) => |h| Some(x::music_object_play_state(h, music)),
                0x0070_9d80 "FmodMusic::syncStandby" () => |h| Some(x::music_object_sync_standby(h, music)),
                0x0070_9eb0 "FmodMusic::volumeFadeOut" (id, q) => |h| Some(x::music_object_volume_fade_out(h, music, id, q)),
            );
            method!(a, b, voice,
                0x0070_b2b0 "FmodVoice::setEntireVolume" (v) => |h| x::voice_set_entire_volume(h, voice, v),
                0x0070_b330 "FmodVoice::setMute" (paused) => |h| Some(x::voice_set_mute(h, voice, paused)),
                0x0070_b370 "FmodVoice::setPaused" (paused) => |h| Some(x::voice_object_set_paused(h, voice, paused)),
                0x0070_b3f0 "FfxFmod_soundInit_setLang" (lang) => |h| Some(x::voice_set_lang(h, voice, lang)),
                0x0070_b430 "FmodVoice::shoutStart" (p, q, r, s4) => |h| Some(x::voice_shout_start(h, voice, [p, q, r, s4])),
                0x0070_b570 "FmodVoice::soundStageFlush" (flush) => |h| Some(x::voice_sound_stage_flush(h, voice, flush)),
            );
            // the sfx: ids of the tree, one, the stop limit, any
            let sid = if !ids.is_empty() && rng.below(3) != 0 {
                ids[rng.below(u32::try_from(ids.len()).unwrap()) as usize]
            } else {
                [1, 0x7f, 0x80, p][rng.below(4) as usize]
            };
            let key = alloc(a, b, 4);
            put(a, b, key, sid);
            let tree = sfx + x::SFX_EVENTS;
            check(
                "FmodSfx tree find",
                a,
                b,
                0x0070_ba70,
                Conv::Thiscall(tree),
                &[key],
                |h| Some(x::sfx_find(h, tree, sid)),
            );
            check_masked(
                "FmodSfx::canStartNewEvent",
                a,
                b,
                0x0070_c7d0,
                Conv::Thiscall(sfx),
                &[sid],
                0xff,
                |h| Some(x::sfx_can_start_new_event(h, sfx, sid)),
            );
            method!(a, b, sfx,
                0x0070_cc30 "FmodSfx_delayStart" (sid, q, r, s4) => |h| x::sfx_delay_start(h, sfx, [sid, q, r, s4]),
                0x0070_ccd0 "FmodSfx::forceStart" (sid, q, r) => |h| x::sfx_force_start(h, sfx, sid, q, r),
                0x0070_d890 "FmodSfx_Start" (sid, q, r) => |h| x::sfx_start(h, sfx, sid, q, r),
                0x0070_d640 "FmodSfx::setPan" (sid, q) => |h| {
                    x::sfx_set_pan(h, sfx, sid, q);
                    None
                },
                0x0070_d690 "FmodSfx::setPan/3" (sid, q, r) => |h| {
                    x::sfx_set_pan_translate(h, sfx, sid, q, r);
                    None
                },
                0x0070_d6e0 "FmodSfx::setVolume" (sid, q) => |h| {
                    x::sfx_set_volume(h, sfx, sid, q);
                    None
                },
                0x0070_d730 "FmodSfx::setVolume/3" (sid, q, r) => |h| {
                    x::sfx_set_volume_translate(h, sfx, sid, q, r);
                    None
                },
                0x0070_d8f0 "FmodSfx::stop" (sid) => |h| {
                    x::sfx_stop(h, sfx, sid);
                    None
                },
                0x0070_db90 "FmodSfx::stopNonBlockingSound" (sid) => |h| x::sfx_stop_non_blocking(h, sfx, sid),
            );
        }
        eprintln!("{world}: FMOD object methods agree");
    }
}

// ---- pc round 4: Snd3dFootRequest (0x00876950) -------------------------

/// [`Run`] with the float calls (`Host::call_st0`) the 3D request makes.
struct RunF<'a>(&'a mut Oracle);

impl Mem for RunF<'_> {
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

impl Host for RunF<'_> {
    fn call(&mut self, addr: u32, conv: host::Conv, args: &[u32]) -> u32 {
        Run(self.0).call(addr, conv, args)
    }
    fn call_st0(&mut self, addr: u32, conv: host::Conv, args: &[u32]) -> f32 {
        assert_eq!(conv, host::Conv::Cdecl);
        let r = self
            .0
            .call(addr, Conv::Cdecl, args)
            .unwrap_or_else(|e| panic!("callee {addr:#x}: {e}"));
        r.st0_f32().expect("a float in ST0")
    }
    fn scratch(&mut self) -> u32 {
        Run(self.0).scratch()
    }
}

/// The stand-ins' control block: their answers, then a log.
mod foot_ctl {
    pub const POS: u32 = 0;
    pub const KIND: u32 = 0xc;
    pub const ROT: u32 = 0x10;
    pub const ANGLE: u32 = 0x1c;
    pub const HASH: u32 = 0x20;
    pub const SCENARIO: u32 = 0x24;
    pub const SCENARIO_CALLS: u32 = 0x2c;
    pub const LOG_COUNT: u32 = 0x30;
    pub const LOG: u32 = 0x34;
    pub const CELLS: u32 = 0x800;
    pub const LEN: usize = 0x900;
}

fn foot_log(s: &mut ffx_oracle::StandIn<'_, '_>, ctl: u32, tag: u32, n: u32) {
    let k: u32 = Mem::read(s, ctl + foot_ctl::LOG_COUNT);
    let at = ctl + foot_ctl::LOG + 20 * (k % 64);
    Mem::write(s, at, tag);
    for i in 0..n {
        let v = s.arg(i);
        Mem::write(s, at + 4 + 4 * i, v);
    }
    Mem::write(s, ctl + foot_ctl::LOG_COUNT, k + 1);
}

/// Rust stand-ins for the request's script, renderer and scenario
/// callees, alike in both oracles: answers from `ctl`, calls logged there.
fn foot_stand_ins(o: &mut Oracle, ctl: u32) {
    use ffx_oracle::Returns;
    use ffx_sound::foot3d::bridge as f3;
    let cell = |k: u32| ctl + foot_ctl::CELLS + 8 * k;
    for (k, (sym, out)) in (0..).zip([
        (f3::ACTOR_POS, foot_ctl::POS),
        (f3::ACTOR_ROT, foot_ctl::ROT),
    ]) {
        let at = o.rebase(sym);
        o.stand_in(at, Returns::Eax, cell(k), move |s| {
            // the id only: the pointers are each side's frame
            foot_log(s, ctl, sym, 1);
            for i in 0..3 {
                let v: u32 = Mem::read(s, ctl + out + 4 * i);
                let p = s.arg(1 + i);
                Mem::write(s, p, v);
            }
            0x1234
        })
        .unwrap();
    }
    let at = o.rebase(f3::ACTOR_TYPE);
    o.stand_in(at, Returns::Eax, cell(2), move |s| {
        foot_log(s, ctl, f3::ACTOR_TYPE, 1);
        u64::from(Mem::read::<u32>(s, ctl + foot_ctl::KIND))
    })
    .unwrap();
    let at = o.rebase(f3::ATAN2_XZ);
    o.stand_in(at, Returns::St0F32, cell(3), move |s| {
        foot_log(s, ctl, f3::ATAN2_XZ, 4);
        u64::from(Mem::read::<u32>(s, ctl + foot_ctl::ANGLE))
    })
    .unwrap();
    let at = o.rebase(f3::LEVEL_NAME_HASH);
    o.stand_in(at, Returns::Eax, cell(4), move |s| {
        foot_log(s, ctl, f3::LEVEL_NAME_HASH, 0);
        u64::from(Mem::read::<u32>(s, ctl + foot_ctl::HASH))
    })
    .unwrap();
    let at = o.rebase(f3::SCENARIO_FLAG);
    o.stand_in(at, Returns::Eax, cell(5), move |s| {
        foot_log(s, ctl, f3::SCENARIO_FLAG, 0);
        let k: u32 = Mem::read(s, ctl + foot_ctl::SCENARIO_CALLS);
        Mem::write(s, ctl + foot_ctl::SCENARIO_CALLS, k + 1);
        u64::from(Mem::read::<u32>(s, ctl + foot_ctl::SCENARIO + 4 * (k & 1)))
    })
    .unwrap();
}

/// A float for the request's geometry: small, edge or any.
fn foot_float(rng: &mut Xorshift) -> f32 {
    match rng.below(10) {
        0 => 0.0,
        1 => f32::from_bits(rng.next()),
        2 => -1.0,
        3 => 1.0,
        _ => {
            #[allow(clippy::cast_precision_loss)] // small integers
            let v = (rng.next() % 4001) as f32 / 100.0 - 20.0;
            v
        }
    }
}

fn quiet(f: f32) -> f32 {
    if f.is_nan() {
        f32::from_bits(f.to_bits() | 0x0040_0000)
    } else {
        f
    }
}

/// A request [`pc_r4_snd3d_foot_request`] crafts (float bits): where the
/// original's roundings to `f32` show.
struct FootCase {
    x: u32,
    angle: u32,
    pan_scale: u32,
    pan: (u32, u32),
    volume: (u32, u32),
    far_volume: u8,
    near_volume: u8,
    ratio: u32,
}

const F1: u32 = 0x3f80_0000;
const F30: u32 = 0x41f0_0000;
const INF: u32 = 0x7f80_0000;

const FOOT_CRAFTED: [FootCase; 6] = [
    // pi/2 just below f32 pi/2 times just above 2 is f32 pi: pan 127,
    // 126 without that rounding or with a double pi
    FootCase {
        x: F1,
        angle: 0x3fc9_0fb5,
        pan_scale: 0x4000_0018,
        pan: (0, 0),
        volume: (0, 0),
        far_volume: 1,
        near_volume: 1,
        ratio: F1,
    },
    FootCase {
        x: F1,
        angle: 0x3fc9_0fdb,
        pan_scale: 0x4000_0000,
        pan: (0, 0),
        volume: (0, 0),
        far_volume: 1,
        near_volume: 1,
        ratio: F1,
    },
    // the volume's ratio rounded to f32 before the mix
    FootCase {
        x: 0x417c_a1ad,
        angle: 0,
        pan_scale: F1,
        pan: (0, 0),
        volume: (0, F30),
        far_volume: 94,
        near_volume: 37,
        ratio: F1,
    },
    // the mix rounded to f32 before the total ratio
    FootCase {
        x: 0x3ea0_ebd5,
        angle: 0,
        pan_scale: F1,
        pan: (0, 0),
        volume: (0, F1),
        far_volume: 190,
        near_volume: 42,
        ratio: 0x3f94_f1c0,
    },
    // exactly at the far volume distance: the mix, clamped
    FootCase {
        x: 0x40a0_0000,
        angle: 0,
        pan_scale: F1,
        pan: (0, 0),
        volume: (0, 0x40a0_0000),
        far_volume: 200,
        near_volume: 0,
        ratio: F1,
    },
    // an infinite distance at an infinite pan distance: ratio 1
    FootCase {
        x: 0x7f61_b1e6,
        angle: F1,
        pan_scale: F1,
        pan: (0, INF),
        volume: (0, F1),
        far_volume: 9,
        near_volume: 9,
        ratio: F1,
    },
];

#[test]
fn pc_r4_snd3d_foot_request() {
    use ffx_sound::foot3d::bridge as f3;
    const IDENTITY: [f32; 16] = [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ];
    let mut rng = Xorshift(0x5d3f_0401);
    for (world, mut a, mut b) in worlds_except(&[s::FOOT3D_REQUEST]) {
        let (a, b) = (&mut a, &mut b);
        let ctl = a.alloc(foot_ctl::LEN).unwrap();
        assert_eq!(b.alloc(foot_ctl::LEN).unwrap(), ctl);
        foot_stand_ins(a, ctl);
        foot_stand_ins(b, ctl);
        let req = a.alloc(0x40).unwrap();
        let p = a.alloc(0x40).unwrap();
        let work = a.alloc(0x40).unwrap();
        assert_eq!(
            (b.alloc(0x40).unwrap(), b.alloc(0x40).unwrap()),
            (p - 0x40, p)
        );
        assert_eq!(b.alloc(0x40).unwrap(), work);
        // the original's frame from ebp - 0x24 (three arguments, the
        // return address and the saved ebp under the call's top)
        let frame = a.stack_range().end - 0x100 - 0x14 - 0x24;
        let special: Vec<u32> = [0x00b5_bffc, 0x00b5_c004, 0x00b5_c000]
            .iter()
            .map(|&c| Mem::read(a, a.rebase(c)))
            .collect();
        for n in 0..6000u32 {
            stub_rets(a, b, &mut rng);
            reset_log(a, b);
            put(a, b, ctl + foot_ctl::LOG_COUNT, 0u32);
            put(a, b, ctl + foot_ctl::SCENARIO_CALLS, 0u32);
            let identity = rng.below(3) == 0;
            for src in [f3::CAMERA_MATRIX_4, f3::CAMERA_MATRIX, f3::VU0_MATRIX] {
                for i in 0..16 {
                    let v = if identity {
                        IDENTITY[i as usize]
                    } else {
                        quiet(foot_float(&mut rng))
                    };
                    put(a, b, a.rebase(src) + 4 * i, v);
                }
            }
            let flag = [0, 1, u32::MAX, 2][rng.below(4) as usize];
            put(a, b, a.rebase(0x0132_8308), flag);
            let vf00 = if rng.below(4) == 0 {
                quiet(foot_float(&mut rng))
            } else {
                0.0
            };
            put(a, b, a.rebase(f3::VU0_VF00_X), vf00);
            let ratio = if rng.below(3) == 0 {
                quiet(foot_float(&mut rng))
            } else {
                1.0
            };
            put(a, b, a.rebase(f3::VOLUME_RATIO), ratio);
            for i in 0..4 {
                put(
                    a,
                    b,
                    a.rebase(f3::SEGMENT_START) + 4 * i,
                    quiet(foot_float(&mut rng)),
                );
            }
            // the request
            let id = match rng.below(8) {
                0 => 0,
                1..=3 => 0x0001_1df8 + rng.below(6),
                _ => rng.next(),
            };
            put(a, b, req, id);
            put(a, b, req + 4, if rng.below(12) == 0 { 0 } else { p });
            let x = if identity && rng.below(2) == 0 {
                f32::from_bits(special[rng.below(3) as usize])
            } else {
                quiet(foot_float(&mut rng))
            };
            put(a, b, req + 8, x);
            for i in 1..6 {
                put(a, b, req + 8 + 4 * i, quiet(foot_float(&mut rng)));
            }
            let flags = (rng.next() & 7) as u16 | if rng.below(4) == 0 { 0x8000 } else { 0 };
            put(a, b, p, flags);
            put(a, b, p + 2, rng.next() as u8);
            put(a, b, p + 3, rng.next() as u8);
            put(a, b, p + 4, quiet(foot_float(&mut rng)));
            // distances around the request's, sometimes equal or reversed
            let mut d = [0f32; 4];
            for v in &mut d {
                *v = match rng.below(6) {
                    0 => quiet(foot_float(&mut rng)),
                    1 => 0.0,
                    _ => f32::from(u8::try_from(rng.below(40)).unwrap()),
                };
            }
            if rng.below(6) == 0 {
                d[1] = d[0];
            }
            if rng.below(6) == 0 {
                d[3] = d[2];
            }
            for (i, v) in (0..).zip(d) {
                put(a, b, p + 8 + 4 * i, v);
            }
            // the stand-ins' answers
            let x = if identity && rng.below(2) == 0 {
                f32::from_bits(special[rng.below(3) as usize])
            } else {
                quiet(foot_float(&mut rng))
            };
            put(a, b, ctl + foot_ctl::POS, x);
            for i in 1..3 {
                put(
                    a,
                    b,
                    ctl + foot_ctl::POS + 4 * i,
                    quiet(foot_float(&mut rng)),
                );
            }
            put(
                a,
                b,
                ctl + foot_ctl::KIND,
                [2, 5, 0, 1, 3][rng.below(5) as usize],
            );
            for i in 0..3 {
                put(
                    a,
                    b,
                    ctl + foot_ctl::ROT + 4 * i,
                    quiet(foot_float(&mut rng)),
                );
            }
            let angle = match rng.below(6) {
                0 => quiet(f32::from_bits(rng.next())),
                1 => std::f32::consts::FRAC_PI_2,
                2 => -std::f32::consts::FRAC_PI_2,
                _ => {
                    #[allow(clippy::cast_precision_loss)] // small integers
                    let v = (rng.next() % 6284) as f32 / 1000.0 - std::f32::consts::PI;
                    v
                }
            };
            put(a, b, ctl + foot_ctl::ANGLE, angle);
            put(
                a,
                b,
                ctl + foot_ctl::HASH,
                if rng.below(2) == 0 {
                    0x76f
                } else {
                    rng.below(0x800)
                },
            );
            for i in 0..2 {
                put(
                    a,
                    b,
                    ctl + foot_ctl::SCENARIO + 4 * i,
                    [2000, 0x7c6, 0x1_07d0, 0][rng.below(4) as usize],
                );
            }
            // what the original's frame holds before it runs: the
            // direction (words 0..4, read unwritten with flag 2 for an
            // actor of another type and flag 1) 0, the port's value (brief
            // 2026-10-24), which the port writes itself over garbage
            for i in 0..8 {
                let v = quiet(foot_float(&mut rng));
                if i < 4 {
                    Mem::write(a, frame + 4 * i, 0u32);
                    Mem::write(b, work + 4 * i, quiet(foot_float(&mut rng)));
                } else {
                    Mem::write(a, frame + 4 * i, v);
                    Mem::write(b, work + 4 * i, v);
                }
            }
            let play = rng.below(2);
            let mut count = [0, 2, 3, 0x100, u32::MAX][rng.below(5) as usize];
            if let Some(c) = FOOT_CRAFTED.get((n / 4) as usize) {
                // a request on the camera's x axis whose distance is |x|
                for src in [f3::CAMERA_MATRIX_4, f3::CAMERA_MATRIX, f3::VU0_MATRIX] {
                    for (i, v) in (0..).zip(IDENTITY) {
                        put(a, b, a.rebase(src) + 4 * i, v);
                    }
                }
                put(a, b, a.rebase(f3::VU0_VF00_X), 0u32);
                put(a, b, a.rebase(f3::VOLUME_RATIO), c.ratio);
                put(a, b, req, 0x100u32);
                put(a, b, req + 4, p);
                put(a, b, req + 8, [c.x, 0, 0]);
                put(a, b, p, 0u16);
                put(a, b, p + 2, [c.far_volume, c.near_volume]);
                put(
                    a,
                    b,
                    p + 4,
                    [c.pan_scale, c.volume.0, c.volume.1, c.pan.0, c.pan.1],
                );
                put(a, b, ctl + foot_ctl::ANGLE, c.angle);
                count = 3;
            }
            let at = a.rebase(s::FOOT3D_REQUEST);
            let ra = a
                .call(at, Conv::Cdecl, &[req, play, count])
                .unwrap_or_else(|e| panic!("{world} {n}: {e}"));
            assert!(ra.st0.is_none(), "{world} {n}: x87 stack left");
            let rb = f3::snd3d_foot_request(&mut RunF(b), req, play, count.cast_signed(), work);
            let ran = Mem::read::<u32>(a, req) != 0 && Mem::read::<u32>(a, req + 4) != 0;
            if ran {
                assert_eq!(ra.eax, rb, "{world} {n}: eax");
            }
            let fa: Vec<u32> = (0..8).map(|i| Mem::read(a, frame + 4 * i)).collect();
            let fb: Vec<u32> = (0..8).map(|i| Mem::read(b, work + 4 * i)).collect();
            assert_eq!(fa, fb, "{world} {n}: frame");
            let diff: Vec<_> = a
                .diff(b)
                .unwrap()
                .into_iter()
                .filter(|d| !(work..work + 0x20).contains(&d.0))
                .collect();
            assert_eq!(diff, [], "{world} {n}: memory (logged {})", logged(a));
        }
    }
}
