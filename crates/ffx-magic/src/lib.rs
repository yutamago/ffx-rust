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

#![allow(clippy::cast_possible_truncation)]
// guest addresses are u32 by design
// every port names the game `g`, its address `f` and its operand reader `w`
#![allow(clippy::many_single_char_names)]

// the runtime and code shared by many overlays
pub mod l1_opu;
pub mod l3_common;
pub mod l3_opu;
pub mod l5_objhost;
pub mod l5_opu_boot;
pub mod nodes;
pub mod ob_model;
pub mod ob_summon;
pub mod objcb;
pub mod opu;
pub mod opu_parts;
pub mod pe;
pub mod pushes;
pub mod rcef;
pub mod runtime;
// the shared work list's chunk A (functions used by 2 or more overlays):
// the node pool, the summon family's handlers, the magic_0162 family's
// script commands and helpers without a family yet
pub mod sa_misc;
pub mod sa_pool;
pub mod sa_script;
pub mod sa_summon;
pub mod sb_common;
pub mod sb_matrix;
pub mod sb_misc;
pub mod sb_nodes;
pub mod sb_ppp;
pub mod sb_sse;
pub mod sb_vm;
// the shared work list's third round (shared-3)
pub mod s3_matrix;
pub mod s3_misc;
pub mod s3_motion;
pub mod s3_nodes;
pub mod s3_summon;
pub mod s3_vm;
pub mod s3_wrap;

// round 4, the own work list's families (o4_*)
pub mod o4_effect;
pub mod o4_fam2;
pub mod o4_fam3;
pub mod o4_fam4;
pub mod o4_parts;
pub mod o4_rcef;
pub mod o4_small;
pub mod o4_state;
pub mod o4_steps;
pub mod o4_tiny;
pub mod o4_tiny2;
pub mod o4_tiny3;
pub mod o4_tiny4;
pub mod o4_tiny5;

// named after the DLL they were ported from (some bind in many)
pub mod m0003;
pub mod m0009;
pub mod m0018;
pub mod m0019;
pub mod m0020;
pub mod m0032;
pub mod m0042;
pub mod m0045;
pub mod m0046;
pub mod m0052;
pub mod m0054;
pub mod m0070;
pub mod m0071;
pub mod m0072;
pub mod m0074;
pub mod m0078;
pub mod m0084;
pub mod m0090;
pub mod m0093;
pub mod m0098;
pub mod m0153;
pub mod m0154;
pub mod m0159;
pub mod m0163;
pub mod m0166;
pub mod m0170;
pub mod m0187;
pub mod m0194;
pub mod m0196;
pub mod m0219;
pub mod m0390;
pub mod m0393;
pub mod m0394;
pub mod m0395;
pub mod m0396;
pub mod m0481;
pub mod m0596;
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

/// What a call into the game returned: `eax`, `edx`, a float left in
/// ST0 and `xmm0` as the callee left it (a float returned there, by the
/// few functions that use SSE results). The game runs x87 at 53-bit
/// precision (control word 0x027f), so an ST0 result is exact as an
/// `f64`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Out {
    pub eax: u32,
    pub edx: u32,
    pub st0: Option<f64>,
    pub xmm0: u128,
}

/// A float in the low lane of an SSE register (what a caller reads of a
/// `movss`/`movsd` result), as bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Xmm {
    F32(u32),
    F64(u64),
}

impl Xmm {
    #[must_use]
    pub fn f32(x: f32) -> Xmm {
        Xmm::F32(x.to_bits())
    }

    #[must_use]
    pub fn f64(x: f64) -> Xmm {
        Xmm::F64(x.to_bits())
    }

    /// A register holding this value in its low lane, the rest zero.
    #[must_use]
    pub fn reg(self) -> u128 {
        match self {
            Xmm::F32(b) => u128::from(b),
            Xmm::F64(b) => u128::from(b),
        }
    }

    /// Whether the low lane of `reg` holds this value, bit for bit (the
    /// lanes above it are not part of the result).
    #[must_use]
    pub fn is_in(self, reg: u128) -> bool {
        match self {
            Xmm::F32(b) => reg as u32 == b,
            Xmm::F64(b) => reg as u64 == b,
        }
    }
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
/// read (register arguments; `eax` for code that returns it untouched;
/// `xmm0`..`xmm3` for the float arguments LTCG passes the overlays'
/// internal functions there).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Frame {
    pub at: u32,
    pub esp: u32,
    pub eax: u32,
    pub ecx: u32,
    pub edx: u32,
    pub xmm: [u128; 4],
}

impl Frame {
    /// Stack argument `i` (0 = the word above the return address).
    pub fn arg(&self, m: &impl Mem, i: u32) -> u32 {
        m.read::<u32>(self.esp + 4 + 4 * i)
    }

    /// The `f32` in the low lane of `xmm<i>`.
    #[must_use]
    pub fn xmm_f32(&self, i: usize) -> f32 {
        f32::from_bits(self.xmm[i] as u32)
    }

    /// The `f64` in the low lane of `xmm<i>`.
    #[must_use]
    pub fn xmm_f64(&self, i: usize) -> f64 {
        f64::from_bits(self.xmm[i] as u64)
    }
}

/// What a port leaves for its caller: `eax`, `edx`, a float in ST0 or in
/// `xmm0` (an internal function's SSE result) and the argument bytes it
/// popped (`ret n`; 0 for cdecl). Or a tail jump:
/// `jump` is where the original jumps with the caller's stack as it was
/// at the entry (`jmp eax` with eax = the target); the host continues
/// there and the target returns to the caller.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Ret {
    pub eax: u32,
    pub edx: u32,
    pub st0: Option<f64>,
    pub xmm0: Option<Xmm>,
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

    /// A float returned in `xmm0`, `eax` as the original leaves it.
    #[must_use]
    pub fn xmm0(x: Xmm, eax: u32) -> Ret {
        Ret {
            eax,
            xmm0: Some(x),
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

const fn same_str(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// Whether one callee check can accept what the other does (`Any`
/// accepts everything).
const fn callees_overlap(a: &Callee, b: &Callee) -> bool {
    match (a, b) {
        (Callee::Any, _) | (_, Callee::Any) => true,
        (Callee::Sig(x), Callee::Sig(y)) => sigs_overlap(x, y),
        (Callee::Import(x), Callee::Import(y)) => same_str(x, y),
        _ => false,
    }
}

/// Whether some code could match both signatures: the same length and
/// masked hash (the `magic-sig` code identity), the same `rel32` fields
/// with overlapping callees, and the same imports. Two such ports would
/// both bind at one address, and [`bind`] would silently take the first.
#[must_use]
pub const fn sigs_overlap(a: &Sig, b: &Sig) -> bool {
    if a.len != b.len
        || a.hash != b.hash
        || a.calls.len() != b.calls.len()
        || a.imports.len() != b.imports.len()
    {
        return false;
    }
    let mut i = 0;
    while i < a.calls.len() {
        if a.calls[i].0 != b.calls[i].0 || !callees_overlap(&a.calls[i].1, &b.calls[i].1) {
            return false;
        }
        i += 1;
    }
    let mut i = 0;
    while i < a.imports.len() {
        if a.imports[i].0 != b.imports[i].0 || !same_str(a.imports[i].1, b.imports[i].1) {
            return false;
        }
        i += 1;
    }
    true
}

/// The first pair `(i, j)`, `i < j`, of entries of `sigs` that could bind
/// at the same address ([`sigs_overlap`]); indices past `a.len()` are in
/// `b`. [`PORTS`] and [`INTERNAL`] are checked with it when the crate
/// builds, so a second registration of one identity does not compile:
///
/// ```compile_fail
/// use ffx_magic::{first_overlap, runtime::ZERO_RET};
/// const _: () = assert!(first_overlap(&[("a", &ZERO_RET)], &[("b", &ZERO_RET)]).is_none());
/// ```
#[must_use]
pub const fn first_overlap(a: &[(&str, &Sig)], b: &[(&str, &Sig)]) -> Option<(usize, usize)> {
    let n = a.len() + b.len();
    let mut i = 0;
    while i < n {
        let x = if i < a.len() {
            a[i].1
        } else {
            b[i - a.len()].1
        };
        let mut j = i + 1;
        while j < n {
            let y = if j < a.len() {
                a[j].1
            } else {
                b[j - a.len()].1
            };
            if sigs_overlap(x, y) {
                return Some((i, j));
            }
            j += 1;
        }
        i += 1;
    }
    None
}

/// Every port: its name, signature and a dispatcher index. A signature
/// registered twice (here or in [`INTERNAL`]) does not build: one port
/// per code identity.
macro_rules! ports {
    ($($name:literal => $sig:path, $m:ident :: $f:ident;)*) => {
        /// The ports, in [`bind`]'s order: `(name, signature)`.
        pub const PORTS: &[(&str, &Sig)] = &[$(($name, &$sig)),*];

        // the pairwise check outgrew the const evaluator's step budget
        // (about 1,000 signatures); it is quadratic, not stuck
        #[allow(long_running_const_eval)]
        const _: () = assert!(
            first_overlap(PORTS, INTERNAL).is_none(),
            "two ports (PORTS or INTERNAL) with the same code identity: \
             keep one (cargo test -p ffx-magic one_port_per_identity names them)"
        );

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
    "opu part init data" => opu_parts::INIT_DATA, opu_parts::init_data;
    "opu part alloc" => opu_parts::ALLOC, opu_parts::alloc;
    "opu part run" => opu_parts::RUN, opu_parts::run;
    "opu parts save and free" => opu_parts::SAVE_AND_FREE, opu_parts::save_and_free;
    "opu parts restore" => opu_parts::RESTORE, opu_parts::restore;
    "opu parts clear and free" => opu_parts::CLEAR_AND_FREE, opu_parts::clear_and_free;
    "opu part free" => opu_parts::FREE, opu_parts::free;
    "opu parts forget" => opu_parts::FORGET, opu_parts::forget;
    "opu init env" => opu_parts::INIT_ENV, opu_parts::init_env;
    "m0604 start-up" => m0604::E1660, m0604::e1660;
    "m0604 shutdown" => m0604::E1820, m0604::e1820;
    "l1 opu start-up (set-up tail)" => l1_opu::START_UP, l1_opu::start_up;
    "l1 opu part init data (found: data)" => l1_opu::INIT_DATA, l1_opu::init_data;
    "m0003 tick handler" => m0003::TICK_HANDLER, m0003::tick_handler;
    "m0003 follow handler" => m0003::FOLLOW, m0003::follow;
    "m0003 load steps" => m0003::LOAD_STEPS, m0003::load_steps;
    "m0078 clock" => m0078::CLOCK, m0078::clock;
    "xor eax, eax; ret" => runtime::ZERO_RET, runtime::zero_ret;
    "node visit thunk" => nodes::THUNK_VISIT, nodes::thunk_visit;
    "node visit thunk (vu0)" => nodes::THUNK_VISIT_VU0, nodes::thunk_visit_vu0;
    "node visit thunk (slot 268)" => nodes::THUNK_VISIT_268, nodes::thunk_visit_268;
    "node visit thunk (children)" => nodes::THUNK_VISIT_CHILDREN, nodes::thunk_visit_children;
    "node visit plain" => nodes::PLAIN, nodes::visit_plain;
    "node visit gated" => nodes::GATED, nodes::visit_gated;
    "node visit add float" => nodes::ADD, nodes::visit_add_float;
    "node unlink list" => nodes::UNLINK_LIST, nodes::unlink_list;
    "node flag call" => nodes::FLAG_CALL, nodes::flag_method_call;
    "node flag pointer call" => nodes::FLAG_PTR, nodes::flag_pointer_call;
    "node all free tail" => nodes::FREE_TAIL, nodes::all_free_tail;
    "node free then unlink" => nodes::FREE_THEN_UNLINK, nodes::free_then_unlink;
    "node insert" => nodes::INSERT, nodes::node_insert;
    "node unlink" => nodes::UNLINK, nodes::unlink;
    "opu start-up (byte flag)" => l3_opu::START_UP, l3_opu::start_up;
    "m0154 start-up" => m0154::E1690, m0154::e1690;
    "m0163 particle step" => m0163::E1D10, m0163::e1d10;
    "m0153 data wait" => m0153::E8920, m0153::e8920;
    "m0170 start-up" => m0170::START, m0170::start;
    "m0187 wrapper 1000" => m0187::E1000, m0187::e1000;
    "m0187 wrapper 1050" => m0187::E1050, m0187::e1050;
    "m0187 wrapper 10a0" => m0187::E10A0, m0187::e10a0;
    "m0187 wrapper 1170" => m0187::E1170, m0187::e1170;
    "m0187 wrapper 10f0" => m0187::E10F0, m0187::e10f0;
    "m0194 effect wrapper 1230" => m0194::E1230, m0194::e1230;
    "m0196 effect wrapper 11d0" => m0196::E11D0, m0196::e11d0;
    "opu part init data (mode 1)" => opu_parts::INIT_DATA_1, opu_parts::init_data;
    "opu boot a" => l5_opu_boot::BOOT_A, l5_opu_boot::boot_a;
    "opu boot b" => l5_opu_boot::BOOT_B, l5_opu_boot::boot_b;
    "opu boot c" => l5_opu_boot::BOOT_C, l5_opu_boot::boot_c;
    "opu shutdown" => l5_opu_boot::SHUTDOWN, l5_opu_boot::shutdown;
    "objhost start" => l5_objhost::START, l5_objhost::start;
    "objhost end" => l5_objhost::END, l5_objhost::end;
    "objhost draw" => l5_objhost::DRAW, l5_objhost::draw;
    "objhost update" => l5_objhost::UPDATE, l5_objhost::update;
    "objhost countdown" => l5_objhost::COUNTDOWN, l5_objhost::countdown;
    "objhost task step" => l5_objhost::TASK_STEP, l5_objhost::task_step;
    "objhost finish" => l5_objhost::FINISH, l5_objhost::finish;
    "m0219 per-frame" => m0219::PER_FRAME, m0219::per_frame;
    "rcef effect ready seq" => rcef::EFFECT_READY_SEQ, rcef::effect_ready_seq;
    "rcef create task wait" => rcef::MAIN_CREATE_TASK_WAIT, rcef::main_create_task_wait;
    "m0334 set seq, call seq" => rcef::SET_SEQ_CALL_SEQ, rcef::set_seq_call_seq;
    "m0334 main init, create task" => rcef::MAIN_INIT_CREATE_TASK, rcef::main_init_create_task;
    "m0335 create task pair" => rcef::CREATE_TASK_PAIR, rcef::create_task_pair;
    "m0336 obj attach, set par C" => rcef::OBJ_ATTACH_SET_PAR_C, rcef::obj_attach_set_par_c;
    "m0336 create task wait" => rcef::CREATE_TASK_WAIT, rcef::create_task_wait;
    "m0336 damage seq" => rcef::DAMAGE_SEQ, rcef::damage_seq;
    "m0337 obj attach, set par B" => rcef::OBJ_ATTACH_SET_PAR_B, rcef::obj_attach_set_par_b;
    "m0339 task seq, no args" => rcef::TASK_SEQ_NO_ARGS, rcef::task_seq_no_args;
    "m0339 obj attach, set par" => rcef::OBJ_ATTACH_SET_PAR, rcef::obj_attach_set_par;
    "m0018 bounce step" => m0018::BOUNCE_STEP, m0018::bounce_step;
    "m0019 spin step" => m0019::SPIN_STEP, m0019::spin_step;
    "m0020 step 1000" => m0020::E1000, m0020::e1000;
    "m0020 step 1030" => m0020::E1030, m0020::e1030;
    "m0020 step 1060" => m0020::E1060, m0020::e1060;
    "m0020 step 1080" => m0020::E1080, m0020::e1080;
    "m0020 step 10d0 (fade)" => m0020::E10D0, m0020::e10d0;
    "m0020 object 13d0" => m0020::E13D0, m0020::e13d0;
    "m0020 object 1470" => m0020::E1470, m0020::e1470;
    "m0020 object 16e0" => m0020::E16E0, m0020::e16e0;
    "m0020 object 17b0" => m0020::E17B0, m0020::e17b0;
    "m0020 start-up 1a10" => m0020::E1A10, m0020::e1a10;
    "m0020 frame 1e00" => m0020::E1E00, m0020::e1e00;
    "m0042 visit, pool" => m0042::VISIT_POOL, m0042::visit_pool;
    "m0045 object 1250" => m0045::E1250, m0045::e1250;
    "m0045 object 1470" => m0045::E1470, m0045::e1470;
    "m0045 object 15a0" => m0045::E15A0, m0045::e15a0;
    "m0045 object 16b0" => m0045::E16B0, m0045::e16b0;
    "m0045 object 18b0" => m0045::E18B0, m0045::e18b0;
    "m0045 object 19c0" => m0045::E19C0, m0045::e19c0;
    "m0045 start-up" => m0045::E1C10, m0045::e1c10;
    "m0070 step d3c0" => m0070::E_D3C0, m0070::e_d3c0;
    "m0070 step d410" => m0070::STEP_D410, m0070::step_d410;
    "m0071 spin visit" => m0071::E3F00, m0071::e3f00;
    "m0072 place" => m0072::PLACE, m0072::place;
    "m0090 step" => m0090::STEP, m0090::step;
    "m0098 show" => m0098::SHOW, m0098::show;
    "m0098 hide" => m0098::HIDE, m0098::hide;
    "m0098 draw" => m0098::DRAW, m0098::draw;
    "m0098 start" => m0098::START, m0098::start;
    "m0098 load" => m0098::LOAD, m0098::load;
    "m0020 step 1d90" => m0020::E1D90, m0020::e1d90;
    "m0390 task step" => m0390::E13F0, m0390::e13f0;
    "m0390 entry 15b0" => m0390::E15B0, m0390::e15b0;
    "m0390 object callback 15f0" => m0390::E15F0, m0390::e15f0;
    "m0390 entry 1be0 (curve)" => m0390::E1BE0, m0390::e1be0;
    "m0390 entry 2100" => m0390::E2100, m0390::e2100;
    "m0481 start" => m0481::START, m0481::start;
    "m0481 end" => m0481::END, m0481::end;
    "m0596 start" => m0596::START, m0596::start;
    "m0596 copy height" => m0596::COPY_HEIGHT, m0596::copy_height;
    "m0393 task step" => m0393::STEP, m0393::step;
    "m0393 models" => m0393::MODELS, m0393::models;
    "m0393 attach" => m0393::ATTACH, m0393::attach;
    "m0393 camera above" => m0393::CAMERA_ABOVE, m0393::camera_above;
    "m0393 no target" => m0393::NO_TARGET, m0393::no_target;
    "m0393 place" => m0393::PLACE, m0393::place;
    "m0393 update" => m0393::UPDATE, m0393::update;
    "m0393 cue" => m0393::CUE, m0393::cue;
    "m0393 frame" => m0393::FRAME, m0393::frame;
    "m0393 eye" => m0393::EYE, m0393::eye;
    "m0395 vibrate" => m0395::VIBRATE, m0395::vibrate;
    "m0396 start" => m0396::START, m0396::start;
    "m0396 vibrate" => m0396::VIBRATE, m0396::vibrate;
    "m0396 cue" => m0396::CUE, m0396::cue;
    "m0396 fade" => m0396::FADE, m0396::fade;
    "m0396 fade start" => m0396::FADE_START, m0396::fade_start;
    "m0396 weapon frame" => m0396::WEAPON_FRAME, m0396::weapon_frame;
    "m0394 task step" => ob_summon::STEP_0394, ob_summon::step;
    "m0395 task step" => ob_summon::STEP_0395, ob_summon::step;
    "m0396 task step" => ob_summon::STEP_0396, ob_summon::step;
    "m0397 task step" => ob_summon::STEP_0397, ob_summon::step;
    "m0596 task step (no pause)" => ob_summon::STEP_PLAIN_0596, ob_summon::step_plain;
    "m0430 task step (no pause)" => ob_summon::STEP_PLAIN_0430, ob_summon::step_plain;
    "m0431 task step (no pause)" => ob_summon::STEP_PLAIN_0431, ob_summon::step_plain;
    "m0432 task step (no pause)" => ob_summon::STEP_PLAIN_0432, ob_summon::step_plain;
    "m0434 task step (no pause)" => ob_summon::STEP_PLAIN_0434, ob_summon::step_plain;
    "m0447 task step (no pause)" => ob_summon::STEP_PLAIN_0447, ob_summon::step_plain;
    "m0511 task step (no pause)" => ob_summon::STEP_PLAIN_0511, ob_summon::step_plain;
    "m0586 task step (no pause)" => ob_summon::STEP_PLAIN_0586, ob_summon::step_plain;
    "m0587 task step (no pause)" => ob_summon::STEP_PLAIN_0587, ob_summon::step_plain;
    "m0588 task step (no pause)" => ob_summon::STEP_PLAIN_0588, ob_summon::step_plain;
    "m0589 task step (no pause)" => ob_summon::STEP_PLAIN_0589, ob_summon::step_plain;
    "m0598 task step (no pause)" => ob_summon::STEP_PLAIN_0598, ob_summon::step_plain;
    "m0599 task step (no pause)" => ob_summon::STEP_PLAIN_0599, ob_summon::step_plain;
    "m0610 task step (no pause)" => ob_summon::STEP_PLAIN_0610, ob_summon::step_plain;
    "m0638 task step (no pause)" => ob_summon::STEP_PLAIN_0638, ob_summon::step_plain;
    "m0673 task step (no pause)" => ob_summon::STEP_PLAIN_0673, ob_summon::step_plain;
    "m0689 task step (no pause)" => ob_summon::STEP_PLAIN_0689, ob_summon::step_plain;
    "m0394 afterimage" => ob_summon::AFTERIMAGE_0394, ob_summon::afterimage;
    "m0395 afterimage" => ob_summon::AFTERIMAGE_0395, ob_summon::afterimage;
    "m0396 afterimage" => ob_summon::AFTERIMAGE_0396, ob_summon::afterimage;
    "m0397 afterimage" => ob_summon::AFTERIMAGE_0397, ob_summon::afterimage;
    "m0448 afterimage" => ob_summon::AFTERIMAGE_0448, ob_summon::afterimage;
    "m0394 afterimage fade" => ob_summon::AFTERIMAGE_FADE_0394, ob_summon::afterimage_fade;
    "m0395 afterimage fade" => ob_summon::AFTERIMAGE_FADE_0395, ob_summon::afterimage_fade;
    "m0396 afterimage fade" => ob_summon::AFTERIMAGE_FADE_0396, ob_summon::afterimage_fade;
    "m0397 afterimage fade" => ob_summon::AFTERIMAGE_FADE_0397, ob_summon::afterimage_fade;
    "m0447 afterimage fade" => ob_summon::AFTERIMAGE_FADE_0447, ob_summon::afterimage_fade;
    "m0448 afterimage fade" => ob_summon::AFTERIMAGE_FADE_0448, ob_summon::afterimage_fade;
    "m0394 flight" => m0394::FLIGHT, m0394::flight;
    "summon aim handler" => sa_summon::AIM, sa_summon::aim;
    "script wait off" => sa_script::WAIT_OFF, sa_script::wait_off;
    "script ret" => sa_script::RET, sa_script::ret;
    "script call" => sa_script::CALL, sa_script::call;
    "script wait word" => sa_script::WAIT_WORD, sa_script::wait_word;
    "script wait short" => sa_script::WAIT_SHORT, sa_script::wait_short;
    "script branch on" => sa_script::BRANCH_ON, sa_script::branch_on;
    "script record clear" => sa_script::RECORD_CLEAR, sa_script::record_clear;
    "script record set" => sa_script::RECORD_SET, sa_script::record_set;
    "script record vector" => sa_script::RECORD_VECTOR, sa_script::record_vector;
    "script record ramp" => sa_script::RECORD_RAMP, sa_script::record_ramp;
    "script record fade" => sa_script::RECORD_FADE, sa_script::record_fade;
    "script record fade to" => sa_script::RECORD_FADE_TO, sa_script::record_fade_to;
    "script flag bit" => sa_script::FLAG_BIT, sa_script::flag_bit;
    "script check then" => sa_script::CHECK_THEN, sa_script::check_then;
    "script actor call" => sa_script::ACTOR_CALL, sa_script::actor_call;
    "script effect start" => sa_script::EFFECT_START, sa_script::effect_start;
    "script effect pair" => sa_script::EFFECT_PAIR, sa_script::effect_pair;
    "script actor 546" => sa_script::ACTOR_546, sa_script::actor_546;
    "script actor 525" => sa_script::ACTOR_525, sa_script::actor_525;
    "script actor show" => sa_script::ACTOR_SHOW, sa_script::actor_show;
    "script actor visible" => sa_script::ACTOR_VISIBLE, sa_script::actor_visible;
    "script actor scaled" => sa_script::ACTOR_SCALED, sa_script::actor_scaled;
    "set bit 2" => sa_misc::SET_BIT2, sa_misc::set_bit2;
    "handle slot 435" => sa_misc::HANDLE_435, sa_misc::handle_435;
    "handle slot 437" => sa_misc::HANDLE_437, sa_misc::handle_437;
    "handle slot 804" => sa_misc::HANDLE_804, sa_misc::handle_804;
    "block reset" => sa_misc::BLOCK_RESET, sa_misc::block_reset;
    "host start" => sa_misc::HOST_START, sa_misc::host_start;
    "fade step" => sa_misc::FADE_STEP, sa_misc::fade_step;
    "host start (b)" => sa_misc::HOST_START_B, sa_misc::host_start_b;
    "part refresh (a)" => sa_misc::PART_REFRESH_A, sa_misc::part_refresh_a;
    "hook callback" => sa_misc::HOOK_CALLBACK, sa_misc::hook_callback;
    "wait pair d74c" => sa_script::WAIT_PAIR_D74C, sa_script::wait_pair_d74c;
    "wait pair 5c08" => sa_script::WAIT_PAIR_5C08, sa_script::wait_pair_5c08;
    "release part" => sa_misc::RELEASE_PART, sa_misc::release_part;
    "slot 250 (0, 0)" => sa_misc::SLOT250_ZERO, sa_misc::slot250_zero;
    "effect 0x78" => sa_misc::EFFECT_78, sa_misc::effect_78;
    "handle close" => sa_misc::HANDLE_CLOSE, sa_misc::handle_close;
    "part show" => sa_misc::PART_SHOW, sa_misc::part_show;
    "follow object" => sa_misc::FOLLOW_OBJECT, sa_misc::follow_object;
    "host start (c)" => sa_misc::HOST_START_C, sa_misc::host_start_c;
    "fx flag" => sa_misc::FX_FLAG, sa_misc::fx_flag;
    "handle 0x3015" => sa_misc::HANDLE_3015, sa_misc::handle_3015;
    "handle swap" => sa_misc::HANDLE_SWAP, sa_misc::handle_swap;
    "load wait" => sa_misc::LOAD_WAIT, sa_misc::load_wait;
    "fade states" => sa_misc::FADE_STATES, sa_misc::fade_states;
    "fx flag d618" => sa_misc::FX_FLAG_D618, sa_misc::fx_flag_d618;
    "wait pair 83f8" => sa_script::WAIT_PAIR_83F8, sa_script::wait_pair_83f8;
    "wait pair eab0" => sa_script::WAIT_PAIR_EAB0, sa_script::wait_pair_eab0;
    "release fx object" => sa_misc::RELEASE_FX_OBJECT, sa_misc::release_fx_object;
    "mem open" => sa_misc::MEM_OPEN, sa_misc::mem_open;
    "fade table" => sa_misc::FADE_TABLE, sa_misc::fade_table;
    "host start (d)" => sa_misc::HOST_START_D, sa_misc::host_start_d;
    "slot 239 pair" => sa_misc::SLOT239_PAIR, sa_misc::slot239_pair;
    "slot 399, done" => sa_misc::SLOT399_DONE, sa_misc::slot399_done;
    "hook pair" => sa_misc::HOOK_PAIR, sa_misc::hook_pair;
    "anim step" => sa_misc::ANIM_STEP, sa_misc::anim_step;
    // shared-B list
    "node children visit (+0x20)" => sb_nodes::VISIT_CHILDREN_20, sb_nodes::visit_children_20;
    "node visit, slot 668" => sb_nodes::VISIT_668, sb_nodes::visit_668;
    "node slot 779, flagged" => sb_nodes::CALL_779_FLAGGED, sb_nodes::call_779_flagged;
    "vm jump" => sb_vm::OP_JUMP, sb_vm::op_jump;
    "vm jump reg" => sb_vm::OP_JUMP_REG, sb_vm::op_jump_reg;
    "vm set 28" => sb_vm::OP_SET_28, sb_vm::op_set_28;
    "vm label" => sb_vm::OP_LABEL, sb_vm::op_label;
    "vm call" => sb_vm::OP_CALL, sb_vm::op_call;
    "vm set reg" => sb_vm::OP_SET_REG, sb_vm::op_set_reg;
    "vm 543" => sb_vm::OP_543, sb_vm::op_543;
    "vm 371" => sb_vm::OP_371, sb_vm::op_371;
    "vm loop" => sb_vm::OP_LOOP, sb_vm::op_loop;
    "vm 222" => sb_vm::OP_222, sb_vm::op_222;
    "vm 549 550" => sb_vm::OP_549_550, sb_vm::op_549_550;
    "vm 271 267" => sb_vm::OP_271_267, sb_vm::op_271_267;
    "vm select" => sb_vm::OP_SELECT, sb_vm::op_select;
    "vm 306" => sb_vm::OP_306, sb_vm::op_306;
    "vm loop end" => sb_vm::OP_LOOP_END, sb_vm::op_loop_end;
    "vm set vector" => sb_vm::OP_SET_VECTOR, sb_vm::op_set_vector;
    "vm hide" => sb_vm::OP_HIDE, sb_vm::op_hide;
    "vm show group" => sb_vm::OP_SHOW_GROUP, sb_vm::op_show_group;
    "vm timer" => sb_vm::OP_TIMER, sb_vm::op_timer;
    "vm set scaled" => sb_vm::OP_SET_SCALED, sb_vm::op_set_scaled;
    "vm fade 4" => sb_vm::OP_FADE_4, sb_vm::op_fade_4;
    "vm 256" => sb_vm::OP_256, sb_vm::op_256;
    "vm fade 6" => sb_vm::OP_FADE_6, sb_vm::op_fade_6;
    "slot 239 callback" => sb_common::CALL_239, sb_common::call_239;
    "start steps (225, 313, 549)" => sb_misc::START_STEPS, sb_misc::start_steps;
    "slot 439 entry" => sb_misc::CALL_439, sb_misc::call_439;
    "slot 438 entry" => sb_misc::CALL_438, sb_misc::call_438;
    "free part, 1" => sb_misc::FREE_PART_1, sb_misc::free_part_1;
    "load step" => sb_misc::LOAD_STEP, sb_misc::load_step;
    "op free part" => sb_misc::OP_FREE_PART, sb_misc::op_free_part;
    "place model" => sb_misc::PLACE_MODEL, sb_misc::place_model;
    "place model unscaled" => sb_misc::PLACE_MODEL_UNSCALED, sb_misc::place_model_unscaled;
    "state match (0x8528)" => sb_misc::STATE_MATCH_8528, sb_misc::state_match;
    "state match (0xebe0)" => sb_misc::STATE_MATCH_EBE0, sb_misc::state_match;
    "objhost start with 551" => sb_misc::START_551, sb_misc::start_551;
    "show 219" => sb_misc::SHOW_219, sb_misc::show_219;
    "free own part" => sb_misc::FREE_OWN_PART, sb_misc::free_own_part;
    "start seq" => sb_misc::START_SEQ, sb_misc::start_seq;
    // shared-3 list
    "thunk: place" => s3_wrap::JMP_PLACE, s3_wrap::ecx_jmp;
    "thunk: model setup" => s3_wrap::JMP_MODEL_SETUP, s3_wrap::ecx_jmp;
    "thunk: sub walk" => s3_wrap::JMP_SUB_WALK, s3_wrap::ecx_jmp;
    "thunk: scaled place" => s3_wrap::JMP_SCALED_PLACE, s3_wrap::ecx_jmp;
    "thunk: vector op" => s3_wrap::JMP_VECTOR_OP, s3_wrap::ecx_jmp;
    "thunk: summon bounce" => s3_wrap::JMP_SUMMON_BOUNCE, s3_wrap::ecx_jmp;
    "thunk: pair aim" => s3_wrap::JMP_PAIR_AIM, s3_wrap::ecx_jmp;
    "thunk: bone (scaled)" => s3_wrap::JMP_BONE_SCALED, s3_wrap::ecx_jmp;
    "thunk: add to parent" => s3_wrap::JMP_ADD_TO_PARENT, s3_wrap::ecx_jmp;
    "thunk: blur setup" => s3_wrap::JMP_BLUR_SETUP, s3_wrap::ecx_jmp;
    "thunk: visit pair 229" => s3_wrap::JMP_VISIT_PAIR_229, s3_wrap::ecx_jmp;
    "thunk: visit add vector" => s3_wrap::JMP_VISIT_ADD_VECTOR, s3_wrap::ecx_jmp;
    "thunk: visit offset (int)" => s3_wrap::JMP_VISIT_OFFSET_INT, s3_wrap::ecx_jmp;
    "thunk: visit offset (float)" => s3_wrap::JMP_VISIT_OFFSET_FLOAT, s3_wrap::ecx_jmp;
    "thunk: axis sprites" => s3_wrap::JMP_AXIS_SPRITES, s3_wrap::ecx_jmp;
    "thunk: node bounce" => s3_wrap::JMP_NODE_BOUNCE, s3_wrap::ecx_jmp;
    "thunk: draw model" => s3_wrap::JMP_DRAW_MODEL, s3_wrap::ecx_jmp;
    "thunk: run part" => s3_wrap::JMP_RUN_PART, s3_wrap::ecx_jmp;
    "thunk: aim parent" => s3_wrap::JMP_AIM_PARENT, s3_wrap::ecx_jmp;
    "thunk: rotation (0x52a0)" => s3_wrap::JMP_ROTATION_52A0, s3_wrap::ecx_edx_jmp;
    "thunk: rotation (0x19d0)" => s3_wrap::JMP_ROTATION_19D0, s3_wrap::ecx_edx_jmp;
    "thunk: rotation (0x47c0)" => s3_wrap::JMP_ROTATION_47C0, s3_wrap::ecx_edx_jmp;
    "thunk: rotation (0x4d40)" => s3_wrap::JMP_ROTATION_4D40, s3_wrap::ecx_edx_jmp;
    "thunk: rotation (0x4250)" => s3_wrap::JMP_ROTATION_4250, s3_wrap::ecx_edx_jmp;
    "thunk: bone (call)" => s3_wrap::CALL_BONE, s3_wrap::ecx_call;
    "vm each actor" => s3_vm::OP_EACH_ACTOR, s3_vm::op_each_actor;
    "vm show others" => s3_vm::OP_SHOW_OTHERS, s3_vm::op_show_others;
    "vm show all" => s3_vm::OP_SHOW_ALL, s3_vm::op_show_all;
    "vm actor record" => s3_vm::OP_ACTOR_RECORD, s3_vm::op_actor_record;
    "vm actor block" => s3_vm::OP_ACTOR_BLOCK, s3_vm::op_actor_block;
    "vm each actor unless listed" => s3_vm::OP_EACH_UNLESS_LISTED, s3_vm::op_each_unless_listed;
    "summon frame" => s3_summon::SUMMON_FRAME, s3_summon::summon_frame;
    "colour fade" => s3_misc::COLOUR_FADE, s3_misc::colour_fade;
    "colour step" => s3_misc::COLOUR_STEP, s3_misc::colour_step;
    "value step" => s3_misc::VALUE_STEP, s3_misc::value_step;
    // round 4, own list
    "o4 part stores (d8, ret 1) 0206@18a0" => o4_parts::STORES_D8_RET1_0206_18A0, o4_parts::stores_d8_ret1;
    "o4 part stores (d8, ret 1) 0255@1190" => o4_parts::STORES_D8_RET1_0255_1190, o4_parts::stores_d8_ret1;
    "o4 part stores (d8, ret 1) 0371@1000" => o4_parts::STORES_D8_RET1_0371_1000, o4_parts::stores_d8_ret1;
    "o4 part stores (d8, ret 1) 0376@1000" => o4_parts::STORES_D8_RET1_0376_1000, o4_parts::stores_d8_ret1;
    "o4 part stores (d8, ret 1) 0376@13e0" => o4_parts::STORES_D8_RET1_0376_13E0, o4_parts::stores_d8_ret1;
    "o4 part stores (d8, ret 1) 0376@1a00" => o4_parts::STORES_D8_RET1_0376_1A00, o4_parts::stores_d8_ret1;
    "o4 part stores (d8, ret 1) 0488@12b0" => o4_parts::STORES_D8_RET1_0488_12B0, o4_parts::stores_d8_ret1;
    "o4 part stores (d8, ret 1) 0489@18e0" => o4_parts::STORES_D8_RET1_0489_18E0, o4_parts::stores_d8_ret1;
    "o4 part stores (d8, ret 1) 0496@1270" => o4_parts::STORES_D8_RET1_0496_1270, o4_parts::stores_d8_ret1;
    "o4 part stores (d8, ret 1) 0664@1250" => o4_parts::STORES_D8_RET1_0664_1250, o4_parts::stores_d8_ret1;
    "o4 part stores (d8, ret 1) 0664@1bc0" => o4_parts::STORES_D8_RET1_0664_1BC0, o4_parts::stores_d8_ret1;
    "o4 part stores (d8, ret 1) 0664@1d80" => o4_parts::STORES_D8_RET1_0664_1D80, o4_parts::stores_d8_ret1;
    "o4 part stores (d8, ret 1) 0665@1250" => o4_parts::STORES_D8_RET1_0665_1250, o4_parts::stores_d8_ret1;
    "o4 part stores (d8, ret 1) 0665@13b0" => o4_parts::STORES_D8_RET1_0665_13B0, o4_parts::stores_d8_ret1;
    "o4 part stores (d8, ret 1) 0665@18d0" => o4_parts::STORES_D8_RET1_0665_18D0, o4_parts::stores_d8_ret1;
    "o4 part stores (d8, ret 1) 0665@1a70" => o4_parts::STORES_D8_RET1_0665_1A70, o4_parts::stores_d8_ret1;
    "o4 part stores (d8, ret 1) 0666@1670" => o4_parts::STORES_D8_RET1_0666_1670, o4_parts::stores_d8_ret1;
    "o4 part stores (d8, ret 1) 0666@1860" => o4_parts::STORES_D8_RET1_0666_1860, o4_parts::stores_d8_ret1;
    "o4 part stores (d8, ret 1) 0666@2500" => o4_parts::STORES_D8_RET1_0666_2500, o4_parts::stores_d8_ret1;
    "o4 part stores (d8, ret 1) 0666@26d0" => o4_parts::STORES_D8_RET1_0666_26D0, o4_parts::stores_d8_ret1;
    "o4 part stores (d8, ret 1) 0669@1810" => o4_parts::STORES_D8_RET1_0669_1810, o4_parts::stores_d8_ret1;
    "o4 part stores (d8, ret 1) 0669@19f0" => o4_parts::STORES_D8_RET1_0669_19F0, o4_parts::stores_d8_ret1;
    "o4 part stores (d8) 0375@1000" => o4_parts::STORES_D8_0375_1000, o4_parts::stores_d8;
    "o4 part stores (d8) 0375@17d0" => o4_parts::STORES_D8_0375_17D0, o4_parts::stores_d8;
    "o4 part stores (d8) 0375@1900" => o4_parts::STORES_D8_0375_1900, o4_parts::stores_d8;
    "o4 part stores (d8) 0375@1a80" => o4_parts::STORES_D8_0375_1A80, o4_parts::stores_d8;
    "o4 part stores (d8) 0375@1b20" => o4_parts::STORES_D8_0375_1B20, o4_parts::stores_d8;
    "o4 part stores (d8) 0375@1dd0" => o4_parts::STORES_D8_0375_1DD0, o4_parts::stores_d8;
    "o4 part stores (d8) 0375@1e70" => o4_parts::STORES_D8_0375_1E70, o4_parts::stores_d8;
    "o4 part stores (d8) 0375@1f10" => o4_parts::STORES_D8_0375_1F10, o4_parts::stores_d8;
    "o4 part stores (d8) 0376@1ff0" => o4_parts::STORES_D8_0376_1FF0, o4_parts::stores_d8;
    "o4 part stores (d8) 0376@21f0" => o4_parts::STORES_D8_0376_21F0, o4_parts::stores_d8;
    "o4 part stores (d8) 0487@1000" => o4_parts::STORES_D8_0487_1000, o4_parts::stores_d8;
    "o4 part stores (d8) 0498@1000" => o4_parts::STORES_D8_0498_1000, o4_parts::stores_d8;
    "o4 part stores (entry 0, ret 1) 0206@1650" => o4_parts::STORES_0_RET1_0206_1650, o4_parts::stores_0_ret1;
    "o4 part stores (entry 0, ret 1) 0255@1000" => o4_parts::STORES_0_RET1_0255_1000, o4_parts::stores_0_ret1;
    "o4 part stores (entry 0, ret 1) 0376@2290" => o4_parts::STORES_0_RET1_0376_2290, o4_parts::stores_0_ret1;
    "o4 part stores (entry 0, ret 1) 0497@1190" => o4_parts::STORES_0_RET1_0497_1190, o4_parts::stores_0_ret1;
    "o4 part stores (entry 0, ret 1) 0658@1000" => o4_parts::STORES_0_RET1_0658_1000, o4_parts::stores_0_ret1;
    "o4 part stores (entry 0, ret 1) 0669@1b90" => o4_parts::STORES_0_RET1_0669_1B90, o4_parts::stores_0_ret1;
    "o4 part stores (entry 0) 0372@1180" => o4_parts::STORES_0_0372_1180, o4_parts::stores_0;
    "o4 part stores (entry 0) 0498@1160" => o4_parts::STORES_0_0498_1160, o4_parts::stores_0;
    "o4 part stores (entry 0) 0536@1200" => o4_parts::STORES_0_0536_1200, o4_parts::stores_0;
    "o4 part stores (entry 0) 0539@1000" => o4_parts::STORES_0_0539_1000, o4_parts::stores_0;
    "o4 part stores (entry 0) 0561@1000" => o4_parts::STORES_0_0561_1000, o4_parts::stores_0;
    "o4 part stores (entry 0) 0616@1000" => o4_parts::STORES_0_0616_1000, o4_parts::stores_0;
    "o4 part stores (entry 0) 0617@1000" => o4_parts::STORES_0_0617_1000, o4_parts::stores_0;
    "o4 seq wait (447) 0191@1110" => o4_rcef::SEQ_WAIT_447_0191_1110, o4_rcef::seq_wait_447;
    "o4 seq wait (447) 0357@10a0" => o4_rcef::SEQ_WAIT_447_0357_10A0, o4_rcef::seq_wait_447;
    "o4 seq wait (447) 0360@1110" => o4_rcef::SEQ_WAIT_447_0360_1110, o4_rcef::seq_wait_447;
    "o4 seq wait (447) 0360@1210" => o4_rcef::SEQ_WAIT_447_0360_1210, o4_rcef::seq_wait_447;
    "o4 seq wait (447) 0361@1210" => o4_rcef::SEQ_WAIT_447_0361_1210, o4_rcef::seq_wait_447;
    "o4 seq wait (447) 0361@1490" => o4_rcef::SEQ_WAIT_447_0361_1490, o4_rcef::seq_wait_447;
    "o4 seq wait (447) 0384@12a0" => o4_rcef::SEQ_WAIT_447_0384_12A0, o4_rcef::seq_wait_447;
    "o4 seq wait (447) 0384@13b0" => o4_rcef::SEQ_WAIT_447_0384_13B0, o4_rcef::seq_wait_447;
    "o4 seq wait (447) 0421@12b0" => o4_rcef::SEQ_WAIT_447_0421_12B0, o4_rcef::seq_wait_447;
    "o4 seq wait (447) 0522@1780" => o4_rcef::SEQ_WAIT_447_0522_1780, o4_rcef::seq_wait_447;
    "o4 seq wait (447) 0525@1410" => o4_rcef::SEQ_WAIT_447_0525_1410, o4_rcef::seq_wait_447;
    "o4 seq wait (447) 0535@12a0" => o4_rcef::SEQ_WAIT_447_0535_12A0, o4_rcef::seq_wait_447;
    "o4 create task wait 0191@1160" => o4_rcef::CREATE_TASK_WAIT_0191_1160, rcef::create_task_wait;
    "o4 create task wait 0300@1120" => o4_rcef::CREATE_TASK_WAIT_0300_1120, rcef::create_task_wait;
    "o4 create task wait 0301@1a30" => o4_rcef::CREATE_TASK_WAIT_0301_1A30, rcef::create_task_wait;
    "o4 create task wait 0378@1080" => o4_rcef::CREATE_TASK_WAIT_0378_1080, rcef::create_task_wait;
    "o4 create task wait 0416@10a0" => o4_rcef::CREATE_TASK_WAIT_0416_10A0, rcef::create_task_wait;
    "o4 create task wait 0521@1090" => o4_rcef::CREATE_TASK_WAIT_0521_1090, rcef::create_task_wait;
    "o4 create task wait 0522@11b0" => o4_rcef::CREATE_TASK_WAIT_0522_11B0, rcef::create_task_wait;
    "o4 start turned 0596@7060" => o4_effect::START_TURNED_0596_7060, o4_effect::start_turned;
    "o4 start turned 0162@29f0" => o4_effect::START_TURNED_0162_29F0, o4_effect::start_turned;
    "o4 start turned 0304@1c10" => o4_effect::START_TURNED_0304_1C10, o4_effect::start_turned;
    "o4 start turned 0307@2430" => o4_effect::START_TURNED_0307_2430, o4_effect::start_turned;
    "o4 start turned 0568@1860" => o4_effect::START_TURNED_0568_1860, o4_effect::start_turned;
    "o4 start turned 0586@2750" => o4_effect::START_TURNED_0586_2750, o4_effect::start_turned;
    "o4 start turned 0587@1890" => o4_effect::START_TURNED_0587_1890, o4_effect::start_turned;
    "o4 start turned 0588@1b70" => o4_effect::START_TURNED_0588_1B70, o4_effect::start_turned;
    "o4 start turned 0589@1e00" => o4_effect::START_TURNED_0589_1E00, o4_effect::start_turned;
    "o4 start turned 0598@7470" => o4_effect::START_TURNED_0598_7470, o4_effect::start_turned;
    "o4 start turned 0605@7db0" => o4_effect::START_TURNED_0605_7DB0, o4_effect::start_turned;
    "o4 attach set par 0357@18e0" => o4_rcef::ATTACH_SET_PAR_0357_18E0, o4_rcef::attach_set_par;
    "o4 attach set par 0359@1820" => o4_rcef::ATTACH_SET_PAR_0359_1820, o4_rcef::attach_set_par;
    "o4 attach set par 0361@1be0" => o4_rcef::ATTACH_SET_PAR_0361_1BE0, o4_rcef::attach_set_par;
    "o4 attach set par 0416@1710" => o4_rcef::ATTACH_SET_PAR_0416_1710, o4_rcef::attach_set_par;
    "o4 attach set par 0521@1200" => o4_rcef::ATTACH_SET_PAR_0521_1200, o4_rcef::attach_set_par;
    "o4 attach set par 0521@12d0" => o4_rcef::ATTACH_SET_PAR_0521_12D0, o4_rcef::attach_set_par;
    "o4 attach set par 0522@1290" => o4_rcef::ATTACH_SET_PAR_0522_1290, o4_rcef::attach_set_par;
    "o4 attach set par 0526@1380" => o4_rcef::ATTACH_SET_PAR_0526_1380, o4_rcef::attach_set_par;
    "o4 task step (no pause) 0162@2190" => o4_steps::STEP_PLAIN_0162_2190, ob_summon::step_plain;
    "o4 task step (no pause) 0348@65c0" => o4_steps::STEP_PLAIN_0348_65C0, ob_summon::step_plain;
    "o4 task step (no pause) 0350@5a10" => o4_steps::STEP_PLAIN_0350_5A10, ob_summon::step_plain;
    "o4 task step (no pause) 0352@4c90" => o4_steps::STEP_PLAIN_0352_4C90, ob_summon::step_plain;
    "o4 task step (no pause) 0353@1940" => o4_steps::STEP_PLAIN_0353_1940, ob_summon::step_plain;
    "o4 task step (no pause) 0392@1460" => o4_steps::STEP_PLAIN_0392_1460, ob_summon::step_plain;
    "o4 step then register 0165@1760" => o4_steps::STEP_THEN_REGISTER_0165_1760, o4_steps::step_then_register;
    "o4 step then register 0307@15a0" => o4_steps::STEP_THEN_REGISTER_0307_15A0, o4_steps::step_then_register;
    "o4 step then register 0605@6ef0" => o4_steps::STEP_THEN_REGISTER_0605_6EF0, o4_steps::step_then_register;
    "o4 step then register 0612@7390" => o4_steps::STEP_THEN_REGISTER_0612_7390, o4_steps::step_then_register;
    "o4 step then register 0688@3a90" => o4_steps::STEP_THEN_REGISTER_0688_3A90, o4_steps::step_then_register;
    "o4 release node 0162@4120" => o4_steps::RELEASE_NODE_0162_4120, o4_steps::release_node;
    "o4 release node 0165@3050" => o4_steps::RELEASE_NODE_0165_3050, o4_steps::release_node;
    "o4 release node 0430@3190" => o4_steps::RELEASE_NODE_0430_3190, o4_steps::release_node;
    "o4 release node 0431@4360" => o4_steps::RELEASE_NODE_0431_4360, o4_steps::release_node;
    "o4 release node 0586@3d30" => o4_steps::RELEASE_NODE_0586_3D30, o4_steps::release_node;
    "o4 release node 0587@3af0" => o4_steps::RELEASE_NODE_0587_3AF0, o4_steps::release_node;
    "o4 release node 0588@2580" => o4_steps::RELEASE_NODE_0588_2580, o4_steps::release_node;
    "o4 sub const 0168@33a0" => o4_steps::SUB_CONST_0168_33A0, o4_steps::sub_const;
    "o4 sub const 0430@3860" => o4_steps::SUB_CONST_0430_3860, o4_steps::sub_const;
    "o4 sub const 0568@2820" => o4_steps::SUB_CONST_0568_2820, o4_steps::sub_const;
    "o4 sub const 0568@2dc0" => o4_steps::SUB_CONST_0568_2DC0, o4_steps::sub_const;
    "o4 sub const 0587@2ec0" => o4_steps::SUB_CONST_0587_2EC0, o4_steps::sub_const;
    "o4 pool node 0350@6280" => o4_steps::POOL_NODE_0350_6280, o4_steps::pool_node;
    "o4 pool node 0353@2180" => o4_steps::POOL_NODE_0353_2180, o4_steps::pool_node;
    "o4 pool node 0434@7a50" => o4_steps::POOL_NODE_0434_7A50, o4_steps::pool_node;
    "o4 pool node 0610@b980" => o4_steps::POOL_NODE_0610_B980, o4_steps::pool_node;
    "o4 pool node 0672@1860" => o4_steps::POOL_NODE_0672_1860, o4_steps::pool_node;
    "o4 store imm 0168@3400" => o4_small::STORE_IMM_0168_3400, o4_small::store_imm;
    "o4 store imm 0673@2030" => o4_small::STORE_IMM_0673_2030, o4_small::store_imm;
    "o4 store imm 0673@2060" => o4_small::STORE_IMM_0673_2060, o4_small::store_imm;
    "o4 store imm 0673@2070" => o4_small::STORE_IMM_0673_2070, o4_small::store_imm;
    "o4 slot imm 0303@14b0" => o4_small::SLOT_IMM_0303_14B0, o4_small::slot_imm;
    "o4 slot imm 0473@20c0" => o4_small::SLOT_IMM_0473_20C0, o4_small::slot_imm;
    "o4 slot imm 0511@2b20" => o4_small::SLOT_IMM_0511_2B20, o4_small::slot_imm;
    "o4 slot imm, 1 0443@1c00" => o4_small::SLOT_IMM_RET1_0443_1C00, o4_small::slot_imm_ret1;
    "o4 slot imm, 1 0540@1030" => o4_small::SLOT_IMM_RET1_0540_1030, o4_small::slot_imm_ret1;
    "o4 slot imm, 1 0540@1060" => o4_small::SLOT_IMM_RET1_0540_1060, o4_small::slot_imm_ret1;
    "o4 copy twice 0162@39c0" => o4_small::COPY_TWICE_0162_39C0, o4_small::copy_twice;
    "o4 copy twice 0304@3790" => o4_small::COPY_TWICE_0304_3790, o4_small::copy_twice;
    "o4 copy twice 0586@35d0" => o4_small::COPY_TWICE_0586_35D0, o4_small::copy_twice;
    "o4 slot 237, scale 0276@32f0" => o4_small::SLOT237_SCALE_0276_32F0, o4_small::slot237_scale;
    "o4 slot 237, scale 0305@1cb0" => o4_small::SLOT237_SCALE_0305_1CB0, o4_small::slot237_scale;
    "o4 slot 237, scale 0417@48e0" => o4_small::SLOT237_SCALE_0417_48E0, o4_small::slot237_scale;
    "o4 slots 719, 306 0417@4920" => o4_small::SLOT719_306_0417_4920, o4_small::slot719_306;
    "o4 slots 719, 306 0419@1d10" => o4_small::SLOT719_306_0419_1D10, o4_small::slot719_306;
    "o4 slots 719, 306 0432@20d0" => o4_small::SLOT719_306_0432_20D0, o4_small::slot719_306;
    "o4 slots 719, 306 0572@1630" => o4_small::SLOT719_306_0572_1630, o4_small::slot719_306;
    "o4 slot, seq, wait 0378@1040" => o4_small::SLOT_SEQ_WAIT_0378_1040, o4_small::slot_seq_wait;
    "o4 slot, seq, wait 0521@1130" => o4_small::SLOT_SEQ_WAIT_0521_1130, o4_small::slot_seq_wait;
    "o4 slot, seq, wait 0525@1150" => o4_small::SLOT_SEQ_WAIT_0525_1150, o4_small::slot_seq_wait;
    "o4 seq keep (447) 0193@1c70" => o4_small::SEQ_KEEP_447_0193_1C70, o4_small::seq_keep_447;
    "o4 seq keep (447) 0300@1280" => o4_small::SEQ_KEEP_447_0300_1280, o4_small::seq_keep_447;
    "o4 seq keep (447) 0525@15a0" => o4_small::SEQ_KEEP_447_0525_15A0, o4_small::seq_keep_447;
    "o4 seq keep (447) 0529@15c0" => o4_small::SEQ_KEEP_447_0529_15C0, o4_small::seq_keep_447;
    "o4 record node 0596@3900" => o4_small::RECORD_NODE_0596_3900, o4_small::record_node;
    "o4 record node 0348@29a0" => o4_small::RECORD_NODE_0348_29A0, o4_small::record_node;
    "o4 record node 0353@4a00" => o4_small::RECORD_NODE_0353_4A00, o4_small::record_node;
    "o4 record node 0612@3e60" => o4_small::RECORD_NODE_0612_3E60, o4_small::record_node;
    "o4 static record node 0163@2410" => o4_small::STATIC_RECORD_NODE_0163_2410, o4_small::static_record_node;
    "o4 static record node 0253@48d0" => o4_small::STATIC_RECORD_NODE_0253_48D0, o4_small::static_record_node;
    "o4 static record node 0298@2850" => o4_small::STATIC_RECORD_NODE_0298_2850, o4_small::static_record_node;
    "o4 static record node 0508@2b80" => o4_small::STATIC_RECORD_NODE_0508_2B80, o4_small::static_record_node;
    "o4 part and vectors 0477@11c0" => o4_small::PART_AND_VECTORS_0477_11C0, o4_small::part_and_vectors;
    "o4 part and vectors 0477@13a0" => o4_small::PART_AND_VECTORS_0477_13A0, o4_small::part_and_vectors;
    "o4 part and vectors 0486@11c0" => o4_small::PART_AND_VECTORS_0486_11C0, o4_small::part_and_vectors;
    "o4 part and vectors 0486@1360" => o4_small::PART_AND_VECTORS_0486_1360, o4_small::part_and_vectors;
    "o4 attach plain 8 0300@1300" => o4_rcef::ATTACH_PLAIN8_0300_1300, o4_rcef::attach_plain8;
    "o4 attach plain 8 0301@1de0" => o4_rcef::ATTACH_PLAIN8_0301_1DE0, o4_rcef::attach_plain8;
    "o4 attach plain 8 0510@15c0" => o4_rcef::ATTACH_PLAIN8_0510_15C0, o4_rcef::attach_plain8;
    "o4 attach plain 8 0529@1270" => o4_rcef::ATTACH_PLAIN8_0529_1270, o4_rcef::attach_plain8;
    "o4 attach plain 12 0301@1d00" => o4_rcef::ATTACH_PLAIN12_0301_1D00, o4_rcef::attach_plain12;
    "o4 attach plain 12 0360@1400" => o4_rcef::ATTACH_PLAIN12_0360_1400, o4_rcef::attach_plain12;
    "o4 attach plain 12 0360@14e0" => o4_rcef::ATTACH_PLAIN12_0360_14E0, o4_rcef::attach_plain12;
    "o4 attach plain 12 0510@14e0" => o4_rcef::ATTACH_PLAIN12_0510_14E0, o4_rcef::attach_plain12;
    "o4 fade start 0596@70f0" => o4_small::FADE_START_0596_70F0, o4_small::fade_start;
    "o4 fade start 0602@8460" => o4_small::FADE_START_0602_8460, o4_small::fade_start;
    "o4 fade start 0606@77b0" => o4_small::FADE_START_0606_77B0, o4_small::fade_start;
    "o4 tiny set_bit_global 0307@5220" => o4_tiny::SET_BIT_GLOBAL_0307_5220, o4_tiny::set_bit_global;
    "o4 tiny copy_global 0269@1740" => o4_tiny::COPY_GLOBAL_0269_1740, o4_tiny::copy_global;
    "o4 tiny set_one_ret0 0671@1330" => o4_tiny::SET_ONE_RET0_0671_1330, o4_tiny::set_one_ret0;
    "o4 tiny set_bit_field 0563@50f0" => o4_tiny::SET_BIT_FIELD_0563_50F0, o4_tiny::set_bit_field;
    "o4 tiny clear_bit_field 0563@5100" => o4_tiny::CLEAR_BIT_FIELD_0563_5100, o4_tiny::clear_bit_field;
    "o4 tiny is_zero_field 0434@95c0" => o4_tiny::IS_ZERO_FIELD_0434_95C0, o4_tiny::is_zero_field;
    "o4 tiny is_zero_field 0610@d490" => o4_tiny::IS_ZERO_FIELD_0610_D490, o4_tiny::is_zero_field;
    "o4 tiny slot_ecx 0577@eca0" => o4_tiny::SLOT_ECX_0577_ECA0, o4_tiny::slot_ecx;
    "o4 tiny slot_imm8 0577@ec80" => o4_tiny::SLOT_IMM8_0577_EC80, o4_tiny::slot_imm8;
    "o4 tiny slot_imm8x3 0519@4e40" => o4_tiny::SLOT_IMM8X3_0519_4E40, o4_tiny::slot_imm8x3;
    "o4 tiny copy_to_arg 0673@1b40" => o4_tiny::COPY_TO_ARG_0673_1B40, o4_tiny::copy_to_arg;
    "o4 tiny two_stores 0350@6300" => o4_tiny::TWO_STORES_0350_6300, o4_tiny::two_stores;
    "o4 tiny slot_7_0_arg 0150@eaf0" => o4_tiny::SLOT_7_0_ARG_0150_EAF0, o4_tiny::slot_7_0_arg;
    "o4 tiny set_callback 0353@2440" => o4_tiny::SET_CALLBACK_0353_2440, o4_tiny::set_callback;
    "o4 tiny slot_then_flag 0195@19c0" => o4_tiny::SLOT_THEN_FLAG_0195_19C0, o4_tiny::slot_then_flag;
    "o4 tiny slot_two_globals 0483@2650" => o4_tiny::SLOT_TWO_GLOBALS_0483_2650, o4_tiny::slot_two_globals;
    "o4 tiny add_const 0568@2de0" => o4_tiny::ADD_CONST_0568_2DE0, o4_tiny::add_const;
    "o4 tiny add_const 0673@2040" => o4_tiny::ADD_CONST_0673_2040, o4_tiny::add_const;
    "o4 tiny script_then_end 0671@1310" => o4_tiny::SCRIPT_THEN_END_0671_1310, o4_tiny::script_then_end;
    "o4 tiny slot_float 0263@8980" => o4_tiny::SLOT_FLOAT_0263_8980, o4_tiny::slot_float;
    "o4 tiny slot237_edx 0483@26a0" => o4_tiny::SLOT237_EDX_0483_26A0, o4_tiny::slot237_edx;
    "o4 tiny slot_keep_nonzero 0661@15b0" => o4_tiny::SLOT_KEEP_NONZERO_0661_15B0, o4_tiny::slot_keep_nonzero;
    "o4 tiny slot237_arg 0343@6970" => o4_tiny::SLOT237_ARG_0343_6970, o4_tiny::slot237_arg;
    "o4 tiny slot_then_store_ret1 0443@1ab0" => o4_tiny::SLOT_THEN_STORE_RET1_0443_1AB0, o4_tiny::slot_then_store_ret1;
    "o4 tiny slot_then_clear 0499@13c0" => o4_tiny::SLOT_THEN_CLEAR_0499_13C0, o4_tiny::slot_then_clear;
    "o4 tiny set_bits_indexed 0165@36f0" => o4_tiny::SET_BITS_INDEXED_0165_36F0, o4_tiny::set_bits_indexed;
    "o4 tiny clear_bits_indexed 0165@3720" => o4_tiny::CLEAR_BITS_INDEXED_0165_3720, o4_tiny::clear_bits_indexed;
    "o4 tiny slot_obj_word 0540@1000" => o4_tiny::SLOT_OBJ_WORD_0540_1000, o4_tiny::slot_obj_word;
    "o4 tiny set_four 0263@6f40" => o4_tiny::SET_FOUR_0263_6F40, o4_tiny::set_four;
    "o4 tiny add_const_twice 0162@3990" => o4_tiny::ADD_CONST_TWICE_0162_3990, o4_tiny::add_const_twice;
    "o4 tiny add_const_twice 0586@35a0" => o4_tiny::ADD_CONST_TWICE_0586_35A0, o4_tiny::add_const_twice;
    "o4 tiny sub_const_twice 0304@2df0" => o4_tiny::SUB_CONST_TWICE_0304_2DF0, o4_tiny::sub_const_twice;
    "o4 tiny field_slot_then_flag 0525@1750" => o4_tiny::FIELD_SLOT_THEN_FLAG_0525_1750, o4_tiny::field_slot_then_flag;
    "o4 tiny script_then_give_back 0638@3170" => o4_tiny::SCRIPT_THEN_GIVE_BACK_0638_3170, o4_tiny::script_then_give_back;
    "o4 tiny script_then_give_back 0689@3170" => o4_tiny::SCRIPT_THEN_GIVE_BACK_0689_3170, o4_tiny::script_then_give_back;
    "o4 tiny slot_keep_store_ret1 0661@1500" => o4_tiny::SLOT_KEEP_STORE_RET1_0661_1500, o4_tiny::slot_keep_store_ret1;
    "o4 tiny handle_release 0391@31a0" => o4_tiny2::HANDLE_RELEASE_0391_31A0, o4_tiny2::handle_release;
    "o4 tiny handle_start 0391@1680" => o4_tiny2::HANDLE_START_0391_1680, o4_tiny2::handle_start;
    "o4 tiny handle_start 0392@1e10" => o4_tiny2::HANDLE_START_0392_1E10, o4_tiny2::handle_start;
    "o4 tiny code_slot 0275@4160" => o4_tiny2::CODE_SLOT_0275_4160, o4_tiny2::code_slot;
    "o4 tiny fade_at 0511@2af0" => o4_tiny2::FADE_AT_0511_2AF0, o4_tiny2::fade_at;
    "o4 tiny slot_pair_k 0186@5360" => o4_tiny2::SLOT_PAIR_K_0186_5360, o4_tiny2::slot_pair_k;
    "o4 tiny slot_field_flag 0416@1d80" => o4_tiny2::SLOT_FIELD_FLAG_0416_1D80, o4_tiny2::slot_field_flag;
    "o4 tiny slot_field_flag 0521@1400" => o4_tiny2::SLOT_FIELD_FLAG_0521_1400, o4_tiny2::slot_field_flag;
    "o4 tiny slot_then_cb 0193@14a0" => o4_tiny2::SLOT_THEN_CB_0193_14A0, o4_tiny2::slot_then_cb;
    "o4 tiny byte_slot_chain 0304@37b0" => o4_tiny2::BYTE_SLOT_CHAIN_0304_37B0, o4_tiny2::byte_slot_chain;
    "o4 tiny byte_slot_chain 0304@37e0" => o4_tiny2::BYTE_SLOT_CHAIN_0304_37E0, o4_tiny2::byte_slot_chain;
    "o4 tiny slot3_then_store 0669@1000" => o4_tiny2::SLOT3_THEN_STORE_0669_1000, o4_tiny2::slot3_then_store;
    "o4 tiny seq_wait 0525@1190" => o4_tiny2::SEQ_WAIT_0525_1190, o4_tiny2::seq_wait;
    "o4 tiny done_when_flag 0101@31a0" => o4_tiny2::DONE_WHEN_FLAG_0101_31A0, o4_tiny2::done_when_flag;
    "o4 tiny done_script_objects 0383@1340" => o4_tiny2::DONE_SCRIPT_OBJECTS_0383_1340, o4_tiny2::done_script_objects;
    "o4 tiny store_then_slot4 0661@1670" => o4_tiny2::STORE_THEN_SLOT4_0661_1670, o4_tiny2::store_then_slot4;
    "o4 tiny count_down 0661@16b0" => o4_tiny2::COUNT_DOWN_0661_16B0, o4_tiny2::count_down;
    "o4 tiny script_motion_give_back 0672@1480" => o4_tiny2::SCRIPT_MOTION_GIVE_BACK_0672_1480, o4_tiny2::script_motion_give_back;
    "o4 tiny sub_chain 0168@33c0" => o4_tiny2::SUB_CHAIN_0168_33C0, o4_tiny2::sub_chain;
    "o4 tiny sub_chain 0587@2ee0" => o4_tiny2::SUB_CHAIN_0587_2EE0, o4_tiny2::sub_chain;
    "o4 tiny add_chain 0563@24f0" => o4_tiny2::ADD_CHAIN_0563_24F0, o4_tiny2::add_chain;
    "o4 tiny add_chain_stored 0307@29f0" => o4_tiny2::ADD_CHAIN_STORED_0307_29F0, o4_tiny2::add_chain_stored;
    "o4 tiny done_field 0473@17c0" => o4_tiny3::DONE_FIELD_0473_17C0, o4_tiny3::done_field;
    "o4 tiny done_field_register 0446@4ba0" => o4_tiny3::DONE_FIELD_REGISTER_0446_4BA0, o4_tiny3::done_field_register;
    "o4 tiny end_when_idle 0391@1380" => o4_tiny3::END_WHEN_IDLE_0391_1380, o4_tiny3::end_when_idle;
    "o4 tiny end_when_idle 0448@4c80" => o4_tiny3::END_WHEN_IDLE_0448_4C80, o4_tiny3::end_when_idle;
    "o4 tiny end_register 0606@69e0" => o4_tiny3::END_REGISTER_0606_69E0, o4_tiny3::end_register;
    "o4 tiny done_count_flag 0100@1680" => o4_tiny3::DONE_COUNT_FLAG_0100_1680, o4_tiny3::done_count_flag;
    "o4 tiny done_motion_word 0246@17d0" => o4_tiny3::DONE_MOTION_WORD_0246_17D0, o4_tiny3::done_motion_word;
    "o4 tiny done_motion_count 0444@3000" => o4_tiny3::DONE_MOTION_COUNT_0444_3000, o4_tiny3::done_motion_count;
    "o4 tiny done_motion_field 0275@3a00" => o4_tiny3::DONE_MOTION_FIELD_0275_3A00, o4_tiny3::done_motion_field;
    "o4 tiny done_then_grid 0243@1840" => o4_tiny3::DONE_THEN_GRID_0243_1840, o4_tiny3::done_then_grid;
    "o4 tiny script_motion_release 0432@1310" => o4_tiny3::SCRIPT_MOTION_RELEASE_0432_1310, o4_tiny3::script_motion_release;
    "o4 tiny slot237_scale32 0165@2ed0" => o4_tiny3::SLOT237_SCALE32_0165_2ED0, o4_tiny3::slot237_scale32;
    "o4 tiny slot237_scale32 0590@1640" => o4_tiny3::SLOT237_SCALE32_0590_1640, o4_tiny3::slot237_scale32;
    "o4 tiny task_then_cb 0526@1100" => o4_tiny3::TASK_THEN_CB_0526_1100, o4_tiny3::task_then_cb;
    "o4 tiny two_tasks_flag 0301@1cc0" => o4_tiny3::TWO_TASKS_FLAG_0301_1CC0, o4_tiny3::two_tasks_flag;
    "o4 tiny clear_bit_four 0307@5230" => o4_tiny3::CLEAR_BIT_FOUR_0307_5230, o4_tiny3::clear_bit_four;
    "o4 tiny six_stores 0303@1660" => o4_tiny3::SIX_STORES_0303_1660, o4_tiny3::six_stores;
    "o4 tiny six_stores 0303@16a0" => o4_tiny3::SIX_STORES_0303_16A0, o4_tiny3::six_stores;
    "o4 tiny record_six 0253@18a0" => o4_tiny3::RECORD_SIX_0253_18A0, o4_tiny3::record_six;
    "o4 tiny grow_after 0307@4da0" => o4_tiny3::GROW_AFTER_0307_4DA0, o4_tiny3::grow_after;
    "o4 tiny sub_two 0304@2ef0" => o4_tiny3::SUB_TWO_0304_2EF0, o4_tiny3::sub_two;
    "o4 tiny add_two 0304@3500" => o4_tiny3::ADD_TWO_0304_3500, o4_tiny3::add_two;
    "o4 tiny sub_copy 0304@34c0" => o4_tiny3::SUB_COPY_0304_34C0, o4_tiny3::sub_copy;
    "o4 tiny slot_keep_two 0430@28c0" => o4_tiny3::SLOT_KEEP_TWO_0430_28C0, o4_tiny3::slot_keep_two;
    "o4 tiny slot_keep_two 0588@2090" => o4_tiny3::SLOT_KEEP_TWO_0588_2090, o4_tiny3::slot_keep_two;
    "o4 tiny three_slots 0416@1530" => o4_tiny3::THREE_SLOTS_0416_1530, o4_tiny3::three_slots;
    "o4 tiny count_then_task 0360@18f0" => o4_tiny3::COUNT_THEN_TASK_0360_18F0, o4_tiny3::count_then_task;
    "o4 tiny free_part_of 0537@1260" => o4_tiny3::FREE_PART_OF_0537_1260, o4_tiny3::free_part_of;
    "o4 tiny free_parts 0641@1970" => o4_tiny3::FREE_PARTS_0641_1970, o4_tiny3::free_parts;
    "o4 tiny float_product 0253@2bf0" => o4_tiny3::FLOAT_PRODUCT_0253_2BF0, o4_tiny3::float_product;
    "o4 tiny index_slot 0351@2730" => o4_tiny4::INDEX_SLOT_0351_2730, o4_tiny4::index_slot;
    "o4 tiny slot_three_words 0708@1830" => o4_tiny4::SLOT_THREE_WORDS_0708_1830, o4_tiny4::slot_three_words;
    "o4 tiny slot_by_count 0197@1500" => o4_tiny4::SLOT_BY_COUNT_0197_1500, o4_tiny4::slot_by_count;
    "o4 tiny record_float 0273@7690" => o4_tiny4::RECORD_FLOAT_0273_7690, o4_tiny4::record_float;
    "o4 tiny slots_g_g_none 0497@1150" => o4_tiny4::SLOTS_G_G_NONE_0497_1150, o4_tiny4::slots_g_g_none;
    "o4 tiny three_slots_m 0514@3180" => o4_tiny4::THREE_SLOTS_M_0514_3180, o4_tiny4::three_slots_m;
    "o4 tiny add_and_stores 0303@16e0" => o4_tiny4::ADD_AND_STORES_0303_16E0, o4_tiny4::add_and_stores;
    "o4 tiny slot_seq_call 0361@17a0" => o4_tiny4::SLOT_SEQ_CALL_0361_17A0, o4_tiny4::slot_seq_call;
    "o4 tiny field_slots_seq 0191@12f0" => o4_tiny4::FIELD_SLOTS_SEQ_0191_12F0, o4_tiny4::field_slots_seq;
    "o4 tiny slot237_obj 0348@6df0" => o4_tiny4::SLOT237_OBJ_0348_6DF0, o4_tiny4::slot237_obj;
    "o4 tiny done_two_fields_register 0205@15c0" => o4_tiny4::DONE_TWO_FIELDS_REGISTER_0205_15C0, o4_tiny4::done_two_fields_register;
    "o4 tiny push_task 0146@7170" => o4_tiny4::PUSH_TASK_0146_7170, o4_tiny4::push_task;
    "o4 tiny task_while_count 0359@1530" => o4_tiny4::TASK_WHILE_COUNT_0359_1530, o4_tiny4::task_while_count;
    "o4 tiny slot_task_cb 0528@10d0" => o4_tiny4::SLOT_TASK_CB_0528_10D0, o4_tiny4::slot_task_cb;
    "o4 tiny done_all_grid 0270@1410" => o4_tiny4::DONE_ALL_GRID_0270_1410, o4_tiny4::done_all_grid;
    "o4 tiny set_two_seq_call 0359@1000" => o4_tiny4::SET_TWO_SEQ_CALL_0359_1000, o4_tiny4::set_two_seq_call;
    "o4 tiny set_two_seq_call 0360@1000" => o4_tiny4::SET_TWO_SEQ_CALL_0360_1000, o4_tiny4::set_two_seq_call;
    "o4 tiny byte_slot_part 0430@2450" => o4_tiny4::BYTE_SLOT_PART_0430_2450, o4_tiny4::byte_slot_part;
    "o4 tiny vectors_then_float 0388@6810" => o4_tiny4::VECTORS_THEN_FLOAT_0388_6810, o4_tiny4::vectors_then_float;
    "o4 tiny seq_wait_short 0194@10a0" => o4_tiny5::SEQ_WAIT_SHORT_0194_10A0, o4_tiny5::seq_wait_short;
    "o4 tiny seq_wait_short 0525@1600" => o4_tiny5::SEQ_WAIT_SHORT_0525_1600, o4_tiny5::seq_wait_short;
    "o4 tiny slot3_seq_wait 0356@17a0" => o4_tiny5::SLOT3_SEQ_WAIT_0356_17A0, o4_tiny5::slot3_seq_wait;
    "o4 tiny seven_stores 0380@1b10" => o4_tiny5::SEVEN_STORES_0380_1B10, o4_tiny5::seven_stores;
    "o4 tiny task_seq_call 0521@1170" => o4_tiny5::TASK_SEQ_CALL_0521_1170, o4_tiny5::task_seq_call;
    "o4 tiny two_slots_seq 0529@1210" => o4_tiny5::TWO_SLOTS_SEQ_0529_1210, o4_tiny5::two_slots_seq;
    "o4 tiny done_slot_h 0252@1540" => o4_tiny5::DONE_SLOT_H_0252_1540, o4_tiny5::done_slot_h;
    "o4 tiny done_count_bit 0492@31b0" => o4_tiny5::DONE_COUNT_BIT_0492_31B0, o4_tiny5::done_count_bit;
    "o4 tiny three_slots_obj 0514@3520" => o4_tiny5::THREE_SLOTS_OBJ_0514_3520, o4_tiny5::three_slots_obj;
    "o4 tiny task_keep_seq_wait 0382@1170" => o4_tiny5::TASK_KEEP_SEQ_WAIT_0382_1170, o4_tiny5::task_keep_seq_wait;
    "o4 tiny field_seq_call 0522@1200" => o4_tiny5::FIELD_SEQ_CALL_0522_1200, o4_tiny5::field_seq_call;
    "o4 tiny two_tasks_seq 0195@1300" => o4_tiny5::TWO_TASKS_SEQ_0195_1300, o4_tiny5::two_tasks_seq;
    "o4 tiny slot9_seq_wait 0360@1300" => o4_tiny5::SLOT9_SEQ_WAIT_0360_1300, o4_tiny5::slot9_seq_wait;
    "o4 tiny task_field_cb 0195@1270" => o4_tiny5::TASK_FIELD_CB_0195_1270, o4_tiny5::task_field_cb;
    "o4 tiny pair_seq_call 0378@1120" => o4_tiny5::PAIR_SEQ_CALL_0378_1120, o4_tiny5::pair_seq_call;
    "o4 fam2 slot_by_flag 0252@1340" => o4_fam2::SLOT_BY_FLAG_0252_1340, o4_fam2::slot_by_flag;
    "o4 fam2 slot_by_flag 0303@1420" => o4_fam2::SLOT_BY_FLAG_0303_1420, o4_fam2::slot_by_flag;
    "o4 fam2 slot_by_flag 0318@1540" => o4_fam2::SLOT_BY_FLAG_0318_1540, o4_fam2::slot_by_flag;
    "o4 fam2 seq_keep_447_32 0416@1d20" => o4_fam2::SEQ_KEEP_447_32_0416_1D20, o4_fam2::seq_keep_447_32;
    "o4 fam2 seq_keep_447_32 0510@1430" => o4_fam2::SEQ_KEEP_447_32_0510_1430, o4_fam2::seq_keep_447_32;
    "o4 fam2 seq_keep_447_32 0525@13b0" => o4_fam2::SEQ_KEEP_447_32_0525_13B0, o4_fam2::seq_keep_447_32;
    "o4 fam2 byte_list_slot 0397@63f0" => o4_fam2::BYTE_LIST_SLOT_0397_63F0, o4_fam2::byte_list_slot;
    "o4 fam2 byte_list_slot 0397@6450" => o4_fam2::BYTE_LIST_SLOT_0397_6450, o4_fam2::byte_list_slot;
    "o4 fam2 task_list_seq_wait 0313@1920" => o4_fam2::TASK_LIST_SEQ_WAIT_0313_1920, o4_fam2::task_list_seq_wait;
    "o4 fam2 task_list_seq_wait 0313@1980" => o4_fam2::TASK_LIST_SEQ_WAIT_0313_1980, o4_fam2::task_list_seq_wait;
    "o4 fam2 ptr_stores8 0307@2990" => o4_fam2::PTR_STORES8_0307_2990, o4_fam2::ptr_stores8;
    "o4 fam2 ptr_stores8 0563@2490" => o4_fam2::PTR_STORES8_0563_2490, o4_fam2::ptr_stores8;
    "o4 fam2 slot_task_seq_wait 0382@10a0" => o4_fam2::SLOT_TASK_SEQ_WAIT_0382_10A0, o4_fam2::slot_task_seq_wait;
    "o4 fam2 slot_task_seq_wait 0522@1090" => o4_fam2::SLOT_TASK_SEQ_WAIT_0522_1090, o4_fam2::slot_task_seq_wait;
    "o4 fam2 store_node 0353@2390" => o4_fam2::STORE_NODE_0353_2390, o4_fam2::store_node;
    "o4 fam2 store_node 0672@1cf0" => o4_fam2::STORE_NODE_0672_1CF0, o4_fam2::store_node;
    "o4 fam2 field_slot_flag8 0356@1840" => o4_fam2::FIELD_SLOT_FLAG8_0356_1840, o4_fam2::field_slot_flag8;
    "o4 fam2 field_slot_flag8 0529@1620" => o4_fam2::FIELD_SLOT_FLAG8_0529_1620, o4_fam2::field_slot_flag8;
    "o4 fam2 scale_actors 0165@33a0" => o4_fam2::SCALE_ACTORS_0165_33A0, o4_fam2::scale_actors;
    "o4 fam2 scale_actors 0606@7810" => o4_fam2::SCALE_ACTORS_0606_7810, o4_fam2::scale_actors;
    "o4 fam2 main init 0195@19e0" => o4_fam2::MAIN_INIT_0195_19E0, rcef::main_init_create_task;
    "o4 fam2 main init 0507@10d0" => o4_fam2::MAIN_INIT_0507_10D0, rcef::main_init_create_task;
    "o4 fam3 part_vectors 0370@1550" => o4_fam3::PART_VECTORS_0370_1550, o4_fam3::part_vectors;
    "o4 fam3 part_vectors 0370@18c0" => o4_fam3::PART_VECTORS_0370_18C0, o4_fam3::part_vectors;
    "o4 fam3 static_node 0267@37f0" => o4_fam3::STATIC_NODE_0267_37F0, o4_fam3::static_node;
    "o4 fam3 static_node 0311@24c0" => o4_fam3::STATIC_NODE_0311_24C0, o4_fam3::static_node;
    "o4 fam3 sub_stores8 0307@2a30" => o4_fam3::SUB_STORES8_0307_2A30, o4_fam3::sub_stores8;
    "o4 fam3 sub_stores8 0563@2530" => o4_fam3::SUB_STORES8_0563_2530, o4_fam3::sub_stores8;
    "o4 fam3 slot_list 0519@2a50" => o4_fam3::SLOT_LIST_0519_2A50, o4_fam3::slot_list;
    "o4 fam3 slot_list 0519@2ac0" => o4_fam3::SLOT_LIST_0519_2AC0, o4_fam3::slot_list;
    "o4 fam3 two_tasks_seq_wait 0194@10f0" => o4_fam3::TWO_TASKS_SEQ_WAIT_0194_10F0, o4_fam3::two_tasks_seq_wait;
    "o4 fam3 two_tasks_seq_wait 0300@10a0" => o4_fam3::TWO_TASKS_SEQ_WAIT_0300_10A0, o4_fam3::two_tasks_seq_wait;
    "o4 fam3 vector_globals 0165@4750" => o4_fam3::VECTOR_GLOBALS_0165_4750, o4_fam3::vector_globals;
    "o4 fam3 vector_globals 0563@5670" => o4_fam3::VECTOR_GLOBALS_0563_5670, o4_fam3::vector_globals;
    "o4 fam3 release_and_reset 0162@3900" => o4_fam3::RELEASE_AND_RESET_0162_3900, o4_fam3::release_and_reset;
    "o4 fam3 release_and_reset 0586@3510" => o4_fam3::RELEASE_AND_RESET_0586_3510, o4_fam3::release_and_reset;
    "o4 fam4 effect_record 0307@24c0" => o4_fam4::EFFECT_RECORD_0307_24C0, o4_fam4::effect_record;
    "o4 fam4 effect_record 0605@7e40" => o4_fam4::EFFECT_RECORD_0605_7E40, o4_fam4::effect_record;
    "o4 fam4 deal_then_slots 0391@2a20" => o4_fam4::DEAL_THEN_SLOTS_0391_2A20, o4_fam4::deal_then_slots;
    "o4 fam4 deal_then_slots 0448@6440" => o4_fam4::DEAL_THEN_SLOTS_0448_6440, o4_fam4::deal_then_slots;
    "o4 fam4 record_at_node 0673@1f10" => o4_fam4::RECORD_AT_NODE_0673_1F10, o4_fam4::record_at_node;
    "o4 fam4 record_at_node 0673@1fa0" => o4_fam4::RECORD_AT_NODE_0673_1FA0, o4_fam4::record_at_node;
    "o4 fam4 attach_par_seq 0312@15b0" => o4_fam4::ATTACH_PAR_SEQ_0312_15B0, o4_fam4::attach_par_seq;
    "o4 fam4 attach_par_seq 0378@1250" => o4_fam4::ATTACH_PAR_SEQ_0378_1250, o4_fam4::attach_par_seq;
    "o4 fam4 copy_then_rect 0511@2f70" => o4_fam4::COPY_THEN_RECT_0511_2F70, o4_fam4::copy_then_rect;
    "o4 fam4 copy_then_rect 0589@2b50" => o4_fam4::COPY_THEN_RECT_0589_2B50, o4_fam4::copy_then_rect;
    "o4 fam4 packet 0267@2f80" => o4_fam4::PACKET_0267_2F80, o4_fam4::packet;
    "o4 fam4 packet 0431@1430" => o4_fam4::PACKET_0431_1430, o4_fam4::packet;
    "o4 fam4 mixed_record_node 0473@2010" => o4_fam4::MIXED_RECORD_NODE_0473_2010, o4_fam4::mixed_record_node;
    "o4 fam4 mixed_record_node 0673@1850" => o4_fam4::MIXED_RECORD_NODE_0673_1850, o4_fam4::mixed_record_node;
    "o4 fam4 three_tasks_keep 0313@1240" => o4_fam4::THREE_TASKS_KEEP_0313_1240, o4_fam4::three_tasks_keep;
    "o4 fam4 three_tasks_keep 0361@1330" => o4_fam4::THREE_TASKS_KEEP_0361_1330, o4_fam4::three_tasks_keep;
    "o4 fam4 slot_attach_mode_par 0194@1620" => o4_fam4::SLOT_ATTACH_MODE_PAR_0194_1620, o4_fam4::slot_attach_mode_par;
    "o4 fam4 slot_attach_mode_par 0359@1420" => o4_fam4::SLOT_ATTACH_MODE_PAR_0359_1420, o4_fam4::slot_attach_mode_par;
}

/// Functions the ports reproduce inside others (no entry of their own:
/// custom register conventions, or reached only from ported code), for
/// the work list's coverage (`magic-sig --covered`).
pub const INTERNAL: &[(&str, &Sig)] = &[
    ("opu set-up", &opu::SETUP),
    ("opu printf wrapper", &opu::DEBUG_PRINTF),
    ("pppStartPart wrapper", &opu_parts::START_PART),
    ("m0003 tick", &m0003::TICK),
    ("m0003 node init", &m0003::NODE_INIT),
    ("m0032 free-list pop", &m0032::POP_NODE),
    ("m0032 state reset (0x14)", &m0032::RESET_STATE),
    ("m0032 state reset (0)", &m0032::RESET_STATE_0),
    ("m0032 state reset (edx, 0x12)", &m0032::RESET_STATE_EDX),
    ("m0074 pick", &m0074::PICK),
    ("m0052 model reset", &m0052::RESET_MODEL),
    ("m0042 node init", &m0042::NODE_INIT),
    ("m0070 node init (big)", &m0070::NODE_INIT_BIG),
    ("m0070 node init (root)", &m0070::NODE_INIT_ROOT),
    ("m0070 node init (small)", &m0070::NODE_INIT_SMALL),
    ("m0070 part free", &m0070::FREE_PART),
    ("m0070 node init (big, table)", &m0070::NODE_INIT_BIG_TABLE),
    ("m0070 model state", &m0070::MODEL_STATE),
    ("m0070 show", &m0070::SHOW),
    ("m0070 ring take", &m0070::RING_TAKE),
    ("m0070 part call (255)", &m0070::PART_CALL_255),
    ("m0070 part call (531)", &m0070::PART_CALL_531),
    ("m0070 model position", &m0070::MODEL_POSITION),
    ("m0070 hide model", &m0070::HIDE_MODEL),
    ("m0042 release", &m0042::RELEASE),
    ("m0042 set value", &m0042::SET_VALUE),
    ("m0042 unlink", &m0042::UNLINK),
    ("m0042 node init (big)", &m0042::NODE_INIT_BIG),
    ("m0042 time left", &m0042::TIME_LEFT),
    ("m0052 place", &m0052::PLACE),
    ("m0052 scaled camera", &m0052::SCALED_CAMERA),
    ("m0054 ppp init", &m0054::INIT_PPP),
    ("node visit", &nodes::VISIT),
    ("node visit (vu0)", &nodes::VISIT_VU0),
    ("node visit (slot 268)", &nodes::VISIT_268),
    ("node visit (children)", &nodes::VISIT_CHILDREN),
    ("character fade", &l3_common::CHR_FADE),
    ("list unlink current", &l3_common::LIST_UNLINK_CURRENT),
    ("character show", &l3_common::CHR_SHOW),
    ("m0159 facing angle", &m0159::FACING_ANGLE),
    ("m0163 target angle", &m0163::TARGET_ANGLE),
    ("m0166 reset record", &m0166::RESET_RECORD),
    ("scale node", &l3_common::SCALE_NODE),
    ("record node", &l3_common::RECORD_NODE),
    ("m0153 particle node", &m0153::PARTICLE_NODE),
    ("m0153 timed node", &m0153::TIMED_NODE),
    ("m0153 node drop", &m0153::NODE_DROP),
    ("m0153 owner node", &m0153::OWNER_NODE),
    ("m0153 release effects", &m0153::RELEASE_EFFECTS),
    ("m0153 ambient node", &m0153::AMBIENT_NODE),
    ("m0153 vector angle (+0x0, +0x8)", &m0153::VECTOR_ANGLE_XZ),
    ("m0153 vector angle (+0xc, +0x4)", &m0153::VECTOR_ANGLE_WY),
    ("objhost list_cur", &l5_objhost::LIST_CUR),
    ("objhost list_pop_tail", &l5_objhost::LIST_POP_TAIL),
    ("objhost list_remove", &l5_objhost::LIST_REMOVE),
    ("objhost pool_alloc", &l5_objhost::POOL_ALLOC),
    ("objhost tasks_init", &l5_objhost::TASKS_INIT),
    ("objhost run_group", &l5_objhost::RUN_GROUP),
    ("objhost obj_block_init", &l5_objhost::OBJ_BLOCK_INIT),
    ("objhost obj_block_fill", &l5_objhost::OBJ_BLOCK_FILL),
    ("objhost obj_script", &l5_objhost::OBJ_SCRIPT),
    ("objhost run_objects", &l5_objhost::RUN_OBJECTS),
    ("objhost part_init", &l5_objhost::PART_INIT),
    ("m0219 boot", &m0219::BOOT),
    ("m0219 make_object", &m0219::MAKE_OBJECT),
    ("m0219 object_init", &m0219::OBJECT_INIT),
    ("m0009 set-up", &m0009::SETUP),
    ("m0009 fill", &m0009::FILL),
    ("m0020 bars", &m0020::BARS),
    ("m0042 build", &m0042::BUILD),
    ("m0046 start object", &m0046::START_OBJECT),
    ("m0070 node init (scaled)", &m0070::NODE_INIT_SCALED),
    ("m0070 root child", &m0070::ROOT_CHILD),
    ("m0070 static child", &m0070::STATIC_CHILD),
    ("m0071 node init (vec)", &m0071::NODE_INIT_VEC),
    ("m0071 static node", &m0071::STATIC_NODE),
    ("m0071 root node", &m0071::ROOT_NODE),
    ("m0072 pool set-up", &m0072::POOL_SETUP),
    ("m0072 pool take", &m0072::POOL_TAKE),
    ("m0084 start object", &m0084::START_OBJECT),
    ("m0093 state reset (0x12)", &m0093::RESET_STATE_12),
    ("m0098 target height", &m0098::TARGET_HEIGHT),
    ("m0098 view matrix", &m0098::VIEW_MATRIX),
    ("m0481 free record", &m0481::FREE_RECORD),
    ("m0481 draw records", &m0481::DRAW_RECORDS),
    ("m0481 facing", &m0481::FACING),
    ("m0481 spawn", &m0481::SPAWN),
    ("m0596 place object", &m0596::PLACE_OBJECT),
    ("m0596 part init", &m0596::PART_INIT),
    ("m0596 object init", &m0596::OBJECT_INIT),
    ("m0596 place pair", &m0596::PLACE_PAIR),
    ("m0393 face", &m0393::FACE),
    ("m0394 root", &m0394::ROOT),
    ("m0394 scaled", &m0394::SCALED),
    ("m0394 place", &m0394::PLACE),
    ("m0395 face", &ob_summon::FACE_0395),
    ("m0396 face", &ob_summon::FACE_0396),
    ("m0396 aim", &m0396::AIM),
    ("m0396 refresh", &m0396::REFRESH),
    ("m0395 resets", &m0395::RESETS),
    ("pool take", &sa_pool::TAKE),
    ("pool give back", &sa_pool::GIVE_BACK),
    ("summon place", &sa_summon::PLACE),
    ("summon model setup", &sa_summon::MODEL_SETUP),
    ("summon sub walk", &sa_summon::SUB_WALK),
    ("summon scaled place", &sa_summon::SCALED_PLACE),
    ("summon model node init", &sa_summon::MODEL_NODE_INIT),
    ("summon bind models", &sa_summon::BIND_MODELS),
    ("summon vector op", &sa_summon::VECTOR_OP),
    ("summon bounce", &sa_summon::BOUNCE),
    ("summon pair aim", &sa_summon::PAIR_AIM),
    ("summon bone", &sa_summon::BONE),
    ("summon bone (scaled)", &sa_summon::BONE_SCALED),
    ("summon statics init", &sa_summon::STATICS_INIT),
    ("script each actor", &sa_script::EACH_ACTOR),
    ("script show all", &sa_script::SHOW_ALL),
    ("script actor block", &sa_script::ACTOR_BLOCK),
    ("set four", &sa_misc::SET_FOUR),
    ("reset transform", &sa_misc::RESET_TRANSFORM),
    ("unit block", &sa_misc::UNIT_BLOCK),
    ("node leave", &sa_misc::LEAVE),
    ("register parts", &sa_misc::REGISTER_PARTS),
    ("records init", &sa_misc::RECORDS_INIT),
    ("me block", &sa_misc::ME_BLOCK),
    ("/GS epilogue", &sa_misc::GS_EPILOGUE),
    ("script run", &sa_script::RUN),
    ("link part", &sa_misc::LINK_PART),
    ("timeline", &sa_misc::TIMELINE),
    ("drift", &sa_misc::DRIFT),
    ("reset transform (0)", &sa_misc::RESET_TRANSFORM_0),
    ("node append", &sa_misc::NODE_APPEND),
    ("reset transform (3)", &sa_misc::RESET_TRANSFORM_3),
    ("part refresh (b)", &sa_misc::PART_REFRESH_B),
    ("actor vector", &sa_script::ACTOR_VECTOR),
    ("add to parent", &sa_misc::ADD_TO_PARENT),
    ("node append (owned)", &sa_misc::NODE_APPEND_OWNED),
    ("reset transform (0x14)", &sa_misc::RESET_TRANSFORM_14),
    ("insert with", &sa_misc::INSERT_WITH),
    ("records init (0x400)", &sa_misc::RECORDS_INIT_400),
    ("ring take (0x50)", &sa_misc::RING_TAKE),
    ("move start", &sa_misc::MOVE_START),
    ("sprite init", &sa_misc::SPRITE_INIT),
    ("sprite fade", &sa_misc::SPRITE_FADE),
    ("timers reset", &sa_misc::TIMERS_RESET),
    ("part ranges", &sa_misc::PART_RANGES),
    ("camera target", &sa_misc::CAMERA_TARGET),
    ("colour node init", &sa_misc::COLOUR_NODE_INIT),
    ("register 16", &sa_misc::REGISTER_16),
    ("facing int", &sa_misc::FACING_INT),
    ("model id", &sa_misc::MODEL_ID),
    ("actor kind check", &sa_misc::ACTOR_KIND_CHECK),
    ("scale rotate", &sa_misc::SCALE_ROTATE),
    ("sound start", &sa_misc::SOUND_START),
    ("actor scale", &sa_misc::ACTOR_SCALE),
    ("actor mode", &sa_script::ACTOR_MODE),
    ("blur setup", &sa_misc::BLUR_SETUP),
    ("model position", &sa_misc::MODEL_POSITION),
    ("actor slot 256", &sa_script::ACTOR_SLOT256),
    ("records init (0x100)", &sa_misc::RECORDS_INIT_100),
    ("ring reserve", &sa_misc::RING_RESERVE),
    ("part matrix", &sa_misc::PART_MATRIX),
    ("fx reset d74c", &sa_misc::FX_RESET_D74C),
    ("static child", &sa_misc::STATIC_CHILD),
    ("reels match", &sa_misc::REELS_MATCH),
    ("camera target (b)", &sa_misc::CAMERA_TARGET_B),
    ("register grid", &sa_misc::REGISTER_GRID),
    ("place me", &sa_misc::PLACE_ME),
    ("kind records", &sa_misc::KIND_RECORDS),
    ("jitter", &sa_misc::JITTER),
    ("block 331", &sa_misc::BLOCK_331),
    ("slot 306", &sa_misc::SLOT306),
    ("record 296", &sa_misc::RECORD_296),
    ("records clear", &sa_misc::RECORDS_CLEAR),
    ("record free", &sa_misc::RECORD_FREE),
    ("spread set", &sa_misc::SPREAD_SET),
    ("repeat 309", &sa_misc::REPEAT_309),
    ("register one", &sa_misc::REGISTER_ONE),
    ("object to node", &sa_misc::OBJECT_TO_NODE),
    ("node start", &sa_misc::NODE_START),
    ("register 8", &sa_misc::REGISTER_8),
    ("register records", &sa_misc::REGISTER_RECORDS),
    ("register counted", &sa_misc::REGISTER_COUNTED),
    ("place records", &sa_misc::PLACE_RECORDS),
    ("records 295", &sa_misc::RECORDS_295),
    ("ring take (0x68)", &sa_misc::RING_TAKE_68),
    ("turn matrix", &sa_misc::TURN_MATRIX),
    ("static node", &sa_misc::STATIC_NODE),
    // shared-B list
    ("set speed", &sb_common::SET_SPEED),
    ("float of slot 289", &sb_nodes::FLOAT_289),
    ("node visit pair, slot 229", &sb_nodes::VISIT_PAIR_229),
    ("node visit add vector", &sb_nodes::VISIT_ADD_VECTOR),
    ("node visit offset (int)", &sb_nodes::VISIT_OFFSET_INT),
    ("node visit offset (float)", &sb_nodes::VISIT_OFFSET_FLOAT),
    ("scale ratio, slot 801", &sb_nodes::SCALE_RATIO_801),
    ("particle node", &sb_nodes::PARTICLE_NODE),
    ("effect node (slot 719)", &sb_nodes::EFFECT_NODE),
    ("node axis sprites", &sb_nodes::AXIS_SPRITES),
    ("node bounce", &sb_nodes::BOUNCE),
    ("node draw model", &sb_nodes::DRAW_MODEL),
    ("rotation matrix (0x52a0)", &sb_matrix::ROTATION_52A0),
    ("rotation matrix (0x19d0)", &sb_matrix::ROTATION_19D0),
    ("node run part", &sb_ppp::RUN_PART),
    ("vm show others", &sb_vm::SHOW_OTHERS),
    ("vm actor record", &sb_vm::ACTOR_RECORD),
    ("anim step", &sb_common::ANIM_STEP),
    ("mean position", &sb_misc::MEAN_POSITION),
    ("pool take (0x74)", &sb_misc::POOL_TAKE),
    ("node append", &sb_misc::NODE_APPEND),
    ("part setup", &sb_misc::PART_SETUP),
    ("rotate offset, list pop", &sb_misc::ROTATE_OFFSET),
    ("move start", &sb_misc::MOVE_START),
    ("move start 2", &sb_misc::MOVE_START_2),
    ("ease sample", &sb_misc::EASE_SAMPLE),
    ("ease angle", &sb_misc::EASE_ANGLE),
    ("anim reset", &sb_misc::ANIM_RESET),
    ("script run", &sb_misc::SCRIPT_RUN),
    ("timeline run", &sb_misc::TIMELINE_RUN),
    ("sprite init", &sb_misc::SPRITE_INIT),
    ("actor scale", &sb_misc::ACTOR_SCALE),
    ("lookup actors", &sb_misc::LOOKUP_ACTORS),
    ("effect reset", &sb_misc::EFFECT_RESET),
    ("draw frame", &sb_misc::DRAW_FRAME),
    ("pool take (0x140)", &sb_misc::POOL_TAKE_140),
    ("sprite init colour", &sb_misc::SPRITE_INIT_COLOUR),
    ("timeline stop", &sb_misc::TIMELINE_STOP),
    ("timeline stop (globals)", &sb_misc::TIMELINE_STOP_GLOBAL),
    (
        "timeline stop (globals, CRT tail)",
        &sb_misc::TIMELINE_STOP_GLOBAL_CRT,
    ),
    ("static nodes init", &sb_misc::STATIC_NODES_INIT),
    ("draw all", &sb_misc::DRAW_ALL),
    ("deal", &sb_misc::DEAL),
    ("draw grid", &sb_misc::DRAW_GRID),
    ("anim reset 2", &sb_misc::ANIM_RESET_2),
    ("call 220", &sb_misc::CALL_220),
    ("call 628, 213", &sb_misc::CALL_628_213),
    ("light ranges", &sb_misc::LIGHT_RANGES),
    ("aim parent", &sb_misc::AIM_PARENT),
    ("pick variant", &sb_misc::PICK_VARIANT),
    ("read pads", &sb_misc::READ_PADS),
    // shared-3 list
    ("rotation matrix (0x47c0)", &s3_matrix::ROTATION_47C0),
    ("rotation matrix (0x4d40)", &s3_matrix::ROTATION_4D40),
    ("rotation matrix (0x4250)", &s3_matrix::ROTATION_4250),
    ("leave and reset", &s3_nodes::LEAVE_AND_RESET),
    ("particle node with", &s3_nodes::PARTICLE_NODE_WITH),
    ("effect node with", &s3_nodes::EFFECT_NODE_WITH),
    ("motion step", &s3_motion::MOTION_STEP),
    ("model nodes", &s3_summon::MODEL_NODES),
    ("colour node", &s3_misc::COLOUR_NODE),
    ("fade task", &s3_misc::FADE_TASK),
    ("key step", &s3_misc::KEY_STEP),
    ("key step (curved)", &s3_misc::KEY_STEP_CURVED),
    ("task start", &s3_misc::TASK_START),
    (
        "o4 state reset (arg) 0182@1a50",
        &o4_state::RESET_STATE_ARG_0182_1A50,
    ),
    (
        "o4 state reset (arg) 0197@19b0",
        &o4_state::RESET_STATE_ARG_0197_19B0,
    ),
    (
        "o4 state reset (arg) 0199@2b20",
        &o4_state::RESET_STATE_ARG_0199_2B20,
    ),
    (
        "o4 state reset (arg) 0252@1ee0",
        &o4_state::RESET_STATE_ARG_0252_1EE0,
    ),
    (
        "o4 state reset (arg) 0266@3900",
        &o4_state::RESET_STATE_ARG_0266_3900,
    ),
    (
        "o4 state reset (arg) 0311@2b40",
        &o4_state::RESET_STATE_ARG_0311_2B40,
    ),
    (
        "o4 state reset (arg) 0365@1560",
        &o4_state::RESET_STATE_ARG_0365_1560,
    ),
    (
        "o4 state reset (arg) 0447@3200",
        &o4_state::RESET_STATE_ARG_0447_3200,
    ),
    (
        "o4 state reset (arg) 0475@26c0",
        &o4_state::RESET_STATE_ARG_0475_26C0,
    ),
    (
        "o4 state reset (bare) 0245@23f0",
        &o4_state::RESET_STATE_BARE_0245_23F0,
    ),
    (
        "o4 state reset (bare) 0298@2ce0",
        &o4_state::RESET_STATE_BARE_0298_2CE0,
    ),
    (
        "o4 state reset (bare) 0308@1ec0",
        &o4_state::RESET_STATE_BARE_0308_1EC0,
    ),
    (
        "o4 state reset (bare) 0435@2790",
        &o4_state::RESET_STATE_BARE_0435_2790,
    ),
    (
        "o4 state reset (bare) 0671@2b50",
        &o4_state::RESET_STATE_BARE_0671_2B50,
    ),
    (
        "o4 state reset (edx) 0108@1af0",
        &o4_state::RESET_STATE_EDX_0108_1AF0,
    ),
    (
        "o4 state reset (edx) 0221@1af0",
        &o4_state::RESET_STATE_EDX_0221_1AF0,
    ),
    (
        "o4 state reset (edx) 0419@37c0",
        &o4_state::RESET_STATE_EDX_0419_37C0,
    ),
    (
        "o4 fam2 register_loop 0171@2500",
        &o4_fam2::REGISTER_LOOP_0171_2500,
    ),
    (
        "o4 fam2 register_loop 0180@1870",
        &o4_fam2::REGISTER_LOOP_0180_1870,
    ),
    (
        "o4 fam2 register_loop 0392@4830",
        &o4_fam2::REGISTER_LOOP_0392_4830,
    ),
    (
        "o4 fam2 flag_slots 0202@6d90",
        &o4_fam2::FLAG_SLOTS_0202_6D90,
    ),
    (
        "o4 fam2 flag_slots 0317@6a40",
        &o4_fam2::FLAG_SLOTS_0317_6A40,
    ),
    (
        "o4 fam2 flag_slots 0597@8e80",
        &o4_fam2::FLAG_SLOTS_0597_8E80,
    ),
    (
        "o4 fam3 record_copy 0480@2880",
        &o4_fam3::RECORD_COPY_0480_2880,
    ),
    (
        "o4 fam3 record_copy 0480@2910",
        &o4_fam3::RECORD_COPY_0480_2910,
    ),
    (
        "o4 fam4 mesh_flags 0391@3e40",
        &o4_fam4::MESH_FLAGS_0391_3E40,
    ),
    (
        "o4 fam4 mesh_flags 0392@4070",
        &o4_fam4::MESH_FLAGS_0392_4070,
    ),
];

/// What covers the function at `addr`: a port's name (an entry
/// [`bind`] finds, or an [`INTERNAL`] function).
#[must_use]
pub fn covered(img: &Image, addr: u32) -> Option<&'static str> {
    bind(img, addr).map(|i| PORTS[i].0).or_else(|| {
        INTERNAL
            .iter()
            .find(|(_, sig)| img.in_text(addr) && img.matches(addr, sig))
            .map(|(n, _)| *n)
    })
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

#[cfg(test)]
mod tests {
    use super::{Callee, INTERNAL, PORTS, Sig, first_overlap, sigs_overlap};

    /// Every pair of registered signatures that could bind at one address.
    fn overlaps() -> Vec<String> {
        let all: Vec<&(&str, &Sig)> = PORTS.iter().chain(INTERNAL).collect();
        let mut out = Vec::new();
        for (i, a) in all.iter().enumerate() {
            for b in &all[i + 1..] {
                if sigs_overlap(a.1, b.1) {
                    out.push(format!(
                        "{:?} and {:?} ({} bytes, {:#018x})",
                        a.0, b.0, a.1.len, a.1.hash
                    ));
                }
            }
        }
        out
    }

    #[test]
    fn one_port_per_identity() {
        let o = overlaps();
        assert!(
            o.is_empty(),
            "{} overlapping signatures:\n{}",
            o.len(),
            o.join("\n")
        );
    }

    const A: Sig = Sig {
        len: 12,
        hash: 1,
        calls: &[],
        imports: &[],
    };
    const B: Sig = Sig {
        len: 12,
        hash: 2,
        calls: &[],
        imports: &[],
    };
    const TA: Sig = Sig {
        len: 5,
        hash: 9,
        calls: &[(1, Callee::Sig(&A))],
        imports: &[],
    };
    const TB: Sig = Sig {
        len: 5,
        hash: 9,
        calls: &[(1, Callee::Sig(&B))],
        imports: &[],
    };
    const TANY: Sig = Sig {
        len: 5,
        hash: 9,
        calls: &[(1, Callee::Any)],
        imports: &[],
    };
    const IA: Sig = Sig {
        len: 6,
        hash: 3,
        calls: &[],
        imports: &[(2, "rand")],
    };
    const IB: Sig = Sig {
        len: 6,
        hash: 3,
        calls: &[],
        imports: &[(2, "free")],
    };

    #[test]
    fn duplicates_are_refused() {
        // the same identity registered twice, across the two lists too
        let a_twice: &[(&str, &Sig)] = &[("a", &A), ("b", &B), ("a again", &A)];
        assert_eq!(first_overlap(a_twice, &[]), Some((0, 2)));
        assert_eq!(
            first_overlap(&[("a", &A)], &[("b", &B), ("a", &A)]),
            Some((0, 2))
        );
        // the same bytes with other callees: distinct functions (the thunks)
        assert_eq!(first_overlap(&[("ta", &TA), ("tb", &TB)], &[]), None);
        // a callee left unchecked accepts the other's code
        assert_eq!(
            first_overlap(&[("ta", &TA), ("tany", &TANY)], &[]),
            Some((0, 1))
        );
        // the same bytes with other imports
        assert_eq!(first_overlap(&[("ia", &IA), ("ib", &IB)], &[]), None);
        assert_eq!(first_overlap(&[("a", &A), ("b", &B)], &[("ta", &TA)]), None);
    }
}
