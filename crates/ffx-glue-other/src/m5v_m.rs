//! M5 stage 1, list `vfx`, round 5 batch 8: the opus file command, a
//! debug dump of an effect character's texture images and palettes.

/// `g_EtChr` (0x20-byte records).
pub const ET_CHR: u32 = 0x012a_22c0;
/// `opu_file`'s texts: the start, the image count (wrong / right), an
/// image, the image path, the palette count (wrong / right), a palette,
/// the palette path.
pub const START_TEXT: u32 = 0x00b5_8b58;
pub const IMAGE_COUNT_BAD: u32 = 0x00b5_8b94;
pub const IMAGE_COUNT_OK: u32 = 0x00b5_8bbc;
pub const IMAGE_TEXT: u32 = 0x00b5_8bd8;
pub const IMAGE_PATH: u32 = 0x00b5_8c04;
pub const CLUT_COUNT_BAD: u32 = 0x00b5_8c30;
pub const CLUT_COUNT_OK: u32 = 0x00b5_8c54;
pub const CLUT_TEXT: u32 = 0x00b5_8c6c;
pub const CLUT_PATH: u32 = 0x00b5_8c94;
/// `MSVCR110!sprintf` (the slot).
pub const SPRINTF_SLOT: u32 = 0x00b0_c3dc;

// ---- callees ----

/// `op_et_chr_index_by_no(no)`, `dbgPrintf(fmt, ...)`,
/// `Ch_TexGetNbImage(chr)`, `Ch_TexGetImage(chr, i, &w, &h)`,
/// `Ch_TexGetNbClut(chr)`, `Ch_TexGetClut(chr, i, &n)`, `sceOpen(name,
/// mode)`, `sceWrite(fd, p, n)`, `sceClose(fd)` (cdecl).
pub const CHR_INDEX_BY_NO: u32 = 0x007f_eaa0;
pub const DBG_PRINTF: u32 = 0x0062_f500;
pub const TEX_NB_IMAGE: u32 = 0x0083_cdb0;
pub const TEX_IMAGE: u32 = 0x0083_cd50;
pub const TEX_NB_CLUT: u32 = 0x0083_cd90;
pub const TEX_CLUT: u32 = 0x0083_cd20;
pub const SCE_OPEN: u32 = 0x0062_fa30;
pub const SCE_WRITE: u32 = 0x0062_fdc0;
pub const SCE_CLOSE: u32 = 0x0062_f610;

pub mod bridge {
    use super::{
        CHR_INDEX_BY_NO, CLUT_COUNT_BAD, CLUT_COUNT_OK, CLUT_PATH, CLUT_TEXT, DBG_PRINTF, ET_CHR,
        IMAGE_COUNT_BAD, IMAGE_COUNT_OK, IMAGE_PATH, IMAGE_TEXT, SCE_CLOSE, SCE_OPEN, SCE_WRITE,
        SPRINTF_SLOT, START_TEXT, TEX_CLUT, TEX_IMAGE, TEX_NB_CLUT, TEX_NB_IMAGE,
    };
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    fn print(h: &mut impl Host, text: u32, args: &[u32]) {
        let mut words = vec![h.rebase(text)];
        words.extend_from_slice(args);
        cdecl(h, DBG_PRINTF, &words);
    }

    /// Writes `n` bytes at `p` to the file named by `fmt` (number, index)
    /// in `path`.
    fn dump(h: &mut impl Host, path: u32, fmt: u32, no: u32, i: u32, p: u32, n: u32) {
        let f: u32 = h.read(h.rebase(SPRINTF_SLOT));
        let fmt = h.rebase(fmt);
        h.call(f, Conv::Cdecl, &[path, fmt, no, i]);
        let fd = cdecl(h, SCE_OPEN, &[path, 0x602]);
        if fd != 0 {
            cdecl(h, SCE_WRITE, &[fd, p, n]);
            cdecl(h, SCE_CLOSE, &[fd]);
        }
    }

    /// `opu_file(host, cmd)` -> `cmd + 10`, or `cmd` unless `i16 +2` is 0:
    /// the effect character numbered `i16 +4` (when it has a handle) has
    /// each texture image and palette written to a file (`frame`: the
    /// original's `ebp - 0x224`: the palette size at `+0x10`, the image
    /// size at `+4`/`+8`, the path at `+0x20`), the counts checked
    /// against `i16 +6` and `+8` in the log.
    pub fn opu_file(h: &mut impl Host, frame: u32, cmd: u32) -> u32 {
        if h.read::<u16>(cmd + 2) != 0 {
            return cmd;
        }
        let no = i32::from(h.read::<i16>(cmd + 4)).cast_unsigned();
        let index = cdecl(h, CHR_INDEX_BY_NO, &[no]);
        if index.cast_signed() < 0 {
            return cmd + 10;
        }
        let chr: u32 = h.read(h.rebase(ET_CHR) + (index << 5));
        if chr == 0 {
            return cmd + 10;
        }
        let cluts = i32::from(h.read::<i16>(cmd + 8)).cast_unsigned();
        let images = i32::from(h.read::<i16>(cmd + 6)).cast_unsigned();
        let (hh, w, count, path) = (frame + 4, frame + 8, frame + 0x10, frame + 0x20);
        print(h, START_TEXT, &[no]);
        let n = cdecl(h, TEX_NB_IMAGE, &[chr]);
        print(
            h,
            if n == images {
                IMAGE_COUNT_OK
            } else {
                IMAGE_COUNT_BAD
            },
            &[n],
        );
        let mut i = 0_u32;
        while i.cast_signed() < n.cast_signed() {
            let p = cdecl(h, TEX_IMAGE, &[chr, i, w, hh]);
            let size = h.read::<u32>(w).wrapping_mul(h.read(hh));
            print(h, IMAGE_TEXT, &[i, size]);
            let size = h.read::<u32>(w).wrapping_mul(h.read(hh));
            dump(h, path, IMAGE_PATH, no, i, p, size);
            i += 1;
        }
        let n = cdecl(h, TEX_NB_CLUT, &[chr]);
        print(
            h,
            if n == cluts {
                CLUT_COUNT_OK
            } else {
                CLUT_COUNT_BAD
            },
            &[n],
        );
        let mut i = 0_u32;
        while i.cast_signed() < n.cast_signed() {
            let p = cdecl(h, TEX_CLUT, &[chr, i, count]);
            let c: u32 = h.read(count);
            print(h, CLUT_TEXT, &[i, c]);
            let c: u32 = h.read(count);
            dump(h, path, CLUT_PATH, no, i, p, c.wrapping_mul(4));
            i += 1;
        }
        cmd + 10
    }
}
