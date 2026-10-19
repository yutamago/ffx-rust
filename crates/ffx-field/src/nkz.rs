//! `Nkz`: the overdrive minigames' tables (Tidus's, Auron's and Lulu's
//! time limits, Auron's button sequences), and `Sqs`: the sound driver's
//! status words. Both are in the `map` part of the image.

use ffx_bridge::Global;

/// Auron's time limits, `f32` per level (`NkzGetAuronLimitTime`, `.data`).
pub const AURON_LIMIT_TIME: Global<u32> = Global::new(0x00c8_6b60);
/// Auron's button sequences, `0x20` bytes each (`NkzGetAuronLimitButtonData`).
pub const AURON_LIMIT_BUTTONS: Global<u8> = Global::new(0x00c8_6b70);
/// Bytes per button sequence.
pub const AURON_BUTTONS_SIZE: u32 = 0x20;
/// Tidus's time limits, `f32` per level (`NkzGetTidusLimitTime`).
pub const TIDUS_LIMIT_TIME: Global<u32> = Global::new(0x00c8_6bf0);
/// Lulu's time limit, an `f32` constant in `.rdata` (`NkzGetLuluLimitTime`).
pub const LULU_LIMIT_TIME: Global<u32> = Global::new(0x00b9_22f8);
/// The build date string `NkzVersion` returns (`.rdata`).
pub const VERSION: Global<u8> = Global::new(0x00b7_b7dc);
/// The sound driver's status words: 0 the system status, 1 the error
/// status (`SqsReportIopSoundStatus`).
pub const SQS_IOP_STATUS: Global<u32> = Global::new(0x012f_4e80);

/// Set when a shout sound was skipped (`SqsShoutStreamingPlayState`).
pub const SHOUT_SKIPPED: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x012f_4e94);
/// The SIF DMA records (two of 16 bytes: source, destination, size,
/// attribute).
pub const SIF_DMA: u32 = 0x012f_4e00;
/// The id of the last SIF DMA.
pub const SIF_DMA_ID: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x012f_4ed0);
/// The shout sound buffer's IOP address.
pub const SHOUT_BUFFER: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x012f_4ecc);
/// The battle streaming sound's id (`SqsBattleStreamingDataSend`; -1
/// none).
pub const BATTLE_STREAM: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x00c4_9498);

/// The digit table `NkzSetLuluLimitTbl` gives Lulu for a battle group
/// (0..=18, `0x00a5cec8` maps it to six rule sets) and her magic: the
/// first threshold the magic is below picks the table (each `0x14` bytes
/// in `.rdata` from `0x00b7b7f4`).
#[must_use]
pub fn lulu_limit_table(group: u32, mag: i32) -> Option<u32> {
    const A: u32 = 0x00b7_b7f4;
    let t = |k: u32| A + 0x14 * k;
    let rules: &[(i32, u32)] = match group {
        0..=3 => &[
            (0x23, 0),
            (0x3c, 1),
            (100, 2),
            (200, 3),
            (0xff, 4),
            (i32::MAX, 5),
        ],
        4..=7 => &[
            (0x19, 6),
            (0x2d, 0),
            (0x5a, 1),
            (200, 2),
            (0xff, 3),
            (i32::MAX, 5),
        ],
        8..=14 => &[(0x28, 6), (0x4b, 0), (100, 1), (200, 2), (i32::MAX, 3)],
        15 | 16 => &[
            (0x1e, 7),
            (0x32, 0),
            (100, 1),
            (200, 2),
            (0xff, 3),
            (i32::MAX, 5),
        ],
        17 => &[(0x32, 8), (0x4b, 9), (100, 10), (0xff, 11), (i32::MAX, 12)],
        18 => &[
            (0x28, 13),
            (0x4b, 14),
            (0x96, 8),
            (0xff, 15),
            (i32::MAX, 16),
        ],
        _ => return None,
    };
    rules
        .iter()
        .find(|&&(below, _)| mag < below || below == i32::MAX)
        .map(|&(_, k)| t(k))
}

/// The tables where the game keeps them, through [`ffx_bridge::Mem`].
/// `"NKZ:Illigal PC Number %d\n"` (`.rdata`), the aeon gains' error.
pub const ILLEGAL_PC: u32 = 0x00b7_b948;
/// `dbgPrintf` (`0x0062f500`).
pub const DBG_PRINTF: u32 = 0x0062_f500;
/// 0.25 (`f64`, `.rdata`): the aeons' HP gain per root.
pub const HP_SCALE: u32 = 0x00b3_4f90;
/// 16.0 (`f64`): the aeons' base HP gain.
pub const HP_BASE: u32 = 0x00b3_13a0;
/// 3.0 (`f64`): the aeons' MP gain divisor.
pub const MP_DIV: u32 = 0x00b2_c5c8;
/// 10.0 (`f64`): the aeons' base MP gain.
pub const MP_BASE: u32 = 0x00b9_22b0;
/// The aeon stat table `TkMenuCtrlSummon` reads (`0x1c` bytes a stat:
/// HP, MP, strength .. accuracy from row 1; the gain function first, the
/// maximum last).
pub const AEON_STATS: u32 = 0x00c5_a604;

// ---- field round 33: the Sqs IOP calls and SE wrappers ----

/// `FFXSifCheckStatRpc(client)` (`0x00a44760`; returns 0 on PC).
pub const SIF_CHECK_STAT_RPC: u32 = 0x00a4_4760;
/// `FFXSifCallRpc(client, cmd, mode, send, ssize, recv, rsize, end, arg)`
/// (`0x00a44750`; returns 0 on PC).
pub const SIF_CALL_RPC: u32 = 0x00a4_4750;
/// `sceSifDmaStat(id)` (`0x00a44770`; returns -1 on PC).
pub const SIF_DMA_STAT: u32 = 0x00a4_4770;
/// The sound driver's RPC client (`Sqs_CallRpc`, `SqsIopCommandSend`).
pub const SQS_RPC_CLIENT: u32 = 0x012f_4e20;
/// The second RPC client (`Sqs_CallRpc2`).
pub const SQS_RPC_CLIENT2: u32 = 0x012f_4e48;
/// Pointer to the RPC send buffer (0x40 bytes, `Sqs_CallRpc`; the SE
/// mask `SqsSeSeparateMaskClear` writes).
pub const SQS_RPC_SEND: Global<u32> = Global::new(0x00c4_9458);
/// Pointer to the second client's send buffer.
pub const SQS_RPC_SEND2: Global<u32> = Global::new(0x00c4_945c);
/// Pointer to the RPC receive buffer (0x400 bytes).
pub const SQS_RPC_RECV: Global<u32> = Global::new(0x00c4_9460);
/// Pointer to the second client's receive buffer.
pub const SQS_RPC_RECV2: Global<u32> = Global::new(0x00c4_9464);
/// The IOP command queue's start (`SqsIopCommandSend`).
pub const SQS_CMD_TOP: Global<u32> = Global::new(0x00c4_9468);
/// The IOP command queue's next free slot (32-byte commands).
pub const SQS_CMD_NEXT: Global<u32> = Global::new(0x00c4_946c);
/// The command buffer `SqsIopCommandSend` sends.
pub const SQS_CMD_BUF: u32 = 0x012f_4a00;
/// The 0x40-byte reply `SqsIopCommandSend` receives.
pub const SQS_REPLY: u32 = 0x012f_4f00;
/// Four histories of four words: each send shifts them down and puts
/// reply words `+0x10`, `+0x18`, `+0x30` and `+0x34` last.
pub const SQS_REPLY_HISTORY: u32 = 0x012f_4f48;
/// Standby requests sent (`SqsReportStandby`); `SqsReportStandbyStatus`
/// reports this less [`SQS_IOP_RESULT`], the requests answered.
pub const SQS_STANDBY_SENT: Global<u32> = Global::new(0x012f_414c);
/// The end function `SqsReportStandby` gives `Sqs_CallRpc2` (not a
/// function in the dump; it stores the answered count to [`SQS_IOP_RESULT`]).
pub const SQS_STANDBY_DONE: u32 = 0x0081_fcb0;
/// Flags; bit 0 cleared by the standby request's end function.
pub const SQS_RPC_FLAGS: Global<u32> = Global::new(0x012f_4e74);
/// Set while a reply history update is pending (`SqsReportStandby` reads
/// it, without effect).
pub const SQS_REPLY_PENDING: Global<u32> = Global::new(0x012f_4f44);
/// Bit 0: report a full command queue (`dbgPrintf`) before sending it.
pub const SQS_DEBUG_FLAGS: Global<u32> = Global::new(0x012f_4148);
/// The end of the command queue's space: a command that would pass it
/// sends the queue first.
pub const SQS_CMD_LIMIT: u32 = 0x012f_4e00;
/// The full-queue message (`.rdata`).
pub const SQS_CMD_FULL_MESSAGE: u32 = 0x00b5_8d28;
/// The play mode `SqsSetPlayMode` keeps (0..2).
pub const SQS_PLAY_MODE: Global<u32> = Global::new(0x012f_4144);
/// Non-zero: `SqsInitWaveInfo` does nothing.
pub const SQS_WAVE_INFO_DONE: Global<u32> = Global::new(0x012f_4140);
/// The IOP address `SqsInitWaveInfo` sends the wave info to.
pub const SQS_WAVE_INFO_DST: Global<u32> = Global::new(0x012f_4f88);
/// The straight stream's id (`SqsStraightStreamingStart` sends it).
pub const SQS_STRAIGHT_STREAM: Global<u32> = Global::new(0x012f_4e70);
/// The wave ids `SqsSeSeparateDataSend` loads (256 `u32`).
pub const SQS_SEP_WAVES: u32 = 0x012f_b3c0;
/// `SqsSeSeparateDataSend`'s messages (`.rdata`): the size error's format
/// and prefix, the red and reset escapes and the overflow text.
pub const SQS_SEP_SIZE_FORMAT: u32 = 0x00b5_8e08;
/// See [`SQS_SEP_SIZE_FORMAT`].
pub const SQS_SEP_SIZE_PREFIX: u32 = 0x00b5_8df4;
/// See [`SQS_SEP_SIZE_FORMAT`].
pub const SQS_SEP_RED: u32 = 0x00b1_f84c;
/// See [`SQS_SEP_SIZE_FORMAT`].
pub const SQS_SEP_RESET: u32 = 0x00b1_f854;
/// See [`SQS_SEP_SIZE_FORMAT`].
pub const SQS_SEP_TOO_MANY: u32 = 0x00b5_8dbc;
/// MSVCR110's `fflush` import slot.
pub const CRT_FFLUSH: u32 = 0x00b0_c3bc;
/// What `SqsIopCommandSend` returns.
pub const SQS_IOP_RESULT: Global<u32> = Global::new(0x012f_4150);
/// The last SE handle `SqsSePlay`/`SqsSeForcePlay` gave (never 0).
pub const SQS_SE_HANDLE: Global<u32> = Global::new(0x012f_4e7c);
/// The newest word of the second reply history; bit 0 set by an SE start.
pub const SQS_SE_STARTED: Global<u32> = Global::new(0x012f_4f64);
/// ADPCM filter coefficients, two `i32` per filter.
pub const SQS_ADPCM_COEFS: u32 = 0x00c4_9470;
/// ADPCM decoder history: left `s1`, `s2`, right `s1`, `s2` (`i32`).
pub const SQS_ADPCM_STATE: u32 = 0x012f_4f90;
/// `"FMOD SFX ERROR! There are battle streaming sound is started!!!\n"`.
pub const NO_MAGIC_MESSAGE: u32 = 0x00b5_8fa8;
/// `"FMOD SFX ERROR! Try to start battle streaming sound without set
/// sequence id!!!\n"`.
pub const NO_STREAM_MESSAGE: u32 = 0x00b5_8fe8;
/// The FMOD glue the SE wrappers call.
pub const I_SFX_FORCE_START: u32 = 0x0067_a310;
/// See [`I_SFX_FORCE_START`].
pub const I_SFX_IS_PLAYING: u32 = 0x0067_a340;
/// See [`I_SFX_FORCE_START`].
pub const I_SFX_LOAD_DATA: u32 = 0x0067_a360;
/// See [`I_SFX_FORCE_START`].
pub const I_SFX_PLAY_BATTLE_STREAMING: u32 = 0x0067_a3a0;
/// See [`I_SFX_FORCE_START`].
pub const I_SFX_SET_PAN: u32 = 0x0067_a3c0;
/// See [`I_SFX_FORCE_START`].
pub const I_SFX_SET_PAN_TRANSLATE: u32 = 0x0067_a3e0;
/// See [`I_SFX_FORCE_START`].
pub const I_SFX_SET_VOLUME: u32 = 0x0067_a400;
/// See [`I_SFX_FORCE_START`].
pub const I_SFX_SET_VOLUME_TRANSLATE: u32 = 0x0067_a420;
/// See [`I_SFX_FORCE_START`].
pub const I_SFX_START: u32 = 0x0067_a440;
/// See [`I_SFX_FORCE_START`].
pub const I_SFX_STOP: u32 = 0x0067_a460;
/// See [`I_SFX_FORCE_START`].
pub const I_SFX_STOP_ALL_NON_BLOCKING_SOUND: u32 = 0x0067_a480;
/// See [`I_SFX_FORCE_START`].
pub const I_SFX_STOP_NON_BLOCKING_SOUND: u32 = 0x0067_a4a0;
/// See [`I_SFX_FORCE_START`].
pub const I_SFX_UNLOAD_ALL_DATA: u32 = 0x0067_a4d0;

/// One PS-ADPCM block of 16 samples per header byte: the header's high
/// nibble picks the filter, the low one the shift; each data byte gives
/// two samples, low nibble first. `s` is (`s1`, `s2`); `coefs(filter)` the
/// filter's two `i32`. Arithmetic wraps as the original's `imul`.
pub fn adpcm_block(
    header: u8,
    data: &[u8; 8],
    s: &mut (i32, i32),
    coefs: impl Fn(u32) -> (i32, i32),
    out: &mut [i16; 16],
) {
    let (c0, c1) = coefs(u32::from(header >> 4));
    let shift = u32::from(header & 0xf);
    for (i, &b) in data.iter().enumerate() {
        for (j, d) in [u16::from(b) << 12, (u16::from(b) & 0xf0) << 8]
            .into_iter()
            .enumerate()
        {
            let d = i32::from(d.cast_signed()) >> shift;
            let v = (c0.wrapping_mul(s.0).wrapping_add(c1.wrapping_mul(s.1)) >> 6).wrapping_add(d);
            *s = (v, s.0);
            #[allow(clippy::cast_possible_truncation)] // the low half is stored
            {
                out[2 * i + j] = v as i16;
            }
        }
    }
}

fn rdata(mem: &impl ffx_bridge::Mem, at: u32) -> f64 {
    mem.read(mem.rebase(at))
}

pub mod bridge {
    use ffx_bridge::{Mem, x87};

    use super::{
        AURON_BUTTONS_SIZE, AURON_LIMIT_BUTTONS, AURON_LIMIT_TIME, LULU_LIMIT_TIME, SQS_IOP_STATUS,
        TIDUS_LIMIT_TIME, VERSION,
    };

    fn entry(mem: &impl Mem, table: ffx_bridge::Global<u32>, i: i32) -> f32 {
        #[allow(clippy::cast_sign_loss)] // the original indexes unchecked
        let at = table.ptr(mem).at(i as u32);
        f32::from_bits(at.get(mem))
    }

    /// `NkzVersion()` (`0x00a5cf00`): the build date string.
    pub fn nkz_version(mem: &impl Mem) -> u32 {
        VERSION.ptr(mem).addr
    }

    /// `NkzGetLuluLimitTime()` (`0x00a5cb10`).
    pub fn nkz_get_lulu_limit_time(mem: &impl Mem) -> f32 {
        f32::from_bits(LULU_LIMIT_TIME.get(mem))
    }

    /// `NkzGetAuronLimitTime(level)` (`0x00a5ca70`).
    pub fn nkz_get_auron_limit_time(mem: &impl Mem, level: i32) -> f32 {
        entry(mem, AURON_LIMIT_TIME, level)
    }

    /// `NkzGetTidusLimitTime(level)` (`0x00a5cbd0`).
    pub fn nkz_get_tidus_limit_time(mem: &impl Mem, level: i32) -> f32 {
        entry(mem, TIDUS_LIMIT_TIME, level)
    }

    /// `NkzGetAuronLimitButtonData(n)` (`0x00a5ca60`): sequence `n`.
    pub fn nkz_get_auron_limit_button_data(mem: &impl Mem, n: u32) -> u32 {
        AURON_LIMIT_BUTTONS
            .ptr(mem)
            .addr
            .wrapping_add(n.wrapping_mul(AURON_BUTTONS_SIZE))
    }

    /// `SqsReportIopSoundStatus(i)` (`0x0081e250`): status word `i`.
    pub fn sqs_report_iop_sound_status(mem: &impl Mem, i: u32) -> u32 {
        SQS_IOP_STATUS.ptr(mem).at(i).get(mem)
    }

    /// `SqsIopSoundSystemStatus()` (`0x0081d710`).
    pub fn sqs_iop_sound_system_status(mem: &impl Mem) -> u32 {
        sqs_report_iop_sound_status(mem, 0)
    }

    /// `SqsIopSoundErrorStatus()` (`0x0081d700`).
    pub fn sqs_iop_sound_error_status(mem: &impl Mem) -> u32 {
        sqs_report_iop_sound_status(mem, 1)
    }

    /// `SqsBattleStreamingDataSend(a, b)` (`0x0081cef0`): the battle
    /// streaming sound's id (`0x00c49498`) from `a`'s `+8`, else `b`'s,
    /// else -1. Without a magic file (`MagicFile_GetMagicId` -1) it only
    /// prints an error to stderr (`error`).
    pub fn sqs_battle_streaming_data_send<M: Mem>(
        mem: &mut M,
        a: u32,
        b: u32,
        error: impl FnOnce(&mut M),
    ) {
        if ffx_system::instance::bridge::magic_id(mem) == u32::MAX {
            error(mem);
            return;
        }
        let src = if a != 0 { a } else { b };
        let v = if src == 0 {
            u32::MAX
        } else {
            mem.read::<u32>(src.wrapping_add(8))
        };
        super::BATTLE_STREAM.set(mem, v);
    }

    /// `NkzSetLuluLimitTbl(group)` (`0x00a5cc70`): Lulu's overdrive digit
    /// table for her magic (`MsGetRamChrMag(5)`) and the battle group
    /// (`TOBtlSetLuluLimitTbl` with the chosen table, [`super::lulu_limit_table`]);
    /// groups past 18 do nothing.
    pub fn nkz_set_lulu_limit_tbl(mem: &mut impl Mem, group: u32) {
        #[allow(clippy::cast_possible_wrap)] // compared signed
        let mag = ffx_battle::actor::bridge::ms_get_ram_chr_mag(mem, 5) as i32;
        if let Some(t) = super::lulu_limit_table(group, mag) {
            let at = mem.rebase(t);
            ffx_system::btl::bridge::set_lulu_limit_table(mem, at);
        }
    }

    /// `NkzGetMonsterListNum()` (`0x00a5cb20`): 139 monster list entries.
    #[must_use]
    pub const fn nkz_get_monster_list_num() -> u32 {
        0x8b
    }

    /// `SqsShoutStreamingPlayState()` (`0x0081edf0`): clears the shout
    /// skip flag (`0x012f4e94`; the PC build prints "Virtuos: Shout sound
    /// skipped" through the empty `dbgPrintf`); returns 0.
    pub fn sqs_shout_streaming_play_state(mem: &mut impl Mem) -> u32 {
        if super::SHOUT_SKIPPED.get(mem) != 0 {
            super::SHOUT_SKIPPED.set(mem, 0);
        }
        0
    }

    /// `SqsSifDmaStart(_, src, dst, size)` (`0x0081eeb0`): the SIF DMA
    /// record's first transfer (`0x012f4e00`: source, destination, size,
    /// attribute 0) and its id (`0x012f4ed0`) 0; returns 0.
    pub fn sqs_sif_dma_start(mem: &mut impl Mem, src: u32, dst: u32, size: u32) -> u32 {
        let r = mem.rebase(super::SIF_DMA);
        mem.write(r, [src, dst, size, 0]);
        super::SIF_DMA_ID.set(mem, 0);
        0
    }

    /// `SqsShoutStreamingDataSend(src, size)` (`0x0081ed40`): for data, a
    /// SIF DMA to the shout buffer (`0x012f4ecc`): one transfer up to
    /// `0xa0000` bytes, else `0x80000` and the rest; the id `sceSifSetDma`
    /// gives (the PC stub's) into `0x012f4ed0`.
    pub fn sqs_shout_streaming_data_send(mem: &mut impl Mem, src: u32, size: i32) {
        if src == 0 {
            return;
        }
        let r = mem.rebase(super::SIF_DMA);
        mem.write(r, src);
        mem.write(r + 0xc, 0u32);
        let buf = super::SHOUT_BUFFER.get(mem);
        if size > 0xa_0000 {
            mem.write(r + 0x10, src.wrapping_add(0x8_0000));
            mem.write(r + 0x14, buf.wrapping_add(0x8_0000));
            mem.write(r + 4, buf);
            mem.write(r + 8, 0x8_0000u32);
            mem.write(r + 0x18, size.wrapping_sub(0x8_0000).cast_unsigned());
            mem.write(r + 0x1c, 0u32);
        } else {
            mem.write(r + 4, buf);
            mem.write(r + 8, size.cast_unsigned());
        }
        super::SIF_DMA_ID.set(mem, ffx_system::stub::SIF_SET_DMA);
    }

    /// `SqsCmdSetC2(ctx, cmd)` (`0x0081c140`, an IOP command
    /// handler in the table at `0x00c48f30`): the command's second `u16`
    /// to `+0xc2` of the context's `+0x220` record. Returns the next
    /// command (`cmd + 4`).
    pub fn sqs_cmd_set_c2(mem: &mut impl Mem, ctx: u32, cmd: u32) -> u32 {
        let v = mem.read::<u32>(ctx.wrapping_add(0x220));
        let x = mem.read::<u16>(cmd.wrapping_add(2));
        mem.write(v.wrapping_add(0xc2), x);
        cmd.wrapping_add(4)
    }

    /// `NkzTidusLimitTableA(i)` (`0x00a5cb30`) and `NkzTidusLimitTableB`
    /// (`0x00a5cb80`): entry `i` of a four-entry local table (24, 20, 20,
    /// 18 and 11, 12, 13, 14); indices past it read the original's stack,
    /// this gives 0.
    #[must_use]
    pub fn nkz_tidus_limit_table(table: u32, i: u32) -> u32 {
        let t: [u32; 4] = if table == 0 {
            [0x18, 0x14, 0x14, 0x12]
        } else {
            [0xb, 0xc, 0xd, 0xe]
        };
        t.get(i as usize).copied().unwrap_or(0)
    }

    // ---- field round 24: the aeons' stat gains ----

    /// The eight half gains (`0x00a5c660` strength, `0x00a5c740` defence,
    /// `0x00a5c910` magic, `0x00a5c980` magic defence, `0x00a5c9f0`
    /// agility, `0x00a5c860` luck, `0x00a5c6d0` evasion, `0x00a5c7f0`
    /// accuracy; identical bodies): for an aeon (PC 8..=17) `v / 2`
    /// (toward zero), at least 1; another PC prints the original's error
    /// (`dbgPrintf`) and gives 100.
    pub fn nkz_aeon_gain_half<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        pc: u32,
        v: i32,
        call: &mut C,
    ) -> u32 {
        if pc.wrapping_sub(8) > 9 {
            let fmt = mem.rebase(super::ILLEGAL_PC);
            let f = mem.rebase(super::DBG_PRINTF);
            call(mem, f, &[fmt, pc]);
            return 100;
        }
        let h = v / 2;
        if h == 0 { 1 } else { h.cast_unsigned() }
    }

    /// `0x00a5c7b0` (HP, name `NkzAeonGainHp` ours): `sqrt(v) * 0.25 +
    /// 16.0` truncated (`_ftol2_sse`), at least 1. The PC is not checked.
    pub fn nkz_aeon_gain_hp(mem: &impl Mem, v: i32) -> u32 {
        let x = x87::add(
            x87::mul(x87::sqrt(f64::from(v)), super::rdata(mem, super::HP_SCALE)),
            super::rdata(mem, super::HP_BASE),
        );
        let r = x87::ftol(mem, x);
        if r == 0 { 1 } else { r }
    }

    /// `0x00a5c8d0` (MP): `(sqrt(v) + sqrt(v)) / 3.0 + 10.0` truncated, at
    /// least 1.
    pub fn nkz_aeon_gain_mp(mem: &impl Mem, v: i32) -> u32 {
        let r = x87::sqrt(f64::from(v));
        let x = x87::add(
            x87::div(x87::add(r, r), super::rdata(mem, super::MP_DIV)),
            super::rdata(mem, super::MP_BASE),
        );
        let r = x87::ftol(mem, x);
        if r == 0 { 1 } else { r }
    }

    // ---- field round 33: the Sqs IOP calls and SE wrappers ----

    /// Waits until `client` is idle (`FFXSifCheckStatRpc` 0).
    fn rpc_idle<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        client: u32,
        call: &mut C,
    ) {
        let f = mem.rebase(super::SIF_CHECK_STAT_RPC);
        while call(mem, f, &[client]) != 0 {}
    }

    /// `FFXSifCallRpc` until it accepts (not negative); the arguments are
    /// built again for each try, as the original reloads its globals.
    fn rpc_call<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
        args: impl Fn(&M) -> [u32; 9],
    ) -> u32 {
        let f = mem.rebase(super::SIF_CALL_RPC);
        loop {
            let a = args(mem);
            let r = call(mem, f, &a);
            if r.cast_signed() >= 0 {
                return r;
            }
        }
    }

    /// `Sqs_CallRpc(cmd)` (`0x0081cbd0`): `cmd` to the sound client
    /// (`0x012f4e20`), sending 0x40 bytes and receiving 0x400 through the
    /// buffers at `0x00c49458`/`0x00c49460`; returns `FFXSifCallRpc`'s.
    pub fn sqs_call_rpc<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        cmd: u32,
        call: &mut C,
    ) -> u32 {
        let client = mem.rebase(super::SQS_RPC_CLIENT);
        rpc_idle(mem, client, call);
        rpc_call(mem, call, |m| {
            let (send, recv) = (super::SQS_RPC_SEND.get(m), super::SQS_RPC_RECV.get(m));
            [client, cmd, 0, send, 0x40, recv, 0x400, 0, 0]
        })
    }

    /// `Sqs_CallRpc2(cmd, _, end, arg)` (`0x0081cc20`, name ours): the same
    /// on the second client (`0x012f4e48`), 0x40 bytes each way
    /// (`0x00c4945c`/`0x00c49464`), passing `end` and `arg` on.
    pub fn sqs_call_rpc2<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        cmd: u32,
        end: u32,
        arg: u32,
        call: &mut C,
    ) -> u32 {
        let client = mem.rebase(super::SQS_RPC_CLIENT2);
        rpc_idle(mem, client, call);
        rpc_call(mem, call, |m| {
            let (send, recv) = (super::SQS_RPC_SEND2.get(m), super::SQS_RPC_RECV2.get(m));
            [client, cmd, 0, send, 0x40, recv, 0x40, end, arg]
        })
    }

    /// `SqsIopCommandSend()` (`0x0081d5f0`): sends the queued commands
    /// (`next - top` rounded down to 32 bytes, from `0x012f4a00`) on the
    /// sound client with a 0x40-byte reply at `0x012f4f00`, empties the
    /// queue (`next = top`), shifts the four reply histories
    /// (`0x012f4f48`, 4 x 4 words) and puts reply words `+0x10 +0x18
    /// +0x30 +0x34` last. Returns the word at `0x012f4150`.
    pub fn sqs_iop_command_send<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
    ) -> u32 {
        let client = mem.rebase(super::SQS_RPC_CLIENT);
        rpc_idle(mem, client, call);
        let size = super::SQS_CMD_NEXT
            .get(mem)
            .wrapping_sub(super::SQS_CMD_TOP.get(mem))
            & !0x1f;
        let (buf, reply) = (mem.rebase(super::SQS_CMD_BUF), mem.rebase(super::SQS_REPLY));
        rpc_call(mem, call, |_| [client, 0, 0, buf, size, reply, 0x40, 0, 0]);
        let top = super::SQS_CMD_TOP.get(mem);
        super::SQS_CMD_NEXT.set(mem, top);
        let h = mem.rebase(super::SQS_REPLY_HISTORY);
        for (row, word) in [0x10u32, 0x18, 0x30, 0x34].into_iter().enumerate() {
            let r = h + 0x10 * u32::try_from(row).unwrap_or(0);
            let [_, a, b, c] = mem.read::<[u32; 4]>(r);
            let last = mem.read::<u32>(reply + word);
            mem.write(r, [a, b, c, last]);
        }
        super::SQS_IOP_RESULT.get(mem)
    }

    /// `SqsSifDmaState(_, wait)` (`0x0081eef0`): `sceSifDmaStat` of the
    /// last DMA (`0x012f4ed0`, read each time), again while `wait` is set
    /// and it is not negative; returns the last. The `FlushCache(0)` it
    /// starts with is a bare `ret` on PC and its delay loop touches only
    /// registers: both left out.
    pub fn sqs_sif_dma_state<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        wait: u32,
        call: &mut C,
    ) -> u32 {
        let f = mem.rebase(super::SIF_DMA_STAT);
        loop {
            let id = super::SIF_DMA_ID.get(mem);
            let r = call(mem, f, &[id]);
            if wait == 0 || r.cast_signed() < 0 {
                return r;
            }
        }
    }

    /// Calls the FMOD glue function `f` (an image address), returning its
    /// `eax`.
    fn glue<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        f: u32,
        args: &[u32],
        call: &mut C,
    ) -> u32 {
        let f = mem.rebase(f);
        call(mem, f, args)
    }

    const ID: u32 = 0x7fff_ffff;

    /// `SqsSePlay(_, id, pan, volume)` (`0x0081e4e0`, `iSfxStart`) and
    /// `SqsSeForcePlay` (`0x0081e410`, `iSfxForceStart`): the next SE
    /// handle (`0x012f4e7c`, 0 skipped), the start with `id` (bit 31
    /// cleared), `volume & 0x7f`, `pan & 0x7f`, bit 0 of `0x012f4f64` set;
    /// returns the handle (read after the call).
    pub fn sqs_se_play<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        start: u32,
        [id, pan, volume]: [u32; 3],
        call: &mut C,
    ) -> u32 {
        let h = super::SQS_SE_HANDLE.get(mem).wrapping_add(1);
        super::SQS_SE_HANDLE.set(mem, if h == 0 { 1 } else { h });
        glue(mem, start, &[id & ID, volume & 0x7f, pan & 0x7f], call);
        let h = super::SQS_SE_HANDLE.get(mem);
        let f = super::SQS_SE_STARTED.get(mem);
        super::SQS_SE_STARTED.set(mem, f | 1);
        h
    }

    /// `SqsSeStop(_, id)` (`0x0081e7c0`): `iSfxStop(id)`.
    pub fn sqs_se_stop<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        id: u32,
        call: &mut C,
    ) -> u32 {
        glue(mem, super::I_SFX_STOP, &[id & ID], call)
    }

    /// `SqsSePanDirect(_, id, pan)` (`0x0081e460`): `iSfxSetPan(id, pan &
    /// 0x7f)`; `SqsSeVolumeDirect` (`0x0081e820`) the same with
    /// `iSfxSetVolume`.
    pub fn sqs_se_direct<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        f: u32,
        id: u32,
        v: u32,
        call: &mut C,
    ) -> u32 {
        glue(mem, f, &[id & ID, v & 0x7f], call)
    }

    /// `SqsSePanTranslate(_, id, frames, pan)` (`0x0081e480`):
    /// `iSfxSetPanTranslate(id, pan & 0x7f, frames)`;
    /// `SqsSeVolumeTranslate` (`0x0081e860`) the same with
    /// `iSfxSetVolumeTranslate`.
    pub fn sqs_se_translate<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        f: u32,
        [id, frames, v]: [u32; 3],
        call: &mut C,
    ) -> u32 {
        glue(mem, f, &[id & ID, v & 0x7f, frames], call)
    }

    /// `SqsSeVolumeFadeOut(_, id, frames)` (`0x0081e840`):
    /// `iSfxSetVolumeTranslate(id, 0, frames)`.
    pub fn sqs_se_volume_fade_out<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        id: u32,
        frames: u32,
        call: &mut C,
    ) -> u32 {
        glue(
            mem,
            super::I_SFX_SET_VOLUME_TRANSLATE,
            &[id & ID, 0, frames],
            call,
        )
    }

    /// `SqsSeBlockPlaying(id)` (`0x0081e3e0`) and `SqsSeSeparatePlaying`
    /// (`0x0081e7a0`): `iSfxIsPlaying(id)` (a tail call).
    pub fn sqs_se_playing<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        id: u32,
        call: &mut C,
    ) -> u32 {
        glue(mem, super::I_SFX_IS_PLAYING, &[id & ID], call)
    }

    /// `SqsSeSeparateClear(id)` (`0x0081e5d0`): for -1, the send buffer's
    /// first word -1 and `iSfxStopAllNonBlockingSound(buffer)`; else
    /// `iSfxStopNonBlockingSound(id)` (id as given).
    pub fn sqs_se_separate_clear<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        id: u32,
        call: &mut C,
    ) -> u32 {
        if id == u32::MAX {
            let buf = super::SQS_RPC_SEND.get(mem);
            mem.write(buf, u32::MAX);
            glue(mem, super::I_SFX_STOP_ALL_NON_BLOCKING_SOUND, &[buf], call)
        } else {
            glue(mem, super::I_SFX_STOP_NON_BLOCKING_SOUND, &[id], call)
        }
    }

    /// `SqsSeSeparateMaskClear(m0..m7)` (`0x0081e750`): the eight mask
    /// words into the send buffer, -1 after them,
    /// `iSfxStopAllNonBlockingSound(buffer)`, then `iSfxUnloadAllData()`
    /// (a tail call, its result returned).
    pub fn sqs_se_separate_mask_clear<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        mask: [u32; 8],
        call: &mut C,
    ) -> u32 {
        let buf = super::SQS_RPC_SEND.get(mem);
        mem.write(buf, mask);
        mem.write(buf.wrapping_add(0x20), u32::MAX);
        glue(mem, super::I_SFX_STOP_ALL_NON_BLOCKING_SOUND, &[buf], call);
        glue(mem, super::I_SFX_UNLOAD_ALL_DATA, &[], call)
    }

    /// `SqsBattleWaveDataSend(wave)` (`0x0081cf90`): a wave file (`u16`
    /// `"WD"`, 0x4457) goes to `iSfxLoadData(i16 +2, 0)`, whose result it
    /// returns; anything else returns `wave`.
    pub fn sqs_battle_wave_data_send<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        wave: u32,
        call: &mut C,
    ) -> u32 {
        if mem.read::<u16>(wave) != 0x4457 {
            return wave;
        }
        let id = i32::from(mem.read::<i16>(wave.wrapping_add(2))).cast_unsigned();
        glue(mem, super::I_SFX_LOAD_DATA, &[id, 0], call)
    }

    /// `fprintf(stderr, message)` through the import slots; its result.
    fn print_error<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        message: u32,
        call: &mut C,
    ) -> u32 {
        let iob = mem.read::<u32>(mem.rebase(crate::yi::CRT_IOB_FUNC));
        let fprintf = mem.read::<u32>(mem.rebase(crate::yi::CRT_FPRINTF));
        let msg = mem.rebase(message);
        let stderr = call(mem, iob, &[]).wrapping_add(0x40);
        call(mem, fprintf, &[stderr, msg])
    }

    /// `SqsBattleStreamingStart(_, a, b)` (`0x0081cf40`): without a magic
    /// file (`g_MagicFile_MagicId` -1) or a battle stream id
    /// (`0x00c49498` -1) an error on stderr; else
    /// `iSfxPlayBattleStreaming(id, b, a)`. Returns the callee's `eax`.
    pub fn sqs_battle_streaming_start<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        a: u32,
        b: u32,
        call: &mut C,
    ) -> u32 {
        if ffx_system::instance::bridge::magic_id(mem) == u32::MAX {
            return print_error(mem, super::NO_MAGIC_MESSAGE, call);
        }
        let id = super::BATTLE_STREAM.get(mem);
        if id == u32::MAX {
            return print_error(mem, super::NO_STREAM_MESSAGE, call);
        }
        glue(mem, super::I_SFX_PLAY_BATTLE_STREAMING, &[id, b, a], call)
    }

    /// `SqsCmdSetC4C6(ctx, cmd)` (`0x0081c260`, name ours; an IOP command
    /// handler like [`sqs_cmd_set_c2`]): the command's second `u16` to
    /// `+0xc4` of the context's `+0x220` record when bit 12 of the `u16`
    /// at `ctx +0x21a` is set, else to `+0xc6`. Returns `cmd + 4`.
    pub fn sqs_cmd_set_c4_c6(mem: &mut impl Mem, ctx: u32, cmd: u32) -> u32 {
        let v = mem.read::<u32>(ctx.wrapping_add(0x220));
        let x = mem.read::<u16>(cmd.wrapping_add(2));
        let off = if mem.read::<u16>(ctx.wrapping_add(0x21a)) & 0x1000 != 0 {
            0xc4
        } else {
            0xc6
        };
        mem.write(v.wrapping_add(off), x);
        cmd.wrapping_add(4)
    }

    /// `op_malloc(heap, n)` (`0x007ff780`).
    pub const OP_MALLOC: u32 = 0x007f_f780;
    /// `opu_id_list_add(list, id)` (`0x00800d60`).
    pub const OPU_ID_LIST_ADD: u32 = 0x0080_0d60;
    /// `opus_no_get(list, unit, no)` (`0x00801060`).
    pub const OPUS_NO_GET: u32 = 0x0080_1060;
    /// `op_xyz_scale(dst, src, s)` (`0x007ea850`).
    pub const OP_XYZ_SCALE: u32 = 0x007e_a850;
    /// The CRT's `printf` (import slot `0x00b0c380`).
    pub const PRINTF_SLOT: u32 = 0x00b0_c380;
    /// `"anmm should never be NULL in original PS2 version.\n"`.
    pub const ANMM_NULL_MESSAGE: u32 = 0x00b5_8cf4;
    /// 16.0 (`f32`, `.rdata`): the `+0x90` vector's scale.
    pub const ANMM_SCALE: u32 = 0x00b9_231c;
    /// `g_Ee_Gpr` (`0x00c8f508`): the emulated R5900's zero register
    /// (16 bytes), then the rest.
    pub const EE_GPR: u32 = 0x00c8_f508;
    /// The emulated register the record's vectors are copied through
    /// (`0x00c8f528`, 16 bytes).
    pub const EE_COPY_REG: u32 = 0x00c8_f528;

    /// `SqsCmdAnmm(ctx, cmd)` (`0x0081bbc0`, name ours, after its
    /// message; an IOP command handler like [`sqs_cmd_set_c2`]) on the
    /// context's record (`+0x220`), by the mode bits `0x7000` of `ctx
    /// +0x21a`:
    /// - 0: the record gets a ring of `n` (`cmd +2`) entries of 0x50 bytes
    ///   (`op_malloc(ctx +0x3a8, 0x50 n + 0x70)`, at its `+0xbc` and
    ///   `+0xa8`, state `+0xbb` 9): `n`, 0 used, the record's `u16`s
    ///   `+0xc4`, `+0xc6` and word `+0xc8`, each entry's `+0x20` word 0;
    ///   the context's id (`+0x214`) joins its list (`opu_id_list_add(ctx
    ///   +0x300, ..)`). Returns `cmd + 4`.
    /// - `0x1000` (the record) or `0x2000` (`opus_no_get(ctx +0x300,
    ///   record, cmd +4)`): unless `ctx +0x212` is set, the target's ring
    ///   (state 9, else the CRT's `printf` of [`ANMM_NULL_MESSAGE`]) gets
    ///   its next entry (count `+2` modulo `n`): `+0x20` the command's
    ///   jump target (`cmd + cmd +2`), `+0x24` 0, `+0x29` 1, `+0x28`/`+0x2c`
    ///   and `+0x70..+0x78` from [`EE_GPR`], `+0x6c` the record's offset
    ///   from `ctx +0x320` / 256 (`u16`), `+0x40` and `+0x50` the record's
    ///   `+0x60` and `+0x80` vectors, `+0x30` its `+0x40` vector (with
    ///   bit 15 of `+0x21a`: `+0x90` times 16.0, `op_xyz_scale`), `+0x5c`
    ///   0 (the vectors pass through [`EE_COPY_REG`]). Returns `cmd + 4` or
    ///   `cmd + 6`.
    /// - Any other mode: returns `cmd`.
    pub fn sqs_cmd_anmm<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        ctx: u32,
        cmd: u32,
        call: &mut C,
    ) -> u32 {
        let rec = mem.read::<u32>(ctx.wrapping_add(0x220));
        let (target, step) = match mem.read::<u16>(ctx.wrapping_add(0x21a)) & 0x7000 {
            0 => return anmm_create(mem, ctx, cmd, rec, call),
            0x1000 => (rec, 4),
            0x2000 => {
                let f = mem.rebase(OPUS_NO_GET);
                let no = u32::from(mem.read::<u16>(cmd.wrapping_add(4)));
                (call(mem, f, &[ctx.wrapping_add(0x300), rec, no]), 6)
            }
            _ => return cmd,
        };
        let done = cmd.wrapping_add(step);
        if mem.read::<u16>(ctx.wrapping_add(0x212)) != 0 {
            return done;
        }
        if mem.read::<u8>(target.wrapping_add(0xbb)) != 9 {
            let f = mem.read::<u32>(mem.rebase(PRINTF_SLOT));
            let msg = mem.rebase(ANMM_NULL_MESSAGE);
            call(mem, f, &[msg]);
            return done;
        }
        let ring = mem.read::<u32>(target.wrapping_add(0xa8));
        let used = mem.read::<u16>(ring.wrapping_add(2));
        let n = mem.read::<u16>(ring);
        // idiv: the original faults on an empty ring
        let slot = if n == 0 { 0 } else { used % n };
        mem.write(ring.wrapping_add(2), used.wrapping_add(1));
        let e = ring.wrapping_add(u32::from(slot) * 0x50);
        let gpr = mem.rebase(EE_GPR);
        let zero = mem.read::<[u32; 4]>(gpr);
        mem.write(e.wrapping_add(0x20), zero);
        let jump = i32::from(mem.read::<i16>(cmd.wrapping_add(2))).cast_unsigned();
        mem.write(e.wrapping_add(0x20), jump.wrapping_add(cmd));
        mem.write(e.wrapping_add(0x24), 0u32);
        mem.write(e.wrapping_add(0x29), 1u8);
        let zero = mem.read::<[u32; 4]>(gpr);
        mem.write(e.wrapping_add(0x6c), zero);
        let base = i64::from(mem.read::<i32>(ctx.wrapping_add(0x320)));
        let offset = (i64::from(rec.cast_signed()) - base) / 0x100;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // the low word
        mem.write(e.wrapping_add(0x6c), offset as u16);
        copy_through_reg(mem, rec.wrapping_add(0x60), e.wrapping_add(0x40));
        copy_through_reg(mem, rec.wrapping_add(0x80), e.wrapping_add(0x50));
        if mem.read::<u16>(ctx.wrapping_add(0x21a)) & 0x8000 != 0 {
            let f = mem.rebase(OP_XYZ_SCALE);
            let s = x87::round_trip(mem.read::<f32>(mem.rebase(ANMM_SCALE))).to_bits();
            call(mem, f, &[e.wrapping_add(0x30), rec.wrapping_add(0x90), s]);
        } else {
            copy_through_reg(mem, rec.wrapping_add(0x40), e.wrapping_add(0x30));
        }
        mem.write(e.wrapping_add(0x5c), 0u32);
        done
    }

    /// The four words at `src` into [`EE_COPY_REG`], then from it to `dst`
    /// (the first from the value read, the rest read back).
    fn copy_through_reg(mem: &mut impl Mem, src: u32, dst: u32) {
        let reg = mem.rebase(EE_COPY_REG);
        let v = mem.read::<[u32; 4]>(src);
        mem.write(reg, v);
        mem.write(dst, v[0]);
        for i in 1..4 {
            let w = mem.read::<u32>(reg + 4 * i);
            mem.write(dst.wrapping_add(4 * i), w);
        }
    }

    /// Mode 0 of [`sqs_cmd_anmm`].
    fn anmm_create<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        ctx: u32,
        cmd: u32,
        rec: u32,
        call: &mut C,
    ) -> u32 {
        mem.write(rec.wrapping_add(0xbb), 9u8);
        let n = mem.read::<u16>(cmd.wrapping_add(2));
        let f = mem.rebase(OP_MALLOC);
        let heap = mem.read::<u32>(ctx.wrapping_add(0x3a8));
        let ring = call(mem, f, &[heap, u32::from(n) * 0x50 + 0x70]);
        mem.write(rec.wrapping_add(0xbc), ring);
        mem.write(ring, n);
        mem.write(ring.wrapping_add(2), 0u16);
        let a = mem.read::<u16>(rec.wrapping_add(0xc4));
        mem.write(ring.wrapping_add(4), a);
        let b = mem.read::<u16>(rec.wrapping_add(0xc6));
        mem.write(ring.wrapping_add(6), b);
        let c = mem.read::<u32>(rec.wrapping_add(0xc8));
        mem.write(ring.wrapping_add(8), c);
        mem.write(rec.wrapping_add(0xa8), ring);
        for i in 0..u32::from(n) {
            mem.write(ring.wrapping_add(0x20 + 0x50 * i), 0u32);
        }
        let f = mem.rebase(OPU_ID_LIST_ADD);
        let id = u32::from(mem.read::<u16>(ctx.wrapping_add(0x214)));
        call(mem, f, &[ctx.wrapping_add(0x300), id]);
        cmd.wrapping_add(4)
    }

    /// `opus_draw_off(list, id)` (`0x00800d10`).
    pub const OPUS_DRAW_OFF: u32 = 0x0080_0d10;

    /// `SqsCmdAnmmMode(ctx, cmd)` (`0x0081ca60`, name ours; the IOP command
    /// that sets [`sqs_cmd_anmm`]'s mode) on the context's record
    /// (`+0x220`), by the mode bits `0xf000` of `ctx +0x21a`:
    /// - 0: the mode byte `+0xbb` from `cmd +2`, two `u16`s (`cmd +4`,
    ///   `+6`) to `+0xa8`/`+0xaa`, the context's id joins its list
    ///   (`opu_id_list_add(ctx +0x300, +0x214)`); returns `cmd + 8`.
    /// - `0x1000`/`0x3000`: the bits `cmd +2` set in / cleared from the
    ///   record's `+0xc0`; returns `cmd + 4`.
    /// - `0x2000`: the mode byte, `+0xa8` the inline table at `cmd + 6`, the
    ///   id joins the list; returns past the table (`cmd + 6 + 2 n`, `n`
    ///   at `cmd +4`, the doubling in 16 bits).
    /// - `0x4000`: mode 0 and `opus_draw_off(ctx +0x300, id)`; returns
    ///   `cmd + 2`.
    /// - Anything else returns `cmd`.
    pub fn sqs_cmd_anmm_mode<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        ctx: u32,
        cmd: u32,
        call: &mut C,
    ) -> u32 {
        let rec = mem.read::<u32>(ctx.wrapping_add(0x220));
        let list = ctx.wrapping_add(0x300);
        let id = u32::from(mem.read::<u16>(ctx.wrapping_add(0x214)));
        let bits = |mem: &M| mem.read::<u16>(cmd.wrapping_add(2));
        let flags = rec.wrapping_add(0xc0);
        match mem.read::<u16>(ctx.wrapping_add(0x21a)) & 0xf000 {
            0 => {
                mem.write(rec.wrapping_add(0xbb), mem.read::<u8>(cmd.wrapping_add(2)));
                mem.write(rec.wrapping_add(0xa8), mem.read::<u16>(cmd.wrapping_add(4)));
                mem.write(rec.wrapping_add(0xaa), mem.read::<u16>(cmd.wrapping_add(6)));
                let f = mem.rebase(OPU_ID_LIST_ADD);
                call(mem, f, &[list, id]);
                cmd.wrapping_add(8)
            }
            0x1000 => {
                mem.write(flags, mem.read::<u16>(flags) | bits(mem));
                cmd.wrapping_add(4)
            }
            0x2000 => {
                mem.write(rec.wrapping_add(0xbb), mem.read::<u8>(cmd.wrapping_add(2)));
                mem.write(rec.wrapping_add(0xa8), cmd.wrapping_add(6));
                let f = mem.rebase(OPU_ID_LIST_ADD);
                call(mem, f, &[list, id]);
                let n = mem.read::<u16>(cmd.wrapping_add(4)).wrapping_mul(2);
                u32::from(n).wrapping_add(6).wrapping_add(cmd)
            }
            0x3000 => {
                mem.write(flags, mem.read::<u16>(flags) & !bits(mem));
                cmd.wrapping_add(4)
            }
            0x4000 => {
                mem.write(rec.wrapping_add(0xbb), 0u8);
                let f = mem.rebase(OPUS_DRAW_OFF);
                call(mem, f, &[list, id]);
                cmd.wrapping_add(2)
            }
            _ => cmd,
        }
    }

    /// `SqsCmdAnmmOef(ctx, cmd)` (`0x0081b8d0`, name ours; an IOP command
    /// like [`sqs_cmd_anmm`]): the record's (`ctx +0x220`) animation from
    /// its effect data (`op_oef2_a_get(ctx +0x360[record +0x1e], 4, cmd
    /// +2)`, kept at `+0xe0`, its table `+0x10` at `+0xe4` and that
    /// table's end at `+0xe8`): mode `+0xbb` 1, `+0xc0` `0x480080`, `+0xc4`
    /// 0, `+0xf2` 0x100, `+0xf0` 0x200, `+0xec` `0x0200ffff`, and from
    /// `ctx +0x21a` bits 13..15, 9..11 and the low byte less 0x24 at
    /// `+0xa8`..`+0xaa`; the context's id joins its list. Returns `cmd +
    /// 4`.
    pub fn sqs_cmd_anmm_oef<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        ctx: u32,
        cmd: u32,
        call: &mut C,
    ) -> u32 {
        let rec = mem.read::<u32>(ctx.wrapping_add(0x220));
        let slot = u32::from(mem.read::<u8>(rec.wrapping_add(0x1e)));
        let src = mem.read::<u32>(ctx.wrapping_add(0x360).wrapping_add(4 * slot));
        let id = u32::from(mem.read::<u16>(cmd.wrapping_add(2)));
        let f = mem.rebase(OP_OEF2_A_GET);
        let data = call(mem, f, &[src, 4, id]);
        mem.write(rec.wrapping_add(0xbb), 1u8);
        mem.write(rec.wrapping_add(0xe0), data);
        mem.write(rec.wrapping_add(0xc0), 0x0048_0080u32);
        mem.write(rec.wrapping_add(0xc4), 0u32);
        mem.write(rec.wrapping_add(0xf2), 0x100u32);
        let flags = mem.read::<u16>(ctx.wrapping_add(0x21a));
        #[allow(clippy::cast_possible_truncation)] // three bits each
        {
            mem.write(rec.wrapping_add(0xa8), (flags >> 13 & 7) as u8);
            mem.write(rec.wrapping_add(0xa9), (flags >> 9 & 7) as u8);
        }
        let low = mem.read::<u8>(ctx.wrapping_add(0x21a));
        mem.write(rec.wrapping_add(0xaa), low.wrapping_sub(0x24));
        let f = mem.rebase(OPU_ID_LIST_ADD);
        let id = u32::from(mem.read::<u16>(ctx.wrapping_add(0x214)));
        call(mem, f, &[ctx.wrapping_add(0x300), id]);
        let table = data.wrapping_add(0x10);
        mem.write(rec.wrapping_add(0xe4), table);
        let end = i32::from(mem.read::<i16>(table))
            .cast_unsigned()
            .wrapping_add(data);
        mem.write(rec.wrapping_add(0xe8), end);
        mem.write(rec.wrapping_add(0xf0), 0x200u16);
        mem.write(rec.wrapping_add(0xec), 0x0200_ffffu32);
        cmd.wrapping_add(4)
    }

    /// `opus_chr_width_00800cd0(chr, battle)` (`0x00800cd0`, in `ST0`).
    pub const OPUS_CHR_WIDTH: u32 = 0x0080_0cd0;

    /// `SqsCmdAnmmParam(ctx, cmd)` (`0x0081c160`, name ours; an IOP
    /// command like [`sqs_cmd_anmm`]) on the context's record (`+0x220`),
    /// by the mode bits `0xf000` of `ctx +0x21a`:
    /// - 0: bit 15 of the record's `+0xc0` set; returns `cmd + 2`.
    /// - `0x1000`: for the byte offsets `cmd +2` and `+3` (0 skips one),
    ///   the record's `f32` there truncated (`_ftol2`) to `+0xf2` and
    ///   `+0xf4`; returns `cmd + 6`.
    /// - `0x2000`: `+0xc6` the `u16` at `cmd +2`, with bit 9 of `+0x21a`
    ///   times the character's width (`opus_chr_width_00800cd0(record
    ///   +0x18, ctx +0x21e)`, the value as an `i16`, truncated); returns
    ///   `cmd + 4`.
    /// - `0x3000`: `+0xc4` the `u16` at `cmd +2`; returns `cmd + 4`.
    /// - Anything else returns `cmd`.
    pub fn sqs_cmd_anmm_param<M, F>(mem: &mut M, ctx: u32, cmd: u32, fcall: &mut F) -> u32
    where
        M: Mem,
        F: FnMut(&mut M, u32, crate::camera::FArgs<'_>) -> f64,
    {
        let rec = mem.read::<u32>(ctx.wrapping_add(0x220));
        let flags = mem.read::<u16>(ctx.wrapping_add(0x21a));
        #[allow(clippy::cast_possible_truncation)] // the low word is stored
        let low = |v: u32| v as u16;
        match flags & 0xf000 {
            0 => {
                let at = rec.wrapping_add(0xc0);
                mem.write(at, mem.read::<u16>(at) | 0x8000);
                cmd.wrapping_add(2)
            }
            0x1000 => {
                for (k, to) in [(2u32, 0xf2u32), (3, 0xf4)] {
                    let off = u32::from(mem.read::<u8>(cmd.wrapping_add(k)));
                    if off != 0 {
                        let v = x87::load(mem.read::<u32>(rec.wrapping_add(off)));
                        let i = x87::ftol(mem, v);
                        mem.write(rec.wrapping_add(to), low(i));
                    }
                }
                cmd.wrapping_add(6)
            }
            0x2000 => {
                let mut v = mem.read::<u16>(cmd.wrapping_add(2));
                if flags & 0x200 != 0 {
                    let chr = u32::from(mem.read::<u8>(rec.wrapping_add(0x18)));
                    let battle = i32::from(mem.read::<i8>(ctx.wrapping_add(0x21e))).cast_unsigned();
                    let f = mem.rebase(OPUS_CHR_WIDTH);
                    let width = fcall(mem, f, crate::camera::FArgs::Stack(&[chr, battle]));
                    let scale = x87::load(x87::store(f64::from(v.cast_signed())));
                    v = low(x87::ftol(mem, width * scale));
                }
                mem.write(rec.wrapping_add(0xc6), v);
                cmd.wrapping_add(4)
            }
            0x3000 => {
                mem.write(rec.wrapping_add(0xc4), mem.read::<u16>(cmd.wrapping_add(2)));
                cmd.wrapping_add(4)
            }
            _ => cmd,
        }
    }

    /// `Sqs_DecodeAdpcm(src, dst)` (`0x0081cc70`, name ours; from
    /// `SqsStraightStreamingDataSend`): 25 rounds of 256 left samples then
    /// 256 right ones (`i16` into `dst`, 0x6400 bytes), each from 16
    /// 9-byte blocks of `src` ([`super::adpcm_block`]), the filters at
    /// `0x00c49470`, each channel's history at `0x012f4f90`. Returns the
    /// end of `dst`.
    pub fn sqs_decode_adpcm(mem: &mut impl Mem, src: u32, dst: u32) -> u32 {
        let coefs_at = mem.rebase(super::SQS_ADPCM_COEFS);
        let state = mem.rebase(super::SQS_ADPCM_STATE);
        let (mut src, mut dst) = (src, dst);
        for _ in 0..25 {
            for ch in 0..2u32 {
                let at = state + 8 * ch;
                let [s1, s2] = mem.read::<[i32; 2]>(at);
                let mut s = (s1, s2);
                for _ in 0..16 {
                    let header = mem.read::<u8>(src);
                    let data = mem.read::<[u8; 8]>(src.wrapping_add(1));
                    let mut out = [0i16; 16];
                    super::adpcm_block(
                        header,
                        &data,
                        &mut s,
                        |k| {
                            let [c0, c1] = mem.read::<[i32; 2]>(coefs_at.wrapping_add(8 * k));
                            (c0, c1)
                        },
                        &mut out,
                    );
                    mem.write(dst, out);
                    src = src.wrapping_add(9);
                    dst = dst.wrapping_add(0x20);
                }
                mem.write(at, [s.0, s.1]);
            }
        }
        dst
    }

    // ---- field round 36: standby requests, data sends, info files ----

    /// Sends the queued IOP commands when there are any (`next > top`).
    fn flush_commands<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(mem: &mut M, call: &mut C) {
        if super::SQS_CMD_NEXT.get(mem) > super::SQS_CMD_TOP.get(mem) {
            sqs_iop_command_send(mem, call);
        }
    }

    /// `SqsReportStandby()` (`0x0081e260`, name ours: it sorts between
    /// `SqsReportIopSoundStatus` and `SqsReportStandbyStatus`): with
    /// requests unanswered (`0x012f414c` != `0x012f4150`) the queued
    /// commands go first; then the request count goes up and `(1, count)`
    /// to the second client's send buffer, sent with `Sqs_CallRpc2(0, _,
    /// end, 0)` (`end` = `0x0081fcb0`), whose result it returns.
    pub fn sqs_report_standby<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
    ) -> u32 {
        if super::SQS_STANDBY_SENT.get(mem) != super::SQS_IOP_RESULT.get(mem) {
            flush_commands(mem, call);
        }
        let n = super::SQS_STANDBY_SENT.get(mem).wrapping_add(1);
        super::SQS_STANDBY_SENT.set(mem, n);
        let buf = super::SQS_RPC_SEND2.get(mem);
        mem.write(buf, 1u32);
        let buf = super::SQS_RPC_SEND2.get(mem);
        mem.write(buf.wrapping_add(4), n);
        let done = mem.rebase(super::SQS_STANDBY_DONE);
        sqs_call_rpc2(mem, 0, done, 0, call)
    }

    /// `SqsReportStandbyStatus(flush)` (`0x0081e2d0`): with `flush` and
    /// requests unanswered, the queued commands go; returns the requests
    /// unanswered (sent less answered, wrapping).
    pub fn sqs_report_standby_status<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        flush: u32,
        call: &mut C,
    ) -> u32 {
        if flush != 0 && super::SQS_STANDBY_SENT.get(mem) != super::SQS_IOP_RESULT.get(mem) {
            flush_commands(mem, call);
        }
        super::SQS_STANDBY_SENT
            .get(mem)
            .wrapping_sub(super::SQS_IOP_RESULT.get(mem))
    }

    /// `SqsDataSendingCompleted(wait)` (`0x0081cfd0`): 1 when
    /// `SqsSifDmaState(1, wait)` is not negative, else 0.
    pub fn sqs_data_sending_completed<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        wait: u32,
        call: &mut C,
    ) -> u32 {
        u32::from(sqs_sif_dma_state(mem, wait, call).cast_signed() >= 0)
    }

    /// Copies the string `s` (its NUL included) to the send buffer and
    /// calls `Sqs_CallRpc(cmd)`.
    fn send_string<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        s: u32,
        cmd: u32,
        call: &mut C,
    ) -> u32 {
        let buf = super::SQS_RPC_SEND.get(mem);
        let mut i = 0u32;
        loop {
            let c = mem.read::<u8>(s.wrapping_add(i));
            mem.write(buf.wrapping_add(i), c);
            i = i.wrapping_add(1);
            if c == 0 {
                break;
            }
        }
        sqs_call_rpc(mem, cmd, call)
    }

    /// `SqsInitMusicInfoHdd(path)` (`0x0081d3c0`, command 7) and
    /// `SqsInitWaveInfoHdd(path)` (`0x0081d470`, command 6): the path to
    /// the send buffer, then `Sqs_CallRpc`; its result.
    pub fn sqs_init_info_hdd<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        path: u32,
        cmd: u32,
        call: &mut C,
    ) -> u32 {
        send_string(mem, path, cmd, call)
    }

    /// `SqsSetProjectBase(path)` (`0x0081eab0`): the path through command 2;
    /// a non-zero result is a pointer, whose word it returns (0 stays 0).
    pub fn sqs_set_project_base<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        path: u32,
        call: &mut C,
    ) -> u32 {
        let r = send_string(mem, path, 2, call);
        if r == 0 { 0 } else { mem.read::<u32>(r) }
    }

    // ---- field round 39: the IOP command builders ----

    /// Appends the 32-byte command `cmd` to the queue (`0x00c4946c`); a
    /// command that would pass `0x012f4e00` sends the queue first, with a
    /// `dbgPrintf` when bit 0 of `0x012f4148` is set. Returns the `eax`
    /// this leaves: the send's result, else the old end plus 0x20.
    fn push_command<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        cmd: [u32; 8],
        call: &mut C,
    ) -> u32 {
        let mut at = super::SQS_CMD_NEXT.get(mem);
        let mut eax = at.wrapping_add(0x20);
        if eax > mem.rebase(super::SQS_CMD_LIMIT) {
            if super::SQS_DEBUG_FLAGS.get(mem) & 1 != 0 {
                let f = mem.rebase(super::DBG_PRINTF);
                let msg = mem.rebase(super::SQS_CMD_FULL_MESSAGE);
                call(mem, f, &[msg]);
            }
            eax = sqs_iop_command_send(mem, call);
            at = super::SQS_CMD_NEXT.get(mem);
        }
        mem.write(at, cmd);
        super::SQS_CMD_NEXT.set(mem, at.wrapping_add(0x20));
        eax
    }

    /// `(op, a, b, c)` padded with zeros to a command.
    const fn cmd(op: u32, a: u32, b: u32, c: u32) -> [u32; 8] {
        [op, a, b, c, 0, 0, 0, 0]
    }

    /// The one-argument command builders that return their argument:
    /// `SqsSetCycleMode(a)` (`0x0081e900`, op 4), `SqsSetReverbPlaySwitch`
    /// (`0x0081ec20`, op `0xc`), `SqsSetStraightStreamAreaSize`
    /// (`0x0081ef50`, op 5).
    pub fn sqs_cmd_arg<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        op: u32,
        a: u32,
        call: &mut C,
    ) -> u32 {
        push_command(mem, cmd(op, a, 0, 0), call);
        a
    }

    /// `SqsAllVoiceTerminate()` (`0x0081ce70`): command 0.
    pub fn sqs_all_voice_terminate<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
    ) -> u32 {
        push_command(mem, cmd(0, 0, 0, 0), call)
    }

    /// `SqsMusicDataRead(a)` (`0x0081d820`): a standby request
    /// ([`sqs_report_standby`]), command `(0x15, a)`, then the queue sent
    /// (a tail call to `SqsIopCommandSend`, whose result it returns).
    pub fn sqs_music_data_read<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        a: u32,
        call: &mut C,
    ) -> u32 {
        sqs_report_standby(mem, call);
        push_command(mem, cmd(0x15, a, 0, 0), call);
        sqs_iop_command_send(mem, call)
    }

    /// `SqsSetPlayMode(mode)` (`0x0081ea00`): modes 3 and up (signed)
    /// become 0; kept in `0x012f4144` and sent as `(3, mode)`.
    pub fn sqs_set_play_mode<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        mode: u32,
        call: &mut C,
    ) -> u32 {
        let v = if mode.cast_signed() >= 3 { 0 } else { mode };
        super::SQS_PLAY_MODE.set(mem, v);
        push_command(mem, cmd(3, v, 0, 0), call)
    }

    /// `SqsShoutStreamingStart(a)` (`0x0081ee20`): `(0x37, a, 0x40,
    /// 0x7f)`, then `0x012f4f84` and the shout flag `0x012f4e94` 1.
    /// Returns `a`.
    pub fn sqs_shout_streaming_start<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        a: u32,
        call: &mut C,
    ) -> u32 {
        push_command(mem, cmd(0x37, a, 0x40, 0x7f), call);
        let h = mem.rebase(super::SQS_REPLY_HISTORY);
        mem.write(h + 0x3c, 1u32);
        super::SHOUT_SKIPPED.set(mem, 1);
        a
    }

    /// `SqsStraightStreamingStart()` (`0x0081f6c0`): `(0x38, id)` with the
    /// stream id `0x012f4e70` (read before a send), then `0x012f4e90` 0,
    /// `0x012f4edc` 1, `0x012f4ef8` 0.
    pub fn sqs_straight_streaming_start<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
    ) -> u32 {
        let id = super::SQS_STRAIGHT_STREAM.get(mem);
        let r = push_command(mem, cmd(0x38, id, 0, 0), call);
        for (at, v) in [(0x012f_4e90, 0u32), (0x012f_4edc, 1), (0x012f_4ef8, 0)] {
            let a = mem.rebase(at);
            mem.write(a, v);
        }
        r
    }

    /// `SqsStraightStreamingStop()` (`0x0081f770`): `(0x33, 0)`, then
    /// `0x012f4efc` 3 and `0x012f4edc` 0.
    pub fn sqs_straight_streaming_stop<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
    ) -> u32 {
        let r = push_command(mem, cmd(0x33, 0, 0, 0), call);
        for (at, v) in [(0x012f_4efc, 3u32), (0x012f_4edc, 0)] {
            let a = mem.rebase(at);
            mem.write(a, v);
        }
        r
    }

    /// `SqsStraightStreamingVolumeDirect(v)` (`0x0081f800`): `(0x3e, v &
    /// 0x7f)`.
    pub fn sqs_straight_streaming_volume_direct<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        v: u32,
        call: &mut C,
    ) -> u32 {
        push_command(mem, cmd(0x3e, v & 0x7f, 0, 0), call)
    }

    /// `SqsStraightStreamingVolumeTranslate(frames, v)` (`0x0081f880`):
    /// `(0x3f, frames, v & 0x7f)`; returns `frames`.
    pub fn sqs_straight_streaming_volume_translate<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        frames: u32,
        v: u32,
        call: &mut C,
    ) -> u32 {
        push_command(mem, cmd(0x3f, frames, v & 0x7f, 0), call);
        frames
    }

    /// `SqsInitWaveInfo(src, _, a)` (`0x0081d3f0`): nothing once
    /// `0x012f4140` is set (the caller's `eax` comes back). Else a source
    /// sends 0x5000 bytes by SIF DMA to `*0x012f4f88` (the record at
    /// `0x012f4e00`, id 0, then `SqsSifDmaState(1, 1)`; the `FlushCache(0)`
    /// before is a bare `ret`), and `a` goes to the send buffer for
    /// `Sqs_CallRpc(4)`. No caller reads `eax`.
    pub fn sqs_init_wave_info<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        src: u32,
        a: u32,
        call: &mut C,
    ) {
        if super::SQS_WAVE_INFO_DONE.get(mem) != 0 {
            return;
        }
        if src != 0 {
            let dst = super::SQS_WAVE_INFO_DST.get(mem);
            let r = mem.rebase(super::SIF_DMA);
            mem.write(r, [src, dst, 0x5000, 0]);
            super::SIF_DMA_ID.set(mem, 0);
            sqs_sif_dma_state(mem, 1, call);
        }
        let buf = super::SQS_RPC_SEND.get(mem);
        mem.write(buf, a);
        sqs_call_rpc(mem, 4, call);
    }

    // ---- field round 40: separate SE wave data ----

    /// `SqsSeSeparateDataSend(data, size)` (`0x0081e600`): collects the
    /// wave ids of the chunks in `data` and loads each.
    /// - A size outside `1..=0x10000` only reports (`dbgPrintf`, a bare
    ///   `ret`, when bit 0 of `0x012f4148` is set) and returns `size - 1`.
    /// - While more than 16 bytes are left, a chunk must start with the
    ///   words `0x65536553`, `0x20202070` and have a positive multiple of 4
    ///   at `+0xc` (its length); its `u16` id at `+0x11` joins the list at
    ///   `0x012fb3c0` unless already there (256 at most; reaching 256
    ///   prints "Too many wave datat..." in red to stderr and flushes it).
    /// - Each listed id goes to `iSfxLoadData(id, 0)`, then the queue is
    ///   sent (a tail call to `SqsIopCommandSend`, whose result it returns).
    pub fn sqs_se_separate_data_send<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        data: u32,
        size: u32,
        call: &mut C,
    ) -> u32 {
        if size.wrapping_sub(1) > 0xffff {
            if super::SQS_DEBUG_FLAGS.get(mem) & 1 != 0 {
                let f = mem.rebase(super::DBG_PRINTF);
                let fmt = mem.rebase(super::SQS_SEP_SIZE_FORMAT);
                let pre = mem.rebase(super::SQS_SEP_SIZE_PREFIX);
                call(mem, f, &[fmt, pre, size]);
            }
            return size.wrapping_sub(1);
        }
        let table = mem.rebase(super::SQS_SEP_WAVES);
        let (mut p, mut left, mut n) = (data, size, 0u32);
        while left > 0x10 {
            let [magic, tag, _, len] = mem.read::<[u32; 4]>(p);
            if magic != 0x6553_6553 || tag != 0x2020_2070 || len.cast_signed() <= 0 || len & 3 != 0
            {
                break;
            }
            let id = u32::from(mem.read::<u8>(p.wrapping_add(0x12))) << 8
                | u32::from(mem.read::<u8>(p.wrapping_add(0x11)));
            let known = (0..n).any(|i| mem.read::<u32>(table + 4 * i) == id);
            if !known && n < 0x100 {
                mem.write(table + 4 * n, id);
                n += 1;
                if n >= 0x100 {
                    for msg in [
                        super::SQS_SEP_RED,
                        super::SQS_SEP_TOO_MANY,
                        super::SQS_SEP_RESET,
                    ] {
                        print_error(mem, msg, call);
                    }
                    let iob = mem.read::<u32>(mem.rebase(crate::yi::CRT_IOB_FUNC));
                    let fflush = mem.read::<u32>(mem.rebase(super::CRT_FFLUSH));
                    let stderr = call(mem, iob, &[]).wrapping_add(0x40);
                    call(mem, fflush, &[stderr]);
                }
            }
            left = left.wrapping_sub(len);
            p = p.wrapping_add(len);
        }
        for i in 0..n {
            let id = mem.read::<u32>(table + 4 * i);
            glue(mem, super::I_SFX_LOAD_DATA, &[id, 0], call);
        }
        sqs_iop_command_send(mem, call)
    }

    /// `SndStandbyShoutStream(data)` (`0x0081e3b0`): for a length at
    /// `+0xc` in `1..=0x10000` (signed), `iSfxLoadData(i16 +8, 0)`, whose
    /// result it returns; else the length (left in `eax`).
    pub fn snd_standby_shout_stream<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        data: u32,
        call: &mut C,
    ) -> u32 {
        let len = mem.read::<i32>(data.wrapping_add(0xc));
        if len <= 0 || len > 0x1_0000 {
            return len.cast_unsigned();
        }
        let id = i32::from(mem.read::<i16>(data.wrapping_add(8))).cast_unsigned();
        glue(mem, super::I_SFX_LOAD_DATA, &[id, 0], call)
    }

    /// `op_oef2_a_get(oef, kind, n)` (`0x007e3810`).
    pub const OP_OEF2_A_GET: u32 = 0x007e_3810;
    /// `op_md_packet_set_addr(p, lo, hi)` (`0x00a69de0`).
    pub const OP_MD_PACKET_SET_ADDR: u32 = 0x00a6_9de0;

    /// `SqsOefSetPacketAddrRange(oef, lo, hi, v)` (`0x0081ca20`, name ours;
    /// called by `opu_md_etc`, placed among the `Sqs*`): for each `n` from
    /// `lo` to `hi` (signed, inclusive), the effect's kind-4 entry `n`
    /// (`op_oef2_a_get`) gets `op_md_packet_set_addr(entry, v, v >> 31)`
    /// when there is one. Its caller ignores `eax`.
    pub fn sqs_oef_set_packet_addr_range<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        [oef, lo, hi, v]: [u32; 4],
        call: &mut C,
    ) {
        let (get, set) = (mem.rebase(OP_OEF2_A_GET), mem.rebase(OP_MD_PACKET_SET_ADDR));
        let mut n = lo.cast_signed();
        while n <= hi.cast_signed() {
            let e = call(mem, get, &[oef, 4, n.cast_unsigned()]);
            if e != 0 {
                call(mem, set, &[e, v, (v.cast_signed() >> 31).cast_unsigned()]);
            }
            n = n.wrapping_add(1);
        }
    }

    // ---- field r13, sound command stubs ----

    /// The PC build's "not implemented" messages of `SqsSePitchDirect`
    /// (`0x00b58e30`) and `SqsSePitchTranslate` (`0x00b58e88`), `%d` the
    /// sequence id.
    pub const PITCH_MESSAGES: [u32; 2] = [0x00b5_8e30, 0x00b5_8e88];
    /// `iSfxPause(id)` (`0x0067a380`).
    pub const SFX_PAUSE: u32 = 0x0067_a380;

    /// `SqsSePitchDirect(kind, id, ..)` (`0x0081e4a0`) and
    /// `SqsSePitchTranslate` (`0x0081e4c0`, `translate`): on PC only
    /// `fprintf(stderr, "SFX SqsSePitch..: try to set pitch for seq(%d),
    /// function is not implemented yet!!!\n", id)`, whose `eax` it
    /// returns.
    pub fn sqs_se_pitch<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        translate: bool,
        id: u32,
        call: &mut C,
    ) -> u32 {
        let iob = mem.read::<u32>(mem.rebase(crate::yi::CRT_IOB_FUNC));
        let fprintf = mem.read::<u32>(mem.rebase(crate::yi::CRT_FPRINTF));
        let message = mem.rebase(PITCH_MESSAGES[usize::from(translate)]);
        let stderr = call(mem, iob, &[]).wrapping_add(0x40);
        call(mem, fprintf, &[stderr, message, id])
    }

    /// `SqsSetPlayPause(flags, id)` (`0x0081ea90`): `iSfxPause(id)` when
    /// bit 1 of `flags` is set (its `eax`), else nothing (`None`: the
    /// caller's `eax` stays).
    pub fn sqs_set_play_pause<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        [flags, id]: [u32; 2],
        call: &mut C,
    ) -> Option<u32> {
        if flags & 2 == 0 {
            return None;
        }
        let f = mem.rebase(SFX_PAUSE);
        Some(call(mem, f, &[id]))
    }

    // ---- field r14 ----

    /// `FFXSifBindRpc(client, id, mode)` (`0x00a44740`), `FlushCache(0)`
    /// (`0x0062f4c0`) and `Sqs_CallRpc(cmd)` (`0x0081cbd0`):
    /// [`sqs_init_iop`]'s calls.
    pub const INIT_IOP_CALLS: [u32; 3] = [0x00a4_4740, 0x0062_f4c0, 0x0081_cbd0];
    /// The words [`sqs_init_iop`] clears (`0x012f4f44`, `0x012f4e78`,
    /// `0x012f4e74`, `0x012f4edc`), the one it sets to 3 (`0x012f4efc`) and
    /// the IOP buffer words it fills from the reply (`0x012f4f88`,
    /// `0x012f4f8c`, `0x012f4ec4`, `0x012f4ec0`, `g_SqsShoutBuffer`).
    pub const INIT_IOP_CLEARED: [u32; 4] = [0x012f_4f44, 0x012f_4e78, 0x012f_4e74, 0x012f_4edc];
    /// See [`INIT_IOP_CLEARED`].
    pub const INIT_IOP_THREE: u32 = 0x012f_4efc;
    /// The reply words' homes: reply words 1, 2, 3, 0 and 4.
    pub const INIT_IOP_REPLY: [(u32, u32); 5] = [
        (4, 0x012f_4f88),
        (8, 0x012f_4f8c),
        (0xc, 0x012f_4ec4),
        (0, 0x012f_4ec0),
        (0x10, 0x012f_4ecc),
    ];
    /// The shout buffer's size word (`0x012f4ec8`, 0xc800) and the ready
    /// flag (`0x012f4148`, 1).
    pub const INIT_IOP_SIZES: [(u32, u32); 2] = [(0x012f_4ec8, 0xc800), (0x012f_4148, 1)];
    /// `g_SqsWaveInfoDone` (`0x012f4140`), which gets the mode.
    pub const WAVE_INFO_DONE: u32 = 0x012f_4140;

    /// `SqsInitIop_0081efd0(mode)` (`0x0081efd0`, from `0x008879d0` with
    /// the debug byte `0x00b5ec4f`; name ours): the sound driver's two RPC
    /// clients are bound (`FFXSifBindRpc(0x012f4e20, 0x1000, 0)`, then
    /// `(0x012f4e48, 0x1001, 0)`, again until both clients' server words
    /// `+0x24` are set; a negative bind returns 0). Then `FlushCache(0)`,
    /// the IOP state words cleared ([`INIT_IOP_CLEARED`], 3 at
    /// [`INIT_IOP_THREE`]), `mode` to `g_SqsWaveInfoDone` and the send
    /// buffer's first word, the status words' address
    /// ([`super::SQS_IOP_STATUS`]) to its second, `Sqs_CallRpc(1)`; the
    /// reply's five words go to [`INIT_IOP_REPLY`], [`INIT_IOP_SIZES`] are
    /// set and the 16 status words cleared. Returns reply word 0.
    pub fn sqs_init_iop<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        mode: u32,
        call: &mut C,
    ) -> u32 {
        let [bind, flush, call_rpc] = INIT_IOP_CALLS.map(|f| mem.rebase(f));
        let [client, client2] =
            [super::SQS_RPC_CLIENT, super::SQS_RPC_CLIENT2].map(|a| mem.rebase(a));
        let bound = |mem: &M, c: u32| mem.read::<u32>(c.wrapping_add(0x24)) != 0;
        let mut r = call(mem, bind, &[client, 0x1000, 0]);
        loop {
            if r.cast_signed() < 0 {
                return 0;
            }
            r = call(mem, bind, &[client2, 0x1001, 0]);
            if r.cast_signed() < 0 {
                return 0;
            }
            if bound(mem, client) && bound(mem, client2) {
                break;
            }
            r = call(mem, bind, &[client, 0x1000, 0]);
        }
        call(mem, flush, &[0]);
        let send = super::SQS_RPC_SEND.get(mem);
        for a in INIT_IOP_CLEARED {
            mem.write(mem.rebase(a), 0u32);
        }
        mem.write(mem.rebase(INIT_IOP_THREE), 3u32);
        mem.write(mem.rebase(WAVE_INFO_DONE), mode);
        mem.write(send, mode);
        let status = super::SQS_IOP_STATUS.ptr(mem).addr;
        mem.write(send.wrapping_add(4), status);
        call(mem, call_rpc, &[1]);
        let recv = super::SQS_RPC_RECV.get(mem);
        let reply = mem.read::<[u32; 5]>(recv);
        for (o, at) in INIT_IOP_REPLY {
            let w = reply[(o / 4) as usize];
            mem.write(mem.rebase(at), w);
        }
        for (at, v) in INIT_IOP_SIZES {
            mem.write(mem.rebase(at), v);
        }
        mem.write(status, [0u32; 16]);
        reply[0]
    }
}
