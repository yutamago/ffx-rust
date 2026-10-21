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
    let r = a
        .call(a.rebase(sym), Conv::Cdecl, args)
        .unwrap_or_else(|e| panic!("{what} {args:x?}: {e}"));
    // A float the original leaves in ST0 is a result its callers use: a
    // port without one (void or an integer) drops it (use [`ftwin`]).
    assert!(
        r.st0.is_none(),
        "{what} {args:x?}: the original returns {:?} in ST0",
        r.st0
    );
    let ra = r.eax & mask;
    let rb = rust(b) & mask;
    assert_eq!(ra, rb, "{what} {args:x?}: result");
}

/// [`twin`] for a function returning an `f32` in ST0: the port's result
/// (bits) against ST0 rounded to `f32` as a caller's `fstp dword` would.
pub fn ftwin(
    what: &str,
    a: &mut Oracle,
    b: &mut Oracle,
    sym: u32,
    args: &[u32],
    rust: impl FnOnce(&mut Oracle) -> u32,
) {
    let r = a
        .call(a.rebase(sym), Conv::Cdecl, args)
        .unwrap_or_else(|e| panic!("{what} {args:x?}: {e}"));
    let st0 = r
        .st0_raw
        .unwrap_or_else(|| panic!("{what} {args:x?}: no float in ST0"));
    let ra = crt::f80_to_f32(st0);
    let rb = rust(b);
    assert_eq!(ra, rb, "{what} {args:x?}: float result");
}

/// Both wrote the same.
pub fn same(what: &str, a: &mut Oracle, b: &mut Oracle) {
    assert_eq!(a.diff(b).unwrap(), [], "{what}: memory");
    // Every page either side wrote is equal now and stays so until a side
    // writes it again, which marks it once more: later diffs need only
    // those. Without this each diff re-read every page any earlier case
    // wrote (battle_target's whole-file tests spent most of their time
    // there).
    a.clear_dirty();
    b.clear_dirty();
}

/// Whether `len` bytes at `addr` can be read (a pointer in a snapshot).
pub fn readable(o: &mut Oracle, addr: u32, len: usize) -> bool {
    addr != 0 && o.read(addr, len).is_ok()
}

pub const ALL: u32 = u32::MAX;

/// `0x007ceb30 + 0x340` (the tail of `MsCheckMemory` and the debug
/// menu's `0x007ceeb0`, `MsExecuteCommand`, `0x007cef80`: unreachable;
/// `0x007ceb30 + 0..0x340` holds `battle_target`'s `START_STUBS2`): where
/// the recording stubs go in both oracles (the heap is not executable).
pub const STUBS: u32 = 0x007c_ee70;

/// `AtelDecodeSignal` and `AtelDecodeCommand` as stubs in one oracle,
/// each counting its calls and recording its two arguments at `rec`
/// (+0, +0x10): running the scripts' code would reach the OS's waits.
pub fn script_stubs(o: &mut Oracle, rec: u32) {
    let base = o.rebase(STUBS);
    for (k, f) in [
        ffx_battle::atel::bridge::ATEL_DECODE_SIGNAL,
        ffx_battle::atel::bridge::ATEL_DECODE_COMMAND,
    ]
    .into_iter()
    .enumerate()
    {
        let r = rec + 0x10 * u32::try_from(k).unwrap();
        let mut code = vec![0xff, 0x05];
        code.extend(r.to_le_bytes());
        for (arg, to) in [(4u8, r + 4), (8, r + 8)] {
            code.extend([0x8b, 0x44, 0x24, arg, 0xa3]);
            code.extend(to.to_le_bytes());
        }
        code.extend([0x31, 0xc0, 0xc3]);
        let at = base + 0x20 * u32::try_from(k).unwrap();
        o.write(at, &code).unwrap();
        // the function jumps to its stub
        let entry = o.rebase(f);
        let rel = at.wrapping_sub(entry + 5);
        o.write(entry, &[[0xe9].as_slice(), &rel.to_le_bytes()].concat())
            .unwrap();
    }
}

/// Replaces the function at `f` (ffx.sym address) in one oracle by a stub
/// at `at` (ffx.sym address, unused code) that counts its calls at `rec`,
/// keeps its `n` arguments at `rec + 4` on and returns the word after them
/// (`ret` at first; write it to change the result): callees of other
/// modules whose effects the test checks through what they got.
pub fn recording_stub(o: &mut Oracle, f: u32, at: u32, rec: u32, n: u8, ret: u32) {
    let at = o.rebase(at);
    let mut code = vec![0xff, 0x05];
    code.extend(rec.to_le_bytes());
    for k in 0..n {
        code.extend([0x8b, 0x44, 0x24, 4 + 4 * k, 0xa3]);
        code.extend((rec + 4 + 4 * u32::from(k)).to_le_bytes());
    }
    let r = rec + 4 + 4 * u32::from(n);
    o.write(r, &ret.to_le_bytes()).unwrap();
    code.push(0xa1);
    code.extend(r.to_le_bytes());
    code.push(0xc3);
    o.write(at, &code).unwrap();
    let entry = o.rebase(f);
    let rel = at.wrapping_sub(entry + 5);
    o.write(entry, &[[0xe9].as_slice(), &rel.to_le_bytes()].concat())
        .unwrap();
}

/// [`recording_stub`] keeping every call: the count at `rec`, the result
/// at `rec + 4` (write it to change it), the `n` arguments of the calls in
/// a ring of 16 from `rec + 8` (`8 + 0x40 n` bytes in all). For callees
/// called many times in one call (the read queue's loads). With `f` 0
/// only the code is written (for a function pointer to point at).
pub fn logging_stub(o: &mut Oracle, f: u32, at: u32, rec: u32, n: u8, ret: u32) {
    let at = o.rebase(at);
    let mut code = vec![0x8b, 0x0d];
    code.extend(rec.to_le_bytes());
    code.extend([0xff, 0x05]);
    code.extend(rec.to_le_bytes());
    code.extend([0x83, 0xe1, 0x0f, 0x6b, 0xc9, 4 * n, 0x81, 0xc1]);
    code.extend((rec + 8).to_le_bytes());
    for k in 0..n {
        code.extend([0x8b, 0x44, 0x24, 4 + 4 * k, 0x89, 0x41, 4 * k]);
    }
    code.push(0xa1);
    code.extend((rec + 4).to_le_bytes());
    code.push(0xc3);
    // 0x40 bytes of code for up to 4 arguments, 0x80 for 5 (place those
    // 0x80 apart)
    assert!(code.len() <= if n > 4 { 0x80 } else { 0x40 });
    o.write(rec, &[0; 4]).unwrap();
    o.write(rec + 4, &ret.to_le_bytes()).unwrap();
    o.write(at, &code).unwrap();
    // f 0: the code alone (a function pointer's target)
    if f != 0 {
        let entry = o.rebase(f);
        let rel = at.wrapping_sub(entry + 5);
        o.write(entry, &[[0xe9].as_slice(), &rel.to_le_bytes()].concat())
            .unwrap();
    }
}
