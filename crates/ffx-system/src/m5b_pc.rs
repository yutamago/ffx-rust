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

/// `JobWorker`'s thread loop (`0x00703a20`): `JobSchedule::JobScheduler`
/// (`0x00cdede4`, read once), `AsyncJobQueue_pop_006db8a0`,
/// `ThreadCall_run_00703910`, `AsyncJobQueue_push_006db800` (thiscall).
/// `FileReadMain`'s read task record `0x01128adc`.
pub const JOB_SCHEDULER: u32 = 0x00cd_ede4;
pub const QUEUE_POP: u32 = 0x006d_b8a0;
pub const THREAD_CALL_RUN: u32 = 0x0070_3910;
pub const QUEUE_PUSH: u32 = 0x006d_b800;
pub const READ_TASK: u32 = 0x0112_8adc;

/// `updateVFXRenderData_AkazaScrFlt`'s: `graphicVFXGetRenderDataByInstance`,
/// `graphicDynGeoMemManagerGetBufferForSegment`, `graphicVFXSetVisible`,
/// the grid step (f64 `0x00b501a0`, 1/32) and 2.0 (`0x00b92270`).
pub const RENDER_DATA_BY_INSTANCE: u32 = 0x0064_53d0;
pub const BUFFER_FOR_SEGMENT: u32 = 0x0063_fdb0;
pub const VFX_SET_VISIBLE: u32 = 0x0064_5710;
pub const GRID_STEP: u32 = 0x00b5_01a0;
pub const TWO: u32 = 0x00b9_2270;

/// `FUN_0092c740`'s (the guide map's character marker): `g_RcuiWork`,
/// `Ch_GetRootGlobalMatrix`, `rcbgScaleMatrix100`, `Ch_GetThicknessBtl`
/// (`ST0`), 10.0 (`0x00b922b0`), `crt_memset_thunk`,
/// `VFX_CopyViaC8f980_0092cbf0`, the line drawer `0x009278d0`, the
/// marker colours `0x00c5e7a8` (the selected character) and
/// `0x00c5e7f8`, `g_pppSinTable`.
pub const RCUI_WORK: u32 = 0x0193_c0f0;
pub const ROOT_MATRIX: u32 = 0x0083_3f80;
pub const SCALE_MATRIX100: u32 = 0x0091_ffd0;
pub const THICKNESS_BTL: u32 = 0x0082_ad90;
pub const TEN: u32 = 0x00b9_22b0;
pub const CRT_MEMSET: u32 = 0x0094_964c;
pub const COPY_VIA_C8F980: u32 = 0x0092_cbf0;
pub const DRAW_LINES: u32 = 0x0092_78d0;
pub const MARKER_COLOURS: u32 = 0x00c5_e798;
pub const SIN_TABLE: u32 = 0x00c4_4bf0;

/// `JobSchedule_pppPart_fillBuffer_drawShp_007287b0`'s constants: the
/// f64s 0.2 (the second y), 255 (alpha), 1/128 (size) and the f32 0.5.
pub const SHP_Y_DROP: u32 = 0x00b3_eb60;
pub const SHP_ALPHA_SCALE: u32 = 0x00b1_70c0;
pub const SHP_SIZE_SCALE: u32 = 0x00b1_70b8;
pub const SHP_HALF: u32 = 0x00b9_2194;
/// `JobSchedule_pppPart_fillBuffer_drawShp`'s f64 1/4096 (texture
/// coordinates and alpha in 1/4096).
pub const SHP_UV_SCALE: u32 = 0x00b5_0190;

/// `rcbgDebugDrawLayerTex_0092c960`'s: `Yn_DepthToScreen`,
/// `rcbgTexGetLog2Size`, `rcbgTrxSetNo`, `rcbgPalSetNo`, the packet data's
/// init/open/close/draw, the packet address word, the f32 depth 0.1.
pub const YN_DEPTH_TO_SCREEN: u32 = 0x0092_01a0;
pub const TEX_LOG2_SIZE: u32 = 0x0092_0050;
pub const TRX_SET_NO: u32 = 0x0092_d670;
pub const PAL_SET_NO: u32 = 0x0092_9ab0;
pub const PKT_DATA_INIT: u32 = 0x0092_7a20;
pub const PKT_DATA_OPEN: u32 = 0x0092_7a70;
pub const PKT_DATA_CLOSE: u32 = 0x0092_75b0;
pub const PKT_DATA_DRAW: u32 = 0x0092_7600;
pub const PKT_DATA_ADR: u32 = 0x00c4_98dc;
pub const LAYER_DEPTH: u32 = 0x00b1_0f38;

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
    /// One pass of `JobWorker`'s loop: a job popped from the worker's
    /// queue (`[this + 0x10]`) of the scheduler `sched`, run
    /// (`ThreadCall_run(job)`), pushed back to that queue (done).
    pub fn job_worker_pass(h: &mut impl Host, sched: u32, this: u32) {
        let queue: u32 = h.read(this.wrapping_add(0x10));
        let job = h.call_sym(super::QUEUE_POP, Conv::Thiscall(sched), &[queue]);
        h.call_sym(super::THREAD_CALL_RUN, Conv::Thiscall(job), &[]);
        let queue: u32 = h.read(this.wrapping_add(0x10));
        h.call_sym(super::QUEUE_PUSH, Conv::Thiscall(sched), &[job, queue]);
    }

    /// `ThreadLoop_00703a20(this)` (thiscall; `JobWorker`'s thread routine,
    /// its vtable's slot 1): the scheduler read once, then
    /// [`job_worker_pass`] for ever.
    pub fn job_worker_loop(h: &mut impl Host, this: u32) -> ! {
        let sched: u32 = h.read(h.rebase(super::JOB_SCHEDULER));
        loop {
            job_worker_pass(h, sched, this);
        }
    }

    /// `FileReadMain()`: a tail jump to the read task's current step (the
    /// first word of the record at `0x01128adc`), no stack words; its
    /// `eax`.
    pub fn file_read_main(h: &mut impl Host) -> u32 {
        let task: u32 = h.read(h.rebase(super::READ_TASK));
        let step: u32 = h.read(task);
        h.call(step, Conv::Cdecl, &[])
    }
    /// `updateVFXRenderData_AkazaScrFlt(lo, hi, uv, colour; slots)` with its
    /// frame at `ebp`: the instance's render data (`lo`, `hi`; none 0) and
    /// its buffer (`graphicDynGeoMemManagerGetBufferForSegment(data, 0,
    /// [+4], [+8])` of the data's record `+0x94`; none 0) filled with a
    /// 33 x 33 grid: vertex (i, j) at `(2 j s - 1, 2 i s - 1, 0)` (`s` =
    /// 1/32, x87 in that order, `i`'s row value rounded to `f32` first),
    /// colour the four floats at `colour`, UVs the next pair from `uv`
    /// (the pointer advanced in its argument slot), and for i, j < 32 two
    /// triangles of `u16` indices. Then `graphicVFXSetVisible(lo, hi, 1)`,
    /// 1. The frame's four words end as the original leaves them.
    pub fn akaza_scr_flt(h: &mut impl Host, ebp: u32, args: [u32; 4], slots: u32) -> u32 {
        use ffx_bridge::x87;
        let [lo, hi, uv, colour] = args;
        let data = cdecl(h, super::RENDER_DATA_BY_INSTANCE, &[lo, hi]);
        if data == 0 {
            return 0;
        }
        let rec: u32 = h.read(data.wrapping_add(0x94));
        let (a, b): (u32, u32) = (h.read(rec.wrapping_add(4)), h.read(rec.wrapping_add(8)));
        if cdecl(h, super::BUFFER_FOR_SEGMENT, &[data, 0, a, b]) == 0 {
            return 0;
        }
        let rec: u32 = h.read(data.wrapping_add(0x94));
        let two = f64::from_bits(h.read::<u64>(h.rebase(super::TWO)));
        let step = f64::from_bits(h.read::<u64>(h.rebase(super::GRID_STEP)));
        let mut pos: u32 = h.read(rec.wrapping_add(0xc));
        let mut col: u32 = h.read(rec.wrapping_add(0x14));
        let mut tex: u32 = h.read(rec.wrapping_add(0x18));
        let mut idx: u32 = h.read(rec.wrapping_add(0x1c));
        let mut uv = uv;
        let grid = |k: u32| -> f64 {
            let kf = f64::from(k); // fild + fstp dword: exact for 0..=32
            x87::sub(x87::mul(x87::mul(kf, two), step), 1.0)
        };
        for i in 0..=32u32 {
            let row = x87::load(x87::store(grid(i)));
            for j in 0..=32u32 {
                put_words(h, pos, &[x87::store(grid(j)), x87::store(row), 0]);
                let colour_words: [u32; 4] =
                    [0u32, 4, 8, 0xc].map(|off| h.read(colour.wrapping_add(off)));
                put_words(h, col, &colour_words);
                let t: [u32; 2] = [h.read(uv), h.read(uv.wrapping_add(4))];
                put_words(h, tex, &t);
                uv = uv.wrapping_add(8);
                h.write(slots.wrapping_add(8), uv);
                pos = pos.wrapping_add(0xc);
                col = col.wrapping_add(0x10);
                tex = tex.wrapping_add(8);
                if i < 32 && j < 32 {
                    let base = i * 33 + j;
                    for (k, v) in [0u32, 1, 0x21, 1, 0x22, 0x21].into_iter().enumerate() {
                        #[allow(clippy::cast_possible_truncation)] // the u16 stores
                        h.write(idx.wrapping_add(2 * k as u32), (base + v) as u16);
                    }
                    idx = idx.wrapping_add(0xc);
                }
            }
        }
        put_words(h, ebp.wrapping_sub(0x10), &[tex, col, 33, 33]);
        cdecl(h, super::VFX_SET_VISIBLE, &[lo, hi, 1]);
        1
    }
    /// The sine table's entry `i` (`g_pppSinTable`, f32).
    fn sin_entry(h: &impl Host, i: u32) -> f64 {
        ffx_bridge::x87::load(h.read(h.rebase(super::SIN_TABLE).wrapping_add(4 * i)))
    }

    /// `FUN_0092c740(out, chr)` with its frame at `ebp`: the character's
    /// guide-map marker. Its root matrix at `ebp - 0x44`
    /// (`Ch_GetRootGlobalMatrix`, `rcbgScaleMatrix100`), the radius
    /// `Ch_GetThicknessBtl(chr) * 10.0` (`f32`); 33 16-byte points from
    /// `ebp - 0x254` cleared. A diamond of 5 points (x = `sin[0] r`, `sin
    /// [0x638] r`, 0, `sin[0x9c7] r`, the first again; z the negated
    /// cosines) drawn by `0x009278d0(out, points, 5, colour, matrix)`,
    /// then the circle of 33 points (x = `sin[a] r`, z = `-sin[a + 0x400]
    /// r` for `a` = `0x800 k`, k = 0..32) the same way. The colour:
    /// `0x00c5e7a8` when `chr` is the selected character (`g_RcuiWork`'s
    /// list `+0x870` at `+0x8b0`), else `0x00c5e7f8`. The last draw's
    /// `eax`.
    pub fn guide_marker(h: &mut impl Host, ebp: u32, out: u32, chr: u32) -> u32 {
        use ffx_bridge::x87;
        let work: u32 = h.read(h.rebase(super::RCUI_WORK));
        let matrix = ebp.wrapping_sub(0x44);
        cdecl(h, super::ROOT_MATRIX, &[chr, matrix]);
        cdecl(h, super::SCALE_MATRIX100, &[matrix]);
        let thick = h.call_sym_st0_f64(super::THICKNESS_BTL, Conv::Cdecl, &[chr]);
        let ten = f64::from_bits(h.read::<u64>(h.rebase(super::TEN)));
        let r_bits = x87::store(x87::mul(thick, ten));
        h.write(ebp.wrapping_sub(0x258), r_bits);
        let r = x87::load(r_bits);
        let pts = ebp.wrapping_sub(0x254);
        cdecl(h, super::CRT_MEMSET, &[pts, 0, 0x210]);
        // the diamond: (x, z) of points 0, 1, 2, 3 at +0, +8 of each
        for (k, (sx, sz)) in (0u32..).zip([
            (0u32, 0x400u32),
            (0x638, 0xa38),
            (0x800, 0xc00),
            (0x9c7, 0xdc7),
        ]) {
            let at = pts.wrapping_add(16 * k);
            let x = x87::mul(sin_entry(h, sx), r);
            h.write(at, x87::store(x));
            let z = x87::mul(-sin_entry(h, sz), r);
            h.write(at.wrapping_add(8), x87::store(z));
        }
        cdecl(h, super::COPY_VIA_C8F980, &[pts.wrapping_add(0x40), pts]);
        put_words(h, pts.wrapping_add(0x20), &[0, 0, 0, 0]);
        let list: u32 = h.read(work.wrapping_add(0x8b0));
        let selected: u32 = h.read(work.wrapping_add(0x870 + 4 * list));
        let colour_k = if selected == chr { 1 } else { 6 };
        let colour = h.rebase(super::MARKER_COLOURS).wrapping_add(colour_k << 4);
        h.write(ebp.wrapping_sub(0x25c), colour);
        cdecl(h, super::DRAW_LINES, &[out, pts, 5, colour, matrix]);
        let mut at = pts;
        let mut a = 0u32;
        while a <= 0x20_0000 {
            let turn = a.cast_signed().wrapping_div(32).cast_unsigned();
            let sx = (turn >> 4) & 0xfff;
            let sz = (turn.wrapping_add(0x4000) >> 4) & 0xfff;
            h.write(at, x87::store(x87::mul(sin_entry(h, sx), r)));
            h.write(
                at.wrapping_add(8),
                x87::store(x87::mul(-sin_entry(h, sz), r)),
            );
            at = at.wrapping_add(0x10);
            a += 0x1_0000;
        }
        cdecl(h, super::DRAW_LINES, &[out, pts, 0x21, colour, matrix])
    }
    /// The f32 at `a` loaded to the x87 stack.
    fn shp_load(h: &impl Host, a: u32) -> f64 {
        ffx_bridge::x87::load(h.read(a))
    }

    /// The f32 at `a` through `fld`/`fstp`.
    fn shp_copy(h: &impl Host, a: u32) -> u32 {
        ffx_bridge::x87::store(shp_load(h, a))
    }

    /// Zeroes `block`'s index pairs `from..3000` (its pointer `+0x1c`
    /// read before each store); the last pointer read.
    fn shp_clear(h: &mut impl Host, block: u32, from: u32) -> u32 {
        let mut idx = 0u32;
        for k in from..3000 {
            idx = h.read(block.wrapping_add(0x1c));
            h.write(idx.wrapping_add(k * 4), 0u16);
            idx = h.read(block.wrapping_add(0x1c));
            h.write(idx.wrapping_add(k * 4 + 2), 0u16);
        }
        idx
    }

    /// `JobSchedule_pppPart_fillBuffer_drawShp_007287b0(job)` with its
    /// frame at `ebp`: the particles at `job + 0x1c` (`job + 0x20` of
    /// them, 0x20 bytes each) into up to three vertex blocks (`job + 0x10`,
    /// 3000 entries each: positions `+0xc` stride 0x18, colours `+0x14`
    /// stride 0x20, index pairs `+0x1c`). A particle aged 0x20 or more is
    /// skipped; else its position moves by its velocity (`+0x10..`, x87
    /// sums), its age goes up, and from age 1 it is written: the position
    /// twice (the second `y - 0.2`), `0.5` and `u8 +0xe / 255` and the
    /// size (`age` mirrored at 16, `* 8 / 128`) into the colour, the index
    /// pair `2n, 2n + 1`. A full block moves on to the next; the third
    /// full stops. The rest of the block and the later blocks' index
    /// pairs are zeroed. The frame's words end as the original leaves
    /// them; `eax` the block after the last plus one, or the last index
    /// pointer read.
    #[allow(
        clippy::many_single_char_names,
        clippy::too_many_lines,
        clippy::cast_possible_truncation
    )]
    pub fn draw_shp_fill(h: &mut impl Host, ebp: u32, job: u32) -> u32 {
        use ffx_bridge::x87;
        let f = |o: u32| ebp.wrapping_sub(o);
        let y_drop = f64::from_bits(h.read::<u64>(h.rebase(super::SHP_Y_DROP)));
        let alpha_scale = f64::from_bits(h.read::<u64>(h.rebase(super::SHP_ALPHA_SCALE)));
        let size_scale = f64::from_bits(h.read::<u64>(h.rebase(super::SHP_SIZE_SCALE)));
        let half: u32 = h.read(h.rebase(super::SHP_HALF));
        let mut block: u32 = h.read(job.wrapping_add(0x10));
        let mut part: u32 = h.read(job.wrapping_add(0x1c));
        let mut left: u32 = h.read(job.wrapping_add(0x20));
        let mut blk = 0u32;
        h.write(f(4), blk);
        let mut n = 0u32;
        'blocks: loop {
            let mut slot = job.wrapping_add(0x10).wrapping_add(blk.wrapping_mul(4));
            let mut count = 0u32;
            h.write(f(0x10), count);
            h.write(f(0xc), slot);
            loop {
                h.write(f(8), left);
                if left.cast_signed() <= 0 {
                    break 'blocks;
                }
                let age = h.read::<u16>(part.wrapping_add(0x1c)).cast_signed();
                if age >= 0x20 {
                    left = left.wrapping_sub(1);
                    part = part.wrapping_add(0x20);
                    continue;
                }
                let mirrored = if age >= 0x10 {
                    0x20 - i32::from(age)
                } else {
                    i32::from(age)
                };
                let size = mirrored.cast_unsigned() << 3;
                let x = x87::store(x87::add(
                    shp_load(h, part.wrapping_add(0x10)),
                    shp_load(h, part),
                ));
                h.write(f(0x14), x);
                h.write(part, x);
                let aged = age.wrapping_add(1);
                h.write(part.wrapping_add(0x1c), aged.cast_unsigned());
                for o in [4u32, 8] {
                    let v = x87::store(x87::add(
                        shp_load(h, part.wrapping_add(0x10 + o)),
                        shp_load(h, part.wrapping_add(o)),
                    ));
                    h.write(part.wrapping_add(o), v);
                }
                if aged > 1 {
                    let mut pos = h
                        .read::<u32>(block.wrapping_add(0xc))
                        .wrapping_add(n.wrapping_mul(0x18));
                    let mut col = h.read::<u32>(block.wrapping_add(0x14)).wrapping_add(n << 5);
                    if n >= 3000 {
                        blk = blk.wrapping_add(1);
                        h.write(f(4), blk);
                        block = h.read(slot.wrapping_add(4));
                        n = 0;
                        col = h.read(block.wrapping_add(0x14));
                        slot = slot.wrapping_add(4);
                        h.write(f(0xc), slot);
                        pos = h.read(block.wrapping_add(0xc));
                    }
                    let y = shp_copy(h, part.wrapping_add(4));
                    let z = shp_copy(h, part.wrapping_add(8));
                    let x2 = shp_copy(h, part);
                    let y2 = x87::store(x87::sub(shp_load(h, part.wrapping_add(4)), y_drop));
                    let z2 = shp_copy(h, part.wrapping_add(8));
                    put_words(h, pos, &[x, y, z, x2, y2, z2]);
                    let a = u32::from(h.read::<u8>(part.wrapping_add(0xe)));
                    h.write(f(0x14), a);
                    h.write(f(0x1c), f64::from(a).to_bits());
                    let alpha = x87::store(x87::div(f64::from(a), alpha_scale));
                    h.write(f(0x18), alpha);
                    h.write(f(0x14), size);
                    // `size` is 8..=128 here: the fild's unsigned fix-up
                    // (+2^32 when negative) never applies
                    let sized = x87::store(f64::from(size));
                    h.write(f(0x14), sized);
                    let scaled = x87::store(x87::mul(x87::load(sized), size_scale));
                    h.write(f(0x14), scaled);
                    let idx: u32 = h.read(block.wrapping_add(0x1c));
                    for o in [0x10u32, 0, 0x14, 4] {
                        h.write(col.wrapping_add(o), half);
                    }
                    h.write(col.wrapping_add(0x18), alpha);
                    h.write(col.wrapping_add(8), alpha);
                    h.write(col.wrapping_add(0x1c), scaled);
                    h.write(col.wrapping_add(0xc), scaled);
                    h.write(idx.wrapping_add(n * 4), (n * 2) as u16);
                    let idx: u32 = h.read(block.wrapping_add(0x1c));
                    h.write(idx.wrapping_add(n * 4 + 2), (n * 2 + 1) as u16);
                    n += 1;
                }
                count = count.wrapping_add(1);
                h.write(f(0x10), count);
                if n >= 3000 && blk == 2 {
                    break 'blocks;
                }
                left = left.wrapping_sub(1);
                part = part.wrapping_add(0x20);
                if count >= 0x10 {
                    continue 'blocks;
                }
            }
        }
        if n < 3000 {
            shp_clear(h, block, n);
        }
        let mut r = blk.wrapping_add(1);
        let mut b = r;
        while b.cast_signed() < 3 {
            let block: u32 = h.read(job.wrapping_add(0x10).wrapping_add(b * 4));
            r = shp_clear(h, block, 0);
            b += 1;
        }
        r
    }
    /// A triangle's u16 table number at `tri + o`, times 4 in 16 bits
    /// (`shl ax, 2`): the f32 index of its 16-byte table entry.
    fn tri_entry(h: &impl Host, tri: u32, o: u32) -> u32 {
        u32::from(h.read::<u16>(tri.wrapping_add(o)).wrapping_shl(2))
    }

    /// `JobSchedule_pppPart_fillBuffer_md0_ref_0279(job)`: a model's
    /// triangles into the vertex block `job + 0x10` (positions `+0xc`
    /// stride 0x24, colours `+0x14` stride 0x30, texture coordinates
    /// `+0x18` stride 0x18, u16 index triples `+0x1c` stride 6). The model
    /// `job + 0x14` (`+4` the offset of its groups, i16 `+0x10` the
    /// triangles in all) has groups of a 0x20-byte head (i16 `+2` the
    /// triangles) and 0x1c-byte triangles: u16 position numbers `+0`,
    /// `+2`, `+4` and texture numbers `+0x14`, `+0x16`, `+0x18` into the
    /// 16-byte tables `job + 0x18` (x, y, z copied) and `job + 0x1c` (u,
    /// `1 - v`). Every vertex gets the colour `(a * b + c) / 128` from
    /// the f32s `job + 0x20` (a), `+0x30` (b) and `+0x40` (c), the alpha
    /// halved. Returns `eax`: `7 *` the last group's count, or the word
    /// `job + 0x4c` when the model has no triangles.
    #[allow(clippy::many_single_char_names, clippy::cast_possible_truncation)]
    pub fn md0_ref_fill(h: &mut impl Host, job: u32) -> u32 {
        use ffx_bridge::x87;
        let block: u32 = h.read(job.wrapping_add(0x10));
        let model: u32 = h.read(job.wrapping_add(0x14));
        let pos_tab: u32 = h.read(job.wrapping_add(0x18));
        let uv_tab: u32 = h.read(job.wrapping_add(0x1c));
        let f: [f64; 12] =
            core::array::from_fn(|k| shp_load(h, job.wrapping_add(0x20 + 4 * k as u32)));
        let last: u32 = h.read(job.wrapping_add(0x4c));
        let mut group = model.wrapping_add(h.read::<u32>(model.wrapping_add(4)));
        let mut left = i32::from(h.read::<i16>(model.wrapping_add(0x10)));
        if left <= 0 {
            return last;
        }
        let scale = f64::from_bits(h.read::<u64>(h.rebase(super::SHP_SIZE_SCALE)));
        let half = f64::from_bits(h.read::<u64>(h.rebase(super::HALF)));
        // each channel: a * b + c rounded to f32, then / 128 (the alpha
        // rounded once more before the halving)
        let channel = |k: usize| {
            let sum = x87::store(x87::add(x87::mul(f[k], f[k + 4]), f[k + 8]));
            x87::mul(x87::load(sum), scale)
        };
        let colour = [
            x87::store(channel(0)),
            x87::store(channel(1)),
            x87::store(channel(2)),
            x87::store(x87::mul(x87::load(x87::store(channel(3))), half)),
        ];
        let mut n = 0u32;
        loop {
            let mut k = 0i32;
            let mut tri = group.wrapping_add(0x20);
            while k < i32::from(h.read::<i16>(group.wrapping_add(2))) {
                let pos_out = h
                    .read::<u32>(block.wrapping_add(0xc))
                    .wrapping_add(n.wrapping_mul(0x24));
                let uv_out = h
                    .read::<u32>(block.wrapping_add(0x18))
                    .wrapping_add(n.wrapping_mul(0x18));
                let col_out = h
                    .read::<u32>(block.wrapping_add(0x14))
                    .wrapping_add(n.wrapping_mul(0x30));
                let pos = [0u32, 2, 4].map(|o| tri_entry(h, tri, o));
                let uv = [0x14u32, 0x16, 0x18].map(|o| tri_entry(h, tri, o));
                for (v, &e) in (0u32..).zip(&pos) {
                    for c in 0..3u32 {
                        let w = shp_copy(h, pos_tab.wrapping_add((e + c) * 4));
                        h.write(pos_out.wrapping_add(12 * v + 4 * c), w);
                    }
                }
                for (v, &e) in (0u32..).zip(&uv) {
                    let u = shp_copy(h, uv_tab.wrapping_add(e * 4));
                    h.write(uv_out.wrapping_add(8 * v), u);
                    let t = shp_load(h, uv_tab.wrapping_add((e + 1) * 4));
                    h.write(uv_out.wrapping_add(8 * v + 4), x87::store(x87::sub(1.0, t)));
                }
                for (c, &w) in (0u32..).zip(&colour) {
                    let order: &[u32] = if c == 3 { &[0, 1, 2] } else { &[2, 1, 0] };
                    for &v in order {
                        h.write(col_out.wrapping_add(0x10 * v + 4 * c), w);
                    }
                }
                let first = n.wrapping_mul(3);
                for j in 0..3u32 {
                    let idx: u32 = h.read(block.wrapping_add(0x1c));
                    h.write(
                        idx.wrapping_add(n.wrapping_mul(6)).wrapping_add(2 * j),
                        first.wrapping_add(j) as u16,
                    );
                }
                n = n.wrapping_add(1);
                k += 1;
                tri = tri.wrapping_add(0x1c);
            }
            let count = i32::from(h.read::<i16>(group.wrapping_add(2)));
            left = left.wrapping_sub(count);
            group = group
                .wrapping_add(0x20)
                .wrapping_add(count.cast_unsigned().wrapping_mul(0x1c));
            if left <= 0 {
                return count.cast_unsigned().wrapping_mul(7);
            }
        }
    }
    /// `JobSchedule_pppPart_fillBuffer_drawShp(job)` with the caller's
    /// argument slot at `slot`: a shape model's triangles into the vertex
    /// block `job + 0x10` (positions `+0xc` stride 0x24, colours `+0x14`
    /// stride 0x30, texture coordinates `+0x18` stride 0x18, u16 index
    /// triples `+0x1c` stride 6). The model `job + 0x14` is laid out as
    /// in [`md0_ref_fill`]; a triangle's u16 at `+0`, `+2`, `+4` number
    /// its positions (table `job + 0x1c`, x, y, z copied), the six at
    /// `+8..+0x12` are its texture coordinates in 1/4096 and the three
    /// at `+0x14..+0x18` number its colours (table `job + 0x20`, r, g, b
    /// copied). Each triangle takes the next entry of the i16 list
    /// `[job + 0x18] + 0xc` from the running number `job + 0x24`: -2
    /// leaves the triangle out (index triple zeroed), -1 gives it the
    /// alpha 0.5, any other `e` the alpha `u16 [job + 0x18] + 0x8e +
    /// 0x60 e` / 8192. Groups whose word `+8` has type 0x24 in bits
    /// 26..29 and whose byte `+0xc` ends in 8 take the coordinates at
    /// full scale from `v = 0.5` down, others at half scale from 0. The
    /// indices go `3n, 3n + 2, 3n + 1`. The original keeps the triangle
    /// pointer (`+4`) in its argument slot. Returns `eax`: the triangle
    /// count left over (0 or less).
    #[allow(
        clippy::too_many_lines,
        clippy::cast_possible_truncation,
        clippy::many_single_char_names
    )]
    pub fn shp_model_fill(h: &mut impl Host, slot: u32, job: u32) -> u32 {
        use ffx_bridge::x87;
        let model: u32 = h.read(job.wrapping_add(0x14));
        let block: u32 = h.read(job.wrapping_add(0x10));
        let parts: u32 = h.read(job.wrapping_add(0x18));
        let pos_tab: u32 = h.read(job.wrapping_add(0x1c));
        let col_tab: u32 = h.read(job.wrapping_add(0x20));
        let mut number: u32 = h.read(job.wrapping_add(0x24));
        let mut group = model.wrapping_add(h.read::<u32>(model.wrapping_add(4)));
        let total = i32::from(h.read::<i16>(model.wrapping_add(0x10)));
        let mut left = total;
        if left <= 0 {
            return total.cast_unsigned();
        }
        let half = x87::load(h.read(h.rebase(super::SHP_HALF)));
        let q = f64::from_bits(h.read::<u64>(h.rebase(super::SHP_UV_SCALE)));
        let half64 = f64::from_bits(h.read::<u64>(h.rebase(super::HALF)));
        let mut n = 0u32;
        loop {
            let kind: u32 = h.read(group.wrapping_add(8));
            let sub: u8 = h.read(group.wrapping_add(0xc));
            // (v offset, u scale): f32 frame copies of the constants
            let (top, scale) = if kind & 0x3c00_0000 == 0x2400_0000 && sub & 0xf == 8 {
                (half, 1.0)
            } else {
                (0.0, half)
            };
            let one_minus_top = x87::sub(1.0, top);
            let mut k = 0i32;
            let mut tri = group.wrapping_add(0x20);
            while k < i32::from(h.read::<i16>(group.wrapping_add(2))) {
                let list: u32 = h.read(parts.wrapping_add(0xc));
                let entry: u16 = h.read(list.wrapping_add(number.wrapping_mul(2)));
                if entry == 0xfffe {
                    let idx: u32 = h.read(block.wrapping_add(0x1c));
                    h.write(idx.wrapping_add(n.wrapping_mul(6)), 0u32);
                    h.write(idx.wrapping_add(n.wrapping_mul(6)).wrapping_add(4), 0u16);
                } else {
                    let uv_out = h
                        .read::<u32>(block.wrapping_add(0x18))
                        .wrapping_add(n.wrapping_mul(0x18));
                    let col_out = h
                        .read::<u32>(block.wrapping_add(0x14))
                        .wrapping_add(n.wrapping_mul(0x30));
                    let pos_out = h
                        .read::<u32>(block.wrapping_add(0xc))
                        .wrapping_add(n.wrapping_mul(0x24));
                    let pos = [0u32, 2, 4].map(|o| tri_entry(h, tri, o));
                    let col = [0x14u32, 0x16, 0x18].map(|o| tri_entry(h, tri, o));
                    for (v, &e) in (0u32..).zip(&pos) {
                        for c in 0..3u32 {
                            let w = shp_copy(h, pos_tab.wrapping_add((e + c) * 4));
                            h.write(pos_out.wrapping_add(12 * v + 4 * c), w);
                        }
                    }
                    // u and 1 - v in 1/4096, each rounded to f32
                    let mut uv = [0u32; 6];
                    for (k2, w) in (0u32..).zip(&mut uv) {
                        let raw = f64::from(h.read::<u16>(tri.wrapping_add(8 + 2 * k2)));
                        *w = if k2 % 2 == 0 {
                            x87::store(x87::mul(raw, q))
                        } else {
                            x87::store(x87::sub(1.0, x87::mul(raw, q)))
                        };
                    }
                    for (k2, &w) in (0u32..).zip(&uv) {
                        let out = if k2 % 2 == 0 {
                            x87::add(x87::mul(scale, x87::load(w)), 0.0)
                        } else {
                            x87::sub(one_minus_top, x87::mul(x87::load(w), half64))
                        };
                        h.write(uv_out.wrapping_add(4 * k2), x87::store(out));
                    }
                    let rgb = col.map(|e| {
                        [0u32, 1, 2].map(|c| shp_copy(h, col_tab.wrapping_add((e + c) * 4)))
                    });
                    let alpha = if entry == 0xffff {
                        x87::store(half)
                    } else {
                        let at = parts.wrapping_add(0x8e).wrapping_add(
                            i32::from(entry.cast_signed())
                                .cast_unsigned()
                                .wrapping_mul(0x60),
                        );
                        let w = f64::from(h.read::<u16>(at));
                        x87::store(x87::mul(x87::mul(w, q), half64))
                    };
                    for (v, c) in (0u32..).zip(&rgb) {
                        let o = col_out.wrapping_add(0x10 * v);
                        h.write(o, c[0]);
                        h.write(o.wrapping_add(4), c[1]);
                        h.write(o.wrapping_add(8), c[2]);
                        h.write(o.wrapping_add(0xc), alpha);
                    }
                    let first = n.wrapping_mul(3);
                    for (j, add) in [(0u32, 0u32), (1, 2), (2, 1)] {
                        let idx: u32 = h.read(block.wrapping_add(0x1c));
                        h.write(
                            idx.wrapping_add(n.wrapping_mul(6)).wrapping_add(2 * j),
                            first.wrapping_add(add) as u16,
                        );
                    }
                }
                n = n.wrapping_add(1);
                number = number.wrapping_add(1);
                k += 1;
                tri = tri.wrapping_add(0x1c);
                h.write(slot, tri.wrapping_add(4));
            }
            let count = i32::from(h.read::<i16>(group.wrapping_add(2)));
            left = left.wrapping_sub(count);
            group = group
                .wrapping_add(0x20)
                .wrapping_add(count.cast_unsigned().wrapping_mul(0x1c));
            if left <= 0 {
                return left.cast_unsigned();
            }
        }
    }

    /// `rcbgDebugDrawLayerTex_0092c960(layer)` with its frame at `ebp`
    /// (the debug overlay's layer texture): `Yn_DepthToScreen(0.1)` into
    /// `ebp - 0x18`; without a map (`g_YnMapSystem`'s first word) its
    /// `eax`. Else the layer `[[map] + 8] + 8 + 0xc0 * layer`:
    /// `rcbgTexGetLog2Size(w +4, h +8, ebp - 0xc, ebp - 8, ebp - 0x10)`,
    /// its texture and palette set (`rcbgTrxSetNo(+0xa0, 0)`,
    /// `rcbgPalSetNo(+0xb0, 0)`), a packet opened at `ebp - 0x20`
    /// (`[0x00c498dc]`): a GS tag, TEX0 from the log2 sizes and the width
    /// (64-bit, sign-extended ORs), `+0x38`/`+0x3c`, RGBAQ from the bytes
    /// `+0x40..`, then PRIM, the scissor from `+0x24`, `+0x28` and the
    /// size, and a sprite of the size at the screen offset and depth;
    /// closed and drawn. The draw's `eax`.
    #[allow(clippy::many_single_char_names, clippy::cast_possible_truncation)]
    pub fn debug_layer_tex(h: &mut impl Host, ebp: u32, layer: u32) -> u32 {
        let f = |o: u32| ebp.wrapping_sub(o);
        let sext = |v: u32| {
            u64::from(v)
                | if v.cast_signed() < 0 {
                    0xffff_ffff_0000_0000
                } else {
                    0
                }
        };
        let root = h.rebase(super::YN_MAP_SYSTEM);
        let sys: u32 = h.read(root);
        let map: u32 = if sys == 0 { 0 } else { h.read(sys) };
        let tenth =
            ffx_bridge::x87::round_trip(f32::from_bits(h.read(h.rebase(super::LAYER_DEPTH))));
        let depth = cdecl(h, super::YN_DEPTH_TO_SCREEN, &[tenth.to_bits()]);
        h.write(f(0x18), depth);
        if map == 0 {
            return depth;
        }
        let sys: u32 = h.read(root);
        let list: u32 = if sys == 0 {
            0
        } else {
            let m: u32 = h.read(sys);
            if m == 0 { 0 } else { h.read(m.wrapping_add(8)) }
        };
        let base: u32 = h.read(list.wrapping_add(8));
        let lay = layer.wrapping_mul(0xc0).wrapping_add(base);
        h.write(f(4), lay);
        let w: u32 = h.read(lay.wrapping_add(4));
        let ht: u32 = h.read(lay.wrapping_add(8));
        cdecl(h, super::TEX_LOG2_SIZE, &[w, ht, f(0xc), f(8), f(0x10)]);
        let trx: u32 = h.read(lay.wrapping_add(0xa0));
        cdecl(h, super::TRX_SET_NO, &[trx, 0]);
        cdecl(h, super::PAL_SET_NO, &[lay.wrapping_add(0xb0), 0]);
        let pkt = f(0x20);
        cdecl(h, super::PKT_DATA_INIT, &[pkt]);
        let adr: u32 = h.read(h.rebase(super::PKT_DATA_ADR));
        cdecl(h, super::PKT_DATA_OPEN, &[pkt, adr]);
        let p: u32 = h.read(f(0x1c));
        h.write(p.wrapping_add(8), 0x3fu32);
        let p2: u32 = h.read(f(0x1c));
        h.write(p2.wrapping_add(0xc), 0u32);
        let [lh, lw, tbw] = [8u32, 0xc, 0x10].map(|o| sext(h.read(f(o))));
        let t = ((lh << 4 | lw) << 12 | tbw) << 14 | 0x2005_c004_0130_3c00;
        put_words(h, p.wrapping_add(0x10), &[t as u32, (t >> 32) as u32, 6, 0]);
        let tex1: [u32; 2] = [
            h.read(lay.wrapping_add(0x38)),
            h.read(lay.wrapping_add(0x3c)),
        ];
        put_words(h, p.wrapping_add(0x20), &[tex1[0], tex1[1], 0x42, 0]);
        let rgba = u32::from_le_bytes(
            [0x40u32, 0x41, 0x42, 0x43].map(|o| h.read::<u8>(lay.wrapping_add(o))),
        );
        h.write(p.wrapping_add(0x38), 1u32);
        h.write(p.wrapping_add(0x3c), 0u32);
        put_words(h, p.wrapping_add(0x30), &[rgba, 0]);
        let y0: u32 = h.read(lay.wrapping_add(0x28));
        let y1 = ht.wrapping_sub(1).wrapping_add(y0);
        let q = p.wrapping_add(0x40);
        h.write(f(0x14), q);
        let x0: u32 = h.read(lay.wrapping_add(0x24));
        let lay2: u32 = h.read(f(4));
        let x1 = h
            .read::<u32>(lay2.wrapping_add(4))
            .wrapping_sub(1)
            .wrapping_add(x0);
        let mut s = sext(y1) << 10 | sext(y0);
        s = s << 10 | sext(x1);
        s = s << 10 | sext(x0);
        s = s << 4 | 0xa;
        h.write(q.wrapping_add(8), 8u32);
        h.write(q.wrapping_add(0xc), 0u32);
        h.write(q.wrapping_add(4), (s >> 32) as u32);
        h.write(q, s as u32);
        put_words(
            h,
            q.wrapping_add(0x10),
            &[0x60, 0, 0x14, 0, 0x3_0000, 0, 0x47, 0],
        );
        let lay3: u32 = h.read(f(4));
        let sw = h.read::<u32>(lay3.wrapping_add(4)) << 3;
        let sh = h.read::<u32>(lay3.wrapping_add(8)) << 3;
        put_words(h, q.wrapping_add(0x30), &[0, 0]);
        put_words(h, q.wrapping_add(0x40), &[0x6ff8, 0x7378, depth, 0]);
        put_words(
            h,
            q.wrapping_add(0x50),
            &[sw.wrapping_mul(2), sh.wrapping_mul(2)],
        );
        put_words(
            h,
            q.wrapping_add(0x60),
            &[sw.wrapping_add(0x6ff8), sh.wrapping_add(0x7378), depth, 0],
        );
        h.write(f(0x1c), q.wrapping_add(0x70));
        cdecl(h, super::PKT_DATA_CLOSE, &[pkt]);
        cdecl(h, super::PKT_DATA_DRAW, &[pkt])
    }
}
