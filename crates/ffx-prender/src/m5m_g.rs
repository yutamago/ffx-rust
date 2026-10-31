//! M5 stage 1, list `map`, round 5 batch 3: the debug map tool's scene
//! text config: the per-object keyword loop and the config file's reader
//! (`rcbgSceneLoadTextSubConfig`).
//!
//! Callees stay calls to the originals (cdecl). The parser and chunk the
//! reader keeps in its frame come in as addresses (the shim passes a
//! `StackFrame`).

/// The keyword tables (`0x00c60e58` object keys, `0x00c60e38` the file's
/// sections).
pub const OBJECT_KEYS: u32 = 0x00c6_0e58;
pub const SECTION_KEYS: u32 = 0x00c6_0e38;

// ---- callees ----

/// `rcdtParseNextToken(p)`, `rcdtParseIsEnd(p)`, `rcdtParseGetKeyword(p,
/// table)`, `rcdtParseGetInt(p)`, `rcdtParseGetFloat(p)` (ST0) (cdecl).
pub const NEXT_TOKEN: u32 = 0x0092_ba30;
pub const IS_END: u32 = 0x0092_bfb0;
pub const GET_KEYWORD: u32 = 0x0092_be40;
pub const GET_INT: u32 = 0x0092_bc50;
pub const GET_FLOAT: u32 = 0x0092_bbb0;
/// `rcbgAnimeLoadConfig(anime, p)`, `rcbgPalLoadConfig(pal, p)`,
/// `rcbgSceneLoadTextSubConfigKeys(scene, p)` (cdecl).
pub const ANIME_LOAD_CONFIG: u32 = 0x0093_a650;
pub const PAL_LOAD_CONFIG: u32 = 0x0093_adf0;
pub const SCENE_CONFIG_KEYS: u32 = 0x0093_aee0;
/// `rcbgSceneLoadTextObjConfig_0093ab80(obj, p)` (this batch).
pub const OBJ_CONFIG: u32 = 0x0093_ab80;
/// `rcdtChunkInit(c)`, `rcdtParseInit(p)`, `rcdtParseSetMode(p, mode)`,
/// `rcdtChunkLoadFromFile(c, name)`, `rcdtParseSet(p, c)`,
/// `rcdtChunkFini(c)`, `rcdtParseFini(p)` (cdecl).
pub const CHUNK_INIT: u32 = 0x0092_b840;
pub const PARSE_INIT: u32 = 0x0092_bf50;
pub const PARSE_SET_MODE: u32 = 0x0092_c0c0;
pub const CHUNK_LOAD: u32 = 0x0092_b880;
pub const PARSE_SET: u32 = 0x0092_c0a0;
pub const CHUNK_FINI: u32 = 0x0092_b7f0;
pub const PARSE_FINI: u32 = 0x0092_ba60;

pub mod bridge {
    use super::{
        ANIME_LOAD_CONFIG, CHUNK_FINI, CHUNK_INIT, CHUNK_LOAD, GET_FLOAT, GET_INT, GET_KEYWORD,
        IS_END, NEXT_TOKEN, OBJ_CONFIG, OBJECT_KEYS, PAL_LOAD_CONFIG, PARSE_FINI, PARSE_INIT,
        PARSE_SET, PARSE_SET_MODE, SCENE_CONFIG_KEYS, SECTION_KEYS,
    };
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// `rcbgSceneLoadTextObjConfig_0093ab80(obj, p)` -> `rcdtParseIsEnd`'s
    /// `eax`, or 9 at the closing keyword: from the next token on, keywords
    /// (`0x00c60e58`) set the object's flags (`+0`, bit 1 cleared), `+0xc`,
    /// `+0x10`, four floats (`+0x14..+0x20`), `+0x2c`, `+0x30`, the
    /// sign-extended 64-bit `+0x38`, its animation (`+0x90`) or palette
    /// (`+0xb0`) config; others are skipped.
    pub fn obj_config(h: &mut impl Host, obj: u32, p: u32) -> u32 {
        cdecl(h, NEXT_TOKEN, &[p]);
        let end = cdecl(h, IS_END, &[p]);
        if end != 0 {
            return end;
        }
        let keys = h.rebase(OBJECT_KEYS);
        loop {
            match cdecl(h, GET_KEYWORD, &[p, keys]) {
                0 => {
                    let v = cdecl(h, GET_INT, &[p]);
                    h.write(obj, v & !2);
                }
                1 => {
                    let v = cdecl(h, GET_INT, &[p]);
                    h.write(obj + 0xc, v);
                }
                2 => {
                    let v = cdecl(h, GET_INT, &[p]);
                    h.write(obj + 0x10, v);
                }
                3 => {
                    for k in 0..4 {
                        let v = h.call_sym_st0(GET_FLOAT, Conv::Cdecl, &[p]);
                        h.write(obj + 0x14 + 4 * k, v.to_bits());
                    }
                }
                4 => {
                    let v = cdecl(h, GET_INT, &[p]);
                    h.write(obj + 0x2c, v);
                }
                5 => {
                    let v = cdecl(h, GET_INT, &[p]);
                    h.write(obj + 0x30, v);
                }
                6 => {
                    let v = cdecl(h, GET_INT, &[p]);
                    h.write(obj + 0x38, v);
                    h.write(obj + 0x3c, (v.cast_signed() >> 31).cast_unsigned());
                }
                7 => {
                    cdecl(h, ANIME_LOAD_CONFIG, &[obj + 0x90, p]);
                }
                8 => {
                    cdecl(h, PAL_LOAD_CONFIG, &[obj + 0xb0, p]);
                }
                9 => return 9,
                _ => {}
            }
            let end = cdecl(h, IS_END, &[p]);
            if end != 0 {
                return end;
            }
        }
    }

    /// `rcbgSceneLoadTextSubConfig(scene, name)` -> 1 when the file loads,
    /// else 0: the file read into a chunk (`frame`, the original's `ebp -
    /// 0x98`) and parsed in text mode (the parser at `frame + 8`): section
    /// 0 the scene's keys, section 1 the next of its objects (`+8`, 0xc0
    /// each, `+4` of them); the chunk and parser freed.
    pub fn load_text_sub_config(h: &mut impl Host, frame: u32, scene: u32, name: u32) -> u32 {
        let chunk = frame;
        let p = frame + 8;
        cdecl(h, CHUNK_INIT, &[chunk]);
        cdecl(h, PARSE_INIT, &[p]);
        cdecl(h, PARSE_SET_MODE, &[p, 1]);
        let mut result = 0;
        if cdecl(h, CHUNK_LOAD, &[chunk, name]) != 0 {
            cdecl(h, PARSE_SET, &[p, chunk]);
            let mut n = 0_u32;
            let mut off = 0_u32;
            let keys = h.rebase(SECTION_KEYS);
            while cdecl(h, IS_END, &[p]) == 0 {
                match cdecl(h, GET_KEYWORD, &[p, keys]) {
                    0 => {
                        cdecl(h, SCENE_CONFIG_KEYS, &[scene, p]);
                    }
                    1 if n.cast_signed() < h.read::<u32>(scene + 4).cast_signed() => {
                        let objs: u32 = h.read(scene + 8);
                        cdecl(h, OBJ_CONFIG, &[objs.wrapping_add(off), p]);
                        n += 1;
                        off += 0xc0;
                    }
                    _ => {}
                }
            }
            result = 1;
        }
        cdecl(h, CHUNK_FINI, &[chunk]);
        cdecl(h, PARSE_FINI, &[p]);
        result
    }
}
