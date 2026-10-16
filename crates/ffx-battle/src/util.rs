//! Small helpers of the battle code (`docs/rewrite-plan.md`, M2): memory
//! fills, the battle's string copy, key repeat, effect IDs.

/// The length `MsStrCopy` copies of a string: bytes before the first
/// `0xff`, 0, `\r` or `\n`.
#[must_use]
pub fn str_len(bytes: impl IntoIterator<Item = u8>) -> i32 {
    let mut n = 0i32;
    for b in bytes {
        if matches!(b, 0xff | 0 | b'\r' | b'\n') {
            break;
        }
        n = n.wrapping_add(1);
    }
    n
}

/// How many bytes `MsStrCopy` copies of a string of `len` into a buffer of
/// `max` (signed): `min(len + 1, max) - 1`; below 1 nothing.
#[must_use]
pub const fn copy_len(len: i32, max: i32) -> i32 {
    let n = len.wrapping_add(1);
    let n = if n > max { max } else { n };
    n.wrapping_sub(1)
}

/// `MsGetEffID(group, a, b)` (`0x00788df0`): an effect ID, `group & 0xfff`
/// in the high half, then the low bytes of `a` and `b`.
#[must_use]
pub const fn eff_id(group: u32, a: u32, b: u32) -> u32 {
    ((group & 0xfff) << 16) | ((a & 0xff) << 8) | (b & 0xff)
}

/// A key repeat (`MsAutoRepeatInit`): the last input, frames it has been
/// held, the frames to wait, and the first and later waits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AutoRepeat {
    /// `+0x0`: last input, -1 none.
    pub last: i32,
    /// `+0x4`: frames since it fired.
    pub held: i32,
    /// `+0x8`: frames to wait now.
    pub wait: i32,
    /// `+0xc`: first wait.
    pub first: i32,
    /// `+0x10`: later waits.
    pub next: i32,
}

impl AutoRepeat {
    /// `MsAutoRepeatInit`.
    #[must_use]
    pub const fn new(first: i32, next: i32) -> Self {
        Self {
            last: -1,
            held: 0,
            wait: first,
            first,
            next,
        }
    }

    /// `MsAutoRepeatCheck`: -1 when `input` fires (it changed, or was held
    /// longer than the wait), else 0.
    pub const fn check(&mut self, input: i32) -> i32 {
        if self.last != input {
            self.wait = self.first;
            self.held = 0;
            self.last = input;
            return -1;
        }
        self.held = self.held.wrapping_add(1);
        if self.held > self.wait {
            self.wait = self.next;
            self.held = 0;
            self.last = input;
            return -1;
        }
        self.last = input;
        0
    }
}

/// The helpers on game memory, through [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::Mem;

    use super::{AutoRepeat, copy_len, eff_id, str_len};

    /// `MsStructClear(p, n)` (`0x0079f230`): zeroes `n` bytes (nothing
    /// unless `n` is above 0).
    pub fn ms_struct_clear(mem: &mut impl Mem, p: u32, n: i32) {
        if n > 0 {
            mem.write_bytes(p, &vec![0; n.cast_unsigned() as usize]);
        }
    }

    /// `MsStructFill(p, n, v)` (`0x0079f260`): fills `n / 2` (signed)
    /// halfwords with `v`.
    pub fn ms_struct_fill(mem: &mut impl Mem, p: u32, n: i32, v: u16) {
        let count = n / 2;
        if count > 0 {
            let bytes: Vec<u8> = (0..count).flat_map(|_| v.to_le_bytes()).collect();
            mem.write_bytes(p, &bytes);
        }
    }

    /// `MsStrCopy(src, dst, max)` (`0x007cf6c0`): copies [`copy_len`] bytes
    /// of the string at `src` to `dst` (forward, byte by byte, like the
    /// original), ends it with 0 and returns the length copied.
    pub fn ms_str_copy(mem: &mut impl Mem, src: u32, dst: u32, max: i32) -> i32 {
        let len = str_len((0u32..).map(|i| mem.read::<u8>(src.wrapping_add(i))));
        let n = copy_len(len, max);
        let mut end = dst;
        if n >= 1 {
            for i in 0..n.cast_unsigned() {
                let b: u8 = mem.read(src.wrapping_add(i));
                mem.write(dst.wrapping_add(i), b);
            }
            end = dst.wrapping_add(n.cast_unsigned());
        }
        mem.write(end, 0u8);
        n
    }

    fn get(mem: &impl Mem, p: u32) -> AutoRepeat {
        let w: [i32; 5] = mem.read(p);
        AutoRepeat {
            last: w[0],
            held: w[1],
            wait: w[2],
            first: w[3],
            next: w[4],
        }
    }

    /// `MsAutoRepeatInit(p, first, next)` (`0x007c89a0`).
    pub fn ms_auto_repeat_init(mem: &mut impl Mem, p: u32, first: i32, next: i32) {
        let r = AutoRepeat::new(first, next);
        mem.write(p, [r.last, r.held, r.wait, r.first, r.next]);
    }

    /// `MsAutoRepeatCheck(p, input)` (`0x007c8950`): writes the first three
    /// words back.
    pub fn ms_auto_repeat_check(mem: &mut impl Mem, p: u32, input: i32) -> i32 {
        let mut r = get(mem, p);
        let fired = r.check(input);
        mem.write(p, [r.last, r.held, r.wait]);
        fired
    }

    /// `MsGetEffID(group, a, b)` (`0x00788df0`).
    #[must_use]
    pub fn ms_get_eff_id(group: u32, a: u32, b: u32) -> u32 {
        eff_id(group, a, b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copies_stop_at_terminators() {
        assert_eq!(str_len(*b"ab\ncd"), 2);
        assert_eq!(str_len([1, 2, 0xff, 3]), 2);
        assert_eq!(copy_len(5, 3), 2);
        assert_eq!(copy_len(2, 100), 2);
        assert_eq!(copy_len(2, 0), -1);
        assert_eq!(eff_id(0x1234, 0x156, 0x78), 0x0234_5678);
    }

    #[test]
    fn repeat_fires_on_change_then_after_waits() {
        let mut r = AutoRepeat::new(2, 1);
        assert_eq!(r.check(5), -1);
        assert_eq!(r.check(5), 0);
        assert_eq!(r.check(5), 0);
        assert_eq!(r.check(5), -1);
        assert_eq!(r.check(5), 0);
        assert_eq!(r.check(5), -1);
    }
}
