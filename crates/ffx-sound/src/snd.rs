//! The PS2 sound API (`Snd*`, `0x00885d00..0x008879d0`): the game-side
//! sound state (now-playing music, the music volume table, the delayed
//! music clear queue, footstep sound tables, voice/stream status words)
//! over the `Sqs*` sequencer layer and the FMOD command queue
//! ([`crate::fmod`]). `docs/formats/sound.md` has the layout.
//!
//! The PC build keeps the PS2 functions in alphabetical order, which names
//! the ones the Switch build's symbols have and the dump does not.

/// Sound-effect ids: bit 31 set for a block SE (kind 0), clear for a
/// separate SE (kind 1); `SndSep_IsValidHandle`/`SndSep_HandleIndex` split them for `Sqs*`.
#[must_use]
pub const fn sep_kind(id: u32) -> u32 {
    !(id >> 31) & 1
}

/// The id without its kind bit.
#[must_use]
pub const fn sep_id(id: u32) -> u32 {
    id & 0x7fff_ffff
}

/// Footstep sound tables: 16 kinds (8 and up are the "kick stay"
/// variants of 0..7), 16 party slots, and 1 or 4 variations.
pub mod foot {
    /// Where kind `k`'s table is (an `ffx.sym` address) and whether it has
    /// four variations per party slot (`SndGetFootSepWork`'s jump table).
    #[must_use]
    pub const fn table(kind: u32) -> (u32, bool) {
        match kind {
            1 => (0x0132_f790, true),
            2 => (0x0132_f990, false),
            3 => (0x0132_f9d0, false),
            4 => (0x0132_fa90, false),
            5 => (0x0132_f890, true),
            6 => (0x0132_fa10, false),
            7 => (0x0132_fa50, false),
            8 => (0x0132_fad0, true),
            9 => (0x0132_fbd0, true),
            10 => (0x0132_fdd0, false),
            11 => (0x0132_fe10, false),
            12 => (0x0132_fed0, false),
            13 => (0x0132_fcd0, true),
            14 => (0x0132_fe50, false),
            15 => (0x0132_fe90, false),
            _ => (0x0132_f690, true),
        }
    }

    /// `SndGetFootSepWork(kind, party, variation)`: the slot's address
    /// (`ffx.sym` coordinates). The party clamps to 0..15; a variation
    /// below 0 is 0, above 3 is 3, and ignored for one-variation kinds.
    #[must_use]
    pub const fn slot(kind: u32, party: i32, variation: i32) -> u32 {
        let party = if party < 0 {
            0
        } else if party > 15 {
            15
        } else {
            party
        }
        .cast_unsigned();
        let (base, four) = table(kind);
        if !four {
            return base + party * 4;
        }
        let v = if variation < 0 {
            0
        } else if variation > 3 {
            3
        } else {
            variation
        }
        .cast_unsigned();
        base + (v + party * 4) * 4
    }

    /// The highest variation `SndKickFoot` adds to a kind's sound
    /// (`0x00886140`).
    #[must_use]
    pub const fn max_variation(kind: u32) -> u32 {
        match kind {
            0..=3 | 5 | 8..=11 | 13 => 3,
            4 | 12 => 1,
            _ => 0,
        }
    }

    /// The highest variation index a kind's table has (`0x00886190`).
    #[must_use]
    pub const fn last_variation(kind: u32) -> u32 {
        match kind {
            2..=4 | 6 | 7 | 10..=12 | 14 | 15 => 0,
            _ => 3,
        }
    }
}

/// `SndStreamPCName(id, buf)`'s file name: `ve`/`vj` and the voice number
/// (`id >> 12`, `%06d`), two letters from bits 6..10 and 0..4 (`0x60 +`)
/// over the eighth and ninth characters, then `.vs`. Returns the bytes the
/// original writes at `buf` (from offset 0), with their offsets.
#[must_use]
pub fn stream_pc_name(id: u32, english: bool) -> Vec<(usize, u8)> {
    let mut s = Vec::new();
    s.extend_from_slice(if english { b"ve" } else { b"vj" });
    s.extend_from_slice(format!("{:06}", id >> 12).as_bytes());
    s.push(0);
    let mut out: Vec<(usize, u8)> = s.iter().copied().enumerate().collect();
    #[allow(clippy::cast_possible_truncation)] // masked to five bits
    let (hi, lo) = (((id >> 6) & 0x1f) as u8 + 0x60, (id & 0x1f) as u8 + 0x60);
    let mut buf = s;
    buf.resize(buf.len().max(11), 0);
    for (at, b) in [(8, hi), (9, lo), (10, 0)] {
        buf[at] = b;
        out.push((at, b));
    }
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    for (i, &b) in b".vs\0".iter().enumerate() {
        out.push((end + i, b));
    }
    out
}

/// The original functions, over game memory.
#[allow(clippy::many_single_char_names)] // the originals' arguments
pub mod bridge {
    use ffx_bridge::{Global, Mem, x87};

    use super::{foot, sep_id, sep_kind};
    use crate::fmod::{Conv, FmodHost as Host, bridge as i};

    // ---- state (docs/formats/sound.md, "Snd state")
    /// `g_TkSndVoiceFlags` (0x0132ff10): voice read in progress.
    pub const VOICE_READ: Global<u32> = Global::new(0x0132_ff10);
    /// `g_TkSndShoutRead` (0x0132ff14): shout stream busy.
    pub const SHOUT_READ: Global<u32> = Global::new(0x0132_ff14);
    /// `g_TkSndVagRead` (0x0132ff18): VAG stream busy.
    pub const VAG_READ: Global<u32> = Global::new(0x0132_ff18);
    /// `g_TkSndStandbyVoice` (0x0132ff1c).
    pub const STANDBY_VOICE: Global<u32> = Global::new(0x0132_ff1c);
    /// `g_SndSendingData` (0x0132ff20): `SqsDataSendingCompleted` result.
    pub const SENDING_DATA: Global<u32> = Global::new(0x0132_ff20);
    /// `g_TkSndStatus` (0x0132ff24).
    pub const STATUS: Global<u32> = Global::new(0x0132_ff24);
    /// `g_SndVoiceSectorLength` (0x0132ff28), set by `SndStreamDVDPos`.
    pub const VOICE_SECTORS: Global<u32> = Global::new(0x0132_ff28);
    /// `g_SndFoot3dMask` (0x0132ff2c), a party bit set; +4 its pushed copy.
    pub const FOOT3D_MASK: Global<u32> = Global::new(0x0132_ff2c);
    /// `g_SndFootMask` (0x0132ff34); +4 its pushed copy.
    pub const FOOT_MASK: Global<u32> = Global::new(0x0132_ff34);
    /// `g_SndKinuMask` (0x0132ff3c, cloth rustle); +4 its pushed copy.
    pub const KINU_MASK: Global<u32> = Global::new(0x0132_ff3c);
    /// `g_SndEventMask` (0x0132ff44); +4 its pushed copy.
    pub const EVENT_MASK: Global<u32> = Global::new(0x0132_ff44);
    /// `g_TkSndFootKickStay` (0x0132ff4c).
    pub const FOOT_KICK_STAY: Global<u32> = Global::new(0x0132_ff4c);
    /// `g_SndFoot3dCtrl` (0x0132ff50): the 3D footstep request.
    pub const FOOT3D_CTRL: Global<u32> = Global::new(0x0132_ff50);
    /// `g_SndSoundEvent` (0x0132ff54): `u32 count, u32 sep[count]`.
    pub const SOUND_EVENT: Global<u32> = Global::new(0x0132_ff54);
    /// `g_SndIopErrorStatus` (0x0132ff58): the last error bits.
    pub const IOP_ERROR: Global<u32> = Global::new(0x0132_ff58);
    /// `g_SndNowPlayMusic` (0x0132ff5c): the music playing, -1 for none.
    pub const NOW_PLAY: Global<u32> = Global::new(0x0132_ff5c);
    /// `g_SndIopStatus` (0x0132ff60): `u32[16]` status reports.
    pub const IOP_STATUS: u32 = 0x0132_ff60;
    /// `g_PartyWorkers` (0x0132ffa0): `i32[16]` actor per party slot.
    pub const PARTY_WORKERS: u32 = 0x0132_ffa0;
    /// `g_SndMusicClearQue` (0x0132ffe0): 17 entries `{i16 music (< 0: an
    /// SE), i16 frames, u32 sep}`.
    pub const CLEAR_QUE: u32 = 0x0132_ffe0;
    /// `g_SndMusicClearQueCount` (0x01330068).
    pub const CLEAR_COUNT: Global<i32> = Global::new(0x0133_0068);
    /// `g_SndStatusPrev` (0x0133006c).
    pub const STATUS_PREV: Global<u32> = Global::new(0x0133_006c);
    /// `g_SndStatusFrame` (0x01330070): status bits raised this frame.
    pub const STATUS_FRAME: Global<u32> = Global::new(0x0133_0070);
    /// `g_SndShoutPrev` (0x01330078).
    pub const SHOUT_PREV: Global<u32> = Global::new(0x0133_0078);
    /// `g_SndShoutFrame` (0x0133007c).
    pub const SHOUT_FRAME: Global<u32> = Global::new(0x0133_007c);
    /// Cleared by `SndInitWork` (0x01330080).
    pub const WORK_80: Global<u32> = Global::new(0x0133_0080);
    /// `g_SndVagPrev` (0x01330084).
    pub const VAG_PREV: Global<u32> = Global::new(0x0133_0084);
    /// `g_SndVagFrame` (0x01330088).
    pub const VAG_FRAME: Global<u32> = Global::new(0x0133_0088);
    /// Cleared by `SndInitWork` (0x0133008c).
    pub const WORK_8C: Global<u32> = Global::new(0x0133_008c);
    /// `g_SndMusicMute` (0x01330090, u8): music volumes become 0.
    pub const MUSIC_MUTE: Global<u8> = Global::new(0x0133_0090);
    /// `g_SndSepLock` (0x01330091, u8): `SndPlay` plays nothing.
    pub const SEP_LOCK: Global<u8> = Global::new(0x0133_0091);
    /// `g_SndSoundMode` (0x01330092, u8): 0..2.
    pub const SOUND_MODE: Global<u8> = Global::new(0x0133_0092);
    /// `g_SndSoundType` (0x01330093, u8): 0 or 1.
    pub const SOUND_TYPE: Global<u8> = Global::new(0x0133_0093);
    /// `g_TkSndKickFlag` (0x01330094, u8).
    pub const KICK_FLAG: Global<u8> = Global::new(0x0133_0094);
    /// `g_TkSndLastKickId` (0x01330095, u8).
    pub const LAST_KICK: Global<u8> = Global::new(0x0133_0095);
    /// `g_SndSpuMemorySortSwitch` (0x01330096, u8).
    pub const SPU_SORT: Global<u8> = Global::new(0x0133_0096);
    /// `g_SndHddError` (0x01330097, u8): set once the HDD read failed.
    pub const HDD_ERROR: Global<u8> = Global::new(0x0133_0097);
    /// `g_SndBattleMusicDataSize` (0x0133009c): `iMusicDataSend`'s result.
    pub const BATTLE_MUSIC_SIZE: Global<u32> = Global::new(0x0133_009c);
    /// `g_SndMusicVolume` (0x013300e0): `u8[256]` per music.
    pub const MUSIC_VOLUME: u32 = 0x0133_00e0;
    /// `g_SndLastFadeOutMusic` (0x00c54dd0): -1 in the image.
    pub const LAST_FADE_OUT: Global<u32> = Global::new(0x00c5_4dd0);
    /// `g_SndDefaultWaves` (0x00c54db0): `i16[16]`, -1 unused.
    pub const DEFAULT_WAVES: u32 = 0x00c5_4db0;
    /// `g_SndStreamTable` (0x023cbec4): the voice stream position table.
    pub const STREAM_TABLE: Global<u32> = Global::new(0x023c_bec4);
    /// The CD read wait flag (0x0133c913, u8).
    pub const CD_WAIT: Global<u8> = Global::new(0x0133_c913);
    /// The battle state `SndPlayClick` checks against 1 (0x0133d6e0).
    pub const CLICK_GATE: Global<u32> = Global::new(0x0133_d6e0);
    /// `FFXGetVideoMode`'s byte (0x01330831).
    pub const VIDEO_MODE: Global<i8> = Global::new(0x0133_0831);
    /// `g_TkLang` (0x01330830, i8).
    pub const LANG: Global<i8> = Global::new(0x0133_0830);

    // ---- callees left original
    /// `SqsDataSendingCompleted(x)`.
    pub const SQS_DATA_SENDING_COMPLETED: u32 = 0x0081_cfd0;
    /// `SqsSeForcePlay(kind, id, pan, volume)`.
    pub const SQS_SE_FORCE_PLAY: u32 = 0x0081_e410;
    /// `SqsEeSoundErrorStatus()`.
    pub const SQS_EE_SOUND_ERROR_STATUS: u32 = 0x0081_d2e0;
    /// `SqsSePlayState()`.
    pub const SQS_SE_PLAY_STATE: u32 = 0x0081_e530;
    /// `SqsSeBlockPlaying(id)`.
    pub const SQS_SE_BLOCK_PLAYING: u32 = 0x0081_e3e0;
    /// `SqsSeSeparatePlaying(id)`.
    pub const SQS_SE_SEPARATE_PLAYING: u32 = 0x0081_e7a0;
    /// `AtelClearCameraMatrixFlag()`.
    pub const ATEL_CLEAR_CAMERA_MATRIX_FLAG: u32 = 0x0087_72e0;
    /// `Snd3dFootRequest(req, 1, 0x100)` (the 3D footstep).
    pub const FOOT3D_REQUEST: u32 = 0x0087_6950;
    /// `SqsMusicDataRead(id)`.
    pub const SQS_MUSIC_DATA_READ: u32 = 0x0081_d820;
    /// `SqsSePlay(kind, id, pan, volume)`.
    pub const SQS_SE_PLAY: u32 = 0x0081_e4e0;
    /// `MsBattleCheck()`.
    pub const MS_BATTLE_CHECK: u32 = 0x0078_0c90;
    /// `SqsReportStandbyStatus(x)`.
    pub const SQS_REPORT_STANDBY_STATUS: u32 = 0x0081_e2d0;
    /// `SqsSePitchDirect(kind, id, pitch, x)`.
    pub const SQS_SE_PITCH_DIRECT: u32 = 0x0081_e4a0;
    /// `SqsSePitchTranslate(kind, id, a, pitch, b)`.
    pub const SQS_SE_PITCH_TRANSLATE: u32 = 0x0081_e4c0;
    /// `SqsSeSeparateClear(id)` (-1: all).
    pub const SQS_SE_SEPARATE_CLEAR: u32 = 0x0081_e5d0;
    /// `SqsSePanDirect(kind, id, pan)`.
    pub const SQS_SE_PAN_DIRECT: u32 = 0x0081_e460;
    /// `SqsSePanTranslate(kind, id, pan, frames)`.
    pub const SQS_SE_PAN_TRANSLATE: u32 = 0x0081_e480;
    /// `SndStop(kind, id)` (a `jmp SqsSeStop`).
    pub const SND_STOP: u32 = 0x0088_7750;
    /// `SqsSeSeparateDataSend(a, b, blocking)`.
    pub const SQS_SE_SEPARATE_DATA_SEND: u32 = 0x0081_e600;
    /// `SqsSeVolumeDirect(kind, id, volume)`.
    pub const SQS_SE_VOLUME_DIRECT: u32 = 0x0081_e820;
    /// `SqsSeVolumeFadeOut(kind, id, frames)`.
    pub const SQS_SE_VOLUME_FADE_OUT: u32 = 0x0081_e840;
    /// `SqsSeVolumeTranslate(kind, id, volume, frames)`.
    pub const SQS_SE_VOLUME_TRANSLATE: u32 = 0x0081_e860;
    /// `SqsIopCommandSend()`.
    pub const SQS_IOP_COMMAND_SEND: u32 = 0x0081_d5f0;
    /// `TkCdReadSync(mode)`.
    pub const TK_CD_READ_SYNC: u32 = 0x0088_d1c0;
    /// `TkRefreshHDDMode()`.
    pub const TK_REFRESH_HDD_MODE: u32 = 0x0088_e930;
    /// `set_hdd_err(on)`.
    pub const SET_HDD_ERR: u32 = 0x0077_9f70;
    /// `SqsSetPlayMode(mode)`.
    pub const SQS_SET_PLAY_MODE: u32 = 0x0081_ea00;
    /// `SqsStraightStreamingDigitalModeSet(x)`.
    pub const SQS_STRAIGHT_DIGITAL_MODE: u32 = 0x0081_f550;
    /// `SqsSortCommonWave(list)`.
    pub const SQS_SORT_COMMON_WAVE: u32 = 0x0081_ef40;
    /// `SqsSetStraightStreamAreaSize(x)`.
    pub const SQS_SET_STRAIGHT_STREAM_AREA_SIZE: u32 = 0x0081_ef50;
    /// `SqsIopSoundErrorStatus()`.
    pub const SQS_IOP_SOUND_ERROR_STATUS: u32 = 0x0081_d700;
    /// `SqsReportIopSoundStatus(i)`.
    pub const SQS_REPORT_IOP_SOUND_STATUS: u32 = 0x0081_e250;
    /// `iVoicePlayStatus()`.
    pub const I_VOICE_PLAY_STATUS: u32 = 0x0067_a710;
    /// `SqsShoutStreamingPlayState(x)`.
    pub const SQS_SHOUT_STREAMING_PLAY_STATE: u32 = 0x0081_edf0;
    /// `TkSndUpdateMain()`.
    pub const TK_SND_UPDATE_MAIN: u32 = 0x0088_7bd0;
    /// `TkSyncV(x)`.
    pub const TK_SYNC_V: u32 = 0x0088_7f00;
    /// `SqsBattleWaveDataSend(data, size)`.
    pub const SQS_BATTLE_WAVE_DATA_SEND: u32 = 0x0081_cf90;
    /// `SqsSetStandbyWave()` (a `ret`).
    pub const SQS_SET_STANDBY_WAVE: u32 = 0x0081_eca0;
    /// `iVoiceRead(id)`.
    pub const I_VOICE_READ: u32 = 0x0067_a720;
    /// `SqsBattleStreamingStart(a, b, c)`.
    pub const SQS_BATTLE_STREAMING_START: u32 = 0x0081_cf40;
    /// `TkCheckPlayVoice()`.
    pub const TK_CHECK_PLAY_VOICE: u32 = 0x0087_2340;
    /// `iShoutStart(a, b, volume)`.
    pub const I_SHOUT_START: u32 = 0x0067_a4e0;
    /// `SqsShoutStreamingStart(a, b, volume)`.
    pub const SQS_SHOUT_STREAMING_START: u32 = 0x0081_ee20;
    /// `iVoiceStart(pan, volume)`.
    pub const I_VOICE_START: u32 = 0x0067_a770;
    /// `TkSndSetStandbyVoice(x)`.
    pub const TK_SND_SET_STANDBY_VOICE: u32 = 0x0088_7b50;
    /// `iVoiceStop()`.
    pub const I_VOICE_STOP: u32 = 0x0067_a7a0;
    /// `iVoiceVolumeDirect(volume)`.
    pub const I_VOICE_VOLUME_DIRECT: u32 = 0x0067_a7b0;
    /// `iVoiceVolumeTranslate(frames, volume)`.
    pub const I_VOICE_VOLUME_TRANSLATE: u32 = 0x0067_a7d0;

    // SndCalcVoiceWait's doubles (.rdata)
    const D_2048: Global<f64> = Global::new(0x00b5_1bc0);
    const D_32: Global<f64> = Global::new(0x00b4_12f8);
    const D_48000: Global<f64> = Global::new(0x00b5_dd28);
    const D_16: Global<f64> = Global::new(0x00b3_13a0);
    const D_28: Global<f64> = Global::new(0x00b3_13a8);
    const D_2016: Global<f64> = Global::new(0x00b5_dd20);
    const D_1_2048: Global<f64> = Global::new(0x00b5_8590);
    const D_WAIT: Global<f64> = Global::new(0x00b5_dd18);

    fn call(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    fn w16(h: &impl Mem, a: u32) -> u32 {
        u32::from(h.read::<u16>(a))
    }

    fn or(h: &mut impl Mem, g: Global<u32>, bits: u32) {
        let v = g.get(h) | bits;
        g.set(h, v);
    }

    /// The `status |= frame |= 1` the transfer functions end with; the new
    /// frame bits (what they leave in `eax`).
    fn raise_status(h: &mut impl Mem) -> u32 {
        let s = STATUS_FRAME.get(h) | 1;
        or(h, STATUS, s);
        STATUS_FRAME.set(h, s);
        s
    }

    /// `while (shout | vag busy) { TkSndUpdateMain(); TkSyncV(0); }`.
    fn wait_streams(h: &mut impl Host) {
        while SHOUT_READ.get(h) | VAG_READ.get(h) != 0 {
            call(h, TK_SND_UPDATE_MAIN, &[]);
            call(h, TK_SYNC_V, &[0]);
        }
    }

    /// `SndSep_IsValidHandle(id)` (0x00885d00): the SE kind.
    #[must_use]
    pub fn snd_sep_kind(id: u32) -> u32 {
        sep_kind(id)
    }

    /// `SndSep_HandleIndex(id)` (0x00885d10).
    #[must_use]
    pub fn snd_sep_id(id: u32) -> u32 {
        sep_id(id)
    }

    /// `SndCalcVoiceWait()` (0x00885d20): frames to wait for the voice of
    /// `g_SndVoiceSectorLength` sectors, 0 in video mode 0. Each step is
    /// stored to `f32` as the original does; `ftol2_sse` at the end.
    pub fn snd_calc_voice_wait(h: &impl Mem) -> u32 {
        #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)] // fild, fstp dword
        let v = VOICE_SECTORS.get(h).cast_signed() as f32;
        if VIDEO_MODE.get(h) == 0 || v < 0.0 {
            return 0;
        }
        let r = |x: f64| -> f64 {
            #[allow(clippy::cast_possible_truncation)] // fstp dword
            let f = x as f32;
            f64::from(f)
        };
        let mut x = r(f64::from(v) * D_2048.get(h));
        x = r(x - D_32.get(h));
        x = r(x / D_48000.get(h));
        x = r(x * D_16.get(h) / D_28.get(h));
        x = r(x * D_2016.get(h) * D_1_2048.get(h));
        x = r(x * D_WAIT.get(h));
        x87::ftol(h, x)
    }

    /// `SndInitNowPlayMusic()` (0x00885db0).
    pub fn snd_init_now_play_music(h: &mut impl Mem) {
        NOW_PLAY.set(h, u32::MAX);
    }

    /// `SndClearMusicClearQue()` (0x00885dc0): count 0 and the first word
    /// of every entry 0.
    pub fn snd_clear_music_clear_que(h: &mut impl Mem) {
        CLEAR_COUNT.set(h, 0);
        let base = h.rebase(CLEAR_QUE);
        for i in 0..17 {
            h.write(base + 8 * i, 0u32);
        }
    }

    /// `SndClearFootSepWork(unused)` (0x00885de0): every footstep slot 0
    /// (only nonzero ones are written), the kick stay off.
    pub fn snd_clear_foot_sep_work(h: &mut impl Mem) {
        FOOT_KICK_STAY.set(h, 0);
        for kind in 0..16 {
            let last = foot::last_variation(kind);
            for party in 0..16 {
                for v in 0..=last {
                    let p = snd_get_foot_sep_work(h, kind, party, v.cast_signed());
                    if h.read::<u32>(p) != 0 {
                        h.write(p, 0u32);
                    }
                }
            }
        }
    }

    /// `SndClearFootStatus()` (0x00885e50): the four masks 0.
    pub fn snd_clear_foot_status(h: &mut impl Mem) {
        for g in [FOOT3D_MASK, FOOT_MASK, KINU_MASK, EVENT_MASK] {
            g.set(h, 0);
        }
    }

    /// `SndClearNowPlayMusic(id)` (0x00885e80): forgets `id` as playing.
    pub fn snd_clear_now_play_music(h: &mut impl Mem, id: u32) {
        if NOW_PLAY.get(h) == id {
            NOW_PLAY.set(h, u32::MAX);
        }
    }

    /// `SndCancelMusicClear(id)` (0x00885ea0): `SndExecMusicClearQue(0, id)`.
    pub fn snd_cancel_music_clear(h: &mut impl Host, id: u32) {
        snd_exec_music_clear_que(h, 0, id);
    }

    /// `SndDataSendSync(x)` (0x00885ee0).
    pub fn snd_data_send_sync(h: &mut impl Host, x: u32) -> u32 {
        let r = call(h, SQS_DATA_SENDING_COMPLETED, &[x]);
        SENDING_DATA.set(h, r);
        r
    }

    /// `SndEventClearPcToActorFoot()` (0x00885f10): every party slot's actor
    /// -1.
    pub fn snd_event_clear_pc_to_actor_foot(h: &mut impl Mem) {
        let base = h.rebase(PARTY_WORKERS);
        for i in 0..16 {
            h.write(base + 4 * i, u32::MAX);
        }
    }

    /// `SndEventSetFoot3dCtrlp(p)` (0x00885fc0); returns `p`.
    pub fn snd_event_set_foot3d_ctrlp(h: &mut impl Mem, p: u32) -> u32 {
        FOOT3D_CTRL.set(h, p);
        p
    }

    /// `SndEventSetPcToActorFoot(party, actor)` (0x00885fd0), for party
    /// below 16 (signed).
    pub fn snd_event_set_pc_to_actor_foot(h: &mut impl Mem, party: i32, actor: u32) {
        if party < 16 {
            let at = h
                .rebase(PARTY_WORKERS)
                .wrapping_add((party * 4).cast_unsigned());
            h.write(at, actor);
        }
    }

    /// `SndEventSetSoundEvent(table)` (0x00885ff0); returns `table`.
    pub fn snd_event_set_sound_event(h: &mut impl Mem, table: u32) -> u32 {
        SOUND_EVENT.set(h, table);
        table
    }

    /// `SndExecMusicClearQue(mode, id)` (0x00886000): the delayed music
    /// clears. Mode 0 cancels `id`'s music, -1 the SE `id`; any other mode
    /// counts every entry down and clears (`SndMusicClear`) or stops
    /// (`SndSepStop`) the music or SE whose count reaches 0. Then the
    /// spent entries are removed (the original's shift, which copies
    /// `count - i` entries past the removed one, kept as it is).
    pub fn snd_exec_music_clear_que(h: &mut impl Host, mode: u32, id: u32) {
        let base = h.rebase(CLEAR_QUE);
        let mut count = CLEAR_COUNT.get(h);
        let mut i = 0;
        while i < count {
            let e = base + 8 * i.cast_unsigned();
            let music = h.read::<i16>(e);
            let sep = h.read::<u32>(e + 4);
            if mode == 0 && i32::from(music).cast_unsigned() == id || mode == u32::MAX && sep == id
            {
                h.write(e + 2, 0u16);
            } else {
                let frames = h.read::<i16>(e + 2);
                if frames > 0 {
                    let left = frames - 1;
                    h.write(e + 2, left);
                    if left <= 0 {
                        if music < 0 {
                            snd_sep_stop(h, sep);
                        } else {
                            snd_music_clear(h, i32::from(music).cast_unsigned());
                        }
                        count = CLEAR_COUNT.get(h);
                    }
                }
            }
            i += 1;
        }
        let total = count;
        let mut kept = count;
        if count > 0 {
            let mut at = base;
            for idx in 1..=total {
                if h.read::<i16>(at + 2) > 0 {
                    at += 8;
                } else {
                    if idx < total {
                        let mut p = at;
                        for _ in 0..total - idx {
                            let a = h.read::<u32>(p + 8);
                            h.write(p, a);
                            let b = h.read::<u32>(p + 12);
                            h.write(p + 4, b);
                            p += 8;
                        }
                    }
                    kept -= 1;
                }
            }
            CLEAR_COUNT.set(h, kept);
        }
    }

    /// `SndForcePlayClick(id)` (0x008860f0): `SqsSeForcePlay(0, id, 63, 127)`.
    pub fn snd_force_play_click(h: &mut impl Host, id: u32) -> u32 {
        call(h, SQS_SE_FORCE_PLAY, &[0, sep_id(id), 0x3f, 0x7f])
    }

    /// `SndGetEventMask()` (0x00886120).
    pub fn snd_get_event_mask(h: &impl Mem) -> u32 {
        EVENT_MASK.get(h)
    }

    /// `SndGetFoot3dMask()` (0x00886130).
    pub fn snd_get_foot3d_mask(h: &impl Mem) -> u32 {
        FOOT3D_MASK.get(h)
    }

    /// `SndGetFootMaxVariation(kind)` (0x00886140).
    #[must_use]
    pub fn snd_get_foot_max_variation(kind: u32) -> u32 {
        foot::max_variation(kind)
    }

    /// `SndGetFootLastVariation(kind)` (0x00886190).
    #[must_use]
    pub fn snd_get_foot_last_variation(kind: u32) -> u32 {
        foot::last_variation(kind)
    }

    /// `SndGetFootMask()` (0x008861d0).
    pub fn snd_get_foot_mask(h: &impl Mem) -> u32 {
        FOOT_MASK.get(h)
    }

    /// `SndGetFootSepWork(kind, party, variation)` (0x008861e0): the slot.
    pub fn snd_get_foot_sep_work(h: &impl Mem, kind: u32, party: i32, variation: i32) -> u32 {
        h.rebase(foot::slot(kind, party, variation))
    }

    /// `SndGetKinuMask()` (0x008863b0).
    pub fn snd_get_kinu_mask(h: &impl Mem) -> u32 {
        KINU_MASK.get(h)
    }

    /// `SndGetMusicPlayState(id)` (0x008863c0): `iMusicPlayState`.
    pub fn snd_get_music_play_state(h: &mut impl Host, id: u32) -> u32 {
        i::i_music_play_state(h, id)
    }

    /// `SndGetMusicVolume(id)` (0x008863d0): the byte in `al` over `id`'s
    /// upper bytes, as the original leaves `eax`; `0xffffff00` for -1.
    pub fn snd_get_music_volume(h: &impl Mem, id: u32) -> u32 {
        if id == u32::MAX {
            return 0xffff_ff00;
        }
        let v = h.read::<u8>(h.rebase(MUSIC_VOLUME).wrapping_add(id));
        id & 0xffff_ff00 | u32::from(v)
    }

    /// `SndGetNowPlayMusic()` (0x008863f0).
    pub fn snd_get_now_play_music(h: &impl Mem) -> u32 {
        NOW_PLAY.get(h)
    }

    /// `SndGetSePlayState()` (0x00886400): `SqsSePlayState`.
    pub fn snd_get_se_play_state(h: &mut impl Host) -> u32 {
        call(h, SQS_SE_PLAY_STATE, &[])
    }

    /// `SndGetSepNumber(sep)` (0x00886410): `sep +8`.
    pub fn snd_get_sep_number(h: &impl Mem, sep: u32) -> u32 {
        h.read(sep.wrapping_add(8))
    }

    /// `SndGetSepSize(sep)` (0x00886420): `sep +0xc`.
    pub fn snd_get_sep_size(h: &impl Mem, sep: u32) -> u32 {
        h.read(sep.wrapping_add(0xc))
    }

    /// `SndGetSoundType()` (0x00886440).
    pub fn snd_get_sound_type(h: &impl Mem) -> u32 {
        u32::from(SOUND_TYPE.get(h))
    }

    /// `SndGetSpuMemorySortSwitch()` (0x00886450).
    pub fn snd_get_spu_memory_sort_switch(h: &impl Mem) -> u32 {
        u32::from(SPU_SORT.get(h))
    }

    /// `SndGetStreamState()` (0x008864b0): voice | shout, and the shout
    /// bits join the voice word.
    pub fn snd_get_stream_state(h: &mut impl Mem) -> u32 {
        let voice = VOICE_READ.get(h);
        let shout = SHOUT_READ.get(h);
        VOICE_READ.set(h, voice | shout);
        voice | shout
    }

    /// `SndGetWaveSize(wave)` (0x008864e0): `wave +4 + (wave +8 + 8 +
    /// wave +0xc * 8) * 4`.
    pub fn snd_get_wave_size(h: &impl Mem, wave: u32) -> u32 {
        let a = h.read::<u32>(wave.wrapping_add(4));
        let b = h.read::<u32>(wave.wrapping_add(8));
        let c = h.read::<u32>(wave.wrapping_add(0xc));
        a.wrapping_add(
            b.wrapping_add(8)
                .wrapping_add(c.wrapping_mul(8))
                .wrapping_mul(4),
        )
    }

    fn reset_foot(h: &mut impl Mem) {
        FOOT3D_MASK.set(h, 0);
        FOOT_MASK.set(h, u32::MAX);
        KINU_MASK.set(h, u32::MAX);
        EVENT_MASK.set(h, u32::MAX);
    }

    /// `SndInitFootSepWork()` (0x00886500).
    pub fn snd_init_foot_sep_work(h: &mut impl Mem) {
        snd_clear_foot_sep_work(h);
        reset_foot(h);
        FOOT3D_CTRL.set(h, 0);
        SOUND_EVENT.set(h, 0);
    }

    /// `SndInitWork()` (0x00886550).
    pub fn snd_init_work(h: &mut impl Host) {
        snd_init_foot_sep_work(h);
        STATUS.set(h, 0);
        VOICE_READ.set(h, 0);
        STANDBY_VOICE.set(h, 0);
        snd_init_now_play_music(h);
        snd_clear_music_clear_que(h);
        snd_spu_memory_sort_switch(h, 0);
        for g in [
            STATUS_PREV,
            SHOUT_PREV,
            VAG_PREV,
            STATUS_FRAME,
            SHOUT_FRAME,
            WORK_80,
            VAG_FRAME,
            WORK_8C,
        ] {
            g.set(h, 0);
        }
    }

    /// `SndIopEioErrorStatus()` (0x008865e0): bit 31 of
    /// `SqsEeSoundErrorStatus`.
    pub fn snd_iop_eio_error_status(h: &mut impl Host) -> u32 {
        u32::from(call(h, SQS_EE_SOUND_ERROR_STATUS, &[]) & 0x8000_0000 != 0)
    }

    /// `SndIsReadShoutVoiceStatus()` (0x00886640).
    pub fn snd_is_read_shout_voice_status(h: &impl Mem) -> u32 {
        SHOUT_READ.get(h)
    }

    /// `SndIsReadVagVoiceStatus()` (0x00886660).
    pub fn snd_is_read_vag_voice_status(h: &impl Mem) -> u32 {
        VAG_READ.get(h)
    }

    /// `SndIsSepPlay(id)` (0x00886680): block SEs ask `SqsSeBlockPlaying`,
    /// separate ones `SqsSeSeparatePlaying`.
    pub fn snd_is_sep_play(h: &mut impl Host, id: u32) -> u32 {
        let f = if sep_kind(id) == 0 {
            SQS_SE_BLOCK_PLAYING
        } else {
            SQS_SE_SEPARATE_PLAYING
        };
        call(h, f, &[sep_id(id)])
    }

    /// `SndKickFoot(kind, party, kick, variation)` (0x008866b0): plays a
    /// footstep (kind 0xff: the sound event `variation`) when the party
    /// slot's mask bit is set; through the 3D request when the slot is in
    /// the 3D mask and has an actor.
    pub fn snd_kick_foot(h: &mut impl Host, kind: u32, party: u32, kick: u32, variation: i32) {
        let (mask, sep) = if kind == 0xff {
            call(h, ATEL_CLEAR_CAMERA_MATRIX_FLAG, &[]);
            let mask = EVENT_MASK.get(h);
            let sep = snd_kick_foot_event(h, variation.cast_unsigned());
            if sep == 0 {
                return;
            }
            (mask, sep)
        } else {
            if party > 0xf {
                return;
            }
            call(h, ATEL_CLEAR_CAMERA_MATRIX_FLAG, &[]);
            let mask = if kind == 4 {
                KINU_MASK.get(h)
            } else {
                FOOT_MASK.get(h)
            };
            let var = if variation < 0 {
                0
            } else {
                variation.min(foot::max_variation(kind).cast_signed())
            };
            let kind = if FOOT_KICK_STAY.get(h) == 0 {
                kind
            } else {
                kind.wrapping_add(8)
            };
            let p = snd_get_foot_sep_work(h, kind, party.cast_signed(), kick.cast_signed());
            let sep = h.read::<u32>(p);
            if sep == 0 {
                return;
            }
            (mask, sep.wrapping_add(var.cast_unsigned()))
        };
        let actor = if party.cast_signed() < 0x10 {
            h.read::<u32>(h.rebase(PARTY_WORKERS).wrapping_add(party.wrapping_mul(4)))
        } else {
            0
        };
        let bit = 1u32 << (party & 31);
        if mask & bit == 0 {
            return;
        }
        let ctrl = FOOT3D_CTRL.get(h);
        if FOOT3D_MASK.get(h) & bit != 0 && ctrl != 0 && actor.cast_signed() >= 0 {
            #[allow(clippy::cast_possible_truncation)] // the actor's low word
            h.write(ctrl + 0x20, actor as u16);
            h.write(ctrl + 0x18, sep);
            let flags = h.read::<u16>(ctrl) & 0xfffe | 2;
            h.write(ctrl, flags);
            h.write(ctrl + 0x1c, ctrl);
            KICK_FLAG.set(h, 1);
            #[allow(clippy::cast_possible_truncation)] // the low byte
            LAST_KICK.set(h, kick as u8);
            call(h, FOOT3D_REQUEST, &[ctrl + 0x18, 1, 0x100]);
            return;
        }
        KICK_FLAG.set(h, 1);
        #[allow(clippy::cast_possible_truncation)] // the low byte
        LAST_KICK.set(h, kick as u8);
        snd_sep_play_simple(h, sep);
    }

    /// `SndKickFootEvent(i)` (0x008867e0): the event table's `i`th sound,
    /// 0 past its end or without a table.
    pub fn snd_kick_foot_event(h: &impl Mem, i: u32) -> u32 {
        let t = SOUND_EVENT.get(h);
        if t == 0 || h.read::<u32>(t) <= i {
            return 0;
        }
        h.read(t.wrapping_add(i.wrapping_mul(4)).wrapping_add(4))
    }

    /// `SndMusicClear(id)` (0x00886800), for ids below 0x100.
    pub fn snd_music_clear(h: &mut impl Host, id: u32) {
        if id <= 0xff {
            i::i_music_clear(h, id);
            snd_clear_now_play_music(h, id);
        }
    }

    fn music_volume(h: &mut impl Mem, id: u32, v: u8) {
        let at = h.rebase(MUSIC_VOLUME) + id;
        h.write(at, v);
    }

    /// `SndMusicCrossStart(id, frames)` (0x00886830): crosses to `id`
    /// (volume 0 when it is the one last faded out and nothing plays); an
    /// id of 0x100 or more fades the playing one out.
    pub fn snd_music_cross_start(h: &mut impl Host, id: u32, frames: u32) {
        let now = snd_get_now_play_music(h);
        if id == now {
            return;
        }
        if id > 0xff {
            snd_music_volume_fade_out(h, now, frames);
            return;
        }
        snd_cancel_music_clear(h, id);
        snd_regist_now_play_music(h, id);
        let v = if LAST_FADE_OUT.get(h) == id && now == u32::MAX {
            0
        } else {
            0x7f
        };
        music_volume(h, id, v);
        i::i_music_cross_play(h, id, frames);
    }

    /// `SndMusicMultiStart(id)` (0x008868b0).
    pub fn snd_music_multi_start(h: &mut impl Host, id: u32) {
        if id <= 0xff {
            snd_cancel_music_clear(h, id);
            snd_regist_now_play_music(h, id);
            i::i_music_multi_play(h, id);
            snd_music_volume_direct(h, id, 0x7f);
        }
    }

    /// `SndMusicPhraseStart(id, phrase)` (0x008868f0).
    pub fn snd_music_phrase_start(h: &mut impl Host, id: u32, phrase: u32) {
        if id <= 0xff {
            snd_cancel_music_clear(h, id);
            snd_regist_now_play_music(h, id);
            i::i_music_phrase_play(h, id, phrase);
            snd_music_volume_direct(h, id, 0x7f);
        }
    }

    /// `SndMusicPitchDirect(pitch, b)` (0x00886930): the pitch's low byte.
    pub fn snd_music_pitch_direct(h: &mut impl Host, pitch: u32, b: u32) -> u32 {
        i::i_music_pitch_direct(h, pitch & 0xff, b)
    }

    /// `SndMusicPitchTranslate(id, pitch, frames)` (0x00886940): the pitch's
    /// low byte to `iMusicVolumeTranslate` (as the PC build has it).
    pub fn snd_music_pitch_translate(h: &mut impl Host, id: u32, pitch: u32, frames: u32) -> u32 {
        i::i_music_volume_translate(h, id, pitch & 0xff, frames)
    }

    /// `SndMusicRead(id)` (0x00886960): waits for the streams, then loads.
    pub fn snd_music_read(h: &mut impl Host, id: u32) {
        if id <= 0xff {
            snd_cancel_music_clear(h, id);
            wait_streams(h);
            i::i_music_read(h, id);
            call(h, SQS_MUSIC_DATA_READ, &[id]);
            raise_status(h);
        }
    }

    /// `SndMusicStandby(id)` (0x00886a00): `iMusicDataSend`'s result kept
    /// as the battle music size.
    pub fn snd_music_standby(h: &mut impl Host, id: u32) -> u32 {
        let r = i::i_music_data_send(h, id);
        BATTLE_MUSIC_SIZE.set(h, r);
        r
    }

    /// `SndMusicStart(id)` (0x00886a20).
    pub fn snd_music_start(h: &mut impl Host, id: u32) {
        if id <= 0xff {
            snd_cancel_music_clear(h, id);
            snd_regist_now_play_music(h, id);
            i::i_music_start(h, id);
            music_volume(h, id, 0x7f);
            snd_music_volume_direct(h, id, 0x7f);
        }
    }

    /// `SndMusicStop(id)` (0x00886a60).
    pub fn snd_music_stop(h: &mut impl Host, id: u32) {
        if id <= 0xff {
            i::i_music_stop(h, id);
            snd_clear_now_play_music(h, id);
        }
    }

    /// `SndMusicTempoDirect(tempo)` (0x00886a90): the low byte.
    pub fn snd_music_tempo_direct(h: &mut impl Host, tempo: u32) -> u32 {
        i::i_music_tempo_direct(h, tempo & 0xff)
    }

    /// `SndMusicTempoTranslate(a, tempo)` (0x00886aa0): the low byte.
    pub fn snd_music_tempo_translate(h: &mut impl Host, a: u32, tempo: u32) -> u32 {
        i::i_music_tempo_translate(h, a, tempo & 0xff)
    }

    fn unless_muted(h: &impl Mem, v: u32) -> u32 {
        if MUSIC_MUTE.get(h) == 0 { v } else { 0 }
    }

    /// `SndMusicVolumeDirect(id, volume)` (0x00886ab0): 0 while muted.
    pub fn snd_music_volume_direct(h: &mut impl Host, id: u32, volume: u32) {
        let v = unless_muted(h, volume);
        if id <= 0xff {
            i::i_music_volume_direct(h, id, v);
            #[allow(clippy::cast_possible_truncation)] // the low byte
            music_volume(h, id, v as u8);
        }
    }

    /// `SndMusicVolumeFadeOut(id, frames)` (0x00886af0): fades out and
    /// remembers `id` as the last faded one.
    pub fn snd_music_volume_fade_out(h: &mut impl Host, id: u32, frames: u32) {
        if id <= 0xff {
            i::i_music_volume_fade_out(h, id, frames);
            music_volume(h, id, 0);
            LAST_FADE_OUT.set(h, id);
            snd_clear_now_play_music(h, id);
            snd_cancel_music_clear(h, id);
        }
    }

    /// `SndMusicVolumeTranslate(id, frames, volume)` (0x00886b40).
    pub fn snd_music_volume_translate(h: &mut impl Host, id: u32, frames: u32, volume: u32) {
        let v = unless_muted(h, volume);
        if id <= 0xff {
            i::i_music_volume_translate(h, id, frames, v);
            #[allow(clippy::cast_possible_truncation)] // the low byte
            music_volume(h, id, v as u8);
        }
    }

    /// `SndPlay(kind, id, pan, volume)` (0x00886b80): `SqsSePlay` unless
    /// locked (0 then; the original leaves `eax` as it was).
    pub fn snd_play(h: &mut impl Host, kind: u32, id: u32, pan: u32, volume: u32) -> u32 {
        if SEP_LOCK.get(h) != 0 {
            return 0;
        }
        call(h, SQS_SE_PLAY, &[kind, id, pan, volume])
    }

    /// `SndPlayClick(id)` (0x00886ba0): a menu click, not during a battle
    /// check (`MsBattleCheck` nonzero is returned as it is).
    pub fn snd_play_click(h: &mut impl Host, id: u32) -> u32 {
        if CLICK_GATE.get(h) == 1 {
            let r = call(h, MS_BATTLE_CHECK, &[]);
            if r != 0 {
                return r;
            }
        }
        call(h, SQS_SE_PLAY, &[0, sep_id(id), 0x3f, 0x7f])
    }

    /// `SndPlayFootStatus()` (0x00886be0): 3D off, the others all on.
    pub fn snd_play_foot_status(h: &mut impl Mem) {
        reset_foot(h);
    }

    /// `SndPopFootStatus()` (0x00886c10): the pushed masks back; returns the
    /// event mask.
    pub fn snd_pop_foot_status(h: &mut impl Mem) -> u32 {
        let mut v = 0;
        for g in [FOOT3D_MASK, FOOT_MASK, KINU_MASK, EVENT_MASK] {
            v = h.read::<u32>(g.ptr(h).addr + 4);
            g.set(h, v);
        }
        v
    }

    /// `SndPushFootStatus()` (0x00886c40): keeps the masks; returns the event
    /// mask.
    pub fn snd_push_foot_status(h: &mut impl Mem) -> u32 {
        let mut v = 0;
        for g in [FOOT3D_MASK, FOOT_MASK, KINU_MASK, EVENT_MASK] {
            v = g.get(h);
            let at = g.ptr(h).addr + 4;
            h.write(at, v);
        }
        v
    }

    /// `SndRegistNowPlayMusic(id)` (0x00886c70): `id` playing, unless it
    /// already is (a one-entry list).
    pub fn snd_regist_now_play_music(h: &mut impl Mem, id: u32) {
        let cur = NOW_PLAY.get(h);
        if cur == id {
            return;
        }
        NOW_PLAY.set(h, id);
    }

    /// `SndRemoveFootSepWork()` (0x00886cb0).
    pub fn snd_remove_foot_sep_work(h: &mut impl Mem) {
        snd_init_foot_sep_work(h);
    }

    /// `SndReportStandbyStatusDirect(x)` (0x00886d00).
    pub fn snd_report_standby_status_direct(h: &mut impl Host, x: u32) -> u32 {
        let r = call(h, SQS_REPORT_STANDBY_STATUS, &[x]);
        or(h, STATUS, r);
        or(h, STATUS_FRAME, r);
        r
    }

    /// `SndSePitchDirect(id, pitch, x)` (0x00886d20): the pitch's low byte.
    pub fn snd_se_pitch_direct(h: &mut impl Host, id: u32, pitch: u32, x: u32) -> u32 {
        call(
            h,
            SQS_SE_PITCH_DIRECT,
            &[sep_kind(id), sep_id(id), pitch & 0xff, x],
        )
    }

    /// `SndSePitchTranslate(id, a, pitch, b)` (0x00886d50).
    pub fn snd_se_pitch_translate(h: &mut impl Host, id: u32, a: u32, pitch: u32, b: u32) -> u32 {
        call(
            h,
            SQS_SE_PITCH_TRANSLATE,
            &[sep_kind(id), sep_id(id), a, pitch & 0xff, b],
        )
    }

    /// `SndSepClear(id)` (0x00886d80): -1 clears every separate SE; a block
    /// SE nothing (0).
    pub fn snd_sep_clear(h: &mut impl Host, id: u32) -> u32 {
        let arg = if id == u32::MAX {
            u32::MAX
        } else if sep_kind(id) == 0 {
            return 0;
        } else {
            sep_id(id)
        };
        call(h, SQS_SE_SEPARATE_CLEAR, &[arg])
    }

    /// `SndSepClearAll()` (0x00886db0).
    pub fn snd_sep_clear_all(h: &mut impl Host) -> u32 {
        call(h, SQS_SE_SEPARATE_CLEAR, &[u32::MAX])
    }

    /// `SndSepLock(on)` (0x00886dd0); returns `on`.
    pub fn snd_sep_lock(h: &mut impl Mem, on: u32) -> u32 {
        SEP_LOCK.set(h, u8::from(on != 0));
        on
    }

    /// `SndSepPanDirect(id, pan)` (0x00886df0).
    pub fn snd_sep_pan_direct(h: &mut impl Host, id: u32, pan: u32) -> u32 {
        call(h, SQS_SE_PAN_DIRECT, &[sep_kind(id), sep_id(id), pan])
    }

    /// `SndSepPanTranslate(id, pan, frames)` (0x00886e20).
    pub fn snd_sep_pan_translate(h: &mut impl Host, id: u32, pan: u32, frames: u32) -> u32 {
        call(
            h,
            SQS_SE_PAN_TRANSLATE,
            &[sep_kind(id), sep_id(id), pan, frames],
        )
    }

    /// `SndSepPlay(id, pan, volume)` (0x00886e50).
    pub fn snd_sep_play(h: &mut impl Host, id: u32, pan: u32, volume: u32) -> u32 {
        snd_play(h, sep_kind(id), sep_id(id), pan, volume)
    }

    /// `SndSepPlaySimple(id)` (0x00886e80): centred, full volume.
    pub fn snd_sep_play_simple(h: &mut impl Host, id: u32) -> u32 {
        snd_play(h, sep_kind(id), sep_id(id), 0x3f, 0x7f)
    }

    /// `SndSepStop(id)` (0x00886eb0).
    pub fn snd_sep_stop(h: &mut impl Host, id: u32) -> u32 {
        call(h, SND_STOP, &[sep_kind(id), sep_id(id)])
    }

    /// `SndSepTrans(a, b)` (0x00886ee0): a blocking separate-data send.
    pub fn snd_sep_trans(h: &mut impl Host, a: u32, b: u32) -> u32 {
        call(h, SQS_SE_SEPARATE_DATA_SEND, &[a, b, 1])
    }

    /// `SndSepTransNonblock(a, b)` (0x00886f00).
    pub fn snd_sep_trans_nonblock(h: &mut impl Host, a: u32, b: u32) -> u32 {
        call(h, SQS_SE_SEPARATE_DATA_SEND, &[a, b, 1]);
        raise_status(h)
    }

    /// `SndSepTransSepOnly(a, b)` (0x00886f30).
    pub fn snd_sep_trans_sep_only(h: &mut impl Host, a: u32, b: u32) -> u32 {
        call(h, SQS_SE_SEPARATE_DATA_SEND, &[a, b, 0]);
        raise_status(h)
    }

    /// `SndSepVolumeDirect(id, volume)` (0x00886f60).
    pub fn snd_sep_volume_direct(h: &mut impl Host, id: u32, volume: u32) -> u32 {
        call(h, SQS_SE_VOLUME_DIRECT, &[sep_kind(id), sep_id(id), volume])
    }

    /// `SndSepVolumeFadeOut(id, frames)` (0x00886f90).
    pub fn snd_sep_volume_fade_out(h: &mut impl Host, id: u32, frames: u32) -> u32 {
        call(
            h,
            SQS_SE_VOLUME_FADE_OUT,
            &[sep_kind(id), sep_id(id), frames],
        )
    }

    /// `SndSepVolumeFadeOut2(id, volume)` (0x00886fc0): a translation over 0
    /// frames.
    pub fn snd_sep_volume_fade_out2(h: &mut impl Host, id: u32, volume: u32) -> u32 {
        call(
            h,
            SQS_SE_VOLUME_TRANSLATE,
            &[sep_kind(id), sep_id(id), volume, 0],
        )
    }

    /// `SndSepVolumeTranslate(id, volume, frames)` (0x00886ff0).
    pub fn snd_sep_volume_translate(h: &mut impl Host, id: u32, volume: u32, frames: u32) -> u32 {
        call(
            h,
            SQS_SE_VOLUME_TRANSLATE,
            &[sep_kind(id), sep_id(id), volume, frames],
        )
    }

    /// `SndSeqIopCommandSendMain()` (0x00887020): sends the IOP commands;
    /// a first EIO error while the CD wait flag is up switches to the HDD
    /// error mode once.
    pub fn snd_seq_iop_command_send_main(h: &mut impl Host) {
        call(h, SQS_IOP_COMMAND_SEND, &[]);
        if HDD_ERROR.get(h) == 0
            && CD_WAIT.get(h) != 0
            && snd_iop_eio_error_status(h) != 0
            && call(h, TK_CD_READ_SYNC, &[1]) == 0
        {
            CD_WAIT.set(h, 0);
            HDD_ERROR.set(h, 1);
            call(h, TK_REFRESH_HDD_MODE, &[]);
            call(h, SET_HDD_ERR, &[1]);
        }
    }

    fn set_bit(h: &mut impl Mem, g: Global<u32>, bit: u32, on: u32) -> u32 {
        let m = 1u32 << (bit & 31);
        let v = if on != 0 { g.get(h) | m } else { g.get(h) & !m };
        g.set(h, v);
        if on != 0 { m } else { !m }
    }

    /// `SndSetEventStatus(party, on)` (0x00887080); returns the bit (its
    /// complement when clearing), as `eax` holds.
    pub fn snd_set_event_status(h: &mut impl Mem, party: u32, on: u32) -> u32 {
        set_bit(h, EVENT_MASK, party, on)
    }

    /// `SndSetFoot3dStatus(party, on)` (0x008870b0).
    pub fn snd_set_foot3d_status(h: &mut impl Mem, party: u32, on: u32) -> u32 {
        set_bit(h, FOOT3D_MASK, party, on)
    }

    /// `SndSetFootSep(kind, party, variation, sep)` (0x008870e0); returns the
    /// slot.
    pub fn snd_set_foot_sep(
        h: &mut impl Mem,
        kind: u32,
        party: i32,
        variation: i32,
        sep: u32,
    ) -> u32 {
        let p = snd_get_foot_sep_work(h, kind, party, variation);
        h.write(p, sep);
        p
    }

    /// `SndSetFootStatus(party, on)` (0x00887100).
    pub fn snd_set_foot_status(h: &mut impl Mem, party: u32, on: u32) -> u32 {
        set_bit(h, FOOT_MASK, party, on)
    }

    /// `SndSetKinuStatus(party, on)` (0x00887130).
    pub fn snd_set_kinu_status(h: &mut impl Mem, party: u32, on: u32) -> u32 {
        set_bit(h, KINU_MASK, party, on)
    }

    /// `SndSetMusicMute()` (0x00887160): unmutes (the PC build has no
    /// argument).
    pub fn snd_set_music_mute(h: &mut impl Mem) {
        MUSIC_MUTE.set(h, 0);
    }

    /// `SndSetSoundMode(mode)` (0x00887180): clamped to 0..2.
    pub fn snd_set_sound_mode(h: &mut impl Host, mode: i32) -> u32 {
        let m = mode.clamp(0, 2).cast_unsigned();
        let r = call(h, SQS_SET_PLAY_MODE, &[m]);
        #[allow(clippy::cast_possible_truncation)] // 0..2
        SOUND_MODE.set(h, m as u8);
        r
    }

    /// `SndSetSoundType(t)` (0x008871c0): 0 below 0, 1 above 1; returns `t`.
    pub fn snd_set_sound_type(h: &mut impl Mem, t: i32) -> u32 {
        SOUND_TYPE.set(h, u8::try_from(t.clamp(0, 1)).unwrap_or(0));
        t.cast_unsigned()
    }

    /// `SndSetStraightStreamingDigitalMode(x)` (0x00887210).
    pub fn snd_set_straight_streaming_digital_mode(h: &mut impl Host, x: u32) -> u32 {
        call(h, SQS_STRAIGHT_DIGITAL_MODE, &[x]);
        raise_status(h)
    }

    /// `SndSetWaveData(list)` (0x00887240): loads each wave of the 16-entry
    /// list (the default list for 0), -1 skipped.
    pub fn snd_set_wave_data(h: &mut impl Host, list: u32) {
        let list = if list == 0 {
            h.rebase(DEFAULT_WAVES)
        } else {
            list
        };
        STATUS.set(h, 1);
        for k in 0..16 {
            let w = h.read::<i16>(list.wrapping_add(2 * k));
            if w != -1 {
                i::i_sfx_load_data(h, i32::from(w).cast_unsigned(), 1);
            }
        }
    }

    /// `SndSortCommonWave(list)` (0x00887280).
    pub fn snd_sort_common_wave(h: &mut impl Host, list: u32) -> u32 {
        let list = if list == 0 {
            h.rebase(DEFAULT_WAVES)
        } else {
            list
        };
        call(h, SQS_SORT_COMMON_WAVE, &[list])
    }

    /// `SndSortCommonWaveWithSet(list)` (0x008872a0).
    pub fn snd_sort_common_wave_with_set(h: &mut impl Host, list: u32) {
        snd_sort_common_wave(h, list);
        snd_set_wave_data(h, list);
    }

    /// `SndSpuMemorySortSwitch(x)` (0x008872c0).
    pub fn snd_spu_memory_sort_switch(h: &mut impl Host, x: u32) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // the low byte
        SPU_SORT.set(h, x as u8);
        call(h, SQS_SET_STRAIGHT_STREAM_AREA_SIZE, &[x])
    }

    /// `SndSqsIopFrame()` (0x00887310): sends the IOP commands and moves
    /// this frame's status words to the previous ones; returns the frame
    /// status.
    pub fn snd_sqs_iop_frame(h: &mut impl Host) -> u32 {
        snd_seq_iop_command_send_main(h);
        let v = VAG_FRAME.get(h);
        VAG_PREV.set(h, v);
        let s = SHOUT_FRAME.get(h);
        SHOUT_PREV.set(h, s);
        let f = STATUS_FRAME.get(h);
        VAG_FRAME.set(h, 0);
        SHOUT_FRAME.set(h, 0);
        STATUS_PREV.set(h, f);
        STATUS_FRAME.set(h, 0);
        f
    }

    /// `SndSqsIopSoundErrorStatus()` (0x00887360): reports each newly set
    /// error bit's status.
    pub fn snd_sqs_iop_sound_error_status(h: &mut impl Host) -> u32 {
        let r = call(h, SQS_IOP_SOUND_ERROR_STATUS, &[]);
        for i in 0..16 {
            let bit = 1u32 << i;
            if r & bit != 0 && IOP_ERROR.get(h) & bit == 0 {
                let s = call(h, SQS_REPORT_IOP_SOUND_STATUS, &[i]);
                let at = h.rebase(IOP_STATUS) + 4 * i;
                h.write(at, s);
            }
        }
        IOP_ERROR.set(h, r);
        r
    }

    /// `SndSqsReportIopSoundStatus(i)` (0x008873c0).
    pub fn snd_sqs_report_iop_sound_status(h: &impl Mem, i: u32) -> u32 {
        h.read(h.rebase(IOP_STATUS).wrapping_add(i.wrapping_mul(4)))
    }

    /// `SndSqsShoutStreamingPlayState()` (0x008873f0).
    pub fn snd_sqs_shout_streaming_play_state(h: &mut impl Host) -> u32 {
        let v = call(h, I_VOICE_PLAY_STATUS, &[]);
        or(h, SHOUT_FRAME, v);
        let s = call(h, SQS_SHOUT_STREAMING_PLAY_STATE, &[0]);
        or(h, SHOUT_FRAME, s);
        SHOUT_PREV.get(h) | SHOUT_FRAME.get(h)
    }

    /// `SndSqsVagStreamingPlayState()` (0x00887420).
    pub fn snd_sqs_vag_streaming_play_state(h: &mut impl Host) -> u32 {
        let v = call(h, I_VOICE_PLAY_STATUS, &[]);
        or(h, VAG_FRAME, v);
        VAG_PREV.get(h) | VAG_FRAME.get(h)
    }

    /// `SndStandbyBattleWave(data, size)` (0x00887450): sizes from
    /// `0x30000` are refused.
    pub fn snd_standby_battle_wave(h: &mut impl Host, data: u32, size: i32) {
        STATUS.set(h, 1);
        SENDING_DATA.set(h, 1);
        if size >= 0x30000 {
            return;
        }
        wait_streams(h);
        call(h, SQS_BATTLE_WAVE_DATA_SEND, &[data, size.cast_unsigned()]);
        snd_sqs_iop_frame(h);
        raise_status(h);
    }

    /// `SndStandbyBattleWaveBlock(data, size)` (0x008874d0).
    pub fn snd_standby_battle_wave_block(h: &mut impl Host, data: u32, size: i32) {
        STATUS.set(h, 1);
        SENDING_DATA.set(h, 1);
        if size >= 0x30000 {
            return;
        }
        wait_streams(h);
        call(h, SQS_BATTLE_WAVE_DATA_SEND, &[data, size.cast_unsigned()]);
        snd_sqs_iop_frame(h);
        let _ = snd_sync_wave_trans(1);
    }

    /// `SndStandbySepWave()` (0x00887550).
    pub fn snd_standby_sep_wave(h: &mut impl Host) {
        STATUS.set(h, 1);
        call(h, SQS_SET_STANDBY_WAVE, &[]);
    }

    /// `SndStandbyStream(id)` (0x00887570): reads voice `id`.
    pub fn snd_standby_stream(h: &mut impl Host, id: u32) {
        call(h, I_VOICE_READ, &[id]);
        raise_status(h);
        STANDBY_VOICE.set(h, 1);
    }

    /// `SndStartBattleStream(a, b, c)` (0x008875a0).
    pub fn snd_start_battle_stream(h: &mut impl Host, a: u32, b: u32, c: u32) -> u32 {
        call(h, SQS_BATTLE_STREAMING_START, &[a, b, c]);
        raise_status(h)
    }

    fn voice_volume(h: &mut impl Host, v: u32) -> u32 {
        if call(h, TK_CHECK_PLAY_VOICE, &[]) != 0 {
            v
        } else {
            0
        }
    }

    /// `SndStartShoutStream(a, b, volume)` (0x008875d0): volume 0 when voices
    /// are off.
    pub fn snd_start_shout_stream(h: &mut impl Host, a: u32, b: u32, volume: u32) -> u32 {
        let v = voice_volume(h, volume);
        call(h, I_SHOUT_START, &[a, b, v]);
        call(h, SQS_SHOUT_STREAMING_START, &[a, b, v]);
        let s = SHOUT_FRAME.get(h) | 1;
        or(h, SHOUT_READ, s);
        SHOUT_FRAME.set(h, s);
        s
    }

    fn voice_started(h: &mut impl Host) -> u32 {
        raise_status(h);
        let s = VAG_FRAME.get(h) | 1;
        or(h, VAG_READ, s);
        VAG_FRAME.set(h, s);
        s
    }

    /// `SndStartStream()` (0x00887630): plays the read voice centred.
    pub fn snd_start_stream(h: &mut impl Host) -> u32 {
        or(h, VOICE_READ, 1);
        or(h, VAG_READ, 1);
        STANDBY_VOICE.set(h, 0);
        let v = voice_volume(h, 0x7f);
        call(h, I_VOICE_START, &[0x3f, v]);
        voice_started(h)
    }

    /// `SndStartStream2(pan, volume)` (0x00887690).
    pub fn snd_start_stream2(h: &mut impl Host, pan: u32, volume: u32) -> u32 {
        or(h, VAG_READ, 1);
        VOICE_READ.set(h, 1);
        STANDBY_VOICE.set(h, 0);
        let v = voice_volume(h, volume);
        call(h, I_VOICE_START, &[pan, v]);
        voice_started(h)
    }

    /// `SndStatusUpdate()` (0x00887700): folds the stream states into
    /// the busy words.
    pub fn snd_status_update(h: &mut impl Host) {
        let vag = snd_sqs_vag_streaming_play_state(h);
        let shout = snd_sqs_shout_streaming_play_state(h);
        let vag = VAG_READ.get(h) | vag;
        let shout = SHOUT_READ.get(h) | shout;
        VAG_READ.set(h, vag);
        or(h, VOICE_READ, vag | shout);
        SHOUT_READ.set(h, shout);
        let r = call(h, SQS_DATA_SENDING_COMPLETED, &[0]);
        or(h, SENDING_DATA, r);
    }

    /// `SndStopBattleStream()` (0x00887760).
    pub fn snd_stop_battle_stream(h: &mut impl Host) -> u32 {
        call(h, TK_SND_SET_STANDBY_VOICE, &[0]);
        call(h, I_VOICE_STOP, &[])
    }

    /// `SndStopStream()` (0x00887780).
    pub fn snd_stop_stream(h: &mut impl Host) -> u32 {
        snd_stop_battle_stream(h)
    }

    /// `SndStreamDVDPos(id)` (0x00887790): the voice's position in the
    /// stream table (`id >> 12` as three two-digit indices), `(group << 16)
    /// | sector`, -1 if absent; its length goes to
    /// `g_SndVoiceSectorLength`.
    pub fn snd_stream_dvd_pos(h: &mut impl Mem, id: u32) -> u32 {
        let x = id >> 12;
        let (d0, d1, d2) = (x % 100, (x / 100) % 100, (x / 10000) % 100);
        VOICE_SECTORS.set(h, 0);
        let t = STREAM_TABLE.get(h);
        if t == 0 || d2 >= w16(h, t) {
            return u32::MAX;
        }
        let g = t.wrapping_add(w16(h, t.wrapping_add(d2 * 2 + 2)));
        if d1 >= w16(h, g) {
            return u32::MAX;
        }
        let e = t.wrapping_add(w16(h, g.wrapping_add(d1 * 2 + 2)));
        if d0 >= w16(h, e) {
            return u32::MAX;
        }
        let v = w16(h, e.wrapping_add(d0 * 2 + 2));
        if v == 0xffff {
            return u32::MAX;
        }
        let pos = v | d2 << 16;
        let mut p = e.wrapping_add(d0 * 2 + 4);
        let mut n = w16(h, p);
        while n == 0xffff {
            n = w16(h, p.wrapping_add(2));
            p = p.wrapping_add(2);
        }
        VOICE_SECTORS.set(h, n.saturating_sub(v));
        pos
    }

    /// `SndStreamPCName(id, buf)` (0x00887870): the voice file name
    /// ([`super::stream_pc_name`]).
    pub fn snd_stream_pc_name(h: &mut impl Mem, id: u32, buf: u32) {
        let english = i32::from(LANG.get(h)).cast_unsigned() <= 6;
        for (at, b) in super::stream_pc_name(id, english) {
            h.write(buf.wrapping_add(u32::try_from(at).unwrap_or(0)), b);
        }
    }

    /// `SndSyncMusicStandby()` (0x00887900): 0.
    #[must_use]
    pub fn snd_sync_music_standby() -> u32 {
        0
    }

    /// `SndSyncTrans(block)` (0x00887910): 0 (the PC build's sync status is
    /// always 0).
    #[must_use]
    pub fn snd_sync_trans(_block: u32) -> u32 {
        0
    }

    /// `SndSyncWaveTrans(block)` (0x00887930).
    #[must_use]
    pub fn snd_sync_wave_trans(block: u32) -> u32 {
        snd_sync_trans(block)
    }

    /// `SndToUpper(s)` (0x00887940): ASCII lower case to upper, in place.
    pub fn snd_to_upper(h: &mut impl Mem, s: u32) {
        let mut p = s;
        let mut c = h.read::<i8>(p);
        while c != 0 {
            if (i32::from(c) - 0x61).cast_unsigned() <= 0x19 {
                h.write(p, c.cast_unsigned() & 0xdf);
            }
            p = p.wrapping_add(1);
            c = h.read::<i8>(p);
        }
    }

    /// `SndVagStreamVolumeDirect(volume)` (0x00887990): 0 when voices are off.
    pub fn snd_vag_stream_volume_direct(h: &mut impl Host, volume: u32) -> u32 {
        let v = voice_volume(h, volume);
        call(h, I_VOICE_VOLUME_DIRECT, &[v])
    }

    /// `SndVagStreamVolumeTranslate(frames, volume)` (0x008879b0).
    pub fn snd_vag_stream_volume_translate(h: &mut impl Host, frames: u32, volume: u32) -> u32 {
        let v = voice_volume(h, volume);
        call(h, I_VOICE_VOLUME_TRANSLATE, &[frames, v])
    }
}
