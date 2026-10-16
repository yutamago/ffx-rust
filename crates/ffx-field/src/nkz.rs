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
pub mod bridge {
    use ffx_bridge::Mem;

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
}
