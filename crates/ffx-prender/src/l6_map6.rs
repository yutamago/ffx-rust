//! M5 stage 1, round 6 (list `left6`, from `map2.left`), batch 4: the
//! debug map tool's main menu window (`FUN_00935000`), which opens the
//! tool's other windows.
//!
//! Callees stay calls to the originals (cdecl). Its tail jumps become
//! calls (their targets read none of its arguments).

/// `g_RcuiWork`, `g_YnMapSystem`.
pub const RCUI_WORK: u32 = 0x0193_c0f0;
pub const YN_MAP_SYSTEM: u32 = 0x0193_bf80;

// ---- callees ----

/// `rcuiAddItems(window, items)`, `rcuiMakeBasePath()`,
/// `rcuiWindowCreate(holder, x, y, w, h, flags, proc, title, parent)`,
/// `rcuiLoadSetTitle()`, `Yn_FpMemWinOpen()`, the tool's shutdown
/// `0x00932e90()` (not in the dump), the print helpers.
pub const ADD_ITEMS: u32 = 0x0093_7780;
pub const MAKE_BASE_PATH: u32 = 0x0093_7af0;
pub const WINDOW_CREATE: u32 = 0x0093_7ef0;
pub const LOAD_SET_TITLE: u32 = 0x0093_4fb0;
pub const FP_MEM_WIN_OPEN: u32 = 0x0093_3c80;
pub const SHUTDOWN: u32 = 0x0093_2e90;
pub const PRINT_SET_X: u32 = 0x0084_8820;
pub const PRINT_SET_Y: u32 = 0x0084_8860;
pub const PRINT_SET_BOLD: u32 = 0x0084_8720;
pub const GU_PRINTF: u32 = 0x0084_8880;
/// The menu's items and its two resolution texts (high, low).
pub const MENU_ITEMS: u32 = 0x00c5_e8c0;
pub const HI_RES_TEXT: u32 = 0x00b6_84d8;
pub const LO_RES_TEXT: u32 = 0x00b6_84e0;

/// The windows the menu buttons 1..12 open: (holder offset in
/// `g_RcuiWork`, width, height, flags, procedure, title). Button 11 opens
/// the FP memory window through its own opener instead.
pub const WINDOWS: [(u32, u32, u32, u32, u32, u32); 12] = [
    (0x4, 0x154, 0xbe, 0xe, 0x0093_4bc0, 0x00b6_7e58),
    (0x8, 0x4c, 0x41, 6, 0x0093_64d0, 0x00b6_7e60),
    (0xc, 0x78, 0xb6, 6, 0x0093_4470, 0x00b6_7e68),
    (0x20, 0x90, 0xa8, 6, 0x0093_1910, 0x00b5_62f8),
    (0x24, 0x78, 0x50, 6, 0x0093_65c0, 0x00b6_7e70),
    (0x28, 0x78, 0x6a, 6, 0x0093_5710, 0x00b6_7e78),
    (0x30, 0x78, 0xb8, 6, 0x0093_3cc0, 0x00b6_7e80),
    (0x38, 0x78, 0x36, 6, 0x0093_4110, 0x00b6_84c0),
    (0x3c, 0x78, 0x91, 6, 0x0093_1e90, 0x00b6_84cc),
    (0x48, 0x78, 0x43, 6, 0x0093_6840, 0x00b5_a3f4),
    (0, 0, 0, 0, 0, 0),
    (0x54, 0x4c, 0xb5, 6, 0x0093_25c0, 0x00b6_7ea4),
];

pub mod bridge {
    use super::{
        ADD_ITEMS, FP_MEM_WIN_OPEN, GU_PRINTF, HI_RES_TEXT, LO_RES_TEXT, LOAD_SET_TITLE,
        MAKE_BASE_PATH, MENU_ITEMS, PRINT_SET_BOLD, PRINT_SET_X, PRINT_SET_Y, RCUI_WORK, SHUTDOWN,
        WINDOW_CREATE, WINDOWS, YN_MAP_SYSTEM,
    };
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    fn work(h: &impl Host) -> u32 {
        h.read(h.rebase(RCUI_WORK))
    }

    /// Opens menu window `k` (0-based) of [`WINDOWS`] under `window`.
    fn open(h: &mut impl Host, w: u32, k: usize, window: u32) -> u32 {
        let (holder, width, height, flags, proc_, title) = WINDOWS[k];
        let (proc_, title) = (h.rebase(proc_), h.rebase(title));
        cdecl(
            h,
            WINDOW_CREATE,
            &[
                w.wrapping_add(holder),
                8,
                8,
                width,
                height,
                flags,
                proc_,
                title,
                window,
            ],
        )
    }

    /// `FUN_00935000(window, msg, param)` -> `eax` as the original
    /// (`None`: no work area or system, the caller's `eax`): the map tool's
    /// main menu. 0x8001 adds its items, sets the text-save flag
    /// (`+0x64`) and makes the base path; 0x8003 (closed) clears its
    /// holder and shuts the tool down (`0x00932e90`); 0x8006 buttons 1..12
    /// open the windows ([`WINDOWS`]: 1 the load window, also clearing
    /// `+0x60`, the save flag and path, its title after; 2 and 5 only with
    /// a scene, `+0x5c`; 3 only with layers; 11 the FP memory window);
    /// 0x8011 shows the map's resolution and counts the frames (`+0x58`).
    pub fn menu_proc(h: &mut impl Host, window: u32, msg: u32, param: u32) -> Option<u32> {
        let w = work(h);
        if w == 0 {
            return None;
        }
        let sys: u32 = h.read(h.rebase(YN_MAP_SYSTEM));
        if sys == 0 {
            return None;
        }
        let i = msg.wrapping_sub(0x8001);
        Some(match msg {
            0x8001 => {
                let items = h.rebase(MENU_ITEMS);
                cdecl(h, ADD_ITEMS, &[window, items]);
                h.write(work(h).wrapping_add(0x64), 1_u32);
                cdecl(h, MAKE_BASE_PATH, &[])
            }
            0x8003 => {
                h.write(w, 0_u32);
                if work(h) == 0 {
                    return Some(1);
                }
                cdecl(h, SHUTDOWN, &[])
            }
            0x8006 => {
                let b = param.wrapping_sub(1);
                match b {
                    0 => {
                        h.write(w.wrapping_add(0x60), 0_u32);
                        h.write(work(h).wrapping_add(0x64), 1_u32);
                        cdecl(h, MAKE_BASE_PATH, &[]);
                        let w = work(h);
                        open(h, w, 0, window);
                        cdecl(h, LOAD_SET_TITLE, &[])
                    }
                    1 | 4 if h.read::<u32>(w.wrapping_add(0x5c)) == 0 => b,
                    2 => {
                        let map: u32 = h.read(sys);
                        if map == 0 {
                            return Some(0);
                        }
                        let list: u32 = h.read(map.wrapping_add(8));
                        if h.read::<u32>(list.wrapping_add(4)).cast_signed() <= 0 {
                            return Some(list);
                        }
                        open(h, w, 2, window)
                    }
                    10 => cdecl(h, FP_MEM_WIN_OPEN, &[]),
                    1 | 3..=9 | 11 => open(h, w, b as usize, window),
                    _ => b,
                }
            }
            0x8011 => {
                let map: u32 = h.read(sys);
                let mut w = w;
                let r = if map == 0 {
                    0
                } else {
                    let list: u32 = h.read(map.wrapping_add(8));
                    let hi: u32 = h.read(list.wrapping_add(0xf8));
                    cdecl(h, PRINT_SET_X, &[2]);
                    cdecl(h, PRINT_SET_Y, &[0xa0]);
                    cdecl(h, PRINT_SET_BOLD, &[hi]);
                    let text = h.rebase(if hi != 0 { HI_RES_TEXT } else { LO_RES_TEXT });
                    let r = cdecl(h, GU_PRINTF, &[text]);
                    w = work(h);
                    r
                };
                let n: u32 = h.read(w.wrapping_add(0x58));
                h.write(w.wrapping_add(0x58), n.wrapping_add(1));
                r
            }
            _ if i <= 0x10 => 4,
            _ => i,
        })
    }

    /// `rcuiScaleVector_00937610(dst, v, s)`, `rcuiVu0SubVector(dst, a, b)`,
    /// `rcVec3fNormalize(v)`, `rcuiVu0AddVector(dst, a, b)`,
    /// `Sg_IdGetPtr()`, `get_ptr32_func(p)`, `rcVec3fLength(v)` (`ST0`),
    /// `Sg_GetPlayer()`, `Ch_SetPos3(chr, x, y, z)`.
    pub const SCALE_VECTOR: u32 = 0x0093_7610;
    pub const SUB_VECTOR: u32 = 0x0093_76b0;
    pub const NORMALIZE: u32 = 0x0092_1360;
    pub const ADD_VECTOR: u32 = 0x0093_7540;
    pub const ID_GET_PTR: u32 = 0x0083_e970;
    pub const GET_PTR32: u32 = 0x00a4_4650;
    pub const LENGTH: u32 = 0x0092_1250;
    pub const GET_PLAYER: u32 = 0x0082_d860;
    pub const SET_POS3: u32 = 0x0082_b480;
    /// f64 5.0 (the distance ahead), f32 `FLT_MAX`, f32 1/3.
    pub const AHEAD: u32 = 0x00b4_1d20;
    pub const FAR: u32 = 0x00b1_7504;
    pub const THIRD: u32 = 0x00b2_c5c0;
    /// `rcuiResetChrPos`'s frame.
    pub const RESET_FRAME: u32 = 0x5c;

    /// `fld dword`: an `f32` as the x87 holds it.
    fn ld(bits: u32) -> f64 {
        ffx_bridge::x87::load(bits)
    }

    /// `fstp dword`.
    fn st(x: f64) -> u32 {
        ffx_bridge::x87::store(x)
    }

    /// The frame's two lowest words (`ebp - 0x5c`, `- 0x58`).
    fn low(h: &impl Host, ebp: u32) -> [u32; 2] {
        [
            h.read(ebp.wrapping_sub(0x5c)),
            h.read(ebp.wrapping_sub(0x58)),
        ]
    }

    /// An `i16` vertex coordinate through the frame as the original moves
    /// it: the `i32` at `ebp - 0x18`, `fild`, the `f64` at `ebp - 0x1c`.
    fn coord(h: &mut impl Host, ebp: u32, v: i16) -> f64 {
        let i = i32::from(v);
        h.write(ebp.wrapping_sub(0x18), i.cast_unsigned());
        let x = f64::from(i);
        h.write(ebp.wrapping_sub(0x1c), x.to_bits());
        x
    }

    /// `rcuiResetChrPos()` -> `Ch_SetPos3`'s `eax`: the player put in
    /// front of the camera on the scene's ground. The point 5 units ahead
    /// (`[[map + 8]]`'s camera: `+0x68` the eye, `+0x78` the target, both
    /// scaled by `+0xf4`); with the scene model (`Sg_IdGetPtr`) its
    /// nearest vertex (`+0x18` the `i16` vertices, `+0xc` their scale)
    /// and the first triangle of its groups (`+0x1c`, `i16 +6` of them)
    /// that uses it: the player goes to that triangle's centre, else to
    /// the point. `ebp` is the original's frame (the vectors, the
    /// counters and the coordinate scratch, left as it leaves them).
    #[allow(clippy::too_many_lines)]
    ///
    /// `rcuiScaleVector_00937610` takes the scale as a by-value vector of
    /// which the original pushes only x: its y, z, w are the words above
    /// the slot, the caller's saved `esi` and the frame's two lowest words
    /// (`ebp - 0x5c`, `- 0x58`) for the first three calls, the caller's
    /// saved `edi`, `ebx`, `esi` for the last (`regs`: `esi`, `ebx`,
    /// `edi`; junk read, the shim passes 0: brief 2026-10-24). The callee
    /// copies them into `0x00c8f974..`.
    pub fn reset_chr_pos(h: &mut impl Host, ebp: u32, regs: [u32; 3]) -> u32 {
        use ffx_bridge::x87;
        let fr = |off: u32| ebp.wrapping_sub(off);
        let sys: u32 = h.read(h.rebase(YN_MAP_SYSTEM));
        let list: u32 = if sys == 0 {
            0
        } else {
            let map: u32 = h.read(sys);
            if map == 0 {
                0
            } else {
                h.read(map.wrapping_add(8))
            }
        };
        let s = st(ld(h.read(list.wrapping_add(0xf4))));
        h.write(fr(0x20), s);
        let [l0, l1] = low(h, ebp);
        // junk read: the caller's esi and the frame's lowest words
        cdecl(
            h,
            SCALE_VECTOR,
            &[fr(0x5c), list.wrapping_add(0x78), s, regs[0], l0, l1],
        );
        let [l0, l1] = low(h, ebp);
        cdecl(
            h,
            SCALE_VECTOR,
            &[fr(0x30), list.wrapping_add(0x68), s, regs[0], l0, l1],
        );
        cdecl(h, SUB_VECTOR, &[fr(0x14), fr(0x5c), fr(0x30)]);
        cdecl(h, NORMALIZE, &[fr(0x14)]);
        let ahead = f64::from_bits(h.read(h.rebase(AHEAD)));
        let s2 = st(x87::mul(ld(s), ahead));
        h.write(fr(0x18), s2);
        let [l0, l1] = low(h, ebp);
        cdecl(h, SCALE_VECTOR, &[fr(0x4c), fr(0x14), s2, regs[0], l0, l1]);
        cdecl(h, ADD_VECTOR, &[fr(0x4c), fr(0x4c), fr(0x30)]);
        let m = cdecl(h, ID_GET_PTR, &[]);
        h.write(fr(0x38), m);
        if m != 0 {
            let n = i32::from(h.read::<i16>(m.wrapping_add(0xa)));
            h.write(fr(0x34), n.cast_unsigned());
            if n > 0 {
                nearest_triangle(h, ebp, m, n, s, regs);
            }
        }
        let chr = cdecl(h, GET_PLAYER, &[]);
        let p = [0x4c_u32, 0x48, 0x44].map(|o| st(ld(h.read(fr(o)))));
        cdecl(h, SET_POS3, &[chr, p[0], p[1], p[2]])
    }

    /// The vertex of `m` nearest the point (`ebp - 0x4c`), then the first
    /// triangle using it: its centre (a third of the corners' sum) into
    /// the point.
    #[allow(clippy::too_many_lines)]
    fn nearest_triangle(h: &mut impl Host, ebp: u32, m: u32, n: i32, s: u32, regs: [u32; 3]) {
        use ffx_bridge::x87;
        let fr = |off: u32| ebp.wrapping_sub(off);
        let verts = cdecl(h, GET_PTR32, &[h.read(m.wrapping_add(0x18))]);
        let unit = ld(h.read(m.wrapping_add(0xc)));
        let scale = st(x87::mul(x87::div(1.0, unit), ld(s)));
        h.write(fr(0x3c), scale);
        let far = st(ld(h.read(h.rebase(FAR))));
        h.write(fr(0x20), far);
        let sc = ld(scale);
        let mut best = 0_u32;
        let mut min = ld(far);
        for k in 0..n.cast_unsigned() {
            let v = verts.wrapping_add(8 * k);
            for (j, off) in [0x30_u32, 0x2c, 0x28].iter().enumerate() {
                let c = h.read::<i16>(v.wrapping_add(2 * j as u32));
                let x = coord(h, ebp, c);
                h.write(fr(*off), st(x87::mul(x, sc)));
            }
            cdecl(h, SUB_VECTOR, &[fr(0x14), fr(0x30), fr(0x4c)]);
            let d = h.call_sym_st0(LENGTH, Conv::Cdecl, &[fr(0x14)]).to_bits();
            h.write(fr(0x18), d);
            let dv = ld(d);
            if dv < min {
                min = dv;
                h.write(fr(0x20), d);
                best = k;
            }
        }
        let groups = i32::from(h.read::<i16>(m.wrapping_add(6)));
        h.write(fr(0x34), 0_u32);
        let mut g = 0_i32;
        while g < groups {
            let off = g.cast_unsigned().wrapping_mul(0x10);
            h.write(fr(0x20), off);
            let t = cdecl(h, GET_PTR32, &[h.read(m.wrapping_add(0x1c))]);
            let tris = cdecl(
                h,
                GET_PTR32,
                &[h.read(t.wrapping_add(off).wrapping_add(0xc))],
            );
            h.write(fr(0x18), tris);
            let t = cdecl(h, GET_PTR32, &[h.read(m.wrapping_add(0x1c))]);
            let mut count = i32::from(h.read::<i16>(t.wrapping_add(off).wrapping_add(8)));
            let mut j = 0_i32;
            while j < count {
                let tri = tris.wrapping_add(j.cast_unsigned().wrapping_mul(0x10));
                let mut hit = false;
                for q in 0..3_u32 {
                    if i32::from(h.read::<i16>(tri.wrapping_add(2 * q))) == best.cast_signed() {
                        hit = true;
                        break;
                    }
                }
                if hit {
                    let verts = cdecl(h, GET_PTR32, &[h.read(m.wrapping_add(0x18))]);
                    let mut p = [0_u32; 3];
                    for corner in 0..3_u32 {
                        let idx = i32::from(h.read::<i16>(tri.wrapping_add(2 * corner)));
                        let v = verts.wrapping_add(idx.cast_unsigned().wrapping_mul(8));
                        for (a, q) in p.iter_mut().enumerate() {
                            let c = h.read::<i16>(v.wrapping_add(2 * a as u32));
                            let x = x87::mul(coord(h, ebp, c), sc);
                            *q = if corner == 0 {
                                st(x)
                            } else {
                                st(x87::add(x, ld(*q)))
                            };
                            h.write(fr(0x30 - 4 * a as u32), *q);
                        }
                    }
                    let third = st(ld(h.read(h.rebase(THIRD))));
                    // junk read: the caller's saved edi, ebx, esi
                    let w = [fr(0x4c), fr(0x30), third, regs[2], regs[1], regs[0]];
                    cdecl(h, SCALE_VECTOR, &w);
                    return;
                }
                j += 1;
                let t = cdecl(h, GET_PTR32, &[h.read(m.wrapping_add(0x1c))]);
                count = i32::from(h.read::<i16>(t.wrapping_add(off).wrapping_add(8)));
            }
            g += 1;
            h.write(fr(0x34), g.cast_unsigned());
            h.write(fr(0x20), off.wrapping_add(0x10));
        }
    }

    /// The FP window's callees: `Fp_Restart(id)`, `Fp_End(id)`,
    /// `pppFpIDdispOn(id)`, `pppFpIDdispOff(id)`, `Fp_RestartMode2(id)`,
    /// `rcbgFpLoadTextCurrent_0093a7e0()`, `rcbgFpSaveCurrent_00938340()`,
    /// `pppFpDispDebug_thunk_00919b70()`, `Ch_OptposGetElement(chr, e, 0)`,
    /// `pppFpSetStickChr(id, chr, e)`, `Fp_RestartGroup(g)`,
    /// `pppFpIDfree_007295f0(id)`, `Fp_EndGroup(g)`, `Fp_IdState_0072a760(id)`,
    /// `Gu_WindowControlSetText(window, id, s)`.
    pub const FP_RESTART: u32 = 0x0072_9700;
    pub const FP_END: u32 = 0x0072_9660;
    pub const FP_DISP_ON: u32 = 0x0072_95b0;
    pub const FP_DISP_OFF: u32 = 0x0072_9570;
    pub const FP_RESTART_MODE2: u32 = 0x0072_9810;
    pub const FP_LOAD_TEXT: u32 = 0x0093_a7e0;
    pub const FP_SAVE: u32 = 0x0093_8340;
    pub const FP_DISP_DEBUG: u32 = 0x0091_9b70;
    pub const OPTPOS_GET_ELEMENT: u32 = 0x0083_5660;
    pub const FP_SET_STICK_CHR: u32 = 0x0072_a530;
    pub const FP_RESTART_GROUP: u32 = 0x0072_9530;
    pub const FP_FREE: u32 = 0x0072_95f0;
    pub const FP_END_GROUP: u32 = 0x0072_94f0;
    pub const FP_ID_STATE: u32 = 0x0072_a760;
    pub const SET_TEXT: u32 = 0x0084_a710;
    /// `g_pppFpDisp`; the FP window's items, the effect list window's
    /// procedure and title, the on/off texts, `"%d"`, `sprintf`.
    pub const FP_DISP: u32 = 0x00ce_c1f0;
    pub const FP_ITEMS: u32 = 0x00c5_f828;
    pub const FP_LIST_PROC: u32 = 0x0093_2f20;
    pub const FP_LIST_TITLE: u32 = 0x00b6_8618;
    pub const ON_TEXT: u32 = 0x00b4_0648;
    pub const OFF_TEXT: u32 = 0x00b4_0644;
    pub const FORMAT_D: u32 = 0x00b0_f604;
    pub const SPRINTF_SLOT: u32 = 0x00b0_c3dc;
    /// `0x00933cc0`'s frame.
    pub const FP_FRAME: u32 = 0x14;

    /// `FUN_00933cc0(window, msg, param)` -> `eax` as the original
    /// (`None`: no work area, the cookie's `eax`): the field effect (FP)
    /// test window over the effect number `g_RcuiWork + 0x5fc` (0..99).
    /// 0x8001 adds its items; 0x8003 (closed) clears `+0x30`; 0x8006
    /// buttons 0x6a/0x6b step the number up 1/10 (wrapping), 0x6c..0x70
    /// restart, end, show, hide, restart in mode 2, 0x72 toggles the
    /// display, 0x73 reloads and saves the effect text, 0x74 opens the
    /// effect list, 0x75 the debug display, 0x76 sticks the effect to the
    /// selected character (an element of it, `+0x8b4`) and restarts it,
    /// 0x77 restarts its group; 0x800a steps down, frees, ends the group;
    /// 0x8011 the number, its state and the display texts. `ebp` is the
    /// original's frame (the texts at `ebp - 0x14`).
    #[allow(clippy::too_many_lines)]
    pub fn fp_proc(h: &mut impl Host, ebp: u32, window: u32, msg: u32, param: u32) -> Option<u32> {
        let w = work(h);
        if w == 0 {
            return None;
        }
        let num = w.wrapping_add(0x5fc);
        let id16 = u32::from(h.read::<u16>(num));
        let j = param.wrapping_sub(0x6a);
        let i = msg.wrapping_sub(0x8001);
        Some(match msg {
            0x8001 => {
                let items = h.rebase(FP_ITEMS);
                cdecl(h, ADD_ITEMS, &[window, items])
            }
            0x8003 => {
                h.write(w.wrapping_add(0x30), 0_u32);
                1
            }
            0x8006 => match param {
                0x6a => {
                    let v = h.read::<u32>(num).wrapping_add(1);
                    h.write(num, v);
                    if v.cast_signed() <= 0x63 {
                        return Some(0);
                    }
                    let w = work(h);
                    h.write(w.wrapping_add(0x5fc), 0_u32);
                    w
                }
                0x6b => {
                    let v = h.read::<u32>(num).wrapping_add(10);
                    h.write(num, v);
                    let at = work(h).wrapping_add(0x5fc);
                    let v: u32 = h.read(at);
                    if v.cast_signed() <= 0x63 {
                        return Some(v);
                    }
                    h.write(at, v.wrapping_sub(100));
                    v.wrapping_sub(100)
                }
                0x6c..=0x70 => {
                    let f = [
                        FP_RESTART,
                        FP_END,
                        FP_DISP_ON,
                        FP_DISP_OFF,
                        FP_RESTART_MODE2,
                    ][(param - 0x6c) as usize];
                    cdecl(h, f, &[id16])
                }
                0x71 => 7,
                0x72 => {
                    let at = h.rebase(FP_DISP);
                    let v: u32 = h.read(at);
                    h.write(at, v ^ 1);
                    8
                }
                0x73 => {
                    cdecl(h, FP_LOAD_TEXT, &[]);
                    cdecl(h, FP_SAVE, &[])
                }
                0x74 => {
                    let (proc_, title) = (h.rebase(FP_LIST_PROC), h.rebase(FP_LIST_TITLE));
                    cdecl(
                        h,
                        WINDOW_CREATE,
                        &[
                            w.wrapping_add(0x34),
                            8,
                            8,
                            0x78,
                            0xc5,
                            6,
                            proc_,
                            title,
                            window,
                        ],
                    )
                }
                0x75 => cdecl(h, FP_DISP_DEBUG, &[]),
                0x76 => {
                    let sel: u32 = h.read(w.wrapping_add(0x8b0));
                    let chr: u32 = h.read(w.wrapping_add(0x870).wrapping_add(sel.wrapping_mul(4)));
                    let (e, w) = if h.read::<u32>(w.wrapping_add(0x8b4)) == 0 {
                        (h.read(w.wrapping_add(0x8c4)), w)
                    } else {
                        let k: u32 = h.read(w.wrapping_add(0x8d0));
                        let e = cdecl(h, OPTPOS_GET_ELEMENT, &[chr, k, 0]);
                        (e, work(h))
                    };
                    let v: u32 = h.read(w.wrapping_add(0x5fc));
                    cdecl(h, FP_SET_STICK_CHR, &[v, chr, e]);
                    let v = u32::from(h.read::<u16>(work(h).wrapping_add(0x5fc)));
                    cdecl(h, FP_RESTART, &[v])
                }
                0x77 => cdecl(h, FP_RESTART_GROUP, &[id16]),
                _ => j,
            },
            0x800a => match param {
                0x6a => {
                    let v = h.read::<u32>(num).wrapping_sub(1);
                    h.write(num, v);
                    if v.cast_signed() >= 0 {
                        return Some(0);
                    }
                    let w = work(h);
                    h.write(w.wrapping_add(0x5fc), 0x63_u32);
                    w
                }
                0x6b => {
                    let v = h.read::<u32>(num).wrapping_sub(10);
                    h.write(num, v);
                    let at = work(h).wrapping_add(0x5fc);
                    let v: u32 = h.read(at);
                    if v.cast_signed() >= 0 {
                        return Some(v);
                    }
                    h.write(at, v.wrapping_add(100));
                    v.wrapping_add(100)
                }
                0x6d | 0x76 => cdecl(h, FP_FREE, &[id16]),
                0x77 => cdecl(h, FP_END_GROUP, &[id16]),
                _ if j <= 0xd => 4,
                _ => j,
            },
            0x8011 => {
                let buf = ebp.wrapping_sub(0x14);
                let fmt = h.rebase(FORMAT_D);
                let f: u32 = h.read(h.rebase(SPRINTF_SLOT));
                let v: u32 = h.read(num);
                h.call(f, Conv::Cdecl, &[buf, fmt, v]);
                cdecl(h, SET_TEXT, &[window, 0x6a, buf]);
                let v = u32::from(h.read::<u16>(work(h).wrapping_add(0x5fc)));
                let st = cdecl(h, FP_ID_STATE, &[v]);
                h.call(f, Conv::Cdecl, &[buf, fmt, st]);
                cdecl(h, SET_TEXT, &[window, 0x71, buf]);
                let on = h.read::<u32>(h.rebase(FP_DISP)) != 0;
                let text = h.rebase(if on { ON_TEXT } else { OFF_TEXT });
                cdecl(h, SET_TEXT, &[window, 0x72, text])
            }
            _ if i <= 0x10 => 5,
            _ => i,
        })
    }
}
