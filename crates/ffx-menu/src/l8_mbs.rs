//! M5 stage 1, round 8 (list `left8`, from `battle.list`): the battle's
//! screen shake debug window (`Ms_Gui_ShakeProc`).
//!
//! Callees stay calls to the originals (cdecl). x87 code is written as
//! the formulas at 53 bits (`ffx_bridge::x87`), each value rounded to
//! `f32` where the original stores it.

/// The originals over game memory.
#[allow(clippy::many_single_char_names)] // the original's registers
pub mod bridge {
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// `Gu_WindowSetTitle(w, s)`, `Gu_WindowAddButton(w, id, s, x, y, cx,
    /// cy)`, `Gu_WindowControlSetText(w, id, s)`, `Gu_PrintSetX(x)`,
    /// `Gu_PrintSetY(y)`, `Gu_Printf(s)`, `MsAutoRepeatInit(r, a, b)`,
    /// `MsAutoRepeatCheck(r, id)`, `MsScreenShake2(...)` (9 words),
    /// `FUN_0088c740(s)`, `dbgPrintf(fmt, ...)`, the CRT's `sprintf`
    /// (import slot).
    pub const WINDOW_SET_TITLE: u32 = 0x0084_b6a0;
    pub const WINDOW_ADD_BUTTON: u32 = 0x0084_a470;
    pub const WINDOW_SET_TEXT: u32 = 0x0084_a710;
    pub const PRINT_SET_X: u32 = 0x0084_8820;
    pub const PRINT_SET_Y: u32 = 0x0084_8860;
    pub const PRINTF: u32 = 0x0084_8880;
    pub const AUTO_REPEAT_INIT: u32 = 0x007c_89a0;
    pub const AUTO_REPEAT_CHECK: u32 = 0x007c_8950;
    pub const SCREEN_SHAKE2: u32 = 0x007c_57b0;
    pub const SHAKE_LOG: u32 = 0x0088_c740;
    pub const DBG_PRINTF: u32 = 0x0062_f500;
    pub const SPRINTF_SLOT: u32 = 0x00b0_c3dc;
    /// `g_BtlScreen` (the battle screen's state pointer), the three
    /// auto-repeat records (steps of 1, 100 and 10), the window's open
    /// flag, the on/off texts (`[0x00c4468c + 4 * bit]`).
    pub const BTL_SCREEN: u32 = 0x0231_14c0;
    pub const REPEAT_1: u32 = 0x0113_f91c;
    pub const REPEAT_100: u32 = 0x0113_f930;
    pub const REPEAT_10: u32 = 0x0113_f944;
    pub const SHAKE_OPEN: u32 = 0x0113_9f08;
    pub const ON_OFF: u32 = 0x00c4_468c;
    /// The window's title and texts: the labels it prints, the value
    /// formats (`%f`-like for floats, `%d` for integers), the parameter
    /// dump's format and its log line.
    pub const SHAKE_TITLE: u32 = 0x00b5_7cc4;
    pub const SHAKE_HEAD1: u32 = 0x00b5_7d60;
    pub const SHAKE_HEAD2: u32 = 0x00b5_7d6c;
    pub const FLOAT_FMT: u32 = 0x00b5_6334;
    pub const INT_FMT: u32 = 0x00b5_7c0c;
    pub const DUMP_FMT: u32 = 0x00b5_7cf8;
    pub const DUMP_LOG: u32 = 0x00b5_7d5c;
    /// 0.01, 0.001 and 10 (doubles).
    pub const STEP_SPEED: u32 = 0x00b5_4cb8;
    pub const STEP_DECAY: u32 = 0x00b9_21b0;
    pub const TEN: u32 = 0x00b9_22b0;
    /// The window's controls: (id, text, x, y, cx), height 8; the two
    /// on/off buttons (0xf0, 0xf6) take their text from `+0x18d` /
    /// `+0x1b1` of the screen state (text 0 here).
    pub const SHAKE_BUTTONS: [(u32, u32, u32, u32, u32); 38] = [
        (0xf7, 0x00b5_7cd4, 0x6c, 0, 0x30),
        (0xf8, 0x00b5_5e14, 0xc6, 0, 0x30),
        (0xeb, 0x00b5_7cdc, 2, 0xa, 0x5a),
        (0x61, 0x00b5_5220, 0x72, 0xa, 0x12),
        (0x62, 0x00b4_8250, 0x86, 0xa, 0x12),
        (0xed, 0x00b5_7ce4, 2, 0x14, 0x5a),
        (0x63, 0x00b5_5220, 0x72, 0x14, 0x12),
        (0x64, 0x00b4_8250, 0x86, 0x14, 0x12),
        (0xec, 0x00b5_7ce8, 2, 0x1e, 0x5a),
        (0x65, 0x00b5_5220, 0x72, 0x1e, 0x12),
        (0x66, 0x00b4_8250, 0x86, 0x1e, 0x12),
        (0xee, 0x00b5_7cf0, 2, 0x28, 0x5a),
        (0x67, 0x00b5_5220, 0x72, 0x28, 0x12),
        (0x68, 0x00b4_8250, 0x86, 0x28, 0x12),
        (0xef, 0x00b1_7a68, 2, 0x32, 0x5a),
        (0xf0, 0, 0x60, 0x32, 0xe),
        (0x69, 0x00b5_5220, 0x72, 0x32, 0x12),
        (0x6a, 0x00b4_8250, 0x86, 0x32, 0x12),
        (0xf7, 0x00b5_7cd4, 0x6c, 0x3c, 0x30),
        (0xf8, 0x00b5_5e14, 0xc6, 0x3c, 0x30),
        (0xf1, 0x00b5_7cdc, 2, 0x46, 0x5a),
        (0x75, 0x00b5_5220, 0x72, 0x46, 0x12),
        (0x76, 0x00b4_8250, 0x86, 0x46, 0x12),
        (0xf3, 0x00b5_7ce4, 2, 0x50, 0x5a),
        (0x77, 0x00b5_5220, 0x72, 0x50, 0x12),
        (0x78, 0x00b4_8250, 0x86, 0x50, 0x12),
        (0xf2, 0x00b5_7ce8, 2, 0x5a, 0x5a),
        (0x79, 0x00b5_5220, 0x72, 0x5a, 0x12),
        (0x7a, 0x00b4_8250, 0x86, 0x5a, 0x12),
        (0xf4, 0x00b5_7cf0, 2, 0x64, 0x5a),
        (0x7b, 0x00b5_5220, 0x72, 0x64, 0x12),
        (0x7c, 0x00b4_8250, 0x86, 0x64, 0x12),
        (0xf5, 0x00b1_7a68, 2, 0x6e, 0x5a),
        (0xf6, 0, 0x60, 0x6e, 0xe),
        (0x7d, 0x00b5_5220, 0x72, 0x6e, 0x12),
        (0x7e, 0x00b4_8250, 0x86, 0x6e, 0x12),
        (0xf7, 0x00b5_7cd4, 0x6c, 0x78, 0x30),
        (0xf8, 0x00b5_5e14, 0xc6, 0x78, 0x30),
    ];
    /// `Ms_Gui_ShakeProc`'s frame: its text buffer at `ebp - 0x104`.
    pub const SHAKE_FRAME: u32 = 0x104;

    fn c(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    fn sprintf(h: &mut impl Host, args: &[u32]) {
        let f: u32 = h.read(h.rebase(SPRINTF_SLOT));
        // call width: 20 (the parameter dump's words)
        h.call_wide(f, args);
    }

    fn screen(h: &impl Host) -> u32 {
        h.read(h.rebase(BTL_SCREEN))
    }

    fn k64(h: &impl Host, sym: u32) -> f64 {
        f64::from_bits(h.read(h.rebase(sym)))
    }

    /// An `f32` at `p` as `fld dword` loads it.
    fn f(h: &impl Host, p: u32) -> f64 {
        x87::load(h.read(p))
    }

    /// The `f32` at `p` as a pushed `fstp dword` word (quieted).
    fn word32(h: &impl Host, p: u32) -> u32 {
        x87::store(f(h, p))
    }

    /// The `f32` at `p` as a pushed `fstp qword` (low, high).
    fn words64(h: &impl Host, p: u32) -> [u32; 2] {
        let b = f(h, p).to_bits();
        #[allow(clippy::cast_possible_truncation)] // the two halves
        [b as u32, (b >> 32) as u32]
    }

    fn sbyte(h: &impl Host, p: u32) -> u32 {
        u32::from(h.read::<u8>(p))
    }

    fn sword(h: &impl Host, p: u32) -> u32 {
        i32::from(h.read::<i16>(p)).cast_unsigned()
    }

    /// The on/off text for bit 0 of the byte at `p`.
    fn on_off(h: &impl Host, p: u32) -> u32 {
        let k = u32::from(h.read::<u8>(p) & 1);
        h.read(h.rebase(ON_OFF).wrapping_add(4 * k))
    }

    /// `Ms_Gui_ShakeProc(window, msg, param)` (`0x007dfde0`) -> 0: the
    /// battle's screen shake debug window over `g_BtlScreen`'s two shake
    /// parameter sets (`+0x178` speed, `+0x188` decay, `+0x18c` count
    /// 1..99, `+0x18e` amplitude -0x100..0x100, `+0x172` length -1..
    /// 0x7fff, `+0x18d` on/off; the second at `+0x19c`, `+0x1ac`,
    /// `+0x1b0`, `+0x1b2`, `+0x196`, `+0x1b1`). 0x8001 builds it, 0x8003
    /// closes it, 0x8006/0x800a/0x800e reset the three auto-repeats;
    /// 0x8005/0x8009/0x800d (steps of 1/10/100 through the auto-repeat)
    /// step a value (speed by 0.01, decay by 0.001 and not below 0, the
    /// integers clamped), the 0xeb.. buttons round them (speed to an
    /// integer, decay to tenths, integers to tens) or flip the on/off
    /// bits, 0xf7 shakes with both sets (`MsScreenShake2`), 0xf8 dumps
    /// them; 0x8011 draws (counts at least 1, the on/off texts, the
    /// values).
    #[allow(clippy::too_many_lines)]
    pub fn shake_proc(h: &mut impl Host, ebp: u32, window: u32, msg: u32, param: u32) -> u32 {
        match msg {
            0x8001 => {
                let title = h.rebase(SHAKE_TITLE);
                c(h, WINDOW_SET_TITLE, &[window, title]);
                for r in [REPEAT_1, REPEAT_100, REPEAT_10] {
                    let r = h.rebase(r);
                    c(h, AUTO_REPEAT_INIT, &[r, 0xf, 3]);
                }
                for (id, text, x, y, cx) in SHAKE_BUTTONS {
                    let text = match id {
                        0xf0 => on_off(&*h, screen(&*h).wrapping_add(0x18d)),
                        0xf6 => on_off(&*h, screen(&*h).wrapping_add(0x1b1)),
                        _ => h.rebase(text),
                    };
                    c(h, WINDOW_ADD_BUTTON, &[window, id, text, x, y, cx, 8]);
                }
            }
            0x8003 => {
                let at = h.rebase(SHAKE_OPEN);
                h.write(at, 0_u32);
            }
            0x8006 | 0x800a | 0x800e => {
                let r = match msg {
                    0x8006 => REPEAT_1,
                    0x800a => REPEAT_10,
                    _ => REPEAT_100,
                };
                let r = h.rebase(r);
                c(h, AUTO_REPEAT_INIT, &[r, 0xf, 3]);
            }
            0x8005 | 0x8009 | 0x800d => {
                let (r, step) = match msg {
                    0x8005 => (REPEAT_1, 1),
                    0x8009 => (REPEAT_10, 10),
                    _ => (REPEAT_100, 100),
                };
                let r = h.rebase(r);
                if c(h, AUTO_REPEAT_CHECK, &[r, param]) != 0 {
                    shake_step(h, ebp, param, step);
                }
            }
            0x8011 => shake_draw(h, ebp, window),
            _ => {}
        }
        0
    }

    /// One parameter set's offsets: speed, decay, count, amplitude,
    /// length, on/off.
    const SETS: [[u32; 6]; 2] = [
        [0x178, 0x188, 0x18c, 0x18e, 0x172, 0x18d],
        [0x19c, 0x1ac, 0x1b0, 0x1b2, 0x196, 0x1b1],
    ];

    /// A float at `off` of the screen state stepped by `step * k` (`k`
    /// the double at `ksym`): down is `v - step k`, up `step k + v`;
    /// with `floor` a result below 0 is cleared (the state read again).
    fn step_float(h: &mut impl Host, off: u32, step: i32, ksym: u32, up: bool, floor: bool) {
        let p = screen(&*h).wrapping_add(off);
        let d = x87::mul(f64::from(step), k64(&*h, ksym));
        let v = if up {
            x87::add(d, f(&*h, p))
        } else {
            x87::sub(f(&*h, p), d)
        };
        h.write(p, x87::store(v));
        if floor && !up {
            let at = screen(&*h).wrapping_add(off);
            // fcom against 0: only a number below it is cleared
            if f(&*h, at) < 0.0 {
                h.write(at, 0_u32);
            }
        }
    }

    /// The `i16` at `p` truncated to tens.
    fn tens16(h: &mut impl Host, p: u32) {
        let v = i32::from(h.read::<i16>(p));
        #[allow(clippy::cast_possible_truncation)] // mov word
        h.write(p, ((v / 10) * 10) as i16);
    }

    /// The button `param` with the step `step` (`MsAutoRepeatCheck`
    /// said go).
    #[allow(clippy::too_many_lines)]
    fn shake_step(h: &mut impl Host, ebp: u32, param: u32, step: i32) {
        let (set, k) = match param {
            0x61..=0x6a => (0, param - 0x61),
            0x75..=0x7e => (1, param - 0x75),
            _ => (usize::MAX, 0),
        };
        if set != usize::MAX {
            let o = SETS[set];
            let s = screen(&*h);
            let at = |i: usize| s.wrapping_add(o[i]);
            let up = k & 1 == 1;
            match k >> 1 {
                0 => step_float(h, o[0], step, STEP_SPEED, up, false),
                1 => step_float(h, o[1], step, STEP_DECAY, up, true),
                2 => {
                    let v = i32::from(h.read::<u8>(at(2)));
                    let v = if up {
                        (v + step).min(99)
                    } else {
                        (v - step).max(1)
                    };
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // mov byte
                    h.write(at(2), v as u8);
                }
                3 | 4 => {
                    let (p, lo, hi) = if k >> 1 == 3 {
                        (at(3), -0x100, 0x100)
                    } else {
                        (at(4), -1, 0x7fff)
                    };
                    let v = i32::from(h.read::<i16>(p));
                    let v = if up {
                        (v + step).min(hi)
                    } else {
                        (v - step).max(lo)
                    };
                    #[allow(clippy::cast_possible_truncation)] // mov word
                    h.write(p, v as i16);
                }
                _ => {}
            }
            return;
        }
        let set = usize::from((0xf1..=0xf6).contains(&param));
        let o = SETS[set];
        let s = screen(&*h);
        let at = |i: usize| s.wrapping_add(o[i]);
        match param {
            0xeb | 0xf1 => {
                let i = x87::ftol(&*h, f(&*h, at(0)));
                #[allow(clippy::cast_precision_loss)] // fild, then fstp m32
                h.write(at(0), (i.cast_signed() as f32).to_bits());
            }
            0xed | 0xf3 => {
                let ten = k64(&*h, TEN);
                let i = x87::ftol(&*h, x87::mul(f(&*h, at(1)), ten));
                #[allow(clippy::cast_precision_loss)] // fild, then fstp m32
                let i = x87::load((i.cast_signed() as f32).to_bits());
                h.write(at(1), x87::store(x87::div(i, ten)));
            }
            0xec | 0xf2 => {
                let v = h.read::<u8>(at(2)) / 10;
                h.write(at(2), v.wrapping_mul(10));
            }
            0xee | 0xf4 => tens16(h, at(3)),
            0xef | 0xf5 => tens16(h, at(4)),
            0xf0 | 0xf6 => {
                let v: u8 = h.read(at(5));
                h.write(at(5), v.wrapping_add(1));
                let s = screen(&*h);
                let v: u8 = h.read(s.wrapping_add(o[5]));
                h.write(s.wrapping_add(o[5]), v & 1);
            }
            0xf7 => {
                for (set, which) in [(0, 1), (1, 2)] {
                    let s = screen(&*h);
                    let o = SETS[set];
                    let a = |i: usize| s.wrapping_add(o[i]);
                    let args = [
                        0,
                        which,
                        9,
                        sbyte(&*h, a(2)),
                        sword(&*h, a(4)),
                        word32(&*h, a(0)),
                        sword(&*h, a(3)),
                        word32(&*h, a(1)),
                        sbyte(&*h, a(5)),
                    ];
                    // call width: 9 (MsScreenShake2's words)
                    c(h, SCREEN_SHAKE2, &args);
                }
            }
            0xf8 => {
                let s = screen(&*h);
                let a = |set: usize, i: usize| s.wrapping_add(SETS[set][i]);
                let buf = ebp.wrapping_sub(SHAKE_FRAME);
                let fmt = h.rebase(DUMP_FMT);
                let [s0l, s0h] = words64(&*h, a(0, 0));
                let [d0l, d0h] = words64(&*h, a(0, 1));
                let [s1l, s1h] = words64(&*h, a(1, 0));
                let [d1l, d1h] = words64(&*h, a(1, 1));
                let args = [
                    buf,
                    fmt,
                    0,
                    sbyte(&*h, a(0, 2)),
                    sword(&*h, a(0, 4)),
                    s0l,
                    s0h,
                    sword(&*h, a(0, 3)),
                    d0l,
                    d0h,
                    sbyte(&*h, a(0, 5)),
                    0,
                    sbyte(&*h, a(1, 2)),
                    sword(&*h, a(1, 4)),
                    s1l,
                    s1h,
                    sword(&*h, a(1, 3)),
                    d1l,
                    d1h,
                    sbyte(&*h, a(1, 5)),
                ];
                sprintf(h, &args);
                c(h, SHAKE_LOG, &[buf]);
                let log = h.rebase(DUMP_LOG);
                c(h, DBG_PRINTF, &[log, buf]);
            }
            _ => {}
        }
    }

    /// `Ms_Gui_ShakeProc`'s 0x8011: the counts kept at least 1, the on/off
    /// texts, then each set's values printed down the right column.
    fn shake_draw(h: &mut impl Host, ebp: u32, window: u32) {
        let buf = ebp.wrapping_sub(SHAKE_FRAME);
        for set in SETS {
            let s = screen(&*h);
            if h.read::<u8>(s.wrapping_add(set[2])) < 1 {
                h.write(s.wrapping_add(set[2]), 1_u8);
            }
        }
        for (id, set) in [(0xf0, 0), (0xf6, 1)] {
            let t = on_off(&*h, screen(&*h).wrapping_add(SETS[set][5]));
            c(h, WINDOW_SET_TEXT, &[window, id, t]);
        }
        for (set, head, y0) in [(0, SHAKE_HEAD1, 1_u32), (1, SHAKE_HEAD2, 0x3d)] {
            c(h, PRINT_SET_X, &[4]);
            c(h, PRINT_SET_Y, &[y0]);
            let head = h.rebase(head);
            c(h, PRINTF, &[head]);
            for (k, i) in [0, 1, 2, 3, 4].into_iter().enumerate() {
                c(h, PRINT_SET_X, &[0xb4]);
                #[allow(clippy::cast_possible_truncation)] // five rows
                c(h, PRINT_SET_Y, &[y0 + 0xa + 0xa * k as u32]);
                let s = screen(&*h);
                let p = s.wrapping_add(SETS[set][i]);
                if i < 2 {
                    let [lo, hi] = words64(&*h, p);
                    let fmt = h.rebase(FLOAT_FMT);
                    sprintf(h, &[buf, fmt, lo, hi]);
                } else {
                    let v = if i == 2 { sbyte(&*h, p) } else { sword(&*h, p) };
                    let fmt = h.rebase(INT_FMT);
                    sprintf(h, &[buf, fmt, v]);
                }
                c(h, PRINTF, &[buf]);
            }
        }
    }

    /// What `Ms_Gui_BattleInfoProc` calls besides the shake window's
    /// callees: `MsEffectGetLastMagicNum()`, `MsSEPtransAmount(m)`,
    /// `MsBtlReadCueLevel(a)`, `SndIsReadStatus()`,
    /// `SndIsReadVagVoiceStatus()`, `SndIsReadShoutVoiceStatus()`,
    /// `MsBattleGetStatus()`, `FUN_007915f0()`, `MsGetSaveWindow(a, b,
    /// &n)`, `MsGetChrCommand(id)`, `MsGetChrBit(a, b, c, d)`,
    /// `MsGetChr(id, ...)`, `MsGetChrTop()`, `MsGetMon(k)`,
    /// `TkGetNowEventJumpMapNo()` (`ax`), `FUN_007dbee0(kind, out, text,
    /// sel, chr)`.
    pub const LAST_MAGIC: u32 = 0x0078_7be0;
    pub const SEP_TRANS_AMOUNT: u32 = 0x007a_04b0;
    pub const READ_CUE_LEVEL: u32 = 0x007a_d180;
    pub const SND_READ: u32 = 0x0088_6650;
    pub const SND_READ_VAG: u32 = 0x0088_6660;
    pub const SND_READ_SHOUT: u32 = 0x0088_6640;
    pub const BATTLE_GET_STATUS: u32 = 0x0078_16f0;
    pub const BATTLE_PHASE: u32 = 0x0079_15f0;
    pub const GET_SAVE_WINDOW: u32 = 0x0078_5410;
    pub const GET_CHR_COMMAND: u32 = 0x007b_09b0;
    pub const GET_CHR_BIT: u32 = 0x0079_4330;
    pub const GET_CHR: u32 = 0x0079_4020;
    pub const GET_CHR_TOP: u32 = 0x0079_5970;
    pub const GET_MON: u32 = 0x0079_5aa0;
    pub const EVENT_JUMP_MAP: u32 = 0x0086_c1f0;
    pub const INFO_ROW: u32 = 0x007d_bee0;
    /// The window's state: its two auto-repeats, the row and page
    /// selectors (`0x0113f898` 0..1, `0x0113f89c` 0..3), the selected
    /// field id (`0x00c44a3c`), the open flag; the texts of the two
    /// selector buttons (`0x00c44a24`/`0x00c44a2c` by value).
    pub const INFO_REPEAT_UP: u32 = 0x0113_f870;
    pub const INFO_REPEAT_DOWN: u32 = 0x0113_f884;
    pub const INFO_ROWS: u32 = 0x0113_f898;
    pub const INFO_PAGE: u32 = 0x0113_f89c;
    pub const INFO_FIELD: u32 = 0x00c4_4a3c;
    pub const INFO_OPEN: u32 = 0x0113_9ef0;
    pub const INFO_ROWS_TEXT: u32 = 0x00c4_4a24;
    pub const INFO_PAGE_TEXT: u32 = 0x00c4_4a2c;
    /// The battle globals it prints.
    pub const BTL_STATUS: u32 = 0x0112_a8e0;
    pub const BTL_FIELD_INDEX: u32 = 0x0112_c256;
    pub const BTL_VOICE_EVENT: u32 = 0x0112_a9c0;
    pub const BTL_COMMAND_ACTOR: u32 = 0x0112_c9e0;
    pub const BTL_MODE: u32 = 0x0112_c254;
    /// The window's controls (id, text, x, y, cx), height 8; 0x40 and
    /// 0x91 take their texts from the selectors (text 0 here).
    pub const INFO_BUTTONS: [(u32, u32, u32, u32, u32); 45] = [
        (0x40, 0, 8, 0x14, 0x28),
        (0x51, 0x00b5_6a20, 0x3a, 0x14, 0x16),
        (0x50, 0x00b5_6a24, 0x54, 0x14, 0x16),
        (0x54, 0x00b5_6a28, 0x6e, 0x14, 0x16),
        (0x55, 0x00b5_6a2c, 0x88, 0x14, 0x16),
        (0x56, 0x00b5_6a30, 0xa2, 0x14, 0x16),
        (0x57, 0x00b5_6a34, 0xbc, 0x14, 0x16),
        (0x4f, 0x00b5_6a38, 0xd6, 0x14, 0x16),
        (0x8f, 0x00b5_6a3c, 0xf0, 0x14, 0x16),
        (0x5c, 0x00b5_6a40, 0x10a, 0x14, 0x16),
        (0x91, 0, 8, 0x1e, 0x28),
        (0x90, 0x00b4_a0a4, 0x3a, 0x1e, 0x16),
        (0x96, 0x00b5_6a44, 0x54, 0x1e, 0x16),
        (0xc6, 0x00b5_6a48, 0x6e, 0x1e, 0x16),
        (0x99, 0x00b5_6a4c, 0x88, 0x1e, 0x16),
        (0xcd, 0x00b5_6a50, 0xa2, 0x1e, 0x16),
        (0xce, 0x00b5_6a54, 0xbc, 0x1e, 0x16),
        (0x58, 0x00b5_6a58, 0xd6, 0x1e, 0x16),
        (0x59, 0x00b5_6a5c, 0xf0, 0x1e, 0x16),
        (0x5a, 0x00b5_6a60, 0x10a, 0x1e, 0x16),
        (0xcf, 0x00b5_6a64, 0x3a, 0x28, 0x16),
        (0xd0, 0x00b5_6a68, 0x54, 0x28, 0x16),
        (0xd3, 0x00b5_6a6c, 0x6e, 0x28, 0x16),
        (0xdc, 0x00b5_6a70, 0x88, 0x28, 0x16),
        (0xde, 0x00b5_6a74, 0xa2, 0x28, 0x16),
        (0xdf, 0x00b5_6a78, 0xbc, 0x28, 0x16),
        (0xe0, 0x00b5_6a7c, 0xd6, 0x28, 0x16),
        (0xe2, 0x00b5_6a80, 0xf0, 0x28, 0x16),
        (0xe4, 0x00b5_6a84, 0x10a, 0x28, 0x16),
        (0xa6, 0x00b5_6a88, 0x3a, 0x32, 0x16),
        (0xa7, 0x00b5_6a8c, 0x54, 0x32, 0x16),
        (0xa8, 0x00b5_6a90, 0x6e, 0x32, 0x16),
        (0xa9, 0x00b5_6a94, 0x88, 0x32, 0x16),
        (0xaa, 0x00b5_6a98, 0xa2, 0x32, 0x16),
        (0xab, 0x00b5_6a9c, 0xbc, 0x32, 0x16),
        (0xac, 0x00b5_6aa0, 0xd6, 0x32, 0x16),
        (0xad, 0x00b5_6aa4, 0xf0, 0x32, 0x16),
        (0x9e, 0x00b5_6aa8, 0x3a, 0x3c, 0x16),
        (0x9f, 0x00b5_6aac, 0x54, 0x3c, 0x16),
        (0xa0, 0x00b5_6ab0, 0x6e, 0x3c, 0x16),
        (0xa1, 0x00b5_6ab4, 0x88, 0x3c, 0x16),
        (0xa2, 0x00b5_6ab8, 0xa2, 0x3c, 0x16),
        (0xa3, 0x00b5_6abc, 0xbc, 0x3c, 0x16),
        (0xa4, 0x00b5_6ac0, 0xd6, 0x3c, 0x16),
        (0xa5, 0x00b5_6ac4, 0xf0, 0x3c, 0x16),
    ];
    /// `Ms_Gui_BattleInfoProc`'s frame: the text at `ebp - 0x104`, the
    /// line at `ebp - 0x204`, `MsGetSaveWindow`'s count at `ebp - 0x214`.
    pub const INFO_FRAME: u32 = 0x214;

    fn sb(h: &impl Host, sym: u32) -> u32 {
        i32::from(h.read::<i8>(h.rebase(sym))).cast_unsigned()
    }

    fn ub(h: &impl Host, sym: u32) -> u32 {
        u32::from(h.read::<u8>(h.rebase(sym)))
    }

    fn gw(h: &impl Host, sym: u32) -> u32 {
        h.read(h.rebase(sym))
    }

    fn print(h: &mut impl Host, x: u32, y: u32, s: u32) {
        c(h, PRINT_SET_X, &[x]);
        c(h, PRINT_SET_Y, &[y]);
        c(h, PRINTF, &[s]);
    }

    fn sprintf_r(h: &mut impl Host, buf: u32, fmt: u32, args: &[u32]) {
        let mut a = vec![buf, h.rebase(fmt)];
        a.extend_from_slice(args);
        sprintf(h, &a);
    }

    /// `Ms_Gui_BattleInfoProc(window, msg, param)` (`0x007daed0`) -> 0:
    /// the battle info debug window. 0x8001 builds it (two auto-repeats,
    /// the selectors at 0, 45 buttons), 0x8003 closes it, 0x8006/0x800a
    /// reset an auto-repeat; 0x8005/0x8009 (up/down through the
    /// auto-repeat) step the row selector (0x40, 0..1 wrapping) or the
    /// page (0x91, 0..3 wrapping), any other button selects its field;
    /// 0x8011 shows the selectors, the scene in the title, the page
    /// (scene and voice, battle state, save window counts, command
    /// actor) and the party / monster or the other actors' rows (each
    /// through `FUN_007dbee0` while a battle runs).
    #[allow(clippy::too_many_lines)]
    pub fn battle_info_proc(h: &mut impl Host, ebp: u32, window: u32, msg: u32, param: u32) -> u32 {
        match msg {
            0x8001 => {
                for r in [INFO_REPEAT_UP, INFO_REPEAT_DOWN] {
                    let r = h.rebase(r);
                    c(h, AUTO_REPEAT_INIT, &[r, 0xf, 3]);
                }
                let (rows, page) = (h.rebase(INFO_ROWS), h.rebase(INFO_PAGE));
                h.write(rows, 0_u32);
                h.write(page, 0_u32);
                for (id, text, x, y, cx) in INFO_BUTTONS {
                    let text = match id {
                        0x40 => gw(&*h, INFO_ROWS_TEXT),
                        0x91 => {
                            let k = gw(&*h, INFO_PAGE);
                            gw(&*h, INFO_PAGE_TEXT.wrapping_add(4 * k))
                        }
                        _ => h.rebase(text),
                    };
                    c(h, WINDOW_ADD_BUTTON, &[window, id, text, x, y, cx, 8]);
                }
            }
            0x8003 => {
                let at = h.rebase(INFO_OPEN);
                h.write(at, 0_u32);
            }
            0x8006 | 0x800a => {
                let r = h.rebase(if msg == 0x8006 {
                    INFO_REPEAT_UP
                } else {
                    INFO_REPEAT_DOWN
                });
                c(h, AUTO_REPEAT_INIT, &[r, 0xf, 3]);
            }
            0x8005 | 0x8009 => {
                let (r, d) = if msg == 0x8005 {
                    (INFO_REPEAT_UP, 1_i32)
                } else {
                    (INFO_REPEAT_DOWN, -1)
                };
                let r = h.rebase(r);
                if c(h, AUTO_REPEAT_CHECK, &[r, param]) == 0 {
                    return 0;
                }
                let (sym, wrap_lo, top) = match param {
                    0x40 => (INFO_ROWS, 1_u32, 2),
                    0x91 => (INFO_PAGE, 3, 4),
                    _ => {
                        let at = h.rebase(INFO_FIELD);
                        h.write(at, param);
                        return 0;
                    }
                };
                let at = h.rebase(sym);
                let v = h.read::<u32>(at).cast_signed().wrapping_add(d);
                h.write(at, v);
                if v < 0 {
                    h.write(at, wrap_lo);
                } else if v >= top {
                    h.write(at, 0_u32);
                }
            }
            0x8011 => info_draw(h, ebp, window),
            _ => {}
        }
        0
    }

    /// `Ms_Gui_BattleInfoProc`'s 0x8011.
    #[allow(clippy::too_many_lines)]
    fn info_draw(h: &mut impl Host, ebp: u32, window: u32) {
        let t = ebp.wrapping_sub(0x104);
        let line = ebp.wrapping_sub(0x204);
        let k = gw(&*h, INFO_ROWS);
        let text = gw(&*h, INFO_ROWS_TEXT.wrapping_add(4 * k));
        c(h, WINDOW_SET_TEXT, &[window, 0x40, text]);
        let k = gw(&*h, INFO_PAGE);
        let text = gw(&*h, INFO_PAGE_TEXT.wrapping_add(4 * k));
        c(h, WINDOW_SET_TEXT, &[window, 0x91, text]);
        // the scene in the title
        if ub(&*h, BTL_STATUS) == 0 {
            sprintf_r(h, t, 0x00b2_b91c, &[]);
        } else {
            let s = if h.read::<u16>(h.rebase(BTL_FIELD_INDEX)) != 0 {
                0x0112_c25a
            } else if ub(&*h, 0x0112_c259) == 0 {
                0x00c4_4a00
            } else {
                0x00c4_4a08
            };
            let s = h.rebase(s);
            sprintf_r(h, t, 0x00b0_e160, &[s]);
        }
        sprintf_r(h, line, 0x00b5_6ac8, &[t]);
        c(h, WINDOW_SET_TITLE, &[window, line]);
        match gw(&*h, INFO_PAGE) {
            1 => {
                c(h, PRINT_SET_X, &[2]);
                c(h, PRINT_SET_Y, &[1]);
                let magic = c(h, LAST_MAGIC, &[]);
                let amount = c(h, SEP_TRANS_AMOUNT, &[magic]);
                let level = c(h, READ_CUE_LEVEL, &[amount]);
                // the three results stay pushed: RD, SE and M of the line
                let a = [
                    sb(&*h, 0x0112_a8e0),
                    sb(&*h, 0x0112_a8e4),
                    sb(&*h, 0x0112_c9d0),
                    sb(&*h, 0x0112_c9e6),
                    level,
                    amount,
                    magic,
                ];
                sprintf_r(h, line, 0x00b5_6b54, &a);
                c(h, PRINTF, &[line]);
                c(h, PRINT_SET_X, &[2]);
                c(h, PRINT_SET_Y, &[0xb]);
                let s = u32::from(c(h, SND_READ, &[]) != 0);
                let v = u32::from(c(h, SND_READ_VAG, &[]) != 0);
                let sh = u32::from(c(h, SND_READ_SHOUT, &[]) != 0);
                sprintf_r(h, line, 0x00b5_6b80, &[s, v, sh]);
                c(h, PRINTF, &[line]);
                if ub(&*h, 0x0112_c9ea) != 0 {
                    c(h, PRINT_SET_X, &[0x3a]);
                    c(h, PRINT_SET_Y, &[0xb]);
                    sprintf_r(h, line, 0x00b5_6b90, &[]);
                    c(h, PRINTF, &[line]);
                }
                if sb(&*h, 0x0112_c9e4).cast_signed() >= 0 {
                    c(h, PRINT_SET_X, &[0x5a]);
                    c(h, PRINT_SET_Y, &[0xb]);
                    let v = sb(&*h, 0x0112_c9e4);
                    sprintf_r(h, line, 0x00b5_6b94, &[v]);
                    c(h, PRINTF, &[line]);
                }
                let st = if c(h, BATTLE_GET_STATUS, &[]) != 0 {
                    c(h, BATTLE_PHASE, &[])
                } else {
                    0
                };
                c(h, PRINT_SET_X, &[0x89]);
                c(h, PRINT_SET_Y, &[0xb]);
                let a = [
                    ub(&*h, 0x0112_c9f5),
                    sb(&*h, 0x0112_c9f6),
                    st,
                    sb(&*h, 0x0112_c9d4),
                    sb(&*h, 0x0112_c9d5),
                ];
                sprintf_r(h, line, 0x00b5_6b9c, &a);
                c(h, PRINTF, &[line]);
            }
            2 => {
                if c(h, BATTLE_GET_STATUS, &[]) != 0 {
                    let n = ebp.wrapping_sub(0x214);
                    for (kind, y, fmt) in [(9, 1, 0x00b5_6b14), (0xb, 0xb, 0x00b5_6b34)] {
                        let p = c(h, GET_SAVE_WINDOW, &[0, kind, n]);
                        c(h, PRINT_SET_X, &[2]);
                        c(h, PRINT_SET_Y, &[y]);
                        let cnt: u32 = h.read(n);
                        let w = [0, 2, 4, 6].map(|o| u32::from(h.read::<u16>(p.wrapping_add(o))));
                        let a = [cnt, w[0], w[1], w[2], w[3]];
                        sprintf_r(h, line, fmt, &a);
                        c(h, PRINTF, &[line]);
                    }
                }
            }
            3 => {
                let actor = ub(&*h, BTL_COMMAND_ACTOR);
                let cmd = c(h, GET_CHR_COMMAND, &[actor]);
                c(h, PRINT_SET_X, &[2]);
                c(h, PRINT_SET_Y, &[1]);
                let b1 = c(h, GET_CHR_BIT, &[0, 0xffff_fffc, 0, 1]);
                let b2 = c(h, GET_CHR_BIT, &[0, 0xffff_fffd, 0, 1]);
                let a = [sb(&*h, 0x0112_ca01), gw(&*h, 0x0112_c8f8), b2, b1];
                sprintf_r(h, line, 0x00b5_6ad8, &a);
                c(h, PRINTF, &[line]);
                c(h, PRINT_SET_X, &[2]);
                c(h, PRINT_SET_Y, &[0xb]);
                let n = u32::from(h.read::<u8>(cmd.wrapping_add(2)));
                let target: u32 = h.read(cmd.wrapping_add(n.wrapping_add(1).wrapping_mul(16)));
                let (e9ec, ca15) = (ub(&*h, 0x0112_c9ec), sb(&*h, 0x0112_ca15));
                let chr = c(h, GET_CHR, &[actor, target, n, e9ec, ca15]);
                let mg = i32::from(h.read::<i8>(chr.wrapping_add(0xdf0))).cast_unsigned();
                // the words MsGetChr got stay pushed as the rest
                sprintf_r(h, line, 0x00b5_6af4, &[actor, mg, target, n, e9ec, ca15]);
                c(h, PRINTF, &[line]);
            }
            _ => {
                let a = [
                    i32::from(h.read::<i16>(h.rebase(BTL_FIELD_INDEX))).cast_unsigned(),
                    sb(&*h, 0x0112_c258),
                    sb(&*h, 0x0112_c259),
                    sb(&*h, 0x0112_bde2),
                    sb(&*h, 0x0112_bde3),
                    sb(&*h, 0x0112_bde0),
                    sb(&*h, 0x0112_bde1),
                ];
                c(h, PRINT_SET_X, &[8]);
                c(h, PRINT_SET_Y, &[1]);
                sprintf_r(h, t, 0x00b5_6bb4, &a);
                c(h, PRINTF, &[t]);
                let ve = gw(&*h, BTL_VOICE_EVENT);
                let a =
                    [0, 1, 2].map(|o| i32::from(h.read::<i8>(ve.wrapping_add(o))).cast_unsigned());
                sprintf_r(h, t, 0x00b5_6bdc, &a);
                print(h, 8, 0xb, t);
                let mode = gw(&*h, BTL_MODE);
                let mf = if mode & 0x400 != 0 {
                    0x00b5_6bec
                } else {
                    0x00b5_6bf4
                };
                sprintf_r(h, t, mf, &[mode & 0x3ff]);
                print(h, 0x50, 0xb, t);
                let en = if ub(&*h, 0x0112_a9d7) != 0 {
                    0x00b5_6bfc
                } else {
                    0x00b5_6c10
                };
                let map = c(h, EVENT_JUMP_MAP, &[]);
                // cwde of ax
                #[allow(clippy::cast_possible_truncation)]
                let map = i32::from((map as u16).cast_signed()).cast_unsigned();
                let ten = f64::from_bits(h.read(h.rebase(TEN)));
                let a = x87::ftol(&*h, x87::div(f(&*h, h.rebase(0x0112_a9dc)), ten));
                let k100 = f64::from_bits(h.read(h.rebase(0x00b9_22e0)));
                let b = x87::ftol(
                    &*h,
                    x87::div(
                        x87::mul(f(&*h, h.rebase(0x0112_a9e0)), k100),
                        f(&*h, h.rebase(0x0112_a9e4)),
                    ),
                );
                let d6 = ub(&*h, 0x0112_a9d6);
                sprintf_r(h, t, en, &[100_u32.wrapping_sub(b), a, d6, map]);
                print(h, 0x84, 0xb, t);
            }
        }
        info_rows(h, t, line);
    }

    /// The row text `t` printed, or while a battle runs (`MsGetChrTop`)
    /// `FUN_007dbee0(kind, line, t, sel, who)` of the actor `get(id)`
    /// printed from `line`.
    fn info_row(h: &mut impl Host, t: u32, line: u32, kind: u32, get: u32, id: u32) {
        if c(h, GET_CHR_TOP, &[]) == 0 {
            c(h, PRINTF, &[t]);
            return;
        }
        let who = c(h, get, &[id]);
        let sel = gw(&*h, INFO_FIELD);
        c(h, INFO_ROW, &[kind, line, t, sel, who]);
        c(h, PRINTF, &[line]);
    }

    /// The rows under the page: the party and the monsters (rows 0) or
    /// the other actors (rows 1).
    fn info_rows(h: &mut impl Host, t: u32, line: u32) {
        let mut y = 0x47;
        match gw(&*h, INFO_ROWS) {
            0 => {
                for i in 0..7 {
                    c(h, PRINT_SET_X, &[2]);
                    c(h, PRINT_SET_Y, &[y]);
                    sprintf_r(h, t, 0x00b5_6c24, &[i, i]);
                    info_row(h, t, line, 0, GET_CHR, i);
                    y += 10;
                }
                for k in 0..8 {
                    c(h, PRINT_SET_X, &[2]);
                    c(h, PRINT_SET_Y, &[y]);
                    let ve = gw(&*h, BTL_VOICE_EVENT);
                    let w = u32::from(h.read::<u16>(ve.wrapping_add(0xc + 2 * k)));
                    if w & 0xfff == 0xfff {
                        sprintf_r(h, t, 0x00b5_6c30, &[k + 1]);
                    } else {
                        sprintf_r(h, t, 0x00b5_6c3c, &[k + 1, w & 0xfff]);
                    }
                    info_row(h, t, line, 1, GET_MON, k);
                    y += 10;
                }
            }
            1 => {
                c(h, PRINT_SET_X, &[2]);
                c(h, PRINT_SET_Y, &[y]);
                sprintf_r(h, t, 0x00b5_6c48, &[0, 0]);
                info_row(h, t, line, 0, GET_CHR, 7);
                y = 0x51;
                for i in 0..10 {
                    c(h, PRINT_SET_X, &[2]);
                    c(h, PRINT_SET_Y, &[y]);
                    sprintf_r(h, t, 0x00b5_6c54, &[i, i]);
                    info_row(h, t, line, 0, GET_CHR, i + 8);
                    y += 10;
                }
                for i in 0..3 {
                    c(h, PRINT_SET_X, &[2]);
                    c(h, PRINT_SET_Y, &[y]);
                    if c(h, GET_CHR_TOP, &[]) == 0 {
                        sprintf_r(h, t, 0x00b5_6c6c, &[i]);
                        c(h, PRINTF, &[t]);
                    } else {
                        let who = c(h, GET_CHR, &[i + 0x1c]);
                        let b = u32::from(h.read::<u8>(who.wrapping_add(0xc)));
                        sprintf_r(h, t, 0x00b5_6c60, &[i, b]);
                        let sel = gw(&*h, INFO_FIELD);
                        c(h, INFO_ROW, &[0, line, t, sel, who]);
                        c(h, PRINTF, &[line]);
                    }
                    y += 10;
                }
            }
            _ => {}
        }
    }

    /// What `Ms_Gui_BattleChr2Proc` calls besides the windows' common
    /// callees: `Ch_SetWeight(chr, w)`, `MsSetMotBinAddress(chr, k)`,
    /// `MsGetPlySaveName(buf, name)`, `MsPlySave(buf)`,
    /// `MsSetFileName(buf, id)`, `MsMonSave(buf, k, id)`,
    /// `MsStrCopy(src, dst, n)`.
    pub const CH_SET_WEIGHT: u32 = 0x0082_b940;
    pub const SET_MOT_BIN: u32 = 0x0079_cd80;
    pub const PLY_SAVE_NAME: u32 = 0x007d_cec0;
    pub const PLY_SAVE: u32 = 0x007d_d5e0;
    pub const SET_FILE_NAME: u32 = 0x007d_ce80;
    pub const MON_SAVE: u32 = 0x007d_d480;
    pub const STR_COPY: u32 = 0x007c_f6c0;
    /// The window's three auto-repeats (steps of 1, 100 and 10), the
    /// selected actor (`0x0113f8a0`: 0..0x13 party, 0x14..0x1e
    /// monsters), the open flag, the actor names (`0x00c44950`).
    pub const CHR2_REPEAT_1: u32 = 0x0113_f8e0;
    pub const CHR2_REPEAT_100: u32 = 0x0113_f8f4;
    pub const CHR2_REPEAT_10: u32 = 0x0113_f908;
    pub const CHR2_ACTOR: u32 = 0x0113_f8a0;
    pub const CHR2_OPEN: u32 = 0x0113_9f04;
    pub const CHR2_NAMES: u32 = 0x00c4_4950;
    /// The float rows: (the - button, field, step), + is the next id.
    pub const CHR2_FLOATS: [(u32, u32, u32); 8] = [
        (0x63, 0x4bc, STEP_TENTH),
        (0x65, 0x4c0, STEP_TENTH),
        (0x67, 0x4c4, STEP_SPEED),
        (0x69, 0x4c8, STEP_TENTH),
        (0x6b, 0x4d4, STEP_SPEED),
        (0x6d, 0x4d8, STEP_SPEED),
        (0x6f, 0x4cc, STEP_SPEED),
        (0x71, 0x4d0, STEP_SPEED),
    ];
    /// 0.1 (a double).
    pub const STEP_TENTH: u32 = 0x00b5_51e0;
    /// The drawn rows: (y, field, kind 0 f32 / 1 u16 / 2 u8 / 3 i8,
    /// label).
    pub const CHR2_ROWS: [(u32, u32, u8, u32); 18] = [
        (0x15, 0x4bc, 0, 0x00b5_7b60),
        (0x1f, 0x4c0, 0, 0x00b5_7b74),
        (0x29, 0x4c4, 0, 0x00b5_7b84),
        (0x33, 0x4c8, 0, 0x00b5_7b94),
        (0x3d, 0x4d4, 0, 0x00b5_7bac),
        (0x47, 0x4d8, 0, 0x00b5_7bbc),
        (0x51, 0x4cc, 0, 0x00b5_7bd4),
        (0x5b, 0x4d0, 0, 0x00b5_7bec),
        (0x65, 0x4dc, 0, 0x00b5_7c00),
        (0x6f, 0x4b0, 1, 0x00b5_7c10),
        (0x79, 0x4b2, 1, 0x00b5_7c20),
        (0x83, 0x4b9, 2, 0x00b5_7c38),
        (0x8d, 0x4ba, 2, 0x00b5_7c4c),
        (0x97, 0x4b8, 3, 0x00b5_7c60),
        (0xa1, 0x4bb, 2, 0x00b5_7c70),
        (0xab, 0x40c, 2, 0x00b5_7c80),
        (0xb5, 0x40d, 2, 0x00b5_7c98),
        (0xbf, 0x40e, 2, 0x00b5_7cac),
    ];
    /// `Ms_Gui_BattleChr2Proc`'s frame: its text buffer at `ebp - 0x104`.
    pub const CHR2_FRAME: u32 = 0x104;

    /// `Ms_Gui_BattleChr2Proc(window, msg, param)` (`0x007de790`) -> 0:
    /// the battle's "CHR data 2" debug window over the selected actor
    /// (`MsGetChr(0x0113f8a0)`): its attack, move and run floats, weight,
    /// frames and bytes. 0x8001 builds it (three auto-repeats, 41
    /// buttons), 0x8003 closes it, 0x8006/0x800a/0x800e reset an
    /// auto-repeat; while a battle runs 0x8005/0x8009/0x800d (steps of
    /// 1/10/100 through the auto-repeat) pick the actor (0x61/0x62:
    /// wrapping 0..0x1e by one, or a bigger step jumping between party
    /// and monsters), step a float (by 0.1 or 0.01), the weight (whole
    /// steps, `Ch_SetWeight` when the actor has a model), the integers
    /// (not below 0, capped), the motion type (`MsSetMotBinAddress`),
    /// round the floats (0xd5), save the party (0xd6) or a monster
    /// (0xd7); 0x8011 prints the actor's name, id and values.
    #[allow(clippy::too_many_lines)]
    pub fn battle_chr2_proc(h: &mut impl Host, ebp: u32, window: u32, msg: u32, param: u32) -> u32 {
        let buf = ebp.wrapping_sub(CHR2_FRAME);
        match msg {
            0x8001 => {
                let title = h.rebase(0x00b5_7b54);
                c(h, WINDOW_SET_TITLE, &[window, title]);
                for r in [CHR2_REPEAT_1, CHR2_REPEAT_100, CHR2_REPEAT_10] {
                    let r = h.rebase(r);
                    c(h, AUTO_REPEAT_INIT, &[r, 0xf, 3]);
                }
                chr2_buttons(h, window);
            }
            0x8003 => {
                let at = h.rebase(CHR2_OPEN);
                h.write(at, 0_u32);
            }
            0x8006 | 0x800a | 0x800e => {
                let r = match msg {
                    0x8006 => CHR2_REPEAT_1,
                    0x800a => CHR2_REPEAT_10,
                    _ => CHR2_REPEAT_100,
                };
                let r = h.rebase(r);
                c(h, AUTO_REPEAT_INIT, &[r, 0xf, 3]);
            }
            0x8005 | 0x8009 | 0x800d => {
                if ub(&*h, BTL_STATUS) == 0 {
                    return 0;
                }
                let actor = gw(&*h, CHR2_ACTOR);
                let chr = c(h, GET_CHR, &[actor]);
                let model: u32 = h.read(chr);
                let (r, step) = match msg {
                    0x8005 => (CHR2_REPEAT_1, 1_i32),
                    0x8009 => (CHR2_REPEAT_10, 10),
                    _ => (CHR2_REPEAT_100, 100),
                };
                let r = h.rebase(r);
                if c(h, AUTO_REPEAT_CHECK, &[r, param]) != 0 {
                    chr2_step(h, buf, chr, model, param, step);
                }
            }
            0x8011 if ub(&*h, BTL_STATUS) != 0 => chr2_draw(h, buf),
            _ => {}
        }
        0
    }

    /// The 41 buttons: the actor and save row, then a -/+ pair per value.
    fn chr2_buttons(h: &mut impl Host, window: u32) {
        let (minus, plus) = (h.rebase(0x00b5_5220), h.rebase(0x00b4_8250));
        let top = [
            (0x61, minus, 8, 0x12),
            (0x62, plus, 0x1e, 0x12),
            (0xd5, h.rebase(0x00b5_72ac), 0xd6, 0x28),
            (0xd6, h.rebase(0x00b5_72b0), 0x102, 0x40),
            (0xd7, h.rebase(0x00b5_72b8), 0x146, 0x40),
        ];
        for (id, text, x, cx) in top {
            c(h, WINDOW_ADD_BUTTON, &[window, id, text, x, 0, cx, 8]);
        }
        for k in 0..18 {
            let y = 0x14 + 0xa * k;
            c(
                h,
                WINDOW_ADD_BUTTON,
                &[window, 0x63 + 2 * k, minus, 8, y, 0x12, 8],
            );
            c(
                h,
                WINDOW_ADD_BUTTON,
                &[window, 0x64 + 2 * k, plus, 0x1e, y, 0x12, 8],
            );
        }
    }

    /// An integer field stepped down (not below 0) or up (capped at
    /// `cap`, or `set` past `cap`), read with `read` and stored low.
    fn chr2_int(
        h: &mut impl Host,
        p: u32,
        signed: bool,
        up: bool,
        step: i32,
        cap: i32,
        set: i32,
    ) -> u32 {
        let v = if signed {
            i32::from(h.read::<i8>(p))
        } else {
            i32::from(h.read::<u8>(p))
        };
        let v = if up {
            let v = v.wrapping_add(step);
            if v > cap { set } else { v }
        } else {
            let v = v.wrapping_sub(step);
            if v < 0 { 0 } else { v }
        };
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // mov byte
        h.write(p, v as u8);
        v.cast_unsigned()
    }

    /// The button `param` with the step `step` on the actor `chr`.
    #[allow(clippy::too_many_lines)]
    fn chr2_step(h: &mut impl Host, buf: u32, chr: u32, model: u32, param: u32, step: i32) {
        let at = |o: u32| chr.wrapping_add(o);
        let actor = h.rebase(CHR2_ACTOR);
        match param {
            0x61 | 0x62 => {
                let v = h.read::<u32>(actor).cast_signed();
                if step != 1 {
                    h.write(actor, if v >= 0x14 { 0_u32 } else { 0x14 });
                } else if param == 0x61 {
                    let v = v.wrapping_sub(1);
                    h.write(actor, v);
                    if v < 0 {
                        h.write(actor, 0x1e_u32);
                    }
                } else {
                    let v = v.wrapping_add(1);
                    h.write(actor, v);
                    if v >= 0x1f {
                        h.write(actor, 0_u32);
                    }
                }
            }
            0x63..=0x72 => {
                let k = ((param - 0x63) / 2) as usize;
                let (_, off, ksym) = CHR2_FLOATS[k];
                let up = (param - 0x63) & 1 == 1;
                let d = x87::mul(f64::from(step), k64(&*h, ksym));
                let p = at(off);
                let v = if up {
                    x87::add(d, f(&*h, p))
                } else {
                    x87::sub(f(&*h, p), d)
                };
                h.write(p, x87::store(v));
            }
            0x73 | 0x74 => {
                let p = at(0x4dc);
                let w = f(&*h, p);
                let s = f64::from(step);
                let w = x87::store(if param == 0x74 {
                    x87::add(w, s)
                } else {
                    x87::sub(w, s)
                });
                h.write(p, w);
                if model != 0 {
                    c(h, CH_SET_WEIGHT, &[model, w]);
                }
            }
            0x75..=0x78 => {
                let p = at(if param <= 0x76 { 0x4b0 } else { 0x4b2 });
                let v = i32::from(h.read::<u16>(p));
                let v = if param & 1 == 0 {
                    v.wrapping_add(step)
                } else {
                    let v = v.wrapping_sub(step);
                    if v < 0 { 0 } else { v }
                };
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // mov word
                h.write(p, v as u16);
            }
            0x79 | 0x7a => {
                chr2_int(h, at(0x4b9), false, param == 0x7a, step, 0x80, 0x80);
            }
            0x7b | 0x7c => {
                chr2_int(h, at(0x4ba), false, param == 0x7c, step, 0x80, 0x80);
            }
            0x7d | 0x7e => {
                chr2_int(h, at(0x4b8), true, param == 0x7e, step, 0x7e, 0x7f);
            }
            0x7f | 0x80 => {
                let v = chr2_int(h, at(0x4bb), false, param == 0x80, step, 0x7f, 0x7f);
                c(h, SET_MOT_BIN, &[chr, v & 0xff]);
            }
            0x81..=0x86 => {
                let off = [0x40c, 0x40d, 0x40e][((param - 0x81) / 2) as usize];
                chr2_int(h, at(off), false, param & 1 == 0, step, 0xff, 0xff);
            }
            0xd5 => {
                for off in [0x4bc, 0x4c0, 0x4c8, 0x4d4, 0x4d8, 0x4cc, 0x4d0, 0x4dc] {
                    let i = x87::ftol(&*h, f(&*h, at(off)));
                    #[allow(clippy::cast_precision_loss)] // fild, then fstp m32
                    h.write(at(off), (i.cast_signed() as f32).to_bits());
                }
            }
            0xd6 => {
                let name = h.rebase(0x00b5_48f0);
                c(h, PLY_SAVE_NAME, &[buf, name]);
                let m = h.rebase(0x00b5_72c0);
                c(h, DBG_PRINTF, &[m, buf]);
                c(h, PLY_SAVE, &[buf]);
            }
            0xd7 => {
                let k: u32 = h.read(actor);
                if k.cast_signed() >= 0x14 {
                    let id: u32 = h.read(at(4));
                    c(h, SET_FILE_NAME, &[buf, id]);
                    let m = h.rebase(0x00b5_72c0);
                    c(h, DBG_PRINTF, &[m, buf]);
                    let k: u32 = h.read(actor);
                    let id: u32 = h.read(at(4));
                    c(h, MON_SAVE, &[buf, k, id]);
                }
            }
            _ => {}
        }
    }

    /// `Ms_Gui_BattleChr2Proc`'s 0x8011: the actor's name and id, then
    /// its rows (value at x 0x36, label at x 0x98).
    fn chr2_draw(h: &mut impl Host, buf: u32) {
        let actor = gw(&*h, CHR2_ACTOR);
        let chr = c(h, GET_CHR, &[actor]);
        c(h, PRINT_SET_X, &[0x36]);
        c(h, PRINT_SET_Y, &[1]);
        let actor = gw(&*h, CHR2_ACTOR);
        let name = gw(&*h, CHR2_NAMES.wrapping_add(4 * actor));
        c(h, STR_COPY, &[name, buf, 0x100]);
        c(h, PRINTF, &[buf]);
        c(h, PRINT_SET_X, &[0x98]);
        c(h, PRINT_SET_Y, &[1]);
        if gw(&*h, CHR2_ACTOR).cast_signed() < 0x14 {
            let id = u32::from(h.read::<u8>(chr.wrapping_add(0xc)));
            sprintf_r(h, buf, 0x00b5_72cc, &[id]);
        } else {
            let id = h.read::<u32>(chr.wrapping_add(4)) & 0x3ff;
            sprintf_r(h, buf, 0x00b5_72d8, &[id]);
        }
        c(h, PRINTF, &[buf]);
        for (y, off, kind, label) in CHR2_ROWS {
            c(h, PRINT_SET_X, &[0x36]);
            c(h, PRINT_SET_Y, &[y]);
            let p = chr.wrapping_add(off);
            if kind == 0 {
                let [lo, hi] = words64(&*h, p);
                sprintf_r(h, buf, FLOAT_FMT, &[lo, hi]);
            } else {
                let v = match kind {
                    1 => u32::from(h.read::<u16>(p)),
                    2 => u32::from(h.read::<u8>(p)),
                    _ => i32::from(h.read::<i8>(p)).cast_unsigned(),
                };
                sprintf_r(h, buf, INT_FMT, &[v]);
            }
            c(h, PRINTF, &[buf]);
            c(h, PRINT_SET_X, &[0x98]);
            c(h, PRINT_SET_Y, &[y]);
            let label = h.rebase(label);
            c(h, PRINTF, &[label]);
        }
    }

    /// What `Ms_Gui_BattleConfigProc` calls besides the window callees:
    /// `MsDebugEnvSave(name)`, `MsDebugFullItem()`,
    /// `MsDebugFullCommand()`, `MsDebugFullMember()`,
    /// `MsDebugSetAllSP_00784ae0()`, `MsDebugFullGill()`,
    /// `MsDebugFullStats_00784b00()`, `MsDebugWeakAll_00784b80()`,
    /// `MsDebugAllItemsOne_00784c20()`, `MsDebugAbilityOne_00784c40()`,
    /// `MsDebugClearItems_00784cd0()`, `MsDebugWeakParty_00784c80(on)`,
    /// `MsDebugWeakMonsters_00784bc0(on)`, `MsCameraPriorityMask(on)`.
    pub const ENV_SAVE: u32 = 0x007c_d800;
    pub const FULL_ITEM: u32 = 0x0078_49d0;
    pub const FULL_COMMAND: u32 = 0x0078_48b0;
    pub const FULL_MEMBER: u32 = 0x0078_4ac0;
    pub const ALL_SP: u32 = 0x0078_4ae0;
    pub const FULL_GILL: u32 = 0x0078_49c0;
    pub const FULL_STATS: u32 = 0x0078_4b00;
    pub const WEAK_ALL: u32 = 0x0078_4b80;
    pub const ALL_ITEMS_ONE: u32 = 0x0078_4c20;
    pub const ABILITY_ONE: u32 = 0x0078_4c40;
    pub const CLEAR_ITEMS: u32 = 0x0078_4cd0;
    pub const WEAK_PARTY: u32 = 0x0078_4c80;
    pub const WEAK_MONSTERS: u32 = 0x0078_4bc0;
    pub const CAMERA_PRIORITY_MASK: u32 = 0x007b_e100;

    /// The window's two auto-repeats, its open flag, the env file name.
    pub const CONFIG_REPEAT_A: u32 = 0x0113_9eb0;
    pub const CONFIG_REPEAT_B: u32 = 0x0113_9ec4;
    pub const CONFIG_OPEN: u32 = 0x0113_9efc;
    pub const ENV_NAME: u32 = 0x0231_1400;
    pub const CONFIG_TITLE: u32 = 0x00b5_5294;

    /// The command buttons along the top: (id, text, x, cx); y 0, cy 8.
    pub const CONFIG_COMMANDS: [(u32, u32, u32, u32); 7] = [
        (0x5f, 0x00b5_5228, 0x2, 0x2e),
        (0x97, 0x00b5_52a4, 0x34, 0x2e),
        (0x9d, 0x00b5_52ac, 0x66, 0x2e),
        (0xe7, 0x00b5_52b0, 0x98, 0x2e),
        (0xcb, 0x00b5_52b4, 0xca, 0x2e),
        (0xe1, 0x00b5_52b8, 0xfc, 0x2e),
        (0xf9, 0x00b5_52bc, 0x12e, 0x1c),
    ];

    /// The on/off buttons in creation order (19 a column, x 2 then
    /// 0x13a, y from 0xa by 0xa): (id, the `u8` debug flag).
    pub const CONFIG_TOGGLES: [(u32, u32); 38] = [
        (0x9c, 0x0112_a905),
        (0x53, 0x0112_a8e1),
        (0x8e, 0x0112_a8fb),
        (0x60, 0x0112_a8fc),
        (0x5d, 0x0112_a8f5),
        (0x8b, 0x0112_a8f9),
        (0x5b, 0x0112_a8f8),
        (0x98, 0x0112_a902),
        (0x89, 0x0112_a900),
        (0x8a, 0x0112_a901),
        (0x9a, 0x0112_a904),
        (0x92, 0x0112_a908),
        (0x94, 0x0112_a90a),
        (0xd2, 0x0112_a90d),
        (0x93, 0x0112_a909),
        (0x9b, 0x0112_a906),
        (0xd1, 0x0112_a914),
        (0xc2, 0x0112_a90c),
        (0xc7, 0x0112_a916),
        (0x95, 0x0112_a90b),
        (0xca, 0x0112_a91a),
        (0x8c, 0x0112_a8fa),
        (0xc3, 0x0112_a922),
        (0xc4, 0x0112_a915),
        (0xc5, 0x0112_a907),
        (0xc8, 0x0112_a918),
        (0xc9, 0x0112_a91c),
        (0xcc, 0x0112_a917),
        (0xd4, 0x0112_a91b),
        (0xd8, 0x0112_a91d),
        (0xd9, 0x0112_a91e),
        (0xda, 0x0112_a91f),
        (0xdb, 0x0112_a920),
        (0xdd, 0x0112_a921),
        (0xe3, 0x0112_a924),
        (0xe5, 0x0112_a925),
        (0xe6, 0x0112_a926),
        (0xe8, 0x0112_a927),
    ];

    /// The order 0x8011 sets the on/off texts in (ids).
    pub const CONFIG_TEXT_ORDER: [u32; 38] = [
        0x9c, 0x53, 0x8e, 0x60, 0x5d, 0x8b, 0x5b, 0x98, 0x89, 0x8a, 0x8c, 0x92, 0x94, 0xd2, 0x93,
        0x9a, 0x95, 0x9b, 0xc2, 0xc7, 0xc3, 0xc4, 0xc5, 0xc8, 0xc9, 0xca, 0xcc, 0xd1, 0xd4, 0xd8,
        0xd9, 0xda, 0xdb, 0xdd, 0xe3, 0xe5, 0xe6, 0xe8,
    ];

    /// The row labels (19 at x 0x18, then 19 at x 0xb2; y from 0xb by
    /// 0xa), each copied through `sprintf` into the frame buffer.
    pub const CONFIG_LABELS: [u32; 38] = [
        0x00b5_52c0,
        0x00b5_52cc,
        0x00b5_52d8,
        0x00b5_52e8,
        0x00b5_52f8,
        0x00b5_5308,
        0x00b5_5318,
        0x00b5_5328,
        0x00b5_5334,
        0x00b5_5340,
        0x00b5_534c,
        0x00b5_5358,
        0x00b5_5368,
        0x00b5_5378,
        0x00b5_5384,
        0x00b5_5394,
        0x00b5_53a4,
        0x00b5_53b4,
        0x00b5_53c4,
        0x00b5_53d4,
        0x00b5_53e0,
        0x00b5_53f0,
        0x00b5_53fc,
        0x00b5_540c,
        0x00b5_5418,
        0x00b5_5428,
        0x00b5_5438,
        0x00b5_5448,
        0x00b5_5458,
        0x00b5_5464,
        0x00b5_546c,
        0x00b5_5474,
        0x00b5_5484,
        0x00b5_548c,
        0x00b5_5498,
        0x00b5_54a4,
        0x00b5_54b4,
        0x00b5_54c0,
    ];

    pub const CONFIG_FRAME: u32 = 0x104;

    /// The on/off text for the `u8` flag at `sym`, indexed sign-extended
    /// (`movsx`) as the original does.
    fn flag_text(h: &impl Host, sym: u32) -> u32 {
        let k = sb(h, sym);
        h.read(h.rebase(ON_OFF).wrapping_add(k.wrapping_mul(4)))
    }

    fn config_flag(id: u32) -> Option<u32> {
        CONFIG_TOGGLES.iter().find(|t| t.0 == id).map(|t| t.1)
    }

    /// `Ms_Gui_BattleConfigProc(window, msg, param)` (`0x007c6e20`) -> 0:
    /// the battle debug settings window. 0x8001 builds it (two
    /// auto-repeats, 7 command buttons, 38 on/off buttons), 0x8003 closes
    /// it, 0x8006/0x800a reset an auto-repeat; 0x8005/0x8009 (through
    /// the auto-repeat) flip a flag (`(v - 1) & 1`; party / monster HP 1
    /// and the camera priority mask also pass the new value on) or run a
    /// command (env save, full items/commands/members/SP/gil, full
    /// stats, weak all, every item once, abilities once, clear items);
    /// 0x8011 sets the on/off texts and prints the labels.
    pub fn battle_config_proc(
        h: &mut impl Host,
        ebp: u32,
        window: u32,
        msg: u32,
        param: u32,
    ) -> u32 {
        match msg {
            0x8001 => {
                let title = h.rebase(CONFIG_TITLE);
                c(h, WINDOW_SET_TITLE, &[window, title]);
                for r in [CONFIG_REPEAT_A, CONFIG_REPEAT_B] {
                    let r = h.rebase(r);
                    c(h, AUTO_REPEAT_INIT, &[r, 0xf, 3]);
                }
                for (id, text, x, cx) in CONFIG_COMMANDS {
                    let text = h.rebase(text);
                    c(h, WINDOW_ADD_BUTTON, &[window, id, text, x, 0, cx, 8]);
                }
                for (k, (id, flag)) in (0_u32..).zip(CONFIG_TOGGLES) {
                    let (x, y) = if k < 19 {
                        (2, 0xa + 0xa * k)
                    } else {
                        (0x13a, 0xa + 0xa * (k - 19))
                    };
                    let text = flag_text(&*h, flag);
                    c(h, WINDOW_ADD_BUTTON, &[window, id, text, x, y, 0xe, 8]);
                }
            }
            0x8003 => {
                let at = h.rebase(CONFIG_OPEN);
                h.write(at, 0_u32);
            }
            0x8006 | 0x800a => {
                let r = h.rebase(if msg == 0x8006 {
                    CONFIG_REPEAT_A
                } else {
                    CONFIG_REPEAT_B
                });
                c(h, AUTO_REPEAT_INIT, &[r, 0xf, 3]);
            }
            0x8005 | 0x8009 => {
                let r = h.rebase(if msg == 0x8005 {
                    CONFIG_REPEAT_A
                } else {
                    CONFIG_REPEAT_B
                });
                if c(h, AUTO_REPEAT_CHECK, &[r, param]) != 0 {
                    config_step(h, param);
                }
            }
            0x8011 => config_draw(h, ebp, window),
            _ => {}
        }
        0
    }

    fn config_step(h: &mut impl Host, param: u32) {
        if let Some(flag) = config_flag(param) {
            let at = h.rebase(flag);
            let v = h.read::<u8>(at).wrapping_sub(1) & 1;
            h.write(at, v);
            let pass = match param {
                0xd8 => WEAK_PARTY,
                0xd9 => WEAK_MONSTERS,
                0xe3 => CAMERA_PRIORITY_MASK,
                _ => return,
            };
            c(h, pass, &[u32::from(v)]);
            return;
        }
        let calls: &[u32] = match param {
            0x5f => {
                let name = h.rebase(ENV_NAME);
                c(h, ENV_SAVE, &[name]);
                &[]
            }
            0x97 => &[FULL_ITEM, FULL_COMMAND, FULL_MEMBER, ALL_SP, FULL_GILL],
            0x9d => &[WEAK_ALL],
            0xcb => &[ALL_ITEMS_ONE],
            0xe1 => &[ABILITY_ONE],
            0xe7 => &[CLEAR_ITEMS],
            0xf9 => &[FULL_ITEM, FULL_STATS],
            _ => &[],
        };
        for &f in calls {
            c(h, f, &[]);
        }
    }

    fn config_draw(h: &mut impl Host, ebp: u32, window: u32) {
        let buf = ebp.wrapping_sub(CONFIG_FRAME);
        for id in CONFIG_TEXT_ORDER {
            let flag = config_flag(id).unwrap_or(0);
            let text = flag_text(&*h, flag);
            c(h, WINDOW_SET_TEXT, &[window, id, text]);
        }
        for (k, label) in (0_u32..).zip(CONFIG_LABELS) {
            let (x, y) = if k < 19 {
                (0x18, 0xb + 0xa * k)
            } else {
                (0xb2, 0xb + 0xa * (k - 19))
            };
            c(h, PRINT_SET_X, &[x]);
            c(h, PRINT_SET_Y, &[y]);
            sprintf_r(h, buf, label, &[]);
            c(h, PRINTF, &[buf]);
        }
    }
}
