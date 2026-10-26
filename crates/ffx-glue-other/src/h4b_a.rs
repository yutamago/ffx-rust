//! List `h4-glue-other`, second agent (h4b), batch 1: code without much
//! float work at `0x00800000` and up: the PS2 GS layer's display/draw
//! environments and its DMA wait, two GS image uploads (an image and the
//! debug font), the battle window's top-level drawer and the font macro
//! expansion `TODrawChangeMacro2`.
//!
//! Callees stay calls into the game (most are ported elsewhere and run
//! through their own hooks). Floats follow the x87 at 53 bits
//! (`ffx_bridge::x87`).

use ffx_bridge::x87::{add, load, store};
use ffx_graphic::host::{Conv, Host};

use crate::h4_e1::int_f32;

/// `crt_memset_thunk(p, c, n)`.
pub const MEMSET: u32 = 0x0094_964c;

// ---- TODrawChangeMacro2

/// `TOGetChangeMacroAddress(id, code)` (cdecl).
pub const GET_CHANGE_MACRO: u32 = 0x008b_6660;
/// `achievementUnlockAchievement(id)` (cdecl).
pub const UNLOCK_ACHIEVEMENT: u32 = 0x0082_2380;
/// `TOSkipCode(p)` (cdecl): the next code.
pub const SKIP_CODE: u32 = 0x008d_fd80;
/// `TOGetGroup(code)` (cdecl, reads the low byte).
pub const GET_GROUP: u32 = 0x008d_f610;
/// `TOGetFontInfo(text, out)` (cdecl).
pub const GET_FONT_INFO: u32 = 0x008b_7240;
/// `TOMakePktFont1612(pen, glyph)` (cdecl).
pub const MAKE_PKT_FONT: u32 = 0x008f_c7c0;
/// `g_TOFontGlyphInfo`: the 0x40-byte glyph record.
pub const GLYPH_INFO: u32 = 0x0184_1d50;

/// `TODrawChangeMacro2(window, code, pen)` (cdecl): expands the change
/// macro `code` of window `window` (`i16 +4` its id) into glyphs at `pen`
/// (`f32 +4` the x). The glyph record is cleared first. The Al Bhed
/// achievement (0x10) unlocks on map 0x9e with save bytes `'G'`/`'2'` and
/// latch byte `0x00c5a03d` set. Per code until the text ends or the
/// window's count `+0x9c` reaches its limit `+0xa0`: code 7 with an
/// argument byte `0xe0..0xf0` moves the pen right by `byte - 0xe0`; a
/// group-3 code expands recursively; any other is drawn
/// (`TOGetFontInfo`, `TOMakePktFont1612`), counts one, and with `+0xb4`
/// and `+0xac` clear raises the limit by one up to `+0xa8`.
///
/// Returns the original's `eax`: 0 without a macro, else the last value
/// it held (0x9e or the achievement call's, the count, the next code
/// pointer or the limit). The original also leaves the integer and then
/// the float of the last pen step in its first argument slot (`slot`,
/// when the caller's argument area is given).
pub fn change_macro2(h: &mut impl Host, win: u32, code: u32, pen: u32, slot: Option<u32>) -> u32 {
    let glyph = h.rebase(GLYPH_INFO);
    h.call_sym(MEMSET, Conv::Cdecl, &[glyph, 0, 0x40]);
    let id = i32::from(h.read::<i16>(win + 4)) as u32;
    let mut p = h.call_sym(GET_CHANGE_MACRO, Conv::Cdecl, &[id, code]);
    if p == 0 {
        return 0;
    }
    let mut eax = 0x9e;
    if h.read::<u16>(h.rebase(0x0112_ca90)) == 0x9e
        && h.read::<u8>(h.rebase(0x0112_cddb)) == 0x32
        && h.read::<u8>(h.rebase(0x0112_cdda)) == 0x47
        && h.read::<u8>(h.rebase(0x00c5_a03d)) == 1
    {
        let latch = h.rebase(0x00c5_a03d);
        h.write::<u8>(latch, 0);
        eax = h.call_sym(UNLOCK_ACHIEVEMENT, Conv::Cdecl, &[0x10]);
    }
    loop {
        let c = h.read::<u8>(p);
        if c == 0 {
            return eax;
        }
        let count: i32 = h.read(win + 0x9c);
        if count >= h.read::<i32>(win + 0xa0) {
            return count as u32;
        }
        if c == 7 {
            let b = h.read::<u8>(p + 1);
            if (0xe0..0xf0).contains(&b) {
                let n = i32::from(b) - 0xe0;
                let f = int_f32(n);
                if let Some(s) = slot {
                    h.write(s, f);
                }
                let x = store(add(load(f), load(h.read(pen + 4))));
                h.write(pen + 4, x);
            }
            p = h.call_sym(SKIP_CODE, Conv::Cdecl, &[p]);
            eax = p;
        } else if h.call_sym(GET_GROUP, Conv::Cdecl, &[u32::from(c)]) == 3 {
            change_macro2(h, win, p, pen, None);
            p = h.call_sym(SKIP_CODE, Conv::Cdecl, &[p]);
            eax = p;
        } else {
            let glyph = h.rebase(GLYPH_INFO);
            h.call_sym(GET_FONT_INFO, Conv::Cdecl, &[p, glyph]);
            h.call_sym(MAKE_PKT_FONT, Conv::Cdecl, &[pen, glyph]);
            p = h.call_sym(SKIP_CODE, Conv::Cdecl, &[p]);
            eax = p;
            let n: u32 = h.read(win + 0x9c);
            h.write(win + 0x9c, n.wrapping_add(1));
            if h.read::<u32>(win + 0xb4) == 0 && h.read::<u32>(win + 0xac) == 0 {
                let limit: i32 = h.read(win + 0xa0);
                eax = limit as u32;
                if limit < h.read::<i32>(win + 0xa8) {
                    eax = limit.wrapping_add(1) as u32;
                    h.write(win + 0xa0, eax);
                }
            }
        }
    }
}

// ---- TODrawBtlWindow

/// `MsBattleCursorCalc()`.
pub const BATTLE_CURSOR_CALC: u32 = 0x0078_0cc0;
/// `TOBtlInitGS()`.
pub const BTL_INIT_GS: u32 = 0x008f_3820;
/// `TOMakePktScissor(x, y, w, h)`.
pub const MAKE_PKT_SCISSOR: u32 = 0x008f_dee0;
/// `TODrawLimitBG()`.
pub const DRAW_LIMIT_BG: u32 = 0x008a_1ce0;
/// `TOSeqDraw(seq, layer, flag)`.
pub const SEQ_DRAW: u32 = 0x008a_e2f0;
/// `TOBtlDrawDamageValue2()`.
pub const DAMAGE_VALUE2: u32 = 0x0089_3290;
/// `TOBtlDrawSimpleHelpMes(which)`.
pub const SIMPLE_HELP_MES: u32 = 0x0089_d9b0;
/// `MsBattleGetStatus()`.
pub const BATTLE_GET_STATUS: u32 = 0x0078_16f0;
/// `TOBtlDrawStatusWin()`.
pub const STATUS_WIN: u32 = 0x0089_6450;
/// The battle window's drawers called in a row when the battle runs:
/// `TOBtlDrawEnemyLimit`, `TOBtlDrawLibraWindow`,
/// `TOBtlDrawAuronLimitWindow`, `TOBtlDrawTidusLimitWindow`,
/// `TOBtlDrawLuluLimitWindow`, `0x008955e0`.
pub const BTL_DRAWERS: [u32; 6] = [
    0x0089_33c0,
    0x0089_5390,
    0x0089_2740,
    0x0089_7240,
    0x0089_5660,
    0x0089_55e0,
];
/// `TOBtlDrawActiveChrCursor(chr)`.
pub const ACTIVE_CHR_CURSOR: u32 = 0x0089_2590;
/// `TOBtlDrawCtbWindow(a, b)`.
pub const CTB_WINDOW: u32 = 0x008a_0590;
/// `TOBtlDrawHelpWin()`.
pub const HELP_WIN: u32 = 0x0089_36d0;
/// `TOBtlDrawInfoWin(id, x, y)`.
pub const INFO_WIN: u32 = 0x0089_3a30;
/// `TOBtlDrawInfoWinMes(id)`.
pub const INFO_WIN_MES: u32 = 0x0089_5140;
/// `TODrawScissorXYWH(x, y, w, h)`.
pub const SCISSOR_XYWH: u32 = 0x008f_9230;
/// The battle cursors' drawer `0x008930f0` (tail-jumped to).
pub const BTL_CURSORS: u32 = 0x0089_30f0;
/// The eight battle sub-windows: `0xf0`-byte records from `0x0133c950`,
/// a draw callback at `+0x90`, a state byte at `+1` (drawn from 3 up).
pub const SUB_WINDOWS: u32 = 0x0133_c950;

/// Whether one of the limit windows (or the byte flag `0x0133c93f`) is
/// up: the third argument of `TOSeqDraw`.
fn limit_up(h: &impl Host) -> bool {
    h.read::<u16>(h.rebase(0x0133_d6f4)) != 0
        || h.read::<u16>(h.rebase(0x0133_d708)) != 0
        || h.read::<u16>(h.rebase(0x0133_d734)) != 0
        || h.read::<u8>(h.rebase(0x0133_c93f)) != 0
}

/// `TODrawBtlWindow()` (cdecl): the battle window when `0x0133c948` is
/// set: `MsBattleCursorCalc`, `TOBtlInitGS`, the full-screen scissor
/// packet, the limit background when a limit window is up (its words
/// tested first, the flag byte after), the sequence layer 0, the damage
/// values and both simple helps; then, while the battle status is 2 or
/// 1 (asked twice when not 2): the status window (byte `0x00c58ef4` 1),
/// the six limit and info drawers, and with the window open and enabled
/// (`0x023cc08c`, `0x00c58ef8` == 1) the active character's cursor
/// (unless `0x0133d6e0`), the CTB window, the eight sub-windows'
/// callbacks, the help and info windows; the scissor, sequence layer 1,
/// and unless `0x0133d6e0` the battle cursors (a tail jump).
///
/// `eax` is the last callee's; callers drop it.
pub fn draw_btl_window(h: &mut impl Host) {
    if h.read::<u32>(h.rebase(0x0133_c948)) == 0 {
        return;
    }
    h.call_sym(BATTLE_CURSOR_CALC, Conv::Cdecl, &[]);
    h.call_sym(BTL_INIT_GS, Conv::Cdecl, &[]);
    h.call_sym(MAKE_PKT_SCISSOR, Conv::Cdecl, &[0, 0, 0x200, 0x1a0]);
    let words = h.read::<u16>(h.rebase(0x0133_d6f4)) != 0
        || h.read::<u16>(h.rebase(0x0133_d708)) != 0
        || h.read::<u16>(h.rebase(0x0133_d734)) != 0;
    let flag = if words {
        h.call_sym(DRAW_LIMIT_BG, Conv::Cdecl, &[]);
        1
    } else {
        u32::from(h.read::<u8>(h.rebase(0x0133_c93f)) != 0)
    };
    h.call_sym(SEQ_DRAW, Conv::Cdecl, &[2, 0, flag]);
    h.call_sym(DAMAGE_VALUE2, Conv::Cdecl, &[]);
    h.call_sym(SIMPLE_HELP_MES, Conv::Cdecl, &[0]);
    h.call_sym(SIMPLE_HELP_MES, Conv::Cdecl, &[1]);
    if h.call_sym(BATTLE_GET_STATUS, Conv::Cdecl, &[]) != 2
        && h.call_sym(BATTLE_GET_STATUS, Conv::Cdecl, &[]) != 1
    {
        return;
    }
    if h.read::<u8>(h.rebase(0x00c5_8ef4)) == 1 {
        h.call_sym(STATUS_WIN, Conv::Cdecl, &[]);
    }
    for f in BTL_DRAWERS {
        h.call_sym(f, Conv::Cdecl, &[]);
    }
    if h.read::<u32>(h.rebase(0x023c_c08c)) != 0 && h.read::<u32>(h.rebase(0x00c5_8ef8)) == 1 {
        if h.read::<u32>(h.rebase(0x0133_d6e0)) == 0 {
            let chr = i32::from(h.read::<i16>(h.rebase(0x023c_c088))) as u32;
            h.call_sym(ACTIVE_CHR_CURSOR, Conv::Cdecl, &[chr]);
        }
        let b = i32::from(h.read::<i16>(h.rebase(0x025d_0a88))) as u32;
        let a = i32::from(h.read::<i16>(h.rebase(0x025d_0a8a))) as u32;
        h.call_sym(CTB_WINDOW, Conv::Cdecl, &[a, b]);
        let first = h.rebase(SUB_WINDOWS);
        for k in 0..8 {
            let w = first + 0xf0 * k;
            let f: u32 = h.read(w + 0x90);
            if h.read::<i8>(w + 1) >= 3 && f != 0 {
                h.call(f, Conv::Cdecl, &[w]);
            }
        }
        h.call_sym(HELP_WIN, Conv::Cdecl, &[]);
        let id = u32::from(h.read::<u16>(h.rebase(0x0133_d1f4)));
        h.call_sym(INFO_WIN, Conv::Cdecl, &[id, 0xb3, 0xf0]);
        let id = u32::from(h.read::<u16>(h.rebase(0x0133_d1f4)));
        h.call_sym(INFO_WIN_MES, Conv::Cdecl, &[id]);
    }
    h.call_sym(SCISSOR_XYWH, Conv::Cdecl, &[0, 0, 0x200, 0x1a0]);
    let flag = u32::from(limit_up(h));
    h.call_sym(SEQ_DRAW, Conv::Cdecl, &[2, 1, flag]);
    if h.read::<u32>(h.rebase(0x0133_d6e0)) == 0 {
        h.call_sym(BTL_CURSORS, Conv::Cdecl, &[]);
    }
}

// ---- Sg_MFIFOSync

/// `dbgPrintf(format, ...)` (cdecl, varargs).
pub const DBG_PRINTF: u32 = 0x0062_f500;
/// `scePrintf(format, ...)` (cdecl, varargs).
pub const SCE_PRINTF: u32 = 0x0062_fbf0;
/// `g_GsRegisterSink`: the emulated `D1_CHCR` (bit 8 busy).
pub const GS_REGISTER_SINK: u32 = 0x0230_56e8;
/// `g_SgFrame`.
pub const SG_FRAME: u32 = 0x023c_bbf0;
/// The PS2's VIF1 registers, `0x10003c00` + 16 k (absolute addresses,
/// unmapped on the PC: the timeout report would fault there).
pub const VIF1_REGS: u32 = 0x1000_3c00;

/// The timeout report's format strings, in order: a rule, `D1_CHCR`,
/// then 15 VIF1/DMA register lines, the rule again.
const MFIFO_FORMATS: [u32; 18] = [
    0x00b5_a6e8,
    0x00b5_a720,
    0x00b5_a750,
    0x00b5_a774,
    0x00b5_a798,
    0x00b5_a7bc,
    0x00b5_a7e0,
    0x00b5_a804,
    0x00b5_a828,
    0x00b5_a84c,
    0x00b5_a870,
    0x00b5_a894,
    0x00b5_a8b8,
    0x00b5_a8dc,
    0x00b5_a900,
    0x00b5_a924,
    0x00b5_a948,
    0x00b5_a6e8,
];

/// `Sg_MFIFOSync()` (cdecl): waits for the GS path's DMA: while bit 8 of
/// `g_GsRegisterSink` is set, up to 0x1234 rounds of a 0x4d3-step delay,
/// re-reading it each round (the sink is the PS2 register writes' dummy
/// target, so bit 8 is whatever the last such write left). On a timeout
/// it prints the VIF1 register report (`dbgPrintf`, the registers read
/// from the PS2 addresses `0x10003c00`.., outside the image and not
/// relocated: the port reads and writes the same addresses, so it faults
/// exactly where the original would when nothing maps them),
/// writes 9 to `VIF1_FBRST`, clears bit 8 and prints the frame
/// (`scePrintf`). Bits 2 and 3 are cleared at the end.
///
/// Returns the original's `eax`: the sink as last read, or after a
/// timeout `scePrintf`'s.
pub fn mfifo_sync(h: &mut impl Host) -> u32 {
    let sink = h.rebase(GS_REGISTER_SINK);
    let mut rounds = 0x1234_u32;
    let mut eax: u32 = h.read(sink);
    while eax & 0x100 != 0 {
        if rounds == 0 {
            break;
        }
        // the original's delay: a counter in memory counted down from
        // 0x4d2 to -1 (kept, so the wait lasts about as long)
        let mut delay = 0x4d2_i32;
        while core::hint::black_box(delay) >= 0 {
            delay -= 1;
        }
        rounds -= 1;
        eax = h.read(sink);
    }
    if rounds == 0 {
        eax = mfifo_timeout(h);
    }
    let v: u32 = h.read(sink);
    h.write(sink, v & 0xffff_fff3);
    eax
}

/// The timeout report of [`mfifo_sync`]; `scePrintf`'s `eax`.
fn mfifo_timeout(h: &mut impl Host) -> u32 {
    let sink = h.rebase(GS_REGISTER_SINK);
    let f0 = h.rebase(MFIFO_FORMATS[0]);
    h.call_sym(DBG_PRINTF, Conv::Cdecl, &[f0]);
    let s: u32 = h.read(sink);
    let f1 = h.rebase(MFIFO_FORMATS[1]);
    h.call_sym(DBG_PRINTF, Conv::Cdecl, &[f1, s]);
    for k in 0..15 {
        let s: u32 = h.read(sink);
        let reg: u32 = h.read(VIF1_REGS + 0x10 * k);
        let f = h.rebase(MFIFO_FORMATS[2 + k as usize]);
        h.call_sym(DBG_PRINTF, Conv::Cdecl, &[f, reg, s]);
    }
    let f = h.rebase(MFIFO_FORMATS[17]);
    h.call_sym(DBG_PRINTF, Conv::Cdecl, &[f]);
    h.write::<u32>(VIF1_REGS + 0x10, 9);
    let frame: u32 = h.read(h.rebase(SG_FRAME));
    let v: u32 = h.read(sink);
    h.write(sink, v & 0xffff_feff);
    let fmt = h.rebase(0x00b5_a96c);
    h.call_sym(SCE_PRINTF, Conv::Cdecl, &[fmt, frame])
}

// ---- the display and draw environments

/// `Sg_SetScreenWH(w, h)` (cdecl).
pub const SET_SCREEN_WH: u32 = 0x0082_1d10;
/// `sceGsSetDefDispEnv(env, psm, w, h, dx, dy)` (cdecl).
pub const SET_DEF_DISP_ENV: u32 = 0x0062_f7f0;
/// The draw environment's first half (`0x0062f800`, cdecl, 6 words).
pub const SET_DEF_DRAW_ENV1: u32 = 0x0062_f800;
/// `sceGsSetDefDrawEnv(env, psm, w, h, ztest, zpsm)` (cdecl).
pub const SET_DEF_DRAW_ENV: u32 = 0x0062_f810;
/// `Sg_VramGetAddrZ()` (cdecl).
pub const VRAM_GET_ADDR_Z: u32 = 0x0082_ce80;
/// `g_GuScreenSize`: the height (`+0`) and the width (`+4`).
pub const SCREEN_SIZE: u32 = 0x023c_bbe0;

/// `a` with its low nine bits from `b` (the original's `a ^ (a ^ b) & 0x1ff`).
fn low9(a: u32, b: u32) -> u32 {
    a ^ ((a ^ b) & 0x1ff)
}

/// `Sg_SetDispDrawEnv_008217d0(buffer)` (cdecl): sets the screen size
/// from the video mode byte `0x012fb8fb` (`i16` pairs at `0x00c494ac`),
/// then fills one of the two display/draw environment sets: odd `buffer`
/// the second (`0x012fb900`.., frame base 0), even the first
/// (`0x012fb928`.., frame base `w * h / 2048` toward zero). The display
/// environment, the draw environment's two halves (ztest 3 in mode 0,
/// else 0; zpsm 0x31), the frame bases, then the Z buffer's base
/// (`Sg_VramGetAddrZ() / 2048`) into both Z words. The original leaves
/// `buffer & 1` in its argument slot (`slot`).
///
/// Returns the last word's merge (`eax`): `(old ^ new) & 0x1ff` of the
/// second Z word.
pub fn set_disp_draw_env(h: &mut impl Host, buffer: u32, slot: Option<u32>) -> u32 {
    let mode = h.read::<u8>(h.rebase(0x012f_b8fb));
    let k = i32::from(mode as i8) as u32;
    let table = h.rebase(0x00c4_94ac);
    let sh = i32::from(h.read::<i16>(table.wrapping_add(k.wrapping_mul(4)).wrapping_add(2))) as u32;
    let sw = i32::from(h.read::<i16>(table.wrapping_add(k.wrapping_mul(4)))) as u32;
    h.call_sym(SET_SCREEN_WH, Conv::Cdecl, &[sw, sh]);
    let ztest = if mode == 0 { 3 } else { 0 };
    let size = h.rebase(SCREEN_SIZE);
    let height: u32 = h.read(size);
    let width: u32 = h.read(size + 4);
    let base = (height.wrapping_mul(width).cast_signed() / 2048) as u32;
    let odd = buffer & 1;
    if let Some(s) = slot {
        h.write(s, odd);
    }
    let (disp, regs, draw1, draw) = if odd != 0 {
        (0x012f_b900, 0x012f_ba80, 0x012f_ba90, 0x012f_baf0)
    } else {
        (0x012f_b928, 0x012f_b950, 0x012f_b960, 0x012f_b9c0)
    };
    let disp = h.rebase(disp);
    h.call_sym(
        SET_DEF_DISP_ENV,
        Conv::Cdecl,
        &[disp, 1, width, height, 0, 0xe],
    );
    if odd != 0 {
        let v: u32 = h.read(disp + 0x10);
        h.write(disp + 0x10, v & 0xffff_fe00);
    } else {
        let v: u32 = h.read(disp + 0x10);
        h.write(disp + 0x10, low9(v, base));
    }
    let regs = h.rebase(regs);
    let v: u32 = h.read(regs);
    h.write(regs, v & 0xffff_800c | 0x800c);
    let v: u32 = h.read(regs + 4);
    h.write(regs + 4, v & 0x0fff_bfff | 0x1000_0000);
    let v: u32 = h.read(regs + 8);
    h.write(regs + 8, v & 0xffff_fffe | 0xe);
    let draw1 = h.rebase(draw1);
    let draw = h.rebase(draw);
    h.call_sym(
        SET_DEF_DRAW_ENV1,
        Conv::Cdecl,
        &[draw1, 1, width, height, ztest, 0x31],
    );
    h.call_sym(
        SET_DEF_DRAW_ENV,
        Conv::Cdecl,
        &[draw, 1, width, height, ztest, 0x31],
    );
    let frame = if odd != 0 {
        h.read::<u32>(draw) & 0xffff_fe00
    } else {
        low9(h.read(draw), base)
    };
    let first = low9(h.read(draw1), frame);
    h.write(draw, frame);
    h.write(draw1, first);
    let z = h.call_sym(VRAM_GET_ADDR_Z, Conv::Cdecl, &[]);
    let zbase = (z.cast_signed() / 2048) as u32;
    let old2: u32 = h.read(draw1 + 0xc);
    let z1 = low9(h.read(draw + 0x10), zbase);
    let eax = (old2 ^ z1) & 0x1ff;
    h.write(draw + 0x10, z1);
    h.write(draw1 + 0xc, old2 ^ eax);
    eax
}

// ---- the GS image uploads

/// `sceDmaGetChan(id)` (cdecl; 0 on the PC).
pub const DMA_GET_CHAN: u32 = 0x0062_f6e0;
/// `sceDmaSend(chan, tag)` (cdecl; a bare `ret` on the PC).
pub const DMA_SEND: u32 = 0x0062_f710;
/// `sceDmaSendN(chan, data, qwc)` (cdecl).
pub const DMA_SEND_N: u32 = 0x0062_f720;
/// `sceDmaSync(chan, mode, timeout)` (cdecl).
pub const DMA_SYNC: u32 = 0x0062_f730;
/// `FlushCache(mode)` (cdecl).
pub const FLUSH_CACHE: u32 = 0x0062_f4c0;
/// `sceGsSyncPath(mode, timeout)` (cdecl).
pub const GS_SYNC_PATH: u32 = 0x0062_f880;
/// `TOTransImageHL`'s GIF packet (`0x50` bytes).
pub const TRANS_PACKET: u32 = 0x0186_5ea0;

/// `TOTransImageHL(dbp, dbw, psm, x, y, w, h)` (cdecl): uploads an
/// image through the GS: the transfer's size in quadwords from the
/// pixel format (`psm` 0: `w*h*4`, 1: `*3`, 2/10: `*2`, 0x13/0x1b: `w*h`,
/// 0x14/0x24/0x2c: `(w/2)*h`, any other: the DMA channel number;
/// divisions toward zero), the GIF packet at `0x01865ea0` (BITBLTBUF,
/// TRXPOS, TRXREG, TRXDIR and the image tag), and a DMA tag in the
/// caller's frame (`tag`, eight words: `0x30000006`, the packet, 0, 0,
/// the size, `dbp`, 0, 0) sent between two path syncs.
///
/// Returns the last `sceGsSyncPath`'s `eax`.
#[allow(clippy::too_many_arguments)]
pub fn trans_image_hl(
    h: &mut impl Host,
    tag: u32,
    dbp: u32,
    dbw: u32,
    psm: u32,
    x: u32,
    y: u32,
    w: u32,
    hh: u32,
) -> u32 {
    let chan = h.call_sym(DMA_GET_CHAN, Conv::Cdecl, &[2]);
    let (wi, hi) = (w.cast_signed(), hh.cast_signed());
    let bytes = match psm {
        0 => wi.wrapping_mul(hi).wrapping_shl(2),
        1 => wi.wrapping_mul(hi).wrapping_mul(3),
        2 | 10 => wi.wrapping_mul(hi).wrapping_mul(2),
        0x13 | 0x1b => wi.wrapping_mul(hi),
        0x14 | 0x24 | 0x2c => (wi / 2).wrapping_mul(hi),
        _ => chan.cast_signed(),
    };
    let qwc = (bytes / 16) as u32;
    let mut w64 = (wi / 64) as u32;
    if w64 == 0 {
        w64 = 1;
    }
    let packet = h.rebase(TRANS_PACKET);
    let words: [u32; 0x18] = [
        4,
        0x1000_0000,
        0xe,
        0,
        0,
        ((psm << 8 | w64) << 16) | dbw,
        0x50,
        0,
        0,
        y << 16 | x,
        0x51,
        0,
        w,
        hh | (wi >> 31) as u32,
        0x52,
        0,
        0,
        0,
        0x53,
        0,
        qwc | 0x8000,
        ((bytes / 16) >> 31) as u32 | 0x0800_0000,
        0x54,
        0,
    ];
    for (k, v) in words.iter().enumerate() {
        h.write(packet + 4 * k as u32, *v);
    }
    let dma = [0x3000_0006, packet, 0, 0, qwc, dbp, 0, 0];
    for (k, v) in dma.iter().enumerate() {
        h.write(tag + 4 * k as u32, *v);
    }
    h.call_sym(FLUSH_CACHE, Conv::Cdecl, &[0]);
    h.call_sym(GS_SYNC_PATH, Conv::Cdecl, &[0, 0]);
    h.call_sym(DMA_SEND, Conv::Cdecl, &[chan, tag]);
    h.call_sym(GS_SYNC_PATH, Conv::Cdecl, &[0, 0])
}

/// `Sg_GetDBuffDC(&frame)` (cdecl): the current display context; the
/// frame number through the pointer.
pub const GET_DBUFF_DC: u32 = 0x0082_0530;
/// The debug font: 16 bytes per glyph from `' '`, one bit per pixel.
pub const DEBUG_FONT: u32 = 0x025d_17f8;
/// The debug print's packet (`0x19350f0`: header and BITBLT registers,
/// the image from `0x1935160`) and its 8x16 RGBA glyph block
/// (`0x19390f0`).
pub const DEBUG_PACKET: u32 = 0x0193_50f0;
pub const DEBUG_GLYPH: u32 = 0x0193_90f0;

/// `yiPrintGs_0090aaa0(text, x, y, cols, rows, center, &colour)` (cdecl):
/// the assertion screen's text, one 8x16 glyph per character uploaded to
/// the GS: the display context (`Sg_GetDBuffDC(frame)`, `frame` a word of
/// the caller's frame) into `0x019350e8`; the colour from four bytes;
/// with `center` the line starts `strlen(text) * 4` left of `x`. Per
/// character: a newline or `cols` characters start a new line (`y` + 16,
/// at most `rows` lines); others (`' '..'~'`, anything else the first
/// glyph) are expanded to 8x16 words (`0xffffffff` for a set bit, else
/// the colour), the BITBLT packet built (its destination words left 0, as
/// the original computes and drops them), the glyph copied in, and the
/// packet sent (`FlushCache`, `sceDmaSync`, `sceDmaSendN`, `sceDmaSync`).
///
/// The original keeps its pen in its argument slots (`slots`: the
/// caller's first argument word): `y` + 16 per line in the third,
/// the pen's x in the seventh. Returns 0.
#[allow(clippy::too_many_arguments)]
pub fn print_gs(
    h: &mut impl Host,
    frame: u32,
    mut text: u32,
    x: u32,
    y: u32,
    cols: u32,
    rows: u32,
    center: u32,
    colour: u32,
    slots: Option<u32>,
) -> u32 {
    let dc = h.call_sym(GET_DBUFF_DC, Conv::Cdecl, &[frame]);
    let dc_cell = h.rebase(0x0193_50e8);
    let c: u32 = h.read(colour);
    h.write(dc_cell, dc);
    let mut pen = x;
    if center != 0 {
        let mut n = 0u32;
        while h.read::<u8>(text.wrapping_add(n)) != 0 {
            n += 1;
        }
        pen = x.wrapping_sub(n.wrapping_mul(8) >> 1);
    }
    let mut line_y = y;
    let mut col = 0u32;
    let mut row = 0u32;
    write_slots(h, slots, pen, line_y);
    loop {
        let ch = h.read::<u8>(text);
        if ch == 0 {
            return 0;
        }
        if ch == 0xa || col >= cols {
            pen = x;
            line_y = line_y.wrapping_add(0x10);
            row += 1;
            col = 0;
            write_slots(h, slots, pen, line_y);
            if ch == 0xa {
                text = text.wrapping_add(1);
            }
            if row >= rows {
                return 0;
            }
            continue;
        }
        let mut glyph = h.rebase(DEBUG_FONT);
        if (0x20..0x7e).contains(&ch) {
            glyph += (u32::from(ch) - 0x20) << 4;
        }
        let block = h.rebase(DEBUG_GLYPH);
        for r in 0..16 {
            let bits = h.read::<u8>(glyph + r);
            for b in 0..8 {
                let v = if bits & (0x80 >> b) != 0 {
                    0xffff_ffff
                } else {
                    c
                };
                h.write(block + 0x20 * r + 4 * b, v);
            }
        }
        debug_packet(h);
        let dc: u32 = h.read(dc_cell);
        let _env: u32 = if h.read::<u8>(frame) & 1 != 0 {
            h.read(dc + 0x38)
        } else {
            h.read(dc + 0x10)
        };
        let packet = h.rebase(DEBUG_PACKET);
        for k in 0..0x80 {
            let v: u32 = h.read(block + 4 * k);
            h.write(packet + 0x70 + 4 * k, v);
        }
        h.call_sym(FLUSH_CACHE, Conv::Cdecl, &[0]);
        let ch2 = h.call_sym(DMA_GET_CHAN, Conv::Cdecl, &[2]);
        h.call_sym(DMA_SYNC, Conv::Cdecl, &[ch2, 0, 0x100]);
        h.call_sym(GS_SYNC_PATH, Conv::Cdecl, &[0, 0]);
        let ch2 = h.call_sym(DMA_GET_CHAN, Conv::Cdecl, &[2]);
        h.call_sym(DMA_SEND_N, Conv::Cdecl, &[ch2, packet, 0x27]);
        h.call_sym(GS_SYNC_PATH, Conv::Cdecl, &[0, 0]);
        let ch2 = h.call_sym(DMA_GET_CHAN, Conv::Cdecl, &[2]);
        h.call_sym(DMA_SYNC, Conv::Cdecl, &[ch2, 0, 0x1000]);
        pen = pen.wrapping_add(8);
        write_slots(h, slots, pen, line_y);
        text = text.wrapping_add(1);
        col += 1;
    }
}

/// The pen words [`print_gs`] leaves in its argument slots.
fn write_slots(h: &mut impl Host, slots: Option<u32>, pen: u32, line_y: u32) {
    if let Some(s) = slots {
        h.write(s + 8, line_y);
        h.write(s + 0x18, pen);
    }
}

/// The header words of [`print_gs`]'s packet (`0x019350f0`..`0x01935160`).
fn debug_packet(h: &mut impl Host) {
    let words: [u32; 0x1c] = [
        5,
        0x1003_0000,
        0xe,
        0,
        0,
        0,
        0x50,
        0,
        0x2_0000,
        0,
        0x47,
        0,
        0,
        0,
        0x51,
        0,
        8,
        0,
        0x52,
        0,
        0,
        0,
        0x53,
        0,
        0x8020,
        0x0800_0000,
        0,
        0,
    ];
    let packet = h.rebase(DEBUG_PACKET);
    for (k, v) in words.iter().enumerate() {
        h.write(packet + 4 * k as u32, *v);
    }
}
