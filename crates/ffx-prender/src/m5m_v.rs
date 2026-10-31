//! M5 stage 1, list `map`, round 5 batch 18: the guide map conversion of
//! every map in the map id table (`rcuiGuideProcConvertAll`).
//!
//! The chunk and parser live in the original's frame and come in through
//! its `ebp` (the shim passes a `StackFrame`).

pub use crate::m5m_l::{
    CHUNK_FINI, CHUNK_INIT, CHUNK_LOAD, CSV_PATH, ISDIGIT_SLOT, PARSE_ADD_SEPARATORS,
    PARSE_ADVANCE_LINE, PARSE_FINI, PARSE_GET_INT, PARSE_INIT, PARSE_IS_END, PARSE_SET,
    PARSE_SET_MODE, SEPARATORS,
};

/// The frame: `ebp` minus its lowest local.
pub const FRAME: u32 = 0x98;
/// `---- Guide Map Data Convert START` / `END`.
pub const START_TEXT: u32 = 0x00b6_86c4;
pub const END_TEXT: u32 = 0x00b6_86e8;

// ---- callees ----

/// `0x00938590(id)` (a map's guide map conversion), `rcPrint(fmt, ...)`
/// (cdecl).
pub const GUIDE_MAP_CONV: u32 = 0x0093_8590;
pub const RC_PRINT: u32 = 0x0092_7550;

pub mod bridge {
    use super::{
        CHUNK_FINI, CHUNK_INIT, CHUNK_LOAD, CSV_PATH, END_TEXT, GUIDE_MAP_CONV, ISDIGIT_SLOT,
        PARSE_ADD_SEPARATORS, PARSE_ADVANCE_LINE, PARSE_FINI, PARSE_GET_INT, PARSE_INIT,
        PARSE_IS_END, PARSE_SET, PARSE_SET_MODE, RC_PRINT, SEPARATORS, START_TEXT,
    };
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// Whether the parser's cursor (`+4`) is on a digit.
    fn on_digit(h: &mut impl Host, parse: u32) -> bool {
        let cursor: u32 = h.read(parse + 4);
        let c = u32::from(h.read::<u8>(cursor));
        let f: u32 = h.read(h.rebase(ISDIGIT_SLOT));
        h.call(f, Conv::Cdecl, &[c]) != 0
    }

    /// `rcuiGuideProcConvertAll()` -> `rcdtChunkFini`'s `eax`: the map id
    /// table (`mapid.csv`, parse mode 1) past its lines before the first
    /// starting with a digit (no end check, as the original), each line's
    /// id converted (`0x00938590`) between a start and an end line
    /// printed.
    pub fn convert_all(h: &mut impl Host, ebp: u32) -> u32 {
        let (chunk, parse) = (ebp - 0x98, ebp - 0x90);
        cdecl(h, CHUNK_INIT, &[chunk]);
        let path = h.rebase(CSV_PATH);
        if cdecl(h, CHUNK_LOAD, &[chunk, path]) != 0 {
            cdecl(h, PARSE_INIT, &[parse]);
            cdecl(h, PARSE_SET, &[parse, chunk]);
            cdecl(h, PARSE_SET_MODE, &[parse, 1]);
            let separators = h.rebase(SEPARATORS);
            cdecl(h, PARSE_ADD_SEPARATORS, &[parse, separators]);
            while !on_digit(h, parse) {
                cdecl(h, PARSE_ADVANCE_LINE, &[parse]);
            }
            let text = h.rebase(START_TEXT);
            cdecl(h, RC_PRINT, &[text]);
            while cdecl(h, PARSE_IS_END, &[parse]) == 0 {
                let id = cdecl(h, PARSE_GET_INT, &[parse]);
                cdecl(h, GUIDE_MAP_CONV, &[id]);
                cdecl(h, PARSE_ADVANCE_LINE, &[parse]);
            }
            let text = h.rebase(END_TEXT);
            cdecl(h, RC_PRINT, &[text]);
            cdecl(h, PARSE_FINI, &[parse]);
        }
        cdecl(h, CHUNK_FINI, &[chunk])
    }
}
