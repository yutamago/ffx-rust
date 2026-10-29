//! M5 `system-b` (round 2): PC glue at the end of the image: the
//! occluder-geometry script accessors, the HDD read hooks
//! `PcReadFunc_set` installs, `pc_ReadWholeFile`, the window focus
//! handler, the job pushes of the character batches and `str_replace`.

/// `Phyre::PScripting::PScriptAccessors::PObjectAccessor<POccluderGeometryObject *>::Get`
/// (cdecl) and the accessors' shared helpers: `0x00436e80(L, value,
/// registry)`, `0x0094ac00(L, n)` and `lua_pushnil(L)` (cdecl),
/// `0x0043d6a0(this; a, flag)` (thiscall, `ret 8`) on the registry
/// `0x01a85990`; `Phyre_PhyrePrintf(level, fmt)` (cdecl) with the
/// message `0x00b0e998`.
pub const OCCLUDER_GET: u32 = 0x00a4_12b0;
pub const PUSH_OBJECT: u32 = 0x0043_6e80;
pub const LUA_94AC00: u32 = 0x0094_ac00;
pub const LUA_PUSHNIL: u32 = 0x0094_b730;
pub const REGISTRY_SET: u32 = 0x0043_d6a0;
pub const SCRIPT_REGISTRY: u32 = 0x01a8_5990;
pub const PHYRE_PRINTF: u32 = 0x0043_52f0;
pub const NOT_SUPPORTED_MSG: u32 = 0x00b0_e998;

/// `CDfileReadSector_CD` and `cd_file_sector` (cdecl), and
/// `g_CdLib_FileBase`.
pub const CD_READ_SECTOR: u32 = 0x0076_bdd0;
pub const CD_FILE_SECTOR: u32 = 0x0076_d510;
pub const CD_FILE_BASE: u32 = 0x0231_0c60;

/// `pc_ReadWholeFile`'s callees (cdecl): `sceOpen`, `dbgPrintf`,
/// `movie_err`, `sceLseek`, `user_malloc`, `FlushCache`, `sceRead`,
/// `sceClose`; its strings (the two "can't open" messages, the source
/// file and the assertion).
pub const SCE_OPEN: u32 = 0x0062_fa30;
pub const DBG_PRINTF: u32 = 0x0062_f500;
pub const MOVIE_ERR: u32 = 0x0076_efa0;
pub const SCE_LSEEK: u32 = 0x0062_f8e0;
pub const USER_MALLOC: u32 = 0x0068_7020;
pub const FLUSH_CACHE: u32 = 0x0062_f4c0;
pub const SCE_READ: u32 = 0x0062_fc00;
pub const SCE_CLOSE: u32 = 0x0062_f610;
pub const MSG_NO_NAME: u32 = 0x00b7_ae20;
pub const MSG_CANT_OPEN: u32 = 0x00b7_ae3c;
pub const SRC_FILE: u32 = 0x00b7_ad20;
pub const MSG_ASSERT: u32 = 0x00b7_ae54;

/// The window focus handler's callees: `g_FreeCamera`, `g_FFEscMenu`,
/// `operator_new`, `FreeCamera_FreeCamera` (thiscall),
/// `Sg_SetPauseAllSounds`, `PhyFMVPlayerManager_GetInstance` (cdecl),
/// `PhyFMVPlayerManager_pauseMovie` (thiscall).
pub const FREE_CAMERA: u32 = 0x00cc_c830;
pub const ESC_MENU: u32 = 0x00cc_c870;
pub const OPERATOR_NEW: u32 = 0x0063_0310;
pub const FREE_CAMERA_CTOR: u32 = 0x0067_fe70;
pub const PAUSE_ALL_SOUNDS: u32 = 0x0082_1cc0;
pub const FMV_MANAGER: u32 = 0x006d_73a0;
pub const FMV_PAUSE: u32 = 0x006d_a4b0;
/// The handler object `0x00c85ec0` (its byte at `+4`) and
/// `WindowEventHandler_insert_00625c70` (cdecl).
pub const FOCUS_HANDLER: u32 = 0x00c8_5ec0;
pub const HANDLER_INSERT: u32 = 0x0062_5c70;

/// The job scheduler `*0x00cdede4`, the jobs `Ch_Cloth_PerCharacter` and
/// `Ch_CalcMiddle_Batch`, `KERNEL32` `WaitForSingleObject` and
/// `ReleaseSemaphore` (stdcall).
pub const SCHEDULER: u32 = 0x00cd_ede4;
pub const CLOTH_JOB: u32 = 0x0082_e390;
pub const CALC_MIDDLE_JOB: u32 = 0x0083_3530;
/// Where the two originals hold their job: the `push imm32` operands at
/// `0x00a44b52` and `0x00a44ba5`. The harness points them at seeded
/// wrappers (`harness.rs`, `patch_seeded_jobs`), so the hook side reads
/// the job from there, as the original does.
pub const CLOTH_JOB_PUSH: u32 = 0x00a4_4b52;
pub const CALC_MIDDLE_JOB_PUSH: u32 = 0x00a4_4ba5;
pub const WAIT_FOR_SINGLE_OBJECT_IAT: u32 = 0x00b0_c0f8;
pub const RELEASE_SEMAPHORE_IAT: u32 = 0x00b0_c0f4;
/// `0x00a44a50` and `0x0071e720`: the two- and three-argument job
/// pushes (thiscall).
pub const PUSH_JOB2: u32 = 0x00a4_4a50;
pub const PUSH_JOB3: u32 = 0x0071_e720;

/// `str_replace`'s callees: `user_malloc`, `user_realloc`,
/// `user_free_thunk` (cdecl), `MSVCR110` `strstr` and `memmove`.
pub const USER_REALLOC: u32 = 0x0068_7060;
pub const USER_FREE: u32 = 0x0068_7010;
pub const STRSTR_IAT: u32 = 0x00b0_c378;
pub const MEMMOVE_IAT: u32 = 0x00b0_c34c;

/// The originals over game memory.
pub mod bridge {
    use super::{
        CD_FILE_BASE, CD_FILE_SECTOR, CD_READ_SECTOR, DBG_PRINTF, ESC_MENU, FLUSH_CACHE,
        FMV_MANAGER, FMV_PAUSE, FOCUS_HANDLER, FREE_CAMERA, FREE_CAMERA_CTOR, HANDLER_INSERT,
        LUA_94AC00, LUA_PUSHNIL, MEMMOVE_IAT, MOVIE_ERR, MSG_ASSERT, MSG_CANT_OPEN, MSG_NO_NAME,
        NOT_SUPPORTED_MSG, OCCLUDER_GET, OPERATOR_NEW, PAUSE_ALL_SOUNDS, PHYRE_PRINTF, PUSH_OBJECT,
        REGISTRY_SET, RELEASE_SEMAPHORE_IAT, SCE_CLOSE, SCE_LSEEK, SCE_OPEN, SCE_READ, SCHEDULER,
        SCRIPT_REGISTRY, SRC_FILE, STRSTR_IAT, USER_FREE, USER_MALLOC, USER_REALLOC,
        WAIT_FOR_SINGLE_OBJECT_IAT,
    };
    use ffx_graphic::host::{Conv, Host};

    fn import(h: &mut impl Host, slot: u32, conv: Conv, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, conv, args)
    }

    // ---- the occluder accessors (vtable methods; `this` unused) ----

    /// `0x00a424b0(value, out)` (`ret 8`): `*out` = `Get(value)`; returns
    /// it.
    pub fn occluder_get(h: &mut impl Host, value: u32, out: u32) -> u32 {
        let r = h.call_sym(OCCLUDER_GET, Conv::Cdecl, &[value]);
        h.write(out, r);
        r
    }

    /// `0x00a424e0(a, b)` (`ret 8`): `PhyrePrintf(2, msg)`; its `eax`.
    pub fn not_supported(h: &mut impl Host) -> u32 {
        let msg = h.rebase(NOT_SUPPORTED_MSG);
        h.call_sym(PHYRE_PRINTF, Conv::Cdecl, &[2, msg])
    }

    /// `0x00a42700(L, slot)` (`ret 8`): the object `*slot` pushed through
    /// the registry, or for none `0x0094ac00(L, 1)` and `lua_pushnil(L)`.
    /// Returns the last callee's `eax`.
    pub fn push_slot(h: &mut impl Host, l: u32, slot: u32) -> u32 {
        let v: u32 = h.read(slot);
        if v != 0 {
            let reg = h.rebase(SCRIPT_REGISTRY);
            return h.call_sym(PUSH_OBJECT, Conv::Cdecl, &[l, v, reg]);
        }
        h.call_sym(LUA_94AC00, Conv::Cdecl, &[l, 1]);
        h.call_sym(LUA_PUSHNIL, Conv::Cdecl, &[l])
    }

    /// `0x00a42740` / `0x00a42760(L, value)` (`ret 8`): `value` pushed
    /// through the registry; its `eax`.
    pub fn push_value(h: &mut impl Host, l: u32, v: u32) -> u32 {
        let reg = h.rebase(SCRIPT_REGISTRY);
        h.call_sym(PUSH_OBJECT, Conv::Cdecl, &[l, v, reg])
    }

    /// `0x00a427e0(a, flag)` (`ret 8`): `registry.0x0043d6a0(a, flag's low
    /// byte != 0)`; `al` = 1 over the callee's `eax`.
    pub fn registry_set(h: &mut impl Host, a: u32, flag: u32) -> u32 {
        let reg = h.rebase(SCRIPT_REGISTRY);
        let r = h.call_sym(
            REGISTRY_SET,
            Conv::Thiscall(reg),
            &[a, u32::from(flag & 0xff != 0)],
        );
        (r & !0xff) | 1
    }

    /// `0x00a42810(this; a, b)` (`ret 8`): the function at `this + 0x94`
    /// called with `(a, b)` (cdecl); its `eax`.
    pub fn call_field(h: &mut impl Host, this: u32, a: u32, b: u32) -> u32 {
        let f: u32 = h.read(this.wrapping_add(0x94));
        h.call(f, Conv::Cdecl, &[a, b])
    }

    // ---- the HDD read hooks ----

    /// `0x00a42870(sector, buffer, size, callback, opts)`: a tail jump to
    /// `CDfileReadSector_CD`.
    pub fn read_sector(
        h: &mut impl Host,
        sector: u32,
        buffer: u32,
        size: u32,
        callback: u32,
        opts: u32,
    ) -> u32 {
        h.call_sym(
            CD_READ_SECTOR,
            Conv::Cdecl,
            &[sector, buffer, size, callback, opts],
        )
    }

    /// `0x00a42880(file)`: the module's file base added into the argument
    /// slot (`slot`), then a tail jump to `cd_file_sector`.
    pub fn file_sector(h: &mut impl Host, file: u32, slot: u32) -> u32 {
        let base: u32 = h.read(h.rebase(CD_FILE_BASE));
        let n = base.wrapping_add(file);
        h.write(slot, n);
        h.call_sym(CD_FILE_SECTOR, Conv::Cdecl, &[n])
    }

    /// `pc_ReadWholeFile_00a42a00(name, size_out)` (cdecl): `sceOpen(name,
    /// 1)`; when that fails, a message (with the name unless it is empty)
    /// and the assertion through `movie_err`, then on with file 0. The
    /// size from `sceLseek(f, 0, 2)`, back to 0, a buffer of the size
    /// rounded up to whole words (signed: a negative size rounds toward
    /// zero), the whole file read, closed, the cache flushed around it;
    /// the size into `size_out` when set. Returns the buffer.
    pub fn read_whole_file(h: &mut impl Host, name: u32, size_out: u32) -> u32 {
        let f = h.call_sym(SCE_OPEN, Conv::Cdecl, &[name, 1]);
        if f == 0 {
            let first: u8 = h.read(name);
            if first == 0 {
                let msg = h.rebase(MSG_NO_NAME);
                h.call_sym(DBG_PRINTF, Conv::Cdecl, &[msg]);
            } else {
                let msg = h.rebase(MSG_CANT_OPEN);
                h.call_sym(DBG_PRINTF, Conv::Cdecl, &[msg, name]);
            }
            let (msg, src) = (h.rebase(MSG_ASSERT), h.rebase(SRC_FILE));
            h.call_sym(MOVIE_ERR, Conv::Cdecl, &[msg, src, 0x227]);
        }
        let size = h.call_sym(SCE_LSEEK, Conv::Cdecl, &[f, 0, 2]);
        h.call_sym(SCE_LSEEK, Conv::Cdecl, &[f, 0, 0]);
        let words = (size.cast_signed().wrapping_add(3) / 4).wrapping_mul(4);
        let buf = h.call_sym(USER_MALLOC, Conv::Cdecl, &[words.cast_unsigned()]);
        h.call_sym(FLUSH_CACHE, Conv::Cdecl, &[0]);
        h.call_sym(SCE_READ, Conv::Cdecl, &[f, buf, size]);
        h.call_sym(SCE_CLOSE, Conv::Cdecl, &[f]);
        h.call_sym(FLUSH_CACHE, Conv::Cdecl, &[0]);
        if size_out != 0 {
            h.write(size_out, size);
        }
        buf
    }

    // ---- the window focus handler (`CWindowEventHandler_vftable`) ----

    /// `g_FreeCamera`, created on first use (`new(0x340)`, then its
    /// constructor; 0 when the allocation failed).
    fn free_camera(h: &mut impl Host) -> u32 {
        let cell = h.rebase(FREE_CAMERA);
        let cam: u32 = h.read(cell);
        if cam != 0 {
            return cam;
        }
        let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x340]);
        let cam = if p == 0 {
            0
        } else {
            h.call_sym(FREE_CAMERA_CTOR, Conv::Thiscall(p), &[])
        };
        h.write(cell, cam);
        cam
    }

    /// Whether the game reacts to the focus: the free camera neither on
    /// (`+0x338`, `+0x33c`) nor in mode 2, and the escape menu (read
    /// before the camera is made) absent or closed (`+0x70`). `Err` holds
    /// the camera, which the original leaves in `eax` when it doesn't.
    fn focus_applies(h: &mut impl Host) -> Result<(), u32> {
        let esc: u32 = h.read(h.rebase(ESC_MENU));
        let cam = free_camera(h);
        let on: u8 = h.read(cam.wrapping_add(0x338));
        let held: u8 = h.read(cam.wrapping_add(0x33c));
        let mode: u32 = h.read(cam);
        if on != 0 || held != 0 || mode == 2 {
            return Err(cam);
        }
        if esc == 0 || h.read::<u8>(esc.wrapping_add(0x70)) == 0 {
            Ok(())
        } else {
            Err(cam)
        }
    }

    /// `0x00a44890(this)` (slot 0, the focus lost): when it applies, all
    /// sounds paused, `+4` = 0, and a playing movie (manager `+0x6fc`
    /// clear) paused with `+4` = 1. Returns the original's `eax`: the
    /// camera when it doesn't apply, else the manager (0 or one already
    /// paused) or `pauseMovie`'s `eax`.
    pub fn focus_lost(h: &mut impl Host, this: u32) -> u32 {
        if let Err(cam) = focus_applies(h) {
            return cam;
        }
        h.call_sym(PAUSE_ALL_SOUNDS, Conv::Cdecl, &[1]);
        h.write(this.wrapping_add(4), 0u8);
        let m = h.call_sym(FMV_MANAGER, Conv::Cdecl, &[]);
        if m == 0 || h.read::<u8>(m.wrapping_add(0x6fc)) != 0 {
            return m;
        }
        let r = h.call_sym(FMV_PAUSE, Conv::Thiscall(m), &[]);
        h.write(this.wrapping_add(4), 1u8);
        r
    }

    /// `0x00a44960(this)` (slot 1, the focus back): when it applies, the
    /// movie this handler paused (`+4`) toggled back and the sounds
    /// resumed (their `eax`); else the camera in `eax`.
    pub fn focus_gained(h: &mut impl Host, this: u32) -> u32 {
        if let Err(cam) = focus_applies(h) {
            return cam;
        }
        let m = h.call_sym(FMV_MANAGER, Conv::Cdecl, &[]);
        if m != 0 && h.read::<u8>(this.wrapping_add(4)) != 0 {
            h.call_sym(FMV_PAUSE, Conv::Thiscall(m), &[]);
        }
        h.call_sym(PAUSE_ALL_SOUNDS, Conv::Cdecl, &[0])
    }

    /// `WindowEventHandler_Register_00a44a30()`: the focus handler's
    /// byte cleared and the handler inserted; the insert's `eax`.
    pub fn register_focus_handler(h: &mut impl Host) -> u32 {
        let handler = h.rebase(FOCUS_HANDLER);
        h.write(handler.wrapping_add(4), 0u8);
        h.call_sym(HANDLER_INSERT, Conv::Cdecl, &[handler])
    }

    // ---- job pushes ----

    /// `i % 8` of the ring index after `i + 7` (signed, as the
    /// original's `and 0x80000007` and sign fix).
    fn ring_prev(i: u32) -> u32 {
        (i.wrapping_add(7).cast_signed() % 8).cast_unsigned()
    }

    /// A job with `args` (two or three words) pushed on queue `queue` of
    /// the scheduler `this` (`this + queue * 0x1e8`, written into the
    /// caller's `queue` slot `queue_slot` and read back from it): the
    /// semaphore `+0x234` and the lock `+0x1fc` taken, the free ring's
    /// index `+0x1f0` stepped back, its count `+0x1f8` down, the entry at
    /// `+0x1d0 + 4 i` taken, the lock released; the entry gets the
    /// function (at `+8` for two words, `+0xc` for three), the first word
    /// at `+0x14`, the rest from `+0x20` with pointers to them from
    /// `+0x18`, and the count at `+0x10`; then under the run lock `+0x22c`
    /// it goes on the run ring (`+0x224`, `+0x200 + 4 j`, `+0x228` up) and
    /// the run semaphore `+0x230` is released. Returns that release's
    /// `eax`.
    pub fn push_job(
        h: &mut impl Host,
        this: u32,
        func: u32,
        args: &[u32],
        queue: u32,
        queue_slot: u32,
    ) -> u32 {
        let q = this.wrapping_add(queue.wrapping_mul(0x1e8));
        let at = |off: u32| q.wrapping_add(off);
        let sem: u32 = h.read(at(0x234));
        h.write(queue_slot, q);
        import(
            h,
            WAIT_FOR_SINGLE_OBJECT_IAT,
            Conv::Stdcall,
            &[sem, u32::MAX],
        );
        let lock: u32 = h.read(at(0x1fc));
        import(
            h,
            WAIT_FOR_SINGLE_OBJECT_IAT,
            Conv::Stdcall,
            &[lock, u32::MAX],
        );
        let free = ring_prev(h.read(at(0x1f0)));
        let count: u32 = h.read(at(0x1f8));
        h.write(at(0x1f8), count.wrapping_sub(1));
        h.write(at(0x1f0), free);
        let entry: u32 = h.read(at(0x1d0).wrapping_add(free.wrapping_mul(4)));
        let lock: u32 = h.read(at(0x1fc));
        import(h, RELEASE_SEMAPHORE_IAT, Conv::Stdcall, &[lock, 1, 0]);
        let e = |off: u32| entry.wrapping_add(off);
        let n = u32::try_from(args.len()).unwrap_or(0);
        if let Some((&first, rest)) = args.split_first() {
            h.write(e(if n == 2 { 8 } else { 0xc }), func);
            h.write(e(0x14), first);
            for (off, &w) in (0..).step_by(4).zip(rest) {
                h.write(e(0x20 + off), w);
                h.write(e(0x18 + off), e(0x20 + off));
            }
        }
        h.write(e(0x10), n);
        let q: u32 = h.read(queue_slot);
        let at = |off: u32| q.wrapping_add(off);
        let run_lock: u32 = h.read(at(0x22c));
        import(
            h,
            WAIT_FOR_SINGLE_OBJECT_IAT,
            Conv::Stdcall,
            &[run_lock, u32::MAX],
        );
        let run = ring_prev(h.read(at(0x224)));
        h.write(at(0x224), run);
        h.write(at(0x200).wrapping_add(run.wrapping_mul(4)), entry);
        let queued: u32 = h.read(at(0x228));
        h.write(at(0x228), queued.wrapping_add(1));
        let run_lock: u32 = h.read(at(0x22c));
        import(h, RELEASE_SEMAPHORE_IAT, Conv::Stdcall, &[run_lock, 1, 0]);
        let run_sem: u32 = h.read(at(0x230));
        import(h, RELEASE_SEMAPHORE_IAT, Conv::Stdcall, &[run_sem, 1, 0])
    }

    /// `JobSchedule_Ch_Cloth_PerCharacter(chars, count)` (cdecl): `job`
    /// (`Ch_Cloth_PerCharacter`, the original's `push imm32` at
    /// [`super::CLOTH_JOB_PUSH`]) pushed with both on queue 0 of the
    /// scheduler (`0x00a44a50`; `slot` stands for its queue argument).
    pub fn schedule_cloth(h: &mut impl Host, chars: u32, count: u32, job: u32, slot: u32) -> u32 {
        let s: u32 = h.read(h.rebase(SCHEDULER));
        push_job(h, s, job, &[chars, count], 0, slot)
    }

    /// `JobSchedule_Ch_CalcMiddle_Batch(chars, pass, count)` (cdecl): `job`
    /// (`Ch_CalcMiddle_Batch`, the original's `push imm32` at
    /// [`super::CALC_MIDDLE_JOB_PUSH`]) pushed with the three on queue 0
    /// (`0x0071e720`; `slot` stands for its queue argument).
    pub fn schedule_calc_middle(h: &mut impl Host, args: [u32; 3], job: u32, slot: u32) -> u32 {
        let s: u32 = h.read(h.rebase(SCHEDULER));
        push_job(h, s, job, &args, 0, slot)
    }

    // ---- str_replace ----

    fn strlen(h: &impl Host, s: u32) -> u32 {
        let mut n = 0u32;
        while h.read::<u8>(s.wrapping_add(n)) != 0 {
            n = n.wrapping_add(1);
        }
        n
    }

    /// `str_replace(s, what, with)` (cdecl): a new string (`user_malloc`)
    /// with every `what` in `s` (found with `strstr`, left to right, not
    /// overlapping) replaced by `with`. The buffer grows by the length
    /// difference per match (`user_realloc`); when that fails the buffer
    /// is freed and 0 returned. 0 when the first allocation fails.
    pub fn str_replace(h: &mut impl Host, s: u32, what: u32, with: u32) -> u32 {
        let mut total = strlen(h, s).wrapping_add(1);
        let what_len = strlen(h, what);
        let with_len = strlen(h, with);
        let mut buf = h.call_sym(USER_MALLOC, Conv::Cdecl, &[total]);
        if buf == 0 {
            return 0;
        }
        let mut out = buf;
        let mut src = s;
        let mut hit = import(h, STRSTR_IAT, Conv::Cdecl, &[src, what]);
        while hit != 0 {
            let prefix = hit.wrapping_sub(src);
            total = total.wrapping_add(with_len.wrapping_sub(what_len));
            let grown = h.call_sym(USER_REALLOC, Conv::Cdecl, &[buf, total]);
            if grown == 0 {
                h.call_sym(USER_FREE, Conv::Cdecl, &[buf]);
                return 0;
            }
            out = out.wrapping_add(grown.wrapping_sub(buf));
            buf = grown;
            import(h, MEMMOVE_IAT, Conv::Cdecl, &[out, src, prefix]);
            out = out.wrapping_add(prefix);
            import(h, MEMMOVE_IAT, Conv::Cdecl, &[out, with, with_len]);
            out = out.wrapping_add(with_len);
            src = src.wrapping_add(prefix.wrapping_add(what_len));
            hit = import(h, STRSTR_IAT, Conv::Cdecl, &[src, what]);
        }
        loop {
            let c: u8 = h.read(src);
            h.write(out, c);
            src = src.wrapping_add(1);
            out = out.wrapping_add(1);
            if c == 0 {
                return buf;
            }
        }
    }
}
