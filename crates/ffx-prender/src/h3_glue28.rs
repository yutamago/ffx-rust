//! The render-side glue, twenty-eighth batch (M4 port round 7, list
//! `h3-glue-render`): the scaled button icons.

/// `graphicUiRemapX2(v)` / `graphicUiRemapY2(v)` (cdecl, a float in
/// `ST0`), `TOGetFFXLangJmp()`, `DrawCrossButtonIconRGBA(id, x, y, w, h,
/// colour, flags)` (cdecl) and `g_CurrentMapNo`.
pub const UI_REMAP_X2: u32 = crate::h3_glue20::UI_REMAP_X2;
pub const UI_REMAP_Y2: u32 = crate::h3_glue20::UI_REMAP_Y2;
pub const GET_LANG: u32 = 0x008a_c2f0;
pub const DRAW_CROSS_BUTTON_ICON: u32 = 0x008e_6070;
pub const CURRENT_MAP_NO: u32 = crate::h3_glue27::CURRENT_MAP_NO;

/// The `f64` factors: 0.5 (rounding), the sizes 54, 64 and 40 (the
/// shoulder and pad buttons), 84.5 and 60, 147.4 and 50, the offsets 7
/// and 1.5, the nudges 5 and 2.5.
pub const HALF: u32 = 0x00b9_2220;
pub const SIZE_54: u32 = 0x00b5_f1f8;
pub const SIZE_64: u32 = 0x00b4_1308;
pub const SIZE_40: u32 = 0x00b5_af28;
pub const SIZE_84_5: u32 = 0x00b6_6cc0;
pub const SIZE_60: u32 = 0x00b4_a180;
pub const SIZE_147_4: u32 = 0x00b6_6cd0;
pub const SIZE_50: u32 = 0x00b1_f2c0;
pub const OFFSET_7: u32 = 0x00b9_22a0;
pub const OFFSET_1_5: u32 = 0x00b4_1d00;
pub const NUDGE_5: u32 = 0x00b4_1d20;
pub const NUDGE_2_5: u32 = 0x00b4_3d70;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// The `f64` at `sym`.
    fn constant(h: &impl Host, sym: u32) -> f64 {
        let at = h.rebase(sym);
        f64::from_bits(h.read::<u64>(at))
    }

    /// The icon sizes in progress: the arguments as the original keeps
    /// them in its slots (`x`, `y` and the scratch word over `id`), and
    /// the remapped width and height.
    struct Icon {
        slots: u32,
        scale: f64,
        x: u32,
        y: u32,
        w: u32,
        ht: u32,
    }

    impl Icon {
        /// `remap(f32(ftol(scale * factor + 0.5)))`, the integer and then
        /// the float going through the scratch slot.
        fn rounded(&self, h: &mut impl Host, factor: u32, remap: u32) -> u32 {
            let product = x87::mul(self.scale, constant(h, factor));
            let i = x87::ftol(h, x87::add(product, constant(h, HALF)));
            h.write(self.slots, i);
            let f = x87::store(f64::from(i as i32));
            h.write(self.slots, f);
            x87::store(h.call_sym_st0_f64(remap, Conv::Cdecl, &[f]))
        }

        /// `y += remapY2(f32(scale * offset))` (the product through the
        /// scratch slot).
        fn lower(&mut self, h: &mut impl Host, offset: f64) {
            let t = x87::store(x87::mul(offset, self.scale));
            h.write(self.slots, t);
            let r = h.call_sym_st0_f64(UI_REMAP_Y2, Conv::Cdecl, &[t]);
            self.y = x87::store(x87::add(r, x87::load(self.y)));
            h.write(self.slots + 8, self.y);
        }

        fn nudge(&mut self, h: &mut impl Host, by: f64, add: bool) {
            let x = x87::load(self.x);
            self.x = x87::store(if add {
                x87::add(x, by)
            } else {
                x87::sub(x, by)
            });
            h.write(self.slots + 4, self.x);
        }
    }

    /// `DrawScaleButtonIconRGBA(id, x, y, scale, colour, flags; slots)`
    /// (cdecl, the floats as bits): a button icon (ids 600..611, 0x264..
    /// 0x267, 0x268, 0x269) drawn by `DrawCrossButtonIconRGBA` at a size
    /// from `scale` (each side rounded to an integer, then remapped), the
    /// last four moved down by a remapped `scale * 7` (`scale * 1.5` for
    /// 0x269); on map 0xb8 icon 0x261 moves right by 5 in language 0xb
    /// (and skips the rest), icon 0x268 by 2.5 either way. Other ids draw
    /// at size 0. The original keeps `x` and `y` in their slots and its
    /// scratch values in `id`'s. Returns the draw's `eax`.
    #[allow(clippy::too_many_arguments)] // the original's six and the slots
    pub fn draw_scale_button_icon(
        h: &mut impl Host,
        slots: u32,
        id: u32,
        x: u32,
        y: u32,
        scale: u32,
        colour: u32,
        flags: u32,
    ) -> u32 {
        let mut icon = Icon {
            slots,
            scale: x87::load(scale),
            x,
            y,
            w: 0,
            ht: 0,
        };
        let mut skip = false;
        if id.wrapping_sub(600) < 0xc {
            let product = x87::mul(icon.scale, constant(h, SIZE_54));
            let i = x87::ftol(h, x87::add(constant(h, HALF), product));
            h.write(slots, i);
            let f = x87::store(f64::from(i as i32));
            h.write(slots, f);
            icon.w = x87::store(h.call_sym_st0_f64(UI_REMAP_X2, Conv::Cdecl, &[f]));
            icon.ht = x87::store(h.call_sym_st0_f64(UI_REMAP_Y2, Conv::Cdecl, &[f]));
            if map_no(h) == 0xb8 && id == 0x261 {
                skip = true;
                if lang(h) == 0xb {
                    let by = constant(h, NUDGE_5);
                    icon.nudge(h, by, true);
                }
            }
        }
        if !skip {
            if id.wrapping_sub(0x264) < 4 {
                let product = x87::mul(icon.scale, constant(h, SIZE_64));
                let i = x87::ftol(h, x87::add(constant(h, HALF), product));
                h.write(slots, i);
                let f = x87::store(f64::from(i as i32));
                h.write(slots, f);
                icon.w = x87::store(h.call_sym_st0_f64(UI_REMAP_X2, Conv::Cdecl, &[f]));
                icon.ht = icon.rounded(h, SIZE_40, UI_REMAP_Y2);
                let offset = constant(h, OFFSET_7);
                icon.lower(h, offset);
            }
            if id == 0x268 {
                let offset = constant(h, OFFSET_7);
                icon.lower(h, offset);
                icon.w = icon.rounded(h, SIZE_84_5, UI_REMAP_X2);
                icon.ht = icon.rounded(h, SIZE_60, UI_REMAP_Y2);
                if map_no(h) == 0xb8 {
                    let english = lang(h) == 0xb;
                    let by = constant(h, NUDGE_2_5);
                    icon.nudge(h, by, english);
                }
            } else if id == 0x269 {
                let offset = constant(h, OFFSET_1_5);
                icon.lower(h, offset);
                icon.w = icon.rounded(h, SIZE_147_4, UI_REMAP_X2);
                icon.ht = icon.rounded(h, SIZE_50, UI_REMAP_Y2);
            }
        }
        let pass = |bits: u32| x87::store(x87::load(bits));
        h.call_sym(
            DRAW_CROSS_BUTTON_ICON,
            Conv::Cdecl,
            &[
                id,
                pass(icon.x),
                pass(icon.y),
                pass(icon.w),
                pass(icon.ht),
                colour,
                flags,
            ],
        )
    }

    fn map_no(h: &impl Host) -> u32 {
        let at = h.rebase(CURRENT_MAP_NO);
        h.read(at)
    }

    fn lang(h: &mut impl Host) -> u32 {
        h.call_sym(GET_LANG, Conv::Cdecl, &[])
    }
}
