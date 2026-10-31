//! M5 stage 1, list `map`, round 5 batch 16: a scene's textures
//! (`rcbgSceneLoadTexture`): the layer count from the highest letter of
//! the map's files in `<dir>/2d/tex`, the layers (depth 100 - i), then
//! each a plain texture when `<map><c>.tm2` is listed, else an animated
//! one, and its centre.
//!
//! The file list, the path and the spills live in the original's frame
//! and come in through its `ebp` (the shim passes a `StackFrame`).

pub use crate::m5m_e::RCUI_WORK;

/// The frame: `ebp` minus its lowest local.
pub const FRAME: u32 = 0x118;
/// `%s/2d/tex`, `%s%c.tm2`, `%s_%c.tm2`, the source file's name,
/// `g_HundredF64`.
pub const DIR_FORMAT: u32 = 0x00b6_8c0c;
pub const FILE_FORMAT: u32 = 0x00b6_8c18;
pub const FILE_FORMAT_HIGH: u32 = 0x00b6_8c24;
pub const SOURCE_FILE: u32 = 0x00b6_8be8;
pub const HUNDRED: u32 = 0x00b9_22e0;
/// `MSVCR110!sprintf`, `strncmp` (the slots).
pub const SPRINTF_SLOT: u32 = 0x00b0_c3dc;
pub const STRNCMP_SLOT: u32 = 0x00b0_c3e4;

// ---- callees ----

/// `Gu_MakeFileList(list, dir)` (`list`: count, _, names),
/// `rcAlloc(n, file, line)`, `rcbgLayerInit(l)`,
/// `rcbgSceneLoadTextureNormal(scene, i)`, `rcbgSceneLoadTextureAnime(scene,
/// i)`, `rcbgLayerLoadCalcCenter(l)` (cdecl).
pub const MAKE_FILE_LIST: u32 = 0x0084_98e0;
pub const RC_ALLOC: u32 = 0x0092_58a0;
pub const LAYER_INIT: u32 = 0x0092_ad20;
pub const LOAD_NORMAL: u32 = 0x0093_bd10;
pub const LOAD_ANIME: u32 = 0x0093_b930;
pub const LAYER_CALC_CENTER: u32 = 0x0092_b270;

pub mod bridge {
    use super::{
        DIR_FORMAT, FILE_FORMAT, FILE_FORMAT_HIGH, HUNDRED, LAYER_CALC_CENTER, LAYER_INIT,
        LOAD_ANIME, LOAD_NORMAL, MAKE_FILE_LIST, RC_ALLOC, RCUI_WORK, SOURCE_FILE, SPRINTF_SLOT,
        STRNCMP_SLOT,
    };
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, Conv::Cdecl, args)
    }

    fn strlen(h: &impl Host, mut at: u32) -> u32 {
        let start = at;
        while h.read::<u8>(at) != 0 {
            at += 1;
        }
        at - start
    }

    /// The layer a texture file's name gives (`[6]` `a`..`z`: 1..26, or
    /// `_` and `[7]` `a`..`z`: 27..52), when it starts with the map's name.
    fn layer_of(h: &mut impl Host, name: u32) -> Option<i32> {
        let map = RCUI_WORK.get(h) + 0x27c;
        let len = strlen(h, map);
        if import(h, STRNCMP_SLOT, &[name, map, len]) != 0 {
            return None;
        }
        let c = h.read::<u8>(name + 6);
        let d = h.read::<u8>(name + 7);
        if c.wrapping_sub(b'a') <= 0x19 {
            Some(i32::from(c.cast_signed()) - 0x60)
        } else if c == b'_' && d.wrapping_sub(b'a') <= 0x19 {
            Some(i32::from(d.cast_signed()) - 0x46)
        } else {
            None
        }
    }

    /// `rcbgSceneLoadTexture(scene)` -> the last callee's `eax`: the
    /// files of `<dir>/2d/tex` (the rcui work's `+0x174`) give the layer
    /// count (`scene +4`, the highest of [`layer_of`]); `scene +8` the
    /// layers (0xc0 bytes each, `rcAlloc`ed at line 0xab, initialised,
    /// `+0x14` the depth 100 - i); then each layer `i` is a plain texture
    /// when a listed name starts with `<map><c>.tm2`, else an animated one,
    /// and gets its centre.
    pub fn load_texture(h: &mut impl Host, ebp: u32, scene: u32) -> u32 {
        h.write(ebp - 0x10c, scene);
        let list = ebp - 0x118;
        for k in 0..3 {
            h.write(list + 4 * k, 0_u32);
        }
        let path = ebp - 0x104;
        let fmt = h.rebase(DIR_FORMAT);
        let dir = RCUI_WORK.get(h) + 0x174;
        import(h, SPRINTF_SLOT, &[path, fmt, dir]);
        cdecl(h, MAKE_FILE_LIST, &[list, path]);
        let mut n = 0_i32;
        let mut j = 0_i32;
        while j < h.read::<i32>(list) {
            let names: u32 = h.read(list + 8);
            let name: u32 = h.read(names.wrapping_add(j.cast_unsigned().wrapping_mul(4)));
            if let Some(v) = layer_of(h, name) {
                n = n.max(v);
            }
            j += 1;
        }
        h.write(scene + 4, n.cast_unsigned());
        let file = h.rebase(SOURCE_FILE);
        let layers = cdecl(
            h,
            RC_ALLOC,
            &[n.cast_unsigned().wrapping_mul(0xc0), file, 0xab],
        );
        h.write(scene + 8, layers);
        let mut eax = layers;
        h.write(ebp - 0x108, 0_u32);
        let hundred = f64::from_bits(h.read::<u64>(h.rebase(HUNDRED)));
        let mut i = 0_i32;
        while i < n {
            let base: u32 = h.read(scene + 8);
            let at = base.wrapping_add(i.cast_unsigned().wrapping_mul(0xc0));
            cdecl(h, LAYER_INIT, &[at]);
            // fild, fstp dword; fsubr qword 100.0; fstp dword
            let f = x87::load(x87::store(f64::from(i)));
            let depth = x87::store(x87::sub(hundred, f));
            i += 1;
            h.write(ebp - 0x108, i.cast_unsigned());
            let base: u32 = h.read(scene + 8);
            eax = base;
            let at = base.wrapping_add(i.cast_unsigned().wrapping_mul(0xc0));
            h.write(at.wrapping_sub(0xac), depth);
        }
        let mut i = 0_u32;
        h.write(ebp - 0x108, 0_u32);
        while i.cast_signed() < h.read::<i32>(scene + 4) {
            let (fmt, letter) = if i.cast_signed() < 0x1a {
                (FILE_FORMAT, i.wrapping_add(0x61))
            } else {
                (FILE_FORMAT_HIGH, i.wrapping_add(0x47))
            };
            let fmt = h.rebase(fmt);
            let map = RCUI_WORK.get(h) + 0x27c;
            import(h, SPRINTF_SLOT, &[path, fmt, map, letter]);
            let mut listed = false;
            let mut j = 0_i32;
            while j < h.read::<i32>(list) {
                let names: u32 = h.read(list + 8);
                let name: u32 = h.read(names.wrapping_add(j.cast_unsigned().wrapping_mul(4)));
                let len = strlen(h, path);
                if import(h, STRNCMP_SLOT, &[name, path, len]) == 0 {
                    listed = true;
                    break;
                }
                j += 1;
            }
            let load = if listed { LOAD_NORMAL } else { LOAD_ANIME };
            cdecl(h, load, &[scene, i]);
            let base: u32 = h.read(scene + 8);
            let offset: u32 = h.read(ebp - 0x108);
            eax = cdecl(h, LAYER_CALC_CENTER, &[base.wrapping_add(offset)]);
            i += 1;
            h.write(ebp - 0x108, offset.wrapping_add(0xc0));
        }
        eax
    }
}
