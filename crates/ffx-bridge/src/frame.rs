//! A ported function's locals that its callees take pointers to.
//!
//! The original keeps them on its stack and passes their addresses; the
//! Rust bridge passes addresses too (the callees are bridges on [`Mem`]), at
//! a range no game memory uses, backed by a host buffer: [`Frame`] is a
//! [`Mem`] that serves that range itself and the rest from the game's
//! memory. Nothing lands in the game (or in an oracle's memory), as nothing
//! visible landed there from the original's stack.

use crate::Mem;

/// Game memory plus `len` zeroed bytes at `base`.
pub struct Frame<'m, M: Mem> {
    mem: &'m mut M,
    base: u32,
    bytes: Vec<u8>,
}

impl<'m, M: Mem> Frame<'m, M> {
    /// `len` bytes at `base`: an address range the game never maps (the
    /// top of the 32-bit space, which is the kernel's). A function that
    /// calls another with a frame picks a different `base`.
    pub fn new(mem: &'m mut M, base: u32, len: usize) -> Self {
        Self {
            mem,
            base,
            bytes: vec![0; len],
        }
    }

    /// The address of byte `offset` of the frame.
    #[must_use]
    pub fn at(&self, offset: u32) -> u32 {
        self.base + offset
    }

    /// The game's memory itself.
    pub fn inner(&mut self) -> &mut M {
        self.mem
    }

    fn local(&self, addr: u32, len: usize) -> Option<std::ops::Range<usize>> {
        let start = addr.checked_sub(self.base)? as usize;
        (start + len <= self.bytes.len()).then_some(start..start + len)
    }
}

impl<M: Mem> Mem for Frame<'_, M> {
    fn read_bytes(&self, addr: u32, out: &mut [u8]) {
        match self.local(addr, out.len()) {
            Some(r) => out.copy_from_slice(&self.bytes[r]),
            None => self.mem.read_bytes(addr, out),
        }
    }
    fn write_bytes(&mut self, addr: u32, data: &[u8]) {
        match self.local(addr, data.len()) {
            Some(r) => self.bytes[r].copy_from_slice(data),
            None => self.mem.write_bytes(addr, data),
        }
    }
    fn image_base(&self) -> u32 {
        self.mem.image_base()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::VecMem;

    #[test]
    fn locals_stay_in_the_frame() {
        let mut m = VecMem::new(0x1000, 0x100);
        let mut f = Frame::new(&mut m, 0xfff0_0000, 0x10);
        let a = f.at(4);
        f.write(a, 7u32);
        f.write(0x1010, 9u32);
        assert_eq!(f.read::<u32>(a), 7);
        assert_eq!(m.read::<u32>(0x1010), 9);
        assert!(!m.bytes.contains(&7));
    }
}
