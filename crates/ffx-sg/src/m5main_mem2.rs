//! M5 `main`: the `Sg_Mem` heaps' debug side (block and heap checks,
//! dumps, the free-space and usage queries), the host file read and
//! write of the PS2 debug build, the resource dump, the aligned
//! allocation wrappers and the cache natives' thunks.
//!
//! The heap layout is in `m5main_mem`.

/// The originals over game memory.
#[allow(
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::verbose_bit_mask
)] // registers; the state tests as written
pub mod bridge {
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// `dbgPrintf(fmt, ...)`, `yiAssert(abort, part, fmt, ...)` (cdecl,
    /// varargs).
    pub const DBG_PRINTF: u32 = 0x0062_f500;
    pub const YI_ASSERT: u32 = 0x0090_ae50;
    /// `MSVCR110!sprintf`'s import slot (cdecl).
    pub const SPRINTF_IAT: u32 = 0x00b0_c3dc;
    /// `sceOpen`, `sceLseek`, `sceRead`, `sceWrite`, `sceClose` (cdecl).
    pub const SCE_OPEN: u32 = 0x0062_fa30;
    pub const SCE_LSEEK: u32 = 0x0062_f8e0;
    pub const SCE_READ: u32 = 0x0062_fc00;
    pub const SCE_WRITE: u32 = 0x0062_fdc0;
    pub const SCE_CLOSE: u32 = 0x0062_f610;
    /// `user_malloc(size)` (cdecl).
    pub const USER_MALLOC: u32 = 0x0068_7020;
    /// `g_SgMemHeaps` (0x18 bytes each).
    pub const HEAPS: u32 = 0x0130_1968;

    fn sprintf(h: &mut impl Host, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(SPRINTF_IAT));
        h.call(f, Conv::Cdecl, args)
    }

    fn dbg(h: &mut impl Host, fmt: u32, args: &[u32]) -> u32 {
        let mut w = vec![h.rebase(fmt)];
        w.extend_from_slice(args);
        h.call_sym(DBG_PRINTF, Conv::Cdecl, &w)
    }

    fn heap_of(h: &impl Host, block: u32) -> u32 {
        (h.read::<u32>(block + 0xc) >> 27) & 3
    }

    fn heap_record(h: &impl Host, heap: u32) -> u32 {
        h.rebase(HEAPS).wrapping_add(heap.wrapping_mul(0x18))
    }

    /// The host-write switch; `"%s%s"`, `"host0:"`; the failed-open
    /// assertion's part and format.
    pub const HOST_WRITE_ON: u32 = 0x0133_c910;
    pub const FMT_CONCAT: u32 = 0x00b4_0eb0;
    pub const STR_HOST0: u32 = 0x00b5_9c78;
    pub const STR_PC_WRITE_PART: u32 = 0x00b5_9ca0;
    pub const FMT_PC_WRITE_FAIL: u32 = 0x00b5_9c80;
    /// `Sg_PcWrite`'s frame below `ebp`: the name at `ebp - 0x104`.
    pub const PC_WRITE_FRAME: u32 = 0x104;

    /// `Sg_PcWrite(p, n, name)` (`0x0083afe0`, cdecl, with its frame at
    /// `ebp`): with host writes on, `host0:<name>` opened and `n` bytes
    /// written (the assertion when it can't be opened). Returns the
    /// close's (or the assertion's) `eax`; with host writes off the
    /// original's `eax` is junk (the cookie), here 0.
    pub fn pc_write(h: &mut impl Host, ebp: u32, p: u32, n: u32, name: u32) -> u32 {
        if h.read::<u8>(h.rebase(HOST_WRITE_ON)) == 0 {
            return 0;
        }
        let buf = ebp - PC_WRITE_FRAME;
        let fmt = h.rebase(FMT_CONCAT);
        let host0 = h.rebase(STR_HOST0);
        sprintf(h, &[buf, fmt, host0, name]);
        let fd = h.call_sym(SCE_OPEN, Conv::Cdecl, &[buf, 0x602]);
        if fd == 0 {
            let part = h.rebase(STR_PC_WRITE_PART);
            let fmt = h.rebase(FMT_PC_WRITE_FAIL);
            return h.call_sym(YI_ASSERT, Conv::Cdecl, &[0, part, fmt, name]);
        }
        h.call_sym(SCE_WRITE, Conv::Cdecl, &[fd, p, n]);
        h.call_sym(SCE_CLOSE, Conv::Cdecl, &[fd])
    }

    /// `Sg_PcRead`'s strings: the mode names (0, 1, else), the log
    /// lines, the path format, the assertion's part and format.
    pub const STR_MODE0: u32 = 0x00b8_fbeb;
    pub const STR_MODE1: u32 = 0x00b8_f7b0;
    pub const STR_MODE_OTHER: u32 = 0x00b8_f7c0;
    pub const MSG_PC_READ: u32 = 0x00b5_9c30;
    pub const FMT_PC_READ_PATH: u32 = 0x00b5_46c0;
    pub const MSG_PC_READ_NONE: u32 = 0x00b5_9bcc;
    pub const MSG_PC_READ_SKIP: u32 = 0x00b5_9be0;
    pub const STR_PC_READ_PART: u32 = 0x00b5_9c58;
    pub const FMT_PC_READ_FAIL: u32 = 0x00b5_9c44;
    pub const MSG_PC_READ_SIZE: u32 = 0x00b5_9c64;
    pub const MSG_PC_READ_DONE: u32 = 0x00b5_9c28;
    /// The word set after the first read (a missing file is only logged
    /// then) and the last file's size.
    pub const PC_READ_SEEN: u32 = 0x00c4_9798;
    pub const PC_READ_SIZE: u32 = 0x0130_1818;
    /// `Sg_PcRead`'s frame below `ebp`: the path at `ebp - 0x84`.
    pub const PC_READ_FRAME: u32 = 0x84;

    /// `Sg_PcRead_0083b1d0(name, mode)` (cdecl, with its frame at `ebp`):
    /// the file read whole into a `user_malloc` buffer (modes 0..2; other
    /// modes read into address 0). A missing file returns 0 when
    /// [`PC_READ_SEEN`] is set, else asserts and reads from descriptor 0.
    /// [`PC_READ_SEEN`] is set at the end. Returns the buffer.
    pub fn pc_read(h: &mut impl Host, ebp: u32, name: u32, mode: u32) -> u32 {
        let kind = match mode {
            0 => STR_MODE0,
            1 => STR_MODE1,
            _ => STR_MODE_OTHER,
        };
        let kind = h.rebase(kind);
        dbg(h, MSG_PC_READ, &[kind, name]);
        let buf = ebp - PC_READ_FRAME;
        let fmt = h.rebase(FMT_PC_READ_PATH);
        sprintf(h, &[buf, fmt, name]);
        let fd = h.call_sym(SCE_OPEN, Conv::Cdecl, &[buf, 1]);
        let r = 'read: {
            if fd == 0 {
                dbg(h, MSG_PC_READ_NONE, &[]);
                if h.read::<u32>(h.rebase(PC_READ_SEEN)) != 0 {
                    dbg(h, MSG_PC_READ_SKIP, &[]);
                    break 'read 0;
                }
                let part = h.rebase(STR_PC_READ_PART);
                let fmt = h.rebase(FMT_PC_READ_FAIL);
                h.call_sym(YI_ASSERT, Conv::Cdecl, &[0, part, fmt, name]);
            }
            let size = h.call_sym(SCE_LSEEK, Conv::Cdecl, &[fd, 0, 2]);
            let size_at = h.rebase(PC_READ_SIZE);
            h.write(size_at, size);
            h.call_sym(SCE_LSEEK, Conv::Cdecl, &[fd, 0, 0]);
            let mut p = 0;
            if mode <= 2 {
                let size: u32 = h.read(size_at);
                p = h.call_sym(USER_MALLOC, Conv::Cdecl, &[size]);
            }
            let size: u32 = h.read(size_at);
            dbg(h, MSG_PC_READ_SIZE, &[size, p]);
            let size: u32 = h.read(size_at);
            h.call_sym(SCE_READ, Conv::Cdecl, &[fd, p, size]);
            h.call_sym(SCE_CLOSE, Conv::Cdecl, &[fd]);
            dbg(h, MSG_PC_READ_DONE, &[]);
            p
        };
        h.write(h.rebase(PC_READ_SEEN), 1u32);
        r
    }

    /// `Sg_MemCheckMCB`'s six messages.
    pub const MSG_MCB: [u32; 6] = [
        0x00b5_9f44,
        0x00b5_9f60,
        0x00b5_9f84,
        0x00b5_9fa4,
        0x00b5_9fc4,
        0x00b5_9ff4,
    ];

    /// `Sg_MemCheckMCB(block)` (`0x0083b960`, cdecl): the block, its next
    /// (unless it is the end block) and its previous (when set) inside
    /// their heap (unsigned compares). Returns 0, or -1..-6 by the first
    /// failed test, with a log line.
    pub fn check_mcb(h: &mut impl Host, b: u32) -> u32 {
        let flags: u32 = h.read(b + 0xc);
        let heap = (flags >> 27) & 3;
        let r = heap_record(h, heap);
        let first: u32 = h.read(r + 0xc);
        if b < first {
            dbg(h, MSG_MCB[0], &[b, first]);
            return 0xffff_ffff;
        }
        let end: u32 = h.read(r + 0x10);
        if b > end {
            dbg(h, MSG_MCB[1], &[b, end, heap]);
            return 0xffff_fffe;
        }
        let next: u32 = h.read(b);
        if next < first && flags & 0xf != 0xf {
            dbg(h, MSG_MCB[2], &[next, first]);
            return 0xffff_fffd;
        }
        if next > end {
            dbg(h, MSG_MCB[3], &[next, end]);
            return 0xffff_fffc;
        }
        let prev: u32 = h.read(b + 4);
        if prev < first && prev != 0 {
            let state = h.read::<u32>(b + 0xc) & 0xf;
            dbg(h, MSG_MCB[4], &[state, heap, b, prev, first]);
            return 0xffff_fffb;
        }
        if prev > end {
            dbg(h, MSG_MCB[5], &[prev, end]);
            return 0xffff_fffa;
        }
        0
    }

    /// `Sg_MemDumpMCB`'s strings and the state names' table.
    pub const FMT_DUMP_HEAD: u32 = 0x00b5_a2f4;
    pub const MSG_DUMP_END: u32 = 0x00b5_a300;
    pub const FMT_DUMP_SIZE: u32 = 0x00b5_a310;
    pub const STATE_NAMES: u32 = 0x00c4_98e0;
    pub const MSG_DUMP_FREE: u32 = 0x00b5_a318;
    pub const MSG_DUMP_LAST: u32 = 0x00b5_a32c;
    pub const FMT_DUMP_FUNC: u32 = 0x00b5_a33c;
    pub const FMT_DUMP_NAME: u32 = 0x00b5_a348;
    pub const MSG_NEWLINE: u32 = 0x00b7_cae0;
    /// `Sg_GetFuncName(addr)`, `FUN_008456d0()`, `Sg_MemGetStr(p)`
    /// (cdecl).
    pub const GET_FUNC_NAME: u32 = 0x0084_56c0;
    pub const FUNC_BASE: u32 = 0x0084_56d0;
    pub const MEM_GET_STR: u32 = 0x0083_c070;

    /// `Sg_MemDumpMCB(block, index)` (`0x0083bbc0`, cdecl): one block's
    /// log lines: its index and data, its size, a `*` (0x2a) for checked
    /// blocks, its state's name; for used blocks the allocating function
    /// (from bits 4..25 of the flags) and the block's name when it has
    /// one. Returns the last `dbgPrintf`'s `eax`.
    pub fn dump_mcb(h: &mut impl Host, b: u32, index: u32) -> u32 {
        let data = b + 0x10;
        dbg(h, FMT_DUMP_HEAD, &[index, data]);
        let flags: u32 = h.read(b + 0xc);
        if flags & 0xf == 0xf {
            dbg(h, MSG_DUMP_END, &[]);
        } else {
            let size = h.read::<u32>(b).wrapping_sub(b).wrapping_sub(0x10);
            let mark = if flags & 0x0400_0000 != 0 { 0x2a } else { 0x20 };
            dbg(h, FMT_DUMP_SIZE, &[size, mark]);
            let state = h.read::<u32>(b + 0xc) & 0xf;
            let name: u32 = h.read(h.rebase(STATE_NAMES) + 4 * state);
            h.call_sym(DBG_PRINTF, Conv::Cdecl, &[name]);
        }
        let state = h.read::<u32>(b + 0xc) & 0xf;
        match state {
            0 => {
                dbg(h, MSG_DUMP_FREE, &[]);
            }
            0xf => {
                dbg(h, MSG_DUMP_LAST, &[]);
            }
            _ => {
                let v = (h.read::<u32>(b + 0xc) >> 4) & 0x3f_ffff;
                let base = h.call_sym(FUNC_BASE, Conv::Cdecl, &[]);
                let off = (v * 4).wrapping_sub(base);
                let func = h.call_sym(GET_FUNC_NAME, Conv::Cdecl, &[v * 4]);
                dbg(h, FMT_DUMP_FUNC, &[func, off]);
                let s = h.call_sym(MEM_GET_STR, Conv::Cdecl, &[data]);
                if s != 0 {
                    dbg(h, FMT_DUMP_NAME, &[s]);
                }
            }
        }
        dbg(h, MSG_NEWLINE, &[])
    }

    /// `Ch_MemRelinkBack(heap)`, `Sg_MemDumpMCB` (cdecl).
    pub const RELINK_BACK: u32 = 0x0083_c6a0;
    pub const DUMP_MCB: u32 = 0x0083_bbc0;
    /// `Sg_MemDump_0083c630`'s head and tail lines.
    pub const MSG_HEAP_DUMP: u32 = 0x00b5_a350;
    pub const MSG_HEAP_DUMP_END: u32 = 0x00b5_a388;

    /// `Sg_MemDump_0083c630(heap; slots)` (cdecl): relinks the heap when
    /// its freed byte is set, then dumps every used block and every free
    /// block of 0x40 bytes or more (with its index in the list); the
    /// closing line goes into its argument slot for the tail jump to
    /// `dbgPrintf`, whose `eax` it returns.
    pub fn heap_dump(h: &mut impl Host, heap: u32, slots: u32) -> u32 {
        let mut b: u32 = h.read(heap + 0xc);
        if h.read::<u8>(heap) != 0 {
            h.call_sym(RELINK_BACK, Conv::Cdecl, &[heap]);
        }
        dbg(h, MSG_HEAP_DUMP, &[]);
        let mut i = 0u32;
        while b != 0 {
            let size = h.read::<u32>(b).wrapping_sub(b).wrapping_sub(0x10);
            if size > 0x3f || h.read::<u8>(b + 0xc) & 0xf != 0 {
                h.call_sym(DUMP_MCB, Conv::Cdecl, &[b, i]);
            }
            b = h.read(b);
            i += 1;
        }
        let msg = h.rebase(MSG_HEAP_DUMP_END);
        h.write(slots, msg);
        h.call_sym(DBG_PRINTF, Conv::Cdecl, &[msg])
    }

    /// `Sg_MemGetSum(header)` (cdecl, `ax`), `Ch_FunctionHistoryDump()`,
    /// `Sg_MemCheckMCB`, `Sg_MemDump_0083c630` (cdecl).
    pub const MEM_GET_SUM: u32 = 0x0083_c0b0;
    pub const HISTORY_DUMP: u32 = 0x0082_69e0;
    pub const CHECK_MCB: u32 = 0x0083_b960;
    pub const HEAP_DUMP: u32 = 0x0083_c630;
    /// `Sg_MemCheckHeap`'s messages: the checksum report (file and line,
    /// its two lines, the separator, the second report), the assertion's
    /// part and its two formats, the broken-link report.
    pub const FMT_SUM_AT: u32 = 0x00b5_a0b8;
    pub const MSG_SUM_BAD: u32 = 0x00b5_a0e8;
    pub const MSG_SEPARATOR: u32 = 0x00b5_a118;
    pub const FMT_SUM_AT2: u32 = 0x00b5_a144;
    pub const MSG_SUM_BAD2: u32 = 0x00b5_a174;
    pub const MSG_SUM_END: u32 = 0x00b5_a1a8;
    pub const STR_CHECK_PART: u32 = 0x00b5_a1ec;
    pub const FMT_SUM_ASSERT: u32 = 0x00b5_a1d4;
    pub const FMT_LINK_AT: u32 = 0x00b5_a1fc;
    pub const FMT_LINK_CODE: u32 = 0x00b5_a22c;
    pub const FMT_LINK_ASSERT: u32 = 0x00b5_a260;

    /// `Sg_MemCheckHeap(heap, file, line)` (`0x0083b7d0`, cdecl): when
    /// the heap's check byte (`+1`) is set, each block: a checked block
    /// (`+0xc` bit 26) whose 16-bit sum differs from its `+8` halfword is
    /// reported (the history and the heap dumped, an assertion); a next
    /// block failing `Sg_MemCheckMCB` likewise. Its `eax` is junk to the
    /// callers.
    pub fn check_heap(h: &mut impl Host, heap: u32, file: u32, line: u32) {
        let mut b: u32 = h.read(heap + 0xc);
        if h.read::<u8>(heap + 1) == 0 || b == 0 {
            return;
        }
        while b != 0 {
            if h.read::<u32>(b + 0xc) & 0x0400_0000 != 0 {
                let sum = h.call_sym(MEM_GET_SUM, Conv::Cdecl, &[b]);
                if u32::from(h.read::<u16>(b + 8)) != sum & 0xffff {
                    dbg(h, FMT_SUM_AT, &[file, line]);
                    dbg(h, MSG_SUM_BAD, &[]);
                    h.call_sym(DUMP_MCB, Conv::Cdecl, &[b, 0]);
                    dbg(h, MSG_SEPARATOR, &[]);
                    h.call_sym(HISTORY_DUMP, Conv::Cdecl, &[]);
                    let rec = heap_record(h, heap_of(h, b));
                    h.call_sym(HEAP_DUMP, Conv::Cdecl, &[rec]);
                    dbg(h, FMT_SUM_AT2, &[file, line]);
                    dbg(h, MSG_SUM_BAD2, &[]);
                    h.call_sym(DUMP_MCB, Conv::Cdecl, &[b, 0]);
                    dbg(h, MSG_SUM_END, &[]);
                    let part = h.rebase(STR_CHECK_PART);
                    let fmt = h.rebase(FMT_SUM_ASSERT);
                    h.call_sym(YI_ASSERT, Conv::Cdecl, &[0, part, fmt]);
                }
            }
            let next: u32 = h.read(b);
            if next != 0 {
                let code = h.call_sym(CHECK_MCB, Conv::Cdecl, &[next]);
                if code != 0 {
                    dbg(h, FMT_LINK_AT, &[file, line]);
                    dbg(h, FMT_LINK_CODE, &[code]);
                    h.call_sym(DUMP_MCB, Conv::Cdecl, &[b, 0]);
                    dbg(h, MSG_SEPARATOR, &[]);
                    h.call_sym(HISTORY_DUMP, Conv::Cdecl, &[]);
                    let rec = heap_record(h, heap_of(h, b));
                    h.call_sym(HEAP_DUMP, Conv::Cdecl, &[rec]);
                    dbg(h, FMT_SUM_AT, &[file, line]);
                    dbg(h, FMT_LINK_CODE, &[code]);
                    h.call_sym(DUMP_MCB, Conv::Cdecl, &[b, 0]);
                    dbg(h, MSG_SEPARATOR, &[]);
                    let part = h.rebase(STR_CHECK_PART);
                    let fmt = h.rebase(FMT_LINK_ASSERT);
                    h.call_sym(YI_ASSERT, Conv::Cdecl, &[0, part, fmt]);
                }
            }
            b = h.read(b);
        }
    }

    /// `Sg_MemCheckHeap` (cdecl).
    pub const CHECK_HEAP: u32 = 0x0083_b7d0;

    /// `Sg_MemCheck(id, file, line; slots)` (`0x0083b7b0`, cdecl): heap
    /// `id`'s record into its first argument slot, then a tail jump to
    /// `Sg_MemCheckHeap`; its `eax`.
    pub fn mem_check(h: &mut impl Host, id: u32, file: u32, line: u32, slots: u32) -> u32 {
        let rec = h
            .rebase(HEAPS)
            .wrapping_add(id.wrapping_mul(3).wrapping_mul(8));
        h.write(slots, rec);
        h.call_sym(CHECK_HEAP, Conv::Cdecl, &[rec, file, line])
    }

    /// `user_mem_size(p)` (cdecl).
    pub const USER_MEM_SIZE: u32 = 0x0068_7050;

    /// `Sg_MemGetSize(p)` (`0x0083c050`, cdecl): 0 for null, else a tail
    /// jump to `user_mem_size` (the slot rewritten with the same word);
    /// its `eax`.
    pub fn mem_get_size(h: &mut impl Host, p: u32) -> u32 {
        if p == 0 {
            return 0;
        }
        h.call_sym(USER_MEM_SIZE, Conv::Cdecl, &[p])
    }

    /// `Sg_MemGetSize`, `ch_create_name(word)` (cdecl).
    pub const MEM_GET_SIZE: u32 = 0x0083_c050;
    pub const CREATE_NAME: u32 = 0x0083_8100;
    /// The resource dump's two formats and the kind names (kinds 1..4).
    pub const FMT_RES_MEM: u32 = 0x00b5_a4dc;
    pub const FMT_RES_INFO: u32 = 0x00b5_a4f8;
    pub const RES_KIND_NAMES: [u32; 4] = [0x00b5_a514, 0x00b5_a51c, 0x00b5_a524, 0x00b5_a52c];

    /// `FUN_0083ff20(res; slots)` (cdecl): one resource's log lines (its
    /// memory and size, the bytes at `+5` and `+8`, the name of its id
    /// word, its kind for kinds 1..4); the newline into its argument slot
    /// for the tail jump to `dbgPrintf`, whose `eax` it returns. Nothing
    /// without memory (`+0xc`); the original's `eax` is junk then, here 0.
    pub fn resource_dump(h: &mut impl Host, res: u32, slots: u32) -> u32 {
        let memory: u32 = h.read(res + 0xc);
        if memory == 0 {
            return 0;
        }
        let b8 = u32::from(h.read::<u8>(res + 8));
        let b5 = u32::from(h.read::<u8>(res + 5));
        let size = h.call_sym(MEM_GET_SIZE, Conv::Cdecl, &[memory]);
        let memory: u32 = h.read(res + 0xc);
        dbg(h, FMT_RES_MEM, &[memory, size, b5, b8 & 7]);
        let w: u32 = h.read(res);
        let name = h.call_sym(CREATE_NAME, Conv::Cdecl, &[w]);
        let b6 = i32::from(h.read::<i8>(res + 6)).cast_unsigned();
        let b8 = u32::from(h.read::<u8>(res + 8)) >> 3;
        dbg(
            h,
            FMT_RES_INFO,
            &[b8, b6, name, (w.cast_signed() >> 16).cast_unsigned()],
        );
        let kind = i32::from(h.read::<i8>(res + 4))
            .wrapping_sub(1)
            .cast_unsigned();
        if kind <= 3 {
            dbg(h, RES_KIND_NAMES[kind as usize], &[]);
        }
        let nl = h.rebase(MSG_NEWLINE);
        h.write(slots, nl);
        h.call_sym(DBG_PRINTF, Conv::Cdecl, &[nl])
    }

    /// `Cache_Dump(fmt)` (cdecl) and the line `Sg_MemDump` hands it.
    pub const CACHE_DUMP: u32 = 0x0084_0510;
    pub const MSG_CACHE_DUMP: u32 = 0x00b5_a3a0;

    /// `Sg_MemDump(heap; slots)` (`0x0083bb50`, cdecl): heap 1, 2 or (any
    /// other) 0 dumped, then the cache line into its argument slot for
    /// the tail jump to `Cache_Dump`, whose `eax` it returns.
    pub fn mem_dump(h: &mut impl Host, heap: u32, slots: u32) -> u32 {
        let k = match heap {
            1 => 1,
            2 => 2,
            _ => 0,
        };
        let rec = heap_record(h, k);
        h.call_sym(HEAP_DUMP, Conv::Cdecl, &[rec]);
        let msg = h.rebase(MSG_CACHE_DUMP);
        h.write(slots, msg);
        h.call_sym(CACHE_DUMP, Conv::Cdecl, &[msg])
    }

    /// `make_yi_font16()` (`0x0083c5a0`): heap 0 relinked; its `eax`.
    pub fn relink_heap0(h: &mut impl Host) -> u32 {
        let rec = h.rebase(HEAPS);
        h.call_sym(RELINK_BACK, Conv::Cdecl, &[rec])
    }

    /// `make_yi_font16`, `Ch_MemMove_Over()`, `Cache_OverFlow()` (cdecl).
    pub const RELINK_HEAP0: u32 = 0x0083_c5a0;
    pub const MEM_MOVE_OVER: u32 = 0x0082_7260;
    pub const CACHE_OVERFLOW: u32 = 0x0084_09a0;

    /// `Sg_MemGet(mode)` (`0x0083bff0`, cdecl): mode 1 relinks heap 0
    /// first; mode 2 moves blocks until nothing more moves and frees
    /// cache until nothing is left to free. Returns the largest free
    /// block's size in heap 0's list (from its first block as it was on
    /// entry).
    pub fn mem_get(h: &mut impl Host, mode: u32) -> u32 {
        let mut b: u32 = h.read(h.rebase(HEAPS) + 0xc);
        if mode == 1 {
            h.call_sym(RELINK_HEAP0, Conv::Cdecl, &[]);
        } else if mode == 2 {
            while h.call_sym(MEM_MOVE_OVER, Conv::Cdecl, &[]) != 0 {}
            while h.call_sym(CACHE_OVERFLOW, Conv::Cdecl, &[]) == 0 {}
        }
        let mut best = 0u32;
        while b != 0 {
            if h.read::<u8>(b + 0xc) & 0xf == 0 {
                let size = h.read::<u32>(b).wrapping_sub(b).wrapping_sub(0x10);
                if size > best {
                    best = size;
                }
            }
            b = h.read(b);
        }
        best
    }

    /// `Sg_MemCheck` (cdecl) and the file name `FUN_0083c0f0` passes.
    pub const MEM_CHECK: u32 = 0x0083_b7b0;
    pub const STR_USAGE_FILE: u32 = 0x00b5_a3dc;

    /// `FUN_0083c0f0(out)` (cdecl): heap 0 checked, then its blocks'
    /// sizes (with headers) summed into nine counters by state: 0 → 7;
    /// 1, 2, 4, 9 → 2; 12 → 1; 13 → 3; 7 by the kind in bits 11..15 of
    /// `+0xa` (0 → 0, 1 → 4, 2 → 5, 3 → 6, 4 → 8, 5 → 2, else the
    /// previous block's counter, none after an end block); 15 (the end
    /// block) none; the rest → 0. Returns heap 0's size + 0x10.
    pub fn mem_usage(h: &mut impl Host, out: u32) -> u32 {
        let mut b: u32 = h.read(h.rebase(HEAPS) + 0xc);
        let file = h.rebase(STR_USAGE_FILE);
        h.call_sym(MEM_CHECK, Conv::Cdecl, &[0, file, 0x3c1]);
        h.write(out, [0u32; 9]);
        let mut cat: Option<u32> = Some(0);
        while b != 0 {
            let state = h.read::<u32>(b + 0xc) & 0xf;
            cat = match state {
                0 => Some(7),
                1 | 2 | 4 | 9 => Some(2),
                12 => Some(1),
                13 => Some(3),
                15 => None,
                7 => match u32::from(h.read::<u16>(b + 0xa)) >> 11 {
                    0 => Some(0),
                    1 => Some(4),
                    2 => Some(5),
                    3 => Some(6),
                    4 => Some(8),
                    5 => Some(2),
                    _ => cat,
                },
                _ => Some(0),
            };
            if let Some(c) = cat {
                let size = h.read::<u32>(b).wrapping_sub(b);
                let at = out.wrapping_add(4 * c);
                let v: u32 = h.read(at);
                h.write(at, v.wrapping_add(size));
            }
            b = h.read(b);
        }
        let size: u32 = h.read(h.rebase(HEAPS) + 8);
        size.wrapping_add(0x10)
    }

    /// `Sg_MemSetRtAddr(a)` (cdecl), its source word and its result.
    pub const SET_RT_ADDR: u32 = 0x0083_c560;
    pub const RT_ADDR_SOURCE: u32 = 0x00c8_f6f8;
    pub const RT_ADDR: u32 = 0x0130_19b0;

    /// `Sg_MemSetCurrentAddr(p)` (`0x0083c4e0`, cdecl): for a block's data
    /// `p`, the current return address recorded in bits 4..25 of its
    /// flags. Its `eax` is nothing the callers read.
    pub fn set_current_addr(h: &mut impl Host, p: u32) {
        if p == 0 {
            return;
        }
        let src: u32 = h.read(h.rebase(RT_ADDR_SOURCE));
        h.call_sym(SET_RT_ADDR, Conv::Cdecl, &[src]);
        let rt: u32 = h.read(h.rebase(RT_ADDR));
        let f: u32 = h.read(p - 4);
        h.write(p - 4, f ^ (((rt << 4) ^ f) & 0x03ff_fff0));
    }

    /// `user_aligned_free` (`jmp` thunk, cdecl) and `user_aligned_malloc`.
    pub const ALIGNED_FREE: u32 = 0x0068_6fe0;
    pub const ALIGNED_MALLOC: u32 = 0x0068_6ff0;

    /// `Sg_FreeA(p)` (`0x0083f090`): `user_aligned_free(p)`'s `eax`.
    pub fn free_a(h: &mut impl Host, p: u32) -> u32 {
        h.call_sym(ALIGNED_FREE, Conv::Cdecl, &[p])
    }

    /// `Sg_MallocA(size, align)` (`0x0083f0a0`): `user_aligned_malloc`'s
    /// `eax`.
    pub fn malloc_a(h: &mut impl Host, size: u32, align: u32) -> u32 {
        h.call_sym(ALIGNED_MALLOC, Conv::Cdecl, &[size, align])
    }

    /// `Cache_Clear()`, `Cache_ClearFast()` (cdecl).
    pub const CACHE_CLEAR: u32 = 0x0084_0030;
    pub const CACHE_CLEAR_FAST: u32 = 0x0084_0150;

    /// The cache natives' thunks (`code_00840e40` -> `Cache_Clear`,
    /// `code_00840e50` -> `Cache_ClearFast`): the target's `eax`.
    pub fn cache_thunk(h: &mut impl Host, target: u32) -> u32 {
        h.call_sym(target, Conv::Cdecl, &[])
    }

    /// `cosf(x)`, `sinf(x)` (cdecl, `f32` in, `ST0` out).
    pub const COSF: u32 = 0x004c_eac0;
    pub const SINF: u32 = 0x004c_eae0;
    /// The angle step's two factors (`f64`, `.rdata`) and the debug
    /// draw's 13-entry cosine and sine tables.
    pub const ANGLE_MUL: u32 = 0x00b5_9638;
    pub const ANGLE_DIV: u32 = 0x00b4_b468;
    pub const COS_TABLE: u32 = 0x0130_3308;
    pub const SIN_TABLE: u32 = 0x0130_32d4;

    /// `FUN_00841100()` (cdecl): for `i` in 0..13, `a = f32(i * mul /
    /// div)` and the tables' `cosf(a)` and `sinf(a)` (as `f32`). Leaves
    /// nothing the callers read.
    pub fn db_init_tables(h: &mut impl Host) {
        let mul = f64::from_bits(h.read::<u64>(h.rebase(ANGLE_MUL)));
        let div = f64::from_bits(h.read::<u64>(h.rebase(ANGLE_DIV)));
        for i in 0..13u32 {
            let a = x87::store(x87::div(x87::mul(mul, f64::from(i)), div));
            let c = h.call_sym_st0_f64(COSF, Conv::Cdecl, &[a]);
            h.write(h.rebase(COS_TABLE) + 4 * i, x87::store(c));
            let s = h.call_sym_st0_f64(SINF, Conv::Cdecl, &[a]);
            h.write(h.rebase(SIN_TABLE) + 4 * i, x87::store(s));
        }
    }

    /// The debug draw's buffer (both words) and `FUN_00841100`.
    pub const DB_BUFFER: u32 = 0x0130_32cc;
    pub const DB_CURSOR: u32 = 0x0130_32d0;
    pub const DB_INIT_TABLES: u32 = 0x0084_1100;

    /// `Sg_DbInitAll()` (`0x00841440`, cdecl): with host writes on, a
    /// 64 KB debug-draw buffer and the angle tables. Its `eax` is
    /// nothing the callers read.
    pub fn db_init_all(h: &mut impl Host) {
        if h.read::<u8>(h.rebase(HOST_WRITE_ON)) == 0 {
            return;
        }
        let p = h.call_sym(USER_MALLOC, Conv::Cdecl, &[0x1_0000]);
        h.write(h.rebase(DB_BUFFER), p);
        h.write(h.rebase(DB_CURSOR), p);
        h.call_sym(DB_INIT_TABLES, Conv::Cdecl, &[]);
    }

    /// The two counters `FUN_00843560` reports and its format.
    pub const DB_COUNT_A: u32 = 0x0130_334c;
    pub const DB_COUNT_B: u32 = 0x0130_3350;
    pub const FMT_DB_COUNTS: u32 = 0x00b5_a6c8;

    /// `FUN_00843560()` (cdecl): both counters logged when the first is
    /// above 3 (signed) or the second is set.
    pub fn db_report(h: &mut impl Host) {
        let a: u32 = h.read(h.rebase(DB_COUNT_A));
        if a.cast_signed() > 3 || h.read::<u32>(h.rebase(DB_COUNT_B)) != 0 {
            let b: u32 = h.read(h.rebase(DB_COUNT_B));
            let a: u32 = h.read(h.rebase(DB_COUNT_A));
            dbg(h, FMT_DB_COUNTS, &[a, b]);
        }
    }

    /// `Sg_IdFixupAndSet_00844bc0(map)`, `op_ivec4_to_fvec4_007e3100(dst,
    /// src)` (cdecl).
    pub const ID_FIXUP_AND_SET: u32 = 0x0084_4bc0;
    pub const IVEC4_TO_FVEC4: u32 = 0x007e_3100;

    /// `Sg_IdFixupAndSetThunk(map)`: `jmp Sg_IdFixupAndSet_00844bc0`
    /// with the caller's word; its `eax`.
    pub fn id_fixup_and_set_thunk(h: &mut impl Host, map: u32) -> u32 {
        h.call_sym(ID_FIXUP_AND_SET, Conv::Cdecl, &[map])
    }

    /// `code_00845de0(dst, src)`: `jmp op_ivec4_to_fvec4_007e3100` with
    /// the caller's words; its `eax`.
    pub fn ivec4_to_fvec4_thunk(h: &mut impl Host, dst: u32, src: u32) -> u32 {
        h.call_sym(IVEC4_TO_FVEC4, Conv::Cdecl, &[dst, src])
    }

    /// `sceDeci2Open(protocol, cell, handler)` (cdecl), the socket cell
    /// and the handler.
    pub const DECI2_OPEN: u32 = 0x0062_f680;
    pub const DECI2_SOCKET: u32 = 0x0130_3d00;
    pub const DECI2_HANDLER: u32 = 0x0084_ed10;

    /// `code_0084eee0()` (cdecl): the DECI2 socket opened (protocol
    /// 0xe000); 1 when it is not negative.
    pub fn deci2_open(h: &mut impl Host) -> u32 {
        let cell = h.rebase(DECI2_SOCKET);
        let f = h.rebase(DECI2_HANDLER);
        let s = h.call_sym(DECI2_OPEN, Conv::Cdecl, &[0xe000, cell, f]);
        h.write(cell, s);
        let s: u32 = h.read(cell);
        u32::from(s.cast_signed() >= 0)
    }

    /// `SystemOnHost(command, out)` (cdecl), the host's date command and
    /// the `sscanf` format; `MSVCR110!sscanf`'s slot.
    pub const SYSTEM_ON_HOST: u32 = 0x0084_ede0;
    pub const STR_DATE_COMMAND: u32 = 0x00b5_af9c;
    pub const FMT_DATE: u32 = 0x00b5_afb8;
    pub const SSCANF_IAT: u32 = 0x00b0_c448;
    /// The debug clock: frames, seconds, minutes, hours, weekday, day,
    /// month (the scan's order: month, day, weekday, hours, minutes,
    /// seconds), and its frame-rate word.
    pub const CLOCK_FRAMES: u32 = 0x0132_3d40;
    pub const CLOCK_RATE: u32 = 0x0132_3d44;
    pub const CLOCK_SEC: u32 = 0x023c_bc28;
    pub const CLOCK_MIN: u32 = 0x023c_bc2c;
    pub const CLOCK_HOUR: u32 = 0x023c_bc30;
    pub const CLOCK_WEEKDAY: u32 = 0x023c_bc1c;
    pub const CLOCK_DAY: u32 = 0x023c_bc20;
    pub const CLOCK_MONTH: u32 = 0x023c_bc24;
    /// `FUN_0084f080`'s frame below `ebp`: the reply at `ebp - 0x104`.
    pub const CLOCK_FRAME: u32 = 0x104;

    /// `FUN_0084f080()` (cdecl, with its frame at `ebp`): the host's date
    /// scanned into the debug clock, its frames at 30 and its rate word
    /// 0xd2f0. Returns `SystemOnHost`'s `eax` when it fails, else
    /// `sscanf`'s.
    pub fn clock_from_host(h: &mut impl Host, ebp: u32) -> u32 {
        let buf = ebp - CLOCK_FRAME;
        let cmd = h.rebase(STR_DATE_COMMAND);
        let r = h.call_sym(SYSTEM_ON_HOST, Conv::Cdecl, &[cmd, buf]);
        if r.cast_signed() < 0 {
            return r;
        }
        let f: u32 = h.read(h.rebase(SSCANF_IAT));
        let fmt = h.rebase(FMT_DATE);
        let cells = [
            CLOCK_MONTH,
            CLOCK_DAY,
            CLOCK_WEEKDAY,
            CLOCK_HOUR,
            CLOCK_MIN,
            CLOCK_SEC,
        ];
        let mut w = vec![buf, fmt];
        w.extend(cells.iter().map(|&c| h.rebase(c)));
        let r = h.call(f, Conv::Cdecl, &w); // call width: 8 (buf, format, six cells)
        h.write(h.rebase(CLOCK_FRAMES), 0x1eu32);
        h.write(h.rebase(CLOCK_RATE), 0xd2f0u32);
        r
    }

    /// `FUN_0084ef10()` (cdecl): the debug clock's tick: 60 frames a
    /// second, 60 seconds, 60 minutes, 24 hours; at midnight the day goes
    /// up, the weekday wraps after 6 and the rate word becomes 0x96.
    /// Returns the last value it compared (the hours, else the new
    /// weekday).
    pub fn clock_tick(h: &mut impl Host) -> u32 {
        let frames_at = h.rebase(CLOCK_FRAMES);
        let frames = h.read::<u32>(frames_at).wrapping_add(1);
        h.write(frames_at, frames);
        let mut prev_rolled = frames == 0x3c;
        let mut v = 0;
        for (cell, below) in [
            (CLOCK_SEC, CLOCK_FRAMES),
            (CLOCK_MIN, CLOCK_SEC),
            (CLOCK_HOUR, CLOCK_MIN),
        ] {
            let at = h.rebase(cell);
            v = h.read::<u32>(at);
            if prev_rolled {
                v = v.wrapping_add(1);
                h.write(h.rebase(below), 0u32);
                h.write(at, v);
            }
            prev_rolled = v == 0x3c;
        }
        if v != 0x18 {
            return v;
        }
        let wd: u32 = h.read(h.rebase(CLOCK_WEEKDAY));
        let day_at = h.rebase(CLOCK_DAY);
        let day: u32 = h.read(day_at);
        h.write(day_at, day.wrapping_add(1));
        h.write(h.rebase(CLOCK_HOUR), 0u32);
        let r = if wd == 6 {
            h.write(h.rebase(CLOCK_WEEKDAY), 0u32);
            6
        } else {
            h.write(h.rebase(CLOCK_WEEKDAY), wd.wrapping_add(1));
            wd.wrapping_add(1)
        };
        h.write(h.rebase(CLOCK_RATE), 0x96u32);
        r
    }
}
