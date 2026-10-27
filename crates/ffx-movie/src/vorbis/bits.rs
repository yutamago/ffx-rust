//! libogg's `bitwise.c` read side (`oggpack_*`, `0x0042e1d0..0x0042ee40`
//! in FFX.exe): LSb-first bit reading with libogg's end-of-packet rules.
//! C `long` is 32 bits on the game's target, so a 32-bit read can come
//! back negative, as there.

const MASK: [u32; 33] = {
    let mut m = [0u32; 33];
    let mut i = 1;
    while i < 33 {
        m[i] = if i == 32 { u32::MAX } else { (1u32 << i) - 1 };
        i += 1;
    }
    m
};

/// `oggpack_buffer` for reading.
#[derive(Debug, Clone)]
pub struct Reader<'a> {
    buf: &'a [u8],
    /// `storage`: the byte count the caller gave (may exceed `buf`; bytes
    /// past it read as 0).
    storage: i32,
    endbyte: i32,
    endbit: i32,
}

impl<'a> Reader<'a> {
    /// `oggpack_readinit(b, buf, bytes)`.
    #[must_use]
    pub fn new(buf: &'a [u8], bytes: i32) -> Self {
        Self {
            buf,
            storage: bytes,
            endbyte: 0,
            endbit: 0,
        }
    }

    /// A reader over a whole slice.
    #[must_use]
    pub fn over(buf: &'a [u8]) -> Self {
        Self::new(buf, i32::try_from(buf.len()).unwrap_or(i32::MAX))
    }

    fn byte(&self, k: i32) -> u32 {
        usize::try_from(self.endbyte + k)
            .ok()
            .and_then(|i| self.buf.get(i))
            .map_or(0, |&b| u32::from(b))
    }

    fn gather(&self, bits: i32) -> u32 {
        let e = self.endbit.cast_unsigned();
        let mut ret = self.byte(0) >> e;
        if bits > 8 {
            ret |= self.byte(1) << (8 - e);
            if bits > 16 {
                ret |= self.byte(2) << (16 - e);
                if bits > 24 {
                    ret |= self.byte(3).wrapping_shl(24 - e);
                    if bits > 32 && e != 0 {
                        ret |= self.byte(4).wrapping_shl(32 - e);
                    }
                }
            }
        }
        ret
    }

    fn overflow(&mut self) -> i32 {
        self.endbyte = self.storage;
        self.endbit = 1;
        -1
    }

    /// `oggpack_look`: the next `bits` bits without advancing, -1 past
    /// the end.
    #[must_use]
    pub fn look(&self, bits: i32) -> i32 {
        if !(0..=32).contains(&bits) {
            return -1;
        }
        let m = MASK[bits.cast_unsigned() as usize];
        let bits = bits + self.endbit;
        if self.endbyte >= self.storage - 4 {
            if self.endbyte > self.storage - ((bits + 7) >> 3) {
                return -1;
            } else if bits == 0 {
                return 0;
            }
        }
        (m & self.gather(bits)).cast_signed()
    }

    /// `oggpack_adv`.
    pub fn adv(&mut self, bits: i32) {
        let bits = bits + self.endbit;
        if self.endbyte > self.storage - ((bits + 7) >> 3) {
            self.overflow();
            return;
        }
        self.endbyte += bits / 8;
        self.endbit = bits & 7;
    }

    /// `oggpack_read`: `bits` (0..=32) bits, -1 past the end.
    pub fn read(&mut self, bits: i32) -> i32 {
        if !(0..=32).contains(&bits) {
            return self.overflow();
        }
        let m = MASK[bits.cast_unsigned() as usize];
        let bits = bits + self.endbit;
        if self.endbyte >= self.storage - 4 {
            if self.endbyte > self.storage - ((bits + 7) >> 3) {
                return self.overflow();
            } else if bits == 0 {
                return 0;
            }
        }
        let ret = (self.gather(bits) & m).cast_signed();
        self.endbyte += bits / 8;
        self.endbit = bits & 7;
        ret
    }

    /// `oggpack_bytes`: the bytes read so far, a partial one counted.
    #[must_use]
    pub fn bytes(&self) -> i32 {
        self.endbyte + (self.endbit + 7) / 8
    }

    /// `opb->storage`.
    #[must_use]
    pub fn storage(&self) -> i32 {
        self.storage
    }
}

#[cfg(test)]
mod tests {
    use super::Reader;

    #[test]
    fn lsb_first_and_end() {
        let b = [0b1010_1101u8, 0xff];
        let mut r = Reader::over(&b);
        assert_eq!(r.read(1), 1);
        assert_eq!(r.read(3), 0b110);
        assert_eq!(r.read(8), 0xfa);
        assert_eq!(r.bytes(), 2);
        assert_eq!(r.read(4), 0xf);
        assert_eq!(r.read(1), -1);
        assert_eq!(r.read(0), -1);
    }
}
