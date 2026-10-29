//! M5 `main`: the PS2 task layer's PC stand-ins (`sa_task_*`, the task
//! groups' create/run/delete entries, the "not implemented" warnings),
//! the movie side's CD callbacks and state resets, the debug printf
//! helpers, the read-name table's first-use clear and the decode
//! dispatcher.
//!
//! The task groups: eight records at [`GROUPS`] (`0x0112a818`), filled by
//! `sa_task_init2`; each group's create, run and delete entry is a small
//! function with its group number, kept in the tables at `0x00c41924`,
//! `0x00c41944` and `0x00c41964` that `sa_task_init2` copies into the
//! record. The asm parts (`task_create_asm`, `task_init_asm`, the run
//! step `0x0077a4d0`) do nothing on the PC but print a warning.

/// The originals over game memory.
#[allow(
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::verbose_bit_mask
)] // registers, points; the `al` test as written
pub mod bridge {
    use ffx_graphic::host::{Conv, Host};

    /// `dbgPrintf(fmt, ...)` (cdecl, varargs).
    pub const DBG_PRINTF: u32 = 0x0062_f500;
    /// The "Virtuos Warning: ... not implemented" lines of `task_wait`,
    /// `task_init_asm`, `sa_camera_set`, `code_0076f170` and
    /// `task_create_asm` (`.rdata`).
    pub const MSG_TASK_WAIT: u32 = 0x00b5_3fa8;
    pub const MSG_TASK_INIT_ASM: u32 = 0x00b5_3ea4;
    pub const MSG_CAMERA_SET: u32 = 0x00b5_2880;
    pub const MSG_0076F170: u32 = 0x00b5_2db8;
    pub const MSG_TASK_CREATE_ASM: u32 = 0x00b5_3ee8;

    /// `dbgPrintf(msg)`: the warning-only stand-ins (`task_wait`,
    /// `task_init_asm`, `sa_camera_set`, `code_0076f170`); its `eax`.
    pub fn warn(h: &mut impl Host, msg: u32) -> u32 {
        let s = h.rebase(msg);
        h.call_sym(DBG_PRINTF, Conv::Cdecl, &[s])
    }

    /// `task_create_asm()` (`0x0077a4b0`, cdecl; its callers pass three
    /// words it never reads): the warning, then 0.
    pub fn task_create_asm(h: &mut impl Host) -> u32 {
        warn(h, MSG_TASK_CREATE_ASM);
        0
    }

    /// `task_create_asm` (cdecl).
    pub const TASK_CREATE_ASM: u32 = 0x0077_a4b0;
    /// `task_init_asm` (cdecl).
    pub const TASK_INIT_ASM: u32 = 0x0077_a4c0;
    /// The run step's asm stand-in (`0x0077a4d0`, a bare `ret`).
    pub const TASK_RUN_ASM: u32 = 0x0077_a4d0;

    /// `FUN_0077a320()` (cdecl): a frame, then a tail jump to
    /// `task_create_asm`; its `eax` (0).
    pub fn task_create_asm_thunk(h: &mut impl Host) -> u32 {
        h.call_sym(TASK_CREATE_ASM, Conv::Cdecl, &[])
    }

    /// The eight task group records' pointers.
    pub const GROUPS: u32 = 0x0112_a818;
    /// The current task's stack words: priority/id, stack end, stack top,
    /// stack base; and the running record (`FUN_0077a420`).
    pub const CUR_ID: u32 = 0x0231_0c00;
    pub const CUR_STACK_END: u32 = 0x0231_0c04;
    pub const CUR_STACK_TOP: u32 = 0x0231_0c08;
    pub const CUR_STACK_BASE: u32 = 0x0231_0c0c;
    pub const CUR_RECORD: u32 = 0x0231_0c10;
    /// The groups' create, run and delete entries (eight each).
    pub const CREATE_TABLE: u32 = 0x00c4_1924;
    pub const RUN_TABLE: u32 = 0x00c4_1944;
    pub const DELETE_TABLE: u32 = 0x00c4_1964;
    /// `movie_err(msg, file, line)` (cdecl) and its arguments: the
    /// `i!=TASK_GRUPE_MAX` assertion.
    pub const MOVIE_ERR: u32 = 0x0076_efa0;
    pub const MSG_GROUPS_FULL: u32 = 0x00b5_3e54;
    pub const MSG_TASK_FILE: u32 = 0x00b5_3e34;

    /// `sa_task_init2(id, record, flag)` (`0x0077a1d0`, cdecl): fills the
    /// record (its stack at `record + 0x28` rounded up to 16, `id *
    /// 0x1310` bytes, then 0x370 more; the group's entries; `flag` at
    /// `+0x24`) in the first free group slot (the assertion when all eight
    /// are taken, and the ninth word written anyway), runs
    /// `task_init_asm` with the current-task words set from it and puts
    /// [`CUR_STACK_TOP`] and [`CUR_ID`] back. Returns `record`.
    pub fn task_init2(h: &mut impl Host, id: u32, record: u32, flag: u32) -> u32 {
        let size = id.wrapping_mul(0x1310);
        let top_at = h.rebase(CUR_STACK_TOP);
        let id_at = h.rebase(CUR_ID);
        let saved_top: u32 = h.read(top_at);
        let saved_id: u32 = h.read(id_at);
        let mut base = record.wrapping_add(0x28);
        if base & 0xf != 0 {
            base = base.wrapping_add(0x10 - (base & 0xf));
        }
        h.write(record + 0xc, base);
        let groups = h.rebase(GROUPS);
        let mut slot = 0;
        while slot < 8 && h.read::<u32>(groups + 4 * slot) != 0 {
            slot += 1;
        }
        if slot == 8 {
            let msg = h.rebase(MSG_GROUPS_FULL);
            let file = h.rebase(MSG_TASK_FILE);
            h.call_sym(MOVIE_ERR, Conv::Cdecl, &[msg, file, 0x103]);
        }
        h.write(groups + 4 * slot, record);
        h.write(record + 0x20, slot);
        h.write(record + 0xc, base);
        h.write(h.rebase(CUR_STACK_BASE), base);
        let top = base.wrapping_add(size);
        h.write(record + 0x10, top);
        h.write(top_at, top);
        let end = base.wrapping_add(0x370).wrapping_add(size);
        h.write(record + 0x14, end);
        h.write(h.rebase(CUR_STACK_END), end);
        h.write(record + 0x18, id);
        h.write(id_at, id);
        let create: u32 = h.read(h.rebase(CREATE_TABLE) + 4 * slot);
        h.write(record + 8, create);
        let run: u32 = h.read(h.rebase(RUN_TABLE) + 4 * slot);
        h.write(record, run);
        let delete: u32 = h.read(h.rebase(DELETE_TABLE) + 4 * slot);
        h.write(record + 4, delete);
        h.write(record + 0x1c, 0u32);
        h.write(record + 0x24, flag);
        h.call_sym(TASK_INIT_ASM, Conv::Cdecl, &[]);
        h.write(top_at, saved_top);
        h.write(id_at, saved_id);
        record
    }

    /// `sa_task_buff_size_get(id)`, `user_malloc(size)`,
    /// `sa_task_init2` (cdecl).
    pub const TASK_BUFF_SIZE: u32 = 0x0077_a180;
    pub const USER_MALLOC: u32 = 0x0068_7020;
    pub const TASK_INIT2: u32 = 0x0077_a1d0;

    /// `sa_task_init(id)` (`0x0077a1a0`, cdecl): a record of the task's
    /// size from the user heap, then `sa_task_init2(id, it, 1)`'s `eax`.
    pub fn task_init(h: &mut impl Host, id: u32) -> u32 {
        let size = h.call_sym(TASK_BUFF_SIZE, Conv::Cdecl, &[id]);
        let p = h.call_sym(USER_MALLOC, Conv::Cdecl, &[size]);
        h.call_sym(TASK_INIT2, Conv::Cdecl, &[id, p, 1])
    }

    /// `task_create_in_group(group, a, b, c)` (`0x0077a390`, cdecl): the
    /// current-task words from the group's record, `task_create_asm(a, b,
    /// c)`, [`CUR_ID`] and [`CUR_STACK_TOP`] put back; its `eax`.
    pub fn task_create_in_group(h: &mut impl Host, group: u32, a: u32, b: u32, c: u32) -> u32 {
        with_group(h, group, |h| {
            h.call_sym(TASK_CREATE_ASM, Conv::Cdecl, &[a, b, c])
        })
        .1
    }

    /// Sets the current-task words from `group`'s record, runs `f`, puts
    /// [`CUR_ID`] and [`CUR_STACK_TOP`] back. Returns the record and
    /// `f`'s result.
    fn with_group<H: Host>(h: &mut H, group: u32, f: impl FnOnce(&mut H) -> u32) -> (u32, u32) {
        let record: u32 = h.read(h.rebase(GROUPS).wrapping_add(group.wrapping_mul(4)));
        let top_at = h.rebase(CUR_STACK_TOP);
        let id_at = h.rebase(CUR_ID);
        let saved_top: u32 = h.read(top_at);
        let saved_id: u32 = h.read(id_at);
        let base: u32 = h.read(record + 0xc);
        h.write(h.rebase(CUR_STACK_BASE), base);
        let top: u32 = h.read(record + 0x10);
        h.write(top_at, top);
        let end: u32 = h.read(record + 0x14);
        h.write(h.rebase(CUR_STACK_END), end);
        let r = f(h);
        h.write(id_at, saved_id);
        h.write(top_at, saved_top);
        (record, r)
    }

    /// `task_create_in_group` (cdecl).
    pub const TASK_CREATE_IN_GROUP: u32 = 0x0077_a390;

    /// The group create entries (`FUN_00779f80` .. `FUN_0077a040`,
    /// groups 0..6, cdecl `(a, b, c)`): `task_create_in_group(group, a,
    /// b, c)`'s `eax`.
    pub fn task_create_group(h: &mut impl Host, group: u32, a: u32, b: u32, c: u32) -> u32 {
        h.call_sym(TASK_CREATE_IN_GROUP, Conv::Cdecl, &[group, a, b, c])
    }

    /// `FUN_0077a420(group)` (cdecl): the run step: the group's record
    /// becomes [`CUR_RECORD`], the current-task words come from it (its
    /// id too), its run count `+0x1c` goes up, the asm step runs (a bare
    /// `ret`), [`CUR_ID`] and [`CUR_STACK_TOP`] are put back. Returns the
    /// record's id (`eax` from before the step).
    pub fn task_run_in_group(h: &mut impl Host, group: u32) -> u32 {
        let record: u32 = h.read(h.rebase(GROUPS).wrapping_add(group.wrapping_mul(4)));
        h.write(h.rebase(CUR_RECORD), record);
        let mut id = 0;
        with_group(h, group, |h| {
            id = h.read(record + 0x18);
            h.write(h.rebase(CUR_ID), id);
            let n: u32 = h.read(record + 0x1c);
            h.write(record + 0x1c, n.wrapping_add(1));
            h.call_sym(TASK_RUN_ASM, Conv::Cdecl, &[]);
            0
        });
        id
    }

    /// `FUN_0077a420` (cdecl).
    pub const TASK_RUN_IN_GROUP: u32 = 0x0077_a420;

    /// The group run entries (`code_0077a080` .. `code_0077a0f0`, groups
    /// 0..7, cdecl): `FUN_0077a420(group)`'s `eax`.
    pub fn task_run_group(h: &mut impl Host, group: u32) -> u32 {
        h.call_sym(TASK_RUN_IN_GROUP, Conv::Cdecl, &[group])
    }

    /// `user_free_thunk(p)` (cdecl).
    pub const USER_FREE: u32 = 0x0068_7010;

    /// `FUN_0077a3f0(group)` (cdecl): the group's record freed when it
    /// came from the heap (`+0x24`), its slot cleared. Returns the record,
    /// or `user_free`'s `eax`.
    pub fn task_delete_in_group(h: &mut impl Host, group: u32) -> u32 {
        let slot = h.rebase(GROUPS).wrapping_add(group.wrapping_mul(4));
        let record: u32 = h.read(slot);
        let mut r = record;
        if h.read::<u32>(record + 0x24) != 0 {
            r = h.call_sym(USER_FREE, Conv::Cdecl, &[record]);
        }
        h.write(slot, 0u32);
        r
    }

    /// `FUN_0077a3f0` (cdecl).
    pub const TASK_DELETE_IN_GROUP: u32 = 0x0077_a3f0;

    /// The group delete entries (`code_0077a100` .. `code_0077a170`,
    /// groups 0..7, cdecl): `FUN_0077a3f0(group)`'s `eax`.
    pub fn task_delete_group(h: &mut impl Host, group: u32) -> u32 {
        h.call_sym(TASK_DELETE_IN_GROUP, Conv::Cdecl, &[group])
    }

    /// The read-name table: its first-use byte and the two 16-word
    /// halves (`g_ReadChgName_From`, `g_ReadChgName_To`).
    pub const CHGNAME_READY: u32 = 0x01a8_5ba0;
    pub const CHGNAME_FROM: u32 = 0x01a8_5bb0;
    pub const CHGNAME_TO: u32 = 0x01a8_5bf0;

    /// `FUN_00a43130()` (cdecl): on first use, the ready byte set and both
    /// halves of the read-name table cleared. `eax` is 0 then (the
    /// original leaves it alone otherwise; no caller reads it).
    pub fn chgname_init(h: &mut impl Host) {
        let ready = h.rebase(CHGNAME_READY);
        if h.read::<u8>(ready) != 0 {
            return;
        }
        h.write(ready, 1u8);
        for k in 0..16 {
            h.write(h.rebase(CHGNAME_TO) + 4 * k, 0u32);
        }
        for k in 0..16 {
            h.write(h.rebase(CHGNAME_FROM) + 4 * k, 0u32);
        }
    }

    /// `printf2(fmt, args)` (cdecl, a `va_list`).
    pub const PRINTF2: u32 = 0x0076_dd10;

    /// `FUN_0076de70(fmt, ...)` (cdecl, varargs): `printf2(fmt, &args)`
    /// with the address of its own variable arguments (`slots + 4`, the
    /// word after `fmt`); its `eax`.
    pub fn printf(h: &mut impl Host, fmt: u32, slots: u32) -> u32 {
        h.call_sym(PRINTF2, Conv::Cdecl, &[fmt, slots + 4])
    }

    /// `sa_printf_set(f)`, `FlushCache(mode)` (cdecl).
    pub const PRINTF_SET: u32 = 0x0077_a4a0;
    pub const FLUSH_CACHE: u32 = 0x0062_f4c0;
    /// The printf hook `printf_cut` installs (code in `.text`).
    pub const PRINTF_NONE: u32 = 0x0076_deb0;

    /// `printf_cut()` (`0x0076de90`, cdecl): [`PRINTF_NONE`] as the
    /// printf hook, `FlushCache(0)` and `FlushCache(2)`; the last's
    /// `eax`.
    pub fn printf_cut(h: &mut impl Host) -> u32 {
        let f = h.rebase(PRINTF_NONE);
        h.call_sym(PRINTF_SET, Conv::Cdecl, &[f]);
        h.call_sym(FLUSH_CACHE, Conv::Cdecl, &[0]);
        h.call_sym(FLUSH_CACHE, Conv::Cdecl, &[2])
    }

    /// `TkRegistBGThreadFunc(f)` (cdecl) and the read thread's function.
    pub const REGIST_BG_THREAD: u32 = 0x0088_8740;
    pub const READ_THREAD: u32 = 0x0076_e5f0;

    /// `read_thread_init()` (`0x0076e630`, cdecl): registers the read
    /// thread's function; its `eax`.
    pub fn read_thread_init(h: &mut impl Host) -> u32 {
        let f = h.rebase(READ_THREAD);
        h.call_sym(REGIST_BG_THREAD, Conv::Cdecl, &[f])
    }

    /// `sa_read_callback(req)` (`0x0076e640`, cdecl): the request's
    /// callback `+0x20` with the request (cdecl), then its status word
    /// `+0x31` cleared; the callback's `eax`.
    pub fn read_callback(h: &mut impl Host, req: u32) -> u32 {
        let f: u32 = h.read(req + 0x20);
        let r = h.call(f, Conv::Cdecl, &[req]);
        h.write(req + 0x31, 0u16);
        r
    }

    /// `sa_task_start`, `sceGsSetDefDrawEnv2`, `FFXIsDebug`, `MsEnvInit`
    /// (cdecl, none), `CdEnvInit(flag)` (cdecl).
    pub const TASK_START: u32 = 0x0077_a2c0;
    pub const SET_DEF_DRAW_ENV2: u32 = 0x0088_7c60;
    pub const CD_ENV_INIT: u32 = 0x0076_c560;
    pub const IS_DEBUG: u32 = 0x0088_7c50;
    pub const MS_ENV_INIT: u32 = 0x0078_2770;
    /// `sa_cd_callback_init`'s banner and its two closing lines.
    pub const MSG_CD_BANNER: u32 = 0x00b5_2d64;
    pub const MSG_CD_DONE: u32 = 0x00b5_2d80;
    pub const MSG_CD_DONE_NO_ENV: u32 = 0x00b5_2d9c;

    /// `sa_cd_callback_init(flag; slots)` (`0x0076f100`, cdecl): the task
    /// start and the banner; with the draw environment (`== 1`)
    /// `CdEnvInit(flag != 0)` and `MsEnvInit` in debug builds, else
    /// `CdEnvInit(1)`. The closing line goes into its own argument slot
    /// before the tail jump to `dbgPrintf`, whose `eax` it returns.
    pub fn cd_callback_init(h: &mut impl Host, flag: u32, slots: u32) -> u32 {
        h.call_sym(TASK_START, Conv::Cdecl, &[]);
        let s = h.rebase(MSG_CD_BANNER);
        h.call_sym(DBG_PRINTF, Conv::Cdecl, &[s]);
        let msg = if h.call_sym(SET_DEF_DRAW_ENV2, Conv::Cdecl, &[]) == 1 {
            h.call_sym(CD_ENV_INIT, Conv::Cdecl, &[u32::from(flag != 0)]);
            if h.call_sym(IS_DEBUG, Conv::Cdecl, &[]) != 0 {
                h.call_sym(MS_ENV_INIT, Conv::Cdecl, &[]);
            }
            MSG_CD_DONE
        } else {
            h.call_sym(CD_ENV_INIT, Conv::Cdecl, &[1]);
            MSG_CD_DONE_NO_ENV
        };
        let msg = h.rebase(msg);
        h.write(slots, msg);
        h.call_sym(DBG_PRINTF, Conv::Cdecl, &[msg])
    }

    /// The movie state words `code_0076f3b0` clears (in its order) and
    /// its three halfwords; its warning.
    pub const MOVIE_RESET_WORDS: [u32; 12] = [
        0x0112_a03c,
        0x0112_a044,
        0x0112_a030,
        0x0112_a0a4,
        0x0112_a01c,
        0x0112_a080,
        0x0112_a020,
        0x0112_a024,
        0x0112_a028,
        0x0112_a02c,
        0x0112_a09c,
        0x0112_a034,
    ];
    pub const MOVIE_RESET_HALVES: [u32; 3] = [0x0112_a08c, 0x0112_a090, 0x0112_a094];
    pub const MSG_MOVIE_RESET: u32 = 0x00b5_30cc;

    /// `code_0076f3b0()` (cdecl; a table entry): the warning, then the
    /// movie state cleared. Returns 0.
    pub fn movie_reset(h: &mut impl Host) -> u32 {
        warn(h, MSG_MOVIE_RESET);
        for w in MOVIE_RESET_WORDS {
            h.write(h.rebase(w), 0u32);
        }
        for w in MOVIE_RESET_HALVES {
            h.write(h.rebase(w), 0u16);
        }
        0
    }

    /// `TkFree(p)` (cdecl).
    pub const TK_FREE: u32 = 0x0088_d520;
    /// The two buffers `code_00770340` frees, its handler object and the
    /// words it clears (in its order); its warning.
    pub const MOVIE_BUF_A: u32 = 0x0112_a040;
    pub const MOVIE_BUF_B: u32 = 0x0112_a03c;
    pub const MOVIE_HANDLER: u32 = 0x0112_a014;
    pub const MOVIE_HANDLER_DONE: u32 = 0x0112_a018;
    pub const MOVIE_END_WORDS: [u32; 17] = [
        0x0112_a040,
        0x0112_a03c,
        0x0112_a0a4,
        0x0112_a044,
        0x0112_a01c,
        0x0112_a080,
        0x0112_a020,
        0x0112_a024,
        0x0112_a02c,
        0x0112_a034,
        0x0112_a030,
        0x0112_a010,
        0x0112_a048,
        0x0112_a050,
        0x0112_a04c,
        0x0112_a064,
        0x0112_a068,
    ];
    pub const MSG_MOVIE_END: u32 = 0x00b5_3354;

    /// `code_00770340()` (cdecl; a table entry): the warning, both
    /// buffers freed, the movie state cleared; the handler's slot 1
    /// (`[handler + 4]`, no arguments) when there is one, and the handler
    /// cleared; the done word cleared. Returns the handler call's `eax`,
    /// or 0.
    pub fn movie_end(h: &mut impl Host) -> u32 {
        warn(h, MSG_MOVIE_END);
        let a: u32 = h.read(h.rebase(MOVIE_BUF_A));
        h.call_sym(TK_FREE, Conv::Cdecl, &[a]);
        let b: u32 = h.read(h.rebase(MOVIE_BUF_B));
        h.call_sym(TK_FREE, Conv::Cdecl, &[b]);
        let handler: u32 = h.read(h.rebase(MOVIE_HANDLER));
        for w in MOVIE_END_WORDS {
            h.write(h.rebase(w), 0u32);
        }
        let mut r = 0;
        if handler != 0 {
            let f: u32 = h.read(handler + 4);
            r = h.call(f, Conv::Cdecl, &[]);
            h.write(h.rebase(MOVIE_HANDLER), 0u32);
        }
        h.write(h.rebase(MOVIE_HANDLER_DONE), 0u32);
        r
    }

    /// `MsGetSaveBattleCount`, `TkGetEncountCounterEventStart`,
    /// `MsGetGIL` (cdecl).
    pub const SAVE_BATTLE_COUNT: u32 = 0x0078_4f50;
    pub const ENCOUNT_EVENT_START: u32 = 0x0088_d5c0;
    pub const GET_GIL: u32 = 0x0078_4e80;

    /// `code_007722a0()` (cdecl; a native's helper): the battle count
    /// minus the count at the event's start.
    pub fn battles_since_event(h: &mut impl Host) -> u32 {
        let n = h.call_sym(SAVE_BATTLE_COUNT, Conv::Cdecl, &[]);
        n.wrapping_sub(h.call_sym(ENCOUNT_EVENT_START, Conv::Cdecl, &[]))
    }

    /// `code_00772310()`: `jmp MsGetSaveBattleCount`.
    pub fn battle_count_thunk(h: &mut impl Host) -> u32 {
        h.call_sym(SAVE_BATTLE_COUNT, Conv::Cdecl, &[])
    }

    /// `code_00772550()`: `jmp MsGetGIL`.
    pub fn gil_thunk(h: &mut impl Host) -> u32 {
        h.call_sym(GET_GIL, Conv::Cdecl, &[])
    }

    /// The word `FUN_00820520` returns.
    pub const WORD_012FBBF4: u32 = 0x012f_bbf4;

    /// `FUN_00820520()` (cdecl): the word at `0x012fbbf4`.
    pub fn get_012fbbf4(h: &mut impl Host) -> u32 {
        h.read(h.rebase(WORD_012FBBF4))
    }

    /// The decoders by a stream's first byte (`0x00c85ea4`).
    pub const DECODERS: u32 = 0x00c8_5ea4;

    /// `sa_decode(out, src; slots)` (`0x00a43de0`, cdecl): the decoder for
    /// `src`'s first byte, called `(src, out, &src)` with the address of
    /// its own `src` slot (`slots + 4`, which the decoder advances).
    /// Returns that slot afterwards.
    pub fn decode(h: &mut impl Host, out: u32, src: u32, slots: u32) -> u32 {
        let kind: u8 = h.read(src);
        let f: u32 = h.read(h.rebase(DECODERS) + 4 * u32::from(kind));
        h.call(f, Conv::Cdecl, &[src, out, slots + 4]);
        h.read(slots + 4)
    }

    /// `MSVCR110!vsprintf`'s import slot (cdecl).
    pub const VSPRINTF_IAT: u32 = 0x00b0_c430;
    /// The ring of 256 print buffers (0x404 bytes each, the text at
    /// `+4`) and its index.
    pub const RING_INDEX: u32 = 0x010e_7878;
    pub const RING: u32 = 0x010e_7880;
    /// `FUN_0076b860`'s frame below `ebp`: the sixteen argument copies
    /// at `ebp - 0x44` and the cookie.
    pub const RING_FRAME: u32 = 0x44;

    /// `FUN_0076b860(fmt, a[16])` (cdecl, with its frame at `ebp`): the
    /// sixteen words after `fmt` copied to `ebp - 0x44`, the next ring
    /// buffer taken (the index wraps at 256) and `vsprintf(buffer + 4,
    /// fmt, copy)`; its `eax`.
    pub fn ring_printf(h: &mut impl Host, ebp: u32, fmt: u32, a: [u32; 16]) -> u32 {
        let copy = ebp - RING_FRAME;
        for (k, w) in (0u32..).zip(a) {
            h.write(copy + 4 * k, w);
        }
        let at = h.rebase(RING_INDEX);
        let i: u32 = h.read(at);
        let buf = i.wrapping_mul(0x404).wrapping_add(h.rebase(RING));
        let next = i.wrapping_add(1);
        h.write(at, if next.cast_signed() >= 0x100 { 0 } else { next });
        let f: u32 = h.read(h.rebase(VSPRINTF_IAT));
        h.call(f, Conv::Cdecl, &[buf.wrapping_add(4), fmt, copy])
    }
}
