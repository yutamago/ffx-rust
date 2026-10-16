//! The debug print buffer (`TODaps*`): text and position codes collected in
//! a 0x800-byte buffer, its fill level a 16-bit count. Appends stop at 0x7c0
//! bytes; a position code is five bytes, `8` then x and y each as hundreds
//! and the rest.

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    /// The buffer (`0x0133fab0`, 0x800 bytes).
    pub const BUFFER: Global<[u8; 0x800]> = Global::new(0x0133_fab0);
    /// Bytes used (`0x0133faa0`, i16).
    pub const USED: Global<i16> = Global::new(0x0133_faa0);
    /// Two 16-bit settings `TODapsInit` sets to 10 (`0x013402b0`,
    /// `0x013402b4`).
    pub const SETTING_A: Global<i16> = Global::new(0x0134_02b0);
    /// See [`SETTING_A`].
    pub const SETTING_B: Global<i16> = Global::new(0x0134_02b4);
    /// Appends stop here.
    pub const LIMIT: i32 = 0x7c0;

    fn at(mem: &impl Mem, i: i32) -> u32 {
        BUFFER.ptr(mem).addr.wrapping_add(i.cast_unsigned())
    }

    /// `TODapsInit()` (`0x0089dc40`) and `TODapsDraw()` (`0x0089dc10`,
    /// the same code): the buffer cleared, both settings 10; returns 10.
    pub fn init(mem: &mut impl Mem) -> u32 {
        let b = BUFFER.ptr(mem).addr;
        for k in 0..0x800 {
            mem.write(b + k, 0u8);
        }
        USED.set(mem, 0);
        SETTING_A.set(mem, 10);
        SETTING_B.set(mem, 10);
        10
    }

    /// `TODapsString(s)` (`0x0089dec0`): `s` with its terminator appended
    /// unless empty or reaching [`LIMIT`]; the count grows by its length.
    /// Returns what `eax` ends as: `s`, or past the copied terminator.
    pub fn string(mem: &mut impl Mem, s: u32) -> u32 {
        let mut len = 0u32;
        while mem.read::<u8>(s.wrapping_add(len)) != 0 {
            len += 1;
        }
        let used = USED.get(mem);
        if len == 0 || i32::from(used).wrapping_add(len.cast_signed()) >= LIMIT {
            return s;
        }
        let dest = at(mem, i32::from(used));
        let mut k = 0;
        loop {
            let c: u8 = mem.read(s.wrapping_add(k));
            mem.write(dest.wrapping_add(k), c);
            k += 1;
            if c == 0 {
                break;
            }
        }
        #[allow(clippy::cast_possible_truncation)] // the original adds 16 bits
        USED.set(mem, used.wrapping_add(len as i16));
        s.wrapping_add(k)
    }

    /// `TODapsSetPos(x, y)` (`0x0089de30`): unless the buffer is at
    /// [`LIMIT`], the code `8, x / 100, x % 100, y / 100, y % 100` (bytes,
    /// truncating division) appended. Returns what `eax` ends as: 0x7c0
    /// when full, else the old count plus 4.
    pub fn set_pos(mem: &mut impl Mem, x: i32, y: i32) -> u32 {
        let used = USED.get(mem);
        if i32::from(used) >= LIMIT {
            return LIMIT.cast_unsigned();
        }
        let mut code = [8u8; 5];
        for (k, v) in [x, y].into_iter().enumerate() {
            let q = v / 100;
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // bytes
            {
                code[1 + 2 * k] = q as u8;
                code[2 + 2 * k] = (v as u8).wrapping_sub((q as u8).wrapping_mul(100));
            }
        }
        let mut pos = used;
        for c in code {
            let a = at(mem, i32::from(pos));
            mem.write(a, c);
            pos = pos.wrapping_add(1);
        }
        USED.set(mem, pos);
        i32::from(used.wrapping_add(4)).cast_unsigned()
    }
}
