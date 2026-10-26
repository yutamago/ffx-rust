//! Continuation round 4 of list `h4-glue-other`, batch 5: the FMOD
//! voice's event read and the sync manager's setup.

use ffx_graphic::host::{Conv, Host};

use crate::frames::{ERRCHECK, FIOS_UNIFY, FMOD_VOICE_FILE};
use crate::h4_c3::VOICE_STOP;
use crate::h4_d1::EVENT_GET_INFO;
use crate::h4_e5::{FPRINTF, IOB_FUNC};
use crate::h4_f1::VOICE_NO_PLAY;
use crate::h4_f3::{MEMSET, SPRINTF_IAT};

fn errcheck(h: &mut impl Host, r: u32, line: u32) {
    let file = h.rebase(FMOD_VOICE_FILE);
    h.call_sym(ERRCHECK, Conv::Cdecl, &[r, file, line]);
}

// ---- the voice

/// The voice ids `FmodVoice_read` skips while the voice is off (`+8` 0).
pub const SKIPPED_WHEN_OFF: [u32; 4] = [0x0290_d1c1, 0x0290_e1c1, 0x0290_f1c1, 0x0291_01c1];
/// The voice ids up to `0x33000000` that are read from the common group
/// (`+0x1c`) rather than a numbered one.
pub const COMMON_IDS: [u32; 5] = [0x0834_1504, 0x0c54_5504, 0x2874_5504, 0x2874_6504, 0x504];
/// `"voiceRead: mVoiceEvent not find! fileName = %s\n"` (`0x00b4fb30`).
pub const NOT_FOUND: u32 = 0x00b4_fb30;

/// Gets event `name` of `group` (vtable `+0x20`: (group, name, 0,
/// `this + 0x14`), `ERRCHECK` at `line`); a missing one (0x59) is
/// reported on `stderr`.
fn get_event(h: &mut impl Host, this: u32, group: u32, name: u32, line: u32) {
    let vtable: u32 = h.read(group);
    let f: u32 = h.read(vtable.wrapping_add(0x20));
    // stdcall with the group on the stack; `ecx` holds it too
    let r = h.call(
        f,
        Conv::Thiscall(group),
        &[group, name, 0, this.wrapping_add(0x14)],
    );
    errcheck(h, r, line);
    if r == 0x59 {
        let iob = IOB_FUNC.get(h);
        let stderr = h.call(iob, Conv::Cdecl, &[]).wrapping_add(0x20);
        let fprintf = FPRINTF.get(h);
        let fmt = h.rebase(NOT_FOUND);
        h.call(fprintf, Conv::Cdecl, &[stderr, fmt, name]);
    }
}

/// `FmodVoice_read(this; id, flag)` (thiscall, ret 8): nothing for `flag`
/// 0, or for the four ids [`SKIPPED_WHEN_OFF`] while the voice is off
/// (`+8` 0). A playing event (`+0x14`) is stopped (`FmodVoice_stop`;
/// `+0x40`/`+0x44`/`+0x48` 0); the group (`+0x10`) is cleared. In the
/// voice table (`+0x24`: `"ML\0\0"`, a count, 8-byte (id, offset)
/// entries) `id` gives its event name (table + offset; none: nothing
/// more). Ids above `0x33000000` and the five [`COMMON_IDS`] come from
/// the common group (`+0x1c`, line 0x3f5); the others from numbered group
/// `10 (name[0] - '0') + name[1] - '0'` of `+0xc` (group 0 with the loop
/// flag `0x00cec184` 1 or voice `+0` 0x10e; line 0x40a). With the event
/// the group becomes `+0x10`. Unless an info is pending (`+0x60 == 1`):
/// with an event, its info (`+0x68`, 13 words cleared first) is fetched
/// (`Event::getInfo(event, 0, 0, info)`, line 0x429), its `+8` kept in
/// `+0x6c`, `id` in `+0x5c`, `+0x60` 1; then `+0x64`, `+0x58` 0. Returns
/// 0.
pub fn voice_read(h: &mut impl Host, this: u32, id: u32, flag: u32) -> u32 {
    if flag == 0 {
        return 0;
    }
    if h.read::<u32>(this.wrapping_add(8)) == 0 && SKIPPED_WHEN_OFF.contains(&id) {
        return 0;
    }
    if h.read::<u32>(this.wrapping_add(0x14)) != 0 {
        h.call_sym(VOICE_STOP, Conv::Thiscall(this), &[]);
        for off in [0x40u32, 0x44, 0x48] {
            h.write(this.wrapping_add(off), 0u32);
        }
    }
    let table: u32 = h.read(this.wrapping_add(0x24));
    h.write(this.wrapping_add(0x10), 0u32);
    if h.read::<u32>(table) != u32::from_le_bytes(*b"ML\0\0") {
        return 0;
    }
    let n: u32 = h.read(table.wrapping_add(4));
    let mut entry = None;
    for i in 0..n {
        let e = table.wrapping_add(8 + 8 * i);
        if h.read::<u32>(e) == id {
            entry = Some(e);
            break;
        }
    }
    let Some(e) = entry else {
        return 0;
    };
    let name = h
        .read::<u32>(e.wrapping_add(4))
        .wrapping_add(h.read(this.wrapping_add(0x24)));
    if id > 0x3300_0000 || COMMON_IDS.contains(&id) {
        let group: u32 = h.read(this.wrapping_add(0x1c));
        if group != 0 {
            get_event(h, this, group, name, 0x3f5);
            if h.read::<u32>(this.wrapping_add(0x14)) != 0 {
                let group: u32 = h.read(this.wrapping_add(0x1c));
                h.write(this.wrapping_add(0x10), group);
            }
        }
    } else {
        let k = if h.read::<u32>(h.rebase(VOICE_NO_PLAY)) == 1 || h.read::<u32>(this) == 0x10e {
            0
        } else {
            let tens = i32::from(h.read::<i8>(name));
            let ones = i32::from(h.read::<i8>(name.wrapping_add(1)));
            tens.wrapping_mul(5)
                .wrapping_sub(0x108)
                .wrapping_mul(2)
                .wrapping_add(ones)
                .cast_unsigned()
        };
        let groups: u32 = h.read(this.wrapping_add(0xc));
        let group: u32 = h.read(groups.wrapping_add(k.wrapping_mul(4)));
        if group != 0 {
            get_event(h, this, group, name, 0x40a);
            if h.read::<u32>(this.wrapping_add(0x14)) != 0 {
                let groups: u32 = h.read(this.wrapping_add(0xc));
                let group: u32 = h.read(groups.wrapping_add(k.wrapping_mul(4)));
                h.write(this.wrapping_add(0x10), group);
            }
        }
    }
    if h.read::<u32>(this.wrapping_add(0x60)) == 1 {
        return 0;
    }
    let event: u32 = h.read(this.wrapping_add(0x14));
    if event != 0 {
        for k in 0..13u32 {
            let info: u32 = h.read(this.wrapping_add(0x68));
            h.write(info.wrapping_add(4 * k), 0u32);
        }
        let info: u32 = h.read(this.wrapping_add(0x68));
        let event: u32 = h.read(this.wrapping_add(0x14));
        let r = h.call_sym(EVENT_GET_INFO, Conv::Stdcall, &[event, 0, 0, info]);
        errcheck(h, r, 0x429);
        let info: u32 = h.read(this.wrapping_add(0x68));
        let v: u32 = h.read(info.wrapping_add(8));
        h.write(this.wrapping_add(0x6c), v);
        h.write(this.wrapping_add(0x5c), id);
        h.write(this.wrapping_add(0x60), 1u32);
    }
    h.write(this.wrapping_add(0x64), 0u32);
    h.write(this.wrapping_add(0x58), 0u32);
    0
}

// ---- the sync manager

/// `SyncManager_readSyncFile(this; path)` (thiscall): the file's bytes
/// (aligned allocation) or 0.
pub const READ_SYNC_FILE: u32 = 0x006f_acf0;
/// `user_aligned_free(p)` (cdecl).
pub const USER_ALIGNED_FREE: u32 = 0x0068_6fe0;
/// `dbgPrintf(fmt, ...)` (cdecl).
pub const DBG_PRINTF: u32 = 0x0062_f500;
/// `"/FFX_Data/GameData/%sData/Sync"`-style list path format
/// (`0x00b4c370`).
pub const SYNC_LIST_FORMAT: u32 = 0x00b4_c370;
/// `"Read SyncList.txt failed!\n"` (`0x00b4c3a0`).
pub const SYNC_LIST_FAILED: u32 = 0x00b4_c3a0;
/// Bytes of [`sync_init`]'s frame, from `ebp - 0x204`: the unified path
/// (`+0`) and the formatted one (`+0x100`), 0x100 bytes each.
pub const SYNC_INIT_FRAME: u32 = 0x200;

/// `SyncManager_syncInit(this)` (thiscall; `frame` see
/// [`SYNC_INIT_FRAME`]): the sync list's path (`sprintf` of
/// `0x00b4c370` with `+0`, unified by `fiosUnifyFilename` into zeroed
/// buffers) is read (`SyncManager_readSyncFile`; none: `"Read
/// SyncList.txt failed!"` printed). An `"SL\0\0"` file's count of words
/// is appended to the list at `+0x40` (count `+0x18`); the file is freed
/// (`user_aligned_free`). Returns 0.
pub fn sync_init(h: &mut impl Host, frame: u32, this: u32) -> u32 {
    let unified = frame;
    let path = frame.wrapping_add(0x100);
    for buf in [unified, path] {
        h.write(buf, 0u8);
        h.call_sym(MEMSET, Conv::Cdecl, &[buf.wrapping_add(1), 0, 0xff]);
    }
    let sprintf: u32 = h.read(h.rebase(SPRINTF_IAT));
    let fmt = h.rebase(SYNC_LIST_FORMAT);
    let name: u32 = h.read(this);
    h.call(sprintf, Conv::Cdecl, &[path, fmt, name]);
    h.call_sym(FIOS_UNIFY, Conv::Cdecl, &[path, unified, 0x100]);
    let data = h.call_sym(READ_SYNC_FILE, Conv::Thiscall(this), &[unified]);
    if data == 0 {
        let msg = h.rebase(SYNC_LIST_FAILED);
        h.call_sym(DBG_PRINTF, Conv::Cdecl, &[msg]);
        return 0;
    }
    if h.read::<u32>(data) == u32::from_le_bytes(*b"SL\0\0") {
        let n: u32 = h.read(data.wrapping_add(4));
        for k in 0..n {
            let v: u32 = h.read(data.wrapping_add(8 + 4 * k));
            let count: u32 = h.read(this.wrapping_add(0x18));
            let list: u32 = h.read(this.wrapping_add(0x40));
            h.write(list.wrapping_add(count.wrapping_mul(4)), v);
            let count: u32 = h.read(this.wrapping_add(0x18));
            h.write(this.wrapping_add(0x18), count.wrapping_add(1));
        }
    }
    h.call_sym(USER_ALIGNED_FREE, Conv::Cdecl, &[data]);
    0
}
