//! The input log's replay command (list `phyre-framework-input`, batch
//! 9): `PApplicationScriptProxy::Frames`, which feeds the recorded input
//! frames (0x90-byte records, `+4` the frame number) whose time has come
//! to the application, and drops the log when it has played out.

use ffx_graphic::host::{Conv, Host};

/// The input log: record count (`PArray` count, top bit "borrowed"), the
/// next record's index, the records.
pub const G_LOG_COUNT: u32 = 0x00cc_9d14;
pub const G_LOG_NEXT: u32 = 0x00cc_9cf8;
pub const G_LOG_RECORDS: u32 = 0x00cc_9d18;
/// `PApplicationScriptProxy::applyFrameInput(app, record, frame)` (cdecl,
/// `al`; in the list).
pub const APPLY_FRAME_INPUT: u32 = 0x0062_6070;
/// The application's next scripted step (thiscall).
pub const SCRIPT_STEP: u32 = 0x0062_7940;
/// `Phyre::PhyreFree(p)` (cdecl).
pub const PHYRE_FREE: u32 = 0x0042_fb20;

pub mod bridge {
    use super::{
        APPLY_FRAME_INPUT, Conv, G_LOG_COUNT, G_LOG_NEXT, G_LOG_RECORDS, Host, PHYRE_FREE,
        SCRIPT_STEP,
    };

    /// The log dropped: the next index 0 and, for a non-empty log, the
    /// owned records freed and both words cleared. Returns the count word
    /// the original keeps in `ecx` afterwards (0, or `count` when 0).
    fn drop_log(h: &mut impl Host, count: u32) -> u32 {
        let next = h.rebase(G_LOG_NEXT);
        h.write(next, 0u32);
        if count == 0 {
            return 0;
        }
        let recs_at = h.rebase(G_LOG_RECORDS);
        let recs: u32 = h.read(recs_at);
        if recs != 0 && count & 0x8000_0000 == 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[recs]);
        }
        h.write(recs_at, 0u32);
        let count_at = h.rebase(G_LOG_COUNT);
        h.write(count_at, 0u32);
        0
    }

    /// `PApplicationScriptProxy::Frames(app, args)` (`0x00623660`, cdecl):
    /// `+0x14` cleared; every logged record from the next index whose
    /// frame is before the application's frame `+0x34`, then those of
    /// exactly that frame, is applied (an applied one makes the
    /// application's virtual `+0x1c` run once after, whose non-zero answer
    /// is returned); a played-out log is dropped. Then, while paused
    /// (`+0x39`), the log is dropped too and 0 returned; else the next
    /// scripted step, whose 0 answer steps the frame back (`+0x34` one
    /// less).
    pub fn frames(h: &mut impl Host, app: u32) -> u32 {
        h.write(app + 0x14, 0u8);
        let count_at = h.rebase(G_LOG_COUNT);
        let next_at = h.rebase(G_LOG_NEXT);
        let recs_at = h.rebase(G_LOG_RECORDS);
        let mut count: u32 = h.read(count_at);
        let n = count & 0x7fff_ffff;
        if n != 0 {
            let frame: u32 = h.read(app + 0x34);
            let mut idx: u32 = h.read(next_at);
            let mut any = false;
            let mut exact = false;
            while idx < n {
                let recs: u32 = h.read(recs_at);
                let rec = recs.wrapping_add(idx.wrapping_mul(0x90));
                let at: u32 = h.read(rec + 4);
                if at >= frame {
                    exact = true;
                    break;
                }
                if h.call_sym(APPLY_FRAME_INPUT, Conv::Cdecl, &[app, rec, at]) as u8 != 0 {
                    any = true;
                }
                idx = h.read::<u32>(next_at).wrapping_add(1);
                h.write(next_at, idx);
            }
            if exact {
                while idx < n {
                    let recs: u32 = h.read(recs_at);
                    let rec = recs.wrapping_add(idx.wrapping_mul(0x90));
                    if h.read::<u32>(rec + 4) != frame {
                        break;
                    }
                    if h.call_sym(APPLY_FRAME_INPUT, Conv::Cdecl, &[app, rec, frame]) as u8 != 0 {
                        any = true;
                    }
                    idx = h.read(next_at);
                    if idx >= n {
                        break;
                    }
                    idx = idx.wrapping_add(1);
                    h.write(next_at, idx);
                }
            }
            if any {
                let r = h.call_virtual(app, 7, &[]);
                if r != 0 {
                    return r;
                }
                idx = h.read(next_at);
            }
            count = h.read(count_at);
            if idx >= n {
                count = drop_log(h, count);
            }
        }
        if h.read::<u8>(app + 0x39) != 0 {
            drop_log(h, count);
            return 0;
        }
        let r = h.call_sym(SCRIPT_STEP, Conv::Thiscall(app), &[]);
        if r == 0 {
            let f: u32 = h.read(app + 0x34);
            h.write(app + 0x34, f.wrapping_sub(1));
        }
        r
    }
}
