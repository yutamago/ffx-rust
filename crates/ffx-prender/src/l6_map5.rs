//! M5 stage 1, round 6 (list `left6`, from `map2.left`), batch 3: the
//! scene text loader's camera record (`rcbgSceneLoadTextSub`'s camera
//! line: a position and target, or a position and three angles).
//!
//! Callees stay calls to the originals (cdecl); `rcdtParseGetFloat` returns
//! in `ST0` (stored as the original stores it), the angles go through the
//! CRT's `_CIfmod`.

/// `rcdtParseIsEnd(p)`, `rcdtParseGetKeyword(p, table)`,
/// `rcdtParseNextToken(p)`, `rcdtParseGetFloat(p)` (`ST0`),
/// `rcbgSceneSetMatrix(scene)`, `rcbgSceneLoadTextCameraCalcRef(scene)`.
pub const PARSE_IS_END: u32 = 0x0092_bfb0;
pub const PARSE_GET_KEYWORD: u32 = 0x0092_be40;
pub const PARSE_NEXT_TOKEN: u32 = 0x0092_ba30;
pub const PARSE_GET_FLOAT: u32 = 0x0092_bbb0;
pub const SCENE_SET_MATRIX: u32 = 0x0092_0c00;
pub const CAMERA_CALC_REF: u32 = 0x0093_b110;
/// The keyword table (the target form), `_CIfmod`'s thunk, f64 360,
/// f32 -180, 180 and -1.
pub const TARGET_KEYWORDS: u32 = 0x00c6_0ea4;
pub const CI_FMOD: u32 = 0x0094_98e0;
pub const DEG_360: u32 = 0x00b4_8380;
pub const DEG_MINUS_180: u32 = 0x00b5_5f80;
pub const DEG_180: u32 = 0x00b5_a480;
pub const MINUS_ONE: u32 = 0x00b9_2340;

pub mod bridge {
    use super::{
        CAMERA_CALC_REF, CI_FMOD, DEG_180, DEG_360, DEG_MINUS_180, MINUS_ONE, PARSE_GET_FLOAT,
        PARSE_GET_KEYWORD, PARSE_IS_END, PARSE_NEXT_TOKEN, SCENE_SET_MATRIX, TARGET_KEYWORDS,
    };
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// The next float (`fstp dword`), negated first with `neg` (`fchs`).
    fn float(h: &mut impl Host, parse: u32, neg: bool) -> u32 {
        let v = h
            .call_sym_st0(PARSE_GET_FLOAT, Conv::Cdecl, &[parse])
            .to_bits();
        if neg { v ^ 0x8000_0000 } else { v }
    }

    /// `scene + off` = `v`.
    fn w(h: &mut impl Host, scene: u32, off: u32, v: u32) {
        h.write(scene.wrapping_add(off), v);
    }

    /// An `f32` constant as `fld dword` loads it.
    fn k32(h: &impl Host, sym: u32) -> f64 {
        x87::load(h.read(h.rebase(sym)))
    }

    /// `FUN_0093af60(scene, parse)` -> `eax` as the original: at the end
    /// of the text `rcdtParseIsEnd`'s; else the camera line. With the
    /// target keyword: the position (`+0x68`, y and z negated) and the
    /// target (`+0x78`, likewise), the up vector (0, -1, 0 at `+0x88`),
    /// three floats skipped and the field of view (`+0xa8`); otherwise the
    /// position, three angles (`+0x98`, y and z negated) each wrapped into
    /// -180..180 (`fmod` 360, then +360 below -180, -360 above 180, each
    /// step rounded to f32) and the field of view, then the reference
    /// point (`rcbgSceneLoadTextCameraCalcRef`). Then the scene's matrix
    /// (`rcbgSceneSetMatrix`), whose `eax` it returns.
    pub fn camera_line(h: &mut impl Host, scene: u32, parse: u32) -> u32 {
        let end = cdecl(h, PARSE_IS_END, &[parse]);
        if end != 0 {
            return end;
        }
        let table = h.rebase(TARGET_KEYWORDS);
        if cdecl(h, PARSE_GET_KEYWORD, &[parse, table]) != 0 {
            cdecl(h, PARSE_NEXT_TOKEN, &[parse]);
            cdecl(h, PARSE_NEXT_TOKEN, &[parse]);
            for (off, neg) in [
                (0x68, false),
                (0x6c, true),
                (0x70, true),
                (0x78, false),
                (0x7c, true),
                (0x80, true),
            ] {
                let v = float(h, parse, neg);
                w(h, scene, off, v);
            }
            w(h, scene, 0x88, 0);
            let m1 = x87::store(k32(h, MINUS_ONE));
            w(h, scene, 0x8c, m1);
            w(h, scene, 0x90, 0);
            for _ in 0..3 {
                float(h, parse, false);
            }
            let fov = float(h, parse, false);
            w(h, scene, 0xa8, fov);
            return cdecl(h, SCENE_SET_MATRIX, &[scene]);
        }
        cdecl(h, PARSE_NEXT_TOKEN, &[parse]);
        for (off, neg) in [
            (0x68, false),
            (0x6c, true),
            (0x70, true),
            (0x98, false),
            (0x9c, true),
            (0xa0, true),
            (0xa8, false),
        ] {
            let v = float(h, parse, neg);
            w(h, scene, off, v);
        }
        let full = f64::from_bits(h.read(h.rebase(DEG_360)));
        let fmod = h.rebase(CI_FMOD);
        for off in [0x98_u32, 0x9c, 0xa0] {
            let a = x87::load(h.read(scene.wrapping_add(off)));
            let mut v = x87::load(h.call_intrinsic2(fmod, a, full).to_bits());
            if k32(h, DEG_MINUS_180) > v {
                v = x87::load(x87::store(x87::add(v, full)));
            }
            if k32(h, DEG_180) < v {
                v = x87::load(x87::store(x87::sub(v, full)));
            }
            w(h, scene, off, x87::store(v));
        }
        cdecl(h, CAMERA_CALC_REF, &[scene]);
        cdecl(h, SCENE_SET_MATRIX, &[scene])
    }

    /// `Gu_WindowGetArg`, `rcuiAddItems`, `Gu_WindowAddClient(window, id,
    /// x, y, w, h)`, `Gu_WindowCloseTree`, the mouse move helpers,
    /// `rcuiGaugeDraw_0093c730(window, x, y, v)`, the print helpers.
    pub const WINDOW_GET_ARG: u32 = 0x0084_aec0;
    pub const ADD_ITEMS: u32 = 0x0093_7780;
    pub const ADD_CLIENT: u32 = 0x0084_a4a0;
    pub const CLOSE_TREE: u32 = 0x0084_ab80;
    pub const MOVE_RESET: u32 = 0x0093_c110;
    pub const MOVE_BEGIN: u32 = 0x0093_c150;
    pub const MOVE_END: u32 = 0x0093_c050;
    pub const MOVE_IS_ID: u32 = 0x0093_c120;
    pub const MOVE_GET: u32 = 0x0093_c060;
    pub const MOVE_GET_ID: u32 = 0x0093_c100;
    pub const GAUGE_DRAW: u32 = 0x0093_c730;
    pub const PRINT_SET_X: u32 = 0x0084_8820;
    pub const PRINT_SET_Y: u32 = 0x0084_8860;
    pub const GU_PRINTF: u32 = 0x0084_8880;
    pub const PRINT_SET_METRICS: u32 = 0x0084_87e0;
    /// The slider's items, f64 100, its texts (the two values, the unit,
    /// the three scale marks).
    pub const SLIDER_ITEMS: u32 = 0x00c6_1090;
    pub const HUNDRED: u32 = 0x00b9_22e0;
    pub const SLIDER_TEXTS: [u32; 6] = [
        0x00b6_8e44,
        0x00b5_6864,
        0x00b6_8e4c,
        0x00b6_8e50,
        0x00b6_8e54,
        0x00b6_8e58,
    ];
    /// `0x0093c860`'s frame.
    pub const SLIDER_FRAME: u32 = 0xc;

    /// `e + off` += `d`.
    fn add(h: &mut impl Host, e: u32, off: u32, d: u32) {
        let v: u32 = h.read(e.wrapping_add(off));
        h.write(e.wrapping_add(off), v.wrapping_add(d));
    }

    /// The slider's two parts clamped (`+0xc` 0..999, `+0x10` 0..99) and
    /// the value `f32(f32(b) / 100 + f32(a))` into the target (`[+4]`);
    /// the original keeps `a` at `ebp - 8` and the floats in its `msg`
    /// slot.
    fn slider_value(h: &mut impl Host, ebp: u32, slot: u32, e: u32) {
        let clamp = |v: u32, hi: u32| -> u32 {
            if v.cast_signed() < 0 {
                0
            } else if v.cast_signed() > hi.cast_signed() {
                hi
            } else {
                v
            }
        };
        let a = clamp(h.read(e.wrapping_add(0xc)), 999);
        h.write(ebp.wrapping_sub(8), a);
        h.write(e.wrapping_add(0xc), a);
        let b = clamp(h.read(e.wrapping_add(0x10)), 99);
        h.write(e.wrapping_add(0x10), b);
        let hundred = f64::from_bits(h.read(h.rebase(HUNDRED)));
        let fb = x87::load(x87::store(f64::from(b.cast_signed())));
        let q = x87::div(fb, hundred);
        let fa = x87::store(f64::from(a.cast_signed()));
        let v = x87::store(x87::add(q, x87::load(fa)));
        h.write(slot, v);
        let target: u32 = h.read(e.wrapping_add(4));
        h.write(target, v);
    }

    /// `FUN_0093c860(window, msg, param; slots)` -> `eax` as the original:
    /// the slider window's procedure (a value as 0..999 and 0..99
    /// hundredths). The entry is the window's argument (`+0` shown, `+4`
    /// the target float, `+8` the old value, `+0xc`/`+0x10` the parts,
    /// `+0x14` cancelled, `+0x18` restored, `+0x1c` changed). 0x8001 adds
    /// its items and the two gauges; 0x8003 hides it; 0x8007 on a gauge
    /// (8, 9) starts the mouse move; 0x8006 the buttons (0..7 steps of 10
    /// and 1, 0xa closes, 0xb restores the old value and closes), then the
    /// value and the mouse move ended; 0x8011 follows the mouse, sets the
    /// value and draws the gauges and texts. `ebp` is the original's frame
    /// (the mouse deltas, `a`), `slots` its argument words.
    #[allow(clippy::too_many_lines)]
    pub fn slider_proc(
        h: &mut impl Host,
        ebp: u32,
        slots: u32,
        window: u32,
        msg: u32,
        param: u32,
    ) -> u32 {
        let e = cdecl(h, WINDOW_GET_ARG, &[window]);
        let slot = slots.wrapping_add(4);
        match msg {
            0x8001 => {
                let items = h.rebase(SLIDER_ITEMS);
                cdecl(h, ADD_ITEMS, &[window, items]);
                cdecl(h, ADD_CLIENT, &[window, 8, 0x8a, 6, 0x10, 0x65]);
                cdecl(h, ADD_CLIENT, &[window, 9, 0x9e, 6, 0x10, 0x65]);
                cdecl(h, MOVE_RESET, &[])
            }
            0x8003 => {
                h.write(e, 0_u32);
                e
            }
            0x8007 => {
                if param == 8 || param == 9 {
                    cdecl(h, MOVE_BEGIN, &[window, param])
                } else {
                    param
                }
            }
            0x8006 => {
                let step = match param {
                    0 => Some((0xc, 10)),
                    1 => Some((0xc, (-10_i32).cast_unsigned())),
                    2 => Some((0xc, 1)),
                    3 => Some((0xc, u32::MAX)),
                    4 => Some((0x10, 10)),
                    5 => Some((0x10, (-10_i32).cast_unsigned())),
                    6 => Some((0x10, 1)),
                    7 => Some((0x10, u32::MAX)),
                    _ => None,
                };
                if let Some((off, d)) = step {
                    add(h, e, off, d);
                    h.write(e.wrapping_add(0x1c), 1_u32);
                } else if param == 0xa {
                    cdecl(h, CLOSE_TREE, &[window]);
                    h.write(e.wrapping_add(0x14), 1_u32);
                } else if param == 0xb {
                    let target: u32 = h.read(e.wrapping_add(4));
                    let old: u32 = h.read(e.wrapping_add(8));
                    h.write(target, x87::store(x87::load(old)));
                    let r = cdecl(h, CLOSE_TREE, &[window]);
                    h.write(e.wrapping_add(0x18), 1_u32);
                    return r;
                }
                slider_value(h, ebp, slot, e);
                cdecl(h, MOVE_END, &[])
            }
            0x8011 => {
                if e == 0 || h.read::<u32>(e) == 0 {
                    return e;
                }
                if cdecl(h, MOVE_IS_ID, &[window]) != 0 {
                    let (dy, dx) = (ebp.wrapping_sub(4), ebp.wrapping_sub(0xc));
                    cdecl(h, MOVE_GET, &[dx, dy]);
                    let id = cdecl(h, MOVE_GET_ID, &[]);
                    if id == 8 || id == 9 {
                        let d: u32 = h.read(dy);
                        add(
                            h,
                            e,
                            if id == 8 { 0xc } else { 0x10 },
                            0_u32.wrapping_sub(d),
                        );
                        if h.read::<u32>(dy) != 0 {
                            h.write(e.wrapping_add(0x1c), 1_u32);
                        }
                    }
                    slider_value(h, ebp, slot, e);
                }
                let a: u32 = h.read(e.wrapping_add(0xc));
                cdecl(h, GAUGE_DRAW, &[window, 0x8a, 6, a]);
                let b: u32 = h.read(e.wrapping_add(0x10));
                cdecl(h, GAUGE_DRAW, &[window, 0x9e, 6, b]);
                let t = SLIDER_TEXTS.map(|x| h.rebase(x));
                for (y, fmt, off) in [(0xf_u32, t[0], 0xc_u32), (0x1c, t[1], 0x10)] {
                    cdecl(h, PRINT_SET_X, &[0x20]);
                    cdecl(h, PRINT_SET_Y, &[y]);
                    let v: u32 = h.read(e.wrapping_add(off));
                    cdecl(h, GU_PRINTF, &[fmt, v]);
                }
                cdecl(h, PRINT_SET_METRICS, &[0x90]);
                cdecl(h, GU_PRINTF, &[t[2]]);
                cdecl(h, PRINT_SET_METRICS, &[0xc0]);
                let mut r = 0;
                for (y, fmt) in [(2_u32, t[3]), (0x34, t[4]), (0x66, t[5])] {
                    cdecl(h, PRINT_SET_X, &[0x70]);
                    cdecl(h, PRINT_SET_Y, &[y]);
                    r = cdecl(h, GU_PRINTF, &[fmt]);
                }
                r
            }
            _ => e,
        }
    }

    /// `g_RcuiWork`.
    pub const RCUI_WORK: u32 = 0x0193_c0f0;
    /// `MsEffectEndMotion(i, kind)`, `rcbgChrHandleIndex(chr)`,
    /// `MsEffectSetMotion(i, motion)`, `Ch_GetSeqFrame(chr)`,
    /// `rcuiMotionStep(d)`, `rcuiNumberCreate(holder, value, kind, title,
    /// parent)`, `Ch_SetMotionSpeed(chr, speed)`, `Ch_MotionGetFrame(chr)`,
    /// `Gu_WindowControlSetText(window, id, s)`.
    pub const END_MOTION: u32 = 0x0078_7950;
    pub const CHR_HANDLE_INDEX: u32 = 0x0092_4f30;
    pub const SET_MOTION: u32 = 0x0078_82c0;
    pub const GET_SEQ_FRAME: u32 = 0x0083_6af0;
    pub const MOTION_STEP: u32 = 0x0093_5690;
    pub const NUMBER_CREATE: u32 = 0x0093_c2e0;
    pub const SET_MOTION_SPEED: u32 = 0x0082_b380;
    pub const MOTION_GET_FRAME: u32 = 0x0083_9860;
    pub const SET_TEXT: u32 = 0x0084_a710;
    /// The motion window's items, the motion table (8 bytes: id, name),
    /// the speed entry's title, its frame format, `"%d"`, `sprintf`.
    pub const MOTION_ITEMS: u32 = 0x00c6_00f0;
    pub const MOTIONS: u32 = 0x00c6_0a78;
    pub const SPEED_TITLE: u32 = 0x00b6_8710;
    pub const FRAME_FORMAT: u32 = 0x00b6_8720;
    pub const FORMAT_D: u32 = 0x00b0_f604;
    pub const SPRINTF_SLOT: u32 = 0x00b0_c3dc;
    /// `rcuiMotionProc`'s frame.
    pub const MOTION_FRAME: u32 = 0x14;

    fn work(h: &impl Host) -> u32 {
        h.read(h.rebase(RCUI_WORK))
    }

    /// `rcuiMotionProc(window, msg, param)` -> `eax` as the original
    /// (`None`: no work area, the cookie's `eax`): the motion test window
    /// for the selected character (`g_RcuiWork + 0x870[+0x8b0]`, -1 none).
    /// 0x8001 adds its items; 0x8003 ends its motion (`MsEffectEndMotion(chr,
    /// 0)`) and clears `+0x40`; 0x8006 buttons 0x99 play the motion
    /// `+0x8d8` (the frame count to `+0x8e0`), 0x9a/0x9d step a group,
    /// 0x9b/0x9c step one (wrapping 0..0x68), 0x9e the speed entry, 0x9f
    /// pause; 0x800a 0x9e speed 0, 0x9f run; 0x8011 the speed
    /// (`Ch_SetMotionSpeed`: `+0x8d4`, 0 paused, 0x100 once after a step),
    /// the motion's name and speed texts and the frame. `ebp` is the
    /// original's frame (the speed text at `ebp - 0x14`).
    #[allow(clippy::too_many_lines)]
    pub fn motion_proc(
        h: &mut impl Host,
        ebp: u32,
        window: u32,
        msg: u32,
        param: u32,
    ) -> Option<u32> {
        let w = work(h);
        if w == 0 {
            return None;
        }
        let sel: u32 = h.read(w.wrapping_add(0x8b0));
        let chr: u32 = h.read(w.wrapping_add(0x870).wrapping_add(sel.wrapping_mul(4)));
        let none = chr == u32::MAX;
        let i = msg.wrapping_sub(0x8001);
        Some(match msg {
            0x8001 => {
                let items = h.rebase(MOTION_ITEMS);
                cdecl(h, ADD_ITEMS, &[window, items])
            }
            0x8003 => {
                let (r, w) = if none {
                    (1, w)
                } else {
                    let r = cdecl(h, END_MOTION, &[chr, 0]);
                    (r, work(h))
                };
                h.write(w.wrapping_add(0x40), 0_u32);
                r
            }
            0x8006 => match param {
                0x99 => {
                    if none {
                        return Some(0);
                    }
                    let k = cdecl(h, CHR_HANDLE_INDEX, &[chr]);
                    let m: u32 = h.read(work(h).wrapping_add(0x8d8));
                    let id: u32 = h.read(h.rebase(MOTIONS).wrapping_add(m.wrapping_mul(8)));
                    cdecl(h, SET_MOTION, &[k, id]);
                    let f = cdecl(h, GET_SEQ_FRAME, &[chr]);
                    h.write(work(h).wrapping_add(0x8e0), f);
                    f
                }
                0x9a => cdecl(h, MOTION_STEP, &[u32::MAX]),
                0x9d => cdecl(h, MOTION_STEP, &[1]),
                0x9b => {
                    let m = h.read::<u32>(w.wrapping_add(0x8d8)).wrapping_sub(1);
                    h.write(w.wrapping_add(0x8d8), m);
                    if m.cast_signed() >= 0 {
                        return Some(2);
                    }
                    let w = work(h);
                    h.write(w.wrapping_add(0x8d8), 0x68_u32);
                    w
                }
                0x9c => {
                    let m = h.read::<u32>(w.wrapping_add(0x8d8)).wrapping_add(1);
                    h.write(w.wrapping_add(0x8d8), m);
                    if m < 0x69 {
                        return Some(3);
                    }
                    let w = work(h);
                    h.write(w.wrapping_add(0x8d8), 0_u32);
                    w
                }
                0x9e => {
                    let title = h.rebase(SPEED_TITLE);
                    cdecl(
                        h,
                        NUMBER_CREATE,
                        &[
                            w.wrapping_add(0x8e8),
                            w.wrapping_add(0x8d4),
                            5,
                            title,
                            window,
                        ],
                    )
                }
                0x9f => {
                    h.write(w.wrapping_add(0x8dc), 1_u32);
                    let w = work(h);
                    h.write(w.wrapping_add(0x8e4), 1_u32);
                    w
                }
                _ => param.wrapping_sub(0x99),
            },
            0x800a => match param {
                0x9e => {
                    h.write(w.wrapping_add(0x8d4), 0_u32);
                    0
                }
                0x9f => {
                    h.write(w.wrapping_add(0x8dc), 0_u32);
                    0
                }
                _ => param.wrapping_sub(0x9f),
            },
            0x8011 => {
                let mut w = w;
                if !none {
                    let paused = h.read::<u32>(w.wrapping_add(0x8dc)) != 0;
                    let step = paused && h.read::<u32>(w.wrapping_add(0x8e4)) != 0;
                    let speed = match (paused, step) {
                        (false, _) => h.read(w.wrapping_add(0x8d4)),
                        (true, true) => 0x100,
                        (true, false) => 0,
                    };
                    cdecl(h, SET_MOTION_SPEED, &[chr, speed]);
                    if step {
                        let w = work(h);
                        h.write(w.wrapping_add(0x8e4), 0_u32);
                    }
                    w = work(h);
                }
                let m: u32 = h.read(w.wrapping_add(0x8d8));
                let name: u32 = h.read(
                    h.rebase(MOTIONS)
                        .wrapping_add(4)
                        .wrapping_add(m.wrapping_mul(8)),
                );
                cdecl(h, SET_TEXT, &[window, 0x99, name]);
                let v: u32 = h.read(work(h).wrapping_add(0x8d4));
                let buf = ebp.wrapping_sub(0x14);
                let fmt = h.rebase(FORMAT_D);
                let f: u32 = h.read(h.rebase(SPRINTF_SLOT));
                h.call(f, Conv::Cdecl, &[buf, fmt, v]);
                let r = cdecl(h, SET_TEXT, &[window, 0x9e, buf]);
                if none {
                    return Some(r);
                }
                let fr = cdecl(h, MOTION_GET_FRAME, &[chr]).cast_signed() / 256;
                cdecl(h, PRINT_SET_X, &[8]);
                cdecl(h, PRINT_SET_Y, &[0x29]);
                let total: u32 = h.read(work(h).wrapping_add(0x8e0));
                let fmt = h.rebase(FRAME_FORMAT);
                cdecl(h, GU_PRINTF, &[fmt, fr.cast_unsigned(), total])
            }
            _ if i <= 0x10 => 5,
            _ => i,
        })
    }

    /// `g_YnMapSystem`; `rcuiMouseMoveEnd`, `rcuiNumberIsOK(holder)`; the
    /// alpha window's items, its mode names (two tables of four) and the
    /// number entry's title.
    pub const YN_MAP_SYSTEM: u32 = 0x0193_bf80;
    pub const NUMBER_IS_OK: u32 = 0x0093_c3d0;
    pub const ALPHA_ITEMS: u32 = 0x00c5_f1a8;
    pub const ALPHA_NAMES: u32 = 0x00c5_e8a8;
    pub const ALPHA_NAMES_C: u32 = 0x00c5_e8b4;
    pub const ALPHA_TITLE: u32 = 0x00b6_856c;
    /// `rcuiAlphaProc`'s frame.
    pub const ALPHA_FRAME: u32 = 0x1c;

    /// `rcuiAlphaProc(window, msg, param)` -> `eax` as the original
    /// (`None`: no work area, map or layer, the cookie's `eax`): the
    /// selected layer's (`g_RcuiWork + 0x288` of `[map + 8]`) alpha blend
    /// word (`+0x38`: four 2-bit fields A, B, C, D, `+0x3c` the fixed
    /// value FIX). 0x8001 adds its items; 0x8003 (closed) clears `+0x1c`;
    /// 0x8006 ends the mouse move, then buttons 0x3e..0x41 cycle a field
    /// (0, 1, 2; its new value, shifted, in `eax`), 0x42 the FIX entry
    /// (`rcuiNumberCreate`), 0x43..0x46 four presets; 0x8011 takes an
    /// entered FIX (`rcuiNumberIsOK`) and writes the five texts. `ebp` is
    /// the original's frame (the FIX text at `ebp - 0x14`, its byte at
    /// `- 0x18`).
    #[allow(clippy::too_many_lines)]
    pub fn alpha_proc(
        h: &mut impl Host,
        ebp: u32,
        window: u32,
        msg: u32,
        param: u32,
    ) -> Option<u32> {
        let w = work(h);
        if w == 0 {
            return None;
        }
        let sys: u32 = h.read(h.rebase(YN_MAP_SYSTEM));
        if sys == 0 {
            return None;
        }
        let map: u32 = h.read(sys);
        if map == 0 {
            return None;
        }
        let list: u32 = h.read(map.wrapping_add(8));
        if h.read::<u32>(list.wrapping_add(4)).cast_signed() <= 0 {
            return None;
        }
        let k: u32 = h.read(w.wrapping_add(0x288));
        let ent = k
            .wrapping_mul(3)
            .wrapping_shl(6)
            .wrapping_add(h.read(list.wrapping_add(8)));
        let lo_at = ent.wrapping_add(0x38);
        let hi_at = ent.wrapping_add(0x3c);
        let i = msg.wrapping_sub(0x8001);
        Some(match msg {
            0x8001 => {
                let items = h.rebase(ALPHA_ITEMS);
                cdecl(h, ADD_ITEMS, &[window, items])
            }
            0x8003 => {
                h.write(w.wrapping_add(0x1c), 0_u32);
                1
            }
            0x8006 => {
                cdecl(h, MOVE_END, &[]);
                let j = param.wrapping_sub(0x3e);
                match j {
                    0..=3 => {
                        let shift = 2 * j;
                        let lo: u32 = h.read(lo_at);
                        let v = ((lo >> shift) & 3) + 1;
                        let v = if v > 2 { 0 } else { v };
                        h.write(lo_at, (lo & !(3 << shift)) | (v << shift));
                        v << shift
                    }
                    4 => {
                        let fix = u32::from(h.read::<u8>(hi_at));
                        h.write(w.wrapping_add(0x408), fix);
                        let w = work(h);
                        let title = h.rebase(ALPHA_TITLE);
                        cdecl(
                            h,
                            NUMBER_CREATE,
                            &[
                                w.wrapping_add(0x3dc),
                                w.wrapping_add(0x408),
                                5,
                                title,
                                window,
                            ],
                        )
                    }
                    5..=8 => {
                        let preset = [0x44_u32, 0x48, 0x42, 0x89][(j - 5) as usize];
                        h.write(lo_at, preset);
                        h.write(hi_at, 0_u32);
                        j
                    }
                    _ => j,
                }
            }
            0x8011 => {
                if cdecl(h, NUMBER_IS_OK, &[w.wrapping_add(0x3dc)]) != 0 {
                    h.write(hi_at, 0_u8);
                    let v: u32 = h.read(work(h).wrapping_add(0x408));
                    let hi: u32 = h.read(hi_at);
                    h.write(hi_at, hi | v);
                }
                let hi: u32 = h.read(hi_at);
                let lo: u32 = h.read(lo_at);
                h.write(ebp.wrapping_sub(0x18), hi);
                let (a, c) = (h.rebase(ALPHA_NAMES), h.rebase(ALPHA_NAMES_C));
                for (id, shift, table) in [
                    (0x3e_u32, 0_u32, a),
                    (0x3f, 2, a),
                    (0x40, 4, c),
                    (0x41, 6, a),
                ] {
                    let name: u32 = h.read(table.wrapping_add(4 * ((lo >> shift) & 3)));
                    cdecl(h, SET_TEXT, &[window, id, name]);
                }
                let buf = ebp.wrapping_sub(0x14);
                let fmt = h.rebase(FORMAT_D);
                let f: u32 = h.read(h.rebase(SPRINTF_SLOT));
                h.call(f, Conv::Cdecl, &[buf, fmt, hi & 0xff]);
                cdecl(h, SET_TEXT, &[window, 0x42, buf])
            }
            _ if i <= 0x10 => 4,
            _ => i,
        })
    }
}
