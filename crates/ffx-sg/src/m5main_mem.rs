//! M5 `main`: the `Sg_Mem` heaps (set-up, a heap's first and end blocks,
//! freeing and merging, moving a block down into free space), the
//! `Sg` allocation wrappers, the debug file list (a host file read whole
//! and split into lines), and small getters and setters the natives and
//! the debug menus reach.
//!
//! A heap block has a 16-byte header: `+0` the next block, `+4` the
//! previous one, `+8` a word, `+0xc` flags: bits 0..3 the state (0 free,
//! 0xf the end block), bits 27..28 the heap. The heaps are 0x18-byte
//! records at [`HEAPS`] (`g_SgMemHeaps`).

/// The originals over game memory.
#[allow(clippy::many_single_char_names, clippy::similar_names)] // registers
pub mod bridge {
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// `user_malloc(size)`, `user_free_thunk(p)`, `user_realloc(p, size)`,
    /// `user_aligned_malloc(size, align)` (cdecl).
    pub const USER_MALLOC: u32 = 0x0068_7020;
    pub const USER_FREE: u32 = 0x0068_7010;
    pub const USER_REALLOC: u32 = 0x0068_7060;
    pub const USER_ALIGNED_MALLOC: u32 = 0x0068_6ff0;
    /// `dbgPrintf(fmt, ...)` (cdecl).
    pub const DBG_PRINTF: u32 = 0x0062_f500;

    /// `Sg_MemRealloc(heap, kind, p, size)` (`0x0083c480`, cdecl): the PC
    /// ignores the heap and kind: `user_realloc(p, size)`'s `eax`.
    pub fn mem_realloc(h: &mut impl Host, p: u32, size: u32) -> u32 {
        h.call_sym(USER_REALLOC, Conv::Cdecl, &[p, size])
    }

    /// `Sg_MemRealloc` (cdecl).
    pub const MEM_REALLOC: u32 = 0x0083_c480;

    /// `SgRealloc(p, size)` (`0x0083f010`, heap 0) and `SgReallocD(p,
    /// size)` (`0x0083f030`, heap 1): `Sg_MemRealloc(heap, 6, p, size)`'s
    /// `eax`.
    pub fn sg_realloc(h: &mut impl Host, heap: u32, p: u32, size: u32) -> u32 {
        h.call_sym(MEM_REALLOC, Conv::Cdecl, &[heap, 6, p, size])
    }

    /// `FUN_0083f100(size)`: a frame, then a tail jump to `user_malloc`
    /// with the caller's word; its `eax`.
    pub fn malloc_thunk(h: &mut impl Host, size: u32) -> u32 {
        h.call_sym(USER_MALLOC, Conv::Cdecl, &[size])
    }

    /// `FUN_0083f100` (cdecl).
    pub const MALLOC_THUNK: u32 = 0x0083_f100;

    /// `SgStrdupD(s)` (`0x0083f050`, cdecl): a copy of `s` (with its 0)
    /// from `FUN_0083f100`. Returns it.
    pub fn strdup(h: &mut impl Host, s: u32) -> u32 {
        let mut len = 0u32;
        while h.read::<u8>(s.wrapping_add(len)) != 0 {
            len += 1;
        }
        let p = h.call_sym(MALLOC_THUNK, Conv::Cdecl, &[len + 1]);
        let mut k = 0u32;
        loop {
            let c: u8 = h.read(s.wrapping_add(k));
            h.write(p.wrapping_add(k), c);
            if c == 0 {
                break;
            }
            k += 1;
        }
        p
    }

    /// `sceOpen(name, mode)`, `sceLseek(fd, off, whence)`, `sceRead(fd,
    /// p, n)`, `sceClose(fd)`, `FlushCache(mode)` (cdecl).
    pub const SCE_OPEN: u32 = 0x0062_fa30;
    pub const SCE_LSEEK: u32 = 0x0062_f8e0;
    pub const SCE_READ: u32 = 0x0062_fc00;
    pub const SCE_CLOSE: u32 = 0x0062_f610;
    pub const FLUSH_CACHE: u32 = 0x0062_f4c0;

    /// `FUN_0084f1c0(name, out_size)` (cdecl): the host file read whole
    /// into a `user_malloc` buffer (its size rounded up to 4, toward zero
    /// for a negative size), the size to `*out_size` when given. Returns
    /// the buffer.
    pub fn read_whole_file(h: &mut impl Host, name: u32, out_size: u32) -> u32 {
        let fd = h.call_sym(SCE_OPEN, Conv::Cdecl, &[name, 1]);
        let size = h.call_sym(SCE_LSEEK, Conv::Cdecl, &[fd, 0, 2]);
        h.call_sym(SCE_LSEEK, Conv::Cdecl, &[fd, 0, 0]);
        let n = size.wrapping_add(3).cast_signed();
        let rounded = (n / 4 * 4).cast_unsigned();
        let buf = h.call_sym(USER_MALLOC, Conv::Cdecl, &[rounded]);
        h.call_sym(FLUSH_CACHE, Conv::Cdecl, &[0]);
        h.call_sym(SCE_READ, Conv::Cdecl, &[fd, buf, size]);
        h.call_sym(SCE_CLOSE, Conv::Cdecl, &[fd]);
        h.call_sym(FLUSH_CACHE, Conv::Cdecl, &[0]);
        if out_size != 0 {
            h.write(out_size, size);
        }
        buf
    }

    /// `MSVCR110!strchr`'s import slot (cdecl).
    pub const STRCHR_IAT: u32 = 0x00b0_c3c0;
    /// The extension the filtered list keeps (`.rdata`).
    pub const STR_EXTENSION: u32 = 0x00b5_abcc;
    /// `SgStrdupD` (cdecl).
    pub const STRDUP: u32 = 0x0083_f050;
    /// `FUN_00850590`'s frame below `ebp`: the line buffer at `ebp -
    /// 0x84` (0x80 bytes), then the cookie.
    pub const SPLIT_FRAME: u32 = 0x84;

    /// `FUN_00850590(text, out, filter)` (cdecl, with its frame at
    /// `ebp`): splits `text` in place at line feeds (carriage returns
    /// become 0 and the scan goes on to the feed or the end), copies each
    /// line to the frame's buffer and keeps a `SgStrdupD` copy in
    /// `out[n++]`; with `filter`, only lines whose first `.` starts the
    /// kept extension, cut there. After a line that ends the text the
    /// scan reads one byte past its 0, as the original does. Returns `n`.
    pub fn split_lines(h: &mut impl Host, ebp: u32, text: u32, out: u32, filter: u32) -> u32 {
        let buf = ebp - SPLIT_FRAME;
        let mut n = 0u32;
        let mut p = text;
        if h.read::<u8>(p) == 0 {
            return 0;
        }
        loop {
            let line = p;
            while h.read::<u8>(p) != 0 {
                match h.read::<u8>(p) {
                    0xd => h.write(p, 0u8),
                    0xa => {
                        h.write(p, 0u8);
                        break;
                    }
                    _ => {}
                }
                p += 1;
            }
            let mut k = 0u32;
            loop {
                let c: u8 = h.read(line + k);
                h.write(buf + k, c);
                if c == 0 {
                    break;
                }
                k += 1;
            }
            if filter == 0 {
                let s = h.call_sym(STRDUP, Conv::Cdecl, &[buf]);
                h.write(out.wrapping_add(4 * n), s);
                n += 1;
            } else {
                let f: u32 = h.read(h.rebase(STRCHR_IAT));
                let dot = h.call(f, Conv::Cdecl, &[buf, u32::from(b'.')]);
                if dot != 0 && same_string(h, dot, h.rebase(STR_EXTENSION)) {
                    h.write(dot, 0u8);
                    let s = h.call_sym(STRDUP, Conv::Cdecl, &[buf]);
                    h.write(out.wrapping_add(4 * n), s);
                    n += 1;
                }
            }
            p += 1;
            if h.read::<u8>(p) == 0 {
                return n;
            }
        }
    }

    /// The inlined `strcmp(a, b) == 0`.
    fn same_string(h: &impl Host, a: u32, b: u32) -> bool {
        let mut k = 0;
        loop {
            let x: u8 = h.read(a + k);
            if x != h.read::<u8>(b + k) {
                return false;
            }
            if x == 0 {
                return true;
            }
            k += 1;
        }
    }

    /// `MSVCR110!sprintf`'s import slot (cdecl), `memcpy`'s import thunk
    /// (cdecl).
    pub const SPRINTF_IAT: u32 = 0x00b0_c3dc;
    pub const MEMCPY: u32 = 0x0094_9256;
    /// `sg_make_filelist`'s strings: the two log lines, the list file,
    /// the probe's format and its two words.
    pub const MSG_FILELIST_PATH: u32 = 0x00b5_afcc;
    pub const MSG_FILELIST_OPEN: u32 = 0x00b5_afe4;
    pub const STR_FILELIST_FILE: u32 = 0x00b5_b004;
    pub const FMT_FILELIST_PROBE: u32 = 0x00b5_b018;
    pub const STR_FILELIST_PROBE_ARG: u32 = 0x00b5_aadc;
    /// `FUN_0084f1c0`, `FUN_00850590` (cdecl).
    pub const READ_WHOLE_FILE: u32 = 0x0084_f1c0;
    pub const SPLIT_LINES: u32 = 0x0085_0590;
    /// `sg_make_filelist`'s frame below `ebp`: the probe name at `ebp -
    /// 0x44` (0x40 bytes) and the file size at `ebp - 0x48`.
    pub const FILELIST_FRAME: u32 = 0x48;

    /// `sg_make_filelist(list, path, filter)` (`0x008509e0`, cdecl, with
    /// its frame at `ebp`): a 64 KB text buffer filled from the list file
    /// (when it is there and smaller), the host probes logged; the old
    /// list's strings and array freed; the array sized by the line feeds
    /// and zeroed; `list[0]` = `FUN_00850590(text, array, filter)`; the
    /// text freed. Returns that free's `eax`.
    pub fn make_filelist(h: &mut impl Host, ebp: u32, list: u32, path: u32, filter: u32) -> u32 {
        let size_at = ebp - 0x48;
        let name = ebp - 0x44;
        h.write(size_at, 0u32);
        let text = h.call_sym(USER_MALLOC, Conv::Cdecl, &[0x1_0000]);
        h.write(text, 0u8);
        let msg = h.rebase(MSG_FILELIST_PATH);
        h.call_sym(DBG_PRINTF, Conv::Cdecl, &[msg, path]);
        let fd = h.call_sym(SCE_OPEN, Conv::Cdecl, &[path, 0]);
        let msg = h.rebase(MSG_FILELIST_OPEN);
        h.call_sym(DBG_PRINTF, Conv::Cdecl, &[msg, fd]);
        let file = h.rebase(STR_FILELIST_FILE);
        let data = h.call_sym(READ_WHOLE_FILE, Conv::Cdecl, &[file, size_at]);
        if data != 0 {
            let size: u32 = h.read(size_at);
            if size.cast_signed() < 0x1_0000 {
                h.call_sym(MEMCPY, Conv::Cdecl, &[text, data, size]);
                h.write(text.wrapping_add(size), 0u8);
                h.call_sym(USER_FREE, Conv::Cdecl, &[data]);
            }
        }
        let f: u32 = h.read(h.rebase(SPRINTF_IAT));
        let fmt = h.rebase(FMT_FILELIST_PROBE);
        let arg = h.rebase(STR_FILELIST_PROBE_ARG);
        h.call(f, Conv::Cdecl, &[name, fmt, arg, arg]);
        h.call_sym(SCE_OPEN, Conv::Cdecl, &[name, 0]);
        if h.read::<u32>(list + 8) != 0 {
            let mut i = 0u32;
            while i.cast_signed() < h.read::<u32>(list + 4).cast_signed() {
                let arr: u32 = h.read(list + 8);
                let s: u32 = h.read(arr.wrapping_add(4 * i));
                if s != 0 {
                    h.call_sym(USER_FREE, Conv::Cdecl, &[s]);
                }
                i += 1;
            }
            let arr: u32 = h.read(list + 8);
            h.call_sym(USER_FREE, Conv::Cdecl, &[arr]);
            h.write(list + 8, 0u32);
            h.write(list + 4, 0u32);
        }
        let mut lines = 0u32;
        let mut p = text;
        loop {
            let c: u8 = h.read(p);
            if c == 0 {
                break;
            }
            if c == 0xa {
                lines += 1;
            }
            p += 1;
        }
        h.write(list + 4, lines);
        let arr = h.call_sym(USER_MALLOC, Conv::Cdecl, &[lines.wrapping_mul(4)]);
        h.write(list + 8, arr);
        let mut i = 0u32;
        while i.cast_signed() < lines.cast_signed() {
            let arr: u32 = h.read(list + 8);
            h.write(arr.wrapping_add(4 * i), 0u32);
            i += 1;
        }
        let arr: u32 = h.read(list + 8);
        let n = h.call_sym(SPLIT_LINES, Conv::Cdecl, &[text, arr, filter]);
        h.write(list, n);
        h.call_sym(USER_FREE, Conv::Cdecl, &[text])
    }

    /// `g_SqsCmdNext`, the queue's limit, the debug flags and the
    /// queue-full line; `SqsIopCommandSend()` (cdecl).
    pub const SQS_CMD_NEXT: u32 = 0x00c4_946c;
    pub const SQS_CMD_LIMIT: u32 = 0x012f_4e00;
    pub const SQS_DEBUG_FLAGS: u32 = 0x012f_4148;
    pub const MSG_SQS_FULL: u32 = 0x00b5_8d28;
    pub const SQS_SEND: u32 = 0x0081_d5f0;

    /// `FUN_0081fc30(a, b)` (cdecl): IOP command 0x3b `(a, b & 0x7f)`
    /// appended to the queue (sent first when it would pass the limit,
    /// logged when the debug flag is set). Returns `a`.
    pub fn sqs_command_3b(h: &mut impl Host, a: u32, b: u32) -> u32 {
        let next_at = h.rebase(SQS_CMD_NEXT);
        let mut at: u32 = h.read(next_at);
        if at.wrapping_add(0x20) > h.rebase(SQS_CMD_LIMIT) {
            if h.read::<u8>(h.rebase(SQS_DEBUG_FLAGS)) & 1 != 0 {
                let msg = h.rebase(MSG_SQS_FULL);
                h.call_sym(DBG_PRINTF, Conv::Cdecl, &[msg]);
            }
            h.call_sym(SQS_SEND, Conv::Cdecl, &[]);
            at = h.read(next_at);
        }
        h.write(at, [0x3b, a, b & 0x7f, 0, 0, 0, 0, 0u32]);
        h.write(next_at, at.wrapping_add(0x20));
        a
    }

    /// `Sg_SetResource(kind, value)` (cdecl).
    pub const SET_RESOURCE: u32 = 0x0083_fe10;
    /// The byte `FUN_00821e40` toggles.
    pub const RESOURCE9_TOGGLE: u32 = 0x012f_bbb9;

    /// `FUN_0083ff00(on)` (cdecl): `Sg_SetResource(9, 1 - on)`'s `eax`.
    pub fn set_resource9(h: &mut impl Host, on: u32) -> u32 {
        h.call_sym(SET_RESOURCE, Conv::Cdecl, &[9, 1u32.wrapping_sub(on)])
    }

    /// `FUN_0083ff00` (cdecl).
    pub const SET_RESOURCE9: u32 = 0x0083_ff00;

    /// `FUN_00821e40()` (cdecl): the toggle byte flipped (`(b - 1) & 1`),
    /// passed on sign-extended. Returns `1 - toggle` (read again).
    pub fn toggle_resource9(h: &mut impl Host) -> u32 {
        let at = h.rebase(RESOURCE9_TOGGLE);
        let b = h.read::<u8>(at).wrapping_sub(1) & 1;
        h.write(at, b);
        h.call_sym(SET_RESOURCE9, Conv::Cdecl, &[u32::from(b)]);
        let b = i32::from(h.read::<i8>(at));
        1u32.wrapping_sub(b.cast_unsigned())
    }

    /// The word `code_00821740` returns.
    pub const WORD_012FBC3C: u32 = 0x012f_bc3c;

    /// `code_00821740()`: the word at `0x012fbc3c`.
    pub fn get_012fbc3c(h: &mut impl Host) -> u32 {
        h.read(h.rebase(WORD_012FBC3C))
    }

    /// `g_SgMemHeaps`: 0x18 bytes per heap: `+0` the freed byte, `+1` a
    /// byte, `+4` the heap's id, `+8` its size, `+0xc` its first block,
    /// `+0x10` its end block, `+0x14` the lowest freed block.
    pub const HEAPS: u32 = 0x0130_1968;
    /// The heaps' area pointer and a word `FUN_0083c200` clears.
    pub const HEAP_AREA: u32 = 0x0130_1960;
    pub const HEAP_WORD: u32 = 0x0130_1964;
    /// `Sg_MemInit`'s log line.
    pub const MSG_MEM_INIT: u32 = 0x00b5_9f38;

    /// `Sg_MemInit(id, start, end)` (`0x0083c260`, cdecl): heap `id` over
    /// `[start & ~3, end & ~3)`: the first block at the start (free, next
    /// the end block), the end block 16 bytes before the end (state 0xf,
    /// previous the first), the heap's record filled. Returns the first
    /// block.
    pub fn mem_init(h: &mut impl Host, id: u32, start: u32, end: u32) -> u32 {
        let msg = h.rebase(MSG_MEM_INIT);
        h.call_sym(DBG_PRINTF, Conv::Cdecl, &[msg, start, end]);
        let r = h.rebase(HEAPS).wrapping_add(id.wrapping_mul(0x18));
        let first = start & !3;
        let end = end & !3;
        let bits = (id & 3) << 27;
        h.write(r + 4, id);
        h.write(r + 0xc, first);
        h.write(r + 0x10, end.wrapping_sub(0x10));
        h.write(r + 8, end.wrapping_sub(first));
        h.write(first, [0u32; 4]);
        let last: u32 = h.read(r + 0x10);
        h.write(first, last);
        let f: u32 = h.read(first + 0xc);
        h.write(first + 4, 0u32);
        h.write(first + 0xc, (f & 0xe7ff_fff0) | bits);
        let last: u32 = h.read(r + 0x10);
        h.write(last + 8, 0u32);
        h.write(last, 0u32);
        h.write(last + 4, first);
        h.write(last + 0xc, bits | 0xf);
        let first: u32 = h.read(r + 0xc);
        h.write(r + 0x14, first);
        h.write(r, 0x100u16);
        first
    }

    /// `Sg_MemSetCheck(on)`, `Sg_MemSetFast(on)`,
    /// `Sg_MemSetMoveHeapStartAddr(p)`, `Sg_MemInit` (cdecl).
    pub const SET_CHECK: u32 = 0x0083_c4d0;
    pub const SET_FAST: u32 = 0x0083_c520;
    pub const SET_MOVE_START: u32 = 0x0083_c530;
    pub const MEM_INIT: u32 = 0x0083_c260;

    /// `FUN_0083c200()` (cdecl): heap 0 over a fresh 17 MB aligned block
    /// (the two heaps' bytes and a word cleared, checks on, fast off),
    /// its first block as the move area's start; that call's `eax`.
    pub fn mem_setup(h: &mut impl Host) -> u32 {
        let heaps = h.rebase(HEAPS);
        h.write(heaps + 1, 0u8);
        h.write(heaps + 0x19, 0u8);
        h.write(h.rebase(HEAP_WORD), 0u32);
        h.call_sym(SET_CHECK, Conv::Cdecl, &[1]);
        h.call_sym(SET_FAST, Conv::Cdecl, &[0]);
        let p = h.call_sym(USER_ALIGNED_MALLOC, Conv::Cdecl, &[0x110_0000, 0x10]);
        h.write(h.rebase(HEAP_AREA), p);
        h.call_sym(MEM_INIT, Conv::Cdecl, &[0, p, p.wrapping_add(0x110_0000)]);
        let first: u32 = h.read(heaps + 0xc);
        h.call_sym(SET_MOVE_START, Conv::Cdecl, &[first])
    }

    /// `achievementUnlockAchievement(id)` (cdecl).
    pub const UNLOCK_ACHIEVEMENT: u32 = 0x0082_2380;
    /// The monsters whose defeat unlocks an achievement, with it.
    pub const DEFEAT_ACHIEVEMENTS: [(u32, u32); 5] = [
        (0x114, 0x1f),
        (0x158, 0x1e),
        (0x82, 0xa),
        (0xb0, 0x20),
        (0x83, 0xb),
    ];

    /// `trophyCheckDefeatMonster(monster)` (`0x00822760`, cdecl): the
    /// achievement for each matching entry of [`DEFEAT_ACHIEVEMENTS`]
    /// (kept in its frame; nothing else sees it). Its `eax` is junk.
    pub fn trophy_check_defeat_monster(h: &mut impl Host, monster: u32) {
        for (m, a) in DEFEAT_ACHIEVEMENTS {
            if m == monster {
                h.call_sym(UNLOCK_ACHIEVEMENT, Conv::Cdecl, &[a]);
            }
        }
    }

    fn state(h: &impl Host, block: u32) -> u8 {
        h.read::<u8>(block + 0xc) & 0xf
    }

    /// `FUN_0083c5b0(block)` (cdecl): a free block merged with the free
    /// blocks before and after it (the first of the run takes the rest).
    /// Returns the merged block, or `block` when it is in use.
    pub fn merge_free(h: &mut impl Host, block: u32) -> u32 {
        let mut b = block;
        if state(h, b) != 0 {
            return b;
        }
        loop {
            let prev: u32 = h.read(b + 4);
            if state(h, prev) != 0 {
                break;
            }
            b = prev;
        }
        if state(h, b) == 0 {
            loop {
                let next: u32 = h.read(b);
                if state(h, next) != 0 {
                    break;
                }
                let after: u32 = h.read(next);
                h.write(b, after);
            }
        }
        let next: u32 = h.read(b);
        h.write(next + 4, b);
        b
    }

    /// `Sg_MemSetFree(block)`, `FUN_0083c5b0(block)` (cdecl).
    pub const SET_FREE: u32 = 0x0083_bdb0;
    pub const MERGE_FREE: u32 = 0x0083_c5b0;

    /// `Sg_MemCreateFree(heap, block, prev, next)` (`0x0083baf0`, cdecl):
    /// a free block of `heap` linked in between, freed and merged, the
    /// heap's freed byte cleared. Returns `heap * 3`.
    pub fn create_free(h: &mut impl Host, heap: u32, block: u32, prev: u32, next: u32) -> u32 {
        h.write(block, [0u32; 4]);
        let f: u32 = h.read(block + 0xc);
        h.write(block + 4, prev);
        h.write(block + 0xc, f ^ ((heap << 27 ^ f) & 0x1800_0000));
        h.write(block, next);
        h.write(prev, block);
        let next: u32 = h.read(block);
        h.write(next + 4, block);
        h.call_sym(SET_FREE, Conv::Cdecl, &[block]);
        h.call_sym(MERGE_FREE, Conv::Cdecl, &[block]);
        let k = heap.wrapping_mul(3);
        h.write(h.rebase(HEAPS).wrapping_add(k.wrapping_mul(8)), 0u8);
        k
    }

    /// The block `Sg_MemMoveWork` starts its search at.
    pub const MOVE_START: u32 = 0x0130_19b8;
    /// `MSVCR110!memmove`'s import slot (cdecl).
    pub const MEMMOVE_IAT: u32 = 0x00b0_c34c;
    /// `Sg_MemCreateFree` (cdecl).
    pub const CREATE_FREE: u32 = 0x0083_baf0;

    /// `Sg_MemMoveWork(pp, moved)` (`0x0083c350`, cdecl): moves the block
    /// `*pp` points into (its header 16 bytes before) down: into the
    /// first free block from [`MOVE_START`] below its previous block
    /// (signed address compare) that is larger than it, or else into its
    /// free previous block. The rest becomes a free block; `*pp` follows
    /// and `*moved` grows by the block's size. Returns how far `*pp`
    /// moved (0 when nothing moved or `*pp` is 0).
    pub fn move_work(h: &mut impl Host, pp: u32, moved: u32) -> u32 {
        let start: u32 = h.rebase(MOVE_START);
        let mut scan: u32 = h.read(start);
        let data: u32 = h.read(pp);
        if data == 0 {
            return 0;
        }
        let block = data.wrapping_sub(0x10);
        let size = h.read::<u32>(block).wrapping_sub(block).wrapping_sub(0x10);
        let memmove: u32 = h.read(h.rebase(MEMMOVE_IAT));
        if state(h, scan) != 0xf {
            let prev: u32 = h.read(block + 4);
            while scan.cast_signed() < prev.cast_signed() {
                if state(h, scan) == 0 {
                    let room = h.read::<u32>(scan).wrapping_sub(scan).wrapping_sub(0x10);
                    if room > size {
                        let next: u32 = h.read(scan);
                        let before: u32 = h.read(scan + 4);
                        h.call(memmove, Conv::Cdecl, &[scan, block, size + 0x10]);
                        h.write(pp, scan + 0x10);
                        h.call_sym(SET_FREE, Conv::Cdecl, &[data.wrapping_sub(0x10)]);
                        let rest = size.wrapping_add(0x10).wrapping_add(scan);
                        h.write(scan + 4, before);
                        h.write(scan, rest);
                        h.call_sym(CREATE_FREE, Conv::Cdecl, &[0, rest, scan, next]);
                        h.call_sym(MERGE_FREE, Conv::Cdecl, &[data.wrapping_sub(0x10)]);
                        let delta = h.read::<u32>(pp).wrapping_sub(data);
                        return finish(h, moved, size, delta);
                    }
                }
                scan = h.read(scan);
                if state(h, scan) == 0xf {
                    break;
                }
            }
        }
        let prev: u32 = h.read(block + 4);
        if state(h, prev) != 0 {
            return 0;
        }
        let into = h.call_sym(MERGE_FREE, Conv::Cdecl, &[prev]);
        let block = data.wrapping_sub(0x10);
        let next: u32 = h.read(block);
        let before: u32 = h.read(into + 4);
        h.call(memmove, Conv::Cdecl, &[into, block, size + 0x10]);
        h.write(pp, into + 0x10);
        let rest = size.wrapping_add(0x10).wrapping_add(into);
        h.write(into + 4, before);
        h.write(into, rest);
        h.call_sym(CREATE_FREE, Conv::Cdecl, &[0, rest, into, next]);
        let delta = h.read::<u32>(pp).wrapping_sub(data);
        finish(h, moved, size, delta)
    }

    fn finish(h: &mut impl Host, moved: u32, size: u32, delta: u32) -> u32 {
        if moved != 0 {
            let m: u32 = h.read(moved);
            h.write(moved, m.wrapping_add(size));
        }
        delta
    }

    /// The halfword the event natives clear and read.
    pub const HALF_0130074C: u32 = 0x0130_074c;

    /// `code_0082cc00()`: the halfword cleared; returns 0.
    pub fn clear_half_0130074c(h: &mut impl Host) -> u32 {
        h.write(h.rebase(HALF_0130074C), 0u16);
        0
    }

    /// `code_0082cc10()`: the halfword, sign-extended.
    pub fn get_half_0130074c(h: &mut impl Host) -> u32 {
        i32::from(h.read::<i16>(h.rebase(HALF_0130074C))).cast_unsigned()
    }

    /// The camera flags byte (`0x00c496d5`).
    pub const CAMERA_FLAGS: u32 = 0x00c4_96d5;

    /// `Sg_GetCameraFollow_0082d850()`: bit 1 of the camera flags.
    pub fn get_camera_follow(h: &mut impl Host) -> u32 {
        (i32::from(h.read::<i8>(h.rebase(CAMERA_FLAGS))).cast_unsigned() & 2) >> 1
    }

    /// `MsCameraGetYaw()` (cdecl, `ST0`).
    pub const MS_CAMERA_GET_YAW: u32 = 0x007b_cf20;
    /// `Sg_CameraGetYaw` (cdecl, `ST0`).
    pub const CAMERA_GET_YAW: u32 = 0x0083_f2d0;
    /// Where `code_0082d930` keeps the yaw, and the word it zeroes.
    pub const YAW_CELL: u32 = 0x0130_079c;
    pub const YAW_ZERO: u32 = 0x0130_0780;

    /// `Sg_CameraGetYaw()` (`0x0083f2d0`): `jmp MsCameraGetYaw`; its `ST0`.
    pub fn camera_get_yaw(h: &mut impl Host) -> f64 {
        h.call_sym_st0_f64(MS_CAMERA_GET_YAW, Conv::Cdecl, &[])
    }

    /// `code_0082d930()`: the yaw stored as `f32`, the other cell 0.0.
    pub fn store_camera_yaw(h: &mut impl Host) {
        let y = h.call_sym_st0_f64(CAMERA_GET_YAW, Conv::Cdecl, &[]);
        h.write(h.rebase(YAW_CELL), x87::store(y));
        h.write(h.rebase(YAW_ZERO), 0u32);
    }

    /// `Sg_PLightDispose` (cdecl).
    pub const PLIGHT_DISPOSE: u32 = 0x0083_1ba0;

    /// `Sg_PLightDispose_thunk()`: `jmp Sg_PLightDispose`; its `eax`.
    pub fn plight_dispose_thunk(h: &mut impl Host) -> u32 {
        h.call_sym(PLIGHT_DISPOSE, Conv::Cdecl, &[])
    }

    /// The capture's frames-left word, its blend and interlace bytes.
    pub const CAPTURE_LEFT: u32 = 0x0130_1804;
    pub const CAPTURE_BLEND: u32 = 0x00c4_97d3;
    pub const CAPTURE_INTERLACED: u32 = 0x00c4_97d2;

    /// `code_0083a570()`: the capture's frames-left word cleared.
    pub fn capture_clear(h: &mut impl Host) {
        h.write(h.rebase(CAPTURE_LEFT), 0u32);
    }

    /// `Sg_CaptureSetFilter(on)` / `Sg_CaptureSetMode(on)`: the byte
    /// stored (`al` holds it after; the rest of `eax` is the caller's).
    pub fn capture_set(h: &mut impl Host, cell: u32, on: u32) {
        #[allow(clippy::cast_possible_truncation)] // the low byte, `mov al`
        h.write(h.rebase(cell), on as u8);
    }
}
