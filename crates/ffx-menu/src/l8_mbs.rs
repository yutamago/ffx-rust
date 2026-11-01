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
}
