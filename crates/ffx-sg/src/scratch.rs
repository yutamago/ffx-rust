//! Stack locals for bridges that pass them to other bridges: a
//! [`Scratch`] memory maps a few addresses in the top page (never the
//! game's: user space ends below it) to a local buffer and forwards every
//! other access, so an original's `lea eax, [ebp-0x14]; push eax` becomes
//! a call with [`SCRATCH`].

use ffx_bridge::Mem;

/// The first scratch address.
pub const SCRATCH: u32 = 0xffff_f000;
/// Scratch bytes.
const LEN: usize = 0x200;

/// A memory with [`LEN`] local bytes at [`SCRATCH`].
pub struct Scratch<'a, M: Mem> {
    mem: &'a mut M,
    buf: [u8; LEN],
}

impl<'a, M: Mem> Scratch<'a, M> {
    /// Zeroed locals over `mem`.
    pub fn new(mem: &'a mut M) -> Self {
        Self { mem, buf: [0; LEN] }
    }

    fn local(addr: u32, len: usize) -> Option<usize> {
        let off = usize::try_from(addr.checked_sub(SCRATCH)?).ok()?;
        (off + len <= LEN).then_some(off)
    }
}

impl<M: Mem> Mem for Scratch<'_, M> {
    fn read_bytes(&self, addr: u32, out: &mut [u8]) {
        match Self::local(addr, out.len()) {
            Some(off) => out.copy_from_slice(&self.buf[off..off + out.len()]),
            None => self.mem.read_bytes(addr, out),
        }
    }

    fn write_bytes(&mut self, addr: u32, data: &[u8]) {
        match Self::local(addr, data.len()) {
            Some(off) => self.buf[off..off + data.len()].copy_from_slice(data),
            None => self.mem.write_bytes(addr, data),
        }
    }

    fn image_base(&self) -> u32 {
        self.mem.image_base()
    }
}

/// Calls go to the game: no scratch address reaches a callee.
impl<M: ffx_bridge::Host> ffx_bridge::Host for Scratch<'_, M> {
    fn call(&mut self, addr: u32, args: &[u32]) -> u32 {
        self.mem.call(addr, args)
    }
}
