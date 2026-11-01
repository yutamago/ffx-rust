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
pub mod h6_setup;
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
pub mod part_draw;
pub mod pe;
pub mod pushes;
// round 9, own rows below 0450 (magic-r9-own): the load-step family
pub mod r9_load;
pub mod r9_misc;
pub mod r9_summon;
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
// the shared work list's fourth round (shared-4)
pub mod s4_fam446;
pub mod s4_misc;
pub mod s4_small;
pub mod s4_summon;
pub mod s4_wrap;
// the shared work list's fifth round (shared-5: rows 737 on)
pub mod s5_misc;
pub mod s6_misc;
// the shared work list's seventh round (shared-7)
pub mod s7_misc;
// the shared work list's eighth round (shared-8, worklist5)
pub mod s8_misc;

// census2: functions the old census hid inside merged rows
pub mod sf_hidden;
pub mod sl_host;
pub mod sl_wrap;

// round 6, the shape families across both work lists (o6_fam_*)
pub mod o6_fam_bone;
pub mod o6_fam_boot;
pub mod o6_fam_cursor;
pub mod o6_fam_launch;
pub mod o6_fam_node;
pub mod o6_fam_object;
pub mod o6_fam_pairs;
pub mod o6_fam_pairs2;
pub mod o6_fam_pairs3;
pub mod o6_fam_pairs4;
pub mod o6_fam_pairs5;
pub mod o6_fam_pairs6;
pub mod o6_fam_pool;
pub mod o6_fam_setup;
pub mod o6_fam_start;

// round 4, the own work list's families (o4_*)
pub mod o4_ctor;
pub mod o4_effect;
pub mod o4_fam2;
pub mod o4_fam3;
pub mod o4_fam4;
pub mod o4_fam5;
pub mod o4_flags;
pub mod o4_list;
pub mod o4_misc;
pub mod o4_more;
pub mod o4_more2;
pub mod o4_parts;
pub mod o4_rcef;
pub mod o4_rcef2;
pub mod o4_rcef3;
pub mod o4_rcef4;
pub mod o4_setup;
pub mod o4_single;
pub mod o4_single3;
pub mod o4_small;
pub mod o4_state;
pub mod o4_static;
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
pub mod m0053;
pub mod m0054;
pub mod m0070;
pub mod m0071;
pub mod m0072;
pub mod m0074;
pub mod m0076;
pub mod m0078;
pub mod m0082;
pub mod m0084;
pub mod m0089;
pub mod m0090;
pub mod m0093;
pub mod m0098;
pub mod m0146;
pub mod m0149;
pub mod m0150;
pub mod m0152;
pub mod m0153;
pub mod m0154;
pub mod m0159;
pub mod m0162;
pub mod m0163;
pub mod m0164;
pub mod m0165;
pub mod m0166;
pub mod m0168;
pub mod m0170;
pub mod m0171;
pub mod m0176;
pub mod m0180;
pub mod m0183;
pub mod m0184;
pub mod m0186;
pub mod m0187;
pub mod m0191;
pub mod m0193;
pub mod m0194;
pub mod m0195;
pub mod m0196;
pub mod m0198;
pub mod m0199;
pub mod m0201;
pub mod m0203;
pub mod m0205;
pub mod m0206;
pub mod m0207;
pub mod m0219;
pub mod m0243;
pub mod m0244;
pub mod m0245;
pub mod m0249;
pub mod m0250;
pub mod m0251;
pub mod m0253;
pub mod m0255;
pub mod m0260;
pub mod m0262;
pub mod m0263;
pub mod m0267;
pub mod m0269;
pub mod m0270;
pub mod m0271;
pub mod m0272;
pub mod m0273;
pub mod m0284;
pub mod m0287;
pub mod m0292;
pub mod m0295;
pub mod m0297;
pub mod m0298;
pub mod m0300;
pub mod m0301;
pub mod m0303;
pub mod m0304;
pub mod m0305;
pub mod m0307;
pub mod m0311;
pub mod m0312;
pub mod m0313;
pub mod m0316;
pub mod m0318;
pub mod m0325;
pub mod m0330;
pub mod m0341;
pub mod m0342;
pub mod m0348;
pub mod m0350;
pub mod m0351;
pub mod m0352;
pub mod m0353;
pub mod m0354;
pub mod m0356;
pub mod m0357;
pub mod m0359;
pub mod m0360;
pub mod m0361;
pub mod m0367;
pub mod m0368;
pub mod m0370;
pub mod m0371;
pub mod m0372;
pub mod m0375;
pub mod m0377;
pub mod m0378;
pub mod m0379;
pub mod m0380;
pub mod m0381;
pub mod m0382;
pub mod m0385;
pub mod m0387;
pub mod m0388;
pub mod m0389;
pub mod m0390;
pub mod m0393;
pub mod m0394;
pub mod m0395;
pub mod m0396;
pub mod m0397;
pub mod m0413;
pub mod m0415;
pub mod m0416;
pub mod m0417;
pub mod m0419;
pub mod m0421;
pub mod m0424;
pub mod m0429;
pub mod m0430;
pub mod m0431;
pub mod m0432;
pub mod m0434;
pub mod m0440;
pub mod m0443;
pub mod m0445;
pub mod m0447;
pub mod m0449;
pub mod m0450;
pub mod m0459;
pub mod m0473;
pub mod m0474;
pub mod m0475;
pub mod m0476;
pub mod m0477;
pub mod m0478;
pub mod m0479;
pub mod m0480;
pub mod m0481;
pub mod m0482;
pub mod m0483;
pub mod m0484;
pub mod m0486;
pub mod m0487;
pub mod m0488;
pub mod m0489;
pub mod m0490;
pub mod m0493;
pub mod m0496;
pub mod m0497;
pub mod m0498;
pub mod m0499;
pub mod m0505;
pub mod m0508;
pub mod m0510;
pub mod m0511;
pub mod m0512;
pub mod m0513;
pub mod m0514;
pub mod m0519;
pub mod m0522;
pub mod m0523;
pub mod m0525;
pub mod m0526;
pub mod m0527;
pub mod m0528;
pub mod m0529;
pub mod m0530;
pub mod m0531;
pub mod m0533;
pub mod m0534;
pub mod m0535;
pub mod m0536;
pub mod m0537;
pub mod m0539;
pub mod m0540;
pub mod m0561;
pub mod m0564;
pub mod m0568;
pub mod m0573;
pub mod m0577;
pub mod m0582;
pub mod m0585;
pub mod m0587;
pub mod m0588;
pub mod m0589;
pub mod m0591;
pub mod m0596;
pub mod m0597;
pub mod m0598;
pub mod m0601;
pub mod m0602;
pub mod m0604;
pub mod m0605;
pub mod m0606;
pub mod m0608;
pub mod m0609;
pub mod m0610;
pub mod m0613;
pub mod m0616;
pub mod m0617;
pub mod m0638;
pub mod m0640;
pub mod m0641;
pub mod m0645;
pub mod m0661;
pub mod m0664;
pub mod m0665;
pub mod m0666;
pub mod m0669;
pub mod m0671;
pub mod m0672;
pub mod m0673;
pub mod m0680;
pub mod m0684;
pub mod m0689;
pub mod m0700;
pub mod m0704;
pub mod m0705;
pub mod m0706;
pub mod m0707;
pub mod m0708;
pub mod m0709;

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

    /// Whether `addr` lies inside a relocated word (an operand, or a jump
    /// table's entry): never an instruction's first byte, so never a
    /// function start.
    #[must_use]
    pub fn in_reloc(&self, addr: u32) -> bool {
        let a = addr.wrapping_sub(self.base);
        let i = self.relocs.partition_point(|&r| r <= a);
        i > 0 && a - self.relocs[i - 1] < 4
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

/// Every port: its name, signature and a dispatcher index. One port per
/// code identity: a signature registered twice (here or in [`INTERNAL`])
/// fails `cargo test -p ffx-magic one_port_per_identity`. (That check ran
/// at compile time until 2026-11-01; pairwise over ~1,000 ports it made
/// 21 s of every 25 s `cargo check` and 52 s of every 73 s release build.)
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
    "m0272 vector op (calling thunk)" => m0272::CALL_VECTOR_OP, s3_wrap::ecx_call;
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
    "m0390 task step 13b0" => m0390::TASK_STEP, m0390::task_step;
    "m0390 task step 0447" => m0390::TASK_STEP_0447, m0390::task_step;
    "m0390 entry 1750" => m0390::E1750, m0390::e1750;
    "m0390 entry 1ef0" => m0390::E1EF0, m0390::e1ef0;
    // round 9 (magic-r9-own): the load-step family (r9_load)
    "r9 load step 0018" => r9_load::LOAD_0018, r9_load::load_0018;
    "r9 load step 0019" => r9_load::LOAD_0019, r9_load::load_0019;
    "r9 load step 0046" => r9_load::LOAD_0046, r9_load::load_0046;
    "r9 load step 0064" => r9_load::LOAD_0064, r9_load::load_0064;
    "r9 load step 0171" => r9_load::LOAD_0171, r9_load::load_0171;
    "r9 load step 0367" => r9_load::LOAD_0367, r9_load::load_0367;
    "r9 load step 0369" => r9_load::LOAD_0369, r9_load::load_0369;
    "r9 load step 0383" => r9_load::LOAD_0383, r9_load::load_0383;
    "r9 load step 0390" => r9_load::LOAD_0390, r9_load::load_0390;
    "r9 load step 0395" => r9_load::LOAD_0395, r9_load::load_0395;
    "r9 load step 0447" => r9_load::LOAD_0447, r9_load::load_0447;
    "r9 load step 0108" => r9_load::LOAD_0108, r9_load::load_0108;
    "r9 load step 0221" => r9_load::LOAD_0221, r9_load::load_0221;
    "r9 load step 0229" => r9_load::LOAD_0229, r9_load::load_0229;
    "r9 load step 0166" => r9_load::LOAD_0166, r9_load::load_0166;
    "r9 load step 0308" => r9_load::LOAD_0308, r9_load::load_0308;
    "r9 task step (542) 0391" => r9_misc::TASK_STEP_542_0391, r9_misc::task_step_542;
    "r9 task step (542) 0448" => r9_misc::TASK_STEP_542_0448, r9_misc::task_step_542;
    "r9 smoke start 0285" => r9_misc::SMOKE_START_0285, r9_misc::smoke_start;
    "r9 smoke start 0286" => r9_misc::SMOKE_START_0286, r9_misc::smoke_start;
    "r9 two objects 0350" => r9_misc::TWO_OBJECTS_0350, r9_misc::two_objects;
    "r9 two objects 0352" => r9_misc::TWO_OBJECTS_0352, r9_misc::two_objects;
    "r9 object at point 0165@2e40" => r9_misc::OBJECT_AT_POINT_0165_2E40, r9_misc::object_at_point;
    "r9 object at point 0165@3310" => r9_misc::OBJECT_AT_POINT_0165_3310, r9_misc::object_at_point;
    "m0447 object fade 5290" => m0447::OBJECT_FADE, m0447::object_fade;
    "m0447 camera turn 4cb0" => m0447::CAMERA_TURN, m0447::camera_turn;
    "m0447 camera curve 4f60" => m0447::CAMERA_CURVE, m0447::camera_curve;
    "r9 summon start 0077" => r9_summon::START_0077, r9_summon::start_0077;
    "r9 summon start 0078" => r9_summon::START_0078, r9_summon::start_0078;
    "r9 summon start 0079" => r9_summon::START_0079, r9_summon::start_0079;
    "r9 summon start 0080" => r9_summon::START_0080, r9_summon::start_0080;
    "r9 summon start 0081" => r9_summon::START_0081, r9_summon::start_0081;
    "r9 summon start (nodes) 0083" => r9_summon::NODES_0083, r9_summon::nodes_0083;
    "r9 summon start (nodes) 0091" => r9_summon::NODES_0091, r9_summon::nodes_0091;
    "r9 summon start (nodes) 0094" => r9_summon::NODES_0094, r9_summon::nodes_0094;
    "r9 summon start (nodes) 0095" => r9_summon::NODES_0095, r9_summon::nodes_0095;
    "r9 summon start (nodes) 0096" => r9_summon::NODES_0096, r9_summon::nodes_0096;
    "r9 summon start (nodes) 0097" => r9_summon::NODES_0097, r9_summon::nodes_0097;
    "m0481 start" => m0481::START, m0481::start;
    "m0481 end" => m0481::END, m0481::end;
    "m0481 burst frame" => m0481::BURST_FRAME, m0481::burst_frame;
    "m0638 cue start" => m0638::CUE_START_0638, m0638::cue_start;
    "m0689 cue start" => m0638::CUE_START_0689, m0638::cue_start;
    "m0689 start" => m0689::START, m0689::start;
    "m0591 cue start" => m0591::CUE_START, m0591::cue_start;
    "m0591 scale up a" => m0591::SCALE_UP_A, m0591::scale_up_a;
    "m0591 scale up b" => m0591::SCALE_UP_B, m0591::scale_up_b;
    "m0528 step" => m0528::STEP, m0528::step;
    "m0528 spark" => m0528::SPARK, m0528::spark;
    "m0523 effect start" => m0523::EFFECT_START, m0523::effect_start;
    "m0536 effect start" => m0536::EFFECT_START, m0536::effect_start;
    "m0526 aim spark" => m0526::AIM_SPARK, m0526::aim_spark;
    "m0529 spark" => m0529::SPARK, m0529::spark;
    "m0522 bone spark" => m0522::BONE_SPARK, m0522::bone_spark;
    "m0700 start" => m0700::START, m0700::start;
    "m0700 frame" => m0700::FRAME, m0700::frame;
    "m0707 effect start" => m0707::EFFECT_START, m0707::effect_start;
    "m0707 frame" => m0707::FRAME, m0707::frame;
    "m0707 draw" => m0707::DRAW, m0707::draw;
    "m0704 start" => m0704::START, m0704::start;
    "m0704 draw" => m0704::DRAW, m0704::draw;
    "m0705 start" => m0705::START, m0705::start;
    "m0684 step" => m0684::STEP, m0684::step;
    "m0493 step" => m0493::STEP, m0493::step;
    "m0493 ppp start" => m0493::PPP_START, m0493::ppp_start;
    "m0459 step" => m0459::STEP, m0459::step;
    "m0704 frame" => m0704::FRAME, m0704::frame;
    "m0577 draw detached (arg)" => m0577::DRAW_DETACHED_ARG, m0577::draw_detached_arg;
    "m0587 reload groups" => m0587::RELOAD_GROUPS, m0587::reload_groups;
    "m0473 end objects" => m0473::END_OBJECTS, m0473::end_objects;
    "m0537 draw entry" => m0537::DRAW_ENTRY, m0537::draw_entry;
    "m0605 scale 30" => m0605::SCALE_30, m0605::scale_30;
    "m0511 place at point" => m0511::PLACE_AT_POINT, m0511::place_at_point;
    "m0671 end" => m0671::END, m0671::end;
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
    // magic r5 own (0450 and up): entries
    "m0450 frame" => m0450::FRAME, m0450::frame;
    "m0450 load models" => m0450::LOAD_MODELS, m0450::load_models;
    "m0450 place camera" => m0450::PLACE_CAMERA, m0450::place_camera;
    "m0450 blend camera" => m0450::BLEND_CAMERA, m0450::blend_camera;
    "m0450 swing start" => m0450::SWING_START, m0450::swing_start;
    "m0450 model reset" => m0450::MODEL_RESET, m0450::model_reset;
    "m0450 flight start" => m0450::FLIGHT_START, m0450::flight_start;
    "m0450 flight frame" => m0450::FLIGHT_FRAME, m0450::flight_frame;
    "m0450 shrink" => m0450::SHRINK, m0450::shrink;
    "m0450 turn step" => m0450::TURN_STEP, m0450::turn_step;
    "m0475 task" => m0475::TASK, m0475::task;
    "m0476 fade step" => m0476::FADE_STEP, m0476::fade_step;
    "m0477 e1070" => m0477::E1070, m0477::e1070;
    "m0477 e1250" => m0477::E1250, m0477::e1250;
    "m0477 e1430" => m0477::E1430, m0477::e1430;
    "m0477 e1580" => m0477::E1580, m0477::e1580;
    "m0477 e1670" => m0477::E1670, m0477::e1670;
    "m0477 e17b0" => m0477::E17B0, m0477::e17b0;
    "m0477 e1850" => m0477::E1850, m0477::e1850;
    "m0477 e1980" => m0477::E1980, m0477::e1980;
    "m0477 e1d50" => m0477::E1D50, m0477::e1d50;
    "m0477 e1de0" => m0477::E1DE0, m0477::e1de0;
    "m0477 start-up" => m0477::START_UP, m0477::start_up;
    "m0478 task" => m0478::TASK, m0478::task;
    "m0482 task" => m0482::TASK, m0482::task;
    "m0483 start" => m0483::START, m0483::start;
    "m0483 end" => m0483::END, m0483::end;
    "m0484 e1000" => m0484::E1000, m0484::e1000;
    "m0484 e1100" => m0484::E1100, m0484::e1100;
    "m0484 e1360" => m0484::E1360, m0484::e1360;
    "m0484 e13f0" => m0484::E13F0, m0484::e13f0;
    "m0484 start-up" => m0484::START_UP, m0484::start_up;
    "m0484 frame" => m0484::FRAME, m0484::frame;
    "m0486 e1070" => m0486::E1070, m0486::e1070;
    "m0486 e1250" => m0486::E1250, m0486::e1250;
    "m0486 e13f0" => m0486::E13F0, m0486::e13f0;
    "m0486 start-up" => m0486::START_UP, m0486::start_up;
    "m0486 frame" => m0486::FRAME, m0486::frame;
    "m0487 start" => m0487::START, m0487::start;
    "m0487 step" => m0487::STEP, m0487::step;
    "m0487 start-up" => m0487::START_UP, m0487::start_up;
    "m0487 frame" => m0487::FRAME, m0487::frame;
    "m0488 start-up" => m0488::START_UP, m0488::start_up;
    "m0489 e1980" => m0489::E1980, m0489::e1980;
    "m0489 e1a80" => m0489::E1A80, m0489::e1a80;
    "m0489 e1c20" => m0489::E1C20, m0489::e1c20;
    "m0489 e1d00" => m0489::E1D00, m0489::e1d00;
    "m0489 start-up" => m0489::START_UP, m0489::start_up;
    "m0489 frame" => m0489::FRAME, m0489::frame;
    "m0490 start-up" => m0490::START_UP, m0490::start_up;
    "m0496 e1000" => m0496::E1000, m0496::e1000;
    "m0496 e11a0" => m0496::E11A0, m0496::e11a0;
    "m0496 e1310" => m0496::E1310, m0496::e1310;
    "m0496 e13c0" => m0496::E13C0, m0496::e13c0;
    "m0496 start-up" => m0496::START_UP, m0496::start_up;
    "m0496 frame" => m0496::FRAME, m0496::frame;
    "m0497 e1000" => m0497::E1000, m0497::e1000;
    "m0497 e10d0" => m0497::E10D0, m0497::e10d0;
    "m0497 start-up" => m0497::START_UP, m0497::start_up;
    "m0498 start-up" => m0498::START_UP, m0498::start_up;
    "m0499 start" => m0499::START, m0499::start;
    "m0505 e1180" => m0505::E1180, m0505::e1180;
    "m0505 e1380" => m0505::E1380, m0505::e1380;
    "m0505 e1210" => m0505::E1210, m0505::e1210;
    "m0505 register" => m0505::REGISTER, m0505::register;
    "m0505 frame" => m0505::FRAME, m0505::frame;
    "m0508 e1b40" => m0508::E1B40, m0508::e1b40;
    "m0510 e1000" => m0510::E1000, m0510::e1000;
    "m0510 e11b0" => m0510::E11B0, m0510::e11b0;
    "m0510 e12a0" => m0510::E12A0, m0510::e12a0;
    "m0510 e1380" => m0510::E1380, m0510::e1380;
    "m0510 e1680" => m0510::E1680, m0510::e1680;
    "m0511 fade" => m0511::FADE, m0511::fade;
    "m0511 e2c50" => m0511::E2C50, m0511::e2c50;
    "m0511 e2da0" => m0511::E2DA0, m0511::e2da0;
    "m0511 sprite start" => m0511::SPRITE_START, m0511::sprite_start;
    "m0510 e1b10" => m0510::E1B10, m0510::e1b10;
    "m0510 e1db0" => m0510::E1DB0, m0510::e1db0;
    "m0512 task" => m0512::TASK, m0512::task;
    "m0513 end" => m0513::END, m0513::end;
    "m0514 e7260" => m0514::E7260, m0514::e7260;
    "m0514 e7660" => m0514::E7660, m0514::e7660;
    "m0522 e1140" => m0522::E1140, m0522::e1140;
    "m0522 e1360" => m0522::E1360, m0522::e1360;
    "m0522 e1400" => m0522::E1400, m0522::e1400;
    "m0523 e1000" => m0523::E1000, m0523::e1000;
    "m0523 e1150" => m0523::E1150, m0523::e1150;
    "m0523 e1240" => m0523::E1240, m0523::e1240;
    "m0523 e1380" => m0523::E1380, m0523::e1380;
    "m0525 e10b0" => m0525::E10B0, m0525::e10b0;
    "m0525 e11c0" => m0525::E11C0, m0525::e11c0;
    "m0525 e12f0" => m0525::E12F0, m0525::e12f0;
    "m0525 e1650" => m0525::E1650, m0525::e1650;
    "m0525 e16d0" => m0525::E16D0, m0525::e16d0;
    "m0526 e1140" => m0526::E1140, m0526::e1140;
    "m0526 e1650" => m0526::E1650, m0526::e1650;
    "m0526 e16e0" => m0526::E16E0, m0526::e16e0;
    "m0527 draw" => m0527::DRAW, m0527::draw;
    "m0527 frame" => m0527::FRAME, m0527::frame;
    "m0528 e1000" => m0528::E1000, m0528::e1000;
    "m0528 e14f0" => m0528::E14F0, m0528::e14f0;
    "m0529 e1000" => m0529::E1000, m0529::e1000;
    "m0529 e1100" => m0529::E1100, m0529::e1100;
    "m0529 e1790" => m0529::E1790, m0529::e1790;
    "m0529 e16c0" => m0529::E16C0, m0529::e16c0;
    "m0529 e1800" => m0529::E1800, m0529::e1800;
    "m0530 start" => m0530::START, m0530::start;
    "m0531 start" => m0531::START, m0531::start;
    "m0534 e11a0" => m0534::E11A0, m0534::e11a0;
    "m0534 e12b0" => m0534::E12B0, m0534::e12b0;
    "m0534 e1390" => m0534::E1390, m0534::e1390;
    "m0536 start" => m0536::START, m0536::start;
    "m0536 frame" => m0536::FRAME, m0536::frame;
    "m0537 frame" => m0537::FRAME, m0537::frame;
    "m0539 frame obj" => m0539::FRAME_OBJ, m0539::frame_obj;
    "m0539 frame" => m0539::FRAME, m0539::frame;
    "m0540 e10a0" => m0540::E10A0, m0540::e10a0;
    "m0540 e1170" => m0540::E1170, m0540::e1170;
    "m0540 e1250" => m0540::E1250, m0540::e1250;
    "m0540 e12c0" => m0540::E12C0, m0540::e12c0;
    "m0540 frame" => m0540::FRAME, m0540::frame;
    "m0561 frame" => m0561::FRAME, m0561::frame;
    "m0573 task" => m0573::TASK, m0573::task;
    "m0577 end" => m0577::END, m0577::end;
    "m0577 task" => m0577::TASK, m0577::task;
    "m0582 start" => m0582::START, m0582::start;
    "m0585 sound" => m0585::SOUND, m0585::sound;
    "m0585 two sounds" => m0585::TWO_SOUNDS, m0585::two_sounds;
    "m0587 glow start" => m0587::GLOW_START, m0587::glow_start;
    "m0587 glow colour" => m0587::GLOW_COLOUR, m0587::glow_colour;
    "m0587 glow turn" => m0587::GLOW_TURN, m0587::glow_turn;
    "m0588 sound" => m0588::SOUND, m0588::sound;
    "m0588 part start" => m0588::PART_START, m0588::part_start;
    "m0588 colours" => m0588::COLOURS, m0588::colours;
    "m0589 flash" => m0589::FLASH, m0589::flash;
    "m0589 two parts" => m0589::TWO_PARTS, m0589::two_parts;
    "m0588 light" => m0588::LIGHT, m0588::light;
    "m0597 frame" => m0597::FRAME, m0597::frame;
    "m0598 fade out" => m0598::FADE_OUT, m0598::fade_out;
    "m0601 end" => m0601::END, m0601::end;
    "m0605 end" => m0605::END, m0605::end;
    "m0606 show targets" => m0606::SHOW_TARGETS, m0606::show_targets;
    "m0606 camera points" => m0606::CAMERA_POINTS, m0606::camera_points;
    "m0608 start" => m0608::START, m0608::start;
    "m0616 frame" => m0616::FRAME, m0616::frame;
    "m0616 start up" => m0616::START_UP, m0616::start_up;
    "m0617 frame" => m0617::FRAME, m0617::frame;
    "m0640 register" => m0640::REGISTER, m0640::register;
    "m0641 flash parts" => m0641::FLASH_PARTS, m0641::flash_parts;
    "m0641 glow parts" => m0641::GLOW_PARTS, m0641::glow_parts;
    "m0645 value task" => m0645::VALUE_TASK, m0645::value_task;
    "m0661 sound stop" => m0661::SOUND_STOP, m0661::sound_stop;
    "m0661 glow part" => m0661::GLOW_PART, m0661::glow_part;
    "m0664 ring part" => m0664::RING_PART, m0664::ring_part;
    "m0664 frame" => m0664::FRAME, m0664::frame;
    "m0664 effect frame" => m0664::EFFECT_FRAME, m0664::effect_frame;
    "m0666 spark" => m0666::SPARK, m0666::spark;
    "m0666 ring" => m0666::RING, m0666::ring;
    "m0669 frame" => m0669::FRAME, m0669::frame;
    "m0669 draw" => m0669::DRAW, m0669::draw;
    "m0706 start" => m0706::START, m0706::start;
    "m0709 start" => m0709::START, m0709::start;
    "m0673 start" => m0673::START, m0673::start;
    "m0665 frame" => m0665::FRAME, m0665::frame;
    "m0666 flare" => m0666::FLARE, m0666::flare;
    "m0669 draw restart" => m0669::DRAW_RESTART, m0669::draw_restart;
    "m0661 beam part" => m0661::BEAM_PART, m0661::beam_part;
    // end of magic r5 own (0450 and up) entries
    // magic r6 own (0450 and up): entries
    "m0499 frame" => m0499::FRAME, m0499::frame;
    "m0513 task" => m0513::TASK, m0513::task;
    "m0513 motion task" => m0513::MOTION_TASK, m0513::motion_task;
    "m0513 light task" => m0513::LIGHT_TASK, m0513::light_task;
    "m0514 motion task" => m0514::MOTION_TASK, m0514::motion_task;
    "m0514 fade task" => m0514::FADE_TASK, m0514::fade_task;
    "m0514 colour task" => m0514::COLOUR_TASK, m0514::colour_task;
    "m0530 frame" => m0530::FRAME, m0530::frame;
    "m0530 start effect" => m0530::START_EFFECT, m0530::start_effect;
    "m0531 frame" => m0531::FRAME, m0531::frame;
    "m0531 start effect" => m0531::START_EFFECT, m0531::start_effect;
    "m0534 aim" => m0534::AIM, m0534::aim;
    "m0537 setup" => m0537::SETUP, m0537::setup;
    "m0539 start effect" => m0539::START_EFFECT, m0539::start_effect;
    "m0539 setup" => m0539::SETUP, m0539::setup;
    "m0540 setup" => m0540::SETUP, m0540::setup;
    "m0535 frame step" => m0535::FRAME_STEP, m0535::frame_step;
    "m0573 motion task" => m0573::MOTION_TASK, m0573::motion_task;
    "m0577 exit task" => m0577::EXIT_TASK, m0577::exit_task;
    "m0585 drift task" => m0585::DRIFT_TASK, m0585::drift_task;
    "m0587 sway frame" => m0587::SWAY_FRAME, m0587::sway_frame;
    "m0577 spark task" => m0577::SPARK_TASK, m0577::spark_task;
    "m0589 blend" => m0589::BLEND, m0589::blend;
    "m0672 fade task" => m0672::FADE_TASK, m0672::fade_task;
    "m0672 fade start" => m0672::FADE_START, m0672::fade_start;
    "m0666 start b" => m0666::START_B, m0666::start_b;
    "m0641 frame" => m0641::FRAME, m0641::frame;
    "m0669 frame at" => m0669::FRAME_AT, m0669::frame_at;
    "m0661 start at" => m0661::START_AT, m0661::start_at;
    "m0664 frame bone" => m0664::FRAME_BONE, m0664::frame_bone;
    // end of magic r6 own (0450 and up) entries
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
    // shared-4 list
    "thunk: leave and reset" => s4_wrap::JMP_LEAVE_AND_RESET, s4_wrap::ecx_jmp;
    "thunk: element bone" => s4_wrap::JMP_ELEM_BONE, s4_wrap::ecx_jmp;
    "thunk: colour node fade" => s4_wrap::JMP_COLOUR_NODE_FADE, s4_wrap::ecx_jmp;
    "thunk: motion state" => s4_wrap::JMP_MOTION_STATE, s4_wrap::ecx_jmp;
    "thunk: step node" => s4_wrap::JMP_STEP_NODE, s4_wrap::ecx_jmp;
    "thunk: nodes reset" => s4_wrap::JMP_NODES_RESET, s4_wrap::ecx_jmp;
    "thunk: particle node" => s4_wrap::JMP_PARTICLE_NODE, s4_wrap::ecx_jmp;
    "summon start (targets)" => s4_summon::SUMMON_START, s4_summon::summon_start;
    "summon start (targets, b)" => s4_summon::SUMMON_START_B, s4_summon::summon_start_b;
    "summon frame (0042)" => s4_summon::SUMMON_FRAME_B, s4_summon::summon_frame_b;
    "sprites run" => s4_misc::SPRITES_RUN, s4_misc::sprites_run;
    "shake task" => s4_misc::SHAKE_TASK, s4_misc::shake_task;
    "follow task" => s4_misc::FOLLOW_TASK, s4_misc::follow_task;
    "sprites run (0171)" => s4_misc::SPRITES_RUN_B, s4_misc::sprites_run_b;
    "frame step (timeline)" => s4_misc::FRAME_STEP, s4_misc::frame_step;
    "timed task" => s4_misc::TIMED_TASK, s4_misc::timed_task;
    "task motion" => s4_misc::TASK_MOTION, s4_misc::task_motion;
    "register callback" => s4_misc::REGISTER_CALLBACK, s4_misc::register_callback;
    "register key" => s4_misc::REGISTER_KEY, s4_misc::register_key;
    "state listed" => s4_misc::STATE_LISTED, s4_misc::state_listed;
    "free part item" => s4_misc::FREE_PART_ITEM, s4_misc::free_part_item;
    "count done (a)" => s4_misc::COUNT_DONE_A, s4_misc::count_done_a;
    "count done (b)" => s4_misc::COUNT_DONE_B, s4_misc::count_done_b;
    "grid frame" => s4_misc::GRID_FRAME, s4_misc::grid_frame;
    "free block object" => s4_misc::FREE_BLOCK_OBJECT, s4_misc::free_block_object;
    "objects end (a)" => s4_misc::OBJECTS_END_A, s4_misc::objects_end_a;
    "objects end (b)" => s4_misc::OBJECTS_END_B, s4_misc::objects_end_b;
    "counter set-up" => s4_misc::COUNTER_SETUP, s4_misc::counter_setup;
    "part start (0700)" => s4_misc::PART_START, s4_misc::part_start;
    "effect frame" => s4_misc::EFFECT_FRAME, s4_misc::effect_frame;
    "models slot 276" => s4_misc::MODELS_276, s4_misc::models_276;
    "repeat slot 310" => s4_misc::REPEAT_310, s4_misc::repeat_310;
    "range stop" => s4_misc::RANGE_STOP, s4_misc::range_stop;
    "hook object" => s4_misc::HOOK_OBJECT, s4_misc::hook_object;
    "view matrix" => s4_misc::VIEW_MATRIX, s4_misc::view_matrix;
    "part block start" => s4_misc::PART_BLOCK_START, s4_misc::part_block_start;
    "0446 start kind" => s4_fam446::START_KIND_A, s4_fam446::start_kind_a;
    "0446 start kind (2)" => s4_fam446::START_KIND_B, s4_fam446::start_kind_b;
    "0446 start 0x1430" => s4_fam446::START_1430, s4_fam446::start_1430;
    "0446 start 0x1300" => s4_fam446::START_1300, s4_fam446::start_1300;
    "0446 start 0x1430 low" => s4_fam446::START_1430_LOW, s4_fam446::start_1430_low;
    "0446 start 0x720" => s4_fam446::START_720, s4_fam446::start_720;
    "0446 start 0x260" => s4_fam446::START_260, s4_fam446::start_260;
    "0644 start kind" => s4_fam446::START_KIND_C, s4_fam446::start_kind_c;
    "0565 start 0x1300" => s4_fam446::START_1300_B, s4_fam446::start_1300_b;
    "0644 start 0x1300" => s4_fam446::START_1300_C, s4_fam446::start_1300_c;
    "0565 start 0xf70" => s4_fam446::START_F70, s4_fam446::start_f70;
    "0565 start 0x1300 low" => s4_fam446::START_1300_LOW, s4_fam446::start_1300_low;
    "0644 start 0x11d0" => s4_fam446::START_11D0, s4_fam446::start_11d0;
    "0565 start-up" => s4_fam446::START_UP_B, s4_fam446::start_up_b;
    "0644 start-up" => s4_fam446::START_UP_C, s4_fam446::start_up_c;
    "0565 load step" => s4_fam446::LOAD_STEP_B, s4_fam446::load_step_b;
    "0644 load step" => s4_fam446::LOAD_STEP_C, s4_fam446::load_step_c;
    "word clear" => s4_small::WORD_CLEAR, s4_small::word_clear;
    "call slot 574" => s4_small::CALL_574, s4_small::call_574;
    "store 1.0, return 1" => s4_small::STORE_ONE_F, s4_small::store_one;
    "store 0, return 1" => s4_small::STORE_ONE_0, s4_small::store_one;
    "done after slot 432" => s4_small::DONE_432, s4_small::done_432;
    "done after slot 434" => s4_small::DONE_434, s4_small::done_434;
    "done after slot 213" => s4_small::DONE_213, s4_small::done_213;
    "call slot 941" => s4_small::CALL_941, s4_small::call_941;
    "call slot 579" => s4_small::CALL_579, s4_small::call_579;
    "call slot 579 (callback)" => s4_small::CALL_579_WITH, s4_small::call_579_with;
    "call slot 239 (back)" => s4_small::CALL_239_BACK, s4_small::call_239_back;
    "call slot 239 (object)" => s4_small::CALL_239_OBJ, s4_small::call_239_obj;
    "call slot 239 when idle" => s4_small::CALL_239_IDLE, s4_small::call_239_idle;
    "call slot 450" => s4_small::CALL_450, s4_small::call_450;
    "give back +0xeac" => s4_small::GIVE_BACK_EAC, s4_small::give_back_eac;
    "call slot 231" => s4_small::CALL_231, s4_small::call_231;
    "state 0x3016" => s4_small::STATE_3016, s4_small::state_3016;
    "call slot 632" => s4_small::CALL_632, s4_small::call_632;
    "set handler" => s4_small::SET_HANDLER, s4_small::set_handler;
    "first free record" => s4_small::FIRST_FREE, s4_small::first_free;
    "two models" => s4_small::TWO_MODELS, s4_small::two_models;
    "part slot 276" => s4_small::PART_276, s4_small::part_276;
    "release +0x1ee4" => s4_small::RELEASE_1EE4, s4_small::release_1ee4;
    "finish and give back" => s4_small::FINISH_GIVE_BACK, s4_small::finish_give_back;
    "slot 371 twice" => s4_small::CALL_371_TWICE, s4_small::call_371_twice;
    "call slot 211, flag" => s4_small::CALL_211_FLAG, s4_small::call_211_flag;
    "reset, then slot 575" => s4_small::RESET_THEN_575, s4_small::reset_then_575;
    "callback and 15" => s4_small::CALLBACK_AND_15, s4_small::callback_and_count;
    "callback and 30" => s4_small::CALLBACK_AND_30, s4_small::callback_and_count;
    "callback and 10" => s4_small::CALLBACK_AND_10, s4_small::callback_and_10;
    "ready and callback" => s4_small::READY_CALLBACK, s4_small::ready_callback;
    "callback slot 461" => s4_small::CALLBACK_461, s4_small::callback_461;
    "take +0x1ee4" => s4_small::TAKE_1EE4, s4_small::take_1ee4;
    "run, then slot 214" => s4_small::RUN_THEN_214, s4_small::run_then_214;
    "script step done" => s4_small::SCRIPT_STEP_DONE, s4_small::script_step_done;
    "motion step done" => s4_small::MOTION_STEP_DONE, s4_small::motion_step_done;
    "part slot 257" => s4_small::PART_257, s4_small::part_257;
    "free slot 713" => s4_small::FREE_713, s4_small::free_713;
    "place at bone 0x99" => s4_small::PLACE_BONE_99, s4_small::place_bone_99;
    "place at own bone" => s4_small::PLACE_BONE_OWN, s4_small::place_bone_own;
    "call slot 388" => s4_small::CALL_388, s4_small::call_388;
    "check slot 411" => s4_small::CHECK_411, s4_small::check_411;
    "ranges off" => s4_small::RANGES_OFF, s4_small::ranges_off;
    "place from block" => s4_small::PLACE_FROM_BLOCK, s4_small::place_from_block;
    "count or slot 257" => s4_small::COUNT_OR_257, s4_small::count_or_257;
    "records slot 296" => s4_small::RECORDS_296_OFF, s4_small::records_296_off;
    "records slot 296 (with)" => s4_small::RECORDS_296_WITH, s4_small::records_296_with;
    "clear and count 18" => s4_small::CLEAR_COUNT_18, s4_small::clear_count_18;
    "clear and two callbacks" => s4_small::CLEAR_TWO_CALLBACKS, s4_small::clear_two_callbacks;
    "states 0x3011, 0x3013" => s4_small::STATES_3011_3013, s4_small::states_3011_3013;
    "place by slot 268" => s4_small::PLACE_268, s4_small::place_268;
    "place by slot 237 (table)" => s4_small::PLACE_237_TABLE, s4_small::place_237_table;
    "scale product" => s4_small::SCALE_PRODUCT, s4_small::scale_product;
    "unlist block object" => s4_small::UNLIST_BLOCK_OBJECT, s4_small::unlist_block_object;
    "three callbacks (2)" => s4_small::THREE_CALLBACKS_2, s4_small::three_callbacks;
    "three callbacks (5)" => s4_small::THREE_CALLBACKS_5, s4_small::three_callbacks;
    "fade down" => s4_small::FADE_DOWN, s4_small::fade_down;
    "motion step, register" => s4_small::MOTION_STEP_REGISTER, s4_small::motion_step_register;
    "records from table" => s4_small::RECORDS_FROM_TABLE, s4_small::records_from_table;
    "three with slot 440" => s4_small::THREE_440, s4_small::three_440;
    "two callbacks, entry" => s4_small::TWO_CALLBACKS_ENTRY, s4_small::two_callbacks_entry;
    "callback and 90" => s4_small::CALLBACK_451_90, s4_small::callback_451_90;
    "three with slot 448" => s4_small::THREE_448, s4_small::three_448;
    "fade 541 or 38" => s4_small::FADE_541_OR_38, s4_small::fade_541_or_38;
    "scales task" => s4_small::SCALES_TASK, s4_small::scales_task;
    "level down" => s4_small::LEVEL_DOWN, s4_small::level_down;
    "release slot 804" => s4_small::RELEASE_804, s4_small::release_804;
    "states 0x4029" => s4_small::STATES_4029, s4_small::states_4029;
    "drift and 6" => s4_small::DRIFT_AND_6, s4_small::drift_and_count;
    "drift and 12" => s4_small::DRIFT_AND_12, s4_small::drift_and_count;
    "run, draw, register" => s4_small::RUN_DRAW_REGISTER, s4_small::run_draw_register;
    "motion entry 0xb4" => s4_small::MOTION_ENTRY_B4, s4_small::motion_entry;
    "motion entry 0xba" => s4_small::MOTION_ENTRY_BA, s4_small::motion_entry;
    "motion entry 0xc2" => s4_small::MOTION_ENTRY_C2, s4_small::motion_entry;
    "motion entry 0xc0" => s4_small::MOTION_ENTRY_C0, s4_small::motion_entry;
    "motion entry 0xb3" => s4_small::MOTION_ENTRY_B3, s4_small::motion_entry;
    "records start" => s4_small::RECORDS_START, s4_small::records_start;
    "motion pair" => s4_small::MOTION_PAIR, s4_small::motion_pair;
    "drift by count" => s4_small::DRIFT_BY_COUNT, s4_small::drift_by_count;
    "entry by slot 434" => s4_small::ENTRY_434, s4_small::entry_434;
    "ranges, four" => s4_small::RANGES_FOUR, s4_small::ranges_four;
    "chain to slot 461" => s4_small::CHAIN_461, s4_small::chain_461;
    "speeds and count" => s4_small::SPEEDS_COUNT, s4_small::speeds_count;
    "register all" => s4_small::REGISTER_ALL, s4_small::register_all;
    "object done" => s4_small::OBJECT_DONE, s4_small::object_done;
    "run once, motion" => s4_small::RUN_ONCE_MOTION, s4_small::run_once_motion;
    "states 0x301a..0x301c" => s4_small::STATES_301A, s4_small::states_301a;
    "block task" => s4_small::BLOCK_TASK, s4_small::block_task;
    "counted task 24" => s4_small::COUNTED_TASK_24, s4_small::counted_task;
    "counted task 16" => s4_small::COUNTED_TASK_16, s4_small::counted_task;
    "counted task 20" => s4_small::COUNTED_TASK_20, s4_small::counted_task;
    "static node link (b)" => s4_small::STATIC_NODE_LINK_B, s4_small::static_node_link_b;
    "sparkle reset" => s4_small::SPARKLE_RESET, s4_small::sparkle_reset;
    "handle 0x3016" => s4_small::HANDLE_3016, s4_small::handle_3016;
    "static leave, slot 846" => s4_small::STATIC_LEAVE_846, s4_small::static_leave_846;
    "model ranges" => s4_small::MODEL_RANGES, s4_small::model_ranges;
    "colour one" => s4_small::COLOUR_ONE, s4_small::colour_one;
    "static node link (c)" => s4_small::STATIC_NODE_LINK_C, s4_small::static_node_link_c;
    "records clear 500" => s4_small::RECORDS_CLEAR_500, s4_small::records_clear;
    "records clear 1000" => s4_small::RECORDS_CLEAR_1000, s4_small::records_clear;
    "colour task" => s4_small::COLOUR_TASK, s4_small::colour_task;
    "static leave, model" => s4_small::STATIC_LEAVE_MODEL, s4_small::static_leave_model;
    "static link last" => s4_small::STATIC_LINK_LAST, s4_small::static_link_last;
    "light once" => s4_small::LIGHT_ONCE, s4_small::light_once;
    "block defaults" => s4_small::BLOCK_DEFAULTS, s4_small::block_defaults;
    "actor place, slot 211" => s4_small::ACTOR_PLACE_211, s4_small::actor_place_211;
    "static link (word)" => s4_small::STATIC_LINK_WORD, s4_small::static_link_word;
    "static link last (word)" => s4_small::STATIC_LINK_LAST_WORD, s4_small::static_link_last_word;
    "place at bone 0x10" => s4_small::PLACE_BONE_10, s4_small::place_bone_10;
    "actors slot 530" => s4_small::ACTORS_530, s4_small::actors_530;
    "wait, then step" => s4_small::WAIT_THEN_STEP, s4_small::wait_then_step;
    "camera point (-50)" => s4_small::CAMERA_POINT_50, s4_small::camera_point;
    "camera point (-10)" => s4_small::CAMERA_POINT_10, s4_small::camera_point;
    "load steps" => s4_small::LOAD_STEPS, s4_small::load_steps;
    "parts buffer 0x80000" => s4_small::PARTS_BUFFER_80000, s4_small::parts_buffer;
    "parts buffer 0x50000" => s4_small::PARTS_BUFFER_50000, s4_small::parts_buffer;
    "parts buffer 0x100000" => s4_small::PARTS_BUFFER_100000, s4_small::parts_buffer;
    "light once, on" => s4_small::LIGHT_ONCE_ON, s4_small::light_once_on;
    "static link, slot 845" => s4_small::STATIC_LINK_845, s4_small::static_link_845;
    "part handles" => s4_small::PART_HANDLES, s4_small::part_handles;
    "handles 0x3011, 0x3013" => s4_small::HANDLES_3011_3013, s4_small::handles_3011_3013;
    "clock done" => s4_small::CLOCK_DONE, s4_small::clock_done;
    "camera floats" => s4_small::CAMERA_FLOATS, s4_small::camera_floats;
    "part drop (fixed)" => s4_small::PART_DROP_FIXED, s4_small::part_drop_fixed;
    "reset and scale" => s4_small::RESET_AND_SCALE, s4_small::reset_and_scale;
    "reset and scale (165)" => s4_small::RESET_AND_SCALE_165, s4_small::reset_and_scale;
    "reset and scale (511)" => s4_small::RESET_AND_SCALE_511, s4_small::reset_and_scale;
    "reset and scale (563)" => s4_small::RESET_AND_SCALE_563, s4_small::reset_and_scale;
    "reset and scale (430)" => s4_small::RESET_AND_SCALE_430, s4_small::reset_and_scale;
    "colour one, give back" => s4_small::COLOUR_ONE_GIVE_BACK, s4_small::colour_one_give_back;
    "part drop (argument)" => s4_small::PART_DROP_ARG, s4_small::part_drop_arg;
    "part frame" => s4_small::PART_FRAME, s4_small::part_frame;
    "load steps (b)" => s4_small::LOAD_STEPS_B, s4_small::load_steps_b;
    "particles start" => s4_small::PARTICLES_START, s4_small::particles_start;
    "register square" => s4_small::REGISTER_SQUARE, s4_small::register_square;
    "particles start (b)" => s4_small::PARTICLES_START_B, s4_small::particles_start_b;
    "static link (x)" => s4_small::STATIC_LINK_X, s4_small::static_link_x;
    "camera floats, on" => s4_small::CAMERA_FLOATS_ON, s4_small::camera_floats_on;
    "static link (tail)" => s4_small::STATIC_LINK_TAIL, s4_small::static_link_tail;
    "walk, ring entry" => s4_small::WALK_RING_ENTRY, s4_small::walk_ring_entry;
    "particles start (c)" => s4_small::PARTICLES_START_C, s4_small::particles_start_c;
    "static leave, model, flag" => s4_small::STATIC_LEAVE_MODEL_FLAG, s4_small::static_leave_model_flag;
    "particle fall" => s4_small::PARTICLE_FALL, s4_small::particle_fall;
    "opu new" => s4_small::OPU_NEW, s4_small::opu_new;
    "part frame (counted)" => s4_small::PART_FRAME_COUNTED, s4_small::part_frame_counted;
    "0644 mode set-up" => s4_fam446::MODE_SETUP_644, s4_fam446::mode_setup;
    "0565 mode set-up" => s4_fam446::MODE_SETUP_565, s4_fam446::mode_setup;
    "0565 mode object" => s4_fam446::MODE_OBJECT_565, s4_fam446::mode_object_565;
    "0644 mode object" => s4_fam446::MODE_OBJECT_644, s4_fam446::mode_object_644;
    "0644 start pair" => s4_fam446::START_PAIR_644, s4_fam446::start_pair_644;
    "0565 start pair" => s4_fam446::START_PAIR_565, s4_fam446::start_pair_565;
    "part drop (unlock)" => s4_small::PART_DROP_UNLOCK, s4_small::part_drop_unlock;
    "static node (arguments)" => s4_small::STATIC_NODE_ARGS, s4_small::static_node_args;
    "particles start (d)" => s4_small::PARTICLES_START_D, s4_small::particles_start_d;
    "effect frame (timeline)" => s4_small::EFFECT_FRAME_TIMELINE, s4_small::effect_frame_timeline;
    "colour state task" => s4_small::COLOUR_STATE_TASK, s4_small::colour_state_task;
    "particles start (e)" => s4_small::PARTICLES_START_E, s4_small::particles_start_e;
    "camera place" => s4_small::CAMERA_PLACE, s4_small::camera_place;
    "trail step (short)" => s4_small::TRAIL_STEP_SHORT, s4_small::trail_step_short;
    "trail step (long)" => s4_small::TRAIL_STEP_LONG, s4_small::trail_step_long;
    "actors and records" => s4_small::ACTORS_RECORDS, s4_small::actors_records;
    "motion set" => s4_small::MOTION_SET, s4_small::motion_set;
    "spin and run" => s4_small::SPIN_AND_RUN, s4_small::spin_and_run;
    "sound record" => s4_small::SOUND_RECORD, s4_small::sound_record;
    "effect frame (slot 261)" => s4_small::EFFECT_FRAME_261, s4_small::effect_frame_261;
    "colour and scale task" => s4_small::COLOUR_AND_SCALE_TASK, s4_small::colour_and_scale_task;
    "light copy" => s4_small::LIGHT_COPY, s4_small::light_copy;
    "actors and entries" => s4_small::ACTORS_ENTRIES, s4_small::actors_entries;
    "part drop (guarded)" => s4_small::PART_DROP_GUARDED, s4_small::part_drop_guarded;
    "walk, two children" => s4_small::WALK_TWO_CHILDREN, s4_small::walk_two_children;
    "actor place and turn" => s4_small::ACTOR_PLACE_TURN, s4_small::actor_place_turn;
    "particles start (f)" => s4_small::PARTICLES_START_F, s4_small::particles_start_f;
    "particles start, sound" => s4_small::PARTICLES_START_SOUND, s4_small::particles_start_sound;
    "motion set (b)" => s4_small::MOTION_SET_B, s4_small::motion_set_b;
    "camera boxes" => s4_small::CAMERA_BOXES, s4_small::camera_boxes;
    "clock mode task" => s4_small::CLOCK_MODE_TASK, s4_small::clock_mode_task;
    "white and count" => s4_small::WHITE_AND_COUNT, s4_small::white_and_count;
    "effect frame (sound)" => s4_small::EFFECT_FRAME_SOUND, s4_small::effect_frame_sound;
    "register resource parts" => s4_small::REGISTER_RESOURCE_PARTS, s4_small::register_resource_parts;
    "three actors shown" => s4_small::THREE_ACTORS_SHOWN, s4_small::three_actors_shown;
    "point turned offset" => s4_small::POINT_TURNED_OFFSET, s4_small::point_turned_offset;
    "start part with sound" => s4_small::START_PART_WITH_SOUND, s4_small::start_part_with_sound;
    "count down two objects" => s4_small::COUNT_DOWN_TWO_OBJECTS, s4_small::count_down_two_objects;
    "start part sound at" => s4_small::START_PART_SOUND_AT, s4_small::start_part_sound_at;
    "object model and callbacks" => s4_small::OBJECT_MODEL_AND_CALLBACKS, s4_small::object_model_and_callbacks;
    "effect end states" => s4_small::EFFECT_END_STATES, s4_small::effect_end_states;
    "light block defaults" => s4_small::LIGHT_BLOCK_DEFAULTS, s4_small::light_block_defaults;
    "sprite record setup" => s4_small::SPRITE_RECORD_SETUP, s4_small::sprite_record_setup;
    "release and remove two" => s4_small::RELEASE_AND_REMOVE_TWO, s4_small::release_and_remove_two;
    "actors restored" => s4_small::ACTORS_RESTORED, s4_small::actors_restored;
    "command block reset" => s4_small::COMMAND_BLOCK_RESET, s4_small::command_block_reset;
    "spin records move" => s4_small::SPIN_RECORDS_MOVE, s4_small::spin_records_move;
    "camera face actor" => s4_small::CAMERA_FACE_ACTOR, s4_small::camera_face_actor;
    "entry hook in" => s4_small::ENTRY_HOOK_IN, s4_small::entry_hook_in;
    "load steps and frame" => s4_small::LOAD_STEPS_AND_FRAME, s4_small::load_steps_and_frame;
    "falling parts step" => s4_small::FALLING_PARTS_STEP, s4_small::falling_parts_step;
    "release resource triples" => s4_small::RELEASE_RESOURCE_TRIPLES, s4_small::release_resource_triples;
    "data pointers" => s4_small::DATA_POINTERS, s4_small::data_pointers;
    "effect frame (two timelines)" => s4_small::EFFECT_FRAME_TWO_TIMELINES, s4_small::effect_frame_two_timelines;
    "step done, reset" => s4_small::STEP_DONE_RESET, s4_small::step_done_reset;
    "mean on screen" => s4_small::MEAN_ON_SCREEN, s4_small::mean_on_screen;
    "0644 start kind (b)" => s4_fam446::START_KIND_E, s4_fam446::start_kind_e;
    "thunk: walk and track" => s4_wrap::JMP_WALK_TRACK, s4_wrap::ecx_jmp;
    "thunk: walk, child place" => s4_wrap::JMP_WALK_CHILD_PLACE, s4_wrap::ecx_jmp;
    "thunk: walk and ring write" => s4_wrap::JMP_WALK_RING, s4_wrap::ecx_jmp;
    "0565 start kind" => s4_fam446::START_KIND_D, s4_fam446::start_kind_d;
    "model reset, release" => s4_small::MODEL_RESET_RELEASE, s4_small::model_reset_release;
    "0644 start 0x1300 raised" => s4_fam446::START_1300_RAISED, s4_fam446::start_1300_raised;
    "thunk: walk and reset" => s4_wrap::JMP_WALK_RESET, s4_wrap::ecx_jmp;
    "thunk: table object" => s4_wrap::JMP_TABLE_OBJECT, s4_wrap::ecx_field_jmp;
    "thunk: walk, then slot 275" => s4_wrap::JMP_WALK_275, s4_wrap::ecx_jmp;
    "0446 start child" => s4_fam446::START_CHILD_A, s4_fam446::start_child_a;
    "0446 start child (scaled)" => s4_fam446::START_CHILD_B, s4_fam446::start_child_b;
    "0446 start child (slot 641)" => s4_fam446::START_CHILD_C, s4_fam446::start_child_c;
    "0446 start-up" => s4_fam446::START_UP, s4_fam446::start_up;
    "0446 mode set-up" => s4_fam446::MODE_SETUP, s4_fam446::mode_setup;
    "0446 load step" => s4_fam446::LOAD_STEP, s4_fam446::load_step;
    "0565 record reset" => s4_fam446::RECORD_RESET, s4_fam446::record_reset;
    "0565 host start" => s4_fam446::HOST_START_565, s4_fam446::host_start_565;
    "0565 screen bars" => s4_fam446::SCREEN_BARS, s4_fam446::screen_bars;
    "0543 objects task" => s4_fam446::OBJECTS_TASK, s4_fam446::objects_task;
    "0543 show caster" => s4_fam446::SHOW_CASTER, s4_fam446::show_caster;
    "0543 counter reset" => s4_fam446::COUNTER_RESET, s4_fam446::counter_reset;
    "effect node record (a)" => s4_fam446::EFFECT_NODE_A, s4_fam446::effect_node_a;
    "effect node record (b)" => s4_fam446::EFFECT_NODE_B, s4_fam446::effect_node_b;
    "0446 mode object" => s4_fam446::MODE_OBJECT, s4_fam446::mode_object;
    "0446 start child, sound" => s4_fam446::START_CHILD_SOUND_A, s4_fam446::start_child_sound_a;
    "0446 start child (scaled), sound" => s4_fam446::START_CHILD_SOUND_B, s4_fam446::start_child_sound_b;
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
    "o4 fam5 attach_mode_wait 0313@1340" => o4_fam5::ATTACH_MODE_WAIT_0313_1340, o4_fam5::attach_mode_wait;
    "o4 fam5 attach_mode_wait 0313@1630" => o4_fam5::ATTACH_MODE_WAIT_0313_1630, o4_fam5::attach_mode_wait;
    "o4 fam5 part_shape 0665@1810" => o4_fam5::PART_SHAPE_0665_1810, o4_fam5::part_shape;
    "o4 fam5 part_shape 0666@17a0" => o4_fam5::PART_SHAPE_0666_17A0, o4_fam5::part_shape;
    "o4 fam5 clear_three 0162@47e0" => o4_fam5::CLEAR_THREE_0162_47E0, o4_fam5::clear_three;
    "o4 fam5 clear_three 0598@7e30" => o4_fam5::CLEAR_THREE_0598_7E30, o4_fam5::clear_three;
    "o4 fam5 part_scale_by_index 0664@1420" => o4_fam5::PART_SCALE_BY_INDEX_0664_1420, o4_fam5::part_scale_by_index;
    "o4 fam5 part_scale_by_index 0665@1610" => o4_fam5::PART_SCALE_BY_INDEX_0665_1610, o4_fam5::part_scale_by_index;
    "o4 fam5 attach_mode32_par_wait 0421@14d0" => o4_fam5::ATTACH_MODE32_PAR_WAIT_0421_14D0, o4_fam5::attach_mode32_par_wait;
    "o4 fam5 attach_mode32_par_wait 0535@14b0" => o4_fam5::ATTACH_MODE32_PAR_WAIT_0535_14B0, o4_fam5::attach_mode32_par_wait;
    "o4 fam5 part_free_pair 0661@18d0" => o4_fam5::PART_FREE_PAIR_0661_18D0, o4_fam5::part_free_pair;
    "o4 fam5 part_free_pair 0661@1990" => o4_fam5::PART_FREE_PAIR_0661_1990, o4_fam5::part_free_pair;
    "o4 fam5 part_free_pair 0661@1b60" => o4_fam5::PART_FREE_PAIR_0661_1B60, o4_fam5::part_free_pair;
    "o4 single record_actor_slot 0270@1c70" => o4_single::RECORD_ACTOR_SLOT_0270_1C70, o4_single::record_actor_slot;
    "o4 single tail_free 0203@45f0" => o4_single::TAIL_FREE_0203_45F0, o4_single::tail_free;
    "o4 single free_by_index 0351@1bf0" => o4_single::FREE_BY_INDEX_0351_1BF0, o4_single::free_by_index;
    "o4 single find_same_id 0275@4310" => o4_single::FIND_SAME_ID_0275_4310, o4_single::find_same_id;
    "o4 single flag_pick_slot 0601@8070" => o4_single::FLAG_PICK_SLOT_0601_8070, o4_single::flag_pick_slot;
    "o4 single model_slots 0263@6b90" => o4_single::MODEL_SLOTS_0263_6B90, o4_single::model_slots;
    "o4 single display_list 0598@77a0" => o4_single::DISPLAY_LIST_0598_77A0, o4_single::display_list;
    "o4 single colour_then_run 0671@1340" => o4_single::COLOUR_THEN_RUN_0671_1340, o4_single::colour_then_run;
    "o4 single set_then_timeline 0540@1080" => o4_single::SET_THEN_TIMELINE_0540_1080, o4_single::set_then_timeline;
    "o4 ctor effect_ctor_arg 0159@6700" => o4_ctor::EFFECT_CTOR_ARG_0159_6700, o4_ctor::effect_ctor_arg;
    "o4 ctor effect_ctor_arg2 0202@6f10" => o4_ctor::EFFECT_CTOR_ARG2_0202_6F10, o4_ctor::effect_ctor_arg2;
    "o4 ctor effect_ctor_this 0388@6d70" => o4_ctor::EFFECT_CTOR_THIS_0388_6D70, o4_ctor::effect_ctor_this;
    "o4 ctor big_ctor_this 0597@8f40" => o4_ctor::BIG_CTOR_THIS_0597_8F40, o4_ctor::big_ctor_this;
    "o4 ctor big_table_ctor 0262@6c10" => o4_ctor::BIG_TABLE_CTOR_0262_6C10, o4_ctor::big_table_ctor;
    "o4 ctor colour_ctor 0070@97b0" => o4_ctor::COLOUR_CTOR_0070_97B0, o4_ctor::colour_ctor;
    "o4 ctor vec_ctor 0071@7530" => o4_ctor::VEC_CTOR_0071_7530, o4_ctor::vec_ctor;
    "o4 single seq_wait_table 0195@12c0" => o4_single::SEQ_WAIT_TABLE_0195_12C0, o4_single::seq_wait_table;
    "o4 single copy_seq_wait 0382@1120" => o4_single::COPY_SEQ_WAIT_0382_1120, o4_single::copy_seq_wait;
    "o4 single seq_call_then_jump 0415@1000" => o4_single::SEQ_CALL_THEN_JUMP_0415_1000, o4_single::seq_call_then_jump;
    "o4 single slot_chain_306 0397@5870" => o4_single::SLOT_CHAIN_306_0397_5870, o4_single::slot_chain_306;
    "o4 single remove_and_free 0397@6390" => o4_single::REMOVE_AND_FREE_0397_6390, o4_single::remove_and_free;
    "o4 rcef2 clear_slot_seq_call 0191@1000" => o4_rcef2::CLEAR_SLOT_SEQ_CALL_0191_1000, o4_rcef2::clear_slot_seq_call;
    "o4 rcef2 indexed_slot_seq_wait 0191@10b0" => o4_rcef2::INDEXED_SLOT_SEQ_WAIT_0191_10B0, o4_rcef2::indexed_slot_seq_wait;
    "o4 rcef2 two_tasks_seq_wait8 0300@1170" => o4_rcef2::TWO_TASKS_SEQ_WAIT8_0300_1170, o4_rcef2::two_tasks_seq_wait8;
    "o4 rcef2 slots_seq_call 0300@11d0" => o4_rcef2::SLOTS_SEQ_CALL_0300_11D0, o4_rcef2::slots_seq_call;
    "o4 rcef2 pair_slot_seq_wait 0301@1740" => o4_rcef2::PAIR_SLOT_SEQ_WAIT_0301_1740, o4_rcef2::pair_slot_seq_wait;
    "o4 rcef2 field_task_seq_wait 0301@17a0" => o4_rcef2::FIELD_TASK_SEQ_WAIT_0301_17A0, o4_rcef2::field_task_seq_wait;
    "o4 rcef2 task_seq_keep 0359@1160" => o4_rcef2::TASK_SEQ_KEEP_0359_1160, o4_rcef2::task_seq_keep;
    "o4 rcef2 tasks_while_count 0359@14d0" => o4_rcef2::TASKS_WHILE_COUNT_0359_14D0, o4_rcef2::tasks_while_count;
    "o4 rcef2 keep_task_seq_wait 0361@1260" => o4_rcef2::KEEP_TASK_SEQ_WAIT_0361_1260, o4_rcef2::keep_task_seq_wait;
    "o4 rcef2 slot_seq_keep 0361@12d0" => o4_rcef2::SLOT_SEQ_KEEP_0361_12D0, o4_rcef2::slot_seq_keep;
    "o4 rcef2 pair_field_seq_wait 0526@1090" => o4_rcef2::PAIR_FIELD_SEQ_WAIT_0526_1090, o4_rcef2::pair_field_seq_wait;
    "o4 rcef2 indexed_field_seq_wait 0526@11c0" => o4_rcef2::INDEXED_FIELD_SEQ_WAIT_0526_11C0, o4_rcef2::indexed_field_seq_wait;
    "o4 rcef2 task_count_cb 0526@1270" => o4_rcef2::TASK_COUNT_CB_0526_1270, o4_rcef2::task_count_cb;
    "o4 rcef2 effect_slots_seq 0529@1650" => o4_rcef2::EFFECT_SLOTS_SEQ_0529_1650, o4_rcef2::effect_slots_seq;
    "o4 setup two_setups 0042@b060" => o4_setup::TWO_SETUPS_0042_B060, o4_setup::two_setups;
    "o4 setup setups_of_block 0262@6350" => o4_setup::SETUPS_OF_BLOCK_0262_6350, o4_setup::setups_of_block;
    "o4 setup setups_of_table 0262@6e50" => o4_setup::SETUPS_OF_TABLE_0262_6E50, o4_setup::setups_of_table;
    "o4 rcef3 field_slots_seq 0361@2770" => o4_rcef3::FIELD_SLOTS_SEQ_0361_2770, o4_rcef3::field_slots_seq;
    "o4 rcef3 tasks_or_seq 0382@1790" => o4_rcef3::TASKS_OR_SEQ_0382_1790, o4_rcef3::tasks_or_seq;
    "o4 rcef3 step_or_flag 0382@1c70" => o4_rcef3::STEP_OR_FLAG_0382_1C70, o4_rcef3::step_or_flag;
    "o4 rcef3 task_keep_seq_call 0313@19e0" => o4_rcef3::TASK_KEEP_SEQ_CALL_0313_19E0, o4_rcef3::task_keep_seq_call;
    "o4 misc static_flag_slots 0343@6900" => o4_misc::STATIC_FLAG_SLOTS_0343_6900, o4_misc::static_flag_slots;
    "o4 misc reset_then_float 0351@2450" => o4_misc::RESET_THEN_FLOAT_0351_2450, o4_misc::reset_then_float;
    "o4 rcef4 effect_task_seq_wait 0360@10a0" => o4_rcef4::EFFECT_TASK_SEQ_WAIT_0360_10A0, o4_rcef4::effect_task_seq_wait;
    "o4 rcef4 effect_keep_seq_call 0525@1000" => o4_rcef4::EFFECT_KEEP_SEQ_CALL_0525_1000, o4_rcef4::effect_keep_seq_call;
    "o4 rcef4 seq_by_count 0534@1040" => o4_rcef4::SEQ_BY_COUNT_0534_1040, o4_rcef4::seq_by_count;
    "o4 rcef4 step_capped 0536@11a0" => o4_rcef4::STEP_CAPPED_0536_11A0, o4_rcef4::step_capped;
    "o4 rcef4 free_three 0185@1500" => o4_rcef4::FREE_THREE_0185_1500, o4_rcef4::free_three;
    "o4 rcef4 countdown_then_done 0163@1cb0" => o4_rcef4::COUNTDOWN_THEN_DONE_0163_1CB0, o4_rcef4::countdown_then_done;
    "o4 rcef4 remove_and_release 0353@2e00" => o4_rcef4::REMOVE_AND_RELEASE_0353_2E00, o4_rcef4::remove_and_release;
    "o4 single3 slot_seq_call_keep 0194@11c0" => o4_single3::SLOT_SEQ_CALL_KEEP_0194_11C0, o4_single3::slot_seq_call_keep;
    "o4 single3 place_from_slot 0303@14d0" => o4_single3::PLACE_FROM_SLOT_0303_14D0, o4_single3::place_from_slot;
    "o4 single3 task_node_tail 0379@6910" => o4_single3::TASK_NODE_TAIL_0379_6910, o4_single3::task_node_tail;
    "o4 single3 model_flag_step 0263@6be0" => o4_single3::MODEL_FLAG_STEP_0263_6BE0, o4_single3::model_flag_step;
    "o4 single3 drop_current 0263@6b30" => o4_single3::DROP_CURRENT_0263_6B30, o4_single3::drop_current;
    "o4 single3 object_task_head 0577@b130" => o4_single3::OBJECT_TASK_HEAD_0577_B130, o4_single3::object_task_head;
    "o4 single3 model_node_tail 0577@8d70" => o4_single3::MODEL_NODE_TAIL_0577_8D70, o4_single3::model_node_tail;
    "o4 static ten_stores 0303@1730" => o4_static::TEN_STORES_0303_1730, o4_static::ten_stores;
    "o4 static record_two_node 0303@48d0" => o4_static::RECORD_TWO_NODE_0303_48D0, o4_static::record_two_node;
    "o4 static slot_by_flag_obj 0316@16b0" => o4_static::SLOT_BY_FLAG_OBJ_0316_16B0, o4_static::slot_by_flag_obj;
    "o4 static shape_local 0380@1a20" => o4_static::SHAPE_LOCAL_0380_1A20, o4_static::shape_local;
    "o4 more slot_pair_loop 0165@5490" => o4_more::SLOT_PAIR_LOOP_0165_5490, o4_more::slot_pair_loop;
    "o4 more object_list_node 0165@2ff0" => o4_more::OBJECT_LIST_NODE_0165_2FF0, o4_more::object_list_node;
    "o4 more reset_record_handle 0297@8020" => o4_more::RESET_RECORD_HANDLE_0297_8020, o4_more::reset_record_handle;
    "o4 more arg_list_node 0270@1b80" => o4_more::ARG_LIST_NODE_0270_1B80, o4_more::arg_list_node;
    "o4 more slot_each_flag 0301@22d0" => o4_more::SLOT_EACH_FLAG_0301_22D0, o4_more::slot_each_flag;
    "o4 list timer_unlink 0513@6d50" => o4_list::TIMER_UNLINK_0513_6D50, o4_list::timer_unlink;
    "o4 list table_reset 0523@21b0" => o4_list::TABLE_RESET_0523_21B0, o4_list::table_reset;
    "o4 list this_flag_step 0273@7390" => o4_list::THIS_FLAG_STEP_0273_7390, o4_list::this_flag_step;
    "o4 more2 count_fade 0673@1c70" => o4_more2::COUNT_FADE_0673_1C70, o4_more2::count_fade;
    "o4 more2 clear_halves_slot 0519@2cf0" => o4_more2::CLEAR_HALVES_SLOT_0519_2CF0, o4_more2::clear_halves_slot;
    "o4 more2 slot_chain_jump 0446@4d30" => o4_more2::SLOT_CHAIN_JUMP_0446_4D30, o4_more2::slot_chain_jump;
    // shared-5
    "wot frame" => s5_misc::WOT_FRAME, s5_misc::wot_frame;
    "facing scale" => s5_misc::FACING_SCALE, s5_misc::facing_scale;
    "motion towards" => s5_misc::MOTION_TOWARDS, s5_misc::motion_towards;
    "chara setup (0x3015)" => s5_misc::CHARA_SETUP, s5_misc::chara_setup;
    "targets start (0xa0)" => s5_misc::TARGETS_START, s5_misc::targets_start;
    "particles rethrow" => s5_misc::PARTICLES_RETHROW, s5_misc::particles_rethrow;
    "camera rect task" => s5_misc::CAMERA_RECT_TASK, s5_misc::camera_rect_task;
    "host start (chara 0x10af)" => s5_misc::HOST_START_CHARA, s5_misc::host_start_chara;
    "node set-up (0x1ac)" => s5_misc::NODE_SETUP, s5_misc::node_setup;
    "rising part" => s5_misc::RISING_PART, s5_misc::rising_part;
    "host start (sound bank)" => s5_misc::HOST_START_BANK, s5_misc::host_start_bank;
    "host start (523, bit 10)" => s5_misc::HOST_START_523_A, s5_misc::host_start_523_a;
    "host start (523, bit 11)" => s5_misc::HOST_START_523_B, s5_misc::host_start_523_b;
    "host start (two streams)" => s5_misc::HOST_START_STREAMS, s5_misc::host_start_streams;
    "host start (one stream)" => s5_misc::HOST_START_STREAM, s5_misc::host_start_stream;
    "host start (two data)" => s5_misc::HOST_START_TWO, s5_misc::host_start_two;
    "face map centre" => s5_misc::FACE_MAP_CENTRE, s5_misc::face_map_centre;
    "host start (facing the centre)" => s5_misc::HOST_START_FACING, s5_misc::host_start_facing;
    "four-part task" => s5_misc::FOUR_PART_TASK, s5_misc::four_part_task;
    "moving part" => s5_misc::MOVING_PART, s5_misc::moving_part;
    "ppp effect set-up" => s5_misc::PPP_EFFECT_SETUP, s5_misc::ppp_effect_setup;
    "ppp effect set-up (six buffers)" => s5_misc::PPP_EFFECT_SETUP_6, s5_misc::ppp_effect_setup_6;
    "packed texture" => s5_misc::PACKED_TEXTURE, s5_misc::packed_texture;
    "two lights start" => s5_misc::TWO_LIGHTS_START, s5_misc::two_lights_start;
    "targets mean offset" => s5_misc::TARGETS_MEAN_OFFSET, s5_misc::targets_mean_offset;
    "three charas 0x10af" => s5_misc::THREE_CHARAS, s5_misc::three_charas;
    "fading model task" => s5_misc::FADING_MODEL_TASK, s5_misc::fading_model_task;
    "target measure" => s5_misc::TARGET_MEASURE, s5_misc::target_measure;
    "weapon swing" => s5_misc::WEAPON_SWING, s5_misc::weapon_swing;
    "shade reset, colour task" => s5_misc::SHADE_RESET_COLOUR_TASK, s5_misc::shade_reset_colour_task;
    "effect end, character shown" => s5_misc::EFFECT_END_SHOW, s5_misc::effect_end_show;
    "screen fade step" => s5_misc::FADE_STEP, s5_misc::fade_step;
    "summon ppp start" => s5_misc::SUMMON_PPP_START, s5_misc::summon_ppp_start;
    "images store and load" => s5_misc::IMAGES_STORE_LOAD, s5_misc::images_store_load;
    "target measure (modes)" => s5_misc::TARGET_MEASURE_MODES, s5_misc::target_measure_modes;
    "root point part" => s5_misc::ROOT_POINT_PART, s5_misc::root_point_part;
    "summon ppp start (stream)" => s5_misc::SUMMON_PPP_START_STREAM, s5_misc::summon_ppp_start_stream;
    "spark pairs" => s5_misc::SPARK_PAIRS, s5_misc::spark_pairs;
    "summon ppp start (stream 1)" => s5_misc::SUMMON_PPP_START_STREAM_1, s5_misc::summon_ppp_start_stream_1;
    "summon ppp start (kind 3)" => s5_misc::SUMMON_PPP_START_K3, s5_misc::summon_ppp_start_k3;
    "summon ppp start (big env)" => s5_misc::SUMMON_PPP_START_BIG, s5_misc::summon_ppp_start_big;
    "summon ppp start (flag)" => s5_misc::SUMMON_PPP_START_FLAG, s5_misc::summon_ppp_start_flag;
    "three points ease" => s5_misc::THREE_POINTS_EASE, s5_misc::three_points_ease;
    "arc carry" => s5_misc::ARC_CARRY, s5_misc::arc_carry;
    "objects around" => s5_misc::OBJECTS_AROUND, s5_misc::objects_around;
    "task scatter" => s5_misc::TASK_SCATTER, s5_misc::task_scatter;
    "throw aim" => s5_misc::THROW_AIM, s5_misc::throw_aim;
    "weapon swing (drop)" => s5_misc::WEAPON_SWING_DROP, s5_misc::weapon_swing_drop;
    "camera part" => s5_misc::CAMERA_PART, s5_misc::camera_part;
    "homing trail" => s5_misc::HOMING_TRAIL, s5_misc::homing_trail;
    "sine rise" => s5_misc::SINE_RISE, s5_misc::sine_rise;
    "signed axes" => s5_misc::SIGNED_AXES, s5_misc::signed_axes;
    "shower step" => s5_misc::SHOWER_STEP, s5_misc::shower_step;
    "sine rise start" => s5_misc::SINE_RISE_START, s5_misc::sine_rise_start;
    "sparks thrown" => s5_misc::SPARKS_THROWN, s5_misc::sparks_thrown;
    "targets more, tasks" => s5_misc::TARGETS_MORE_TASKS, s5_misc::targets_more_tasks;
    "camera tree" => s5_misc::CAMERA_TREE, s5_misc::camera_tree;
    "models around" => s5_misc::MODELS_AROUND, s5_misc::models_around;
    "models around start" => s5_misc::MODELS_AROUND_START, s5_misc::models_around_start;
    "camera behind" => s5_misc::CAMERA_BEHIND, s5_misc::camera_behind;
    "look part" => s5_misc::LOOK_PART, s5_misc::look_part;
    "curve carry" => s5_misc::CURVE_CARRY, s5_misc::curve_carry;
    "actor clip draw" => s5_misc::ACTOR_CLIP_DRAW, s5_misc::actor_clip_draw;
    "clock light" => s5_misc::CLOCK_LIGHT, s5_misc::clock_light;
    "objects at targets" => s5_misc::OBJECTS_AT_TARGETS, s5_misc::objects_at_targets;
    "scene_light" => s5_misc::SCENE_LIGHT, s5_misc::scene_light;
    "spray_on_cue" => s5_misc::SPRAY_ON_CUE, s5_misc::spray_on_cue;
    "weapon_bounce" => s5_misc::WEAPON_BOUNCE, s5_misc::weapon_bounce;
    "scene_two_records" => s5_misc::SCENE_TWO_RECORDS, s5_misc::scene_two_records;
    "camera_behind_start" => s5_misc::CAMERA_BEHIND_START, s5_misc::camera_behind_start;
    // census2: functions the old census hid inside merged rows (sf_hidden)
    "sf elem point table" => sf_hidden::ELEM_POINT_TABLE, sf_hidden::elem_point_table;
    "sf optpos point" => sf_hidden::OPTPOS_POINT, sf_hidden::optpos_point;
    "sf elem point offset" => sf_hidden::ELEM_POINT_OFFSET, sf_hidden::elem_point_offset;
    "sf spin angles" => sf_hidden::SPIN_ANGLES, sf_hidden::spin_angles;
    "sf caster script task" => sf_hidden::CASTER_SCRIPT_TASK, sf_hidden::caster_script_task;
    "sf summon frame 673 first" => sf_hidden::SUMMON_FRAME_673_FIRST, sf_hidden::summon_frame_673_first;
    "sf fly-in task" => sf_hidden::FLY_IN_TASK, sf_hidden::fly_in_task;
    "sf motion sequence task" => sf_hidden::MOTION_SEQUENCE_TASK, sf_hidden::motion_sequence_task;
    "sf two model task" => sf_hidden::TWO_MODEL_TASK, sf_hidden::two_model_task;
    "sf centred parts task" => sf_hidden::CENTRED_PARTS_TASK, sf_hidden::centred_parts_task;
    "sf shade fade task" => sf_hidden::SHADE_FADE_TASK, sf_hidden::shade_fade_task;
    "sf spark burst task" => sf_hidden::SPARK_BURST_TASK, sf_hidden::spark_burst_task;
    // sf_hidden: end of ports
    "camera_records_start" => s5_misc::CAMERA_RECORDS_START, s5_misc::camera_records_start;
    "timed_sparks" => s5_misc::TIMED_SPARKS, s5_misc::timed_sparks;
    "spin_jump_light" => s5_misc::SPIN_JUMP_LIGHT, s5_misc::spin_jump_light;
    "bursts_task" => s5_misc::BURSTS_TASK, s5_misc::bursts_task;
    "object_three_points" => s5_misc::OBJECT_THREE_POINTS, s5_misc::object_three_points;
    "swing_pair_start" => s5_misc::SWING_PAIR_START, s5_misc::swing_pair_start;
    "camera_behind_party" => s5_misc::CAMERA_BEHIND_PARTY, s5_misc::camera_behind_party;
    // round 5, own rows
    "m0003 spawn rings" => m0003::SPAWN_RINGS, m0003::spawn_rings;
    "m0003 spawn node 0004" => m0003::SPAWN_NODE_0004, m0003::spawn_node;
    "m0003 spawn node 0005" => m0003::SPAWN_NODE_0005, m0003::spawn_node;
    "m0003 spawn facing 0006" => m0003::SPAWN_FACING, m0003::spawn_facing;
    "m0003 spawn kind table 0007" => m0003::SPAWN_KIND_TABLE, m0003::spawn_kind_table;
    "m0003 spawn kind branch 0008" => m0003::SPAWN_KIND_BRANCH, m0003::spawn_kind_branch;
    "m0009 objects frame" => m0009::OBJECTS_FRAME, m0009::objects_frame;
    "m0018 start 0018" => m0018::START_0018, m0018::start;
    "m0018 start 0019" => m0018::START_0019, m0018::start;
    "m0020 fade frame" => m0020::FADE_FRAME, m0020::fade_frame;
    "m0020 fade end" => m0020::FADE_END, m0020::fade_end;
    "m0020 close frame" => m0020::CLOSE_FRAME, m0020::close_frame;
    "m0045 close frame" => m0045::CLOSE_FRAME, m0045::close_frame;
    "m0046 start-up" => m0046::START_UP, m0046::start_up;
    "m0052 start-up" => m0052::START_UP, m0052::start_up;
    "m0053 sound step" => m0053::SOUND_GATE_STEP, m0053::sound_gate_step;
    "m0053 sound step 0065" => m0053::SOUND_PLAIN_STEP, m0053::sound_plain_step;
    "m0052 start-up 0064" => m0052::START_UP_0064, m0052::start_up_0064;
    "m0042 start-up" => m0042::START_UP, m0042::start_up;
    "m0076 model start" => m0076::MODEL_START_0076, m0076::model_start_0076;
    "m0076 model start 0077" => m0076::MODEL_START_0077, m0076::model_start_0077;
    "m0078 start" => m0078::START_0078, m0078::start_0078;
    "m0078 start 0079" => m0078::START_0079, m0078::start_0079;
    "m0078 start 0081" => m0078::START_0081, m0078::start_0081;
    "m0078 start 0080" => m0078::START_0080, m0078::start_0080;
    "m0078 start 0083" => m0078::START_0083, m0078::start_0083;
    "m0078 start 0086" => m0078::START_0086, m0078::start_0083;
    "m0078 start 0097" => m0078::START_0097, m0078::start_0083;
    "m0089 camera start 0089" => m0089::START_0089, m0089::start_0089;
    "m0089 camera start 0090" => m0089::START_0090, m0089::start_0090;
    "m0089 camera start 0091" => m0089::START_0091, m0089::start_0091;
    "m0089 camera start 0094" => m0089::START_0094, m0089::start_0094;
    "m0089 camera start 0095" => m0089::START_0095, m0089::start_0095;
    "m0089 camera start 0096" => m0089::START_0096, m0089::start_0096;
    "m0082 camera step" => m0082::CAMERA_STEP, m0082::camera_step;
    "m0082 camera parts step" => m0082::CAMERA_PARTS_STEP, m0082::camera_parts_step;
    "m0070 summon frame (376)" => m0070::SUMMON_FRAME_376, m0070::summon_frame_376;
    "m0098 frame step" => m0098::FRAME_STEP, m0098::frame_step;
    "m0098 part callback" => m0098::PART_CALLBACK, m0098::part_callback;
    "m0098 height follow" => m0098::HEIGHT_FOLLOW, m0098::height_follow;
    "m0098 part draw" => m0098::PART_DRAW, m0098::part_draw;
    "m0098 tilt" => m0098::TILT, m0098::tilt;
    "m0098 pair follow" => m0098::PAIR_FOLLOW, m0098::pair_follow;
    "m0098 fade frame" => m0098::FADE_FRAME, m0098::fade_frame;
    "m0098 part begin" => m0098::PART_BEGIN, m0098::part_begin;
    "m0146 sub-node visit" => m0146::SUB_NODE_VISIT, m0146::sub_node_visit;
    "m0149 countdown step" => m0149::COUNTDOWN_STEP, m0149::countdown_step;
    "m0150 start-up" => m0150::START_UP, m0150::start_up;
    "m0150 load frame" => m0150::LOAD_FRAME, m0150::load_frame;
    "m0150 model start" => m0150::MODEL_START, m0150::model_start;
    "m0165 look_offset" => m0165::LOOK_OFFSET, m0165::look_offset;
    "m0165 look_pair" => m0165::LOOK_PAIR, m0165::look_pair;
    "m0165 copy_point" => m0165::COPY_POINT, m0165::copy_point;
    "m0165 sound_fade" => m0165::SOUND_FADE, m0165::sound_fade;
    "m0165 sprite_task" => m0165::SPRITE_TASK, m0165::sprite_task;
    "m0165 sprite_point_task" => m0165::SPRITE_POINT_TASK, m0165::sprite_point_task;
    "m0165 look_follow" => m0165::LOOK_FOLLOW, m0165::look_follow;
    "m0165 objects_off_task" => m0165::OBJECTS_OFF_TASK, m0165::objects_off_task;
    "m0165 three_tasks" => m0165::THREE_TASKS, m0165::three_tasks;
    "m0168 camera_frame" => m0168::CAMERA_FRAME, m0168::camera_frame;
    "m0176 part_colour" => m0176::PART_COLOUR, m0176::part_colour;
    "m0184 release_four" => m0184::RELEASE_FOUR, m0184::release_four;
    "m0186 int_motion" => m0186::INT_MOTION, m0186::int_motion;
    "m0191 object_setup" => m0191::OBJECT_SETUP, m0191::object_setup;
    "m0193 spawn_three" => m0193::SPAWN_THREE, m0193::spawn_three;
    "m0187 object_setup" => m0187::OBJECT_SETUP, m0187::object_setup;
    "m0195 count_step" => m0195::COUNT_STEP, m0195::count_step;
    "m0194 object_start" => m0194::OBJECT_START, m0194::object_start;
    "m0195 flag_step" => m0195::FLAG_STEP, m0195::flag_step;
    "m0195 next_step" => m0195::NEXT_STEP, m0195::next_step;
    "m0195 object_start" => m0195::OBJECT_START, m0195::object_start;
    "m0198 load" => m0198::LOAD, m0198::load;
    "m0201 matrix_visit" => m0201::MATRIX_VISIT, m0201::matrix_visit;
    "m0191 chain_next" => m0191::CHAIN_NEXT, m0191::chain_next;
    "m0191 chain_setup" => m0191::CHAIN_SETUP, m0191::chain_setup;
    "m0203 node_setup" => m0203::NODE_SETUP, m0203::node_setup;
    "m0203 task_release" => m0203::TASK_RELEASE, m0203::task_release;
    "m0263 node_setup" => m0263::NODE_SETUP, m0263::node_setup;
    "m0351 sounds_3" => m0351::SOUNDS_3, m0351::sounds_3;
    "m0385 step_ready" => m0385::STEP_READY, m0385::step_ready;
    "m0397 script_end" => m0397::SCRIPT_END, m0397::script_end;
    // own3 ports (magic-r6-own)
    "m0164 summon frame (214)" => m0164::SUMMON_FRAME_214, m0164::summon_frame_214;
    "m0171 start-up" => m0171::START_UP, m0171::start_up;
    "m0184 part setup" => m0184::PART_SETUP, m0184::part_setup;
    "m0187 object start" => m0187::OBJECT_START, m0187::object_start;
    "m0195 copy start" => m0195::COPY_START, m0195::copy_start;
    "m0195 table start" => m0195::TABLE_START, m0195::table_start;
    "m0243 facing" => m0243::FACING, m0243::facing;
    "m0243 object facing" => m0243::OBJECT_FACING, m0243::object_facing;
    "m0245 frame" => m0245::FRAME, m0245::frame;
    "m0251 part programs" => m0251::PART_PROGRAMS, m0251::part_programs;
    "m0253 frame" => m0253::FRAME, m0253::frame;
    "m0253 part slots" => m0253::PART_SLOTS, m0253::part_slots;
    "m0255 draw placed" => m0255::DRAW_PLACED, m0255::draw_placed;
    "m0255 draw moved" => m0255::DRAW_MOVED, m0255::draw_moved;
    "m0262 static node 6d20" => m0262::STATIC_NODE_6D20, m0262::static_node_6d20;
    "m0262 static node 6dc0" => m0262::STATIC_NODE_6DC0, m0262::static_node_6dc0;
    "m0262 static node 6e90" => m0262::STATIC_NODE_6E90, m0262::static_node_6e90;
    "m0262 model start" => m0262::MODEL_START, m0262::model_start;
    "m0263 static node" => m0263::STATIC_NODE, m0263::static_node;
    "m0263 model start" => m0263::MODEL_START, m0263::model_start;
    "m0269 sprite block" => m0269::SPRITE_BLOCK, m0269::sprite_block;
    "m0270 object facing" => m0270::OBJECT_FACING, m0270::object_facing;
    "m0270 sound task" => m0270::SOUND_TASK, m0270::sound_task;
    "m0273 static node" => m0273::STATIC_NODE, m0273::static_node;
    "m0292 start-up" => m0292::START_UP, m0292::start_up;
    "m0292 draw restart" => m0292::DRAW_RESTART, m0292::draw_restart;
    "m0292 draw 0293" => m0292::DRAW_0293, m0292::draw_0293;
    "m0301 motion step" => m0301::MOTION_STEP, m0301::motion_step;
    "m0301 object start" => m0301::OBJECT_START, m0301::object_start;
    "m0301 mark step" => m0301::MARK_STEP, m0301::mark_step;
    "m0303 frame" => m0303::FRAME, m0303::frame;
    "m0303 place" => m0303::PLACE, m0303::place;
    "m0304 rise" => m0304::RISE, m0304::rise;
    "m0304 drift" => m0304::DRIFT, m0304::drift;
    "m0304 eased height" => m0304::EASED_HEIGHT, m0304::eased_height;
    "m0305 frame" => m0305::FRAME, m0305::frame;
    "m0307 end" => m0307::END, m0307::end;
    "m0307 part model" => m0307::PART_MODEL, m0307::part_model;
    "m0311 frame" => m0311::FRAME, m0311::frame;
    "m0312 start" => m0312::START, m0312::start;
    "m0312 start flagged" => m0312::START_FLAGGED, m0312::start_flagged;
    "m0312 callbacks" => m0312::CALLBACKS, m0312::callbacks;
    "m0312 start plain" => m0312::START_PLAIN, m0312::start_plain;
    "m0312 place" => m0312::PLACE, m0312::place;
    "m0312 object start" => m0312::OBJECT_START, m0312::object_start;
    "m0316 frame" => m0316::FRAME, m0316::frame;
    "m0316 sprite task" => m0316::SPRITE_TASK, m0316::sprite_task;
    "m0325 frame" => m0325::FRAME, m0325::frame;
    "m0313 object start" => m0313::OBJECT_START, m0313::object_start;
    "m0313 turn there" => m0313::TURN_THERE, m0313::turn_there;
    "m0313 turn back" => m0313::TURN_BACK, m0313::turn_back;
    "m0313 mark" => m0313::MARK, m0313::mark;
    "m0341 start-up" => m0341::START_UP, m0341::start_up;
    "m0341 free parts" => m0341::FREE_PARTS, m0341::free_parts;
    "m0342 summon frame" => m0342::SUMMON_FRAME, m0342::summon_frame;
    "m0348 tasks" => m0348::TASKS, m0348::tasks;
    "m0351 model reset" => m0351::MODEL_RESET, m0351::model_reset;
    "m0351 part start a" => m0351::PART_START_A, m0351::part_start_a;
    "m0351 part start b" => m0351::PART_START_B, m0351::part_start_b;
    "m0351 part start c" => m0351::PART_START_C, m0351::part_start_c;
    "m0351 camera fade" => m0351::CAMERA_FADE, m0351::camera_fade;
    "m0352 frame" => m0352::FRAME, m0352::frame;
    "m0353 motion reset" => m0353::MOTION_RESET, m0353::motion_reset;
    "m0353 fade step" => m0353::FADE_STEP, m0353::fade_step;
    "m0357 object start" => m0357::OBJECT_START, m0357::object_start;
    "m0372 part start" => m0372::PART_START, m0372::part_start;
    "m0356 copies step" => m0356::COPIES_STEP, m0356::copies_step;
    "m0357 copies step" => m0357::COPIES_STEP, m0357::copies_step;
    "m0357 mark repeat" => m0357::MARK_REPEAT, m0357::mark_repeat;
    "m0359 step a" => m0359::STEP_A, m0359::step_a;
    "m0359 step b" => m0359::STEP_B, m0359::step_b;
    "m0359 step c" => m0359::STEP_C, m0359::step_c;
    "m0359 step d" => m0359::STEP_D, m0359::step_d;
    "m0360 step a" => m0360::STEP_A, m0360::step_a;
    "m0360 step b" => m0360::STEP_B, m0360::step_b;
    "m0360 place" => m0360::PLACE, m0360::place;
    "m0361 count step" => m0361::COUNT_STEP, m0361::count_step;
    "m0361 next step" => m0361::NEXT_STEP, m0361::next_step;
    "m0367 start-up" => m0367::START_UP, m0367::start_up;
    "m0367 start-up 0369" => m0367::START_UP_0369, m0367::start_up_0369;
    "m0368 draw" => m0368::DRAW, m0368::draw;
    "m0370 start" => m0370::START, m0370::start;
    "m0372 frame" => m0372::FRAME, m0372::frame;
    "m0378 object start" => m0378::OBJECT_START, m0378::object_start;
    "m0380 task" => m0380::TASK, m0380::task;
    "m0381 poses" => m0381::POSES, m0381::poses;
    "m0381 start-up 0383" => m0381::START_UP_0383, m0381::start_up_0383;
    "m0382 mark start" => m0382::MARK_START, m0382::mark_start;
    "m0382 big start" => m0382::BIG_START, m0382::big_start;
    "m0388 motion wait" => m0388::MOTION_WAIT, m0388::motion_wait;
    "m0375 part start" => m0375::PART_START, m0375::part_start;
    "m0389 mode switch" => m0389::MODE_SWITCH, m0389::mode_switch;
    "m0397 actor position" => m0397::ACTOR_POSITION, m0397::actor_position;
    "m0397 party check" => m0397::PARTY_CHECK, m0397::party_check;
    "m0416 copies step" => m0416::COPIES_STEP, m0416::copies_step;
    "m0413 frame" => m0413::FRAME, m0413::frame;
    "m0415 turns step" => m0415::TURNS_STEP, m0415::turns_step;
    "m0419 frame" => m0419::FRAME, m0419::frame;
    "m0421 copies start" => m0421::COPIES_START, m0421::copies_start;
    "m0397 sprite tasks" => m0397::SPRITE_TASKS, m0397::sprite_tasks;
    "m0395 script end" => m0395::SCRIPT_END, m0395::script_end;
    "m0430 start" => m0430::START, m0430::start;
    "m0430 fade step" => m0430::FADE_STEP, m0430::fade_step;
    "m0430 fade start" => m0430::FADE_START, m0430::fade_start;
    "m0443 sound step" => m0443::SOUND_STEP, m0443::sound_step;
    "m0447 camera copy" => m0447::CAMERA_COPY, m0447::camera_copy;
    "m0449 frame" => m0449::FRAME, m0449::frame;
    "m0449 task step (pass)" => m0449::TASK_STEP_PASS, m0449::task_step_pass;
    "m0394 script end" => m0394::SCRIPT_END, m0394::script_end;
    "m0429 frame" => m0429::FRAME, m0429::frame;
    "m0440 frame" => m0440::FRAME, m0440::frame;
    "m0445 step" => m0445::STEP, m0445::step;
    "m0396 script end" => m0396::SCRIPT_END, m0396::script_end;
    "m0397 camera turn" => m0397::CAMERA_TURN, m0397::camera_turn;
    "m0397 point step" => m0397::POINT_STEP, m0397::point_step;
    "m0316 step" => m0316::STEP, m0316::step;
    "m0352 camera point" => m0352::CAMERA_POINT, m0352::camera_point;
    "m0375 part draw" => m0375::PART_DRAW, m0375::part_draw;
    "m0352 camera point show" => m0352::CAMERA_POINT_SHOW, m0352::camera_point_show;
    "m0351 frame" => m0351::FRAME, m0351::frame;
    "m0370 draw" => m0370::DRAW, m0370::draw;
    "m0267 frame" => m0267::FRAME, m0267::frame;
    "m0361 start" => m0361::START, m0361::start;
    "m0370 frame" => m0370::FRAME, m0370::frame;
    "m0361 object start" => m0361::OBJECT_START, m0361::object_start;
    "m0370 part start" => m0370::PART_START, m0370::part_start;
    "m0375 part restart" => m0375::PART_RESTART, m0375::part_restart;
    "m0361 sound start" => m0361::SOUND_START, m0361::sound_start;
    "staged nodes task" => s6_misc::STAGED_NODES_TASK, s6_misc::staged_nodes_task;
    "staged nodes task (b)" => s6_misc::STAGED_NODES_TASK_B, s6_misc::staged_nodes_task;
    "staged nodes task (c)" => s6_misc::STAGED_NODES_TASK_C, s6_misc::staged_nodes_task;
    "staged nodes task (d)" => s6_misc::STAGED_NODES_TASK_D, s6_misc::staged_nodes_task;
    "part centres" => s6_misc::PART_CENTRES, s6_misc::part_centres;
    "spin stage two" => s6_misc::SPIN_STAGE_TWO, s6_misc::spin_stage_two;
    "three points around" => s6_misc::THREE_POINTS_AROUND, s6_misc::three_points_around;
    "sprite fan start" => s6_misc::SPRITE_FAN_START, s6_misc::sprite_fan_start;
    "sprite fan task" => s6_misc::SPRITE_FAN_TASK, s6_misc::sprite_fan_task;
    "objects freed" => s6_misc::OBJECTS_FREED, s6_misc::objects_freed;
    "countdown tint" => s6_misc::COUNTDOWN_TINT, s6_misc::countdown_tint;
    "actor timed task" => s6_misc::ACTOR_TIMED_TASK, s6_misc::actor_timed_task;
    "thunk: spring walk" => s6_misc::JMP_SPRING_WALK, s6_misc::ecx_jmp;
    "thunk: aimed node walk" => s6_misc::JMP_AIMED_NODE_WALK, s6_misc::ecx_jmp;
    "selection groups" => s6_misc::SELECTION_GROUPS, s6_misc::selection_groups;
    "motion script step" => s6_misc::MOTION_SCRIPT_STEP, s6_misc::motion_script_step;
    "host start (one object)" => s6_misc::HOST_START_OBJECT, s6_misc::host_start_object;
    "summon script task" => s6_misc::SUMMON_SCRIPT_TASK, s6_misc::summon_script_task;
    "static part start" => s6_misc::STATIC_PART_START, s6_misc::static_part_start;
    "bone walk tail" => s6_misc::BONE_WALK_TAIL, s6_misc::bone_walk_tail;
    "particle drift" => s6_misc::PARTICLE_DRIFT, s6_misc::particle_drift;
    "homing part task" => s6_misc::HOMING_PART_TASK, s6_misc::homing_part_task;
    "camera tracks" => s6_misc::CAMERA_TRACKS, s6_misc::camera_tracks;
    "particle drift (5)" => s6_misc::PARTICLE_DRIFT_5, s6_misc::particle_drift_5;
    "particle hand-over" => s6_misc::PARTICLE_HAND_OVER, s6_misc::particle_hand_over;
    "camera blocks reset" => s6_misc::CAMERA_BLOCKS_RESET, s6_misc::camera_blocks_reset;
    "camera tracks (from a record)" => s6_misc::CAMERA_TRACKS_FROM, s6_misc::camera_tracks_from;
    "host start (when ready)" => s6_misc::HOST_START_WHEN_READY, s6_misc::host_start_when_ready;
    "weapon_bounce (part matrix)" => s5_misc::WEAPON_BOUNCE_P, s5_misc::weapon_bounce;
    "weapon swing (drop, part matrix)" => s5_misc::WEAPON_SWING_DROP_P, s5_misc::weapon_swing_drop;
    "weapon swing (part matrix)" => s5_misc::WEAPON_SWING_P, s5_misc::weapon_swing;
    // round 7, shared-7 (s7_misc)
    "scaled matrices" => s7_misc::SCALED_MATRICES, s7_misc::scaled_matrices;
    "countdown children" => s7_misc::COUNTDOWN_CHILDREN, s7_misc::countdown_children;
    "summon frame (bound, bone first)" => s7_misc::SUMMON_FRAME_BOUND, s7_misc::summon_frame_bound;
    "summon frame (bound, ring first)" => s7_misc::SUMMON_FRAME_RING, s7_misc::summon_frame_ring;
    "summon frame (pass first)" => s7_misc::SUMMON_FRAME_PASS, s7_misc::summon_frame_pass;
    "summon frame (slot 673 first)" => s7_misc::SUMMON_FRAME_673, s7_misc::summon_frame_673;
    "camera settings" => s7_misc::CAMERA_SETTINGS, s7_misc::camera_settings;
    "summon load frame" => s7_misc::SUMMON_LOAD_FRAME, s7_misc::summon_load_frame;
    "landing puff" => s7_misc::LANDING_PUFF, s7_misc::landing_puff;
    "summon script frame" => s7_misc::SUMMON_SCRIPT_FRAME, s7_misc::summon_script_frame;
    "summon script frame (selection groups)" => s7_misc::SUMMON_SCRIPT_FRAME_B, s7_misc::summon_script_frame;
    "thunk: moving light node" => s7_misc::JMP_MOVING_LIGHT_NODE, s7_misc::ecx_jmp;
    "thunk: count fade" => s7_misc::JMP_COUNT_FADE, s7_misc::ecx_jmp;
    "thunk: model draw node" => s7_misc::JMP_MODEL_DRAW_NODE, s7_misc::ecx_jmp;
    "thunk: light node" => s7_misc::JMP_LIGHT_NODE, s7_misc::ecx_jmp;
    "thunk: handles registered" => s7_misc::JMP_HANDLES_REGISTERED, s7_misc::ecx_jmp;
    "thunk: model points node" => s7_misc::JMP_MODEL_POINTS_NODE, s7_misc::ecx_jmp;
    "thunk: ring key frame" => s7_misc::JMP_RING_KEY_FRAME, s7_misc::ecx_jmp;
    "thunk: thunder node" => s7_misc::JMP_THUNDER_NODE, s7_misc::ecx_jmp;
    "thunk: sprite fade node" => s7_misc::JMP_SPRITE_FADE_NODE, s7_misc::ecx_jmp;
    "thunk: shake node" => s7_misc::JMP_SHAKE_NODE, s7_misc::ecx_jmp;
    "thunk: flagged object step" => s7_misc::JMP_FLAGGED_OBJECT_STEP, s7_misc::ecx_jmp;
    "camera settings (after 267)" => s7_misc::CAMERA_SETTINGS_4, s7_misc::camera_settings_4;
    "fading level" => s7_misc::FADING_LEVEL, s7_misc::fading_level;
    "near points fade" => s7_misc::NEAR_POINTS_FADE, s7_misc::near_points_fade;
    "two-stage fade" => s7_misc::TWO_STAGE_FADE, s7_misc::two_stage_fade;
    "two-stage fade (two constants)" => s7_misc::TWO_STAGE_FADE_B, s7_misc::two_stage_fade;
    "random part" => s7_misc::RANDOM_PART, s7_misc::random_part;
    "summon fade-in" => s7_misc::SUMMON_FADE_IN, s7_misc::summon_fade_in;
    "summon set-up task" => s7_misc::SUMMON_SETUP_TASK, s7_misc::summon_setup_task;
    "model items node" => s7_misc::MODEL_ITEMS_NODE, s7_misc::model_items_node;
    "model sound record" => s7_misc::SOUND_RECORD, s7_misc::sound_record;
    "falling sparks" => s7_misc::FALLING_SPARKS, s7_misc::falling_sparks;
    "camera path task" => s7_misc::CAMERA_PATH_TASK, s7_misc::camera_path_task;
    "children items node" => s7_misc::CHILDREN_ITEMS_NODE, s7_misc::children_items_node;
    "actors objects start" => s7_misc::ACTORS_OBJECTS_START, s7_misc::actors_objects_start;
    "camera blends" => s7_misc::CAMERA_BLENDS, s7_misc::camera_blends;
    "summon load objects" => s7_misc::SUMMON_LOAD_OBJECTS, s7_misc::summon_load_objects;
    "staged volley task" => s7_misc::STAGED_VOLLEY_TASK, s7_misc::staged_volley_task;
    "staged volley task (b)" => s7_misc::STAGED_VOLLEY_TASK_B, s7_misc::staged_volley_task;
    "summon objects start" => s7_misc::SUMMON_OBJECTS_START, s7_misc::summon_objects_start;
    "sprites step (globals)" => s7_misc::SPRITES_STEP_GLOBAL, s7_misc::sprites_step_global;
    // round 8, shared-8 (s8_misc)
    "camera rigs step" => s8_misc::CAMERA_RIGS_STEP, s8_misc::camera_rigs_step;
    "reel game phases" => s8_misc::REEL_GAME_PHASES, s8_misc::reel_game_phases;
    "table flight step (a)" => s8_misc::TABLE_FLIGHT_STEP_A, s8_misc::table_flight_step;
    "table flight step (b)" => s8_misc::TABLE_FLIGHT_STEP_B, s8_misc::table_flight_step;
    "summon start task (0248)" => s8_misc::SUMMON_START_TASK_0248, s8_misc::summon_start_task_0248;
    "opu boot (6 MiB)" => s8_misc::OPU_BOOT_6M, s8_misc::opu_boot_6m;
    "parts placed" => s8_misc::PARTS_PLACED, s8_misc::parts_placed;
    "weapon point" => s8_misc::WEAPON_POINT, s8_misc::weapon_point;
    "staged script" => s8_misc::STAGED_SCRIPT, s8_misc::staged_script;
    "summon frame (camera rigs)" => s8_misc::SUMMON_FRAME_RIGS, s8_misc::summon_frame_rigs;
    "summon frame (camera rigs, fade)" => s8_misc::SUMMON_FRAME_FADE, s8_misc::summon_frame_fade;
    "object, fan, two fades" => s8_misc::OBJECT_FAN_TWO_FADES, s8_misc::object_fan_two_fades;
    "object, fan, two fades (b)" => s8_misc::OBJECT_FAN_TWO_FADES_B, s8_misc::object_fan_two_fades;
    "objects freed, fan, fade" => s8_misc::OBJECTS_FREED_FAN_FADE, s8_misc::objects_freed_fan_fade;
    "emitter sounds" => s8_misc::EMITTER_SOUNDS, s8_misc::emitter_sounds;
    "model effect placed" => s8_misc::MODEL_EFFECT_PLACED, s8_misc::model_effect_placed;
    "summon frame (colours)" => s8_misc::SUMMON_FRAME_COLOURS, s8_misc::summon_frame_colours;
    "summon frame (colours, b)" => s8_misc::SUMMON_FRAME_COLOURS_B, s8_misc::summon_frame_colours_b;
    "thunk: bone node" => s8_misc::CALL_BONE_NODE, s8_misc::ecx_call;
    "light swing" => s8_misc::LIGHT_SWING, s8_misc::light_swing;
    // round 8, shared-8 (s8_misc): end of ports
    // round 6, the shape families (o6_fam_*)
    "o6 fam setup object_setup 0361@16a0" => o6_fam_setup::OBJECT_SETUP_0361_16A0, o6_fam_setup::object_setup;
    "o6 fam setup object_setup 0361@17f0" => o6_fam_setup::OBJECT_SETUP_0361_17F0, o6_fam_setup::object_setup;
    "o6 fam setup object_setup 0361@18f0" => o6_fam_setup::OBJECT_SETUP_0361_18F0, o6_fam_setup::object_setup;
    "o6 fam setup object_setup 0361@19f0" => o6_fam_setup::OBJECT_SETUP_0361_19F0, o6_fam_setup::object_setup;
    "o6 fam setup object_setup 0384@14b0" => o6_fam_setup::OBJECT_SETUP_0384_14B0, o6_fam_setup::object_setup;
    "o6 fam setup object_setup 0415@12a0" => o6_fam_setup::OBJECT_SETUP_0415_12A0, o6_fam_setup::object_setup;
    "o6 fam start host_start_parts 0162@10a0" => o6_fam_start::HOST_START_PARTS_0162_10A0, o6_fam_start::host_start_parts;
    "o6 fam start host_start_parts 0165@10a0" => o6_fam_start::HOST_START_PARTS_0165_10A0, o6_fam_start::host_start_parts;
    "o6 fam start host_start_parts 0304@10a0" => o6_fam_start::HOST_START_PARTS_0304_10A0, o6_fam_start::host_start_parts;
    "o6 fam start host_start_parts 0430@10a0" => o6_fam_start::HOST_START_PARTS_0430_10A0, o6_fam_start::host_start_parts;
    "o6 fam start host_start_parts 0568@10a0" => o6_fam_start::HOST_START_PARTS_0568_10A0, o6_fam_start::host_start_parts;
    "o6 fam start host_start_parts 0586@10a0" => o6_fam_start::HOST_START_PARTS_0586_10A0, o6_fam_start::host_start_parts;
    "o6 fam start host_start_parts 0588@10a0" => o6_fam_start::HOST_START_PARTS_0588_10A0, o6_fam_start::host_start_parts;
    "o6 fam start host_start_parts 0606@6370" => o6_fam_start::HOST_START_PARTS_0606_6370, o6_fam_start::host_start_parts;
    "o6 fam boot effect_boot 0368@1140" => o6_fam_boot::EFFECT_BOOT_0368_1140, o6_fam_boot::effect_boot;
    "o6 fam boot effect_boot 0527@11c0" => o6_fam_boot::EFFECT_BOOT_0527_11C0, o6_fam_boot::effect_boot;
    "o6 fam boot effect_boot 0255@12e0" => o6_fam_boot::EFFECT_BOOT_0255_12E0, o6_fam_boot::effect_boot;
    "o6 fam boot effect_boot 0617@11b0" => o6_fam_boot::EFFECT_BOOT_0617_11B0, o6_fam_boot::effect_boot;
    "o6 fam boot effect_boot 0372@1220" => o6_fam_boot::EFFECT_BOOT_0372_1220, o6_fam_boot::effect_boot;
    "o6 fam boot effect_boot 0669@1ce0" => o6_fam_boot::EFFECT_BOOT_0669_1CE0, o6_fam_boot::effect_boot;
    "o6 fam boot effect_boot 0206@1940" => o6_fam_boot::EFFECT_BOOT_0206_1940, o6_fam_boot::effect_boot;
    "o6 fam setup object_setup 0193@1660" => o6_fam_setup::OBJECT_SETUP_0193_1660, o6_fam_setup::object_setup;
    "o6 fam setup object_setup 0382@1ae0" => o6_fam_setup::OBJECT_SETUP_0382_1AE0, o6_fam_setup::object_setup;
    "o6 fam pairs scene_start 0162@3820" => o6_fam_pairs::SCENE_START_0162_3820, o6_fam_pairs::scene_start;
    "o6 fam pairs scene_start 0586@3430" => o6_fam_pairs::SCENE_START_0586_3430, o6_fam_pairs::scene_start;
    "o6 fam pairs ring_part_start 0666@1250" => o6_fam_pairs::RING_PART_START_0666_1250, o6_fam_pairs::ring_part_start;
    "o6 fam pairs ring_part_start 0666@2080" => o6_fam_pairs::RING_PART_START_0666_2080, o6_fam_pairs::ring_part_start;
    "o6 fam pairs angle_reset 0353@1980" => o6_fam_pairs::ANGLE_RESET_0353_1980, o6_fam_pairs::angle_reset;
    "o6 fam pairs angle_reset 0672@1500" => o6_fam_pairs::ANGLE_RESET_0672_1500, o6_fam_pairs::angle_reset;
    "o6 fam pairs placed_part_start 0206@17b0" => o6_fam_pairs::PLACED_PART_START_0206_17B0, o6_fam_pairs::placed_part_start;
    "o6 fam pairs placed_part_start 0376@1170" => o6_fam_pairs::PLACED_PART_START_0376_1170, o6_fam_pairs::placed_part_start;
    "o6 fam pairs slot_part_start 0371@1520" => o6_fam_pairs::SLOT_PART_START_0371_1520, o6_fam_pairs::slot_part_start;
    "o6 fam pairs slot_part_start 0371@1620" => o6_fam_pairs::SLOT_PART_START_0371_1620, o6_fam_pairs::slot_part_start;
    "o6 fam pairs2 list_angle_reset 0348@6600" => o6_fam_pairs2::LIST_ANGLE_RESET_0348_6600, o6_fam_pairs2::list_angle_reset;
    "o6 fam pairs2 list_angle_reset 0350@5a50" => o6_fam_pairs2::LIST_ANGLE_RESET_0350_5A50, o6_fam_pairs2::list_angle_reset;
    "o6 fam pairs2 shake_visit_thunk 0201@4910" => o6_fam_pairs2::SHAKE_VISIT_THUNK_0201_4910, o6_fam_pairs2::shake_visit_thunk;
    "o6 fam pairs2 shake_visit_thunk 0387@6040" => o6_fam_pairs2::SHAKE_VISIT_THUNK_0387_6040, o6_fam_pairs2::shake_visit_thunk;
    "o6 fam pairs3 two_node_start 0638@3270" => o6_fam_pairs3::TWO_NODE_START_0638_3270, o6_fam_pairs3::two_node_start;
    "o6 fam pairs3 two_node_start 0689@32c0" => o6_fam_pairs3::TWO_NODE_START_0689_32C0, o6_fam_pairs3::two_node_start;
    "o6 fam pairs3 matrix_scale_setup 0434@7c90" => o6_fam_pairs3::MATRIX_SCALE_SETUP_0434_7C90, o6_fam_pairs3::matrix_scale_setup;
    "o6 fam pairs3 party_colours 0644@6150" => o6_fam_pairs3::PARTY_COLOURS_0644_6150, o6_fam_pairs3::party_colours;
    "o6 fam pairs3 bone_transform 0249@1d70" => o6_fam_pairs3::BONE_TRANSFORM_0249_1D70, o6_fam_pairs3::bone_transform;
    "o6 fam pairs3 models_detach_thunk 0164@80c0" => o6_fam_pairs3::MODELS_DETACH_THUNK_0164_80C0, o6_fam_pairs3::models_detach_thunk;
    "o6 fam pairs3 models_detach_thunk 0155@8040" => o6_fam_pairs3::MODELS_DETACH_THUNK_0155_8040, o6_fam_pairs3::models_detach_thunk;
    "o6 fam pairs3 walk_scale_steps_thunk 0262@4650" => o6_fam_pairs3::WALK_SCALE_STEPS_THUNK_0262_4650, o6_fam_pairs3::walk_scale_steps_thunk;
    "o6 fam pairs3 visit_pool_thunk 0042@10b0" => o6_fam_pairs3::VISIT_POOL_THUNK_0042_10B0, o6_fam_pairs3::visit_pool_thunk;
    "o6 fam pairs3 spin_visit_thunk 0071@4080" => o6_fam_pairs3::SPIN_VISIT_THUNK_0071_4080, o6_fam_pairs3::spin_visit_thunk;
    "o6 fam pairs3 random_particle_setup 0359@1580" => o6_fam_pairs3::RANDOM_PARTICLE_SETUP_0359_1580, o6_fam_pairs3::random_particle_setup;
    "o6 fam pairs3 random_particle_setup 0359@16d0" => o6_fam_pairs3::RANDOM_PARTICLE_SETUP_0359_16D0, o6_fam_pairs3::random_particle_setup;
    "o6 fam pairs3 button_timelines 0375@2370" => o6_fam_pairs3::BUTTON_TIMELINES_0375_2370, o6_fam_pairs3::button_timelines;
    "o6 fam pairs3 button_timelines 0376@2700" => o6_fam_pairs3::BUTTON_TIMELINES_0376_2700, o6_fam_pairs3::button_timelines;
    "o6 fam pairs3 near_bone_matrix 0172@1920" => o6_fam_pairs3::NEAR_BONE_MATRIX_0172_1920, o6_fam_pairs3::near_bone_matrix;
    "o6 fam pairs3 near_bone_matrix 0174@1ad0" => o6_fam_pairs3::NEAR_BONE_MATRIX_0174_1AD0, o6_fam_pairs3::near_bone_matrix;
    "o6 fam pairs4 pulse_lights 0249@1f30" => o6_fam_pairs4::PULSE_LIGHTS_0249_1F30, o6_fam_pairs4::pulse_lights;
    "o6 fam pairs4 battle_effect_end 0434@95e0" => o6_fam_pairs4::BATTLE_EFFECT_END_0434_95E0, o6_fam_pairs4::battle_effect_end;
    "o6 fam pairs4 battle_effect_end 0610@d4b0" => o6_fam_pairs4::BATTLE_EFFECT_END_0610_D4B0, o6_fam_pairs4::battle_effect_end;
    "o6 fam pairs4 camera_reset_nodes 0307@6bc0" => o6_fam_pairs4::CAMERA_RESET_NODES_0307_6BC0, o6_fam_pairs4::camera_reset_nodes;
    "o6 fam pairs4 camera_reset_nodes 0605@a100" => o6_fam_pairs4::CAMERA_RESET_NODES_0605_A100, o6_fam_pairs4::camera_reset_nodes;
    "o6 fam pairs4 spark_task 0249@1b20" => o6_fam_pairs4::SPARK_TASK_0249_1B20, o6_fam_pairs4::spark_task;
    // round 6, the shape families (o6_fam_*): end of ports
    "o6 fam pairs5 look_matrix_callback 0664@16e0" => o6_fam_pairs5::LOOK_MATRIX_CALLBACK_0664_16E0, o6_fam_pairs5::look_matrix_callback;
    "o6 fam pairs5 look_matrix_callback 0045@1310" => o6_fam_pairs5::LOOK_MATRIX_CALLBACK_0045_1310, o6_fam_pairs5::look_matrix_callback;
    "o6 fam pairs5 record_unlist 0479@3c00" => o6_fam_pairs5::RECORD_UNLIST_0479_3C00, o6_fam_pairs5::record_unlist;
    "o6 fam pairs5 record_unlist 0673@15b0" => o6_fam_pairs5::RECORD_UNLIST_0673_15B0, o6_fam_pairs5::record_unlist;
    "o6 fam pairs5 two_particle_burst 0162@48a0" => o6_fam_pairs5::TWO_PARTICLE_BURST_0162_48A0, o6_fam_pairs5::two_particle_burst;
    "o6 fam pairs5 two_particle_burst 0598@7ef0" => o6_fam_pairs5::TWO_PARTICLE_BURST_0598_7EF0, o6_fam_pairs5::two_particle_burst;
    "o6 fam pairs5 level_particles 0307@5310" => o6_fam_pairs5::LEVEL_PARTICLES_0307_5310, o6_fam_pairs5::level_particles;
    "o6 fam pairs5 level_particles 0563@35b0" => o6_fam_pairs5::LEVEL_PARTICLES_0563_35B0, o6_fam_pairs5::level_particles;
    "o6 fam pairs5 two_part_visit_thunk 0070@14a0" => o6_fam_pairs5::TWO_PART_VISIT_THUNK_0070_14A0, o6_fam_pairs5::two_part_visit_thunk;
    "o6 fam pairs5 scrolled_texture 0188@1220" => o6_fam_pairs5::SCROLLED_TEXTURE_0188_1220, o6_fam_pairs5::scrolled_texture;
    "o6 fam pairs5 scrolled_texture 0351@1080" => o6_fam_pairs5::SCROLLED_TEXTURE_0351_1080, o6_fam_pairs5::scrolled_texture;
    "o6 fam pairs5 ribbon_visit_thunk 0070@8840" => o6_fam_pairs5::RIBBON_VISIT_THUNK_0070_8840, o6_fam_pairs5::ribbon_visit_thunk;
    "o6 fam pairs5 ribbon_visit_thunk 0155@8340" => o6_fam_pairs5::RIBBON_VISIT_THUNK_0155_8340, o6_fam_pairs5::ribbon_visit_thunk;
    "o6 fam pairs5 timed_items_visit_thunk 0155@8050" => o6_fam_pairs5::TIMED_ITEMS_VISIT_THUNK_0155_8050, o6_fam_pairs5::timed_items_visit_thunk;
    "o6 fam pairs5 timed_items_visit_thunk 0164@80d0" => o6_fam_pairs5::TIMED_ITEMS_VISIT_THUNK_0164_80D0, o6_fam_pairs5::timed_items_visit_thunk;
    "o6 fam pairs5 sprite_visit_thunk 0578@33c0" => o6_fam_pairs5::SPRITE_VISIT_THUNK_0578_33C0, o6_fam_pairs5::sprite_visit_thunk;
    "o6 fam pairs5 turn_box 0307@1d20" => o6_fam_pairs5::TURN_BOX_0307_1D20, o6_fam_pairs5::turn_box;
    "o6 fam pairs5 turn_box 0605@76a0" => o6_fam_pairs5::TURN_BOX_0605_76A0, o6_fam_pairs5::turn_box;
    "o6 fam pairs5 particle_rows 0304@2ae0" => o6_fam_pairs5::PARTICLE_ROWS_0304_2AE0, o6_fam_pairs5::particle_rows;
    "o6 fam pairs5 particle_rows 0568@2510" => o6_fam_pairs5::PARTICLE_ROWS_0568_2510, o6_fam_pairs5::particle_rows;
    "o6 fam pairs5 node_rings 0150@b210" => o6_fam_pairs5::NODE_RINGS_0150_B210, o6_fam_pairs5::node_rings;
    "o6 fam pairs5 node_rings 0603@b140" => o6_fam_pairs5::NODE_RINGS_0603_B140, o6_fam_pairs5::node_rings;
    "o6 fam pairs5 two_pass_particles 0304@2740" => o6_fam_pairs5::TWO_PASS_PARTICLES_0304_2740, o6_fam_pairs5::two_pass_particles;
    "o6 fam pairs5 two_pass_particles 0568@2170" => o6_fam_pairs5::TWO_PASS_PARTICLES_0568_2170, o6_fam_pairs5::two_pass_particles;
    "o6 fam pairs5 mesh_centres 0307@2010" => o6_fam_pairs5::MESH_CENTRES_0307_2010, o6_fam_pairs5::mesh_centres;
    "o6 fam pairs5 mesh_centres 0605@7990" => o6_fam_pairs5::MESH_CENTRES_0605_7990, o6_fam_pairs5::mesh_centres;
    "o6 fam pairs5 target_objects 0644@8900" => o6_fam_pairs5::TARGET_OBJECTS_0644_8900, o6_fam_pairs5::target_objects;
    "o6 fam pairs5 target_objects 0565@48e0" => o6_fam_pairs5::TARGET_OBJECTS_0565_48E0, o6_fam_pairs5::target_objects;
    "o6 fam pairs5 moving_sprites 0250@1ee0" => o6_fam_pairs5::MOVING_SPRITES_0250_1EE0, o6_fam_pairs5::moving_sprites;
    "o6 fam pairs5 actor_light 0565@5470" => o6_fam_pairs5::ACTOR_LIGHT_0565_5470, o6_fam_pairs5::actor_light;
    "o6 fam pairs5 actor_light 0644@92b0" => o6_fam_pairs5::ACTOR_LIGHT_0644_92B0, o6_fam_pairs5::actor_light;
    "o6 fam pairs6 aimed_objects 0565@40d0" => o6_fam_pairs6::AIMED_OBJECTS_0565_40D0, o6_fam_pairs6::aimed_objects;
    "o6 fam pairs6 aimed_objects 0644@80f0" => o6_fam_pairs6::AIMED_OBJECTS_0644_80F0, o6_fam_pairs6::aimed_objects;
    "o6 fam pairs6 curve_flyers 0307@2be0" => o6_fam_pairs6::CURVE_FLYERS_0307_2BE0, o6_fam_pairs6::curve_flyers;
    "o6 fam pairs6 curve_flyers 0563@26d0" => o6_fam_pairs6::CURVE_FLYERS_0563_26D0, o6_fam_pairs6::curve_flyers;
    "o6 fam pairs6 actor_spawner 0565@5bd0" => o6_fam_pairs6::ACTOR_SPAWNER_0565_5BD0, o6_fam_pairs6::actor_spawner;
    "o6 fam pairs6 actor_spawner 0644@9a00" => o6_fam_pairs6::ACTOR_SPAWNER_0644_9A00, o6_fam_pairs6::actor_spawner;
    "o6 fam pairs6 staged_nodes 0398@6960" => o6_fam_pairs6::STAGED_NODES_0398_6960, o6_fam_pairs6::staged_nodes;
    "o6 fam pairs6 staged_nodes 0402@6960" => o6_fam_pairs6::STAGED_NODES_0402_6960, o6_fam_pairs6::staged_nodes;
    "o6 fam pairs6 trail_task 0544@2250" => o6_fam_pairs6::TRAIL_TASK_0544_2250, o6_fam_pairs6::trail_task;
    "o6 fam pairs6 trail_task 0555@2250" => o6_fam_pairs6::TRAIL_TASK_0555_2250, o6_fam_pairs6::trail_task;
    "o6 fam pairs6 trail_task 0556@2250" => o6_fam_pairs6::TRAIL_TASK_0556_2250, o6_fam_pairs6::trail_task;
    "o6 fam pairs6 basis_start 0666@1900" => o6_fam_pairs6::BASIS_START_0666_1900, o6_fam_pairs6::basis_start;
    "o6 fam pairs6 basis_start 0664@18d0" => o6_fam_pairs6::BASIS_START_0664_18D0, o6_fam_pairs6::basis_start;
    "o6 fam pairs6 basis_start 0665@1b10" => o6_fam_pairs6::BASIS_START_0665_1B10, o6_fam_pairs6::basis_start;
    "o6 fam pairs6 overlay_setup 0673@1000" => o6_fam_pairs6::OVERLAY_SETUP_0673_1000, o6_fam_pairs6::overlay_setup;
    "o6 fam pairs6 overlay_setup 0253@1000" => o6_fam_pairs6::OVERLAY_SETUP_0253_1000, o6_fam_pairs6::overlay_setup;
    // families2 (o6_fam_pairs5..): end of ports
    "thunk: m0146 sub-node visit" => sl_wrap::JMP_SUB_NODE_VISIT, sl_wrap::ecx_jmp;
    "thunk: m0186 int motion" => sl_wrap::JMP_INT_MOTION, sl_wrap::ecx_jmp;
    "thunk: m0201 matrix visit" => sl_wrap::JMP_MATRIX_VISIT, sl_wrap::ecx_jmp;
    "m0052 load step" => m0052::LOAD_STEP, m0052::load_step;
    "m0098 motion step" => m0098::MOTION_STEP, m0098::motion_step;
    "m0162 scale start" => m0162::SCALE_START, m0162::scale_start;
    "m0162 object start" => m0162::OBJECT_START, m0162::object_start;
    "m0164 mean visit" => m0164::MEAN_VISIT, m0164::mean_visit;
    "thunk: m0164 mean visit" => sl_wrap::JMP_MEAN_VISIT, sl_wrap::ecx_jmp;
    "m0165 scale start" => m0165::SCALE_START, m0165::scale_start;
    "m0165 selected start" => m0165::SELECTED_START, m0165::selected_start;
    "m0165 record start" => m0165::RECORD_START, m0165::record_start;
    "m0165 turn start" => m0165::TURN_START, m0165::turn_start;
    "m0168 object start" => m0168::OBJECT_START, m0168::object_start;
    "m0171 grid step" => m0171::GRID_STEP, m0171::grid_step;
    "m0180 square step" => m0180::SQUARE_STEP, m0180::square_step;
    "m0180 main start" => m0180::MAIN_START, m0180::main_start;
    "m0199 script step" => m0199::SCRIPT_STEP, m0199::script_step;
    "m0203 model visit" => m0203::MODEL_VISIT, m0203::model_visit;
    "thunk: m0203 model visit" => sl_wrap::JMP_MODEL_VISIT, sl_wrap::ecx_jmp;
    "m0206 records start" => m0206::RECORDS_START, m0206::records_start;
    "m0249 camera step" => m0249::CAMERA_STEP, m0249::camera_step;
    "m0250 grid step" => m0250::GRID_STEP, m0250::grid_step;
    "m0250 spark" => m0250::SPARK, m0250::spark;
    "m0253 record start" => m0253::RECORD_START, m0253::record_start;
    "m0260 programs step" => m0260::PROGRAMS_STEP, m0260::programs_step;
    "m0260 actor start" => m0260::ACTOR_START, m0260::actor_start;
    "m0271 point below" => m0271::POINT_BELOW, m0271::point_below;
    "m0284 smoke start" => m0284::SMOKE_START, m0284::smoke_start;
    "m0287 smoke main" => m0287::SMOKE_MAIN, m0287::smoke_main;
    "m0298 models step" => m0298::MODELS_STEP, m0298::models_step;
    "m0301 object setup" => m0301::OBJECT_SETUP, m0301::object_setup;
    "m0301 callbacks setup" => m0301::CALLBACKS_SETUP, m0301::callbacks_setup;
    "m0303 record nodes" => m0303::RECORD_NODES, m0303::record_nodes;
    "m0303 light start" => m0303::LIGHT_START, m0303::light_start;
    "m0304 place point" => m0304::PLACE_POINT, m0304::place_point;
    "m0304 host start" => m0304::HOST_START, m0304::host_start;
    "thunk: s7 lit colour node" => sl_wrap::JMP_LIT_COLOUR_NODE, sl_wrap::ecx_jmp;
    "thunk: m0573 refresh" => sl_wrap::JMP_REFRESH_0573, sl_wrap::ecx_jmp;
    "thunk: 0585 sprite visit" => sl_wrap::JMP_SPRITE_VISIT_0585, sl_wrap::ecx_jmp;
    "m0307 scale 255c" => m0307::SCALE_255C, m0307::scale_255c;
    "m0307 scale 2524" => m0307::SCALE_2524, m0307::scale_2524;
    "m0307 seventh start" => m0307::SEVENTH_START, m0307::seventh_start;
    "m0307 pair start" => m0307::PAIR_START, m0307::pair_start;
    "m0307 third restart" => m0307::THIRD_RESTART, m0307::third_restart;
    "m0313 object setup" => m0313::OBJECT_SETUP, m0313::object_setup;
    "m0316 scale two" => m0316::SCALE_TWO, m0316::scale_two;
    "m0318 count step" => m0318::COUNT_STEP, m0318::count_step;
    "m0341 bone midpoint" => m0341::BONE_MIDPOINT, m0341::bone_midpoint;
    "m0351 reset scale" => m0351::RESET_SCALE, m0351::reset_scale;
    "m0352 fifth start" => m0352::FIFTH_START, m0352::fifth_start;
    "m0361 flag after 213" => m0361::FLAG_AFTER_213, m0361::flag_after_213;
    "m0371 records start" => m0371::RECORDS_START, m0371::records_start;
    "m0375 records start" => m0375::RECORDS_START, m0375::records_start;
    "m0353 record words" => m0353::RECORD_WORDS, m0353::record_words;
    "m0367 fourth start" => m0367::FOURTH_START, m0367::fourth_start;
    "m0377 parts step" => m0377::PARTS_STEP, m0377::parts_step;
    "m0380 scripts step" => m0380::SCRIPTS_STEP, m0380::scripts_step;
    "m0381 scripts step" => m0381::SCRIPTS_STEP, m0381::scripts_step;
    "m0393 fifth start" => m0393::FIFTH_START, m0393::fifth_start;
    "m0432 object start" => m0432::OBJECT_START, m0432::object_start;
    "m0430 scale e3c" => m0430::SCALE_E3C, m0430::scale_e3c;
    "m0432 point below" => m0432::POINT_BELOW, m0432::point_below;
    "m0434 second start" => m0434::SECOND_START, m0434::second_start;
    "m0396 object or fade" => m0396::OBJECT_OR_FADE, m0396::object_or_fade;
    "m0393 ninth start" => m0393::NINTH_START, m0393::ninth_start;
    "m0445 fade visit" => m0445::FADE_VISIT, m0445::fade_visit;
    "thunk: m0445 fade visit" => sl_wrap::JMP_FADE_VISIT, sl_wrap::ecx_jmp;
    "m0367 pair start" => m0367::PAIR_START, m0367::pair_start;
    "m0354 countdown step" => m0354::COUNTDOWN_STEP, m0354::countdown_step;
    "m0393 ninth scaled" => m0393::NINTH_SCALED, m0393::ninth_scaled;
    "m0397 object fade" => m0397::OBJECT_FADE, m0397::object_fade;
    "m0300 object setup" => m0300::OBJECT_SETUP, m0300::object_setup;
    "m0301 aimed setup" => m0301::AIMED_SETUP, m0301::aimed_setup;
    "m0301 thrown setup" => m0301::THROWN_SETUP, m0301::thrown_setup;
    "m0300 bone setup" => m0300::BONE_SETUP, m0300::bone_setup;
    "m0300 offset setup" => m0300::OFFSET_SETUP, m0300::offset_setup;
    "m0415 model setup" => m0415::MODEL_SETUP, m0415::model_setup;
    // magic small-lo: end of ports
}

/// Functions the ports reproduce inside others (no entry of their own:
/// custom register conventions, or reached only from ported code), for
/// the work list's coverage (`magic-sig --covered`).
pub const INTERNAL: &[(&str, &Sig)] = &[
    // round 9 (magic-r9-own)
    ("m0390 facing", &m0390::FACING),
    ("m0390 facing 0447", &m0390::FACING_0447),
    ("m0390 start-up", &m0390::START_UP),
    ("m0390 start-up 0447", &m0390::START_UP_0447),
    (
        "r9 start object (state) 0447",
        &r9_misc::START_OBJECT_STATE_0447,
    ),
    (
        "r9 start object (state) 0100",
        &r9_misc::START_OBJECT_STATE_0100,
    ),
    (
        "r9 start object (reset) 0182",
        &r9_misc::START_OBJECT_RESET_0182,
    ),
    (
        "r9 start object (reset) 0197",
        &r9_misc::START_OBJECT_RESET_0197,
    ),
    (
        "r9 start object (reset) 0252",
        &r9_misc::START_OBJECT_RESET_0252,
    ),
    (
        "r9 start object (reset) 0266",
        &r9_misc::START_OBJECT_RESET_0266,
    ),
    (
        "r9 start object (reset) 0311",
        &r9_misc::START_OBJECT_RESET_0311,
    ),
    (
        "r9 start object (reset) 0365",
        &r9_misc::START_OBJECT_RESET_0365,
    ),
    (
        "r9 start object (bare reset) 0245",
        &r9_misc::START_OBJECT_BARE_0245,
    ),
    (
        "r9 start object (bare reset) 0298",
        &r9_misc::START_OBJECT_BARE_0298,
    ),
    (
        "r9 start object (bare reset) 0308",
        &r9_misc::START_OBJECT_BARE_0308,
    ),
    (
        "r9 start object (bare reset) 0435",
        &r9_misc::START_OBJECT_BARE_0435,
    ),
    (
        "r9 start object (edx reset) 0108",
        &r9_misc::START_OBJECT_EDX_0108,
    ),
    (
        "r9 start object (edx reset) 0221",
        &r9_misc::START_OBJECT_EDX_0221,
    ),
    (
        "r9 start object (edx reset) 0419",
        &r9_misc::START_OBJECT_EDX_0419,
    ),
    ("r9 objects start 0108", &r9_misc::OBJECTS_START_0108),
    ("r9 objects start 0221", &r9_misc::OBJECTS_START_0221),
    ("r9 objects start 0229", &r9_misc::OBJECTS_START_0229),
    ("r9 effect start 0166", &r9_misc::EFFECT_START_0166),
    ("r9 effect start 0308", &r9_misc::EFFECT_START_0308),
    ("r9 main object fade 0392", &r9_misc::MAIN_OBJECT_FADE_0392),
    ("r9 main object fade 0449", &r9_misc::MAIN_OBJECT_FADE_0449),
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
    // magic r5 own (0450 and up): internal functions
    ("m0450 orient", &m0450::ORIENT),
    ("m0450 show models", &m0450::SHOW_MODELS),
    ("m0473 scale start", &m0473::SCALE_START),
    ("m0474 timed start", &m0474::TIMED_START),
    ("m0478 init", &m0478::INIT),
    ("m0479 start", &m0479::START),
    ("m0480 clear", &m0480::CLEAR),
    ("m0482 insert", &m0482::INSERT),
    ("m0483 draw", &m0483::DRAW),
    ("m0488 draw", &m0488::DRAW),
    ("m0493 init", &m0493::INIT),
    ("m0505 clear", &m0505::CLEAR),
    ("m0512 insert", &m0512::INSERT),
    ("m0514 timed init", &m0514::TIMED_INIT),
    ("m0514 scaled init", &m0514::SCALED_INIT),
    ("m0519 place", &m0519::PLACE),
    ("m0519 filter", &m0519::FILTER),
    ("m0519 fade", &m0519::FADE),
    ("m0537 start", &m0537::START),
    ("m0537 draw", &m0537::DRAW),
    ("m0564 hide", &m0564::HIDE),
    ("m0573 show", &m0573::SHOW),
    ("m0573 add node", &m0573::ADD_NODE),
    ("m0573 init", &m0573::INIT),
    ("m0577 colour", &m0577::COLOUR),
    ("m0577 flash off", &m0577::FLASH_OFF),
    ("m0577 glow off", &m0577::GLOW_OFF),
    ("m0577 move start", &m0577::MOVE_START),
    ("m0577 draw detached", &m0577::DRAW_DETACHED),
    ("m0577 place", &m0577::PLACE),
    ("m0577 halve", &m0577::HALVE),
    ("m0577 centre", &m0577::CENTRE),
    ("m0585 setup", &m0585::SETUP),
    ("m0585 sound start", &m0585::SOUND_START),
    ("m0585 two sounds start", &m0585::TWO_SOUNDS_START),
    ("m0585 glow clear", &m0585::GLOW_CLEAR),
    ("m0587 camera reset", &m0587::CAMERA_RESET),
    ("m0587 load groups", &m0587::LOAD_GROUPS),
    ("m0601 static node", &m0601::STATIC_NODE),
    ("m0602 sounds free", &m0602::SOUNDS_FREE),
    ("m0602 sounds stop", &m0602::SOUNDS_STOP),
    ("m0605 move pieces", &m0605::MOVE_PIECES),
    ("m0605 actor part", &m0605::ACTOR_PART),
    ("m0609 task start", &m0609::TASK_START),
    ("m0610 check targets", &m0610::CHECK_TARGETS),
    ("m0613 task start", &m0613::TASK_START),
    ("m0613 centre", &m0613::CENTRE),
    ("m0641 clear pool", &m0641::CLEAR_POOL),
    ("m0641 take requests", &m0641::TAKE_REQUESTS),
    ("m0641 step flashes", &m0641::STEP_FLASHES),
    ("m0641 part defaults", &m0641::PART_DEFAULTS),
    ("m0645 setup", &m0645::SETUP),
    ("m0645 value start", &m0645::VALUE_START),
    ("m0645 mover", &m0645::MOVER),
    ("m0665 timeline reset", &m0665::TIMELINE_RESET),
    ("m0672 camera reset", &m0672::CAMERA_RESET),
    ("m0680 task", &m0680::TASK),
    ("m0704 fade", &m0704::FADE),
    ("m0704 load", &m0704::LOAD),
    ("m0705 fade", &m0705::FADE),
    ("m0705 tint", &m0705::TINT),
    ("m0705 glow", &m0705::GLOW),
    ("m0707 start", &m0707::START),
    ("m0708 fade", &m0708::FADE),
    ("m0708 rise", &m0708::RISE),
    ("m0708 bind parts", &m0708::BIND_PARTS),
    ("m0708 shake", &m0708::SHAKE),
    ("m0708 place", &m0708::PLACE),
    ("m0671 start", &m0671::START),
    ("m0672 copy keys", &m0672::COPY_KEYS),
    // end of magic r5 own (0450 and up) internal functions
    // magic r6 own (0450 and up): internal functions
    ("m0499 parts refresh", &m0499::PARTS_REFRESH),
    ("m0512 setup", &m0512::SETUP),
    ("m0512 setup from", &m0512::SETUP_FROM),
    ("m0512 ground", &m0512::GROUND),
    ("m0513 over node", &m0513::OVER_NODE),
    ("m0513 between", &m0513::BETWEEN),
    ("m0513 for target", &m0513::FOR_TARGET),
    ("m0513 lit node", &m0513::LIT_NODE),
    ("m0514 colour setup", &m0514::COLOUR_SETUP),
    ("m0514 glow setup", &m0514::GLOW_SETUP),
    ("m0514 burst setup", &m0514::BURST_SETUP),
    ("m0514 spark setup", &m0514::SPARK_SETUP),
    ("m0533 model setup", &m0533::MODEL_SETUP),
    ("m0568 sway", &m0568::SWAY),
    ("m0573 refresh", &m0573::REFRESH),
    ("m0573 motion setup", &m0573::MOTION_SETUP),
    ("m0577 spark setup", &m0577::SPARK_SETUP),
    ("m0577 light node", &m0577::LIGHT_NODE),
    ("m0588 actor matrix", &m0588::ACTOR_MATRIX_0588),
    ("m0600 actor matrix", &m0588::ACTOR_MATRIX_0600),
    ("m0585 drift setup", &m0585::DRIFT_SETUP),
    ("m0606 start tasks", &m0606::START_TASKS),
    // end of magic r6 own (0450 and up) internal functions
    ("m0393 face", &m0393::FACE),
    ("m0394 root", &m0394::ROOT),
    ("m0394 scaled", &m0394::SCALED),
    ("m0394 place", &m0394::PLACE),
    ("m0395 face", &ob_summon::FACE_0395),
    ("m0396 face", &ob_summon::FACE_0396),
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
    ("records init (0x80)", &sa_misc::RECORDS_INIT_80),
    ("ring reserve", &sa_misc::RING_RESERVE),
    ("part matrix", &sa_misc::PART_MATRIX),
    ("fx reset d74c", &sa_misc::FX_RESET_D74C),
    ("static child", &sa_misc::STATIC_CHILD),
    ("reels match", &sa_misc::REELS_MATCH),
    ("camera target (b)", &sa_misc::CAMERA_TARGET_B),
    ("camera target (c)", &sa_misc::CAMERA_TARGET_C), // shared-4 list
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
    // shared-4 list
    ("element bone", &s4_summon::ELEM_BONE),
    ("colour node fade", &s4_summon::COLOUR_NODE_FADE),
    ("motion state", &s4_summon::MOTION_STATE),
    ("step node", &s4_summon::STEP_NODE),
    ("particle node (0070)", &s4_summon::PARTICLE_NODE),
    ("start object at", &s4_misc::START_OBJECT_AT),
    ("start object (anim)", &s4_misc::START_OBJECT_ANIM),
    (
        "start object (anim, first part)",
        &s4_misc::START_OBJECT_ANIM_FIRST,
    ),
    ("start object (owners)", &s4_misc::START_OBJECT_OWNERS),
    ("start object (plain)", &s4_misc::START_OBJECT_PLAIN),
    ("start object (first part)", &s4_misc::START_OBJECT_FIRST),
    ("sparks (0446)", &s4_fam446::SPARKS_A),
    ("sparks (0565)", &s4_fam446::SPARKS_B),
    ("sparks (0446, c)", &s4_fam446::SPARKS_C),
    ("sparks (0446, d)", &s4_fam446::SPARKS_D),
    ("glyph quad", &s4_fam446::GLYPH_QUAD),
    ("actor point", &s4_misc::ACTOR_POINT),
    ("screen place", &s4_misc::SCREEN_PLACE),
    ("scale task", &s4_misc::SCALE_TASK),
    ("float of slot 307", &s4_misc::FLOAT_307),
    ("unlist and free", &s4_misc::UNLIST_FREE),
    ("sprite take", &s4_misc::SPRITE_TAKE),
    ("part toggle", &s4_misc::PART_TOGGLE),
    ("nodes reset", &s4_misc::NODES_RESET),
    ("call slot 220", &s4_misc::CALL_220),
    ("call slot 239", &s4_misc::CALL_239),
    ("free record", &s4_misc::FREE_RECORD),
    ("model node with", &s4_misc::MODEL_NODE_WITH),
    ("records clear", &s4_misc::RECORDS_CLEAR),
    ("float of slot 664", &s4_misc::FLOAT_664),
    ("actors slot 296", &s4_misc::ACTORS_296),
    ("ring run", &s4_misc::RING_RUN),
    ("register 0x80", &s4_misc::REGISTER_80),
    ("fade set", &s4_misc::FADE_SET),
    ("register parts (b)", &s4_misc::REGISTER_PARTS_B),
    ("models slot 295", &s4_misc::MODELS_295),
    ("static node", &s4_misc::STATIC_NODE),
    ("state task", &s4_misc::STATE_TASK),
    ("motion script", &s4_misc::MOTION_SCRIPT),
    ("trail draw", &s4_misc::TRAIL_DRAW),
    ("targets init", &s4_summon::TARGETS_INIT),
    ("targets init (b)", &s4_summon::TARGETS_INIT_B),
    ("targets init (c)", &s4_summon::TARGETS_INIT_C),
    ("target nodes", &s4_summon::TARGET_NODES),
    ("angle step", &s4_misc::ANGLE_STEP),
    ("angle step (curved)", &s4_misc::ANGLE_STEP_CURVED),
    ("call slot 296 (this)", &s4_small::CALL_296),
    ("call slot 276 (this + 4)", &s4_small::CALL_276),
    ("call slot 37 (this)", &s4_small::CALL_37),
    ("call slot 239 (edx)", &s4_small::CALL_239_EDX),
    ("bone with rate", &s4_small::BONE_RATE),
    ("call slot 237 (model)", &s4_small::CALL_237),
    ("call slot 250 (xmm)", &s4_small::CALL_250),
    ("locked slot 878", &s4_small::LOCKED_878),
    ("call slot 213 when idle", &s4_small::CALL_213_IDLE),
    ("handle for slot 393", &s4_small::HANDLE_393),
    ("attach slot 211", &s4_small::ATTACH_211),
    ("locked slot 712", &s4_small::LOCKED_712),
    ("frames to time", &s4_small::FRAMES_TIME),
    ("big node", &s4_small::BIG_NODE),
    ("particle node from", &s4_small::PARTICLE_FROM),
    ("record reset", &s4_small::RECORD_RESET),
    ("each slot 542", &s4_small::EACH_542),
    ("light set", &s4_small::LIGHT_SET),
    ("light on", &s4_small::LIGHT_ON),
    ("actor index", &s4_small::ACTOR_INDEX),
    ("fade 541 or 38 (xmm1)", &s4_small::FADE_541_OR_38_BY),
    ("link node", &s4_small::LINK_NODE),
    ("link node, x", &s4_small::LINK_NODE_X),
    ("link node, +0x38", &s4_small::LINK_NODE_38),
    ("link node, x y", &s4_small::LINK_NODE_XY),
    ("records clear 100", &s4_small::RECORDS_CLEAR_100),
    ("register 320", &s4_small::REGISTER_320),
    ("link node, +0x38 and 1.0", &s4_small::LINK_NODE_38_ONE),
    ("link node, x and 0xff", &s4_small::LINK_NODE_X_FF),
    ("link node, +0x2c", &s4_small::LINK_NODE_2C),
    ("walk, then slot 275", &s4_small::WALK_275),
    ("link node, grey", &s4_small::LINK_NODE_GREY),
    ("link node, blocks", &s4_small::LINK_NODE_BLOCKS),
    ("model place", &s4_small::MODEL_PLACE),
    ("static node link", &s4_small::STATIC_NODE_LINK),
    ("two vectors", &s4_small::TWO_VECTORS),
    ("state reset", &s4_small::STATE_RESET),
    ("child node", &s4_small::CHILD_NODE),
    ("move set-up", &s4_small::MOVE_SETUP),
    ("link node, a d", &s4_small::LINK_NODE_AD),
    ("track key", &s4_small::TRACK_KEY),
    ("static child (0513)", &s4_small::STATIC_CHILD),
    ("strip window", &s4_small::STRIP_WINDOW),
    ("fader task", &s4_small::FADER_TASK),
    ("fader task 40", &s4_small::FADER_TASK_40),
    ("fader task 50", &s4_small::FADER_TASK_50),
    ("int fade", &s4_small::INT_FADE),
    ("link node, slot 942", &s4_small::LINK_NODE_942),
    ("ring write", &s4_small::RING_WRITE),
    ("static link (xmm2)", &s4_small::STATIC_LINK_XMM),
    ("face point", &s4_small::FACE_POINT),
    ("table object", &s4_small::TABLE_OBJECT),
    ("leave and register", &s4_small::LEAVE_REGISTER),
    ("0644 start 0x260, placed", &s4_fam446::START_260_PLACED),
    ("part drop", &s4_small::PART_DROP),
    ("walk and reset", &s4_small::WALK_RESET),
    ("walk and ring write", &s4_small::WALK_RING),
    ("record take", &s4_small::RECORD_TAKE),
    ("walk and track", &s4_small::WALK_TRACK),
    ("id list", &s4_small::ID_LIST),
    ("actor scale by index", &s4_small::ACTOR_SCALE_BY),
    ("colour from words", &s4_small::COLOUR_FROM_WORDS),
    ("walk, child place", &s4_small::WALK_CHILD_PLACE),
    ("node from arguments", &s4_small::NODE_FROM_ARGS),
    ("heading to a point", &s4_small::HEADING_TO),
    ("level colour", &s4_small::LEVEL_COLOUR),
    ("walk, scale steps", &s4_small::WALK_SCALE_STEPS),
    ("actor to a point", &s4_small::ACTOR_TO_POINT),
    ("part drop (rates)", &s4_small::PART_DROP_RATES),
    ("move set-up (static)", &s4_small::MOVE_SETUP_STATIC),
    ("light node", &s4_small::LIGHT_NODE),
    ("turn a point", &s4_small::TURN_POINT),
    ("node copy", &s4_small::NODE_COPY),
    ("scale task (edx)", &s4_small::SCALE_TASK_EDX),
    ("light level", &s4_small::LIGHT_LEVEL),
    ("summon nodes", &s4_small::SUMMON_NODES),
    ("light link", &s4_small::LIGHT_LINK),
    ("actor index to a point", &s4_small::ACTOR_INDEX_TO_POINT),
    ("scale task 30", &s4_small::SCALE_TASK_30),
    ("scale task 20", &s4_small::SCALE_TASK_20),
    ("scale task 1", &s4_small::SCALE_TASK_1),
    ("scale task (argument)", &s4_small::SCALE_TASK_ARG),
    ("move ends", &s4_small::MOVE_ENDS),
    ("pulse colour", &s4_small::PULSE_COLOUR),
    ("summon nodes (b)", &s4_small::SUMMON_NODES_B),
    ("start host object", &s4_small::START_HOST_OBJECT),
    ("key curve", &s4_small::KEY_CURVE),
    ("light object", &s4_small::LIGHT_OBJECT),
    ("sprite node", &s4_small::SPRITE_NODE),
    ("named parts", &s4_small::NAMED_PARTS),
    ("reset owners (30)", &s4_small::RESET_OWNERS_30),
    ("reset owners (3)", &s4_small::RESET_OWNERS_3),
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
    (
        "o4 fam5 fade_record 0269@3210",
        &o4_fam5::FADE_RECORD_0269_3210,
    ),
    (
        "o4 fam5 fade_record 0325@2250",
        &o4_fam5::FADE_RECORD_0325_2250,
    ),
    (
        "o4 single actor_slot 0171@29c0",
        &o4_single::ACTOR_SLOT_0171_29C0,
    ),
    (
        "o4 single actor_slot_edx 0298@2250",
        &o4_single::ACTOR_SLOT_EDX_0298_2250,
    ),
    (
        "o4 misc vector_set 0343@69e0",
        &o4_misc::VECTOR_SET_0343_69E0,
    ),
    (
        "o4 single3 fade_setup 0577@9170",
        &o4_single3::FADE_SETUP_0577_9170,
    ),
    (
        "o4 single3 head_node 0577@ac20",
        &o4_single3::HEAD_NODE_0577_AC20,
    ),
    (
        "o4 flags flag_step_arg 0203@73c0",
        &o4_flags::FLAG_STEP_ARG_0203_73C0,
    ),
    (
        "o4 flags flag_step_edx 0262@6af0",
        &o4_flags::FLAG_STEP_EDX_0262_6AF0,
    ),
    (
        "o4 flags object_head_node 0203@7940",
        &o4_flags::OBJECT_HEAD_NODE_0203_7940,
    ),
    (
        "o4 more float_ratio_half 0519@2750",
        &o4_more::FLOAT_RATIO_HALF_0519_2750,
    ),
    (
        "o4 more2 object_node_edx 0307@b480",
        &o4_more2::OBJECT_NODE_EDX_0307_B480,
    ),
    // shared-5 internal
    ("model draw (772)", &s5_misc::MODEL_DRAW),
    ("thickness margin", &s5_misc::THICKNESS_MARGIN),
    ("weapon matrix", &s5_misc::WEAPON_MATRIX),
    ("angle keys", &s5_misc::ANGLE_KEYS),
    ("model parts draw", &s5_misc::MODEL_PARTS_DRAW),
    ("aim task", &s5_misc::AIM_TASK),
    ("three parts draw", &s5_misc::THREE_PARTS_DRAW),
    ("aim object", &s5_misc::AIM_OBJECT),
    ("flight task", &s5_misc::FLIGHT_TASK),
    ("sprite packet", &s5_misc::SPRITE_PACKET),
    ("wave table", &s5_misc::WAVE_TABLE),
    ("thrown effect", &s5_misc::THROWN_EFFECT),
    ("burst records", &s5_misc::BURST_RECORDS),
    ("parts vertices", &s5_misc::PARTS_VERTICES),
    ("thrown effect (far nodes)", &s5_misc::THROWN_EFFECT_FAR),
    // round 5, own rows
    ("m0009 part object", &m0009::PART_OBJECT),
    ("m0093 part object", &m0093::PART_OBJECT),
    ("m0098 motion sequence", &m0098::MOTION_SEQUENCE),
    ("m0098 camera matrix", &m0098::CAMERA_MATRIX),
    ("m0146 model nodes (pick)", &m0146::MODEL_NODES_PICK),
    ("m0146 move start", &m0146::MOVE_START),
    ("m0152 build (287)", &m0152::BUILD_287),
    ("m0171 actor block", &m0171::ACTOR_BLOCK),
    ("m0171 aim", &m0171::AIM),
    ("m0183 camera point", &m0183::CAMERA_POINT),
    ("m0186 node cleared", &m0186::NODE_CLEARED),
    ("m0186 node abc", &m0186::NODE_ABC),
    // own3 internal (magic-r6-own)
    ("m0164 mean point", &m0164::MEAN_POINT),
    ("m0186 model record", &m0186::MODEL_RECORD),
    ("m0203 part record", &m0203::PART_RECORD),
    ("m0205 scale matrix", &m0205::SCALE_MATRIX),
    ("m0244 object start", &m0244::OBJECT_START),
    ("m0260 part programs", &m0260::PART_PROGRAMS),
    ("m0262 models leave", &m0262::MODELS_LEAVE),
    ("m0267 transform points", &m0267::TRANSFORM_POINTS),
    ("m0273 tail node", &m0273::TAIL_NODE),
    ("m0295 node under", &m0295::NODE_UNDER),
    ("m0297 motion setup", &m0297::MOTION_SETUP),
    ("m0297 mode switch", &m0297::MODE_SWITCH),
    ("m0298 models to list", &m0298::MODELS_TO_LIST),
    ("m0330 model nodes", &m0330::MODEL_NODES),
    ("m0379 node under", &m0379::NODE_UNDER),
    ("m0389 strip record", &m0389::STRIP_RECORD),
    ("m0387 sprite record", &m0387::SPRITE_RECORD),
    ("m0387 strip record", &m0387::STRIP_RECORD),
    ("m0388 motion task", &m0388::MOTION_TASK),
    ("m0417 take record", &m0417::TAKE_RECORD),
    ("m0387 sprite size", &m0387::SPRITE_SIZE),
    ("m0424 release", &m0424::RELEASE),
    ("m0424 part record", &m0424::PART_RECORD),
    ("m0431 sounds ready", &m0431::SOUNDS_READY),
    ("m0431 colour reset", &m0431::COLOUR_RESET),
    ("m0449 camera start", &m0449::CAMERA_START),
    ("m0449 actors pass", &m0449::ACTORS_PASS),
    ("m0432 sound start", &m0432::SOUND_START),
    ("m0434 colour show", &m0434::COLOUR_SHOW),
    ("m0431 camera start", &m0431::CAMERA_START),
    ("m0431 show and sounds", &m0431::SHOW_AND_SOUNDS),
    ("m0371 block reset", &m0371::BLOCK_RESET),
    ("m0397 party mean", &m0397::PARTY_MEAN),
    ("m0395 start-up", &m0395::START_UP),
    ("m0273 sprite setup", &m0273::SPRITE_SETUP),
    ("m0267 scale matrix", &m0267::SCALE_MATRIX),
    ("m0352 camera start", &m0352::CAMERA_START),
    ("part motion", &s6_misc::PART_MOTION),
    ("motion sequence step", &s6_misc::MOTION_SEQUENCE_STEP),
    ("start object (anim 2)", &s6_misc::START_OBJECT_ANIM_2),
    ("sprites step", &s6_misc::SPRITES_STEP),
    // "targets mean offset (edx)" (s6_misc::TARGETS_MEAN_OFFSET_OF) is the same code
    // identity as "m0577 centre" above: registered once
    ("spring walk", &s6_misc::SPRING_WALK),
    ("counters step", &s6_misc::COUNTERS_STEP),
    ("path step", &s6_misc::PATH_STEP),
    ("node release", &s6_misc::NODE_RELEASE),
    ("parts axes", &s6_misc::PARTS_AXES),
    ("thrown effect (plain)", &s6_misc::THROWN_EFFECT_PLAIN),
    ("start object (reset state)", &s6_misc::START_OBJECT_RESET),
    ("sparks on a model", &s6_misc::SPARKS_ON_MODEL),
    ("parts ring", &s6_misc::PARTS_RING),
    ("live parts axes", &s6_misc::LIVE_PARTS_AXES),
    ("turning path step", &s6_misc::TURNING_PATH_STEP),
    ("eased turning path step", &s6_misc::EASED_TURNING_PATH_STEP),
    ("keyed part step", &s6_misc::KEYED_PART_STEP),
    ("m0032 state reset (0x1e)", &s6_misc::RESET_STATE_1E),
    ("lights and give back", &s6_misc::LIGHTS_AND_GIVE_BACK),
    ("scene point", &s6_misc::SCENE_POINT),
    ("ring sparks", &s6_misc::RING_SPARKS),
    ("smoke puff", &s6_misc::SMOKE_PUFF),
    ("move ends (the DLL's object)", &s6_misc::MOVE_ENDS_OF),
    ("model direct", &s6_misc::MODEL_DIRECT),
    ("spline point", &s6_misc::SPLINE_POINT),
    ("level look matrix", &s6_misc::LEVEL_LOOK_MATRIX),
    ("sprite GS packet", &s6_misc::SPRITE_GS_PACKET),
    ("aimed node walk", &s6_misc::AIMED_NODE_WALK),
    ("ring take", &s6_misc::RING_TAKE),
    // round 7, shared-7 (s7_misc)
    ("handles registered", &s7_misc::HANDLES_REGISTERED),
    ("count fade", &s7_misc::COUNT_FADE),
    ("ring key frame", &s7_misc::RING_KEY_FRAME),
    ("global differences", &s7_misc::GLOBAL_DIFFERENCES),
    (
        "start object (reset state 0x1e)",
        &s7_misc::START_OBJECT_RESET_1E,
    ),
    ("node to live", &s7_misc::NODE_TO_LIVE),
    ("model points node", &s7_misc::MODEL_POINTS_NODE),
    ("light node", &s7_misc::LIGHT_NODE),
    ("model draw node", &s7_misc::MODEL_DRAW_NODE),
    ("look matrix", &s7_misc::LOOK_MATRIX),
    ("moving light node", &s7_misc::MOVING_LIGHT_NODE),
    ("spark at a point", &s7_misc::SPARK_AT_POINT),
    ("stream block", &s7_misc::STREAM_BLOCK),
    ("sprite fade node", &s7_misc::SPRITE_FADE_NODE),
    ("spline moments", &s7_misc::SPLINE_MOMENTS),
    ("spline set-up", &s7_misc::SPLINE_SETUP),
    ("shake node", &s7_misc::SHAKE_NODE),
    ("thunder node", &s7_misc::THUNDER_NODE),
    ("spline object", &s7_misc::SPLINE_OBJECT),
    ("aimed emitter", &s7_misc::AIMED_EMITTER),
    (
        "aimed emitter (angles kept)",
        &s7_misc::AIMED_EMITTER_ANGLES,
    ),
    ("two sparks", &s7_misc::TWO_SPARKS),
    ("two sparks (block word 4)", &s7_misc::TWO_SPARKS_B),
    ("summon object set-up", &s7_misc::SUMMON_OBJECT_SETUP),
    ("flight step", &s7_misc::FLIGHT_STEP),
    ("lit colour node", &s7_misc::LIT_COLOUR_NODE),
    ("emitter set-up", &s7_misc::EMITTER_SETUP),
    ("reel step", &s7_misc::REEL_STEP),
    ("reel phases", &s7_misc::REEL_PHASES),
    ("flagged object step", &s7_misc::FLAGGED_OBJECT_STEP),
    ("aim records", &s7_misc::AIM_RECORDS),
    ("model object set-up", &s7_misc::MODEL_OBJECT_SETUP),
    ("flagged two sparks", &s7_misc::FLAGGED_TWO_SPARKS),
    // round 8, shared-8 (s8_misc)
    ("object node", &s8_misc::OBJECT_NODE),
    ("child object set-up", &s8_misc::CHILD_OBJECT_SETUP),
    ("bone node", &s8_misc::BONE_NODE),
    ("spark burst", &s8_misc::SPARK_BURST),
    ("mesh bounds", &s8_misc::MESH_BOUNDS),
    // round 8, shared-8 (s8_misc): end of INTERNAL
    // round 6, the shape families (o6_fam_*)
    (
        "o6 fam pool pool_reset_take 0180@2140",
        &o6_fam_pool::POOL_RESET_TAKE_0180_2140,
    ),
    (
        "o6 fam pool pool_reset_take 0205@2a60",
        &o6_fam_pool::POOL_RESET_TAKE_0205_2A60,
    ),
    (
        "o6 fam pool pool_reset_take 0243@2800",
        &o6_fam_pool::POOL_RESET_TAKE_0243_2800,
    ),
    (
        "o6 fam pool pool_reset_take 0350@2cf0",
        &o6_fam_pool::POOL_RESET_TAKE_0350_2CF0,
    ),
    (
        "o6 fam pool pool_reset_take 0385@1df0",
        &o6_fam_pool::POOL_RESET_TAKE_0385_1DF0,
    ),
    (
        "o6 fam pool pool_reset_take 0417@4350",
        &o6_fam_pool::POOL_RESET_TAKE_0417_4350,
    ),
    (
        "o6 fam object start_object_first 0074@1c70",
        &o6_fam_object::START_OBJECT_FIRST_0074_1C70,
    ),
    (
        "o6 fam object start_object_first 0247@25e0",
        &o6_fam_object::START_OBJECT_FIRST_0247_25E0,
    ),
    (
        "o6 fam object start_object_first 0200@2be0",
        &o6_fam_object::START_OBJECT_FIRST_0200_2BE0,
    ),
    (
        "o6 fam object start_object_first 0369@2c10",
        &o6_fam_object::START_OBJECT_FIRST_0369_2C10,
    ),
    (
        "o6 fam object start_object_first 0606@4d70",
        &o6_fam_object::START_OBJECT_FIRST_0606_4D70,
    ),
    (
        "o6 fam object start_object_first 0166@1fd0",
        &o6_fam_object::START_OBJECT_FIRST_0166_1FD0,
    ),
    (
        "o6 fam bone bone_node_setup 0201@6f20",
        &o6_fam_bone::BONE_NODE_SETUP_0201_6F20,
    ),
    (
        "o6 fam bone bone_node_setup 0203@7710",
        &o6_fam_bone::BONE_NODE_SETUP_0203_7710,
    ),
    (
        "o6 fam bone bone_node_setup 0273@7770",
        &o6_fam_bone::BONE_NODE_SETUP_0273_7770,
    ),
    (
        "o6 fam node node_reset_sound 0577@36d0",
        &o6_fam_node::NODE_RESET_SOUND_0577_36D0,
    ),
    (
        "o6 fam node node_reset_sound 0155@2fc0",
        &o6_fam_node::NODE_RESET_SOUND_0155_2FC0,
    ),
    (
        "o6 fam node node_reset_sound 0070@3ed0",
        &o6_fam_node::NODE_RESET_SOUND_0070_3ED0,
    ),
    (
        "o6 fam launch random_launch 0384@15e0",
        &o6_fam_launch::RANDOM_LAUNCH_0384_15E0,
    ),
    (
        "o6 fam launch random_launch 0421@15e0",
        &o6_fam_launch::RANDOM_LAUNCH_0421_15E0,
    ),
    (
        "o6 fam launch random_launch 0535@15c0",
        &o6_fam_launch::RANDOM_LAUNCH_0535_15C0,
    ),
    (
        "o6 fam pool pool_reset_take 0250@1e00",
        &o6_fam_pool::POOL_RESET_TAKE_0250_1E00,
    ),
    (
        "o6 fam pool pool_reset_take 0284@3430",
        &o6_fam_pool::POOL_RESET_TAKE_0284_3430,
    ),
    (
        "o6 fam pairs2 models_detach 0164@7cf0",
        &o6_fam_pairs2::MODELS_DETACH_0164_7CF0,
    ),
    (
        "o6 fam pairs2 models_detach 0155@7c70",
        &o6_fam_pairs2::MODELS_DETACH_0155_7C70,
    ),
    (
        "o6 fam pairs2 actor_object_start 0644@8df0",
        &o6_fam_pairs2::ACTOR_OBJECT_START_0644_8DF0,
    ),
    (
        "o6 fam pairs2 actor_object_start 0565@4dd0",
        &o6_fam_pairs2::ACTOR_OBJECT_START_0565_4DD0,
    ),
    (
        "o6 fam pairs2 shake_visit 0201@4780",
        &o6_fam_pairs2::SHAKE_VISIT_0201_4780,
    ),
    (
        "o6 fam pairs2 shake_visit 0387@5eb0",
        &o6_fam_pairs2::SHAKE_VISIT_0387_5EB0,
    ),
    (
        "o6 fam object start_object_first 0132@3cc0",
        &o6_fam_object::START_OBJECT_FIRST_0132_3CC0,
    ),
    (
        "o6 fam object start_object_first 0126@22a0",
        &o6_fam_object::START_OBJECT_FIRST_0126_22A0,
    ),
    (
        "o6 fam pairs3 part_matrix_handover 0307@2550",
        &o6_fam_pairs3::PART_MATRIX_HANDOVER_0307_2550,
    ),
    (
        "o6 fam pairs3 part_matrix_handover 0605@7ed0",
        &o6_fam_pairs3::PART_MATRIX_HANDOVER_0605_7ED0,
    ),
    (
        "o6 fam pairs4 three_pass_model 0162@1a80",
        &o6_fam_pairs4::THREE_PASS_MODEL_0162_1A80,
    ),
    (
        "o6 fam pairs4 three_pass_model 0586@1a20",
        &o6_fam_pairs4::THREE_PASS_MODEL_0586_1A20,
    ),
    // round 6, the shape families (o6_fam_*): end of INTERNAL
    (
        "o6 fam pairs5 record_node_start 0199@2160",
        &o6_fam_pairs5::RECORD_NODE_START_0199_2160,
    ),
    (
        "o6 fam pairs5 record_node_start 0251@2550",
        &o6_fam_pairs5::RECORD_NODE_START_0251_2550,
    ),
    (
        "o6 fam pairs5 global_node_start 0253@4370",
        &o6_fam_pairs5::GLOBAL_NODE_START_0253_4370,
    ),
    (
        "o6 fam pairs5 global_node_start 0311@1f60",
        &o6_fam_pairs5::GLOBAL_NODE_START_0311_1F60,
    ),
    (
        "o6 fam pairs5 two_part_visit 0070@1160",
        &o6_fam_pairs5::TWO_PART_VISIT_0070_1160,
    ),
    (
        "o6 fam pairs5 ribbon_visit 0070@86d0",
        &o6_fam_pairs5::RIBBON_VISIT_0070_86D0,
    ),
    (
        "o6 fam pairs5 ribbon_visit 0155@8170",
        &o6_fam_pairs5::RIBBON_VISIT_0155_8170,
    ),
    (
        "o6 fam pairs5 timed_items_visit 0155@7d70",
        &o6_fam_pairs5::TIMED_ITEMS_VISIT_0155_7D70,
    ),
    (
        "o6 fam pairs5 timed_items_visit 0164@7df0",
        &o6_fam_pairs5::TIMED_ITEMS_VISIT_0164_7DF0,
    ),
    (
        "o6 fam pairs5 sprite_visit 0578@30d0",
        &o6_fam_pairs5::SPRITE_VISIT_0578_30D0,
    ),
    (
        "o6 fam pairs5 sprite_visit 0585@3110",
        &o6_fam_pairs5::SPRITE_VISIT_0585_3110,
    ),
    (
        "o6 fam pairs5 light_setup 0644@b390",
        &o6_fam_pairs5::LIGHT_SETUP_0644_B390,
    ),
    (
        "o6 fam pairs5 sprite_frame 0206@2730",
        &o6_fam_pairs5::SPRITE_FRAME_0206_2730,
    ),
    (
        "o6 fam pairs6 trail_draw 0446@4e20",
        &o6_fam_pairs6::TRAIL_DRAW_0446_4E20,
    ),
    (
        "o6 fam pairs6 trail_draw 0544@4ce0",
        &o6_fam_pairs6::TRAIL_DRAW_0544_4CE0,
    ),
    (
        "o6 fam pairs6 packed_sprite 0353@13b0",
        &o6_fam_pairs6::PACKED_SPRITE_0353_13B0,
    ),
    (
        "o6 fam pairs6 packed_sprite 0672@1310",
        &o6_fam_pairs6::PACKED_SPRITE_0672_1310,
    ),
    (
        "o6 fam pairs6 packed_sprite 0266@1310",
        &o6_fam_pairs6::PACKED_SPRITE_0266_1310,
    ),
    (
        "o6 fam pairs6 scene_end 0610@ab20",
        &o6_fam_pairs6::SCENE_END_0610_AB20,
    ),
    (
        "o6 fam pairs6 scene_end 0434@5b90",
        &o6_fam_pairs6::SCENE_END_0434_5B90,
    ),
    // families2 (o6_fam_pairs5..): end of INTERNAL
    ("m0152 build node", &m0152::BUILD_NODE),
    ("m0168 object task", &m0168::OBJECT_TASK),
    ("m0184 scatter", &m0184::SCATTER),
    ("m0186 model node", &m0186::MODEL_NODE),
    ("m0186 cleared node", &m0186::CLEARED_NODE),
    ("m0207 copy node", &m0207::COPY_NODE),
    ("m0295 leave and register", &m0295::LEAVE_AND_REGISTER),
    ("m0348 lights models", &m0348::LIGHTS_MODELS),
    ("m0350 lights models parts", &m0350::LIGHTS_MODELS_PARTS),
    ("m0389 static strip", &m0389::STATIC_STRIP),
    ("sl_host 0391", &sl_host::HOSTED_0391),
    ("sl_host 0448", &sl_host::HOSTED_0448),
    ("sl_host 0444", &sl_host::HOSTED_0444),
    ("sl_host 0199", &sl_host::HOSTED_0199),
    // magic small-lo: end of INTERNAL
];

/// What covers the function at `addr`: a port's name (an entry
/// [`bind`] finds, or an [`INTERNAL`] function).
#[must_use]
pub fn covered(img: &Image, addr: u32) -> Option<&'static str> {
    bind(img, addr).map(|i| PORTS[i].0).or_else(|| {
        INTERNAL
            .iter()
            .find(|(_, sig)| img.in_text(addr) && !img.in_reloc(addr) && img.matches(addr, sig))
            .map(|(n, _)| *n)
    })
}

/// The port of the function at `addr` of `img`: its index in [`PORTS`].
/// `None` when no port matches the code there. Slot thunks differ only
/// in the slot, so they bind by shape ([`runtime::slot_of_thunk`]).
///
/// An address inside a relocated word binds nothing: `ffx-host` offers
/// every relocated word pointing into `.text`, and a jump table's address
/// (`jmp [eax*4 + table]`) is one. `magic_0168` 0x10005c94 and seven
/// others start with `0xc3`, which the lone `ret` matched, and the
/// host's `int3` there broke the table's first entry.
#[must_use]
pub fn bind(img: &Image, addr: u32) -> Option<usize> {
    if !img.in_text(addr) || img.in_reloc(addr) {
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
