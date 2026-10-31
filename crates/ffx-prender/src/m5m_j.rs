//! M5 stage 1, list `map`, round 5 batch 6: the rcui number window's
//! create (`rcuiNumWindowCreate_0093c670`): a window over a float, split
//! into a whole part (0..=999) and hundredths (0..=99).
//!
//! The whole part `modf` writes lives in the original's frame and comes in
//! through its `ebp` (the shim passes a `StackFrame`).

/// The frame: `ebp` minus its lowest local.
pub const FRAME: u32 = 8;
/// The window's procedure, `g_HundredF64`, `MSVCR110!modf` (the slot).
pub const WINDOW_PROC: u32 = 0x0093_c860;
pub const HUNDRED: u32 = 0x00b9_22e0;
pub const MODF_SLOT: u32 = 0x00b0_c49c;

// ---- callees ----

/// `rcuiWindowCreate(holder, x, y, w, h, flags, proc, title, parent)`,
/// `Gu_WindowSetArg(window, arg)` (cdecl).
pub const WINDOW_CREATE: u32 = 0x0093_7ef0;
pub const WINDOW_SET_ARG: u32 = 0x0084_b560;

pub mod bridge {
    use super::{HUNDRED, MODF_SLOT, WINDOW_CREATE, WINDOW_PROC, WINDOW_SET_ARG};
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    fn f64_words(x: f64) -> [u32; 2] {
        let b = x.to_bits().to_le_bytes();
        [
            u32::from_le_bytes([b[0], b[1], b[2], b[3]]),
            u32::from_le_bytes([b[4], b[5], b[6], b[7]]),
        ]
    }

    /// `rcuiNumWindowCreate_0093c670(win, value, title, parent)` -> the
    /// whole part: the window (8, 8, 0xbe x 0x7d, flags 6, its procedure)
    /// with `win` as its argument; `win +4` the value's address, `+8` the
    /// value, `+0xc` its whole part (`modf`, toward zero) clamped to
    /// 0..=999, `+0x10` its fraction times 100 (toward zero) clamped to
    /// 0..=99, `+0x14` and `+0x1c` cleared.
    pub fn num_window_create(
        h: &mut impl Host,
        ebp: u32,
        win: u32,
        value: u32,
        title: u32,
        parent: u32,
    ) -> u32 {
        let proc_ = h.rebase(WINDOW_PROC);
        h.call_sym(
            WINDOW_CREATE,
            Conv::Cdecl,
            &[win, 8, 8, 0xbe, 0x7d, 6, proc_, title, parent],
        );
        let window: u32 = h.read(win);
        h.call_sym(WINDOW_SET_ARG, Conv::Cdecl, &[window, win]);
        h.write(win + 4, value);
        let bits: u32 = h.read(value);
        // fld / fstp dword: only a signalling NaN changes
        h.write(win + 8, x87::store(x87::load(bits)));
        let [lo, hi] = f64_words(x87::load(bits));
        let whole_at = ebp - 8;
        let modf: u32 = h.read(h.rebase(MODF_SLOT));
        let fraction = h.call_st0_f64(modf, Conv::Cdecl, &[lo, hi, whole_at]);
        let hundred = f64::from_bits(h.read::<u64>(h.rebase(HUNDRED)));
        let hundredths = x87::ftol(h, x87::mul(fraction, hundred));
        h.write(win + 0x10, hundredths);
        let whole = f64::from_bits(h.read::<u64>(whole_at));
        let whole = x87::ftol(h, whole).cast_signed();
        h.write(win + 0xc, whole.cast_unsigned());
        h.write(win + 0x14, 0_u32);
        h.write(win + 0x1c, 0_u32);
        let whole = whole.clamp(0, 999).cast_unsigned();
        h.write(win + 0xc, whole);
        let hundredths = hundredths.cast_signed().clamp(0, 99).cast_unsigned();
        h.write(win + 0x10, hundredths);
        whole
    }
}
