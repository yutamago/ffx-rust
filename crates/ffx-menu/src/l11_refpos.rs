//! M5 round 11 (`tail11`): `Ms_Gui_ReferencePos` (`0x007d9c20`, cdecl
//! `(window, msg, param)` -> 0), the battle debug `Reference` window: the
//! reference mode's slider and buttons, the pad-driven free reference
//! point, and its printout.
//!
//! x87: every value is computed at 53 bits in the original's order and
//! rounded where the original stores a `dword` (`ffx_bridge::x87`).

/// The original as a bridge over [`ffx_graphic::host::Host`].
#[allow(clippy::many_single_char_names, clippy::cast_possible_truncation)] // x87 terms and host handles; f64 halves, small counts
pub mod bridge {
    use crate::m5m_n::bridge as mn;
    use ffx_bridge::x87::{add, load, mul, store, sub};
    use ffx_graphic::host::{Conv, Host};

    /// `AtelChangeCurrentCtrl(0)`, `MsSaveUcID(id)`,
    /// `AtelGetTotalActorNumber2()`, `AtelSetCtrlChar(id)`,
    /// `Sg_SetPlayer(n)`, `MsVectorPolar2Rect(v, r, yaw, pitch, dist)`,
    /// `MsCameraGetYaw()` (ST0) (cdecl).
    pub const ATEL_CHANGE_CURRENT_CTRL: u32 = 0x0086_2d30;
    /// See [`ATEL_CHANGE_CURRENT_CTRL`].
    pub const MS_SAVE_UC_ID: u32 = 0x007d_42e0;
    /// See [`ATEL_CHANGE_CURRENT_CTRL`].
    pub const ATEL_GET_TOTAL_ACTOR_NUMBER2: u32 = 0x0086_c510;
    /// See [`ATEL_CHANGE_CURRENT_CTRL`].
    pub const ATEL_SET_CTRL_CHAR: u32 = 0x0086_f580;
    /// See [`ATEL_CHANGE_CURRENT_CTRL`].
    pub const SG_SET_PLAYER: u32 = 0x0082_dad0;
    /// See [`ATEL_CHANGE_CURRENT_CTRL`].
    pub const MS_VECTOR_POLAR2RECT: u32 = 0x007c_47c0;
    /// See [`ATEL_CHANGE_CURRENT_CTRL`].
    pub const MS_CAMERA_GET_YAW: u32 = 0x007b_cf20;
    /// The reference helpers (cdecl): `MsDebugRefModeNext(mode, step,
    /// pick, obj, cam, base, id)`, `MsDebugRefDraw(cam)`,
    /// `MsDebugRefAdd(delta)` (ST0), `MsDebugRefRound()`,
    /// `MsDebugRefFromCamera(cam)`, `MsDebugRefSet(7 f32)` (ST0).
    pub const REF_MODE_NEXT: u32 = 0x007d_9b00;
    /// See [`REF_MODE_NEXT`].
    pub const REF_DRAW: u32 = 0x007d_34a0;
    /// See [`REF_MODE_NEXT`].
    pub const REF_ADD: u32 = 0x007d_2f50;
    /// See [`REF_MODE_NEXT`].
    pub const REF_ROUND: u32 = 0x007d_23d0;
    /// See [`REF_MODE_NEXT`].
    pub const REF_FROM_CAMERA: u32 = 0x007d_36b0;
    /// See [`REF_MODE_NEXT`].
    pub const REF_SET: u32 = 0x007d_3420;
    /// The mode sliders (cdecl): XYZ `(obj, id, f32)`, actors `(obj, id,
    /// f32, cam id)`, reference `(mode, obj, id, f32)`.
    pub const SLIDER_XYZ: u32 = 0x007d_abc0;
    /// See [`SLIDER_XYZ`].
    pub const SLIDER_ACTORS: u32 = 0x007d_95e0;
    /// See [`SLIDER_XYZ`].
    pub const SLIDER_REFERENCE: u32 = 0x007d_1bd0;
    /// The printers (cdecl, -> the next y): XYZ `(obj, id, y)`, actors
    /// `(obj, id, y)`, `MsDebugBtlPrint(mode, base, obj, id, y)`, dashes
    /// `(y)`.
    pub const PRINT_XYZ: u32 = 0x007d_acd0;
    /// See [`PRINT_XYZ`].
    pub const PRINT_ACTORS: u32 = 0x007d_9700;
    /// See [`PRINT_XYZ`].
    pub const PRINT_BTL: u32 = 0x007d_1f50;
    /// See [`PRINT_XYZ`].
    pub const PRINT_DASHES: u32 = 0x007d_9950;
    /// `_CIcos` / `_CIsin` thunks (`fld`, call, ST0).
    pub const CI_COS: u32 = 0x0094_98c2;
    /// See [`CI_COS`].
    pub const CI_SIN: u32 = 0x0094_98c8;

    /// `g_DebugCamera` (`0x011370c1` + 0x1c0 x `0x01139f10`): the debug
    /// camera's index byte.
    pub const DEBUG_CAMERA: u32 = 0x0113_70c1;
    /// See [`DEBUG_CAMERA`].
    pub const DEBUG_CAMERA_INDEX: u32 = 0x0113_9f10;
    /// The camera blocks (0xda0 bytes each): `+0` here (the reference
    /// object at `+0xc0 + 0x130 x num`), `+0x61c` the base.
    pub const CAMERAS: u32 = 0x0113_7570;
    /// The picked actor (`0x0113f864`, -1 the camera), the actor count
    /// (`0x0113f86c`), the pad bits (`0x0113f868`), the window flag
    /// (`0x01139ee4`).
    pub const ACTOR: u32 = 0x0113_f864;
    /// See [`ACTOR`].
    pub const ACTORS: u32 = 0x0113_f86c;
    /// See [`ACTOR`].
    pub const PAD: u32 = 0x0113_f868;
    /// See [`ACTOR`].
    pub const OPEN: u32 = 0x0113_9ee4;
    /// The three auto-repeat states (steps 1, 100, 10).
    pub const REPEAT1: u32 = 0x0113_f828;
    /// See [`REPEAT1`].
    pub const REPEAT100: u32 = 0x0113_f83c;
    /// See [`REPEAT1`].
    pub const REPEAT10: u32 = 0x0113_f850;
    /// The mode names (`0x00c44640`, by mode), the keep names
    /// (`0x00c44264`), the mode order (`0x00c44a10`, 5 words).
    pub const MODE_NAMES: u32 = 0x00c4_4640;
    /// See [`MODE_NAMES`].
    pub const KEEP_NAMES: u32 = 0x00c4_4264;
    /// See [`MODE_NAMES`].
    pub const MODE_ORDER: u32 = 0x00c4_4a10;
    /// Strings: `Reference`, `REFmode`, "fit", "<", ">", "CENT", "OUT",
    /// "UCON", "%3d:%3d", "camera", "FILT", "%.4f", the FLST line, "%s\n".
    pub const S_TITLE: u32 = 0x00b5_696c;
    /// See [`S_TITLE`].
    pub const S_REFMODE: u32 = 0x00b5_6978;
    /// See [`S_TITLE`].
    pub const S_FIT: u32 = 0x00b5_6308;
    /// See [`S_TITLE`].
    pub const S_LESS: u32 = 0x00b5_5220;
    /// See [`S_TITLE`].
    pub const S_MORE: u32 = 0x00b4_8250;
    /// See [`S_TITLE`].
    pub const S_CENT: u32 = 0x00b5_6980;
    /// See [`S_TITLE`].
    pub const S_OUT: u32 = 0x00b5_5e14;
    /// See [`S_TITLE`].
    pub const S_UCON: u32 = 0x00b5_6988;
    /// See [`S_TITLE`].
    pub const S_ACTOR: u32 = 0x00b5_6998;
    /// See [`S_TITLE`].
    pub const S_CAMERA: u32 = 0x00b5_6990;
    /// See [`S_TITLE`].
    pub const S_FILT: u32 = 0x00b5_69a0;
    /// See [`S_TITLE`].
    pub const S_FLOAT: u32 = 0x00b5_6334;
    /// See [`S_TITLE`].
    pub const S_FLST: u32 = 0x00b5_69a8;
    /// See [`S_TITLE`].
    pub const S_LINE: u32 = 0x00b1_2ad0;
    /// `f64` constants: -0.001 (`0x00b56a00`), 0.001 (`0x00b921b0`), pi/4
    /// (`0x00b54f30`), 0.25 (`0x00b34f90`), 2 degrees (`0x00b569f8`), 1
    /// degree (`0x00b11330`), 2 (`0x00b92270`).
    pub const D_MINUS_MILLI: u32 = 0x00b5_6a00;
    /// See [`D_MINUS_MILLI`].
    pub const D_MILLI: u32 = 0x00b9_21b0;
    /// See [`D_MINUS_MILLI`].
    pub const D_QUARTER_PI: u32 = 0x00b5_4f30;
    /// See [`D_MINUS_MILLI`].
    pub const D_QUARTER: u32 = 0x00b3_4f90;
    /// See [`D_MINUS_MILLI`].
    pub const D_YAW_STEP: u32 = 0x00b5_69f8;
    /// See [`D_MINUS_MILLI`].
    pub const D_PITCH_STEP: u32 = 0x00b1_1330;
    /// See [`D_MINUS_MILLI`].
    pub const D_TWO: u32 = 0x00b9_2270;
    /// `f32` pad directions: up-right `0x00b11328`, up-left `0x00b56968`,
    /// down-right `0x00b5695c`, down-left `0x00b56960`, down `0x00b922d0`,
    /// right `0x00b43d38`, left `0x00b56964`.
    pub const F_UP_RIGHT: u32 = 0x00b1_1328;
    /// See [`F_UP_RIGHT`].
    pub const F_UP_LEFT: u32 = 0x00b5_6968;
    /// See [`F_UP_RIGHT`].
    pub const F_DOWN_RIGHT: u32 = 0x00b5_695c;
    /// See [`F_UP_RIGHT`].
    pub const F_DOWN_LEFT: u32 = 0x00b5_6960;
    /// See [`F_UP_RIGHT`].
    pub const F_DOWN: u32 = 0x00b9_22d0;
    /// See [`F_UP_RIGHT`].
    pub const F_RIGHT: u32 = 0x00b4_3d38;
    /// See [`F_UP_RIGHT`].
    pub const F_LEFT: u32 = 0x00b5_6964;

    /// Bytes of [`reference_pos`]'s frame below `ebp`: the polar form the
    /// callees write (`ebp - 0x124` distance, `- 0x11c` yaw, `- 0x114`
    /// pitch) and the text buffer (`ebp - 0x104`).
    pub const FRAME: u32 = 0x124;

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    fn f64_at(h: &impl Host, sym: u32) -> f64 {
        f64::from_bits(h.read(h.rebase(sym)))
    }

    fn f32_at(h: &impl Host, at: u32) -> f64 {
        load(h.read(at))
    }

    /// An `f32` as `fld dword` / `fstp dword` copy it.
    fn fcopy(bits: u32) -> u32 {
        store(load(bits))
    }

    fn clamp_actor(h: &mut impl Host) {
        let total: i32 = h.read(h.rebase(ACTORS));
        let actor: i32 = h.read(h.rebase(ACTOR));
        if actor < -1 {
            h.write(h.rebase(ACTOR), total.wrapping_sub(1));
        } else if actor >= total {
            h.write(h.rebase(ACTOR), -1i32);
        }
    }

    /// What every message reads first.
    struct Ctx {
        window: u32,
        msg: u32,
        param: u32,
        ebp: u32,
        /// The debug camera's index.
        cam: u32,
        /// `MsCameraGetID(cam)`.
        id: u32,
        /// `0x130 x MsCameraGetNum(cam)`.
        off: u32,
        /// The camera block and its base (`+0x61c`).
        block: u32,
        base: u32,
        /// The reference object.
        obj: u32,
        keep: u32,
        mode: u32,
        mode1: u32,
        /// `id >> 16 & 0xf`.
        kind: u32,
        arg: u32,
    }

    /// `Ms_Gui_ReferencePos(window, msg, param)` (`0x007d9c20`, cdecl) -> 0.
    /// Every message first sets the current control, reads the debug
    /// camera, its reference object, the keep and the two camera modes,
    /// refreshes the picked actor (unless kind 5) and the actor count (31
    /// for kind 5, at least 1). `0x8001` builds the window (its argument
    /// 0x94 bytes with a selector at `+4`, 22 buttons, the auto-repeats);
    /// `0x8003` clears the open flag; `0x8006` / `0x800a` / `0x800e` reset
    /// an auto-repeat; `0x8005` / `0x8009` / `0x800d` (steps 1 / 10 / 100)
    /// run the mode's slider and the button, then pick the actor;
    /// `0x8011` sets the reference from the object's lists, refreshes the
    /// texts, moves the free reference point by the pad (camera picked) and
    /// prints; `0x8012` takes the mode selector's pick. `ebp` is the
    /// original's frame ([`FRAME`] bytes below it).
    pub fn reference_pos(h: &mut impl Host, ebp: u32, window: u32, msg: u32, param: u32) -> u32 {
        cdecl(h, ATEL_CHANGE_CURRENT_CTRL, &[0]);
        let index: u32 = h.read(h.rebase(DEBUG_CAMERA_INDEX));
        let cam = u32::from(
            h.read::<u8>(
                h.rebase(DEBUG_CAMERA)
                    .wrapping_add(index.wrapping_mul(0x1c0)),
            ),
        );
        let id = cdecl(h, mn::MS_CAMERA_GET_ID, &[cam]);
        let num = cdecl(h, mn::MS_CAMERA_GET_NUM, &[cam]);
        let off = num.wrapping_mul(0x130);
        let block = h.rebase(CAMERAS).wrapping_add(cam.wrapping_mul(0xda0));
        let base = block.wrapping_add(0x61c);
        let obj = block.wrapping_add(0xc0).wrapping_add(off);
        let keep = cdecl(h, mn::MS_DEBUG_CAMERA_KEEP, &[0]);
        let mode = cdecl(h, mn::MS_DEBUG_GET_CAMERA_MODE, &[0]);
        let mode1 = cdecl(h, mn::MS_DEBUG_GET_CAMERA_MODE, &[1]);
        let kind = (id.cast_signed() >> 16).cast_unsigned() & 0xf;
        let mut actor: i32 = h.read(h.rebase(ACTOR));
        if actor >= 0 && kind != 5 {
            actor = cdecl(h, mn::ATEL_GET_NOW_CTRL_TOP_ID, &[]).cast_signed();
            h.write(h.rebase(ACTOR), actor);
        }
        cdecl(h, MS_SAVE_UC_ID, &[actor.cast_unsigned()]);
        let total = if kind == 5 {
            0x1f
        } else {
            let n = cdecl(h, ATEL_GET_TOTAL_ACTOR_NUMBER2, &[]).cast_signed();
            if n > 0 { n } else { 1 }
        };
        h.write(h.rebase(ACTORS), total);
        let arg = cdecl(h, mn::CP_WINDOW_GET_ARG, &[window]);
        let c = Ctx {
            window,
            msg,
            param,
            ebp,
            cam,
            id,
            off,
            block,
            base,
            obj,
            keep,
            mode,
            mode1,
            kind,
            arg,
        };
        match msg {
            0x8001 => create(h, &c),
            0x8003 => h.write(h.rebase(OPEN), 0u32),
            0x8006 => repeat_init(h, REPEAT1),
            0x800a => repeat_init(h, REPEAT10),
            0x800e => repeat_init(h, REPEAT100),
            0x8005 | 0x8009 | 0x800d => step(h, &c),
            0x8011 => refresh(h, &c),
            0x8012 if param == 0x36 => pick(h, &c),
            _ => {}
        }
        0
    }

    /// The window argument's selector (`+4`), re-read as the original.
    fn selector(h: &impl Host, c: &Ctx) -> u32 {
        h.read(c.arg.wrapping_add(4))
    }

    fn repeat_init(h: &mut impl Host, state: u32) {
        let at = h.rebase(state);
        cdecl(h, mn::MS_AUTO_REPEAT_INIT, &[at, 0xf, 1]);
    }

    /// `0x8001`: the argument, the selector, the title and the buttons.
    fn create(h: &mut impl Host, c: &Ctx) {
        let block = cdecl(h, mn::CP_USER_MALLOC, &[0x94]);
        cdecl(h, mn::CP_MEMSET, &[block, 0, 0x94]);
        cdecl(h, mn::CP_WINDOW_SET_ARG, &[c.window, block]);
        let sel = cdecl(h, mn::GU_SELECTOR_ALLOCATE, &[0x78, 0x28, 0x5a, 0x64]);
        h.write(block.wrapping_add(4), sel);
        let title = h.rebase(S_TITLE);
        cdecl(h, mn::GU_WINDOW_SET_TITLE, &[c.window, title]);
        let mode_name: u32 = h.read(h.rebase(MODE_NAMES).wrapping_add(c.mode.wrapping_mul(4)));
        let keep_name: u32 = h.read(h.rebase(KEEP_NAMES).wrapping_add(c.keep.wrapping_mul(4)));
        let (less, more) = (h.rebase(S_LESS), h.rebase(S_MORE));
        let mut buttons = vec![
            (0x36, h.rebase(S_REFMODE), 0, 0, 0x44),
            (2, mode_name, 0x48, 0, 0x34),
            (0x33, h.rebase(S_FIT), 0x80, 0, 0x34),
        ];
        // the slider pairs: "<" at x 0x30 and ">" at x 0x44, one row each
        for (k, (lo, hi)) in [
            (0x27, 0x28),
            (0x29, 0x2a),
            (0x2b, 0x2c),
            (0x2d, 0x2e),
            (0x2f, 0x30),
            (0x31, 0x32),
            (9, 10),
            (0xd, 0xe),
        ]
        .into_iter()
        .enumerate()
        {
            let y = 0xa * (k as u32 + 1);
            buttons.push((lo, less, 0x30, y, 0x12));
            buttons.push((hi, more, 0x44, y, 0x12));
        }
        buttons.push((8, h.rebase(S_CENT), 4, 0x64, 0x34));
        buttons.push((6, keep_name, 0x40, 0x64, 0x34));
        buttons.push((5, h.rebase(S_OUT), 0x7c, 0x64, 0x34));
        for (button, text, x, y, w) in buttons {
            cdecl(
                h,
                mn::GU_WINDOW_ADD_BUTTON,
                &[c.window, button, text, x, y, w, 8],
            );
        }
        for state in [REPEAT1, REPEAT100, REPEAT10] {
            repeat_init(h, state);
        }
        cdecl(h, mn::MS_SET_TABLE_ENTRY, &[2, 1]);
    }

    /// `0x8012` with the mode selector: the picked name's mode, if it is
    /// one of the five, through `MsDebugRefModeNext(mode, 0, k, ...)`.
    fn pick(h: &mut impl Host, c: &Ctx) {
        let sel: u32 = h.read(c.arg.wrapping_add(4));
        let names = h.rebase(MODE_NAMES);
        let name = cdecl(h, mn::GU_SELECTOR_GET_ITEM_NAME, &[sel]);
        let found = cdecl(h, mn::MS_DEBUG_FIND_NAME, &[name, names]);
        let order = h.rebase(MODE_ORDER);
        for k in 0..5u32 {
            if h.read::<u32>(order + 4 * k) == found {
                let args = [c.mode, 0, k, c.obj, c.block, c.base, c.id];
                cdecl(h, REF_MODE_NEXT, &args);
                return;
            }
        }
    }

    /// `0x8005` / `0x8009` / `0x800d`: the auto-repeat, the mode's slider,
    /// the button, the actor's bounds and the control change.
    fn step(h: &mut impl Host, c: &Ctx) {
        let (state, step) = match c.msg {
            0x8005 => (REPEAT1, 1),
            0x800d => (REPEAT100, 100),
            _ => (REPEAT10, 10),
        };
        let at = h.rebase(state);
        if cdecl(h, mn::MS_AUTO_REPEAT_CHECK, &[at, c.param]) == 0 {
            return;
        }
        let prev: i32 = h.read(h.rebase(ACTOR));
        let amount = store(f64::from(step));
        match c.mode {
            1 => {
                cdecl(h, SLIDER_XYZ, &[c.obj, c.param, amount]);
            }
            3 => {
                cdecl(h, SLIDER_ACTORS, &[c.obj, c.param, amount, c.id]);
            }
            0xb..=0xd => {
                cdecl(h, SLIDER_REFERENCE, &[c.mode, c.obj, c.param, amount]);
            }
            _ => {}
        }
        let up = c.msg == 0x8005;
        match c.param {
            2 => {
                let dir = if up { 1 } else { u32::MAX };
                let args = [c.mode, dir, 0, c.obj, c.block, c.base, c.id];
                cdecl(h, REF_MODE_NEXT, &args);
            }
            5 => {
                cdecl(h, mn::MS_DEBUG_OUT_CAMERA, &[]);
            }
            6 => {
                let dir = if up { 1 } else { u32::MAX };
                cdecl(h, mn::MS_DEBUG_CAMERA_KEEP, &[dir]);
            }
            8 => {
                cdecl(h, REF_FROM_CAMERA, &[c.cam]);
                cdecl(h, REF_DRAW, &[c.cam]);
            }
            9 | 0xa => {
                let a: i32 = h.read(h.rebase(ACTOR));
                let v = if c.param == 9 {
                    a.wrapping_sub(step)
                } else {
                    a.wrapping_add(step)
                };
                h.write(h.rebase(ACTOR), v);
            }
            0xd | 0xe => {
                let k = if c.param == 0xd {
                    D_MINUS_MILLI
                } else {
                    D_MILLI
                };
                let delta = store(mul(f64::from(step), f64_at(h, k)));
                h.call_sym_st0(REF_ADD, Conv::Cdecl, &[delta]);
                cdecl(h, REF_DRAW, &[c.cam]);
            }
            0x33 => {
                cdecl(h, REF_ROUND, &[]);
            }
            0x36 => {
                let s = selector(h, c);
                cdecl(h, mn::GU_SELECTOR_RESET_NB_ITEM, &[s, 5]);
                let order = h.rebase(MODE_ORDER);
                let names = h.rebase(MODE_NAMES);
                for k in 0..5u32 {
                    let m: u32 = h.read(order + 4 * k);
                    let name: u32 = h.read(names.wrapping_add(m.wrapping_mul(4)));
                    let s = selector(h, c);
                    cdecl(h, mn::GU_SELECTOR_SET_ITEM, &[s, k, name, k]);
                }
                let s = selector(h, c);
                cdecl(h, mn::GU_SELECTOR_OPEN, &[s, c.window, 0x36]);
            }
            _ => {}
        }
        clamp_actor(h);
        let actor: i32 = h.read(h.rebase(ACTOR));
        let total: i32 = h.read(h.rebase(ACTORS));
        if prev != actor && total > 0 && cdecl(h, mn::MS_BATTLE_GET_STATUS, &[]) == 0 {
            cdecl(h, ATEL_SET_CTRL_CHAR, &[actor.cast_unsigned()]);
        }
    }

    /// `0x8011`: the reference from the object's five lists, the actor's
    /// bounds, the texts, the free point (camera picked), the printout.
    fn refresh(h: &mut impl Host, c: &Ctx) {
        if h.read::<u8>(c.obj.wrapping_add(4)) != 0 {
            set_reference(h, c);
        }
        clamp_actor(h);
        let names = h.rebase(MODE_NAMES);
        let text: u32 = h.read(names.wrapping_add(c.mode.wrapping_mul(4)));
        cdecl(h, mn::GU_WINDOW_SET_BUTTON_TEXT, &[c.window, 2, text]);
        let keeps = h.rebase(KEEP_NAMES);
        let text: u32 = h.read(keeps.wrapping_add(c.keep.wrapping_mul(4)));
        cdecl(h, mn::GU_WINDOW_SET_BUTTON_TEXT, &[c.window, 6, text]);
        let mut mode = c.mode;
        if h.read::<i32>(h.rebase(ACTOR)) < 0 {
            mode = 1;
            free_point(h, c);
        }
        print(h, c, mode);
    }

    /// `MsDebugRefSet` from the first list of kind 1 or 3 (its point at
    /// `+0x18`, `+0x1c`) and the first of another kind (value `+0xc`, the
    /// point `+0x18`, `+0x1c` and the ends `+0x10 + 0x18`, `+0x14 +
    /// 0x1c`), lists of 0x20 bytes from `obj + 0x90`; 1 and 0 otherwise.
    fn set_reference(h: &mut impl Host, c: &Ctx) {
        let zero = 0u32;
        let mut value = 1f32.to_bits();
        let (mut px, mut dx, mut py, mut pz, mut dy, mut dz) = (zero, zero, zero, zero, zero, zero);
        let (mut found13, mut found) = (false, false);
        for k in 0..5u32 {
            let rec = c.obj.wrapping_add(0x90 + 0x20 * k);
            let t: u32 = h.read(rec);
            if t == 0 {
                continue;
            }
            if t == 1 || t == 3 {
                if !found13 {
                    py = fcopy(h.read(rec.wrapping_add(0x18)));
                    found13 = true;
                    dy = fcopy(h.read(rec.wrapping_add(0x1c)));
                }
            } else if !found {
                value = fcopy(h.read(rec.wrapping_add(0xc)));
                found = true;
                px = fcopy(h.read(rec.wrapping_add(0x18)));
                dx = store(add(
                    f32_at(h, rec.wrapping_add(0x10)),
                    f32_at(h, rec.wrapping_add(0x18)),
                ));
                pz = fcopy(h.read(rec.wrapping_add(0x1c)));
                dz = store(add(
                    f32_at(h, rec.wrapping_add(0x14)),
                    f32_at(h, rec.wrapping_add(0x1c)),
                ));
            }
        }
        h.call_sym_st0(REF_SET, Conv::Cdecl, &[value, px, py, pz, dx, dy, dz]);
    }

    /// The pad's direction (`0x1000` up, `0x4000` down, `0x8000` right,
    /// `0x2000` left) as an angle, if any.
    fn pad_angle(h: &impl Host, pad: u32) -> Option<u32> {
        let f = |sym: u32| -> u32 { h.read(h.rebase(sym)) };
        if pad & 0x1000 != 0 {
            Some(if pad & 0x8000 != 0 {
                fcopy(f(F_UP_RIGHT))
            } else if pad & 0x2000 != 0 {
                fcopy(f(F_UP_LEFT))
            } else {
                0
            })
        } else if pad & 0x4000 != 0 {
            Some(fcopy(if pad & 0x8000 != 0 {
                f(F_DOWN_RIGHT)
            } else if pad & 0x2000 != 0 {
                f(F_DOWN_LEFT)
            } else {
                f(F_DOWN)
            }))
        } else if pad & 0x8000 != 0 {
            Some(fcopy(f(F_RIGHT)))
        } else if pad & 0x2000 != 0 {
            Some(fcopy(f(F_LEFT)))
        } else {
            None
        }
    }

    /// The camera picked: the object back from the block (mode 3), XYZ mode,
    /// the player 0, the pad moving the object (along the camera's yaw) and
    /// turning the base's polar form.
    fn free_point(h: &mut impl Host, c: &Ctx) {
        let obj = c.obj;
        if c.mode == 3 {
            for k in 0..4u32 {
                let v = fcopy(h.read(c.block.wrapping_add(0x20 + 4 * k)));
                h.write(obj.wrapping_add(0x30 + 4 * k), v);
            }
        }
        cdecl(h, mn::MS_DEBUG_SET_CAMERA_MODE, &[0, 1]);
        cdecl(h, SG_SET_PLAYER, &[0]);
        let pad = cdecl(h, mn::TK_GET_CONTROLER_HIST, &[0, 0, 0]) & 0xffff;
        h.write(h.rebase(PAD), pad);
        let dir = pad_angle(h, pad);
        let yaw = h.call_sym_st0_f64(MS_CAMERA_GET_YAW, Conv::Cdecl, &[]);
        let angle = store(add(
            sub(yaw, f64_at(h, D_QUARTER_PI)),
            load(dir.unwrap_or(0)),
        ));
        let pad: u32 = h.read(h.rebase(PAD));
        if dir.is_some() {
            let cos = h.call_intrinsic(h.rebase(CI_COS), load(angle)).to_bits();
            let sin = h.call_intrinsic(h.rebase(CI_SIN), load(angle)).to_bits();
            let a = store(sub(load(cos), load(sin)));
            let b = store(add(load(cos), load(sin)));
            let (x, z) = (obj.wrapping_add(0x30), obj.wrapping_add(0x38));
            if pad & 0x40 != 0 {
                let q = f64_at(h, D_QUARTER);
                h.write(x, store(add(mul(load(a), q), f32_at(h, x))));
                h.write(z, store(add(mul(q, load(b)), f32_at(h, z))));
            } else {
                h.write(x, store(add(f32_at(h, x), load(a))));
                h.write(z, store(add(f32_at(h, z), load(b))));
            }
        }
        let y = obj.wrapping_add(0x34);
        if pad & 2 != 0 {
            h.write(y, store(sub(f32_at(h, y), 1.0)));
        }
        if pad & 1 != 0 {
            h.write(y, store(add(1.0, f32_at(h, y))));
        }
        polar(h, c);
    }

    /// The base's polar form turned by the pad (`8`/`4` yaw, `0x20`/`0x80`
    /// pitch, `0x10`/`0x40` distance), written back when any of them moved.
    fn polar(h: &mut impl Host, c: &Ctx) {
        let (dist, yaw, pitch) = (c.ebp - 0x124, c.ebp - 0x11c, c.ebp - 0x114);
        let vec = c.base.wrapping_add(c.off);
        let radius: u32 = h.read(c.base.wrapping_add(0x8c));
        if c.mode1 == 2 {
            for (to, k) in [(yaw, 0x100u32), (pitch, 0x104), (dist, 0x108)] {
                let v = fcopy(h.read(vec.wrapping_add(k)));
                h.write(to, v);
            }
        } else {
            let at = vec.wrapping_add(0xf0);
            cdecl(
                h,
                mn::MS_VECTOR_RECT2_POLAR,
                &[at, radius, yaw, pitch, dist],
            );
            if c.mode1 == 4 {
                let v = fcopy(h.read(vec.wrapping_add(0x104)));
                h.write(pitch, v);
            }
        }
        let pad: u32 = h.read(h.rebase(PAD));
        let mut moved = false;
        let steps = [
            (yaw, D_YAW_STEP, 8u32, 4u32),
            (pitch, D_PITCH_STEP, 0x20, 0x80),
        ];
        for (at, k, minus, plus) in steps {
            let d = f64_at(h, k);
            if pad & minus != 0 {
                h.write(at, store(sub(f32_at(h, at), d)));
                moved = true;
            }
            if pad & plus != 0 {
                h.write(at, store(add(d, f32_at(h, at))));
                moved = true;
            }
        }
        let two = f64_at(h, D_TWO);
        if pad & 0x10 != 0 {
            h.write(dist, store(sub(f32_at(h, dist), two)));
            moved = true;
        }
        if pad & 0x40 != 0 {
            h.write(dist, store(add(two, f32_at(h, dist))));
            moved = true;
        }
        if !moved {
            return;
        }
        if c.mode1 == 2 {
            for (from, k) in [(yaw, 0x100u32), (pitch, 0x104), (dist, 0x108)] {
                let v = fcopy(h.read(from));
                h.write(vec.wrapping_add(k), v);
            }
        } else {
            let at = vec.wrapping_add(0xf0);
            // junk read: the original pushes five words and the callee reads
            // eight (+0x1c..+0x24, its centre by value when `radius` is 0:
            // the caller's saved registers); 0 (brief 2026-10-24)
            let words = [
                at,
                radius,
                fcopy(h.read(yaw)),
                fcopy(h.read(pitch)),
                fcopy(h.read(dist)),
                0,
                0,
                0,
            ];
            cdecl(h, MS_VECTOR_POLAR2RECT, &words);
            if c.mode1 == 4 {
                let v = fcopy(h.read(pitch));
                h.write(vec.wrapping_add(0x104), v);
            }
        }
    }

    /// Copies the i16 at `from` to `to` when it is -1 or more.
    fn copy_short(h: &mut impl Host, from: u32, to: u32) {
        let v: i16 = h.read(from);
        if v >= -1 {
            h.write(to, v);
        }
    }

    /// One row: `Gu_PrintSetX(x)`, `Gu_PrintSetY(y)`, `sprintf(buf,
    /// args)`, `Gu_Printf("%s\n", buf)`.
    fn row(h: &mut impl Host, buf: u32, x: u32, y: u32, args: &[u32]) {
        cdecl(h, mn::GU_PRINT_SET_X, &[x]);
        cdecl(h, mn::GU_PRINT_SET_Y, &[y]);
        let mut a = vec![buf];
        a.extend_from_slice(args);
        let f: u32 = h.read(h.rebase(mn::SPRINTF));
        h.call(f, Conv::Cdecl, &a);
        let line = h.rebase(S_LINE);
        cdecl(h, mn::GU_PRINTF, &[line, buf]);
    }

    /// The mode's printout, then UCON (the actor or "camera"), FILT (the
    /// reference value), the lists' line, `MsDebugCameraApply(0)`.
    fn print(h: &mut impl Host, c: &Ctx, mode: u32) {
        let obj = c.obj;
        let y = match mode {
            1 => cdecl(h, PRINT_XYZ, &[obj, c.id, 1]),
            3 => {
                let y = cdecl(h, PRINT_ACTORS, &[obj, c.id, 1]);
                if c.kind != 5 {
                    copy_short(h, obj.wrapping_add(0xc), obj.wrapping_add(0x10));
                    copy_short(h, obj.wrapping_add(0xe), obj.wrapping_add(0x12));
                }
                y
            }
            0xb..=0xd => {
                let y = cdecl(h, PRINT_BTL, &[mode, c.block, obj, c.id, 1]);
                if c.kind != 5 {
                    copy_short(h, obj.wrapping_add(0x10), obj.wrapping_add(0xc));
                    copy_short(h, obj.wrapping_add(0x12), obj.wrapping_add(0xe));
                }
                y
            }
            0xe => cdecl(h, mn::MS_DEBUG_SPLINE_PRINT, &[c.block, obj, c.id, 1]),
            _ => cdecl(h, PRINT_DASHES, &[1]),
        };
        let buf = c.ebp - 0x104;
        let y = y.wrapping_add(0xa);
        row(h, buf, 0, y, &[h.rebase(S_UCON)]);
        let actor: i32 = h.read(h.rebase(ACTOR));
        let total: i32 = h.read(h.rebase(ACTORS));
        if actor >= 0 && total != 0 {
            let f = h.rebase(S_ACTOR);
            row(
                h,
                buf,
                0x5c,
                y,
                &[f, actor.cast_unsigned(), total.cast_unsigned()],
            );
        } else {
            row(h, buf, 0x5c, y, &[h.rebase(S_CAMERA)]);
        }
        let y = y.wrapping_add(0xa);
        row(h, buf, 0, y, &[h.rebase(S_FILT)]);
        cdecl(h, mn::GU_PRINT_SET_X, &[0x5c]);
        cdecl(h, mn::GU_PRINT_SET_Y, &[y]);
        let v = h.call_sym_st0_f64(REF_ADD, Conv::Cdecl, &[0]).to_bits();
        let f: u32 = h.read(h.rebase(mn::SPRINTF));
        let fmt = h.rebase(S_FLOAT);
        h.call(f, Conv::Cdecl, &[buf, fmt, v as u32, (v >> 32) as u32]);
        let line = h.rebase(S_LINE);
        cdecl(h, mn::GU_PRINTF, &[line, buf]);
        let mut args = vec![
            h.rebase(S_FLST),
            u32::from(h.read::<u8>(obj.wrapping_add(4))),
            u32::from(h.read::<u8>(obj.wrapping_add(5))),
        ];
        for k in 0..5u32 {
            args.push(h.read(obj.wrapping_add(0x90 + 0x20 * k)));
        }
        // call width: 9 (the buffer, the format, two bytes, five words)
        row(h, buf, 0, y.wrapping_add(0xa), &args);
        cdecl(h, mn::MS_DEBUG_CAMERA_APPLY, &[0]);
    }
}
