//! M5 stage 1, list `map`, round 5 batch 11: a scene's text load
//! (`rcbgSceneLoadTextSub`): the scene config (`<map>.2d`, else
//! `<map>.txt`), then the camera text (`<map>cam.txt`, `camera.txt` or
//! `<map>`) parsed into the scene.
//!
//! The chunk, parser and path live in the original's frame and come in
//! through its `ebp` (the shim passes a `StackFrame`).

pub use crate::m5m_e::RCUI_WORK;

/// The frame: `ebp` minus its lowest local.
pub const FRAME: u32 = 0x198;
/// `host:%s/txt/%s.2d`, `host:%s/txt/%s.txt`, `host:%s/txt/%scam.txt`,
/// `host:%s/txt/camera.txt`, `host:%s/txt/%s`.
pub const CONFIG_2D_FORMAT: u32 = 0x00b6_8a60;
pub const CONFIG_TXT_FORMAT: u32 = 0x00b6_8d10;
pub const CAMERA_MAP_FORMAT: u32 = 0x00b6_8d24;
pub const CAMERA_FORMAT: u32 = 0x00b6_8d3c;
pub const CAMERA_PLAIN_FORMAT: u32 = 0x00b6_8d54;
/// `MSVCR110!sprintf` (the slot).
pub const SPRINTF_SLOT: u32 = 0x00b0_c3dc;

// ---- callees ----

/// `rcbgSceneLoadTextSubConfig(scene, path)` -> 0 when not found,
/// `0x0093af60(scene, parse)` (the camera text), `rcdtChunkInit(c)`,
/// `rcdtChunkLoadFromFile(c, path)`, `rcdtChunkFini(c)`, `rcdtParseInit(p)`,
/// `rcdtParseSetMode(p, mode)`, `rcdtParseSet(p, c)`, `rcdtParseFini(p)`
/// (cdecl).
pub const LOAD_CONFIG: u32 = 0x0093_b590;
pub const LOAD_CAMERA: u32 = 0x0093_af60;
pub const CHUNK_INIT: u32 = 0x0092_b840;
pub const CHUNK_LOAD: u32 = 0x0092_b880;
pub const CHUNK_FINI: u32 = 0x0092_b7f0;
pub const PARSE_INIT: u32 = 0x0092_bf50;
pub const PARSE_SET_MODE: u32 = 0x0092_c0c0;
pub const PARSE_SET: u32 = 0x0092_c0a0;
pub const PARSE_FINI: u32 = 0x0092_ba60;

pub mod bridge {
    use super::{
        CAMERA_FORMAT, CAMERA_MAP_FORMAT, CAMERA_PLAIN_FORMAT, CHUNK_FINI, CHUNK_INIT, CHUNK_LOAD,
        CONFIG_2D_FORMAT, CONFIG_TXT_FORMAT, LOAD_CAMERA, LOAD_CONFIG, PARSE_FINI, PARSE_INIT,
        PARSE_SET, PARSE_SET_MODE, RCUI_WORK, SPRINTF_SLOT,
    };
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// The path from `fmt` with the rcui work's directory (`+0x174`) and,
    /// with `map`, its map name (`+0x27c`).
    fn path(h: &mut impl Host, sprintf: u32, at: u32, fmt: u32, map: bool) {
        let work = RCUI_WORK.get(h);
        let fmt = h.rebase(fmt);
        if map {
            h.call(sprintf, Conv::Cdecl, &[at, fmt, work + 0x174, work + 0x27c]);
        } else {
            h.call(sprintf, Conv::Cdecl, &[at, fmt, work + 0x174]);
        }
    }

    /// `rcbgSceneLoadTextSub(scene)` -> `rcdtParseFini`'s `eax`: the config
    /// from `<map>.2d`, else `<map>.txt`; a mode 1 parser over the first
    /// of `<map>cam.txt`, `camera.txt`, `<map>` that loads, given to the
    /// camera load (none loading: skipped).
    pub fn load_text_sub(h: &mut impl Host, ebp: u32, scene: u32) -> u32 {
        let sprintf: u32 = h.read(h.rebase(SPRINTF_SLOT));
        let at = ebp - 0x104;
        let (chunk, parse) = (ebp - 0x198, ebp - 0x190);
        path(h, sprintf, at, CONFIG_2D_FORMAT, true);
        if cdecl(h, LOAD_CONFIG, &[scene, at]) == 0 {
            path(h, sprintf, at, CONFIG_TXT_FORMAT, true);
            cdecl(h, LOAD_CONFIG, &[scene, at]);
        }
        cdecl(h, CHUNK_INIT, &[chunk]);
        cdecl(h, PARSE_INIT, &[parse]);
        cdecl(h, PARSE_SET_MODE, &[parse, 1]);
        let mut loaded = false;
        for (fmt, map) in [
            (CAMERA_MAP_FORMAT, true),
            (CAMERA_FORMAT, false),
            (CAMERA_PLAIN_FORMAT, true),
        ] {
            path(h, sprintf, at, fmt, map);
            if cdecl(h, CHUNK_LOAD, &[chunk, at]) != 0 {
                loaded = true;
                break;
            }
        }
        if loaded {
            cdecl(h, PARSE_SET, &[parse, chunk]);
            cdecl(h, LOAD_CAMERA, &[scene, parse]);
        }
        cdecl(h, CHUNK_FINI, &[chunk]);
        cdecl(h, PARSE_FINI, &[parse])
    }
}
