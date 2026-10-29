//! M5 stage 1, the menu, battle and script leftovers (round 4), batch 1:
//! the `Gu_` debug GUI's client box (`Gu_DrawClient`), its printer
//! (`Gu_Printf`), the debug save-slot files, the hard-disk wallpaper's
//! sprite packets, the event text loaders, the character viewer's motion
//! window, the focus window and the debug-menu dispatcher, the debug font's
//! string core and the PS2 memory-card read callback.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::manual_clamp,
    clippy::many_single_char_names,
    clippy::too_many_lines
)]

pub mod bridge {
    use ffx_bridge::{Global, Mem};
    use ffx_graphic::host::{Conv, Host};

    use crate::get_ptr32_func;
    use crate::gu::bridge::{H_SCALE, PACKET, PACKET_COUNT, PRINT_COLOR, gu_print_set_metrics};

    fn at(p: u32, o: u32) -> u32 {
        p.wrapping_add(o)
    }

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// A CRT import (cdecl) through its slot.
    fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, Conv::Cdecl, args)
    }

    /// `x / d` as `idiv` computes it; a zero divisor (the original raises
    /// a divide error there) gives 0.
    fn div(x: i32, d: i32) -> i32 {
        if d == 0 { 0 } else { x.wrapping_div(d) }
    }

    /// The C string's length at `p`.
    fn strlen(mem: &impl Mem, p: u32) -> u32 {
        let mut n = 0u32;
        while mem.read::<u8>(at(p, n)) != 0 {
            n = n.wrapping_add(1);
        }
        n
    }

    /// The original's inline `strcat(dst, src)`: `src` with its 0 byte
    /// copied after `dst`'s first 0 byte.
    fn strcat(mem: &mut impl Mem, dst: u32, src: u32) {
        let n = strlen(mem, src).wrapping_add(1);
        let end = at(dst, strlen(mem, dst));
        for i in 0..n {
            let b: u8 = mem.read(at(src, i));
            mem.write(at(end, i), b);
        }
    }

    // ---- Gu_DrawClient ----

    pub const GU_SET_TEST: u32 = 0x0084_85d0;
    /// `g_GuPacketData`, the packet origin (`x`, `y`), the screen size.
    pub const PACKET_DATA: Global<u32> = Global::new(0x0130_36c8);
    pub const PACKET_ORIGIN: Global<[u32; 2]> = Global::new(0x0130_36d4);
    pub const SCREEN_W: Global<i32> = Global::new(0x023c_bbe4);
    pub const SCREEN_H: Global<i32> = Global::new(0x023c_bbe0);

    /// An `XYZ2` quadword: the screen position (16ths of a pixel), 0, 4, 0.
    fn xyz(mem: &mut impl Mem, q: u32, x: i32, y: i32) {
        let sx = 0x1000i32.wrapping_sub(SCREEN_W.get(mem)) / 2;
        let sy = 0x1000i32.wrapping_sub(SCREEN_H.get(mem)) / 2;
        mem.write(q, (x.wrapping_add(sx) << 4) as u16);
        mem.write(at(q, 2), (y.wrapping_add(sy) << 4) as u16);
        mem.write(at(q, 4), [0u32, 4, 0]);
    }

    /// An `RGBAQ` quadword.
    fn rgbaq(mem: &mut impl Mem, q: u32, c: u32) {
        mem.write(q, [c, 0x8000, 1, 0]);
    }

    /// `old + (target - old) / d` on the `i16` at `+o` (target `+o + 8`),
    /// stored; the full `eax` of the sum (its low half is the new value).
    fn ease_word(mem: &mut impl Mem, c: u32, o: u32, d: i32) -> i32 {
        let old: u16 = mem.read(at(c, o));
        let tgt: i16 = mem.read(at(c, o + 8));
        let q = div(i32::from(tgt) - i32::from(old as i16), d);
        let v = q.wrapping_add(i32::from(old));
        mem.write(at(c, o), v as u16);
        v
    }

    /// `Gu_DrawClient(client, x, y)` (`0x00847b50`): the client box's
    /// rectangle (`i16` `+0x08..+0x0e`) eased toward its target (`+0x10..`)
    /// by `1/d` (`d` = `i16 +0x24`, or 4 with a target width, else 3) and
    /// its colour (`+0x1c..+0x1e`) by a third toward `+0x18..+0x1a`; then
    /// a GIF tag (`0x3039`, `0x1005c000`, `0xe`), the packet origin
    /// cleared, `Gu_SetTest(1, 1, 0x7f, 0, ...)`, and 19 quadwords: the
    /// frame's light top line, the face's two bands in the box colour and
    /// the dark bottom lines (`x0 = x + 1 + left`, `x1 = x0 + width - 2`;
    /// rows `y + 1 + top` (+4, +9), scaled by `g_GuHScale`). The tag's
    /// quadword count closes it; returns the masked `eax` of that merge.
    pub fn draw_client<H: Host>(h: &mut H, c: u32, x: u32, y: u32) -> u32 {
        let d = {
            let w24: i16 = h.read(at(c, 0x24));
            if w24 != 0 {
                i32::from(w24)
            } else if h.read::<i16>(at(c, 0x14)) != 0 {
                4
            } else {
                3
            }
        };
        let ease = |h: &mut H, o: u32| ease_word(h, c, o, d);
        let nx = ease(h, 0x08);
        let ny = ease(h, 0x0a);
        let nw = ease(h, 0x0c);
        ease(h, 0x0e);
        for (o, t) in [(0x1d, 0x19), (0x1e, 0x1a), (0x1c, 0x18)] {
            let cur: u8 = h.read(at(c, o));
            let tgt: u8 = h.read(at(c, t));
            let step = (i32::from(tgt) - i32::from(cur)) / 3;
            h.write(at(c, o), cur.wrapping_add(step as u8));
        }
        let x0 = (x as i32)
            .wrapping_add(1)
            .wrapping_add(i32::from(nx as i16));
        let y0 = (y as i32)
            .wrapping_add(1)
            .wrapping_add(i32::from(ny as i16));
        let x1 = i32::from(nw as i16).wrapping_sub(2).wrapping_add(x0);
        let c0 = PACKET_COUNT.get(h);
        let tag = at(PACKET.get(h), c0.wrapping_mul(16));
        h.write(tag, [0x3039u32, 0x1005_c000, 0xe, 0]);
        PACKET_ORIGIN.set(h, [0, 0]);
        PACKET_COUNT.set(h, c0.wrapping_add(1));
        cdecl(h, GU_SET_TEST, &[1, 1, 0x7f, 0, 0, 0, 0, 0]);
        let n = PACKET_COUNT.get(h);
        let data = PACKET_DATA.get(h);
        let scaled = |h: &H, dy: i32| H_SCALE.get(h).wrapping_mul(y0.wrapping_add(dy)) >> 8;
        let ya = scaled(h, 0);
        let yb = scaled(h, 4);
        let yc = scaled(h, 9);
        let b: [u8; 3] = h.read(at(c, 0x1c));
        let face = 0xc000_0000 | u32::from(b[2]) << 16 | u32::from(b[1]) << 8 | u32::from(b[0]);
        let q = |i: u32| at(data, n.wrapping_add(i).wrapping_mul(16));
        rgbaq(h, q(0), 0xc0e0_e0e0);
        xyz(h, q(1), x0, ya);
        xyz(h, q(2), x1, ya);
        rgbaq(h, q(3), face);
        xyz(h, q(4), x0, yb);
        rgbaq(h, q(5), 0xc0e0_e0e0);
        xyz(h, q(6), x1, ya);
        rgbaq(h, q(7), face);
        xyz(h, q(8), x0, yb);
        xyz(h, q(9), x1, yb);
        xyz(h, q(10), x0, yb);
        xyz(h, q(11), x1, yb);
        rgbaq(h, q(12), 0xc030_3030);
        xyz(h, q(13), x0, yc);
        rgbaq(h, q(14), face);
        xyz(h, q(15), x1, yb);
        rgbaq(h, q(16), 0xc030_3030);
        xyz(h, q(17), x0, yc);
        xyz(h, q(18), x1, yc);
        let end = n.wrapping_add(19);
        PACKET_COUNT.set(h, end);
        let old: u32 = h.read(tag);
        let e = (end.wrapping_sub(c0).wrapping_sub(1) ^ old) & 0x7fff;
        h.write(tag, old ^ e);
        e
    }

    // ---- Gu_Printf ----

    pub const GU_PRINT_BEGIN: u32 = 0x0084_9170;
    pub const GU_PARSE_HEX: u32 = 0x0084_8f60;
    pub const GU_PARSE_NUMBER: u32 = 0x0084_8fe0;
    pub const SET_PRINTF_FUNC: u32 = 0x0085_10c0;
    pub const SG_VFPRINTF: u32 = 0x0085_11d0;
    /// The putter `Gu_Printf` hands `sg_vfprintf` (`0x00849070`).
    pub const GU_PUT_CHAR: u32 = 0x0084_9070;
    /// `g_GuPrintItalic`, `g_GuPrintBold`, the print size
    /// (`Gu_PrintSetMetrics`'s word), and the characters printed since the
    /// GIF tag (`0x013036bc`, set by the putter).
    pub const PRINT_ITALIC: Global<u32> = Global::new(0x0130_3718);
    pub const PRINT_BOLD: Global<u32> = Global::new(0x0130_3720);
    pub const PRINT_SIZE: Global<i32> = Global::new(0x0130_3728);
    pub const PRINT_CHARS: Global<u32> = Global::new(0x0130_36bc);

    /// `Gu_Printf(fmt, ...)` (`0x00848880`; `va` = the caller's argument
    /// words after `fmt`, `cell` = the original's `ebp - 4`, the parsers'
    /// out word). A leading `"%~"` starts style codes, each after a `~`:
    /// `b` bold, `i` italic (both leave the text at the code letter), `c`
    /// a hex colour (`Gu_ParseHex_00848f60`: blue `v >> 1`, green `v >> 9`,
    /// red `v >> 17`, 7 bits each; the cell left `v >> 16`), `w` a size
    /// (`Gu_ParseNumber_00848fe0`, the metrics set), `.` skips itself, the
    /// end of the string returns 0 at once (nothing restored); any other
    /// letter is printed. Then `Gu_PrintBegin`, a text GIF tag at
    /// `g_GuPacket` (`| 0x8000`, `0xc089c000` over the top bits of the
    /// second word, `0x43434343`, `0x4343`), `sg_vfprintf(0, text, va)`
    /// through the GUI's putter, the tag's count set to the characters
    /// printed, the colour, style and size put back; returns the size's
    /// `x4 >> 8` metric.
    pub fn gu_printf(h: &mut impl Host, cell: u32, fmt: u32, va: u32) -> u32 {
        let colour = PRINT_COLOR.get(h);
        let bold = PRINT_BOLD.get(h);
        let italic = PRINT_ITALIC.get(h);
        let size = PRINT_SIZE.get(h);
        let mut text = fmt;
        if h.read::<u8>(fmt) == b'%' && h.read::<u8>(at(fmt, 1)) == b'~' {
            let mut p = at(fmt, 1);
            text = p;
            while h.read::<u8>(p) == b'~' {
                p = at(p, 1);
                text = p;
                match h.read::<u8>(p) {
                    0 => return 0,
                    b'.' => {
                        text = at(p, 1);
                        break;
                    }
                    b'b' => PRINT_BOLD.set(h, 1),
                    b'i' => PRINT_ITALIC.set(h, 1),
                    b'c' => {
                        p = at(p, 1);
                        p = cdecl(h, GU_PARSE_HEX, &[p, cell]);
                        let v: i32 = h.read(cell);
                        let mut c = PRINT_COLOR.get(h);
                        c[2] = ((v >> 1) & 0x7f) as u32;
                        c[1] = ((v >> 9) & 0x7f) as u32;
                        h.write(cell, v >> 16);
                        c[0] = ((v >> 17) & 0x7f) as u32;
                        PRINT_COLOR.set(h, c);
                        text = p;
                    }
                    b'w' => {
                        p = at(p, 1);
                        p = cdecl(h, GU_PARSE_NUMBER, &[p, cell]);
                        let v: i32 = h.read(cell);
                        gu_print_set_metrics(h, v);
                        text = p;
                    }
                    _ => break,
                }
            }
        }
        cdecl(h, GU_PRINT_BEGIN, &[]);
        let n = PACKET_COUNT.get(h);
        let tag = at(PACKET.get(h), n.wrapping_mul(16));
        let w1: u32 = h.read(at(tag, 4));
        let w0: u32 = h.read(tag);
        h.write(tag, w0 | 0x8000);
        h.write(at(tag, 4), (w1 & 0x3fff) | 0xc089_c000);
        h.write(at(tag, 8), [0x4343_4343u32, 0x4343]);
        PACKET_COUNT.set(h, n.wrapping_add(1));
        PRINT_CHARS.set(h, 0);
        let put = h.rebase(GU_PUT_CHAR);
        let old = cdecl(h, SET_PRINTF_FUNC, &[put]);
        cdecl(h, SG_VFPRINTF, &[0, text, va]);
        let w0: u32 = h.read(tag);
        let chars = PRINT_CHARS.get(h);
        h.write(tag, w0 ^ ((w0 ^ chars) & 0x7fff));
        cdecl(h, SET_PRINTF_FUNC, &[old]);
        PRINT_COLOR.set(h, colour);
        PRINT_BOLD.set(h, bold);
        PRINT_ITALIC.set(h, italic);
        gu_print_set_metrics(h, size) as u32
    }

    // ---- the debug save-slot files ----

    pub const CRT_MEMSET: u32 = 0x0094_964c;
    pub const DATA_DIRECTORY: u32 = 0x0067_9a10;
    pub const CROSS_ASSERT_OUTPUT: u32 = 0x006e_f310;
    pub const SAVE_DATA_CHECK_CRC: u32 = 0x0064_7d70;
    pub const TK_CRC: u32 = 0x008b_1450;
    /// `MSVCR110` `sprintf`, `fopen`, `fread`, `fwrite`, `fclose` slots.
    pub const SPRINTF_IAT: u32 = 0x00b0_c3dc;
    pub const FOPEN_IAT: u32 = 0x00b0_c3f0;
    pub const FREAD_IAT: u32 = 0x00b0_c3f4;
    pub const FWRITE_IAT: u32 = 0x00b0_c428;
    pub const FCLOSE_IAT: u32 = 0x00b0_c3ec;
    /// `"/FFX_Data/GameData/PS3Data/saves/"`, `"%02d"`, `"%d"`, `".SAV"`,
    /// `"rb"`, `"wb"` and the two failure messages.
    pub const SAVES_KEY: u32 = 0x00b4_ba30;
    pub const FMT_02D: u32 = 0x00b4_1ec0;
    pub const FMT_D: u32 = 0x00b0_f604;
    pub const EXT_SAV: u32 = 0x00b4_bab0;
    pub const MODE_RB: u32 = 0x00b3_f69c;
    pub const MODE_WB: u32 = 0x00b4_1ec8;
    pub const LOAD_FAILED: u32 = 0x00b4_1e94;
    pub const SAVE_FAILED: u32 = 0x00b4_1ecc;

    /// Which debug save-slot function.
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum SlotFile {
        /// `debugLoad_00646b30`: `<saves>NN.SAV`, read.
        Load,
        /// `debugLoad_00646ca0`: `<saves>N`, read.
        LoadBare,
        /// `debugSave`: `<saves>NN.SAV`, written.
        Save,
    }

    /// The three debug save-slot functions (`slot, buf, size`; frame at
    /// `ebp`): the path built at `ebp - 0x104` (cleared with `memset`,
    /// then `FmodFile_getDataDirectory(saves, ebp - 0x204, 0x100)`, the
    /// slot number `sprintf`ed at `ebp - 0x408` and, but for
    /// [`SlotFile::LoadBare`], `".SAV"` appended inline), opened with
    /// `fopen`. A failure is reported (`sprintf` at `ebp - 0x404`,
    /// `crossAssertOutput`) and returns -1. A load keeps `buf` at
    /// `ebp - 0x40c`, `fread`s `size` bytes, closes, and runs
    /// `SaveDataCheckCrc`; a save stores `TkCRC(0x64b8, buf + 0x40)` at
    /// `u16 +0x1a` and `u32 +0x64f4` (cleared first) and `fwrite`s. Both
    /// return 0.
    pub fn slot_file(
        h: &mut impl Host,
        ebp: u32,
        kind: SlotFile,
        slot: u32,
        buf: u32,
        size: u32,
    ) -> u32 {
        let path = ebp.wrapping_sub(0x104);
        let dir = ebp.wrapping_sub(0x204);
        let num = ebp.wrapping_sub(0x408);
        if kind != SlotFile::Save {
            h.write(ebp.wrapping_sub(0x40c), buf);
        }
        cdecl(h, CRT_MEMSET, &[path, 0, 0x100]);
        let fmt = h.rebase(if kind == SlotFile::LoadBare {
            FMT_D
        } else {
            FMT_02D
        });
        import(h, SPRINTF_IAT, &[num, fmt, slot]);
        let key = h.rebase(SAVES_KEY);
        cdecl(h, DATA_DIRECTORY, &[key, dir, 0x100]);
        strcat(h, path, dir);
        strcat(h, path, num);
        if kind != SlotFile::LoadBare {
            let ext = h.rebase(EXT_SAV);
            strcat(h, path, ext);
        }
        let mode = h.rebase(if kind == SlotFile::Save {
            MODE_WB
        } else {
            MODE_RB
        });
        let f = import(h, FOPEN_IAT, &[path, mode]);
        if f == 0 {
            let msg = ebp.wrapping_sub(0x404);
            let fail = h.rebase(if kind == SlotFile::Save {
                SAVE_FAILED
            } else {
                LOAD_FAILED
            });
            import(h, SPRINTF_IAT, &[msg, fail, path]);
            cdecl(h, CROSS_ASSERT_OUTPUT, &[msg]);
            return 0xffff_ffff;
        }
        if kind == SlotFile::Save {
            h.write(at(buf, 0x64f4), 0u32);
            let crc = cdecl(h, TK_CRC, &[0x64b8, at(buf, 0x40)]) & 0xffff;
            h.write(at(buf, 0x1a), crc as u16);
            h.write(at(buf, 0x64f4), crc);
            import(h, FWRITE_IAT, &[buf, 1, size, f]);
            import(h, FCLOSE_IAT, &[f]);
        } else {
            import(h, FREAD_IAT, &[buf, 1, size, f]);
            import(h, FCLOSE_IAT, &[f]);
            cdecl(h, SAVE_DATA_CHECK_CRC, &[]);
        }
        0
    }

    // ---- the hard-disk wallpaper's packets ----

    /// `GsPkt_TexturedSprite_00774ff0(pkt, tbp, psm, x, y, w, h)`: a
    /// 16-quadword DMA packet of a textured sprite: the `TEX0` word built
    /// from `tbp`, `w / 64` (signed), the texture's log sizes (1 up to 16,
    /// then 5..10 for each doubling past 16, 32, ... 512) and `psm`, the
    /// fixed `A+D` settings, the sprite from `(x + 0x700, y + 0x730)`
    /// sized `w x h` (UVs at half-texel offsets), and the end tag.
    /// Returns 0.
    #[allow(clippy::too_many_arguments)]
    pub fn textured_sprite(
        mem: &mut impl Mem,
        p: u32,
        tbp: u32,
        psm: u32,
        x: u32,
        y: u32,
        w: u32,
        hh: u32,
    ) -> u32 {
        let log = |v: i32| -> i64 {
            let mut l = 1;
            for (lim, n) in [
                (0x10, 5),
                (0x20, 6),
                (0x40, 7),
                (0x80, 8),
                (0x100, 9),
                (0x200, 10),
            ] {
                if v > lim {
                    l = n;
                }
            }
            l
        };
        let (wi, hi) = (w as i32, hh as i32);
        let tw = log(wi);
        let th = log(hi);
        let sx = |v: u32| i64::from(v as i32);
        let mut v = sx(psm);
        v = (v << 7) | th;
        v = (v << 4) | tw;
        v = (v << 12) | i64::from(wi / 64);
        v |= 0x8000_0010_04c0;
        v = (v << 14) | sx(tbp);
        let tex0 = v as u64;
        let x0 = x.wrapping_add(0x700);
        let y0 = y.wrapping_add(0x730);
        let words: [u32; 64] = [
            0x1000_000e,
            0,
            0,
            0,
            6,
            0x1000_0000,
            0xe,
            0,
            0,
            0,
            0x3f,
            0,
            tex0 as u32,
            (tex0 >> 32) as u32,
            6,
            0,
            0x44,
            0,
            0x42,
            0,
            0x5001d,
            0,
            0x47,
            0,
            0,
            0,
            0x49,
            0,
            0,
            0,
            0x14,
            0,
            0x8001,
            0x60ab_4000,
            0x0041_3413,
            0,
            8,
            8,
            0x3f80_0000,
            0,
            0x80,
            0x80,
            0x80,
            0x80,
            x0 << 4,
            y0 << 4,
            0x00ff_ffff,
            0,
            (w << 4).wrapping_add(8),
            (hh << 4).wrapping_add(8),
            0x3f80_0000,
            0,
            0x80,
            0x80,
            0x80,
            0x80,
            x0.wrapping_add(w) << 4,
            y0.wrapping_add(hh) << 4,
            0x00ff_ffff,
            0,
            0x7000_0000,
            0,
            0,
            0,
        ];
        mem.write(p, words);
        0
    }

    /// `HddKabe_FramePacket_00775330(pkt, x, y, w, h)` (name ours): a
    /// 16-quadword DMA packet of an untextured sprite, colour
    /// `(0x20, 0x20, 0x20, 0x40)`, from `(x + 0x6fc, y + 0x72c)` to
    /// `(x + w + 0x704, y + h + 0x736)`, with the fixed `A+D` settings and
    /// the end tag. Returns 0.
    pub fn frame_packet(mem: &mut impl Mem, p: u32, x: u32, y: u32, w: u32, hh: u32) -> u32 {
        let x0 = x.wrapping_add(0x6fc) << 4;
        let y0 = y.wrapping_add(0x72c) << 4;
        let x1 = x.wrapping_add(w).wrapping_add(0x704) << 4;
        let y1 = hh.wrapping_add(y).wrapping_add(0x736) << 4;
        let colour = [0x20u32, 0x20, 0x20, 0x40];
        let mut words = [0u32; 64];
        words[..28].copy_from_slice(&[
            0x1000_000e,
            0,
            0,
            0,
            4,
            0x1000_0000,
            0xe,
            0,
            0x44,
            0,
            0x42,
            0,
            0x5001d,
            0,
            0x47,
            0,
            0,
            0,
            0x49,
            0,
            0,
            0,
            0x14,
            0,
            0x8001,
            0x8022_4000,
            0x4141_4141,
            0,
        ]);
        for (i, (vx, vy)) in [(x0, y0), (x1, y0), (x0, y1), (x1, y1)]
            .into_iter()
            .enumerate()
        {
            let q = 28 + 8 * i;
            words[q..q + 4].copy_from_slice(&colour);
            words[q + 4..q + 8].copy_from_slice(&[vx, vy, 0x00ff_ffff, 0]);
        }
        words[60] = 0x7000_0000;
        mem.write(p, words);
        0
    }

    pub const SCE_GS_SET_DEF_LOAD_IMAGE: u32 = 0x0062_f820;
    pub const FLUSH_CACHE: u32 = 0x0062_f4c0;
    pub const SCE_GS_EXEC_LOAD_IMAGE: u32 = 0x0062_f7a0;
    pub const SCE_GS_SYNC_PATH: u32 = 0x0062_f880;
    /// The wallpaper's texture base (`0x00c416cc`): its palette at `+0`,
    /// its picture at `+4`.
    pub const KABE_TBP: Global<u32> = Global::new(0x00c4_16cc);

    /// `HddKabe_LoadImage_00775720(img, out_w, out_h)` (name ours; the
    /// load-image descriptor at the original's aligned `ebp - 0x60`):
    /// `*out_w`, `*out_h` = the image's first two words; its 16x16 palette
    /// (`img + 0x10`) loaded at the base, then its picture (`img +
    /// 0x410`, `w x h`, PSMT8 `0x13`, width `w / 64`) at the base + 4,
    /// each `sceGsSetDefLoadImage`, `FlushCache(0)`,
    /// `sceGsExecLoadImage`, `sceGsSyncPath(0, 0)`. Returns the last
    /// `sceGsSyncPath`'s `eax`.
    pub fn kabe_load_image(h: &mut impl Host, desc: u32, img: u32, out_w: u32, out_h: u32) -> u32 {
        let base = KABE_TBP.get(h);
        let w: u32 = h.read(img);
        h.write(out_w, w);
        let hh: u32 = h.read(at(img, 4));
        h.write(out_h, hh);
        cdecl(
            h,
            SCE_GS_SET_DEF_LOAD_IMAGE,
            &[desc, base, 1, 0, 0, 0, 0x10, 0x10],
        );
        cdecl(h, FLUSH_CACHE, &[0]);
        cdecl(h, SCE_GS_EXEC_LOAD_IMAGE, &[desc, at(img, 0x10)]);
        cdecl(h, SCE_GS_SYNC_PATH, &[0, 0]);
        let w: i32 = h.read(img);
        let w16 = u32::from(h.read::<u16>(img));
        let h16 = u32::from(h.read::<u16>(at(img, 4)));
        let args = [desc, at(base, 4), (w / 64) as u32, 0x13, 0, 0, w16, h16];
        cdecl(h, SCE_GS_SET_DEF_LOAD_IMAGE, &args);
        cdecl(h, FLUSH_CACHE, &[0]);
        cdecl(h, SCE_GS_EXEC_LOAD_IMAGE, &[desc, at(img, 0x410)]);
        cdecl(h, SCE_GS_SYNC_PATH, &[0, 0])
    }

    // ---- the event text loaders ----

    pub const TO_GET_FFX_LANG: u32 = 0x0064_10e0;
    pub const FIOS_UNIFY_FILENAME: u32 = 0x0067_9820;
    pub const FIOS_IS_FILE_EXIST: u32 = 0x0067_96f0;
    pub const FIOS_OPEN: u32 = 0x0067_9730;
    pub const FIOS_FILE_SIZE: u32 = 0x0067_9570;
    pub const FIOS_READ: u32 = 0x0067_9800;
    pub const FIOS_CLOSE: u32 = 0x0067_9510;
    pub const SG_MEM_SET_ALIGN: u32 = 0x0083_c4a0;
    pub const USER_MALLOC: u32 = 0x0068_7020;
    pub const DBG_PRINTF: u32 = 0x0062_f500;
    /// `strstr`, `sprintf_s` import slots (cdecl).
    pub const STRSTR_IAT: u32 = 0x00b0_c378;
    pub const SPRINTF_S_IAT: u32 = 0x00b0_c394;
    /// `"/ffx_ps2/ffx/master/new_"`, `"%s"`, `"/"`.
    pub const MASTER_NEW: u32 = 0x00b5_ea8c;
    pub const FMT_S: u32 = 0x00b0_e160;
    pub const SLASH: u32 = 0x00b8_f88c;

    /// The `"%s<lang>pc/"` format of a `TOGetFFXLang` value (the
    /// originals' jump table): Japanese 0, French 2, Spanish 3, German 4,
    /// Italian 5, Korean 9, Chinese 10, US otherwise.
    fn lang_dir(lang: u32) -> u32 {
        match lang {
            0 => 0x00b5_eaa8,
            2 => 0x00b5_eab8,
            3 => 0x00b5_eac0,
            4 => 0x00b5_eac8,
            5 => 0x00b5_ead0,
            9 => 0x00b5_ead8,
            10 => 0x00b5_eae0,
            _ => 0x00b5_eab0,
        }
    }

    /// What a loader's name rule passes `sprintf` after the language dir.
    #[derive(Clone, Copy)]
    enum Tail {
        /// Nothing more.
        None,
        /// The name.
        Name,
        /// The name twice.
        NameName,
    }

    /// Which loader.
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum EventFile {
        /// `TKMallocAndReadEventFTC` (`0x0088c860`).
        Ftc,
        /// `TKMallocAndReadEventMessage` (`0x0088cac0`).
        Message,
    }

    /// `(needle, format, tail)` rules in order, the fallback format (dir,
    /// name, `strstr(name, "/")`), the missing-file message.
    fn rules(kind: EventFile) -> (&'static [(u32, u32, Tail)], u32, u32) {
        match kind {
            EventFile::Ftc => (
                &[
                    (0x00b5_5e74, 0x00b5_eb54, Tail::NameName),
                    (0x00b5_eb6c, 0x00b5_eb74, Tail::None),
                    (0x00b5_eb84, 0x00b5_eb8c, Tail::None),
                    (0x00b4_04a0, 0x00b5_eba0, Tail::None),
                ],
                0x00b5_ebb0,
                0x00b5_ebcc,
            ),
            EventFile::Message => (
                &[
                    (0x00b5_eae8, 0x00b5_eaf0, Tail::Name),
                    (0x00b5_5e74, 0x00b5_eb00, Tail::NameName),
                    (0x00b5_eb18, 0x00b5_eb20, Tail::Name),
                ],
                0x00b5_eb38,
                0x00b4_1e3c,
            ),
        }
    }

    /// `TKMallocAndReadEventFTC(name)` / `TKMallocAndReadEventMessage(name)`
    /// (frame at `ebp`: the language dir `ebp - 0x44`, the path `ebp -
    /// 0xc4`, the unified name `ebp - 0x1c4`): the path by the first rule
    /// whose needle `strstr` finds in `name` (FTC: `"_"` a battle's,
    /// `"base"`, `"newkit"`, `"help"`; message: `"macro"`, `"_"`,
    /// `"monster"`), else the event's (`name` and its `"/"` part), unified
    /// (`fiosUnifyFilename`, then `sprintf_s(path, 0x7f, "%s", unified)`).
    /// A missing file prints the message and returns 0; else the file is
    /// read whole into `user_malloc` memory (size rounded up to 16, the
    /// block to 2 KB, signed, `Sg_MemSetAlign(0x40)` first; the first
    /// word cleared), which it returns.
    pub fn event_file(h: &mut impl Host, ebp: u32, kind: EventFile, name: u32) -> u32 {
        let dir = ebp.wrapping_sub(0x44);
        let path = ebp.wrapping_sub(0xc4);
        let unified = ebp.wrapping_sub(0x1c4);
        let lang = cdecl(h, TO_GET_FFX_LANG, &[]);
        let fmt = h.rebase(lang_dir(lang));
        let master = h.rebase(MASTER_NEW);
        import(h, SPRINTF_IAT, &[dir, fmt, master]);
        let (list, fallback, missing) = rules(kind);
        let mut done = false;
        for &(needle, fmt, tail) in list {
            let needle = h.rebase(needle);
            if import(h, STRSTR_IAT, &[name, needle]) != 0 {
                let fmt = h.rebase(fmt);
                match tail {
                    Tail::None => import(h, SPRINTF_IAT, &[path, fmt, dir]),
                    Tail::Name => import(h, SPRINTF_IAT, &[path, fmt, dir, name]),
                    Tail::NameName => import(h, SPRINTF_IAT, &[path, fmt, dir, name, name]),
                };
                done = true;
                break;
            }
        }
        if !done {
            let slash = h.rebase(SLASH);
            let part = import(h, STRSTR_IAT, &[name, slash]);
            let fmt = h.rebase(fallback);
            import(h, SPRINTF_IAT, &[path, fmt, dir, name, part]);
        }
        cdecl(h, FIOS_UNIFY_FILENAME, &[path, unified, 0x100]);
        let s = h.rebase(FMT_S);
        import(h, SPRINTF_S_IAT, &[path, 0x7f, s, unified]);
        if cdecl(h, FIOS_IS_FILE_EXIST, &[path]) == 0 {
            let msg = h.rebase(missing);
            cdecl(h, DBG_PRINTF, &[msg, path]);
            return 0;
        }
        let f = cdecl(h, FIOS_OPEN, &[path, 1]);
        let size = cdecl(h, FIOS_FILE_SIZE, &[f]).wrapping_add(0xf) & 0xffff_fff0;
        cdecl(h, SG_MEM_SET_ALIGN, &[0x40]);
        let t = size.wrapping_add(0x7ff) as i32;
        let block = (t.wrapping_add((t >> 31) & 0x7ff) as u32) & 0xffff_f800;
        let data = cdecl(h, USER_MALLOC, &[block]);
        h.write(data, 0u32);
        cdecl(h, FIOS_READ, &[f, data, size]);
        cdecl(h, FIOS_CLOSE, &[f]);
        data
    }

    // ---- the character viewer's motion window ----

    pub const GU_WINDOW_GET_ARG: u32 = 0x0084_aec0;
    pub const GU_MOUSE_SET_CAPTURE: u32 = 0x0084_9b60;
    pub const GU_WINDOW_ADD_BUTTON: u32 = 0x0084_a470;
    pub const GU_WINDOW_ADD_CLIENT: u32 = 0x0084_a4a0;
    pub const GU_WINDOW_ADD_STATIC: u32 = 0x0084_a4d0;
    pub const GU_WINDOW_SET_TITLE: u32 = 0x0084_b6a0;
    pub const GU_WINDOW_GET_WIDTH: u32 = 0x0084_afa0;
    pub const GU_MOUSE_GET_WINDOW_Y: u32 = 0x0084_9aa0;
    pub const GU_WINDOW_CONTROL_SET_RECT: u32 = 0x0084_96d0;
    pub const GU_WINDOW_CONTROL_SET_TEXT: u32 = 0x0084_a710;
    pub const GU_PACKET_BEGIN_TAG: u32 = 0x0084_8530;
    pub const GU_PACKET_END_TAG: u32 = 0x0084_8460;
    pub const GU_SET_PRIM: u32 = 0x0084_8480;
    pub const GU_SET_RGBAQ: u32 = 0x0084_84c0;
    pub const GU_SET_XYZF2: u32 = 0x0084_8680;
    pub const GU_MOUSE_IN_CLIENT_RECT: u32 = 0x0084_9b00;
    /// The motion window's button labels (8, `.rdata`) and its title
    /// format `"[%s] Frame:%d.%02x(%x)/%d"`.
    pub const MOTION_BUTTONS: u32 = 0x00b5_ab10;
    pub const MOTION_TITLE: u32 = 0x00b5_abb0;

    fn i32_at(mem: &impl Mem, p: u32, o: u32) -> i32 {
        mem.read(at(p, o))
    }

    /// `ChrViewMotionCtlProc(window, msg, id)` (frame at `ebp`: the title
    /// at `ebp - 0x44`). The window's argument (`Gu_WindowGetArg`) holds
    /// the viewed character at `+4` (through `get_ptr32_func`), whose
    /// `+0x740` is the frame (8.8), `+0x754` the speed (`i16`), `+0x800`
    /// the motion's frame count (`i16` behind a pointer).
    /// - `0x8001` (create): buttons 2..9 (labels 20 bytes apart, x from 8
    ///   by 0x16) and client 1.
    /// - `0x8005` (button): 1 grabs the bar (the frame from the argument's
    ///   `+0x48`, `Gu_MouseSetCapture(window, 1)`); 2 rewinds (frame and
    ///   speed 0); 3 reverse (`0xff00`, frame's low byte 0); 5 / 7 speed
    ///   -2 / +2; 6 stop; 9 play (`0x100`, low byte 0).
    /// - `0x8006` (repeat): 4 / 8 step a whole frame back / on (fraction
    ///   dropped) and stop.
    /// - `0x8011` (paint), with a motion: the title, the scroll bar's
    ///   rectangle in the argument (`+0x38..+0x44` = 5, 12, width - 15,
    ///   13), the grab position from the mouse (`+0x48`, clamped to
    ///   `0..=span - 0x200`), then the bar, the frame and the grab boxes
    ///   (the grab white under the mouse), and a tick per frame.
    ///
    /// Returns 0.
    pub fn chr_view_motion(h: &mut impl Host, ebp: u32, window: u32, msg: u32, id: u32) -> u32 {
        let arg = cdecl(h, GU_WINDOW_GET_ARG, &[window]);
        let chr = |h: &mut _| get_ptr32_func(Mem::read::<u32>(h, at(arg, 4)));
        let mut c = 0;
        if msg != 0x8001 {
            c = chr(h);
        }
        match msg {
            0x8001 => {
                for i in 0..8u32 {
                    let label = h.rebase(MOTION_BUTTONS + 0x14 * i);
                    let args = [window, i + 2, label, 8 + 0x16 * i, 0, 0x14, 9];
                    cdecl(h, GU_WINDOW_ADD_BUTTON, &args);
                }
                cdecl(h, GU_WINDOW_ADD_CLIENT, &[window, 1, 5, 0xc, 0, 0x19]);
            }
            0x8005 => match id {
                1 => {
                    let c = chr(h);
                    let v: u32 = h.read(at(arg, 0x48));
                    h.write(at(c, 0x740), v);
                    cdecl(h, GU_MOUSE_SET_CAPTURE, &[window, id]);
                }
                2 => {
                    h.write(at(c, 0x740), 0u32);
                    h.write(at(c, 0x754), 0u16);
                }
                3 | 9 => {
                    h.write(at(c, 0x754), if id == 3 { 0xff00u16 } else { 0x100 });
                    h.write(at(c, 0x740), 0u8);
                }
                5 | 7 => {
                    let s: u16 = h.read(at(c, 0x754));
                    let d = if id == 5 { 0xfffe } else { 2 };
                    h.write(at(c, 0x754), s.wrapping_add(d));
                }
                6 => h.write(at(c, 0x754), 0u16),
                _ => {}
            },
            0x8006 if id == 4 || id == 8 => {
                let f: u32 = h.read(at(c, 0x740));
                let f = f & 0xffff_ff00;
                let f = if id == 4 {
                    f.wrapping_sub(0x100)
                } else {
                    f.wrapping_add(0x100)
                };
                h.write(at(c, 0x740), f);
                h.write(at(c, 0x754), 0u16);
            }
            0x8011 if h.read::<u32>(at(c, 0x800)) != 0 => {
                motion_paint(h, ebp, window, arg);
            }
            _ => {}
        }
        0
    }

    /// The `0x8011` paint of [`chr_view_motion`].
    fn motion_paint(h: &mut impl Host, ebp: u32, window: u32, arg: u32) {
        let chr = |h: &mut _| get_ptr32_func(Mem::read::<u32>(h, at(arg, 4)));
        let c = chr(h);
        let count_p: u32 = h.read(at(c, 0x800));
        let count = i32::from(h.read::<i16>(count_p));
        let c = chr(h);
        let speed = (i32::from(h.read::<i16>(at(c, 0x754))) & 0xfff) as u32;
        let c = chr(h);
        let frac = u32::from(h.read::<u8>(at(c, 0x740)));
        let c = chr(h);
        let frame = i32_at(h, c, 0x740) >> 8;
        let c = chr(h);
        let name: u32 = h.read(at(c, 4));
        let title = ebp.wrapping_sub(0x44);
        let fmt = h.rebase(MOTION_TITLE);
        let args = [title, fmt, name, frame as u32, frac, speed, count as u32];
        import(h, SPRINTF_IAT, &args);
        cdecl(h, GU_WINDOW_SET_TITLE, &[window, title]);
        let c = chr(h);
        let count_p: u32 = h.read(at(c, 0x800));
        let span = (i32::from(h.read::<i16>(count_p)) + 1) << 8;
        let width = (cdecl(h, GU_WINDOW_GET_WIDTH, &[window]) as i32).wrapping_sub(10);
        h.write(at(arg, 0x38), 5u32);
        h.write(at(arg, 0x40), width.wrapping_sub(5));
        let left = i32_at(h, arg, 0x38);
        h.write(at(arg, 0x3c), 0xcu32);
        h.write(at(arg, 0x44), 0xdu32);
        let my = cdecl(h, GU_MOUSE_GET_WINDOW_Y, &[window]) as i32;
        let grab = div(
            my.wrapping_sub(left).wrapping_mul(span),
            i32_at(h, arg, 0x40),
        );
        h.write(at(arg, 0x48), grab);
        let lim = span.wrapping_sub(0x200);
        if grab > lim {
            h.write(at(arg, 0x48), lim);
        }
        if i32_at(h, arg, 0x48) < 0 {
            h.write(at(arg, 0x48), 0u32);
        }
        let r: [u32; 3] = h.read(at(arg, 0x3c));
        let args = [window, 1, left as u32, r[0], r[1], r[2]];
        cdecl(h, GU_WINDOW_CONTROL_SET_RECT, &args);
        cdecl(h, GU_PACKET_BEGIN_TAG, &[window]);
        cdecl(h, GU_SET_TEST, &[1, 5, 0x7f, 0, 0, 0, 0, 1]);
        cdecl(h, GU_SET_PRIM, &[6]);
        let xy = |h: &mut _, x: i32, y: i32| {
            cdecl(h, GU_SET_XYZF2, &[x as u32, y as u32]);
        };
        let top = |h: &_| i32_at(h, arg, 0x3c);
        let bottom = |h: &_| i32_at(h, arg, 0x44).wrapping_add(i32_at(h, arg, 0x3c));
        let x_of = |v: i32| div(v.wrapping_mul(width), span).wrapping_add(5);
        cdecl(h, GU_SET_RGBAQ, &[0x80, 0x80, 0xc0, 0xff]);
        let (x, y) = (i32_at(h, arg, 0x38), top(h));
        xy(h, x, y);
        let y = bottom(h);
        xy(h, x_of(span.wrapping_sub(0x100)), y);
        let c = chr(h);
        let f = i32_at(h, c, 0x740);
        cdecl(h, GU_SET_RGBAQ, &[0xff, 0x80, 0x80, 0xff]);
        let y = top(h);
        xy(h, x_of(f), y);
        let y = bottom(h);
        xy(h, x_of(f.wrapping_add(0x100)), y);
        let g = i32_at(h, arg, 0x48);
        let r: [u32; 4] = h.read(at(arg, 0x38));
        let over = cdecl(
            h,
            GU_MOUSE_IN_CLIENT_RECT,
            &[window, r[0], r[1], r[2], r[3]],
        );
        let green = if over == 0 { 0xff } else { 0x80 };
        cdecl(h, GU_SET_RGBAQ, &[0xff, green, 0xff, 0xff]);
        let y = top(h);
        xy(h, x_of(g), y);
        let mut last = x_of(g.wrapping_add(0x100));
        let y = bottom(h);
        xy(h, last, y);
        cdecl(h, GU_SET_PRIM, &[1]);
        cdecl(h, GU_SET_RGBAQ, &[0, 0, 0, 0xff]);
        if span > 0 {
            let n = ((span - 1) as u32 >> 8) + 1;
            let mut acc = 0i32;
            for _ in 0..n {
                last = div(acc, span).wrapping_add(5);
                let y = top(h);
                xy(h, last, y);
                let y = bottom(h);
                xy(h, last, y);
                acc = acc.wrapping_add(width << 8);
            }
        }
        let y = top(h);
        xy(h, 5, y);
        xy(h, last, y);
        let y = bottom(h);
        xy(h, 5, y);
        let y = bottom(h);
        xy(h, last, y);
        cdecl(h, GU_PACKET_END_TAG, &[]);
    }

    // ---- the debug menu's dispatcher and its focus window ----

    pub const GUI_EVE_GWM_BUTTONUP: u32 = 0x0085_4e80;
    pub const ATEL_DEBUG_SET_0086FEC0: u32 = 0x0086_fec0;
    pub const SG_TOGGLE_ENABLE_TK_ANTI: u32 = 0x0083_b100;
    pub const GU_WINDOW_CREATE: u32 = 0x0084_a920;
    pub const ATEL_INIT_SAVE_WORK: u32 = 0x0086_d4e0;
    pub const SG_GET_KEEP_FPS: u32 = 0x0082_05a0;
    pub const SG_SET_KEEP_FPS: u32 = 0x0082_1ae0;
    pub const SG_GUI_GET_DRAW_CHR: u32 = 0x0082_0660;
    pub const SG_GUI_SET_DRAW_CHR: u32 = 0x0082_0670;
    pub const SG_GUI_GET_TEXTURE_ANIM: u32 = 0x0083_1930;
    pub const SG_GUI_SET_TEXTURE_ANIM: u32 = 0x0083_1940;
    pub const GU_BUTTON_SET_TOGGLE_VALUE: u32 = 0x0084_93e0;
    /// The word `0x0086fec0` is handed (`0x01325928`).
    pub const DEBUG_ACTOR: Global<u32> = Global::new(0x0132_5928);
    /// The window procedures the menu opens.
    pub const FOCUS_PROC: u32 = 0x0085_4980;
    pub const PROC_00854420: u32 = 0x0085_4420;
    pub const PROC_00854270: u32 = 0x0085_4270;
    pub const PROC_00854590: u32 = 0x0085_4590;

    /// `Gu_DebugMenuCommand_00854c60(window, id; slots)` (name ours): the
    /// debug menu's button handler. `0x2c` releases the button
    /// (`gui_eve_GWM_BUTTONUP`), `0x2e` calls `0x0086fec0(word, 0)`, `0x2f`
    /// tail-jumps to `Sg_ToggleEnableTkAnti`, `0x32`/`0x37`/`0x3b`/`0x3c`
    /// open windows, `0x51` tail-jumps to `AtelInitSaveWork`, `0x55` cycles
    /// the kept frame rate 0..2, `0x6a`/`0x6b` flip the character drawing /
    /// texture animation and set the button's toggle (`0x6b` tail-jumps
    /// to `Gu_ButtonSetToggleValue` through its own argument slots: the
    /// caller's words after `window` become `0x6b` and the value). Returns
    /// the callee's `eax`; another id leaves `id - 0x2c`, or 0xb (the
    /// jump table's index) inside the table's range.
    fn create(h: &mut impl Host, x: u32, y: u32, w: u32, hh: u32, proc_: u32) -> u32 {
        let p = h.rebase(proc_);
        cdecl(h, GU_WINDOW_CREATE, &[x, y, w, hh, 6, p])
    }

    pub fn debug_menu_command(h: &mut impl Host, window: u32, id: u32, slots: u32) -> u32 {
        let k = id.wrapping_sub(0x2c);
        if k > 0x3f {
            return k;
        }
        match id {
            0x2c => cdecl(h, GUI_EVE_GWM_BUTTONUP, &[window]),
            0x2e => {
                let v = DEBUG_ACTOR.get(h);
                cdecl(h, ATEL_DEBUG_SET_0086FEC0, &[v, 0])
            }
            0x2f => cdecl(h, SG_TOGGLE_ENABLE_TK_ANTI, &[window, id]),
            0x32 => create(h, 0x50, 0x50, 0x6e, 0x2e, FOCUS_PROC),
            0x37 => create(h, 0x64, 0x28, 0x5a, 0x1e, PROC_00854420),
            0x3b => create(h, 0x64, 0x28, 0x5a, 0x2d, PROC_00854270),
            0x3c => create(h, 0x64, 0x28, 0x64, 0x46, PROC_00854590),
            0x51 => cdecl(h, ATEL_INIT_SAVE_WORK, &[window, id]),
            0x55 => {
                let v = cdecl(h, SG_GET_KEEP_FPS, &[]).wrapping_add(1);
                let keep = if (v as i32) > 2 { 0 } else { v };
                cdecl(h, SG_SET_KEEP_FPS, &[keep])
            }
            0x6a => {
                let v = cdecl(h, SG_GUI_GET_DRAW_CHR, &[]);
                cdecl(h, SG_GUI_SET_DRAW_CHR, &[1u32.wrapping_sub(v)]);
                let v = cdecl(h, SG_GUI_GET_DRAW_CHR, &[]);
                let args = [window, 0x6a, 1u32.wrapping_sub(v)];
                cdecl(h, GU_BUTTON_SET_TOGGLE_VALUE, &args)
            }
            0x6b => {
                let v = cdecl(h, SG_GUI_GET_TEXTURE_ANIM, &[]);
                cdecl(h, SG_GUI_SET_TEXTURE_ANIM, &[1u32.wrapping_sub(v)]);
                let v = cdecl(h, SG_GUI_GET_TEXTURE_ANIM, &[]);
                h.write(at(slots, 8), v);
                h.write(at(slots, 4), 0x6bu32);
                cdecl(h, GU_BUTTON_SET_TOGGLE_VALUE, &[window, 0x6b, v])
            }
            _ => 0xb,
        }
    }

    pub const SG_GET_FOCUS: u32 = 0x0082_cd30;
    pub const SG_GET_FOCUS_ENABLE: u32 = 0x0082_cd40;
    pub const SG_TOGGLE_FOCUS: u32 = 0x0082_cd80;
    pub const SG_SET_FOCUS: u32 = 0x0082_ce20;
    /// How long a focus button has been held (frames, `0x01325940`).
    pub const FOCUS_HELD: Global<i32> = Global::new(0x0132_5940);
    /// `"Focus"`, the toggle's label, `"\xa3"`, `""`, `"\xa5"`, `"ON"`,
    /// `"OFF"`, `"    %s"`, `"%x"`.
    pub const FOCUS_TITLE: u32 = 0x00b5_b3c8;
    pub const FOCUS_TOGGLE: u32 = 0x00b5_b8d8;
    pub const FOCUS_DOWN: u32 = 0x00b5_adcc;
    pub const FOCUS_EMPTY: u32 = 0x00b8_fbeb;
    pub const FOCUS_UP: u32 = 0x00b5_add4;
    pub const TEXT_ON: u32 = 0x00b4_0648;
    pub const TEXT_OFF: u32 = 0x00b4_0644;
    pub const FOCUS_STATE: u32 = 0x00b5_b8dc;
    pub const FMT_X: u32 = 0x00b4_1f0c;
    pub const GU_PRINTF: u32 = 0x0084_8880;

    /// `Gu_FocusProc_00854980(window, msg, id)` (name ours; frame at `ebp`:
    /// a number at `ebp - 0x18`): the focus window. Create (`0x8001`):
    /// title, the toggle button `0x4d2`, and per focus value (two, ids
    /// 0x00 and 0x10): its index, a down button (`id`), its value static
    /// (`id + 1`), an up button (`id + 2`). A press (`0x8005`) or repeat
    /// (`0x8009`) of a down/up button moves the value by 0x10 / 0x100 (8
    /// times that after 60 frames held; the hold counter counts every
    /// press and repeat), clamped to `0..=0x50000`; a release (`0x8006`,
    /// `0x800a`) clears the counter; the toggle button's release
    /// (`0x8007`, `0x800b`) runs `Sg_ToggleFocus`. Paint (`0x8011`): the
    /// on/off state and each value in hex. Returns 0.
    pub fn focus_proc(h: &mut impl Host, ebp: u32, window: u32, msg: u32, id: u32) -> u32 {
        let num = ebp.wrapping_sub(0x18);
        let toggle = id == 0x4d2;
        match msg {
            0x8001 => {
                let t = h.rebase(FOCUS_TITLE);
                cdecl(h, GU_WINDOW_SET_TITLE, &[window, t]);
                let l = h.rebase(FOCUS_TOGGLE);
                cdecl(h, GU_WINDOW_ADD_BUTTON, &[window, 0x4d2, l, 8, 0, 0x14, 9]);
                let mut y = 10u32;
                let mut i = 0u32;
                while i < 0x20 {
                    let fmt = h.rebase(FMT_D);
                    import(h, SPRINTF_IAT, &[num, fmt, i >> 4]);
                    let args = [window, 0xffff_ffff, num, 2, y, 10, 9];
                    cdecl(h, GU_WINDOW_ADD_STATIC, &args);
                    let l = h.rebase(FOCUS_DOWN);
                    cdecl(h, GU_WINDOW_ADD_BUTTON, &[window, i, l, 0xe, y, 0xe, 9]);
                    let l = h.rebase(FOCUS_EMPTY);
                    cdecl(
                        h,
                        GU_WINDOW_ADD_STATIC,
                        &[window, i + 1, l, 0x1b, y, 0x32, 9],
                    );
                    let l = h.rebase(FOCUS_UP);
                    cdecl(
                        h,
                        GU_WINDOW_ADD_BUTTON,
                        &[window, i + 2, l, 0x52, y, 0xe, 9],
                    );
                    i += 0x10;
                    y += 10;
                }
            }
            0x8005 | 0x8009 if !toggle => {
                let slot = (id as i32 >> 4) as u32;
                let big = |held: i32| if held <= 0x3c { 1 } else { 8 };
                let shift = if msg == 0x8005 { 4 } else { 8 };
                let k = if msg == 0x8009 {
                    big(FOCUS_HELD.get(h))
                } else {
                    0
                };
                let v = cdecl(h, SG_GET_FOCUS, &[slot]) as i32;
                let k = if msg == 0x8005 {
                    big(FOCUS_HELD.get(h))
                } else {
                    k
                };
                let v = match id & 0xf {
                    0 => v.wrapping_add(-k << shift),
                    2 => v.wrapping_add(k << shift),
                    _ => v,
                };
                let held = FOCUS_HELD.get(h);
                FOCUS_HELD.set(h, held.wrapping_add(1));
                let v = if v < 0 {
                    0
                } else if v > 0x50000 {
                    0x50000
                } else {
                    v
                };
                cdecl(h, SG_SET_FOCUS, &[slot, v as u32]);
            }
            0x8006 | 0x800a if !toggle => FOCUS_HELD.set(h, 0),
            0x8007 | 0x800b if toggle => {
                cdecl(h, SG_TOGGLE_FOCUS, &[]);
            }
            0x8011 => {
                let on = cdecl(h, SG_GET_FOCUS_ENABLE, &[]);
                let s = h.rebase(if on == 0 { TEXT_OFF } else { TEXT_ON });
                let fmt = h.rebase(FOCUS_STATE);
                cdecl(h, GU_PRINTF, &[fmt, s]);
                let mut i = 0u32;
                while i < 0x20 {
                    let v = cdecl(h, SG_GET_FOCUS, &[i >> 4]);
                    let fmt = h.rebase(FMT_X);
                    import(h, SPRINTF_IAT, &[num, fmt, v]);
                    cdecl(
                        h,
                        GU_WINDOW_CONTROL_SET_TEXT,
                        &[window, (i & 0xf0) + 1, num],
                    );
                    i += 0x10;
                }
            }
            _ => {}
        }
        0
    }

    // ---- the debug font's string core ----

    pub const TK_SET_DEBUG_FONT_BUFFER: u32 = 0x0088_fab0;
    pub const SET_PTR32: u32 = 0x00a4_4700;
    /// `g_TkDebugFontBuffer` (a pointer to the buffer's words: `+0` the
    /// write cursor, `+0xc` the end), the glyph-page mask, the x cursor,
    /// the y, the tab origin; `g_FontWidths`.
    pub const FONT_BUFFER: Global<u32> = Global::new(0x0133_c8cc);
    pub const FONT_PAGES: Global<u32> = Global::new(0x0133_c8d0);
    pub const FONT_X: Global<i32> = Global::new(0x0133_c8d4);
    pub const FONT_Y: Global<u32> = Global::new(0x0133_c8d8);
    pub const FONT_TAB_ORIGIN: Global<i32> = Global::new(0x0133_c8dc);
    pub const FONT_WIDTHS: u32 = 0x023c_bf6c;
    pub const NO_FREE_BUFFER: u32 = 0x00b5_ec28;

    /// `TkPutDebugFontStringCore_0088f240(prop, colour, scale, s)`: with
    /// a non-empty `s` and the x cursor at most 0x2000, the string's
    /// glyphs as sprite blocks of at most 32 (`colour` clamped to 0..=7,
    /// `scale` to 0..=0x3ff), at the buffer's cursor. A byte from 0x80
    /// starts a two-byte code (page `b - 0x80`, from 0xe0 `b - 0xc2`; the
    /// glyph the second byte); a tab moves x to the next 0x200 column
    /// from the tab origin; control bytes print as spaces. Each glyph is
    /// a quadword (`0x6503c000 | 3i`, its cell, x, y, page), the page's
    /// bit set in the mask, x advanced by its width (`prop`:
    /// `(g_FontWidths[code] + 2) * 16`, else 0x90 or 0xd0 for two-byte
    /// codes). A block closes with its colour/scale word over the last
    /// glyph's top half, a closing quadword, and its GIF tag; past
    /// x 0x2000 the string ends. At the end the cursor is stored back
    /// (`TkSetDebugFontBuffer`, `set_ptr32`; past the buffer's end the
    /// "no free buffer" message) and the mask returned; an empty or
    /// clipped string returns `s`.
    pub fn put_string_core(h: &mut impl Host, prop: u32, colour: u32, scale: u32, s: u32) -> u32 {
        if h.read::<u8>(s) == 0 || FONT_X.get(h) > 0x2000 {
            return s;
        }
        let colour = colour as i32;
        let colour = if colour > 7 {
            7
        } else if colour < 0 {
            0
        } else {
            colour
        };
        let scale = scale as i32;
        let scale = if scale > 0x3ff {
            0x3ff
        } else if scale < 0 {
            0
        } else {
            scale
        };
        let mut mask = FONT_PAGES.get(h);
        let buf = FONT_BUFFER.get(h);
        let mut block = get_ptr32_func(h.read(buf));
        let mut x = FONT_X.get(h);
        let mut q = at(block, 0x10);
        let mut count = 0u32;
        let mut s = s;
        loop {
            let b = u32::from(h.read::<u8>(s));
            s = at(s, 1);
            let mut glyph = b;
            let mut code = b;
            let mut page = 0u32;
            if b >= 0x80 {
                let lead = if b >= 0xe0 { b - 0x42 } else { b };
                glyph = u32::from(h.read::<u8>(s));
                s = at(s, 1);
                page = lead.wrapping_sub(0x80);
                code = (b << 8).wrapping_add(glyph);
            }
            if glyph == 9 {
                let base = FONT_TAB_ORIGIN.get(h);
                let t = x.wrapping_sub(base).wrapping_add(0x200);
                x = (t.wrapping_add((t >> 31) & 0x1ff) & !0x1ff).wrapping_add(base);
                FONT_X.set(h, x);
            } else if glyph != 0 && glyph < 0x20 {
                glyph = 0x20;
            }
            let flush = if x > 0x2000 && count != 0 {
                page = 0;
                code = 0;
                glyph = 0;
                true
            } else {
                glyph == 0 || count == 0x20
            };
            if flush {
                if count != 0 {
                    let word = ((scale << 4) | (colour + 1)) as u16;
                    h.write(q.wrapping_sub(2), word);
                    let mid = if glyph == 0 { 0x1300_0000u32 } else { 0 };
                    h.write(q, [0x1300_0000u32, 0x1700_0000, mid, 0]);
                    q = at(q, 0x10);
                    let n = (q.wrapping_sub(block).wrapping_add(0xf) >> 4).wrapping_sub(1);
                    h.write(block, n as u16);
                    h.write(at(block, 2), 0x1000u16);
                    h.write(at(block, 4), [0u32, 0x0100_0404, 0]);
                }
                if glyph == 0 {
                    break;
                }
                block = q;
                q = at(q, 0x10);
                count = 0;
            }
            h.write(q, (count * 3) | 0x6503_c000);
            count += 1;
            h.write(at(q, 4), ((glyph & 0xf) << 8) as u16);
            let y = FONT_Y.get(h);
            h.write(at(q, 6), ((glyph >> 4) << 8) as u16);
            h.write(at(q, 8), x as u16);
            h.write(at(q, 0xa), y as u16);
            h.write(at(q, 0xc), page as u16);
            h.write(at(q, 0xe), 0u16);
            mask |= 1u32 << ((page >> 2) & 31);
            q = at(q, 0x10);
            let adv = if prop != 0 {
                let w: u8 = h.read(at(h.rebase(FONT_WIDTHS), code));
                (i32::from(w) + 2) << 4
            } else if code >= 0x100 {
                0xd0
            } else {
                0x90
            };
            x = x.wrapping_add(adv);
            FONT_X.set(h, x);
        }
        cdecl(h, TK_SET_DEBUG_FONT_BUFFER, &[q]);
        let v = cdecl(h, SET_PTR32, &[q]);
        let buf = FONT_BUFFER.get(h);
        h.write(buf, v);
        let end = get_ptr32_func(h.read(at(buf, 0xc)));
        if q > end {
            let m = h.rebase(NO_FREE_BUFFER);
            cdecl(h, DBG_PRINTF, &[m]);
        }
        FONT_PAGES.set(h, mask);
        mask
    }

    // ---- the PS2 memory card's read callback ----

    pub const SCE_MC_READ_0062F650: u32 = 0x0062_f650;
    pub const SCE_MC_0062F670: u32 = 0x0062_f670;
    /// The card buffer (`0x0132e570`, 0x1000 bytes), its header bytes and
    /// words, the copy cursor and the bytes left.
    pub const MC_BUFFER: u32 = 0x0132_e570;
    pub const MC_DEST: Global<u32> = Global::new(0x0132_f578);
    pub const MC_LEFT: Global<u16> = Global::new(0x0132_f57e);
    /// The three counters it adds to (`| 0x20000000`, the PS2's uncached
    /// mirror of these words).
    pub const MC_SUM_A: u32 = 0x0132_d56c;
    pub const MC_SUM_B: u32 = 0x0132_f584;
    pub const MC_LAST: u32 = 0x0132_f588;

    /// `TkMcCallback_008852a0(op, size, req)` (name ours): the card
    /// request's result step (`op` stored at `req + 0x12`). 1 (read):
    /// `0x0062f650(req[0], buffer, n)` with `n = min(size, left)`; when
    /// the request wasn't done (`+0x11 != 2`) and the buffer's header
    /// says done (`+8 == 2`), its two `i16` counts are added to the
    /// uncached mirrors and its `u16` stored; `n` capped at 0x1000 (the
    /// request then done); a negative result sets `+0xe` to -1, else the
    /// buffer is copied to the cursor (`memcpy`, with any left), which
    /// moves on. 2: `+8` cleared, `+0x11` 0. 3: `0x0062f670(req[0], req[1],
    /// u16 +0xc)`, a negative result sets `+0xc` to -1. 4: `+0x10` 0.
    /// 6: a set `+0x10` sets `+0xc` to -1, a set `+0x11` sets `+0xe`; both
    /// cleared. Returns `eax` as the original leaves it.
    pub fn mc_callback(h: &mut impl Host, op: u32, size: u32, req: u32) -> u32 {
        h.write(at(req, 0x12), op as u8);
        match op {
            1 => {
                let left = u32::from(MC_LEFT.get(h));
                let mut n = if (size as i32) > left as i32 {
                    left
                } else {
                    size
                };
                let r0: u32 = h.read(req);
                let buffer = h.rebase(MC_BUFFER);
                let r = cdecl(h, SCE_MC_READ_0062F650, &[r0, buffer, n]);
                let mut eax = r;
                if h.read::<u8>(at(req, 0x11)) != 2 && h.read::<u8>(at(buffer, 8)) == 2 {
                    let a = i32::from(h.read::<i16>(at(buffer, 0xe)));
                    let p = h.rebase(MC_SUM_A) | 0x2000_0000;
                    let v: i32 = h.read(p);
                    h.write(p, v.wrapping_add(a));
                    let b = i32::from(h.read::<i16>(at(buffer, 0x10)));
                    let p = h.rebase(MC_SUM_B) | 0x2000_0000;
                    let v: i32 = h.read(p);
                    h.write(p, v.wrapping_add(b));
                    let c = u32::from(h.read::<u16>(at(buffer, 0xc)));
                    let p = h.rebase(MC_LAST) | 0x2000_0000;
                    h.write(p, c);
                    eax = p;
                }
                if (n as i32) > 0x1000 {
                    n = 0x1000;
                    h.write(at(req, 0x11), 2u8);
                }
                if (r as i32) < 0 {
                    h.write(at(req, 0xe), 0xffffu16);
                    return 0xffff;
                }
                let dest = MC_DEST.get(h);
                if dest == 0 {
                    return eax;
                }
                let mut d = dest;
                if MC_LEFT.get(h) != 0 {
                    cdecl(h, CRT_MEMCPY, &[dest, buffer, n]);
                    let left = MC_LEFT.get(h);
                    eax = u32::from(left).wrapping_sub(n);
                    MC_LEFT.set(h, eax as u16);
                    d = dest.wrapping_add(n);
                }
                MC_DEST.set(h, d);
                eax
            }
            2 => {
                h.write(at(req, 8), 0u32);
                h.write(at(req, 0x11), 0u8);
                1
            }
            3 => {
                let r0: u32 = h.read(req);
                let r1: u32 = h.read(at(req, 4));
                let c = u32::from(h.read::<u16>(at(req, 0xc)));
                let r = cdecl(h, SCE_MC_0062F670, &[r0, r1, c]);
                if (r as i32) < 0 {
                    h.write(at(req, 0xc), 0xffffu16);
                    return 0xffff;
                }
                r
            }
            4 => {
                h.write(at(req, 0x10), 0u8);
                3
            }
            6 => {
                if h.read::<u8>(at(req, 0x10)) != 0 {
                    h.write(at(req, 0xc), 0xffffu16);
                    h.write(at(req, 0x10), 0u8);
                }
                if h.read::<u8>(at(req, 0x11)) != 0 {
                    h.write(at(req, 0xe), 0xffffu16);
                    h.write(at(req, 0x11), 0u8);
                }
                0xffff
            }
            _ => op.wrapping_sub(1),
        }
    }

    /// `memcpy` (`0x00949256`, the CRT's).
    pub const CRT_MEMCPY: u32 = 0x0094_9256;

    // ---- the battle-end screen's draw (batch 2) ----

    pub const TK_VU1_SYNC_PATH: u32 = 0x0088_ec60;
    pub const TK_MENU_DRAW_SCISSOR_CLEAR: u32 = 0x008d_d440;
    pub const TO_DRAW_MENU_BG: u32 = 0x008f_5c10;
    pub const TK_BTL_END_DRAW: u32 = 0x008b_c340;
    pub const BTL_END_DRAW_GIL_PAGES: u32 = 0x008b_c080;
    pub const BTL_END_DRAW_SPOIL_PAGES: u32 = 0x008b_d940;
    pub const SND_SEP_STOP: u32 = 0x0088_6eb0;
    pub const SND_SEP_PLAY_SIMPLE: u32 = 0x0088_6e80;
    pub const BTL_END_DRAW_CAPTION: u32 = 0x008b_9f30;
    pub const TK_BTL_END_DRAW_LEGEND_NOTICE: u32 = 0x008b_c100;
    pub const BTL_END_DRAW_FRAME: u32 = 0x008b_b9c0;
    pub const BTL_END_DRAW_CURSOR: u32 = 0x008b_b8c0;
    pub const ATEL_GET_KEEP_PLAY_STATUS: u32 = 0x0086_bc60;
    pub const TK_MENU_DRAW_RECT: u32 = 0x008d_d340;
    pub const TK_MENU_DRAW_KICK: u32 = 0x008d_ce50;
    pub const TO_MENU_CLEAR_MENU_TYPE: u32 = 0x008b_e3f0;
    pub const BTL_END_TAKE_SPOILS: u32 = 0x008b_d9c0;
    pub const MS_BTL_GET_INIT: u32 = 0x0079_8b60;
    pub const TK_MENU_MAIN_REMOVE_MENU: u32 = 0x008a_ab20;
    /// The battle-end draw state (0 first frame, 1 drawing).
    pub const BTL_END_DRAW_STATE: Global<u32> = Global::new(0x0186_9f0c);
    /// `g_TkBtlEndList`'s count, `g_TkBtlEndFlag`, `g_BtlEndFadeMain`,
    /// `g_BtlEndCtrlDone`.
    pub const BTL_END_COUNT: Global<u8> = Global::new(0x0186_9f18);
    pub const BTL_END_FLAG: Global<u32> = Global::new(0x0186_9fbc);
    pub const BTL_END_FADE_MAIN: Global<i16> = Global::new(0x0186_a0b4);
    pub const BTL_END_CTRL_DONE: Global<u8> = Global::new(0x00c5_a068);

    /// `BtlEnd_Draw_008bb7a0()` (name ours; the battle-end screen's table
    /// entry, data `0x00c59528`): `TkVU1SyncPath`, the scissor cleared,
    /// the menu background; in state 0 (then 1) or 1 the result rows
    /// (`TkBtlEndDraw(i)` for each, the count re-read), the cursor
    /// (`0x008bb8c0`) while any, and the gil and spoil pages. A set
    /// `g_TkBtlEndFlag` stops and replays sound `0x80000017`; the flag
    /// cleared. The caption, the legend notice, the frame (`0x008bb9c0`);
    /// below full fade the black cover (alpha `((0x1000 - fade) << 7) /
    /// 0x1000` in the top byte, white while the fanfare keeps playing) over
    /// the screen; the scissor cleared, two kicks; with
    /// `g_BtlEndCtrlDone` the menu type cleared, the spoils taken, the
    /// battle counts reset and menu 0x11 removed. Returns the last callee's
    /// `eax`.
    pub fn btl_end_draw(h: &mut impl Host) -> u32 {
        cdecl(h, TK_VU1_SYNC_PATH, &[]);
        cdecl(h, TK_MENU_DRAW_SCISSOR_CLEAR, &[]);
        cdecl(h, TO_DRAW_MENU_BG, &[]);
        let state = BTL_END_DRAW_STATE.get(h);
        if state == 0 {
            BTL_END_DRAW_STATE.set(h, 1);
        }
        if state <= 1 {
            let mut n = BTL_END_COUNT.get(h);
            if n != 0 {
                let mut i = 0u32;
                loop {
                    cdecl(h, TK_BTL_END_DRAW, &[i]);
                    n = BTL_END_COUNT.get(h);
                    i += 1;
                    if i >= u32::from(n) {
                        break;
                    }
                }
                if n != 0 {
                    cdecl(h, BTL_END_DRAW_CURSOR, &[]);
                }
            }
            cdecl(h, BTL_END_DRAW_GIL_PAGES, &[]);
            cdecl(h, BTL_END_DRAW_SPOIL_PAGES, &[]);
        }
        if BTL_END_FLAG.get(h) != 0 {
            cdecl(h, SND_SEP_STOP, &[0x8000_0017]);
            cdecl(h, SND_SEP_PLAY_SIMPLE, &[0x8000_0017]);
        }
        BTL_END_FLAG.set(h, 0);
        cdecl(h, BTL_END_DRAW_CAPTION, &[]);
        cdecl(h, TK_BTL_END_DRAW_LEGEND_NOTICE, &[]);
        cdecl(h, BTL_END_DRAW_FRAME, &[]);
        let fade = BTL_END_FADE_MAIN.get(h);
        if fade != 0x1000 {
            let a = (0x1000 - i32::from(fade)) << 7;
            let mut c = ((a.wrapping_add((a >> 31) & 0xfff) >> 12) as u32) << 24;
            if cdecl(h, ATEL_GET_KEEP_PLAY_STATUS, &[]) != 0 {
                c |= 0x00ff_ffff;
            }
            cdecl(h, TK_MENU_DRAW_RECT, &[0, 0, 0x200, 0x1a0, c, c]);
        }
        cdecl(h, TK_MENU_DRAW_SCISSOR_CLEAR, &[]);
        cdecl(h, TK_MENU_DRAW_KICK, &[]);
        let eax = cdecl(h, TK_MENU_DRAW_KICK, &[]);
        if BTL_END_CTRL_DONE.get(h) == 0 {
            return eax;
        }
        cdecl(h, TO_MENU_CLEAR_MENU_TYPE, &[0x10000]);
        cdecl(h, BTL_END_TAKE_SPOILS, &[]);
        cdecl(h, MS_BTL_GET_INIT, &[]);
        cdecl(h, TK_MENU_MAIN_REMOVE_MENU, &[0x11])
    }

    pub const MATH_COS_LOOKUP: u32 = 0x0088_e750;
    pub const MATH_SIN_LOOKUP: u32 = 0x0088_e730;
    /// `graphicUiRemapY2`, `graphicUiRemapX2` (`f32` in, `ST0` out).
    pub const UI_REMAP_Y2: u32 = 0x0064_4820;
    pub const UI_REMAP_X2: u32 = 0x0064_47e0;
    pub const TO_DRAW_MENU_PLATE_XYWH_TYPE: u32 = 0x008f_5f70;
    pub const TO_DRAW_SHEET_CELL: u32 = 0x008f_8bb0;
    pub const TO_MKP_SHAPE_XYWH_UV: u32 = 0x0090_3bb0;
    pub const TO_MKP_SHAPE_XYWH_UVC2: u32 = 0x0090_3ee0;
    pub const TO_MKP_CROSS_EXT_MES_FONT_L_CLUT: u32 = 0x0090_16b0;
    /// The battle-end cursor's state and timer (`i16`).
    pub const BTL_END_CURSOR_STATE: Global<i16> = Global::new(0x0186_9f24);
    pub const BTL_END_CURSOR_TIME: Global<i16> = Global::new(0x0186_9f22);
    /// The battle-end screen's message (`0x025cc588`).
    pub const BTL_END_MESSAGE: Global<u32> = Global::new(0x025c_c588);

    /// An `f32` constant's bits (`fld`/`fstp` keep them).
    fn cst(h: &impl Host, sym: u32) -> u32 {
        h.read(h.rebase(sym))
    }

    /// `graphicUiRemap{X,Y}2` of the constant at `sym`, as `fstp dword`
    /// stores it.
    fn remap(h: &mut impl Host, f: u32, sym: u32) -> u32 {
        let v = cst(h, sym);
        h.call_sym_st0(f, Conv::Cdecl, &[v]).to_bits()
    }

    /// `BtlEnd_DrawCursor_008bb8c0()` (name ours): by the cursor state
    /// (`i16`): 0 and 5 nothing (returns the state); 1 a cosine and 4 a
    /// sine lookup of the timer (results unused); then, as for 2, 3 and
    /// any other value, the plate `TODrawMenuPlateXYWHType(x, y, w, h, 2)`
    /// and sheet cell 10 over remapped constants. Returns the cell's
    /// `eax`.
    pub fn btl_end_draw_cursor(h: &mut impl Host) -> u32 {
        let s = i32::from(BTL_END_CURSOR_STATE.get(h));
        match s {
            0 | 5 => return s as u32,
            1 => {
                let t = i32::from(BTL_END_CURSOR_TIME.get(h));
                cdecl(h, MATH_COS_LOOKUP, &[t as u32]);
            }
            4 => {
                let t = i32::from(BTL_END_CURSOR_TIME.get(h));
                cdecl(h, MATH_SIN_LOOKUP, &[0x1000u32.wrapping_sub(t as u32)]);
            }
            _ => {}
        }
        let a = remap(h, UI_REMAP_Y2, 0x00b3_eb9c);
        let b = remap(h, UI_REMAP_X2, 0x00b5_eff4);
        let c = remap(h, UI_REMAP_Y2, 0x00b6_56d4);
        let d = remap(h, UI_REMAP_X2, 0x00b5_efc4);
        cdecl(h, TO_DRAW_MENU_PLATE_XYWH_TYPE, &[d, c, b, a, 2]);
        let e = remap(h, UI_REMAP_Y2, 0x00b5_f2f8);
        let f = remap(h, UI_REMAP_X2, 0x00b5_ef68);
        let g = remap(h, UI_REMAP_Y2, 0x00b5_f35c);
        let k = remap(h, UI_REMAP_X2, 0x00b5_f3e8);
        cdecl(h, TO_DRAW_SHEET_CELL, &[0xa, k, g, f, e])
    }

    /// `BtlEnd_DrawFrame_008bb9c0()` (name ours): the battle-end window:
    /// `TOMkpShapeXYWHUVC2(-1, ...)` grey, `TOMkpShapeXYWHUV(0x3e, ...)`,
    /// and with a message (`0x025cc588`) its text
    /// (`TOMkpCrossExtMesFontLClut(0, msg, x, y, 0, size, 1.0)`), at
    /// remapped constants. Returns the last call's `eax`.
    pub fn btl_end_draw_frame(h: &mut impl Host) -> u32 {
        let a1 = remap(h, UI_REMAP_Y2, 0x00b5_eea8);
        let a2 = remap(h, UI_REMAP_X2, 0x00b5_f030);
        let a3 = remap(h, UI_REMAP_Y2, 0x00b4_3db8);
        let a4 = remap(h, UI_REMAP_X2, 0x00b5_eef0);
        let args = [
            0xffff_ffff,
            a4,
            a3,
            a2,
            a1,
            0,
            cst(h, 0x00b5_efb0),
            cst(h, 0x00b5_f030),
            cst(h, 0x00b5_efcc),
            0x8080_8080,
            0x0080_8080,
        ];
        h.call_sym_wide(TO_MKP_SHAPE_XYWH_UVC2, &args);
        let b1 = remap(h, UI_REMAP_Y2, 0x00b5_ee64);
        let b2 = remap(h, UI_REMAP_X2, 0x00b5_eedc);
        let b3 = remap(h, UI_REMAP_Y2, 0x00b5_eed0);
        let b4 = remap(h, UI_REMAP_X2, 0x00b5_eef0);
        let args = [
            0x3e,
            b4,
            b3,
            b2,
            b1,
            cst(h, 0x00b5_edf4),
            cst(h, 0x00b5_ee20),
            cst(h, 0x00b5_ee0c),
            cst(h, 0x00b5_ee2c),
        ];
        let eax = cdecl(h, TO_MKP_SHAPE_XYWH_UV, &args);
        let msg = BTL_END_MESSAGE.get(h);
        if msg == 0 {
            return eax;
        }
        let d1 = remap(h, UI_REMAP_Y2, 0x00b5_eee4);
        let d2 = remap(h, UI_REMAP_X2, 0x00b5_ef2c);
        let size = cst(h, 0x00b5_ee10);
        let args = [0, msg, d2, d1, 0, size, 0x3f80_0000];
        cdecl(h, TO_MKP_CROSS_EXT_MES_FONT_L_CLUT, &args)
    }

    pub const TO_DRAW_SCISSOR_XYWH: u32 = 0x008f_9230;
    pub const MS_GET_SAVE_WEAPON: u32 = 0x007a_bbd0;
    pub const MS_CALC_WEAPON_CAP: u32 = 0x0079_8eb0;
    pub const TO_DRAW_CROSS_MENU_ICON_ABILITY_NAME: u32 = 0x008f_4f10;
    /// The spoils list's state: the selection mode words and indices, the
    /// item list (`0x025cc580`), the party's weapons (`u16` from
    /// `0x02310f8e`).
    pub const SPOIL_MODE: Global<u32> = Global::new(0x0186_a19c);
    pub const SPOIL_FLIP: Global<u32> = Global::new(0x0186_a1a8);
    pub const SPOIL_TOP: Global<u32> = Global::new(0x0186_a190);
    pub const SPOIL_CURSOR: Global<u32> = Global::new(0x0186_a1a0);
    pub const SPOIL_ITEMS: Global<u32> = Global::new(0x025c_c580);
    pub const SPOIL_WEAPON_INDEX: Global<u32> = Global::new(0x0186_a1a4);
    pub const SPOIL_WEAPONS: u32 = 0x0231_0f8e;

    /// `f32(a + b)` the way `fadd dword` then `fstp dword` rounds it.
    fn fadd(a: u32, b: u32) -> u32 {
        ffx_bridge::x87::store(ffx_bridge::x87::add(
            ffx_bridge::x87::load(a),
            ffx_bridge::x87::load(b),
        ))
    }

    /// `BtlEnd_DrawWeaponAbilities_008bd020(x)` (name ours; frame at
    /// `ebp`: `MsGetSaveWeapon`'s out word at `ebp - 0x10`): the scissor
    /// over the screen, the plate and sheet cell 7 shifted right by `x`
    /// (`f32`), then the selected weapon's (`MsGetSaveWeapon` of the item
    /// list's entry, or with the mode flipped the party's weapon) ability
    /// rows: when it exists (`+2`), `MsCalcWeaponCap`, and for each of its
    /// `u8 +0xb` slots (`u16` from `+0xe`) a plate, with the ability's
    /// icon and name unless the slot is 0 or 0xff, each row lower by a
    /// remapped step. Returns the weapon (no weapon), `MsCalcWeaponCap`'s
    /// `eax` (no slots) or the slot count.
    pub fn btl_end_weapon_abilities(h: &mut impl Host, ebp: u32, x: u32) -> u32 {
        cdecl(h, TO_DRAW_SCISSOR_XYWH, &[0, 0, 0x200, 0x1a0]);
        let fx = ffx_bridge::x87::store(f64::from(x as i32));
        let left = fadd(remap(h, UI_REMAP_X2, 0x00b5_ef1c), fx);
        let mut y = remap(h, UI_REMAP_Y2, 0x00b6_5708);
        let a1 = remap(h, UI_REMAP_Y2, 0x00b3_eb9c);
        let b2 = remap(h, UI_REMAP_X2, 0x00b5_eff4);
        let c3 = remap(h, UI_REMAP_Y2, 0x00b5_efdc);
        let e4 = fadd(remap(h, UI_REMAP_X2, 0x00b5_ef1c), fx);
        cdecl(h, TO_DRAW_MENU_PLATE_XYWH_TYPE, &[e4, c3, b2, a1, 2]);
        let g1 = remap(h, UI_REMAP_Y2, 0x00b5_f2f8);
        let g2 = remap(h, UI_REMAP_X2, 0x00b5_ef68);
        let g3 = remap(h, UI_REMAP_Y2, 0x00b5_0220);
        let g4 = fadd(remap(h, UI_REMAP_X2, 0x00b5_f3b0), fx);
        cdecl(h, TO_DRAW_SHEET_CELL, &[7, g4, g3, g2, g1]);
        let mut party = SPOIL_MODE.get(h);
        if SPOIL_FLIP.get(h) != 0 {
            party = u32::from(party == 0);
        }
        let id = if party == 0 {
            let i = SPOIL_TOP.get(h).wrapping_add(SPOIL_CURSOR.get(h));
            let list = SPOIL_ITEMS.get(h);
            u32::from(h.read::<u16>(at(list, i.wrapping_mul(2))))
        } else {
            let i = SPOIL_WEAPON_INDEX.get(h);
            let p = h.rebase(SPOIL_WEAPONS).wrapping_add(i.wrapping_mul(2));
            u32::from(h.read::<u16>(p))
        };
        let w = cdecl(h, MS_GET_SAVE_WEAPON, &[id, ebp.wrapping_sub(0x10)]);
        if h.read::<u8>(at(w, 2)) == 0 {
            return w;
        }
        let eax = cdecl(h, MS_CALC_WEAPON_CAP, &[w]);
        let mut n = u32::from(h.read::<u8>(at(w, 0xb)));
        if n == 0 {
            return eax;
        }
        let mut i = 0u32;
        loop {
            let a = u32::from(h.read::<u16>(at(w, 0xe + 2 * i)));
            let ph = remap(h, UI_REMAP_Y2, 0x00b3_eb9c);
            let pw = remap(h, UI_REMAP_X2, 0x00b5_eff4);
            cdecl(h, TO_DRAW_MENU_PLATE_XYWH_TYPE, &[left, y, pw, ph, 7]);
            if a != 0 && a != 0xff {
                cdecl(h, TO_DRAW_CROSS_MENU_ICON_ABILITY_NAME, &[a, left, y, 0]);
            }
            i += 1;
            let step = remap(h, UI_REMAP_Y2, 0x00b4_1d88);
            y = fadd(step, y);
            n = u32::from(h.read::<u8>(at(w, 0xb)));
            if i as i32 >= n as i32 {
                return n;
            }
        }
    }

    pub const MENU_DRAW_DIVIDER_SHAPES: u32 = 0x008e_71d0;
    pub const TK_VU1_SYNC_PATH_THUNK: u32 = 0x008c_1790;
    pub const TK_MN2_DRAW_INIT_PACKET: u32 = 0x008c_0be0;
    pub const TK_MENU_GET_CURRENT_PLAYER: u32 = 0x008a_9860;
    pub const MENU_STATUS_DRAW_PAGE: u32 = 0x008d_4ee0;
    pub const TK_MENU_EQUIP_DRAW_CURSOR: u32 = 0x008c_f990;
    pub const MENU_EQUIP_DRAW_WEAPON_ABILITIES: u32 = 0x008d_02e0;
    pub const TK_MN2_DRAW_KICK_SYNC_PACKET: u32 = 0x008c_0cc0;
    /// `g_TkMenuEquipFade`; the status page's four out words.
    pub const MENU_EQUIP_FADE: Global<u32> = Global::new(0x0186_a614);
    pub const EQUIP_STATUS_WORDS: [u32; 4] = [0x023c_c3c8, 0x023c_c3c4, 0x023c_c3c0, 0x023c_c3bc];

    /// A plate (`TODrawMenuPlateXYWHType(x, y, w, h, 2)`) whose remapped
    /// constants are `(h_sym, w_sym, y_sym, x_sym)`.
    fn plate(h: &mut impl Host, syms: [u32; 4]) -> u32 {
        let a = remap(h, UI_REMAP_Y2, syms[0]);
        let b = remap(h, UI_REMAP_X2, syms[1]);
        let c = remap(h, UI_REMAP_Y2, syms[2]);
        let d = remap(h, UI_REMAP_X2, syms[3]);
        cdecl(h, TO_DRAW_MENU_PLATE_XYWH_TYPE, &[d, c, b, a, 2])
    }

    /// Sheet cell `cell` over the remapped `(h, w, y, x)` constants.
    fn sheet_cell(h: &mut impl Host, cell: u32, syms: [u32; 4]) -> u32 {
        let a = remap(h, UI_REMAP_Y2, syms[0]);
        let b = remap(h, UI_REMAP_X2, syms[1]);
        let c = remap(h, UI_REMAP_Y2, syms[2]);
        let d = remap(h, UI_REMAP_X2, syms[3]);
        cdecl(h, TO_DRAW_SHEET_CELL, &[cell, d, c, b, a])
    }

    /// `MenuEquip_Draw_008cf670()` (name ours; the equipment screen's
    /// table entry, data `0x00c5a748`): background, dividers (style 4),
    /// two kicks, the VU1 sync, the packet set up, the current player's
    /// status page 0 (`MenuStatus_DrawPage(0, 0, player, 0, the four status
    /// words, g_TkMenuEquipFade)`), two plates with sheet cells 6 and 7,
    /// the command cursor, the weapon's abilities; tail-jumps to
    /// `TkMn2DrawKickSyncPacket` (its `eax`).
    pub fn equip_draw(h: &mut impl Host) -> u32 {
        cdecl(h, TO_DRAW_MENU_BG, &[]);
        cdecl(h, MENU_DRAW_DIVIDER_SHAPES, &[4]);
        cdecl(h, TK_MENU_DRAW_KICK, &[]);
        cdecl(h, TK_MENU_DRAW_KICK, &[]);
        cdecl(h, TK_VU1_SYNC_PATH_THUNK, &[]);
        cdecl(h, TK_MN2_DRAW_INIT_PACKET, &[]);
        let player = cdecl(h, TK_MENU_GET_CURRENT_PLAYER, &[]);
        let fade = MENU_EQUIP_FADE.get(h);
        let w = EQUIP_STATUS_WORDS.map(|s| h.rebase(s));
        let args = [0, 0, player, 0, w[0], w[1], w[2], w[3], fade];
        cdecl(h, MENU_STATUS_DRAW_PAGE, &args);
        plate(h, [0x00b3_eb9c, 0x00b5_eff4, 0x00b5_efa4, 0x00b5_ef1c]);
        sheet_cell(h, 6, [0x00b5_f2f8, 0x00b5_ef68, 0x00b6_5e50, 0x00b5_f3b0]);
        plate(h, [0x00b3_eb9c, 0x00b5_eff4, 0x00b5_efa4, 0x00b6_5724]);
        sheet_cell(h, 7, [0x00b5_f2f8, 0x00b5_ef68, 0x00b6_5e50, 0x00b6_5b3c]);
        cdecl(h, TK_MENU_EQUIP_DRAW_CURSOR, &[]);
        cdecl(h, MENU_EQUIP_DRAW_WEAPON_ABILITIES, &[]);
        cdecl(h, TK_MN2_DRAW_KICK_SYNC_PACKET, &[])
    }

    /// The customize screen's weapon list (`u16` per row).
    pub const KAIZOU_WEAPONS: Global<u32> = Global::new(0x0186_aa38);

    /// `MenuKaizou_DrawWeaponAbilities_008d63f0(row)` (name ours; frame at
    /// `ebp`: the weapon lookup's out word at `ebp - 0x10`): the row's
    /// weapon (`MsGetSaveWeapon`); when it exists (`+2`),
    /// `MsCalcWeaponCap`, a plate (type 7) per slot (`u8 +0xb`), then the
    /// four ability slots' icons and names (not 0 or 0xff) a remapped
    /// height above each plate's baseline (`f32(y - h)`), each row a
    /// remapped step lower. Returns `eax` as the original leaves it: the
    /// weapon (none), `MsCalcWeaponCap`'s, the slot count, or the last
    /// icon's (the remaps keep `eax`).
    pub fn kaizou_weapon_abilities(h: &mut impl Host, ebp: u32, row: u32) -> u32 {
        let list = KAIZOU_WEAPONS.get(h);
        let id = u32::from(h.read::<u16>(at(list, row.wrapping_mul(2))));
        let w = cdecl(h, MS_GET_SAVE_WEAPON, &[id, ebp.wrapping_sub(0x10)]);
        if h.read::<u8>(at(w, 2)) == 0 {
            return w;
        }
        let x = remap(h, UI_REMAP_X2, 0x00b6_5724);
        let mut y = remap(h, UI_REMAP_Y2, 0x00b6_6384);
        let mut eax = cdecl(h, MS_CALC_WEAPON_CAP, &[w]);
        let mut i = 0u32;
        while i < u32::from(h.read::<u8>(at(w, 0xb))) {
            let ph = remap(h, UI_REMAP_Y2, 0x00b3_eb9c);
            let pw = remap(h, UI_REMAP_X2, 0x00b5_eff4);
            cdecl(h, TO_DRAW_MENU_PLATE_XYWH_TYPE, &[x, y, pw, ph, 7]);
            i += 1;
            let step = remap(h, UI_REMAP_Y2, 0x00b5_eec0);
            y = fadd(step, y);
            eax = u32::from(h.read::<u8>(at(w, 0xb)));
        }
        let x = remap(h, UI_REMAP_X2, 0x00b6_5724);
        let mut y = remap(h, UI_REMAP_Y2, 0x00b6_6384);
        for k in 0..4u32 {
            let a = u32::from(h.read::<u16>(at(w, 0xe + 2 * k)));
            if a != 0 && a != 0xff {
                let up = remap(h, UI_REMAP_Y2, 0x00b9_2268);
                let v = ffx_bridge::x87::sub(ffx_bridge::x87::load(y), ffx_bridge::x87::load(up));
                let yy = ffx_bridge::x87::store(v);
                eax = cdecl(h, TO_DRAW_CROSS_MENU_ICON_ABILITY_NAME, &[a, x, yy, 0]);
            }
            let step = remap(h, UI_REMAP_Y2, 0x00b5_eec0);
            y = fadd(step, y);
        }
        eax
    }

    pub const HARD_CODE_TO_FIX_THE_WRONG_DATA: u32 = 0x008e_02f0;
    pub const TO_GET_GROUP: u32 = 0x008d_f610;
    pub const TO_CONV_ALBHED_WORD: u32 = 0x0090_6780;
    pub const TO_GET_FONT_INFO: u32 = 0x008b_7240;
    pub const TO_MAKE_PKT_FONT_1612: u32 = 0x008f_c7c0;
    pub const TO_FIRST_INIT_MESSAGE_NORMAL: u32 = 0x008d_f520;
    pub const TO_SET_DRAW_INFO: u32 = 0x008d_f8f0;
    pub const TO_DRAW_CHANGE_MACRO2: u32 = 0x008b_5960;
    pub const TO_SKIP_CODE: u32 = 0x008d_fd80;
    /// Per window kind (`i16 +4`): whether its text wraps (`0x025d0b20`).
    pub const WINDOW_WRAPS: u32 = 0x025d_0b20;
    /// `f64` 5.0 (`0x00b41d20`) and `g_Vu0FixedScale` (16.0).
    pub const WRAP_MARGIN: u32 = 0x00b4_1d20;
    pub const LINE_STEP: u32 = 0x00b3_13a0;

    /// `TODrawMessageText_008dea40(win)` (name ours; table entry, data
    /// `0x00c5b1c4`; aligned frame at `ebp`: `ebp - 0x64` a text word the
    /// Al Bhed translation writes, `ebp - 0x60` the font info, `ebp -
    /// 0x20` the pen, 7 words): the pen from `win + 0xb8` with its x, y,
    /// right, `+0x10` and `+0` words from `win + 0x48..0x58` and its colour
    /// byte (`+0x1a` of the pen) from `win + 0x1a`; the cursor `+0x8c` set
    /// to the text `+0x94`, the shown count `+0x9c` 0. While the count is
    /// below `+0xa0`: a wrapping window (`0x025d0b20[i16 +4]`) starts a new
    /// line once x passes `right - 5`; then `hardCodeToFixTheWrongData`,
    /// `TOGetGroup` of the code byte (with that call's upper `eax`), and by
    /// group: 1 a line break (x back, y + 16; window kind 1 re-inits the
    /// message at the next byte), 2 control codes (`TOSetDrawInfo`; button
    /// icons `0x10 0x50..0x57` and the colour macro through
    /// `TODrawChangeMacro2`; code 9 above `0xbf` only at the reveal edge,
    /// else only from `+0xa4`), 3 `TODrawChangeMacro2` (Al Bhed windows,
    /// `u16 +0x10 == 0x4000`, colour 0x56), 4 a character (counted; the
    /// reveal limit `+0xa0` raised toward `+0xa8` unless paused, `+0xb4`
    /// or `+0xac`; Al Bhed text translated first, colour 0x60 or 0x56;
    /// font info, `TOMakePktFont1612`); then `TOSkipCode`. At the end
    /// `+0xa4` = `+0xa0` and the pen back to `win + 0xd4`. Returns 0.
    pub fn message_text(h: &mut impl Host, ebp: u32, win: u32) -> u32 {
        use ffx_bridge::x87;
        let word = ebp.wrapping_sub(0x64);
        let info = ebp.wrapping_sub(0x60);
        let pen = ebp.wrapping_sub(0x20);
        h.write(word, win);
        let text: u32 = h.read(at(win, 0x94));
        h.write(at(win, 0x8c), text);
        let saved: [u32; 7] = h.read(at(win, 0xb8));
        h.write(pen, saved);
        let r: [u32; 5] = h.read(at(win, 0x48));
        h.write(at(pen, 4), [r[0], r[1], r[2], r[3]]);
        h.write(pen, r[4]);
        let c: u8 = h.read(at(win, 0x1a));
        h.write(at(pen, 0x1a), c);
        h.write(at(win, 0x9c), 0u32);
        let i32_at = |h: &_, o: u32| -> i32 { Mem::read(h, at(win, o)) };
        if i32_at(h, 0xa0) > 0 {
            let margin: f64 = h.read(h.rebase(WRAP_MARGIN));
            let step: f64 = h.read(h.rebase(LINE_STEP));
            loop {
                let kind = i32::from(h.read::<i16>(at(win, 4)));
                let wraps: u32 = h.read(h.rebase(WINDOW_WRAPS).wrapping_add((kind as u32) << 2));
                if wraps != 0 {
                    let x = x87::load(h.read(at(pen, 4)));
                    let right = x87::load(h.read(at(pen, 0xc)));
                    if x > x87::sub(right, margin) {
                        let x0: u32 = h.read(at(win, 0x48));
                        h.write(at(pen, 4), x0);
                        let y = x87::load(h.read(at(pen, 8)));
                        h.write(at(pen, 8), x87::store(x87::add(step, y)));
                    }
                }
                let cur: u32 = h.read(at(win, 0x8c));
                let shown: u32 = h.read(at(win, 0x9c));
                let r = cdecl(h, HARD_CODE_TO_FIX_THE_WRONG_DATA, &[cur, shown]);
                let t: u32 = h.read(at(win, 0x8c));
                let b = h.read::<u8>(t);
                let code = (r & 0xffff_ff00) | u32::from(b);
                let group = cdecl(h, TO_GET_GROUP, &[code]);
                match group.wrapping_sub(1) {
                    0 => {
                        let x0: u32 = h.read(at(win, 0x48));
                        h.write(at(pen, 4), x0);
                        let y = x87::load(h.read(at(pen, 8)));
                        h.write(at(pen, 8), x87::store(x87::add(y, step)));
                        if h.read::<u16>(at(win, 0xe)) == 1 {
                            cdecl(
                                h,
                                TO_FIRST_INIT_MESSAGE_NORMAL,
                                &[win, t.wrapping_add(1), pen],
                            );
                        }
                    }
                    1 => {
                        let next: u8 = h.read(at(t, 1));
                        if b == 0x10 && (0x50..=0x57).contains(&next) {
                            cdecl(h, TO_DRAW_CHANGE_MACRO2, &[win, t, pen]);
                        } else if b == 9 && next > 0xbf {
                            let lim = i32_at(h, 0xa0);
                            if lim != i32_at(h, 0xa4) && i32_at(h, 0x9c).wrapping_add(1) == lim {
                                cdecl(h, TO_SET_DRAW_INFO, &[t, win, pen]);
                            }
                            let n: u32 = h.read(at(win, 0x9c));
                            h.write(at(win, 0x9c), n.wrapping_add(1));
                        } else if b == 9 && i32_at(h, 0x9c) < i32_at(h, 0xa4) {
                        } else {
                            cdecl(h, TO_SET_DRAW_INFO, &[t, win, pen]);
                        }
                    }
                    2 => {
                        if h.read::<u16>(at(win, 0x10)) == 0x4000 {
                            h.write(at(pen, 0x1b), 0x56u8);
                        }
                        cdecl(h, TO_DRAW_CHANGE_MACRO2, &[win, t, pen]);
                    }
                    3 => {
                        let n: u32 = h.read(at(win, 0x9c));
                        h.write(at(win, 0x9c), n.wrapping_add(1));
                        if i32_at(h, 0xb4) == 0 && i32_at(h, 0xac) == 0 {
                            let lim = i32_at(h, 0xa0);
                            if lim < i32_at(h, 0xa8) {
                                h.write(at(win, 0xa0), lim.wrapping_add(1));
                            }
                        }
                        let src = if h.read::<u16>(at(win, 0x10)) == 0x4000 {
                            match cdecl(h, TO_CONV_ALBHED_WORD, &[t, word]) {
                                0 | 1 => h.write(at(pen, 0x1b), 0x60u8),
                                2 => h.write(at(pen, 0x1b), 0x56u8),
                                _ => {}
                            }
                            word
                        } else {
                            t
                        };
                        cdecl(h, TO_GET_FONT_INFO, &[src, info]);
                        cdecl(h, TO_MAKE_PKT_FONT_1612, &[pen, info]);
                    }
                    _ => {}
                }
                let cur: u32 = h.read(at(win, 0x8c));
                let next = cdecl(h, TO_SKIP_CODE, &[cur]);
                h.write(at(win, 0x8c), next);
                if i32_at(h, 0x9c) >= i32_at(h, 0xa0) {
                    break;
                }
            }
        }
        let lim: u32 = h.read(at(win, 0xa0));
        h.write(at(win, 0xa4), lim);
        let p: [u32; 7] = h.read(pen);
        h.write(at(win, 0xd4), p);
        0
    }

    pub const TO_MENU_GET_CONTROL_PAD: u32 = 0x008b_e410;
    pub const TO_MENU_GET_CONTROL_PAD_REP: u32 = 0x008b_e470;
    pub const TO_MENU_GET_CONTROL_PAD_TRG: u32 = 0x008b_e4b0;

    /// `TkMenuCtrlAskYesNoWindow(win)` (`0x008e3730`): the yes/no prompt's
    /// step by state `+0x28` (the pad read first; repeat and trigger
    /// buttons as `u16`): 0 resets (`+0x45` 0, choice `+0x48` 1) and waits
    /// for the text (`+0x58`: state 1, else 2 or 3 by `+0x90 == 1`); 1 a
    /// trigger `0x20` accepts (sound 1; `+0x45` 1, result `+0x2c` the
    /// choice, state 5), `0x40` cancels (sound 4; `+0x45` 1, result and
    /// choice 1, state 7), up/down (repeat `0x5000`) flips the choice
    /// (sound 1); 2 waits for any trigger (sound 1 when `+0x37`), then
    /// state 4; 3 goes to 4; 4 accepts as above; 6 cancels as above.
    /// Returns `eax` as the original leaves it.
    pub fn ask_yes_no(h: &mut impl Host, win: u32) -> u32 {
        cdecl(h, TO_MENU_GET_CONTROL_PAD, &[]);
        let rep = cdecl(h, TO_MENU_GET_CONTROL_PAD_REP, &[]) & 0xffff;
        let trg = cdecl(h, TO_MENU_GET_CONTROL_PAD_TRG, &[]) & 0xffff;
        let mut state: u32 = h.read(at(win, 0x28));
        loop {
            match state {
                0 => {
                    h.write(at(win, 0x45), 0u8);
                    h.write(at(win, 0x48), 1u16);
                    if h.read::<u32>(at(win, 0x58)) != 0 {
                        h.write(at(win, 0x28), 1u32);
                        return 0;
                    }
                    let s = u32::from(h.read::<u32>(at(win, 0x90)) != 1) + 2;
                    h.write(at(win, 0x28), s);
                    return s;
                }
                1 => {
                    if trg & 0x20 != 0 {
                        cdecl(h, SND_SEP_PLAY_SIMPLE, &[0x8000_0001]);
                        state = 4;
                    } else if trg & 0x40 != 0 {
                        cdecl(h, SND_SEP_PLAY_SIMPLE, &[0x8000_0004]);
                        state = 6;
                    } else if rep & 0x5000 != 0 {
                        cdecl(h, SND_SEP_PLAY_SIMPLE, &[0x8000_0001]);
                        let v = u32::from(h.read::<u16>(at(win, 0x48)) == 0);
                        h.write(at(win, 0x48), v as u16);
                        return v;
                    } else {
                        return 1;
                    }
                }
                2 => {
                    if trg as u16 == 0 {
                        return 2;
                    }
                    let mut eax = 2;
                    if h.read::<u8>(at(win, 0x37)) != 0 {
                        eax = cdecl(h, SND_SEP_PLAY_SIMPLE, &[0x8000_0001]);
                    }
                    h.write(at(win, 0x28), 4u32);
                    return eax;
                }
                3 => {
                    h.write(at(win, 0x28), 4u32);
                    return 3;
                }
                4 => {
                    let c: u16 = h.read(at(win, 0x48));
                    h.write(at(win, 0x45), 1u8);
                    h.write(at(win, 0x2c), c);
                    h.write(at(win, 0x28), 5u32);
                    state = 5;
                }
                6 => {
                    h.write(at(win, 0x45), 1u8);
                    h.write(at(win, 0x2c), 1u16);
                    h.write(at(win, 0x48), 1u16);
                    h.write(at(win, 0x28), 7u32);
                    state = 7;
                }
                _ => return state,
            }
        }
    }

    pub const TK_MENU_DRAW_1612_WIDTH_HEIGHT: u32 = 0x008d_c9f0;
    pub const TK_MENU_DRAW_WINDOW_1612: u32 = 0x008d_cf40;
    pub const TK_MENU_DRAW_EASY_MES_1612_WITH_MACRO: u32 = 0x008d_cc70;
    pub const TK_MENU_DRAW_EASY_MES_1612: u32 = 0x008d_cb80;
    pub const TK_MENU_DRAW_SP30: u32 = 0x008d_ca20;

    /// `TkMenuDrawAskYesNoWindow_008e3bc0(win)` (name ours; its address is
    /// taken next to `TkMenuCtrlAskYesNoWindow`'s): the prompt's window
    /// centred on the screen (`0x1dc` wide, `0xc6` high, `+0x60` lower)
    /// around its text `+0x24` (`TkMenuDraw1612WidthHeight`: width low,
    /// height high half) and, with choices (`+0x58`, `+0x5c`), the wider
    /// of the two plus 0x26; `TkMenuDrawWindow1612`, and unless it is
    /// clipped the text and the two choices centred below, the cursor
    /// (`TkMenuDrawSp30`) at the chosen one (`+0x48` 0 the first). Ends
    /// with the scissor cleared; returns its `eax`.
    pub fn ask_yes_no_draw(h: &mut impl Host, win: u32) -> u32 {
        let text: u32 = h.read(at(win, 0x24));
        let r = cdecl(h, TK_MENU_DRAW_1612_WIDTH_HEIGHT, &[text, 0]);
        let tw = (r & 0xffff) as i32;
        let th = ((r as i32) >> 16) & 0xffff;
        let mut hh = th;
        let mut cw = 0i32;
        let c1: u32 = h.read(at(win, 0x58));
        if c1 != 0 {
            hh = th + 0x26;
            let a = (cdecl(h, TK_MENU_DRAW_1612_WIDTH_HEIGHT, &[c1, 0]) & 0xffff) as i32;
            let c2: u32 = h.read(at(win, 0x5c));
            let b = (cdecl(h, TK_MENU_DRAW_1612_WIDTH_HEIGHT, &[c2, 0]) & 0xffff) as i32;
            cw = if a > b { a } else { b };
        }
        let x = (0x1dc - tw) / 2;
        let below: i32 = h.read(at(win, 0x60));
        let y = ((0xc6 - hh) / 2).wrapping_add(below);
        let args = [x as u32, y as u32, (tw + 0x24) as u32, (hh + 0xa) as u32];
        if cdecl(h, TK_MENU_DRAW_WINDOW_1612, &args) != 0 {
            let text: u32 = h.read(at(win, 0x24));
            let ty = y.wrapping_add(5);
            let args = [text, 0, (x + 0x12) as u32, ty as u32];
            cdecl(h, TK_MENU_DRAW_EASY_MES_1612_WITH_MACRO, &args);
            let c1: u32 = h.read(at(win, 0x58));
            if c1 != 0 {
                let cx = ((0x1dc - cw) / 2) as u32;
                let mut cy = ty.wrapping_add(th + 8) as u32;
                cdecl(h, TK_MENU_DRAW_EASY_MES_1612, &[c1, cx, cy]);
                if h.read::<u16>(at(win, 0x48)) == 0 {
                    cdecl(h, TK_MENU_DRAW_SP30, &[cx, cy, cw as u32]);
                }
                cy = cy.wrapping_add(0xf);
                let c2: u32 = h.read(at(win, 0x5c));
                cdecl(h, TK_MENU_DRAW_EASY_MES_1612, &[c2, cx, cy]);
                if h.read::<u16>(at(win, 0x48)) != 0 {
                    cdecl(h, TK_MENU_DRAW_SP30, &[cx, cy, cw as u32]);
                }
            }
        }
        cdecl(h, TK_MENU_DRAW_SCISSOR_CLEAR, &[])
    }

    pub const GRAPHIC_DRAW_UI_ELEMENT: u32 = 0x0063_eee0;
    /// The element `0x008e5f60` draws (`0x00c5b440`).
    pub const UI_ELEMENT_00C5B440: u32 = 0x00c5_b440;

    /// `MenuDrawUiElement_008e5f60()` (name ours; table entry, data
    /// `0x00c65a9c`; frame at `ebp`, the quad at `ebp - 0x9c`): a quad at
    /// the origin (x, y 0; `0 * (1/512)`, `0 / 416` as the x87 leaves
    /// them), colours 0x80, size from `.rdata` (512 x 416 as `f32`),
    /// scale 1, 1; `graphicDrawUIElement(quad, 0x00c5b440, 1, 0, 0)`.
    /// The callee reads the quad's vertices 2 and 3 (`ebp - 0x5c ..
    /// ebp - 0x20`), which the original never writes: the port leaves
    /// them 0. Returns its `eax`.
    pub fn draw_ui_element_008e5f60(h: &mut impl Host, ebp: u32) -> u32 {
        use ffx_bridge::x87;
        let q = ebp.wrapping_sub(0x9c);
        let a: f64 = h.read(h.rebase(0x00b4_1cc0));
        let b: f64 = h.read(h.rebase(0x00b4_1d30));
        let u = x87::store(x87::mul(a, 0.0));
        let v = x87::store(x87::div(0.0, b));
        h.write(q, [0u32, 0, u, v, 0x80, 0x80, 0x80, 0x80]);
        let w: u32 = h.read(h.rebase(0x00b4_1d9c));
        let hh: u32 = h.read(h.rebase(0x00b4_1d98));
        h.write(at(q, 0x20), [w, hh, 0x3f80_0000, 0x3f80_0000]);
        h.write(at(q, 0x30), [0x80u32, 0x80, 0x80, 0x80]);
        h.write(ebp.wrapping_sub(0xc), [0u32, 0]);
        // junk read: [ebp-0x5c..ebp-0x10] never written; 0 (brief 2026-10-24)
        let e = h.rebase(UI_ELEMENT_00C5B440);
        cdecl(h, GRAPHIC_DRAW_UI_ELEMENT, &[q, e, 1, 0, 0])
    }
}
