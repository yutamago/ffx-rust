//! M5 round 11 (`tail11`): `Ms_Gui_BattleProc` (`0x007d80a0`, cdecl
//! `(window, msg, param)` -> 0), the battle debug `BattlePos` window: the
//! position table's ground groups, kinds and picks, the picked position
//! moved by the buttons or the pad, the position files saved and loaded,
//! and its printout.
//!
//! x87: every value is computed at 53 bits in the original's order and
//! rounded where the original stores a `dword` (`ffx_bridge::x87`).

/// The original as a bridge over [`ffx_graphic::host::Host`].
#[allow(clippy::many_single_char_names, clippy::cast_possible_truncation)] // x87 terms and host handles; f64 halves, small counts
pub mod bridge {
    use crate::m5m_n::bridge as mn;
    use ffx_bridge::x87::{add, load, mul, store, sub};
    use ffx_graphic::host::{Conv, Host};

    /// `Ms_Gui_BattleProc` itself (the pad's edges call it again).
    pub const SELF: u32 = 0x007d_80a0;
    /// The position helpers (cdecl): `MsDebugPosGroup()` -> the group
    /// record, `MsDebugPosNextKind(step)`, `MsBtlPosAccess(dir, group,
    /// kind, pick, v)`, `MsDebugDirAt(group, kind, pick)` (ST0),
    /// `MsDebugSetDirAt(group, kind, pick, f32)`, `MsDebugIsTextMode(kind)`,
    /// `MsDebugPosDistances(group, which)`, `MsBtlPosWriteAll(out)`,
    /// `MsCopyPointsOffset(base, other, k)`, `MsDebugPlaceAll()`,
    /// `MsDebugPosWindowsAll(group)`, `MsBtlPosEditFree()`,
    /// `MsBtlPosWriteScr(name)`, `MsBtlPosLoad(name)`.
    pub const POS_GROUP: u32 = 0x007d_72e0;
    /// See [`POS_GROUP`].
    pub const NEXT_KIND: u32 = 0x007d_92d0;
    /// See [`POS_GROUP`].
    pub const POS_ACCESS: u32 = 0x007c_9ca0;
    /// See [`POS_GROUP`].
    pub const DIR_AT: u32 = 0x007d_9470;
    /// See [`POS_GROUP`].
    pub const SET_DIR_AT: u32 = 0x007d_9550;
    /// See [`POS_GROUP`].
    pub const IS_TEXT_MODE: u32 = 0x007d_79e0;
    /// See [`POS_GROUP`].
    pub const POS_DISTANCES: u32 = 0x007d_7b20;
    /// See [`POS_GROUP`].
    pub const WRITE_ALL: u32 = 0x007c_9bc0;
    /// See [`POS_GROUP`].
    pub const COPY_POINTS: u32 = 0x007a_beb0;
    /// See [`POS_GROUP`].
    pub const PLACE_ALL: u32 = 0x007d_73e0;
    /// See [`POS_GROUP`].
    pub const WINDOWS_ALL: u32 = 0x007d_74e0;
    /// See [`POS_GROUP`].
    pub const EDIT_FREE: u32 = 0x007d_7450;
    /// See [`POS_GROUP`].
    pub const WRITE_SCR: u32 = 0x007c_b640;
    /// See [`POS_GROUP`].
    pub const POS_LOAD: u32 = 0x007c_a4a0;
    /// `MsDebugBattlePosInit(out, prefix, ext)`, `MsCheckFile(name)`,
    /// `MsStrCopy(src, dst, n)`, `MsPathSetExt(path, ext, n, force)`,
    /// `op_script_compile(source, path)`, `MsGetBattleName(n, out)`,
    /// `dbgPrintf(format, ...)`, `MsCameraGetYaw()` (ST0) (cdecl).
    pub const BATTLE_POS_INIT: u32 = 0x007c_90f0;
    /// See [`BATTLE_POS_INIT`].
    pub const CHECK_FILE: u32 = 0x007c_8a80;
    /// See [`BATTLE_POS_INIT`].
    pub const STR_COPY: u32 = 0x007c_f6c0;
    /// See [`BATTLE_POS_INIT`].
    pub const PATH_SET_EXT: u32 = 0x007c_f590;
    /// See [`BATTLE_POS_INIT`].
    pub const SCRIPT_COMPILE: u32 = 0x007e_1010;
    /// See [`BATTLE_POS_INIT`].
    pub const GET_BATTLE_NAME: u32 = 0x007c_5fc0;
    /// See [`BATTLE_POS_INIT`].
    pub const DBG_PRINTF: u32 = 0x0062_f500;
    /// See [`BATTLE_POS_INIT`].
    pub const CAMERA_GET_YAW: u32 = 0x007b_cf20;
    /// `_CIcos` / `_CIsin` thunks.
    pub const CI_COS: u32 = 0x0094_98c2;
    /// See [`CI_COS`].
    pub const CI_SIN: u32 = 0x0094_98c8;

    /// The auto-repeat states (steps 1, 100, 10).
    pub const REPEAT1: u32 = 0x0113_f514;
    /// See [`REPEAT1`].
    pub const REPEAT100: u32 = 0x0113_f6a8;
    /// See [`REPEAT1`].
    pub const REPEAT10: u32 = 0x0113_f6bc;
    /// The pick of each kind (12 words), the kind (`0x00c449cc`), the kind
    /// names (`0x00c449d0`), the picked position (3 f32 at `0x0113f70c`) and
    /// direction (`0x0113f718`), the kind's count (`0x0113f700`).
    pub const PICKS: u32 = 0x0113_f6d0;
    /// See [`PICKS`].
    pub const KIND: u32 = 0x00c4_49cc;
    /// See [`PICKS`].
    pub const KIND_NAMES: u32 = 0x00c4_49d0;
    /// See [`PICKS`].
    pub const POS: u32 = 0x0113_f70c;
    /// See [`PICKS`].
    pub const DIR: u32 = 0x0113_f718;
    /// See [`PICKS`].
    pub const COUNT: u32 = 0x0113_f700;
    /// The ground group (i8 `0x0112bdec`), the distance pick (i8
    /// `0x0112bdee`), `g_BtlStatus` (i8 `0x0112a8e0`), `g_BtlFieldIndex`
    /// (i16 `0x0112c256`), the field's points (`0x0112a990`, `+1` their
    /// count), `g_BtlPosFile` (`0x0112a9b0`).
    pub const GROUP: u32 = 0x0112_bdec;
    /// See [`GROUP`].
    pub const DIST: u32 = 0x0112_bdee;
    /// See [`GROUP`].
    pub const BTL_STATUS: u32 = 0x0112_a8e0;
    /// See [`GROUP`].
    pub const FIELD_INDEX: u32 = 0x0112_c256;
    /// See [`GROUP`].
    pub const FIELD_POINTS: u32 = 0x0112_a990;
    /// See [`GROUP`].
    pub const POS_FILE: u32 = 0x0112_a9b0;
    /// The position windows (0x60 words), the edited buffer
    /// (`0x0113f510`), the battle name (`0x0113f728`), the blink
    /// (`0x0113f708`), `0x0113f704`, the pad now and before
    /// (`0x0113f720`, `0x0113f71c`), the open flag (`0x01139ef4`).
    pub const WINDOWS: u32 = 0x0113_f528;
    /// See [`WINDOWS`].
    pub const BUFFER: u32 = 0x0113_f510;
    /// See [`WINDOWS`].
    pub const BATTLE_NAME: u32 = 0x0113_f728;
    /// See [`WINDOWS`].
    pub const BLINK: u32 = 0x0113_f708;
    /// See [`WINDOWS`].
    pub const SELECTED: u32 = 0x0113_f704;
    /// See [`WINDOWS`].
    pub const PAD: u32 = 0x0113_f720;
    /// See [`WINDOWS`].
    pub const PAD_BEFORE: u32 = 0x0113_f71c;
    /// See [`WINDOWS`].
    pub const OPEN: u32 = 0x0113_9ef4;

    /// Strings: "<", ">", "DST", "DST2", "LOAD", "RAD", "SAVE", ".psc",
    /// "pos_", the not-found and read messages, "tmp", `BattlePos <%s>`,
    /// "AREA", "%d:%d", "-:-", "POS", "D", "OFF", "%3d", "X", "Y", "Z",
    /// "%.4f", "------".
    pub const S_LESS: u32 = 0x00b5_5220;
    /// See [`S_LESS`].
    pub const S_MORE: u32 = 0x00b4_8250;
    /// See [`S_LESS`].
    pub const S_DST: u32 = 0x00b5_67f4;
    /// See [`S_LESS`].
    pub const S_DST2: u32 = 0x00b5_67f8;
    /// See [`S_LESS`].
    pub const S_LOAD: u32 = 0x00b5_6800;
    /// See [`S_LESS`].
    pub const S_RAD: u32 = 0x00b5_6808;
    /// See [`S_LESS`].
    pub const S_SAVE: u32 = 0x00b5_5228;
    /// See [`S_LESS`].
    pub const S_PSC: u32 = 0x00b5_680c;
    /// See [`S_LESS`].
    pub const S_POS_PREFIX: u32 = 0x00b5_6814;
    /// See [`S_LESS`].
    pub const S_NOT_FOUND: u32 = 0x00b5_681c;
    /// See [`S_LESS`].
    pub const S_TMP: u32 = 0x00b5_682c;
    /// See [`S_LESS`].
    pub const S_READ: u32 = 0x00b5_6830;
    /// See [`S_LESS`].
    pub const S_TITLE: u32 = 0x00b5_683c;
    /// See [`S_LESS`].
    pub const S_AREA: u32 = 0x00b5_684c;
    /// See [`S_LESS`].
    pub const S_PAIR: u32 = 0x00b5_6854;
    /// See [`S_LESS`].
    pub const S_NONE: u32 = 0x00b5_685c;
    /// See [`S_LESS`].
    pub const S_POS: u32 = 0x00b5_6860;
    /// See [`S_LESS`].
    pub const S_D: u32 = 0x00b8_f7b0;
    /// See [`S_LESS`].
    pub const S_OFF: u32 = 0x00b4_0644;
    /// See [`S_LESS`].
    pub const S_3D: u32 = 0x00b5_6864;
    /// See [`S_LESS`].
    pub const S_X: u32 = 0x00b8_f800;
    /// See [`S_LESS`].
    pub const S_Y: u32 = 0x00b8_f804;
    /// See [`S_LESS`].
    pub const S_Z: u32 = 0x00b8_f808;
    /// See [`S_LESS`].
    pub const S_FLOAT: u32 = 0x00b5_6334;
    /// See [`S_LESS`].
    pub const S_DASHES: u32 = 0x00b5_6750;
    /// `f32` steps 0.01 (`0x00b1f2b0`) and 0.1 (`0x00b10f38`); `f64` pi/4
    /// (`0x00b54f30`) and 0.25 (`0x00b34f90`); the `f32` pad directions as
    /// in `Ms_Gui_ReferencePos`.
    pub const F_FINE: u32 = 0x00b1_f2b0;
    /// See [`F_FINE`].
    pub const F_COARSE: u32 = 0x00b1_0f38;
    /// See [`F_FINE`].
    pub const D_QUARTER_PI: u32 = 0x00b5_4f30;
    /// See [`F_FINE`].
    pub const D_QUARTER: u32 = 0x00b3_4f90;
    /// The pad directions: up-right, up-left, down-right, down-left, down,
    /// right, left.
    pub const F_DIRECTIONS: [u32; 7] = [
        0x00b1_1328,
        0x00b5_6968,
        0x00b5_695c,
        0x00b5_6960,
        0x00b9_22d0,
        0x00b4_3d38,
        0x00b5_6964,
    ];

    /// Bytes of [`battle_proc`]'s frame below `ebp`: the three text buffers
    /// (`ebp - 0x104` the line, `- 0x204` the name, `- 0x304` its copy).
    pub const FRAME: u32 = 0x304;

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    fn g32(h: &impl Host, sym: u32) -> u32 {
        h.read(h.rebase(sym))
    }

    fn set32(h: &mut impl Host, sym: u32, v: u32) {
        h.write(h.rebase(sym), v);
    }

    fn i8_at(h: &impl Host, sym: u32) -> i8 {
        h.read(h.rebase(sym))
    }

    fn f32_g(h: &impl Host, sym: u32) -> f64 {
        load(g32(h, sym))
    }

    /// The picked position's coordinate `c`.
    fn pos_at(h: &impl Host, c: u32) -> u32 {
        h.rebase(POS) + 4 * c
    }

    fn kind(h: &impl Host) -> u32 {
        g32(h, KIND)
    }

    /// The kind's pick.
    fn pick(h: &impl Host, kind: u32) -> u32 {
        h.read(h.rebase(PICKS).wrapping_add(kind.wrapping_mul(4)))
    }

    /// The ground group as the sign-extended word the callees get.
    fn group(h: &impl Host) -> u32 {
        i32::from(i8_at(h, GROUP)).cast_unsigned()
    }

    /// `MsBtlPosAccess(dir, group, kind, pick, POS)`.
    fn access(h: &mut impl Host, dir: u32) {
        let k = kind(h);
        let args = [dir, group(h), k, pick(h, k), h.rebase(POS)];
        cdecl(h, POS_ACCESS, &args);
    }

    /// `MsBtlPosWriteAll(&buffer)`, the field's points offset by the
    /// distance pick, `MsDebugPlaceAll()`.
    fn rewrite(h: &mut impl Host) {
        let buffer = h.rebase(BUFFER);
        cdecl(h, WRITE_ALL, &[buffer]);
        let dist = i8_at(h, DIST);
        if dist > 0 {
            let points = g32(h, FIELD_POINTS);
            let buf = g32(h, BUFFER);
            let k = (i32::from(dist) - 1).cast_unsigned();
            cdecl(h, COPY_POINTS, &[points, buf, k]);
        }
        cdecl(h, PLACE_ALL, &[]);
    }

    fn repeat_init(h: &mut impl Host, state: u32) {
        let at = h.rebase(state);
        cdecl(h, mn::MS_AUTO_REPEAT_INIT, &[at, 0xf, 3]);
    }

    /// `Ms_Gui_BattleProc(window, msg, param)` (`0x007d80a0`, cdecl) -> 0.
    /// `0x8001` builds the window (three auto-repeats, the picks cleared,
    /// 20 buttons, the windows' list cleared, the pad states -1);
    /// `0x8003` closes the position windows; `0x8006` / `0x800a` /
    /// `0x800e` reset an auto-repeat; `0x8005` / `0x8009` / `0x800d` (steps
    /// 1 / 10 / 100) run a button on the picked position; `0x8011` moves
    /// it by the pad, refreshes the windows and prints, and turns the
    /// pad's new presses into steps (calling itself). `ebp` is the
    /// original's frame ([`FRAME`] bytes below it).
    pub fn battle_proc(h: &mut impl Host, ebp: u32, window: u32, msg: u32, param: u32) -> u32 {
        match msg {
            0x8001 => create(h, window),
            0x8003 => close(h),
            0x8006 => repeat_init(h, REPEAT1),
            0x800a => repeat_init(h, REPEAT10),
            0x800e => repeat_init(h, REPEAT100),
            0x8005 | 0x8009 | 0x800d => step(h, ebp, msg, param),
            0x8011 => refresh(h, ebp, window),
            _ => {}
        }
        0
    }

    fn create(h: &mut impl Host, window: u32) {
        for state in [REPEAT1, REPEAT100, REPEAT10] {
            repeat_init(h, state);
        }
        for k in 0..12u32 {
            h.write(h.rebase(PICKS) + 4 * k, 0u32);
        }
        let k = kind(h);
        let kind_name: u32 = h.read(h.rebase(KIND_NAMES).wrapping_add(k.wrapping_mul(4)));
        let (l, m) = (S_LESS, S_MORE);
        let buttons = [
            (0x3d, Some(l), 0x34, 0, 0x12),
            (0x3e, Some(m), 0x74, 0, 0x12),
            (0x40, None, 0x9a, 0, 0x48),
            (0x15, Some(l), 0x34, 0xa, 0x12),
            (0x16, Some(m), 0x74, 0xa, 0x12),
            (0x49, Some(l), 0x9a, 0xa, 0x12),
            (0x4a, Some(m), 0xd0, 0xa, 0x12),
            (0x41, Some(l), 0x34, 0x14, 0x12),
            (0x42, Some(m), 0x9a, 0x14, 0x12),
            (0x52, Some(S_DST), 0xb2, 0x14, 0x30),
            (0x43, Some(l), 0x34, 0x1e, 0x12),
            (0x44, Some(m), 0x9a, 0x1e, 0x12),
            (0x8d, Some(S_DST2), 0xb2, 0x1e, 0x30),
            (0x45, Some(l), 0x34, 0x28, 0x12),
            (0x46, Some(m), 0x9a, 0x28, 0x12),
            (0x1f, Some(S_LOAD), 0xb2, 0x28, 0x30),
            (0x4b, Some(S_RAD), 4, 0x32, 0x28),
            (0x47, Some(l), 0x34, 0x32, 0x12),
            (0x48, Some(m), 0x9a, 0x32, 0x12),
            (0x1e, Some(S_SAVE), 0xb2, 0x32, 0x30),
        ];
        for (id, text, x, y, w) in buttons {
            let text = text.map_or(kind_name, |s| h.rebase(s));
            cdecl(h, mn::GU_WINDOW_ADD_BUTTON, &[window, id, text, x, y, w, 8]);
        }
        for k in 0..0x60u32 {
            h.write(h.rebase(WINDOWS) + 4 * k, 0u32);
        }
        set32(h, BLINK, 0);
        set32(h, SELECTED, 0);
        set32(h, PAD, u32::MAX);
        set32(h, PAD_BEFORE, u32::MAX);
    }

    fn close(h: &mut impl Host) {
        set32(h, OPEN, 0);
        set32(h, SELECTED, u32::MAX);
        for k in 0..0x60u32 {
            let slot = h.rebase(WINDOWS) + 4 * k;
            let w: u32 = h.read(slot);
            if w != 0 {
                h.write(w.wrapping_add(0x20), 0u8);
                let w: u32 = h.read(slot);
                h.write(w.wrapping_add(0x21), 0u8);
                h.write(slot, 0u32);
            }
        }
    }

    /// What a step did: nothing, a group or pick change (1), or a
    /// position change to write back (-1).
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Change {
        None,
        Moved,
        Edited,
    }

    /// `0x8005` / `0x8009` / `0x800d`: the auto-repeat, the picked position
    /// read, the button, the table written back.
    #[allow(clippy::too_many_lines)] // one button switch
    fn step(h: &mut impl Host, ebp: u32, msg: u32, param: u32) {
        let (state, step) = match msg {
            0x8005 => (REPEAT1, 1),
            0x800d => (REPEAT100, 100),
            _ => (REPEAT10, 10),
        };
        let at = h.rebase(state);
        if cdecl(h, mn::MS_AUTO_REPEAT_CHECK, &[at, param]) == 0 {
            return;
        }
        cdecl(h, POS_GROUP, &[]);
        let mut k = kind(h);
        let scale = f32_g(h, if k == 0 { F_FINE } else { F_COARSE });
        for c in 0..3u32 {
            h.write(h.rebase(POS) + 4 * c, 0u32);
        }
        let mut in_battle = i8_at(h, BTL_STATUS) != 0;
        if in_battle {
            access(h, u32::MAX);
            let k2 = kind(h);
            let args = [group(h), k2, pick(h, k2)];
            let dir = h.call_sym_st0(DIR_AT, Conv::Cdecl, &args);
            set32(h, DIR, dir.to_bits());
            k = kind(h);
            in_battle = i8_at(h, BTL_STATUS) != 0;
        }
        // `fild` of the step through an f32, times the f32 scale
        let delta = mul(load(store(f64::from(step))), scale);
        let mut change = Change::None;
        match param {
            0x15 | 0x16 => {
                if cdecl(h, IS_TEXT_MODE, &[k]) != 0 && in_battle {
                    let at = h.rebase(PICKS).wrapping_add(k.wrapping_mul(4));
                    let v: u32 = h.read(at);
                    let v = if param == 0x15 {
                        v.wrapping_sub(1)
                    } else {
                        v.wrapping_add(1)
                    };
                    h.write(at, v);
                    change = Change::Moved;
                }
            }
            0x1e if in_battle => {
                let name = ebp - 0x204;
                let (prefix, ext) = (h.rebase(S_POS_PREFIX), h.rebase(S_PSC));
                cdecl(h, BATTLE_POS_INIT, &[name, prefix, ext]);
                cdecl(h, WRITE_SCR, &[name]);
            }
            0x1f if in_battle => {
                if load_file(h, ebp) {
                    change = Change::Moved;
                }
            }
            0x3d | 0x3e => {
                let g = i8_at(h, GROUP);
                let g = if param == 0x3d {
                    g.wrapping_sub(1)
                } else {
                    g.wrapping_add(1)
                };
                h.write(h.rebase(GROUP), g);
                change = Change::Moved;
            }
            0x40 => {
                let (k, dir) = if msg == 0x8005 {
                    (k.wrapping_add(1), 1)
                } else {
                    (k.wrapping_sub(1), u32::MAX)
                };
                set32(h, KIND, k);
                cdecl(h, NEXT_KIND, &[dir]);
            }
            0x41..=0x46 => {
                let c = (param - 0x41) / 2;
                let at = pos_at(h, c);
                let v = if param % 2 == 1 {
                    sub(load(h.read(at)), delta)
                } else {
                    add(delta, load(h.read(at)))
                };
                h.write(at, store(v));
                change = Change::Edited;
            }
            0x47 | 0x48 => {
                if k == 0xb {
                    let at = pos_at(h, 0);
                    let v = if param == 0x47 {
                        sub(load(h.read(at)), delta)
                    } else {
                        add(delta, load(h.read(at)))
                    };
                    h.write(at, store(v));
                    change = Change::Edited;
                } else if k == 3 || k == 5 || k == 4 {
                    let v = if param == 0x47 {
                        sub(f32_g(h, DIR), delta)
                    } else {
                        add(delta, f32_g(h, DIR))
                    };
                    set32(h, DIR, store(v));
                    let k = kind(h);
                    let deg = store(f32_g(h, DIR));
                    let args = [group(h), k, pick(h, k), deg];
                    cdecl(h, SET_DIR_AT, &args);
                    change = Change::Edited;
                }
            }
            0x49 | 0x4a => {
                if h.read::<i16>(h.rebase(FIELD_INDEX)) > 0 {
                    let d = i8_at(h, DIST);
                    let d = if param == 0x49 {
                        let d = d.wrapping_sub(1);
                        if d < 0 { 0 } else { d }
                    } else {
                        let d = d.wrapping_add(1);
                        h.write(h.rebase(DIST), d);
                        let points = g32(h, FIELD_POINTS);
                        let max: i8 = h.read(points.wrapping_add(1));
                        if d > max { max } else { d }
                    };
                    h.write(h.rebase(DIST), d);
                    change = Change::Moved;
                }
            }
            0x4b if k == 0xb => {
                h.write(pos_at(h, 0), 0u32);
                change = Change::Edited;
            }
            0x52 | 0x8d => {
                let which = u32::from(param == 0x8d);
                let g = group(h);
                cdecl(h, POS_DISTANCES, &[g, which]);
            }
            _ => {}
        }
        cdecl(h, POS_GROUP, &[]);
        if change == Change::None {
            return;
        }
        if change == Change::Edited {
            access(h, 1);
        }
        rewrite(h);
    }

    /// Button `0x1f`: the battle's position script compiled into a
    /// temporary file and loaded. Returns whether it loaded.
    fn load_file(h: &mut impl Host, ebp: u32) -> bool {
        let (name, copy) = (ebp - 0x204, ebp - 0x304);
        let (prefix, ext) = (h.rebase(S_POS_PREFIX), h.rebase(S_PSC));
        cdecl(h, BATTLE_POS_INIT, &[name, prefix, ext]);
        if cdecl(h, CHECK_FILE, &[name]).cast_signed() < 0 {
            let f = h.rebase(S_NOT_FOUND);
            cdecl(h, DBG_PRINTF, &[f, name]);
            return false;
        }
        cdecl(h, EDIT_FREE, &[]);
        cdecl(h, STR_COPY, &[name, copy, 0x100]);
        let tmp = h.rebase(S_TMP);
        cdecl(h, PATH_SET_EXT, &[copy, tmp, 0x100, 1]);
        cdecl(h, SCRIPT_COMPILE, &[name, copy]);
        let loaded = cdecl(h, POS_LOAD, &[copy]) != 0;
        if loaded {
            let f = h.rebase(S_READ);
            cdecl(h, DBG_PRINTF, &[f, name]);
            let buffer = h.rebase(BUFFER);
            cdecl(h, WRITE_ALL, &[buffer]);
            let b = g32(h, BUFFER);
            set32(h, POS_FILE, b);
            h.write(h.rebase(DIST), 0u8);
        }
        cdecl(h, NEXT_KIND, &[1]);
        loaded
    }

    /// The pad's direction as an angle, if any (as `Ms_Gui_ReferencePos`).
    fn pad_angle(h: &impl Host, pad: u32) -> Option<u32> {
        let f = |k: usize| -> u32 { store(load(h.read(h.rebase(F_DIRECTIONS[k])))) };
        if pad & 0x1000 != 0 {
            Some(if pad & 0x8000 != 0 {
                f(0)
            } else if pad & 0x2000 != 0 {
                f(1)
            } else {
                0
            })
        } else if pad & 0x4000 != 0 {
            Some(if pad & 0x8000 != 0 {
                f(2)
            } else if pad & 0x2000 != 0 {
                f(3)
            } else {
                f(4)
            })
        } else if pad & 0x8000 != 0 {
            Some(f(5))
        } else if pad & 0x2000 != 0 {
            Some(f(6))
        } else {
            None
        }
    }

    /// One printed line: `Gu_PrintSetX(x)`, `Gu_PrintSetY(y)`,
    /// `sprintf(buf, args)`, `Gu_Printf(buf)`.
    fn line(h: &mut impl Host, buf: u32, x: u32, y: u32, args: &[u32]) {
        cdecl(h, mn::GU_PRINT_SET_X, &[x]);
        cdecl(h, mn::GU_PRINT_SET_Y, &[y]);
        let mut a = vec![buf];
        a.extend_from_slice(args);
        let f = g32(h, mn::SPRINTF);
        h.call(f, Conv::Cdecl, &a);
        cdecl(h, mn::GU_PRINTF, &[buf]);
    }

    /// `"%.4f"` of an `f32` global, or `"------"`.
    fn value_args(h: &impl Host, v: Option<u32>) -> Vec<u32> {
        match v {
            Some(sym) => {
                let b = f32_g(h, sym).to_bits();
                vec![h.rebase(S_FLOAT), b as u32, (b >> 32) as u32]
            }
            None => vec![h.rebase(S_DASHES)],
        }
    }

    /// `0x8011`: the pad's move, the windows' flags, the title and the
    /// printout, the pad's new presses.
    #[allow(clippy::too_many_lines)] // the printout, in order
    fn refresh(h: &mut impl Host, ebp: u32, window: u32) {
        let g = group(h);
        cdecl(h, WINDOWS_ALL, &[g]);
        let pad = cdecl(h, mn::TK_GET_CONTROLER_HIST, &[0, 0, 0]) & 0xffff;
        set32(h, PAD, pad);
        if let Some(dir) = pad_angle(h, pad) {
            let k = kind(h);
            if k != 0xb && k != 0 {
                pad_move(h, dir);
            }
        }
        let blink = g32(h, BLINK).wrapping_sub(1) & 1;
        set32(h, BLINK, blink);
        let grp = cdecl(h, POS_GROUP, &[]);
        for sym in [POS, POS + 4, POS + 8, DIR] {
            set32(h, sym, 0);
        }
        let k = kind(h);
        let name: u32 = h.read(h.rebase(KIND_NAMES).wrapping_add(k.wrapping_mul(4)));
        cdecl(h, mn::GU_WINDOW_SET_BUTTON_TEXT, &[window, 0x40, name]);
        for k in 0..0x60u32 {
            let w: u32 = h.read(h.rebase(WINDOWS) + 4 * k);
            if w != 0 {
                let on = h.read::<u8>(h.rebase(BTL_STATUS)) & 1;
                h.write(w.wrapping_add(0x21), on);
            }
        }
        let names = h.rebase(BATTLE_NAME);
        cdecl(h, GET_BATTLE_NAME, &[1, names]);
        let sprintf = g32(h, mn::SPRINTF);
        let title = ebp - 0x204;
        let fmt = h.rebase(S_TITLE);
        h.call(sprintf, Conv::Cdecl, &[title, fmt, names]);
        cdecl(h, mn::GU_WINDOW_SET_TITLE, &[window, title]);
        let buf = ebp - 0x104;
        line(h, buf, 8, 1, &[h.rebase(S_AREA)]);
        let groups = if i8_at(h, BTL_STATUS) != 0 {
            h.read::<i8>(grp.wrapping_add(1))
        } else {
            0
        };
        if groups > 0 {
            let args = [
                h.rebase(S_PAIR),
                group(h),
                (i32::from(h.read::<i8>(grp.wrapping_add(1))) - 1).cast_unsigned(),
            ];
            line(h, buf, 0x50, 1, &args);
        } else {
            line(h, buf, 0x50, 1, &[h.rebase(S_NONE)]);
        }
        line(h, buf, 8, 0xb, &[h.rebase(S_POS)]);
        let count = g32(h, COUNT).cast_signed();
        if i8_at(h, BTL_STATUS) != 0 && count > 0 {
            let k = kind(h);
            let n = (g32(h, COUNT).cast_signed() - 1).cast_unsigned();
            let args = [h.rebase(S_PAIR), pick(h, k), n];
            line(h, buf, 0x50, 0xb, &args);
        } else {
            line(h, buf, 0x50, 0xb, &[h.rebase(S_NONE)]);
        }
        line(h, buf, 0x8c, 0xb, &[h.rebase(S_D)]);
        let dist = i8_at(h, DIST);
        if dist == 0 {
            line(h, buf, 0xb2, 0xb, &[h.rebase(S_OFF)]);
        } else {
            let d = (i32::from(dist) - 1).cast_unsigned();
            line(h, buf, 0xb2, 0xb, &[h.rebase(S_3D), d]);
        }
        // which values to show: -1 the position, 1 its x as the radius
        let mut show = 0i32;
        let mut show_dir = false;
        if i8_at(h, BTL_STATUS) != 0 {
            access(h, u32::MAX);
            let k = kind(h);
            if k == 0xb {
                show = 1;
            } else {
                show = -1;
                if k == 3 || k == 5 || k == 4 {
                    show_dir = true;
                    let args = [group(h), k, pick(h, k)];
                    let dir = h.call_sym_st0(DIR_AT, Conv::Cdecl, &args);
                    set32(h, DIR, dir.to_bits());
                }
            }
        }
        for (c, (label, y)) in [(S_X, 0x15u32), (S_Y, 0x1f), (S_Z, 0x29)]
            .into_iter()
            .enumerate()
        {
            line(h, buf, 8, y, &[h.rebase(label)]);
            let v = (show < 0).then_some(POS + 4 * c as u32);
            let args = value_args(h, v);
            // call width: 4 (the buffer, the format and an f64)
            line(h, buf, 0x4e, y, &args);
        }
        let v = if show > 0 {
            Some(POS)
        } else if show_dir {
            Some(DIR)
        } else {
            None
        };
        let args = value_args(h, v);
        // call width: 4 (the buffer, the format and an f64)
        line(h, buf, 0x4e, 0x33, &args);
        let edges = [
            (4u32, REPEAT1, 0x8005u32, 0x15u32),
            (8, REPEAT1, 0x8005, 0x16),
            (1, REPEAT10, 0x8009, 0x40),
            (2, REPEAT1, 0x8005, 0x40),
        ];
        for (bit, state, msg, param) in edges {
            let before = g32(h, PAD_BEFORE);
            if before & bit == 0 && u32::from(h.read::<u8>(h.rebase(PAD))) & bit != 0 {
                set32(h, state, u32::MAX);
                cdecl(h, SELF, &[window, msg, param]);
            }
        }
        let pad = g32(h, PAD);
        set32(h, PAD_BEFORE, pad);
    }

    /// The pad's move of the picked position along the camera's yaw (a
    /// quarter with `0x40`), written back, the table rewritten.
    fn pad_move(h: &mut impl Host, dir: u32) {
        let yaw = h.call_sym_st0_f64(CAMERA_GET_YAW, Conv::Cdecl, &[]);
        let quarter_pi = f64::from_bits(h.read(h.rebase(D_QUARTER_PI)));
        let angle = store(add(sub(yaw, quarter_pi), load(dir)));
        let cos = h.call_intrinsic(h.rebase(CI_COS), load(angle)).to_bits();
        let sin = h.call_intrinsic(h.rebase(CI_SIN), load(angle)).to_bits();
        let a = store(sub(load(cos), load(sin)));
        let b = store(add(load(cos), load(sin)));
        access(h, u32::MAX);
        let (x, z) = (h.rebase(POS), h.rebase(POS + 8));
        if h.read::<u8>(h.rebase(PAD)) & 0x40 != 0 {
            let q = f64::from_bits(h.read(h.rebase(D_QUARTER)));
            h.write(x, store(add(mul(load(a), q), load(h.read(x)))));
            h.write(z, store(add(mul(q, load(b)), load(h.read(z)))));
        } else {
            h.write(x, store(add(load(h.read(x)), load(a))));
            h.write(z, store(add(load(h.read(z)), load(b))));
        }
        access(h, 1);
        rewrite(h);
    }
}
