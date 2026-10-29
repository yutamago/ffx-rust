//! The spell and summon overlays (`magicFiles/FFX/magic_*.dll`) in Rust
//! (docs/formats/magic.md, "Porting the overlays").
//!
//! A Rust overlay stands behind the DLL's own interface: the game still
//! "loads" the player's `magic_NNNN.dll`, its headers and data sections
//! are laid out at the image base the original would get (so every
//! pointer the game stores into overlay data stays the same), and every
//! address the game calls into the overlay's code (`DllMain`,
//! `InitMagicPRX`, `GetEffectOverlayTable`, the 7 table entries,
//! callbacks the overlay stored) is answered by a port bound to that
//! address. The ports reach the game only through `g_MagicImports`, the
//! table `InitMagicPRX` received, exactly as the DLL does.
//!
//! - **Binding by code identity.** A port declares the function it
//!   replaces as a [`Sig`]: its length and a hash of its bytes with every
//!   relocated word and every `rel32` call target zeroed (the words a
//!   link moves). [`bind`] finds the port whose signature matches the
//!   code at an address of a loaded image, so a function shared by many
//!   DLLs (the CRT, the slot thunks, `InitMagicPRX`, the OPU host common
//!   to 130 overlays) is ported once and binds in every one of them.
//!   The addresses that differ between DLLs (their globals, their tables)
//!   are read from the DLL's own code words ([`Image::word`]), the same
//!   operands the original instructions carry.
//! - **No code runs.** The player's file is read as data: the data
//!   sections into game memory, the code section into the [`Image`] for
//!   its operands and the binding.
//! - **One interface for every host.** Ports are generic over [`Game`]
//!   (`ffx_bridge::Mem` plus calls into the game and the few runtime
//!   imports whose results must be the game's own); the 32-bit hook, the
//!   parity tests and `ffx-host` each provide one.

#![allow(clippy::cast_possible_truncation)] // guest addresses are u32 by design

pub mod opu;
pub mod pe;
pub mod runtime;

pub mod m0604;

pub use ffx_bridge::Mem;
// the mutation schemata (tools/mutants.py) call `ffx_core::mutant`
use ffx_core as _;

/// A malformed DLL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeError {
    BadImage(String),
}

impl std::fmt::Display for PeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PeError::BadImage(s) => write!(f, "not a usable overlay image: {s}"),
        }
    }
}

impl std::error::Error for PeError {}

/// What a call into the game returned: `eax`, `edx` and a float left in
/// ST0. The game runs x87 at 53-bit precision (control word 0x027f), so
/// an ST0 result is exact as an `f64`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Out {
    pub eax: u32,
    pub edx: u32,
    pub st0: Option<f64>,
}

/// The `_libm_sse2_*_precise` imports (`xmm0` in, `xmm0` out).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Libm {
    Sin,
    Cos,
    Atan,
    Acos,
}

/// The game as an overlay sees it: memory, calls through
/// `g_MagicImports` (or any other game address), and the `MSVCR110`
/// imports whose results must be the game's own (`rand` shares its
/// sequence with `FFX.exe`'s calls; the libm functions are MSVCR110's
/// results, not Rust's).
pub trait Game: Mem {
    /// Calls the cdecl function at `target` with `args` (32-bit words;
    /// an `f32` argument as its bits), `ecx` set for a thiscall.
    fn call(&mut self, target: u32, ecx: u32, args: &[u32]) -> Out;
    /// `MSVCR110!rand`.
    fn rand(&mut self) -> u32;
    /// `MSVCR110!_libm_sse2_*_precise`.
    fn libm(&mut self, f: Libm, x: f64) -> f64;
    /// `MSVCR110!_CIatan2(y, x)` (x87: ST1 = y, ST0 = x).
    fn atan2(&mut self, y: f64, x: f64) -> f64;
}

/// A call into overlay code: the address called, `esp` at the return
/// address (the stack arguments above it) and the registers a port may
/// read (register arguments; `eax` for code that returns it untouched).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Frame {
    pub at: u32,
    pub esp: u32,
    pub eax: u32,
    pub ecx: u32,
    pub edx: u32,
}

impl Frame {
    /// Stack argument `i` (0 = the word above the return address).
    pub fn arg(&self, m: &impl Mem, i: u32) -> u32 {
        m.read::<u32>(self.esp + 4 + 4 * i)
    }
}

/// What a port leaves for its caller: `eax`, `edx`, a float in ST0 and
/// the argument bytes it popped (`ret n`; 0 for cdecl). Or a tail jump:
/// `jump` is where the original jumps with the caller's stack as it was
/// at the entry (`jmp eax` with eax = the target); the host continues
/// there and the target returns to the caller.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Ret {
    pub eax: u32,
    pub edx: u32,
    pub st0: Option<f64>,
    pub pops: u32,
    pub jump: Option<u32>,
}

impl Ret {
    /// A cdecl return of `eax`.
    #[must_use]
    pub fn eax(eax: u32) -> Ret {
        Ret {
            eax,
            ..Ret::default()
        }
    }

    /// A tail jump to `target` (`jmp eax`).
    #[must_use]
    pub fn jump(target: u32) -> Ret {
        Ret {
            eax: target,
            jump: Some(target),
            ..Ret::default()
        }
    }
}

/// A port of one overlay function, called with the overlay it runs for.
pub type Entry<G> = fn(&mut G, &Image, &Frame) -> Ret;

/// What a `rel32` call or jump of a signature must reach.
#[derive(Debug, Clone, Copy)]
pub enum Callee {
    /// A function with this signature.
    Sig(&'static Sig),
    /// `jmp [IAT]` to this import (the 6-byte import thunks).
    Import(&'static str),
    /// Not checked: the CRT's internal calls (`DllMain`'s
    /// `__security_init_cookie` and `__DllMainCRTStartup`), whose code
    /// differs between DLLs only by the link (their callees' places).
    Any,
}

/// A function's identity in an overlay (see the crate docs).
#[derive(Debug)]
pub struct Sig {
    /// Bytes, trailing `int3` padding left out.
    pub len: u32,
    /// [`Image::masked_hash`] of those bytes.
    pub hash: u64,
    /// Offsets of `rel32` fields (from the function's start) and what
    /// they must reach; zeroed for the hash, checked when binding.
    pub calls: &'static [(u32, Callee)],
    /// Offsets of relocated words that must hold an import's IAT slot
    /// (`call [IAT]`), checked when binding.
    pub imports: &'static [(u32, &'static str)],
}

/// One loaded overlay: its number, where it lives, and the relocated
/// image (code included, as data) for operands and binding.
#[derive(Debug, Clone)]
pub struct Image {
    /// `NNNN` of `magic_NNNN.dll`.
    pub number: u16,
    pub base: u32,
    pub preferred_base: u32,
    /// The image laid out at `base`, relocated; imports as in the file.
    bytes: Vec<u8>,
    /// RVAs of the relocated words, sorted.
    relocs: Vec<u32>,
    /// The executable range (RVAs).
    pub text: (u32, u32),
    /// IAT slot (RVA) -> import name.
    imports: Vec<(u32, String)>,
    pub entry: u32,
    pub exports: Vec<(String, u32)>,
}

/// `magic_NNNN.dll` -> `NNNN`.
#[must_use]
pub fn dll_number(name: &str) -> Option<u16> {
    let file = name.rsplit(['\\', '/']).next()?.to_ascii_lowercase();
    file.strip_prefix("magic_")?
        .strip_suffix(".dll")?
        .parse()
        .ok()
}

impl Image {
    /// Lays out `file` (`magic_NNNN.dll`, `name`) at `base`.
    ///
    /// # Errors
    /// Not an overlay image.
    pub fn new(name: &str, file: &[u8], base: u32) -> Result<Image, PeError> {
        let pe = pe::Pe::parse(file)?;
        let bytes = pe.layout(file, base)?;
        let mut relocs = pe.relocs.clone();
        relocs.sort_unstable();
        let text = pe
            .sections
            .iter()
            .find(|s| s.exec)
            .map(|s| (s.rva, s.rva + s.size))
            .ok_or_else(|| PeError::BadImage("no code".into()))?;
        Ok(Image {
            number: dll_number(name).unwrap_or(0),
            base,
            preferred_base: pe.preferred_base,
            bytes,
            relocs,
            text,
            imports: pe.imports.iter().map(|i| (i.iat, i.name.clone())).collect(),
            entry: pe.entry,
            exports: pe.exports.clone(),
        })
    }

    /// Writes the headers and every section outside the code range into
    /// game memory at `base` (what the original DLL's data looks like
    /// after the loader). The import address table keeps the file's
    /// values: no port reads it.
    pub fn write_data(&self, m: &mut impl Mem) {
        let t0 = self.text.0 & !0xfff;
        let t1 = (self.text.1 + 0xfff) & !0xfff;
        m.write_bytes(self.base, &self.bytes[..t0 as usize]);
        if (t1 as usize) < self.bytes.len() {
            m.write_bytes(self.base + t1, &self.bytes[t1 as usize..]);
        }
    }

    /// The relocated word at absolute address `addr` of the image (code
    /// included): an instruction's operand, a table's entry.
    #[must_use]
    pub fn word(&self, addr: u32) -> u32 {
        let at = addr.wrapping_sub(self.base) as usize;
        self.bytes
            .get(at..at + 4)
            .map_or(0, |b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    /// `base + rva`.
    #[must_use]
    pub fn at(&self, rva: u32) -> u32 {
        self.base + rva
    }

    /// Whether `addr` is in the code section.
    #[must_use]
    pub fn in_text(&self, addr: u32) -> bool {
        let r = addr.wrapping_sub(self.base);
        r >= self.text.0 && r < self.text.1
    }

    /// The export's address.
    #[must_use]
    pub fn export(&self, name: &str) -> Option<u32> {
        self.exports
            .iter()
            .find(|(n, _)| n == name)
            .map(|e| self.base + e.1)
    }

    /// FNV-1a (64) of `len` bytes at `addr` with every relocated word
    /// inside them and each `rel32` field at `zero` (offsets) zeroed.
    #[must_use]
    pub fn masked_hash(&self, addr: u32, len: u32, zero: &[u32]) -> Option<u64> {
        let r0 = addr.checked_sub(self.base)?;
        let mut b = self.bytes.get(r0 as usize..(r0 + len) as usize)?.to_vec();
        let i = self.relocs.partition_point(|&r| r < r0);
        for &r in &self.relocs[i..] {
            if r + 4 > r0 + len {
                break;
            }
            let o = (r - r0) as usize;
            b[o..o + 4].fill(0);
        }
        for &o in zero {
            if let Some(w) = b.get_mut(o as usize..o as usize + 4) {
                w.fill(0);
            }
        }
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for x in b {
            h ^= u64::from(x);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
        Some(h)
    }

    /// The relocated words of `len` bytes at `addr` (the globals and
    /// tables the code there refers to).
    #[must_use]
    pub fn operands(&self, addr: u32, len: u32) -> Vec<u32> {
        let r0 = addr.wrapping_sub(self.base);
        let i = self.relocs.partition_point(|&r| r < r0);
        self.relocs[i..]
            .iter()
            .take_while(|&&r| r + 4 <= r0 + len)
            .map(|&r| self.word(self.base + r))
            .collect()
    }

    /// The target of the `rel32` field at `field`.
    #[must_use]
    pub fn rel32_target(&self, field: u32) -> u32 {
        field.wrapping_add(4).wrapping_add(self.word(field))
    }

    /// The import whose IAT slot is at absolute `addr`.
    #[must_use]
    pub fn import_at(&self, addr: u32) -> Option<&str> {
        let rva = addr.wrapping_sub(self.base);
        self.imports
            .iter()
            .find(|(a, _)| *a == rva)
            .map(|(_, n)| n.as_str())
    }

    /// Whether the code at `addr` is the function `sig` describes,
    /// callees and imports included.
    #[must_use]
    pub fn matches(&self, addr: u32, sig: &Sig) -> bool {
        self.matches_depth(addr, sig, 4)
    }

    fn matches_depth(&self, addr: u32, sig: &Sig, depth: u32) -> bool {
        let zero: Vec<u32> = sig.calls.iter().map(|c| c.0).collect();
        if self.masked_hash(addr, sig.len, &zero) != Some(sig.hash) {
            return false;
        }
        let imports_ok = sig
            .imports
            .iter()
            .all(|&(off, name)| self.import_at(self.word(addr + off)) == Some(name));
        imports_ok
            && sig.calls.iter().all(|&(off, callee)| {
                let t = self.rel32_target(addr + off);
                match callee {
                    Callee::Sig(s) => depth > 0 && self.matches_depth(t, s, depth - 1),
                    Callee::Any => true,
                    // ff 25 <IAT>
                    Callee::Import(name) => {
                        self.word(t).to_le_bytes()[..2] == [0xff, 0x25]
                            && self.import_at(self.word(t + 2)) == Some(name)
                    }
                }
            })
    }
}

/// Every port: its name, signature and a dispatcher index.
macro_rules! ports {
    ($($name:literal => $sig:path, $m:ident :: $f:ident;)*) => {
        /// The ports, in [`bind`]'s order: `(name, signature)`.
        pub const PORTS: &[(&str, &Sig)] = &[$(($name, &$sig)),*];

        /// The port at `PORTS[i]` for a game type.
        #[must_use]
        pub fn entry<G: Game>(i: usize) -> Option<Entry<G>> {
            let table: &[Entry<G>] = &[$($m::$f::<G>),*];
            table.get(i).copied()
        }
    };
}

ports! {
    "slot thunk" => runtime::SLOT_THUNK, runtime::slot_thunk;
    "DllMain (CRT start-up)" => runtime::DLL_MAIN, runtime::dll_main;
    "GetEffectOverlayTable" => runtime::GET_EFFECT_OVERLAY_TABLE, runtime::get_effect_overlay_table;
    "InitMagicPRX" => runtime::INIT_MAGIC_PRX, runtime::init_magic_prx;
    "ret" => opu::NOP, opu::nop;
    "opu start" => opu::START, opu::start;
    "opu run" => opu::RUN, opu::run;
    "opu stop" => opu::STOP, opu::stop;
    "m0604 start-up" => m0604::E1660, m0604::e1660;
    "m0604 shutdown" => m0604::E1820, m0604::e1820;
}

/// The port of the function at `addr` of `img`: its index in [`PORTS`].
/// `None` when no port matches the code there. Slot thunks differ only
/// in the slot, so they bind by shape ([`runtime::slot_of_thunk`]).
#[must_use]
pub fn bind(img: &Image, addr: u32) -> Option<usize> {
    if !img.in_text(addr) {
        return None;
    }
    if runtime::table_global(img)
        .and_then(|t| runtime::slot_of_thunk(img, addr, t))
        .is_some()
    {
        return Some(0);
    }
    PORTS
        .iter()
        .skip(1)
        .position(|(_, sig)| img.matches(addr, sig))
        .map(|i| i + 1)
}
