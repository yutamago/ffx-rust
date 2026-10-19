//! Battle-window state of the `TO` layer (`TOBtl*`): help and sign windows,
//! the cursor position stack, Lulu's limit table, target callbacks.

/// One entry of `TOBtlSetLuluLimitTbl`'s running sums of ASCII digits:
/// the previous entry (0 for the first) plus the digit's value (bytes,
/// wrapping).
#[must_use]
pub const fn lulu_limit_step(prev: u8, digit: u8) -> u8 {
    prev.wrapping_sub(0x30).wrapping_add(digit)
}

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    /// The battle help window is open (`TOBtlCloseHelpWin` clears it).
    pub const HELP_WIN: Global<u8> = Global::new(0x0133_c93c);
    /// Set by `TOBtlInitAnm`.
    pub const INIT_ANM: Global<u8> = Global::new(0x0134_0452);
    /// The sign window (`TOBtlCloseSignWindow` clears it).
    pub const SIGN_WINDOW: Global<u32> = Global::new(0x0133_f520);
    /// The simple help message (`TOBtlCloseSimpleHelpMes` clears it).
    pub const SIMPLE_HELP: Global<u32> = Global::new(0x0133_f510);
    /// `TOGetBtlWindowControlEnable`'s flag.
    pub const WINDOW_CONTROL: Global<u32> = Global::new(0x00c5_8ef8);
    /// Cursor positions pushed (`i16`, at most 4).
    pub const CUR_POS_COUNT: Global<i16> = Global::new(0x0133_d6f0);
    /// The cursor positions: 8-byte records, x at `+0`, y at `+4`, z at `+6`.
    pub const CUR_POS: Global<[u16; 4]> = Global::new(0x0133_f7b4);
    /// Lulu's limit table (16 bytes).
    pub const LULU_LIMIT: Global<[u8; 16]> = Global::new(0x0133_f798);

    /// `TOBtlCloseHelpWin()` (`0x00890eb0`).
    pub fn close_help_win(mem: &mut impl Mem) {
        HELP_WIN.set(mem, 0);
    }

    /// `TOBtlInitAnm()` (`0x008a0cd0`).
    pub fn init_anm(mem: &mut impl Mem) {
        INIT_ANM.set(mem, 1);
    }

    /// `TOBtlCloseSignWindow()` (`0x00890ee0`).
    pub fn close_sign_window(mem: &mut impl Mem) {
        SIGN_WINDOW.set(mem, 0);
    }

    /// `TOBtlCloseSimpleHelpMes()` (`0x00890ef0`).
    pub fn close_simple_help(mem: &mut impl Mem) {
        SIMPLE_HELP.set(mem, 0);
    }

    /// `TOGetBtlWindowControlEnable()` (`0x0089b590`).
    pub fn window_control_enable(mem: &impl Mem) -> u32 {
        WINDOW_CONTROL.get(mem)
    }

    /// `TOBtlSetCurPos(x, y, z)` (`0x008999f0`): pushes a cursor position
    /// while fewer than 4 are pushed (the count is signed).
    pub fn set_cur_pos(mem: &mut impl Mem, x: u16, y: u16, z: u16) {
        let count = CUR_POS_COUNT.get(mem);
        if count >= 4 {
            return;
        }
        let r = CUR_POS
            .ptr(mem)
            .addr
            .wrapping_add(i32::from(count).cast_unsigned().wrapping_mul(8));
        mem.write(r, x);
        mem.write(r.wrapping_add(4), y);
        mem.write(r.wrapping_add(6), z);
        CUR_POS_COUNT.set(mem, count.wrapping_add(1));
    }

    /// `TOBtlSetLuluLimitTbl(digits)` (`0x0089a2f0`): 16 entries, each
    /// read back from the table and the digits as the original goes.
    pub fn set_lulu_limit_table(mem: &mut impl Mem, digits: u32) {
        let t = LULU_LIMIT.ptr(mem).addr;
        for i in 0..16 {
            let prev = if i == 0 { 0 } else { mem.read::<u8>(t + i - 1) };
            let d = mem.read::<u8>(digits.wrapping_add(i));
            mem.write(t + i, super::lulu_limit_step(prev, d));
        }
    }

    /// `TOBtlSetTargetCallback(w)` (`0x008a1090`): the window's target
    /// handlers (`+0x90`, `+0x98`, `+0x9c`, `+0xa0`; `+0x94` cleared), code
    /// addresses of this image.
    pub fn set_target_callback(mem: &mut impl Mem, w: u32) {
        let at = |o: u32| w.wrapping_add(o);
        mem.write(at(0x94), 0u32);
        for (o, code) in [
            (0x98, 0x008a_8780),
            (0x9c, 0x008a_8be0),
            (0xa0, 0x008a_2ac0),
            (0x90, 0x008a_84c0),
        ] {
            let c = mem.rebase(code);
            mem.write(at(o), c);
        }
    }

    /// What `TOCloseBtlWindow` clears: a word, then nine words.
    pub const BTL_WINDOW_OPEN: Global<u32> = Global::new(0x023c_c08c);
    /// The nine words after it.
    pub const BTL_WINDOW_STATE: Global<[u32; 9]> = Global::new(0x0133_d74c);

    /// `TOCloseBtlWindow()` (`0x0089ade0`): returns 0 (left in `eax`).
    pub fn close_btl_window(mem: &mut impl Mem) -> u32 {
        BTL_WINDOW_OPEN.set(mem, 0);
        BTL_WINDOW_STATE.set(mem, [0; 9]);
        0
    }

    /// The battle command-window stack (`0x0133c900`): 0xf0 bytes per
    /// level, indexed by [`ACTION_DEPTH`].
    pub const ACTION_STACK: Global<[u8; 0xf0]> = Global::new(0x0133_c900);
    /// The current level (`0x023cc092`, i8).
    pub const ACTION_DEPTH: Global<i8> = Global::new(0x023c_c092);
    /// Stored at `+0x58` of the level left (`0x023cc088`, u16).
    pub const ACTION_PREV_VALUE: Global<u16> = Global::new(0x023c_c088);
    /// Cleared when the level left shows command 0x311e or 0x3029
    /// (`0x0133c94c`).
    pub const ACTION_MASK: Global<u32> = Global::new(0x0133_c94c);
    /// Restored from [`ACTION_SAVED`] when going back to a level whose
    /// `+0x52` is 1 (`0x0133d0d0`, 0x140 bytes).
    pub const ACTION_LIST: Global<[u8; 0x140]> = Global::new(0x0133_d0d0);
    /// See [`ACTION_LIST`] (`0x0133d210`).
    pub const ACTION_SAVED: Global<[u8; 0x140]> = Global::new(0x0133_d210);
    /// Zeroed with that restore (`0x0133f106`, u16).
    pub const ACTION_LIST_CURSOR: Global<u16> = Global::new(0x0133_f106);

    /// `TOBtlSetPrevAction()` (`0x0089a630`): back one level of the command
    /// windows. The level left: `+0x51` 5, `+0x58` [`ACTION_PREV_VALUE`],
    /// `+0x66` 0xff, [`ACTION_MASK`] cleared for commands 0x311e and 0x3029
    /// (`+0x5c`), and when its `+0x52` is 1 the next level's `+6` takes its
    /// `+6`. The level returned to: `+0x51` 2, its cursor `+0x5b` back one
    /// when at or past `+0x5a` and positive, and when its `+0x52` is 1 the
    /// list restored. 0 at level 0 (or below), else 1.
    pub fn set_prev_action(mem: &mut impl Mem) -> u32 {
        let depth = ACTION_DEPTH.get(mem);
        if depth <= 0 {
            return 0;
        }
        let base = ACTION_STACK.ptr(mem).addr;
        let level = |d: i8| base.wrapping_add(i32::from(d).cast_unsigned().wrapping_mul(0xf0));
        let e = level(depth);
        mem.write(e + 0x51, 5u8);
        let prev = ACTION_PREV_VALUE.get(mem);
        mem.write(e + 0x58, prev);
        mem.write(e + 0x66, 0xffu16);
        let command: u16 = mem.read(e + 0x5c);
        if command == 0x311e || command == 0x3029 {
            ACTION_MASK.set(mem, 0);
        }
        if mem.read::<u8>(e + 0x52) == 1 {
            let v: u16 = mem.read(e + 6);
            mem.write(e + 0xf6, v);
        }
        let depth = depth - 1;
        ACTION_DEPTH.set(mem, depth);
        let p = level(depth);
        mem.write(p + 0x51, 2u8);
        let cursor: i8 = mem.read(p + 0x5b);
        if cursor >= mem.read::<i8>(p + 0x5a) && cursor > 0 {
            mem.write(p + 0x5b, cursor - 1);
        }
        if mem.read::<u8>(p + 0x52) == 1 {
            let saved: [u8; 0x140] = mem.read(ACTION_SAVED.ptr(mem).addr);
            let list = ACTION_LIST.ptr(mem).addr;
            mem.write(list, saved);
            ACTION_LIST_CURSOR.set(mem, 0);
        }
        1
    }

    /// The damage values shown (`0x0133f108`): an `i16` count, then from
    /// `0x0133f110` up to 63 records of 16 bytes.
    pub const DAMAGE_COUNT: Global<i16> = Global::new(0x0133_f108);
    /// The first damage record.
    pub const DAMAGE_VALUES: Global<[u8; 16]> = Global::new(0x0133_f110);

    /// `TOBtlSetDamageValue(a, b, c, d, e, f, g, h)` (`0x00899a40`): a
    /// record (`u16`, `u16`, `u32`, `u32`, four bytes) appended; the new
    /// count, or -1 with 63 already.
    pub fn set_damage_value(mem: &mut impl Mem, a: [u32; 8]) -> i32 {
        add_damage_record(mem, a, true)
    }

    /// `0x0089baa0(a, b, d, e, f, g, h)`: [`set_damage_value`] without the
    /// word at `+4` (left as it was).
    pub fn set_damage_value_short(mem: &mut impl Mem, a: [u32; 7]) -> i32 {
        add_damage_record(mem, [a[0], a[1], 0, a[2], a[3], a[4], a[5], a[6]], false)
    }

    fn add_damage_record(mem: &mut impl Mem, a: [u32; 8], word4: bool) -> i32 {
        let n = DAMAGE_COUNT.get(mem);
        if n >= 0x3f {
            return -1;
        }
        let r = DAMAGE_VALUES
            .ptr(mem)
            .addr
            .wrapping_add(i32::from(n).cast_unsigned() << 4);
        #[allow(clippy::cast_possible_truncation)] // the low halves
        {
            mem.write(r, a[0] as u16);
            mem.write(r.wrapping_add(2), a[1] as u16);
        }
        if word4 {
            mem.write(r.wrapping_add(4), a[2]);
        }
        mem.write(r.wrapping_add(8), a[3]);
        for (k, w) in (0u32..).zip(&a[4..]) {
            mem.write(r.wrapping_add(0xc + k), w.to_le_bytes()[0]);
        }
        let n = n.wrapping_add(1);
        DAMAGE_COUNT.set(mem, n);
        i32::from(n)
    }

    /// The two target lists (`0x0133f0d8`, `0x0133f0ec`): a count, then
    /// two-byte entries whose first byte is a target bit.
    pub const TARGET_LISTS: [u32; 2] = [0x0133_f0d8, 0x0133_f0ec];
    /// The masks they are OR-ed into (`0x0133d0d4`, `0x0133d0d8`) and the
    /// union (`0x0133d0dc`).
    pub const TARGET_MASKS: Global<[u32; 3]> = Global::new(0x0133_d0d4);

    /// `TOBtlMakeTargetListBit()` (`0x008985b0`): each list's target bits
    /// OR-ed into its mask (stored only for a non-empty list), the union of
    /// both into the third; returns the sum of the counts.
    pub fn make_target_list_bit(mem: &mut impl Mem) -> i32 {
        let masks = TARGET_MASKS.ptr(mem).addr;
        let mut out = [0u32; 2];
        let mut total = 0i32;
        for (k, &list) in TARGET_LISTS.iter().enumerate() {
            let list = mem.rebase(list);
            let n: i32 = mem.read(list);
            let at = masks.wrapping_add(4 * u32::try_from(k).unwrap_or(0));
            let mut m: u32 = mem.read(at);
            for i in 0..n.max(0).cast_unsigned() {
                let bit: u8 = mem.read(list.wrapping_add(4 + 2 * i));
                m |= 1u32.wrapping_shl(u32::from(bit));
            }
            if n > 0 {
                mem.write(at, m);
            }
            out[k] = m;
            total = total.wrapping_add(n);
        }
        mem.write(masks.wrapping_add(8), out[0] | out[1]);
        total
    }

    /// `BtlEnd_BuildList_008bde00()` (own name): the battle-end list.
    /// `0x01869f19` is set when any character `g_BtlCountsBattle`
    /// (`0x02310ea0`, bytes) counts has a nonzero `g_TkBtlEndResults`
    /// (`0x02310f20`, words) entry. For each counted character `i` (of 8),
    /// entry `n` (14 bytes from `0x01869f20`) gets: `+0` `BtlEnd_RowX(n,
    /// count)`, `+2` `6n`, `+4` 0, `+6` `i` (`u16`s), `+8`
    /// `TkMenuGetPlayerDamegedColor(i)`, `+9` `BtlEnd_InParty(i)`, `+0xa`
    /// 0 (bytes). The count goes to `g_TkBtlEndList` (`0x01869f18`), then
    /// `BtlEnd_CompactList` (its `eax` returned). Callees through `call`.
    pub fn btl_end_build_list<M: Mem>(
        mem: &mut M,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let counts = mem.rebase(0x0231_0ea0);
        let results = mem.rebase(0x0231_0f20);
        let any_result = mem.rebase(0x0186_9f19);
        mem.write(any_result, 0u8);
        let mut count = 0u32;
        for i in 0..8 {
            if mem.read::<u8>(counts + i) != 0 {
                if mem.read::<u32>(results + 4 * i) != 0 {
                    mem.write(any_result, 1u8);
                }
                count += 1;
            }
        }
        let mut n = 0u32;
        let mut entry = mem.rebase(0x0186_9f20);
        for i in 0..8 {
            if mem.read::<u8>(counts + i) == 0 {
                continue;
            }
            #[allow(clippy::cast_possible_truncation)] // u16 stores
            {
                mem.write(entry + 2, (n * 6) as u16);
                mem.write(entry + 4, 0u16);
                mem.write(entry + 6, i as u16);
                let colour = call(mem, 0x008b_9eb0, &[i]);
                mem.write(entry + 8, colour as u8);
                let in_party = call(mem, 0x008b_9ec0, &[i]);
                mem.write(entry + 9, in_party as u8);
                let x = call(mem, 0x008b_a370, &[n, count]);
                mem.write(entry, x as u16);
            }
            mem.write(entry + 0xa, 0u8);
            n += 1;
            entry += 0xe;
        }
        #[allow(clippy::cast_possible_truncation)] // at most 8
        mem.write(mem.rebase(0x0186_9f18), count as u8);
        call(mem, 0x008b_a400, &[])
    }

    /// `TOBtlDrawDamageValue2()` (`0x00893290`): after
    /// `TOMkpSetEnvSubFont`, draws each of the `i16` count at `0x0133f108`
    /// (read again after each) 16-byte entries from `0x0133f110` (`i16` x
    /// `+0`, y `+2`, kind `+4`, value `+8`, colour bytes `+0xc..+0xf`), then
    /// clears the count. Before each, a pending 9999 (`0x01133390`) or 99999
    /// (`0x01133394`) damage unlocks achievement 0x12 or 0x15 and is
    /// cleared. In magic 0x11b, kind 3 entries are skipped. Values 0..=9
    /// are digits (`TOMakePktBtlDamageNumRGBA(value, x, y, r, g, b, a)`),
    /// 0x3d, 0x3c, 0x3b and 0x3a icons 0..=3 (`TOMakePktBtlIconA(icon, x,
    /// y, a)`); other values draw nothing. Callees through `call`; returns
    /// 0.
    pub fn draw_damage_value2<M: Mem>(
        mem: &mut M,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let count = mem.rebase(0x0133_f108);
        let damage_9999 = mem.rebase(0x0113_3390);
        let damage_99999 = mem.rebase(0x0113_3394);
        call(mem, 0x0090_3300, &[]);
        let mut entry = mem.rebase(0x0133_f110);
        let mut i = 0i32;
        while i < i32::from(mem.read::<i16>(count)) {
            if mem.read::<u32>(damage_9999) != 0 {
                call(mem, 0x0082_2380, &[0x12]);
                mem.write(damage_9999, 0u32);
            }
            if mem.read::<u32>(damage_99999) != 0 {
                call(mem, 0x0082_2380, &[0x15]);
                mem.write(damage_99999, 0u32);
            }
            let magic = call(mem, 0x009d_a3b0, &[]);
            if magic != 0x11b || mem.read::<u32>(entry + 4) != 3 {
                let value: i32 = mem.read(entry + 8);
                let x = i32::from(mem.read::<i16>(entry)).cast_unsigned();
                let y = i32::from(mem.read::<i16>(entry + 2)).cast_unsigned();
                let colour = |m: &M, k: u32| u32::from(m.read::<u8>(entry + 0xc + k));
                if (0..=9).contains(&value) {
                    let rgba = [
                        colour(mem, 0),
                        colour(mem, 1),
                        colour(mem, 2),
                        colour(mem, 3),
                    ];
                    let args = [
                        value.cast_unsigned(),
                        x,
                        y,
                        rgba[0],
                        rgba[1],
                        rgba[2],
                        rgba[3],
                    ];
                    call(mem, 0x008f_b840, &args);
                } else {
                    let icon = match value {
                        0x3d => Some(0),
                        0x3c => Some(1),
                        0x3b => Some(2),
                        0x3a => Some(3),
                        _ => None,
                    };
                    if let Some(icon) = icon {
                        let alpha = colour(mem, 3);
                        call(mem, 0x008f_bcc0, &[icon, x, y, alpha]);
                    }
                }
            }
            i += 1;
            entry += 0x10;
        }
        mem.write(count, 0u16);
        0
    }

    /// `TOBtlDrawSummonPrim(chr, x, y)` (`0x008a0ba0`) and
    /// `TOBtlDrawChangePrim(chr, x, y)` (`0x0089f0e0`): a battle window
    /// at the integer position (`x`, `y`) with the name of character `chr`
    /// centred in it and its HP/MP plate (`TOBtlDrawCrossParamPlate`).
    /// `remap(mem, y_axis, v)` is `graphicUiRemapY2`/`X2` (an `f32`);
    /// `scratch` is the 4-byte name width `ToGetBtlEasyFontWidth` writes;
    /// callees through `call` (`ffx.sym` addresses). Returns the plate's
    /// `eax`.
    pub struct BtlPrim<'a, M, R, C> {
        pub mem: &'a mut M,
        pub remap: R,
        pub call: C,
        pub scratch: u32,
    }

    impl<M, R, C> BtlPrim<'_, M, R, C>
    where
        M: Mem,
        R: FnMut(&mut M, bool, f32) -> f32,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        fn x(&mut self, v: f32) -> f64 {
            f64::from((self.remap)(self.mem, false, v))
        }

        fn y(&mut self, v: f32) -> f64 {
            f64::from((self.remap)(self.mem, true, v))
        }

        fn run(&mut self, sym: u32, args: &[u32]) -> u32 {
            (self.call)(self.mem, sym, args)
        }

        /// The name width of `name` at `scale` (`ToGetBtlEasyFontWidth(name,
        /// &w, 0, scale, 1.0)`).
        fn name_width(&mut self, name: u32, scale: f32) -> f32 {
            let args = [name, self.scratch, 0, scale.to_bits(), 1.0f32.to_bits()];
            self.run(0x0090_5290, &args);
            self.mem.read(self.scratch)
        }

        /// `ToMakeBtlEasyEdgeFont(name, x, y, 0, scale, 1.0)`.
        fn edge_font(&mut self, name: u32, x: f32, y: f32, scale: f32) {
            let args = [
                name,
                x.to_bits(),
                y.to_bits(),
                0,
                scale.to_bits(),
                1.0f32.to_bits(),
            ];
            self.run(0x0090_5930, &args);
        }

        /// `TOBtlDrawCrossParamPlate(chr, x, y)`; its `eax`.
        fn plate(&mut self, chr: u32, x: f32, y: f32) -> u32 {
            self.run(0x0089_2cb0, &[chr, x.to_bits(), y.to_bits()])
        }

        /// `TOBtlDrawSummonPrim`: the net window (`TODrawBtlNetWindowXYWHType
        /// (x, y, X2(344), Y2(114), 0)`), the name at y `Y2(16) + y` in
        /// width scale 0.8 centred in `X2(344)` (drawn at scale 0.7), the
        /// plate at (`X2(15) + x`, `Y2(61) + y`).
        #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
        pub fn summon(&mut self, chr: u32, x: i32, y: i32) -> u32 {
            let (fx, fy) = (x as f32, y as f32);
            let h = self.y(114.0) as f32;
            let w = self.x(344.0) as f32;
            let window = [fx.to_bits(), fy.to_bits(), w.to_bits(), h.to_bits(), 0];
            self.run(0x008f_41b0, &window);
            let name = self.run(0x0089_76f0, &[chr]);
            let width = self.name_width(name, 0.8);
            let name_y = (self.y(16.0) + f64::from(fy)) as f32;
            let name_x = ((self.x(344.0) - f64::from(width)) * 0.5 + f64::from(fx)) as f32;
            self.edge_font(name, name_x, name_y, 0.7);
            let plate_y = (self.y(61.0) + f64::from(fy)) as f32;
            let plate_x = (self.x(15.0) + f64::from(fx)) as f32;
            self.plate(chr, plate_x, plate_y)
        }

        /// `TOBtlDrawChangePrim`: the frame `TOMkpShapePartIndexXYWHC2(part,
        /// 0, X2(20) + x, Y2(14) + y, X2(70), Y2(50), grey, grey)` with part
        /// 0x1c0 for an aeon (`chr >= 8`), 0x1d4 for character 6 while
        /// `AtelGetScenarioFlag() < 0x434` (signed), else `chr + 0x1b8`; the
        /// name at y `Y2(16) + y` and x `X2(210) + x` less half its width
        /// (scale 0.71 both); the plate at (`X2(12) + x`, `Y2(62) + y`).
        #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
        pub fn change(&mut self, chr: u32, x: i32, y: i32) -> u32 {
            let part = if chr.cast_signed() >= 8 {
                0x1c0
            } else if chr == 6 && self.run(0x0086_c470, &[]).cast_signed() < 0x434 {
                0x1d4
            } else {
                chr.wrapping_add(0x1b8)
            };
            let (fx, fy) = (x as f32, y as f32);
            let h = self.y(50.0) as f32;
            let w = self.x(70.0) as f32;
            let top = (self.y(14.0) + f64::from(fy)) as f32;
            let left = (self.x(20.0) + f64::from(fx)) as f32;
            let grey = 0x8080_8080;
            let frame = [
                part,
                0,
                left.to_bits(),
                top.to_bits(),
                w.to_bits(),
                h.to_bits(),
                grey,
                grey,
            ];
            self.run(0x0090_34b0, &frame);
            let name = self.run(0x0089_76f0, &[chr]);
            let width = self.name_width(name, 0.71);
            let name_y = (self.y(16.0) + f64::from(fy)) as f32;
            let name_x = (self.x(210.0) + f64::from(fx) - f64::from(width) * 0.5) as f32;
            self.edge_font(name, name_x, name_y, 0.71);
            let plate_y = (self.y(62.0) + f64::from(fy)) as f32;
            let plate_x = (self.x(12.0) + f64::from(fx)) as f32;
            self.plate(chr, plate_x, plate_y)
        }
    }

    /// The battle status window's party (`0x0133f7ac`, three `u16`
    /// characters, 0xff for none).
    pub const STATUS_PARTY: u32 = 0x0133_f7ac;
    /// The battle status window's rows (`0x0133f7d8`, three of 0x90 bytes).
    pub const STATUS_ROWS: u32 = 0x0133_f7d8;

    /// `TOBtlInitStatusWin()` (`0x00898010`): clears the three status rows,
    /// takes the party from `MsGetSaveWindow(0, 0x12, &n)` (its first three
    /// `u16`s; `n`, in `count`, is trimmed past trailing 0xff entries), and
    /// for each character `c` there (re-read before each call) fills its
    /// row: HP (`MsGetRamChrHP`) at `+0xc` and `+0x10`, MP
    /// (`MsGetRamChrMP`) at `+0x24` and `+0x28`, the limit gauge scaled to
    /// the `i16` width at `0x025d0aa4` (`gauge * width / max`, signed) at
    /// `+0x34` and `+0x38`, `TOBtlMakeStatusBuff(c, row + 0x42)` at `+0x40`
    /// (`u16`) and 0x14 at `+0x3c` (`u16`); `+0x14`, `+0x2c` (words) and
    /// `+0x18`, `+0x30` (bytes) cleared. A zero maximum (or `i32::MIN /
    /// -1`) faults in the original; here it gives 0. Callees through
    /// `call`. Returns what the original leaves in `eax`: 0x14 when the
    /// last slot holds a character, else 0xff.
    pub fn init_status_win<M: Mem>(
        mem: &mut M,
        count: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let rows = mem.rebase(STATUS_ROWS);
        for k in (0..0x1b0).step_by(4) {
            mem.write(rows + k, 0u32);
        }
        mem.write(count, 0u32);
        let window = call(mem, 0x0078_5410, &[0, 0x12, count]);
        let mut n: i32 = mem.read(count);
        while n > 0 && mem.read::<u16>(window.wrapping_add((2 * n - 2).cast_unsigned())) == 0xff {
            n -= 1;
            mem.write(count, n);
        }
        let party = mem.rebase(STATUS_PARTY);
        for k in 0..3 {
            let c: u16 = mem.read(window.wrapping_add(2 * k));
            mem.write(party + 2 * k, c);
        }
        let width = mem.rebase(0x025d_0aa4);
        let mut eax = 0;
        for k in 0..3 {
            let slot = party + 2 * k;
            let row = rows + 0x90 * k;
            let c = u32::from(mem.read::<u16>(slot));
            if c == 0xff {
                eax = 0xff;
                continue;
            }
            let hp = call(mem, 0x0079_add0, &[c]);
            mem.write(row + 0xc, hp);
            mem.write(row + 0x10, hp);
            let c = u32::from(mem.read::<u16>(slot));
            mem.write(row + 0x14, 0u32);
            mem.write(row + 0x18, 0u8);
            let mp = call(mem, 0x0079_ae50, &[c]);
            mem.write(row + 0x24, mp);
            mem.write(row + 0x28, mp);
            let c = u32::from(mem.read::<u16>(slot));
            mem.write(row + 0x2c, 0u32);
            mem.write(row + 0x30, 0u8);
            let gauge = call(mem, 0x0079_5550, &[c]).cast_signed();
            let scale = i32::from(mem.read::<i16>(width));
            let c = u32::from(mem.read::<u16>(slot));
            let max = call(mem, 0x0079_5590, &[c]).cast_signed();
            let bar = gauge.wrapping_mul(scale).checked_div(max).unwrap_or(0);
            mem.write(row + 0x34, bar);
            mem.write(row + 0x38, bar);
            let c = u32::from(mem.read::<u16>(slot));
            let buff = call(mem, 0x0089_8260, &[c, row + 0x42]);
            #[allow(clippy::cast_possible_truncation)] // the u16 store
            mem.write(row + 0x40, buff as u16);
            mem.write(row + 0x3c, 0x14u16);
            eax = 0x14;
        }
        eax
    }
}
