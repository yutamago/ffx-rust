//! The menu plates: `TODrawMenuPlateXYWHType` (`0x008f5f70`), the framed
//! backgrounds of the menus and windows.
//!
//! A plate is made of a few kinds of pieces, all sized through the HD UI
//! remap (`graphicUiRemapX2`/`Y2`, written `rx`/`ry` below):
//!
//! - **Bands**: a row of three `TODrawCrossBoxXYWHC2` gradient boxes over
//!   the plate's three columns. The columns are a left cap `rx(cap)` wide,
//!   the middle (the width less `rx(2 cap)`) and a right cap. The caps fade
//!   to their edge colours. The banded plates draw a drop shadow under the
//!   plate, the body, a white highlight one line down and a dark line along
//!   the bottom.
//! - **Sheet images**: shape -4 (`TOMkpShapeXYWHUVC2`), the plate texture.
//!   Its UVs map 1920 × 1080 screen units onto the 512 × 416 sheet, either
//!   stretched over the whole plate or cut into the same three columns.
//! - **Nine-slice frames**: eight shape-0xc8 pieces (`TOMkpShapeXYWHUV`)
//!   with 7-unit corners around the box.
//! - Single shapes and the `0x008f9c20` and `0x008f4df0` panels.
//!
//! The coordinates keep the original's rounding: sums and differences
//! are formed in `f64`, as the x87 holds them at the game's 53-bit
//! precision, and rounded to `f32` where the original stores a dword.
//! The calls come in the original's order.

/// The bridges over game memory.
pub mod bridge {
    use ffx_bridge::Mem;

    /// `TODrawCrossBoxXYWHC2(x, y, w, h, c0, c1)`.
    const CROSS_BOX: u32 = 0x008f_4b20;
    /// `TOMkpShapeXYWHUV(shape, x, y, w, h, u0, v0, u1, v1)`.
    const SHAPE: u32 = 0x0090_3bb0;
    /// `TOMkpShapeXYWHUVC2(shape, x, y, w, h, u0, v0, u1, v1, c0, c1)`.
    const SHAPE_C2: u32 = 0x0090_3ee0;
    /// The framed panel `0x008f9c20(x, y, w, h, kind)`.
    const FRAME: u32 = 0x008f_9c20;
    /// The ability-map panel `0x008f4df0(x, y, w, h, c0, c1)`.
    const PANEL: u32 = 0x008f_4df0;
    /// `graphicGetAbmap()`.
    const GET_ABMAP: u32 = 0x0064_1060;

    /// The sheet image, shape -4.
    const SHEET: u32 = 0xffff_fffc;
    /// The texture atlas of `TOMkpShapeXYWHUV`'s shape 0xc8.
    const ATLAS: u32 = 0xc8;
    /// Grey, opaque and transparent.
    const GREY: u32 = 0x8080_8080;
    const GREY_CLEAR: u32 = 0x0080_8080;

    /// A UV of the 1024-unit shape atlas.
    const fn atlas(n: f32) -> f32 {
        n / 1024.0
    }

    /// A box: `x, y, w, h`.
    type Rect = [f32; 4];

    /// `a + b` at x87 precision, stored as a dword.
    fn add(a: f32, b: f32) -> f32 {
        #[allow(clippy::cast_possible_truncation)] // fstp dword
        let r = (f64::from(a) + f64::from(b)) as f32;
        r
    }

    /// `a - b` at x87 precision, stored as a dword.
    fn sub(a: f32, b: f32) -> f32 {
        #[allow(clippy::cast_possible_truncation)] // fstp dword
        let r = (f64::from(a) - f64::from(b)) as f32;
        r
    }

    /// `(a + b) - c` at x87 precision, stored once as a dword.
    fn add_sub(a: f32, b: f32, c: f32) -> f32 {
        #[allow(clippy::cast_possible_truncation)] // fstp dword
        let r = ((f64::from(a) + f64::from(b)) - f64::from(c)) as f32;
        r
    }

    /// The sheet U of screen width `w`: `1920 w / 512` (less `minus`).
    fn sheet_u(w: f32, minus: f64) -> f32 {
        #[allow(clippy::cast_possible_truncation)] // fstp dword
        let r = (1920.0 * f64::from(w) * 0.001_953_125 - minus) as f32;
        r
    }

    /// The sheet V of screen height `h`: `1080 h / 416`.
    fn sheet_v(h: f32) -> f32 {
        #[allow(clippy::cast_possible_truncation)] // fstp dword
        let r = (1080.0 * f64::from(h) / 416.0) as f32;
        r
    }

    /// One plate being drawn: the box, the remap and the draw calls.
    struct Plate<'a, M, R, C> {
        mem: &'a mut M,
        remap: R,
        call: C,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    }

    #[allow(clippy::many_single_char_names)] // x, y, w, h and the rect
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

        fn cross_box(&mut self, r: Rect, c0: u32, c1: u32) {
            let [x, y, w, h] = r.map(f32::to_bits);
            (self.call)(self.mem, CROSS_BOX, &[x, y, w, h, c0, c1]);
        }

        fn shape(&mut self, shape: u32, r: Rect, uv: [f32; 4]) {
            let [x, y, w, h] = r.map(f32::to_bits);
            let [u0, v0, u1, v1] = uv.map(f32::to_bits);
            (self.call)(self.mem, SHAPE, &[shape, x, y, w, h, u0, v0, u1, v1]);
        }

        fn shape_c2(&mut self, shape: u32, r: Rect, uv: [f32; 4], c0: u32, c1: u32) {
            let [x, y, w, h] = r.map(f32::to_bits);
            let [u0, v0, u1, v1] = uv.map(f32::to_bits);
            (self.call)(
                self.mem,
                SHAPE_C2,
                &[shape, x, y, w, h, u0, v0, u1, v1, c0, c1],
            );
        }

        /// The `0x008f9c20` frame of kind `kind` over the plate.
        fn frame(&mut self, kind: u32) {
            let [x, y, w, h] = [self.x, self.y, self.w, self.h].map(f32::to_bits);
            (self.call)(self.mem, FRAME, &[x, y, w, h, kind]);
        }

        /// The `0x008f4df0` ability-map panel over the plate.
        fn panel(&mut self, c0: u32, c1: u32) {
            let [x, y, w, h] = [self.x, self.y, self.w, self.h].map(f32::to_bits);
            (self.call)(self.mem, PANEL, &[x, y, w, h, c0, c1]);
        }

        /// The three columns `[x, w]` for caps `cap` wide.
        fn columns(&mut self, cap: f32) -> [[f32; 2]; 3] {
            let side = self.rx(cap);
            let middle = sub(self.w, self.rx(2.0 * cap));
            let left_of_mid = add(self.rx(cap), self.x);
            let right = add_sub(self.x, self.w, self.rx(cap));
            [[self.x, side], [left_of_mid, middle], [right, self.rx(cap)]]
        }

        /// A band of three boxes `y, h` over the columns, the caps fading
        /// from `colours[0]` and to `colours[2]`.
        fn band(&mut self, cols: [[f32; 2]; 3], y: f32, h: f32, colours: [u32; 3]) {
            let [left, mid, right] = colours;
            self.cross_box([cols[0][0], y, cols[0][1], h], left, mid);
            self.cross_box([cols[1][0], y, cols[1][1], h], mid, mid);
            self.cross_box([cols[2][0], y, cols[2][1], h], mid, right);
        }

        /// The drop shadow, `shadow` high under the plate.
        fn shadow_band(&mut self, cols: [[f32; 2]; 3], shadow: f32) {
            let y = add(self.y, self.h);
            let h = self.ry(shadow);
            self.band(cols, y, h, [0, 0x2600_0000, 0]);
        }

        /// The white highlight one line down and the dark bottom line.
        fn edge_bands(&mut self, cols: [[f32; 2]; 3]) {
            let y = add(self.ry(1.0), self.y);
            let h = self.ry(2.0);
            self.band(cols, y, h, [0x00ff_ffff, 0x5aff_ffff, 0x00ff_ffff]);
            let bottom = add_sub(self.y, self.h, self.ry(3.0));
            let h = self.ry(3.0);
            self.band(cols, bottom, h, [0, 0x8000_0000, 0]);
        }

        /// The sheet image cut into the three columns of cap `cap`.
        fn sheet_columns(&mut self, cap: f32) {
            let v = sheet_v(self.h);
            let side = self.rx(cap);
            self.shape_c2(
                SHEET,
                [self.x, self.y, side, self.h],
                [0.0, 0.0, cap, v],
                GREY_CLEAR,
                GREY,
            );
            let mid_u = sheet_u(self.w, f64::from(cap));
            let middle = sub(self.w, self.rx(2.0 * cap));
            let mid_x = add(self.rx(cap), self.x);
            self.shape_c2(
                SHEET,
                [mid_x, self.y, middle, self.h],
                [cap, 0.0, mid_u, v],
                GREY,
                GREY,
            );
            let right_x = add_sub(self.x, self.w, self.rx(cap));
            let side = self.rx(cap);
            let end_u = sheet_u(self.w, 0.0);
            self.shape_c2(
                SHEET,
                [right_x, self.y, side, self.h],
                [mid_u, 0.0, end_u, v],
                GREY,
                GREY_CLEAR,
            );
        }

        /// The sheet image stretched over the plate.
        fn sheet_whole(&mut self, colour: u32) {
            #[allow(clippy::cast_possible_truncation)] // fstp dword
            let uv = [
                0.0,
                0.0,
                (f64::from(self.w) * 1920.0 * 0.001_953_125) as f32,
                (f64::from(self.h) * 1080.0 / 416.0) as f32,
            ];
            self.shape_c2(SHEET, [self.x, self.y, self.w, self.h], uv, colour, colour);
        }

        /// The nine-slice frame: corners, then the left, top/bottom and
        /// right edges, column by column.
        fn nine_slice(&mut self) {
            const U: [f32; 4] = [atlas(385.0), atlas(393.0), atlas(441.0), atlas(450.0)];
            const V: [f32; 4] = [atlas(415.0), atlas(424.0), atlas(471.0), atlas(480.0)];
            let (x, y, w, h) = (self.x, self.y, self.w, self.h);
            let c = self.rx(7.0);
            let r = self.ry(7.0);
            let inner_h = sub(h, self.ry(14.0));
            let mid_y = add(self.ry(7.0), y);
            let bottom = add_sub(y, h, self.ry(7.0));
            self.shape(ATLAS, [x, y, c, r], [U[0], V[0], U[1], V[1]]);
            let c = self.rx(7.0);
            self.shape(ATLAS, [x, mid_y, c, inner_h], [U[0], V[1], U[1], V[2]]);
            let (c, r) = (self.rx(7.0), self.ry(7.0));
            self.shape(ATLAS, [x, bottom, c, r], [U[0], V[2], U[1], V[3]]);
            let mid_x = add(self.rx(7.0), x);
            let inner_w = sub(w, self.rx(14.0));
            let r = self.ry(7.0);
            self.shape(ATLAS, [mid_x, y, inner_w, r], [U[1], V[0], U[2], V[1]]);
            let r = self.ry(7.0);
            self.shape(ATLAS, [mid_x, bottom, inner_w, r], [U[1], V[2], U[2], V[3]]);
            let right = add_sub(x, w, self.rx(7.0));
            let (c, r) = (self.rx(7.0), self.ry(7.0));
            self.shape(ATLAS, [right, y, c, r], [U[2], V[0], U[3], V[1]]);
            let c = self.rx(7.0);
            self.shape(ATLAS, [right, mid_y, c, inner_h], [U[2], V[1], U[3], V[2]]);
            let (c, r) = (self.rx(7.0), self.ry(7.0));
            self.shape(ATLAS, [right, bottom, c, r], [U[2], V[2], U[3], V[3]]);
        }

        /// A single atlas or sheet piece at `x + rx(dx), y + ry(dy)`.
        fn offset(&mut self, dx: f32, dy: f32) -> [f32; 2] {
            [add(self.rx(dx), self.x), add(self.ry(dy), self.y)]
        }
    }

    /// The banded plates: (cap, shadow height, body colours).
    fn banded(kind: u32) -> Option<(f32, f32, [u32; 3])> {
        Some(match kind {
            1 => (300.0, 6.0, [0, 0x5a00_0000, 0]),
            2 => (140.0, 6.0, [0, 0x6600_0000, 0]),
            4 | 5 => (160.0, 6.0, [0x007a_5255, 0x407d_4036, 0x007d_4036]),
            7 | 14 => (140.0, 4.0, [0x0070_3140, 0x4070_3140, 0x0070_3140]),
            11 => (140.0, 4.0, [0x0070_3140, 0x8070_3140, 0x0070_3140]),
            13 => (200.0, 4.0, [0x0063_2d0d, 0x4063_2d0d, 0x0063_2d0d]),
            _ => return None,
        })
    }

    /// The title strip of plates 7 and 11 (sheet `-1`, the pixels
    /// (1515, 170)..(1959, 228)): (x offset, width, colour).
    fn title_strip(kind: u32) -> Option<(f32, f32, u32)> {
        match kind {
            7 => Some((86.0, 554.0, 0x4080_8080)),
            11 => Some((456.0, 504.0, GREY)),
            _ => None,
        }
    }

    /// `TODrawMenuPlateXYWHType(x, y, w, h, kind)` (`0x008f5f70`): draws
    /// the menu plate `kind` (1-14) over the box `x, y, w, h` (f32 bits).
    /// Remaps go through `remap(y_axis, v)` (`graphicUiRemapX2`/`Y2`),
    /// the shape, frame and panel calls through `call`.
    ///
    /// - 1, 2: the sheet in three columns (caps 300 and 140), then bands.
    /// - 4/5, 7/14, 11, 13: bands only, with 7 and 11 adding a title strip
    ///   after the body and 4 an icon at the top right.
    /// - 3: a shadowed `0x008f9c20` frame with an inset atlas panel.
    /// - 6, 8: the `0x008f9c20` frame kind 4 with an atlas bar (two
    ///   variants).
    /// - 9, 10: the stretched sheet, the ability-map panel and the
    ///   nine-slice frame; 10 adds a drop shadow and is skipped when the box
    ///   is narrower than the frame's corners, and its panel colours follow
    ///   `graphicGetAbmap`.
    /// - 12: the nine-slice frame alone.
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    pub fn to_draw_menu_plate_xywh_type<M: Mem>(
        mem: &mut M,
        args: [u32; 5],
        remap: impl FnMut(&mut M, bool, f32) -> f32,
        call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let [x, y, w, h, kind] = args;
        let mut p = Plate {
            mem,
            remap,
            call,
            x: f32::from_bits(x),
            y: f32::from_bits(y),
            w: f32::from_bits(w),
            h: f32::from_bits(h),
        };
        if let Some((cap, shadow, body)) = banded(kind) {
            if kind <= 2 {
                p.sheet_columns(cap);
            }
            let cols = p.columns(cap);
            p.shadow_band(cols, shadow);
            p.band(cols, p.y, p.h, body);
            if let Some((dx, width, colour)) = title_strip(kind) {
                let [sx, sy] = p.offset(dx, 3.0);
                let (sw, sh) = (p.rx(width), p.ry(54.0));
                p.shape_c2(
                    0xffff_ffff,
                    [sx, sy, sw, sh],
                    [1515.0, 170.0, 1959.0, 228.0],
                    colour,
                    colour,
                );
            }
            p.edge_bands(cols);
            if kind == 4 {
                let ix = add(p.rx(602.0), p.x);
                let iy = sub(p.y, p.ry(2.0));
                let (iw, ih) = (p.rx(68.0), p.ry(68.0));
                p.shape(
                    ATLAS,
                    [ix, iy, iw, ih],
                    [atlas(498.0), atlas(603.0), atlas(573.0), atlas(678.0)],
                );
            }
            return;
        }
        match kind {
            3 => {
                let [sx, sy] = p.offset(6.0, 6.0);
                p.cross_box([sx, sy, p.w, p.h], 0x2400_0000, 0x2400_0000);
                p.frame(0);
                let [ix, iy] = p.offset(3.0, 3.0);
                let iw = sub(p.w, p.rx(6.0));
                let ih = sub(p.h, p.ry(6.0));
                p.shape_c2(
                    ATLAS,
                    [ix, iy, iw, ih],
                    [atlas(494.0), atlas(337.0), atlas(497.0), atlas(401.0)],
                    0x6480_8080,
                    0x6480_8080,
                );
            }
            6 | 8 => {
                p.frame(4);
                let [bx, by] = p.offset(6.0, 5.0);
                let bw = p.rx(466.0);
                let bh = sub(p.h, p.ry(10.0));
                let (v0, v1) = if kind == 6 {
                    (atlas(122.0), atlas(172.0))
                } else {
                    (atlas(172.0), atlas(223.0))
                };
                p.shape(
                    ATLAS,
                    [bx, by, bw, bh],
                    [atlas(385.0), v0, atlas(851.0), v1],
                );
            }
            9 => {
                p.sheet_whole(GREY);
                p.panel(0x668f_4b2d, 0x6617_080a);
                p.nine_slice();
            }
            10 => {
                let corners = p.rx(14.0);
                if f64::from(corners) > f64::from(p.w) {
                    return;
                }
                let [sx, sy] = p.offset(6.0, 6.0);
                p.cross_box([sx, sy, p.w, p.h], 0x401a_1a1a, 0x401a_1a1a);
                p.sheet_whole(0x4080_8080);
                let (c0, c1) = if (p.call)(p.mem, GET_ABMAP, &[]) == 0 {
                    (0x608f_2c45, 0x6050_1826)
                } else {
                    (0x6034_0614, 0x6030_0410)
                };
                p.panel(c0, c1);
                p.nine_slice();
            }
            12 => p.nine_slice(),
            _ => {}
        }
    }
}
