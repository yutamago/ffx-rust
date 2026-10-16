//! What the battle oracle tests share: the two oracles started alike, a
//! fixed-seed random source, and the compare helpers.

#![allow(dead_code)] // each test file uses its own share

pub mod crt;
pub mod world;

use std::path::PathBuf;

use ffx_bridge::Global;
use ffx_oracle::{Conv, Oracle, Pages};
pub fn exe() -> Option<PathBuf> {
    let dir = std::env::var_os("FFX_GAME_DIR").map(PathBuf::from);
    if dir.is_none() {
        eprintln!("FFX_GAME_DIR is not set; skipping");
    }
    Some(dir?.join("FFX.exe"))
}

/// The two oracles on the image alone, the x87 as on the game thread
/// (`0x027f`: 53 bits, exceptions masked).
pub fn synthetic() -> Option<(Oracle, Oracle)> {
    let exe = exe()?;
    let (mut a, mut b) = (Oracle::load(&exe).unwrap(), Oracle::load(&exe).unwrap());
    a.set_fpu_control(0x027f).unwrap();
    b.set_fpu_control(0x027f).unwrap();
    Some((a, b))
}

/// The two oracles on `FFX_SNAPSHOT`.
pub fn snapshot() -> Option<(Oracle, Oracle)> {
    let exe = exe()?;
    let Some(snap) = std::env::var_os("FFX_SNAPSHOT") else {
        eprintln!("FFX_SNAPSHOT is not set; skipping");
        return None;
    };
    let pages = Pages::read(PathBuf::from(snap)).unwrap();
    Some((
        Oracle::from_snapshot(&exe, &pages).unwrap(),
        Oracle::from_snapshot(&exe, &pages).unwrap(),
    ))
}

pub struct Xorshift(pub u32);

impl Xorshift {
    pub fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }

    /// Below `n`.
    pub fn below(&mut self, n: u32) -> u32 {
        self.next() % n
    }

    /// In `lo..hi`.
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        lo + self.below((hi - lo).cast_unsigned()).cast_signed()
    }

    /// A byte, often 0 or 0xff.
    pub fn byte(&mut self) -> u8 {
        match self.below(4) {
            0 => 0,
            1 => 0xff,
            _ => self.next().to_le_bytes()[0],
        }
    }

    /// A word, often small or at a limit.
    pub fn word(&mut self) -> u32 {
        match self.below(6) {
            0 => 0,
            1 => u32::MAX,
            2 => self.below(16),
            3 => 0x8000_0000,
            _ => self.next(),
        }
    }
}

/// Writes `bytes` at `addr` in both.
pub fn poke(a: &mut Oracle, b: &mut Oracle, addr: u32, bytes: &[u8]) {
    a.write(addr, bytes).unwrap();
    b.write(addr, bytes).unwrap();
}

/// Random bytes (see [`Xorshift::byte`]) at `addr` in both.
pub fn fill(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, addr: u32, len: u32) {
    let bytes: Vec<u8> = (0..len).map(|_| rng.byte()).collect();
    poke(a, b, addr, &bytes);
}

/// Sets a global in both.
pub fn set<T: ffx_bridge::Value>(a: &mut Oracle, b: &mut Oracle, g: Global<T>, v: T) {
    g.set(a, v);
    g.set(b, v);
}

/// Zeroed scratch memory both oracles allocate alike.
pub fn scratch(a: &mut Oracle, b: &mut Oracle, len: usize) -> u32 {
    let x = a.alloc(len).unwrap();
    assert_eq!(b.alloc(len).unwrap(), x);
    x
}

/// Calls the original at its `ffx.sym` address in `a`, the Rust in `b`;
/// compares the results under `mask`.
pub fn twin(
    what: &str,
    a: &mut Oracle,
    b: &mut Oracle,
    sym: u32,
    args: &[u32],
    mask: u32,
    rust: impl FnOnce(&mut Oracle) -> u32,
) {
    let ra = a
        .call(a.rebase(sym), Conv::Cdecl, args)
        .unwrap_or_else(|e| panic!("{what} {args:x?}: {e}"))
        .eax
        & mask;
    let rb = rust(b) & mask;
    assert_eq!(ra, rb, "{what} {args:x?}: result");
}

/// Both wrote the same.
pub fn same(what: &str, a: &mut Oracle, b: &mut Oracle) {
    assert_eq!(a.diff(b).unwrap(), [], "{what}: memory");
}

/// Whether `len` bytes at `addr` can be read (a pointer in a snapshot).
pub fn readable(o: &mut Oracle, addr: u32, len: usize) -> bool {
    addr != 0 && o.read(addr, len).is_ok()
}

pub const ALL: u32 = u32::MAX;
