//! The application's viewport fit and notifier removal (list
//! `phyre-framework-input`, batch 8).
//!
//! `PApplication::setViewportKeepingAspect` letterboxes or pillarboxes a
//! rectangle to the application's aspect ratio (`+0x10`, `f32`) with the
//! x87 at 53-bit precision: unsigned sizes become `f32` (`fild` plus 2^32
//! for the top bit, stored as `f32`), quotients and products are
//! truncated (`fistp qword` under round-toward-zero) and the low 32 bits
//! kept.

use ffx_bridge::x87;
use ffx_graphic::host::{Conv, Host};

/// The notifier map's holder (`+0` the head).
pub const NOTIFIERS: u32 = 0x00cc_9ee4;
/// `std::map::equal_range(&out, &key)` (thiscall, `ret 8`).
pub const MAP_EQUAL_RANGE: u32 = 0x0062_58e0;
/// `std::map::erase(&out, first, last)` (thiscall, `ret 0xc`; in the
/// list).
pub const MAP_ERASE_RANGE: u32 = 0x0062_7130;

/// An unsigned word as the originals load it: `fild` (signed) plus
/// `2^32` when the top bit is set, stored to `f32`.
#[must_use]
pub fn u32_to_f32(v: u32) -> f32 {
    let mut x = f64::from(v as i32);
    if (v as i32) < 0 {
        x = x87::add(x, 4_294_967_296.0);
    }
    f32::from_bits(x87::store(x))
}

/// `fistp qword` under round-toward-zero: the integer part, or the
/// integer indefinite (`0x8000_0000_0000_0000`) for NaNs and values out
/// of the 64-bit range; the low 32 bits.
#[must_use]
pub fn trunc_low32(x: f64) -> u32 {
    const LIMIT: f64 = 9_223_372_036_854_775_808.0;
    if x.is_nan() || !(-LIMIT..LIMIT).contains(&x) {
        return 0;
    }
    #[allow(clippy::cast_possible_truncation)] // in range, checked above
    let i = x.trunc() as i64;
    i as u32
}

/// The rectangle `PApplication::setViewportKeepingAspect` stores for `(x,
/// y, w, h)` and the aspect `aspect`.
#[must_use]
pub fn fitted(aspect: f32, x: u32, y: u32, w: u32, h: u32) -> [u32; 4] {
    // fucompp of 1.0: exactly equal (a NaN is not)
    if aspect.to_bits() == 1f32.to_bits() {
        return [x, y, w, h];
    }
    let a = f64::from(aspect);
    // the height the width asks for, and the bars above and below
    let need_h = trunc_low32(x87::div(f64::from(u32_to_f32(w)), a));
    let bar_h = h.wrapping_sub(need_h);
    let top = trunc_low32(x87::mul(f64::from(u32_to_f32(bar_h)), 0.5)).wrapping_add(y);
    // the width the height asks for, and the bars left and right
    let need_w = trunc_low32(x87::mul(f64::from(u32_to_f32(h)), a));
    let bar_w = w.wrapping_sub(need_w);
    let left = trunc_low32(x87::mul(0.5, f64::from(u32_to_f32(bar_w)))).wrapping_add(x);
    if top != 0 && need_h < h {
        [x, top, w, need_h]
    } else {
        [left, y, need_w, h]
    }
}

pub mod bridge {
    use super::{Conv, Host, MAP_EQUAL_RANGE, MAP_ERASE_RANGE, NOTIFIERS, fitted};

    /// `PApplication::setViewportKeepingAspect(x, y, w, h)` (`0x0062cba0`,
    /// `ret 0x10`): the fitted rectangle to `+0x14..+0x20`, then the
    /// virtual `+0x10` (apply); its `eax`.
    pub fn set_viewport_keeping_aspect(h: &mut impl Host, this: u32, rect: [u32; 4]) -> u32 {
        let aspect = f32::from_bits(h.read(this + 0x10));
        let [x, y, w, hh] = rect;
        let r = fitted(aspect, x, y, w, hh);
        for (i, v) in r.into_iter().enumerate() {
            h.write(this + 0x14 + 4 * i as u32, v);
        }
        h.call_virtual(this, 4, &[])
    }

    /// `0x0062bce0` (cdecl `(key)`): every notifier registered under
    /// `key` (the caller's argument word at `key_at`) removed from the
    /// map: its equal range (into the frame block `frame`, 8 bytes), then
    /// the range erased (its iterator into `frame + 4`); the erase's
    /// `eax`.
    pub fn remove_notifier(h: &mut impl Host, key_at: u32, frame: u32) -> u32 {
        let map = h.rebase(NOTIFIERS);
        let range = h.call_sym(MAP_EQUAL_RANGE, Conv::Thiscall(map), &[frame, key_at]);
        let first: u32 = h.read(range);
        let last: u32 = h.read(range + 4);
        h.call_sym(
            MAP_ERASE_RANGE,
            Conv::Thiscall(map),
            &[frame + 4, first, last],
        )
    }
}
