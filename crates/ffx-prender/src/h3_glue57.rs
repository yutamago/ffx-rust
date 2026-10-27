//! The render-side glue, fifty-seventh batch (M4 port round 17, list
//! `h3-glue-render`): a character's place in the shadow map atlas.

/// `ShadowManager_AlreadyinShadowMap(manager; instance, &col, &row)`
/// (`ret 0xc`, `al`).
pub const ALREADY_IN_MAP: u32 = 0x006c_8040;
/// `graphicCharacterInstanceGetMaxiumBoundingBox(instance, &x, &y, &z)`
/// (cdecl).
pub const MAX_BOUNDING_BOX: u32 = 0x0063_b1b0;
/// `ShadowManager_InsertIntoShadowMap_Double` and
/// `ShadowManager_InsertIntoShadowMap` (`manager; instance, a, &col, &row,
/// c, d`; `ret 0x18`, `al`).
pub const INSERT_DOUBLE: u32 = 0x006c_e3e0;
pub const INSERT: u32 = 0x006c_e4f0;
/// `Phyre_Matrix4_translation_005746b0(out, &v)` (cdecl) -> `out`.
pub const TRANSLATION: u32 = 0x0057_46b0;
/// `PMatrix4_Copy_004c9430(out; m)` (`ret 4`).
pub const COPY_4C9430: u32 = 0x004c_9430;
/// `PMatrix4_FromMatrix3x4_00451080(out; m)` (`ret 4`).
pub const FROM_3X4: u32 = 0x0045_1080;
/// `PMatrix4_Scale_004cd270(out, &v)` (cdecl) -> `out`.
pub const SCALE: u32 = 0x004c_d270;
/// `PMatrix4_MulThis_004c9da0(a; out, b)` (`ret 8`) -> `out`.
pub const MUL_THIS: u32 = 0x004c_9da0;
/// `Matrix_RotationX(out, angle)` (cdecl) -> `out`.
pub const ROTATION_X: u32 = 0x0066_c8a0;
/// `PMatrix4_Copy_00453340(out; m)` (`ret 4`).
pub const COPY_453340: u32 = 0x0045_3340;
/// `ShadowManager_castersExtent_006d2530(instance, &x, &y, &z, matrix)`
/// (cdecl).
pub const CASTERS_EXTENT: u32 = 0x006d_2530;
/// `ShadowManager_fitShadowSize_006d23d0(w, h, &w, &h, n)` (`ret 0x14`;
/// the original hands it the manager in `ecx`, which it never reads).
pub const FIT_SHADOW_SIZE: u32 = 0x006d_23d0;
/// `g_ShadowSize` (`i32`, the atlas's side in texels).
pub const SHADOW_SIZE: u32 = 0x00c3_4408;
/// The image's f32 the light's tilt takes (0xbfc90ff9, about -pi/2).
pub const TILT: u32 = 0x00b4_99a0;

pub mod bridge {
    //! The original over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// Words of the manager's per-cell tables (64 cells, 8 by 8).
    const USED: u32 = 0xfe8;
    const WIDTH: u32 = 0x4b8;
    const HEIGHT: u32 = 0x5b8;
    const EXTENT_X: u32 = 0x6b8;
    const EXTENT_Y: u32 = 0x7b8;
    /// The cell's texture rectangle (4 f32 per cell).
    const RECT: u32 = 0x13e8;
    /// The cell's light matrix (16 f32 per cell).
    const MATRIX: u32 = 0x17e8;
    const SCALE_OUT: u32 = 0x27e8;
    const HALF: u32 = 0x28e8;
    /// 1 where the cell holds one character, else two.
    const SINGLE: u32 = 0x29e8;

    /// The frame (`ebp` offsets).
    const COL: u32 = 0x8;
    const ROW: u32 = 0x4;
    const X: u32 = 0xc;
    const Z: u32 = 0x20;
    const CENTRE: u32 = 0x1c;
    const SCALE_V: u32 = 0x70;
    /// The working matrix: the instance's, scaled, lifted, tilted.
    const WORK: u32 = 0x60;
    const MOVED: u32 = 0xb0;
    const TRANSLATED: u32 = 0xf0;
    const SCALED: u32 = 0x130;
    const TILTED: u32 = 0x170;
    const TRANSLATION_TMP: u32 = 0x1b0;
    const PRODUCT: u32 = 0x1f0;
    const TILT_TMP: u32 = 0x230;
    const SCALE_TMP: u32 = 0x270;
    const FINAL: u32 = 0x2b0;

    /// The argument slots (`slots` = `ebp + 8`).
    const OUT: u32 = 0;
    const INSTANCE: u32 = 4;
    const A: u32 = 8;
    const NEG_X: u32 = 0xc;
    const FLOOR: u32 = 0x10;
    const Z_OUT: u32 = 0x14;
    const X_OUT: u32 = 0x18;
    const Y_OUT: u32 = 0x1c;
    const RADIUS: u32 = 0x20;
    const SPREAD: u32 = 0x24;
    const C: u32 = 0x28;
    const D: u32 = 0x2c;

    const ONE: u32 = 0x3f80_0000;

    fn f(h: &impl Host, at: u32) -> f64 {
        x87::load(h.read(at))
    }

    /// `fild` then `fstp dword`: rounded once to `f32`.
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_wrap)]
    fn int_f32(v: u32) -> u32 {
        (v as i32 as f32).to_bits()
    }

    /// `g_ShadowSize` rounded toward 0 to a multiple of 8, as an `f32`
    /// (the original reloads the global for each use).
    #[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
    fn size_f32(h: &impl Host) -> u32 {
        let size: u32 = h.read(h.rebase(SHADOW_SIZE));
        let rounded = (size as i32).wrapping_add((size as i32 >> 31) & 7) & !7;
        int_f32(rounded as u32)
    }

    /// `ShadowManager_updateShadowMap(manager; out, instance, a, &-x,
    /// floor, &z, &x, &y, radius, spread, c, d)` (`ret 0x30`; Ghidra's
    /// thiscall; frame from `ebp - 0x2b0`, `slots` = `ebp + 8`) -> `out`.
    /// `out` gets the instance's cell rectangle (four f32 of `+0x13e8`),
    /// or zeros without an instance (or one whose first word is 0 but for
    /// its top bit). A character not yet in the atlas is inserted (one
    /// cell when its bounding box is wider or deeper than 30, else half
    /// of one); when that fails only the rectangle of cell (col, row) is
    /// copied. Otherwise the cell is marked used and its light matrix
    /// (`+0x17e8`) built: the instance's 3x4 matrix scaled by 1 /
    /// `radius` (0 for 0), its translation's y lowered to min(-floor,
    /// y) - 0.2, times the translation to the box's centre; `&-x` and
    /// `&z` get that product's translation (x negated); the result tilted
    /// by the image's angle (about -pi/2). The casters' extent (x, y)
    /// goes to `&x`, `&y` and the cell's tables; the cell's size in
    /// texels (`fitShadowSize` of the extent times 10, or 20 for a
    /// shared cell, clamped to 1 ..= `g_ShadowSize` / 4 - 4) to `+0x4b8`
    /// and `+0x5b8`, and its rectangle in atlas units to `+0x13e8`.
    /// The argument slots are the original's scratch.
    pub fn update_shadow_map(h: &mut impl Host, ebp: u32, slots: u32, this: u32) -> u32 {
        h.write(ebp - COL, 0u32);
        h.write(ebp - ROW, 0u32);
        let instance: u32 = h.read(slots + INSTANCE);
        let present = instance != 0 && h.read::<u32>(instance) & 0x7fff_ffff != 0;
        if !present {
            let out: u32 = h.read(slots + OUT);
            for k in 0..4 {
                // fldz; fst [ebp - 0x1c + 4k]; the word copied out
                h.write(ebp - CENTRE + 4 * k, 0u32);
                h.write(out.wrapping_add(4 * k), 0u32);
            }
            return out;
        }
        let found = h.call_sym(
            ALREADY_IN_MAP,
            Conv::Thiscall(this),
            &[instance, ebp - COL, ebp - ROW],
        );
        if found as u8 == 0 && !insert(h, ebp, slots, this, instance) {
            return copy_rect(h, ebp, slots, this);
        }
        let col: u32 = h.read(ebp - COL);
        let row: u32 = h.read(ebp - ROW);
        let cell = col.wrapping_add(row.wrapping_mul(8));
        h.write(slots + C, cell);
        h.write(
            this.wrapping_add(USED).wrapping_add(cell.wrapping_mul(4)),
            1u32,
        );
        light_matrix(h, ebp, slots, this, instance, cell);
        extent(h, ebp, slots, this, instance, cell);
        copy_rect(h, ebp, slots, this)
    }

    /// The bounding box, then the insertion: whether it worked (`al`).
    fn insert(h: &mut impl Host, ebp: u32, slots: u32, this: u32, instance: u32) -> bool {
        h.call_sym(
            MAX_BOUNDING_BOX,
            Conv::Cdecl,
            &[instance, ebp - X, slots + INSTANCE, ebp - Z],
        );
        let x = f(h, ebp - X);
        let y = f(h, slots + INSTANCE);
        // fcom against 30.0: unordered counts as not wider
        let wide = x > 30.0 || y > 30.0;
        let target = if wide { INSERT } else { INSERT_DOUBLE };
        let a: u32 = h.read(slots + A);
        let c: u32 = h.read(slots + C);
        let d: u32 = h.read(slots + D);
        let done = h.call_sym(
            target,
            Conv::Thiscall(this),
            &[instance, a, ebp - COL, ebp - ROW, c, d],
        );
        done & 0xff != 0
    }

    /// `out` = the rectangle of cell (col, row); `out`.
    fn copy_rect(h: &mut impl Host, ebp: u32, slots: u32, this: u32) -> u32 {
        let col: u32 = h.read(ebp - COL);
        let row: u32 = h.read(ebp - ROW);
        let cell = col.wrapping_add(row.wrapping_mul(8));
        let out: u32 = h.read(slots + OUT);
        let rect = this.wrapping_add(RECT).wrapping_add(cell.wrapping_mul(16));
        for k in 0..4 {
            let v: u32 = h.read(rect.wrapping_add(4 * k));
            h.write(out.wrapping_add(4 * k), v);
        }
        out
    }

    /// A 4x4 copy of `from` into the frame (`fld`/`fstp` per float), the
    /// first 15 floats.
    fn copy15(h: &mut impl Host, from: u32, to: u32) {
        for k in 0..15 {
            let v = f(h, from.wrapping_add(4 * k));
            h.write(to + 4 * k, x87::store(v));
        }
    }

    /// The cell's light matrix (`+0x17e8 + 0x40 * cell`); `&-x` and `&z`.
    fn light_matrix(h: &mut impl Host, ebp: u32, slots: u32, this: u32, instance: u32, cell: u32) {
        // the box's centre on the ground
        let mesh = mesh(h, instance);
        let bounds: u32 = h.read(mesh.wrapping_add(0x1c));
        let cx = x87::add(x87::mul(f(h, bounds.wrapping_add(0x10)), 0.5), f(h, bounds));
        h.write(ebp - CENTRE, x87::store(cx));
        h.write(ebp - CENTRE + 4, 0u32);
        let cz = x87::add(
            x87::mul(0.5, f(h, bounds.wrapping_add(0x18))),
            f(h, bounds.wrapping_add(8)),
        );
        h.write(ebp - CENTRE + 8, x87::store(cz));
        let m = h.call_sym(
            TRANSLATION,
            Conv::Cdecl,
            &[ebp - TRANSLATION_TMP, ebp - CENTRE],
        );
        h.call_sym(COPY_4C9430, Conv::Thiscall(ebp - TRANSLATED), &[m]);
        let mesh = self::mesh(h, instance);
        let source: u32 = h.read(mesh.wrapping_add(4));
        h.call_sym(FROM_3X4, Conv::Thiscall(ebp - WORK), &[source]);
        // 1 / radius, 0 for a radius of 0 (a NaN gives NaN)
        let radius = f(h, slots + RADIUS);
        let s = if radius == 0.0 {
            0.0
        } else {
            x87::div(1.0, radius)
        };
        let s = x87::store(s);
        h.write(slots + A, s);
        for k in 0..3 {
            h.write(ebp - SCALE_V + 4 * k, s);
        }
        let m = h.call_sym(SCALE, Conv::Cdecl, &[ebp - SCALE_TMP, ebp - SCALE_V]);
        h.call_sym(COPY_4C9430, Conv::Thiscall(ebp - SCALED), &[m]);
        let m = h.call_sym(
            MUL_THIS,
            Conv::Thiscall(ebp - WORK),
            &[ebp - PRODUCT, ebp - SCALED],
        );
        copy15(h, m, ebp - WORK);
        // the translation's y: min(-floor, y) - 0.2 (an unordered compare
        // keeps -floor)
        let floor = -f(h, slots + FLOOR);
        h.write(slots + D, x87::store(floor));
        let y = f(h, ebp - WORK + 0x34);
        let floor = f(h, slots + D);
        let low = if floor > y { y } else { floor };
        h.write(slots + D, x87::store(low));
        // the image's f64 0.2000000029802322 (0.2 as f32)
        let low = x87::store(x87::sub(f(h, slots + D), f64::from(0.2_f32)));
        h.write(slots + D, low);
        h.write(ebp - WORK + 0x34, low);
        h.write(ebp - WORK + 0x3c, ONE);
        h.call_sym(
            MUL_THIS,
            Conv::Thiscall(ebp - WORK),
            &[ebp - MOVED, ebp - TRANSLATED],
        );
        let tx = f(h, ebp - MOVED + 0x30);
        let neg_x: u32 = h.read(slots + NEG_X);
        h.write(neg_x, x87::store(-tx));
        let z_out: u32 = h.read(slots + Z_OUT);
        let tz = f(h, ebp - MOVED + 0x38);
        h.write(z_out, x87::store(tz));
        h.write(ebp - WORK + 0x30, x87::store(tx));
        let ty = f(h, ebp - MOVED + 0x34);
        h.write(ebp - WORK + 0x34, x87::store(ty));
        h.write(ebp - WORK + 0x38, x87::store(tz));
        h.write(ebp - WORK + 0x3c, ONE);
        let tilt = x87::store(f(h, h.rebase(TILT)));
        let m = h.call_sym(ROTATION_X, Conv::Cdecl, &[ebp - TILT_TMP, tilt]);
        h.call_sym(COPY_4C9430, Conv::Thiscall(ebp - TILTED), &[m]);
        let m = h.call_sym(
            MUL_THIS,
            Conv::Thiscall(ebp - WORK),
            &[ebp - FINAL, ebp - TILTED],
        );
        let target = this.wrapping_add(MATRIX).wrapping_add(cell << 6);
        h.call_sym(COPY_453340, Conv::Thiscall(target), &[m]);
    }

    /// The instance's mesh record (`[[instance + 4]]`).
    fn mesh(h: &impl Host, instance: u32) -> u32 {
        let node: u32 = h.read(instance.wrapping_add(4));
        h.read(node)
    }

    /// The casters' extent, the cell's size and rectangle.
    #[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
    fn extent(h: &mut impl Host, ebp: u32, slots: u32, this: u32, instance: u32, cell: u32) {
        let matrix = this.wrapping_add(MATRIX).wrapping_add(cell << 6);
        h.call_sym(
            CASTERS_EXTENT,
            Conv::Cdecl,
            &[instance, ebp - X, slots + INSTANCE, ebp - Z, matrix],
        );
        let x = f(h, ebp - X);
        let x_out: u32 = h.read(slots + X_OUT);
        // the cell from the slot the original keeps it in
        let cell: u32 = h.read(slots + C);
        h.write(x_out, x87::store(x));
        let y_out: u32 = h.read(slots + Y_OUT);
        let y = f(h, slots + INSTANCE);
        h.write(y_out, x87::store(y));
        let at = |base: u32| this.wrapping_add(base).wrapping_add(cell.wrapping_mul(4));
        h.write(at(EXTENT_X), x87::store(x));
        let y = f(h, slots + INSTANCE);
        h.write(at(EXTENT_Y), x87::store(y));
        let y = f(h, slots + INSTANCE);
        let size: u32 = h.read(h.rebase(SHADOW_SIZE));
        let eighth = (size as i32) / 8;
        let single = h.read::<u32>(at(SINGLE)) == 1;
        let (k, n) = if single {
            (10.0, eighth.wrapping_sub(4))
        } else {
            (20.0, eighth.wrapping_mul(2).wrapping_sub(4))
        };
        let height = x87::store(x87::mul(y, k));
        h.write(slots + D, height);
        let width = x87::store(x87::mul(k, f(h, ebp - X)));
        h.write(slots + D, width);
        h.call_sym(
            FIT_SHADOW_SIZE,
            Conv::Thiscall(this),
            &[width, height, slots + D, slots + C, n as u32],
        );
        let limit = eighth.wrapping_mul(2).wrapping_sub(4);
        let clamp = |v: u32| (v as i32).min(limit).max(1) as u32;
        let w = clamp(h.read(slots + D));
        let ht = clamp(h.read(slots + C));
        h.write(slots + D, w);
        h.write(slots + C, ht);
        h.write(at(WIDTH), w);
        h.write(at(HEIGHT), ht);
        rect(h, ebp, slots, this, cell);
        // the radius's reciprocal and half the spread
        let s = f(h, slots + A);
        h.write(at(SCALE_OUT), x87::store(s));
        let half = x87::mul(0.5, f(h, slots + SPREAD));
        h.write(at(HALF), x87::store(half));
    }

    /// The cell's rectangle in atlas units: col / 8 + 0 / size, then
    /// the same + (w + 4) / 2 / size; the rows likewise. `slots + 0x20`
    /// is the original's conversion scratch.
    fn rect(h: &mut impl Host, ebp: u32, slots: u32, this: u32, cell: u32) {
        let rect = this.wrapping_add(RECT).wrapping_add(cell.wrapping_mul(16));
        let scratch = slots + RADIUS;
        let col = int_f32(h.read(ebp - COL));
        h.write(scratch, col);
        let col = x87::mul(x87::load(col), 0.125);
        let size = size_f32(h);
        h.write(scratch, size);
        let left = x87::add(x87::div(0.0, x87::load(size)), col);
        h.write(rect, x87::store(left));
        let w: u32 = h.read(this.wrapping_add(WIDTH).wrapping_add(cell.wrapping_mul(4)));
        let w = int_f32(w.wrapping_add(4));
        h.write(scratch, w);
        let half_w = x87::mul(x87::load(w), 0.5);
        let size = size_f32(h);
        h.write(scratch, size);
        let right = x87::store(x87::add(col, x87::div(half_w, x87::load(size))));
        h.write(scratch, right);
        h.write(rect.wrapping_add(8), right);
        let row = int_f32(h.read(ebp - ROW));
        h.write(scratch, row);
        let row = x87::mul(0.125, x87::load(row));
        let size = size_f32(h);
        h.write(scratch, size);
        let top = x87::add(x87::div(0.0, x87::load(size)), row);
        h.write(rect.wrapping_add(4), x87::store(top));
        let ht: u32 = h.read(this.wrapping_add(HEIGHT).wrapping_add(cell.wrapping_mul(4)));
        let ht = int_f32(ht.wrapping_add(4));
        h.write(scratch, ht);
        let half_h = x87::mul(x87::load(ht), 0.5);
        let size = size_f32(h);
        h.write(scratch, size);
        let bottom = x87::add(row, x87::div(half_h, x87::load(size)));
        h.write(rect.wrapping_add(12), x87::store(bottom));
    }
}
