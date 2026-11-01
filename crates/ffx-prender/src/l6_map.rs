//! M5 stage 1, round 6 (list `left6`, from `map2.left`): pieces of the
//! debug map tool (`rcui*`, `rcbg*`) earlier rounds left for time: three
//! window procedures (the save, the point-simulation and the guide-target
//! windows), the palette window's procedure, the character selection's
//! reset, and the guide map's projected point lists and axis marker.
//!
//! None of it ran in a coverage run. Callees stay calls to the originals,
//! cdecl as their `ret` says. Stack cookies are left out; where the
//! original hands a callee a frame buffer, the bridge takes the original's
//! `ebp` (the shim passes a `StackFrame`).
//!
//! The window procedures return `Option<u32>`: `None` where the original
//! leaves the `eax` its caller had (or the stack cookie's `eax`), which no
//! dispatcher reads (Gu's window dispatch sites `0x00849e66`, `0x00849eac`,
//! `0x0084ab5e`, `0x0084ab97`, ... overwrite or drop `eax` after the call).
//! Jump-table paths that touch no register leave the table's index byte in
//! `eax`; that is what they return.

use ffx_bridge::Global;

/// `g_RcuiWork`, `g_YnMapSystem`.
pub const RCUI_WORK: Global<u32> = Global::new(0x0193_c0f0);
pub const YN_MAP_SYSTEM: Global<u32> = Global::new(0x0193_bf80);

// ---- callees ----

/// `memset` (the CRT thunk, cdecl).
pub const MEMSET: u32 = 0x0094_964c;
/// `rcefIsBattle()`, `rcuiCollectChrHandlesByGroup()`,
/// `rcuiCollectChrHandles()`, `rcuiChrMotionSetup()`.
pub const RCEF_IS_BATTLE: u32 = 0x0092_e620;
pub const COLLECT_BY_GROUP: u32 = 0x0093_23b0;
pub const COLLECT: u32 = 0x0093_2570;
pub const CHR_MOTION_SETUP: u32 = 0x0093_2470;
/// `rcuiAddItems(window, items)`, `rcuiCloseWindow(holder)`.
pub const ADD_ITEMS: u32 = 0x0093_7780;
pub const CLOSE_WINDOW: u32 = 0x0093_7f50;
/// `rcuiMakeBasePath()`, `rcuiMakeFullPath()`, `rcbgSaveBin_00938fd0()`,
/// `rcbgSaveText()`.
pub const MAKE_BASE_PATH: u32 = 0x0093_7af0;
pub const MAKE_FULL_PATH: u32 = 0x0093_7b20;
pub const SAVE_BIN: u32 = 0x0093_8fd0;
pub const SAVE_TEXT: u32 = 0x0093_9b10;
/// The item lists the windows add on 0x8001.
pub const SAVE_ITEMS: u32 = 0x00c5_ea30;
pub const PSIM_ITEMS: u32 = 0x00c5_f6f0;
pub const GUIDE_ITEMS: u32 = 0x00c5_fec0;
pub const PALETTE_ITEMS: u32 = 0x00c5_f000;
/// `Yn_PSimGetCnt()`, `EmptyStub_00919f30()`, `Yn_PSimGetHandle(n)`,
/// `Yn_PSimSetPos(p, which, v)`.
pub const PSIM_GET_CNT: u32 = 0x0091_9ef0;
pub const EMPTY_STUB: u32 = 0x0091_9f30;
pub const PSIM_GET_HANDLE: u32 = 0x0091_9f00;
pub const PSIM_SET_POS: u32 = 0x0091_9f10;
/// The point-simulation ends' z: 15.0 and -15.0 (f32 constants).
pub const PSIM_Z_PLUS: u32 = 0x00b5_99e8;
pub const PSIM_Z_MINUS: u32 = 0x00b5_99f8;
/// `rcbgGuideGetTarget(g)`, `rcbgGuideSetTarget(g, t)`,
/// `rcbgGuideMapConvCurrent_00938570()`.
pub const GUIDE_GET_TARGET: u32 = 0x0091_d480;
pub const GUIDE_SET_TARGET: u32 = 0x0091_d930;
pub const GUIDE_CONV_CURRENT: u32 = 0x0093_8570;
/// `Gu_WindowControlSetText(window, id, s)`, `Gu_PrintSetX(x)`,
/// `Gu_PrintSetY(y)`, `Gu_Printf(fmt, ...)`.
pub const SET_TEXT: u32 = 0x0084_a710;
pub const PRINT_SET_X: u32 = 0x0084_8820;
pub const PRINT_SET_Y: u32 = 0x0084_8860;
pub const GU_PRINTF: u32 = 0x0084_8880;
/// `MSVCR110` `sprintf` slot; `"%d"`.
pub const SPRINTF_SLOT: u32 = 0x00b0_c3dc;
pub const FORMAT_D: u32 = 0x00b0_f604;
/// The palette window's mode names (0, 1, 2) and its `"%02d"`.
pub const PALETTE_MODE_TEXTS: [u32; 3] = [0x00b8_f870, 0x00b5_5f70, 0x00b8_f998];
pub const PALETTE_FORMAT: u32 = 0x00b4_1ec0;
/// `FFXVu0MulMatrix(dst, a, b)`, `FFXVu0ApplyMatrix(dst, m, v)`,
/// `FFXVu0DivVectorXYZ(dst, v, s)`, `Ch_SysGetScreenMatrixPtr(v)` (the
/// point's clip flags here), `rcCopyQword_00927a90(dst, src)`,
/// `rcVectorToFixed_00927ac0(dst, v)`.
pub const MUL_MATRIX: u32 = 0x006e_d5c0;
pub const APPLY_MATRIX: u32 = 0x006e_d2e0;
pub const DIV_VECTOR_XYZ: u32 = 0x006e_d420;
pub const SCREEN_CLIP: u32 = 0x0062_fc90;
pub const COPY_QWORD: u32 = 0x0092_7a90;
pub const TO_FIXED: u32 = 0x0092_7ac0;
/// `g_CameraMatrix`, `g_ScreenMatrix`.
pub const CAMERA_MATRIX: u32 = 0x0231_1440;
pub const SCREEN_MATRIX: u32 = 0x0231_1480;
/// `VFX_MulXByValue_0092cc40(dst, src, v)` (`v` a 16-byte vector by
/// value), `0x009278d0` (this round), `rcCalcPersMatrix(dst, m)`,
/// `VFX_CopyC0a004_0092cc20(dst)`, `rcPacketCircle_00927720(pkt, x, y, r,
/// rgba)`.
pub const MUL_X: u32 = 0x0092_cc40;
pub const PACK_POINTS: u32 = 0x0092_78d0;
pub const CALC_PERS_MATRIX: u32 = 0x0092_0020;
pub const COPY_C0A004: u32 = 0x0092_cc20;
pub const PACKET_CIRCLE: u32 = 0x0092_7720;
/// The axis marker's colours (the three lines' and the circle's) and its
/// three 3-point lines.
pub const AXIS_COLOURS: [u32; 4] = [0x00c5_e7a8, 0x00c5_e7b8, 0x00c5_e7d8, 0x00c5_e808];
pub const AXIS_LINES: [u32; 3] = [0x00c5_e818, 0x00c5_e848, 0x00c5_e878];

/// `0x009278d0`'s frame (`ebp` minus its lowest local).
pub const PACK_FRAME: u32 = 0x6c;
/// `0x0092c600`'s frame.
pub const AXIS_FRAME: u32 = 0x2c;
/// `0x00935710`'s and `0x00934110`'s frames.
pub const PSIM_FRAME: u32 = 0x14;
pub const GUIDE_FRAME: u32 = 0x14;

pub mod bridge {
    use super::{
        ADD_ITEMS, APPLY_MATRIX, AXIS_COLOURS, AXIS_LINES, CALC_PERS_MATRIX, CAMERA_MATRIX,
        CHR_MOTION_SETUP, CLOSE_WINDOW, COLLECT, COLLECT_BY_GROUP, COPY_C0A004, COPY_QWORD,
        DIV_VECTOR_XYZ, EMPTY_STUB, FORMAT_D, GU_PRINTF, GUIDE_CONV_CURRENT, GUIDE_GET_TARGET,
        GUIDE_ITEMS, GUIDE_SET_TARGET, MAKE_BASE_PATH, MAKE_FULL_PATH, MEMSET, MUL_MATRIX, MUL_X,
        PACK_POINTS, PACKET_CIRCLE, PALETTE_FORMAT, PALETTE_ITEMS, PALETTE_MODE_TEXTS, PRINT_SET_X,
        PRINT_SET_Y, PSIM_GET_CNT, PSIM_GET_HANDLE, PSIM_ITEMS, PSIM_SET_POS, PSIM_Z_MINUS,
        PSIM_Z_PLUS, RCEF_IS_BATTLE, RCUI_WORK, SAVE_BIN, SAVE_ITEMS, SAVE_TEXT, SCREEN_CLIP,
        SCREEN_MATRIX, SET_TEXT, SPRINTF_SLOT, TO_FIXED, YN_MAP_SYSTEM,
    };
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, Conv::Cdecl, args)
    }

    /// An `f32` copied through the x87 (`fld dword`, `fstp dword`).
    fn copy_f32(bits: u32) -> u32 {
        x87::store(x87::load(bits))
    }

    /// `fild dword` then `fstp dword`: an `i32` rounded to `f32`.
    fn int_to_f32(v: u32) -> u32 {
        x87::store(f64::from(v.cast_signed()))
    }

    /// `FUN_00932320()` -> `rcuiChrMotionSetup`'s `eax` (a tail jump): the
    /// character selection reset: `g_RcuiWork + 0x824/0x828/0x82c` cleared,
    /// the 16 handles at `+0x870` set to -1, `+0x8b0` (the selected one)
    /// and `+0x8c4` cleared; the handles collected by group in a battle
    /// (`rcefIsBattle`), else all; then the motion setup.
    pub fn chr_select_reset(h: &mut impl Host) -> u32 {
        for off in [0x82c_u32, 0x828, 0x824] {
            let w = RCUI_WORK.get(h);
            h.write(w.wrapping_add(off), 0_u32);
        }
        let w = RCUI_WORK.get(h);
        cdecl(h, MEMSET, &[w.wrapping_add(0x870), 0xff, 0x40]);
        for off in [0x8b0_u32, 0x8c4] {
            let w = RCUI_WORK.get(h);
            h.write(w.wrapping_add(off), 0_u32);
        }
        if cdecl(h, RCEF_IS_BATTLE, &[]) != 0 {
            cdecl(h, COLLECT_BY_GROUP, &[]);
        } else {
            cdecl(h, COLLECT, &[]);
        }
        cdecl(h, CHR_MOTION_SETUP, &[])
    }

    /// `FUN_009364d0(window, msg, param)`: the save window's procedure.
    /// Needs `g_RcuiWork` (else `None`) and a map (`g_YnMapSystem`, its
    /// `+0`: `eax` is that word's holder or 0 without). 0x8001 adds its
    /// items; 0x8003 (closed) clears the holder `+8`; 0x8006 (a button,
    /// `param`): ids 0xd..0x10 set the text flag `+0x64` (0 for 0xd and
    /// 0xf), make the paths and save binary (0xf, 0x10) or text (0xd,
    /// 0xe); any non-zero id closes the window (`rcuiCloseWindow(+8)`,
    /// with `g_RcuiWork` read again after a save). Other messages leave
    /// `msg - 0x8006` in `eax`.
    pub fn save_proc(h: &mut impl Host, window: u32, msg: u32, param: u32) -> Option<u32> {
        let w = RCUI_WORK.get(h);
        if w == 0 {
            return None;
        }
        let sys = YN_MAP_SYSTEM.get(h);
        if sys == 0 {
            return Some(0);
        }
        if h.read::<u32>(sys) == 0 {
            return Some(sys);
        }
        Some(match msg {
            0x8001 => {
                let items = h.rebase(SAVE_ITEMS);
                cdecl(h, ADD_ITEMS, &[window, items])
            }
            0x8003 => {
                h.write(w.wrapping_add(8), 0_u32);
                0
            }
            0x8006 => {
                if param == 0 {
                    return Some(0);
                }
                let mut w = w;
                if param.wrapping_sub(0xd) <= 3 {
                    let flag = u32::from(param != 0xd && param != 0xf);
                    h.write(w.wrapping_add(0x64), flag);
                    cdecl(h, MAKE_BASE_PATH, &[]);
                    cdecl(h, MAKE_FULL_PATH, &[]);
                    if param.cast_signed() >= 0xf {
                        cdecl(h, SAVE_BIN, &[]);
                    } else {
                        cdecl(h, SAVE_TEXT, &[]);
                    }
                    w = RCUI_WORK.get(h);
                }
                cdecl(h, CLOSE_WINDOW, &[w.wrapping_add(8)])
            }
            _ => msg.wrapping_sub(0x8006),
        })
    }

    /// `FUN_00935710(window, msg)`: the point-simulation window's
    /// procedure (`ebp` the original's frame: the position vector at
    /// `ebp - 0x14`). Needs `g_RcuiWork` (else `None`: the cookie's
    /// `eax`). 0x8001 adds its items and, with no simulation
    /// (`Yn_PSimGetCnt` 0), runs `EmptyStub_00919f30`; 0x8003 (closed)
    /// clears `+0x28`; 0x8011, with a simulation: its two ends (handle 0)
    /// at (0, 0, -15) and (0, 0, 15) (`Yn_PSimSetPos`). Messages
    /// 0x8001..0x8011 the table sends nowhere leave its index byte (2 for
    /// 0x8006, else 4), others `msg - 0x8001`.
    pub fn psim_proc(h: &mut impl Host, ebp: u32, window: u32, msg: u32) -> Option<u32> {
        let w = RCUI_WORK.get(h);
        if w == 0 {
            return None;
        }
        let i = msg.wrapping_sub(0x8001);
        Some(match msg {
            0x8001 => {
                let items = h.rebase(PSIM_ITEMS);
                cdecl(h, ADD_ITEMS, &[window, items]);
                let n = cdecl(h, PSIM_GET_CNT, &[]);
                if n != 0 {
                    return Some(n);
                }
                cdecl(h, EMPTY_STUB, &[])
            }
            0x8003 => {
                h.write(w.wrapping_add(0x28), 0_u32);
                1
            }
            0x8011 => {
                let n = cdecl(h, PSIM_GET_CNT, &[]);
                if n == 0 {
                    return Some(0);
                }
                let p = cdecl(h, PSIM_GET_HANDLE, &[0]);
                let v = ebp.wrapping_sub(0x14);
                let mut r = 0;
                for (which, z) in [(0_u32, PSIM_Z_MINUS), (1, PSIM_Z_PLUS)] {
                    h.write(v.wrapping_add(4), 0_u32);
                    h.write(v, 0_u32);
                    let z: u32 = h.read(h.rebase(z));
                    h.write(v.wrapping_add(8), copy_f32(z));
                    r = cdecl(h, PSIM_SET_POS, &[p, which, v]);
                }
                r
            }
            0x8006 => 2,
            _ if i <= 0x10 => 4,
            _ => i,
        })
    }

    /// `FUN_00934110(window, msg, param)`: the guide-target window's
    /// procedure (`ebp` the original's frame: the text at `ebp - 0x14`).
    /// Needs `g_RcuiWork` and a guide (`g_YnMapSystem` `+4`; else `None`:
    /// the cookie's `eax`). 0x8001 adds its items; 0x8003 (closed) clears
    /// `+0x38`; 0x8006: button 0x8f steps the guide's target (0..4,
    /// wrapping to 0), 0x91 converts the current map's guide map; 0x8011
    /// writes the target (`"%d"`) into control 0x8f.
    pub fn guide_proc(
        h: &mut impl Host,
        ebp: u32,
        window: u32,
        msg: u32,
        param: u32,
    ) -> Option<u32> {
        let w = RCUI_WORK.get(h);
        if w == 0 {
            return None;
        }
        let sys = YN_MAP_SYSTEM.get(h);
        if sys == 0 {
            return None;
        }
        let g: u32 = h.read(sys.wrapping_add(4));
        if g == 0 {
            return None;
        }
        let i = msg.wrapping_sub(0x8001);
        Some(match msg {
            0x8001 => {
                let items = h.rebase(GUIDE_ITEMS);
                cdecl(h, ADD_ITEMS, &[window, items])
            }
            0x8003 => {
                h.write(w.wrapping_add(0x38), 0_u32);
                1
            }
            0x8006 => match param {
                0x8f => {
                    let t = cdecl(h, GUIDE_GET_TARGET, &[g]).wrapping_add(1);
                    let t = if t.cast_signed() >= 5 { 0 } else { t };
                    cdecl(h, GUIDE_SET_TARGET, &[g, t])
                }
                0x91 => cdecl(h, GUIDE_CONV_CURRENT, &[]),
                _ => param.wrapping_sub(0x91),
            },
            0x8011 => {
                let t = cdecl(h, GUIDE_GET_TARGET, &[g]);
                let buf = ebp.wrapping_sub(0x14);
                let fmt = h.rebase(FORMAT_D);
                import(h, SPRINTF_SLOT, &[buf, fmt, t]);
                cdecl(h, SET_TEXT, &[window, 0x8f, buf])
            }
            _ if i <= 0x10 => 4,
            _ => i,
        })
    }

    /// `rcuiPaletteProc(window, msg, param)`: the palette window's
    /// procedure. Needs `g_RcuiWork` (else `None`: the caller's `eax`) and
    /// a map (`eax` its holder or 0 without). The entry is the map's
    /// palette list (`[[map + 8] + 8] + 0xb0`, 0xc0 bytes each) at
    /// `g_RcuiWork + 0x288`. 0x8001 adds its items; 0x8003 (closed)
    /// clears `+0x14`; 0x8006: button 0x33 cycles the mode (`+0xa`, 0..2),
    /// 0x34/0x35 step the number (`+0xb`, 0..99); 0x8011 writes the mode's
    /// name into control 0x33 (modes past 2 none) and prints the number
    /// at (0x47, 0xe).
    pub fn palette_proc(h: &mut impl Host, window: u32, msg: u32, param: u32) -> Option<u32> {
        let w = RCUI_WORK.get(h);
        if w == 0 {
            return None;
        }
        let sys = YN_MAP_SYSTEM.get(h);
        if sys == 0 {
            return Some(0);
        }
        let map: u32 = h.read(sys);
        if map == 0 {
            return Some(sys);
        }
        let list: u32 = h.read(map.wrapping_add(8));
        let base: u32 = h.read(list.wrapping_add(8));
        let k: u32 = h.read(w.wrapping_add(0x288));
        let ent = base
            .wrapping_add(0xb0)
            .wrapping_add(k.wrapping_mul(3).wrapping_shl(6));
        let i = msg.wrapping_sub(0x8001);
        Some(match msg {
            0x8001 => {
                let items = h.rebase(PALETTE_ITEMS);
                cdecl(h, ADD_ITEMS, &[window, items])
            }
            0x8003 => {
                h.write(w.wrapping_add(0x14), 0_u32);
                1
            }
            0x8006 => {
                if ent == 0 {
                    return Some(2);
                }
                match param {
                    0x33 => {
                        let m = h.read::<u8>(ent.wrapping_add(0xa)).wrapping_add(1);
                        h.write(ent.wrapping_add(0xa), m);
                        if m >= 3 {
                            h.write(ent.wrapping_add(0xa), 0_u8);
                        }
                        0
                    }
                    0x34 => {
                        let n = h.read::<u8>(ent.wrapping_add(0xb));
                        if n == 0 {
                            return Some(0);
                        }
                        h.write(ent.wrapping_add(0xb), n - 1);
                        u32::from(n - 1)
                    }
                    0x35 => {
                        let n = h.read::<u8>(ent.wrapping_add(0xb));
                        if n >= 0x63 {
                            return Some(u32::from(n));
                        }
                        h.write(ent.wrapping_add(0xb), n + 1);
                        u32::from(n + 1)
                    }
                    _ => param.wrapping_sub(0x35),
                }
            }
            0x8011 => {
                if ent == 0 {
                    return Some(3);
                }
                let mode = h.read::<u8>(ent.wrapping_add(0xa));
                if let Some(&text) = PALETTE_MODE_TEXTS.get(usize::from(mode)) {
                    let text = h.rebase(text);
                    cdecl(h, SET_TEXT, &[window, 0x33, text]);
                }
                cdecl(h, PRINT_SET_X, &[0x47]);
                cdecl(h, PRINT_SET_Y, &[0xe]);
                let n = u32::from(h.read::<u8>(ent.wrapping_add(0xb)));
                let fmt = h.rebase(PALETTE_FORMAT);
                cdecl(h, GU_PRINTF, &[fmt, n])
            }
            _ if i <= 0x10 => 4,
            _ => i,
        })
    }

    /// `FUN_009278d0(pkt, points, n, colour, matrix)` -> `pkt`: a guide map
    /// point list into the packet at `pkt + 4` (advanced past it). The
    /// matrix is `screen * (camera * matrix)` (`ebp - 0x54`); the packet
    /// starts with the colour's four floats as integers (`ftol`), then
    /// one 16-byte entry per point: the point (w 1) through the matrix,
    /// divided by its w, and with the clip flags of it and the point
    /// before (`Ch_SysGetScreenMatrixPtr`, low 2 bits) both clear its
    /// fixed-point x and y (`rcVectorToFixed_00927ac0`, `ebp - 0x6c`) and
    /// zeros, else 0x8000 in its `+0xc` (the rest left).
    pub fn pack_points(
        h: &mut impl Host,
        ebp: u32,
        pkt: u32,
        points: u32,
        n: u32,
        colour: u32,
        matrix: u32,
    ) -> u32 {
        let m = ebp.wrapping_sub(0x54);
        let v = ebp.wrapping_sub(0x14);
        let fixed = ebp.wrapping_sub(0x6c);
        let mut out: u32 = h.read(pkt.wrapping_add(4));
        h.write(ebp.wrapping_sub(0x5c), pkt);
        let camera = h.rebase(CAMERA_MATRIX);
        cdecl(h, MUL_MATRIX, &[m, camera, matrix]);
        let screen = h.rebase(SCREEN_MATRIX);
        cdecl(h, MUL_MATRIX, &[m, screen, m]);
        for k in 0..4_u32 {
            let c: u32 = h.read(colour.wrapping_add(4 * k));
            let i = x87::ftol(h, x87::load(c));
            h.write(out.wrapping_add(4 * k), i);
        }
        out = out.wrapping_add(0x10);
        let mut flags = 0_u32;
        h.write(ebp.wrapping_sub(0x58), flags);
        let mut p = points;
        for _ in 0..n.cast_signed().max(0) {
            cdecl(h, COPY_QWORD, &[v, p]);
            h.write(v.wrapping_add(0xc), 1.0_f32.to_bits());
            cdecl(h, APPLY_MATRIX, &[v, m, v]);
            let s = copy_f32(h.read(v.wrapping_add(0xc)));
            cdecl(h, DIV_VECTOR_XYZ, &[v, v, s]);
            let clip = cdecl(h, SCREEN_CLIP, &[v]);
            flags = (clip | flags.wrapping_add(flags)) & 3;
            h.write(ebp.wrapping_sub(0x58), flags);
            if flags == 0 {
                cdecl(h, TO_FIXED, &[fixed, v]);
                let x: u32 = h.read(fixed);
                h.write(out, x);
                let y: u32 = h.read(fixed.wrapping_add(4));
                h.write(out.wrapping_add(4), y);
                h.write(out.wrapping_add(8), 0_u32);
                h.write(out.wrapping_add(0xc), 0_u32);
            } else {
                h.write(out.wrapping_add(0xc), 0x8000_u32);
            }
            out = out.wrapping_add(0x10);
            p = p.wrapping_add(0x10);
        }
        h.write(pkt.wrapping_add(4), out);
        pkt
    }

    /// `FUN_0092c600(pkt, matrix, size)` -> `rcPacketCircle`'s `eax`: the
    /// guide map's axis marker: three 3-point lines (`0x009278d0`) in
    /// colours scaled by `size` (`VFX_MulXByValue_0092cc40`, the scale
    /// `f32(size)`), then the perspective matrix (`rcCalcPersMatrix`) and a
    /// circle of radius `g_RcuiWork + 0x58` mod 8 at the projected origin
    /// (`VFX_CopyC0a004_0092cc20`, divided by its w), in the fourth colour.
    /// `ebp` is the original's frame (the colour at `ebp - 0x14`, the
    /// origin at `- 0x24`).
    ///
    /// The scale goes by value as a 16-byte vector of which the original
    /// pushes only x: its y, z and w are the words above, the caller's
    /// `edi`, `esi` and `ebx` the original saved there (`junk`, in that
    /// order; the shim passes 0: brief 2026-10-24). The callee copies them
    /// into `0x00c8f970 + 4..`; its result uses only x.
    pub fn axis_marker(
        h: &mut impl Host,
        ebp: u32,
        pkt: u32,
        matrix: u32,
        size: u32,
        junk: [u32; 3],
    ) -> u32 {
        let colour = ebp.wrapping_sub(0x14);
        let origin = ebp.wrapping_sub(0x24);
        let scale = copy_f32(int_to_f32(size));
        let w = RCUI_WORK.get(h);
        for (c, line) in AXIS_COLOURS.iter().zip(AXIS_LINES) {
            let c = h.rebase(*c);
            // junk read: the vector's y, z, w are the caller's saved
            // edi, esi, ebx; 0 in the shim (brief 2026-10-24)
            cdecl(h, MUL_X, &[colour, c, scale, junk[0], junk[1], junk[2]]);
            let line = h.rebase(line);
            cdecl(h, PACK_POINTS, &[pkt, line, 3, colour, matrix]);
        }
        cdecl(h, CALC_PERS_MATRIX, &[matrix, matrix]);
        cdecl(h, COPY_C0A004, &[origin]);
        cdecl(h, APPLY_MATRIX, &[origin, matrix, origin]);
        let ow = copy_f32(h.read(origin.wrapping_add(0xc)));
        cdecl(h, DIV_VECTOR_XYZ, &[origin, origin, ow]);
        let k: u32 = h.read(w.wrapping_add(0x58));
        let r = int_to_f32((k.cast_signed() % 8).cast_unsigned());
        let c = h.rebase(AXIS_COLOURS[3]);
        // junk read: as above; 0 in the shim (brief 2026-10-24)
        cdecl(h, MUL_X, &[colour, c, scale, junk[0], junk[1], junk[2]]);
        let x = copy_f32(h.read(origin));
        let y = copy_f32(h.read(origin.wrapping_add(4)));
        cdecl(h, PACKET_CIRCLE, &[pkt, x, y, copy_f32(r), colour])
    }
}
