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
}
