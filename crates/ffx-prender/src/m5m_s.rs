//! M5 stage 1, list `map`, round 5 batch 15: the 2D pack check
//! (`rcbgSaveChkPackList`, `2dpack.txt` beside the map directory): the
//! pack (`[packNN]` sections) holding the current map, and the maps of
//! its area (the same first 4 characters) as 2-digit numbers.
//!
//! The chunk, parser and path live in the original's frame and come in
//! through its `ebp` (the shim passes a `StackFrame`).

pub use crate::m5m_e::RCUI_WORK;

/// The frame: `ebp` minus its lowest local.
pub const FRAME: u32 = 0x1a4;
/// `host:%s`, `/2dpack.txt` (12 bytes with its NUL).
pub const HOST_FORMAT: u32 = 0x00b6_8960;
pub const PACK_FILE: u32 = 0x00b6_8968;
/// `MSVCR110!sprintf`, `strrchr`, `strncmp` (the slots).
pub const SPRINTF_SLOT: u32 = 0x00b0_c3dc;
pub const STRRCHR_SLOT: u32 = 0x00b0_c3a4;
pub const STRNCMP_SLOT: u32 = 0x00b0_c3e4;

// ---- callees ----

/// `rcdtChunkInit(c)`, `rcdtChunkLoadFromFile(c, path)` -> 0 on failure,
/// `rcdtChunkFini(c)`, `rcdtParseInit(p)`, `rcdtParseSet(p, c)`,
/// `rcdtParseIsEnd(p)`, `rcdtParseAdvanceLine(p)`, `rcdtParseFini(p)`
/// (cdecl).
pub const CHUNK_INIT: u32 = 0x0092_b840;
pub const CHUNK_LOAD: u32 = 0x0092_b880;
pub const CHUNK_FINI: u32 = 0x0092_b7f0;
pub const PARSE_INIT: u32 = 0x0092_bf50;
pub const PARSE_SET: u32 = 0x0092_c0a0;
pub const PARSE_IS_END: u32 = 0x0092_bfb0;
pub const PARSE_ADVANCE_LINE: u32 = 0x0092_b9b0;
pub const PARSE_FINI: u32 = 0x0092_ba60;

pub mod bridge {
    use super::{
        CHUNK_FINI, CHUNK_INIT, CHUNK_LOAD, HOST_FORMAT, PACK_FILE, PARSE_ADVANCE_LINE, PARSE_FINI,
        PARSE_INIT, PARSE_IS_END, PARSE_SET, RCUI_WORK, SPRINTF_SLOT, STRNCMP_SLOT, STRRCHR_SLOT,
    };
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, Conv::Cdecl, args)
    }

    /// `rcbgSaveChkPackList(maps, pack)` -> the number of maps, or 0 when
    /// the current map (the rcui work's `+0x27c`) is in no pack: the path
    /// is `host:` + the work's directory (`+0x174`) up to its last `/` +
    /// `/2dpack.txt`; each `[` line opens pack `[5] * 10 + [6]` (digits)
    /// and restarts the list, unless the map was found (then the scan
    /// ends); each line starting with the map's first 4 characters adds
    /// its `[4] * 10 + [5]` to `maps` (a byte each) and the full 6 mark the
    /// map found. Found: `*pack` = the pack.
    pub fn chk_pack_list(h: &mut impl Host, ebp: u32, maps: u32, pack: u32) -> u32 {
        h.write(ebp - 0x19c, maps);
        h.write(ebp - 0x198, pack);
        let mut id = u32::MAX;
        h.write(ebp - 0x194, id);
        let mut found = false;
        let mut n = 0_u32;
        let path = ebp - 0x104;
        let fmt = h.rebase(HOST_FORMAT);
        let dir = RCUI_WORK.get(h) + 0x174;
        import(h, SPRINTF_SLOT, &[path, fmt, dir]);
        let slash = import(h, STRRCHR_SLOT, &[path, u32::from(b'/')]);
        h.write(slash, 0_u8);
        let mut end = path;
        while h.read::<u8>(end) != 0 {
            end += 1;
        }
        let file = h.rebase(PACK_FILE);
        for k in 0..3 {
            let w: u32 = h.read(file + 4 * k);
            h.write(end + 4 * k, w);
        }
        let (chunk, parse) = (ebp - 0x1a4, ebp - 0x190);
        cdecl(h, CHUNK_INIT, &[chunk]);
        if cdecl(h, CHUNK_LOAD, &[chunk, path]) != 0 {
            cdecl(h, PARSE_INIT, &[parse]);
            cdecl(h, PARSE_SET, &[parse, chunk]);
            while cdecl(h, PARSE_IS_END, &[parse]) == 0 {
                let line: u32 = h.read(parse + 4);
                if h.read::<u8>(line) == b'[' {
                    if found {
                        break;
                    }
                    n = 0;
                    let tens = u32::from(h.read::<u8>(line + 5));
                    let ones = u32::from(h.read::<u8>(line + 6));
                    h.write(ebp - 0x194, (tens * 10 + ones).wrapping_sub(0x210));
                } else {
                    let map = RCUI_WORK.get(h) + 0x27c;
                    if import(h, STRNCMP_SLOT, &[map, line, 4]) == 0 {
                        let map = RCUI_WORK.get(h) + 0x27c;
                        let line: u32 = h.read(parse + 4);
                        if import(h, STRNCMP_SLOT, &[map, line, 6]) == 0 {
                            found = true;
                        }
                        let line: u32 = h.read(parse + 4);
                        let c4 = h.read::<u8>(line + 4).wrapping_sub(0x30);
                        let c5 = h.read::<u8>(line + 5);
                        let v = c4.wrapping_mul(10).wrapping_sub(0x30).wrapping_add(c5);
                        n += 1;
                        let maps: u32 = h.read(ebp - 0x19c);
                        h.write(maps.wrapping_add(n - 1), v);
                    }
                }
                cdecl(h, PARSE_ADVANCE_LINE, &[parse]);
            }
            id = h.read(ebp - 0x194);
            cdecl(h, PARSE_FINI, &[parse]);
        }
        cdecl(h, CHUNK_FINI, &[chunk]);
        if found {
            let pack: u32 = h.read(ebp - 0x198);
            h.write(pack, id);
            n
        } else {
            0
        }
    }
}
