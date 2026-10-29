//! M5 stage 1, list `small`, round 3: small leftovers of the chara,
//! movie, ps2-emu and platform lists (texture animation tag helpers,
//! character memory and history, the movie viewer's frame steps, the
//! cluster map's clear, `mspace_memalign`, thunks).

/// The originals over game memory.
#[allow(clippy::many_single_char_names, clippy::verbose_bit_mask)] // registers; the original's test
pub mod bridge {
    use ffx_graphic::host::{Conv, Host};

    fn at(p: u32, o: u32) -> u32 {
        p.wrapping_add(o)
    }

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// `TexAnim_SetQword(q, a, b, c)` (cdecl): `q = [b, c, a, 0]`,
    /// returns `a`.
    pub const SET_QWORD: u32 = 0x0077_ebc0;

    /// `FUN_0077f0e0(q, b)` (cdecl): the GS register 0x53 (`TRXDIR`)
    /// quadword `[b, 0, 0x53, 0]`. Returns `TexAnim_SetQword`'s `eax`.
    pub fn trxdir(h: &mut impl Host, q: u32, b: u32) -> u32 {
        cdecl(h, SET_QWORD, &[q, 0x53, b, 0])
    }

    /// `FUN_0077f100(q, x, y, dir)` (cdecl, tail jump): the `TRXPOS`
    /// quadword (register 0x51): `[((x << 11 | dir) << 16 | y) as u32,
    /// 0, 0x51, 0]` (the 64-bit value's low word; the original builds it
    /// in its own argument slots and jumps to `TexAnim_SetQword`).
    pub fn trxpos(h: &mut impl Host, q: u32, x: u32, y: u32, dir: u32) -> u32 {
        let v = ((u64::from(x) << 11 | u64::from(dir)) << 16) | u64::from(y);
        #[allow(clippy::cast_possible_truncation)] // the low word
        let low = v as u32;
        cdecl(h, SET_QWORD, &[q, 0x51, 0, low])
    }

    /// `FUN_0077f140(q, b, c)` (cdecl): the `TRXREG` quadword (register
    /// 0x52) `[b, c, 0x52, 0]`.
    pub fn trxreg(h: &mut impl Host, q: u32, b: u32, c: u32) -> u32 {
        cdecl(h, SET_QWORD, &[q, 0x52, b, c])
    }

    /// `g_GsRegisterSink` (`0x023056e8`).
    pub const GS_SINK: u32 = 0x0230_56e8;

    /// `FUN_0077cf80()` (cdecl): 0 when bit 14 of the GS sink word is
    /// set, else the inverse of its bit 15.
    pub fn gs_sink_ready(h: &mut impl Host) -> u32 {
        let at_ = h.rebase(GS_SINK);
        if h.read::<u32>(at_) & 0x4000 != 0 {
            return 0;
        }
        !(h.read::<u32>(at_) >> 15) & 1
    }

    /// `user_malloc(n)`, `SgRealloc(p, n)`, `Sg_MemSetStr(p, s, n)`,
    /// `Sg_MemSetMoveHeapStartAddr(p)`; the heap's name.
    pub const USER_MALLOC: u32 = 0x0068_7020;
    pub const SG_REALLOC: u32 = 0x0083_f010;
    pub const MEM_SET_STR: u32 = 0x0083_c580;
    pub const MEM_SET_MOVE_HEAP: u32 = 0x0083_c530;
    pub const CHR_HEAP_NAME: u32 = 0x00b5_94cc;

    /// `Ch_ChrMemHeapSet()` (`0x00826230`, cdecl): a 4 KB block named
    /// for the character heap (`SgRealloc(p, 0)`, `Sg_MemSetStr(p, name,
    /// -1)`) becomes the movable heap's start. Returns the last call's
    /// `eax`.
    pub fn chr_mem_heap_set(h: &mut impl Host) -> u32 {
        let p = cdecl(h, USER_MALLOC, &[0x1000]);
        cdecl(h, SG_REALLOC, &[p, 0]);
        let name = h.rebase(CHR_HEAP_NAME);
        cdecl(h, MEM_SET_STR, &[p, name, u32::MAX]);
        cdecl(h, MEM_SET_MOVE_HEAP, &[p])
    }

    /// The 40 character blocks (0x12c bytes from `0x023c8d00`, the handle
    /// first), `Ch_BlkDispose(block)`, `Sg_CacheDisposeChr(handle)`.
    pub const BLOCKS: u32 = 0x023c_8d00;
    pub const BLK_DISPOSE: u32 = 0x0082_58d0;
    pub const CACHE_DISPOSE_CHR: u32 = 0x0084_0e60;

    /// `Ch_BlkDisposeHandle(handle)` (`0x008259f0`, cdecl): the block
    /// holding `handle` disposed and its cache entries dropped. Returns
    /// the last call's `eax`, or the end of the blocks when none holds it.
    pub fn blk_dispose_handle(h: &mut impl Host, handle: u32) -> u32 {
        let base = h.rebase(BLOCKS);
        for i in 0..40 {
            let b = at(base, i * 0x12c);
            if h.read::<u32>(b) == handle {
                cdecl(h, BLK_DISPOSE, &[b]);
                return cdecl(h, CACHE_DISPOSE_CHR, &[handle]);
            }
        }
        at(base, 40 * 0x12c)
    }

    /// The character animation window's character, its title format and
    /// text, `dbgPrintf`, `Gu_WindowCreate(x, y, w, h, flags, proc)` and
    /// its procedure (`0x0093e370`).
    pub const ANIM_WIN_CHR: u32 = 0x0193_c148;
    pub const ANIM_WIN_FMT: u32 = 0x00b6_2d20;
    pub const ANIM_WIN_TEXT: u32 = 0x00b6_8f7c;
    pub const DBG_PRINTF: u32 = 0x0062_f500;
    pub const WINDOW_CREATE: u32 = 0x0084_a920;
    pub const ANIM_WIN_PROC: u32 = 0x0093_e370;

    /// `chr_anim_win(chr)` (`0x0093e510`, cdecl): remembers the character,
    /// prints the window's note and opens it (100, 50, 180 x 100, flags
    /// 0xe). Returns `Gu_WindowCreate`'s `eax`.
    pub fn chr_anim_win(h: &mut impl Host, chr: u32) -> u32 {
        h.write(h.rebase(ANIM_WIN_CHR), chr);
        let (f, t) = (h.rebase(ANIM_WIN_FMT), h.rebase(ANIM_WIN_TEXT));
        cdecl(h, DBG_PRINTF, &[f, t]);
        let p = h.rebase(ANIM_WIN_PROC);
        cdecl(h, WINDOW_CREATE, &[0x64, 0x32, 0xb4, 0x64, 0xe, p])
    }

    /// `Ch_MgrpRead(id, n)` and `Sg_SetKeepFps(v)` (cdecl).
    pub const MGRP_READ: u32 = 0x0082_a4c0;
    pub const SET_KEEP_FPS: u32 = 0x0082_1ae0;

    /// A lone `jmp target` thunk with the caller's words: the target's
    /// `eax`.
    pub fn thunk(h: &mut impl Host, target: u32, args: &[u32]) -> u32 {
        cdecl(h, target, args)
    }

    /// `ClusterMap_eraseSubtree(map; node)` (thiscall, `ret 4`) and
    /// `operator delete(p)`.
    pub const ERASE_SUBTREE: u32 = 0x0069_a2d0;
    pub const OPERATOR_DELETE: u32 = 0x0063_0390;

    /// `ClusterMap_clear_0069a970(map)` (thiscall): every node from the
    /// root down its left links (`+0`) erased (its right subtree `+8`
    /// through `0x0069a2d0`, the node deleted) until the sentinel (byte
    /// `+0xd` set); the head's links point at itself, the size `+4` 0.
    /// Returns the head (`eax`).
    pub fn cluster_map_clear(h: &mut impl Host, map: u32) -> u32 {
        let head: u32 = h.read(map);
        let mut node: u32 = h.read(at(head, 4));
        while h.read::<u8>(at(node, 0xd)) == 0 {
            let right: u32 = h.read(at(node, 8));
            h.call_sym(ERASE_SUBTREE, Conv::Thiscall(map), &[right]);
            let next: u32 = h.read(node);
            cdecl(h, OPERATOR_DELETE, &[node]);
            node = next;
        }
        let head: u32 = h.read(map);
        h.write(at(head, 4), head);
        let head: u32 = h.read(map);
        h.write(head, head);
        let head: u32 = h.read(map);
        h.write(at(head, 8), head);
        h.write(at(map, 4), 0u32);
        head
    }

    /// `internal_memalign(m, align, n)` and `memset`'s thunk.
    pub const INTERNAL_MEMALIGN: u32 = 0x006e_a5f0;
    pub const MEMSET: u32 = 0x0094_964c;

    /// `mspace_memalign(m, align, n)` (`0x006ec8d0`, cdecl): the block,
    /// its usable bytes (the chunk size less 4, or 8 when the chunk is
    /// mmapped: both in-use bits clear) filled with 0xcd. Returns it.
    pub fn mspace_memalign(h: &mut impl Host, m: u32, align: u32, n: u32) -> u32 {
        let p = cdecl(h, INTERNAL_MEMALIGN, &[m, align, n]);
        let head: u32 = h.read(p.wrapping_sub(4));
        let overhead = if head & 3 == 0 { 8 } else { 4 };
        let size = head & !7;
        if size > overhead {
            cdecl(h, MEMSET, &[p, 0xcd, size - overhead]);
        }
        p
    }

    /// `GlobalMemoryStatusEx`'s import slot (stdcall, one word).
    pub const GLOBAL_MEMORY_STATUS_EX: u32 = 0x00b0_c074;

    /// `FUN_009437c0()` (cdecl): `GlobalMemoryStatusEx` on a
    /// `MEMORYSTATUSEX` in its frame (`buf`, `ebp - 0x44`, 0x40 bytes,
    /// `dwLength` 0x40); the result is not used. Returns the API's `eax`.
    pub fn memory_status(h: &mut impl Host, buf: u32) -> u32 {
        h.write(buf, 0x40u32);
        let f: u32 = h.read(h.rebase(GLOBAL_MEMORY_STATUS_EX));
        h.call(f, Conv::Stdcall, &[buf])
    }

    /// `movie_err` and the HDD read's error strings.
    pub const MOVIE_ERR: u32 = 0x0076_efa0;
    pub const READ_ERR_NONAME: u32 = 0x00b7_ae20;
    pub const READ_ERR_NAME: u32 = 0x00b7_ae3c;
    pub const READ_ERR_MSG: u32 = 0x00b7_ae54;
    pub const READ_ERR_FILE: u32 = 0x00b7_ad20;

    /// `FUN_00a42f50(ok, name)` (cdecl): unless `ok`, prints which file
    /// failed (or that it had no name) and stops through `movie_err`.
    pub fn read_check(h: &mut impl Host, ok: u32, name: u32) {
        if ok != 0 {
            return;
        }
        if h.read::<u8>(name) == 0 {
            let f = h.rebase(READ_ERR_NONAME);
            cdecl(h, DBG_PRINTF, &[f]);
        } else {
            let f = h.rebase(READ_ERR_NAME);
            cdecl(h, DBG_PRINTF, &[f, name]);
        }
        let (m, f) = (h.rebase(READ_ERR_MSG), h.rebase(READ_ERR_FILE));
        cdecl(h, MOVIE_ERR, &[m, f, 0x227]);
    }

    /// The character function history (128 entries of 16 bytes from
    /// `0x012fbda8`, the next index at `0x012fbda4`), its header and line
    /// formats, and `Sg_GetFuncName(f)`.
    pub const HISTORY: u32 = 0x012f_bda8;
    pub const HISTORY_NEXT: u32 = 0x012f_bda4;
    pub const HISTORY_HEADER: u32 = 0x00b5_95d4;
    pub const HISTORY_LINE: u32 = 0x00b5_9604;
    pub const GET_FUNC_NAME: u32 = 0x0084_56c0;

    /// `Ch_FunctionHistoryDump()` (`0x008269e0`, cdecl): the history
    /// printed newest first (up to 127 entries, until an empty one): its
    /// word `+0xc`, the entry, the function's name. Returns the last
    /// `dbgPrintf`'s `eax`.
    pub fn function_history_dump(h: &mut impl Host) -> u32 {
        let mut k = h.read::<u32>(h.rebase(HISTORY_NEXT)).wrapping_sub(1);
        let f = h.rebase(HISTORY_HEADER);
        let mut r = cdecl(h, DBG_PRINTF, &[f]);
        let base = h.rebase(HISTORY);
        for _ in 0..0x7f {
            let e = at(base, (k & 0x7f) << 4);
            if h.read::<u8>(e) == 0 {
                break;
            }
            let fun: u32 = h.read(at(e, 8));
            let name = cdecl(h, GET_FUNC_NAME, &[fun]);
            let w: u32 = h.read(at(e, 0xc));
            let f = h.rebase(HISTORY_LINE);
            r = cdecl(h, DBG_PRINTF, &[f, w, e, name]);
            k = k.wrapping_sub(1);
        }
        r
    }

    /// The movie viewer's state (`0x00c421b0`, a pointer): `+0x2ddc0` the
    /// PCM buffer, `+0x5051c` the width, `+0x5059c` the stream cursor,
    /// `+0x506c8` the audio mode, `+0x5076d` "camera set".
    pub const VIEWER: u32 = 0x00c4_21b0;
    pub const DECODE_BLOCKS: u32 = 0x0077_afc0;
    pub const MOVIE_CAMERA: u32 = 0x00a4_3e60;

    /// `code_0077c360()` (cdecl; not a function in the dump): the viewer's
    /// audio block decoded (`AudioDecodeBlocks_0077afc0(cursor + 0x10,
    /// pcm, b, viewer)`, `b` 0 in mode 0, `cursor + 0x1c40` in mode 1).
    /// Returns its `eax`, or `mode - 1` for another mode.
    pub fn viewer_audio(h: &mut impl Host) -> u32 {
        let d: u32 = h.read(h.rebase(VIEWER));
        let mode: u32 = h.read(at(d, 0x5_06c8));
        let cursor: u32 = h.read(at(d, 0x5_059c));
        let pcm = at(d, 0x2_ddc0);
        match mode {
            0 => cdecl(h, DECODE_BLOCKS, &[at(cursor, 0x10), pcm, 0, d]),
            1 => cdecl(
                h,
                DECODE_BLOCKS,
                &[at(cursor, 0x10), pcm, at(cursor, 0x1c40), d],
            ),
            _ => mode.wrapping_sub(1),
        }
    }

    /// `code_0077c470()` (cdecl; not a function in the dump): the
    /// viewer's next stream record (`[cursor] + 0x10` bytes; the cursor
    /// moves past it); a record with a body is the camera, given to
    /// `0x00a43e60(record + 0x10, width, 0x1c0, viewer)` and the camera
    /// byte set. Returns the new cursor, or the viewer.
    pub fn viewer_camera(h: &mut impl Host) -> u32 {
        let d: u32 = h.read(h.rebase(VIEWER));
        let cursor: u32 = h.read(at(d, 0x5_059c));
        let n = h.read::<u32>(cursor).wrapping_add(0x10);
        let next = at(cursor, n);
        h.write(at(d, 0x5_059c), next);
        if n == 0x10 {
            return next;
        }
        let w: u32 = h.read(at(d, 0x5_051c));
        cdecl(h, MOVIE_CAMERA, &[at(cursor, 0x10), w, 0x1c0, d]);
        let d: u32 = h.read(h.rebase(VIEWER));
        h.write(at(d, 0x5_076d), 1u8);
        d
    }
}
