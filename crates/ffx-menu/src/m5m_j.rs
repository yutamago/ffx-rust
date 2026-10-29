//! M5 stage 1, the menu, battle and script leftovers (round 3), batch 3:
//! the boot's character table loader (`MallocAndReadGamecode`) and the
//! customize screen's fade band.

#![allow(clippy::cast_precision_loss, clippy::many_single_char_names)]

pub mod bridge {
    use ffx_bridge::{Global, x87};
    use ffx_graphic::host::{Conv, Host};

    pub const SCE_OPEN: u32 = 0x0062_fa30;
    pub const SCE_LSEEK: u32 = 0x0062_f8e0;
    pub const SCE_READ: u32 = 0x0062_fc00;
    pub const SCE_CLOSE: u32 = 0x0062_f610;
    pub const DBG_PRINTF: u32 = 0x0062_f500;
    pub const TK_MALLOC: u32 = 0x0088_e650;
    pub const TK_FREE: u32 = 0x0088_d520;
    /// `msvcrt!sprintf`'s import slot (cdecl).
    pub const SPRINTF_IAT: u32 = 0x00b0_c3dc;

    /// `g_CharsetCjk`, `g_CharsetWestern`.
    pub const CHARSET_CJK: Global<u32> = Global::new(0x00cc_b95c);
    pub const CHARSET_WESTERN: Global<u32> = Global::new(0x00cc_b960);

    /// `"host0:/ffx/master/jppc/ffx_encoding"` and the four file name
    /// formats (`.rdata`), and the missing-file message.
    pub const ENCODING_DIR: u32 = 0x00b4_1db8;
    pub const NAME_JP: u32 = 0x00b4_1ddc;
    pub const NAME_KR: u32 = 0x00b4_1e0c;
    pub const NAME_CH: u32 = 0x00b4_1e24;
    pub const NAME_US: u32 = 0x00b4_1df4;
    pub const MISSING: u32 = 0x00b4_1e3c;

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// The language's table: Japanese (0), Korean (9) and Chinese (10)
    /// load into `g_CharsetCjk`, every other value (the original's two
    /// jump tables) the US table into `g_CharsetWestern`.
    fn table(lang: u32) -> (u32, bool) {
        match lang {
            0 => (NAME_JP, true),
            9 => (NAME_KR, true),
            10 => (NAME_CH, true),
            _ => (NAME_US, false),
        }
    }

    /// `MallocAndReadGamecode(lang)` (`0x00646920`, frame at `ebp`): the
    /// name `sprintf(ebp - 0x84, "%s/ffxsjistbl_<lang>.bin", dir)`,
    /// `sceOpen(name, 1)` (the descriptor kept at `ebp - 0x88`); a 0
    /// descriptor prints the missing-file message and returns 0. Else the
    /// size (`sceLseek(fd, 0, 2)`, rounded up to 16, signed) is read into
    /// `TkMalloc(size + 1)` (first word cleared, a 0 byte after the data)
    /// and replaces the table (the old one `TkFree`d); returns 1.
    pub fn malloc_and_read_gamecode(h: &mut impl Host, ebp: u32, lang: u32) -> u32 {
        let (fmt, cjk) = table(lang);
        let name = ebp.wrapping_sub(0x84);
        let fmt = h.rebase(fmt);
        let dir = h.rebase(ENCODING_DIR);
        let sprintf: u32 = h.read(h.rebase(SPRINTF_IAT));
        h.call(sprintf, Conv::Cdecl, &[name, fmt, dir]);
        let fd = cdecl(h, SCE_OPEN, &[name, 1]);
        h.write(ebp.wrapping_sub(0x88), fd);
        if fd == 0 {
            let msg = h.rebase(MISSING);
            cdecl(h, DBG_PRINTF, &[msg, name]);
            return 0;
        }
        let end = cdecl(h, SCE_LSEEK, &[fd, 0, 2]).cast_signed();
        let t = end.wrapping_add(0xf);
        let size = t.wrapping_add((t >> 31) & 0xf).cast_unsigned() & 0xffff_fff0;
        cdecl(h, SCE_LSEEK, &[fd, 0, 0]);
        let data = cdecl(h, TK_MALLOC, &[size.wrapping_add(1)]);
        h.write(data, 0u32);
        cdecl(h, SCE_READ, &[fd, data, size]);
        cdecl(h, SCE_CLOSE, &[fd]);
        h.write(data.wrapping_add(size), 0u8);
        let slot = if cjk { CHARSET_CJK } else { CHARSET_WESTERN };
        let old = slot.get(h);
        if old != 0 {
            cdecl(h, TK_FREE, &[old]);
        }
        slot.set(h, data);
        1
    }

    pub const TO_MKP_SHAPE_XYWHUVC2: u32 = 0x0090_3ee0;
    /// The customize screen's fade line (`i16`).
    pub const KAIZOU_FADE_Y: Global<i16> = Global::new(0x0186_aa42);
    /// `f32` 1024, 1920, 512 and `f64` 1024, 416 (`.rdata`).
    pub const SHAPE_V1: u32 = 0x00b5_5178;
    pub const SHAPE_U1: u32 = 0x00b4_a1ac;
    pub const SHAPE_H: u32 = 0x00b4_1d9c;
    pub const SCALE_MUL: u32 = 0x00b5_1ba8;
    pub const SCALE_DIV: u32 = 0x00b4_1d30;

    /// An `f32` constant as `fld`/`fstp` passes it on.
    fn f32_word(h: &impl Host, sym: u32) -> u32 {
        let v: u32 = h.read(h.rebase(sym));
        x87::store(x87::load(v))
    }

    /// `FUN_008d6340(c)` (the customize screen's fade band): with `a =
    /// (c & !0x1f) << 19` nonzero, `y = i16 [0x0186aa42]` and colour `a |
    /// 0x808080`, `TOMkpShapeXYWHUVC2(-2, 0.0, f32(y), f32 512, f32(0x1a0 -
    /// y), 0.0, f32(y * 1024.0 / 416.0), f32 1920, f32 1024, colour,
    /// colour)` (the constants read from `.rdata`); returns its `eax`, or
    /// 0.
    pub fn kaizou_fade_band(h: &mut impl Host, c: u32) -> u32 {
        let a = (c & 0xffff_ffe0) << 19;
        if a == 0 {
            return 0;
        }
        let y = i32::from(KAIZOU_FADE_Y.get(h));
        let colour = a | 0x0080_8080;
        let mul: f64 = h.read(h.rebase(SCALE_MUL));
        let div: f64 = h.read(h.rebase(SCALE_DIV));
        let v = x87::store(x87::div(x87::mul(f64::from(y), mul), div));
        let zero = x87::store(0.0);
        let int = |v: i32| x87::store(f64::from(v as f32));
        let args = [
            0xffff_fffe,
            zero,
            int(y),
            f32_word(h, SHAPE_H),
            int(0x1a0 - y),
            zero,
            v,
            f32_word(h, SHAPE_U1),
            f32_word(h, SHAPE_V1),
            colour,
            colour,
        ];
        h.call_sym_wide(TO_MKP_SHAPE_XYWHUVC2, &args)
    }
}
