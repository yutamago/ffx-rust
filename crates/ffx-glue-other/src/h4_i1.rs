//! Continuation round 7 of list `h4-glue-other`, batch 1: the Win32
//! video playback's close, the PS2 layer's VRAM save and debug file read,
//! the application's window setup, the VFX line buffer, a particle
//! model's vertex copy and two of `GDraw`'s pool helpers.

use ffx_graphic::host::{Conv, Host};

use crate::h4_e1::{int_f32, pass};

// ---- PVideoPlaybackWin32_close

/// `CDataFrameQueue_Close(queue)` (thiscall).
pub const QUEUE_CLOSE: u32 = 0x00a3_dea0;
/// `Phyre_PEventFlagWin32_wait(flag)` (thiscall).
pub const EVENT_WAIT: u32 = 0x0043_02a0;
/// `FUN_00430240(flag)` (thiscall): the event flag's reset (an import on
/// `[flag]`).
pub const EVENT_RESET: u32 = 0x0043_0240;
/// `mkvparser_Segment_delete(segment)` (cdecl).
pub const SEGMENT_DELETE: u32 = 0x0041_fd10;
/// `mkvparser_ResetAllocator()` (cdecl).
pub const RESET_ALLOCATOR: u32 = 0x0041_e5d0;
/// `VbfMkvReader_Close(reader)` (thiscall).
pub const READER_CLOSE: u32 = 0x00a3_e150;
/// `FUN_004301b0(thread; &code)` (thiscall, ret 4): waits for the thread
/// handle `[thread]` and stores its exit code through `code`.
pub const THREAD_JOIN: u32 = 0x0043_01b0;
/// `FUN_00430260(thread)` (thiscall): `xor eax, eax; ret`.
pub const THREAD_CLOSE: u32 = 0x0043_0260;
/// `vpx_codec_destroy(ctx)` (cdecl).
pub const VPX_DESTROY: u32 = 0x0040_9370;
/// `vpx_mem_unset_functions()` (cdecl).
pub const VPX_UNSET: u32 = 0x0040_a630;
/// `PVideoPlaybackWin32_AudioShutdown(player)` (thiscall).
pub const AUDIO_SHUTDOWN: u32 = 0x00a2_78f0;
/// `CDataFrameQueue_Destroy(queue)` (thiscall).
pub const QUEUE_DESTROY: u32 = 0x00a3_df20;
/// Bytes of [`video_close`]'s frame: the exit-code word (`ebp - 4`).
pub const CLOSE_FRAME: u32 = 4;

/// `PVideoPlaybackWin32_close(this)` (`0x00a28350`, thiscall; `frame` the
/// word at `ebp - 4`, which starts as `this`: the original's `push ecx`).
/// For a state `+0xe4` other than 0 and 3: the state set to 3 and `+0xec`
/// to 1, the frame queue `+0` closed, the decode thread's event flag
/// `+0x2b4` waited on and reset (its byte `+0x2b8` cleared), the MKV
/// segment `+0x110` deleted and the allocator reset, the reader `+0x118`
/// and the queue `+0x40` closed, the thread `+0xc8` joined (its exit code
/// into the frame) and closed (its byte `+0xcc` cleared), the VPX codec
/// `+0x130` destroyed and its memory functions unset, the audio shut down
/// when `+0x294` is set, the queues `+0` and `+0x40` destroyed.
///
/// The original returns with only `al` set (`mov al, 1`): the rest of
/// `eax` is the last callee's (`CDataFrameQueue_Destroy` leaves its
/// entry `eax` when the queue is not open) or the state's zero upper
/// bytes on the early path. Two callers reach it by a `jmp` from code
/// Ghidra left outside any function (`0x00658210`, `0x0066fb40`), so the
/// port returns that whole `eax`.
pub fn video_close(h: &mut impl Host, frame: u32, this: u32) -> u32 {
    h.write(frame, this);
    let state: u32 = h.read(this.wrapping_add(0xe4));
    if state == 0 || state == 3 {
        // eax = the state (0 or 3), al = 1
        return 1;
    }
    let at = |off: u32| this.wrapping_add(off);
    h.write(at(0xe4), 3u32);
    h.write(at(0xec), 1u32);
    h.call_sym(QUEUE_CLOSE, Conv::Thiscall(this), &[]);
    h.call_sym(EVENT_WAIT, Conv::Thiscall(at(0x2b4)), &[]);
    h.call_sym(EVENT_RESET, Conv::Thiscall(at(0x2b4)), &[]);
    h.write(at(0x2b8), 0u8);
    let segment: u32 = h.read(at(0x110));
    h.call_sym(SEGMENT_DELETE, Conv::Cdecl, &[segment]);
    h.call_sym(RESET_ALLOCATOR, Conv::Cdecl, &[]);
    h.call_sym(READER_CLOSE, Conv::Thiscall(at(0x118)), &[]);
    h.call_sym(QUEUE_CLOSE, Conv::Thiscall(at(0x40)), &[]);
    h.call_sym(THREAD_JOIN, Conv::Thiscall(at(0xc8)), &[frame]);
    h.call_sym(THREAD_CLOSE, Conv::Thiscall(at(0xc8)), &[]);
    h.write(at(0xcc), 0u8);
    h.call_sym(VPX_DESTROY, Conv::Cdecl, &[at(0x130)]);
    h.call_sym(VPX_UNSET, Conv::Cdecl, &[]);
    if h.read::<u32>(at(0x294)) != 0 {
        h.call_sym(AUDIO_SHUTDOWN, Conv::Thiscall(this), &[]);
    }
    h.call_sym(QUEUE_DESTROY, Conv::Thiscall(this), &[]);
    let eax = h.call_sym(QUEUE_DESTROY, Conv::Thiscall(at(0x40)), &[]);
    (eax & !0xff) | 1
}

// ---- Sg_SaveVramResource

/// `dbgPrintf(format, ...)` (cdecl).
pub const DBG_PRINTF: u32 = 0x0062_f500;
/// `memclear(p, n)` (cdecl).
pub const MEMCLEAR: u32 = 0x0062_f4e0;
/// `sceGsSetDefLoadImage(image, dbp, dbw, psm, x, y, w, h)` (cdecl).
pub const SET_DEF_LOAD_IMAGE: u32 = 0x0062_f820;
/// `FlushCache(mode)` (cdecl).
pub const FLUSH_CACHE: u32 = 0x0062_f4c0;
/// `sceGsExecLoadImage(image, data)` (cdecl).
pub const EXEC_LOAD_IMAGE: u32 = 0x0062_f7a0;
/// `sceGsSyncPath(mode, timeout)` (cdecl).
pub const SYNC_PATH: u32 = 0x0062_f880;
/// The VRAM resource block (`0x01301bf0`, 0x400 bytes): its first two
/// bytes are the marker `0x29, 0xff`.
pub const VRAM_RESOURCE: u32 = 0x0130_1bf0;
/// `"SG:res_clr %x %x\n"` (`0x00b5a4ac`).
pub const RES_CLR_FORMAT: u32 = 0x00b5_a4ac;
/// Bytes of [`save_vram_resource`]'s frame: the load-image record at
/// `ebp - 0x60` (16-byte aligned: the original aligns its `ebp`).
pub const SAVE_VRAM_FRAME: u32 = 0x60;

/// `Sg_SaveVramResource()` (`0x0083fd60`, cdecl; `frame` its load-image
/// record at the aligned `ebp - 0x60`): a block whose marker is not
/// `0x29, 0xff` is logged, cleared and marked; then it is loaded into
/// GS memory at 0x2efc as a 16x16 PSMCT32 image (`sceGsSetDefLoadImage`,
/// `FlushCache(0)`, `sceGsExecLoadImage`) and the path synced. Returns
/// `sceGsSyncPath`'s `eax` (the cookie check keeps it).
pub fn save_vram_resource(h: &mut impl Host, frame: u32) -> u32 {
    let block = h.rebase(VRAM_RESOURCE);
    let lo: u8 = h.read(block);
    let hi: u8 = h.read(block.wrapping_add(1));
    if lo != 0x29 || hi != 0xff {
        let format = h.rebase(RES_CLR_FORMAT);
        h.call_sym(
            DBG_PRINTF,
            Conv::Cdecl,
            &[format, u32::from(lo), u32::from(hi)],
        );
        h.call_sym(MEMCLEAR, Conv::Cdecl, &[block, 0x400]);
        h.write(block, 0xff29u16);
    }
    h.call_sym(
        SET_DEF_LOAD_IMAGE,
        Conv::Cdecl,
        &[frame, 0x2efc, 1, 0, 0, 0, 0x10, 0x10],
    );
    h.call_sym(FLUSH_CACHE, Conv::Cdecl, &[0]);
    h.call_sym(EXEC_LOAD_IMAGE, Conv::Cdecl, &[frame, block]);
    h.call_sym(SYNC_PATH, Conv::Cdecl, &[0, 0])
}

// ---- the debug file read

/// `sceOpen(path, mode)` (cdecl).
pub const SCE_OPEN: u32 = 0x0062_fa30;
/// `sceLseek(fd, offset, whence)` (cdecl).
pub const SCE_LSEEK: u32 = 0x0062_f8e0;
/// `sceRead(fd, buffer, n)` (cdecl).
pub const SCE_READ: u32 = 0x0062_fc00;
/// `sceClose(fd)` (cdecl).
pub const SCE_CLOSE: u32 = 0x0062_f610;
/// `sceGsSetDefDrawEnv2()` (cdecl).
pub const SET_DEF_DRAW_ENV2: u32 = 0x0088_7c60;
/// `user_malloc(n)` (cdecl).
pub const USER_MALLOC: u32 = 0x0068_7020;
/// `movie_err(expr, file, line)` (cdecl).
pub const MOVIE_ERR: u32 = 0x0076_efa0;
/// `"file open err !!! <NULL>\n"` (`0x00b7ae20`).
pub const OPEN_ERR_NULL: u32 = 0x00b7_ae20;
/// `"file open err !!! <%s>\n"` (`0x00b7ae3c`).
pub const OPEN_ERR_NAME: u32 = 0x00b7_ae3c;
/// `"fd != 0"` (`0x00b7ae54`).
pub const FD_EXPR: u32 = 0x00b7_ae54;
/// `"..\\program\\lib\\src\\read_debug.c"` (`0x00b7ad20`).
pub const READ_DEBUG_FILE: u32 = 0x00b7_ad20;

/// `ReadDebug_loadFile_00a42ab0(path, size_out)` (cdecl): the whole file
/// `path` read into a new `user_malloc` block (its size rounded up to 4
/// as a signed division would: `(n + 3 + (n + 3 < 0 ? 3 : 0)) & ~3`),
/// the size stored through `size_out` when set. A failed open (fd 0) is
/// logged (`movie_err`) and the reads go on with fd 0, as in the
/// original. `sceGsSetDefDrawEnv2` and `FlushCache(0)` between, as the
/// original calls them. Returns the block.
pub fn read_debug_file(h: &mut impl Host, path: u32, size_out: u32) -> u32 {
    let fd = h.call_sym(SCE_OPEN, Conv::Cdecl, &[path, 1]);
    if fd == 0 {
        if h.read::<u8>(path) == 0 {
            let s = h.rebase(OPEN_ERR_NULL);
            h.call_sym(DBG_PRINTF, Conv::Cdecl, &[s]);
        } else {
            let s = h.rebase(OPEN_ERR_NAME);
            h.call_sym(DBG_PRINTF, Conv::Cdecl, &[s, path]);
        }
        let (expr, file) = (h.rebase(FD_EXPR), h.rebase(READ_DEBUG_FILE));
        h.call_sym(MOVIE_ERR, Conv::Cdecl, &[expr, file, 0x227]);
    }
    let n = h.call_sym(SCE_LSEEK, Conv::Cdecl, &[fd, 0, 2]);
    h.call_sym(SCE_LSEEK, Conv::Cdecl, &[fd, 0, 0]);
    h.call_sym(SET_DEF_DRAW_ENV2, Conv::Cdecl, &[]);
    let padded = n.wrapping_add(3);
    let bias = if (padded as i32) < 0 { 3 } else { 0 };
    let size = padded.wrapping_add(bias) & !3;
    let block = h.call_sym(USER_MALLOC, Conv::Cdecl, &[size]);
    h.call_sym(FLUSH_CACHE, Conv::Cdecl, &[0]);
    h.call_sym(SCE_READ, Conv::Cdecl, &[fd, block, n]);
    h.call_sym(SCE_CLOSE, Conv::Cdecl, &[fd]);
    h.call_sym(FLUSH_CACHE, Conv::Cdecl, &[0]);
    if size_out != 0 {
        h.write(size_out, n);
    }
    block
}

// ---- PApplication_InitWindowAndApp

/// `Phyre_PFramework_PWindowWin32Base_createWindow(window; suffix)`
/// (thiscall, ret 4).
pub const CREATE_WINDOW: u32 = 0x0062_6cb0;
/// `Phyre_PFramework_PApplication_onInit(app; a, b)` (thiscall, ret 8).
pub const ON_INIT: u32 = 0x0062_a600;
/// `Phyre_PFramework_PApplication_onExit(app)` (thiscall).
pub const ON_EXIT: u32 = 0x0062_a130;
/// The window title suffix (`0x00b8fbeb`, an empty string).
pub const TITLE_SUFFIX: u32 = 0x00b8_fbeb;
/// `"PHYREMSG_SENDHWND"` (`0x00b40d04`).
pub const SEND_HWND: u32 = 0x00b4_0d04;
/// The window record the application points at (`0x00cca310`): the
/// `HWND` (`+0`), 1 (`+4`), the two style bytes (`+8`, `+9`).
pub const WINDOW_RECORD: u32 = 0x00cc_a310;
/// Its guard (`0x00cca31c`, bit 0: initialised).
pub const WINDOW_RECORD_GUARD: u32 = 0x00cc_a31c;
/// The window's `HWND` (`0x00cc9ce8`).
pub const HWND: u32 = 0x00cc_9ce8;
/// `g_PhyreApplication` (`0x00cc9cd8`).
pub const APPLICATION: u32 = 0x00cc_9cd8;
/// Whether the window is embedded (`0x00cc9cd2`, a byte).
pub const EMBEDDED: u32 = 0x00cc_9cd2;
/// The parent window (`0x00cc9ce0`).
pub const PARENT_HWND: u32 = 0x00cc_9ce0;
/// `SetParent`'s import slot (stdcall, 2 words).
pub const SET_PARENT_IAT: u32 = 0x00b0_c69c;
/// `RegisterWindowMessageA`'s import slot (stdcall, 1 word).
pub const REGISTER_MESSAGE_IAT: u32 = 0x00b0_c68c;
/// `SendMessageA`'s import slot (stdcall, 4 words).
pub const SEND_MESSAGE_IAT: u32 = 0x00b0_c6a4;

/// `PApplication_InitWindowAndApp_00629ab0(this; a, b)` (thiscall, ret 8):
/// the window created (`9` when that fails, negative); the window record
/// (`0x00cca310`) initialised once, then set to the `HWND`, 1 and the
/// style bytes `[[this] + 0x2c]` and `[[this] + 0x1c]`, and given to the
/// application (`+0x264`); an embedded window with a parent is
/// reparented and the parent told its `HWND` (`PHYREMSG_SENDHWND`); then
/// `onInit(a, b)`, and `onExit` when that fails. Returns `onInit`'s
/// result.
pub fn init_window_and_app(h: &mut impl Host, this: u32, a: u32, b: u32) -> u32 {
    let suffix = h.rebase(TITLE_SUFFIX);
    let r = h.call_sym(CREATE_WINDOW, Conv::Thiscall(this), &[suffix]);
    if (r as i32) < 0 {
        return 9;
    }
    let record = h.rebase(WINDOW_RECORD);
    let guard = h.rebase(WINDOW_RECORD_GUARD);
    let flags: u32 = h.read(guard);
    if flags & 1 == 0 {
        h.write(guard, flags | 1);
        h.write(record.wrapping_add(4), 1u32);
        h.write(record.wrapping_add(8), 0u16);
    }
    let hwnd: u32 = h.read(h.rebase(HWND));
    h.write(record, hwnd);
    let window: u32 = h.read(this);
    let style: u8 = h.read(window.wrapping_add(0x2c));
    h.write(record.wrapping_add(8), style);
    let window: u32 = h.read(this);
    let style: u8 = h.read(window.wrapping_add(0x1c));
    h.write(record.wrapping_add(9), style);
    let app: u32 = h.read(h.rebase(APPLICATION));
    h.write(record.wrapping_add(4), 1u32);
    h.write(app.wrapping_add(0x264), record);
    if h.read::<u8>(h.rebase(EMBEDDED)) != 0 {
        let parent: u32 = h.read(h.rebase(PARENT_HWND));
        let child: u32 = h.read(h.rebase(HWND));
        if parent != 0 {
            let set_parent: u32 = h.read(h.rebase(SET_PARENT_IAT));
            h.call(set_parent, Conv::Stdcall, &[child, parent]);
            let register: u32 = h.read(h.rebase(REGISTER_MESSAGE_IAT));
            let name = h.rebase(SEND_HWND);
            let msg = h.call(register, Conv::Stdcall, &[name]);
            let send: u32 = h.read(h.rebase(SEND_MESSAGE_IAT));
            h.call(send, Conv::Stdcall, &[parent, msg, child, 0]);
        }
    }
    let app: u32 = h.read(h.rebase(APPLICATION));
    let r = h.call_sym(ON_INIT, Conv::Thiscall(app), &[a, b]);
    if r != 0 {
        let app: u32 = h.read(h.rebase(APPLICATION));
        h.call_sym(ON_EXIT, Conv::Thiscall(app), &[]);
    }
    r
}

// ---- createVFXRenderDataBufferLine

/// `crt_memset_thunk(p, c, n)` (cdecl).
pub const MEMSET: u32 = 0x0094_964c;
/// `sprintf`'s import slot (cdecl).
pub const SPRINTF_IAT: u32 = 0x00b0_c3dc;
/// `"%s"` (`0x00b0e160`).
pub const PERCENT_S: u32 = 0x00b0_e160;
/// `"empty"` (`0x00b4ff8c`).
pub const EMPTY: u32 = 0x00b4_ff8c;
/// `graphicVFXCreate(a, b, arg)` (cdecl).
pub const VFX_CREATE: u32 = 0x0064_4de0;
/// `graphicVFXCreateDataBuffer(a, b, n, counts0, counts1, names, kinds)`
/// (cdecl).
pub const VFX_CREATE_DATA_BUFFER: u32 = 0x0064_5040;
/// Bytes of [`create_line_buffer`]'s frame, from `ebp - 0x68b4`: the
/// kinds (`+0`, a word then 0x18c bytes), the second counts (`+0x190`,
/// likewise), the first counts (`+0x320`, likewise), the names (`+0x4b0`,
/// 0x6400 bytes).
pub const LINE_FRAME: u32 = 0x68b4;

/// `createVFXRenderDataBufferLine(a, b, n, arg)` (`0x00714b60`, cdecl;
/// `frame` see [`LINE_FRAME`]): one buffer named `"empty"` of kind 1 with
/// `2 n` vertices in both counts, after `graphicVFXCreate(a, b, arg)`.
/// Returns 1.
pub fn create_line_buffer(h: &mut impl Host, frame: u32, a: u32, b: u32, n: u32, arg: u32) -> u32 {
    let kinds = frame;
    let counts1 = frame.wrapping_add(0x190);
    let counts0 = frame.wrapping_add(0x320);
    let names = frame.wrapping_add(0x4b0);
    h.call_sym(MEMSET, Conv::Cdecl, &[counts0.wrapping_add(4), 0, 0x18c]);
    h.call_sym(MEMSET, Conv::Cdecl, &[counts1.wrapping_add(4), 0, 0x18c]);
    h.write(kinds, 1u32);
    h.call_sym(MEMSET, Conv::Cdecl, &[kinds.wrapping_add(4), 0, 0x18c]);
    h.write(counts0, 0u32);
    h.write(counts1, 0u32);
    let sprintf: u32 = h.read(h.rebase(SPRINTF_IAT));
    let (format, empty) = (h.rebase(PERCENT_S), h.rebase(EMPTY));
    h.call(sprintf, Conv::Cdecl, &[names, format, empty]);
    let count = n.wrapping_add(n);
    h.write(counts0, count);
    h.write(counts1, count);
    h.call_sym(VFX_CREATE, Conv::Cdecl, &[a, b, arg]);
    h.call_sym(
        VFX_CREATE_DATA_BUFFER,
        Conv::Cdecl,
        &[a, b, 1, counts0, counts1, names, kinds],
    );
    1
}

// ---- the particle model's vertex copy

/// `get_ptr32_func(handle)` (cdecl).
pub const GET_PTR32: u32 = 0x00a4_4650;
/// `FFXVu0MulMatrix(out, a, b)` (cdecl).
pub const MUL_MATRIX: u32 = 0x006e_d5c0;
/// `g_pppScreenMatrix` (`0x0230fe60`).
pub const SCREEN_MATRIX: u32 = 0x0230_fe60;

/// `pppMdl_copyPositions_007163e0(part, job)` (cdecl, from
/// `JobSchedule_pppPart_fillBuffer_drawMdl`; `matrix` its 0x40-byte local
/// at `ebp - 0x44`): the screen matrix times the job's matrix `[job +
/// 0x20]` (and again times `[job + 0x44]` when set) into `matrix`, which
/// nothing reads after; then the model's (`get_ptr32_func([part +
/// 0x1c])`) `i16` count at `+0x12` of positions copied from `[job +
/// 0x38]` to `[job + 4]` (16-byte stride), as floats when bit 0x400 of
/// `[job]` is set (each through `fld`/`fstp`, 16-byte stride), else as
/// `i16` triples (6-byte stride) converted. The count is read again after
/// each vertex. Returns the last count read (sign-extended), or 0 when it
/// is not positive at the start.
pub fn copy_positions(h: &mut impl Host, matrix: u32, part: u32, job: u32) -> u32 {
    let src: u32 = h.read(job.wrapping_add(0x38));
    let mut dst: u32 = h.read(job.wrapping_add(4));
    let handle: u32 = h.read(part.wrapping_add(0x1c));
    let model = h.call_sym(GET_PTR32, Conv::Cdecl, &[handle]);
    let screen = h.rebase(SCREEN_MATRIX);
    let m: u32 = h.read(job.wrapping_add(0x20));
    h.call_sym(MUL_MATRIX, Conv::Cdecl, &[matrix, screen, m]);
    let m: u32 = h.read(job.wrapping_add(0x44));
    if m != 0 {
        h.call_sym(MUL_MATRIX, Conv::Cdecl, &[matrix, screen, m]);
    }
    let count_at = model.wrapping_add(0x12);
    let floats = h.read::<u32>(job) & 0x400 != 0;
    let mut n = i32::from(h.read::<i16>(count_at));
    if n <= 0 {
        return 0;
    }
    let mut i = 0;
    let mut src = src;
    while i < n {
        if floats {
            for k in 0..3 {
                let v = pass(h.read(src.wrapping_add(4 * k)));
                h.write(dst.wrapping_add(4 * k), v);
            }
            src = src.wrapping_add(0x10);
        } else {
            for k in 0..3 {
                let v = int_f32(i32::from(h.read::<i16>(src.wrapping_add(2 * k))));
                h.write(dst.wrapping_add(4 * k), v);
            }
            src = src.wrapping_add(6);
        }
        dst = dst.wrapping_add(0x10);
        i += 1;
        n = i32::from(h.read::<i16>(count_at));
    }
    n as u32
}

// ---- GDraw's pool helpers

/// `IggyGDrawSendWarning`'s import slot (cdecl, varargs).
pub const SEND_WARNING_IAT: u32 = 0x00b0_c874;
/// `IggyDiscardVertexBufferCallback`'s import slot (stdcall, 2 words).
pub const DISCARD_VERTEX_BUFFER_IAT: u32 = 0x00b0_c878;
/// `GDraw_pool_reclaim_00a3b500(pool, arg)` (cdecl): frees one resource,
/// 0 when there is none to free.
pub const POOL_RECLAIM: u32 = 0x00a3_b500;
/// `IggyHeap_takeFree_00a39a30(heap)` (cdecl).
pub const TAKE_FREE: u32 = 0x00a3_9a30;
/// `op_node_requeue_all_00a39f60(set)` (cdecl).
pub const REQUEUE_ALL: u32 = 0x00a3_9f60;
/// `GDraw_releaseTextureViews_00a34bc0(texture)` (cdecl).
pub const RELEASE_VIEWS: u32 = 0x00a3_4bc0;
/// `"GDraw Out of static vertex buffer %s"` (`0x00b7a468`).
pub const OUT_OF_VERTEX: u32 = 0x00b7_a468;
/// `"GDraw Out of texture %s"` (`0x00b7a490`).
pub const OUT_OF_TEXTURE: u32 = 0x00b7_a490;
/// `"memory (single resource larger than entire pool)"` (`0x00b7a4a8`).
pub const TOO_LARGE: u32 = 0x00b7_a4a8;
/// `"memory"` (`0x00b7a4dc`).
pub const MEMORY: u32 = 0x00b7_a4dc;
/// `"handles"` (`0x00b7a4e4`).
pub const HANDLES: u32 = 0x00b7_a4e4;
/// `"GDraw Thrashing vertex memory"` (`0x00b7a428`).
pub const THRASHING_VERTEX: u32 = 0x00b7_a428;
/// `"GDraw Thrashing texture memory"` (`0x00b7a448`).
pub const THRASHING_TEXTURE: u32 = 0x00b7_a448;

/// `IggyGDrawSendWarning(0, ...)` with `args` after the 0.
fn warn(h: &mut impl Host, args: &[u32]) {
    let send: u32 = h.read(h.rebase(SEND_WARNING_IAT));
    let mut words = vec![0u32];
    words.extend_from_slice(args);
    h.call(send, Conv::Cdecl, &words);
}

/// The pool's "out of" format: vertex buffers for bit 0 of `+0xc`, else
/// textures.
fn out_of(h: &mut impl Host, pool: u32) -> u32 {
    let vertex = h.read::<u8>(pool.wrapping_add(0xc)) & 1 != 0;
    h.rebase(if vertex {
        OUT_OF_VERTEX
    } else {
        OUT_OF_TEXTURE
    })
}

/// `opd_config_memory(pool, size, arg)` (`0x00a3b210`, cdecl): a size above
/// the pool's whole `+4` (signed) is warned about; otherwise resources are
/// reclaimed while the free amount `+0` is below the size, warning when
/// none is left. Then a handle: the first of the free ring (`[pool +
/// 0x2c]`, unless it is the ring head `pool + 0x10`) is unlinked, linked
/// at the end of its owner's (`[h + 0x18] + 0x160`) list, its state `+0x30`
/// set to 0x60000000 and `+0x14` cleared; with none, one more reclaim and
/// `IggyHeap_takeFree`, warning "handles" when that fails. Returns the
/// handle or 0.
pub fn config_memory(h: &mut impl Host, pool: u32, size: u32, arg: u32) -> u32 {
    if size as i32 > h.read::<i32>(pool.wrapping_add(4)) {
        let f = out_of(h, pool);
        let what = h.rebase(TOO_LARGE);
        warn(h, &[f, what]);
    } else {
        while h.read::<i32>(pool) < size as i32 {
            if h.call_sym(POOL_RECLAIM, Conv::Cdecl, &[pool, arg]) == 0 {
                let f = out_of(h, pool);
                let what = h.rebase(MEMORY);
                warn(h, &[f, what]);
                break;
            }
        }
    }
    let first: u32 = h.read(pool.wrapping_add(0x2c));
    let mut handle = 0;
    if first != pool.wrapping_add(0x10) {
        handle = first;
        let next: u32 = h.read(handle.wrapping_add(0x20));
        let prev: u32 = h.read(handle.wrapping_add(0x1c));
        let owner: u32 = h.read(handle.wrapping_add(0x18));
        h.write(next.wrapping_add(0x1c), prev);
        let prev: u32 = h.read(handle.wrapping_add(0x1c));
        let next: u32 = h.read(handle.wrapping_add(0x20));
        h.write(prev.wrapping_add(0x20), next);
        let head = owner.wrapping_add(0x160);
        h.write(handle.wrapping_add(0x1c), head);
        let last: u32 = h.read(head.wrapping_add(0x20));
        h.write(handle.wrapping_add(0x20), last);
        h.write(last.wrapping_add(0x1c), handle);
        let head: u32 = h.read(handle.wrapping_add(0x1c));
        h.write(head.wrapping_add(0x20), handle);
        h.write(handle.wrapping_add(0x30), 0x6000_0000u32);
        h.write(handle.wrapping_add(0x14), 0u32);
    }
    if handle == 0 && h.call_sym(POOL_RECLAIM, Conv::Cdecl, &[pool, arg]) != 0 {
        handle = h.call_sym(TAKE_FREE, Conv::Cdecl, &[pool]);
        if handle == 0 {
            let f = out_of(h, pool);
            let what = h.rebase(HANDLES);
            warn(h, &[f, what]);
        }
    }
    handle
}

/// `GDraw_pool_freeAll_00a3b380(pool, stats)` (cdecl): the pool marked
/// (`+0xc` bit 2: no thrashing warning), its nodes requeued, then every
/// resource of the used ring (`[pool + 0x64]`, until the head `pool +
/// 0x48` or 0) freed: a vertex pool's buffer discarded through Iggy, a
/// "thrashing" warning (once) for a resource used in a frame after the
/// pool's (`+0x2c:+0x28` above `+0x1a4:+0x1a0`) while the mark is clear,
/// its texture views released, the stats updated (flag 8 in the `u16`
/// at `+0`, the `u16` count at `+6` incremented, the size `[r + 0x30] &
/// 0x0fffffff` added at `+0x20` and to its owner's `[[r + 0x18]]`), and
/// it moves to the end of its owner's free ring (`[r + 0x18] + 0x10`)
/// with `+0x14` and its state cleared.
pub fn free_all(h: &mut impl Host, pool: u32, stats: u32) {
    let flags: u32 = h.read(pool.wrapping_add(0xc));
    h.write(pool.wrapping_add(0xc), flags | 2);
    h.call_sym(REQUEUE_ALL, Conv::Cdecl, &[pool]);
    let head = pool.wrapping_add(0x48);
    loop {
        let r: u32 = h.read(head.wrapping_add(0x1c));
        if r == head || r == 0 {
            break;
        }
        if h.read::<u8>(pool.wrapping_add(0xc)) & 1 != 0 {
            let buffer: u32 = h.read(r.wrapping_add(0x14));
            if buffer != 0 {
                let discard: u32 = h.read(h.rebase(DISCARD_VERTEX_BUFFER_IAT));
                h.call(discard, Conv::Stdcall, &[buffer, r]);
            }
        }
        let frame_hi: u32 = h.read(pool.wrapping_add(0x1a4));
        let used_hi: u32 = h.read(r.wrapping_add(0x2c));
        let later = if frame_hi == used_hi {
            let frame_lo: u32 = h.read(pool.wrapping_add(0x1a0));
            frame_lo < h.read::<u32>(r.wrapping_add(0x28))
        } else {
            frame_hi < used_hi
        };
        if later {
            let flags: u32 = h.read(pool.wrapping_add(0xc));
            if flags & 2 == 0 {
                let what = h.rebase(if flags & 1 != 0 {
                    THRASHING_VERTEX
                } else {
                    THRASHING_TEXTURE
                });
                warn(h, &[what]);
                let flags: u32 = h.read(pool.wrapping_add(0xc));
                h.write(pool.wrapping_add(0xc), flags | 2);
            }
        }
        h.call_sym(RELEASE_VIEWS, Conv::Cdecl, &[r]);
        let f: u16 = h.read(stats);
        h.write(stats, f | 8);
        let c: u16 = h.read(stats.wrapping_add(6));
        h.write(stats.wrapping_add(6), c.wrapping_add(1));
        let size = h.read::<u32>(r.wrapping_add(0x30)) & 0x0fff_ffff;
        let total: u32 = h.read(stats.wrapping_add(0x20));
        h.write(stats.wrapping_add(0x20), total.wrapping_add(size));
        let size = h.read::<u32>(r.wrapping_add(0x30)) & 0x0fff_ffff;
        let owner_total: u32 = h.read(r.wrapping_add(0x18));
        let t: u32 = h.read(owner_total);
        h.write(owner_total, t.wrapping_add(size));
        let next: u32 = h.read(r.wrapping_add(0x20));
        let prev: u32 = h.read(r.wrapping_add(0x1c));
        let state: u32 = h.read(r.wrapping_add(0x30));
        h.write(r.wrapping_add(0x30), state & 0xf000_0000);
        let owner: u32 = h.read(r.wrapping_add(0x18));
        h.write(r.wrapping_add(0x14), 0u32);
        h.write(next.wrapping_add(0x1c), prev);
        let prev: u32 = h.read(r.wrapping_add(0x1c));
        let next: u32 = h.read(r.wrapping_add(0x20));
        h.write(prev.wrapping_add(0x20), next);
        let free_head = owner.wrapping_add(0x10);
        h.write(r.wrapping_add(0x1c), free_head);
        let last: u32 = h.read(free_head.wrapping_add(0x20));
        h.write(r.wrapping_add(0x20), last);
        h.write(last.wrapping_add(0x1c), r);
        let free_head: u32 = h.read(r.wrapping_add(0x1c));
        h.write(free_head.wrapping_add(0x20), r);
        let state: u32 = h.read(r.wrapping_add(0x30));
        h.write(r.wrapping_add(0x30), state & 0x0fff_ffff);
    }
}
