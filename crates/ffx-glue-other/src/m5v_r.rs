//! M5 stage 1, list `vfx`, round 5 batch 13: the effect debug number
//! printer's front end (`opd_num`, a `printf` of `%x`, `%d`, `%f` and text
//! to GS sprites) and its decimal and text emitters; the effect dump's
//! flag word printer; the font upload.

use ffx_bridge::Global;

/// The printer's font record (`0x02286140`: `+0` its packet, `+4` the
/// colour, `+0x16`/`+0x18` the offsets, `+0x1a`/`+0x1b` the cell size),
/// its display switch and the print position words.
pub const FONT: u32 = 0x0228_6140;
pub const FONT_PACKET: Global<u32> = Global::new(0x0228_6140);
pub const DISPLAY: Global<u16> = Global::new(0x0228_6154);
pub const PRINT_X: Global<u16> = Global::new(0x0228_614c);
pub const PRINT_Y: Global<u16> = Global::new(0x0228_614e);
pub const PRINT_COLUMN: Global<u16> = Global::new(0x0228_6150);
/// The emitters' context (`0x02322760`: `+0` the text, `+4` the next
/// character, `+8` the packet, `+0xc` its end, `+0x10`/`+0x12` the column
/// and row, `+0x16`).
pub const CONTEXT: u32 = 0x0232_2760;
/// The DMA channel the packet goes to.
pub const DMA_CHANNEL: Global<u32> = Global::new(0x0228_81d0);
/// The digit and character glyph cells, the blank cell.
pub const GLYPH_U: u32 = 0x00c8_7670;
pub const GLYPH_V: u32 = 0x00c8_7770;
pub const BLANK_U: Global<u32> = Global::new(0x00c8_7728);
pub const BLANK_V: Global<u32> = Global::new(0x00c8_7828);

/// `opd_debug_flags_str_00a65570`'s texts: the four flag letters with
/// bit 0 set and clear, the dot, the separator, an operand and a blank.
pub const FLAG_TEXTS_SET: [u32; 4] = [0x00b5_639c, 0x00b5_63a0, 0x00b5_63a4, 0x00b7_bf9c];
pub const FLAG_TEXTS_CLEAR: [u32; 4] = [0x00b7_bfa0, 0x00b5_b24c, 0x00b0_db48, 0x00b3_0da0];
pub const FLAG_DOT: u32 = 0x00b8_f888;
pub const FLAG_SEPARATOR: u32 = 0x00b5_7fbc;
pub const OPERAND_FORMAT: u32 = 0x00b7_bfa4;
pub const NO_OPERAND: u32 = 0x00b7_bfac;
/// `MSVCR110!sprintf` (the slot).
pub const SPRINTF_SLOT: u32 = 0x00b0_c3dc;

/// The font upload: the load-image record, the five images' 7-word
/// layouts (`0x00c87870`, 0x10 apart) and their pixels, the DMA packet
/// (`0x02288180..`) and its count word.
pub const LOAD_IMAGE: u32 = 0x0228_81e0;
pub const IMAGE_LAYOUTS: u32 = 0x00c8_7870;
pub const IMAGE_PIXELS: [u32; 5] = [
    0x00c8_7d90,
    0x00c8_8590,
    0x00c8_85d0,
    0x00c8_8610,
    0x00c8_8650,
];
pub const UPLOAD_PACKET: u32 = 0x0228_8180;
pub const UPLOAD_COUNT: Global<u32> = Global::new(0x0228_8170);
/// `sceGsSetDefLoadImage(li, ...7)`, `sceGsExecLoadImage(li, src)`,
/// `sceDmaSendN(ch, p, n)` (cdecl).
pub const SET_DEF_LOAD_IMAGE: u32 = 0x0062_f820;
pub const EXEC_LOAD_IMAGE: u32 = 0x0062_f7a0;
pub const DMA_SEND_N: u32 = 0x0062_f720;

// ---- callees ----

/// `sceGsSyncPath(mode, timeout)`, `FlushCache(n)`, `sceDmaSend(ch, p)`,
/// `op_gs1_transfer()` (cdecl).
pub const GS_SYNC_PATH: u32 = 0x0062_f880;
pub const FLUSH_CACHE: u32 = 0x0062_f4c0;
pub const DMA_SEND: u32 = 0x0062_f710;
pub const GS1_TRANSFER: u32 = 0x007e_b580;
/// The emitters: `opd_num_put_text_00a5f040(ctx, font)`,
/// `opd_num_put_hex_00a5eea0(v, n, ctx, font)`,
/// `opd_num_put_dec_00a5e880(v, n, ctx, font)`,
/// `opd_num_put_float_00a5ea30(lo, hi, prec, n, ctx, font)`,
/// `opd_num_tex_tag_00a5e7b0(p, n, pal)`, `opd_num_end_tag_00a5f2d0(p, end)`
/// (cdecl).
pub const PUT_TEXT: u32 = 0x00a5_f040;
pub const PUT_HEX: u32 = 0x00a5_eea0;
pub const PUT_DEC: u32 = 0x00a5_e880;
pub const PUT_FLOAT: u32 = 0x00a5_ea30;
pub const TEX_TAG: u32 = 0x00a5_e7b0;
pub const END_TAG: u32 = 0x00a5_f2d0;

pub mod bridge {
    use super::{
        BLANK_U, BLANK_V, CONTEXT, DISPLAY, DMA_CHANNEL, DMA_SEND, DMA_SEND_N, END_TAG,
        EXEC_LOAD_IMAGE, FLAG_DOT, FLAG_SEPARATOR, FLAG_TEXTS_CLEAR, FLAG_TEXTS_SET, FLUSH_CACHE,
        FONT, FONT_PACKET, GLYPH_U, GLYPH_V, GS_SYNC_PATH, GS1_TRANSFER, IMAGE_LAYOUTS,
        IMAGE_PIXELS, LOAD_IMAGE, NO_OPERAND, OPERAND_FORMAT, PRINT_COLUMN, PRINT_X, PRINT_Y,
        PUT_DEC, PUT_FLOAT, PUT_HEX, PUT_TEXT, SET_DEF_LOAD_IMAGE, SPRINTF_SLOT, TEX_TAG,
        UPLOAD_COUNT, UPLOAD_PACKET,
    };
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// One glyph sprite (0x40 bytes at `out`): cell `(u, v)` (in 16ths, the
    /// far corner 8 texels on) at `(x, y)`, `w` x `h` big, in `colour`.
    #[allow(clippy::too_many_arguments)]
    fn sprite(
        h: &mut impl Host,
        out: u32,
        u: u32,
        v: u32,
        x: u32,
        y: u32,
        w: u32,
        hh: u32,
        colour: u32,
    ) {
        let (u, v) = (u << 4, v << 4);
        for (off, val) in [
            (0, u),
            (0x20, u.wrapping_add(0x80)),
            (0x14, y),
            (4, v),
            (0x10, x),
            (0x18, colour),
            (0x24, v.wrapping_add(0x80)),
            (0x30, w.wrapping_add(x)),
            (0x34, hh.wrapping_add(y)),
            (0x38, colour),
        ] {
            h.write(out + off, val);
        }
    }

    /// The start of a line of cells at the context's column and row, in the
    /// font's cell size less its offsets, in 1/16 units from 0x8000.
    fn origin(h: &impl Host, ctx: u32, font: u32) -> (u32, u32) {
        let cw = u32::from(h.read::<u8>(font + 0x1a));
        let ch = u32::from(h.read::<u8>(font + 0x1b));
        let x = u32::from(h.read::<u16>(ctx + 0x10))
            .wrapping_mul(cw)
            .wrapping_sub((u32::from(h.read::<u16>(font + 0x16)) & !1) << 3)
            .wrapping_add(0x8000);
        let y = u32::from(h.read::<u16>(ctx + 0x12))
            .wrapping_mul(ch)
            .wrapping_sub((u32::from(h.read::<u16>(font + 0x18)) & !1) << 3)
            .wrapping_add(0x8000);
        (x, y)
    }

    /// `opd_num_put_dec_00a5e880(value, n, ctx, font)` -> `ctx`: `value`
    /// (negative: 99999999) in `n` decimal digits (reduced modulo `10^n`),
    /// leading zeros blank but the last, as sprites after the texture tag;
    /// the column advances by `n`. The argument slots (`slots`) hold the
    /// remaining value and count. (`n` of 32 or more divides by 0 in the
    /// original; here the remainder is then 0.)
    pub fn put_dec(h: &mut impl Host, slots: u32, value: u32, n: u32, ctx: u32, font: u32) -> u32 {
        let p: u32 = h.read(ctx + 8);
        let mut out = p + 0x40;
        let colour: u32 = h.read(font + 4);
        let (mut x, y) = origin(h, ctx, font);
        let mut div = 1_i32;
        let mut k = n.cast_signed();
        while k > 0 {
            div = div.wrapping_mul(10);
            k -= 1;
        }
        let mut value = value.cast_signed();
        if value < 0 {
            value = 99_999_999;
            h.write(slots, value.cast_unsigned());
        }
        let max = u64::from_ne_bytes(i64::from(div.wrapping_sub(1)).to_ne_bytes());
        let wide = |v: i32| u64::from_ne_bytes(i64::from(v).to_ne_bytes());
        if wide(value) > max {
            loop {
                value = value.checked_rem(div).unwrap_or(0);
                if wide(value) <= max {
                    break;
                }
            }
            h.write(slots, value.cast_unsigned());
        }
        let mut count = 0_u32;
        let mut started = false;
        let mut left = n.cast_signed();
        while left > 0 {
            div /= 10;
            let q = value.checked_div(div).unwrap_or(0);
            value = value.checked_rem(div).unwrap_or(0);
            h.write(slots, value.cast_unsigned());
            let (u, v) = if !started && left > 1 && q == 0 {
                (BLANK_U.get(h), BLANK_V.get(h))
            } else {
                started = true;
                let q = q.cast_unsigned();
                (
                    h.read(h.rebase(GLYPH_U) + 4 * q),
                    h.read(h.rebase(GLYPH_V) + 4 * q),
                )
            };
            let cw = u32::from(h.read::<u8>(font + 0x1a));
            let ch = u32::from(h.read::<u8>(font + 0x1b));
            sprite(h, out, u, v, x, y, cw, ch, colour);
            out += 0x40;
            count += 1;
            x = x.wrapping_add(cw);
            left -= 1;
            h.write(slots + 4, left.cast_unsigned());
        }
        cdecl(h, TEX_TAG, &[p, count, 0]);
        let column: u16 = h.read(ctx + 0x10);
        h.write(ctx + 0x10, column.wrapping_add(count as u16));
        h.write(ctx + 0xc, out);
        ctx
    }

    /// `opd_num_put_text_00a5f040(ctx, font)` -> the glyphs drawn: the text
    /// from `ctx +0` up to a `%` or `$` (a newline ends it and is cleared),
    /// as sprites after the texture tag (the font record `0x02286140`;
    /// digits, letters, `- : , . ( ) _ /`; spaces and tabs (to the next
    /// multiple of 8) skip cells). `+4` gets where it stopped, `+0xc` the
    /// packet's end, the column the cells used. (A tab with a zero cell
    /// width divides by 0 in the original; here it skips nothing.)
    pub fn put_text(h: &mut impl Host, ctx: u32) -> u32 {
        let font = h.rebase(FONT);
        let p: u32 = h.read(ctx + 8);
        let mut out = p + 0x40;
        let text: u32 = h.read(ctx);
        let colour: u32 = h.read(font + 4);
        let (mut x, y) = origin(h, ctx, font);
        let mut skipped = 0_u32;
        let mut count = 0_u32;
        let mut i = 0_u32;
        let mut c: u8 = h.read(text);
        if c != 0 {
            loop {
                if c == b'\n' {
                    h.write(text + i, 0_u8);
                    break;
                }
                if c == b'%' || c == b'$' {
                    break;
                }
                let cw = u32::from(h.read::<u8>(font + 0x1a));
                let glyph = match c {
                    b'-' => Some(0x24),
                    b':' => Some(0x25),
                    b',' => Some(0x26),
                    b'.' => Some(0x27),
                    b'(' => Some(0x28),
                    b')' => Some(0x29),
                    b'_' => Some(0x2a),
                    b'/' => Some(0x2b),
                    b' ' => {
                        skipped += 1;
                        x = x.wrapping_add(cw);
                        None
                    }
                    b'\t' => {
                        let q = x.cast_signed().checked_div(cw.cast_signed()).unwrap_or(0);
                        let k = ((q + 8) & 0xfff8) - q;
                        skipped = skipped.wrapping_add(k.cast_unsigned());
                        x = x.wrapping_add(cw.wrapping_mul(k.cast_unsigned()));
                        None
                    }
                    _ if c <= b'9' => Some(c.wrapping_sub(0x30)),
                    _ => Some((c & 0x5f).wrapping_sub(0x37)),
                };
                if let Some(g) = glyph {
                    let g = u32::from(g);
                    let u: u32 = h.read(h.rebase(GLYPH_U) + 4 * g);
                    let v: u32 = h.read(h.rebase(GLYPH_V) + 4 * g);
                    let ch = u32::from(h.read::<u8>(font + 0x1b));
                    sprite(h, out, u, v, x, y, cw, ch, colour);
                    count += 1;
                    out += 0x40;
                    let cw = u32::from(h.read::<u8>(font + 0x1a));
                    x = x.wrapping_add(cw);
                }
                c = h.read(text + i + 1);
                i += 1;
                if c == 0 {
                    break;
                }
            }
        }
        let used = if count != 0 {
            cdecl(h, TEX_TAG, &[p, count, 0]);
            (count + 1) << 6
        } else {
            0
        };
        let column: u16 = h.read(ctx + 0x10);
        h.write(
            ctx + 0x10,
            column.wrapping_add(skipped.wrapping_add(count) as u16),
        );
        let packet: u32 = h.read(ctx + 8);
        h.write(ctx + 4, text + i);
        h.write(ctx + 0xc, packet.wrapping_add(used));
        count
    }

    /// `opd_num(format, ...)` -> `op_gs1_transfer`'s `eax`, or
    /// `sceGsSyncPath`'s with the display off: the format's text, `%x`,
    /// `%d` and `%f` (hex-digit widths and precisions, default 4) emitted
    /// into the font's packet (the context `0x02322760` at the print
    /// position, row plus column), its end tag set, the packet sent and the
    /// column counted. The arguments are read from the argument slots
    /// (`slots`, the format first).
    pub fn opd_num(h: &mut impl Host, slots: u32) -> u32 {
        let r = cdecl(h, GS_SYNC_PATH, &[0, 0]);
        if DISPLAY.get(h) == 0 {
            return r;
        }
        let ctx = h.rebase(CONTEXT);
        let font = h.rebase(FONT);
        let format: u32 = h.read(slots);
        let packet = FONT_PACKET.get(h);
        h.write(ctx + 0x16, 0_u16);
        let px = PRINT_X.get(h);
        h.write(ctx + 0x10, px);
        let row = PRINT_Y.get(h).wrapping_add(PRINT_COLUMN.get(h));
        let mut end = packet + 0x50;
        h.write(ctx + 4, format);
        h.write(ctx, format);
        h.write(ctx + 0xc, end);
        h.write(ctx + 8, end);
        h.write(ctx + 0x12, row);
        let mut at = format;
        let mut c: u8 = h.read(at);
        let mut arg = slots;
        let mut packet = packet;
        if c != 0 {
            loop {
                if c != b'%' && c != b'$' {
                    h.write(ctx, at);
                    h.write(ctx + 8, end);
                    cdecl(h, PUT_TEXT, &[ctx, font]);
                } else {
                    let mut width = 0_u32;
                    let mut precision = 0_u32;
                    let kind = loop {
                        at += 1;
                        let d: u8 = h.read(at);
                        match d {
                            b'x' | b'd' | b'f' => break d,
                            b'.' => {
                                precision = width;
                                width = 0;
                            }
                            _ if d.wrapping_sub(0x30) <= 9 => {
                                width = width
                                    .wrapping_sub(3)
                                    .wrapping_shl(4)
                                    .wrapping_add(u32::from(d));
                            }
                            _ => {}
                        }
                    };
                    h.write(ctx + 4, at + 1);
                    h.write(ctx, at + 1);
                    let e: u32 = h.read(ctx + 0xc);
                    h.write(ctx + 8, e);
                    if width == 0 {
                        width = 4;
                    }
                    match kind {
                        b'f' => {
                            if precision == 0 {
                                precision = 4;
                            }
                            // `fld qword` / `fstp qword`: a signalling NaN comes
                            // out quiet
                            let (lo, mut hi): (u32, u32) = (h.read(arg + 4), h.read(arg + 8));
                            if hi & 0x7ff0_0000 == 0x7ff0_0000 && (hi & 0x000f_ffff != 0 || lo != 0)
                            {
                                hi |= 0x0008_0000;
                            }
                            arg += 8;
                            cdecl(h, PUT_FLOAT, &[lo, hi, precision, width, ctx, font]);
                        }
                        _ => {
                            arg += 4;
                            let v: u32 = h.read(arg);
                            let put = if kind == b'x' { PUT_HEX } else { PUT_DEC };
                            cdecl(h, put, &[v, width, ctx, font]);
                        }
                    }
                }
                at = h.read(ctx + 4);
                end = h.read(ctx + 0xc);
                c = h.read(at);
                if c == 0 {
                    break;
                }
            }
            packet = FONT_PACKET.get(h);
        }
        cdecl(h, END_TAG, &[packet, end]);
        cdecl(h, GS_SYNC_PATH, &[0, 0]);
        cdecl(h, FLUSH_CACHE, &[0]);
        let (ch, p) = (DMA_CHANNEL.get(h), FONT_PACKET.get(h));
        cdecl(h, DMA_SEND, &[ch, p]);
        let r = cdecl(h, GS1_TRANSFER, &[]);
        let column = PRINT_COLUMN.get(h).wrapping_add(1);
        PRINT_COLUMN.set(h, column);
        r
    }

    /// `opd_debug_flags_str_00a65570(buf, cmd)` -> the last `sprintf`'s
    /// `eax`: the command's flag word (`u16 cmd`): a letter or a dot for
    /// each of bits 12..15 (the letters by bit 0), a separator, then for
    /// each of those bits its operand (the next `u16` after the flags) or a
    /// blank. The second argument slot (`slots + 4`) follows the first two
    /// operands.
    pub fn flags_str(h: &mut impl Host, slots: u32, buf: u32, cmd: u32) -> u32 {
        let sprintf: u32 = h.read(h.rebase(SPRINTF_SLOT));
        let flags = u32::from(h.read::<u16>(cmd));
        let mut p = cmd + 2;
        h.write(slots + 4, p);
        let texts = if flags & 1 != 0 {
            FLAG_TEXTS_SET
        } else {
            FLAG_TEXTS_CLEAR
        };
        let mut at = buf;
        for (k, text) in texts.into_iter().enumerate() {
            let text = if flags & (0x1000 << k) != 0 {
                text
            } else {
                FLAG_DOT
            };
            let text = h.rebase(text);
            at = at.wrapping_add(h.call(sprintf, Conv::Cdecl, &[at, text]));
        }
        let sep = h.rebase(FLAG_SEPARATOR);
        at = at.wrapping_add(h.call(sprintf, Conv::Cdecl, &[at, sep]));
        let mut r = 0;
        for k in 0..4 {
            if flags & (0x1000 << k) != 0 {
                let v = u32::from(h.read::<u16>(p));
                let fmt = h.rebase(OPERAND_FORMAT);
                r = h.call(sprintf, Conv::Cdecl, &[at, fmt, v]);
                // the third operand's step stays in a register: the slot
                // keeps the second's
                if k < 3 {
                    p += 2;
                }
                if k < 2 {
                    h.write(slots + 4, p);
                }
            } else {
                let fmt = h.rebase(NO_OPERAND);
                r = h.call(sprintf, Conv::Cdecl, &[at, fmt]);
            }
            at = at.wrapping_add(r);
        }
        r
    }

    /// `opd_num_load_font_00a5e4a0()` -> `sceDmaSendN`'s `eax`: the five
    /// font images loaded to VRAM (each layout's seven `u16`s to
    /// `sceGsSetDefLoadImage`, path 0 synced, the cache flushed, the pixels
    /// sent), then the upload packet's tag set up (`0x02288180`: NLOOP
    /// 0x8000.., EOP, A+D registers 0xe, two words cleared, count 0), the
    /// channel's bit 6 set and the packet sent once.
    pub fn load_font(h: &mut impl Host) -> u32 {
        let li = h.rebase(LOAD_IMAGE);
        let layouts = h.rebase(IMAGE_LAYOUTS);
        for (i, pixels) in IMAGE_PIXELS.into_iter().enumerate() {
            let at = layouts + 0x10 * i as u32;
            let mut args = [li; 8];
            let mut field = at;
            for a in &mut args[1..] {
                *a = u32::from(h.read::<u16>(field));
                field += 2;
            }
            cdecl(h, SET_DEF_LOAD_IMAGE, &args);
            cdecl(h, GS_SYNC_PATH, &[0, 0]);
            cdecl(h, FLUSH_CACHE, &[0]);
            let pixels = h.rebase(pixels);
            cdecl(h, EXEC_LOAD_IMAGE, &[li, pixels]);
        }
        cdecl(h, FLUSH_CACHE, &[0]);
        let packet = h.rebase(UPLOAD_PACKET);
        let w: u32 = h.read(packet);
        h.write(packet, (w & 0xffff_8000) | 0x8000);
        let w: u32 = h.read(packet + 4);
        h.write(packet + 4, (w & 0x0fff_ffff) | 0x1000_0000);
        let w: u32 = h.read(packet + 8);
        UPLOAD_COUNT.set(h, 0);
        h.write(packet + 8, (w & !1) | 0xe);
        h.write(packet + 0x34, 0_u32);
        h.write(packet + 0x38, 0_u32);
        cdecl(h, FLUSH_CACHE, &[0]);
        let ch = DMA_CHANNEL.get(h);
        let v: u32 = h.read(ch);
        h.write(ch, v | 0x40);
        cdecl(h, GS_SYNC_PATH, &[0, 0]);
        cdecl(h, FLUSH_CACHE, &[0]);
        let n = UPLOAD_COUNT.get(h).wrapping_add(1);
        let ch = DMA_CHANNEL.get(h);
        cdecl(h, DMA_SEND_N, &[ch, packet, n])
    }
}
