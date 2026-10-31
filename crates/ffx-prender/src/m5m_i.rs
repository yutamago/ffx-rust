//! M5 stage 1, list `map`, round 5 batch 5: the guide-map converter's
//! model load (`rcbgGuideMapConvSub`).

use ffx_bridge::Global;

/// `g_YnMapSystem`.
pub const YN_MAP_SYSTEM: Global<u32> = Global::new(0x0193_bf80);
/// The guide-map model path formats (without and with a sub-map).
pub const PATH_FORMAT: u32 = 0x00b6_89b0;
pub const SUB_PATH_FORMAT: u32 = 0x00b6_89e8;
/// `MSVCR110!sprintf` (the slot).
pub const SPRINTF_SLOT: u32 = 0x00b0_c3dc;

// ---- callees ----

/// `rcuiGetStageName(id, out)`, `sceOpen(name, mode)`, `sceClose(fd)`,
/// `rcbgModelNew()`, `rcbgModelLoadByExt_0093ad60(m, name)`,
/// `rcbgModelDelete(m)` (cdecl).
pub const GET_STAGE_NAME: u32 = 0x0093_7940;
pub const SCE_OPEN: u32 = 0x0062_fa30;
pub const SCE_CLOSE: u32 = 0x0062_f610;
pub const MODEL_NEW: u32 = 0x0092_83b0;
pub const MODEL_LOAD_BY_EXT: u32 = 0x0093_ad60;
pub const MODEL_DELETE: u32 = 0x0092_8000;

pub mod bridge {
    use super::{
        GET_STAGE_NAME, MODEL_DELETE, MODEL_LOAD_BY_EXT, MODEL_NEW, PATH_FORMAT, SCE_CLOSE,
        SCE_OPEN, SPRINTF_SLOT, SUB_PATH_FORMAT, YN_MAP_SYSTEM,
    };
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// `rcbgGuideMapConvSub(id, sub)` -> the model, or 0: the stage's
    /// guide-map model (its name into `stage`, the original's `ebp - 0xc`;
    /// the path into `path`, `ebp - 0x10c`, with the sub-map when `sub` is
    /// set), only when the file opens (closed again); loaded with a map,
    /// its vertex words cleared, `+0xa4` and `+0x94` 0.0 and flags 6 set;
    /// otherwise deleted.
    pub fn guide_map_conv_sub(h: &mut impl Host, stage: u32, path: u32, id: u32, sub: u32) -> u32 {
        if id == 0 || cdecl(h, GET_STAGE_NAME, &[id, stage]) == 0 {
            return 0;
        }
        let c: [u32; 4] =
            core::array::from_fn(|k| i32::from(h.read::<i8>(stage + k as u32)).cast_unsigned());
        let sprintf: u32 = h.read(h.rebase(SPRINTF_SLOT));
        if sub == 0 {
            let fmt = h.rebase(PATH_FORMAT);
            h.call(
                sprintf,
                Conv::Cdecl,
                &[path, fmt, c[0], c[1], c[2], c[3], stage],
            );
        } else {
            let fmt = h.rebase(SUB_PATH_FORMAT);
            h.call(
                sprintf,
                Conv::Cdecl,
                &[path, fmt, c[0], c[1], c[2], c[3], stage, sub],
            );
        }
        let fd = cdecl(h, SCE_OPEN, &[path, 1]);
        if fd == 0 {
            return 0;
        }
        cdecl(h, SCE_CLOSE, &[fd]);
        let model = cdecl(h, MODEL_NEW, &[]);
        let r = cdecl(h, MODEL_LOAD_BY_EXT, &[model, path]);
        let sys = YN_MAP_SYSTEM.get(h);
        if r != 0 && sys != 0 && h.read::<u32>(sys) != 0 {
            let mesh: u32 = h.read(model + 0x14);
            let n = i32::from(h.read::<i16>(mesh + 0x12));
            let mut at = mesh.wrapping_add(h.read::<u32>(mesh + 8)).wrapping_add(2);
            for _ in 0..n.max(0) {
                h.write(at, 0_u16);
                at = at.wrapping_add(6);
            }
            h.write(model + 0xa4, 0_u32);
            h.write(model + 0x94, 0_u32);
            let flags: u32 = h.read(model + 0x10);
            h.write(model + 0x10, flags | 6);
            return model;
        }
        if model != 0 {
            cdecl(h, MODEL_DELETE, &[model]);
        }
        0
    }
}
