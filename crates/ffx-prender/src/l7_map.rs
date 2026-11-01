//! M5 stage 1, round 7 (list `left7`, from `map.list`): the map tool's
//! windows (layer animation, camera, layer, VRAM, FP adjust, particle) and
//! the `.rsd` model loader (`rcbgModelCreateFromFile`).
//!
//! Callees stay calls to the originals (cdecl). Frames (texts, mouse
//! deltas, tables passed by pointer) come in as the original's `ebp`.
//! Window procedures return what the original leaves in `eax` (`None`: the
//! cookie's, which the shim turns into 0; Gu's dispatch drops it).

/// `g_RcuiWork`, `g_YnMapSystem`.
pub const RCUI_WORK: u32 = 0x0193_c0f0;
pub const YN_MAP_SYSTEM: u32 = 0x0193_bf80;

// ---- callees ----

/// `rcuiAddItems(window, items)`, `rcuiMouseMoveReset()`,
/// `rcuiMouseMoveBegin(window, id)`, `rcuiMouseMoveEnd()`,
/// `rcuiNumberCreate(num, value, kind, title, parent)`,
/// `rcuiNumberIsOK(num)`, `rcbgAnimeSetDataCnt(anim, count)`,
/// `rcuiMouseMoveIsID(window)`, `rcuiMouseMoveGet(&dx, &dy)`,
/// `rcuiMouseMoveGetID()`, `Gu_WindowControlSetText(window, id, text)`,
/// the print helpers.
pub const ADD_ITEMS: u32 = 0x0093_7780;
pub const MOVE_RESET: u32 = 0x0093_c110;
pub const MOVE_BEGIN: u32 = 0x0093_c150;
pub const MOVE_END: u32 = 0x0093_c050;
pub const NUMBER_CREATE: u32 = 0x0093_c2e0;
pub const NUMBER_IS_OK: u32 = 0x0093_c3d0;
pub const ANIME_SET_DATA_CNT: u32 = 0x0093_08d0;
pub const MOVE_IS_ID: u32 = 0x0093_c120;
pub const MOVE_GET: u32 = 0x0093_c060;
pub const MOVE_GET_ID: u32 = 0x0093_c100;
pub const SET_TEXT: u32 = 0x0084_a710;
pub const PRINT_SET_X: u32 = 0x0084_8820;
pub const PRINT_SET_Y: u32 = 0x0084_8860;
pub const GU_PRINTF: u32 = 0x0084_8880;
/// `MSVCR110` `sprintf` slot.
pub const SPRINTF_SLOT: u32 = 0x00b0_c3dc;

/// The animation window's items, its title, "Stop"/"Play", "%d",
/// "%02d", "X:%d", "Y:%d".
pub const ANIME_ITEMS: u32 = 0x00c5_edd0;
pub const ANIME_TITLE: u32 = 0x00b6_8558;
pub const S_STOP: u32 = 0x00b5_d330;
pub const S_PLAY: u32 = 0x00b6_8564;
pub const FORMAT_D: u32 = 0x00b0_f604;
pub const FORMAT_02D: u32 = 0x00b4_1ec0;
pub const FORMAT_X: u32 = 0x00b6_8548;
pub const FORMAT_Y: u32 = 0x00b6_8550;

/// `rcuiAnimeProc`'s frame.
pub const ANIME_FRAME: u32 = 0x20;

pub mod bridge {
    use super::{
        ADD_ITEMS, ANIME_ITEMS, ANIME_SET_DATA_CNT, ANIME_TITLE, FORMAT_02D, FORMAT_D, FORMAT_X,
        FORMAT_Y, GU_PRINTF, MOVE_BEGIN, MOVE_END, MOVE_GET, MOVE_GET_ID, MOVE_IS_ID, MOVE_RESET,
        NUMBER_CREATE, NUMBER_IS_OK, PRINT_SET_X, PRINT_SET_Y, RCUI_WORK, S_PLAY, S_STOP, SET_TEXT,
        SPRINTF_SLOT, YN_MAP_SYSTEM,
    };
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    fn work(h: &impl Host) -> u32 {
        h.read(h.rebase(RCUI_WORK))
    }

    fn at(p: u32, off: u32) -> u32 {
        p.wrapping_add(off)
    }

    /// `Gu_PrintSetX(x)`, `Gu_PrintSetY(y)`, `Gu_Printf(fmt, v)`.
    fn print_at(h: &mut impl Host, x: u32, y: u32, fmt: u32, v: u32) -> u32 {
        cdecl(h, PRINT_SET_X, &[x]);
        cdecl(h, PRINT_SET_Y, &[y]);
        let f = h.rebase(fmt);
        cdecl(h, GU_PRINTF, &[f, v])
    }

    /// `rcuiAnimeProc(window, msg, param)` -> `eax` as the original
    /// (`None`: no work area, the cookie's `eax`; 0 without a system, the
    /// system without a map). The selected layer's (`g_RcuiWork + 0x288`
    /// of `[map + 8] + 0x90`, 0xc0 bytes each) animation: `+0` flags (bit
    /// 0 playing), `+6` the stopped frame (i16), `+8` the frame count,
    /// `+0xc` the frames (4 bytes: pattern, wait, x, y), `+0x10` the
    /// pattern table (`+0x30` its count). The current frame
    /// (`g_RcuiWork + 0x308`) is clamped to the count first.
    ///
    /// 0x8001 adds the items; 0x8003 (closed) clears `+0x10`; 0x8006 ends
    /// the mouse move, then buttons 0x25 (play/stop), 0x26 (the count
    /// through `rcuiNumberCreate`), 0x27/0x28 (the frame, wrapping),
    /// 0x29/0x2a (the pattern, wrapping at the table's count), 0x2b/0x2c
    /// (the wait, 0..99), 0x2d (the wait copied to every frame), 0x2e..0x31
    /// (y and x by 1); 0x8007 starts dragging on 0x32; 0x8011 takes an
    /// entered count, moves the frame by the drag, writes the texts and
    /// prints the frame. `ebp` is the original's frame (the text at `ebp -
    /// 0x14`, the table at `- 0x18`, the deltas at `- 0x1c`, `- 0x20`).
    #[allow(clippy::too_many_lines)]
    pub fn anime_proc<H: Host>(
        h: &mut H,
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
            return Some(0);
        }
        let map: u32 = h.read(sys);
        if map == 0 {
            return Some(sys);
        }
        let k: u32 = h.read(at(w, 0x288));
        let list: u32 = h.read(at(map, 8));
        let anim = k
            .wrapping_mul(3)
            .wrapping_shl(6)
            .wrapping_add(0x90)
            .wrapping_add(h.read(at(list, 8)));
        let count = |h: &mut H| -> i32 { h.read::<u32>(at(anim, 8)).cast_signed() };
        let n = count(h);
        let frame = if n < 1 {
            0
        } else {
            let cur = h.read::<u32>(at(w, 0x308)).cast_signed();
            let v = if cur < 0 {
                0
            } else if cur > n - 1 {
                n - 1
            } else {
                cur
            };
            h.write(at(w, 0x308), v);
            let w = work(h);
            let frames: u32 = h.read(at(anim, 0xc));
            let cur: u32 = h.read(at(w, 0x308));
            frames.wrapping_add(cur.wrapping_mul(4))
        };
        let table: u32 = h.read(at(anim, 0x10));
        h.write(ebp.wrapping_sub(0x18), table);
        let i = msg.wrapping_sub(0x8001);
        Some(match msg {
            0x8001 => {
                let items = h.rebase(ANIME_ITEMS);
                cdecl(h, ADD_ITEMS, &[window, items]);
                cdecl(h, MOVE_RESET, &[])
            }
            0x8003 => {
                h.write(at(work(h), 0x10), 0_u32);
                1
            }
            0x8007 => {
                if param == 0x32 {
                    cdecl(h, MOVE_BEGIN, &[window, 0x32])
                } else {
                    3
                }
            }
            0x8006 => {
                cdecl(h, MOVE_END, &[]);
                let j = param.wrapping_sub(0x25);
                match j {
                    0 => {
                        let f: u32 = h.read(anim);
                        h.write(anim, f ^ 1);
                        if f & 1 == 0 {
                            let stop = i32::from(h.read::<i16>(at(anim, 6)));
                            let w = work(h);
                            h.write(at(w, 0x308), stop);
                            w
                        } else {
                            0
                        }
                    }
                    1 => {
                        let w = work(h);
                        let n: u32 = h.read(at(anim, 8));
                        h.write(at(w, 0x394), n);
                        let w = work(h);
                        let title = h.rebase(ANIME_TITLE);
                        cdecl(
                            h,
                            NUMBER_CREATE,
                            &[at(w, 0x368), at(w, 0x394), 5, title, window],
                        )
                    }
                    2 => {
                        let w = work(h);
                        let v = h.read::<u32>(at(w, 0x308)).wrapping_sub(1);
                        h.write(at(w, 0x308), v);
                        if v.cast_signed() < 0 {
                            let n = h.read::<u32>(at(anim, 8)).wrapping_sub(1);
                            let w = work(h);
                            h.write(at(w, 0x308), n);
                        }
                        work(h)
                    }
                    3 => {
                        let w = work(h);
                        let v = h.read::<u32>(at(w, 0x308)).wrapping_add(1);
                        h.write(at(w, 0x308), v);
                        if v.cast_signed() < count(h) {
                            v
                        } else {
                            let w = work(h);
                            h.write(at(w, 0x308), 0_u32);
                            w
                        }
                    }
                    4..=7 if frame == 0 => j,
                    4 => {
                        let b = h.read::<u8>(frame).wrapping_sub(1);
                        h.write(frame, b);
                        let cnt: u32 = h.read(at(table, 0x30));
                        if i32::from(b) < cnt.cast_signed() {
                            u32::from(b)
                        } else {
                            let v = (cnt as u8).wrapping_sub(1);
                            h.write(frame, v);
                            u32::from(v)
                        }
                    }
                    5 => {
                        let b = h.read::<u8>(frame).wrapping_add(1);
                        h.write(frame, b);
                        let cnt: u32 = h.read(at(table, 0x30));
                        if i32::from(b) >= cnt.cast_signed() {
                            h.write(frame, 0_u8);
                        }
                        u32::from(b)
                    }
                    6 => {
                        let b: u8 = h.read(at(frame, 1));
                        if b != 0 {
                            h.write(at(frame, 1), b - 1);
                            u32::from(b - 1)
                        } else {
                            0
                        }
                    }
                    7 => {
                        let b: u8 = h.read(at(frame, 1));
                        if b < 0x63 {
                            h.write(at(frame, 1), b + 1);
                            u32::from(b + 1)
                        } else {
                            u32::from(b)
                        }
                    }
                    8 => {
                        let n: u32 = h.read(at(anim, 8));
                        if n.cast_signed() <= 0 {
                            return Some(n);
                        }
                        let mut b = 0_u8;
                        let mut k = 0_i32;
                        while k < count(h) {
                            let frames: u32 = h.read(at(anim, 0xc));
                            b = h.read(at(frame, 1));
                            h.write(at(frames, 1).wrapping_add(k.cast_unsigned() * 4), b);
                            k += 1;
                        }
                        (n & !0xff) | u32::from(b)
                    }
                    9..=0xc => {
                        if frame != 0 {
                            let o = if j < 0xb { 3 } else { 2 };
                            let b: u8 = h.read(at(frame, o));
                            let b = if j % 2 == 1 {
                                b.wrapping_sub(1)
                            } else {
                                b.wrapping_add(1)
                            };
                            h.write(at(frame, o), b);
                        }
                        j
                    }
                    _ => j,
                }
            }
            0x8011 => {
                if cdecl(h, NUMBER_IS_OK, &[at(w, 0x368)]) != 0 {
                    let n: u32 = h.read(at(work(h), 0x394));
                    cdecl(h, ANIME_SET_DATA_CNT, &[anim, n]);
                }
                if frame != 0 && cdecl(h, MOVE_IS_ID, &[window]) != 0 {
                    let (dx, dy) = (ebp.wrapping_sub(0x1c), ebp.wrapping_sub(0x20));
                    cdecl(h, MOVE_GET, &[dx, dy]);
                    if cdecl(h, MOVE_GET_ID, &[]) == 0x32 {
                        let x = h.read::<u8>(at(frame, 2)).wrapping_add(h.read(dx));
                        h.write(at(frame, 2), x);
                        let y = h.read::<u8>(at(frame, 3)).wrapping_add(h.read(dy));
                        h.write(at(frame, 3), y);
                    }
                }
                let text = if h.read::<u32>(anim) & 1 == 0 {
                    S_PLAY
                } else {
                    let cur: u16 = h.read(at(work(h), 0x308));
                    h.write(at(anim, 6), cur);
                    S_STOP
                };
                let text = h.rebase(text);
                cdecl(h, SET_TEXT, &[window, 0x25, text]);
                let n: u32 = h.read(at(anim, 8));
                let buf = ebp.wrapping_sub(0x14);
                let fmt = h.rebase(FORMAT_D);
                let f: u32 = h.read(h.rebase(SPRINTF_SLOT));
                h.call(f, Conv::Cdecl, &[buf, fmt, n]);
                cdecl(h, SET_TEXT, &[window, 0x26, buf]);
                let cur: u32 = h.read(at(work(h), 0x308));
                let r = print_at(h, 0x47, 0x1b, FORMAT_02D, cur);
                if frame == 0 {
                    return Some(r);
                }
                let b = u32::from(h.read::<u8>(frame));
                print_at(h, 0x47, 0x28, FORMAT_02D, b);
                let b = u32::from(h.read::<u8>(at(frame, 1)));
                print_at(h, 0x47, 0x35, FORMAT_02D, b);
                let x = i32::from(h.read::<i8>(at(frame, 2))).cast_unsigned();
                print_at(h, 2, 0x5a, FORMAT_X, x);
                let y = i32::from(h.read::<i8>(at(frame, 3))).cast_unsigned();
                print_at(h, 2, 100, FORMAT_Y, y)
            }
            _ if i <= 0x10 => 5,
            _ => i,
        })
    }

    /// The camera window's items; its ten number entries' titles ("Camera
    /// Pos.X" and on, 16 bytes apart); "%.2f"; f64 0.1 (the drag step) and
    /// 180/pi (as f64 57.29578018...); `rcbgSceneLoadTextCameraCalcRef(scene)`,
    /// `MsCameraGetYaw()` (`ST0`).
    pub const CAMERA_ITEMS: u32 = 0x00c5_f350;
    pub const CAMERA_TITLES: u32 = 0x00b6_8578;
    pub const FORMAT_2F: u32 = 0x00b5_5f78;
    pub const TENTH: u32 = 0x00b9_21f0;
    pub const DEG_PER_RAD: u32 = 0x00b6_8838;
    pub const CAMERA_CALC_REF: u32 = 0x0093_b110;
    pub const CAMERA_GET_YAW: u32 = 0x007b_cf20;
    /// `rcuiCameraProc`'s frame.
    pub const CAMERA_FRAME: u32 = 0x20;

    /// An `f64` constant.
    fn k64(h: &impl Host, sym: u32) -> f64 {
        f64::from_bits(h.read(h.rebase(sym)))
    }

    /// `sprintf(buf, "%.2f", (double)[p])` (the `f32` widened as `fld`
    /// does), then `Gu_WindowControlSetText(window, id, buf)`.
    fn text_2f(h: &mut impl Host, ebp: u32, window: u32, id: u32, p: u32) -> u32 {
        let v = x87::load(h.read(p)).to_bits();
        let buf = ebp.wrapping_sub(0x14);
        let fmt = h.rebase(FORMAT_2F);
        let f: u32 = h.read(h.rebase(SPRINTF_SLOT));
        #[allow(clippy::cast_possible_truncation)] // the two halves
        h.call(f, Conv::Cdecl, &[buf, fmt, v as u32, (v >> 32) as u32]);
        cdecl(h, SET_TEXT, &[window, id, buf])
    }

    /// `rcuiCameraProc(window, msg, param)` -> `eax` as the original
    /// (`None`: no work area, the cookie's `eax`; 0 without a system, the
    /// system without a map). The map's scene (`[map + 8]`): its camera
    /// position (`+0x68..`), reference point (`+0x78..`), angle (`+0xa8`)
    /// and rotation (`+0x98..`).
    ///
    /// 0x8001 adds the items; 0x8003 (closed) clears `g_RcuiWork + 0x20`;
    /// 0x8006 buttons 0x47..0x51 open a float entry on their value
    /// (`rcuiNumberCreate(work + 0x40c + 0x2c k, value, 0xb, title,
    /// window)`, 0x4e none), 0x52 recomputes the reference point; 0x800a
    /// ends the mouse move, 0x800b starts it on 0x47..0x4d; 0x8011 adds a
    /// drag on those (`f32(dx) * 0.1`) and writes the texts and the yaw in
    /// degrees. `ebp` is the original's frame (the text at `ebp - 0x14`,
    /// `f32(dx)` at `- 0x18`, the deltas at `- 0x1c`, `- 0x20`).
    #[allow(clippy::too_many_lines)]
    pub fn camera_proc(
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
            return Some(0);
        }
        let map: u32 = h.read(sys);
        if map == 0 {
            return Some(sys);
        }
        let scene: u32 = h.read(at(map, 8));
        let i = msg.wrapping_sub(0x8001);
        Some(match msg {
            0x8001 => {
                let items = h.rebase(CAMERA_ITEMS);
                cdecl(h, ADD_ITEMS, &[window, items]);
                cdecl(h, MOVE_RESET, &[])
            }
            0x8003 => {
                h.write(at(w, 0x20), 0_u32);
                1
            }
            0x8006 => {
                let j = param.wrapping_sub(0x47);
                let value = match j {
                    0..=2 => 0x68 + 4 * j,
                    3..=5 => 0x78 + 4 * (j - 3),
                    6 => 0xa8,
                    8..=10 => 0x98 + 4 * (j - 8),
                    0xb => return Some(cdecl(h, CAMERA_CALC_REF, &[scene])),
                    _ => return Some(j),
                };
                let k = if j < 7 { j } else { j - 1 };
                let title = h.rebase(CAMERA_TITLES).wrapping_add(0x10 * k);
                cdecl(
                    h,
                    NUMBER_CREATE,
                    &[
                        at(w, 0x40c + 0x2c * k),
                        at(scene, value),
                        0xb,
                        title,
                        window,
                    ],
                )
            }
            0x800a => cdecl(h, MOVE_END, &[]),
            0x800b => {
                if param.wrapping_sub(0x47) < 7 {
                    cdecl(h, MOVE_BEGIN, &[window, param])
                } else {
                    param.wrapping_sub(0x47)
                }
            }
            0x8011 => {
                if cdecl(h, MOVE_IS_ID, &[window]) != 0 {
                    let (dx, dy) = (ebp.wrapping_sub(0x1c), ebp.wrapping_sub(0x20));
                    cdecl(h, MOVE_GET, &[dx, dy]);
                    let id = cdecl(h, MOVE_GET_ID, &[]);
                    let value = match id {
                        0x47..=0x49 => Some(0x68 + 4 * (id - 0x47)),
                        0x4a..=0x4c => Some(0x78 + 4 * (id - 0x4a)),
                        0x4d => Some(0xa8),
                        _ => None,
                    };
                    if let Some(off) = value {
                        // fild, then fstp dword: f32(dx), kept at ebp - 0x18
                        let d = x87::store(f64::from(h.read::<u32>(dx).cast_signed()));
                        h.write(ebp.wrapping_sub(0x18), d);
                        let step = x87::mul(x87::load(d), k64(h, TENTH));
                        let p = at(scene, off);
                        let v = x87::add(step, x87::load(h.read(p)));
                        h.write(p, x87::store(v));
                    }
                }
                for (id, off) in [
                    (0x47, 0x68),
                    (0x48, 0x6c),
                    (0x49, 0x70),
                    (0x4a, 0x78),
                    (0x4b, 0x7c),
                    (0x4c, 0x80),
                    (0x4d, 0xa8),
                ] {
                    text_2f(h, ebp, window, id, at(scene, off));
                }
                cdecl(h, PRINT_SET_X, &[0x36]);
                cdecl(h, PRINT_SET_Y, &[0x5c]);
                let yaw = h.call_sym_st0_f64(CAMERA_GET_YAW, Conv::Cdecl, &[]);
                let deg = x87::ftol(h, x87::mul(yaw, k64(h, DEG_PER_RAD)));
                let fmt = h.rebase(FORMAT_D);
                cdecl(h, GU_PRINTF, &[fmt, deg]);
                let mut r = 0;
                for (id, off) in [(0x4f, 0x98), (0x50, 0x9c), (0x51, 0xa0)] {
                    r = text_2f(h, ebp, window, id, at(scene, off));
                }
                r
            }
            _ if i <= 0x10 => 6,
            _ => i,
        })
    }

    /// The layer window's items; `rcuiClearSelect()`, `rcuiSetSelect()`,
    /// `rcuiNumWindowCreate_0093c670(win, value, title, parent)`,
    /// `rcbgSceneChkSortLayer(scene)`, `rcuiWindowCreate(holder, x, y, w,
    /// h, flags, proc, title, parent)`; its strings.
    pub const LAYER_ITEMS: u32 = 0x00c5_eac0;
    pub const CLEAR_SELECT: u32 = 0x0093_4af0;
    pub const SET_SELECT: u32 = 0x0093_4b50;
    pub const NUM_WINDOW_CREATE: u32 = 0x0093_c670;
    pub const CHK_SORT_LAYER: u32 = 0x0092_b460;
    pub const WINDOW_CREATE: u32 = 0x0093_7ef0;
    pub const S_LAYER_Z: u32 = 0x00b6_850c;
    pub const S_X_SPEED: u32 = 0x00b6_8514;
    pub const S_Y_SPEED: u32 = 0x00b6_851c;
    pub const S_ANIME: u32 = 0x00b6_7ef8;
    pub const S_PALETTE: u32 = 0x00b6_8524;
    pub const S_DEPTH_Z: u32 = 0x00b6_852c;
    pub const S_ALPHA: u32 = 0x00b6_7f10;
    pub const FORMAT_C: u32 = 0x00b4_1f28;
    pub const FORMAT_UC: u32 = 0x00b6_8534;
    pub const S_NORM: u32 = 0x00b6_7f88;
    pub const S_WHITE: u32 = 0x00b6_8538;
    pub const S_ONLY: u32 = 0x00b6_8540;
    pub const S_OFF: u32 = 0x00b4_0644;
    pub const S_ON: u32 = 0x00b4_0648;
    pub const S_DASH: u32 = 0x00b8_f870;
    /// The windows the layer window opens: `rcuiAnimeProc`,
    /// `rcuiPaletteProc`, `rcuiLayerRectProc_00932a60`, `rcuiAlphaProc`.
    pub const ANIME_PROC: u32 = 0x0093_13f0;
    pub const PALETTE_PROC: u32 = 0x0093_5820;
    pub const LAYER_RECT_PROC: u32 = 0x0093_2a60;
    pub const ALPHA_PROC: u32 = 0x0093_1050;
    /// `rcuiLayerProc`'s frame.
    pub const LAYER_FRAME: u32 = 0x1c;

    /// `sprintf(buf, "%d", v)` at `ebp - 0x14`, then
    /// `Gu_WindowControlSetText(window, id, buf)`.
    fn text_d(h: &mut impl Host, ebp: u32, window: u32, id: u32, v: u32) -> u32 {
        let buf = ebp.wrapping_sub(0x14);
        let fmt = h.rebase(FORMAT_D);
        let f: u32 = h.read(h.rebase(SPRINTF_SLOT));
        h.call(f, Conv::Cdecl, &[buf, fmt, v]);
        cdecl(h, SET_TEXT, &[window, id, buf])
    }

    /// `[p] ^= bit`.
    fn flip(h: &mut impl Host, p: u32, bit: u32) {
        let f: u32 = h.read(p);
        h.write(p, f ^ bit);
    }

    /// `Gu_WindowControlSetText(window, id, text)` of a string.
    fn text(h: &mut impl Host, window: u32, id: u32, s: u32) -> u32 {
        let s = h.rebase(s);
        cdecl(h, SET_TEXT, &[window, id, s])
    }

    /// `rcuiLayerProc(window, msg, param)` -> `eax` as the original
    /// (`None`: no work area, the cookie's `eax`; 0 without a system, the
    /// system without a map). The map's scene (`[map + 8]`: `+4` the
    /// layer count, `+8` the layers, 0xc0 bytes each, `+0x104` a flag)
    /// and its selected layer (`g_RcuiWork + 0x288`; none without
    /// layers): `+0` flags (bits 0, 2 the depth window open, 4, 5, 6),
    /// `+0xc`/`+0x10` x/y, `+0x14` z (f32), `+0x2c`/`+0x30` the speeds.
    ///
    /// 0x8001 adds the items; 0x8003 (closed) clears `g_RcuiWork + 0xc`;
    /// 0x8006 ends the mouse move, then with a layer: 0x11/0x12 the layer
    /// (wrapping), 0x13 the selection mode 0..2 (between
    /// `rcuiClearSelect` and `rcuiSetSelect`), 0x14..0x17 y and x by 1,
    /// 0x19..0x1b the entries (z, the speeds), 0x1c..0x20 the flags (0x1d
    /// resorts the layers), 0x21..0x24 open the animation, palette, depth
    /// and alpha windows; 0x8007 starts dragging on 0x18; 0x8011 with a
    /// layer adds the drag and writes the texts. `ebp` is the original's
    /// frame (the text at `ebp - 0x14`, the deltas at `- 0x1c`, `- 0x18`).
    #[allow(clippy::too_many_lines)]
    pub fn layer_proc<H: Host>(
        h: &mut H,
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
            return Some(0);
        }
        let map: u32 = h.read(sys);
        if map == 0 {
            return Some(sys);
        }
        let scene: u32 = h.read(at(map, 8));
        let layer = if h.read::<u32>(at(scene, 4)).cast_signed() <= 0 {
            0
        } else {
            let k: u32 = h.read(at(w, 0x288));
            k.wrapping_mul(3)
                .wrapping_shl(6)
                .wrapping_add(h.read(at(scene, 8)))
        };
        let i = msg.wrapping_sub(0x8001);
        Some(match msg {
            0x8001 => {
                let items = h.rebase(LAYER_ITEMS);
                cdecl(h, ADD_ITEMS, &[window, items]);
                cdecl(h, MOVE_RESET, &[])
            }
            0x8003 => {
                h.write(at(w, 0xc), 0_u32);
                1
            }
            0x8007 => {
                if param == 0x18 {
                    cdecl(h, MOVE_BEGIN, &[window, 0x18])
                } else {
                    3
                }
            }
            0x8006 => {
                let r = cdecl(h, MOVE_END, &[]);
                if layer == 0 {
                    return Some(r);
                }
                if param == 0 {
                    return Some(0);
                }
                let j = param.wrapping_sub(0x11);
                let open = |h: &mut H, slot: u32, size: (u32, u32), proc: u32, title: u32| {
                    let w = work(h);
                    let (proc, title) = (h.rebase(proc), h.rebase(title));
                    cdecl(
                        h,
                        WINDOW_CREATE,
                        &[at(w, slot), 8, 8, size.0, size.1, 6, proc, title, window],
                    )
                };
                match j {
                    0..=2 => {
                        cdecl(h, CLEAR_SELECT, &[]);
                        let w = work(h);
                        if j == 2 {
                            let v = h.read::<u32>(at(w, 0x28c)).wrapping_add(1);
                            h.write(at(w, 0x28c), v);
                            if v.cast_signed() > 2 {
                                h.write(at(work(h), 0x28c), 0_u32);
                            }
                        } else {
                            let v = h.read::<u32>(at(w, 0x288));
                            let v = if j == 0 {
                                v.wrapping_sub(1)
                            } else {
                                v.wrapping_add(1)
                            };
                            h.write(at(w, 0x288), v);
                            let n = h.read::<u32>(at(scene, 4));
                            if j == 0 && v.cast_signed() < 0 {
                                h.write(at(work(h), 0x288), n.wrapping_sub(1));
                            } else if j == 1 && v.cast_signed() >= n.cast_signed() {
                                h.write(at(work(h), 0x288), 0_u32);
                            }
                        }
                        cdecl(h, SET_SELECT, &[])
                    }
                    3..=6 => {
                        let p = at(layer, if j < 5 { 0x10 } else { 0xc });
                        let v: u32 = h.read(p);
                        let v = if j % 2 == 1 {
                            v.wrapping_sub(1)
                        } else {
                            v.wrapping_add(1)
                        };
                        h.write(p, v);
                        j
                    }
                    8 => {
                        let w = work(h);
                        let title = h.rebase(S_LAYER_Z);
                        let r = cdecl(
                            h,
                            NUM_WINDOW_CREATE,
                            &[at(w, 0x2e8), at(layer, 0x14), title, window],
                        );
                        let f: u32 = h.read(layer);
                        h.write(layer, f & !4);
                        r
                    }
                    9 | 10 => {
                        let w = work(h);
                        let (slot, value, title) = if j == 9 {
                            (0x290, 0x2c, S_X_SPEED)
                        } else {
                            (0x2bc, 0x30, S_Y_SPEED)
                        };
                        let title = h.rebase(title);
                        cdecl(
                            h,
                            NUMBER_CREATE,
                            &[at(w, slot), at(layer, value), 5, title, window],
                        )
                    }
                    11 => {
                        flip(h, layer, 1);
                        j
                    }
                    12 => {
                        flip(h, layer, 0x10);
                        cdecl(h, CHK_SORT_LAYER, &[scene])
                    }
                    13 | 14 => {
                        flip(h, layer, if j == 13 { 0x20 } else { 0x40 });
                        j
                    }
                    15 => {
                        flip(h, at(scene, 0x104), 1);
                        j
                    }
                    16 => open(h, 0x10, (0x78, 0x81), ANIME_PROC, S_ANIME),
                    17 => open(h, 0x14, (0x78, 0x28), PALETTE_PROC, S_PALETTE),
                    18 => {
                        let r = open(h, 0x18, (0xc6, 0x4e), LAYER_RECT_PROC, S_DEPTH_Z);
                        let f: u32 = h.read(layer);
                        h.write(layer, f | 4);
                        r
                    }
                    19 => open(h, 0x1c, (0x78, 0x81), ALPHA_PROC, S_ALPHA),
                    _ => j,
                }
            }
            0x8011 => {
                if layer == 0 {
                    return Some(4);
                }
                if cdecl(h, MOVE_IS_ID, &[window]) != 0 {
                    let (dx, dy) = (ebp.wrapping_sub(0x1c), ebp.wrapping_sub(0x18));
                    cdecl(h, MOVE_GET, &[dx, dy]);
                    for (d, off) in [(dx, 0xc), (dy, 0x10)] {
                        let v = h.read::<u32>(at(layer, off)).wrapping_add(h.read(d));
                        h.write(at(layer, off), v);
                    }
                }
                cdecl(h, PRINT_SET_X, &[0x47]);
                cdecl(h, PRINT_SET_Y, &[1]);
                let sel: u32 = h.read(at(work(h), 0x288));
                let (fmt, c) = if sel.cast_signed() < 0x1a {
                    (FORMAT_C, sel.wrapping_add(0x61))
                } else {
                    (FORMAT_UC, sel.wrapping_add(0x47))
                };
                let fmt = h.rebase(fmt);
                cdecl(h, GU_PRINTF, &[fmt, c]);
                let mode: u32 = h.read(at(work(h), 0x28c));
                let name = match mode {
                    0 => Some(S_NORM),
                    1 => Some(S_WHITE),
                    2 => Some(S_ONLY),
                    _ => None,
                };
                if let Some(name) = name {
                    text(h, window, 0x13, name);
                }
                let x: u32 = h.read(at(layer, 0xc));
                print_at(h, 2, 0x26, FORMAT_X, x);
                let y: u32 = h.read(at(layer, 0x10));
                print_at(h, 2, 0x30, FORMAT_Y, y);
                let v: u32 = h.read(at(layer, 0x2c));
                text_d(h, ebp, window, 0x1a, v);
                let v: u32 = h.read(at(layer, 0x30));
                text_d(h, ebp, window, 0x1b, v);
                for (id, bit, on_set) in [
                    (0x1c, 1, false),
                    (0x1d, 0x10, true),
                    (0x1e, 0x20, false),
                    (0x1f, 0x40, true),
                ] {
                    let set = h.read::<u32>(layer) & bit != 0;
                    text(h, window, id, if set == on_set { S_ON } else { S_OFF });
                }
                let on = h.read::<u32>(at(scene, 0x104)) != 0;
                text(h, window, 0x20, if on { S_ON } else { S_OFF });
                if h.read::<u32>(layer) & 4 != 0 {
                    return Some(text(h, window, 0x19, S_DASH));
                }
                let z = x87::ftol(h, x87::load(h.read(at(layer, 0x14))));
                text_d(h, ebp, window, 0x19, z)
            }
            _ if i <= 0x10 => 5,
            _ => i,
        })
    }

    /// The VRAM window's items; the entries' titles ("TBW", "TW", "TH",
    /// "W", "H"); the 13 pixel-format names ("CT32" .. "Z16S"); "%04x",
    /// "%s", the scale names ("x1", "x1.5", "x2"); `rcuiVramWrite()`.
    pub const VRAM_ITEMS: u32 = 0x00c6_0618;
    pub const S_TBW: u32 = 0x00b6_8054;
    pub const S_TW: u32 = 0x00b6_805c;
    pub const S_TH: u32 = 0x00b6_8060;
    pub const S_W: u32 = 0x00b8_f7fc;
    pub const S_H: u32 = 0x00b8_f7c0;
    pub const PSM_NAMES: [u32; 13] = [
        0x00b6_8760,
        0x00b6_8768,
        0x00b6_8770,
        0x00b6_8778,
        0x00b6_8780,
        0x00b6_8784,
        0x00b6_8788,
        0x00b6_878c,
        0x00b6_8794,
        0x00b6_879c,
        0x00b6_87a0,
        0x00b6_87a4,
        0x00b6_87a8,
    ];
    pub const FORMAT_04X: u32 = 0x00b6_87b0;
    pub const FORMAT_S: u32 = 0x00b0_e160;
    pub const SCALE_NAMES: [u32; 3] = [0x00b6_87b8, 0x00b6_87bc, 0x00b6_87c4];
    pub const VRAM_WRITE: u32 = 0x0093_7190;
    /// `rcuiVramProc_00936a00`'s frame.
    pub const VRAM_FRAME: u32 = 0x48;

    /// `[w + off] = ([w + off] + step) & 0x3fff` (two stores, the second
    /// through a fresh `g_RcuiWork`); `eax` the work area.
    fn step_14(h: &mut impl Host, w: u32, off: u32, step: u32) -> u32 {
        let v = h.read::<u32>(at(w, off)).wrapping_add(step);
        h.write(at(w, off), v);
        let w = work(h);
        let v = h.read::<u32>(at(w, off)) & 0x3fff;
        h.write(at(w, off), v);
        w
    }

    /// `rcuiVramProc_00936a00(window, msg, param)` -> `eax` as the
    /// original (`None`: no work area or system, the cookie's `eax`): the
    /// VRAM transfer window over `g_RcuiWork`: `+0xae4` the address
    /// (14 bits, hex digits stepped), `+0xae8` TBW, `+0xaec` the pixel
    /// format (0..12, wrapping), `+0xaf0`/`+0xaf4` TW/TH, `+0xaf8` a second
    /// address, `+0xafc`/`+0xb00` W/H, `+0xb04` the scale (0..2), `+0xb08`
    /// a flag.
    ///
    /// 0x8001 adds the items; 0x8003 (closed) clears `+0x4c`; 0x8006
    /// (left click) 0xb8..0xbb and 0xc0..0xc3 add 0x1000/0x100/0x10/1,
    /// 0xbc/0xbe/0xbf/0xc4/0xc5 open the number entries, 0xbd/0xc6 step the
    /// format/scale up, 0xc7 writes the VRAM (`rcuiVramWrite`, the flag
    /// cleared); 0x800a (right click) the same steps down, 0xc4/0xc5 copy
    /// H to W and W to H, 0xc7 flips the flag; 0x8011 writes the texts.
    /// `ebp` is the original's frame (the name table at `ebp - 0x48`, the
    /// text at `- 0x14`).
    #[allow(clippy::too_many_lines)]
    pub fn vram_proc<H: Host>(
        h: &mut H,
        ebp: u32,
        window: u32,
        msg: u32,
        param: u32,
    ) -> Option<u32> {
        let w = work(h);
        if w == 0 || h.read::<u32>(h.rebase(YN_MAP_SYSTEM)) == 0 {
            return None;
        }
        let i = msg.wrapping_sub(0x8001);
        let j = param.wrapping_sub(0xb8);
        let entry = |h: &mut H, slot: u32, value: u32, title: u32| {
            let title = h.rebase(title);
            cdecl(
                h,
                NUMBER_CREATE,
                &[at(w, slot), at(w, value), 5, title, window],
            )
        };
        Some(match msg {
            0x8001 => {
                let items = h.rebase(VRAM_ITEMS);
                cdecl(h, ADD_ITEMS, &[window, items])
            }
            0x8003 => {
                h.write(at(w, 0x4c), 0_u32);
                1
            }
            0x8006 => match j {
                0..=3 => step_14(h, w, 0xae4, 0x1000 >> (4 * j)),
                8..=11 => step_14(h, w, 0xaf8, 0x1000 >> (4 * (j - 8))),
                4 => entry(h, 0xb0c, 0xae8, S_TBW),
                6 => entry(h, 0xb38, 0xaf0, S_TW),
                7 => entry(h, 0xb64, 0xaf4, S_TH),
                0xc => entry(h, 0xb90, 0xafc, S_W),
                0xd => entry(h, 0xbbc, 0xb00, S_H),
                5 | 0xe => {
                    let (off, top) = if j == 5 { (0xaec, 0xc) } else { (0xb04, 2) };
                    let v = h.read::<u32>(at(w, off)).wrapping_add(1);
                    h.write(at(w, off), v);
                    if v.cast_signed() > top {
                        let w = work(h);
                        h.write(at(w, off), 0_u32);
                        w
                    } else {
                        j
                    }
                }
                0xf => {
                    cdecl(h, VRAM_WRITE, &[]);
                    let w = work(h);
                    h.write(at(w, 0xb08), 0_u32);
                    w
                }
                _ => j,
            },
            0x800a => match j {
                0..=3 => step_14(h, w, 0xae4, (0x1000_u32 >> (4 * j)).wrapping_neg()),
                8..=11 => step_14(h, w, 0xaf8, (0x1000_u32 >> (4 * (j - 8))).wrapping_neg()),
                5 | 0xe => {
                    let (off, top) = if j == 5 { (0xaec, 0xc_u32) } else { (0xb04, 2) };
                    let v = h.read::<u32>(at(w, off)).wrapping_sub(1);
                    h.write(at(w, off), v);
                    if v.cast_signed() < 0 {
                        let w = work(h);
                        h.write(at(w, off), top);
                        w
                    } else {
                        j
                    }
                }
                0xc | 0xd => {
                    let (from, to) = if j == 0xc {
                        (0xb00, 0xafc)
                    } else {
                        (0xafc, 0xb00)
                    };
                    let v: u32 = h.read(at(w, from));
                    h.write(at(w, to), v);
                    v
                }
                0xf => {
                    let f: u32 = h.read(at(w, 0xb08));
                    h.write(at(w, 0xb08), f ^ 1);
                    j
                }
                _ => j,
            },
            0x8011 => {
                let names = ebp.wrapping_sub(0x48);
                for (k, &s) in PSM_NAMES.iter().enumerate() {
                    let s = h.rebase(s);
                    h.write(names.wrapping_add(4 * k as u32), s);
                }
                let v: u32 = h.read(at(work(h), 0xae4));
                print_at(h, 0x36, 3, FORMAT_04X, v);
                let v: u32 = h.read(at(work(h), 0xae8));
                text_d(h, ebp, window, 0xbc, v);
                let k: u32 = h.read(at(work(h), 0xaec));
                let name: u32 = h.read(names.wrapping_add(k.wrapping_mul(4)));
                let buf = ebp.wrapping_sub(0x14);
                let fmt = h.rebase(FORMAT_S);
                let f: u32 = h.read(h.rebase(SPRINTF_SLOT));
                h.call(f, Conv::Cdecl, &[buf, fmt, name]);
                cdecl(h, SET_TEXT, &[window, 0xbd, buf]);
                for (id, off) in [(0xbe, 0xaf0), (0xbf, 0xaf4)] {
                    let v: u32 = h.read(at(work(h), off));
                    text_d(h, ebp, window, id, v);
                }
                let v: u32 = h.read(at(work(h), 0xaf8));
                print_at(h, 0x36, 0x51, FORMAT_04X, v);
                for (id, off) in [(0xc4, 0xafc), (0xc5, 0xb00)] {
                    let v: u32 = h.read(at(work(h), off));
                    text_d(h, ebp, window, id, v);
                }
                let scale: u32 = h.read(at(work(h), 0xb04));
                match SCALE_NAMES.get(scale as usize) {
                    Some(&s) => text(h, window, 0xc6, s),
                    None => scale.wrapping_sub(2),
                }
            }
            _ if i <= 0x10 => 5,
            _ => i,
        })
    }

    /// The FP adjust window's items; its entries' titles ("FP Pos.X" ..
    /// "Part. Scl.Z", 12 bytes apart); "%.1f"; f64 10 and 0.05, f32 0.1,
    /// the scale's f32 11.3778 (`0x41360b61`); the mode names ("Norm.",
    /// "Chr.", "Map", "Mat."); `pppFpGetMngPtr(fp, 0, &out)`,
    /// `pppFpResetStick(fp)`, `MsGetChrHandle(chr)`,
    /// `pppFpSetStickChr(fp, handle, part)`, `pppFpSetStickMap(fp, chr)`,
    /// `rcuiScaleVector(dst, v, s, a, b, c)`, `FFXVu0UnitMatrix(m)`,
    /// `rcMat4fSetScaleVec3f(m, v)`, `op_rt_m_yxz(dst, m, rot, pos)`.
    pub const FP_ITEMS: u32 = 0x00c5_fab0;
    pub const FP_TITLES: [u32; 9] = [
        0x00b6_8620,
        0x00b6_862c,
        0x00b6_8638,
        0x00b6_8644,
        0x00b6_8650,
        0x00b6_865c,
        0x00b6_8668,
        0x00b6_8674,
        0x00b6_8680,
    ];
    pub const FORMAT_1F: u32 = 0x00b5_5f68;
    pub const TEN: u32 = 0x00b9_22b0;
    pub const FIVE_HUNDREDTHS: u32 = 0x00b4_12e8;
    pub const F32_TENTH: u32 = 0x00b1_0f38;
    pub const FP_SCALE: u32 = 0x00b6_78ec;
    pub const FP_MODE_NAMES: [u32; 4] = [0x00b6_7f88, 0x00b6_7e8c, 0x00b6_868c, 0x00b6_8690];
    pub const FP_GET_MNG_PTR: u32 = 0x0072_92f0;
    pub const FP_RESET_STICK: u32 = 0x0072_a120;
    pub const GET_CHR_HANDLE: u32 = 0x0079_5080;
    pub const FP_SET_STICK_CHR: u32 = 0x0072_a530;
    pub const FP_SET_STICK_MAP: u32 = 0x0072_a590;
    pub const SCALE_VECTOR: u32 = 0x0093_0f30;
    pub const UNIT_MATRIX: u32 = 0x006e_d9a0;
    pub const SET_SCALE_VEC3: u32 = 0x0092_5820;
    pub const RT_M_YXZ: u32 = 0x007e_9a30;
    /// `rcuiFpAdjProc_00932f20`'s frame.
    pub const FP_ADJ_FRAME: u32 = 0x68;

    /// `[p] = f32([p] + d)` (`fld dword`, `fadd qword`, `fstp dword`;
    /// `fsub` with `sub`).
    fn add_f(h: &mut impl Host, p: u32, d: f64, sub: bool) {
        let x = x87::load(h.read(p));
        let v = if sub { x87::sub(x, d) } else { x87::add(x, d) };
        h.write(p, x87::store(v));
    }

    /// `sprintf(buf, fmt, (double)[p])`, then
    /// `Gu_WindowControlSetText(window, id, buf)`.
    fn text_f(h: &mut impl Host, ebp: u32, window: u32, id: u32, fmt: u32, p: u32) -> u32 {
        let v = x87::load(h.read(p)).to_bits();
        let buf = ebp.wrapping_sub(0x14);
        let fmt = h.rebase(fmt);
        let f: u32 = h.read(h.rebase(SPRINTF_SLOT));
        #[allow(clippy::cast_possible_truncation)] // the two halves
        h.call(f, Conv::Cdecl, &[buf, fmt, v as u32, (v >> 32) as u32]);
        cdecl(h, SET_TEXT, &[window, id, buf])
    }

    /// `rcuiFpAdjProc_00932f20(window, msg, param)` -> `eax` as the
    /// original (`None`: no work area, the cookie's `eax`): the particle
    /// (FP) placement window over `g_RcuiWork`: `+0x62c` the FP number,
    /// `+0x630` the mode (0..3: none, a character, the map, a matrix),
    /// `+0x634`/`+0x638` the character and its part, `+0x63c..` the
    /// position, `+0x64c..` the angles, `+0x65c..` the scale; `+0x34` its
    /// window holder.
    ///
    /// 0x8001 adds the items and sets the scale to 1; 0x8006 (left click)
    /// 0x78..0x7b step the numbers up (the mode wrapping at 4), the even
    /// buttons 0x7c..0x8c open the float entries, the odd ones add 10 (the
    /// scale 0.05), 0x8e copies the scale's x to y and z; 0x800a (right
    /// click) steps down, zeroes (the scale: 0.1) or subtracts, and every
    /// path there clears `+0x34`, as 0x8003 (closed) does; 0x8011 places
    /// the FP (its manager's flags, the stick, the matrix through
    /// `op_rt_m_yxz`) and writes the texts. `ebp` is the original's frame
    /// (the manager pointer at `ebp - 0x68`, the matrix at `- 0x64`, the
    /// scaled angles at `- 0x24`, the text at `- 0x14`).
    ///
    /// Junk read: `rcuiScaleVector` takes its scale as a 16-byte vector by
    /// value of which the caller pushes only x; y, z, w are the frame's
    /// lowest words: the manager pointer and `[ebp-0x64]`, `[ebp-0x60]`
    /// (the matrix, not yet written), passed as they are (a zeroed frame in
    /// the shim: 0, brief 2026-10-24).
    #[allow(clippy::too_many_lines)]
    pub fn fp_adj_proc<H: Host>(
        h: &mut H,
        ebp: u32,
        window: u32,
        msg: u32,
        param: u32,
    ) -> Option<u32> {
        let w = work(h);
        if w == 0 {
            return None;
        }
        let i = msg.wrapping_sub(0x8001);
        let j = param.wrapping_sub(0x78);
        let ten = k64(h, TEN);
        let twentieth = k64(h, FIVE_HUNDREDTHS);
        // the float buttons: 0x7c..0x87 the position and the angles, 0x88..
        // 0x8d the scale
        let float_at = |j: u32| -> u32 {
            if j < 0xa {
                0x63c + 2 * (j - 4)
            } else if j < 0x10 {
                0x64c + 2 * (j - 0xa)
            } else {
                0x65c + 2 * (j - 0x10)
            }
        };
        Some(match msg {
            0x8001 => {
                let items = h.rebase(FP_ITEMS);
                cdecl(h, ADD_ITEMS, &[window, items]);
                let one = 1.0_f32.to_bits();
                for off in [0x664, 0x660, 0x65c] {
                    h.write(at(work(h), off), one);
                }
                work(h)
            }
            0x8006 => match j {
                0 | 2 | 3 => {
                    let off = 0x62c + 4 * j;
                    let v = h.read::<u32>(at(w, off)).wrapping_add(1);
                    h.write(at(w, off), v);
                    j
                }
                1 => {
                    let v = h.read::<u32>(at(w, 0x630)).wrapping_add(1);
                    h.write(at(w, 0x630), v);
                    if v.cast_signed() < 4 {
                        j
                    } else {
                        let w = work(h);
                        h.write(at(w, 0x630), 0_u32);
                        w
                    }
                }
                4..=0x15 if j.is_multiple_of(2) => {
                    let k = (j - 4) / 2;
                    let value = float_at(j);
                    let title = h.rebase(FP_TITLES[k as usize]);
                    cdecl(
                        h,
                        NUMBER_CREATE,
                        &[at(w, 0x698 + 0x2c * k), at(w, value), 0xb, title, window],
                    )
                }
                4..=0x15 => {
                    let value = float_at(j - 1);
                    add_f(
                        h,
                        at(w, value),
                        if j < 0x10 { ten } else { twentieth },
                        false,
                    );
                    j
                }
                0x16 => {
                    // fstp dword [ebp-0x68], fld: the f32 kept in the frame
                    let x = x87::store(x87::load(h.read(at(w, 0x65c))));
                    h.write(ebp.wrapping_sub(0x68), x);
                    h.write(at(w, 0x664), x);
                    let w = work(h);
                    h.write(at(w, 0x660), x);
                    w
                }
                _ => j,
            },
            0x800a => {
                let r = match j {
                    0 | 2 | 3 => {
                        let off = 0x62c + 4 * j;
                        let v = h.read::<u32>(at(w, off)).wrapping_sub(1);
                        h.write(at(w, off), v);
                        j
                    }
                    1 => {
                        let v = h.read::<u32>(at(w, 0x630)).wrapping_sub(1);
                        h.write(at(w, 0x630), v);
                        if v.cast_signed() < 0 {
                            let w = work(h);
                            h.write(at(w, 0x630), 3_u32);
                            w
                        } else {
                            j
                        }
                    }
                    4..=0x15 if j.is_multiple_of(2) => {
                        let v = if j < 0x10 {
                            0
                        } else {
                            h.read(h.rebase(F32_TENTH))
                        };
                        h.write(at(w, float_at(j)), v);
                        j
                    }
                    4..=0x15 => {
                        let d = if j < 0x10 { ten } else { twentieth };
                        add_f(h, at(w, float_at(j - 1)), d, true);
                        j
                    }
                    _ => j,
                };
                // the path out of every case: `+0x34` cleared through a
                // fresh g_RcuiWork (the default through the entry's)
                let holder = if j < 0x16 { work(h) } else { w };
                h.write(at(holder, 0x34), 0_u32);
                r
            }
            0x8003 => {
                h.write(at(w, 0x34), 0_u32);
                1
            }
            0x8011 => {
                let mng = ebp.wrapping_sub(0x68);
                let fp: u32 = h.read(at(w, 0x62c));
                cdecl(h, FP_GET_MNG_PTR, &[fp, 0, mng]);
                if h.read::<u32>(mng) != 0 {
                    let fp: u32 = h.read(at(work(h), 0x62c));
                    cdecl(h, FP_RESET_STICK, &[fp]);
                    let w = work(h);
                    let fp: u32 = h.read(at(w, 0x62c));
                    let chr: u32 = h.read(at(w, 0x634));
                    match h.read::<u32>(at(w, 0x630)) {
                        1 => {
                            let part: u32 = h.read(at(w, 0x638));
                            let handle = cdecl(h, GET_CHR_HANDLE, &[chr]);
                            let fp: u32 = h.read(at(work(h), 0x62c));
                            cdecl(h, FP_SET_STICK_CHR, &[fp, handle, part]);
                        }
                        2 => {
                            cdecl(h, FP_SET_STICK_MAP, &[fp, chr]);
                        }
                        3 => {
                            let p: u32 = h.read(mng);
                            let f: u32 = h.read(p);
                            h.write(p, f | 4);
                        }
                        _ => {}
                    }
                    let p: u32 = h.read(mng);
                    let f: u32 = h.read(p);
                    h.write(p, f | 0x60);
                    let scale: u32 = h.read(h.rebase(FP_SCALE));
                    let rot = ebp.wrapping_sub(0x24);
                    let m = ebp.wrapping_sub(0x64);
                    // junk read: [ebp-0x64], [ebp-0x60] never written
                    // (brief 2026-10-24); the frame's words as they are
                    let (a, b, c): (u32, u32, u32) =
                        (h.read(mng), h.read(m), h.read(m.wrapping_add(4)));
                    let ang = at(work(h), 0x64c);
                    cdecl(h, SCALE_VECTOR, &[rot, ang, scale, a, b, c]);
                    cdecl(h, UNIT_MATRIX, &[m]);
                    let sc = at(work(h), 0x65c);
                    cdecl(h, SET_SCALE_VEC3, &[m, sc]);
                    let pos = at(work(h), 0x63c);
                    let dst = h.read::<u32>(mng).wrapping_add(0xc);
                    cdecl(h, RT_M_YXZ, &[dst, m, rot, pos]);
                }
                let v: u32 = h.read(at(work(h), 0x62c));
                text_d(h, ebp, window, 0x78, v);
                let mode: u32 = h.read(at(work(h), 0x630));
                if let Some(&s) = FP_MODE_NAMES.get(mode as usize) {
                    text(h, window, 0x79, s);
                }
                for (id, off) in [(0x7a, 0x634), (0x7b, 0x638)] {
                    let v: u32 = h.read(at(work(h), off));
                    text_d(h, ebp, window, id, v);
                }
                let mut r = 0;
                for (id, off, fmt) in [
                    (0x7c, 0x63c, FORMAT_2F),
                    (0x7e, 0x640, FORMAT_2F),
                    (0x80, 0x644, FORMAT_2F),
                    (0x82, 0x64c, FORMAT_1F),
                    (0x84, 0x650, FORMAT_1F),
                    (0x86, 0x654, FORMAT_1F),
                    (0x88, 0x65c, FORMAT_2F),
                    (0x8a, 0x660, FORMAT_2F),
                    (0x8c, 0x664, FORMAT_2F),
                ] {
                    let p = at(work(h), off);
                    r = text_f(h, ebp, window, id, fmt, p);
                }
                r
            }
            _ if i <= 0x10 => 5,
            _ => i,
        })
    }

    /// The particle window's items; its entries' titles; the mode names
    /// ("Norm.", "Chr.Y", "Ang.Y", "Pos."); `rcuiParProcKick(start)`.
    pub const PART_ITEMS: u32 = 0x00c6_01f0;
    pub const PART_TITLES: [u32; 9] = [
        0x00b6_872c,
        0x00b6_8738,
        0x00b6_8744,
        0x00b6_8644,
        0x00b6_8650,
        0x00b6_865c,
        0x00b6_8668,
        0x00b6_8674,
        0x00b6_8680,
    ];
    pub const PART_MODE_NAMES: [u32; 4] = [0x00b6_7f88, 0x00b6_8750, 0x00b6_8758, 0x00b6_7ec4];
    pub const PAR_PROC_KICK: u32 = 0x0093_6340;
    /// `rcuiParticleProc_00935970`'s frame.
    pub const PARTICLE_FRAME: u32 = 0x18;
    /// The particle's nine floats (position, angles, scale).
    const PART_FLOATS: [u32; 9] = [
        0x924, 0x928, 0x92c, 0x934, 0x938, 0x93c, 0x944, 0x948, 0x94c,
    ];

    /// `rcuiParticleProc_00935970(window, msg, param)` -> `eax` as the
    /// original (`None`: no work area, the cookie's `eax`): the particle
    /// test window over `g_RcuiWork`: `+0x914`/`+0x918` two numbers,
    /// `+0x91c` the repeat flag, `+0x920` the mode (0..3), `+0x924..` the
    /// position, `+0x934..` the angles, `+0x944..` the scale; `+0x44` its
    /// window holder.
    ///
    /// 0x8001 adds the items and sets the scale to 0.1; 0x8003 (closed)
    /// clears the flag and `+0x44`; 0x8006 (left click) 0xa0..0xa2 step up
    /// (the mode wrapping at 4), 0xa3 starts the particle
    /// (`rcuiParProcKick(1)`), the even buttons 0xa4..0xb4 open the float
    /// entries, the odd ones add 0.1, 10 or 0.05, 0xb6 copies the scale's x
    /// to y and z; 0x800a (right click) steps down, flips the flag, zeroes
    /// (the scale: 0.1) or subtracts; 0x8011 restarts it while the flag is
    /// set (`rcuiParProcKick(0)`) and writes the texts. `ebp` is the
    /// original's frame (the text at `ebp - 0x14`, the scale copy at
    /// `- 0x18`).
    #[allow(clippy::too_many_lines)]
    pub fn particle_proc<H: Host>(
        h: &mut H,
        ebp: u32,
        window: u32,
        msg: u32,
        param: u32,
    ) -> Option<u32> {
        let w = work(h);
        if w == 0 {
            return None;
        }
        let i = msg.wrapping_sub(0x8001);
        let j = param.wrapping_sub(0xa0);
        let step = |h: &H, k: u32| match k {
            0..=2 => k64(h, TENTH),
            3..=5 => k64(h, TEN),
            _ => k64(h, FIVE_HUNDREDTHS),
        };
        Some(match msg {
            0x8001 => {
                let items = h.rebase(PART_ITEMS);
                cdecl(h, ADD_ITEMS, &[window, items]);
                let v: u32 = h.read(h.rebase(F32_TENTH));
                for off in [0x94c, 0x948, 0x944] {
                    h.write(at(work(h), off), v);
                }
                work(h)
            }
            0x8003 => {
                h.write(at(w, 0x91c), 0_u32);
                let w = work(h);
                h.write(at(w, 0x44), 0_u32);
                w
            }
            0x8006 => match j {
                0 | 1 => {
                    let off = 0x914 + 4 * j;
                    let v = h.read::<u32>(at(w, off)).wrapping_add(1);
                    h.write(at(w, off), v);
                    j
                }
                2 => {
                    let v = h.read::<u32>(at(w, 0x920)).wrapping_add(1);
                    h.write(at(w, 0x920), v);
                    if v.cast_signed() < 4 {
                        j
                    } else {
                        let w = work(h);
                        h.write(at(w, 0x920), 0_u32);
                        w
                    }
                }
                3 => cdecl(h, PAR_PROC_KICK, &[1]),
                4..=0x15 if j.is_multiple_of(2) => {
                    let k = (j - 4) / 2;
                    let value = PART_FLOATS[k as usize];
                    let title = h.rebase(PART_TITLES[k as usize]);
                    cdecl(
                        h,
                        NUMBER_CREATE,
                        &[at(w, 0x954 + 0x2c * k), at(w, value), 0xb, title, window],
                    )
                }
                4..=0x15 => {
                    let k = (j - 5) / 2;
                    let d = step(h, k);
                    add_f(h, at(w, PART_FLOATS[k as usize]), d, false);
                    j
                }
                0x16 => {
                    // fstp dword [ebp-0x18], fld: the f32 kept in the frame
                    let x = x87::store(x87::load(h.read(at(w, 0x944))));
                    h.write(ebp.wrapping_sub(0x18), x);
                    h.write(at(w, 0x94c), x);
                    let w = work(h);
                    h.write(at(w, 0x948), x);
                    w
                }
                _ => j,
            },
            0x800a => match j {
                0 | 1 => {
                    let off = 0x914 + 4 * j;
                    let v = h.read::<u32>(at(w, off)).wrapping_sub(1);
                    h.write(at(w, off), v);
                    j
                }
                2 => {
                    let v = h.read::<u32>(at(w, 0x920)).wrapping_sub(1);
                    h.write(at(w, 0x920), v);
                    if v.cast_signed() < 0 {
                        let w = work(h);
                        h.write(at(w, 0x920), 3_u32);
                        w
                    } else {
                        j
                    }
                }
                3 => {
                    let f: u32 = h.read(at(w, 0x91c));
                    h.write(at(w, 0x91c), f ^ 1);
                    j
                }
                4..=0x15 if j.is_multiple_of(2) => {
                    let k = (j - 4) / 2;
                    let v = if k < 6 {
                        0
                    } else {
                        h.read(h.rebase(F32_TENTH))
                    };
                    h.write(at(w, PART_FLOATS[k as usize]), v);
                    j
                }
                4..=0x15 => {
                    let k = (j - 5) / 2;
                    let d = step(h, k);
                    add_f(h, at(w, PART_FLOATS[k as usize]), d, true);
                    j
                }
                _ => j,
            },
            0x8011 => {
                if h.read::<u32>(at(w, 0x91c)) != 0 {
                    cdecl(h, PAR_PROC_KICK, &[0]);
                }
                for (id, off) in [(0xa0, 0x914), (0xa1, 0x918)] {
                    let v: u32 = h.read(at(work(h), off));
                    text_d(h, ebp, window, id, v);
                }
                let buf = ebp.wrapping_sub(0x14);
                let mode: u32 = h.read(at(work(h), 0x920));
                if let Some(&s) = PART_MODE_NAMES.get(mode as usize) {
                    // the name is the format, with no arguments
                    let s = h.rebase(s);
                    let f: u32 = h.read(h.rebase(SPRINTF_SLOT));
                    h.call(f, Conv::Cdecl, &[buf, s]);
                }
                cdecl(h, SET_TEXT, &[window, 0xa2, buf]);
                let mut r = 0;
                for (k, &off) in PART_FLOATS.iter().enumerate() {
                    let fmt = if (3..6).contains(&k) {
                        FORMAT_1F
                    } else {
                        FORMAT_2F
                    };
                    let p = at(work(h), off);
                    r = text_f(h, ebp, window, 0xa4 + 2 * k as u32, fmt, p);
                }
                r
            }
            _ if i <= 0x10 => 5,
            _ => i,
        })
    }

    /// `rcdtRsd2New()`, `rcdtRsdLoad_009243c0(rsd, path)`,
    /// `rcdtListsDelete(rsd)`, `rcAlloc(size, file, line)`, the CRT's
    /// `memset` thunk, `rcbgModelCreate(model, &desc)`,
    /// `rcbgModelGetPrim(model, k)`, `rcBBoxSet(box, lo, hi)`,
    /// `rcBBoxGetShortConvertScale(box)` (`ST0`), `rcbgModelFToS0(data,
    /// lo, hi, scale)`, `rcbgModelSetScale(model, scale)`; the source file
    /// name `rcAlloc` records.
    pub const RSD2_NEW: u32 = 0x0092_4750;
    pub const RSD_LOAD: u32 = 0x0092_43c0;
    pub const LISTS_DELETE: u32 = 0x0092_42d0;
    pub const RC_ALLOC: u32 = 0x0092_58a0;
    pub const MEMSET: u32 = 0x0094_964c;
    pub const MODEL_CREATE: u32 = 0x0092_7be0;
    pub const MODEL_GET_PRIM: u32 = 0x0092_82a0;
    pub const BBOX_SET: u32 = 0x0092_1530;
    pub const BBOX_SHORT_SCALE: u32 = 0x0092_1410;
    pub const MODEL_F_TO_S0: u32 = 0x0092_81d0;
    pub const MODEL_SET_SCALE: u32 = 0x0092_8540;
    pub const LOAD_TEXT_FILE: u32 = 0x00b6_8be8;
    /// `rcbgModelCreateFromFile`'s frame.
    pub const MODEL_FILE_FRAME: u32 = 0x40;

    /// A primitive's kind (0..3): bit 1 of its flags byte, and bit 2 from
    /// bit 0 (`movsx`, `and 2`, `or 4`, `shr 1`).
    fn prim_kind(h: &impl Host, prim: u32) -> u32 {
        let f = i32::from(h.read::<i8>(prim)).cast_unsigned();
        let k = if f & 1 == 0 { f & 2 } else { (f & 2) | 4 };
        k >> 1
    }

    /// `rcbgModelCreateFromFile(model, path)` -> 1, or 0 when the `.rsd`
    /// doesn't load. Reads the file into a new `rcdtRsd2` (`+0`/`+4` the
    /// bounds' corners, `+0x10` the primitive count, `+0x14` the
    /// primitives, 0x58 bytes each: `+0` flags, `+4..+0x10` four u16
    /// indexes in words, `+0x24` the group, `+0x48..` four words; `+0x18`
    /// the group count, at least 1), counts the primitives per group and
    /// kind (the quirk kept: a group whose first primitive is in it counts
    /// every primitive), builds the model from one entry per non-empty
    /// count (kind 0/2/4/6, count, group: `rcbgModelCreate(model, &{n,
    /// entries, [rsd], 0})`), then copies each group's primitives of each
    /// kind into the model's next primitive block (kinds 0/1: three words
    /// and three u16, stride 0x14/0x20; kinds 2/3: four words, y and z
    /// swapped, four u16 in the order 0, 2, 1, 3, stride 0x18/0x28), sets
    /// the bounds and the short-coordinate scale and deletes the file. The
    /// counts and the entries are never freed (as in the original). `ebp`
    /// is the original's frame (the kinds at `ebp - 0x14`, the descriptor
    /// at `- 0x40`; the scale's f32 at `- 0x1c`).
    #[allow(clippy::too_many_lines)]
    pub fn model_create_from_file<H: Host>(h: &mut H, ebp: u32, model: u32, path: u32) -> u32 {
        let fr = |o: u32| ebp.wrapping_sub(o);
        for (k, v) in [0_u32, 2, 4, 6].into_iter().enumerate() {
            h.write(fr(0x14 - 4 * k as u32), v);
        }
        let rsd = cdecl(h, RSD2_NEW, &[]);
        if cdecl(h, RSD_LOAD, &[rsd, path]) == 0 {
            if rsd != 0 {
                cdecl(h, LISTS_DELETE, &[rsd]);
            }
            return 0;
        }
        let groups = match h.read::<u32>(at(rsd, 0x18)) {
            0 => 1,
            n => n,
        };
        let slots = groups.wrapping_shl(2);
        let size = slots.wrapping_shl(2);
        let file = h.rebase(LOAD_TEXT_FILE);
        let counts = cdecl(h, RC_ALLOC, &[size, file, 0x4c2]);
        cdecl(h, MEMSET, &[counts, 0, size]);
        let n_prims = |h: &H| -> i32 { h.read::<u32>(at(rsd, 0x10)).cast_signed() };
        let mut g = 0_u32;
        while g.cast_signed() < groups.cast_signed() {
            let first: u32 = h.read(at(rsd, 0x14));
            if h.read::<u32>(at(first, 0x24)) == g {
                let mut prim = first;
                let mut k = 0_i32;
                while k < n_prims(h) {
                    let c = at(counts, 0x10 * g + 4 * prim_kind(h, prim));
                    k += 1;
                    let v = h.read::<u32>(c).wrapping_add(1);
                    h.write(c, v);
                    prim = prim.wrapping_add(0x58);
                }
            }
            g = g.wrapping_add(1);
        }
        let mut used = 0_u32;
        let mut i = 0_u32;
        while i.cast_signed() < slots.cast_signed() {
            if h.read::<u32>(at(counts, 4 * i)) != 0 {
                used = used.wrapping_add(1);
            }
            i = i.wrapping_add(1);
        }
        h.write(fr(0x40), used);
        let table = cdecl(h, RC_ALLOC, &[used.wrapping_mul(12), file, 0x4df]);
        h.write(fr(0x3c), table);
        let lo: u32 = h.read(rsd);
        h.write(fr(0x38), lo);
        h.write(fr(0x34), 0_u32);
        let mut e = table;
        let mut i = 0_u32;
        while i.cast_signed() < slots.cast_signed() {
            let c: u32 = h.read(at(counts, 4 * i));
            if c != 0 {
                let kind: u32 = h.read(fr(0x14 - 4 * (i & 3)));
                h.write(e, kind);
                h.write(at(e, 4), c);
                h.write(at(e, 8), i >> 2);
                e = e.wrapping_add(0xc);
            }
            i = i.wrapping_add(1);
        }
        cdecl(h, MODEL_CREATE, &[model, fr(0x40)]);
        let mut block = 0_u32;
        let mut i = 0_u32;
        while i.cast_signed() < slots.cast_signed() {
            if h.read::<u32>(at(counts, 4 * i)) != 0 {
                let (kind, group) = (i & 3, i >> 2);
                let prims: u32 = h.read(at(rsd, 0x14));
                let mut dst = cdecl(h, MODEL_GET_PRIM, &[model, block]);
                if dst != 0 {
                    let mut prim = prims;
                    let mut k = 0_i32;
                    while k < n_prims(h) {
                        if h.read::<u32>(at(prim, 0x24)) == group && prim_kind(h, prim) == kind {
                            let w = |h: &H, o: u32| -> u32 { h.read(at(prim, o)) };
                            let s = |h: &H, o: u32| -> u16 { h.read(at(prim, o)) };
                            if kind < 2 {
                                for (d, o) in [(0, 0x48), (4, 0x4c), (8, 0x50)] {
                                    let v = w(h, o);
                                    h.write(at(dst, d), v);
                                }
                                for (d, o) in [(0xc, 4), (0xe, 8), (0x10, 0xc)] {
                                    let v = s(h, o);
                                    h.write(at(dst, d), v);
                                }
                                dst = dst.wrapping_add(if kind == 0 { 0x14 } else { 0x20 });
                            } else {
                                for (d, o) in [(0, 0x48), (4, 0x50), (8, 0x4c), (0xc, 0x54)] {
                                    let v = w(h, o);
                                    h.write(at(dst, d), v);
                                }
                                for (d, o) in [(0x10, 4), (0x12, 0xc), (0x14, 8), (0x16, 0x10)] {
                                    let v = s(h, o);
                                    h.write(at(dst, d), v);
                                }
                                dst = dst.wrapping_add(if kind == 2 { 0x18 } else { 0x28 });
                            }
                        }
                        k += 1;
                        prim = prim.wrapping_add(0x58);
                    }
                    block = block.wrapping_add(1);
                }
            }
            i = i.wrapping_add(1);
        }
        let bbox = at(model, 0x90);
        let (lo, hi): (u32, u32) = (h.read(rsd), h.read(at(rsd, 4)));
        cdecl(h, BBOX_SET, &[bbox, hi, lo]);
        let scale = x87::store(h.call_sym_st0_f64(BBOX_SHORT_SCALE, Conv::Cdecl, &[bbox]));
        h.write(fr(0x1c), scale);
        let t: u32 = h.read(at(model, 0x14));
        let data = h.read::<u32>(at(t, 8)).wrapping_add(t);
        let (lo, hi): (u32, u32) = (h.read(rsd), h.read(at(rsd, 4)));
        cdecl(h, MODEL_F_TO_S0, &[data, hi, lo, scale]);
        cdecl(h, MODEL_SET_SCALE, &[model, scale]);
        cdecl(h, LISTS_DELETE, &[rsd]);
        1
    }
}
