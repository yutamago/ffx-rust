//! M5 stage 1, list `vfx`, round 5 batch 7: the effect debug number
//! printer (`opd_num*`): the queued printouts' draw and the hex digits;
//! the opus motion command.

use ffx_bridge::Global;

/// The printer's u16 state words (`0x0228614c` x, `0x0228614e` y,
/// `0x02286150` column, `0x02286160`/`0x02286162` the queue's origin,
/// `0x02286164` the queued count) and the queue's head and size.
pub const PRINT_X: Global<u16> = Global::new(0x0228_614c);
pub const PRINT_Y: Global<u16> = Global::new(0x0228_614e);
pub const PRINT_COLUMN: Global<u16> = Global::new(0x0228_6150);
pub const QUEUE_X: Global<u16> = Global::new(0x0228_6160);
pub const QUEUE_Y: Global<u16> = Global::new(0x0228_6162);
pub const QUEUE_COUNT: Global<u16> = Global::new(0x0228_6164);
pub const QUEUE_HEAD: Global<u32> = Global::new(0x0228_6168);
pub const QUEUE_LIMIT: Global<u32> = Global::new(0x0228_616c);
/// The 32 queued printouts: 0x200-byte texts at `0x02288230 + 0x2000 i`,
/// each followed by 16 argument words 0x200 apart.
pub const QUEUE: u32 = 0x0228_8230;
/// The hex digits' glyph columns and rows.
pub const GLYPH_U: u32 = 0x00c8_7670;
pub const GLYPH_V: u32 = 0x00c8_7770;

// ---- callees ----

/// `FUN_00a5e4a0()`, `opd_num(text, 16 words)` (cdecl, 17 words: the
/// wide call), `opd_num_tex_tag_00a5e7b0(p, n, pal)` (cdecl).
pub const PREPARE: u32 = 0x00a5_e4a0;
pub const OPD_NUM: u32 = 0x00a5_d3c0;
pub const TEX_TAG: u32 = 0x00a5_e7b0;
/// `g_EtChrHandles` (240 words).
pub const ET_CHR_HANDLES: u32 = 0x012a_8100;
/// `opus_copy_1k_00800af0(src, dst)`, `opus_copy_1k_00800ab0(dst, src)`,
/// `Ch_DrawMainEffect(magic)`, `Ch_DrawSetSituation(chr, n)`,
/// `Ch_EnClip(chr, on)` (cdecl).
pub const COPY_1K_SAVE: u32 = 0x0080_0af0;
pub const COPY_1K_RESTORE: u32 = 0x0080_0ab0;
pub const DRAW_MAIN_EFFECT: u32 = 0x0082_6950;
pub const DRAW_SET_SITUATION: u32 = 0x0083_0160;
pub const EN_CLIP: u32 = 0x0082_aa30;

pub mod bridge {
    use super::{
        COPY_1K_RESTORE, COPY_1K_SAVE, DRAW_MAIN_EFFECT, DRAW_SET_SITUATION, EN_CLIP,
        ET_CHR_HANDLES, GLYPH_U, GLYPH_V, OPD_NUM, PREPARE, PRINT_COLUMN, PRINT_X, PRINT_Y, QUEUE,
        QUEUE_COUNT, QUEUE_HEAD, QUEUE_LIMIT, QUEUE_X, QUEUE_Y, TEX_TAG,
    };
    use ffx_graphic::host::{Conv, Host};

    /// `opd_num_draw_queue_00a5e2e0()` -> the saved column: after
    /// `FUN_00a5e4a0`, the printer starts at the queue's origin and prints
    /// the last `min(count, limit)` queued texts (a 32-entry ring ending at
    /// the head) with their 16 words; its position and column are put
    /// back.
    pub fn draw_queue(h: &mut impl Host) -> u32 {
        h.call_sym(PREPARE, Conv::Cdecl, &[]);
        let (y, x) = (PRINT_Y.get(h), PRINT_X.get(h));
        let mut n = u32::from(QUEUE_COUNT.get(h));
        let column = PRINT_COLUMN.get(h);
        let qx = QUEUE_X.get(h);
        PRINT_X.set(h, qx);
        let qy = QUEUE_Y.get(h);
        PRINT_Y.set(h, qy);
        PRINT_COLUMN.set(h, 0);
        let limit = QUEUE_LIMIT.get(h);
        if n > limit {
            n = limit;
        }
        let mut k = QUEUE_HEAD.get(h).wrapping_sub(n) & 0x1f;
        let queue = h.rebase(QUEUE);
        let mut left = n;
        while left.cast_signed() > 0 {
            let text = queue + (k << 13);
            let mut args = vec![text];
            for w in 0..16 {
                let v: u32 = h.read(text + 0x200 * (w + 1));
                args.push(v);
            }
            // call width: 17
            h.call_sym_wide(OPD_NUM, &args);
            k = (k + 1) & 0x1f;
            left -= 1;
        }
        PRINT_Y.set(h, y);
        PRINT_X.set(h, x);
        PRINT_COLUMN.set(h, column);
        u32::from(column)
    }

    /// `opd_num_put_hex_00a5eea0(value, n, ctx, font)` -> `ctx`, or the new
    /// column for no digits: `n` hex digits of `value` (lowest first, right
    /// to left from the column `ctx +0x10`) as sprites after a texture tag
    /// (`ctx +8`), the glyph cells from `0x00c87670`/`0x00c87770`, the
    /// font's cell size (`+0x1a`, `+0x1b`), offsets (`+0x16`, `+0x18`) and
    /// colour (`+4`); the column advances by `n`, the packet end goes to
    /// `ctx +0xc`. The argument slots (`slots`) hold the shifted value, the
    /// count and the row.
    pub fn put_hex(h: &mut impl Host, slots: u32, value: u32, n: u32, ctx: u32, font: u32) -> u32 {
        let p: u32 = h.read(ctx + 8);
        let mut out = p + 0x40;
        h.call_sym(TEX_TAG, Conv::Cdecl, &[p, n, 1]);
        let column = u32::from(h.read::<u16>(ctx + 0x10));
        let colour: u32 = h.read(font + 4);
        let cw = u32::from(h.read::<u8>(font + 0x1a));
        let ch = u32::from(h.read::<u8>(font + 0x1b));
        let mut x = n
            .wrapping_sub(1)
            .wrapping_add(column)
            .wrapping_mul(cw)
            .wrapping_sub((u32::from(h.read::<u16>(font + 0x16)) & !1) << 3);
        let row = u32::from(h.read::<u16>(ctx + 0x12));
        let y = row
            .wrapping_mul(ch)
            .wrapping_sub((u32::from(h.read::<u16>(font + 0x18)) & !1) << 3)
            .wrapping_add(0x8000);
        h.write(slots + 0xc, y);
        let next = column.wrapping_add(n);
        h.write(ctx + 0x10, next as u16);
        x = x.wrapping_add(0x8000);
        if n.cast_signed() <= 0 {
            h.write(ctx + 0xc, out);
            return next;
        }
        let (gu, gv) = (h.rebase(GLYPH_U), h.rebase(GLYPH_V));
        let mut value = value;
        let mut left = n;
        while left.cast_signed() > 0 {
            let d = value & 0xf;
            value = (value.cast_signed() >> 4).cast_unsigned();
            h.write(slots, value);
            let u = h.read::<u32>(gu + 4 * d) << 4;
            let v = h.read::<u32>(gv + 4 * d) << 4;
            h.write(out + 0x14, y);
            h.write(out + 0x18, colour);
            h.write(out + 0x20, u.wrapping_add(0x80));
            h.write(out + 0x10, x);
            h.write(out, u);
            h.write(out + 4, v);
            h.write(out + 0x24, v.wrapping_add(0x80));
            h.write(out + 0x30, cw.wrapping_add(x));
            h.write(out + 0x34, ch.wrapping_add(y));
            h.write(out + 0x38, colour);
            out += 0x40;
            x = x.wrapping_sub(cw);
            left -= 1;
            h.write(slots + 4, left);
        }
        h.write(ctx + 0xc, out);
        ctx
    }

    /// `opu_motion_etc_00804b70(host, cmd)` -> the next command, by the
    /// nibble of `+0x21a`: 0 sets the unit's character (`g_EtChrHandles`
    /// by its `+0x18`) situation `i16 cmd +2`; 0x1000 draws main effect
    /// `i16 cmd +2` with the host's first KiB saved to `buf` (the
    /// original's 16-aligned `ebp - 0x410`) and put back; 0x2000 / 0x3000
    /// turn the character's clipping on / off (2 bytes); others stay.
    pub fn motion_etc(h: &mut impl Host, buf: u32, host: u32, cmd: u32) -> u32 {
        let nibble = h.read::<u16>(host + 0x21a) & 0xf000;
        let unit: u32 = h.read(host + 0x220);
        match nibble {
            0 => {
                let k = u32::from(h.read::<u8>(unit + 0x18));
                let chr: u32 = h.read(h.rebase(ET_CHR_HANDLES) + 4 * k);
                let n = i32::from(h.read::<i16>(cmd + 2)).cast_unsigned();
                h.call_sym(DRAW_SET_SITUATION, Conv::Cdecl, &[chr, n]);
                cmd + 4
            }
            0x1000 => {
                h.call_sym(COPY_1K_SAVE, Conv::Cdecl, &[host, buf]);
                let magic = i32::from(h.read::<i16>(cmd + 2)).cast_unsigned();
                h.call_sym(DRAW_MAIN_EFFECT, Conv::Cdecl, &[magic]);
                h.call_sym(COPY_1K_RESTORE, Conv::Cdecl, &[host, buf]);
                cmd + 4
            }
            0x2000 | 0x3000 => {
                let k = u32::from(h.read::<u8>(unit + 0x18));
                let chr: u32 = h.read(h.rebase(ET_CHR_HANDLES) + 4 * k);
                let on = u32::from(nibble == 0x2000);
                h.call_sym(EN_CLIP, Conv::Cdecl, &[chr, on]);
                cmd + 2
            }
            _ => cmd,
        }
    }
}
