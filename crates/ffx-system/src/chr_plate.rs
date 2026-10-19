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
    use ffx_bridge::{Mem, x87};

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

    /// `x` stored as a dword (`fstp`, NaNs as the x87 keeps them).
    fn store32(x: f64) -> f32 {
        f32::from_bits(x87::store(x))
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
        /// Where the name row's parts sit.
        rows: Rows,
    }

    /// The name row's offsets: `ry` of the name, the overdrive label, the
    /// HP text, the Sphere Level label and the MP text, and whether the
    /// language gap goes into the numbers' `rx` (else after it).
    #[derive(Clone, Copy)]
    struct Rows {
        name: f32,
        label: f32,
        hp: f32,
        sphere: f32,
        mp: f32,
        gap_remapped: bool,
    }

    /// The status plate's rows.
    const STATUS_ROWS: Rows = Rows {
        name: 9.0,
        label: 7.0,
        hp: 8.0,
        sphere: 34.0,
        mp: 36.0,
        gap_remapped: false,
    };

    /// The small plate's rows: one unit lower, the gap remapped.
    const SMALL_ROWS: Rows = Rows {
        name: 10.0,
        label: 8.0,
        hp: 9.0,
        sphere: 35.0,
        mp: 37.0,
        gap_remapped: true,
    };

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
            self.quad(shape, [x, y, w, h], uv);
        }

        /// A `TOMkpShapeXYWHUV` quad of remapped size `(w, h)` at `(x, y)`.
        fn quad(&mut self, shape: u32, [x, y, w, h]: [f32; 4], uv: [f32; 4]) {
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

        /// A battle plate's label: `rx(w)` x `ry(h)` (height first) at
        /// `(x + rx(dx), y + ry(2))`; the wide layout adds 0.0 to `y` and
        /// takes `back` off `x` at 53 bits before each store.
        fn cross_label(
            &mut self,
            shape: u32,
            wide: bool,
            [w, h]: [f32; 2],
            dx: f32,
            back: f64,
            uv: [f32; 4],
        ) {
            let h = self.ry(h);
            let w = self.rx(w);
            let y = x87::add(f64::from(self.ry(2.0)), f64::from(self.y));
            let y = store32(if wide { x87::add(y, 0.0) } else { y });
            let x = x87::add(f64::from(self.rx(dx)), f64::from(self.x));
            let x = store32(if wide { x87::sub(x, back) } else { x });
            self.quad(shape, [x, y, w, h], uv);
        }

        /// A battle plate's number, right-aligned at `(x + rx(dx), y - ry(4))`.
        fn cross_digits(&mut self, value: u32, dx: f32, colour: u32) {
            let y = store32(x87::sub(f64::from(self.y), f64::from(self.ry(4.0))));
            let x = store32(x87::add(f64::from(self.rx(dx)), f64::from(self.x)));
            let args = [
                value,
                x.to_bits(),
                y.to_bits(),
                colour,
                BTL_DIGIT_SCALE.to_bits(),
                0,
            ];
            self.run(EDGE_DIGIT_RIGHT, &args);
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
            let y = self.at_y(self.rows.label);
            let x = self.at_x(490.0);
            self.shape(0x21, x, y, [w, h], [a32(356.0), a32(15.0), uv_w, uv_h]);
            [x, y]
        }

        fn sphere_label(&mut self, label: Label) -> [f32; 2] {
            let wide = f64::from(label.wide);
            let uv_w = dword(atlas(wide + 570.0));
            let w = dword(wide + 49.0);
            let y = self.at_y(self.rows.sphere);
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
            let x = if self.rows.gap_remapped {
                self.at_x(dword(f64::from(gap) + f64::from(dx)))
            } else {
                dword(f64::from(self.rx(dx)) + f64::from(self.x) + f64::from(gap))
            };
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
            let y = self.at_y(self.rows.name);
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
            let row = [604.0, self.rows.hp];
            self.ratio(scratch, HP_FORMAT, [hp, max], row, label.gap);
            self.sphere_label(label);
            let max = self.run(MAX_MP, &[chr]);
            let mp = self.run(MP, &[chr]);
            let row = [625.0, self.rows.mp];
            self.ratio(scratch, MP_FORMAT, [mp, max], row, label.gap);
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
            rows: STATUS_ROWS,
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

    /// `TOMenuDrawChrSmallPlate_008f51e0(chr, x, y)` (`0x008f51e0`, own
    /// name; caller `0x008c1260`): the status plate's top half, one unit
    /// lower ([`SMALL_ROWS`]). A party member (`chr` below 8, unsigned)
    /// on the yes/no window mask gets the clear sheet and the water, one
    /// not on it the dark sheet; then the name row (the language gap
    /// inside the numbers' `rx`). Any other number gets the dark sheet and
    /// the two empty labels. `remap`, `call` and `scratch` (the numbers'
    /// text) as in [`draw_chr_status_plate`]; returns the last call's
    /// `eax`.
    pub fn draw_chr_small_plate<M: Mem>(
        mem: &mut M,
        [chr, x, y]: [u32; 3],
        scratch: u32,
        remap: impl FnMut(&mut M, bool, f32) -> f32,
        call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let mut p = Plate {
            mem,
            remap,
            call,
            chr,
            x: f32::from_bits(x),
            y: f32::from_bits(y),
            eax: 0,
            rows: SMALL_ROWS,
        };
        let [x, y] = [p.x, p.y];
        let dark = [0.0, 700.0, 1110.0, 772.0];
        if chr < 8 {
            let mask = p.run(YES_NO_WINDOW, &[]);
            if mask & 1u32.wrapping_shl(chr) != 0 {
                p.shape(0xffff_fffd, x, y, [1110.0, 72.0], [0.0, 0.0, 1110.0, 72.0]);
                p.water();
            } else {
                p.shape(0xffff_fffd, x, y, [1110.0, 72.0], dark);
            }
            p.name_row(scratch);
        } else {
            p.shape(0xffff_fffd, x, y, [1110.0, 72.0], dark);
            let mut label = Label::default();
            if p.run(LANG, &[]) == 0xb {
                label.wide = 22.0;
                label.high = 1.0;
            }
            p.label_shapes(label);
            p.sphere_label(label);
        }
        p.eax
    }

    /// `TOBtlDrawCrossLimitGauge(chr, x, y, w, h)`.
    const CROSS_LIMIT_GAUGE: u32 = 0x0089_2910;
    /// `MsGetRamChrHP(chr)`, `MsGetRamChrHPmax`, `MsGetRamChrMP` and
    /// `MsCalcWeakLevel(hp, max)` (battle module).
    const BTL_HP: u32 = 0x0079_add0;
    const BTL_MAX_HP: u32 = 0x0079_adf0;
    const BTL_MP: u32 = 0x0079_ae50;
    const WEAK_LEVEL: u32 = 0x0078_bf00;
    /// `ToMakeBtlEasyEdgeDigitRight(value, x, y, colour, scale, extra)`.
    const EDGE_DIGIT_RIGHT: u32 = 0x0090_5890;
    /// The battle numbers' scale (`0.65`, extra 0.0).
    const BTL_DIGIT_SCALE: f32 = 0.65;

    /// `TOBtlDrawCrossParamPlate(chr, x, y)` (`0x00892cb0`): battle member
    /// `chr`'s small HP/MP plate at `(x, y)`: the frame (shape -1, UVs in
    /// texels), the overdrive gauge (`TOBtlDrawCrossLimitGauge(chr, x +
    /// rx(10), y + ry(29), rx(300), ry(8))`), the HP and MP labels
    /// (shapes 0xe9 and 0xea; language 11 has wider ones, `+ 0.0` on `y`
    /// and 3 or 6 off `x`, at 53 bits before the store) and both numbers
    /// right-aligned (`ToMakeBtlEasyEdgeDigitRight(n, x + rx(160 or 305),
    /// y - ry(4), colour, 0.65, 0.0)`). The HP's colour comes from
    /// `MsCalcWeakLevel(hp, max)`: 1 and 2 give 2, 3 gives 0x53, anything
    /// else 0 (the MP's is 0). `remap` and `call` as in
    /// [`draw_chr_status_plate`]; returns the last call's `eax`.
    #[allow(clippy::many_single_char_names)] // x, y and the plate
    pub fn draw_cross_param_plate<M: Mem>(
        mem: &mut M,
        [chr, x, y]: [u32; 3],
        remap: impl FnMut(&mut M, bool, f32) -> f32,
        call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let mut p = Plate {
            mem,
            remap,
            call,
            chr,
            x: f32::from_bits(x),
            y: f32::from_bits(y),
            eax: 0,
            rows: STATUS_ROWS,
        };
        let (px, py) = (f64::from(p.x), f64::from(p.y));
        p.shape(
            u32::MAX,
            p.x,
            p.y,
            [320.0, 45.0],
            [984.0, 98.0, 1581.0, 148.0],
        );
        let h = p.ry(8.0);
        let w = p.rx(300.0);
        let gy = store32(x87::add(f64::from(p.ry(29.0)), py));
        let gx = store32(x87::add(f64::from(p.rx(10.0)), px));
        p.run(
            CROSS_LIMIT_GAUGE,
            &[chr, gx.to_bits(), gy.to_bits(), w.to_bits(), h.to_bits()],
        );
        let wide = p.run(LANG, &[]) == 0xb;
        if wide {
            p.cross_label(
                0xe9,
                true,
                [68.0, 24.0],
                5.0,
                3.0,
                [a32(370.0), a32(12.0), a32(455.0), a32(42.0)],
            );
        } else {
            p.cross_label(
                0xe9,
                false,
                [61.5, 23.0],
                5.0,
                0.0,
                [a32(356.0), a32(15.0), a32(436.0), a32(41.0)],
            );
        }
        let max = p.run(BTL_MAX_HP, &[chr]);
        let hp = p.run(BTL_HP, &[chr]);
        let colour = match p.run(WEAK_LEVEL, &[hp, max]) {
            1 | 2 => 2,
            3 => 0x53,
            _ => 0,
        };
        let hp = p.run(BTL_HP, &[chr]);
        p.cross_digits(hp, 160.0, colour);
        let wide = p.run(LANG, &[]) == 0xb;
        if wide {
            p.cross_label(
                0xea,
                true,
                [66.4, 24.0],
                185.0,
                6.0,
                [a32(500.0), a32(12.0), a32(583.0), a32(42.0)],
            );
        } else {
            p.cross_label(
                0xea,
                false,
                [49.0, 23.0],
                185.0,
                0.0,
                [a32(510.0), a32(15.0), a32(570.0), a32(41.0)],
            );
        }
        let mp = p.run(BTL_MP, &[chr]);
        p.cross_digits(mp, 305.0, 0);
        p.eax
    }

    /// `TOMkpShapeXY(shape, x, y)` and `TOMkpShapeXYWHC2(shape, x, y, w,
    /// h, c0, c1)` (integer positions).
    const SHAPE_XY: u32 = 0x0090_3a10;
    const SHAPE_XYWH_C2: u32 = 0x0090_3b70;
    /// `MenuPlayer_DamageClass(chr)`: the name's alpha on the layout plate.
    const DAMAGE_CLASS1: u32 = 0x008a_99f0;
    /// `ToMakeBtlEasyFont(text, x, y, alpha, scale, extra)`.
    const BTL_EASY_FONT: u32 = 0x0090_5ab0;
    /// `TOAdpCrossNumber_009019f0(value, x, y, colour, scale, gap)`.
    const CROSS_NUMBER: u32 = 0x0090_19f0;
    /// The two sprites of an empty plate (`0x008f5e90(a, b, x, y, w, c0,
    /// c1)`).
    const EMPTY_SPRITES: u32 = 0x008f_5e90;
    /// The packet write cursor (`g_TOPktCursor`).
    const PKT_CURSOR: u32 = 0x0187_16cc;

    /// Words of the menu layout `TOSeqInitParam` fills
    /// (`0x025d164a..0x025d1672`) that place the layout plate's parts.
    mod layout {
        /// The Sphere Level number: x (with [`AP_X`]) and y (with
        /// [`AP_Y`]).
        pub const LEVEL_Y: u32 = 0x025d_164a;
        pub const LEVEL_X: u32 = 0x025d_164c;
        /// The AP number's offsets, and the 0x107 label's on the empty
        /// plate.
        pub const AP_Y: u32 = 0x025d_164e;
        pub const AP_X: u32 = 0x025d_1650;
        /// The MP number, the MP label (0x22 / 0xea), the HP number, the
        /// HP label (0x21 / 0xe9) and the name: `(y, x)` pairs.
        pub const MP_TEXT: u32 = 0x025d_1652;
        pub const MP_LABEL: u32 = 0x025d_1656;
        pub const HP_TEXT: u32 = 0x025d_165a;
        pub const HP_LABEL: u32 = 0x025d_165e;
        pub const NAME: u32 = 0x025d_1662;
        /// The frame's two corner colours.
        pub const COLOUR1: u32 = 0x025d_1668;
        pub const COLOUR0: u32 = 0x025d_166c;
        /// The empty plate's sprite width.
        pub const EMPTY_WIDTH: u32 = 0x025d_1672;
    }

    /// One shape of the layout plate's gauge row: offset from `(x, y)`
    /// (added at double precision), size (remapped or as is) and UVs.
    struct GaugePart {
        at: [f64; 2],
        size: [f32; 2],
        remapped: bool,
        uv: [f32; 4],
    }

    /// The gauge row's two bars and two labels.
    const GAUGE_PARTS: [GaugePart; 4] = [
        GaugePart {
            at: [315.0, -0.0],
            size: [60.0, 60.0],
            remapped: true,
            uv: [a32(189.0), a32(24.0), a32(362.0), a32(200.0)],
        },
        GaugePart {
            at: [295.0, 22.0],
            size: [60.0, 20.0],
            remapped: true,
            uv: [a32(830.0), a32(70.0), a32(950.0), a32(115.0)],
        },
        GaugePart {
            at: [342.0, 5.0],
            size: [25.0, 16.0],
            remapped: false,
            uv: [a32(640.0), 0.0, a32(723.0), a32(52.0)],
        },
        GaugePart {
            at: [438.0, 19.0],
            size: [24.0, 15.0],
            remapped: false,
            uv: [a32(380.0), a32(64.0), a32(476.0), a32(120.0)],
        },
    ];

    /// The layout plate being drawn: positions are integers `x + dx`
    /// (layout words) turned into `f32`, sizes go through
    /// `graphicUiRemapX`/`Y` (`remap(mem, y_axis, v)`).
    struct LayoutPlate<'a, M, R, C> {
        mem: &'a mut M,
        remap: R,
        call: C,
        x: i32,
        y: i32,
    }

    impl<M: Mem, R, C> LayoutPlate<'_, M, R, C>
    where
        R: FnMut(&mut M, bool, f32) -> f32,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        fn run(&mut self, sym: u32, args: &[u32]) -> u32 {
            let f = self.mem.rebase(sym);
            (self.call)(self.mem, f, args)
        }

        fn word(&self, at: u32) -> i32 {
            i32::from(self.mem.read::<i16>(self.mem.rebase(at)))
        }

        fn dword(&self, at: u32) -> u32 {
            self.mem.read(self.mem.rebase(at))
        }

        /// The `(x, y)` pair at layout `at` (y first, x next) added to
        /// the plate's position, as `f32` (`fild`, `fstp dword`).
        fn at(&self, at: u32) -> [f32; 2] {
            let y = self.word(at).wrapping_add(self.y);
            let x = self.word(at + 2).wrapping_add(self.x);
            [int_f32(x), int_f32(y)]
        }

        /// A `TOMkpShapeXYWHUV` quad sized `remap(w)` x `remap(h)` (the
        /// height first).
        fn remapped(&mut self, shape: u32, [x, y]: [f32; 2], [w, h]: [f32; 2], uv: [f32; 4]) {
            let h = (self.remap)(self.mem, true, h);
            let w = (self.remap)(self.mem, false, w);
            self.quad(shape, [x, y, w, h], uv);
        }

        fn quad(&mut self, shape: u32, [x, y, w, h]: [f32; 4], uv: [f32; 4]) {
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

        /// `sprintf(text, format, value, max)`, then the text at layout
        /// `at` (`TOMkpCrossEasyStrFontSClut(text, x, y, 0x25, 1.0, 0)`).
        fn ratio(&mut self, text: u32, format: u32, [value, max]: [u32; 2], at: u32) {
            let sprintf: u32 = self.dword(SPRINTF_SLOT);
            let format = self.mem.rebase(format);
            (self.call)(self.mem, sprintf, &[text, format, value, max]);
            let [x, y] = self.at(at);
            let args = [text, x.to_bits(), y.to_bits(), 0x25, 1f32.to_bits(), 0];
            self.run(CROSS_TEXT, &args);
        }

        /// A `TOAdpCrossNumber_009019f0` number at `(x, y)` (scale 1,
        /// gap -3).
        fn number(&mut self, value: u32, [x, y]: [i32; 2], colour: u32) {
            let args = [
                value,
                int_f32(x).to_bits(),
                int_f32(y).to_bits(),
                colour,
                1f32.to_bits(),
                (-3f32).to_bits(),
            ];
            self.run(CROSS_NUMBER, &args);
        }

        /// A party member's plate.
        fn member(&mut self, chr: u32, text: u32) {
            let (fx, fy) = (int_f32(self.x), int_f32(self.y));
            let colour0 = self.dword(layout::COLOUR0);
            let colour1 = self.dword(layout::COLOUR1);
            let frame = |shape: u32, x: f32, w: f32, u: [f32; 2], c: [u32; 2]| {
                [
                    shape,
                    x.to_bits(),
                    fy.to_bits(),
                    w.to_bits(),
                    38f32.to_bits(),
                    u[0].to_bits(),
                    0,
                    u[1].to_bits(),
                    a32(155.0).to_bits(),
                    c[0],
                    c[1],
                ]
            };
            let args = frame(4, fx, 142.0, [0.0, a32(28.0)], [colour0, colour1]);
            self.run(SHAPE_C2, &args);
            let colour1 = self.dword(layout::COLOUR1);
            let right = int_f32(self.x.wrapping_add(0x8e));
            let args = frame(5, right, 40.0, [a32(32.0), a32(190.0)], [colour1, colour1]);
            self.run(SHAPE_C2, &args);

            let one = 1f32.to_bits();
            let alpha = self.run(DAMAGE_CLASS1, &[chr, one, one]);
            let [x, y] = self.at(layout::NAME);
            let name = self.run(CHR_NAME, &[chr]);
            let args = [name, x.to_bits(), y.to_bits(), alpha, one, one];
            self.run(BTL_EASY_FONT, &args);

            let at = self.at(layout::HP_LABEL);
            let uv = [a32(382.0), a32(2.0), a32(454.0), a32(64.0)];
            self.remapped(0x21, at, [46.0, 26.0], uv);
            let max = self.run(MAX_HP, &[chr]);
            let hp = self.run(HP, &[chr]);
            self.ratio(text, HP_FORMAT, [hp, max], layout::HP_TEXT);
            let at = self.at(layout::MP_LABEL);
            let uv = [a32(505.0), a32(2.0), a32(586.0), a32(64.0)];
            self.remapped(0x22, at, [46.0, 26.0], uv);
            let max = self.run(MAX_MP, &[chr]);
            let mp = self.run(MP, &[chr]);
            self.ratio(text, MP_FORMAT, [mp, max], layout::MP_TEXT);

            self.gauge_row(fx, fy);

            // Sphere Levels and the AP still needed
            let ly = self.word(layout::AP_Y);
            let y = self
                .word(layout::LEVEL_Y)
                .wrapping_add(self.y)
                .wrapping_add(ly);
            let lx = self.word(layout::LEVEL_X).wrapping_add(self.x);
            let x = self.word(layout::AP_X).wrapping_sub(13).wrapping_add(lx);
            let sp = self.run(SP, &[chr]);
            self.number(sp, [x, y], 0);
            let y = self
                .word(layout::AP_Y)
                .wrapping_sub(15)
                .wrapping_add(self.y);
            let x = self
                .word(layout::AP_X)
                .wrapping_add(self.x.wrapping_add(0x8b));
            let next = self.run(NEXT_AP, &[chr]);
            let saved = self.run(SAVE_NEXT_AP, &[chr]);
            self.number(next.wrapping_sub(saved), [x, y], 0x25);
        }

        /// The gauge row: frame, two bars and two labels (sheet 0x107).
        fn gauge_row(&mut self, fx: f32, fy: f32) {
            let at = [
                int_f32(self.x.wrapping_add(0x14c)),
                int_f32(self.y.wrapping_add(5)),
            ];
            let uv = [0.0, a32(832.0), a32(576.0), a32(880.0)];
            self.quad(0x107, [at[0], at[1], 96.0, 14.0], uv);
            let (fx, fy) = (f64::from(fx), f64::from(fy));
            for part in GAUGE_PARTS {
                let at = [dword(fx + part.at[0]), dword(fy + part.at[1])];
                if part.remapped {
                    self.remapped(0x107, at, part.size, part.uv);
                } else {
                    let [w, h] = part.size;
                    self.quad(0x107, [at[0], at[1], w, h], part.uv);
                }
            }
        }

        /// Any other number: the two sprites, the HP and MP labels
        /// (0xe9, 0xea), the bar 0xe7 across the screen and label 0x107.
        fn empty(&mut self, width: i32) {
            let (x, y) = (self.x.cast_unsigned(), self.y.cast_unsigned());
            let colour0 = self.dword(layout::COLOUR0);
            let colour1 = self.dword(layout::COLOUR1);
            let w = width.cast_unsigned();
            self.run(EMPTY_SPRITES, &[0xcc, 0xcd, x, y, w, colour0, colour1]);
            for (shape, at) in [(0xe9, layout::HP_LABEL), (0xea, layout::MP_LABEL)] {
                let sy = self.word(at).wrapping_add(self.y).cast_unsigned();
                let sx = self.word(at + 2).wrapping_add(self.x).cast_unsigned();
                self.run(SHAPE_XY, &[shape, sx, sy]);
            }
            self.run(SHAPE_XYWH_C2, &[0xe7, 0, y, 0x200, 0, GREY, GREY]);
            let sy = self.word(layout::AP_Y).wrapping_add(self.y).cast_unsigned();
            let sx = (self.word(layout::AP_X) + 0x68).wrapping_add(self.x);
            self.run(SHAPE_XY, &[0x107, sx.cast_unsigned(), sy]);
        }
    }

    /// An integer as the x87 stores it after `fild` (exact) and `fstp
    /// dword`.
    #[allow(clippy::cast_precision_loss)] // rounded as fstp rounds
    fn int_f32(n: i32) -> f32 {
        n as f32
    }

    /// The scratch text the layout plate formats its numbers in (32
    /// bytes, `[ebp-0x24]`).
    pub const LAYOUT_TEXT_BYTES: u32 = 0x20;

    /// `TOMenuDrawChrLayoutPlate_008f0a70(cursor, chr, x, y)`
    /// (`0x008f0a70`): a character plate placed by the menu layout words
    /// (`TOSeqInitParam`), written at packet cursor `cursor` (stored in
    /// `g_TOPktCursor` first). A party member (`chr` below 8, unsigned)
    /// gets the frame (corner shapes 4 and 5 with the layout colours),
    /// the name (`ToMakeBtlEasyFont`, alpha `MenuPlayer_DamageClass`),
    /// the HP and MP labels and `HP/max`, `MP/max` (`"%5d/%5d"`,
    /// `"%4d/%4d"` in `text`), the gauge row of sheet 0x107 and the
    /// Sphere Level and next-AP numbers; any other number the empty
    /// plate. Positions are integers made `f32`; sizes go through
    /// `graphicUiRemapX`/`Y` (`remap(mem, y_axis, v)`); `call` takes
    /// absolute addresses. Returns `g_TOPktCursor` after drawing (the
    /// new cursor).
    pub fn draw_layout_plate<M: Mem>(
        mem: &mut M,
        [cursor, chr, x, y]: [u32; 4],
        text: u32,
        remap: impl FnMut(&mut M, bool, f32) -> f32,
        call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let pkt = mem.rebase(PKT_CURSOR);
        mem.write(pkt, cursor);
        let mut p = LayoutPlate {
            mem,
            remap,
            call,
            x: x.cast_signed(),
            y: y.cast_signed(),
        };
        let width = p.word(layout::EMPTY_WIDTH);
        if chr < 8 {
            p.member(chr, text);
        } else {
            p.empty(width);
        }
        p.mem.read(pkt)
    }
}
