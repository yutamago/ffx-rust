//! M5 stage 1, round 6 (list `left6`, from `map2.left`), batch 6: the map
//! tool's load window (`rcuiLoadProc`): a file list in columns, picked
//! with the mouse, a directory then a map.
//!
//! Callees stay calls to the originals (cdecl). The frame (the list's
//! layout, the selection, the path text) comes in as the original's `ebp`.

/// `g_RcuiWork`, `g_YnMapSystem`.
pub const RCUI_WORK: u32 = 0x0193_c0f0;
pub const YN_MAP_SYSTEM: u32 = 0x0193_bf80;

// ---- callees ----

/// `Gu_WindowGetWidth(window)`, `Gu_WindowGetHeight(window)`,
/// `Gu_MouseGetWindowY(window)`, `Gu_MouseGetWindowX(window)`,
/// `rcuiMakeBasePath()`, `rcuiLoadSetTitle()`, `Gu_MakeFileList(list,
/// path)`, `rcuiMakeFullPath()`, `rcuiReset()`, `rcbgPreLoadText()`,
/// `rcdtChunkNew()`, `rcdtChunkLoadFromFile(chunk, path)`,
/// `Yn_PreBgSetData(data)`, `get_ptr32_func(p)`, `rcFree(p)`,
/// `set_ptr32(p)`, `rcbgIDLoad()`, `rcuiResetChrPos()`,
/// `rcuiCloseWindow(holder)`, the print helpers.
pub const GET_WIDTH: u32 = 0x0084_afa0;
pub const GET_HEIGHT: u32 = 0x0084_af30;
pub const MOUSE_WINDOW_Y: u32 = 0x0084_9aa0;
pub const MOUSE_WINDOW_X: u32 = 0x0084_9ac0;
pub const MAKE_BASE_PATH: u32 = 0x0093_7af0;
pub const LOAD_SET_TITLE: u32 = 0x0093_4fb0;
pub const MAKE_FILE_LIST: u32 = 0x0084_98e0;
pub const MAKE_FULL_PATH: u32 = 0x0093_7b20;
pub const RESET: u32 = 0x0093_6570;
pub const PRE_LOAD_TEXT: u32 = 0x0093_ae60;
pub const CHUNK_NEW: u32 = 0x0092_b900;
pub const CHUNK_LOAD: u32 = 0x0092_b880;
pub const PRE_BG_SET_DATA: u32 = 0x0091_aa60;
pub const GET_PTR32: u32 = 0x00a4_4650;
pub const RC_FREE: u32 = 0x0092_58e0;
pub const SET_PTR32: u32 = 0x00a4_4700;
pub const ID_LOAD: u32 = 0x0092_5730;
pub const RESET_CHR_POS: u32 = 0x0093_7b70;
pub const CLOSE_WINDOW: u32 = 0x0093_7f50;
pub const PRINT_SET_X: u32 = 0x0084_8820;
pub const PRINT_SET_Y: u32 = 0x0084_8860;
pub const PRINT_SET_BOLD: u32 = 0x0084_8720;
pub const GU_PRINTF: u32 = 0x0084_8880;
/// `MSVCR110` `sprintf` slot; the directory path format and the chunk
/// path format.
pub const SPRINTF_SLOT: u32 = 0x00b0_c3dc;
pub const DIR_FORMAT: u32 = 0x00b4_0eb0;
pub const CHUNK_FORMAT: u32 = 0x00b6_84e8;

/// `rcuiLoadProc`'s frame.
pub const LOAD_FRAME: u32 = 0x11c;

pub mod bridge {
    use super::{
        CHUNK_FORMAT, CHUNK_LOAD, CHUNK_NEW, CLOSE_WINDOW, DIR_FORMAT, GET_HEIGHT, GET_PTR32,
        GET_WIDTH, GU_PRINTF, ID_LOAD, LOAD_SET_TITLE, MAKE_BASE_PATH, MAKE_FILE_LIST,
        MAKE_FULL_PATH, MOUSE_WINDOW_X, MOUSE_WINDOW_Y, PRE_BG_SET_DATA, PRE_LOAD_TEXT,
        PRINT_SET_BOLD, PRINT_SET_X, PRINT_SET_Y, RC_FREE, RCUI_WORK, RESET, RESET_CHR_POS,
        SET_PTR32, SPRINTF_SLOT, YN_MAP_SYSTEM,
    };
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    fn work(h: &impl Host) -> u32 {
        h.read(h.rebase(RCUI_WORK))
    }

    fn sys(h: &impl Host) -> u32 {
        h.read(h.rebase(YN_MAP_SYSTEM))
    }

    fn sprintf(h: &mut impl Host, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(SPRINTF_SLOT));
        h.call(f, Conv::Cdecl, args)
    }

    /// `v / n` toward zero (`n` a positive constant).
    fn div(v: u32, n: i32) -> u32 {
        (v.cast_signed() / n).cast_unsigned()
    }

    /// `strcpy(dst, src)` as the original's byte loop.
    fn copy_str(h: &mut impl Host, dst: u32, src: u32) {
        let mut k = 0_u32;
        loop {
            let c: u8 = h.read(src.wrapping_add(k));
            h.write(dst.wrapping_add(k), c);
            if c == 0 {
                break;
            }
            k = k.wrapping_add(1);
        }
    }

    /// The map chosen (`+0x27c`): its full path, the tool reset, its text
    /// (`+0x60` 0: `rcbgPreLoadText`) or its background chunk (`+0x174`
    /// through the chunk at `g_YnMapSystem + 0x18`, made on first use),
    /// the scene ids reloaded (`+0xc00`), the player placed, the scene
    /// flag (`+0x5c`) and the window closed.
    fn load_map(h: &mut impl Host, ebp: u32) -> u32 {
        let buf = ebp.wrapping_sub(0x104);
        cdecl(h, MAKE_FULL_PATH, &[]);
        cdecl(h, RESET, &[]);
        let w = work(h);
        if h.read::<u32>(w.wrapping_add(0x60)) == 0 {
            cdecl(h, PRE_LOAD_TEXT, &[]);
        } else {
            if h.read::<u32>(sys(h).wrapping_add(0x18)) == 0 {
                let c = cdecl(h, CHUNK_NEW, &[]);
                let at = sys(h).wrapping_add(0x18);
                h.write(at, c);
            }
            let w = work(h);
            let fmt = h.rebase(CHUNK_FORMAT);
            sprintf(h, &[buf, fmt, w.wrapping_add(0x174)]);
            let chunk: u32 = h.read(sys(h).wrapping_add(0x18));
            if cdecl(h, CHUNK_LOAD, &[chunk, buf]) != 0 {
                let chunk: u32 = h.read(sys(h).wrapping_add(0x18));
                let data: u32 = h.read(chunk);
                cdecl(h, PRE_BG_SET_DATA, &[data]);
            }
        }
        let ids: u32 = h.read(work(h).wrapping_add(0xc00));
        let p = cdecl(h, GET_PTR32, &[ids]);
        if p != 0 {
            cdecl(h, RC_FREE, &[p]);
        }
        let v = cdecl(h, SET_PTR32, &[0]);
        h.write(work(h).wrapping_add(0xc00), v);
        let id = cdecl(h, ID_LOAD, &[]);
        let v = cdecl(h, SET_PTR32, &[id]);
        h.write(work(h).wrapping_add(0xc00), v);
        cdecl(h, RESET_CHR_POS, &[]);
        h.write(work(h).wrapping_add(0x5c), 1_u32);
        let holder = work(h).wrapping_add(4);
        cdecl(h, CLOSE_WINDOW, &[holder])
    }

    /// `rcuiLoadProc(window, msg, param)` -> `eax` as the original: the
    /// load window. Without a work area 0, without a system the work
    /// area. The list (`g_RcuiWork + 0x68`: `+0` its count, `+8` the
    /// names) fills columns of 80 by rows of 10 pixels; the entry under
    /// the mouse is the selection. 0x800e toggles the text/binary flag
    /// (`+0x64`), makes the base path and title, then as 0x8001/0x800a:
    /// the list of the base directory (`+0x74`), the directory (`+0x274`)
    /// and map (`+0x27c`) names cleared; 0x8003 (closed) clears the holder
    /// `+4`; 0x8006 picks the selected entry: first a directory (the list
    /// of it), then a map ([`load_map`]); 0x8011 draws the list, the
    /// selection bold. `ebp` is the original's frame (`- 0x104` the path,
    /// `- 0x108..- 0x11c` the layout and cursors).
    #[allow(clippy::too_many_lines)]
    pub fn load_proc(h: &mut impl Host, ebp: u32, window: u32, msg: u32) -> u32 {
        let fr = |off: u32| ebp.wrapping_sub(off);
        let w = work(h);
        if w == 0 {
            return 0;
        }
        if sys(h) == 0 {
            return w;
        }
        let list = w.wrapping_add(0x68);
        h.write(fr(0x108), list);
        let cols = div(cdecl(h, GET_WIDTH, &[window]), 80);
        h.write(fr(0x110), cols);
        let rows = div(cdecl(h, GET_HEIGHT, &[window]), 10).wrapping_sub(1);
        h.write(fr(0x114), rows);
        let mx = div(cdecl(h, MOUSE_WINDOW_Y, &[window]), 80);
        let my = div(cdecl(h, MOUSE_WINDOW_X, &[window]), 10).wrapping_sub(1);
        let si = |v: u32| v.cast_signed();
        let mut sel = u32::MAX;
        if si(mx) >= 0 && si(mx) < si(cols) && si(my) >= 0 && si(my) < si(rows) {
            let k = mx.wrapping_mul(rows).wrapping_add(my);
            if si(k) < si(h.read(list)) {
                sel = k;
            }
        }
        h.write(fr(0x10c), sel);
        let i = msg.wrapping_sub(0x8001);
        match msg {
            0x800e | 0x8001 | 0x800a => {
                if msg == 0x800e {
                    let at = work(h).wrapping_add(0x64);
                    let v: u32 = h.read(at);
                    h.write(at, v ^ 1);
                    cdecl(h, MAKE_BASE_PATH, &[]);
                    cdecl(h, LOAD_SET_TITLE, &[]);
                }
                let base = work(h).wrapping_add(0x74);
                cdecl(h, MAKE_FILE_LIST, &[list, base]);
                h.write(work(h).wrapping_add(0x274), 0_u8);
                let w = work(h);
                h.write(w.wrapping_add(0x27c), 0_u8);
                w
            }
            0x8003 => {
                let w = work(h);
                h.write(w.wrapping_add(4), 0_u32);
                w
            }
            0x8006 => {
                if si(sel) < 0 {
                    return 2;
                }
                let w = work(h);
                let names: u32 = h.read(list.wrapping_add(8));
                let name: u32 = h.read(names.wrapping_add(sel.wrapping_mul(4)));
                if h.read::<u8>(w.wrapping_add(0x274)) == 0 {
                    copy_str(h, w.wrapping_add(0x274), name);
                    let w = work(h);
                    let buf = fr(0x104);
                    let fmt = h.rebase(DIR_FORMAT);
                    sprintf(h, &[buf, fmt, w.wrapping_add(0x74), w.wrapping_add(0x274)]);
                    cdecl(h, MAKE_FILE_LIST, &[list, buf])
                } else {
                    copy_str(h, w.wrapping_add(0x27c), name);
                    load_map(h, ebp)
                }
            }
            0x8011 => {
                h.write(fr(0x11c), 0_u32);
                if si(cols) <= 0 {
                    return 4;
                }
                let mut x = 0_u32;
                h.write(fr(0x118), x);
                let mut k = 0_u32;
                for col in 1..=cols {
                    let mut y = 0_u32;
                    for _ in 0..si(rows).max(0) {
                        if si(k) >= si(h.read(list)) {
                            return x;
                        }
                        cdecl(h, PRINT_SET_X, &[x]);
                        cdecl(h, PRINT_SET_Y, &[y]);
                        cdecl(h, PRINT_SET_BOLD, &[u32::from(k == sel)]);
                        let names: u32 = h.read(list.wrapping_add(8));
                        let name: u32 = h.read(names.wrapping_add(k.wrapping_mul(4)));
                        cdecl(h, GU_PRINTF, &[name]);
                        k = k.wrapping_add(1);
                        y = y.wrapping_add(10);
                    }
                    x = x.wrapping_add(0x50);
                    h.write(fr(0x11c), col);
                    h.write(fr(0x118), x);
                }
                x
            }
            _ if i <= 0x10 => 5,
            _ => i,
        }
    }

    /// `rcuiAddItems`, `rcuiNumWindowCreate_0093c670(holder, value, title,
    /// parent)`, `rcuiTake14/1C/18(item)`, the packet and text helpers.
    pub const ADD_ITEMS: u32 = 0x0093_7780;
    pub const NUM_WINDOW_CREATE: u32 = 0x0093_c670;
    pub const TAKE_14: u32 = 0x0093_c840;
    pub const TAKE_1C: u32 = 0x0093_c820;
    pub const TAKE_18: u32 = 0x0093_c800;
    pub const PACKET_BEGIN_TAG: u32 = 0x0084_8530;
    pub const SET_PRIM: u32 = 0x0084_8480;
    pub const SET_RGBAQ: u32 = 0x0084_84c0;
    pub const SET_XYZF2: u32 = 0x0084_8680;
    pub const PACKET_END_TAG: u32 = 0x0084_8460;
    pub const SET_TEXT: u32 = 0x0084_a710;
    /// The rectangle window's items, its entry title, `"%d"`.
    pub const RECT_ITEMS: u32 = 0x00c5_f0a8;
    pub const RECT_TITLE: u32 = 0x00b6_852c;
    pub const FORMAT_D: u32 = 0x00b0_f604;
    /// `0x00932a60`'s frame.
    pub const RECT_FRAME: u32 = 0x18;
    /// The rectangle window's drawing: twelve points.
    const RECT_POINTS: [(u32, u32); 16] = [
        (0x3d, 5),
        (0x53, 5),
        (0x68, 5),
        (0x7f, 5),
        (0x3d, 0x39),
        (0x53, 0x39),
        (0x68, 0x39),
        (0x7f, 0x39),
        (0x1e, 0xd),
        (0x1e, 0x19),
        (0x1e, 0x27),
        (0x1e, 0x33),
        (0x9e, 0xd),
        (0x9e, 0x19),
        (0x9e, 0x27),
        (0x9e, 0x33),
    ];

    /// An `f32` copied through the x87.
    fn cp(bits: u32) -> u32 {
        ffx_bridge::x87::store(ffx_bridge::x87::load(bits))
    }

    /// `FUN_00932a60(window, msg, param)` -> `eax` as the original
    /// (`None`: no work area, map or layer, the cookie's `eax`): the
    /// selected layer's rectangle (`+0x14..+0x20`: x0, y0, x1, y1). 0x8001
    /// adds its items and keeps four presets (`+0x3b8..`: x0, x1, x0, y0);
    /// 0x8003 (closed) clears `+0x18`; 0x8006 without an entry open
    /// (`+0x398`) keeps the rectangle (`+0x3c8..`) and opens a number entry
    /// on a coordinate (0x36..0x39) or a preset (0x3a..0x3d), the button in
    /// `+0x3d8`; 0x8011 takes an entered preset into its pair of
    /// coordinates, restores the rectangle on a cancel, draws the frame and
    /// writes the four coordinates (`ftol`). `ebp` is the original's frame
    /// (the text at `ebp - 0x14`, a copied preset at `- 0x18`).
    #[allow(clippy::too_many_lines)]
    pub fn rect_proc(
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
        let sys = sys(h);
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
        let e = |off: u32| ent.wrapping_add(off);
        let i = msg.wrapping_sub(0x8001);
        Some(match msg {
            0x8001 => {
                let items = h.rebase(RECT_ITEMS);
                cdecl(h, ADD_ITEMS, &[window, items]);
                for (dst, src) in [
                    (0x3b8_u32, 0x14_u32),
                    (0x3bc, 0x1c),
                    (0x3c0, 0x14),
                    (0x3c4, 0x18),
                ] {
                    let v = cp(h.read(e(src)));
                    h.write(work(h).wrapping_add(dst), v);
                }
                work(h)
            }
            0x8003 => {
                h.write(w.wrapping_add(0x18), 0_u32);
                1
            }
            0x8006 => {
                let entry = w.wrapping_add(0x398);
                if h.read::<u32>(entry) != 0 {
                    return Some(2);
                }
                let mut ww = w;
                for (j, src) in (0_u32..).zip([0x14_u32, 0x18, 0x1c, 0x20]) {
                    let v = cp(h.read(e(src)));
                    h.write(ww.wrapping_add(0x3c8 + 4 * j), v);
                    ww = work(h);
                }
                let j = param.wrapping_sub(0x36);
                if j > 7 {
                    let w = work(h);
                    h.write(w.wrapping_add(0x3d8), 0_u32);
                    return Some(w);
                }
                let value = if j < 4 {
                    ent.wrapping_add(param.wrapping_sub(0x31).wrapping_mul(4))
                } else {
                    work(h)
                        .wrapping_add(param.wrapping_mul(4))
                        .wrapping_add(0x2d0)
                };
                let title = h.rebase(RECT_TITLE);
                cdecl(h, NUM_WINDOW_CREATE, &[entry, value, title, window]);
                let w = work(h);
                h.write(w.wrapping_add(0x3d8), param);
                w
            }
            0x8011 => {
                let mut took = cdecl(h, TAKE_14, &[w.wrapping_add(0x398)]) != 0;
                if !took {
                    took = cdecl(h, TAKE_1C, &[work(h).wrapping_add(0x398)]) != 0;
                }
                let w = work(h);
                if took {
                    let sel = h.read::<u32>(w.wrapping_add(0x3d8)).wrapping_sub(0x3a);
                    if sel <= 3 {
                        let v = cp(cp(h.read(w.wrapping_add(0x3b8 + 4 * sel))));
                        h.write(ebp.wrapping_sub(0x18), v);
                        let (a, b) = [
                            (0x18_u32, 0x14_u32),
                            (0x20, 0x1c),
                            (0x1c, 0x14),
                            (0x20, 0x18),
                        ][sel as usize];
                        h.write(e(a), v);
                        h.write(e(b), v);
                    }
                }
                let w = work(h);
                if cdecl(h, TAKE_18, &[w.wrapping_add(0x398)]) != 0 {
                    for (j, dst) in (0_u32..).zip([0x14_u32, 0x18, 0x1c, 0x20]) {
                        let v = cp(h.read(work(h).wrapping_add(0x3c8 + 4 * j)));
                        h.write(e(dst), v);
                    }
                }
                cdecl(h, PACKET_BEGIN_TAG, &[window]);
                cdecl(h, SET_PRIM, &[1]);
                cdecl(h, SET_RGBAQ, &[0, 0, 0, 0]);
                for (x, y) in RECT_POINTS {
                    cdecl(h, SET_XYZF2, &[x, y]);
                }
                cdecl(h, PACKET_END_TAG, &[]);
                let buf = ebp.wrapping_sub(0x14);
                let fmt = h.rebase(FORMAT_D);
                let mut r = 0;
                for (off, id) in [
                    (0x14_u32, 0x36_u32),
                    (0x18, 0x37),
                    (0x1c, 0x38),
                    (0x20, 0x39),
                ] {
                    let v = ffx_bridge::x87::ftol(h, ffx_bridge::x87::load(h.read(e(off))));
                    sprintf(h, &[buf, fmt, v]);
                    r = cdecl(h, SET_TEXT, &[window, id, buf]);
                }
                r
            }
            _ if i <= 0x10 => 4,
            _ => i,
        })
    }

    /// The debug window's callees: `rcuiMouseMoveReset()`,
    /// `rcuiWindowCreate`, `rcuiLoadProc` and its title,
    /// `rcbgSceneSetMatrix(list)`, `rcbgPreSetSceneNo(n)`,
    /// `rcuiGuideProcConvertAll()`, `rcuiPreMemWinOpen()`,
    /// `pppFpStopStatus(on)`; f64 pi/180; the window's items, the text
    /// window's procedure and title, the three formats.
    pub const MOUSE_MOVE_RESET: u32 = 0x0093_c110;
    pub const WINDOW_CREATE: u32 = 0x0093_7ef0;
    pub const LOAD_PROC: u32 = 0x0093_4bc0;
    pub const LOAD_TITLE: u32 = 0x00b6_7e58;
    pub const SCENE_SET_MATRIX: u32 = 0x0092_0c00;
    pub const PRE_SET_SCENE_NO: u32 = 0x0092_17e0;
    pub const GUIDE_CONVERT_ALL: u32 = 0x0093_4250;
    pub const PRE_MEM_WIN_OPEN: u32 = 0x0093_64a0;
    pub const FP_STOP_STATUS: u32 = 0x0072_a750;
    pub const DEG_TO_RAD: u32 = 0x00b1_1330;
    pub const DEBUG_ITEMS: u32 = 0x00c6_08f0;
    pub const TEXT_PROC: u32 = 0x0093_6a00;
    pub const TEXT_TITLE: u32 = 0x00b6_8084;
    pub const DEBUG_FORMATS: [u32; 3] = [0x00b6_8818, 0x00b6_8820, 0x00b6_8828];
    /// `0x009325c0`'s frame.
    pub const DEBUG_FRAME: u32 = 0x18;

    /// The camera's field of view step: `+0xc04` moved by `d` degrees,
    /// `f32(f32(deg) * pi / 180)` into the camera (`[map + 8] + 0xac`;
    /// the degrees as f32 at `ebp - 0x18`), then its matrix.
    fn fov_step(h: &mut impl Host, ebp: u32, w: u32, map: u32, d: u32) -> u32 {
        let list: u32 = h.read(map.wrapping_add(8));
        let v: u32 = h.read(w.wrapping_add(0xc04));
        h.write(w.wrapping_add(0xc04), v.wrapping_add(d));
        let deg = ffx_bridge::x87::store(f64::from(
            h.read::<u32>(work(h).wrapping_add(0xc04)).cast_signed(),
        ));
        h.write(ebp.wrapping_sub(0x18), deg);
        let k = f64::from_bits(h.read(h.rebase(DEG_TO_RAD)));
        let f = ffx_bridge::x87::store(ffx_bridge::x87::mul(ffx_bridge::x87::load(deg), k));
        h.write(list.wrapping_add(0xac), f);
        cdecl(h, SCENE_SET_MATRIX, &[list])
    }

    /// The scene number step: `+0xc08` moved by `d`, clamped to 0..(the
    /// map's `+0` count - 1), then `rcbgPreSetSceneNo`.
    fn scene_step(h: &mut impl Host, w: u32, d: u32) -> u32 {
        let v: u32 = h.read(w.wrapping_add(0xc08));
        h.write(w.wrapping_add(0xc08), v.wrapping_add(d));
        let w = work(h);
        let mut c: u32 = h.read(w.wrapping_add(0xc08));
        if c.cast_signed() < 0 {
            c = 0;
        } else {
            let map: u32 = h.read(sys(h));
            let n: u32 = h.read(map);
            if c.cast_signed() > n.wrapping_sub(1).cast_signed() {
                c = n.wrapping_sub(1);
            }
        }
        h.write(w.wrapping_add(0xc08), c);
        let c: u32 = h.read(work(h).wrapping_add(0xc08));
        cdecl(h, PRE_SET_SCENE_NO, &[c])
    }

    /// `FUN_009325c0(window, msg, param)` -> `eax` as the original
    /// (`None`: no work area or system, the cookie's `eax`): the map
    /// tool's debug window. 0x8001 adds its items and resets the mouse
    /// move; 0x8003 (closed) clears `+0x54`; 0x8006 buttons 0xc8 the text
    /// load window, 0xc9 flips every layer's bit 1 (by the first's),
    /// 0xca/0xcb the field of view and scene number up, 0xcd a counter,
    /// 0xce the guide map conversion, 0xcf the text window, 0xd0 the
    /// memory window, 0xd1 the effects stopped (0xd1..0xd4 also call empty
    /// functions, left out); 0x800a 0xca/0xcb/0xcd down, 0xd1 the effects
    /// run; 0x8011 the three values. `ebp` is the original's frame.
    #[allow(clippy::too_many_lines)]
    pub fn debug_proc(
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
        let sys = sys(h);
        if sys == 0 {
            return None;
        }
        let i = msg.wrapping_sub(0x8001);
        Some(match msg {
            0x8001 => {
                let items = h.rebase(DEBUG_ITEMS);
                cdecl(h, ADD_ITEMS, &[window, items]);
                cdecl(h, MOUSE_MOVE_RESET, &[])
            }
            0x8003 => {
                h.write(w.wrapping_add(0x54), 0_u32);
                1
            }
            0x8006 => {
                let j = param.wrapping_sub(0xc8);
                match param {
                    0xc8 => {
                        h.write(w.wrapping_add(0x60), 1_u32);
                        let w = work(h);
                        let (proc_, title) = (h.rebase(LOAD_PROC), h.rebase(LOAD_TITLE));
                        cdecl(
                            h,
                            WINDOW_CREATE,
                            &[
                                w.wrapping_add(4),
                                8,
                                8,
                                0x154,
                                0xbe,
                                0xe,
                                proc_,
                                title,
                                window,
                            ],
                        );
                        cdecl(h, super::LOAD_SET_TITLE, &[])
                    }
                    0xc9 => {
                        let map: u32 = h.read(sys);
                        if map == 0 {
                            return Some(1);
                        }
                        let list: u32 = h.read(map.wrapping_add(8));
                        if list == 0 {
                            return Some(1);
                        }
                        let mut ent: u32 = h.read(list.wrapping_add(8));
                        if ent == 0 {
                            return Some(0);
                        }
                        let clear = h.read::<u8>(ent) & 2 != 0;
                        let mut k = 0_u32;
                        while k.cast_signed() < h.read::<u32>(list.wrapping_add(4)).cast_signed() {
                            let v: u32 = h.read(ent);
                            h.write(ent, if clear { v & !2 } else { v | 2 });
                            k = k.wrapping_add(1);
                            ent = ent.wrapping_add(0xc0);
                        }
                        ent
                    }
                    0xca => {
                        let map: u32 = h.read(sys);
                        if map == 0 {
                            return Some(2);
                        }
                        fov_step(h, ebp, w, map, 1)
                    }
                    0xcb => {
                        if h.read::<u32>(sys) == 0 {
                            return Some(3);
                        }
                        scene_step(h, w, 1)
                    }
                    0xcd => {
                        let v: u32 = h.read(w.wrapping_add(0xbfc));
                        h.write(w.wrapping_add(0xbfc), v.wrapping_add(1));
                        5
                    }
                    0xce => cdecl(h, GUIDE_CONVERT_ALL, &[]),
                    0xcf => {
                        let (proc_, title) = (h.rebase(TEXT_PROC), h.rebase(TEXT_TITLE));
                        cdecl(
                            h,
                            WINDOW_CREATE,
                            &[
                                w.wrapping_add(0x4c),
                                8,
                                8,
                                0x78,
                                0xab,
                                6,
                                proc_,
                                title,
                                window,
                            ],
                        )
                    }
                    0xd0 => cdecl(h, PRE_MEM_WIN_OPEN, &[]),
                    // 0xd1..0xd4 also call bare `ret`s (0x0092cca0..d0)
                    0xd1 => cdecl(h, FP_STOP_STATUS, &[1]),
                    _ => j,
                }
            }
            0x800a => {
                let j = param.wrapping_sub(0xca);
                match param {
                    0xca => {
                        let map: u32 = h.read(sys);
                        if map == 0 {
                            return Some(0);
                        }
                        fov_step(h, ebp, w, map, u32::MAX)
                    }
                    0xcb => {
                        if h.read::<u32>(sys) == 0 {
                            return Some(1);
                        }
                        scene_step(h, w, u32::MAX)
                    }
                    0xcd => {
                        let v: u32 = h.read(w.wrapping_add(0xbfc));
                        h.write(w.wrapping_add(0xbfc), v.wrapping_sub(1));
                        3
                    }
                    0xd1 => cdecl(h, FP_STOP_STATUS, &[0]),
                    _ => j,
                }
            }
            0x8011 => {
                let buf = ebp.wrapping_sub(0x14);
                let mut r = 0;
                for (fmt, off, id) in [
                    (DEBUG_FORMATS[0], 0xc04_u32, 0xca_u32),
                    (DEBUG_FORMATS[1], 0xc08, 0xcb),
                    (DEBUG_FORMATS[2], 0xbfc, 0xcd),
                ] {
                    let v: u32 = h.read(work(h).wrapping_add(off));
                    let fmt = h.rebase(fmt);
                    sprintf(h, &[buf, fmt, v]);
                    r = cdecl(h, SET_TEXT, &[window, id, buf]);
                }
                r
            }
            _ if i <= 0x10 => 5,
            _ => i,
        })
    }

    /// The character window's callees: `FUN_00932320()` (the selection
    /// reset), `rcuiChrMotionSetup()`; its items, the motion and model
    /// windows' procedures and titles, its two mode texts and range format.
    pub const CHR_SELECT_RESET: u32 = 0x0093_2320;
    pub const CHR_MOTION_SETUP: u32 = 0x0093_2470;
    pub const CHR_ITEMS: u32 = 0x00c5_ff68;
    pub const MOTION_PROC: u32 = 0x0093_5340;
    pub const MOTION_TITLE: u32 = 0x00b5_ac04;
    pub const MODEL_PROC: u32 = 0x0093_5970;
    pub const MODEL_TITLE: u32 = 0x00b6_8028;
    pub const MODE_TEXTS: [u32; 2] = [0x00b6_7fe0, 0x00b6_8020];
    pub const RANGE_FORMAT: u32 = 0x00b6_8708;
    /// `0x00931e90`'s frame.
    pub const CHR_FRAME: u32 = 0x14;

    /// `FUN_00931e90(window, msg, param)` -> `eax` as the original (`None`:
    /// no work area, the cookie's `eax`): the character window. The
    /// character `+0x8b0` (of `+0x824`, ids at `+0x830`), the element mode
    /// `+0x8b4`, the element range `+0x8c0` (ten of `+0x8b8`) and element
    /// `+0x8c4` (`+0x8bc` one picked), the part `+0x8d0` (of `+0x8cc`).
    /// 0x8001 adds its items and resets the selection; 0x8003 (closed)
    /// clears `+0x3c`; 0x8006 buttons step them up (wrapping), 0x93 the
    /// mode, 0x97/0x98 the motion and model windows; 0x800a down; 0x8011
    /// the texts. `ebp` is the original's frame (the text at `ebp - 0x14`).
    #[allow(clippy::too_many_lines)]
    pub fn chr_proc(h: &mut impl Host, ebp: u32, window: u32, msg: u32, param: u32) -> Option<u32> {
        let w = work(h);
        if w == 0 {
            return None;
        }
        let i = msg.wrapping_sub(0x8001);
        let si = |v: u32| v.cast_signed();
        Some(match msg {
            0x8001 => {
                let items = h.rebase(CHR_ITEMS);
                cdecl(h, ADD_ITEMS, &[window, items]);
                cdecl(h, CHR_SELECT_RESET, &[])
            }
            0x8003 => {
                h.write(w.wrapping_add(0x3c), 0_u32);
                1
            }
            0x8006 => match param {
                0x92 => {
                    let v = h.read::<u32>(w.wrapping_add(0x8b0)).wrapping_add(1);
                    h.write(w.wrapping_add(0x8b0), v);
                    if si(v) >= si(wget(h, 0x824)) {
                        wset(h, 0x8b0, 0);
                    }
                    cdecl(h, CHR_MOTION_SETUP, &[])
                }
                0x93 => {
                    let v: u32 = h.read(w.wrapping_add(0x8b4));
                    h.write(w.wrapping_add(0x8b4), v ^ 1);
                    1
                }
                0x94 => {
                    h.write(w.wrapping_add(0x8bc), 0_u32);
                    let v = wget(h, 0x8c0).wrapping_add(10);
                    wset(h, 0x8c0, v);
                    if si(wget(h, 0x8c0)) >= si(wget(h, 0x8b8)) {
                        wset(h, 0x8c0, 0);
                    }
                    let v = wget(h, 0x8c0);
                    wset(h, 0x8c4, v);
                    v
                }
                0x95 | 0x96 => {
                    let (at, n) = if param == 0x95 {
                        h.write(w.wrapping_add(0x8bc), 1_u32);
                        (0x8c4, 0x8b8)
                    } else {
                        (0x8d0, 0x8cc)
                    };
                    let v = wget(h, at).wrapping_add(1);
                    wset(h, at, v);
                    let v = wget(h, at);
                    if si(v) >= si(wget(h, n)) {
                        wset(h, at, 0);
                    }
                    v
                }
                0x97 | 0x98 => {
                    let (holder, height, proc_, title) = if param == 0x97 {
                        (0x40, 0x4d, MOTION_PROC, MOTION_TITLE)
                    } else {
                        (0x44, 0xc2, MODEL_PROC, MODEL_TITLE)
                    };
                    let (proc_, title) = (h.rebase(proc_), h.rebase(title));
                    cdecl(
                        h,
                        WINDOW_CREATE,
                        &[
                            w.wrapping_add(holder),
                            8,
                            8,
                            0x78,
                            height,
                            6,
                            proc_,
                            title,
                            window,
                        ],
                    )
                }
                _ => param.wrapping_sub(0x92),
            },
            0x800a => match param {
                0x92 => {
                    let v = h.read::<u32>(w.wrapping_add(0x8b0)).wrapping_sub(1);
                    h.write(w.wrapping_add(0x8b0), v);
                    if si(v) < 0 {
                        let n = wget(h, 0x824).wrapping_sub(1);
                        wset(h, 0x8b0, n);
                    }
                    cdecl(h, CHR_MOTION_SETUP, &[])
                }
                0x94 => {
                    h.write(w.wrapping_add(0x8bc), 0_u32);
                    let v = wget(h, 0x8c0).wrapping_sub(10);
                    wset(h, 0x8c0, v);
                    if si(wget(h, 0x8c0)) < 0 {
                        let n = wget(h, 0x8b8).wrapping_sub(1);
                        wset(h, 0x8c0, n);
                    }
                    let v = wget(h, 0x8c0);
                    wset(h, 0x8c4, v);
                    v
                }
                0x95 => {
                    h.write(w.wrapping_add(0x8bc), 1_u32);
                    let w2 = work(h);
                    let v = wget(h, 0x8c4).wrapping_sub(1);
                    h.write(w2.wrapping_add(0x8c4), v);
                    if si(v) >= 0 {
                        return Some(w2);
                    }
                    let n = wget(h, 0x8b8).wrapping_sub(1);
                    wset(h, 0x8c4, n);
                    n
                }
                0x96 => {
                    let v = h.read::<u32>(w.wrapping_add(0x8d0)).wrapping_sub(1);
                    h.write(w.wrapping_add(0x8d0), v);
                    if si(v) >= 0 {
                        return Some(4);
                    }
                    let n = wget(h, 0x8cc).wrapping_sub(1);
                    wset(h, 0x8d0, n);
                    n
                }
                _ => param.wrapping_sub(0x92),
            },
            0x8011 => {
                let buf = ebp.wrapping_sub(0x14);
                let d = h.rebase(FORMAT_D);
                let k: u32 = h.read(w.wrapping_add(0x8b0));
                let id: u32 = h.read(w.wrapping_add(0x830).wrapping_add(k.wrapping_mul(4)));
                sprintf(h, &[buf, d, id]);
                cdecl(h, SET_TEXT, &[window, 0x92, buf]);
                let mode = h.rebase(MODE_TEXTS[usize::from(wget(h, 0x8b4) != 0)]);
                cdecl(h, SET_TEXT, &[window, 0x93, mode]);
                cdecl(h, PRINT_SET_X, &[0x36]);
                cdecl(h, PRINT_SET_Y, &[0x1c]);
                let n = wget(h, 0x8b8);
                cdecl(h, GU_PRINTF, &[d, n]);
                let first = wget(h, 0x8c0);
                let n = wget(h, 0x8b8);
                let last = if si(first.wrapping_add(9)) < si(n) {
                    first.wrapping_add(9)
                } else {
                    n.wrapping_sub(1)
                };
                let range = h.rebase(RANGE_FORMAT);
                sprintf(h, &[buf, range, first, last]);
                cdecl(h, SET_TEXT, &[window, 0x94, buf]);
                let v = wget(h, 0x8c4);
                sprintf(h, &[buf, d, v]);
                cdecl(h, SET_TEXT, &[window, 0x95, buf]);
                cdecl(h, PRINT_SET_X, &[0x36]);
                cdecl(h, PRINT_SET_Y, &[0x43]);
                let n = wget(h, 0x8cc);
                cdecl(h, GU_PRINTF, &[d, n]);
                let v = wget(h, 0x8d0);
                sprintf(h, &[buf, d, v]);
                cdecl(h, SET_TEXT, &[window, 0x96, buf])
            }
            _ if i <= 0x10 => 5,
            _ => i,
        })
    }

    /// `g_RcuiWork + off`'s word.
    fn wget(h: &impl Host, off: u32) -> u32 {
        h.read(work(h).wrapping_add(off))
    }

    /// `g_RcuiWork + off` = `v`.
    fn wset(h: &mut impl Host, off: u32, v: u32) {
        let at = work(h).wrapping_add(off);
        h.write(at, v);
    }
}
