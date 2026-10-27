//! The render-side glue (M4, list `h3-glue-render`, agent h3c):
//! `graphicFontDraw` (`0x00640660`), one glyph quad of the PS2 font into
//! the menu's (or the sphere grid's) dynamic vertex buffer.
//!
//! The glyph record (`rec`, built by the `TO*Font*` callers): `+0`/`+4`
//! the glyph's left and top in PS2 units, `+8`/`+0xc` u0/v1, `+0x10..+0x1c`
//! the top-left and bottom-right vertices' colour (ints 0-255, alpha
//! 0-128), `+0x20`/`+0x24` the right and the height, `+0x28`/`+0x2c`
//! u1/v0, `+0x30..+0x3c` the other two vertices' colour, `+0x80` the font
//! kind (0x7d1 widens the bottom edge by 5), `+0x90`/`+0x94` two RGBA
//! tints (bytes, high to low).
//!
//! The vertex record (`MenuManager_GetMenuVertexInfo(...)+0x94`, or an
//! `AbmapManager_GetVertexInfo` record of 0x6c bytes): `+0` textured,
//! `+4` vertex count, `+8` index count, `+0xc` positions (3 floats),
//! `+0x10` normals, `+0x14` colours (4 floats), `+0x18` uvs, `+0x1c`
//! indices (`u16`), `+0x20` the texture name, `+0x2c + 0x1c id` /
//! `+0x30 + 0x1c id` the counts per dynamic buffer.

/// `graphicUiElementRemap(rec, kind)` (cdecl).
pub const UI_ELEMENT_REMAP: u32 = 0x0064_4320;
/// `MenuManager_GetMenuVertexInfo(ui; name, kind)` (`ret 8`).
pub const GET_MENU_VERTEX_INFO: u32 = 0x0067_ba50;
/// `AbmapManager_GetVertexInfo(abmap; name, kind)` (`ret 8`).
pub const ABMAP_GET_VERTEX_INFO: u32 = 0x0068_4cd0;
/// `DynGeoMemManager_getCurrentBufferId(fill)` (thiscall, no arguments).
pub const GET_CURRENT_BUFFER_ID: u32 = 0x006d_e9f0;
/// `"NoTexture"`.
pub const STR_NO_TEXTURE: u32 = 0x00b4_1b50;
/// f64 constants: 255, 2, 0.5, 3.75 (the PS2 x scale), the glyph scale
/// (2.596153736114502), 5, 540, 960 and 1.5.
pub const K_255: u32 = 0x00b1_70c0;
pub const K_2: u32 = 0x00b9_2270;
pub const K_HALF: u32 = 0x00b9_2220;
pub const K_X_SCALE: u32 = 0x00b4_1d18;
pub const K_SCALE: u32 = 0x00b4_1d10;
pub const K_5: u32 = 0x00b4_1d20;
pub const K_540: u32 = 0x00b4_1d40;
pub const K_960: u32 = 0x00b4_1d50;
pub const K_1_5: u32 = 0x00b4_1d00;
/// The font kind whose bottom edge moves down by 5.
pub const KIND_WIDE: u32 = 0x7d1;
/// The size of an `AbmapManager` vertex record.
pub const ABMAP_RECORD: u32 = 0x6c;

pub mod bridge {
    //! The original over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use crate::glue::DYN_GEO_FILL;
    use crate::glue2::{GRAPHIC_ABMAP, GRAPHIC_SKIP_FRAME, GRAPHIC_UI};
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// f32 bits of 1.0.
    const ONE: u32 = 0x3f80_0000;

    fn k(h: &impl Host, at: u32) -> f64 {
        f64::from_bits(h.read::<u64>(h.rebase(at)))
    }

    /// `fild dword` then `fstp dword`: an int rounded to `f32`.
    fn int_f32(i: u32) -> u32 {
        x87::store(f64::from(i.cast_signed()))
    }

    /// The constants the original keeps on the x87 stack.
    struct Consts {
        k255: f64,
        k2: f64,
    }

    /// A tint's four factors from its RGBA bytes (high to low): r, g, b
    /// over 255, alpha times 2 over 255.
    fn tint(c: &Consts, rgba: u32) -> [u32; 4] {
        let [a, b, g, r] = rgba.to_le_bytes();
        let ch = |v: u8| x87::store(x87::div(f64::from(v), c.k255));
        [
            ch(r),
            ch(g),
            ch(b),
            x87::store(x87::div(x87::mul(f64::from(a), c.k2), c.k255)),
        ]
    }

    /// `graphicFontDraw(rec, name, x, y, slot)` (cdecl; `x`, `y` are `f32`
    /// bits). Leaves nothing the callers read.
    pub fn graphic_font_draw(h: &mut impl Host, rec: u32, name: u32, x: u32, y: u32, slot: u32) {
        if GRAPHIC_SKIP_FRAME.get(h) != 0 {
            return;
        }
        let c = Consts {
            k255: k(h, K_255),
            k2: k(h, K_2),
        };
        // the hi tint (+0x94) for vertices 2 and 3, the lo (+0x90) for 0, 1
        let mut hi = [ONE; 4];
        let mut lo = [ONE; 4];
        let abmap = GRAPHIC_ABMAP.get(h);
        let vi = if abmap != 0 && h.read::<u32>(abmap + 4).cast_signed() < 2 {
            hi = tint(&c, h.read(rec + 0x94));
            lo = tint(&c, h.read(rec + 0x90));
            let abmap = GRAPHIC_ABMAP.get(h);
            let (kind, index) = if slot.cast_signed() < 4 {
                (0xe, slot)
            } else {
                (0xd, slot.wrapping_sub(4))
            };
            let info = h.call_sym(ABMAP_GET_VERTEX_INFO, Conv::Thiscall(abmap), &[name, kind]);
            index
                .wrapping_mul(ABMAP_RECORD)
                .wrapping_add(h.read(info + 0x94))
        } else {
            h.call_sym(UI_ELEMENT_REMAP, Conv::Cdecl, &[rec, 2]);
            let ui = GRAPHIC_UI.get(h);
            if name == 0 {
                let none = h.rebase(STR_NO_TEXTURE);
                let info = h.call_sym(GET_MENU_VERTEX_INFO, Conv::Thiscall(ui), &[none, 0]);
                let vi: u32 = h.read(info + 0x94);
                h.write(vi + 0x20, none);
                h.write(vi, 0u32);
                vi
            } else {
                let info = h.call_sym(GET_MENU_VERTEX_INFO, Conv::Thiscall(ui), &[name, 0]);
                if info == 0 {
                    return;
                }
                h.write(info + 0xbc, 1u32);
                hi = tint(&c, h.read(rec + 0x94));
                lo = tint(&c, h.read(rec + 0x90));
                let vi: u32 = h.read(info + 0x94);
                h.write(vi + 0x20, name);
                h.write(vi, 1u32);
                vi
            }
        };
        if vi == 0 {
            return;
        }
        colours(h, &c, rec, vi, &hi, &lo);
        quad(h, rec, vi, x, y);
    }

    /// The four vertices' colours: the record's ints over 255 (alpha times
    /// 2), times the tint.
    fn colours(h: &mut impl Host, c: &Consts, rec: u32, vi: u32, hi: &[u32; 4], lo: &[u32; 4]) {
        let mut row = h.read::<u32>(vi + 4).wrapping_mul(16);
        for v in 0..4 {
            let src = if v == 0 || v == 3 {
                rec + 0x10
            } else {
                rec + 0x30
            };
            for j in 0..3 {
                let i = int_f32(h.read(src + 4 * j));
                let out = x87::store(x87::div(x87::load(i), c.k255));
                let at = h.read::<u32>(vi + 0x14).wrapping_add(row + 4 * j);
                h.write(at, out);
            }
            let i = int_f32(h.read(src + 0xc));
            let alpha = x87::store(x87::div(x87::mul(x87::load(i), c.k2), c.k255));
            let at = h.read::<u32>(vi + 0x14).wrapping_add(row + 0xc);
            h.write(at, alpha);
            let tint = if v == 2 || v == 3 { hi } else { lo };
            for (j, &f) in tint.iter().enumerate() {
                let at = h.read::<u32>(vi + 0x14).wrapping_add(row + 4 * j as u32);
                let cur: u32 = h.read(at);
                h.write(at, x87::store(x87::mul(x87::load(cur), x87::load(f))));
            }
            row = row.wrapping_add(16);
        }
    }

    /// The quad's positions (snapped to whole pixels of the 1920x1080
    /// screen, centred), normals, uvs and indices; the counts.
    #[allow(clippy::similar_names)] // x0/y0, x1/y1
    fn quad(h: &mut impl Host, rec: u32, vi: u32, x: u32, y: u32) {
        let half = k(h, K_HALF);
        let scale = k(h, K_SCALE);
        let k960 = k(h, K_960);
        let k540 = k(h, K_540);
        let f32_of = |i: u32| int_f32(i);
        let left = x87::load(h.read(rec));
        let x = x87::load(x);
        let y = x87::load(y);
        let t = x87::store(x87::add(x87::mul(left, k(h, K_X_SCALE)), x));
        let x0 = f32_of(x87::ftol(h, x87::add(x87::load(t), half)));
        let width = x87::store(x87::sub(
            x87::load(h.read(rec + 0x20)),
            x87::load(h.read(rec)),
        ));
        let t = x87::store(x87::add(x87::mul(x87::load(width), scale), x87::load(x0)));
        let x1 = f32_of(x87::ftol(h, x87::add(x87::load(t), half)));
        let p0 = x87::store(x87::sub(x87::load(x0), k960));
        let t = x87::store(x87::add(x87::mul(x87::load(h.read(rec + 4)), scale), y));
        let y0 = f32_of(x87::ftol(h, x87::add(x87::load(t), half)));
        let p1 = x87::store(x87::sub(x87::load(y0), k540));
        let p2 = x87::store(x87::sub(x87::load(x1), k960));
        let t = x87::store(x87::add(x87::mul(scale, x87::load(h.read(rec + 0x24))), y));
        let y1 = f32_of(x87::ftol(h, x87::add(half, x87::load(t))));
        let p3 = x87::store(x87::sub(x87::load(y1), k540));

        let count: u32 = h.read(vi + 4);
        let row = count.wrapping_mul(12);
        let pos = [p0, p3, ONE, p2, p3, ONE, p2, p1, ONE, p0, p1, ONE];
        for (j, &w) in pos.iter().enumerate() {
            let at = h.read::<u32>(vi + 0xc).wrapping_add(row + 4 * j as u32);
            h.write(at, w);
        }
        if h.read::<u32>(rec + 0x80) == KIND_WIDE {
            let k5 = k(h, K_5);
            for off in [0x18, 0x24] {
                let at = h.read::<u32>(vi + 0xc).wrapping_add(row + off);
                let cur: u32 = h.read(at);
                h.write(at, x87::store(x87::add(x87::load(cur), k5)));
            }
        }
        for j in 0..12u32 {
            let at = h.read::<u32>(vi + 0x10).wrapping_add(row + 4 * j);
            h.write(at, if j % 3 == 2 { ONE } else { 0 });
        }

        let row = h.read::<u32>(vi + 4).wrapping_mul(8);
        let one_minus = |v: u32| x87::store(x87::sub(1.0, x87::load(v)));
        let uv = [
            x87::store(x87::load(h.read(rec + 8))),
            one_minus(h.read(rec + 0x2c)),
            x87::store(x87::load(h.read(rec + 0x28))),
            one_minus(h.read(rec + 0x2c)),
            x87::store(x87::load(h.read(rec + 0x28))),
            one_minus(h.read(rec + 0xc)),
            x87::store(x87::load(h.read(rec + 8))),
            one_minus(h.read(rec + 0xc)),
        ];
        for (j, &w) in uv.iter().enumerate() {
            let at = h.read::<u32>(vi + 0x18).wrapping_add(row + 4 * j as u32);
            h.write(at, w);
        }

        // the index base: the count as unsigned, rounded to f32, times 1.5
        let count: u32 = h.read(vi + 4);
        let n = x87::store(f64::from(count));
        let at = x87::ftol(h, x87::mul(x87::load(n), k(h, K_1_5)));
        #[allow(clippy::cast_possible_truncation)] // `mov word`: the low half
        let base = count as u16;
        let idx = [0, 1, 3, 3, 1, 2].map(|d: u16| base.wrapping_add(d));
        for (j, &w) in idx.iter().enumerate() {
            let p = h
                .read::<u32>(vi + 0x1c)
                .wrapping_add(at.wrapping_mul(2) + 2 * j as u32);
            h.write(p, w);
        }
        h.write(vi + 4, h.read::<u32>(vi + 4).wrapping_add(4));
        h.write(vi + 8, h.read::<u32>(vi + 8).wrapping_add(6));
        let fill = DYN_GEO_FILL.get(h);
        let id = h.call_sym(GET_CURRENT_BUFFER_ID, Conv::Thiscall(fill), &[]);
        let slot = vi.wrapping_add(id.wrapping_mul(0x1c));
        h.write(slot + 0x2c, h.read::<u32>(vi + 4));
        h.write(slot + 0x30, h.read::<u32>(vi + 8));
    }
}
