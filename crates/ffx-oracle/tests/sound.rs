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
    let Some(exe) = exe() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut a = Oracle::load(&exe).unwrap();
    let mut b = Oracle::load(&exe).unwrap();
    setup(&mut a, &mut b, false);
    out.push(("image", a, b));
    if let Some(snap) = std::env::var_os("FFX_SNAPSHOT") {
        let pages: Arc<Pages> = Pages::read(PathBuf::from(snap)).unwrap();
        let mut a = Oracle::from_snapshot(&exe, &pages).unwrap();
        let mut b = Oracle::from_snapshot(&exe, &pages).unwrap();
        setup(&mut a, &mut b, true);
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
/// The log: 32-byte entries `{tag, this, args[6]}`.
const LOG: u32 = 0x0271_0000;
const LOG_ENTRIES: u32 = 256;

/// The callees replaced by recorders: (address, stack words, thiscall).
const STUBS: &[(u32, u32, bool)] = &[
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
    (f::FMOD_INITIALIZE, 0, false),
    (fmod::PUSH_COMMAND, 5, true),
    (fmod::REGIST_COMMAND_SYNC, 5, true),
];

/// `SqsSeStop` (`SndStop` jumps to it).
const SQS_SE_STOP: u32 = 0x0081_e7c0;

/// x86 for a recorder: logs `tag`, `this` (thiscall) and `words` stack
/// arguments, returns `[ret]`, keeping `ecx` and `edx`.
fn recorder(tag: u32, words: u32, this: bool, ptr: u32, ret: u32) -> Vec<u8> {
    let mut c = vec![0x51, 0x52, 0x8b, 0x15];
    c.extend_from_slice(&ptr.to_le_bytes());
    c.extend_from_slice(&[0xc7, 0x02]);
    c.extend_from_slice(&tag.to_le_bytes());
    if this {
        c.extend_from_slice(&[0x89, 0x4a, 0x04]);
    }
    for i in 0..words {
        #[allow(clippy::cast_possible_truncation)] // small offsets
        let (from, to) = ((0x0c + 4 * i) as u8, (8 + 4 * i) as u8);
        c.extend_from_slice(&[0x8b, 0x4c, 0x24, from, 0x89, 0x4a, to]);
    }
    c.extend_from_slice(&[0x83, 0xc2, 0x20, 0x89, 0x15]);
    c.extend_from_slice(&ptr.to_le_bytes());
    c.extend_from_slice(&[0x5a, 0x59, 0xa1]);
    c.extend_from_slice(&ret.to_le_bytes());
    if this {
        #[allow(clippy::cast_possible_truncation)] // at most 5 words
        c.extend_from_slice(&[0xc2, (4 * words) as u8, 0]);
    } else {
        c.push(0xc3);
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

/// Installs the recorders and, on the image, a fake `FmodManager` chain.
fn setup(a: &mut Oracle, b: &mut Oracle, snapshot: bool) {
    for (k, &(sym, words, this)) in STUBS.iter().enumerate() {
        let k = u32::try_from(k).unwrap();
        let code = a.rebase(CODE) + 0x80 * k;
        let ret = a.rebase(RETS) + 4 * k;
        let rec = recorder(sym, words, this, a.rebase(LOG_PTR), ret);
        assert!(rec.len() <= 0x80);
        a.write(code, &rec).unwrap();
        b.write(code, &rec).unwrap();
        let at = a.rebase(sym);
        let rel = code.wrapping_sub(at + 5);
        let mut jmp = vec![0xe9];
        jmp.extend_from_slice(&rel.to_le_bytes());
        a.write(at, &jmp).unwrap();
        b.write(at, &jmp).unwrap();
    }
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
    let k = STUBS.iter().position(|s| s.0 == sym).expect("a recorder");
    let at = a.rebase(RETS) + 4 * u32::try_from(k).unwrap();
    put(a, b, at, v);
}

/// Random return values for every recorder.
fn stub_rets(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    for &(sym, _, _) in STUBS {
        let v = rng.word();
        stub_ret(a, b, sym, v);
    }
}

fn reset_log(a: &mut Oracle, b: &mut Oracle) {
    let log = a.rebase(LOG);
    put(a, b, a.rebase(LOG_PTR), log);
    let zero = vec![0u8; (LOG_ENTRIES * 32) as usize];
    a.write(log, &zero).unwrap();
    b.write(log, &zero).unwrap();
}

/// How many entries the case logged (the same in both, or the diff fails).
fn logged(a: &Oracle) -> u32 {
    (Mem::read::<u32>(a, a.rebase(LOG_PTR)) - a.rebase(LOG)) / 32
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
    reset_log(a, b);
    let at = a.rebase(sym);
    let want = a
        .call(at, conv, args)
        .unwrap_or_else(|e| panic!("{what} {args:x?}: {e}"))
        .eax;
    let got = bridge(&mut Run(b));
    if let Some(got) = got {
        assert_eq!(got, want, "{what} {args:x?}: eax");
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
