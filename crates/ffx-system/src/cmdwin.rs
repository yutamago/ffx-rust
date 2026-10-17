//! The battle command window: `TOBtlDrawCommandWindow` (`0x0089f370`), the
//! list of commands, skills or items the player picks from in every battle.
//!
//! A TO window (`g_Windows`) drawn in three parts:
//!
//! - **Panel animation** (two or more columns only): the pulsing cursor
//!   plate on the selected cell, three plate rows that slide in from the
//!   left over 0.1 s each (staggered by 0.1 s) with a gloss strip, and, from
//!   0.3 s on, the water ripples over each row (`DrawWaterWaveShapeC2`).
//! - **Frame**: the scroll bar (`DrawCrossMenuScrollParts`), the scissor of
//!   the window's box and the window's own draw step (`+0x88`).
//! - **Rows**: each visible command's parts: an icon, a count (items, MP
//!   cost, HP, MP) or the label (`QuickRecoverMenuDraw`).
//!
//! Layout is computed here, at the original's x87 precision (`f64` for the
//! registers, rounded to `f32` where the original stores a dword); the draw
//! calls stay the original functions. The sizes come from the image's
//! constants and go through the HD UI remap (`graphicUiRemapX2`/`Y2`).

/// The bridges over game memory.
#[allow(clippy::many_single_char_names)] // x, y, w, h: boxes; m, r: metrics, remap
pub mod bridge {
    use ffx_bridge::Mem;
    use ffx_bridge::x87::{add, div, ftol, ftol2, load, mul, store, sub};

    /// `graphicGetTime()`, seconds in ST0.
    const GET_TIME: u32 = 0x0064_1410;
    /// `TOMkpShapeXYWHUVC2(shape, x, y, w, h, u0, v0, u1, v1, c0, c1)`.
    const SHAPE_C2: u32 = 0x0090_3ee0;
    /// `DrawWaterWaveShapeC2(x, y, w, h, u, v, uw, vh, su, sv, c0, c1)`.
    const WATER: u32 = 0x008e_7d30;
    /// `DrawCrossMenuScrollParts(x, y, w, h, top, rows, total)`.
    const SCROLL_PARTS: u32 = 0x008e_6cc0;
    /// `TOMakePktScissor(x0, y0, x1, y1)`.
    const SCISSOR: u32 = 0x008f_dee0;
    /// `TOCheckBtlCommandUse(chr, command)`.
    const CHECK_USE: u32 = 0x0089_aca0;
    /// `MsGetComData(command, &strings)`.
    const COM_DATA: u32 = 0x0079_a4b0;
    /// `MsGetSaveItemNum(item)`.
    const ITEM_COUNT: u32 = 0x0079_04b0;
    /// `MsGetCommandMP(chr, record)`.
    const COMMAND_MP: u32 = 0x0078_cf70;
    /// `MsGetRamChrHP(chr)`.
    const CHR_HP: u32 = 0x0079_add0;
    /// `MsGetRamChrMP(chr)`.
    const CHR_MP: u32 = 0x0079_ae50;
    /// `MsGetSaveWeapon(id, &unused)`.
    const SAVE_WEAPON: u32 = 0x007a_bbd0;
    /// `DrawCrossMenuIconXYWHRGBA(x, y, w, h, id, r, g, b, a)`.
    const ICON: u32 = 0x008e_6af0;
    /// `ToMakeBtlEasyDigitRight(value, x, y, alpha, scale, extra)`.
    const DIGITS: u32 = 0x0090_55c0;
    /// `QuickRecoverMenuDraw(text, x, y, w, alpha, scale, extra, align,
    /// scroll, slot)`.
    const LABEL: u32 = 0x0090_4ba0;

    /// The sheet image, shape -1 of `TOMkpShapeXYWHUVC2`.
    const SHEET: u32 = u32::MAX;
    /// The plate rows' grey (alpha 0x60) and the gloss's (0x40).
    const PLATE_GREY: u32 = 0x6080_8080;
    const GLOSS_GREY: u32 = 0x4080_8080;
    /// The ripples' fade: transparent grey to half grey.
    const CLEAR: u32 = 0x0080_8080;
    /// The second weapon icon, shown when its +4 and +6 bytes agree.
    const WEAPON_MARK: u32 = 0x31;
    /// A command slot without a command.
    const NO_COMMAND: u16 = 0xff;
    /// `TOCheckBtlCommandUse`'s "hidden" answer.
    const HIDDEN: i32 = -4;
    /// The panel's row pitch (PS2 units).
    const ROW_PITCH: i32 = 0x34;
    /// The panel's three rows.
    const PANEL_ROWS: i32 = 3;

    /// The image constants the window reads, by meaning: `f32` dwords
    /// unless the address is marked as an `f64` qword.
    #[derive(Clone, Copy)]
    struct Metrics {
        /// 154 (`0x00b5f344`): the panel's left edge, remapped; also the
        /// scroll bar's height.
        panel_x: f32,
        /// 835 (`0x00b5f2b8`, f64): the panel's top.
        panel_top: f64,
        /// The cursor plate: height 50 (`0x00b1f2c8`, also the plate rows'
        /// `v1`), UV 1619, 105 to 1920, 156 (`0x00b5f43c`, `0x00b5f324`,
        /// `0x00b4a1ac`, `0x00b5f34c`).
        plate_h: f32,
        cursor_uv: [f32; 4],
        /// The cursor's pulse: `cos(t × 5)` (`0x00b41d20`) × 32 + 96
        /// (`0x00b412f8`, `0x00b5ee40`, f64).
        pulse: [f64; 3],
        /// The plate rows' `u1` 438 (`0x00b5f3c0`).
        plate_u1: f32,
        /// A row's slide: 0.1 s (`0x00b921f0`, f64).
        slide: f64,
        /// The gloss strip: top 838 (`0x00b5f2c0`, f64), size 234 × 44
        /// (`0x00b5f370`, `0x00b5f300`), `u` 1553 to 1788 (`0x00b5f438`,
        /// `0x00b5f44c`), `v` 53 per row (`0x00b5f1f0`) + 270 to + 315
        /// (`0x00b5f260`, `0x00b5f278`, f64).
        gloss_top: f64,
        gloss_wh: [f32; 2],
        gloss_u: [f32; 2],
        gloss_v: [f64; 3],
        /// When the ripples start: 0.3 s (`0x00b3eb44`).
        ripples_from: f32,
        /// The ripples: lower strip at 880 (`0x00b5f2d0`, f64); height and
        /// `vh` 3 (`0x00b3eb94`), `uw` 210 (`0x00b5ef1c`), speeds 40
        /// (`0x00b43db4`); `v` 20 and 50 per row (`0x00b49968`,
        /// `0x00b1f2c0`, f64), the right tile's `u` 20 per row and both
        /// tiles' second half + 210 (`0x00b49970`, f64).
        ripple_low: f64,
        ripple_h: f32,
        ripple_uw: f32,
        ripple_speed: f32,
        ripple_v: [f64; 2],
        ripple_u: f64,
        ripple_u_half: f64,
        /// 0.5 (`0x00b92220`) and 1.5 (`0x00b41d00`), f64.
        half: f64,
        one_half: f64,
        /// The scroll bar: x 150 (`0x00b48388`), y 835 (`0x00b5f400`),
        /// width 8 (`0x00b92314`).
        scroll: [f32; 3],
        /// The row icons: size 29 × 36 (`0x00b5ee78`, `0x00b5f2f8`), x
        /// offset 37 (`0x00b5f2fc`), y offset 3 for commands (`0x00b3eb94`)
        /// and 2 for weapons (`0x00b92268`).
        icon_wh: [f32; 2],
        icon_dx: f32,
        icon_dy: [f32; 2],
        /// The counts' right margin 28 (`0x00b5ee74`) and the battle font's
        /// scale 0.78 (`0x00b5ee10`).
        digits_margin: f32,
        font_scale: f32,
        /// The label's x with an icon in front: 72 (`0x00b5f318`).
        label_x: f32,
    }

    impl Metrics {
        fn read<M: Mem>(mem: &M) -> Self {
            let f = |a: u32| -> f32 { mem.read(mem.rebase(a)) };
            let d = |a: u32| -> f64 { mem.read(mem.rebase(a)) };
            Self {
                panel_x: f(0x00b5_f344),
                panel_top: d(0x00b5_f2b8),
                plate_h: f(0x00b1_f2c8),
                cursor_uv: [
                    f(0x00b5_f43c),
                    f(0x00b5_f324),
                    f(0x00b4_a1ac),
                    f(0x00b5_f34c),
                ],
                pulse: [d(0x00b4_1d20), d(0x00b4_12f8), d(0x00b5_ee40)],
                plate_u1: f(0x00b5_f3c0),
                slide: d(0x00b9_21f0),
                gloss_top: d(0x00b5_f2c0),
                gloss_wh: [f(0x00b5_f370), f(0x00b5_f300)],
                gloss_u: [f(0x00b5_f438), f(0x00b5_f44c)],
                gloss_v: [d(0x00b5_f1f0), d(0x00b5_f260), d(0x00b5_f278)],
                ripples_from: f(0x00b3_eb44),
                ripple_low: d(0x00b5_f2d0),
                ripple_h: f(0x00b3_eb94),
                ripple_uw: f(0x00b5_ef1c),
                ripple_speed: f(0x00b4_3db4),
                ripple_v: [d(0x00b4_9968), d(0x00b1_f2c0)],
                ripple_u: d(0x00b4_9968),
                ripple_u_half: d(0x00b4_9970),
                half: d(0x00b9_2220),
                one_half: d(0x00b4_1d00),
                scroll: [f(0x00b4_8388), f(0x00b5_f400), f(0x00b9_2314)],
                icon_wh: [f(0x00b5_ee78), f(0x00b5_f2f8)],
                icon_dx: f(0x00b5_f2fc),
                icon_dy: [f(0x00b3_eb94), f(0x00b9_2268)],
                digits_margin: f(0x00b5_ee74),
                font_scale: f(0x00b5_ee10),
                label_x: f(0x00b5_f318),
            }
        }
    }

    /// The fields of the TO window the command window reads.
    #[derive(Clone, Copy)]
    struct Window {
        /// `+0x08`: the acting character.
        chr: i16,
        /// `+0x24`: how many command IDs (`u16`) the list at `+0x20` has.
        /// (Read as they are used: the list, `+0x28` the row's parts (byte
        /// pairs from `+0x2c`, the kind in the second) and `+0x2a`, 1 to
        /// show zero counts.)
        count: i32,
        /// `+0x38` columns, `+0x3a` visible rows, `+0x40` first visible
        /// entry, `+0x42` the cursor.
        cols: i16,
        rows: i16,
        first: i16,
        cursor: i16,
        /// `+0x62` the cell width, `+0x66` the label's.
        pitch: i16,
        label_w: i16,
        /// `+0x68..+0x74` the box (f32 x0, y0, x1, y1), `+0x7c` the rows'
        /// top.
        frame: [u32; 4],
        top: u32,
        /// `+0x88` the window's draw step.
        draw: u32,
        /// `+0xdc` when it opened (0: not yet), `+0xe4` labels after an
        /// icon (non-zero) or centred (0).
        opened: u32,
        after_icon: u32,
    }

    impl Window {
        fn read<M: Mem>(mem: &M, w: u32) -> Self {
            let h = |o: u32| -> i16 { mem.read(w.wrapping_add(o)) };
            let u = |o: u32| -> u32 { mem.read(w.wrapping_add(o)) };
            Self {
                chr: h(8),
                count: u(0x24).cast_signed(),
                cols: h(0x38),
                rows: h(0x3a),
                first: h(0x40),
                cursor: h(0x42),
                pitch: h(0x62),
                label_w: h(0x66),
                frame: [u(0x68), u(0x6c), u(0x70), u(0x74)],
                top: u(0x7c),
                draw: u(0x88),
                opened: u(0xdc),
                after_icon: u(0xe4),
            }
        }
    }

    /// `fild dword; fstp dword; fld dword`: an integer rounded to `f32`.
    #[allow(clippy::cast_precision_loss)] // the dword store rounds
    fn f32_of(v: i32) -> f64 {
        f64::from(v as f32)
    }

    /// `idiv`: the quotient and remainder (0 for a zero divisor, where the
    /// original faults).
    fn idiv(a: i32, b: i32) -> (i32, i32) {
        if b == 0 {
            (0, 0)
        } else {
            (a.wrapping_div(b), a.wrapping_rem(b))
        }
    }

    /// [`f32_of`] as a dword argument.
    fn f32_bits(v: i32) -> u32 {
        store(f32_of(v))
    }

    /// The bits of an image constant as a dword argument.
    fn arg(v: f32) -> u32 {
        store(f64::from(v))
    }

    /// `graphicUiRemapX2`/`Y2` (`true` for y).
    type Remap<'a> = &'a mut dyn FnMut(bool, f32) -> f32;

    /// The cursor plate's colour: grey with alpha `trunc(c × 32 + 96)`,
    /// `c` the f32 cosine of the time × 5 (`fistp qword`, chopped).
    fn cursor_colour(m: &Metrics, cos: f64) -> u32 {
        let c = load(store(cos));
        let alpha = ftol2(add(mul(c, m.pulse[1]), m.pulse[2]));
        (alpha << 24).wrapping_add(0x0080_8080)
    }

    /// The cursor plate on cell `at` (cursor less first visible entry) of
    /// a window `pitch` wide per column.
    fn cursor_plate(
        m: &Metrics,
        at: i32,
        cols: i32,
        pitch: i32,
        colour: u32,
        r: Remap,
    ) -> [u32; 11] {
        let (row, col) = idiv(at, cols);
        let h = r(true, m.plate_h);
        let y = r(
            true,
            f32::from_bits(store(add(f32_of(row.wrapping_mul(ROW_PITCH)), m.panel_top))),
        );
        let x = store(add(
            f64::from(r(false, m.panel_x)),
            f32_of(col.wrapping_mul(pitch)),
        ));
        let [u0, v0, u1, v1] = m.cursor_uv.map(arg);
        [
            SHEET,
            x,
            y.to_bits(),
            f32_bits(pitch),
            h.to_bits(),
            u0,
            v0,
            u1,
            v1,
            colour,
            colour,
        ]
    }

    /// Where a sliding piece is at time `t` into its row: from `-pitch` to
    /// `end`, `(left + extra) t / 0.1 - pitch`, held inside the range.
    fn slide(m: &Metrics, left: f64, extra: f64, t: u32, pitch: i32, end: f64) -> u32 {
        let wd = f64::from(pitch);
        let x = store(sub(div(mul(add(left, extra), load(t)), m.slide), wd));
        let lo = f32_of(pitch.wrapping_neg());
        let xf = load(x);
        if lo > xf {
            store(lo)
        } else if end < xf {
            store(end)
        } else {
            x
        }
    }

    /// Panel row `k` at `t` seconds into its slide: the plate (which
    /// stops at the panel's left), its gloss strip, and the second plate
    /// one cell behind (which stops a cell further).
    fn panel_row(m: &Metrics, k: i32, t: u32, pitch: i32, r: Remap) -> [[u32; 11]; 3] {
        let wd = f64::from(pitch);
        let left = f64::from(r(false, m.panel_x));
        let first = slide(m, left, wd, t, pitch, left);
        let row = f32_of(k.wrapping_mul(ROW_PITCH));
        let plate_y = r(true, f32::from_bits(store(add(row, m.panel_top)))).to_bits();
        let plate = |x: u32, h: f32| {
            [
                SHEET,
                x,
                plate_y,
                f32_bits(pitch),
                h.to_bits(),
                0,
                0,
                arg(m.plate_u1),
                arg(m.plate_h),
                PLATE_GREY,
                PLATE_GREY,
            ]
        };
        let h = r(true, m.plate_h);
        let near = plate(first, h);
        let band = mul(f32_of(k), m.gloss_v[0]);
        let gloss_h = r(true, m.gloss_wh[1]);
        let gloss_w = r(false, m.gloss_wh[0]);
        let gloss_y = r(true, f32::from_bits(store(add(row, m.gloss_top))));
        let gloss = [
            SHEET,
            first,
            gloss_y.to_bits(),
            gloss_w.to_bits(),
            gloss_h.to_bits(),
            arg(m.gloss_u[0]),
            store(add(band, m.gloss_v[1])),
            arg(m.gloss_u[1]),
            store(add(band, m.gloss_v[2])),
            GLOSS_GREY,
            GLOSS_GREY,
        ];
        let left = f64::from(r(false, m.panel_x));
        let second = slide(
            m,
            left,
            f32_of(pitch.wrapping_mul(2)),
            t,
            pitch,
            add(left, wd),
        );
        let h = r(true, m.plate_h);
        [near, gloss, plate(second, h)]
    }

    /// The ripples over panel row `k`: an upper and a lower strip, each two
    /// tiles of two half-cell pieces that fade in and out
    /// (`DrawWaterWaveShapeC2` arguments, in the original's order).
    #[allow(clippy::similar_names)]
    fn ripple_row(m: &Metrics, k: i32, pitch: i32, r: Remap) -> [[u32; 12]; 8] {
        let wd = f64::from(pitch);
        let next = f32_of(k.wrapping_add(1));
        let this = f32_of(k);
        let row = f32_of(k.wrapping_mul(ROW_PITCH));
        // the upper strip's v, the left tile's u (always 0)
        let v_up = store(mul(m.ripple_v[0], next));
        let u_left = mul(0.0, this);
        let y_up = store(add(row, m.panel_top));
        let u_left_half = store(add(u_left, m.ripple_u_half));
        let v_low = store(mul(next, m.ripple_v[1]));
        let y_low = store(add(row, m.ripple_low));
        let u_right = mul(this, m.ripple_u);
        let u_right_half = store(add(u_right, m.ripple_u_half));
        let width = store(mul(wd, m.half));
        let h = r(true, m.ripple_h).to_bits();
        let left = f64::from(r(false, m.panel_x));
        let xs = [
            store(left),
            store(add(left, mul(wd, m.half))),
            store(add(left, wd)),
            store(add(left, mul(wd, m.one_half))),
        ];
        let ys = [y_up, y_low].map(|y| r(true, f32::from_bits(y)).to_bits());
        let tiles = [
            ([store(u_left), u_left_half], [xs[0], xs[1]]),
            ([store(u_right), u_right_half], [xs[2], xs[3]]),
        ];
        let mut out = [[0u32; 12]; 8];
        let mut n = 0;
        for (us, xs) in tiles {
            for (y, v) in [(ys[0], v_up), (ys[1], v_low)] {
                for (half, colours) in [(0, [CLEAR, GLOSS_GREY]), (1, [GLOSS_GREY, CLEAR])] {
                    out[n] = [
                        xs[half],
                        y,
                        width,
                        h,
                        us[half],
                        v,
                        arg(m.ripple_uw),
                        arg(m.ripple_h),
                        arg(m.ripple_speed),
                        arg(m.ripple_speed),
                        colours[0],
                        colours[1],
                    ];
                    n += 1;
                }
            }
        }
        out
    }

    /// `DrawCrossMenuScrollParts`'s arguments: the bar right of the panel,
    /// the first visible line (`first / cols`), the visible lines and the
    /// line count (its low word, sign-extended).
    fn scroll_bar(m: &Metrics, w: &Window, lines: u32, r: Remap) -> [u32; 7] {
        let h = r(true, m.panel_x);
        let width = r(false, m.scroll[2]);
        let y = r(true, m.scroll[1]);
        let x = store(add(f64::from(r(false, m.scroll[0])), f64::from(w.pitch)));
        let top = idiv(i32::from(w.first), i32::from(w.cols)).0;
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        // cwde of the low word
        let lines = i32::from(lines as u16 as i16);
        [
            x,
            y.to_bits(),
            width.to_bits(),
            h.to_bits(),
            top.cast_unsigned(),
            i32::from(w.rows).cast_unsigned(),
            lines.cast_unsigned(),
        ]
    }

    /// The visible entries: from the first visible one (0 at least, the
    /// count at most) for `cols × rows` entries, within the count.
    fn visible(w: &Window, count: i32) -> (i32, i32) {
        let first = i32::from(w.first);
        let start = if first > count { count } else { first.max(0) };
        let end = i32::from(w.cols)
            .wrapping_mul(i32::from(w.rows))
            .wrapping_add(start);
        let end = if end > count { count } else { end.max(0) };
        (start, end)
    }

    /// A row icon (`DrawCrossMenuIconXYWHRGBA`) at the cell's `x`, `y`.
    fn icon(m: &Metrics, x: i32, y: i32, id: u32, dy: f32, r: Remap) -> [u32; 9] {
        let h = r(true, m.icon_wh[1]);
        let w = r(false, m.icon_wh[0]);
        let top = store(add(f64::from(r(true, dy)), f32_of(y)));
        let left = store(add(f64::from(r(false, m.icon_dx)), f32_of(x)));
        [
            left,
            top,
            w.to_bits(),
            h.to_bits(),
            id,
            0x80,
            0x80,
            0x80,
            0x80,
        ]
    }

    /// A count right-aligned in the cell (`ToMakeBtlEasyDigitRight`).
    fn count(m: &Metrics, value: u32, right: i32, y: i32, alpha: u32, r: Remap) -> [u32; 6] {
        let x = store(sub(f32_of(right), f64::from(r(false, m.digits_margin))));
        [
            value,
            x,
            f32_bits(y),
            alpha,
            arg(m.font_scale),
            1f32.to_bits(),
        ]
    }

    /// The command's label (`QuickRecoverMenuDraw`): after an icon from the
    /// cell's left, or centred in the cell; it scrolls when selected.
    #[allow(clippy::too_many_arguments)]
    fn label(
        m: &Metrics,
        w: &Window,
        text: u32,
        x: i32,
        y: i32,
        alpha: u32,
        selected: bool,
        r: Remap,
    ) -> [u32; 10] {
        let (left, align) = if w.after_icon != 0 {
            (store(add(f64::from(r(false, m.label_x)), f32_of(x))), 0)
        } else {
            (store(add(mul(f64::from(w.pitch), m.half), f32_of(x))), 1)
        };
        [
            text,
            left,
            f32_bits(y),
            f32_bits(i32::from(w.label_w)),
            alpha,
            arg(m.font_scale),
            1f32.to_bits(),
            align,
            u32::from(selected),
            0,
        ]
    }

    /// The part kinds of a row (the second byte of each `+0x2c` pair).
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Part {
        CommandIcon,
        ItemCount,
        MpCost,
        Hp,
        Mp,
        WeaponIcon,
        Label,
    }

    impl Part {
        fn of(kind: u8) -> Option<Self> {
            Some(match kind {
                1 => Self::CommandIcon,
                2 => Self::ItemCount,
                4 => Self::MpCost,
                5 => Self::Hp,
                6 => Self::Mp,
                7 => Self::WeaponIcon,
                0x10 => Self::Label,
                _ => return None,
            })
        }
    }

    /// `TOBtlDrawCommandWindow(w)` (`0x0089f370`, cdecl): draws the battle
    /// command window `w`; returns 0.
    ///
    /// - `+0xe0` becomes the time since `+0xdc` (`graphicGetTime`, through
    ///   `fcall`), 0 while `+0xdc` is not positive.
    /// - With two or more columns: the cursor plate when the cursor is in
    ///   the panel's three rows (alpha pulsing with `cos(5t)`, through
    ///   `cos`), the three sliding plate rows (row `k` at `+0xe0 - 0.1k`),
    ///   and from `+0xe0` ≥ 0.3 the ripples.
    /// - The scroll bar, the scissor of the box `+0x68` (each edge
    ///   truncated), the window's `+0x88` step (`call_at`, an absolute
    ///   address).
    /// - Each visible entry not `0xff` that `TOCheckBtlCommandUse` does not
    ///   hide (-4): its parts.
    ///
    /// `scratch` (8 bytes) stands for two words of the original's frame:
    /// `+0` the strings pointer `MsGetComData` and `MsGetSaveWeapon` write,
    /// `+4` the word whose low byte is the counts' alpha (and the label's
    /// alpha argument). The panel's loops leave 0x9c there (156, the end
    /// of their row offset) or, after the ripples, the bits of 150.0; a
    /// label sets the low byte to whether the command is unusable. With
    /// fewer than two columns it starts as whatever the word holds (stack
    /// garbage in the original).
    #[allow(clippy::too_many_lines, clippy::too_many_arguments)]
    pub fn to_btl_draw_command_window<M: Mem>(
        mem: &mut M,
        w: u32,
        scratch: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
        mut call_at: impl FnMut(&mut M, u32, &[u32]) -> u32,
        mut fcall: impl FnMut(&mut M, u32, &[u32]) -> f64,
        mut cos: impl FnMut(&mut M, f64) -> f64,
        mut remap: impl FnMut(&mut M, bool, f32) -> f32,
    ) -> u32 {
        let m = Metrics::read(mem);
        let win = Window::read(mem, w);
        let opened = load(win.opened);
        let elapsed = if 0.0 < opened {
            store(sub(fcall(mem, GET_TIME, &[]), opened))
        } else {
            0
        };
        mem.write(w.wrapping_add(0xe0), elapsed);
        let columns = i32::from(win.cols);
        let lines = idiv(win.count.wrapping_sub(1).wrapping_add(columns), columns).0 & 0xffff;
        let strings = scratch;
        let mut alpha: u32 = mem.read(scratch.wrapping_add(4));
        if win.cols >= 2 {
            let at = i32::from(win.cursor) - i32::from(win.first);
            if at > -1 && at < columns.wrapping_mul(PANEL_ROWS) {
                let t = fcall(mem, GET_TIME, &[]);
                let c = cos(mem, mul(t, m.pulse[0]));
                let colour = cursor_colour(&m, c);
                let quad = {
                    let mut r = |y, v| remap(mem, y, v);
                    cursor_plate(&m, at, columns, i32::from(win.pitch), colour, &mut r)
                };
                call(mem, SHAPE_C2, &quad);
            }
            let mut t = elapsed;
            for k in 0..PANEL_ROWS {
                let pitch = i32::from(mem.read::<i16>(w.wrapping_add(0x62)));
                let quads = {
                    let mut r = |y, v| remap(mem, y, v);
                    panel_row(&m, k, t, pitch, &mut r)
                };
                for quad in quads {
                    call(mem, SHAPE_C2, &quad);
                }
                t = store(sub(load(t), m.slide));
            }
            alpha = (PANEL_ROWS * ROW_PITCH).cast_unsigned();
            let now = load(mem.read(w.wrapping_add(0xe0)));
            if now >= f64::from(m.ripples_from) {
                for k in 0..PANEL_ROWS {
                    let pitch = i32::from(mem.read::<i16>(w.wrapping_add(0x62)));
                    let pieces = {
                        let mut r = |y, v| remap(mem, y, v);
                        ripple_row(&m, k, pitch, &mut r)
                    };
                    for piece in pieces {
                        call(mem, WATER, &piece);
                    }
                    alpha = pieces[2][5];
                }
            }
        }
        let bar = {
            let mut r = |y, v| remap(mem, y, v);
            scroll_bar(&m, &win, lines.cast_unsigned(), &mut r)
        };
        call(mem, SCROLL_PARTS, &bar);
        let frame =
            [0x68u32, 0x6c, 0x70, 0x74].map(|o| ftol(mem, load(mem.read(w.wrapping_add(o)))));
        call(mem, SCISSOR, &frame);
        call_at(mem, win.draw, &[w]);
        let win = Window::read(mem, w);
        let (start, end) = visible(&win, win.count);
        for i in start..end {
            let entry = |mem: &M| -> i16 {
                let list: u32 = mem.read(w.wrapping_add(0x20));
                mem.read(list.wrapping_add(i.cast_unsigned().wrapping_mul(2)))
            };
            let command = entry(mem);
            if command.cast_unsigned() == NO_COMMAND {
                continue;
            }
            let chr = i32::from(mem.read::<i16>(w.wrapping_add(8))).cast_unsigned();
            let usable =
                call(mem, CHECK_USE, &[chr, i32::from(command).cast_unsigned()]).cast_signed();
            if usable == HIDDEN {
                continue;
            }
            let id = i32::from(entry(mem)).cast_unsigned();
            let record = call(mem, COM_DATA, &[id, strings]);
            let win = Window::read(mem, w);
            let text = u32::from(mem.read::<u16>(record)).wrapping_add(mem.read(strings));
            let selected = i == i32::from(win.cursor);
            let columns = i32::from(win.cols);
            let x = idiv(i, columns)
                .1
                .wrapping_mul(i32::from(win.pitch))
                .wrapping_add(ftol(mem, load(win.frame[0])).cast_signed());
            let line = f32_of(
                idiv(i.wrapping_sub(i32::from(win.first)), columns)
                    .0
                    .wrapping_mul(ROW_PITCH),
            );
            let y = remap(mem, true, f32::from_bits(store(line)));
            let y = ftol(mem, add(f64::from(y), load(win.top))).cast_signed();
            let mut k = 0i32;
            while k < i32::from(mem.read::<i16>(w.wrapping_add(0x28))) {
                let kind: u8 = mem.read(
                    w.wrapping_add(0x2d)
                        .wrapping_add(k.cast_unsigned().wrapping_mul(2)),
                );
                k += 1;
                let Some(part) = Part::of(kind) else {
                    continue;
                };
                let win = Window::read(mem, w);
                let value = match part {
                    Part::ItemCount => Some(call(
                        mem,
                        ITEM_COUNT,
                        &[i32::from(entry(mem)).cast_unsigned()],
                    )),
                    Part::MpCost => Some(call(
                        mem,
                        COMMAND_MP,
                        &[i32::from(win.chr).cast_unsigned(), record],
                    )),
                    Part::Hp => Some(call(mem, CHR_HP, &[i32::from(entry(mem)).cast_unsigned()])),
                    Part::Mp => Some(call(mem, CHR_MP, &[i32::from(entry(mem)).cast_unsigned()])),
                    _ => None,
                };
                if let Some(value) = value {
                    if value.cast_signed() > 0 || mem.read::<i16>(w.wrapping_add(0x2a)) == 1 {
                        let right =
                            i32::from(mem.read::<i16>(w.wrapping_add(0x62))).wrapping_add(x);
                        let args = {
                            let mut r = |y, v| remap(mem, y, v);
                            count(&m, value, right, y, alpha & 0xff, &mut r)
                        };
                        call(mem, DIGITS, &args);
                    }
                    continue;
                }
                match part {
                    Part::CommandIcon => {
                        let id = u32::from(mem.read::<u8>(record.wrapping_add(0x14)));
                        let args = {
                            let mut r = |y, v| remap(mem, y, v);
                            icon(&m, x, y, id, m.icon_dy[0], &mut r)
                        };
                        call(mem, ICON, &args);
                    }
                    Part::WeaponIcon => {
                        let id = i32::from(entry(mem)).cast_unsigned();
                        let weapon = call(mem, SAVE_WEAPON, &[id, strings]);
                        let b = |mem: &M, o: u32| -> u8 { mem.read(weapon.wrapping_add(o)) };
                        let id = b(mem, 4)
                            .wrapping_mul(2)
                            .wrapping_add(1)
                            .wrapping_add(b(mem, 5));
                        let args = {
                            let mut r = |y, v| remap(mem, y, v);
                            icon(&m, x, y, u32::from(id), m.icon_dy[1], &mut r)
                        };
                        call(mem, ICON, &args);
                        if b(mem, 4) == b(mem, 6) {
                            let args = {
                                let mut r = |y, v| remap(mem, y, v);
                                icon(&m, x, y, WEAPON_MARK, m.icon_dy[1], &mut r)
                            };
                            call(mem, ICON, &args);
                        }
                    }
                    Part::Label => {
                        alpha = (alpha & !0xff) | u32::from(usable < 0);
                        let args = {
                            let mut r = |y, v| remap(mem, y, v);
                            label(&m, &win, text, x, y, alpha, selected, &mut r)
                        };
                        call(mem, LABEL, &args);
                    }
                    _ => {}
                }
            }
        }
        mem.write(scratch.wrapping_add(4), alpha);
        0
    }
}
