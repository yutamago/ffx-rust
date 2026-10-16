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
}
