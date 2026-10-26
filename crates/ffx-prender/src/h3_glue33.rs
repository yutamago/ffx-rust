//! The render-side glue, thirty-third batch (M4 port round 8, list
//! `h3-glue-render`): the shadow caster's frustum test and a character
//! asset's arrival. The exception frame and cookie are left out, as in
//! [`crate::glue10`].

/// `PMatrix4_TransformPoint_004c9cb0(m; out, p)` (`ret 8`), `ClipOutcode
/// (v)` (cdecl), the doubles 3, 0.9 and -1.1.
pub const TRANSFORM_POINT: u32 = 0x004c_9cb0;
pub const CLIP_OUTCODE: u32 = 0x0065_2470;
pub const THREE: u32 = 0x00b2_c5c8;
pub const NINE_TENTHS: u32 = 0x00b4_9958;
pub const LIMIT: u32 = 0x00b4_99b0;

/// `AsyncLoadManager_sGetInstance`, the MSVC string's `assign(this; p,
/// n)` (`ret 8`), `AssetListReader_getAssetPath(this; i)` (`ret 4`) and
/// `getAssetCount(this)`, `strstr`, `strncpy` and `atoi` (cdecl, through
/// their import slots), `"/fp/"`, `fiosUnifyFilename(src, dst, n)`
/// (cdecl), the asset's item `0x0065ae40` (`ret 0x14`), the scene's
/// all-loaded step (`ret 0x18`) and `operator delete`.
pub const ASYNC_LOAD_INSTANCE: u32 = 0x0069_1b40;
pub const STRING_ASSIGN: u32 = 0x0040_4270;
pub const ASSET_PATH: u32 = 0x006f_41b0;
pub const ASSET_COUNT: u32 = 0x006f_4170;
pub const STRSTR: u32 = 0x00b0_c378;
pub const STRNCPY: u32 = 0x00b0_c368;
pub const ATOI: u32 = 0x00b0_c40c;
pub const FP_FOLDER: u32 = 0x00b4_2678;
pub const UNIFY_FILENAME: u32 = 0x0067_9820;
pub const ASSET_ITEM: u32 = 0x0065_ae40;
pub const ALL_ASSETS_LOADED: u32 = 0x0065_a760;
pub const OPERATOR_DELETE: u32 = 0x0063_0390;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// `fld dword [at]`.
    fn ld(h: &impl Host, at: u32) -> f64 {
        x87::load(h.read(at))
    }

    /// A double constant of the image.
    fn constant(h: &impl Host, sym: u32) -> f64 {
        h.read(h.rebase(sym))
    }

    /// `0x006c9270(this, m; slots)` (thiscall, `ret 4`; frame `ebp -
    /// 0x60` .. `- 4`): whether a caster with the matrix `m` may cast into
    /// the camera's view (`this`: a box, `+0` .. `+0x18`). The point
    /// (`+0`, `(+0x14 + +4) - (+0x18 + +0x10) / 3`, `+8`) is transformed by
    /// `m` (`o`); `P = o + m.row0 * +0x10`, `Q = o + m.row2 * +0x18`,
    /// `R = Q's offset + P`, each rounded to `f32` per component, and the
    /// second component of `o`, `P`, `Q` and `R` replaced by 0.9 w. All
    /// four outside one clip plane together: 0. Else 1 when any of
    /// `o.y / o.w`, `P.y / P.w`, `Q.y / Q.w`, `R.y / R.w` (the unscaled
    /// ys) is above -1.1, else 0. `al` only (callers test it); `this`'s
    /// argument slot keeps the last float stored there.
    pub fn caster_visible(h: &mut impl Host, ebp: u32, slots: u32, this: u32, m: u32) -> u8 {
        // the first and third rows scaled by +0x10 and +0x18
        let s = x87::store(ld(h, this + 0x10));
        h.write(slots, s);
        let s = x87::load(s);
        for k in 0..4 {
            let v = x87::store(x87::mul(ld(h, m + 4 * k), s));
            h.write(ebp - 0x40 + 4 * k, v);
        }
        let t = x87::store(ld(h, this + 0x18));
        h.write(slots, t);
        let t = x87::load(t);
        for k in 0..4 {
            let v = x87::store(x87::mul(ld(h, m + 0x20 + 4 * k), t));
            h.write(ebp - 0x50 + 4 * k, v);
        }
        // the point, transformed
        let x = x87::store(ld(h, this));
        h.write(ebp - 0x30, x);
        let a = x87::add(ld(h, this + 0x14), ld(h, this + 4));
        let b = x87::add(ld(h, this + 0x18), ld(h, this + 0x10));
        let b = x87::div(b, constant(h, THREE));
        h.write(ebp - 0x2c, x87::store(x87::sub(a, b)));
        let z = x87::store(ld(h, this + 8));
        h.write(ebp - 0x28, z);
        let w: u32 = h.read(m + 0x2c);
        h.write(ebp - 0x24, w);
        h.call_sym(
            TRANSFORM_POINT,
            Conv::Thiscall(m),
            &[ebp - 0x60, ebp - 0x30],
        );
        let o: [f64; 4] = std::array::from_fn(|k| ld(h, ebp - 0x60 + 4 * k as u32));
        let x_row: [f64; 4] = std::array::from_fn(|k| ld(h, ebp - 0x40 + 4 * k as u32));
        let y_row: [f64; 4] = std::array::from_fn(|k| ld(h, ebp - 0x50 + 4 * k as u32));
        let mut p = [0u32; 4];
        let mut q = [0u32; 4];
        let mut r = [0u32; 4];
        for k in 0..4 {
            p[k] = x87::store(x87::add(o[k], x_row[k]));
            h.write(ebp - 0x30 + 4 * k as u32, p[k]);
        }
        for k in 0..4 {
            q[k] = x87::store(x87::add(o[k], y_row[k]));
            h.write(ebp - 0x40 + 4 * k as u32, q[k]);
            r[k] = x87::store(x87::add(y_row[k], x87::load(p[k])));
            h.write(ebp - 0x50 + 4 * k as u32, r[k]);
        }
        // the ys and ws kept for the ratios
        let ow = x87::store(o[3]);
        let oy = x87::store(o[1]);
        for (at, v) in [
            (0xc, ow),
            (8, oy),
            (0x14, p[3]),
            (0x10, p[1]),
            (0x1c, q[3]),
            (0x18, q[1]),
            (0x20, r[3]),
            (4, r[1]),
        ] {
            h.write(ebp - at, v);
        }
        h.write(slots, r[3]);
        // y = 0.9 w
        let k9 = constant(h, NINE_TENTHS);
        for (at, w) in [(0x5c, ow), (0x2c, p[3]), (0x3c, q[3]), (0x4c, r[3])] {
            let v = x87::store(x87::mul(x87::load(w), k9));
            h.write(ebp - at, v);
        }
        let mut codes = [0u32; 4];
        for (k, at) in [0x50, 0x40, 0x30, 0x60].into_iter().enumerate() {
            codes[k] = h.call_sym(CLIP_OUTCODE, Conv::Cdecl, &[ebp - at]);
        }
        if codes[0] & codes[1] & codes[2] & codes[3] != 0 {
            return 0;
        }
        let limit = constant(h, LIMIT);
        for (y, w) in [(oy, ow), (p[1], p[3]), (q[1], q[3]), (r[1], r[3])] {
            let ratio = x87::store(x87::div(x87::load(y), x87::load(w)));
            h.write(slots, ratio);
            if x87::load(ratio) > limit {
                return 1;
            }
        }
        0
    }

    /// A cdecl import through its slot.
    fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, Conv::Cdecl, args)
    }

    /// `PhyreScene_onCharacterAssetLoaded(task, request)` (cdecl, a load
    /// callback; frame: the task at `ebp - 0x50`, the request's words
    /// below, an MSVC string at `ebp - 0x30`, a 5-character number at `-
    /// 0x18`). The request: scene, out, reader, array, id, name, asset
    /// index, extra. The name is copied into the string. An asset under
    /// `/fp/` takes the scene's slot for `id` (8 slots of 0x20c bytes at
    /// `+0xfccc`: the one holding `id`, else the first free one, else
    /// -1), marked used with `id`; the number of its file name's folder
    /// goes into the slot's first free word of two (`+8`), and the path,
    /// unified, into the matching 0x100-byte name. The asset is added
    /// (`0x0065ae40`), and after the last one the scene's all-loaded step
    /// runs. `eax`: the last call's.
    pub fn character_asset_loaded(h: &mut impl Host, ebp: u32, task: u32, request: u32) -> u32 {
        h.write(ebp - 0x50, task);
        h.call_sym(ASYNC_LOAD_INSTANCE, Conv::Cdecl, &[]);
        let [scene, out, reader, array, id, name, index, extra]: [u32; 8] =
            std::array::from_fn(|k| h.read(request + 4 * k as u32));
        let string = ebp - 0x30;
        h.write(string + 0x14, 0xfu32);
        h.write(string + 0x10, 0u32);
        h.write(string, 0u8);
        for (at, v) in [
            (0x38, scene),
            (0x3c, reader),
            (0x44, out),
            (0x40, array),
            (0x48, id),
            (0x4c, name),
            (0x34, index),
            (0x58, extra),
        ] {
            h.write(ebp - at, v);
        }
        let len = if h.read::<u8>(name) == 0 {
            0
        } else {
            strlen(h, name)
        };
        h.call_sym(STRING_ASSIGN, Conv::Thiscall(string), &[name, len]);
        let path = h.call_sym(ASSET_PATH, Conv::Thiscall(reader), &[index]);
        let fp = h.rebase(FP_FOLDER);
        if import(h, STRSTR, &[path, fp]) != 0 {
            fp_slot(h, ebp, scene, reader, id, index);
        }
        h.call_sym(
            ASSET_ITEM,
            Conv::Thiscall(scene),
            &[out, reader, array, task, index],
        );
        let count = h.call_sym(ASSET_COUNT, Conv::Thiscall(reader), &[]);
        let mut last = count;
        if index.wrapping_add(1) == count {
            last = h.call_sym(
                ALL_ASSETS_LOADED,
                Conv::Thiscall(scene),
                &[out, reader, array, id, name, extra],
            );
        }
        if h.read::<u32>(string + 0x14) >= 0x10 {
            let data: u32 = h.read(string);
            last = h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[data]);
        }
        last
    }

    /// `strlen`.
    fn strlen(h: &impl Host, p: u32) -> u32 {
        let mut n = 0u32;
        while h.read::<u8>(p.wrapping_add(n)) != 0 {
            n = n.wrapping_add(1);
        }
        n
    }

    /// The `/fp/` asset's slot, number and unified path (see
    /// [`character_asset_loaded`]).
    fn fp_slot(h: &mut impl Host, ebp: u32, scene: u32, reader: u32, id: u32, index: u32) {
        let mut slot = u32::MAX;
        for k in 0..8u32 {
            let at = scene + 0xfccc + 0x20c * k;
            if h.read::<u32>(at) == 0 {
                if slot == u32::MAX {
                    slot = k;
                }
            } else if h.read::<u32>(at + 4) == id {
                slot = k;
                break;
            }
        }
        let offset = slot.wrapping_mul(0x20c);
        h.write(ebp - 0x54, offset);
        h.write(scene.wrapping_add(offset).wrapping_add(0xfccc), 1u32);
        h.write(scene.wrapping_add(offset).wrapping_add(0xfcd0), id);
        let path = h.call_sym(ASSET_PATH, Conv::Thiscall(reader), &[index]);
        let len = strlen(h, path);
        let path = h.call_sym(ASSET_PATH, Conv::Thiscall(reader), &[index]);
        let mut at = path.wrapping_add(len);
        while h.read::<u8>(at) != b'/' {
            at = at.wrapping_sub(1);
        }
        let digits = ebp - 0x18;
        import(h, STRNCPY, &[digits, at.wrapping_add(1), 5]);
        h.write(ebp - 0x13, 0u8);
        let number = import(h, ATOI, &[digits]) as u16;
        let words = scene.wrapping_add(offset).wrapping_add(0xfcd4);
        for k in 0..2u32 {
            if h.read::<u16>(words.wrapping_add(2 * k)) == 0 {
                h.write(words.wrapping_add(2 * k), number);
                let dst = scene
                    .wrapping_add(0xfcd8)
                    .wrapping_add(k << 8)
                    .wrapping_add(offset);
                let path = h.call_sym(ASSET_PATH, Conv::Thiscall(reader), &[index]);
                h.call_sym(UNIFY_FILENAME, Conv::Cdecl, &[path, dst, 0x100]);
                return;
            }
        }
    }
}
