//! M5 stage 1, list `vfx`, round 5 batch 1: small functions the earlier
//! rounds left: the field effects' (FP) reset natives, the battle opus
//! object frees and back colour, the effect characters' field-mode
//! refresh, the pad port close, the battle debug file reader's helpers,
//! and Iggy `GDraw` D3D11 callbacks (named after what they do, as Iggy's
//! `gdraw.h` callbacks are).
//!
//! Stack cookies are left out. Where the original hands a callee the
//! address of a frame buffer, the bridge takes that buffer's address (the
//! shim passes a `StackFrame`).

use ffx_bridge::Global;

/// `g_FpData`: the field's FP table (u16 count at `+4`, 0x50-byte entries
/// with the id word at `+0x64`).
pub const FP_DATA: Global<u32> = Global::new(0x00d2_c258);
/// `g_FpRun`: 0x80 bytes per FP entry.
pub const FP_RUN: u32 = 0x00d2_c260;
/// `g_pppEnvSlot`: where the current ppp environment pointer lives.
pub const PPP_ENV_SLOT: Global<u32> = Global::new(0x00c3_a4cc);
/// `g_pppFpHeap`: the field effects' ppp heap.
pub const PPP_FP_HEAP: u32 = 0x00ce_c22c;
/// The battle opus objects: 0x100 slot words, their 0x80-byte run blocks,
/// and the heap they belong to.
pub const OPUS_SLOTS: u32 = 0x012e_9c70;
pub const OPUS_RUNS: u32 = 0x012e_a070;
pub const OPUS_HEAP: u32 = 0x012f_2070;
/// `g_EtBindDefault`, `g_EtChr` (0x20-byte records).
pub const BIND_DEFAULT: Global<u16> = Global::new(0x011a_0080);
pub const ET_CHR: u32 = 0x012a_22c0;
/// `g_OpusBattle`, `g_OpEffectRunLimit`, `g_OpLatch0` and the word the
/// run limit is computed from (`0x02332e90`).
pub const OPUS_BATTLE: Global<u32> = Global::new(0x012a_40c0);
pub const OP_EFFECT_RUN_LIMIT: Global<u32> = Global::new(0x0233_2e88);
pub const OP_LATCH0: Global<u32> = Global::new(0x0233_2e8c);
pub const OP_LATCH_SRC: Global<u32> = Global::new(0x0233_2e90);
/// `g_OpusMapBackColorKept` (byte), `g_OpusMapBackColor` (4 words).
pub const BACK_COLOR_KEPT: Global<u8> = Global::new(0x011a_006f);
pub const BACK_COLOR: u32 = 0x011a_0070;
/// `g_TkControlerWork`: 2 words of open-slot bits.
pub const CONTROLER_WORK: u32 = 0x0133_0280;
/// `g_OpDebugMode` and the file-load switch after it (`0x02322795`).
pub const OP_DEBUG_MODE: Global<u8> = Global::new(0x0232_2790);
pub const OP_DEBUG_FILES: Global<u8> = Global::new(0x0232_2795);
/// `op_debug_file_load`'s "can't open" format.
pub const OPEN_FAILED_FORMAT: u32 = 0x00b5_81f0;
/// `MSVCR110!malloc` (the import slot, cdecl).
pub const MALLOC_SLOT: u32 = 0x00b0_c468;
/// Iggy `GDraw` D3D11's state pointer (`[0x01a858b4]`).
pub const GDRAW: Global<u32> = Global::new(0x01a8_58b4);
/// `IggyGDrawSendWarning` (the import slot; cdecl, varargs) and the two
/// formats of the D3D error report.
pub const SEND_WARNING_SLOT: u32 = 0x00b0_c874;
pub const OUT_OF_MEMORY_FORMAT: u32 = 0x00b7_a4ec;
pub const D3D_ERROR_FORMAT: u32 = 0x00b7_a50c;
/// `E_OUTOFMEMORY`.
pub const E_OUTOFMEMORY: u32 = 0x8007_000e;

// ---- callees ----

/// `pppAllFreePObject(run)` (cdecl).
pub const PPP_ALL_FREE_POBJECT: u32 = 0x0071_1f20;
/// `Ch_GetFieldMode(handle)` (cdecl): the byte in `al`.
pub const CH_GET_FIELD_MODE: u32 = 0x0082_ab90;
/// `opu_main_host(battle, list)` (cdecl).
pub const OPU_MAIN_HOST: u32 = 0x0080_cc00;
/// `yiSetBackColor(rgba)` / `yiGetBackColor(out)` (cdecl).
pub const YI_SET_BACK_COLOR: u32 = 0x0090_a040;
pub const YI_GET_BACK_COLOR: u32 = 0x0090_86d0;
/// `scePadPortClose(port, slot)` (cdecl).
pub const PAD_PORT_CLOSE: u32 = 0x0062_fb90;
/// `ms_free(p)` (cdecl).
pub const MS_FREE: u32 = 0x007c_f8f0;
/// `sceOpen(name, mode)`, `sceLseek(fd, off, whence)`, `sceRead(fd, buf,
/// n)`, `sceClose(fd)`, `dbgPrintf(format, ...)`, `debug_exit_trace(n)`,
/// `FlushCache(n)` (cdecl).
pub const SCE_OPEN: u32 = 0x0062_fa30;
pub const SCE_LSEEK: u32 = 0x0062_f8e0;
pub const SCE_READ: u32 = 0x0062_fc00;
pub const SCE_CLOSE: u32 = 0x0062_f610;
pub const DBG_PRINTF: u32 = 0x0062_f500;
pub const DEBUG_EXIT_TRACE: u32 = 0x0062_f510;
pub const FLUSH_CACHE: u32 = 0x0062_f4c0;
/// `StoreComputed_00a3a010(res, 2)` (cdecl): moves a `GDraw` resource to
/// another ring.
pub const GDRAW_RING_MOVE: u32 = 0x00a3_a010;
/// `crt_memset_thunk(p, v, n)` (cdecl).
pub const MEMSET: u32 = 0x0094_964c;
/// `GDraw_freeResources_00a36300(stats)` (cdecl).
pub const GDRAW_FREE_RESOURCES: u32 = 0x00a3_6300;
/// `FUN_00a3b480(handle, stats)` (cdecl): frees a `GDraw` handle.
pub const GDRAW_FREE_HANDLE: u32 = 0x00a3_b480;

pub mod bridge {
    use super::{
        BACK_COLOR, BACK_COLOR_KEPT, BIND_DEFAULT, CH_GET_FIELD_MODE, CONTROLER_WORK,
        D3D_ERROR_FORMAT, DBG_PRINTF, DEBUG_EXIT_TRACE, E_OUTOFMEMORY, ET_CHR, FLUSH_CACHE,
        FP_DATA, FP_RUN, GDRAW, GDRAW_FREE_HANDLE, GDRAW_FREE_RESOURCES, GDRAW_RING_MOVE,
        MALLOC_SLOT, MEMSET, MS_FREE, OP_DEBUG_FILES, OP_DEBUG_MODE, OP_EFFECT_RUN_LIMIT,
        OP_LATCH_SRC, OP_LATCH0, OPEN_FAILED_FORMAT, OPU_MAIN_HOST, OPUS_BATTLE, OPUS_HEAP,
        OPUS_RUNS, OPUS_SLOTS, OUT_OF_MEMORY_FORMAT, PAD_PORT_CLOSE, PPP_ALL_FREE_POBJECT,
        PPP_ENV_SLOT, PPP_FP_HEAP, SCE_CLOSE, SCE_LSEEK, SCE_OPEN, SCE_READ, SEND_WARNING_SLOT,
        YI_GET_BACK_COLOR, YI_SET_BACK_COLOR,
    };
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// Makes the heap at `heap` the current ppp environment.
    fn set_env(h: &mut impl Host, heap: u32) {
        let slot = PPP_ENV_SLOT.get(h);
        let heap = h.rebase(heap);
        h.write(slot, heap);
    }

    /// The FP entries: their count, or `None` without a table.
    fn fp_entries(h: &impl Host) -> Option<(u32, u32)> {
        let data = FP_DATA.get(h);
        if data == 0 {
            return None;
        }
        Some((data, u32::from(h.read::<u16>(data + 4))))
    }

    /// `pppFpFreeAll_007296a0()` (Atel native 0x8008): with an FP table,
    /// the FP heap is made current, and every entry with an id has its
    /// objects freed (`pppAllFreePObject` on its run record, the heap made
    /// current again before each) and its frame counter set to -0x1000
    /// (not started).
    pub fn fp_free_all(h: &mut impl Host) {
        let Some((data, n)) = fp_entries(h) else {
            return;
        };
        set_env(h, PPP_FP_HEAP);
        let run = h.rebase(FP_RUN);
        for i in 0..n {
            if h.read::<u16>(data + 0x64 + 0x50 * i) != 0 {
                let r = run + 0x80 * i;
                set_env(h, PPP_FP_HEAP);
                cdecl(h, PPP_ALL_FREE_POBJECT, &[r]);
                h.write(r, 0xffff_f000_u32);
            }
        }
    }

    /// `pppFpResetSameIds_00729740()` (Atel native 0x8007): for every FP
    /// entry with an id, every entry with the same id (itself included)
    /// gets its frame counter and status cleared (counter 0: running).
    pub fn fp_reset_same_ids(h: &mut impl Host) {
        let Some((data, n)) = fp_entries(h) else {
            return;
        };
        let run = h.rebase(FP_RUN);
        let mut data = data;
        for i in 0..n {
            let id: u16 = h.read(data + 0x64 + 0x50 * i);
            if id == 0 {
                continue;
            }
            let m = u32::from(h.read::<u16>(data + 4));
            for j in 0..m {
                if h.read::<u16>(data + 0x64 + 0x50 * j) == id {
                    let r = run + 0x80 * j;
                    h.write(r, 0_u32);
                    h.write(r + 0x1d, 0_u8);
                }
            }
            data = FP_DATA.get(h);
        }
    }

    /// `pppFpClearFrames_007297e0()` (Atel native 0x805d's first step):
    /// every FP entry's frame counter to 0.
    pub fn fp_clear_frames(h: &mut impl Host) {
        let Some((_, n)) = fp_entries(h) else {
            return;
        };
        let run = h.rebase(FP_RUN);
        for i in 0..n {
            h.write(run + 0x80 * i, 0_u32);
        }
    }

    /// `op_et_chr_field_mode_update_007fb690()` (Atel native 0x0244): bit
    /// 11 of `g_EtBindDefault` cleared, then each of the first 0xd0 effect
    /// characters with a handle gets `Ch_GetFieldMode(handle)` at `+0x16`.
    pub fn et_chr_field_mode_update(h: &mut impl Host) {
        let flags = BIND_DEFAULT.get(h) & 0xf7ff;
        BIND_DEFAULT.set(h, flags);
        let chr = h.rebase(ET_CHR);
        for i in 0..0xd0 {
            let rec = chr + 0x20 * i;
            let handle: u32 = h.read(rec);
            if handle != 0 {
                let mode = cdecl(h, CH_GET_FIELD_MODE, &[handle]);
                h.write(rec + 0x16, mode as u8);
            }
        }
    }

    /// `opu_free_all_pobjects_00800970()` -> 0: the opus heap made current,
    /// then each of the 0x100 opus slots that holds an object is cleared
    /// and its run block's objects freed (the heap made current again
    /// before each).
    pub fn opu_free_all_pobjects(h: &mut impl Host) -> u32 {
        set_env(h, OPUS_HEAP);
        let slots = h.rebase(OPUS_SLOTS);
        let runs = h.rebase(OPUS_RUNS);
        for i in 0..0x100 {
            let slot = slots + 4 * i;
            if h.read::<u32>(slot) != 0 {
                h.write(slot, 0_u32);
                set_env(h, OPUS_HEAP);
                cdecl(h, PPP_ALL_FREE_POBJECT, &[runs + 0x80 * i]);
            }
        }
        0
    }

    /// `opu_run_battle_host_00800a30()` -> 0: runs `opu_main_host(0, 2)`;
    /// unless the battle opus has `+0x58` set, the run limit is opened
    /// around it (`+0x54` keeps `g_OpLatch0`, the limit is the latch
    /// source + 0xf0000, and the latch is put back after). The limit is
    /// cleared after the run.
    pub fn opu_run_battle_host(h: &mut impl Host) -> u32 {
        let o = OPUS_BATTLE.get(h);
        if h.read::<u32>(o + 0x58) == 0 {
            let latch = OP_LATCH0.get(h);
            h.write(o + 0x54, latch);
            let limit = OP_LATCH_SRC.get(h).wrapping_add(0xf_0000);
            OP_EFFECT_RUN_LIMIT.set(h, limit);
        }
        cdecl(h, OPU_MAIN_HOST, &[0, 2]);
        if h.read::<u32>(o + 0x58) == 0 {
            let kept: u32 = h.read(o + 0x54);
            OP_LATCH0.set(h, kept);
        }
        OP_EFFECT_RUN_LIMIT.set(h, 0);
        0
    }

    /// `opus_map_back_color_restore_0080a4b0()`: when a back colour was
    /// kept, `yiSetBackColor` puts it back and the flag is cleared.
    pub fn back_color_restore(h: &mut impl Host) {
        if BACK_COLOR_KEPT.get(h) != 0 {
            let at = h.rebase(BACK_COLOR);
            cdecl(h, YI_SET_BACK_COLOR, &[at]);
            BACK_COLOR_KEPT.set(h, 0);
        }
    }

    /// `opus_map_back_color_set_0080a4d0(rgba)` -> `yiSetBackColor`'s
    /// `eax`: the first time, the current back colour is kept
    /// (`yiGetBackColor`); then the four floats at `rgba`, each truncated
    /// (`ftol2_sse`), go to `yiSetBackColor` through `buf` (the
    /// original's 16-aligned frame, `ebp - 0x20`).
    pub fn back_color_set(h: &mut impl Host, buf: u32, rgba: u32) -> u32 {
        if BACK_COLOR_KEPT.get(h) == 0 {
            let at = h.rebase(BACK_COLOR);
            BACK_COLOR_KEPT.set(h, 1);
            cdecl(h, YI_GET_BACK_COLOR, &[at]);
        }
        for k in 0..4 {
            let v = x87::load(h.read(rgba + 4 * k));
            let i = x87::ftol(h, v);
            h.write(buf + 4 * k, i);
        }
        cdecl(h, YI_SET_BACK_COLOR, &[buf])
    }

    /// `Pad_PortClose_00888990(port, slot)`: a negative port closes slot 0
    /// of ports 0 and 1 first and goes on with the negative port; when bit
    /// `slot` of `g_TkControlerWork[port]` is set, `scePadPortClose(port,
    /// slot)` and the bit is cleared.
    pub fn pad_port_close(h: &mut impl Host, port: u32, slot: u32) {
        if port.cast_signed() < 0 {
            for p in 0..2 {
                pad_port_close(h, p, 0);
            }
        }
        let mask = 1_u32 << (slot & 31);
        let at = h.rebase(CONTROLER_WORK).wrapping_add(port.wrapping_mul(4));
        if h.read::<u32>(at) & mask != 0 {
            cdecl(h, PAD_PORT_CLOSE, &[port, slot]);
            let v: u32 = h.read(at);
            h.write(at, v & !mask);
        }
    }

    /// `MsDbgFile_freeTree_007e1b70(node)`: along the `+0x20` chain, each
    /// node's `+0` block, its `+8` subtree (recursively), its `+4` block
    /// and the node itself go to `ms_free`.
    pub fn dbg_file_free_tree(h: &mut impl Host, node: u32) {
        let mut node = node;
        while node != 0 {
            let next: u32 = h.read(node + 0x20);
            let a: u32 = h.read(node);
            if a != 0 {
                cdecl(h, MS_FREE, &[a]);
            }
            let sub: u32 = h.read(node + 8);
            dbg_file_free_tree(h, sub);
            let b: u32 = h.read(node + 4);
            if b != 0 {
                cdecl(h, MS_FREE, &[b]);
            }
            cdecl(h, MS_FREE, &[node]);
            node = next;
        }
    }

    /// `op_debug_file_load_007e5c10(name, buf)` -> bytes read or 0: only
    /// with the op debug mode and its file switch on. A file that doesn't
    /// open is reported (`dbgPrintf`, `debug_exit_trace(1)`); otherwise
    /// its size (`sceLseek` to the end and back) is read into `buf`, and
    /// the size is returned only when `sceRead` read all of it.
    pub fn debug_file_load(h: &mut impl Host, name: u32, buf: u32) -> u32 {
        if OP_DEBUG_MODE.get(h) == 0 || OP_DEBUG_FILES.get(h) == 0 {
            return 0;
        }
        let fd = cdecl(h, SCE_OPEN, &[name, 1]);
        if fd == 0 {
            let fmt = h.rebase(OPEN_FAILED_FORMAT);
            cdecl(h, DBG_PRINTF, &[fmt, name]);
            cdecl(h, DEBUG_EXIT_TRACE, &[1]);
            return 0;
        }
        let size = cdecl(h, SCE_LSEEK, &[fd, 0, 2]);
        cdecl(h, SCE_LSEEK, &[fd, 0, 0]);
        let read = cdecl(h, SCE_READ, &[fd, buf, size]);
        let result = if size == read { size } else { 0 };
        cdecl(h, SCE_CLOSE, &[fd]);
        cdecl(h, FLUSH_CACHE, &[0]);
        result
    }

    /// The name hash: the sum of each (signed) character xor 0x6f times
    /// its index; with `lower`, A..Z count as a..z.
    fn name_hash(h: &impl Host, name: u32, lower: bool) -> u32 {
        let mut sum = 0_u32;
        let mut k = 0_u32;
        let mut at = name;
        loop {
            let mut c: u8 = h.read(at);
            if c == 0 {
                return sum;
            }
            if lower && c.wrapping_sub(0x41) <= 0x19 {
                c = c.wrapping_add(0x20);
            }
            let c = i32::from(c.cast_signed()).cast_unsigned();
            sum = sum.wrapping_add(c ^ k);
            k = k.wrapping_add(0x6f);
            at += 1;
        }
    }

    /// `MsDbgFile_hashNames_007e2d90(table)` -> the terminating entry's
    /// name pointer: for each 0x24-byte entry up to one whose name (`+0`)
    /// is empty, `+0x18` the name's hash, `+0x1c` its case-blind hash and
    /// `+0x20` the address of `+0x24`.
    pub fn dbg_file_hash_names(h: &mut impl Host, table: u32) -> u32 {
        let mut e = table;
        let mut i = 0_u32;
        loop {
            let name: u32 = h.read(e);
            if h.read::<u8>(name) == 0 {
                return name;
            }
            let a = name_hash(h, name, false);
            h.write(e + 0x18, a);
            let b = name_hash(h, name, true);
            h.write(e + 0x20, e + 0x24);
            h.write(e + 0x1c, b);
            i += 1;
            e = table.wrapping_add(i.wrapping_mul(0x24));
        }
    }

    /// `op_file_read(rec, name)` -> `sceClose`'s `eax`, or the negative
    /// descriptor: the file (`sceOpen(name, 1)`, descriptor at `+8`) is
    /// read whole into a zeroed `malloc` block (`+0`) of its size rounded
    /// up to 16 (`+0x10`, signed); `+4` and `+0xc` cleared, `+8` cleared
    /// after the close.
    pub fn op_file_read(h: &mut impl Host, rec: u32, name: u32) -> u32 {
        let fd = cdecl(h, SCE_OPEN, &[name, 1]);
        h.write(rec + 8, fd);
        if fd.cast_signed() < 0 {
            return fd;
        }
        h.write(rec + 4, 0_u32);
        let size = cdecl(h, SCE_LSEEK, &[fd, 0, 2]);
        let fd2: u32 = h.read(rec + 8);
        cdecl(h, SCE_LSEEK, &[fd2, 0, 0]);
        let n = size.wrapping_add(0xf).cast_signed();
        let n = (n + ((n >> 31) & 0xf)) & !0xf;
        let n = n.cast_unsigned();
        h.write(rec + 0x10, n);
        h.write(rec + 0xc, 0_u32);
        let malloc: u32 = h.read(h.rebase(MALLOC_SLOT));
        let p = h.call(malloc, Conv::Cdecl, &[n]);
        h.write(rec, p);
        let mut i = 0_u32;
        while i.cast_signed() < h.read::<u32>(rec + 0x10).cast_signed() {
            let p: u32 = h.read(rec);
            h.write(p.wrapping_add(i), 0_u8);
            i += 1;
        }
        let (fd, p, n): (u32, u32, u32) = (h.read(rec + 8), h.read(rec), h.read(rec + 0x10));
        cdecl(h, SCE_READ, &[fd, p, n]);
        let fd: u32 = h.read(rec + 8);
        let r = cdecl(h, SCE_CLOSE, &[fd]);
        h.write(rec + 8, 0_u32);
        r
    }

    // ---- Iggy GDraw D3D11 callbacks ----

    /// `gdraw_SetTextureUniqueID_00a3abc0(handle, old, new)` (stdcall) ->
    /// the handle's id after: replaced (`+0x14`) when it was `old`.
    pub fn set_texture_unique_id(h: &mut impl Host, handle: u32, old: u32, new: u32) -> u32 {
        let id: u32 = h.read(handle + 0x14);
        if id == old {
            h.write(handle + 0x14, new);
            return new;
        }
        id
    }

    /// `gdraw_Set3DTransform_00a3a9b0(m)` (stdcall) -> the `GDraw` state:
    /// with `m`, its 12 words go to the state's `+0x74c` and `+0x77c` is
    /// 1; without, `+0x77c` is 0.
    pub fn set_3d_transform(h: &mut impl Host, m: u32) -> u32 {
        let s = GDRAW.get(h);
        if m == 0 {
            h.write(s + 0x77c, 0_u32);
            return s;
        }
        h.write(s + 0x77c, 1_u32);
        for k in 0..12 {
            let v: u32 = h.read(m + 4 * k);
            h.write(s + 0x74c + 4 * k, v);
        }
        s
    }

    /// `gdraw_TryLock*` (`0x00a3b010`, `0x00a3b050`, `0x00a3b0c0`; stdcall
    /// `(handle, id, stats)`) -> 1/0: a handle in state 5 (the top nibble
    /// of `+0x30`) is usable; otherwise only when its id (`+0x14`) is
    /// `id`, and one in state 1 moves to the locked ring
    /// (`StoreComputed_00a3a010(handle, 2)`).
    pub fn try_lock(h: &mut impl Host, handle: u32, id: u32) -> u32 {
        let state = h.read::<u32>(handle + 0x30) >> 28;
        if state != 5 {
            if h.read::<u32>(handle + 0x14) != id {
                return 0;
            }
            if state == 1 {
                cdecl(h, GDRAW_RING_MOVE, &[handle, 2]);
            }
        }
        1
    }

    /// `gdraw_Unlock_00a3b100(handle, _, _)` (stdcall) -> `+0x30`'s value
    /// after: a handle in state 2 leaves its ring and goes to the front of
    /// its pool's (`[+0x18] + 0x48`), in state 1.
    pub fn unlock(h: &mut impl Host, handle: u32) -> u32 {
        let state = h.read::<u32>(handle + 0x30) & 0xf000_0000;
        if state != 0x2000_0000 {
            return state;
        }
        let next: u32 = h.read(handle + 0x20);
        let prev: u32 = h.read(handle + 0x1c);
        let pool: u32 = h.read(handle + 0x18);
        h.write(next + 0x1c, prev);
        let prev: u32 = h.read(handle + 0x1c);
        let next: u32 = h.read(handle + 0x20);
        h.write(prev + 0x20, next);
        let head = pool.wrapping_add(0x48);
        h.write(handle + 0x1c, head);
        let first: u32 = h.read(head + 0x20);
        h.write(handle + 0x20, first);
        h.write(first + 0x1c, handle);
        let prev: u32 = h.read(handle + 0x1c);
        h.write(prev + 0x20, handle);
        let v = (h.read::<u32>(handle + 0x30) & 0x0fff_ffff) | 0x1000_0000;
        h.write(handle + 0x30, v);
        v
    }

    /// `gdraw_UpdateTextureRect_00a3b160(handle, _, x, y, pitch, w, h_,
    /// data, _)` (stdcall, 9 words) -> the method's `eax`: the immediate
    /// context's `UpdateSubresource` (vtable `+0xc0`) on the handle's
    /// resource, subresource 0, with the box `(x, y, 0)..(x + w, y + h_,
    /// 1)` at `bx` (the original's frame, `ebp - 0x1c`).
    #[allow(clippy::too_many_arguments)]
    pub fn update_texture_rect(
        h: &mut impl Host,
        bx: u32,
        handle: u32,
        x: u32,
        y: u32,
        pitch: u32,
        w: u32,
        hh: u32,
        data: u32,
    ) -> u32 {
        for (k, v) in [x, y, 0, x.wrapping_add(w), y.wrapping_add(hh), 1]
            .into_iter()
            .enumerate()
        {
            h.write(bx + 4 * k as u32, v);
        }
        let ctx: u32 = h.read(GDRAW.get(h) + 4);
        let vt: u32 = h.read(ctx);
        let f: u32 = h.read(vt + 0xc0);
        let res: u32 = h.read(handle);
        h.call(f, Conv::Stdcall, &[ctx, res, 0, bx, data, pitch, 0])
    }

    /// `gdraw_reportD3DError_00a3c170(hr, what, where_)` -> the warning's
    /// `eax`: `IggyGDrawSendWarning(0, ...)`, the out-of-memory text for
    /// `E_OUTOFMEMORY`, else the generic one with the code.
    pub fn report_d3d_error(h: &mut impl Host, hr: u32, what: u32, where_: u32) -> u32 {
        let warn: u32 = h.read(h.rebase(SEND_WARNING_SLOT));
        if hr == E_OUTOFMEMORY {
            let fmt = h.rebase(OUT_OF_MEMORY_FORMAT);
            h.call(warn, Conv::Cdecl, &[0, fmt, what, where_])
        } else {
            let fmt = h.rebase(D3D_ERROR_FORMAT);
            h.call(warn, Conv::Cdecl, &[0, fmt, what, where_, hr])
        }
    }

    /// Zeroes the 0x30-byte stats at `stats` as the originals do: the
    /// first word's low half stored, the rest by `memset`.
    fn clear_stats(h: &mut impl Host, stats: u32) {
        h.write(stats, 0_u16);
        cdecl(h, MEMSET, &[stats + 2, 0, 0x2e]);
    }

    /// `gdraw_FreeAll_00a380b0()`: with a `GDraw` state, its resources freed
    /// (`GDraw_freeResources_00a36300` with zeroed stats at `stats`, the
    /// original's `ebp - 0x30`) and `+0x6c0`, `+0x6c4` cleared.
    pub fn free_all(h: &mut impl Host, stats: u32) {
        if GDRAW.get(h) == 0 {
            return;
        }
        clear_stats(h, stats);
        cdecl(h, GDRAW_FREE_RESOURCES, &[stats]);
        let s = GDRAW.get(h);
        h.write(s + 0x6c0, 0_u32);
        h.write(s + 0x6c4, 0_u32);
    }

    /// `gdraw_FreeRenderTarget_00a37ac0(t)` (stdcall) -> `FUN_00a3b480`'s
    /// `eax`: its two COM objects (`+4`, then `+0`) released (vtable `+8`)
    /// and cleared, then the handle freed with zeroed stats at `stats`
    /// (the original's `ebp - 0x30`).
    pub fn free_render_target(h: &mut impl Host, stats: u32, t: u32) -> u32 {
        for at in [t + 4, t] {
            let obj: u32 = h.read(at);
            if obj != 0 {
                let vt: u32 = h.read(obj);
                let f: u32 = h.read(vt + 8);
                h.call(f, Conv::Stdcall, &[obj]);
                h.write(at, 0_u32);
            }
        }
        clear_stats(h, stats);
        cdecl(h, GDRAW_FREE_HANDLE, &[t, stats])
    }
}
