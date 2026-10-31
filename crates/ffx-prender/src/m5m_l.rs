//! M5 stage 1, list `map`, round 5 batch 8: a map's id from the map id
//! table (`rcuiMapIdFromCsv_009377e0`, `mapid.csv`), the lookup
//! `rcuiMakeFullPath` uses.
//!
//! The chunk and parser live in the original's frame and come in through
//! its `ebp` (the shim passes a `StackFrame`).

/// The frame: `ebp` minus its lowest local.
pub const FRAME: u32 = 0x98;
/// `host:/ffx/proj/map/master/info/mapid.csv`, the separators.
pub const CSV_PATH: u32 = 0x00b6_8698;
pub const SEPARATORS: u32 = 0x00b8_f884;
/// `MSVCR110!isdigit`, `MSVCR110!strncmp` (the slots).
pub const ISDIGIT_SLOT: u32 = 0x00b0_c498;
pub const STRNCMP_SLOT: u32 = 0x00b0_c3e4;

// ---- callees ----

/// `rcdtChunkInit(c)`, `rcdtChunkLoadFromFile(c, path)` -> 0 on failure,
/// `rcdtChunkFini(c)`, `rcdtParseInit(p)`, `rcdtParseSet(p, c)`,
/// `rcdtParseSetMode(p, mode)`, `rcdtParseAddSeparators(p, chars)`,
/// `rcdtParseAdvanceLine(p)`, `rcdtParseIsEnd(p)`, `rcdtParseGetInt(p)`,
/// `rcdtParseFini(p)` (cdecl).
pub const CHUNK_INIT: u32 = 0x0092_b840;
pub const CHUNK_LOAD: u32 = 0x0092_b880;
pub const CHUNK_FINI: u32 = 0x0092_b7f0;
pub const PARSE_INIT: u32 = 0x0092_bf50;
pub const PARSE_SET: u32 = 0x0092_c0a0;
pub const PARSE_SET_MODE: u32 = 0x0092_c0c0;
pub const PARSE_ADD_SEPARATORS: u32 = 0x0092_c0f0;
pub const PARSE_ADVANCE_LINE: u32 = 0x0092_b9b0;
pub const PARSE_IS_END: u32 = 0x0092_bfb0;
pub const PARSE_GET_INT: u32 = 0x0092_bc50;
pub const PARSE_FINI: u32 = 0x0092_ba60;

pub mod bridge {
    use super::{
        CHUNK_FINI, CHUNK_INIT, CHUNK_LOAD, CSV_PATH, ISDIGIT_SLOT, PARSE_ADD_SEPARATORS,
        PARSE_ADVANCE_LINE, PARSE_FINI, PARSE_GET_INT, PARSE_INIT, PARSE_IS_END, PARSE_SET,
        PARSE_SET_MODE, SEPARATORS, STRNCMP_SLOT,
    };
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, Conv::Cdecl, args)
    }

    /// Whether the parser's cursor (`+4`) is on a digit.
    fn on_digit(h: &mut impl Host, parse: u32) -> bool {
        let cursor: u32 = h.read(parse + 4);
        let c = u32::from(h.read::<u8>(cursor));
        import(h, ISDIGIT_SLOT, &[c]) != 0
    }

    /// `rcuiMapIdFromCsv_009377e0(name)` -> the id on the line whose
    /// token after the id matches `name`'s first 6 characters, or -1 (no
    /// table, or no such line): the table's lines up to the first one
    /// starting with a digit are skipped (as the original, without an end
    /// check), then each line's int is read and the cursor compared.
    pub fn map_id_from_csv(h: &mut impl Host, ebp: u32, name: u32) -> u32 {
        let chunk = ebp - 0x98;
        let parse = ebp - 0x90;
        let mut id = u32::MAX;
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
            while cdecl(h, PARSE_IS_END, &[parse]) == 0 {
                let value = cdecl(h, PARSE_GET_INT, &[parse]);
                let cursor: u32 = h.read(parse + 4);
                if import(h, STRNCMP_SLOT, &[name, cursor, 6]) == 0 {
                    id = value;
                    break;
                }
                cdecl(h, PARSE_ADVANCE_LINE, &[parse]);
            }
            cdecl(h, PARSE_FINI, &[parse]);
        }
        cdecl(h, CHUNK_FINI, &[chunk]);
        id
    }
}
