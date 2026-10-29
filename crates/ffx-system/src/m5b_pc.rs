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

/// The magic-file preloads: `g_MagicFilePreloadOn`, the 12-slot
/// `g_MagicFilePreloadTable` (magic, ?, file), the per-slot counts
/// `0x0193fd44` and handles `0x0193fd78`, the report format;
/// `MSVCR110` `__iob_func`, `fprintf`, `printf` (cdecl);
/// `PhyreFIOS_UnloadMagicfilePrx(count, &handle)` (cdecl).
pub const PRELOAD_ON: u32 = 0x0193_fd40;
pub const PRELOAD_TABLE: u32 = 0x00c6_4c10;
pub const PRELOAD_COUNTS: u32 = 0x0193_fd44;
pub const PRELOAD_HANDLES: u32 = 0x0193_fd78;
pub const PRELOAD_MSG: u32 = 0x00b7_0858;
pub const IOB_FUNC_IAT: u32 = 0x00b0_c350;
pub const FPRINTF_IAT: u32 = 0x00b0_c38c;
pub const PRINTF_IAT: u32 = 0x00b0_c380;
pub const UNLOAD_PRX: u32 = 0x0067_94f0;
/// `onMagicDLLLoaded`: `g_MagicFile_Handle`, its message, `KERNEL32`
/// `GetLastError` and `GetProcAddress` (stdcall), and the two exports
/// (name, cell).
pub const MAGIC_HANDLE: u32 = 0x00c6_4cac;
pub const MAGIC_LOAD_FAILED: u32 = 0x00b7_03e4;
pub const GET_LAST_ERROR_IAT: u32 = 0x00b0_c030;
pub const GET_PROC_ADDRESS_IAT: u32 = 0x00b0_c118;
pub const MAGIC_EXPORTS: [(u32, u32); 2] = [(0x00b7_0418, 0x022f_b6a4), (0x00b7_0430, 0x022f_b6a0)];
/// The vector cells `0x00c0a004` and `0x00c8f980`; `VFX_Copy16To(src,
/// dst)` and `VFX_Copy16From(dst, src)` (cdecl).
pub const VEC_C0A004: u32 = 0x00c0_a004;
pub const VEC_C8F980: u32 = 0x00c8_f980;
pub const COPY16_TO: u32 = 0x0071_0e70;
pub const COPY16_FROM: u32 = 0x0071_0e30;

/// Round 3: `RadialBlur`'s two draws (`0x007030e0`, `0x00703480`): its
/// parameter setters (thiscall), `PRenderSurface`'s constructor and
/// (empty) destructor, `PRenderer`'s `setColorTarget`,
/// `setDepthStencilTarget`, `beginScene`, `renderMeshInstance` (4 words),
/// `endScene` (thiscall), the two shader cells `0x00cdeba8`/`0x00cdebc8`
/// put at the renderer's `+0x48`, `g_Vu0FixedRound` (0.5).
pub const SET_PARAM58: u32 = 0x0070_3780;
pub const SET_TEXTURE_PARAM: u32 = 0x0070_3610;
pub const SET_PARAM5C: u32 = 0x0070_3820;
pub const SURFACE_CTOR: u32 = 0x005b_35c0;
pub const SURFACE_DTOR: u32 = 0x005b_3640;
pub const SET_COLOR_TARGET: u32 = 0x005b_b130;
pub const SET_DEPTH_TARGET: u32 = 0x005b_b5b0;
pub const BEGIN_SCENE: u32 = 0x005b_5e30;
pub const RENDER_MESH_INSTANCE: u32 = 0x005b_9a90;
pub const END_SCENE: u32 = 0x005b_6950;
pub const BLUR_SHADER: [u32; 2] = [0x00cd_eba8, 0x00cd_ebc8];
pub const HALF: u32 = 0x00b9_2220;
/// The sphere grid's item check (`0x00a44d30`, `0x00a44ea0`):
/// `g_AbmapRunning`, `g_Abmap`, `g_AbmapPad` (its node list: count `i16
/// +0x1a1e`, 12-byte entries from `+0x1a44`), `AbmapFillUseItemMenu`,
/// `MsGetSaveItemNum`, `MsGetRomItem`, `MsGetExcelData` and its table
/// `0x01a860e4`, `AbmapForEachNode`, `AbmapForNodeAndLinked` and their
/// callback `AbmapItemNodeEffect`.
pub const ABMAP_RUNNING: u32 = 0x01a8_5f70;
pub const ABMAP: u32 = 0x0230_5834;
pub const ABMAP_PAD: u32 = 0x01a8_6108;
pub const FILL_USE_ITEM_MENU: u32 = 0x00a4_5fd0;
pub const SAVE_ITEM_NUM: u32 = 0x0079_04b0;
pub const ROM_ITEM: u32 = 0x0079_09f0;
pub const EXCEL_DATA: u32 = 0x007a_b870;
pub const ABILITY_TABLE: u32 = 0x01a8_60e4;
pub const FOR_EACH_NODE: u32 = 0x00a5_9710;
pub const FOR_NODE_AND_LINKED: u32 = 0x00a5_9760;
pub const ITEM_NODE_EFFECT: u32 = 0x00a4_9440;

/// The game's `main` (`0x00822490`): "debug"/"DEBUG", the flag
/// `0x0133c910`, `0x0083c200`, `0x00820310`, `0x00820860`.
pub const DEBUG_WORDS: [u32; 2] = [0x00b5_9148, 0x00b5_9150];
pub const DEBUG_FLAG: u32 = 0x0133_c910;
pub const MAIN_PREPARE: u32 = 0x0083_c200;
pub const MAIN_INIT: u32 = 0x0082_0310;
pub const MAIN_LOOP: u32 = 0x0082_0860;

/// `FUN_0092cdd0`'s: `TkGetNowEventJumpMapNo_0086c1f0`, `rcuiGetStageName`,
/// the CRT's `sprintf` (import slot) and the map-object path format,
/// `g_YnMapSystem`, `rcPrint` and its messages (the assert's file, line
/// 0xa9), `rcdtChunkNew`, `rcdtChunkLoadFromFile`, `rcbgLoadBin`,
/// `rcbgGuideSetModel`, `rcdtChunkDelete`.
pub const EVENT_JUMP_MAP: u32 = 0x0086_c1f0;
pub const STAGE_NAME: u32 = 0x0093_7940;
pub const SPRINTF_IAT: u32 = 0x00b0_c3dc;
pub const MAP_OBJECT_FMT: u32 = 0x00b6_7d38;
pub const YN_MAP_SYSTEM: u32 = 0x0193_bf80;
pub const RC_PRINT: u32 = 0x0092_7550;
pub const RC_ASSERT_MSG: [u32; 3] = [0x00b6_7a14, 0x00b6_7d90, 0x00b6_7d70];
pub const RC_MSG: [u32; 3] = [0x00b6_7db4, 0x00b6_7d18, 0x00b6_7dd4];
pub const CHUNK_NEW: u32 = 0x0092_b900;
pub const CHUNK_LOAD: u32 = 0x0092_b880;
pub const BG_LOAD_BIN: u32 = 0x0092_1d60;
pub const GUIDE_SET_MODEL: u32 = 0x0091_d590;
pub const CHUNK_DELETE: u32 = 0x0092_b7d0;

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
    // ---- magic files ----

    /// `MagicFile_clearAllPreloadedPrx(magic)`: when preloading is on,
    /// `MagicFile_ClearPreloadedPrx(magic, -1)`. Off, `eax` is left alone
    /// (`None`).
    pub fn clear_all_preloaded(h: &mut impl Host, magic: u32) -> Option<u32> {
        if h.read::<u32>(h.rebase(super::PRELOAD_ON)) == 0 {
            return None;
        }
        clear_preloaded(h, magic, u32::MAX)
    }

    /// `MagicFile_ClearPreloadedPrx_009dacc0(magic, file)`: when preloading
    /// is on, each of the 12 preload slots whose magic and file match
    /// (a negative argument matches any, signed) is unloaded: a held
    /// handle (count `0x0193fd44 + 4 i` > 0, signed) is reported on
    /// stdout (`fprintf(__iob_func() + 0x20, fmt, file, i, count,
    /// handle)`) and released (`PhyreFIOS_UnloadMagicfilePrx(count,
    /// &handle)`), then count and handle cleared. Returns `file` (`eax`
    /// holds it throughout); off, `eax` untouched (`None`).
    pub fn clear_preloaded(h: &mut impl Host, magic: u32, file: u32) -> Option<u32> {
        if h.read::<u32>(h.rebase(super::PRELOAD_ON)) == 0 {
            return None;
        }
        let table = h.rebase(super::PRELOAD_TABLE);
        let counts = h.rebase(super::PRELOAD_COUNTS);
        let handles = h.rebase(super::PRELOAD_HANDLES);
        for i in 0..12u32 {
            let slot = table.wrapping_add(12 * i);
            let magic_ok = magic.cast_signed() < 0 || magic == h.read::<u32>(slot);
            if !magic_ok {
                continue;
            }
            if file.cast_signed() >= 0 && file != h.read::<u32>(slot.wrapping_add(8)) {
                continue;
            }
            let count_at = counts.wrapping_add(4 * i);
            let handle_at = handles.wrapping_add(4 * i);
            let count: u32 = h.read(count_at);
            if count.cast_signed() > 0 {
                let handle: u32 = h.read(handle_at);
                let fmt = h.rebase(super::PRELOAD_MSG);
                let iob = import(h, super::IOB_FUNC_IAT, Conv::Cdecl, &[]);
                import(
                    h,
                    super::FPRINTF_IAT,
                    Conv::Cdecl,
                    &[iob.wrapping_add(0x20), fmt, file, i, count, handle],
                );
                let count: u32 = h.read(count_at);
                h.call_sym(super::UNLOAD_PRX, Conv::Cdecl, &[count, handle_at]);
            }
            h.write(count_at, 0u32);
            h.write(handle_at, 0u32);
        }
        Some(file)
    }

    /// `onMagicDLLLoaded(module)`: `g_MagicFile_Handle` = `module`; without
    /// one, the error printed (`printf(fmt, 0, GetLastError())`), the
    /// handle -1 and 0; else the two exports looked up
    /// (`GetProcAddress`) into `0x022fb6a4` and `0x022fb6a0`, and 1.
    pub fn on_magic_dll_loaded(h: &mut impl Host, module: u32) -> u32 {
        let cell = h.rebase(super::MAGIC_HANDLE);
        h.write(cell, module);
        if module == 0 {
            let e = import(h, super::GET_LAST_ERROR_IAT, Conv::Stdcall, &[]);
            let fmt = h.rebase(super::MAGIC_LOAD_FAILED);
            import(h, super::PRINTF_IAT, Conv::Cdecl, &[fmt, module, e]);
            h.write(cell, u32::MAX);
            return 0;
        }
        for (name, out) in super::MAGIC_EXPORTS {
            let name = h.rebase(name);
            let p = import(
                h,
                super::GET_PROC_ADDRESS_IAT,
                Conv::Stdcall,
                &[module, name],
            );
            let out = h.rebase(out);
            h.write(out, p);
        }
        1
    }

    // ---- the VFX vector cells ----

    /// `FUN_0092cc20(dst)`: `VFX_Copy16To(0x00c0a004, dst)`; its `eax`.
    pub fn copy_c0a004(h: &mut impl Host, dst: u32) -> u32 {
        let src = h.rebase(super::VEC_C0A004);
        h.call_sym(super::COPY16_TO, Conv::Cdecl, &[src, dst])
    }

    /// `FUN_0092cbf0(dst, src)`: `src` into the cell `0x00c8f980`
    /// (`VFX_Copy16From`), then the cell into `dst` (`VFX_Copy16To`); its
    /// `eax`.
    pub fn copy_via_c8f980(h: &mut impl Host, dst: u32, src: u32) -> u32 {
        let cell = h.rebase(super::VEC_C8F980);
        h.call_sym(super::COPY16_FROM, Conv::Cdecl, &[cell, src]);
        h.call_sym(super::COPY16_TO, Conv::Cdecl, &[cell, dst])
    }
    // ---- round 3: RadialBlur's draws, the sphere grid's item check ----

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// Writes `words` from `p` on.
    fn put_words(h: &mut impl Host, p: u32, words: &[u32]) {
        for (k, &w) in (0u32..).zip(words) {
            h.write(p.wrapping_add(4 * k), w);
        }
    }

    /// `1.0f32`'s bits.
    const ONE: u32 = 0x3f80_0000;

    /// The scene both blur draws render: the surface at `surface`
    /// (`PRenderSurface(surface; target, 0)`) set as the colour target of
    /// the renderer `[this + 0x1c]` (`depth`: no depth target), the scene
    /// begun (`begin`), and when that returns 0 the shader cell put at
    /// the renderer's `+0x48` and the blur mesh (`[this + 0x6c]`) drawn,
    /// the scene ended. The surface's (empty) destructor; returns the
    /// scene's result.
    fn blur_scene(
        h: &mut impl Host,
        this: u32,
        surface: u32,
        target: u32,
        depth: bool,
        begin: u32,
        shader: u32,
    ) -> u32 {
        h.call_sym(super::SURFACE_CTOR, Conv::Thiscall(surface), &[target, 0]);
        let renderer: u32 = h.read(this.wrapping_add(0x1c));
        h.call_sym(
            super::SET_COLOR_TARGET,
            Conv::Thiscall(renderer),
            &[surface, 0],
        );
        if depth {
            let renderer: u32 = h.read(this.wrapping_add(0x1c));
            h.call_sym(super::SET_DEPTH_TARGET, Conv::Thiscall(renderer), &[0]);
        }
        let renderer: u32 = h.read(this.wrapping_add(0x1c));
        let mut r = h.call_sym(super::BEGIN_SCENE, Conv::Thiscall(renderer), &[begin]);
        if r == 0 {
            let renderer: u32 = h.read(this.wrapping_add(0x1c));
            let shader = h.rebase(shader);
            h.write(renderer.wrapping_add(0x48), shader);
            let renderer: u32 = h.read(this.wrapping_add(0x1c));
            let mesh: u32 = h.read(this.wrapping_add(0x6c));
            let args = [
                mesh,
                h.read(mesh.wrapping_add(0x10)),
                0,
                h.read(renderer.wrapping_add(0x13c)),
            ];
            h.call_sym(super::RENDER_MESH_INSTANCE, Conv::Thiscall(renderer), &args);
            let renderer: u32 = h.read(this.wrapping_add(0x1c));
            r = h.call_sym(super::END_SCENE, Conv::Thiscall(renderer), &[1]);
        }
        h.call_sym(super::SURFACE_DTOR, Conv::Thiscall(surface), &[]);
        r
    }

    /// `FUN_007030e0(this; texture, _)` (thiscall, `ret 8`) with its frame
    /// at `ebp`: off (`byte this` 0) 0; else the blur's parameters
    /// (`+0x58` = (1, 1, 1, 1), the texture with `this + 0x30`, `+0x5c` =
    /// (1, 1, 0, 0); the floats passed by value also left at `ebp -
    /// 0x1c..`) and [`blur_scene`] into `[this + 0x68]` with no depth
    /// target, `beginScene(1)`, shader `0x00cdeba8`; the surface at `ebp -
    /// 0x2c`.
    pub fn blur_draw(h: &mut impl Host, ebp: u32, this: u32, texture: u32) -> u32 {
        if h.read::<u8>(this) == 0 {
            return 0;
        }
        let tmp = ebp.wrapping_sub(0x1c);
        put_words(h, tmp, &[ONE; 4]);
        h.call_sym(super::SET_PARAM58, Conv::Thiscall(this), &[ONE; 4]);
        let sampler = this.wrapping_add(0x30);
        h.call_sym(
            super::SET_TEXTURE_PARAM,
            Conv::Thiscall(this),
            &[texture, sampler],
        );
        put_words(h, tmp, &[ONE, ONE, 0, 0]);
        h.call_sym(super::SET_PARAM5C, Conv::Thiscall(this), &[ONE, ONE, 0, 0]);
        let target: u32 = h.read(this.wrapping_add(0x68));
        let surface = ebp.wrapping_sub(0x2c);
        blur_scene(h, this, surface, target, true, 1, super::BLUR_SHADER[0])
    }

    /// `FUN_00703480(this; target)` (thiscall, `ret 4`) with its frame at
    /// `ebp`: off 0; else `+0x58` from `this + 4..0x14`, the texture
    /// `[this + 0x64]` with `this + 0x30`, `+0x5c` = (1/sx, 1/sy, (1 -
    /// 1/sx) * 0.5, (1 - 1/sy) * 0.5) of the scales `sx` `+0x14`, `sy`
    /// `+0x18` (x87, each quotient recomputed in the register; the values
    /// passed also left at `ebp - 0x2c..`, the scales at `ebp - 0x1c..`),
    /// and [`blur_scene`] into `target`, `beginScene(0)`, shader
    /// `0x00cdebc8`; the surface at `ebp - 0x3c`.
    pub fn blur_draw_scaled(h: &mut impl Host, ebp: u32, this: u32, target: u32) -> u32 {
        use ffx_bridge::x87;
        if h.read::<u8>(this) == 0 {
            return 0;
        }
        let mut p58 = [0u32; 4];
        for (k, x) in (0u32..).zip(&mut p58) {
            *x = h.read(this.wrapping_add(4 + 4 * k));
        }
        h.call_sym(super::SET_PARAM58, Conv::Thiscall(this), &p58);
        let texture: u32 = h.read(this.wrapping_add(0x64));
        let sampler = this.wrapping_add(0x30);
        h.call_sym(
            super::SET_TEXTURE_PARAM,
            Conv::Thiscall(this),
            &[texture, sampler],
        );
        let sy = x87::store(x87::load(h.read(this.wrapping_add(0x18))));
        let sx = x87::store(x87::load(h.read(this.wrapping_add(0x14))));
        put_words(h, ebp.wrapping_sub(0x1c), &[sy, sx, sy, sx]);
        let (fx, fy) = (x87::load(sx), x87::load(sy));
        let half = f64::from_bits(h.read::<u64>(h.rebase(super::HALF)));
        let inv_x = x87::div(1.0, fx);
        let inv_y = x87::div(1.0, fy);
        let off_x = x87::mul(x87::sub(1.0, x87::div(1.0, fx)), half);
        let off_y = x87::mul(x87::sub(1.0, x87::div(1.0, fy)), half);
        let p5c = [inv_x, inv_y, off_x, off_y].map(x87::store);
        put_words(h, ebp.wrapping_sub(0x2c), &p5c);
        h.call_sym(super::SET_PARAM5C, Conv::Thiscall(this), &p5c);
        let surface = ebp.wrapping_sub(0x3c);
        blur_scene(h, this, surface, target, false, 0, super::BLUR_SHADER[1])
    }

    /// The sphere grid list's node count, `i16 pad + 0x1a1e`.
    fn node_count(h: &impl Host, pad: u32) -> i32 {
        i32::from(h.read::<i16>(pad.wrapping_add(0x1a1e)))
    }

    /// `FUN_00a44d30()` with its frame at `ebp`: for each node of the
    /// sphere grid's list (`g_AbmapPad`, count `i16 +0x1a1e`, 12-byte
    /// entries from `+0x1a44`), whether the item `u16 +4` is usable
    /// there: none in stock (`MsGetSaveItemNum`) marks it 1; else the
    /// item's ability (`MsGetExcelData(byte +0x5d of MsGetRomItem(item,
    /// ebp - 8), table, ebp - 8)`, none for 0xff) is tried by
    /// `AbmapItemNodeEffect` over every node (kind 0x20:
    /// `AbmapForEachNode`) or the current node and its links
    /// (`AbmapForNodeAndLinked(node)`), with the context at `ebp - 0x24`
    /// (the ability, the character `byte g_Abmap + 0x115bc`, its bit, a
    /// found flag); marked 0 when found, else 1. Returns the count read
    /// last (0 for none).
    pub fn abmap_item_usable(h: &mut impl Host, ebp: u32) -> u32 {
        let abmap: u32 = h.read(h.rebase(super::ABMAP));
        let chr = u32::from(h.read::<u8>(abmap.wrapping_add(0x115bc)));
        let node = u32::from(h.read::<u16>(abmap.wrapping_add(chr * 80 + 0x110cc)));
        h.write(ebp.wrapping_sub(0xc), node);
        let pad: u32 = h.read(h.rebase(super::ABMAP_PAD));
        h.write(ebp.wrapping_sub(0x10), pad);
        if node_count(h, pad) <= 0 {
            return 0;
        }
        let ctx = ebp.wrapping_sub(0x24);
        let out = ebp.wrapping_sub(8);
        let mut i = 0i32;
        let mut entry = pad.wrapping_add(0x1a44);
        loop {
            let item = u32::from(h.read::<u16>(entry.wrapping_add(4)));
            let abmap: u32 = h.read(h.rebase(super::ABMAP));
            let chr = u32::from(h.read::<u8>(abmap.wrapping_add(0x115bc)));
            h.write(ebp.wrapping_sub(4), chr);
            let mut usable = false;
            if cdecl(h, super::SAVE_ITEM_NUM, &[item]) != 0 {
                h.write(ctx.wrapping_add(4), chr);
                h.write(ctx.wrapping_add(0xc), 0u32);
                h.write(ctx.wrapping_add(8), 1u32.wrapping_shl(chr));
                let rom = cdecl(h, super::ROM_ITEM, &[item, out]);
                let ability = h.read::<u8>(rom.wrapping_add(0x5d));
                let data = if ability == 0xff {
                    0
                } else {
                    let table: u32 = h.read(h.rebase(super::ABILITY_TABLE));
                    cdecl(h, super::EXCEL_DATA, &[u32::from(ability), table, out])
                };
                h.write(ctx, data);
                if data != 0 {
                    let effect = h.rebase(super::ITEM_NODE_EFFECT);
                    if h.read::<u8>(data.wrapping_add(0xc)) == 0x20 {
                        cdecl(h, super::FOR_EACH_NODE, &[effect, ctx]);
                    } else {
                        let node: u32 = h.read(ebp.wrapping_sub(0xc));
                        cdecl(h, super::FOR_NODE_AND_LINKED, &[node, effect, ctx]);
                    }
                }
                usable = h.read::<u32>(ctx.wrapping_add(0xc)) != 0;
            }
            h.write(entry, u8::from(!usable));
            i += 1;
            entry = entry.wrapping_add(12);
            let n = node_count(h, pad);
            if i >= n {
                return n.cast_unsigned();
            }
        }
    }

    /// `FUN_00a44ea0()`: while the sphere grid runs (`g_AbmapRunning`),
    /// its node list emptied, `AbmapFillUseItemMenu(8, 3)` and the jump to
    /// [`abmap_item_usable`] (its `eax`). Else `eax` is left alone
    /// (`None`).
    pub fn abmap_refresh_items(h: &mut impl Host, ebp: u32) -> Option<u32> {
        if h.read::<u32>(h.rebase(super::ABMAP_RUNNING)) == 0 {
            return None;
        }
        let pad: u32 = h.read(h.rebase(super::ABMAP_PAD));
        h.write(pad.wrapping_add(0x1a1e), 0u16);
        cdecl(h, super::FILL_USE_ITEM_MENU, &[8, 3]);
        Some(abmap_item_usable(h, ebp))
    }
    /// `strcmp` as the inlined loop: equal or not.
    fn str_eq(h: &impl Host, first: u32, second: u32) -> bool {
        let mut at = 0u32;
        loop {
            let c: u8 = h.read(first.wrapping_add(at));
            if c != h.read::<u8>(second.wrapping_add(at)) {
                return false;
            }
            if c == 0 {
                return true;
            }
            at = at.wrapping_add(1);
        }
    }

    /// `FUN_00822490(argc, argv)` (the game's `main`): the debug flag
    /// `0x0133c910` = `argv[1]` is "debug" or "DEBUG" (with an argument),
    /// then `0x0083c200()`, `0x00820310(argc, argv)` and
    /// `0x00820860(argc, argv)`; returns 1.
    pub fn game_main(h: &mut impl Host, count: u32, words: u32) -> u32 {
        let debug = count.cast_signed() > 1 && {
            let arg: u32 = h.read(words.wrapping_add(4));
            let [lower, upper] = super::DEBUG_WORDS.map(|s| h.rebase(s));
            str_eq(h, lower, arg) || str_eq(h, upper, arg)
        };
        let at = h.rebase(super::DEBUG_FLAG);
        h.write(at, u8::from(debug));
        cdecl(h, super::MAIN_PREPARE, &[]);
        cdecl(h, super::MAIN_INIT, &[count, words]);
        cdecl(h, super::MAIN_LOOP, &[count, words]);
        1
    }
    /// `FUN_0092cdd0()` with its frame at `ebp` (a debug reload of the
    /// map's objects): the current map (`TkGetNowEventJumpMapNo(ebp -
    /// 0xc)`, four signed chars) when `rcuiGetStageName` knows it: the
    /// path `sprintf(ebp - 0x10c, fmt, c0, c1, c2, c3, ebp - 0xc)`, an
    /// assert print while `g_YnMapSystem + 0x14` holds a chunk, a chunk
    /// (`rcdtChunkNew`) loaded from it (`rcdtChunkLoadFromFile`): loaded,
    /// printed, `rcbgLoadBin([chunk])`, kept at `+0x14` and the guide
    /// model set (`rcbgGuideSetModel(map +4, [[map] + 8] + 0x1c)`); not
    /// loaded, deleted. Returns the last `rcPrint`'s `eax`, or 0 for an
    /// unknown map.
    pub fn reload_map_objects(h: &mut impl Host, ebp: u32) -> u32 {
        let id = ebp.wrapping_sub(0xc);
        let map = cdecl(h, super::EVENT_JUMP_MAP, &[id]);
        // the map word goes on with the first call's word still on the stack
        let known = cdecl(h, super::STAGE_NAME, &[map, id]);
        if known == 0 {
            return 0;
        }
        let path = ebp.wrapping_sub(0x10c);
        let fmt = h.rebase(super::MAP_OBJECT_FMT);
        let c =
            [0u32, 1, 2, 3].map(|k| i32::from(h.read::<i8>(id.wrapping_add(k))).cast_unsigned());
        let sprintf: u32 = h.read(h.rebase(super::SPRINTF_IAT));
        h.call(
            sprintf,
            Conv::Cdecl,
            &[path, fmt, c[0], c[1], c[2], c[3], id],
        );
        let system: u32 = h.read(h.rebase(super::YN_MAP_SYSTEM));
        if h.read::<u32>(system.wrapping_add(0x14)) != 0 {
            let [m0, m1, m2] = super::RC_ASSERT_MSG.map(|s| h.rebase(s));
            cdecl(h, super::RC_PRINT, &[m0, m1, m2, 0xa9]);
        }
        let [n0, n1, n2] = super::RC_MSG.map(|s| h.rebase(s));
        cdecl(h, super::RC_PRINT, &[n0]);
        let chunk = cdecl(h, super::CHUNK_NEW, &[]);
        if cdecl(h, super::CHUNK_LOAD, &[chunk, path]) != 0 {
            cdecl(h, super::RC_PRINT, &[n1, path]);
            let data: u32 = h.read(chunk);
            cdecl(h, super::BG_LOAD_BIN, &[data]);
            let system: u32 = h.read(h.rebase(super::YN_MAP_SYSTEM));
            h.write(system.wrapping_add(0x14), chunk);
            let system: u32 = h.read(h.rebase(super::YN_MAP_SYSTEM));
            let map: u32 = if system == 0 { 0 } else { h.read(system) };
            let guide: u32 = h.read(map.wrapping_add(8));
            let model: u32 = h.read(guide.wrapping_add(0x1c));
            let at: u32 = h.read(system.wrapping_add(4));
            cdecl(h, super::GUIDE_SET_MODEL, &[at, model]);
        } else if chunk != 0 {
            cdecl(h, super::CHUNK_DELETE, &[chunk]);
        }
        cdecl(h, super::RC_PRINT, &[n2])
    }
}
