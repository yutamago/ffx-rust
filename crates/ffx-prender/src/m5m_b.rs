//! M5 stage 1, list `map`, batch 2: what the static call graph reaches of
//! the PS2 field layer's sound streaming (`SqsStraightStreaming*`), the
//! debug GUI's window helpers (`rcuiWindowCreate`, `rcuiInit`, the fog
//! panel's window), the GS packet send, the chunk file load, a few flag
//! setters, and the 5-byte `jmp` thunks the remaster left in front of
//! `Sqs*`, `rc*` and `yi*` functions.
//!
//! None of it ran in a coverage run. Callees stay calls to the originals
//! (through their hooks where they are ported), with the convention their
//! `ret` says; a thunk passes on every word its target reads.

use ffx_bridge::Global;

// ---- the Sqs straight stream (globals around g_SqsSifDma) ----

/// `g_SqsPlayMode`, `g_SqsDebugFlags`.
pub const PLAY_MODE: Global<u32> = Global::new(0x012f_4144);
pub const DEBUG_FLAGS: Global<u32> = Global::new(0x012f_4148);
/// `g_SqsSifDma`: two 16-byte SIF DMA records.
pub const SIF_DMA: u32 = 0x012f_4e00;
/// `g_SqsStraightStream`.
pub const STREAM_ID: Global<u32> = Global::new(0x012f_4e70);
/// The stream's state words (names ours, from these functions).
pub const STREAM_POS: Global<u32> = Global::new(0x012f_4e90);
pub const STREAM_SRC_BASE: Global<u32> = Global::new(0x012f_4ea0);
pub const STREAM_IOP_BASE: Global<u32> = Global::new(0x012f_4ec0);
pub const STREAM_IOP_DATA: Global<u32> = Global::new(0x012f_4ec4);
pub const STREAM_SIZE: Global<u32> = Global::new(0x012f_4ec8);
pub const SIF_DMA_ID: Global<u32> = Global::new(0x012f_4ed0);
pub const STREAM_HALF: Global<u32> = Global::new(0x012f_4ed4);
pub const STREAM_SENDS: Global<u32> = Global::new(0x012f_4ed8);
pub const STREAM_RUNNING: Global<u32> = Global::new(0x012f_4edc);
/// `0x012f4ee0`: the record `code_0081f760` returns (its `+0x14`, `+4..+0x10`).
pub const STREAM_RECORD: u32 = 0x012f_4ee0;
pub const STREAM_LAST_HALF: Global<u32> = Global::new(0x012f_4ee4);
pub const STREAM_LAST_ARG: Global<u32> = Global::new(0x012f_4ee8);
pub const STREAM_LAST_POS: Global<u32> = Global::new(0x012f_4eec);
pub const STREAM_LAST_WORD: Global<u32> = Global::new(0x012f_4ef0);
pub const STREAM_SRC: Global<u32> = Global::new(0x012f_4ef4);
pub const STREAM_SKIPPED: Global<u32> = Global::new(0x012f_4ef8);
pub const STREAM_HALF_MASK: Global<u32> = Global::new(0x012f_4efc);
pub const STREAM_PREV_HALF: Global<u32> = Global::new(0x012f_4f40);
/// `g_SqsAdpcmState` (four words).
pub const ADPCM_STATE: u32 = 0x012f_4f90;
/// The decoded PCM buffer `Sqs_DecodeAdpcm` fills.
pub const PCM: u32 = 0x012f_4fc0;
/// `g_SqsRpcSend` (pointer), `g_SqsCmdNext` (pointer).
pub const RPC_SEND: Global<u32> = Global::new(0x00c4_9458);
pub const CMD_NEXT: Global<u32> = Global::new(0x00c4_946c);
/// The command queue's end: `g_SqsSifDma` follows it.
pub const CMD_END: u32 = 0x012f_4e00;

/// `"%s %d, %4d, %4d : %8.8X\n"`, the function name, `"%s %d\n"`-style
/// prints and the queue-full message.
pub const FMT_SEND: u32 = 0x00b5_8f24;
pub const FMT_NEXT: u32 = 0x00b5_8f1c;
pub const FN_NAME: u32 = 0x00b5_8df4;
pub const QUEUE_FULL: u32 = 0x00b5_8d28;

pub const DBG_PRINTF: u32 = 0x0062_f500;
pub const FLUSH_CACHE: u32 = 0x0062_f4c0;
pub const DECODE_ADPCM: u32 = 0x0081_cc70;
pub const SIF_SET_DMA: u32 = 0x00a4_4790;
pub const SIF_DMA_STAT: u32 = 0x00a4_4770;
pub const CALL_RPC: u32 = 0x0081_cbd0;
pub const IOP_COMMAND_SEND: u32 = 0x0081_d5f0;
pub const REPORT_STANDBY: u32 = 0x0081_e260;

// ---- the debug GUI ----

pub const WINDOW_CREATE: u32 = 0x0084_a920;
pub const WINDOW_OPEN: u32 = 0x0084_b250;
pub const WINDOW_SET_TITLE: u32 = 0x0084_b6a0;
pub const WINDOW_SET_PARENT: u32 = 0x0084_b660;
pub const RC_CALLOC: u32 = 0x0092_58b0;
pub const RCUI_WINDOW_CREATE: u32 = 0x0093_7ef0;
/// `g_RcuiWork` and the free-memory window's holder (`Yn_FpMemWinOpen`).
pub const RCUI_WORK: Global<u32> = Global::new(0x0193_c0f0);
pub const FP_MEM_WINDOW: u32 = 0x0193_c0f8;
/// The windows' procedures and titles.
pub const FOG_PROC: u32 = 0x0091_62f0;
pub const FP_MEM_PROC: u32 = 0x0093_3aa0;
pub const FP_MEM_TITLE: u32 = 0x00b6_87c8;
pub const RCUI_PROC: u32 = 0x0093_5000;
pub const RCUI_TITLE: u32 = 0x00b5_b28c;
/// `rcuiInit`'s source file name for `rcCalloc`.
pub const RCUI_FILE: u32 = 0x00b6_84a0;

// ---- the field renderer and Yn ----

pub const YI_RENDER: u32 = 0x0187_6e00;
pub const YI_RENDER_INIT_FOG: u32 = 0x0091_3f50;
pub const YI_RENDER_STATUS: Global<u32> = Global::new(0x018d_ed08);
pub const YN_MAP_SYSTEM: Global<u32> = Global::new(0x0193_bf80);
/// `0x00638f50` / `0x00638f80`: `g_PhyreScene`'s methods `0x0065f0e0` /
/// `0x0066f380` (thiscall, no words) through a `mov ecx`/`jmp`.
pub const SCENE_CALL_CLEAR: u32 = 0x0063_8f50;
pub const SCENE_CALL_SET: u32 = 0x0063_8f80;
/// The vibration-curve callback's result (an address in the image).
pub const VIBRATE_CURVE: u32 = 0x00b6_6ee4;

// ---- GS / DMA ----

pub const MFIFO_SYNC: u32 = 0x0084_3360;
pub const GS_SYNC_PATH: u32 = 0x0062_f880;
pub const DMA_GET_CHAN: u32 = 0x0062_f6e0;
pub const DMA_SYNC: u32 = 0x0062_f730;
pub const DMA_SEND_N: u32 = 0x0062_f720;

// ---- the chunk file ----

pub const CHUNK_FINI: u32 = 0x0092_b7f0;
pub const SCE_OPEN: u32 = 0x0062_fa30;
pub const SCE_LSEEK: u32 = 0x0062_f8e0;
pub const RC_ALLOC: u32 = 0x0092_58a0;
pub const SCE_READ: u32 = 0x0062_fc00;
pub const CHUNK_SET: u32 = 0x0092_b920;
pub const SCE_CLOSE: u32 = 0x0062_f610;
/// `rcdtChunkLoadFromFile`'s source file name for `rcAlloc`.
pub const RCDT_FILE: u32 = 0x00b6_7c8c;

/// A 5-byte `jmp` thunk: its address, the target, and how many stack
/// words the target reads (passed on).
#[derive(Debug, Clone, Copy)]
pub struct Thunk {
    pub at: u32,
    pub target: u32,
    pub words: usize,
}

pub const SE_SEPARATE_MASK_CLEAR_THUNK: Thunk = Thunk {
    at: 0x0088_6dc0,
    target: 0x0081_e750,
    words: 8,
};
pub const SET_CYCLE_MODE_THUNK: Thunk = Thunk {
    at: 0x0088_7070,
    target: 0x0081_e900,
    words: 1,
};
pub const SET_REVERB_PLAY_SWITCH_THUNK: Thunk = Thunk {
    at: 0x0088_7170,
    target: 0x0081_ec20,
    words: 1,
};
pub const ALL_VOICE_TERMINATE_THUNK: Thunk = Thunk {
    at: 0x0088_72e0,
    target: 0x0081_ce70,
    words: 0,
};
pub const SHOUT_STREAMING_DATA_SEND_THUNK: Thunk = Thunk {
    at: 0x0088_7560,
    target: 0x0081_ed40,
    words: 2,
};
pub const YI_FREE: Thunk = Thunk {
    at: 0x0090_bbc0,
    target: 0x0068_7010,
    words: 1,
};
pub const RCBG_INIT_THUNK: Thunk = Thunk {
    at: 0x0091_9a70,
    target: 0x0092_1ce0,
    words: 0,
};
pub const FP_DISP_DEBUG_THUNK: Thunk = Thunk {
    at: 0x0091_9b70,
    target: 0x0072_9060,
    words: 0,
};
pub const FP_MEM_WIN_OPEN_THUNK: Thunk = Thunk {
    at: 0x0091_9b80,
    target: 0x0093_3c80,
    words: 0,
};
pub const PRE_BG_SET_DATA_THUNK_A: Thunk = Thunk {
    at: 0x0091_9b90,
    target: 0x0091_aa60,
    words: 1,
};
pub const TRX_PAIR_SET_NO_THUNK: Thunk = Thunk {
    at: 0x0091_9bd0,
    target: 0x0092_21e0,
    words: 2,
};
pub const PRE_BG_SET_DATA_THUNK_B: Thunk = Thunk {
    at: 0x0091_9d30,
    target: 0x0091_aa60,
    words: 1,
};
pub const SCENE_SET_CHADIST_THUNK: Thunk = Thunk {
    at: 0x0092_0780,
    target: 0x0092_0790,
    words: 1,
};
pub const RC_FREE: Thunk = Thunk {
    at: 0x0092_58e0,
    target: 0x0068_7010,
    words: 1,
};
pub const RC_FREE_THUNK: Thunk = Thunk {
    at: 0x0092_ba50,
    target: 0x0092_58e0,
    words: 1,
};
pub const TASK_RET_SEQ_CONT_THUNK: Thunk = Thunk {
    at: 0x0093_04d0,
    target: 0x0092_ee00,
    words: 1,
};

/// `x / 2` as `cdq; sub eax, edx; sar eax, 1` (toward zero).
#[must_use]
pub const fn half(x: u32) -> u32 {
    (x.cast_signed() / 2).cast_unsigned()
}

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::Mem;
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, f: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Cdecl, args)
    }

    fn word(h: &impl Mem, at: u32) -> u32 {
        h.read(at)
    }

    /// A `jmp` thunk: its target with the caller's words -> its `eax`.
    pub fn thunk(h: &mut impl Host, t: Thunk, args: &[u32]) -> u32 {
        cdecl(h, t.target, &args[..t.words]) // call width: 8 (the widest thunk)
    }

    /// `SqsStraightStreamingNextHalf_0081f160()` -> the half mask: the
    /// stream position (`0x012f4e90`) becomes the current half
    /// (`0x012f4ed4`, printed with debug flag 2) and the record's
    /// `0x012f4ee0`; the skip flag cleared, and while running set when the
    /// half changed with its bit already in the mask. The half's bit is
    /// set in the mask; returned without it while running.
    pub fn stream_next_half(h: &mut impl Host) -> u32 {
        let pos = STREAM_POS.get(h);
        STREAM_SKIPPED.set(h, 0);
        STREAM_HALF.set(h, pos);
        if DEBUG_FLAGS.get(h) & 2 != 0 {
            let (f, n) = (h.rebase(FMT_NEXT), h.rebase(FN_NAME));
            cdecl(h, DBG_PRINTF, &[f, n, pos]);
        }
        let half = STREAM_HALF.get(h);
        let mut mask = STREAM_HALF_MASK.get(h);
        h.write(h.rebase(STREAM_RECORD), half);
        if STREAM_RUNNING.get(h) == 0 {
            return mask;
        }
        let bit = 1_u32 << (half & 1);
        if STREAM_PREV_HALF.get(h) != half && mask & bit != 0 {
            STREAM_SKIPPED.set(h, 1);
        }
        mask |= bit;
        STREAM_PREV_HALF.set(h, half);
        STREAM_HALF_MASK.set(h, mask);
        if STREAM_RUNNING.get(h) == 0 {
            return mask;
        }
        !bit & mask
    }

    /// `SqsStraightStreamingRecord_0081f760()`: the source base copied to
    /// the record's source word -> the record (`0x012f4ee0`).
    pub fn stream_record(h: &mut impl Host) -> u32 {
        let src = STREAM_SRC_BASE.get(h);
        STREAM_SRC.set(h, src);
        h.rebase(STREAM_RECORD)
    }

    /// `SqsStraightStreamingDataSend(data, start, adpcm, arg)` (`slots`:
    /// its argument words, which it reuses): see `symbols/ffx.sym`.
    #[allow(clippy::too_many_lines)] // one original
    pub fn straight_data_send(
        h: &mut impl Host,
        slots: u32,
        data: u32,
        start: u32,
        adpcm: u32,
        arg: u32,
    ) -> u32 {
        let mode = PLAY_MODE.get(h);
        let iop = STREAM_IOP_BASE.get(h).wrapping_add(0x8000);
        let first = if mode == 0 { start } else { 0 };
        let mut mask = STREAM_HALF_MASK.get(h);
        let mut dst = STREAM_IOP_DATA.get(h);
        let mut dbg_half = 0_u32;
        let fresh;
        let shift;
        if STREAM_RUNNING.get(h) == 0 && mask == 3 {
            fresh = 0_u32;
            for g in [
                0x012f_4ed8_u32,
                0x012f_4f9c,
                0x012f_4f98,
                0x012f_4f94,
                0x012f_4f90,
                0x012f_4ed4,
            ] {
                h.write(h.rebase(g), 0_u32);
            }
            let mut id = 0;
            STREAM_ID.set(h, 0);
            if data != 0 {
                id = 1;
                STREAM_ID.set(h, 1);
            }
            if mode == 0 && first != 0 {
                id |= 2;
                STREAM_ID.set(h, id);
            }
            if adpcm != 0 {
                id |= 1;
                STREAM_ID.set(h, id);
            }
            STREAM_POS.set(h, 0);
            shift = 0;
        } else {
            let size = half(STREAM_SIZE.get(h));
            let other = !STREAM_HALF.get(h) & 1;
            dst = dst.wrapping_add(size.wrapping_mul(other));
            fresh = 1;
            dbg_half = other;
            shift = other;
        }
        h.write(slots + 4, fresh);
        mask &= !(1_u32 << (shift & 31));
        STREAM_HALF_MASK.set(h, mask);
        cdecl(h, FLUSH_CACHE, &[0]);
        let mut src = data;
        if adpcm != 0 {
            let pcm = h.rebase(PCM);
            cdecl(h, DECODE_ADPCM, &[adpcm, pcm]);
            src = pcm;
            h.write(slots, pcm);
            if fresh == 0 {
                h.write(adpcm, pcm);
            }
        }
        if src != 0 && PLAY_MODE.get(h) == 2 {
            // mono: each 0x200-byte left block averaged with its right one
            let end = src.wrapping_add(half(STREAM_SIZE.get(h)));
            let mut block = src;
            loop {
                let mut p = block;
                loop {
                    let l: i16 = h.read(p.wrapping_add(0x200));
                    let r: i16 = h.read(p);
                    let avg = (i32::from(r) + i32::from(l)).cast_unsigned() >> 1;
                    #[allow(clippy::cast_possible_truncation)] // the low half
                    let avg = avg as u16;
                    h.write(p.wrapping_add(0x200), avg);
                    h.write(p, avg);
                    p = p.wrapping_add(2);
                    if p >= block.wrapping_add(0x200) {
                        break;
                    }
                }
                block = block.wrapping_add(0x400);
                if block >= end {
                    break;
                }
            }
        }
        let dma = h.rebase(SIF_DMA);
        let mut rec = dma;
        let mut n = 0_u32;
        if first != 0 {
            h.write(dma + 4, iop);
            let size = STREAM_SIZE.get(h).cast_signed();
            let q = size / 0x3000;
            n = 1;
            h.write(dma, first);
            let len = if size - q * 0x3000 != 0 {
                if arg.cast_signed() % 6 != 0 {
                    0x1c00
                } else {
                    0x2300
                }
            } else {
                q.wrapping_mul(0x700).cast_unsigned()
            };
            h.write(dma + 8, len);
            h.write(dma + 0xc, 0_u32);
            rec = dma + 0x10;
        }
        let data = word(h, slots);
        if data != 0 {
            h.write(rec + 4, dst);
            n += 1;
            h.write(rec, data);
            let size = half(STREAM_SIZE.get(h));
            h.write(rec + 8, size);
            h.write(rec + 0xc, 0_u32);
        } else if adpcm != 0 {
            n += 1;
            h.write(rec, adpcm);
            let base = STREAM_IOP_BASE.get(h);
            h.write(rec + 4, base);
            h.write(rec + 8, 0x5000_u32);
            h.write(rec + 0xc, 0_u32);
        }
        if n != 0 {
            let id = loop {
                let id = cdecl(h, SIF_SET_DMA, &[dma, n]);
                if id != 0 {
                    break id;
                }
            };
            SIF_DMA_ID.set(h, id);
        }
        cdecl(h, FLUSH_CACHE, &[0]);
        // the wait: the original spins 0x10000 times every other poll
        let mut eax = loop {
            let id = SIF_DMA_ID.get(h);
            let s = cdecl(h, SIF_DMA_STAT, &[id]);
            if s.cast_signed() < 0 {
                break s;
            }
        };
        if first != 0 {
            let p = RPC_SEND.get(h);
            let fresh = word(h, slots + 4);
            h.write(p, fresh);
            h.write(p + 4, arg);
            eax = cdecl(h, CALL_RPC, &[0x11, 0]);
        }
        if data != 0 {
            if DEBUG_FLAGS.get(h) & 2 != 0 {
                let w = word(h, data);
                let (pos, sends) = (STREAM_HALF.get(h), STREAM_SENDS.get(h));
                let (f, name) = (h.rebase(FMT_SEND), h.rebase(FN_NAME));
                cdecl(h, DBG_PRINTF, &[f, name, dbg_half, sends, pos, w]);
            }
            STREAM_LAST_HALF.set(h, dbg_half);
            STREAM_LAST_ARG.set(h, arg);
            let pos = STREAM_HALF.get(h);
            STREAM_LAST_POS.set(h, pos);
            eax = word(h, data);
            STREAM_LAST_WORD.set(h, eax);
        }
        let n = STREAM_SENDS.get(h);
        STREAM_SENDS.set(h, n.wrapping_add(1));
        eax
    }

    /// `SqsStraightStreamingDigitalModeSet(x)` -> 0: the queue sent and the
    /// standby reported; when the next command would pass the queue's end,
    /// the queue-full message (debug flag 1) and the queue sent; command
    /// `(0x39, x, 0 ...)` queued and the queue sent.
    pub fn digital_mode_set(h: &mut impl Host, x: u32) -> u32 {
        cdecl(h, IOP_COMMAND_SEND, &[]);
        cdecl(h, REPORT_STANDBY, &[]);
        let mut c = CMD_NEXT.get(h);
        if c.wrapping_add(0x20) > h.rebase(CMD_END) {
            if DEBUG_FLAGS.get(h) & 1 != 0 {
                let m = h.rebase(QUEUE_FULL);
                cdecl(h, DBG_PRINTF, &[m]);
            }
            cdecl(h, IOP_COMMAND_SEND, &[]);
            c = CMD_NEXT.get(h);
        }
        h.write(c, 0x39_u32);
        h.write(c.wrapping_add(4), x);
        for k in 2..8 {
            h.write(c.wrapping_add(4 * k), 0_u32);
        }
        CMD_NEXT.set(h, c.wrapping_add(0x20));
        cdecl(h, IOP_COMMAND_SEND, &[]);
        0
    }

    /// `yiGui_setfogparam()`: the fog window
    /// (`Gu_WindowCreate(5, 5, 0xa0, 0x60, 0xe, yiGui_setfogparam_proc)`)
    /// -> it.
    pub fn set_fog_param(h: &mut impl Host) -> u32 {
        let proc = h.rebase(FOG_PROC);
        cdecl(h, WINDOW_CREATE, &[5, 5, 0xa0, 0x60, 0xe, proc])
    }

    /// `rcuiWindowCreate(holder, x, y, w, h, flags, proc, title, parent)`:
    /// an existing window (`*holder`) opened -> `Gu_WindowOpen`'s `eax`;
    /// else created into `*holder`, titled and, with a parent, parented
    /// -> `Gu_WindowSetParent`'s `eax` (0 without a parent).
    pub fn window_create(
        h: &mut impl Host,
        holder: u32,
        a: [u32; 6],
        title: u32,
        parent: u32,
    ) -> u32 {
        let w = word(h, holder);
        if w != 0 {
            return cdecl(h, WINDOW_OPEN, &[w]);
        }
        let w = cdecl(h, WINDOW_CREATE, &a);
        h.write(holder, w);
        cdecl(h, WINDOW_SET_TITLE, &[w, title]);
        if parent == 0 {
            return 0;
        }
        let w = word(h, holder);
        cdecl(h, WINDOW_SET_PARENT, &[w, parent])
    }

    fn rcui_window(h: &mut impl Host, holder: u32, a: [u32; 6], title: u32) -> u32 {
        let t = h.rebase(title);
        cdecl(
            h,
            RCUI_WINDOW_CREATE,
            &[holder, a[0], a[1], a[2], a[3], a[4], a[5], t, 0],
        )
    }

    /// `Yn_FpMemWinOpen()`: the free-memory window -> `rcuiWindowCreate`'s
    /// `eax`.
    pub fn fp_mem_win_open(h: &mut impl Host) -> u32 {
        let holder = h.rebase(FP_MEM_WINDOW);
        let proc = h.rebase(FP_MEM_PROC);
        rcui_window(h, holder, [8, 8, 0x8c, 0x1e, 6, proc], FP_MEM_TITLE)
    }

    /// `rcuiInit()` -> `g_RcuiWork`: the work area (`rcCalloc(1, 0xc0c)`),
    /// its window, and the defaults at `+0xae4..+0xb08`.
    pub fn rcui_init(h: &mut impl Host) -> u32 {
        let file = h.rebase(RCUI_FILE);
        let work = cdecl(h, RC_CALLOC, &[1, 0xc0c, file, 0x3d9]);
        RCUI_WORK.set(h, work);
        let proc = h.rebase(RCUI_PROC);
        rcui_window(h, work, [8, 8, 0x4c, 0xb7, 6, proc], RCUI_TITLE);
        for (off, v) in [
            (0xae4_u32, 0x3600_u32),
            (0xae8, 4),
            (0xaec, 4),
            (0xaf0, 8),
            (0xaf4, 8),
            (0xaf8, 0x3700),
            (0xafc, 0x100),
            (0xb00, 0x100),
            (0xb04, 1),
            (0xb08, 0),
        ] {
            let w = RCUI_WORK.get(h);
            h.write(w.wrapping_add(off), v);
        }
        RCUI_WORK.get(h)
    }

    /// `Native_8062_ClearGlobal018ded08Bit8`'s callee `code_00908440()`:
    /// bit 8 of `g_YiRenderStatus` cleared.
    pub fn yi_status_clear_bit8(h: &mut impl Host) {
        let v = YI_RENDER_STATUS.get(h);
        YI_RENDER_STATUS.set(h, v & !0x100);
    }

    /// `code_0090a420()` -> 0: `yiRender_initfog(&g_YiRender)`.
    pub fn yi_init_fog(h: &mut impl Host) -> u32 {
        let r = h.rebase(YI_RENDER);
        cdecl(h, YI_RENDER_INIT_FOG, &[r]);
        0
    }

    /// `code_009090b0()` -> `0x00b66ee4`.
    pub fn vibrate_curve(h: &mut impl Host) -> u32 {
        h.rebase(VIBRATE_CURVE)
    }

    /// `code_0091a200` / `code_0091a220` (natives 0x802d / 0x802c): the
    /// map's chain object (`g_YnMapSystem->map->+8`) gets bit 0 of its
    /// first word cleared (`on` false) or set, then a tail jump to
    /// `0x00638f50` / `0x00638f80` -> its `eax`; 0 when any link is 0.
    pub fn chain_bit0(h: &mut impl Host, on: bool) -> u32 {
        let sys = YN_MAP_SYSTEM.get(h);
        if sys == 0 {
            return 0;
        }
        let map = word(h, sys);
        if map == 0 {
            return 0;
        }
        let list = word(h, map.wrapping_add(8));
        if list == 0 {
            return 0;
        }
        let v = word(h, list);
        if on {
            h.write(list, v | 1);
            cdecl(h, SCENE_CALL_SET, &[])
        } else {
            h.write(list, v & !1);
            cdecl(h, SCENE_CALL_CLEAR, &[])
        }
    }

    /// `yiSendGsPacket(packet, qwc)` (`slots`: its argument words, which
    /// it clears for the tail call): `Sg_MFIFOSync`, `sceGsSyncPath(0, 0)`,
    /// DMA channel 2 synced, its CHCR `|= 0x41`, `&= 0xffffff73`,
    /// `FlushCache(0)`, `sceDmaSendN(chan, packet, qwc)`, then a tail jump
    /// to `sceGsSyncPath(0, 0)` -> its `eax`.
    pub fn send_gs_packet(h: &mut impl Host, slots: u32, packet: u32, qwc: u32) -> u32 {
        cdecl(h, MFIFO_SYNC, &[]);
        cdecl(h, GS_SYNC_PATH, &[0, 0]);
        let chan = cdecl(h, DMA_GET_CHAN, &[2]);
        cdecl(h, DMA_SYNC, &[chan, 0, 0x100]);
        let v = word(h, chan);
        h.write(chan, v | 0x41);
        let v = word(h, chan);
        h.write(chan, v & 0xffff_ff73);
        cdecl(h, FLUSH_CACHE, &[0]);
        cdecl(h, DMA_SEND_N, &[chan, packet, qwc]);
        h.write(slots + 4, 0_u32);
        h.write(slots, 0_u32);
        cdecl(h, GS_SYNC_PATH, &[0, 0])
    }

    /// `rcdtChunkLoadFromFile(chunk, path)` -> 1 when read: the chunk
    /// emptied; the file opened (`sceOpen(path, 1)`), sized (two
    /// `sceLseek`s), read into `rcAlloc(size)` and set as the chunk's
    /// data, closed. 0 when it doesn't open or the allocation fails.
    pub fn chunk_load_from_file(h: &mut impl Host, chunk: u32, path: u32) -> u32 {
        cdecl(h, CHUNK_FINI, &[chunk]);
        let f = cdecl(h, SCE_OPEN, &[path, 1]);
        if f == 0 {
            return 0;
        }
        let size = cdecl(h, SCE_LSEEK, &[f, 0, 2]);
        cdecl(h, SCE_LSEEK, &[f, 0, 0]);
        let file = h.rebase(RCDT_FILE);
        let buf = cdecl(h, RC_ALLOC, &[size, file, 0x55]);
        let mut ok = 0;
        if buf != 0 {
            cdecl(h, SCE_READ, &[f, buf, size]);
            cdecl(h, CHUNK_SET, &[chunk, buf, size]);
            ok = 1;
        }
        cdecl(h, SCE_CLOSE, &[f]);
        ok
    }
}
