//! The function table (docs/host.md, "The function table"): every port
//! under its original address. Unicorn code that reaches a registered
//! address enters the port; a port that calls an address goes through
//! [`crate::exec::Exec::call`], which takes the port when there is one and
//! Unicorn otherwise. Later this is the modding seam: a mod replaces an
//! entry.

use std::collections::BTreeMap;

use crate::exec::Exec;

/// How a function takes its arguments and who pops them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Conv {
    /// Stack words; the caller pops.
    Cdecl,
    /// Stack words; the callee pops (`ret 4n`).
    Stdcall,
    /// `this` in `ecx`, stack words; the callee pops.
    Thiscall,
    /// `ecx`, `edx`, stack words; the callee pops.
    Fastcall,
    /// `ecx`, `edx`, stack words; the caller pops: the custom convention
    /// MSVC's link-time code generation made for some internal functions
    /// (docs/hooking.md, `fastcall-caller-pops`).
    FastcallCallerPops,
}

impl Conv {
    /// Whether the callee pops its stack words.
    #[must_use]
    pub const fn callee_pops(self) -> bool {
        matches!(self, Self::Stdcall | Self::Thiscall | Self::Fastcall)
    }

    /// How many leading arguments travel in registers (`ecx`, `edx`).
    #[must_use]
    pub const fn register_args(self) -> usize {
        match self {
            Self::Cdecl | Self::Stdcall => 0,
            Self::Thiscall => 1,
            Self::Fastcall | Self::FastcallCallerPops => 2,
        }
    }

    /// The `ported!` ABI string's convention.
    #[must_use]
    pub fn from_abi(abi: &str) -> Option<Self> {
        Some(match abi {
            "C" | "cdecl" => Self::Cdecl,
            "stdcall" => Self::Stdcall,
            "thiscall" => Self::Thiscall,
            "fastcall" => Self::Fastcall,
            "fastcall-caller-pops" => Self::FastcallCallerPops,
            _ => return None,
        })
    }
}

/// A call's arguments as the original's caller passed them: the register
/// arguments, then the stack words.
#[derive(Debug, Clone, Default)]
pub struct Args {
    /// `ecx`.
    pub ecx: u32,
    /// `edx`.
    pub edx: u32,
    /// The stack words, lowest address first.
    pub stack: Vec<u32>,
    /// The game address of the first stack word (a port whose original
    /// writes its own argument slots writes them there).
    pub slots: u32,
}

impl Args {
    /// The `i`-th argument in the original's order: the register ones
    /// first (by `conv`), then the stack words; 0 past the end.
    #[must_use]
    pub fn arg(&self, conv: Conv, i: usize) -> u32 {
        let regs = conv.register_args();
        match (regs, i) {
            (1.., 0) => self.ecx,
            (2, 1) => self.edx,
            _ => self.stack.get(i - regs).copied().unwrap_or(0),
        }
    }
}

/// What a port returns, where the original's caller reads it.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Ret {
    /// `eax`.
    pub eax: u32,
    /// `edx` (64-bit results), or `None` to leave the caller's.
    pub edx: Option<u32>,
    /// `ST0` (float results), or `None` for an empty x87 stack.
    pub st0: Option<f64>,
    /// Whether `eax` is a result (`false`: the caller's `eax` is kept, as
    /// the hook's thunk keeps it for a void shim).
    pub sets_eax: bool,
}

impl Ret {
    /// Nothing returned: every register the caller's.
    pub const VOID: Self = Self {
        eax: 0,
        edx: None,
        st0: None,
        sets_eax: false,
    };

    /// A result in `eax`.
    #[must_use]
    pub const fn eax(v: u32) -> Self {
        Self {
            eax: v,
            edx: None,
            st0: None,
            sets_eax: true,
        }
    }

    /// A result in `edx:eax`.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)] // the two halves
    pub const fn u64(v: u64) -> Self {
        Self {
            eax: v as u32,
            edx: Some((v >> 32) as u32),
            st0: None,
            sets_eax: true,
        }
    }

    /// A result in `ST0`.
    #[must_use]
    pub const fn st0(v: f64) -> Self {
        Self {
            eax: 0,
            edx: None,
            st0: Some(v),
            sets_eax: false,
        }
    }
}

/// A port: the original's arguments in, its result out, with the executor
/// for calls back into the game.
pub type Port = fn(&mut Exec, &Args) -> Ret;

/// One entry.
#[derive(Debug, Clone, Copy)]
pub struct Entry {
    /// The `ffx.sym` name.
    pub name: &'static str,
    /// The convention.
    pub conv: Conv,
    /// Stack words the caller pushes (register arguments not counted).
    pub words: usize,
    /// The port.
    pub port: Port,
}

/// Every port by original address.
#[derive(Debug, Default, Clone)]
pub struct Table {
    entries: BTreeMap<u32, Entry>,
    /// Ports that wrap their original: address → the original's first
    /// instructions (whole ones, position-independent), which the executor
    /// copies into a trampoline ([`crate::exec::Exec::call_original`]).
    wraps: BTreeMap<u32, &'static [u8]>,
}

impl Table {
    /// An empty table.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers `entry` at `addr`, replacing (and returning) an earlier one.
    pub fn register(&mut self, addr: u32, entry: Entry) -> Option<Entry> {
        self.entries.insert(addr, entry)
    }

    /// Registers `entry` at `addr` as a wrapper of the original there,
    /// whose code starts with `prologue` (checked against the image when
    /// the executor is built; the wrapper calls the original through
    /// [`crate::exec::Exec::call_original`]).
    pub fn wrap(&mut self, addr: u32, entry: Entry, prologue: &'static [u8]) -> Option<Entry> {
        self.wraps.insert(addr, prologue);
        self.entries.insert(addr, entry)
    }

    /// The wrapped originals: address and prologue.
    pub fn wrapped(&self) -> impl Iterator<Item = (u32, &'static [u8])> + '_ {
        self.wraps.iter().map(|(&a, &p)| (a, p))
    }

    /// The port at `addr`.
    #[must_use]
    pub fn get(&self, addr: u32) -> Option<&Entry> {
        self.entries.get(&addr)
    }

    /// Every address with a port.
    pub fn addresses(&self) -> impl Iterator<Item = u32> + '_ {
        self.entries.keys().copied()
    }

    /// How many ports.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether there are none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
