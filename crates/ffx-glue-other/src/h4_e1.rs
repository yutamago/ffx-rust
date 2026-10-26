//! Continuation round 3 of list `h4-glue-other`, batch 1: the menus' x87
//! cursor and plate drawing (the equipment, weapon-shop and item lists'
//! cursors, the brushes, the status background, the weapon ability
//! lists, the battle target cursors and the enemies' limit gauges).
//!
//! Every float is computed as the x87 does at 53 bits (`ffx_bridge::x87`):
//! `f64` operations in the original's order, rounded to `f32` where the
//! original stores one (`fstp dword`). A float the original only copies
//! through `fld`/`fstp` is [`pass`]ed. `graphicUiRemapX2`/`Y2` return their
//! float in `ST0` ([`Host::call_sym_st0_f64`]). Floats travel as their bits.
//!
//! Bridges that hand callees a local take `frame`, the block laid out as
//! the original's frame from its lowest local up.

use ffx_bridge::Global;
use ffx_bridge::x87::{add, div, ftol, load, mul, store, sub};
use ffx_graphic::host::{Conv, Host};

/// `graphicUiRemapX2(x)` (cdecl, float in `ST0`).
pub const REMAP_X2: u32 = 0x0064_47e0;
/// `graphicUiRemapY2(y)` (cdecl, float in `ST0`).
pub const REMAP_Y2: u32 = 0x0064_4820;
/// `TkMn2DrawCrossCmdCursor(x, y, blink)` (cdecl).
pub const CROSS_CMD_CURSOR: u32 = 0x008c_0580;
/// `TkMn2DrawCrossCursor(x, y, kind)` (cdecl).
pub const CROSS_CURSOR: u32 = 0x008c_0670;
/// `TOMkpShapeXYWHUV(shape, x, y, w, h, u, v, uw, vh)` (cdecl).
pub const SHAPE_UV: u32 = 0x0090_3bb0;
/// `TOMkpShapeXYWHUVC2(shape, x, y, w, h, u, v, uw, vh, c0, c1)` (cdecl).
pub const SHAPE_UV_C2: u32 = 0x0090_3ee0;
/// `TkMenuGetTimer()` (cdecl; the u8 timer zero-extended).
pub const MENU_TIMER: u32 = 0x008a_9c60;
/// `graphicGetTime()` (cdecl, float in `ST0`).
pub const GET_TIME: u32 = 0x0064_1410;
/// `_CIcos`'s import thunk.
pub const CI_COS: u32 = 0x0094_98c2;
/// `RatioSquared(a, b)` (cdecl, float in `ST0`).
pub const RATIO_SQUARED: u32 = 0x008f_38c0;
/// `toGetStatusWeaponMesPos()` (cdecl).
pub const WEAPON_MES_POS: u32 = 0x008d_54a0;
/// `MsCalcWeaponCap(weapon)` (cdecl).
pub const CALC_WEAPON_CAP: u32 = 0x0079_8eb0;
/// `MsGetSaveWeapon(id, &name)` (cdecl).
pub const SAVE_WEAPON: u32 = 0x007a_bbd0;
/// `TODrawMenuPlateXYWHType(x, y, w, h, kind)` (cdecl).
pub const MENU_PLATE: u32 = 0x008f_5f70;
/// `TODrawCrossMenuIconAbilityName(id, x, y, alpha)` (cdecl).
pub const ABILITY_NAME: u32 = 0x008f_4f10;

/// A float through `fld dword` and `fstp dword` (a signalling NaN comes
/// out quiet).
#[must_use]
pub fn pass(bits: u32) -> u32 {
    store(load(bits))
}

/// An integer through `fild` and `fstp dword`.
#[must_use]
pub fn int_f32(n: i32) -> u32 {
    store(f64::from(n))
}

/// The `f32` constant at image address `sym`, passed through `fld`/`fstp`.
fn konst(h: &impl Host, sym: u32) -> u32 {
    pass(h.read(h.rebase(sym)))
}

/// The `f64` constant at image address `sym` (`fmul qword` and friends).
fn konst64(h: &impl Host, sym: u32) -> f64 {
    h.read(h.rebase(sym))
}

/// `graphicUiRemapX2` of the constant at `sym`: `ST0`.
fn remap_x(h: &mut impl Host, sym: u32) -> f64 {
    let v = konst(h, sym);
    h.call_sym_st0_f64(REMAP_X2, Conv::Cdecl, &[v])
}

/// `graphicUiRemapY2` of the constant at `sym`: `ST0`.
fn remap_y(h: &mut impl Host, sym: u32) -> f64 {
    let v = konst(h, sym);
    h.call_sym_st0_f64(REMAP_Y2, Conv::Cdecl, &[v])
}

/// `constant remapped + v` stored (`fadd dword [v]; fstp dword`).
fn plus(r: f64, v: u32) -> u32 {
    store(add(r, load(v)))
}

/// A list's row offset: `i16 +0x48 - i16 +0x32` (the cursor row less the
/// scroll), with `halve` the row first halved toward zero.
fn row(h: &impl Host, list: u32, halve: bool) -> i32 {
    let cursor = i32::from(h.read::<i16>(list.wrapping_add(0x48)));
    let cursor = if halve { cursor / 2 } else { cursor };
    cursor.wrapping_sub(i32::from(h.read::<i16>(list.wrapping_add(0x32))))
}

// ---- list cursors

/// `g_TkMenuEquipFade` (i32).
pub const EQUIP_FADE: Global<u32> = Global::new(0x0186_a614);
/// `g_TkMenuEquipAltMode`.
pub const EQUIP_ALT_MODE: Global<u32> = Global::new(0x0186_a624);
/// `g_TkMenuEquipListOpen`.
pub const EQUIP_LIST_OPEN: Global<u32> = Global::new(0x0186_a62c);

/// `TkMenuEquip_DrawCursor_008cf990()` (cdecl): once the equipment
/// menu's fade (`0x0186a614`, signed) reaches 0x1000, the command cursor
/// at x `toGetStatusWeaponMesPos()` (as a float) and y
/// `graphicUiRemapY2(0x16a + 0x46 * alt mode)`, blinking by the list-open
/// word (`0x0186a62c`). The original's `eax` is the cursor's (a void
/// callee's) or the caller's: none.
pub fn equip_cursor(h: &mut impl Host) {
    if EQUIP_FADE.get(h).cast_signed() < 0x1000 {
        return;
    }
    let open = EQUIP_LIST_OPEN.get(h);
    let y = 0x16a_u32.wrapping_add(EQUIP_ALT_MODE.get(h).wrapping_mul(0x46));
    let y = h.call_sym_st0_f64(REMAP_Y2, Conv::Cdecl, &[pass(int_f32(y.cast_signed()))]);
    let y = store(y);
    let x = h.call_sym(WEAPON_MES_POS, Conv::Cdecl, &[]);
    let x = pass(int_f32(x.cast_signed()));
    h.call_sym(CROSS_CMD_CURSOR, Conv::Cdecl, &[x, y, open]);
}

/// `MenuWeapon_DrawListCursor_008d0100(list, x, y)` (cdecl): the command
/// cursor at `(float)x` and `(float)ftol(remapY2(0x00b65e48) * row + y)`
/// (row: [`row`]), not blinking.
pub fn weapon_list_cursor(h: &mut impl Host, list: u32, x: u32, y: u32) {
    let r = remap_y(h, 0x00b6_5e48);
    let n = int_f32(row(h, list, false));
    let v = add(mul(r, load(n)), f64::from(y.cast_signed()));
    let v = ftol(h, v);
    let yv = pass(int_f32(v.cast_signed()));
    let xv = pass(int_f32(x.cast_signed()));
    h.call_sym(CROSS_CMD_CURSOR, Conv::Cdecl, &[xv, yv, 0]);
}

/// The weapon shop's list cursors: `TkMn2DrawCrossCursor(x + remapX2(cx),
/// y + remapY2(cy) + remapY2(0x00b5eecc) * row, 0)`, with `cx` chosen by
/// `pick_x` from the list. Returns the cursor's `eax`.
fn shop_cursor(h: &mut impl Host, list: u32, x: u32, y: u32, cx: u32, cy: u32, halve: bool) -> u32 {
    let xv = plus(remap_x(h, cx), x);
    let yv = plus(remap_y(h, cy), y);
    let r = remap_y(h, 0x00b5_eecc);
    let n = int_f32(row(h, list, halve));
    let yv = store(add(mul(r, load(n)), load(yv)));
    h.call_sym(CROSS_CURSOR, Conv::Cdecl, &[pass(xv), pass(yv), 0])
}

/// `MenuWShop_DrawBuyListCursor_008d8bb0(list, x, y)` and its exact copy
/// `MenuWShop_DrawSellListCursor_008d8e60` (cdecl): [`shop_cursor`] with
/// the offsets `0x00b664e0`/`0x00b664dc`.
pub fn shop_list_cursor(h: &mut impl Host, list: u32, x: u32, y: u32) -> u32 {
    shop_cursor(h, list, x, y, 0x00b6_64e0, 0x00b6_64dc, false)
}

/// `MenuWShop_BuyListCursor_008dab70(list, x, y)` (cdecl): [`shop_cursor`]
/// with `0x00b65e4c`/`0x00b6636c`.
pub fn shop_buy_cursor(h: &mut impl Host, list: u32, x: u32, y: u32) -> u32 {
    shop_cursor(h, list, x, y, 0x00b6_5e4c, 0x00b6_636c, false)
}

/// `MenuWShop_SellWindowCursor_008dae00(list, x, y)` (cdecl):
/// [`shop_cursor`] with x offset `0x00b66720` on an odd row (bit 0 of
/// byte `+0x48`) else `0x00b65e4c`, y `0x00b6636c`, the row halved (two
/// columns).
pub fn shop_sell_cursor(h: &mut impl Host, list: u32, x: u32, y: u32) -> u32 {
    let cx = if h.read::<u8>(list.wrapping_add(0x48)) & 1 != 0 {
        0x00b6_6720
    } else {
        0x00b6_5e4c
    };
    shop_cursor(h, list, x, y, cx, 0x00b6_636c, true)
}

/// `g_TkMenuItemTargetMode`.
pub const ITEM_TARGET_MODE: Global<u32> = Global::new(0x0186_a40c);

/// `MenuItem_DrawListCursor_008c7af0(list, x, y)` (cdecl): the item list's
/// command cursor. The entry `i8 +0x45` (clamped below the count `u8
/// +0x42` for the column test) is drawn when it lies in the list (or the
/// list has two entries) and is not negative, else only in a target mode
/// (`0x0186a40c`). A filled column (`+0x58 + 4 * entry`) moves x by
/// `remapX2(0x00b5870c)`; y moves by `remapY2(0x00b5eecc) * row`. In
/// target mode 2 the cursor blinks (hidden on bit 1 of the menu timer
/// when the entry is positive). Returns what the original leaves in
/// `eax` on its early exits (the clamped entry, the timer halved); the
/// cursor's own is a void callee's (`None`).
pub fn item_list_cursor(h: &mut impl Host, list: u32, x: u32, y: u32) -> Option<u32> {
    let entry: i8 = h.read(list.wrapping_add(0x45));
    let count = u32::from(h.read::<u8>(list.wrapping_add(0x42)));
    let idx = i32::from(entry);
    let col = if idx < count.cast_signed() {
        idx.cast_unsigned()
    } else {
        count.wrapping_sub(1)
    };
    let shown = (idx < count.cast_signed() || count == 2) && entry >= 0;
    if !shown && ITEM_TARGET_MODE.get(h) == 0 {
        return Some(col);
    }
    let mut x = x;
    let filled: u32 = h.read(list.wrapping_add(0x58).wrapping_add(col.wrapping_mul(4)));
    if filled != 0 {
        x = plus(remap_x(h, 0x00b5_870c), x);
    }
    let r = remap_y(h, 0x00b5_eecc);
    let n = int_f32(row(h, list, false));
    let y = store(add(mul(r, load(n)), load(y)));
    let mut blink = 0;
    if ITEM_TARGET_MODE.get(h) == 2 {
        let t = h.call_sym(MENU_TIMER, Conv::Cdecl, &[]);
        let half = (t.cast_signed() / 2).cast_unsigned();
        let entry: i8 = h.read(list.wrapping_add(0x45));
        if half & 1 != 0 && entry > 0 {
            return Some(half);
        }
        let entry: i8 = h.read(list.wrapping_add(0x45));
        if ITEM_TARGET_MODE.get(h) == 2 && entry > 0 {
            blink = 1;
        }
    }
    let yv = plus(remap_y(h, 0x00b9_2314), y);
    let xv = plus(remap_x(h, 0x00b5_a480), x);
    h.call_sym(CROSS_CMD_CURSOR, Conv::Cdecl, &[pass(xv), pass(yv), blink]);
    None
}

// ---- the shape cursors

/// The arrow shape both command cursors draw at `(x, y)`:
/// `TOMkpShapeXYWHUV(0xc8, x, y, remapX2(0x00b5eec0), remapY2(0x00b5ee90),
/// 0x00b51b9c, 0x00b65854, 0x00b657fc, 0x00b6585c)` (the last four image
/// constants).
fn cursor_shape(h: &mut impl Host, x: u32, y: u32) {
    let [c4, c3, c2, c1] =
        [0x00b6_585c, 0x00b6_57fc, 0x00b6_5854, 0x00b5_1b9c].map(|c| konst(h, c));
    let height = store(remap_y(h, 0x00b5_ee90));
    let width = store(remap_x(h, 0x00b5_eec0));
    h.call_sym(
        SHAPE_UV,
        Conv::Cdecl,
        &[0xc8, pass(x), pass(y), width, height, c1, c2, c3, c4],
    );
}

/// `TkMn2DrawCrossCmdCursor(x, y, blink)` (cdecl): hidden while blinking
/// on bit 1 of the menu timer; else the arrow ([`cursor_shape`]) at `x -
/// remapX2(0x00b5eec0)` (not blinking: plus `cos(time * 0x00b922a8) *
/// 0x00b2c5c8`, the `f64` constants) and `y - remapY2(0)`. Returns the
/// timer on the hidden path (`Some`); the shape is a void callee's.
pub fn cross_cmd_cursor(h: &mut impl Host, x: u32, y: u32, blink: u32) -> Option<u32> {
    let t = h.call_sym(MENU_TIMER, Conv::Cdecl, &[]);
    if t & 2 != 0 && blink != 0 {
        return Some(t);
    }
    let r = remap_x(h, 0x00b5_eec0);
    let mut x = store(sub(load(x), r));
    if blink == 0 {
        let time = h.call_sym_st0_f64(GET_TIME, Conv::Cdecl, &[]);
        let angle = mul(time, konst64(h, 0x00b9_22a8));
        let cos = h.rebase(CI_COS);
        let c = h.call_intrinsic(cos, angle).to_bits();
        let wobble = mul(load(c), konst64(h, 0x00b2_c5c8));
        x = store(add(wobble, load(x)));
    }
    let r = h.call_sym_st0_f64(REMAP_Y2, Conv::Cdecl, &[0]);
    let y = store(sub(load(y), r));
    cursor_shape(h, x, y);
    None
}

/// `TkMn2DrawStatiCursor(x, y, hidden)` (cdecl): unless `hidden`, the
/// arrow ([`cursor_shape`]) at `x - remapX2(0x00b5eec0)`, `y -
/// remapY2(0x00b3eb98)`.
pub fn stati_cursor(h: &mut impl Host, x: u32, y: u32, hidden: u32) {
    if hidden != 0 {
        return;
    }
    let r = remap_x(h, 0x00b5_eec0);
    let x = store(sub(load(x), r));
    let r = remap_y(h, 0x00b3_eb98);
    let y = store(sub(load(y), r));
    cursor_shape(h, x, y);
}

// ---- brushes and backgrounds

/// `TkMn2DrawBrush(a, x, y)` (cdecl; `a` unused): the brush shape
/// `TOMkpShapeXYWHUV(0xc8, (float)x, (float)y, remapX2(0x00b6589c),
/// remapY2(0x00b92314), 0, 0x00b5f0c8, 0x00b65804, 0x00b65820)`.
pub fn draw_brush(h: &mut impl Host, x: u32, y: u32) {
    let s4 = konst(h, 0x00b6_5820);
    let s3 = konst(h, 0x00b6_5804);
    let s2 = konst(h, 0x00b5_f0c8);
    let height = store(remap_y(h, 0x00b9_2314));
    let width = store(remap_x(h, 0x00b6_589c));
    let yv = pass(int_f32(y.cast_signed()));
    let xv = pass(int_f32(x.cast_signed()));
    h.call_sym(
        SHAPE_UV,
        Conv::Cdecl,
        &[0xc8, xv, yv, width, height, 0, s2, s3, s4],
    );
}

/// The menu brushes (16-byte records from `0x018762e0`).
pub const BRUSHES: u32 = 0x0187_62e0;

/// `TOMenuDrawBrush2(i)` (cdecl): brush `i` (u16 `+0` on): its step `i16
/// +0xa` counts up to `+0xc`; the drawn width `+8` is
/// `ftol(height * ratio)` (`+6`; ratio `RatioSquared(step, steps)`), and
/// the shape `TOMkpShapeXYWHUV(0xc8, x +2, y +4, width, remapY2(0x00b92314),
/// 0, 0x00b5f0c8, ratio * 0x00b66d08 * 0x00b58b50, 0x00b65820)`.
pub fn draw_brush2(h: &mut impl Host, i: u32) {
    let e = h.rebase(BRUSHES).wrapping_add(i << 4);
    if h.read::<u16>(e) == 0 {
        return;
    }
    let steps: i16 = h.read(e.wrapping_add(0xc));
    let step: i16 = h.read(e.wrapping_add(0xa));
    if step < steps {
        h.write(e.wrapping_add(0xa), step.wrapping_add(1));
    }
    let step: i16 = h.read(e.wrapping_add(0xa));
    let ratio = h.call_sym_st0_f64(
        RATIO_SQUARED,
        Conv::Cdecl,
        &[
            i32::from(step).cast_unsigned(),
            i32::from(steps).cast_unsigned(),
        ],
    );
    let ratio = store(ratio);
    let full = f64::from(h.read::<i16>(e.wrapping_add(6)));
    let width = ftol(h, mul(full, load(ratio)));
    let s1 = konst(h, 0x00b6_5820);
    #[allow(clippy::cast_possible_truncation)] // `mov word`
    h.write(e.wrapping_add(8), width as u16);
    let v = mul(
        mul(load(ratio), konst64(h, 0x00b6_6d08)),
        konst64(h, 0x00b5_8b50),
    );
    let s0 = pass(store(v));
    let t2 = konst(h, 0x00b5_f0c8);
    let t0 = store(remap_y(h, 0x00b9_2314));
    let [u2, u1, u0] =
        [8u32, 4, 2].map(|off| pass(int_f32(i32::from(h.read::<i16>(e.wrapping_add(off))))));
    h.call_sym(
        SHAPE_UV,
        Conv::Cdecl,
        &[0xc8, u0, u1, u2, t0, 0, t2, s0, s1],
    );
}

/// `TODrawMenuBGC2(a, b)` (cdecl): the menu background
/// `TOMkpShapeXYWHUVC2(-2, 0, 0, 0x00b41d9c, 0x00b41d98, 0, 0,
/// 0x00b4a1ac, 0x00b55178, a & 0xff000000, b & 0xff000000)` (the colours'
/// alpha bytes only).
pub fn draw_menu_bg_c2(h: &mut impl Host, a: u32, b: u32) {
    let depth = konst(h, 0x00b5_5178);
    let q6 = konst(h, 0x00b4_a1ac);
    let q3 = konst(h, 0x00b4_1d98);
    let q2 = konst(h, 0x00b4_1d9c);
    h.call_sym(
        SHAPE_UV_C2,
        Conv::Cdecl,
        &[
            0xffff_fffe,
            0,
            0,
            q2,
            q3,
            0,
            0,
            q6,
            depth,
            a & 0xff00_0000,
            b & 0xff00_0000,
        ],
    );
}

/// `TkMn2DrawStatusBG(colour, kind)` (cdecl): with `colour`'s grey
/// (`(colour & !0x1f) << 19`) not 0, the status background in that grey
/// `| 0x808080`: its height `h` 0x25 for kind 4, else
/// `ftol(remapY2(0x00b49988))` (kind 1) or `ftol(remapY2(0x00b661c8))`,
/// drawn as `TOMkpShapeXYWHUVC2(-2, 0, h, 0x00b41d9c, (0x1a0 - h) -
/// remapY2(0x00b567a8), 0, h * 0x00b51ba8 / 0x00b41d30, 0x00b4a1ac,
/// 0x00b66230, grey, grey)`.
pub fn draw_status_bg(h: &mut impl Host, colour: u32, kind: u32) {
    let grey = (colour & 0xffff_ffe0) << 0x13;
    if grey == 0 {
        return;
    }
    let grey = grey | 0x0080_8080;
    let height = match kind {
        1 => {
            let r = remap_y(h, 0x00b4_9988);
            ftol(h, r)
        }
        4 => 0x25,
        _ => {
            let r = remap_y(h, 0x00b6_61c8);
            ftol(h, r)
        }
    };
    let hf = pass(int_f32(height.cast_signed()));
    let q4 = konst(h, 0x00b6_6230);
    let q3 = konst(h, 0x00b4_a1ac);
    let w = store(div(
        mul(load(hf), konst64(h, 0x00b5_1ba8)),
        konst64(h, 0x00b4_1d30),
    ));
    let rest = int_f32(0x1a0_u32.wrapping_sub(height).cast_signed());
    let r = remap_y(h, 0x00b5_67a8);
    let top = store(sub(load(rest), r));
    let r2 = konst(h, 0x00b4_1d9c);
    h.call_sym(
        SHAPE_UV_C2,
        Conv::Cdecl,
        &[
            0xffff_fffe,
            0,
            pass(hf),
            r2,
            pass(top),
            0,
            pass(w),
            q3,
            q4,
            grey,
            grey,
        ],
    );
}

// ---- weapon abilities

/// The weapon's ability plates and names: `MsCalcWeaponCap(weapon)`, a
/// plate (`TODrawMenuPlateXYWHType(x, y', remapX2(0x00b5eff4),
/// remapY2(0x00b3eb9c), kind)`) per slot (`u8 +0xb`), each row `step`
/// lower (`remapY2(step)`), then the four abilities (u16 `+0xe..`; 0 and
/// 0xff skipped) by name at `name_y(y')`, again `step` apart.
struct Abilities {
    kind: u32,
    step: u32,
}

/// `Menu_DrawWeaponAbilities_008d8aa0(x, y, weapon)` (cdecl): plates of
/// kind 7 a row `0x00b5eec0` apart; the names at `y' - remapY2(0x00b92268)`
/// with alpha 0.
pub fn weapon_abilities(h: &mut impl Host, x: u32, y: u32, weapon: u32) {
    let ab = Abilities {
        kind: 7,
        step: 0x00b5_eec0,
    };
    let mut row = pass(y);
    h.call_sym(CALC_WEAPON_CAP, Conv::Cdecl, &[weapon]);
    plates(h, &ab, x, &mut row, weapon);
    let mut row = pass(y);
    for k in 0..4u32 {
        let id = u32::from(h.read::<u16>(weapon.wrapping_add(0xe + 2 * k)));
        if id != 0 && id != 0xff {
            let r = remap_y(h, 0x00b9_2268);
            let ny = store(sub(load(row), r));
            h.call_sym(ABILITY_NAME, Conv::Cdecl, &[id, pass(x), pass(ny), 0]);
        }
        row = plus(remap_y(h, ab.step), row);
    }
}

/// The plates of [`Abilities`] at `x`, from `row` down.
fn plates(h: &mut impl Host, ab: &Abilities, x: u32, row: &mut u32, weapon: u32) {
    let mut i = 0u32;
    while i < u32::from(h.read::<u8>(weapon.wrapping_add(0xb))) {
        let ph = store(remap_y(h, 0x00b3_eb9c));
        let pw = store(remap_x(h, 0x00b5_eff4));
        h.call_sym(
            MENU_PLATE,
            Conv::Cdecl,
            &[pass(x), pass(*row), pw, ph, ab.kind],
        );
        i += 1;
        *row = plus(remap_y(h, ab.step), *row);
    }
}

/// `g_TkMenuEquipItem`.
pub const EQUIP_ITEM: Global<u32> = Global::new(0x0186_a630);
/// Bytes of [`equip_weapon_abilities`]'s frame (the name pointer
/// `MsGetSaveWeapon` fills, the two coordinates).
pub const EQUIP_ABILITIES_FRAME: u32 = 0xc;

/// `MenuEquip_DrawWeaponAbilities_008d02e0()` (cdecl; `frame` its name
/// pointer at `ebp - 0xc`): for the equipment menu's item (`0x0186a630`,
/// not 0xff), the weapon (`MsGetSaveWeapon(item, &name)`) at
/// `(remapX2(0x00b65724), remapY2(0x00b5efcc))`: plates of kind 0xe a row
/// `0x00b65e48` apart, then its abilities at the same rows. Returns the
/// item on the early exit (`Some`); else a void callee's `eax`.
pub fn equip_weapon_abilities(h: &mut impl Host, frame: u32) -> Option<u32> {
    let item = EQUIP_ITEM.get(h);
    if item == 0xff {
        return Some(item);
    }
    let weapon = h.call_sym(SAVE_WEAPON, Conv::Cdecl, &[item, frame]);
    let ab = Abilities {
        kind: 0xe,
        step: 0x00b6_5e48,
    };
    let x = store(remap_x(h, 0x00b6_5724));
    let mut row = store(remap_y(h, 0x00b5_efcc));
    h.call_sym(CALC_WEAPON_CAP, Conv::Cdecl, &[weapon]);
    plates(h, &ab, x, &mut row, weapon);
    let x = store(remap_x(h, 0x00b6_5724));
    let mut row = store(remap_y(h, 0x00b5_efcc));
    for k in 0..4u32 {
        let id = u32::from(h.read::<u16>(weapon.wrapping_add(0xe + 2 * k)));
        if id != 0 && id != 0xff {
            h.call_sym(ABILITY_NAME, Conv::Cdecl, &[id, pass(x), pass(row), 0]);
        }
        row = plus(remap_y(h, ab.step), row);
    }
    None
}

// ---- battle cursors and gauges

/// `0x0133d6e0`: 1 hides the target cursors.
pub const TARGET_HIDE: Global<u32> = Global::new(0x0133_d6e0);
/// `g_BtlTargetSelMask`.
pub const TARGET_MASK: Global<u32> = Global::new(0x0133_d178);
/// `MsGetChrTargetPos(i, &x, &y)` (cdecl).
pub const TARGET_POS: u32 = 0x0079_5920;
/// `TOSetEnvCursor()` (cdecl).
pub const SET_ENV_CURSOR: u32 = 0x0090_46d0;
/// Bytes of [`target_cursor`]'s frame (a float, x, y).
pub const TARGET_FRAME: u32 = 0xc;

/// `TOBtlDrawTargetCursor()` (cdecl; `frame` its locals from `ebp -
/// 0xc`: x at `+4`, y at `+8`): unless hidden (`0x0133d6e0 == 1`), a
/// cross cursor at each selected target (bit `i` of `0x0133d178`, `i <
/// 28`): `MsGetChrTargetPos(i, &x, &y)`, `TOSetEnvCursor()`, then
/// `TkMn2DrawCrossCursor((float)x, (float)y, 4)` for characters (`i <
/// 20`), kind 5 for the rest. The original's `eax` is the last call's or
/// the caller's: none.
pub fn target_cursor(h: &mut impl Host, frame: u32) {
    if TARGET_HIDE.get(h) == 1 {
        return;
    }
    for i in 0..0x1c_u32 {
        if TARGET_MASK.get(h) & (1 << i) == 0 {
            continue;
        }
        h.call_sym(
            TARGET_POS,
            Conv::Cdecl,
            &[i, frame.wrapping_add(4), frame.wrapping_add(8)],
        );
        h.call_sym(SET_ENV_CURSOR, Conv::Cdecl, &[]);
        let y = int_f32(h.read::<i32>(frame.wrapping_add(8)));
        h.write(frame, y);
        let yv = pass(y);
        let x = int_f32(h.read::<i32>(frame.wrapping_add(4)));
        h.write(frame, x);
        let kind = if i < 0x14 { 4 } else { 5 };
        h.call_sym(CROSS_CURSOR, Conv::Cdecl, &[pass(x), yv, kind]);
    }
}

/// `MsGetBattleEndStatus(0)` (cdecl).
pub const BATTLE_END: u32 = 0x0079_28d0;
/// `TOMakePktScissor(x, y, w, h)` (cdecl).
pub const PKT_SCISSOR: u32 = 0x008f_dee0;
/// The enemies' count (`0x0133f0ec`, i32) and ids (u16 from `0x0133f0f0`).
pub const ENEMY_COUNT: Global<u32> = Global::new(0x0133_f0ec);
/// The enemies' ids (u16).
pub const ENEMY_IDS: u32 = 0x0133_f0f0;
/// The enemies' limit gauge words (16-byte records).
pub const ENEMY_GAUGES: u32 = 0x0133_f98c;
/// `MsGetChrLimitBarFlag(id)` (cdecl).
pub const LIMIT_BAR_FLAG: u32 = 0x0079_5250;
/// `MsGetChrGroundPos(id, &x, &y)` (cdecl).
pub const GROUND_POS: u32 = 0x0079_4ff0;
/// `TOBtlDrawStatusLimitGauge(gauge, x, y, w, h)` (cdecl).
pub const LIMIT_GAUGE: u32 = 0x0089_60e0;
/// Bytes of [`enemy_limit`]'s frame (an `f64`, y, a float, x).
pub const ENEMY_LIMIT_FRAME: u32 = 0x14;

/// `TOBtlDrawEnemyLimit()` (cdecl; `frame` its locals from `ebp - 0x14`:
/// y at `+8`, the float at `+0xc`, x at `+0x10`): unless the battle has
/// ended (`MsGetBattleEndStatus(0) == 1`), under a 512 x 416 scissor,
/// each enemy (`0x0133f0ec`, signed, reread) with an id (not 0xff) whose
/// limit bar is on (`MsGetChrLimitBarFlag(id) == 1`) gets its gauge
/// (`TOBtlDrawStatusLimitGauge(+0x0133f98c, (float)x -
/// remapX2(0x00b5ef78) * 0x00b92220, (float)y, remapX2(0x00b5ef78),
/// remapY2(0x00b92314))` at its ground position (`MsGetChrGroundPos`).
/// The original's `eax` is 1 on the early exit, else a call's: none
/// read.
pub fn enemy_limit(h: &mut impl Host, frame: u32) {
    if h.call_sym(BATTLE_END, Conv::Cdecl, &[0]) == 1 {
        return;
    }
    h.call_sym(PKT_SCISSOR, Conv::Cdecl, &[0, 0, 0x200, 0x1a0]);
    let mut i = 0u32;
    while i.cast_signed() < ENEMY_COUNT.get(h).cast_signed() {
        let id = u32::from(h.read::<u16>(h.rebase(ENEMY_IDS).wrapping_add(2 * i)));
        if id != 0xff && h.call_sym(LIMIT_BAR_FLAG, Conv::Cdecl, &[id]) == 1 {
            let (px, py) = (frame.wrapping_add(0x10), frame.wrapping_add(8));
            h.call_sym(GROUND_POS, Conv::Cdecl, &[id, px, py]);
            let xf = int_f32(h.read::<i32>(px));
            h.write(frame.wrapping_add(0xc), xf);
            let xd = load(xf);
            h.write(frame, xd);
            let height = store(remap_y(h, 0x00b9_2314));
            let width = store(remap_x(h, 0x00b5_ef78));
            let yf = int_f32(h.read::<i32>(py));
            h.write(frame.wrapping_add(0xc), yf);
            let r = remap_x(h, 0x00b5_ef78);
            let left = store(sub(xd, mul(r, konst64(h, 0x00b9_2220))));
            h.write(frame.wrapping_add(0xc), left);
            let gauge: u32 = h.read(h.rebase(ENEMY_GAUGES).wrapping_add(16 * i));
            h.call_sym(
                LIMIT_GAUGE,
                Conv::Cdecl,
                &[gauge, pass(left), pass(yf), width, height],
            );
        }
        i += 1;
    }
}
