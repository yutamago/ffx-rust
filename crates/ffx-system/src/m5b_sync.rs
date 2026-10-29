//! M5 `system-b`: the voice/lip-sync data lists (`SyncManager`), the FMOD
//! command ring's flush and the sound objects' constructor.
//!
//! `SyncManager`: `+0` the platform name (`"%sData"`), `+4` the 128 loaded
//! sync files, `+0xc` the current event, `+0x10` the file entries,
//! `+0x14` the voice entries, `+0x1c` enabled, `+0x20` done, and the
//! entry columns (128 words each): files `+0x28`, `+0x30`, `+0x34`,
//! `+0x38`; voices `+0x24`, `+0x2c`, `+0x3c`. A list file starts with
//! `"SL\0\0"` and a record count, then records of five (files) or four
//! (voices) words, the first (files) or second (voices) the event id.

#![allow(
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::many_single_char_names
)]

/// The list paths and the per-entry file name format.
pub const FILE_LIST_FMT: u32 = 0x00b4_c3bc;
pub const VOICE_LIST_FMT: u32 = 0x00b4_c410;
pub const ENTRY_FMT: u32 = 0x00b4_c470;
pub const LIST_FAILED: u32 = 0x00b4_c3f0;
/// The list readers' frame: the unified path at `ebp - 0x104`, the
/// formatted one at `ebp - 0x204` (0x100 bytes each).
pub const LIST_FRAME: u32 = 0x214;
/// `syncDataChange`'s frame (`ebp - 0x104`, `ebp - 0x204`), and the list
/// readers' `ebp` 0x214 below its own.
pub const CHANGE_FRAME: u32 = 0x204;
pub const CHANGE_LIST_EBP: u32 = 0x214;
/// `SyncManager_readSyncFile(this; path)` (thiscall, `ret 4`),
/// `code_00686fe0` (the aligned free, cdecl), `dbgPrintf`,
/// `fiosUnifyFilename`, `crt_memset_thunk`, `AsyncLoadManager_sGetInstance`
/// (cdecl); `MSVCR110` `sprintf`.
pub const READ_SYNC_FILE: u32 = 0x006f_acf0;
pub const ALIGNED_FREE: u32 = 0x0068_6fe0;
pub const DBG_PRINTF: u32 = 0x0062_f500;
pub const FIOS_UNIFY: u32 = 0x0067_9820;
pub const MEMSET: u32 = 0x0094_964c;
pub const ASYNC_INSTANCE: u32 = 0x0069_1b40;
pub const SPRINTF_IAT: u32 = 0x00b0_c3dc;

/// `g_FmodCommandHandlers` (70 cdecl handlers `(fmod, args)`) and the
/// command ring (150 records of 20 bytes: the kind, then its arguments).
pub const FMOD_HANDLERS: u32 = 0x00c3_a368;
pub const FMOD_RING: u32 = 0x00ce_8458;
/// `KERNEL32` `SetEvent` (stdcall).
pub const SET_EVENT_IAT: u32 = 0x00b0_c0ec;

/// `operator_new` (cdecl); `FmodMusic_FmodMusic`, `0x0070a1a0` and
/// `FmodSfx_ctor_0070bfc0` (thiscall constructors).
pub const OPERATOR_NEW: u32 = 0x0063_0310;
pub const MUSIC_CTOR: u32 = 0x0070_8630;
pub const SOUND_CTOR: u32 = 0x0070_a1a0;
pub const SFX_CTOR: u32 = 0x0070_bfc0;

/// The originals over game memory.
pub mod bridge {
    use super::{
        ALIGNED_FREE, ASYNC_INSTANCE, CHANGE_LIST_EBP, DBG_PRINTF, ENTRY_FMT, FILE_LIST_FMT,
        FIOS_UNIFY, FMOD_HANDLERS, FMOD_RING, LIST_FAILED, MEMSET, MUSIC_CTOR, OPERATOR_NEW,
        READ_SYNC_FILE, SET_EVENT_IAT, SFX_CTOR, SOUND_CTOR, SPRINTF_IAT, VOICE_LIST_FMT,
    };
    use ffx_graphic::host::{Conv, Host};

    fn import(h: &mut impl Host, slot: u32, conv: Conv, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, conv, args)
    }

    /// Clears a 0x100-byte name buffer as the originals do (first byte,
    /// then `memset` of the other 0xff).
    fn clear_name(h: &mut impl Host, at: u32) {
        h.write(at, 0u8);
        h.call_sym(MEMSET, Conv::Cdecl, &[at.wrapping_add(1), 0, 0xff]);
    }

    /// Reads a list (`fmt` with the platform name), unified into the
    /// frame; returns the file's data, or `None` after the failure message.
    fn read_list(h: &mut impl Host, ebp: u32, this: u32, fmt: u32) -> Option<u32> {
        let unified = ebp.wrapping_sub(0x104);
        let formatted = ebp.wrapping_sub(0x204);
        clear_name(h, unified);
        clear_name(h, formatted);
        let platform: u32 = h.read(this);
        let fmt = h.rebase(fmt);
        import(h, SPRINTF_IAT, Conv::Cdecl, &[formatted, fmt, platform]);
        h.call_sym(FIOS_UNIFY, Conv::Cdecl, &[formatted, unified, 0x100]);
        let data = h.call_sym(READ_SYNC_FILE, Conv::Thiscall(this), &[unified]);
        if data == 0 {
            let msg = h.rebase(LIST_FAILED);
            h.call_sym(DBG_PRINTF, Conv::Cdecl, &[msg]);
            return None;
        }
        Some(data)
    }

    /// Whether a list file starts with `"SL\0\0"`.
    fn is_list(h: &impl Host, data: u32) -> bool {
        h.read::<u8>(data) == b'S'
            && h.read::<u8>(data.wrapping_add(1)) == b'L'
            && h.read::<u8>(data.wrapping_add(2)) == 0
            && h.read::<u8>(data.wrapping_add(3)) == 0
    }

    /// Appends `values` to the columns `cols` at entry `*count`.
    fn add_entry(h: &mut impl Host, this: u32, count: u32, cols: &[u32], values: &[u32]) {
        for (&col, &v) in cols.iter().zip(values) {
            let k: u32 = h.read(this.wrapping_add(count));
            let base: u32 = h.read(this.wrapping_add(col));
            h.write(base.wrapping_add(k.wrapping_mul(4)), v);
        }
        let k: u32 = h.read(this.wrapping_add(count));
        h.write(this.wrapping_add(count), k.wrapping_add(1));
    }

    /// `SyncManager::initSyncFileList(this)` (`0x006faa30`, thiscall;
    /// `ebp` the original's frame pointer, the cookie left out) -> 0: the
    /// file list read; its records of five words whose second is the
    /// current event add the first, third, fourth and fifth to the file
    /// columns; the data freed.
    pub fn init_sync_file_list(h: &mut impl Host, ebp: u32, this: u32) -> u32 {
        let Some(data) = read_list(h, ebp, this, FILE_LIST_FMT) else {
            return 0;
        };
        if is_list(h, data) {
            let mut n: u32 = h.read(data.wrapping_add(4));
            let mut p = data.wrapping_add(4);
            while n != 0 {
                let id: u32 = h.read(p.wrapping_add(4));
                let a: u32 = h.read(p.wrapping_add(8));
                let b: u32 = h.read(p.wrapping_add(0xc));
                let c: u32 = h.read(p.wrapping_add(0x10));
                let d: u32 = h.read(p.wrapping_add(0x14));
                p = p.wrapping_add(0x14);
                if id == h.read::<u32>(this.wrapping_add(0xc)) {
                    add_entry(h, this, 0x10, &[0x28, 0x30, 0x34, 0x38], &[a, b, c, d]);
                }
                n = n.wrapping_sub(1);
            }
        }
        h.call_sym(ALIGNED_FREE, Conv::Cdecl, &[data]);
        0
    }

    /// `SyncManager::initSyncVoiceList(this)` (`0x006faba0`): as
    /// [`init_sync_file_list`] for the voice list: records of four words
    /// whose second is the current event add the first, third and fourth
    /// to the voice columns.
    pub fn init_sync_voice_list(h: &mut impl Host, ebp: u32, this: u32) -> u32 {
        let Some(data) = read_list(h, ebp, this, VOICE_LIST_FMT) else {
            return 0;
        };
        if is_list(h, data) {
            let mut n: u32 = h.read(data.wrapping_add(4));
            let mut p = data.wrapping_add(4);
            while n != 0 {
                let a: u32 = h.read(p.wrapping_add(4));
                let id: u32 = h.read(p.wrapping_add(8));
                let b: u32 = h.read(p.wrapping_add(0xc));
                let c: u32 = h.read(p.wrapping_add(0x10));
                p = p.wrapping_add(0x10);
                if id == h.read::<u32>(this.wrapping_add(0xc)) {
                    add_entry(h, this, 0x14, &[0x24, 0x2c, 0x3c], &[a, b, c]);
                }
                n = n.wrapping_sub(1);
            }
        }
        h.call_sym(ALIGNED_FREE, Conv::Cdecl, &[data]);
        0
    }

    /// `SyncManager::releaseSyncData(this)` (`0x006faee0`) -> 0: the
    /// counts cleared, the six columns cleared (0x200 bytes each), the
    /// loaded files freed (aligned free) and their table cleared.
    pub fn release_sync_data(h: &mut impl Host, this: u32) -> u32 {
        h.write(this.wrapping_add(0x10), 0u32);
        h.write(this.wrapping_add(0x14), 0u32);
        for col in [0x28u32, 0x2c, 0x30, 0x34, 0x38, 0x3c] {
            let p: u32 = h.read(this.wrapping_add(col));
            h.call_sym(MEMSET, Conv::Cdecl, &[p, 0, 0x200]);
        }
        for off in (0..0x200u32).step_by(4) {
            let files: u32 = h.read(this.wrapping_add(4));
            let f: u32 = h.read(files.wrapping_add(off));
            if f != 0 {
                h.call_sym(ALIGNED_FREE, Conv::Cdecl, &[f]);
                let files: u32 = h.read(this.wrapping_add(4));
                h.write(files.wrapping_add(off), 0u32);
            }
        }
        let files: u32 = h.read(this.wrapping_add(4));
        h.call_sym(MEMSET, Conv::Cdecl, &[files, 0, 0x200]);
        0
    }

    /// `SyncManager::syncDataChange(this; _)` (`0x006faf90`, thiscall,
    /// `ret 4`; `ebp` the original's frame pointer, the list readers'
    /// frames 0x214 below it) -> 0: when enabled, both lists read, then
    /// each file entry's sync file loaded (its name formatted from the
    /// platform, the event and the four columns) into the file table.
    pub fn sync_data_change(h: &mut impl Host, ebp: u32, this: u32) -> u32 {
        if h.read::<u32>(this.wrapping_add(0x1c)) == 0 {
            return 0;
        }
        let inner = ebp.wrapping_sub(CHANGE_LIST_EBP);
        init_sync_file_list(h, inner, this);
        init_sync_voice_list(h, inner, this);
        let name = ebp.wrapping_sub(0x104);
        let unified = ebp.wrapping_sub(0x204);
        let mut i = 0u32;
        while (i as i32) < (h.read::<u32>(this.wrapping_add(0x10)) as i32) {
            clear_name(h, unified);
            clear_name(h, name);
            let mut v = [0u32; 4];
            for (k, off) in [0x28u32, 0x30, 0x34, 0x38].into_iter().enumerate() {
                let base: u32 = h.read(this.wrapping_add(off));
                v[k] = h.read(base.wrapping_add(i.wrapping_mul(4)));
            }
            let platform: u32 = h.read(this);
            let event: u32 = h.read(this.wrapping_add(0xc));
            let fmt = h.rebase(ENTRY_FMT);
            import(
                h,
                SPRINTF_IAT,
                Conv::Cdecl,
                &[name, fmt, platform, event, v[0], v[1], v[2], v[3]],
            );
            h.call_sym(FIOS_UNIFY, Conv::Cdecl, &[name, unified, 0x100]);
            let f = h.call_sym(READ_SYNC_FILE, Conv::Thiscall(this), &[unified]);
            i = i.wrapping_add(1);
            let files: u32 = h.read(this.wrapping_add(4));
            h.write(files.wrapping_add(i.wrapping_mul(4)).wrapping_sub(4), f);
        }
        0
    }

    /// `SyncManager_syncDataChangeAsyncTask(args)` (`0x006fb130`, cdecl;
    /// `ebp` for [`sync_data_change`]'s frame) -> 0:
    /// `AsyncLoadManager_sGetInstance()`, `syncDataChange(args[0];
    /// args[1])`, done (`+0x20`) = 1.
    pub fn sync_data_change_task(h: &mut impl Host, ebp: u32, args: u32) -> u32 {
        h.call_sym(ASYNC_INSTANCE, Conv::Cdecl, &[]);
        let this: u32 = h.read(args);
        let r = sync_data_change(h, ebp, this);
        h.write(this.wrapping_add(0x20), 1u32);
        r
    }

    /// Runs one FMOD command: the handler for `kind` with the manager's
    /// FMOD object and the argument block.
    fn run_command(h: &mut impl Host, this: u32, kind: u32, args: u32) -> u32 {
        let table = h.rebase(FMOD_HANDLERS);
        let f: u32 = h.read(table.wrapping_add(kind.wrapping_mul(4)));
        let fmod: u32 = h.read(this.wrapping_add(8));
        h.call(f, Conv::Cdecl, &[fmod, args])
    }

    /// The pending synchronous command (`+0x30`, its arguments at
    /// `+0x34`): run once, its result at `+0x48`, marked done (`+0x44`),
    /// its event (`+0x1c`) set.
    fn run_pending(h: &mut impl Host, this: u32) {
        if h.read::<u8>(this.wrapping_add(0x44)) != 0 {
            return;
        }
        let kind: u32 = h.read(this.wrapping_add(0x30));
        let r = run_command(h, this, kind, this.wrapping_add(0x34));
        h.write(this.wrapping_add(0x48), r);
        h.write(this.wrapping_add(0x44), 1u8);
        let event: u32 = h.read(this.wrapping_add(0x1c));
        import(h, SET_EVENT_IAT, Conv::Stdcall, &[event]);
    }

    /// `FmodManager_flushCommands_006fa7e0(this)` (thiscall): the pending
    /// synchronous command, then the ring from `+0x2c` to `+0x28` (150
    /// records), the pending one checked before each. Returns the
    /// original's `eax`: the read index when the ring was empty, else the
    /// last index step's quotient.
    pub fn fmod_flush(h: &mut impl Host, this: u32) -> u32 {
        run_pending(h, this);
        let read: u32 = h.read(this.wrapping_add(0x2c));
        if read == h.read::<u32>(this.wrapping_add(0x28)) {
            return read;
        }
        let ring = h.rebase(FMOD_RING);
        loop {
            run_pending(h, this);
            let i: u32 = h.read(this.wrapping_add(0x2c));
            let rec = ring.wrapping_add(i.wrapping_mul(20));
            let kind: u32 = h.read(rec);
            run_command(h, this, kind, rec.wrapping_add(4));
            let i: u32 = h.read(this.wrapping_add(0x2c));
            let next = (i as i32).wrapping_add(1);
            h.write(this.wrapping_add(0x2c), next.wrapping_rem(150) as u32);
            if next.wrapping_rem(150) as u32 == h.read::<u32>(this.wrapping_add(0x28)) {
                return next.wrapping_div(150) as u32;
            }
        }
    }

    /// `FmodObjects_ctor_00706490(this)` (thiscall; without its SEH
    /// frame) -> `this`: the four floats at `+0x18` 1.0, `+0..+8` 0, and
    /// the music (0x34 bytes), sound (0x90) and sfx (0xd4) objects
    /// constructed into `+0xc`, `+0x10`, `+0x14` (0 when `new` fails).
    pub fn fmod_objects_ctor(h: &mut impl Host, this: u32) -> u32 {
        for off in [0x18u32, 0x1c, 0x20, 0x24] {
            h.write(this.wrapping_add(off), 1.0f32.to_bits());
        }
        for off in [0u32, 4, 8] {
            h.write(this.wrapping_add(off), 0u32);
        }
        for (size, ctor, off) in [
            (0x34u32, MUSIC_CTOR, 0xcu32),
            (0x90, SOUND_CTOR, 0x10),
            (0xd4, SFX_CTOR, 0x14),
        ] {
            let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[size]);
            let v = if p == 0 {
                0
            } else {
                h.call_sym(ctor, Conv::Thiscall(p), &[])
            };
            h.write(this.wrapping_add(off), v);
        }
        this
    }
}
