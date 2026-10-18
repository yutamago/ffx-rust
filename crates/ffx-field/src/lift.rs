//! A small emulation of the x86 state lifted functions need: the
//! registers, the flags of the last test or compare, the x87 control and
//! status words, and the x87 stack as `f64`s at the game's 53-bit
//! precision (field agent r3; `ffx-work/field/lift/lift.py` writes the
//! state machines that use it).

use ffx_bridge::Mem;

/// The emulated registers and flags.
#[derive(Default)]
pub(crate) struct Regs {
    pub(crate) eax: u32,
    pub(crate) ebx: u32,
    pub(crate) ecx: u32,
    pub(crate) edx: u32,
    pub(crate) esi: u32,
    pub(crate) edi: u32,
    pub(crate) esp: u32,
    pub(crate) ebp: u32,
    /// The x87 control word (`fnstcw`/`fldcw`).
    pub(crate) cw: u16,
    /// The x87 condition bits as `fnstsw` puts them in `ah`.
    pub(crate) sw: u8,
    /// The last flag-setting operation: (result or left, right, `ah` test).
    pub(crate) flag: (u32, u32, bool),
    /// Whether it was a `cmp`.
    pub(crate) cmp: bool,
}

impl Regs {
    pub(crate) fn res(&self) -> u32 {
        if self.cmp {
            self.flag.0.wrapping_sub(self.flag.1)
        } else {
            self.flag.0
        }
    }
    pub(crate) fn zf(&self) -> bool {
        self.res() == 0
    }
    pub(crate) fn sf(&self) -> bool {
        self.res().cast_signed() < 0
    }
    pub(crate) fn pf(&self) -> bool {
        (self.res() & 0xff).count_ones().is_multiple_of(2)
    }
    /// `ja`: unsigned above (`cmp`), else not zero.
    pub(crate) fn above(&self) -> bool {
        if self.cmp {
            self.flag.0 > self.flag.1
        } else {
            !self.zf()
        }
    }

    #[allow(dead_code)] // the lifts so far test only above
    /// `jb`: unsigned below (`cmp`); a test clears the carry.
    pub(crate) fn below(&self) -> bool {
        self.cmp && self.flag.0 < self.flag.1
    }

    pub(crate) fn lt(&self) -> bool {
        if self.cmp {
            self.flag.0.cast_signed() < self.flag.1.cast_signed()
        } else {
            self.sf()
        }
    }
}

pub(crate) fn ld32(mem: &impl Mem, a: u32) -> f64 {
    crate::x87::load(mem.read::<u32>(a))
}

pub(crate) fn st0(st: &[f64]) -> f64 {
    sti(st, 0)
}

pub(crate) fn sti(st: &[f64], i: usize) -> f64 {
    st[st.len() - 1 - i]
}

pub(crate) fn set(st: &mut [f64], i: usize, v: f64) {
    let n = st.len();
    st[n - 1 - i] = v;
}

/// `fcom`: the condition bits of `a` against `b` (C0 below, C3 equal,
/// all three unordered).
#[allow(clippy::float_cmp)] // fcom: exact
pub(crate) fn fcom(a: f64, b: f64) -> u8 {
    if a.is_nan() || b.is_nan() {
        0x45
    } else if a < b {
        0x01
    } else if a == b {
        0x40
    } else {
        0
    }
}

/// `fistp qword` under control word `cw`'s rounding; the integer
/// indefinite outside `i64`.
#[allow(clippy::cast_possible_truncation)] // in range, checked
pub(crate) fn fist64(x: f64, cw: u16) -> u64 {
    let r = match (cw >> 10) & 3 {
        0 => x.round_ties_even(),
        1 => x.floor(),
        2 => x.ceil(),
        _ => x.trunc(),
    };
    if r.is_nan() || r >= 9_223_372_036_854_775_808.0 || r < -9_223_372_036_854_775_808.0 {
        0x8000_0000_0000_0000
    } else {
        (r as i64).cast_unsigned()
    }
}
