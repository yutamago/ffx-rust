//! Reading a family member's operands in order (round 6, the shape
//! families). The copies of one family differ in immediates whose
//! encoding can change length (`push imm8` against `push imm32`), so the
//! ports walk the code with a cursor instead of fixed offsets: each
//! method reads the operand of the instruction at the cursor and steps
//! over it. The instruction kinds are those the family's code identity
//! fixes; only the operands move.

use crate::Image;

/// A position in a member's code.
#[derive(Debug, Clone, Copy)]
pub struct Cursor<'a> {
    pub img: &'a Image,
    pub at: u32,
}

impl<'a> Cursor<'a> {
    #[must_use]
    pub fn new(img: &'a Image, at: u32) -> Cursor<'a> {
        Cursor { img, at }
    }

    fn byte(&self, off: u32) -> u8 {
        self.img.word(self.at + off) as u8
    }

    /// Steps over `n` bytes (an instruction without a varying operand).
    pub fn skip(&mut self, n: u32) -> &mut Self {
        self.at += n;
        self
    }

    /// The 32-bit word at `+off` from the cursor, then `n` bytes on (an
    /// instruction whose operand is a fixed-width word).
    pub fn word(&mut self, off: u32, n: u32) -> u32 {
        let v = self.img.word(self.at + off);
        self.at += n;
        v
    }

    /// The signed 8-bit operand at `+off` sign-extended, then `n` bytes on.
    pub fn disp8(&mut self, off: u32, n: u32) -> u32 {
        let v = i32::from(self.byte(off).cast_signed()).cast_unsigned();
        self.at += n;
        v
    }

    /// `push imm8` (`6a ib`, sign-extended) or `push imm32` (`68 id`):
    /// the word pushed.
    pub fn push_imm(&mut self) -> u32 {
        if self.byte(0) == 0x6a {
            self.disp8(1, 2)
        } else {
            self.word(1, 5)
        }
    }

    /// `mov eax, [eax + disp32]` (`8b 80 id`) on the slot table, or the
    /// `disp8` form (`8b 40 ib`) for slots below 32: the slot.
    pub fn slot_load(&mut self) -> u32 {
        if self.byte(1) == 0x40 {
            self.disp8(2, 3) / 4
        } else {
            self.word(2, 6) / 4
        }
    }

    /// `mov dword [esi + disp32], imm32` (`c7 86 id id`) or with a `disp8`
    /// (`c7 46 ib id`): `(disp, value)`.
    pub fn store_imm(&mut self) -> (u32, u32) {
        if self.byte(1) & 0xc0 == 0x40 {
            let d = i32::from(self.byte(2).cast_signed()).cast_unsigned();
            let v = self.word(3, 7);
            (d, v)
        } else {
            let d = self.img.word(self.at + 2);
            let v = self.word(6, 10);
            (d, v)
        }
    }
}
