//! A character's status plate in the menus
//! (`TOMenuDrawChrStatusPlate_008ff490`): the background, name, HP and
//! MP of a party member (`chr` 0..8) or an aeon (8..=0x11), and the
//! overdrive, Sphere Level and AP or overdrive gauge row; other numbers
//! get the empty frame only.
//!
//! Every size goes through the HD UI remap (`graphicUiRemapX2`/`Y2`,
//! `rx`/`ry` below, `f32` in and out). Positions are `x + rx(dx)` and
//! `y + ry(dy)`, formed in `f64` as the x87 holds them at the game's
//! 53-bit precision and rounded to `f32` where the original stores a
//! dword; the draw calls come in the original's order.
//!
//! Japanese, French... all share one layout; language 11 moves the
//! overdrive and Sphere Level labels (the `Label` offsets).

/// The bridges over game memory.
pub mod bridge {
    use ffx_bridge::Mem;

    /// `TOMkpShapeXYWHUV(shape, x, y, w, h, u0, v0, u1, v1)`.
    const SHAPE: u32 = 0x0090_3bb0;
    /// `TOMkpShapeXYWHUVC2(shape, x, y, w, h, u0, v0, u1, v1, c0, c1)`.
    const SHAPE_C2: u32 = 0x0090_3ee0;
    /// `DrawWaterWaveShapeC2(x, y, w, h, u, v, uw, vh, su, sv, c0, c1)`.
    const WAVE: u32 = 0x008e_7d30;
    /// `TOMenuGetYesNoWindow()`: the members drawn on the water sheet.
    const YES_NO_WINDOW: u32 = 0x008a_9b60;
    /// `TOGetFFXLang()`.
    const LANG: u32 = 0x008a_c2f0;
    /// `TkMenuGetHP(chr)`, `TkMenuGetMaxHP`, `TkMenuGetMP`,
    /// `TkMenuGetMaxMP`, `TkMenuGetSP`, `TkMenuGetLimit`,
    /// `TkMenuGetLimitMax` (menu module).
    const HP: u32 = 0x008a_98c0;
    const MAX_HP: u32 = 0x008a_9990;
    const MP: u32 = 0x008a_9970;
    const MAX_MP: u32 = 0x008a_99b0;
    const SP: u32 = 0x008a_9b80;
    const LIMIT: u32 = 0x008a_9930;
    const LIMIT_MAX: u32 = 0x008a_9950;
    /// `MenuPlayer_DamageClass2(chr)`: the name's alpha.
    const DAMAGE_CLASS: u32 = 0x008a_9a70;
    /// `TOGetSaveChrName(chr)`.
    const CHR_NAME: u32 = 0x008a_c850;
    /// `ToMakeBtlEasyEdgeFont(text, x, y, alpha, scale, extra)`.
    const EDGE_FONT: u32 = 0x0090_5930;
    /// The CRT `sprintf` import slot.
    const SPRINTF_SLOT: u32 = 0x00b0_c3dc;
    /// `"%5d/%5d"` and `"%4d/%4d"`.
    const HP_FORMAT: u32 = 0x00b6_684c;
    const MP_FORMAT: u32 = 0x00b6_6854;
    /// `TOMkpCrossEasyStrFontSClut(text, x, y, alpha, scale, extra)`.
    const CROSS_TEXT: u32 = 0x0090_1660;
    /// `TkMn2DrawSummonPlateCheckHP(x, y, chr)` (menu module): a fallen
    /// aeon's plate.
    const FALLEN_AEON: u32 = 0x008c_eb00;
    /// `ToGetBtlEasyDigitWidth2(value, &width, scale, extra)`.
    const DIGIT_WIDTH: u32 = 0x0090_5230;
    /// `ToMakeBtlEasyEdgeDigit2(value, x, y, alpha, scale, extra)`.
    const EDGE_DIGIT: u32 = 0x0090_5820;
    /// `ToMakeBtlEasyDigitRight(value, x, y, alpha, scale, extra)`.
    const DIGIT_RIGHT: u32 = 0x0090_55c0;
    /// `MsGetNextAP(chr)` and `MsGetSaveNextAP(chr)` (battle module).
    const NEXT_AP: u32 = 0x0078_4e90;
    const SAVE_NEXT_AP: u32 = 0x0078_52b0;
    /// `TkMn2DrawCrossLimitParam(x, y, w, h, limit, max)` (menu module).
    const LIMIT_GAUGE: u32 = 0x008d_3690;

    /// The texts' scale and extra (`0.68`, 0) and the name's (`0.82`, 1).
    const TEXT_SCALE: f32 = 0.68;
    const NAME_SCALE: f32 = 0.82;
    /// The water sheet's colours: clear, and two alphas of the same blue.
    const WATER_CLEAR: u32 = 0x00c5_6870;
    const WATER_LIGHT: u32 = 0x1ac5_6870;
    const WATER_DARK: u32 = 0x33c5_6870;
    /// The aeon gauges' grey.
    const GREY: u32 = 0x8080_8080;

    /// A UV of the 1024-unit shape atlas.
    const fn atlas(n: f64) -> f64 {
        n / 1024.0
    }

    /// A UV of the 1024-unit shape atlas, as the dword constant.
    const fn a32(n: f32) -> f32 {
        n / 1024.0
    }

    /// `x` rounded to a dword.
    #[allow(clippy::cast_possible_truncation)] // fstp dword
    fn dword(x: f64) -> f32 {
        x as f32
    }

    /// The label offsets language 11 moves (zero elsewhere): the
    /// overdrive and Sphere Level labels' widths and UVs, the gap before
    /// the HP and MP numbers, and the AP label.
    #[derive(Clone, Copy, Default)]
    struct Label {
        /// Widens the label shapes 0x21/0x22/0xc8 (`[ebp-0x30]`).
        wide: f32,
        /// Heightens shape 0x21 (`[ebp-0x48]`).
        high: f32,
        /// Moves the HP and MP numbers right (`[ebp-0x34]`).
        gap: f32,
        /// The AP label (shape 0x3d): x, U, width and U end
        /// (`[ebp-0x44]`, `[ebp-0x40]`, `[ebp-0x38]`, `[ebp-0x3c]`).
        ap: [f32; 4],
    }

    /// One plate being drawn.
    struct Plate<'a, M, R, C> {
        mem: &'a mut M,
        remap: R,
        call: C,
        chr: u32,
        x: f32,
        y: f32,
        /// What the original leaves in `eax`: the last call's result.
        eax: u32,
    }

    #[allow(clippy::many_single_char_names)]
    impl<M: Mem, R, C> Plate<'_, M, R, C>
    where
        R: FnMut(&mut M, bool, f32) -> f32,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        fn rx(&mut self, v: f32) -> f32 {
            (self.remap)(self.mem, false, v)
        }

        fn ry(&mut self, v: f32) -> f32 {
            (self.remap)(self.mem, true, v)
        }

        /// `x + rx(dx)`.
        fn at_x(&mut self, dx: f32) -> f32 {
            dword(f64::from(self.rx(dx)) + f64::from(self.x))
        }

        /// `y + ry(dy)`.
        fn at_y(&mut self, dy: f32) -> f32 {
            dword(f64::from(self.ry(dy)) + f64::from(self.y))
        }

        /// The original function at `sym` (absolute address through
        /// `call`); its `eax`.
        fn run(&mut self, sym: u32, args: &[u32]) -> u32 {
            let f = self.mem.rebase(sym);
            self.eax = (self.call)(self.mem, f, args);
            self.eax
        }

        /// A `TOMkpShapeXYWHUV` quad `rx(w)` x `ry(h)` at `(x, y)` (the
        /// height remapped first, as the original does).
        fn shape(&mut self, shape: u32, x: f32, y: f32, [w, h]: [f32; 2], uv: [f32; 4]) {
            let h = self.ry(h);
            let w = self.rx(w);
            let [u0, v0, u1, v1] = uv.map(f32::to_bits);
            let args = [
                shape,
                x.to_bits(),
                y.to_bits(),
                w.to_bits(),
                h.to_bits(),
                u0,
                v0,
                u1,
                v1,
            ];
            self.run(SHAPE, &args);
        }

        /// A grey `TOMkpShapeXYWHUVC2` quad, as [`Plate::shape`].
        fn shape_c2(&mut self, shape: u32, x: f32, y: f32, [w, h]: [f32; 2], uv: [f32; 4]) {
            let h = self.ry(h);
            let w = self.rx(w);
            let [u0, v0, u1, v1] = uv.map(f32::to_bits);
            let args = [
                shape,
                x.to_bits(),
                y.to_bits(),
                w.to_bits(),
                h.to_bits(),
                u0,
                v0,
                u1,
                v1,
                GREY,
                GREY,
            ];
            self.run(SHAPE_C2, &args);
        }

        /// One 250 x 64 water-sheet quad at `x` (`DrawWaterWaveShapeC2`),
        /// sheet U `u`, scrolling by `(su, sv)`, fading `c0` to `c1`.
        fn wave(&mut self, x: f32, u: f32, scroll: [f32; 2], colours: [u32; 2]) {
            let h = self.ry(64.0);
            let w = self.rx(250.0);
            let args = [
                x.to_bits(),
                self.y.to_bits(),
                w.to_bits(),
                h.to_bits(),
                u.to_bits(),
                64f32.to_bits(),
                250f32.to_bits(),
                64f32.to_bits(),
                scroll[0].to_bits(),
                scroll[1].to_bits(),
                colours[0],
                colours[1],
            ];
            self.run(WAVE, &args);
        }

        /// The water sheet across the plate: two 250-wide strips side by
        /// side scrolling one way, and two the other way under them.
        fn water(&mut self) {
            for (u, scroll, alpha) in [
                (0.0, [50.0, -20.0], WATER_LIGHT),
                (300.0, [-80.0, -10.0], WATER_DARK),
            ] {
                self.wave(self.x, u, scroll, [WATER_CLEAR, alpha]);
                let right = self.at_x(250.0);
                self.wave(right, u + 250.0, scroll, [alpha, WATER_CLEAR]);
            }
        }

        /// The background: a party member drawn on the water (bit `chr`
        /// of the yes/no window mask) gets the clear sheet and the water,
        /// one not on it the dark sheet; aeons and others the plain sheet
        /// 0xfffffffa over the water. Then the right-hand panel.
        fn background(&mut self) {
            let [x, y] = [self.x, self.y];
            if self.chr.cast_signed() < 8 {
                let mask = self.run(YES_NO_WINDOW, &[]);
                if mask & 1u32.wrapping_shl(self.chr) != 0 {
                    self.shape(0xffff_fffd, x, y, [1110.0, 72.0], [0.0, 0.0, 1110.0, 72.0]);
                    self.water();
                } else {
                    self.shape(
                        0xffff_fffd,
                        x,
                        y,
                        [1110.0, 72.0],
                        [0.0, 700.0, 1110.0, 772.0],
                    );
                }
                self.right_panel();
            } else {
                self.shape(0xffff_fffa, x, y, [1110.0, 70.0], [0.0, 0.0, 1024.0, 71.0]);
                self.right_panel();
                self.water();
            }
        }

        /// The panel behind the gauges, 1090 units in.
        fn right_panel(&mut self) {
            let x = self.at_x(1090.0);
            let y = self.y;
            self.shape(
                0xffff_fffd,
                x,
                y,
                [415.0, 70.0],
                [689.0, 618.0, 1104.0, 688.0],
            );
        }

        /// `TOGetFFXLang()` 11 moves the AP label (its width offset
        /// differs between the party row and the empty row).
        fn label_offsets(&mut self, label: &mut Label, ap_width: f32) {
            if self.run(LANG, &[]) == 0xb {
                label.ap = [-10.0, -35.0, ap_width, 20.0];
            }
        }

        /// The overdrive (0x21) and Sphere Level (0x22) labels.
        fn label_shapes(&mut self, label: Label) -> [f32; 2] {
            let (wide, high) = (f64::from(label.wide), f64::from(label.high));
            let uv_w = dword(atlas(wide + 436.0));
            let uv_h = dword(atlas(41.0 + high));
            let h = dword(high + 23.0);
            let w = dword(wide + 61.5);
            let y = self.at_y(7.0);
            let x = self.at_x(490.0);
            self.shape(0x21, x, y, [w, h], [a32(356.0), a32(15.0), uv_w, uv_h]);
            [x, y]
        }

        fn sphere_label(&mut self, label: Label) -> [f32; 2] {
            let wide = f64::from(label.wide);
            let uv_w = dword(atlas(wide + 570.0));
            let w = dword(wide + 49.0);
            let y = self.at_y(34.0);
            let x = self.at_x(540.0);
            self.shape(
                0x22,
                x,
                y,
                [w, 23.0],
                [a32(510.0), a32(15.0), uv_w, a32(41.0)],
            );
            [x, y]
        }

        /// `sprintf(text, format, value, max)` into the scratch text, then
        /// `TOMkpCrossEasyStrFontSClut` at `(x + rx(dx) + gap, y +
        /// ry(dy))`.
        fn ratio(
            &mut self,
            text: u32,
            format: u32,
            [value, max]: [u32; 2],
            [dx, dy]: [f32; 2],
            gap: f32,
        ) {
            let sprintf: u32 = self.mem.read(self.mem.rebase(SPRINTF_SLOT));
            let format = self.mem.rebase(format);
            self.eax = (self.call)(self.mem, sprintf, &[text, format, value, max]);
            let y = self.at_y(dy);
            let x = dword(f64::from(self.rx(dx)) + f64::from(self.x) + f64::from(gap));
            let args = [
                text,
                x.to_bits(),
                y.to_bits(),
                0x25,
                TEXT_SCALE.to_bits(),
                0,
            ];
            self.run(CROSS_TEXT, &args);
        }

        /// The name, the two labels and `HP/max HP`, `MP/max MP` (all but
        /// fallen aeons).
        fn name_row(&mut self, scratch: u32) -> Label {
            let chr = self.chr;
            let alpha = self.run(DAMAGE_CLASS, &[chr]);
            let y = self.at_y(9.0);
            let x = self.at_x(160.0);
            let name = self.run(CHR_NAME, &[chr]);
            let args = [
                name,
                x.to_bits(),
                y.to_bits(),
                alpha,
                NAME_SCALE.to_bits(),
                1f32.to_bits(),
            ];
            self.run(EDGE_FONT, &args);
            let mut label = Label::default();
            if self.run(LANG, &[]) == 0xb {
                label.wide = 22.0;
                label.high = 1.0;
                label.gap = 11.0;
            }
            self.label_shapes(label);
            let max = self.run(MAX_HP, &[chr]);
            let hp = self.run(HP, &[chr]);
            self.ratio(scratch, HP_FORMAT, [hp, max], [604.0, 8.0], label.gap);
            self.sphere_label(label);
            let max = self.run(MAX_MP, &[chr]);
            let mp = self.run(MP, &[chr]);
            self.ratio(scratch, MP_FORMAT, [mp, max], [625.0, 36.0], label.gap);
            label
        }

        /// The party member's gauge row: the overdrive frame and bar, the
        /// AP label, Sphere Levels (centred on 1076 by their width, which
        /// `ToGetBtlEasyDigitWidth2` writes over the gap word) and the AP
        /// still needed for the next level.
        fn party_row(&mut self, mut label: Label, gap_word: u32) {
            let chr = self.chr;
            let (x, y) = (self.at_x(1096.0), self.at_y(2.0));
            self.shape(
                0xc8,
                x,
                y,
                [384.0, 28.0],
                [a32(376.0), a32(127.0), a32(760.0), a32(156.0)],
            );
            let (x, y) = (self.at_x(844.0), self.at_y(28.0));
            self.shape(0xffff_ffff, x, y, [208.0, 33.0], [0.0, 603.0, 208.0, 636.0]);
            let y = dword(f64::from(self.y) - f64::from(self.ry(3.0)));
            let x = self.at_x(1038.0);
            self.shape(
                0x3d,
                x,
                y,
                [75.0, 75.0],
                [a32(498.0), a32(603.0), a32(573.0), a32(678.0)],
            );
            self.label_offsets(&mut label, 25.0);
            self.ap_label(label, 952.0);
            let sp = self.run(SP, &[chr]);
            let scale = TEXT_SCALE.to_bits();
            self.run(DIGIT_WIDTH, &[sp, gap_word, scale, 0]);
            let width: f32 = self.mem.read(gap_word);
            let y = self.at_y(16.0);
            let x = dword(f64::from(self.rx(1076.0)) + f64::from(self.x) - f64::from(width) * 0.5);
            let sp = self.run(SP, &[chr]);
            self.run(EDGE_DIGIT, &[sp, x.to_bits(), y.to_bits(), 0, scale, 0]);
            self.level_label(label);
            let (x, y) = (self.at_x(1333.0), self.at_y(28.0));
            let next = self.run(NEXT_AP, &[chr]);
            let saved = self.run(SAVE_NEXT_AP, &[chr]);
            let args = [
                next.wrapping_sub(saved),
                x.to_bits(),
                y.to_bits(),
                0x25,
                scale,
                0,
            ];
            self.run(DIGIT_RIGHT, &args);
        }

        /// The AP label (shape 0x3d) at `x + rx(base + ap x)`.
        fn ap_label(&mut self, label: Label, base: f64) {
            let [ax, au, aw, au_end] = label.ap.map(f64::from);
            let uv_end = dword(atlas(au_end + 978.0));
            let u = dword(atlas(au + 834.0));
            let w = dword(aw + 117.0);
            let y = self.at_y(35.0);
            let x = self.at_x(dword(ax + base));
            self.shape(0x3d, x, y, [w, 26.0], [u, a32(84.0), uv_end, a32(116.0)]);
        }

        /// The Sphere Level label (shape 0xc8) at 1348.
        fn level_label(&mut self, label: Label) {
            let wide = f64::from(label.wide);
            let uv_w = dword(atlas(wide + 434.0));
            let w = dword(wide + 55.0);
            let (x, y) = (self.at_x(1348.0), self.at_y(34.0));
            self.shape(
                0xc8,
                x,
                y,
                [w, 28.0],
                [a32(382.0), a32(77.0), uv_w, a32(105.0)],
            );
        }

        /// The aeon's overdrive gauge row.
        fn aeon_row(&mut self) {
            let chr = self.chr;
            let (x, y) = (self.at_x(1102.0), self.at_y(4.0));
            self.shape_c2(0xffff_ffff, x, y, [332.0, 32.0], [274.0, 61.0, 606.0, 93.0]);
            let (x, y) = (self.at_x(1136.0), self.at_y(8.0));
            self.shape_c2(
                0x190,
                x,
                y,
                [201.0, 24.0],
                [a32(64.0), a32(467.0), a32(343.0), a32(504.0)],
            );
            let max = self.run(LIMIT_MAX, &[chr]);
            let limit = self.run(LIMIT, &[chr]);
            let (x, y) = (self.at_x(1130.0), self.at_y(40.0));
            let (h, w) = (self.ry(8.0), self.rx(350.0));
            self.run(
                LIMIT_GAUGE,
                &[
                    x.to_bits(),
                    y.to_bits(),
                    w.to_bits(),
                    h.to_bits(),
                    limit,
                    max,
                ],
            );
        }

        /// A number above 0x11: the labels and frames without values.
        fn empty_row(&mut self) {
            let mut label = Label::default();
            if self.run(LANG, &[]) == 0xb {
                label.wide = 22.0;
                label.high = 1.0;
            }
            self.label_shapes(label);
            self.sphere_label(label);
            let (x, y) = (self.at_x(1096.0), self.at_y(2.0));
            self.shape(
                0xc8,
                x,
                y,
                [384.0, 28.0],
                [a32(376.0), a32(127.0), a32(760.0), a32(156.0)],
            );
            let (x, y) = (self.at_x(842.0), self.at_y(28.0));
            self.shape(0xffff_ffff, x, y, [208.0, 33.0], [0.0, 603.0, 208.0, 636.0]);
            let y = dword(f64::from(self.y) - f64::from(self.ry(5.0)));
            let x = self.at_x(1034.0);
            self.shape(
                0x3d,
                x,
                y,
                [75.0, 75.0],
                [a32(498.0), a32(603.0), a32(573.0), a32(678.0)],
            );
            self.label_offsets(&mut label, 20.0);
            self.ap_label(label, 948.0);
            self.level_label(label);
        }
    }

    /// The scratch the plate hands its callees: the numbers' text (32
    /// bytes, `[ebp-0x24]`) and the Sphere Level width word
    /// (`[ebp-0x34]`), 0x24 bytes.
    pub const SCRATCH_BYTES: u32 = 0x24;

    /// `TOMenuDrawChrStatusPlate_008ff490(chr, x, y)` (`0x008ff490`): the
    /// status plate of character `chr` at `(x, y)` (see the module).
    /// After the background, characters above 0x11 (unsigned) get the
    /// empty labels; an aeon (8..=0x11) with no HP left
    /// `TkMn2DrawSummonPlateCheckHP(ftol(x), ftol(y), chr)` instead of
    /// the name row; then party members their gauge row and aeons theirs.
    /// `remap(mem, y_axis, v)` is `graphicUiRemapY2`/`X2`; `call` takes
    /// absolute addresses. Returns what the original leaves in `eax`.
    pub fn draw_chr_status_plate<M: Mem>(
        mem: &mut M,
        [chr, x, y]: [u32; 3],
        scratch: u32,
        remap: impl FnMut(&mut M, bool, f32) -> f32,
        call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::ftol;
        let mut p = Plate {
            mem,
            remap,
            call,
            chr,
            x: f32::from_bits(x),
            y: f32::from_bits(y),
            eax: 0,
        };
        p.background();
        if chr > 0x11 {
            p.empty_row();
            return p.eax;
        }
        let aeon = chr.cast_signed() >= 8;
        let mut label = Label::default();
        if aeon && p.run(HP, &[chr]).cast_signed() <= 0 {
            let fy = ftol(p.mem, f64::from(p.y));
            let fx = ftol(p.mem, f64::from(p.x));
            p.run(FALLEN_AEON, &[fx, fy, chr]);
        } else {
            let text = scratch;
            label = p.name_row(text);
        }
        if aeon {
            p.aeon_row();
        } else {
            let gap_word = scratch.wrapping_add(0x20);
            p.mem.write(gap_word, label.gap);
            p.party_row(label, gap_word);
        }
        p.eax
    }
}
