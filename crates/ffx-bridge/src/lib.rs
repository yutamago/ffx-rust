//! Game memory as Rust replacements see it (`docs/rewrite-plan.md`).
//!
//! While original code still shares a piece of state, a Rust replacement has
//! to read and write it where the game keeps it: at 32-bit addresses, in the
//! original layouts. It does so only through [`Mem`], never through host
//! pointers, so the same code runs
//! - in the game (`ffx-hook`, i686): [`RawMem`], plain loads and stores;
//! - in `ffx-oracle` (any host): emulator memory loaded from a snapshot of
//!   the running game, next to the original machine code run on the same
//!   snapshot, so their writes can be compared byte for byte.
//!
//! Addresses from `symbols/ffx.sym` assume the preferred image base
//! [`IMAGE_BASE`]; [`Global`] rebases them through [`Mem::image_base`]
//! (the game runs with ASLR). Heap addresses are used as they are.

pub mod snapshot;

use std::marker::PhantomData;

/// Image base the addresses in `symbols/ffx.sym` are relative to.
pub const IMAGE_BASE: u32 = 0x0040_0000;

/// A plain value with a fixed little-endian layout in game memory.
pub trait Value: Copy {
    /// Size in bytes.
    const SIZE: usize;
    /// Decodes from exactly [`Value::SIZE`] bytes.
    fn from_bytes(b: &[u8]) -> Self;
    /// Encodes into exactly [`Value::SIZE`] bytes.
    fn to_bytes(self, out: &mut [u8]);
}

macro_rules! value {
    ($($t:ty),*) => {$(
        impl Value for $t {
            const SIZE: usize = size_of::<$t>();
            fn from_bytes(b: &[u8]) -> Self {
                <$t>::from_le_bytes(b.try_into().expect("size checked by the caller"))
            }
            fn to_bytes(self, out: &mut [u8]) {
                out.copy_from_slice(&self.to_le_bytes());
            }
        }
    )*};
}
value!(u8, i8, u16, i16, u32, i32, u64, i64, f32, f64);

impl<T: Value, const N: usize> Value for [T; N] {
    const SIZE: usize = T::SIZE * N;
    fn from_bytes(b: &[u8]) -> Self {
        std::array::from_fn(|i| T::from_bytes(&b[i * T::SIZE..(i + 1) * T::SIZE]))
    }
    fn to_bytes(self, out: &mut [u8]) {
        for (i, v) in self.into_iter().enumerate() {
            v.to_bytes(&mut out[i * T::SIZE..(i + 1) * T::SIZE]);
        }
    }
}

/// The game's address space.
pub trait Mem {
    /// Fills `out` from `addr`.
    fn read_bytes(&self, addr: u32, out: &mut [u8]);
    /// Stores `data` at `addr`.
    fn write_bytes(&mut self, addr: u32, data: &[u8]);
    /// Where `FFX.exe` is loaded.
    fn image_base(&self) -> u32;

    /// A value at `addr`.
    fn read<T: Value>(&self, addr: u32) -> T {
        let mut b = [0u8; 64];
        if T::SIZE <= b.len() {
            self.read_bytes(addr, &mut b[..T::SIZE]);
            T::from_bytes(&b[..T::SIZE])
        } else {
            let mut v = vec![0u8; T::SIZE];
            self.read_bytes(addr, &mut v);
            T::from_bytes(&v)
        }
    }

    /// Stores a value at `addr`.
    fn write<T: Value>(&mut self, addr: u32, value: T) {
        let mut b = [0u8; 64];
        if T::SIZE <= b.len() {
            value.to_bytes(&mut b[..T::SIZE]);
            self.write_bytes(addr, &b[..T::SIZE]);
        } else {
            let mut v = vec![0u8; T::SIZE];
            value.to_bytes(&mut v);
            self.write_bytes(addr, &v);
        }
    }

    /// An address from `symbols/ffx.sym` where the image really is.
    fn rebase(&self, sym: u32) -> u32 {
        sym.wrapping_sub(IMAGE_BASE).wrapping_add(self.image_base())
    }
}

/// A typed 32-bit pointer into game memory.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ptr<T> {
    /// The address.
    pub addr: u32,
    ty: PhantomData<fn() -> T>,
}

impl<T> Clone for Ptr<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for Ptr<T> {}

impl<T> Ptr<T> {
    /// A pointer to `addr`.
    #[must_use]
    pub const fn new(addr: u32) -> Self {
        Self {
            addr,
            ty: PhantomData,
        }
    }

    /// The null pointer.
    #[must_use]
    pub const fn null() -> Self {
        Self::new(0)
    }

    /// Whether it is null.
    #[must_use]
    pub const fn is_null(self) -> bool {
        self.addr == 0
    }

    /// `offset` bytes further, as another type (a struct field).
    #[must_use]
    pub const fn field<U>(self, offset: u32) -> Ptr<U> {
        Ptr::new(self.addr.wrapping_add(offset))
    }

    /// The pointer cast to another type.
    #[must_use]
    pub const fn cast<U>(self) -> Ptr<U> {
        Ptr::new(self.addr)
    }
}

impl<T: Value> Ptr<T> {
    /// The pointed-to value.
    pub fn get(self, mem: &impl Mem) -> T {
        mem.read(self.addr)
    }

    /// Stores the pointed-to value.
    pub fn set(self, mem: &mut impl Mem, value: T) {
        mem.write(self.addr, value);
    }

    /// The `index`-th element of an array starting here.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)] // values are at most 64 bytes
    pub const fn at(self, index: u32) -> Self {
        Self::new(self.addr.wrapping_add(index.wrapping_mul(T::SIZE as u32)))
    }
}

impl<T> Ptr<Ptr<T>> {
    /// Follows a pointer stored here.
    pub fn load(self, mem: &impl Mem) -> Ptr<T> {
        Ptr::new(mem.read::<u32>(self.addr))
    }

    /// Stores a pointer here.
    pub fn store(self, mem: &mut impl Mem, ptr: Ptr<T>) {
        mem.write(self.addr, ptr.addr);
    }
}

/// A global of `FFX.exe`, at its `symbols/ffx.sym` address.
#[derive(Debug)]
pub struct Global<T> {
    /// Address at the preferred image base.
    pub sym: u32,
    ty: PhantomData<fn() -> T>,
}

impl<T> Clone for Global<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for Global<T> {}

impl<T> Global<T> {
    /// The global at `sym` (an `ffx.sym` address).
    #[must_use]
    pub const fn new(sym: u32) -> Self {
        Self {
            sym,
            ty: PhantomData,
        }
    }

    /// Where it is in this process.
    pub fn ptr(self, mem: &impl Mem) -> Ptr<T> {
        Ptr::new(mem.rebase(self.sym))
    }
}

impl<T: Value> Global<T> {
    /// Its value.
    pub fn get(self, mem: &impl Mem) -> T {
        self.ptr(mem).get(mem)
    }

    /// Stores its value.
    pub fn set(self, mem: &mut impl Mem, value: T) {
        let p = self.ptr(mem);
        p.set(mem, value);
    }
}

/// Plain host memory, for tests: a flat buffer standing for addresses
/// `base..base + len`.
#[derive(Debug, Clone)]
pub struct VecMem {
    /// Lowest address.
    pub base: u32,
    /// Contents.
    pub bytes: Vec<u8>,
    /// Image base to report.
    pub image: u32,
}

impl VecMem {
    /// `len` zero bytes from `base`; the image at its preferred base.
    #[must_use]
    pub fn new(base: u32, len: usize) -> Self {
        Self {
            base,
            bytes: vec![0; len],
            image: IMAGE_BASE,
        }
    }

    fn range(&self, addr: u32, len: usize) -> std::ops::Range<usize> {
        let start = addr
            .checked_sub(self.base)
            .expect("address below the buffer") as usize;
        start..start + len
    }
}

impl Mem for VecMem {
    fn read_bytes(&self, addr: u32, out: &mut [u8]) {
        out.copy_from_slice(&self.bytes[self.range(addr, out.len())]);
    }
    fn write_bytes(&mut self, addr: u32, data: &[u8]) {
        let r = self.range(addr, data.len());
        self.bytes[r].copy_from_slice(data);
    }
    fn image_base(&self) -> u32 {
        self.image
    }
}

/// The game's own memory, from inside the game process (32-bit).
#[cfg(target_pointer_width = "32")]
#[derive(Debug, Clone, Copy)]
pub struct RawMem {
    /// Where `FFX.exe` was loaded.
    pub image: u32,
}

#[cfg(target_pointer_width = "32")]
impl Mem for RawMem {
    fn read_bytes(&self, addr: u32, out: &mut [u8]) {
        // SAFETY: replacements only touch what the original would; the
        // address is the game's own, valid in this process.
        unsafe {
            std::ptr::copy_nonoverlapping(addr as usize as *const u8, out.as_mut_ptr(), out.len())
        }
    }
    fn write_bytes(&mut self, addr: u32, data: &[u8]) {
        // SAFETY: as above.
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), addr as usize as *mut u8, data.len())
        }
    }
    fn image_base(&self) -> u32 {
        self.image
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_round_trip() {
        let mut m = VecMem::new(0x1000, 0x100);
        m.write(0x1004, 0x1234_5678u32);
        assert_eq!(m.read::<u8>(0x1004), 0x78);
        assert_eq!(m.read::<u16>(0x1006), 0x1234);
        m.write(0x1010, [1.5f32, -2.0, 0.25, 4.0]);
        assert_eq!(m.read::<[f32; 4]>(0x1010), [1.5, -2.0, 0.25, 4.0]);
        let p: Ptr<[f32; 4]> = Ptr::new(0x1010);
        assert_eq!(p.field::<f32>(4).get(&m), -2.0);
        assert_eq!(Ptr::<u32>::new(0x1000).at(1).addr, 0x1004);
    }

    #[test]
    fn globals_follow_the_image() {
        let mut m = VecMem::new(0x0100_0000, 0x100);
        m.image = 0x00c0_0000; // loaded 0x800000 higher than preferred
        let g: Global<u32> = Global::new(0x0080_0010);
        g.set(&mut m, 7);
        assert_eq!(m.read::<u32>(0x0100_0010), 7);
    }
}
