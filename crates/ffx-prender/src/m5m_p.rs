//! M5 stage 1, list `map`, round 5 batch 12: a numbered TIM2 series into
//! a texture transfer (`rcbgTrxTexLoadFromFile`): each
//! `host:<dir>/<name><nn>.tm2` copied into its entry of the transfer
//! (`+0x20`, PSMT8) and its CLUT into the palette transfer (`+0x60`).
//!
//! The TIM2 record, the name format and the path live in the original's
//! frame and come in through its `ebp` (the shim passes a `StackFrame`).

/// The frame: `ebp` minus its lowest local.
pub const FRAME: u32 = 0x25c;
/// `host:%s/%s%%0%dd.tm2`, `Error in %s, line %d: `, the source file's
/// name, `File not found.`.
pub const NAME_FORMAT: u32 = 0x00b6_8e0c;
pub const ERROR_FORMAT: u32 = 0x00b6_7878;
pub const SOURCE_FILE: u32 = 0x00b6_8be8;
pub const NOT_FOUND_TEXT: u32 = 0x00b6_8ca4;
/// `MSVCR110!sprintf` (the slot).
pub const SPRINTF_SLOT: u32 = 0x00b0_c3dc;

// ---- callees ----

/// `rcbgCountNumberedFiles_0093acb0(dir, name, digits)` -> the count,
/// `rcdtTm2Init(t)`, `rcdtTm2Load(t, path)` -> 0 when not found,
/// `rcbgTrxAllocEntries(t, format, w, h, n)`, `rcbgTrxGetEntryData(t, n,
/// ..)`, `rcdtTm2CopyRect(t, dst, x, y, w, h)`, `get_ptr32_func(p)`,
/// `rcdtTm2Fini(t)`, `rcPrint(fmt, ...)` (cdecl).
pub const COUNT_FILES: u32 = 0x0093_acb0;
pub const TM2_INIT: u32 = 0x0092_37e0;
pub const TM2_LOAD: u32 = 0x0092_3800;
pub const TRX_ALLOC_ENTRIES: u32 = 0x0092_d3e0;
pub const TRX_GET_ENTRY_DATA: u32 = 0x0092_d4f0;
pub const TM2_COPY_RECT: u32 = 0x0092_3ae0;
pub const GET_PTR32: u32 = 0x00a4_4650;
pub const TM2_FINI: u32 = 0x0092_3470;
pub const RC_PRINT: u32 = 0x0092_7550;

pub mod bridge {
    use super::{
        COUNT_FILES, ERROR_FORMAT, GET_PTR32, NAME_FORMAT, NOT_FOUND_TEXT, RC_PRINT, SOURCE_FILE,
        SPRINTF_SLOT, TM2_COPY_RECT, TM2_FINI, TM2_INIT, TM2_LOAD, TRX_ALLOC_ENTRIES,
        TRX_GET_ENTRY_DATA,
    };
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// `rcbgTrxTexLoadFromFile(t, dir, name, digits)` -> `rcdtTm2Fini`'s
    /// `eax`: for each of the `rcbgCountNumberedFiles` files (`%0<digits>d`
    /// numbered) the TIM2 loaded (missing: an error printed and skipped);
    /// the first one loaded sizes the transfers (`+0x20` PSMT8 0x13 at the
    /// image's size, `+0x60` 16 x 16 CLUTs, one entry per file; `+0x10`/
    /// `+0x12` the size); each gets its pixels and 256 CLUT words. The
    /// sizing pushes the TIM2's size words whole (`+0x24` w | h << 16,
    /// `+0x26` h | the next half).
    pub fn tex_load_from_file(
        h: &mut impl Host,
        ebp: u32,
        t: u32,
        dir: u32,
        name: u32,
        digits: u32,
    ) -> u32 {
        h.write(ebp - 0x25c, t);
        let n = cdecl(h, COUNT_FILES, &[dir, name, digits]);
        h.write(ebp - 0x258, n);
        let tm2 = ebp - 0x254;
        cdecl(h, TM2_INIT, &[tm2]);
        let sprintf: u32 = h.read(h.rebase(SPRINTF_SLOT));
        let format = ebp - 0x204;
        let fmt = h.rebase(NAME_FORMAT);
        h.call(sprintf, Conv::Cdecl, &[format, fmt, dir, name, digits]);
        let path = ebp - 0x104;
        let mut i = 0_u32;
        while i.cast_signed() < n.cast_signed() {
            let sprintf: u32 = h.read(h.rebase(SPRINTF_SLOT));
            h.call(sprintf, Conv::Cdecl, &[path, format, i]);
            if cdecl(h, TM2_LOAD, &[tm2, path]) == 0 {
                let (fmt, file) = (h.rebase(ERROR_FORMAT), h.rebase(SOURCE_FILE));
                cdecl(h, RC_PRINT, &[fmt, file, 0x616]);
                let text = h.rebase(NOT_FOUND_TEXT);
                cdecl(h, RC_PRINT, &[text]);
            } else {
                let t: u32 = h.read(ebp - 0x25c);
                if i == 0 {
                    let wh: u32 = h.read(tm2 + 0x24);
                    let hx: u32 = h.read(tm2 + 0x26);
                    cdecl(h, TRX_ALLOC_ENTRIES, &[t + 0x20, 0x13, wh, hx, n]);
                    cdecl(h, TRX_ALLOC_ENTRIES, &[t + 0x60, 0, 0x10, 0x10, n]);
                    let w: u16 = h.read(tm2 + 0x24);
                    let hh: u16 = h.read(tm2 + 0x26);
                    h.write(t + 0x10, w);
                    h.write(t + 0x12, hh);
                }
                let w = u32::from(h.read::<u16>(tm2 + 0x24));
                let hh = u32::from(h.read::<u16>(tm2 + 0x26));
                let data = cdecl(h, TRX_GET_ENTRY_DATA, &[t + 0x20, i, 0, 0, w, hh]);
                // the original leaves the rectangle on the stack: the copy
                // gets it
                cdecl(h, TM2_COPY_RECT, &[tm2, data, 0, 0, w, hh]);
                let clut_at: u32 = h.read(tm2 + 0x44);
                let src = cdecl(h, GET_PTR32, &[clut_at]);
                let dst = cdecl(h, TRX_GET_ENTRY_DATA, &[t + 0x60, i]);
                for k in 0..0x100 {
                    let v: u32 = h.read(src + 4 * k);
                    h.write(dst + 4 * k, v);
                }
            }
            i += 1;
        }
        cdecl(h, TM2_FINI, &[tm2])
    }
}
